// Crate root and CLI: event loop, Steel discovery, child invocation, and 0x1D/0x1F stdout
// demux. 0x1E does not reach kore because --save advances the world file.

mod aliases;
mod app;
mod registry_tx;
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

// Mode selection precedes the terminal (kore.c main): `--check <files>` ORs
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
    let args: Vec<String> = std::env::args_os()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    if args.get(1).map(String::as_str) == Some("alias") {
        return aliases::run(&args[1..]);
    }
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
                 \x20      kore --edit file.reg seg row col value        (headless cell splice)\n\
                 \x20      kore alias file.reg list|set|mask|resolve|delete|clear (dynamic-alias admin)\n"
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
            app.say(&format!(
                "{} demos — enter opens, r resets the world, n steps it, : prompts",
                n
            ));
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
// KMAXDEMO; sorted by text::cmp_demo. kore.c walk_demos.
pub fn walk_demos(app: &mut App, dir: &str) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
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
        let Ok(st) = std::fs::metadata(&p) else {
            continue;
        };
        if st.is_dir() {
            walk_demos(app, &p);
        } else if p.len() > 4 && p.ends_with(".ano") && app.demo_list.len() < crate::app::KMAXDEMO {
            app.demo_list.push(p);
        }
    }
}

// Cached: $STEEL when executable; else steel beside kore's own binary via current_exe
// (the cargo sibling in target/release or target/debug); else target/release/steel under
// CWD; else "steel" from PATH. Kore invokes only Steel; no retired compiler participates
// in discovery, execution, or validation.
pub fn find_steel(app: &mut App) -> String {
    if let Some(p) = &app.steel_path {
        return p.clone();
    }
    let found = find_steel_probe();
    app.steel_path = Some(found.clone());
    found
}

