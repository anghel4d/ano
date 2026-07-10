/* kore.c — これ, the world at hand: the ano editor, a TUI over the anoc process and
 * text boundary. kore spawns anoc as a child exactly as anoc spawns cbqn; its data
 * contract is the .reg format. It never includes ano.h.
 *
 * Six surfaces (EDITOR.md): the demos rail, the code editor, the world table, the
 * space (the world as a glyph grid), the output log, and the prompt. Each REPL
 * submission is one program against the current world — anoc --run --save advances
 * the world file through the staged-rename commit loop; the undo ring is the loop's
 * free gift (pre-states are files under .kore/undo/).
 *
 * Zero deps: raw ANSI CSI rendering + termios raw mode, double-buffered into one
 * write(2) per frame; SGR mouse reporting; SIGWINCH resize; CJK/kana/fullwidth
 * codepoints occupy 2 cells. kore's .reg reader is line-oriented: data lines parse
 * into tables for display, schema and unknown lines are preserved verbatim —
 * pass-through, never regeneration. Cell edits splice one word in the .reg text.
 *
 * Entry points: `kore <file.reg>` the bare world, REPL-only; `kore <file.ano>` the
 * demo form; bare `kore` the rail. Headless verification hooks: `kore --check
 * <file.reg>…` loads and renders both views to memory and reports; `kore --edit
 * <file.reg> <seg> <row> <col> <value>` performs one cell splice and prints the line.
 */
#define _GNU_SOURCE
#include <ctype.h>
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <poll.h>
#include <signal.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <termios.h>
#include <unistd.h>

#define KMAXENT 512
#define KMAXDEMO 1024
#define KMAXHIST 128
#define KNAMESZ 256

/* ---------- growable byte buffer ---------- */

typedef struct { char *s; size_t len, cap; } Buf;

/* Inputs: buffer, bytes + length. Output: appended verbatim, NUL-kept. */
static void bput(Buf *b, const char *s, size_t n) {
  if (b->len + n + 1 > b->cap) {
    size_t cap = b->cap ? b->cap : 256;
    while (cap < b->len + n + 1) cap *= 2;
    b->s = realloc(b->s, cap);
    if (!b->s) abort();
    b->cap = cap;
  }
  memcpy(b->s + b->len, s, n);
  b->len += n;
  b->s[b->len] = 0;
}
static void bprintf(Buf *b, const char *fmt, ...) {
  char tmp[4096];
  va_list ap;
  va_start(ap, fmt);
  int n = vsnprintf(tmp, sizeof tmp, fmt, ap);
  va_end(ap);
  if (n > 0) bput(b, tmp, (size_t)n < sizeof tmp ? (size_t)n : sizeof tmp - 1);
}
static void bfree(Buf *b) { free(b->s); b->s = NULL; b->len = b->cap = 0; }

static void *xalloc(size_t n) { void *p = calloc(1, n ? n : 1); if (!p) abort(); return p; }
static char *xstrdup(const char *s) { char *p = strdup(s ? s : ""); if (!p) abort(); return p; }

/* ---------- UTF-8 + display width ---------- */

/* Inputs: byte cursor. Output: codepoint (replacement on malformed), cursor advanced. */
static uint32_t u8next(const char **pp) {
  const unsigned char *p = (const unsigned char *)*pp;
  uint32_t c = *p;
  int n = c < 0x80 ? 1 : c < 0xC0 ? 1 : c < 0xE0 ? 2 : c < 0xF0 ? 3 : 4;
  if (n == 1) { *pp += 1; return c < 0x80 ? c : 0xFFFD; }
  uint32_t v = c & (0x7F >> n);
  for (int i = 1; i < n; i++) {
    if ((p[i] & 0xC0) != 0x80) { *pp += 1; return 0xFFFD; }
    v = (v << 6) | (p[i] & 0x3F);
  }
  *pp += n;
  return v;
}

/* Inputs: codepoint. Output: terminal cell width — 2 for CJK, kana, hangul, and
 * fullwidth blocks, else 1 (control chars render elsewhere as 1-cell escapes). */
static int cw(uint32_t c) {
  if (c < 0x1100) return 1;
  if ((c >= 0x1100 && c <= 0x115F) || (c >= 0x231A && c <= 0x231B) ||
      (c >= 0x2B1B && c <= 0x2B1C) || (c >= 0x2E80 && c <= 0x303E) ||
      (c >= 0x3041 && c <= 0x33FF) || (c >= 0x3400 && c <= 0x4DBF) ||
      (c >= 0x4E00 && c <= 0x9FFF) || (c >= 0xA000 && c <= 0xA4CF) ||
      (c >= 0xAC00 && c <= 0xD7A3) || (c >= 0xF900 && c <= 0xFAFF) ||
      (c >= 0xFE30 && c <= 0xFE4F) || (c >= 0xFF00 && c <= 0xFF60) ||
      (c >= 0xFFE0 && c <= 0xFFE6) || (c >= 0x1F300 && c <= 0x1FAFF) ||
      (c >= 0x20000 && c <= 0x3FFFD)) return 2;
  return 1;
}

/* Inputs: UTF-8 string. Output: total display width. */
static int swidth(const char *s) {
  int w = 0;
  while (*s) w += cw(u8next(&s));
  return w;
}

/* ---------- terminal: raw mode, cell grid, one write per frame ---------- */

enum { A_DIM = 1, A_BOLD = 2, A_REV = 4 };

typedef struct { char g[8]; uint8_t attr, fg, cont; } Cell; /* fg: 0 default, else SGR code */

static struct {
  int rows, cols;
  Cell *grid;
  Buf out;
  struct termios saved;
  int rawOn, resized;
} T;

/* Every exit path restores the terminal and the mouse state. Idempotent. */
static void term_leave(void) {
  if (!T.rawOn) return;
  T.rawOn = 0;
  const char *bye = "\x1b[?1002l\x1b[?1006l\x1b[?25h\x1b[?1049l\x1b[0m";
  ssize_t r = write(1, bye, strlen(bye));
  (void)r;
  tcsetattr(0, TCSAFLUSH, &T.saved);
}
static void on_fatal(int sig) { term_leave(); signal(sig, SIG_DFL); raise(sig); }
static void on_winch(int sig) { (void)sig; T.resized = 1; }

static void term_size(void) {
  struct winsize ws;
  if (ioctl(0, TIOCGWINSZ, &ws) == 0 && ws.ws_col > 0 && ws.ws_row > 0) { T.rows = ws.ws_row; T.cols = ws.ws_col; }
  else { T.rows = 24; T.cols = 80; }
  /* a floor keeps every layout rect non-degenerate; drawing past a smaller real
   * terminal just wraps, it never writes out of the grid */
  if (T.rows < 4) T.rows = 4;
  if (T.cols < 20) T.cols = 20;
  free(T.grid);
  T.grid = xalloc((size_t)T.rows * T.cols * sizeof(Cell));
}

static int term_enter(void) {
  if (tcgetattr(0, &T.saved)) return -1;
  struct termios t = T.saved;
  t.c_lflag &= ~(unsigned)(ICANON | ECHO | ISIG);
  t.c_iflag &= ~(unsigned)(IXON | ICRNL);
  t.c_cc[VMIN] = 0;
  t.c_cc[VTIME] = 0;
  if (tcsetattr(0, TCSAFLUSH, &t)) return -1;
  T.rawOn = 1;
  const char *hi = "\x1b[?1049h\x1b[?25l\x1b[?1002h\x1b[?1006h";
  ssize_t r = write(1, hi, strlen(hi));
  (void)r;
  term_size();
  return 0;
}

static void frame_clear(void) {
  for (int i = 0; i < T.rows * T.cols; i++) {
    T.grid[i] = (Cell){ .g = " " };
  }
}

/* Inputs: cell coords, attributes, fg, UTF-8 text, max width (-1: unbounded).
 * Output: cells written, clipped to the grid; wide glyphs take two cells, the
 * second marked continuation. Returns the width consumed. */
static int put(int x, int y, int attr, int fg, const char *s, int maxw) {
  if (y < 0 || y >= T.rows) return 0;
  int w = 0;
  while (*s) {
    const char *at = s;
    uint32_t c = u8next(&s);
    int gw = cw(c);
    if (maxw >= 0 && w + gw > maxw) break;
    if (x + w + gw > T.cols) break;
    if (x + w >= 0 && c >= 0x20) {
      Cell *cl = &T.grid[y * T.cols + x + w];
      /* overwriting half of a wide glyph must not shift the row: writing onto a
       * continuation blanks its owner, and burying a wide head blanks its orphan */
      if (cl->cont && x + w > 0) { Cell *own = cl - 1; own->g[0] = ' '; own->g[1] = 0; }
      size_t bl = (size_t)(s - at);
      if (bl > 7) bl = 7;
      memcpy(cl->g, at, bl);
      cl->g[bl] = 0;
      cl->attr = (uint8_t)attr;
      cl->fg = (uint8_t)fg;
      cl->cont = 0;
      if (gw == 2 && x + w + 1 < T.cols) {
        Cell *c2 = &T.grid[y * T.cols + x + w + 1];
        *c2 = (Cell){ .cont = 1 };
      } else if (gw == 1 && x + w + 1 < T.cols && cl[1].cont) {
        cl[1] = (Cell){ .g = " " };
      }
    }
    w += gw;
  }
  return w;
}

static void fill(int x, int y, int w, int h, const char *g, int attr, int fg) {
  for (int j = y; j < y + h; j++) {
    if (j < 0 || j >= T.rows) continue;
    /* wide glyphs straddling the region's edges must not shift the row */
    if (x > 0 && x < T.cols && T.grid[j * T.cols + x].cont) {
      Cell *own = &T.grid[j * T.cols + x - 1];
      own->g[0] = ' '; own->g[1] = 0;
    }
    if (x + w >= 0 && x + w < T.cols && T.grid[j * T.cols + x + w].cont)
      T.grid[j * T.cols + x + w] = (Cell){ .g = " " };
    for (int i = x; i < x + w; i++)
      if (i >= 0 && i < T.cols) {
        Cell *c = &T.grid[j * T.cols + i];
        snprintf(c->g, sizeof c->g, "%s", g);
        c->attr = (uint8_t)attr;
        c->fg = (uint8_t)fg;
        c->cont = 0;
      }
  }
}

/* reverse-video the cell at (x,y), stepping to a wide glyph's head; bounds-checked —
 * the one door for cursor overlays. */
static void rev_cell(int x, int y) {
  if (x < 0 || y < 0 || y >= T.rows || x >= T.cols) return;
  Cell *c = &T.grid[y * T.cols + x];
  if (c->cont && x > 0) c--;
  c->attr |= A_REV;
}

/* Panel border with a title in the top rule; the focused panel's title glows. */
static void box(int x, int y, int w, int h, const char *title, int focused) {
  if (w < 2 || h < 2) return;
  int a = focused ? A_BOLD : A_DIM;
  int fg = focused ? 96 : 0;
  put(x, y, a, fg, "┌", 1);
  put(x + w - 1, y, a, fg, "┐", 1);
  put(x, y + h - 1, a, fg, "└", 1);
  put(x + w - 1, y + h - 1, a, fg, "┘", 1);
  for (int i = 1; i < w - 1; i++) { put(x + i, y, a, fg, "─", 1); put(x + i, y + h - 1, a, fg, "─", 1); }
  for (int j = 1; j < h - 1; j++) { put(x, y + j, a, fg, "│", 1); put(x + w - 1, y + j, a, fg, "│", 1); }
  if (title && *title) {
    put(x + 2, y, a | A_BOLD, fg, "╴", 1);
    int tw = put(x + 3, y, focused ? A_BOLD : 0, fg, title, w - 6);
    put(x + 3 + tw, y, a | A_BOLD, fg, "╶", 1);
  }
}

/* One write(2) per frame: home the cursor, emit rows with minimal SGR churn. */
static void flush_frame(void) {
  Buf *o = &T.out;
  o->len = 0;
  bput(o, "\x1b[H", 3);
  int cattr = -1, cfg = -1;
  for (int y = 0; y < T.rows; y++) {
    if (y) bput(o, "\r\n", 2);
    for (int x = 0; x < T.cols; x++) {
      Cell *c = &T.grid[y * T.cols + x];
      if (c->cont) continue;
      if (c->attr != cattr || c->fg != cfg) {
        bprintf(o, "\x1b[0%s%s%s", (c->attr & A_DIM) ? ";2" : "", (c->attr & A_BOLD) ? ";1" : "",
                (c->attr & A_REV) ? ";7" : "");
        if (c->fg) bprintf(o, ";%d", c->fg);
        bput(o, "m", 1);
        cattr = c->attr;
        cfg = c->fg;
      }
      bput(o, c->g, strlen(c->g));
    }
  }
  bput(o, "\x1b[0m", 4);
  ssize_t r = write(1, o->s, o->len);
  (void)r;
}

/* ---------- input events ---------- */

enum { EV_NONE, EV_CHAR, EV_KEY, EV_MOUSE };
enum { K_UP = 1, K_DOWN, K_LEFT, K_RIGHT, K_ENTER, K_ESC, K_TAB, K_BS, K_DEL, K_HOME, K_END, K_PGUP, K_PGDN };
enum { M_PRESS, M_RELEASE, M_DRAG, M_WHEELUP, M_WHEELDN };

typedef struct { int type, key, mkind, mx, my; uint32_t ch; char u8[8]; } Ev;

/* Inputs: timeout ms. Output: one byte or -1. */
static int rbyte(int ms) {
  struct pollfd pf = { .fd = 0, .events = POLLIN };
  if (poll(&pf, 1, ms) <= 0) return -1;
  unsigned char b;
  return read(0, &b, 1) == 1 ? b : -1;
}

/* Inputs: blocking-ish read (100ms poll so SIGWINCH is noticed). Output: one event.
 * Decodes UTF-8 chars, CSI keys, and SGR mouse (\x1b[<b;x;yM|m). */
