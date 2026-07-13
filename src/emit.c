/* emit.c — AST -> BQN codegen for anoc. Ordinary statements read pre-state, stage effects,
 * then commit at the barrier. Installed rules share one pre-state and commit set per
 * synthetic tick. Continuations reuse anoSel, padded to the current entity count.
 * Conventions the .reg fixtures rely on:
 *   - columns emit as their registry spelling (first letter lowercased); a name that is
 *     not a BQN-legal identifier emits as jp<i> by registry index, the human spelling
 *     kept as a comment on its fixture line; presence masks emit as pres_<name> or
 *     pres_jp<i>; set-valued rels emit as fiber lists.
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
                 char *along;   /* scan-along order used to restore row order before scatter */
                 const RegEntry *relEnt; /* the rel whose values v holds (chain hops resolve
                                   the next leg through THIS rel's key space) */
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
  char traceSel[32];     /* the statement's selection BEFORE RHS-guard refinement (--trace):
                            "row failure" means selected then dropped by the guard, and the
                            refined selVar has already dropped them */
  char cntVar[32];       /* copy-space counts (spawn replicate) */
  char idxVar[64];       /* copy-space per-copy index */
  const char *pipeExpand;  /* selection-space counts from a pipeline expand stage (arena) */
  /* hop integrity: needIdx is the pre-scan verdict — the program reads through the idx
   * key space after a despawn can have committed, so the fixture materializes the row
   * iota ONCE as the hidden column anoIdx, filtered through despawns and minted on
   * spawns; worldShifted flips when the first despawn commits, and from then on every
   * idx-keyed read resolves by ⊐ against anoIdx instead of the positional gather */
  int needIdx;
  int worldShifted;
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

static const RegEntry *find(Em *em, const char *n) { return reg_find(em->reg, n); }
static int entIdx(Em *em, const RegEntry *e) { return (int)(e - em->reg->ents); }

/* BQN identifier legality: [A-Za-z][A-Za-z0-9_]*. Legal names emit through lc/pres_/Fn_
 * exactly as always — the emitted BQN for every ASCII registry is a hard invariant. */
static int bqnlegal(const char *n) {
  if (!((n[0] >= 'A' && n[0] <= 'Z') || (n[0] >= 'a' && n[0] <= 'z'))) return 0;
  for (const char *p = n + 1; *p; p++)
    if (!((*p >= 'A' && *p <= 'Z') || (*p >= 'a' && *p <= 'z') ||
          (*p >= '0' && *p <= '9') || *p == '_')) return 0;
  return 1;
}

/* Inputs: a registry entry. Output: its BQN variable spelling — lc(name) when BQN-legal,
 * else jp<i> by registry index: deterministic, stable within a compile, disjoint from
 * emitter temporaries. The human spelling rides as a comment at the fixture line. */
static char *bqnv(Em *em, const RegEntry *e) {
  if (bqnlegal(e->name)) return lc(em, e->name);
  return efmt(em, "jp%d", entIdx(em, e));
}

/* the presence-mask spelling beside bqnv: pres_<raw name> stays the fixture convention */
static char *presv(Em *em, const RegEntry *e) {
  if (bqnlegal(e->name)) return efmt(em, "pres_%s", e->name);
  return efmt(em, "pres_jp%d", entIdx(em, e));
}

/* registry-fn BQN spelling: Fn_ prefix keeps case-insensitive BQN identifiers from
 * colliding with a column of the same name (fn threat vs col threat) */
static char *fnv(Em *em, const RegEntry *e) {
  if (bqnlegal(e->name)) return efmt(em, "Fn_%s", e->name);
  return efmt(em, "Fn_jp%d", entIdx(em, e));
}

/* defs are program variables: exact-byte match, outside the registry's case contract */
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
/* the world's stable-id column, by reg_role's one ladder (role-declared id/keys, else a
 * declared unique column — it declares what the names guess — else the magic name), else
 * the hidden anoIdx once the world has shifted, else the row iota. Relationship values
 * denote these ids; membership survives structural compaction (ex12). */
static char *idCol(Em *em) {
  const RegEntry *e = reg_role(em->reg, "id");
  if (!e) e = reg_role(em->reg, "keys");
  if (e && e->kind == RK_COL) return bqnv(em, e);
  if (em->needIdx && em->worldShifted) return "anoIdx";
  return efmt(em, "(↕%s)", frN(em));
}

/* the key expression a functional rel resolves through: its declared unique column, the
 * hidden fixture-row column once rows have shifted, else NULL — the positional identity.
 * NULL keeps today's `0⌈` gather byte-for-byte; a key means one ⊐ with one found-guard,
 * the keyed hop rel;unique⁻¹ (injectivity is the inversion license). */
static char *relKey(Em *em, const RegEntry *rel) {
  if (rel->keyOf[0]) {
    const RegEntry *kc = reg_find(em->reg, rel->keyOf);
    if (kc) return bqnv(em, kc);
  }
  if (em->needIdx && em->worldShifted) return "anoIdx";
  return NULL;
}

/* the has-a-live-target guard for a bare rel: positional worlds keep the 0≤ sentinel
 * test; a keyed or shifted world tests found-ness against the key column */
static char *relGuard(Em *em, const RegEntry *e) {
  char *key = relKey(em, e);
  if (!key) return efmt(em, "(0≤%s)", bqnv(em, e));
  return efmt(em, "((%s⊐%s)<≠%s)", key, bqnv(em, e), key);
}

/* trace origin ids (--trace): the world's id column by idCol's own ladder, but the iota
 * fallback sizes by the rel column, not the frame — a hop under a lattice frame still
 * crosses entity-length links. World space, length ≠rel. */
static char *traceIds(Em *em, const char *relExpr) {
  const RegEntry *e = reg_role(em->reg, "id");
  if (!e) e = reg_role(em->reg, "keys");
  if (e && e->kind == RK_COL) return bqnv(em, e);
  if (em->needIdx && em->worldShifted) return "anoIdx";
  return efmt(em, "(↕≠%s)", relExpr);
}

/* the dead-link diagnostic (--trace): one RELATION <column> <origin> -> <sink> IS DEAD !
 * line per link that was set (0≤rel — ¯1 was never linked) yet fails the found-guard —
 * the target no longer exists. Staged beside the hop it observes; pure output, world
 * space, self-contained (the guard convention), zero lines emitted without the flag.
 * g (NULL = total) is the accumulated guard of earlier legs: a chain row already dead
 * upstream wraps through the clamp and must not report a crossing it never made. */
