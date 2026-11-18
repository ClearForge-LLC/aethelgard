//! `AET-WO-1314`: the via ring rule and the capacitance ladder, through the
//! CLI. Where a specification declares the fabricator's tolerances, every
//! via keeps the declared ring at the worst declared hole size and position,
//! and the board is routed at the ladder's rung with the largest smallest
//! margin on its capacitance-bounded nets; `extract` judges the result on
//! its own. A specification without them is routed as before.

#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use serde_json::{Value, json};

const S03: &str = "crates/aethelgard-cli/tests/fixtures/sentinel-03.json";

fn root() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aet-1314-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str]) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .current_dir(root())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    (
        out.status.code().unwrap(),
        serde_json::from_str(&stdout).unwrap(),
    )
}

fn s03() -> Value {
    serde_json::from_str(&fs::read_to_string(root().join(S03)).unwrap()).unwrap()
}

fn write(label: &str, v: &Value) -> PathBuf {
    let path = scratch(label).join("spec.json");
    fs::write(&path, serde_json::to_string_pretty(v).unwrap()).unwrap();
    path
}

/// `emit` of `spec` into a fresh directory: its exit code, its report, and
/// the board's path when it wrote one.
fn emit(label: &str, spec: &Path) -> (i32, Value, Option<PathBuf>) {
    let dir = scratch(&format!("{label}-board"));
    let (code, v) = run(&[
        "emit",
        "--file",
        spec.to_str().unwrap(),
        "--out",
        dir.to_str().unwrap(),
    ]);
    let board = v["board"]["path"].as_str().map(PathBuf::from);
    (code, v, board)
}

/// Sentinel-03 as committed, emitted once.
fn s03_emitted() -> &'static (i32, Value, Option<PathBuf>) {
    static ONCE: OnceLock<(i32, Value, Option<PathBuf>)> = OnceLock::new();
    ONCE.get_or_init(|| emit("s03", &root().join(S03)))
}

/// Each via on a board, by (pad, drill) in mm as KiCad writes them.
fn vias(board: &Path) -> BTreeMap<(String, String), usize> {
    let text = fs::read_to_string(board).unwrap();
    let mut out = BTreeMap::new();
    for item in text.split("\n\t(via").skip(1) {
        let body = item.split("\n\t)").next().unwrap();
        let field = |key: &str| {
            let at = body.find(&format!("({key} ")).unwrap() + key.len() + 2;
            body[at..].split(')').next().unwrap().to_string()
        };
        *out.entry((field("size"), field("drill"))).or_insert(0) += 1;
    }
    out
}

