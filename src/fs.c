/* fs.c — path values and fortified file reading for anoc, ported and miniaturized
 * from the anoptic-engine filesystem module (src/filesystem/). Paths are checked
 * values: truncation is a detectable len == 0, never a silently clamped open target.
 * Linux primary (/proc/self/exe is kernel-resolved); macOS behind an ifdef needs the
 * realpath step. Normalization is purely lexical — symlinks are not resolved, so a
 * collapsed path prints and behaves the same whether or not the file exists. No CWD
 * mutation anywhere: anoc resolves registries against the source file and rt.bqn
 * against the exe, and the user's relative file.ano argument depends on CWD. */
#define _GNU_SOURCE
#include "ano.h"
#include <errno.h>
#include <limits.h>
#include <unistd.h>
#include <sys/stat.h>
#if defined(__APPLE__)
#include <mach-o/dyld.h>
#endif

/* Inputs: NUL-terminated s (NULL allowed). Output: checked path value; len == 0 when
 * s is NULL or does not fit. Invariant: str is always NUL-terminated. */
AnoPath fs_path(const char *s) {
  AnoPath p = {0};
  if (!s) return p;
  size_t n = strlen(s);
  if (n >= ANO_PATHSZ) return p;
  memcpy(p.str, s, n + 1);
  p.len = (unsigned)n;
  return p;
}

/* Inputs: none. Output: directory of the running binary, no file name, no trailing
 * separator; len == 0 when the exe path is unreadable or does not fit. The split is
 * hand-rolled because dirname() is not portably reentrant. */
AnoPath fs_exe_dir(void) {
  AnoPath r = {0};
  char raw[PATH_MAX];
  size_t len;
#if defined(__APPLE__)
  uint32_t rsz = sizeof raw;
  char res[PATH_MAX];
  /* the mach-o path is not kernel-resolved; realpath canonicalizes it */
  if (_NSGetExecutablePath(raw, &rsz) != 0 || !realpath(raw, res)) return r;
  len = strlen(res);
  memcpy(raw, res, len + 1);
#else
  ssize_t n = readlink("/proc/self/exe", raw, sizeof raw - 1);
  if (n <= 0 || n >= (ssize_t)sizeof raw - 1) return r;   /* exact fill = possible truncation */
  raw[n] = 0;
  len = (size_t)n;
#endif
  while (len > 0 && raw[len - 1] != '/') len--;
  if (len > 1) len--;                   /* drop the trailing slash, keep "/" at root */
  if (len == 0 || len >= ANO_PATHSZ) return r;
  memcpy(r.str, raw, len);
  r.str[len] = 0;
  r.len = (unsigned)len;
  return r;
}

/* Inputs: any path. Output: its containing directory — "." when slash-free, "/" kept
 * at root, no trailing separator otherwise; len == 0 when the input does not fit. */
AnoPath fs_dirname(const char *path) {
  AnoPath r = fs_path(path);
  if (!r.len) return path && path[0] ? r : fs_path(".");
  size_t len = r.len;
  while (len > 0 && r.str[len - 1] != '/') len--;
  if (len == 0) return fs_path(".");
  while (len > 1 && r.str[len - 1] == '/') len--;   /* strip separators, keep "/" at root */
  r.str[len] = 0;
  r.len = (unsigned)len;
  return r;
}

/* Inputs: directory, relative or absolute spec. Output: dir/rel, or rel verbatim when
 * rel is absolute; len == 0 on overflow. */
AnoPath fs_join(const char *dir, const char *rel) {
  if (rel && rel[0] == '/') return fs_path(rel);
  AnoPath r = {0};
  int n = snprintf(r.str, ANO_PATHSZ, "%s/%s", dir && dir[0] ? dir : ".", rel ? rel : "");
  if (n < 0 || n >= ANO_PATHSZ) return (AnoPath){0};
  r.len = (unsigned)n;
  return r;
}

