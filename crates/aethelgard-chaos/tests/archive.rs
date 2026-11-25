//! §3.3 tampering, §3.5 the mirror boundary, and the adversarial pass's
//! hostile archives (§5): each must refuse with its own variant.

mod common;

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use aethelgard_chaos::cbor::{self, Value};
use aethelgard_chaos::mirror::{NOT_BUILT, VARIABLES, mirror_step};
use aethelgard_chaos::replay::Replayer;
use aethelgard_chaos::synthetic;
use aethelgard_chaos::{ArchiveError as E, Event, Manifest, replay_dir};
use sha2::{Digest, Sha256};

use common::*;

fn events(n: usize) -> Vec<Event> {
    synthetic::events(0x0400).take(n).collect()
}

fn enc(event: &Event) -> Vec<u8> {
    cbor::to_bytes(&event.to_value())
}

/// A committed run directory around arbitrary trace bytes. `state_hash` is
/// whatever the caller says; every other manifest field is computed.
fn commit_raw(name: &str, trace: &[u8], event_count: u64, state_hash: [u8; 32]) -> PathBuf {
    let dir = scratch(name).join("run");
    fs::create_dir(&dir).unwrap();
    fs::write(dir.join("trace.cbor"), trace).unwrap();
    let manifest = Manifest {
        trace_bytes: trace.len() as u64,
        trace_sha256: Sha256::digest(trace).into(),
        event_count,
        state_hash,
    };
    fs::write(
        dir.join("manifest.cbor"),
        cbor::to_bytes(&manifest.to_value()),
    )
    .unwrap();
    dir
}

fn raw_trace(header: &[u8], items: &[Vec<u8>]) -> Vec<u8> {
    let mut trace = header.to_vec();
    items.iter().for_each(|i| trace.extend(i));
    trace
}

/// Rewrite `manifest.cbor` for whatever `trace.cbor` now holds, keeping the
/// old event count and state hash.
fn recompute_manifest(dir: &Path, old: &Manifest) {
    let trace = fs::read(dir.join("trace.cbor")).unwrap();
    let manifest = Manifest {
        trace_bytes: trace.len() as u64,
        trace_sha256: Sha256::digest(&trace).into(),
        ..*old
    };
    fs::write(
        dir.join("manifest.cbor"),
        cbor::to_bytes(&manifest.to_value()),
    )
    .unwrap();
}

// ---- §3.3 tampering ----

#[test]
fn a_one_byte_change_is_a_hash_mismatch() {
    let (dir, _) = write_run("flip", 0x0400, &events(300));
    let path = dir.join("trace.cbor");
    let original = fs::read(&path).unwrap();
    for at in (0..original.len()).step_by(7).chain([original.len() - 1]) {
        let mut bytes = original.clone();
        bytes[at] ^= 0x01;
        fs::write(&path, &bytes).unwrap();
        assert!(
            matches!(replay_dir(&dir), Err(E::HashMismatch)),
            "byte {at}"
        );
    }
    fs::write(&path, &original).unwrap();
    assert!(replay_dir(&dir).is_ok());
}

#[test]
fn truncation_at_every_event_boundary_and_inside_an_event_is_truncated() {
    let evs = events(100);
    let (dir, _) = write_run("trunc", 0x0400, &evs);
    let path = dir.join("trace.cbor");
    let original = fs::read(&path).unwrap();
    let mut cut = header_bytes().len();
    let mut boundaries = vec![cut];
    for e in &evs {
        cut += enc(e).len();
        boundaries.push(cut);
    }
    assert_eq!(*boundaries.last().unwrap(), original.len());
    for &b in &boundaries[..boundaries.len() - 1] {
        fs::write(&path, &original[..b]).unwrap();
        assert!(
            matches!(replay_dir(&dir), Err(E::Truncated { .. })),
            "cut at {b}"
        );
    }
    let inside = boundaries[50] + 1;
    assert!(!boundaries.contains(&inside));
    fs::write(&path, &original[..inside]).unwrap();
    assert!(matches!(replay_dir(&dir), Err(E::Truncated { .. })));
    fs::write(&path, b"").unwrap();
    assert!(matches!(replay_dir(&dir), Err(E::Truncated { .. })));
}

#[test]
fn an_appended_byte_is_extended() {
    let (dir, _) = write_run("append", 0x0400, &events(50));
    let path = dir.join("trace.cbor");
    let mut bytes = fs::read(&path).unwrap();
    bytes.push(0);
    fs::write(&path, bytes).unwrap();
    assert!(matches!(replay_dir(&dir), Err(E::Extended { .. })));
}

#[test]
fn a_missing_manifest_is_uncommitted() {
    let (dir, _) = write_run("nomanifest", 0x0400, &events(50));
    fs::remove_file(dir.join("manifest.cbor")).unwrap();
    assert!(matches!(replay_dir(&dir), Err(E::Uncommitted)));
}

#[test]
fn a_missing_trace_is_named() {
    let (dir, _) = write_run("notrace", 0x0400, &events(50));
    fs::remove_file(dir.join("trace.cbor")).unwrap();
    assert!(matches!(replay_dir(&dir), Err(E::MissingTrace)));
}

/// Swap events `i` and `i+1` in a copy of the stream, write that trace with a
/// manifest recomputed for the new bytes but the original state hash.
fn swapped(name: &str, evs: &[Event], i: usize) -> Result<(), E> {
    let (dir, manifest) = write_run(name, 0x0400, evs);
    let mut items: Vec<Vec<u8>> = evs.iter().map(enc).collect();
    items.swap(i, i + 1);
    fs::write(dir.join("trace.cbor"), raw_trace(&header_bytes(), &items)).unwrap();
    recompute_manifest(&dir, &manifest);
    replay_dir(&dir).map(|_| ())
}

