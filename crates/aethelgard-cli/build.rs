//! Records the commit the binary is built from (`AET-WO-1802` (2)) as
//! `AETHELGARD_COMMIT`, `AETHELGARD_TREE` and `AETHELGARD_IDENTITY`;
//! `src/commit.rs` decides it. No committed or hashed byte carries them: the
//! verdict, the fabrication notes, the KiCad files, the hwdef header and a
//! run's archive header name the version alone.
//!
//! It reruns exactly when the binary's own inputs change: in a checkout, the
//! workspace manifest, its lock file, `crates/`, `models/` and the toolchain
//! file; in a package, the package. A shard of CI dates every one of those to
//! the epoch (`ci/shard.sh unpack`), so it never reruns there. `.git` is not
//! watched: a commit that moves no input leaves the binary's sources what
//! they were, and so does its name for them.

#[path = "src/commit.rs"]
mod commit;

use std::path::PathBuf;

fn main() {
    let dir = PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"),
    );
    let version = std::env::var("CARGO_PKG_VERSION").expect("cargo sets CARGO_PKG_VERSION");
    let (found, source) = commit::decide(&dir);
    match source {
        commit::Source::Git(top) => {
            for p in commit::WATCHED {
                let p = top.join(p);
                // A path that does not exist would rerun every build.
                if p.exists() {
                    println!("cargo:rerun-if-changed={}", p.display());
                }
            }
        }
        commit::Source::Package | commit::Source::None => {
            println!("cargo:rerun-if-changed={}", dir.display());
        }
    }
    println!("cargo:rustc-env=AETHELGARD_COMMIT={}", found.sha());
    println!("cargo:rustc-env=AETHELGARD_TREE={}", found.tree());
    println!(
        "cargo:rustc-env=AETHELGARD_IDENTITY={}",
        found.identity(&version)
    );
}
