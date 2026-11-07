//! The run directory: the writer's commit protocol and the reader's checks
//! (`README.md`, "Commit protocol" and "Refusals").

use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::ArchiveError;
use crate::cbor;
use crate::event::{Event, Fault, Header, Manifest};
use crate::replay::{Replayer, State};

pub const TRACE: &str = "trace.cbor";
pub const MANIFEST: &str = "manifest.cbor";
pub const MIRROR: &str = "mirror.cbor";

/// Where a commit is, for tests that stop the writer there. The mirror's
/// record goes through the same steps and reports the same two last stages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitStage {
    /// The trace is flushed and `sync_all`ed; no manifest exists yet.
    TraceSynced,
    /// `manifest.cbor.tmp` is written and synced; not yet renamed.
    ManifestTempWritten,
    /// `manifest.cbor` exists; the directory is not yet synced.
    ManifestRenamed,
}

fn sync_dir(dir: &Path) -> Result<(), ArchiveError> {
    let handle = File::open(dir).map_err(|e| ArchiveError::io(dir, e))?;
    handle.sync_all().map_err(|e| ArchiveError::io(dir, e))
}

/// Write `name` in `dir` by temp file, `sync_all`, rename, directory sync.
/// `between` is told each stage that is reached before the directory sync.
pub(crate) fn publish(
    dir: &Path,
    name: &str,
    bytes: &[u8],
    mut between: impl FnMut(CommitStage),
) -> Result<(), ArchiveError> {
    let tmp = dir.join(format!("{name}.tmp"));
    let target = dir.join(name);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .map_err(|e| ArchiveError::io(&tmp, e))?;
    file.write_all(bytes)
        .map_err(|e| ArchiveError::io(&tmp, e))?;
    file.sync_all().map_err(|e| ArchiveError::io(&tmp, e))?;
    drop(file);
    between(CommitStage::ManifestTempWritten);
    fs::rename(&tmp, &target).map_err(|e| ArchiveError::io(&target, e))?;
    between(CommitStage::ManifestRenamed);
    sync_dir(dir)
}

/// The archive's single writer for one run.
pub struct TraceWriter {
    dir: PathBuf,
    trace: BufWriter<File>,
    trace_path: PathBuf,
    hasher: Sha256,
    bytes: u64,
    replayer: Replayer,
    scratch: Vec<u8>,
}

impl TraceWriter {
    /// Create `dir` (which must not exist) and write the header.
    pub fn create(dir: &Path, header: &Header) -> Result<Self, ArchiveError> {
        Self::create_with_capacity(dir, header, 8 * 1024)
    }

    /// As [`create`](Self::create), with the write buffer's size chosen.
    pub fn create_with_capacity(
        dir: &Path,
        header: &Header,
        capacity: usize,
    ) -> Result<Self, ArchiveError> {
        let header_bytes = cbor::to_bytes(&header.to_value());
        // The writer refuses a header its own reader would refuse.
        let reread = cbor::decode_exact(&header_bytes)
            .map_err(|_| ArchiveError::InvalidHeader("header is outside the profile's bounds"))?;
        if let Err(Fault::Malformed(reason)) = Header::from_value(&reread) {
            return Err(ArchiveError::InvalidHeader(reason));
        }
        match fs::create_dir(dir) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(ArchiveError::RunDirExists(dir.display().to_string()));
            }
            Err(e) => return Err(ArchiveError::io(dir, e)),
        }
        let trace_path = dir.join(TRACE);
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&trace_path)
            .map_err(|e| ArchiveError::io(&trace_path, e))?;
        let mut writer = Self {
            dir: dir.to_path_buf(),
            trace: BufWriter::with_capacity(capacity, file),
            trace_path,
            hasher: Sha256::new(),
            bytes: 0,
            replayer: Replayer::new(&header_bytes),
            scratch: Vec::new(),
        };
        writer.put(&header_bytes)?;
        Ok(writer)
    }

    fn put(&mut self, bytes: &[u8]) -> Result<(), ArchiveError> {
        self.trace
            .write_all(bytes)
            .map_err(|e| ArchiveError::io(&self.trace_path, e))?;
        self.hasher.update(bytes);
        self.bytes += bytes.len() as u64;
        Ok(())
    }

    /// Append one event. Refuses what the reader would refuse, and a
    /// timestamp earlier than the last one.
    pub fn append(&mut self, event: &Event) -> Result<(), ArchiveError> {
        let index = self.replayer.event_count();
        event
            .check()
            .map_err(|reason| ArchiveError::InvalidEvent { index, reason })?;
        self.replayer.apply(event)?;
        let mut scratch = std::mem::take(&mut self.scratch);
        scratch.clear();
        cbor::encode(&event.to_value(), &mut scratch);
        let result = self.put(&scratch);
        self.scratch = scratch;
        result
    }

    /// Finish the run: sync the trace, then publish the manifest.
    pub fn commit(self) -> Result<Manifest, ArchiveError> {
        self.commit_observed(|_| {})
    }

    /// As [`commit`](Self::commit), calling `observe` at each [`CommitStage`].
    pub fn commit_observed(
        mut self,
        mut observe: impl FnMut(CommitStage),
    ) -> Result<Manifest, ArchiveError> {
        self.trace
            .flush()
            .map_err(|e| ArchiveError::io(&self.trace_path, e))?;
        self.trace
            .get_ref()
            .sync_all()
            .map_err(|e| ArchiveError::io(&self.trace_path, e))?;
        observe(CommitStage::TraceSynced);
        let manifest = Manifest {
            trace_bytes: self.bytes,
            trace_sha256: self.hasher.finalize().into(),
            event_count: self.replayer.event_count(),
            state_hash: self.replayer.state_hash(),
        };
        let bytes = cbor::to_bytes(&manifest.to_value());
        publish(&self.dir, MANIFEST, &bytes, observe)?;
        Ok(manifest)
    }
}

