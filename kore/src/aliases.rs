//! Dynamic-alias administration integrated into Kore.

use steel::alias::{sidecar_path, AliasEnvironment};
use steel::registry::reg_find;
use steel::{num, registry, Registry};

fn usage() -> i32 {
    eprintln!(
        "usage: kore alias file.reg list\n\
         \x20      kore alias file.reg set ^name target\n\
         \x20      kore alias file.reg mask ^name 0 1 …\n\
         \x20      kore alias file.reg resolve ^name resolver-id key=value …\n\
         \x20      kore alias file.reg delete ^name\n\
         \x20      kore alias file.reg clear"
    );
    2
}

fn alias_name(word: &str) -> &str {
    word.strip_prefix('^').unwrap_or(word)
}

fn load(path: &str) -> Result<(Registry, AliasEnvironment), String> {
    let reg = registry::reg_load(path).map_err(|diag| diag.msg)?;
    let aliases = AliasEnvironment::load(sidecar_path(path), &reg).map_err(|diag| diag.msg)?;
    Ok((reg, aliases))
}

fn save(path: &str, aliases: &AliasEnvironment) -> Result<(), String> {
    aliases.save(sidecar_path(path)).map_err(|diag| diag.msg)
}

// Inputs: the frozen registry, the live overlay. Output: the whole listing text — the environment
// version and schema header, then one line per entry, marked `(shadows <entry>)` when the stem
// also resolves in the bare namespace.  Separate from the print so the shadow diagnosis is
// testable without capturing stdout.
fn listing_text(reg: &Registry, aliases: &AliasEnvironment) -> String {
    let mut text = format!("environment v{}\tschema {:016x}\n", aliases.version(), aliases.schema());
    if aliases.is_empty() {
        text.push_str("no dynamic aliases\n");
        return text;
    }
    for (name, target) in aliases.iter() {
        text.push_str(&format!("^{}\t{}", name, target.describe()));
        if let Some(index) = reg_find(reg, name) {
            text.push_str(&format!("\t(shadows {})", reg.ents[index].name));
        }
        text.push('\n');
    }
    text
}

fn list(reg: &Registry, aliases: &AliasEnvironment) {
    print!("{}", listing_text(reg, aliases));
}

// Inputs: the `key=value` operands. Output: the frozen resolver input members, or a refusal —
// an operand without '=' is a host error, never a keyless member.
fn input_pairs(words: &[String]) -> Result<Vec<(String, String)>, String> {
    let mut pairs = Vec::with_capacity(words.len());
    for member in words {
        let Some((key, value)) = member.split_once('=') else {
            return Err(format!("resolver input '{}' is not key=value", member));
        };
        pairs.push((key.to_string(), value.to_string()));
    }
    Ok(pairs)
}

// `clear` alone loads leniently: it is the operator-initiated reset for a sidecar this registry can
// no longer load, so a stale environment stays correctable from the host API.  Never automatic, and
// the bare namespace is untouched.
fn clear(path: &str) -> i32 {
    let reg = match registry::reg_load(path) {
        Ok(reg) => reg,
        Err(diag) => {
            eprintln!("kore alias: {}", diag.msg);
            return 2;
        }
    };
    let (mut aliases, discarded) = AliasEnvironment::load_or_recover(sidecar_path(path), &reg);
    if let Some(reason) = discarded {
        eprintln!("kore alias: discarded a stale environment — {}", reason);
    }
    aliases.clear(&reg);
    if let Err(message) = save(path, &aliases) {
        eprintln!("kore alias: {}", message);
        return 2;
    }
    0
}

