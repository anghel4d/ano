// The write seal, read back out of the emitted text across the active demo corpus.  The seal
// itself lives at the point of lowering; this asserts the property it exists for, independently
// of the emitter's own accounting: nothing reaches a relationship variable except a staged
// binding whose validity was asserted first.  The corpus is the demos the quarantine block in
// todo/TODO.md leaves active — that block is the single authority for the exclusion, shared by
// every harness rather than restated here.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use steel::alias::AliasEnvironment;
use steel::emit::emit_with_aliases;
use steel::lex::lex;
use steel::parse::parse;
use steel::{Directives, Interner, RegEntryKind, Registry};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

// Inputs: the text of todo/TODO.md. Output: every decommissioned demo number, ranges expanded.
// The block is one fenced list of `NNN` items and `NNN–NNN` ranges under its own heading.
fn quarantined(todo: &str) -> BTreeSet<u32> {
    let block = todo
        .split("## Demo evidence quarantine")
        .nth(1)
        .expect("quarantine heading")
        .split("```text")
        .nth(1)
        .expect("quarantine block")
        .split("```")
        .next()
        .expect("quarantine block end");
    let mut out = BTreeSet::new();
    for item in block.split(',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        let mut ends = item
            .split('–')
            .map(|n| n.trim().parse::<u32>().expect("demo number"));
        let low = ends.next().expect("range start");
        let high = ends.next().unwrap_or(low);
        for number in low..=high {
            out.insert(number);
        }
    }
    out
}

// Inputs: a demo path. Output: its three-digit number, or None when it is not a numbered demo.
fn number(path: &Path) -> Option<u32> {
    let stem = path.file_name()?.to_str()?;
    stem.get(..3)
        .filter(|n| n.len() == 3)
        .and_then(|n| n.parse().ok())
}

fn demos(root: &Path, skip: &BTreeSet<u32>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root.join("demos"))
        .expect("demos")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    for dir in dirs {
        let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
            .expect("demo directory")
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "ano"))
            .collect();
        files.sort();
        for file in files {
            let Some(number) = number(&file) else {
                continue;
            };
            if !skip.contains(&number) {
                out.push(file);
            }
        }
    }
    out
}

// Inputs: a demo's source and its directory. Output: the world it declares, the path it was
// loaded from, and its surface flag. Only the two directives that decide what is emitted are
// read; expectations do not change which values reach a relationship. The path rides along so
// the sweep can load the sidecar overlay the driver itself would load beside the registry.
fn world(source: &str, dir: &Path) -> (Registry, Option<PathBuf>, bool) {
    let mut reg = Registry::default();
    let mut reg_path = None;
    let mut ja = false;
    for line in source.lines() {
        let Some(tail) = line.trim_start().strip_prefix("--!") else {
            continue;
        };
        let mut words = tail.split_whitespace();
        match (words.next(), words.next()) {
            (Some("ja"), _) => ja = true,
            (Some("registry"), Some(spec)) => {
                // the driver's rule: a '/'-bearing or .reg-suffixed spec is a literal path
                // against the source file's dir, a bare name resolves as <dir>/<name>.reg
                let literal = spec.contains('/') || (spec.len() > 4 && spec.ends_with(".reg"));
                let path = dir.join(if literal {
                    spec.to_string()
                } else {
                    format!("{}.reg", spec)
                });
                reg = steel::registry::reg_load(path.to_str().expect("path")).expect(spec);
                reg_path = Some(path);
            }
            _ => {}
        }
    }
    (reg, reg_path, ja)
}

// Inputs: a registry. Output: the BQN variable and Ano name of every relationship a write must
// seal — the surfaces relationship::validate_registry seals, an inverse fiber excluded because
// it is derived from the endpoint that was already sealed on write.
fn sealed(reg: &Registry) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (i, entry) in reg.ents.iter().enumerate() {
        let watched = match &entry.kind {
            RegEntryKind::Rel { .. } => true,
            RegEntryKind::SRel { inv_of, .. } => inv_of.is_none(),
            _ => false,
        };
        if !watched {
            continue;
        }
        let legal = !entry.name.is_empty()
            && entry
                .name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
            && entry.name.starts_with(|c: char| c.is_ascii_alphabetic());
        let var = if legal {
            let mut chars = entry.name.chars();
            let first = chars.next().expect("nonempty").to_ascii_lowercase();
            format!("{}{}", first, chars.as_str())
        } else {
            format!("jp{}", i)
        };
        out.push((var, entry.name.clone()));
    }
    out
}

