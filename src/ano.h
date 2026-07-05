/* ano.h — the one contract for anoc: tokens, AST, registry, directives, module APIs.
 * anoc pipeline: main.c (directives, driver) -> registry.c (world fixtures)
 * -> lex.c (ASCII + JA skins) -> parse.c (Pratt, 14 levels) -> emit.c (BQN codegen).
 * The emitted BQN runs under CBQN with src/rt.bqn prepended; assertions carry the verdict.
 */
#ifndef ANO_H
#define ANO_H

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdarg.h>

#define ANO_ERRSZ 512
#define ANO_NAMESZ 256

/* ---------- utils: arena + string buffer (header-only) ---------- */

typedef struct AnoArena { char *base; size_t used, cap; struct AnoArena *next; } Arena;

/* Inputs: arena (may hold a live chain), byte count. Output: zeroed block that lives
 * until arena_free. Invariant: never returns NULL; aborts on OOM. */
static inline void *arena_alloc(Arena *a, size_t n) {
  n = (n + 15) & ~(size_t)15;
  if (!a->base || a->used + n > a->cap) {
    size_t cap = n > (1 << 18) ? n : (1 << 18);
    Arena *old = NULL;
    if (a->base) { old = (Arena *)malloc(sizeof *old); if (!old) abort(); *old = *a; }
    a->base = (char *)calloc(1, cap); if (!a->base) abort();
    a->cap = cap; a->used = 0; a->next = old;
  }
  void *p = a->base + a->used; a->used += n; return p;
}
static inline char *arena_strdup(Arena *a, const char *s, size_t n) {
  char *p = (char *)arena_alloc(a, n + 1); memcpy(p, s, n); p[n] = 0; return p;
}
static inline void arena_free(Arena *a) {
  free(a->base);
  for (Arena *c = a->next; c;) { Arena *nx = c->next; free(c->base); free(c); c = nx; }
  a->base = NULL; a->next = NULL; a->used = a->cap = 0;
}

typedef struct { char *s; size_t len, cap; } StrBuf;

/* Inputs: buffer, printf format. Output: appended, NUL-kept. Invariant: s always valid. */
static inline void sb_printf(StrBuf *b, const char *fmt, ...) {
  va_list ap; va_start(ap, fmt);
  va_list ap2; va_copy(ap2, ap);
  int need = vsnprintf(NULL, 0, fmt, ap2); va_end(ap2);
  if (need < 0) { va_end(ap); return; }
  if (b->len + (size_t)need + 1 > b->cap) {
    size_t cap = b->cap ? b->cap : 256;
    while (cap < b->len + (size_t)need + 1) cap *= 2;
    b->s = (char *)realloc(b->s, cap); if (!b->s) abort();
    b->cap = cap;
  }
  vsnprintf(b->s + b->len, b->cap - b->len, fmt, ap);
  b->len += (size_t)need;
  va_end(ap);
}
static inline void sb_free(StrBuf *b) { free(b->s); b->s = NULL; b->len = b->cap = 0; }

/* ---------- tokens ---------- */

typedef enum {
  T_EOF = 0, T_NL,
  /* atoms */
  T_NAME, T_ALIAS, T_SYM, T_NUM, T_COUNTER, T_STR, T_WILD,
  /* structure */
  T_COMMA, T_ARROW /* => */, T_SEMI, T_PIPEGT /* |> */,
  T_AMP, T_BAR, T_BANG,
  T_EQEQ, T_NE, T_LT, T_LE, T_GT, T_GE, T_EQ,
  T_PLUSEQ, T_MINUSEQ, T_STAREQ, T_SLASHEQ,
  T_PLUS, T_MINUS, T_STAR, T_SLASH, T_PCT,
  T_AT, T_DOT, T_TICK, T_LP, T_RP, T_LB, T_RB, T_LARROW /* <- */, T_TILDE,
  T_FOLD /* +/ */, T_SCANOP /* +\ */, T_IOTA /* ↕ */,
  /* keywords (closed set; GRAMMAR.md) */
  T_DEF, T_SPAWN, T_ATKW /* at */, T_TO, T_VIA, T_ALONG,
  T_ORDER, T_BY, T_TAKE, T_DESC, T_TOP, T_GRADE,
  T_REDUCE, T_SCANKW /* scan */, T_SCAN2, T_CROSS, T_EXPAND,
  T_KINDCOUNT
} TokKind;

typedef struct {
  TokKind kind;
  char name[ANO_NAMESZ];  /* NAME/ALIAS/SYM/STR text; FOLD/SCANOP op spelling ("+","*","&","|","#","max","min","avg"); COUNTER unit */
  double num;             /* NUM/COUNTER value */
  int line;
} Tok;

