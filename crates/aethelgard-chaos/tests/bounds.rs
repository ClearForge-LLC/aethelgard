//! `AET-WO-0400a`: linear replay, the delta chain, and decoding bounded by
//! shape before anything nested is built.

mod common;

use std::fs;
use std::io::Write;
use std::time::Instant;

use aethelgard_chaos::archive::{MAX_EVENT_BYTES, MAX_HEADER_BYTES, MAX_TRACE_BYTES};
use aethelgard_chaos::cbor;
use aethelgard_chaos::replay::Replayer;
use aethelgard_chaos::synthetic;
use aethelgard_chaos::{ArchiveError as E, Event, Manifest, replay_dir};
use sha2::{Digest, Sha256};

use common::*;

fn header_bytes() -> Vec<u8> {
    cbor::to_bytes(&synthetic::header(0x0400).to_value())
}

fn write(timestamp: u64, address: u32, value: u32) -> Event {
    Event::RegisterWrite {
        timestamp,
        address,
        value,
    }
}

/// `n` register writes to `n` distinct addresses, folded through the replayer.
fn replay_distinct(n: u32) -> f64 {
    let mut replayer = Replayer::new(&header_bytes());
    let start = Instant::now();
    for i in 0..n {
        replayer
            .apply(&write(u64::from(i), i, i.wrapping_mul(7)))
            .unwrap();
    }
    start.elapsed().as_secs_f64()
}

#[test]
fn replay_is_linear_in_events() {
    // Best of three for the small run, to keep a noisy neighbour out of the ratio.
    let small = (0..3)
        .map(|_| replay_distinct(20_000))
        .fold(f64::MAX, f64::min);
    let large = replay_distinct(200_000);
    eprintln!("REPLAY 20,000 distinct writes: {small:.3} s; 200,000: {large:.3} s");
    // Ten times the events: linear is about 10x, quadratic about 100x.
    assert!(large <= small * 15.0, "{large} s is over 15x {small} s");
}

/// The chain is `h_i = sha256(h_{i-1} ‖ encode(delta_i))`, spelled out by hand.
#[test]
fn the_chain_is_the_documented_deltas() {
    let events = [
        write(1, 0x10, 5),
        Event::PinLevel {
            timestamp: 2,
            pin: "PA1".into(),
            level: 1,
        },
        Event::Interrupt {
            timestamp: 3,
            line: 7,
        },
        Event::Interrupt {
            timestamp: 4,
            line: 7,
        },
        Event::AnalogSample {
            timestamp: 5,
            net: "N".into(),
            microvolts: -2,
        },
        Event::FaultInjected {
            timestamp: 6,
            scenario: "s".into(),
            detail: "d".into(),
        },
    ];
    let deltas: [&[u8]; 6] = [
        &[0x83, 0x00, 0x10, 0x05],
        &[0x83, 0x01, 0x63, b'P', b'A', b'1', 0x01],
        &[0x83, 0x02, 0x07, 0x01],
        &[0x83, 0x02, 0x07, 0x02],
        &[0x83, 0x03, 0x61, b'N', 0x21],
        &[0x82, 0x04, 0x01],
    ];
    let mut expected: [u8; 32] = Sha256::digest(header_bytes()).into();
    let mut replayer = Replayer::new(&header_bytes());
    for (event, delta) in events.iter().zip(deltas) {
        let mut h = Sha256::new();
        h.update(expected);
        h.update(delta);
        expected = h.finalize().into();
        replayer.apply(event).unwrap();
        assert_eq!(replayer.state_hash(), expected);
    }
}

#[test]
fn a_no_op_event_changes_the_state_hash() {
    let base = vec![write(1, 1, 1), write(2, 2, 2)];
    let mut with_noop = base.clone();
    with_noop.push(write(3, 2, 2)); // leaves the state as it was
    let (_, a) = write_run("noop-a", 0x0400, &base);
    let (_, b) = write_run("noop-b", 0x0400, &with_noop);
    assert_ne!(a.state_hash, b.state_hash);
    // Likewise a repeated pin level.
    let pin = |t| Event::PinLevel {
        timestamp: t,
        pin: "P".into(),
        level: 1,
    };
    let (_, a) = write_run("noop-pin-a", 0x0400, &[pin(1)]);
    let (_, b) = write_run("noop-pin-b", 0x0400, &[pin(1), pin(2)]);
    assert_ne!(a.state_hash, b.state_hash);
}

/// What `state_hash` still does not see (the residue of finding 7): timestamps
/// and a fault's text. `trace_sha256` covers them.
#[test]
fn timestamps_are_covered_by_the_trace_hash_not_the_state_hash() {
    let (_, a) = write_run("ts-a", 0x0400, &[write(1, 1, 1)]);
    let (_, b) = write_run("ts-b", 0x0400, &[write(9, 1, 1)]);
    assert_eq!(a.state_hash, b.state_hash);
    assert_ne!(a.trace_sha256, b.trace_sha256);
}