#[test]
fn no_relationship_write_escapes_its_seal() {
    let root = root();
    let todo = std::fs::read_to_string(root.join("todo/TODO.md")).expect("todo/TODO.md");
    let skip = quarantined(&todo);
    let files = demos(&root, &skip);
    assert!(
        files.len() > 40,
        "the active corpus is {} files",
        files.len()
    );

    let mut writes = 0usize;
    let mut refusals = Vec::new();
    for file in &files {
        let source = std::fs::read_to_string(file).expect("demo source");
        let dir = file.parent().expect("demo directory");
        let (reg, reg_path, ja) = world(&source, dir);
        let mut it = Interner::new();
        // the overlay the driver would load: the sidecar beside the registry, or the empty
        // environment when no world (or no sidecar) is declared
        let environment = match &reg_path {
            Some(path) => AliasEnvironment::load(
                steel::alias::sidecar_path(path.to_str().expect("path")),
                &reg,
            )
            .expect("sidecar"),
            None => AliasEnvironment::for_registry(&reg),
        };
        let emitted = lex(source.as_bytes(), ja, &mut it)
            .and_then(|toks| parse(&toks, &mut it))
            .and_then(|prog| {
                let snapshot = environment.snapshot(&reg)?;
                emit_with_aliases(&prog, &reg, &Directives::default(), &it, snapshot)
            });
        let bqn = match emitted {
            Ok(bqn) => bqn,
            Err(diagnostic) => {
                // the emitter's own reconciliation refuses here; that must never be the reason
                assert!(
                    !diagnostic.msg.contains("validity seal")
                        && !diagnostic.msg.contains("reached the world"),
                    "{}: {}",
                    file.display(),
                    diagnostic.msg
                );
                refusals.push(file.clone());
                continue;
            }
        };
        let mut staged: BTreeSet<String> = BTreeSet::new();
        for line in bqn.lines() {
            if let Some(rest) = line.trim_start().strip_prefix("anoRelStage") {
                if let Some((serial, _)) = rest.split_once(" ← ") {
                    staged.insert(format!("anoRelStage{}", serial));
                }
            }
        }
        let watched = sealed(&reg);
        let mut published: BTreeSet<String> = BTreeSet::new();
        for line in bqn.lines() {
            let line = line.trim_start();
            for (var, name) in &watched {
                let Some(rhs) = line.strip_prefix(&format!("{} ↩ ", var)) else {
                    continue;
                };
                assert!(
                    staged.contains(rhs),
                    "{}: relationship '{}' is written outside the seal: {}",
                    file.display(),
                    name,
                    line
                );
                // one staged binding publishes once: a second publication is a second write
                assert!(
                    published.insert(rhs.to_string()),
                    "{}: {} publishes twice",
                    file.display(),
                    rhs
                );
                writes += 1;
            }
        }
        assert_eq!(
            staged.len(),
            published.len(),
            "{}: {} writes staged, {} published",
            file.display(),
            staged.len(),
            published.len()
        );
        // the assertion stands between the staging and the publication, in that order
        for stage in &staged {
            let staging = bqn.find(&format!("{} ← ", stage)).expect("staging");
            let assertion = bqn[staging..].find("\"relationship ").expect("assertion") + staging;
            let publication = bqn[staging..]
                .find(&format!("↩ {}\n", stage))
                .expect("publication")
                + staging;
            assert!(
                staging < assertion && assertion < publication,
                "{}: {}",
                file.display(),
                stage
            );
        }
    }
    // a corpus that writes no relationship would prove nothing
    assert!(writes > 0, "no relationship write in {} demos", files.len());
    assert!(
        refusals.is_empty(),
        "active demos that do not emit: {:?}",
        refusals
    );
}

// The seal rests on the staging variable being the emitter's alone.  anoRelStage<n> is a
// generated family rather than one fixed name, so the loader reserves the stem the way it
// reserves the twelve fixed emitter identifiers: a world column that mangled onto a member
// would sit between the staging and the publication.
#[test]
fn the_staging_family_is_reserved_against_world_columns() {
    let path = std::env::temp_dir().join("ano-relstage-reserved.reg");
    for name in ["anoRelStage0", "AnoRelStage7", "anorelstage12"] {
        std::fs::write(&path, format!("n 2\ncol {} num 1 2\n", name)).expect("fixture");
        let refusal = steel::registry::reg_load(path.to_str().expect("path")).expect_err(name);
        assert_eq!(
            refusal.msg,
            format!(
                "registry line 2: '{}' collides with the emitter's reserved 'anoRelStage<n>' names under the case fold",
                name
            )
        );
    }
    // the stem is what is reserved, not every name that merely starts with `ano`
    std::fs::write(&path, "n 2\ncol anoRelay num 1 2\n").expect("fixture");
    assert!(steel::registry::reg_load(path.to_str().expect("path")).is_ok());
    let _ = std::fs::remove_file(&path);
}