/* ---------- registry ---------- */

typedef enum { RK_COL, RK_REL, RK_SREL, RK_ALIAS, RK_BIND, RK_FN, RK_FIELD } RegKind;
typedef enum { CT_NUM, CT_BOOL, CT_SYM, CT_CHAR } ColType;

typedef struct {
  RegKind kind;
  char name[ANO_NAMESZ];
  ColType type;             /* RK_COL / RK_FIELD */
  double *nums; int nnums;  /* numeric/bool/rel data; rel: -1 = dangling */
  char (*syms)[ANO_NAMESZ]; int nsyms;   /* CT_SYM data */
  /* RK_SREL: fibers flattened; fiber i = fibVals[fibOff[i] .. fibOff[i]+fibLen[i]) */
  int *fibOff, *fibLen; double *fibVals; int nfib;
  char invOf[ANO_NAMESZ];   /* RK_SREL as the inverse read of a functional rel */
  char bindKind[16];        /* RK_BIND: "entity" | "mask" | "point" | "num" | "vec" */
  double defval;            /* spawn default for columns (0 unless `default` line) */
  double *pres; int hasPres;/* optional presence mask (RK_COL) */
} RegEntry;

typedef struct {
  int n;                    /* entity-table row count (0 if pure-space world) */
  int latW, latH;           /* lattice shape; 0 0 when absent */
  RegEntry *ents; int nents;
  char (*jaFrom)[ANO_NAMESZ]; char (*jaTo)[ANO_NAMESZ]; int nja; /* JA alias -> registry name */
} Registry;

/* registry.c
 * File format (NAME.reg beside a demo), line-based, space-separated, `#` comments:
 *   n 6
 *   col gold num 100 200 300 400 500 600
 *   col faction sym Bandit Player Bandit Nord Nord Nord
 *   col name bool 1 0 1 ...            # bool columns are masks
 *   pres twoHanded 1 1 0 1 1 1         # presence; absent rows carry junk values
 *   default gold 0                     # spawn default
 *   rel mentor -1 0 3 -1 2 2           # functional rel, -1 dangling
 *   srel targets 3 4 | 4 5 | | | | | | # set-valued; n fibers, `|`-separated
 *   inv livestock pen                  # set-valued as inverse read of `pen`
 *   alias cursor 0 0 1 0 0 0
 *   bind Player entity 2
 *   bind Whiterun mask 1 1 0 0 1 1
 *   bind rally point 10 20
 *   bind spacing num 4
 *   fn fib
 *   lattice 8 8
 *   field elevation num 0 1 2 ...      # w*h values, row-major
 *   ja 北 nord
 * Inputs: path, out registry, err buffer. Output: 0 ok / -1 with err set. */
int reg_load(const char *path, Registry *reg, Arena *a, char *err, size_t errsz);
const RegEntry *reg_find(const Registry *reg, const char *name);      /* case-insensitive on first letter */
const char *reg_ja(const Registry *reg, const char *jaWord);          /* NULL when unmapped */

/* ---------- AST ---------- */

