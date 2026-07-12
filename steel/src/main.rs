// main.rs — the steel driver. Mirrors src/main.c: CLI scan, --! directive extraction (lines
// blanked to spaces, newlines kept), registry resolution beside the source file, the
// lex -> parse -> emit pipeline, --tokens / --dump / --run / --save modes, run_bqn with the
// per-line 0x1E capture (0x1D label and 0x1F trace lines forward verbatim), the save
// pipe-back, and the exit-code table: 0 ok, 1 test failure (same-tokens mismatch; bqn's own
// code passes VERBATIM), 2 usage/compile/IO/save refusal, 127 bqn exec failure.

use std::io::{self, Read, Write};
use std::process::ExitCode;

use steel::{
    ANO_NAMESZ, BindKind, ColType, Diag, Directives, Expect, Interner, RegEntryKind, Registry,
    TokKind, Toks, emit, fs, lex, num, parse, registry,
};

const MAXEXPECT: usize = 64;
const MAXVALS: usize = 8192;

// rt.bqn embedded at compile time; --rt <path> overrides at runtime (fs_path checked, NO
// canon, fortified read, C diagnostics). The newline guarantee — append '\n' iff nonempty
// and the last byte isn't one — applies to the embedded runtime and a --rt file alike, in
// both the emit-mode stdout and the --run temp file.
// One canonical rt.bqn, kept at src/ beside the C binary that reads it at runtime.
const RT_BQN: &str = include_str!("../../src/rt.bqn");

fn main() -> ExitCode {
    ExitCode::from((run() & 0xff) as u8)
}

