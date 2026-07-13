// sys.rs — the one FFI module: hand-rolled extern "C" against the system libc, x86_64 linux-gnu.
// All unsafe lives here; every exported item is safe. Surface: termios raw mode with the byte-exact
// hello/bye escape strings, TIOCGWINSZ, SIGWINCH -> AtomicBool, fatal-signal terminal restore
// (kore.c on_fatal), poll+read single-byte input (rbyte), write_all stdout, the
// merged-capture child runner (run_child), the editor spawn (editor_hop),
// access/realpath/strerror. VMIN=0/VTIME=0 in the raw termios; all waiting is poll(2), as the C.

use std::cell::UnsafeCell;
use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};

// ---------- x86_64 linux-gnu layouts ----------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Termios {
    pub c_iflag: u32,
    pub c_oflag: u32,
    pub c_cflag: u32,
    pub c_lflag: u32,
    pub c_line: u8,
    pub c_cc: [u8; 32],
    pub c_ispeed: u32,
    pub c_ospeed: u32,
}

impl Termios {
    const fn zeroed() -> Termios {
        Termios { c_iflag: 0, c_oflag: 0, c_cflag: 0, c_lflag: 0, c_line: 0, c_cc: [0; 32], c_ispeed: 0, c_ospeed: 0 }
    }
}

#[repr(C)]
struct Winsize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}

#[repr(C)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
}

// glibc struct sigaction: handler, 128-byte sigset_t, flags, restorer.
#[repr(C)]
struct SigAction {
    handler: usize,
    mask: [u64; 16],
    flags: i32,
    restorer: usize,
}

// Layout pins for the x86_64 linux-gnu glibc ABI.
const _: () = assert!(std::mem::size_of::<Termios>() == 60);
const _: () = assert!(std::mem::size_of::<Winsize>() == 8);
const _: () = assert!(std::mem::size_of::<PollFd>() == 8);
const _: () = assert!(std::mem::size_of::<SigAction>() == 152);

const ISIG: u32 = 0o1;
const ICANON: u32 = 0o2;
const ECHO: u32 = 0o10;
const ICRNL: u32 = 0o400;
const IXON: u32 = 0o2000;
const VTIME: usize = 5;
const VMIN: usize = 6;
const TCSAFLUSH: i32 = 2;
const TIOCGWINSZ: u64 = 0x5413;
const POLLIN: i16 = 0x1;
const SA_RESTART: i32 = 0x1000_0000; // the C installs via signal(): BSD semantics

const SIGINT: i32 = 2;
const SIGABRT: i32 = 6;
const SIGBUS: i32 = 7;
const SIGFPE: i32 = 8;
const SIGSEGV: i32 = 11;
const SIGTERM: i32 = 15;
const SIGWINCH: i32 = 28;
const SIG_DFL: usize = 0;
const SIG_IGN: usize = 1;
const EINTR: i32 = 4;

const F_OK: i32 = 0;
const X_OK: i32 = 1;
const R_OK: i32 = 4;

