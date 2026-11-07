//! Shared helpers. Run directories live under `CARGO_TARGET_TMPDIR` (on disk,
//! not the RAM-backed `/tmp`), one fresh directory per name and process.
#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;

use aethelgard_chaos::synthetic;
use aethelgard_chaos::{Event, Manifest, TraceWriter};

pub fn scratch(name: &str) -> PathBuf {
    let base =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    base
}

/// Write a committed run of `events` into a fresh directory; returns it.
pub fn write_run(name: &str, seed: u64, events: &[Event]) -> (PathBuf, Manifest) {
    let dir = scratch(name).join("run");
    let mut writer = TraceWriter::create(&dir, &synthetic::header(seed)).unwrap();
    for event in events {
        writer.append(event).unwrap();
    }
    let manifest = writer.commit().unwrap();
    (dir, manifest)
}

pub fn golden_events() -> Vec<Event> {
    synthetic::events(synthetic::GOLDEN_SEED)
        .take(synthetic::GOLDEN_EVENTS)
        .collect()
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
