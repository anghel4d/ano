/* main.c — the anoc driver: directive extraction, registry resolution, and the
 * lex -> parse -> emit pipeline; --tokens dumps the stream, --emit prints rt.bqn +
 * program, --run executes it under bqn. Exit: 0 ok, 1 test failure, 2 usage/compile. */
#define _GNU_SOURCE
#include "ano.h"
#include <errno.h>
#include <unistd.h>
#include <sys/wait.h>

#define MAXEXPECT 64
#define MAXVALS 8192

/* TokKind -> stable enum spelling for --tokens and mismatch dumps. */
static const char *tokname[T_KINDCOUNT] = {
  [T_EOF]="T_EOF", [T_NL]="T_NL",
  [T_NAME]="T_NAME", [T_ALIAS]="T_ALIAS", [T_SYM]="T_SYM", [T_NUM]="T_NUM",
  [T_COUNTER]="T_COUNTER", [T_STR]="T_STR", [T_WILD]="T_WILD",
  [T_COMMA]="T_COMMA", [T_ARROW]="T_ARROW", [T_SEMI]="T_SEMI", [T_PIPEGT]="T_PIPEGT",
  [T_AMP]="T_AMP", [T_BAR]="T_BAR", [T_BANG]="T_BANG",
  [T_EQEQ]="T_EQEQ", [T_NE]="T_NE", [T_LT]="T_LT", [T_LE]="T_LE",
  [T_GT]="T_GT", [T_GE]="T_GE", [T_EQ]="T_EQ",
  [T_PLUSEQ]="T_PLUSEQ", [T_MINUSEQ]="T_MINUSEQ", [T_STAREQ]="T_STAREQ", [T_SLASHEQ]="T_SLASHEQ",
  [T_PLUS]="T_PLUS", [T_MINUS]="T_MINUS", [T_STAR]="T_STAR", [T_SLASH]="T_SLASH", [T_PCT]="T_PCT",
  [T_AT]="T_AT", [T_DOT]="T_DOT", [T_TICK]="T_TICK", [T_LP]="T_LP", [T_RP]="T_RP",
  [T_LB]="T_LB", [T_RB]="T_RB", [T_LARROW]="T_LARROW", [T_TILDE]="T_TILDE",
  [T_FOLD]="T_FOLD", [T_SCANOP]="T_SCANOP", [T_IOTA]="T_IOTA",
  [T_DEF]="T_DEF", [T_SPAWN]="T_SPAWN", [T_ATKW]="T_ATKW", [T_TO]="T_TO",
  [T_VIA]="T_VIA", [T_ALONG]="T_ALONG",
  [T_ORDER]="T_ORDER", [T_BY]="T_BY", [T_TAKE]="T_TAKE", [T_DESC]="T_DESC",
  [T_TOP]="T_TOP", [T_GRADE]="T_GRADE",
  [T_REDUCE]="T_REDUCE", [T_SCANKW]="T_SCANKW", [T_SCAN2]="T_SCAN2",
  [T_CROSS]="T_CROSS", [T_EXPAND]="T_EXPAND",
};

/* Inputs: none. Output: usage on stderr. Returns 2 (usage error exit code). */
static int usage(void) {
  fprintf(stderr, "usage: anoc [--tokens] [--emit] [--run] [--dump <path>] [--save <path>] [--rt <path>] [--registry <path-or-name>] file.ano\n");
  return 2;
}

/* Inputs: buffer, bytes + length. Output: appended verbatim, NUL-kept — the raw sibling
 * of sb_printf for pipe chunks that may hold '%' or partial UTF-8. */
static void sb_put(StrBuf *b, const char *s, size_t n) {
  if (b->len + n + 1 > b->cap) {
    size_t cap = b->cap ? b->cap : 256;
    while (cap < b->len + n + 1) cap *= 2;
    b->s = (char *)realloc(b->s, cap); if (!b->s) abort();
    b->cap = cap;
  }
  memcpy(b->s + b->len, s, n);
  b->len += n;
  b->s[b->len] = 0;
}

