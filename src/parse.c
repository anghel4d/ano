/* parse.c — Pratt parser for anoc: token columns (Toks) -> AST per GRAMMAR.md's 14 levels.
 * Statement classification: def / continuation (~ or leading ,) / comprehension /
 * elided-effect head / selection-hinge-effects / bare query. Line splicing: a line
 * whose first token is T_TO or T_PIPEGT continues the previous line (ex34 board literal).
 * Owns node_new and node_addkid. On error: err filled "line N: msg", returns NULL.
 */
#include "ano.h"

/* ---------- node constructors ---------- */

/* Inputs: arena, node kind, source line. Output: zeroed node with kind+line set.
 * Invariant: never NULL (arena aborts on OOM). */
Node *node_new(Arena *a, NodeKind k, int line) {
  Node *n = (Node *)arena_alloc(a, sizeof *n);
  n->kind = k; n->line = line; n->name = "";
  return n;
}

/* Inputs: arena, parent, kid (NULL allowed: positional empty slot).
 * Output: kid appended; kids array grows by doubling. */
void node_addkid(Arena *a, Node *n, Node *kid) {
  if ((n->nkids & (n->nkids - 1)) == 0) {   /* at 0,1,2,4,8,... grow */
    int cap = n->nkids ? n->nkids * 2 : 1;
    Node **nk = (Node **)arena_alloc(a, (size_t)cap * sizeof *nk);
    if (n->nkids) memcpy(nk, n->kids, (size_t)n->nkids * sizeof *nk);
    n->kids = nk;
  }
  n->kids[n->nkids++] = kid;
}

/* ---------- parser state ---------- */

typedef struct {
  const Toks *t; int i;
  Arena *a;
  char *err; size_t errsz;
  int depth;                /* parse_expr nesting; bounded so degenerate input errors, not overflows */
} P;

static TokKind pk(P *p) { return p->t->kind[p->i]; }
static TokKind pk2(P *p, int k) { int j = p->i + k; return j < p->t->n ? p->t->kind[j] : T_EOF; }
static const char *tname(P *p) { return p->t->name[p->i]; }
static double tnum(P *p) { return p->t->num[p->i]; }
static int tline(P *p) { return p->t->line[p->i]; }
static void adv(P *p) { if (p->i < p->t->n - 1) p->i++; }

/* Inputs: parser, line, printf args. Output: NULL; first error wins. */
static Node *perrf(P *p, int line, const char *fmt, ...) {
  if (p->err && p->errsz && !p->err[0]) {
    int off = snprintf(p->err, p->errsz, "line %d: ", line);
    if (off < 0 || (size_t)off >= p->errsz) return NULL;
    va_list ap; va_start(ap, fmt);
    vsnprintf(p->err + off, p->errsz - (size_t)off, fmt, ap);
    va_end(ap);
  }
  return NULL;
}

/* Inputs: parser, expected kind, description. Output: 0 and consumed / -1 with err. */
static int expect(P *p, TokKind k, const char *what) {
  if (pk(p) != k) { perrf(p, tline(p), "expected %s", what); return -1; }
  adv(p);
  return 0;
}

static void setname(Node *n, const char *s) { n->name = s; }   /* interned or static; no copy */

/* ---------- token classes ---------- */

/* Input: token kind. Output: binary precedence level per GRAMMAR.md, 0 = not binary. */
static int binlevel(TokKind k) {
  switch (k) {
    case T_PIPEGT: return 3;
    case T_BAR:    return 5;
    case T_AMP:    return 6;
    case T_EQEQ: case T_NE: case T_LT: case T_LE: case T_GT: case T_GE: case T_EQ: return 8;
    case T_PLUS: case T_MINUS: return 10;
    case T_STAR: case T_SLASH: case T_PCT: return 11;
    case T_AT:  return 12;
    case T_DOT: case T_TICK: return 13;
    default: return 0;
  }
}

/* Input: comparison token. Output: N_CMP op char per ano.h. */
static char cmpop(TokKind k) {
  switch (k) {
    case T_EQEQ: case T_EQ: return '=';
    case T_NE: return '!'; case T_LT: return '<'; case T_LE: return 'l';
    case T_GT: return '>'; case T_GE: return 'g';
    default: return 0;
  }
}

/* Input: arithmetic token. Output: N_ARITH op char. */
static char arithop(TokKind k) {
  switch (k) {
    case T_PLUS: return '+'; case T_MINUS: return '-';
    case T_STAR: return '*'; case T_SLASH: return '/'; case T_PCT: return '%';
    default: return 0;
  }
}

/* Input: effect assignment token. Output: N_EASSIGN op char, 0 = not one. */
static char assignop(TokKind k) {
  switch (k) {
    case T_EQ: return '='; case T_PLUSEQ: return '+'; case T_MINUSEQ: return '-';
    case T_STAREQ: return '*'; case T_SLASHEQ: return '/';
    default: return 0;
  }
}

/* Input: token kind. Output: 1 when it can begin an expression atom. */
static int atomstart(TokKind k) {
  switch (k) {
    case T_NAME: case T_ALIAS: case T_SYM: case T_NUM: case T_COUNTER:
    case T_STR: case T_WILD: case T_LP: case T_IOTA: return 1;
    default: return 0;
  }
}

/* ---------- expressions ---------- */

static Node *parse_expr(P *p, int min);
static Node *parse_effect(P *p);

/* Input: parser at T_NAME. Output: N_NAME node, token consumed. */
static Node *mkname(P *p) {
  Node *n = node_new(p->a, N_NAME, tline(p));
  setname(n, tname(p));
  adv(p);
  return n;
}

/* Input: name node. Output: N_SETHOP wrapping it (name copied, kids[0]=name node). */
static Node *mksethop(P *p, Node *nm) {
  Node *s = node_new(p->a, N_SETHOP, nm->line);
  s->name = nm->name;
  node_addkid(p->a, s, nm);
  return s;
}

/* Input: parser at NUM (after @ or after to). Output: N_SHAPE of 1-2 dims.
 * allowWild permits T_WILD dims (the `to 4 _` form). */
static Node *parse_shape(P *p, int allowWild) {
  Node *sh = node_new(p->a, N_SHAPE, tline(p));
  for (int d = 0; d < 2; d++) {
    if (pk(p) == T_NUM) {
      Node *nn = node_new(p->a, N_NUM, tline(p)); nn->num = tnum(p);
      adv(p); node_addkid(p->a, sh, nn);
    } else if (allowWild && pk(p) == T_WILD) {
      node_addkid(p->a, sh, node_new(p->a, N_WILD, tline(p)));
      adv(p);
    } else if (d == 0) {
      return perrf(p, tline(p), "expected shape dimension");
    } else break;
  }
  return sh;
}

/* Input: parser at T_TO. Output: N_TO, kids[0]=N_SHAPE (dims NUM or WILD). */
static Node *parse_to(P *p) {
  int line = tline(p);
  adv(p);
  Node *to = node_new(p->a, N_TO, line);
  Node *sh = parse_shape(p, 1);
  if (!sh) return NULL;
  node_addkid(p->a, to, sh);
  return to;
}