// ---- bounds ----

/// A committed run around `items`, with the real length and hash and the
/// given event count and state hash.
fn commit(name: &str, header: &[u8], item: &[u8]) -> std::path::PathBuf {
    let dir = scratch(name).join("run");
    fs::create_dir(&dir).unwrap();
    let mut sha = Sha256::new();
    let mut file = fs::File::create(dir.join("trace.cbor")).unwrap();
    for part in [header, item] {
        sha.update(part);
        file.write_all(part).unwrap();
    }
    drop(file);
    let manifest = Manifest {
        trace_bytes: (header.len() + item.len()) as u64,
        trace_sha256: sha.finalize().into(),
        event_count: 1,
        state_hash: [0; 32],
    };
    fs::write(
        dir.join("manifest.cbor"),
        cbor::to_bytes(&manifest.to_value()),
    )
    .unwrap();
    dir
}

/// `[0, 0, <byte string>]`, `total` bytes in all.
fn event_of(total: usize) -> Vec<u8> {
    let mut item = vec![0x83, 0x00, 0x00, 0x59];
    let body = total - 6;
    item.extend(u16::try_from(body).unwrap().to_be_bytes());
    item.extend(vec![0; body]);
    assert_eq!(item.len(), total);
    item
}

#[test]
fn an_event_over_4096_bytes_is_refused_before_it_is_built() {
    let over = commit(
        "event-4097",
        &header_bytes(),
        &event_of(MAX_EVENT_BYTES + 1),
    );
    assert!(matches!(
        replay_dir(&over),
        Err(E::MalformedEvent { reason, .. }) if reason == "event is over 4096 bytes"
    ));
    // At the bound it is built, and refused for being the wrong shape instead.
    let at = commit("event-4096", &header_bytes(), &event_of(MAX_EVENT_BYTES));
    assert!(matches!(
        replay_dir(&at),
        Err(E::MalformedEvent { reason, .. }) if reason != "event is over 4096 bytes"
    ));
}

#[test]
fn a_header_over_1_mib_is_refused_before_it_is_built() {
    // `[<byte string>]`, `total` bytes in all.
    let header = |total: usize| {
        let mut item = vec![0x81, 0x5a];
        item.extend(((total - 6) as u32).to_be_bytes());
        item.extend(vec![0; total - 6]);
        item
    };
    let over = commit("header-over", &header(MAX_HEADER_BYTES + 1), &[]);
    assert!(matches!(
        replay_dir(&over),
        Err(E::MalformedHeader(reason)) if reason == "header is over 1 MiB"
    ));
    let at = commit("header-at", &header(MAX_HEADER_BYTES), &[]);
    assert!(matches!(
        replay_dir(&at),
        Err(E::MalformedHeader(reason)) if reason != "header is over 1 MiB"
    ));
}

#[test]
fn a_manifest_over_256_mib_is_refused_before_the_trace_is_read() {
    let (dir, m) = write_run("manifest-big", 0x0400, &[write(1, 1, 1)]);
    let declare = |trace_bytes| {
        let m = Manifest { trace_bytes, ..m };
        fs::write(dir.join("manifest.cbor"), cbor::to_bytes(&m.to_value())).unwrap();
    };
    declare(MAX_TRACE_BYTES + 1);
    assert!(matches!(
        replay_dir(&dir),
        Err(E::TraceTooLarge { declared, limit }) if declared == MAX_TRACE_BYTES + 1 && limit == MAX_TRACE_BYTES
    ));
    declare(MAX_TRACE_BYTES);
    assert!(matches!(replay_dir(&dir), Err(E::Truncated { .. })));
}

/// Finding 2's crafted event: 65,536 arrays of 255 empty arrays, 16.8 MB, which
/// the unbounded decoder built into about 578 MB. Peak memory is measured on
/// Furnace with `/usr/bin/time -v` (FEEDBACK), not asserted here.
#[test]
fn the_crafted_16_8_mb_event_is_refused() {
    let dir = {
        let mut item = vec![0x9a, 0x00, 0x01, 0x00, 0x00];
        for _ in 0..65_536 {
            item.extend([0x98, 0xff]);
            item.extend(vec![0x80; 255]);
        }
        assert!(item.len() > 16_800_000);
        commit("crafted", &header_bytes(), &item)
    };
    assert!(matches!(
        replay_dir(&dir),
        Err(E::MalformedEvent { reason, .. }) if reason == "event is over 4096 bytes"
    ));
}