#[test]
fn two_writes_to_different_registers_swapped_are_a_replay_divergence() {
    // Same timestamp and different registers: the final state is identical
    // either way, and only the state sequence tells the orders apart.
    let evs = vec![
        write(5, 0, 0),
        write(9, 0x10, 1),
        write(9, 0x20, 2),
        write(12, 0x30, 3),
    ];
    assert!(matches!(
        swapped("swap-equal-ts", &evs, 1),
        Err(E::ReplayDiverged)
    ));
}

#[test]
fn two_events_swapped_across_timestamps_are_a_timestamp_regression() {
    let evs = vec![write(5, 0, 0), write(9, 0x10, 1), write(11, 0x20, 2)];
    assert!(matches!(
        swapped("swap-ts", &evs, 1),
        Err(E::TimestampRegressed { index: 2 })
    ));
}

#[test]
fn a_swap_inside_the_generated_stream_never_passes() {
    let evs = events(400);
    let mut refused = 0;
    for i in (0..399).step_by(13) {
        match swapped(&format!("swap-gen-{i}"), &evs, i) {
            Err(E::ReplayDiverged | E::TimestampRegressed { .. }) => refused += 1,
            Ok(()) if evs[i] == evs[i + 1] => {}
            other => panic!("swap at {i}: {other:?}"),
        }
    }
    assert!(refused > 20);
}

// ---- §5.1 forgery: integrity, not authenticity ----

#[test]
fn a_forged_trace_with_a_recomputed_manifest_passes() {
    // Recorded plainly: the manifest proves the bytes are the ones committed,
    // not who committed them. An attacker who rewrites the events and every
    // manifest field gets a trace that replays. Nothing less does: each
    // single-field change is refused above and below.
    let header = header_bytes();
    let forged = [write(1, 0xdead, 0xbeef)];
    let trace = raw_trace(&header, &forged.iter().map(enc).collect::<Vec<_>>());
    let mut r = Replayer::new(&header);
    forged.iter().for_each(|e| r.apply(e).unwrap());
    let dir = commit_raw("forged", &trace, 1, r.state_hash());
    let replayed = replay_dir(&dir).unwrap();
    assert_eq!(replayed.state.registers[&0xdead], 0xbeef);
}

#[test]
fn each_manifest_field_alone_is_checked() {
    let (dir, m) = write_run("fields", 0x0400, &events(60));
    let try_with = |m: Manifest| {
        fs::write(dir.join("manifest.cbor"), cbor::to_bytes(&m.to_value())).unwrap();
        replay_dir(&dir).map(|_| ())
    };
    assert!(matches!(
        try_with(Manifest {
            event_count: m.event_count + 1,
            ..m
        }),
        Err(E::EventCountMismatch { .. })
    ));
    assert!(matches!(
        try_with(Manifest {
            event_count: m.event_count - 1,
            ..m
        }),
        Err(E::EventCountMismatch { .. })
    ));
    let mut h = m.state_hash;
    h[31] ^= 1;
    assert!(matches!(
        try_with(Manifest { state_hash: h, ..m }),
        Err(E::ReplayDiverged)
    ));
    let mut h = m.trace_sha256;
    h[0] ^= 1;
    assert!(matches!(
        try_with(Manifest {
            trace_sha256: h,
            ..m
        }),
        Err(E::HashMismatch)
    ));
    assert!(matches!(
        try_with(Manifest {
            trace_bytes: m.trace_bytes - 1,
            ..m
        }),
        Err(E::Extended { .. })
    ));
    assert!(matches!(
        try_with(Manifest {
            trace_bytes: m.trace_bytes + 1,
            ..m
        }),
        Err(E::Truncated { .. })
    ));
    assert!(try_with(m).is_ok());
}

// ---- §5.2 hostile archives ----

fn raw_with_event(name: &str, item: Vec<u8>) -> Result<(), E> {
    let dir = commit_raw(name, &raw_trace(&header_bytes(), &[item]), 1, [0; 32]);
    replay_dir(&dir).map(|_| ())
}

#[test]
fn a_declared_length_of_u64_max_is_refused_without_allocating() {
    let mut item = vec![0x5b];
    item.extend([0xff; 8]);
    assert!(matches!(
        raw_with_event("hostile-len", item.clone()),
        Err(E::NotDeterministicCbor(e)) if e.kind == cbor::CborErrorKind::StringTooLong(u64::MAX)
    ));
    // In the header position, and in an array length.
    let dir = commit_raw("hostile-len-header", &item, 0, [0; 32]);
    assert!(matches!(replay_dir(&dir), Err(E::NotDeterministicCbor(_))));
    let mut arr = vec![0x9b];
    arr.extend([0xff; 8]);
    assert!(matches!(
        raw_with_event("hostile-arr", arr),
        Err(E::NotDeterministicCbor(_))
    ));
    // A manifest that declares a trace of u64::MAX bytes is over the bound,
    // and the trace is never read.
    let (dir, m) = write_run("hostile-manifest-len", 0x0400, &events(5));
    let m = Manifest {
        trace_bytes: u64::MAX,
        ..m
    };
    fs::write(dir.join("manifest.cbor"), cbor::to_bytes(&m.to_value())).unwrap();
    assert!(matches!(replay_dir(&dir), Err(E::TraceTooLarge { .. })));
}