static void traceDead(Em *em, const char *name, char *key, char *rel, char *g) {
  if (!em->dirs->trace) return;
  char *dm = tv(em);
  if (g) stage(em, "%s ← %s∧(0≤%s)∧(≠%s)≤%s⊐%s", dm, g, rel, key, key, rel);
  else stage(em, "%s ← (0≤%s)∧(≠%s)≤%s⊐%s", dm, rel, key, key, rel);
  stage(em, "AnoTraceDead ⟨\"%s\", %s/%s, %s/%s⟩", name, dm, traceIds(em, rel), dm, rel);
}

/* guard conjunction; NULL = total */
static char *gAnd(Em *em, char *a, char *b) {
  if (!a) return b;
  if (!b) return a;
  return efmt(em, "(%s)∧(%s)", a, b);
}

static int emitVal(Em *em, const Node *nd, Mode m, EV *ev);
static int emitMask(Em *em, const Node *nd, char **out);

/* number spelling with BQN high-minus — every '-' (sign and exponent alike, so a
 * saved-world 1e-09 re-emits as 1e¯09) and no C '+' exponent; the range guard runs
 * before the cast, which is UB on out-of-range doubles */
static char *numLit(Em *em, double x) {
  char buf[64];
  if (x >= -9e15 && x <= 9e15 && x == (long long)x) snprintf(buf, sizeof buf, "%lld", (long long)x);
  else snprintf(buf, sizeof buf, "%.17g", x);
  char out[136];
  int o = 0;
  for (const char *p = buf; *p && o < 130; p++) {
    if (*p == '-') { out[o++] = '\xC2'; out[o++] = '\xAF'; }
    else if (*p == '+') continue;
    else out[o++] = *p;
  }
  out[o] = 0;
  return efmt(em, "%s", out);
}

/* gather a world-space column expr into the current mode */
static char *inMode(Em *em, char *worldExpr, Mode m) {
  if (m == MODE_WORLD) return worldExpr;
  if (m == MODE_SEL) return efmt(em, "(%s/%s)", em->selVar, worldExpr);
  return efmt(em, "(%s/%s/%s)", em->cntVar, em->selVar, worldExpr);
}

/* presence mask for a column entry, world space; NULL when total */
static char *presOf(Em *em, const RegEntry *e) {
  if (e->kind == RK_COL && e->hasPres) return presv(em, e);
  return NULL;
}

/* Inputs: a derived-tag entry. Output: 0 with *out the world-space mask the tag
 * denotes — present(carrier) ∧ carrier = value, recomputed against the live column
 * (the left-join-null rule) — or -1 on a dangling carrier. A tag binds no fixture
 * variable; every use, bare or hopped-onto, expands through here. */
static int tagMask(Em *em, const RegEntry *e, int line, char **out) {
  const RegEntry *c = find(em, e->tagCol);
  if (!c) return fail(em, line, "derived tag '%s': carrier '%s' unregistered", e->name, e->tagCol);
  char *eq = e->type == CT_SYM ? efmt(em, "((<\"%s\")≡¨%s)", e->syms[0], bqnv(em, c))
                               : efmt(em, "(%s=%s)", bqnv(em, c), numLit(em, e->nums[0]));
  *out = (c->kind == RK_COL && c->hasPres) ? efmt(em, "(%s∧%s)", presv(em, c), eq) : eq;
  return 0;
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
      char *v = bqnv(em, e);
      ev->pair = em->isPair[entIdx(em, e)];
      ev->sym = (e->type == CT_SYM);
      ev->g = presOf(em, e);
      ev->v = inMode(em, v, m);
      return 0;
    }
    case RK_TAG: {
      /* Recomputed mask; no separate guard. */
      char *mk; if (tagMask(em, e, nd->line, &mk)) return -1;
      ev->v = inMode(em, mk, m);
      return 0;
    }
    case RK_REL: { ev->v = inMode(em, bqnv(em, e), m); ev->g = relGuard(em, e); ev->relEnt = e; return 0; }
    case RK_ALIAS: { ev->v = inMode(em, bqnv(em, e), m); return 0; }
    case RK_BIND:
      if (!strcmp(e->bindKind, "num")) { ev->v = numLit(em, e->nums[0]); ev->unit = 1; return 0; }
      if (!strcmp(e->bindKind, "point")) {
        ev->v = efmt(em, "(<%s‿%s)", numLit(em, e->nums[0]), numLit(em, e->nums[1]));
        ev->pair = 1; ev->unit = 1; return 0;
      }
      if (!strcmp(e->bindKind, "entity")) { ev->v = numLit(em, e->nums[0]); ev->unit = 1; return 0; }
      if (!strcmp(e->bindKind, "mask")) { ev->v = inMode(em, bqnv(em, e), m); return 0; }
      if (!strcmp(e->bindKind, "vec")) { ev->v = bqnv(em, e); ev->unit = 1; return 0; }
      return fail(em, nd->line, "binding '%s' of kind %s in value position", n, e->bindKind);
    case RK_SREL: { ev->v = bqnv(em, e); ev->unit = 1; return 0; }
    case RK_PROTO: return fail(em, nd->line, "proto '%s' in value position: a proto is spawned, never read", n);
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
  char *v = bqnv(em, e);
  switch (e->kind) {
    case RK_COL: case RK_FIELD:
      if (e->type == CT_BOOL) { *out = e->hasPres ? efmt(em, "(%s∧%s)", presv(em, e), v) : v; return 0; }
      *out = e->hasPres ? presv(em, e) : efmt(em, "(1¨%s)", v);
      return 0;
    case RK_TAG: return tagMask(em, e, nd->line, out);
    case RK_ALIAS: *out = v; return 0;
    case RK_BIND:
      if (!strcmp(e->bindKind, "mask")) { *out = v; return 0; }
      if (!strcmp(e->bindKind, "entity")) { *out = efmt(em, "((↕anoN)=%s)", numLit(em, e->nums[0])); return 0; }
      return fail(em, nd->line, "binding '%s' (%s) as mask", n, e->bindKind);
    case RK_REL: *out = relGuard(em, e); return 0;
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
      const RegEntry *fe = find(em, st->name);
      if (!fe) return fail(em, st->line, "unregistered callable '%s'", st->name);
      char *args = efmt(em, "⟨%s", vw->idx);
      for (int k = 0; k < st->nkids; k++) {
        EV av; if (emitVal(em, st->kids[k], MODE_WORLD, &av)) return -1;
        args = efmt(em, "%s, %s", args, av.v);
      }
      vw->idx = efmt(em, "(%s %s⟩)", fnv(em, fe), args);
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
  if (e && e->kind == RK_SREL) { *out = bqnv(em, e); return 0; }
  if (e && e->kind == RK_COL && e->type == CT_NUM) {
    char *t = tv(em);
    stage(em, "%s ← {/%s=𝕩}¨%s", t, bqnv(em, e), idCol(em));
    *out = t;
    return 0;
  }
  return fail(em, nd->line, "'%s' is not a set-valued relationship or a key column", nd->name);
}