fn find_steel_probe() -> String {
    if let Some(env) = std::env::var_os("STEEL") {
        let env = env.to_string_lossy().into_owned();
        if !env.is_empty() && sys::access_x(&env) {
            return env;
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        let exe = exe.to_string_lossy().into_owned();
        if let Some(sl) = exe.rfind('/') {
            let cand = format!("{}/steel", &exe[..sl]);
            if sys::access_x(&cand) {
                return cand;
            }
        }
    }
    if sys::access_x("target/release/steel") {
        return "target/release/steel".to_string();
    }
    "steel".to_string()
}

// One steel run, stdout+stderr merged (sys::run_capture): (capture, exit code).
pub fn run_steel(argv: &[&str]) -> (Vec<u8>, i32) {
    let mut owned: Vec<String> = argv.iter().map(|arg| (*arg).to_string()).collect();
    if !owned.iter().any(|arg| arg == "--aliases") {
        if let Some(index) = owned.iter().position(|arg| arg == "--registry") {
            if let Some(registry) = owned.get(index + 1).cloned() {
                owned.push("--aliases".to_string());
                owned.push(
                    steel::alias::sidecar_path(&registry)
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
    }
    let borrowed: Vec<&str> = owned.iter().map(String::as_str).collect();
    let mut cap = Vec::new();
    let code = sys::run_capture(&borrowed, &mut cap);
    (cap, code)
}

// Trailing space/tab/\r stripped copy — the 0x1D tag label. kore.c label_dup.
pub fn label_dup(s: &[u8]) -> Vec<u8> {
    let mut d = s.to_vec();
    while matches!(d.last(), Some(b' ' | b'\t' | b'\r')) {
        d.pop();
    }
    d
}

// buf_take_rstrip (kore.c): take the buffer with trailing \n \r space tab stripped.
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

// The one demux (kore.c cap_split). Nonzero exit: the whole capture to history,
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
            recs.push(QRec {
                label,
                value: Vec::new(),
            });
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
        cap_split(
            &mut a,
            b"pre-tag\n\x1Dq1@2\n42  \n\x1Ftrace mid\nmore\n\x1Dweird\nv\n",
            0,
            7,
        );
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

    // A12 through the demux: an identityless fold over nothing emits neither its 0x1D tag nor
    // its value, so a suppressed query contributes no QRec and Kore shows no OUTPUTS row. Steel
    // stages tag and value inside ONE conditional precisely so this stream is what arrives.
    #[test]
    fn suppressed_labels_yield_no_output_rows() {
        // a run whose every query is suppressed: no tag line, so no group at all
        let mut a = App::new();
        a.run_lines_set(b"max/ Gold @ Burning\nmin/ Gold @ Burning\n");
        cap_split(&mut a, b"", 0, 4);
        assert!(a.qgroups.is_empty());
        // history still flows; a record-less run leaves no seam behind it
        let mut b = App::new();
        b.run_lines_set(b"max/ Gold @ Burning\n");
        cap_split(&mut b, b"\x1Fnote\n", 0, 4);
        assert!(b.qgroups.is_empty());
        assert_eq!(b.out_log, b"note\n");
        // and one suppressed query beside one live one yields exactly one row, the live one
        let mut c = App::new();
        c.run_lines_set(b"max/ Gold @ Burning\n+/ Gold\n");
        cap_split(&mut c, b"\x1Dq2@2\n10\n", 0, 4);
        assert_eq!(c.qgroups.len(), 1);
        assert_eq!(c.qgroups[0].recs.len(), 1);
        assert_eq!(c.qgroups[0].recs[0].label, b"+/ Gold");
        assert_eq!(c.qgroups[0].recs[0].value, b"10");
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

// The Kore half of todo/03 item 9, observed end to end: a real Steel invocation over a real
// BQN backend, demuxed by Kore's own cap_split into the OUTPUTS data model draw_outputs
// renders.  The handcrafted-capture pins above prove the demux law; these prove the stream
// Steel actually sends is the stream that law was written for.
#[cfg(test)]
mod outputs_end_to_end {
    use super::*;

    // The workspace steel binary beside this test's own target directory.  $STEEL-style PATH
    // discovery is deliberately bypassed: a test must never fall back to a stale release build.
    fn steel_bin() -> String {
        let mut p = std::env::current_exe().expect("current_exe");
        p.pop(); // deps
        p.pop(); // debug
        p.push("steel");
        assert!(
            p.exists(),
            "steel binary missing at {} — build the workspace first (cargo test/build --workspace)",
            p.display()
        );
        p.to_string_lossy().into_owned()
    }

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("kore-outputs-{}-{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    // An identityless fold over an empty scope reaches Kore as a suppressed record: no 0x1D
    // tag, no value, no OUTPUTS row — while the identity folds and a live sibling land as
    // ordinary rows.  No fabricated scalar exists anywhere in the pane's data model.
    #[test]
    fn empty_identityless_fold_shows_no_outputs_row() {
        let dir = scratch("empty");
        let reg = dir.join("world.reg");
        std::fs::write(
            &reg,
            "n 3\ncol gold num 1 2 3\ncol burning bool 0 0 0\ncol alive bool 1 1 1\n",
        )
        .unwrap();
        let prog = format!(
            "--! registry {}\nmax/ Gold @ Burning\n+/ Gold @ Burning\n#/ Burning\navg/ Gold @ Burning\n+/ Gold @ Alive\n",
            reg.display()
        );
        let ano = dir.join("world.ano");
        std::fs::write(&ano, &prog).unwrap();

        let steel = steel_bin();
        let argv = [
            steel.as_str(),
            "--run",
            "--label",
            ano.to_str().expect("utf8 path"),
        ];
        let (cap, code) = crate::run_steel(&argv);
        assert_eq!(
            code,
            0,
            "steel --run failed:\n{}",
            String::from_utf8_lossy(&cap)
        );

        let mut app = App::new();
        app.run_lines_set(prog.as_bytes());
        cap_split(&mut app, &cap, code, 1);

        // one group; the two identityless folds contribute no record at all
        assert_eq!(app.qgroups.len(), 1, "{}", String::from_utf8_lossy(&cap));
        let recs = &app.qgroups[0].recs;
        let labels: Vec<&[u8]> = recs.iter().map(|r| r.label.as_slice()).collect();
        assert_eq!(
            labels,
            vec![
                b"+/ Gold @ Burning".as_slice(),
                b"#/ Burning".as_slice(),
                b"+/ Gold @ Alive".as_slice(),
            ],
            "suppressed queries left rows behind: {}",
            String::from_utf8_lossy(&cap)
        );
        assert_eq!(recs[0].value, b"0");
        assert_eq!(recs[1].value, b"0");
        assert_eq!(recs[2].value, b"6");
    }

    // Every query suppressed: the run leaves no group and no seam — the OUTPUTS pane keeps
    // showing its placeholder, not a stack of empty steps.
    #[test]
    fn all_suppressed_queries_leave_no_group() {
        let dir = scratch("allempty");
        let reg = dir.join("world.reg");
        std::fs::write(&reg, "n 2\ncol gold num 5 7\ncol burning bool 0 0\n").unwrap();
        let prog = format!(
            "--! registry {}\nmax/ Gold @ Burning\nmin/ Gold @ Burning\navg/ Gold @ Burning\n",
            reg.display()
        );
        let ano = dir.join("world.ano");
        std::fs::write(&ano, &prog).unwrap();

        let steel = steel_bin();
        let argv = [
            steel.as_str(),
            "--run",
            "--label",
            ano.to_str().expect("utf8 path"),
        ];
        let (cap, code) = crate::run_steel(&argv);
        assert_eq!(
            code,
            0,
            "steel --run failed:\n{}",
            String::from_utf8_lossy(&cap)
        );

        let mut app = App::new();
        app.run_lines_set(prog.as_bytes());
        cap_split(&mut app, &cap, code, 1);
        assert!(app.qgroups.is_empty(), "{}", String::from_utf8_lossy(&cap));
    }

    // The 02 gate's Kore half for the rewritten alias demos: the demo file runs through the
    // exact plumbing a Kore session uses — run_steel, the sidecar riding the registry
    // directive, cap_split — and its overlay-dependent observations land as OUTPUTS rows.
    #[test]
    fn rewritten_alias_demo_passes_through_kore() {
        let demo = format!(
            "{}/../demos/1-selection/005-alias-overlay.ano",
            env!("CARGO_MANIFEST_DIR")
        );
        let text = std::fs::read(&demo).expect("demo 005 exists");

        let steel = steel_bin();
        let argv = [steel.as_str(), "--run", "--label", demo.as_str()];
        let (cap, code) = crate::run_steel(&argv);
        assert_eq!(
            code,
            0,
            "demo 005 failed under kore's runner:\n{}",
            String::from_utf8_lossy(&cap)
        );

        let mut app = App::new();
        app.run_lines_set(&text);
        cap_split(&mut app, &cap, code, 1);
        assert_eq!(app.qgroups.len(), 1, "{}", String::from_utf8_lossy(&cap));
        let recs = &app.qgroups[0].recs;
        // the overlay observations: sigiled and bare focus diverge in one world
        assert_eq!(recs[0].label, b"#/ ^focus");
        assert_eq!(recs[0].value, b"2");
        assert_eq!(recs[1].label, b"#/ focus");
        assert_eq!(recs[1].value, b"2");
        assert_eq!(recs[2].label, b"#/ (^focus & focus)");
        assert_eq!(recs[2].value, b"0");
    }
}
