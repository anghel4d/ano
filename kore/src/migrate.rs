//! Durable, rollback-safe publication of a Steel registry migration bundle.

use std::io::Write;
use std::path::{Path, PathBuf};

use steel::alias::{AliasEnvironment, sidecar_path};
use steel::migration::{MigrationOutcome, MigrationPlan, SchemaManifest, manifest_path, migrate};

const JOURNAL_MAGIC: &str = "ano-migration-journal-v1";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Preparing,
    Prepared,
    Committed,
    RolledBack,
}

impl Phase {
    fn word(self) -> &'static str {
        match self {
            Phase::Preparing => "preparing",
            Phase::Prepared => "prepared",
            Phase::Committed => "committed",
            Phase::RolledBack => "rolled-back",
        }
    }

    fn parse(word: &str) -> Option<Self> {
        Some(match word {
            "preparing" => Phase::Preparing,
            "prepared" => Phase::Prepared,
            "committed" => Phase::Committed,
            "rolled-back" => Phase::RolledBack,
            _ => return None,
        })
    }
}

struct Transaction {
    live: String,
    nonce: String,
    existed: u8,
}

impl Transaction {
    fn targets(&self) -> [PathBuf; 4] {
        targets(&self.live)
    }

    fn staged(&self) -> [PathBuf; 4] {
        self.targets()
            .map(|path| artifact_path(&path, "migration-stage", &self.nonce))
    }

    fn backups(&self) -> [PathBuf; 4] {
        self.targets()
            .map(|path| artifact_path(&path, "migration-backup", &self.nonce))
    }
}

fn targets(live: &str) -> [PathBuf; 4] {
    [
        PathBuf::from(live),
        sidecar_path(live),
        manifest_path(live),
        PathBuf::from(format!("{}.migrations", live)),
    ]
}

fn journal_path(live: &str) -> PathBuf {
    PathBuf::from(format!("{}.migration-journal", live))
}

fn artifact_path(target: &Path, kind: &str, nonce: &str) -> PathBuf {
    PathBuf::from(format!("{}.{}.{}", target.to_string_lossy(), kind, nonce))
}

fn nonce(live: &str) -> String {
    let mut serial = 0u64;
    loop {
        let nonce = format!("{}.{}", std::process::id(), serial);
        let transaction = Transaction {
            live: live.to_string(),
            nonce: nonce.clone(),
            existed: 0,
        };
        if transaction
            .staged()
            .iter()
            .chain(transaction.backups().iter())
            .all(|path| !path.exists())
        {
            return nonce;
        }
        serial = serial.wrapping_add(1);
    }
}

fn sync_parent(path: &Path) {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        && let Ok(directory) = std::fs::File::open(parent)
    {
        let _ = directory.sync_all();
    }
}

fn write_new(path: &Path, data: &[u8]) -> Result<(), String> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("cannot stage {}: {}", path.display(), error))?;
    file.write_all(data)
        .map_err(|error| format!("cannot stage {}: {}", path.display(), error))?;
    file.sync_all()
        .map_err(|error| format!("cannot sync {}: {}", path.display(), error))
}

fn replace_file(path: &Path, data: &[u8]) -> Result<(), String> {
    let staged = PathBuf::from(format!(
        "{}.next.{}",
        path.to_string_lossy(),
        std::process::id()
    ));
    let _ = std::fs::remove_file(&staged);
    write_new(&staged, data)?;
    std::fs::rename(&staged, path).map_err(|error| {
        let _ = std::fs::remove_file(&staged);
        format!("cannot publish {}: {}", path.display(), error)
    })?;
    sync_parent(path);
    Ok(())
}

fn write_journal(transaction: &Transaction, phase: Phase) -> Result<(), String> {
    let text = format!(
        "{}\t{}\t{}\t{:02x}\n",
        JOURNAL_MAGIC,
        phase.word(),
        transaction.nonce,
        transaction.existed
    );
    replace_file(&journal_path(&transaction.live), text.as_bytes())
}

fn read_journal(live: &str) -> Result<Option<(Transaction, Phase)>, String> {
    let path = journal_path(live);
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path).map_err(|error| {
        format!(
            "cannot read migration journal {}: {}",
            path.display(),
            error
        )
    })?;
    let fields: Vec<&str> = text.trim_end().split('\t').collect();
    if fields.len() != 4 || fields[0] != JOURNAL_MAGIC {
        return Err(format!("migration journal {} is malformed", path.display()));
    }
    let phase = Phase::parse(fields[1])
        .ok_or_else(|| format!("migration journal {} has a bad phase", path.display()))?;
    if fields[2].is_empty()
        || !fields[2]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.')
    {
        return Err(format!(
            "migration journal {} has a bad nonce",
            path.display()
        ));
    }
    let existed = u8::from_str_radix(fields[3], 16)
        .map_err(|_| format!("migration journal {} has a bad target mask", path.display()))?;
    if existed & !0x0f != 0 {
        return Err(format!(
            "migration journal {} has a bad target mask",
            path.display()
        ));
    }
    Ok(Some((
        Transaction {
            live: live.to_string(),
            nonce: fields[2].to_string(),
            existed,
        },
        phase,
    )))
}

