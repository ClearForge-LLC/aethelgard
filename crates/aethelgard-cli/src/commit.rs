//! The commit this binary is built from (`AET-WO-1802` (2)), decided once by
//! the build script, which includes this file by `#[path]`. The library
//! compiles it only for its own tests, which plant each case.
//!
//! Two sources, in this order, and nothing else (`INV-06`: no default):
//! - **git**, only when its work tree is this crate's: the toplevel joined
//!   with [`PATH_IN_VCS`] is the manifest directory, and the manifest is
//!   tracked there. A crate unpacked under `target/package/` of this
//!   repository, or a registry copy inside some other repository, is not, so
//!   git never names a commit for a tree it was not built from; nor is a
//!   crate whose `.cargo_vcs_info.json` is tracked (a package committed into
//!   a repository is that package). The state is `dirty` when `git status`
//!   names any change or untracked file in the binary's inputs, [`WATCHED`],
//!   the paths the build script reruns on, or when an index entry there is
//!   marked skip-worktree or assume-unchanged (which hide a change from
//!   `git status`); `clean` otherwise.
//! - **`.cargo_vcs_info.json`**, which `cargo package` writes into a crate's
//!   root: the commit it was packaged from. Its `path_in_vcs` must be
//!   [`PATH_IN_VCS`]. Cargo records `dirty` for this package's own files
//!   only, never its siblings', and nothing checks the unpacked files, so the
//!   state is `dirty` when cargo recorded it and otherwise `packaged`, never
//!   `clean`.
//!
//! Otherwise the commit is `unknown`, and so is the state.
//!
//! git runs in the build's environment less every `GIT_*` variable, so no
//! `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE` or `GIT_CONFIG_*` the build
//! inherited can point it at another repository or another configuration.
//! `HOME` stays, so the builder's global git configuration applies:
//! `core.excludesFile` keeps a globally ignored file (an editor's swap file)
//! from reading as `dirty`, and `safe.directory` admits a checkout another
//! user owns, which is otherwise `unknown`. That configuration could also
//! hide a change (an `excludesFile` naming a source) or forge the state (a
//! `core.fsmonitor`, as could a repository's own `.git/config` or another
//! `git` on `PATH`); those are the builder's own machine, and out of reach
//! of any build-time record. git also runs with `--no-replace-objects`, so a replace ref cannot rename `HEAD`; and
//! with `--no-optional-locks`, so `git status` never rewrites the index.
//!
//! The record is of the sources when the script last ran. A source edited
//! while cargo is still compiling, after the script ran, is in the binary
//! but not in the record; the next build reruns the script (`AET-WO-1802`
//! adversarial F4, inherent to any build-time record).

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Where this crate sits in its repository.
pub const PATH_IN_VCS: &str = "crates/aethelgard-cli";
/// The file `cargo package` writes.
pub const VCS_INFO: &str = ".cargo_vcs_info.json";
/// The largest `.cargo_vcs_info.json` read; cargo's is about 120 bytes.
const MAX_VCS_INFO: u64 = 4096;
/// The binary's inputs in a checkout, from its toplevel: what the build
/// script reruns on, and what `dirty` is judged over.
pub const WATCHED: [&str; 5] = [
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "crates",
    "models",
];

/// What the build knows of its sources against the commit it names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tree {
    /// A checkout whose inputs are the commit's.
    Clean,
    /// A checkout whose inputs differ from it, or a package cargo recorded
    /// as packaged from a dirty tree.
    Dirty,
    /// A package: the commit it was packaged from, its files not checked.
    Packaged,
}

/// What the build knows of its commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Commit {
    /// A commit, and the sources against it.
    Known { sha: String, tree: Tree },
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

    /// `clean`, `dirty`, `packaged` or `unknown`.
    pub fn tree(&self) -> &'static str {
        match self {
            Commit::Known {
                tree: Tree::Clean, ..
            } => "clean",
            Commit::Known {
                tree: Tree::Dirty, ..
            } => "dirty",
            Commit::Known {
                tree: Tree::Packaged,
                ..
            } => "packaged",
            Commit::Unknown => "unknown",
        }
    }

    /// The engine's identity, `version` and the commit: `0.3.0 (commit
    /// <sha>, clean|dirty|packaged)` or `0.3.0 (commit unknown)`.
    pub fn identity(&self, version: &str) -> String {
        match self {
            Commit::Known { sha, .. } => format!("{version} (commit {sha}, {})", self.tree()),
            Commit::Unknown => format!("{version} (commit unknown)"),
        }
    }
}

