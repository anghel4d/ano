// Steel's BQN backend. Ordinary statements read pre-state, stage effects, then commit at the
// barrier. Installed rules share one pre-state and commit set per synthetic tick.
// Continuations reuse anoSel.

use crate::alias::LookupMode;
use crate::num::{fmt_g, int_fast};
use crate::reducer;
use crate::registry::{names_eq, reg_find, reg_role};
use crate::{
    ArithOp, AssignOp, BindKind, CmpOp, ColType, Diag, Directives, Expect, Interner, Node,
    NodeKind, RegEntry, RegEntryKind, Registry, Symbol,
};

type R<T> = Result<T, Diag>;

// Inputs: the stem the source requested, the resolver request it made. Output: the spelling to
// print in a lookup refusal — `^name` keeps its sigil, so `^Nope` never reads back as `Nope`.
fn spelled(req: &str, lm: LookupMode) -> String {
    match lm {
        LookupMode::Bare => req.to_string(),
        LookupMode::DynamicAliasThenBare => format!("^{}", req),
    }
}

// Inputs: the resolved lookup name, the requested stem, the resolver request. Output: true when
// the overlay moved the lookup off the source spelling, so a refusal must name both.
fn moved(n: &str, req: &str, lm: LookupMode) -> bool {
    lm == LookupMode::DynamicAliasThenBare && n != req
}

/* ---------- types ---------- */

// FR_ENT entity world; FR_LINE 1-D shape (w = length); FR_LAT w×h lattice; FR_BOARD board
// literal (lit = glyph run).
#[derive(Clone, Copy, PartialEq, Eq)]
enum FrameKind {
    Ent,
    Lat,
    Line,
    Board,
}

#[derive(Clone)]
struct Frame {
    kind: FrameKind,
    w: i32,
    h: i32,
    lit: String,
}

impl Frame {
    fn ent() -> Frame {
        Frame { kind: FrameKind::Ent, w: 0, h: 0, lit: String::new() }
    }
}

// The three gather spaces (C Mode): World = frame length, Sel = selection-compressed,
// Copy = selection then spawn-count replicated.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    World,
    Sel,
    Copy,
}

// C EV plus lineage: v is the expression (already in mode), g the WORLD-SPACE guard,
// pair/unit/sym its value shape, along an ordered scan's row order, domain the world-row vector
// aligned with every scan result, and rel_ent the rel whose VALUES v holds. gv marks g as a
// VALIDITY guard — an identityless fold over nothing has no result row, so the query path must
// consume it before binding, labelling, comparing, or displaying. Relationship foundness guards
// stay untagged and keep their existing behavior.
#[derive(Clone, Default)]
struct Ev {
    v: String,
    g: Option<String>,
    gv: bool,
    pair: bool,
    unit: bool,
    sym: bool,
    along: Option<String>,
    domain: Option<String>,
    rel_ent: Option<usize>,
}

// Pipeline view: base mask temp (None = iota/callable-rebased), idx ordered indices into the
// filtered space, expand_cnt from an expand stage, is_iota flag.
#[derive(Default)]
struct View {
    base: Option<String>,
    idx: String,
    expand_cnt: Option<String>,
    is_iota: bool,
}

// One pending column replacement. fam merge family: b'+' b'*' b'=' b'|' b'&' b'v'.
struct Commit {
    col_idx: usize,
    new_expr: String,
    fam: u8,
    field: Option<String>,
    rules: u32,
}

// One spawn group: cnt selection-space counts temp, tot "(+´cnt)", pos copy-space positions
// temp, proto_name a named proto/column, proto_expr a staged computed-proto temp.
struct SpawnG {
    cnt: String,
    tot: String,
    pos: Option<String>,
    pos_pair: bool,
    proto_name: Option<Symbol>,
    proto_expr: Option<String>,
}

// Per-statement effect staging (C Fx). despawn_sel ORs across a rule tick.
#[derive(Default)]
struct Fx {
    commits: Vec<Commit>,
    despawn: bool,
    despawn_sel: Option<String>,
    sp: Vec<SpawnG>,
}

// The emit context (C Em). tmp is the global temp counter, never reset; stmt is one counter
// shared by statements, queries, comprehensions, and rule ticks.
struct Em<'a> {
    reg: &'a Registry,
    dirs: &'a Directives,
    it: &'a Interner,
    out: String,
    pre: String,
    defs: Vec<&'a Node>,
    is_pair: Vec<bool>,
    fr: Frame,
    saved_fr: Frame,
    have_saved: bool,
    cur_rule: i32,
    rule_disj: [u32; 32],
    sel_var: String,
    trace_sel: String,
    cnt_var: String,
    idx_var: String,
    pipe_expand: String,
    need_idx: bool,
    world_shifted: bool,
    tmp: i32,
    out_idx: usize,
    stmt: i32,
    // the program prologue: helper declarations in the order the lowering asked for them, and
    // the byte offset the fixture reserved for them. rel_seals counts the relationship writes
    // this lowering staged, which the final assembly reconciles against the emitted text.
    decls: Vec<String>,
    decl_at: usize,
    rel_seals: usize,
    // trace identity: plan collects one record per staged runtime crossing; in_effect is the
    // phase (false during the discarded guard probe); site_kind is the s/q/c block letter.
    plan: crate::trace::TracePlan,
    in_effect: bool,
    site_kind: char,
}

/* ---------- free helpers ---------- */

// Inputs: message. Output: at most 399 bytes of it (C vsnprintf into char[400]), trimmed to a
// char boundary.
fn clip399(msg: &str) -> &str {
    if msg.len() < 400 {
        return msg;
    }
    let mut i = 399;
    while i > 0 && !msg.is_char_boundary(i) {
        i -= 1;
    }
    &msg[..i]
}

// Inputs: 1-based source line, message. Output: the refusal Diag "emit: line %d: %s".
fn fail(line: i32, msg: impl AsRef<str>) -> Diag {
    Diag::refuse(format!("emit: line {}: {}", line, clip399(msg.as_ref())))
}

// BQN identifier legality: [A-Za-z][A-Za-z0-9_]*.
fn bqnlegal(n: &str) -> bool {
    let b = n.as_bytes();
    if b.is_empty() || !b[0].is_ascii_alphabetic() {
        return false;
    }
    b[1..].iter().all(|&c| c.is_ascii_alphanumeric() || c == b'_')
}

// True only for one whole ASCII BQN identifier, never a prefix such as t1 inside t10.
fn mentions_bqn_name(expr: &str, name: &str) -> bool {
    let ident = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
    expr.match_indices(name).any(|(at, _)| {
        let bytes = expr.as_bytes();
        let end = at + name.len();
        (at == 0 || !ident(bytes[at - 1]))
            && (end == bytes.len() || !ident(bytes[end]))
    })
}

// First letter lowercased (C tolower on byte 0; only reached on bqnlegal names).
fn lc(n: &str) -> String {
    let mut cs = n.chars();
    match cs.next() {
        Some(c) => {
            let mut s = String::with_capacity(n.len());
            s.push(c.to_ascii_lowercase());
            s.push_str(cs.as_str());
            s
        }
        None => String::new(),
    }
}

// Number spelling with BQN high-minus: integer fast path (no -0 exclusion) via %lld, else
// %.17g; then every '-' -> ¯ and every '+' dropped.
fn num_lit(x: f64) -> String {
    if x == f64::INFINITY {
        return "∞".to_string();
    }
    if x == f64::NEG_INFINITY {
        return "¯∞".to_string();
    }
    let buf = match int_fast(x) {
        Some(i) => i.to_string(),
        None => fmt_g(17, x),
    };
    let mut out = String::with_capacity(buf.len() + 8);
    for c in buf.chars() {
        if c == '-' {
            out.push('¯');
        } else if c != '+' {
            out.push(c);
        }
    }
    out
}

// Guard conjunction; None = total. Both operands parenthesized.
fn g_and(a: Option<String>, b: Option<String>) -> Option<String> {
    match (a, b) {
        (None, b) => b,
        (a, None) => a,
        (Some(a), Some(b)) => Some(format!("({})∧({})", a, b)),
    }
}

// Merge row-domain lineage through a pointwise dyad. A scalar extends over either side. An
// ordinary column may join a scan only when the current evaluation mode supplies that exact row
// vector; this admits full-world scan/column expressions without weakening foreign-domain checks.
fn scan_domain_join(a: &Ev, b: &Ev, implicit: Option<&str>, line: i32) -> R<Option<String>> {
    match (&a.domain, &b.domain) {
        (None, None) => Ok(None),
        (Some(x), Some(y)) if x == y => Ok(Some(x.clone())),
        (Some(x), None) if b.unit || implicit == Some(x.as_str()) => Ok(Some(x.clone())),
        (None, Some(y)) if a.unit || implicit == Some(y.as_str()) => Ok(Some(y.clone())),
        (Some(_), Some(_)) => Err(fail(line, "values from distinct scan domains cannot compose")),
        _ => Err(fail(line, "scan value composed against a value without its row domain")),
    }
}

// The same law for traversal order. Assignment later grades this order back to world-row order.
fn scan_order_join(a: &Ev, b: &Ev, line: i32) -> R<Option<String>> {
    match (&a.along, &b.along) {
        (None, None) => Ok(None),
        (Some(x), Some(y)) if x == y => Ok(Some(x.clone())),
        (Some(x), None) if b.unit => Ok(Some(x.clone())),
        (None, Some(y)) if a.unit => Ok(Some(y.clone())),
        _ => Err(fail(line, "scan-along composed against a differently-ordered operand")),
    }
}

// Inputs: a CANONICAL head spelling (normalize already rebound it) and the form. Output: the
// one descriptor the resolver checked. A miss is a registered name or an internal invariant
// break, never a user diagnostic.
fn desc_of(op: &str, form: reducer::Form) -> Option<reducer::OpDesc> {
    reducer::canonical(op, form)
}

// Folds whose descriptor registers an empty identity: empty gathers need no guard.
fn fold_has_id(op: &str) -> bool {
    desc_of(op, reducer::Form::Fold).is_some_and(|d| d.empty_identity().is_some())
}

// C bindKind strings for diagnostics.
fn bind_kind_str(k: BindKind) -> &'static str {
    match k {
        BindKind::Entity => "entity",
        BindKind::Mask => "mask",
        BindKind::Point => "point",
        BindKind::Num => "num",
        BindKind::Vec => "vec",
    }
}

// Element type of a Col/Field entry; None otherwise.
fn col_ty(e: &RegEntry) -> Option<ColType> {
    match &e.kind {
        RegEntryKind::Col { ty, .. } => Some(*ty),
        RegEntryKind::Field { ty, .. } => Some(*ty),
        _ => None,
    }
}

// C e->hasPres: presence lives only on RK_COL.
fn has_pres(e: &RegEntry) -> bool {
    matches!(e.kind, RegEntryKind::Col { pres: Some(_), .. })
}

// C e->uniq: declared injectivity, RK_COL only.
fn is_uniq(e: &RegEntry) -> bool {
    matches!(e.kind, RegEntryKind::Col { uniq: true, .. })
}

// C e->keyOf non-empty: the declared key column spelling of a rel/srel.
fn key_of(e: &RegEntry) -> Option<&str> {
    match &e.kind {
        RegEntryKind::Rel { key_of, .. } => key_of.as_deref(),
        RegEntryKind::SRel { key_of, .. } => key_of.as_deref(),
        _ => None,
    }
}

// C nd->num where the port keeps typed payloads: Num nodes only, 0 otherwise (Wild dims).
fn num_of(nd: &Node) -> f64 {
    if let NodeKind::Num(x) = nd.kind { x } else { 0.0 }
}

// The Shape node's dim list; empty when not a Shape.
fn shape_dims(nd: &Node) -> &[Node] {
    if let NodeKind::Shape(v) = &nd.kind { v } else { &[] }
}

// The def body (C d->kids[0]).
fn def_body(d: &Node) -> &Node {
    if let NodeKind::DefStmt { body, .. } = &d.kind { body } else { d }
}

// The fiber-name spelling of a node in relation position (C nd->name; N_SETHOP copies the
// name of its N_NAME kid).
fn fiber_sym(nd: &Node) -> Symbol {
    match &nd.kind {
        NodeKind::Name(s) | NodeKind::Alias { look: s, .. } => *s,
        NodeKind::SetHop { rel } => *rel,
        _ => Symbol::EMPTY,
    }
}

// C kid order per node kind — the generic traversal for scanDespawn/containsIota/
// findShapeScope; the phantom N_NAME kids of SetHop/CmpAny are handled at their use sites.
fn children(nd: &Node) -> Vec<&Node> {
    use NodeKind::*;
    match &nd.kind {
        Not(a) | IotaX(a) | Expand(a) | Query(a) => vec![a.as_ref()],
        And(a, b) | Or(a, b) => vec![a.as_ref(), b.as_ref()],
        Cmp { l, r, .. } | Arith { l, r, .. } | Hop { l, r } => vec![l.as_ref(), r.as_ref()],
        Scope { l, r, origin } => {
            let mut v = vec![l.as_ref(), r.as_ref()];
            if let Some(o) = origin {
                v.push(o.as_ref());
            }
            v
        }
        Call { args, .. } | EVerb { args, .. } => args.iter().collect(),
        Fold { operand, .. } | ScanExpr { operand, .. } => vec![operand.as_ref()],
        ScanAlong { col, order, .. } => vec![col.as_ref(), order.as_ref()],
        Shape(v) | Tuple(v) | Program(v) => v.iter().collect(),
        To { shape, poured } => {
            let mut v = vec![shape.as_ref()];
            if let Some(p) = poured {
                v.push(p.as_ref());
            }
            v
        }
        Grade { key, .. } => vec![key.as_ref()],
        Top { inner, .. } => vec![inner.as_ref()],
        Pipe { src, stages } => {
            let mut v = vec![src.as_ref()];
            v.extend(stages.iter());
            v
        }
        OrderBy { key, .. } => vec![key.as_ref()],
        CrossV { a, b, .. } => vec![a.as_ref(), b.as_ref()],
        Binder { source, .. } => vec![source.as_ref()],
        EAssign { target, rhs, .. } => vec![target.as_ref(), rhs.as_ref()],
        ESpawn { what, count, at } => {
            let mut v = vec![what.as_ref()];
            if let Some(c) = count {
                v.push(c.as_ref());
            }
            if let Some(a) = at {
                v.push(a.as_ref());
            }
            v
        }
        EVia { col, .. } => vec![col.as_ref()],
        Stmt { sel, effects, .. } => {
            let mut v: Vec<&Node> = Vec::new();
            if let Some(s) = sel {
                v.push(s.as_ref());
            }
            v.extend(effects.iter());
            v
        }
        DefStmt { body, .. } => vec![body.as_ref()],
        Compr { sel, effect, rest } => {
            let mut v = vec![sel.as_ref(), effect.as_ref()];
            v.extend(rest.iter());
            v
        }
        _ => Vec::new(),
    }
}

// Leftmost leaf of an &/|/@ chain — the frame-setting source position.
fn leftmost(nd: &Node) -> &Node {
    let mut nd = nd;
    loop {
        match &nd.kind {
            NodeKind::And(l, _) | NodeKind::Or(l, _) => nd = l,
            NodeKind::Scope { l, .. } => nd = l,
            _ => return nd,
        }
    }
}

// First @-scope with a shape rhs anywhere in the selection; returns the SHAPE node.
fn find_shape_scope(nd: &Node) -> Option<&Node> {
    if let NodeKind::Scope { r, .. } = &nd.kind {
        if matches!(r.kind, NodeKind::Shape(_)) {
            return Some(r);
        }
    }
    for k in children(nd) {
        if let Some(x) = find_shape_scope(k) {
            return Some(x);
        }
    }
    None
}

// Strip a frame-setting source out of the predicate: SHAPE / board-TO -> None; AND whose
// kids[0] is directly SHAPE -> the right arm; a buried SHAPE falls through UNstripped (quirk).
fn strip_frame(sel: Option<&Node>) -> Option<&Node> {
    let sel = sel?;
    if matches!(sel.kind, NodeKind::Shape(_)) {
        return None;
    }
    if let NodeKind::To { poured: Some(p), .. } = &sel.kind {
        if matches!(p.kind, NodeKind::Str(_)) {
            return None;
        }
    }
    if let NodeKind::And(l, r) = &sel.kind {
        let lm = leftmost(sel);
        if matches!(lm.kind, NodeKind::Shape(_)) && matches!(l.kind, NodeKind::Shape(_)) {
            return Some(r);
        }
    }
    Some(sel)
}

// 1 when an ↕ generator appears anywhere in the subtree.
fn contains_iota(nd: &Node) -> bool {
    if matches!(nd.kind, NodeKind::IotaX(_)) {
        return true;
    }
    children(nd).into_iter().any(contains_iota)
}

// 1 when the subtree contains a despawn effect.
fn scan_despawn(nd: &Node) -> bool {
    if matches!(nd.kind, NodeKind::EDespawn) {
        return true;
    }
    children(nd).into_iter().any(scan_despawn)
}

// Does this subtree start a gamma (a set hop at fold-operand level)?
fn is_gamma_operand(nd: &Node) -> bool {
    match &nd.kind {
        NodeKind::SetHop { .. } => true,
        NodeKind::Hop { l, .. } => matches!(l.kind, NodeKind::SetHop { .. }),
        NodeKind::And(l, _) => matches!(l.kind, NodeKind::SetHop { .. }),
        _ => false,
    }
}

// The statement's selection / effect kids (C st->kids[0] / kids[1..]).
fn stmt_sel(st: &Node) -> Option<&Node> {
    if let NodeKind::Stmt { sel, .. } = &st.kind { sel.as_deref() } else { None }
}

fn stmt_effects(st: &Node) -> &[Node] {
    if let NodeKind::Stmt { effects, .. } = &st.kind { effects } else { &[] }
}

// sscanf("%63s") on a verb's raw text: skip C isspace, take up to 63 bytes of the first word.
fn verb_target(raw: &str) -> String {
    let b = raw.as_bytes();
    let issp = |c: u8| matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r');
    let mut i = 0;
    while i < b.len() && issp(b[i]) {
        i += 1;
    }
    let start = i;
    let mut n = 0;
    while i < b.len() && !issp(b[i]) && n < 63 {
        i += 1;
        n += 1;
    }
    String::from_utf8_lossy(&b[start..i]).into_owned()
}

fn claims_call_namespace(kind: &RegEntryKind) -> bool {
    matches!(
        kind,
        RegEntryKind::Fn { .. }
            | RegEntryKind::TypedFn { .. }
            | RegEntryKind::Ctor { .. }
    )
}

/* ---------- Em infrastructure ---------- */

impl<'a> Em<'a> {
    fn rs(&self, s: Symbol) -> &'a str {
        self.it.resolve(s)
    }

    fn ent(&self, i: usize) -> &'a RegEntry {
        &self.reg.ents[i]
    }

    fn find(&self, n: &str) -> Option<usize> {
        reg_find(self.reg, n)
    }

    fn role(&self, r: &str) -> Option<usize> {
        reg_role(self.reg, r)
    }

    // Defs are program variables: exact-byte match (Symbol equality), first match.
    fn find_def(&self, name: Symbol) -> Option<&'a Node> {
        for d in &self.defs {
            if let NodeKind::DefStmt { name: dn, .. } = &d.kind {
                if *dn == name {
                    return Some(d);
                }
            }
        }
        None
    }

    // Fresh temp name t<n>; the counter is global and never reset (probe burns included).
    fn tv(&mut self) -> String {
        let t = format!("t{}", self.tmp);
        self.tmp += 1;
        t
    }

    // Stage one prologue line into the per-statement buffer.
    fn stage(&mut self, line: String) {
        self.pre.push_str(&line);
        self.pre.push('\n');
    }

    // Registry-var spelling: lc(name) when BQN-legal, else jp<i> by registry entry index.
    fn bqnv(&self, i: usize) -> String {
        let n = &self.ent(i).name;
        if bqnlegal(n) { lc(n) } else { format!("jp{}", i) }
    }

    // Presence-mask spelling: pres_<raw name> (NOT lowercased) or pres_jp<i>.
    fn presv(&self, i: usize) -> String {
        let n = &self.ent(i).name;
        if bqnlegal(n) { format!("pres_{}", n) } else { format!("pres_jp{}", i) }
    }

    // Registry-fn spelling: Fn_<raw name> or Fn_jp<i>.
    fn fnv(&self, i: usize) -> String {
        let n = &self.ent(i).name;
        if bqnlegal(n) { format!("Fn_{}", n) } else { format!("Fn_jp{}", i) }
    }

    // Frame cell count as a BQN expr.
    fn fr_n(&self) -> String {
        match self.fr.kind {
            FrameKind::Ent => "anoN".to_string(),
            FrameKind::Line => format!("{}", self.fr.w),
            _ => format!("{}", self.fr.w.wrapping_mul(self.fr.h)),
        }
    }

    // The world's stable-id column: role id, else role keys (RK_COL only), else anoIdx once
    // shifted, else the row iota minted at use.
    fn id_col(&self) -> String {
        let mut e = self.role("id");
        if e.is_none() {
            e = self.role("keys");
        }
        if let Some(i) = e {
            if matches!(self.ent(i).kind, RegEntryKind::Col { .. }) {
                return self.bqnv(i);
            }
        }
        if self.need_idx && self.world_shifted {
            return "anoIdx".to_string();
        }
        format!("(↕{})", self.fr_n())
    }

    // The key expression a functional rel resolves through; None = the positional identity.
    fn rel_key(&self, ri: usize) -> Option<String> {
        if let Some(k) = key_of(self.ent(ri)) {
            if let Some(kc) = self.find(k) {
                return Some(self.bqnv(kc));
            }
        }
        if self.need_idx && self.world_shifted {
            return Some("anoIdx".to_string());
        }
        None
    }

    // Inputs: a fold head as written and the rendered BQN operand expression.  Output: the BQN
    // text computing that fold over the operand, or None when the head names a registered
    // reducer or a prefix machine, whose rendering each fold site spells for its own position.
    // The rendering realizes the unseeded left recurrence, so a non-associative head such as
    // subtraction or division yields the exact ordered result.  The helper declaration the
    // rendering depends on is registered in the prologue.  Reductions the emitter performs for
    // its own bookkeeping are ordinary BQN reductions and never come through here; only a fold
    // written in Ano carries this contract.
    fn render_fold(&mut self, op: &str, operand: &str) -> Option<String> {
        let desc = desc_of(op, reducer::Form::Fold)?;
        let (call, declarations) = reducer::render_fold(&desc, operand)?;
        for declaration in declarations {
            self.need_declaration(&declaration);
        }
        Some(call)
    }

    // Inputs: a CANONICAL head spelling and the scan form it was resolved against.  Output: the
    // BQN scan expression for that head, with the step's own declaration registered in the
    // prologue; None when the registry supplies the step.  BQN's scan modifier is already the
    // left recurrence, so the only thing a scan owes beyond the glyph is that declaration — the
    // char instances scan through a declared step because `⌈` and `⌊` refuse characters.
    fn render_scan(&mut self, op: &str, form: reducer::Form) -> Option<String> {
        let desc = desc_of(op, form)?;
        let glyph = desc.scan_glyph()?;
        if let Some(declaration) = desc.step_declaration() {
            self.need_declaration(&declaration);
        }
        Some(glyph)
    }

    // Inputs: none.  Output: the BQN name of the mean machine's finish, with the declarations it
    // depends on registered in the prologue.  The finish sums its payload in the machine's own
    // order, so `avg/` is the last prefix of `avg\` rather than BQN's right-folded mean.
    fn render_mean(&mut self) -> &'static str {
        let (name, declarations) = reducer::render_mean();
        for declaration in declarations {
            self.need_declaration(&declaration);
        }
        name
    }

    // Inputs: one BQN helper declaration a rendering depends on.  Output: (); the declaration
    // joins the prologue, which the final assembly flushes into the program preamble directly
    // after the `anoSel ← ⟨⟩` line.  Entries deduplicate and hold insertion order, so one
    // program's text is byte-identical across runs.
    fn need_declaration(&mut self, declaration: &str) {
        if !self.decls.iter().any(|held| held == declaration) {
            self.decls.push(declaration.to_string());
        }
    }

    // The prologue flush, run once by the final assembly: every declaration a rendering asked
    // for, newline-terminated, spliced at the offset the fixture reserved for it.
    fn flush_declarations(&mut self) {
        let decls = std::mem::take(&mut self.decls);
        let mut block = String::new();
        for declaration in decls {
            block.push_str(&declaration);
            block.push('\n');
        }
        self.out.insert_str(self.decl_at, &block);
    }

    // Inputs: a registry entry index. Output: the endpoint's target carrier and whether it is
    // functional, or None when the entry is not a relationship surface a write must seal —
    // exactly the surfaces relationship::validate_registry seals in a fixture.
    fn seal_policy(&self, ri: usize) -> R<Option<(crate::relationship::TargetCarrier, bool)>> {
        Ok(match &self.ent(ri).kind {
            RegEntryKind::Rel { key_of, .. } => {
                Some((crate::relationship::carrier_for(self.reg, key_of.as_deref())?, true))
            }
            // an inverse fiber is derived from the endpoint that was already sealed on write
            RegEntryKind::SRel { key_of, inv_of: None, .. } => {
                Some((crate::relationship::carrier_for(self.reg, key_of.as_deref())?, false))
            }
            _ => None,
        })
    }

    // Inputs: a registry entry index and the rendered value being committed to it. Output: ();
    // the value is published to that entry's variable, through the validity seal when the entry
    // is a relationship or numeric carrier. Every commit to a world variable is published here,
    // so neither a malformed endpoint nor NaN can reach the world unstaged.
    fn publish(&mut self, ri: usize, value: String) -> R<()> {
        if self.seal_policy(ri)?.is_some() {
            // the seal owns its publication: staging, assertion, and commit are one unit
            self.stage_relationship_write(ri, &value)?;
            return Ok(());
        }
        if matches!(
            col_ty(self.ent(ri)),
            Some(ColType::Num | ColType::Bool | ColType::Nat | ColType::Int)
        ) {
            let name = self.ent(ri).name.clone();
            let pair = self.is_pair[ri];
            let stage = self.tv();
            self.stage(format!("{} ← {}", stage, value));
            let elements = if pair {
                format!("∾{}", stage)
            } else {
                stage.clone()
            };
            // x=x is false exactly for NaN. Infinity remains a value on num; refinements have
            // already retracted to their finite carrier before this publication seal runs.
            self.stage(format!(
                "\"numeric {}\" ! ∧´1∾(({})=({}))",
                name, elements, elements
            ));
            let var = self.bqnv(ri);
            self.stage(format!("{} ↩ {}", var, stage));
            return Ok(());
        }
        let var = self.bqnv(ri);
        self.stage(format!("{} ↩ {}", var, value));
        Ok(())
    }

    // Inputs: the registry index of a functional or set-valued relationship and the rendered
    // right-hand side being written to it.  Output: the BQN that stages that value into a fresh
    // binding, asserts its validity through crate::relationship::bqn_validity, and only then
    // publishes it to the relationship's variable.
    // Every write to a relationship passes through this path, so no relationship value reaches
    // the world unvalidated.  A refused write leaves the exact pre-state.  The staged binding's
    // name is lowercase, because BQN reads an uppercase initial as a function.
    fn stage_relationship_write(&mut self, ri: usize, rhs: &str) -> Result<String, Diag> {
        let Some((carrier, functional)) = self.seal_policy(ri)? else {
            return Err(fail(0, format!("'{}' is not a sealed relationship", self.ent(ri).name)));
        };
        let name = self.ent(ri).name.clone();
        let var = self.bqnv(ri);
        let stage = format!("anoRelStage{}", self.rel_seals);
        self.rel_seals += 1;
        self.stage(format!("{} ← {}", stage, rhs));
        // a set endpoint validates its members, so the fibers join before the elementwise test
        let value = if functional { stage.clone() } else { format!("∾{}", stage) };
        let predicate = crate::relationship::bqn_validity(&value, carrier, functional);
        // the assertion refuses before the publication runs, so the world keeps its pre-state
        self.stage(format!("\"relationship {}\" ! ∧´1∾({})", name, predicate));
        self.stage(format!("{} ↩ {}", var, stage));
        Ok(stage)
    }

    // Has-a-live-target guard for a bare rel.
    fn rel_guard(&self, ri: usize) -> String { let v = self.bqnv(ri); let key = self.rel_key(ri); crate::relationship::bqn_found(&v, key.as_deref()) }

    // Trace origin ids: the idCol ladder with the iota fallback sized by the REL column.
    fn trace_ids(&self, rel_expr: &str) -> String {
        let mut e = self.role("id");
        if e.is_none() {
            e = self.role("keys");
        }
        if let Some(i) = e {
            if matches!(self.ent(i).kind, RegEntryKind::Col { .. }) {
                return self.bqnv(i);
            }
        }
        if self.need_idx && self.world_shifted {
            return "anoIdx".to_string();
        }
        format!("(↕≠{})", rel_expr)
    }

    // A16: a predicate crossing reports over X, an effect crossing over S = the statement mask.
    // Inputs: the emission mode. Output: true when trace_sel must be conjoined into the mask.
    // The probe runs with in_effect false, so its discarded lines never narrow.
    fn trace_scoped(&self, m: Mode) -> bool {
        (m != Mode::World || self.in_effect) && !self.trace_sel.is_empty()
    }

    // Inputs: whether the statement mask was conjoined, the source line, the relation name.
    // Output: the self-describing suffix the runtime line carries. One record per crossing,
    // minted in staging order; domain can never disagree with the mask actually emitted.
    fn record_use(&mut self, scoped: bool, line: i32, relation: &str) -> String {
        let phase = if self.in_effect {
            crate::trace::TracePhase::Effect
        } else {
            crate::trace::TracePhase::Predicate
        };
        let domain = if scoped {
            crate::trace::TraceDomain::Selected
        } else {
            crate::trace::TraceDomain::Source
        };
        let site = format!("{}{}", self.site_kind, self.stmt);
        let id = self.plan.record(phase, domain, site, line.max(0) as u32, relation);
        format!("USE {} {} {}", id.0, phase.word(), domain.word())
    }

    // --trace dead-link hook: zero output without the flag; key None is the positional carrier,
    // g (None = total) the accumulated guard of earlier legs, m the emission mode.  Exactly ¯1
    // stays silent in both carriers.  DEAD is spelled as the non-sentinel complement of the hop's
    // own guard rather than derived a second time, so the two cannot partition the targets
    // differently (A17).
    fn trace_dead(&mut self, name: &str, key: Option<&str>, rel: &str, g: Option<&str>, m: Mode, line: i32) {
        if !self.dirs.trace {
            return;
        }
        let dead = format!("(¯1≠{})∧¬{}", rel, crate::relationship::bqn_found(rel, key));
        let mut mask = match g {
            Some(g) => format!("{}∧{}", g, dead),
            None => dead,
        };
        let scoped = self.trace_scoped(m);
        if scoped {
            mask = format!("{}∧{}", self.trace_sel, mask);
        }
        let dm = self.tv();
        self.stage(format!("{} ← {}", dm, mask));
        let usage = self.record_use(scoped, line, name);
        let tids = self.trace_ids(rel);
        self.stage(format!(
            "AnoTraceDead ⟨\"{}\", {}/{}, {}/{}, \"{}\"⟩",
            name, dm, tids, dm, rel, usage
        ));
    }

    // Gather a world-space column expr into the requested mode.
    fn in_mode(&self, w: String, m: Mode) -> String {
        match m {
            Mode::World => w,
            Mode::Sel => format!("({}/{})", self.sel_var, w),
            Mode::Copy => format!("({}/{}/{})", self.cnt_var, self.sel_var, w),
        }
    }

    // Presence mask for a column entry, world space; None when total.
    fn pres_of(&self, i: usize) -> Option<String> {
        if has_pres(self.ent(i)) { Some(self.presv(i)) } else { None }
    }

    // The world-space mask a derived tag denotes: present(carrier) ∧ carrier = value,
    // recomputed against the live column; Err on a dangling carrier.
    fn tag_mask(&mut self, ti: usize, line: i32) -> R<String> {
        let e = self.ent(ti);
        let RegEntryKind::Tag { col, carrier_ty, num, sym } = &e.kind else {
            return Err(fail(line, format!("derived tag '{}': carrier '{}' unregistered", e.name, "")));
        };
        let Some(ci) = self.find(col) else {
            return Err(fail(line, format!("derived tag '{}': carrier '{}' unregistered", e.name, col)));
        };
        let eq = if *carrier_ty == ColType::Sym {
            format!("((<\"{}\")≡¨{})", sym.as_deref().unwrap_or(""), self.bqnv(ci))
        } else {
            format!("({}={})", self.bqnv(ci), num_lit(*num))
        };
        Ok(if has_pres(self.ent(ci)) { format!("({}∧{})", self.presv(ci), eq) } else { eq })
    }

    // The C nd->name view of a node (spelling fields; "" where C left name unset).
    fn node_name(&self, nd: &Node) -> &'a str {
        match &nd.kind {
            NodeKind::Name(s) | NodeKind::Alias { look: s, .. } | NodeKind::Sym(s) | NodeKind::Str(s) => {
                self.rs(*s)
            }
            NodeKind::SetHop { rel } => self.rs(*rel),
            NodeKind::Call { callee, .. } => self.rs(*callee),
            NodeKind::CmpAny { name } => self.rs(*name),
            _ => "",
        }
    }
}

