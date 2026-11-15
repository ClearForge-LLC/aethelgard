//! `AET-WO-4300g` §1.5: the support gate, tier 1. The reference board is
//! emitted, its netlist is written from the board's own pads in KiCad's
//! netlist format (the tier-2 test reads KiCad's real export), and each
//! mutation of §5 must fail the gate by name. The gate reads the MCU's row
//! and the specification, never the emitter's parts.

use std::collections::BTreeMap;

use aethelgard_eda::schematic;
use aethelgard_eda::stack::Stack;
use aethelgard_eda::{route, strict, support};
use aethelgard_ir::{PinTable, SolvedTopology, Specification};

const REFERENCE: &str = "examples/sentinel-01-minimal.json";

fn repo(path: &str) -> String {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    std::fs::read_to_string(format!("{root}/{path}")).unwrap()
}

fn reference() -> (Specification, SolvedTopology, PinTable) {
    let spec: Specification = serde_json::from_str(&repo(REFERENCE)).unwrap();
    spec.validate().unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    (spec, solved, table)
}

/// Each footprint's value and each pad's net, from the board text.
type Pads = (BTreeMap<String, String>, BTreeMap<(String, String), String>);

fn pads_of(board: &str) -> Pads {
    let tree = strict::board(board).unwrap();
    let mut values = BTreeMap::new();
    let mut nets = BTreeMap::new();
    for fp in tree.root().all("footprint") {
        let reference = fp.field("Reference").unwrap().unwrap().to_owned();
        values.insert(
            reference.clone(),
            fp.field("Value").unwrap().unwrap().to_owned(),
        );
        for pad in fp.all("pad") {
            let Some(net) = pad.one("net").unwrap().and_then(|n| n.last()) else {
                continue;
            };
            nets.insert(
                (reference.clone(), pad.arg(1).unwrap().to_owned()),
                net.to_owned(),
            );
        }
    }
    (values, nets)
}

/// A netlist in KiCad 10.0.7's `kicadsexpr` shape.
fn netlist_text((values, nets): &Pads) -> String {
    let mut by_net: BTreeMap<&str, Vec<(&str, &str)>> = BTreeMap::new();
    for ((r, p), n) in nets {
        by_net.entry(n).or_default().push((r, p));
    }
    let mut s = String::from("(export (version \"E\") (components\n");
    for (r, v) in values {
        s.push_str(&format!("(comp (ref \"{r}\") (value \"{v}\"))\n"));
    }
    s.push_str(")\n(nets\n");
    for (i, (n, nodes)) in by_net.iter().enumerate() {
        s.push_str(&format!("(net (code \"{}\") (name \"{n}\")", i + 1));
        for (r, p) in nodes {
            s.push_str(&format!(" (node (ref \"{r}\") (pin \"{p}\"))"));
        }
        s.push_str(")\n");
    }
    s.push_str("))\n");
    s
}

fn judge(spec: &Specification, pads: &Pads, board: &str) -> support::Record {
    let n = support::netlist(&strict::netlist(&netlist_text(pads)).unwrap()).unwrap();
    let tree = strict::board(board).unwrap();
    let (b, _) = route::read(&tree, &Stack::of(spec)).unwrap();
    support::judge(spec, &n, &b).unwrap()
}

fn kinds(r: &support::Record) -> Vec<&'static str> {
    r.findings.iter().map(|f| f.kind).collect()
}

/// The reference's own board: every role, strap, the programming path and
/// the source.
fn passing() -> (Specification, Pads, String, support::Record) {
    let (spec, solved, table) = reference();
    let e = schematic::emit(&spec, &solved, &table).unwrap();
    let pads = pads_of(&e.board);
    let r = judge(&spec, &pads, &e.board);
    (spec, pads, e.board, r)
}

/// The part serving `role` at `pin` on the reference.
fn part_at(r: &support::Record, role: &str, pin: &str) -> String {
    r.roles
        .iter()
        .find(|x| x.role == role && x.pin == pin)
        .and_then(|x| x.part.clone())
        .unwrap()
}

