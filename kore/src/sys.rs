// sys.rs — the syscall boundary rides nix for target-aware layouts and constants.
// All unsafe lives here; every exported item is safe. The only raw externs left are
// strtod/snprintf, which preserve the byte-exact numeric-spelling contract.
// Surface: termios raw mode, TIOCGWINSZ, signal handling, polling and byte I/O,
// merged-capture and editor child runners, filesystem probes, and errno text.

use nix::errno::Errno;
use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
use nix::sys::signal::{SaFlags, SigAction, SigHandler, SigSet, Signal, raise, sigaction, signal};
use nix::sys::termios::{
    InputFlags, LocalFlags, SetArg, SpecialCharacterIndices, Termios, tcgetattr, tcsetattr,
};
use nix::sys::wait::{WaitStatus, waitpid};
use nix::unistd::{
    AccessFlags, ForkResult, access, dup2_stderr, dup2_stdout, execvp, fork, pipe, read, write,
};
use std::cell::UnsafeCell;
use std::ffi::CString;
use std::os::unix::io::{AsFd, BorrowedFd};
use std::sync::atomic::{AtomicBool, Ordering};

#[repr(C)]
struct Winsize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}

nix::ioctl_read_bad!(tiocgwinsz, nix::libc::TIOCGWINSZ, Winsize);