#[test]
fn nesting_nine_deep_is_refused() {
    // An event whose payload nests: [0, 0, [[[[...]]]]] nine arrays deep in all, the event itself the first.
    let mut item = vec![0x83, 0x00, 0x00];
    item.extend(vec![0x81; 7]);
    item.push(0x80);
    assert!(matches!(
        raw_with_event("deep9", item),
        Err(E::NotDeterministicCbor(e)) if e.kind == cbor::CborErrorKind::TooDeep
    ));
    // Eight deep passes the codec and is then refused as a malformed event.
    let mut item = vec![0x83, 0x00, 0x00];
    item.extend(vec![0x81; 6]);
    item.push(0x80);
    assert!(matches!(
        raw_with_event("deep8", item),
        Err(E::MalformedEvent { .. })
    ));
}

#[test]
fn a_timestamp_of_u64_max_followed_by_another_event_is_refused() {
    let items = [enc(&write(u64::MAX, 0, 0)), enc(&write(0, 0, 0))];
    let dir = commit_raw("tsmax", &raw_trace(&header_bytes(), &items), 2, [0; 32]);
    assert!(matches!(
        replay_dir(&dir),
        Err(E::TimestampRegressed { index: 1 })
    ));
    // A timestamp of u64::MAX, the most CBOR's eight bytes hold, decodes
    // whole, never wrapped: this event is refused by its payload's length,
    // after the codec read it (`AET-WO-2302b`: asserted by name).
    let mut big = vec![0x83, 0x00, 0x1b];
    big.extend([0xff; 8]);
    big.extend([0x00, 0x00]);
    assert!(matches!(
        raw_with_event("tsmax-ok", big),
        Err(E::MalformedEvent {
            reason: "event payload has the wrong length",
            ..
        })
    ));
}

#[test]
fn an_unknown_version_is_refused() {
    // [magic, version, ...] -> the version byte is the one right after the
    // magic text. The trace's version is 19 since AET-WO-2310 hashed the
    // header's tools and placed the planted instants from the boot word (18
    // since AET-WO-1211c added kinds 33
    // to 37 and the overrides' tags 8 and 9, 17 since AET-WO-2305 added kind
    // 32, 16 since AET-WO-3305 narrowed kind 13's `absent`, 15 since
    // AET-WO-2304b added kind 31
    // and the overrides' tags 6 and 7, 14 since AET-WO-4400g made kind
    // 26's period the realized ns at kind 25's kernel clock, 13 since
    // AET-WO-2302 gave kinds 26 and 29 their windows and added kind 30, 12 since AET-WO-4230 added the
    // brown-out word, 11 since AET-WO-4300b added kinds 27
    // to 29, 10 since AET-WO-4220 named each of several configuration steps,
    // 9 since AET-WO-4210 made kinds 12, 13, 21 and 22 one event per
    // script step, 8 since AET-WO-4300a added kinds 24 to 26, 7 since
    // AET-WO-4200 added kinds 21 to 23, 6 since AET-WO-2205 named the tools in
    // the header, 5 since AET-WO-2204 made kinds 13 and 19 carry the device
    // model's log lines, 4 since AET-WO-2301 added kinds 16 to 20, 3 since
    // AET-WO-3100 added kind 15, 2 since AET-WO-2300 added the campaign's
    // kinds); every other version from 1 to one past it is refused, so an
    // older trace never replays under this version's rules (INV-06).
    let at = 1 + 1 + "aethelgard-trace".len();
    let version = u8::try_from(aethelgard_chaos::event::TRACE_VERSION).unwrap();
    for found in (1..=version + 1).filter(|&v| v != version) {
        let mut trace = header_bytes();
        assert_eq!(trace[at], version);
        trace[at] = found;
        let dir = commit_raw(&format!("v{found}-trace"), &trace, 0, [0; 32]);
        // An older trace is refused with the rows since it, oldest first
        // (`AET-WO-2310`); a newer one with none.
        let want: Vec<u64> = (u64::from(found) + 1..=u64::from(version))
            .filter(|_| found < version)
            .collect();
        assert!(
            matches!(
                replay_dir(&dir),
                Err(E::UnknownVersion {
                    item: "header",
                    found: f,
                    changes,
                }) if f == u64::from(found)
                    && changes.iter().map(|c| c.version).collect::<Vec<_>>() == want
            ),
            "{found}"
        );
    }

    let (dir, m) = write_run("v2-manifest", 0x0400, &events(5));
    let mut bytes = cbor::to_bytes(&m.to_value());
    let at = 1 + 1 + "aethelgard-manifest".len();
    // The manifest's version is 2 since AET-WO-0400a; 1 (the old state hash) and 3 are refused.
    assert_eq!(bytes[at], 0x02);
    for found in [1u8, 3] {
        bytes[at] = found;
        fs::write(dir.join("manifest.cbor"), &bytes).unwrap();
        assert!(matches!(
            replay_dir(&dir),
            Err(E::UnknownVersion {
                item: "manifest",
                found: f,
                changes: [],
            }) if f == u64::from(found)
        ));
    }
}

