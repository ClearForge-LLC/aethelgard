//! `AET-WO-4100`, tier 1 (no tool runs): a specification declares 2 or 4
//! copper layers, its stackup and its rules, and the writer, the router, the
//! reader and extraction follow that declaration and never the board's own
//! claims. The KiCad half (DRC, the package) is `stackups_tier2.rs`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

const SPEC: &str = "examples/sentinel-01-minimal.json";
const FOUR: &str = "crates/aethelgard-cli/tests/fixtures/four-layer.json";
const D1: &str = "crates/aethelgard-cli/tests/fixtures/d1-24-i2c.json";
const D3: &str = "crates/aethelgard-cli/tests/fixtures/d3-24-i2c-12-spi.json";

fn root() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aet-4100-{label}-{}", std::process::id()));
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

fn load(path: &str) -> Value {
    serde_json::from_slice(&fs::read(root().join(path)).unwrap()).unwrap()
}

/// Writes `spec` into `dir` and returns its path.
fn write_spec(dir: &Path, name: &str, spec: &Value) -> String {
    let path = dir.join(name);
    fs::write(&path, serde_json::to_vec_pretty(spec).unwrap()).unwrap();
    path.to_str().unwrap().to_owned()
}

/// `emit` of `spec` into `dir`: the exit code, the verdict and the board text.
fn emit(dir: &Path, spec: &str) -> (i32, Value, String) {
    let out = dir.join("out");
    let (code, v) = run(&["emit", "--file", spec, "--out", out.to_str().unwrap()]);
    let board = match v["board"]["path"].as_str() {
        Some(p) => fs::read_to_string(p).unwrap(),
        None => String::new(),
    };
    (code, v, board)
}

fn extract(dir: &Path, board: &str, spec: &str) -> (i32, Value) {
    let path = dir.join("judged.kicad_pcb");
    fs::write(&path, board).unwrap();
    run(&["extract", "--board", path.to_str().unwrap(), "--spec", spec])
}

#[test]
fn a_version_one_specification_is_refused_by_name() {
    let dir = scratch("v1");
    let mut v1 = load(SPEC);
    v1["schema_version"] = json!(1);
    v1.as_object_mut().unwrap().remove("board");
    let path = write_spec(&dir, "v1.json", &v1);
    for sub in ["validate", "emit"] {
        let args: Vec<&str> = if sub == "validate" {
            vec![sub, "--file", &path]
        } else {
            vec![sub, "--file", &path, "--out", "/nonexistent-aet-4100"]
        };
        let (code, v) = run(&args);
        assert_eq!(code, 2, "{sub}: {v}");
        assert_eq!(v["error"]["kind"], "UnsupportedSchemaVersion", "{sub}: {v}");
        assert!(v["error"]["message"].as_str().unwrap().contains("not 2"));
    }
    fs::remove_dir_all(&dir).unwrap();
}