unsafe extern "C" {
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

struct SavedTermios(UnsafeCell<Option<Termios>>);
// Written once in term_enter before RAW_ON is set; read by term_leave and the fatal handler.
unsafe impl Sync for SavedTermios {}
static SAVED: SavedTermios = SavedTermios(UnsafeCell::new(None));

// strerror(errno) as an owned String — the exec-failure and file-error message text.
pub fn errno_str() -> String {
    Errno::last().desc().to_string()
}

// ---------- raw mode ----------

// Enter raw mode and write the hello string. false when fd 0 is not a terminal
// (main prints `kore: not a terminal`, exit 2). VMIN=0/VTIME=0: reads never block,
// all waiting is poll in rbyte. OPOST untouched — the frame carries explicit \r\n.
pub fn term_enter() -> bool {
    let input = std::io::stdin();
    let saved = match tcgetattr(input.as_fd()) {
        Ok(saved) => saved,
        Err(_) => return false,
    };
    let mut t = saved.clone();
    unsafe { *SAVED.0.get() = Some(saved) };
    t.local_flags
        .remove(LocalFlags::ICANON | LocalFlags::ECHO | LocalFlags::ISIG);
    t.input_flags.remove(InputFlags::IXON | InputFlags::ICRNL);
    t.control_chars[SpecialCharacterIndices::VMIN as usize] = 0;
    t.control_chars[SpecialCharacterIndices::VTIME as usize] = 0;
    if tcsetattr(input.as_fd(), SetArg::TCSAFLUSH, &t).is_err() {
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
    if let Some(saved) = unsafe { &*SAVED.0.get() } {
        let input = unsafe { BorrowedFd::borrow_raw(nix::libc::STDIN_FILENO) };
        let _ = tcsetattr(input, SetArg::TCSAFLUSH, saved);
    }
}

extern "C" fn on_fatal(sig: i32) {
    term_leave();
    if let Ok(sig) = Signal::try_from(sig) {
        unsafe {
            let _ = signal(sig, SigHandler::SigDfl);
        }
        let _ = raise(sig);
    }
}

extern "C" fn on_winch(_sig: i32) {
    RESIZED.store(true, Ordering::Relaxed);
}

fn install(sig: Signal, handler: extern "C" fn(i32)) {
    let act = SigAction::new(
        SigHandler::Handler(handler),
        SaFlags::SA_RESTART,
        SigSet::empty(),
    );
    unsafe {
        let _ = sigaction(sig, &act);
    }
}

// INT/TERM/SEGV/ABRT/BUS/FPE restore-then-die; WINCH flags. A panic hook restores
// before the message prints (Rust's abort/panic stands in for the C's abort-on-OOM).
pub fn install_signals() {
    for sig in [
        Signal::SIGINT,
        Signal::SIGTERM,
        Signal::SIGSEGV,
        Signal::SIGABRT,
        Signal::SIGBUS,
        Signal::SIGFPE,
    ] {
        install(sig, on_fatal);
    }
    install(Signal::SIGWINCH, on_winch);
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
    let mut ws = Winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    if unsafe { tiocgwinsz(0, &mut ws) }.is_err() {
        return None;
    }
    if ws.ws_row > 0 && ws.ws_col > 0 {
        Some((ws.ws_row as i32, ws.ws_col as i32))
    } else {
        None
    }
}

// ---------- input / output ----------

// One byte from fd 0 within ms milliseconds, else -1 (timeout, EINTR, or short read).
// The 100ms outer tick and 25ms inter-byte window live at the call sites (ev_read).
pub fn rbyte(ms: i32) -> i32 {
    let input = std::io::stdin();
    let mut fds = [PollFd::new(input.as_fd(), PollFlags::POLLIN)];
    let timeout = PollTimeout::try_from(ms).unwrap_or(PollTimeout::NONE);
    if poll(&mut fds, timeout).unwrap_or(0) <= 0 {
        return -1;
    }
    let mut b = [0u8; 1];
    if read(input.as_fd(), &mut b) != Ok(1) {
        return -1;
    }
    b[0] as i32
}

// write_all to fd 1: same bytes as the C's single fire-and-forget write, but short
// writes complete instead of tearing a frame; errors other than EINTR are discarded.
pub fn write_stdout(buf: &[u8]) {
    let output = unsafe { BorrowedFd::borrow_raw(nix::libc::STDOUT_FILENO) };
    let mut off = 0;
    while off < buf.len() {
        match write(output, &buf[off..]) {
            Err(Errno::EINTR) => continue,
            Ok(0) | Err(_) => return,
            Ok(n) => off += n,
        }
    }
}

// ---------- child processes ----------

fn cstrings(argv: &[&str]) -> Vec<CString> {
    argv.iter()
        .map(|a| CString::new(*a).unwrap_or_default())
        .collect()
}

// Run argv to completion, child stdout AND stderr merged onto one pipe (dup2 both —
// kernel interleaving order, exactly as kore.c run_child). Capture appended to cap.
// exec failure prints `kore: cannot exec <argv0>: <strerror>\n` through the pipe, exit 127.
// Returns the exit status, or -1 on fork/pipe failure or abnormal termination.
pub fn run_capture(argv: &[&str], cap: &mut Vec<u8>) -> i32 {
    let cargs = cstrings(argv);
    let (read_end, write_end) = match pipe() {
        Ok(pipe) => pipe,
        Err(_) => return -1,
    };
    match unsafe { fork() } {
        Err(_) => -1,
        Ok(ForkResult::Child) => {
            drop(read_end);
            let _ = dup2_stdout(&write_end);
            let _ = dup2_stderr(&write_end);
            drop(write_end);
            match execvp(&cargs[0], &cargs) {
                Ok(_) => unreachable!(),
                Err(e) => {
                    let msg = format!(
                        "kore: cannot exec {}: {}\n",
                        argv.first().unwrap_or(&""),
                        e.desc()
                    );
                    write_stdout(msg.as_bytes());
                    unsafe { nix::libc::_exit(127) }
                }
            }
        }
        Ok(ForkResult::Parent { child }) => {
            drop(write_end);
            let mut buf = [0u8; 4096];
            loop {
                match read(&read_end, &mut buf) {
                    Ok(0) => break,
                    Ok(n) => cap.extend_from_slice(&buf[..n]),
                    Err(Errno::EINTR) => continue,
                    Err(_) => break,
                }
            }
            drop(read_end);
            match waitpid(child, None) {
                Ok(WaitStatus::Exited(_, code)) => code,
                _ => -1,
            }
        }
    }
}

// The editor hop's fork+exec+wait with SIGINT/SIGTERM parked at SIG_IGN in the parent
// (the editor owns ^C); the child restores SIG_DFL. Caller wraps with term_leave/term_enter.
// Returns the editor's exit status or -1.
pub fn spawn_wait(prog: &str, arg: &str) -> i32 {
    let old_int = match unsafe { signal(Signal::SIGINT, SigHandler::SigIgn) } {
        Ok(handler) => handler,
        Err(_) => return -1,
    };
    let old_term = match unsafe { signal(Signal::SIGTERM, SigHandler::SigIgn) } {
        Ok(handler) => handler,
        Err(_) => {
            unsafe {
                let _ = signal(Signal::SIGINT, old_int);
            }
            return -1;
        }
    };
    let cp = CString::new(prog).unwrap_or_default();
    let ca = CString::new(arg).unwrap_or_default();
    let cargs = [cp, ca];
    let result = match unsafe { fork() } {
        Err(_) => -1,
        Ok(ForkResult::Child) => {
            unsafe {
                let _ = signal(Signal::SIGINT, SigHandler::SigDfl);
                let _ = signal(Signal::SIGTERM, SigHandler::SigDfl);
            }
            let _ = execvp(&cargs[0], &cargs);
            unsafe { nix::libc::_exit(127) }
        }
        Ok(ForkResult::Parent { child }) => match waitpid(child, None) {
            Ok(WaitStatus::Exited(_, code)) => code,
            _ => -1,
        },
    };
    unsafe {
        let _ = signal(Signal::SIGINT, old_int);
        let _ = signal(Signal::SIGTERM, old_term);
    }
    result
}

// ---------- filesystem probes ----------

fn access_ok(path: &str, mode: AccessFlags) -> bool {
    access(path, mode).is_ok()
}

pub fn access_x(path: &str) -> bool {
    access_ok(path, AccessFlags::X_OK)
}

pub fn access_r(path: &str) -> bool {
    access_ok(path, AccessFlags::R_OK)
}

pub fn access_f(path: &str) -> bool {
    access_ok(path, AccessFlags::F_OK)
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

// std::fs::canonicalize to an absolute normalized path, then strict UTF-8 conversion;
// None on either filesystem failure or a non-UTF-8 result.
pub fn real_path(path: &str) -> Option<String> {
    std::fs::canonicalize(path)
        .ok()?
        .into_os_string()
        .into_string()
        .ok()
}
