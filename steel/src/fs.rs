// Path values and the shared file reader/writer. AnoPath is a checked
// value (the C len == 0 error state becomes None at every constructor), normalization is
// purely lexical, writes commit via staged file + rename(2). No CWD mutation anywhere.

use std::io::{Error, Read, Write};


pub const ANO_PATHSZ: usize = 1024;

// A resolved path. Invariant: only constructed through the checked fns below — anything the
// C spelled len == 0 (empty input, >= 1024 bytes, unresolvable exe dir) is None instead.
#[derive(Debug, Clone)]
pub struct AnoPath {
    pub s: String,
}

// Inputs: any string. Output: checked copy; None when empty or len >= ANO_PATHSZ
// (truncation is an error, never silent).
pub fn fs_path(s: &str) -> Option<AnoPath> {
    if s.is_empty() || s.len() >= ANO_PATHSZ {
        return None;
    }
    Some(AnoPath { s: s.to_string() })
}

// Output: directory of the running binary (/proc/self/exe on Linux), no file name, trailing
// slash dropped but "/" kept at root; None when unreadable or overlong. Callers fall back to ".".
pub fn fs_exe_dir() -> Option<AnoPath> {
    let exe = std::env::current_exe().ok()?;
    let s = exe.into_os_string().into_string().ok()?;
    let b = s.as_bytes();
    let mut len = b.len();
    while len > 0 && b[len - 1] != b'/' {
        len -= 1;
    }
    // drop the trailing slash, keep "/" at root
    if len > 1 {
        len -= 1;
    }
    if len == 0 || len >= ANO_PATHSZ {
        return None;
    }
    Some(AnoPath { s: s[..len].to_string() })
}

// Inputs: a path. Output: its directory — "." when slash-free or empty input, "/" kept at
// root, trailing separators stripped ("foo/bar/" -> "foo/bar"); None only on overflow.
pub fn fs_dirname(path: &str) -> Option<AnoPath> {
    if path.is_empty() {
        return fs_path(".");
    }
    let r = fs_path(path)?;
    let b = r.s.as_bytes();
    let mut len = b.len();
    while len > 0 && b[len - 1] != b'/' {
        len -= 1;
    }
    if len == 0 {
        return fs_path(".");
    }
    // strip separators, keep "/" at root
    while len > 1 && b[len - 1] == b'/' {
        len -= 1;
    }
    Some(AnoPath { s: r.s[..len].to_string() })
}

// Inputs: dir (empty -> "."), rel. Output: absolute rel passes VERBATIM; else "dir/rel";
// None on overflow (combined len >= ANO_PATHSZ).
pub fn fs_join(dir: &str, rel: &str) -> Option<AnoPath> {
    if rel.as_bytes().first() == Some(&b'/') {
        return fs_path(rel);
    }
    let d = if dir.is_empty() { "." } else { dir };
    let s = format!("{}/{}", d, rel);
    if s.len() >= ANO_PATHSZ {
        return None;
    }
    Some(AnoPath { s })
}

// In-place LEXICAL ./.. collapse, no filesystem access: "." and empty segments drop; ".."
// pops a real segment but never a retained leading ".."; leading ".."s survive on relative
// paths, clamp at root on absolute ones; a relative path collapsing to nothing becomes ".".
// Invariant: output never exceeds input length.
pub fn fs_norm(p: &mut AnoPath) {
    if p.s.is_empty() {
        return;
    }
    let src = std::mem::take(&mut p.s);
    let b = src.as_bytes();
    let abs = b[0] == b'/';
    let mut out = String::with_capacity(src.len());
    let mut start: Vec<usize> = Vec::new(); // segment start offsets into out
    let mut i = if abs { 1 } else { 0 };
    while i < b.len() {
        let mut j = i;
        while j < b.len() && b[j] != b'/' {
            j += 1;
        }
        let seg = &src[i..j];
        let dotdot = seg == "..";
        let last_is_dotdot = start
            .last()
            .is_some_and(|&st| &out[st..] == "..");
        if seg.is_empty() || seg == "." {
            // skip
        } else if dotdot && !start.is_empty() && !last_is_dotdot {
            // pop a real segment (and its separator)
            if let Some(st) = start.pop() {
                out.truncate(if start.is_empty() { 0 } else { st - 1 });
            }
        } else if dotdot && abs && start.is_empty() {
            // clamp at root
        } else {
            if !out.is_empty() {
                out.push('/');
            }
            start.push(out.len());
            out.push_str(seg);
        }
        i = if j < b.len() { j + 1 } else { j };
    }
    p.s = if abs {
        format!("/{}", out)
    } else if out.is_empty() {
        ".".to_string()
    } else {
        out
    };
}

// realpath (std::fs::canonicalize) when the target exists and the result fits < ANO_PATHSZ;
// fs_norm fallback otherwise — a missing file still reports a clean path with the right errno.
pub fn fs_canon(p: &mut AnoPath) {
    if !p.s.is_empty() {
        if let Ok(real) = std::fs::canonicalize(&p.s) {
            if let Ok(s) = real.into_os_string().into_string() {
                if s.len() < ANO_PATHSZ {
                    p.s = s;
                    return;
                }
            }
        }
    }
    fs_norm(p);
}

