//! `--chain` on the built binary (`AET-WO-2304b`), with the real libngspice
//! (the `tools` job, as `rail.rs`).

use std::path::Path;
use std::process::Command;

use serde_json::{Value, json};

const BRIDGE: &str = env!("CARGO_BIN_EXE_aethelgard-bridge");

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The reference case: a 9 V pack, a 47 uF electrolytic with 250 mOhm ESR,
/// 20.44 uF of regulator input ceramics, and two sinks.
fn request(healthy: u64, fault: u64) -> Value {
    json!({
        "v_open_microvolts": 9_000_000,
        "healthy_milliohms": healthy,
        "fault_milliohms": fault,
        "start_ns": 10_000_000,
        "span_ns": 100_000_000,
        "bulk": [{"picofarads": 47_000_000, "esr_milliohms": 250}],
        "ceramics_picofarads": 20_440_000,
        "sinks": [
            {"name": "U10", "milliwatts": 11_645, "floor_microvolts": 7_000_000},
            {"name": "U11", "milliwatts": 3_107, "floor_microvolts": 3_800_000},
        ],
    })
}

/// The exit code, the raw stdout, and the lines after the identity line.
fn run(name: &str, request: &Value) -> (i32, String, Vec<Value>) {
    let dir = scratch(name);
    let path = dir.join("chain.json");
    std::fs::write(&path, serde_json::to_vec(request).unwrap()).unwrap();
    let out = Command::new(BRIDGE)
        .arg("--chain")
        .arg(&path)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let mut lines = stdout.lines();
    let id: Value = serde_json::from_str(lines.next().unwrap()).unwrap();
    assert_eq!(id["bridge"], "aethelgard-bridge");
    assert_eq!(id["source_sha256"].as_str().unwrap().len(), 64);
    let rest = lines.map(|l| serde_json::from_str(l).unwrap()).collect();
    let _ = std::fs::remove_dir_all(&dir);
    (out.status.code().unwrap(), stdout, rest)
}

fn solved(name: &str, request: &Value) -> Value {
    let (code, stdout, rest) = run(name, request);
    assert_eq!(code, 0, "{stdout}");
    assert_eq!(rest.len(), 1, "{stdout}");
    assert_eq!(rest[0]["chain"], "solved");
    rest[0].clone()
}

/// The HIGH root of V = V_open - R * P / V, in microvolts.
fn closed_form_uv(milliohms: f64) -> f64 {
    let p = 11.645 + 3.107;
    (9.0 + (81.0 - 4.0 * milliohms / 1e3 * p).sqrt()) / 2.0 * 1e6
}

/// ngspice stops a Newton iteration and a timestep within `reltol` * |V|, and
/// the deck sets `reltol=1e-6`: 9 uV at 9 V. The measured values are within
/// 0.5 uV (they are rounded to whole microvolts), so 10 uV is that bound.
const TOLERANCE_UV: f64 = 10.0;

#[test]
fn the_steady_state_is_the_high_root_of_the_closed_form() {
    // fault 1 mOhm is not above healthy 1, so the first point is 1 -> 2.
    for (fault, settled) in [(2u64, 2.0), (500, 500.0), (900, 900.0)] {
        let a = solved(&format!("steady-{fault}"), &request(1, fault));
        let at = a["final_microvolts"].as_i64().unwrap() as f64;
        let want = closed_form_uv(settled);
        eprintln!("fault {fault} mOhm: solved {at} uV, closed form {want:.2} uV");
        assert!(
            (at - want).abs() <= TOLERANCE_UV,
            "fault {fault}: {at} against {want}"
        );
        // The start is the HIGH point of the healthy 1 mOhm, not the low one.
        let start = a["initial_microvolts"].as_i64().unwrap() as f64;
        assert!((start - closed_form_uv(1.0)).abs() <= TOLERANCE_UV, "{a}");
        assert_eq!(a["step_ns"], 10_000);
        assert!(a["accepted_points"].as_u64().unwrap() > 10_000);
        // Neither floor is crossed on the way to these points.
        for s in a["sinks"].as_array().unwrap() {
            assert!(s["at_ps"].is_null(), "{a}");
        }
    }
}

