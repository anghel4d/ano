/* registry.c — dummy-registry loader for anoc: reg_load, reg_find, reg_ja per ano.h.
 *
 * Conventions this loader sets where ano.h lacks a field (emit.c must follow):
 * - vec columns (`col pos vec 0 0 | 1 2 | ...`): RK_COL with type CT_NUM, pairs flattened
 *   row-major in nums, nnums == 2*reg->n. A CT_NUM column whose nnums equals 2*world-n is
 *   a pair column; nnums == n is a scalar column. ano.h has no CT_VEC.
 * - char columns (`col glyph char .#..#.`): RK_COL with type CT_CHAR, the glyph string
 *   stored NUL-terminated starting at syms[0]. Strings longer than ANO_NAMESZ-1 span
 *   contiguous syms slots; nsyms == 1 regardless; read as (const char *)e->syms[0].
 *   The payload is the raw line tail, so glyph strings may contain spaces. Length is
 *   checked in bytes against n (or latW*latH for `field ... char`).
 * - fn raw BQN (`fn fib {𝕩∾+´¯2↑𝕩}`): optional text after the name is captured verbatim
 *   the same way, at syms[0] with nsyms == 1 (contiguous slots when long); nsyms == 0
 *   when the fn line has no body. ano.h has no free-length string field.
 * - `#` begins a comment only at a word boundary (line start or after whitespace), so
 *   mid-word glyph data like `.#.#` survives; a payload word starting with '#' is not
 *   representable in this format.
 * - `inv name rel`: RK_SREL with invOf set; fibers are computed at load from the already
 *   loaded functional rel — fiber for target t is the ascending source ids whose rel
 *   value equals t; nfib == world n. srel fiber counts are stored as written (the ano.h
 *   sample line itself has 8 fibers against n 6), not checked against n.
 */
#include "ano.h"
#include <ctype.h>

/* Inputs: err buffer + size, 1-based line number (0: no line), printf format.
 * Output: -1 always; err holds "registry line N: msg". Invariant: never overflows err. */
static int rerr(char *err, size_t errsz, int line, const char *fmt, ...) {
  if (!err || !errsz) return -1;
  int off = line ? snprintf(err, errsz, "registry line %d: ", line) : 0;
  if (off < 0 || (size_t)off >= errsz) return -1;
  va_list ap; va_start(ap, fmt);
  vsnprintf(err + off, errsz - (size_t)off, fmt, ap);
  va_end(ap);
  return -1;
}

/* Inputs: path, arena, out byte count. Output: NUL-terminated contents in arena, or NULL
 * on I/O failure. Invariant: buffer outlives the registry (same arena). */
static char *read_file(const char *path, Arena *a, long *outlen) {
  FILE *f = fopen(path, "rb");
  if (!f) return NULL;
  if (fseek(f, 0, SEEK_END)) { fclose(f); return NULL; }
  long len = ftell(f);
  if (len < 0 || fseek(f, 0, SEEK_SET)) { fclose(f); return NULL; }
  char *buf = (char *)arena_alloc(a, (size_t)len + 1);
  size_t rd = fread(buf, 1, (size_t)len, f);
  fclose(f);
  buf[rd] = 0;
  *outlen = (long)rd;
  return buf;
}

/* Inputs: mutable line. Output: none; truncates at a word-boundary '#' and trims
 * trailing whitespace/CR in place. Invariant: mid-word '#' is preserved. */
static void strip_line(char *line) {
  int bow = 1; /* at beginning of word */
  for (char *p = line; *p; p++) {
    if (*p == '#' && bow) { *p = 0; break; }
    bow = (*p == ' ' || *p == '\t');
  }
  size_t len = strlen(line);
  while (len && (line[len-1] == ' ' || line[len-1] == '\t' || line[len-1] == '\r'))
    line[--len] = 0;
}

/* Inputs: stripped line, keyword. Output: 1 when the line's first word equals kw. */
static int fw(const char *line, const char *kw) {
  while (*line == ' ' || *line == '\t') line++;
  size_t k = strlen(kw);
  return strncmp(line, kw, k) == 0 && (line[k] == 0 || line[k] == ' ' || line[k] == '\t');
}

/* Inputs: mutable line copy, out word array, capacity. Output: word count; words point
 * into the copy, each NUL-terminated. Invariant: comment already stripped by strip_line. */