/* Input: parser at T_NAME with '(' next. Output: N_CALL, args comma-separated at min 5. */
static Node *parse_call(P *p) {
  Node *c = node_new(p->a, N_CALL, tline(p));
  setname(c, tname(p));
  adv(p); adv(p);                                     /* name ( */
  if (pk(p) != T_RP) {
    for (;;) {
      Node *arg = parse_expr(p, 5);
      if (!arg) return NULL;
      node_addkid(p->a, c, arg);
      if (pk(p) == T_COMMA) { adv(p); continue; }
      break;
    }
  }
  if (expect(p, T_RP, "')' after call arguments")) return NULL;
  return c;
}

/* Input: parser, min level. Output: one tuple element: expr, with the
 * presence-any form `NAME _` wrapped as N_CMP op '_' kids[0]=name. */
static Node *parse_tupelem(P *p, int min) {
  Node *e = parse_expr(p, min);
  if (!e) return NULL;
  if (e->kind == N_NAME && pk(p) == T_WILD) {
    Node *c = node_new(p->a, N_CMP, e->line);
    c->op = '_';
    node_addkid(p->a, c, e);
    adv(p);
    return c;
  }
  return e;
}

/* Input: parser at T_LP. Output: parenthesized expr, N_TUPLE on top-level comma,
 * or juxtaposed N_CALL when a bare name is followed by atoms ((pieceOf char)). */
static Node *parse_paren(P *p) {
  int line = tline(p);
  adv(p);
  Node *e = parse_tupelem(p, 2);
  if (!e) return NULL;
  if (e->kind == N_NAME && atomstart(pk(p))) {        /* juxtaposed call */
    Node *c = node_new(p->a, N_CALL, e->line);
    c->name = e->name;
    while (atomstart(pk(p))) {
      Node *arg = parse_expr(p, 10);
      if (!arg) return NULL;
      node_addkid(p->a, c, arg);
    }
    e = c;
  }
  if (pk(p) == T_COMMA) {                             /* tuple */
    Node *tu = node_new(p->a, N_TUPLE, line);
    node_addkid(p->a, tu, e);
    while (pk(p) == T_COMMA) {
      adv(p);
      Node *el = parse_tupelem(p, 5);
      if (!el) return NULL;
      node_addkid(p->a, tu, el);
    }
    e = tu;
  }
  if (expect(p, T_RP, "')'")) return NULL;
  return e;
}

/* Input: parser at level-14 position. Output: atom node. */
static Node *parse_atom(P *p) {
  int line = tline(p);
  switch (pk(p)) {
    case T_NUM: {
      Node *n = node_new(p->a, N_NUM, line); n->num = tnum(p); adv(p); return n;
    }
    case T_COUNTER: {
      Node *n = node_new(p->a, N_COUNTER, line);
      n->num = tnum(p); setname(n, tname(p)); adv(p); return n;
    }
    case T_SYM: {
      Node *n = node_new(p->a, N_SYM, line); setname(n, tname(p)); adv(p); return n;
    }
    case T_STR: {
      Node *n = node_new(p->a, N_STR, line); setname(n, tname(p)); adv(p); return n;
    }
    case T_ALIAS: {
      Node *n = node_new(p->a, N_ALIAS, line); setname(n, tname(p)); adv(p); return n;
    }
    case T_WILD: adv(p); return node_new(p->a, N_WILD, line);
    case T_NAME:
      if (pk2(p, 1) == T_LP) return parse_call(p);
      return mkname(p);
    case T_IOTA: {
      adv(p);
      Node *n = node_new(p->a, N_IOTAX, line);
      Node *k = parse_expr(p, 12);                    /* just the atom-ish operand */
      if (!k) return NULL;
      node_addkid(p->a, n, k);
      return n;
    }
    case T_LP: return parse_paren(p);
    default: return perrf(p, line, "unexpected token in expression");
  }
}

/* Input: parser at an order-by head. Output: N_ORDERBY kids[0]=key, F_DESC on desc. */
static Node *parse_orderby(P *p) {
  int line = tline(p);
  adv(p);
  if (expect(p, T_BY, "'by' after 'order'")) return NULL;
  Node *n = node_new(p->a, N_ORDERBY, line);
  Node *key = parse_expr(p, 5);
  if (!key) return NULL;
  node_addkid(p->a, n, key);
  if (pk(p) == T_DESC) { adv(p); n->flags |= F_DESC; }
  return n;
}

/* Input: parser at a pipeline stage head. Output: stage node
 * (N_ORDERBY | N_TAKE | N_EXPAND | juxtaposed N_CALL). */
static Node *parse_stage(P *p) {
  int line = tline(p);
  switch (pk(p)) {
    case T_ORDER: return parse_orderby(p);
    case T_TAKE: {
      adv(p);
      if (pk(p) != T_NUM) return perrf(p, tline(p), "expected count after 'take'");
      Node *n = node_new(p->a, N_TAKE, line); n->num = tnum(p); adv(p); return n;
    }
    case T_EXPAND: {
      adv(p);
      Node *n = node_new(p->a, N_EXPAND, line);
      Node *col = parse_expr(p, 5);
      if (!col) return NULL;
      node_addkid(p->a, n, col);
      return n;
    }
    case T_NAME: {
      Node *n = node_new(p->a, N_CALL, line);
      setname(n, tname(p));
      adv(p);
      while (atomstart(pk(p))) {
        Node *arg = parse_expr(p, 10);
        if (!arg) return NULL;
        node_addkid(p->a, n, arg);
      }
      return n;
    }
    default: return perrf(p, line, "expected pipeline stage");
  }
}

/* Input: parser inside scan/scan2 parens. Output: 0 with op spelling or
 * reducer name copied into n->name; -1 with err. */
static int parse_opname(P *p, Node *n) {
  switch (pk(p)) {
    case T_PLUS:  setname(n, "+"); break;
    case T_MINUS: setname(n, "-"); break;
    case T_STAR:  setname(n, "*"); break;
    case T_AMP:   setname(n, "&"); break;
    case T_BAR:   setname(n, "|"); break;
    case T_NAME:  setname(n, tname(p)); break;
    default: perrf(p, tline(p), "expected operator or reducer name"); return -1;
  }
  adv(p);
  return 0;
}

/* Input: parser, min level. Output: prefix construct (! at 7; fold/scan/grade/top/
 * fold(f)/scan-along/scan2/cross at 9; order-by head at 3) or an atom. Fold-family
 * operands parse at min 10 so @ (12) and . (13) fall inside the operand. */
