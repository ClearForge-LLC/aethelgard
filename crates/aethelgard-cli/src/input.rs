//! Inputs: checked, copied into the call's temporary directory, and hashed as
//! copied. The tool judges the copy, so the verdict's hash is of exactly the
//! bytes judged (`INV-03`) and nothing is ever written beside the original.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::run::Stop;

/// The cap on one input, in bytes (`RUL-13`): `validate`'s file or stdin, and
/// the sum of every file a tool input copies. Larger is refused unread past the
/// cap, never truncated.
pub const MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;

/// The cap on the files in a crate, counted after a top-level `target/` is
/// skipped.
pub const MAX_CRATE_FILES: usize = 4096;

/// An input ready to be judged.
pub struct Input {
    /// The canonical path of the original, as reported in the verdict.
    pub original: PathBuf,
    /// The copy inside the temporary directory, as given to the tool.
    pub copy: PathBuf,
    pub sha256: String,
}

pub fn hex(digest: impl AsRef<[u8]>) -> String {
    digest.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

/// The canonical path, which must be UTF-8 so the verdict can report it
/// exactly.
fn canonical(path: &Path) -> Result<PathBuf, Stop> {
    let resolved = fs::canonicalize(path)
        .map_err(|e| Stop::unreadable(format!("cannot resolve {}: {e}", path.display())))?;
    if resolved.to_str().is_none() {
        return Err(Stop::unreadable(format!(
            "{} is not a UTF-8 path",
            resolved.display()
        )));
    }
    Ok(resolved)
}

/// Copies `from` to `to`, hashing exactly the bytes written. Reads at most one
/// byte past `budget`, the bytes the whole input may still use, and refuses
/// (naming `what`) rather than copy past it; on success `budget` is reduced by
/// the bytes copied.
fn copy_hashed(from: &Path, to: &Path, budget: &mut u64, what: &str) -> Result<String, Stop> {
    let unreadable =
        |e: std::io::Error| Stop::unreadable(format!("cannot copy {}: {e}", from.display()));
    let src = File::open(from).map_err(unreadable)?;
    // The open file must still be a regular file: no swap to a FIFO or device
    // between the check and the open.
    if !src.metadata().map_err(unreadable)?.is_file() {
        return Err(Stop::unreadable(format!(
            "{} is not a regular file",
            from.display()
        )));
    }
    let mut src = src.take(*budget + 1);
    let mut dst = File::create(to).map_err(unreadable)?;
    let mut copied = 0u64;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = src.read(&mut buf).map_err(unreadable)?;
        if n == 0 {
            break;
        }
        copied += n as u64;
        if copied > *budget {
            return Err(Stop::too_large(format!(
                "{what} exceeds {MAX_INPUT_BYTES} bytes"
            )));
        }
        hasher.update(&buf[..n]);
        dst.write_all(&buf[..n]).map_err(unreadable)?;
    }
    *budget -= copied;
    Ok(hex(hasher.finalize()))
}

/// Refuses, before any tool is looked for, a path that is not a regular file
/// after following symlinks. Returns its canonical path.
pub fn file_check(path: &Path) -> Result<PathBuf, Stop> {
    let original = canonical(path)?;
    if !original.is_file() {
        return Err(Stop::unreadable(format!(
            "{} is not a regular file",
            original.display()
        )));
    }
    Ok(original)
}

/// Refuses, before any tool is looked for, a crate path that is not a directory.
pub fn dir_check(path: &Path) -> Result<(), Stop> {
    if canonical(path)?.is_dir() {
        Ok(())
    } else {
        Err(Stop::unreadable(format!(
            "{} is not a directory",
            path.display()
        )))
    }
}

/// A board, schematic or platform file: a regular file after following
/// symlinks, copied to `dest`.
pub fn file(path: &Path, dest: &Path) -> Result<Input, Stop> {
    let original = file_check(path)?;
    let what = original.display().to_string();
    let mut budget = MAX_INPUT_BYTES;
    let sha256 = copy_hashed(&original, dest, &mut budget, &what)?;
    Ok(Input {
        original,
        copy: dest.to_path_buf(),
        sha256,
    })
}

/// A Renode platform: a file, or `@path` inside Renode's own tree (the
/// directory holding the real `renode` binary), as Renode's `@` syntax reads.
pub fn platform(arg: &Path, renode_root: &Path, dest: &Path) -> Result<Input, Stop> {
    let text = arg.as_os_str().as_encoded_bytes();
    let Some(rest) = text.strip_prefix(b"@") else {
        return file(arg, dest);
    };
    let rel = Path::new(
        std::str::from_utf8(rest)
            .map_err(|_| Stop::unreadable("the @ path is not UTF-8".to_owned()))?,
    );
    let root = canonical(renode_root)?;
    let resolved = canonical(&root.join(rel))?;
    if !resolved.starts_with(&root) {
        return Err(Stop::unreadable(format!(
            "{} is outside Renode's tree {}",
            resolved.display(),
            root.display()
        )));
    }
    file(&resolved, dest)
}

/// A crate directory, copied to `dest` minus a top-level `target/`, with the
/// tree hash defined in the README.
pub fn crate_dir(path: &Path, dest: &Path) -> Result<Input, Stop> {
    let original = canonical(path)?;
    let mut files = Vec::new();
    walk(&original, &original, &mut files)?;
    files.sort();
    let what = format!("crate {}", original.display());
    let mut budget = MAX_INPUT_BYTES;
    let mut tree = Sha256::new();
    for rel in &files {
        let to = dest.join(rel);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| Stop::unreadable(format!("cannot copy {rel}: {e}")))?;
        }
        let digest = copy_hashed(&original.join(rel), &to, &mut budget, &what)?;
        tree.update(format!("{digest}  {rel}\n").as_bytes());
    }
    fs::create_dir_all(dest)
        .map_err(|e| Stop::unreadable(format!("cannot copy the crate: {e}")))?;
    Ok(Input {
        original,
        copy: dest.to_path_buf(),
        sha256: hex(tree.finalize()),
    })
}

/// Collects every regular file beneath `dir` as a `/`-separated UTF-8 path
/// relative to `root`, skipping `root/target`. Refuses symlinks, special files
/// and names that are not UTF-8.
fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), Stop> {
    let entries = fs::read_dir(dir)
        .map_err(|e| Stop::unreadable(format!("cannot read {}: {e}", dir.display())))?;
    for entry in entries {
        let entry =
            entry.map_err(|e| Stop::unreadable(format!("cannot read {}: {e}", dir.display())))?;
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .ok()
            .and_then(Path::to_str)
            .ok_or_else(|| {
                Stop::unreadable(format!("{} has a name that is not UTF-8", path.display()))
            })?
            .to_owned();
        let kind = entry
            .file_type()
            .map_err(|e| Stop::unreadable(format!("cannot read {rel}: {e}")))?;
        if kind.is_symlink() {
            return Err(Stop::unreadable(format!("{rel} in the crate is a symlink")));
        } else if kind.is_dir() {
            if dir == root && rel == "target" {
                continue;
            }
            walk(root, &path, out)?;
        } else if kind.is_file() {
            if out.len() >= MAX_CRATE_FILES {
                return Err(Stop::too_large(format!(
                    "crate {} holds more than {MAX_CRATE_FILES} files",
                    root.display()
                )));
            }
            out.push(rel);
        } else {
            return Err(Stop::unreadable(format!(
                "{rel} in the crate is not a regular file"
            )));
        }
    }
    Ok(())
}
