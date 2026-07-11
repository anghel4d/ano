/* registry.c — registry loader and dumper for anoc: reg_load, reg_find, reg_role,
 * reg_dump, names_eq per ano.h.
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
#include <errno.h>
#include <math.h>

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

/* Inputs: word. Output: 0 with *out set via strtod, -1 on empty/trailing junk or a
 * non-finite parse (inf, nan, any spelling strtod accepts, hex-float overflow included).
 * The registry's value domain is exactly the finite doubles — the save already refuses
 * to write a non-finite world, so the load refuses to read one, and load ∘ save = id. */
static int wnum(const char *w, double *out) {
  char *end;
  *out = strtod(w, &end);
  return (end != w && *end == 0 && isfinite(*out)) ? 0 : -1;
}

/* Inputs: word, destination + size. Output: 0 / -1 via rerr when the word overflows. */
static int wname(const char *w, char *dst, size_t dstsz, char *err, size_t errsz, int ln) {
  size_t l = strlen(w);
  if (l == 0 || l >= dstsz) return rerr(err, errsz, ln, "bad name '%s'", w);
  memcpy(dst, w, l + 1);
  return 0;
}

/* Inputs: two names. Output: 1 when equal under the case contract — ASCII letters
 * compare case-folded, every other byte exact, so kanji and all UTF-8 names are
 * untouched by construction. Names fold; values never do. */
int names_eq(const char *a, const char *b) {
  for (;; a++, b++) {
    unsigned char x = (unsigned char)*a, y = (unsigned char)*b;
    if (x >= 'A' && x <= 'Z') x += 32;
    if (y >= 'A' && y <= 'Z') y += 32;
    if (x != y) return 0;
    if (!x) return 1;
  }
}

/* Inputs: an entry name or alias source word. Output: 0 / -1 via rerr when the lexer
 * owns the word on either surface (lex_reserved_fold) — the name is the address, and
 * a word the closed grammar resolves first in any spelling is unaddressable. */
static int wfree(const char *w, char *err, size_t errsz, int ln) {
  if (lex_reserved_fold(w))
    return rerr(err, errsz, ln, "'%s' is lexer-reserved and cannot name an entry", w);
  return 0;
}

/* Inputs: registry, the just-named entry (already counted). Output: 0 / -1 via rerr
 * when another entry's name folds equal — two spellings of one name are one name — or
 * when the name folds onto an identifier the emitted BQN reserves (the fixture's
 * anoN/anoSel/anoIdx, the --save serializer's three, the rt.bqn prelude helpers): BQN
 * identifiers are case-insensitive, so the fold IS the collision. Exactly these names,
 * never a blanket ano prefix — `anointed` stays legal. */
static int wuniq(Registry *reg, const RegEntry *e, char *err, size_t errsz, int ln) {
  static const char *resv[] = { "anoN", "anoSel", "anoIdx",
    "anoSaveSep", "AnoSaveNum", "AnoSaveRow",
    "AnoRank", "AnoScat", "AnoAvg", "AnoNbrClamp", "AnoImage", "AnoInvFib" };
  for (size_t i = 0; i < sizeof resv / sizeof *resv; i++)
    if (names_eq(e->name, resv[i]))
      return rerr(err, errsz, ln, "'%s' collides with the emitter's reserved '%s' under the case fold",
                  e->name, resv[i]);
  for (int i = 0; i < reg->nents; i++)
    if (&reg->ents[i] != e && names_eq(reg->ents[i].name, e->name))
      return rerr(err, errsz, ln, "'%s' collides with entry '%s' under the case fold",
                  e->name, reg->ents[i].name);
  return 0;
}

/* Inputs: registry, an alias source word. Output: 0 / -1 via rerr when an already
 * declared alias source folds equal — same rule as wuniq, per name kind. */