/// (i) `AET-WO-2310`: an older trace's refusal lists exactly the versions
/// since it, each with its work order and change, and says so in its text; a
/// newer one lists none.
#[test]
fn an_older_trace_is_refused_with_what_changed_since_it() {
    use aethelgard_chaos::event::{TRACE_CHANGES, TRACE_VERSION, changes_since};
    assert_eq!(TRACE_VERSION, 20);
    assert_eq!(TRACE_CHANGES.last().map(|c| c.version), Some(TRACE_VERSION));
    let versions: Vec<u64> = TRACE_CHANGES.iter().map(|c| c.version).collect();
    assert_eq!(versions, (2..=20).collect::<Vec<_>>());
    let at = 1 + 1 + "aethelgard-trace".len();
    let refused = |found: u8| {
        let mut trace = header_bytes();
        trace[at] = found;
        let dir = commit_raw(&format!("since-v{found}"), &trace, 0, [0; 32]);
        replay_dir(&dir).expect_err("refused")
    };
    for (found, want) in [(14u8, vec![15, 16, 17, 18, 19, 20]), (19, vec![20])] {
        let e = refused(found);
        let E::UnknownVersion {
            item: "header",
            found: f,
            changes,
        } = &e
        else {
            panic!("{e:?}")
        };
        assert_eq!(*f, u64::from(found));
        assert_eq!(changes.iter().map(|c| c.version).collect::<Vec<_>>(), want);
        assert_eq!(*changes, changes_since(u64::from(found)));
        let text = e.to_string();
        assert!(
            text.starts_with(&format!(
                "unknown format version {found} in header; this reader is version 20, and since then: {} (",
                found + 1
            )),
            "{text}"
        );
        for c in *changes {
            assert!(
                text.contains(&format!("{} ({}): {}", c.version, c.work_order, c.change)),
                "{text}"
            );
        }
    }
    assert!(
        refused(18)
            .to_string()
            .contains("since then: 19 (AET-WO-2310)")
    );
    assert!(
        refused(19)
            .to_string()
            .contains("since then: 20 (AET-WO-4400i)")
    );
    assert!(
        refused(14)
            .to_string()
            .contains("since then: 15 (AET-WO-2304b)")
    );
    let e = refused(21);
    assert!(
        matches!(
            e,
            E::UnknownVersion {
                item: "header",
                found: 21,
                changes: [],
            }
        ),
        "{e:?}"
    );
    assert_eq!(e.to_string(), "unknown format version 21 in header");
    assert!(changes_since(20).is_empty() && changes_since(21).is_empty());
}

/// A header with `tools`, `[name, version, [] or [sha256]]` each, as
/// trace 19 writes it (`AET-WO-2310`).
fn header_with_tools(tools: Vec<Value>) -> Value {
    let Value::Array(mut items) = synthetic::header(0x0400).to_value() else {
        panic!("the header is an array")
    };
    assert_eq!(
        items.len(),
        6,
        "magic, version, run id, unit, artifacts, tools"
    );
    items[5] = Value::Array(tools);
    Value::Array(items)
}

fn tool(name: &str, version: &str, hash: &[Value]) -> Value {
    Value::Array(vec![
        Value::Text(name.into()),
        Value::Text(version.into()),
        Value::Array(hash.to_vec()),
    ])
}

/// `AET-WO-2310`: a tool's hash round-trips through CBOR as the third item
/// of its entry, `[]` for a tool not hashed.
#[test]
fn a_tool_hash_round_trips_through_the_header() {
    let mut h = synthetic::header(0x0400);
    h.tools = vec![
        ("aethelgard".into(), "0.1.0".into()),
        ("ngspice".into(), "45".into()),
        ("rust-toolchain".into(), "rustc 1.91.0".into()),
    ];
    h.tool_sha256 = vec![
        ("ngspice".into(), [0xA5; 32]),
        ("rust-toolchain".into(), [0x5A; 32]),
    ];
    let v = h.to_value();
    let Value::Array(items) = &v else { panic!() };
    assert_eq!(
        items[5],
        Value::Array(vec![
            tool("aethelgard", "0.1.0", &[]),
            tool("ngspice", "45", &[Value::Bytes(vec![0xA5; 32])]),
            tool(
                "rust-toolchain",
                "rustc 1.91.0",
                &[Value::Bytes(vec![0x5A; 32])]
            ),
        ])
    );
    assert_eq!(
        aethelgard_chaos::Header::from_value(&v).map_err(|_| ()),
        Ok(h.clone())
    );
    assert_eq!(h.sha256_of("ngspice"), Some(&[0xA5; 32]));
    assert_eq!(h.sha256_of("aethelgard"), None);
    // Through a committed trace and replay too: an empty run's state hash is
    // its header's digest.
    let raw = cbor::to_bytes(&v);
    let dir = commit_raw("tool-hash", &raw, 0, Sha256::digest(&raw).into());
    assert_eq!(replay_dir(&dir).unwrap().header, h);
}