static Node *parse_prefix(P *p, int min) {
  int line = tline(p);
  TokKind k = pk(p);
  if (k == T_BANG && min <= 7) {
    adv(p);
    Node *n = node_new(p->a, N_NOT, line);
    Node *x = parse_expr(p, 8);
    if (!x) return NULL;
    node_addkid(p->a, n, x);
    return n;
  }
  if (min <= 9) switch (k) {
    case T_FOLD: case T_SCANOP: {
      Node *n = node_new(p->a, k == T_FOLD ? N_FOLD : N_SCANEXPR, line);
      setname(n, tname(p));
      adv(p);
      Node *x = parse_expr(p, 10);
      if (!x) return NULL;
      node_addkid(p->a, n, x);
      return n;
    }
    case T_GRADE: {
      adv(p);
      Node *n = node_new(p->a, N_GRADE, line);
      if (pk(p) == T_DESC) { adv(p); n->flags |= F_DESC; }
      Node *key = parse_expr(p, 10);
      if (!key) return NULL;
      node_addkid(p->a, n, key);
      return n;
    }
    case T_TOP: {
      adv(p);
      if (pk(p) != T_NUM) return perrf(p, tline(p), "expected count after 'top'");
      Node *n = node_new(p->a, N_TOP, line);
      n->num = tnum(p); adv(p);
      Node *x = parse_expr(p, 10);
      if (!x) return NULL;
      node_addkid(p->a, n, x);
      return n;
    }
    case T_FOLDKW: { /* fold(f): the long form of f/ — one node, N_FOLD */
      adv(p);
      Node *n = node_new(p->a, N_FOLD, line);
      if (expect(p, T_LP, "'(' after 'fold'")) return NULL;
      if (pk(p) != T_NAME) return perrf(p, tline(p), "expected reducer name");
      setname(n, tname(p)); adv(p);
      if (expect(p, T_RP, "')' after reducer")) return NULL;
      Node *x = parse_expr(p, 10);
      if (!x) return NULL;
      node_addkid(p->a, n, x);
      return n;
    }
    case T_SCANKW: {
      adv(p);
      Node *n = node_new(p->a, N_SCANALONG, line);
      if (expect(p, T_LP, "'(' after 'scan'")) return NULL;
      if (parse_opname(p, n)) return NULL;
      if (expect(p, T_RP, "')' after scan operator")) return NULL;
      Node *col = parse_expr(p, 10);
      if (!col) return NULL;
      node_addkid(p->a, n, col);
      if (expect(p, T_ALONG, "'along' in scan")) return NULL;
      Node *ord = parse_expr(p, 10);
      if (!ord) return NULL;
      node_addkid(p->a, n, ord);
      return n;
    }
    case T_SCAN2: {
      adv(p);
      Node *n = node_new(p->a, N_SCANEXPR, line);
      n->flags |= F_SCAN2;
      if (expect(p, T_LP, "'(' after 'scan2'")) return NULL;
      if (parse_opname(p, n)) return NULL;
      if (expect(p, T_RP, "')' after scan2 operator")) return NULL;
      Node *x = parse_expr(p, 10);
      if (!x) return NULL;
      node_addkid(p->a, n, x);
      return n;
    }
    case T_CROSS: {
      adv(p);
      Node *n = node_new(p->a, N_CROSSV, line);
      if (pk(p) != T_NAME) return perrf(p, tline(p), "expected function after 'cross'");
      setname(n, tname(p)); adv(p);
      Node *x = parse_expr(p, 10);
      if (!x) return NULL;
      node_addkid(p->a, n, x);
      Node *y = parse_expr(p, 10);
      if (!y) return NULL;
      node_addkid(p->a, n, y);
      return n;
    }
    default: break;
  }
  if (k == T_ORDER && min <= 3) return parse_orderby(p);  /* src-less pipeline head */
  return parse_atom(p);
}

/* Inputs: parser, parsed left operand, min level. Output: expression climbed from l:
 * left-assoc binaries at their level, . and ' postfix at 13, |> pipeline at 3,
 * @-then-NUM shape scope. */
static Node *parse_binloop(P *p, Node *l, int min) {
  for (;;) {
    TokKind k = pk(p);
    int lv = binlevel(k);
    if (!lv || lv < min) return l;
    int line = tline(p);
    if (k == T_DOT) {
      adv(p);
      if (pk(p) != T_NAME) return perrf(p, tline(p), "expected name after '.'");
      Node *h = node_new(p->a, N_HOP, line);
      node_addkid(p->a, h, l);
      node_addkid(p->a, h, mkname(p));
      l = h;
      continue;
    }
    if (k == T_TICK) {
      adv(p);
      if (l->kind == N_NAME) l = mksethop(p, l);
      else if (l->kind == N_HOP && l->nkids == 2 && l->kids[1] && l->kids[1]->kind == N_NAME)
        l->kids[1] = mksethop(p, l->kids[1]);
      else return perrf(p, line, "tick after non-name");
      continue;
    }
    if (k == T_PIPEGT) {
      Node *pipe = node_new(p->a, N_PIPE, line);
      node_addkid(p->a, pipe, l);
      while (pk(p) == T_PIPEGT) {
        adv(p);
        Node *st = parse_stage(p);
        if (!st) return NULL;
        node_addkid(p->a, pipe, st);
      }
      l = pipe;
      continue;
    }
    adv(p);
    Node *r;
    if (k == T_AT && pk(p) == T_NUM) r = parse_shape(p, 0);   /* @ 64 64 */
    else r = parse_expr(p, lv + 1);
    if (!r) return NULL;
    Node *n;
    if (k == T_BAR)      n = node_new(p->a, N_OR, line);
    else if (k == T_AMP) n = node_new(p->a, N_AND, line);
    else if (k == T_AT)  n = node_new(p->a, N_SCOPE, line);
    else if (cmpop(k))   { n = node_new(p->a, N_CMP, line); n->op = cmpop(k); }
    else                 { n = node_new(p->a, N_ARITH, line); n->op = arithop(k); }
    node_addkid(p->a, n, l);
    node_addkid(p->a, n, r);
    if (k == T_AT && pk(p) == T_ATKW) {       /* mask @ frame at origin: anchored frame */
      adv(p);
      Node *org = parse_expr(p, 10);
      if (!org) return NULL;
      node_addkid(p->a, n, org);
    }
    l = n;
  }
}

/* Inputs: parser, min level. Output: full expression at that level.
 * Invariant: depth-capped, so a degenerate paren tower is a parse error, never
 * stack exhaustion. */
static Node *parse_expr(P *p, int min) {
  if (p->depth >= 4096) return perrf(p, tline(p), "expression nested too deeply");
  p->depth++;
  Node *l = parse_prefix(p, min);
  Node *r = l ? parse_binloop(p, l, min) : NULL;
  p->depth--;
  return r;
}

/* ---------- effects ---------- */

/* Input: parser at an effect head. Output: one effect node per GRAMMAR.md level 4. */
static Node *parse_effect(P *p) {
  int line = tline(p);
  switch (pk(p)) {
    case T_PLUS: case T_MINUS: {
      NodeKind nk = pk(p) == T_PLUS ? N_EADD : N_EDEL;
      adv(p);
      if (pk(p) != T_NAME) return perrf(p, tline(p), "expected component name");
      Node *n = node_new(p->a, nk, line);
      setname(n, tname(p)); adv(p);
      return n;
    }
    case T_TILDE: adv(p); return node_new(p->a, N_EDESPAWN, line);
    case T_SPAWN: {
      adv(p);
      Node *what;
      if (pk(p) == T_NAME) what = pk2(p, 1) == T_LP ? parse_call(p) : mkname(p);
      else if (pk(p) == T_LP) what = parse_paren(p);
      else return perrf(p, tline(p), "expected prototype after 'spawn'");
      if (!what) return NULL;
      Node *count = NULL, *pos = NULL;
      if (pk(p) == T_STAR) { adv(p); count = parse_expr(p, 10); if (!count) return NULL; }
      if (pk(p) == T_ATKW) { adv(p); pos = parse_expr(p, 10); if (!pos) return NULL; }
      Node *n = node_new(p->a, N_ESPAWN, line);
      node_addkid(p->a, n, what);
      node_addkid(p->a, n, count);
      node_addkid(p->a, n, pos);
      return n;
    }
    case T_NAME: {
      Node *tgt = mkname(p);
      if (pk(p) == T_VIA) {                            /* fn via Col */
        adv(p);
        if (pk(p) != T_NAME) return perrf(p, tline(p), "expected relation after 'via'");
        Node *n = node_new(p->a, N_EVIA, line);
        n->name = tgt->name;
        node_addkid(p->a, n, mkname(p));
        return n;
      }
      while (pk(p) == T_DOT) {                         /* pos.x target chain */
        adv(p);
        if (pk(p) != T_NAME) return perrf(p, tline(p), "expected field after '.'");
        Node *h = node_new(p->a, N_HOP, line);
        node_addkid(p->a, h, tgt);
        node_addkid(p->a, h, mkname(p));
        tgt = h;
      }
      char op = assignop(pk(p));
      if (op) {
        adv(p);
        Node *rhs = pk(p) == T_TO ? parse_to(p) : parse_expr(p, 4);
        if (!rhs) return NULL;
        Node *n = node_new(p->a, N_EASSIGN, line);
        n->op = op;
        node_addkid(p->a, n, tgt);
        node_addkid(p->a, n, rhs);
        return n;
      }
      if (tgt->kind == N_NAME) {                       /* registered verb */
        Node *n = node_new(p->a, N_EVERB, line);
        n->name = tgt->name;
        while (atomstart(pk(p))) {
          Node *arg = parse_expr(p, 10);
          if (!arg) return NULL;
          node_addkid(p->a, n, arg);
        }
        return n;
      }
      return perrf(p, tline(p), "expected assignment after target");
    }
    default: return perrf(p, line, "expected effect");
  }
}