fn cleanup(transaction: &Transaction, journal: bool) {
    for path in transaction
        .staged()
        .iter()
        .chain(transaction.backups().iter())
    {
        let _ = std::fs::remove_file(path);
    }
    if journal {
        let _ = std::fs::remove_file(journal_path(&transaction.live));
        sync_parent(Path::new(&transaction.live));
    }
}

fn rollback_with_failure(
    transaction: &Transaction,
    fail_after: Option<usize>,
) -> Result<(), String> {
    let targets = transaction.targets();
    let backups = transaction.backups();
    let mut restored = 0usize;
    for index in (0..targets.len()).rev() {
        if transaction.existed & (1 << index) != 0 {
            if !backups[index].exists() {
                return Err(format!(
                    "migration recovery is missing backup {}",
                    backups[index].display()
                ));
            }
            let bytes = std::fs::read(&backups[index]).map_err(|error| {
                format!(
                    "cannot read recovery backup {}: {}",
                    backups[index].display(),
                    error
                )
            })?;
            replace_file(&targets[index], &bytes)?;
        } else if targets[index].exists() {
            std::fs::remove_file(&targets[index]).map_err(|error| {
                format!(
                    "cannot remove uncommitted {}: {}",
                    targets[index].display(),
                    error
                )
            })?;
            sync_parent(&targets[index]);
        }
        restored += 1;
        if fail_after == Some(restored) {
            return Err(format!(
                "injected rollback failure after target {}",
                restored
            ));
        }
    }
    write_journal(transaction, Phase::RolledBack)?;
    cleanup(transaction, true);
    Ok(())
}

fn rollback(transaction: &Transaction) -> Result<(), String> {
    rollback_with_failure(transaction, None)
}

// Called before every Kore registry validation/publication. A prepared transaction rolls back;
// a committed one only sheds its durable recovery artifacts. No partial bundle reaches the load.
pub fn recover(live: &str) -> Result<(), String> {
    let Some((transaction, phase)) = read_journal(live)? else {
        return Ok(());
    };
    match phase {
        Phase::Preparing => {
            cleanup(&transaction, true);
            Ok(())
        }
        Phase::Prepared => rollback(&transaction),
        Phase::Committed | Phase::RolledBack => {
            cleanup(&transaction, true);
            Ok(())
        }
    }
}

fn stage_bundle(transaction: &Transaction, outcome: &MigrationOutcome) -> Result<(), String> {
    let staged = transaction.staged();
    steel::registry::reg_dump(&outcome.registry, &staged[0].to_string_lossy())
        .map_err(|diag| diag.msg)?;
    outcome.aliases.save(&staged[1]).map_err(|diag| diag.msg)?;
    write_new(&staged[2], &outcome.manifest.encode())?;
    let log_path = transaction.targets()[3].clone();
    let mut log = match std::fs::read(&log_path) {
        Ok(log) => log,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => {
            return Err(format!(
                "cannot read migration log {}: {}",
                log_path.display(),
                error
            ));
        }
    };
    log.extend_from_slice(outcome.receipt.log_line().as_bytes());
    write_new(&staged[3], &log)?;
    for path in &staged {
        std::fs::File::open(path)
            .and_then(|file| file.sync_all())
            .map_err(|error| {
                format!(
                    "cannot sync staged migration target {}: {}",
                    path.display(),
                    error
                )
            })?;
    }

    let registry =
        steel::registry::reg_load(&staged[0].to_string_lossy()).map_err(|diag| diag.msg)?;
    AliasEnvironment::load(&staged[1], &registry).map_err(|diag| diag.msg)?;
    SchemaManifest::load(&staged[2], &registry).map_err(|diag| diag.msg)?;
    Ok(())
}

