/* emit.c — AST -> BQN codegen for anoc.
 * One ano statement = one gather-effect-scatter barrier: emit computes the selection
 * mask against pre-state, stages every effect as a new-column temp read only from
 * pre-state, then commits all writes at the end of the statement block. Continuations
 * (~ / leading comma / elided subject) reuse the saved mask anoSel, zero-padded to the
 * current world length, never a re-gather.
 * Conventions the .reg fixtures rely on:
 *   - columns emit as their registry spelling (first letter lowercased by reg_find);
 *     presence masks emit as pres_<name>; set-valued rels emit as fiber lists.
 *   - symbol columns are lists of BQN strings; comparisons are (<"Sym")≡¨col.
 *   - pair-valued columns (vec) are lists of x‿y pairs; expects compare against ∾col.
 *   - registry fns emit as <Name-with-first-letter-uppercased>; raw BQN from the .reg
 *     is bound verbatim. Effect verbs/via register as: fn <name> <targetcol> <dfn>,
 *     called as: target ↩ sel FnName ⟨target, args…⟩. Pipeline call stages and
 *     callables are applied per emitVal (1-arg: Fn¨ a; 2-arg: a Fn¨ b).
 *   - a numeric shape source (12 / 8 8) frames the statement over a line/lattice whose
 *     backing storage is the registry world (n == length) or lattice fields.
 * Inputs: N_PROGRAM, Registry, Directives. Output: BQN appended to out. 0 / -1 + err.
 */
#include "ano.h"
#include <ctype.h>

typedef enum { FR_ENT, FR_LAT, FR_LINE, FR_BOARD } FrameKind;
typedef struct {
  FrameKind kind;
  int w, h;          /* FR_LAT/FR_BOARD dims; FR_LINE: w = length */
  const char *lit;   /* FR_BOARD glyphs (interned; arena lifetime) */
} Frame;

typedef enum { MODE_WORLD, MODE_SEL, MODE_COPY } Mode;

typedef struct { char *v; char *g; int pair; int unit; int sym;
                 char *along;   /* scan-along order expr: values are in along order, and a
                                   write-back must conjugate — sort, act, unsort (Tier 2) */
} EV;

typedef struct {
  const Registry *reg;
  const Directives *dirs;
  StrBuf *out;
  Arena *a;
  char *err; size_t errsz;
  /* defs */
  const Node *defs[128]; int ndefs;
  /* pairness per registry entry (index-aligned), grown knowledge */
  unsigned char isPair[512];
  /* statement state */
  Frame fr, savedFr;
  int haveSaved;
  /* shared rule tick (§11): curRule indexes the rule whose effects are being staged
   * (-1 outside a tick); ruleDisj[r] is the bitmask of rules whose masks are certified
   * row-disjoint from r by complementary guard literals */
  int curRule;
  unsigned ruleDisj[32];
  char selVar[32];       /* current statement's refined selection variable */
  char cntVar[32];       /* copy-space counts (spawn replicate) */
  char idxVar[64];       /* copy-space per-copy index */
  const char *pipeExpand;  /* selection-space counts from a pipeline expand stage (arena) */
  StrBuf pre;            /* staged lines for the current statement */
  int tmp;
  int outIdx;
  int stmt;
  int failed;
} Em;

/* Inputs: emitter, printf format. Output: arena string. Invariant: never NULL. */
static char *efmt(Em *em, const char *fmt, ...) {
  va_list ap; va_start(ap, fmt);
  va_list ap2; va_copy(ap2, ap);
  int need = vsnprintf(NULL, 0, fmt, ap2); va_end(ap2);
  char *s = (char *)arena_alloc(em->a, (size_t)need + 1);
  vsnprintf(s, (size_t)need + 1, fmt, ap); va_end(ap);
  return s;
}

static int fail(Em *em, int line, const char *fmt, ...) {
  va_list ap; va_start(ap, fmt);
  char msg[400]; vsnprintf(msg, sizeof msg, fmt, ap); va_end(ap);
  snprintf(em->err, em->errsz, "emit: line %d: %s", line, msg);
  em->failed = 1;
  return -1;
}

/* fresh temp name */
static char *tv(Em *em) { return efmt(em, "t%d", em->tmp++); }
/* stage a prologue line (appends directly; no fixed line buffer) */
static void stage(Em *em, const char *fmt, ...) {
  va_list ap; va_start(ap, fmt);
  sb_vprintf(&em->pre, fmt, ap);
  va_end(ap);
  sb_printf(&em->pre, "\n");
}

/* registry-var spelling: first letter lowercased */
static char *lc(Em *em, const char *n) {
  char *s = arena_strdup(em->a, n, strlen(n));
  s[0] = (char)tolower((unsigned char)s[0]); return s;
}
static char *uc(Em *em, const char *n) {
  char *s = arena_strdup(em->a, n, strlen(n));
  s[0] = (char)toupper((unsigned char)s[0]); return s;
}
/* registry-fn BQN spelling: Fn_ prefix keeps case-insensitive BQN identifiers from
 * colliding with a column of the same name (fn threat vs col threat) */
static char *fnv(Em *em, const char *n) { return efmt(em, "Fn_%s", n); }

static const RegEntry *find(Em *em, const char *n) { return reg_find(em->reg, n); }
static int entIdx(Em *em, const RegEntry *e) { return (int)(e - em->reg->ents); }

static const Node *findDef(Em *em, const char *n) {
  for (int i = 0; i < em->ndefs; i++)
    if (!strcmp(em->defs[i]->name, n)) return em->defs[i];
  return NULL;
}

/* frame cell count as BQN expr */
static char *frN(Em *em) {
  switch (em->fr.kind) {
    case FR_ENT: return "anoN";
    case FR_LINE: return efmt(em, "%d", em->fr.w);
    default: return efmt(em, "%d", em->fr.w * em->fr.h);
  }
}
/* the world's stable-id column: a registered `id`/`keys` column, else the row iota.
 * Relationship values denote these ids; membership survives structural compaction (ex12). */
static char *idCol(Em *em) {
  const RegEntry *e = reg_find(em->reg, "id");
  if (!e) e = reg_find(em->reg, "keys");
  if (e && e->kind == RK_COL) return lc(em, e->name);
  return efmt(em, "(↕%s)", frN(em));
}

/* guard conjunction; NULL = total */
static char *gAnd(Em *em, char *a, char *b) {
  if (!a) return b;
  if (!b) return a;
  return efmt(em, "(%s)∧(%s)", a, b);
}

static int emitVal(Em *em, const Node *nd, Mode m, EV *ev);
static int emitMask(Em *em, const Node *nd, char **out);

/* number spelling with BQN high-minus */
static char *numLit(Em *em, double x) {
  char buf[64];
  if (x == (long long)x) snprintf(buf, sizeof buf, "%lld", (long long)x);
  else snprintf(buf, sizeof buf, "%.17g", x);
  if (buf[0] == '-') return efmt(em, "¯%s", buf + 1);
  return efmt(em, "%s", buf);
}

/* gather a world-space column expr into the current mode */
static char *inMode(Em *em, char *worldExpr, Mode m) {
  if (m == MODE_WORLD) return worldExpr;
  if (m == MODE_SEL) return efmt(em, "(%s/%s)", em->selVar, worldExpr);
  return efmt(em, "(%s/%s/%s)", em->cntVar, em->selVar, worldExpr);
}

/* presence mask for a column entry, world space; NULL when total */
static char *presOf(Em *em, const RegEntry *e) {
  if (e->kind == RK_COL && e->hasPres) return efmt(em, "pres_%s", e->name);
  return NULL;
}

/* ---------- names as values ---------- */

/* Inputs: name node. Output: EV in mode m. Handles cols, fields, coords, index,
 * defs, bindings, aliases. Invariant: guard is world-space. */
static int emitNameVal(Em *em, const Node *nd, Mode m, EV *ev) {
  const char *n = nd->name;
  memset(ev, 0, sizeof *ev);
  if (!strcmp(n, "index")) {
    if (m == MODE_COPY) { ev->v = efmt(em, "%s", em->idxVar); return 0; }
    if (m == MODE_SEL) { ev->v = efmt(em, "(↕+´%s)", em->selVar); return 0; }
    ev->v = efmt(em, "(↕%s)", frN(em)); return 0;
  }
  if ((em->fr.kind == FR_LAT || em->fr.kind == FR_BOARD) &&
      (!strcmp(n, "x") || !strcmp(n, "y")) && !find(em, n)) {
    char *w = n[0] == 'y' ? efmt(em, "(⌊(↕%d)÷%d)", em->fr.w * em->fr.h, em->fr.w)
                          : efmt(em, "(%d|↕%d)", em->fr.w, em->fr.w * em->fr.h);
    ev->v = inMode(em, w, m); return 0;
  }
  if (!strcmp(n, "char") && em->fr.kind == FR_BOARD) {
    ev->v = inMode(em, efmt(em, "brd%d", em->stmt), m); return 0;
  }
  const Node *d = findDef(em, n);
  if (d) return emitVal(em, d->kids[0], m, ev);
  const RegEntry *e = find(em, n);
  if (!e) return fail(em, nd->line, "unregistered name '%s'", n);
  switch (e->kind) {
    case RK_COL: case RK_FIELD: {
      char *v = lc(em, e->name);
      ev->pair = em->isPair[entIdx(em, e)];
      ev->sym = (e->type == CT_SYM);
      ev->g = presOf(em, e);
      ev->v = inMode(em, v, m);
      return 0;
    }
    case RK_REL: { ev->v = inMode(em, lc(em, e->name), m); ev->g = efmt(em, "(0≤%s)", lc(em, e->name)); return 0; }
    case RK_ALIAS: { ev->v = inMode(em, lc(em, e->name), m); return 0; }
    case RK_BIND:
      if (!strcmp(e->bindKind, "num")) { ev->v = numLit(em, e->nums[0]); ev->unit = 1; return 0; }
      if (!strcmp(e->bindKind, "point")) {
        ev->v = efmt(em, "(<%s‿%s)", numLit(em, e->nums[0]), numLit(em, e->nums[1]));
        ev->pair = 1; ev->unit = 1; return 0;
      }
      if (!strcmp(e->bindKind, "entity")) { ev->v = numLit(em, e->nums[0]); ev->unit = 1; return 0; }
      if (!strcmp(e->bindKind, "mask")) { ev->v = inMode(em, lc(em, e->name), m); return 0; }
      if (!strcmp(e->bindKind, "vec")) { ev->v = lc(em, e->name); ev->unit = 1; return 0; }
      return fail(em, nd->line, "binding '%s' of kind %s in value position", n, e->bindKind);
    case RK_SREL: { ev->v = lc(em, e->name); ev->unit = 1; return 0; }
    default: return fail(em, nd->line, "name '%s' (fn) in value position", n);
  }
}

/* ---------- masks (world space, guards folded in) ---------- */

/* bare name as a mask: bool col -> pres∧values; value col -> presence; alias/bind mask. */
static int emitNameMask(Em *em, const Node *nd, char **out) {
  const char *n = nd->name;
  const Node *d = findDef(em, n);
  if (d) return emitMask(em, d->kids[0], out);
  const RegEntry *e = find(em, n);
  if (!e) return fail(em, nd->line, "unregistered mask name '%s'", n);
  char *v = lc(em, e->name);
  switch (e->kind) {
    case RK_COL: case RK_FIELD:
      if (e->type == CT_BOOL) { *out = e->hasPres ? efmt(em, "(pres_%s∧%s)", e->name, v) : v; return 0; }
      *out = e->hasPres ? efmt(em, "pres_%s", e->name) : efmt(em, "(1¨%s)", v);
      return 0;
    case RK_ALIAS: *out = v; return 0;
    case RK_BIND:
      if (!strcmp(e->bindKind, "mask")) { *out = v; return 0; }
      if (!strcmp(e->bindKind, "entity")) { *out = efmt(em, "((↕anoN)=%s)", numLit(em, e->nums[0])); return 0; }
      return fail(em, nd->line, "binding '%s' (%s) as mask", n, e->bindKind);
    case RK_REL: *out = efmt(em, "(0≤%s)", v); return 0;
    default: return fail(em, nd->line, "'%s' cannot be a mask", n);
  }
}

