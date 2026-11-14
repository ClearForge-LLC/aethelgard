//! `aethelgard emit` and the DRC gate's preparation, without KiCad
//! (`AET-WO-1300` §1.4). What KiCad says about the output is tier 2
//! (`with_tools.rs`).

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use aethelgard_eda::sexpr;
use serde_json::{Value, json};

fn example(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

struct Dir(PathBuf);

impl Dir {
    fn new(tag: &str) -> Dir {
        let d = std::env::temp_dir().join(format!(
            "aet-emit-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&d).unwrap();
        Dir(d)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(args: &[&str], path: Option<&Path>) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_aethelgard"));
    c.args(args).stdin(Stdio::null());
    if let Some(p) = path {
        c.env("PATH", p);
    }
    c.output().unwrap()
}

fn json_line(out: &Output, code: i32) -> Value {
    let stdout = String::from_utf8(out.stdout.clone()).unwrap();
    assert_eq!(
        stdout.lines().count(),
        1,
        "stdout {stdout} stderr {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(out.status.code(), Some(code), "{v}");
    v
}

fn emit(spec: &Path, out: &Path) -> Output {
    run(
        &[
            "emit",
            "--file",
            spec.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ],
        None,
    )
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn sentinel_spec() -> Value {
    serde_json::from_slice(&fs::read(example("sentinel-01-minimal.json")).unwrap()).unwrap()
}

fn write_spec(d: &Dir, spec: &Value) -> PathBuf {
    let p = d.0.join("spec.json");
    fs::write(&p, serde_json::to_vec_pretty(spec).unwrap()).unwrap();
    p
}

#[test]
fn emit_writes_the_schematic_and_bom_and_two_runs_are_byte_identical() {
    let d = Dir::new("twice");
    let (a, b) = (d.0.join("a"), d.0.join("b/deeper"));
    let ra = json_line(&emit(&example("sentinel-01-minimal.json"), &a), 0);
    let rb = json_line(&emit(&example("sentinel-01-minimal.json"), &b), 0);
    assert_eq!(ra["ok"], true);
    assert_eq!(ra["project_name"], "sentinel-01");
    for name in ["sentinel-01.kicad_sch", "sentinel-01.bom.csv"] {
        let (x, y) = (
            fs::read(a.join(name)).unwrap(),
            fs::read(b.join(name)).unwrap(),
        );
        assert!(!x.is_empty());
        assert_eq!(x, y, "{name} differs between two emissions");
    }
    assert_eq!(ra["schematic"]["sha256"], rb["schematic"]["sha256"]);
    assert_eq!(
        ra["bill_of_materials"]["sha256"],
        rb["bill_of_materials"]["sha256"]
    );
    assert_eq!(
        ra["schematic"]["path"],
        a.join("sentinel-01.kicad_sch").to_str().unwrap()
    );
    assert_eq!(ra["specification_sha256"].as_str().unwrap().len(), 64);
}

#[test]
fn the_emitted_schematic_is_kicad_10_with_every_part_and_no_fp_filters() {
    let d = Dir::new("shape");
    json_line(&emit(&example("sentinel-01-minimal.json"), &d.0), 0);
    let text = fs::read_to_string(d.0.join("sentinel-01.kicad_sch")).unwrap();
    let sch = sexpr::parse(&text).unwrap();
    assert_eq!(sch.head(), Some("kicad_sch"));
    assert_eq!(
        sch.child("version").and_then(|v| v.arg(1)),
        Some("20260306")
    );
    assert!(!text.contains("ki_fp_filters"));
    let refs: Vec<&str> = sch
        .children("symbol")
        .filter_map(|s| s.property("Reference"))
        .filter(|r| !r.starts_with('#'))
        .collect();
    for want in ["U1", "U2", "U3", "U4", "R1", "C1"] {
        assert!(refs.contains(&want), "{want} missing from {refs:?}");
    }
    // Every UUID in the file is distinct.
    let mut uuids: Vec<&str> = text
        .match_indices("(uuid \"")
        .map(|(i, _)| &text[i + 7..i + 43])
        .collect();
    let n = uuids.len();
    uuids.sort();
    uuids.dedup();
    assert_eq!(uuids.len(), n);
    let bom = fs::read_to_string(d.0.join("sentinel-01.bom.csv")).unwrap();
    assert!(bom.starts_with("Reference,Value,Footprint,Symbol,Note\r\n"));
    assert!(
        bom.contains("U4,W25Q64JVSS,Package_SO:SOIC-8_5.3x5.3mm_P1.27mm,Memory_Flash:W25Q32JVSS,\"symbol Memory_Flash:W25Q32JVSS stands in for W25Q64JV"),
        "{bom}"
    );
    assert!(bom.contains("U2,BME280,Package_LGA:Bosch_LGA-8"), "{bom}");
}

#[test]
fn the_mcu_symbol_pins_match_st_pin_positions() {
    let table = aethelgard_svd_hal::pin_table("STM32H753ZITx").unwrap();
    let sym = aethelgard_eda::library::symbol("MCU_ST_STM32H7:STM32H753ZITx").unwrap();
    let pins = aethelgard_eda::schematic::pins(&sym).unwrap();
    // Every ST pin's package position is a symbol pin, and the names agree
    // but for the two analog-switch pins KiCad suffixes `_C`. The emitter
    // maps by position, so this is a witness, not a dependency.
    let mut renamed = Vec::new();
    for (name, pin) in &table.pins {
        let number = pin.position.to_string();
        let sym = pins
            .iter()
            .find(|p| p.number == number)
            .unwrap_or_else(|| panic!("{name}: no symbol pin {number}"));
        if &sym.name != name {
            renamed.push(format!("{name}->{}", sym.name));
        }
    }
    assert_eq!(renamed, ["PC2->PC2_C", "PC3->PC3_C"]);
    assert!(table.pins.len() >= 100, "{}", table.pins.len());
}

#[test]
fn an_unknown_part_and_an_unanswerable_address_are_refused() {
    let d = Dir::new("parts");
    let mut spec = sentinel_spec();
    spec["i2c_devices"][1]["part"] = json!("SHT31");
    let v = json_line(&emit(&write_spec(&d, &spec), &d.0.join("o")), 2);
    assert_eq!(v["error"]["kind"], "UnknownPart");
    assert_eq!(v["error"]["id"], "U3");
    let mut spec = sentinel_spec();
    spec["i2c_devices"][0]["address"] = json!(0x40);
    let v = json_line(&emit(&write_spec(&d, &spec), &d.0.join("o")), 2);
    assert_eq!(v["error"]["kind"], "UnsupportedAddress");
    assert_eq!(v["error"]["id"], "U2");
    assert!(!d.0.join("o").exists(), "nothing is written for a refusal");
}

#[test]
fn a_net_the_emitter_would_use_is_refused_by_name() {
    let d = Dir::new("reserved");
    let mut spec = sentinel_spec();
    spec["spi_devices"][0]["cs"]["net"] = json!("U2_SDO");
    let v = json_line(&emit(&write_spec(&d, &spec), &d.0.join("o")), 2);
    assert_eq!(v["error"]["kind"], "ReservedName");
    assert_eq!(v["error"]["id"], "U2_SDO");
    let mut spec = sentinel_spec();
    spec["voltage_domains"][0]["id"] = json!("GND");
    spec["mcu"]["supply"] = json!("GND");
    for list in ["i2c_buses", "spi_buses", "i2c_devices", "spi_devices"] {
        for item in spec[list].as_array_mut().unwrap() {
            let field = if list.ends_with("buses") {
                "domain"
            } else {
                "supply"
            };
            item[field] = json!("GND");
        }
    }
    let v = json_line(&emit(&write_spec(&d, &spec), &d.0.join("o")), 2);
    assert_eq!(v["error"]["kind"], "ReservedName", "{v}");
}

/// `AET-WO-1204` §1.3: names that differ only in case are refused, each error
/// naming both spellings. KiCad's ERC refuses such a pair.
#[test]
fn names_that_differ_only_in_case_from_the_emitters_are_refused() {
    let d = Dir::new("fold");
    let refused = |spec: &Value| json_line(&emit(&write_spec(&d, spec), &d.0.join("o")), 2);
    let has_both = |v: &Value, field: &str, a: &str, b: &str| {
        let text = v["error"][field].as_str().unwrap().to_owned();
        assert!(text.contains(a) && text.contains(b), "{a} / {b}: {v}");
        assert!(!d.0.join("o").exists(), "nothing is written for a refusal");
    };
    // A signal net beside the emitter's own `U1_VCAP1`.
    let mut spec = sentinel_spec();
    spec["spi_devices"][0]["cs"]["net"] = json!("u1_vcap1");
    let v = refused(&spec);
    assert_eq!(v["error"]["kind"], "ReservedName", "{v}");
    has_both(&v, "id", "u1_vcap1", "U1_VCAP1");
    // A device reference beside the MCU's `U1`.
    let mut spec = sentinel_spec();
    spec["i2c_devices"][1]["reference"] = json!("u1");
    let v = refused(&spec);
    assert_eq!(v["error"]["kind"], "DuplicateReference", "{v}");
    has_both(&v, "id", "u1", "U1");
    // `U2` beside `u2`: `validate` refuses it, naming the first spelling.
    let mut spec = sentinel_spec();
    spec["i2c_devices"][1]["reference"] = json!("u2");
    let v = refused(&spec);
    assert_eq!(v["error"]["kind"], "DuplicateReference", "{v}");
    assert_eq!(v["error"]["id"], "u2", "{v}");
    has_both(&v, "message", "u2", "U2");
}

#[test]
fn an_invalid_specification_is_refused_as_validate_refuses_it() {
    let d = Dir::new("invalid");
    let v = json_line(
        &emit(&example("invalid-zero-domain.json"), &d.0.join("o")),
        2,
    );
    assert_eq!(v["error"]["kind"], "ZeroNominalVoltage");
    let mut spec = sentinel_spec();
    spec["project_name"] = json!("a/b");
    let v = json_line(&emit(&write_spec(&d, &spec), &d.0.join("o")), 2);
    assert_eq!(v["error"]["kind"], "InvalidIdentifier");
    assert_eq!(v["error"]["path"], "project_name");
}

#[test]
fn an_out_that_cannot_hold_the_files_is_refused() {
    let d = Dir::new("out");
    let file = d.0.join("a-file");
    fs::write(&file, "x").unwrap();
    let v = json_line(&emit(&example("sentinel-01-minimal.json"), &file), 3);
    assert_eq!(v["error"]["kind"], "UnwritableOutput");
    // A symlink where the schematic goes is not followed.
    let out = d.0.join("o");
    fs::create_dir(&out).unwrap();
    let target = d.0.join("elsewhere");
    symlink(&target, out.join("sentinel-01.kicad_sch")).unwrap();
    let v = json_line(&emit(&example("sentinel-01-minimal.json"), &out), 3);
    assert_eq!(v["error"]["kind"], "UnwritableOutput");
    assert!(!target.exists(), "the symlink was followed");
}

// ---- the DRC gate's preparation, with a stand-in kicad-cli ----

/// A `kicad-cli` that checks the scratch copy the gate prepared, writes a
/// clean DRC report and exits 0; exit 9x names the check that failed.
fn checking_kicad(dir: &Path, checks: &str) -> PathBuf {
    let tb = dir.join("toolbox");
    fs::create_dir_all(&tb).unwrap();
    let script = format!(
        r#"#!/bin/sh
if [ "$1" = version ]; then echo 10.0.7; exit 0; fi
out=; prev=; last=; refill=
for a in "$@"; do
  if [ "$prev" = -o ]; then out=$a; fi
  if [ "$a" = --refill-zones ]; then refill=1; fi
  prev=$a; last=$a
done
d=$(dirname "$last")
{checks}
printf '%s' '{{"$schema":"https://schemas.kicad.org/drc.v1.json","coordinate_units":"mm","date":"x","included_severities":["error","warning"],"kicad_version":"10.0.7","source":"input.kicad_pcb","violations":[],"unconnected_items":[],"schematic_parity":[]}}' > "$out"
exit 0
"#
    );
    let stub = tb.join("kicad-cli");
    fs::write(&stub, script).unwrap();
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();
    tb
}

/// A board whose own rules are loose. With `--spec` it must also be the
/// declared board (`AET-WO-4100`: two copper layers, 1.6 mm), or the gate
/// refuses it before writing anything, so it carries the reference's layer
/// table and thickness; its subject is the rule files, not the board.
const LOOSE_BOARD: &str = "(kicad_pcb\n\t(version 20240108)\n\t(general\n\t\t(thickness 1.6)\n\t)\n\t(layers\n\t\t(0 \"F.Cu\" signal)\n\t\t(2 \"B.Cu\" signal)\n\t)\n\t(setup\n\t\t(pad_to_mask_clearance 0)\n\t\t(trace_clearance 0.05)\n\t\t(clearance_min 0.05)\n\t)\n\t(net 0 \"\")\n)\n";

#[test]
fn the_gate_writes_every_rule_and_library_file_and_reads_none_from_the_board() {
    let d = Dir::new("gate");
    let board = d.0.join("loose.kicad_pcb");
    fs::write(&board, LOOSE_BOARD).unwrap();
    // The board's own sidecars sit beside it; none may reach the judge.
    fs::write(
        d.0.join("loose.kicad_pro"),
        "{\"net_settings\":{\"classes\":[{\"name\":\"Default\",\"clearance\":0.05}]}}",
    )
    .unwrap();
    fs::write(
        d.0.join("loose.kicad_dru"),
        "(version 1)\n(rule own (constraint clearance (min 0.05mm)))\n",
    )
    .unwrap();
    let common = r#"
[ "$refill" = 1 ] || exit 90
[ -f "$d/input.kicad_pro" ] || exit 91
[ -f "$d/input.kicad_dru" ] || exit 92
grep -q 'clearance_min\|trace_clearance' "$last" && exit 93
grep -q '(pad_to_mask_clearance 0)' "$last" || exit 94
grep -q 'Package_QFP.pretty' "$d/fp-lib-table" || exit 95
[ -f "$d/lib/footprints/Package_QFP.pretty/LQFP-144_20x20mm_P0.5mm.kicad_mod" ] || exit 96
grep -q 'Device.kicad_symdir' "$d/sym-lib-table" || exit 97
grep -q '"lib_symbol_issues": "ignore"' "$d/input.kicad_pro" || exit 98
grep -q 'own' "$d/input.kicad_dru" && exit 89
ls "$d" | grep -q loose && exit 88
"#;
    let spec_checks = format!(
        "{common}grep -q 'specification_clearance' \"$d/input.kicad_dru\" || exit 81\ngrep -q 'track_width (min 0.2mm)' \"$d/input.kicad_dru\" || exit 82\ngrep -q '\"min_clearance\": 0.2,' \"$d/input.kicad_pro\" || exit 83"
    );
    let tb = checking_kicad(&d.0, &spec_checks);
    let b = board.to_str().unwrap();
    let spec = example("sentinel-01-minimal.json");
    let v = json_line(
        &run(
            &[
                "check",
                "drc",
                "--board",
                b,
                "--spec",
                spec.to_str().unwrap(),
            ],
            Some(&tb),
        ),
        0,
    );
    assert_eq!(v["rules"], "specification", "{v}");
    assert_eq!(v["specification_sha256"].as_str().unwrap().len(), 64);
    assert_eq!(v["input"]["sha256"], sha256_hex(LOOSE_BOARD.as_bytes()));

    let default_checks = format!(
        "{common}grep -q 'kicad_default_clearance' \"$d/input.kicad_dru\" || exit 84\ngrep -q 'track_width' \"$d/input.kicad_dru\" && exit 85\ngrep -q '\"min_clearance\": 0,' \"$d/input.kicad_pro\" || exit 86"
    );
    let tb = checking_kicad(&d.0.join("default"), &default_checks);
    let v = json_line(&run(&["check", "drc", "--board", b], Some(&tb)), 0);
    assert_eq!(v["rules"], "kicad-default", "{v}");
    assert!(v.get("specification_sha256").is_none(), "{v}");
    // The original is untouched.
    assert_eq!(fs::read_to_string(&board).unwrap(), LOOSE_BOARD);
}

#[test]
fn a_board_carrying_its_own_rule_is_refused_before_any_tool_runs() {
    let d = Dir::new("refuse");
    // No kicad-cli on PATH: the refusal must not depend on one.
    let empty = d.0.join("empty");
    fs::create_dir(&empty).unwrap();
    let cases = [
        (
            "(kicad_pcb (footprint \"R:R\" (property \"Reference\" \"R7\") (pad \"1\" smd rect (clearance 0.05))))",
            "LocalClearance",
            Some("pad 1 of footprint R7"),
        ),
        (
            "(kicad_pcb (footprint \"R:R\" (property \"Reference\" \"R7\") (clearance 0.05)))",
            "LocalClearance",
            Some("footprint R7"),
        ),
        (
            "(kicad_pcb (zone (net 1) (net_name \"GND\") (connect_pads (clearance 0.5))))",
            "LocalClearance",
            Some("zone on net GND"),
        ),
        (
            "(kicad_pcb (net_class Default \"\" (clearance 0.05)))",
            "BoardNetClass",
            Some("net class Default"),
        ),
        ("x", "MalformedBoard", None),
        ("(kicad_sch)", "MalformedBoard", None),
    ];
    for (text, kind, id) in cases {
        let board = d.0.join("b.kicad_pcb");
        fs::write(&board, text).unwrap();
        let v = json_line(
            &run(
                &["check", "drc", "--board", board.to_str().unwrap()],
                Some(&empty),
            ),
            3,
        );
        assert_eq!(v["error"]["kind"], kind, "{text}: {v}");
        assert_eq!(v["error"]["id"].as_str(), id, "{text}: {v}");
    }
}

#[test]
fn an_invalid_spec_for_check_drc_is_refused_as_validate_refuses_it() {
    let d = Dir::new("drcspec");
    let board = d.0.join("b.kicad_pcb");
    fs::write(&board, "(kicad_pcb (version 20240108))\n").unwrap();
    let v = json_line(
        &run(
            &[
                "check",
                "drc",
                "--board",
                board.to_str().unwrap(),
                "--spec",
                example("invalid-zero-domain.json").to_str().unwrap(),
            ],
            Some(&d.0),
        ),
        2,
    );
    assert_eq!(v["error"]["kind"], "ZeroNominalVoltage");
}

/// The emitted sheet's geometry, checked the way ERC would see it: every
/// wire runs from one pin to one label or power-symbol pin, no point of one
/// part touches another part's, and every net reaches two pins or more.
#[test]
fn every_connection_point_belongs_to_exactly_what_the_emitter_meant() {
    use std::collections::BTreeMap;
    let d = Dir::new("geometry");
    json_line(&emit(&example("sentinel-01-minimal.json"), &d.0), 0);
    let text = fs::read_to_string(d.0.join("sentinel-01.kicad_sch")).unwrap();
    let sch = sexpr::parse(&text).unwrap();
    let nm = |s: &sexpr::Sx, i: usize| sexpr::nm(s.arg(i).unwrap()).unwrap();
    let lib: BTreeMap<&str, &sexpr::Sx> = sch
        .child("lib_symbols")
        .unwrap()
        .children("symbol")
        .map(|s| (s.arg(1).unwrap(), s))
        .collect();
    // point -> what sits there: ("pin", reference), ("label", text), ("nc", "")
    let mut at: BTreeMap<(i64, i64), Vec<(String, String)>> = BTreeMap::new();
    let mut pin_net: BTreeMap<(i64, i64), String> = BTreeMap::new();
    for inst in sch.children("symbol") {
        let reference = inst.property("Reference").unwrap().to_owned();
        let pos = inst.child("at").unwrap();
        let (x, y) = (nm(pos, 1), nm(pos, 2));
        let sym = lib[inst.child("lib_id").unwrap().arg(1).unwrap()];
        let mut points: Vec<(i64, i64)> = aethelgard_eda::schematic::pins(sym)
            .unwrap()
            .iter()
            .map(|p| (x + p.at.0, y - p.at.1))
            .collect();
        points.sort();
        points.dedup();
        for p in points {
            at.entry(p)
                .or_default()
                .push(("pin".into(), reference.clone()));
            if reference.starts_with('#') {
                pin_net.insert(p, inst.property("Value").unwrap().to_owned());
            }
        }
    }
    for label in sch.children("label") {
        let pos = label.child("at").unwrap();
        let p = (nm(pos, 1), nm(pos, 2));
        at.entry(p)
            .or_default()
            .push(("label".into(), label.arg(1).unwrap().to_owned()));
        pin_net.insert(p, label.arg(1).unwrap().to_owned());
    }
    for nc in sch.children("no_connect") {
        let pos = nc.child("at").unwrap();
        at.entry((nm(pos, 1), nm(pos, 2)))
            .or_default()
            .push(("nc".into(), String::new()));
    }
    // A point may hold one part's pins, plus one label, or one power symbol,
    // or a no-connect; or, in the flag row, a power symbol and a PWR_FLAG.
    for (p, here) in &at {
        let parts: Vec<&String> = here
            .iter()
            .filter(|(k, _)| k == "pin")
            .map(|(_, r)| r)
            .collect();
        let real: Vec<&&String> = parts.iter().filter(|r| !r.starts_with('#')).collect();
        assert!(real.len() <= 1, "{p:?} holds pins of {parts:?}");
        let flagged = parts.iter().any(|r| r.starts_with("#FLG"));
        assert!(
            flagged || parts.len() <= 1 || here.iter().any(|(k, _)| k == "nc"),
            "{p:?}: {here:?}"
        );
        assert!(here.len() <= 2, "{p:?}: {here:?}");
    }
    // Every wire: one end on a real part's pin, the other on a label or a
    // power symbol's pin, and nothing else on either end.
    let mut members: BTreeMap<String, usize> = BTreeMap::new();
    for wire in sch.children("wire") {
        let pts: Vec<(i64, i64)> = wire
            .child("pts")
            .unwrap()
            .children("xy")
            .map(|xy| (nm(xy, 1), nm(xy, 2)))
            .collect();
        assert_eq!(pts.len(), 2);
        let start = &at[&pts[0]];
        assert!(
            start.iter().all(|(k, r)| k == "pin" && !r.starts_with('#')),
            "{start:?}"
        );
        let end = &at[&pts[1]];
        assert_eq!(end.len(), 1, "{end:?}");
        let net = pin_net
            .get(&pts[1])
            .unwrap_or_else(|| panic!("wire end {:?} on {end:?}", pts[1]));
        *members.entry(net.clone()).or_default() += 1;
    }
    for (net, n) in &members {
        assert!(*n >= 2, "net {net} reaches only {n} pin(s)");
    }
    for want in [
        "I2C_SDA", "I2C_SCL", "SPI_SCK", "SPI_MOSI", "SPI_MISO", "FLASH_CS", "VDD_3V3", "GND",
    ] {
        assert!(
            members.contains_key(want),
            "{want} missing from {members:?}"
        );
    }
}

/// A pin ST's table and KiCad's symbol name differently (`PC2` and `PC2_C`,
/// package pin 28) is drawn by its position, so it is not refused.
#[test]
fn a_solved_pin_is_drawn_by_its_package_position() {
    let d = Dir::new("pc2");
    let mut spec = sentinel_spec();
    spec["spi_buses"][0]["miso"]["pin"] = json!("PC2");
    let r = json_line(&emit(&write_spec(&d, &spec), &d.0.join("o")), 0);
    let text = fs::read_to_string(r["schematic"]["path"].as_str().unwrap()).unwrap();
    assert!(text.contains("(label \"SPI_MISO\""), "no SPI_MISO label");
}