static Ev ev_read(void) {
  Ev e = { 0 };
  int b = rbyte(100);
  if (b < 0) return e;
  if (b == 0x1b) {
    int b2 = rbyte(25);
    if (b2 < 0) { e.type = EV_KEY; e.key = K_ESC; return e; }
    if (b2 != '[' && b2 != 'O') return e;            /* Alt chord: swallowed whole */
    char seq[48];
    int n = 0, trunc = 0;
    for (;;) {
      int c = rbyte(25);
      if (c < 0) return e;
      if (n < (int)sizeof seq - 1) seq[n++] = (char)c;
      else trunc = 1;                                /* overlong: drain to the final byte */
      if (c >= '@' && c <= '~' && c != '[') break;
    }
    seq[n] = 0;
    if (trunc) return e;
    char fin = seq[n - 1];
    if (seq[0] == '<') { /* SGR mouse; modifier bits 4/8/16 strip, unknown codes drop */
      int mb = 0, mx = 0, my = 0;
      sscanf(seq + 1, "%d;%d;%d", &mb, &mx, &my);
      e.type = EV_MOUSE;
      e.mx = mx - 1;
      e.my = my - 1;
      int base = mb & ~28;
      if (base == 64) e.mkind = M_WHEELUP;
      else if (base == 65) e.mkind = M_WHEELDN;
      else if (base >= 66) e.type = EV_NONE;         /* horizontal wheel and beyond */
      else if (base >= 32) {
        if ((base & 3) == 3) e.type = EV_NONE;       /* motion without a button */
        else e.mkind = M_DRAG;
      } else if ((base & 3) == 3) e.type = EV_NONE;
      else e.mkind = fin == 'm' ? M_RELEASE : M_PRESS;
      return e;
    }
    e.type = EV_KEY;
    switch (fin) {
      case 'A': e.key = K_UP; break;
      case 'B': e.key = K_DOWN; break;
      case 'C': e.key = K_RIGHT; break;
      case 'D': e.key = K_LEFT; break;
      case 'H': e.key = K_HOME; break;
      case 'F': e.key = K_END; break;
      case '~': {
        int code = atoi(seq);                        /* "15~" is F5, not Home */
        e.key = code == 3 ? K_DEL : code == 5 ? K_PGUP : code == 6 ? K_PGDN
              : (code == 1 || code == 7) ? K_HOME : (code == 4 || code == 8) ? K_END : 0;
        if (!e.key) e.type = EV_NONE;
        break;
      }
      default: e.type = EV_NONE;
    }
    return e;
  }
  if (b == '\r' || b == '\n') { e.type = EV_KEY; e.key = K_ENTER; return e; }
  if (b == '\t') { e.type = EV_KEY; e.key = K_TAB; return e; }
  if (b == 0x7f || b == 0x08) { e.type = EV_KEY; e.key = K_BS; return e; }
  if (b < 0x20) return e;
  /* UTF-8 continuation */
  e.type = EV_CHAR;
  e.u8[0] = (char)b;
  int need = b < 0x80 ? 0 : b < 0xE0 ? 1 : b < 0xF0 ? 2 : 3;
  for (int i = 0; i < need; i++) {
    int c = rbyte(25);
    if (c < 0) break;
    e.u8[1 + i] = (char)c;
  }
  e.u8[1 + need] = 0;
  const char *p = e.u8;
  e.ch = u8next(&p);
  return e;
}

/* ---------- files ---------- */

/* Inputs: path. Output: malloc'd NUL-terminated contents or NULL. */
static char *read_file(const char *path, size_t *lenOut) {
  FILE *f = fopen(path, "rb");
  if (!f) return NULL;
  fseek(f, 0, SEEK_END);
  long sz = ftell(f);
  if (sz < 0) { fclose(f); return NULL; }
  fseek(f, 0, SEEK_SET);
  char *buf = xalloc((size_t)sz + 1);
  size_t got = fread(buf, 1, (size_t)sz, f);
  fclose(f);
  buf[got] = 0;
  if (lenOut) *lenOut = got;
  return buf;
}

/* The staged commit, kore's copy of the loop's write half: <path>.staged, then
 * rename(2) — atomic, crash leaves old or new, never a torn file. */
static int write_commit(const char *path, const char *data, size_t len) {
  char staged[PATH_MAX];
  if (snprintf(staged, sizeof staged, "%s.staged", path) >= (int)sizeof staged) return -1;
  FILE *f = fopen(staged, "wb");
  if (!f) return -1;
  if (fwrite(data, 1, len, f) != len || fflush(f) || fsync(fileno(f))) { fclose(f); unlink(staged); return -1; }
  if (fclose(f)) { unlink(staged); return -1; }
  if (rename(staged, path)) { unlink(staged); return -1; }
  return 0;
}

static int copy_file(const char *from, const char *to) {
  size_t len = 0;
  char *d = read_file(from, &len);
  if (!d) return -1;
  int rc = write_commit(to, d, len);
  free(d);
  return rc;
}

static void mkdirs(const char *path) {
  char tmp[PATH_MAX];
  snprintf(tmp, sizeof tmp, "%s", path);
  for (char *p = tmp + 1; *p; p++)
    if (*p == '/') { *p = 0; mkdir(tmp, 0755); *p = '/'; }
  mkdir(tmp, 0755);
}

/* ---------- the .reg reader: data lines into tables, schema verbatim ---------- */

typedef enum { E_COL, E_FIELD, E_PRES, E_REL, E_SREL, E_ALIAS } EKind;
typedef enum { V_NUM, V_BOOL, V_SYM, V_CHAR, V_VEC } VType;

typedef struct {
  EKind kind;
  VType type;
  char name[KNAMESZ];
  int line;                 /* index into World.lines — the cell edit splice target */
  double *nums; int nn;     /* num/bool/rel/pres/alias values; vec flattened pairs */
  char **syms; int ns;      /* sym words */
  char *chars;              /* char payload copy */
  int *fibOff, *fibLen; double *fibVals; int nfib; /* srel */
  int isInv;                /* srel spelled as `inv` — fibers derived, not edited */
  char inv[KNAMESZ];        /* the rel an inv derives from */
} Ent;

typedef struct {
  char path[PATH_MAX];
  char **lines; int nlines; /* the file's raw lines, verbatim — the one source of truth */
  int n, latW, latH;
  Ent ents[KMAXENT]; int nents;
  char posCol[KNAMESZ];     /* role pos target, else "" (falls back to literal `pos`) */
  int loaded;
} World;

/* The loader's case contract: ASCII letters fold, every other byte exact. */
static int names_eq(const char *a, const char *b) {
  for (;; a++, b++) {
    unsigned char x = (unsigned char)*a, y = (unsigned char)*b;
    if (x >= 'A' && x <= 'Z') x += 32;
    if (y >= 'A' && y <= 'Z') y += 32;
    if (x != y) return 0;
    if (!x) return 1;
  }
}

/* Inputs: a col/field char line. Output: 0 with the glyph run's byte span — from the
 * 4th word to the comment or line end, trailing whitespace trimmed — mirroring the
 * loader's strip_line + raw-tail read exactly; -1 when the line carries no run. */
static int word_span(const char *line, int idx, int *off, int *len);
static int char_span(const char *line, int *off, int *len) {
  int o, l;
  if (word_span(line, 3, &o, &l)) return -1;
  int end = o, bow = 1;
  for (int i = o; line[i]; i++) {
    if (line[i] == '#' && bow) break;
    bow = (line[i] == ' ' || line[i] == '\t');
    end = i + 1;
  }
  while (end > o && (line[end - 1] == ' ' || line[end - 1] == '\t' || line[end - 1] == '\r')) end--;
  *off = o;
  *len = end - o;
  return 0;
}

/* the loader's representability rules for a glyph run: never empty, no boundary
 * whitespace, no word-boundary '#' — shared by every char write kore performs */
static int run_ok(const char *run, int rows) {
  if (rows <= 0 || (int)strlen(run) != rows) return 0;
  if (run[0] == ' ' || run[0] == '\t' || run[0] == '#') return 0;
  if (run[rows - 1] == ' ' || run[rows - 1] == '\t') return 0;
  for (int j = 1; j < rows; j++)
    if (run[j] == '#' && (run[j - 1] == ' ' || run[j - 1] == '\t')) return 0;
  return 1;
}

/* Inputs: raw line, 0-based word index. Output: 0 with byte offset+len of that word,
 * -1 when the line has fewer words. Splits on space/tab; a word-boundary '#' ends the
 * data (the loader's comment rule), so value words never index into a trailing comment. */
static int word_span(const char *line, int idx, int *off, int *len) {
  const char *p = line;
  int bow = 1, i = 0;
  while (*p) {
    while (*p == ' ' || *p == '\t') { p++; bow = 1; }
    if (!*p || (bow && *p == '#')) return -1;
    const char *w = p;
    while (*p && *p != ' ' && *p != '\t') p++;
    if (i == idx) { *off = (int)(w - line); *len = (int)(p - w); return 0; }
    i++;
    bow = 0;
  }
  return -1;
}

