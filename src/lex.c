/* lex.c — anoc tokenizer: the ASCII surface and the Japanese spaced skin (--! ja).
 * Contract: ano.h (Tok, ano_lex), GRAMMAR.md "Lexical". ASCII mode fuses fold/scan
 * tokens and decides @alias vs @ by term position. JA mode lexes space-separated
 * words — registry ja aliases, the particle table, a kanji numeral reader — then
 * normalizes per demos/9-nihongo/40-tokenizer-skin.bqn: drop the に TGT marker and
 * re-root each postfix operator before its operand. The postfix flag lives beside
 * the token buffer and never escapes this file. */
#include "ano.h"

/* internal に target marker; deleted in normalization, never returned */
#define K_TGT ((TokKind)T_KINDCOUNT)

/* char classes, ASCII only, locale-free */
static int nstart(int c) { return (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z'); }
static int nchar(int c)  { return nstart(c) || (c >= '0' && c <= '9') || c == '_'; }
static int dig(int c)    { return c >= '0' && c <= '9'; }

/* growing token buffer; post[] marks JA postfix operators, internal only */
typedef struct { Tok *t; unsigned char *post; int n, cap; Arena *a; } TokBuf;

/* Inputs: buffer, kind, line. Output: pointer to the zeroed appended token.
 * Invariant: t and post grow together; old blocks stay in the arena. */
static Tok *tb_push(TokBuf *b, TokKind k, int line) {
  if (b->n == b->cap) {
    int cap = b->cap ? b->cap * 2 : 128;
    Tok *nt = (Tok *)arena_alloc(b->a, (size_t)cap * sizeof *nt);
    unsigned char *np = (unsigned char *)arena_alloc(b->a, (size_t)cap);
    if (b->n) { memcpy(nt, b->t, (size_t)b->n * sizeof *nt); memcpy(np, b->post, (size_t)b->n); }
    b->t = nt; b->post = np; b->cap = cap;
  }
  Tok *t = &b->t[b->n];
  memset(t, 0, sizeof *t);
  t->kind = k; t->line = line;
  b->post[b->n] = 0;
  b->n++;
  return t;
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

/* Inputs: buffer. Output: 1 when the next token sits in term position — line
 * start, or after an operator/keyword — per GRAMMAR.md's alias-sigil rule. */
static int termpos(const TokBuf *b) {
  if (b->n == 0) return 1;
  TokKind k = b->t[b->n - 1].kind;
  if (k >= T_DEF && k < T_KINDCOUNT) return 1;
  switch (k) {
    case T_NL: case T_COMMA: case T_ARROW: case T_SEMI: case T_PIPEGT:
    case T_AMP: case T_BAR: case T_BANG:
    case T_EQEQ: case T_NE: case T_LT: case T_LE: case T_GT: case T_GE: case T_EQ:
    case T_PLUSEQ: case T_MINUSEQ: case T_STAREQ: case T_SLASHEQ:
    case T_PLUS: case T_MINUS: case T_STAR: case T_SLASH: case T_PCT:
    case T_AT: case T_DOT: case T_LP: case T_LB: case T_LARROW:
      return 1;
    default:
      return 0;
  }
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
 * for max/min/avg and never before '='; @alias only in term position. */
static int lex_ascii(const char *src, TokBuf *b, char *err, size_t errsz) {
  int line = 1;
  size_t i = 0;
  while (src[i]) {
    unsigned char c = (unsigned char)src[i];
    if (c == '\n') {
      if (b->n && b->t[b->n - 1].kind != T_NL) tb_push(b, T_NL, line);
      line++; i++; continue;
    }
    if (c == ' ' || c == '\t' || c == '\r') { i++; continue; }
    if (c == '-' && src[i + 1] == '-') { while (src[i] && src[i] != '\n') i++; continue; }
    if (nstart(c)) {
      size_t j = i + 1;
      while (nchar((unsigned char)src[j])) j++;
      size_t len = j - i;
      if (len >= ANO_NAMESZ) return lex_err(err, errsz, line, "name too long");
      char nm[ANO_NAMESZ]; memcpy(nm, src + i, len); nm[len] = 0;
      /* reducer fold: max/ min/ avg/ — no whitespace, and 'max/= 2' stays SLASHEQ */
      int red = !strcmp(nm, "max") || !strcmp(nm, "min") || !strcmp(nm, "avg");
      if (red && src[j] == '/' && src[j + 1] != '=') {
        Tok *t = tb_push(b, T_FOLD, line); strcpy(t->name, nm); i = j + 1; continue;
      }
      if (!strcmp(nm, "max") && src[j] == '\\') {
        Tok *t = tb_push(b, T_SCANOP, line); strcpy(t->name, nm); i = j + 1; continue;
      }
      TokKind kk = kwkind(nm);
      if (kk) { tb_push(b, kk, line); i = j; continue; }
      Tok *t = tb_push(b, T_NAME, line); strcpy(t->name, nm);
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
        if (k - j >= ANO_NAMESZ) return lex_err(err, errsz, line, "counter unit too long");
        Tok *t = tb_push(b, T_COUNTER, line); t->num = v; memcpy(t->name, src + j, k - j);
        i = k;
      } else {
        Tok *t = tb_push(b, T_NUM, line); t->num = v;
        i = j;
      }
      continue;
    }
    if (c == '"') {
      size_t j = i + 1;
      while (src[j] && src[j] != '"' && src[j] != '\n') j++;
      if (src[j] != '"') return lex_err(err, errsz, line, "unterminated string");
      size_t len = j - i - 1;
      if (len >= ANO_NAMESZ)
        return lex_err(err, errsz, line, "string longer than %d bytes (Tok.name contract)", ANO_NAMESZ - 1);
      Tok *t = tb_push(b, T_STR, line); memcpy(t->name, src + i + 1, len);
      i = j + 1; continue;
    }
    if (c == '`') {
      if (!nstart((unsigned char)src[i + 1])) return lex_err(err, errsz, line, "backtick needs a name");
      size_t j = i + 2;
      while (nchar((unsigned char)src[j])) j++;
      size_t len = j - i - 1;
      if (len >= ANO_NAMESZ) return lex_err(err, errsz, line, "symbol too long");
      Tok *t = tb_push(b, T_SYM, line); memcpy(t->name, src + i + 1, len);
      i = j; continue;
    }
    if (c == '_') {
      if (nchar((unsigned char)src[i + 1])) return lex_err(err, errsz, line, "names cannot start with '_'");
      tb_push(b, T_WILD, line); i++; continue;
    }
    if (c == 0xE2 && (unsigned char)src[i + 1] == 0x86 && (unsigned char)src[i + 2] == 0x95) {
      tb_push(b, T_IOTA, line); i += 3; continue;              /* ↕ */
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
        else if (d == '/') { Tok *t = tb_push(b, T_FOLD, line); strcpy(t->name, "|"); i += 2; }
        else { tb_push(b, T_BAR, line); i++; }
        break;
      case '&':
        if (d == '/') { Tok *t = tb_push(b, T_FOLD, line); strcpy(t->name, "&"); i += 2; }
        else { tb_push(b, T_AMP, line); i++; }
        break;
      case '+':
        if (d == '=') { tb_push(b, T_PLUSEQ, line); i += 2; }
        else if (d == '/') { Tok *t = tb_push(b, T_FOLD, line); strcpy(t->name, "+"); i += 2; }
        else if (d == '\\') { Tok *t = tb_push(b, T_SCANOP, line); strcpy(t->name, "+"); i += 2; }
        else { tb_push(b, T_PLUS, line); i++; }
        break;
      case '-':
        if (d == '=') { tb_push(b, T_MINUSEQ, line); i += 2; } else { tb_push(b, T_MINUS, line); i++; }
        break;
      case '*':
        if (d == '=') { tb_push(b, T_STAREQ, line); i += 2; }
        else if (d == '/') { Tok *t = tb_push(b, T_FOLD, line); strcpy(t->name, "*"); i += 2; }
        else if (d == '\\') { Tok *t = tb_push(b, T_SCANOP, line); strcpy(t->name, "*"); i += 2; }
        else { tb_push(b, T_STAR, line); i++; }
        break;
      case '/':
        if (d == '=') { tb_push(b, T_SLASHEQ, line); i += 2; } else { tb_push(b, T_SLASH, line); i++; }
        break;
      case '#':
        if (d == '/') { Tok *t = tb_push(b, T_FOLD, line); strcpy(t->name, "#"); i += 2; }
        else return lex_err(err, errsz, line, "'#' begins only the fold '#/'");
        break;
      case '@':
        if (nstart(d) && termpos(b)) {                         /* @alias sigil */
          size_t j = i + 2;
          while (nchar((unsigned char)src[j])) j++;
          size_t len = j - i - 1;
          if (len >= ANO_NAMESZ) return lex_err(err, errsz, line, "alias name too long");
          Tok *t = tb_push(b, T_ALIAS, line); memcpy(t->name, src + i + 1, len);
          i = j;
        } else { tb_push(b, T_AT, line); i++; }
        break;
      case '\'': return lex_err(err, errsz, line, "stray tick: ' is postfix on a name");
      case '\\': return lex_err(err, errsz, line, "stray '\\': scans are +\\ *\\ max\\");
      default:   return lex_err(err, errsz, line, "unknown byte 0x%02X", c);
    }
  }
  return 0;
}

/* Inputs: s at a UTF-8 char boundary. Outputs: *cp. Output: byte length 1-4, 0 malformed. */
static int ucp(const unsigned char *s, unsigned *cp) {
  if (s[0] < 0x80) { *cp = s[0]; return 1; }
  if ((s[0] & 0xE0) == 0xC0 && (s[1] & 0xC0) == 0x80) {
    *cp = ((unsigned)(s[0] & 0x1F) << 6) | (s[1] & 0x3F); return 2;
  }
  if ((s[0] & 0xF0) == 0xE0 && (s[1] & 0xC0) == 0x80 && (s[2] & 0xC0) == 0x80) {
    *cp = ((unsigned)(s[0] & 0x0F) << 12) | ((unsigned)(s[1] & 0x3F) << 6) | (s[2] & 0x3F); return 3;
  }
  if ((s[0] & 0xF8) == 0xF0 && (s[1] & 0xC0) == 0x80 && (s[2] & 0xC0) == 0x80 && (s[3] & 0xC0) == 0x80) {
    *cp = ((unsigned)(s[0] & 0x07) << 18) | ((unsigned)(s[1] & 0x3F) << 12)
        | ((unsigned)(s[2] & 0x3F) << 6) | (s[3] & 0x3F); return 4;
  }
  return 0;
}

/* Inputs: codepoint. Output: kanji digit value 0-9, or -1. */
static int jadig(unsigned cp) {
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
    int l = ucp((const unsigned char *)buf + p, &cp);
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
  size_t i = 0;
  while (src[i]) {
    unsigned char c = (unsigned char)src[i];
    if (c == '\n') {
      if (b->n && b->t[b->n - 1].kind != T_NL) tb_push(b, T_NL, line);
      line++; i++; continue;
    }
    if (c == ' ' || c == '\t' || c == '\r') { i++; continue; }
    if (c == 0xE3 && (unsigned char)src[i + 1] == 0x80 && (unsigned char)src[i + 2] == 0x80) {
      i += 3; continue;                                        /* U+3000 ideographic space */
    }
    if (c == '-' && src[i + 1] == '-') { while (src[i] && src[i] != '\n') i++; continue; }
    /* word: run to the next space/newline */
    size_t j = i;
    while (src[j]) {
      unsigned char d = (unsigned char)src[j];
      if (d == ' ' || d == '\t' || d == '\r' || d == '\n') break;
      if (d == 0xE3 && (unsigned char)src[j + 1] == 0x80 && (unsigned char)src[j + 2] == 0x80) break;
      j++;
    }
    char w[128];
    if (j - i >= sizeof w) return lex_err(err, errsz, line, "word too long");
    memcpy(w, src + i, j - i); w[j - i] = 0;
    i = j;
    const char *cn = reg ? reg_ja(reg, w) : NULL;
    if (cn) {
      if (strlen(cn) >= ANO_NAMESZ) return lex_err(err, errsz, line, "registry name too long");
      Tok *t = tb_push(b, T_NAME, line); strcpy(t->name, cn);
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
    double v; char u[ANO_NAMESZ];
    if (ja_numeral(w, &v, u)) {
      Tok *t = tb_push(b, u[0] ? T_COUNTER : T_NUM, line);
      t->num = v; strcpy(t->name, u);
      continue;
    }
    return lex_err(err, errsz, line, "unknown word '%s'", w);
  }
  /* normalize, step 1: delete the fused TGT markers */
  int m = 0;
  for (int k = 0; k < b->n; k++)
    if (b->t[k].kind != K_TGT) { b->t[m] = b->t[k]; b->post[m] = b->post[k]; m++; }
  b->n = m;
  /* step 2: re-root each postfix operator before its operand (never across T_NL) */
  for (int k = 0; k < b->n; k++) {
    if (!b->post[k]) continue;
    if (k == 0 || b->t[k - 1].kind == T_NL)
      return lex_err(err, errsz, b->t[k].line, "postfix operator with no operand");
    Tok tmp = b->t[k]; b->t[k] = b->t[k - 1]; b->t[k - 1] = tmp;
    b->post[k] = 0; b->post[k - 1] = 0;
  }
  return 0;
}

/* Inputs: full source (directives blanked by main), ja flag, registry, arena.
 * Outputs: *toks and *ntoks — the stream, T_NL between lines, ending in T_EOF.
 * Output: 0 ok / -1 with err set. Invariant: blank lines emit nothing. */
int ano_lex(const char *src, int ja, const Registry *reg, Arena *a,
            Tok **toks, int *ntoks, char *err, size_t errsz) {
  TokBuf b = {0};
  b.a = a;
  if (err && errsz) err[0] = 0;
  if (!src) src = "";
  int rc = ja ? lex_ja(src, reg, &b, err, errsz) : lex_ascii(src, &b, err, errsz);
  if (rc) return -1;
  if (b.n && b.t[b.n - 1].kind == T_NL) b.n--;   /* NL separates, never terminates */
  int line = b.n ? b.t[b.n - 1].line : 1;
  tb_push(&b, T_EOF, line);
  *toks = b.t; *ntoks = b.n;
  return 0;
}

/* ---------- self-test ---------- */
#ifdef LEX_TEST

/* stub ja lookup for the self-test; the real one lives in registry.c (not linked) */
const char *reg_ja(const Registry *reg, const char *jaWord) {
  (void)reg;
  static const char *tab[][2] = {
    {"北", "nord"}, {"両手", "twoHanded"}, {"金", "gold"},
    {"商人", "Merchant"}, {"ホワイトラン", "Whiterun"},
  };
  for (size_t k = 0; k < sizeof tab / sizeof *tab; k++)
    if (!strcmp(jaWord, tab[k][0])) return tab[k][1];
  return NULL;
}

static const char *kindname[T_KINDCOUNT] = {
  "EOF","NL","NAME","ALIAS","SYM","NUM","COUNTER","STR","WILD",
  "COMMA","ARROW","SEMI","PIPEGT","AMP","BAR","BANG",
  "EQEQ","NE","LT","LE","GT","GE","EQ",
  "PLUSEQ","MINUSEQ","STAREQ","SLASHEQ",
  "PLUS","MINUS","STAR","SLASH","PCT",
  "AT","DOT","TICK","LP","RP","LB","RB","LARROW","TILDE",
  "FOLD","SCANOP","IOTA",
  "DEF","SPAWN","ATKW","TO","VIA","ALONG","ORDER","BY","TAKE","DESC","TOP","GRADE",
  "REDUCE","SCANKW","SCAN2","CROSS","EXPAND",
};

/* Inputs: label, source, ja flag, expected "KIND:name:num" lines. Output: 0 pass /
 * 1 fail; prints the actual stream, flags every divergence. */
static int runcase(const char *label, const char *src, int ja, const char **exp, int nexp) {
  Arena a = {0};
  Tok *toks = NULL; int n = 0;
  char err[ANO_ERRSZ];
  Registry reg; memset(&reg, 0, sizeof reg);
  int bad = 0;
  printf("== %s\n", label);
  if (ano_lex(src, ja, &reg, &a, &toks, &n, err, sizeof err) != 0) {
    printf("LEX ERROR: %s\n", err);
    arena_free(&a);
    return 1;
  }
  for (int k = 0; k < n; k++) {
    char got[256];
    snprintf(got, sizeof got, "%s:%s:%g", kindname[toks[k].kind], toks[k].name, toks[k].num);
    printf("%s\n", got);
    if (k >= nexp) { printf("  ^ EXTRA (expected %d tokens)\n", nexp); bad = 1; }
    else if (strcmp(got, exp[k])) { printf("  ^ MISMATCH want %s\n", exp[k]); bad = 1; }
  }
  if (n < nexp) { printf("MISSING: want %d tokens, got %d\n", nexp, n); bad = 1; }
  arena_free(&a);
  return bad;
}

int main(void) {
  static const char *srcA =
    "Cow & Weight < avg/ Weight @ Cow\n"
    "Speed /= 2\n"
    "def dps = Damage / Cooldown\n"
    "#/ (livestock' & Cattle)\n"
    "max\\ Height @ (Eye + ↕n * north)\n"
    "+/ Gold @ Nord\n";
  static const char *expA[] = {
    "NAME:Cow:0","AMP::0","NAME:Weight:0","LT::0","FOLD:avg:0","NAME:Weight:0","AT::0","NAME:Cow:0","NL::0",
    "NAME:Speed:0","SLASHEQ::0","NUM::2","NL::0",
    "DEF::0","NAME:dps:0","EQ::0","NAME:Damage:0","SLASH::0","NAME:Cooldown:0","NL::0",
    "FOLD:#:0","LP::0","NAME:livestock:0","TICK::0","AMP::0","NAME:Cattle:0","RP::0","NL::0",
    "SCANOP:max:0","NAME:Height:0","AT::0","LP::0","NAME:Eye:0","PLUS::0","IOTA::0","NAME:n:0","STAR::0","NAME:north:0","RP::0","NL::0",
    "FOLD:+:0","NAME:Gold:0","AT::0","NAME:Nord:0","EOF::0",
  };
  static const char *srcB =
    "@cursor , Knockback 5 ; Flash `Red ; -Shielded\n"
    "Node , OutDeg = +/ Adj@row\n"
    "Cheese @ cellar & Aged > 3mo , Price *= 2\n"
    "\"RNBQKBNR\" to 8 8 , spawn (pieceOf char)\n"
    "Soldier , pos = to 4 _\n";
  static const char *expB[] = {
    "ALIAS:cursor:0","COMMA::0","NAME:Knockback:0","NUM::5","SEMI::0","NAME:Flash:0","SYM:Red:0","SEMI::0","MINUS::0","NAME:Shielded:0","NL::0",
    "NAME:Node:0","COMMA::0","NAME:OutDeg:0","EQ::0","FOLD:+:0","NAME:Adj:0","AT::0","NAME:row:0","NL::0",
    "NAME:Cheese:0","AT::0","NAME:cellar:0","AMP::0","NAME:Aged:0","GT::0","COUNTER:mo:3","COMMA::0","NAME:Price:0","STAREQ::0","NUM::2","NL::0",
    "STR:RNBQKBNR:0","TO::0","NUM::8","NUM::8","COMMA::0","SPAWN::0","LP::0","NAME:pieceOf:0","NAME:char:0","RP::0","NL::0",
    "NAME:Soldier:0","COMMA::0","NAME:pos:0","EQ::0","TO::0","NUM::4","WILD::0","EOF::0",
  };
  static const char *srcC =
    "[ t & c , +InRange | t <- Tower, c <- Creep, dist(t, c) < 50 ]\n"
    "Enemy |> order by Threat desc |> take 5 , +Targeted\n"
    "def spread = Plot & p => +Planted\n"
    "12 , offset = prev.offset + prev.prev.offset\n";
  static const char *expC[] = {
    "LB::0","NAME:t:0","AMP::0","NAME:c:0","COMMA::0","PLUS::0","NAME:InRange:0","BAR::0",
    "NAME:t:0","LARROW::0","NAME:Tower:0","COMMA::0","NAME:c:0","LARROW::0","NAME:Creep:0","COMMA::0",
    "NAME:dist:0","LP::0","NAME:t:0","COMMA::0","NAME:c:0","RP::0","LT::0","NUM::50","RB::0","NL::0",
    "NAME:Enemy:0","PIPEGT::0","ORDER::0","BY::0","NAME:Threat:0","DESC::0","PIPEGT::0","TAKE::0","NUM::5","COMMA::0","PLUS::0","NAME:Targeted:0","NL::0",
    "DEF::0","NAME:spread:0","EQ::0","NAME:Plot:0","AMP::0","NAME:p:0","ARROW::0","PLUS::0","NAME:Planted:0","NL::0",
    "NUM::12","COMMA::0","NAME:offset:0","EQ::0","NAME:prev:0","DOT::0","NAME:offset:0","PLUS::0",
    "NAME:prev:0","DOT::0","NAME:prev:0","DOT::0","NAME:offset:0","EOF::0",
  };
  static const char *srcD =
    "Nord , Gold += 100 -- pay them\n"
    "\n"
    "-- only a comment\n"
    "Dragon , Health = 0\n";
  static const char *expD[] = {
    "NAME:Nord:0","COMMA::0","NAME:Gold:0","PLUSEQ::0","NUM::100","NL::0",
    "NAME:Dragon:0","COMMA::0","NAME:Health:0","EQ::0","NUM::0","EOF::0",
  };
  static const char *srcE = "北 と 両手 六十 より 、 金 に 千 たす\n";
  static const char *expE[] = {
    "NAME:nord:0","AMP::0","NAME:twoHanded:0","GT::0","NUM::60","COMMA::0",
    "NAME:gold:0","PLUSEQ::0","NUM::1000","EOF::0",
  };
  static const char *srcF =
    "商人 ホワイトラン で 、 金 に 五千 たす\n"
    "金 に 九千九百九十九 たす\n"
    "金 に 一・〇五 かける\n"
    "金 三ヶ月 未満\n";
  static const char *expF[] = {
    "NAME:Merchant:0","AT::0","NAME:Whiterun:0","COMMA::0","NAME:gold:0","PLUSEQ::0","NUM::5000","NL::0",
    "NAME:gold:0","PLUSEQ::0","NUM::9999","NL::0",
    "NAME:gold:0","STAREQ::0","NUM::1.05","NL::0",
    "NAME:gold:0","LT::0","COUNTER:mo:3","EOF::0",
  };
  int bad = 0;
  bad |= runcase("A: folds, scans, division, tick", srcA, 0, expA, (int)(sizeof expA / sizeof *expA));
  bad |= runcase("B: alias vs @, counter, sym, string, wild", srcB, 0, expB, (int)(sizeof expB / sizeof *expB));
  bad |= runcase("C: comprehension, pipeline, arrow, hops", srcC, 0, expC, (int)(sizeof expC / sizeof *expC));
  bad |= runcase("D: newlines, blanks, comments", srcD, 0, expD, (int)(sizeof expD / sizeof *expD));
  bad |= runcase("E: ja ex40 sentence", srcE, 1, expE, (int)(sizeof expE / sizeof *expE));
  bad |= runcase("F: ja scope, numerals, decimal, counter", srcF, 1, expF, (int)(sizeof expF / sizeof *expF));
  printf(bad ? "LEX_TEST: FAIL\n" : "LEX_TEST: PASS\n");
  return bad;
}
#endif