static int split_words(char *line, char **words, int maxw) {
  int nw = 0;
  char *p = line;
  while (*p) {
    while (*p == ' ' || *p == '\t') p++;
    if (!*p || nw >= maxw) break;
    words[nw++] = p;
    while (*p && *p != ' ' && *p != '\t') p++;
    if (*p) *p++ = 0;
  }
  return nw;
}

/* Inputs: word. Output: 0 with *out set via strtod, -1 on empty/trailing junk. */
static int wnum(const char *w, double *out) {
  char *end;
  *out = strtod(w, &end);
  return (end != w && *end == 0) ? 0 : -1;
}

/* Inputs: word, destination + size. Output: 0 / -1 via rerr when the word overflows. */
static int wname(const char *w, char *dst, size_t dstsz, char *err, size_t errsz, int ln) {
  size_t l = strlen(w);
  if (l == 0 || l >= dstsz) return rerr(err, errsz, ln, "bad name '%s'", w);
  memcpy(dst, w, l + 1);
  return 0;
}

/* Inputs: words[from..nw), expected count (-1: any), arena. Output: 0 with out and cnt
 * set, -1 via rerr on count mismatch or non-number. */
static int wnums(char **words, int from, int nw, int expect, Arena *a,
                 double **out, int *cnt, char *err, size_t errsz, int ln) {
  int c = nw - from;
  if (expect >= 0 && c != expect)
    return rerr(err, errsz, ln, "expected %d values, got %d", expect, c);
  double *v = (double *)arena_alloc(a, (size_t)(c > 0 ? c : 1) * sizeof *v);
  for (int i = 0; i < c; i++)
    if (wnum(words[from + i], &v[i]))
      return rerr(err, errsz, ln, "bad number '%s'", words[from + i]);
  *out = v;
  *cnt = c;
  return 0;
}

/* Inputs: arena, entry, NUL-terminated string of any length. Output: none; stores the
 * string at syms[0] across contiguous ANO_NAMESZ slots, nsyms = 1. */
static void store_string(Arena *a, RegEntry *e, const char *s) {
  size_t len = strlen(s);
  size_t slots = (len + ANO_NAMESZ) / ANO_NAMESZ; /* >= 1, covers len+1 bytes */
  e->syms = (char (*)[ANO_NAMESZ])arena_alloc(a, slots * ANO_NAMESZ);
  memcpy(e->syms[0], s, len + 1);
  e->nsyms = 1;
}

/* Inputs: registry, name. Output: mutable entry on exact name match, else NULL. */
static RegEntry *find_exact(Registry *reg, const char *name) {
  for (int i = 0; i < reg->nents; i++)
    if (strcmp(reg->ents[i].name, name) == 0) return &reg->ents[i];
  return NULL;
}

/* Inputs: path to a .reg file, out registry, arena, err buffer. Output: 0 ok / -1 with
 * err set. Invariants: all memory from the arena; reg zeroed on entry; conventions per
 * the file-top comment; `n` precedes columns, `lattice` precedes fields. */