/* ---------- names as values, masks, grades, pipes ---------- */

impl<'a> Em<'a> {
    // Inputs: a resolved lookup name + the stem the source requested + line, mode, the resolver
    // request that produced it. Output: EV. Handles index/x/y/char specials, defs, then registry
    // entries by kind. Invariant: guard is world-space; req/lm only spell the source request back
    // in the unregistered refusal, so a moved `^name` never reads back as its target.
    fn emit_name_val(&mut self, sym: Symbol, req: Symbol, line: i32, m: Mode, lm: LookupMode) -> R<Ev> {
        let n = self.rs(sym);
        let mut ev = Ev::default();
        if n == "index" {
            match m {
                Mode::Copy => ev.v = self.idx_var.clone(),
                Mode::Sel => ev.v = format!("(↕+´{})", self.sel_var),
                Mode::World => ev.v = format!("(↕{})", self.fr_n()),
            }
            return Ok(ev);
        }
        if matches!(self.fr.kind, FrameKind::Lat | FrameKind::Board)
            && (n == "x" || n == "y")
            && self.find(n).is_none()
        {
            let wh = self.fr.w.wrapping_mul(self.fr.h);
            let w = if n.as_bytes()[0] == b'y' {
                format!("(⌊(↕{})÷{})", wh, self.fr.w)
            } else {
                format!("({}|↕{})", self.fr.w, wh)
            };
            ev.v = self.in_mode(w, m);
            return Ok(ev);
        }
        if n == "char" && self.fr.kind == FrameKind::Board {
            ev.v = self.in_mode(format!("brd{}", self.stmt), m);
            return Ok(ev);
        }
        if let Some(d) = self.find_def(sym) {
            return self.emit_val(def_body(d), m);
        }
        let Some(ei) = self.find(n) else {
            return Err(fail(line, format!("unregistered name '{}'", spelled(self.rs(req), lm))));
        };
        let e = self.ent(ei);
        match &e.kind {
            RegEntryKind::Col { ty, .. } | RegEntryKind::Field { ty, .. } => {
                let v = self.bqnv(ei);
                ev.pair = self.is_pair[ei];
                ev.sym = *ty == ColType::Sym;
                ev.g = self.pres_of(ei);
                ev.v = self.in_mode(v, m);
                Ok(ev)
            }
            RegEntryKind::Tag { .. } => {
                // Recomputed mask; no separate guard.
                let mk = self.tag_mask(ei, line)?;
                ev.v = self.in_mode(mk, m);
                Ok(ev)
            }
            RegEntryKind::Rel { .. } => {
                // A10: the bare rel evaluates the same foundness guard a hop does, so the
                // guard evaluation is a crossing and owes its DEAD report.
                let rel = self.bqnv(ei);
                let key = self.rel_key(ei);
                let rname = self.ent(ei).name.as_str();
                self.trace_dead(rname, key.as_deref(), &rel, None, m, line);
                ev.v = self.in_mode(rel, m);
                ev.g = Some(self.rel_guard(ei));
                ev.rel_ent = Some(ei);
                Ok(ev)
            }
            RegEntryKind::AliasMask { .. } => {
                ev.v = self.in_mode(self.bqnv(ei), m);
                Ok(ev)
            }
            RegEntryKind::Bind { kind, vals } => {
                match kind {
                    BindKind::Num => {
                        ev.v = num_lit(vals.first().copied().unwrap_or(0.0));
                        ev.unit = true;
                    }
                    BindKind::Point => {
                        ev.v = format!(
                            "(<{}‿{})",
                            num_lit(vals.first().copied().unwrap_or(0.0)),
                            num_lit(vals.get(1).copied().unwrap_or(0.0))
                        );
                        ev.pair = true;
                        ev.unit = true;
                    }
                    BindKind::Entity => {
                        ev.v = num_lit(vals.first().copied().unwrap_or(0.0));
                        ev.unit = true;
                    }
                    BindKind::Mask => ev.v = self.in_mode(self.bqnv(ei), m),
                    BindKind::Vec => {
                        ev.v = self.bqnv(ei);
                        ev.unit = true;
                    }
                }
                Ok(ev)
            }
            RegEntryKind::SRel { .. } => {
                ev.v = self.bqnv(ei);
                ev.unit = true;
                Ok(ev)
            }
            RegEntryKind::Proto { .. } => Err(fail(
                line,
                format!("proto '{}' in value position: a proto is spawned, never read", n),
            )),
            RegEntryKind::Fn { .. } | RegEntryKind::TypedFn { .. } => {
                Err(fail(line, format!("name '{}' (fn) in value position", n)))
            }
            RegEntryKind::Array { .. } => Err(fail(line, format!("array '{}' needs a host attachment", n))),
            RegEntryKind::Service { .. } => Err(fail(line, format!("service '{}' is not a value", n))),
            RegEntryKind::Enum { .. } => Err(fail(line, format!("enum '{}' is a type, not a value", n))),
            RegEntryKind::Ctor { .. } => {
                Err(fail(line, format!("constructor '{}' must be called", n)))
            }
        }
    }

    // A resolved name as a mask: bool col -> pres∧values; value col -> presence; static alias
    // mask/bind mask.  req/lm spell the source request back in the refusals; when the overlay
    // moved the lookup, a kind mismatch names both the request and the target it reached.
    fn emit_name_mask(&mut self, sym: Symbol, req: Symbol, line: i32, lm: LookupMode) -> R<String> {
        let n = self.rs(sym);
        let rq = self.rs(req);
        if let Some(d) = self.find_def(sym) {
            return self.emit_mask(def_body(d));
        }
        let Some(ei) = self.find(n) else {
            return Err(fail(line, format!("unregistered mask name '{}'", spelled(rq, lm))));
        };
        let v = self.bqnv(ei);
        let e = self.ent(ei);
        match &e.kind {
            RegEntryKind::Col { ty, pres, .. } => {
                if *ty == ColType::Bool {
                    return Ok(if pres.is_some() { format!("({}∧{})", self.presv(ei), v) } else { v });
                }
                Ok(if pres.is_some() { self.presv(ei) } else { format!("(1¨{})", v) })
            }
            RegEntryKind::Field { ty, .. } => {
                if *ty == ColType::Bool { Ok(v) } else { Ok(format!("(1¨{})", v)) }
            }
            RegEntryKind::Tag { .. } => self.tag_mask(ei, line),
            RegEntryKind::AliasMask { .. } => Ok(v),
            RegEntryKind::Bind { kind, vals } => match kind {
                BindKind::Mask => Ok(v),
                BindKind::Entity => {
                    let key = num_lit(vals.first().copied().unwrap_or(0.0));
                    Ok(format!("({}={})", self.id_col(), key))
                }
                k if moved(n, rq, lm) => Err(fail(
                    line,
                    format!(
                        "'^{}' resolves to binding '{}' ({}), not a mask",
                        rq,
                        n,
                        bind_kind_str(*k)
                    ),
                )),
                k => Err(fail(line, format!("binding '{}' ({}) as mask", n, bind_kind_str(*k)))),
            },
            RegEntryKind::Rel { .. } => {
                // mask position is a predicate crossing over X
                let key = self.rel_key(ei);
                let rname = self.ent(ei).name.as_str();
                self.trace_dead(rname, key.as_deref(), &v, None, Mode::World, line);
                Ok(self.rel_guard(ei))
            }
            _ if moved(n, rq, lm) => Err(fail(
                line,
                format!("'^{}' resolves to '{}', which cannot be a mask", rq, n),
            )),
            _ => Err(fail(line, format!("'{}' cannot be a mask", n))),
        }
    }

    // grade [desc] key [@ scope] -> ids expr in frame space.
    fn emit_grade_ids(&mut self, key: &Node, desc: bool) -> R<String> {
        let (key, scope): (&Node, Option<&Node>) =
            if let NodeKind::Scope { l, r, .. } = &key.kind { (l, Some(r)) } else { (key, None) };
        let g = if desc { "⍒" } else { "⍋" };
        if let Some(sc) = scope {
            if !matches!(sc.kind, NodeKind::Shape(_)) {
                // mask scope: ids in world space
                let msk = self.emit_mask(sc)?;
                let kv = self.emit_val(key, Mode::World)?;
                return Ok(format!("((/{})⊏˜{}({}/{}))", msk, g, msk, kv.v));
            }
        }
        let kv = self.emit_val(key, Mode::World)?;
        Ok(format!("({}{})", g, kv.v))
    }

    // Pipeline view: base mask + ordered indices into the filtered space (or iota base).
    fn emit_pipe(&mut self, nd: &Node) -> R<View> {
        let NodeKind::Pipe { src, stages } = &nd.kind else {
            return Err(fail(nd.line, "unsupported pipeline stage"));
        };
        let mut vw = View::default();
        if let NodeKind::IotaX(inner) = &src.kind {
            let n = self.emit_val(inner, Mode::World)?;
            vw.is_iota = true;
            vw.idx = format!("(↕{})", n.v);
        } else {
            let msk = self.emit_mask(src)?;
            let t = self.tv();
            self.stage(format!("{} ← {}", t, msk));
            vw.idx = format!("(↕+´{})", t);
            vw.base = Some(t);
        }
        for st in stages {
            match &st.kind {
                NodeKind::OrderBy { key, desc } => {
                    let kv = self.emit_val(key, Mode::World)?;
                    let keyf = match &vw.base {
                        Some(b) => format!("({}/{})", b, kv.v),
                        None => kv.v.clone(),
                    };
                    vw.idx = format!(
                        "(({})⊏˜{}(({})⊏{}))",
                        vw.idx,
                        if *desc { "⍒" } else { "⍋" },
                        vw.idx,
                        keyf
                    );
                }
                NodeKind::Take { k } => {
                    vw.idx = format!("({}↑{})", *k as i32, vw.idx);
                }
                NodeKind::Expand(inner) => {
                    let cv = self.emit_val(inner, Mode::World)?;
                    vw.expand_cnt = Some(match &vw.base {
                        Some(b) => format!("({}/{})", b, cv.v),
                        None => cv.v,
                    });
                }
                NodeKind::Call { callee, args } => {
                    let cn = self.rs(*callee);
                    let Some(fe) = self.find(cn) else {
                        return Err(fail(st.line, format!("unregistered callable '{}'", cn)));
                    };
                    match self.ent(fe).kind {
                        RegEntryKind::Fn { .. } => {}
                        RegEntryKind::TypedFn { .. } => {
                            return Err(fail(
                                st.line,
                                format!(
                                    "typed pipeline callable '{}' needs a declared domain signature",
                                    cn
                                ),
                            ));
                        }
                        _ => {
                            return Err(fail(
                                st.line,
                                format!("declaration '{}' is not callable", cn),
                            ));
                        }
                    }
                    let mut a = format!("⟨{}", vw.idx);
                    for k in args {
                        let av = self.emit_val(k, Mode::World)?;
                        a = format!("{}, {}", a, av.v);
                    }
                    vw.idx = format!("({} {}⟩)", self.fnv(fe), a);
                    vw.is_iota = true;
                    vw.base = None;
                }
                _ => return Err(fail(st.line, "unsupported pipeline stage")),
            }
        }
        Ok(vw)
    }

    // View ids in frame space (world row ids).
    fn view_world_ids(&self, vw: &View) -> String {
        match &vw.base {
            Some(b) => format!("((/{})⊏˜{})", b, vw.idx),
            None => vw.idx.clone(),
        }
    }
}

/* ---------- folds and scans ---------- */

impl<'a> Em<'a> {
    // The set-hop fiber expression for a rel name: srel var, or a key-valued CT_NUM column's
    // inverse image over the stable-id column (staged).  Output: the iteration variable and a
    // DURABLE self-contained spelling of the same fibers — a guard may name only durable
    // variables, since the assignment probe discards every staged line.
    fn fiber_var(&mut self, sym: Symbol, line: i32) -> R<(String, String)> {
        let n = self.rs(sym);
        if let Some(ei) = self.find(n) {
            match &self.ent(ei).kind {
                RegEntryKind::SRel { .. } => {
                    let v = self.bqnv(ei);
                    return Ok((v.clone(), v));
                }
                RegEntryKind::Col { ty: ColType::Num | ColType::Nat | ColType::Int, .. } => {
                    let t = self.tv();
                    let idc = self.id_col();
                    let v = self.bqnv(ei);
                    self.stage(format!("{} ← {{/{}=𝕩}}¨{}", t, v, idc));
                    return Ok((t, format!("({{/{}=𝕩}}¨{})", v, idc)));
                }
                _ => {}
            }
        }
        Err(fail(line, format!("'{}' is not a set-valued relationship or a key column", n)))
    }

    // The key space stored fibers resolve through; None when they already hold row indices.
    fn fiber_key(&self, sym: Symbol) -> Option<String> {
        let ei = self.find(self.rs(sym))?;
        let RegEntryKind::SRel { fib: fibers, key_of: ko, .. } = &self.ent(ei).kind else {
            return None;
        };
        if fibers.len() != self.reg.n as usize {
            return None;
        }
        match ko {
            Some(k) => self.find(k).map(|kc| self.bqnv(kc)),
            None if self.world_shifted => Some(self.id_col()),
            None => None,
        }
    }

    // Translate stored srel fibers to CURRENT row indices when the key space can disagree:
    // declared key always, the idx key once shifted; else pass through untouched.
    fn fiber_rows(&mut self, sym: Symbol, fib: &mut String, m: Mode, line: i32) {
        let Some(ei) = self.find(self.rs(sym)) else { return };
        let Some(key) = self.fiber_key(sym) else { return };
        let name = self.ent(ei).name.as_str();
        // --trace: a dead member is a dead link crossed inside the fiber; an effect crossing
        // reports only over the statement mask
        if self.dirs.trace {
            let scoped = self.trace_scoped(m);
            let usage = self.record_use(scoped, line, name);
            let tids = self.trace_ids(fib);
            let (ids, src) = if scoped {
                (
                    format!("({}/{})", self.trace_sel, tids),
                    format!("({}/{})", self.trace_sel, fib),
                )
            } else {
                (tids, fib.clone())
            };
            self.stage(format!(
                "{} {{m←(≠{})≤{}⊐𝕩 ⋄ AnoTraceDead ⟨\"{}\", (+´m)⥊𝕨, m/𝕩, \"{}\"⟩}}¨ {}",
                ids, key, key, name, usage, src
            ));
        }
        let t = self.tv();
        self.stage(format!("{} ← {{k←{}⊐𝕩 ⋄ (k<≠{})/k}}¨{}", t, key, key, fib));
        *fib = t;
    }

    // --trace empty-fiber hook: effect position scopes to the statement mask, a predicate
    // stays whole-column.
    fn trace_empty(&mut self, name: &str, fib: &str, m: Mode, line: i32) {
        if !self.dirs.trace {
            return;
        }
        let scoped = self.trace_scoped(m);
        let mask = if scoped {
            format!("({}∧0=≠¨{})", self.trace_sel, fib)
        } else {
            format!("(0=≠¨{})", fib)
        };
        let usage = self.record_use(scoped, line, name);
        let tids = self.trace_ids(fib);
        self.stage(format!("AnoTraceEmpty ⟨\"{}\", {}/{}, \"{}\"⟩", name, mask, tids, usage));
    }

    // The nonempty-fiber guard, spelled against DURABLE variables only: a keyed fiber counts
    // its found members, an unkeyed one its raw length.
    fn fiber_nonempty(&self, sym: Symbol, raw: &str) -> String {
        match self.fiber_key(sym) {
            Some(key) => format!("(0<{{+´(≠{})>{}⊐𝕩}}¨{})", key, key, raw),
            None => format!("(0<≠¨{})", raw),
        }
    }

    // Gamma fold: fold/ rel'.Comp | fold/ (rel' & pred) | fold/ rel' -> per-source column + guard.
    fn emit_gamma(&mut self, op: &str, operand: &Node, m: Mode) -> R<Ev> {
        let mut ev = Ev::default();
        match &operand.kind {
            NodeKind::SetHop { rel } => {
                // #/ attackers'
                let (mut fib, _raw) = self.fiber_var(*rel, operand.line)?;
                self.fiber_rows(*rel, &mut fib, m, operand.line);
                let body = format!("≠¨{}", fib);
                if op != "#" {
                    // other folds over bare fiber make no sense
                    return Err(fail(operand.line, format!("bare rel' under {}/", op)));
                }
                ev.v = self.in_mode(format!("({})", body), m);
                Ok(ev)
            }
            NodeKind::Hop { l, r } if matches!(l.kind, NodeKind::SetHop { .. }) => {
                // fold/ rel'.Comp
                let rel = fiber_sym(l);
                let (mut fib, raw) = self.fiber_var(rel, l.line)?;
                self.fiber_rows(rel, &mut fib, m, l.line);
                let fb_name = self.rs(rel);
                let nonempty = self.fiber_nonempty(rel, &raw);
                let cv = self.emit_val(r, Mode::World)?;
                // a machine and a registered head render nothing here, and register nothing
                let call = self.render_fold(op, &format!("𝕩⊏{}", cv.v));
                let t = self.tv();
                if op == "avg" {
                    self.trace_empty(fb_name, &fib, m, l.line);
                    let mean = self.render_mean();
                    self.stage(format!("{} ← {{0=≠𝕩 ? 0 ; {} 𝕩⊏{}}}¨{}", t, mean, cv.v, fib));
                    ev.g = Some(nonempty);
                    ev.gv = true;
                } else if op == "#" {
                    // the count machine's finish over the presence stream the normalizer wrapped
                    // onto the component: a 0/1 mask sums exactly in either order, so this
                    // reduction needs no reversal
                    self.stage(format!("{} ← {{+´𝕩⊏{}}}¨{}", t, cv.v, fib));
                } else if let Some(call) = call {
                    if fold_has_id(op) {
                        self.stage(format!("{} ← {{{}}}¨{}", t, call, fib));
                    } else {
                        // max/min: no identity, guard empties
                        self.trace_empty(fb_name, &fib, m, l.line);
                        self.stage(format!("{} ← {{0=≠𝕩 ? 0 ; {}}}¨{}", t, call, fib));
                        ev.g = Some(nonempty);
                        ev.gv = true;
                    }
                } else {
                    if let Some(ei) = self.find(op) {
                        if matches!(self.ent(ei).kind, RegEntryKind::Fn { .. } | RegEntryKind::TypedFn { .. }) {
                            return Err(fail(
                                operand.line,
                                format!("named reducer '{}' over fibers is not yet supported", op),
                            ));
                        }
                    }
                    return Err(fail(operand.line, format!("unknown reducer '{}'", op)));
                }
                ev.v = self.in_mode(t, m);
                Ok(ev)
            }
            NodeKind::And(l, r) if matches!(l.kind, NodeKind::SetHop { .. }) => {
                // fold/ (rel' & pred)
                let rel = fiber_sym(l);
                let (mut fib, _raw) = self.fiber_var(rel, l.line)?;
                self.fiber_rows(rel, &mut fib, m, l.line);
                let pm = self.emit_mask(r)?;
                let tp = self.tv();
                self.stage(format!("{} ← {}", tp, pm));
                let t = self.tv();
                match op {
                    // the count machine's finish: a 0/1 mask sums exactly in either order, so this
                    // reduction needs no reversal — the operand is what makes it free, not the
                    // machine, whose payload accumulations do carry the ordered recurrence
                    "#" => self.stage(format!("{} ← {{+´𝕩⊏{}}}¨{}", t, tp, fib)),
                    "|" | "&" => {
                        let Some(call) = self.render_fold(op, &format!("𝕩⊏{}", tp)) else {
                            return Err(fail(operand.line, format!("fold {}/ over filtered fiber", op)));
                        };
                        self.stage(format!("{} ← {{{}}}¨{}", t, call, fib));
                    }
                    _ => return Err(fail(operand.line, format!("fold {}/ over filtered fiber", op))),
                }
                ev.v = self.in_mode(t, m);
                Ok(ev)
            }
            _ => Err(fail(operand.line, "unsupported gamma operand")),
        }
    }

    // Scoped-global fold -> scalar EV (unit), except gamma/@row columns.
    fn emit_fold(&mut self, op_sym: Symbol, operand: &Node, line: i32, m: Mode) -> R<Ev> {
        let op = self.rs(op_sym);
        let mut ev = Ev::default();
        ev.unit = true;
        // Adj@row: per-row fold over a fiber-matrix column
        if let NodeKind::Scope { l, r, .. } = &operand.kind {
            if matches!(&r.kind, NodeKind::Name(s) if self.rs(*s) == "row") && self.find("row").is_none() {
                let fs = fiber_sym(l);
                let (mut fib, raw) = self.fiber_var(fs, l.line)?;
                let nonempty = self.fiber_nonempty(fs, &raw);
                let fb_name = self.rs(fs);
                self.fiber_rows(fs, &mut fib, m, l.line);
                let Some(call) = self.render_fold(op, "𝕩") else {
                    return Err(fail(line, format!("fold {}/ @row", op)));
                };
                ev.unit = false;
                if fold_has_id(op) {
                    ev.v = self.in_mode(format!("({{{}}}¨{})", call, fib), m);
                    return Ok(ev);
                }
                // no identity: an empty row has no answer (A12), so the row drops through the
                // validity channel instead of aborting on BQN's missing fold identity
                self.trace_empty(fb_name, &fib, m, l.line);
                let t = self.tv();
                self.stage(format!("{} ← {{0=≠𝕩 ? 0 ; {}}}¨{}", t, call, fib));
                ev.g = Some(nonempty);
                ev.gv = true;
                ev.v = self.in_mode(t, m);
                return Ok(ev);
            }
        }
        if is_gamma_operand(operand) {
            return self.emit_gamma(op, operand, m);
        }
        // split operand @ scope
        let (x, scope): (&Node, Option<&Node>) =
            if let NodeKind::Scope { l, r, .. } = &operand.kind { (l, Some(r)) } else { (operand, None) };
        let gathered: String;
        let cnt: String;
        match scope {
            None => {
                // #/ (mask) or fold over full frame
                if op == "#" {
                    let msk = self.emit_mask(x)?;
                    ev.v = format!("(+´{})", msk);
                    return Ok(ev);
                }
                let xv = self.emit_val(x, Mode::World)?;
                let v = match &xv.g {
                    Some(g) => format!("(({})/{})", g, xv.v),
                    None => xv.v.clone(),
                };
                cnt = format!("(≠{})", v);
                gathered = v;
            }
            Some(sc) if matches!(sc.kind, NodeKind::Shape(_)) => {
                // fold over a lattice field
                let xv = self.emit_val(x, Mode::World)?;
                cnt = format!("(≠{})", xv.v);
                gathered = xv.v;
            }
            Some(sc) if matches!(sc.kind, NodeKind::Pipe { .. }) => {
                let vw = self.emit_pipe(sc)?;
                let xv = self.emit_val(x, Mode::World)?;
                let xf = match &vw.base {
                    Some(b) => format!("({}/{})", b, xv.v),
                    None => xv.v.clone(),
                };
                gathered = format!("(({})⊏{})", vw.idx, xf);
                cnt = format!("(≠{})", vw.idx);
            }
            Some(sc) => {
                // mask scope
                let msk = self.emit_mask(sc)?;
                let xv = self.emit_val(x, Mode::World)?;
                let m2 = match xv.g {
                    Some(g) => g_and(Some(msk), Some(g)).unwrap_or_default(),
                    None => msk,
                };
                if op == "#" {
                    ev.v = format!("(+´{})", m2);
                    return Ok(ev);
                }
                gathered = format!("({}/{})", m2, xv.v);
                cnt = format!("(+´{})", m2);
            }
        }
        if op == "#" {
            ev.v = cnt;
            return Ok(ev);
        }
        if op == "avg" {
            let t = self.tv();
            let mean = self.render_mean();
            self.stage(format!("{} ← {{0=≠𝕩 ? 0 ; {} 𝕩}} {}", t, mean, gathered));
            ev.v = t;
            ev.g = Some(format!("(0<{})", cnt));
            ev.gv = true;
            return Ok(ev);
        }
        // an identity answers the empty gather itself; without one the gather is guarded, and
        // the fold then sees only the nonempty case
        let identity = fold_has_id(op);
        let operand = if identity { gathered.as_str() } else { "𝕩" };
        let Some(call) = self.render_fold(op, operand) else {
            // named reducer: the registry fn steps the same left recurrence
            let fe = self.find(op).filter(|&i| matches!(self.ent(i).kind, RegEntryKind::Fn { .. } | RegEntryKind::TypedFn { .. }));
            let Some(fe) = fe else {
                return Err(fail(line, format!("unknown reducer '{}'", op)));
            };
            let t = self.tv();
            let fv = self.fnv(fe);
            let call = reducer::render_named_fold(&fv, "𝕩");
            self.stage(format!("{} ← {{0=≠𝕩 ? 0 ; {}}} {}", t, call, gathered));
            ev.v = t;
            ev.g = Some(format!("(0<{})", cnt));
            ev.gv = true;
            return Ok(ev);
        };
        if !identity {
            let t = self.tv();
            self.stage(format!("{} ← {{0=≠𝕩 ? 0 ; {}}} {}", t, call, gathered));
            ev.v = t;
            ev.g = Some(format!("(0<{})", cnt));
            ev.gv = true;
            return Ok(ev);
        }
        ev.v = format!("({})", call);
        Ok(ev)
    }

    // Scans: result is an ordered column over the scan's scope, returned as a flat value.
    fn emit_scan(&mut self, op_sym: Symbol, operand: &Node, line: i32) -> R<Ev> {
        let op = self.rs(op_sym);
        let mut ev = Ev::default();
        // a per-fiber scan needs a ragged result representation that does not exist yet; the
        // count machine's presence wrapper and an @scope do not change what the operand IS
        let mut core = operand;
        if let NodeKind::Scope { l, .. } = &core.kind {
            core = l;
        }
        while let NodeKind::Not(inner) = &core.kind {
            core = inner;
        }
        if is_gamma_operand(core) {
            return Err(fail(line, "scan over fibers is not yet supported"));
        }
        let (x, scope): (&Node, Option<&Node>) =
            if let NodeKind::Scope { l, r, .. } = &operand.kind { (l, Some(r)) } else { (operand, None) };
        let gl: String = match self.render_scan(op, reducer::Form::Scan) {
            Some(gl) => gl,
            None => {
                // named reducer scan: registry fn accumulates pairwise; the empty scope
                // yields the empty column — a scan is length-preserving, no identity consulted
                let fe = self.find(op).filter(|&i| matches!(self.ent(i).kind, RegEntryKind::Fn { .. } | RegEntryKind::TypedFn { .. }));
                let Some(fe) = fe else {
                    return Err(fail(line, format!("internal: no scan descriptor for '{}'", op)));
                };
                format!("{}`", self.fnv(fe))
            }
        };
        let xv = self.emit_val(x, Mode::World)?;
        if let Some(sc) = scope {
            if let NodeKind::Shape(dims) = &sc.kind {
                let w = dims.first().map(num_of).unwrap_or(0.0) as i32;
                let h = if dims.len() > 1 { num_of(&dims[1]) as i32 } else { 1 };
                ev.v = format!("(⥊{}({}‿{}⥊{}))", gl, h, w, xv.v);
                ev.domain = Some(format!("(↕{})", w.wrapping_mul(h)));
                return Ok(ev);
            }
            if matches!(sc.kind, NodeKind::Pipe { .. }) {
                let vw = self.emit_pipe(sc)?;
                let xf = match &vw.base {
                    Some(b) => format!("({}/{})", b, xv.v),
                    None => xv.v.clone(),
                };
                ev.v = format!("({}({})⊏{})", gl, vw.idx, xf);
                let domain = self.view_world_ids(&vw);
                ev.along = Some(domain.clone());
                ev.domain = Some(domain);
                return Ok(ev);
            }
            // mask scope -> compress; id-list scope (iota arithmetic / vec bind) -> index
            let mut idlist = contains_iota(sc);
            if !idlist {
                if let NodeKind::Name(s) | NodeKind::Alias { look: s, .. } = &sc.kind {
                    if let Some(ei) = self.find(self.rs(*s)) {
                        if matches!(self.ent(ei).kind, RegEntryKind::Bind { kind: BindKind::Vec, .. }) {
                            idlist = true;
                        }
                    }
                }
            }
            if idlist {
                let sv = self.emit_val(sc, Mode::World)?;
                let domain = sv.v.clone();
                ev.v = format!("({}({})⊏{})", gl, domain, xv.v);
                ev.along = Some(domain.clone());
                ev.domain = Some(domain);
                return Ok(ev);
            }
            let msk = self.emit_mask(sc)?;
            ev.v = format!("({}{}/{})", gl, msk, xv.v);
            ev.domain = Some(format!("({}/↕{})", msk, self.fr_n()));
            return Ok(ev);
        }
        ev.v = format!("({}{})", gl, xv.v);
        ev.domain = Some(format!("(↕{})", self.fr_n()));
        Ok(ev)
    }
}

/* ---------- general value expressions ---------- */

impl<'a> Em<'a> {
    fn mode_domain(&self, m: Mode) -> Option<String> {
        match m {
            Mode::World => Some(format!("(↕{})", self.fr_n())),
            Mode::Sel => Some(format!("({}/↕{})", self.sel_var, self.fr_n())),
            Mode::Copy => None,
        }
    }

    fn emit_call(&mut self, callee: Symbol, args: &[Node], line: i32, m: Mode) -> R<Ev> {
        let name = self.rs(callee);
        let mut ev = Ev::default();
        if name == "rank" && self.find(name).is_none_or(|index| !claims_call_namespace(&self.ent(index).kind)) {
            if let Some(a0) = args.first() {
                let a = self.emit_val(a0, m)?;
                ev.v = format!("(AnoRank {})", a.v);
                return Ok(ev);
            }
        }
        let e = self.find(name);
        let fnn: String = match e {
            Some(i)
                if matches!(
                    self.ent(i).kind,
                    RegEntryKind::Fn { .. } | RegEntryKind::TypedFn { .. }
                ) =>
            {
                self.fnv(i)
            }
            Some(_) => return Err(fail(line, format!("declaration '{}' is not callable", name))),
            None if name == "abs" => "|".to_string(),
            None if name == "sin" => "•math.Sin".to_string(),
            None => return Err(fail(line, format!("unregistered callable '{}'", name))),
        };
        match args.len() {
            1 => {
                let a = self.emit_val(&args[0], m)?;
                ev.g = a.g;
                ev.gv = a.gv;
                ev.unit = a.unit;
                ev.along = a.along;
                ev.domain = a.domain;
                ev.v = format!("({}¨{})", fnn, a.v);
                Ok(ev)
            }
            2 => {
                let a = self.emit_val(&args[0], m)?;
                let b = self.emit_val(&args[1], m)?;
                let implicit = self.mode_domain(m);
                ev.domain = scan_domain_join(&a, &b, implicit.as_deref(), line)?;
                ev.along = scan_order_join(&a, &b, line)?;
                ev.g = g_and(a.g, b.g);
                ev.gv = a.gv || b.gv;
                ev.unit = a.unit && b.unit;
                ev.v = format!("({} {}¨{})", a.v, fnn, b.v);
                Ok(ev)
            }
            n => Err(fail(line, format!("callable arity {} unsupported", n))),
        }
    }