/* Inputs: parser, statement node. Output: 0 with ';'-separated effects appended / -1. */
static int parse_effects(P *p, Node *stmt) {
  for (;;) {
    Node *e = parse_effect(p);
    if (!e) return -1;
    node_addkid(p->a, stmt, e);
    if (pk(p) == T_SEMI) { adv(p); continue; }
    return 0;
  }
}

/* ---------- statements ---------- */

/* Input: parser at T_LB. Output: N_COMPR: sel, effect, then binders and filters. */
static Node *parse_compr(P *p) {
  int line = tline(p);
  adv(p);
  Node *c = node_new(p->a, N_COMPR, line);
  Node *sel = parse_expr(p, 5);
  if (!sel) return NULL;
  node_addkid(p->a, c, sel);
  if (expect(p, T_COMMA, "',' before comprehension effect")) return NULL;
  Node *eff = parse_effect(p);
  if (!eff) return NULL;
  node_addkid(p->a, c, eff);
  if (expect(p, T_BAR, "'|' before comprehension binders")) return NULL;
  for (;;) {
    if (pk(p) == T_NAME && pk2(p, 1) == T_LARROW) {
      Node *b = node_new(p->a, N_BINDER, tline(p));
      setname(b, tname(p));
      adv(p); adv(p);
      Node *src = parse_expr(p, 5);
      if (!src) return NULL;
      node_addkid(p->a, b, src);
      node_addkid(p->a, c, b);
    } else {
      Node *f = parse_expr(p, 5);
      if (!f) return NULL;
      node_addkid(p->a, c, f);
    }
    if (pk(p) == T_COMMA) { adv(p); continue; }
    break;
  }
  if (expect(p, T_RB, "']'")) return NULL;
  return c;
}

/* Input: parser at the first token of a line (never NL/EOF). Output: one statement:
 * N_DEFSTMT | N_STMT (F_RULE/F_CONT/F_ELIDED) | N_QUERY | N_COMPR. */
static Node *parse_stmt(P *p) {
  int line = tline(p);
  TokKind k = pk(p);
  if (k == T_DEF) {
    adv(p);
    if (pk(p) != T_NAME) return perrf(p, tline(p), "expected name after 'def'");
    /* the closed grammar outranks all names, defs included: a def named for a reserved
     * word is a name the lexer resolves first, unreachable on the JA surface where it is
     * the numeral/particle — bar it on both, the §2 law applied past the loader */
    if (lex_reserved(tname(p)))
      return perrf(p, tline(p), "'%s' is lexer-reserved and cannot name a def", tname(p));
    Node *d = node_new(p->a, N_DEFSTMT, line);
    setname(d, tname(p)); adv(p);
    if (expect(p, T_EQ, "'=' after def name")) return NULL;
    Node *body = parse_expr(p, 2);
    if (!body) return NULL;
    if (pk(p) == T_ARROW) {                            /* def name = sel => effects */
      adv(p);
      Node *st = node_new(p->a, N_STMT, line);
      st->flags |= F_RULE;
      node_addkid(p->a, st, body);
      if (parse_effects(p, st)) return NULL;
      body = st;
    }
    node_addkid(p->a, d, body);
    return d;
  }
  if (k == T_TILDE && (pk2(p, 1) == T_NL || pk2(p, 1) == T_EOF)) {  /* lone ~ */
    adv(p);
    Node *st = node_new(p->a, N_STMT, line);
    st->flags |= F_CONT;
    node_addkid(p->a, st, NULL);
    node_addkid(p->a, st, node_new(p->a, N_EDESPAWN, line));
    return st;
  }
  if (k == T_COMMA) {                                  /* leading-comma continuation */
    adv(p);
    Node *st = node_new(p->a, N_STMT, line);
    st->flags |= F_CONT;
    node_addkid(p->a, st, NULL);
    if (parse_effects(p, st)) return NULL;
    return st;
  }
  if (k == T_LB) return parse_compr(p);
  /* eval "<statement>" — APL's ⍎ constrained to a literal: the quotation is re-lexed and
   * spliced HERE, at parse time, so the spliced statement's footprint stays visible to
   * every later static check. One statement per quotation; dynamic strings are not this. */
  if (k == T_NAME && !strcmp(tname(p), "eval") && pk2(p, 1) == T_STR &&
      (pk2(p, 2) == T_NL || pk2(p, 2) == T_EOF)) {
    const char *quoted = p->t->name[p->i + 1];
    adv(p); adv(p);
    Toks ts = {0};
    if (ano_lex(quoted, 0, p->a, &ts, p->err, p->errsz)) return NULL;
    P q = { &ts, 0, p->a, p->err, p->errsz, 0 };
    while (pk(&q) == T_NL) adv(&q);
    if (pk(&q) == T_EOF) return perrf(p, line, "eval of an empty quotation");
    Node *s = parse_stmt(&q);
    if (!s) return NULL;
    while (pk(&q) == T_NL) adv(&q);
    if (pk(&q) != T_EOF) return perrf(p, line, "eval: one statement per quotation");
    return s;
  }
  if (k == T_SPAWN ||
      ((k == T_PLUS || k == T_MINUS) && pk2(p, 1) == T_NAME &&
       (pk2(p, 2) == T_NL || pk2(p, 2) == T_SEMI || pk2(p, 2) == T_EOF))) {
    Node *st = node_new(p->a, N_STMT, line);           /* elided subject */
    st->flags |= F_ELIDED;
    node_addkid(p->a, st, NULL);
    if (parse_effects(p, st)) return NULL;
    return st;
  }
  /* source-position specials, then the selection expression */
  Node *sel = NULL;
  if (k == T_STR && pk2(p, 1) == T_TO) {               /* "glyphs" to 8 8 */
    Node *s = node_new(p->a, N_STR, line);
    setname(s, tname(p)); adv(p);
    Node *to = parse_to(p);
    if (!to) return NULL;
    node_addkid(p->a, to, s);
    sel = to;
  } else if (k == T_NUM && pk2(p, 1) == T_NUM) {       /* 8 8 lattice */
    sel = parse_shape(p, 0);
  } else if (k == T_NUM &&
             (pk2(p, 1) == T_COMMA || pk2(p, 1) == T_AMP || pk2(p, 1) == T_ARROW)) {
    sel = parse_shape(p, 0);                           /* 12 line */
  }
  if (sel) sel = parse_binloop(p, sel, 2);
  else sel = parse_expr(p, 2);
  if (!sel) return NULL;
  if (pk(p) == T_COMMA || pk(p) == T_ARROW) {
    Node *st = node_new(p->a, N_STMT, line);
    if (pk(p) == T_ARROW) st->flags |= F_RULE;
    adv(p);
    node_addkid(p->a, st, sel);
    if (parse_effects(p, st)) return NULL;
    return st;
  }
  if (pk(p) == T_NL || pk(p) == T_EOF) {
    Node *q = node_new(p->a, N_QUERY, line);
    node_addkid(p->a, q, sel);
    return q;
  }
  if (sel->kind == N_NAME && atomstart(pk(p))) {       /* juxtaposed verb: elided N_EVERB */
    Node *st = node_new(p->a, N_STMT, line);
    st->flags |= F_ELIDED;
    node_addkid(p->a, st, NULL);
    Node *v = node_new(p->a, N_EVERB, sel->line);
    v->name = sel->name;
    while (atomstart(pk(p))) {
      Node *arg = parse_expr(p, 10);
      if (!arg) return NULL;
      node_addkid(p->a, v, arg);
    }
    node_addkid(p->a, st, v);
    while (pk(p) == T_SEMI) {
      adv(p);
      Node *e = parse_effect(p);
      if (!e) return NULL;
      node_addkid(p->a, st, e);
    }
    return st;
  }
  return perrf(p, tline(p), "unexpected token after selection");
}