pub fn run(args: &[String]) -> i32 {
    if args.len() < 3 {
        return usage();
    }
    let path = &args[1];
    if args[2] == "clear" && args.len() == 3 {
        return clear(path);
    }
    let (reg, mut aliases) = match load(path) {
        Ok(value) => value,
        Err(message) => {
            eprintln!("kore alias: {}", message);
            return 2;
        }
    };

    let result = match args[2].as_str() {
        "list" if args.len() == 3 => {
            list(&reg, &aliases);
            return 0;
        }
        "set" if args.len() == 5 => {
            let name = alias_name(&args[3]);
            aliases.install_binding(&reg, name, &args[4])
        }
        "mask" if args.len() >= 4 => {
            let name = alias_name(&args[3]);
            let mut values = Vec::with_capacity(args.len() - 4);
            for spelling in &args[4..] {
                let Some(value) = num::wnum(spelling) else {
                    eprintln!("kore alias: '{}' is not a finite number", spelling);
                    return 2;
                };
                values.push(value);
            }
            aliases.install_mask(&reg, name, &values)
        }
        "resolve" if args.len() >= 5 => {
            let name = alias_name(&args[3]);
            let pairs = match input_pairs(&args[5..]) {
                Ok(pairs) => pairs,
                Err(message) => {
                    eprintln!("kore alias: {}", message);
                    return 2;
                }
            };
            aliases.install_resolver(&reg, name, &args[4], &pairs)
        }
        "delete" if args.len() == 4 => {
            let name = alias_name(&args[3]);
            if !aliases.delete(name) {
                eprintln!("kore alias: no dynamic alias ^{}", name);
                return 2;
            }
            Ok(())
        }
        _ => return usage(),
    };

    if let Err(diag) = result {
        eprintln!("kore alias: {}", diag.msg);
        return 2;
    }
    if let Err(message) = save(path, &aliases) {
        eprintln!("kore alias: {}", message);
        return 2;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    // Two total number columns over three rows: `gold` shadows the column `Gold` in the bare
    // namespace, `focus` shadows nothing.
    const FIXTURE: &str = "n 3\ncol Gold num 1 2 3\ncol Silver num 4 5 6\n";

    // A private world under the system temp root, unique per test and per process.
    fn world(tag: &str) -> String {
        let dir = std::env::temp_dir().join(format!("ano-kore-alias-{}-{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("world.reg").to_string_lossy().into_owned();
        std::fs::write(&path, FIXTURE).unwrap();
        path
    }

    fn argv(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| word.to_string()).collect()
    }

    fn side(path: &str) -> String {
        sidecar_path(path).to_string_lossy().into_owned()
    }

    fn text(path: &str) -> String {
        let (reg, aliases) = load(path).unwrap();
        listing_text(&reg, &aliases)
    }

    #[test]
    fn alias_cli_set_lists_with_version_header_and_shadow_marker() {
        let path = world("set-list");
        assert_eq!(run(&argv(&["alias", &path, "set", "^gold", "Silver"])), 0);
        assert_eq!(run(&argv(&["alias", &path, "set", "^focus", "Gold"])), 0);
        let listing = text(&path);
        assert!(listing.starts_with("environment v2\tschema "), "{}", listing);
        assert!(listing.contains("^gold\tSilver (number)\t(shadows Gold)\n"), "{}", listing);
        assert!(listing.contains("^focus\tGold (number)\n"), "{}", listing);
        assert_eq!(run(&argv(&["alias", &path, "list"])), 0);
    }

    #[test]
    fn alias_cli_mask_and_resolver_targets_install() {
        let path = world("mask-resolve");
        assert_eq!(run(&argv(&["alias", &path, "mask", "^hot", "1", "0", "1"])), 0);
        assert_eq!(
            run(&argv(&["alias", &path, "resolve", "^here", "input.entity", "entity=1"])),
            0
        );
        let listing = text(&path);
        assert!(listing.contains("^hot\tmask [1 0 1]\n"), "{}", listing);
        assert!(
            listing.contains("^here\tresolver input.entity (entity, service 1, input v2)\n"),
            "{}",
            listing
        );
        // an operand without '=' is a host error, not a keyless member
        assert_eq!(run(&argv(&["alias", &path, "resolve", "^x", "input.entity", "entity1"])), 2);
        // and the out-of-domain row refuses at install
        assert_eq!(
            run(&argv(&["alias", &path, "resolve", "^x", "input.entity", "entity=9"])),
            2
        );
    }

    #[test]
    fn alias_cli_rebind_moves_the_target_and_the_version() {
        let path = world("rebind");
        assert_eq!(run(&argv(&["alias", &path, "set", "^focus", "Gold"])), 0);
        assert!(text(&path).contains("^focus\tGold (number)"));
        assert_eq!(run(&argv(&["alias", &path, "set", "^focus", "Silver"])), 0);
        let listing = text(&path);
        assert!(listing.starts_with("environment v2\t"), "{}", listing);
        assert!(listing.contains("^focus\tSilver (number)"), "{}", listing);
        assert!(!listing.contains("Gold (number)"), "{}", listing);
    }

    // A refused transition is not a transition: the sidecar stays byte-identical because `save`
    // only runs on Ok.  Structurally guaranteed today, pinned here.
    #[test]
    fn alias_cli_failed_transitions_leave_the_sidecar_identical() {
        let path = world("failed");
        assert_eq!(run(&argv(&["alias", &path, "set", "^focus", "Gold"])), 0);
        let before = std::fs::read(side(&path)).unwrap();

        assert_eq!(run(&argv(&["alias", &path, "set", "^focus", "Nope"])), 2);
        assert_eq!(run(&argv(&["alias", &path, "mask", "^hot", "1", "0"])), 2);
        assert_eq!(run(&argv(&["alias", &path, "mask", "^hot", "1", "2", "0"])), 2);
        assert_eq!(run(&argv(&["alias", &path, "resolve", "^x", "input.nope", "entity=0"])), 2);
        assert_eq!(run(&argv(&["alias", &path, "delete", "^absent"])), 2);

        assert_eq!(std::fs::read(side(&path)).unwrap(), before);
        assert!(text(&path).contains("^focus\tGold (number)"));
    }

    #[test]
    fn alias_cli_clear_empties_a_healthy_environment() {
        let path = world("clear");
        assert_eq!(run(&argv(&["alias", &path, "set", "^focus", "Gold"])), 0);
        assert_eq!(run(&argv(&["alias", &path, "clear"])), 0);
        let listing = text(&path);
        assert!(listing.starts_with("environment v2\t"), "{}", listing);
        assert!(listing.contains("no dynamic aliases"), "{}", listing);
    }

    // A sidecar this registry can no longer load bricks every strict verb; `clear` is the one
    // recovery verb, and the version counter keeps moving forward across the reset (ruling e).
    #[test]
    fn alias_cli_clear_recovers_a_stale_sidecar() {
        let path = world("recover");
        assert_eq!(run(&argv(&["alias", &path, "set", "^focus", "Gold"])), 0);
        std::fs::write(side(&path), "garbage\n").unwrap();

        assert_eq!(run(&argv(&["alias", &path, "set", "^focus", "Silver"])), 2);
        assert_eq!(run(&argv(&["alias", &path, "list"])), 2);

        assert_eq!(run(&argv(&["alias", &path, "clear"])), 0);
        assert_eq!(run(&argv(&["alias", &path, "list"])), 0);
        let listing = text(&path);
        assert!(listing.starts_with("environment v2\t"), "{}", listing);
        assert!(listing.contains("no dynamic aliases"), "{}", listing);
        assert_eq!(run(&argv(&["alias", &path, "set", "^focus", "Silver"])), 0);
    }
}