// Inputs: argv. Output: the process exit code.
// Invariants: diagnostics to stderr as "<path>: <msg>" (path exactly as given, never
// canonicalized) except "steel: --save requires --run"; flag scan order and post-scan check
// order exactly as main.c; the source buffer truncates at its first NUL byte before
// directive parsing (C-string fidelity); --tokens returns before parse; --dump happens
// before lexing and exits 0 when no other mode was asked.
fn run() -> i32 {
    let args: Vec<String> =
        std::env::args_os().skip(1).map(|a| a.to_string_lossy().into_owned()).collect();
    let (mut mode_tokens, mut mode_run, mut mode_emit) = (false, false, false);
    let (mut label_flag, mut trace_flag) = (false, false);
    let mut rt_flag: Option<String> = None;
    let mut reg_flag: Option<String> = None;
    let mut dump_flag: Option<String> = None;
    let mut save_flag: Option<String> = None;
    let mut path: Option<String> = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--tokens" => mode_tokens = true,
            "--emit" => mode_emit = true, // the default mode; tracked so an explicit ask
            // survives --dump
            "--run" => mode_run = true,
            "--label" => label_flag = true, // labeled query display; independent of --run/--save
            "--trace" => trace_flag = true, // 0x1F diagnostic lines; never changes post-state
            "--dump" => {
                i += 1;
                if i >= args.len() {
                    return usage();
                }
                dump_flag = Some(args[i].clone());
            }
            "--save" => {
                i += 1;
                if i >= args.len() {
                    return usage();
                }
                save_flag = Some(args[i].clone());
            }
            "--rt" => {
                i += 1;
                if i >= args.len() {
                    return usage();
                }
                rt_flag = Some(args[i].clone());
            }
            "--registry" => {
                i += 1;
                if i >= args.len() {
                    return usage();
                }
                reg_flag = Some(args[i].clone());
            }
            s => {
                let b = s.as_bytes();
                if b.len() > 1 && b[0] == b'-' {
                    return usage();
                }
                if path.is_none() {
                    path = Some(args[i].clone());
                } else {
                    return usage();
                }
            }
        }
        i += 1;
    }
    let Some(path) = path else { return usage() };
    if save_flag.is_some() && !mode_run {
        eprintln!("steel: --save requires --run");
        return 2;
    }

    let mut src = match fs::fs_read(&path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}: cannot read: {}", path, fs::strerror(&e));
            return 2;
        }
    };
    // C-string fidelity: everything past the first NUL is invisible downstream
    if let Some(p) = src.iter().position(|&b| b == 0) {
        src.truncate(p);
    }

    let mut dirs = Directives::default();
    if let Err(d) = parse_directives(&mut src, &mut dirs) {
        eprintln!("{}: {}", path, d.msg);
        return 2;
    }

    // registry: flag wins over directive. A '/'-bearing or .reg-suffixed spec is a literal
    // path against the source file's dir; a bare name resolves as <dir>/<name>.reg.
    let mut reg = Registry::default();
    let rspec: Option<String> =
        reg_flag.or_else(|| (!dirs.registry.is_empty()).then(|| dirs.registry.clone()));
    if let Some(rspec) = &rspec {
        let is_path = rspec.contains('/') || (rspec.len() > 4 && rspec.ends_with(".reg"));
        let spec = if is_path { rspec.clone() } else { format!("{}.reg", rspec) };
        let regp = fs::fs_dirname(&path)
            .filter(|_| !spec.is_empty() && spec.len() < fs::ANO_PATHSZ)
            .and_then(|d| fs::fs_join(&d.s, &spec)) // absolute rspec passes verbatim
            .map(|mut p| {
                fs::fs_canon(&mut p);
                p
            });
        let Some(regp) = regp else {
            eprintln!("{}: registry path too long: {}", path, rspec);
            return 2;
        };
        reg = match registry::reg_load(&regp.s) {
            Ok(r) => r,
            Err(d) => {
                eprintln!("{}: {}", path, d.msg);
                return 2;
            }
        };
    }

    // the write-out half of the commit loop: dump the loaded fixture state and, with no
    // other mode asked for, stop — the dump was the job
    if let Some(dump) = &dump_flag {
        if rspec.is_none() {
            eprintln!("{}: --dump needs a registry", path);
            return 2;
        }
        if let Err(d) = registry::reg_dump(&reg, dump) {
            eprintln!("{}: {}", path, d.msg);
            return 2;
        }
        if !mode_run && !mode_tokens && !mode_emit {
            return 0;
        }
    }
    if save_flag.is_some() && rspec.is_none() {
        eprintln!("{}: --save needs a registry", path);
        return 2;
    }
    dirs.save = save_flag.is_some();
    dirs.label = label_flag;
    dirs.trace = trace_flag;

    let mut it = Interner::new();
    let toks = match lex::lex(&src, dirs.ja, &mut it) {
        Ok(t) => t,
        Err(d) => {
            eprintln!("{}: {}", path, d.msg);
            return 2;
        }
    };

    if mode_tokens {
        let so = io::stdout();
        let mut so = so.lock();
        let _ = print_toks(&mut so, &toks, &it);
        let _ = so.flush();
        return 0;
    }

    // ex40 equivalence: the ASCII directive line must lex to the file's own stream
    if !dirs.same_tokens.is_empty() {
        let dtoks = match lex::lex(dirs.same_tokens.as_bytes(), false, &mut it) {
            Ok(t) => t,
            Err(d) => {
                eprintln!("{}: same-tokens: {}", path, d.msg);
                return 2;
            }
        };
        if !same_stream(&toks, &dtoks, &reg, &it) {
            let se = io::stderr();
            let mut se = se.lock();
            let _ = write!(se, "{}: same-tokens mismatch\n-- file stream:\n", path);
            let _ = print_toks(&mut se, &toks, &it);
            let _ = se.write_all(b"-- directive stream:\n");
            let _ = print_toks(&mut se, &dtoks, &it);
            return 1;
        }
    }

    let prog = match parse::parse(&toks, &mut it) {
        Ok(p) => p,
        Err(d) => {
            eprintln!("{}: {}", path, d.msg);
            return 2;
        }
    };

    let out = match emit::emit(&prog, &reg, &dirs, &it) {
        Ok(s) => s,
        Err(d) => {
            eprintln!("{}: {}", path, d.msg);
            return 2;
        }
    };

    let rt: std::borrow::Cow<[u8]> = if let Some(rtf) = &rt_flag {
        let Some(rtp) = fs::fs_path(rtf) else {
            eprintln!("{}: runtime path too long", path);
            return 2;
        };
        match fs::fs_read(&rtp.s) {
            Ok(v) => std::borrow::Cow::Owned(v),
            Err(e) => {
                eprintln!("{}: cannot read runtime {}: {}", path, rtp.s, fs::strerror(&e));
                return 2;
            }
        }
    } else {
        std::borrow::Cow::Borrowed(RT_BQN.as_bytes())
    };

    if mode_run {
        // the pipe-back: on exit 0 the captured sentinel lines patch the loaded Registry
        // to post-state and the world commits through reg_dump (staged, rename, atomic);
        // on a nonzero child exit nothing is written
        let saving = save_flag.is_some();
        let mut cap: Vec<u8> = Vec::new();
        let mut code = run_bqn(&path, &rt, &out, if saving { Some(&mut cap) } else { None });
        if saving && code == 0 {
            // an exit-0 child that never reached the serializer (an early •Exit) must not
            // pass its pre-state off as post-state — no sentinel lines, no save
            if cap.is_empty() {
                eprintln!(
                    "{}: save: the program printed no post-state (child exited early?)",
                    path
                );
                code = 2;
            } else if let Err(d) = save_patch(&mut reg, &cap)
                .and_then(|()| registry::reg_dump(&reg, save_flag.as_deref().unwrap_or("")))
            {
                eprintln!("{}: {}", path, d.msg);
                code = 2;
            }
        }
        code
    } else {
        let so = io::stdout();
        let mut so = so.lock();
        let _ = so.write_all(&rt);
        if !rt.is_empty() && rt.last() != Some(&b'\n') {
            let _ = so.write_all(b"\n");
        }
        let _ = so.write_all(out.as_bytes());
        let _ = so.flush();
        0
    }
}