static int wuniq_alias(Registry *reg, const char *w, char *err, size_t errsz, int ln) {
  for (int i = 0; i < reg->nas; i++)
    if (names_eq(reg->asFrom[i], w))
      return rerr(err, errsz, ln, "alias source '%s' collides with '%s' under the case fold",
                  w, reg->asFrom[i]);
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

/* Inputs: registry, name. Output: mutable entry under names_eq, else NULL — the same
 * comparator every resolution uses, serving the pres/default/inv/role/as targets. */
static RegEntry *find_ent(Registry *reg, const char *name) {
  for (int i = 0; i < reg->nents; i++)
    if (names_eq(reg->ents[i].name, name)) return &reg->ents[i];
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

  /* pass 1: capacity; an `as` line is an alias or a derived-tag entry by arity, so it
   * counts toward both (arena, over-allocation is free) */
  static const char *entkinds[] = { "col", "rel", "srel", "inv", "alias", "bind", "fn", "field", "as", "unique", "def" };
  int entcap = 0, ascap = 0;
  for (int i = 0; i < nlines; i++) {
    for (size_t k = 0; k < sizeof entkinds / sizeof *entkinds; k++)
      if (fw(lines[i], entkinds[k])) { entcap++; break; }
    if (fw(lines[i], "ja") || fw(lines[i], "as")) ascap++;
  }
  reg->ents = (RegEntry *)arena_alloc(a, (size_t)(entcap ? entcap : 1) * sizeof *reg->ents);
  reg->asFrom = (char (*)[ANO_NAMESZ])arena_alloc(a, (size_t)(ascap ? ascap : 1) * ANO_NAMESZ);
  reg->asTo = (char (*)[ANO_NAMESZ])arena_alloc(a, (size_t)(ascap ? ascap : 1) * ANO_NAMESZ);
  reg->asJa = (unsigned char *)arena_alloc(a, (size_t)(ascap ? ascap : 1));
  char **words = (char **)arena_alloc(a, ((size_t)flen / 2 + 2) * sizeof *words);

  /* pass 2: parse */
  int sawN = 0, sawLat = 0;
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
      /* one header: a redeclared n would let lines validate against different counts,
       * and the world would have no one-header spelling for reg_dump to write back */
      if (sawN++) return rerr(err, errsz, ln, "n redeclared");
      reg->n = (int)v;

    } else if (strcmp(k, "lattice") == 0) {
      double w, h;
      if (nw != 3 || wnum(words[1], &w) || wnum(words[2], &h) || w < 0 || h < 0)
        return rerr(err, errsz, ln, "usage: lattice <w> <h>");
      if (sawLat++) return rerr(err, errsz, ln, "lattice redeclared");
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
      if (wuniq(reg, e, err, errsz, ln)) return -1;
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

    } else if (strcmp(k, "unique") == 0) {
      /* declared injectivity: one num column, every element pairwise-distinct. The check
       * IS the ruled content (2026-07-11); mint-on-spawn and write-refusal ride on it. */
      if (nw < 3) return rerr(err, errsz, ln, "usage: unique <name> <values>");
      RegEntry *e = &reg->ents[reg->nents++];
      e->kind = RK_COL;
      e->type = CT_NUM;
      e->uniq = 1;
      if (wfree(words[1], err, errsz, ln)) return -1;
      if (wname(words[1], e->name, sizeof e->name, err, errsz, ln)) return -1;
      if (wuniq(reg, e, err, errsz, ln)) return -1;
      if (wnums(words, 2, nw, reg->n, a, &e->nums, &e->nnums, err, errsz, ln)) return -1;
      for (int x = 0; x < e->nnums; x++)
        for (int y = x + 1; y < e->nnums; y++)
          if (e->nums[x] == e->nums[y])
            return rerr(err, errsz, ln, "unique %s: value %g repeats (rows %d, %d)",
                        e->name, e->nums[x], x, y);

    } else if (strcmp(k, "pres") == 0) {
      if (nw < 2) return rerr(err, errsz, ln, "usage: pres <col> <mask>");
      RegEntry *e = find_ent(reg, words[1]);
      if (!e || e->kind != RK_COL) return rerr(err, errsz, ln, "pres: no column '%s'", words[1]);
      if (e->uniq) return rerr(err, errsz, ln, "pres on unique column '%s': a key column is total", e->name);
      int cnt;
      if (wnums(words, 2, nw, reg->n, a, &e->pres, &cnt, err, errsz, ln)) return -1;
      e->hasPres = 1;

    } else if (strcmp(k, "default") == 0) {
      double v;
      if (nw != 3) return rerr(err, errsz, ln, "usage: default <name> <v>");
      if (wnum(words[2], &v)) return rerr(err, errsz, ln, "bad number '%s'", words[2]);
      RegEntry *e = find_ent(reg, words[1]);
      if (!e) return rerr(err, errsz, ln, "default: no entry '%s'", words[1]);
      /* a unique column's values are minted, never defaulted: a shared default is a
       * standing violation of the declared injectivity */
      if (e->uniq) return rerr(err, errsz, ln, "default on unique column '%s'", e->name);
      e->defval = v;

    } else if (strcmp(k, "rel") == 0 || strcmp(k, "alias") == 0) {
      if (nw < 2) return rerr(err, errsz, ln, "usage: %s <name> <values>", k);
      /* keyed form (rel only): data is always numeric, so two names before it mean the
       * first is the key column and the second the declared name — LL(1), no lookahead */
      double kd;
      int keyed = k[0] == 'r' && nw >= 3 && wnum(words[2], &kd) && strcmp(words[2], "|");
      int ni = keyed ? 2 : 1;
      RegEntry *e = &reg->ents[reg->nents++];
      e->kind = k[0] == 'r' ? RK_REL : RK_ALIAS;
      if (wfree(words[ni], err, errsz, ln)) return -1;
      if (wname(words[ni], e->name, sizeof e->name, err, errsz, ln)) return -1;
      if (wuniq(reg, e, err, errsz, ln)) return -1;
      if (keyed) {
        RegEntry *kc = find_ent(reg, words[1]);
        if (!kc || !kc->uniq)
          return rerr(err, errsz, ln, "rel %s: key '%s' is not a unique column", e->name, words[1]);
        for (int j = 0; j < kc->nnums; j++)
          if (kc->nums[j] < 0)
            return rerr(err, errsz, ln, "rel %s: key column '%s' holds a negative value — "
                        "-1 is the dangling sentinel", e->name, kc->name);
        if (wname(kc->name, e->keyOf, sizeof e->keyOf, err, errsz, ln)) return -1;
      }
      if (wnums(words, ni + 1, nw, reg->n, a, &e->nums, &e->nnums, err, errsz, ln)) return -1;

    } else if (strcmp(k, "srel") == 0) {
      if (nw < 2) return rerr(err, errsz, ln, "usage: srel <name> <fibers>");
      double kd;
      int keyed = nw >= 3 && wnum(words[2], &kd) && strcmp(words[2], "|");
      int ni = keyed ? 2 : 1;
      RegEntry *e = &reg->ents[reg->nents++];
      e->kind = RK_SREL;
      if (wfree(words[ni], err, errsz, ln)) return -1;
      if (wname(words[ni], e->name, sizeof e->name, err, errsz, ln)) return -1;
      if (wuniq(reg, e, err, errsz, ln)) return -1;
      if (keyed) {
        RegEntry *kc = find_ent(reg, words[1]);
        if (!kc || !kc->uniq)
          return rerr(err, errsz, ln, "srel %s: key '%s' is not a unique column", e->name, words[1]);
        if (wname(kc->name, e->keyOf, sizeof e->keyOf, err, errsz, ln)) return -1;
      }
      int nfib = 1, nvals = 0;
      for (int j = ni + 1; j < nw; j++) strcmp(words[j], "|") == 0 ? nfib++ : nvals++;
      e->fibOff = (int *)arena_alloc(a, (size_t)nfib * sizeof *e->fibOff);
      e->fibLen = (int *)arena_alloc(a, (size_t)nfib * sizeof *e->fibLen);
      e->fibVals = (double *)arena_alloc(a, (size_t)(nvals ? nvals : 1) * sizeof *e->fibVals);
      int fib = 0, vi = 0;
      e->fibOff[0] = 0;
      for (int j = ni + 1; j < nw; j++) {
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
      RegEntry *rel = find_ent(reg, words[2]);
      if (!rel || rel->kind != RK_REL)
        return rerr(err, errsz, ln, "inv: no functional rel '%s'", words[2]);
      RegEntry *e = &reg->ents[reg->nents++];
      e->kind = RK_SREL;
      if (wfree(words[1], err, errsz, ln)) return -1;
      if (wname(words[1], e->name, sizeof e->name, err, errsz, ln)) return -1;
      if (wuniq(reg, e, err, errsz, ln)) return -1;
      if (wname(words[2], e->invOf, sizeof e->invOf, err, errsz, ln)) return -1;
      /* fiber for target t: ascending source ids with rel==key(t); nfib = world n.
       * The inverse of a keyed rel is keyed automatically: targets match through the key
       * column and fiber values are the sources' keys, so the whole fiber stays in key space. */
      const RegEntry *kc = NULL;
      if (rel->keyOf[0]) {
        kc = find_ent(reg, rel->keyOf);
        if (!kc) return rerr(err, errsz, ln, "inv %s: key column '%s' missing", e->name, rel->keyOf);
        if (wname(rel->keyOf, e->keyOf, sizeof e->keyOf, err, errsz, ln)) return -1;
      }
      int n = reg->n;
      e->fibOff = (int *)arena_alloc(a, (size_t)(n ? n : 1) * sizeof *e->fibOff);
      e->fibLen = (int *)arena_alloc(a, (size_t)(n ? n : 1) * sizeof *e->fibLen);
      e->fibVals = (double *)arena_alloc(a, (size_t)(n ? n : 1) * sizeof *e->fibVals);
      int vi = 0;
      for (int t = 0; t < n; t++) {
        e->fibOff[t] = vi;
        double tk = kc ? kc->nums[t] : (double)t;
        for (int s = 0; s < rel->nnums; s++)
          if (rel->nums[s] == tk) e->fibVals[vi++] = kc ? kc->nums[s] : (double)s;
        e->fibLen[t] = vi - e->fibOff[t];
      }
      e->nfib = n;

    } else if (strcmp(k, "bind") == 0) {
      if (nw < 3) return rerr(err, errsz, ln, "usage: bind <name> <kind> <values>");
      RegEntry *e = &reg->ents[reg->nents++];
      e->kind = RK_BIND;
      if (wfree(words[1], err, errsz, ln)) return -1;
      if (wname(words[1], e->name, sizeof e->name, err, errsz, ln)) return -1;
      if (wuniq(reg, e, err, errsz, ln)) return -1;
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
      if (wuniq(reg, e, err, errsz, ln)) return -1;
      if (raw2) store_string(a, e, raw2); /* verbatim BQN body convention */

    } else if (strcmp(k, "ja") == 0 || (strcmp(k, "as") == 0 && nw == 3)) {
      /* the name alias: one hop, no transitivity, outranked by real entries; `ja` and
       * 2-arity `as` fill one table — the ja spelling documents the JA surface */
      if (nw != 3) return rerr(err, errsz, ln, "usage: ja <word> <name>");
      if (wfree(words[1], err, errsz, ln)) return -1;
      if (wuniq_alias(reg, words[1], err, errsz, ln)) return -1;
      if (wname(words[1], reg->asFrom[reg->nas], ANO_NAMESZ, err, errsz, ln)) return -1;
      if (wname(words[2], reg->asTo[reg->nas], ANO_NAMESZ, err, errsz, ln)) return -1;
      reg->asJa[reg->nas] = (k[0] == 'j');
      reg->nas++;

    } else if (strcmp(k, "as") == 0) {
      /* the derived tag: `as <word> <col> <value>` — the word names the equality mask
       * over the live column, recomputed at each use (DATAMODEL.md). The word is an
       * entry name and folds; the value is a value and never does. */
      if (nw != 4) return rerr(err, errsz, ln, "usage: as <word> <name> | as <word> <col> <value>");
      RegEntry *c = find_ent(reg, words[2]);
      if (!c || (c->kind != RK_COL && c->kind != RK_FIELD))
        return rerr(err, errsz, ln, "as: no column '%s'", words[2]);
      if (c->type == CT_CHAR)
        return rerr(err, errsz, ln, "as: derived tag over a char column is unsupported");
      int crows = c->kind == RK_FIELD ? reg->latW * reg->latH : reg->n;
      if (c->type == CT_NUM && crows > 0 && c->nnums == 2 * crows)
        return rerr(err, errsz, ln, "as: derived tag over a vec column is unsupported");
      RegEntry *e = &reg->ents[reg->nents++];
      e->kind = RK_TAG;
      if (wfree(words[1], err, errsz, ln)) return -1;
      if (wname(words[1], e->name, sizeof e->name, err, errsz, ln)) return -1;
      if (wuniq(reg, e, err, errsz, ln)) return -1;
      if (wname(words[2], e->tagCol, sizeof e->tagCol, err, errsz, ln)) return -1;
      e->type = c->type;
      if (c->type == CT_SYM) {
        e->syms = (char (*)[ANO_NAMESZ])arena_alloc(a, ANO_NAMESZ);
        if (wname(words[3], e->syms[0], ANO_NAMESZ, err, errsz, ln)) return -1;
        e->nsyms = 1;
      } else {
        if (wnums(words, 3, nw, 1, a, &e->nums, &e->nnums, err, errsz, ln)) return -1;
      }

    } else if (strcmp(k, "role") == 0) {
      /* point a system role at a native column; the emitter routes spawn machinery
       * through reg_role, so `role pos 位置` gives a kanji column the `pos` behaviour. */
      if (nw != 3) return rerr(err, errsz, ln, "usage: role <role> <col>");
      static const char *known[] = { "keys", "id", "parent", "proto", "pos" };
      int ok = 0;
      for (size_t r = 0; r < sizeof known / sizeof *known; r++) ok |= !strcmp(words[1], known[r]);
      if (!ok) return rerr(err, errsz, ln, "unknown role '%s' (keys id parent proto pos)", words[1]);
      RegEntry *c = find_ent(reg, words[2]);
      if (!c || c->kind != RK_COL) return rerr(err, errsz, ln, "role: no column '%s'", words[2]);
      if (reg->nroles >= ANO_NROLES) return rerr(err, errsz, ln, "too many roles");
      if (wname(words[1], reg->roleName[reg->nroles], ANO_NAMESZ, err, errsz, ln)) return -1;
      if (wname(words[2], reg->roleCol[reg->nroles], ANO_NAMESZ, err, errsz, ln)) return -1;
      reg->nroles++;

    } else if (strcmp(k, "def") == 0) {
      /* the proto, a registered archetype: named field=value pairs, the registry's first
       * row-oriented construct. Spawn fill layer one; layers two and three are `default`
       * and the type zero. Vocabulary reuse across the registry/program boundary is ruled
       * fine (2026-07-11): this `def` never meets the program's. */
      if (nw < 2) return rerr(err, errsz, ln, "usage: def <name> [<col>=<v> ...]");
      RegEntry *e = &reg->ents[reg->nents++];
      e->kind = RK_PROTO;
      if (wfree(words[1], err, errsz, ln)) return -1;
      if (wname(words[1], e->name, sizeof e->name, err, errsz, ln)) return -1;
      if (wuniq(reg, e, err, errsz, ln)) return -1;
      int nf = nw - 2;
      e->syms = (char (*)[ANO_NAMESZ])arena_alloc(a, (size_t)(nf ? 2 * nf : 1) * ANO_NAMESZ);
      e->nums = (double *)arena_alloc(a, (size_t)(nf ? nf : 1) * sizeof *e->nums);
      for (int j = 0; j < nf; j++) {
        char *eq = strchr(words[2 + j], '=');
        if (!eq || eq == words[2 + j] || !eq[1])
          return rerr(err, errsz, ln, "def %s: field '%s' is not <col>=<v>", e->name, words[2 + j]);
        *eq = 0;
        RegEntry *c = find_ent(reg, words[2 + j]);
        if (!c || (c->kind != RK_COL && c->kind != RK_REL))
          return rerr(err, errsz, ln, "def %s: no column '%s'", e->name, words[2 + j]);
        if (c->uniq)
          return rerr(err, errsz, ln, "def %s: unique column '%s' is minted, not defaulted", e->name, c->name);
        /* the id/keys role column mints at spawn even when merely magic-named — a proto
         * field on it would be silently outranked by the mint */
        if (c == reg_role(reg, "keys") || c == reg_role(reg, "id"))
          return rerr(err, errsz, ln, "def %s: key column '%s' is minted, not defaulted", e->name, c->name);
        int crows = reg->n;
        if (c->kind == RK_COL && c->type == CT_NUM && crows > 0 && c->nnums == 2 * crows)
          return rerr(err, errsz, ln, "def %s: proto field over vec column '%s' unsupported", e->name, c->name);
        if (c->kind == RK_COL && c->type == CT_CHAR)
          return rerr(err, errsz, ln, "def %s: proto field over char column '%s' unsupported", e->name, c->name);
        e->nums[j] = 0;
        if (c->kind == RK_REL || c->type != CT_SYM) {
          if (wnum(eq + 1, &e->nums[j]))
            return rerr(err, errsz, ln, "def %s: bad number '%s' for '%s'", e->name, eq + 1, c->name);
        }
        if (wname(c->name, e->syms[2 * j], ANO_NAMESZ, err, errsz, ln)) return -1;
        if (wname(eq + 1, e->syms[2 * j + 1], ANO_NAMESZ, err, errsz, ln)) return -1;
        e->nsyms = 2 * (j + 1);
      }

    } else if (strcmp(k, "reap") == 0) {
      /* ~ reclamation policy, storage only (ruled 2026-07-11): the gen bump plus presence
       * clear is the mark, reuse at tick seal is the reap; `host` hands reclamation to the
       * host. anoc compacts at the barrier either way — the emitted algebra never changes. */
      if (nw != 2 || (strcmp(words[1], "seal") && strcmp(words[1], "host")))
        return rerr(err, errsz, ln, "usage: reap <seal|host>");
      if (reg->reap[0]) return rerr(err, errsz, ln, "reap redeclared");
      if (wname(words[1], reg->reap, sizeof reg->reap, err, errsz, ln)) return -1;

    } else {
      return rerr(err, errsz, ln, "unknown kind '%s'", k);
    }
  }
  return 0;
}

