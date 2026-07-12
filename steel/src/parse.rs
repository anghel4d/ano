// parse.rs — the Pratt parser, GRAMMAR.md's 14 levels, one grammar, one AST, surface-blind.
// Mirrors src/parse.c: the hinge split, selection/effect forms, comprehensions, defs,
// continuations, queries, the eval splice. Diagnostics "line %d: %s"; FIRST error wins
// (later errors dropped silently — model err as a set-once Option).

use crate::{ArithOp, AssignOp, CmpOp, Diag, Interner, Node, NodeKind, Symbol, TokKind, Toks};

/* ---------- diagnostics ---------- */

// Inputs: 1-based source line, message tail. Output: the refusal "line %d: %s", no newline.
fn perr(line: i32, msg: impl std::fmt::Display) -> Diag {
    Diag::refuse(format!("line {line}: {msg}"))
}

/* ---------- token classes ---------- */

// Input: token kind. Output: binary precedence level per GRAMMAR.md, 0 = not binary.
fn binlevel(k: TokKind) -> i32 {
    use TokKind::*;
    match k {
        PipeGt => 3,
        Bar => 5,
        Amp => 6,
        EqEq | Ne | Lt | Le | Gt | Ge | Eq => 8,
        Plus | Minus => 10,
        Star | Slash | Pct => 11,
        At => 12,
        Dot | Tick => 13,
        _ => 0,
    }
}

// Input: comparison token. Output: Cmp op; T_EQ and T_EQEQ both map to Eq.
fn cmpop(k: TokKind) -> Option<CmpOp> {
    match k {
        TokKind::EqEq | TokKind::Eq => Some(CmpOp::Eq),
        TokKind::Ne => Some(CmpOp::Ne),
        TokKind::Lt => Some(CmpOp::Lt),
        TokKind::Le => Some(CmpOp::Le),
        TokKind::Gt => Some(CmpOp::Gt),
        TokKind::Ge => Some(CmpOp::Ge),
        _ => None,
    }
}

// Input: arithmetic token. Output: Arith op.
fn arithop(k: TokKind) -> Option<ArithOp> {
    match k {
        TokKind::Plus => Some(ArithOp::Add),
        TokKind::Minus => Some(ArithOp::Sub),
        TokKind::Star => Some(ArithOp::Mul),
        TokKind::Slash => Some(ArithOp::Div),
        TokKind::Pct => Some(ArithOp::Mod),
        _ => None,
    }
}

// Input: effect assignment token. Output: EAssign op, None = not one.
fn assignop(k: TokKind) -> Option<AssignOp> {
    match k {
        TokKind::Eq => Some(AssignOp::Set),
        TokKind::PlusEq => Some(AssignOp::Add),
        TokKind::MinusEq => Some(AssignOp::Sub),
        TokKind::StarEq => Some(AssignOp::Mul),
        TokKind::SlashEq => Some(AssignOp::Div),
        _ => None,
    }
}

// Input: token kind. Output: true when it can begin an expression atom.
fn atomstart(k: TokKind) -> bool {
    use TokKind::*;
    matches!(k, Name | Alias | Sym | Num | Counter | Str | Wild | Lp | Iota)
}

/* ---------- parser state ---------- */

// Cursor over a preprocessed stream (final token guaranteed Eof). depth bounds parse_expr
// nesting so degenerate input errors, never overflows; eval sub-parsers get a fresh budget.
struct P<'t, 'i> {
    t: &'t Toks,
    i: usize,
    depth: i32,
    it: &'i mut Interner,
}

impl P<'_, '_> {
    fn pk(&self) -> TokKind {
        self.t.kind[self.i]
    }

    fn pk2(&self, k: usize) -> TokKind {
        let j = self.i + k;
        if j < self.t.len() { self.t.kind[j] } else { TokKind::Eof }
    }

    fn tname(&self) -> Symbol {
        self.t.name[self.i]
    }

    fn tnum(&self) -> f64 {
        self.t.num[self.i]
    }

    fn tline(&self) -> i32 {
        self.t.line[self.i]
    }

    fn adv(&mut self) {
        if self.i + 1 < self.t.len() {
            self.i += 1;
        }
    }

    // Inputs: expected kind, description. Output: consumed / Diag "expected %s" at the
    // CURRENT token's line.
    fn expect(&mut self, k: TokKind, what: &str) -> Result<(), Diag> {
        if self.pk() != k {
            return Err(perr(self.tline(), format!("expected {what}")));
        }
        self.adv();
        Ok(())
    }

    /* ---------- expressions ---------- */

    // Input: cursor at T_NAME. Output: Name node, token consumed.
    fn mkname(&mut self) -> Node {
        let n = Node::new(NodeKind::Name(self.tname()), self.tline());
        self.adv();
        n
    }

    // Input: cursor at NUM (after @ or after to). Output: Shape of 1-2 dims.
    // allow_wild permits Wild dims (the `to 4 _` form).
    fn parse_shape(&mut self, allow_wild: bool) -> Result<Node, Diag> {
        let line = self.tline();
        let mut dims = Vec::new();
        for d in 0..2 {
            if self.pk() == TokKind::Num {
                dims.push(Node::new(NodeKind::Num(self.tnum()), self.tline()));
                self.adv();
            } else if allow_wild && self.pk() == TokKind::Wild {
                dims.push(Node::new(NodeKind::Wild, self.tline()));
                self.adv();
            } else if d == 0 {
                return Err(perr(self.tline(), "expected shape dimension"));
            } else {
                break;
            }
        }
        Ok(Node::new(NodeKind::Shape(dims), line))
    }

    // Input: cursor at T_TO. Output: To with shape (dims Num or Wild), poured None.
    fn parse_to(&mut self) -> Result<Node, Diag> {
        let line = self.tline();
        self.adv();
        let sh = self.parse_shape(true)?;
        Ok(Node::new(NodeKind::To { shape: Box::new(sh), poured: None }, line))
    }

    // Input: cursor at T_NAME with '(' next. Output: Call, args comma-separated at min 5.
    fn parse_call(&mut self) -> Result<Node, Diag> {
        let line = self.tline();
        let callee = self.tname();
        self.adv(); // name
        self.adv(); // (
        let mut args = Vec::new();
        if self.pk() != TokKind::Rp {
            loop {
                args.push(self.parse_expr(5)?);
                if self.pk() == TokKind::Comma {
                    self.adv();
                    continue;
                }
                break;
            }
        }
        self.expect(TokKind::Rp, "')' after call arguments")?;
        Ok(Node::new(NodeKind::Call { callee, args }, line))
    }

    // Inputs: min level. Output: one tuple element: expr, with the presence-any form
    // `NAME _` wrapped as CmpAny.
    fn parse_tupelem(&mut self, min: i32) -> Result<Node, Diag> {
        let e = self.parse_expr(min)?;
        if let NodeKind::Name(name) = e.kind {
            if self.pk() == TokKind::Wild {
                let c = Node::new(NodeKind::CmpAny { name }, e.line);
                self.adv();
                return Ok(c);
            }
        }
        Ok(e)
    }

    // Input: cursor at T_LP. Output: parenthesized expr, Tuple on top-level comma, or
    // juxtaposed Call when a bare name is followed by atoms ((pieceOf char)).
    fn parse_paren(&mut self) -> Result<Node, Diag> {
        let line = self.tline();
        self.adv();
        let mut e = self.parse_tupelem(2)?;
        if let NodeKind::Name(callee) = e.kind {
            if atomstart(self.pk()) {
                // juxtaposed call
                let eline = e.line;
                let mut args = Vec::new();
                while atomstart(self.pk()) {
                    args.push(self.parse_expr(10)?);
                }
                e = Node::new(NodeKind::Call { callee, args }, eline);
            }
        }
        if self.pk() == TokKind::Comma {
            // tuple
            let mut els = vec![e];
            while self.pk() == TokKind::Comma {
                self.adv();
                els.push(self.parse_tupelem(5)?);
            }
            e = Node::new(NodeKind::Tuple(els), line);
        }
        self.expect(TokKind::Rp, "')'")?;
        Ok(e)
    }

