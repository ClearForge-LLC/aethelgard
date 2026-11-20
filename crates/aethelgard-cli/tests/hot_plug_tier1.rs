//! `AET-WO-1315`: the emitter draws what the power tree declares. Each
//! regulator's and translator's control pin as its `controls` entry says,
//! never by a rule of the emitter's own (§1.4).
//!
//! Judged from the placed board (`schematic::placement`, no routing), where
//! each pad's net is the copper's, and from the parts table's pin roles,
//! never from the emitter's part list (`INV-07`). Refusals of input that
//! `validate` would refuse first are reached through `schematic::anchored`,
//! which draws without validating, as an unvalidated caller would.

#![allow(clippy::unwrap_used)]

mod common;

use aethelgard_eda::schematic::{self, EmitError};
use aethelgard_eda::stack::Stack;
use aethelgard_eda::{route, strict};
use aethelgard_ir::{PinTable, SolvedTopology, Specification};
use serde_json::{Value, json};

/// The committed power fixture, crossing and all.
fn full() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../aethelgard-ir/tests/fixtures/sentinel-02-power.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

/// `v` validated and solved.
fn solved(v: &Value) -> (Specification, SolvedTopology, PinTable) {
    let spec: Specification = serde_json::from_value(v.clone()).unwrap();
    spec.validate().unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    (spec, solved, table)
}

/// The placed board of `v`, read back strictly.
fn placed(v: &Value) -> route::Board {
    let (spec, solved, table) = solved(v);
    let board = schematic::placement(&spec, &solved, &table).unwrap();
    let (b, _) = route::read(&strict::board(&board).unwrap(), &Stack::of(&spec).unwrap()).unwrap();
    b
}

/// The net of `reference`'s pad `pad` on `b`.
fn net<'a>(b: &'a route::Board, reference: &str, pad: &str) -> Option<&'a str> {
    let p = b.parts.iter().find(|p| p.reference == reference).unwrap();
    let pads: Vec<_> = p.pads.iter().filter(|x| x.number == pad).collect();
    assert!(!pads.is_empty(), "{reference} has no pad {pad}");
    pads[0].net.as_deref()
}

/// The pad of `role` in `part`'s row of the power parts table.
fn pad(part: &str, role: &str) -> String {
    aethelgard_ir::power_parts::table()
        .unwrap()
        .get(part)
        .unwrap()
        .pins[role]
        .clone()
}

/// The `controls` list of the regulator or crossing named `reference`.
fn controls<'a>(v: &'a mut Value, reference: &str) -> &'a mut Value {
    let p = &mut v["power"];
    let owner = ["regulators", "crossings"]
        .into_iter()
        .flat_map(|k| {
            p[k].as_array()
                .unwrap()
                .iter()
                .cloned()
                .enumerate()
                .map(move |(i, x)| (k, i, x))
        })
        .find(|(_, _, x)| x["reference"] == reference)
        .map(|(k, i, _)| (k, i))
        .unwrap();
    &mut v["power"][owner.0][owner.1]["controls"]
}

fn set(v: &mut Value, reference: &str, pin: &str, entry: Value) {
    let list = controls(v, reference).as_array_mut().unwrap();
    let c = list.iter_mut().find(|c| c["pin"] == pin).unwrap();
    *c = entry;
}

/// `why` of the `PowerNotDrawable` that `result` must be, for `reference`.
fn not_drawable<T: std::fmt::Debug>(result: Result<T, EmitError>, reference: &str) -> String {
    match result {
        Err(EmitError::PowerNotDrawable { reference: r, why }) if r == reference => why,
        other => panic!("{reference}: {other:?}"),
    }
}

/// The committed fixture's declarations, drawn as declared: each buck's EN
/// on its input and PG open, the LDO's EN on its input, OE on VCCA (the
/// pins the emitter tied by its own rule before `AET-WO-1315`, unmoved).
#[test]
fn the_committed_declarations_are_drawn_as_declared() {
    let b = placed(&full());
    let buck = |r: &str, role: &str| net(&b, r, &pad("LMR33630ADDA", role));
    for u in ["U10", "U11"] {
        assert!(buck(u, "EN").is_some());
        assert_eq!(buck(u, "EN"), buck(u, "VIN"), "{u}");
        // KiCad's name for a pad on no net (`aethelgard_ir::names::UNCONNECTED`).
        assert!(buck(u, "PG").unwrap().starts_with("unconnected-"), "{u}");
    }
    let ldo = |role: &str| net(&b, "U12", &pad("TLV75518PDBV", role));
    assert!(ldo("EN").is_some());
    assert_eq!(ldo("EN"), ldo("IN"));
    let tx = |role: &str| net(&b, "U13", &pad("TXU0304PW", role));
    assert_eq!(tx("OE"), tx("VCCA"));
    assert_ne!(tx("OE"), tx("VCCB"));
}

