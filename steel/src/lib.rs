// Shared compiler types and contracts.
// Pipeline: main.rs (directives, driver) -> registry.rs (world fixtures) -> lex.rs (ASCII + JA
// skins) -> parse.rs (Pratt, 14 levels) -> emit.rs (BQN codegen); fs.rs serves main and registry;
// num.rs owns numeric parsing and spelling.

pub mod alias;
pub mod emit;
pub mod fs;
pub mod lex;
pub mod migration;
pub mod num;
pub mod parse;
pub mod reducer;
pub mod registry;
pub mod relationship;
pub mod trace;

pub const ANO_NAMESZ: usize = 256;

// 2^53, the largest bound for which every integer in [-bound, bound] is exactly representable as f64.
pub const ANO_NATMAX: f64 = 9_007_199_254_740_992.0;

/* ---------- diagnostics ---------- */

// One diagnostic: byte-exact message (the C snprintf output, verbatim, no trailing newline)
// plus the process exit code it carries. Every module refusal is code 2; main prints
// "<path>: <msg>\n" to stderr and exits with code.
#[derive(Debug, Clone)]
pub struct Diag {
    pub msg: String,
    pub code: i32,
}

impl Diag {
    // Inputs: byte-exact message. Output: a refusal — exit code 2, the module-error code.
    pub fn refuse(msg: impl Into<String>) -> Diag {
        Diag { msg: msg.into(), code: 2 }
    }
}

impl std::fmt::Display for Diag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.msg)
    }
}

/* ---------- interner ---------- */

// Interned spelling. Symbol equality replaces C's pointer equality on interned strings.
// Symbol::EMPTY is the interned "" — the "name is \"\" when absent, never NULL" invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol(pub u32);

impl Symbol {
    pub const EMPTY: Symbol = Symbol(0);
}

// Dedup pool: one canonical copy per distinct spelling, stable for the pool's lifetime.
// Iteration order never leaks into output (all observable orders are encounter order in Vecs).
#[derive(Debug, Clone)]
pub struct Interner {
    map: std::collections::HashMap<String, u32>,
    spellings: Vec<String>,
}

impl Interner {
    // Output: a pool with "" pre-interned as Symbol::EMPTY.
    pub fn new() -> Interner {
        let mut it = Interner { map: std::collections::HashMap::new(), spellings: Vec::new() };
        let e = it.intern("");
        debug_assert_eq!(e, Symbol::EMPTY);
        it
    }

    // Inputs: any spelling. Output: its Symbol; equal spellings return the same Symbol.
    pub fn intern(&mut self, s: &str) -> Symbol {
        if let Some(&i) = self.map.get(s) {
            return Symbol(i);
        }
        let i = self.spellings.len() as u32;
        self.spellings.push(s.to_string());
        self.map.insert(s.to_string(), i);
        Symbol(i)
    }

    // Inputs: a Symbol from this pool. Output: its canonical spelling.
    pub fn resolve(&self, sym: Symbol) -> &str {
        &self.spellings[sym.0 as usize]
    }
}

impl Default for Interner {
    fn default() -> Interner {
        Interner::new()
    }
}

/* ---------- tokens ---------- */

// Exact C declaration order (ano.h TokKind), T_KINDCOUNT dropped. K_TGT (the JA に marker)
// is lex.rs-internal and never appears here. Order itself is not printed anywhere; the
// spelling per kind (c_name) is what --tokens pins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokKind {
    Eof,
    Nl,
    // atoms
    Name,
    Alias,
    Sym,
    Num,
    Counter,
    Str,
    Wild,
    // structure
    Comma,
    Arrow,  // =>
    Semi,
    PipeGt, // |>
    Amp,
    Bar,
    Bang,
    EqEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    Plus,
    Minus,
    Star,
    Slash,
    Pct,
    At,
    Dot,
    Tick,
    Lp,
    Rp,
    Lb,
    Rb,
    LArrow, // <-
    Tilde,
    Fold,   // +/  (name column carries the op spelling)
    ScanOp, // +\  (name column carries the op spelling)
    Iota,   // til
    // keywords (closed set; GRAMMAR.md)
    Def,
    Undef,
    Spawn,
    AtKw, // at
    To,
    Via,
    Along,
    Order,
    By,
    Take,
    Desc,
    Top,
    Grade,
    FoldKw, // fold
    ScanKw, // scan
Cross,
    Expand,
}

