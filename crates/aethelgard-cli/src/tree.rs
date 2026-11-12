//! A generated tree on disk: written under one directory and nowhere else,
//! compared byte for byte with a directory, and hashed as `prove` and `build`
//! hash a crate (`AET-WO-3301`).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::os::fd::OwnedFd;
use std::path::Path;

use rustix::fs::{Mode, OFlags};
use sha2::{Digest, Sha256};

use crate::input::{MAX_CRATE_FILES, hex};
use crate::run::Stop;

fn unwritable(message: String) -> Stop {
    Stop::refused("UnwritableOutput", message)
}

/// Writes `files` under `out`, which must be an empty directory or absent.
///
/// Every directory and file is created relative to a descriptor of its
/// parent, never followed through a symlink and never opened if it exists:
/// directories with `mkdirat`, files with `O_CREAT | O_EXCL | O_NOFOLLOW`. So
/// nothing is written outside `out` and nothing that exists is overwritten,
/// whatever another process does to `out` meanwhile.
pub fn write(out: &Path, files: &BTreeMap<String, String>) -> Result<(), Stop> {
    match fs::symlink_metadata(out) {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => {
            return Err(unwritable(format!(
                "{} exists and is not a directory",
                out.display()
            )));
        }
        Err(_) => fs::create_dir_all(out)
            .map_err(|e| unwritable(format!("cannot create {}: {e}", out.display())))?,
    }
    let root = rustix::fs::open(
        out,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|e| unwritable(format!("cannot open {}: {e}", out.display())))?;
    // Empty as opened: the check is on the descriptor written through.
    let entries = rustix::fs::Dir::read_from(&root)
        .map_err(|e| unwritable(format!("cannot read {}: {e}", out.display())))?;
    for entry in entries {
        let entry = entry.map_err(|e| unwritable(format!("cannot read {}: {e}", out.display())))?;
        let name = entry.file_name().to_bytes();
        if name != b"." && name != b".." {
            return Err(unwritable(format!(
                "{} is not empty: generate writes only into an empty or absent directory",
                out.display()
            )));
        }
    }
    let mut dirs: BTreeMap<String, OwnedFd> = BTreeMap::new();
    for (path, text) in files {
        let parts: Vec<&str> = path.split('/').collect();
        if parts
            .iter()
            .any(|p| p.is_empty() || *p == "." || *p == "..")
        {
            return Err(unwritable(format!("`{path}` is not a relative path")));
        }
        let (name, parents) = parts.split_last().expect("split gives one part");
        let mut at = String::new();
        for part in parents {
            let parent = if at.is_empty() { &root } else { &dirs[&at] };
            let next = if at.is_empty() {
                (*part).to_owned()
            } else {
                format!("{at}/{part}")
            };
            if !dirs.contains_key(&next) {
                // Made here or refused: a directory that already exists was
                // not made by this call.
                rustix::fs::mkdirat(parent, *part, Mode::from_raw_mode(0o755))
                    .map_err(|e| unwritable(format!("cannot create {next}: {e}")))?;
                let fd = rustix::fs::openat(
                    parent,
                    *part,
                    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::empty(),
                )
                .map_err(|e| unwritable(format!("cannot open {next}: {e}")))?;
                dirs.insert(next.clone(), fd);
            }
            at = next;
        }
        let parent = if at.is_empty() { &root } else { &dirs[&at] };
        let fd = rustix::fs::openat(
            parent,
            *name,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::from_raw_mode(0o644),
        )
        .map_err(|e| unwritable(format!("cannot create {path}: {e}")))?;
        fs::File::from(fd)
            .write_all(text.as_bytes())
            .map_err(|e| unwritable(format!("cannot write {path}: {e}")))?;
    }
    Ok(())
}

/// How one path of a directory differs from the generated tree.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Difference {
    /// In the tree, not in the directory.
    Missing(String),
    /// In both, with other bytes, or not a regular file in the directory.
    Differs(String),
    /// In the directory, not in the tree.
    Extra(String),
}

