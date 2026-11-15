//! Off-board connectors (`AET-WO-4300f`), tier 1: no tool runs. What KiCad
//! says about the emitted board is tier 2 (`board_tier2.rs`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use aethelgard_eda::sexpr::{self, Sx};
use serde_json::Value;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/connectors.json")
}

fn spec() -> Value {
    serde_json::from_slice(&fs::read(fixture()).unwrap()).unwrap()
}

struct Dir(PathBuf);

impl Dir {
    fn new(tag: &str) -> Dir {
        let d = std::env::temp_dir().join(format!(
            "aet-4300f-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&d).unwrap();
        Dir(d)
    }

    fn write(&self, name: &str, v: &Value) -> PathBuf {
        let p = self.0.join(name);
        fs::write(&p, serde_json::to_vec_pretty(v).unwrap()).unwrap();
        p
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap()
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

fn emit(spec: &Path, out: &Path) -> Value {
    json_line(
        &run(&[
            "emit",
            "--file",
            spec.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ]),
        0,
    )
}

fn nm(s: &Sx, i: usize) -> i64 {
    sexpr::nm(s.arg(i).unwrap()).unwrap()
}

/// The emitted board: its text, parsed.
fn board(d: &Dir, name: &str) -> (String, Sx) {
    let text = fs::read_to_string(d.0.join(name).join("connectors.kicad_pcb")).unwrap();
    let pcb = sexpr::parse(&text).unwrap();
    (text, pcb)
}

fn footprint<'a>(pcb: &'a Sx, reference: &str) -> &'a Sx {
    pcb.children("footprint")
        .find(|f| f.property("Reference") == Some(reference))
        .unwrap_or_else(|| panic!("no footprint {reference}"))
}

fn pad<'a>(fp: &'a Sx, number: &str) -> Vec<&'a Sx> {
    fp.children("pad")
        .filter(|p| p.arg(1) == Some(number))
        .collect()
}

fn pad_net(p: &Sx) -> &str {
    p.child("net").unwrap().arg(1).unwrap()
}

fn no_connects(d: &Dir, name: &str) -> usize {
    let text = fs::read_to_string(d.0.join(name).join("connectors.kicad_sch")).unwrap();
    sexpr::parse(&text).unwrap().children("no_connect").count()
}

/// The emitted fixture: the bill of materials lists both connectors, a pin
/// with no net is left unconnected on the board and carries a no-connect
/// flag on the sheet, each mounting pad is on GND, and the outline's bottom
/// is below every other footprint's lowest pad.
#[test]
fn the_fixture_emits_its_connectors_at_the_bottom_edge() {
    let d = Dir::new("emit");
    let r = emit(&fixture(), &d.0.join("a"));
    assert_eq!(r["ok"], true, "{r}");

    let bom = fs::read_to_string(d.0.join("a/connectors.bom.csv")).unwrap();
    assert!(
        bom.lines().any(|l| l.starts_with("J1,SM04B-GHS-TB,")),
        "the BOM does not list J1 with value SM04B-GHS-TB: {bom}"
    );
    assert!(
        bom.lines().any(|l| l.starts_with("J2,SM06B-GHS-TB,")),
        "the BOM does not list J2 with value SM06B-GHS-TB: {bom}"
    );

    let (text, pcb) = board(&d, "a");
    let j1 = footprint(&pcb, "J1");
    let j2 = footprint(&pcb, "J2");
    let mp = pad(j1, "MP");
    assert_eq!(mp.len(), 2, "J1 does not have two MP pads");
    for p in mp {
        assert_eq!(pad_net(p), "GND", "a J1 MP pad is not on GND");
    }
    for p in pad(j2, "MP") {
        assert_eq!(pad_net(p), "GND", "a J2 MP pad is not on GND");
    }
    assert_eq!(
        pad_net(pad(j2, "5")[0]),
        "unconnected-(J2-Pin_5-Pad5)",
        "J2 pin 5, declared with no net, is not left unconnected on the board"
    );
    assert!(
        !text.contains("unconnected-(J1-"),
        "J1, every pin of which has a net, has an unconnected pad"
    );

    // The sheet flags that pin: one more no-connect than the same
    // specification with the pin on GND.
    let mut other = spec();
    let pin5 = &mut other["connectors"][1]["pins"][4];
    assert_eq!(pin5["pin"], "5", "the fixture's J2 pin 5 moved");
    pin5["net"] = "GND".into();
    let p = d.write("grounded.json", &other);
    emit(&p, &d.0.join("b"));
    assert_eq!(
        no_connects(&d, "a"),
        no_connects(&d, "b") + 1,
        "J2 pin 5 does not carry exactly one no-connect flag on the sheet"
    );

    // The outline's bottom is below every footprint's lowest pad, and the
    // connectors are the lowest footprints.
    let rect = pcb.child("gr_rect").unwrap();
    let bottom = nm(rect.child("end").unwrap(), 2);
    let lowest = |fp: &Sx| {
        let y = nm(fp.child("at").unwrap(), 2);
        fp.children("pad")
            .map(|p| {
                let (at, size) = (p.child("at").unwrap(), p.child("size").unwrap());
                y + nm(at, 2) + nm(size, 2) / 2
            })
            .max()
            .unwrap()
    };
    for fp in pcb.children("footprint") {
        let reference = fp.property("Reference").unwrap();
        assert!(
            lowest(fp) < bottom,
            "the outline's bottom is not below {reference}'s lowest pad"
        );
        if !reference.starts_with('J') {
            assert!(
                lowest(fp) < lowest(j1).min(lowest(j2)),
                "{reference} is below a connector"
            );
        }
    }
}