/// `AET-WO-2310`: a tool entry of two items (trace 18's), a hash that is not
/// 32 bytes, or not one byte string, is a malformed header; so is a hash
/// list out of order or naming no tool of the header.
#[test]
fn a_tool_entry_of_two_items_or_a_short_hash_is_malformed() {
    let bytes = |n: usize| Value::Bytes(vec![1; n]);
    for (name, entry, reason) in [
        (
            "tool-pair",
            Value::Array(vec![
                Value::Text("ngspice".into()),
                Value::Text("45".into()),
            ]),
            "tool is not [name, version, [] or [sha256]]",
        ),
        (
            "tool-31",
            tool("ngspice", "45", &[bytes(31)]),
            "tool hash is not 32 bytes",
        ),
        (
            "tool-33",
            tool("ngspice", "45", &[bytes(33)]),
            "tool hash is not 32 bytes",
        ),
        (
            "tool-two-hashes",
            tool("ngspice", "45", &[bytes(32), bytes(32)]),
            "tool hash is not [] or [sha256]",
        ),
        (
            "tool-hash-text",
            tool("ngspice", "45", &[Value::Text("x".into())]),
            "tool hash is not [] or [sha256]",
        ),
        (
            "tool-hash-bytes",
            Value::Array(vec![
                Value::Text("ngspice".into()),
                Value::Text("45".into()),
                bytes(32),
            ]),
            "tool is not [name, version, [] or [sha256]]",
        ),
    ] {
        let dir = commit_raw(
            name,
            &cbor::to_bytes(&header_with_tools(vec![entry])),
            0,
            [0; 32],
        );
        match replay_dir(&dir) {
            Err(E::MalformedHeader(m)) => assert_eq!(m, reason, "{name}"),
            other => panic!("{name}: {other:?}"),
        }
    }
    let ok = cbor::to_bytes(&header_with_tools(vec![tool(
        "ngspice",
        "45",
        &[bytes(32)],
    )]));
    let dir = commit_raw("tool-ok", &ok, 0, Sha256::digest(&ok).into());
    assert!(replay_dir(&dir).is_ok());
    // A hash naming no tool, or out of order, is refused by the header's check.
    let mut h = synthetic::header(0x0400);
    h.tools = vec![("a".into(), "1".into()), ("b".into(), "1".into())];
    h.tool_sha256 = vec![("c".into(), [0; 32])];
    assert_eq!(h.check(), Err("a tool hash names no tool of the header"));
    h.tool_sha256 = vec![("b".into(), [0; 32]), ("a".into(), [0; 32])];
    assert_eq!(h.check(), Err("tool hashes are not sorted by unique name"));
    h.tool_sha256 = vec![("a".into(), [0; 32]), ("b".into(), [0; 32])];
    assert_eq!(h.check(), Ok(()));
}

#[test]
fn a_second_manifest_or_any_stray_entry_is_refused() {
    for (i, extra) in [
        "manifest2.cbor",
        "manifest.cbor.tmp",
        "notes.txt",
        ".hidden",
    ]
    .into_iter()
    .enumerate()
    {
        let (dir, _) = write_run(&format!("stray-{i}"), 0x0400, &events(5));
        fs::copy(dir.join("manifest.cbor"), dir.join(extra)).unwrap();
        assert!(
            matches!(replay_dir(&dir), Err(E::UnexpectedEntry(n)) if n == extra),
            "{extra}"
        );
    }
    let (dir, _) = write_run("stray-dir", 0x0400, &events(5));
    fs::create_dir(dir.join("sub")).unwrap();
    assert!(matches!(replay_dir(&dir), Err(E::UnexpectedEntry(n)) if n == "sub"));
}

#[test]
fn symlinks_are_refused_wherever_they_sit() {
    for name in ["trace.cbor", "manifest.cbor"] {
        let (dir, _) = write_run(&format!("link-{name}"), 0x0400, &events(5));
        fs::rename(dir.join(name), dir.parent().unwrap().join("moved")).unwrap();
        symlink(dir.parent().unwrap().join("moved"), dir.join(name)).unwrap();
        assert!(
            matches!(replay_dir(&dir), Err(E::UnexpectedEntry(n)) if n == name),
            "{name}"
        );
    }
    let (dir, _) = write_run("link-mirror", 0x0400, &events(5));
    symlink(dir.join("trace.cbor"), dir.join("mirror.cbor")).unwrap();
    assert!(matches!(replay_dir(&dir), Err(E::UnexpectedEntry(n)) if n == "mirror.cbor"));
    // The run directory itself.
    let (dir, _) = write_run("link-dir", 0x0400, &events(5));
    let alias = dir.parent().unwrap().join("alias");
    symlink(&dir, &alias).unwrap();
    assert!(replay_dir(&dir).is_ok());
    assert!(matches!(replay_dir(&alias), Err(E::UnexpectedEntry(_))));
    let slashed = PathBuf::from(format!("{}/", alias.display()));
    assert!(matches!(replay_dir(&slashed), Err(E::UnexpectedEntry(_))));
}

