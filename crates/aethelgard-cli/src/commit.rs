//! The commit this binary is built from (`AET-WO-1802` (2)), decided once by
//! the build script, which includes this file by `#[path]`. The library
//! compiles it only for its own tests, which plant each case.
//!
//! Two sources, in this order, and nothing else (`INV-06`: no default):
//! - **git**, only when its work tree is this crate's: the toplevel joined
//!   with [`PATH_IN_VCS`] is the manifest directory, and the manifest is
//!   tracked there. A crate unpacked under `target/package/` of this
//!   repository, or a registry copy inside some other repository, is not, so
//!   git never names a commit for a tree it was not built from. The state is
//!   `dirty` when `git status` names any change or untracked file, anywhere
//!   in the tree.
//! - **`.cargo_vcs_info.json`**, which `cargo package` writes into a crate's
//!   root: the commit it was packaged from, `dirty` when cargo recorded that
//!   it packaged a dirty tree. Its `path_in_vcs` must be [`PATH_IN_VCS`].
//!
//! Otherwise the commit is `unknown`, and so is the state.
//!
//! git runs with an empty environment, so no `GIT_DIR`, `GIT_WORK_TREE` or
//! `GIT_INDEX_FILE` the build inherited can point it at another repository,
//! and with `--no-optional-locks`, so `git status` never rewrites the index.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Where this crate sits in its repository.
pub const PATH_IN_VCS: &str = "crates/aethelgard-cli";
/// The file `cargo package` writes.
pub const VCS_INFO: &str = ".cargo_vcs_info.json";
/// The largest `.cargo_vcs_info.json` read; cargo's is about 120 bytes.
const MAX_VCS_INFO: u64 = 4096;

/// What the build knows of its tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Commit {
    /// A commit, and whether the tree it was built from differed from it.
    Known { sha: String, dirty: bool },
    /// No source could name one.
    Unknown,
}

/// Where a [`Commit::Known`] came from; it decides what the build watches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// git, in the work tree whose toplevel this is.
    Git(PathBuf),
    /// The crate's `.cargo_vcs_info.json`.
    Package,
    /// Neither.
    None,
}

impl Commit {
    /// The commit, or `unknown`.
    pub fn sha(&self) -> &str {
        match self {
            Commit::Known { sha, .. } => sha,
            Commit::Unknown => "unknown",
        }
    }

    /// `clean`, `dirty` or `unknown`.
    pub fn tree(&self) -> &'static str {
        match self {
            Commit::Known { dirty: false, .. } => "clean",
            Commit::Known { dirty: true, .. } => "dirty",
            Commit::Unknown => "unknown",
        }
    }

    /// The engine's identity, `version` and the commit: `0.3.0 (commit
    /// <sha>, clean)`, `0.3.0 (commit <sha>, dirty)` or `0.3.0 (commit
    /// unknown)`.
    pub fn identity(&self, version: &str) -> String {
        match self {
            Commit::Known { sha, .. } => format!("{version} (commit {sha}, {})", self.tree()),
            Commit::Unknown => format!("{version} (commit unknown)"),
        }
    }
}

/// The commit of the crate whose manifest directory is `dir`.
pub fn decide(dir: &Path) -> (Commit, Source) {
    if let Some((commit, top)) = from_git(dir) {
        return (commit, Source::Git(top));
    }
    match from_package(dir) {
        Some(commit) => (commit, Source::Package),
        None => (Commit::Unknown, Source::None),
    }
}

/// `git` in `dir`, with an empty environment but `PATH`; its stdout when it
/// exits 0.
fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let mut c = Command::new("git");
    c.env_clear();
    if let Some(path) = std::env::var_os("PATH") {
        c.env("PATH", path);
    }
    let out = c
        .env("LC_ALL", "C")
        .arg("--no-optional-locks")
        .arg("-C")
        .arg(dir)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout).ok()
}