/// `extract` on the emitted board: a connector on an I2C line adds its
/// declared budget as the off-board term, and no `Ci` for a connector pin.
#[test]
fn extract_counts_each_connectors_declared_budget_and_no_ci() {
    let d = Dir::new("extract");
    emit(&fixture(), &d.0);
    let board = d.0.join("connectors.kicad_pcb");
    let out = run(&[
        "extract",
        "--board",
        board.to_str().unwrap(),
        "--spec",
        fixture().to_str().unwrap(),
    ]);
    let v = json_line(&out, 0);
    assert_eq!(v["verdict"], "pass", "{v}");
    assert_eq!(v["findings"], Value::Array(vec![]), "{v}");
    let lines = v["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 2, "{v}");
    for line in lines {
        let net = line["net"].as_str().unwrap();
        let j1_pad = match net {
            "I2C_SCL" => "J1.2",
            "I2C_SDA" => "J1.3",
            other => panic!("an unexpected line {other}"),
        };
        assert_eq!(
            line["terms"]["offboard"], 30_000,
            "{net}: the off-board term is not the declared budget"
        );
        let named = line["offboard"].as_array().unwrap();
        assert_eq!(
            named.len(),
            1,
            "{net}: the off-board list is not exactly one pad"
        );
        assert!(
            named[0]
                .as_str()
                .unwrap()
                .starts_with(&format!("{j1_pad}: 30000 fF (")),
            "{net}: the off-board list does not name {j1_pad} with its budget: {named:?}"
        );
        let pins = line["pins"].as_array().unwrap();
        assert!(
            pins.iter().all(|p| !p.as_str().unwrap().starts_with('J')),
            "{net}: a connector pad is counted as a pin with a Ci: {pins:?}"
        );
        assert_eq!(
            line["terms"]["pins"], 30_000,
            "{net}: pins' Ci is not three pins at 10 pF (the connector adds none)"
        );
    }
}

/// The digital twin reads only the solved topology: with the connectors
/// emptied, `solve` differs from the fixture's only in the specification's
/// hash.
#[test]
fn solve_does_not_see_the_connectors() {
    let d = Dir::new("solve");
    let mut bare = spec();
    bare["connectors"] = Value::Array(vec![]);
    let bare = d.write("bare.json", &bare);
    let solved = |p: &Path| {
        let mut v = json_line(&run(&["solve", "--file", p.to_str().unwrap()]), 0);
        let sha = v
            .as_object_mut()
            .unwrap()
            .remove("specification_sha256")
            .expect("solve names the specification's hash");
        (v, sha)
    };
    let (with, with_sha) = solved(&fixture());
    let (without, without_sha) = solved(&bare);
    assert_ne!(
        with_sha, without_sha,
        "the two specifications hash alike, so the twin was not given different input"
    );
    assert_eq!(
        with, without,
        "solve's output depends on the connectors, not only on the solved topology"
    );
}

/// A declared connector pin with a net and no off-board budget is refused
/// with the kind and the connector's reference.
#[test]
fn a_connector_pin_with_no_budget_is_refused_naming_the_connector() {
    let d = Dir::new("refuse");
    let mut bad = spec();
    let pin2 = &mut bad["connectors"][0]["pins"][1];
    assert_eq!(pin2["pin"], "2", "the fixture's J1 pin 2 moved");
    pin2["offboard"] = Value::Null;
    let p = d.write("bad.json", &bad);
    let out = run(&["validate", "--file", p.to_str().unwrap()]);
    let v = json_line(&out, 2);
    assert_eq!(v["ok"], false, "{v}");
    assert_eq!(
        v["error"]["kind"], "ConnectorBudgetMissing",
        "the refusal's kind is wrong: {v}"
    );
    assert_eq!(
        v["error"]["id"], "J1",
        "the refusal does not name the connector: {v}"
    );
}

/// With `connectors` empty the board has no connector footprint: the
/// reference board's emission is untouched by this work order (its hashes
/// are pinned by the tests that already hold them).
#[test]
fn without_connectors_the_board_has_no_connector_footprint() {
    let d = Dir::new("none");
    let mut bare = spec();
    bare["connectors"] = Value::Array(vec![]);
    let p = d.write("bare.json", &bare);
    emit(&p, &d.0.join("a"));
    let (text, _) = board(&d, "a");
    assert!(
        !text.contains("Connector_JST"),
        "a board with no connectors has a Connector_JST footprint"
    );
}