impl TokKind {
    // Output: the C enum identifier spelling, byte-exact — main.c's tokname[] table,
    // printed by --tokens as "%s %s %g".
    pub fn c_name(self) -> &'static str {
        match self {
            TokKind::Eof => "T_EOF",
            TokKind::Nl => "T_NL",
            TokKind::Name => "T_NAME",
            TokKind::Alias => "T_ALIAS",
            TokKind::Sym => "T_SYM",
            TokKind::Num => "T_NUM",
            TokKind::Counter => "T_COUNTER",
            TokKind::Str => "T_STR",
            TokKind::Wild => "T_WILD",
            TokKind::Comma => "T_COMMA",
            TokKind::Arrow => "T_ARROW",
            TokKind::Semi => "T_SEMI",
            TokKind::PipeGt => "T_PIPEGT",
            TokKind::Amp => "T_AMP",
            TokKind::Bar => "T_BAR",
            TokKind::Bang => "T_BANG",
            TokKind::EqEq => "T_EQEQ",
            TokKind::Ne => "T_NE",
            TokKind::Lt => "T_LT",
            TokKind::Le => "T_LE",
            TokKind::Gt => "T_GT",
            TokKind::Ge => "T_GE",
            TokKind::Eq => "T_EQ",
            TokKind::PlusEq => "T_PLUSEQ",
            TokKind::MinusEq => "T_MINUSEQ",
            TokKind::StarEq => "T_STAREQ",
            TokKind::SlashEq => "T_SLASHEQ",
            TokKind::Plus => "T_PLUS",
            TokKind::Minus => "T_MINUS",
            TokKind::Star => "T_STAR",
            TokKind::Slash => "T_SLASH",
            TokKind::Pct => "T_PCT",
            TokKind::At => "T_AT",
            TokKind::Dot => "T_DOT",
            TokKind::Tick => "T_TICK",
            TokKind::Lp => "T_LP",
            TokKind::Rp => "T_RP",
            TokKind::Lb => "T_LB",
            TokKind::Rb => "T_RB",
            TokKind::LArrow => "T_LARROW",
            TokKind::Tilde => "T_TILDE",
            TokKind::Fold => "T_FOLD",
            TokKind::ScanOp => "T_SCANOP",
            TokKind::Iota => "T_IOTA",
            TokKind::Def => "T_DEF",
            TokKind::Undef => "T_UNDEF",
            TokKind::Spawn => "T_SPAWN",
            TokKind::AtKw => "T_ATKW",
            TokKind::To => "T_TO",
            TokKind::Via => "T_VIA",
            TokKind::Along => "T_ALONG",
            TokKind::Order => "T_ORDER",
            TokKind::By => "T_BY",
            TokKind::Take => "T_TAKE",
            TokKind::Desc => "T_DESC",
            TokKind::Top => "T_TOP",
            TokKind::Grade => "T_GRADE",
            TokKind::FoldKw => "T_FOLDKW",
            TokKind::ScanKw => "T_SCANKW",
            TokKind::Cross => "T_CROSS",
            TokKind::Expand => "T_EXPAND",
        }
    }
}

// The token stream, struct-of-arrays exactly as the C Toks: four index-aligned columns.
// name is Symbol::EMPTY when absent, never "missing". num carries NUM/COUNTER values, 0.0
// otherwise. line is the 1-based physical source line. The final token is always Eof
// (n INCLUDES it); Nl separates nonempty lines, never leads, doubles, or trails.
#[derive(Debug, Clone, Default)]
pub struct Toks {
    pub kind: Vec<TokKind>,
    pub name: Vec<Symbol>,
    pub num: Vec<f64>,
    pub line: Vec<i32>,
}

impl Toks {
    pub fn new() -> Toks {
        Toks::default()
    }

    pub fn len(&self) -> usize {
        self.kind.len()
    }

    pub fn is_empty(&self) -> bool {
        self.kind.is_empty()
    }

    // Appends one row across all four columns.
    pub fn push(&mut self, kind: TokKind, name: Symbol, num: f64, line: i32) {
        self.kind.push(kind);
        self.name.push(name);
        self.num.push(num);
        self.line.push(line);
    }
}

/* ---------- AST ---------- */

// N_CMP op codes: '<' '>' 'l'(<=) 'g'(>=) '=' '!'(!=). T_EQ and T_EQEQ both map to Eq.
// emitCompr's binder-filter table sends Le/Ge (C 'l'/'g') to ≠ — quirk, port faithfully.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOp {
    Lt, // '<'
    Gt, // '>'
    Le, // 'l'
    Ge, // 'g'
    Eq, // '='
    Ne, // '!'
}

