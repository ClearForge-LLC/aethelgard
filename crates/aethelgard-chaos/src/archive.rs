//! The run directory: the writer's commit protocol and the reader's checks
//! (`README.md`, "Commit protocol" and "Refusals").

use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use rustix::fd::OwnedFd;
use rustix::fs::{AtFlags, Dir, FileType, Mode, OFlags, openat, statat};
use rustix::io::Errno;
use sha2::{Digest, Sha256};

use crate::ArchiveError;
use crate::cbor;
use crate::event::{Event, Fault, Header, Manifest};
use crate::replay::{Replayer, State};

pub const TRACE: &str = "trace.cbor";
pub const MANIFEST: &str = "manifest.cbor";
pub const MIRROR: &str = "mirror.cbor";
/// The mirror step's temporary file. Replay ignores it, as it ignores `mirror.cbor`.
pub const MIRROR_TMP: &str = "mirror.cbor.tmp";

/// A manifest declaring a longer trace is refused before the trace is read
/// (`OQ-03` revisits the number when P2 measures real volume).
pub const MAX_TRACE_BYTES: u64 = 256 << 20;
/// The longest encoded header item the reader will build.
/// The most a manifest may be: it holds a handful of integers and a hash.
pub const MAX_MANIFEST_BYTES: u64 = 1 << 20;
pub const MAX_HEADER_BYTES: usize = 1 << 20;
/// The longest encoded event item the reader will build, and the writer write.
pub const MAX_EVENT_BYTES: usize = 4096;

/// Decode the item at `offset`, looking at no more than `max` bytes. An item
/// that runs past that window is `Ok(None)`: nothing nested is built from it,
/// so the allocation is bounded by `max`, not by the input.
fn decode_bounded(
    trace: &[u8],
    offset: usize,
    max: usize,
) -> Result<Option<(cbor::Value, usize)>, cbor::CborError> {
    let end = offset.saturating_add(max).min(trace.len());
    match cbor::decode_prefix(&trace[..end], offset) {
        Err(e) if end < trace.len() && e.kind == cbor::CborErrorKind::UnexpectedEnd => Ok(None),
        other => other.map(Some),
    }
}

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
        if header_bytes.len() > MAX_HEADER_BYTES {
            return Err(ArchiveError::InvalidHeader("header is over 1 MiB"));
        }
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
        let mut scratch = std::mem::take(&mut self.scratch);
        scratch.clear();
        cbor::encode(&event.to_value(), &mut scratch);
        if scratch.len() > MAX_EVENT_BYTES {
            self.scratch = scratch;
            return Err(ArchiveError::InvalidEvent {
                index,
                reason: "event is over 4096 bytes",
            });
        }
        self.replayer.apply(event)?;
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

fn read_all(file: &mut File, path: &Path) -> Result<Vec<u8>, ArchiveError> {
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|e| ArchiveError::io(path, e))?;
    Ok(bytes)
}

/// The manifest, read through `take`: a sparse multi-gigabyte file is refused,
/// never allocated (review A11.2).
fn read_manifest(file: &mut File, path: &Path) -> Result<Vec<u8>, ArchiveError> {
    let mut bytes = Vec::new();
    file.take(MAX_MANIFEST_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| ArchiveError::io(path, e))?;
    if bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(ArchiveError::ManifestTooLarge {
            limit: MAX_MANIFEST_BYTES,
        });
    }
    Ok(bytes)
}

/// The run directory, opened once without following a symlink. Every check
/// and every read after this goes through the descriptor, so nothing swapped
/// in by name after the check is ever read (`AET-WO-2300` adversarial F1).
fn open_dir(dir: &Path) -> Result<OwnedFd, ArchiveError> {
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    rustix::fs::open(dir, flags, Mode::empty()).map_err(|e| match e {
        Errno::LOOP | Errno::NOTDIR => {
            ArchiveError::UnexpectedEntry(format!("{} is not a real directory", dir.display()))
        }
        e => ArchiveError::io(dir, e.into()),
    })
}

