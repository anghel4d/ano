/* registry.c — registry loader for anoc: reg_load, reg_find per ano.h.
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

/* Inputs: an entry name or ja alias source word. Output: 0 / -1 via rerr when the
 * lexer owns the word on either surface (lex_reserved) — the name is the address, and
 * a word the closed grammar resolves first is unaddressable. */
static int wfree(const char *w, char *err, size_t errsz, int ln) {
  if (lex_reserved(w))
    return rerr(err, errsz, ln, "'%s' is lexer-reserved and cannot name an entry", w);
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
  size_t flen = 0;
  char *buf = fs_read(path, a, &flen);
  if (!buf) return rerr(err, errsz, 0, "cannot read '%s'", path);

  /* physical lines; index i is line i+1 */
  int nlines = 1;
  for (size_t i = 0; i < flen; i++) if (buf[i] == '\n') nlines++;
  char **lines = (char **)arena_alloc(a, (size_t)nlines * sizeof *lines);
  int li = 0;
  lines[li++] = buf;
  for (size_t i = 0; i < flen; i++)
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
      if (wfree(words[1], err, errsz, ln)) return -1;
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
      if (wfree(words[1], err, errsz, ln)) return -1;
      if (wname(words[1], e->name, sizeof e->name, err, errsz, ln)) return -1;
      if (wnums(words, 2, nw, reg->n, a, &e->nums, &e->nnums, err, errsz, ln)) return -1;

    } else if (strcmp(k, "srel") == 0) {
      if (nw < 2) return rerr(err, errsz, ln, "usage: srel <name> <fibers>");
      RegEntry *e = &reg->ents[reg->nents++];
      e->kind = RK_SREL;
      if (wfree(words[1], err, errsz, ln)) return -1;
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
      if (wfree(words[1], err, errsz, ln)) return -1;
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
      if (wfree(words[1], err, errsz, ln)) return -1;
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
      if (wfree(words[1], err, errsz, ln)) return -1;
      if (wname(words[1], e->name, sizeof e->name, err, errsz, ln)) return -1;
      if (raw2) store_string(a, e, raw2); /* verbatim BQN body convention */

    } else if (strcmp(k, "ja") == 0) {
      if (nw != 3) return rerr(err, errsz, ln, "usage: ja <word> <name>");
      if (wfree(words[1], err, errsz, ln)) return -1;
      if (wname(words[1], reg->jaFrom[reg->nja], ANO_NAMESZ, err, errsz, ln)) return -1;
      if (wname(words[2], reg->jaTo[reg->nja], ANO_NAMESZ, err, errsz, ln)) return -1;
      reg->nja++;

    } else if (strcmp(k, "role") == 0) {
      /* point a system role at a native column; the emitter routes spawn machinery
       * through reg_role, so `role pos 位置` gives a kanji column the `pos` behaviour. */
      if (nw != 3) return rerr(err, errsz, ln, "usage: role <role> <col>");
      static const char *known[] = { "keys", "id", "parent", "proto", "pos" };
      int ok = 0;
      for (size_t r = 0; r < sizeof known / sizeof *known; r++) ok |= !strcmp(words[1], known[r]);
      if (!ok) return rerr(err, errsz, ln, "unknown role '%s' (keys id parent proto pos)", words[1]);
      RegEntry *c = find_exact(reg, words[2]);
      if (!c || c->kind != RK_COL) return rerr(err, errsz, ln, "role: no column '%s'", words[2]);
      if (reg->nroles >= ANO_NROLES) return rerr(err, errsz, ln, "too many roles");
      if (wname(words[1], reg->roleName[reg->nroles], ANO_NAMESZ, err, errsz, ln)) return -1;
      if (wname(words[2], reg->roleCol[reg->nroles], ANO_NAMESZ, err, errsz, ln)) return -1;
      reg->nroles++;

    } else {
      return rerr(err, errsz, ln, "unknown kind '%s'", k);
    }
  }
  return 0;
}

/* Inputs: registry, surface name. Output: entry or NULL. Exact entry names first,
 * case-insensitive on the first letter only ('Gold' matches 'gold'), exact bytes on
 * the rest; then the ja alias hop, exact bytes on both the word and the target —
 * a pure name alias, outranked by real entries, identical from either surface. */
const RegEntry *reg_find(const Registry *reg, const char *name) {
  for (int i = 0; i < reg->nents; i++) {
    const char *e = reg->ents[i].name;
    if (tolower((unsigned char)e[0]) == tolower((unsigned char)name[0]) &&
        (name[0] == 0 || strcmp(e + 1, name + 1) == 0))
      return &reg->ents[i];
  }
  for (int i = 0; i < reg->nja; i++)
    if (strcmp(reg->jaFrom[i], name) == 0) {
      for (int j = 0; j < reg->nents; j++)
        if (strcmp(reg->ents[j].name, reg->jaTo[i]) == 0) return &reg->ents[j];
      return NULL;
    }
  return NULL;
}

/* Inputs: registry, a system role (keys id parent proto pos). Output: the declared
 * role column, else the column named `role` — case-insensitive on the first letter, the
 * same rule reg_find uses, so every ASCII world routes exactly as before — else NULL. */
const RegEntry *reg_role(const Registry *reg, const char *role) {
  for (int i = 0; i < reg->nroles; i++)
    if (strcmp(reg->roleName[i], role) == 0)
      for (int j = 0; j < reg->nents; j++)
        if (strcmp(reg->ents[j].name, reg->roleCol[i]) == 0) return &reg->ents[j];
  for (int i = 0; i < reg->nents; i++) {
    const char *e = reg->ents[i].name;
    if (tolower((unsigned char)e[0]) == tolower((unsigned char)role[0]) &&
        (role[0] == 0 || strcmp(e + 1, role + 1) == 0))
      return &reg->ents[i];
  }
  return NULL;
}