/* Inputs: cursor into a NUL-terminated line. Output: next space/tab-separated word,
 * NUL-terminated in place, or NULL at end; cursor advances past it. */
static char *word(char **pp) {
  char *p = *pp;
  while (*p == ' ' || *p == '\t') p++;
  if (!*p) { *pp = p; return NULL; }
  char *w = p;
  while (*p && *p != ' ' && *p != '\t') p++;
  if (*p) *p++ = 0;
  *pp = p;
  return w;
}

/* Inputs: one directive line after "--!" (trimmed, mutable), dirs, arena.
 * Output: 0 / -1 with err set. Grammar: registry N | expect col = v... |
 * expect-n K | out v... | ja | same-tokens <rest verbatim>. Values stay raw words. */
static int dir_line(char *s, Directives *dirs, Arena *a, char *err, size_t errsz) {
  char *p = s;
  char *key = word(&p);
  if (!key) return 0;                                        /* bare --! */
  if (!strcmp(key, "registry")) {
    char *v = word(&p);
    if (!v) { snprintf(err, errsz, "directive: registry needs a name"); return -1; }
    snprintf(dirs->registry, sizeof dirs->registry, "%s", v);
  } else if (!strcmp(key, "ja")) {
    dirs->ja = 1;
  } else if (!strcmp(key, "expect-n")) {
    char *v = word(&p);
    if (!v) { snprintf(err, errsz, "directive: expect-n needs a count"); return -1; }
    dirs->expectN = atoi(v);
  } else if (!strcmp(key, "same-tokens")) {
    while (*p == ' ' || *p == '\t') p++;
    snprintf(dirs->sameTokens, sizeof dirs->sameTokens, "%s", p);
  } else if (!strcmp(key, "expect") || !strcmp(key, "out")) {
    if (dirs->nexpects >= MAXEXPECT) { snprintf(err, errsz, "directive: too many expects"); return -1; }
    Expect *e = &dirs->expects[dirs->nexpects++];           /* arena block: already zeroed */
    char *w;
    if (!strcmp(key, "out")) { e->isOut = 1; w = word(&p); }
    else {
      char *col = word(&p);
      if (!col) { snprintf(err, errsz, "directive: expect needs a column"); return -1; }
      snprintf(e->col, sizeof e->col, "%s", col);
      w = word(&p);
      if (w && !strcmp(w, "=")) w = word(&p);               /* optional '=' */
    }
    char **tmp = (char **)arena_alloc(a, MAXVALS * sizeof *tmp); int n = 0;
    while (w) { if (n < MAXVALS) tmp[n++] = w; w = word(&p); }
    e->vals = (char **)arena_alloc(a, (size_t)(n ? n : 1) * sizeof *e->vals);
    for (int i = 0; i < n; i++) e->vals[i] = arena_strdup(a, tmp[i], strlen(tmp[i]));
    e->nvals = n;
  } else {
    snprintf(err, errsz, "directive: unknown key '%s'", key);
    return -1;
  }
  return 0;
}

/* Inputs: mutable source text, dirs (zeroed, expects allocated, expectN preset -1), arena.
 * Output: 0 / -1 with err set. Directive lines are blanked to spaces in src; newlines
 * survive so lexer line numbers match the file. */
static int parse_directives(char *src, Directives *dirs, Arena *a, char *err, size_t errsz) {
  char *line = src;
  while (*line) {
    char *end = strchr(line, '\n');
    size_t len = end ? (size_t)(end - line) : strlen(line);
    char *p = line;
    while (*p == ' ' || *p == '\t') p++;
    if ((size_t)(p - line) + 3 <= len && !strncmp(p, "--!", 3)) {
      size_t blen = len - (size_t)(p - line) - 3;
      char *buf = (char *)arena_alloc(a, blen + 1);
      memcpy(buf, p + 3, blen); buf[blen] = 0;
      while (blen && (buf[blen-1] == '\r' || buf[blen-1] == ' ' || buf[blen-1] == '\t')) buf[--blen] = 0;
      if (dir_line(buf, dirs, a, err, errsz)) return -1;
      memset(line, ' ', len);
    }
    if (!end) break;
    line = end + 1;
  }
  return 0;
}

