//! `AET-WO-1211d`: pin devices drawn, tier 1 (no tool runs). The emitter is
//! driven at library level: the CLI's `emit` refuses a pin device
//! (`PinDeviceNotGenerated`) until `AET-WO-1211b` lifts it (ruled Q2).
//!
//! The fixtures are built here from `1211a`'s committed D1 and D2, so that no
//! committed specification and none of their bytes move:
//! - **D2 drawn** is D2 on a drawn board with each bridge input on a GPIO
//!   output (`AET-WO-1211` Q7). D2 as committed drives its inputs by PWM and
//!   is refused `PartLimitUndeclared` by `solve` itself, since SLVSAR1E states
//!   no input PWM maximum, so it cannot be drawn at any level.
//! - **D1 drawn** is D1 on a drawn board. It is refused `PassiveValueUnstated`
//!   for CVM2 (ruled S1), after every other part of it is drawn.
//!
//! The image runs (ERC, DRC with parity, `check support`, `extract`) are in
//! `spike/1211d/RESULTS.md`.

use std::collections::{BTreeMap, BTreeSet};

use aethelgard_eda::schematic::{self, Drawing, EmitError};
use aethelgard_ir::{PinTable, SolvedTopology, Specification};
use serde_json::{Value, json};

fn fixture(name: &str) -> Value {
    let path = format!(
        "{}/../aethelgard-ir/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

/// D2 on a drawn board, each bridge input a GPIO output that starts low.
fn d2_drawn() -> Value {
    let mut v = fixture("d2-drv8833.json");
    v["board"].as_object_mut().unwrap().remove("drawn");
    let inputs = [
        ("A_IN1", "AIN1"),
        ("A_IN2", "AIN2"),
        ("B_IN1", "BIN1"),
        ("B_IN2", "BIN2"),
    ];
    v["motor_outputs"]
        .as_array_mut()
        .unwrap()
        .retain(|m| !inputs.iter().any(|(net, _)| m["label"] == *net));
    let signals = v["pin_devices"][0]["signals"].as_array_mut().unwrap();
    for (net, pin) in inputs {
        signals.push(json!({"device_pin": pin, "net": net, "pin": null,
            "direction": "output", "initial": "low"}));
    }
    v
}

/// D1 on a drawn board.
fn d1_drawn() -> Value {
    let mut v = fixture("d1-drv8876.json");
    v["board"].as_object_mut().unwrap().remove("drawn");
    v
}

fn spec(v: &Value) -> Specification {
    serde_json::from_value(v.clone()).unwrap()
}

fn solved(v: &Value) -> (Specification, SolvedTopology, PinTable) {
    let spec = spec(v);
    spec.validate().unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    (spec, solved, table)
}

fn drawing_err(v: &Value) -> EmitError {
    let (spec, solved, table) = solved(v);
    match Drawing::of(&spec, &solved, &table) {
        Ok(_) => panic!("drawn"),
        Err(e) => e,
    }
}

/// SLVSAR1E p. 3, Pin Functions (HTSSOP): each pin, the net D2 declares for
/// it, or the rail. `U2_VCP` and `U2_VINT` are the emitter's own nets for
/// the capacitors p. 3 asks for; 17 is the PowerPAD, "must be connected to
/// ground".
const D2_PINS: [(&str, &str); 17] = [
    ("1", "DRV_NSLEEP"),
    ("2", "MA_OUT1"),
    ("3", "GND"),
    ("4", "MA_OUT2"),
    ("5", "MB_OUT2"),
    ("6", "GND"),
    ("7", "MB_OUT1"),
    ("8", "DRV_NFAULT"),
    ("9", "B_IN1"),
    ("10", "B_IN2"),
    ("11", "U2_VCP"),
    ("12", "VMOTOR"),
    ("13", "GND"),
    ("14", "U2_VINT"),
    ("15", "A_IN2"),
    ("16", "A_IN1"),
    ("17", "GND"),
];

/// SLVSAR1E p. 3 and §10.1 p. 15: the three capacitors, each between its two
/// nets, with the value and rating the page states (VM's rating is the VM
/// domain's maximum, 6 V + 5 %).
const D2_CAPS: [([&str; 2], &str); 3] = [
    (["GND", "VMOTOR"], "10u 6.3V"),
    (["U2_VCP", "VMOTOR"], "10n 16V X7R"),
    (["GND", "U2_VINT"], "2u2 6.3V"),
];

fn nets_of(d: &Drawing<'_>, reference: &str) -> BTreeSet<String> {
    d.drawn_nets().remove(reference).unwrap_or_default()
}

/// Every part drawn on exactly `nets`, with its value.
fn parts_on(d: &Drawing<'_>, nets: &[&str]) -> Vec<(String, String)> {
    let want: BTreeSet<String> = nets.iter().map(|n| (*n).to_owned()).collect();
    let values = d.drawn();
    d.drawn_nets()
        .into_iter()
        .filter(|(_, on)| *on == want)
        .map(|(r, _)| {
            let v = values[&r].clone();
            (r, v)
        })
        .collect()
}

#[test]
fn d2_is_drawn_pin_by_pin_from_its_row_and_its_declaration() {
    let (spec, solved, table) = solved(&d2_drawn());
    let d = Drawing::of(&spec, &solved, &table).unwrap();
    let u2 = d.pins("U2").unwrap();
    assert_eq!(u2.symbol, "Driver_Motor:DRV8833PWP");
    assert_eq!(
        u2.footprint,
        "Package_SO:HTSSOP-16-1EP_4.4x5mm_P0.65mm_EP3.4x5mm_Mask2.46x2.31mm"
    );
    assert_eq!(u2.value, "DRV8833PWPR");
    let want: BTreeMap<String, Option<String>> = D2_PINS
        .iter()
        .map(|(n, net)| ((*n).to_owned(), Some((*net).to_owned())))
        .collect();
    assert_eq!(u2.pins, want);
    for (nets, value) in D2_CAPS {
        let found = parts_on(&d, &nets);
        assert_eq!(found.len(), 1, "{nets:?}: {found:?}");
        assert_eq!(found[0].1, value, "{nets:?}");
        assert!(found[0].0.starts_with('C'), "{found:?}");
    }
    // nFAULT's pull is the MCU's (D2 declares `mcu: up`): no resistor.
    let pulls: Vec<_> = d
        .drawn_nets()
        .into_iter()
        .filter(|(r, on)| r.starts_with('R') && on.contains("DRV_NFAULT"))
        .collect();
    assert!(pulls.is_empty(), "{pulls:?}");
}

/// Each bridge output reaches the declared connector pin carrying its net
/// (J5 for bridge A, J6 for bridge B).
#[test]
fn each_bridge_output_reaches_its_declared_connector() {
    let (spec, solved, table) = solved(&d2_drawn());
    let d = Drawing::of(&spec, &solved, &table).unwrap();
    for (j, nets) in [
        ("J5", ["MA_OUT1", "MA_OUT2"]),
        ("J6", ["MB_OUT1", "MB_OUT2"]),
    ] {
        let on = nets_of(&d, j);
        for n in nets {
            assert!(on.contains(n), "{j}: {on:?}");
        }
    }
}

/// The WO's mutant: a bridge output to no connector. With J6 gone, D2 still
/// validates, and the emitter refuses to draw a motor output with nowhere to
/// go.
#[test]
fn a_bridge_output_on_no_connector_is_refused() {
    let mut v = d2_drawn();
    v["connectors"]
        .as_array_mut()
        .unwrap()
        .retain(|c| c["reference"] != "J6");
    let e = drawing_err(&v);
    assert_eq!(
        (e.kind(), e.id()),
        ("BridgeOutputNotOnConnector", Some("U2"))
    );
    assert!(e.to_string().contains("MB_OUT"), "{e}");
}

/// The WO's mutant: a strap left floating. `validate` refuses an undeclared
/// strap (`StrapUndeclared`); the emitter, given the declaration without it
/// after the solve, refuses by name rather than leaving the pin open.
#[test]
fn a_strap_missing_from_the_declaration_is_refused_never_floated() {
    let full = d2_drawn();
    let (_, solved, table) = solved(&full);
    let mut cut = full.clone();
    cut["pin_devices"][0]["straps"]
        .as_object_mut()
        .unwrap()
        .remove("BISEN");
    let spec = spec(&cut);
    let e = match Drawing::of(&spec, &solved, &table) {
        Ok(_) => panic!("drawn with BISEN floating"),
        Err(e) => e,
    };
    assert_eq!((e.kind(), e.id()), ("PinDeviceNotDrawable", Some("U2")));
    assert!(e.to_string().contains("`BISEN`"), "{e}");
}

/// The WO's mutant: nFAULT's pull-up missing. D2 with nFAULT pulled up by a
/// declared resistor instead of the MCU: it is drawn between nFAULT's net
/// and the rail it names, on its row's part and footprint.
#[test]
fn a_declared_nfault_pull_up_is_drawn_between_its_net_and_its_rail() {
    let mut v = d2_drawn();
    v["pin_devices"][0]["signals"][1]["pull"] = json!({
        "resistor": {"reference": "R9", "part": "CRCW0603_F_K", "ohms": 10000, "to": "VDD_3V3"},
        "source": "AET-WO-1211d test: a declared pull-up"
    });
    // A resistor is judged over a temperature (`TemperatureUndeclared`): D1's.
    v["pin_devices"][0]["temperature_celsius"] =
        fixture("d1-drv8876.json")["pin_devices"][0]["temperature_celsius"].clone();
    let (spec, solved, table) = solved(&v);
    let d = Drawing::of(&spec, &solved, &table).unwrap();
    let r9 = d.pins("R9").unwrap();
    assert_eq!(r9.footprint, "Resistor_SMD:R_0603_1608Metric");
    assert_eq!(r9.value, "10k CRCW0603_F_K");
    let on: BTreeSet<_> = r9.pins.values().flatten().cloned().collect();
    assert_eq!(on, BTreeSet::from(["DRV_NFAULT".into(), "VDD_3V3".into()]));
}

/// An open-drain output left undeclared is a no-connect (its pull is the
/// declaration's); a current output left undeclared is refused, since
/// SLVSDS7B p. 13's alternative, IPROPI tied to GND, cannot be declared.
#[test]
fn an_undeclared_nfault_is_open_and_an_undeclared_ipropi_is_refused() {
    let mut v = d2_drawn();
    v["pin_devices"][0]["signals"]
        .as_array_mut()
        .unwrap()
        .retain(|s| s["device_pin"] != "nFAULT");
    let (spec, solved, table) = solved(&v);
    let d = Drawing::of(&spec, &solved, &table).unwrap();
    assert_eq!(d.pins("U2").unwrap().pins["8"], None);

    let mut v = d1_drawn();
    v["pin_devices"][0]["analog"] = json!([]);
    let e = drawing_err(&v);
    assert_eq!((e.kind(), e.id()), ("PinDeviceNotDrawable", Some("U2")));
    assert!(e.to_string().contains("`IPROPI`"), "{e}");
}

/// Ruled S1: D1 is refused for CVM2, whose value SLVSDS7B does not state,
/// and for nothing else: the arm draws CVM2 last, after the device, its
/// declared resistors and reservoir and every other capacitor, so any other
/// refusal would come first.
#[test]
fn d1_is_refused_for_cvm2_alone() {
    let e = drawing_err(&d1_drawn());
    assert!(
        matches!(&e, EmitError::PassiveValueUnstated { reference, part, passive, .. }
            if reference == "U2" && part == "DRV8876PWPR" && passive == "CVM2"),
        "{e:?}"
    );
    assert!(e.to_string().contains("SLVSDS7B"), "{e}");
    // And emit, the one path a board takes, refuses it the same way.
    let (spec, solved, table) = solved(&d1_drawn());
    let e = schematic::emit(&spec, &solved, &table).unwrap_err();
    assert_eq!((e.kind(), e.id()), ("PassiveValueUnstated", Some("U2")));
}

/// Q1 (ruled): the pin devices' own nets are in `own_nets`, so `validate`
/// refuses a specification that spells one, and nothing else moved.
#[test]
fn the_pin_devices_own_nets_are_listed() {
    let own = aethelgard_ir::names::own_nets(&spec(&d2_drawn())).unwrap();
    for n in ["U2_VCP", "U2_VINT"] {
        assert!(own.contains(&n.to_owned()), "{n}: {own:?}");
    }
    let own = aethelgard_ir::names::own_nets(&spec(&d1_drawn())).unwrap();
    for n in ["U2_VCP", "U2_CPH", "U2_CPL"] {
        assert!(own.contains(&n.to_owned()), "{n}: {own:?}");
    }
}