/* grade ids: returns ids expr (frame space) */
static int emitGradeIds(Em *em, const Node *nd, char **out) {
  const Node *key = nd->kids[0];
  const Node *scope = NULL;
  if (key->kind == N_SCOPE && key->kids[1]->kind == N_SHAPE) { scope = key->kids[1]; key = key->kids[0]; }
  else if (key->kind == N_SCOPE) { scope = key->kids[1]; key = key->kids[0]; }
  const char *g = (nd->flags & F_DESC) ? "⍒" : "⍋";
  if (scope && scope->kind != N_SHAPE) {  /* mask scope: ids in world space */
    char *msk; if (emitMask(em, scope, &msk)) return -1;
    EV kv; if (emitVal(em, key, MODE_WORLD, &kv)) return -1;
    *out = efmt(em, "((/%s)⊏˜%s(%s/%s))", msk, g, msk, kv.v);
    return 0;
  }
  EV kv; if (emitVal(em, key, MODE_WORLD, &kv)) return -1;
  *out = efmt(em, "(%s%s)", g, kv.v);
  return 0;
}

/* pipeline view: base mask + ordered indices into the filtered space (or iota base) */
typedef struct { char *base; char *idx; char *expandCnt; int isIota; } View;

static int emitPipe(Em *em, const Node *nd, View *vw) {
  memset(vw, 0, sizeof *vw);
  const Node *src = nd->kids[0];
  if (src->kind == N_IOTAX) {
    EV n; if (emitVal(em, src->kids[0], MODE_WORLD, &n)) return -1;
    vw->isIota = 1; vw->idx = efmt(em, "(↕%s)", n.v);
  } else {
    char *msk; if (emitMask(em, src, &msk)) return -1;
    char *t = tv(em); stage(em, "%s ← %s", t, msk);
    vw->base = t;
    vw->idx = efmt(em, "(↕+´%s)", t);
  }
  for (int i = 1; i < nd->nkids; i++) {
    const Node *st = nd->kids[i];
    if (st->kind == N_ORDERBY) {
      EV kv; if (emitVal(em, st->kids[0], MODE_WORLD, &kv)) return -1;
      char *keyf = vw->base ? efmt(em, "(%s/%s)", vw->base, kv.v) : kv.v;
      vw->idx = efmt(em, "((%s)⊏˜%s%s)", vw->idx, (st->flags & F_DESC) ? "⍒" : "⍋",
                     efmt(em, "((%s)⊏%s)", vw->idx, keyf));
    } else if (st->kind == N_TAKE) {
      vw->idx = efmt(em, "(%d↑%s)", (int)st->num, vw->idx);
    } else if (st->kind == N_EXPAND) {
      EV cv; if (emitVal(em, st->kids[0], MODE_WORLD, &cv)) return -1;
      vw->expandCnt = vw->base ? efmt(em, "(%s/%s)", vw->base, cv.v) : cv.v;
    } else if (st->kind == N_CALL) {
      char *args = efmt(em, "⟨%s", vw->idx);
      for (int k = 0; k < st->nkids; k++) {
        EV av; if (emitVal(em, st->kids[k], MODE_WORLD, &av)) return -1;
        args = efmt(em, "%s, %s", args, av.v);
      }
      vw->idx = efmt(em, "(%s %s⟩)", fnv(em, st->name), args);
      vw->isIota = 1; vw->base = NULL;
    } else return fail(em, st->line, "unsupported pipeline stage");
  }
  return 0;
}

/* view ids in frame space (world row ids) */
static char *viewWorldIds(Em *em, View *vw) {
  if (vw->base) return efmt(em, "((/%s)⊏˜%s)", vw->base, vw->idx);
  return vw->idx;
}

/* ---------- folds ---------- */

static const char *foldGl(const char *op) {
  if (!strcmp(op, "+")) return "+´";
  if (!strcmp(op, "*")) return "×´";
  if (!strcmp(op, "&")) return "∧´";
  if (!strcmp(op, "|")) return "∨´";
  if (!strcmp(op, "max")) return "⌈´";
  if (!strcmp(op, "min")) return "⌊´";
  return NULL;
}
static int foldHasId(const char *op) {
  return !strcmp(op,"+") || !strcmp(op,"*") || !strcmp(op,"&") || !strcmp(op,"|") || !strcmp(op,"#");
}
/* the set-hop fiber expression for a rel name (forward srel, precomputed inverse, or a
 * key-valued column standing in relation position: its fibers are the inverse image over
 * the stable-id column — the value-level rel, w3-c) */
static int fiberVar(Em *em, const Node *nd, char **out) {
  const RegEntry *e = find(em, nd->name);
  if (e && e->kind == RK_SREL) { *out = lc(em, e->name); return 0; }
  if (e && e->kind == RK_COL && e->type == CT_NUM) {
    char *t = tv(em);
    stage(em, "%s ← {/%s=𝕩}¨%s", t, lc(em, e->name), idCol(em));
    *out = t;
    return 0;
  }
  return fail(em, nd->line, "'%s' is not a set-valued relationship or a key column", nd->name);
}

/* gamma fold: fold/ rel'.Comp | fold/ (rel' & pred) | fold/ rel'  -> per-source column + guard */
static int emitGamma(Em *em, const char *op, const Node *operand, Mode m, EV *ev) {
  memset(ev, 0, sizeof *ev);
  char *fib = NULL, *body = NULL;
  if (operand->kind == N_SETHOP) {           /* #/ attackers' */
    if (fiberVar(em, operand->kids[0], &fib)) return -1;
    body = efmt(em, "≠¨%s", fib);
    if (strcmp(op, "#")) { /* other folds over bare fiber make no sense */
      return fail(em, operand->line, "bare rel' under %s/", op);
    }
    ev->v = inMode(em, efmt(em, "(%s)", body), m);
    return 0;
  }
  if (operand->kind == N_HOP && operand->kids[0]->kind == N_SETHOP) { /* fold/ rel'.Comp */
    if (fiberVar(em, operand->kids[0]->kids[0], &fib)) return -1;
    EV cv; if (emitVal(em, operand->kids[1], MODE_WORLD, &cv)) return -1;
    const char *gl = foldGl(op);
    char *t = tv(em);
    if (!strcmp(op, "avg")) {
      stage(em, "%s ← {0=≠𝕩 ? 0 ; AnoAvg 𝕩⊏%s}¨%s", t, cv.v, fib);
      ev->g = efmt(em, "(0<≠¨%s)", fib);
    } else if (!strcmp(op, "#")) {
      stage(em, "%s ← {+´𝕩⊏%s}¨%s", t, cv.v, fib);
    } else if (gl && foldHasId(op)) {
      stage(em, "%s ← {%s𝕩⊏%s}¨%s", t, gl, cv.v, fib);
    } else if (gl) { /* max/min: no identity, guard empties */
      stage(em, "%s ← {0=≠𝕩 ? 0 ; %s𝕩⊏%s}¨%s", t, gl, cv.v, fib);
      ev->g = efmt(em, "(0<≠¨%s)", fib);
    } else return fail(em, operand->line, "unknown reducer '%s'", op);
    ev->v = inMode(em, t, m);
    if (ev->g) ev->g = ev->g; /* world-space guard */
    return 0;
  }
  if (operand->kind == N_AND && operand->kids[0]->kind == N_SETHOP) { /* fold/ (rel' & pred) */
    if (fiberVar(em, operand->kids[0]->kids[0], &fib)) return -1;
    char *pm; if (emitMask(em, operand->kids[1], &pm)) return -1;
    char *tp = tv(em); stage(em, "%s ← %s", tp, pm);
    char *t = tv(em);
    if (!strcmp(op, "#")) stage(em, "%s ← {+´𝕩⊏%s}¨%s", t, tp, fib);
    else if (!strcmp(op, "|")) stage(em, "%s ← {∨´𝕩⊏%s}¨%s", t, tp, fib);
    else if (!strcmp(op, "&")) stage(em, "%s ← {∧´𝕩⊏%s}¨%s", t, tp, fib);
    else return fail(em, operand->line, "fold %s/ over filtered fiber", op);
    ev->v = inMode(em, t, m);
    return 0;
  }
  return fail(em, operand->line, "unsupported gamma operand");
}

/* does this subtree start a gamma (contains a set hop at fold-operand level)? */
static int isGammaOperand(const Node *nd) {
  if (nd->kind == N_SETHOP) return 1;
  if (nd->kind == N_HOP && nd->kids[0]->kind == N_SETHOP) return 1;
  if (nd->kind == N_AND && nd->kids[0]->kind == N_SETHOP) return 1;
  return 0;
}

/* scoped-global fold -> scalar EV (unit) */
static int emitFold(Em *em, const Node *nd, Mode m, EV *ev) {
  const char *op = nd->name;
  const Node *operand = nd->kids[0];
  memset(ev, 0, sizeof *ev);
  ev->unit = 1;
  /* Adj@row: per-row fold over a fiber-matrix column */
  if (operand->kind == N_SCOPE && operand->kids[1]->kind == N_NAME &&
      !strcmp(operand->kids[1]->name, "row") && !find(em, "row")) {
    char *fib; if (fiberVar(em, operand->kids[0], &fib)) return -1;
    const char *gl = foldGl(op); if (!gl) return fail(em, nd->line, "fold %s/ @row", op);
    ev->unit = 0;
    ev->v = inMode(em, efmt(em, "(%s¨%s)", gl, fib), m);
    return 0;
  }
  if (isGammaOperand(operand)) { ev->unit = 0; return emitGamma(em, op, operand, m, ev); }
  /* split operand @ scope */
  const Node *scope = NULL; const Node *x = operand;
  if (operand->kind == N_SCOPE) { x = operand->kids[0]; scope = operand->kids[1]; }
  char *gathered = NULL; char *cnt = NULL;
  if (!scope) {                       /* #/ (mask) or fold over full frame */
    if (!strcmp(op, "#")) {
      char *msk; if (emitMask(em, x, &msk)) return -1;
      ev->v = efmt(em, "(+´%s)", msk); return 0;
    }
    EV xv; if (emitVal(em, x, MODE_WORLD, &xv)) return -1;
    char *v = xv.g ? efmt(em, "((%s)/%s)", xv.g, xv.v) : xv.v;
    gathered = v; cnt = efmt(em, "(≠%s)", v);
  } else if (scope->kind == N_SHAPE) { /* fold over a lattice field */
    EV xv; if (emitVal(em, x, MODE_WORLD, &xv)) return -1;
    gathered = xv.v; cnt = efmt(em, "(≠%s)", xv.v);
  } else if (scope->kind == N_PIPE) {
    View vw; if (emitPipe(em, scope, &vw)) return -1;
    EV xv; if (emitVal(em, x, MODE_WORLD, &xv)) return -1;
    char *xf = vw.base ? efmt(em, "(%s/%s)", vw.base, xv.v) : xv.v;
    gathered = efmt(em, "((%s)⊏%s)", vw.idx, xf);
    cnt = efmt(em, "(≠%s)", vw.idx);
  } else {                            /* mask scope */
    char *msk; if (emitMask(em, scope, &msk)) return -1;
    EV xv; if (emitVal(em, x, MODE_WORLD, &xv)) return -1;
    char *m2 = xv.g ? gAnd(em, msk, xv.g) : msk;
    if (!strcmp(op, "#")) { ev->v = efmt(em, "(+´%s)", m2); return 0; }
    gathered = efmt(em, "(%s/%s)", m2, xv.v);
    cnt = efmt(em, "(+´%s)", m2);
  }
  if (!strcmp(op, "#")) { ev->v = cnt; return 0; }
  if (!strcmp(op, "avg")) {
    char *t = tv(em);
    stage(em, "%s ← {0=≠𝕩 ? 0 ; AnoAvg 𝕩} %s", t, gathered);
    ev->v = t; ev->g = efmt(em, "(0<%s)", cnt);
    return 0;
  }
  const char *gl = foldGl(op);
  if (!gl) { /* named reducer: registry fn folds pairwise */
    const RegEntry *e = find(em, op);
    if (!e || e->kind != RK_FN) return fail(em, nd->line, "unknown reducer '%s'", op);
    char *t = tv(em);
    stage(em, "%s ← {0=≠𝕩 ? 0 ; %s´ 𝕩} %s", t, fnv(em, e->name), gathered);
    ev->v = t; ev->g = efmt(em, "(0<%s)", cnt);
    return 0;
  }
  if (!foldHasId(op)) {
    char *t = tv(em);
    stage(em, "%s ← {0=≠𝕩 ? 0 ; %s𝕩} %s", t, gl, gathered);
    ev->v = t; ev->g = efmt(em, "(0<%s)", cnt);
    return 0;
  }
  ev->v = efmt(em, "(%s%s)", gl, gathered);
  return 0;
}