/* Inputs: stream, token columns. Output: one 'KIND name num' line per token. */
static void print_toks(FILE *f, const Toks *t) {
  for (int i = 0; i < t->n; i++)
    fprintf(f, "%s %s %g\n", tokname[t->kind[i]] ? tokname[t->kind[i]] : "?", t->name[i], t->num[i]);
}

/* Inputs: two token streams, the registry. Output: 1 when kind/name/num sequences match
 * ignoring T_NL/T_EOF, else 0. Names carry surface spellings, so two spellings are equal
 * when they resolve to one registry entry (北 vs Nord); otherwise exact bytes.
 * Invariant: nums compare exactly — both streams come from ano_lex. */
static int same_stream(const Toks *x, const Toks *y, const Registry *reg) {
  int i = 0, j = 0;
  for (;;) {
    while (i < x->n && (x->kind[i] == T_NL || x->kind[i] == T_EOF)) i++;
    while (j < y->n && (y->kind[j] == T_NL || y->kind[j] == T_EOF)) j++;
    if (i >= x->n || j >= y->n) return i >= x->n && j >= y->n;
    if (x->kind[i] != y->kind[j] || x->num[i] != y->num[j]) return 0;
    if (strcmp(x->name[i], y->name[j])) {
      /* only NAME/ALIAS payloads compare through the resolver — sym, string, and
       * counter payloads are values, and values never fold (the case contract) */
      if (x->kind[i] != T_NAME && x->kind[i] != T_ALIAS) return 0;
      const RegEntry *ex = reg_find(reg, x->name[i]), *ey = reg_find(reg, y->name[j]);
      if (!ex || ex != ey) return 0;
    }
    i++; j++;
  }
}

/* ---------- the --save pipe-back: sentinel lines patch the loaded Registry ---------- */

/* Inputs: word. Output: 0 with *out set via strtod, -1 on empty/trailing junk. */
static int save_num(const char *w, double *out) {
  char *end;
  *out = strtod(w, &end);
  return (end != w && *end == 0) ? 0 : -1;
}

/* Inputs: registry, name. Output: the mutable entry under names_eq, else NULL — the
 * pipe-back patches entries in place, which reg_find's const view cannot serve. */
static RegEntry *save_ent(Registry *reg, const char *name) {
  for (int i = 0; i < reg->nents; i++)
    if (names_eq(reg->ents[i].name, name)) return &reg->ents[i];
  return NULL;
}

/* Inputs: arena, entry, NUL-terminated string of any length. Output: none; stores the
 * string at syms[0] across contiguous ANO_NAMESZ slots, nsyms = 1 (the loader's own
 * char/fn convention, registry.c store_string). */
static void save_str(Arena *a, RegEntry *e, const char *s) {
  size_t len = strlen(s);
  size_t slots = (len + ANO_NAMESZ) / ANO_NAMESZ;
  e->syms = (char (*)[ANO_NAMESZ])arena_alloc(a, slots * ANO_NAMESZ);
  memcpy(e->syms[0], s, len + 1);
  e->nsyms = 1;
}

/* Inputs: registry, arena, one 0x1E-stripped post-state line (mutable). Output: 0 / -1
 * with err set. Grammar (emit.c emitSave): n <k> | col <name> <values…> | field … |
 * pres <name> <bits> | rel <name> <indexes> | srel <name> <fibers |-separated>. Arrays
 * are always allocated fresh — n may grow on spawn — and `n` arrives first, so every
 * later count checks against the post-state row count. */