/* Inputs: registry, surface name. Output: entry or NULL. Entry names first, then the
 * alias hop, every compare under names_eq — a pure name alias, one hop, outranked by
 * real entries, identical from either surface. Load rejects fold collisions within
 * each name kind, so resolution order never depends on declaration order. */
const RegEntry *reg_find(const Registry *reg, const char *name) {
  for (int i = 0; i < reg->nents; i++)
    if (names_eq(reg->ents[i].name, name)) return &reg->ents[i];
  for (int i = 0; i < reg->nas; i++)
    if (names_eq(reg->asFrom[i], name)) {
      for (int j = 0; j < reg->nents; j++)
        if (names_eq(reg->ents[j].name, reg->asTo[i])) return &reg->ents[j];
      return NULL;
    }
  return NULL;
}

/* Inputs: registry. Output: the first declared unique column, else NULL. */
static const RegEntry *uniq_col(const Registry *reg) {
  for (int i = 0; i < reg->nents; i++)
    if (reg->ents[i].kind == RK_COL && reg->ents[i].uniq) return &reg->ents[i];
  return NULL;
}

/* Inputs: registry, a system role (keys id parent proto pos). Output: the declared
 * role column when one is set; for id/keys next the first declared unique column
 * (declared beats guess — a declared unique outranks a merely magic-named id); else
 * reg_find on the literal role name — one resolver, entries then aliases, so every
 * world routes through one path — else NULL. */