#[test]
fn the_reference_passes_the_support_gate() {
    let (_, _, _, r) = passing();
    assert!(r.findings.is_empty(), "{:?}", r.findings);
    assert_eq!(r.mcu, "U1");
    let count = |role: &str| r.roles.iter().filter(|x| x.role == role).count();
    assert_eq!(count("vdd_bypass"), 11);
    for role in [
        "vdd_bulk",
        "analog_bypass",
        "analog_bulk",
        "usb_bypass",
        "usb_bulk",
        "reset",
    ] {
        assert_eq!(count(role), 1, "{role}");
    }
    assert_eq!(count("vcap"), 2);
    for x in &r.roles {
        assert_eq!(x.verdict, "pass", "{x:?}");
        if let (Some(g), Some(b)) = (x.gap_um, x.bound_um) {
            assert!(g <= b, "{x:?}");
        }
    }
    assert_eq!(r.straps.len(), 4);
    let p = r.programming.as_ref().unwrap();
    assert_eq!((p.connector.as_str(), p.nrst), ("J1", false));
    assert_eq!(r.supply_source.as_deref(), Some("J2 pin 1"));
    assert!(
        r.conditions
            .iter()
            .any(|c| c.starts_with("reset placement"))
    );
}

/// §5.1: a per-pin capacitor removed from the netlist.
#[test]
fn a_removed_per_pin_capacitor_fails() {
    let (spec, mut pads, board, r) = passing();
    let c = part_at(&r, "vdd_bypass", "17");
    pads.0.remove(&c);
    pads.1.retain(|(reference, _), _| reference != &c);
    let out = judge(&spec, &pads, &board);
    assert_eq!(
        kinds(&out),
        ["support_capacitor_missing"],
        "{:?}",
        out.findings
    );
    assert!(
        out.findings[0].message.contains("VDD 17"),
        "{}",
        out.findings[0].message
    );
}

/// §5.1: a per-pin capacitor moved to another pin's net (VCAP1's).
#[test]
fn a_capacitor_on_another_pins_net_fails() {
    let (spec, mut pads, board, r) = passing();
    let c = part_at(&r, "vdd_bypass", "121");
    let vcap = pads.1[&("U1".to_owned(), "71".to_owned())].clone();
    for ((reference, _), net) in pads.1.iter_mut() {
        if reference == &c && net == &spec.mcu.supply {
            *net = vcap.clone();
        }
    }
    let out = judge(&spec, &pads, &board);
    assert!(
        kinds(&out).contains(&"support_capacitor_missing"),
        "{:?}",
        out.findings
    );
    assert!(
        out.findings.iter().any(|f| f.message.contains("VDD 121")),
        "{:?}",
        out.findings
    );
}

/// §5.1: a per-pin capacitor of 10 nF, outside 100 nF's class.
#[test]
fn a_10_nf_capacitor_fails() {
    let (spec, mut pads, board, r) = passing();
    let c = part_at(&r, "vdd_bypass", "84");
    pads.0.insert(c, "10n".to_owned());
    let out = judge(&spec, &pads, &board);
    assert_eq!(
        kinds(&out),
        ["support_capacitor_missing"],
        "{:?}",
        out.findings
    );
    assert!(out.findings[0].message.contains("VDD 84"));
}

/// A bounded capacitor moved off its pin, on the board: the netlist is the
/// same, and the distance fails it. An unbounded one (NRST's) moved the
/// same way still passes, named as a condition.
#[test]
fn a_capacitor_moved_off_its_pin_fails_and_an_unbounded_one_does_not() {
    let (spec, solved, table) = reference();
    let (_, _, plain, r) = passing();
    let tree = strict::board(&plain).unwrap();
    let (b, _) = route::read(&tree, &Stack::of(&spec)).unwrap();
    let away = (b.outline.x1 + 25_000_000, b.outline.y0);
    for (role, pin, fails) in [
        ("vdd_bypass", "52", true),
        ("vcap", "106", true),
        ("vdd_bulk", "VDD", true),
        ("reset", "25", false),
    ] {
        let c = part_at(&r, role, pin);
        let moved = schematic::emit_moved(&spec, &solved, &table, &[(c.clone(), away, 0)]).unwrap();
        let out = judge(&spec, &pads_of(&moved.board), &moved.board);
        if fails {
            assert_eq!(
                kinds(&out),
                ["support_capacitor_missing"],
                "{c}: {:?}",
                out.findings
            );
        } else {
            assert!(out.findings.is_empty(), "{c}: {:?}", out.findings);
        }
    }
}

/// A strap and a supply pin off their nets.
#[test]
fn a_strap_or_a_supply_pin_off_its_net_fails() {
    let (spec, pads, board, _) = passing();
    let mut boot = pads.clone();
    boot.1
        .insert(("U1".into(), "138".into()), spec.mcu.supply.clone());
    assert_eq!(kinds(&judge(&spec, &boot, &board)), ["strap_wrong"]);
    let mut vdd = pads.clone();
    vdd.1.insert(("U1".into(), "62".into()), "GND".into());
    let k = kinds(&judge(&spec, &vdd, &board));
    assert!(k.contains(&"support_pin_off_net"), "{k:?}");
}

