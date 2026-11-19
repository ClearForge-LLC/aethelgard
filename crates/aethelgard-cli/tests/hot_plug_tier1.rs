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
    let cases: [(&str, &str, Option<Value>, &str); 8] = [
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
            "EN is strapped `vcca`, which its row does not allow",
        ),
        (
            "U13",
            "OE",
            Some(json!({"pin": "OE", "strap": "input", "wire": null})),
            "OE is strapped `input`, which its row does not allow",
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