/* Inputs: expr subtree. Output: 1 when an ↕ generator appears anywhere in it. */
static int containsIota(const Node *nd) {
  if (nd->kind == N_IOTAX) return 1;
  for (int i = 0; i < nd->nkids; i++)
    if (nd->kids[i] && containsIota(nd->kids[i])) return 1;
  return 0;
}

/* scans: result is an ordered column over the scan's scope, returned as a flat value */
static int emitScan(Em *em, const Node *nd, EV *ev) {
  const char *op = nd->name;
  const Node *operand = nd->kids[0];
  memset(ev, 0, sizeof *ev);
  const Node *scope = NULL; const Node *x = operand;
  if (operand->kind == N_SCOPE) { x = operand->kids[0]; scope = operand->kids[1]; }
  const char *gl = !strcmp(op, "+") ? "+`" : !strcmp(op, "*") ? "×`" : !strcmp(op, "max") ? "⌈`" : NULL;
  if (!gl) return fail(em, nd->line, "unknown scan op '%s'", op);
  EV xv; if (emitVal(em, x, MODE_WORLD, &xv)) return -1;
  if (scope && scope->kind == N_SHAPE) {
    int w = (int)scope->kids[0]->num;
    int h = scope->nkids > 1 ? (int)scope->kids[1]->num : 1;
    if (nd->flags & F_SCAN2)
      ev->v = efmt(em, "(⥊%s˘%s(%d‿%d⥊%s))", gl, gl, h, w, xv.v);
    else
      ev->v = efmt(em, "(⥊%s(%d‿%d⥊%s))", gl, h, w, xv.v);
    return 0;
  }
  if (scope && scope->kind == N_PIPE) {
    View vw; if (emitPipe(em, scope, &vw)) return -1;
    char *xf = vw.base ? efmt(em, "(%s/%s)", vw.base, xv.v) : xv.v;
    ev->v = efmt(em, "(%s(%s)⊏%s)", gl, vw.idx, xf);
    return 0;
  }
  if (scope) {
    /* mask scope -> compress; id-list scope (iota arithmetic / vec bind) -> index */
    int idlist = containsIota(scope);
    if (!idlist && scope->kind == N_NAME) {
      const RegEntry *e = find(em, scope->name);
      if (e && e->kind == RK_BIND && !strcmp(e->bindKind, "vec")) idlist = 1;
    }
    if (idlist) {
      EV sv; if (emitVal(em, scope, MODE_WORLD, &sv)) return -1;
      ev->v = efmt(em, "(%s(%s)⊏%s)", gl, sv.v, xv.v);
      return 0;
    }
    char *msk; if (emitMask(em, scope, &msk)) return -1;
    ev->v = efmt(em, "(%s%s/%s)", gl, msk, xv.v);
    return 0;
  }
  ev->v = efmt(em, "(%s%s)", gl, xv.v);
  return 0;
}

/* ---------- general value expressions ---------- */

static int emitCall(Em *em, const Node *nd, Mode m, EV *ev) {
  memset(ev, 0, sizeof *ev);
  if (!strcmp(nd->name, "rank")) {
    EV a; if (emitVal(em, nd->kids[0], m, &a)) return -1;
    ev->v = efmt(em, "(AnoRank %s)", a.v);
    return 0;
  }
  const RegEntry *e = find(em, nd->name);
  const char *fn = e ? fnv(em, e->name) : uc(em, nd->name);
  if (!e && !strcmp(nd->name, "abs")) fn = "|";
  else if (!e && !strcmp(nd->name, "sin")) fn = "•math.Sin";
  else if (!e) return fail(em, nd->line, "unregistered callable '%s'", nd->name);
  if (nd->nkids == 1) {
    EV a; if (emitVal(em, nd->kids[0], m, &a)) return -1;
    ev->g = a.g; ev->unit = a.unit;
    ev->v = efmt(em, "(%s¨%s)", fn, a.v);
    return 0;
  }
  if (nd->nkids == 2) {
    EV a, b;
    if (emitVal(em, nd->kids[0], m, &a)) return -1;
    if (emitVal(em, nd->kids[1], m, &b)) return -1;
    ev->g = gAnd(em, a.g, b.g); ev->unit = a.unit && b.unit;
    ev->v = efmt(em, "(%s %s¨%s)", a.v, fn, b.v);
    return 0;
  }
  return fail(em, nd->line, "callable arity %d unsupported", nd->nkids);
}

/* hop chains: rel.Comp, Player.pos, ^cursor.pos, prev.prev.X, neighbor(clamp).X, pos.x */
static int emitHop(Em *em, const Node *nd, Mode m, EV *ev) {
  memset(ev, 0, sizeof *ev);
  const Node *base = nd->kids[0], *field = nd->kids[1];
  /* count prev chain */
  int prevs = 0; const Node *b = nd;
  while (b->kind == N_HOP) { b = b->kids[0]; }
  if ((b->kind == N_NAME && !strcmp(b->name, "prev") && !find(em, "prev"))) {
    const Node *w = nd; prevs = 0;
    while (w->kind == N_HOP && w->kids[0]->kind == N_HOP) { prevs++; w = w->kids[0]; }
    prevs++;
    EV cv; if (emitVal(em, field, MODE_WORLD, &cv)) return -1;
    char *sh;
    if (em->fr.kind == FR_LAT || em->fr.kind == FR_BOARD) {
      /* rank-2 prev: » shifts a whole zero row along the leading axis */
      char glyphs[16] = ""; for (int i = 0; i < prevs && i < 7; i++) strcat(glyphs, "»");
      sh = efmt(em, "(⥊%s(%d‿%d⥊%s))", glyphs, em->fr.h, em->fr.w, cv.v);
    } else {
      sh = cv.v;
      for (int i = 0; i < prevs; i++) sh = efmt(em, "(»%s)", sh);
    }
    ev->v = inMode(em, sh, m);
    return 0;
  }
  if (b->kind == N_NAME && !strncmp(b->name, "neighbor", 8) && !find(em, "neighbor")) {
    EV cv; if (emitVal(em, field, MODE_WORLD, &cv)) return -1;
    char *sh;
    if (em->fr.kind == FR_LAT || em->fr.kind == FR_BOARD)
      sh = efmt(em, "(⥊AnoNbrClamp(%d‿%d⥊%s))", em->fr.h, em->fr.w, cv.v);
    else sh = efmt(em, "(AnoNbrClamp %s)", cv.v);
    ev->v = inMode(em, sh, m);
    return 0;
  }
  if (b->kind == N_CALL && !strncmp(b->name, "neighbor", 8)) {
    EV cv; if (emitVal(em, field, MODE_WORLD, &cv)) return -1;
    char *sh = efmt(em, "(⥊AnoNbrClamp(%d‿%d⥊%s))", em->fr.h, em->fr.w, cv.v);
    ev->v = inMode(em, sh, m);
    return 0;
  }
  /* pos.x / pos.y field projection on a pair column */
  if (field->kind == N_NAME && (!strcmp(field->name, "x") || !strcmp(field->name, "y"))) {
    const RegEntry *e = base->kind == N_NAME ? find(em, base->name) : NULL;
    if (e && em->isPair[entIdx(em, e)]) {
      int i = field->name[0] == 'y';
      ev->v = inMode(em, efmt(em, "(%d⊸⊑¨%s)", i, lc(em, e->name)), m);
      return 0;
    }
  }
  /* singleton roots: bindings and aliases mirror-read one row */
  if (base->kind == N_NAME || base->kind == N_ALIAS) {
    const RegEntry *be = find(em, base->name);
    const RegEntry *fe = field->kind == N_NAME ? find(em, field->name) : NULL;
    /* a point binding IS its pos: Player.pos with `bind Player point x y` */
    if (be && be->kind == RK_BIND && !strcmp(be->bindKind, "point")) {
      ev->v = efmt(em, "(<%s‿%s)", numLit(em, be->nums[0]), numLit(em, be->nums[1]));
      ev->pair = 1; ev->unit = 1;
      return 0;
    }
    if (be && fe && (be->kind == RK_BIND || be->kind == RK_ALIAS) &&
        (fe->kind == RK_COL || fe->kind == RK_FIELD)) {
      char *id = be->kind == RK_BIND ? numLit(em, be->nums[0])
                                     : efmt(em, "(⊑/%s)", lc(em, be->name));
      int pair = em->isPair[entIdx(em, fe)];
      ev->v = pair ? efmt(em, "(<%s⊑%s)", id, lc(em, fe->name))
                   : efmt(em, "(%s⊑%s)", id, lc(em, fe->name));
      ev->pair = pair; ev->unit = 1;
      return 0;
    }
    /* functional relationship hop: rel.Comp with ¯1 dangling */
    if (be && be->kind == RK_REL && fe) {
      char *rel = lc(em, be->name);
      char *comp = lc(em, fe->name);
      char *w = efmt(em, "((0⌈%s)⊏%s)", rel, comp);
      ev->g = efmt(em, "(0≤%s)", rel);
      if (fe->hasPres) ev->g = gAnd(em, ev->g, efmt(em, "((0⌈%s)⊏pres_%s)", rel, fe->name));
      ev->sym = fe->type == CT_SYM;
      ev->v = inMode(em, w, m);
      return 0;
    }
    /* chained hop: rel.rel2.Comp */
    if (be && be->kind == RK_REL && field->kind == N_HOP) {
      return fail(em, nd->line, "nested hop chains beyond one level: spell left-assoc");
    }
  }
  /* left-assoc chain: (rel.rel).Comp */
  if (base->kind == N_HOP) {
    EV bv; if (emitHop(em, base, MODE_WORLD, &bv)) return -1;
    const RegEntry *fe = field->kind == N_NAME ? find(em, field->name) : NULL;
    if (!fe) return fail(em, nd->line, "hop target '%s' unregistered", field->name);
    char *w = efmt(em, "((0⌈%s)⊏%s)", bv.v, lc(em, fe->name));
    ev->g = gAnd(em, bv.g, efmt(em, "(0≤%s)", bv.v));
    ev->sym = fe->type == CT_SYM;
    ev->v = inMode(em, w, m);
    return 0;
  }
  return fail(em, nd->line, "unsupported hop");
}