    // Input: cursor at level-14 position. Output: atom node.
    fn parse_atom(&mut self) -> Result<Node, Diag> {
        let line = self.tline();
        match self.pk() {
            TokKind::Num => {
                let n = Node::new(NodeKind::Num(self.tnum()), line);
                self.adv();
                Ok(n)
            }
            TokKind::Counter => {
                let n = Node::new(NodeKind::Counter { val: self.tnum(), unit: self.tname() }, line);
                self.adv();
                Ok(n)
            }
            TokKind::Sym => {
                let n = Node::new(NodeKind::Sym(self.tname()), line);
                self.adv();
                Ok(n)
            }
            TokKind::Str => {
                let n = Node::new(NodeKind::Str(self.tname()), line);
                self.adv();
                Ok(n)
            }
            TokKind::Alias => {
                let n = Node::new(NodeKind::Alias(self.tname()), line);
                self.adv();
                Ok(n)
            }
            TokKind::Wild => {
                self.adv();
                Ok(Node::new(NodeKind::Wild, line))
            }
            TokKind::Name => {
                if self.pk2(1) == TokKind::Lp {
                    return self.parse_call();
                }
                Ok(self.mkname())
            }
            TokKind::Iota => {
                self.adv();
                let k = self.parse_expr(12)?; // just the atom-ish operand
                Ok(Node::new(NodeKind::IotaX(Box::new(k)), line))
            }
            TokKind::Lp => self.parse_paren(),
            _ => Err(perr(line, "unexpected token in expression")),
        }
    }

    // Input: cursor at an order-by head. Output: OrderBy with key, desc on trailing desc.
    fn parse_orderby(&mut self) -> Result<Node, Diag> {
        let line = self.tline();
        self.adv();
        self.expect(TokKind::By, "'by' after 'order'")?;
        let key = self.parse_expr(5)?;
        let mut desc = false;
        if self.pk() == TokKind::Desc {
            self.adv();
            desc = true;
        }
        Ok(Node::new(NodeKind::OrderBy { key: Box::new(key), desc }, line))
    }

    // Input: cursor at a pipeline stage head. Output: stage node
    // (OrderBy | Take | Expand | juxtaposed Call).
    fn parse_stage(&mut self) -> Result<Node, Diag> {
        let line = self.tline();
        match self.pk() {
            TokKind::Order => self.parse_orderby(),
            TokKind::Take => {
                self.adv();
                if self.pk() != TokKind::Num {
                    return Err(perr(self.tline(), "expected count after 'take'"));
                }
                let n = Node::new(NodeKind::Take { k: self.tnum() }, line);
                self.adv();
                Ok(n)
            }
            TokKind::Expand => {
                self.adv();
                let col = self.parse_expr(5)?;
                Ok(Node::new(NodeKind::Expand(Box::new(col)), line))
            }
            TokKind::Name => {
                let callee = self.tname();
                self.adv();
                let mut args = Vec::new();
                while atomstart(self.pk()) {
                    args.push(self.parse_expr(10)?);
                }
                Ok(Node::new(NodeKind::Call { callee, args }, line))
            }
            _ => Err(perr(line, "expected pipeline stage")),
        }
    }

    // Input: cursor inside scan/scan2 parens. Output: op spelling or reducer name.
    fn parse_opname(&mut self) -> Result<Symbol, Diag> {
        let op = match self.pk() {
            TokKind::Plus => self.it.intern("+"),
            TokKind::Minus => self.it.intern("-"),
            TokKind::Star => self.it.intern("*"),
            TokKind::Amp => self.it.intern("&"),
            TokKind::Bar => self.it.intern("|"),
            TokKind::Name => self.tname(),
            _ => return Err(perr(self.tline(), "expected operator or reducer name")),
        };
        self.adv();
        Ok(op)
    }

    // Inputs: min level. Output: prefix construct (! at 7; fold/scan/grade/top/fold(f)/
    // scan-along/scan2/cross at 9; order-by head at 3) or an atom. Fold-family operands
    // parse at min 10 so @ (12) and . (13) fall inside the operand.
    fn parse_prefix(&mut self, min: i32) -> Result<Node, Diag> {
        let line = self.tline();
        let k = self.pk();
        if k == TokKind::Bang && min <= 7 {
            self.adv();
            let x = self.parse_expr(8)?;
            return Ok(Node::new(NodeKind::Not(Box::new(x)), line));
        }
        if min <= 9 {
            match k {
                TokKind::Fold | TokKind::ScanOp => {
                    let op = self.tname();
                    self.adv();
                    let x = self.parse_expr(10)?;
                    let kind = if k == TokKind::Fold {
                        NodeKind::Fold { op, operand: Box::new(x) }
                    } else {
                        NodeKind::ScanExpr { op, operand: Box::new(x), scan2: false }
                    };
                    return Ok(Node::new(kind, line));
                }
                TokKind::Grade => {
                    self.adv();
                    let mut desc = false;
                    if self.pk() == TokKind::Desc {
                        self.adv();
                        desc = true;
                    }
                    let key = self.parse_expr(10)?;
                    return Ok(Node::new(NodeKind::Grade { key: Box::new(key), desc }, line));
                }
                TokKind::Top => {
                    self.adv();
                    if self.pk() != TokKind::Num {
                        return Err(perr(self.tline(), "expected count after 'top'"));
                    }
                    let kk = self.tnum();
                    self.adv();
                    let x = self.parse_expr(10)?;
                    return Ok(Node::new(NodeKind::Top { k: kk, inner: Box::new(x) }, line));
                }
                TokKind::FoldKw => {
                    // fold(f): the long form of f/ — one node, Fold
                    self.adv();
                    self.expect(TokKind::Lp, "'(' after 'fold'")?;
                    if self.pk() != TokKind::Name {
                        return Err(perr(self.tline(), "expected reducer name"));
                    }
                    let op = self.tname();
                    self.adv();
                    self.expect(TokKind::Rp, "')' after reducer")?;
                    let x = self.parse_expr(10)?;
                    return Ok(Node::new(NodeKind::Fold { op, operand: Box::new(x) }, line));
                }
                TokKind::ScanKw => {
                    self.adv();
                    self.expect(TokKind::Lp, "'(' after 'scan'")?;
                    let op = self.parse_opname()?;
                    self.expect(TokKind::Rp, "')' after scan operator")?;
                    let col = self.parse_expr(10)?;
                    self.expect(TokKind::Along, "'along' in scan")?;
                    let ord = self.parse_expr(10)?;
                    return Ok(Node::new(
                        NodeKind::ScanAlong { op, col: Box::new(col), order: Box::new(ord) },
                        line,
                    ));
                }
                TokKind::Scan2 => {
                    self.adv();
                    self.expect(TokKind::Lp, "'(' after 'scan2'")?;
                    let op = self.parse_opname()?;
                    self.expect(TokKind::Rp, "')' after scan2 operator")?;
                    let x = self.parse_expr(10)?;
                    return Ok(Node::new(
                        NodeKind::ScanExpr { op, operand: Box::new(x), scan2: true },
                        line,
                    ));
                }
                TokKind::Cross => {
                    self.adv();
                    if self.pk() != TokKind::Name {
                        return Err(perr(self.tline(), "expected function after 'cross'"));
                    }
                    let f = self.tname();
                    self.adv();
                    let x = self.parse_expr(10)?;
                    let y = self.parse_expr(10)?;
                    return Ok(Node::new(
                        NodeKind::CrossV { f, a: Box::new(x), b: Box::new(y) },
                        line,
                    ));
                }
                _ => {}
            }
        }
        if k == TokKind::Order && min <= 3 {
            return self.parse_orderby(); // src-less pipeline head
        }
        self.parse_atom()
    }

