// Steel's BQN backend. Ordinary statements read pre-state, stage effects, then commit at the
// barrier. Installed rules share one pre-state and commit set per synthetic tick.
// Continuations reuse anoSel. --emit remains differential-tested against the C oracle.

use crate::num::{fmt_g, int_fast};
use crate::registry::{names_eq, reg_find, reg_role};
use crate::{
    ArithOp, AssignOp, BindKind, CmpOp, ColType, Diag, Directives, Expect, Interner, Node,
    NodeKind, RegEntry, RegEntryKind, Registry, Symbol,
};

type R<T> = Result<T, Diag>;

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

// C EV: v the expr (already in mode), g the WORLD-SPACE guard (None = total), pair/unit/sym
// value shape, along the scan-along order expr, rel_ent the rel whose VALUES v holds.
#[derive(Clone, Default)]
struct Ev {
    v: String,
    g: Option<String>,
    pair: bool,
    unit: bool,
    sym: bool,
    along: Option<String>,
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

// The fold glyph table; None for #, avg, named reducers.
fn fold_gl(op: &str) -> Option<&'static str> {
    match op {
        "+" => Some("+´"),
        "*" => Some("×´"),
        "&" => Some("∧´"),
        "|" => Some("∨´"),
        "max" => Some("⌈´"),
        "min" => Some("⌊´"),
        _ => None,
    }
}

// Folds with identities: empty gathers need no guard.
fn fold_has_id(op: &str) -> bool {
    matches!(op, "+" | "*" | "&" | "|" | "#")
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
        NodeKind::Name(s) => *s,
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

    // Has-a-live-target guard for a bare rel.
    fn rel_guard(&self, ri: usize) -> String {
        let v = self.bqnv(ri);
        match self.rel_key(ri) {
            None => format!("(0≤{})", v),
            Some(k) => format!("(({}⊐{})<≠{})", k, v, k),
        }
    }

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

    // --trace dead-link hook: zero output without the flag; g (None = total) is the
    // accumulated guard of earlier legs.
    fn trace_dead(&mut self, name: &str, key: &str, rel: &str, g: Option<&str>) {
        if !self.dirs.trace {
            return;
        }
        let dm = self.tv();
        match g {
            Some(g) => self.stage(format!("{} ← {}∧(0≤{})∧(≠{})≤{}⊐{}", dm, g, rel, key, key, rel)),
            None => self.stage(format!("{} ← (0≤{})∧(≠{})≤{}⊐{}", dm, rel, key, key, rel)),
        }
        let tids = self.trace_ids(rel);
        self.stage(format!("AnoTraceDead ⟨\"{}\", {}/{}, {}/{}⟩", name, dm, tids, dm, rel));
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
            NodeKind::Name(s) | NodeKind::Alias(s) | NodeKind::Sym(s) | NodeKind::Str(s) => self.rs(*s),
            NodeKind::SetHop { rel } => self.rs(*rel),
            NodeKind::Call { callee, .. } => self.rs(*callee),
            NodeKind::CmpAny { name } => self.rs(*name),
            _ => "",
        }
    }
}

/* ---------- names as values, masks, grades, pipes ---------- */