/* Inputs: path value. Output: lexically normalized in place: "." and empty segments
 * drop, ".." pops a real segment, leading ".."s survive on relative paths and clamp
 * at root on absolute ones. Invariant: output never exceeds input length. */
void fs_norm(AnoPath *p) {
  if (!p->len) return;
  const char *s = p->str;
  int abs = s[0] == '/';
  char out[ANO_PATHSZ];
  unsigned start[ANO_PATHSZ / 2 + 1];   /* segment start offsets into out */
  size_t o = 0;
  int ns = 0;
  size_t i = abs ? 1 : 0;
  while (s[i]) {
    size_t j = i;
    while (s[j] && s[j] != '/') j++;
    size_t len = j - i;
    int dotdot = len == 2 && s[i] == '.' && s[i + 1] == '.';
    if (len == 0 || (len == 1 && s[i] == '.')) {
      /* skip */
    } else if (dotdot && ns &&
               !(o - start[ns - 1] == 2 && out[start[ns - 1]] == '.' && out[start[ns - 1] + 1] == '.')) {
      ns--;                             /* pop a real segment (and its separator) */
      o = ns ? start[ns] - 1 : 0;
    } else if (dotdot && abs && ns == 0) {
      /* clamp at root */
    } else {
      if (o) out[o++] = '/';
      start[ns++] = (unsigned)o;
      memcpy(out + o, s + i, len);
      o += len;
    }
    i = s[j] ? j + 1 : j;
  }
  if (abs) {
    p->str[0] = '/';
    memcpy(p->str + 1, out, o);
    p->str[1 + o] = 0;
    p->len = (unsigned)(1 + o);
  } else if (o == 0) {
    p->str[0] = '.'; p->str[1] = 0;
    p->len = 1;
  } else {
    memcpy(p->str, out, o);
    p->str[o] = 0;
    p->len = (unsigned)o;
  }
}

/* Inputs: path value. Output: kernel-resolved canonical path when the target exists
 * (realpath: symlinks and dots resolved, so what opens is exactly what the kernel
 * sees — old anoc's fopen semantics); lexical collapse otherwise, so a missing file
 * still reports a clean path with the right errno. */
void fs_canon(AnoPath *p) {
  char res[PATH_MAX];
  if (p->len && realpath(p->str, res)) {
    size_t n = strlen(res);
    if (n < ANO_PATHSZ) { memcpy(p->str, res, n + 1); p->len = (unsigned)n; return; }
  }
  fs_norm(p);
}

/* Inputs: path, arena, optional out byte count. Output: NUL-terminated contents in
 * the arena, or NULL with errno telling the story (EISDIR for a directory, EIO for a
 * short read, EINVAL for other non-regular files). Invariants: rejects non-regular
 * files before trusting ftell; errno survives fclose. The one reader — main.c and
 * registry.c both go through it. */
char *fs_read(const char *path, Arena *a, size_t *lenOut) {
  FILE *f = fopen(path, "rb");
  if (!f) return NULL;
  struct stat st;
  if (fstat(fileno(f), &st)) { int e = errno; fclose(f); errno = e; return NULL; }
  if (!S_ISREG(st.st_mode)) { fclose(f); errno = S_ISDIR(st.st_mode) ? EISDIR : EINVAL; return NULL; }
  if (fseek(f, 0, SEEK_END)) { int e = errno; fclose(f); errno = e; return NULL; }
  long sz = ftell(f);
  if (sz < 0 || fseek(f, 0, SEEK_SET)) { int e = errno; fclose(f); errno = e; return NULL; }
  char *buf = (char *)arena_alloc(a, (size_t)sz + 1);
  size_t got = fread(buf, 1, (size_t)sz, f);
  if (got != (size_t)sz) { int e = ferror(f) ? errno : EIO; fclose(f); errno = e; return NULL; }
  fclose(f);
  buf[got] = 0;
  if (lenOut) *lenOut = got;
  return buf;
}