    // Hop chains: rel.Comp, Player.pos, ^cursor.pos, prev.prev.X, neighbor(clamp).X, pos.x.
    fn emit_hop(&mut self, nd: &Node, m: Mode) -> R<Ev> {
        let NodeKind::Hop { l: base, r: field } = &nd.kind else {
            return Err(fail(nd.line, "unsupported hop"));
        };
        let mut ev = Ev::default();
        let lat = matches!(self.fr.kind, FrameKind::Lat | FrameKind::Board);
        // count prev chain
        let mut b: &Node = nd;
        while let NodeKind::Hop { l, .. } = &b.kind {
            b = l;
        }
        if matches!(&b.kind, NodeKind::Name(s) if self.rs(*s) == "prev") && self.find("prev").is_none() {
            let mut prevs = 0usize;
            let mut w: &Node = nd;
            while let NodeKind::Hop { l, .. } = &w.kind {
                if matches!(l.kind, NodeKind::Hop { .. }) {
                    prevs += 1;
                    w = l;
                } else {
                    break;
                }
            }
            prevs += 1;
            let cv = self.emit_val(field, Mode::World)?;
            let sh = if lat {
                // rank-2 prev: » shifts a whole zero row along the leading axis
                let glyphs = "»".repeat(prevs.min(7));
                format!("(⥊{}({}‿{}⥊{}))", glyphs, self.fr.h, self.fr.w, cv.v)
            } else {
                let mut sh = cv.v.clone();
                for _ in 0..prevs {
                    sh = format!("(»{})", sh);
                }
                sh
            };
            ev.v = self.in_mode(sh, m);
            return Ok(ev);
        }
        if matches!(&b.kind, NodeKind::Name(s) if self.rs(*s).starts_with("neighbor"))
            && self.find("neighbor").is_none()
        {
            let cv = self.emit_val(field, Mode::World)?;
            let sh = if lat {
                format!("(⥊AnoNbrClamp({}‿{}⥊{}))", self.fr.h, self.fr.w, cv.v)
            } else {
                format!("(AnoNbrClamp {})", cv.v)
            };
            ev.v = self.in_mode(sh, m);
            return Ok(ev);
        }
        if matches!(&b.kind, NodeKind::Call { callee, .. } if self.rs(*callee).starts_with("neighbor")) {
            let cv = self.emit_val(field, Mode::World)?;
            let sh = format!("(⥊AnoNbrClamp({}‿{}⥊{}))", self.fr.h, self.fr.w, cv.v);
            ev.v = self.in_mode(sh, m);
            return Ok(ev);
        }
        // pos.x / pos.y field projection on a pair column
        if let NodeKind::Name(fs) = &field.kind {
            let fname = self.rs(*fs);
            if fname == "x" || fname == "y" {
                let e = if let NodeKind::Name(bs) | NodeKind::Alias { look: bs, .. } = &base.kind {
                    self.find(self.rs(*bs))
                } else {
                    None
                };
                if let Some(ei) = e {
                    if self.is_pair[ei] {
                        let i = (fname.as_bytes()[0] == b'y') as i32;
                        ev.v = self.in_mode(format!("({}⊸⊑¨{})", i, self.bqnv(ei)), m);
                        return Ok(ev);
                    }
                }
            }
        }
        // Singleton roots have one declared face. Entity bindings carry stable keys and resolve
        // through the key column at every mirror-read; masks and numeric constants are not handles.
        if let NodeKind::Name(bs) | NodeKind::Alias { look: bs, .. } = &base.kind {
            let be = self.find(self.rs(*bs));
            let fe = if let NodeKind::Name(fs) = &field.kind { self.find(self.rs(*fs)) } else { None };
            if let Some(bi) = be {
                let binding_name = self.ent(bi).name.clone();
                if let RegEntryKind::Bind { kind: BindKind::Point, vals } = &self.ent(bi).kind {
                    if names_eq(self.node_name(field), "pos") {
                        ev.v = format!(
                            "(<{}‿{})",
                            num_lit(vals.first().copied().unwrap_or(0.0)),
                            num_lit(vals.get(1).copied().unwrap_or(0.0))
                        );
                        ev.pair = true;
                        ev.unit = true;
                        return Ok(ev);
                    }
                    return Err(fail(
                        nd.line,
                        format!("point binding '{}' exposes only '.pos'", binding_name),
                    ));
                }
                if let Some(fi) = fe {
                    match &self.ent(bi).kind {
                        RegEntryKind::Bind {
                            kind: BindKind::Entity,
                            vals,
                        } if matches!(self.ent(fi).kind, RegEntryKind::Col { .. }) => {
                            let key = num_lit(vals.first().copied().unwrap_or(0.0));
                            let row = format!("(⊑({}⊐{}))", self.id_col(), key);
                            let pair = self.is_pair[fi];
                            ev.v = if pair {
                                format!("(<{}⊑{})", row, self.bqnv(fi))
                            } else {
                                format!("({}⊑{})", row, self.bqnv(fi))
                            };
                            ev.pair = pair;
                            ev.unit = true;
                            return Ok(ev);
                        }
                        RegEntryKind::Bind { kind, .. } => {
                            return Err(fail(
                                nd.line,
                                format!(
                                    "binding '{}' ({}) is not an entity mirror-read",
                                    binding_name,
                                    bind_kind_str(*kind)
                                ),
                            ));
                        }
                        RegEntryKind::AliasMask { .. } => {
                            return Err(fail(
                                nd.line,
                                format!(
                                    "selection '{}' is not a unique entity mirror-read",
                                    binding_name
                                ),
                            ));
                        }
                        _ => {}
                    }
                }
                // functional relationship hop: rel.Comp with ¯1 dangling
                if matches!(self.ent(bi).kind, RegEntryKind::Rel { .. }) {
                    if let Some(fi) = fe {
                        let rel = self.bqnv(bi);
                        let fe_ent = self.ent(fi);
                        let is_tag = matches!(fe_ent.kind, RegEntryKind::Tag { .. });
                        let comp = if is_tag { self.tag_mask(fi, nd.line)? } else { self.bqnv(fi) };
                        let w;
                        // every piece stays a self-contained expression: an assignment probe
                        // reuses the guard after discarding the staging buffer
                        let bname = self.ent(bi).name.as_str();
                        let key = self.rel_key(bi);
                        self.trace_dead(bname, key.as_deref(), &rel, None, m, nd.line);
                        if let Some(key) = key {
                            let ix = format!("((≠{})|{}⊐{})", key, key, rel);
                            w = format!("({}⊏{})", ix, comp);
                            ev.g = Some(crate::relationship::bqn_found(&rel, Some(&key)));
                            if !is_tag && has_pres(fe_ent) {
                                ev.g = g_and(ev.g, Some(format!("({}⊏{})", ix, self.presv(fi))));
                            }
                        } else {
                            // bound the gather like the keyed shape: a sealed-valid but
                            // out-of-range target must report DEAD, not crash the gather
                            let ix = format!("((≠{})|0⌈{})", comp, rel);
                            w = format!("({}⊏{})", ix, comp);
                            ev.g = Some(crate::relationship::bqn_found(&rel, None));
                            if !is_tag && has_pres(fe_ent) {
                                ev.g = g_and(ev.g, Some(format!("({}⊏{})", ix, self.presv(fi))));
                            }
                        }
                        ev.sym = !is_tag && col_ty(fe_ent) == Some(ColType::Sym);
                        if matches!(fe_ent.kind, RegEntryKind::Rel { .. }) {
                            ev.rel_ent = Some(fi);
                        }
                        ev.v = self.in_mode(w, m);
                        return Ok(ev);
                    }
                    // chained hop: rel.rel2.Comp
                    if matches!(field.kind, NodeKind::Hop { .. }) {
                        return Err(fail(nd.line, "nested hop chains beyond one level: spell left-assoc"));
                    }
                }
            }
        }
        // left-assoc chain: (rel.rel).Comp — the next leg resolves through the key space of
        // the rel whose VALUES the base gathered (bv.rel_ent), never the base rel's own
        if matches!(base.kind, NodeKind::Hop { .. }) {
            let bv = self.emit_hop(base, Mode::World)?;
            let fe = if let NodeKind::Name(fs) = &field.kind { self.find(self.rs(*fs)) } else { None };
            let Some(fi) = fe else {
                return Err(fail(nd.line, format!("hop target '{}' unregistered", self.node_name(field))));
            };
            let fe_ent = self.ent(fi);
            let is_tag = matches!(fe_ent.kind, RegEntryKind::Tag { .. });
            let comp = if is_tag { self.tag_mask(fi, nd.line)? } else { self.bqnv(fi) };
            let key: Option<String> = match bv.rel_ent {
                Some(ri) => self.rel_key(ri),
                None => {
                    if self.need_idx && self.world_shifted { Some("anoIdx".to_string()) } else { None }
                }
            };
            let w;
            // self-contained expressions only (the assignment-probe rule above)
            if let Some(ri) = bv.rel_ent {
                let rname = self.ent(ri).name.as_str();
                self.trace_dead(rname, key.as_deref(), &bv.v, bv.g.as_deref(), m, nd.line);
            }
            if let Some(key) = key {
                let ix = format!("((≠{})|{}⊐{})", key, key, bv.v);
                w = format!("({}⊏{})", ix, comp);
                ev.g = g_and(bv.g.clone(), Some(crate::relationship::bqn_found(&bv.v, Some(&key))));
            } else {
                let ix = format!("((≠{})|0⌈{})", comp, bv.v);
                w = format!("({}⊏{})", ix, comp);
                ev.g = g_and(bv.g.clone(), Some(crate::relationship::bqn_found(&bv.v, None)));
            }
            ev.sym = !is_tag && col_ty(fe_ent) == Some(ColType::Sym);
            if matches!(fe_ent.kind, RegEntryKind::Rel { .. }) {
                ev.rel_ent = Some(fi);
            }
            ev.v = self.in_mode(w, m);
            return Ok(ev);
        }
        Err(fail(nd.line, "unsupported hop"))
    }

    // The value dispatch (C emitVal).
    fn emit_val(&mut self, nd: &Node, m: Mode) -> R<Ev> {
        let mut ev = Ev::default();
        match &nd.kind {
            NodeKind::Num(x) => {
                ev.v = num_lit(*x);
                ev.unit = true;
                Ok(ev)
            }
            NodeKind::Counter { val, unit } => {
                let e = self.find(self.rs(*unit));
                ev.unit = true;
                ev.v = match e {
                    Some(i) => format!("({}×{})", num_lit(*val), self.bqnv(i)),
                    None => num_lit(*val),
                };
                Ok(ev)
            }
            NodeKind::Sym(s) => {
                ev.v = format!("(<\"{}\")", self.rs(*s));
                ev.unit = true;
                ev.sym = true;
                Ok(ev)
            }
            NodeKind::Str(s) => {
                ev.v = format!("\"{}\"", self.rs(*s));
                ev.unit = true;
                Ok(ev)
            }
            NodeKind::Name(s) => self.emit_name_val(*s, *s, nd.line, m, LookupMode::Bare),
            // Post-normalize Alias is a resolved dynamic-alias request; on the A4 bare fallback
            // `look` is still the requested stem, so defs and the index/x/y/char frame specials
            // are reachable exactly as they are for a bare name.
            NodeKind::Alias { look, req } => {
                self.emit_name_val(*look, *req, nd.line, m, LookupMode::DynamicAliasThenBare)
            }
            NodeKind::Arith { op, l, r } => {
                let a = self.emit_val(l, m)?;
                let b = self.emit_val(r, m)?;
                ev.g = g_and(a.g.clone(), b.g.clone());
                ev.gv = a.gv || b.gv;
                ev.pair = a.pair || b.pair;
                ev.unit = a.unit && b.unit;
                // Pointwise scalar extension preserves a scan's row domain and traversal order.
                let implicit = self.mode_domain(m);
                ev.domain = scan_domain_join(&a, &b, implicit.as_deref(), nd.line)?;
                ev.along = scan_order_join(&a, &b, nd.line)?;
                if *op == ArithOp::Mod {
                    // a % b = b | a
                    ev.v = format!("({}|{})", b.v, a.v);
                    return Ok(ev);
                }
                let opg = match op {
                    ArithOp::Add => "+",
                    ArithOp::Sub => "-",
                    ArithOp::Mul => "×",
                    ArithOp::Div => "÷",
                    ArithOp::Mod => "|",
                };
                ev.v = format!("({}{}{})", a.v, opg, b.v);
                Ok(ev)
            }
            NodeKind::Cmp { op, l, r } => {
                let a = self.emit_val(l, m)?;
                let b = self.emit_val(r, m)?;
                let implicit = self.mode_domain(m);
                ev.domain = scan_domain_join(&a, &b, implicit.as_deref(), nd.line)?;
                ev.along = scan_order_join(&a, &b, nd.line)?;
                ev.g = g_and(a.g, b.g);
                ev.gv = a.gv || b.gv;
                let sym = a.sym || b.sym;
                if sym && matches!(op, CmpOp::Eq | CmpOp::Ne) {
                    ev.v = format!("({}{}≡¨{})", if *op == CmpOp::Ne { "¬" } else { "" }, a.v, b.v);
                    return Ok(ev);
                }
                let opg = match op {
                    CmpOp::Lt => "<",
                    CmpOp::Gt => ">",
                    CmpOp::Le => "≤",
                    CmpOp::Ge => "≥",
                    CmpOp::Ne => "≠",
                    CmpOp::Eq => "=",
                };
                ev.v = format!("({}{}{})", a.v, opg, b.v);
                Ok(ev)
            }
            NodeKind::And(..) | NodeKind::Or(..) | NodeKind::Not(..) => {
                let msk = self.emit_mask(nd)?;
                ev.v = self.in_mode(msk, m);
                Ok(ev)
            }
            NodeKind::Scope { .. } => {
                // value @ scope outside a fold: locative AND at mask level
                let msk = self.emit_mask(nd)?;
                ev.v = self.in_mode(msk, m);
                Ok(ev)
            }
            NodeKind::Hop { .. } => self.emit_hop(nd, m),
            NodeKind::Call { callee, args } => self.emit_call(*callee, args, nd.line, m),
            NodeKind::Fold { op, operand } => self.emit_fold(*op, operand, nd.line, m),
            NodeKind::ScanExpr { op, operand } => self.emit_scan(*op, operand, nd.line),
            NodeKind::ScanAlong { op, col, order } => {
                let xv = self.emit_val(col, Mode::World)?;
                let ov = self.emit_val(order, Mode::World)?;
                let opn = self.rs(*op);
                let gl = match self.render_scan(opn, reducer::Form::ScanAlong) {
                    Some(gl) => gl,
                    None => {
                        let fe = self
                            .find(opn)
                            .filter(|&i| matches!(self.ent(i).kind, RegEntryKind::Fn { .. } | RegEntryKind::TypedFn { .. }));
                        let Some(fe) = fe else {
                            return Err(fail(
                                nd.line,
                                format!("internal: no scan descriptor for '{}'", opn),
                            ));
                        };
                        format!("{}`", self.fnv(fe))
                    }
                };
                ev.v = format!("({}({})⊏{})", gl, ov.v, xv.v);
                ev.along = Some(ov.v.clone());
                ev.domain = Some(ov.v);
                Ok(ev)
            }
            NodeKind::IotaX(inner) => {
                let n = self.emit_val(inner, Mode::World)?;
                ev.v = format!("(↕{})", n.v);
                Ok(ev)
            }
            NodeKind::Tuple(kids) => {
                if kids.len() != 2 {
                    return Err(fail(nd.line, format!("value tuple arity {}", kids.len())));
                }
                let a = self.emit_val(&kids[0], m)?;
                let b = self.emit_val(&kids[1], m)?;
                ev.pair = true;
                ev.g = g_and(a.g, b.g);
                ev.gv = a.gv || b.gv;
                if a.unit && b.unit {
                    ev.unit = true;
                    ev.v = format!("(<{}‿{})", a.v, b.v);
                    return Ok(ev);
                }
                ev.v = format!("({}⋈¨{})", a.v, b.v);
                Ok(ev)
            }
            NodeKind::To { shape, .. } => {
                // reshape rhs: exactly count-of-selection positions, in selection space
                let dims = shape_dims(shape);
                let dim = |d: Option<&Node>| -> i32 {
                    match d {
                        Some(d) if matches!(d.kind, NodeKind::Wild) => -1,
                        Some(d) => num_of(d) as i32,
                        None => -1,
                    }
                };
                let d0 = dim(dims.first());
                if dims.len() <= 1 {
                    ev.v = format!("(↕{})", d0);
                    return Ok(ev);
                }
                let d1 = dim(dims.get(1));
                let k = format!("(+´{})", self.sel_var);
                let rows = if d0 < 0 { format!("({}÷{})", k, d1) } else { format!("{}", d0) };
                let cols = if d1 < 0 { format!("({}÷{})", k, d0) } else { format!("{}", d1) };
                ev.pair = true;
                ev.v = format!("(⥊↕{}‿{})", rows, cols);
                Ok(ev)
            }
            NodeKind::Grade { key, desc } => {
                ev.v = self.emit_grade_ids(key, *desc)?;
                ev.unit = true;
                Ok(ev)
            }
            NodeKind::Top { k, inner } => {
                // top k truncates an ORDERING; a mask has no order to truncate
                if matches!(
                    inner.kind,
                    NodeKind::And(..)
                        | NodeKind::Or(..)
                        | NodeKind::Not(..)
                        | NodeKind::Cmp { .. }
                        | NodeKind::CmpAny { .. }
                        | NodeKind::Tuple(_)
                ) {
                    return Err(fail(
                        nd.line,
                        format!("top {} over a mask: the operand must be an ordered selection (grade or pipeline)", *k as i32),
                    ));
                }
                let inner_v = if let NodeKind::Grade { key, desc } = &inner.kind {
                    self.emit_grade_ids(key, *desc)?
                } else {
                    self.emit_val(inner, Mode::World)?.v
                };
                ev.v = format!("({}↑{})", *k as i32, inner_v);
                ev.unit = true;
                Ok(ev)
            }
            NodeKind::OrderBy { key, desc } => {
                let kv = self.emit_val(key, Mode::World)?;
                ev.v = format!("({}{})", if *desc { "⍒" } else { "⍋" }, kv.v);
                Ok(ev)
            }
            NodeKind::CrossV { f, a, b } => {
                let am = self.emit_mask(a)?;
                let bm = self.emit_mask(b)?;
                let fe = self.find(self.rs(*f)).filter(|&i| matches!(self.ent(i).kind, RegEntryKind::Fn { .. } | RegEntryKind::TypedFn { .. }));
                let Some(fe) = fe else {
                    return Err(fail(nd.line, "cross needs a registered fn"));
                };
                ev.v = format!("((/{}){}⌜(/{}))", am, self.fnv(fe), bm);
                Ok(ev)
            }
            NodeKind::Pipe { .. } => {
                let vw = self.emit_pipe(nd)?;
                ev.v = self.view_world_ids(&vw);
                ev.unit = true;
                Ok(ev)
            }
            NodeKind::SetHop { .. } => Err(fail(nd.line, "bare rel' outside fold/source position")),
            k => Err(fail(nd.line, format!("unsupported value node {}", k.c_kind()))),
        }
    }
}

/* ---------- masks ---------- */

impl<'a> Em<'a> {
    // Cell coordinate column for the anchored frame: registered x/y fields win, else the
    // lattice frame's computed coordinates; None when neither exists.
    fn cell_coord(&self, axis: char) -> Option<String> {
        let e = self.find(if axis == 'x' { "x" } else { "y" });
        if let Some(i) = e {
            if matches!(self.ent(i).kind, RegEntryKind::Field { .. } | RegEntryKind::Col { .. }) {
                return Some(self.bqnv(i));
            }
        }
        if matches!(self.fr.kind, FrameKind::Lat | FrameKind::Board) {
            let wh = self.fr.w.wrapping_mul(self.fr.h);
            return Some(if axis == 'y' {
                format!("(⌊(↕{})÷{})", wh, self.fr.w)
            } else {
                format!("({}|↕{})", self.fr.w, wh)
            });
        }
        None
    }

    // Mask emission: full frame-length boolean vector, all guards folded in.
    fn emit_mask(&mut self, nd: &Node) -> R<String> {
        match &nd.kind {
            NodeKind::Name(s) => self.emit_name_mask(*s, *s, nd.line, LookupMode::Bare),
            NodeKind::Alias { look, req } => {
                self.emit_name_mask(*look, *req, nd.line, LookupMode::DynamicAliasThenBare)
            }
            NodeKind::And(a, b) | NodeKind::Or(a, b) => {
                let am = self.emit_mask(a)?;
                let bm = self.emit_mask(b)?;
                Ok(format!("({}{}{})", am, if matches!(nd.kind, NodeKind::And(..)) { "∧" } else { "∨" }, bm))
            }
            NodeKind::Not(k) => {
                // absent-component reading for sparse value columns; a fallback ^name takes the
                // same (¬presv) path as the bare spelling, so the two emit byte-identical BQN
                if let NodeKind::Name(s) | NodeKind::Alias { look: s, .. } = &k.kind {
                    if let Some(ei) = self.find(self.rs(*s)) {
                        let e = self.ent(ei);
                        if matches!(e.kind, RegEntryKind::Col { .. })
                            && col_ty(e) != Some(ColType::Bool)
                            && has_pres(e)
                        {
                            return Ok(format!("(¬{})", self.presv(ei)));
                        }
                    }
                }
                let a = self.emit_mask(k)?;
                Ok(format!("(¬{})", a))
            }
            NodeKind::CmpAny { name } => {
                // presence-any tuple element
                let n = self.rs(*name);
                let Some(ei) = self.find(n) else {
                    return Err(fail(nd.line, format!("unregistered '{} _'", n)));
                };
                Ok(if has_pres(self.ent(ei)) {
                    self.presv(ei)
                } else {
                    format!("(1¨{})", self.bqnv(ei))
                })
            }
            NodeKind::Cmp { .. } => {
                let v = self.emit_val(nd, Mode::World)?;
                Ok(match v.g {
                    Some(g) => format!("(({})∧{})", v.v, g),
                    None => v.v,
                })
            }
            NodeKind::Tuple(kids) => {
                // presence tuple: AND of elements
                let mut acc: Option<String> = None;
                for k in kids {
                    let mi = self.emit_mask(k)?;
                    acc = Some(match acc {
                        Some(a) => format!("({}∧{})", a, mi),
                        None => mi,
                    });
                }
                Ok(acc.unwrap_or_default())
            }
            NodeKind::Scope { l, r, origin: Some(org) } => {
                // mask @ frame at origin — the anchored frame (frame join): @ fixes (o, S) and
                // `at` fills the origin slot with a mirror-read; the registered frame fn runs
                // per cell as Fn ⟨cell, origin, args…⟩ and returns the frame mask
                let a = self.emit_mask(l)?;
                let fnode: &Node = r;
                let orgv = self.emit_val(org, Mode::World)?;
                if !orgv.unit || !orgv.pair {
                    return Err(fail(nd.line, "frame anchor must be one point (a pair mirror-read)"));
                }
                let NodeKind::Call { callee, args } = &fnode.kind else {
                    return Err(fail(fnode.line, "anchored frame wants fn(args…) — a registered frame predicate"));
                };
                let cn = self.rs(*callee);
                let fe = self.find(cn).filter(|&i| matches!(self.ent(i).kind, RegEntryKind::Fn { .. }));
                let Some(fe) = fe else {
                    return Err(fail(fnode.line, format!("anchored frame '{}' is not a registered fn", cn)));
                };
                let xs = self.cell_coord('x');
                let ys = self.cell_coord('y');
                let (Some(xs), Some(ys)) = (xs, ys) else {
                    return Err(fail(fnode.line, "anchored frame needs cell coordinates (x/y fields or a lattice frame)"));
                };
                let mut argstr = String::new();
                for k in args {
                    let av = self.emit_val(k, Mode::World)?;
                    argstr = format!("{}, {}", argstr, av.v);
                }
                let t = self.tv();
                let fv = self.fnv(fe);
                self.stage(format!("{} ← {{{} ⟨𝕩, ⊑{}{}⟩}}¨({}⋈¨{})", t, fv, orgv.v, argstr, xs, ys));
                Ok(format!("({}∧{})", a, t))
            }
            NodeKind::Scope { l, r, origin: None } => {
                if matches!(r.kind, NodeKind::Shape(_)) {
                    // frame scope: predicate over the lattice
                    return self.emit_mask(l);
                }
                let a = self.emit_mask(l)?;
                let b = self.emit_mask(r)?;
                Ok(format!("({}∧{})", a, b))
            }
            NodeKind::Hop { l, r } if matches!(r.kind, NodeKind::SetHop { .. }) => {
                // image: Sel.rel' — union of the selected sources' fibers, membership by stable
                // id; a keyed srel's fibers hold keys, so membership runs against its own key column
                let src = self.emit_mask(l)?;
                let rel = fiber_sym(r);
                let (fib, _raw) = self.fiber_var(rel, r.line)?;
                let se = self.find(self.rs(rel));
                let mut ids: Option<String> = None;
                if let Some(si) = se {
                    if let RegEntryKind::SRel { key_of: Some(k), .. } = &self.ent(si).kind {
                        if let Some(kc) = self.find(k) {
                            ids = Some(self.bqnv(kc));
                        }
                    }
                }
                let mcol = match ids {
                    Some(i) => i,
                    None => self.id_col(),
                };
                // --trace: a dead member is a dead link the image crosses; only stored
                // entity-sided srel fibers can hold dead members
                if self.dirs.trace {
                    if let Some(si) = se {
                        if let RegEntryKind::SRel { fib: fibers, .. } = &self.ent(si).kind {
                            if fibers.len() == self.reg.n as usize {
                                let sname = self.ent(si).name.as_str();
                                let scoped = self.trace_scoped(Mode::World);
                                let usage = self.record_use(scoped, nd.line, sname);
                                let tids = self.trace_ids(&fib);
                                let sel = if scoped {
                                    format!("({}∧{})", self.trace_sel, src)
                                } else {
                                    src.clone()
                                };
                                self.stage(format!(
                                    "({}/{}) {{m←(≠{})≤{}⊐𝕩 ⋄ AnoTraceDead ⟨\"{}\", (+´m)⥊𝕨, m/𝕩, \"{}\"⟩}}¨ ({}/{})",
                                    sel, tids, mcol, mcol, sname, usage, sel, fib
                                ));
                            }
                        }
                    }
                }
                Ok(format!("({}‿{} AnoImage {})", src, fib, mcol))
            }
            NodeKind::Hop { .. } => {
                let v = self.emit_hop(nd, Mode::World)?;
                Ok(match v.g {
                    Some(g) => format!("(({})∧{})", v.v, g),
                    None => v.v,
                })
            }
            NodeKind::Fold { op, operand } => {
                // quantifier fold as a mask: guards fold in
                let v = self.emit_fold(*op, operand, nd.line, Mode::World)?;
                Ok(match v.g {
                    Some(g) => format!("(({})∧{})", v.v, g),
                    None => v.v,
                })
            }
            NodeKind::Top { .. } | NodeKind::Grade { .. } => {
                let v = self.emit_val(nd, Mode::World)?;
                Ok(format!("((↕{})∊{})", self.fr_n(), v.v))
            }
            NodeKind::Pipe { .. } => {
                let vw = self.emit_pipe(nd)?;
                if let Some(c) = &vw.expand_cnt {
                    // Spawner |> expand Count , spawn X : counts feed the spawn
                    self.pipe_expand = c.clone();
                }
                Ok(format!("((↕{})∊{})", self.fr_n(), self.view_world_ids(&vw)))
            }
            NodeKind::Shape(_) => {
                // pure shape source: everything in frame
                Ok(format!("(1¨↕{})", self.fr_n()))
            }
            _ => {
                let v = self.emit_val(nd, Mode::World)?;
                Ok(match v.g {
                    Some(g) => format!("(({})∧{})", v.v, g),
                    None => v.v,
                })
            }
        }
    }
}

/* ---------- frames, commits, effects ---------- */

impl<'a> Em<'a> {
    // Set the frame from the selection; stages the board literal binding when needed.
    // None (continuation) restores the saved frame.
    fn set_frame(&mut self, sel: Option<&Node>) {
        self.fr = Frame::ent();
        let Some(sel) = sel else {
            self.fr = self.saved_fr.clone();
            return;
        };
        let lm = leftmost(sel);
        if let NodeKind::Shape(dims) = &lm.kind {
            if dims.len() == 2 {
                self.fr.kind = FrameKind::Lat;
                self.fr.w = num_of(&dims[0]) as i32;
                self.fr.h = num_of(&dims[1]) as i32;
            } else {
                self.fr.kind = FrameKind::Line;
                self.fr.w = dims.first().map(num_of).unwrap_or(0.0) as i32;
            }
            return;
        }
        if let NodeKind::To { shape, poured: Some(p) } = &lm.kind {
            if let NodeKind::Str(lit) = &p.kind {
                self.fr.kind = FrameKind::Board;
                let dims = shape_dims(shape);
                self.fr.w = dims.first().map(num_of).unwrap_or(0.0) as i32;
                self.fr.h = if dims.len() > 1 { num_of(&dims[1]) as i32 } else { 1 };
                self.fr.lit = self.rs(*lit).to_string();
                let line = format!("brd{} ← \"{}\"", self.stmt, self.fr.lit);
                self.stage(line);
                return;
            }
        }
        // a shape-scoped subterm pulls the frame: +/ Elevation @ 64 64
        if let Some(sh) = find_shape_scope(sel) {
            let dims = shape_dims(sh);
            self.fr.kind = FrameKind::Lat;
            self.fr.w = dims.first().map(num_of).unwrap_or(0.0) as i32;
            self.fr.h = if dims.len() > 1 { num_of(&dims[1]) as i32 } else { 1 };
        }
    }

    // §10 same-column batch: legal iff same commuting family, both '=' on distinct pair
    // fields, or (§11 third clause) certified row-disjoint rules; the base is the earlier
    // commit's expr — the later effect rebases onto it.
    fn merge_base(&self, fx: &Fx, col_idx: usize, col: String, fam: u8, field: Option<&str>, line: i32, name: &str) -> R<String> {
        let Some(prev) = fx.commits.iter().find(|c| c.col_idx == col_idx) else {
            return Ok(col);
        };
        let mut ok = (prev.fam == fam && matches!(fam, b'+' | b'*' | b'|' | b'&'))
            || (prev.fam == b'=' && fam == b'=' && field.is_some() && prev.field.is_some()
                && field != prev.field.as_deref());
        if !ok
            && self.cur_rule >= 0
            && prev.rules != 0
            && (prev.rules & !self.rule_disj[self.cur_rule as usize]) == 0
            && fam != b'v'
            && prev.fam != b'v'
        {
            ok = true;
        }
        if !ok {
            return Err(fail(line, format!("no merge law: '{}' written twice in one barrier (§10)", name)));
        }
        Ok(prev.new_expr.clone())
    }

    // Replaces an existing commit for the column (the rebase result) and ORs the rule bit.
    fn add_commit(&self, fx: &mut Fx, col_idx: usize, expr: String, fam: u8, field: Option<String>) {
        let bit = if self.cur_rule >= 0 { 1u32 << self.cur_rule } else { 0 };
        for c in fx.commits.iter_mut() {
            if c.col_idx == col_idx {
                c.new_expr = expr;
                c.fam = fam;
                c.field = field;
                c.rules |= bit;
                return;
            }
        }
        fx.commits.push(Commit { col_idx, new_expr: expr, fam, field, rules: bit });
    }

    // The effect dispatch (C emitEffect). Effects read pre-state; commits land at the end.
    // The phase flag rides the whole dispatch: every crossing staged here is an EFFECT
    // crossing over S, and only the discarded guard probe steps back out of it.
    fn emit_effect(&mut self, ef: &Node, fx: &mut Fx) -> R<()> {
        let saved = self.in_effect;
        self.in_effect = true;
        let out = self.emit_effect_inner(ef, fx);
        self.in_effect = saved;
        out
    }

