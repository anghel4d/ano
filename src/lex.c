/* lex.c — anoc tokenizer: the ASCII surface and the Japanese spaced skin (--! ja).
 * Contract: ano.h (Toks, ano_lex), GRAMMAR.md "Lexical". The stream is columnar
 * (kind/name/num/line grown together) and token text is interned — one canonical
 * copy per spelling, no per-token buffers. Source is validated as strict UTF-8 once
 * at the ano_lex boundary (overlongs, surrogates, and out-of-range rejected), so
 * both skins may decode without checking. ASCII mode fuses fold/scan tokens;
 * ^ begins the alias sigil, : a symbol, @ is always the scope operator. JA mode
 * lexes space-separated words — registry ja aliases, the particle table, a kanji
 * numeral reader — then normalizes per demos/9-nihongo/40-tokenizer-skin.bqn: drop
 * the に TGT marker and re-root each postfix operator before its operand. The
 * postfix flag lives beside the token columns and never escapes this file. */
#include "ano.h"

/* internal に target marker; deleted in normalization, never returned */
#define K_TGT ((TokKind)T_KINDCOUNT)

/* char classes, ASCII only, locale-free */
static int nstart(int c) { return (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z'); }
static int nchar(int c)  { return nstart(c) || (c >= '0' && c <= '9') || c == '_'; }
static int dig(int c)    { return c >= '0' && c <= '9'; }

static int ucp(const unsigned char *s, size_t n, unsigned *cp);

/* growing token columns; post[] marks JA postfix operators, internal only */
typedef struct {
  TokKind *kind; const char **name; double *num; int *line;
  unsigned char *post;
  int n, cap;
  Arena *a; Intern *it;
} TokBuf;

/* Inputs: buffer, kind, line. Output: index of the appended token (name "", num 0).
 * Invariant: all five columns grow together; old blocks stay in the arena. */
static int tb_push(TokBuf *b, TokKind k, int line) {
  if (b->n == b->cap) {
    int cap = b->cap ? b->cap * 2 : 128;
    TokKind *nk = (TokKind *)arena_alloc(b->a, (size_t)cap * sizeof *nk);
    const char **nn = (const char **)arena_alloc(b->a, (size_t)cap * sizeof *nn);
    double *nv = (double *)arena_alloc(b->a, (size_t)cap * sizeof *nv);
    int *nl = (int *)arena_alloc(b->a, (size_t)cap * sizeof *nl);
    unsigned char *np = (unsigned char *)arena_alloc(b->a, (size_t)cap);
    if (b->n) {
      memcpy(nk, b->kind, (size_t)b->n * sizeof *nk);
      memcpy(nn, b->name, (size_t)b->n * sizeof *nn);
      memcpy(nv, b->num, (size_t)b->n * sizeof *nv);
      memcpy(nl, b->line, (size_t)b->n * sizeof *nl);
      memcpy(np, b->post, (size_t)b->n);
    }
    b->kind = nk; b->name = nn; b->num = nv; b->line = nl; b->post = np; b->cap = cap;
  }
  int ix = b->n++;
  b->kind[ix] = k; b->name[ix] = ""; b->num[ix] = 0; b->line[ix] = line; b->post[ix] = 0;
  return ix;
}

/* Inputs: err buffer, line, printf format. Output: -1; err = "line N: ...". */
static int lex_err(char *err, size_t errsz, int line, const char *fmt, ...) {
  char msg[ANO_ERRSZ];
  va_list ap; va_start(ap, fmt);
  vsnprintf(msg, sizeof msg, fmt, ap);
  va_end(ap);
  if (err && errsz) snprintf(err, errsz, "line %d: %s", line, msg);
  return -1;
}

/* Inputs: a lexed name. Output: its keyword kind, or 0 (T_EOF) when not one. */
static TokKind kwkind(const char *nm) {
  static const struct { const char *w; TokKind k; } tab[] = {
    {"def",T_DEF},{"spawn",T_SPAWN},{"at",T_ATKW},{"to",T_TO},{"via",T_VIA},
    {"along",T_ALONG},{"order",T_ORDER},{"by",T_BY},{"take",T_TAKE},{"desc",T_DESC},
    {"top",T_TOP},{"grade",T_GRADE},{"reduce",T_REDUCE},{"scan",T_SCANKW},
    {"scan2",T_SCAN2},{"cross",T_CROSS},{"expand",T_EXPAND},
  };
  for (size_t k = 0; k < sizeof tab / sizeof *tab; k++)
    if (!strcmp(nm, tab[k].w)) return tab[k].k;
  return (TokKind)0;
}

/* Inputs: source (directives already blanked), token buffer, err. Output: 0/-1;
 * tokens appended, T_NL between nonempty lines, no trailing NL and no EOF.
 * Invariants: folds/scans fused with no interior whitespace; NAME+'/' folds only
 * for max/min/avg and never before '='; ^ begins the alias sigil, @ is always T_AT. */
static int lex_ascii(const char *src, TokBuf *b, char *err, size_t errsz) {
  int line = 1;
  size_t i = 0, n = strlen(src);
  while (src[i]) {
    unsigned char c = (unsigned char)src[i];
    if (c == '\n') {
      if (b->n && b->kind[b->n - 1] != T_NL) tb_push(b, T_NL, line);
      line++; i++; continue;
    }
    if (c == ' ' || c == '\t' || c == '\r') { i++; continue; }
    if (c == '-' && src[i + 1] == '-') { while (src[i] && src[i] != '\n') i++; continue; }
    if (nstart(c)) {
      size_t j = i + 1;
      while (nchar((unsigned char)src[j])) j++;
      const char *nm = intern(b->it, b->a, src + i, j - i);
      /* reducer fold: max/ min/ avg/ — no whitespace, and 'max/= 2' stays SLASHEQ */
      int red = !strcmp(nm, "max") || !strcmp(nm, "min") || !strcmp(nm, "avg");
      if (red && src[j] == '/' && src[j + 1] != '=') {
        { int ix = tb_push(b, T_FOLD, line); b->name[ix] = nm; } i = j + 1; continue;
      }
      if (!strcmp(nm, "max") && src[j] == '\\') {
        { int ix = tb_push(b, T_SCANOP, line); b->name[ix] = nm; } i = j + 1; continue;
      }
      TokKind kk = kwkind(nm);
      if (kk) { tb_push(b, kk, line); i = j; continue; }
      { int ix = tb_push(b, T_NAME, line); b->name[ix] = nm; }
      if (src[j] == '\'') { tb_push(b, T_TICK, line); j++; }   /* postfix tick */
      i = j; continue;
    }
    if (dig(c)) {
      size_t j = i;
      while (dig((unsigned char)src[j])) j++;
      if (src[j] == '.' && dig((unsigned char)src[j + 1])) {
        j++; while (dig((unsigned char)src[j])) j++;
      }
      char nb[64]; size_t len = j - i;
      if (len >= sizeof nb) return lex_err(err, errsz, line, "number too long");
      memcpy(nb, src + i, len); nb[len] = 0;
      double v = strtod(nb, NULL);
      if (nstart((unsigned char)src[j])) {                     /* counter: 3mo */
        size_t k = j + 1;
        while (nchar((unsigned char)src[k])) k++;
        int ix = tb_push(b, T_COUNTER, line);
        b->num[ix] = v; b->name[ix] = intern(b->it, b->a, src + j, k - j);
        i = k;
      } else {
        { int ix = tb_push(b, T_NUM, line); b->num[ix] = v; }
        i = j;
      }
      continue;
    }
    if (c == '"') {
      size_t j = i + 1;
      while (src[j] && src[j] != '"' && src[j] != '\n') j++;
      if (src[j] != '"') return lex_err(err, errsz, line, "unterminated string");
      { int ix = tb_push(b, T_STR, line); b->name[ix] = intern(b->it, b->a, src + i + 1, j - i - 1); }
      i = j + 1; continue;
    }
    if (c == ':') {
      if (!nstart((unsigned char)src[i + 1])) return lex_err(err, errsz, line, "':' needs a name: symbols are :Name");
      size_t j = i + 2;
      while (nchar((unsigned char)src[j])) j++;
      { int ix = tb_push(b, T_SYM, line); b->name[ix] = intern(b->it, b->a, src + i + 1, j - i - 1); }
      i = j; continue;
    }
    if (c == '_') {
      if (nchar((unsigned char)src[i + 1])) return lex_err(err, errsz, line, "names cannot start with '_'");
      tb_push(b, T_WILD, line); i++; continue;
    }
    if (c >= 0x80) {                                           /* non-ASCII: ↕ only */
      unsigned cp;
      int l = ucp((const unsigned char *)src + i, n - i, &cp);
      if (!l) return lex_err(err, errsz, line, "malformed UTF-8");
      if (cp == 0x2195) { tb_push(b, T_IOTA, line); i += (size_t)l; continue; }
      return lex_err(err, errsz, line, "unknown character U+%04X", cp);
    }
    unsigned char d = (unsigned char)src[i + 1];
    switch (c) {
      case ',': tb_push(b, T_COMMA, line); i++; break;
      case ';': tb_push(b, T_SEMI, line); i++; break;
      case '(': tb_push(b, T_LP, line); i++; break;
      case ')': tb_push(b, T_RP, line); i++; break;
      case '[': tb_push(b, T_LB, line); i++; break;
      case ']': tb_push(b, T_RB, line); i++; break;
      case '~': tb_push(b, T_TILDE, line); i++; break;
      case '.': tb_push(b, T_DOT, line); i++; break;
      case '%': tb_push(b, T_PCT, line); i++; break;
      case '=':
        if (d == '>') { tb_push(b, T_ARROW, line); i += 2; }
        else if (d == '=') { tb_push(b, T_EQEQ, line); i += 2; }
        else { tb_push(b, T_EQ, line); i++; }
        break;
      case '!':
        if (d == '=') { tb_push(b, T_NE, line); i += 2; } else { tb_push(b, T_BANG, line); i++; }
        break;
      case '<':
        if (d == '=') { tb_push(b, T_LE, line); i += 2; }
        else if (d == '-') { tb_push(b, T_LARROW, line); i += 2; }
        else { tb_push(b, T_LT, line); i++; }
        break;
      case '>':
        if (d == '=') { tb_push(b, T_GE, line); i += 2; } else { tb_push(b, T_GT, line); i++; }
        break;
      case '|':
        if (d == '>') { tb_push(b, T_PIPEGT, line); i += 2; }
        else if (d == '/') { { int ix = tb_push(b, T_FOLD, line); b->name[ix] = "|"; } i += 2; }
        else { tb_push(b, T_BAR, line); i++; }
        break;
      case '&':
        if (d == '/') { { int ix = tb_push(b, T_FOLD, line); b->name[ix] = "&"; } i += 2; }
        else { tb_push(b, T_AMP, line); i++; }
        break;
      case '+':
        if (d == '=') { tb_push(b, T_PLUSEQ, line); i += 2; }
        else if (d == '/') { { int ix = tb_push(b, T_FOLD, line); b->name[ix] = "+"; } i += 2; }
        else if (d == '\\') { { int ix = tb_push(b, T_SCANOP, line); b->name[ix] = "+"; } i += 2; }
        else { tb_push(b, T_PLUS, line); i++; }
        break;
      case '-':
        if (d == '=') { tb_push(b, T_MINUSEQ, line); i += 2; } else { tb_push(b, T_MINUS, line); i++; }
        break;
      case '*':
        if (d == '=') { tb_push(b, T_STAREQ, line); i += 2; }
        else if (d == '/') { { int ix = tb_push(b, T_FOLD, line); b->name[ix] = "*"; } i += 2; }
        else if (d == '\\') { { int ix = tb_push(b, T_SCANOP, line); b->name[ix] = "*"; } i += 2; }
        else { tb_push(b, T_STAR, line); i++; }
        break;
      case '/':
        if (d == '=') { tb_push(b, T_SLASHEQ, line); i += 2; } else { tb_push(b, T_SLASH, line); i++; }
        break;
      case '#':
        if (d == '/') { { int ix = tb_push(b, T_FOLD, line); b->name[ix] = "#"; } i += 2; }
        else return lex_err(err, errsz, line, "'#' begins only the fold '#/'");
        break;
      case '@': tb_push(b, T_AT, line); i++; break;
      case '^':
        if (nstart(d)) {                                       /* ^alias sigil, the deictic pronoun */
          size_t j = i + 2;
          while (nchar((unsigned char)src[j])) j++;
          { int ix = tb_push(b, T_ALIAS, line); b->name[ix] = intern(b->it, b->a, src + i + 1, j - i - 1); }
          i = j;
        } else return lex_err(err, errsz, line, "'^' begins only the ^alias sigil");
        break;
      case '\'': return lex_err(err, errsz, line, "stray tick: ' is postfix on a name");
      case '\\': return lex_err(err, errsz, line, "stray '\\': scans are +\\ *\\ max\\");
      default:   return lex_err(err, errsz, line, "unknown byte 0x%02X", c);
    }
  }
  return 0;
}

/* Inputs: s at a UTF-8 char boundary, n bytes available. Outputs: *cp. Output: byte
 * length 1-4, 0 malformed. Strict (anoptic utf8_decode): rejects overlongs, encoded
 * surrogates, cp > U+10FFFF, and truncation — decoder and validator in one. */
static int ucp(const unsigned char *s, size_t n, unsigned *cp) {
  unsigned char b0 = s[0];
  if (b0 < 0x80) { *cp = b0; return 1; }
  int need; unsigned r, min;
  if      ((b0 & 0xE0) == 0xC0) { need = 1; r = b0 & 0x1F; min = 0x80; }
  else if ((b0 & 0xF0) == 0xE0) { need = 2; r = b0 & 0x0F; min = 0x800; }
  else if ((b0 & 0xF8) == 0xF0) { need = 3; r = b0 & 0x07; min = 0x10000; }
  else return 0;                                     /* continuation or F8-FF lead */
  if (n - 1 < (size_t)need) return 0;                /* truncated at end */
  for (int k = 1; k <= need; k++) {
    if ((s[k] & 0xC0) != 0x80) return 0;
    r = (r << 6) | (s[k] & 0x3F);
  }
  if (r < min || r > 0x10FFFF || (r >= 0xD800 && r <= 0xDFFF)) return 0;
  *cp = r;
  return 1 + need;
}

/* Inputs: codepoint. Output: digit value 0-9 — kanji 〇一..九 or fullwidth ０-９ — or -1. */
static int jadig(unsigned cp) {
  if (cp >= 0xFF10 && cp <= 0xFF19) return (int)(cp - 0xFF10);
  switch (cp) {
    case 0x3007: return 0; case 0x4E00: return 1; case 0x4E8C: return 2;
    case 0x4E09: return 3; case 0x56DB: return 4; case 0x4E94: return 5;
    case 0x516D: return 6; case 0x4E03: return 7; case 0x516B: return 8;
    case 0x4E5D: return 9; default: return -1;
  }
}

/* Inputs: codepoint. Output: magnitude 10/100/1000/10000, or 0. */
static int jamag(unsigned cp) {
  switch (cp) {
    case 0x5341: return 10; case 0x767E: return 100;
    case 0x5343: return 1000; case 0x4E07: return 10000;
    default: return 0;
  }
}

/* Inputs: w, a NUL-terminated word. Outputs: *val; unit (>= 4 bytes, "" or "mo").
 * Output: 1 when w is a numeral — Arabic digits, kanji named magnitudes (六十,
 * 九千九百九十九, bare 千 = 1000), digit-string decimal with 〇 and ・ (一・〇五),
 * optional ヶ月 counter suffix (三ヶ月 = 3 "mo") — else 0, outputs untouched. */
static int ja_numeral(const char *w, double *val, char *unit) {
  char buf[128];
  size_t len = strlen(w);
  if (len == 0 || len >= sizeof buf) return 0;
  memcpy(buf, w, len + 1);
  const char *u = "";
  if (len > 6 && !memcmp(buf + len - 6, "\xE3\x83\xB6\xE6\x9C\x88", 6)) {   /* ヶ月 */
    len -= 6; buf[len] = 0; u = "mo";
  }
  if (dig((unsigned char)buf[0])) {                            /* Arabic digits */
    size_t p = 0;
    while (dig((unsigned char)buf[p])) p++;
    if (buf[p] == '.') {
      p++;
      if (!dig((unsigned char)buf[p])) return 0;
      while (dig((unsigned char)buf[p])) p++;
    }
    if (buf[p]) return 0;
    *val = strtod(buf, NULL); strcpy(unit, u); return 1;
  }
  /* decode and classify: ty 0 = digit, 1 = magnitude, 2 = ・ */
  int dv[32], ty[32], n = 0;
  size_t p = 0;
  while (buf[p]) {
    unsigned cp;
    int l = ucp((const unsigned char *)buf + p, len - p, &cp);
    if (!l || n >= 32) return 0;
    p += (size_t)l;
    int d = jadig(cp), m = jamag(cp);
    if (d >= 0) { ty[n] = 0; dv[n] = d; }
    else if (m) { ty[n] = 1; dv[n] = m; }
    else if (cp == 0x30FB) { ty[n] = 2; dv[n] = 0; }
    else return 0;
    n++;
  }
  int hasdot = 0, hasmag = 0;
  for (int k = 0; k < n; k++) { hasdot |= ty[k] == 2; hasmag |= ty[k] == 1; }
  double v = 0;
  if (hasdot) {                                                /* 一・〇五 = 1.05 */
    if (hasmag || ty[0] == 2) return 0;
    int k = 0;
    for (; k < n && ty[k] == 0; k++) v = v * 10 + dv[k];
    if (k >= n || ty[k] != 2 || k + 1 >= n) return 0;
    double sc = 0.1;
    for (k++; k < n; k++) {
      if (ty[k] != 0) return 0;
      v += dv[k] * sc; sc /= 10;
    }
  } else if (hasmag) {                                         /* 九千九百九十九 = 9999 */
    double sect = 0, cur = 0;
    int curset = 0;
    for (int k = 0; k < n; k++) {
      if (ty[k] == 0) { cur = dv[k]; curset = 1; }
      else if (dv[k] == 10000) {
        sect += cur;
        v += (sect > 0 ? sect : 1) * 10000;
        sect = 0; cur = 0; curset = 0;
      } else {
        sect += (curset ? cur : 1) * dv[k];
        cur = 0; curset = 0;
      }
    }
    v += sect + cur;
  } else {                                                     /* positional: 三 = 3 */
    for (int k = 0; k < n; k++) v = v * 10 + dv[k];
  }
  *val = v; strcpy(unit, u); return 1;
}

/* particle/verb table; post marks operators the surface puts after their operand */
static const struct { const char *w; TokKind k; int post; } jatab[] = {
  {"と", T_AMP, 0},   {"か", T_BAR, 0},     {"の", T_DOT, 0},     {"で", T_AT, 1},
  {"、", T_COMMA, 0}, {"が", T_COMMA, 0},   {"は", T_COMMA, 0},
  {"より", T_GT, 1},  {"超", T_GT, 1},      {"未満", T_LT, 1},    {"同", T_EQEQ, 1},
  {"たす", T_PLUSEQ, 1}, {"ひく", T_MINUSEQ, 1}, {"かける", T_STAREQ, 1},
  {"わる", T_SLASHEQ, 1}, {"にする", T_EQ, 1},
  {"に", K_TGT, 0},
};

/* Inputs: source, registry (ja aliases), token buffer, err. Output: 0/-1; the
 * normalized ASCII-equivalent stream, T_NL between nonempty lines, no trailing NL.
 * Per word, in order: registry ja alias -> T_NAME (canonical name); particle table;
 * kanji/Arabic numeral; else error. Then ex40 normalization: delete K_TGT, swap each
 * postfix operator with the token before it. Invariant: K_TGT and post flags never
 * survive this function. */
static int lex_ja(const char *src, const Registry *reg, TokBuf *b, char *err, size_t errsz) {
  int line = 1;
  size_t i = 0, n = strlen(src);
  while (src[i]) {
    unsigned char c = (unsigned char)src[i];
    if (c == '\n') {
      if (b->n && b->kind[b->n - 1] != T_NL) tb_push(b, T_NL, line);
      line++; i++; continue;
    }
    if (c == ' ' || c == '\t' || c == '\r') { i++; continue; }
    if (c >= 0x80) {
      unsigned cp;
      int l = ucp((const unsigned char *)src + i, n - i, &cp);
      if (!l) return lex_err(err, errsz, line, "malformed UTF-8");
      if (cp == 0x3000) { i += (size_t)l; continue; }          /* ideographic space */
    }
    if (c == '-' && src[i + 1] == '-') { while (src[i] && src[i] != '\n') i++; continue; }
    /* word: run to the next space/newline (ASCII or U+3000) */
    size_t j = i;
    while (src[j]) {
      unsigned char d = (unsigned char)src[j];
      if (d == ' ' || d == '\t' || d == '\r' || d == '\n') break;
      if (d >= 0x80) {
        unsigned cp;
        int l = ucp((const unsigned char *)src + j, n - j, &cp);
        if (!l) return lex_err(err, errsz, line, "malformed UTF-8");
        if (cp == 0x3000) break;
        j += (size_t)l;
        continue;
      }
      j++;
    }
    char w[128];
    if (j - i >= sizeof w) return lex_err(err, errsz, line, "word too long");
    memcpy(w, src + i, j - i); w[j - i] = 0;
    i = j;
    const char *cn = reg ? reg_ja(reg, w) : NULL;
    if (cn) {
      { int ix = tb_push(b, T_NAME, line); b->name[ix] = cn; }   /* canonical name lives in the registry */
      continue;
    }
    int hit = 0;
    for (size_t k = 0; k < sizeof jatab / sizeof *jatab; k++) {
      if (!strcmp(w, jatab[k].w)) {
        tb_push(b, jatab[k].k, line);
        b->post[b->n - 1] = (unsigned char)jatab[k].post;
        hit = 1; break;
      }
    }
    if (hit) continue;
    double v; char u[8];
    if (ja_numeral(w, &v, u)) {
      int ix = tb_push(b, u[0] ? T_COUNTER : T_NUM, line);
      b->num[ix] = v;
      if (u[0]) b->name[ix] = intern(b->it, b->a, u, strlen(u));
      continue;
    }
    return lex_err(err, errsz, line, "unknown word '%s'", w);
  }
  /* normalize, step 1: delete the fused TGT markers (compact every column) */
  int m = 0;
  for (int k = 0; k < b->n; k++) {
    if (b->kind[k] == K_TGT) continue;
    b->kind[m] = b->kind[k]; b->name[m] = b->name[k]; b->num[m] = b->num[k];
    b->line[m] = b->line[k]; b->post[m] = b->post[k];
    m++;
  }
  b->n = m;
  /* step 2: re-root each postfix operator before its operand (never across T_NL) */
  for (int k = 0; k < b->n; k++) {
    if (!b->post[k]) continue;
    if (k == 0 || b->kind[k - 1] == T_NL)
      return lex_err(err, errsz, b->line[k], "postfix operator with no operand");
    TokKind tk = b->kind[k]; b->kind[k] = b->kind[k - 1]; b->kind[k - 1] = tk;
    const char *tn = b->name[k]; b->name[k] = b->name[k - 1]; b->name[k - 1] = tn;
    double tv = b->num[k]; b->num[k] = b->num[k - 1]; b->num[k - 1] = tv;
    int tl = b->line[k]; b->line[k] = b->line[k - 1]; b->line[k - 1] = tl;
    b->post[k] = 0; b->post[k - 1] = 0;
  }
  return 0;
}

/* Inputs: full source (directives blanked by main), ja flag, registry, arena.
 * Outputs: *toks — the column stream, T_NL between lines, ending in T_EOF.
 * Output: 0 ok / -1 with err set. Invariant: blank lines emit nothing. */
int ano_lex(const char *src, int ja, const Registry *reg, Arena *a,
            Toks *toks, char *err, size_t errsz) {
  TokBuf b = {0};
  Intern it = {0};
  b.a = a; b.it = &it;
  if (err && errsz) err[0] = 0;
  if (!src) src = "";
  /* strict well-formedness once at the boundary; skins then decode unchecked */
  {
    size_t n = strlen(src);
    int pl = 1;
    for (size_t i = 0; i < n;) {
      unsigned char c = (unsigned char)src[i];
      if (c < 0x80) { pl += c == '\n'; i++; continue; }
      unsigned cp;
      int l = ucp((const unsigned char *)src + i, n - i, &cp);
      if (!l) return lex_err(err, errsz, pl, "malformed UTF-8");
      i += (size_t)l;
    }
  }
  int rc = ja ? lex_ja(src, reg, &b, err, errsz) : lex_ascii(src, &b, err, errsz);
  if (rc) return -1;
  if (b.n && b.kind[b.n - 1] == T_NL) b.n--;     /* NL separates, never terminates */
  int line = b.n ? b.line[b.n - 1] : 1;
  tb_push(&b, T_EOF, line);
  toks->n = b.n; toks->kind = b.kind; toks->name = b.name; toks->num = b.num; toks->line = b.line;
  return 0;
}