/* ---------- entry ---------- */

/* Inputs: token stream (T_EOF-terminated or not), arena, err buffer.
 * Output: N_PROGRAM of statements, or NULL with err set. Splices continuation
 * lines (NL run followed by T_TO or T_PIPEGT) before classifying — a per-column
 * gather into fresh arrays, never a struct move. */
Node *ano_parse(const Toks *toks, Arena *a, char *err, size_t errsz) {
  if (err && errsz) err[0] = 0;
  int ntoks = toks->n;
  Toks ts;
  ts.kind = (TokKind *)arena_alloc(a, ((size_t)ntoks + 1) * sizeof *ts.kind);
  ts.name = (const char **)arena_alloc(a, ((size_t)ntoks + 1) * sizeof *ts.name);
  ts.num = (double *)arena_alloc(a, ((size_t)ntoks + 1) * sizeof *ts.num);
  ts.line = (int *)arena_alloc(a, ((size_t)ntoks + 1) * sizeof *ts.line);
  int m = 0;
  for (int i = 0; i < ntoks; i++) {
    TokKind k = toks->kind[i];
    if (k == T_EOF) break;
    if (k == T_NL) {
      int j = i;
      while (j < ntoks && toks->kind[j] == T_NL) j++;
      if (j < ntoks && (toks->kind[j] == T_TO || toks->kind[j] == T_PIPEGT)) { i = j - 1; continue; }
      ts.kind[m] = T_NL; ts.name[m] = ""; ts.num[m] = 0; ts.line[m] = toks->line[i];
      m++;                                             /* collapse the run to one NL */
      i = j - 1;
      continue;
    }
    ts.kind[m] = k; ts.name[m] = toks->name[i]; ts.num[m] = toks->num[i]; ts.line[m] = toks->line[i];
    m++;
  }
  ts.kind[m] = T_EOF; ts.name[m] = ""; ts.num[m] = 0; ts.line[m] = m ? ts.line[m - 1] : 1;
  m++;
  ts.n = m;
  P p = { &ts, 0, a, err, errsz, 0 };
  Node *prog = node_new(a, N_PROGRAM, 1);
  for (;;) {
    while (pk(&p) == T_NL) adv(&p);
    if (pk(&p) == T_EOF) break;
    Node *s = parse_stmt(&p);
    if (!s) { if (err && errsz && !err[0]) snprintf(err, errsz, "line %d: parse error", tline(&p)); return NULL; }
    node_addkid(a, prog, s);
    if (pk(&p) == T_NL) adv(&p);
    else if (pk(&p) != T_EOF) return perrf(&p, tline(&p), "trailing tokens on line");
  }
  return prog;
}

/* ---------- self-test ---------- */
#ifdef PARSE_TEST

/* stub for standalone compilation: the eval splice needs the real lexer (link lex.c);
 * no self-test case quotes a statement */
int ano_lex(const char *src, int ja, Arena *a,
            Toks *toks, char *err, size_t errsz) {
  (void)src; (void)ja; (void)a; (void)toks;
  snprintf(err, errsz, "ano_lex stub (PARSE_TEST)");
  return -1;
}
/* stub: no self-test case defs a reserved word, so nothing is lexer-owned here */
int lex_reserved(const char *w) { (void)w; return 0; }

/* case tables stay array-of-structs for literal ergonomics; adapted per run */
typedef struct { TokKind kind; const char *name; double num; int line; } Tok;

/* Inputs: AoS case table, count, arena. Output: the SoA stream the parser takes. */
static Toks toks_of(const Tok *t, int n, Arena *a) {
  Toks s;
  s.n = n;
  s.kind = (TokKind *)arena_alloc(a, (size_t)n * sizeof *s.kind);
  s.name = (const char **)arena_alloc(a, (size_t)n * sizeof *s.name);
  s.num = (double *)arena_alloc(a, (size_t)n * sizeof *s.num);
  s.line = (int *)arena_alloc(a, (size_t)n * sizeof *s.line);
  for (int i = 0; i < n; i++) {
    s.kind[i] = t[i].kind; s.name[i] = t[i].name; s.num[i] = t[i].num; s.line[i] = t[i].line;
  }
  return s;
}

/* Input: node kind. Output: static name string (enum order of ano.h). */
static const char *kindname(NodeKind k) {
  static const char *names[] = {
    "NUM", "COUNTER", "SYM", "STR", "NAME", "ALIAS", "WILD",
    "NOT", "AND", "OR", "CMP", "ARITH", "SCOPE", "HOP", "SETHOP", "CALL",
    "FOLD", "SCANEXPR", "SCANALONG", "IOTAX", "SHAPE", "TUPLE", "TO",
    "GRADE", "TOP", "PIPE", "ORDERBY", "TAKE", "EXPAND", "CROSSV", "BINDER",
    "EASSIGN", "EADD", "EDEL", "EDESPAWN", "ESPAWN", "EVERB", "EVIA",
    "STMT", "DEFSTMT", "QUERY", "COMPR", "PROGRAM"
  };
  return names[k];
}

/* Inputs: out buffer, node (NULL prints "()"). Output: S-expression appended:
 * (KIND[:FLAGS] [op] [name] [num] kids...). */