// N_ARITH op codes: '+' '-' '*' '/' '%'.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

// N_EASSIGN op codes: '=' '+' '-' '*' '/'.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Set,
    Add,
    Sub,
    Mul,
    Div,
}

// One AST node: kind + the 1-based source line of its head token (diagnostics).
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub kind: NodeKind,
    pub line: i32,
}

impl Node {
    pub fn new(kind: NodeKind, line: i32) -> Node {
        Node { kind, line }
    }
}

// The one AST — full ADT of ano.h's NodeKind. C's nullable kid slots are Options; the char
// op codes are the enums above; flag bits are named bools scoped to their variants
// (F_DESC on Grade/OrderBy and F_RULE/F_CONT/F_ELIDED on Stmt).
// name payloads are Symbols (interned surface spellings; resolution happens at emit).
#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    // atoms
    Num(f64),
    Counter { val: f64, unit: Symbol }, // 3mo: num=value, name=unit
    Sym(Symbol),                        // :Name, sigil stripped by lexer
    Str(Symbol),                        // body, no escapes
    Name(Symbol),                       // surface spelling, resolved at emit
    // ^name dynamic-alias request, sigil stripped. `look` is the name lowering resolves (the
    // stem on bare fallback, the target on an overlay hit); `req` is always the requested stem,
    // so a refusal can still spell `^req` after the overlay moved the lookup elsewhere.
    Alias { look: Symbol, req: Symbol },
    Wild,                               // _
    // expressions
    Not(Box<Node>),
    And(Box<Node>, Box<Node>),
    Or(Box<Node>, Box<Node>),
    Cmp { op: CmpOp, l: Box<Node>, r: Box<Node> },
    CmpAny { name: Symbol }, // C N_CMP op '_', kids[0]=NAME — the presence-any tuple element (TwoHanded _)
    Arith { op: ArithOp, l: Box<Node>, r: Box<Node> },
    Scope { l: Box<Node>, r: Box<Node>, origin: Option<Box<Node>> }, // l @ r; origin = anchored-frame kids[2]
    Hop { l: Box<Node>, r: Box<Node> }, // l . r; r is Name, SetHop (tick rewrite), or the chain nests in l
    SetHop { rel: Symbol },             // name'; C kids[0] was the same N_NAME — the Symbol carries it
    Call { callee: Symbol, args: Vec<Node> },
    // op carries the SURFACE spelling until normalize resolves the head against the operand
    // carrier and rebinds it to the descriptor's canonical spelling (reducer::resolve_head);
    // @scope binds INSIDE operand as Scope. Count and average leave as prefix-machine rewrites.
    Fold { op: Symbol, operand: Box<Node> },
    ScanExpr { op: Symbol, operand: Box<Node> },
    ScanAlong { op: Symbol, col: Box<Node>, order: Box<Node> },
    IotaX(Box<Node>),  // til expr
    Shape(Vec<Node>),  // 1-2 dims, each Num or Wild
    Tuple(Vec<Node>),  // presence tuple or point literal; context decides at emit
    To { shape: Box<Node>, poured: Option<Box<Node>> }, // to shape; poured = board-literal Str
    Grade { key: Box<Node>, desc: bool },
    Top { k: f64, inner: Box<Node> },
    Pipe { src: Box<Node>, stages: Vec<Node> },
    OrderBy { key: Box<Node>, desc: bool },
    Take { k: f64 },
    Expand(Box<Node>),
    CrossV { f: Symbol, a: Box<Node>, b: Box<Node> },
    Binder { name: Symbol, source: Box<Node> }, // a <- Source
    // effects
    EAssign { op: AssignOp, target: Box<Node>, rhs: Box<Node> }, // target: Name or Hop (pos.x)
    EAdd(Symbol), // +Comp
    EDel(Symbol), // -Comp
    EDespawn,     // ~
    ESpawn { what: Box<Node>, count: Option<Box<Node>>, at: Option<Box<Node>> },
    EVerb { name: Symbol, args: Vec<Node> },
    EVia { f: Symbol, col: Box<Node> }, // fn via Col; col is a Name node
    // statements
    Stmt { sel: Option<Box<Node>>, effects: Vec<Node>, rule: bool, cont: bool, elided: bool },
    DefStmt { name: Symbol, body: Box<Node> }, // body may be Stmt{rule} for standing rules
    UndefStmt { name: Symbol }, // explicit standing-rule retraction at a barrier
    Query(Box<Node>),
    Compr { sel: Box<Node>, effect: Box<Node>, rest: Vec<Node> }, // rest: Binders and filter exprs in source order
    Program(Vec<Node>),
}