static int save_line(Registry *reg, Arena *a, char *line, char *err, size_t errsz) {
  char *p = line;
  char *k = word(&p);
  if (!k) return 0;
  double d;
  if (!strcmp(k, "n")) {
    char *v = word(&p);
    if (!v || save_num(v, &d) || d < 0) { snprintf(err, errsz, "save: bad n line"); return -1; }
    reg->n = (int)d;
    return 0;
  }
  int isCol = !strcmp(k, "col"), isField = !strcmp(k, "field");
  if (!isCol && !isField && strcmp(k, "pres") && strcmp(k, "rel") && strcmp(k, "srel")) {
    snprintf(err, errsz, "save: unknown line kind '%s'", k);
    return -1;
  }
  char *name = word(&p);
  if (!name) { snprintf(err, errsz, "save: %s line without a name", k); return -1; }
  RegEntry *e = save_ent(reg, name);
  if (!e) { snprintf(err, errsz, "save: unknown entry '%s'", name); return -1; }
  int rows = isField ? reg->latW * reg->latH : reg->n;
  /* char payload is the raw tail — word() consumed exactly one separator, so p is the
   * glyph run verbatim, spaces included */
  if ((isCol || isField) && e->type == CT_CHAR && e->kind == (isField ? RK_FIELD : RK_COL)) {
    if ((int)strlen(p) != rows) {
      snprintf(err, errsz, "save: %s %s: %zu glyphs for %d cells", k, name, strlen(p), rows);
      return -1;
    }
    /* the run must survive the reader it is written for: strip_line trims boundary
     * whitespace and takes a word-boundary '#' as a comment, and an empty tail is no
     * run at all — a post-state that spells any of these has no .reg spelling
     * (ISSUES.md, the text-format seam) and must not commit */
    int bad = rows == 0 ||
              p[0] == ' ' || p[0] == '\t' || p[0] == '#' ||
              p[rows - 1] == ' ' || p[rows - 1] == '\t';
    for (int j = 1; j < rows && !bad; j++)
      bad = p[j] == '#' && (p[j - 1] == ' ' || p[j - 1] == '\t');
    if (bad) {
      snprintf(err, errsz, "save: %s %s: the post-state glyph run has no .reg spelling "
               "(empty, boundary whitespace, or a word-boundary '#')", k, name);
      return -1;
    }
    save_str(a, e, p);
    return 0;
  }
  /* everything else splits into words */
  int maxw = (int)(strlen(p) / 2 + 2);
  char **words = (char **)arena_alloc(a, (size_t)maxw * sizeof *words);
  int nw = 0;
  char *w;
  while ((w = word(&p)) && nw < maxw) words[nw++] = w;
  if (isCol || isField) {
    if (e->kind != (isField ? RK_FIELD : RK_COL)) {
      snprintf(err, errsz, "save: '%s' is not a %s", name, k);
      return -1;
    }
    if (e->type == CT_SYM) {
      if (nw != rows) {
        snprintf(err, errsz, "save: %s %s: %d values for %d rows (an empty sym value has no .reg spelling)",
                 k, name, nw, rows);
        return -1;
      }
      e->syms = (char (*)[ANO_NAMESZ])arena_alloc(a, (size_t)(rows ? rows : 1) * ANO_NAMESZ);
      for (int j = 0; j < rows; j++) {
        size_t l = strlen(words[j]);
        if (l >= ANO_NAMESZ) { snprintf(err, errsz, "save: %s %s: sym value too long", k, name); return -1; }
        memcpy(e->syms[j], words[j], l + 1);
      }
      e->nsyms = rows;
      return 0;
    }
    /* num/bool scalar (rows values) or pair column (2*rows values, the vec convention) */
    if (nw != rows && nw != 2 * rows) {
      snprintf(err, errsz, "save: %s %s: %d values for %d rows", k, name, nw, rows);
      return -1;
    }
    double *v = (double *)arena_alloc(a, (size_t)(nw ? nw : 1) * sizeof *v);
    for (int j = 0; j < nw; j++)
      if (save_num(words[j], &v[j])) { snprintf(err, errsz, "save: %s %s: bad number '%s'", k, name, words[j]); return -1; }
    e->nums = v;
    e->nnums = nw;
    return 0;
  }
  if (!strcmp(k, "pres")) {
    if (e->kind != RK_COL) { snprintf(err, errsz, "save: pres on non-column '%s'", name); return -1; }
    if (nw != reg->n) { snprintf(err, errsz, "save: pres %s: %d bits for %d rows", name, nw, reg->n); return -1; }
    double *v = (double *)arena_alloc(a, (size_t)(nw ? nw : 1) * sizeof *v);
    for (int j = 0; j < nw; j++)
      if (save_num(words[j], &v[j])) { snprintf(err, errsz, "save: pres %s: bad bit '%s'", name, words[j]); return -1; }
    e->pres = v;
    e->hasPres = 1;
    return 0;
  }
  if (!strcmp(k, "rel")) {
    if (e->kind != RK_REL) { snprintf(err, errsz, "save: '%s' is not a rel", name); return -1; }
    if (nw != reg->n) { snprintf(err, errsz, "save: rel %s: %d values for %d rows", name, nw, reg->n); return -1; }
    double *v = (double *)arena_alloc(a, (size_t)(nw ? nw : 1) * sizeof *v);
    for (int j = 0; j < nw; j++)
      if (save_num(words[j], &v[j])) { snprintf(err, errsz, "save: rel %s: bad index '%s'", name, words[j]); return -1; }
    e->nums = v;
    e->nnums = nw;
    return 0;
  }
  /* srel: fibers |-separated, empty fibers legal (the loader's own shape) */
  if (e->kind != RK_SREL) { snprintf(err, errsz, "save: '%s' is not an srel", name); return -1; }
  int nfib = 1, nvals = 0;
  for (int j = 0; j < nw; j++) strcmp(words[j], "|") == 0 ? nfib++ : nvals++;
  e->fibOff = (int *)arena_alloc(a, (size_t)nfib * sizeof *e->fibOff);
  e->fibLen = (int *)arena_alloc(a, (size_t)nfib * sizeof *e->fibLen);
  e->fibVals = (double *)arena_alloc(a, (size_t)(nvals ? nvals : 1) * sizeof *e->fibVals);
  int fib = 0, vi = 0;
  e->fibOff[0] = 0;
  for (int j = 0; j < nw; j++) {
    if (strcmp(words[j], "|") == 0) {
      e->fibLen[fib] = vi - e->fibOff[fib];
      fib++;
      e->fibOff[fib] = vi;
    } else if (save_num(words[j], &e->fibVals[vi++])) {
      snprintf(err, errsz, "save: srel %s: bad id '%s'", name, words[j]);
      return -1;
    }
  }
  e->fibLen[fib] = vi - e->fibOff[fib];
  e->nfib = fib + 1;
  e->invOf[0] = 0;
  return 0;
}

