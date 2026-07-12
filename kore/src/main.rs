// main.rs — PORTER 5: the crate root and CLI entry forms, the interactive event loop, anoc
// discovery, the child invocation, and the 0x1D/0x1F stdout demux (0x1E never reaches kore:
// --save advances the world as a file). Contract maps: kmaps/kore-world.md ("main dispatch",
// "the child process layer", "the stdout demux") and kore-term.md §8; C source: kore/kore.c
// l.929-975 (find_anoc/run_child), l.1229-1298 (cap_split), l.3884-3938 (main).

mod app;
mod sys;
mod tables;
mod term;
mod text;
mod ui;
mod world;

use app::{App, Focus, Mode, QGroup, QRec};
use term::Term;

fn main() {
    std::process::exit(kore_main());
}

// Mode selection precedes the terminal (kore.c main l.3884): `--check <files>` ORs
// world::check_reg codes; `--edit` with exactly 5 operands runs world::edit_reg; any other
// `-` flag prints usage (verbatim, three lines) on stderr, exit 2. Bare -> MODE_RAIL
// (walk_demos + sort by text::cmp_demo, verdict `%d demos — enter opens, r resets the
// world, n steps it, : prompts`); `.reg` -> MODE_REG (load or `kore: %s\n` exit 2,
// undo_scan, session_rehydrate, focus prompt); else MODE_DEMO (access R_OK or `kore:
// cannot read %s\n` exit 2, open_demo, focus code). Then sys::install_signals,
// Term::enter (`kore: not a terminal\n` exit 2), and the loop:
// while !quit { resized -> size; ui::draw; ev_read; ui::handle } — draw runs
// unconditionally every pass; term_leave on the way out.
fn kore_main() -> i32 {
    let args: Vec<String> = std::env::args_os().map(|a| a.to_string_lossy().into_owned()).collect();
    let mut app = App::new();
    if args.len() >= 3 && args[1] == "--check" {
        let mut rc = 0;
        for p in &args[2..] {
            rc |= world::check_reg(&mut app, p);
        }
        return rc;
    }
    if args.len() == 7 && args[1] == "--edit" {
        return world::edit_reg(&mut app, &args[2..]);
    }
    let arg = args.get(1);
    if let Some(a) = arg {
        if a.starts_with('-') {
            eprint!(
                "usage: kore [file.reg | file.ano]                    (bare: the demos rail)\n\
                 \x20      kore --check file.reg …                       (headless load+render report)\n\
                 \x20      kore --edit file.reg seg row col value        (headless cell splice)\n"
            );
            return 2;
        }
    }
    match arg {
        None => {
            app.mode = Mode::Rail;
            walk_demos(&mut app, "demos");
            app.demo_list.sort_by(|a, b| text::cmp_demo(a, b));
            app.focus = Focus::Rail;
            let n = app.demo_list.len();
            app.say(&format!("{} demos — enter opens, r resets the world, n steps it, : prompts", n));
        }
        Some(a) if a.len() > 4 && a.ends_with(".reg") => {
            app.mode = Mode::Reg;
            match world::world_load(a) {
                Ok(w) => app.world = w,
                Err(e) => {
                    eprintln!("kore: {}", e);
                    return 2;
                }
            }
            world::undo_scan(&mut app);
            world::session_rehydrate(&mut app);
            app.focus = Focus::Prompt;
            app.say("bare world — the prompt is the program");
        }
        Some(a) => {
            app.mode = Mode::Demo;
            if !sys::access_r(a) {
                eprintln!("kore: cannot read {}", a);
                return 2;
            }
            world::open_demo(&mut app, a);
            app.focus = Focus::Code;
        }
    }
    sys::install_signals();
    let mut term = Term::new();
    if !term.enter() {
        eprintln!("kore: not a terminal");
        return 2;
    }
    while !app.quit {
        if sys::resized_take() {
            term.size();
        }
        ui::draw(&mut app, &mut term);
        let e = term::ev_read();
        ui::handle(&mut app, &mut term, &e);
    }
    sys::term_leave();
    0
}

// Recursive walk from `demos` relative to CWD, dot-entries skipped, `.ano` suffix, cap
// KMAXDEMO; sorted by text::cmp_demo. kore.c walk_demos l.980.
pub fn walk_demos(app: &mut App, dir: &str) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for de in rd.flatten() {
        let name = de.file_name();
        if name.as_encoded_bytes().first() == Some(&b'.') {
            continue;
        }
        let p = format!("{}/{}", dir, name.to_string_lossy());
        // C skips a path snprintf would truncate at PATH_MAX.
        if p.len() >= 4096 {
            continue;
        }
        let Ok(st) = std::fs::metadata(&p) else { continue };
        if st.is_dir() {
            walk_demos(app, &p);
        } else if p.len() > 4 && p.ends_with(".ano") && app.demo_list.len() < crate::app::KMAXDEMO {
            app.demo_list.push(p);
        }
    }
}