/// The four-layer fixture (OSH Park's published stackup, ruled D2) is the
/// board it declares: KiCad's inner layers, a GND plane on In1.Cu and the
/// domain's plane on In2.Cu, both unfilled, no B.Cu zone, through vias, the
/// declared thickness. Extraction passes with the dielectric stated.
#[test]
fn the_four_layer_board_is_the_declared_one_and_extracts() {
    let dir = scratch("four");
    let (code, v, board) = emit(&dir, FOUR);
    assert_eq!(code, 0, "{v}");
    for want in [
        "\t\t(0 \"F.Cu\" signal)\n\t\t(4 \"In1.Cu\" signal)\n\t\t(6 \"In2.Cu\" signal)\n\t\t(2 \"B.Cu\" signal)\n",
        "(thickness 1.6)",
        "(net \"GND\")\n\t\t(layer \"In1.Cu\")",
        "(net \"VDD_3V3\")\n\t\t(layer \"In2.Cu\")",
    ] {
        assert!(board.contains(want), "{want}");
    }
    assert_eq!(board.matches("\t(zone\n").count(), 2);
    assert!(!board.contains("\t(zone\n\t\t(net \"GND\")\n\t\t(layer \"B.Cu\")"));
    assert!(
        !board.contains("(filled_polygon"),
        "planes are emitted unfilled"
    );
    for via in board.split("\t(via\n").skip(1) {
        assert!(via.contains("(layers \"F.Cu\" \"B.Cu\")"), "{via}");
    }
    // No track on an inner layer. A plane net's tracks are F.Cu stubs from a
    // pad to its via; none runs on B.Cu, which is routing copper only.
    for seg in board.split("\t(segment\n").skip(1) {
        let seg = seg.split("\n\t)").next().unwrap();
        assert!(!seg.contains("In1.Cu") && !seg.contains("In2.Cu"), "{seg}");
        if seg.contains("(net \"VDD_3V3\")") || seg.contains("(net \"GND\")") {
            assert!(seg.contains("(layer \"F.Cu\")"), "{seg}");
        }
    }
    let (code, x) = extract(&dir, &board, FOUR);
    assert_eq!(code, 0, "{x}");
    assert_eq!(x["verdict"], "pass");
    let conditions: Vec<&str> = x["conditions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    assert!(conditions.contains(
        &"each dielectric at least its declared minimum: F.Cu to In1.Cu 0.179 mm, In1.Cu to In2.Cu 0.891 mm, In2.Cu to B.Cu 0.179 mm"
    ));
    assert!(conditions.contains(
        &"finished copper at most 44 um thick; finished board from 1.25 mm to 1.692 mm thick"
    ));
    // Measured with the integer path (FEEDBACK §2): evidence for Sentinel-02.
    let bounds: Vec<(String, u64)> = x["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            (
                l["net"].as_str().unwrap().to_owned(),
                l["c_bound_femtofarads"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        bounds,
        [
            ("I2C_SDA".to_owned(), 44_953),
            ("I2C_SCL".to_owned(), 47_792)
        ],
        "re-measure FEEDBACK §2 before changing this line"
    );
    fs::remove_dir_all(&dir).unwrap();
}

/// The reader follows the specification, never the board (ruling, further
/// requirements; `RUL-03`): a board whose copper layers, plane nets or
/// thickness disagree with the declaration is refused by name, and an
/// inner-layer track where a plane is declared is refused.
#[test]
fn the_reader_refuses_a_board_that_disagrees_with_the_specification() {
    let dir = scratch("disagree");
    let (code, v, four) = emit(&dir, FOUR);
    assert_eq!(code, 0, "{v}");
    let ref_dir = dir.join("ref");
    fs::create_dir_all(&ref_dir).unwrap();
    let (code, v, two) = emit(&ref_dir, SPEC);
    assert_eq!(code, 0, "{v}");
    let kind = |board: &str, spec: &str| {
        let (code, v) = extract(&dir, board, spec);
        assert_eq!(code, 3, "{v}");
        v["error"]["kind"].as_str().unwrap().to_owned()
    };
    // Each board against the other's specification.
    assert_eq!(kind(&four, SPEC), "LayersDisagree");
    assert_eq!(kind(&two, FOUR), "LayersDisagree");
    // A plane on the wrong net; a plane dropped; a zone on B.Cu at 4 layers.
    let swapped = four.replace(
        "(net \"VDD_3V3\")\n\t\t(layer \"In2.Cu\")",
        "(net \"GND\")\n\t\t(layer \"In2.Cu\")",
    );
    assert_ne!(swapped, four);
    assert_eq!(kind(&swapped, FOUR), "PlanesDisagree");
    let moved = four.replace(
        "(net \"VDD_3V3\")\n\t\t(layer \"In2.Cu\")",
        "(net \"VDD_3V3\")\n\t\t(layer \"B.Cu\")",
    );
    assert_eq!(kind(&moved, FOUR), "PlanesDisagree");
    // A thickness the specification does not declare.
    let thick = four.replace("(thickness 1.6)", "(thickness 1.2)");
    assert_eq!(kind(&thick, FOUR), "ThicknessDisagrees");
    // A layer table renamed (KiCad reads copper by id).
    let renamed = four.replace("(4 \"In1.Cu\" signal)", "(4 \"In1.Cu\" power)");
    assert_eq!(kind(&renamed, FOUR), "LayersDisagree");
    // An inner-layer track where the specification declares a plane.
    let track = four.replacen(
        "\t(via\n",
        "\t(segment\n\t\t(start 80 80)\n\t\t(end 81 80)\n\t\t(width 0.2)\n\t\t(layer \"In1.Cu\")\n\t\t(net \"/I2C_SDA\")\n\t\t(uuid \"00000000-0000-0000-0000-000000004100\")\n\t)\n\t(via\n",
        1,
    );
    let (code, v) = extract(&dir, &track, FOUR);
    assert_eq!(code, 3, "{v}");
    assert!(
        v["error"]["message"].as_str().unwrap().contains("In1.Cu"),
        "{v}"
    );
    fs::remove_dir_all(&dir).unwrap();
}