#[test]
fn a_fault_at_960_milliohms_crosses_the_7_volt_floor_falling_after_the_start() {
    let a = solved("fault-960", &request(1, 960));
    eprintln!("{a}");
    let u10 = &a["sinks"][0];
    assert_eq!(u10["name"], "U10");
    let at = u10["at_ps"].as_u64().expect("U10 crosses");
    assert!(at > 10_000_000_000, "after the 10 ms start: {at} ps");
    assert!(at < 100_000_000_000, "{at} ps");
    assert!(a["sinks"][1]["at_ps"].is_null(), "U11 does not: {a}");
    assert!(a["minimum_microvolts"].as_i64().unwrap() < 7_000_000, "{a}");
}

#[test]
fn a_fault_with_no_operating_point_still_gets_an_answer_with_crossings() {
    // V_open^2 = 81 < 4 * 1.5 * 14.752 = 88.5: the transient falls.
    let a = solved("fault-1500", &request(1, 1500));
    eprintln!("{a}");
    assert!(a["sinks"][0]["at_ps"].as_u64().unwrap() > 10_000_000_000);
    let min = a["minimum_microvolts"].as_i64().unwrap();
    assert!((0..7_000_000).contains(&min), "{a}");
    assert!(a["minimum_at_ns"].as_u64().unwrap() > 10_000_000, "{a}");
}

#[test]
fn a_far_deeper_fault_falls_through_both_floors_and_is_answered() {
    let a = solved("fault-9000", &request(1, 9_000));
    for s in a["sinks"].as_array().unwrap() {
        assert!(s["at_ps"].as_u64().is_some(), "{a}");
    }
    // Below its floor a sink is a constant current P / floor, so a deep enough fault drives
    // the node negative: bounded and finite, and reported as it is (the brief's guard).
    assert!(a["minimum_microvolts"].as_i64().unwrap() < 3_800_000, "{a}");
}

fn refused(name: &str, mutate: impl FnOnce(&mut Value)) {
    let mut v = request(1, 500);
    mutate(&mut v);
    let (code, stdout, rest) = run(name, &v);
    assert_eq!(code, 2, "{name}: {stdout}");
    assert_eq!(rest.len(), 1, "{name}: {stdout}");
    assert_eq!(rest[0]["error"], "request_refused", "{name}: {stdout}");
}