static int emitVal(Em *em, const Node *nd, Mode m, EV *ev) {
  memset(ev, 0, sizeof *ev);
  switch (nd->kind) {
    case N_NUM: ev->v = numLit(em, nd->num); ev->unit = 1; return 0;
    case N_COUNTER: {
      const RegEntry *e = find(em, nd->name);
      ev->unit = 1;
      ev->v = e ? efmt(em, "(%s×%s)", numLit(em, nd->num), lc(em, e->name))
                : numLit(em, nd->num);
      return 0;
    }
    case N_SYM: ev->v = efmt(em, "(<\"%s\")", nd->name); ev->unit = 1; ev->sym = 1; return 0;
    case N_STR: ev->v = efmt(em, "\"%s\"", nd->name); ev->unit = 1; return 0;
    case N_NAME: return emitNameVal(em, nd, m, ev);
    case N_ALIAS: {
      const RegEntry *e = find(em, nd->name);
      if (!e) return fail(em, nd->line, "unregistered alias '^%s'", nd->name);
      ev->v = inMode(em, lc(em, e->name), m); return 0;
    }
    case N_ARITH: {
      EV a, b;
      if (emitVal(em, nd->kids[0], m, &a)) return -1;
      if (emitVal(em, nd->kids[1], m, &b)) return -1;
      ev->g = gAnd(em, a.g, b.g);
      ev->pair = a.pair || b.pair;
      ev->unit = a.unit && b.unit;
      /* a scan-along value keeps its order through scalar arithmetic; against another
       * column the two orders disagree and no alignment exists */
      if (a.along || b.along) {
        if (a.along && !b.unit) return fail(em, nd->line, "scan-along composed against a differently-ordered operand");
        if (b.along && !a.unit) return fail(em, nd->line, "scan-along composed against a differently-ordered operand");
        ev->along = a.along ? a.along : b.along;
      }
      const char *op = nd->op == '+' ? "+" : nd->op == '-' ? "-" : nd->op == '*' ? "×"
                     : nd->op == '/' ? "÷" : "|";
      if (nd->op == '%') { ev->v = efmt(em, "(%s|%s)", b.v, a.v); return 0; } /* a % b = b | a */
      ev->v = efmt(em, "(%s%s%s)", a.v, op, b.v);
      return 0;
    }
    case N_CMP: {
      EV a, b;
      if (emitVal(em, nd->kids[0], m, &a)) return -1;
      if (emitVal(em, nd->kids[1], m, &b)) return -1;
      ev->g = gAnd(em, a.g, b.g);
      int sym = a.sym || b.sym;
      const char *op = nd->op == '<' ? "<" : nd->op == '>' ? ">" : nd->op == 'l' ? "≤"
                     : nd->op == 'g' ? "≥" : nd->op == '!' ? "≠" : "=";
      if (sym && (nd->op == '=' || nd->op == '!')) {
        ev->v = efmt(em, "(%s%s≡¨%s)", nd->op == '!' ? "¬" : "", a.v, b.v);
        return 0;
      }
      ev->v = efmt(em, "(%s%s%s)", a.v, op, b.v);
      return 0;
    }
    case N_AND: case N_OR: case N_NOT: {
      char *msk; if (emitMask(em, nd, &msk)) return -1;
      ev->v = inMode(em, msk, m);
      return 0;
    }
    case N_SCOPE: {
      /* value @ scope outside a fold: locative AND at mask level */
      char *msk; if (emitMask(em, nd, &msk)) return -1;
      ev->v = inMode(em, msk, m);
      return 0;
    }
    case N_HOP: return emitHop(em, nd, m, ev);
    case N_CALL: return emitCall(em, nd, m, ev);
    case N_FOLD: case N_REDUCE: {
      if (nd->kind == N_REDUCE) {
        Node tmp = *nd; /* reduce(f) col @ scope == f/ col @ scope with named reducer */
        return emitFold(em, &tmp, m, ev);
      }
      EV f; if (emitFold(em, nd, m, &f)) return -1;
      *ev = f;
      if (!f.unit && m != MODE_WORLD) { /* gamma column: gather */
        ev->v = f.v; /* emitGamma already applied inMode */
      }
      return 0;
    }
    case N_SCANEXPR: return emitScan(em, nd, ev);
    case N_SCANALONG: {
      EV xv; if (emitVal(em, nd->kids[0], MODE_WORLD, &xv)) return -1;
      EV ov; if (emitVal(em, nd->kids[1], MODE_WORLD, &ov)) return -1;
      const char *gl = !strcmp(nd->name, "+") ? "+`" : !strcmp(nd->name, "*") ? "×`"
                     : !strcmp(nd->name, "max") ? "⌈`" : !strcmp(nd->name, "min") ? "⌊`" : NULL;
      if (!gl) return fail(em, nd->line, "scan(%s): no registered scan step", nd->name);
      ev->v = efmt(em, "(%s(%s)⊏%s)", gl, ov.v, xv.v);
      ev->along = ov.v;
      return 0;
    }
    case N_IOTAX: {
      EV n; if (emitVal(em, nd->kids[0], MODE_WORLD, &n)) return -1;
      ev->v = efmt(em, "(↕%s)", n.v);
      return 0;
    }
    case N_TUPLE: {
      if (nd->nkids != 2) return fail(em, nd->line, "value tuple arity %d", nd->nkids);
      EV a, b;
      if (emitVal(em, nd->kids[0], m, &a)) return -1;
      if (emitVal(em, nd->kids[1], m, &b)) return -1;
      ev->pair = 1; ev->g = gAnd(em, a.g, b.g);
      if (a.unit && b.unit) { ev->unit = 1; ev->v = efmt(em, "(<%s‿%s)", a.v, b.v); return 0; }
      ev->v = efmt(em, "(%s⋈¨%s)", a.v, b.v);
      return 0;
    }
    case N_TO: {
      /* reshape rhs: exactly count-of-selection positions, in selection space */
      const Node *sh = nd->kids[0];
      int d0 = sh->kids[0]->kind == N_WILD ? -1 : (int)sh->kids[0]->num;
      if (sh->nkids == 1) { ev->v = efmt(em, "(↕%d)", d0); return 0; }
      int d1 = sh->kids[1]->kind == N_WILD ? -1 : (int)sh->kids[1]->num;
      char *k = efmt(em, "(+´%s)", em->selVar);
      char *rows = d0 < 0 ? efmt(em, "(%s÷%d)", k, d1) : efmt(em, "%d", d0);
      char *cols = d1 < 0 ? efmt(em, "(%s÷%d)", k, d0) : efmt(em, "%d", d1);
      ev->pair = 1;
      ev->v = efmt(em, "(⥊↕%s‿%s)", rows, cols);
      return 0;
    }
    case N_GRADE: {
      char *ids; if (emitGradeIds(em, nd, &ids)) return -1;
      ev->v = ids; ev->unit = 1;
      return 0;
    }
    case N_TOP: {
      char *inner;
      const Node *in = nd->kids[0];
      /* top k truncates an ORDERING; a mask has no order to truncate — reject, never k↑mask */
      if (in->kind == N_AND || in->kind == N_OR || in->kind == N_NOT ||
          in->kind == N_CMP || in->kind == N_TUPLE)
        return fail(em, nd->line, "top %d over a mask: the operand must be an ordered selection (grade or pipeline)", (int)nd->num);
      if (in->kind == N_GRADE) { if (emitGradeIds(em, in, &inner)) return -1; }
      else { EV iv; if (emitVal(em, in, MODE_WORLD, &iv)) return -1; inner = iv.v; }
      ev->v = efmt(em, "(%d↑%s)", (int)nd->num, inner);
      ev->unit = 1;
      return 0;
    }
    case N_ORDERBY: {
      EV kv; if (emitVal(em, nd->kids[0], MODE_WORLD, &kv)) return -1;
      ev->v = efmt(em, "(%s%s)", (nd->flags & F_DESC) ? "⍒" : "⍋", kv.v);
      return 0;
    }
    case N_CROSSV: {
      char *am, *bm;
      if (emitMask(em, nd->kids[0], &am)) return -1;
      if (emitMask(em, nd->kids[1], &bm)) return -1;
      const RegEntry *e = find(em, nd->name);
      if (!e || e->kind != RK_FN) return fail(em, nd->line, "cross needs a registered fn");
      ev->v = efmt(em, "(⥊(/%s)%s⌜(/%s))", am, fnv(em, e->name), bm);
      return 0;
    }
    case N_PIPE: {
      View vw; if (emitPipe(em, nd, &vw)) return -1;
      ev->v = viewWorldIds(em, &vw); ev->unit = 1;
      return 0;
    }
    case N_SETHOP: return fail(em, nd->line, "bare rel' outside fold/source position");
    default: return fail(em, nd->line, "unsupported value node %d", nd->kind);
  }
}

/* cell coordinate column for the anchored frame: registered x/y fields win, else the
 * lattice frame's computed coordinates; NULL when neither exists */
static char *cellCoord(Em *em, char axis) {
  const RegEntry *e = find(em, axis == 'x' ? "x" : "y");
  if (e && (e->kind == RK_FIELD || e->kind == RK_COL)) return lc(em, e->name);
  if (em->fr.kind == FR_LAT || em->fr.kind == FR_BOARD)
    return axis == 'y' ? efmt(em, "(⌊(↕%d)÷%d)", em->fr.w * em->fr.h, em->fr.w)
                       : efmt(em, "(%d|↕%d)", em->fr.w, em->fr.w * em->fr.h);
  return NULL;
}