int reg_load(const char *path, Registry *reg, Arena *a, char *err, size_t errsz) {
  memset(reg, 0, sizeof *reg);
  if (err && errsz) err[0] = 0;
  long flen = 0;
  char *buf = read_file(path, a, &flen);
  if (!buf) return rerr(err, errsz, 0, "cannot read '%s'", path);

  /* physical lines; index i is line i+1 */
  int nlines = 1;
  for (long i = 0; i < flen; i++) if (buf[i] == '\n') nlines++;
  char **lines = (char **)arena_alloc(a, (size_t)nlines * sizeof *lines);
  int li = 0;
  lines[li++] = buf;
  for (long i = 0; i < flen; i++)
    if (buf[i] == '\n') { buf[i] = 0; if (li < nlines) lines[li++] = buf + i + 1; }
  for (int i = 0; i < nlines; i++) strip_line(lines[i]);

  /* pass 1: capacity */
  static const char *entkinds[] = { "col", "rel", "srel", "inv", "alias", "bind", "fn", "field" };
  int entcap = 0, jacap = 0;
  for (int i = 0; i < nlines; i++) {
    for (size_t k = 0; k < sizeof entkinds / sizeof *entkinds; k++)
      if (fw(lines[i], entkinds[k])) { entcap++; break; }
    if (fw(lines[i], "ja")) jacap++;
  }
  reg->ents = (RegEntry *)arena_alloc(a, (size_t)(entcap ? entcap : 1) * sizeof *reg->ents);
  reg->jaFrom = (char (*)[ANO_NAMESZ])arena_alloc(a, (size_t)(jacap ? jacap : 1) * ANO_NAMESZ);
  reg->jaTo = (char (*)[ANO_NAMESZ])arena_alloc(a, (size_t)(jacap ? jacap : 1) * ANO_NAMESZ);
  char **words = (char **)arena_alloc(a, ((size_t)flen / 2 + 2) * sizeof *words);

  /* pass 2: parse */
  for (int i = 0; i < nlines; i++) {
    int ln = i + 1;
    char *orig = lines[i];
    char *dup = arena_strdup(a, orig, strlen(orig));
    int nw = split_words(dup, words, (int)((size_t)flen / 2 + 2));
    if (nw == 0) continue;
    /* raw tails into the untouched original, for space-carrying payloads */
    const char *raw2 = nw > 2 ? orig + (words[2] - dup) : NULL;
    const char *raw3 = nw > 3 ? orig + (words[3] - dup) : NULL;
    const char *k = words[0];

    if (strcmp(k, "n") == 0) {
      double v;
      if (nw != 2 || wnum(words[1], &v) || v < 0)
        return rerr(err, errsz, ln, "usage: n <count>");
      reg->n = (int)v;

    } else if (strcmp(k, "lattice") == 0) {
      double w, h;
      if (nw != 3 || wnum(words[1], &w) || wnum(words[2], &h) || w < 0 || h < 0)
        return rerr(err, errsz, ln, "usage: lattice <w> <h>");
      reg->latW = (int)w;
      reg->latH = (int)h;

    } else if (strcmp(k, "col") == 0 || strcmp(k, "field") == 0) {
      int isField = (k[0] == 'f');
      int rows = isField ? reg->latW * reg->latH : reg->n;
      if (nw < 3) return rerr(err, errsz, ln, "usage: %s <name> <type> <values>", k);
      if (isField && rows <= 0) return rerr(err, errsz, ln, "field before lattice");
      RegEntry *e = &reg->ents[reg->nents++];
      e->kind = isField ? RK_FIELD : RK_COL;
      if (wname(words[1], e->name, sizeof e->name, err, errsz, ln)) return -1;
      const char *ty = words[2];
      if (strcmp(ty, "num") == 0 || strcmp(ty, "bool") == 0) {
        e->type = ty[0] == 'n' ? CT_NUM : CT_BOOL;
        if (wnums(words, 3, nw, rows, a, &e->nums, &e->nnums, err, errsz, ln)) return -1;
      } else if (strcmp(ty, "sym") == 0) {
        e->type = CT_SYM;
        if (nw - 3 != rows)
          return rerr(err, errsz, ln, "expected %d values, got %d", rows, nw - 3);
        e->syms = (char (*)[ANO_NAMESZ])arena_alloc(a, (size_t)(rows ? rows : 1) * ANO_NAMESZ);
        for (int j = 0; j < rows; j++)
          if (wname(words[3 + j], e->syms[j], ANO_NAMESZ, err, errsz, ln)) return -1;
        e->nsyms = rows;
      } else if (strcmp(ty, "char") == 0) {
        e->type = CT_CHAR;
        if (!raw3) return rerr(err, errsz, ln, "char %s needs a glyph string", k);
        if ((int)strlen(raw3) != rows)
          return rerr(err, errsz, ln, "expected %d glyphs, got %zu", rows, strlen(raw3));
        store_string(a, e, raw3);
      } else if (strcmp(ty, "vec") == 0) {
        /* pairs '|'-separated; flattened, nnums = 2*rows (pair-column convention) */
        e->type = CT_NUM;
        double *v = (double *)arena_alloc(a, (size_t)(rows ? 2 * rows : 1) * sizeof *v);
        int g = 0, in = 0, vi = 0;
        for (int j = 3; j < nw; j++) {
          if (strcmp(words[j], "|") == 0) {
            if (in != 2) return rerr(err, errsz, ln, "vec row %d needs 2 values", g);
            g++; in = 0;
          } else {
            if (in >= 2 || g >= rows) return rerr(err, errsz, ln, "vec row %d needs 2 values", g);
            if (wnum(words[j], &v[vi++])) return rerr(err, errsz, ln, "bad number '%s'", words[j]);
            in++;
          }
        }
        if (in != 2) return rerr(err, errsz, ln, "vec row %d needs 2 values", g);
        g++;
        if (g != rows) return rerr(err, errsz, ln, "expected %d vec rows, got %d", rows, g);
        e->nums = v;
        e->nnums = 2 * rows;
      } else {
        return rerr(err, errsz, ln, "unknown col type '%s'", ty);
      }

    } else if (strcmp(k, "pres") == 0) {
      if (nw < 2) return rerr(err, errsz, ln, "usage: pres <col> <mask>");
      RegEntry *e = find_exact(reg, words[1]);
      if (!e || e->kind != RK_COL) return rerr(err, errsz, ln, "pres: no column '%s'", words[1]);
      int cnt;
      if (wnums(words, 2, nw, reg->n, a, &e->pres, &cnt, err, errsz, ln)) return -1;
      e->hasPres = 1;

    } else if (strcmp(k, "default") == 0) {
      double v;
      if (nw != 3 || wnum(words[2], &v)) return rerr(err, errsz, ln, "usage: default <name> <v>");
      RegEntry *e = find_exact(reg, words[1]);
      if (!e) return rerr(err, errsz, ln, "default: no entry '%s'", words[1]);
      e->defval = v;

    } else if (strcmp(k, "rel") == 0 || strcmp(k, "alias") == 0) {
      if (nw < 2) return rerr(err, errsz, ln, "usage: %s <name> <values>", k);
      RegEntry *e = &reg->ents[reg->nents++];
      e->kind = k[0] == 'r' ? RK_REL : RK_ALIAS;
      if (wname(words[1], e->name, sizeof e->name, err, errsz, ln)) return -1;
      if (wnums(words, 2, nw, reg->n, a, &e->nums, &e->nnums, err, errsz, ln)) return -1;

    } else if (strcmp(k, "srel") == 0) {
      if (nw < 2) return rerr(err, errsz, ln, "usage: srel <name> <fibers>");
      RegEntry *e = &reg->ents[reg->nents++];
      e->kind = RK_SREL;
      if (wname(words[1], e->name, sizeof e->name, err, errsz, ln)) return -1;
      int nfib = 1, nvals = 0;
      for (int j = 2; j < nw; j++) strcmp(words[j], "|") == 0 ? nfib++ : nvals++;
      e->fibOff = (int *)arena_alloc(a, (size_t)nfib * sizeof *e->fibOff);
      e->fibLen = (int *)arena_alloc(a, (size_t)nfib * sizeof *e->fibLen);
      e->fibVals = (double *)arena_alloc(a, (size_t)(nvals ? nvals : 1) * sizeof *e->fibVals);
      int fib = 0, vi = 0;
      e->fibOff[0] = 0;
      for (int j = 2; j < nw; j++) {
        if (strcmp(words[j], "|") == 0) {
          e->fibLen[fib] = vi - e->fibOff[fib];
          fib++;
          e->fibOff[fib] = vi;
        } else if (wnum(words[j], &e->fibVals[vi++])) {
          return rerr(err, errsz, ln, "bad number '%s'", words[j]);
        }
      }
      e->fibLen[fib] = vi - e->fibOff[fib];
      e->nfib = fib + 1;

    } else if (strcmp(k, "inv") == 0) {
      if (nw != 3) return rerr(err, errsz, ln, "usage: inv <name> <rel>");
      RegEntry *rel = find_exact(reg, words[2]);
      if (!rel || rel->kind != RK_REL)
        return rerr(err, errsz, ln, "inv: no functional rel '%s'", words[2]);
      RegEntry *e = &reg->ents[reg->nents++];
      e->kind = RK_SREL;
      if (wname(words[1], e->name, sizeof e->name, err, errsz, ln)) return -1;
      if (wname(words[2], e->invOf, sizeof e->invOf, err, errsz, ln)) return -1;
      /* fiber for target t: ascending source ids with rel==t; nfib = world n */
      int n = reg->n;
      e->fibOff = (int *)arena_alloc(a, (size_t)(n ? n : 1) * sizeof *e->fibOff);
      e->fibLen = (int *)arena_alloc(a, (size_t)(n ? n : 1) * sizeof *e->fibLen);
      e->fibVals = (double *)arena_alloc(a, (size_t)(n ? n : 1) * sizeof *e->fibVals);
      int vi = 0;
      for (int t = 0; t < n; t++) {
        e->fibOff[t] = vi;
        for (int s = 0; s < rel->nnums; s++)
          if (rel->nums[s] == (double)t) e->fibVals[vi++] = s;
        e->fibLen[t] = vi - e->fibOff[t];
      }
      e->nfib = n;

    } else if (strcmp(k, "bind") == 0) {
      if (nw < 3) return rerr(err, errsz, ln, "usage: bind <name> <kind> <values>");
      RegEntry *e = &reg->ents[reg->nents++];
      e->kind = RK_BIND;
      if (wname(words[1], e->name, sizeof e->name, err, errsz, ln)) return -1;
      const char *bk = words[2];
      int expect;
      if (strcmp(bk, "entity") == 0 || strcmp(bk, "num") == 0) expect = 1;
      else if (strcmp(bk, "point") == 0) expect = 2;
      else if (strcmp(bk, "mask") == 0) expect = reg->n;
      else if (strcmp(bk, "vec") == 0) expect = -1;
      else return rerr(err, errsz, ln, "unknown bind kind '%s'", bk);
      if (wname(bk, e->bindKind, sizeof e->bindKind, err, errsz, ln)) return -1;
      if (wnums(words, 3, nw, expect, a, &e->nums, &e->nnums, err, errsz, ln)) return -1;

    } else if (strcmp(k, "fn") == 0) {
      if (nw < 2) return rerr(err, errsz, ln, "usage: fn <name> [bqn]");
      RegEntry *e = &reg->ents[reg->nents++];
      e->kind = RK_FN;
      if (wname(words[1], e->name, sizeof e->name, err, errsz, ln)) return -1;
      if (raw2) store_string(a, e, raw2); /* verbatim BQN body convention */

    } else if (strcmp(k, "ja") == 0) {
      if (nw != 3) return rerr(err, errsz, ln, "usage: ja <word> <name>");
      if (wname(words[1], reg->jaFrom[reg->nja], ANO_NAMESZ, err, errsz, ln)) return -1;
      if (wname(words[2], reg->jaTo[reg->nja], ANO_NAMESZ, err, errsz, ln)) return -1;
      reg->nja++;

    } else {
      return rerr(err, errsz, ln, "unknown kind '%s'", k);
    }
  }
  return 0;
}