#[test]
fn field_level_refusals_name_their_own_variant() {
    let bad_events: Vec<(&str, Vec<u8>)> = vec![
        (
            "unknown-kind",
            // 5 to `LAST_KIND` are the campaign's records.
            cbor::to_bytes(&Value::Array(vec![
                Value::Uint(aethelgard_chaos::record::LAST_KIND + 1),
                Value::Uint(0),
            ])),
        ),
        (
            "short-payload",
            cbor::to_bytes(&Value::Array(vec![
                Value::Uint(0),
                Value::Uint(0),
                Value::Uint(1),
            ])),
        ),
        (
            "u32-overflow",
            cbor::to_bytes(&Value::Array(vec![
                Value::Uint(0),
                Value::Uint(0),
                Value::Uint(1 << 32),
                Value::Uint(0),
            ])),
        ),
        (
            "wrong-type",
            cbor::to_bytes(&Value::Array(vec![
                Value::Uint(1),
                Value::Uint(0),
                Value::Uint(1),
                Value::Uint(0),
            ])),
        ),
        (
            "bad-level",
            cbor::to_bytes(&Value::Array(vec![
                Value::Uint(1),
                Value::Uint(0),
                Value::Text("PA0".into()),
                Value::Uint(2),
            ])),
        ),
        (
            "irq-1024",
            cbor::to_bytes(&Value::Array(vec![
                Value::Uint(2),
                Value::Uint(0),
                Value::Uint(1024),
            ])),
        ),
        (
            "non-ascii-pin",
            cbor::to_bytes(&Value::Array(vec![
                Value::Uint(1),
                Value::Uint(0),
                Value::Text("PА0".into()),
                Value::Uint(1),
            ])),
        ),
        (
            "space-in-net",
            cbor::to_bytes(&Value::Array(vec![
                Value::Uint(3),
                Value::Uint(0),
                Value::Text("a b".into()),
                Value::Uint(1),
            ])),
        ),
        (
            "empty-scenario",
            cbor::to_bytes(&Value::Array(vec![
                Value::Uint(4),
                Value::Uint(0),
                Value::Text(String::new()),
                Value::Text(String::new()),
            ])),
        ),
        (
            "long-name",
            cbor::to_bytes(&Value::Array(vec![
                Value::Uint(3),
                Value::Uint(0),
                Value::Text("x".repeat(65)),
                Value::Uint(1),
            ])),
        ),
        (
            "tab-in-detail",
            cbor::to_bytes(&Value::Array(vec![
                Value::Uint(4),
                Value::Uint(0),
                Value::Text("s".into()),
                Value::Text("a\tb".into()),
            ])),
        ),
        (
            "float-microvolts",
            vec![0x84, 0x03, 0x00, 0x61, b'n', 0xf9, 0x00, 0x00],
        ),
        ("not-an-array", vec![0x00]),
    ];
    for (name, item) in bad_events {
        let result = raw_with_event(name, item);
        match (name, result) {
            ("unknown-kind", Err(E::UnknownEventKind { kind, index: 0, .. }))
                if kind == aethelgard_chaos::record::LAST_KIND + 1 => {}
            ("float-microvolts", Err(E::NotDeterministicCbor(_))) => {}
            (n, Err(E::MalformedEvent { index: 0, .. }))
                if n != "unknown-kind" && n != "float-microvolts" => {}
            (n, other) => panic!("{n}: {other:?}"),
        }
    }
    // A boundary event that is allowed.
    let ok = Event::FaultInjected {
        timestamp: 0,
        scenario: "x".repeat(64),
        detail: " ".repeat(1024),
    };
    let (dir, _) = write_run("detail-max", 0x0400, &[ok]);
    assert!(replay_dir(&dir).is_ok());
}

#[test]
fn a_header_with_unsorted_or_duplicate_artifacts_is_refused() {
    for (name, list) in [
        ("unsorted", vec![("b", 1u8), ("a", 2)]),
        ("duplicate", vec![("a", 1), ("a", 1)]),
    ] {
        let artifacts = list
            .iter()
            .map(|(n, h)| Value::Array(vec![Value::Text((*n).into()), Value::Bytes(vec![*h; 32])]))
            .collect();
        let header = Value::Array(vec![
            Value::Text("aethelgard-trace".into()),
            Value::Uint(aethelgard_chaos::event::TRACE_VERSION),
            Value::Bytes(vec![0; 16]),
            Value::Text("ns".into()),
            Value::Array(artifacts),
        ]);
        let dir = commit_raw(name, &cbor::to_bytes(&header), 0, [0; 32]);
        assert!(
            matches!(replay_dir(&dir), Err(E::MalformedHeader(_))),
            "{name}"
        );
    }
}

#[test]
fn the_writer_refuses_what_the_reader_would() {
    let dir = scratch("writer-refuses").join("run");
    let mut w = aethelgard_chaos::TraceWriter::create(&dir, &synthetic::header(1)).unwrap();
    w.append(&write(10, 0, 0)).unwrap();
    assert!(matches!(
        w.append(&write(9, 0, 0)),
        Err(E::TimestampRegressed { index: 1 })
    ));
    let bad = Event::PinLevel {
        timestamp: 10,
        pin: "has space".into(),
        level: 1,
    };
    assert!(matches!(
        w.append(&bad),
        Err(E::InvalidEvent { index: 1, .. })
    ));
    // An existing run directory is never reused.
    assert!(matches!(
        aethelgard_chaos::TraceWriter::create(&dir, &synthetic::header(1)),
        Err(E::RunDirExists(_))
    ));
}

#[test]
fn an_empty_run_commits_and_replays() {
    let (dir, m) = write_run("empty-run", 0x0400, &[]);
    let r = replay_dir(&dir).unwrap();
    assert_eq!(r.manifest.event_count, 0);
    assert_eq!(
        m.state_hash,
        <[u8; 32]>::from(Sha256::digest(header_bytes()))
    );
}

// ---- §3.5 the mirror boundary ----

const SENTINELS: [&str; 3] = [
    "SENTINEL-bucket-7f3a",
    "SENTINEL-keyid-91c2",
    "SENTINEL-secret-d40e",
];

fn lookup(set: [bool; 3]) -> impl Fn(&str) -> Option<OsString> {
    move |name| {
        let i = VARIABLES.iter().position(|v| *v == name)?;
        set[i].then(|| OsString::from(SENTINELS[i]))
    }
}

fn mirror_record(dir: &Path) -> Vec<Value> {
    match cbor::decode_exact(&fs::read(dir.join("mirror.cbor")).unwrap()).unwrap() {
        Value::Array(items) => items,
        other => panic!("{other:?}"),
    }
}

fn whole_dir_and_log(dir: &Path, log: &[u8]) -> Vec<u8> {
    let mut all = log.to_vec();
    for entry in fs::read_dir(dir).unwrap() {
        all.extend(fs::read(entry.unwrap().path()).unwrap());
    }
    all
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|w| w == needle.as_bytes())
}