/* mask emission: full frame-length boolean vector, all guards folded in */
static int emitMask(Em *em, const Node *nd, char **out) {
  switch (nd->kind) {
    case N_NAME: return emitNameMask(em, nd, out);
    case N_ALIAS: {
      const RegEntry *e = find(em, nd->name);
      if (!e) return fail(em, nd->line, "unregistered alias '^%s'", nd->name);
      *out = lc(em, e->name);
      return 0;
    }
    case N_AND: case N_OR: {
      /* image source: Mask.rel' as left arm handled at source level; here plain */
      char *a, *b;
      if (emitMask(em, nd->kids[0], &a)) return -1;
      if (emitMask(em, nd->kids[1], &b)) return -1;
      *out = efmt(em, "(%s%s%s)", a, nd->kind == N_AND ? "∧" : "∨", b);
      return 0;
    }
    case N_NOT: {
      /* absent-component reading for sparse value columns */
      const Node *k = nd->kids[0];
      if (k->kind == N_NAME) {
        const RegEntry *e = find(em, k->name);
        if (e && e->kind == RK_COL && e->type != CT_BOOL && e->hasPres) {
          *out = efmt(em, "(¬pres_%s)", e->name);
          return 0;
        }
      }
      char *a; if (emitMask(em, k, &a)) return -1;
      *out = efmt(em, "(¬%s)", a);
      return 0;
    }
    case N_CMP: {
      if (nd->op == '_') { /* presence-any tuple element */
        const RegEntry *e = find(em, nd->kids[0]->name);
        if (!e) return fail(em, nd->line, "unregistered '%s _'", nd->kids[0]->name);
        *out = e->hasPres ? efmt(em, "pres_%s", e->name) : efmt(em, "(1¨%s)", lc(em, e->name));
        return 0;
      }
      EV v; if (emitVal(em, nd, MODE_WORLD, &v)) return -1;
      *out = v.g ? efmt(em, "((%s)∧%s)", v.v, v.g) : v.v;
      return 0;
    }
    case N_TUPLE: { /* presence tuple: AND of elements */
      char *acc = NULL;
      for (int i = 0; i < nd->nkids; i++) {
        char *mi; if (emitMask(em, nd->kids[i], &mi)) return -1;
        acc = acc ? efmt(em, "(%s∧%s)", acc, mi) : mi;
      }
      *out = acc;
      return 0;
    }
    case N_SCOPE: {
      char *a, *b;
      if (nd->nkids == 3) {
        /* mask @ frame at origin — the anchored frame (frame join): @ fixes (o, S) and
         * `at` fills the origin slot with a mirror-read; the registered frame fn runs
         * per cell as Fn ⟨cell, origin, args…⟩ and returns the frame mask */
        if (emitMask(em, nd->kids[0], &a)) return -1;
        const Node *fnode = nd->kids[1];
        EV org; if (emitVal(em, nd->kids[2], MODE_WORLD, &org)) return -1;
        if (!org.unit || !org.pair)
          return fail(em, nd->line, "frame anchor must be one point (a pair mirror-read)");
        if (fnode->kind != N_CALL)
          return fail(em, fnode->line, "anchored frame wants fn(args…) — a registered frame predicate");
        const RegEntry *e = find(em, fnode->name);
        if (!e || e->kind != RK_FN)
          return fail(em, fnode->line, "anchored frame '%s' is not a registered fn", fnode->name);
        char *xs = cellCoord(em, 'x'), *ys = cellCoord(em, 'y');
        if (!xs || !ys)
          return fail(em, fnode->line, "anchored frame needs cell coordinates (x/y fields or a lattice frame)");
        char *args = efmt(em, "%s", "");
        for (int i = 0; i < fnode->nkids; i++) {
          EV av; if (emitVal(em, fnode->kids[i], MODE_WORLD, &av)) return -1;
          args = efmt(em, "%s, %s", args, av.v);
        }
        char *t = tv(em);
        stage(em, "%s ← {%s ⟨𝕩, ⊑%s%s⟩}¨(%s⋈¨%s)", t, fnv(em, e->name), org.v, args, xs, ys);
        *out = efmt(em, "(%s∧%s)", a, t);
        return 0;
      }
      if (nd->kids[1]->kind == N_SHAPE) { /* frame scope: predicate over the lattice */
        if (emitMask(em, nd->kids[0], &a)) return -1;
        *out = a;
        return 0;
      }
      if (emitMask(em, nd->kids[0], &a)) return -1;
      if (emitMask(em, nd->kids[1], &b)) return -1;
      *out = efmt(em, "(%s∧%s)", a, b);
      return 0;
    }
    case N_HOP: {
      /* image: Sel.rel' — union of the selected sources' fibers, membership by stable id */
      if (nd->kids[1]->kind == N_SETHOP) {
        char *src; if (emitMask(em, nd->kids[0], &src)) return -1;
        char *fib; if (fiberVar(em, nd->kids[1]->kids[0], &fib)) return -1;
        *out = efmt(em, "(%s‿%s AnoImage %s)", src, fib, idCol(em));
        return 0;
      }
      EV v; if (emitHop(em, nd, MODE_WORLD, &v)) return -1;
      *out = v.g ? efmt(em, "((%s)∧%s)", v.v, v.g) : v.v;
      return 0;
    }
    case N_FOLD: { /* quantifier fold as a mask: guards fold in */
      EV v; if (emitFold(em, nd, MODE_WORLD, &v)) return -1;
      *out = v.g ? efmt(em, "((%s)∧%s)", v.v, v.g) : v.v;
      return 0;
    }
    case N_TOP: case N_GRADE: {
      EV v; if (emitVal(em, nd, MODE_WORLD, &v)) return -1;
      *out = efmt(em, "((↕%s)∊%s)", frN(em), v.v);
      return 0;
    }
    case N_PIPE: {
      View vw; if (emitPipe(em, nd, &vw)) return -1;
      if (vw.expandCnt) /* Spawner |> expand Count , spawn X : counts feed the spawn */
        em->pipeExpand = vw.expandCnt;
      *out = efmt(em, "((↕%s)∊%s)", frN(em), viewWorldIds(em, &vw));
      return 0;
    }
    case N_SHAPE: { /* pure shape source: everything in frame */
      *out = efmt(em, "(1¨↕%s)", frN(em));
      return 0;
    }
    default: {
      EV v; if (emitVal(em, nd, MODE_WORLD, &v)) return -1;
      *out = v.g ? efmt(em, "((%s)∧%s)", v.v, v.g) : v.v;
      return 0;
    }
  }
}

/* ---------- statements ---------- */

/* find the leftmost leaf of an &-chain to detect a frame-setting source */
static const Node *leftmost(const Node *nd) {
  while (nd->kind == N_AND || nd->kind == N_OR || nd->kind == N_SCOPE) nd = nd->kids[0];
  return nd;
}

/* first @-scope with a shape rhs anywhere in the selection (top 8 (grade desc X @ 64 64)) */
static const Node *findShapeScope(const Node *nd) {
  if (!nd) return NULL;
  if (nd->kind == N_SCOPE && nd->nkids > 1 && nd->kids[1]->kind == N_SHAPE) return nd->kids[1];
  for (int i = 0; i < nd->nkids; i++) {
    const Node *r = nd->kids[i] ? findShapeScope(nd->kids[i]) : NULL;
    if (r) return r;
  }
  return NULL;
}

/* set em->fr from the selection; emit board/lattice bindings when needed */
static void setFrame(Em *em, const Node *sel) {
  em->fr.kind = FR_ENT; em->fr.w = em->fr.h = 0; em->fr.lit = "";
  if (!sel) { em->fr = em->savedFr; return; }
  const Node *lm = leftmost(sel);
  if (lm->kind == N_SHAPE) {
    if (lm->nkids == 2) { em->fr.kind = FR_LAT; em->fr.w = (int)lm->kids[0]->num; em->fr.h = (int)lm->kids[1]->num; }
    else { em->fr.kind = FR_LINE; em->fr.w = (int)lm->kids[0]->num; }
    return;
  }
  if (lm->kind == N_TO && lm->nkids > 1 && lm->kids[1]->kind == N_STR) {
    em->fr.kind = FR_BOARD;
    em->fr.w = (int)lm->kids[0]->kids[0]->num;
    em->fr.h = lm->kids[0]->nkids > 1 ? (int)lm->kids[0]->kids[1]->num : 1;
    em->fr.lit = lm->kids[1]->name;
    stage(em, "brd%d ← \"%s\"", em->stmt, em->fr.lit);
    return;
  }
  /* a shape-scoped subterm pulls the frame: +/ Elevation @ 64 64, top 8 (grade desc X @ 64 64) */
  const Node *sh = findShapeScope(sel);
  if (sh) {
    em->fr.kind = FR_LAT;
    em->fr.w = (int)sh->kids[0]->num;
    em->fr.h = sh->nkids > 1 ? (int)sh->kids[1]->num : 1;
  }
}

/* strip a frame-setting source out of the predicate: SHAPE & p -> p */
static const Node *stripFrame(Em *em, const Node *sel) {
  (void)em;
  if (!sel) return NULL;
  if (sel->kind == N_SHAPE) return NULL;
  if (sel->kind == N_TO && sel->nkids > 1 && sel->kids[1]->kind == N_STR) return NULL;
  if (sel->kind == N_AND) {
    const Node *lm = leftmost(sel);
    if (lm->kind == N_SHAPE && sel->kids[0]->kind == N_SHAPE) return sel->kids[1];
    if (lm->kind == N_SHAPE) {
      /* SHAPE buried deeper: (SHAPE & a) & b — rebuild without it is overkill; handle 1 level */
      if (sel->kids[0]->kind == N_AND && sel->kids[0]->kids[0]->kind == N_SHAPE) {
        /* fabricate AND(a, b) via kids juggling is unsafe on const; fall through */
      }
    }
  }
  return sel;
}

typedef struct {
  int colIdx;          /* registry entry */
  char *newExpr;       /* full replacement column expr (world length) */
  char fam;            /* merge family: '+' additive, '*' multiplicative, '=' set,
                          '|' presence-add, '&' presence-del, 'v' verb dispatch */
  const char *field;   /* pair-field projection name or NULL */
  unsigned rules;      /* bitmask of contributing rules in a shared tick (0 outside) */
} Commit;

/* one spawn group: a statement may batch several spawn effects (§10); each appends
 * its own row group in effect order, keys mint once across the batch */
typedef struct {
  char *cnt;           /* selection-space counts */
  char *tot;           /* (+´cnt) */
  char *pos;           /* copy-space positions or NULL */
  int posPair;
  const char *protoName;   /* named proto col or NULL */
  char *protoExpr;     /* computed proto (sym per copy) or NULL */
} SpawnG;

/* per-statement effect staging */
typedef struct {
  Commit commits[64]; int ncommits;
  int despawn;         /* keep = ¬despawnSel */
  char *despawnSel;    /* the mask despawn was staged under (ORs across a rule tick) */
  SpawnG sp[8]; int nsp;
} Fx;

static Commit *findCommit(Fx *fx, int colIdx) {
  for (int i = 0; i < fx->ncommits; i++)
    if (fx->commits[i].colIdx == colIdx) return &fx->commits[i];
  return NULL;
}

static void addCommit(Em *em, Fx *fx, int colIdx, char *expr, char fam, const char *field) {
  unsigned bit = em->curRule >= 0 ? 1u << em->curRule : 0;
  for (int i = 0; i < fx->ncommits; i++)
    if (fx->commits[i].colIdx == colIdx) {
      fx->commits[i].newExpr = expr; fx->commits[i].fam = fam; fx->commits[i].field = field;
      fx->commits[i].rules |= bit;
      return;
    }
  fx->commits[fx->ncommits].colIdx = colIdx;
  fx->commits[fx->ncommits].newExpr = expr;
  fx->commits[fx->ncommits].fam = fam;
  fx->commits[fx->ncommits].field = field;
  fx->commits[fx->ncommits].rules = bit;
  fx->ncommits++;
}

/* §10 same-column batch: deltas observe pre-state; the commit is their merge. Rebasing a
 * later effect's accumulate onto the earlier commit realizes the merge exactly when the
 * family commutes (additive, multiplicative, idempotent presence, or disjoint pair fields);
 * anything else has no merge law and the batch is rejected. Returns the accumulate base,
 * or NULL after fail. */
static char *mergeBase(Em *em, Fx *fx, int colIdx, char *col, char fam, const char *field,
                       int line, const char *name) {
  Commit *prev = findCommit(fx, colIdx);
  if (!prev) return col;
  int ok = (prev->fam == fam && (fam == '+' || fam == '*' || fam == '|' || fam == '&')) ||
           (prev->fam == '=' && fam == '=' && field && prev->field &&
            strcmp(field, prev->field) != 0);
  /* §11 third clause: inside a shared rule tick, complementary guard literals certify the
   * writers' masks row-disjoint, and disjoint writes merge exactly whatever the families */
  if (!ok && em->curRule >= 0 && prev->rules &&
      !(prev->rules & ~em->ruleDisj[em->curRule]) && fam != 'v' && prev->fam != 'v')
    ok = 1;
  if (!ok) {
    fail(em, line, "no merge law: '%s' written twice in one barrier (§10)", name);
    return NULL;
  }
  return prev->newExpr;
}

/* current column read: staged value if already written this statement (barrier says NO:
 * effects read pre-state, so reads always come from the raw var; commits land at the end) */