// Output: prints the usage line to stderr (trailing newline), returns 2. Byte-exact:
// usage: steel [--tokens] [--emit] [--run] [--label] [--trace] [--dump <path>] [--save <path>] [--rt <path>] [--registry <path-or-name>] file.ano
fn usage() -> i32 {
    eprintln!(
        "usage: steel [--tokens] [--emit] [--run] [--label] [--trace] [--dump <path>] [--save <path>] [--rt <path>] [--registry <path-or-name>] file.ano"
    );
    2
}

// The one directive/save word scanner (main.c word): space/tab separated; a returned word
// leaves the cursor exactly one separator past its end, so rest() is the raw payload tail.
struct Cursor<'a> {
    s: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(s: &'a [u8]) -> Cursor<'a> {
        Cursor { s, pos: 0 }
    }

    fn word(&mut self) -> Option<&'a [u8]> {
        while self.pos < self.s.len() && (self.s[self.pos] == b' ' || self.s[self.pos] == b'\t') {
            self.pos += 1;
        }
        if self.pos >= self.s.len() {
            return None;
        }
        let start = self.pos;
        while self.pos < self.s.len() && self.s[self.pos] != b' ' && self.s[self.pos] != b'\t' {
            self.pos += 1;
        }
        let w = &self.s[start..self.pos];
        if self.pos < self.s.len() {
            self.pos += 1; // past exactly one separator
        }
        Some(w)
    }

    fn rest(&self) -> &'a [u8] {
        &self.s[self.pos..]
    }

    fn rest_after_ws(&mut self) -> &'a [u8] {
        while self.pos < self.s.len() && (self.s[self.pos] == b' ' || self.s[self.pos] == b'\t') {
            self.pos += 1;
        }
        &self.s[self.pos..]
    }
}

