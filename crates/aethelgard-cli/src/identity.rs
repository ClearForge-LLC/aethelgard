//! The engine's identity (`AET-WO-1802`): its version and the commit it was
//! built from, as `build.rs` recorded them (`src/commit.rs` decides).
//!
//! Only runtime output names the commit: `--version`, `doctor` and the MCP
//! server's info, none of which a committed byte or a verdict's hash holds.
//! Every committed or hashed byte names [`VERSION`] alone, so a commit never
//! makes a committed package stale: the verdict's `engine_version`,
//! `FAB-NOTES.md`, the KiCad files' `generator_version`, the hwdef header,
//! and a run's archive header, whose bytes seed the state-sequence hash that
//! the verdict binds (`AET-WO-1802` §7 stop 2, so `AET-WO-2205` F4 stays
//! open).

/// The package version: a release's, or the next one's with `-dev` between
/// releases, so a build from `main` never names a published version it is
/// not.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// The commit's hex name, or `unknown`.
pub const COMMIT: &str = env!("AETHELGARD_COMMIT");
/// `clean` or `dirty` (a checkout), `packaged` or `dirty` (a package), or
/// `unknown`.
pub const TREE: &str = env!("AETHELGARD_TREE");
/// `<version> (commit <sha>, clean|dirty|packaged)`, or `<version> (commit
/// unknown)`.
pub const IDENTITY: &str = env!("AETHELGARD_IDENTITY");
