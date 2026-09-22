//! Validated registry publication for every Kore world edit.

use std::io::Write;
use std::path::{Path, PathBuf};

fn staged_path(path: &Path) -> PathBuf {
    let mut serial = 0u64;
    loop {
        let candidate = PathBuf::from(format!(
            "{}.staged.{}.{}",
            path.to_string_lossy(),
            std::process::id(),
            serial
        ));
        if !candidate.exists() {
            return candidate;
        }
        serial = serial.wrapping_add(1);
    }
}

/// Stage bytes, parse and seal the complete relationship-bearing registry, then publish.
/// The live path is untouched on every validation or I/O failure before rename.
pub fn publish(path: &str, data: &[u8]) -> Result<(), String> {
    crate::migrate::recover(path)?;
    let old = steel::registry::reg_load(path).map_err(|diag| diag.msg)?;
    let old_schema = steel::alias::registry_fingerprint(&old);
    let live = Path::new(path);
    let staged = staged_path(live);
    let result = (|| -> Result<(), String> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged)
            .map_err(|error| format!("cannot stage {}: {}", path, error))?;
        file.write_all(data)
            .map_err(|error| format!("cannot stage {}: {}", path, error))?;
        file.sync_all()
            .map_err(|error| format!("cannot sync staged {}: {}", path, error))?;
        drop(file);
        let registry =
            steel::registry::reg_load(&staged.to_string_lossy()).map_err(|diag| diag.msg)?;
        if steel::alias::registry_fingerprint(&registry) != old_schema {
            return Err(
                "schema-changing registry publication requires `kore migrate`".into(),
            );
        }
        steel::migration::SchemaManifest::load(
            steel::migration::manifest_path(path),
            &registry,
        )
        .map_err(|diag| diag.msg)?;
        std::fs::rename(&staged, live)
            .map_err(|error| format!("cannot publish {}: {}", path, error))?;
        if let Some(parent) = live.parent().filter(|parent| !parent.as_os_str().is_empty()) {
            if let Ok(directory) = std::fs::File::open(parent) {
                let _ = directory.sync_all();
            }
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&staged);
    }
    result
}

pub fn validate(path: &str) -> Result<(), String> {
    crate::migrate::recover(path)?;
    let registry = steel::registry::reg_load(path).map_err(|diag| diag.msg)?;
    steel::migration::SchemaManifest::load(
        steel::migration::manifest_path(path),
        &registry,
    )
    .map(|_| ())
    .map_err(|diag| diag.msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_stage_never_replaces_live_file() {
        let root = std::env::temp_dir().join(format!(
            "ano-registry-tx-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("world.reg");
        let original = b"n 1\ncol Value num 0\n";
        std::fs::write(&path, original).unwrap();
        let invalid = b"n 1\nrel Parent 1.5\n";
        assert!(publish(&path.to_string_lossy(), invalid).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ordinary_publication_cannot_cross_a_schema_barrier() {
        let root = std::env::temp_dir().join(format!(
            "ano-registry-schema-tx-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("world.reg");
        let original = b"n 1\ncol Value num 0\n";
        std::fs::write(&path, original).unwrap();

        let changed = b"n 1\ncol Renamed num 0\n";
        assert!(
            publish(&path.to_string_lossy(), changed)
                .unwrap_err()
                .contains("requires `kore migrate`")
        );
        assert_eq!(std::fs::read(&path).unwrap(), original);
        let _ = std::fs::remove_dir_all(&root);
    }
}