static int emitEffect(Em *em, const Node *ef, Fx *fx) {
  switch (ef->kind) {
    case N_EASSIGN: {
      const Node *tgt = ef->kids[0];
      const char *field = NULL;
      const Node *coln = tgt;
      if (tgt->kind == N_HOP) { coln = tgt->kids[0]; field = tgt->kids[1]->name; }
      const RegEntry *e = find(em, coln->name);
      if (!e) return fail(em, ef->line, "assign to unregistered '%s'", coln->name);
      char *col = lc(em, e->name);
      EV rhs;
      /* guards must refine the mask before gathering: pre-scan via world-mode guard probe */
      StrBuf save = em->pre; StrBuf probe = {0}; em->pre = probe;
      EV probeV; int perr = emitVal(em, ef->kids[1], MODE_WORLD, &probeV);
      sb_free(&em->pre); em->pre = save;
      if (perr) return -1;
      char *oldSel = efmt(em, "%s", em->selVar);
      char *selE = oldSel;
      if (probeV.g) {
        char *t = tv(em);
        stage(em, "%s ← %s∧%s", t, oldSel, probeV.g);
        selE = t;
      }
      snprintf(em->selVar, sizeof em->selVar, "%s", selE);
      if (emitVal(em, ef->kids[1], MODE_SEL, &rhs)) return -1;
      char *rv = rhs.v;
      if (rhs.unit) rv = efmt(em, "((+´%s)⥊%s)", selE, rv);
      /* scan-along: values arrive in along order; the scatter walks the mask in row order,
       * so unsort through the order's grade — the Tier-2 conjugation h(c) = f(c∘σ)∘σ⁻¹ */
      if (rhs.along) rv = efmt(em, "((⍋%s)⊏%s)", rhs.along, rv);
      /* the RHS observed pre-state above; only the accumulate base rebases onto an
       * earlier same-column commit, which is the §10 merge for a commuting family */
      char fam = ef->op == '=' ? '=' : (ef->op == '+' || ef->op == '-') ? '+' : '*';
      char *base = mergeBase(em, fx, entIdx(em, e), col, fam, field, ef->line, coln->name);
      if (!base) return -1;
      if (ef->op != '=') {
        const char *op = ef->op == '+' ? "+" : ef->op == '-' ? "-" : ef->op == '*' ? "×" : "÷";
        char *oldv = field ? efmt(em, "(%d⊸⊑¨(%s/%s))", field[0]=='y', selE, base)
                           : efmt(em, "(%s/%s)", selE, base);
        rv = efmt(em, "(%s%s%s)", oldv, op, rv);
      }
      char *newcol;
      if (field) {
        int yi = field[0] == 'y';
        char *pairs = yi ? efmt(em, "((0⊸⊑¨(%s/%s))⋈¨%s)", selE, base, rv)
                         : efmt(em, "(%s⋈¨(1⊸⊑¨(%s/%s)))", rv, selE, base);
        newcol = efmt(em, "%s‿%s AnoScat %s", selE, pairs, base);
        em->isPair[entIdx(em, e)] = 1;
      } else {
        newcol = efmt(em, "%s‿%s AnoScat %s", selE, rv, base);
        if (rhs.pair) em->isPair[entIdx(em, e)] = 1;
      }
      char *t = tv(em);
      stage(em, "%s ← %s", t, newcol);
      addCommit(em, fx, entIdx(em, e), t, fam, field);
      snprintf(em->selVar, sizeof em->selVar, "%s", oldSel);
      return 0;
    }
    case N_EADD: case N_EDEL: {
      const RegEntry *e = find(em, ef->name);
      if (!e || (e->kind != RK_COL && e->kind != RK_FIELD))
        return fail(em, ef->line, "%cComp on unregistered '%s'", ef->kind == N_EADD ? '+' : '-', ef->name);
      char *col = lc(em, e->name);
      char fam = ef->kind == N_EADD ? '|' : '&';
      char *base = mergeBase(em, fx, entIdx(em, e), col, fam, NULL, ef->line, ef->name);
      if (!base) return -1;
      char *t = tv(em);
      if (ef->kind == N_EADD) stage(em, "%s ← %s∨%s", t, base, em->selVar);
      else stage(em, "%s ← %s∧¬%s", t, base, em->selVar);
      addCommit(em, fx, entIdx(em, e), t, fam, NULL);
      return 0;
    }
    case N_EDESPAWN:
      fx->despawnSel = fx->despawn ? efmt(em, "(%s∨%s)", fx->despawnSel, em->selVar)
                                   : efmt(em, "%s", em->selVar);
      fx->despawn = 1;
      return 0;
    case N_ESPAWN: {
      const Node *what = ef->kids[0];
      const Node *cnt = ef->nkids > 1 ? ef->kids[1] : NULL;
      const Node *at = ef->nkids > 2 ? ef->kids[2] : NULL;
      /* Tier-1 inscription: on a lattice frame, a proto registered as a FIELD takes the
       * figure as a monotone OR — the ground is conserved, no rows mint (ex37) */
      if ((em->fr.kind == FR_LAT || em->fr.kind == FR_BOARD) && !cnt &&
          what->kind == N_NAME) {
        const RegEntry *fe = find(em, what->name);
        if (fe && fe->kind == RK_FIELD) {
          char *base = mergeBase(em, fx, entIdx(em, fe), lc(em, fe->name), '|', NULL,
                                 ef->line, what->name);
          if (!base) return -1;
          char *t = tv(em);
          stage(em, "%s ← %s∨%s", t, base, em->selVar);
          addCommit(em, fx, entIdx(em, fe), t, '|', NULL);
          return 0;
        }
      }
      if (fx->nsp >= 8) return fail(em, ef->line, "too many spawn groups in one statement");
      SpawnG *sg = &fx->sp[fx->nsp++];
      char *cv;
      char *protoSel = NULL;   /* computed proto, selection space, before the skip filter */
      if (cnt) {
        EV c; if (emitVal(em, cnt, MODE_SEL, &c)) return -1;
        cv = c.unit ? efmt(em, "((+´%s)⥊%s)", em->selVar, c.v) : c.v;
      } else if (em->pipeExpand[0]) cv = efmt(em, "%s", em->pipeExpand);
      else if (what->kind == N_CALL) {
        /* spawn (pieceOf char): the lookup runs per source; an empty sym spawns nothing
         * for that cell — ". maps to no spawn" (ex34) */
        EV pv; if (emitVal(em, what, MODE_SEL, &pv)) return -1;
        protoSel = tv(em);
        stage(em, "%s ← %s", protoSel, pv.v);
        cv = efmt(em, "(\"\"⊸≢¨%s)", protoSel);
      }
      else cv = efmt(em, "((+´%s)⥊1)", em->selVar);
      char *cV = tv(em); stage(em, "%s ← %s", cV, cv);
      snprintf(em->cntVar, sizeof em->cntVar, "%s", cV);
      /* §17: an explicit replicate binds index per copy (restarting at each source) and
       * shadows the outer binding; a plain spawn keeps the minting row's ordinal (§21) */
      if (cnt || em->pipeExpand[0]) {
        char *iV = tv(em); stage(em, "%s ← ∾↕¨%s", iV, cV);
        snprintf(em->idxVar, sizeof em->idxVar, "%s", iV);
      } else
        snprintf(em->idxVar, sizeof em->idxVar, "(↕+´%s)", em->selVar);
      sg->cnt = cV;
      sg->tot = efmt(em, "(+´%s)", cV);
      /* stage pos/proto as temps NOW: the commit loop mutates columns in registry order,
       * and a spawn expression must observe pre-state, never a sibling commit (ex28) */
      if (at) {
        EV p; if (emitVal(em, at, MODE_COPY, &p)) return -1;
        char *pv = tv(em);
        stage(em, "%s ← %s", pv, p.unit ? efmt(em, "(%s⥊%s)", sg->tot, p.v) : p.v);
        sg->pos = pv;
        sg->posPair = p.pair;
      } else if (em->fr.kind == FR_LAT || em->fr.kind == FR_BOARD) {
        char *pv = tv(em);
        stage(em, "%s ← %s/%s/((%d|↕%d)⋈¨⌊(↕%d)÷%d)",
              pv, cV, em->selVar, em->fr.w, em->fr.w*em->fr.h, em->fr.w*em->fr.h, em->fr.w);
        sg->pos = pv;
        sg->posPair = 1;
      }
      if (what->kind == N_NAME) sg->protoName = what->name;
      else if (protoSel) {
        char *pt = tv(em);
        stage(em, "%s ← %s/%s", pt, cV, protoSel);
        sg->protoExpr = pt;
      } else if (what->kind == N_CALL) {
        EV pv; if (emitVal(em, what, MODE_COPY, &pv)) return -1;
        char *pt = tv(em);
        stage(em, "%s ← %s", pt, pv.v);
        sg->protoExpr = pt;
      }
      return 0;
    }
    case N_EVERB: case N_EVIA: {
      const RegEntry *e = find(em, ef->name);
      if (!e || e->kind != RK_FN || !e->syms)
        return fail(em, ef->line, "verb '%s' needs a registered fn with a target column", ef->name);
      /* raw text: "<targetcol> <dfn>"; the dfn is bound at fixture as FnName */
      char target[ANO_NAMESZ]; sscanf(e->syms[0], "%63s", target);
      const RegEntry *tc = find(em, target);
      if (!tc) return fail(em, ef->line, "verb '%s' target column '%s' unregistered", ef->name, target);
      char *args = efmt(em, "⟨%s", lc(em, tc->name));
      for (int i = 0; i < ef->nkids; i++) {
        EV av; if (emitVal(em, ef->kids[i], MODE_WORLD, &av)) return -1;
        args = efmt(em, "%s, %s", args, av.v);
      }
      args = efmt(em, "%s⟩", args);
      if (!mergeBase(em, fx, entIdx(em, tc), lc(em, tc->name), 'v', NULL, ef->line, ef->name))
        return -1;
      char *t = tv(em);
      stage(em, "%s ← %s %s %s", t, em->selVar, fnv(em, e->name), args);
      addCommit(em, fx, entIdx(em, tc), t, 'v', NULL);
      return 0;
    }
    default: return fail(em, ef->line, "unsupported effect %d", ef->kind);
  }
}

/* default append value for a column on one spawn group; keys are minted once across the
 * whole batch by the caller, never here */
static char *spawnDefault(Em *em, const RegEntry *e, SpawnG *g) {
  char *tot = g->tot;
  if (e->kind == RK_SREL) return efmt(em, "(%s⥊<⟨⟩)", tot);
  if (e->kind == RK_REL) return efmt(em, "(%s⥊¯1)", tot);
  if (e->type == CT_SYM) return efmt(em, "(%s⥊<\"\")", tot);
  if (em->isPair[entIdx(em, e)]) return efmt(em, "(%s⥊<¯1‿¯1)", tot);
  if (!strcmp(e->name, "parent")) return efmt(em, "(%s//%s)", g->cnt, em->selVar);
  return efmt(em, "(%s⥊%s)", tot, numLit(em, e->defval));
}

/* the batch total: sum of every spawn group's row count */
static char *spawnTotAll(Em *em, Fx *fx) {
  char *tot = NULL;
  for (int g = 0; g < fx->nsp; g++)
    tot = tot ? efmt(em, "(%s+%s)", tot, fx->sp[g].tot) : fx->sp[g].tot;
  return tot;
}

static int commitStmt(Em *em, Fx *fx, int isCont) {
  const Registry *r = em->reg;
  char *keep = NULL;
  if (fx->despawn) {
    keep = tv(em);
    stage(em, "%s ← ¬%s", keep, fx->despawnSel ? fx->despawnSel : em->selVar);
  }
  int structural = fx->despawn || fx->nsp;
  if (em->fr.kind != FR_ENT && structural && fx->despawn)
    return fail(em, 0, "despawn outside the entity world");
  (void)isCont;
  char *totAll = fx->nsp ? spawnTotAll(em, fx) : NULL;
  for (int i = 0; i < r->nents; i++) {
    const RegEntry *e = &r->ents[i];
    int isField = e->kind == RK_FIELD;
    if (e->kind != RK_COL && e->kind != RK_REL && e->kind != RK_SREL && !isField) continue;
    /* lattice-sided relations (a stencil srel over w*h cells) are frame-foreign to entity
     * row structure: never filter on despawn, never pad on spawn */
    if (e->kind == RK_SREL && e->nfib != r->n) continue;
    if (e->kind == RK_REL && e->nnums != r->n) continue;
    char *cur = lc(em, e->name);
    char *base = cur;
    for (int c = 0; c < fx->ncommits; c++)
      if (fx->commits[c].colIdx == i) base = fx->commits[c].newExpr;
    if (isField) { /* lattice fields: value commits only, never row structure */
      if (base != cur) stage(em, "%s ↩ %s", cur, base);
      continue;
    }
    char *app = NULL;
    int anyProto = 0;
    if (fx->nsp) { /* spawn appends one row group per spawn effect, in effect order */
      if (!strcmp(e->name, "keys"))
        app = efmt(em, "((1+⌈´¯1∾keys)+↕%s)", totAll);  /* one mint across the batch */
      else for (int g = 0; g < fx->nsp; g++) {
        SpawnG *sg = &fx->sp[g];
        char *piece;
        int isProto = sg->protoName && !strcmp(lc(em, (char*)sg->protoName), cur);
        anyProto |= isProto;
        if (isProto) piece = efmt(em, "(%s⥊1)", sg->tot);
        else if (sg->protoExpr && !strcmp(cur, "proto")) piece = sg->protoExpr;
        else if (sg->pos && !strcmp(cur, "pos")) { piece = sg->pos; if (sg->posPair) em->isPair[i] = 1; }
        else piece = spawnDefault(em, e, sg);
        app = app ? efmt(em, "%s∾%s", app, piece) : piece;
      }
    }
    if (base == cur && !app && !fx->despawn) continue;
    if (structural) {
      char *kept = keep ? efmt(em, "(%s/%s)", keep, base) : base;
      if (app) stage(em, "%s ↩ %s∾%s", cur, kept, app);
      else stage(em, "%s ↩ %s", cur, kept);
    } else if (base != cur) {
      stage(em, "%s ↩ %s", cur, base);
    }
    if (e->kind == RK_COL && e->hasPres && structural) {
      char *p = efmt(em, "pres_%s", e->name);
      char *kept = keep ? efmt(em, "(%s/%s)", keep, p) : p;
      if (fx->nsp) {
        char *papp = NULL;
        for (int g = 0; g < fx->nsp; g++) {
          int isProto = fx->sp[g].protoName && !strcmp(lc(em, (char*)fx->sp[g].protoName), cur);
          char *piece = efmt(em, "(%s⥊%d)", fx->sp[g].tot, isProto ? 1 : 0);
          papp = papp ? efmt(em, "%s∾%s", papp, piece) : piece;
        }
        stage(em, "%s ↩ %s∾%s", p, kept, papp);
      }
      else stage(em, "%s ↩ %s", p, kept);
    }
    (void)anyProto;
  }
  /* spawn always appends to the entity world, whatever frame selected the sources */
  if (structural) {
    if (fx->despawn && fx->nsp) stage(em, "anoN ↩ (+´%s)+%s", keep, totAll);
    else if (fx->despawn) stage(em, "anoN ↩ +´%s", keep);
    else stage(em, "anoN ↩ anoN+%s", totAll);
  }
  return 0;
}