typedef enum {
  /* atoms */
  N_NUM, N_COUNTER, N_SYM, N_STR, N_NAME, N_ALIAS, N_WILD,
  /* expressions */
  N_NOT, N_AND, N_OR,
  N_CMP,    /* op: '<' '>' 'l'(<=) 'g'(>=) '=' '!'(!=) ; kids: l, r */
  N_ARITH,  /* op: '+' '-' '*' '/' '%' ; kids: l, r */
  N_SCOPE,  /* l @ r */
  N_HOP,    /* l . r  (r: N_NAME field/comp, or nested N_HOP chain) */
  N_SETHOP, /* name' ; kids[0] = N_NAME rel; kids[1] = optional gathered comp (rel'.Comp) */
  N_CALL,   /* name(args...) ; name in tok.name, kids = args */
  N_FOLD,   /* op in name ("+","*","&","|","#","max","min","avg", or reducer name); kids[0]=operand; kids[1]=optional @scope */
  N_SCANEXPR, /* scan: same layout as N_FOLD; flag F_SCAN2 for scan2 */
  N_SCANALONG,/* scan(f) col along order ; kids: col, order; op in name */
  N_REDUCE, /* reduce(f) col @ scope ; kids: col, scope */
  N_IOTAX,  /* ↕ expr */
  N_SHAPE,  /* numeric shape source: kids = dims (N_NUM or N_WILD) */
  N_TUPLE,  /* (a, b, ...) presence tuple or point literal; context decides */
  N_TO,     /* to shape ; kids[0]=N_SHAPE ; as source: kids[1]=poured expr (board literal) */
  N_GRADE,  /* grade [desc] key [@ scope] ; kids: key, optional scope ; flag F_DESC */
  N_TOP,    /* top k inner ; num=k ; kids[0]=inner */
  N_PIPE,   /* src |> stage |> stage ; kids[0]=src, kids[1..]=stages */
  N_ORDERBY,/* order by key [desc] ; kids[0]=key ; flag F_DESC */
  N_TAKE,   /* take k ; num=k */
  N_EXPAND, /* expand countcol ; kids[0]=col */
  N_CROSSV, /* cross f A B ; name=f ; kids: A, B */
  N_BINDER, /* a <- Source ; name=a ; kids[0]=source */
  /* effects */
  N_EASSIGN,/* target op expr ; op '=' '+' '-' '*' '/' ; kids[0]=target (N_NAME or N_HOP pos.x), kids[1]=rhs */
  N_EADD,   /* +Comp ; name */
  N_EDEL,   /* -Comp ; name */
  N_EDESPAWN, /* ~ */
  N_ESPAWN, /* spawn what [* count] [at pos] ; kids[0]=what (N_NAME or N_CALL), kids[1]=count?, kids[2]=at? (NULL allowed) */
  N_EVERB,  /* registered effect verb: name + arg kids */
  N_EVIA,   /* fn via Col ; name=fn ; kids[0]=col */
  /* statements */
  N_STMT,   /* kids[0]=selection (NULL: continuation/elided), kids[1..]=effects ; flags F_RULE F_CONT F_ELIDED */
  N_DEFSTMT,/* def name = body ; kids[0]=body ; body may be N_STMT with F_RULE for standing rules */
  N_QUERY,  /* bare expression statement ; kids[0]=expr */
  N_COMPR,  /* [ sel , effect | binders, filters ] ; kids: sel, effect, then N_BINDER and filter exprs */
  N_PROGRAM
} NodeKind;

enum { F_DESC = 1, F_RULE = 2, F_CONT = 4, F_ELIDED = 8, F_SCAN2 = 16 };

typedef struct Node {
  NodeKind kind;
  char op;
  int flags;
  double num;
  char name[ANO_NAMESZ];
  struct Node **kids; int nkids;
  int line;
} Node;

Node *node_new(Arena *a, NodeKind k, int line);
void node_addkid(Arena *a, Node *n, Node *kid);

/* ---------- directives (main.c owns parsing them) ---------- */

typedef struct {
  char col[ANO_NAMESZ];
  char **vals; int nvals;   /* raw value words; symbols stay words, numbers stay spellings */
  int isOut;                /* --! out : ordered query-output expectation (col unused) */
} Expect;

typedef struct {
  char registry[256];       /* registry name or path; "" = none */
  Expect *expects; int nexpects;
  int expectN;              /* --! expect-n ; -1 = unchecked */
  int ja;                   /* --! ja */
  char sameTokens[512];     /* --! same-tokens <ascii line> : lex this line ASCII and
                               assert kind/name/num equality with the file's own stream (ex40) */
} Directives;

/* ---------- module APIs ---------- */

/* lex.c — Inputs: full source (directives already stripped to blank by main), ja flag,
 * registry (JA aliases), arena. Output: token array ending in T_EOF, T_NL between lines;
 * JA mode: space-separated words mapped (registry ja, particle table, kanji numerals),
 * then normalized: TGT dropped, each postfix operator swapped before its operand (ex40).
 * Returns 0 / -1 with err set. */
int ano_lex(const char *src, int ja, const Registry *reg, Arena *a,
            Tok **toks, int *ntoks, char *err, size_t errsz);

/* parse.c — Inputs: token stream. Output: N_PROGRAM whose kids are statements in order.
 * Implements GRAMMAR.md: the hinge split, precedence 1-14, selection/effect forms,
 * comprehensions, defs, continuations, queries. Returns NULL with err set on failure. */
Node *ano_parse(const Tok *toks, int ntoks, Arena *a, char *err, size_t errsz);

/* emit.c — Inputs: program, registry, directives. Output: complete BQN program appended
 * to out (rt.bqn is prepended by the driver). Fixture bindings from the registry, one
 * gather-effect-scatter block per statement, expectations as `!` assertions.
 * Returns 0 / -1 with err set. */
int ano_emit(const Node *prog, const Registry *reg, const Directives *dirs,
             StrBuf *out, char *err, size_t errsz);

#endif