/// The commit of the crate whose manifest directory is `dir`, git run in
/// this process's environment.
pub fn decide(dir: &Path) -> (Commit, Source) {
    decide_with(dir, &std::env::vars_os().collect::<Vec<_>>())
}

/// [`decide`], git run in `env` less every `GIT_*` variable.
pub fn decide_with(dir: &Path, env: &[(OsString, OsString)]) -> (Commit, Source) {
    if let Some((commit, top)) = from_git(dir, env) {
        return (commit, Source::Git(top));
    }
    match from_package(dir) {
        Some(commit) => (commit, Source::Package),
        None => (Commit::Unknown, Source::None),
    }
}

/// `git` in `dir`, in `env` less every `GIT_*` variable (`GIT_DIR`,
/// `GIT_WORK_TREE`, `GIT_INDEX_FILE`, `GIT_CONFIG_*`, …); its stdout when it
/// exits 0.
fn git(dir: &Path, args: &[&str], env: &[(OsString, OsString)]) -> Option<String> {
    let mut c = Command::new("git");
    c.env_clear();
    for (k, v) in env {
        if !k.as_encoded_bytes().starts_with(b"GIT_") {
            c.env(k, v);
        }
    }
    let out = c
        .env("LC_ALL", "C")
        .arg("--no-optional-locks")
        .arg("--no-replace-objects")
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

fn from_git(dir: &Path, env: &[(OsString, OsString)]) -> Option<(Commit, PathBuf)> {
    let git = |dir: &Path, args: &[&str]| git(dir, args, env);
    let top = PathBuf::from(git(dir, &["rev-parse", "--show-toplevel"])?.trim_end_matches('\n'));
    let top = top.canonicalize().ok()?;
    if top.join(PATH_IN_VCS) != dir.canonicalize().ok()? {
        return None;
    }
    git(dir, &["ls-files", "--error-unmatch", "--", "Cargo.toml"])?;
    // A package committed into a repository is that package (F6).
    if git(dir, &["ls-files", "--error-unmatch", "--", VCS_INFO]).is_some() {
        return None;
    }
    let sha = git(
        &top,
        &["rev-parse", "--verify", "--end-of-options", "HEAD^{commit}"],
    )?;
    let sha = sha.trim_end_matches('\n');
    if !is_sha(sha) {
        return None;
    }
    let mut args = vec![
        "status",
        "--porcelain=v1",
        "--untracked-files=normal",
        "--ignore-submodules=none",
        "--",
    ];
    args.extend(WATCHED);
    let status = git(&top, &args)?;
    // `ls-files -v` tags an assume-unchanged entry in lower case and a
    // skip-worktree one `S`: either hides a change from `git status` (F3).
    let mut args = vec!["ls-files", "-v", "--"];
    args.extend(WATCHED);
    let hidden = git(&top, &args)?.lines().any(|l| {
        l.bytes()
            .next()
            .is_some_and(|t| t == b'S' || t.is_ascii_lowercase())
    });
    let tree = if status.is_empty() && !hidden {
        Tree::Clean
    } else {
        Tree::Dirty
    };
    Some((
        Commit::Known {
            sha: sha.to_owned(),
            tree,
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

/// `.cargo_vcs_info.json`'s commit: `git.sha1`, `dirty` when cargo wrote
/// `git.dirty` true and `packaged` otherwise (`dirty` only `true` or
/// `false`), for [`PATH_IN_VCS`]. Anything else is no commit.
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
    let tree = match git.get("dirty") {
        None => Tree::Packaged,
        Some(d) if d.as_bool()? => Tree::Dirty,
        Some(_) => Tree::Packaged,
    };
    Some(Commit::Known {
        sha: sha.to_owned(),
        tree,
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

    fn head(top: &Path) -> String {
        let out = Command::new("git")
            .arg("-C")
            .arg(top)
            .args(["rev-parse", "HEAD"])
            .output()
            .unwrap();
        String::from_utf8(out.stdout).unwrap().trim().to_owned()
    }

    /// A repository with this crate at [`PATH_IN_VCS`], one commit, clean.
    fn repo(name: &str) -> (PathBuf, PathBuf, String) {
        let top = scratch(name);
        let krate = top.join(PATH_IN_VCS);
        fs::create_dir_all(&krate).unwrap();
        fs::write(krate.join("Cargo.toml"), "[package]\n").unwrap();
        fs::write(top.join("notes.txt"), "r\n").unwrap();
        run_git(&top, &["init", "-q"]);
        run_git(&top, &["add", "-A"]);
        run_git(&top, &["commit", "-q", "-m", "c"]);
        let sha = head(&top);
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
                tree: Tree::Clean
            }
        );
        assert_eq!(s, Source::Git(top.clone()));
        assert_eq!(c.identity("0.3.0"), format!("0.3.0 (commit {sha}, clean)"));
        fs::remove_dir_all(top).unwrap();
    }

    /// A tracked file changed, one staged, one untracked file, a change in
    /// another crate, an entry hidden by skip-worktree or assume-unchanged:
    /// each is `dirty`, with the commit it differs from. A change outside
    /// the binary's inputs (a document at the root) is not (F5).
    #[test]
    fn a_dirty_checkout_says_dirty() {
        let (top, krate, sha) = repo("dirty");
        let dirty = Commit::Known {
            sha: sha.clone(),
            tree: Tree::Dirty,
        };
        let clean = Commit::Known {
            sha: sha.clone(),
            tree: Tree::Clean,
        };
        fs::write(krate.join("Cargo.toml"), "[package]\nx = 1\n").unwrap();
        assert_eq!(decide(&krate).0, dirty, "a tracked file changed");
        run_git(&top, &["add", "-A"]);
        assert_eq!(decide(&krate).0, dirty, "a change staged");
        run_git(&top, &["reset", "-q", "--hard"]);
        assert_eq!(decide(&krate).0, clean);
        fs::write(krate.join("new.rs"), "").unwrap();
        assert_eq!(decide(&krate).0, dirty, "an untracked file");
        fs::remove_file(krate.join("new.rs")).unwrap();
        fs::write(top.join("crates/aethelgard-ir.rs"), "").unwrap();
        assert_eq!(decide(&krate).0, dirty, "another crate's file");
        fs::remove_file(top.join("crates/aethelgard-ir.rs")).unwrap();
        fs::write(top.join("notes.txt"), "changed\n").unwrap();
        assert_eq!(decide(&krate).0, clean, "a document outside the inputs");
        run_git(&top, &["checkout", "-q", "--", "notes.txt"]);
        for flag in ["--skip-worktree", "--assume-unchanged"] {
            run_git(
                &top,
                &["update-index", flag, &format!("{PATH_IN_VCS}/Cargo.toml")],
            );
            fs::write(krate.join("Cargo.toml"), "[package]\nhidden = 1\n").unwrap();
            assert_eq!(decide(&krate).0, dirty, "{flag}");
            run_git(
                &top,
                &[
                    "update-index",
                    &flag.replace("--", "--no-"),
                    &format!("{PATH_IN_VCS}/Cargo.toml"),
                ],
            );
            run_git(&top, &["checkout", "-q", "--", "."]);
            assert_eq!(decide(&krate).0, clean, "{flag} undone");
        }
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

    /// A packaged crate names the commit it was packaged from, `packaged`,
    /// never `clean` (F2), and `dirty` only as cargo recorded it.
    #[test]
    fn a_package_names_the_commit_it_was_packaged_from() {
        let d = scratch("package");
        let krate = d.join("aethelgard-cli-0.3.0");
        fs::create_dir_all(&krate).unwrap();
        for (field, tree) in [
            ("", Tree::Packaged),
            (r#","dirty":false"#, Tree::Packaged),
            (r#","dirty":true"#, Tree::Dirty),
        ] {
            fs::write(krate.join(VCS_INFO), vcs_info(field)).unwrap();
            assert_eq!(
                decide(&krate),
                (
                    Commit::Known {
                        sha: SHA.to_owned(),
                        tree
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
        fs::write(top.join("notes.txt"), "r\n").unwrap();
        run_git(&top, &["init", "-q"]);
        run_git(&top, &["add", "notes.txt"]);
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
        assert_eq!(
            c,
            Commit::Known {
                sha,
                tree: Tree::Dirty
            }
        );
        assert_eq!(s, Source::Git(top.clone()));
        fs::remove_dir_all(top).unwrap();
    }

    /// A package committed into a repository at this crate's path is that
    /// package, never the repository's commit (F6).
    #[test]
    fn a_tracked_package_file_is_the_package() {
        let (top, krate, sha) = repo("tracked");
        fs::write(krate.join(VCS_INFO), vcs_info("")).unwrap();
        run_git(&top, &["add", "-A"]);
        run_git(&top, &["commit", "-q", "-m", "pkg"]);
        let (c, s) = decide(&krate);
        assert_eq!((c.sha(), c.tree(), s), (SHA, "packaged", Source::Package));
        assert_ne!(c.sha(), sha);
        fs::remove_dir_all(top).unwrap();
    }

    /// A replace ref cannot hide a difference (F3): `HEAD` names A while
    /// the index and the work tree are B, and `git replace A B` makes A's
    /// tree read as B's. git is asked with replace refs off, so it is A,
    /// dirty.
    #[test]
    fn a_replace_ref_cannot_hide_a_difference() {
        let (top, krate, a) = repo("replace");
        fs::write(krate.join("Cargo.toml"), "[package]\nb = 1\n").unwrap();
        run_git(&top, &["commit", "-q", "-am", "b"]);
        let b = head(&top);
        run_git(&top, &["update-ref", "HEAD", &a]);
        run_git(&top, &["replace", &a, &b]);
        assert_eq!(
            decide(&krate).0,
            Commit::Known {
                sha: a,
                tree: Tree::Dirty
            }
        );
        fs::remove_dir_all(top).unwrap();
    }

    /// The builder's global git configuration applies, through `HOME`
    /// (`AET-WO-1802` review 21:56Z): a globally ignored swap file is not
    /// `dirty`. Every `GIT_*` variable is dropped, so a `GIT_DIR` naming
    /// another repository changes nothing.
    #[test]
    fn the_global_config_applies_and_git_variables_do_not() {
        let (top, krate, sha) = repo("global");
        let home = scratch("global-home");
        fs::write(home.join("ignore"), "*.swp\n").unwrap();
        fs::write(
            home.join(".gitconfig"),
            format!(
                "[core]\n\texcludesFile = {}\n",
                home.join("ignore").display()
            ),
        )
        .unwrap();
        fs::write(krate.join(".main.rs.swp"), "").unwrap();
        let path = std::env::var_os("PATH").unwrap_or_default();
        let with_home = vec![
            ("PATH".into(), path.clone()),
            ("HOME".into(), home.clone().into()),
        ];
        let tree = |env: &[(OsString, OsString)]| decide_with(&krate, env).0.tree();
        assert_eq!(
            tree(&with_home),
            "clean",
            "ignored by the global excludesFile"
        );
        let no_home = vec![
            ("PATH".into(), path.clone()),
            ("HOME".into(), scratch("no-home").into()),
        ];
        assert_eq!(
            tree(&no_home),
            "dirty",
            "the same file with no global config"
        );
        let (other, _, _) = repo("global-other");
        fs::write(other.join("notes.txt"), "other\n").unwrap();
        run_git(&other, &["commit", "-q", "-am", "other"]);
        let other_sha = head(&other);
        let mut planted = with_home.clone();
        planted.push(("GIT_DIR".into(), other.join(".git").into()));
        planted.push(("GIT_WORK_TREE".into(), other.clone().into()));
        planted.push(("GIT_CONFIG_GLOBAL".into(), "/dev/null".into()));
        let (c, _) = decide_with(&krate, &planted);
        assert_eq!((c.sha(), c.tree()), (sha.as_str(), "clean"));
        assert_ne!(c.sha(), other_sha);
        for d in [top, home, other] {
            fs::remove_dir_all(d).unwrap();
        }
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
