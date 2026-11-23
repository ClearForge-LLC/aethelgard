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

pub fn header_bytes() -> Vec<u8> {
    aethelgard_chaos::cbor::to_bytes(&synthetic::header(0x0400).to_value())
}

pub fn write(timestamp: u64, address: u32, value: u32) -> Event {
    Event::RegisterWrite {
        timestamp,
        address,
        value,
    }
}

/// `AET-WO-2309`: the refusal of a boot that read nothing beside what only a
/// boot Renode read gives (kinds 26, 29, 30, 34, 36 and 37).
pub const RENODE_READ: &str =
    "a boot that read nothing holds a motor, serial, pin or level witness, or a judge's stop";

/// `AET-WO-2309`: `r` read as a boot killed before its reads, as the judge
/// archives one: kind 10's four and its flash accesses, and every register,
/// identity read and witness line beside it, unread. The verdict is left to
/// the caller, which names the bound.
pub fn unread_boot(r: &mut [aethelgard_chaos::record::Record]) {
    use aethelgard_chaos::record::Record;
    for x in r {
        match x {
            Record::BootObserved {
                boot_word,
                instructions_observed,
                pc,
                brown_out,
                flash_access,
                ..
            } => {
                boot_word.observed = None;
                *instructions_observed = None;
                *pc = None;
                brown_out.observed = None;
                *flash_access = None;
            }
            Record::RegisterObserved { reading, .. } => reading.observed = None,
            Record::I2cCheck {
                observed, status, ..
            }
            | Record::SpiCheck {
                observed, status, ..
            } => {
                *observed = None;
                *status = None;
            }
            Record::I2cWitness {
                observed, absent, ..
            } => {
                *observed = None;
                absent.clear();
            }
            Record::SpiWitness {
                observed, unrouted, ..
            } => {
                *observed = None;
                unrouted.clear();
            }
            _ => {}
        }
    }
}