static void sx(StrBuf *b, const Node *n) {
  if (!n) { sb_printf(b, "()"); return; }
  sb_printf(b, "(%s", kindname(n->kind));
  if (n->flags & F_RULE)   sb_printf(b, ":RULE");
  if (n->flags & F_CONT)   sb_printf(b, ":CONT");
  if (n->flags & F_ELIDED) sb_printf(b, ":ELIDED");
  if (n->flags & F_DESC)   sb_printf(b, ":DESC");
  if (n->flags & F_SCAN2)  sb_printf(b, ":SCAN2");
  if (n->op) sb_printf(b, " %c", n->op);
  if (n->name[0]) sb_printf(b, " %s", n->name);
  if (n->kind == N_NUM || n->kind == N_COUNTER || n->kind == N_TOP || n->kind == N_TAKE)
    sb_printf(b, " %g", n->num);
  for (int i = 0; i < n->nkids; i++) { sb_printf(b, " "); sx(b, n->kids[i]); }
  sb_printf(b, ")");
}

#define TK(k)       {k, "", 0, 1}
#define TN(s)       {T_NAME, s, 0, 1}
#define TV(v)       {T_NUM, "", v, 1}
#define TS(s)       {T_SYM, s, 0, 1}
#define TKN(k, s)   {k, s, 0, 1}
#define TKNV(k,s,v) {k, s, v, 1}

/* Nord & TwoHanded > 60 , Gold += 1000 */
static const Tok t01[] = { TN("Nord"), TK(T_AMP), TN("TwoHanded"), TK(T_GT), TV(60),
  TK(T_COMMA), TN("Gold"), TK(T_PLUSEQ), TV(1000), TK(T_EOF) };
static const char *w01 = "(PROGRAM (STMT (AND (NAME Nord) (CMP > (NAME TwoHanded) (NUM 60))) (EASSIGN + (NAME Gold) (NUM 1000))))";

/* Bandit & !Dead & Faction == :Bandit , Faction = :Hostile */
static const Tok t02[] = { TN("Bandit"), TK(T_AMP), TK(T_BANG), TN("Dead"), TK(T_AMP),
  TN("Faction"), TK(T_EQEQ), TS("Bandit"), TK(T_COMMA), TN("Faction"), TK(T_EQ), TS("Hostile"), TK(T_EOF) };
static const char *w02 = "(PROGRAM (STMT (AND (AND (NAME Bandit) (NOT (NAME Dead))) (CMP = (NAME Faction) (SYM Bandit))) (EASSIGN = (NAME Faction) (SYM Hostile))))";

/* Frenzy.targets' , +Frenzied */
static const Tok t03[] = { TN("Frenzy"), TK(T_DOT), TN("targets"), TK(T_TICK),
  TK(T_COMMA), TK(T_PLUS), TN("Frenzied"), TK(T_EOF) };
static const char *w03 = "(PROGRAM (STMT (HOP (NAME Frenzy) (SETHOP targets (NAME targets))) (EADD Frenzied)))";

/* ^cursor , Knockback 5 ; Flash :Red ; -Shielded */
static const Tok t04[] = { TKN(T_ALIAS, "cursor"), TK(T_COMMA), TN("Knockback"), TV(5),
  TK(T_SEMI), TN("Flash"), TS("Red"), TK(T_SEMI), TK(T_MINUS), TN("Shielded"), TK(T_EOF) };
static const char *w04 = "(PROGRAM (STMT (ALIAS cursor) (EVERB Knockback (NUM 5)) (EVERB Flash (SYM Red)) (EDEL Shielded)))";

/* +/ Gold @ Nord */
static const Tok t05[] = { TKN(T_FOLD, "+"), TN("Gold"), TK(T_AT), TN("Nord"), TK(T_EOF) };
static const char *w05 = "(PROGRAM (QUERY (FOLD + (SCOPE (NAME Gold) (NAME Nord)))))";

/* Plot & !Planted & #/ (neighbors' & Planted) >= 2 , +Planted */
static const Tok t06[] = { TN("Plot"), TK(T_AMP), TK(T_BANG), TN("Planted"), TK(T_AMP),
  TKN(T_FOLD, "#"), TK(T_LP), TN("neighbors"), TK(T_TICK), TK(T_AMP), TN("Planted"), TK(T_RP),
  TK(T_GE), TV(2), TK(T_COMMA), TK(T_PLUS), TN("Planted"), TK(T_EOF) };
static const char *w06 = "(PROGRAM (STMT (AND (AND (NAME Plot) (NOT (NAME Planted))) (CMP g (FOLD # (AND (SETHOP neighbors (NAME neighbors)) (NAME Planted))) (NUM 2))) (EADD Planted)))";

/* top 5 (grade desc Threat) , +Targeted */
static const Tok t07[] = { TK(T_TOP), TV(5), TK(T_LP), TK(T_GRADE), TK(T_DESC), TN("Threat"),
  TK(T_RP), TK(T_COMMA), TK(T_PLUS), TN("Targeted"), TK(T_EOF) };
static const char *w07 = "(PROGRAM (STMT (TOP 5 (GRADE:DESC (NAME Threat))) (EADD Targeted)))";

/* Enemy |> order by Threat desc |> take 5 , +Targeted */
static const Tok t08[] = { TN("Enemy"), TK(T_PIPEGT), TK(T_ORDER), TK(T_BY), TN("Threat"),
  TK(T_DESC), TK(T_PIPEGT), TK(T_TAKE), TV(5), TK(T_COMMA), TK(T_PLUS), TN("Targeted"), TK(T_EOF) };
static const char *w08 = "(PROGRAM (STMT (PIPE (NAME Enemy) (ORDERBY:DESC (NAME Threat)) (TAKE 5)) (EADD Targeted)))";

/* +/ threat @ (Enemy |> order by dps desc |> take 10) */
static const Tok t09[] = { TKN(T_FOLD, "+"), TN("threat"), TK(T_AT), TK(T_LP), TN("Enemy"),
  TK(T_PIPEGT), TK(T_ORDER), TK(T_BY), TN("dps"), TK(T_DESC), TK(T_PIPEGT), TK(T_TAKE), TV(10),
  TK(T_RP), TK(T_EOF) };
static const char *w09 = "(PROGRAM (QUERY (FOLD + (SCOPE (NAME threat) (PIPE (NAME Enemy) (ORDERBY:DESC (NAME dps)) (TAKE 10))))))";

/* Unit , Slot = rank(Initiative) */
static const Tok t10[] = { TN("Unit"), TK(T_COMMA), TN("Slot"), TK(T_EQ), TN("rank"),
  TK(T_LP), TN("Initiative"), TK(T_RP), TK(T_EOF) };
static const char *w10 = "(PROGRAM (STMT (NAME Unit) (EASSIGN = (NAME Slot) (CALL rank (NAME Initiative)))))";

/* 12 , offset = prev.offset + prev.prev.offset \n , spawn Cheese at Player.pos + (offset, 0) */
static const Tok t11[] = { TV(12), TK(T_COMMA), TN("offset"), TK(T_EQ),
  TN("prev"), TK(T_DOT), TN("offset"), TK(T_PLUS),
  TN("prev"), TK(T_DOT), TN("prev"), TK(T_DOT), TN("offset"), TK(T_NL),
  TK(T_COMMA), TK(T_SPAWN), TN("Cheese"), TK(T_ATKW),
  TN("Player"), TK(T_DOT), TN("pos"), TK(T_PLUS),
  TK(T_LP), TN("offset"), TK(T_COMMA), TV(0), TK(T_RP), TK(T_EOF) };