unsafe extern "C" {
    fn tcgetattr(fd: i32, t: *mut Termios) -> i32;
    fn tcsetattr(fd: i32, actions: i32, t: *const Termios) -> i32;
    fn ioctl(fd: i32, request: u64, ...) -> i32;
    fn poll(fds: *mut PollFd, nfds: u64, timeout: i32) -> i32;
    fn read(fd: i32, buf: *mut u8, count: usize) -> isize;
    fn write(fd: i32, buf: *const u8, count: usize) -> isize;
    fn sigaction(sig: i32, act: *const SigAction, old: *mut SigAction) -> i32;
    fn signal(sig: i32, handler: usize) -> usize;
    fn raise(sig: i32) -> i32;
    fn fork() -> i32;
    fn pipe(fds: *mut i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn execvp(file: *const u8, argv: *const *const u8) -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn _exit(code: i32) -> !;
    fn access(path: *const u8, mode: i32) -> i32;
    #[link_name = "realpath"]
    fn realpath_c(path: *const u8, resolved: *mut u8) -> *mut u8;
    fn strerror(err: i32) -> *const u8;
    fn __errno_location() -> *mut i32;
    fn strtod(s: *const u8, end: *mut *const u8) -> f64;
    fn snprintf(buf: *mut u8, n: usize, fmt: *const u8, ...) -> i32;
}

// ---------- terminal restore state (shared with the signal handlers) ----------

// The hello/bye byte strings are protocol constants:
// alt screen on, cursor hide, button-event mouse (1002), SGR mouse encoding (1006);
// bye undoes them plus DECSCUSR reset (`\x1b[0 q`, space before q) and SGR reset.
pub const TERM_HELLO: &[u8] = b"\x1b[?1049h\x1b[?25l\x1b[?1002h\x1b[?1006h";
pub const TERM_BYE: &[u8] = b"\x1b[?1002l\x1b[?1006l\x1b[?25h\x1b[0 q\x1b[?1049l\x1b[0m";

static RESIZED: AtomicBool = AtomicBool::new(false);
static RAW_ON: AtomicBool = AtomicBool::new(false);

struct SavedTermios(UnsafeCell<Termios>);
// Written once in term_enter before RAW_ON is set; read by term_leave and the fatal handler.
unsafe impl Sync for SavedTermios {}
static SAVED: SavedTermios = SavedTermios(UnsafeCell::new(Termios::zeroed()));

fn errno() -> i32 {
    unsafe { *__errno_location() }
}

// strerror(errno) as an owned String — the exec-failure and file-error message text.
pub fn errno_str() -> String {
    unsafe {
        let p = strerror(errno());
        let mut n = 0;
        while *p.add(n) != 0 {
            n += 1;
        }
        String::from_utf8_lossy(std::slice::from_raw_parts(p, n)).into_owned()
    }
}

// ---------- raw mode ----------

// Enter raw mode and write the hello string. false when fd 0 is not a terminal
// (main prints `kore: not a terminal`, exit 2). VMIN=0/VTIME=0: reads never block,
// all waiting is poll in rbyte. OPOST untouched — the frame carries explicit \r\n.
pub fn term_enter() -> bool {
    let mut saved = Termios::zeroed();
    if unsafe { tcgetattr(0, &mut saved) } != 0 {
        return false;
    }
    unsafe { *SAVED.0.get() = saved };
    let mut t = saved;
    t.c_lflag &= !(ICANON | ECHO | ISIG);
    t.c_iflag &= !(IXON | ICRNL);
    t.c_cc[VMIN] = 0;
    t.c_cc[VTIME] = 0;
    if unsafe { tcsetattr(0, TCSAFLUSH, &t) } != 0 {
        return false;
    }
    RAW_ON.store(true, Ordering::SeqCst);
    write_stdout(TERM_HELLO);
    true
}

// Idempotent restore: bye string, then the saved termios. Safe from the fatal handler
// (write + tcsetattr are async-signal-safe); every exit path funnels here.
pub fn term_leave() {
    if !RAW_ON.swap(false, Ordering::SeqCst) {
        return;
    }
    write_stdout(TERM_BYE);
    unsafe { tcsetattr(0, TCSAFLUSH, SAVED.0.get()) };
}

extern "C" fn on_fatal(sig: i32) {
    term_leave();
    unsafe {
        signal(sig, SIG_DFL);
        raise(sig);
    }
}

extern "C" fn on_winch(_sig: i32) {
    RESIZED.store(true, Ordering::Relaxed);
}

fn install(sig: i32, handler: extern "C" fn(i32)) {
    let act = SigAction { handler: handler as usize, mask: [0; 16], flags: SA_RESTART, restorer: 0 };
    unsafe { sigaction(sig, &act, std::ptr::null_mut()) };
}

// INT/TERM/SEGV/ABRT/BUS/FPE restore-then-die; WINCH flags. A panic hook restores
// before the message prints (Rust's abort/panic stands in for the C's abort-on-OOM).
pub fn install_signals() {
    for s in [SIGINT, SIGTERM, SIGSEGV, SIGABRT, SIGBUS, SIGFPE] {
        install(s, on_fatal);
    }
    install(SIGWINCH, on_winch);
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        term_leave();
        prev(info);
    }));
}