/* Inputs: registry, surface name. Output: entry or NULL. Invariant: case-insensitive on
 * the first letter only ('Gold' matches 'gold'), exact bytes on the rest. */
const RegEntry *reg_find(const Registry *reg, const char *name) {
  for (int i = 0; i < reg->nents; i++) {
    const char *e = reg->ents[i].name;
    if (tolower((unsigned char)e[0]) == tolower((unsigned char)name[0]) &&
        (name[0] == 0 || strcmp(e + 1, name + 1) == 0))
      return &reg->ents[i];
  }
  return NULL;
}

/* Inputs: registry, JA surface word. Output: mapped registry name or NULL.
 * Invariant: exact byte match on the word. */
const char *reg_ja(const Registry *reg, const char *jaWord) {
  for (int i = 0; i < reg->nja; i++)
    if (strcmp(reg->jaFrom[i], jaWord) == 0) return reg->jaTo[i];
  return NULL;
}

/* ---------- self-test ---------- */
#ifdef REG_TEST

#define CHECK(c) do { if (!(c)) { \
  fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #c); return 1; } } while (0)

/* Inputs: path, contents. Output: 0 / 1 on I/O failure. */
static int put(const char *path, const char *s) {
  FILE *f = fopen(path, "w");
  if (!f) return 1;
  fputs(s, f);
  fclose(f);
  return 0;
}