/// OE strapped `vccb` is drawn on the translator's B rail, not its A rail
/// as the emitter's old rule tied it.
#[test]
fn oe_strapped_vccb_is_drawn_on_the_b_rail() {
    let mut v = full();
    set(
        &mut v,
        "U13",
        "OE",
        json!({"pin": "OE", "strap": "vccb", "wire": null}),
    );
    let b = placed(&v);
    let oe = net(&b, "U13", &pad("TXU0304PW", "OE"));
    assert_eq!(oe, net(&b, "U13", &pad("TXU0304PW", "VCCB")));
    assert_ne!(oe, net(&b, "U13", &pad("TXU0304PW", "VCCA")));
}

/// A wire is drawn on its declared net when a connector pin the
/// specification declares ends it: U10's PG on `NRST`, beside the debug
/// header's pin 3. EN, still strapped `input`, stays on VIN.
#[test]
fn a_wire_to_a_connectors_net_is_drawn_on_it() {
    let mut v = common::power::power_tree();
    let j2 = v["connectors"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["reference"] == "J2")
        .unwrap();
    for p in j2["pins"].as_array_mut().unwrap() {
        if p["pin"] == "3" {
            p["net"] = json!("NRST");
        }
    }
    let wire = json!({"pin": "PG", "strap": null,
        "wire": {"net": "NRST", "role": "holds the MCU in reset until VDD_5V is good"}});
    set(&mut v, "U10", "PG", wire);
    let b = placed(&v);
    let pg = net(&b, "U10", &pad("LMR33630ADDA", "PG"));
    assert!(pg.is_some());
    assert_eq!(pg, net(&b, "J2", "3"));
    assert_ne!(pg, net(&b, "U10", &pad("LMR33630ADDA", "EN")));
    assert_eq!(
        net(&b, "U10", &pad("LMR33630ADDA", "EN")),
        net(&b, "U10", &pad("LMR33630ADDA", "VIN"))
    );
}

/// A wire to a net no declared connector pin or test point ends would be a
/// floating pin with a name: refused, naming the pin and the net.
#[test]
fn a_wire_to_a_net_nothing_else_ends_is_refused() {
    let mut v = common::power::power_tree();
    let wire =
        json!({"pin": "PG", "strap": null, "wire": {"net": "PG_5V", "role": "nothing reads it"}});
    set(&mut v, "U10", "PG", wire);
    let (spec, solved, table) = solved(&v);
    let why = not_drawable(schematic::placement(&spec, &solved, &table), "U10");
    assert!(why.contains("PG is wired to `PG_5V`"), "{why}");
    assert!(why.contains("floating pin"), "{why}");
}