// Inputs: raw bytes. Output: an owned String, invalid UTF-8 replaced (out-of-corpus
// divergence — the corpus is UTF-8, the C kept raw bytes).
fn lossy(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

// Inputs: raw bytes, byte cap. Output: lossy String of the first cap bytes — the C
// snprintf truncation into a fixed buffer (cap = size - 1).
fn trunc_lossy(b: &[u8], cap: usize) -> String {
    lossy(&b[..b.len().min(cap)])
}

// Inputs: a word. Output: C atoi — optional isspace skip, optional sign, leading digits,
// 0 on none; no error detection, partial parse OK.
fn atoi(s: &[u8]) -> i32 {
    let mut i = 0usize;
    while i < s.len() && (s[i] == b' ' || (9..=13).contains(&s[i])) {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let mut v: i64 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        v = v.saturating_mul(10).saturating_add(i64::from(s[i] - b'0'));
        i += 1;
    }
    let v = if neg { -v } else { v };
    v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

// Inputs: one directive tail after "--!" (stripped), directives. Output: () or Diag
// ("directive: ..." texts). Grammar: registry N | expect col [=] v... | expect-n K |
// out v... | ja | same-tokens <rest verbatim>. Values stay raw words; registry truncates
// at 255 bytes, same-tokens at 511 (snprintf fidelity); expects cap 64, values beyond
// 8192 silently dropped.
fn dir_line(s: &[u8], dirs: &mut Directives) -> Result<(), Diag> {
    let mut c = Cursor::new(s);
    let Some(key) = c.word() else { return Ok(()) }; // bare --!
    if key == b"registry" {
        let Some(v) = c.word() else {
            return Err(Diag::refuse("directive: registry needs a name"));
        };
        dirs.registry = trunc_lossy(v, 255);
    } else if key == b"ja" {
        dirs.ja = true;
    } else if key == b"expect-n" {
        let Some(v) = c.word() else {
            return Err(Diag::refuse("directive: expect-n needs a count"));
        };
        dirs.expect_n = atoi(v);
    } else if key == b"same-tokens" {
        dirs.same_tokens = trunc_lossy(c.rest_after_ws(), 511);
    } else if key == b"expect" || key == b"out" {
        if dirs.expects.len() >= MAXEXPECT {
            return Err(Diag::refuse("directive: too many expects"));
        }
        let is_out = key == b"out";
        let mut col = String::new();
        let mut w = if is_out {
            c.word()
        } else {
            let Some(cw) = c.word() else {
                return Err(Diag::refuse("directive: expect needs a column"));
            };
            col = trunc_lossy(cw, ANO_NAMESZ - 1);
            let mut w = c.word();
            if w == Some(b"=".as_slice()) {
                w = c.word(); // optional '='
            }
            w
        };
        let mut vals: Vec<String> = Vec::new();
        while let Some(v) = w {
            if vals.len() < MAXVALS {
                vals.push(lossy(v));
            }
            w = c.word();
        }
        dirs.expects.push(if is_out { Expect::Out { vals } } else { Expect::Col { col, vals } });
    } else {
        return Err(Diag::refuse(format!("directive: unknown key '{}'", lossy(key))));
    }
    Ok(())
}

// Inputs: mutable source bytes, directives out. Output: () or Diag ("directive: ..." texts).
// Invariants: per line, a leading-whitespace-trimmed "--!" hands its stripped tail to the
// directive grammar then blanks the WHOLE original line (indent included) to spaces —
// newlines survive so lexer line numbers hold; registry truncates at 255 bytes, same-tokens
// at 511 (snprintf fidelity); expects cap 64 ("directive: too many expects"), values beyond
// 8192 silently dropped; expect-n parses via atoi semantics (leading sign + digits, 0 on none).
fn parse_directives(src: &mut [u8], dirs: &mut Directives) -> Result<(), Diag> {
    let total = src.len();
    let mut line = 0usize;
    while line < total {
        let end = src[line..].iter().position(|&b| b == b'\n').map(|p| line + p);
        let len = end.unwrap_or(total) - line;
        let mut p = line;
        while p < line + len && (src[p] == b' ' || src[p] == b'\t') {
            p += 1;
        }
        if (p - line) + 3 <= len && src[p..p + 3] == *b"--!" {
            let mut buf = src[p + 3..line + len].to_vec();
            while matches!(buf.last(), Some(b'\r' | b' ' | b'\t')) {
                buf.pop();
            }
            dir_line(&buf, dirs)?;
            src[line..line + len].fill(b' ');
        }
        match end {
            Some(e) => line = e + 1,
            None => break,
        }
    }
    Ok(())
}

// Inputs: sink, token stream, interner. Output: one line per token (Eof included):
// "%s %s %g\n" — TokKind::c_name, the name column (empty field when absent, giving two
// spaces), num through C %g (num::fmt_g(6, x)).
fn print_toks(w: &mut dyn std::io::Write, toks: &Toks, it: &Interner) -> std::io::Result<()> {
    for i in 0..toks.len() {
        writeln!(
            w,
            "{} {} {}",
            toks.kind[i].c_name(),
            it.resolve(toks.name[i]),
            num::fmt_g(6, toks.num[i])
        )?;
    }
    Ok(())
}

// Inputs: file stream, directive stream, registry, interner. Output: true when equal.
// Invariants: both cursors skip Nl and Eof; kinds match exactly, nums compare ==; names
// strcmp-equal pass, else ONLY Name/Alias kinds get the resolver — registry::reg_find on
// both spellings must yield the same entry INDEX (values never fold).
fn same_stream(a: &Toks, b: &Toks, reg: &Registry, it: &Interner) -> bool {
    let (mut i, mut j) = (0usize, 0usize);
    loop {
        while i < a.len() && matches!(a.kind[i], TokKind::Nl | TokKind::Eof) {
            i += 1;
        }
        while j < b.len() && matches!(b.kind[j], TokKind::Nl | TokKind::Eof) {
            j += 1;
        }
        if i >= a.len() || j >= b.len() {
            return i >= a.len() && j >= b.len();
        }
        if a.kind[i] != b.kind[j] || a.num[i] != b.num[j] {
            return false;
        }
        if a.name[i] != b.name[j] {
            // only NAME/ALIAS payloads compare through the resolver — sym, string, and
            // counter payloads are values, and values never fold (the case contract)
            if a.kind[i] != TokKind::Name && a.kind[i] != TokKind::Alias {
                return false;
            }
            let ex = registry::reg_find(reg, it.resolve(a.name[i]));
            let ey = registry::reg_find(reg, it.resolve(b.name[j]));
            if ex.is_none() || ex != ey {
                return false;
            }
        }
        i += 1;
        j += 1;
    }
}

// Output: six [A-Za-z0-9] chars — the mkstemps XXXXXX stand-in; entropy from time, pid,
// and RandomState, salted per attempt.
fn rand6(salt: u64) -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() ^ u64::from(d.subsec_nanos()))
        .unwrap_or(0);
    h.write_u64(now);
    h.write_u32(std::process::id());
    h.write_u64(salt);
    let mut v = h.finish();
    const CS: &[u8; 62] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    (0..6)
        .map(|_| {
            let c = CS[(v % 62) as usize] as char;
            v /= 62;
            c
        })
        .collect()
}