    // Inputs: parsed left operand, min level. Output: expression climbed from l:
    // left-assoc binaries at their level, . and ' postfix at 13, |> pipeline at 3,
    // @-then-NUM shape scope.
    fn parse_binloop(&mut self, mut l: Node, min: i32) -> Result<Node, Diag> {
        loop {
            let k = self.pk();
            let lv = binlevel(k);
            if lv == 0 || lv < min {
                return Ok(l);
            }
            let line = self.tline();
            if k == TokKind::Dot {
                self.adv();
                if self.pk() != TokKind::Name {
                    return Err(perr(self.tline(), "expected name after '.'"));
                }
                let r = self.mkname();
                l = Node::new(NodeKind::Hop { l: Box::new(l), r: Box::new(r) }, line);
                continue;
            }
            if k == TokKind::Tick {
                self.adv();
                let lline = l.line;
                l = match l.kind {
                    NodeKind::Name(rel) => Node::new(NodeKind::SetHop { rel }, lline),
                    NodeKind::Hop { l: hl, r } if matches!(r.kind, NodeKind::Name(_)) => {
                        let NodeKind::Name(rel) = r.kind else { unreachable!() };
                        let s = Node::new(NodeKind::SetHop { rel }, r.line);
                        Node::new(NodeKind::Hop { l: hl, r: Box::new(s) }, lline)
                    }
                    _ => return Err(perr(line, "tick after non-name")),
                };
                continue;
            }
            if k == TokKind::PipeGt {
                let src = Box::new(l);
                let mut stages = Vec::new();
                while self.pk() == TokKind::PipeGt {
                    self.adv();
                    stages.push(self.parse_stage()?);
                }
                l = Node::new(NodeKind::Pipe { src, stages }, line);
                continue;
            }
            self.adv();
            let r = if k == TokKind::At && self.pk() == TokKind::Num {
                self.parse_shape(false)? // @ 64 64
            } else {
                self.parse_expr(lv + 1)?
            };
            let mut n = if k == TokKind::Bar {
                Node::new(NodeKind::Or(Box::new(l), Box::new(r)), line)
            } else if k == TokKind::Amp {
                Node::new(NodeKind::And(Box::new(l), Box::new(r)), line)
            } else if k == TokKind::At {
                Node::new(NodeKind::Scope { l: Box::new(l), r: Box::new(r), origin: None }, line)
            } else if let Some(op) = cmpop(k) {
                Node::new(NodeKind::Cmp { op, l: Box::new(l), r: Box::new(r) }, line)
            } else {
                let op = arithop(k).unwrap(); // binlevel gates: only + - * / % reach here
                Node::new(NodeKind::Arith { op, l: Box::new(l), r: Box::new(r) }, line)
            };
            if k == TokKind::At && self.pk() == TokKind::AtKw {
                // mask @ frame at origin: anchored frame
                self.adv();
                let org = self.parse_expr(10)?;
                if let NodeKind::Scope { origin, .. } = &mut n.kind {
                    *origin = Some(Box::new(org));
                }
            }
            l = n;
        }
    }

    // Inputs: min level. Output: full expression at that level.
    // Invariant: depth-capped, so a degenerate paren tower is a parse error, never
    // stack exhaustion.
    fn parse_expr(&mut self, min: i32) -> Result<Node, Diag> {
        if self.depth >= 4096 {
            return Err(perr(self.tline(), "expression nested too deeply"));
        }
        self.depth += 1;
        let r = self.parse_prefix(min).and_then(|l| self.parse_binloop(l, min));
        self.depth -= 1;
        r
    }

    /* ---------- effects ---------- */

    // Input: cursor at an effect head. Output: one effect node per GRAMMAR.md level 4.
    fn parse_effect(&mut self) -> Result<Node, Diag> {
        let line = self.tline();
        match self.pk() {
            TokKind::Plus | TokKind::Minus => {
                let add = self.pk() == TokKind::Plus;
                self.adv();
                if self.pk() != TokKind::Name {
                    return Err(perr(self.tline(), "expected component name"));
                }
                let name = self.tname();
                self.adv();
                let kind = if add { NodeKind::EAdd(name) } else { NodeKind::EDel(name) };
                Ok(Node::new(kind, line))
            }
            TokKind::Tilde => {
                self.adv();
                Ok(Node::new(NodeKind::EDespawn, line))
            }
            TokKind::Spawn => {
                self.adv();
                let what = if self.pk() == TokKind::Name {
                    if self.pk2(1) == TokKind::Lp { self.parse_call()? } else { self.mkname() }
                } else if self.pk() == TokKind::Lp {
                    self.parse_paren()?
                } else {
                    return Err(perr(self.tline(), "expected prototype after 'spawn'"));
                };
                let mut count = None;
                let mut pos = None;
                if self.pk() == TokKind::Star {
                    self.adv();
                    count = Some(Box::new(self.parse_expr(10)?));
                }
                if self.pk() == TokKind::AtKw {
                    self.adv();
                    pos = Some(Box::new(self.parse_expr(10)?));
                }
                Ok(Node::new(NodeKind::ESpawn { what: Box::new(what), count, at: pos }, line))
            }
            TokKind::Name => {
                let mut tgt = self.mkname();
                if self.pk() == TokKind::Via {
                    // fn via Col
                    self.adv();
                    if self.pk() != TokKind::Name {
                        return Err(perr(self.tline(), "expected relation after 'via'"));
                    }
                    let NodeKind::Name(f) = tgt.kind else { unreachable!() };
                    let col = self.mkname();
                    return Ok(Node::new(NodeKind::EVia { f, col: Box::new(col) }, line));
                }
                while self.pk() == TokKind::Dot {
                    // pos.x target chain
                    self.adv();
                    if self.pk() != TokKind::Name {
                        return Err(perr(self.tline(), "expected field after '.'"));
                    }
                    let r = self.mkname();
                    tgt = Node::new(NodeKind::Hop { l: Box::new(tgt), r: Box::new(r) }, line);
                }
                if let Some(op) = assignop(self.pk()) {
                    self.adv();
                    let rhs = if self.pk() == TokKind::To {
                        self.parse_to()?
                    } else {
                        self.parse_expr(4)?
                    };
                    return Ok(Node::new(
                        NodeKind::EAssign { op, target: Box::new(tgt), rhs: Box::new(rhs) },
                        line,
                    ));
                }
                if let NodeKind::Name(name) = tgt.kind {
                    // registered verb
                    let mut args = Vec::new();
                    while atomstart(self.pk()) {
                        args.push(self.parse_expr(10)?);
                    }
                    return Ok(Node::new(NodeKind::EVerb { name, args }, line));
                }
                Err(perr(self.tline(), "expected assignment after target"))
            }
            _ => Err(perr(line, "expected effect")),
        }
    }

    // Inputs: effect list. Output: ';'-separated effects appended, at least one.
    fn parse_effects(&mut self, effects: &mut Vec<Node>) -> Result<(), Diag> {
        loop {
            effects.push(self.parse_effect()?);
            if self.pk() == TokKind::Semi {
                self.adv();
                continue;
            }
            return Ok(());
        }
    }

    /* ---------- statements ---------- */

