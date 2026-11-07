//! The optional remote mirror's credential boundary (`RUL-06`, `OQ-03`).
//! Nothing is uploaded: the upload waits for a measured volume. What is built
//! is the rule that a missing credential is named once and never stops a run.

use std::ffi::OsString;
use std::io::Write;
use std::path::Path;

use crate::ArchiveError;
use crate::archive::{CommitStage, MANIFEST, MIRROR, MIRROR_TMP, publish};
use crate::cbor::{self, Value};
use crate::event::{MIRROR_MAGIC, VERSION};

/// The three variables, in the order they are read. Names only; no value is
/// ever formatted into anything.
pub const VARIABLES: [&str; 3] = [
    "AETHELGARD_R2_BUCKET",
    "AETHELGARD_R2_ACCESS_KEY_ID",
    "AETHELGARD_R2_SECRET_ACCESS_KEY",
];

pub const NOT_BUILT: &str = "remote mirror not built (RUL-06, OQ-03)";

/// Run the mirror step on a committed run directory. `env` is the lookup
/// (production passes `std::env::var_os`); `log` receives exactly one line
/// when a variable is missing. An unset or empty variable counts as missing.
/// The commit is never touched: its result stands whatever this returns.
pub fn mirror_step(
    dir: &Path,
    env: impl Fn(&str) -> Option<OsString>,
    log: &mut impl Write,
) -> Result<(), ArchiveError> {
    mirror_step_observed(dir, env, log, |_| {})
}

/// As [`mirror_step`], calling `observe` at each [`CommitStage`] its record
/// reaches (`ManifestTempWritten`, then `ManifestRenamed`), for tests that
/// stop it there.
pub fn mirror_step_observed(
    dir: &Path,
    env: impl Fn(&str) -> Option<OsString>,
    log: &mut impl Write,
    observe: impl FnMut(CommitStage),
) -> Result<(), ArchiveError> {
    if !dir.join(MANIFEST).is_file() {
        return Err(ArchiveError::MirrorBeforeCommit);
    }
    if dir.join(MIRROR).exists() {
        return Err(ArchiveError::MirrorExists);
    }
    let missing = VARIABLES
        .iter()
        .find(|name| env(name).is_none_or(|value| value.is_empty()));
    let reason = match missing {
        Some(name) => {
            let line = format!("aethelgard-chaos: remote mirror skipped: {name} is not set\n");
            log.write_all(line.as_bytes())
                .map_err(|e| ArchiveError::io(Path::new("<log>"), e))?;
            format!("{name} is not set")
        }
        None => NOT_BUILT.to_owned(),
    };
    let record = Value::Array(vec![
        Value::Text(MIRROR_MAGIC.to_owned()),
        Value::Uint(VERSION),
        Value::Uint(0),
        Value::Text(reason),
    ]);
    // A crash after the temporary file was written leaves it behind; it is
    // never the record, so it is cleared before ours is written.
    match std::fs::remove_file(dir.join(MIRROR_TMP)) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(ArchiveError::io(&dir.join(MIRROR_TMP), e)),
    }
    publish(dir, MIRROR, &cbor::to_bytes(&record), observe)
}