    fn emit_effect_inner(&mut self, ef: &Node, fx: &mut Fx) -> R<()> {
        match &ef.kind {
            NodeKind::EAssign { op, target, rhs } => {
                let (coln, field): (&Node, Option<&'a str>) = if let NodeKind::Hop { l, r } = &target.kind {
                    (l, Some(self.node_name(r)))
                } else {
                    (target.as_ref(), None)
                };
                let col_name = self.node_name(coln);
                let Some(ei) = self.find(col_name) else {
                    return Err(fail(ef.line, format!("assign to unregistered '{}'", col_name)));
                };
                let e = self.ent(ei);
                // Derived tags are computed from their carrier and cannot be assigned.
                if let RegEntryKind::Tag { col: carrier, .. } = &e.kind {
                    return Err(fail(
                        ef.line,
                        format!("derived tag '{}' is not an effect target: write the carrier column '{}'", col_name, carrier),
                    ));
                }
                // keys are minted at spawn, never written by effects
                if is_uniq(e) {
                    return Err(fail(ef.line, format!("unique column '{}' is minted, never written", col_name)));
                }
                let col = self.bqnv(ei);
                // Guards refine the mask before gathering. The probe's staging is discarded, so
                // its residual must be a closed predicate over durable world variables.
                let saved_pre = std::mem::take(&mut self.pre);
                let saved_uses = self.plan.len();
                let saved_effect = self.in_effect;
                let probe_tmp = self.tmp;
                self.in_effect = false;
                let probe_res = self.emit_val(rhs, Mode::World);
                self.pre = saved_pre;
                self.plan.truncate(saved_uses);
                self.in_effect = saved_effect;
                let probe = probe_res?;
                if let Some(guard) = &probe.g {
                    for temp in probe_tmp..self.tmp {
                        let name = format!("t{}", temp);
                        if mentions_bqn_name(guard, &name) {
                            return Err(fail(
                                ef.line,
                                format!(
                                    "guard residual depends on discarded staging '{}'",
                                    name
                                ),
                            ));
                        }
                    }
                }
                let old_sel = self.sel_var.clone();
                let sel_e = if let Some(g) = &probe.g {
                    let t = self.tv();
                    self.stage(format!("{} ← {}∧{}", t, old_sel, g));
                    t
                } else {
                    old_sel.clone()
                };
                self.sel_var = sel_e.clone();
                let rhs_v = self.emit_val(rhs, Mode::Sel)?;
                let mut rv = rhs_v.v.clone();
                if rhs_v.unit {
                    rv = format!("((+´{})⥊{})", sel_e, rv);
                }
                // Ordered scans grade values and their row witness together. The exact witness
                // must equal the statement's selected world rows: equal width alone is unsound.
                let mut scan_domain = rhs_v.domain.clone();
                if let Some(al) = &rhs_v.along {
                    let grade = format!("(⍋{})", al);
                    rv = format!("({}⊏{})", grade, rv);
                    if let Some(domain) = scan_domain {
                        scan_domain = Some(format!("({}⊏{})", grade, domain));
                    }
                }
                if let Some(domain) = scan_domain {
                    let canonical_name = self.ent(ei).name.clone();
                    self.stage(format!(
                        "\"scan domain {}\" ! ({}) ≡ ({}/↕{})",
                        canonical_name,
                        domain,
                        sel_e,
                        self.fr_n()
                    ));
                }
                let fam = match op {
                    AssignOp::Set => b'=',
                    AssignOp::Add | AssignOp::Sub => b'+',
                    _ => b'*',
                };
                let base = self.merge_base(fx, ei, col, fam, field, ef.line, col_name)?;
                if *op != AssignOp::Set {
                    let opch = match op {
                        AssignOp::Add => "+",
                        AssignOp::Sub => "-",
                        AssignOp::Mul => "×",
                        _ => "÷",
                    };
                    let oldv = match field {
                        Some(f) => format!("({}⊸⊑¨({}/{}))", (f.as_bytes().first() == Some(&b'y')) as i32, sel_e, base),
                        None => format!("({}/{})", sel_e, base),
                    };
                    rv = format!("({}{}{})", oldv, opch, rv);
                }
                let newcol = if let Some(f) = field {
                    let yi = f.as_bytes().first() == Some(&b'y');
                    let pairs = if yi {
                        format!("((0⊸⊑¨({}/{}))⋈¨{})", sel_e, base, rv)
                    } else {
                        format!("({}⋈¨(1⊸⊑¨({}/{})))", rv, sel_e, base)
                    };
                    self.is_pair[ei] = true;
                    format!("{}‿{} AnoScat {}", sel_e, pairs, base)
                } else {
                    if rhs_v.pair {
                        self.is_pair[ei] = true;
                    }
                    format!("{}‿{} AnoScat {}", sel_e, rv, base)
                };
                let t = self.tv();
                self.stage(format!("{} ← {}", t, newcol));
                self.add_commit(fx, ei, t, fam, field.map(String::from));
                self.sel_var = old_sel;
                Ok(())
            }
            NodeKind::EAdd(sym) | NodeKind::EDel(sym) => {
                let is_add = matches!(ef.kind, NodeKind::EAdd(_));
                let n = self.rs(*sym);
                let e = self.find(n);
                if let Some(i) = e {
                    if let RegEntryKind::Tag { col: carrier, .. } = &self.ent(i).kind {
                        return Err(fail(
                            ef.line,
                            format!("derived tag '{}' is not an effect target: write the carrier column '{}'", n, carrier),
                        ));
                    }
                }
                let ok = e.map_or(false, |i| {
                    matches!(self.ent(i).kind, RegEntryKind::Col { .. } | RegEntryKind::Field { .. })
                });
                if !ok {
                    return Err(fail(ef.line, format!("{}Comp on unregistered '{}'", if is_add { '+' } else { '-' }, n)));
                }
                let ei = e.unwrap_or_default();
                if is_uniq(self.ent(ei)) {
                    return Err(fail(ef.line, format!("unique column '{}' is minted, never written", n)));
                }
                let col = self.bqnv(ei);
                let fam = if is_add { b'|' } else { b'&' };
                let base = self.merge_base(fx, ei, col, fam, None, ef.line, n)?;
                let t = self.tv();
                if is_add {
                    self.stage(format!("{} ← {}∨{}", t, base, self.sel_var));
                } else {
                    self.stage(format!("{} ← {}∧¬{}", t, base, self.sel_var));
                }
                self.add_commit(fx, ei, t, fam, None);
                Ok(())
            }
            NodeKind::EDespawn => {
                fx.despawn_sel = Some(if fx.despawn {
                    format!("({}∨{})", fx.despawn_sel.as_deref().unwrap_or(""), self.sel_var)
                } else {
                    self.sel_var.clone()
                });
                fx.despawn = true;
                Ok(())
            }
            NodeKind::ESpawn { what, count, at } => {
                // spawn writes its proto column; a derived tag takes no writes anywhere
                if let NodeKind::Name(ws) = &what.kind {
                    if let Some(wi) = self.find(self.rs(*ws)) {
                        if let RegEntryKind::Tag { col: carrier, .. } = &self.ent(wi).kind {
                            return Err(fail(
                                ef.line,
                                format!("derived tag '{}' is not an effect target: write the carrier column '{}'", self.rs(*ws), carrier),
                            ));
                        }
                    }
                }
                let lat = matches!(self.fr.kind, FrameKind::Lat | FrameKind::Board);
                // On a lattice, spawning a registered field ORs the selection into it; no rows mint.
                if lat && count.is_none() {
                    if let NodeKind::Name(ws) = &what.kind {
                        if let Some(fi2) = self.find(self.rs(*ws)) {
                            if matches!(self.ent(fi2).kind, RegEntryKind::Field { .. }) {
                                let col = self.bqnv(fi2);
                                let base = self.merge_base(fx, fi2, col, b'|', None, ef.line, self.rs(*ws))?;
                                let t = self.tv();
                                self.stage(format!("{} ← {}∨{}", t, base, self.sel_var));
                                self.add_commit(fx, fi2, t, b'|', None);
                                return Ok(());
                            }
                        }
                    }
                }
                if fx.sp.len() >= 8 {
                    return Err(fail(ef.line, "too many spawn groups in one statement"));
                }
                let mut proto_sel: Option<String> = None; // computed proto, selection space
                let cv: String = if let Some(cnt) = count {
                    let c = self.emit_val(cnt, Mode::Sel)?;
                    if c.unit { format!("((+´{})⥊{})", self.sel_var, c.v) } else { c.v }
                } else if !self.pipe_expand.is_empty() {
                    self.pipe_expand.clone()
                } else if matches!(what.kind, NodeKind::Call { .. }) {
                    // spawn (pieceOf char): an empty sym spawns nothing for that cell (ex34)
                    let pv = self.emit_val(what, Mode::Sel)?;
                    let ps = self.tv();
                    self.stage(format!("{} ← {}", ps, pv.v));
                    let c = format!("(\"\"⊸≢¨{})", ps);
                    proto_sel = Some(ps);
                    c
                } else {
                    format!("((+´{})⥊1)", self.sel_var)
                };
                let c_v = self.tv();
                self.stage(format!("{} ← {}", c_v, cv));
                self.cnt_var = c_v.clone();
                // §17: an explicit replicate binds index per copy and shadows the outer
                // binding; a plain spawn keeps the minting row's ordinal (§21)
                if count.is_some() || !self.pipe_expand.is_empty() {
                    let i_v = self.tv();
                    self.stage(format!("{} ← ∾↕¨{}", i_v, c_v));
                    self.idx_var = i_v;
                } else {
                    self.idx_var = format!("(↕+´{})", self.sel_var);
                }
                let tot = format!("(+´{})", c_v);
                // stage pos/proto as temps NOW: a spawn expression must observe pre-state,
                // never a sibling commit (ex28)
                let mut pos: Option<String> = None;
                let mut pos_pair = false;
                if let Some(at_n) = at {
                    let p = self.emit_val(at_n, Mode::Copy)?;
                    let pv = self.tv();
                    let val = if p.unit { format!("({}⥊{})", tot, p.v) } else { p.v.clone() };
                    self.stage(format!("{} ← {}", pv, val));
                    pos = Some(pv);
                    pos_pair = p.pair;
                } else if lat {
                    let pv = self.tv();
                    let wh = self.fr.w.wrapping_mul(self.fr.h);
                    self.stage(format!(
                        "{} ← {}/{}/(({}|↕{})⋈¨⌊(↕{})÷{})",
                        pv, c_v, self.sel_var, self.fr.w, wh, wh, self.fr.w
                    ));
                    pos = Some(pv);
                    pos_pair = true;
                }
                let mut proto_name: Option<Symbol> = None;
                let mut proto_expr: Option<String> = None;
                if let NodeKind::Name(ws) = &what.kind {
                    proto_name = Some(*ws);
                } else if let Some(ps) = &proto_sel {
                    let pt = self.tv();
                    self.stage(format!("{} ← {}/{}", pt, c_v, ps));
                    proto_expr = Some(pt);
                } else if matches!(what.kind, NodeKind::Call { .. }) {
                    let pv = self.emit_val(what, Mode::Copy)?;
                    let pt = self.tv();
                    self.stage(format!("{} ← {}", pt, pv.v));
                    proto_expr = Some(pt);
                }
                fx.sp.push(SpawnG { cnt: c_v, tot, pos, pos_pair, proto_name, proto_expr });
                Ok(())
            }
            NodeKind::EVerb { name, args } => self.emit_verb(ef.line, *name, args, fx),
            NodeKind::EVia { f, col } => self.emit_verb(ef.line, *f, std::slice::from_ref(col), fx),
            k => Err(fail(ef.line, format!("unsupported effect {}", k.c_kind()))),
        }
    }

    // Legacy verbs take their target from the raw prefix. Typed effects take zero or one target
    // from the checked write footprint; zero-target output-service calls still execute at the barrier.
    fn emit_verb(&mut self, line: i32, name: Symbol, args: &[Node], fx: &mut Fx) -> R<()> {
        let n = self.rs(name).to_string();
        let Some(ei) = self.find(&n) else {
            return Err(fail(line, format!("verb '{}' needs a registered fn", n)));
        };
        let target = match &self.ent(ei).kind {
            RegEntryKind::Fn { body: Some(body) } => Some(verb_target(body)),
            RegEntryKind::TypedFn { descriptor, .. } if descriptor.writes.len() <= 1 => {
                descriptor.writes.first().cloned()
            }
            RegEntryKind::TypedFn { descriptor, .. } => {
                return Err(fail(
                    line,
                    format!(
                        "typed effect '{}' has {} write targets; this backend admits at most one",
                        n,
                        descriptor.writes.len()
                    ),
                ));
            }
            _ => return Err(fail(line, format!("verb '{}' needs a registered fn", n))),
        };
        let target_index = match target {
            Some(target) => {
                let Some(index) = self.find(&target) else {
                    return Err(fail(
                        line,
                        format!("verb '{}' target column '{}' unregistered", n, target),
                    ));
                };
                if is_uniq(self.ent(index)) {
                    return Err(fail(
                        line,
                        format!("unique column '{}' is minted, never written", target),
                    ));
                }
                if !matches!(
                    self.ent(index).kind,
                    RegEntryKind::Col { .. } | RegEntryKind::Field { .. }
                ) {
                    return Err(fail(
                        line,
                        format!("verb '{}' target '{}' is not a mutable column", n, target),
                    ));
                }
                Some(index)
            }
            None => None,
        };
        let mut rendered = Vec::new();
        if let Some(index) = target_index {
            rendered.push(self.bqnv(index));
        }
        for arg in args {
            rendered.push(self.emit_val(arg, Mode::World)?.v);
        }
        if let Some(index) = target_index {
            self.merge_base(fx, index, self.bqnv(index), b'v', None, line, &n)?;
        }
        let temporary = self.tv();
        let function = self.fnv(ei);
        self.stage(format!(
            "{} ← {} {} ⟨{}⟩",
            temporary,
            self.sel_var,
            function,
            rendered.join(", ")
        ));
        if let Some(index) = target_index {
            self.add_commit(fx, index, temporary, b'v', None);
        }
        Ok(())
    }
}

/* ---------- spawn fill + the one commit ---------- */

impl<'a> Em<'a> {
    // Default append value for a column on one spawn group; keys are minted by the caller.
    fn spawn_default(&self, i: usize, sg: &SpawnG) -> String {
        let e = self.ent(i);
        let tot = &sg.tot;
        match &e.kind {
            RegEntryKind::SRel { .. } => return format!("({}⥊<⟨⟩)", tot),
            RegEntryKind::Rel { .. } => return format!("({}⥊¯1)", tot),
            _ => {}
        }
        if col_ty(e) == Some(ColType::Sym) {
            return format!("({}⥊<\"\")", tot);
        }
        if self.is_pair[i] {
            return format!("({}⥊<¯1‿¯1)", tot);
        }
        if self.role("parent") == Some(i) {
            return format!("({}//{})", sg.cnt, self.sel_var);
        }
        format!("({}⥊{})", tot, num_lit(e.defval))
    }

    // The proto a spawn group names, when it names one (RK_PROTO).
    fn spawn_proto(&self, sg: &SpawnG) -> Option<usize> {
        let pn = sg.proto_name?;
        let pe = self.find(self.rs(pn))?;
        if matches!(self.ent(pe).kind, RegEntryKind::Proto { .. }) { Some(pe) } else { None }
    }

    // Index of a proto's field for a column name, None when the proto is silent on it.
    fn proto_field(&self, pi: usize, col: &str) -> Option<usize> {
        if let RegEntryKind::Proto { fields } = &self.ent(pi).kind {
            return fields.iter().position(|f| names_eq(&f.col, col));
        }
        None
    }

    // The barrier retraction: a committed write into a refined column normalizes onto the
    // carrier set — bool by 0< (positive is true: 2→1, ¯1→0), nat/int by floor then clamp
    // to ±2^53, a declared range by clamp, outermost. It is a property of the BARRIER, not
    // the write: merged writes retract once, at commit. Mask ops (fam | &) are boolean by
    // construction, so a bool column elides the wrap — the type proves the write in-domain.
    // Appends (spawn fill) and pre-state reads are in-domain by the load seal.
    fn retract(&self, i: usize, fam: u8, expr: String) -> String {
        let e = self.ent(i);
        let (ty, rng) = match &e.kind {
            RegEntryKind::Col { ty, rng, .. } | RegEntryKind::Field { ty, rng, .. } => (*ty, *rng),
            _ => return expr,
        };
        let mut v = expr;
        match ty {
            ColType::Bool => {
                if fam != b'|' && fam != b'&' {
                    v = format!("(0<{})", v);
                }
            }
            ColType::Nat => v = format!("({}⌊0⌈⌊{})", num_lit(crate::ANO_NATMAX), v),
            ColType::Int => {
                v = format!("({}⌊{}⌈⌊{})", num_lit(crate::ANO_NATMAX), num_lit(-crate::ANO_NATMAX), v)
            }
            _ => {}
        }
        if let Some((lo, hi)) = rng {
            v = format!("({}⌊{}⌈{})", num_lit(hi), num_lit(lo), v);
        }
        v
    }

    // The batch total: sum of every spawn group's row count.
    fn spawn_tot_all(&self, fx: &Fx) -> Option<String> {
        let mut tot: Option<String> = None;
        for g in &fx.sp {
            tot = Some(match tot {
                Some(t) => format!("({}+{})", t, g.tot),
                None => g.tot.clone(),
            });
        }
        tot
    }

    // The one commit: despawn keep, per-entry base/append in registry declaration order,
    // presence riders, the hidden idx column, anoN, the --trace tick line.
    fn commit_stmt(&mut self, fx: &Fx, _is_cont: bool) -> R<()> {
        let mut keep: Option<String> = None;
        if fx.despawn {
            let k = self.tv();
            let ds = fx.despawn_sel.clone().unwrap_or_else(|| self.sel_var.clone());
            self.stage(format!("{} ← ¬{}", k, ds));
            keep = Some(k);
        }
        let structural = fx.despawn || !fx.sp.is_empty();
        if self.fr.kind != FrameKind::Ent && structural && fx.despawn {
            return Err(fail(0, "despawn outside the entity world"));
        }
        let tot_all = if !fx.sp.is_empty() { self.spawn_tot_all(fx) } else { None };
        // --trace: the pre-state row count, captured before anoN reassigns
        let mut pre_n: Option<String> = None;
        if self.dirs.trace && structural {
            let p = self.tv();
            self.stage(format!("{} ← anoN", p));
            pre_n = Some(p);
        }
        let n = self.reg.n as usize;
        for i in 0..self.reg.ents.len() {
            let e = self.ent(i);
            let is_field = matches!(e.kind, RegEntryKind::Field { .. });
            let is_stored_mask = matches!(
                e.kind,
                RegEntryKind::AliasMask { .. }
                    | RegEntryKind::Bind {
                        kind: BindKind::Mask,
                        ..
                    }
            );
            if !matches!(e.kind, RegEntryKind::Col { .. } | RegEntryKind::Rel { .. } | RegEntryKind::SRel { .. })
                && !is_field
                && !is_stored_mask
            {
                continue;
            }
            // lattice-sided relations are frame-foreign: never filter or pad
            if let RegEntryKind::SRel { fib, .. } = &e.kind {
                if fib.len() != n {
                    continue;
                }
            }
            if let RegEntryKind::Rel { targets, .. } = &e.kind {
                if targets.len() != n {
                    continue;
                }
            }
            let cur = self.bqnv(i);
            let commit = fx.commits.iter().find(|c| c.col_idx == i);
            let committed = commit.is_some();
            let base = match commit {
                Some(c) => self.retract(i, c.fam, c.new_expr.clone()),
                None => cur.clone(),
            };
            if is_field {
                // lattice fields: value commits only, never row structure
                if committed {
                    self.publish(i, base)?;
                }
                continue;
            }
            let mut app: Option<String> = None;
            if !fx.sp.is_empty() {
                // spawn appends one row group per spawn effect, in effect order
                if self.role("keys") == Some(i) || is_uniq(e) {
                    // Batch mint uses 1 + max(existing, -1) plus iota. No ceiling or collision check is emitted.
                    app = Some(format!("((1+⌈´¯1∾{})+↕{})", cur, tot_all.as_deref().unwrap_or("")));
                } else {
                    for sg in &fx.sp {
                        let is_proto = sg.proto_name.map_or(false, |pn| self.find(self.rs(pn)) == Some(i));
                        // the three-layer fill (ruled 2026-07-11): proto value, else
                        // registered default, else the type zero
                        let pe = self.spawn_proto(sg);
                        let fi = pe.and_then(|pi| self.proto_field(pi, &e.name));
                        let piece = if is_proto {
                            format!("({}⥊1)", sg.tot)
                        } else if let Some(f) = fi {
                            let pi = pe.unwrap_or_default();
                            let (spelling, numv) = if let RegEntryKind::Proto { fields } = &self.ent(pi).kind {
                                (fields[f].spelling.as_str(), fields[f].num)
                            } else {
                                ("", 0.0)
                            };
                            if matches!(e.kind, RegEntryKind::Col { ty: ColType::Sym, .. }) {
                                format!("({}⥊<\"{}\")", sg.tot, spelling)
                            } else {
                                format!("({}⥊{})", sg.tot, num_lit(numv))
                            }
                        } else if pe.is_some()
                            && self.role("proto") == Some(i)
                            && matches!(e.kind, RegEntryKind::Col { ty: ColType::Sym, .. })
                        {
                            // the archetype's noun
                            format!("({}⥊<\"{}\")", sg.tot, self.ent(pe.unwrap_or_default()).name)
                        } else if sg.proto_expr.is_some() && self.role("proto") == Some(i) {
                            sg.proto_expr.clone().unwrap_or_default()
                        } else if sg.pos.is_some() && self.role("pos") == Some(i) {
                            if sg.pos_pair {
                                self.is_pair[i] = true;
                            }
                            sg.pos.clone().unwrap_or_default()
                        } else if is_stored_mask {
                            format!("({}⥊0)", sg.tot)
                        } else {
                            self.spawn_default(i, sg)
                        };
                        app = Some(match app {
                            Some(a) => format!("{}∾{}", a, piece),
                            None => piece,
                        });
                    }
                }
            }
            if !committed && app.is_none() && !fx.despawn {
                continue;
            }
            if structural {
                let kept = match &keep {
                    Some(k) => format!("({}/{})", k, base),
                    None => base.clone(),
                };
                let value = match &app {
                    Some(a) => format!("{}∾{}", kept, a),
                    None => kept,
                };
                self.publish(i, value)?;
            } else if committed {
                self.publish(i, base)?;
            }
            if has_pres(e) && structural {
                let p = self.presv(i);
                let kept = match &keep {
                    Some(k) => format!("({}/{})", k, p),
                    None => p.clone(),
                };
                if !fx.sp.is_empty() {
                    let mut papp: Option<String> = None;
                    for sg in &fx.sp {
                        let mut is_proto = sg.proto_name.map_or(false, |pn| self.find(self.rs(pn)) == Some(i));
                        let pe = self.spawn_proto(sg);
                        if let Some(pi) = pe {
                            if self.proto_field(pi, &e.name).is_some() {
                                is_proto = true; // a proto field is present
                            }
                        }
                        let piece = format!("({}⥊{})", sg.tot, if is_proto { 1 } else { 0 });
                        papp = Some(match papp {
                            Some(a) => format!("{}∾{}", a, piece),
                            None => piece,
                        });
                    }
                    self.stage(format!("{} ↩ {}∾{}", p, kept, papp.unwrap_or_default()));
                } else {
                    self.stage(format!("{} ↩ {}", p, kept));
                }
            }
        }
        // the hidden idx column rides every structural commit like any other column
        if self.need_idx && structural {
            let mint = tot_all.as_ref().map(|t| format!("((1+⌈´¯1∾anoIdx)+↕{})", t));
            match (&keep, &mint) {
                (Some(k), Some(mm)) => self.stage(format!("anoIdx ↩ ({}/anoIdx)∾{}", k, mm)),
                (Some(k), None) => self.stage(format!("anoIdx ↩ {}/anoIdx", k)),
                (None, Some(mm)) => self.stage(format!("anoIdx ↩ anoIdx∾{}", mm)),
                (None, None) => {}
            }
        }
        // spawn always appends to the entity world, whatever frame selected the sources
        if structural {
            if fx.despawn && !fx.sp.is_empty() {
                self.stage(format!(
                    "anoN ↩ (+´{})+{}",
                    keep.as_deref().unwrap_or(""),
                    tot_all.as_deref().unwrap_or("")
                ));
            } else if fx.despawn {
                self.stage(format!("anoN ↩ +´{}", keep.as_deref().unwrap_or("")));
            } else {
                self.stage(format!("anoN ↩ anoN+{}", tot_all.as_deref().unwrap_or("")));
            }
        }
        // --trace: one tick-trace line per structural statement
        if let Some(pn) = pre_n {
            let mut ann: Option<String> = None;
            for sg in &fx.sp {
                let piece = match sg.proto_name {
                    Some(p) => format!("\"spawn {}: +\"∾(AnoTraceNum {})", self.rs(p), sg.tot),
                    None => format!("\"spawn: +\"∾(AnoTraceNum {})", sg.tot),
                };
                ann = Some(match ann {
                    Some(a) => format!("{}∾\", \"∾{}", a, piece),
                    None => piece,
                });
            }
            if fx.despawn {
                let piece = format!("\"kill: -\"∾(AnoTraceNum +´¬{})", keep.as_deref().unwrap_or(""));
                ann = Some(match ann {
                    Some(a) => format!("{}∾\", \"∾{}", a, piece),
                    None => piece,
                });
            }
            self.stage(format!(
                "•Out anoTraceSep∾\"s{}: \"∾(AnoTraceNum {})∾\" rows -> \"∾(AnoTraceNum anoN)∾\" (\"∾{}∾\")\"",
                self.stmt,
                pn,
                ann.unwrap_or_default()
            ));
        }
        Ok(())
    }
}

/* ---------- statements, rule ticks, queries, comprehensions ---------- */

impl<'a> Em<'a> {
    // The statement barrier (C emitStmt).
    fn emit_stmt(&mut self, st: &Node) -> R<()> {
        let NodeKind::Stmt { sel, effects, cont, elided, .. } = &st.kind else {
            return Ok(());
        };
        self.stmt += 1;
        self.site_kind = 's';
        self.out.push_str(&format!("\n# s{}\n", self.stmt));
        self.pre.clear();
        self.pipe_expand.clear();
        let mut fx = Fx::default();
        let is_cont = *cont || *elided;
        self.set_frame(if is_cont { None } else { sel.as_deref() });
        let sv = format!("s{}m", self.stmt);
        if is_cont && self.have_saved {
            if self.fr.kind == FrameKind::Ent {
                self.stage(format!("{} ← anoN↑anoSel", sv));
            } else {
                self.stage(format!("{} ← anoSel", sv));
            }
        } else if is_cont {
            // A cold elided effect is desugared to an explicit `^cursor` selection by the
            // normalizer. Reaching this arm therefore means a true continuation (`~` or a
            // leading comma) had no antecedent; those forms never receive the cold default.
            return Err(fail(st.line, "continuation with no antecedent"));
        } else {
            let pred = strip_frame(sel.as_deref());
            let msk = match pred {
                None => format!("(1¨↕{})", self.fr_n()),
                Some(p) => {
                    self.sel_var = sv.clone(); // for N_TO rhs inside predicates: none
                    self.emit_mask(p)?
                }
            };
            self.stage(format!("{} ← {}", sv, msk));
        }
        self.sel_var = sv.clone();
        self.trace_sel = sv.clone();
        for ef in effects {
            self.emit_effect(ef, &mut fx)?;
        }
        // save the antecedent before structural commits (pre-spawn mask, ex49)
        self.stage(format!("anoSel ↩ {}", sv));
        self.saved_fr = self.fr.clone();
        self.have_saved = true;
        self.commit_stmt(&fx, is_cont)?;
        if fx.despawn {
            self.world_shifted = true; // rows shifted: idx-keyed reads now invert anoIdx
        }
        let pre = std::mem::take(&mut self.pre);
        self.out.push_str(&pre);
        Ok(())
    }