    // Input: cursor at T_LB. Output: Compr: sel, effect, then binders and filters.
    fn parse_compr(&mut self) -> Result<Node, Diag> {
        let line = self.tline();
        self.adv();
        let sel = self.parse_expr(5)?;
        self.expect(TokKind::Comma, "',' before comprehension effect")?;
        let eff = self.parse_effect()?;
        self.expect(TokKind::Bar, "'|' before comprehension binders")?;
        let mut rest = Vec::new();
        loop {
            if self.pk() == TokKind::Name && self.pk2(1) == TokKind::LArrow {
                let bline = self.tline();
                let name = self.tname();
                self.adv();
                self.adv();
                let src = self.parse_expr(5)?;
                rest.push(Node::new(NodeKind::Binder { name, source: Box::new(src) }, bline));
            } else {
                rest.push(self.parse_expr(5)?);
            }
            if self.pk() == TokKind::Comma {
                self.adv();
                continue;
            }
            break;
        }
        self.expect(TokKind::Rb, "']'")?;
        Ok(Node::new(NodeKind::Compr { sel: Box::new(sel), effect: Box::new(eff), rest }, line))
    }

    // Input: cursor at the first token of a line (never Nl/Eof). Output: one statement:
    // DefStmt | Stmt (rule/cont/elided) | Query | Compr.
    fn parse_stmt(&mut self) -> Result<Node, Diag> {
        let line = self.tline();
        let k = self.pk();
        if k == TokKind::Def {
            self.adv();
            if self.pk() != TokKind::Name {
                return Err(perr(self.tline(), "expected name after 'def'"));
            }
            // the closed grammar outranks all names, defs included: a def named for a reserved
            // word is a name the lexer resolves first, unreachable on the JA surface where it is
            // the numeral/particle — bar it on both, the §2 law applied past the loader
            let name = self.tname();
            if crate::lex::lex_reserved(self.it.resolve(name)) {
                return Err(perr(
                    self.tline(),
                    format!("'{}' is lexer-reserved and cannot name a def", self.it.resolve(name)),
                ));
            }
            self.adv();
            self.expect(TokKind::Eq, "'=' after def name")?;
            let mut body = self.parse_expr(2)?;
            if self.pk() == TokKind::Arrow {
                // def name = sel => effects
                self.adv();
                let mut effects = Vec::new();
                self.parse_effects(&mut effects)?;
                body = Node::new(
                    NodeKind::Stmt {
                        sel: Some(Box::new(body)),
                        effects,
                        rule: true,
                        cont: false,
                        elided: false,
                    },
                    line,
                );
            }
            return Ok(Node::new(NodeKind::DefStmt { name, body: Box::new(body) }, line));
        }
        if k == TokKind::Tilde && matches!(self.pk2(1), TokKind::Nl | TokKind::Eof) {
            // lone ~
            self.adv();
            return Ok(Node::new(
                NodeKind::Stmt {
                    sel: None,
                    effects: vec![Node::new(NodeKind::EDespawn, line)],
                    rule: false,
                    cont: true,
                    elided: false,
                },
                line,
            ));
        }
        if k == TokKind::Comma {
            // leading-comma continuation
            self.adv();
            let mut effects = Vec::new();
            self.parse_effects(&mut effects)?;
            return Ok(Node::new(
                NodeKind::Stmt { sel: None, effects, rule: false, cont: true, elided: false },
                line,
            ));
        }
        if k == TokKind::Lb {
            return self.parse_compr();
        }
        // eval "<statement>" — APL's ⍎ constrained to a literal: the quotation is re-lexed and
        // spliced HERE, at parse time, so the spliced statement's footprint stays visible to
        // every later static check. One statement per quotation; dynamic strings are not this.
        if k == TokKind::Name
            && self.it.resolve(self.tname()) == "eval"
            && self.pk2(1) == TokKind::Str
            && matches!(self.pk2(2), TokKind::Nl | TokKind::Eof)
        {
            let quoted = self.it.resolve(self.t.name[self.i + 1]).to_string();
            self.adv();
            self.adv();
            let ts = crate::lex::lex(quoted.as_bytes(), false, self.it)?;
            let mut q = P { t: &ts, i: 0, depth: 0, it: &mut *self.it };
            while q.pk() == TokKind::Nl {
                q.adv();
            }
            if q.pk() == TokKind::Eof {
                return Err(perr(line, "eval of an empty quotation"));
            }
            let s = q.parse_stmt()?;
            while q.pk() == TokKind::Nl {
                q.adv();
            }
            if q.pk() != TokKind::Eof {
                return Err(perr(line, "eval: one statement per quotation"));
            }
            return Ok(s);
        }
        if k == TokKind::Spawn
            || ((k == TokKind::Plus || k == TokKind::Minus)
                && self.pk2(1) == TokKind::Name
                && matches!(self.pk2(2), TokKind::Nl | TokKind::Semi | TokKind::Eof))
        {
            // elided subject
            let mut effects = Vec::new();
            self.parse_effects(&mut effects)?;
            return Ok(Node::new(
                NodeKind::Stmt { sel: None, effects, rule: false, cont: false, elided: true },
                line,
            ));
        }
        // source-position specials, then the selection expression
        let mut sel: Option<Node> = None;
        if k == TokKind::Str && self.pk2(1) == TokKind::To {
            // "glyphs" to 8 8
            let s = Node::new(NodeKind::Str(self.tname()), line);
            self.adv();
            let mut to = self.parse_to()?;
            if let NodeKind::To { poured, .. } = &mut to.kind {
                *poured = Some(Box::new(s));
            }
            sel = Some(to);
        } else if k == TokKind::Num && self.pk2(1) == TokKind::Num {
            // 8 8 lattice
            sel = Some(self.parse_shape(false)?);
        } else if k == TokKind::Num
            && matches!(self.pk2(1), TokKind::Comma | TokKind::Amp | TokKind::Arrow)
        {
            sel = Some(self.parse_shape(false)?); // 12 line
        }
        let sel = match sel {
            Some(s) => self.parse_binloop(s, 2)?,
            None => self.parse_expr(2)?,
        };
        if self.pk() == TokKind::Comma || self.pk() == TokKind::Arrow {
            let rule = self.pk() == TokKind::Arrow;
            self.adv();
            let mut effects = Vec::new();
            self.parse_effects(&mut effects)?;
            return Ok(Node::new(
                NodeKind::Stmt { sel: Some(Box::new(sel)), effects, rule, cont: false, elided: false },
                line,
            ));
        }
        if matches!(self.pk(), TokKind::Nl | TokKind::Eof) {
            return Ok(Node::new(NodeKind::Query(Box::new(sel)), line));
        }
        if let NodeKind::Name(name) = sel.kind {
            if atomstart(self.pk()) {
                // juxtaposed verb: elided EVerb
                let vline = sel.line;
                let mut args = Vec::new();
                while atomstart(self.pk()) {
                    args.push(self.parse_expr(10)?);
                }
                let mut effects = vec![Node::new(NodeKind::EVerb { name, args }, vline)];
                while self.pk() == TokKind::Semi {
                    self.adv();
                    effects.push(self.parse_effect()?);
                }
                return Ok(Node::new(
                    NodeKind::Stmt { sel: None, effects, rule: false, cont: false, elided: true },
                    line,
                ));
            }
        }
        Err(perr(self.tline(), "unexpected token after selection"))
    }
}

/* ---------- entry ---------- */

