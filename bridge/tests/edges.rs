//! The bridge on the adversary's A6 edges, with the real libngspice (the
//! `tools` job; ruled F7 (b) on PR #70). Each line's outcome is a property of
//! the design, reported on stdout with exit 0, and is the same whether the line
//! is measured alone or after the others in one process.

use std::path::Path;
use std::process::Command;

use serde_json::Value;

/// `container/fixtures/bridge-request.json` (the reference `I2C_SCL` line at
/// 2.2 kΩ, 100 pF) with each (net, ohms, capacitance card) in its place.
fn request(dir: &Path, name: &str, lines: &[(&str, &str, &str)]) -> std::path::PathBuf {
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../container/fixtures/bridge-request.json");
    let base: Value = serde_json::from_slice(&std::fs::read(fixture).unwrap()).unwrap();
    let line = &base["lines"][0];
    assert_eq!(line["netlist"][2], "R1 vdd line 2200");
    assert_eq!(line["netlist"][3], "C1 line 0 100000f");
    let lines: Vec<Value> = lines
        .iter()
        .map(|(net, ohms, farads)| {
            let mut l = line.clone();
            l["net"] = (*net).into();
            l["netlist"][2] = format!("R1 vdd line {ohms}").into();
            l["netlist"][3] = format!("C1 line 0 {farads}").into();
            l
        })
        .collect();
    let path = dir.join(name);
    std::fs::write(&path, serde_json::json!({ "lines": lines }).to_string()).unwrap();
    path
}

fn bridge(request: &Path) -> Vec<Value> {
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard-bridge"))
        .arg(request)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(0), "{stdout}");
    let mut lines = stdout
        .lines()
        .map(|l| serde_json::from_str::<Value>(l).unwrap());
    // The identity line comes first in every mode (AET-WO-2200 F15).
    let identity = lines.next().unwrap();
    assert_eq!(identity["bridge"], "aethelgard-bridge", "{identity}");
    lines.collect()
}

#[test]
fn a_slow_line_is_incomplete_and_a_strong_pull_up_is_stuck_high() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("edges-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let lines = [
        ("REF", "2200", "100000f"),
        ("SLOW", "47000", "400000f"),
        ("STUCK", "10", "100000f"),
    ];
    let together = bridge(&request(&dir, "all.json", &lines));
    for (i, line) in lines.iter().enumerate() {
        let alone = bridge(&request(&dir, &format!("{i}.json"), &[*line]));
        assert_eq!(alone, [together[i].clone()], "{}", line.0);
    }
    let rise = together[0]["rise_30_70_ns"].as_f64().unwrap();
    assert!((rise - 186.40).abs() <= 0.05, "{rise}");
    // 47 kΩ and 400 pF: RC is 18.8 µs, so 70 % (1.204 RC after the release)
    // lies past the 22.5 µs window. The bound is the window after t30.
    assert_eq!(together[1]["complete"], false);
    let bound = together[1]["rise_30_70_ns_lower_bound"].as_f64().unwrap();
    assert!((13_000.0..13_600.0).contains(&bound), "{bound}");
    assert!(together[1].get("rise_30_70_ns").is_none());
    // 10 Ω against the switch's 10 Ω: the line is held at half the rail.
    let volts = together[2]["volts_at_release"].as_f64().unwrap();
    assert!((volts - 1.65).abs() < 1e-6, "{volts}");
    let _ = std::fs::remove_dir_all(&dir);
}
