//! The engine's identity (`AET-WO-1802`): its version and the commit it was
//! built from, as `build.rs` recorded them (`src/commit.rs` decides).
//!
//! Only runtime output names the commit: `--version`, `doctor`, the MCP
//! server's info and a campaign run's archive header (`AET-WO-2205` F4),
//! none of which a committed byte or a verdict's hash holds. Every committed
//! or hashed byte (the verdict's `engine_version`, `FAB-NOTES.md`, the KiCad
//! files' `generator_version`, the hwdef header) names [`VERSION`] alone, so
//! a commit never makes a committed package stale.

/// The package version: a release's, or the next one's with `-dev` between
/// releases, so a build from `main` never names a published version it is
/// not.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// The commit's hex name, or `unknown`.
pub const COMMIT: &str = env!("AETHELGARD_COMMIT");
/// `clean`, `dirty` or `unknown`.
pub const TREE: &str = env!("AETHELGARD_TREE");
/// `<version> (commit <sha>, clean|dirty)`, or `<version> (commit unknown)`.
pub const IDENTITY: &str = env!("AETHELGARD_IDENTITY");