// Inputs: token stream (final token guaranteed Eof), interner (the eval splice re-lexes the
// quotation via lex::lex with ja = false and a FRESH depth budget). Output: NodeKind::Program
// with statements in source order, or Diag.
// Invariants: NL preprocessing FIRST, on a rebuilt stream — drop everything after the first
// Eof, splice out NL runs preceding To/PipeGt, collapse other runs to one Nl carrying the
// first NL's line, synthesize one trailing Eof with the previous token's line; statement
// dispatch is the exact 11-case C order (def > lone ~ > leading comma > compr > eval splice >
// elided effect head > source specials > hinge > query > juxtaposed verb > error); depth cap
// 4096 ("expression nested too deeply"); def heads checked via lex::lex_reserved exact-byte;
// ESpawn keeps count/at as positional Options, Stmt continuation/elided keeps sel = None.
pub fn parse(toks: &Toks, it: &mut Interner) -> Result<Node, Diag> {
    let n = toks.len();
    let mut ts = Toks::new();
    let mut i = 0usize;
    while i < n {
        let k = toks.kind[i];
        if k == TokKind::Eof {
            break;
        }
        if k == TokKind::Nl {
            let mut j = i;
            while j < n && toks.kind[j] == TokKind::Nl {
                j += 1;
            }
            if j < n && matches!(toks.kind[j], TokKind::To | TokKind::PipeGt) {
                i = j; // continuation: splice the run out
                continue;
            }
            ts.push(TokKind::Nl, Symbol::EMPTY, 0.0, toks.line[i]); // collapse the run to one NL
            i = j;
            continue;
        }
        ts.push(k, toks.name[i], toks.num[i], toks.line[i]);
        i += 1;
    }
    let eof_line = if ts.is_empty() { 1 } else { ts.line[ts.len() - 1] };
    ts.push(TokKind::Eof, Symbol::EMPTY, 0.0, eof_line);

    let mut p = P { t: &ts, i: 0, depth: 0, it };
    let mut stmts = Vec::new();
    loop {
        while p.pk() == TokKind::Nl {
            p.adv();
        }
        if p.pk() == TokKind::Eof {
            break;
        }
        stmts.push(p.parse_stmt()?);
        if p.pk() == TokKind::Nl {
            p.adv();
        } else if p.pk() != TokKind::Eof {
            return Err(perr(p.tline(), "trailing tokens on line"));
        }
    }
    Ok(Node::new(NodeKind::Program(stmts), 1))
}