/* Inputs: mutable scratch line. Output: word pointers, comment-stripped. */
static int split_words(char *line, char **words, int maxw) {
  int nw = 0, bow = 1;
  for (char *p = line; *p; p++) {
    if (*p == '#' && bow) { *p = 0; break; }
    bow = (*p == ' ' || *p == '\t');
  }
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

static int wnum(const char *w, double *out) {
  char *end;
  *out = strtod(w, &end);
  return (end != w && *end == 0) ? 0 : -1;
}

static void world_free(World *w) {
  for (int i = 0; i < w->nlines; i++) free(w->lines[i]);
  free(w->lines);
  for (int i = 0; i < w->nents; i++) {
    Ent *e = &w->ents[i];
    free(e->nums);
    for (int j = 0; j < e->ns; j++) free(e->syms[j]);
    free(e->syms);
    free(e->chars);
    free(e->fibOff); free(e->fibLen); free(e->fibVals);
  }
  memset(w, 0, sizeof *w);
}

/* Inputs: path. Output: 0 with the world parsed for display / -1 with err set.
 * Data lines (n, lattice, col, field, pres, rel, srel, inv) fill tables; everything
 * else — bind, alias, fn, role, as, ja, default, comments, unknown — passes through
 * untouched in lines[]. role pos is noted for the space view. */
static int world_load(World *w, const char *path, char *err, size_t errsz) {
  World fresh = { 0 };
  snprintf(fresh.path, sizeof fresh.path, "%s", path);
  size_t flen = 0;
  char *buf = read_file(path, &flen);
  if (!buf) { snprintf(err, errsz, "cannot read %s: %s", path, strerror(errno)); return -1; }
  int cap = 64;
  fresh.lines = xalloc((size_t)cap * sizeof(char *));
  char *save = NULL;
  for (char *p = buf;; p = NULL) {
    char *ln = p ? p : save;
    if (!ln) break;
    char *nl = strchr(ln, '\n');
    if (nl) { *nl = 0; save = nl + 1; } else save = NULL;
    /* the empty tail after a final newline is not a line — appending it would grow
     * the file by one blank line per commit cycle */
    if (!nl && !ln[0] && fresh.nlines) break;
    if (fresh.nlines >= cap) { cap *= 2; fresh.lines = realloc(fresh.lines, (size_t)cap * sizeof(char *)); if (!fresh.lines) abort(); }
    size_t l = strlen(ln);
    if (l && ln[l - 1] == '\r') ln[l - 1] = 0;
    fresh.lines[fresh.nlines++] = xstrdup(ln);
    if (!nl) break;
  }
  free(buf);
  for (int li = 0; li < fresh.nlines; li++) {
    char *dup = xstrdup(fresh.lines[li]);
    int maxw = (int)(strlen(dup) / 2 + 2);
    char **words = xalloc((size_t)maxw * sizeof(char *));
    int nw = split_words(dup, words, maxw);
    if (nw == 0) { free(words); free(dup); continue; }
    const char *k = words[0];
    double d;
    Ent *e = fresh.nents < KMAXENT ? &fresh.ents[fresh.nents] : NULL;
    if (!strcmp(k, "n") && nw == 2 && !wnum(words[1], &d)) fresh.n = d >= 0 && d <= 1e6 ? (int)d : 0;
    else if (!strcmp(k, "lattice") && nw == 3) {
      double h;
      if (!wnum(words[1], &d) && !wnum(words[2], &h) && d >= 0 && d <= 4096 && h >= 0 && h <= 4096) {
        fresh.latW = (int)d;
        fresh.latH = (int)h;
      }
    } else if ((!strcmp(k, "col") || !strcmp(k, "field")) && nw >= 3 && e) {
      e->kind = k[0] == 'f' ? E_FIELD : E_COL;
      e->line = li;
      snprintf(e->name, sizeof e->name, "%.*s", KNAMESZ - 1, words[1]);
      const char *ty = words[2];
      if (!strcmp(ty, "num") || !strcmp(ty, "bool") || !strcmp(ty, "vec")) {
        e->type = !strcmp(ty, "bool") ? V_BOOL : !strcmp(ty, "vec") ? V_VEC : V_NUM;
        e->nums = xalloc((size_t)(nw - 3 + 1) * sizeof(double));
        for (int j = 3; j < nw; j++) {
          if (!strcmp(words[j], "|")) continue;
          if (!wnum(words[j], &e->nums[e->nn])) e->nn++;
        }
        fresh.nents++;
      } else if (!strcmp(ty, "sym")) {
        e->type = V_SYM;
        e->syms = xalloc((size_t)(nw - 3 + 1) * sizeof(char *));
        for (int j = 3; j < nw; j++) e->syms[e->ns++] = xstrdup(words[j]);
        fresh.nents++;
      } else if (!strcmp(ty, "char")) {
        e->type = V_CHAR;
        int off, len;
        /* the run is the raw tail from the 4th word — the loader's own read */
        if (char_span(fresh.lines[li], &off, &len) == 0) {
          e->chars = xalloc((size_t)len + 1);
          memcpy(e->chars, fresh.lines[li] + off, (size_t)len);
        } else e->chars = xstrdup("");
        fresh.nents++;
      }
    } else if (!strcmp(k, "pres") && nw >= 2 && e) {
      e->kind = E_PRES;
      e->line = li;
      snprintf(e->name, sizeof e->name, "%.*s", KNAMESZ - 1, words[1]);
      e->type = V_BOOL;
      e->nums = xalloc((size_t)(nw - 2 + 1) * sizeof(double));
      for (int j = 2; j < nw; j++) if (!wnum(words[j], &e->nums[e->nn])) e->nn++;
      fresh.nents++;
    } else if ((!strcmp(k, "rel") || !strcmp(k, "alias")) && nw >= 2 && e) {
      /* an alias is a stored mask VALUE — data, so it displays and edits like a rel */
      e->kind = k[0] == 'r' ? E_REL : E_ALIAS;
      e->line = li;
      snprintf(e->name, sizeof e->name, "%.*s", KNAMESZ - 1, words[1]);
      e->type = k[0] == 'r' ? V_NUM : V_BOOL;
      e->nums = xalloc((size_t)(nw - 2 + 1) * sizeof(double));
      for (int j = 2; j < nw; j++) if (!wnum(words[j], &e->nums[e->nn])) e->nn++;
      fresh.nents++;
    } else if ((!strcmp(k, "srel") || !strcmp(k, "inv")) && nw >= 2 && e) {
      e->kind = E_SREL;
      e->line = li;
      e->isInv = k[0] == 'i';
      if (e->isInv && nw >= 3) snprintf(e->inv, sizeof e->inv, "%.*s", KNAMESZ - 1, words[2]);
      snprintf(e->name, sizeof e->name, "%.*s", KNAMESZ - 1, words[1]);
      if (!e->isInv) {
        int nfib = 1, nvals = 0;
        for (int j = 2; j < nw; j++) !strcmp(words[j], "|") ? nfib++ : nvals++;
        e->fibOff = xalloc((size_t)nfib * sizeof(int));
        e->fibLen = xalloc((size_t)nfib * sizeof(int));
        e->fibVals = xalloc((size_t)(nvals + 1) * sizeof(double));
        int fib = 0, vi = 0;
        for (int j = 2; j < nw; j++) {
          if (!strcmp(words[j], "|")) { e->fibLen[fib] = vi - e->fibOff[fib]; fib++; e->fibOff[fib] = vi; }
          else if (!wnum(words[j], &e->fibVals[vi])) vi++;
        }
        e->fibLen[fib] = vi - e->fibOff[fib];
        e->nfib = fib + 1;
      }
      fresh.nents++;
    } else if (!strcmp(k, "role") && nw == 3 && !strcmp(words[1], "pos")) {
      snprintf(fresh.posCol, sizeof fresh.posCol, "%.*s", KNAMESZ - 1, words[2]);
    }
    /* everything else: schema, preserved verbatim in lines[] */
    free(words);
    free(dup);
  }
  world_free(w);
  *w = fresh;
  w->loaded = 1;
  return 0;
}

static Ent *world_ent(World *w, const char *name, EKind kind) {
  for (int i = 0; i < w->nents; i++)
    if (w->ents[i].kind == kind && names_eq(w->ents[i].name, name)) return &w->ents[i];
  return NULL;
}

/* the pos-role column for the space view: the declared role, else the literal `pos` */
static Ent *world_pos(World *w) {
  if (w->posCol[0]) {
    Ent *e = world_ent(w, w->posCol, E_COL);
    if (e && e->type == V_VEC) return e;
  }
  Ent *e = world_ent(w, "pos", E_COL);
  return (e && e->type == V_VEC) ? e : NULL;
}

/* Inputs: value. Output: static spelling — integers plain, else shortest %g. */
static const char *fmt_num(double v) {
  static char buf[8][64];
  static int at;
  char *b = buf[at++ & 7];
  if (v == (long long)v && v >= -9e15 && v <= 9e15) snprintf(b, 64, "%lld", (long long)v);
  else {
    for (int p = 1; p <= 17; p++) {
      snprintf(b, 64, "%.*g", p, v);
      if (strtod(b, NULL) == v) break;
    }
  }
  return b;
}

/* ---------- world text surgery: splice one value, commit, reload ---------- */

/* Inputs: world, line index, byte offset + length, replacement. Output: 0 / -1.
 * Splices the raw line, writes the whole file staged+rename, reloads — the text is
 * the truth, the tables are a view. */
static int world_splice(World *w, int line, int off, int len, const char *repl, char *err, size_t errsz) {
  if (line < 0 || line >= w->nlines) { snprintf(err, errsz, "splice: no line %d", line); return -1; }
  const char *old = w->lines[line];
  size_t ol = strlen(old), rl = strlen(repl);
  if (off < 0 || len < 0 || (size_t)off + (size_t)len > ol) { snprintf(err, errsz, "splice: bad span"); return -1; }
  char *nl = xalloc(ol - (size_t)len + rl + 1);
  memcpy(nl, old, (size_t)off);
  memcpy(nl + off, repl, rl);
  memcpy(nl + off + rl, old + off + len, ol - (size_t)off - (size_t)len + 1);
  free(w->lines[line]);
  w->lines[line] = nl;
  Buf b = { 0 };
  for (int i = 0; i < w->nlines; i++) { bput(&b, w->lines[i], strlen(w->lines[i])); bput(&b, "\n", 1); }
  int rc = write_commit(w->path, b.s ? b.s : "", b.len);
  bfree(&b);
  if (rc) { snprintf(err, errsz, "cannot write %.180s: %s", w->path, strerror(errno)); return -1; }
  char path[PATH_MAX];
  snprintf(path, sizeof path, "%s", w->path);
  return world_load(w, path, err, errsz);
}

/* ---------- child processes: anoc, $EDITOR ---------- */

static char anocPath[PATH_MAX + 16];

/* $ANOC wins; else anoc beside kore's own binary's sibling src/, else PATH. */
static const char *find_anoc(void) {
  if (anocPath[0]) return anocPath;
  const char *env = getenv("ANOC");
  if (env && *env && access(env, X_OK) == 0) { snprintf(anocPath, sizeof anocPath, "%s", env); return anocPath; }
  char exe[PATH_MAX];
  ssize_t n = readlink("/proc/self/exe", exe, sizeof exe - 1);
  if (n > 0) {
    exe[n] = 0;
    char *sl = strrchr(exe, '/');
    if (sl) {
      *sl = 0;
      char cand[PATH_MAX + 16];
      snprintf(cand, sizeof cand, "%s/../src/anoc", exe);
      if (access(cand, X_OK) == 0) { snprintf(anocPath, sizeof anocPath, "%s", cand); return anocPath; }
    }
  }
  if (access("src/anoc", X_OK) == 0) { snprintf(anocPath, sizeof anocPath, "src/anoc"); return anocPath; }
  snprintf(anocPath, sizeof anocPath, "anoc");
  return anocPath;
}

/* Inputs: NULL-terminated argv, capture buffer. Output: child exit code (-1 spawn
 * failure); stdout and stderr merge into cap verbatim — the output surface's feed. */
static int run_child(char *const argv[], Buf *cap) {
  int pfd[2];
  if (pipe(pfd)) return -1;
  pid_t pid = fork();
  if (pid < 0) { close(pfd[0]); close(pfd[1]); return -1; }
  if (pid == 0) {
    dup2(pfd[1], 1);
    dup2(pfd[1], 2);
    close(pfd[0]);
    close(pfd[1]);
    execvp(argv[0], argv);
    fprintf(stdout, "kore: cannot exec %s: %s\n", argv[0], strerror(errno));
    _exit(127);
  }
  close(pfd[1]);
  char rb[8192];
  ssize_t got;
  while ((got = read(pfd[0], rb, sizeof rb)) > 0) bput(cap, rb, (size_t)got);
  close(pfd[0]);
  int st = 0;
  waitpid(pid, &st, 0);
  return WIFEXITED(st) ? WEXITSTATUS(st) : -1;
}

/* ---------- the demos rail ---------- */

static char *demoList[KMAXDEMO];
static int nDemos;

static void walk_demos(const char *dir) {
  DIR *d = opendir(dir);
  if (!d) return;
  struct dirent *de;
  while ((de = readdir(d))) {
    if (de->d_name[0] == '.') continue;
    char p[PATH_MAX];
    if (snprintf(p, sizeof p, "%s/%s", dir, de->d_name) >= (int)sizeof p) continue;
    struct stat st;
    if (stat(p, &st)) continue;
    if (S_ISDIR(st.st_mode)) walk_demos(p);
    else {
      size_t l = strlen(p);
      if (l > 4 && !strcmp(p + l - 4, ".ano") && nDemos < KMAXDEMO) demoList[nDemos++] = xstrdup(p);
    }
  }
  closedir(d);
}
static int cmp_str(const void *a, const void *b) { return strcmp(*(char *const *)a, *(char *const *)b); }

/* ---------- app state ---------- */

enum Focus { F_RAIL, F_CODE, F_WORLD, F_OUT, F_PROMPT };
enum Mode { MODE_RAIL, MODE_DEMO, MODE_REG };

typedef struct { int x, y, w, h; } Rect;

static struct App {
  enum Mode mode;
  enum Focus focus;
  World world;
  int worldIsCopy;              /* mutation retargeted to the .kore/play copy */
  int worldIsPost;              /* world view shows a demo run's post-state scratch */
  char pristine[PATH_MAX];      /* the demo's own registry (never mutated) */
  char demoPath[PATH_MAX];
  char **code; int ncode;
  int codeDirty, codeInsert, ccy, ccx, codeTop;
  int railSel, railTop;
  int spaceView;
  int wSeg, wRow, wCol, wTop;   /* world cursor: segment 0 = entity table, 1.. fields */
  int editing; char editBuf[512]; int editLen;
  Buf outLog; int outScroll;    /* lines scrolled back from the tail */
  char verdict[512];
  char prompt[1024]; int plen, pcur;
  char *hist[KMAXHIST]; int nhist, histAt;
  int dragging, dragSeg, dragR0, dragC0, dragR1, dragC1;
  int undoSeq;
  int sessJa;                                   /* the session log's surface; -1 unknown */
  char *sdefText[64]; int sdefJa[64]; char sdefName[64][128]; int nsdefs;
  int quit;
  Rect rail, codeR, worldR, outR, promptR;
} A;

static void say(const char *fmt, ...) {
  va_list ap;
  va_start(ap, fmt);
  vsnprintf(A.verdict, sizeof A.verdict, fmt, ap);
  va_end(ap);
}
static void logOut(const char *s, size_t n) { bput(&A.outLog, s, n); A.outScroll = 0; }

/* ---------- .kore scratch: undo ring, play copies, repl program ---------- */

static const char *world_stem(void) {
  static char stem[PATH_MAX];
  const char *sl = strrchr(A.world.path, '/');
  snprintf(stem, sizeof stem, "%s", sl ? sl + 1 : A.world.path);
  char *dot = strrchr(stem, '.');
  if (dot) *dot = 0;
  return stem;
}

/* scratch names: <stem>-<8-hex FNV of the absolute path>, so two worlds sharing a
 * basename never share an undo ring, play copy, or snapshot series. */
static const char *world_tag(void) {
  static char key[PATH_MAX + 16];
  char rp[PATH_MAX];
  const char *p = realpath(A.world.path, rp) ? rp : A.world.path;
  uint64_t h = 0xcbf29ce484222325u;
  for (const char *s = p; *s; s++) { h ^= (unsigned char)*s; h *= 0x100000001b3u; }
  snprintf(key, sizeof key, "%s-%08x", world_stem(), (unsigned)(h & 0xffffffffu));
  return key;
}

/* 1 when the path resolves under a demos/ tree — the immutable corpus */
static int in_demos(const char *path) {
  char rp[PATH_MAX];
  return realpath(path, rp) && strstr(rp, "/demos/") != NULL;
}

/* Pre-states are just files, so the ring survives kore itself: on world open, the
 * highest existing .kore/undo/<stem>-<seq>.reg resumes the count. */
static void undo_scan(void) {
  A.undoSeq = 0;
  char pre[PATH_MAX + 24];
  int n = snprintf(pre, sizeof pre, "%s-", world_tag());
  DIR *d = opendir(".kore/undo");
  if (!d || n <= 0) { if (d) closedir(d); return; }
  struct dirent *de;
  while ((de = readdir(d))) {
    if (strncmp(de->d_name, pre, (size_t)n)) continue;
    int s = atoi(de->d_name + n);
    if (s > A.undoSeq) A.undoSeq = s;
  }
  closedir(d);
}

/* Every world advance first copies the current .reg into the ring; u steps back.
 * Pre-states are just files — the commit loop's free gift. */
static int undo_push(void) {
  mkdirs(".kore/undo");
  char dst[PATH_MAX + 48];
  snprintf(dst, sizeof dst, ".kore/undo/%s-%d.reg", world_tag(), A.undoSeq + 1);
  if (copy_file(A.world.path, dst)) return -1;
  return ++A.undoSeq;
}
static void undo_drop(void) {
  if (A.undoSeq <= 0) return;
  char p[PATH_MAX + 48];
  snprintf(p, sizeof p, ".kore/undo/%s-%d.reg", world_tag(), A.undoSeq);
  unlink(p);
  A.undoSeq--;
}
static void undo_pop(void) {
  if (A.undoSeq <= 0) { say("nothing to undo"); return; }
  char p[PATH_MAX + 48], err[256];
  snprintf(p, sizeof p, ".kore/undo/%s-%d.reg", world_tag(), A.undoSeq);
  if (copy_file(p, A.world.path)) { say("undo: cannot restore %s", p); return; }
  unlink(p);
  A.undoSeq--;
  char path[PATH_MAX];
  snprintf(path, sizeof path, "%s", A.world.path);
  if (world_load(&A.world, path, err, sizeof err)) say("undo: %s", err);
  else say("undo → pre-state #%d restored (%d left)", A.undoSeq + 1, A.undoSeq);
}

static void session_rehydrate(void);

/* demos and their registries are immutable: mutation requires a copy, made once and
 * announced in the output surface. The demo form always copies (its registry is the
 * demo's fixture); a bare .reg world mutates in place unless it sits under demos/. */
static int world_guard(void) {
  if (A.worldIsCopy || (A.mode == MODE_REG && !in_demos(A.world.path))) return 0;
  mkdirs(".kore/play");
  char dst[PATH_MAX + 48], err[256];
  snprintf(dst, sizeof dst, ".kore/play/%s.reg", world_tag());
  if (copy_file(A.world.path, dst)) { say("cannot copy world to %s", dst); return -1; }
  char msg[PATH_MAX + 112];
  int mn = snprintf(msg, sizeof msg, "world copied to %s — the corpus stays immutable\n", dst);
  logOut(msg, (size_t)mn);
  char path[PATH_MAX + 48];
  snprintf(path, sizeof path, "%s", dst);
  if (world_load(&A.world, path, err, sizeof err)) { say("%s", err); return -1; }
  A.worldIsCopy = 1;
  A.worldIsPost = 0;
  A.sessJa = -1;                     /* the session moves beside the copy */
  undo_scan();
  session_rehydrate();
  return 0;
}

/* ---------- the REPL line: one submission, one program, one tick ---------- */

static void session_path(char *out, size_t sz) {
  char dir[PATH_MAX];
  snprintf(dir, sizeof dir, "%s", A.world.path);
  char *sl = strrchr(dir, '/');
  if (sl) *sl = 0; else snprintf(dir, sizeof dir, ".");
  snprintf(out, sz, "%s/session.ano", dir);
}

/* the log's registry line points at session-base.reg — the world as it stood before
 * the first statement — so a session genuinely replays; a statement on the other
 * surface logs as a comment (one --! ja per file). */
static void session_log(const char *stmt, int ja) {
  char p[PATH_MAX + 16];
  session_path(p, sizeof p);
  int fresh = access(p, F_OK) != 0;
  if (!fresh && A.sessJa < 0) {
    size_t ln = 0;
    char *s = read_file(p, &ln);
    A.sessJa = s && strstr(s, "\n--! ja") != NULL;
    free(s);
  }
  FILE *f = fopen(p, "a");
  if (!f) return;
  if (fresh) {
    A.sessJa = ja;
    fprintf(f, "-- kore session — a valid .ano program: replay with anoc --run\n");
    fprintf(f, "--! registry session-base.reg\n");
    if (ja) fprintf(f, "--! ja\n");
  }
  if (ja != A.sessJa) fprintf(f, "-- (other surface, not replayable) %s\n", stmt);
  else fprintf(f, "%s\n", stmt);
  fclose(f);
}

/* def-head name of a submission (`def kin = …`, 定義 …), or "" — exact bytes, per
 * the def-as-program-variable rule */
static void def_head(const char *body, int ja, char *out, size_t outsz) {
  out[0] = 0;
  const char *kw = ja ? "定義 " : "def ";
  size_t kl = strlen(kw);
  if (strncmp(body, kw, kl)) return;
  const char *p = body + kl;
  while (*p == ' ') p++;
  size_t i = 0;
  while (p[i] && p[i] != ' ' && p[i] != '=' && i < outsz - 1) { out[i] = p[i]; i++; }
  out[i] = 0;
}

/* an existing session log rehydrates its defs, so a reopened world continues the
 * conversation exactly where the log left it — the log is the session's memory */
static void session_rehydrate(void) {
  for (int i = 0; i < A.nsdefs; i++) free(A.sdefText[i]);
  A.nsdefs = 0;
  char p[PATH_MAX + 16];
  session_path(p, sizeof p);
  size_t ln = 0;
  char *s = read_file(p, &ln);
  if (!s) return;
  int ja = strstr(s, "\n--! ja") != NULL;
  A.sessJa = ja;
  for (char *l = strtok(s, "\n"); l; l = strtok(NULL, "\n")) {
    while (*l == ' ' || *l == '\t') l++;
    if (!strncmp(l, "--", 2)) continue;
    char dh[128];
    def_head(l, ja, dh, sizeof dh);
    if (!dh[0]) continue;
    int found = -1;
    for (int i = 0; i < A.nsdefs; i++)
      if (!strcmp(A.sdefName[i], dh)) found = i;
    if (found >= 0) { free(A.sdefText[found]); A.sdefText[found] = xstrdup(l); }
    else if (A.nsdefs < 64) {
      A.sdefText[A.nsdefs] = xstrdup(l);
      A.sdefJa[A.nsdefs] = ja;
      snprintf(A.sdefName[A.nsdefs], sizeof A.sdefName[0], "%s", dh);
      A.nsdefs++;
    }
  }
  free(s);
}

static void repl_submit(void) {
  char stmt[1024];
  snprintf(stmt, sizeof stmt, "%s", A.prompt);
  if (!stmt[0]) return;
  if (A.nhist < KMAXHIST) A.hist[A.nhist++] = xstrdup(stmt);
  A.histAt = A.nhist;
  A.prompt[0] = 0;
  A.plen = A.pcur = 0;
  if (!A.world.loaded) { say("no world loaded"); return; }
  if (world_guard()) return;
  const char *body = stmt;
  int ja = 0;
  if (!strncmp(body, "ja ", 3)) { ja = 1; body += 3; }
  char absw[PATH_MAX];
  if (!realpath(A.world.path, absw)) snprintf(absw, sizeof absw, "%s", A.world.path);
  if (strchr(absw, ' ') || strchr(absw, '\t')) {
    say("world path contains a space — the --! registry directive is one word");
    return;
  }
  /* first statement of a session: snapshot the pre-state the log will replay against */
  char sess[PATH_MAX + 16];
  session_path(sess, sizeof sess);
  if (access(sess, F_OK) != 0) {
    char base[PATH_MAX + 24];
    snprintf(base, sizeof base, "%.*s-base.reg", (int)strlen(sess) - 4, sess);
    copy_file(A.world.path, base);
  }
  mkdirs(".kore");
  /* one submission, one program — with the session's defs prepended (same surface,
   * the resubmitted head excluded), so a def survives its submission exactly as the
   * session log replays it, and an installed rule beats once per later submission */
  char dh[128];
  def_head(body, ja, dh, sizeof dh);
  Buf prog = { 0 };
  bprintf(&prog, "--! registry %s\n%s", absw, ja ? "--! ja\n" : "");
  for (int i = 0; i < A.nsdefs; i++)
    if (A.sdefJa[i] == ja && (!dh[0] || strcmp(A.sdefName[i], dh)))
      bprintf(&prog, "%s\n", A.sdefText[i]);
  bprintf(&prog, "%s\n", body);
  if (write_commit(".kore/repl.ano", prog.s, prog.len)) { bfree(&prog); say("cannot write .kore/repl.ano"); return; }
  bfree(&prog);
  int seq = undo_push();
  if (seq < 0) { say("cannot stage undo copy"); return; }
  Buf cap = { 0 };
  char *argv[] = { (char *)find_anoc(), (char *)"--run", (char *)"--save", absw, (char *)".kore/repl.ano", NULL };
  int code = run_child(argv, &cap);
  bprintf(&A.outLog, "> %s\n", stmt);
  if (cap.len) logOut(cap.s, cap.len);
  if (code == 0) {
    char err[256], path[PATH_MAX];
    session_log(body, ja);
    if (dh[0]) {
      int found = -1;
      for (int i = 0; i < A.nsdefs; i++)
        if (A.sdefJa[i] == ja && !strcmp(A.sdefName[i], dh)) found = i;
      if (found >= 0) { free(A.sdefText[found]); A.sdefText[found] = xstrdup(body); }
      else if (A.nsdefs < 64) {
        A.sdefText[A.nsdefs] = xstrdup(body);
        A.sdefJa[A.nsdefs] = ja;
        snprintf(A.sdefName[A.nsdefs], sizeof A.sdefName[0], "%s", dh);
        A.nsdefs++;
      }
    }
    snprintf(path, sizeof path, "%s", A.world.path);
    if (world_load(&A.world, path, err, sizeof err)) say("%s", err);
    else say("world advanced · undo #%d staged · session logged", seq);
  } else {
    undo_drop();
    say("statement failed (exit %d) — the world stands", code);
  }
  bfree(&cap);
  A.outScroll = 0;
}

/* ---------- running a demo ---------- */

static void code_load(const char *path);

static int demo_registry(const char *anoPath, char *out, size_t outsz) {
  size_t len = 0;
  char *src = read_file(anoPath, &len);
  if (!src) return -1;
  int got = -1;
  for (char *ln = strtok(src, "\n"); ln; ln = strtok(NULL, "\n")) {
    while (*ln == ' ' || *ln == '\t') ln++;
    if (strncmp(ln, "--! registry ", 13)) continue;
    char *spec = ln + 13;
    while (*spec == ' ') spec++;
    char *e = spec + strlen(spec) - 1;
    while (e > spec && (*e == ' ' || *e == '\r')) *e-- = 0;
    char dir[PATH_MAX];
    snprintf(dir, sizeof dir, "%s", anoPath);
    char *sl = strrchr(dir, '/');
    if (sl) *sl = 0; else snprintf(dir, sizeof dir, ".");
    size_t sl2 = strlen(spec);
    int isPath = strchr(spec, '/') || (sl2 > 4 && !strcmp(spec + sl2 - 4, ".reg"));
    int wr;
    if (spec[0] == '/') wr = snprintf(out, outsz, "%s%s", spec, isPath ? "" : ".reg");
    else wr = snprintf(out, outsz, "%s/%s%s", dir, spec, isPath ? "" : ".reg");
    got = (wr > 0 && wr < (int)outsz) ? 0 : -1;
    break;
  }
  free(src);
  return got;
}

static void code_save(void) {
  if (!A.demoPath[0]) return;
  Buf b = { 0 };
  for (int i = 0; i < A.ncode; i++) { bput(&b, A.code[i], strlen(A.code[i])); bput(&b, "\n", 1); }
  if (write_commit(A.demoPath, b.s ? b.s : "", b.len)) say("cannot write %s", A.demoPath);
  else {
    A.codeDirty = 0;
    /* an explicit s on a corpus file is the author's call — but it is announced */
    say(in_demos(A.demoPath) ? "saved %s — corpus file edited in place" : "saved %s", A.demoPath);
  }
  bfree(&b);
}

/* r: the demo runs read-only — anoc --run, post-state saved to .kore/post.reg so the
 * world and space surfaces light up with what the program did; the original registry
 * is never written. Rerunning starts from the pristine world again. */
static void run_current(void) {
  if (A.mode == MODE_REG) { say("bare world: the prompt is the program (r runs demos)"); return; }
  if (!A.demoPath[0]) { say("no demo selected"); return; }
  /* never silently write the file under the author: saving is an explicit s */
  if (A.codeDirty) { say("unsaved code — s saves it, then r runs"); return; }
  mkdirs(".kore");
  Buf cap = { 0 };
  int code;
  char reg[PATH_MAX];
  int hasReg = demo_registry(A.demoPath, reg, sizeof reg) == 0;
  if (hasReg) {
    char *argv[] = { (char *)find_anoc(), (char *)"--run", (char *)"--save", (char *)".kore/post.reg", A.demoPath, NULL };
    code = run_child(argv, &cap);
  } else {
    char *argv[] = { (char *)find_anoc(), (char *)"--run", A.demoPath, NULL };
    code = run_child(argv, &cap);
  }
  bprintf(&A.outLog, "$ anoc --run %s\n", A.demoPath);
  if (cap.len) logOut(cap.s, cap.len);
  bfree(&cap);
  if (code == 0) {
    char err[256];
    if (hasReg && world_load(&A.world, ".kore/post.reg", err, sizeof err) == 0) {
      A.worldIsPost = 1;
      A.worldIsCopy = 0;
      say("pins held — world shows the post-state (pristine registry untouched)");
    } else say("pins held");
  } else say("run failed (exit %d) — see output", code);
  A.outScroll = 0;
}

static void open_demo(const char *path) {
  snprintf(A.demoPath, sizeof A.demoPath, "%s", path);
  code_load(path);
  char reg[PATH_MAX], err[256];
  A.worldIsCopy = A.worldIsPost = 0;
  A.undoSeq = 0;
  A.sessJa = -1;
  for (int i = 0; i < A.nsdefs; i++) free(A.sdefText[i]);
  A.nsdefs = 0;
  if (demo_registry(path, reg, sizeof reg) == 0) {
    snprintf(A.pristine, sizeof A.pristine, "%s", reg);
    if (world_load(&A.world, reg, err, sizeof err)) say("%s", err);
  } else {
    world_free(&A.world);
    A.pristine[0] = 0;
  }
  A.wSeg = A.wRow = A.wCol = A.wTop = 0;
  say("%s", path);
}

/* ---------- the code surface ---------- */

static void code_free(void) {
  for (int i = 0; i < A.ncode; i++) free(A.code[i]);
  free(A.code);
  A.code = NULL;
  A.ncode = 0;
}

static void code_load(const char *path) {
  code_free();
  size_t len = 0;
  char *src = read_file(path, &len);
  if (!src) {                       /* unreadable: an empty buffer, never a NULL one */
    A.code = xalloc(sizeof(char *));
    A.code[A.ncode++] = xstrdup("");
    A.ccy = A.ccx = A.codeTop = 0;
    A.codeDirty = A.codeInsert = 0;
    return;
  }
  int cap = 64;
  A.code = xalloc((size_t)cap * sizeof(char *));
  char *save = NULL;
  for (char *p = src;; p = NULL) {
    char *ln = p ? p : save;
    if (!ln) break;
    char *nl = strchr(ln, '\n');
    if (nl) { *nl = 0; save = nl + 1; } else save = NULL;
    if (!nl && !ln[0] && A.ncode) break;   /* the post-final-newline tail is not a line */
    size_t l = strlen(ln);
    if (l && ln[l - 1] == '\r') ln[l - 1] = 0;
    if (A.ncode >= cap) { cap *= 2; A.code = realloc(A.code, (size_t)cap * sizeof(char *)); if (!A.code) abort(); }
    A.code[A.ncode++] = xstrdup(ln);
    if (!nl) break;
  }
  free(src);
  if (!A.ncode) A.code[A.ncode++] = xstrdup("");
  A.ccy = A.ccx = A.codeTop = 0;
  A.codeDirty = A.codeInsert = 0;
}

/* byte position of display column x in a UTF-8 line (for cursor moves) */
static int line_byte_at(const char *s, int col) {
  int w = 0;
  const char *p = s;
  while (*p && w < col) {
    const char *q = p;
    w += cw(u8next(&q));
    p = q;
  }
  return (int)(p - s);
}

static void code_insert_str(const char *u8) {
  char *ln = A.code[A.ccy];
  int at = line_byte_at(ln, A.ccx);
  size_t ol = strlen(ln), il = strlen(u8);
  char *nl = xalloc(ol + il + 1);
  memcpy(nl, ln, (size_t)at);
  memcpy(nl + at, u8, il);
  memcpy(nl + at + il, ln + at, ol - (size_t)at + 1);
  free(A.code[A.ccy]);
  A.code[A.ccy] = nl;
  const char *p = u8;
  A.ccx += cw(u8next(&p));
  A.codeDirty = 1;
}

static void code_key(Ev *e) {
  char *ln = A.code[A.ccy];
  int lw = swidth(ln);
  if (A.codeInsert) {
    if (e->type == EV_KEY && e->key == K_ESC) { A.codeInsert = 0; return; }
    if (e->type == EV_KEY && e->key == K_ENTER) {
      int at = line_byte_at(ln, A.ccx);
      char *rest = xstrdup(ln + at);
      ln[at] = 0;
      A.code = realloc(A.code, (size_t)(A.ncode + 1) * sizeof(char *));
      if (!A.code) abort();
      memmove(&A.code[A.ccy + 2], &A.code[A.ccy + 1], (size_t)(A.ncode - A.ccy - 1) * sizeof(char *));
      A.code[A.ccy + 1] = rest;
      A.ncode++;
      A.ccy++;
      A.ccx = 0;
      A.codeDirty = 1;
      return;
    }
    if (e->type == EV_KEY && e->key == K_BS) {
      int at = line_byte_at(ln, A.ccx);
      if (at > 0) {
        int prev = at - 1;
        while (prev > 0 && ((unsigned char)ln[prev] & 0xC0) == 0x80) prev--;
        char tmp[8] = { 0 };
        memcpy(tmp, ln + prev, (size_t)(at - prev));
        const char *tp = tmp;
        A.ccx -= cw(u8next(&tp));
        memmove(ln + prev, ln + at, strlen(ln + at) + 1);
        A.codeDirty = 1;
      } else if (A.ccy > 0) {
        char *up = A.code[A.ccy - 1];
        A.ccx = swidth(up);
        char *joined = xalloc(strlen(up) + strlen(ln) + 1);
        strcpy(joined, up);
        strcat(joined, ln);
        free(A.code[A.ccy - 1]);
        free(A.code[A.ccy]);
        A.code[A.ccy - 1] = joined;
        memmove(&A.code[A.ccy], &A.code[A.ccy + 1], (size_t)(A.ncode - A.ccy - 1) * sizeof(char *));
        A.ncode--;
        A.ccy--;
        A.codeDirty = 1;
      }
      return;
    }
    if (e->type == EV_CHAR) { code_insert_str(e->u8); return; }
  }
  /* browse: vi-flavored */
  if (e->type == EV_CHAR) {
    switch (e->ch) {
      case 'j': A.ccy = A.ccy < A.ncode - 1 ? A.ccy + 1 : A.ccy; return;
      case 'k': A.ccy = A.ccy > 0 ? A.ccy - 1 : 0; return;
      case 'h': A.ccx = A.ccx > 0 ? A.ccx - 1 : 0; return;
      case 'l': A.ccx = A.ccx < lw ? A.ccx + 1 : lw; return;
      case '0': A.ccx = 0; return;
      case '$': A.ccx = lw; return;
      case 'i': A.codeInsert = 1; return;
      case 'a': A.codeInsert = 1; A.ccx = A.ccx < lw ? A.ccx + 1 : lw; return;
      case 'o': {
        A.code = realloc(A.code, (size_t)(A.ncode + 1) * sizeof(char *));
        if (!A.code) abort();
        memmove(&A.code[A.ccy + 2], &A.code[A.ccy + 1], (size_t)(A.ncode - A.ccy - 1) * sizeof(char *));
        A.code[A.ccy + 1] = xstrdup("");
        A.ncode++;
        A.ccy++;
        A.ccx = 0;
        A.codeInsert = 1;
        A.codeDirty = 1;
        return;
      }
      case 'x': {
        int at = line_byte_at(ln, A.ccx);
        if (ln[at]) {
          int next = at + 1;
          while (ln[next] && ((unsigned char)ln[next] & 0xC0) == 0x80) next++;
          memmove(ln + at, ln + next, strlen(ln + next) + 1);
          A.codeDirty = 1;
        }
        return;
      }
      case 'd': { /* dd: delete line (single d suffices here) */
        if (A.ncode > 1) {
          free(A.code[A.ccy]);
          memmove(&A.code[A.ccy], &A.code[A.ccy + 1], (size_t)(A.ncode - A.ccy - 1) * sizeof(char *));
          A.ncode--;
          if (A.ccy >= A.ncode) A.ccy = A.ncode - 1;
        } else { A.code[0][0] = 0; }
        A.codeDirty = 1;
        return;
      }
      case 's': code_save(); return;
    }
  }
  if (e->type == EV_KEY) {
    switch (e->key) {
      case K_UP: A.ccy = A.ccy > 0 ? A.ccy - 1 : 0; break;
      case K_DOWN: A.ccy = A.ccy < A.ncode - 1 ? A.ccy + 1 : A.ccy; break;
      case K_LEFT: A.ccx = A.ccx > 0 ? A.ccx - 1 : 0; break;
      case K_RIGHT: A.ccx = A.ccx < lw ? A.ccx + 1 : lw; break;
      case K_HOME: A.ccx = 0; break;
      case K_END: A.ccx = lw; break;
      case K_ENTER: A.codeInsert = 1; break;
      default: break;
    }
  }
  int nlw = swidth(A.code[A.ccy]);
  if (A.ccx > nlw) A.ccx = nlw;
}

/* ---------- world table geometry: segments, display columns, cells ---------- */

static const char *glyph_at(const Ent *e, int k);

/* A display column of the entity table (segment 0). */
typedef struct { Ent *e; int width; } DCol;
static DCol dcols[KMAXENT];
static int ndcols;

static void table_cols(void) {
  ndcols = 0;
  for (int i = 0; i < A.world.nents; i++) {
    Ent *e = &A.world.ents[i];
    if (e->kind == E_FIELD) continue;
    int w = swidth(e->name);
    int rows = e->kind == E_SREL ? e->nfib : (e->type == V_VEC ? e->nn / 2 : e->nn);
    for (int r = 0; r < rows; r++) {
      char cell[128];
      cell[0] = 0;
      if (e->kind == E_SREL && !e->isInv) {
        int l = 0;
        for (int j = 0; j < e->fibLen[r] && l < 100; j++)
          l += snprintf(cell + l, sizeof cell - (size_t)l, "%s%s", j ? " " : "", fmt_num(e->fibVals[e->fibOff[r] + j]));
      } else if (e->kind == E_SREL) snprintf(cell, sizeof cell, "(inv)");
      else if (e->type == V_VEC) snprintf(cell, sizeof cell, "%s,%s", fmt_num(e->nums[2 * r]), fmt_num(e->nums[2 * r + 1]));
      else if (e->type == V_SYM) snprintf(cell, sizeof cell, "%s", r < e->ns ? e->syms[r] : "");
      else if (e->type == V_CHAR) snprintf(cell, sizeof cell, "%s", glyph_at(e, r));
      else snprintf(cell, sizeof cell, "%s", r < e->nn ? fmt_num(e->nums[r]) : "");
      int cwd = swidth(cell);
      if (cwd > w) w = cwd;
    }
    if (w > 24) w = 24;
    if (w < 3) w = 3;
    dcols[ndcols].e = e;
    dcols[ndcols].width = w;
    ndcols++;
  }
}

static int nsegs(void) {
  int s = 1;
  for (int i = 0; i < A.world.nents; i++)
    if (A.world.ents[i].kind == E_FIELD) s++;
  return s;
}
static Ent *seg_field(int seg) {
  int s = 0;
  for (int i = 0; i < A.world.nents; i++)
    if (A.world.ents[i].kind == E_FIELD && ++s == seg) return &A.world.ents[i];
  return NULL;
}
static int seg_rows(int seg) {
  if (seg == 0) return A.world.n;
  return A.world.latH ? A.world.latH : 1;
}
static int seg_cols(int seg) {
  if (seg == 0) return ndcols;
  return A.world.latW ? A.world.latW : (seg_field(seg) ? seg_field(seg)->nn : 0);
}

/* cell text for the cursor/edit path (segment 0) */
static void table_cell(int row, int col, char *out, size_t outsz) {
  out[0] = 0;
  if (col < 0 || col >= ndcols) return;
  Ent *e = dcols[col].e;
  if (e->kind == E_SREL && !e->isInv) {
    if (row < e->nfib) {
      int l = 0;
      out[0] = 0;
      for (int j = 0; j < e->fibLen[row] && l < (int)outsz - 16; j++)
        l += snprintf(out + l, outsz - (size_t)l, "%s%s", j ? " " : "", fmt_num(e->fibVals[e->fibOff[row] + j]));
    }
  } else if (e->kind == E_SREL) snprintf(out, outsz, "(inv %s)", e->name);
  else if (e->type == V_VEC) { if (2 * row + 1 < e->nn) snprintf(out, outsz, "%s %s", fmt_num(e->nums[2 * row]), fmt_num(e->nums[2 * row + 1])); }
  else if (e->type == V_SYM) { if (row < e->ns) snprintf(out, outsz, "%s", e->syms[row]); }
  else if (e->type == V_CHAR) { if (e->chars && row < (int)strlen(e->chars)) snprintf(out, outsz, "%s", glyph_at(e, row)); }
  else if (row < e->nn) {
    if (e->kind == E_REL && e->nums[row] < 0) snprintf(out, outsz, "/");
    else snprintf(out, outsz, "%s", fmt_num(e->nums[row]));
  }
}

/* ---------- cell edit: splice the value into the .reg text ---------- */

/* Inputs: the focused cell, the typed replacement. Output: 0 / -1; the world file
 * advances through the undo ring + staged rename, then reloads. Accepts `/` for a
 * rel's none (-1). Numbers are validated; sym takes any word; char one byte. */
/* Inputs: world, char entry, cell index, cell count, one-byte replacement. Output:
 * 0 / -1 with err. The byte must be printable ASCII, and the run it produces must
 * still reload (run_ok) — kore never commits a world the loader rejects. */
static int char_splice(World *w, Ent *e, int cell, int rows, const char *repl, char *err, size_t errsz) {
  if (strlen(repl) != 1 || (unsigned char)repl[0] < 0x20 || (unsigned char)repl[0] >= 0x7F) {
    snprintf(err, errsz, "char cell wants one printable ASCII byte");
    return -1;
  }
  int off, len;
  if (char_span(w->lines[e->line], &off, &len)) { snprintf(err, errsz, "bad char line"); return -1; }
  if (cell < 0 || cell >= len || len != rows) { snprintf(err, errsz, "glyph out of range"); return -1; }
  char *cand = xalloc((size_t)len + 1);
  memcpy(cand, w->lines[e->line] + off, (size_t)len);
  cand[cell] = repl[0];
  int ok = run_ok(cand, rows);
  free(cand);
  if (!ok) {
    snprintf(err, errsz, "that run would have no .reg spelling (boundary space or word-boundary '#')");
    return -1;
  }
  return world_splice(w, e->line, off + cell, 1, repl, err, errsz);
}

static int cell_commit(const char *text, char *err, size_t errsz) {
  World *w = &A.world;
  char repl[512];
  snprintf(repl, sizeof repl, "%s", text);
  if (A.wSeg > 0) {
    Ent *e = seg_field(A.wSeg);
    if (!e) { snprintf(err, errsz, "no field segment"); return -1; }
    int cell = A.wRow * (w->latW ? w->latW : 1) + A.wCol;
    if (e->type == V_CHAR) return char_splice(w, e, cell, w->latW * w->latH, repl, err, errsz);
    double d;
    if (wnum(repl, &d)) { snprintf(err, errsz, "not a number: %.100s", repl); return -1; }
    int off, len;
    if (word_span(w->lines[e->line], 3 + cell, &off, &len)) { snprintf(err, errsz, "value out of range"); return -1; }
    return world_splice(w, e->line, off, len, repl, err, errsz);
  }
  if (A.wCol < 0 || A.wCol >= ndcols) { snprintf(err, errsz, "no column"); return -1; }
  Ent *e = dcols[A.wCol].e;
  int row = A.wRow;
  double d;
  switch (e->kind) {
    case E_COL:
      if (e->type == V_CHAR) return char_splice(w, e, row, w->n, repl, err, errsz);
      if (e->type == V_SYM) {
        if (!repl[0] || strchr(repl, ' ') || repl[0] == '#') { snprintf(err, errsz, "sym wants one word"); return -1; }
        int off, len;
        if (word_span(w->lines[e->line], 3 + row, &off, &len)) { snprintf(err, errsz, "row out of range"); return -1; }
        return world_splice(w, e->line, off, len, repl, err, errsz);
      }
      if (e->type == V_VEC) {
        double x, y;
        if (sscanf(repl, "%lf %lf", &x, &y) != 2) { snprintf(err, errsz, "vec cell wants: x y"); return -1; }
        /* one splice covering both pair words — the reload frees e, so never two */
        int o1, l1, o2, l2;
        if (word_span(w->lines[e->line], 3 + 3 * row, &o1, &l1) ||
            word_span(w->lines[e->line], 4 + 3 * row, &o2, &l2)) { snprintf(err, errsz, "row out of range"); return -1; }
        char pair[144];
        snprintf(pair, sizeof pair, "%s", fmt_num(x));
        snprintf(pair + strlen(pair), sizeof pair - strlen(pair), " %s", fmt_num(y));
        return world_splice(w, e->line, o1, o2 + l2 - o1, pair, err, errsz);
      }
      if (wnum(repl, &d)) { snprintf(err, errsz, "not a number: %.100s", repl); return -1; }
      { int off, len;
        if (word_span(w->lines[e->line], 3 + row, &off, &len)) { snprintf(err, errsz, "row out of range"); return -1; }
        return world_splice(w, e->line, off, len, repl, err, errsz); }
    case E_PRES: case E_ALIAS: {
      if (wnum(repl, &d)) { snprintf(err, errsz, "not a bit: %.100s", repl); return -1; }
      int off, len;
      if (word_span(w->lines[e->line], 2 + row, &off, &len)) { snprintf(err, errsz, "row out of range"); return -1; }
      return world_splice(w, e->line, off, len, repl, err, errsz);
    }
    case E_REL: {
      if (!strcmp(repl, "/")) snprintf(repl, sizeof repl, "-1"); /* the drawing's none */
      if (wnum(repl, &d)) { snprintf(err, errsz, "not a row index: %.100s", repl); return -1; }
      int off, len;
      if (word_span(w->lines[e->line], 2 + row, &off, &len)) { snprintf(err, errsz, "row out of range"); return -1; }
      return world_splice(w, e->line, off, len, repl, err, errsz);
    }
    case E_SREL: {
      if (e->isInv) { snprintf(err, errsz, "inv fibers derive from '%.100s' — edit the rel", e->inv); return -1; }
      /* replace fiber `row` wholesale: every word must parse as a number, or the
       * splice would not reload */
      char scratch[64];
      {
        char vcopy[512];
        snprintf(vcopy, sizeof vcopy, "%s", repl);
        char *vw[128];
        int vn = split_words(vcopy, vw, 128);
        for (int j = 0; j < vn; j++) {
          double dv;
          if (wnum(vw[j], &dv)) { snprintf(err, errsz, "fiber wants numbers: '%.60s'", vw[j]); return -1; }
        }
      }
      const char *ln = w->lines[e->line];
      int wi = 2, fib = 0, firstW = -1, lastW = -1;
      for (;; wi++) {
        int off, len;
        if (word_span(ln, wi, &off, &len)) break;
        snprintf(scratch, sizeof scratch, "%.*s", len < 63 ? len : 63, ln + off);
        if (!strcmp(scratch, "|")) { if (fib == row) break; fib++; continue; }
        if (fib == row) { if (firstW < 0) firstW = wi; lastW = wi; }
      }
      if (fib < row) { snprintf(err, errsz, "fiber out of range"); return -1; }
      if (firstW >= 0) {
        int o1, l1, o2, l2;
        word_span(ln, firstW, &o1, &l1);
        word_span(ln, lastW, &o2, &l2);
        return world_splice(w, e->line, o1, o2 + l2 - o1, repl[0] ? repl : "", err, errsz);
      }
      /* empty fiber: insert before its trailing '|', or at line end for the last */
      if (!repl[0]) return 0;
      int off, len;
      int sep = 2, f2 = 0, insAt = -1;
      for (;; sep++) {
        if (word_span(ln, sep, &off, &len)) break;
        snprintf(scratch, sizeof scratch, "%.*s", len < 63 ? len : 63, ln + off);
        if (!strcmp(scratch, "|")) { if (f2 == row) { insAt = off; break; } f2++; }
      }
      char ins[520];
      if (insAt < 0) { insAt = (int)strlen(ln); snprintf(ins, sizeof ins, " %s", repl); }
      else snprintf(ins, sizeof ins, "%s ", repl);
      return world_splice(w, e->line, insAt, 0, ins, err, errsz);
    }
    default: snprintf(err, errsz, "cell not editable"); return -1;
  }
}

static void cell_edit_commit(void) {
  A.editing = 0;
  char err[256];
  if (world_guard()) return;
  int seq = undo_push();
  if (seq < 0) { say("cannot stage undo copy"); return; }
  if (cell_commit(A.editBuf, err, sizeof err)) { undo_drop(); say("edit: %s", err); }
  else say("cell written · undo #%d staged", seq);
}

/* ---------- drag selection -> predicate skeleton ---------- */

/* Pointing at the world is the language's founding gesture: a drag paints rows or
 * cells and its predicate skeleton pre-fills the prompt, ready for the effect. */
static void drag_skeleton(void) {
  int r0 = A.dragR0 < A.dragR1 ? A.dragR0 : A.dragR1;
  int r1 = A.dragR0 < A.dragR1 ? A.dragR1 : A.dragR0;
  if (A.dragSeg == 0 && !A.spaceView) {
    if (r0 == r1) snprintf(A.prompt, sizeof A.prompt, "index == %d , ", r0);
    else snprintf(A.prompt, sizeof A.prompt, "index >= %d & index <= %d , ", r0, r1);
  } else if (A.world.latW) {
    int c0 = A.dragC0 < A.dragC1 ? A.dragC0 : A.dragC1;
    int c1 = A.dragC0 < A.dragC1 ? A.dragC1 : A.dragC0;
    /* registered x/y fields win, else the emitter's lattice frame: x the column,
     * y the row — sample the fields so either convention lands right */
    Ent *fx = world_ent(&A.world, "x", E_FIELD), *fy = world_ent(&A.world, "y", E_FIELD);
    int w = A.world.latW;
    double xlo = c0, xhi = c1, ylo = r0, yhi = r1;
    if (fx && fy && fx->nn > (r1 * w + c1) && fy->nn > (r1 * w + c1)) {
      xlo = fx->nums[r0 * w + c0]; xhi = fx->nums[r1 * w + c1];
      ylo = fy->nums[r0 * w + c0]; yhi = fy->nums[r1 * w + c1];
      if (xlo > xhi) { double t = xlo; xlo = xhi; xhi = t; }
      if (ylo > yhi) { double t = ylo; ylo = yhi; yhi = t; }
    }
    snprintf(A.prompt, sizeof A.prompt, "%d %d & x >= %s & x <= %s & y >= %s & y <= %s , ",
             A.world.latW, A.world.latH, fmt_num(xlo), fmt_num(xhi), fmt_num(ylo), fmt_num(yhi));
  } else {
    /* no lattice: positioned entities address through their pos pair fields */
    Ent *pos = world_pos(&A.world);
    if (!pos) { say("nothing to select here"); return; }
    int c0 = A.dragC0 < A.dragC1 ? A.dragC0 : A.dragC1;
    int c1 = A.dragC0 < A.dragC1 ? A.dragC1 : A.dragC0;
    snprintf(A.prompt, sizeof A.prompt, "%.100s.x >= %d & %.100s.x <= %d & %.100s.y >= %d & %.100s.y <= %d , ",
             pos->name, c0, pos->name, c1, pos->name, r0, pos->name, r1);
  }
  A.plen = (int)strlen(A.prompt);
  A.pcur = A.plen;
  A.focus = F_PROMPT;
  say("selection painted — finish the effect");
}

/* ---------- drawing the surfaces ---------- */

static void layout(void) {
  int W = T.cols, H = T.rows;
  int railW = A.mode == MODE_RAIL ? (W / 4 < 34 ? (W / 4 > 20 ? W / 4 : 20) : 34) : 0;
  int promptH = 1;
  int outH = H / 5 > 5 ? (H / 5 < 10 ? H / 5 : 10) : 5;
  int codeH = A.mode == MODE_REG ? 0 : (H - promptH - outH) * 2 / 5;
  A.rail = (Rect){ 0, 0, railW, H - promptH };
  int x = railW, w = W - railW;
  A.codeR = (Rect){ x, 0, w, codeH };
  A.worldR = (Rect){ x, codeH, w, H - promptH - outH - codeH };
  A.outR = (Rect){ x, H - promptH - outH, w, outH };
  A.promptR = (Rect){ 0, H - promptH, W, promptH };
}

static void draw_rail(void) {
  Rect r = A.rail;
  if (r.w <= 0) return;
  char t[64];
  snprintf(t, sizeof t, "demos %d", nDemos);
  box(r.x, r.y, r.w, r.h, t, A.focus == F_RAIL);
  int vis = r.h - 2;
  if (A.railSel < A.railTop) A.railTop = A.railSel;
  if (A.railSel >= A.railTop + vis) A.railTop = A.railSel - vis + 1;
  for (int i = 0; i < vis && A.railTop + i < nDemos; i++) {
    int di = A.railTop + i;
    const char *p = demoList[di];
    if (!strncmp(p, "demos/", 6)) p += 6;
    int sel = di == A.railSel;
    if (sel) fill(r.x + 1, r.y + 1 + i, r.w - 2, 1, " ", A_REV, 0);
    put(r.x + 2, r.y + 1 + i, sel ? A_REV : 0, 0, p, r.w - 3);
  }
}

static void draw_code(void) {
  Rect r = A.codeR;
  if (r.h <= 1) return;
  char t[PATH_MAX + 64];
  snprintf(t, sizeof t, "code · %s%s%s", A.demoPath[0] ? A.demoPath : "—",
           A.codeDirty ? " +" : "", A.codeInsert ? " · INSERT" : "");
  box(r.x, r.y, r.w, r.h, t, A.focus == F_CODE);
  int vis = r.h - 2;
  if (A.ccy < A.codeTop) A.codeTop = A.ccy;
  if (A.ccy >= A.codeTop + vis) A.codeTop = A.ccy - vis + 1;
  for (int i = 0; i < vis && A.codeTop + i < A.ncode; i++) {
    int li = A.codeTop + i;
    const char *ln = A.code[li];
    const char *lt = ln;
    while (*lt == ' ' || *lt == '\t') lt++;
    int dim = !strncmp(lt, "--", 2); /* comments and directives dim */
    int x = r.x + 1, y = r.y + 1 + i;
    const char *p = ln;
    int col = 0, maxw = r.w - 2;
    while (*p && col < maxw) {
      const char *at = p;
      uint32_t c = u8next(&p);
      int glow = !dim && (c == ',' || (c == '=' && *p == '>'));
      int glow2 = !dim && c == '>' && at > ln && at[-1] == '=';
      char g[8] = { 0 };
      memcpy(g, at, (size_t)(p - at) < 7 ? (size_t)(p - at) : 7);
      col += put(x + col, y, dim ? A_DIM : (glow || glow2) ? A_BOLD : 0, (glow || glow2) ? 93 : 0, g, maxw - col);
    }
    if (A.focus == F_CODE && li == A.ccy) {
      int cx = x + A.ccx;
      if (cx < r.x + r.w - 1) rev_cell(cx, y);
    }
  }
}

/* the space's board: the lattice when declared, else the positioned entities' box */
static void space_dims(int *gw, int *gh) {
  World *w = &A.world;
  *gw = w->latW;
  *gh = w->latH;
  Ent *pos = world_pos(w);
  if (!*gw && pos)
    for (int i = 0; i + 1 < pos->nn; i += 2) {
      double dx = pos->nums[i], dy = pos->nums[i + 1];
      if (dx >= 0 && dx < 4096 && (int)dx + 1 > *gw) *gw = (int)dx + 1;
      if (dy >= 0 && dy < 4096 && (int)dy + 1 > *gh) *gh = (int)dy + 1;
    }
}

/* a char cell's display: printable ASCII verbatim, anything else · — the run is
 * byte-per-cell, and a multi-byte glyph's fragments must never reach the terminal */
static const char *glyph_at(const Ent *e, int k) {
  static char g[2];
  if (!e->chars || k < 0 || k >= (int)strlen(e->chars)) return " ";
  unsigned char c = (unsigned char)e->chars[k];
  if (c < 0x20 || c >= 0x7F) return "·";
  g[0] = (char)c;
  g[1] = 0;
  return g;
}

/* shade glyph for a normalized value */
static const char *shade(double v, double max) {
  if (max <= 0) max = 1;
  double t = v / max;
  return t <= 0 ? "·" : t < 0.25 ? "░" : t < 0.5 ? "▒" : t < 0.75 ? "▓" : "█";
}
static const int fieldPal[] = { 33, 34, 31, 32, 35, 36, 93, 94, 91, 92 };

static void draw_space(Rect r) {
  World *w = &A.world;
  int gw, gh;
  space_dims(&gw, &gh);
  Ent *pos = world_pos(w);
  if (!gw || !gh) { put(r.x + 2, r.y + 1, A_DIM, 0, "no lattice, no positions — table only (m toggles back)", r.w - 4); return; }
  int ox = r.x + 2, oy = r.y + 1;
  /* fields paint in declaration order: char glyphs exact, bools as colored blocks,
   * nums shaded ░▒▓█; x/y coordinate fields skip (they would drown the picture) */
  for (int cy = 0; cy < gh && oy + cy < r.y + r.h - 1; cy++)
    for (int cx = 0; cx < gw && ox + cx < r.x + r.w - 1; cx++) {
      int k = cy * gw + cx;
      const char *g = "·";
      int fg = 0, attr = A_DIM, pi = 0;
      for (int i = 0; i < w->nents; i++) {
        Ent *e = &w->ents[i];
        if (e->kind != E_FIELD) continue;
        if (names_eq(e->name, "x") || names_eq(e->name, "y")) { pi++; continue; }
        if (e->type == V_CHAR) {
          if (e->chars && k < (int)strlen(e->chars) && e->chars[k] != '.') {
            g = glyph_at(e, k); fg = fieldPal[pi % 10]; attr = 0;
          }
        } else if (k < e->nn && e->nums[k] != 0) {
          if (e->type == V_BOOL) { g = "█"; fg = fieldPal[pi % 10]; attr = 0; }
          else {
            double max = 0;
            for (int j = 0; j < e->nn; j++) if (e->nums[j] > max) max = e->nums[j];
            g = shade(e->nums[k], max); fg = fieldPal[pi % 10]; attr = 0;
          }
        }
        pi++;
      }
      put(ox + cx, oy + cy, attr, fg, g, 1);
    }
  /* positioned entities stand on their cells */
  if (pos)
    for (int i = 0; i + 1 < pos->nn; i += 2) {
      double dx = pos->nums[i], dy = pos->nums[i + 1];
      if (dx < 0 || dx >= gw || dy < 0 || dy >= gh) continue;   /* guard before the cast */
      int px = (int)dx, py = (int)dy;
      if (ox + px < r.x + r.w - 1 && oy + py < r.y + r.h - 1)
        put(ox + px, oy + py, A_BOLD, 97, "@", 1);
    }
  /* the cell cursor works on the map exactly as on the table */
  if (A.focus == F_WORLD) {
    int cx = ox + A.wCol, cy = oy + A.wRow;
    if (A.wCol < gw && A.wRow < gh && cx < r.x + r.w - 1 && cy < r.y + r.h - 1)
      rev_cell(cx, cy);
  }
  if (A.dragging) {
    int rr0 = A.dragR0 < A.dragR1 ? A.dragR0 : A.dragR1, rr1 = A.dragR0 < A.dragR1 ? A.dragR1 : A.dragR0;
    int cc0 = A.dragC0 < A.dragC1 ? A.dragC0 : A.dragC1, cc1 = A.dragC0 < A.dragC1 ? A.dragC1 : A.dragC0;
    for (int yy = rr0; yy <= rr1 && yy < gh; yy++)
      for (int xx = cc0; xx <= cc1 && xx < gw; xx++)
        if (xx >= 0 && yy >= 0 && ox + xx < r.x + r.w - 1 && oy + yy < r.y + r.h - 1) rev_cell(ox + xx, oy + yy);
  }
}

/* virtual rows flatten the world surface — the entity table, then every field's
 * titled w×h block — so one scroll offset serves the whole surface and the cursor
 * is always brought on screen. */
typedef struct { int kind; int seg; int row; } VRow; /* kind: 0 table row, 1 field title, 2 field row, 3 blank */
static VRow vrows[32768];
static int nvrows;

static void world_vrows(void) {
  World *w = &A.world;
  nvrows = 0;
  for (int r = 0; r < w->n && nvrows < 32760; r++) vrows[nvrows++] = (VRow){ 0, 0, r };
  int seg = 1;
  for (int i = 0; i < w->nents; i++) {
    Ent *e = &w->ents[i];
    if (e->kind != E_FIELD) continue;
    int gh = w->latH ? w->latH : 1;
    if (nvrows + gh + 2 >= 32760) break;
    vrows[nvrows++] = (VRow){ 3, seg, 0 };
    vrows[nvrows++] = (VRow){ 1, seg, 0 };
    for (int gy = 0; gy < gh; gy++) vrows[nvrows++] = (VRow){ 2, seg, gy };
    seg++;
  }
}
static int cursor_vrow(void) {
  for (int i = 0; i < nvrows; i++)
    if (vrows[i].seg == A.wSeg && vrows[i].row == A.wRow && (vrows[i].kind == 0 || vrows[i].kind == 2)) return i;
  return 0;
}

static void field_cellw(Ent *e, int *cellW, int *pad) {
  *cellW = 1;
  *pad = e->type == V_CHAR ? 0 : 1;
  if (e->type != V_CHAR)
    for (int k = 0; k < e->nn; k++) {
      int l = (int)strlen(fmt_num(e->nums[k]));
      if (l > *cellW) *cellW = l;
    }
}

static void draw_world(void) {
  Rect r = A.worldR;
  if (r.h <= 1) return;
  World *w = &A.world;
  char t[PATH_MAX + 96];
  snprintf(t, sizeof t, "%s · %s%s%s · n %d", A.spaceView ? "space" : "world",
           w->loaded ? w->path : "—", A.worldIsPost ? " (post-state)" : "", A.worldIsCopy ? " (copy)" : "", w->n);
  if (w->latW) snprintf(t + strlen(t), sizeof t - strlen(t), " · %d×%d", w->latW, w->latH);
  box(r.x, r.y, r.w, r.h, t, A.focus == F_WORLD);
  if (!w->loaded) { put(r.x + 2, r.y + 1, A_DIM, 0, "no world — pick a demo or open a .reg", r.w - 4); return; }
  if (A.spaceView) { draw_space(r); return; }
  table_cols();
  world_vrows();
  if (A.wSeg == 0 && A.wCol >= ndcols) A.wCol = ndcols ? ndcols - 1 : 0;
  int x = r.x + 1, y = r.y + 1;
  int rowLblW = 4;
  /* the table header pins above the scroll */
  put(x, y, A_DIM, 0, "row", rowLblW);
  int cx = x + rowLblW + 1;
  for (int c = 0; c < ndcols && cx < r.x + r.w - 1; c++) {
    put(cx, y, A_BOLD | (dcols[c].e->kind == E_PRES ? A_DIM : 0), 96, dcols[c].e->name, dcols[c].width);
    cx += dcols[c].width + 1;
  }
  int vis = r.h - 3;
  if (vis < 1) vis = 1;
  int cv = cursor_vrow();
  if (cv < A.wTop) A.wTop = cv;
  if (cv >= A.wTop + vis) A.wTop = cv - vis + 1;
  if (A.wTop > nvrows - vis) A.wTop = nvrows - vis;
  if (A.wTop < 0) A.wTop = 0;
  for (int d = 0; d < vis && A.wTop + d < nvrows; d++) {
    VRow *vr = &vrows[A.wTop + d];
    int yy = y + 1 + d;
    if (vr->kind == 3) continue;
    if (vr->kind == 1) { put(x, yy, A_BOLD, 96, seg_field(vr->seg)->name, r.w - 2); continue; }
    if (vr->kind == 0) {
      int row = vr->row;
      char lbl[16];
      snprintf(lbl, sizeof lbl, "%d", row);
      put(x, yy, A_DIM, 0, lbl, rowLblW);
      cx = x + rowLblW + 1;
      for (int c = 0; c < ndcols && cx < r.x + r.w - 1; c++) {
        Ent *e = dcols[c].e;
        char cell[256];
        table_cell(row, c, cell, sizeof cell);
        int dim = 0;
        /* presence gaps visibly absent: a value under pres 0 dims to · */
        if (e->kind == E_COL) {
          Ent *pe = world_ent(w, e->name, E_PRES);
          if (pe && row < pe->nn && pe->nums[row] == 0) { snprintf(cell, sizeof cell, "·"); dim = 1; }
        }
        if (e->kind == E_REL && cell[0] == '/') dim = 1;
        int cur = A.wSeg == 0 && A.focus == F_WORLD && row == A.wRow && c == A.wCol;
        int inDrag = A.dragging && A.dragSeg == 0 &&
                     row >= (A.dragR0 < A.dragR1 ? A.dragR0 : A.dragR1) &&
                     row <= (A.dragR0 < A.dragR1 ? A.dragR1 : A.dragR0);
        if (cur && A.editing) {
          char eb[520];
          snprintf(eb, sizeof eb, "%s▏", A.editBuf);
          put(cx, yy, A_REV | A_BOLD, 93, eb, dcols[c].width);
        } else
          put(cx, yy, (cur || inDrag ? A_REV : 0) | (dim ? A_DIM : 0), e->type == V_SYM ? 95 : 0, cell, dcols[c].width);
        cx += dcols[c].width + 1;
      }
      continue;
    }
    /* field row: the lattice field in its w×h shape */
    Ent *e = seg_field(vr->seg);
    if (!e) continue;
    int gw = w->latW ? w->latW : e->nn;
    int gy = vr->row, cellW, pad;
    field_cellw(e, &cellW, &pad);
    for (int gx = 0; gx < gw; gx++) {
      int k = gy * gw + gx;
      char cell[64];
      if (e->type == V_CHAR) snprintf(cell, sizeof cell, "%s", glyph_at(e, k));
      else if (k < e->nn) snprintf(cell, sizeof cell, "%*s", cellW, fmt_num(e->nums[k]));
      else cell[0] = 0;
      int px = x + gx * (cellW + pad);
      if (px + cellW >= r.x + r.w - 1) break;
      int cur = A.focus == F_WORLD && A.wSeg == vr->seg && A.wRow == gy && A.wCol == gx;
      if (cur && A.editing) {
        char eb[520];
        snprintf(eb, sizeof eb, "%s▏", A.editBuf);
        put(px, yy, A_REV | A_BOLD, 93, eb, cellW + 1);
      } else
        put(px, yy, cur ? A_REV : (e->type == V_BOOL && k < e->nn && e->nums[k] == 0 ? A_DIM : 0), 0, cell, cellW + 1);
    }
  }
}

static void draw_out(void) {
  Rect r = A.outR;
  if (r.h <= 1) return;
  box(r.x, r.y, r.w, r.h, "output", A.focus == F_OUT);
  /* last lines of the log, minus the scrollback */
  int vis = r.h - 3;
  int nls = 0;
  for (size_t i = 0; i < A.outLog.len; i++) if (A.outLog.s[i] == '\n') nls++;
  int first = nls - vis - A.outScroll;
  if (first < 0) first = 0;
  int li = 0, yy = r.y + 1;
  const char *p = A.outLog.s ? A.outLog.s : "";
  while (*p && yy < r.y + r.h - 2) {
    const char *nl = strchr(p, '\n');
    size_t ll = nl ? (size_t)(nl - p) : strlen(p);
    if (li >= first) {
      char line[512];
      snprintf(line, sizeof line, "%.*s", (int)(ll < 500 ? ll : 500), p);
      put(r.x + 2, yy, 0, 0, line, r.w - 4);
      yy++;
    }
    li++;
    if (!nl) break;
    p = nl + 1;
  }
  /* the verdict line: pins held, the failure, or the save/undo status — unambiguous */
  put(r.x + 2, r.y + r.h - 2, A_BOLD, A.verdict[0] ? 92 : 0, A.verdict[0] ? A.verdict : "—", r.w - 4);
}

static void draw_prompt(void) {
  Rect r = A.promptR;
  int on = A.focus == F_PROMPT;
  fill(r.x, r.y, r.w, 1, " ", 0, 0);
  put(r.x, r.y, A_BOLD, on ? 93 : 0, ">", 1);
  put(r.x + 2, r.y, on ? A_BOLD : A_DIM, 0, A.prompt, r.w - 12);
  if (on) {
    char tmp[1024];
    snprintf(tmp, sizeof tmp, "%.*s", A.pcur, A.prompt); /* pcur is a byte offset */
    int cx = r.x + 2 + swidth(tmp);
    if (cx < r.w - 1) rev_cell(cx, r.y);
  }
  const char *hint = on ? "esc leave" : "tab focus · : prompt · m map · r run · u undo · q quit";
  int hw = swidth(hint);
  if (r.w - hw - 1 > 40) put(r.x + r.w - hw - 1, r.y, A_DIM, 0, hint, hw);
}

static void draw(void) {
  frame_clear();
  layout();
  draw_rail();
  draw_code();
  draw_world();
  draw_out();
  draw_prompt();
  flush_frame();
}

/* ---------- input dispatch ---------- */

static void prompt_key(Ev *e) {
  if (e->type == EV_KEY) {
    switch (e->key) {
      case K_ESC: A.focus = A.world.loaded ? F_WORLD : (A.mode == MODE_RAIL ? F_RAIL : F_CODE); return;
      case K_ENTER: repl_submit(); return;
      case K_BS:
        if (A.pcur > 0) {
          int prev = A.pcur - 1;
          while (prev > 0 && ((unsigned char)A.prompt[prev] & 0xC0) == 0x80) prev--;
          memmove(A.prompt + prev, A.prompt + A.pcur, strlen(A.prompt + A.pcur) + 1);
          A.plen -= A.pcur - prev;
          A.pcur = prev;
        }
        return;
      case K_LEFT:
        if (A.pcur > 0) { A.pcur--; while (A.pcur > 0 && ((unsigned char)A.prompt[A.pcur] & 0xC0) == 0x80) A.pcur--; }
        return;
      case K_RIGHT:
        if (A.pcur < A.plen) { A.pcur++; while (A.pcur < A.plen && ((unsigned char)A.prompt[A.pcur] & 0xC0) == 0x80) A.pcur++; }
        return;
      case K_HOME: A.pcur = 0; return;
      case K_END: A.pcur = A.plen; return;
      case K_UP:
        if (A.histAt > 0) {
          A.histAt--;
          snprintf(A.prompt, sizeof A.prompt, "%s", A.hist[A.histAt]);
          A.plen = A.pcur = (int)strlen(A.prompt);
        }
        return;
      case K_DOWN:
        if (A.histAt < A.nhist - 1) {
          A.histAt++;
          snprintf(A.prompt, sizeof A.prompt, "%s", A.hist[A.histAt]);
        } else { A.histAt = A.nhist; A.prompt[0] = 0; }
        A.plen = A.pcur = (int)strlen(A.prompt);
        return;
      default: return;
    }
  }
  if (e->type == EV_CHAR) {
    size_t il = strlen(e->u8);
    if ((size_t)A.plen + il < sizeof A.prompt - 1) {
      memmove(A.prompt + A.pcur + il, A.prompt + A.pcur, strlen(A.prompt + A.pcur) + 1);
      memcpy(A.prompt + A.pcur, e->u8, il);
      A.plen += (int)il;
      A.pcur += (int)il;
    }
  }
}

static void edit_key(Ev *e) {
  if (e->type == EV_KEY && e->key == K_ESC) { A.editing = 0; say("edit cancelled"); return; }
  if (e->type == EV_KEY && e->key == K_ENTER) { cell_edit_commit(); return; }
  if (e->type == EV_KEY && e->key == K_BS) {
    if (A.editLen > 0) {
      A.editLen--;
      while (A.editLen > 0 && ((unsigned char)A.editBuf[A.editLen] & 0xC0) == 0x80) A.editLen--;
      A.editBuf[A.editLen] = 0;
    }
    return;
  }
  if (e->type == EV_CHAR) {
    size_t il = strlen(e->u8);
    if ((size_t)A.editLen + il < sizeof A.editBuf - 1) {
      memcpy(A.editBuf + A.editLen, e->u8, il + 1);
      A.editLen += (int)il;
    }
  }
}

static void world_key(Ev *e) {
  if (A.editing) { edit_key(e); return; }
  World *w = &A.world;
  int sgw = 1, sgh = 1;
  if (A.spaceView) { space_dims(&sgw, &sgh); if (sgw < 1) sgw = 1; if (sgh < 1) sgh = 1; }
  int rows = A.spaceView ? sgh : seg_rows(A.wSeg);
  int cols = A.spaceView ? sgw : seg_cols(A.wSeg);
  int ch = e->type == EV_CHAR ? (int)e->ch : 0;
  int key = e->type == EV_KEY ? e->key : 0;
  if (ch == 'j' || key == K_DOWN) {
    if (A.wRow < rows - 1) A.wRow++;
    else if (!A.spaceView && A.wSeg < nsegs() - 1) { A.wSeg++; A.wRow = 0; A.wCol = 0; }
  } else if (ch == 'k' || key == K_UP) {
    if (A.wRow > 0) A.wRow--;
    else if (!A.spaceView && A.wSeg > 0) { A.wSeg--; A.wRow = seg_rows(A.wSeg) - 1; if (A.wRow < 0) A.wRow = 0; A.wCol = 0; }
  } else if (ch == 'h' || key == K_LEFT) { if (A.wCol > 0) A.wCol--; }
  else if (ch == 'l' || key == K_RIGHT) { if (A.wCol < cols - 1) A.wCol++; }
  else if (key == K_PGUP) { A.wRow -= 10; if (A.wRow < 0) A.wRow = 0; }
  else if (key == K_PGDN) { A.wRow += 10; if (A.wRow >= rows) A.wRow = rows ? rows - 1 : 0; }
  else if (key == K_ENTER) {
    if (!w->loaded) return;
    if (A.spaceView) {
      /* map edit: the field that painted this cell — the glyph you see is the value
       * you edit; a blank cell takes the first paintable field */
      int k = A.wRow * (w->latW ? w->latW : 1) + A.wCol;
      int s = 1, pick = 0, first = 0;
      for (int i = 0; i < w->nents; i++) {
        Ent *fe = &w->ents[i];
        if (fe->kind != E_FIELD) continue;
        if (!names_eq(fe->name, "x") && !names_eq(fe->name, "y")) {
          if (!first) first = s;
          int lit = fe->type == V_CHAR ? (fe->chars && k < (int)strlen(fe->chars) && fe->chars[k] != '.')
                                       : (k < fe->nn && fe->nums[k] != 0);
          if (lit) pick = s;
        }
        s++;
      }
      A.wSeg = pick ? pick : first;
      if (!A.wSeg) { say("no editable field on the map"); return; }
    }
    char cur[512];
    if (A.wSeg == 0) { table_cols(); table_cell(A.wRow, A.wCol, cur, sizeof cur); }
    else {
      Ent *fe = seg_field(A.wSeg);
      int k = A.wRow * (w->latW ? w->latW : 1) + A.wCol;
      if (fe && fe->type == V_CHAR) snprintf(cur, sizeof cur, "%s", fe->chars && k < (int)strlen(fe->chars) ? glyph_at(fe, k) : ".");
      else if (fe && k < fe->nn) snprintf(cur, sizeof cur, "%s", fmt_num(fe->nums[k]));
      else cur[0] = 0;
    }
    snprintf(A.editBuf, sizeof A.editBuf, "%s", cur);
    A.editLen = (int)strlen(A.editBuf);
    A.editing = 1;
    say("editing — enter commits, esc cancels");
  }
}

static void rail_key(Ev *e) {
  int ch = e->type == EV_CHAR ? (int)e->ch : 0;
  int key = e->type == EV_KEY ? e->key : 0;
  if ((ch == 'j' || key == K_DOWN) && A.railSel < nDemos - 1) A.railSel++;
  else if ((ch == 'k' || key == K_UP) && A.railSel > 0) A.railSel--;
  else if (key == K_PGDN) { A.railSel += 10; if (A.railSel >= nDemos) A.railSel = nDemos - 1; }
  else if (key == K_PGUP) { A.railSel -= 10; if (A.railSel < 0) A.railSel = 0; }
  else if (key == K_ENTER && nDemos) { open_demo(demoList[A.railSel]); A.focus = F_WORLD; }
}

static void editor_hop(void) {
  const char *ed = getenv("EDITOR");
  if (!ed || !*ed) ed = "vi";
  const char *file = A.focus == F_WORLD && A.world.loaded ? A.world.path : (A.demoPath[0] ? A.demoPath : A.world.path);
  if (!file || !*file) { say("nothing to edit"); return; }
  term_leave();
  /* the hop restores cooked mode, so ^C raises SIGINT for the whole group — the
   * editor owns it; kore must survive underneath */
  void (*oldInt)(int) = signal(SIGINT, SIG_IGN);
  void (*oldTerm)(int) = signal(SIGTERM, SIG_IGN);
  pid_t pid = fork();
  if (pid == 0) {
    signal(SIGINT, SIG_DFL);
    signal(SIGTERM, SIG_DFL);
    execlp(ed, ed, file, (char *)NULL);
    _exit(127);
  }
  int st = 0;
  waitpid(pid, &st, 0);
  signal(SIGINT, oldInt);
  signal(SIGTERM, oldTerm);
  term_enter();
  char err[256], path[PATH_MAX];
  if (A.focus == F_WORLD && A.world.loaded) {
    snprintf(path, sizeof path, "%s", A.world.path);
    if (world_load(&A.world, path, err, sizeof err)) say("%s", err);
    else say("reloaded %s", path);
  } else if (A.demoPath[0]) {
    code_load(A.demoPath);
    say("reloaded %s", A.demoPath);
  }
}

static void snapshot(void) {
  if (!A.world.loaded) { say("no world to snapshot"); return; }
  static int snapSeq;
  char dst[PATH_MAX + 48];
  snprintf(dst, sizeof dst, ".kore/%s-snap%d.reg", world_tag(), ++snapSeq);
  mkdirs(".kore");
  if (copy_file(A.world.path, dst)) say("snapshot failed");
  else say("snapshot → %s", dst);
}

/* mouse: everything the keys reach, a click reaches */
static int hit(Rect r, int x, int y) { return x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h; }

static void mouse_ev(Ev *e) {
  int x = e->mx, y = e->my;
  if (e->mkind == M_WHEELUP || e->mkind == M_WHEELDN) {
    int d = e->mkind == M_WHEELUP ? -3 : 3;
    if (hit(A.rail, x, y)) { A.railSel += d; if (A.railSel < 0) A.railSel = 0; if (A.railSel >= nDemos) A.railSel = nDemos ? nDemos - 1 : 0; }
    else if (hit(A.codeR, x, y)) { A.ccy += d; if (A.ccy < 0) A.ccy = 0; if (A.ccy >= A.ncode) A.ccy = A.ncode ? A.ncode - 1 : 0; }
    else if (hit(A.worldR, x, y)) {
      int rows, gw;
      if (A.spaceView) space_dims(&gw, &rows);
      else rows = seg_rows(A.wSeg);
      A.wRow += d;
      if (A.wRow < 0) A.wRow = 0;
      if (A.wRow >= rows) A.wRow = rows ? rows - 1 : 0;
    } else if (hit(A.outR, x, y)) { A.outScroll -= d; if (A.outScroll < 0) A.outScroll = 0; }
    return;
  }
  if (e->mkind == M_PRESS) {
    if (hit(A.promptR, x, y)) { A.focus = F_PROMPT; return; }
    if (hit(A.rail, x, y)) {
      A.focus = F_RAIL;
      int i = A.railTop + (y - A.rail.y - 1);
      if (i >= 0 && i < nDemos) {
        if (i == A.railSel) { open_demo(demoList[i]); A.focus = F_WORLD; }
        else A.railSel = i;
      }
      return;
    }
    if (hit(A.codeR, x, y)) {
      A.focus = F_CODE;
      int li = A.codeTop + (y - A.codeR.y - 1);
      if (li >= 0 && li < A.ncode) { A.ccy = li; A.ccx = x - A.codeR.x - 1; int lw = swidth(A.code[li]); if (A.ccx > lw) A.ccx = lw; }
      return;
    }
    if (hit(A.outR, x, y)) { A.focus = F_OUT; return; }
    if (hit(A.worldR, x, y)) {
      A.focus = F_WORLD;
      if (A.spaceView) {
        int gw, gh;
        space_dims(&gw, &gh);
        int cx = x - A.worldR.x - 2, cy = y - A.worldR.y - 1;
        if (cx >= 0 && cy >= 0 && cx < gw && cy < gh) {
          A.wCol = cx; A.wRow = cy;
          A.dragging = 1; A.dragSeg = 1;
          A.dragR0 = A.dragR1 = cy; A.dragC0 = A.dragC1 = cx;
        }
        return;
      }
      table_cols();
      world_vrows();
      int vi = A.wTop + (y - A.worldR.y - 2);
      if (vi >= 0 && vi < nvrows) {
        VRow *vr = &vrows[vi];
        if (vr->kind == 0) {
          A.wSeg = 0;
          A.wRow = vr->row;
          int cx = A.worldR.x + 1 + 5;
          for (int c = 0; c < ndcols; c++) {
            if (x >= cx && x < cx + dcols[c].width) { A.wCol = c; break; }
            cx += dcols[c].width + 1;
          }
          A.dragging = 1; A.dragSeg = 0;
          A.dragR0 = A.dragR1 = vr->row; A.dragC0 = A.dragC1 = A.wCol;
        } else if (vr->kind == 2) {
          Ent *fe = seg_field(vr->seg);
          int cellW, pad;
          if (fe) {
            field_cellw(fe, &cellW, &pad);
            int gx = (x - A.worldR.x - 1) / (cellW + pad);
            int gw = A.world.latW ? A.world.latW : fe->nn;
            if (gx >= 0 && gx < gw) { A.wSeg = vr->seg; A.wRow = vr->row; A.wCol = gx; }
          }
        }
      }
      return;
    }
  }
  if (e->mkind == M_DRAG && A.dragging) {
    if (A.spaceView) {
      int cx = x - A.worldR.x - 2, cy = y - A.worldR.y - 1;
      if (cx >= 0 && cy >= 0) { A.dragC1 = cx; A.dragR1 = cy; }
    } else {
      int row = A.wTop + (y - A.worldR.y - 2);
      if (row >= 0 && row < A.world.n) A.dragR1 = row;
    }
    return;
  }
  if (e->mkind == M_RELEASE && A.dragging) {
    A.dragging = 0;
    if (A.dragR0 != A.dragR1 || A.dragC0 != A.dragC1) drag_skeleton();
    return;
  }
}

static void handle(Ev *e) {
  if (e->type == EV_NONE) return;
  if (e->type == EV_MOUSE) { mouse_ev(e); return; }
  /* text-entry contexts swallow everything */
  if (A.focus == F_PROMPT) { prompt_key(e); return; }
  if (A.focus == F_WORLD && A.editing) { edit_key(e); return; }
  if (A.focus == F_CODE && A.codeInsert) { code_key(e); return; }
  /* global keys */
  if (e->type == EV_CHAR) {
    switch (e->ch) {
      case 'q': A.quit = 1; return;
      case ':': case '>': A.focus = F_PROMPT; return;
      case 'm': A.spaceView = !A.spaceView; A.wRow = A.wCol = 0; return;
      case 'r': run_current(); return;
      case 'u': undo_pop(); return;
      case 'w': snapshot(); return;
      case 'E': editor_hop(); return;
      default: break;
    }
  }
  if (e->type == EV_KEY && e->key == K_TAB) {
    enum Focus order[] = { F_RAIL, F_CODE, F_WORLD, F_OUT, F_PROMPT };
    int n = 5, at = 0;
    for (int i = 0; i < n; i++) if (order[i] == A.focus) at = i;
    for (int i = 1; i <= n; i++) {
      enum Focus f = order[(at + i) % n];
      if (f == F_RAIL && A.mode != MODE_RAIL) continue;
      if (f == F_CODE && A.mode == MODE_REG) continue;
      A.focus = f;
      break;
    }
    return;
  }
  switch (A.focus) {
    case F_RAIL: rail_key(e); break;
    case F_CODE: code_key(e); break;
    case F_WORLD: world_key(e); break;
    case F_OUT:
      if (e->type == EV_CHAR && e->ch == 'j' && A.outScroll > 0) A.outScroll--;
      if (e->type == EV_CHAR && e->ch == 'k') A.outScroll++;
      if (e->type == EV_KEY && e->key == K_DOWN && A.outScroll > 0) A.outScroll--;
      if (e->type == EV_KEY && e->key == K_UP) A.outScroll++;
      break;
    default: break;
  }
}

/* ---------- headless check: the verification hook ---------- */

/* Inputs: a .reg path. Output: 0 with a one-line report after loading and rendering
 * both views into memory, nonzero on any failure — `kore --check` over the corpus is
 * how "walks all 103 registries" becomes a script. */
static int check_reg(const char *path) {
  char err[256];
  if (world_load(&A.world, path, err, sizeof err)) { fprintf(stderr, "FAIL %s: %s\n", path, err); return 1; }
  table_cols();
  char cell[256];
  for (int r = 0; r < A.world.n; r++)
    for (int c = 0; c < ndcols; c++) table_cell(r, c, cell, sizeof cell);
  int fields = 0, space = A.world.latW > 0 || world_pos(&A.world) != NULL;
  for (int i = 0; i < A.world.nents; i++) if (A.world.ents[i].kind == E_FIELD) fields++;
  /* render the space to memory when it exists */
  if (space) {
    T.rows = 200; T.cols = 400;
    T.grid = xalloc((size_t)T.rows * T.cols * sizeof(Cell));
    frame_clear();
    A.worldR = (Rect){ 0, 0, 399, 199 };
    draw_space(A.worldR);
    free(T.grid);
    T.grid = NULL;
  }
  printf("ok %s n=%d lattice=%dx%d cols=%d fields=%d space=%s\n", path, A.world.n,
         A.world.latW, A.world.latH, ndcols, fields, space ? "yes" : "table-only");
  return 0;
}

/* Inputs: file.reg seg row col value. Output: 0 after splicing the cell exactly as the
 * inline edit does (guard and undo ring bypassed — point it at a copy), the touched
 * line printed; nonzero with the error printed. The other half of the --check hook:
 * edit → save → reload round-trips become a script. */
static int edit_reg(char **args) {
  char err[256];
  if (world_load(&A.world, args[0], err, sizeof err)) { fprintf(stderr, "FAIL %s: %s\n", args[0], err); return 1; }
  A.wSeg = atoi(args[1]);
  A.wRow = atoi(args[2]);
  A.wCol = atoi(args[3]);
  table_cols();
  if (cell_commit(args[4], err, sizeof err)) { fprintf(stderr, "FAIL %s: %s\n", args[0], err); return 1; }
  table_cols(); /* the reload rebuilt ents; re-point before printing */
  Ent *e = A.wSeg ? seg_field(A.wSeg) : (A.wCol < ndcols ? dcols[A.wCol].e : NULL);
  printf("ok %s\n", e ? A.world.lines[e->line] : "?");
  return 0;
}

/* ---------- entry ---------- */

int main(int argc, char **argv) {
  if (argc >= 3 && !strcmp(argv[1], "--check")) {
    int rc = 0;
    for (int i = 2; i < argc; i++) rc |= check_reg(argv[i]);
    return rc;
  }
  if (argc == 7 && !strcmp(argv[1], "--edit")) return edit_reg(argv + 2);
  const char *arg = argc > 1 ? argv[1] : NULL;
  if (arg && arg[0] == '-') {
    fprintf(stderr, "usage: kore [file.reg | file.ano]                    (bare: the demos rail)\n"
                    "       kore --check file.reg …                       (headless load+render report)\n"
                    "       kore --edit file.reg seg row col value        (headless cell splice)\n");
    return 2;
  }
  char err[256];
  if (!arg) {
    A.mode = MODE_RAIL;
    walk_demos("demos");
    qsort(demoList, (size_t)nDemos, sizeof *demoList, cmp_str);
    A.focus = F_RAIL;
    say("%d demos — enter opens, r runs, : prompts", nDemos);
  } else {
    size_t l = strlen(arg);
    if (l > 4 && !strcmp(arg + l - 4, ".reg")) {
      A.mode = MODE_REG;
      if (world_load(&A.world, arg, err, sizeof err)) { fprintf(stderr, "kore: %s\n", err); return 2; }
      undo_scan();
      session_rehydrate();
      A.focus = F_PROMPT;
      say("bare world — the prompt is the program");
    } else {
      A.mode = MODE_DEMO;
      if (access(arg, R_OK)) { fprintf(stderr, "kore: cannot read %s\n", arg); return 2; }
      open_demo(arg);
      A.focus = F_CODE;
    }
  }
  signal(SIGINT, on_fatal);
  signal(SIGTERM, on_fatal);
  signal(SIGSEGV, on_fatal);
  signal(SIGABRT, on_fatal);   /* xalloc/bput abort() must still restore the terminal */
  signal(SIGBUS, on_fatal);
  signal(SIGFPE, on_fatal);
  signal(SIGWINCH, on_winch);
  atexit(term_leave);
  if (term_enter()) { fprintf(stderr, "kore: not a terminal\n"); return 2; }
  while (!A.quit) {
    if (T.resized) { T.resized = 0; term_size(); }
    draw();
    Ev e = ev_read();
    handle(&e);
  }
  term_leave();
  return 0;
}