static int emitStmt(Em *em, const Node *st) {
  em->stmt++;
  sb_printf(em->out, "\n# s%d\n", em->stmt);
  em->pre.len = 0;
  em->pipeExpand = "";
  Fx fx; memset(&fx, 0, sizeof fx);

  const Node *sel = st->kids[0];
  int isCont = (st->flags & F_CONT) || (st->flags & F_ELIDED);
  setFrame(em, isCont ? NULL : sel);

  char sv[32]; snprintf(sv, sizeof sv, "s%dm", em->stmt);
  if (isCont && em->haveSaved) {
    if (em->fr.kind == FR_ENT) stage(em, "%s ← anoN↑anoSel", sv);
    else stage(em, "%s ← anoSel", sv);
  } else if (isCont) {
    const RegEntry *cur = find(em, "cursor");
    if (!cur) return fail(em, st->line, "elided subject with no antecedent and no ^cursor alias");
    stage(em, "%s ← %s", sv, lc(em, cur->name));
  } else {
    const Node *pred = stripFrame(em, sel);
    char *msk;
    if (!pred) msk = efmt(em, "(1¨↕%s)", frN(em));
    else {
      snprintf(em->selVar, sizeof em->selVar, "%s", sv); /* for N_TO rhs inside predicates: none */
      if (emitMask(em, pred, &msk)) return -1;
    }
    stage(em, "%s ← %s", sv, msk);
  }
  snprintf(em->selVar, sizeof em->selVar, "%s", sv);

  for (int i = 1; i < st->nkids; i++)
    if (emitEffect(em, st->kids[i], &fx)) return -1;

  /* save the antecedent before structural commits (pre-spawn mask, ex49) */
  stage(em, "anoSel ↩ %s", sv);
  em->savedFr = em->fr; em->haveSaved = 1;

  if (commitStmt(em, &fx, isCont)) return -1;

  sb_printf(em->out, "%s", em->pre.s ? em->pre.s : "");
  return 0;
}

/* Inputs: a rule's selection. Output: positive / negated name literals from its top-level
 * &-chain, resolved to registry entries (defs expand; non-literals and unresolved names
 * are skipped). These are the §11 disjointness certificates: !X in one rule's guard and X
 * in another's prove the two masks share no row of one pre-state. */
#define MAXLITS 16
static void guardLits(Em *em, const Node *sel, const RegEntry **pos, int *np,
                      const RegEntry **neg, int *nn) {
  if (!sel) return;
  if (sel->kind == N_AND) {
    guardLits(em, sel->kids[0], pos, np, neg, nn);
    guardLits(em, sel->kids[1], pos, np, neg, nn);
    return;
  }
  if (sel->kind == N_SCOPE) { guardLits(em, sel->kids[0], pos, np, neg, nn); return; }
  if (sel->kind == N_NAME) {
    const Node *d = findDef(em, sel->name);
    if (d) { guardLits(em, d->kids[0], pos, np, neg, nn); return; }
    const RegEntry *e = find(em, sel->name);
    if (e && *np < MAXLITS) pos[(*np)++] = e;
    return;
  }
  if (sel->kind == N_NOT && sel->kids[0]->kind == N_NAME) {
    const Node *d = findDef(em, sel->kids[0]->name);
    if (d) return;
    const RegEntry *e = find(em, sel->kids[0]->name);
    if (e && *nn < MAXLITS) neg[(*nn)++] = e;
  }
}

/* the shared rule barrier (§11): every rule active in the tick gathers the one pre-state,
 * every effect stages into one commit set, the set scatters once. Same-column writes
 * across rules go through the §10 merge laws, widened by the guard-complement
 * certificates computed here. A single rule degenerates to the plain statement barrier. */
static int emitRuleTick(Em *em, const Node **rules, int nrules) {
  if (nrules == 1) return emitStmt(em, rules[0]);
  if (nrules > 32) return fail(em, rules[0]->line, "rule tick: more than 32 rules");
  em->stmt++;
  sb_printf(em->out, "\n# s%d: %d rules, one shared barrier\n", em->stmt, nrules);
  em->pre.len = 0;
  em->pipeExpand = "";
  Fx fx; memset(&fx, 0, sizeof fx);
  /* pairwise disjointness certificates from complementary guard literals */
  const RegEntry *pos[32][MAXLITS], *neg[32][MAXLITS];
  int np[32], nn[32];
  for (int r = 0; r < nrules; r++) {
    np[r] = nn[r] = 0;
    guardLits(em, rules[r]->kids[0], pos[r], &np[r], neg[r], &nn[r]);
    em->ruleDisj[r] = 0;
  }
  for (int r = 0; r < nrules; r++)
    for (int s = 0; s < nrules; s++) {
      if (r == s) continue;
      int dis = 0;
      for (int i = 0; i < np[r] && !dis; i++)
        for (int j = 0; j < nn[s] && !dis; j++) dis = pos[r][i] == neg[s][j];
      for (int i = 0; i < nn[r] && !dis; i++)
        for (int j = 0; j < np[s] && !dis; j++) dis = neg[r][i] == pos[s][j];
      if (dis) em->ruleDisj[r] |= 1u << s;
    }
  /* one tick, one frame: every rule must select in the same habitat */
  setFrame(em, rules[0]->kids[0]);
  Frame f0 = em->fr;
  for (int r = 1; r < nrules; r++) {
    setFrame(em, rules[r]->kids[0]);
    if (em->fr.kind != f0.kind || em->fr.w != f0.w || em->fr.h != f0.h)
      return fail(em, rules[r]->line, "rule tick: rules select different frames");
  }
  em->fr = f0;
  /* every mask against the one pre-state */
  char *masks[32] = {0};
  for (int r = 0; r < nrules; r++) {
    const Node *pred = stripFrame(em, rules[r]->kids[0]);
    char *msk;
    if (!pred) msk = efmt(em, "(1¨↕%s)", frN(em));
    else if (emitMask(em, pred, &msk)) return -1;
    masks[r] = efmt(em, "s%dr%dm", em->stmt, r);
    stage(em, "%s ← %s", masks[r], msk);
  }
  /* every effect, staged into the one commit set; reads stay pre-state (commits land
   * only in commitStmt), so ordering across rules is invisible */
  for (int r = 0; r < nrules; r++) {
    snprintf(em->selVar, sizeof em->selVar, "%s", masks[r]);
    em->curRule = r;
    for (int i = 1; i < rules[r]->nkids; i++)
      if (emitEffect(em, rules[r]->kids[i], &fx)) { em->curRule = -1; return -1; }
  }
  em->curRule = -1;
  /* the antecedent is the union of the tick's masks */
  char *uni = masks[0];
  for (int r = 1; r < nrules; r++) uni = efmt(em, "%s∨%s", uni, masks[r]);
  stage(em, "anoSel ↩ %s", uni);
  em->savedFr = em->fr; em->haveSaved = 1;
  snprintf(em->selVar, sizeof em->selVar, "%s", masks[0]);
  if (commitStmt(em, &fx, 0)) return -1;
  sb_printf(em->out, "%s", em->pre.s ? em->pre.s : "");
  return 0;
}

static int emitQuery(Em *em, const Node *st) {
  em->stmt++;
  em->pre.len = 0;
  sb_printf(em->out, "\n# q%d\n", em->stmt);
  setFrame(em, st->kids[0]);
  EV v;
  if (emitVal(em, st->kids[0], MODE_WORLD, &v)) return -1;
  char *qv = efmt(em, "q%d", em->stmt);
  stage(em, "%s ← %s", qv, v.v);
  /* match against the next --! out expectation */
  int oi = -1, seen = 0;
  for (int i = 0; i < em->dirs->nexpects; i++) {
    if (!em->dirs->expects[i].isOut) continue;
    if (seen == em->outIdx) { oi = i; break; }
    seen++;
  }
  if (oi >= 0) {
    em->outIdx++;
    const Expect *ex = &em->dirs->expects[oi];
    int allNum = 1;
    char *lst = efmt(em, "⟨");
    for (int i = 0; i < ex->nvals; i++) {
      char *w = ex->vals[i];
      char *piece;
      if ((w[0] >= '0' && w[0] <= '9') || w[0] == '-' || w[0] == '.')
        piece = w[0] == '-' ? efmt(em, "¯%s", w + 1) : efmt(em, "%s", w);
      else { piece = efmt(em, "\"%s\"", w); allNum = 0; }
      lst = efmt(em, "%s%s%s", lst, i ? ", " : "", piece);
    }
    lst = efmt(em, "%s⟩", lst);
    /* numeric outs compare within 1e-9: pinned doubles come from a sibling BQN
     * evaluation whose association order may differ in the last bits */
    if (allNum)
      stage(em, "\"out q%d\" ! %s {(≠𝕨)≠≠𝕩 ? 0 ; ∧´1e¯9≥|𝕨-𝕩} ⥊%s", em->stmt, lst, qv);
    else
      stage(em, "\"out q%d\" ! %s ≡ ⥊%s", em->stmt, lst, qv);
  } else {
    stage(em, "•Show %s", qv);
  }
  sb_printf(em->out, "%s", em->pre.s ? em->pre.s : "");
  return 0;
}