impl NodeKind {
    // Output: the C NodeKind enum value — the %d in "unsupported value node %d",
    // "unsupported effect %d", "unexpected top-level node %d". CmpAny is N_CMP (10).
    pub fn c_kind(&self) -> i32 {
        match self {
            NodeKind::Num(..) => 0,
            NodeKind::Counter { .. } => 1,
            NodeKind::Sym(..) => 2,
            NodeKind::Str(..) => 3,
            NodeKind::Name(..) => 4,
            NodeKind::Alias { .. } => 5,
            NodeKind::Wild => 6,
            NodeKind::Not(..) => 7,
            NodeKind::And(..) => 8,
            NodeKind::Or(..) => 9,
            NodeKind::Cmp { .. } => 10,
            NodeKind::CmpAny { .. } => 10,
            NodeKind::Arith { .. } => 11,
            NodeKind::Scope { .. } => 12,
            NodeKind::Hop { .. } => 13,
            NodeKind::SetHop { .. } => 14,
            NodeKind::Call { .. } => 15,
            NodeKind::Fold { .. } => 16,
            NodeKind::ScanExpr { .. } => 17,
            NodeKind::ScanAlong { .. } => 18,
            NodeKind::IotaX(..) => 19,
            NodeKind::Shape(..) => 20,
            NodeKind::Tuple(..) => 21,
            NodeKind::To { .. } => 22,
            NodeKind::Grade { .. } => 23,
            NodeKind::Top { .. } => 24,
            NodeKind::Pipe { .. } => 25,
            NodeKind::OrderBy { .. } => 26,
            NodeKind::Take { .. } => 27,
            NodeKind::Expand(..) => 28,
            NodeKind::CrossV { .. } => 29,
            NodeKind::Binder { .. } => 30,
            NodeKind::EAssign { .. } => 31,
            NodeKind::EAdd(..) => 32,
            NodeKind::EDel(..) => 33,
            NodeKind::EDespawn => 34,
            NodeKind::ESpawn { .. } => 35,
            NodeKind::EVerb { .. } => 36,
            NodeKind::EVia { .. } => 37,
            NodeKind::Stmt { .. } => 38,
            NodeKind::DefStmt { .. } => 39,
            NodeKind::UndefStmt { .. } => 43,
            NodeKind::Query(..) => 40,
            NodeKind::Compr { .. } => 41,
            NodeKind::Program(..) => 42,
        }
    }
}

/* ---------- registry ---------- */

// Column/field element type; on Tag, the carrier's type. No CT_VEC exists: a pair column
// is CT_NUM with nums.len() == 2*rows — the structural convention dump re-detects.
// Bool/Nat/Int are carrier refinements (ano-ecs.md §10): the loader seals data at rest,
// the barrier retracts every committed write — bool by 0<, nat/int by floor + clamp to
// ±ANO_NATMAX. Num is the untyped double and admits whatever.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColType {
    Num,
    Bool,
    Nat,
    Int,
    Sym,
    Char,
}

// RK_BIND kinds, C bindKind strings "entity" | "mask" | "point" | "num" | "vec".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindKind {
    Entity,
    Mask,
    Point,
    Num,
    Vec,
}

// The ~ reclamation policy. Registry.reap None = undeclared (default seal, dump omits the
// line); Some(Seal) was a written "reap seal" and dumps back. Storage policy only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reap {
    Seal,
    Host,
}

// One proto field pair: col = the column's CANONICAL name (resolved at load), spelling =
// the value word as written, num = the parsed numeric value (0.0 where the field is sym).
#[derive(Debug, Clone, PartialEq)]
pub struct ProtoField {
    pub col: String,
    pub spelling: String,
    pub num: f64,
}

