//! `AET-WO-1313`, tier 1: `board.max_outline_mm`. The Sentinel-03 fixture
//! emits inside its declared 100 x 100 mm bound with its connectors on the
//! side edges; a bound that cannot hold the parts is refused by `validate`
//! before any placement, and one the placer cannot reach is refused by
//! `emit` by name, writing nothing (§5: never a partial board).

use std::path::PathBuf;
use std::process::{Command, Stdio};

use aethelgard_eda::schematic::{self, EmitError};
use aethelgard_eda::strict;
use aethelgard_ir::{PinTable, SolvedTopology, Specification};
use serde_json::{Value, json};

const FIXTURE: &str = "crates/aethelgard-cli/tests/fixtures/sentinel-03.json";

fn root() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn fixture() -> Value {
    serde_json::from_str(&std::fs::read_to_string(root().join(FIXTURE)).unwrap()).unwrap()
}

fn solved(v: &Value) -> (Specification, SolvedTopology, PinTable) {
    let spec: Specification = serde_json::from_value(v.clone()).unwrap();
    spec.validate().unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    (spec, solved, table)
}

fn with_bound(width: u32, height: u32) -> Value {
    let mut v = fixture();
    v["board"]["max_outline_mm"] = json!({"width": width, "height": height});
    v
}

/// The board's outline (width, height) and each connector's rotation.
fn outline_and_rotations(board: &str) -> ((f64, f64), Vec<(String, String)>) {
    let tree = strict::board(board).unwrap();
    let rect = tree.root().all("gr_rect").next().unwrap();
    let xy = |head: &str| -> (f64, f64) {
        let n = rect.one(head).unwrap().unwrap();
        (
            n.arg(1).unwrap().parse().unwrap(),
            n.arg(2).unwrap().parse().unwrap(),
        )
    };
    let ((x0, y0), (x1, y1)) = (xy("start"), xy("end"));
    let mut rotations = Vec::new();
    for fp in tree.root().all("footprint") {
        let r = fp.field("Reference").unwrap().unwrap().to_owned();
        if r.starts_with('J') {
            let at = fp.one("at").unwrap().unwrap();
            rotations.push((r, at.arg(3).unwrap_or("0").to_owned()));
        }
    }
    ((x1 - x0, y1 - y0), rotations)
}

fn run(args: &[&str]) -> (Option<i32>, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .current_dir(root())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    (out.status.code(), serde_json::from_str(&stdout).unwrap())
}

fn scratch(label: &str) -> PathBuf {
    let dir = root()
        .join("target")
        .join(format!("aet-1313-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_fixture_fits_its_bound_with_connectors_on_the_side_edges() {
    let (spec, solved, table) = solved(&fixture());
    assert!(spec.board.max_outline_mm.is_some());
    let e = schematic::emit(&spec, &solved, &table).unwrap();
    let ((w, h), rotations) = outline_and_rotations(&e.board);
    assert!(w <= 100.0 && h <= 100.0, "{w} x {h}");
    assert!(!rotations.is_empty());
    for (r, angle) in &rotations {
        assert!(angle == "90" || angle == "270", "{r} at {angle}");
    }
    // The same specification with no bound is today's board: every
    // connector on the bottom edge (D4; every corpus file is checked
    // byte for byte in FEEDBACK §5).
    let mut v = fixture();
    v["board"].as_object_mut().unwrap().remove("max_outline_mm");
    let (spec, solved, table) = self::solved(&v);
    let e = schematic::emit(&spec, &solved, &table).unwrap();
    let ((w, _), rotations) = outline_and_rotations(&e.board);
    assert!(w > 100.0, "{w}");
    assert!(rotations.iter().all(|(_, a)| a == "0"));
}

#[test]
fn a_bound_the_placer_cannot_reach_is_refused_by_name_never_a_partial_board() {
    // 60 x 60 mm holds the keep-out boxes' sum, but not the placement.
    let v = with_bound(60, 60);
    let (spec, solved, table) = solved(&v);
    schematic::bound_holds(&spec, &solved, &table).unwrap();
    match schematic::emit(&spec, &solved, &table) {
        Err(e @ EmitError::OutlineUnreachable { .. }) => {
            assert_eq!(e.kind(), "OutlineUnreachable");
            assert!(e.to_string().contains("best outline"), "{e}");
        }
        other => panic!("{:?}", other.map(|_| ())),
    }
    let dir = scratch("unreachable");
    let spec_path = dir.join("spec.json");
    std::fs::write(&spec_path, serde_json::to_string(&v).unwrap()).unwrap();
    let out = dir.join("out");
    let (code, verdict) = run(&[
        "emit",
        "--file",
        spec_path.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(2), "{verdict}");
    assert_eq!(verdict["error"]["kind"], "OutlineUnreachable", "{verdict}");
    assert!(!out.exists(), "emit wrote into {}", out.display());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn validate_refuses_a_bound_that_cannot_hold_the_parts() {
    let dir = scratch("too-small");
    for (w, h, id) in [(40, 40, Value::Null), (20, 200, json!("U1"))] {
        let path = dir.join(format!("spec-{w}-{h}.json"));
        std::fs::write(&path, serde_json::to_string(&with_bound(w, h)).unwrap()).unwrap();
        let (code, verdict) = run(&["validate", "--file", path.to_str().unwrap()]);
        assert_eq!(code, Some(2), "{verdict}");
        assert_eq!(verdict["error"]["kind"], "OutlineTooSmall", "{verdict}");
        assert_eq!(verdict["error"]["id"], id, "{verdict}");
    }
    let path = dir.join("spec-100.json");
    std::fs::write(&path, serde_json::to_string(&fixture()).unwrap()).unwrap();
    let (code, verdict) = run(&["validate", "--file", path.to_str().unwrap()]);
    assert_eq!(code, Some(0), "{verdict}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_version_14_file_is_refused_by_its_version_before_its_shape() {
    let dir = scratch("v14");
    let mut v = fixture();
    v["schema_version"] = json!(14);
    let path = dir.join("v14.json");
    std::fs::write(&path, serde_json::to_string(&v).unwrap()).unwrap();
    let (code, verdict) = run(&["validate", "--file", path.to_str().unwrap()]);
    assert_eq!(code, Some(2), "{verdict}");
    assert_eq!(
        verdict["error"]["kind"], "UnsupportedSchemaVersion",
        "{verdict}"
    );
    assert_eq!(
        verdict["error"]["message"], "schema_version is 14, not 15",
        "{verdict}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