/* Inputs: registry, arena, captured sentinel lines (mutable, \n-separated). Output: 0 /
 * -1 with err set. After the per-line patches, stored masks (alias entries, mask binds)
 * reconcile to the new n — zero-padded on growth, truncated on shrink — so the dumped
 * world always reloads; the semantic staleness of a stored mask across structural
 * change is the world model's own (the BQN world does not restructure them either). */
static int save_patch(Registry *reg, Arena *a, StrBuf *cap, char *err, size_t errsz) {
  char *s = cap->s ? cap->s : (char *)"";
  while (*s) {
    char *nl = strchr(s, '\n');
    if (nl) *nl = 0;
    if (save_line(reg, a, s, err, errsz)) return -1;
    if (!nl) break;
    s = nl + 1;
  }
  for (int i = 0; i < reg->nents; i++) {
    RegEntry *e = &reg->ents[i];
    int isMask = e->kind == RK_ALIAS ||
                 (e->kind == RK_BIND && !strcmp(e->bindKind, "mask"));
    if (!isMask || e->nnums == reg->n) continue;
    double *v = (double *)arena_alloc(a, (size_t)(reg->n ? reg->n : 1) * sizeof *v);
    for (int j = 0; j < reg->n; j++) v[j] = j < e->nnums ? e->nums[j] : 0;
    e->nums = v;
    e->nnums = reg->n;
  }
  return 0;
}