const RegEntry *reg_role(const Registry *reg, const char *role) {
  for (int i = 0; i < reg->nroles; i++)
    if (names_eq(reg->roleName[i], role))
      for (int j = 0; j < reg->nents; j++)
        if (names_eq(reg->ents[j].name, reg->roleCol[i])) return &reg->ents[j];
  if (names_eq(role, "id") || names_eq(role, "keys")) {
    const RegEntry *u = uniq_col(reg);
    if (u) return u;
  }
  return reg_find(reg, role);
}

/* Inputs: string buffer, double. Output: a spelling strtod parses back bit-exact —
 * format∘parse∘format = format, so dump -> load -> dump fixpoints. Integers in the
 * exact range spell as plain digits (900, never 9e+02 — the saved world is read by
 * people); everything else takes the shortest round-tripping %g. */
static void dnum(StrBuf *b, double v) {
  char buf[64];
  /* range guard before the cast (UB out of range); -0.0 keeps its sign bit */
  if (v >= -9e15 && v <= 9e15 && v == (long long)v && !(v == 0 && signbit(v))) {
    sb_printf(b, "%lld", (long long)v);
    return;
  }
  for (int p = 1; p <= 17; p++) {
    snprintf(buf, sizeof buf, "%.*g", p, v);
    if (strtod(buf, NULL) == v) break;
  }
  sb_printf(b, "%s", buf);
}