// Inputs: source path (messages only), rt bytes, emitted program, capture sink (None = child
// inherits stdout). Output: bqn's exit code verbatim (127 exec failure, 1 signal death;
// 2 on temp-file/pipe/spawn plumbing failure).
// Invariants: temp file "${TMPDIR:-/tmp}/steel-XXXXXX.bqn" (std has no mkstemps — create_new
// loop with a randomized middle, name shape kept for the "kept %s" message); contents rt +
// newline-guarantee + program; bqn found via PATH; the capture loop is BYTE-oriented — split
// child stdout on '\n', a line whose FIRST byte is 0x1E goes to cap sentinel-stripped with
// its newline kept (an unterminated 0x1E tail gains one), every other line (0x1D and 0x1F
// included) forwards to stdout verbatim; exit 0 unlinks the temp file, nonzero keeps it and
// prints "%s: bqn exited %d, kept %s" to stderr.
fn run_bqn(path: &str, rt: &[u8], prog: &str, cap: Option<&mut Vec<u8>>) -> i32 {
    let tdir = match std::env::var("TMPDIR") {
        Ok(s) if !s.is_empty() => s,
        _ => String::from("/tmp"),
    };
    let (tmpl, mut f) = {
        let mut made = None;
        let mut last: Option<io::Error> = None;
        for salt in 0..100u64 {
            let name = format!("{}/steel-{}.bqn", tdir, rand6(salt));
            match std::fs::OpenOptions::new().write(true).create_new(true).open(&name) {
                Ok(f) => {
                    made = Some((name, f));
                    break;
                }
                Err(e) => {
                    let retry = e.kind() == io::ErrorKind::AlreadyExists;
                    last = Some(e);
                    if !retry {
                        break;
                    }
                }
            }
        }
        match made {
            Some(t) => t,
            None => {
                let e = last.unwrap_or_else(|| io::Error::from(io::ErrorKind::AlreadyExists));
                eprintln!("{}: cannot create temp file in {}: {}", path, tdir, fs::strerror(&e));
                return 2;
            }
        }
    };
    let wrote = (|| -> io::Result<()> {
        f.write_all(rt)?;
        if !rt.is_empty() && rt[rt.len() - 1] != b'\n' {
            f.write_all(b"\n")?;
        }
        f.write_all(prog.as_bytes())
    })();
    drop(f);
    if let Err(e) = wrote {
        let _ = std::fs::remove_file(&tmpl);
        eprintln!("{}: write {}: {}", path, tmpl, fs::strerror(&e));
        return 2;
    }
    let mut cmd = std::process::Command::new("bqn");
    cmd.arg(&tmpl);
    if cap.is_some() {
        cmd.stdout(std::process::Stdio::piped());
    }
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            // the C child's exec failure: its message, then the parent's kept-file verdict
            eprintln!("{}: cannot exec bqn: {}", path, fs::strerror(&e));
            eprintln!("{}: bqn exited 127, kept {}", path, tmpl);
            return 127;
        }
    };
    if let Some(cap) = cap {
        // read loop: split on newlines; a line opening with 0x1E collects (sentinel
        // stripped, newline kept), anything else forwards verbatim
        if let Some(mut cs) = child.stdout.take() {
            let so = io::stdout();
            let mut so = so.lock();
            let mut acc: Vec<u8> = Vec::new();
            let mut rb = [0u8; 8192];
            loop {
                let got = match cs.read(&mut rb) {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                };
                acc.extend_from_slice(&rb[..got]);
                let mut start = 0usize;
                for j in 0..acc.len() {
                    if acc[j] != b'\n' {
                        continue;
                    }
                    if acc[start] == 0x1E {
                        cap.extend_from_slice(&acc[start + 1..=j]);
                    } else {
                        let _ = so.write_all(&acc[start..=j]);
                    }
                    start = j + 1;
                }
                acc.drain(..start);
            }
            if !acc.is_empty() {
                // unterminated tail
                if acc[0] == 0x1E {
                    cap.extend_from_slice(&acc[1..]);
                    cap.push(b'\n');
                } else {
                    let _ = so.write_all(&acc);
                }
            }
            let _ = so.flush();
        }
    }
    let code = match child.wait() {
        Ok(st) => st.code().unwrap_or(1),
        Err(_) => 1,
    };
    if code == 0 {
        let _ = std::fs::remove_file(&tmpl);
    } else {
        eprintln!("{}: bqn exited {}, kept {}", path, code, tmpl);
    }
    code
}