#[test]
fn every_malformed_request_is_refused_with_exit_2() {
    let top = [
        "v_open_microvolts",
        "healthy_milliohms",
        "fault_milliohms",
        "start_ns",
        "span_ns",
        "bulk",
        "ceramics_picofarads",
        "sinks",
    ];
    for k in top {
        refused(&format!("missing-{k}"), |v| {
            v.as_object_mut().unwrap().remove(k);
        });
    }
    for (k, n) in [("bulk", 0), ("sinks", 0)] {
        for inner in [
            "picofarads",
            "esr_milliohms",
            "name",
            "milliwatts",
            "floor_microvolts",
        ] {
            if (k == "bulk") == matches!(inner, "picofarads" | "esr_milliohms") {
                refused(&format!("missing-{k}-{inner}"), |v| {
                    v[k][n].as_object_mut().unwrap().remove(inner);
                });
            }
        }
        refused(&format!("unknown-in-{k}"), |v| v[k][n]["extra"] = 1.into());
    }
    refused("unknown-key", |v| v["extra"] = 1.into());
    refused("not-an-object", |v| *v = json!([1]));
    for k in [
        "v_open_microvolts",
        "healthy_milliohms",
        "start_ns",
        "span_ns",
    ] {
        refused(&format!("float-{k}"), |v| v[k] = json!(5000.5));
        refused(&format!("float-integer-{k}"), |v| v[k] = json!(5000.0));
        refused(&format!("negative-{k}"), |v| v[k] = json!(-5));
        refused(&format!("string-{k}"), |v| v[k] = json!("5"));
        refused(&format!("zero-{k}"), |v| v[k] = json!(0));
    }
    refused("float-sink-power", |v| {
        v["sinks"][0]["milliwatts"] = json!(1.5)
    });
    refused("negative-bulk", |v| v["bulk"][0]["picofarads"] = json!(-1));
    refused("float-ceramics", |v| v["ceramics_picofarads"] = json!(1.5));
    refused("fault-equals-healthy", |v| v["fault_milliohms"] = json!(1));
    refused("fault-below-healthy", |v| {
        v["healthy_milliohms"] = json!(600);
    });
    refused("start-equals-span", |v| v["start_ns"] = json!(100_000_000));
    refused("start-after-span", |v| v["start_ns"] = json!(100_000_001));
    refused("span-too-long", |v| v["span_ns"] = json!(100_000_001));
    refused("v-open-too-high", |v| {
        v["v_open_microvolts"] = json!(100_000_001);
    });
    refused("fault-too-high", |v| {
        v["fault_milliohms"] = json!(10_000_001);
    });
    refused("duplicate-name", |v| v["sinks"][1]["name"] = json!("U10"));
    for (label, name) in [
        ("empty", String::new()),
        ("newline", "U\n1".to_owned()),
        ("tab", "U\t1".to_owned()),
        ("non-ascii", "Ü1".to_owned()),
        ("delete", "U\u{7f}".to_owned()),
        ("long", "x".repeat(65)),
    ] {
        refused(&format!("name-{label}"), |v| {
            v["sinks"][0]["name"] = json!(name);
        });
    }
    refused("name-not-string", |v| v["sinks"][0]["name"] = json!(3));
    refused("floor-equals-v-open", |v| {
        v["sinks"][0]["floor_microvolts"] = json!(9_000_000);
    });
    refused("floor-above-v-open", |v| {
        v["sinks"][1]["floor_microvolts"] = json!(9_000_001);
    });
    refused("floor-zero", |v| {
        v["sinks"][0]["floor_microvolts"] = json!(0)
    });
    refused("zero-capacitance", |v| {
        v["bulk"] = json!([]);
        v["ceramics_picofarads"] = json!(0);
    });
    refused("zero-bulk-picofarads", |v| {
        v["bulk"][0]["picofarads"] = json!(0)
    });
    refused("zero-esr", |v| v["bulk"][0]["esr_milliohms"] = json!(0));
    refused("too-many-bulk", |v| {
        v["bulk"] = json!(vec![json!({"picofarads": 1000, "esr_milliohms": 10}); 9]);
    });
    refused("no-sinks", |v| v["sinks"] = json!([]));
    refused("too-many-sinks", |v| {
        let sinks: Vec<Value> = (0..9)
            .map(|i| json!({"name": format!("S{i}"), "milliwatts": 10, "floor_microvolts": 3_000_000}))
            .collect();
        v["sinks"] = json!(sinks);
    });
    refused("sinks-not-an-array", |v| v["sinks"] = json!({}));
    refused("power-zero", |v| v["sinks"][0]["milliwatts"] = json!(0));
    refused("power-too-high", |v| {
        v["sinks"][0]["milliwatts"] = json!(1_000_001);
    });
    // No operating point at the healthy R, or one at or below a floor.
    refused("no-healthy-operating-point", |v| {
        v["healthy_milliohms"] = json!(1_400);
        v["fault_milliohms"] = json!(1_500);
    });
    refused("healthy-below-a-floor", |v| {
        v["healthy_milliohms"] = json!(900);
        v["fault_milliohms"] = json!(1_000);
        v["sinks"][0]["floor_microvolts"] = json!(8_000_000);
    });
}

#[test]
fn a_wrong_argument_count_is_a_usage_refusal() {
    let out = Command::new(BRIDGE).arg("--chain").output().unwrap();
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn two_runs_are_byte_identical() {
    for fault in [500, 1500] {
        let a = run("det-a", &request(1, fault));
        let b = run("det-b", &request(1, fault));
        assert_eq!(a.0, 0);
        assert_eq!(a.1, b.1);
    }
}