/* Inputs: demo path (for messages), rt.bqn contents + length, emitted program, and the
 * --save capture (NULL: no pipe, the child inherits stdout exactly as always).
 * Output: bqn's exit code (127 exec failure, 1 on signal death); 2 on temp-file or pipe
 * failure. With a capture, child stdout runs through a pipe: lines opening with the
 * 0x1E record separator collect into cap (sentinel stripped), everything else forwards
 * verbatim so demos look identical. On nonzero the temp .bqn is kept and its path
 * printed; on success it is unlinked. */
static int run_bqn(const char *path, const char *rt, size_t rtlen, const StrBuf *prog, StrBuf *cap) {
  char tmpl[512];
  const char *tdir = getenv("TMPDIR");
  if (!tdir || !*tdir) tdir = "/tmp";
  snprintf(tmpl, sizeof tmpl, "%s/anoc-XXXXXX.bqn", tdir);
  int fd = mkstemps(tmpl, 4);
  if (fd < 0) { fprintf(stderr, "%s: cannot create temp file in %s: %s\n", path, tdir, strerror(errno)); return 2; }
  FILE *f = fdopen(fd, "w");
  if (!f) { close(fd); unlink(tmpl); fprintf(stderr, "%s: fdopen: %s\n", path, strerror(errno)); return 2; }
  fwrite(rt, 1, rtlen, f);
  if (rtlen && rt[rtlen-1] != '\n') fputc('\n', f);
  if (prog->s) fwrite(prog->s, 1, prog->len, f);
  if (fclose(f)) { unlink(tmpl); fprintf(stderr, "%s: write %s: %s\n", path, tmpl, strerror(errno)); return 2; }
  int pfd[2] = { -1, -1 };
  if (cap && pipe(pfd)) { unlink(tmpl); fprintf(stderr, "%s: pipe: %s\n", path, strerror(errno)); return 2; }
  pid_t pid = fork();
  if (pid < 0) { unlink(tmpl); fprintf(stderr, "%s: fork: %s\n", path, strerror(errno)); return 2; }
  if (pid == 0) {                                            /* child: stdio inherited,
                                                                stdout piped under --save */
    if (cap) { dup2(pfd[1], 1); close(pfd[0]); close(pfd[1]); }
    execvp("bqn", (char *const[]){ (char *)"bqn", tmpl, NULL });
    fprintf(stderr, "%s: cannot exec bqn: %s\n", path, strerror(errno));
    _exit(127);
  }
  if (cap) {
    /* read loop: split on newlines; a line opening with 0x1E collects (sentinel
     * stripped, newline kept), anything else forwards verbatim */
    close(pfd[1]);
    StrBuf acc = {0};
    char rb[8192];
    ssize_t got;
    while ((got = read(pfd[0], rb, sizeof rb)) > 0) {
      sb_put(&acc, rb, (size_t)got);
      size_t start = 0;
      for (size_t j = 0; j < acc.len; j++) {
        if (acc.s[j] != '\n') continue;
        if (acc.s[start] == 0x1E) sb_put(cap, acc.s + start + 1, j - start);
        else fwrite(acc.s + start, 1, j - start + 1, stdout);
        start = j + 1;
      }
      memmove(acc.s, acc.s + start, acc.len - start);
      acc.len -= start;
      acc.s[acc.len] = 0;
    }
    if (acc.len) {                                           /* unterminated tail */
      if (acc.s[0] == 0x1E) { sb_put(cap, acc.s + 1, acc.len - 1); sb_put(cap, "\n", 1); }
      else fwrite(acc.s, 1, acc.len, stdout);
    }
    sb_free(&acc);
    close(pfd[0]);
    fflush(stdout);
  }
  int st = 0;
  waitpid(pid, &st, 0);
  int code = WIFEXITED(st) ? WEXITSTATUS(st) : 1;
  if (code == 0) unlink(tmpl);
  else fprintf(stderr, "%s: bqn exited %d, kept %s\n", path, code, tmpl);
  return code;
}