impl<'a> Em<'a> {
    // Inputs: a name spelling + line, mode. Output: EV. Handles index/x/y/char specials,
    // defs, then registry entries by kind. Invariant: guard is world-space.
    fn emit_name_val(&mut self, sym: Symbol, line: i32, m: Mode) -> R<Ev> {
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
            return Err(fail(line, format!("unregistered name '{}'", n)));
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
                ev.v = self.in_mode(self.bqnv(ei), m);
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
            RegEntryKind::Fn { .. } => Err(fail(line, format!("name '{}' (fn) in value position", n))),
        }
    }

    // Bare name as a mask: bool col -> pres∧values; value col -> presence; alias/bind mask.
    fn emit_name_mask(&mut self, sym: Symbol, line: i32) -> R<String> {
        let n = self.rs(sym);
        if let Some(d) = self.find_def(sym) {
            return self.emit_mask(def_body(d));
        }
        let Some(ei) = self.find(n) else {
            return Err(fail(line, format!("unregistered mask name '{}'", n)));
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
                    Ok(format!("((↕anoN)={})", num_lit(vals.first().copied().unwrap_or(0.0))))
                }
                k => Err(fail(line, format!("binding '{}' ({}) as mask", n, bind_kind_str(*k)))),
            },
            RegEntryKind::Rel { .. } => Ok(self.rel_guard(ei)),
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
    // inverse image over the stable-id column (staged).
    fn fiber_var(&mut self, sym: Symbol, line: i32) -> R<String> {
        let n = self.rs(sym);
        if let Some(ei) = self.find(n) {
            match &self.ent(ei).kind {
                RegEntryKind::SRel { .. } => return Ok(self.bqnv(ei)),
                RegEntryKind::Col { ty: ColType::Num | ColType::Nat | ColType::Int, .. } => {
                    let t = self.tv();
                    let idc = self.id_col();
                    let v = self.bqnv(ei);
                    self.stage(format!("{} ← {{/{}=𝕩}}¨{}", t, v, idc));
                    return Ok(t);
                }
                _ => {}
            }
        }
        Err(fail(line, format!("'{}' is not a set-valued relationship or a key column", n)))
    }

    // Translate stored srel fibers to CURRENT row indices when the key space can disagree:
    // declared key always, the idx key once shifted; else pass through untouched.
    fn fiber_rows(&mut self, sym: Symbol, fib: &mut String) {
        let Some(ei) = self.find(self.rs(sym)) else { return };
        let e = self.ent(ei);
        let RegEntryKind::SRel { fib: fibers, key_of: ko, .. } = &e.kind else { return };
        if fibers.len() != self.reg.n as usize {
            return;
        }
        let mut key: Option<String> = None;
        if let Some(k) = ko {
            if let Some(kc) = self.find(k) {
                key = Some(self.bqnv(kc));
            }
        } else if self.world_shifted {
            key = Some(self.id_col());
        }
        let Some(key) = key else { return };
        // --trace: a dead member is a dead link crossed inside the fiber
        if self.dirs.trace {
            let tids = self.trace_ids(fib);
            self.stage(format!(
                "{} {{m←(≠{})≤{}⊐𝕩 ⋄ AnoTraceDead ⟨\"{}\", (+´m)⥊𝕨, m/𝕩⟩}}¨ {}",
                tids, key, key, e.name, fib
            ));
        }
        let t = self.tv();
        self.stage(format!("{} ← {{k←{}⊐𝕩 ⋄ (k<≠{})/k}}¨{}", t, key, key, fib));
        *fib = t;
    }

    // --trace empty-fiber failure mask: effect position scopes to the pre-refinement
    // selection; a predicate stays whole-column.
    fn trace_empty_mask(&self, fib: &str, m: Mode) -> String {
        if m != Mode::World && !self.trace_sel.is_empty() {
            return format!("({}∧0=≠¨{})", self.trace_sel, fib);
        }
        format!("(0=≠¨{})", fib)
    }

    // Gamma fold: fold/ rel'.Comp | fold/ (rel' & pred) | fold/ rel' -> per-source column + guard.
    fn emit_gamma(&mut self, op: &str, operand: &Node, m: Mode) -> R<Ev> {
        let mut ev = Ev::default();
        match &operand.kind {
            NodeKind::SetHop { rel } => {
                // #/ attackers'
                let mut fib = self.fiber_var(*rel, operand.line)?;
                self.fiber_rows(*rel, &mut fib);
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
                let mut fib = self.fiber_var(rel, l.line)?;
                self.fiber_rows(rel, &mut fib);
                let fb_name = self.rs(rel);
                let cv = self.emit_val(r, Mode::World)?;
                let gl = fold_gl(op);
                let t = self.tv();
                if op == "avg" {
                    if self.dirs.trace {
                        let tem = self.trace_empty_mask(&fib, m);
                        let tids = self.trace_ids(&fib);
                        self.stage(format!("AnoTraceEmpty ⟨\"{}\", {}/{}⟩", fb_name, tem, tids));
                    }
                    self.stage(format!("{} ← {{0=≠𝕩 ? 0 ; AnoAvg 𝕩⊏{}}}¨{}", t, cv.v, fib));
                    ev.g = Some(format!("(0<≠¨{})", fib));
                } else if op == "#" {
                    self.stage(format!("{} ← {{+´𝕩⊏{}}}¨{}", t, cv.v, fib));
                } else if let Some(gl) = gl {
                    if fold_has_id(op) {
                        self.stage(format!("{} ← {{{}𝕩⊏{}}}¨{}", t, gl, cv.v, fib));
                    } else {
                        // max/min: no identity, guard empties
                        if self.dirs.trace {
                            let tem = self.trace_empty_mask(&fib, m);
                            let tids = self.trace_ids(&fib);
                            self.stage(format!("AnoTraceEmpty ⟨\"{}\", {}/{}⟩", fb_name, tem, tids));
                        }
                        self.stage(format!("{} ← {{0=≠𝕩 ? 0 ; {}𝕩⊏{}}}¨{}", t, gl, cv.v, fib));
                        ev.g = Some(format!("(0<≠¨{})", fib));
                    }
                } else {
                    if let Some(ei) = self.find(op) {
                        if matches!(self.ent(ei).kind, RegEntryKind::Fn { .. }) {
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
                let mut fib = self.fiber_var(rel, l.line)?;
                self.fiber_rows(rel, &mut fib);
                let pm = self.emit_mask(r)?;
                let tp = self.tv();
                self.stage(format!("{} ← {}", tp, pm));
                let t = self.tv();
                match op {
                    "#" => self.stage(format!("{} ← {{+´𝕩⊏{}}}¨{}", t, tp, fib)),
                    "|" => self.stage(format!("{} ← {{∨´𝕩⊏{}}}¨{}", t, tp, fib)),
                    "&" => self.stage(format!("{} ← {{∧´𝕩⊏{}}}¨{}", t, tp, fib)),
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
                let mut fib = self.fiber_var(fs, l.line)?;
                self.fiber_rows(fs, &mut fib);
                let Some(gl) = fold_gl(op) else {
                    return Err(fail(line, format!("fold {}/ @row", op)));
                };
                ev.unit = false;
                ev.v = self.in_mode(format!("({}¨{})", gl, fib), m);
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
            self.stage(format!("{} ← {{0=≠𝕩 ? 0 ; AnoAvg 𝕩}} {}", t, gathered));
            ev.v = t;
            ev.g = Some(format!("(0<{})", cnt));
            return Ok(ev);
        }
        let Some(gl) = fold_gl(op) else {
            // named reducer: registry fn folds pairwise
            let fe = self.find(op).filter(|&i| matches!(self.ent(i).kind, RegEntryKind::Fn { .. }));
            let Some(fe) = fe else {
                return Err(fail(line, format!("unknown reducer '{}'", op)));
            };
            let t = self.tv();
            let fv = self.fnv(fe);
            self.stage(format!("{} ← {{0=≠𝕩 ? 0 ; {}´ 𝕩}} {}", t, fv, gathered));
            ev.v = t;
            ev.g = Some(format!("(0<{})", cnt));
            return Ok(ev);
        };
        if !fold_has_id(op) {
            let t = self.tv();
            self.stage(format!("{} ← {{0=≠𝕩 ? 0 ; {}𝕩}} {}", t, gl, gathered));
            ev.v = t;
            ev.g = Some(format!("(0<{})", cnt));
            return Ok(ev);
        }
        ev.v = format!("({}{})", gl, gathered);
        Ok(ev)
    }

    // Scans: result is an ordered column over the scan's scope, returned as a flat value.
    fn emit_scan(&mut self, op_sym: Symbol, operand: &Node, scan2: bool, line: i32) -> R<Ev> {
        let op = self.rs(op_sym);
        let mut ev = Ev::default();
        let (x, scope): (&Node, Option<&Node>) =
            if let NodeKind::Scope { l, r, .. } = &operand.kind { (l, Some(r)) } else { (operand, None) };
        let gl: String = match op {
            "+" => "+`".to_string(),
            "*" => "×`".to_string(),
            "max" => "⌈`".to_string(),
            "&" => "∧`".to_string(),
            "|" => "∨`".to_string(),
            _ => {
                // named reducer scan: registry fn accumulates pairwise; the empty scope
                // yields the empty column — a scan is length-preserving, no identity consulted
                let fe = self.find(op).filter(|&i| matches!(self.ent(i).kind, RegEntryKind::Fn { .. }));
                let Some(fe) = fe else {
                    return Err(fail(line, format!("unknown scan op '{}'", op)));
                };
                format!("{}`", self.fnv(fe))
            }
        };
        let xv = self.emit_val(x, Mode::World)?;
        if let Some(sc) = scope {
            if let NodeKind::Shape(dims) = &sc.kind {
                let w = dims.first().map(num_of).unwrap_or(0.0) as i32;
                let h = if dims.len() > 1 { num_of(&dims[1]) as i32 } else { 1 };
                ev.v = if scan2 {
                    format!("(⥊{}˘{}({}‿{}⥊{}))", gl, gl, h, w, xv.v)
                } else {
                    format!("(⥊{}({}‿{}⥊{}))", gl, h, w, xv.v)
                };
                return Ok(ev);
            }
            if matches!(sc.kind, NodeKind::Pipe { .. }) {
                let vw = self.emit_pipe(sc)?;
                let xf = match &vw.base {
                    Some(b) => format!("({}/{})", b, xv.v),
                    None => xv.v.clone(),
                };
                ev.v = format!("({}({})⊏{})", gl, vw.idx, xf);
                return Ok(ev);
            }
            // mask scope -> compress; id-list scope (iota arithmetic / vec bind) -> index
            let mut idlist = contains_iota(sc);
            if !idlist {
                if let NodeKind::Name(s) = &sc.kind {
                    if let Some(ei) = self.find(self.rs(*s)) {
                        if matches!(self.ent(ei).kind, RegEntryKind::Bind { kind: BindKind::Vec, .. }) {
                            idlist = true;
                        }
                    }
                }
            }
            if idlist {
                let sv = self.emit_val(sc, Mode::World)?;
                ev.v = format!("({}({})⊏{})", gl, sv.v, xv.v);
                return Ok(ev);
            }
            let msk = self.emit_mask(sc)?;
            ev.v = format!("({}{}/{})", gl, msk, xv.v);
            return Ok(ev);
        }
        ev.v = format!("({}{})", gl, xv.v);
        Ok(ev)
    }
}

/* ---------- general value expressions ---------- */

impl<'a> Em<'a> {
    fn emit_call(&mut self, callee: Symbol, args: &[Node], line: i32, m: Mode) -> R<Ev> {
        let name = self.rs(callee);
        let mut ev = Ev::default();
        if name == "rank" {
            if let Some(a0) = args.first() {
                let a = self.emit_val(a0, m)?;
                ev.v = format!("(AnoRank {})", a.v);
                return Ok(ev);
            }
        }
        let e = self.find(name);
        let fnn: String = match e {
            Some(i) => self.fnv(i),
            None if name == "abs" => "|".to_string(),
            None if name == "sin" => "•math.Sin".to_string(),
            None => return Err(fail(line, format!("unregistered callable '{}'", name))),
        };
        match args.len() {
            1 => {
                let a = self.emit_val(&args[0], m)?;
                ev.g = a.g;
                ev.unit = a.unit;
                ev.v = format!("({}¨{})", fnn, a.v);
                Ok(ev)
            }
            2 => {
                let a = self.emit_val(&args[0], m)?;
                let b = self.emit_val(&args[1], m)?;
                ev.g = g_and(a.g, b.g);
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
                let e = if let NodeKind::Name(bs) = &base.kind { self.find(self.rs(*bs)) } else { None };
                if let Some(ei) = e {
                    if self.is_pair[ei] {
                        let i = (fname.as_bytes()[0] == b'y') as i32;
                        ev.v = self.in_mode(format!("({}⊸⊑¨{})", i, self.bqnv(ei)), m);
                        return Ok(ev);
                    }
                }
            }
        }
        // singleton roots: bindings and aliases mirror-read one row
        if let NodeKind::Name(bs) | NodeKind::Alias(bs) = &base.kind {
            let be = self.find(self.rs(*bs));
            let fe = if let NodeKind::Name(fs) = &field.kind { self.find(self.rs(*fs)) } else { None };
            if let Some(bi) = be {
                // a point binding IS its pos: Player.pos with `bind Player point x y`
                if let RegEntryKind::Bind { kind: BindKind::Point, vals } = &self.ent(bi).kind {
                    ev.v = format!(
                        "(<{}‿{})",
                        num_lit(vals.first().copied().unwrap_or(0.0)),
                        num_lit(vals.get(1).copied().unwrap_or(0.0))
                    );
                    ev.pair = true;
                    ev.unit = true;
                    return Ok(ev);
                }
                if let Some(fi) = fe {
                    let bk = &self.ent(bi).kind;
                    if matches!(bk, RegEntryKind::Bind { .. } | RegEntryKind::AliasMask { .. })
                        && matches!(self.ent(fi).kind, RegEntryKind::Col { .. } | RegEntryKind::Field { .. })
                    {
                        let id = if let RegEntryKind::Bind { vals, .. } = bk {
                            num_lit(vals.first().copied().unwrap_or(0.0))
                        } else {
                            format!("(⊑/{})", self.bqnv(bi))
                        };
                        let pair = self.is_pair[fi];
                        ev.v = if pair {
                            format!("(<{}⊑{})", id, self.bqnv(fi))
                        } else {
                            format!("({}⊑{})", id, self.bqnv(fi))
                        };
                        ev.pair = pair;
                        ev.unit = true;
                        return Ok(ev);
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
                        if let Some(key) = self.rel_key(bi) {
                            // every piece stays a self-contained expression: an assignment
                            // probe reuses the guard after discarding the staging buffer
                            let bname = self.ent(bi).name.as_str();
                            self.trace_dead(bname, &key, &rel, None);
                            let ix = format!("((≠{})|{}⊐{})", key, key, rel);
                            w = format!("({}⊏{})", ix, comp);
                            ev.g = Some(format!("(({}⊐{})<≠{})", key, rel, key));
                            if !is_tag && has_pres(fe_ent) {
                                ev.g = g_and(ev.g, Some(format!("({}⊏{})", ix, self.presv(fi))));
                            }
                        } else {
                            w = format!("((0⌈{})⊏{})", rel, comp);
                            ev.g = Some(format!("(0≤{})", rel));
                            if !is_tag && has_pres(fe_ent) {
                                ev.g = g_and(ev.g, Some(format!("((0⌈{})⊏{})", rel, self.presv(fi))));
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
            if let Some(key) = key {
                // self-contained expressions only (the assignment-probe rule above)
                if let Some(ri) = bv.rel_ent {
                    let rname = self.ent(ri).name.as_str();
                    self.trace_dead(rname, &key, &bv.v, bv.g.as_deref());
                }
                let ix = format!("((≠{})|{}⊐{})", key, key, bv.v);
                w = format!("({}⊏{})", ix, comp);
                ev.g = g_and(bv.g.clone(), Some(format!("(({}⊐{})<≠{})", key, bv.v, key)));
            } else {
                w = format!("((0⌈{})⊏{})", bv.v, comp);
                ev.g = g_and(bv.g.clone(), Some(format!("(0≤{})", bv.v)));
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
            NodeKind::Name(s) => self.emit_name_val(*s, nd.line, m),
            NodeKind::Alias(s) => {
                let n = self.rs(*s);
                let Some(i) = self.find(n) else {
                    return Err(fail(nd.line, format!("unregistered alias '^{}'", n)));
                };
                ev.v = self.in_mode(self.bqnv(i), m);
                Ok(ev)
            }
            NodeKind::Arith { op, l, r } => {
                let a = self.emit_val(l, m)?;
                let b = self.emit_val(r, m)?;
                ev.g = g_and(a.g.clone(), b.g.clone());
                ev.pair = a.pair || b.pair;
                ev.unit = a.unit && b.unit;
                // a scan-along value keeps its order through scalar arithmetic; against another
                // column the two orders disagree and no alignment exists
                if a.along.is_some() || b.along.is_some() {
                    if a.along.is_some() && !b.unit {
                        return Err(fail(nd.line, "scan-along composed against a differently-ordered operand"));
                    }
                    if b.along.is_some() && !a.unit {
                        return Err(fail(nd.line, "scan-along composed against a differently-ordered operand"));
                    }
                    ev.along = a.along.clone().or_else(|| b.along.clone());
                }
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
                ev.g = g_and(a.g, b.g);
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
            NodeKind::ScanExpr { op, operand, scan2 } => self.emit_scan(*op, operand, *scan2, nd.line),
            NodeKind::ScanAlong { op, col, order } => {
                let xv = self.emit_val(col, Mode::World)?;
                let ov = self.emit_val(order, Mode::World)?;
                let opn = self.rs(*op);
                let gl = match opn {
                    "+" => "+`",
                    "*" => "×`",
                    "max" => "⌈`",
                    "min" => "⌊`",
                    _ => return Err(fail(nd.line, format!("scan({}): no registered scan step", opn))),
                };
                ev.v = format!("({}({})⊏{})", gl, ov.v, xv.v);
                ev.along = Some(ov.v);
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
                let fe = self.find(self.rs(*f)).filter(|&i| matches!(self.ent(i).kind, RegEntryKind::Fn { .. }));
                let Some(fe) = fe else {
                    return Err(fail(nd.line, "cross needs a registered fn"));
                };
                ev.v = format!("(⥊(/{}){}⌜(/{}))", am, self.fnv(fe), bm);
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
            NodeKind::Name(s) => self.emit_name_mask(*s, nd.line),
            NodeKind::Alias(s) => {
                let n = self.rs(*s);
                let Some(i) = self.find(n) else {
                    return Err(fail(nd.line, format!("unregistered alias '^{}'", n)));
                };
                Ok(self.bqnv(i))
            }
            NodeKind::And(a, b) | NodeKind::Or(a, b) => {
                let am = self.emit_mask(a)?;
                let bm = self.emit_mask(b)?;
                Ok(format!("({}{}{})", am, if matches!(nd.kind, NodeKind::And(..)) { "∧" } else { "∨" }, bm))
            }
            NodeKind::Not(k) => {
                // absent-component reading for sparse value columns
                if let NodeKind::Name(s) = &k.kind {
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
                let fib = self.fiber_var(rel, r.line)?;
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
                                let tids = self.trace_ids(&fib);
                                self.stage(format!(
                                    "({}/{}) {{m←(≠{})≤{}⊐𝕩 ⋄ AnoTraceDead ⟨\"{}\", (+´m)⥊𝕨, m/𝕩⟩}}¨ ({}/{})",
                                    src, tids, mcol, mcol, sname, src, fib
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
    fn emit_effect(&mut self, ef: &Node, fx: &mut Fx) -> R<()> {
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
                // guards must refine the mask before gathering: pre-scan via world-mode guard
                // probe — the probe's staged lines are DISCARDED but its temps stay burned
                let saved_pre = std::mem::take(&mut self.pre);
                let probe_res = self.emit_val(rhs, Mode::World);
                self.pre = saved_pre;
                let probe = probe_res?;
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
                // Reindex for scatter. This assumes along is a permutation; duplicates and omissions are unchecked.
                if let Some(al) = &rhs_v.along {
                    rv = format!("((⍋{})⊏{})", al, rv);
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

    // N_EVERB/N_EVIA: raw text "<targetcol> <dfn>"; called as sel FnName ⟨target, args…⟩.
    fn emit_verb(&mut self, line: i32, name: Symbol, args: &[Node], fx: &mut Fx) -> R<()> {
        let n = self.rs(name);
        let e = self.find(n);
        let raw: Option<&'a str> = e.and_then(|i| {
            if let RegEntryKind::Fn { body: Some(b) } = &self.ent(i).kind { Some(b.as_str()) } else { None }
        });
        let (Some(ei), Some(raw)) = (e, raw) else {
            return Err(fail(line, format!("verb '{}' needs a registered fn with a target column", n)));
        };
        let target = verb_target(raw);
        let Some(ti) = self.find(&target) else {
            return Err(fail(line, format!("verb '{}' target column '{}' unregistered", n, target)));
        };
        if is_uniq(self.ent(ti)) {
            return Err(fail(line, format!("unique column '{}' is minted, never written", target)));
        }
        let mut args_s = format!("⟨{}", self.bqnv(ti));
        for a in args {
            let av = self.emit_val(a, Mode::World)?;
            args_s = format!("{}, {}", args_s, av.v);
        }
        args_s = format!("{}⟩", args_s);
        self.merge_base(fx, ti, self.bqnv(ti), b'v', None, line, n)?;
        let t = self.tv();
        let fv = self.fnv(ei);
        self.stage(format!("{} ← {} {} {}", t, self.sel_var, fv, args_s));
        self.add_commit(fx, ti, t, b'v', None);
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
            if !matches!(e.kind, RegEntryKind::Col { .. } | RegEntryKind::Rel { .. } | RegEntryKind::SRel { .. })
                && !is_field
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
                    self.stage(format!("{} ↩ {}", cur, base));
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
                match &app {
                    Some(a) => self.stage(format!("{} ↩ {}∾{}", cur, kept, a)),
                    None => self.stage(format!("{} ↩ {}", cur, kept)),
                }
            } else if committed {
                self.stage(format!("{} ↩ {}", cur, base));
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
            let Some(ci) = self.find("cursor") else {
                return Err(fail(st.line, "elided subject with no antecedent and no ^cursor alias"));
            };
            let cv = self.bqnv(ci);
            self.stage(format!("{} ← {}", sv, cv));
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
            NodeKind::Name(s) => {
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
                if let NodeKind::Name(s) = &k.kind {
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
    fn emit_query(&mut self, st: &Node) -> R<()> {
        let NodeKind::Query(inner) = &st.kind else {
            return Ok(());
        };
        self.stmt += 1;
        self.pre.clear();
        self.out.push_str(&format!("\n# q{}\n", self.stmt));
        self.set_frame(Some(inner));
        let v = self.emit_val(inner, Mode::World)?;
        let qv = format!("q{}", self.stmt);
        self.stage(format!("{} ← {}", qv, v.v));
        // --label: a 0x1D tag line names the query and its source line, then the display —
        // for every query, pinned or not, before any assertion
        if self.dirs.label {
            self.stage(format!("•Out (@+29)∾\"q{}@{}\"", self.stmt, st.line));
            self.stage(format!("•Show {}", qv));
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
            let mut lst = String::from("⟨");
            for (k, w) in vals.iter().enumerate() {
                let b0 = w.as_bytes().first().copied().unwrap_or(0);
                let piece = if b0.is_ascii_digit() || b0 == b'-' || b0 == b'.' {
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
            // numeric outs compare within 1e-9: pinned doubles come from a sibling BQN
            // evaluation whose association order may differ in the last bits
            if all_num {
                self.stage(format!(
                    "\"out q{}\" ! {} {{(≠𝕨)≠≠𝕩 ? 0 ; ∧´1e¯9≥|𝕨-𝕩}} ⥊{}",
                    self.stmt, lst, qv
                ));
            } else {
                self.stage(format!("\"out q{}\" ! {} ≡ ⥊{}", self.stmt, lst, qv));
            }
        } else if !self.dirs.label {
            self.stage(format!("•Show {}", qv));
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
                _ => {}
            }
        }
        // --trace: the debug observability prelude — 0x1F-prefixed diagnostic lines
        if self.dirs.trace {
            self.out.push_str("\n# trace (--trace): 0x1F-prefixed diagnostic lines\n");
            self.out.push_str("anoTraceSep ← @+31\n");
            self.out.push_str("AnoTraceNum ← {∾{𝕩='¯' ? \"-\" ; ⋈𝕩}¨•Repr 𝕩}\n");
            self.out.push_str("AnoTraceDead ← {n‿o‿s: o {•Out anoTraceSep∾\"RELATION \"∾n∾\" \"∾(AnoTraceNum 𝕨)∾\" -> \"∾(AnoTraceNum 𝕩)∾\" IS DEAD !\"}¨ s}\n");
            self.out.push_str("AnoTraceEmpty ← {n‿o: {•Out anoTraceSep∾\"FIBER \"∾n∾\" \"∾(AnoTraceNum 𝕩)∾\" IS EMPTY !\"}¨ o}\n");
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
            NodeKind::Name(s) => return self.scan_idx_name(*s, has_id, depth),
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
pub fn emit(prog: &Node, reg: &Registry, dirs: &Directives, it: &Interner) -> Result<String, Diag> {
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
    };
    let empty: [Node; 0] = [];
    let kids: &[Node] = if let NodeKind::Program(v) = &prog.kind { v } else { &empty };
    em.scan_need_idx(kids);
    em.emit_fixture();
    // Each run of new installations fires all installed rules once before the next performed
    // statement or EOF, using one shared barrier.
    let mut installed: Vec<&Node> = Vec::new();
    let mut fresh = false;
    for st in kids {
        let mut rule: Option<&Node> = None;
        match &st.kind {
            NodeKind::DefStmt { body, .. } => {
                if em.defs.len() >= 128 {
                    return Err(fail(st.line, "more than 128 defs"));
                }
                em.defs.push(st);
                if matches!(body.kind, NodeKind::Stmt { .. }) {
                    rule = Some(body);
                }
            }
            NodeKind::Stmt { rule: true, .. } => rule = Some(st),
            _ => {}
        }
        if let Some(r) = rule {
            if installed.len() >= 32 {
                return Err(Diag::refuse("emit: more than 32 installed rules"));
            }
            installed.push(r);
            fresh = true;
            continue;
        }
        if matches!(st.kind, NodeKind::DefStmt { .. }) {
            continue;
        }
        if fresh {
            em.emit_rule_tick(&installed)?;
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
        em.emit_rule_tick(&installed)?;
    }
    em.emit_expects()?;
    if dirs.save {
        em.emit_save();
    }
    Ok(em.out)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Oracle pin (src/anoc --emit on ":Foo" with the empty world): the emitted block after
    // the rt prelude, byte-exact. Sym queries touch no peer module, so this runs today.
    #[test]
    fn sym_query_matches_oracle() {
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

    // Oracle pin: --label replaces the plain display with the 0x1D tag + •Show.
    #[test]
    fn label_query_matches_oracle() {
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

    // Oracle pin: an --! out sym expectation pins the query exactly (≡ against ⥊q1).
    #[test]
    fn out_pin_matches_oracle() {
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
            "\n# fixture\nanoN ← 0\nanoSel ← ⟨⟩\n\n# q1\nq1 ← (<\"Foo\")\n\"out q1\" ! ⟨\"Foo\"⟩ ≡ ⥊q1\n\n# expectations\n\n\"ok\"\n"
        );
    }
}