/* gamma fibers as CURRENT row indices. Stored srel fibers hold ids in the srel's key
 * space; a gamma gathers by row, so the fiber translates through the key (one ⊐ per
 * member, dead members dropped — left-join-null inside the fiber) exactly when the two
 * spaces can disagree: a declared key always, the idx key once the world has shifted.
 * Otherwise the pre-shift fiber IS its row set and the staged var passes through
 * untouched — today's bytes. Key-column fibers (fiberVar's second form) are computed
 * against the live world and are already rows. */
static void fiberRows(Em *em, const Node *nd, char **fib) {
  const RegEntry *e = find(em, nd->name);
  if (!e || e->kind != RK_SREL || e->nfib != em->reg->n) return;
  char *key = NULL;
  if (e->keyOf[0]) {
    const RegEntry *kc = find(em, e->keyOf);
    if (kc) key = bqnv(em, kc);
  } else if (em->worldShifted) key = idCol(em);
  if (!key) return;
  /* --trace: a dead member is a dead link crossed inside the fiber — same RELATION
   * line, origin the fiber's row, sink the member key that resolves nowhere */
  if (em->dirs->trace)
    stage(em, "%s {m←(≠%s)≤%s⊐𝕩 ⋄ AnoTraceDead ⟨\"%s\", (+´m)⥊𝕨, m/𝕩⟩}¨ %s",
          traceIds(em, *fib), key, key, e->name, *fib);
  char *t = tv(em);
  stage(em, "%s ← {k←%s⊐𝕩 ⋄ (k<≠%s)/k}¨%s", t, key, key, *fib);
  *fib = t;
}

/* --trace: the empty-fiber failure mask. In an effect (m ≠ world) it scopes to the
 * statement's pre-refinement selection — a row never selected never failed, and the
 * refined selVar has already dropped exactly the failures; a predicate fold crosses
 * every row and stays whole-column. */
static char *traceEmptyMask(Em *em, char *fib, Mode m) {
  if (m != MODE_WORLD && em->traceSel[0])
    return efmt(em, "(%s∧0=≠¨%s)", em->traceSel, fib);
  return efmt(em, "(0=≠¨%s)", fib);
}