#[test]
fn with_every_variable_unset_one_line_names_the_bucket() {
    let (dir, _) = write_run("mirror-none", 0x0400, &events(20));
    let before = replay_dir(&dir).unwrap();
    let mut log = Vec::new();
    mirror_step(&dir, lookup([false; 3]), &mut log).unwrap();
    let text = String::from_utf8(log).unwrap();
    assert_eq!(text.lines().count(), 1, "{text:?}");
    assert!(text.ends_with('\n'));
    assert!(text.contains("AETHELGARD_R2_BUCKET"));
    assert!(!text.contains("AETHELGARD_R2_ACCESS_KEY_ID"));
    let record = mirror_record(&dir);
    assert_eq!(record[2], Value::Uint(0));
    assert_eq!(
        record[3],
        Value::Text("AETHELGARD_R2_BUCKET is not set".into())
    );
    // The commit is unchanged, and replay does not read the mirror.
    assert_eq!(replay_dir(&dir).unwrap(), before);
}

#[test]
fn each_variable_unset_in_turn_is_the_one_named() {
    for missing in 0..3 {
        let (dir, _) = write_run(&format!("mirror-{missing}"), 0x0400, &events(20));
        let before = replay_dir(&dir).unwrap();
        let mut set = [true; 3];
        set[missing] = false;
        let mut log = Vec::new();
        mirror_step(&dir, lookup(set), &mut log).unwrap();
        let text = String::from_utf8(log.clone()).unwrap();
        assert_eq!(text.lines().count(), 1);
        for (i, name) in VARIABLES.iter().enumerate() {
            assert_eq!(text.contains(name), i == missing, "{text}");
        }
        assert_eq!(
            mirror_record(&dir)[3],
            Value::Text(format!("{} is not set", VARIABLES[missing]))
        );
        let all = whole_dir_and_log(&dir, &log);
        SENTINELS.iter().for_each(|s| assert!(!contains(&all, s)));
        assert_eq!(replay_dir(&dir).unwrap(), before);
    }
}

#[test]
fn with_all_set_nothing_is_claimed_and_no_value_leaks() {
    let (dir, _) = write_run("mirror-all", 0x0400, &events(20));
    let before = replay_dir(&dir).unwrap();
    let mut log = Vec::new();
    mirror_step(&dir, lookup([true; 3]), &mut log).unwrap();
    assert!(log.is_empty());
    let record = mirror_record(&dir);
    assert_eq!(record[2], Value::Uint(0));
    assert_eq!(record[3], Value::Text(NOT_BUILT.into()));
    let all = whole_dir_and_log(&dir, &log);
    SENTINELS
        .iter()
        .for_each(|s| assert!(!contains(&all, s), "{s}"));
    assert_eq!(replay_dir(&dir).unwrap(), before);
}

#[test]
fn an_empty_value_counts_as_missing_and_is_not_logged() {
    let (dir, _) = write_run("mirror-empty", 0x0400, &events(5));
    let mut log = Vec::new();
    let env = |name: &str| (name != VARIABLES[1]).then(|| OsString::from("x"));
    let env_empty = |name: &str| {
        if name == VARIABLES[1] {
            Some(OsString::new())
        } else {
            env(name)
        }
    };
    mirror_step(&dir, env_empty, &mut log).unwrap();
    assert!(String::from_utf8(log).unwrap().contains(VARIABLES[1]));
}

#[test]
fn the_mirror_never_runs_before_the_commit_or_twice() {
    let dir = scratch("mirror-order").join("run");
    let w = aethelgard_chaos::TraceWriter::create(&dir, &synthetic::header(1)).unwrap();
    let mut log = Vec::new();
    assert!(matches!(
        mirror_step(&dir, lookup([false; 3]), &mut log),
        Err(E::MirrorBeforeCommit)
    ));
    assert!(log.is_empty() && !dir.join("mirror.cbor").exists());
    w.commit().unwrap();
    mirror_step(&dir, lookup([false; 3]), &mut log).unwrap();
    assert!(matches!(
        mirror_step(&dir, lookup([false; 3]), &mut log),
        Err(E::MirrorExists)
    ));
}

#[test]
fn a_planted_mirror_tmp_is_ignored_and_then_cleared() {
    let (dir, manifest) = write_run("mirror-tmp", 0x0400, &events(20));
    fs::write(dir.join("mirror.cbor.tmp"), [0xff; 1000]).unwrap();
    assert_eq!(replay_dir(&dir).unwrap().manifest, manifest);
    let mut log = Vec::new();
    mirror_step(&dir, lookup([false; 3]), &mut log).unwrap();
    assert!(!dir.join("mirror.cbor.tmp").exists());
    assert_eq!(replay_dir(&dir).unwrap().manifest, manifest);
    // A tmp that is not a regular file is still a stray entry.
    let (dir, _) = write_run("mirror-tmp-link", 0x0400, &events(5));
    symlink(dir.join("trace.cbor"), dir.join("mirror.cbor.tmp")).unwrap();
    assert!(matches!(replay_dir(&dir), Err(E::UnexpectedEntry(n)) if n == "mirror.cbor.tmp"));
}

#[test]
fn production_passes_the_process_environment() {
    // Compile-time: `|name| std::env::var_os(name)` is the lookup production passes (the bare fn item is not general enough over lifetimes).
    fn takes(_: impl Fn(&str) -> Option<OsString>) {}
    takes(|name| std::env::var_os(name));
}

// ---- plumbing numbers for FEEDBACK (not OQ-03) ----