/// §5.2: SWDIO and SWCLK swapped on the connector, or its GND dropped.
#[test]
fn a_connector_that_cannot_program_the_part_fails() {
    let (spec, pads, board, _) = passing();
    let mut swapped = pads.clone();
    let (four, five) = (
        ("J1".to_owned(), "4".to_owned()),
        ("J1".to_owned(), "5".to_owned()),
    );
    let (a, b) = (swapped.1[&four].clone(), swapped.1[&five].clone());
    swapped.1.insert(four, b);
    swapped.1.insert(five, a);
    let k = kinds(&judge(&spec, &swapped, &board));
    assert_eq!(
        k,
        [
            "connector_pin_off_net",
            "connector_pin_off_net",
            "no_programming_path"
        ]
    );
    let mut no_ground = pads.clone();
    no_ground.1.remove(&("J1".to_owned(), "6".to_owned()));
    let k = kinds(&judge(&spec, &no_ground, &board));
    assert_eq!(k, ["connector_pin_off_net", "no_programming_path"]);
}

/// The declared source's pin, off the supply in the netlist.
#[test]
fn a_source_pin_off_the_supply_fails() {
    let (spec, mut pads, board, _) = passing();
    pads.1.remove(&("J2".to_owned(), "1".to_owned()));
    let k = kinds(&judge(&spec, &pads, &board));
    assert_eq!(k, ["connector_pin_off_net", "supply_source_off_net"]);
}

/// A netlist the gate cannot judge from is refused, never judged in part.
#[test]
fn an_unjudgeable_netlist_is_refused() {
    let (spec, mut pads, board, _) = passing();
    let tree = strict::board(&board).unwrap();
    let (b, _) = route::read(&tree, &Stack::of(&spec)).unwrap();
    // No MCU of the declared part.
    pads.0.insert("U1".into(), "STM32F407".into());
    let n = support::netlist(&strict::netlist(&netlist_text(&pads)).unwrap()).unwrap();
    assert!(support::judge(&spec, &n, &b).is_err());
    // A pin on two nets, a node naming no component, another version.
    for bad in [
        "(export (version \"E\") (components (comp (ref \"C1\") (value \"1u\"))) (nets (net (code \"1\") (name \"A\") (node (ref \"C1\") (pin \"1\"))) (net (code \"2\") (name \"B\") (node (ref \"C1\") (pin \"1\")))))",
        "(export (version \"E\") (components) (nets (net (code \"1\") (name \"A\") (node (ref \"C9\") (pin \"1\")))))",
        "(export (version \"D\") (components) (nets))",
    ] {
        let tree = strict::netlist(bad).unwrap();
        assert!(support::netlist(&tree).is_err(), "{bad}");
    }
    // A child KiCad writes once, given twice, is refused by the reader.
    let twice = "(export (version \"E\") (components (comp (ref \"C1\") (ref \"C2\") (value \"1u\"))) (nets))";
    assert!(strict::netlist(twice).is_err());
}

/// One capacitor serves one pin. VDD pin 30's capacitor is within reach of
/// VDDA (pin 33) too, on the same two nets here, so with VDDA's own capacitor
/// removed a gate that let one capacitor serve two pins would still pass.
#[test]
fn one_capacitor_serves_one_pin() {
    let (spec, mut pads, board, r) = passing();
    let shared = part_at(&r, "vdd_bypass", "30");
    let tree = strict::board(&board).unwrap();
    let (b, _) = route::read(&tree, &Stack::of(&spec)).unwrap();
    let pad = |reference: &str, number: &str| {
        b.parts
            .iter()
            .find(|p| p.reference == reference)
            .unwrap()
            .pads
            .iter()
            .find(|p| p.number == number)
            .unwrap()
            .clone()
    };
    let reach = aethelgard_eda::loops::gap_nm(&pad(&shared, "1"), &pad("U1", "33"));
    assert!(
        reach <= 2_000_000,
        "{shared} is {reach} nm from pin 33: no test"
    );
    let own = part_at(&r, "analog_bypass", "33");
    pads.0.remove(&own);
    pads.1.retain(|(reference, _), _| reference != &own);
    let out = judge(&spec, &pads, &board);
    assert_eq!(
        kinds(&out),
        ["support_capacitor_missing"],
        "{:?}",
        out.findings
    );
}