/// Sentinel-03 on rule-sized vias (ruling 6982875594 D3, the acceptance):
/// every via at least the rule's pad for its drill (0.75 mm for 0.3, the
/// thermal arrays' 0.7 mm for 0.254, raised with a note); the ladder's
/// choice reported with its smallest margin; and `extract`, run on the
/// emitted board on its own, passes every capacitance-bounded net and
/// finds the same smallest margin: the ladder chose, `extract` judges.
#[test]
fn sentinel_03_on_rule_sized_vias_passes_every_bound_judged_by_extract_alone() {
    let (code, report, board) = s03_emitted();
    assert_eq!(*code, 0, "{report}");
    let board = board.as_ref().unwrap();
    let sizes = vias(board);
    assert_eq!(
        sizes.keys().cloned().collect::<Vec<_>>(),
        [
            ("0.7".to_string(), "0.254".to_string()),
            ("0.75".to_string(), "0.3".to_string())
        ],
        "{sizes:?}"
    );
    let notes: Vec<&str> = report["notes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_str().unwrap())
        .collect();
    assert!(
        notes
            .iter()
            .any(|n| n.contains("thermal vias raised from 0.554 to 0.7 mm")),
        "{notes:?}"
    );
    let ladder = &report["capacitance_ladder"];
    let margin = &ladder["smallest_margin"];
    assert!(ladder["rung"].as_u64().unwrap() < 8, "{ladder}");
    // `extract`, on its own
    let (code, v) = run(&["extract", "--board", board.to_str().unwrap(), "--spec", S03]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["verdict"], "pass", "{v}");
    let lines = v["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 8, "{v}");
    for l in lines {
        assert_eq!(l["verdict"], "pass", "{l}");
    }
    let imu = lines.iter().find(|l| l["net"] == "IMU_SCK").unwrap();
    assert!(
        imu["c_bound_femtofarads"].as_u64().unwrap() <= 30_000,
        "{imu}"
    );
    // the smallest margin `extract` finds is the one the ladder reported
    let smallest = lines
        .iter()
        .min_by(|a, b| {
            let m = |l: &Value| {
                let (c, d) = (
                    l["c_bound_femtofarads"].as_u64().unwrap() as i128,
                    l["declared_femtofarads"].as_u64().unwrap() as i128,
                );
                (d - c, d)
            };
            let ((x, xd), (y, yd)) = (m(a), m(b));
            (x * yd).cmp(&(y * xd))
        })
        .unwrap();
    assert_eq!(margin["net"], smallest["net"], "{ladder}");
    assert_eq!(
        margin["c_bound_femtofarads"], smallest["c_bound_femtofarads"],
        "{ladder}"
    );
}

/// A class whose declared via cannot keep the declared ring is refused by
/// name, with the pad that would (D2): never silently raised.
#[test]
fn a_class_via_under_the_ring_is_refused_by_name() {
    let mut v = s03();
    for c in v["net_classes"].as_array_mut().unwrap() {
        if c["name"] == "SUPPLY_MCU" {
            c["via"]["diameter_um"] = json!(700);
        }
    }
    let (code, r, _) = emit("unheld", &write("unheld", &v));
    assert_eq!(code, 2, "{r}");
    assert_eq!(r["error"]["kind"], "ViaRingUnheld", "{r}");
    assert_eq!(r["error"]["id"], "SUPPLY_MCU", "{r}");
    assert!(
        r["error"]["message"]
            .as_str()
            .unwrap()
            .contains("declares a 0.7 mm via pad; the declared ring needs 0.75 mm"),
        "{r}"
    );
}

/// Each tolerance the rule needs is refused by name where the
/// specification declares `fab_tolerances` and lacks it (`RUL-14`).
#[test]
fn each_input_the_ring_rule_needs_is_refused_by_name() {
    for key in ["hole_position_um", "hole_size_plus_um", "via_ring_um"] {
        let mut v = s03();
        v["board"]["stackup"]["fab_tolerances"]
            .as_object_mut()
            .unwrap()
            .remove(key);
        let (code, r, _) = emit(key, &write(key, &v));
        assert_eq!(code, 2, "{key}: {r}");
        assert_eq!(r["error"]["kind"], "FabToleranceUndeclared", "{key}: {r}");
        assert_eq!(
            r["error"]["id"],
            format!("board.stackup.fab_tolerances.{key}"),
            "{key}: {r}"
        );
    }
}

/// No rung fits two bounds no routing can meet: the board is refused,
/// naming each net over its bound with its figure, and nothing is written.
#[test]
fn a_board_no_rung_fits_is_refused_with_every_net_over_its_bound() {
    let mut v = s03();
    for c in v["net_constraints"].as_array_mut().unwrap() {
        if c["net_name"] == "IMU_SCK" || c["net_name"] == "IMU_MOSI" {
            c["max_capacitance_femtofarads"] = json!(12_000);
        }
    }
    let (code, r, board) = emit("over", &write("over", &v));
    assert_eq!(code, 2, "{r}");
    assert!(board.is_none(), "{r}");
    assert_eq!(r["error"]["kind"], "CapacitanceOverBound", "{r}");
    let message = r["error"]["message"].as_str().unwrap();
    for net in ["IMU_SCK", "IMU_MOSI"] {
        assert!(message.contains(&format!("`{net}` ")), "{net}: {r}");
    }
    assert!(message.contains("fF over 12000 fF"), "{r}");
}

/// A specification without `fab_tolerances` is routed as before: the
/// engine's vias and the class's declared one, no ladder, no notes.
#[test]
fn a_board_without_tolerances_is_routed_as_before() {
    let mut v = s03();
    v["board"]["stackup"]
        .as_object_mut()
        .unwrap()
        .remove("fab_tolerances");
    let (code, r, board) = emit("plain", &write("plain", &v));
    assert_eq!(code, 0, "{r}");
    assert!(r.get("capacitance_ladder").is_none(), "{r}");
    assert!(r.get("notes").is_none(), "{r}");
    let sizes = vias(board.as_ref().unwrap());
    assert!(
        sizes.contains_key(&("0.6".to_string(), "0.3".to_string())),
        "{sizes:?}"
    );
    assert!(
        sizes.contains_key(&("0.554".to_string(), "0.254".to_string())),
        "{sizes:?}"
    );
}
