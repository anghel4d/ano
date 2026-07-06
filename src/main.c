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
  fprintf(stderr, "usage: anoc [--tokens] [--emit] [--run] [--rt <path>] [--registry <path-or-name>] file.ano\n");
  return 2;
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

/* Inputs: two token streams. Output: 1 when kind/name/num sequences match ignoring
 * T_NL/T_EOF, else 0. Invariant: nums compare exactly — both streams come from ano_lex. */
static int same_stream(const Toks *x, const Toks *y) {
  int i = 0, j = 0;
  for (;;) {
    while (i < x->n && (x->kind[i] == T_NL || x->kind[i] == T_EOF)) i++;
    while (j < y->n && (y->kind[j] == T_NL || y->kind[j] == T_EOF)) j++;
    if (i >= x->n || j >= y->n) return i >= x->n && j >= y->n;
    if (x->kind[i] != y->kind[j] || strcmp(x->name[i], y->name[j]) || x->num[i] != y->num[j]) return 0;
    i++; j++;
  }
}

/* Inputs: demo path (for messages), rt.bqn contents + length, emitted program.
 * Output: bqn's exit code (127 exec failure, 1 on signal death); 2 on temp-file failure.
 * On nonzero the temp .bqn is kept and its path printed; on success it is unlinked. */
static int run_bqn(const char *path, const char *rt, size_t rtlen, const StrBuf *prog) {
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
  pid_t pid = fork();
  if (pid < 0) { unlink(tmpl); fprintf(stderr, "%s: fork: %s\n", path, strerror(errno)); return 2; }
  if (pid == 0) {                                            /* child: stdio inherited */
    execvp("bqn", (char *const[]){ (char *)"bqn", tmpl, NULL });
    fprintf(stderr, "%s: cannot exec bqn: %s\n", path, strerror(errno));
    _exit(127);
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
  int modeTokens = 0, modeRun = 0;
  const char *rtFlag = NULL, *regFlag = NULL, *path = NULL;
  for (int i = 1; i < argc; i++) {
    const char *s = argv[i];
    if (!strcmp(s, "--tokens")) modeTokens = 1;
    else if (!strcmp(s, "--emit")) { /* the default mode */ }
    else if (!strcmp(s, "--run")) modeRun = 1;
    else if (!strcmp(s, "--rt")) { if (++i >= argc) return usage(); rtFlag = argv[i]; }
    else if (!strcmp(s, "--registry")) { if (++i >= argc) return usage(); regFlag = argv[i]; }
    else if (s[0] == '-' && s[1]) return usage();
    else if (!path) path = s;
    else return usage();
  }
  if (!path) return usage();

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

  Toks toks = {0};
  if (ano_lex(src, dirs.ja, &reg, &a, &toks, err, sizeof err)) { fprintf(stderr, "%s: %s\n", path, err); return 2; }

  if (modeTokens) { print_toks(stdout, &toks); return 0; }

  /* ex40 equivalence: the ASCII directive line must lex to the file's own stream */
  if (dirs.sameTokens[0]) {
    Toks dtoks = {0};
    if (ano_lex(dirs.sameTokens, 0, &reg, &a, &dtoks, err, sizeof err)) { fprintf(stderr, "%s: same-tokens: %s\n", path, err); return 2; }
    if (!same_stream(&toks, &dtoks)) {
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
  if (modeRun) code = run_bqn(path, rt, rtlen, &out);
  else {
    fwrite(rt, 1, rtlen, stdout);
    if (rtlen && rt[rtlen-1] != '\n') fputc('\n', stdout);
    if (out.s) fwrite(out.s, 1, out.len, stdout);
    code = 0;
  }
  sb_free(&out); arena_free(&a);                     /* src and rt live in the arena */
  return code;
}
