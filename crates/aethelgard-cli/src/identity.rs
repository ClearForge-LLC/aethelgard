//! The engine's identity (`AET-WO-1802`, ruled 2026-11-26T22:29Z): its
//! version, and the commit its builder stated.
//!
//! The commit comes only from `AETHELGARD_COMMIT` in the build's environment
//! (`option_env!`, which rustc records as an env dependency, so a change
//! rebuilds). No build script runs (`workspace_shape.rs`,
//! `no_member_has_a_build_script`): nothing reads git or a package file.
//! A stated commit is the builder's claim, never proven from bytes, and the
//! identity says so. Today only the engine image states one, the commit its
//! revision label names; every other build says `commit unknown`.
//!
//! Only runtime output names the commit: `--version`, `doctor` and the MCP
//! server's info. Every committed or hashed byte names [`VERSION`] alone:
//! the verdict's `engine_version`, `FAB-NOTES.md`, the KiCad files'
//! `generator_version`, the hwdef header, and a run's archive header, whose
//! bytes seed the state-sequence hash that the verdict binds (`AET-WO-1802`
//! §7 stop 2).

use std::sync::LazyLock;

/// The package version: a release's, or the next one's with `-dev` between
/// releases, so a build from `main` never names a published version it is
/// not.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The commit the build stated, if it stated one.
pub const COMMIT: Option<&str> = option_env!("AETHELGARD_COMMIT");

// A stated commit that is not 40 lowercase hex digits fails the build.
const _: () = if let Some(c) = COMMIT {
    assert!(
        is_commit(c),
        "AETHELGARD_COMMIT must be 40 lowercase hex digits"
    );
};

/// 40 lowercase hex digits.
pub const fn is_commit(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 40 {
        return false;
    }
    let mut i = 0;
    while i < b.len() {
        if !matches!(b[i], b'0'..=b'9' | b'a'..=b'f') {
            return false;
        }
        i += 1;
    }
    true
}

/// `<version> (commit <sha>, stated by the build)`, or `<version> (commit
/// unknown)`.
pub fn identity_of(version: &str, commit: Option<&str>) -> String {
    match commit {
        Some(c) => format!("{version} (commit {c}, stated by the build)"),
        None => format!("{version} (commit unknown)"),
    }
}

/// This build's identity, as `--version` and the MCP server's info print it.
pub static IDENTITY: LazyLock<String> = LazyLock::new(|| identity_of(VERSION, COMMIT));

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "0123456789abcdef0123456789abcdef01234567";

    #[test]
    fn a_stated_commit_is_named_as_stated_and_none_is_unknown() {
        assert_eq!(
            identity_of("0.3.0", Some(SHA)),
            format!("0.3.0 (commit {SHA}, stated by the build)")
        );
        assert_eq!(identity_of("0.3.0", None), "0.3.0 (commit unknown)");
    }

    /// What the build refuses to state.
    #[test]
    fn only_forty_lowercase_hex_digits_are_a_commit() {
        assert!(is_commit(SHA));
        for bad in [
            "",
            &SHA[..39],
            &format!("{SHA}0"),
            &SHA.to_uppercase(),
            &SHA.replace('a', "g"),
            &format!(" {}", &SHA[1..]),
            "unknown",
        ] {
            assert!(!is_commit(bad), "{bad:?}");
        }
    }
}
