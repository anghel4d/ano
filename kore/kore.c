/* kore.c — これ, the world at hand: the ano editor, a TUI over the anoc process and
 * text boundary. kore spawns anoc as a child exactly as anoc spawns cbqn; its data
 * contract is the .reg format. It never includes ano.h.
 *
 * Six surfaces (EDITOR.md): the demos rail, the code editor, the world table, the
 * space (the world as a glyph map, or a half-block bitmap), the output log, and the
 * prompt. Each REPL
 * submission is one program against the current world — anoc --run --save advances
 * the world file through the staged-rename commit loop; the undo ring is the loop's
 * free gift (pre-states are files under .kore/undo/).
 *
 * Zero external deps: raw ANSI CSI rendering + termios raw mode, double-buffered into
 * one write(2) per frame; SGR mouse reporting; SIGWINCH resize; CJK/kana/fullwidth
 * codepoints occupy 2 cells. Strings, collation, and the world arena come from
 * ../common (the anoptic strings module). kore's .reg reader is line-oriented: data
 * lines parse into tables for display, schema and unknown lines are preserved
 * verbatim — pass-through, never regeneration. Cell edits splice one word of .reg
 * text. Each parsed world lives in one arena and dies with the load that replaces it.
 *
 * Entry points: `kore <file.reg>` the bare world, REPL-only; `kore <file.ano>` the
 * demo form; bare `kore` the rail. Headless verification hooks: `kore --check
 * <file.reg>…` loads and renders every view (table, glyph map, bitmap) to memory
 * and reports; `kore --edit
 * <file.reg> <seg> <row> <col> <value>` performs one cell splice and prints the line.
 */
#define _GNU_SOURCE
#include <ctype.h>
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <math.h>
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

#include "anoptic_memory.h"
#include "anoptic_strings.h"
#include "anoptic_strings_utf.h"

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

typedef struct { int x, y, w, h; } Rect;

enum { A_DIM = 1, A_BOLD = 2, A_REV = 4 };

/* the palette: xterm-256 indexes. The ground is forced — pastel ink needs a dark
 * canvas, so every cell paints C_BG behind it and fg 0 means C_TEXT, never the
 * terminal's own colors (TODO.md: derive from the shell theme instead, as nvim does) */
enum {
  C_BG = 234, C_TEXT = 252,                     /* near-black canvas, soft off-white ink */
  C_FRAME = 245,                                /* unfocused borders */
  C_RAILC = 141, C_CODEC = 179, C_WORLDC = 110, /* per-panel accents: violet, gold, sky */
  C_OUTC = 108, C_PROMPTC = 114,                /* moss, green */
  C_OUTPUTSC = 218,                             /* rose — the outputs panel, beside moss */
  C_GLOW = 222, C_DIRECTIVE = 66, C_NUMLIT = 151, C_OP = 117, C_DEF = 216,
  C_SYM = 183, C_REL = 210, C_BOOL = 115, C_CHAR = 223, C_HDR = 117, C_ROWLBL = 242,
  C_OK = 114, C_ERR = 203, C_NIHONGO = 176, C_AT = 213, C_SEARCH = 220,
  C_CASEUP = 230, C_CASELO = 168,               /* cased glyphs: upper ivory, lower rose */
};

typedef struct { char g[8]; uint8_t attr, fg, bg, cont; } Cell; /* fg/bg: 0 default, else a palette index */

static struct {
  int rows, cols;
  Cell *grid;
  Buf out;
  struct termios saved;
  int rawOn, resized;
  int curX, curY, curShape; /* terminal cursor for this frame: DECSCUSR shape, 0 hidden */
} T;

/* Every exit path restores the terminal, the cursor shape, and the mouse state. Idempotent. */
static void term_leave(void) {
  if (!T.rawOn) return;
  T.rawOn = 0;
  const char *bye = "\x1b[?1002l\x1b[?1006l\x1b[?25h\x1b[0 q\x1b[?1049l\x1b[0m";
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
  if (T.rows < 12) T.rows = 12;
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
  T.curShape = 0;
  for (int i = 0; i < T.rows * T.cols; i++) {
    T.grid[i] = (Cell){ .g = " " };
  }
}

/* Inputs: cell coords, attributes, fg, bg (0: the canvas), UTF-8 text, max width
 * (-1: unbounded). Output: cells written, clipped to the grid; wide glyphs take two
 * cells, the second marked continuation. Returns the width consumed. */