// Cached: $ANOC when executable; else anoc beside kore's own binary via /proc/self/exe
// (target/release under the workspace — the Steel sibling always wins over a stale C
// build); else <exedir>/../src/anoc; else src/anoc under CWD; else "anoc" (PATH via
// execvp). kore.c find_anoc l.929, the sibling rung added for the cargo layout.
pub fn find_anoc(app: &mut App) -> String {
    if let Some(p) = &app.anoc_path {
        return p.clone();
    }
    let found = find_anoc_probe();
    app.anoc_path = Some(found.clone());
    found
}

fn find_anoc_probe() -> String {
    if let Some(env) = std::env::var_os("ANOC") {
        let env = env.to_string_lossy().into_owned();
        if !env.is_empty() && sys::access_x(&env) {
            return env;
        }
    }
    if let Ok(exe) = std::fs::read_link("/proc/self/exe") {
        let exe = exe.to_string_lossy().into_owned();
        if let Some(sl) = exe.rfind('/') {
            for rel in ["/anoc", "/../src/anoc"] {
                let cand = format!("{}{}", &exe[..sl], rel);
                if sys::access_x(&cand) {
                    return cand;
                }
            }
        }
    }
    if sys::access_x("src/anoc") {
        return "src/anoc".to_string();
    }
    "anoc".to_string()
}

// One anoc run, stdout+stderr merged (sys::run_capture): (capture, exit code).
pub fn run_anoc(argv: &[&str]) -> (Vec<u8>, i32) {
    let mut cap = Vec::new();
    let code = sys::run_capture(argv, &mut cap);
    (cap, code)
}

// Trailing space/tab/\r stripped copy — the 0x1D tag label. kore.c label_dup l.1211.
pub fn label_dup(s: &[u8]) -> Vec<u8> {
    let mut d = s.to_vec();
    while matches!(d.last(), Some(b' ' | b'\t' | b'\r')) {
        d.pop();
    }
    d
}

// buf_take_rstrip (kore.c l.1201): take the buffer with trailing \n \r space tab stripped.
fn take_rstrip(b: &mut Vec<u8>) -> Vec<u8> {
    while matches!(b.last(), Some(b'\n' | b'\r' | b' ' | b'\t')) {
        b.pop();
    }
    std::mem::take(b)
}

// sscanf %d over a byte cursor: whitespace skip, optional sign, >=1 digit; i32-clamped
// (an out-of-range line number falls out of run_line's bounds anyway).
fn scan_i32(s: &[u8], i: &mut usize) -> Option<i32> {
    while matches!(s.get(*i), Some(b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')) {
        *i += 1;
    }
    let mut j = *i;
    let neg = match s.get(j) {
        Some(b'-') => {
            j += 1;
            true
        }
        Some(b'+') => {
            j += 1;
            false
        }
        _ => false,
    };
    let d0 = j;
    let mut v: i64 = 0;
    while let Some(&d) = s.get(j) {
        if !d.is_ascii_digit() {
            break;
        }
        v = (v * 10 + (d - b'0') as i64).min(i32::MAX as i64 + 1);
        j += 1;
    }
    if j == d0 {
        return None;
    }
    *i = j;
    let v = if neg { -v } else { v };
    Some(v.clamp(i32::MIN as i64, i32::MAX as i64) as i32)
}

// sscanf(tag, "q%d@%d") == 2 -> Some(ln): literal q, %d, literal @, %d.
fn scan_q_tag(tag: &[u8]) -> Option<i32> {
    if tag.first() != Some(&b'q') {
        return None;
    }
    let mut i = 1;
    scan_i32(tag, &mut i)?;
    if tag.get(i) != Some(&b'@') {
        return None;
    }
    i += 1;
    scan_i32(tag, &mut i)
}

// The one demux (kore.c cap_split l.1229). Nonzero exit: the whole capture to history,
// a leading 0x1F stripped per non-empty line, no group. Exit 0: walk lines — 0x1F to
// history sentinel-stripped EVEN mid-record; 0x1D closes any open value (rstripped) and
// opens a record (tag <= 63 bytes; `q%d@%d` resolves the label through app.run_line, else
// the raw tag); untagged lines accumulate into the open value, pre-tag lines flow to
// history. Close the last value; a record-less group is dropped (never stacks seams);
// outputs_bound; outputs_scroll = 0.
pub fn cap_split(app: &mut App, cap: &[u8], code: i32, step: i32) {
    let s = cap;
    let n = s.len();
    if code != 0 {
        // the failure path logs the capture whole; trace sentinels still strip so the
        // diagnostic lines read clean beside the compiler error
        let mut raw: Vec<u8> = Vec::new();
        let mut at = 0;
        while at < n {
            let mut e = at;
            while e < n && s[e] != b'\n' {
                e += 1;
            }
            let st = at + if e > at && s[at] == 0x1F { 1 } else { 0 };
            raw.extend_from_slice(&s[st..e]);
            raw.push(b'\n');
            at = e + 1;
        }
        if !raw.is_empty() {
            app.log(&raw);
        }
        return;
    }
    let mut recs: Vec<QRec> = Vec::new();
    let mut hist: Vec<u8> = Vec::new();
    let mut val: Vec<u8> = Vec::new();
    let mut open = false;
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j < n && s[j] != b'\n' {
            j += 1;
        }
        let ll = j - i;
        if ll > 0 && s[i] == 0x1F {
            // trace diagnostics ride to history whatever record is open
            hist.extend_from_slice(&s[i + 1..j]);
            hist.push(b'\n');
        } else if ll > 0 && s[i] == 0x1D {
            if open {
                recs.last_mut().unwrap().value = take_rstrip(&mut val);
            }
            // snprintf "%.*s" into char[64]: 63-byte cap, stops at an embedded NUL
            let mut tag = &s[i + 1..i + 1 + (ll - 1).min(63)];
            if let Some(z) = tag.iter().position(|&b| b == 0) {
                tag = &tag[..z];
            }
            let lbl = scan_q_tag(tag).and_then(|ln| app.run_line(ln));
            let label = label_dup(lbl.unwrap_or(tag));
            recs.push(QRec { label, value: Vec::new() });
            open = true;
        } else if open {
            val.extend_from_slice(&s[i..j]);
            val.push(b'\n');
        } else {
            hist.extend_from_slice(&s[i..j]);
            hist.push(b'\n');
        }
        i = j + 1;
    }
    if open {
        recs.last_mut().unwrap().value = take_rstrip(&mut val);
    }
    if !hist.is_empty() {
        app.log(&hist);
    }
    // a record-less run leaves no group: outputs_bound counts records alone, so empty
    // groups (mutation-only submissions) would otherwise stack seams without bound
    if recs.is_empty() {
        return;
    }
    app.qgroups.push(QGroup { step, recs });
    app.outputs_bound();
    app.outputs_scroll = 0;
}

