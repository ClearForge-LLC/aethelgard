//! `AET-WO-4411` (b), ruled 2026-11-26T00:22Z: the pour ladder, tier 1 (no
//! tool runs). After routing, `emit` meters each buck's joined copper
//! (`pour::joined`) and holds its junction screen's need
//! (`power::junction_need_cm2`) on every joined layer, stepping through
//! [`aethelgard_eda::route::POUR_STEPS`] after step 0, today's routing. A
//! board with no buck runs step 0 alone and moves no byte; Sentinel-03 needs
//! a later step; a need no step clears is refused by name (`BuckPourShort`).
//! One emit at one step is about 35 s here (eight rungs, each on its own
//! thread, in the debug build).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;
use sha2::{Digest, Sha256};

fn root() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

fn emit(label: &str, spec: &Path) -> (i32, Value, PathBuf) {
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("pour-ladder-{label}"));
    let _ = fs::remove_dir_all(&out);
    let o = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(["emit", "--file"])
        .arg(spec)
        .arg("--out")
        .arg(&out)
        .current_dir(root())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8(o.stdout).unwrap();
    let v = serde_json::from_str(stdout.lines().last().unwrap_or("{}")).unwrap();
    (o.status.code().unwrap(), v, out)
}

fn s03() -> Value {
    serde_json::from_slice(&fs::read(root().join("examples/sentinel-03.json")).unwrap()).unwrap()
}

fn write(label: &str, v: &Value) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("pour-ladder-spec-{label}"));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("spec.json");
    fs::write(&path, serde_json::to_string_pretty(v).unwrap()).unwrap();
    path
}

/// Sentinel-03's step 0 leaves U11's F.Cu cut off at its pad (36 mm²,
/// measured); it settles at a later step with every buck at its need on
/// every joined layer, F.Cu, B.Cu and In1.Cu.
#[test]
fn sentinel_03_needs_a_later_step_and_settles_with_every_buck_at_its_need() {
    let (code, v, _) = emit("s03", &root().join("examples/sentinel-03.json"));
    assert_eq!(code, 0, "{v}");
    let l = &v["capacitance_ladder"];
    let step = l["pour_step"].as_u64().unwrap();
    assert!(step >= 1, "{l}");
    assert_eq!(
        l["pour_square_permille"],
        aethelgard_eda::route::POUR_STEPS[usize::try_from(step).unwrap() - 1],
        "{l}"
    );
    let copper = l["buck_copper"].as_array().unwrap();
    assert_eq!(copper.len(), 9, "{l}");
    for c in copper {
        assert!(
            c["square_millimetres"].as_u64() >= c["need"].as_u64(),
            "{c}"
        );
    }
    let need = |b: &str| {
        copper.iter().find(|c| c["buck"] == b).unwrap()["need"]
            .as_u64()
            .unwrap()
    };
    // U10's 1170 mW clears 125 C at the figure's 15 cm²; U11's and U13's at 7.
    assert_eq!((need("U10"), need("U11"), need("U13")), (1500, 700, 700));
}

/// A need no step clears: U10's loss at 1590 mW clears 125 C only at the
/// figure's last point, 58 cm² per layer, which no square about it on this
/// board holds. Refused by name, with the buck, the layer and the area.
#[test]
fn a_need_no_step_clears_is_refused_by_name() {
    let mut v = s03();
    for r in v["power"]["regulators"].as_array_mut().unwrap() {
        if r["reference"] == "U10" {
            r["junction"]["loss_milliwatts"]["value"] = 1590.into();
        }
    }
    let (code, out, dir) = emit("short", &write("short", &v));
    assert_ne!(code, 0, "{out}");
    assert_eq!(out["error"]["kind"], "BuckPourShort", "{out}");
    let m = out["error"]["message"].as_str().unwrap();
    assert!(m.contains("U10 on ") && m.contains("under 5800 mm2"), "{m}");
    assert!(
        !dir.join("sentinel-03.kicad_pcb").exists(),
        "a board was written"
    );
}

/// A board with no buck runs step 0 alone, today's ladder: its emit names
/// no pour step, and Sentinel-01's board is the same bytes as before the
/// ladder (sha256 measured on 3a35be0f's build and on this one). The board
/// names the engine's version (`generator_version`), so each version moves
/// the hash (`AET-WO-1802`; 0.3.0's here).
#[test]
fn a_board_with_no_buck_takes_step_0_and_moves_no_byte() {
    let (code, v, dir) = emit("s01", &root().join("examples/sentinel-01.json"));
    assert_eq!(code, 0, "{v}");
    assert!(v["capacitance_ladder"].get("pour_step").is_none(), "{v}");
    let board = fs::read(dir.join("sentinel-01-bench.kicad_pcb")).unwrap();
    let hash: String = Sha256::digest(&board)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(
        hash,
        "6f37e94551a3f0a8709b38069925a0746ad8cb14f58a7d602dc5850ceff54482"
    );
    let ready = root().join("crates/aethelgard-cli/tests/fixtures/sentinel-01-ready.json");
    let (code, v, _) = emit("ready", &ready);
    assert_eq!(code, 0, "{v}");
    // laddered, with no buck: the capacitance ladder alone
    assert!(v["capacitance_ladder"]["rung"].is_u64(), "{v}");
    assert!(v["capacitance_ladder"].get("pour_step").is_none(), "{v}");
}