// Inputs: one wire word. Output: Some(finite value) iff strtod consumes the whole word and
// the result is finite — main.c save_num; the finite gate keeps a NaN tick refused, world
// standing (CBQN spells overflow '∞', which refuses at parse).
fn save_num(w: &[u8]) -> Option<f64> {
    std::str::from_utf8(w).ok().and_then(num::wnum)
}

// Inputs: mutable registry, one 0x1E-stripped capture line. Output: () or Diag ("save: ..."
// texts, byte-exact).
// Invariants: "n <k>" first resets the row count (later counts check the POST-state n);
// line kinds col/field/pres/rel/srel only; numbers gate through num::wnum (the finite seal —
// CBQN's "NaN" parses then refuses, "∞" refuses at parse; both quote the exact word); char
// payloads are the raw tail after name + exactly one separator, spaces kept, byte-length
// checked, and the .reg-spelling guard (empty, boundary whitespace, word-boundary '#')
// refuses; arrays allocate fresh (n may grow); an srel patch clears inv_of.
fn save_line(reg: &mut Registry, line: &[u8]) -> Result<(), Diag> {
    let mut c = Cursor::new(line);
    let Some(k) = c.word() else { return Ok(()) };
    if k == b"n" {
        return match c.word().and_then(save_num) {
            Some(d) if d >= 0.0 => {
                reg.n = d as i32;
                Ok(())
            }
            _ => Err(Diag::refuse("save: bad n line")),
        };
    }
    let is_col = k == b"col";
    let is_field = k == b"field";
    let kw = lossy(k);
    if !is_col && !is_field && k != b"pres" && k != b"rel" && k != b"srel" {
        return Err(Diag::refuse(format!("save: unknown line kind '{}'", kw)));
    }
    let Some(name_b) = c.word() else {
        return Err(Diag::refuse(format!("save: {} line without a name", kw)));
    };
    let name = lossy(name_b);
    // char payload is the raw tail — word() consumed exactly one separator, so rest() is
    // the glyph run verbatim, spaces included
    let tail = c.rest();
    let Some(ei) = reg.ents.iter().position(|e| registry::names_eq(&e.name, &name)) else {
        return Err(Diag::refuse(format!("save: unknown entry '{}'", name)));
    };
    let n_now = reg.n;
    let rows = if is_field { reg.lat_w * reg.lat_h } else { n_now };
    let rows_u = rows.max(0) as usize;
    let char_target = match &reg.ents[ei].kind {
        RegEntryKind::Col { ty: ColType::Char, .. } => is_col,
        RegEntryKind::Field { ty: ColType::Char, .. } => is_field,
        _ => false,
    };
    if (is_col || is_field) && char_target {
        if tail.len() != rows_u {
            return Err(Diag::refuse(format!(
                "save: {} {}: {} glyphs for {} cells",
                kw,
                name,
                tail.len(),
                rows
            )));
        }
        // the run must survive the reader it is written for: strip_line trims boundary
        // whitespace and takes a word-boundary '#' as a comment, and an empty tail is no
        // run at all — a post-state that spells any of these has no .reg spelling
        let mut bad = rows_u == 0
            || tail[0] == b' '
            || tail[0] == b'\t'
            || tail[0] == b'#'
            || tail[rows_u - 1] == b' '
            || tail[rows_u - 1] == b'\t';
        if !bad {
            bad = (1..rows_u).any(|j| tail[j] == b'#' && (tail[j - 1] == b' ' || tail[j - 1] == b'\t'));
        }
        if bad {
            return Err(Diag::refuse(format!(
                "save: {} {}: the post-state glyph run has no .reg spelling (empty, boundary whitespace, or a word-boundary '#')",
                kw, name
            )));
        }
        let run = lossy(tail);
        if let RegEntryKind::Col { syms, .. } | RegEntryKind::Field { syms, .. } =
            &mut reg.ents[ei].kind
        {
            *syms = vec![run];
        }
        return Ok(());
    }
    // everything else splits into words
    let mut words: Vec<&[u8]> = Vec::new();
    while let Some(w) = c.word() {
        words.push(w);
    }
    let nw = words.len() as i32;
    if is_col || is_field {
        let kind_ok = match &reg.ents[ei].kind {
            RegEntryKind::Col { .. } => is_col,
            RegEntryKind::Field { .. } => is_field,
            _ => false,
        };
        if !kind_ok {
            return Err(Diag::refuse(format!("save: '{}' is not a {}", name, kw)));
        }
        let is_sym = matches!(
            &reg.ents[ei].kind,
            RegEntryKind::Col { ty: ColType::Sym, .. } | RegEntryKind::Field { ty: ColType::Sym, .. }
        );
        if is_sym {
            if nw != rows {
                return Err(Diag::refuse(format!(
                    "save: {} {}: {} values for {} rows (an empty sym value has no .reg spelling)",
                    kw, name, nw, rows
                )));
            }
            let mut vs = Vec::with_capacity(words.len());
            for &w in &words {
                if w.len() >= ANO_NAMESZ {
                    return Err(Diag::refuse(format!("save: {} {}: sym value too long", kw, name)));
                }
                vs.push(lossy(w));
            }
            if let RegEntryKind::Col { syms, .. } | RegEntryKind::Field { syms, .. } =
                &mut reg.ents[ei].kind
            {
                *syms = vs;
            }
            return Ok(());
        }
        // num/bool scalar (rows values) or pair column (2*rows values, the vec convention)
        if nw != rows && nw != 2 * rows {
            return Err(Diag::refuse(format!(
                "save: {} {}: {} values for {} rows",
                kw, name, nw, rows
            )));
        }
        let mut v = Vec::with_capacity(words.len());
        for &w in &words {
            match save_num(w) {
                Some(x) => v.push(x),
                None => {
                    return Err(Diag::refuse(format!(
                        "save: {} {}: bad number '{}'",
                        kw,
                        name,
                        lossy(w)
                    )));
                }
            }
        }
        if let RegEntryKind::Col { nums, .. } | RegEntryKind::Field { nums, .. } =
            &mut reg.ents[ei].kind
        {
            *nums = v;
        }
        return Ok(());
    }
    if k == b"pres" {
        if !matches!(&reg.ents[ei].kind, RegEntryKind::Col { .. }) {
            return Err(Diag::refuse(format!("save: pres on non-column '{}'", name)));
        }
        if nw != n_now {
            return Err(Diag::refuse(format!(
                "save: pres {}: {} bits for {} rows",
                name, nw, n_now
            )));
        }
        let mut v = Vec::with_capacity(words.len());
        for &w in &words {
            match save_num(w) {
                Some(x) => v.push(x),
                None => {
                    return Err(Diag::refuse(format!("save: pres {}: bad bit '{}'", name, lossy(w))));
                }
            }
        }
        if let RegEntryKind::Col { pres, .. } = &mut reg.ents[ei].kind {
            *pres = Some(v);
        }
        return Ok(());
    }
    if k == b"rel" {
        if !matches!(&reg.ents[ei].kind, RegEntryKind::Rel { .. }) {
            return Err(Diag::refuse(format!("save: '{}' is not a rel", name)));
        }
        if nw != n_now {
            return Err(Diag::refuse(format!(
                "save: rel {}: {} values for {} rows",
                name, nw, n_now
            )));
        }
        let mut v = Vec::with_capacity(words.len());
        for &w in &words {
            match save_num(w) {
                Some(x) => v.push(x),
                None => {
                    return Err(Diag::refuse(format!("save: rel {}: bad index '{}'", name, lossy(w))));
                }
            }
        }
        if let RegEntryKind::Rel { targets, .. } = &mut reg.ents[ei].kind {
            *targets = v;
        }
        return Ok(());
    }
    // srel: fibers |-separated, empty fibers legal (the loader's own shape)
    if !matches!(&reg.ents[ei].kind, RegEntryKind::SRel { .. }) {
        return Err(Diag::refuse(format!("save: '{}' is not an srel", name)));
    }
    let mut fibers: Vec<Vec<f64>> = vec![Vec::new()];
    for &w in &words {
        if w == b"|" {
            fibers.push(Vec::new());
        } else {
            match save_num(w) {
                Some(x) => {
                    if let Some(f) = fibers.last_mut() {
                        f.push(x);
                    }
                }
                None => {
                    return Err(Diag::refuse(format!("save: srel {}: bad id '{}'", name, lossy(w))));
                }
            }
        }
    }
    if let RegEntryKind::SRel { fib, inv_of, .. } = &mut reg.ents[ei].kind {
        *fib = fibers;
        *inv_of = None;
    }
    Ok(())
}

// Inputs: mutable registry, the whole 0x1E capture. Output: () or the first save_line Diag.
// Invariants: split on '\n', first failure aborts; then the mask reconciliation — every
// AliasMask and Bind{Mask} whose length != n resizes (zero-padded on growth, truncated on
// shrink) so the dumped world always reloads.
fn save_patch(reg: &mut Registry, cap: &[u8]) -> Result<(), Diag> {
    // C-string view: processing stops at the first NUL byte in the capture
    let end = cap.iter().position(|&b| b == 0).unwrap_or(cap.len());
    for line in cap[..end].split(|&b| b == b'\n') {
        save_line(reg, line)?;
    }
    let n = reg.n.max(0) as usize;
    for e in &mut reg.ents {
        match &mut e.kind {
            RegEntryKind::AliasMask { mask } if mask.len() != n => mask.resize(n, 0.0),
            RegEntryKind::Bind { kind: BindKind::Mask, vals } if vals.len() != n => {
                vals.resize(n, 0.0)
            }
            _ => {}
        }
    }
    Ok(())
}