static const char *w11 = "(PROGRAM (STMT (SHAPE (NUM 12)) (EASSIGN = (NAME offset) (ARITH + (HOP (NAME prev) (NAME offset)) (HOP (HOP (NAME prev) (NAME prev)) (NAME offset))))) (STMT:CONT () (ESPAWN (NAME Cheese) () (ARITH + (HOP (NAME Player) (NAME pos)) (TUPLE (NAME offset) (NUM 0))))))";

/* Soldier , pos = to 4 _ */
static const Tok t12[] = { TN("Soldier"), TK(T_COMMA), TN("pos"), TK(T_EQ), TK(T_TO),
  TV(4), TK(T_WILD), TK(T_EOF) };
static const char *w12 = "(PROGRAM (STMT (NAME Soldier) (EASSIGN = (NAME pos) (TO (SHAPE (NUM 4) (WILD))))))";

/* 8 8 & (x + y) % 2 == 0 , spawn Wheat */
static const Tok t13[] = { TV(8), TV(8), TK(T_AMP), TK(T_LP), TN("x"), TK(T_PLUS), TN("y"),
  TK(T_RP), TK(T_PCT), TV(2), TK(T_EQEQ), TV(0), TK(T_COMMA), TK(T_SPAWN), TN("Wheat"), TK(T_EOF) };
static const char *w13 = "(PROGRAM (STMT (AND (SHAPE (NUM 8) (NUM 8)) (CMP = (ARITH % (ARITH + (NAME x) (NAME y)) (NUM 2)) (NUM 0))) (ESPAWN (NAME Wheat) () ())))";

/* "RNBQ" \n to 8 8 , spawn (pieceOf char)   — exercises the NL-before-to splice */
static const Tok t14[] = { TKN(T_STR, "RNBQ"), TK(T_NL), TK(T_TO), TV(8), TV(8),
  TK(T_COMMA), TK(T_SPAWN), TK(T_LP), TN("pieceOf"), TN("char"), TK(T_RP), TK(T_EOF) };
static const char *w14 = "(PROGRAM (STMT (TO (SHAPE (NUM 8) (NUM 8)) (STR RNBQ)) (ESPAWN (CALL pieceOf (NAME char)) () ())))";

/* Nord & Dead , spawn Ghost \n ~ */
static const Tok t15[] = { TN("Nord"), TK(T_AMP), TN("Dead"), TK(T_COMMA), TK(T_SPAWN),
  TN("Ghost"), TK(T_NL), TK(T_TILDE), TK(T_EOF) };
static const char *w15 = "(PROGRAM (STMT (AND (NAME Nord) (NAME Dead)) (ESPAWN (NAME Ghost) () ())) (STMT:CONT () (EDESPAWN)))";

/* [ t & c , +InRange | t <- Tower, c <- Creep, dist(t, c) < 50 ] */
static const Tok t16[] = { TK(T_LB), TN("t"), TK(T_AMP), TN("c"), TK(T_COMMA),
  TK(T_PLUS), TN("InRange"), TK(T_BAR),
  TN("t"), TK(T_LARROW), TN("Tower"), TK(T_COMMA),
  TN("c"), TK(T_LARROW), TN("Creep"), TK(T_COMMA),
  TN("dist"), TK(T_LP), TN("t"), TK(T_COMMA), TN("c"), TK(T_RP), TK(T_LT), TV(50),
  TK(T_RB), TK(T_EOF) };
static const char *w16 = "(PROGRAM (COMPR (AND (NAME t) (NAME c)) (EADD InRange) (BINDER t (NAME Tower)) (BINDER c (NAME Creep)) (CMP < (CALL dist (NAME t) (NAME c)) (NUM 50))))";

/* def threat = Damage * Speed / Range */
static const Tok t17[] = { TK(T_DEF), TN("threat"), TK(T_EQ), TN("Damage"), TK(T_STAR),
  TN("Speed"), TK(T_SLASH), TN("Range"), TK(T_EOF) };
static const char *w17 = "(PROGRAM (DEFSTMT threat (ARITH / (ARITH * (NAME Damage) (NAME Speed)) (NAME Range))))";

/* def spread = Plot & p => +Planted */
static const Tok t18[] = { TK(T_DEF), TN("spread"), TK(T_EQ), TN("Plot"), TK(T_AMP), TN("p"),
  TK(T_ARROW), TK(T_PLUS), TN("Planted"), TK(T_EOF) };
static const char *w18 = "(PROGRAM (DEFSTMT spread (STMT:RULE (AND (NAME Plot) (NAME p)) (EADD Planted))))";

/* Hostile , shortestPath via Adj */
static const Tok t19[] = { TN("Hostile"), TK(T_COMMA), TN("shortestPath"), TK(T_VIA),
  TN("Adj"), TK(T_EOF) };
static const char *w19 = "(PROGRAM (STMT (NAME Hostile) (EVIA shortestPath (NAME Adj))))";

/* 64 64 & top 5 (grade desc Resource) , +MiningNode */
static const Tok t20[] = { TV(64), TV(64), TK(T_AMP), TK(T_TOP), TV(5), TK(T_LP),
  TK(T_GRADE), TK(T_DESC), TN("Resource"), TK(T_RP), TK(T_COMMA), TK(T_PLUS),
  TN("MiningNode"), TK(T_EOF) };
static const char *w20 = "(PROGRAM (STMT (AND (SHAPE (NUM 64) (NUM 64)) (TOP 5 (GRADE:DESC (NAME Resource)))) (EADD MiningNode)))";

/* order by threat desc */
static const Tok t21[] = { TK(T_ORDER), TK(T_BY), TN("threat"), TK(T_DESC), TK(T_EOF) };
static const char *w21 = "(PROGRAM (QUERY (ORDERBY:DESC (NAME threat))))";

/* Cell @ row , Height = prev.Height + prev.prev.Height */
static const Tok t22[] = { TN("Cell"), TK(T_AT), TN("row"), TK(T_COMMA), TN("Height"),
  TK(T_EQ), TN("prev"), TK(T_DOT), TN("Height"), TK(T_PLUS),
  TN("prev"), TK(T_DOT), TN("prev"), TK(T_DOT), TN("Height"), TK(T_EOF) };
static const char *w22 = "(PROGRAM (STMT (SCOPE (NAME Cell) (NAME row)) (EASSIGN = (NAME Height) (ARITH + (HOP (NAME prev) (NAME Height)) (HOP (HOP (NAME prev) (NAME prev)) (NAME Height))))))";

/* scan(+) Weight along pathCells */
static const Tok t23[] = { TK(T_SCANKW), TK(T_LP), TK(T_PLUS), TK(T_RP), TN("Weight"),
  TK(T_ALONG), TN("pathCells"), TK(T_EOF) };
static const char *w23 = "(PROGRAM (QUERY (SCANALONG + (NAME Weight) (NAME pathCells))))";

/* (Nord, TwoHanded _) , +Trained */
static const Tok t24[] = { TK(T_LP), TN("Nord"), TK(T_COMMA), TN("TwoHanded"), TK(T_WILD),
  TK(T_RP), TK(T_COMMA), TK(T_PLUS), TN("Trained"), TK(T_EOF) };
static const char *w24 = "(PROGRAM (STMT (TUPLE (NAME Nord) (CMP _ (NAME TwoHanded))) (EADD Trained)))";

/* spawn Wheat */
static const Tok t25[] = { TK(T_SPAWN), TN("Wheat"), TK(T_EOF) };
static const char *w25 = "(PROGRAM (STMT:ELIDED () (ESPAWN (NAME Wheat) () ())))";