#[test]
fn report_encoded_sizes_and_write_throughput() {
    let evs = golden_events();
    let mut by_kind = [(0usize, 0usize); 5];
    for e in &evs {
        let k = match e {
            Event::RegisterWrite { .. } => 0,
            Event::PinLevel { .. } => 1,
            Event::Interrupt { .. } => 2,
            Event::AnalogSample { .. } => 3,
            Event::FaultInjected { .. } => 4,
            Event::Record { .. } => unreachable!("the synthetic stream has no campaign record"),
        };
        by_kind[k].0 += 1;
        by_kind[k].1 += enc(e).len();
    }
    for (k, (n, bytes)) in by_kind.iter().enumerate() {
        eprintln!(
            "PLUMBING kind {k}: {n} events, {:.1} bytes/event",
            *bytes as f64 / *n as f64
        );
    }
    let start = std::time::Instant::now();
    let (dir, m) = write_run("throughput", 0x0400, &evs);
    let secs = start.elapsed().as_secs_f64();
    eprintln!(
        "PLUMBING write+commit of 10000 events: {:.3} s ({:.0} events/s, {} bytes), profile={}",
        secs,
        10_000.0 / secs,
        m.trace_bytes,
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );
    let start = std::time::Instant::now();
    replay_dir(&dir).unwrap();
    eprintln!("PLUMBING replay: {:.3} s", start.elapsed().as_secs_f64());
}

#[test]
fn a_two_mebibyte_manifest_is_refused_by_name() {
    let (dir, _m) = write_run("huge-manifest", 0x0400, &events(5));
    // Sparse: 2 MiB of length, no 2 MiB of disk.
    let f = fs::OpenOptions::new()
        .write(true)
        .open(dir.join("manifest.cbor"))
        .unwrap();
    f.set_len(2 << 20).unwrap();
    assert!(matches!(
        replay_dir(&dir),
        Err(E::ManifestTooLarge { limit }) if limit == 1 << 20
    ));
}

/// `AET-WO-4400i`: a foreign run's records (kinds 38 to 40) round-trip in
/// parts under the event bound, each refuses its malformed shapes, and a
/// campaign run never carries one.
#[test]
fn a_foreign_runs_records_round_trip_in_parts_and_refuse_their_bad_shapes() {
    use aethelgard_chaos::record::{FOREIGN_PART_BYTES, FOREIGN_PART_STIMULI, Record};
    let input = |part, parts, file: Vec<u8>| Record::ForeignInput {
        label: "CONSOLE".into(),
        peripheral: "USART1".into(),
        spacing_ns: 100_000,
        part,
        parts,
        file,
    };
    let stimuli = |part, parts, n: usize| Record::Stimuli {
        part,
        parts,
        entries: (0..n as u64)
            .map(|k| {
                (
                    k * 83_300_000,
                    "i2c1.u7".into(),
                    "Pressure".into(),
                    "101325.080".into(),
                )
            })
            .collect(),
    };
    let capture = |part, parts, bytes: Vec<u8>| Record::SerialCaptured {
        label: "CONSOLE".into(),
        end_ns: 5_000_000_000,
        part,
        parts,
        bytes,
    };
    let records = vec![
        input(0, 2, vec![b'{'; FOREIGN_PART_BYTES]),
        input(1, 2, b"}".to_vec()),
        stimuli(0, 1, FOREIGN_PART_STIMULI),
        capture(0, 2, vec![0xFD; FOREIGN_PART_BYTES]),
        capture(1, 2, Vec::new()),
    ];
    assert_eq!(
        records.iter().map(Record::kind).collect::<Vec<_>>(),
        [38, 38, 39, 40, 40]
    );
    let events: Vec<Event> = records
        .iter()
        .cloned()
        .map(|record| Event::Record {
            timestamp: record.timestamp(5_000_000_000),
            record,
        })
        .collect();
    let (dir, _) = write_run("foreign-roundtrip", 10, &events);
    assert_eq!(replay_dir(&dir).unwrap().state.records, records);
    for r in &records {
        assert_eq!(Record::from_fields(r.kind(), &r.fields()).as_ref(), Some(r));
    }
    // Each shape the reader refuses.
    for (bad, why) in [
        (input(2, 2, b"x".to_vec()), "part"),
        (input(0, 1, Vec::new()), "part"),
        (input(0, 1, vec![0; FOREIGN_PART_BYTES + 1]), "part"),
        (stimuli(0, 1, FOREIGN_PART_STIMULI + 1), "part"),
        (capture(1, 1, Vec::new()), "part"),
        (capture(0, 1, vec![0; FOREIGN_PART_BYTES + 1]), "part"),
    ] {
        let e = bad.check().expect_err(why);
        assert!(e.contains("part"), "{e}");
    }
    // A port's label and a spacing of 0.
    let mut bad = input(0, 1, b"x".to_vec());
    if let Record::ForeignInput { label, .. } = &mut bad {
        *label = String::new();
    }
    assert!(bad.check().unwrap_err().contains("label"));
    let mut bad = input(0, 1, b"x".to_vec());
    if let Record::ForeignInput { spacing_ns, .. } = &mut bad {
        *spacing_ns = 0;
    }
    assert!(bad.check().unwrap_err().contains("spacing"));
    let mut bad = capture(0, 1, Vec::new());
    if let Record::SerialCaptured { label, .. } = &mut bad {
        *label = " ".into();
    }
    assert!(bad.check().unwrap_err().contains("label"));
    let unordered = Record::Stimuli {
        part: 0,
        parts: 1,
        entries: vec![
            (2, "d".into(), "p".into(), "1".into()),
            (1, "d".into(), "p".into(), "1".into()),
        ],
    };
    assert_eq!(
        unordered.check(),
        Err("a run's stimuli are not in time order")
    );
    // A campaign run carrying one is refused by its rules.
    assert_eq!(
        aethelgard_chaos::campaign::check_run(&records).unwrap_err(),
        "a foreign run's record (kinds 38 to 40) is not a campaign's"
    );
}