// The nine RegEntry meanings as a real ADT. Value-level conventions preserved from C:
// -1.0 is the dangling-rel sentinel (data, never an Option); CT_CHAR stores the glyph run
// as syms[0] (exactly one element); CT_SYM values are exact bytes; keyOf/invOf "" sentinels
// are Options holding CANONICAL key spellings (keyOf) or the rel word AS WRITTEN (invOf, tag col).
#[derive(Debug, Clone, PartialEq)]
pub enum RegEntryKind {
    // Entity column. uniq: declared injectivity (unique line) — pairwise-distinct at load,
    // spawn mints, effects never assign. pres: optional presence mask, n values.
    // rng: the declared `range <col> <lo> <hi>` bounds — a value refinement the barrier
    // clamps writes into and the loader seals data at rest against (num-family only).
    Col { ty: ColType, uniq: bool, nums: Vec<f64>, syms: Vec<String>, pres: Option<Vec<f64>>, rng: Option<(f64, f64)> },
    // Lattice field, lat_w*lat_h values; no pres, no uniq (load refuses both).
    Field { ty: ColType, nums: Vec<f64>, syms: Vec<String>, rng: Option<(f64, f64)> },
    // Functional rel: n values, -1 dangling; key_of = declared unique key column (canonical
    // spelling) or None = keyed to the row index.
    Rel { targets: Vec<f64>, key_of: Option<String> },
    // Set-valued rel: exactly n fibers as written. inv_of = the rel word
    // as written when this is an inverse read (fibers recomputed at load, never dumped).
    SRel { fib: Vec<Vec<f64>>, inv_of: Option<String>, key_of: Option<String> },
    // A static alias mask: a stored mask VALUE (the `alias` line) — not a spelling alias (that
    // table is Registry.aliases) and not the dynamic ^name overlay.
    AliasMask { mask: Vec<f64> },
    // Named binding: entity/num 1 value, point 2, mask n, vec any count.
    Bind { kind: BindKind, vals: Vec<f64> },
    // Registered fn; body = the raw .reg line tail verbatim (spaces preserved), None when bodyless.
    Fn { body: Option<String> },
    // Derived tag: the equality mask over the live carrier column, recomputed at each use.
    // col = the carrier word AS WRITTEN; carrier_ty = the carrier's type; sym value exact bytes.
    Tag { col: String, carrier_ty: ColType, num: f64, sym: Option<String> },
    // Registered archetype (the .reg `def` line): spawn fill layer one.
    Proto { fields: Vec<ProtoField> },
}

// One registry entry. defval lives HERE, not inside Col: the C `default` line has no kind
// check — a default on a fn or bind is stored and dumped (defval != 0.0 dumps a line).
#[derive(Debug, Clone, PartialEq)]
pub struct RegEntry {
    pub name: String,
    pub defval: f64,
    pub kind: RegEntryKind,
}

// One row of the spelling-alias table (`as`/`ja` two-word lines): surface word -> entry name,
// target unvalidated free text; ja keeps the declared spelling so dumps round-trip.
#[derive(Debug, Clone, PartialEq)]
pub struct AliasRow {
    pub from: String,
    pub to: String,
    pub ja: bool,
}

// The loaded world. Entries in DECLARATION ORDER — order is normative (reg_dump emits it,
// bqnv mangles jp<i> by index, emit iterates it); never a map. roles capped at 8 by load;
// duplicates accepted, first wins at reg_role.
#[derive(Debug, Clone, Default)]
pub struct Registry {
    pub n: i32,
    pub lat_w: i32,
    pub lat_h: i32,
    pub ents: Vec<RegEntry>,
    pub aliases: Vec<AliasRow>, // the spelling-alias table (`as`/`ja`), not the dynamic ^name overlay
    pub roles: Vec<(String, String)>, // (role word, col word) as written
    pub reap: Option<Reap>,
}

/* ---------- directives (main.rs owns parsing them) ---------- */

// An expectation: a column pin or (Out) an ordered query-output pin (--! out).
// Value words stay RAW — symbols stay words, numbers stay spellings; emit compares raw.
#[derive(Debug, Clone, PartialEq)]
pub enum Expect {
    Col { col: String, vals: Vec<String> },
    Out { vals: Vec<String> },
}

// C sentinels kept where the C semantics need them: registry/same_tokens "" = none
// (main reproduces the snprintf truncation at 255/511 bytes); expect_n -1 = unchecked
// (a directive-parsed -1 is also unchecked — emitExpects tests >= 0).
// Master invariant: with save/label/trace off, the emitted BQN is byte-identical to a
// flagless steel — the flags gate whole appended blocks, never restructure shared output.
#[derive(Debug, Clone)]
pub struct Directives {
    pub registry: String,
    pub aliases: String, // --aliases: the dynamic-alias overlay sidecar path ("" = none)
    pub expects: Vec<Expect>,
    pub expect_n: i32,
    pub ja: bool,
    pub save: bool,
    pub label: bool,
    pub trace: bool,
    pub same_tokens: String,
}

impl Default for Directives {
    fn default() -> Directives {
        Directives {
            registry: String::new(),
            aliases: String::new(),
            expects: Vec::new(),
            expect_n: -1,
            ja: false,
            save: false,
            label: false,
            trace: false,
            same_tokens: String::new(),
        }
    }
}