/// Check the directory's shape: only regular files named `trace.cbor`,
/// `manifest.cbor` and optionally `mirror.cbor` and `mirror.cbor.tmp`.
fn check_entries(dirfd: &OwnedFd, dir: &Path) -> Result<(), ArchiveError> {
    let io = |e: Errno| ArchiveError::io(dir, e.into());
    let mut bad = Vec::new();
    for entry in Dir::read_from(dirfd).map_err(io)? {
        let entry = entry.map_err(io)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "." || name == ".." {
            continue;
        }
        let kind = match entry.file_type() {
            FileType::Unknown => statat(dirfd, entry.file_name(), AtFlags::SYMLINK_NOFOLLOW)
                .map(|st| FileType::from_raw_mode(st.st_mode))
                .map_err(io)?,
            kind => kind,
        };
        let regular = kind == FileType::RegularFile;
        if !(regular && [TRACE, MANIFEST, MIRROR, MIRROR_TMP].contains(&name.as_str())) {
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

/// `name` in the run directory, opened through its descriptor without
/// following a symlink and without blocking on a FIFO, and checked to be a
/// regular file on the open descriptor; `None` if it does not exist.
fn open_entry(dirfd: &OwnedFd, dir: &Path, name: &str) -> Result<Option<File>, ArchiveError> {
    let path = dir.join(name);
    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let fd = match openat(dirfd, name, flags, Mode::empty()) {
        Ok(fd) => fd,
        Err(Errno::NOENT) => return Ok(None),
        Err(Errno::LOOP) => return Err(ArchiveError::UnexpectedEntry(name.to_owned())),
        Err(e) => return Err(ArchiveError::io(&path, e.into())),
    };
    let file = File::from(fd);
    let meta = file.metadata().map_err(|e| ArchiveError::io(&path, e))?;
    if !meta.is_file() {
        return Err(ArchiveError::UnexpectedEntry(name.to_owned()));
    }
    Ok(Some(file))
}

/// Replay a run directory, refusing anything it cannot prove complete.
pub fn replay_dir(dir: &Path) -> Result<Replayed, ArchiveError> {
    // Components drop a trailing separator, which would make the open follow
    // a symlinked run directory.
    let dir: PathBuf = dir.components().collect();
    let dir = dir.as_path();
    let dirfd = open_dir(dir)?;
    check_entries(&dirfd, dir)?;
    let manifest_path = dir.join(MANIFEST);
    let Some(mut manifest_file) = open_entry(&dirfd, dir, MANIFEST)? else {
        return Err(ArchiveError::Uncommitted);
    };
    let manifest_bytes = read_manifest(&mut manifest_file, &manifest_path)?;
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

    if manifest.trace_bytes > MAX_TRACE_BYTES {
        return Err(ArchiveError::TraceTooLarge {
            declared: manifest.trace_bytes,
            limit: MAX_TRACE_BYTES,
        });
    }
    let trace_path = dir.join(TRACE);
    let Some(mut trace_file) = open_entry(&dirfd, dir, TRACE)? else {
        return Err(ArchiveError::MissingTrace);
    };
    let actual = trace_file
        .metadata()
        .map_err(|e| ArchiveError::io(&trace_path, e))?
        .len();
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
    let trace = read_all(&mut trace_file, &trace_path)?;
    if <[u8; 32]>::from(Sha256::digest(&trace)) != manifest.trace_sha256 {
        return Err(ArchiveError::HashMismatch);
    }

    let (header_value, mut offset) = decode_bounded(&trace, 0, MAX_HEADER_BYTES)
        .map_err(ArchiveError::NotDeterministicCbor)?
        .ok_or_else(|| ArchiveError::MalformedHeader("header is over 1 MiB".to_owned()))?;
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
        let (value, end) = decode_bounded(&trace, offset, MAX_EVENT_BYTES)
            .map_err(ArchiveError::NotDeterministicCbor)?
            .ok_or(ArchiveError::MalformedEvent {
                index,
                offset,
                reason: "event is over 4096 bytes",
            })?;
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