/* Inputs: argv per usage(). Output: exit code 0 ok / 1 test failure / 2 usage or
 * compile error. Default action with no mode flag: --emit to stdout. */
int main(int argc, char **argv) {
  int modeTokens = 0, modeRun = 0, modeEmit = 0;
  const char *rtFlag = NULL, *regFlag = NULL, *dumpFlag = NULL, *saveFlag = NULL, *path = NULL;
  for (int i = 1; i < argc; i++) {
    const char *s = argv[i];
    if (!strcmp(s, "--tokens")) modeTokens = 1;
    else if (!strcmp(s, "--emit")) modeEmit = 1;   /* the default mode; tracked so an
                                                      explicit ask survives --dump */
    else if (!strcmp(s, "--run")) modeRun = 1;
    else if (!strcmp(s, "--dump")) { if (++i >= argc) return usage(); dumpFlag = argv[i]; }
    else if (!strcmp(s, "--save")) { if (++i >= argc) return usage(); saveFlag = argv[i]; }
    else if (!strcmp(s, "--rt")) { if (++i >= argc) return usage(); rtFlag = argv[i]; }
    else if (!strcmp(s, "--registry")) { if (++i >= argc) return usage(); regFlag = argv[i]; }
    else if (s[0] == '-' && s[1]) return usage();
    else if (!path) path = s;
    else return usage();
  }
  if (!path) return usage();
  if (saveFlag && !modeRun) { fprintf(stderr, "anoc: --save requires --run\n"); return 2; }

  char err[ANO_ERRSZ]; err[0] = 0;
  Arena a = {0};
  char *src = fs_read(path, &a, NULL);
  if (!src) { fprintf(stderr, "%s: cannot read: %s\n", path, strerror(errno)); return 2; }

  Directives dirs; memset(&dirs, 0, sizeof dirs);
  dirs.expectN = -1;
  dirs.expects = (Expect *)arena_alloc(&a, MAXEXPECT * sizeof(Expect));
  if (parse_directives(src, &dirs, &a, err, sizeof err)) { fprintf(stderr, "%s: %s\n", path, err); return 2; }

  /* registry: flag wins over directive. Absolute spec used verbatim; a relative one
   * resolves against the source file's directory — a '/'-bearing or .reg-suffixed spec
   * as a literal path (../registries/x.reg), a bare name as <dir>/<name>.reg beside the
   * twin. Both layouts work; the demos keep their fixtures in demos/registries/. */
  Registry reg; memset(&reg, 0, sizeof reg);
  const char *rspec = regFlag ? regFlag : (dirs.registry[0] ? dirs.registry : NULL);
  if (rspec) {
    size_t rl = strlen(rspec);
    int isPath = strchr(rspec, '/') || (rl > 4 && !strcmp(rspec + rl - 4, ".reg"));
    AnoPath dir = fs_dirname(path);
    char spec[ANO_PATHSZ];
    int sl = snprintf(spec, sizeof spec, "%s%s", rspec, isPath ? "" : ".reg");
    AnoPath regp = {0};
    if (dir.len && sl > 0 && sl < (int)sizeof spec) {
      regp = fs_join(dir.str, spec);                 /* absolute rspec passes verbatim */
      fs_canon(&regp);
    }
    if (!regp.len) { fprintf(stderr, "%s: registry path too long: %s\n", path, rspec); return 2; }
    if (reg_load(regp.str, &reg, &a, err, sizeof err)) { fprintf(stderr, "%s: %s\n", path, err); return 2; }
  }

  /* the write-out half of the commit loop: dump the in-memory world (the loaded
   * fixture state) and, with no other mode asked for, stop — the dump was the job */
  if (dumpFlag) {
    if (!rspec) { fprintf(stderr, "%s: --dump needs a registry\n", path); return 2; }
    if (reg_dump(&reg, dumpFlag, err, sizeof err)) { fprintf(stderr, "%s: %s\n", path, err); return 2; }
    if (!modeRun && !modeTokens && !modeEmit) { arena_free(&a); return 0; }
  }
  if (saveFlag && !rspec) { fprintf(stderr, "%s: --save needs a registry\n", path); return 2; }
  dirs.save = saveFlag != NULL;

  Toks toks = {0};
  if (ano_lex(src, dirs.ja, &a, &toks, err, sizeof err)) { fprintf(stderr, "%s: %s\n", path, err); return 2; }

  if (modeTokens) { print_toks(stdout, &toks); return 0; }

  /* ex40 equivalence: the ASCII directive line must lex to the file's own stream */
  if (dirs.sameTokens[0]) {
    Toks dtoks = {0};
    if (ano_lex(dirs.sameTokens, 0, &a, &dtoks, err, sizeof err)) { fprintf(stderr, "%s: same-tokens: %s\n", path, err); return 2; }
    if (!same_stream(&toks, &dtoks, &reg)) {
      fprintf(stderr, "%s: same-tokens mismatch\n-- file stream:\n", path);
      print_toks(stderr, &toks);
      fprintf(stderr, "-- directive stream:\n");
      print_toks(stderr, &dtoks);
      return 1;
    }
  }

  Node *prog = ano_parse(&toks, &a, err, sizeof err);
  if (!prog) { fprintf(stderr, "%s: %s\n", path, err); return 2; }

  StrBuf out = {0};
  if (ano_emit(prog, &reg, &dirs, &out, err, sizeof err)) { fprintf(stderr, "%s: %s\n", path, err); return 2; }

  AnoPath rtp;
  if (rtFlag) rtp = fs_path(rtFlag);
  else {
    AnoPath exed = fs_exe_dir();                     /* len 0: /proc unreadable, fall back to "." */
    rtp = fs_join(exed.len ? exed.str : ".", "rt.bqn");
    fs_canon(&rtp);
  }
  if (!rtp.len) { fprintf(stderr, "%s: runtime path too long\n", path); return 2; }
  size_t rtlen = 0;
  char *rt = fs_read(rtp.str, &a, &rtlen);
  if (!rt) { fprintf(stderr, "%s: cannot read runtime %s: %s\n", path, rtp.str, strerror(errno)); return 2; }

  int code;
  if (modeRun) {
    /* the pipe-back: on exit 0 the captured sentinel lines patch the loaded Registry
     * to post-state and the world commits to <saveFlag> through reg_dump (staged,
     * rename(2), atomic); on a nonzero child exit nothing is written */
    StrBuf cap = {0};
    code = run_bqn(path, rt, rtlen, &out, saveFlag ? &cap : NULL);
    if (saveFlag && code == 0) {
      /* an exit-0 child that never reached the serializer (an early •Exit) must not
       * pass its pre-state off as post-state — no sentinel lines, no save */
      if (cap.len == 0) {
        fprintf(stderr, "%s: save: the program printed no post-state (child exited early?)\n", path);
        code = 2;
      } else if (save_patch(&reg, &a, &cap, err, sizeof err) ||
                 reg_dump(&reg, saveFlag, err, sizeof err)) {
        fprintf(stderr, "%s: %s\n", path, err);
        code = 2;
      }
    }
    sb_free(&cap);
  } else {
    fwrite(rt, 1, rtlen, stdout);
    if (rtlen && rt[rtlen-1] != '\n') fputc('\n', stdout);
    if (out.s) fwrite(out.s, 1, out.len, stdout);
    code = 0;
  }
  sb_free(&out); arena_free(&a);                     /* src and rt live in the arena */
  return code;
}