/// 1302 B10 (ruled D3): the reviewer's `D1` (24 I2C devices, short
/// references) fails closed at 2 layers and routes at 4; `D3` (the same with
/// 4 SPI buses of 3 flashes) still fails closed at 4, naming its net, the
/// target of the escape-planning work order.
#[test]
fn d1_routes_at_four_layers_and_fails_closed_at_two() {
    let dir = scratch("d1");
    let (code, v, board) = emit(&dir, D1);
    assert_eq!(code, 0, "{v}");
    assert!(board.contains("(4 \"In1.Cu\" signal)"));
    let mut two = load(D1);
    two["board"] = load(SPEC)["board"].clone();
    let path = write_spec(&dir, "d1-two.json", &two);
    let two_dir = dir.join("two");
    fs::create_dir_all(&two_dir).unwrap();
    let (code, v, _) = emit(&two_dir, &path);
    assert_eq!(code, 2, "{v}");
    assert_eq!(v["error"]["kind"], "Unroutable", "{v}");
    assert!(
        v["error"]["message"].as_str().unwrap().contains("rip-ups"),
        "{v}"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn d3_fails_closed_at_four_layers_naming_its_net() {
    let dir = scratch("d3");
    let (code, v, _) = emit(&dir, D3);
    assert_eq!(code, 2, "{v}");
    assert_eq!(v["error"]["kind"], "Unroutable", "{v}");
    assert!(
        v["error"]["id"].as_str().is_some_and(|n| !n.is_empty()),
        "{v}"
    );
    fs::remove_dir_all(&dir).unwrap();
}

/// The rules field (ruled): the router routes with the declared track width
/// and the reader requires it; a width or clearance the reference's parts
/// cannot meet fails closed rather than being built at the floor.
#[test]
fn the_declared_rules_are_the_ones_routed() {
    let dir = scratch("rules");
    let mut wide = load(FOUR);
    wide["board"]["rules"]["track_width_um"] = json!(210);
    let path = write_spec(&dir, "w210.json", &wide);
    let (code, v, board) = emit(&dir, &path);
    assert_eq!(code, 0, "{v}");
    let tracks: Vec<&str> = board.split("\t(segment\n").skip(1).collect();
    assert!(!tracks.is_empty());
    for t in &tracks {
        assert!(t.contains("(width 0.21)"), "{t}");
    }
    let (code, x) = extract(&dir, &board, &path);
    assert_eq!(code, 0, "{x}");
    // The same board judged by the floor's width is refused.
    let (code, x) = extract(&dir, &board, FOUR);
    assert_eq!(code, 3, "{x}");
    // A clearance the LQFP's own 0.2 mm pad gaps break fails closed.
    let mut wider = load(SPEC);
    wider["board"]["rules"]["clearance_um"] = json!(250);
    let path = write_spec(&dir, "c250.json", &wider);
    let out = dir.join("c250");
    fs::create_dir_all(&out).unwrap();
    let (code, v, _) = emit(&out, &path);
    assert_eq!(code, 2, "{v}");
    assert_eq!(v["error"]["kind"], "Unroutable", "{v}");
    fs::remove_dir_all(&dir).unwrap();
}

/// Ruled D5: on the reference (a 200 µm dielectric, `h = S`), today's integer
/// path and the cap path give equal integers for every term of every line.
#[test]
fn the_old_and_new_paths_agree_on_the_reference() {
    use aethelgard_eda::{extract, stack::Stack};
    let bytes = fs::read(root().join(SPEC)).unwrap();
    let spec: aethelgard_ir::Specification = serde_json::from_slice(&bytes).unwrap();
    spec.validate().unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    let lines = extract::lines(&spec, &solved, &table).unwrap();
    let dir = scratch("paths");
    let (code, v, board) = emit(&dir, SPEC);
    assert_eq!(code, 0, "{v}");
    let stack = Stack::of(&spec);
    let read = extract::read(&board, &stack).unwrap();
    let old = extract::bound(&read, &lines).unwrap();
    let new = extract::bound_by_cap_path(&read, &lines).unwrap();
    assert_eq!(old, new);
    assert_eq!(
        old.iter()
            .map(|l| l.c_bound_femtofarads)
            .collect::<Vec<_>>(),
        [46_800, 58_235],
        "the reference's bounds (AET-WO-1305 §3) moved"
    );
    fs::remove_dir_all(&dir).unwrap();
}

/// The reference declares today's two-layer board (ruled D5), and this is
/// the literal `aethelgard_eda::stack::reference()` the eda unit tests use.
#[test]
fn the_reference_declares_todays_two_layer_board() {
    use aethelgard_eda::stack::{Stack, Thickness};
    let spec: aethelgard_ir::Specification =
        serde_json::from_slice(&fs::read(root().join(SPEC)).unwrap()).unwrap();
    spec.validate().unwrap();
    let s = Stack::of(&spec);
    let thickness_source = "AET-WO-1305 conditions: 1.6 mm declared; 0.34 mm = 2 x 70 um + 0.2 mm; 2.0 mm = 1.6 mm + 25 %";
    assert_eq!(
        s,
        Stack {
            copper: vec!["F.Cu".into(), "B.Cu".into()],
            planes: vec![],
            thickness: Thickness {
                nominal_nm: 1_600_000,
                min_nm: 340_000,
                max_nm: 2_000_000,
                source: thickness_source.into(),
            },
            dielectrics: vec![(
                "F.Cu".into(),
                "B.Cu".into(),
                200_000,
                "implied: the finished minimum 340 um less twice the copper maximum 70 um".into(),
            )],
            copper_max_nm: 70_000,
            copper_source: "AET-WO-1305 condition: twice KiCad's default 35 um (2 oz)".into(),
            permittivity_permille: 5400,
            permittivity_source: "the IPC-4101 FR-4 maximum, specified at 1 MHz".into(),
            clearance_nm: 200_000,
            width_nm: 200_000,
        }
    );
}

/// Adversarial A1 (`AET-WO-4100` §5.1): `check drc --spec` judges the board
/// against the declared board before any tool runs, so a board that is not
/// the specification's is refused by name, whatever KiCad would say: the
/// 2-layer reference under the 4-layer specification and the reverse, the
/// planes swapped, and a GND via made blind or micro.
#[test]
fn check_drc_with_spec_refuses_a_board_that_is_not_the_declared_one() {
    let dir = scratch("drc-declared");
    let (code, v, four) = emit(&dir, FOUR);
    assert_eq!(code, 0, "{v}");
    let ref_dir = dir.join("ref");
    fs::create_dir_all(&ref_dir).unwrap();
    let (code, v, two) = emit(&ref_dir, SPEC);
    assert_eq!(code, 0, "{v}");
    let drc = |board: &str, spec: &str| {
        let path = dir.join("judged.kicad_pcb");
        fs::write(&path, board).unwrap();
        let (code, v) = run(&[
            "check",
            "drc",
            "--board",
            path.to_str().unwrap(),
            "--spec",
            spec,
        ]);
        assert_eq!(code, 3, "{v}");
        v["error"]["kind"].as_str().unwrap().to_owned()
    };
    assert_eq!(drc(&two, FOUR), "LayersDisagree");
    assert_eq!(drc(&four, SPEC), "LayersDisagree");
    let swapped = four
        .replace(
            "(net \"GND\")\n\t\t(layer \"In1.Cu\")",
            "(net \"GND\")\n\t\t(layer \"In9\")",
        )
        .replace(
            "(net \"VDD_3V3\")\n\t\t(layer \"In2.Cu\")",
            "(net \"VDD_3V3\")\n\t\t(layer \"In1.Cu\")",
        )
        .replace(
            "(net \"GND\")\n\t\t(layer \"In9\")",
            "(net \"GND\")\n\t\t(layer \"In2.Cu\")",
        );
    assert_ne!(swapped, four);
    assert_eq!(drc(&swapped, FOUR), "PlanesDisagree");
    for kind in ["blind", "micro"] {
        let typed = four.replacen("\t(via\n", &format!("\t(via {kind}\n"), 1);
        assert_eq!(drc(&typed, FOUR), "ViaNotThrough", "{kind}");
    }
    let skipping = four.replacen(
        "(layers \"F.Cu\" \"B.Cu\")",
        "(layers \"F.Cu\" \"In1.Cu\")",
        1,
    );
    assert_eq!(drc(&skipping, FOUR), "ViaNotThrough");
    // `extract` refuses the same boards.
    let (code, x) = extract(&dir, &four.replacen("\t(via\n", "\t(via blind\n", 1), FOUR);
    assert_eq!(code, 3, "{x}");
    fs::remove_dir_all(&dir).unwrap();
}