/* Cheese @ cellar & Aged > 3mo , Price *= 2 */
static const Tok t26[] = { TN("Cheese"), TK(T_AT), TN("cellar"), TK(T_AMP), TN("Aged"),
  TK(T_GT), TKNV(T_COUNTER, "mo", 3), TK(T_COMMA), TN("Price"), TK(T_STAREQ), TV(2), TK(T_EOF) };
static const char *w26 = "(PROGRAM (STMT (AND (SCOPE (NAME Cheese) (NAME cellar)) (CMP > (NAME Aged) (COUNTER mo 3))) (EASSIGN * (NAME Price) (NUM 2))))";

/* max\ Height @ (Eye + til n * north) */
static const Tok t27[] = { TKN(T_SCANOP, "max"), TN("Height"), TK(T_AT), TK(T_LP), TN("Eye"),
  TK(T_PLUS), TK(T_IOTA), TN("n"), TK(T_STAR), TN("north"), TK(T_RP), TK(T_EOF) };
static const char *w27 = "(PROGRAM (QUERY (SCANEXPR max (SCOPE (NAME Height) (ARITH + (NAME Eye) (ARITH * (IOTAX (NAME n)) (NAME north)))))))";

/* +/ Elevation @ 64 64 */
static const Tok t28[] = { TKN(T_FOLD, "+"), TN("Elevation"), TK(T_AT), TV(64), TV(64), TK(T_EOF) };
static const char *w28 = "(PROGRAM (QUERY (FOLD + (SCOPE (NAME Elevation) (SHAPE (NUM 64) (NUM 64))))))";

/* Spawner |> expand Count , spawn Minion */
static const Tok t29[] = { TN("Spawner"), TK(T_PIPEGT), TK(T_EXPAND), TN("Count"),
  TK(T_COMMA), TK(T_SPAWN), TN("Minion"), TK(T_EOF) };
static const char *w29 = "(PROGRAM (STMT (PIPE (NAME Spawner) (EXPAND (NAME Count))) (ESPAWN (NAME Minion) () ())))";

/* Spawner , spawn Minion * Count */
static const Tok t30[] = { TN("Spawner"), TK(T_COMMA), TK(T_SPAWN), TN("Minion"),
  TK(T_STAR), TN("Count"), TK(T_EOF) };
static const char *w30 = "(PROGRAM (STMT (NAME Spawner) (ESPAWN (NAME Minion) (NAME Count) ())))";

/* fold(threat) Damage @ Enemies */
static const Tok t31[] = { TK(T_FOLDKW), TK(T_LP), TN("threat"), TK(T_RP), TN("Damage"),
  TK(T_AT), TN("Enemies"), TK(T_EOF) };
static const char *w31 = "(PROGRAM (QUERY (FOLD threat (SCOPE (NAME Damage) (NAME Enemies)))))";

/* +\ Weight @ (til steps |> route A B) */
static const Tok t32[] = { TKN(T_SCANOP, "+"), TN("Weight"), TK(T_AT), TK(T_LP), TK(T_IOTA),
  TN("steps"), TK(T_PIPEGT), TN("route"), TN("A"), TN("B"), TK(T_RP), TK(T_EOF) };
static const char *w32 = "(PROGRAM (QUERY (SCANEXPR + (SCOPE (NAME Weight) (PIPE (IOTAX (NAME steps)) (CALL route (NAME A) (NAME B)))))))";

/* Plot , Moisture = avg/ neighbors'.Moisture */
static const Tok t33[] = { TN("Plot"), TK(T_COMMA), TN("Moisture"), TK(T_EQ),
  TKN(T_FOLD, "avg"), TN("neighbors"), TK(T_TICK), TK(T_DOT), TN("Moisture"), TK(T_EOF) };
static const char *w33 = "(PROGRAM (STMT (NAME Plot) (EASSIGN = (NAME Moisture) (FOLD avg (HOP (SETHOP neighbors (NAME neighbors)) (NAME Moisture))))))";

/* Knockback 5   — line-start juxtaposed verb, elided subject */
static const Tok t34[] = { TN("Knockback"), TV(5), TK(T_EOF) };
static const char *w34 = "(PROGRAM (STMT:ELIDED () (EVERB Knockback (NUM 5))))";

/* cross dist Tower Creep */
static const Tok t35[] = { TK(T_CROSS), TN("dist"), TN("Tower"), TN("Creep"), TK(T_EOF) };
static const char *w35 = "(PROGRAM (QUERY (CROSSV dist (NAME Tower) (NAME Creep))))";

typedef struct { const char *label; const Tok *toks; int n; const char *want; } Case;
#define CASE(i, lbl) { lbl, t##i, (int)(sizeof t##i / sizeof t##i[0]), w##i }

/* Inputs: none. Output: exit 0 when every AST matches its expected S-expression. */
int main(void) {
  Case cases[] = {
    CASE(01, "hinge+cmp"),      CASE(02, "not+sym+eq"),    CASE(03, "hop-tick"),
    CASE(04, "verb-batch"),     CASE(05, "fold-scope"),    CASE(06, "count-fold-cmp"),
    CASE(07, "top-grade"),      CASE(08, "pipeline"),      CASE(09, "fold-pipe-paren"),
    CASE(10, "call-rhs"),       CASE(11, "shape+cont"),    CASE(12, "to-wild"),
    CASE(13, "lattice-pred"),   CASE(14, "board-splice"),  CASE(15, "despawn-cont"),
    CASE(16, "comprehension"),  CASE(17, "def-column"),    CASE(18, "def-rule"),
    CASE(19, "via"),            CASE(20, "shape-top"),     CASE(21, "orderby-query"),
    CASE(22, "stencil"),        CASE(23, "scan-along"),    CASE(24, "presence-tuple"),
    CASE(25, "elided-spawn"),   CASE(26, "counter"),       CASE(27, "iota-scan"),
    CASE(28, "at-shape"),       CASE(29, "expand"),        CASE(30, "spawn-mult"),
    CASE(31, "fold-long"),      CASE(32, "iota-pipe"),     CASE(33, "sethop-gather"),
    CASE(34, "verb-line"),      CASE(35, "cross"),
  };
  int fails = 0;
  for (size_t i = 0; i < sizeof cases / sizeof cases[0]; i++) {
    Arena a = {0};
    char err[ANO_ERRSZ] = "";
    StrBuf b = {0};
    Toks ts = toks_of(cases[i].toks, cases[i].n, &a);
    Node *prog = ano_parse(&ts, &a, err, sizeof err);
    if (!prog) {
      printf("FAIL %-16s parse error: %s\n", cases[i].label, err);
      fails++;
    } else {
      sx(&b, prog);
      if (strcmp(b.s, cases[i].want) != 0) {
        printf("FAIL %-16s got:  %s\n     %-16s want: %s\n", cases[i].label, b.s, "", cases[i].want);
        fails++;
      } else {
        printf("ok   %-16s %s\n", cases[i].label, b.s);
      }
    }
    sb_free(&b);
    arena_free(&a);
  }
  printf("%s: %d/%zu passed\n", fails ? "FAIL" : "PASS",
         (int)(sizeof cases / sizeof cases[0]) - fails, sizeof cases / sizeof cases[0]);
  return fails ? 1 : 0;
}
#endif /* PARSE_TEST */