/* gamma fold: fold/ rel'.Comp | fold/ (rel' & pred) | fold/ rel'  -> per-source column + guard */
static int emitGamma(Em *em, const char *op, const Node *operand, Mode m, EV *ev) {
  memset(ev, 0, sizeof *ev);
  char *fib = NULL, *body = NULL;
  if (operand->kind == N_SETHOP) {           /* #/ attackers' */
    if (fiberVar(em, operand->kids[0], &fib)) return -1;
    fiberRows(em, operand->kids[0], &fib);
    body = efmt(em, "≠¨%s", fib);
    if (strcmp(op, "#")) { /* other folds over bare fiber make no sense */
      return fail(em, operand->line, "bare rel' under %s/", op);
    }
    ev->v = inMode(em, efmt(em, "(%s)", body), m);
    return 0;
  }
  if (operand->kind == N_HOP && operand->kids[0]->kind == N_SETHOP) { /* fold/ rel'.Comp */
    if (fiberVar(em, operand->kids[0]->kids[0], &fib)) return -1;
    fiberRows(em, operand->kids[0]->kids[0], &fib);
    /* --trace: an identityless fold over an empty fiber is the empty-fiber row failure —
     * the guard drops the row silently, the trace names it */
    const Node *fbn = operand->kids[0]->kids[0];
    const char *fbName = fbn->kind == N_NAME ? fbn->name : "fiber";
    EV cv; if (emitVal(em, operand->kids[1], MODE_WORLD, &cv)) return -1;
    const char *gl = foldGl(op);
    char *t = tv(em);
    if (!strcmp(op, "avg")) {
      if (em->dirs->trace)
        stage(em, "AnoTraceEmpty ⟨\"%s\", %s/%s⟩", fbName, traceEmptyMask(em, fib, m), traceIds(em, fib));
      stage(em, "%s ← {0=≠𝕩 ? 0 ; AnoAvg 𝕩⊏%s}¨%s", t, cv.v, fib);
      ev->g = efmt(em, "(0<≠¨%s)", fib);
    } else if (!strcmp(op, "#")) {
      stage(em, "%s ← {+´𝕩⊏%s}¨%s", t, cv.v, fib);
    } else if (gl && foldHasId(op)) {
      stage(em, "%s ← {%s𝕩⊏%s}¨%s", t, gl, cv.v, fib);
    } else if (gl) { /* max/min: no identity, guard empties */
      if (em->dirs->trace)
        stage(em, "AnoTraceEmpty ⟨\"%s\", %s/%s⟩", fbName, traceEmptyMask(em, fib, m), traceIds(em, fib));
      stage(em, "%s ← {0=≠𝕩 ? 0 ; %s𝕩⊏%s}¨%s", t, gl, cv.v, fib);
      ev->g = efmt(em, "(0<≠¨%s)", fib);
    } else {
      const RegEntry *e = find(em, op);
      if (e && e->kind == RK_FN)
        return fail(em, operand->line, "named reducer '%s' over fibers is not yet supported", op);
      return fail(em, operand->line, "unknown reducer '%s'", op);
    }
    ev->v = inMode(em, t, m);
    if (ev->g) ev->g = ev->g; /* world-space guard */
    return 0;
  }
  if (operand->kind == N_AND && operand->kids[0]->kind == N_SETHOP) { /* fold/ (rel' & pred) */
    if (fiberVar(em, operand->kids[0]->kids[0], &fib)) return -1;
    fiberRows(em, operand->kids[0]->kids[0], &fib);
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
    fiberRows(em, operand->kids[0], &fib);
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
    stage(em, "%s ← {0=≠𝕩 ? 0 ; %s´ 𝕩} %s", t, fnv(em, e), gathered);
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

/* Inputs: expr subtree. Output: 1 when a til generator appears anywhere in it. */
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
  const char *gl = !strcmp(op, "+") ? "+`" : !strcmp(op, "*") ? "×`" : !strcmp(op, "max") ? "⌈`"
                 : !strcmp(op, "&") ? "∧`" : !strcmp(op, "|") ? "∨`" : NULL;
  if (!gl) { /* named reducer scan: registry fn accumulates pairwise; the empty scope
              * yields the empty column — a scan is length-preserving, no identity consulted */
    const RegEntry *e = find(em, op);
    if (!e || e->kind != RK_FN) return fail(em, nd->line, "unknown scan op '%s'", op);
    gl = efmt(em, "%s`", fnv(em, e));
  }
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
  const char *fn = e ? fnv(em, e) : uc(em, nd->name);
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
      ev->v = inMode(em, efmt(em, "(%d⊸⊑¨%s)", i, bqnv(em, e)), m);
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
                                     : efmt(em, "(⊑/%s)", bqnv(em, be));
      int pair = em->isPair[entIdx(em, fe)];
      ev->v = pair ? efmt(em, "(<%s⊑%s)", id, bqnv(em, fe))
                   : efmt(em, "(%s⊑%s)", id, bqnv(em, fe));
      ev->pair = pair; ev->unit = 1;
      return 0;
    }
    /* functional relationship hop: rel.Comp with ¯1 dangling. Keyed (or post-shift):
     * one ⊐ against the key column, one found-guard — not-found is dangling is dead,
     * so the ¯1 sentinel and a despawned target fail the same test (left-join-null);
     * the (≠key)| clamp is the 0⌈ clamp's keyed twin, dead weight under the guard. */
    if (be && be->kind == RK_REL && fe) {
      char *rel = bqnv(em, be);
      char *comp;
      if (fe->kind == RK_TAG) {   /* the tag is its recomputed mask; the hop indexes it */
        if (tagMask(em, fe, nd->line, &comp)) return -1;
      } else comp = bqnv(em, fe);
      char *key = relKey(em, be);
      char *w;
      if (key) {
        /* every piece stays a self-contained expression: an assignment probe reuses the
         * guard after discarding the staging buffer, so no temp may carry it */
        traceDead(em, be->name, key, rel, NULL);
        char *ix = efmt(em, "((≠%s)|%s⊐%s)", key, key, rel);
        w = efmt(em, "(%s⊏%s)", ix, comp);
        ev->g = efmt(em, "((%s⊐%s)<≠%s)", key, rel, key);
        if (fe->kind != RK_TAG && fe->hasPres)
          ev->g = gAnd(em, ev->g, efmt(em, "(%s⊏%s)", ix, presv(em, fe)));
      } else {
        w = efmt(em, "((0⌈%s)⊏%s)", rel, comp);
        ev->g = efmt(em, "(0≤%s)", rel);
        if (fe->kind != RK_TAG && fe->hasPres)
          ev->g = gAnd(em, ev->g, efmt(em, "((0⌈%s)⊏%s)", rel, presv(em, fe)));
      }
      ev->sym = fe->kind != RK_TAG && fe->type == CT_SYM;
      if (fe->kind == RK_REL) ev->relEnt = fe;
      ev->v = inMode(em, w, m);
      return 0;
    }
    /* chained hop: rel.rel2.Comp */
    if (be && be->kind == RK_REL && field->kind == N_HOP) {
      return fail(em, nd->line, "nested hop chains beyond one level: spell left-assoc");
    }
  }
  /* left-assoc chain: (rel.rel).Comp — the next leg resolves through the key space of
   * the rel whose VALUES the base gathered (bv.relEnt), never the base rel's own */
  if (base->kind == N_HOP) {
    EV bv; if (emitHop(em, base, MODE_WORLD, &bv)) return -1;
    const RegEntry *fe = field->kind == N_NAME ? find(em, field->name) : NULL;
    if (!fe) return fail(em, nd->line, "hop target '%s' unregistered", field->name);
    char *comp;
    if (fe->kind == RK_TAG) {
      if (tagMask(em, fe, nd->line, &comp)) return -1;
    } else comp = bqnv(em, fe);
    char *key = bv.relEnt ? relKey(em, bv.relEnt)
                          : (em->needIdx && em->worldShifted ? "anoIdx" : NULL);
    char *w;
    if (key) {
      /* self-contained expressions only (the assignment-probe rule above) */
      if (bv.relEnt) traceDead(em, bv.relEnt->name, key, bv.v, bv.g);
      char *ix = efmt(em, "((≠%s)|%s⊐%s)", key, key, bv.v);
      w = efmt(em, "(%s⊏%s)", ix, comp);
      ev->g = gAnd(em, bv.g, efmt(em, "((%s⊐%s)<≠%s)", key, bv.v, key));
    } else {
      w = efmt(em, "((0⌈%s)⊏%s)", bv.v, comp);
      ev->g = gAnd(em, bv.g, efmt(em, "(0≤%s)", bv.v));
    }
    ev->sym = fe->kind != RK_TAG && fe->type == CT_SYM;
    if (fe->kind == RK_REL) ev->relEnt = fe;
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
      ev->v = e ? efmt(em, "(%s×%s)", numLit(em, nd->num), bqnv(em, e))
                : numLit(em, nd->num);
      return 0;
    }
    case N_SYM: ev->v = efmt(em, "(<\"%s\")", nd->name); ev->unit = 1; ev->sym = 1; return 0;
    case N_STR: ev->v = efmt(em, "\"%s\"", nd->name); ev->unit = 1; return 0;
    case N_NAME: return emitNameVal(em, nd, m, ev);
    case N_ALIAS: {
      const RegEntry *e = find(em, nd->name);
      if (!e) return fail(em, nd->line, "unregistered alias '^%s'", nd->name);
      ev->v = inMode(em, bqnv(em, e), m); return 0;
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
    case N_FOLD: {
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
      ev->v = efmt(em, "(⥊(/%s)%s⌜(/%s))", am, fnv(em, e), bm);
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
  if (e && (e->kind == RK_FIELD || e->kind == RK_COL)) return bqnv(em, e);
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
      *out = bqnv(em, e);
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
          *out = efmt(em, "(¬%s)", presv(em, e));
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
        *out = e->hasPres ? presv(em, e) : efmt(em, "(1¨%s)", bqnv(em, e));
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
        stage(em, "%s ← {%s ⟨𝕩, ⊑%s%s⟩}¨(%s⋈¨%s)", t, fnv(em, e), org.v, args, xs, ys);
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
      /* image: Sel.rel' — union of the selected sources' fibers, membership by stable id;
       * a keyed srel's fibers hold keys, so membership runs against its own key column */
      if (nd->kids[1]->kind == N_SETHOP) {
        char *src; if (emitMask(em, nd->kids[0], &src)) return -1;
        char *fib; if (fiberVar(em, nd->kids[1]->kids[0], &fib)) return -1;
        const RegEntry *se = find(em, nd->kids[1]->kids[0]->name);
        char *ids = NULL;
        if (se && se->kind == RK_SREL && se->keyOf[0]) {
          const RegEntry *kc = find(em, se->keyOf);
          if (kc) ids = bqnv(em, kc);
        }
        char *mcol = ids ? ids : idCol(em);
        /* --trace: a dead member is a dead link the image crosses — same RELATION line as
         * fiberRows, origin the fiber's row, sink the member that resolves nowhere. Only
         * the selected sources' fibers are crossed (AnoImage unions m/f), and only stored
         * srel fibers can hold dead members (key-column fibers compute from the live world). */
        if (em->dirs->trace && se && se->kind == RK_SREL && se->nfib == em->reg->n)
          stage(em, "(%s/%s) {m←(≠%s)≤%s⊐𝕩 ⋄ AnoTraceDead ⟨\"%s\", (+´m)⥊𝕨, m/𝕩⟩}¨ (%s/%s)",
                src, traceIds(em, fib), mcol, mcol, se->name, src, fib);
        *out = efmt(em, "(%s‿%s AnoImage %s)", src, fib, mcol);
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

static int emitEffect(Em *em, const Node *ef, Fx *fx) {
  switch (ef->kind) {
    case N_EASSIGN: {
      const Node *tgt = ef->kids[0];
      const char *field = NULL;
      const Node *coln = tgt;
      if (tgt->kind == N_HOP) { coln = tgt->kids[0]; field = tgt->kids[1]->name; }
      const RegEntry *e = find(em, coln->name);
      if (!e) return fail(em, ef->line, "assign to unregistered '%s'", coln->name);
      /* Derived tags are computed from their carrier and cannot be assigned. */
      if (e->kind == RK_TAG)
        return fail(em, ef->line, "derived tag '%s' is not an effect target: write the carrier column '%s'",
                    coln->name, e->tagCol);
      /* Unique columns are minted at spawn and cannot be assigned. */
      if (e->uniq)
        return fail(em, ef->line, "unique column '%s' is minted, never written", coln->name);
      char *col = bqnv(em, e);
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
      /* Restore row order before scattering a scan-along result. */
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
      if (e && e->kind == RK_TAG)
        return fail(em, ef->line, "derived tag '%s' is not an effect target: write the carrier column '%s'",
                    ef->name, e->tagCol);
      if (!e || (e->kind != RK_COL && e->kind != RK_FIELD))
        return fail(em, ef->line, "%cComp on unregistered '%s'", ef->kind == N_EADD ? '+' : '-', ef->name);
      if (e->uniq)
        return fail(em, ef->line, "unique column '%s' is minted, never written", ef->name);
      char *col = bqnv(em, e);
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
      /* spawn writes its proto column; a derived tag takes no writes anywhere */
      if (what->kind == N_NAME) {
        const RegEntry *we = find(em, what->name);
        if (we && we->kind == RK_TAG)
          return fail(em, ef->line, "derived tag '%s' is not an effect target: write the carrier column '%s'",
                      what->name, we->tagCol);
      }
      /* On a lattice, spawning a registered field ORs the selection into it; no rows mint. */
      if ((em->fr.kind == FR_LAT || em->fr.kind == FR_BOARD) && !cnt &&
          what->kind == N_NAME) {
        const RegEntry *fe = find(em, what->name);
        if (fe && fe->kind == RK_FIELD) {
          char *base = mergeBase(em, fx, entIdx(em, fe), bqnv(em, fe), '|', NULL,
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
      if (tc->uniq)
        return fail(em, ef->line, "unique column '%s' is minted, never written", target);
      char *args = efmt(em, "⟨%s", bqnv(em, tc));
      for (int i = 0; i < ef->nkids; i++) {
        EV av; if (emitVal(em, ef->kids[i], MODE_WORLD, &av)) return -1;
        args = efmt(em, "%s, %s", args, av.v);
      }
      args = efmt(em, "%s⟩", args);
      if (!mergeBase(em, fx, entIdx(em, tc), bqnv(em, tc), 'v', NULL, ef->line, ef->name))
        return -1;
      char *t = tv(em);
      stage(em, "%s ← %s %s %s", t, em->selVar, fnv(em, e), args);
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
  if (e == reg_role(em->reg, "parent")) return efmt(em, "(%s//%s)", g->cnt, em->selVar);
  return efmt(em, "(%s⥊%s)", tot, numLit(em, e->defval));
}

/* the proto a spawn group names, when it names one (RK_PROTO), else NULL */
static const RegEntry *spawnProto(Em *em, SpawnG *sg) {
  if (!sg->protoName) return NULL;
  const RegEntry *pe = find(em, sg->protoName);
  return pe && pe->kind == RK_PROTO ? pe : NULL;
}

/* index of a proto's field for a column name, -1 when the proto is silent on it */
static int protoField(const RegEntry *pe, const char *col) {
  for (int j = 0; j < pe->nsyms / 2; j++)
    if (names_eq(pe->syms[2 * j], col)) return j;
  return -1;
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
  /* --trace: the tick trace captures the pre-state row count here — anoN is not
   * reassigned until the end of this commit — and prints after the anoN update */
  char *preN = NULL;
  if (em->dirs->trace && structural) {
    preN = tv(em);
    stage(em, "%s ← anoN", preN);
  }
  for (int i = 0; i < r->nents; i++) {
    const RegEntry *e = &r->ents[i];
    int isField = e->kind == RK_FIELD;
    if (e->kind != RK_COL && e->kind != RK_REL && e->kind != RK_SREL && !isField) continue;
    /* lattice-sided relations (a stencil srel over w*h cells) are frame-foreign to entity
     * row structure: never filter on despawn, never pad on spawn */
    if (e->kind == RK_SREL && e->nfib != r->n) continue;
    if (e->kind == RK_REL && e->nnums != r->n) continue;
    char *cur = bqnv(em, e);
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
      if (e == reg_role(r, "keys") || (e->kind == RK_COL && e->uniq))
        app = efmt(em, "((1+⌈´¯1∾%s)+↕%s)", cur, totAll);  /* one mint across the batch:
              declared injectivity forces the fresh fill — any shared value would break it */
      else for (int g = 0; g < fx->nsp; g++) {
        SpawnG *sg = &fx->sp[g];
        char *piece;
        int isProto = sg->protoName && find(em, sg->protoName) == e;
        anyProto |= isProto;
        /* the three-layer fill (ruled 2026-07-11): proto value, else registered default,
         * else the type zero — the last two live in spawnDefault */
        const RegEntry *pe = spawnProto(em, sg);
        int fi = pe ? protoField(pe, e->name) : -1;
        if (isProto) piece = efmt(em, "(%s⥊1)", sg->tot);
        else if (fi >= 0)
          piece = (e->kind == RK_COL && e->type == CT_SYM)
                    ? efmt(em, "(%s⥊<\"%s\")", sg->tot, pe->syms[2 * fi + 1])
                    : efmt(em, "(%s⥊%s)", sg->tot, numLit(em, pe->nums[fi]));
        else if (pe && e == reg_role(r, "proto") && e->kind == RK_COL && e->type == CT_SYM)
          piece = efmt(em, "(%s⥊<\"%s\")", sg->tot, pe->name);  /* the archetype's noun */
        else if (sg->protoExpr && e == reg_role(r, "proto")) piece = sg->protoExpr;
        else if (sg->pos && e == reg_role(r, "pos")) { piece = sg->pos; if (sg->posPair) em->isPair[i] = 1; }
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
      char *p = presv(em, e);
      char *kept = keep ? efmt(em, "(%s/%s)", keep, p) : p;
      if (fx->nsp) {
        char *papp = NULL;
        for (int g = 0; g < fx->nsp; g++) {
          int isProto = fx->sp[g].protoName && find(em, fx->sp[g].protoName) == e;
          const RegEntry *pe = spawnProto(em, &fx->sp[g]);
          if (pe && protoField(pe, e->name) >= 0) isProto = 1; /* a proto field is present */
          char *piece = efmt(em, "(%s⥊%d)", fx->sp[g].tot, isProto ? 1 : 0);
          papp = papp ? efmt(em, "%s∾%s", papp, piece) : piece;
        }
        stage(em, "%s ↩ %s∾%s", p, kept, papp);
      }
      else stage(em, "%s ↩ %s", p, kept);
    }
    (void)anyProto;
  }
  /* the hidden idx column rides every structural commit like any other column: filtered
   * by keep, minted fresh on spawn — the fixture-row identity the idx-keyed reads invert */
  if (em->needIdx && structural) {
    char *mint = fx->nsp ? efmt(em, "((1+⌈´¯1∾anoIdx)+↕%s)", totAll) : NULL;
    if (keep && mint) stage(em, "anoIdx ↩ (%s/anoIdx)∾%s", keep, mint);
    else if (keep) stage(em, "anoIdx ↩ %s/anoIdx", keep);
    else if (mint) stage(em, "anoIdx ↩ anoIdx∾%s", mint);
  }
  /* spawn always appends to the entity world, whatever frame selected the sources */
  if (structural) {
    if (fx->despawn && fx->nsp) stage(em, "anoN ↩ (+´%s)+%s", keep, totAll);
    else if (fx->despawn) stage(em, "anoN ↩ +´%s", keep);
    else stage(em, "anoN ↩ anoN+%s", totAll);
  }
  /* Emit one structural trace line from the staged row and effect counts. */
  if (preN) {
    char *ann = NULL;
    for (int g = 0; g < fx->nsp; g++) {
      SpawnG *sg = &fx->sp[g];
      char *piece = sg->protoName
        ? efmt(em, "\"spawn %s: +\"∾(AnoTraceNum %s)", sg->protoName, sg->tot)
        : efmt(em, "\"spawn: +\"∾(AnoTraceNum %s)", sg->tot);
      ann = ann ? efmt(em, "%s∾\", \"∾%s", ann, piece) : piece;
    }
    if (fx->despawn) {
      char *piece = efmt(em, "\"kill: -\"∾(AnoTraceNum +´¬%s)", keep);
      ann = ann ? efmt(em, "%s∾\", \"∾%s", ann, piece) : piece;
    }
    stage(em, "•Out anoTraceSep∾\"s%d: \"∾(AnoTraceNum %s)∾\" rows -> \"∾(AnoTraceNum anoN)∾\" (\"∾%s∾\")\"",
          em->stmt, preN, ann);
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
    stage(em, "%s ← %s", sv, bqnv(em, cur));
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
  snprintf(em->traceSel, sizeof em->traceSel, "%s", sv);

  for (int i = 1; i < st->nkids; i++)
    if (emitEffect(em, st->kids[i], &fx)) return -1;

  /* save the antecedent before structural commits (pre-spawn mask, ex49) */
  stage(em, "anoSel ↩ %s", sv);
  em->savedFr = em->fr; em->haveSaved = 1;

  if (commitStmt(em, &fx, isCont)) return -1;
  if (fx.despawn) em->worldShifted = 1;   /* rows shifted: idx-keyed reads now invert anoIdx */

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
    snprintf(em->traceSel, sizeof em->traceSel, "%s", masks[r]);
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
  if (fx.despawn) em->worldShifted = 1;
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
  /* --label: a 0x1D tag line names the query and its source line, then the display —
   * for every query, pinned or not, before any assertion. 0x1D sits one below the
   * 0x1E world channel: run_bqn captures only 0x1E lines, so the tag and the •Show
   * forward verbatim to the caller. Without the flag the emitted bytes are today's. */
  if (em->dirs->label) {
    stage(em, "•Out (@+29)∾\"q%d@%d\"", em->stmt, st->line);
    stage(em, "•Show %s", qv);
  }
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
  } else if (!em->dirs->label) {
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
      stage(em, "%s ↩ %s∧((%s %s⌜ %s)%s%s)", M, M, aI, fnv(em, e), bI, op, rv.v);
    } else if (f->kind == N_CALL) {
      const RegEntry *e = find(em, f->name);
      if (!e) return fail(em, f->line, "unregistered '%s' in comprehension filter", f->name);
      stage(em, "%s ↩ %s∧(%s %s⌜ %s)", M, M, aI, fnv(em, e), bI);
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
  snprintf(em->traceSel, sizeof em->traceSel, "%s", sv);
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
        stage(em, "%s ← %s∨%s", t, bqnv(em, ce), sv);
        addCommit(em, &fx, entIdx(em, ce), t, '|', NULL);
        continue;
      }
    }
    if (emitEffect(em, e2, &fx)) return -1;
  }
  stage(em, "anoSel ↩ %s", sv);
  em->savedFr = em->fr; em->haveSaved = 1;
  if (commitStmt(em, &fx, 0)) return -1;
  if (fx.despawn) em->worldShifted = 1;
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
  /* the hidden idx key: the row iota materialized ONCE, here, then carried through every
   * structural commit — never reminted at use. Emitted only when the pre-scan proved an
   * idx-keyed read can follow a despawn, so every existing emit stays byte-identical. */
  if (em->needIdx) sb_printf(em->out, "anoIdx ← ↕anoN\n");
  for (int i = 0; i < r->nents; i++) {
    const RegEntry *e = &r->ents[i];
    char *v = bqnv(em, e);
    /* mangled entries keep their human spelling as a comment on the definition line */
    const char *cm = bqnlegal(e->name) ? "" : efmt(em, "  # %s", e->name);
    switch (e->kind) {
      case RK_COL: case RK_FIELD: {
        int n = e->kind == RK_FIELD ? r->latW * r->latH : r->n;
        if (e->type == CT_SYM) {
          sb_printf(em->out, "%s ← ⟨", v);
          for (int k = 0; k < e->nsyms; k++) sb_printf(em->out, "%s\"%s\"", k ? ", " : "", e->syms[k]);
          sb_printf(em->out, "⟩%s\n", cm);
        } else if (e->type == CT_CHAR) {
          sb_printf(em->out, "%s ← \"%s\"%s\n", v, e->syms ? e->syms[0] : "", cm);
        } else if (e->nnums == 2 * n && n > 0) {
          em->isPair[i] = 1;
          sb_printf(em->out, "%s ← ⟨", v);
          for (int k = 0; k < n; k++) {
            char a[64], b[64];
            snprintf(a, sizeof a, "%s", numLit(em, e->nums[2*k]));
            snprintf(b, sizeof b, "%s", numLit(em, e->nums[2*k+1]));
            sb_printf(em->out, "%s%s‿%s", k ? ", " : "", a, b);
          }
          sb_printf(em->out, "⟩%s\n", cm);
        } else {
          sb_printf(em->out, "%s ← ⟨", v);
          for (int k = 0; k < e->nnums; k++) sb_printf(em->out, "%s%s", k ? ", " : "", numLit(em, e->nums[k]));
          sb_printf(em->out, "⟩%s\n", cm);
        }
        if (e->hasPres) {
          sb_printf(em->out, "%s ← ⟨", presv(em, e));
          for (int k = 0; k < r->n; k++) sb_printf(em->out, "%s%s", k ? ", " : "", numLit(em, e->pres[k]));
          sb_printf(em->out, "⟩%s\n", cm);
        }
        break;
      }
      case RK_REL: case RK_ALIAS: {
        sb_printf(em->out, "%s ← ⟨", v);
        for (int k = 0; k < e->nnums; k++) sb_printf(em->out, "%s%s", k ? ", " : "", numLit(em, e->nums[k]));
        sb_printf(em->out, "⟩%s\n", cm);
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
        sb_printf(em->out, "⟩%s\n", cm);
        break;
      }
      case RK_BIND: {
        if (!strcmp(e->bindKind, "mask") || !strcmp(e->bindKind, "vec")) {
          sb_printf(em->out, "%s ← ⟨", v);
          for (int k = 0; k < e->nnums; k++) sb_printf(em->out, "%s%s", k ? ", " : "", numLit(em, e->nums[k]));
          sb_printf(em->out, "⟩%s\n", cm);
        } else if (!strcmp(e->bindKind, "num")) {
          sb_printf(em->out, "%s ← %s%s\n", v, numLit(em, e->nums[0]), cm);
        }
        break;
      }
      case RK_FN: {
        if (e->syms && e->syms[0][0]) {
          /* raw form: either "<dfn>" or "<targetcol> <dfn>" (verbs) — bind the dfn part */
          const char *raw = e->syms[0];
          const char *br = strchr(raw, '{');
          if (br && br != raw) sb_printf(em->out, "%s ← %s%s\n", fnv(em, e), br, cm);
          else if (br) sb_printf(em->out, "%s ← %s%s\n", fnv(em, e), raw, cm);
        }
        break;
      }
      default: break;
    }
  }
  /* --trace: the debug observability prelude. Diagnostic lines ride their own control
   * byte 0x1F, one below the 0x1D label channel exactly as 0x1D sits one below the 0x1E
   * world channel — run_bqn captures only 0x1E, so trace lines forward verbatim and can
   * never contaminate the save pipe-back or the label stream. Observability, never
   * semantics: every trace helper is pure output over pre-state reads. */
  if (em->dirs->trace) {
    sb_printf(em->out, "\n# trace (--trace): 0x1F-prefixed diagnostic lines\n");
    sb_printf(em->out, "anoTraceSep ← @+31\n");
    sb_printf(em->out, "AnoTraceNum ← {∾{𝕩='¯' ? \"-\" ; ⋈𝕩}¨•Repr 𝕩}\n");
    sb_printf(em->out, "AnoTraceDead ← {n‿o‿s: o {•Out anoTraceSep∾\"RELATION \"∾n∾\" \"∾(AnoTraceNum 𝕨)∾\" -> \"∾(AnoTraceNum 𝕩)∾\" IS DEAD !\"}¨ s}\n");
    sb_printf(em->out, "AnoTraceEmpty ← {n‿o: {•Out anoTraceSep∾\"FIBER \"∾n∾\" \"∾(AnoTraceNum 𝕩)∾\" IS EMPTY !\"}¨ o}\n");
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
    if (e->kind == RK_TAG) {
      snprintf(em->err, em->errsz, "expect: '%s' is a derived tag; pin the carrier column '%s'",
               ex->col, e->tagCol);
      return -1;
    }
    char *v = bqnv(em, e);
    int sym = (e->kind == RK_COL || e->kind == RK_FIELD) && e->type == CT_SYM;
    int chr = (e->kind == RK_COL || e->kind == RK_FIELD) && e->type == CT_CHAR;
    if (chr) {
      /* char column: the fixture holds a BQN string, so the pin is the glyph run
       * (space-joined when written in parts) compared exactly, never the numeric law.
       * The directive tokenizer collapses whitespace runs, so a glyph string with
       * consecutive or edge spaces is not pinnable this way — the demo glyphs are dot/hash. */
      char *s = efmt(em, "");
      for (int k = 0; k < ex->nvals; k++) s = efmt(em, "%s%s%s", s, k ? " " : "", ex->vals[k]);
      sb_printf(em->out, "\"expect %s\" ! \"%s\" ≡ %s\n", ex->col, s, v);
      continue;
    }
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

/* the --save pipe-back serializer, emitted only under the flag (dirs->save): after the
 * pins have held, print the post-state data — one line per datum, each prefixed with the
 * record-separator byte 0x1E so no user-visible print can collide. Lines: `n <k>`, then
 * per data-carrying entry in declaration order — col/field values (num via •Repr with
 * ¯ swapped to ASCII '-' so strtod round-trips, sym bare words, char the exact glyph
 * run, pairs flattened to 2k numbers), pres bits, rel indexes (-1 the none sentinel),
 * srel fibers as their `|` rows. Schema never pipes: fns, binds, aliases, roles, and
 * derived tags are load-side; inv fibers recompute from their rel at load. */
static void emitSave(Em *em) {
  const Registry *r = em->reg;
  sb_printf(em->out, "\n# save pipe-back (--save): 0x1E-prefixed post-state lines\n");
  sb_printf(em->out, "anoSaveSep ← @+30\n");
  sb_printf(em->out, "AnoSaveNum ← {∾{𝕩='¯' ? \"-\" ; ⋈𝕩}¨•Repr 𝕩}\n");
  sb_printf(em->out, "AnoSaveRow ← {∾{\" \"∾𝕩}¨𝕩}\n");
  sb_printf(em->out, "•Out anoSaveSep∾\"n \"∾AnoSaveNum anoN\n");
  for (int i = 0; i < r->nents; i++) {
    const RegEntry *e = &r->ents[i];
    char *v = bqnv(em, e);
    switch (e->kind) {
      case RK_COL: case RK_FIELD: {
        const char *kw = e->kind == RK_FIELD ? "field" : "col";
        if (e->type == CT_SYM)
          sb_printf(em->out, "•Out anoSaveSep∾\"%s %s\"∾AnoSaveRow %s\n", kw, e->name, v);
        else if (e->type == CT_CHAR)
          sb_printf(em->out, "•Out anoSaveSep∾\"%s %s \"∾%s\n", kw, e->name, v);
        else if (i < (int)(sizeof em->isPair) && em->isPair[i])
          sb_printf(em->out, "•Out anoSaveSep∾\"%s %s\"∾AnoSaveRow AnoSaveNum¨∾%s\n", kw, e->name, v);
        else
          sb_printf(em->out, "•Out anoSaveSep∾\"%s %s\"∾AnoSaveRow AnoSaveNum¨%s\n", kw, e->name, v);
        if (e->hasPres)
          sb_printf(em->out, "•Out anoSaveSep∾\"pres %s\"∾AnoSaveRow AnoSaveNum¨%s\n", e->name, presv(em, e));
        break;
      }
      case RK_REL:
        sb_printf(em->out, "•Out anoSaveSep∾\"rel %s\"∾AnoSaveRow AnoSaveNum¨%s\n", e->name, v);
        break;
      case RK_SREL:
        if (e->invOf[0]) break;
        sb_printf(em->out, "•Out anoSaveSep∾\"srel %s\"∾2↓∾{\" |\"∾AnoSaveRow AnoSaveNum¨𝕩}¨%s\n", e->name, v);
        break;
      default: break;
    }
  }
}

/* ---------- the idx pre-scan ---------- */

/* 1 when the subtree contains a despawn effect */
static int scanDespawn(const Node *nd) {
  if (!nd) return 0;
  if (nd->kind == N_EDESPAWN) return 1;
  for (int i = 0; i < nd->nkids; i++)
    if (nd->kids[i] && scanDespawn(nd->kids[i])) return 1;
  return 0;
}

/* 1 when the subtree reads through the idx key space: an unkeyed functional rel by name,
 * or a set-hop/fiber form whose membership would fall to the minted-at-use row iota
 * because the world declares no id (no role line, no unique column, no magic name).
 * Defs expand; depth caps the expansion. */
static int scanIdxUse(Em *em, const Node *nd, int hasId, int depth) {
  if (!nd || depth > 16) return 0;
  if (nd->kind == N_NAME) {
    const Node *d = findDef(em, nd->name);
    if (d) return scanIdxUse(em, d->kids[0], hasId, depth + 1);
    const RegEntry *e = find(em, nd->name);
    if (e && e->kind == RK_REL && !e->keyOf[0]) return 1;
  }
  if (nd->kind == N_SETHOP && !hasId && nd->kids[0]->kind == N_NAME) {
    const RegEntry *e = find(em, nd->kids[0]->name);
    if (e && ((e->kind == RK_SREL && !e->keyOf[0] && e->nfib == em->reg->n) ||
              e->kind == RK_COL)) return 1;
  }
  for (int i = 0; i < nd->nkids; i++)
    if (nd->kids[i] && scanIdxUse(em, nd->kids[i], hasId, depth)) return 1;
  return 0;
}

/* Inputs: the program and a primed Em. Output: em->needIdx set when an idx-keyed read
 * can follow a despawn — statements strictly after the first despawn-carrying barrier
 * count (a barrier's own reads observe pre-state), and once any despawn exists every
 * installed rule counts too (rules refire at later edges). The verdict arms the hidden
 * anoIdx column; a program that never trips it emits today's bytes exactly. */
static void scanNeedIdx(Em *em, const Node *prog) {
  int anyDespawn = 0;
  for (int i = 0; i < prog->nkids; i++) anyDespawn |= scanDespawn(prog->kids[i]);
  if (!anyDespawn) return;
  /* the same reg_role ladder idCol resolves through: hasId iff idCol names a real column */
  const RegEntry *ide = reg_role(em->reg, "id");
  if (!ide) ide = reg_role(em->reg, "keys");
  if (ide && ide->kind != RK_COL) ide = NULL;
  int hasId = ide != NULL;
  /* defs visible to the whole scan; the real emission re-adds them in order */
  for (int i = 0; i < prog->nkids; i++)
    if (prog->kids[i]->kind == N_DEFSTMT && em->ndefs < 128) em->defs[em->ndefs++] = prog->kids[i];
  int shifted = 0;
  for (int i = 0; i < prog->nkids && !em->needIdx; i++) {
    const Node *st = prog->kids[i];
    const Node *body = st->kind == N_DEFSTMT ? st->kids[0] : st;
    int isRule = body->kind == N_STMT && (body->flags & F_RULE);
    if (isRule) {
      if (scanIdxUse(em, body, hasId, 0)) em->needIdx = 1;
      if (scanDespawn(body)) shifted = 1;  /* its first edge precedes later statements */
    } else {
      if (shifted && scanIdxUse(em, st, hasId, 0)) em->needIdx = 1;
      if (scanDespawn(st)) shifted = 1;
    }
  }
  em->ndefs = 0;
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

  scanNeedIdx(&em, prog);
  emitFixture(&em);

  /* Rules persist after installation. Each run of new installations fires all installed
   * rules once before the next performed statement or EOF, using one shared barrier. */
  const Node *installed[32]; int ninst = 0, fresh = 0;
  int rc = 0;
  for (int i = 0; i < prog->nkids && !rc; i++) {
    const Node *st = prog->kids[i];
    const Node *rule = NULL;
    if (st->kind == N_DEFSTMT) {
      if (em.ndefs >= 128) { rc = fail(&em, st->line, "more than 128 defs"); break; }
      em.defs[em.ndefs++] = st;
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
  if (!rc && dirs->save) emitSave(&em);
  sb_free(&em.pre);
  arena_free(&a);
  return rc;
}