#[cfg(test)]
mod demux_tests {
    use super::*;

    #[test]
    fn nonzero_exit_whole_capture_sentinel_stripped() {
        let mut a = App::new();
        cap_split(&mut a, b"\x1Ftrace line\nerror: boom\n", 3, 5);
        assert_eq!(a.out_log, b"trace line\nerror: boom\n");
        assert!(a.qgroups.is_empty());
        // empty line: 0x1F NOT stripped only when line empty; lone \x1F line is non-empty
        let mut b = App::new();
        cap_split(&mut b, b"\n\x1F\n", 1, 0);
        assert_eq!(b.out_log, b"\n\n");
    }

    #[test]
    fn exit0_records_resolve_tags() {
        let mut a = App::new();
        a.run_lines_set(b"--! registry w\n  #/ Planted\nsecond\n");
        cap_split(&mut a, b"pre-tag\n\x1Dq1@2\n42  \n\x1Ftrace mid\nmore\n\x1Dweird\nv\n", 0, 7);
        assert_eq!(a.out_log, b"pre-tag\ntrace mid\n");
        assert_eq!(a.qgroups.len(), 1);
        let g = &a.qgroups[0];
        assert_eq!(g.step, 7);
        assert_eq!(g.recs.len(), 2);
        assert_eq!(g.recs[0].label, b"#/ Planted");
        assert_eq!(g.recs[0].value, b"42  \nmore"); // rstrip is tail-only; interior spaces stay
        assert_eq!(g.recs[1].label, b"weird");
        assert_eq!(g.recs[1].value, b"v");
    }

    #[test]
    fn recordless_group_dropped_and_bad_tag_forms() {
        let mut a = App::new();
        cap_split(&mut a, b"just output\n", 0, 3);
        assert!(a.qgroups.is_empty());
        assert_eq!(a.out_log, b"just output\n");
        // q tag with out-of-range line -> raw tag as label
        let mut b = App::new();
        b.run_lines_set(b"one\n");
        cap_split(&mut b, b"\x1Dq1@9\nx\n", 0, 1);
        assert_eq!(b.qgroups[0].recs[0].label, b"q1@9");
        // sscanf tolerance: q +2@1 with space matches (%d skips ws)
        let mut c = App::new();
        c.run_lines_set(b"  \tstmt\n");
        cap_split(&mut c, b"\x1Dq 1@ 1\nv\n", 0, 1);
        assert_eq!(c.qgroups[0].recs[0].label, b"stmt"); // ltrimmed by run_line
    }

    #[test]
    fn label_dup_and_tag_cap() {
        assert_eq!(label_dup(b"abc \t\r"), b"abc");
        let mut a = App::new();
        let mut cap = vec![0x1D];
        cap.extend_from_slice(&[b'x'; 100]);
        cap.extend_from_slice(b"\nv\n");
        cap_split(&mut a, &cap, 0, 1);
        assert_eq!(a.qgroups[0].recs[0].label.len(), 63);
    }
}