/* comprehension: theta-join over two generators with filters; effects tag both sides */
static int emitCompr(Em *em, const Node *st) {
  em->stmt++;
  em->pre.len = 0;
  sb_printf(em->out, "\n# c%d\n", em->stmt);
  em->fr.kind = FR_ENT;
  const Node *sel = st->kids[0];
  const Node *eff = st->kids[1];
  const Node *binders[2] = {0}; int nb = 0;
  const Node *filters[8] = {0}; int nf = 0;
  for (int i = 2; i < st->nkids; i++) {
    if (st->kids[i]->kind == N_BINDER) { if (nb < 2) binders[nb++] = st->kids[i]; }
    else filters[nf++] = st->kids[i];
  }
  if (nb != 2) return fail(em, st->line, "comprehension needs two generators");
  char *am, *bm;
  if (emitMask(em, binders[0]->kids[0], &am)) return -1;
  if (emitMask(em, binders[1]->kids[0], &bm)) return -1;
  char *aI = tv(em), *bI = tv(em);
  stage(em, "%s ← /%s", aI, am);
  stage(em, "%s ← /%s", bI, bm);
  /* pair filter matrix, all-ones then AND each filter in */
  char *M = tv(em);
  stage(em, "%s ← (≠%s)‿(≠%s)⥊1", M, aI, bI);
  for (int i = 0; i < nf; i++) {
    const Node *f = filters[i];
    if (f->kind == N_CMP && f->kids[0]->kind == N_NAME && f->kids[1]->kind == N_NAME &&
        !strcmp(f->kids[0]->name, binders[0]->name) && !strcmp(f->kids[1]->name, binders[1]->name)) {
      const char *op = f->op == '<' ? "<" : f->op == '>' ? ">" : f->op == '=' ? "=" : "≠";
      stage(em, "%s ↩ %s∧%s%s⌜%s", M, M, aI, op, bI);
    } else if (f->kind == N_CMP && f->kids[0]->kind == N_CALL) {
      const Node *c = f->kids[0];
      const RegEntry *e = find(em, c->name);
      if (!e) return fail(em, f->line, "unregistered '%s' in comprehension filter", c->name);
      EV rv; if (emitVal(em, f->kids[1], MODE_WORLD, &rv)) return -1;
      const char *op = f->op == '<' ? "<" : f->op == '>' ? ">" : f->op == 'l' ? "≤" : "≥";
      stage(em, "%s ↩ %s∧((%s %s⌜ %s)%s%s)", M, M, aI, fnv(em, e->name), bI, op, rv.v);
    } else if (f->kind == N_CALL) {
      const RegEntry *e = find(em, f->name);
      if (!e) return fail(em, f->line, "unregistered '%s' in comprehension filter", f->name);
      stage(em, "%s ↩ %s∧(%s %s⌜ %s)", M, M, aI, fnv(em, e->name), bI);
    } else return fail(em, f->line, "unsupported comprehension filter");
  }
  /* effect over both sides: rows/cols with any surviving pair */
  char *aAny = tv(em), *bAny = tv(em);
  stage(em, "%s ← ∨´˘%s", aAny, M);
  stage(em, "%s ← ∨´˘⍉%s", bAny, M);
  char *sideMask = tv(em);
  stage(em, "%s ← ((↕anoN)∊%s/%s)∨((↕anoN)∊%s/%s)", sideMask, aAny, aI, bAny, bI);
  char sv[32]; snprintf(sv, sizeof sv, "%s", sideMask);
  snprintf(em->selVar, sizeof em->selVar, "%s", sv);
  Fx fx; memset(&fx, 0, sizeof fx);
  const Node *effs[8]; int ne = 0;
  effs[ne++] = eff;
  for (int i = 0; i < ne; i++) {
    const Node *e2 = effs[i];
    if (e2->kind == N_EVERB && e2->nkids == 0) {
      /* bare verb tag (Collide): treat as +Name when a bool col exists */
      const RegEntry *ce = find(em, e2->name);
      if (ce && ce->kind == RK_COL && ce->type == CT_BOOL) {
        char *t = tv(em);
        stage(em, "%s ← %s∨%s", t, lc(em, ce->name), sv);
        addCommit(em, &fx, entIdx(em, ce), t, '|', NULL);
        continue;
      }
    }
    if (emitEffect(em, e2, &fx)) return -1;
  }
  stage(em, "anoSel ↩ %s", sv);
  em->savedFr = em->fr; em->haveSaved = 1;
  if (commitStmt(em, &fx, 0)) return -1;
  sb_printf(em->out, "%s", em->pre.s ? em->pre.s : "");
  (void)sel;
  return 0;
}

/* ---------- fixture + expectations ---------- */

static void emitFixture(Em *em) {
  const Registry *r = em->reg;
  sb_printf(em->out, "\n# fixture\n");
  sb_printf(em->out, "anoN ← %d\n", r->n);
  sb_printf(em->out, "anoSel ← ⟨⟩\n");
  for (int i = 0; i < r->nents; i++) {
    const RegEntry *e = &r->ents[i];
    char *v = lc(em, e->name);
    switch (e->kind) {
      case RK_COL: case RK_FIELD: {
        int n = e->kind == RK_FIELD ? r->latW * r->latH : r->n;
        if (e->type == CT_SYM) {
          sb_printf(em->out, "%s ← ⟨", v);
          for (int k = 0; k < e->nsyms; k++) sb_printf(em->out, "%s\"%s\"", k ? ", " : "", e->syms[k]);
          sb_printf(em->out, "⟩\n");
        } else if (e->type == CT_CHAR) {
          sb_printf(em->out, "%s ← \"%s\"\n", v, e->syms ? e->syms[0] : "");
        } else if (e->nnums == 2 * n && n > 0) {
          em->isPair[i] = 1;
          sb_printf(em->out, "%s ← ⟨", v);
          for (int k = 0; k < n; k++) {
            char a[64], b[64];
            snprintf(a, sizeof a, "%s", numLit(em, e->nums[2*k]));
            snprintf(b, sizeof b, "%s", numLit(em, e->nums[2*k+1]));
            sb_printf(em->out, "%s%s‿%s", k ? ", " : "", a, b);
          }
          sb_printf(em->out, "⟩\n");
        } else {
          sb_printf(em->out, "%s ← ⟨", v);
          for (int k = 0; k < e->nnums; k++) sb_printf(em->out, "%s%s", k ? ", " : "", numLit(em, e->nums[k]));
          sb_printf(em->out, "⟩\n");
        }
        if (e->hasPres) {
          sb_printf(em->out, "pres_%s ← ⟨", e->name);
          for (int k = 0; k < r->n; k++) sb_printf(em->out, "%s%s", k ? ", " : "", numLit(em, e->pres[k]));
          sb_printf(em->out, "⟩\n");
        }
        break;
      }
      case RK_REL: case RK_ALIAS: {
        sb_printf(em->out, "%s ← ⟨", v);
        for (int k = 0; k < e->nnums; k++) sb_printf(em->out, "%s%s", k ? ", " : "", numLit(em, e->nums[k]));
        sb_printf(em->out, "⟩\n");
        break;
      }
      case RK_SREL: {
        sb_printf(em->out, "%s ← ⟨", v);
        for (int f = 0; f < e->nfib; f++) {
          sb_printf(em->out, "%s⟨", f ? ", " : "");
          for (int k = 0; k < e->fibLen[f]; k++)
            sb_printf(em->out, "%s%s", k ? ", " : "", numLit(em, e->fibVals[e->fibOff[f] + k]));
          sb_printf(em->out, "⟩");
        }
        sb_printf(em->out, "⟩\n");
        break;
      }
      case RK_BIND: {
        if (!strcmp(e->bindKind, "mask") || !strcmp(e->bindKind, "vec")) {
          sb_printf(em->out, "%s ← ⟨", v);
          for (int k = 0; k < e->nnums; k++) sb_printf(em->out, "%s%s", k ? ", " : "", numLit(em, e->nums[k]));
          sb_printf(em->out, "⟩\n");
        } else if (!strcmp(e->bindKind, "num")) {
          sb_printf(em->out, "%s ← %s\n", v, numLit(em, e->nums[0]));
        }
        break;
      }
      case RK_FN: {
        if (e->syms && e->syms[0][0]) {
          /* raw form: either "<dfn>" or "<targetcol> <dfn>" (verbs) — bind the dfn part */
          const char *raw = e->syms[0];
          const char *br = strchr(raw, '{');
          if (br && br != raw) sb_printf(em->out, "%s ← %s\n", fnv(em, e->name), br);
          else if (br) sb_printf(em->out, "%s ← %s\n", fnv(em, e->name), raw);
        }
        break;
      }
      default: break;
    }
  }
}

static int emitExpects(Em *em) {
  sb_printf(em->out, "\n# expectations\n");
  const Directives *d = em->dirs;
  for (int i = 0; i < d->nexpects; i++) {
    const Expect *ex = &d->expects[i];
    if (ex->isOut) continue;
    const RegEntry *e = find(em, ex->col);
    if (!e) { snprintf(em->err, em->errsz, "expect: unknown column '%s'", ex->col); return -1; }
    char *v = lc(em, e->name);
    int sym = (e->kind == RK_COL || e->kind == RK_FIELD) && e->type == CT_SYM;
    char *lst = efmt(em, "⟨");
    for (int k = 0; k < ex->nvals; k++) {
      char *w = ex->vals[k];
      char *piece;
      if (sym) piece = efmt(em, "\"%s\"", w);
      else piece = w[0] == '-' ? efmt(em, "¯%s", w + 1) : efmt(em, "%s", w);
      lst = efmt(em, "%s%s%s", lst, k ? ", " : "", piece);
    }
    lst = efmt(em, "%s⟩", lst);
    char *rhs = em->isPair[entIdx(em, e)] ? efmt(em, "∾%s", v) : v;
    /* numeric expects compare within 1e-9 (float pins from a sibling evaluation);
     * sym columns stay exact */
    if (sym)
      sb_printf(em->out, "\"expect %s\" ! %s ≡ %s\n", ex->col, lst, rhs);
    else
      sb_printf(em->out, "\"expect %s\" ! %s {(≠𝕨)≠≠𝕩 ? 0 ; ∧´1e¯9≥|𝕨-𝕩} %s\n", ex->col, lst, rhs);
  }
  if (d->expectN >= 0)
    sb_printf(em->out, "\"expect-n\" ! %d ≡ anoN\n", d->expectN);
  sb_printf(em->out, "\n\"ok\"\n");
  return 0;
}

/* ---------- entry ---------- */

int ano_emit(const Node *prog, const Registry *reg, const Directives *dirs,
             StrBuf *out, char *err, size_t errsz) {
  Em em; memset(&em, 0, sizeof em);
  Arena a = {0};
  em.reg = reg; em.dirs = dirs; em.out = out; em.a = &a;
  em.err = err; em.errsz = errsz;
  em.fr.kind = FR_ENT; em.fr.lit = "";
  em.savedFr.kind = FR_ENT; em.savedFr.lit = "";
  em.pipeExpand = "";
  em.curRule = -1;

  emitFixture(&em);

  /* def only installs; the clock fires. anoc has no clock, so it pretends one clock
   * edge at the end of each unbroken run of installs (plain defs don't end a run; a
   * performed statement or EOF does). Each edge fires EVERY rule installed so far in
   * one shared barrier — installs persist, an earlier rule fires again at a later
   * edge exactly as it would on the engine's next tick. */
  const Node *installed[32]; int ninst = 0, fresh = 0;
  int rc = 0;
  for (int i = 0; i < prog->nkids && !rc; i++) {
    const Node *st = prog->kids[i];
    const Node *rule = NULL;
    if (st->kind == N_DEFSTMT) {
      if (em.ndefs < 128) em.defs[em.ndefs++] = st;
      if (st->kids[0]->kind == N_STMT) rule = st->kids[0];
    } else if (st->kind == N_STMT && (st->flags & F_RULE)) rule = st;
    if (rule) {
      if (ninst >= 32) { snprintf(err, errsz, "emit: more than 32 installed rules"); rc = -1; break; }
      installed[ninst++] = rule;
      fresh = 1;
      continue;
    }
    if (st->kind == N_DEFSTMT) continue;
    if (fresh) { rc = emitRuleTick(&em, installed, ninst); fresh = 0; if (rc) break; }
    switch (st->kind) {
      case N_STMT: rc = emitStmt(&em, st); break;
      case N_QUERY: rc = emitQuery(&em, st); break;
      case N_COMPR: rc = emitCompr(&em, st); break;
      default: snprintf(err, errsz, "emit: unexpected top-level node %d", st->kind); rc = -1;
    }
  }
  if (!rc && fresh) rc = emitRuleTick(&em, installed, ninst);
  if (!rc) rc = emitExpects(&em);
  sb_free(&em.pre);
  arena_free(&a);
  return rc;
}