static int putp(int x, int y, int attr, int fg, int bg, const char *s, int maxw) {
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
      cl->bg = (uint8_t)bg;
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
static int put(int x, int y, int attr, int fg, const char *s, int maxw) {
  return putp(x, y, attr, fg, 0, s, maxw);
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
        c->bg = 0;
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

/* Panel border with a title in the top rule; the focused panel glows in its accent. */
static void box(int x, int y, int w, int h, const char *title, int focused, int accent) {
  if (w < 2 || h < 2) return;
  int a = focused ? A_BOLD : A_DIM;
  int fg = focused ? accent : C_FRAME;
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

/* Inputs: box rect, first visible index, visible count, total count, accent. Output: a
 * thumb on the right border showing where the view sits — drawn only when the content
 * overflows the window, so a quiet panel keeps its plain rule. */
static void scrollbar(Rect r, int top, int vis, int total, int accent) {
  if (total <= vis || r.h <= 3 || vis <= 0) return;
  int track = r.h - 2;
  int thumb = vis * track / total;
  if (thumb < 1) thumb = 1;
  int maxTop = total - vis;
  int at = maxTop > 0 ? top * (track - thumb) / maxTop : 0;
  if (at > track - thumb) at = track - thumb;
  for (int j = 0; j < track; j++) {
    int on = j >= at && j < at + thumb;
    put(r.x + r.w - 1, r.y + 1 + j, on ? 0 : A_DIM, on ? accent : C_FRAME, on ? "┃" : "│", 1);
  }
}

/* One write(2) per frame: hide the cursor, home, emit rows with minimal SGR churn,
 * then park the terminal cursor — hidden, or shaped (DECSCUSR) at the frame's request. */
static void flush_frame(void) {
  Buf *o = &T.out;
  o->len = 0;
  bput(o, "\x1b[?25l\x1b[H", 9);
  int cattr = -1, cfg = -1, cbg = -1;
  for (int y = 0; y < T.rows; y++) {
    if (y) bput(o, "\r\n", 2);
    for (int x = 0; x < T.cols; x++) {
      Cell *c = &T.grid[y * T.cols + x];
      if (c->cont) continue;
      if (c->attr != cattr || c->fg != cfg || c->bg != cbg) {
        bprintf(o, "\x1b[0;48;5;%d%s%s%s", c->bg ? c->bg : C_BG, (c->attr & A_DIM) ? ";2" : "", (c->attr & A_BOLD) ? ";1" : "",
                (c->attr & A_REV) ? ";7" : "");
        bprintf(o, ";38;5;%d", c->fg ? c->fg : C_TEXT);
        bput(o, "m", 1);
        cattr = c->attr;
        cfg = c->fg;
        cbg = c->bg;
      }
      bput(o, c->g, strlen(c->g));
    }
  }
  bput(o, "\x1b[0m", 4);
  if (T.curShape) bprintf(o, "\x1b[%d;%dH\x1b[%d q\x1b[?25h", T.curY + 1, T.curX + 1, T.curShape);
  ssize_t r = write(1, o->s, o->len);
  (void)r;
}

/* ---------- input events ---------- */

enum { EV_NONE, EV_CHAR, EV_KEY, EV_MOUSE };
enum { K_UP = 1, K_DOWN, K_LEFT, K_RIGHT, K_ENTER, K_ESC, K_TAB, K_BS, K_DEL, K_HOME, K_END, K_PGUP, K_PGDN, K_NEWLINE, K_RESETALL };
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
    if (b2 == '\r' || b2 == '\n') { e.type = EV_KEY; e.key = K_NEWLINE; return e; }  /* Alt+Enter */
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
      case 'u': {                                    /* kitty CSI-u: modified Enter is a newline */
        int code = 0, mod = 0;
        sscanf(seq, "%d;%d", &code, &mod);
        if (code == 13) e.key = mod >= 2 ? K_NEWLINE : K_ENTER;
        else if ((code == 114 || code == 82) && mod == 6) e.key = K_RESETALL;  /* ctrl+shift+r */
        else e.type = EV_NONE;
        break;
      }
      case '~': {
        int code = atoi(seq);                        /* "15~" is F5, not Home */
        if (code == 27) {                            /* xterm modifyOtherKeys: "27;mod;13~" */
          int m1 = 0, mod = 0, key = 0;
          sscanf(seq, "%d;%d;%d", &m1, &mod, &key);
          if (key == 13) { e.key = mod >= 2 ? K_NEWLINE : K_ENTER; break; }
          if ((key == 114 || key == 82) && mod == 6) { e.key = K_RESETALL; break; }
        }
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
  int keyw;                 /* data word-index delta: +1 keyed rel/srel (`rel id mentor …`),
                               -1 `unique` (no type word) — the splice targets shift with it */
} Ent;

typedef struct {
  char path[PATH_MAX];
  ano_arena_t *heap;        /* every load's allocations live here and die together */
  char **lines; int nlines; /* the file's raw lines, verbatim — the one source of truth */
  int n, latW, latH;
  Ent ents[KMAXENT]; int nents;
  char posCol[KNAMESZ];     /* role pos target, else "" (falls back to literal `pos`) */
  char glyphCol[KNAMESZ];   /* role glyph target, else "" (falls back to literal `glyph`) */
  char protoCol[KNAMESZ];   /* role proto target, else "" (falls back to literal `proto`) */
  int loaded;
} World;

/* strdup into a world's arena; aborts on OOM like xalloc (the terminal restores) */
static char *adup(ano_arena_t *a, const char *s) {
  size_t n = strlen(s) + 1;
  char *p = ano_arena_alloc(a, n);
  if (!p) abort();
  memcpy(p, s, n);
  return p;
}

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
  /* the value domain is the finite doubles, exactly — anoc's loader refuses
     non-finites, so kore must never bless a world anoc would reject */
  return (end != w && *end == 0 && isfinite(*out)) ? 0 : -1;
}

static void world_free(World *w) {
  ano_arena_destroy(w->heap);   /* lines, line bytes, ent arrays, syms — one region */
  memset(w, 0, sizeof *w);
}

/* Inputs: a parsed world. Output: 0 / -1 with err naming the first repeated key —
 * the pairwise-distinct check anoc's loader runs on `unique` lines (keyw -1 marks
 * them here), so a duplicate-key world fails --check exactly as it fails anoc. */
static int world_check_unique(const World *w, char *err, size_t errsz) {
  for (int i = 0; i < w->nents; i++) {
    const Ent *e = &w->ents[i];
    if (e->kind != E_COL || e->keyw != -1) continue;
    for (int x = 0; x < e->nn; x++)
      for (int y = x + 1; y < e->nn; y++)
        if (e->nums[x] == e->nums[y]) {
          snprintf(err, errsz, "line %d: unique %s: value %g repeats (rows %d, %d)",
                   e->line + 1, e->name, e->nums[x], x, y);
          return -1;
        }
  }
  return 0;
}

/* Inputs: path. Output: 0 with the world parsed for display / -1 with err set.
 * Data lines (n, lattice, col, field, pres, rel, srel, inv) fill tables; everything
 * else — bind, alias, fn, role, as, ja, default, comments, unknown — passes through
 * untouched in lines[]. role pos/glyph/proto are noted for the space views. The whole parsed state
 * allocates from one arena and dies with the load that replaces it. */
static int world_load(World *w, const char *path, char *err, size_t errsz) {
  World fresh = { 0 };
  snprintf(fresh.path, sizeof fresh.path, "%s", path);
  size_t flen = 0;
  char *buf = read_file(path, &flen);
  if (!buf) { snprintf(err, errsz, "cannot read %s: %s", path, strerror(errno)); return -1; }
  fresh.heap = ano_arena_new(0);
  if (!fresh.heap) abort();
  int cap = 2;
  for (const char *p = buf; *p; p++) cap += *p == '\n';
  fresh.lines = ano_arena_alloc(fresh.heap, (size_t)cap * sizeof(char *));
  if (!fresh.lines) abort();
  char *save = NULL;
  for (char *p = buf;; p = NULL) {
    char *ln = p ? p : save;
    if (!ln) break;
    char *nl = strchr(ln, '\n');
    if (nl) { *nl = 0; save = nl + 1; } else save = NULL;
    /* the empty tail after a final newline is not a line — appending it would grow
     * the file by one blank line per commit cycle */
    if (!nl && !ln[0] && fresh.nlines) break;
    size_t l = strlen(ln);
    if (l && ln[l - 1] == '\r') ln[l - 1] = 0;
    fresh.lines[fresh.nlines++] = adup(fresh.heap, ln);
    if (!nl) break;
  }
  free(buf);
  for (int li = 0; li < fresh.nlines; li++) {
    /* tokenizing scratch comes from the world's own region: dead after this
     * iteration, bounded by ~2x the file, gone with the arena at the next load —
     * no free sites for an early continue to miss */
    char *dup = adup(fresh.heap, fresh.lines[li]);
    int maxw = (int)(strlen(dup) / 2 + 2);
    char **words = ano_arena_zalloc(fresh.heap, (size_t)maxw * sizeof(char *));
    if (!words) abort();
    int nw = split_words(dup, words, maxw);
    if (nw == 0) continue;
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
        e->nums = ano_arena_zalloc(fresh.heap, (size_t)(nw - 3 + 1) * sizeof(double));
        if (!e->nums) abort();
        for (int j = 3; j < nw; j++) {
          if (!strcmp(words[j], "|")) continue;
          if (!wnum(words[j], &e->nums[e->nn])) e->nn++;
        }
        fresh.nents++;
      } else if (!strcmp(ty, "sym")) {
        e->type = V_SYM;
        e->syms = ano_arena_zalloc(fresh.heap, (size_t)(nw - 3 + 1) * sizeof(char *));
        if (!e->syms) abort();
        for (int j = 3; j < nw; j++) e->syms[e->ns++] = adup(fresh.heap, words[j]);
        fresh.nents++;
      } else if (!strcmp(ty, "char")) {
        e->type = V_CHAR;
        int off, len;
        /* the run is the raw tail from the 4th word — the loader's own read */
        if (char_span(fresh.lines[li], &off, &len) == 0) {
          e->chars = ano_arena_zalloc(fresh.heap, (size_t)len + 1);
          if (!e->chars) abort();
          memcpy(e->chars, fresh.lines[li] + off, (size_t)len);
        } else e->chars = adup(fresh.heap, "");
        fresh.nents++;
      }
    } else if (!strcmp(k, "pres") && nw >= 2 && e) {
      e->kind = E_PRES;
      e->line = li;
      snprintf(e->name, sizeof e->name, "%.*s", KNAMESZ - 1, words[1]);
      e->type = V_BOOL;
      e->nums = ano_arena_zalloc(fresh.heap, (size_t)(nw - 2 + 1) * sizeof(double));
      if (!e->nums) abort();
      for (int j = 2; j < nw; j++) if (!wnum(words[j], &e->nums[e->nn])) e->nn++;
      fresh.nents++;
    } else if (!strcmp(k, "unique") && nw >= 2 && e) {
      /* declared injectivity: a num column with no type word — data starts one word early */
      e->kind = E_COL;
      e->type = V_NUM;
      e->keyw = -1;
      e->line = li;
      snprintf(e->name, sizeof e->name, "%.*s", KNAMESZ - 1, words[1]);
      e->nums = ano_arena_zalloc(fresh.heap, (size_t)(nw - 2 + 1) * sizeof(double));
      if (!e->nums) abort();
      for (int j = 2; j < nw; j++) if (!wnum(words[j], &e->nums[e->nn])) e->nn++;
      fresh.nents++;
    } else if ((!strcmp(k, "rel") || !strcmp(k, "alias")) && nw >= 2 && e) {
      /* an alias is a stored mask VALUE — data, so it displays and edits like a rel;
       * a keyed rel (`rel id mentor …`) carries its key column as one extra name */
      double kd;
      int keyed = k[0] == 'r' && nw >= 3 && wnum(words[2], &kd) && strcmp(words[2], "|");
      e->kind = k[0] == 'r' ? E_REL : E_ALIAS;
      e->keyw = keyed ? 1 : 0;
      e->line = li;
      snprintf(e->name, sizeof e->name, "%.*s", KNAMESZ - 1, words[1 + e->keyw]);
      e->type = k[0] == 'r' ? V_NUM : V_BOOL;
      e->nums = ano_arena_zalloc(fresh.heap, (size_t)(nw - 2 + 1) * sizeof(double));
      if (!e->nums) abort();
      for (int j = 2 + e->keyw; j < nw; j++) if (!wnum(words[j], &e->nums[e->nn])) e->nn++;
      fresh.nents++;
    } else if ((!strcmp(k, "srel") || !strcmp(k, "inv")) && nw >= 2 && e) {
      double kd;
      int keyed = k[0] == 's' && nw >= 3 && wnum(words[2], &kd) && strcmp(words[2], "|");
      e->kind = E_SREL;
      e->keyw = keyed ? 1 : 0;
      e->line = li;
      e->isInv = k[0] == 'i';
      if (e->isInv && nw >= 3) snprintf(e->inv, sizeof e->inv, "%.*s", KNAMESZ - 1, words[2]);
      snprintf(e->name, sizeof e->name, "%.*s", KNAMESZ - 1, words[1 + e->keyw]);
      if (!e->isInv) {
        int nfib = 1, nvals = 0;
        for (int j = 2 + e->keyw; j < nw; j++) !strcmp(words[j], "|") ? nfib++ : nvals++;
        e->fibOff = ano_arena_zalloc(fresh.heap, (size_t)nfib * sizeof(int));
        e->fibLen = ano_arena_zalloc(fresh.heap, (size_t)nfib * sizeof(int));
        e->fibVals = ano_arena_zalloc(fresh.heap, (size_t)(nvals + 1) * sizeof(double));
        if (!e->fibOff || !e->fibLen || !e->fibVals) abort();
        int fib = 0, vi = 0;
        for (int j = 2 + e->keyw; j < nw; j++) {
          if (!strcmp(words[j], "|")) { e->fibLen[fib] = vi - e->fibOff[fib]; fib++; e->fibOff[fib] = vi; }
          else if (!wnum(words[j], &e->fibVals[vi])) vi++;
        }
        e->fibLen[fib] = vi - e->fibOff[fib];
        e->nfib = fib + 1;
      }
      fresh.nents++;
    } else if (!strcmp(k, "role") && nw == 3 && !strcmp(words[1], "pos")) {
      snprintf(fresh.posCol, sizeof fresh.posCol, "%.*s", KNAMESZ - 1, words[2]);
    } else if (!strcmp(k, "role") && nw == 3 && !strcmp(words[1], "glyph")) {
      snprintf(fresh.glyphCol, sizeof fresh.glyphCol, "%.*s", KNAMESZ - 1, words[2]);
    } else if (!strcmp(k, "role") && nw == 3 && !strcmp(words[1], "proto")) {
      snprintf(fresh.protoCol, sizeof fresh.protoCol, "%.*s", KNAMESZ - 1, words[2]);
    }
    /* everything else: schema, preserved verbatim in lines[] */
  }
  if (world_check_unique(&fresh, err, errsz)) { world_free(&fresh); return -1; }
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

/* a sym column reached by role name, else by literal name — NULL when neither holds */
static Ent *world_sym_col(World *w, const char *role, const char *lit) {
  if (role[0]) {
    Ent *e = world_ent(w, role, E_COL);
    if (e && e->type == V_SYM) return e;
  }
  Ent *e = world_ent(w, lit, E_COL);
  return (e && e->type == V_SYM) ? e : NULL;
}
static Ent *world_glyph_col(World *w) { return world_sym_col(w, w->glyphCol, "glyph"); }
static Ent *world_proto_col(World *w) { return world_sym_col(w, w->protoCol, "proto"); }

/* entity ink: a stable palette color per archetype — the bool column's name hashes
 * (FNV-1a, ASCII case folded to match names_eq, so a def noun and its folded column
 * agree) into the palette; wheat stays wheat-colored on every load, in every view */
static const int entPal[] = { 114, 183, 210, 117, 221, 80, 213, 147, 84, 173, 152, 229 };
static int arch_color(const char *name) {
  uint32_t h = 2166136261u;
  for (const unsigned char *p = (const unsigned char *)name; *p; p++) {
    unsigned c = (*p >= 'A' && *p <= 'Z') ? *p + 32u : *p;
    h = (h ^ c) * 16777619u;
  }
  return entPal[h % (sizeof entPal / sizeof *entPal)];
}

/* Inputs: world, entity row, out[8]. Output: 1 with the entity's glyph — the glyph
 * role's column, else a column literally named glyph (clamped to its first rune: one
 * glyph, at most the 2-column cell), else the proto-role sym when the noun IS a
 * single rune — else 0, the bold-@ fallback. */
static int ent_glyph(World *w, int row, char out[8]) {
  Ent *e = world_glyph_col(w);
  const char *s = (e && row < e->ns) ? e->syms[row] : NULL;
  if (!s || !s[0]) {
    e = world_proto_col(w);
    s = (e && row < e->ns) ? e->syms[row] : NULL;
    if (!s || !s[0]) return 0;
    const char *q = s;
    u8next(&q);
    if (*q) return 0;             /* a multi-rune noun is a name, not a glyph */
  }
  const char *p = s;
  u8next(&p);
  size_t n = (size_t)(p - s) < 7 ? (size_t)(p - s) : 7;
  memcpy(out, s, n);
  out[n] = 0;
  return 1;
}

/* Inputs: world, entity row, its resolved glyph or NULL. Output: the entity's ink —
 * an ASCII-cased glyph takes the case color (the chess convention: case carries
 * side), else the first set archetype bool column's hashed color, else C_AT. */
static int ent_color(World *w, int row, const char *glyph) {
  if (glyph && !glyph[1]) {
    if (glyph[0] >= 'A' && glyph[0] <= 'Z') return C_CASEUP;
    if (glyph[0] >= 'a' && glyph[0] <= 'z') return C_CASELO;
  }
  for (int i = 0; i < w->nents; i++) {
    Ent *e = &w->ents[i];
    if (e->kind == E_COL && e->type == V_BOOL && row < e->nn && e->nums[row] != 0)
      return arch_color(e->name);
  }
  return C_AT;
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
  char *nl = ano_arena_alloc(w->heap, ol - (size_t)len + rl + 1);
  if (!nl) abort();
  memcpy(nl, old, (size_t)off);
  memcpy(nl + off, repl, rl);
  memcpy(nl + off + rl, old + off + len, ol - (size_t)off - (size_t)len + 1);
  w->lines[line] = nl;  /* the old line stays in the region until the reload below */
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
/* Natural collation: maximal ASCII digit runs compare as numbers (leading zeros
 * stripped; more significant digits = larger), the stretches between them by DUCET.
 * Returns <0/0/>0. Overflow-proof: digits compare by count then bytes, never a parse. */
static int isdig(char c) { return c >= '0' && c <= '9'; }
static int collate_natural(const char *x, size_t lx, const char *y, size_t ly) {
  size_t i = 0, j = 0;
  while (i < lx && j < ly) {
    int xd = isdig(x[i]), yd = isdig(y[j]);
    /* digit against non-digit: straight DUCET on the remainders decides */
    if (xd != yd) return anostr_collate(anostr_view(x + i, lx - i), anostr_view(y + j, ly - j));
    if (xd) {
      size_t si = i, sj = j;
      while (i < lx && isdig(x[i])) i++;
      while (j < ly && isdig(y[j])) j++;
      while (si + 1 < i && x[si] == '0') si++;
      while (sj + 1 < j && y[sj] == '0') sj++;
      size_t nx = i - si, ny = j - sj;
      if (nx != ny) return nx < ny ? -1 : 1;
      int c = memcmp(x + si, y + sj, nx);
      if (c) return c < 0 ? -1 : 1;
      /* equal value (01 vs 1): run on, the caller's byte tiebreak settles it */
    } else {
      size_t si = i, sj = j;
      while (i < lx && !isdig(x[i])) i++;
      while (j < ly && !isdig(y[j])) j++;
      int c = anostr_collate(anostr_view(x + si, i - si), anostr_view(y + sj, j - sj));
      if (c) return c;
    }
  }
  return i < lx ? 1 : j < ly ? -1 : 0;
}
/* Rail order is human order: natural collation (kana in gojuon, 2- before 10-, the
 * file-browser order) over the path with its .ano extension stripped — so a stem that
 * prefixes its own conjugate (01-x before 01-x-nihongo) sorts first, not after byte '-' < '.'. */
static int cmp_demo(const void *a, const void *b) {
  const char *x = *(char *const *)a, *y = *(char *const *)b;
  size_t lx = strlen(x), ly = strlen(y);
  int c = collate_natural(x, lx > 4 ? lx - 4 : lx, y, ly > 4 ? ly - 4 : ly);
  return c ? c : strcmp(x, y);
}

/* ---------- app state ---------- */

enum Focus { F_RAIL, F_CODE, F_WORLD, F_OUTPUTS, F_OUT, F_PROMPT };
enum Mode { MODE_RAIL, MODE_DEMO, MODE_REG };

/* one labeled query result; a run's records group under the step it staged */
typedef struct { char *label; char *value; } QRec;
typedef struct { int step; QRec *recs; int nrecs; } QGroup;

static struct App {
  enum Mode mode;
  enum Focus focus;
  World world;
  int worldIsCopy;              /* the world is the demo's .kore/play scratch */
  char pristine[PATH_MAX];      /* the demo's own registry (never mutated) */
  char demoPath[PATH_MAX];      /* the demo's identity: tags, sessions, anchors key off it */
  char demoLive[PATH_MAX + 64]; /* the file backing the code buffer — the corpus demo until the first save, its play copy after */
  char worldOrig[PATH_MAX];     /* MODE_REG: the corpus .reg the play copy shadows */
  int confirmReset;             /* >reset armed: y wipes the play tree, any other key cancels */
  char **code; int ncode;
  int codeDirty, codeInsert, ccy, ccx, codeTop;
  int codePending, codeG, codeD; /* vim state: count, gg chord, dd chord (count stored) */
  char search[128]; int searchLen, searching;
  int railSel, railTop;
  int spaceView;
  int wSeg, wRow, wCol, wTop;   /* world cursor: segment 0 = entity table, 1.. fields */
  int editing; char editBuf[512]; int editLen;
  int spcRet, spcRow, spcCol;   /* a space-view entity edit parked the cell cursor here */
  Buf outLog; int outScroll;    /* lines scrolled back from the tail */
  QGroup *qgroups; int nqgroups;/* the OUTPUTS store: tick-grouped query results */
  int outputsScroll;            /* lines scrolled down from the newest tick */
  char verdict[512]; int verdictBad;
  char prompt[1024]; int plen, pcur, pscroll, ptop; /* byte cursor, h-scroll, line scroll */
  char *hist[KMAXHIST]; int nhist, histAt;
  int dragging, dragSeg, dragR0, dragC0, dragR1, dragC1;
  int undoSeq;
  int trace;                    /* t: pass --trace to anoc — 0x1F diagnostic lines land in
                                   history (never OUTPUTS); off by default, never changes
                                   post-state */
  int sessJa;                                   /* the session log's surface; -1 unknown */
  char *sdefText[64]; int sdefJa[64]; char sdefName[64][128]; int nsdefs;
  int quit;
  Rect rail, codeR, worldR, outputsR, outR, promptR;
} A;

static void say(const char *fmt, ...) {
  va_list ap;
  va_start(ap, fmt);
  vsnprintf(A.verdict, sizeof A.verdict, fmt, ap);
  va_end(ap);
  A.verdictBad = 0;
}
/* say, but the verdict line draws in the error hue */
static void sayerr(const char *fmt, ...) {
  va_list ap;
  va_start(ap, fmt);
  vsnprintf(A.verdict, sizeof A.verdict, fmt, ap);
  va_end(ap);
  A.verdictBad = 1;
}
static void logOut(const char *s, size_t n) { bput(&A.outLog, s, n); A.outScroll = 0; }

/* ---------- the OUTPUTS store: labeled query results, tick-grouped ---------- */

#define KMAXQREC 256

/* the run's composed program (next.ano / repl.ano) as lines, kept for the run's
 * duration so a 0x1D tag q<N>@<L> resolves L (1-based) to its statement text */
static char **runLines;
static int nRunLines;

static void run_lines_set(const char *text) {
  for (int i = 0; i < nRunLines; i++) free(runLines[i]);
  free(runLines);
  nRunLines = 0;
  int cap = 2;
  for (const char *p = text; *p; p++) cap += *p == '\n';
  runLines = xalloc((size_t)cap * sizeof(char *));
  const char *p = text;
  while (*p) {
    const char *nl = strchr(p, '\n');
    size_t ll = nl ? (size_t)(nl - p) : strlen(p);
    char *l = xalloc(ll + 1);
    memcpy(l, p, ll);
    runLines[nRunLines++] = l;
    if (!nl) break;
    p = nl + 1;
  }
}

/* the 1-based program line, leading whitespace trimmed; NULL when out of range */
static const char *run_line(int ln) {
  if (ln < 1 || ln > nRunLines) return NULL;
  const char *s = runLines[ln - 1];
  while (*s == ' ' || *s == '\t') s++;
  return s;
}

static void qgroup_free(QGroup *g) {
  for (int i = 0; i < g->nrecs; i++) { free(g->recs[i].label); free(g->recs[i].value); }
  free(g->recs);
  memset(g, 0, sizeof *g);
}

static void outputs_clear(void) {
  for (int i = 0; i < A.nqgroups; i++) qgroup_free(&A.qgroups[i]);
  free(A.qgroups);
  A.qgroups = NULL;
  A.nqgroups = 0;
  A.outputsScroll = 0;
}

/* u's inverse of a push: every group whose step lies past the restored one goes */
static void outputs_drop_after(int seq) {
  while (A.nqgroups && A.qgroups[A.nqgroups - 1].step > seq)
    qgroup_free(&A.qgroups[--A.nqgroups]);
  A.outputsScroll = 0;
}

/* bounded: whole oldest groups drop past the record cap — no unbounded growth */
static void outputs_bound(void) {
  int total = 0, drop = 0;
  for (int i = 0; i < A.nqgroups; i++) total += A.qgroups[i].nrecs;
  while (drop < A.nqgroups - 1 && total > KMAXQREC) {
    total -= A.qgroups[drop].nrecs;
    qgroup_free(&A.qgroups[drop]);
    drop++;
  }
  if (drop) {
    memmove(A.qgroups, A.qgroups + drop, (size_t)(A.nqgroups - drop) * sizeof *A.qgroups);
    A.nqgroups -= drop;
  }
}

/* display lines the panel holds: a seam per group, a line per record, plus a
 * multi-line value's own lines indented beneath its label */
static int outputs_total_lines(void) {
  int t = 0;
  for (int i = 0; i < A.nqgroups; i++) {
    t += 1 + A.qgroups[i].nrecs;
    for (int j = 0; j < A.qgroups[i].nrecs; j++) {
      const char *v = A.qgroups[i].recs[j].value;
      if (!strchr(v, '\n')) continue;
      int n = 1;
      for (const char *p = v; *p; p++) n += *p == '\n';
      t += n;
    }
  }
  return t;
}

/* a byte-capped copy must not end mid-codepoint: drop a trailing partial sequence */
static void u8_tail_fix(char *s) {
  size_t n = strlen(s), k = n;
  while (k && ((unsigned char)s[k - 1] & 0xC0) == 0x80) k--;
  if (!k) return;
  unsigned char h = (unsigned char)s[k - 1];
  size_t need = h < 0xC0 ? 1 : h < 0xE0 ? 2 : h < 0xF0 ? 3 : 4;
  if (need > n - k + 1) s[k - 1] = 0;
}

/* take the buffer as a fresh string, trailing whitespace stripped; resets the buffer */
static char *buf_take_rstrip(Buf *b) {
  while (b->len && (b->s[b->len - 1] == '\n' || b->s[b->len - 1] == '\r' ||
                    b->s[b->len - 1] == ' ' || b->s[b->len - 1] == '\t'))
    b->s[--b->len] = 0;
  char *r = xstrdup(b->s ? b->s : "");
  b->len = 0;
  if (b->s) b->s[0] = 0;
  return r;
}

static char *label_dup(const char *s) {
  char *d = xstrdup(s);
  size_t l = strlen(d);
  while (l && (d[l - 1] == ' ' || d[l - 1] == '\t' || d[l - 1] == '\r')) d[--l] = 0;
  return d;
}

/* Inputs: the child's merged capture, its exit code, the step this run staged.
 * Output: on exit 0, one group pushed when it holds at least one record — a record-less
 * run (a mutation-only statement) leaves no group. A line opening with 0x1D starts a query record
 * (q<N>@<L>, L resolved through run_line to the statement text), following untagged
 * lines are that record's value block, and lines before any tag flow to the history
 * log; on nonzero exit the whole capture is history, exactly as before the split.
 * A line opening with 0x1F is a --trace diagnostic (one below the 0x1D label channel):
 * it goes to history verbatim, sentinel stripped, never into a value block and never
 * into OUTPUTS — even mid-record, so a trace line between a tag and its value cannot
 * contaminate the result. Values lose trailing whitespace; query records never enter
 * the history feed. */
static void cap_split(const char *s, size_t n, int code, int step) {
  if (!s) n = 0;
  if (code != 0) {
    /* the failure path logs the capture whole; trace sentinels still strip so the
     * diagnostic lines read clean beside the compiler error */
    Buf raw = { 0 };
    size_t at = 0;
    while (at < n) {
      size_t e = at;
      while (e < n && s[e] != '\n') e++;
      size_t st = at + ((e > at && (unsigned char)s[at] == 0x1F) ? 1 : 0);
      bput(&raw, s + st, e - st);
      bput(&raw, "\n", 1);
      at = e + 1;
    }
    if (raw.len) logOut(raw.s, raw.len);
    bfree(&raw);
    return;
  }
  A.qgroups = realloc(A.qgroups, (size_t)(A.nqgroups + 1) * sizeof *A.qgroups);
  if (!A.qgroups) abort();
  QGroup *g = &A.qgroups[A.nqgroups++];
  memset(g, 0, sizeof *g);
  g->step = step;
  Buf hist = { 0 }, val = { 0 };
  int open = 0;
  size_t i = 0;
  while (i < n) {
    size_t j = i;
    while (j < n && s[j] != '\n') j++;
    size_t ll = j - i;
    if (ll && (unsigned char)s[i] == 0x1F) {
      /* trace diagnostics ride to history whatever record is open */
      bput(&hist, s + i + 1, ll - 1);
      bput(&hist, "\n", 1);
    } else if (ll && (unsigned char)s[i] == 0x1D) {
      if (open) g->recs[g->nrecs - 1].value = buf_take_rstrip(&val);
      char tag[64];
      snprintf(tag, sizeof tag, "%.*s", (int)(ll - 1 < 63 ? ll - 1 : 63), s + i + 1);
      int qn = 0, ln = 0;
      const char *lbl = sscanf(tag, "q%d@%d", &qn, &ln) == 2 ? run_line(ln) : NULL;
      g->recs = realloc(g->recs, (size_t)(g->nrecs + 1) * sizeof *g->recs);
      if (!g->recs) abort();
      g->recs[g->nrecs].label = label_dup(lbl ? lbl : tag);
      g->recs[g->nrecs].value = NULL;
      g->nrecs++;
      open = 1;
    } else if (open) {
      bput(&val, s + i, ll);
      bput(&val, "\n", 1);
    } else {
      bput(&hist, s + i, ll);
      bput(&hist, "\n", 1);
    }
    i = j + 1;
  }
  if (open) g->recs[g->nrecs - 1].value = buf_take_rstrip(&val);
  bfree(&val);
  if (hist.len) logOut(hist.s, hist.len);
  bfree(&hist);
  /* a record-less run leaves no group: outputs_bound counts records alone, so empty
   * groups (mutation-only submissions) would otherwise stack seams without bound */
  if (g->nrecs == 0) { qgroup_free(g); A.nqgroups--; return; }
  outputs_bound();
  A.outputsScroll = 0;
}

/* ---------- .kore scratch: undo ring, play copies, repl program ---------- */

/* scratch names: <stem>-<8-hex FNV of the absolute path>, so two files sharing a
 * basename never share an undo ring, play directory, or snapshot series. */
static void tag_of(const char *path, char *out, size_t sz) {
  char rp[PATH_MAX], stem[KNAMESZ];
  const char *p = realpath(path, rp) ? rp : path;
  uint64_t h = 0xcbf29ce484222325u;
  for (const char *s = p; *s; s++) { h ^= (unsigned char)*s; h *= 0x100000001b3u; }
  const char *sl = strrchr(path, '/');
  snprintf(stem, sizeof stem, "%s", sl ? sl + 1 : path);
  char *dot = strrchr(stem, '.');
  if (dot) *dot = 0;
  snprintf(out, sz, "%s-%08x", stem, (unsigned)(h & 0xffffffffu));
}

static const char *world_tag(void) {
  static char key[PATH_MAX + 16];
  tag_of(A.world.path, key, sizeof key);
  return key;
}

/* the play scratch: .kore/play/<demo tag>/<registry basename> — the demo's own mutable
 * world, with its session log, base snapshot, and (by tag) undo ring beside it; keyed
 * by the DEMO's path, so two demos never share a session, and one demo's twins do. */
static int play_scratch(char *out, size_t sz) {
  const char *keyPath = A.demoPath[0] ? A.demoPath : A.world.path;
  const char *src = A.pristine[0] ? A.pristine : A.world.path;
  char tag[PATH_MAX + 16];
  tag_of(keyPath, tag, sizeof tag);
  const char *sl = strrchr(src, '/');
  return snprintf(out, sz, ".kore/play/%s/%s", tag, sl ? sl + 1 : src) < (int)sz ? 0 : -1;
}

/* the play code copy: .kore/play/<demo tag>/<demo basename> — the demo's own editable
 * .ano, created by the first mutating act (s, E); the corpus file is never a write target */
static int play_code(char *out, size_t sz) {
  char tag[PATH_MAX + 16];
  tag_of(A.demoPath, tag, sizeof tag);
  const char *sl = strrchr(A.demoPath, '/');
  return snprintf(out, sz, ".kore/play/%s/%s", tag, sl ? sl + 1 : A.demoPath) < (int)sz ? 0 : -1;
}

/* ensure the scratch's directory exists; path is the scratch file itself */
static void play_mkdir(const char *scratch) {
  char dir[PATH_MAX + 64];
  snprintf(dir, sizeof dir, "%s", scratch);
  char *sl = strrchr(dir, '/');
  if (sl) *sl = 0;
  mkdirs(dir);
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
/* drop the whole ring keyed by a play file's path — the file must still exist
 * (tag_of realpaths it), so wipe before the unlink that orphans the ring */
static void undo_wipe(const char *path) {
  char tag[PATH_MAX + 16], pre[PATH_MAX + 24];
  tag_of(path, tag, sizeof tag);
  int n = snprintf(pre, sizeof pre, "%s-", tag);
  DIR *d = opendir(".kore/undo");
  if (!d || n <= 0) { if (d) closedir(d); return; }
  struct dirent *de;
  while ((de = readdir(d))) {
    if (strncmp(de->d_name, pre, (size_t)n)) continue;
    char f[PATH_MAX + 64];
    snprintf(f, sizeof f, ".kore/undo/%s", de->d_name);
    unlink(f);
  }
  closedir(d);
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
  if (copy_file(p, A.world.path)) { sayerr("undo: cannot restore %s", p); return; }
  unlink(p);
  A.undoSeq--;
  outputs_drop_after(A.undoSeq);   /* the stepped-back tick's results go with it */
  char path[PATH_MAX];
  snprintf(path, sizeof path, "%s", A.world.path);
  if (world_load(&A.world, path, err, sizeof err)) sayerr("undo: %s", err);
  else say("undo → pre-state #%d restored (%d left)", A.undoSeq + 1, A.undoSeq);
}

static void session_rehydrate(void);

/* demos and their registries are immutable: mutation requires a copy, made once into
 * the demo's own play directory and announced in the output surface. The demo form
 * always copies (its registry is the demo's fixture); a bare .reg world mutates in
 * place unless it sits under demos/. */
static int world_guard(void) {
  if (A.worldIsCopy || (A.mode == MODE_REG && !in_demos(A.world.path))) return 0;
  char dst[PATH_MAX + 64], err[256];
  if (play_scratch(dst, sizeof dst)) { sayerr("play path overlong"); return -1; }
  play_mkdir(dst);
  if (copy_file(A.world.path, dst)) { sayerr("cannot copy world to %s", dst); return -1; }
  char msg[PATH_MAX + 128];
  int mn = snprintf(msg, sizeof msg, "world copied to %s — the corpus stays immutable\n", dst);
  logOut(msg, (size_t)mn);
  snprintf(A.worldOrig, sizeof A.worldOrig, "%s", A.world.path);  /* >reset restores this */
  char path[PATH_MAX + 64];
  snprintf(path, sizeof path, "%s", dst);
  if (world_load(&A.world, path, err, sizeof err)) { sayerr("%s", err); return -1; }
  A.worldIsCopy = 1;
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
  if (ja != A.sessJa) {
    /* every line comments out — a half-commented multi-line body would replay */
    char scopy[1024];
    snprintf(scopy, sizeof scopy, "%s", stmt);
    for (char *l = strtok(scopy, "\n"); l; l = strtok(NULL, "\n"))
      fprintf(f, "-- (other surface, not replayable) %s\n", l);
  } else fprintf(f, "%s\n", stmt);
  fclose(f);
}

/* does any line of body redefine this session def? A resubmitted head is excluded
 * from the prepend so the program holds exactly one copy. */
static int body_redefines(const char *body, int ja, const char *name);

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
  while (p[i] && p[i] != ' ' && p[i] != '=' && p[i] != '\n' && i < outsz - 1) { out[i] = p[i]; i++; }
  out[i] = 0;
}

static int body_redefines(const char *body, int ja, const char *name) {
  char bcopy[1024];
  snprintf(bcopy, sizeof bcopy, "%s", body);
  for (char *l = strtok(bcopy, "\n"); l; l = strtok(NULL, "\n")) {
    char dh[128];
    def_head(l, ja, dh, sizeof dh);
    if (dh[0] && !strcmp(dh, name)) return 1;
  }
  return 0;
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

static void kore_command(const char *cmd);

static void repl_submit(void) {
  char stmt[1024];
  snprintf(stmt, sizeof stmt, "%s", A.prompt);
  if (!stmt[0]) return;
  if (A.nhist < KMAXHIST) A.hist[A.nhist++] = xstrdup(stmt);
  A.histAt = A.nhist;
  A.prompt[0] = 0;
  A.plen = A.pcur = 0;
  if (stmt[0] == '>') { kore_command(stmt + 1); return; }
  if (!A.world.loaded) { sayerr("no world loaded"); return; }
  if (world_guard()) return;
  const char *body = stmt;
  int ja = 0;
  if (!strncmp(body, "ja ", 3)) { ja = 1; body += 3; }
  char absw[PATH_MAX];
  if (!realpath(A.world.path, absw)) snprintf(absw, sizeof absw, "%s", A.world.path);
  if (strchr(absw, ' ') || strchr(absw, '\t')) {
    sayerr("world path contains a space — the --! registry directive is one word");
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
   * any resubmitted head excluded), so a def survives its submission exactly as the
   * session log replays it, and an installed rule beats once per later submission.
   * The body may hold several lines (\⏎ or shift-enter at the prompt): one program. */
  Buf prog = { 0 };
  bprintf(&prog, "--! registry %s\n%s", absw, ja ? "--! ja\n" : "");
  for (int i = 0; i < A.nsdefs; i++)
    if (A.sdefJa[i] == ja && !body_redefines(body, ja, A.sdefName[i]))
      bprintf(&prog, "%s\n", A.sdefText[i]);
  bprintf(&prog, "%s\n", body);
  run_lines_set(prog.s ? prog.s : "");
  if (write_commit(".kore/repl.ano", prog.s, prog.len)) { bfree(&prog); sayerr("cannot write .kore/repl.ano"); return; }
  bfree(&prog);
  int seq = undo_push();
  if (seq < 0) { sayerr("cannot stage undo copy"); return; }
  Buf cap = { 0 };
  /* the trace slot repeats --label when tracing is off: a fixed argv, one flag flipped */
  char *argv[] = { (char *)find_anoc(), (char *)"--run", (char *)"--save", absw, (char *)"--label",
                   A.trace ? (char *)"--trace" : (char *)"--label", (char *)".kore/repl.ano", NULL };
  int code = run_child(argv, &cap);
  bprintf(&A.outLog, "> %s\n", stmt);
  cap_split(cap.s, cap.len, code, seq);
  if (code == 0) {
    char err[256], path[PATH_MAX];
    session_log(body, ja);
    /* every def line of the submission joins the session, exactly as rehydrate reads
     * the log back — a multi-line body tracks each of its defs individually */
    char bcopy[1024];
    snprintf(bcopy, sizeof bcopy, "%s", body);
    for (char *l = strtok(bcopy, "\n"); l; l = strtok(NULL, "\n")) {
      char dh[128];
      def_head(l, ja, dh, sizeof dh);
      if (!dh[0]) continue;
      int found = -1;
      for (int i = 0; i < A.nsdefs; i++)
        if (A.sdefJa[i] == ja && !strcmp(A.sdefName[i], dh)) found = i;
      if (found >= 0) { free(A.sdefText[found]); A.sdefText[found] = xstrdup(l); }
      else if (A.nsdefs < 64) {
        A.sdefText[A.nsdefs] = xstrdup(l);
        A.sdefJa[A.nsdefs] = ja;
        snprintf(A.sdefName[A.nsdefs], sizeof A.sdefName[0], "%s", dh);
        A.nsdefs++;
      }
    }
    snprintf(path, sizeof path, "%s", A.world.path);
    if (world_load(&A.world, path, err, sizeof err)) sayerr("%s", err);
    else say("world advanced · step %d staged · session logged", seq);
  } else {
    undo_drop();
    sayerr("statement failed (exit %d) — the world stands", code);
  }
  bfree(&cap);
  A.outScroll = 0;
}

/* ---------- running a demo ---------- */

static void code_load(const char *path);

/* the --! registry directive, read from anoPath (the live buffer's file) with relative
 * specs resolved against anchor's directory — a play copy keeps the corpus demo's home */
static int demo_registry(const char *anoPath, const char *anchor, char *out, size_t outsz) {
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
    snprintf(dir, sizeof dir, "%s", anchor);
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

/* corpus .ano files are never a write target: the first mutating act (s, E) copies the
 * demo into its play directory and retargets the buffer's backing file there — the code
 * surface's mirror of world_guard. A demo outside demos/ is the author's own file. */
static int code_guard(void) {
  if (!A.demoPath[0] || !in_demos(A.demoLive)) return 0;
  char dst[PATH_MAX + 64];
  if (play_code(dst, sizeof dst)) { sayerr("play path overlong"); return -1; }
  play_mkdir(dst);
  if (copy_file(A.demoLive, dst)) { sayerr("cannot copy code to %s", dst); return -1; }
  snprintf(A.demoLive, sizeof A.demoLive, "%s", dst);
  char msg[PATH_MAX + 128];
  int mn = snprintf(msg, sizeof msg, "code copied to %s — the corpus stays immutable\n", dst);
  logOut(msg, (size_t)mn);
  return 0;
}

static void code_save(void) {
  if (!A.demoPath[0]) return;
  if (code_guard()) return;
  Buf b = { 0 };
  for (int i = 0; i < A.ncode; i++) { bput(&b, A.code[i], strlen(A.code[i])); bput(&b, "\n", 1); }
  if (write_commit(A.demoLive, b.s ? b.s : "", b.len)) sayerr("cannot write %s", A.demoLive);
  else {
    A.codeDirty = 0;
    say("saved %s", A.demoLive);
  }
  bfree(&b);
}

/* a one-line seam in the session log: the world moved by something the log cannot
 * replay (a tick, a reset), recorded where the statements live */
static void session_seam(const char *fmt, const char *arg) {
  char sess[PATH_MAX + 16];
  session_path(sess, sizeof sess);
  if (access(sess, F_OK) != 0) return;
  FILE *f = fopen(sess, "a");
  if (!f) return;
  fprintf(f, fmt, arg);
  fclose(f);
}

/* adopt the demo's play scratch as the world: load it, resume its ring and session.
 * copyFirst copies the pristine registry over it beforehand (creation / reset). */
static int play_adopt(const char *dst, int copyFirst, char *err, size_t errsz) {
  play_mkdir(dst);
  if (copyFirst && copy_file(A.pristine, dst)) { snprintf(err, errsz, "cannot copy %.100s to %.100s", A.pristine, dst); return -1; }
  char path[PATH_MAX + 64];
  snprintf(path, sizeof path, "%s", dst);
  if (world_load(&A.world, path, err, errsz)) return -1;
  if (!A.worldIsCopy) {
    A.worldIsCopy = 1;
    A.sessJa = -1;
    undo_scan();
    session_rehydrate();
  }
  return 0;
}

/* r: load and reset — the pristine registry copied over the demo's play scratch, the
 * pre-reset scratch staged on the ring first so u steps back across a reset. The
 * pristine file itself is only ever read. In a bare world r reloads the file. */
static void world_reset(void) {
  char err[256];
  if (A.mode == MODE_REG) {
    if (!A.world.loaded) { sayerr("no world loaded"); return; }
    char path[PATH_MAX];
    snprintf(path, sizeof path, "%s", A.world.path);
    if (world_load(&A.world, path, err, sizeof err)) sayerr("%s", err);
    else { outputs_clear(); say("world reloaded from %s", path); }
    return;
  }
  if (!A.demoPath[0]) { sayerr("no demo selected"); return; }
  if (!A.pristine[0]) { sayerr("this demo declares no registry — n runs it for the output"); return; }
  char dst[PATH_MAX + 64];
  if (play_scratch(dst, sizeof dst)) { sayerr("play path overlong"); return; }
  int had = access(dst, F_OK) == 0;
  if (had) {
    if (!A.worldIsCopy && play_adopt(dst, 0, err, sizeof err)) { sayerr("%s", err); return; }
    /* stage the pre-reset state, unless it already equals the pristine bytes */
    size_t la = 0, lb = 0;
    char *sa = read_file(A.pristine, &la), *sb = read_file(dst, &lb);
    int same = sa && sb && la == lb && memcmp(sa, sb, la) == 0;
    free(sa);
    free(sb);
    if (!same) {
      if (undo_push() < 0) { sayerr("cannot stage undo copy"); return; }
      session_seam("-- r: world reset to pristine %s (not replayable)\n", A.pristine);
    }
  }
  if (play_adopt(dst, 1, err, sizeof err)) { sayerr("%s", err); return; }
  outputs_clear();
  A.wSeg = A.wRow = A.wCol = A.wTop = 0;
  if (had) say("reset → pristine world (n steps it, u steps back)");
  else say("pristine world loaded → %s (n steps it)", dst);
}

/* a --! expect / expect-n / out pin, matched as anoc tokenizes it (src/main.c dir_line:
 * any spaces/tabs after --!, then the key word) — so --!out and "--!  expect" count too.
 * Input: one line, leading whitespace trimmed. Output: 1 pin / 0 not. */
static int pin_line(const char *lt) {
  if (strncmp(lt, "--!", 3)) return 0;
  const char *p = lt + 3;
  while (*p == ' ' || *p == '\t') p++;
  size_t k = 0;
  while (p[k] && p[k] != ' ' && p[k] != '\t' && p[k] != '\r') k++;
  return (k == 3 && !strncmp(p, "out", 3)) ||
         (k == 6 && !strncmp(p, "expect", 6)) ||
         (k == 8 && !strncmp(p, "expect-n", 8));
}

/* the tick program: the demo verbatim, its --! registry retargeted at absw (the play
 * scratch) and its --! expect / --! out pins stripped — the pins witness the pristine
 * run, and against any later step they would fail the tick and hold the world still.
 * absw NULL is the registry-less demo: no world ever steps, so pins can never go stale
 * and every one is kept and enforced. Writes .kore/next.ano and keeps the composed
 * lines so a labeled query tag resolves to its statement text. 0 / -1 with the verdict said. */
static int tick_program(const char *absw) {
  size_t slen = 0;
  char *src = read_file(A.demoLive, &slen);
  if (!src) { sayerr("cannot read %s", A.demoLive); return -1; }
  Buf prog = { 0 };
  int retargeted = 0;
  char *save = NULL;
  for (char *p = src;; p = NULL) {
    char *ln = p ? p : save;
    if (!ln) break;
    char *nl = strchr(ln, '\n');
    if (nl) { *nl = 0; save = nl + 1; } else save = NULL;
    if (!nl && !ln[0]) break;
    const char *lt = ln;
    while (*lt == ' ' || *lt == '\t') lt++;
    if (absw && !retargeted && !strncmp(lt, "--! registry ", 13)) {
      bprintf(&prog, "--! registry %s\n", absw);
      retargeted = 1;
    } else if (absw && pin_line(lt)) {
      /* dropped: the tick program is scratch, never written back to the demo */
    } else bprintf(&prog, "%s\n", ln);
    if (!nl) break;
  }
  free(src);
  if (absw && !retargeted) { bfree(&prog); sayerr("no --! registry line in %s", A.demoLive); return -1; }
  run_lines_set(prog.s ? prog.s : "");
  if (write_commit(".kore/next.ano", prog.s ? prog.s : "", prog.len)) { bfree(&prog); sayerr("cannot write .kore/next.ano"); return -1; }
  bfree(&prog);
  return 0;
}

/* n: next — one tick: the demo's program, its --! registry retargeted at the play
 * scratch, run through --run --save with the pre-state staged on the ring first.
 * r resets to step zero; u is n's exact inverse. */
static void world_next(void) {
  if (A.mode == MODE_REG) { sayerr("bare world: statements step it — n steps demos"); return; }
  if (!A.demoPath[0]) { sayerr("no demo selected"); return; }
  /* never silently write the file under the author: saving is an explicit s */
  if (A.codeDirty) { sayerr("unsaved code — s saves it, then n steps"); return; }
  mkdirs(".kore");
  Buf cap = { 0 };
  int code;
  if (!A.pristine[0]) {
    /* no registry: nothing to advance — the demo runs verbatim, pins kept: a world
     * that never steps can never stale them, so they stay the witness */
    if (tick_program(NULL)) return;
    char *argv[] = { (char *)find_anoc(), (char *)"--run", (char *)"--label",
                     A.trace ? (char *)"--trace" : (char *)"--label", (char *)".kore/next.ano", NULL };
    code = run_child(argv, &cap);
    bprintf(&A.outLog, "$ anoc --run %s\n", A.demoLive);
    cap_split(cap.s, cap.len, code, 0);
    bfree(&cap);
    if (code == 0) say("pins held (no registry — no world to step)");
    else sayerr("run failed (exit %d) — see output", code);
    A.outScroll = 0;
    return;
  }
  if (!A.worldIsCopy) {
    char dst[PATH_MAX + 64], err[256];
    if (play_scratch(dst, sizeof dst)) { sayerr("play path overlong"); return; }
    if (play_adopt(dst, access(dst, F_OK) != 0, err, sizeof err)) { sayerr("%s", err); return; }
  }
  char absw[PATH_MAX];
  if (!realpath(A.world.path, absw)) snprintf(absw, sizeof absw, "%s", A.world.path);
  if (strchr(absw, ' ') || strchr(absw, '\t')) {
    sayerr("world path contains a space — the --! registry directive is one word");
    return;
  }
  if (tick_program(absw)) return;
  int seq = undo_push();
  if (seq < 0) { sayerr("cannot stage undo copy"); return; }
  char *argv[] = { (char *)find_anoc(), (char *)"--run", (char *)"--save", absw, (char *)"--label",
                   A.trace ? (char *)"--trace" : (char *)"--label", (char *)".kore/next.ano", NULL };
  code = run_child(argv, &cap);
  bprintf(&A.outLog, "$ n — %s against %s\n", A.demoPath, A.world.path);
  cap_split(cap.s, cap.len, code, seq);
  bfree(&cap);
  if (code == 0) {
    char err[256], path[PATH_MAX];
    snprintf(path, sizeof path, "%s", A.world.path);
    session_seam("-- n: %s ticked the world (not replayable)\n", A.demoPath);
    if (world_load(&A.world, path, err, sizeof err)) sayerr("%s", err);
    else say("tick — world advanced · step %d · u steps back", seq);
  } else {
    undo_drop();
    sayerr("tick failed (exit %d) — the world stands", code);
  }
  A.outScroll = 0;
}

static void open_demo(const char *path) {
  snprintf(A.demoPath, sizeof A.demoPath, "%s", path);
  /* an earlier session's play code copy shadows a corpus demo — the buffer rides it */
  char live[PATH_MAX + 64];
  int shadowed = in_demos(path) && play_code(live, sizeof live) == 0 && access(live, F_OK) == 0;
  snprintf(A.demoLive, sizeof A.demoLive, "%s", shadowed ? live : path);
  code_load(A.demoLive);
  if (shadowed) {
    char msg[PATH_MAX + 128];
    int mn = snprintf(msg, sizeof msg, "code: play copy %s resumed — >reset restores the corpus\n", A.demoLive);
    logOut(msg, (size_t)mn);
  }
  char reg[PATH_MAX], err[256];
  A.worldIsCopy = 0;
  A.undoSeq = 0;
  A.sessJa = -1;
  for (int i = 0; i < A.nsdefs; i++) free(A.sdefText[i]);
  A.nsdefs = 0;
  outputs_clear();
  if (demo_registry(A.demoLive, A.demoPath, reg, sizeof reg) == 0) {
    snprintf(A.pristine, sizeof A.pristine, "%s", reg);
    char dst[PATH_MAX + 64];
    /* an earlier session left a play world: resume it — r resets to pristine */
    if (play_scratch(dst, sizeof dst) == 0 && access(dst, F_OK) == 0 &&
        play_adopt(dst, 0, err, sizeof err) == 0) {
      A.wSeg = A.wRow = A.wCol = A.wTop = 0;
      say("%s — play world resumed at step %d (r resets to pristine)", path, A.undoSeq);
      return;
    }
    if (world_load(&A.world, reg, err, sizeof err)) sayerr("%s", err);
  } else {
    world_free(&A.world);
    A.pristine[0] = 0;
  }
  A.wSeg = A.wRow = A.wCol = A.wTop = 0;
  say("%s", path);
}

/* ---------- >reset: the whole play tree back to the pristine corpus ---------- */

static int play_count(void) {
  int n = 0;
  DIR *d = opendir(".kore/play");
  if (!d) return 0;
  struct dirent *de;
  while ((de = readdir(d))) if (de->d_name[0] != '.') n++;
  closedir(d);
  return n;
}

/* confirmed: unlink every play copy, its undo ring, and the tick scratch — every demo
 * returns to the pristine corpus, including the open one. Snapshots are deliberate; they stay. */
static void reset_all(void) {
  int demos = 0, files = 0;
  DIR *d = opendir(".kore/play");
  if (d) {
    struct dirent *de;
    while ((de = readdir(d))) {
      if (de->d_name[0] == '.') continue;
      char dir[PATH_MAX + 64];
      snprintf(dir, sizeof dir, ".kore/play/%s", de->d_name);
      DIR *pd = opendir(dir);
      if (!pd) continue;
      struct dirent *pe;
      while ((pe = readdir(pd))) {
        if (pe->d_name[0] == '.') continue;
        char f[PATH_MAX + 512];
        snprintf(f, sizeof f, "%s/%s", dir, pe->d_name);
        undo_wipe(f);
        if (!unlink(f)) files++;
      }
      closedir(pd);
      if (!rmdir(dir)) demos++;
    }
    closedir(d);
  }
  rmdir(".kore/play");
  unlink(".kore/next.ano");
  outputs_clear();
  if (A.demoPath[0]) {          /* a demo is open — rail mode or demo mode alike */
    char keep[PATH_MAX];
    snprintf(keep, sizeof keep, "%s", A.demoPath);
    open_demo(keep);
  } else if (A.mode == MODE_REG && A.worldIsCopy && A.worldOrig[0]) {
    char err[256];
    A.worldIsCopy = 0;
    A.sessJa = -1;
    for (int i = 0; i < A.nsdefs; i++) free(A.sdefText[i]);
    A.nsdefs = 0;
    if (world_load(&A.world, A.worldOrig, err, sizeof err)) { sayerr("%s", err); return; }
    undo_scan();
    A.wSeg = A.wRow = A.wCol = A.wTop = 0;
  }
  say("reset — %d play cop%s removed (%d file%s); every demo is the pristine corpus again",
      demos, demos == 1 ? "y" : "ies", files, files == 1 ? "" : "s");
}

/* the prompt's > form: kore's own verbs, not ano statements */
static void kore_command(const char *cmd) {
  while (*cmd == ' ') cmd++;
  if (!strcmp(cmd, "reset")) {
    int n = play_count();
    if (!n) { say("nothing to reset — no play copies exist"); return; }
    A.confirmReset = 1;
    sayerr("reset %d play cop%s to the pristine corpus? y confirms — any other key cancels",
           n, n == 1 ? "y" : "ies");
    return;
  }
  sayerr("unknown command >%.60s — commands: >reset", cmd);
}

/* ---------- the code surface ---------- */

static void code_undo_clear(void);

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
  code_undo_clear();
  A.codePending = A.codeG = A.codeD = 0;
  A.searching = 0;
  A.search[0] = 0;
  A.searchLen = 0;
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

/* display column of byte offset `at` — line_byte_at's inverse */
static int line_col_of(const char *s, int at) {
  int w = 0;
  const char *p = s;
  while (*p && (int)(p - s) < at) w += cw(u8next(&p));
  return w;
}

/* ---------- vim vocabulary: word motions, the code undo stack, /-search ---------- */

static int rune_is_word(uint32_t c) { return c == '_' || anorune_is_letter(c) || anorune_is_digit(c); }

/* w: leave the current run (word runes or a punct run), skip whitespace, land on the
 * next head; line ends wrap. b: the mirror, landing on the previous run's head. */
static void code_word_fwd(void) {
  const char *ln = A.code[A.ccy];
  int at = line_byte_at(ln, A.ccx);
  const char *p = ln + at;
  if (!*p) {
    if (A.ccy < A.ncode - 1) { A.ccy++; A.ccx = 0; }
    return;
  }
  const char *q = p;
  uint32_t c = u8next(&q);
  if (!anorune_is_whitespace(c)) {
    int cls = rune_is_word(c);
    p = q;                                   /* past the head rune */
    while (*p) {
      q = p;
      c = u8next(&q);
      if (anorune_is_whitespace(c) || rune_is_word(c) != cls) break;
      p = q;
    }
  }
  while (*p) {
    q = p;
    c = u8next(&q);
    if (!anorune_is_whitespace(c)) break;
    p = q;
  }
  if (!*p) {
    if (A.ccy < A.ncode - 1) { A.ccy++; A.ccx = 0; } else A.ccx = swidth(ln);
    return;
  }
  A.ccx = line_col_of(ln, (int)(p - ln));
}

static void code_word_back(void) {
  const char *ln = A.code[A.ccy];
  int at = line_byte_at(ln, A.ccx);
  if (at == 0) {
    if (A.ccy > 0) { A.ccy--; A.ccx = swidth(A.code[A.ccy]); }
    return;
  }
  anostr_t s = anostr_view(ln, strlen(ln));
  size_t i = (size_t)at;
  anorune_t c = anostr_rune_prev(s, &i);
  while (i > 0 && anorune_is_whitespace(c)) c = anostr_rune_prev(s, &i);
  int cls = rune_is_word(c);
  while (i > 0) {
    size_t j = i;
    anorune_t d = anostr_rune_prev(s, &j);
    if (anorune_is_whitespace(d) || rune_is_word(d) != cls) break;
    i = j;
  }
  A.ccx = line_col_of(ln, (int)i);
}

/* code-local undo: whole-buffer snapshots (u here never touches the world's ring) */
#define KCUNDO 64
static struct { char *text; int cy, cx; } cundo[KCUNDO];
static int ncundo;

static void code_undo_clear(void) {
  for (int i = 0; i < ncundo; i++) free(cundo[i].text);
  ncundo = 0;
}
static void code_undo_push(void) {
  if (ncundo == KCUNDO) {
    free(cundo[0].text);
    memmove(&cundo[0], &cundo[1], (KCUNDO - 1) * sizeof cundo[0]);
    ncundo--;
  }
  Buf b = { 0 };
  for (int i = 0; i < A.ncode; i++) { bput(&b, A.code[i], strlen(A.code[i])); bput(&b, "\n", 1); }
  cundo[ncundo].text = b.s ? b.s : xstrdup("");
  cundo[ncundo].cy = A.ccy;
  cundo[ncundo].cx = A.ccx;
  ncundo++;
}
static void code_undo_pop(void) {
  if (!ncundo) { say("code: nothing to undo"); return; }
  ncundo--;
  code_free();
  char *src = cundo[ncundo].text;
  int cap = 64;
  A.code = xalloc((size_t)cap * sizeof(char *));
  char *save = NULL;
  for (char *p = src;; p = NULL) {
    char *ln = p ? p : save;
    if (!ln) break;
    char *nl = strchr(ln, '\n');
    if (nl) { *nl = 0; save = nl + 1; } else save = NULL;
    if (!nl && !ln[0]) break;
    if (A.ncode >= cap) { cap *= 2; A.code = realloc(A.code, (size_t)cap * sizeof(char *)); if (!A.code) abort(); }
    A.code[A.ncode++] = xstrdup(ln);
    if (!nl) break;
  }
  if (!A.ncode) A.code[A.ncode++] = xstrdup("");
  free(cundo[ncundo].text);
  A.ccy = cundo[ncundo].cy < A.ncode ? cundo[ncundo].cy : A.ncode - 1;
  A.ccx = cundo[ncundo].cx;
  int lw = swidth(A.code[A.ccy]);
  if (A.ccx > lw) A.ccx = lw;
  A.codeDirty = 1;
  say("code undo (%d left)", ncundo);
}

/* the / search: base-letter matching (case- and accent-insensitive), n/N walk it */
static void code_search_jump(int dir) {
  if (!A.search[0]) { say("no search — / sets one"); return; }
  anostr_t needle = anostr_view(A.search, (size_t)A.searchLen);
  int total = A.ncode;
  for (int step = 0; step <= total; step++) {
    int li = ((A.ccy + dir * step) % total + total) % total;
    const char *ln = A.code[li];
    size_t ll = strlen(ln);
    anostr_t hay = anostr_view(ln, ll);
    if (dir > 0) {
      size_t from = 0;
      if (step == 0) {
        from = (size_t)line_byte_at(ln, A.ccx) + 1;
        if (from > ll) continue;
      }
      size_t at = anostr_find_base(hay, needle, from);
      if (at != ANOSTR_NPOS) { A.ccy = li; A.ccx = line_col_of(ln, (int)at); say("/%s", A.search); return; }
    } else {
      size_t limit = step == 0 ? (size_t)line_byte_at(ln, A.ccx) : ll + 1;
      size_t best = ANOSTR_NPOS, at = 0, f;
      while ((f = anostr_find_base(hay, needle, at)) != ANOSTR_NPOS && f < limit) { best = f; at = f + 1; }
      if (best != ANOSTR_NPOS) { A.ccy = li; A.ccx = line_col_of(ln, (int)best); say("?%s", A.search); return; }
    }
  }
  sayerr("no match: %s", A.search);
}

/* keys while typing the / pattern */
static void search_key(Ev *e) {
  if (e->type == EV_KEY && e->key == K_ESC) { A.searching = 0; A.search[0] = 0; A.searchLen = 0; return; }
  if (e->type == EV_KEY && e->key == K_ENTER) {
    A.searching = 0;
    if (A.searchLen) code_search_jump(1);
    return;
  }
  if (e->type == EV_KEY && e->key == K_BS) {
    if (A.searchLen > 0) {
      A.searchLen--;
      while (A.searchLen > 0 && ((unsigned char)A.search[A.searchLen] & 0xC0) == 0x80) A.searchLen--;
      A.search[A.searchLen] = 0;
    }
    return;
  }
  if (e->type == EV_CHAR) {
    size_t il = strlen(e->u8);
    if ((size_t)A.searchLen + il < sizeof A.search - 1) {
      memcpy(A.search + A.searchLen, e->u8, il + 1);
      A.searchLen += (int)il;
    }
  }
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
    /* tab is text here — two spaces, the corpus indents with spaces, never \t;
     * focus-cycling keeps Tab everywhere else */
    if (e->type == EV_KEY && e->key == K_TAB) { code_insert_str(" "); code_insert_str(" "); return; }
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
  /* browse: vim vocabulary — counts, word motions, gg/G, /-search, code-local undo */
  if (e->type == EV_CHAR) {
    if ((e->ch >= '1' && e->ch <= '9') || (A.codePending && e->ch == '0')) {
      A.codePending = A.codePending * 10 + (int)(e->ch - '0');
      if (A.codePending > 999999) A.codePending = 999999;
      return;
    }
    int hadCount = A.codePending != 0;
    int rep = hadCount ? A.codePending : 1;
    A.codePending = 0;
    if (e->ch != 'g') A.codeG = 0;
    if (e->ch != 'd') A.codeD = 0;
    switch (e->ch) {
      case 'j': while (rep-- && A.ccy < A.ncode - 1) A.ccy++; return;
      case 'k': while (rep-- && A.ccy > 0) A.ccy--; return;
      case 'h': A.ccx = A.ccx > rep ? A.ccx - rep : 0; return;
      case 'l': A.ccx = A.ccx + rep < lw ? A.ccx + rep : lw; return;
      case '0': A.ccx = 0; return;
      case '^': {
        const char *p = ln;
        int col = 0;
        while (*p == ' ' || *p == '\t') { p++; col++; }
        A.ccx = col;
        return;
      }
      case '$': A.ccx = lw; return;
      case 'w': while (rep--) code_word_fwd(); return;
      case 'b': while (rep--) code_word_back(); return;
      case 'g': /* gg — [count]gg goes to that line */
        if (A.codeG) {
          int tgt = A.codeG > 1 ? A.codeG : 1;
          A.ccy = tgt <= A.ncode ? tgt - 1 : A.ncode - 1;
          A.ccx = 0;
          A.codeG = 0;
        } else A.codeG = rep;
        return;
      case 'G': /* [count]G goes to that line, bare G to the last */
        A.ccy = hadCount ? (rep <= A.ncode ? rep - 1 : A.ncode - 1) : A.ncode - 1;
        A.ccx = 0;
        return;
      case 'i': code_undo_push(); A.codeInsert = 1; return;
      case 'a': code_undo_push(); A.codeInsert = 1; A.ccx = A.ccx < lw ? A.ccx + 1 : lw; return;
      case 'o': {
        code_undo_push();
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
        code_undo_push();
        while (rep--) {
          int at = line_byte_at(ln, A.ccx);
          if (!ln[at]) break;
          int next = at + 1;
          while (ln[next] && ((unsigned char)ln[next] & 0xC0) == 0x80) next++;
          memmove(ln + at, ln + next, strlen(ln + next) + 1);
          A.codeDirty = 1;
        }
        return;
      }
      case 'd': /* dd — [count]dd deletes that many lines */
        if (A.codeD) {
          int cnt = A.codeD;
          A.codeD = 0;
          code_undo_push();
          while (cnt--) {
            if (A.ncode > 1) {
              free(A.code[A.ccy]);
              memmove(&A.code[A.ccy], &A.code[A.ccy + 1], (size_t)(A.ncode - A.ccy - 1) * sizeof(char *));
              A.ncode--;
              if (A.ccy >= A.ncode) A.ccy = A.ncode - 1;
            } else { A.code[0][0] = 0; break; }
          }
          A.codeDirty = 1;
        } else A.codeD = rep;
        return;
      case 'u': code_undo_pop(); return;
      case 'n': if (A.search[0]) code_search_jump(1); return;
      case 'N': if (A.search[0]) code_search_jump(-1); return;
      case '/': A.searching = 1; A.search[0] = 0; A.searchLen = 0; return;
      case 's': code_save(); return;
    }
  }
  if (e->type == EV_KEY) {
    int page = A.codeR.h > 4 ? A.codeR.h - 3 : 10;
    switch (e->key) {
      case K_UP: A.ccy = A.ccy > 0 ? A.ccy - 1 : 0; break;
      case K_DOWN: A.ccy = A.ccy < A.ncode - 1 ? A.ccy + 1 : A.ccy; break;
      case K_LEFT: A.ccx = A.ccx > 0 ? A.ccx - 1 : 0; break;
      case K_RIGHT: A.ccx = A.ccx < lw ? A.ccx + 1 : lw; break;
      case K_HOME: A.ccx = 0; break;
      case K_END: A.ccx = lw; break;
      case K_PGUP: A.ccy = A.ccy > page ? A.ccy - page : 0; break;
      case K_PGDN: A.ccy = A.ccy + page < A.ncode ? A.ccy + page : A.ncode - 1; break;
      case K_ENTER: code_undo_push(); A.codeInsert = 1; break;
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
        if (word_span(w->lines[e->line], 3 + e->keyw + row, &off, &len)) { snprintf(err, errsz, "row out of range"); return -1; }
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
      if (word_span(w->lines[e->line], 2 + e->keyw + row, &off, &len)) { snprintf(err, errsz, "row out of range"); return -1; }
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
      int wi = 2 + e->keyw, fib = 0, firstW = -1, lastW = -1;
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
      int sep = 2 + e->keyw, f2 = 0, insAt = -1;
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

/* a space-view entity edit borrowed the world cursor for its segment-0 target;
 * hand the cell cursor back once the edit is over */
static void spc_return(void) {
  if (!A.spcRet) return;
  A.spcRet = 0;
  A.wSeg = 0;
  A.wRow = A.spcRow;
  A.wCol = A.spcCol;
}

static void cell_edit_commit(void) {
  A.editing = 0;
  char err[256];
  if (world_guard()) { spc_return(); return; }
  int seq = undo_push();
  if (seq < 0) { spc_return(); sayerr("cannot stage undo copy"); return; }
  if (cell_commit(A.editBuf, err, sizeof err)) { undo_drop(); sayerr("edit: %s", err); }
  else say("cell written · step %d staged", seq);
  spc_return();
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
  /* the prompt's own box grows with its lines (to a third of the screen, then it
   * scrolls) + the key atlas line */
  int plines = 1, statusH = 1;
  for (const char *p = A.prompt; *p; p++) plines += *p == '\n';
  int promptH = plines + 2;
  int maxPH = H / 3 > 3 ? H / 3 : 3;
  if (promptH > maxPH) promptH = maxPH;
  /* history: a slim strip above the prompt — a couple of echo lines + the verdict row */
  int outH = 5;
  /* outputs: the large reclaimed surface between world and history */
  int outputsH = (H - promptH - statusH - outH) * 2 / 5;
  int codeH = A.mode == MODE_REG ? 0 : (H - promptH - statusH - outH - outputsH) * 2 / 5;
  A.rail = (Rect){ 0, 0, railW, H - promptH - statusH };
  int x = railW, w = W - railW;
  A.codeR = (Rect){ x, 0, w, codeH };
  A.worldR = (Rect){ x, codeH, w, H - promptH - statusH - outH - outputsH - codeH };
  A.outputsR = (Rect){ x, H - promptH - statusH - outH - outputsH, w, outputsH };
  A.outR = (Rect){ x, H - promptH - statusH - outH, w, outH };
  A.promptR = (Rect){ 0, H - promptH - statusH, W, promptH };
}

static void draw_rail(void) {
  Rect r = A.rail;
  if (r.w <= 0) return;
  char t[64];
  snprintf(t, sizeof t, "demos %d", nDemos);
  box(r.x, r.y, r.w, r.h, t, A.focus == F_RAIL, C_RAILC);
  int vis = r.h - 2;
  if (A.railSel < A.railTop) A.railTop = A.railSel;
  if (A.railSel >= A.railTop + vis) A.railTop = A.railSel - vis + 1;
  for (int i = 0; i < vis && A.railTop + i < nDemos; i++) {
    int di = A.railTop + i;
    const char *p = demoList[di];
    if (!strncmp(p, "demos/", 6)) p += 6;
    int sel = di == A.railSel;
    if (sel) fill(r.x + 1, r.y + 1 + i, r.w - 2, 1, " ", A_REV, 0);
    /* the directory dims, the file carries the color; -nihongo twins tint violet */
    const char *slash = strrchr(p, '/');
    int xx = r.x + 2, y = r.y + 1 + i;
    if (slash) {
      char dir[160];
      snprintf(dir, sizeof dir, "%.*s", (int)(slash - p + 1) < 159 ? (int)(slash - p + 1) : 159, p);
      xx += put(xx, y, (sel ? A_REV : 0) | A_DIM, 0, dir, r.w - 3);
    }
    const char *fn = slash ? slash + 1 : p;
    put(xx, y, sel ? A_REV : 0, strstr(fn, "-nihongo") ? C_NIHONGO : 0, fn, r.w - 2 - (xx - r.x));
  }
  scrollbar(r, A.railTop, vis, nDemos, C_RAILC);
}

static void draw_code(void) {
  Rect r = A.codeR;
  if (r.h <= 1) return;
  char t[PATH_MAX + 64];
  snprintf(t, sizeof t, "code · %s%s%s · %d/%d", A.demoPath[0] ? A.demoPath : "—",
           A.demoPath[0] && strcmp(A.demoLive, A.demoPath) ? " · play copy" : "",
           A.codeDirty ? " +" : "", A.ccy + 1, A.ncode);
  box(r.x, r.y, r.w, r.h, t, A.focus == F_CODE, C_CODEC);
  /* the mode chip: loud in the pane's own title rule, not the status-line corner */
  if (A.focus == F_CODE) {
    const char *chip = A.codeInsert ? " INSERT " : " BROWSE ";
    int chw = swidth(chip);
    if (r.w > chw + 4)
      put(r.x + r.w - 2 - chw, r.y, A.codeInsert ? A_REV | A_BOLD : A_DIM, A.codeInsert ? C_GLOW : 0, chip, chw);
  }
  int vis = r.h - 2;
  if (A.ccy < A.codeTop) A.codeTop = A.ccy;
  if (A.ccy >= A.codeTop + vis) A.codeTop = A.ccy - vis + 1;
  for (int i = 0; i < vis && A.codeTop + i < A.ncode; i++) {
    int li = A.codeTop + i;
    const char *ln = A.code[li];
    const char *lt = ln;
    while (*lt == ' ' || *lt == '\t') lt++;
    int dirline = !strncmp(lt, "--!", 3);          /* directives in their own hue */
    int dim = !dirline && !strncmp(lt, "--", 2);   /* comments dim */
    int defOff = -1, defEnd = -1;                  /* the def keyword's byte span */
    if (!dim && !dirline) {
      if (!strncmp(lt, "def ", 4)) { defOff = (int)(lt - ln); defEnd = defOff + 3; }
      else if (!strncmp(lt, "定義 ", 7)) { defOff = (int)(lt - ln); defEnd = defOff + 6; }
    }
    int x = r.x + 1, y = r.y + 1 + i;
    const char *p = ln;
    int col = 0, maxw = r.w - 2;
    while (*p && col < maxw) {
      const char *at = p;
      uint32_t c = u8next(&p);
      int attr = 0, fg = 0;
      if (dirline) fg = C_DIRECTIVE;
      else if (dim) attr = A_DIM;
      else if (c == ',' || (c == '=' && *p == '>') || (c == '>' && at > ln && at[-1] == '=')) { attr = A_BOLD; fg = C_GLOW; }
      else if ((int)(at - ln) >= defOff && (int)(at - ln) < defEnd) { attr = A_BOLD; fg = C_DEF; }
      else if (c == '&' || c == '|' || c == '<' || c == '>' || c == '=' || c == '~' ||
               c == '!' || c == '+' || c == '*' || c == '/' || c == '%') fg = C_OP;
      else if (c >= '0' && c <= '9') fg = C_NUMLIT;
      char g[8] = { 0 };
      memcpy(g, at, (size_t)(p - at) < 7 ? (size_t)(p - at) : 7);
      col += put(x + col, y, attr, fg, g, maxw - col);
    }
    /* every /-match on a visible line lights up (span approximated by the needle) */
    if (A.search[0]) {
      anostr_t hay = anostr_view(ln, strlen(ln));
      anostr_t nd = anostr_view(A.search, (size_t)A.searchLen);
      size_t from = 0, f;
      int ndw = swidth(A.search);
      while ((f = anostr_find_base(hay, nd, from)) != ANOSTR_NPOS) {
        int c0 = line_col_of(ln, (int)f);
        for (int cc = c0; cc < c0 + ndw && x + cc < r.x + r.w - 1; cc++) rev_cell(x + cc, y);
        from = f + 1;
      }
    }
    if (A.focus == F_CODE && li == A.ccy) {
      int cx = x + A.ccx;
      if (cx < r.x + r.w - 1) {
        /* insert gets the terminal's own bar cursor (DECSCUSR 5, or the default where
         * unhonored); browse keeps the block reverse cell */
        if (A.codeInsert) { T.curX = cx; T.curY = y; T.curShape = 5; }
        else rev_cell(cx, y);
      }
    }
  }
  if (A.searching || A.search[0]) {
    char sb[160];
    snprintf(sb, sizeof sb, "/%s%s", A.search, A.searching ? "▏" : "");
    put(r.x + 2, r.y + r.h - 1, A_BOLD, C_SEARCH, sb, r.w - 4);
  }
  scrollbar(r, A.codeTop, vis, A.ncode, C_CODEC);
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
static const int fieldPal[] = { 39, 208, 170, 114, 221, 80, 213, 147, 210, 84 };

static void draw_space(Rect r) {
  World *w = &A.world;
  int gw, gh;
  space_dims(&gw, &gh);
  Ent *pos = world_pos(w);
  if (!gw || !gh) { put(r.x + 2, r.y + 1, A_DIM, 0, "no lattice, no positions — table only (m cycles views)", r.w - 4); return; }
  int ox = r.x + 2, oy = r.y + 1;
  /* square cells: two columns per cell, ~1:1 in any font. A cell draws only when
   * both its columns sit inside the border — no straddle across the region edge. */
  /* fields paint in declaration order: char glyphs exact, bools as colored blocks,
   * nums shaded ░▒▓█; x/y coordinate fields skip (they would drown the picture) */
  for (int cy = 0; cy < gh && oy + cy < r.y + r.h - 1; cy++)
    for (int cx = 0; cx < gw && ox + 2 * cx + 2 <= r.x + r.w - 1; cx++) {
      int k = cy * gw + cx;
      const char *g = "·";
      int fg = 0, attr = A_DIM, pi = 0, dbl = 0; /* dbl: block/shade glyphs double up */
      for (int i = 0; i < w->nents; i++) {
        Ent *e = &w->ents[i];
        if (e->kind != E_FIELD) continue;
        if (names_eq(e->name, "x") || names_eq(e->name, "y")) { pi++; continue; }
        if (e->type == V_CHAR) {
          if (e->chars && k < (int)strlen(e->chars) && e->chars[k] != '.') {
            g = glyph_at(e, k); fg = fieldPal[pi % 10]; attr = 0; dbl = 0;
          }
        } else if (k < e->nn && e->nums[k] != 0) {
          if (e->type == V_BOOL) { g = "█"; fg = fieldPal[pi % 10]; attr = 0; dbl = 1; }
          else {
            double max = 0;
            for (int j = 0; j < e->nn; j++) if (e->nums[j] > max) max = e->nums[j];
            g = shade(e->nums[k], max); fg = fieldPal[pi % 10]; attr = 0; dbl = 1;
          }
        }
        pi++;
      }
      put(ox + 2 * cx, oy + cy, attr, fg, g, 1);
      put(ox + 2 * cx + 1, oy + cy, attr, fg, dbl ? g : " ", 1);
    }
  /* positioned entities stand on their cells: their glyph when one resolves
   * (role glyph, `glyph`, a single-rune proto noun), bold @ otherwise; ink from the
   * case convention, else the archetype hash, else C_AT */
  if (pos)
    for (int i = 0; i + 1 < pos->nn; i += 2) {
      double dx = pos->nums[i], dy = pos->nums[i + 1];
      if (dx < 0 || dx >= gw || dy < 0 || dy >= gh) continue;   /* guard before the cast */
      int px = (int)dx, py = (int)dy;
      if (ox + 2 * px + 2 > r.x + r.w - 1 || oy + py >= r.y + r.h - 1) continue;
      char eg[8];
      int have = ent_glyph(w, i / 2, eg);
      int fg = ent_color(w, i / 2, have ? eg : NULL);
      int gwd = putp(ox + 2 * px, oy + py, have ? 0 : A_BOLD, fg, 0, have ? eg : "@", 2);
      if (gwd < 2) put(ox + 2 * px + 1, oy + py, 0, fg, " ", 1);
    }
  /* the cell cursor works on the map exactly as on the table */
  if (A.focus == F_WORLD) {
    int cx = ox + 2 * A.wCol, cy = oy + A.wRow;
    if (A.wCol < gw && A.wRow < gh && cx + 2 <= r.x + r.w - 1 && cy < r.y + r.h - 1) {
      rev_cell(cx, cy);
      rev_cell(cx + 1, cy);
    }
  }
  if (A.dragging) {
    int rr0 = A.dragR0 < A.dragR1 ? A.dragR0 : A.dragR1, rr1 = A.dragR0 < A.dragR1 ? A.dragR1 : A.dragR0;
    int cc0 = A.dragC0 < A.dragC1 ? A.dragC0 : A.dragC1, cc1 = A.dragC0 < A.dragC1 ? A.dragC1 : A.dragC0;
    for (int yy = rr0; yy <= rr1 && yy < gh; yy++)
      for (int xx = cc0; xx <= cc1 && xx < gw; xx++)
        if (xx >= 0 && yy >= 0 && ox + 2 * xx + 2 <= r.x + r.w - 1 && oy + yy < r.y + r.h - 1) {
          rev_cell(ox + 2 * xx, oy + yy);
          rev_cell(ox + 2 * xx + 1, oy + yy);
        }
  }
}

/* the bitmap: the same world at pixel scale — ▀ with fg the upper pixel and bg the
 * lower gives two vertical pixels per terminal row at one column each (~square).
 * Fields shade the ground in declaration order (bools and chars in their field
 * color, nums as a gray ramp against their max), positioned entities land on top as
 * archetype-colored pixels — the exact inks the glyph map uses, at pixel scale. */
static void draw_bitmap(Rect r) {
  World *w = &A.world;
  int gw, gh;
  space_dims(&gw, &gh);
  Ent *pos = world_pos(w);
  if (!gw || !gh) { put(r.x + 2, r.y + 1, A_DIM, 0, "no lattice, no positions — table only (m cycles views)", r.w - 4); return; }
  int ox = r.x + 2, oy = r.y + 1;
  int *pix = xalloc((size_t)gw * (size_t)gh * sizeof(int));
  for (int k = 0; k < gw * gh; k++) pix[k] = C_BG;
  int pi = 0;
  for (int i = 0; i < w->nents; i++) {
    Ent *e = &w->ents[i];
    if (e->kind != E_FIELD) continue;
    if (names_eq(e->name, "x") || names_eq(e->name, "y")) { pi++; continue; }
    if (e->type == V_CHAR) {
      int len = e->chars ? (int)strlen(e->chars) : 0;
      for (int k = 0; k < gw * gh && k < len; k++)
        if (e->chars[k] != '.') pix[k] = fieldPal[pi % 10];
    } else {
      double max = 0;
      for (int j = 0; j < e->nn; j++) if (e->nums[j] > max) max = e->nums[j];
      if (max <= 0) max = 1;
      for (int k = 0; k < gw * gh && k < e->nn; k++)
        if (e->nums[k] != 0) {
          /* nums ramp the xterm grayscale 236..248 — visible on the 234 canvas */
          double t = e->nums[k] / max;
          if (t < 0) t = 0;
          if (t > 1) t = 1;
          pix[k] = e->type == V_BOOL ? fieldPal[pi % 10] : 236 + (int)(t * 12.0);
        }
    }
    pi++;
  }
  if (pos)
    for (int i = 0; i + 1 < pos->nn; i += 2) {
      double dx = pos->nums[i], dy = pos->nums[i + 1];
      if (dx < 0 || dx >= gw || dy < 0 || dy >= gh) continue;
      char eg[8];
      int have = ent_glyph(w, i / 2, eg);
      pix[(int)dy * gw + (int)dx] = ent_color(w, i / 2, have ? eg : NULL);
    }
  for (int ty = 0; 2 * ty < gh && oy + ty < r.y + r.h - 1; ty++)
    for (int cx = 0; cx < gw && ox + cx < r.x + r.w - 1; cx++) {
      int up = pix[2 * ty * gw + cx];
      int lo = 2 * ty + 1 < gh ? pix[(2 * ty + 1) * gw + cx] : C_BG;
      putp(ox + cx, oy + ty, 0, up, lo, "▀", 1);
    }
  free(pix);
  /* the cell cursor addresses one pixel; its terminal cell shows the ▀ pair, and the
   * reverse marks that pair — the tracked cell is exact even where the mark is coarse */
  if (A.focus == F_WORLD && A.wCol < gw && A.wRow < gh) {
    int cx = ox + A.wCol, cy = oy + A.wRow / 2;
    if (cx < r.x + r.w - 1 && cy < r.y + r.h - 1) rev_cell(cx, cy);
  }
  if (A.dragging) {
    int rr0 = A.dragR0 < A.dragR1 ? A.dragR0 : A.dragR1, rr1 = A.dragR0 < A.dragR1 ? A.dragR1 : A.dragR0;
    int cc0 = A.dragC0 < A.dragC1 ? A.dragC0 : A.dragC1, cc1 = A.dragC0 < A.dragC1 ? A.dragC1 : A.dragC0;
    for (int yy = rr0; yy <= rr1 && yy < gh; yy++)
      for (int xx = cc0; xx <= cc1 && xx < gw; xx++)
        if (xx >= 0 && yy >= 0 && ox + xx < r.x + r.w - 1 && oy + yy / 2 < r.y + r.h - 1) rev_cell(ox + xx, oy + yy / 2);
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
  snprintf(t, sizeof t, "%s · %s%s · n %d", A.spaceView == 2 ? "bitmap" : A.spaceView ? "space" : "world",
           w->loaded ? w->path : "—",
           A.worldIsCopy ? " (play)" : (w->loaded && A.mode != MODE_REG ? " (pristine)" : ""), w->n);
  if (A.worldIsCopy && A.undoSeq > 0)
    snprintf(t + strlen(t), sizeof t - strlen(t), " · step %d", A.undoSeq);
  if (w->latW) snprintf(t + strlen(t), sizeof t - strlen(t), " · %d×%d", w->latW, w->latH);
  box(r.x, r.y, r.w, r.h, t, A.focus == F_WORLD, C_WORLDC);
  if (!w->loaded) { put(r.x + 2, r.y + 1, A_DIM, 0, "no world — pick a demo or open a .reg", r.w - 4); return; }
  if (A.spaceView == 2) { draw_bitmap(r); return; }
  if (A.spaceView) { draw_space(r); return; }
  table_cols();
  world_vrows();
  if (A.wSeg == 0 && A.wCol >= ndcols) A.wCol = ndcols ? ndcols - 1 : 0;
  int x = r.x + 1, y = r.y + 1;
  int rowLblW = 4;
  /* the table header pins above the scroll */
  put(x, y, A_DIM, C_ROWLBL, "row", rowLblW);
  int cx = x + rowLblW + 1;
  for (int c = 0; c < ndcols && cx < r.x + r.w - 1; c++) {
    put(cx, y, A_BOLD | (dcols[c].e->kind == E_PRES ? A_DIM : 0), C_HDR, dcols[c].e->name, dcols[c].width);
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
    if (vr->kind == 1) { put(x, yy, A_BOLD, C_HDR, seg_field(vr->seg)->name, r.w - 2); continue; }
    if (vr->kind == 0) {
      int row = vr->row;
      char lbl[16];
      snprintf(lbl, sizeof lbl, "%d", row);
      put(x, yy, A_DIM, C_ROWLBL, lbl, rowLblW);
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
        } else {
          /* value hue by kind: sym lilac, rel salmon, bool teal, char warm, vec pale */
          int cfg = e->type == V_SYM ? C_SYM : e->type == V_CHAR ? C_CHAR
                  : e->kind == E_REL ? C_REL : e->kind == E_ALIAS ? C_DIRECTIVE
                  : e->kind == E_PRES ? 0 : e->type == V_BOOL ? C_BOOL
                  : e->type == V_VEC ? C_NUMLIT : 0;
          put(cx, yy, (cur || inDrag ? A_REV : 0) | (dim ? A_DIM : 0), dim ? 0 : cfg, cell, dcols[c].width);
        }
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
        put(px, yy, cur ? A_REV : (e->type == V_BOOL && k < e->nn && e->nums[k] == 0 ? A_DIM : 0),
            e->type == V_CHAR ? C_CHAR : e->type == V_BOOL ? C_BOOL : 0, cell, cellW + 1);
    }
  }
  scrollbar(r, A.wTop, vis, nvrows, C_WORLDC);
}

/* the OUTPUTS surface: labeled query results, newest tick first — a dim seam line per
 * tick, each record as `q1 · <stmt> → <value>` with per-tick ordinals; a multi-line
 * value renders label first, its lines indented beneath. Scroll is top-anchored: 0
 * pins the newest tick. */
static void draw_outputs(void) {
  Rect r = A.outputsR;
  if (r.h <= 1) return;
  box(r.x, r.y, r.w, r.h, "outputs", A.focus == F_OUTPUTS, C_OUTPUTSC);
  int vis = r.h - 2;
  if (vis < 1) return;
  int total = outputs_total_lines();
  int max = total - vis;
  if (max < 0) max = 0;
  if (A.outputsScroll > max) A.outputsScroll = max;
  if (A.outputsScroll < 0) A.outputsScroll = 0;
  if (!A.nqgroups) {
    put(r.x + 2, r.y + 1, A_DIM, 0, "query results land here — n ticks the demo, the prompt asks", r.w - 4);
    return;
  }
  int li = 0, y = r.y + 1, yend = r.y + r.h - 1;
  for (int gi = A.nqgroups - 1; gi >= 0 && y < yend; gi--) {
    QGroup *g = &A.qgroups[gi];
    if (li >= A.outputsScroll) {
      char seam[48];
      if (g->step > 0) snprintf(seam, sizeof seam, "— step %d", g->step);
      else snprintf(seam, sizeof seam, "— run");
      put(r.x + 2, y++, A_DIM, C_FRAME, seam, r.w - 4);
    }
    li++;
    for (int ri = 0; ri < g->nrecs && y < yend; ri++) {
      QRec *q = &g->recs[ri];
      int multi = strchr(q->value, '\n') != NULL;
      if (li >= A.outputsScroll) {
        int x = r.x + 2, xe = r.x + r.w - 2;
        char ord[16];
        snprintf(ord, sizeof ord, "q%d", ri + 1);
        x += put(x, y, A_BOLD, C_OUTPUTSC, ord, xe - x);
        x += put(x, y, A_DIM, C_FRAME, " · ", xe - x);
        x += put(x, y, 0, 0, q->label, xe - x);
        if (!multi) {
          x += put(x, y, A_DIM, C_FRAME, " → ", xe - x);
          put(x, y, A_BOLD, C_OUTPUTSC, q->value, xe - x);
        }
        y++;
      }
      li++;
      if (!multi) continue;
      for (const char *p = q->value; *p && y < yend;) {
        const char *nl = strchr(p, '\n');
        size_t ll = nl ? (size_t)(nl - p) : strlen(p);
        if (li >= A.outputsScroll) {
          char vb[512];
          snprintf(vb, sizeof vb, "%.*s", (int)(ll < 500 ? ll : 500), p);
          u8_tail_fix(vb);
          put(r.x + 6, y++, 0, C_OUTPUTSC, vb, r.w - 8);
        }
        li++;
        if (!nl) break;
        p = nl + 1;
      }
    }
  }
  scrollbar(r, A.outputsScroll, vis, total, C_OUTPUTSC);
}

static void draw_out(void) {
  Rect r = A.outR;
  if (r.h <= 1) return;
  box(r.x, r.y, r.w, r.h, "history", A.focus == F_OUT, C_OUTC);
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
      /* echoes tint by origin, failures by content; trace diagnostics by their formats */
      int fg = 0, attr = 0;
      if (line[0] == '>' && line[1] == ' ') fg = C_PROMPTC;
      else if (line[0] == '$' && line[1] == ' ') { fg = C_CODEC; attr = A_DIM; }
      else if (strstr(line, "error") || strstr(line, "FAIL") || strstr(line, "cannot")) fg = C_ERR;
      else if (strstr(line, " IS DEAD !") || strstr(line, " IS EMPTY !")) fg = C_ERR;
      else if (strstr(line, " rows -> ")) { fg = C_WORLDC; attr = A_DIM; }
      put(r.x + 2, yy, attr, fg, line, r.w - 4);
      yy++;
    }
    li++;
    if (!nl) break;
    p = nl + 1;
  }
  /* the verdict line: pins held, the failure, or the save/undo status — unambiguous */
  put(r.x + 2, r.y + r.h - 2, A_BOLD, A.verdict[0] ? (A.verdictBad ? C_ERR : C_OK) : 0,
      A.verdict[0] ? A.verdict : "—", r.w - 4);
  scrollbar(r, first, vis, nls, C_OUTC);
}

/* the prompt in its own box: a multi-line statement (\⏎, shift-enter, or alt-enter
 * breaks lines), vertically scrolled to the cursor's line, the cursor's line
 * horizontally scrolled so the cursor is always visible; ↑↓ walk lines then history */
static void draw_prompt(void) {
  Rect r = A.promptR;
  int on = A.focus == F_PROMPT;
  box(r.x, r.y, r.w, r.h, A.mode == MODE_REG ? "prompt · the program" : "prompt", on, C_PROMPTC);
  int vis = r.h - 2;
  if (vis < 1) vis = 1;
  /* the cursor's line, and the line count */
  int cl = 0, nlines = 1;
  for (int i = 0; A.prompt[i]; i++) {
    if (A.prompt[i] != '\n') continue;
    nlines++;
    if (i < A.pcur) cl++;
  }
  if (A.ptop > cl) A.ptop = cl;
  if (cl >= A.ptop + vis) A.ptop = cl - vis + 1;
  if (A.ptop > nlines - vis) A.ptop = nlines - vis;
  if (A.ptop < 0) A.ptop = 0;
  int avail = r.w - 7;
  if (avail < 8) avail = 8;
  int lstart = 0;
  for (int li = 0;; li++) {
    const char *lp = A.prompt + lstart;
    const char *nl = strchr(lp, '\n');
    int llen = nl ? (int)(nl - lp) : (int)strlen(lp);
    if (li >= A.ptop && li < A.ptop + vis) {
      int y = r.y + 1 + li - A.ptop;
      put(r.x + 2, y, A_BOLD, on ? C_PROMPTC : C_FRAME, li == 0 ? ">" : "·", 1);
      int off = 0;
      if (li == cl) {
        /* horizontal window on the cursor's line only */
        if (A.pscroll < lstart || A.pscroll > A.pcur) A.pscroll = lstart;
        for (;;) {
          char seg[1024];
          snprintf(seg, sizeof seg, "%.*s", A.pcur - A.pscroll, A.prompt + A.pscroll);
          if (swidth(seg) < avail) break;
          const char *p = A.prompt + A.pscroll;
          u8next(&p);
          A.pscroll = (int)(p - A.prompt);
        }
        off = A.pscroll - lstart;
      }
      char seg[1024];
      snprintf(seg, sizeof seg, "%.*s", llen - off, lp + off);
      put(r.x + 4, y, on ? 0 : A_DIM, 0, seg, avail);
      if (li == cl && off > 0) put(r.x + 3, y, A_DIM, C_PROMPTC, "…", 1);
      if (on && li == cl) {
        char tmp[1024];
        snprintf(tmp, sizeof tmp, "%.*s", A.pcur - A.pscroll, A.prompt + A.pscroll); /* byte offsets */
        int cx = r.x + 4 + swidth(tmp);
        if (cx < r.x + r.w - 1) rev_cell(cx, y);
      }
    }
    if (!nl) break;
    lstart = (int)(nl - A.prompt) + 1;
  }
  scrollbar(r, A.ptop, vis, nlines, C_PROMPTC);
}

/* the key atlas gets the bottom line to itself — context-sensitive, out of every box */
static void draw_status(void) {
  int y = T.rows - 1;
  fill(0, y, T.cols, 1, " ", 0, 0);
  const char *hint =
    A.confirmReset ? "y wipes every play copy — demos/ becomes the only state · any other key cancels"
    : A.focus == F_PROMPT ? "enter runs · \\⏎ or shift-enter breaks a line · ↑↓ lines, history · esc leaves"
    : A.focus == F_WORLD && A.editing ? "enter commits · esc cancels"
    : A.focus == F_CODE && A.searching ? "type the pattern · enter jumps · esc cancels"
    : A.focus == F_CODE && A.codeInsert ? "insert — esc returns to browse"
    : A.focus == F_CODE ? "hjkl w b gg G 0 ^ $ move · / search, n N · i a o insert · x dd delete · u undo · s save"
    : A.focus == F_OUTPUTS ? "outputs — j k scroll · pgup pgdn page · newest tick first · u drops a tick"
    : "tab focus · > prompt · r reset · n next · m view (table/map/bitmap) · u undo · w snap · t trace · E editor · q quit";
  put(1, y, A_DIM, 0, hint, T.cols - 10);
  const char *mode = A.mode == MODE_RAIL ? "rail" : A.mode == MODE_REG ? "world" : "demo";
  int mfg = A.mode == MODE_RAIL ? C_RAILC : A.mode == MODE_REG ? C_WORLDC : C_CODEC;
  int mw = swidth(mode);
  put(T.cols - mw - 2, y, A_BOLD, mfg, mode, mw);
}

static void draw(void) {
  frame_clear();
  layout();
  draw_rail();
  draw_code();
  draw_world();
  draw_outputs();
  draw_out();
  draw_prompt();
  draw_status();
  flush_frame();
}

/* ---------- input dispatch ---------- */

/* the prompt line containing byte `at`: [start, end) offsets into A.prompt */
static void prompt_line_at(int at, int *start, int *end) {
  int s = at;
  while (s > 0 && A.prompt[s - 1] != '\n') s--;
  int e = at;
  while (A.prompt[e] && A.prompt[e] != '\n') e++;
  *start = s;
  *end = e;
}

static void prompt_newline(void) {
  if (A.plen + 1 >= (int)sizeof A.prompt - 1) return;
  memmove(A.prompt + A.pcur + 1, A.prompt + A.pcur, strlen(A.prompt + A.pcur) + 1);
  A.prompt[A.pcur] = '\n';
  A.plen++;
  A.pcur++;
}

static void prompt_key(Ev *e) {
  if (e->type == EV_KEY) {
    switch (e->key) {
      case K_ESC: A.focus = A.world.loaded ? F_WORLD : (A.mode == MODE_RAIL ? F_RAIL : F_CODE); return;
      case K_ENTER:
        /* \⏎ asks for a line, not a run — the backslash becomes the newline */
        if (A.pcur > 0 && A.prompt[A.pcur - 1] == '\\') { A.prompt[A.pcur - 1] = '\n'; return; }
        repl_submit();
        return;
      case K_NEWLINE: prompt_newline(); return;  /* shift-enter (CSI-u), alt-enter */
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
      case K_HOME: { int ls, le; prompt_line_at(A.pcur, &ls, &le); A.pcur = ls; return; }
      case K_END: { int ls, le; prompt_line_at(A.pcur, &ls, &le); A.pcur = le; return; }
      case K_UP: {
        /* within a multi-line statement the arrows walk lines; history past the top */
        int ls, le;
        prompt_line_at(A.pcur, &ls, &le);
        if (ls > 0) {
          int col = line_col_of(A.prompt + ls, A.pcur - ls);
          int pls, ple;
          prompt_line_at(ls - 1, &pls, &ple);
          int at = line_byte_at(A.prompt + pls, col);
          if (at > ple - pls) at = ple - pls;
          A.pcur = pls + at;
          return;
        }
        if (A.histAt > 0) {
          A.histAt--;
          snprintf(A.prompt, sizeof A.prompt, "%s", A.hist[A.histAt]);
          A.plen = A.pcur = (int)strlen(A.prompt);
        }
        return;
      }
      case K_DOWN: {
        int ls, le;
        prompt_line_at(A.pcur, &ls, &le);
        if (A.prompt[le] == '\n') {
          int col = line_col_of(A.prompt + ls, A.pcur - ls);
          int nls = le + 1, nle;
          prompt_line_at(nls, &nls, &nle);
          int at = line_byte_at(A.prompt + nls, col);
          if (at > nle - nls) at = nle - nls;
          A.pcur = nls + at;
          return;
        }
        if (A.histAt < A.nhist - 1) {
          A.histAt++;
          snprintf(A.prompt, sizeof A.prompt, "%s", A.hist[A.histAt]);
        } else { A.histAt = A.nhist; A.prompt[0] = 0; }
        A.plen = A.pcur = (int)strlen(A.prompt);
        return;
      }
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
  if (e->type == EV_KEY && e->key == K_ESC) { A.editing = 0; spc_return(); say("edit cancelled"); return; }
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
      /* map/bitmap edit: whatever painted this cell — the glyph (or pixel) you see
       * is the value you edit. The topmost positioned entity on the cell wins: its
       * edit targets the entity's own table row at the column that painted it (the
       * glyph source when one resolves, else pos), the space cursor parked and
       * restored after. No entity: the last field lit at the cell, else the first
       * paintable field. */
      Ent *pos = world_pos(w);
      int entRow = -1;
      if (pos)
        for (int i = 0; i + 1 < pos->nn; i += 2)
          if (pos->nums[i] == (double)A.wCol && pos->nums[i + 1] == (double)A.wRow) entRow = i / 2;
      if (entRow >= 0) {
        table_cols();
        Ent *src = world_glyph_col(w);
        if (!(src && entRow < src->ns && src->syms[entRow][0])) {
          Ent *pr = world_proto_col(w);
          src = (pr && entRow < pr->ns && pr->syms[entRow][0]) ? pr : pos;
        }
        int c = 0;
        for (int j = 0; j < ndcols; j++)
          if (dcols[j].e == src) { c = j; break; }
        A.spcRet = 1;
        A.spcRow = A.wRow;
        A.spcCol = A.wCol;
        A.wSeg = 0;
        A.wRow = entRow;
        A.wCol = c;
      } else {
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

/* Inputs: a bare command name (a name with a slash checks directly). Output: 1 when an
 * executable of that name sits on $PATH, 0 otherwise. */
static int path_has(const char *cmd) {
  if (strchr(cmd, '/')) return access(cmd, X_OK) == 0;
  const char *p = getenv("PATH");
  if (!p) return 0;
  char cand[PATH_MAX];
  while (*p) {
    const char *sep = strchr(p, ':');
    size_t dl = sep ? (size_t)(sep - p) : strlen(p);
    if (dl && dl + strlen(cmd) + 2 < sizeof cand) {
      snprintf(cand, sizeof cand, "%.*s/%s", (int)dl, p, cmd);
      if (access(cand, X_OK) == 0) return 1;
    }
    p += dl + (sep ? 1 : 0);
  }
  return 0;
}

/* Output: the hop's editor — $VISUAL, else $EDITOR, else the first of nvim, vim,
 * micro, nano, vi found on PATH. The floor is never vim.tiny. */
static const char *editor_pick(void) {
  const char *ed = getenv("VISUAL");
  if (ed && *ed) return ed;
  ed = getenv("EDITOR");
  if (ed && *ed) return ed;
  static const char *const fall[] = { "nvim", "vim", "micro", "nano", "vi" };
  for (size_t i = 0; i < sizeof fall / sizeof *fall; i++)
    if (path_has(fall[i])) return fall[i];
  return "vi";
}

static void editor_hop(void) {
  const char *ed = editor_pick();
  /* the hop is a mutating act: corpus files guard into their play copies first */
  const char *file = NULL;
  int worldFile = 0;
  if (A.world.loaded && (A.focus == F_WORLD || !A.demoPath[0])) {
    if (world_guard()) return;
    file = A.world.path;
    worldFile = 1;
  } else if (A.demoPath[0]) {
    if (code_guard()) return;
    file = A.demoLive;
  }
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
  if (worldFile && A.world.loaded) {
    snprintf(path, sizeof path, "%s", A.world.path);
    if (world_load(&A.world, path, err, sizeof err)) sayerr("%s", err);
    else say("reloaded %s", path);
  } else if (A.demoPath[0]) {
    code_load(A.demoLive);
    say("reloaded %s", A.demoLive);
  }
}

static void snapshot(void) {
  if (!A.world.loaded) { say("no world to snapshot"); return; }
  static int snapSeq;
  char dst[PATH_MAX + 48];
  snprintf(dst, sizeof dst, ".kore/%s-snap%d.reg", world_tag(), ++snapSeq);
  mkdirs(".kore");
  if (copy_file(A.world.path, dst)) sayerr("snapshot failed");
  else say("snapshot → %s", dst);
}

/* mouse: everything the keys reach, a click reaches */
static int hit(Rect r, int x, int y) { return x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h; }

static void mouse_ev(Ev *e) {
  int x = e->mx, y = e->my;
  if (e->mkind == M_WHEELUP || e->mkind == M_WHEELDN) {
    /* one item per notch — the view follows one line at a time, never a leap */
    int d = e->mkind == M_WHEELUP ? -1 : 1;
    if (hit(A.rail, x, y)) { A.railSel += d; if (A.railSel < 0) A.railSel = 0; if (A.railSel >= nDemos) A.railSel = nDemos ? nDemos - 1 : 0; }
    else if (hit(A.codeR, x, y)) { A.ccy += d; if (A.ccy < 0) A.ccy = 0; if (A.ccy >= A.ncode) A.ccy = A.ncode ? A.ncode - 1 : 0; }
    else if (hit(A.worldR, x, y)) {
      int rows, gw;
      if (A.spaceView) space_dims(&gw, &rows);
      else rows = seg_rows(A.wSeg);
      A.wRow += d;
      if (A.wRow < 0) A.wRow = 0;
      if (A.wRow >= rows) A.wRow = rows ? rows - 1 : 0;
    } else if (hit(A.outputsR, x, y)) {
      int vis = A.outputsR.h - 2;
      if (vis < 1) vis = 1;
      int max = outputs_total_lines() - vis;
      if (max < 0) max = 0;
      A.outputsScroll += d;
      if (A.outputsScroll < 0) A.outputsScroll = 0;
      if (A.outputsScroll > max) A.outputsScroll = max;
    } else if (hit(A.outR, x, y)) { A.outScroll -= d; if (A.outScroll < 0) A.outScroll = 0; }
    else if (hit(A.promptR, x, y)) {
      /* the wheel walks the session history, exactly as ↑↓ do */
      A.focus = F_PROMPT;
      if (e->mkind == M_WHEELUP && A.histAt > 0) {
        A.histAt--;
        snprintf(A.prompt, sizeof A.prompt, "%s", A.hist[A.histAt]);
        A.plen = A.pcur = (int)strlen(A.prompt);
      } else if (e->mkind == M_WHEELDN) {
        if (A.histAt < A.nhist - 1) {
          A.histAt++;
          snprintf(A.prompt, sizeof A.prompt, "%s", A.hist[A.histAt]);
        } else { A.histAt = A.nhist; A.prompt[0] = 0; }
        A.plen = A.pcur = (int)strlen(A.prompt);
      }
    }
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
    if (hit(A.outputsR, x, y)) { A.focus = F_OUTPUTS; return; }
    if (hit(A.outR, x, y)) { A.focus = F_OUT; return; }
    if (hit(A.worldR, x, y)) {
      A.focus = F_WORLD;
      if (A.spaceView) {
        int gw, gh;
        space_dims(&gw, &gh);
        /* map cells are two columns wide; a bitmap column is one cell, its row the
         * ▀ pair's upper pixel */
        int rx = x - A.worldR.x - 2, ry = y - A.worldR.y - 1;
        int cx = A.spaceView == 2 ? rx : rx / 2;
        int cy = A.spaceView == 2 ? 2 * ry : ry;
        if (rx >= 0 && ry >= 0 && cx < gw && cy < gh) {
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
      int rx = x - A.worldR.x - 2, ry = y - A.worldR.y - 1;
      int cx = A.spaceView == 2 ? rx : rx / 2;
      int cy = A.spaceView == 2 ? 2 * ry : ry;
      if (rx >= 0 && ry >= 0) { A.dragC1 = cx; A.dragR1 = cy; }
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
  /* an armed >reset: y wipes, anything else cancels — the one modal in kore */
  if (A.confirmReset) {
    A.confirmReset = 0;
    if (e->type == EV_CHAR && (e->ch == 'y' || e->ch == 'Y')) reset_all();
    else say("reset cancelled");
    return;
  }
  if (e->type == EV_KEY && e->key == K_RESETALL) { kore_command("reset"); return; }
  if (e->type == EV_MOUSE) { mouse_ev(e); return; }
  /* text-entry contexts swallow everything */
  if (A.focus == F_PROMPT) { prompt_key(e); return; }
  if (A.focus == F_WORLD && A.editing) { edit_key(e); return; }
  if (A.focus == F_CODE && A.searching) { search_key(e); return; }
  if (A.focus == F_CODE && A.codeInsert) { code_key(e); return; }
  /* global keys — u, w, n yield to the code surface (vim undo, word motion, match) */
  if (e->type == EV_CHAR) {
    switch (e->ch) {
      case 'q': A.quit = 1; return;
      case ':': A.focus = F_PROMPT; return;
      case '>':   /* the command form: an empty prompt opens pre-filled with > */
        A.focus = F_PROMPT;
        if (!A.plen) { A.prompt[0] = '>'; A.prompt[1] = 0; A.plen = A.pcur = 1; }
        return;
      case 'm': A.spaceView = (A.spaceView + 1) % 3; A.wRow = A.wCol = 0; return;
      case 'r': world_reset(); return;
      case 'n': if (A.focus == F_CODE && A.search[0]) break; world_next(); return;
      case 'u': if (A.focus == F_CODE) break; undo_pop(); return;
      case 'w': if (A.focus == F_CODE) break; snapshot(); return;
      case 't':   /* trace toggle: --trace rides the next n or prompt run; observability
                     only — post-state is identical either way. Global: t is no code-pane
                     vim key here, so it never yields the way u/w/n do */
        A.trace = !A.trace;
        say(A.trace ? "trace on — dead links and tick deltas land in history"
                    : "trace off");
        return;
      case 'E': editor_hop(); return;
      default: break;
    }
  }
  /* esc steps out: an active search clears first, then any panel returns to the
   * mode's home surface — the rail, the code, or the prompt */
  if (e->type == EV_KEY && e->key == K_ESC) {
    if (A.focus == F_CODE && A.search[0]) { A.search[0] = 0; A.searchLen = 0; say("search cleared"); return; }
    A.focus = A.mode == MODE_RAIL ? F_RAIL : A.mode == MODE_REG ? F_PROMPT : F_CODE;
    return;
  }
  if (e->type == EV_KEY && e->key == K_TAB) {
    enum Focus order[] = { F_RAIL, F_CODE, F_WORLD, F_OUTPUTS, F_OUT, F_PROMPT };
    int n = 6, at = 0;
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
    case F_OUTPUTS: {
      int vis = A.outputsR.h - 2;
      if (vis < 1) vis = 1;
      int max = outputs_total_lines() - vis;
      if (max < 0) max = 0;
      int ch = e->type == EV_CHAR ? (int)e->ch : 0;
      int key = e->type == EV_KEY ? e->key : 0;
      if (ch == 'j' || key == K_DOWN) A.outputsScroll++;
      else if (ch == 'k' || key == K_UP) A.outputsScroll--;
      else if (key == K_PGDN) A.outputsScroll += vis;
      else if (key == K_PGUP) A.outputsScroll -= vis;
      if (A.outputsScroll < 0) A.outputsScroll = 0;
      if (A.outputsScroll > max) A.outputsScroll = max;
      break;
    }
    case F_OUT:
      if (e->type == EV_CHAR && e->ch == 'j' && A.outScroll > 0) A.outScroll--;
      if (e->type == EV_CHAR && e->ch == 'k') A.outScroll++;
      if (e->type == EV_KEY && e->key == K_DOWN && A.outScroll > 0) A.outScroll--;
      if (e->type == EV_KEY && e->key == K_UP) A.outScroll++;
      if (e->type == EV_KEY && e->key == K_PGUP) A.outScroll += A.outR.h > 4 ? A.outR.h - 3 : 5;
      if (e->type == EV_KEY && e->key == K_PGDN) { A.outScroll -= A.outR.h > 4 ? A.outR.h - 3 : 5; if (A.outScroll < 0) A.outScroll = 0; }
      break;
    default: break;
  }
}

/* ---------- headless check: the verification hook ---------- */

/* Inputs: a .reg path. Output: 0 with a one-line report after loading and rendering
 * every view (table, glyph map, bitmap) into memory, nonzero on any failure — `kore
 * --check` over the corpus is
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
  /* render the space views to memory when they exist: the glyph map, then the bitmap */
  if (space) {
    T.rows = 200; T.cols = 400;
    T.grid = xalloc((size_t)T.rows * T.cols * sizeof(Cell));
    frame_clear();
    A.worldR = (Rect){ 0, 0, 399, 199 };
    draw_space(A.worldR);
    frame_clear();
    draw_bitmap(A.worldR);
    free(T.grid);
    T.grid = NULL;
  }
  printf("ok %s n=%d lattice=%dx%d cols=%d fields=%d space=%s\n", path, A.world.n,
         A.world.latW, A.world.latH, ndcols, fields, space ? "map+bitmap" : "table-only");
  return 0;
}

/* Inputs: file.reg seg row col value. Output: 0 after splicing the cell exactly as the
 * inline edit does (guard and undo ring bypassed — point it at a copy), the touched
 * line printed; nonzero with the error printed. The other half of the --check hook:
 * edit → save → reload round-trips become a script. */
static int edit_reg(char **args) {
  char err[256];
  /* the corpus is immutable under kore, headless included */
  if (in_demos(args[0])) { fprintf(stderr, "FAIL %s: corpus file — point --edit at a copy\n", args[0]); return 1; }
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
    qsort(demoList, (size_t)nDemos, sizeof *demoList, cmp_demo);
    A.focus = F_RAIL;
    say("%d demos — enter opens, r resets the world, n steps it, : prompts", nDemos);
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
