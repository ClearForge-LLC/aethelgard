//! `--rail` and `--renode` on the built binary (`AET-WO-2301`), with the real
//! libngspice for the rail (the `tools` job, as `edges.rs`).

use std::path::Path;
use std::process::Command;

use serde_json::Value;

const BRIDGE: &str = env!("CARGO_BIN_EXE_aethelgard-bridge");

fn identity_ok(line: &str) {
    let v: Value = serde_json::from_str(line).unwrap();
    assert_eq!(v["bridge"], "aethelgard-bridge");
    assert_eq!(v["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(v["source_sha256"].as_str().unwrap().len(), 64);
    assert_eq!(v.as_object().unwrap().len(), 3);
    assert!(line.starts_with(r#"{"bridge":"aethelgard-bridge","source_sha256":""#));
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

const REQUEST: &str = r#"{"domain":"VDD","nominal_microvolts":3300000,"floor_microvolts":2400000,
    "start_ns":1000000,"fall_ns":20000,"hold_ns":200000,"rise_ns":20000,
    "source_milliohms":500,"load_microamps":100000,"decoupling_picofarads":400000,
    "span_ns":100000000,"levels":[{"name":"VBOR3_falling","microvolts":2610000},
    {"name":"VBOR3_rising","microvolts":2700000}]}"#;

#[test]
fn a_droop_crosses_each_level_where_the_spike_measured() {
    let dir = scratch("rail");
    let path = dir.join("rail.json");
    std::fs::write(&path, REQUEST).unwrap();
    let out = Command::new(BRIDGE)
        .arg("--rail")
        .arg(&path)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{stdout} {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let mut lines = stdout.lines();
    identity_ok(lines.next().unwrap());
    let rows: Vec<Value> = lines.map(|l| serde_json::from_str(l).unwrap()).collect();
    assert_eq!(rows.len(), 5, "four crossings and the summary: {stdout}");
    // In time order: 2.70 V falling, 2.61 V falling, 2.70 V rising, 2.61 V rising.
    let want = [
        ("VBOR3_rising", true, 1_012_422.222),
        ("VBOR3_falling", true, 1_014_422.222),
        ("VBOR3_falling", false, 1_225_977.778),
        ("VBOR3_rising", false, 1_227_977.778),
    ];
    for (row, (name, falling, ns)) in rows.iter().zip(want) {
        let got = row["at_ps"].as_u64().unwrap() as f64 / 1e3;
        assert_eq!(row["level"], name, "{row}");
        assert_eq!(row["falling"], falling, "{row}");
        assert_eq!(row["method"], "interpolated");
        assert_eq!(row["ngspice"], "44.2");
        assert!((got - ns).abs() <= 0.01, "{name}: {got} ns, not {ns}");
    }
    assert_eq!(rows[0]["microvolts"], 2_700_000);
    assert_eq!(rows[1]["microvolts"], 2_610_000);
    let last = &rows[4];
    assert_eq!(last["rail"], "solved");
    assert!(last["accepted_points"].as_u64().unwrap() > 100, "{last}");
    assert_eq!(last["minimum_microvolts"], 2_350_000, "{last}");
    eprintln!(
        "at_ps: {:?}",
        rows[..4]
            .iter()
            .map(|r| r["at_ps"].as_u64().unwrap())
            .collect::<Vec<_>>()
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_refused_rail_request_is_exit_2_after_the_identity_line() {
    let dir = scratch("rail-refused");
    let path = dir.join("bad.json");
    std::fs::write(
        &path,
        REQUEST.replace("\"span_ns\":100000000", "\"span_ns\":1"),
    )
    .unwrap();
    let out = Command::new(BRIDGE)
        .arg("--rail")
        .arg(&path)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(2), "{stdout}");
    let mut lines = stdout.lines();
    identity_ok(lines.next().unwrap());
    let e: Value = serde_json::from_str(lines.next().unwrap()).unwrap();
    assert_eq!(e["error"], "request_refused");
    assert!(lines.next().is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn renode_is_a_child_with_the_bridges_own_streams_and_exit_code() {
    let out = Command::new(BRIDGE)
        .args(["--renode", "/bin/sh", "-c", "echo hi; exit 3"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(3), "{stdout}");
    let mut lines = stdout.lines();
    identity_ok(lines.next().unwrap());
    assert_eq!(lines.next(), Some("hi"));
    assert_eq!(lines.next(), None);
}

#[test]
fn renode_exits_128_plus_the_signal_and_passes_the_environment() {
    let out = Command::new(BRIDGE)
        .args(["--renode", "/bin/sh", "-c", "kill -TERM $$"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(128 + 15));
    let out = Command::new(BRIDGE)
        .env("AET_TEST_PROBE", "kept")
        .args(["--renode", "/bin/sh", "-c", "echo $AET_TEST_PROBE; cat"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.lines().nth(1), Some("kept"), "{stdout}");
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn renode_refuses_a_relative_path() {
    for exe in ["sh", "./sh", "bin/sh"] {
        let out = Command::new(BRIDGE)
            .args(["--renode", exe, "-c", "echo hi"])
            .output()
            .unwrap();
        let stdout = String::from_utf8(out.stdout).unwrap();
        assert_eq!(out.status.code(), Some(2), "{exe}: {stdout}");
        assert!(!stdout.contains("hi"));
        let mut lines = stdout.lines();
        identity_ok(lines.next().unwrap());
        let e: Value = serde_json::from_str(lines.next().unwrap()).unwrap();
        assert_eq!(e["error"], "request_refused");
    }
}
