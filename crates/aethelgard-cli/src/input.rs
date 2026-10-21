//! Inputs: checked, copied into the call's temporary directory, and hashed as
//! copied. The tool judges the copy, so the verdict's hash is of exactly the
//! bytes judged (`INV-03`) and nothing is ever written beside the original.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::run::Stop;

/// An input ready to be judged.
pub struct Input {
    /// The canonical path of the original, as reported in the verdict.
    pub original: PathBuf,
    /// The copy inside the temporary directory, as given to the tool.
    pub copy: PathBuf,
    pub sha256: String,
}

fn hex(digest: impl AsRef<[u8]>) -> String {
    digest.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

fn canonical(path: &Path) -> Result<PathBuf, Stop> {
    fs::canonicalize(path)
        .map_err(|e| Stop::unreadable(format!("cannot resolve {}: {e}", path.display())))
}

/// Copies `from` to `to`, hashing exactly the bytes written.
fn copy_hashed(from: &Path, to: &Path) -> Result<String, Stop> {
    let unreadable =
        |e: std::io::Error| Stop::unreadable(format!("cannot copy {}: {e}", from.display()));
    let mut src = File::open(from).map_err(unreadable)?;
    // The open file must still be a regular file: no swap to a FIFO or device
    // between the check and the open.
    if !src.metadata().map_err(unreadable)?.is_file() {
        return Err(Stop::unreadable(format!(
            "{} is not a regular file",
            from.display()
        )));
    }
    let mut dst = File::create(to).map_err(unreadable)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = src.read(&mut buf).map_err(unreadable)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        dst.write_all(&buf[..n]).map_err(unreadable)?;
    }
    Ok(hex(hasher.finalize()))
}

/// Refuses, before any tool is looked for, a path that is not a regular file
/// after following symlinks. Returns its canonical path.
pub fn file_check(path: &Path) -> Result<PathBuf, Stop> {
    let original = canonical(path)?;
    let meta = fs::metadata(&original)
        .map_err(|e| Stop::unreadable(format!("cannot read {}: {e}", original.display())))?;
    if !meta.is_file() {
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
    let sha256 = copy_hashed(&original, dest)?;
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
    if !original.is_dir() {
        return Err(Stop::unreadable(format!(
            "{} is not a directory",
            original.display()
        )));
    }
    let mut files = Vec::new();
    walk(&original, &original, &mut files)?;
    files.sort();
    let mut tree = Sha256::new();
    for rel in &files {
        let to = dest.join(rel);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| Stop::unreadable(format!("cannot copy {rel}: {e}")))?;
        }
        let digest = copy_hashed(&original.join(rel), &to)?;
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
            out.push(rel);
        } else {
            return Err(Stop::unreadable(format!(
                "{rel} in the crate is not a regular file"
            )));
        }
    }
    Ok(())
}
