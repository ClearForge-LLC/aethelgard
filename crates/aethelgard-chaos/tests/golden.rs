//! Clause 5 of the P0 gate (`INV-04`): a 10,000-event trace replays to a state
//! sequence whose hash equals the original's, and the bytes and the hash are
//! the same on every machine.

mod common;

use aethelgard_chaos::{Event, replay_dir};
use sha2::{Digest, Sha256};

use common::*;

/// sha256 of the golden `trace.cbor`. Changing it requires bumping the format
/// version and saying why in FEEDBACK. It was `51c6193f…a896d` (trace
/// version 1) until `AET-WO-2300` added the campaign's kinds, then
/// `e1d39213…c86e8` (version 2) until `AET-WO-3100` added kind 15; each time
/// only the header's version byte changed, and setting it back gives the old
/// hash again.
const GOLDEN_TRACE_SHA256: &str =
    "a2d551d1ee8e92b88ab9ba3ee1827351d47e693ceee89cbe7451ef272964d166";
/// The golden `state_hash`. Same rule. It was `535db225…adac4` (manifest
/// version 1) until `AET-WO-0400a` made the chain hash each event's delta,
/// then `af2d46bb…b4939` until `AET-WO-2300` changed the header it starts
/// from, then `eb6408ee…93edc` until `AET-WO-3100` changed it again.
const GOLDEN_STATE_HASH: &str = "0db4b4f9398d16db958343199c145a819ff219d3d891b78db56584d2a9310cea";

#[test]
fn golden_trace_and_state_hash_are_pinned() {
    let events = golden_events();
    let (dir, manifest) = write_run("golden", 0x0400, &events);
    let bytes = std::fs::read(dir.join("trace.cbor")).unwrap();
    let trace_sha = hex(&Sha256::digest(&bytes));
    let state_hash = hex(&manifest.state_hash);
    eprintln!(
        "GOLDEN trace_sha256={trace_sha} state_hash={state_hash} bytes={}",
        bytes.len()
    );
    assert_eq!(trace_sha, GOLDEN_TRACE_SHA256);
    assert_eq!(state_hash, GOLDEN_STATE_HASH);
}

#[test]
fn the_stream_covers_every_kind_and_repeats_timestamps() {
    let events = golden_events();
    assert_eq!(events.len(), 10_000);
    let kinds = |f: fn(&Event) -> bool| events.iter().any(f);
    assert!(kinds(|e| matches!(e, Event::RegisterWrite { .. })));
    assert!(kinds(|e| matches!(e, Event::PinLevel { .. })));
    assert!(kinds(|e| matches!(e, Event::Interrupt { .. })));
    assert!(kinds(|e| matches!(e, Event::AnalogSample { .. })));
    assert!(kinds(|e| matches!(e, Event::FaultInjected { .. })));
    assert!(
        events
            .windows(2)
            .any(|w| w[0].timestamp() == w[1].timestamp())
    );
}

#[test]
fn replaying_twice_gives_the_same_state_hash() {
    let (dir, manifest) = write_run("twice", 0x0400, &golden_events());
    let first = replay_dir(&dir).unwrap();
    let second = replay_dir(&dir).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.manifest, manifest);
    assert_eq!(first.manifest.event_count, 10_000);
}

#[test]
fn two_writes_of_the_same_stream_are_byte_identical() {
    let (a, _) = write_run("same-a", 0x0400, &golden_events());
    let (b, _) = write_run("same-b", 0x0400, &golden_events());
    for name in ["trace.cbor", "manifest.cbor"] {
        assert_eq!(
            std::fs::read(a.join(name)).unwrap(),
            std::fs::read(b.join(name)).unwrap()
        );
    }
}