/// A commit's name: 40 lowercase hex digits (SHA-1) or 64 (SHA-256).
pub fn is_sha(s: &str) -> bool {
    (s.len() == 40 || s.len() == 64) && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn from_git(dir: &Path) -> Option<(Commit, PathBuf)> {
    let top = PathBuf::from(git(dir, &["rev-parse", "--show-toplevel"])?.trim_end_matches('\n'));
    let top = top.canonicalize().ok()?;
    if top.join(PATH_IN_VCS) != dir.canonicalize().ok()? {
        return None;
    }
    git(dir, &["ls-files", "--error-unmatch", "--", "Cargo.toml"])?;
    let sha = git(
        &top,
        &["rev-parse", "--verify", "--end-of-options", "HEAD^{commit}"],
    )?;
    let sha = sha.trim_end_matches('\n');
    if !is_sha(sha) {
        return None;
    }
    let status = git(
        &top,
        &[
            "status",
            "--porcelain=v1",
            "--untracked-files=normal",
            "--ignore-submodules=none",
        ],
    )?;
    Some((
        Commit::Known {
            sha: sha.to_owned(),
            dirty: !status.is_empty(),
        },
        top,
    ))
}

fn from_package(dir: &Path) -> Option<Commit> {
    let path = dir.join(VCS_INFO);
    let meta = std::fs::symlink_metadata(&path).ok()?;
    if !meta.file_type().is_file() || meta.len() > MAX_VCS_INFO {
        return None;
    }
    parse_vcs_info(&std::fs::read(&path).ok()?)
}

/// `.cargo_vcs_info.json`'s commit: `git.sha1`, and `git.dirty` when cargo
/// wrote it (only `true` or `false`), for [`PATH_IN_VCS`]. Anything else is
/// no commit.
pub fn parse_vcs_info(bytes: &[u8]) -> Option<Commit> {
    let v: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    if v.get("path_in_vcs")?.as_str()? != PATH_IN_VCS {
        return None;
    }
    let git = v.get("git")?.as_object()?;
    let sha = git.get("sha1")?.as_str()?;
    if !is_sha(sha) {
        return None;
    }
    let dirty = match git.get("dirty") {
        None => false,
        Some(d) => d.as_bool()?,
    };
    Some(Commit::Known {
        sha: sha.to_owned(),
        dirty,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

    /// A fresh directory under the system's temporary one.
    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("aet-1802-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d.canonicalize().unwrap()
    }

    fn run_git(dir: &Path, args: &[&str]) {
        let ok = Command::new("git")
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
            ])
            .arg("-C")
            .arg(dir)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success();
        assert!(ok, "git {args:?} in {}", dir.display());
    }

    /// A repository with this crate at [`PATH_IN_VCS`], one commit, clean.
    fn repo(name: &str) -> (PathBuf, PathBuf, String) {
        let top = scratch(name);
        let krate = top.join(PATH_IN_VCS);
        fs::create_dir_all(&krate).unwrap();
        fs::write(krate.join("Cargo.toml"), "[package]\n").unwrap();
        fs::write(top.join("README.md"), "r\n").unwrap();
        run_git(&top, &["init", "-q"]);
        run_git(&top, &["add", "-A"]);
        run_git(&top, &["commit", "-q", "-m", "c"]);
        let sha = git(&top, &["rev-parse", "HEAD"]).unwrap().trim().to_owned();
        (top, krate, sha)
    }

    fn vcs_info(dirty: &str) -> String {
        format!(r#"{{"git":{{"sha1":"{SHA}"{dirty}}},"path_in_vcs":"crates/aethelgard-cli"}}"#)
    }

    #[test]
    fn a_clean_checkout_names_its_commit_clean() {
        let (top, krate, sha) = repo("clean");
        assert!(is_sha(&sha), "{sha}");
        let (c, s) = decide(&krate);
        assert_eq!(
            c,
            Commit::Known {
                sha: sha.clone(),
                dirty: false
            }
        );
        assert_eq!(s, Source::Git(top.clone()));
        assert_eq!(c.identity("0.3.0"), format!("0.3.0 (commit {sha}, clean)"));
        fs::remove_dir_all(top).unwrap();
    }

    /// A tracked file changed, one staged, one untracked file, one change
    /// outside the crate: each is `dirty`, with the commit it differs from.
    #[test]
    fn a_dirty_checkout_says_dirty() {
        let (top, krate, sha) = repo("dirty");
        let dirty = Commit::Known {
            sha: sha.clone(),
            dirty: true,
        };
        fs::write(krate.join("Cargo.toml"), "[package]\nx = 1\n").unwrap();
        assert_eq!(decide(&krate).0, dirty, "a tracked file changed");
        run_git(&top, &["add", "-A"]);
        assert_eq!(decide(&krate).0, dirty, "a change staged");
        run_git(&top, &["reset", "-q", "--hard"]);
        assert_eq!(
            decide(&krate).0,
            Commit::Known {
                sha: sha.clone(),
                dirty: false
            }
        );
        fs::write(krate.join("new.rs"), "").unwrap();
        assert_eq!(decide(&krate).0, dirty, "an untracked file");
        fs::remove_file(krate.join("new.rs")).unwrap();
        fs::write(top.join("README.md"), "changed\n").unwrap();
        assert_eq!(decide(&krate).0, dirty, "a change outside the crate");
        assert_eq!(
            dirty.identity("0.3.0"),
            format!("0.3.0 (commit {sha}, dirty)")
        );
        fs::remove_dir_all(top).unwrap();
    }

    /// No repository and no `.cargo_vcs_info.json`: `unknown`, never a
    /// default commit.
    #[test]
    fn no_source_is_unknown() {
        let d = scratch("unknown");
        let krate = d.join("aethelgard-cli-0.3.0");
        fs::create_dir_all(&krate).unwrap();
        assert_eq!(decide(&krate), (Commit::Unknown, Source::None));
        assert_eq!(Commit::Unknown.identity("0.3.0"), "0.3.0 (commit unknown)");
        assert_eq!(
            (Commit::Unknown.sha(), Commit::Unknown.tree()),
            ("unknown", "unknown")
        );
        fs::remove_dir_all(d).unwrap();
    }

    /// A packaged crate names the commit it was packaged from, and `dirty`
    /// only as cargo recorded it.
    #[test]
    fn a_package_names_the_commit_it_was_packaged_from() {
        let d = scratch("package");
        let krate = d.join("aethelgard-cli-0.3.0");
        fs::create_dir_all(&krate).unwrap();
        for (field, dirty) in [
            ("", false),
            (r#","dirty":false"#, false),
            (r#","dirty":true"#, true),
        ] {
            fs::write(krate.join(VCS_INFO), vcs_info(field)).unwrap();
            assert_eq!(
                decide(&krate),
                (
                    Commit::Known {
                        sha: SHA.to_owned(),
                        dirty
                    },
                    Source::Package
                ),
                "{field}"
            );
        }
        fs::remove_dir_all(d).unwrap();
    }

    /// A crate unpacked inside this repository but not at its own path (as
    /// `cargo package` verifies one under `target/package/`), or a registry
    /// copy inside another repository: git is not asked, so the commit is
    /// the package's, and with no package file it is `unknown`.
    #[test]
    fn a_crate_elsewhere_in_a_repository_is_not_that_repository_commit() {
        let (top, _krate, sha) = repo("nested");
        let copy = top.join("target/package/aethelgard-cli-0.3.0");
        fs::create_dir_all(&copy).unwrap();
        fs::write(copy.join("Cargo.toml"), "[package]\n").unwrap();
        assert_eq!(decide(&copy), (Commit::Unknown, Source::None));
        fs::write(copy.join(VCS_INFO), vcs_info("")).unwrap();
        let (c, s) = decide(&copy);
        assert_eq!((c.sha(), s), (SHA, Source::Package));
        assert_ne!(c.sha(), sha);
        fs::remove_dir_all(top).unwrap();
    }

    /// The crate's directory in a repository where its manifest is not
    /// tracked: not git's.
    #[test]
    fn an_untracked_crate_is_not_the_repository_commit() {
        let top = scratch("untracked");
        let krate = top.join(PATH_IN_VCS);
        fs::create_dir_all(&krate).unwrap();
        fs::write(top.join("README.md"), "r\n").unwrap();
        run_git(&top, &["init", "-q"]);
        run_git(&top, &["add", "README.md"]);
        run_git(&top, &["commit", "-q", "-m", "c"]);
        fs::write(krate.join("Cargo.toml"), "[package]\n").unwrap();
        assert_eq!(decide(&krate), (Commit::Unknown, Source::None));
        fs::remove_dir_all(top).unwrap();
    }

    /// A `.cargo_vcs_info.json` planted in a checkout does not outrank git:
    /// the checkout's own commit, and dirty, since the file is untracked.
    #[test]
    fn a_planted_package_file_in_a_checkout_is_dirty() {
        let (top, krate, sha) = repo("planted");
        fs::write(krate.join(VCS_INFO), vcs_info("")).unwrap();
        let (c, s) = decide(&krate);
        assert_eq!(c, Commit::Known { sha, dirty: true });
        assert_eq!(s, Source::Git(top.clone()));
        fs::remove_dir_all(top).unwrap();
    }

    /// Every `.cargo_vcs_info.json` that is not cargo's shape names no
    /// commit; a symlink or an oversized file is not read.
    #[test]
    fn a_malformed_package_file_names_no_commit() {
        for bad in [
            String::new(),
            "{".to_owned(),
            r#"{"git":{"sha1":"0123"},"path_in_vcs":"crates/aethelgard-cli"}"#.to_owned(),
            vcs_info("").replace(SHA, &SHA.to_uppercase()),
            vcs_info("").replace("crates/aethelgard-cli", "crates/aethelgard-ir"),
            vcs_info("").replace(r#","path_in_vcs":"crates/aethelgard-cli""#, ""),
            vcs_info(r#","dirty":"yes""#),
            vcs_info(r#","dirty":1"#),
            r#"{"hg":{"sha1":"0123456789abcdef0123456789abcdef01234567"},"path_in_vcs":"crates/aethelgard-cli"}"#.to_owned(),
        ] {
            assert_eq!(parse_vcs_info(bad.as_bytes()), None, "{bad}");
        }
        let d = scratch("malformed");
        let krate = d.join("k");
        fs::create_dir_all(&krate).unwrap();
        fs::write(d.join("real.json"), vcs_info("")).unwrap();
        std::os::unix::fs::symlink(d.join("real.json"), krate.join(VCS_INFO)).unwrap();
        assert_eq!(decide(&krate), (Commit::Unknown, Source::None), "a symlink");
        fs::remove_file(krate.join(VCS_INFO)).unwrap();
        let big = vcs_info(&format!(r#","x":"{}""#, " ".repeat(MAX_VCS_INFO as usize)));
        fs::write(krate.join(VCS_INFO), big).unwrap();
        assert_eq!(decide(&krate), (Commit::Unknown, Source::None), "oversized");
        fs::remove_dir_all(d).unwrap();
    }
}