// The one fortified reader (main and registry both come through here). Inputs: path.
// Output: whole file bytes, or an io::Error whose raw_os_error renders target-native errno text —
// directories refuse as EISDIR, other non-regular files as EINVAL, short reads as EIO.
pub fn fs_read(path: &str) -> std::io::Result<Vec<u8>> {
    let mut f = std::fs::File::open(path)?;
    let md = f.metadata()?;
    let ft = md.file_type();
    if ft.is_dir() {
        return Err(condition(FsCondition::IsADirectory));
    }
    if !ft.is_file() {
        return Err(condition(FsCondition::NotAFile));
    }
    let mut buf = Vec::with_capacity(md.len() as usize + 1);
    // errno tells the story; a non-OS read failure is C's short-read EIO
    f.read_to_end(&mut buf).map_err(os_or_eio)?;
    Ok(buf)
}

// Writes and syncs "<path>.staged", then atomically renames it over the target. The parent
// directory is not synced. Paths at or above ANO_PATHSZ refuse ENAMETOOLONG. Failures after
// staging remove the staged file and preserve the original error.
pub fn fs_write_commit(path: &str, data: &[u8]) -> std::io::Result<()> {
    let staged = format!("{}.staged", path);
    if staged.len() >= ANO_PATHSZ {
        return Err(condition(FsCondition::NameTooLong));
    }
    let mut f = std::fs::File::create(&staged)?;
    if let Err(e) = f.write_all(data) {
        let e = os_or_eio(e);
        drop(f);
        let _ = std::fs::remove_file(&staged);
        return Err(e);
    }
    if let Err(e) = f.sync_all() {
        drop(f);
        let _ = std::fs::remove_file(&staged);
        return Err(e);
    }
    drop(f);
    if let Err(e) = std::fs::rename(&staged, path) {
        let _ = std::fs::remove_file(&staged);
        return Err(e);
    }
    Ok(())
}

// One filesystem condition the reader and the committer report.
pub enum FsCondition {
    // the path names a directory where a file is required
    IsADirectory,
    // the path names something that is not a usable file
    NotAFile,
    // the constructed name exceeds what the platform accepts
    NameTooLong,
    // an underlying operation failed without reporting a cause
    Unattributed,
}

// Inputs: one FsCondition. Output: an io::Error carrying the platform's own code for that
// condition, so a diagnostic reads the same as any other tool on the platform reports it.
fn condition(_which: FsCondition) -> Error {
    todo!()
}

// Inputs: an io::Error. Output: the same error when it already carries an OS code, and the
// unattributed-failure code otherwise.
fn os_or_eio(_e: Error) -> Error {
    todo!()
}

// Inputs: an io::Error. Output: the platform's native message text for it. The text is the
// operating system's own wording and carries no Rust-added decoration such as a trailing
// parenthesized code, so one diagnostic line matches what the platform reports everywhere
// else. A code the platform does not name renders as "Unknown error <code>".
pub fn strerror(_e: &std::io::Error) -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn norm(s: &str) -> String {
        let mut p = fs_path(s).unwrap();
        fs_norm(&mut p);
        p.s
    }

    #[test]
    fn dirname_quirks() {
        assert_eq!(fs_dirname("foo/bar").unwrap().s, "foo");
        assert_eq!(fs_dirname("foo/bar/").unwrap().s, "foo/bar");
        assert_eq!(fs_dirname("foo///bar").unwrap().s, "foo");
        assert_eq!(fs_dirname("/x").unwrap().s, "/");
        assert_eq!(fs_dirname("///").unwrap().s, "/");
        assert_eq!(fs_dirname("x").unwrap().s, ".");
        assert_eq!(fs_dirname("").unwrap().s, ".");
        assert!(fs_dirname(&"a".repeat(ANO_PATHSZ)).is_none());
    }

    #[test]
    fn norm_collapse() {
        assert_eq!(norm("a/./b//c"), "a/b/c");
        assert_eq!(norm("a/b/../c"), "a/c");
        assert_eq!(norm("a/.."), ".");
        assert_eq!(norm("./."), ".");
        assert_eq!(norm("../../a"), "../../a");
        assert_eq!(norm("a/../../b"), "../b");
        assert_eq!(norm("/../a"), "/a");
        assert_eq!(norm("/a/../.."), "/");
        assert_eq!(norm("/"), "/");
        assert_eq!(norm("a/b/../../.."), "..");
    }

    #[test]
    fn join_rules() {
        assert_eq!(fs_join("d", "r").unwrap().s, "d/r");
        assert_eq!(fs_join("", "r").unwrap().s, "./r");
        assert_eq!(fs_join("d", "/abs").unwrap().s, "/abs");
        assert_eq!(fs_join("d", "").unwrap().s, "d/");
        assert!(fs_join(&"a".repeat(1000), &"b".repeat(100)).is_none());
    }
}