// SIGWINCH arrived since the last take; clears the flag (the loop top's swap).
pub fn resized_take() -> bool {
    RESIZED.swap(false, Ordering::Relaxed)
}

// ioctl TIOCGWINSZ; Some((rows, cols)) only when both are positive.
// The 24x80 fallback and the 12/20 floors are the caller's (term.rs term_size).
pub fn win_size() -> Option<(i32, i32)> {
    let mut ws = Winsize { ws_row: 0, ws_col: 0, ws_xpixel: 0, ws_ypixel: 0 };
    if unsafe { ioctl(0, TIOCGWINSZ, &mut ws as *mut Winsize) } != 0 {
        return None;
    }
    if ws.ws_row > 0 && ws.ws_col > 0 { Some((ws.ws_row as i32, ws.ws_col as i32)) } else { None }
}

// ---------- input / output ----------

// One byte from fd 0 within ms milliseconds, else -1 (timeout, EINTR, or short read).
// The 100ms outer tick and 25ms inter-byte window live at the call sites (ev_read).
pub fn rbyte(ms: i32) -> i32 {
    let mut pf = PollFd { fd: 0, events: POLLIN, revents: 0 };
    if unsafe { poll(&mut pf, 1, ms) } <= 0 {
        return -1;
    }
    let mut b: u8 = 0;
    if unsafe { read(0, &mut b, 1) } != 1 {
        return -1;
    }
    b as i32
}

// write_all to fd 1: same bytes as the C's single fire-and-forget write, but short
// writes complete instead of tearing a frame; errors other than EINTR are discarded.
pub fn write_stdout(buf: &[u8]) {
    let mut off = 0;
    while off < buf.len() {
        let n = unsafe { write(1, buf.as_ptr().add(off), buf.len() - off) };
        if n < 0 && errno() == EINTR {
            continue;
        }
        if n <= 0 {
            return;
        }
        off += n as usize;
    }
}

// ---------- child processes ----------

fn cstrings(argv: &[&str]) -> Vec<CString> {
    argv.iter().map(|a| CString::new(*a).unwrap_or_default()).collect()
}

fn exit_code(status: i32) -> i32 {
    if status & 0x7f == 0 { (status >> 8) & 0xff } else { -1 }
}

// Run argv to completion, child stdout AND stderr merged onto one pipe (dup2 both —
// kernel interleaving order, exactly as kore.c run_child). Capture appended to cap.
// exec failure prints `kore: cannot exec <argv0>: <strerror>\n` through the pipe, exit 127.
// Returns the exit status, or -1 on fork/pipe failure or abnormal termination.
pub fn run_capture(argv: &[&str], cap: &mut Vec<u8>) -> i32 {
    let cargs = cstrings(argv);
    let mut ptrs: Vec<*const u8> = cargs.iter().map(|c| c.as_ptr() as *const u8).collect();
    ptrs.push(std::ptr::null());
    let mut pfd = [0i32; 2];
    if unsafe { pipe(pfd.as_mut_ptr()) } != 0 {
        return -1;
    }
    let pid = unsafe { fork() };
    if pid < 0 {
        unsafe {
            close(pfd[0]);
            close(pfd[1]);
        }
        return -1;
    }
    if pid == 0 {
        unsafe {
            close(pfd[0]);
            dup2(pfd[1], 1);
            dup2(pfd[1], 2);
            close(pfd[1]);
            execvp(ptrs[0], ptrs.as_ptr());
        }
        let msg = format!("kore: cannot exec {}: {}\n", argv.first().unwrap_or(&""), errno_str());
        write_stdout(msg.as_bytes());
        unsafe { _exit(127) }
    }
    unsafe { close(pfd[1]) };
    let mut buf = [0u8; 4096];
    loop {
        let n = unsafe { read(pfd[0], buf.as_mut_ptr(), buf.len()) };
        if n > 0 {
            cap.extend_from_slice(&buf[..n as usize]);
            continue;
        }
        if n < 0 && errno() == EINTR {
            continue;
        }
        break;
    }
    unsafe { close(pfd[0]) };
    let mut st = 0i32;
    unsafe { waitpid(pid, &mut st, 0) };
    exit_code(st)
}