    // Positive / negated name literals from a rule selection's top-level &-chain — the §11
    // disjointness certificates. Defs expand for positives; negated defs are skipped.
    fn guard_lits(&self, sel: Option<&Node>, pos: &mut Vec<usize>, neg: &mut Vec<usize>) {
        let Some(sel) = sel else { return };
        match &sel.kind {
            NodeKind::And(a, b) => {
                self.guard_lits(Some(a), pos, neg);
                self.guard_lits(Some(b), pos, neg);
            }
            NodeKind::Scope { l, .. } => self.guard_lits(Some(l), pos, neg),
            NodeKind::Name(s) | NodeKind::Alias { look: s, .. } => {
                if let Some(d) = self.find_def(*s) {
                    self.guard_lits(Some(def_body(d)), pos, neg);
                    return;
                }
                if let Some(e) = self.find(self.rs(*s)) {
                    if pos.len() < 16 {
                        pos.push(e);
                    }
                }
            }
            NodeKind::Not(k) => {
                if let NodeKind::Name(s) | NodeKind::Alias { look: s, .. } = &k.kind {
                    if self.find_def(*s).is_some() {
                        return;
                    }
                    if let Some(e) = self.find(self.rs(*s)) {
                        if neg.len() < 16 {
                            neg.push(e);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // The shared rule barrier (§11): every rule gathers the one pre-state, every effect
    // stages into one commit set, the set scatters once.
    fn emit_rule_tick(&mut self, rules: &[&'a Node]) -> R<()> {
        if rules.len() == 1 {
            return self.emit_stmt(rules[0]);
        }
        if rules.len() > 32 {
            return Err(fail(rules[0].line, "rule tick: more than 32 rules"));
        }
        self.stmt += 1;
        self.site_kind = 's';
        self.out.push_str(&format!("\n# s{}: {} rules, one shared barrier\n", self.stmt, rules.len()));
        self.pre.clear();
        self.pipe_expand.clear();
        let mut fx = Fx::default();
        let nr = rules.len();
        // pairwise disjointness certificates from complementary guard literals
        let mut pos: Vec<Vec<usize>> = vec![Vec::new(); nr];
        let mut neg: Vec<Vec<usize>> = vec![Vec::new(); nr];
        for r in 0..nr {
            self.guard_lits(stmt_sel(rules[r]), &mut pos[r], &mut neg[r]);
            self.rule_disj[r] = 0;
        }
        for r in 0..nr {
            for s in 0..nr {
                if r == s {
                    continue;
                }
                let dis = pos[r].iter().any(|p| neg[s].contains(p))
                    || neg[r].iter().any(|q| pos[s].contains(q));
                if dis {
                    self.rule_disj[r] |= 1u32 << s;
                }
            }
        }
        // one tick, one frame: every rule must select in the same habitat
        self.set_frame(stmt_sel(rules[0]));
        let f0 = self.fr.clone();
        for r in 1..nr {
            self.set_frame(stmt_sel(rules[r]));
            if self.fr.kind != f0.kind || self.fr.w != f0.w || self.fr.h != f0.h {
                return Err(fail(rules[r].line, "rule tick: rules select different frames"));
            }
        }
        self.fr = f0;
        // every mask against the one pre-state
        let mut masks: Vec<String> = Vec::with_capacity(nr);
        for (r, rule) in rules.iter().enumerate() {
            let pred = strip_frame(stmt_sel(rule));
            let msk = match pred {
                None => format!("(1¨↕{})", self.fr_n()),
                Some(p) => self.emit_mask(p)?,
            };
            let mv = format!("s{}r{}m", self.stmt, r);
            self.stage(format!("{} ← {}", mv, msk));
            masks.push(mv);
        }
        // every effect, staged into the one commit set; reads stay pre-state
        for (r, rule) in rules.iter().enumerate() {
            self.sel_var = masks[r].clone();
            self.trace_sel = masks[r].clone();
            self.cur_rule = r as i32;
            for ef in stmt_effects(rule) {
                if let Err(d) = self.emit_effect(ef, &mut fx) {
                    self.cur_rule = -1;
                    return Err(d);
                }
            }
        }
        self.cur_rule = -1;
        // the antecedent is the union of the tick's masks
        let mut uni = masks[0].clone();
        for mv in &masks[1..] {
            uni = format!("{}∨{}", uni, mv);
        }
        self.stage(format!("anoSel ↩ {}", uni));
        self.saved_fr = self.fr.clone();
        self.have_saved = true;
        self.sel_var = masks[0].clone();
        self.commit_stmt(&fx, false)?;
        if fx.despawn {
            self.world_shifted = true;
        }
        let pre = std::mem::take(&mut self.pre);
        self.out.push_str(&pre);
        Ok(())
    }

    // Queries: q<N> value, --label 0x1D tag, the next --! out pin, else the plain display.
    // An identityless fold over nothing has no result row (A12): a validity-tagged guard is
    // consumed BEFORE the label, the expectation, and the display.  A unit result stages the
    // guard as q<N>v and every observation runs under it; a grouped result compresses its rows
    // away, exactly as the assignment path already drops them from the scatter mask.  A query
    // with no validity guard stages byte-identically to an unguarded one.
    fn emit_query(&mut self, st: &Node) -> R<()> {
        let NodeKind::Query(inner) = &st.kind else {
            return Ok(());
        };
        self.stmt += 1;
        self.site_kind = 'q';
        self.pre.clear();
        // a query is a Predicate-phase crossing over X: no statement mask is in scope, and
        // the previous statement's must not leak in
        self.trace_sel.clear();
        self.out.push_str(&format!("\n# q{}\n", self.stmt));
        self.set_frame(Some(inner));
        let v = self.emit_val(inner, Mode::World)?;
        let qv = format!("q{}", self.stmt);
        let validity = match (&v.g, v.gv) {
            (Some(g), true) if v.unit => {
                let vv = format!("q{}v", self.stmt);
                self.stage(format!("{} ← {}", qv, v.v));
                self.stage(format!("{} ← {}", vv, g));
                Some(vv)
            }
            (Some(g), true) => {
                // per-row validity: an empty fiber produces no result row
                self.stage(format!("{} ← ({})/{}", qv, g, v.v));
                None
            }
            _ => {
                self.stage(format!("{} ← {}", qv, v.v));
                None
            }
        };
        // --label: a 0x1D tag line names the query and its source line, then the display —
        // for every query, pinned or not, before any assertion.  Under a false guard Kore
        // receives no QRec at all, so neither tag nor value may escape the conditional.
        if self.dirs.label {
            match &validity {
                Some(vv) => self.stage(format!(
                    "{{•Out (@+29)∾\"q{}@{}\" ⋄ •Show 𝕩}}⍟{} {}",
                    self.stmt, st.line, vv, qv
                )),
                None => {
                    self.stage(format!("•Out (@+29)∾\"q{}@{}\"", self.stmt, st.line));
                    self.stage(format!("•Show {}", qv));
                }
            }
        }
        // match against the next --! out expectation
        let mut oi: Option<usize> = None;
        let mut seen = 0usize;
        for (i, ex) in self.dirs.expects.iter().enumerate() {
            if matches!(ex, Expect::Out { .. }) {
                if seen == self.out_idx {
                    oi = Some(i);
                    break;
                }
                seen += 1;
            }
        }
        if let Some(i) = oi {
            self.out_idx += 1;
            let Expect::Out { vals } = &self.dirs.expects[i] else {
                return Ok(());
            };
            let mut all_num = true;
            let mut all_str = true;
            let mut lst = String::from("⟨");
            for (k, w) in vals.iter().enumerate() {
                let b0 = w.as_bytes().first().copied().unwrap_or(0);
                let piece = if b0.is_ascii_digit() || b0 == b'-' || b0 == b'.' {
                    all_str = false;
                    if b0 == b'-' { format!("¯{}", &w[1..]) } else { w.clone() }
                } else {
                    all_num = false;
                    format!("\"{}\"", w)
                };
                if k > 0 {
                    lst.push_str(", ");
                }
                lst.push_str(&piece);
            }
            lst.push('⟩');
            // the comparator sees the guarded ravel, never the backend placeholder: a false
            // guard makes `--! out` (empty) pass and `--! out 0` fail
            let ravel = match &validity {
                Some(vv) => format!("{}/⥊{}", vv, qv),
                None => format!("⥊{}", qv),
            };
            // numeric outs compare within 1e-9: pinned doubles come from a sibling BQN
            // evaluation whose association order may differ in the last bits
            if all_num {
                self.stage(format!(
                    "\"out q{}\" ! {} {{(≠𝕨)≠≠𝕩 ? 0 ; ∧´1e¯9≥|𝕨-𝕩}} {}",
                    self.stmt, lst, ravel
                ));
            } else if all_str {
                // a wholly textual pin reads either as the list of words a sym result answers
                // with or as the glyph run a char result is, exactly as `--! expect` already
                // reads a char column; joining decides which without a second directive
                self.stage(format!(
                    "\"out q{}\" ! {} {{(𝕨≡𝕩)∨(∾𝕨)≡𝕩}} {}",
                    self.stmt, lst, ravel
                ));
            } else {
                self.stage(format!("\"out q{}\" ! {} ≡ {}", self.stmt, lst, ravel));
            }
        } else if !self.dirs.label {
            match &validity {
                Some(vv) => self.stage(format!("•Show⍟{} {}", vv, qv)),
                None => self.stage(format!("•Show {}", qv)),
            }
        }
        let pre = std::mem::take(&mut self.pre);
        self.out.push_str(&pre);
        Ok(())
    }

    // Comprehension: theta-join over two generators with filters; effects tag both sides.
    fn emit_compr(&mut self, st: &Node) -> R<()> {
        let NodeKind::Compr { effect, rest, .. } = &st.kind else {
            return Ok(());
        };
        self.stmt += 1;
        self.site_kind = 'c';
        self.pre.clear();
        self.out.push_str(&format!("\n# c{}\n", self.stmt));
        self.fr.kind = FrameKind::Ent;
        let mut binders: Vec<&Node> = Vec::new();
        let mut filters: Vec<&Node> = Vec::new();
        for k in rest {
            if matches!(k.kind, NodeKind::Binder { .. }) {
                if binders.len() < 2 {
                    binders.push(k);
                }
            } else {
                filters.push(k);
            }
        }
        if binders.len() != 2 {
            return Err(fail(st.line, "comprehension needs two generators"));
        }
        let (b0name, b0src) = if let NodeKind::Binder { name, source } = &binders[0].kind {
            (*name, source.as_ref())
        } else {
            (Symbol::EMPTY, binders[0])
        };
        let (b1name, b1src) = if let NodeKind::Binder { name, source } = &binders[1].kind {
            (*name, source.as_ref())
        } else {
            (Symbol::EMPTY, binders[1])
        };
        let am = self.emit_mask(b0src)?;
        let bm = self.emit_mask(b1src)?;
        let a_i = self.tv();
        let b_i = self.tv();
        self.stage(format!("{} ← /{}", a_i, am));
        self.stage(format!("{} ← /{}", b_i, bm));
        // pair filter matrix, all-ones then AND each filter in
        let mm = self.tv();
        self.stage(format!("{} ← (≠{})‿(≠{})⥊1", mm, a_i, b_i));
        for f in &filters {
            match &f.kind {
                NodeKind::Cmp { op, l, r }
                    if matches!(&l.kind, NodeKind::Name(s) if *s == b0name)
                        && matches!(&r.kind, NodeKind::Name(s) if *s == b1name) =>
                {
                    let opg = match op {
                        CmpOp::Lt => "<",
                        CmpOp::Gt => ">",
                        CmpOp::Eq => "=",
                        _ => "≠",
                    };
                    self.stage(format!("{} ↩ {}∧{}{}⌜{}", mm, mm, a_i, opg, b_i));
                }
                NodeKind::Cmp { op, l, r } if matches!(l.kind, NodeKind::Call { .. }) => {
                    let NodeKind::Call { callee, .. } = &l.kind else {
                        return Err(fail(f.line, "unsupported comprehension filter"));
                    };
                    let cn = self.rs(*callee);
                    let Some(ei) = self.find(cn) else {
                        return Err(fail(f.line, format!("unregistered '{}' in comprehension filter", cn)));
                    };
                    let rv = self.emit_val(r, Mode::World)?;
                    let opg = match op {
                        CmpOp::Lt => "<",
                        CmpOp::Gt => ">",
                        CmpOp::Le => "≤",
                        _ => "≥",
                    };
                    let fv = self.fnv(ei);
                    self.stage(format!("{} ↩ {}∧(({} {}⌜ {}){}{})", mm, mm, a_i, fv, b_i, opg, rv.v));
                }
                NodeKind::Call { callee, .. } => {
                    let cn = self.rs(*callee);
                    let Some(ei) = self.find(cn) else {
                        return Err(fail(f.line, format!("unregistered '{}' in comprehension filter", cn)));
                    };
                    let fv = self.fnv(ei);
                    self.stage(format!("{} ↩ {}∧({} {}⌜ {})", mm, mm, a_i, fv, b_i));
                }
                _ => return Err(fail(f.line, "unsupported comprehension filter")),
            }
        }
        // effect over both sides: rows/cols with any surviving pair
        let a_any = self.tv();
        let b_any = self.tv();
        self.stage(format!("{} ← ∨´˘{}", a_any, mm));
        self.stage(format!("{} ← ∨´˘⍉{}", b_any, mm));
        let side = self.tv();
        self.stage(format!(
            "{} ← ((↕anoN)∊{}/{})∨((↕anoN)∊{}/{})",
            side, a_any, a_i, b_any, b_i
        ));
        self.sel_var = side.clone();
        self.trace_sel = side.clone();
        let mut fx = Fx::default();
        // bare verb tag (Collide): treat as +Name when a bool col exists
        let mut handled = false;
        if let NodeKind::EVerb { name, args } = &effect.kind {
            if args.is_empty() {
                if let Some(ci) = self.find(self.rs(*name)) {
                    if matches!(self.ent(ci).kind, RegEntryKind::Col { ty: ColType::Bool, .. }) {
                        let t = self.tv();
                        let cv = self.bqnv(ci);
                        self.stage(format!("{} ← {}∨{}", t, cv, side));
                        self.add_commit(&mut fx, ci, t, b'|', None);
                        handled = true;
                    }
                }
            }
        }
        if !handled {
            self.emit_effect(effect, &mut fx)?;
        }
        self.stage(format!("anoSel ↩ {}", side));
        self.saved_fr = self.fr.clone();
        self.have_saved = true;
        self.commit_stmt(&fx, false)?;
        if fx.despawn {
            self.world_shifted = true;
        }
        let pre = std::mem::take(&mut self.pre);
        self.out.push_str(&pre);
        Ok(())
    }
}

/* ---------- fixture, expectations, save ---------- */

impl<'a> Em<'a> {
    // One column/field fixture line (+ presence), pair detection seeding is_pair.
    fn fixture_colfield(&mut self, i: usize, ty: ColType, nums: &[f64], syms: &[String], pres: Option<&[f64]>, nlen: i32, cm: &str) {
        let v = self.bqnv(i);
        match ty {
            ColType::Sym => {
                self.out.push_str(&format!("{} ← ⟨", v));
                for (k, s) in syms.iter().enumerate() {
                    self.out.push_str(&format!("{}\"{}\"", if k > 0 { ", " } else { "" }, s));
                }
                self.out.push_str(&format!("⟩{}\n", cm));
            }
            ColType::Char => {
                self.out.push_str(&format!("{} ← \"{}\"{}\n", v, syms.first().map(|s| s.as_str()).unwrap_or(""), cm));
            }
            _ => {
                if nlen > 0 && nums.len() == 2 * nlen as usize {
                    self.is_pair[i] = true;
                    self.out.push_str(&format!("{} ← ⟨", v));
                    for k in 0..nlen as usize {
                        let a = num_lit(nums[2 * k]);
                        let b = num_lit(nums[2 * k + 1]);
                        self.out.push_str(&format!("{}{}‿{}", if k > 0 { ", " } else { "" }, a, b));
                    }
                    self.out.push_str(&format!("⟩{}\n", cm));
                } else {
                    self.out.push_str(&format!("{} ← ⟨", v));
                    for (k, x) in nums.iter().enumerate() {
                        self.out.push_str(&format!("{}{}", if k > 0 { ", " } else { "" }, num_lit(*x)));
                    }
                    self.out.push_str(&format!("⟩{}\n", cm));
                }
            }
        }
        if let Some(p) = pres {
            let pv = self.presv(i);
            self.out.push_str(&format!("{} ← ⟨", pv));
            for k in 0..self.reg.n as usize {
                let x = p.get(k).copied().unwrap_or(0.0);
                self.out.push_str(&format!("{}{}", if k > 0 { ", " } else { "" }, num_lit(x)));
            }
            self.out.push_str(&format!("⟩{}\n", cm));
        }
    }

    // The fixture block: anoN/anoSel/anoIdx, one line per entry in declaration order,
    // the --trace prelude.
    fn emit_fixture(&mut self) {
        let r = self.reg;
        self.out.push_str("\n# fixture\n");
        self.out.push_str(&format!("anoN ← {}\n", r.n));
        self.out.push_str("anoSel ← ⟨⟩\n");
        // the prologue's place, reserved before any statement can ask for a declaration
        self.decl_at = self.out.len();
        // the hidden idx key: the row iota materialized ONCE, then carried through every
        // structural commit — never reminted at use
        if self.need_idx {
            self.out.push_str("anoIdx ← ↕anoN\n");
        }
        for i in 0..r.ents.len() {
            let e = self.ent(i);
            let v = self.bqnv(i);
            // mangled entries keep their human spelling as a comment on the definition line
            let cm = if bqnlegal(&e.name) { String::new() } else { format!("  # {}", e.name) };
            match &e.kind {
                RegEntryKind::Col { ty, nums, syms, pres, .. } => {
                    self.fixture_colfield(i, *ty, nums, syms, pres.as_deref(), r.n, &cm);
                }
                RegEntryKind::Field { ty, nums, syms, .. } => {
                    self.fixture_colfield(i, *ty, nums, syms, None, r.lat_w.wrapping_mul(r.lat_h), &cm);
                }
                RegEntryKind::Rel { targets, .. } => {
                    self.out.push_str(&format!("{} ← ⟨", v));
                    for (k, x) in targets.iter().enumerate() {
                        self.out.push_str(&format!("{}{}", if k > 0 { ", " } else { "" }, num_lit(*x)));
                    }
                    self.out.push_str(&format!("⟩{}\n", cm));
                }
                RegEntryKind::AliasMask { mask } => {
                    self.out.push_str(&format!("{} ← ⟨", v));
                    for (k, x) in mask.iter().enumerate() {
                        self.out.push_str(&format!("{}{}", if k > 0 { ", " } else { "" }, num_lit(*x)));
                    }
                    self.out.push_str(&format!("⟩{}\n", cm));
                }
                RegEntryKind::SRel { fib, .. } => {
                    self.out.push_str(&format!("{} ← ⟨", v));
                    for (f, fiber) in fib.iter().enumerate() {
                        self.out.push_str(&format!("{}⟨", if f > 0 { ", " } else { "" }));
                        for (k, x) in fiber.iter().enumerate() {
                            self.out.push_str(&format!("{}{}", if k > 0 { ", " } else { "" }, num_lit(*x)));
                        }
                        self.out.push_str("⟩");
                    }
                    self.out.push_str(&format!("⟩{}\n", cm));
                }
                RegEntryKind::Bind { kind, vals } => match kind {
                    BindKind::Mask | BindKind::Vec => {
                        self.out.push_str(&format!("{} ← ⟨", v));
                        for (k, x) in vals.iter().enumerate() {
                            self.out.push_str(&format!("{}{}", if k > 0 { ", " } else { "" }, num_lit(*x)));
                        }
                        self.out.push_str(&format!("⟩{}\n", cm));
                    }
                    BindKind::Num => {
                        self.out.push_str(&format!("{} ← {}{}\n", v, num_lit(vals.first().copied().unwrap_or(0.0)), cm));
                    }
                    _ => {}
                },
                RegEntryKind::Fn { body: Some(raw) } if !raw.is_empty() => {
                    // raw form: either "<dfn>" or "<targetcol> <dfn>" (verbs) — bind the dfn part
                    if let Some(bp) = raw.find('{') {
                        let fv = self.fnv(i);
                        if bp != 0 {
                            self.out.push_str(&format!("{} ← {}{}\n", fv, &raw[bp..], cm));
                        } else {
                            self.out.push_str(&format!("{} ← {}{}\n", fv, raw, cm));
                        }
                    }
                }
                RegEntryKind::TypedFn { body, .. } => {
                    let function = self.fnv(i);
                    self.out
                        .push_str(&format!("{} ← {}{}\n", function, body, cm));
                }
                _ => {}
            }
        }
        // --trace: the debug observability prelude — 0x1F-prefixed diagnostic lines
        if self.dirs.trace {
            self.out.push_str("\n# trace (--trace): 0x1F-prefixed diagnostic lines\n");
            self.out.push_str("anoTraceSep ← @+31\n");
            self.out.push_str("AnoTraceNum ← {∾{𝕩='¯' ? \"-\" ; ⋈𝕩}¨•Repr 𝕩}\n");
            // u is the use suffix: one crossing, many rows, all sharing one USE id
            self.out.push_str("AnoTraceDead ← {n‿o‿s‿u: o {•Out anoTraceSep∾\"RELATION \"∾n∾\" \"∾(AnoTraceNum 𝕨)∾\" -> \"∾(AnoTraceNum 𝕩)∾\" IS DEAD ! \"∾u}¨ s}\n");
            self.out.push_str("AnoTraceEmpty ← {n‿o‿u: {•Out anoTraceSep∾\"FIBER \"∾n∾\" \"∾(AnoTraceNum 𝕩)∾\" IS EMPTY ! \"∾u}¨ o}\n");
        }
    }

    // The expectations block; the two direct diagnostics carry no "emit: line" prefix.
    fn emit_expects(&mut self) -> R<()> {
        self.out.push_str("\n# expectations\n");
        for ex in self.dirs.expects.iter() {
            let Expect::Col { col, vals } = ex else { continue };
            let Some(ei) = self.find(col) else {
                return Err(Diag::refuse(format!("expect: unknown column '{}'", col)));
            };
            let e = self.ent(ei);
            if let RegEntryKind::Tag { col: carrier, .. } = &e.kind {
                return Err(Diag::refuse(format!(
                    "expect: '{}' is a derived tag; pin the carrier column '{}'",
                    col, carrier
                )));
            }
            let v = self.bqnv(ei);
            let sym = col_ty(e) == Some(ColType::Sym);
            let chr = col_ty(e) == Some(ColType::Char);
            if chr {
                // char column: the pin is the glyph run (space-joined when written in parts)
                let s = vals.join(" ");
                self.out.push_str(&format!("\"expect {}\" ! \"{}\" ≡ {}\n", col, s, v));
                continue;
            }
            let mut lst = String::from("⟨");
            for (k, w) in vals.iter().enumerate() {
                let piece = if sym {
                    format!("\"{}\"", w)
                } else if w.starts_with('-') {
                    format!("¯{}", &w[1..])
                } else {
                    w.clone()
                };
                if k > 0 {
                    lst.push_str(", ");
                }
                lst.push_str(&piece);
            }
            lst.push('⟩');
            let rhs = if self.is_pair[ei] { format!("∾{}", v) } else { v };
            // numeric expects compare within 1e-9; sym columns stay exact
            if sym {
                self.out.push_str(&format!("\"expect {}\" ! {} ≡ {}\n", col, lst, rhs));
            } else {
                self.out.push_str(&format!(
                    "\"expect {}\" ! {} {{(≠𝕨)≠≠𝕩 ? 0 ; ∧´1e¯9≥|𝕨-𝕩}} {}\n",
                    col, lst, rhs
                ));
            }
        }
        if self.dirs.expect_n >= 0 {
            self.out.push_str(&format!("\"expect-n\" ! {} ≡ anoN\n", self.dirs.expect_n));
        }
        self.out.push_str("\n\"ok\"\n");
        Ok(())
    }

    // The --save pipe-back serializer: 0x1E-prefixed post-state lines, data-carrying entries
    // in declaration order; schema never pipes; inverse srels skipped.
    fn emit_save(&mut self) {
        self.out.push_str("\n# save pipe-back (--save): 0x1E-prefixed post-state lines\n");
        self.out.push_str("anoSaveSep ← @+30\n");
        self.out.push_str("AnoSaveNum ← {∾{𝕩='¯' ? \"-\" ; ⋈𝕩}¨•Repr 𝕩}\n");
        self.out.push_str("AnoSaveRow ← {∾{\" \"∾𝕩}¨𝕩}\n");
        self.out.push_str("•Out anoSaveSep∾\"n \"∾AnoSaveNum anoN\n");
        for i in 0..self.reg.ents.len() {
            let e = self.ent(i);
            let v = self.bqnv(i);
            match &e.kind {
                RegEntryKind::Col { ty, pres, .. } => {
                    let ty = *ty;
                    let hp = pres.is_some();
                    self.save_colfield("col", i, ty, hp, &v, &e.name);
                }
                RegEntryKind::Field { ty, .. } => {
                    let ty = *ty;
                    self.save_colfield("field", i, ty, false, &v, &e.name);
                }
                RegEntryKind::Rel { .. } => {
                    self.out.push_str(&format!("•Out anoSaveSep∾\"rel {}\"∾AnoSaveRow AnoSaveNum¨{}\n", e.name, v));
                }
                RegEntryKind::SRel { inv_of, .. } => {
                    if inv_of.is_some() {
                        continue;
                    }
                    self.out.push_str(&format!(
                        "•Out anoSaveSep∾\"srel {}\"∾2↓∾{{\" |\"∾AnoSaveRow AnoSaveNum¨𝕩}}¨{}\n",
                        e.name, v
                    ));
                }
                RegEntryKind::AliasMask { .. } => {
                    self.out.push_str(&format!(
                        "•Out anoSaveSep∾\"alias {}\"∾AnoSaveRow AnoSaveNum¨{}\n",
                        e.name, v
                    ));
                }
                RegEntryKind::Bind { kind: BindKind::Mask, .. } => {
                    self.out.push_str(&format!(
                        "•Out anoSaveSep∾\"bindmask {}\"∾AnoSaveRow AnoSaveNum¨{}\n",
                        e.name, v
                    ));
                }
                _ => {}
            }
        }
    }

    // One col/field save line (+ presence).
    fn save_colfield(&mut self, kw: &str, i: usize, ty: ColType, hp: bool, v: &str, name: &str) {
        match ty {
            ColType::Sym => {
                self.out.push_str(&format!("•Out anoSaveSep∾\"{} {}\"∾AnoSaveRow {}\n", kw, name, v));
            }
            ColType::Char => {
                self.out.push_str(&format!("•Out anoSaveSep∾\"{} {} \"∾{}\n", kw, name, v));
            }
            _ => {
                if self.is_pair[i] {
                    self.out.push_str(&format!("•Out anoSaveSep∾\"{} {}\"∾AnoSaveRow AnoSaveNum¨∾{}\n", kw, name, v));
                } else {
                    self.out.push_str(&format!("•Out anoSaveSep∾\"{} {}\"∾AnoSaveRow AnoSaveNum¨{}\n", kw, name, v));
                }
            }
        }
        if hp {
            let pv = self.presv(i);
            self.out.push_str(&format!("•Out anoSaveSep∾\"pres {}\"∾AnoSaveRow AnoSaveNum¨{}\n", name, pv));
        }
    }

    /* ---------- the idx pre-scan ---------- */

    // The N_NAME leg of scanIdxUse: defs expand (depth-capped), unkeyed functional rels hit.
    fn scan_idx_name(&self, s: Symbol, has_id: bool, depth: i32) -> bool {
        if let Some(d) = self.find_def(s) {
            return self.scan_idx_use(def_body(d), has_id, depth + 1);
        }
        if let Some(ei) = self.find(self.rs(s)) {
            if let RegEntryKind::Rel { key_of, .. } = &self.ent(ei).kind {
                if key_of.is_none() {
                    return true;
                }
            }
        }
        false
    }

    // 1 when the subtree reads through the idx key space: an unkeyed functional rel by name,
    // or a set-hop over an unkeyed world-length srel / key column when the world has no id.
    fn scan_idx_use(&self, nd: &Node, has_id: bool, depth: i32) -> bool {
        if depth > 16 {
            return false;
        }
        match &nd.kind {
            NodeKind::Name(s) | NodeKind::Alias { look: s, .. } => return self.scan_idx_name(*s, has_id, depth),
            NodeKind::SetHop { rel } => {
                if !has_id {
                    if let Some(ei) = self.find(self.rs(*rel)) {
                        let hit = match &self.ent(ei).kind {
                            RegEntryKind::SRel { key_of, fib, .. } => {
                                key_of.is_none() && fib.len() == self.reg.n as usize
                            }
                            RegEntryKind::Col { .. } => true,
                            _ => false,
                        };
                        if hit {
                            return true;
                        }
                    }
                }
                return self.scan_idx_name(*rel, has_id, depth);
            }
            NodeKind::CmpAny { name } => return self.scan_idx_name(*name, has_id, depth),
            _ => {}
        }
        children(nd).into_iter().any(|k| self.scan_idx_use(k, has_id, depth))
    }

    // Whole-program pre-scan arming anoIdx: statements strictly after the first
    // despawn-carrying barrier count; once any despawn exists every installed rule counts.
    fn scan_need_idx(&mut self, kids: &'a [Node]) {
        let any = kids.iter().any(|k| scan_despawn(k));
        if !any {
            return;
        }
        // the same reg_role ladder idCol resolves through
        let mut ide = self.role("id");
        if ide.is_none() {
            ide = self.role("keys");
        }
        if let Some(i) = ide {
            if !matches!(self.ent(i).kind, RegEntryKind::Col { .. }) {
                ide = None;
            }
        }
        let has_id = ide.is_some();
        // defs visible to the whole scan; the real emission re-adds them in order
        for st in kids {
            if matches!(st.kind, NodeKind::DefStmt { .. }) && self.defs.len() < 128 {
                self.defs.push(st);
            }
        }
        let mut shifted = false;
        for st in kids {
            if self.need_idx {
                break;
            }
            let body: &Node = if let NodeKind::DefStmt { body, .. } = &st.kind { body } else { st };
            let is_rule = matches!(body.kind, NodeKind::Stmt { rule: true, .. });
            if is_rule {
                if self.scan_idx_use(body, has_id, 0) {
                    self.need_idx = true;
                }
                if scan_despawn(body) {
                    shifted = true; // its first edge precedes later statements
                }
            } else {
                if shifted && self.scan_idx_use(st, has_id, 0) {
                    self.need_idx = true;
                }
                if scan_despawn(st) {
                    shifted = true;
                }
            }
        }
        self.defs.clear();
    }
}

/* ---------- entry ---------- */

// Inputs: the Program node, loaded registry, directives, interner (resolve-only).
// Output: the complete BQN program text (rt.bqn is prepended by the driver) or Diag.
// Invariants: fixture block first, then per top-level node one s<N>/q<N>/c<N> block (one
// shared stmt counter), then expectations (terminal "\n\"ok\"\n"), then the --save serializer;
// the def-install / rule-tick clock edges fire per the C driver; --save/--label/--trace OFF
// means byte-identical output to a flagless emit — every hook gated before any staging;
// temp numbering is part of the bytes (the N_EASSIGN guard probe BURNS temps: run the probe
// for real into a discarded buffer, keep the counter advanced); diagnostics render
// "emit: line %d: %s" except the four direct sites ("expect: ..." x2, "emit: more than 32
// installed rules", "emit: unexpected top-level node %d"); entry INDICES replace every C
// pointer identity (entIdx, base != cur committed-flags, reg_role/find comparisons, guardLits
// certificates); registry declaration order is emission order; numbers spell via the C
// numLit (int fast path / %.17g via num::fmt_g, '-' -> "¯", '+' dropped); quirks replicate,
// never repair (dead uc fallback, duplicated grade split, stripFrame fallthrough, line-0
// despawn diagnostic, emitCompr l/g -> ≠, MAXLITS 16 silent drop, board-frame rule-tick
// duplicate brd lines).
fn emit_lowered(
    prog: &Node,
    reg: &Registry,
    dirs: &Directives,
    it: &Interner,
    plan: crate::trace::TracePlan,
) -> Result<(String, crate::trace::TracePlan), Diag> {
    let mut em = Em {
        reg,
        dirs,
        it,
        out: String::new(),
        pre: String::new(),
        defs: Vec::new(),
        is_pair: vec![false; reg.ents.len()],
        fr: Frame::ent(),
        saved_fr: Frame::ent(),
        have_saved: false,
        cur_rule: -1,
        rule_disj: [0; 32],
        sel_var: String::new(),
        trace_sel: String::new(),
        cnt_var: String::new(),
        idx_var: String::new(),
        pipe_expand: String::new(),
        need_idx: false,
        world_shifted: false,
        tmp: 0,
        out_idx: 0,
        stmt: 0,
        decls: Vec::new(),
        decl_at: 0,
        rel_seals: 0,
        plan,
        in_effect: false,
        site_kind: 's',
    };
    let empty: [Node; 0] = [];
    let kids: &[Node] = if let NodeKind::Program(v) = &prog.kind { v } else { &empty };
    em.scan_need_idx(kids);
    em.emit_fixture();
    // Installation and retraction are host-control edges. A run of new installations fires at
    // the next barrier; `undef name` reaches that barrier first, then removes exactly that rule.
    let mut installed: Vec<(Option<Symbol>, &Node)> = Vec::new();
    let mut fresh = false;
    for st in kids {
        let mut rule: Option<(Option<Symbol>, &Node)> = None;
        match &st.kind {
            NodeKind::DefStmt { name, body } => {
                if em.defs.len() >= 128 {
                    return Err(fail(st.line, "more than 128 defs"));
                }
                em.defs.push(st);
                if matches!(body.kind, NodeKind::Stmt { .. }) {
                    rule = Some((Some(*name), body));
                }
            }
            NodeKind::Stmt { rule: true, .. } => rule = Some((None, st)),
            NodeKind::UndefStmt { name } => {
                if fresh {
                    let active = installed.iter().map(|(_, rule)| *rule).collect::<Vec<_>>();
                    em.emit_rule_tick(&active)?;
                    fresh = false;
                }
                let Some(index) = installed
                    .iter()
                    .position(|(installed_name, _)| *installed_name == Some(*name))
                else {
                    return Err(fail(
                        st.line,
                        format!("rule '{}' is not installed", it.resolve(*name)),
                    ));
                };
                installed.remove(index);
                em.defs.retain(|definition| {
                    !matches!(
                        definition.kind,
                        NodeKind::DefStmt {
                            name: definition_name,
                            ..
                        } if definition_name == *name
                    )
                });
                continue;
            }
            _ => {}
        }
        if let Some((name, body)) = rule {
            if installed.len() >= 32 {
                return Err(Diag::refuse("emit: more than 32 installed rules"));
            }
            if let Some(name) = name
                && installed
                    .iter()
                    .any(|(installed_name, _)| *installed_name == Some(name))
            {
                return Err(fail(
                    st.line,
                    format!("rule '{}' is already installed", it.resolve(name)),
                ));
            }
            installed.push((name, body));
            fresh = true;
            continue;
        }
        if matches!(st.kind, NodeKind::DefStmt { .. }) {
            continue;
        }
        if fresh {
            let active = installed.iter().map(|(_, rule)| *rule).collect::<Vec<_>>();
            em.emit_rule_tick(&active)?;
            fresh = false;
        }
        match &st.kind {
            NodeKind::Stmt { .. } => em.emit_stmt(st)?,
            NodeKind::Query(_) => em.emit_query(st)?,
            NodeKind::Compr { .. } => em.emit_compr(st)?,
            k => return Err(Diag::refuse(format!("emit: unexpected top-level node {}", k.c_kind()))),
        }
    }
    if fresh {
        let active = installed.iter().map(|(_, rule)| *rule).collect::<Vec<_>>();
        em.emit_rule_tick(&active)?;
    }
    em.emit_expects()?;
    if dirs.save {
        em.emit_save();
    }
    em.flush_declarations();
    audit_relationship_writes(&em.out, reg, em.rel_seals)?;
    Ok((em.out, em.plan))
}

// Inputs: the text right after a publication's `↩ `. Output: the serial of the staged binding it
// publishes, or None for anything else — including a longer name that merely starts that way.
fn staged_serial(rhs: &str) -> Option<usize> {
    let digits = rhs.strip_prefix("anoRelStage")?;
    let end = digits.find(|c: char| !c.is_ascii_digit()).unwrap_or(digits.len());
    if end == 0 || digits[end..].starts_with(ident_char) {
        return None;
    }
    digits[..end].parse().ok()
}

// A BQN identifier character: the boundary a bare name must sit inside.
fn ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

// Inputs: the assembled program, the registry, and the number of relationship writes the
// lowering staged. Output: () when every `↩` to a relationship variable in the text publishes a
// distinct one of those staged bindings; a refusal naming the relation otherwise.
// The scan reads the artifact rather than the lowering's own bookkeeping, and it shares none of
// the staging path's assumptions: it finds a publication wherever it sits — its own line, nested
// in a lambda, sequenced after another expression — so a write spelled some other way has no
// stage to publish and is refused instead of quietly skipping the seal. Serials reconcile by
// identity, so a doubled publication cannot pay for a dropped one.
fn audit_relationship_writes(bqn: &str, reg: &Registry, staged: usize) -> R<()> {
    let mut published = vec![false; staged];
    for (i, e) in reg.ents.iter().enumerate() {
        let watched = match &e.kind {
            RegEntryKind::Rel { .. } => true,
            RegEntryKind::SRel { inv_of, .. } => inv_of.is_none(),
            _ => false,
        };
        if !watched {
            continue;
        }
        let var = if bqnlegal(&e.name) { lc(&e.name) } else { format!("jp{}", i) };
        let write = format!("{} ↩ ", var);
        let mut at = 0usize;
        while let Some(hit) = bqn[at..].find(&write) {
            let hit = at + hit;
            at = hit + write.len();
            // pres_mentor is not mentor: the match must start at a name boundary
            if bqn[..hit].ends_with(ident_char) {
                continue;
            }
            match staged_serial(&bqn[at..]).filter(|n| *n < staged) {
                // one staged binding publishes once; a repeat is a second write past the seal
                Some(n) if !std::mem::replace(&mut published[n], true) => {}
                _ => {
                    return Err(Diag::refuse(format!(
                        "emit: relationship '{}' is written outside the validity seal",
                        e.name
                    )));
                }
            }
        }
    }
    let reached = published.iter().filter(|p| **p).count();
    if reached != staged {
        return Err(Diag::refuse(format!(
            "emit: {} relationship writes were staged but {} reached the world",
            staged, reached
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // The write reconciliation, read on text: a publication that is not a staged binding is
    // refused, and a staged binding that never reached its variable is refused from the other
    // side.  This is what makes a missed write loud instead of silent.
    #[test]
    fn relationship_writes_reconcile_against_the_emitted_text() {
        let reg = Registry {
            n: 1,
            ents: vec![RegEntry {
                name: "Mentor".to_string(),
                defval: 0.0,
                kind: RegEntryKind::Rel { targets: vec![0.0], key_of: None },
            }],
            ..Registry::default()
        };
        let sealed = "anoRelStage0 ← t0\n\"relationship Mentor\" ! 1\nmentor ↩ anoRelStage0\n";
        assert!(audit_relationship_writes(sealed, &reg, 1).is_ok());
        let bare = "mentor ↩ t0\n";
        let refusal = audit_relationship_writes(bare, &reg, 0).expect_err("bare write").msg;
        assert!(refusal.contains("'Mentor' is written outside the validity seal"), "{}", refusal);
        let unpublished = audit_relationship_writes("anoRelStage0 ← t0\n", &reg, 1)
            .expect_err("unpublished stage")
            .msg;
        assert!(unpublished.contains("1 relationship writes were staged"), "{}", unpublished);
    }

    // The shapes a write could hide in.  A publication is a publication wherever it sits, one
    // staged binding publishes once, and a longer name that merely contains the variable is not
    // a write to it.
    #[test]
    fn a_write_cannot_hide_from_the_reconciliation() {
        let reg = Registry {
            n: 1,
            ents: vec![RegEntry {
                name: "Mentor".to_string(),
                defval: 0.0,
                kind: RegEntryKind::Rel { targets: vec![0.0], key_of: None },
            }],
            ..Registry::default()
        };
        let nested = "t0 ← {mentor ↩ 𝕩}¨fib\n";
        let refusal = audit_relationship_writes(nested, &reg, 0).expect_err("write in a lambda").msg;
        assert!(refusal.contains("'Mentor' is written outside the validity seal"), "{}", refusal);
        let sequenced = "t0 ← ⟨⟩ ⋄ mentor ↩ t1\n";
        assert!(audit_relationship_writes(sequenced, &reg, 0).is_err(), "sequenced write");
        // one stage, published twice: the second publication is a write the seal never covered
        let doubled = "anoRelStage0 ← t0\nmentor ↩ anoRelStage0\nmentor ↩ anoRelStage0\n";
        assert!(audit_relationship_writes(doubled, &reg, 2).is_err(), "doubled publication");
        // a staged binding whose name only starts like one is not that binding
        let lookalike = "mentor ↩ anoRelStage0b\n";
        assert!(audit_relationship_writes(lookalike, &reg, 1).is_err(), "lookalike stage");
        // and a wider name that ends in the variable is not the variable
        let other = "anoRelStage0 ← t0\nmentor ↩ anoRelStage0\npres_mentor ↩ t1\namentor ↩ t2\n";
        assert!(audit_relationship_writes(other, &reg, 1).is_ok(), "neighbouring names");
    }

    // Native byte-exact emission contract for a symbol query over the empty world.
    #[test]
    fn sym_query_emits_the_native_contract() {
        let mut it = Interner::new();
        let foo = it.intern("Foo");
        let q = Node::new(
            NodeKind::Query(Box::new(Node::new(NodeKind::Sym(foo), 1))),
            1,
        );
        let prog = Node::new(NodeKind::Program(vec![q]), 1);
        let reg = Registry::default();
        let dirs = Directives::default();
        let out = emit(&prog, &reg, &dirs, &it).expect("emit");
        assert_eq!(
            out,
            "\n# fixture\nanoN ← 0\nanoSel ← ⟨⟩\n\n# q1\nq1 ← (<\"Foo\")\n•Show q1\n\n# expectations\n\n\"ok\"\n"
        );
    }

    // --label replaces the plain display with the 0x1D tag plus •Show.
    #[test]
    fn label_query_emits_the_native_contract() {
        let mut it = Interner::new();
        let foo = it.intern("Foo");
        let q = Node::new(NodeKind::Query(Box::new(Node::new(NodeKind::Sym(foo), 1))), 1);
        let prog = Node::new(NodeKind::Program(vec![q]), 1);
        let reg = Registry::default();
        let mut dirs = Directives::default();
        dirs.label = true;
        let out = emit(&prog, &reg, &dirs, &it).expect("emit");
        assert_eq!(
            out,
            "\n# fixture\nanoN ← 0\nanoSel ← ⟨⟩\n\n# q1\nq1 ← (<\"Foo\")\n•Out (@+29)∾\"q1@1\"\n•Show q1\n\n# expectations\n\n\"ok\"\n"
        );
    }

    // An --! out sym expectation pins the query exactly, reading the ravel either as the word
    // list a sym result answers with or as the glyph run a char result is.
    #[test]
    fn out_pin_emits_the_native_contract() {
        let mut it = Interner::new();
        let foo = it.intern("Foo");
        let q = Node::new(NodeKind::Query(Box::new(Node::new(NodeKind::Sym(foo), 2))), 2);
        let prog = Node::new(NodeKind::Program(vec![q]), 2);
        let reg = Registry::default();
        let mut dirs = Directives::default();
        dirs.expects.push(Expect::Out { vals: vec!["Foo".to_string()] });
        let out = emit(&prog, &reg, &dirs, &it).expect("emit");
        assert_eq!(
            out,
            "\n# fixture\nanoN ← 0\nanoSel ← ⟨⟩\n\n# q1\nq1 ← (<\"Foo\")\n\"out q1\" ! ⟨\"Foo\"⟩ {(𝕨≡𝕩)∨(∾𝕨)≡𝕩} ⥊q1\n\n# expectations\n\n\"ok\"\n"
        );
    }
    #[test]
    fn guard_closure_checks_whole_temp_identifiers() {
        assert!(mentions_bqn_name("(t1∧gold)", "t1"));
        assert!(mentions_bqn_name("t0+t1", "t1"));
        assert!(!mentions_bqn_name("t10+t2", "t1"));
        assert!(!mentions_bqn_name("at1+t1_", "t1"));
    }
}

// A16/A17 at the emission boundary: which domain a staged dead-link mask crosses, and the
// per-crossing record that names it.  These need no BQN — they read the emitted text.
#[cfg(test)]
mod trace_domains {
    use super::*;

    // Four rows keyed by Id; mentor dangles on row 0, links live on 1, and links dead on 2/3.
    fn registry() -> Registry {
        let col = |name: &str, ty: ColType, uniq: bool, nums: Vec<f64>| RegEntry {
            name: name.to_string(),
            defval: 0.0,
            kind: RegEntryKind::Col { ty, uniq, nums, syms: Vec::new(), pres: None, rng: None },
        };
        Registry {
            n: 4,
            ents: vec![
                col("Id", ColType::Num, true, vec![1.0, 2.0, 3.0, 4.0]),
                col("Flag", ColType::Bool, false, vec![1.0, 0.0, 1.0, 0.0]),
                col("Gold", ColType::Num, false, vec![0.0, 0.0, 0.0, 0.0]),
                RegEntry {
                    name: "mentor".to_string(),
                    defval: 0.0,
                    kind: RegEntryKind::Rel {
                        targets: vec![-1.0, 2.0, 98.0, 99.0],
                        key_of: Some("Id".to_string()),
                    },
                },
            ],
            ..Registry::default()
        }
    }

    // Unkeyed twin: mentor holds row indices, one of them beyond the world.
    fn unkeyed_registry() -> Registry {
        let mut reg = registry();
        reg.ents.retain(|e| e.name != "Id" && e.name != "mentor");
        reg.ents.push(RegEntry {
            name: "mentor".to_string(),
            defval: 0.0,
            kind: RegEntryKind::Rel { targets: vec![-1.0, 1.0, 9.0, 0.0], key_of: None },
        });
        reg
    }

    fn traced(source: &str, reg: &Registry) -> String {
        let mut it = Interner::new();
        let toks = crate::lex::lex(source.as_bytes(), false, &mut it).expect("lex");
        let prog = crate::parse::parse(&toks, &mut it).expect("parse");
        let mut dirs = Directives::default();
        dirs.trace = true;
        emit(&prog, reg, &dirs, &it).expect("emit")
    }

    fn quiet(source: &str, reg: &Registry) -> String {
        let mut it = Interner::new();
        let toks = crate::lex::lex(source.as_bytes(), false, &mut it).expect("lex");
        let prog = crate::parse::parse(&toks, &mut it).expect("parse");
        emit(&prog, reg, &Directives::default(), &it).expect("emit")
    }

    // The mask expression each staged AnoTraceDead actually reports over, in staging order.
    fn dead_masks(bqn: &str) -> Vec<String> {
        let mut binds: Vec<(String, String)> = Vec::new();
        let mut out = Vec::new();
        for line in bqn.lines() {
            let text = line.trim_start();
            if let Some((name, mask)) = text.split_once(" ← ") {
                binds.push((name.to_string(), mask.to_string()));
            }
            if let Some(rest) = text.strip_prefix("AnoTraceDead ⟨") {
                let dm = rest
                    .split(", ")
                    .nth(1)
                    .and_then(|piece| piece.split('/').next())
                    .unwrap_or_default();
                if let Some((_, mask)) = binds.iter().find(|(name, _)| name == dm) {
                    out.push(mask.clone());
                }
            }
        }
        out
    }

    fn use_lines(bqn: &str) -> Vec<String> {
        bqn.lines()
            .filter_map(|l| l.find("TRACE-USE ").map(|i| l[i..].trim_end_matches('"').to_string()))
            .collect()
    }

    // A predicate crossing reports over X even beside a false sibling, and the report does not
    // move when the conjuncts swap.
    #[test]
    fn predicate_crossing_stays_unconjoined() {
        let reg = registry();
        let a = traced("mentor.Gold = 0 & Flag , Gold += 1\n", &reg);
        let b = traced("Flag & mentor.Gold = 0 , Gold += 1\n", &reg);
        for bqn in [&a, &b] {
            let masks = dead_masks(bqn);
            assert_eq!(masks.len(), 1, "{}", bqn);
            assert!(!masks[0].contains("s1m"), "{}", masks[0]);
        }
        assert_eq!(use_lines(&a), use_lines(&b));
        assert_eq!(use_lines(&a), vec!["TRACE-USE 0 s1:1 PREDICATE SOURCE mentor"]);
    }

    // An effect crossing conjoins the STATEMENT mask, never the guard-refined one.
    #[test]
    fn effect_crossing_scopes_to_the_statement_mask() {
        let reg = registry();
        let bqn = traced("Flag , Gold = mentor.Gold\n", &reg);
        let masks = dead_masks(&bqn);
        assert_eq!(masks.len(), 1, "{}", bqn);
        assert!(masks[0].starts_with("s1m∧"), "{}", masks[0]);
        assert_eq!(use_lines(&bqn), vec!["TRACE-USE 0 s1:1 EFFECT SELECTED mentor"]);
    }

    // One relation crossed in both phases records twice, with distinct ids and domains.
    #[test]
    fn both_phases_record_separately() {
        let reg = registry();
        let bqn = traced("mentor.Gold = 0 & Flag , Gold = mentor.Gold\n", &reg);
        assert_eq!(
            use_lines(&bqn),
            vec![
                "TRACE-USE 0 s1:1 PREDICATE SOURCE mentor",
                "TRACE-USE 1 s1:1 EFFECT SELECTED mentor",
            ]
        );
        // the suffix each staged line carries matches its record
        assert!(bqn.contains("\"USE 0 PREDICATE SOURCE\""), "{}", bqn);
        assert!(bqn.contains("\"USE 1 EFFECT SELECTED\""), "{}", bqn);
    }

    // Two textual crossings in one phase are two uses: a def re-expands per use site, and the
    // site names the statement while the line keeps pointing at the spelling.
    #[test]
    fn every_textual_crossing_is_its_own_use() {
        let reg = registry();
        let bqn = traced(
            "def linked = mentor.Gold = 0\nlinked , Gold += 1\nlinked , Gold += 2\n",
            &reg,
        );
        assert_eq!(
            use_lines(&bqn),
            vec![
                "TRACE-USE 0 s1:1 PREDICATE SOURCE mentor",
                "TRACE-USE 1 s2:1 PREDICATE SOURCE mentor",
            ]
        );
    }

    // A query is a Predicate crossing over X: the previous statement's mask must not leak in.
    #[test]
    fn a_query_after_an_effect_stays_whole_column() {
        let reg = registry();
        let bqn = traced("Flag , Gold = mentor.Gold\nmentor.Gold\n", &reg);
        let masks = dead_masks(&bqn);
        assert_eq!(masks.len(), 2, "{}", bqn);
        assert!(masks[0].starts_with("s1m∧"), "{}", masks[0]);
        assert!(!masks[1].contains("s1m"), "{}", masks[1]);
        assert_eq!(
            use_lines(&bqn),
            vec![
                "TRACE-USE 0 s1:1 EFFECT SELECTED mentor",
                "TRACE-USE 1 q2:2 PREDICATE SOURCE mentor",
            ]
        );
    }

    // An unkeyed hop bounds its gather and guards foundness against the world length.
    #[test]
    fn unkeyed_hop_bounds_its_gather() {
        let reg = unkeyed_registry();
        let bqn = traced("Flag , Gold = mentor.Gold\n", &reg);
        assert!(bqn.contains("((≠gold)|0⌈mentor)⊏gold"), "{}", bqn);
        assert!(bqn.contains("((¯1≠mentor)∧(0≤mentor)∧(mentor<anoN))"), "{}", bqn);
        assert_eq!(use_lines(&bqn), vec!["TRACE-USE 0 s1:1 EFFECT SELECTED mentor"]);
    }

    // The flag gates whole blocks: with it off no trace text is emitted at all.
    #[test]
    fn the_flag_gates_every_trace_line() {
        let reg = registry();
        let bqn = quiet("mentor.Gold = 0 & Flag , Gold = mentor.Gold\n", &reg);
        assert!(!bqn.contains("AnoTraceDead"), "{}", bqn);
        assert!(!bqn.contains("AnoTraceEmpty"), "{}", bqn);
        assert!(!bqn.contains("TRACE-USE"), "{}", bqn);
    }

    // Ids are minted in staging order, so repeated emission is byte-identical.
    #[test]
    fn repeat_emission_is_byte_identical() {
        let reg = registry();
        let source = "mentor.Gold = 0 & Flag , Gold = mentor.Gold\nmentor.Gold\n";
        assert_eq!(traced(source, &reg), traced(source, &reg));
    }
}

mod normalize {
    use super::emit_lowered;
    use crate::alias::{AliasSnapshot, AliasTarget, ResolvedAlias};
    use crate::reducer::{self, Carrier, Form, MachineKind, OpDesc};
    use crate::registry::{names_eq, reg_find, reg_role};
    use crate::trace::{TracePhase, TracePlan};
    use crate::{
        ArithOp, AssignOp, BindKind, CallableDescriptor, ColType, Determinism, Diag, Directives,
        EffectSet, Interner, Node, NodeKind, RegEntry, RegEntryKind, RegType, Registry,
        ServiceDirection, Symbol,
    };
    use std::collections::{BTreeMap, BTreeSet};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Context {
        Neutral,
        Value,
        Mask,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum SemanticCarrier {
        Mask,
        Number,
        Char,
        Sym,
        Other,
    }

    impl SemanticCarrier {
        // Inputs: none.  Output: the reducer carrier this reading resolves a head against, or
        // None for a reading A9 declares no Greater/Lesser instance over.
        fn reducer_carrier(self) -> Option<Carrier> {
            match self {
                SemanticCarrier::Mask => Some(Carrier::Mask),
                SemanticCarrier::Number => Some(Carrier::Number),
                SemanticCarrier::Char => Some(Carrier::Char),
                SemanticCarrier::Sym => None,
                SemanticCarrier::Other => None,
            }
        }

        fn name(self) -> &'static str {
            match self {
                SemanticCarrier::Mask => "mask",
                SemanticCarrier::Number => "number",
                SemanticCarrier::Char => "char",
                SemanticCarrier::Sym => "sym",
                SemanticCarrier::Other => "value",
            }
        }

        // A refusal names the two carriers in this fixed order, never in operand order, so one
        // mixture has one diagnostic however it was written.
        fn rank(self) -> u8 {
            match self {
                SemanticCarrier::Mask => 0,
                SemanticCarrier::Number => 1,
                SemanticCarrier::Char => 2,
                SemanticCarrier::Sym => 3,
                SemanticCarrier::Other => 4,
            }
        }
    }

    // Inputs: the consulted target and the synthetic name a materialized result took. Output: the
    // trace clause naming what answered the consultation; an absent target is the bare fallback.
    fn provenance_of(target: Option<&AliasTarget>, materialized: Option<&str>) -> String {
        target
            .map(|target| target.provenance(materialized))
            .unwrap_or_else(|| "bare fallback".to_string())
    }

    struct Normalizer {
        /// Registry enriched with synthetic reducer/mask entries for the frozen backend.
        reg: Registry,
        /// Immutable registry used to validate aliases. Synthetic entries must not stale snapshots.
        base_reg: Registry,
        it: Interner,
        aliases: AliasSnapshot,
        /// Alias consultations recorded here; relationship crossings are minted by the lowerer.
        trace: TracePlan,
        /// One materialized Greater/Lesser function per carrier: the char instances cannot share
        /// the numeric ones, because their step is a different function.
        greater: Option<Symbol>,
        lesser: Option<Symbol>,
        char_greater: Option<Symbol>,
        char_lesser: Option<Symbol>,
        code_char: Option<Symbol>,
        char_code: Option<Symbol>,
        mask_number: Option<Symbol>,
        extent_number: Option<Symbol>,
        synthetic: u32,
        /// Exact-byte program definitions and the carrier inferred from their normalized body.
        derived: BTreeMap<String, SemanticCarrier>,
        /// Every registry name minted during this emission.  Source may not spell one.
        generated: BTreeSet<String>,
    }

    impl Normalizer {
        fn new(reg: &Registry, it: &Interner, aliases: AliasSnapshot) -> Self {
            Self {
                reg: reg.clone(),
                base_reg: reg.clone(),
                it: it.clone(),
                aliases,
                trace: TracePlan::default(),
                greater: None,
                lesser: None,
                char_greater: None,
                char_lesser: None,
                code_char: None,
                char_code: None,
                mask_number: None,
                extent_number: None,
                synthetic: 0,
                derived: BTreeMap::new(),
                generated: BTreeSet::new(),
            }
        }

        fn spelling(&self, symbol: Symbol) -> &str {
            self.it.resolve(symbol)
        }

        fn entity_key(&self, row: usize) -> f64 {
            reg_role(&self.base_reg, "id")
                .or_else(|| reg_role(&self.base_reg, "keys"))
                .and_then(|index| match &self.base_reg.ents[index].kind {
                    RegEntryKind::Col { nums, .. } => nums.get(row).copied(),
                    _ => None,
                })
                .unwrap_or(row as f64)
        }

        fn intern(&mut self, spelling: &str) -> Symbol {
            self.it.intern(spelling)
        }

        // Inputs: a stem. Output: a registry name no existing entry answers to, recorded in
        // `generated`.  Taken names are skipped, so a registry-declared AnoDynMask7 stays legal.
        fn fresh_registry_name(&mut self, stem: &str) -> String {
            loop {
                let name = format!("{}{}", stem, self.synthetic);
                self.synthetic = self.synthetic.wrapping_add(1);
                if reg_find(&self.reg, &name).is_none() {
                    self.generated.insert(name.clone());
                    return name;
                }
            }
        }

        // Inputs: the checked descriptor of the direct Greater/Lesser instance and which side it
        // is. Output: the symbol of a materialized registry fn carrying that instance's dyad,
        // minted once per (side, carrier) and cached.  Both the dyad and the carrier come from
        // the descriptor, so the direct instance and the fold/scan instances cannot disagree.
        fn function(&mut self, desc: &OpDesc, greater: bool) -> Symbol {
            let chars = desc.reducer().map(|reducer| reducer.input) == Some(Carrier::Char);
            let cached = match (greater, chars) {
                (true, false) => self.greater,
                (false, false) => self.lesser,
                (true, true) => self.char_greater,
                (false, true) => self.char_lesser,
            };
            if let Some(symbol) = cached {
                return symbol;
            }
            let stem = match (greater, chars) {
                (true, false) => "AnoSemGreater",
                (false, false) => "AnoSemLesser",
                (true, true) => "AnoSemCharGreater",
                (false, true) => "AnoSemCharLesser",
            };
            let name = self.fresh_registry_name(stem);
            let body = desc.direct_body().unwrap_or_default();
            self.reg.ents.push(RegEntry {
                name: name.clone(),
                defval: 0.0,
                kind: RegEntryKind::Fn { body: Some(body) },
            });
            let symbol = self.intern(&name);
            match (greater, chars) {
                (true, false) => self.greater = Some(symbol),
                (false, false) => self.lesser = Some(symbol),
                (true, true) => self.char_greater = Some(symbol),
                (false, true) => self.char_lesser = Some(symbol),
            }
            symbol
        }

        // Runtime promotion is an operator elaboration, not a storage conversion. The helper is
        // materialized once and then behaves like every other checked registry function in BQN.
        fn conversion_symbol(&mut self, from: SemanticCarrier, to: SemanticCarrier) -> Symbol {
            let cached = match (from, to) {
                (SemanticCarrier::Mask, SemanticCarrier::Number) => self.mask_number,
                (SemanticCarrier::Char, SemanticCarrier::Number) => self.char_code,
                (SemanticCarrier::Mask | SemanticCarrier::Number, SemanticCarrier::Char) => {
                    self.code_char
                }
                _ => None,
            };
            if let Some(symbol) = cached {
                return symbol;
            }
            let (stem, body) = match (from, to) {
                (SemanticCarrier::Mask, SemanticCarrier::Number) => {
                    ("AnoSemMaskNumber", "{𝕩+0}")
                }
                (SemanticCarrier::Char, SemanticCarrier::Number) => {
                    ("AnoSemCharCode", "{𝕩-@}")
                }
                (SemanticCarrier::Mask | SemanticCarrier::Number, SemanticCarrier::Char) => {
                    ("AnoSemCodeChar", "{@+𝕩}")
                }
                _ => unreachable!("invalid carrier conversion"),
            };
            let name = self.fresh_registry_name(stem);
            self.reg.ents.push(RegEntry {
                name: name.clone(),
                defval: 0.0,
                kind: RegEntryKind::Fn {
                    body: Some(body.to_string()),
                },
            });
            let symbol = self.intern(&name);
            match (from, to) {
                (SemanticCarrier::Mask, SemanticCarrier::Number) => self.mask_number = Some(symbol),
                (SemanticCarrier::Char, SemanticCarrier::Number) => self.char_code = Some(symbol),
                (SemanticCarrier::Mask | SemanticCarrier::Number, SemanticCarrier::Char) => {
                    self.code_char = Some(symbol)
                }
                _ => unreachable!("invalid carrier conversion"),
            }
            symbol
        }

        // Average over a promoted mask is the mean of its bits, so the denominator is the
        // extent of the value stream, not the count of true bits. Pushing the helper through
        // scope and grouped-hop nodes preserves their domain and lineage.
        fn numeric_extent(&mut self, node: Node) -> Node {
            let line = node.line;
            match node.kind {
                NodeKind::Scope { l, r, origin } => Node::new(
                    NodeKind::Scope {
                        l: Box::new(self.numeric_extent(*l)),
                        r,
                        origin,
                    },
                    line,
                ),
                NodeKind::Hop { l, r } if matches!(l.kind, NodeKind::SetHop { .. }) => Node::new(
                    NodeKind::Hop {
                        l,
                        r: Box::new(self.numeric_extent(*r)),
                    },
                    line,
                ),
                kind => {
                    let callee = match self.extent_number {
                        Some(symbol) => symbol,
                        None => {
                            let name = self.fresh_registry_name("AnoSemExtent");
                            self.reg.ents.push(RegEntry {
                                name: name.clone(),
                                defval: 0.0,
                                kind: RegEntryKind::Fn {
                                    body: Some("{1¨𝕩}".to_string()),
                                },
                            });
                            let symbol = self.intern(&name);
                            self.extent_number = Some(symbol);
                            symbol
                        }
                    };
                    Node::new(
                        NodeKind::Call {
                            callee,
                            args: vec![Node::new(kind, line)],
                        },
                        line,
                    )
                }
            }
        }

        fn coerce(&mut self, node: Node, to: SemanticCarrier) -> Node {
            let from = self.infer(&node);
            if from == to {
                return node;
            }
            let line = node.line;
            match node.kind {
                NodeKind::Scope { l, r, origin } => Node::new(
                    NodeKind::Scope { l: Box::new(self.coerce(*l, to)), r, origin },
                    line,
                ),
                NodeKind::Hop { l, r } if matches!(l.kind, NodeKind::SetHop { .. }) => Node::new(
                    NodeKind::Hop { l, r: Box::new(self.coerce(*r, to)) },
                    line,
                ),
                kind => {
                    let node = Node::new(kind, line);
                    let callee = self.conversion_symbol(from, to);
                    Node::new(NodeKind::Call { callee, args: vec![node] }, line)
                }
            }
        }

        // Inputs: the stem of a `^name`, its line. Output: an Alias node whose `look` is the
        // resolved lookup name and whose `req` is the requested stem. THE one dynamic-alias
        // consultation point: the only caller of AliasSnapshot::resolve. The node kind stays
        // Alias on every outcome, so the resolved plan distinguishes `name` from `^name` without
        // re-reading source, and every refusal downstream can still spell `^req`.
        fn resolve_alias_or_bare(&mut self, symbol: Symbol, line: i32) -> Result<Node, Diag> {
            let spelling = self.spelling(symbol).to_string();
            let target = self.aliases.target(&spelling).cloned();
            let overlay = self.aliases.version;
            let (look, clause) = match self.aliases.resolve(&self.base_reg, &spelling)? {
                // explicit bare fallback (A4): no overlay entry; the lowerer performs the bare
                // lookup on the requested stem
                None => (symbol, "bare fallback".to_string()),
                Some(ResolvedAlias::Entry(index)) => {
                    let name = self.reg.ents[index].name.clone();
                    let clause = provenance_of(target.as_ref(), None);
                    (self.intern(&name), clause)
                }
                Some(ResolvedAlias::Mask(values)) => {
                    // A dynamic mask has no registry declaration, so it materializes as a
                    // RegEntryKind::AliasMask entry — the static alias-mask kind is its lowering
                    // vehicle only, never a claim that the overlay entry became static.
                    let name = self.fresh_registry_name("AnoDynMask");
                    self.reg.ents.push(RegEntry {
                        name: name.clone(),
                        defval: 0.0,
                        kind: RegEntryKind::AliasMask { mask: values },
                    });
                    let clause = provenance_of(target.as_ref(), Some(&name));
                    (self.intern(&name), clause)
                }
                Some(ResolvedAlias::EntityRow(row)) => {
                    // A resolver entity result likewise has no declaration: it materializes as a
                    // synthetic Bind{Entity} entry, the same vehicle a host binding lowers through.
                    let name = self.fresh_registry_name("AnoDynEnt");
                    self.reg.ents.push(RegEntry {
                        name: name.clone(),
                        defval: 0.0,
                        kind: RegEntryKind::Bind {
                            kind: BindKind::Entity,
                            vals: vec![self.entity_key(row)],
                        },
                    });
                    let clause = provenance_of(target.as_ref(), Some(&name));
                    (self.intern(&name), clause)
                }
            };
            self.trace
                .record_alias(format!("^{} -> {}, overlay v{}", spelling, clause, overlay));
            Ok(Node::new(NodeKind::Alias { look, req: symbol }, line))
        }

        fn carrier_of_entry(&self, index: usize) -> SemanticCarrier {
            match &self.reg.ents[index].kind {
                RegEntryKind::Col { ty: ColType::Bool, .. }
                | RegEntryKind::Field { ty: ColType::Bool, .. }
                | RegEntryKind::AliasMask { .. }
                | RegEntryKind::Tag { .. }
                | RegEntryKind::Bind { kind: BindKind::Mask, .. } => SemanticCarrier::Mask,
                RegEntryKind::Col { ty: ColType::Num, .. }
                | RegEntryKind::Col { ty: ColType::Nat, .. }
                | RegEntryKind::Col { ty: ColType::Int, .. }
                | RegEntryKind::Field { ty: ColType::Num, .. }
                | RegEntryKind::Field { ty: ColType::Nat, .. }
                | RegEntryKind::Field { ty: ColType::Int, .. }
                | RegEntryKind::Rel { .. }
                | RegEntryKind::Bind {
                    kind: BindKind::Entity,
                    ..
                }
                | RegEntryKind::Bind { kind: BindKind::Point, .. }
                | RegEntryKind::Bind { kind: BindKind::Num, .. }
                | RegEntryKind::Bind { kind: BindKind::Vec, .. } => SemanticCarrier::Number,
                RegEntryKind::Col { ty: ColType::Char, .. }
                | RegEntryKind::Field { ty: ColType::Char, .. } => SemanticCarrier::Char,
                RegEntryKind::Col { ty: ColType::Sym, .. }
                | RegEntryKind::Field { ty: ColType::Sym, .. } => SemanticCarrier::Sym,
                _ => SemanticCarrier::Other,
            }
        }

        fn infer(&self, node: &Node) -> SemanticCarrier {
            match &node.kind {
                NodeKind::Num(..)
                | NodeKind::Counter { .. }
                | NodeKind::Arith { .. }
                | NodeKind::IotaX(..)
                | NodeKind::CrossV { .. }
                | NodeKind::Tuple(..) => SemanticCarrier::Number,
                NodeKind::Sym(..) => SemanticCarrier::Sym,
                NodeKind::Name(symbol) | NodeKind::Alias { look: symbol, .. } => self
                    .derived
                    .get(self.spelling(*symbol))
                    .copied()
                    .or_else(|| {
                        reg_find(&self.reg, self.spelling(*symbol))
                            .map(|index| self.carrier_of_entry(index))
                    })
                    .unwrap_or(SemanticCarrier::Other),
                NodeKind::Cmp { .. }
                | NodeKind::CmpAny { .. }
                | NodeKind::Not(..) => SemanticCarrier::Mask,
                // a string literal is a run of glyphs, and reads on the char carrier
                NodeKind::Str(..) => SemanticCarrier::Char,
                NodeKind::And(left, right) | NodeKind::Or(left, right) => {
                    match (self.infer(left), self.infer(right)) {
                        (SemanticCarrier::Number, SemanticCarrier::Number) => {
                            SemanticCarrier::Number
                        }
                        (SemanticCarrier::Char, SemanticCarrier::Char) => SemanticCarrier::Char,
                        _ => SemanticCarrier::Mask,
                    }
                }
                NodeKind::Fold { op, operand } | NodeKind::ScanExpr { op, operand } => {
                    let spelling = self.spelling(*op);
                    if let Some(descriptor) = self.typed_callable(spelling) {
                        Self::semantic_from_type(&descriptor.signature.output)
                    } else if spelling == "#" {
                        SemanticCarrier::Number
                    } else if matches!(spelling, "charmax" | "charmin") {
                        SemanticCarrier::Char
                    } else if matches!(spelling, "&" | "|")
                        && self.infer(operand) == SemanticCarrier::Mask
                    {
                        SemanticCarrier::Mask
                    } else {
                        SemanticCarrier::Number
                    }
                }
                NodeKind::ScanAlong { op, col, .. } => {
                    let spelling = self.spelling(*op);
                    if let Some(descriptor) = self.typed_callable(spelling) {
                        Self::semantic_from_type(&descriptor.signature.output)
                    } else if matches!(spelling, "charmax" | "charmin") {
                        SemanticCarrier::Char
                    } else if matches!(spelling, "&" | "|")
                        && self.infer(col) == SemanticCarrier::Mask
                    {
                        SemanticCarrier::Mask
                    } else {
                        SemanticCarrier::Number
                    }
                }
                NodeKind::Hop { r, .. } => self.infer(r),
                NodeKind::Scope { l, .. } => self.infer(l),
                // the Greater/Lesser call this normalizer minted keeps its operands' carrier
                NodeKind::Call { callee, .. } => {
                    if Some(*callee) == self.char_greater
                        || Some(*callee) == self.char_lesser
                        || Some(*callee) == self.code_char
                    {
                        SemanticCarrier::Char
                    } else if Some(*callee) == self.char_code || Some(*callee) == self.mask_number {
                        SemanticCarrier::Number
                    } else if let Some(descriptor) = self.typed_callable(self.spelling(*callee)) {
                        Self::semantic_from_type(&descriptor.signature.output)
                    } else {
                        SemanticCarrier::Number
                    }
                }
                _ => SemanticCarrier::Other,
            }
        }

        // Inputs: the ORIGINAL source program (never the normalized one — normalizer-created
        // Greater/Lesser Call nodes and materialized Alias nodes legitimately spell generated
        // names). Output: Ok, or a refusal naming the first source symbol that collides with a
        // name minted this emission. Invariant: order-independent, and only names actually
        // generated here are reserved, so a registry-declared AnoDynMask7 stays usable.
        fn refuse_reserved(&self, prog: &Node) -> Result<(), Diag> {
            if self.generated.is_empty() {
                return Ok(());
            }
            self.walk_reserved(prog)
        }

        fn same_entry(&self, node: &Node, target: usize) -> bool {
            match &node.kind {
                NodeKind::Name(symbol) | NodeKind::Alias { look: symbol, .. } => {
                    reg_find(&self.reg, self.spelling(*symbol)) == Some(target)
                }
                _ => super::children(node)
                    .into_iter()
                    .any(|child| self.same_entry(child, target)),
            }
        }

        fn scan_reads(&self, node: &Node, target: usize) -> bool {
            match &node.kind {
                NodeKind::ScanExpr { operand, .. } => self.same_entry(operand, target),
                NodeKind::ScanAlong { col, order, .. } => {
                    self.same_entry(col, target) || self.same_entry(order, target)
                }
                _ => super::children(node)
                    .into_iter()
                    .any(|child| self.scan_reads(child, target)),
            }
        }

        // A simple selection declaration is a nominal domain witness at plan time.
        fn domain_entry(&self, node: &Node) -> Option<usize> {
            match &node.kind {
                NodeKind::Name(symbol) | NodeKind::Alias { look: symbol, .. } => {
                    reg_find(&self.reg, self.spelling(*symbol))
                }
                _ => None,
            }
        }

        fn ordered_scan_scope(&self, node: &Node) -> bool {
            matches!(
                node.kind,
                NodeKind::Shape(_) | NodeKind::Pipe { .. } | NodeKind::IotaX(_)
            ) || self.domain_entry(node).is_some_and(|entry| {
                matches!(
                    self.reg.ents[entry].kind,
                    RegEntryKind::Bind {
                        kind: BindKind::Vec,
                        ..
                    }
                )
            })
        }

        // Refuse mismatches whose lineage is decidable from syntax. Dynamic and ordered views
        // retain a runtime row witness, so this check never guesses at extensional equality.
        fn check_scan_domains(
            &self,
            node: &Node,
            selection: Option<&Node>,
            dynamic_selection: bool,
            target: &str,
        ) -> Result<(), Diag> {
            if let NodeKind::ScanExpr { operand, .. } = &node.kind {
                let scope = match &operand.kind {
                    NodeKind::Scope { r, .. } => Some(r.as_ref()),
                    _ => None,
                };
                if !dynamic_selection {
                    let mismatch = match (selection, scope) {
                        (None, None) => false,
                        (Some(_), None) | (None, Some(_)) => true,
                        (Some(_statement), Some(scan)) if self.ordered_scan_scope(scan) => false,
                        (Some(statement), Some(scan)) => {
                            if statement.kind == scan.kind {
                                false
                            } else {
                                matches!(
                                    (self.domain_entry(statement), self.domain_entry(scan)),
                                    (Some(left), Some(right)) if left != right
                                )
                            }
                        }
                    };
                    if mismatch {
                        return Err(super::fail(
                            node.line,
                            format!(
                                "scan domain does not match statement selection for '{}'",
                                target
                            ),
                        ));
                    }
                }
            }
            for child in super::children(node) {
                self.check_scan_domains(child, selection, dynamic_selection, target)?;
            }
            Ok(())
        }

        // A cross value owns a product domain, not the world's row domain. Pure expression
        // wrappers retain that lineage; no wrapper may turn an N×M value into a scatter column.
        fn has_product_lineage(&self, node: &Node) -> bool {
            matches!(node.kind, NodeKind::CrossV { .. })
                || super::children(node)
                    .into_iter()
                    .any(|child| self.has_product_lineage(child))
        }

        fn target_entry(&self, node: &Node) -> Option<usize> {
            match &node.kind {
                NodeKind::Name(symbol) | NodeKind::Alias { look: symbol, .. } => {
                    reg_find(&self.reg, self.spelling(*symbol))
                }
                NodeKind::Hop { l, .. } => self.target_entry(l),
                _ => None,
            }
        }

        fn walk_reserved(&self, node: &Node) -> Result<(), Diag> {
            let mut symbols: Vec<Symbol> = Vec::new();
            match &node.kind {
                NodeKind::Name(s)
                | NodeKind::Alias { look: s, .. }
                | NodeKind::Call { callee: s, .. }
                | NodeKind::CmpAny { name: s }
                | NodeKind::SetHop { rel: s }
                | NodeKind::EAdd(s)
                | NodeKind::EDel(s)
                | NodeKind::EVerb { name: s, .. }
                | NodeKind::EVia { f: s, .. }
                | NodeKind::Fold { op: s, .. }
                | NodeKind::ScanExpr { op: s, .. }
                | NodeKind::ScanAlong { op: s, .. }
                | NodeKind::Binder { name: s, .. }
                | NodeKind::DefStmt { name: s, .. }
                | NodeKind::UndefStmt { name: s } => symbols.push(*s),
                _ => {}
            }
            for symbol in symbols {
                let spelling = self.spelling(symbol);
                if self.generated.iter().any(|name| names_eq(name, spelling)) {
                    return Err(super::fail(
                        node.line,
                        format!("reserved synthetic name '{}'", spelling),
                    ));
                }
            }
            for kid in super::children(node) {
                self.walk_reserved(kid)?;
            }
            Ok(())
        }

        fn presence(node: Node) -> Node {
            let line = node.line;
            Node::new(
                NodeKind::Not(Box::new(Node::new(NodeKind::Not(Box::new(node)), line))),
                line,
            )
        }

        // The count machine consumes the selection-presence stream of the fold/scan DOMAIN, not
        // of the whole operand: wrapping inside the scope keeps emit_scan's scope split intact,
        // so `#\ x @ s` and `+\ x @ s` compress the same rows.
        fn scoped_presence(node: Node) -> Node {
            let line = node.line;
            match node.kind {
                NodeKind::Scope { l, r, origin } => Node::new(
                    NodeKind::Scope { l: Box::new(Self::presence(*l)), r, origin },
                    line,
                ),
                _ => Self::presence(node),
            }
        }

        // The same wrap for a fold, pushed onto the hopped component of a grouped operand so the
        // fiber shape survives it.  `#/ rel'.Comp` is the cardinality of the fiber's admitted
        // elements, never a sum of their payloads: count consumes presence, as q's `count` and
        // Haskell's `length` do.  Every other `#/` operand already reaches the emitter through
        // its mask reading, so only this one needs the wrap made explicit.
        fn fiber_presence(node: Node) -> Node {
            let line = node.line;
            match node.kind {
                NodeKind::Hop { l, r } if matches!(l.kind, NodeKind::SetHop { .. }) => {
                    Node::new(NodeKind::Hop { l, r: Box::new(Self::presence(*r)) }, line)
                }
                kind => Node::new(kind, line),
            }
        }

        fn type_of_entry(&self, index: usize) -> Option<RegType> {
            let entry = &self.reg.ents[index];
            match &entry.kind {
                RegEntryKind::Col { ty, nums, .. } => {
                    if *ty == ColType::Num
                        && self.reg.n > 0
                        && nums.len() as i64 == 2 * self.reg.n as i64
                    {
                        return None;
                    }
                    Some(match ty {
                        ColType::Bool => RegType::Mask,
                        ColType::Nat => RegType::Nat,
                        ColType::Int => RegType::Int,
                        ColType::Num => RegType::Num,
                        ColType::Sym => RegType::Sym,
                        ColType::Char => RegType::Char,
                    })
                }
                RegEntryKind::Field { ty, nums, .. } => {
                    let cells = self.reg.lat_w.wrapping_mul(self.reg.lat_h);
                    if *ty == ColType::Num
                        && cells > 0
                        && nums.len() as i64 == 2 * cells as i64
                    {
                        return None;
                    }
                    Some(match ty {
                        ColType::Bool => RegType::Mask,
                        ColType::Nat => RegType::Nat,
                        ColType::Int => RegType::Int,
                        ColType::Num => RegType::Num,
                        ColType::Sym => RegType::Sym,
                        ColType::Char => RegType::Char,
                    })
                }
                RegEntryKind::Rel { .. } => Some(RegType::Entity),
                RegEntryKind::AliasMask { .. } | RegEntryKind::Tag { .. } => {
                    Some(RegType::Mask)
                }
                RegEntryKind::Bind { kind, .. } => match kind {
                    BindKind::Entity => Some(RegType::Entity),
                    BindKind::Mask => Some(RegType::Mask),
                    BindKind::Num => Some(RegType::Num),
                    BindKind::Point | BindKind::Vec => None,
                },
                _ => None,
            }
        }

        fn type_from_semantic(carrier: SemanticCarrier) -> Option<RegType> {
            match carrier {
                SemanticCarrier::Mask => Some(RegType::Mask),
                SemanticCarrier::Number => Some(RegType::Num),
                SemanticCarrier::Char => Some(RegType::Char),
                SemanticCarrier::Sym => Some(RegType::Sym),
                SemanticCarrier::Other => None,
            }
        }
        fn semantic_from_type(carrier: &RegType) -> SemanticCarrier {
            match carrier {
                RegType::Mask => SemanticCarrier::Mask,
                RegType::Char => SemanticCarrier::Char,
                RegType::Sym => SemanticCarrier::Sym,
                RegType::Unit => SemanticCarrier::Other,
                RegType::Nat
                | RegType::Int
                | RegType::Num
                | RegType::Entity
                | RegType::Named(_) => SemanticCarrier::Number,
            }
        }

        // This exact carrier view is used only at typed registry boundaries. Runtime operator
        // promotion remains the broader SemanticCarrier lattice.
        fn node_type(&self, node: &Node) -> Option<RegType> {
            match &node.kind {
                NodeKind::Num(value) => {
                    if crate::registry::type_admits(ColType::Nat, *value) {
                        Some(RegType::Nat)
                    } else if crate::registry::type_admits(ColType::Int, *value) {
                        Some(RegType::Int)
                    } else {
                        Some(RegType::Num)
                    }
                }
                NodeKind::Counter { .. } | NodeKind::Arith { .. } => Some(RegType::Num),
                NodeKind::Sym(..) => Some(RegType::Sym),
                NodeKind::Str(..) => Some(RegType::Char),
                NodeKind::Name(symbol) | NodeKind::Alias { look: symbol, .. } => self
                    .derived
                    .get(self.spelling(*symbol))
                    .copied()
                    .and_then(Self::type_from_semantic)
                    .or_else(|| {
                        reg_find(&self.reg, self.spelling(*symbol))
                            .and_then(|index| self.type_of_entry(index))
                    }),
                NodeKind::Cmp { .. }
                | NodeKind::CmpAny { .. }
                | NodeKind::Not(..) => Some(RegType::Mask),
                NodeKind::And(..) | NodeKind::Or(..) => {
                    Self::type_from_semantic(self.infer(node))
                }
                NodeKind::Scope { l, .. } => self.node_type(l),
                NodeKind::Hop { r, .. } => self.node_type(r),
                NodeKind::Call { callee, .. } => reg_find(&self.reg, self.spelling(*callee))
                    .and_then(|index| match &self.reg.ents[index].kind {
                        RegEntryKind::TypedFn { descriptor, .. } => {
                            Some(descriptor.signature.output.clone())
                        }
                        RegEntryKind::Ctor { .. } => {
                            Some(RegType::Named(self.reg.ents[index].name.clone()))
                        }
                        _ => None,
                    })
                    .or(Some(RegType::Num)),
                NodeKind::Fold { op, operand } | NodeKind::ScanExpr { op, operand } => {
                    if let Some(output) = reg_find(&self.reg, self.spelling(*op)).and_then(
                        |index| match &self.reg.ents[index].kind {
                            RegEntryKind::TypedFn { descriptor, .. } => {
                                Some(descriptor.signature.output.clone())
                            }
                            _ => None,
                        },
                    ) {
                        Some(output)
                    } else if self.spelling(*op) == "#" {
                        Some(RegType::Nat)
                    } else {
                        Self::type_from_semantic(self.infer(operand))
                    }
                }
                NodeKind::ScanAlong { op, col, .. } => {
                    if let Some(output) = reg_find(&self.reg, self.spelling(*op)).and_then(
                        |index| match &self.reg.ents[index].kind {
                            RegEntryKind::TypedFn { descriptor, .. } => {
                                Some(descriptor.signature.output.clone())
                            }
                            _ => None,
                        },
                    ) {
                        Some(output)
                    } else if self.spelling(*op) == "#" {
                        Some(RegType::Nat)
                    } else {
                        Self::type_from_semantic(self.infer(col))
                    }
                }
                NodeKind::IotaX(..) => Some(RegType::Nat),
                NodeKind::CrossV { f, .. } => reg_find(&self.reg, self.spelling(*f)).and_then(
                    |index| match &self.reg.ents[index].kind {
                        RegEntryKind::TypedFn { descriptor, .. } => {
                            Some(descriptor.signature.output.clone())
                        }
                        _ => Some(RegType::Num),
                    },
                ),
                _ => None,
            }
        }

        fn typed_callable(&self, spelling: &str) -> Option<CallableDescriptor> {
            reg_find(&self.reg, spelling).and_then(|index| {
                if let RegEntryKind::TypedFn { descriptor, .. } = &self.reg.ents[index].kind {
                    Some(descriptor.clone())
                } else {
                    None
                }
            })
        }

        fn validate_typed_arguments(
            &self,
            spelling: &str,
            descriptor: &CallableDescriptor,
            args: &[Node],
            line: i32,
        ) -> Result<(), Diag> {
            if args.len() != descriptor.signature.inputs.len() {
                return Err(super::fail(
                    line,
                    format!(
                        "typed callable '{}' expects {} arguments, got {}",
                        spelling,
                        descriptor.signature.inputs.len(),
                        args.len()
                    ),
                ));
            }
            for (position, (arg, expected)) in args
                .iter()
                .zip(&descriptor.signature.inputs)
                .enumerate()
            {
                let actual = self.node_type(arg);
                if actual.as_ref() != Some(expected) {
                    return Err(super::fail(
                        line,
                        format!(
                            "typed callable '{}' argument {} is {}, expected {}",
                            spelling,
                            position + 1,
                            actual
                                .as_ref()
                                .map(crate::registry::reg_type_word)
                                .unwrap_or("untyped"),
                            crate::registry::reg_type_word(expected)
                        ),
                    ));
                }
            }
            Ok(())
        }

        fn has_output_service(&self, descriptor: &CallableDescriptor) -> bool {
            descriptor.services.iter().any(|service| {
                reg_find(&self.reg, service).is_some_and(|index| {
                    matches!(
                        &self.reg.ents[index].kind,
                        RegEntryKind::Service { descriptor }
                            if descriptor.direction == ServiceDirection::Output
                    )
                })
            })
        }

        fn validate_value_call(
            &self,
            spelling: &str,
            args: &[Node],
            line: i32,
        ) -> Result<(), Diag> {
            if spelling == "rank"
                && reg_find(&self.reg, spelling)
                    .is_none_or(|index| !super::claims_call_namespace(&self.reg.ents[index].kind))
            {
                if args.len() != 1 {
                    return Err(super::fail(
                        line,
                        format!("rank expects 1 argument, got {}", args.len()),
                    ));
                }
                return Ok(());
            }
            let Some(index) = reg_find(&self.reg, spelling) else {
                return Ok(());
            };
            match &self.reg.ents[index].kind {
                RegEntryKind::Fn { .. } => Ok(()),
                RegEntryKind::TypedFn { descriptor, .. } => {
                    self.validate_typed_arguments(spelling, descriptor, args, line)?;
                    if !(1..=2).contains(&args.len()) {
                        return Err(super::fail(
                            line,
                            format!(
                                "typed callable '{}' has no {}-argument value ABI; use a unary or binary signature",
                                spelling,
                                args.len()
                            ),
                        ));
                    }
                    if descriptor.signature.output == RegType::Unit {
                        return Err(super::fail(
                            line,
                            format!("typed callable '{}' returns unit and is effect-only", spelling),
                        ));
                    }
                    if descriptor.effects.write
                        || descriptor.determinism == Determinism::Nondeterministic
                        || self.has_output_service(descriptor)
                    {
                        return Err(super::fail(
                            line,
                            format!("typed callable '{}' has effects unavailable in value position", spelling),
                        ));
                    }
                    Ok(())
                }
                RegEntryKind::Ctor { .. } => Err(super::fail(
                    line,
                    format!(
                        "constructor '{}' is a checked host boundary, not raw BQN",
                        spelling
                    ),
                )),
                _ => Err(super::fail(
                    line,
                    format!("declaration '{}' is not callable", spelling),
                )),
            }
        }
        fn validate_cross_call(&self, spelling: &str, line: i32) -> Result<(), Diag> {
            let Some(index) = reg_find(&self.reg, spelling) else {
                return Err(super::fail(line, "cross needs a registered fn"));
            };
            match &self.reg.ents[index].kind {
                RegEntryKind::Fn { .. } => Ok(()),
                RegEntryKind::TypedFn { descriptor, .. } => {
                    if descriptor.signature.inputs.as_slice()
                        != [RegType::Mask, RegType::Mask]
                    {
                        return Err(super::fail(
                            line,
                            format!(
                                "typed cross '{}' requires signature mask,mask->...",
                                spelling
                            ),
                        ));
                    }
                    if descriptor.signature.output == RegType::Unit
                        || descriptor.effects.write
                        || descriptor.determinism == Determinism::Nondeterministic
                        || self.has_output_service(descriptor)
                    {
                        return Err(super::fail(
                            line,
                            format!(
                                "typed cross '{}' requires a value-returning input-only callable",
                                spelling
                            ),
                        ));
                    }
                    Ok(())
                }
                _ => Err(super::fail(line, "cross needs a registered fn")),
            }
        }

        fn validate_effect_call(
            &self,
            spelling: &str,
            args: &[Node],
            line: i32,
        ) -> Result<(), Diag> {
            let Some(index) = reg_find(&self.reg, spelling) else {
                return Err(super::fail(
                    line,
                    format!("verb '{}' needs a registered fn", spelling),
                ));
            };
            match &self.reg.ents[index].kind {
                RegEntryKind::Fn { .. } => Ok(()),
                RegEntryKind::TypedFn { descriptor, .. } => {
                    self.validate_typed_arguments(spelling, descriptor, args, line)?;
                    if descriptor.signature.output != RegType::Unit {
                        return Err(super::fail(
                            line,
                            format!("typed effect '{}' must return unit", spelling),
                        ));
                    }
                    if descriptor.writes.len() > 1 {
                        return Err(super::fail(
                            line,
                            format!(
                                "typed effect '{}' has {} write targets; this backend admits at most one",
                                spelling,
                                descriptor.writes.len()
                            ),
                        ));
                    }
                    if let Some(target) = descriptor.writes.first() {
                        let Some(target_index) = reg_find(&self.reg, target) else {
                            return Err(super::fail(
                                line,
                                format!("typed effect '{}' write target '{}' is missing", spelling, target),
                            ));
                        };
                        if !matches!(
                            self.reg.ents[target_index].kind,
                            RegEntryKind::Col { uniq: false, .. } | RegEntryKind::Field { .. }
                        ) {
                            return Err(super::fail(
                                line,
                                format!(
                                    "typed effect '{}' write target '{}' is not a mutable column",
                                    spelling, target
                                ),
                            ));
                        }
                    } else if !self.has_output_service(descriptor) {
                        return Err(super::fail(
                            line,
                            format!(
                                "typed effect '{}' has neither a write target nor an output service",
                                spelling
                            ),
                        ));
                    }
                    Ok(())
                }
                _ => Err(super::fail(
                    line,
                    format!("verb '{}' needs a registered fn", spelling),
                )),
            }
        }

        // The registry signature, not source arity, decides whether a function is a reducer.
        fn registered_reducer(
            &self,
            spelling: &str,
            line: i32,
            operand: &Node,
        ) -> Result<Option<Carrier>, Diag> {
            let Some(index) = reg_find(&self.reg, spelling) else {
                return Ok(None);
            };
            match &self.reg.ents[index].kind {
                RegEntryKind::Fn { .. } => Ok(Some(Carrier::Number)),
                RegEntryKind::TypedFn { descriptor, .. } => {
                    let signature = &descriptor.signature;
                    if descriptor.effects != EffectSet::default()
                        || descriptor.determinism != Determinism::Deterministic
                        || signature.inputs.len() != 2
                        || signature.inputs[0] != signature.inputs[1]
                        || signature.output != signature.inputs[0]
                    {
                        return Err(super::fail(
                            line,
                            format!(
                                "typed reducer '{}' must be pure, deterministic, and A,A->A",
                                spelling
                            ),
                        ));
                    }
                    let actual = self.node_type(operand);
                    if actual.as_ref() != Some(&signature.inputs[0]) {
                        return Err(super::fail(
                            line,
                            format!(
                                "typed reducer '{}' operand is {}, expected {}",
                                spelling,
                                actual
                                    .as_ref()
                                    .map(crate::registry::reg_type_word)
                                    .unwrap_or("untyped"),
                                crate::registry::reg_type_word(&signature.inputs[0])
                            ),
                        ));
                    }
                    let carrier = match signature.inputs[0] {
                        RegType::Mask => Carrier::Mask,
                        RegType::Nat | RegType::Int | RegType::Num => Carrier::Number,
                        RegType::Char => Carrier::Char,
                        _ => {
                            return Err(super::fail(
                                line,
                                format!(
                                    "typed reducer '{}' has unsupported carrier {}",
                                    spelling,
                                    crate::registry::reg_type_word(&signature.inputs[0])
                                ),
                            ));
                        }
                    };
                    Ok(Some(carrier))
                }
                _ => Ok(None),
            }
        }

        // Inputs: the head spelling and its already-normalized operand. Output: the carrier the
        // head is resolved against. Count consumes presence whatever the operand's payload is.
        fn head_carrier(&self, spelling: &str, operand: &Node) -> Carrier {
            if spelling == "#" {
                return Carrier::Presence;
            }
            self.infer(operand).reducer_carrier().unwrap_or(Carrier::Number)
        }

        fn coerce_numeric_head(&mut self, spelling: &str, operand: Node) -> Node {
            if !matches!(spelling, "+" | "-" | "*" | "/" | "avg") {
                return operand;
            }
            match self.infer(&operand) {
                SemanticCarrier::Mask | SemanticCarrier::Char => {
                    self.coerce(operand, SemanticCarrier::Number)
                }
                _ => operand,
            }
        }

        // THE fold/scan head resolution point. Inputs: spelling, form, operand, line. Output: the
        // checked descriptor whose canonical spelling the emitters retrieve by.
        fn resolve_head(
            &self,
            spelling: &str,
            form: Form,
            operand: &Node,
            line: i32,
        ) -> Result<OpDesc, Diag> {
            let registered = self.registered_reducer(spelling, line, operand)?;
            reducer::resolve_registered_head(
                spelling,
                form,
                self.head_carrier(spelling, operand),
                registered,
            )
            .map_err(|message| super::fail(line, message))
        }

        fn normalize(
            &mut self,
            node: &Node,
            context: Context,
            phase: TracePhase,
        ) -> Result<Node, Diag> {
            let line = node.line;
            let kind = match &node.kind {
                NodeKind::Num(value) => NodeKind::Num(*value),
                NodeKind::Counter { val, unit } => NodeKind::Counter { val: *val, unit: *unit },
                NodeKind::Sym(symbol) => NodeKind::Sym(*symbol),
                NodeKind::Str(symbol) => NodeKind::Str(*symbol),
                // bare lookup never touches the dynamic-alias overlay
                NodeKind::Name(symbol) => NodeKind::Name(*symbol),
                NodeKind::Alias { look: symbol, .. } => return self.resolve_alias_or_bare(*symbol, line),
                NodeKind::Wild => NodeKind::Wild,
                NodeKind::Not(inner) => {
                    NodeKind::Not(Box::new(self.normalize(inner, Context::Mask, phase)?))
                }
                NodeKind::And(left, right) | NodeKind::Or(left, right) => {
                    if context == Context::Mask {
                        let left = Box::new(self.normalize(left, Context::Mask, phase)?);
                        let right = Box::new(self.normalize(right, Context::Mask, phase)?);
                        if matches!(&node.kind, NodeKind::And(..)) {
                            NodeKind::And(left, right)
                        } else {
                            NodeKind::Or(left, right)
                        }
                    } else {
                        let mut left = self.normalize(left, Context::Neutral, phase)?;
                        let mut right = self.normalize(right, Context::Neutral, phase)?;
                        let greater = matches!(&node.kind, NodeKind::Or(..));
                        let spelling = if greater { "|" } else { "&" };
                        let (lc, rc) = (self.infer(&left), self.infer(&right));
                        let value_reading = context == Context::Value
                            || lc == SemanticCarrier::Char
                            || rc == SemanticCarrier::Char
                            || (lc == SemanticCarrier::Number && rc == SemanticCarrier::Number);
                        if value_reading {
                            let admitted = |carrier| {
                                matches!(
                                    carrier,
                                    SemanticCarrier::Mask
                                        | SemanticCarrier::Number
                                        | SemanticCarrier::Char
                                )
                            };
                            if !admitted(lc) || !admitted(rc) {
                                let (first, second) =
                                    if lc.rank() <= rc.rank() { (lc, rc) } else { (rc, lc) };
                                return Err(super::fail(
                                    line,
                                    format!(
                                        "'{}' has no promotion from {} and {}",
                                        spelling,
                                        first.name(),
                                        second.name()
                                    ),
                                ));
                            }
                            let result = if lc == SemanticCarrier::Char
                                || rc == SemanticCarrier::Char
                            {
                                SemanticCarrier::Char
                            } else if lc == SemanticCarrier::Number
                                || rc == SemanticCarrier::Number
                            {
                                SemanticCarrier::Number
                            } else {
                                SemanticCarrier::Mask
                            };
                            if result == SemanticCarrier::Mask {
                                if matches!(&node.kind, NodeKind::And(..)) {
                                    NodeKind::And(Box::new(left), Box::new(right))
                                } else {
                                    NodeKind::Or(Box::new(left), Box::new(right))
                                }
                            } else {
                                if lc != result {
                                    left = self.coerce(left, result);
                                }
                                if rc != result {
                                    right = self.coerce(right, result);
                                }
                                let desc =
                                    self.resolve_head(spelling, Form::Direct, &left, line)?;
                                let callee = self.function(&desc, greater);
                                NodeKind::Call { callee, args: vec![left, right] }
                            }
                        } else if matches!(&node.kind, NodeKind::And(..)) {
                            NodeKind::And(Box::new(left), Box::new(right))
                        } else {
                            NodeKind::Or(Box::new(left), Box::new(right))
                        }
                    }
                }
                NodeKind::Cmp { op, l, r } => {
                    let mut l = self.normalize(l, Context::Value, phase)?;
                    let mut r = self.normalize(r, Context::Value, phase)?;
                    let (lc, rc) = (self.infer(&l), self.infer(&r));
                    if lc == SemanticCarrier::Sym || rc == SemanticCarrier::Sym {
                        if lc != SemanticCarrier::Sym
                            || rc != SemanticCarrier::Sym
                            || !matches!(op, crate::CmpOp::Eq | crate::CmpOp::Ne)
                        {
                            let (first, second) =
                                if lc.rank() <= rc.rank() { (lc, rc) } else { (rc, lc) };
                            return Err(super::fail(
                                line,
                                format!(
                                    "comparison has no promotion from {} and {}",
                                    first.name(),
                                    second.name()
                                ),
                            ));
                        }
                    } else if lc == SemanticCarrier::Char || rc == SemanticCarrier::Char {
                        if self.infer(&l) == SemanticCarrier::Char {
                            l = self.coerce(l, SemanticCarrier::Number);
                        }
                        if self.infer(&r) == SemanticCarrier::Char {
                            r = self.coerce(r, SemanticCarrier::Number);
                        }
                    }
                    NodeKind::Cmp { op: *op, l: Box::new(l), r: Box::new(r) }
                }
                NodeKind::CmpAny { name } => NodeKind::CmpAny { name: *name },
                NodeKind::Arith { op, l, r } => {
                    let mut l = self.normalize(l, Context::Value, phase)?;
                    let mut r = self.normalize(r, Context::Value, phase)?;
                    for operand in [&mut l, &mut r] {
                        match self.infer(operand) {
                            SemanticCarrier::Number => {}
                            SemanticCarrier::Mask | SemanticCarrier::Char => {
                                *operand = self.coerce(operand.clone(), SemanticCarrier::Number)
                            }
                            carrier => {
                                return Err(super::fail(
                                    line,
                                    format!("arithmetic is not defined on {}", carrier.name()),
                                ));
                            }
                        }
                    }
                    NodeKind::Arith { op: *op, l: Box::new(l), r: Box::new(r) }
                }
                NodeKind::Scope { l, r, origin } => NodeKind::Scope {
                    l: Box::new(self.normalize(l, context, phase)?),
                    r: Box::new(self.normalize(r, Context::Neutral, phase)?),
                    origin: match origin {
                        Some(origin) => Some(Box::new(self.normalize(origin, Context::Neutral, phase)?)),
                        None => None,
                    },
                },
                NodeKind::Hop { l, r } => NodeKind::Hop {
                    l: Box::new(self.normalize(l, Context::Value, phase)?),
                    r: Box::new(self.normalize(r, Context::Neutral, phase)?),
                },
                NodeKind::SetHop { rel } => NodeKind::SetHop { rel: *rel },
                NodeKind::Call { callee, args } => {
                    let spelling = self.spelling(*callee).to_string();
                    let args = args
                        .iter()
                        .map(|arg| self.normalize(arg, Context::Value, phase))
                        .collect::<Result<Vec<_>, _>>()?;
                    self.validate_value_call(&spelling, &args, line)?;
                    NodeKind::Call { callee: *callee, args }
                }
                // The three head arms below share ONE resolver and ONE descriptor table. The
                // descriptor's canonical spelling is bound back into the node, so every emitter
                // retrieves the object that was checked here and can hold no second table.
                NodeKind::Fold { op, operand } => {
                    let spelling = self.spelling(*op).to_string();
                    let operand_context = if spelling == "#" { Context::Mask } else { Context::Value };
                    let operand = self.normalize(operand, operand_context, phase)?;
                    let operand = self.coerce_numeric_head(&spelling, operand);
                    let desc = self.resolve_head(&spelling, Form::Fold, &operand, line)?;
                    let operand = match desc.machine().map(|machine| machine.kind) {
                        Some(MachineKind::Count) => Self::fiber_presence(operand),
                        _ => operand,
                    };
                    let op = self.intern(desc.spelling());
                    NodeKind::Fold { op, operand: Box::new(operand) }
                }
                NodeKind::ScanExpr { op, operand } => {
                    let spelling = self.spelling(*op).to_string();
                    let operand_context = if spelling == "#" { Context::Mask } else { Context::Value };
                    let operand = self.normalize(operand, operand_context, phase)?;
                    let promoted_value = matches!(
                        self.infer(&operand),
                        SemanticCarrier::Mask | SemanticCarrier::Char
                    );
                    let operand = self.coerce_numeric_head(&spelling, operand);
                    let desc = self.resolve_head(&spelling, Form::Scan, &operand, line)?;
                    let plus = self.intern("+");
                    match desc.machine().map(|machine| machine.kind) {
                        // running cardinality over the checked presence stream
                        Some(MachineKind::Count) => NodeKind::ScanExpr {
                            op: plus,
                            operand: Box::new(Self::scoped_presence(operand)),
                        },
                        // state (sum,count) projected per prefix, never a homogeneous scanl1
                        Some(MachineKind::Average) => {
                            let numerator = Node::new(
                                NodeKind::ScanExpr {
                                    op: plus,
                                    operand: Box::new(operand.clone()),
                                },
                                line,
                            );
                            let denominator = Node::new(
                                NodeKind::ScanExpr {
                                    op: plus,
                                    operand: Box::new(if promoted_value {
                                        self.numeric_extent(operand)
                                    } else {
                                        Self::scoped_presence(operand)
                                    }),
                                },
                                line,
                            );
                            NodeKind::Arith {
                                op: ArithOp::Div,
                                l: Box::new(numerator),
                                r: Box::new(denominator),
                            }
                        }
                        None => NodeKind::ScanExpr {
                            op: self.intern(desc.spelling()),
                            operand: Box::new(operand),
                        },
                    }
                }
                NodeKind::ScanAlong { op, col, order } => {
                    let spelling = self.spelling(*op).to_string();
                    let col_context = if spelling == "#" { Context::Mask } else { Context::Value };
                    let col = self.normalize(col, col_context, phase)?;
                    let promoted_value = matches!(
                        self.infer(&col),
                        SemanticCarrier::Mask | SemanticCarrier::Char
                    );
                    let col = self.coerce_numeric_head(&spelling, col);
                    let order = self.normalize(order, Context::Value, phase)?;
                    let desc = self.resolve_head(&spelling, Form::ScanAlong, &col, line)?;
                    let plus = self.intern("+");
                    match desc.machine().map(|machine| machine.kind) {
                        Some(MachineKind::Count) => NodeKind::ScanAlong {
                            op: plus,
                            col: Box::new(Self::scoped_presence(col)),
                            order: Box::new(order),
                        },
                        Some(MachineKind::Average) => {
                            let numerator = Node::new(
                                NodeKind::ScanAlong {
                                    op: plus,
                                    col: Box::new(col.clone()),
                                    order: Box::new(order.clone()),
                                },
                                line,
                            );
                            let denominator = Node::new(
                                NodeKind::ScanAlong {
                                    op: plus,
                                    col: Box::new(if promoted_value {
                                        self.numeric_extent(col)
                                    } else {
                                        Self::scoped_presence(col)
                                    }),
                                    order: Box::new(order),
                                },
                                line,
                            );
                            NodeKind::Arith {
                                op: ArithOp::Div,
                                l: Box::new(numerator),
                                r: Box::new(denominator),
                            }
                        }
                        None => NodeKind::ScanAlong {
                            op: self.intern(desc.spelling()),
                            col: Box::new(col),
                            order: Box::new(order),
                        },
                    }
                }
                NodeKind::IotaX(inner) => {
                    NodeKind::IotaX(Box::new(self.normalize(inner, Context::Value, phase)?))
                }
                NodeKind::Shape(items) => NodeKind::Shape(
                    items
                        .iter()
                        .map(|item| self.normalize(item, Context::Value, phase))
                        .collect::<Result<Vec<_>, _>>()?,
                ),
                NodeKind::Tuple(items) => NodeKind::Tuple(
                    items
                        .iter()
                        .map(|item| self.normalize(item, Context::Value, phase))
                        .collect::<Result<Vec<_>, _>>()?,
                ),
                NodeKind::To { shape, poured } => NodeKind::To {
                    shape: Box::new(self.normalize(shape, Context::Value, phase)?),
                    poured: match poured {
                        Some(poured) => Some(Box::new(self.normalize(poured, Context::Value, phase)?)),
                        None => None,
                    },
                },
                NodeKind::Grade { key, desc } => NodeKind::Grade {
                    key: Box::new(self.normalize(key, Context::Value, phase)?),
                    desc: *desc,
                },
                NodeKind::Top { k, inner } => NodeKind::Top {
                    k: *k,
                    inner: Box::new(self.normalize(inner, Context::Neutral, phase)?),
                },
                NodeKind::Pipe { src, stages } => NodeKind::Pipe {
                    src: Box::new(self.normalize(src, Context::Neutral, phase)?),
                    stages: stages
                        .iter()
                        .map(|stage| self.normalize(stage, Context::Neutral, phase))
                        .collect::<Result<Vec<_>, _>>()?,
                },
                NodeKind::OrderBy { key, desc } => NodeKind::OrderBy {
                    key: Box::new(self.normalize(key, Context::Value, phase)?),
                    desc: *desc,
                },
                NodeKind::Take { k } => NodeKind::Take { k: *k },
                NodeKind::Expand(inner) => {
                    NodeKind::Expand(Box::new(self.normalize(inner, Context::Value, phase)?))
                }
                NodeKind::CrossV { f, a, b } => {
                    let spelling = self.spelling(*f).to_string();
                    let a = self.normalize(a, Context::Mask, phase)?;
                    let b = self.normalize(b, Context::Mask, phase)?;
                    self.validate_cross_call(&spelling, line)?;
                    NodeKind::CrossV {
                        f: *f,
                        a: Box::new(a),
                        b: Box::new(b),
                    }
                }
                NodeKind::Binder { name, source } => NodeKind::Binder {
                    name: *name,
                    source: Box::new(self.normalize(source, Context::Value, phase)?),
                },
                NodeKind::EAssign { op, target, rhs } => {
                    let target =
                        self.normalize(target, Context::Neutral, TracePhase::Effect)?;
                    let rhs = self.normalize(rhs, Context::Value, TracePhase::Effect)?;
                    if let Some(target_entry) = self.target_entry(&target) {
                        if self.has_product_lineage(&rhs) {
                            return Err(super::fail(
                                line,
                                format!(
                                    "cannot scatter product value to entity column '{}'",
                                    self.reg.ents[target_entry].name
                                ),
                            ));
                        }
                        let destination = self.carrier_of_entry(target_entry);
                        let result = self.infer(&rhs);
                        let compatible = match (destination, result, op) {
                            (SemanticCarrier::Number, SemanticCarrier::Number, _) => true,
                            (
                                SemanticCarrier::Mask,
                                SemanticCarrier::Mask | SemanticCarrier::Number,
                                _,
                            ) => true,
                            (
                                SemanticCarrier::Char,
                                SemanticCarrier::Char,
                                AssignOp::Set,
                            ) => true,
                            (
                                SemanticCarrier::Sym,
                                SemanticCarrier::Sym,
                                AssignOp::Set,
                            ) => true,
                            (SemanticCarrier::Other, _, _) => true,
                            _ => false,
                        };
                        if !compatible {
                            return Err(super::fail(
                                line,
                                format!(
                                    "cannot write {} result to {} '{}'",
                                    result.name(),
                                    destination.name(),
                                    self.reg.ents[target_entry].name
                                ),
                            ));
                        }
                        if self.scan_reads(&rhs, target_entry) {
                            return Err(super::fail(
                                line,
                                format!(
                                    "scan reads and writes '{}'; recurrence footprints must be disjoint",
                                    self.reg.ents[target_entry].name
                                ),
                            ));
                        }
                    }
                    NodeKind::EAssign {
                        op: *op,
                        target: Box::new(target),
                        rhs: Box::new(rhs),
                    }
                }
                NodeKind::EAdd(symbol) => NodeKind::EAdd(*symbol),
                NodeKind::EDel(symbol) => NodeKind::EDel(*symbol),
                NodeKind::EDespawn => NodeKind::EDespawn,
                NodeKind::ESpawn { what, count, at } => NodeKind::ESpawn {
                    what: Box::new(self.normalize(what, Context::Value, TracePhase::Effect)?),
                    count: match count {
                        Some(count) => Some(Box::new(self.normalize(
                            count,
                            Context::Value,
                            TracePhase::Effect,
                        )?)),
                        None => None,
                    },
                    at: match at {
                        Some(at) => Some(Box::new(self.normalize(
                            at,
                            Context::Value,
                            TracePhase::Effect,
                        )?)),
                        None => None,
                    },
                },
                NodeKind::EVerb { name, args } => {
                    let spelling = self.spelling(*name).to_string();
                    let args = args
                        .iter()
                        .map(|arg| self.normalize(arg, Context::Value, TracePhase::Effect))
                        .collect::<Result<Vec<_>, _>>()?;
                    self.validate_effect_call(&spelling, &args, line)?;
                    NodeKind::EVerb { name: *name, args }
                }
                NodeKind::EVia { f, col } => {
                    let spelling = self.spelling(*f).to_string();
                    let col = self.normalize(col, Context::Value, TracePhase::Effect)?;
                    self.validate_effect_call(&spelling, std::slice::from_ref(&col), line)?;
                    NodeKind::EVia { f: *f, col: Box::new(col) }
                }
                NodeKind::Stmt { sel, effects, rule, cont, elided } => {
                    let dynamic_selection = sel.is_none() && (*cont || *elided);
                    for effect in effects {
                        if let NodeKind::EAssign { target, rhs, .. } = &effect.kind {
                            let target_entry = self.target_entry(target);
                            let target_name = target_entry
                                .map(|entry| self.reg.ents[entry].name.as_str())
                                .unwrap_or("assignment");
                            // A self-recurrence is the stronger violation and keeps diagnostic
                            // precedence over any coincident domain mismatch.
                            if target_entry.is_none_or(|entry| !self.scan_reads(rhs, entry)) {
                                self.check_scan_domains(
                                    rhs,
                                    sel.as_deref(),
                                    dynamic_selection,
                                    target_name,
                                )?;
                            }
                        }
                    }
                    let sel = match sel {
                        Some(sel) => Some(Box::new(self.normalize(
                            sel,
                            Context::Mask,
                            TracePhase::Predicate,
                        )?)),
                        None => None,
                    };
                    let effects = effects
                        .iter()
                        .map(|effect| self.normalize(effect, Context::Neutral, TracePhase::Effect))
                        .collect::<Result<Vec<_>, _>>()?;
                    NodeKind::Stmt {
                        sel,
                        effects,
                        rule: *rule,
                        cont: *cont,
                        elided: *elided,
                    }
                }
                NodeKind::DefStmt { name, body } => {
                    let body = self.normalize(body, Context::Neutral, phase)?;
                    if !matches!(body.kind, NodeKind::Stmt { .. }) {
                        let spelling = self.spelling(*name).to_string();
                        let carrier = self.infer(&body);
                        self.derived.insert(spelling, carrier);
                    }
                    NodeKind::DefStmt {
                        name: *name,
                        body: Box::new(body),
                    }
                }
                NodeKind::UndefStmt { name } => NodeKind::UndefStmt { name: *name },
                NodeKind::Query(inner) => NodeKind::Query(Box::new(self.normalize(
                    inner,
                    Context::Neutral,
                    TracePhase::Predicate,
                )?)),
                NodeKind::Compr { sel, effect, rest } => NodeKind::Compr {
                    sel: Box::new(self.normalize(sel, Context::Mask, TracePhase::Predicate)?),
                    effect: Box::new(self.normalize(effect, Context::Neutral, TracePhase::Effect)?),
                    rest: rest
                        .iter()
                        .map(|item| {
                            let context = if matches!(&item.kind, NodeKind::Binder { .. }) {
                                Context::Value
                            } else {
                                Context::Mask
                            };
                            self.normalize(item, context, TracePhase::Predicate)
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                },
                NodeKind::Program(items) => {
                    let mut normalized = Vec::with_capacity(items.len());
                    let mut have_antecedent = false;
                    let mut fresh_rules = false;
                    for item in items {
                        let installed_rule = match &item.kind {
                            NodeKind::DefStmt { body, .. } => {
                                matches!(body.kind, NodeKind::Stmt { .. })
                            }
                            NodeKind::Stmt { rule: true, .. } => true,
                            _ => false,
                        };
                        if installed_rule {
                            fresh_rules = true;
                            normalized.push(self.normalize(
                                item,
                                Context::Neutral,
                                TracePhase::Predicate,
                            )?);
                            continue;
                        }
                        if matches!(item.kind, NodeKind::DefStmt { .. }) {
                            normalized.push(self.normalize(
                                item,
                                Context::Neutral,
                                TracePhase::Predicate,
                            )?);
                            continue;
                        }
                        // The lowerer fires a pending rule set before the next performed item;
                        // that barrier supplies the same saved antecedent the lowerer records.
                        if fresh_rules {
                            have_antecedent = true;
                            fresh_rules = false;
                        }
                        let cold = matches!(
                            item.kind,
                            NodeKind::Stmt { elided: true, .. }
                        ) && !have_antecedent;
                        let prepared = if cold {
                            let NodeKind::Stmt { effects, rule, cont, .. } = &item.kind else {
                                unreachable!()
                            };
                            let cursor = self.intern("cursor");
                            Node::new(
                                NodeKind::Stmt {
                                    sel: Some(Box::new(Node::new(
                                        NodeKind::Alias { look: cursor, req: cursor },
                                        item.line,
                                    ))),
                                    effects: effects.clone(),
                                    rule: *rule,
                                    cont: *cont,
                                    elided: false,
                                },
                                item.line,
                            )
                        } else {
                            item.clone()
                        };
                        let gives_antecedent = matches!(
                            prepared.kind,
                            NodeKind::Stmt { .. } | NodeKind::Compr { .. }
                        );
                        normalized.push(self.normalize(
                            &prepared,
                            Context::Neutral,
                            TracePhase::Predicate,
                        )?);
                        if gives_antecedent {
                            have_antecedent = true;
                        }
                    }
                    NodeKind::Program(normalized)
                }
            };
            Ok(Node::new(kind, line))
        }
    }

    fn emit_trace_uses(mut bqn: String, plan: &TracePlan, enabled: bool) -> String {
        if !enabled || (plan.uses().is_empty() && plan.alias_lines().is_empty()) {
            return bqn;
        }
        let marker = "\n# expectations\n";
        let mut block = String::from("\n# semantic relationship-use domains\n");
        for use_ in plan.uses() {
            block.push_str(&format!(
                "•Out anoTraceSep∾\"TRACE-USE {} {}:{} {} {} {}\"\n",
                use_.id.0,
                use_.site,
                use_.line,
                use_.phase.word(),
                use_.domain.word(),
                use_.relation
            ));
        }
        // one line per `^name` consultation: which mechanism answered, and at which versions
        for line in plan.alias_lines() {
            block.push_str(&format!("•Out anoTraceSep∾\"TRACE-ALIAS {}\"\n", line));
        }
        if let Some(position) = bqn.find(marker) {
            bqn.insert_str(position, &block);
        } else {
            bqn.push_str(&block);
        }
        bqn
    }

    pub(super) fn emit(
        prog: &Node,
        reg: &Registry,
        dirs: &Directives,
        it: &Interner,
        aliases: AliasSnapshot,
    ) -> Result<String, Diag> {
        crate::relationship::validate_registry(reg)?;
        aliases.validate(reg)?;
        let mut normalizer = Normalizer::new(reg, it, aliases);
        let program = normalizer.normalize(prog, Context::Neutral, TracePhase::Predicate)?;
        normalizer.refuse_reserved(prog)?;
        // the alias plan crosses into the lowerer, which mints one record per staged crossing
        let plan = std::mem::take(&mut normalizer.trace);
        let (bqn, plan) = emit_lowered(&program, &normalizer.reg, dirs, &normalizer.it, plan)?;
        let bqn = emit_trace_uses(bqn, &plan, dirs.trace);
        Ok(bqn)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::alias::AliasEnvironment;

        // Two Num columns over three rows; Silver is the effect target everywhere.
        fn registry() -> Registry {
            let col = |name: &str, nums: Vec<f64>| RegEntry {
                name: name.to_string(),
                defval: 0.0,
                kind: RegEntryKind::Col {
                    ty: ColType::Num,
                    uniq: false,
                    nums,
                    syms: Vec::new(),
                    pres: None,
                    rng: None,
                },
            };
            Registry {
                n: 3,
                ents: vec![col("Gold", vec![1.0, 2.0, 3.0]), col("Silver", vec![0.0, 0.0, 0.0])],
                ..Registry::default()
            }
        }

        // Inputs: Ano source. Outputs: the parsed program and the interner that holds its symbols.
        fn program(source: &str) -> (Node, Interner) {
            let mut it = Interner::new();
            let toks = crate::lex::lex(source.as_bytes(), false, &mut it).expect("lex");
            let prog = crate::parse::parse(&toks, &mut it).expect("parse");
            (prog, it)
        }

        fn nodes(node: &Node, out: &mut Vec<Node>) {
            out.push(node.clone());
            for kid in super::super::children(node) {
                nodes(kid, out);
            }
        }

        // The stems of every Alias node in a tree, in traversal order.
        fn alias_stems(normalizer: &Normalizer, node: &Node) -> Vec<String> {
            let mut all = Vec::new();
            nodes(node, &mut all);
            all.iter()
                .filter_map(|n| match &n.kind {
                    NodeKind::Alias { look: symbol, .. } => Some(normalizer.spelling(*symbol).to_string()),
                    _ => None,
                })
                .collect()
        }

        fn name_stems(normalizer: &Normalizer, node: &Node) -> Vec<String> {
            let mut all = Vec::new();
            nodes(node, &mut all);
            all.iter()
                .filter_map(|n| match &n.kind {
                    NodeKind::Name(symbol) => Some(normalizer.spelling(*symbol).to_string()),
                    _ => None,
                })
                .collect()
        }

        fn normalized(source: &str, environment: &AliasEnvironment, reg: &Registry) -> (Normalizer, Node) {
            let (prog, it) = program(source);
            let mut normalizer = Normalizer::new(reg, &it, environment.snapshot(reg).expect("snapshot"));
            let out = normalizer
                .normalize(&prog, Context::Neutral, TracePhase::Predicate)
                .expect("normalize");
            (normalizer, out)
        }

        /* ---------- folds, scans, and the validity channel ---------- */

        // Gold/Silver numeric, Burning/Path masks, threat a registered reducer.
        fn fixture() -> Registry {
            let mut reg = registry();
            let mask = |name: &str, nums: Vec<f64>| RegEntry {
                name: name.to_string(),
                defval: 0.0,
                kind: RegEntryKind::Col {
                    ty: ColType::Bool,
                    uniq: false,
                    nums,
                    syms: Vec::new(),
                    pres: None,
                    rng: None,
                },
            };
            reg.ents.push(mask("Burning", vec![1.0, 0.0, 1.0]));
            reg.ents.push(mask("Path", vec![1.0, 1.0, 0.0]));
            reg.ents.push(RegEntry {
                name: "threat".to_string(),
                defval: 0.0,
                kind: RegEntryKind::Fn { body: Some("{𝕨⌈𝕩}".to_string()) },
            });
            reg
        }

        fn emitted_with(source: &str, reg: &Registry, dirs: &Directives) -> String {
            let (prog, it) = program(source);
            let environment = AliasEnvironment::for_registry(reg);
            emit(&prog, reg, dirs, &it, environment.snapshot(reg).expect("snapshot")).expect("emit")
        }

        fn emitted(source: &str) -> String {
            emitted_with(source, &fixture(), &Directives::default())
        }

        fn refused(source: &str) -> String {
            let reg = fixture();
            let (prog, it) = program(source);
            let dirs = Directives::default();
            let environment = AliasEnvironment::for_registry(&reg);
            emit(&prog, &reg, &dirs, &it, environment.snapshot(&reg).expect("snapshot"))
                .expect_err("must refuse")
                .msg
        }

        #[test]
        fn derived_definition_carrier_is_visible_to_later_effect_checks() {
            let text = emitted("def score = Gold + Silver\nBurning , Gold = score\n");
            assert!(text.contains("gold ↩"), "{}", text);

            let error = refused("def flag = Burning\nBurning , Gold = flag\n");
            assert!(
                error.contains("cannot write mask result to number 'Gold'"),
                "{}",
                error
            );
        }

        #[test]
        fn full_world_scan_composes_with_an_ordinary_world_column() {
            let text = emitted("(+\\ Gold) == Silver\n");
            assert!(text.contains("((+`gold)=silver)"), "{}", text);
        }

        // The statement body of an emission: the prelude and the fixture are not under test.
        fn body(bqn: &str) -> String {
            let start = bqn.find("\n# q1\n").expect("query");
            let end = bqn.find("\n# expectations\n").expect("expectations");
            bqn[start..end].to_string()
        }

        // The missing glyph bridge: `min\` lowers, and the numeric `&\` instance is the SAME
        // operation, so the two spellings emit one byte-identical program.
        #[test]
        fn numeric_lesser_scan_lowers_through_one_descriptor() {
            let long = emitted("min\\ Gold\n");
            assert_eq!(body(&long), "\n# q1\nq1 ← (⌊`gold)\n•Show q1\n");
            assert_eq!(emitted("&\\ Gold\n"), long);
            assert_eq!(emitted("max\\ Gold\n"), emitted("|\\ Gold\n"));
            // no seed and no manufactured infinity for the extrema
            assert!(!long.contains('∞') && !long.contains("0∾") && !long.contains("1∾"));
        }

        // The mask instances keep their own scan glyphs; the carrier picks the operation.
        #[test]
        fn mask_scans_keep_the_boolean_instance() {
            assert!(emitted("|\\ Burning\n").contains("q1 ← (∨`burning)"));
            assert!(emitted("&\\ Burning\n").contains("q1 ← (∧`burning)"));
        }

        // The count machine consumes the presence stream of the SCOPE, so a scoped running
        // count and a scoped running sum compress exactly the same rows.
        #[test]
        fn scoped_machines_scan_the_scoped_domain() {
            assert!(emitted("#\\ Gold @ Path\n").contains("q1 ← (+`path/(¬(¬(1¨gold))))"));
            // the mean's two prefix sums sit under one compression, so they align by construction
            assert!(emitted("avg\\ Gold @ Path\n")
                .contains("q1 ← ((+`path/gold)÷(+`path/(¬(¬(1¨gold)))))"));
            // a numeric operand is the all-true membership stream: zeros do not skip
            assert!(emitted("#\\ Gold\n").contains("q1 ← (+`(¬(¬(1¨gold))))"));
        }

        // scan(f) col along ord resolves through the same table: the mean desugaring's two
        // operands carry one and the same order, so composing them is one domain.
        #[test]
        fn scan_along_resolves_through_the_same_table() {
            assert!(emitted("scan(avg) Gold along Silver\n")
                .contains("q1 ← ((+`(silver)⊏gold)÷(+`(silver)⊏(¬(¬(1¨gold)))))"));
            assert!(emitted("scan(&) Burning along Silver\n").contains("∧`"));
            assert!(emitted("scan(threat) Gold along Silver\n").contains("Fn_threat`"));
        }

        // Arithmetic heads consume the numeric reading of a mask in every value form. Named
        // reducers and numeric bridge names remain strict because no promotion is declared for them.
        #[test]
        fn arithmetic_heads_promote_masks_across_forms() {
            for source in ["+\\ Burning\n", "*\\ Burning\n", "+/ Burning\n", "avg\\ Burning\n"] {
                let plan = emitted(source);
                assert!(
                    plan.contains("Fn_AnoSemMaskNumber"),
                    "{}: {}",
                    source,
                    plan
                );
            }
            assert!(emitted("avg\\ Burning\n").contains("Fn_AnoSemExtent"));
            assert!(refused("max/ Burning\n").contains("reducer 'max' is not defined on Mask"));
            assert!(refused("threat\\ Burning\n").contains("reducer 'threat' is not defined on Mask"));
            // a name is admitted by the registry, never by being a name
            assert!(refused("nope/ Gold\n").contains("unknown reducer 'nope'"));
            assert!(refused("nope\\ Gold\n").contains("unknown reducer 'nope'"));
        }

        // Greater/Lesser joins mask and number at number in value position. Selection position
        // retains the mask reading; the operator, not a universal carrier hierarchy, decides.
        #[test]
        fn mixed_mask_number_greater_lesser_promote_in_value_position() {
            let greater = emitted("Silver = (Gold | Burning)\n");
            assert!(greater.contains("Fn_AnoSemMaskNumber"), "{}", greater);
            assert!(greater.contains("Fn_AnoSemGreater"), "{}", greater);
            let lesser = emitted("Silver = (Gold & Burning)\n");
            assert!(lesser.contains("Fn_AnoSemMaskNumber"), "{}", lesser);
            assert!(lesser.contains("Fn_AnoSemLesser"), "{}", lesser);
            assert!(emitted("Burning & Gold , +Path\n").contains("burning∧"));
        }

        // A per-fiber scan needs a ragged result representation that does not exist yet.
        #[test]
        fn scan_over_fibers_refuses_precisely() {
            let mut reg = fixture();
            reg.ents.push(RegEntry {
                name: "r".to_string(),
                defval: 0.0,
                kind: RegEntryKind::SRel {
                    fib: vec![vec![0.0], vec![], vec![1.0, 2.0]],
                    key_of: None,
                    inv_of: None,
                },
            });
            let (prog, it) = program("+\\ r'.Gold\n");
            let environment = AliasEnvironment::for_registry(&reg);
            let message = emit(
                &prog,
                &reg,
                &Directives::default(),
                &it,
                environment.snapshot(&reg).expect("snapshot"),
            )
            .expect_err("must refuse")
            .msg;
            assert!(message.contains("scan over fibers is not yet supported"), "{}", message);
        }

        // A12 at the only layer that can express runtime skipping: the guard is staged once and
        // every observation runs under it.  Bare, labelled, and pinned paths all consume it.
        #[test]
        fn identityless_empty_fold_cannot_expose_its_placeholder() {
            let bare = emitted("max/ Gold @ Burning\n");
            // the guard counts the admitted rows: the emitter's own reduction, in BQN's order
            assert!(bare.contains("q1v ← (0<(+´burning))"), "{}", bare);
            assert!(bare.contains("•Show⍟q1v q1"), "{}", bare);

            let mut dirs = Directives::default();
            dirs.label = true;
            let labelled = emitted_with("max/ Gold @ Burning\n", &fixture(), &dirs);
            // one conditional carries BOTH the 0x1D tag and the value: a false guard emits neither
            assert!(
                labelled.contains("{•Out (@+29)∾\"q1@1\" ⋄ •Show 𝕩}⍟q1v q1"),
                "{}",
                labelled
            );
            assert!(!labelled.contains("\n•Out (@+29)∾\"q1@1\"\n"), "{}", labelled);

            let mut pinned = Directives::default();
            pinned.expects.push(crate::Expect::Out { vals: Vec::new() });
            let compared = emitted_with("max/ Gold @ Burning\n", &fixture(), &pinned);
            // the comparator sees the guarded ravel, so the placeholder is unpinnable
            assert!(compared.contains("q1v/⥊q1"), "{}", compared);
            assert!(!compared.contains("} ⥊q1"), "{}", compared);
        }

        // Identity-bearing folds keep their real scalar results and stage no guard at all.
        #[test]
        fn identity_bearing_folds_stay_unguarded() {
            for source in ["+/ Gold @ Burning\n", "*/ Gold @ Burning\n", "#/ Gold @ Burning\n"] {
                let bqn = emitted(source);
                assert!(!bqn.contains("q1v"), "{}: {}", source, bqn);
                assert!(bqn.contains("•Show q1"), "{}: {}", source, bqn);
            }
        }

        // The master invariant: a query with no validity guard stages exactly as it always did.
        // This is the 038 control, pinned as a string.
        #[test]
        fn unguarded_mask_scan_emits_byte_identically() {
            assert_eq!(body(&emitted("|\\ Burning @ Path\n")), "\n# q1\nq1 ← (∨`path/burning)\n•Show q1\n");
            assert_eq!(body(&emitted("|/ Burning @ Path\n")), "\n# q1\nq1 ← (AnoLeftOr (path/burning))\n•Show q1\n");
            assert_eq!(body(&emitted("&/ Burning @ Path\n")), "\n# q1\nq1 ← (AnoLeftAnd (path/burning))\n•Show q1\n");
        }

        // An overlay miss keeps the sigiled request as an Alias node; the bare spelling never
        // becomes one.  This is completion gate 1: node kind, not source text.
        #[test]
        fn normalizer_keeps_fallback_alias_distinct() {
            let reg = registry();
            let environment = AliasEnvironment::for_registry(&reg);
            let (normalizer, out) = normalized("Gold > ^Nope , Silver = 0\n", &environment, &reg);
            assert_eq!(alias_stems(&normalizer, &out), vec!["Nope".to_string()]);

            let (normalizer, out) = normalized("Gold > Nope , Silver = 0\n", &environment, &reg);
            assert!(alias_stems(&normalizer, &out).is_empty());
            assert!(name_stems(&normalizer, &out).iter().any(|n| n == "Nope"));
        }

        // A binding entry moves the lookup name to the target and keeps the Alias kind.
        #[test]
        fn dynamic_alias_entry_resolves_to_target_symbol() {
            let reg = registry();
            let mut environment = AliasEnvironment::for_registry(&reg);
            environment.install_binding(&reg, "focus", "Gold").expect("install");
            let (normalizer, out) = normalized("^focus , Silver = 0\n", &environment, &reg);
            assert_eq!(alias_stems(&normalizer, &out), vec!["Gold".to_string()]);
        }

        // A mask entry materializes one synthetic AliasMask entry and records its reserved name.
        #[test]
        fn dynamic_alias_mask_materializes_reserved_entry() {
            let reg = registry();
            let mut environment = AliasEnvironment::for_registry(&reg);
            environment.install_mask(&reg, "hot", &[1.0, 0.0, 1.0]).expect("install");
            let (normalizer, out) = normalized("^hot , Silver = 0\n", &environment, &reg);
            assert_eq!(alias_stems(&normalizer, &out), vec!["AnoDynMask0".to_string()]);
            assert!(normalizer.generated.contains("AnoDynMask0"));
            let entry = normalizer
                .reg
                .ents
                .iter()
                .find(|e| e.name == "AnoDynMask0")
                .expect("materialized entry");
            assert!(matches!(&entry.kind, RegEntryKind::AliasMask { mask } if mask == &[1.0, 0.0, 1.0]));
        }

        // An overlay entry spelled like a column is invisible to the bare spelling (work 3).
        #[test]
        fn bare_name_never_consults_overlay() {
            let reg = registry();
            let mut environment = AliasEnvironment::for_registry(&reg);
            environment.install_binding(&reg, "gold", "Silver").expect("install");
            let (normalizer, out) = normalized("Gold , Silver = 0\n", &environment, &reg);
            assert!(alias_stems(&normalizer, &out).is_empty());
            assert!(name_stems(&normalizer, &out).iter().any(|n| n == "Gold"));
        }

        // The requested stems of every Alias node in a tree, in traversal order.
        fn alias_reqs(normalizer: &Normalizer, node: &Node) -> Vec<String> {
            let mut all = Vec::new();
            nodes(node, &mut all);
            all.iter()
                .filter_map(|n| match &n.kind {
                    NodeKind::Alias { req, .. } => Some(normalizer.spelling(*req).to_string()),
                    _ => None,
                })
                .collect()
        }

        // Completion gate: the overlay moves the lookup, never the source request.  A refusal
        // reached through a moved `^focus` spells the sigiled stem and names the target it hit.
        #[test]
        fn resolved_alias_refusal_keeps_the_sigiled_request() {
            let mut reg = registry();
            reg.ents.push(RegEntry {
                name: "spot".to_string(),
                defval: 0.0,
                kind: RegEntryKind::Bind { kind: BindKind::Point, vals: vec![1.0, 2.0] },
            });
            let mut environment = AliasEnvironment::for_registry(&reg);
            environment.install_binding(&reg, "focus", "spot").expect("install");

            let (normalizer, out) = normalized("!^focus , Silver = 0\n", &environment, &reg);
            assert_eq!(alias_stems(&normalizer, &out), vec!["spot".to_string()]);
            assert_eq!(alias_reqs(&normalizer, &out), vec!["focus".to_string()]);

            let (prog, it) = program("!^focus , Silver = 0\n");
            let dirs = Directives::default();
            let error = emit(&prog, &reg, &dirs, &it, environment.snapshot(&reg).expect("snapshot"))
                .expect_err("must refuse");
            assert!(
                error.msg.contains("'^focus' resolves to binding 'spot' (point), not a mask"),
                "unexpected diagnostic: {}",
                error.msg
            );
        }

        // A resolver entity result materializes one synthetic Bind{Entity} entry; the node's
        // requested stem still reads back as the source spelling.
        #[test]
        fn resolver_entity_materializes_reserved_bind_entry() {
            let reg = registry();
            let mut environment = AliasEnvironment::for_registry(&reg);
            environment
                .install_resolver(
                    &reg,
                    "focus",
                    "input.entity",
                    &[("entity".to_string(), "1".to_string())],
                )
                .expect("install");
            let (normalizer, out) = normalized("^focus , Silver = 0\n", &environment, &reg);
            assert_eq!(alias_stems(&normalizer, &out), vec!["AnoDynEnt0".to_string()]);
            assert_eq!(alias_reqs(&normalizer, &out), vec!["focus".to_string()]);
            assert!(normalizer.generated.contains("AnoDynEnt0"));
            let entry = normalizer
                .reg
                .ents
                .iter()
                .find(|e| e.name == "AnoDynEnt0")
                .expect("materialized entry");
            assert!(matches!(
                &entry.kind,
                RegEntryKind::Bind { kind: BindKind::Entity, vals } if vals == &[1.0]
            ));
        }

        // One trace line per consultation naming the mechanism and the overlay version, and the
        // master invariant: with the channel off the plan is byte-identical to the flagless one,
        // so the block is purely appended.
        #[test]
        fn alias_trace_provenance_is_gated() {
            let reg = registry();
            let mut environment = AliasEnvironment::for_registry(&reg);
            environment.install_binding(&reg, "focus", "Gold").expect("install");
            let (prog, it) = program("^focus > 1 & ^Silver < 2 , Silver = 0\n");

            let mut dirs = Directives::default();
            let quiet = emit(&prog, &reg, &dirs, &it, environment.snapshot(&reg).expect("snapshot"))
                .expect("emit");
            dirs.trace = true;
            let traced = emit(&prog, &reg, &dirs, &it, environment.snapshot(&reg).expect("snapshot"))
                .expect("emit");

            assert!(!quiet.contains("TRACE-ALIAS"), "{}", quiet);
            let block = concat!(
                "\n# semantic relationship-use domains\n",
                "•Out anoTraceSep∾\"TRACE-ALIAS ^focus -> binding 'Gold', overlay v1\"\n",
                "•Out anoTraceSep∾\"TRACE-ALIAS ^Silver -> bare fallback, overlay v1\"\n",
            );
            assert!(traced.contains(block), "{}", traced);
            // the flag gates whole appended blocks; the lowered statement body is untouched
            let body = |bqn: &str| {
                let start = bqn.find("\n# s1\n").expect("statement");
                bqn[start..bqn.find("\n# expectations\n").expect("expectations")].to_string()
            };
            assert_eq!(body(&traced.replace(block, "")), body(&quiet));
        }

        // Source may not spell a name this emission minted: the synthetic overlay content must
        // not leak into bare lookup.
        #[test]
        fn reserved_synthetic_name_refuses() {
            let reg = registry();
            let mut environment = AliasEnvironment::for_registry(&reg);
            environment.install_mask(&reg, "hot", &[1.0, 0.0, 1.0]).expect("install");
            let (prog, it) = program("AnoDynMask0 & ^hot , Silver = 0\n");
            let dirs = Directives::default();
            let error = emit(&prog, &reg, &dirs, &it, environment.snapshot(&reg).expect("snapshot"))
                .expect_err("must refuse");
            assert!(
                error.msg.contains("reserved synthetic name 'AnoDynMask0'"),
                "unexpected diagnostic: {}",
                error.msg
            );
        }
    }
}

/// Emit against an immutable dynamic-alias snapshot.  This is the host boundary used by Kore.
pub fn emit_with_aliases(
    prog: &Node,
    reg: &Registry,
    dirs: &Directives,
    it: &Interner,
    aliases: crate::alias::AliasSnapshot,
) -> Result<String, Diag> {
    normalize::emit(prog, reg, dirs, it, aliases)
}

/// Canonical public emission path.  Alias state is loaded once and frozen before any lowering.
pub fn emit(
    prog: &Node,
    reg: &Registry,
    dirs: &Directives,
    it: &Interner,
) -> Result<String, Diag> {
    crate::relationship::validate_registry(reg)?;
    let aliases = if dirs.aliases.is_empty() {
        crate::alias::AliasEnvironment::for_registry(reg)
    } else {
        crate::alias::AliasEnvironment::load(&dirs.aliases, reg)?
    };
    emit_with_aliases(prog, reg, dirs, it, aliases.snapshot(reg)?)
}