fn publish_outcome_with_failure(
    live: &str,
    outcome: &MigrationOutcome,
    fail_after: Option<usize>,
) -> Result<(), String> {
    recover(live)?;
    let target_paths = targets(live);
    if target_paths
        .iter()
        .any(|path| path.exists() && !path.is_file())
    {
        return Err("a migration target exists but is not a regular file".into());
    }
    let existed = target_paths
        .iter()
        .enumerate()
        .fold(0u8, |mask, (index, path)| {
            mask | if path.exists() { 1 << index } else { 0 }
        });
    let transaction = Transaction {
        live: live.to_string(),
        nonce: nonce(live),
        existed,
    };
    write_journal(&transaction, Phase::Preparing)?;
    let prepared = (|| -> Result<(), String> {
        stage_bundle(&transaction, outcome)?;
        for (index, target) in transaction.targets().iter().enumerate() {
            if transaction.existed & (1 << index) != 0 {
                let backup = &transaction.backups()[index];
                std::fs::copy(target, backup)
                    .map_err(|error| format!("cannot back up {}: {}", target.display(), error))?;
                std::fs::File::open(backup)
                    .and_then(|file| file.sync_all())
                    .map_err(|error| {
                        format!("cannot sync backup {}: {}", backup.display(), error)
                    })?;
            }
        }
        write_journal(&transaction, Phase::Prepared)
    })();
    if let Err(error) = prepared {
        cleanup(&transaction, true);
        return Err(error);
    }

    let staged = transaction.staged();
    let published = (|| -> Result<(), String> {
        for (index, target) in transaction.targets().iter().enumerate() {
            std::fs::rename(&staged[index], target)
                .map_err(|error| format!("cannot publish {}: {}", target.display(), error))?;
            if fail_after == Some(index + 1) {
                return Err(format!(
                    "injected publication failure after target {}",
                    index + 1
                ));
            }
        }
        write_journal(&transaction, Phase::Committed)
    })();
    if let Err(error) = published {
        rollback(&transaction)?;
        return Err(error);
    }
    cleanup(&transaction, true);
    Ok(())
}

fn publish_outcome(live: &str, outcome: &MigrationOutcome) -> Result<(), String> {
    publish_outcome_with_failure(live, outcome, None)
}