/* Inputs: loaded registry, target path, err buffer. Output: 0 / -1 with err set.
 * The write-out half of the commit loop: read in -> binary tables in memory -> write
 * out staged -> mv commit. Serializes everything reg_load reads — n, lattice, entries
 * in declaration order with pres/default beside their column, inv by its rel (fibers
 * recompute at load), roles, then the alias table with its declared spellings. The
 * world is a column store, so a save is a registry dump; comments and layout are
 * authoring-time only and a dump erases them. */
int reg_dump(const Registry *reg, const char *path, char *err, size_t errsz) {
  StrBuf b = {0};
  sb_printf(&b, "n %d\n", reg->n);
  if (reg->latW || reg->latH) sb_printf(&b, "lattice %d %d\n", reg->latW, reg->latH);
  if (reg->reap[0]) sb_printf(&b, "reap %s\n", reg->reap);
  for (int i = 0; i < reg->nents; i++) {
    const RegEntry *e = &reg->ents[i];
    switch (e->kind) {
      case RK_COL: case RK_FIELD: {
        int rows = e->kind == RK_FIELD ? reg->latW * reg->latH : reg->n;
        const char *kw = e->kind == RK_FIELD ? "field" : "col";
        if (e->uniq) {
          sb_printf(&b, "unique %s", e->name);
          for (int j = 0; j < e->nnums; j++) { sb_printf(&b, " "); dnum(&b, e->nums[j]); }
          sb_printf(&b, "\n");
          break;
        }
        if (e->type == CT_SYM) {
          sb_printf(&b, "%s %s sym", kw, e->name);
          for (int j = 0; j < e->nsyms; j++) sb_printf(&b, " %s", e->syms[j]);
        } else if (e->type == CT_CHAR) {
          sb_printf(&b, "%s %s char %s", kw, e->name, e->syms ? e->syms[0] : "");
        } else if (rows > 0 && e->nnums == 2 * rows) {   /* pair-column convention */
          sb_printf(&b, "%s %s vec", kw, e->name);
          for (int j = 0; j < rows; j++) {
            if (j) sb_printf(&b, " |");
            sb_printf(&b, " "); dnum(&b, e->nums[2*j]);
            sb_printf(&b, " "); dnum(&b, e->nums[2*j+1]);
          }
        } else {
          sb_printf(&b, "%s %s %s", kw, e->name, e->type == CT_BOOL ? "bool" : "num");
          for (int j = 0; j < e->nnums; j++) { sb_printf(&b, " "); dnum(&b, e->nums[j]); }
        }
        sb_printf(&b, "\n");
        if (e->hasPres) {
          sb_printf(&b, "pres %s", e->name);
          for (int j = 0; j < reg->n; j++) { sb_printf(&b, " "); dnum(&b, e->pres[j]); }
          sb_printf(&b, "\n");
        }
        break;
      }
      case RK_REL: case RK_ALIAS: {
        if (e->kind == RK_REL && e->keyOf[0]) sb_printf(&b, "rel %s %s", e->keyOf, e->name);
        else sb_printf(&b, "%s %s", e->kind == RK_REL ? "rel" : "alias", e->name);
        for (int j = 0; j < e->nnums; j++) { sb_printf(&b, " "); dnum(&b, e->nums[j]); }
        sb_printf(&b, "\n");
        break;
      }
      case RK_SREL: {
        if (e->invOf[0]) { sb_printf(&b, "inv %s %s\n", e->name, e->invOf); break; }
        if (e->keyOf[0]) sb_printf(&b, "srel %s %s", e->keyOf, e->name);
        else sb_printf(&b, "srel %s", e->name);
        for (int f = 0; f < e->nfib; f++) {
          if (f) sb_printf(&b, " |");
          for (int j = 0; j < e->fibLen[f]; j++) {
            sb_printf(&b, " "); dnum(&b, e->fibVals[e->fibOff[f] + j]);
          }
        }
        sb_printf(&b, "\n");
        break;
      }
      case RK_BIND: {
        sb_printf(&b, "bind %s %s", e->name, e->bindKind);
        for (int j = 0; j < e->nnums; j++) { sb_printf(&b, " "); dnum(&b, e->nums[j]); }
        sb_printf(&b, "\n");
        break;
      }
      case RK_FN: {
        if (e->nsyms) sb_printf(&b, "fn %s %s\n", e->name, e->syms[0]);
        else sb_printf(&b, "fn %s\n", e->name);
        break;
      }
      case RK_TAG: {
        sb_printf(&b, "as %s %s ", e->name, e->tagCol);
        if (e->type == CT_SYM) sb_printf(&b, "%s", e->syms[0]);
        else dnum(&b, e->nums[0]);
        sb_printf(&b, "\n");
        break;
      }
      case RK_PROTO: {
        sb_printf(&b, "def %s", e->name);
        for (int j = 0; j < e->nsyms / 2; j++)
          sb_printf(&b, " %s=%s", e->syms[2 * j], e->syms[2 * j + 1]);
        sb_printf(&b, "\n");
        break;
      }
    }
    if (e->defval != 0) {
      sb_printf(&b, "default %s ", e->name);
      dnum(&b, e->defval);
      sb_printf(&b, "\n");
    }
  }
  for (int i = 0; i < reg->nroles; i++)
    sb_printf(&b, "role %s %s\n", reg->roleName[i], reg->roleCol[i]);
  for (int i = 0; i < reg->nas; i++)
    sb_printf(&b, "%s %s %s\n", reg->asJa[i] ? "ja" : "as", reg->asFrom[i], reg->asTo[i]);
  int rc = fs_write_commit(path, b.s ? b.s : "", b.len);
  sb_free(&b);
  if (rc) return rerr(err, errsz, 0, "cannot write '%s': %s", path, strerror(errno));
  return 0;
}