// The editor hop's fork+exec+wait with SIGINT/SIGTERM parked at SIG_IGN in the parent
// (the editor owns ^C); the child restores SIG_DFL. Caller wraps with term_leave/term_enter.
// Returns the editor's exit status or -1.
pub fn spawn_wait(prog: &str, arg: &str) -> i32 {
    let old_int = unsafe { signal(SIGINT, SIG_IGN) };
    let old_term = unsafe { signal(SIGTERM, SIG_IGN) };
    let cp = CString::new(prog).unwrap_or_default();
    let ca = CString::new(arg).unwrap_or_default();
    let ptrs: [*const u8; 3] = [cp.as_ptr() as *const u8, ca.as_ptr() as *const u8, std::ptr::null()];
    let pid = unsafe { fork() };
    if pid == 0 {
        unsafe {
            signal(SIGINT, SIG_DFL);
            signal(SIGTERM, SIG_DFL);
            execvp(ptrs[0], ptrs.as_ptr());
            _exit(127)
        }
    }
    let mut st = 0i32;
    if pid > 0 {
        unsafe { waitpid(pid, &mut st, 0) };
    }
    unsafe {
        signal(SIGINT, old_int);
        signal(SIGTERM, old_term);
    }
    if pid < 0 { -1 } else { exit_code(st) }
}

// ---------- filesystem probes ----------

fn access_ok(path: &str, mode: i32) -> bool {
    match CString::new(path) {
        Ok(c) => unsafe { access(c.as_ptr() as *const u8, mode) == 0 },
        Err(_) => false,
    }
}

pub fn access_x(path: &str) -> bool {
    access_ok(path, X_OK)
}

pub fn access_r(path: &str) -> bool {
    access_ok(path, R_OK)
}

pub fn access_f(path: &str) -> bool {
    access_ok(path, F_OK)
}

// ---------- libc number spelling (byte-exact wnum/fmt_num surface) ----------

// C strtod over a byte prefix: (value, bytes consumed). Accepts hex floats, exponents,
// leading +, inf/nan spellings — exactly glibc. Bytes at and past an embedded NUL are
// unreachable, as in C. wnum's full-consume and isfinite guards are the caller's.
pub fn strtod_prefix(s: &[u8]) -> (f64, usize) {
    let mut owned: Vec<u8> = Vec::with_capacity(s.len() + 1);
    owned.extend_from_slice(s);
    owned.push(0);
    let mut end: *const u8 = std::ptr::null();
    let v = unsafe { strtod(owned.as_ptr(), &mut end) };
    let used = unsafe { end.offset_from(owned.as_ptr()) } as usize;
    (v, used.min(s.len()))
}

// snprintf "%.<prec>g" — the fmt_num shortest-round-trip loop's primitive; byte-identical
// to the C's spelling. prec clamped to [1, 17].
pub fn fmt_g(v: f64, prec: i32) -> String {
    let p = prec.clamp(1, 17);
    let fmt = format!("%.{}g\0", p);
    let mut buf = [0u8; 64];
    let n = unsafe { snprintf(buf.as_mut_ptr(), buf.len(), fmt.as_ptr(), v) };
    let n = (n.max(0) as usize).min(buf.len() - 1);
    String::from_utf8_lossy(&buf[..n]).into_owned()
}

// snprintf "%lld" for the integer window of fmt_num.
pub fn fmt_lld(v: i64) -> String {
    v.to_string()
}

// libc realpath into a PATH_MAX buffer; None on failure (call sites choose their
// fallback: raw path everywhere except in_demos, which fails closed).
pub fn real_path(path: &str) -> Option<String> {
    let c = CString::new(path).ok()?;
    let mut buf = [0u8; 4096];
    let r = unsafe { realpath_c(c.as_ptr() as *const u8, buf.as_mut_ptr()) };
    if r.is_null() {
        return None;
    }
    let len = buf.iter().position(|&b| b == 0)?;
    Some(String::from_utf8_lossy(&buf[..len]).into_owned())
}