impl Difference {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Missing(_) => "against_missing",
            Self::Differs(_) => "against_differs",
            Self::Extra(_) => "against_extra",
        }
    }
}

/// Every difference between `dir` and `files`, in path order; empty means
/// byte-identical. A directory named in `skip` (relative, such as
/// `board/target`) is not read, as a crate's `target/` is not part of it.
/// Symlinks are never followed: one is a difference. More than
/// [`MAX_CRATE_FILES`] entries is refused, never truncated.
pub fn diff(
    dir: &Path,
    files: &BTreeMap<String, String>,
    skip: &[&str],
) -> Result<Vec<Difference>, Stop> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    let mut stack = vec![(dir.to_path_buf(), String::new())];
    let unreadable =
        |p: &Path, e: std::io::Error| Stop::unreadable(format!("cannot read {}: {e}", p.display()));
    let mut count = 0usize;
    while let Some((path, rel)) = stack.pop() {
        for entry in fs::read_dir(&path).map_err(|e| unreadable(&path, e))? {
            let entry = entry.map_err(|e| unreadable(&path, e))?;
            count += 1;
            if count > MAX_CRATE_FILES {
                return Err(Stop::too_large(format!(
                    "{} holds more than {MAX_CRATE_FILES} entries",
                    dir.display()
                )));
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let child = if rel.is_empty() {
                name
            } else {
                format!("{rel}/{name}")
            };
            let kind = entry
                .file_type()
                .map_err(|e| unreadable(&entry.path(), e))?;
            if kind.is_dir() {
                if !skip.contains(&child.as_str()) {
                    stack.push((entry.path(), child));
                }
                continue;
            }
            seen.insert(child.clone());
            match files.get(&child) {
                None => out.push(Difference::Extra(child)),
                Some(text) if !kind.is_file() || !same(&entry.path(), text.as_bytes())? => {
                    out.push(Difference::Differs(child));
                }
                Some(_) => {}
            }
        }
    }
    out.extend(
        files
            .keys()
            .filter(|p| !seen.contains(*p))
            .map(|p| Difference::Missing(p.clone())),
    );
    out.sort();
    Ok(out)
}

/// Whether the regular file at `path` holds exactly `want`, reading at most
/// one byte more than `want`.
fn same(path: &Path, want: &[u8]) -> Result<bool, Stop> {
    let opened = fs::OpenOptions::new()
        .read(true)
        .custom_flags((OFlags::NOFOLLOW | OFlags::NONBLOCK).bits() as i32)
        .open(path);
    let Ok(file) = opened else {
        return Ok(false);
    };
    if !file.metadata().is_ok_and(|m| m.is_file()) {
        return Ok(false);
    }
    let mut have = Vec::with_capacity(want.len());
    file.take(want.len() as u64 + 1)
        .read_to_end(&mut have)
        .map_err(|e| Stop::unreadable(format!("cannot read {}: {e}", path.display())))?;
    Ok(have == want)
}

use std::os::unix::fs::OpenOptionsExt;

/// The files of `files` under `prefix` (such as `board/`), relative to it.
pub fn subtree(files: &BTreeMap<String, String>, prefix: &str) -> BTreeMap<String, String> {
    files
        .iter()
        .filter_map(|(path, text)| Some((path.strip_prefix(prefix)?.to_owned(), text.clone())))
        .collect()
}

/// The sha256 of one file's bytes, as hex.
pub fn sha256(bytes: &[u8]) -> String {
    hex(Sha256::digest(bytes))
}