/// Given to the emitter without `validate` (an unvalidated caller), each
/// declaration the emitter cannot draw as declared is refused by name; none
/// falls back to the old tie (EN on the input, PG open, OE on VCCA).
#[test]
fn a_control_the_emitter_cannot_draw_as_declared_is_refused_by_name() {
    let base = full();
    let (_, solved, table) = solved(&base);
    let cases: [(&str, &str, Option<Value>, &str); 9] = [
        (
            "U10",
            "PG",
            None,
            "PG is declared neither strapped nor wired",
        ),
        (
            "U12",
            "EN",
            None,
            "EN is declared neither strapped nor wired",
        ),
        (
            "U13",
            "OE",
            None,
            "OE is declared neither strapped nor wired",
        ),
        (
            "U10",
            "EN",
            Some(json!({"pin": "EN", "strap": "vcca", "wire": null})),
            "EN is strapped `vcca`, which its row does not allow or this part has no rail for",
        ),
        (
            "U10",
            "EN",
            Some(json!({"pin": "EN", "strap": "open", "wire": null})),
            "EN is strapped `open`, which its row does not allow",
        ),
        (
            "U13",
            "OE",
            Some(json!({"pin": "OE", "strap": "input", "wire": null})),
            "OE is strapped `input`, which its row does not allow or this part has no rail for",
        ),
        (
            "U10",
            "EN",
            Some(json!({"pin": "EN", "strap": null, "wire": {"net": "VDD_3V3", "role": "r"}})),
            "EN is wired to `VDD_3V3`, a rail",
        ),
        (
            "U10",
            "EN",
            Some(json!({"pin": "EN", "strap": "input", "wire": {"net": "SWDIO", "role": "r"}})),
            "EN is declared both strapped and wired, or neither",
        ),
        (
            "U10",
            "EN",
            Some(json!({"pin": "VIN", "strap": "input", "wire": null})),
            "`controls` names `VIN`",
        ),
    ];
    for (reference, pin, entry, want) in cases {
        let mut v = base.clone();
        let list = controls(&mut v, reference).as_array_mut().unwrap();
        let at = list.iter().position(|c| c["pin"] == pin).unwrap();
        match entry {
            None => {
                list.remove(at);
            }
            Some(e) => list[at] = e,
        }
        let spec: Specification = serde_json::from_value(v).unwrap();
        let why = not_drawable(schematic::anchored(&spec, &solved, &table), reference);
        assert!(why.contains(want), "{reference} {pin}: {why}");
    }
}

/// Every footprint on a board: its reference and its value.
fn values(board: &str) -> Vec<(String, String)> {
    board
        .split("\n\t(footprint ")
        .skip(1)
        .map(|f| {
            let prop = |name: &str| {
                let key = format!("(property \"{name}\" \"");
                let at = f.find(&key).unwrap() + key.len();
                f[at..].split('"').next().unwrap().to_owned()
            };
            (prop("Reference"), prop("Value"))
        })
        .collect()
}

/// Every `reference` and `part` pair under the specification's `power`,
/// read from the JSON itself.
fn declared_parts(v: &Value, out: &mut Vec<(String, String)>) {
    match v {
        Value::Object(m) => {
            if let (Some(Value::String(r)), Some(Value::String(p))) =
                (m.get("reference"), m.get("part"))
            {
                out.push((r.clone(), p.clone()));
            }
            m.values().for_each(|x| declared_parts(x, out));
        }
        Value::Array(a) => a.iter().for_each(|x| declared_parts(x, out)),
        _ => {}
    }
}