/* ---------- self-test (the PARSE_TEST corpus, t01–t35) ---------- */
#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! tk {
        ($k:ident) => {
            (TokKind::$k, "", 0.0)
        };
    }
    macro_rules! tn {
        ($s:expr) => {
            (TokKind::Name, $s, 0.0)
        };
    }
    macro_rules! tv {
        ($v:expr) => {
            (TokKind::Num, "", $v as f64)
        };
    }
    macro_rules! tsy {
        ($s:expr) => {
            (TokKind::Sym, $s, 0.0)
        };
    }
    macro_rules! tkn {
        ($k:ident, $s:expr) => {
            (TokKind::$k, $s, 0.0)
        };
    }
    macro_rules! tknv {
        ($k:ident, $s:expr, $v:expr) => {
            (TokKind::$k, $s, $v as f64)
        };
    }

    // Inputs: case rows, interner. Output: the SoA stream the parser takes (all line 1).
    fn toks_of(rows: &[(TokKind, &str, f64)], it: &mut Interner) -> Toks {
        let mut t = Toks::new();
        for &(k, name, num) in rows {
            let sym = it.intern(name);
            t.push(k, sym, num, 1);
        }
        t
    }

    fn cmpc(op: CmpOp) -> char {
        match op {
            CmpOp::Lt => '<',
            CmpOp::Gt => '>',
            CmpOp::Le => 'l',
            CmpOp::Ge => 'g',
            CmpOp::Eq => '=',
            CmpOp::Ne => '!',
        }
    }

    fn arithc(op: ArithOp) -> char {
        match op {
            ArithOp::Add => '+',
            ArithOp::Sub => '-',
            ArithOp::Mul => '*',
            ArithOp::Div => '/',
            ArithOp::Mod => '%',
        }
    }

    fn assignc(op: AssignOp) -> char {
        match op {
            AssignOp::Set => '=',
            AssignOp::Add => '+',
            AssignOp::Sub => '-',
            AssignOp::Mul => '*',
            AssignOp::Div => '/',
        }
    }

    // %g for the corpus's small integers; Rust {} matches on this domain.
    fn g(v: f64) -> String {
        format!("{v}")
    }

    fn sx_opt(b: &mut String, n: Option<&Node>, it: &Interner) {
        match n {
            None => b.push_str("()"),
            Some(n) => sx(b, n, it),
        }
    }

    fn sx_kids(b: &mut String, kids: &[Node], it: &Interner) {
        for k in kids {
            b.push(' ');
            sx(b, k, it);
        }
    }

    // Inputs: out buffer, node. Output: the C sx S-expression appended:
    // (KIND[:FLAGS] [op] [name] [num] kids...), "()" for NULL kid slots.
    fn sx(b: &mut String, n: &Node, it: &Interner) {
        use std::fmt::Write;
        use NodeKind::*;
        match &n.kind {
            Num(v) => {
                let _ = write!(b, "(NUM {})", g(*v));
            }
            Counter { val, unit } => {
                let _ = write!(b, "(COUNTER {} {})", it.resolve(*unit), g(*val));
            }
            Sym(s) => {
                let _ = write!(b, "(SYM {})", it.resolve(*s));
            }
            Str(s) => {
                let _ = write!(b, "(STR {})", it.resolve(*s));
            }
            Name(s) => {
                let _ = write!(b, "(NAME {})", it.resolve(*s));
            }
            Alias(s) => {
                let _ = write!(b, "(ALIAS {})", it.resolve(*s));
            }
            Wild => b.push_str("(WILD)"),
            Not(x) => {
                b.push_str("(NOT ");
                sx(b, x, it);
                b.push(')');
            }
            And(l, r) => {
                b.push_str("(AND ");
                sx(b, l, it);
                b.push(' ');
                sx(b, r, it);
                b.push(')');
            }
            Or(l, r) => {
                b.push_str("(OR ");
                sx(b, l, it);
                b.push(' ');
                sx(b, r, it);
                b.push(')');
            }
            Cmp { op, l, r } => {
                let _ = write!(b, "(CMP {} ", cmpc(*op));
                sx(b, l, it);
                b.push(' ');
                sx(b, r, it);
                b.push(')');
            }
            CmpAny { name } => {
                let _ = write!(b, "(CMP _ (NAME {}))", it.resolve(*name));
            }
            Arith { op, l, r } => {
                let _ = write!(b, "(ARITH {} ", arithc(*op));
                sx(b, l, it);
                b.push(' ');
                sx(b, r, it);
                b.push(')');
            }
            Scope { l, r, origin } => {
                b.push_str("(SCOPE ");
                sx(b, l, it);
                b.push(' ');
                sx(b, r, it);
                if let Some(o) = origin {
                    b.push(' ');
                    sx(b, o, it);
                }
                b.push(')');
            }
            Hop { l, r } => {
                b.push_str("(HOP ");
                sx(b, l, it);
                b.push(' ');
                sx(b, r, it);
                b.push(')');
            }
            SetHop { rel } => {
                let s = it.resolve(*rel);
                let _ = write!(b, "(SETHOP {s} (NAME {s}))");
            }
            Call { callee, args } => {
                let _ = write!(b, "(CALL {}", it.resolve(*callee));
                sx_kids(b, args, it);
                b.push(')');
            }
            Fold { op, operand } => {
                let _ = write!(b, "(FOLD {} ", it.resolve(*op));
                sx(b, operand, it);
                b.push(')');
            }
            ScanExpr { op, operand, scan2 } => {
                let f = if *scan2 { ":SCAN2" } else { "" };
                let _ = write!(b, "(SCANEXPR{f} {} ", it.resolve(*op));
                sx(b, operand, it);
                b.push(')');
            }
            ScanAlong { op, col, order } => {
                let _ = write!(b, "(SCANALONG {} ", it.resolve(*op));
                sx(b, col, it);
                b.push(' ');
                sx(b, order, it);
                b.push(')');
            }
            IotaX(x) => {
                b.push_str("(IOTAX ");
                sx(b, x, it);
                b.push(')');
            }
            Shape(dims) => {
                b.push_str("(SHAPE");
                sx_kids(b, dims, it);
                b.push(')');
            }
            Tuple(els) => {
                b.push_str("(TUPLE");
                sx_kids(b, els, it);
                b.push(')');
            }
            To { shape, poured } => {
                b.push_str("(TO ");
                sx(b, shape, it);
                if let Some(s) = poured {
                    b.push(' ');
                    sx(b, s, it);
                }
                b.push(')');
            }
            Grade { key, desc } => {
                let f = if *desc { ":DESC" } else { "" };
                let _ = write!(b, "(GRADE{f} ");
                sx(b, key, it);
                b.push(')');
            }
            Top { k, inner } => {
                let _ = write!(b, "(TOP {} ", g(*k));
                sx(b, inner, it);
                b.push(')');
            }
            Pipe { src, stages } => {
                b.push_str("(PIPE ");
                sx(b, src, it);
                sx_kids(b, stages, it);
                b.push(')');
            }
            OrderBy { key, desc } => {
                let f = if *desc { ":DESC" } else { "" };
                let _ = write!(b, "(ORDERBY{f} ");
                sx(b, key, it);
                b.push(')');
            }
            Take { k } => {
                let _ = write!(b, "(TAKE {})", g(*k));
            }
            Expand(x) => {
                b.push_str("(EXPAND ");
                sx(b, x, it);
                b.push(')');
            }
            CrossV { f, a, b: bb } => {
                let _ = write!(b, "(CROSSV {} ", it.resolve(*f));
                sx(b, a, it);
                b.push(' ');
                sx(b, bb, it);
                b.push(')');
            }
            Binder { name, source } => {
                let _ = write!(b, "(BINDER {} ", it.resolve(*name));
                sx(b, source, it);
                b.push(')');
            }
            EAssign { op, target, rhs } => {
                let _ = write!(b, "(EASSIGN {} ", assignc(*op));
                sx(b, target, it);
                b.push(' ');
                sx(b, rhs, it);
                b.push(')');
            }
            EAdd(s) => {
                let _ = write!(b, "(EADD {})", it.resolve(*s));
            }
            EDel(s) => {
                let _ = write!(b, "(EDEL {})", it.resolve(*s));
            }
            EDespawn => b.push_str("(EDESPAWN)"),
            ESpawn { what, count, at } => {
                b.push_str("(ESPAWN ");
                sx(b, what, it);
                b.push(' ');
                sx_opt(b, count.as_deref(), it);
                b.push(' ');
                sx_opt(b, at.as_deref(), it);
                b.push(')');
            }
            EVerb { name, args } => {
                let _ = write!(b, "(EVERB {}", it.resolve(*name));
                sx_kids(b, args, it);
                b.push(')');
            }
            EVia { f, col } => {
                let _ = write!(b, "(EVIA {} ", it.resolve(*f));
                sx(b, col, it);
                b.push(')');
            }
            Stmt { sel, effects, rule, cont, elided } => {
                b.push_str("(STMT");
                if *rule {
                    b.push_str(":RULE");
                }
                if *cont {
                    b.push_str(":CONT");
                }
                if *elided {
                    b.push_str(":ELIDED");
                }
                b.push(' ');
                sx_opt(b, sel.as_deref(), it);
                sx_kids(b, effects, it);
                b.push(')');
            }
            DefStmt { name, body } => {
                let _ = write!(b, "(DEFSTMT {} ", it.resolve(*name));
                sx(b, body, it);
                b.push(')');
            }
            Query(x) => {
                b.push_str("(QUERY ");
                sx(b, x, it);
                b.push(')');
            }
            Compr { sel, effect, rest } => {
                b.push_str("(COMPR ");
                sx(b, sel, it);
                b.push(' ');
                sx(b, effect, it);
                sx_kids(b, rest, it);
                b.push(')');
            }
            Program(ss) => {
                b.push_str("(PROGRAM");
                sx_kids(b, ss, it);
                b.push(')');
            }
        }
    }

    fn run_case(label: &str, rows: &[(TokKind, &str, f64)], want: &str) {
        let mut it = Interner::new();
        let toks = toks_of(rows, &mut it);
        let prog = match parse(&toks, &mut it) {
            Ok(p) => p,
            Err(d) => panic!("FAIL {label}: parse error: {}", d.msg),
        };
        let mut b = String::new();
        sx(&mut b, &prog, &it);
        assert_eq!(b, want, "case {label}");
    }

    #[test]
    fn parse_corpus() {
        // Nord & TwoHanded > 60 , Gold += 1000
        run_case(
            "hinge+cmp",
            &[tn!("Nord"), tk!(Amp), tn!("TwoHanded"), tk!(Gt), tv!(60), tk!(Comma), tn!("Gold"), tk!(PlusEq), tv!(1000), tk!(Eof)],
            "(PROGRAM (STMT (AND (NAME Nord) (CMP > (NAME TwoHanded) (NUM 60))) (EASSIGN + (NAME Gold) (NUM 1000))))",
        );
        // Bandit & !Dead & Faction == :Bandit , Faction = :Hostile
        run_case(
            "not+sym+eq",
            &[tn!("Bandit"), tk!(Amp), tk!(Bang), tn!("Dead"), tk!(Amp), tn!("Faction"), tk!(EqEq), tsy!("Bandit"), tk!(Comma), tn!("Faction"), tk!(Eq), tsy!("Hostile"), tk!(Eof)],
            "(PROGRAM (STMT (AND (AND (NAME Bandit) (NOT (NAME Dead))) (CMP = (NAME Faction) (SYM Bandit))) (EASSIGN = (NAME Faction) (SYM Hostile))))",
        );
        // Frenzy.targets' , +Frenzied
        run_case(
            "hop-tick",
            &[tn!("Frenzy"), tk!(Dot), tn!("targets"), tk!(Tick), tk!(Comma), tk!(Plus), tn!("Frenzied"), tk!(Eof)],
            "(PROGRAM (STMT (HOP (NAME Frenzy) (SETHOP targets (NAME targets))) (EADD Frenzied)))",
        );
        // ^cursor , Knockback 5 ; Flash :Red ; -Shielded
        run_case(
            "verb-batch",
            &[tkn!(Alias, "cursor"), tk!(Comma), tn!("Knockback"), tv!(5), tk!(Semi), tn!("Flash"), tsy!("Red"), tk!(Semi), tk!(Minus), tn!("Shielded"), tk!(Eof)],
            "(PROGRAM (STMT (ALIAS cursor) (EVERB Knockback (NUM 5)) (EVERB Flash (SYM Red)) (EDEL Shielded)))",
        );
        // +/ Gold @ Nord
        run_case(
            "fold-scope",
            &[tkn!(Fold, "+"), tn!("Gold"), tk!(At), tn!("Nord"), tk!(Eof)],
            "(PROGRAM (QUERY (FOLD + (SCOPE (NAME Gold) (NAME Nord)))))",
        );
        // Plot & !Planted & #/ (neighbors' & Planted) >= 2 , +Planted
        run_case(
            "count-fold-cmp",
            &[tn!("Plot"), tk!(Amp), tk!(Bang), tn!("Planted"), tk!(Amp), tkn!(Fold, "#"), tk!(Lp), tn!("neighbors"), tk!(Tick), tk!(Amp), tn!("Planted"), tk!(Rp), tk!(Ge), tv!(2), tk!(Comma), tk!(Plus), tn!("Planted"), tk!(Eof)],
            "(PROGRAM (STMT (AND (AND (NAME Plot) (NOT (NAME Planted))) (CMP g (FOLD # (AND (SETHOP neighbors (NAME neighbors)) (NAME Planted))) (NUM 2))) (EADD Planted)))",
        );
        // top 5 (grade desc Threat) , +Targeted
        run_case(
            "top-grade",
            &[tk!(Top), tv!(5), tk!(Lp), tk!(Grade), tk!(Desc), tn!("Threat"), tk!(Rp), tk!(Comma), tk!(Plus), tn!("Targeted"), tk!(Eof)],
            "(PROGRAM (STMT (TOP 5 (GRADE:DESC (NAME Threat))) (EADD Targeted)))",
        );
        // Enemy |> order by Threat desc |> take 5 , +Targeted
        run_case(
            "pipeline",
            &[tn!("Enemy"), tk!(PipeGt), tk!(Order), tk!(By), tn!("Threat"), tk!(Desc), tk!(PipeGt), tk!(Take), tv!(5), tk!(Comma), tk!(Plus), tn!("Targeted"), tk!(Eof)],
            "(PROGRAM (STMT (PIPE (NAME Enemy) (ORDERBY:DESC (NAME Threat)) (TAKE 5)) (EADD Targeted)))",
        );
        // +/ threat @ (Enemy |> order by dps desc |> take 10)
        run_case(
            "fold-pipe-paren",
            &[tkn!(Fold, "+"), tn!("threat"), tk!(At), tk!(Lp), tn!("Enemy"), tk!(PipeGt), tk!(Order), tk!(By), tn!("dps"), tk!(Desc), tk!(PipeGt), tk!(Take), tv!(10), tk!(Rp), tk!(Eof)],
            "(PROGRAM (QUERY (FOLD + (SCOPE (NAME threat) (PIPE (NAME Enemy) (ORDERBY:DESC (NAME dps)) (TAKE 10))))))",
        );
        // Unit , Slot = rank(Initiative)
        run_case(
            "call-rhs",
            &[tn!("Unit"), tk!(Comma), tn!("Slot"), tk!(Eq), tn!("rank"), tk!(Lp), tn!("Initiative"), tk!(Rp), tk!(Eof)],
            "(PROGRAM (STMT (NAME Unit) (EASSIGN = (NAME Slot) (CALL rank (NAME Initiative)))))",
        );
        // 12 , offset = prev.offset + prev.prev.offset \n , spawn Cheese at Player.pos + (offset, 0)
        run_case(
            "shape+cont",
            &[tv!(12), tk!(Comma), tn!("offset"), tk!(Eq), tn!("prev"), tk!(Dot), tn!("offset"), tk!(Plus), tn!("prev"), tk!(Dot), tn!("prev"), tk!(Dot), tn!("offset"), tk!(Nl), tk!(Comma), tk!(Spawn), tn!("Cheese"), tk!(AtKw), tn!("Player"), tk!(Dot), tn!("pos"), tk!(Plus), tk!(Lp), tn!("offset"), tk!(Comma), tv!(0), tk!(Rp), tk!(Eof)],
            "(PROGRAM (STMT (SHAPE (NUM 12)) (EASSIGN = (NAME offset) (ARITH + (HOP (NAME prev) (NAME offset)) (HOP (HOP (NAME prev) (NAME prev)) (NAME offset))))) (STMT:CONT () (ESPAWN (NAME Cheese) () (ARITH + (HOP (NAME Player) (NAME pos)) (TUPLE (NAME offset) (NUM 0))))))",
        );
        // Soldier , pos = to 4 _
        run_case(
            "to-wild",
            &[tn!("Soldier"), tk!(Comma), tn!("pos"), tk!(Eq), tk!(To), tv!(4), tk!(Wild), tk!(Eof)],
            "(PROGRAM (STMT (NAME Soldier) (EASSIGN = (NAME pos) (TO (SHAPE (NUM 4) (WILD))))))",
        );
        // 8 8 & (x + y) % 2 == 0 , spawn Wheat
        run_case(
            "lattice-pred",
            &[tv!(8), tv!(8), tk!(Amp), tk!(Lp), tn!("x"), tk!(Plus), tn!("y"), tk!(Rp), tk!(Pct), tv!(2), tk!(EqEq), tv!(0), tk!(Comma), tk!(Spawn), tn!("Wheat"), tk!(Eof)],
            "(PROGRAM (STMT (AND (SHAPE (NUM 8) (NUM 8)) (CMP = (ARITH % (ARITH + (NAME x) (NAME y)) (NUM 2)) (NUM 0))) (ESPAWN (NAME Wheat) () ())))",
        );
        // "RNBQ" \n to 8 8 , spawn (pieceOf char)   — exercises the NL-before-to splice
        run_case(
            "board-splice",
            &[tkn!(Str, "RNBQ"), tk!(Nl), tk!(To), tv!(8), tv!(8), tk!(Comma), tk!(Spawn), tk!(Lp), tn!("pieceOf"), tn!("char"), tk!(Rp), tk!(Eof)],
            "(PROGRAM (STMT (TO (SHAPE (NUM 8) (NUM 8)) (STR RNBQ)) (ESPAWN (CALL pieceOf (NAME char)) () ())))",
        );
        // Nord & Dead , spawn Ghost \n ~
        run_case(
            "despawn-cont",
            &[tn!("Nord"), tk!(Amp), tn!("Dead"), tk!(Comma), tk!(Spawn), tn!("Ghost"), tk!(Nl), tk!(Tilde), tk!(Eof)],
            "(PROGRAM (STMT (AND (NAME Nord) (NAME Dead)) (ESPAWN (NAME Ghost) () ())) (STMT:CONT () (EDESPAWN)))",
        );
        // [ t & c , +InRange | t <- Tower, c <- Creep, dist(t, c) < 50 ]
        run_case(
            "comprehension",
            &[tk!(Lb), tn!("t"), tk!(Amp), tn!("c"), tk!(Comma), tk!(Plus), tn!("InRange"), tk!(Bar), tn!("t"), tk!(LArrow), tn!("Tower"), tk!(Comma), tn!("c"), tk!(LArrow), tn!("Creep"), tk!(Comma), tn!("dist"), tk!(Lp), tn!("t"), tk!(Comma), tn!("c"), tk!(Rp), tk!(Lt), tv!(50), tk!(Rb), tk!(Eof)],
            "(PROGRAM (COMPR (AND (NAME t) (NAME c)) (EADD InRange) (BINDER t (NAME Tower)) (BINDER c (NAME Creep)) (CMP < (CALL dist (NAME t) (NAME c)) (NUM 50))))",
        );
        // Hostile , shortestPath via Adj
        run_case(
            "via",
            &[tn!("Hostile"), tk!(Comma), tn!("shortestPath"), tk!(Via), tn!("Adj"), tk!(Eof)],
            "(PROGRAM (STMT (NAME Hostile) (EVIA shortestPath (NAME Adj))))",
        );
        // 64 64 & top 5 (grade desc Resource) , +MiningNode
        run_case(
            "shape-top",
            &[tv!(64), tv!(64), tk!(Amp), tk!(Top), tv!(5), tk!(Lp), tk!(Grade), tk!(Desc), tn!("Resource"), tk!(Rp), tk!(Comma), tk!(Plus), tn!("MiningNode"), tk!(Eof)],
            "(PROGRAM (STMT (AND (SHAPE (NUM 64) (NUM 64)) (TOP 5 (GRADE:DESC (NAME Resource)))) (EADD MiningNode)))",
        );
        // order by threat desc
        run_case(
            "orderby-query",
            &[tk!(Order), tk!(By), tn!("threat"), tk!(Desc), tk!(Eof)],
            "(PROGRAM (QUERY (ORDERBY:DESC (NAME threat))))",
        );
        // Cell @ row , Height = prev.Height + prev.prev.Height
        run_case(
            "stencil",
            &[tn!("Cell"), tk!(At), tn!("row"), tk!(Comma), tn!("Height"), tk!(Eq), tn!("prev"), tk!(Dot), tn!("Height"), tk!(Plus), tn!("prev"), tk!(Dot), tn!("prev"), tk!(Dot), tn!("Height"), tk!(Eof)],
            "(PROGRAM (STMT (SCOPE (NAME Cell) (NAME row)) (EASSIGN = (NAME Height) (ARITH + (HOP (NAME prev) (NAME Height)) (HOP (HOP (NAME prev) (NAME prev)) (NAME Height))))))",
        );
        // scan(+) Weight along pathCells
        run_case(
            "scan-along",
            &[tk!(ScanKw), tk!(Lp), tk!(Plus), tk!(Rp), tn!("Weight"), tk!(Along), tn!("pathCells"), tk!(Eof)],
            "(PROGRAM (QUERY (SCANALONG + (NAME Weight) (NAME pathCells))))",
        );
        // (Nord, TwoHanded _) , +Trained
        run_case(
            "presence-tuple",
            &[tk!(Lp), tn!("Nord"), tk!(Comma), tn!("TwoHanded"), tk!(Wild), tk!(Rp), tk!(Comma), tk!(Plus), tn!("Trained"), tk!(Eof)],
            "(PROGRAM (STMT (TUPLE (NAME Nord) (CMP _ (NAME TwoHanded))) (EADD Trained)))",
        );
        // spawn Wheat
        run_case(
            "elided-spawn",
            &[tk!(Spawn), tn!("Wheat"), tk!(Eof)],
            "(PROGRAM (STMT:ELIDED () (ESPAWN (NAME Wheat) () ())))",
        );
        // Cheese @ cellar & Aged > 3mo , Price *= 2
        run_case(
            "counter",
            &[tn!("Cheese"), tk!(At), tn!("cellar"), tk!(Amp), tn!("Aged"), tk!(Gt), tknv!(Counter, "mo", 3), tk!(Comma), tn!("Price"), tk!(StarEq), tv!(2), tk!(Eof)],
            "(PROGRAM (STMT (AND (SCOPE (NAME Cheese) (NAME cellar)) (CMP > (NAME Aged) (COUNTER mo 3))) (EASSIGN * (NAME Price) (NUM 2))))",
        );
        // max\ Height @ (Eye + til n * north)
        run_case(
            "iota-scan",
            &[tkn!(ScanOp, "max"), tn!("Height"), tk!(At), tk!(Lp), tn!("Eye"), tk!(Plus), tk!(Iota), tn!("n"), tk!(Star), tn!("north"), tk!(Rp), tk!(Eof)],
            "(PROGRAM (QUERY (SCANEXPR max (SCOPE (NAME Height) (ARITH + (NAME Eye) (ARITH * (IOTAX (NAME n)) (NAME north)))))))",
        );
        // +/ Elevation @ 64 64
        run_case(
            "at-shape",
            &[tkn!(Fold, "+"), tn!("Elevation"), tk!(At), tv!(64), tv!(64), tk!(Eof)],
            "(PROGRAM (QUERY (FOLD + (SCOPE (NAME Elevation) (SHAPE (NUM 64) (NUM 64))))))",
        );
        // Spawner |> expand Count , spawn Minion
        run_case(
            "expand",
            &[tn!("Spawner"), tk!(PipeGt), tk!(Expand), tn!("Count"), tk!(Comma), tk!(Spawn), tn!("Minion"), tk!(Eof)],
            "(PROGRAM (STMT (PIPE (NAME Spawner) (EXPAND (NAME Count))) (ESPAWN (NAME Minion) () ())))",
        );
        // Spawner , spawn Minion * Count
        run_case(
            "spawn-mult",
            &[tn!("Spawner"), tk!(Comma), tk!(Spawn), tn!("Minion"), tk!(Star), tn!("Count"), tk!(Eof)],
            "(PROGRAM (STMT (NAME Spawner) (ESPAWN (NAME Minion) (NAME Count) ())))",
        );
        // fold(threat) Damage @ Enemies
        run_case(
            "fold-long",
            &[tk!(FoldKw), tk!(Lp), tn!("threat"), tk!(Rp), tn!("Damage"), tk!(At), tn!("Enemies"), tk!(Eof)],
            "(PROGRAM (QUERY (FOLD threat (SCOPE (NAME Damage) (NAME Enemies)))))",
        );
        // +\ Weight @ (til steps |> route A B)
        run_case(
            "iota-pipe",
            &[tkn!(ScanOp, "+"), tn!("Weight"), tk!(At), tk!(Lp), tk!(Iota), tn!("steps"), tk!(PipeGt), tn!("route"), tn!("A"), tn!("B"), tk!(Rp), tk!(Eof)],
            "(PROGRAM (QUERY (SCANEXPR + (SCOPE (NAME Weight) (PIPE (IOTAX (NAME steps)) (CALL route (NAME A) (NAME B)))))))",
        );
        // Plot , Moisture = avg/ neighbors'.Moisture
        run_case(
            "sethop-gather",
            &[tn!("Plot"), tk!(Comma), tn!("Moisture"), tk!(Eq), tkn!(Fold, "avg"), tn!("neighbors"), tk!(Tick), tk!(Dot), tn!("Moisture"), tk!(Eof)],
            "(PROGRAM (STMT (NAME Plot) (EASSIGN = (NAME Moisture) (FOLD avg (HOP (SETHOP neighbors (NAME neighbors)) (NAME Moisture))))))",
        );
        // Knockback 5   — line-start juxtaposed verb, elided subject
        run_case(
            "verb-line",
            &[tn!("Knockback"), tv!(5), tk!(Eof)],
            "(PROGRAM (STMT:ELIDED () (EVERB Knockback (NUM 5))))",
        );
        // cross dist Tower Creep
        run_case(
            "cross",
            &[tk!(Cross), tn!("dist"), tn!("Tower"), tn!("Creep"), tk!(Eof)],
            "(PROGRAM (QUERY (CROSSV dist (NAME Tower) (NAME Creep))))",
        );
    }

    // Def statements consult lex::lex_reserved — split out so the corpus above stays green
    // while lex.rs is mid-port.
    #[test]
    fn parse_corpus_defs() {
        // def threat = Damage * Speed / Range
        run_case(
            "def-column",
            &[tk!(Def), tn!("threat"), tk!(Eq), tn!("Damage"), tk!(Star), tn!("Speed"), tk!(Slash), tn!("Range"), tk!(Eof)],
            "(PROGRAM (DEFSTMT threat (ARITH / (ARITH * (NAME Damage) (NAME Speed)) (NAME Range))))",
        );
        // def spread = Plot & p => +Planted
        run_case(
            "def-rule",
            &[tk!(Def), tn!("spread"), tk!(Eq), tn!("Plot"), tk!(Amp), tn!("p"), tk!(Arrow), tk!(Plus), tn!("Planted"), tk!(Eof)],
            "(PROGRAM (DEFSTMT spread (STMT:RULE (AND (NAME Plot) (NAME p)) (EADD Planted))))",
        );
    }
}