int main(void) {
  const char *tmp = getenv("TMPDIR");
  char path[512], bad[512];
  snprintf(path, sizeof path, "%s/anoc_reg_test.reg", tmp ? tmp : "/tmp");
  snprintf(bad, sizeof bad, "%s/anoc_reg_bad.reg", tmp ? tmp : "/tmp");

  CHECK(put(path,
    "# test world\n"
    "n 6\n"
    "col gold num 100 200 300 400 500 600  # trailing comment\n"
    "col cattle bool 1 0 1 0 1 1\n"
    "col faction sym Bandit Player Bandit Nord Nord Nord\n"
    "col twoHanded num 30 70 10 61 80 90\n"
    "col pos vec 0 0 | 1 2 | 3 4 | 5 6 | 7 8 | 9 10\n"
    "col glyph char .#.#.#\n"
    "pres twoHanded 1 1 0 1 1 1\n"
    "default gold 5\n"
    "rel pen -1 0 3 -1 2 2\n"
    "inv livestock pen\n"
    "srel targets 3 4 | 4 5 | | | |\n"
    "alias cursor 0 0 1 0 0 0\n"
    "bind Player entity 2\n"
    "bind Whiterun mask 1 1 0 0 1 1\n"
    "bind rally point 10 20\n"
    "bind spacing num 4\n"
    "bind weights vec 1 2 3\n"
    "fn fib {\xf0\x9d\x95\xa9\xe2\x88\xbe+\xc2\xb4\xc2\xaf""2\xe2\x86\x91\xf0\x9d\x95\xa9}\n"
    "fn noop\n"
    "fn spaced { 1 + 2 }\n"
    "lattice 2 3\n"
    "field elevation num 0 1 2 3 4 5\n"
    "ja \xe5\x8c\x97 nord\n"
    "ja \xe9\x87\x91 gold\n") == 0);

  Arena a = {0};
  Registry reg;
  char err[ANO_ERRSZ];
  CHECK(reg_load(path, &reg, &a, err, sizeof err) == 0);

  CHECK(reg.n == 6 && reg.latW == 2 && reg.latH == 3);
  CHECK(reg.nents == 19 && reg.nja == 2); /* 6 col, rel, inv, srel, alias, 5 bind, 3 fn, field */

  const RegEntry *e = reg_find(&reg, "Gold"); /* first-letter case fold */
  CHECK(e && e->kind == RK_COL && e->type == CT_NUM && e->nnums == 6);
  CHECK(e->nums[0] == 100 && e->nums[5] == 600 && e->defval == 5);
  CHECK(reg_find(&reg, "gold") == e);
  CHECK(reg_find(&reg, "GOLD") == NULL); /* rest is exact */

  e = reg_find(&reg, "cattle");
  CHECK(e && e->type == CT_BOOL && e->nums[1] == 0 && e->nums[2] == 1);

  e = reg_find(&reg, "faction");
  CHECK(e && e->type == CT_SYM && e->nsyms == 6 && strcmp(e->syms[3], "Nord") == 0);

  e = reg_find(&reg, "pos"); /* vec: pair column, nnums == 2*n */
  CHECK(e && e->type == CT_NUM && e->nnums == 12);
  CHECK(e->nums[0] == 0 && e->nums[2] == 1 && e->nums[3] == 2 && e->nums[11] == 10);

  e = reg_find(&reg, "glyph"); /* char: mid-word '#' survives comment stripping */
  CHECK(e && e->type == CT_CHAR && e->nsyms == 1 && strcmp(e->syms[0], ".#.#.#") == 0);

  e = reg_find(&reg, "twoHanded");
  CHECK(e && e->hasPres && e->pres[2] == 0 && e->pres[3] == 1);

  e = reg_find(&reg, "pen");
  CHECK(e && e->kind == RK_REL && e->nums[0] == -1 && e->nums[2] == 3);

  e = reg_find(&reg, "livestock"); /* inv fibers from pen = -1 0 3 -1 2 2 */
  CHECK(e && e->kind == RK_SREL && strcmp(e->invOf, "pen") == 0 && e->nfib == 6);
  CHECK(e->fibLen[0] == 1 && e->fibVals[e->fibOff[0]] == 1);
  CHECK(e->fibLen[1] == 0);
  CHECK(e->fibLen[2] == 2 && e->fibVals[e->fibOff[2]] == 4 && e->fibVals[e->fibOff[2] + 1] == 5);
  CHECK(e->fibLen[3] == 1 && e->fibVals[e->fibOff[3]] == 2);
  CHECK(e->fibLen[4] == 0 && e->fibLen[5] == 0);

  e = reg_find(&reg, "targets"); /* srel: 5 pipes -> 6 fibers */
  CHECK(e && e->kind == RK_SREL && e->invOf[0] == 0 && e->nfib == 6);
  CHECK(e->fibLen[0] == 2 && e->fibVals[0] == 3 && e->fibVals[1] == 4);
  CHECK(e->fibLen[1] == 2 && e->fibVals[2] == 4 && e->fibVals[3] == 5);
  CHECK(e->fibLen[2] == 0 && e->fibLen[5] == 0);

  e = reg_find(&reg, "cursor");
  CHECK(e && e->kind == RK_ALIAS && e->nnums == 6 && e->nums[2] == 1);

  e = reg_find(&reg, "player");
  CHECK(e && e->kind == RK_BIND && strcmp(e->bindKind, "entity") == 0 && e->nums[0] == 2);
  e = reg_find(&reg, "Whiterun");
  CHECK(e && strcmp(e->bindKind, "mask") == 0 && e->nnums == 6 && e->nums[4] == 1);
  e = reg_find(&reg, "rally");
  CHECK(e && strcmp(e->bindKind, "point") == 0 && e->nums[0] == 10 && e->nums[1] == 20);
  e = reg_find(&reg, "spacing");
  CHECK(e && strcmp(e->bindKind, "num") == 0 && e->nums[0] == 4);
  e = reg_find(&reg, "weights");
  CHECK(e && strcmp(e->bindKind, "vec") == 0 && e->nnums == 3 && e->nums[2] == 3);

  e = reg_find(&reg, "fib"); /* verbatim BQN in syms[0] */
  CHECK(e && e->kind == RK_FN && e->nsyms == 1);
  CHECK(strcmp(e->syms[0], "{\xf0\x9d\x95\xa9\xe2\x88\xbe+\xc2\xb4\xc2\xaf""2\xe2\x86\x91\xf0\x9d\x95\xa9}") == 0);
  e = reg_find(&reg, "noop");
  CHECK(e && e->kind == RK_FN && e->nsyms == 0);
  e = reg_find(&reg, "spaced"); /* body spaces preserved */
  CHECK(e && e->nsyms == 1 && strcmp(e->syms[0], "{ 1 + 2 }") == 0);

  e = reg_find(&reg, "elevation");
  CHECK(e && e->kind == RK_FIELD && e->type == CT_NUM && e->nnums == 6 && e->nums[5] == 5);

  CHECK(reg_ja(&reg, "\xe5\x8c\x97") && strcmp(reg_ja(&reg, "\xe5\x8c\x97"), "nord") == 0);
  CHECK(reg_ja(&reg, "\xe9\x87\x91") && strcmp(reg_ja(&reg, "\xe9\x87\x91"), "gold") == 0);
  CHECK(reg_ja(&reg, "nord") == NULL);

  /* error paths */
  Registry r2;
  CHECK(put(bad, "n 2\ncol g num 1 2 3\n") == 0);
  CHECK(reg_load(bad, &r2, &a, err, sizeof err) == -1 && err[0]);
  CHECK(put(bad, "n 2\nzzz 1\n") == 0);
  CHECK(reg_load(bad, &r2, &a, err, sizeof err) == -1 && err[0]);
  CHECK(put(bad, "n 2\npres ghost 1 0\n") == 0);
  CHECK(reg_load(bad, &r2, &a, err, sizeof err) == -1 && err[0]);
  CHECK(put(bad, "n 2\ninv x ghost\n") == 0);
  CHECK(reg_load(bad, &r2, &a, err, sizeof err) == -1 && err[0]);
  CHECK(put(bad, "field f num 1\n") == 0);
  CHECK(reg_load(bad, &r2, &a, err, sizeof err) == -1 && err[0]);
  CHECK(put(bad, "n 2\ncol p vec 1 | 2 3\n") == 0);
  CHECK(reg_load(bad, &r2, &a, err, sizeof err) == -1 && err[0]);

  arena_free(&a);
  remove(path);
  remove(bad);
  printf("ok\n");
  return 0;
}
#endif