/// The tree hash `input::crate_dir` reports for a crate holding exactly
/// `files`: the sha256 of `<sha256>  <path>\n` for each file in path order.
pub fn crate_sha256(files: &BTreeMap<String, String>) -> String {
    let mut tree = Sha256::new();
    for (rel, text) in files {
        tree.update(format!("{}  {rel}\n", sha256(text.as_bytes())).as_bytes());
    }
    hex(tree.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(label: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("aet-3301-tree-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn files() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("Cargo.toml".to_owned(), "[package]\n".to_owned()),
            ("src/lib.rs".to_owned(), "//! x\n".to_owned()),
            (
                "firmware/src/main.rs".to_owned(),
                "fn main() {}\n".to_owned(),
            ),
        ])
    }

    /// The hash `verify` expects is the one `prove` and `build` report.
    #[test]
    fn the_crate_hash_is_the_copied_crates() {
        let dir = scratch("hash");
        write(&dir, &files()).unwrap();
        let copy = scratch("hash-copy");
        let copied = crate::input::crate_dir(&dir, &copy).unwrap();
        assert_eq!(copied.sha256, crate_sha256(&files()));
        fs::remove_dir_all(&dir).unwrap();
        fs::remove_dir_all(&copy).unwrap();
    }

    #[test]
    fn write_refuses_a_used_or_linked_directory_and_a_path_out_of_it() {
        let dir = scratch("used");
        write(&dir, &files()).unwrap();
        let before = diff(&dir, &files(), &[]).unwrap();
        assert!(before.is_empty(), "{before:?}");
        // Not empty: refused, and nothing written.
        let e = write(&dir, &BTreeMap::from([("new".to_owned(), String::new())])).unwrap_err();
        assert_eq!(e.kind, "UnwritableOutput");
        assert!(!dir.join("new").exists());
        // A symlink to an empty directory is not followed.
        let target = scratch("target");
        fs::create_dir_all(&target).unwrap();
        let link = scratch("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert_eq!(write(&link, &files()).unwrap_err().kind, "UnwritableOutput");
        assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
        // A file is not a directory.
        let file = scratch("file");
        fs::write(&file, "x").unwrap();
        assert_eq!(write(&file, &files()).unwrap_err().kind, "UnwritableOutput");
        // A path that would leave the directory is refused.
        let out = scratch("dotdot");
        for bad in ["../escape", "a/../../escape", "/abs", "a//b"] {
            let _ = fs::remove_dir_all(&out);
            let e = write(&out, &BTreeMap::from([(bad.to_owned(), String::new())])).unwrap_err();
            assert_eq!(e.kind, "UnwritableOutput", "{bad}");
        }
        assert!(!out.parent().unwrap().join("escape").exists());
        for p in [&dir, &target, &out] {
            fs::remove_dir_all(p).unwrap();
        }
        fs::remove_file(&link).unwrap();
        fs::remove_file(&file).unwrap();
    }

    #[test]
    fn diff_names_every_missing_differing_extra_and_linked_file() {
        let dir = scratch("diff");
        write(&dir, &files()).unwrap();
        fs::write(dir.join("src/lib.rs"), "//! y\n").unwrap();
        fs::remove_file(dir.join("Cargo.toml")).unwrap();
        fs::write(dir.join("extra.rs"), "").unwrap();
        fs::remove_file(dir.join("firmware/src/main.rs")).unwrap();
        std::os::unix::fs::symlink("/etc/hostname", dir.join("firmware/src/main.rs")).unwrap();
        fs::create_dir_all(dir.join("target/x")).unwrap();
        fs::write(dir.join("target/x/y"), "").unwrap();
        assert_eq!(
            diff(&dir, &files(), &["target"]).unwrap(),
            [
                Difference::Missing("Cargo.toml".to_owned()),
                Difference::Differs("firmware/src/main.rs".to_owned()),
                Difference::Differs("src/lib.rs".to_owned()),
                Difference::Extra("extra.rs".to_owned()),
            ]
        );
        assert!(
            diff(&dir, &files(), &[])
                .unwrap()
                .contains(&Difference::Extra("target/x/y".to_owned()))
        );
        fs::remove_dir_all(&dir).unwrap();
    }
}
