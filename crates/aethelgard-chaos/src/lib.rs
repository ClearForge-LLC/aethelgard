//! Fault scenarios, the campaign runner, the trace archive, and verdicts; never suppresses a panic or a timeout (`docs/architecture.md` §3).
//!
//! This crate so far holds the trace archive (`RUL-06`, `INV-04`): a
//! deterministic CBOR profile ([`cbor`]), a local append-only archive with a
//! commit protocol ([`archive`]), a replay that refuses what it cannot prove
//! complete ([`replay`]), and the optional remote mirror's credential
//! boundary ([`mirror`]). See `README.md`.

#![forbid(unsafe_code)]

pub mod archive;
pub mod campaign;
pub mod cbor;
pub mod event;
pub mod mirror;
pub mod record;
pub mod replay;
pub mod synthetic;

pub use archive::{CommitStage, Replayed, TraceWriter, replay_dir};
pub use event::{Event, Header, Manifest};

use std::io;

use thiserror::Error;

/// Every way the archive refuses, each its own variant (`INV-06`).
#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error("i/o error on {path}: {source}")]
    Io { path: String, source: io::Error },
    #[error("run directory {0} already exists")]
    RunDirExists(String),
    #[error("header refused by the writer: {0}")]
    InvalidHeader(&'static str),
    #[error("event {index} refused by the writer: {reason}")]
    InvalidEvent { index: u64, reason: &'static str },
    #[error("unexpected entry in the run directory: {0}")]
    UnexpectedEntry(String),
    #[error("uncommitted: there is no manifest.cbor")]
    Uncommitted,
    #[error("the manifest names a trace but trace.cbor is missing")]
    MissingTrace,
    #[error("malformed manifest: {0}")]
    MalformedManifest(String),
    #[error("malformed header: {0}")]
    MalformedHeader(String),
    #[error("unknown format version {found} in {item}")]
    UnknownVersion { item: &'static str, found: u64 },
    #[error("the manifest declares a trace of {declared} bytes, over the {limit}-byte bound")]
    TraceTooLarge { declared: u64, limit: u64 },
    #[error("trace is {actual} bytes, the manifest committed {expected}")]
    Truncated { expected: u64, actual: u64 },
    #[error("trace is {actual} bytes, the manifest committed {expected}")]
    Extended { expected: u64, actual: u64 },
    #[error("trace sha256 does not match the manifest")]
    HashMismatch,
    #[error("trace is not deterministic CBOR: {0}")]
    NotDeterministicCbor(cbor::CborError),
    #[error("event {index} at byte {offset} has unknown kind {kind}")]
    UnknownEventKind {
        index: u64,
        offset: usize,
        kind: u64,
    },
    #[error("event {index} at byte {offset} is malformed: {reason}")]
    MalformedEvent {
        index: u64,
        offset: usize,
        reason: &'static str,
    },
    #[error("event {index} has a timestamp earlier than its predecessor's")]
    TimestampRegressed { index: u64 },
    #[error("trace holds {actual} events, the manifest committed {expected}")]
    EventCountMismatch { expected: u64, actual: u64 },
    #[error("replayed state hash does not match the manifest")]
    ReplayDiverged,
    #[error("the commit is missing, so the mirror step has nothing to mirror")]
    MirrorBeforeCommit,
    #[error("mirror.cbor already exists")]
    MirrorExists,
}

impl ArchiveError {
    /// The variant's name, as `aethelgard replay` reports a refusal.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Io { .. } => "Io",
            Self::RunDirExists(_) => "RunDirExists",
            Self::InvalidHeader(_) => "InvalidHeader",
            Self::InvalidEvent { .. } => "InvalidEvent",
            Self::UnexpectedEntry(_) => "UnexpectedEntry",
            Self::Uncommitted => "Uncommitted",
            Self::MissingTrace => "MissingTrace",
            Self::MalformedManifest(_) => "MalformedManifest",
            Self::MalformedHeader(_) => "MalformedHeader",
            Self::UnknownVersion { .. } => "UnknownVersion",
            Self::TraceTooLarge { .. } => "TraceTooLarge",
            Self::Truncated { .. } => "Truncated",
            Self::Extended { .. } => "Extended",
            Self::HashMismatch => "HashMismatch",
            Self::NotDeterministicCbor(_) => "NotDeterministicCbor",
            Self::UnknownEventKind { .. } => "UnknownEventKind",
            Self::MalformedEvent { .. } => "MalformedEvent",
            Self::TimestampRegressed { .. } => "TimestampRegressed",
            Self::EventCountMismatch { .. } => "EventCountMismatch",
            Self::ReplayDiverged => "ReplayDiverged",
            Self::MirrorBeforeCommit => "MirrorBeforeCommit",
            Self::MirrorExists => "MirrorExists",
        }
    }

    pub(crate) fn io(path: &std::path::Path, source: io::Error) -> Self {
        Self::Io {
            path: path.display().to_string(),
            source,
        }
    }
}