/// What a successful replay proved.
#[derive(Debug, PartialEq, Eq)]
pub struct Replayed {
    pub header: Header,
    pub manifest: Manifest,
    pub state: State,
}

fn read_all(path: &Path) -> Result<Vec<u8>, ArchiveError> {
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|mut f| f.read_to_end(&mut bytes))
        .map_err(|e| ArchiveError::io(path, e))?;
    Ok(bytes)
}

/// Check the directory's shape: a real directory holding only regular files
/// named `trace.cbor`, `manifest.cbor` and optionally `mirror.cbor`.
fn check_entries(dir: &Path) -> Result<(), ArchiveError> {
    let meta = fs::symlink_metadata(dir).map_err(|e| ArchiveError::io(dir, e))?;
    if !meta.file_type().is_dir() {
        return Err(ArchiveError::UnexpectedEntry(format!(
            "{} is not a real directory",
            dir.display()
        )));
    }
    let mut bad = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| ArchiveError::io(dir, e))? {
        let entry = entry.map_err(|e| ArchiveError::io(dir, e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let regular = entry.file_type().is_ok_and(|t| t.is_file());
        if !(regular && [TRACE, MANIFEST, MIRROR].contains(&name.as_str())) {
            bad.push(name);
        }
    }
    // Sorted, so the same directory always names the same entry.
    bad.sort();
    match bad.into_iter().next() {
        Some(name) => Err(ArchiveError::UnexpectedEntry(name)),
        None => Ok(()),
    }
}

/// Replay a run directory, refusing anything it cannot prove complete.
pub fn replay_dir(dir: &Path) -> Result<Replayed, ArchiveError> {
    // Components drop a trailing separator, which would make `lstat` follow a
    // symlinked run directory.
    let dir: PathBuf = dir.components().collect();
    let dir = dir.as_path();
    check_entries(dir)?;
    let manifest_path = dir.join(MANIFEST);
    if !manifest_path.exists() {
        return Err(ArchiveError::Uncommitted);
    }
    let manifest_bytes = read_all(&manifest_path)?;
    let value = cbor::decode_exact(&manifest_bytes)
        .map_err(|e| ArchiveError::MalformedManifest(e.to_string()))?;
    let manifest = Manifest::from_value(&value).map_err(|fault| match fault {
        Fault::Version(found) => ArchiveError::UnknownVersion {
            item: "manifest",
            found,
        },
        Fault::Malformed(reason) => ArchiveError::MalformedManifest(reason.to_owned()),
        Fault::UnknownKind(_) => ArchiveError::MalformedManifest("unexpected kind".to_owned()),
    })?;

    let trace_path = dir.join(TRACE);
    let actual = match fs::metadata(&trace_path) {
        Ok(meta) => meta.len(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(ArchiveError::MissingTrace);
        }
        Err(e) => return Err(ArchiveError::io(&trace_path, e)),
    };
    // The length is checked before the file is read, so a trace the manifest
    // does not describe is never loaded.
    if actual < manifest.trace_bytes {
        return Err(ArchiveError::Truncated {
            expected: manifest.trace_bytes,
            actual,
        });
    }
    if actual > manifest.trace_bytes {
        return Err(ArchiveError::Extended {
            expected: manifest.trace_bytes,
            actual,
        });
    }
    let trace = read_all(&trace_path)?;
    if <[u8; 32]>::from(Sha256::digest(&trace)) != manifest.trace_sha256 {
        return Err(ArchiveError::HashMismatch);
    }

    let (header_value, mut offset) =
        cbor::decode_prefix(&trace, 0).map_err(ArchiveError::NotDeterministicCbor)?;
    let header = Header::from_value(&header_value).map_err(|fault| match fault {
        Fault::Version(found) => ArchiveError::UnknownVersion {
            item: "header",
            found,
        },
        Fault::Malformed(reason) => ArchiveError::MalformedHeader(reason.to_owned()),
        Fault::UnknownKind(_) => ArchiveError::MalformedHeader("unexpected kind".to_owned()),
    })?;
    let mut replayer = Replayer::new(&trace[..offset]);
    while offset < trace.len() {
        let index = replayer.event_count();
        let (value, end) =
            cbor::decode_prefix(&trace, offset).map_err(ArchiveError::NotDeterministicCbor)?;
        let event = Event::from_value(&value).map_err(|fault| match fault {
            Fault::UnknownKind(kind) => ArchiveError::UnknownEventKind {
                index,
                offset,
                kind,
            },
            Fault::Malformed(reason) => ArchiveError::MalformedEvent {
                index,
                offset,
                reason,
            },
            Fault::Version(_) => ArchiveError::MalformedEvent {
                index,
                offset,
                reason: "unexpected version",
            },
        })?;
        replayer.apply(&event)?;
        offset = end;
    }
    if replayer.event_count() != manifest.event_count {
        return Err(ArchiveError::EventCountMismatch {
            expected: manifest.event_count,
            actual: replayer.event_count(),
        });
    }
    if replayer.state_hash() != manifest.state_hash {
        return Err(ArchiveError::ReplayDiverged);
    }
    Ok(Replayed {
        header,
        manifest,
        state: replayer.state().clone(),
    })
}