fn sentinel_03() -> Value {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sentinel-03.json");
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

/// §2, the parity test: every reference `power::references` lists is on
/// the placed board, valued with the part number the specification declares
/// for it, on both committed power boards. The pairs are read from the
/// specification, never from the emitter's part list (`INV-07`).
#[test]
fn every_declared_power_reference_is_on_the_board_with_its_part() {
    for v in [full(), sentinel_03()] {
        let (spec, solved, table) = solved(&v);
        let mut parts = Vec::new();
        declared_parts(&v["power"], &mut parts);
        let listed: Vec<&String> = aethelgard_ir::power::references(spec.power.as_ref().unwrap());
        let mut a: Vec<&String> = parts.iter().map(|(r, _)| r).collect();
        let mut b = listed.clone();
        a.sort();
        b.sort();
        assert_eq!(a, b, "{}", spec.project_name);
        assert!(listed.iter().any(|r| *r == "C1"));
        let on_board = values(&schematic::placement(&spec, &solved, &table).unwrap());
        for (reference, part) in &parts {
            let found: Vec<_> = on_board.iter().filter(|(r, _)| r == reference).collect();
            assert_eq!(found.len(), 1, "{}: {reference}", spec.project_name);
            assert!(
                found[0].1.split(' ').any(|w| w == part),
                "{}: {reference} is valued {:?}, not {part}",
                spec.project_name,
                found[0].1
            );
        }
    }
}

/// §2: on both power boards the declared bulk capacitor is `C1`, its
/// positive pad on `VBAT` and the other on `GND`, in the schematic, on the
/// board and in the bill of materials; no part the emitter numbers is `C1`.
#[test]
fn the_declared_bulk_capacitor_is_c1_positive_on_vbat_on_both_power_boards() {
    for v in [common::power::power_tree(), sentinel_03()] {
        let (spec, solved, table) = solved(&v);
        let e = schematic::emit(&spec, &solved, &table).unwrap();
        let name = &spec.project_name;
        let bom: Vec<&str> = e.bom.lines().filter(|l| l.starts_with("C1,")).collect();
        assert_eq!(
            bom,
            ["C1,47u UCZ1H470MCL1GS,Capacitor_SMD:CP_Elec_8x10,Device:C_Polarized,"],
            "{name}"
        );
        let sch: Vec<&str> = e
            .schematic
            .split("\n\t(symbol\n")
            .skip(1)
            .filter(|s| s.contains("(property \"Reference\" \"C1\""))
            .collect();
        assert_eq!(sch.len(), 1, "{name}");
        assert!(sch[0].contains("(lib_id \"Device:C_Polarized\")"), "{name}");
        assert!(
            sch[0].contains("(property \"Value\" \"47u UCZ1H470MCL1GS\""),
            "{name}"
        );
        let (b, _) = route::read(
            &strict::board(&e.board).unwrap(),
            &Stack::of(&spec).unwrap(),
        )
        .unwrap();
        let c1: Vec<_> = b.parts.iter().filter(|p| p.reference == "C1").collect();
        assert_eq!(c1.len(), 1, "{name}");
        let nets: Vec<(&str, Option<&str>)> = c1[0]
            .pads
            .iter()
            .map(|p| (p.number.as_str(), p.net.as_deref()))
            .collect();
        assert_eq!(nets, [("1", Some("VBAT")), ("2", Some("GND"))], "{name}");
    }
}

/// §1.3: the bulk capacitor is the last part of the first battery-fed
/// regulator's sequence, anchored to it; with no regulator fed from the
/// battery it is drawn by its nets alone, anchored to nothing.
#[test]
fn the_bulk_capacitor_anchors_to_the_first_battery_fed_regulator_or_to_nothing() {
    let v = full();
    let (spec, solved, table) = solved(&v);
    let anchored = schematic::anchored(&spec, &solved, &table).unwrap();
    assert_eq!(anchored.get("C1").map(String::as_str), Some("U10"));
    // No regulator fed from the battery: each battery-fed one moved onto
    // `VDD_3V3` (drawn without `validate`, which judges the tree).
    let mut v = full();
    for r in v["power"]["regulators"].as_array_mut().unwrap() {
        if r["input"] == "battery" {
            r["input"] = json!("VDD_3V3");
        }
    }
    let spec: Specification = serde_json::from_value(v).unwrap();
    let drawing = schematic::Drawing::of(&spec, &solved, &table).unwrap();
    assert_eq!(drawing.anchored().get("C1"), None);
    assert_eq!(
        drawing.drawn().get("C1").map(String::as_str),
        Some("47u UCZ1H470MCL1GS")
    );
}

/// §1.1, §5: a hot-plug reference the emitter could not hold is refused by
/// `validate` with the emitter's own kind and id: KiCad's `#` prefix, a
/// case-only twin of the `C1` the emitter would otherwise number, a
/// declared device's reference, and a test point's.
#[test]
fn a_hot_plug_reference_the_emitter_refuses_is_refused_at_validate() {
    let valid: Specification = serde_json::from_value(common::power::power_tree()).unwrap();
    let table = aethelgard_svd_hal::pin_table(&valid.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&valid, &table, &"0".repeat(64)).unwrap();
    // (name, validate's kind and id, the emitter's kind: a test point's
    // reference is `validate`'s own refusal, `AET-WO-1312`)
    for (name, kind, id, emitter) in [
        ("#C1", "ReservedName", "#C1", "ReservedName"),
        (
            "c1",
            "DuplicateReference",
            "C1 and c1",
            "DuplicateReference",
        ),
        ("C10", "DuplicateReference", "C10", "DuplicateReference"),
        (
            "TP9",
            "TestPointReferenceTaken",
            "TP9",
            "DuplicateReference",
        ),
    ] {
        let mut v = common::power::power_tree();
        v["power"]["input"]["hot_plug"][0]["reference"] = name.into();
        v["test_points"] = json!([{"net": "GND", "reference": "TP9"}]);
        let spec: Specification = serde_json::from_value(v).unwrap();
        let refused = spec.validate().unwrap_err();
        assert_eq!(refused.kind(), kind, "{name}: {refused}");
        assert_eq!(refused.id(), Some(id), "{name}: {refused}");
        let e = schematic::anchored(&spec, &solved, &table).unwrap_err();
        assert_eq!(e.kind(), emitter, "{name}: {e}");
    }
}