pub fn run(args: &[String]) -> i32 {
    if args.len() != 4 {
        eprintln!("usage: kore migrate live.reg candidate.reg migration.map");
        return 2;
    }
    let result = (|| -> Result<_, String> {
        let live = &args[1];
        recover(live)?;
        let old = steel::registry::reg_load(live).map_err(|diag| diag.msg)?;
        let candidate = steel::registry::reg_load(&args[2]).map_err(|diag| diag.msg)?;
        let map = std::fs::read_to_string(&args[3])
            .map_err(|error| format!("cannot read migration map '{}': {}", args[3], error))?;
        let plan = MigrationPlan::parse(&map).map_err(|diag| diag.msg)?;
        let manifest = SchemaManifest::load(manifest_path(live), &old).map_err(|diag| diag.msg)?;
        let aliases = AliasEnvironment::load(sidecar_path(live), &old).map_err(|diag| diag.msg)?;
        let outcome =
            migrate(&old, &candidate, &aliases, &manifest, &plan).map_err(|diag| diag.msg)?;
        publish_outcome(live, &outcome)?;
        Ok(outcome.receipt)
    })();
    match result {
        Ok(receipt) => {
            println!(
                "migrated schema v{} {:016x} -> v{} {:016x} event {:016x}",
                receipt.old_schema.version,
                receipt.old_schema.fingerprint,
                receipt.new_schema.version,
                receipt.new_schema.fingerprint,
                receipt.event
            );
            0
        }
        Err(error) => {
            eprintln!("kore: {}", error);
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("ano-kore-migrate-{}-{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn fixture(tag: &str) -> (PathBuf, PathBuf, PathBuf) {
        let root = scratch(tag);
        let live = root.join("world.reg");
        let candidate = root.join("candidate.reg");
        let map = root.join("migration.map");
        std::fs::write(&live, b"n 2\nunique Id nat 10 20\ncol Score nat 1 2\n").unwrap();
        std::fs::write(
            &candidate,
            b"n 2\nunique Id nat 0 1\ncol Power int 0 0\ncol Added num 7 8\n",
        )
        .unwrap();
        std::fs::write(&map, b"preserve Id\nwiden Score Power\nadd Added\n").unwrap();
        let registry = steel::registry::reg_load(&live.to_string_lossy()).unwrap();
        let mut aliases = AliasEnvironment::for_registry(&registry);
        aliases
            .install_binding(&registry, "focus", "Score")
            .unwrap();
        aliases.save(sidecar_path(&live.to_string_lossy())).unwrap();
        let manifest = SchemaManifest::for_registry(&registry);
        std::fs::write(manifest_path(&live.to_string_lossy()), manifest.encode()).unwrap();
        std::fs::write(format!("{}.migrations", live.to_string_lossy()), b"prior\n").unwrap();
        (live, candidate, map)
    }

    fn outcome(live: &Path, candidate: &Path, map: &Path) -> MigrationOutcome {
        let old = steel::registry::reg_load(&live.to_string_lossy()).unwrap();
        let next = steel::registry::reg_load(&candidate.to_string_lossy()).unwrap();
        let plan = MigrationPlan::parse(&std::fs::read_to_string(map).unwrap()).unwrap();
        let manifest = SchemaManifest::load(manifest_path(&live.to_string_lossy()), &old).unwrap();
        let aliases = AliasEnvironment::load(sidecar_path(&live.to_string_lossy()), &old).unwrap();
        migrate(&old, &next, &aliases, &manifest, &plan).unwrap()
    }

    #[test]
    fn command_publishes_registry_alias_manifest_and_log_together() {
        let (live, candidate, map) = fixture("command");
        let args = vec![
            "migrate".to_string(),
            live.to_string_lossy().into_owned(),
            candidate.to_string_lossy().into_owned(),
            map.to_string_lossy().into_owned(),
        ];
        assert_eq!(run(&args), 0);
        let registry = steel::registry::reg_load(&live.to_string_lossy()).unwrap();
        assert_eq!(registry.ents[1].name, "Power");
        let manifest =
            SchemaManifest::load(manifest_path(&live.to_string_lossy()), &registry).unwrap();
        assert_eq!(manifest.version, 1);
        AliasEnvironment::load(sidecar_path(&live.to_string_lossy()), &registry).unwrap();
        let log =
            std::fs::read_to_string(format!("{}.migrations", live.to_string_lossy())).unwrap();
        assert!(log.starts_with("prior\n"));
        assert!(log.contains("ano-migration-v1"));
        let _ = std::fs::remove_dir_all(live.parent().unwrap());
    }

    #[test]
    fn failure_after_partial_publication_restores_every_old_byte() {
        let (live, candidate, map) = fixture("rollback");
        let outcome = outcome(&live, &candidate, &map);
        let before = targets(&live.to_string_lossy()).map(|path| std::fs::read(path).unwrap());
        assert!(publish_outcome_with_failure(&live.to_string_lossy(), &outcome, Some(2)).is_err());
        let after = targets(&live.to_string_lossy()).map(|path| std::fs::read(path).unwrap());
        assert_eq!(before, after);
        assert!(!journal_path(&live.to_string_lossy()).exists());
        let _ = std::fs::remove_dir_all(live.parent().unwrap());
    }

    #[test]
    fn prepared_crash_journal_rolls_back_before_load() {
        let (live, _, _) = fixture("recovery");
        let live_text = live.to_string_lossy();
        let target_paths = targets(&live_text);
        let transaction = Transaction {
            live: live_text.into_owned(),
            nonce: "999.1".into(),
            existed: 0x0f,
        };
        for (target, backup) in target_paths.iter().zip(transaction.backups()) {
            std::fs::copy(target, backup).unwrap();
        }
        write_journal(&transaction, Phase::Prepared).unwrap();
        std::fs::write(&target_paths[0], b"partial publication\n").unwrap();
        recover(&transaction.live).unwrap();
        assert!(steel::registry::reg_load(&transaction.live).is_ok());
        assert!(!journal_path(&transaction.live).exists());
        let _ = std::fs::remove_dir_all(live.parent().unwrap());
    }

    #[test]
    fn rollback_recovery_restarts_after_a_second_crash() {
        let (live, candidate, map) = fixture("rollback-restart");
        let outcome = outcome(&live, &candidate, &map);
        let live_text = live.to_string_lossy().into_owned();
        let target_paths = targets(&live_text);
        let before = target_paths
            .clone()
            .map(|path| std::fs::read(path).unwrap());
        let transaction = Transaction {
            live: live_text,
            nonce: "999.2".into(),
            existed: 0x0f,
        };

        write_journal(&transaction, Phase::Preparing).unwrap();
        stage_bundle(&transaction, &outcome).unwrap();
        for (target, backup) in target_paths.iter().zip(transaction.backups()) {
            std::fs::copy(target, backup).unwrap();
        }
        write_journal(&transaction, Phase::Prepared).unwrap();
        for target in &target_paths {
            std::fs::write(target, b"partial publication\n").unwrap();
        }

        assert!(rollback_with_failure(&transaction, Some(1)).is_err());
        assert!(transaction.backups().iter().all(|backup| backup.exists()));
        assert!(matches!(
            read_journal(&transaction.live).unwrap(),
            Some((_, Phase::Prepared))
        ));

        recover(&transaction.live).unwrap();
        let after = target_paths.map(|path| std::fs::read(path).unwrap());
        assert_eq!(before, after);
        assert!(!journal_path(&transaction.live).exists());
        let _ = std::fs::remove_dir_all(live.parent().unwrap());
    }
}
