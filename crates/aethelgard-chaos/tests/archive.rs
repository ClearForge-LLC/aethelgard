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
    // A timestamp that does not fit in a u64 is a codec refusal, not a wrap.
    let mut big = vec![0x83, 0x00, 0x1b];
    big.extend([0xff; 8]);
    big.extend([0x00, 0x00]);
    assert!(raw_with_event("tsmax-ok", big).is_err());
}

#[test]
fn an_unknown_version_is_refused() {
    // [magic, version, ...] -> the version byte is the one right after the
    // magic text. The trace's version is 4 since AET-WO-2301 added kinds 16 to
    // 20 (3 since AET-WO-3100 added kind 15, 2 since AET-WO-2300 added the
    // campaign's kinds); 1, 2, 3 and 5 are refused, so an older trace never
    // replays under version 4's rules (INV-06).
    let at = 1 + 1 + "aethelgard-trace".len();
    for found in [1u8, 2, 3, 5] {
        let mut trace = header_bytes();
        assert_eq!(trace[at], 0x04);
        trace[at] = found;
        let dir = commit_raw(&format!("v{found}-trace"), &trace, 0, [0; 32]);
        assert!(matches!(
            replay_dir(&dir),
            Err(E::UnknownVersion {
                item: "header",
                found: f
            }) if f == u64::from(found)
        ));
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
                found: f
            }) if f == u64::from(found)
        ));
    }
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
            // 5 to 20 are the campaign's records since AET-WO-2301.
            cbor::to_bytes(&Value::Array(vec![Value::Uint(21), Value::Uint(0)])),
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
            (
                "unknown-kind",
                Err(E::UnknownEventKind {
                    kind: 21, index: 0, ..
                }),
            ) => {}
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
