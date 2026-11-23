//! `AET-WO-4300g` §1.5: the support gate, tier 1. The reference board is
//! emitted, its netlist is written from the board's own pads in KiCad's
//! netlist format (the tier-2 test reads KiCad's real export), and each
//! mutation of §5 must fail the gate by name. The gate reads the MCU's row
//! and the specification, never the emitter's parts.

use aethelgard_eda::schematic;
use aethelgard_eda::stack::Stack;
use aethelgard_eda::{route, strict, support};
use aethelgard_ir::motor::PullFitment;
use aethelgard_ir::{PinTable, SolvedTopology, Specification};

#[path = "common/netlist.rs"]
mod netlist;
use netlist::{Pads, netlist_text, pads_of};

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

fn judge(spec: &Specification, pads: &Pads, board: &str) -> support::Record {
    let n = support::netlist(&strict::netlist(&netlist_text(pads)).unwrap()).unwrap();
    let tree = strict::board(board).unwrap();
    let (b, _) = route::read(&tree, &Stack::of(spec).unwrap()).unwrap();
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
    assert_eq!((p.connector.as_str(), p.nrst), ("J2", false));
    assert_eq!(r.supply_source.as_deref(), Some("J3 pin 1"));
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
    let (b, _) = route::read(&tree, &Stack::of(&spec).unwrap()).unwrap();
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
        ("J2".to_owned(), "4".to_owned()),
        ("J2".to_owned(), "5".to_owned()),
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
    no_ground.1.remove(&("J2".to_owned(), "6".to_owned()));
    let k = kinds(&judge(&spec, &no_ground, &board));
    assert_eq!(k, ["connector_pin_off_net", "no_programming_path"]);
}

/// The declared source's pin, off the supply in the netlist.
#[test]
fn a_source_pin_off_the_supply_fails() {
    let (spec, mut pads, board, _) = passing();
    pads.1.remove(&("J3".to_owned(), "1".to_owned()));
    let k = kinds(&judge(&spec, &pads, &board));
    assert_eq!(k, ["connector_pin_off_net", "supply_source_off_net"]);
}

/// A netlist the gate cannot judge from is refused, never judged in part.
#[test]
fn an_unjudgeable_netlist_is_refused() {
    let (spec, mut pads, board, _) = passing();
    let tree = strict::board(&board).unwrap();
    let (b, _) = route::read(&tree, &Stack::of(&spec).unwrap()).unwrap();
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
    let (b, _) = route::read(&tree, &Stack::of(&spec).unwrap()).unwrap();
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

/// A capacitor whose supply side stays by its pin but whose ground side is on
/// another net (VCAP1's) is between the wrong two nets: the nets alone fail
/// it, with its distance in bound.
#[test]
fn a_capacitor_whose_ground_side_is_on_another_net_fails() {
    let (spec, mut pads, board, r) = passing();
    let c = part_at(&r, "vdd_bypass", "62");
    let vcap = pads.1[&("U1".to_owned(), "71".to_owned())].clone();
    for ((reference, _), net) in pads.1.iter_mut() {
        if reference == &c && net == "GND" {
            *net = vcap.clone();
        }
    }
    let out = judge(&spec, &pads, &board);
    assert_eq!(
        kinds(&out),
        ["support_capacitor_missing"],
        "{:?}",
        out.findings
    );
    assert!(out.findings[0].message.contains("VDD 62"));
}

/// A ground pin off `GND`, and a powered pin (VDDA) on no declared domain.
#[test]
fn a_ground_or_powered_pin_off_its_net_fails() {
    let (spec, pads, board, _) = passing();
    let mut vss = pads.clone();
    vss.1
        .insert(("U1".into(), "38".into()), spec.mcu.supply.clone());
    let k = kinds(&judge(&spec, &vss, &board));
    assert!(k.contains(&"support_pin_off_net"), "{k:?}");
    let mut vdda = pads.clone();
    vdda.1.insert(("U1".into(), "33".into()), "/I2C_SDA".into());
    let out = judge(&spec, &vdda, &board);
    assert!(
        out.findings
            .iter()
            .any(|f| f.kind == "support_pin_off_net" && f.message.contains("VDDA")),
        "{:?}",
        out.findings
    );
}

/// Judges a netlist given as text, for edits the pad map cannot express.
fn judge_text(spec: &Specification, text: &str, board: &str) -> support::Record {
    let n = support::netlist(&strict::netlist(text).unwrap()).unwrap();
    let tree = strict::board(board).unwrap();
    let (b, _) = route::read(&tree, &Stack::of(spec).unwrap()).unwrap();
    support::judge(spec, &n, &b).unwrap()
}

/// Every pin carrying `from`, on `to` instead (a net renamed or merged).
fn renet(pads: &mut Pads, from: &str, to: &str) {
    for net in pads.1.values_mut() {
        if net == from {
            *net = to.to_owned();
        }
    }
}

/// Adversarial A1: NRST, a VCAP and the SWD lines each need a net of their
/// own. Moved onto the supply, onto GND or onto each other, with everything
/// on them moved too, each fails by name.
#[test]
fn a_reset_vcap_or_debug_net_that_is_not_its_own_fails() {
    let (spec, pads, board, _) = passing();
    for (from, to, group) in [
        ("/NRST", "VDD_3V3", "NRST"),
        ("/U1_VCAP1", "VDD_3V3", "VCAP"),
        ("/SWCLK", "/SWDIO", "SWCLK"),
        ("/SWDIO", "GND", "SWDIO"),
        ("/NRST", "/SWDIO", "NRST"),
    ] {
        let mut p = pads.clone();
        renet(&mut p, from, to);
        let out = judge(&spec, &p, &board);
        assert!(
            out.findings
                .iter()
                .any(|f| f.kind == "support_pin_off_net" && f.message.contains(group)),
            "{from} on {to}: {:?}",
            out.findings
        );
    }
}

/// Adversarial A2: a powered group on a declared domain outside its range
/// (VDDA and VREF+ on 5 V, above Table 111's 3.6 V) fails.
#[test]
fn a_powered_group_out_of_its_range_fails() {
    let (mut spec, mut pads, board, r) = passing();
    let mut five = spec.voltage_domains[0].clone();
    five.id = "VDD_5V".into();
    five.nominal_millivolts = 5000;
    spec.voltage_domains.push(five);
    for pin in ["32", "33"] {
        pads.1.insert(("U1".into(), pin.into()), "VDD_5V".into());
    }
    for role in ["analog_bypass", "analog_bulk"] {
        let pin = if role == "analog_bypass" {
            "33"
        } else {
            "VDDA"
        };
        let c = part_at(&r, role, pin);
        for ((reference, _), net) in pads.1.iter_mut() {
            if reference == &c && net == "VDD_3V3" {
                *net = "VDD_5V".into();
            }
        }
    }
    let out = judge(&spec, &pads, &board);
    assert!(
        out.findings
            .iter()
            .any(|f| f.kind == "support_pin_off_net" && f.message.contains("VDDA")),
        "{:?}",
        out.findings
    );
}

/// Adversarial A4: a capacitor marked unpopulated, drawn as another symbol,
/// or whose value carries more than a capacitance and a table part, serves
/// no role.
#[test]
fn an_unpopulated_or_mislabelled_capacitor_serves_no_role() {
    let (spec, pads, board, r) = passing();
    let c = part_at(&r, "vdd_bypass", "17");
    let text = netlist_text(&pads);
    let comp = format!("(comp (ref \"{c}\") (value \"100n\")");
    assert!(text.contains(&comp));
    for (edit, why) in [
        (format!("{comp} (property (name \"dnp\"))"), "dnp"),
        (
            format!("{comp} (property (name \"exclude_from_board\"))"),
            "off the board",
        ),
        (
            format!("{comp} (property (name \"exclude_from_bom\"))"),
            "off the BOM",
        ),
        (comp.replace("100n", "100n DNP"), "a value with DNP"),
    ] {
        let out = judge_text(&spec, &text.replace(&comp, &edit), &board);
        assert_eq!(
            kinds(&out),
            ["support_capacitor_missing"],
            "{why}: {:?}",
            out.findings
        );
    }
    let resistor = text.replace(
        &format!("{comp} (libsource (lib \"Device\") (part \"C\"))"),
        &format!("{comp} (libsource (lib \"Device\") (part \"R\"))"),
    );
    assert_ne!(resistor, text);
    let out = judge_text(&spec, &resistor, &board);
    assert_eq!(
        kinds(&out),
        ["support_capacitor_missing"],
        "{:?}",
        out.findings
    );
}

/// Adversarial A5: a connector pin declared not connected, on a net.
#[test]
fn a_not_connected_connector_pin_on_a_net_fails() {
    let (spec, mut pads, board, _) = passing();
    pads.1.insert(("J2".into(), "2".into()), "/SWCLK".into());
    let k = kinds(&judge(&spec, &pads, &board));
    assert!(k.contains(&"connector_pin_off_net"), "{k:?}");
}

// ---- step 8: each motor output's declared pull-down and link (`AET-WO-4300i`, `AET-WO-4300j`) ----

const MOTORS: &str = "crates/aethelgard-cli/tests/fixtures/motors-pwm.json";

/// The motors fixture emitted, its pads, its board, and MOTOR1's declared
/// pull-down (`R6`, 442 kOhm `CRCW0603_F_K`, to `GND`) and link (`R10`,
/// `CRCW0402_Z_0`), all four outputs `fitted`.
fn motors() -> (Specification, Pads, String, String) {
    let spec: Specification = serde_json::from_str(&repo(MOTORS)).unwrap();
    spec.validate().unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    let e = schematic::emit(&spec, &solved, &table).unwrap();
    let pads = pads_of(&e.board);
    let m = &spec.motor_outputs[0];
    assert_eq!(
        (
            m.label.as_str(),
            m.pull.resistor.reference.as_str(),
            m.pull.link.reference.as_str(),
            m.pull.fitment
        ),
        ("MOTOR1", "R6", "R10", PullFitment::Fitted)
    );
    (spec, pads, e.board, "R6".to_owned())
}

/// MOTOR1's link on the motors fixture.
const LINK: &str = "R10";

/// The motors fixture with every output `open`, and its own board, each
/// link marked `dnp` on it.
fn motors_open() -> (Specification, Pads, String) {
    let mut spec: Specification = serde_json::from_str(&repo(MOTORS)).unwrap();
    for m in &mut spec.motor_outputs {
        m.pull.fitment = PullFitment::Open;
    }
    spec.validate().unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    let e = schematic::emit(&spec, &solved, &table).unwrap();
    let pads = pads_of(&e.board);
    assert_eq!(
        pads.3.iter().map(String::as_str).collect::<Vec<_>>(),
        ["R10", "R11", "R12", "R13"]
    );
    (spec, pads, e.board)
}

/// Step 8's findings, in order, each with its message.
fn motor_findings(r: &support::Record) -> Vec<(&'static str, &str)> {
    r.findings
        .iter()
        .filter(|f| f.kind.starts_with("motor_"))
        .map(|f| (f.kind, f.message.as_str()))
        .collect()
}

fn motor_kinds(r: &support::Record) -> Vec<&'static str> {
    motor_findings(r).iter().map(|x| x.0).collect()
}

/// Every drawn motor pull and link on the fixture passes in both fitments,
/// the gate reading only the netlist and the declaration: no finding of any
/// step.
#[test]
fn the_motor_fixtures_pulls_pass_the_support_gate() {
    let (spec, pads, board, _) = motors();
    assert_eq!(spec.motor_outputs.len(), 4);
    assert!(pads.3.is_empty(), "{:?}", pads.3);
    let r = judge(&spec, &pads, &board);
    assert!(r.findings.is_empty(), "{:?}", r.findings);
    let (spec, pads, board) = motors_open();
    let r = judge(&spec, &pads, &board);
    assert!(r.findings.is_empty(), "{:?}", r.findings);
}

/// A declared pull-down absent from the netlist, drawn as another symbol,
/// or marked unpopulated is missing, by name; nothing else is judged of it.
#[test]
fn a_motor_pull_absent_unpopulated_or_not_a_resistor_is_missing() {
    let (spec, mut pads, board, r6) = motors();
    let text = netlist_text(&pads);
    let comp = format!("(comp (ref \"{r6}\") (value \"442k CRCW0603_F_K\")");
    assert!(text.contains(&comp), "{text}");
    let lib = format!("{comp} (libsource (lib \"Device\") (part \"R\"))");
    assert!(text.contains(&lib));
    for (edit, why) in [
        (
            text.replace(&comp, &format!("{comp} (property (name \"dnp\"))")),
            "dnp",
        ),
        (
            text.replace(&lib, &lib.replace("(part \"R\")", "(part \"C\")")),
            "a capacitor's symbol",
        ),
    ] {
        let out = judge_text(&spec, &edit, &board);
        let f = motor_findings(&out);
        assert_eq!(
            f.iter().map(|x| x.0).collect::<Vec<_>>(),
            ["motor_pull_missing"],
            "{why}: {f:?}"
        );
        assert!(f[0].1.contains("MOTOR1") && f[0].1.contains("R6"), "{f:?}");
    }
    // The resistor stays fitted when its output is `open`.
    let (open, mut open_pads, open_board) = motors_open();
    open_pads.3.insert(r6.clone());
    assert_eq!(
        motor_kinds(&judge(&open, &open_pads, &open_board)),
        ["motor_pull_missing"]
    );
    pads.0.remove(&r6);
    pads.1.retain(|(r, _), _| r != &r6);
    let f = judge(&spec, &pads, &board);
    assert_eq!(motor_kinds(&f), ["motor_pull_missing"], "{:?}", f.findings);
}

/// The adversarial target (`AET-WO-4300i` §5, `AET-WO-4300j` §5): a drawn
/// pull-down whose net disagrees with the declaration. Its far end on the
/// supply, both ends on the pull net, its pull end moved to the motor net
/// (past the link), its ground end on a pin the part does not have or on
/// none, each fail by name, `motor_pull_off_net`.
#[test]
fn a_motor_pull_off_its_nets_fails_by_name() {
    let (spec, pads, board, r6) = motors();
    let pin = |p: &str| (r6.clone(), p.to_owned());
    assert_eq!(pads.1[&pin("1")], "/MOTOR1_PD");
    assert_eq!(pads.1[&pin("2")], "GND");
    let mut cases: Vec<(Pads, &str)> = Vec::new();
    let mut p = pads.clone();
    p.1.insert(pin("2"), "VDD_3V3".into());
    cases.push((p, "the far end on the supply"));
    let mut p = pads.clone();
    p.1.insert(pin("2"), "/MOTOR1_PD".into());
    cases.push((p, "both ends on the pull net"));
    let mut p = pads.clone();
    p.1.insert(pin("1"), "/MOTOR1".into());
    cases.push((p, "the resistor moved to the motor net"));
    let mut p = pads.clone();
    p.1.remove(&pin("2"));
    p.1.insert(pin("3"), "GND".into());
    cases.push((p, "the ground end on pin 3"));
    let mut p = pads.clone();
    p.1.remove(&pin("2"));
    cases.push((p, "the ground end on no net"));
    for (p, why) in cases {
        let out = judge(&spec, &p, &board);
        let f = motor_findings(&out);
        assert_eq!(
            f.iter().map(|x| x.0).collect::<Vec<_>>(),
            ["motor_pull_off_net"],
            "{why}: {f:?}"
        );
        assert!(f[0].1.contains("MOTOR1") && f[0].1.contains("R6"), "{f:?}");
    }
    // Moved to MOTOR2's pull net: MOTOR1's is off its net, and MOTOR2 has two.
    let mut p = pads.clone();
    p.1.insert(pin("1"), "/MOTOR2_PD".into());
    let out = judge(&spec, &p, &board);
    assert_eq!(
        motor_kinds(&out),
        ["motor_pull_off_net", "motor_pull_count"],
        "{:?}",
        out.findings
    );
}

/// A drawn pull-down whose value disagrees with the declaration: another
/// resistance, another part, or no part at all, each `motor_pull_value`.
#[test]
fn a_motor_pull_of_another_value_or_part_fails_by_name() {
    let (spec, pads, board, r6) = motors();
    for value in [
        "40k2 CRCW0603_F_K",
        "442 CRCW0603_F_K",
        "442k RC0603FR-07442KL",
        "442k",
        "442k  CRCW0603_F_K",
    ] {
        let mut p = pads.clone();
        p.0.insert(r6.clone(), value.into());
        let out = judge(&spec, &p, &board);
        let f = motor_findings(&out);
        assert_eq!(
            f.iter().map(|x| x.0).collect::<Vec<_>>(),
            ["motor_pull_value"],
            "{value}: {f:?}"
        );
        assert!(f[0].1.contains(value) && f[0].1.contains("442000"), "{f:?}");
    }
}

/// A drawn pull-down whose footprint disagrees with its part's row, or that
/// names none, fails by name, `motor_pull_footprint`.
#[test]
fn a_motor_pull_on_another_footprint_or_none_fails_by_name() {
    let (spec, pads, board, r6) = motors();
    assert_eq!(pads.2[&r6], "Resistor_SMD:R_0603_1608Metric");
    let mut other = pads.clone();
    other
        .2
        .insert(r6.clone(), "Resistor_SMD:R_0402_1005Metric".into());
    let mut none = pads.clone();
    none.2.remove(&r6);
    for (p, why) in [(other, "0402"), (none, "none")] {
        let out = judge(&spec, &p, &board);
        let f = motor_findings(&out);
        assert_eq!(
            f.iter().map(|x| x.0).collect::<Vec<_>>(),
            ["motor_pull_footprint"],
            "{why}: {f:?}"
        );
        assert!(f[0].1.contains(why), "{f:?}");
    }
}

/// A second part joining the pull net, or the motor net past the link, to
/// `GND` beside the declared pull-down is another value in parallel:
/// `motor_pull_count`, naming it, whatever its symbol. A part on the line
/// alone, or to the rail, is no second pull.
#[test]
fn a_second_resistor_beside_a_motor_pull_fails_by_name() {
    let (spec, pads, board, _) = motors();
    for from in ["/MOTOR1_PD", "/MOTOR1"] {
        for (r, v) in [("C99", "100n"), ("FB99", "600R"), ("JP99", "0")] {
            let mut other = pads.clone();
            other.0.insert(r.into(), v.into());
            other.1.insert((r.into(), "1".into()), from.into());
            other.1.insert((r.into(), "2".into()), "GND".into());
            let out = judge(&spec, &other, &board);
            let f = motor_findings(&out);
            assert_eq!(
                f.iter().map(|x| x.0).collect::<Vec<_>>(),
                ["motor_pull_count"],
                "{from} {r}: {f:?}"
            );
            assert!(f[0].1.contains(r) && f[0].1.contains("R6"), "{f:?}");
        }
    }
    let mut cap = pads.clone();
    cap.0.insert("C99".into(), "100n".into());
    cap.1.insert(("C99".into(), "1".into()), "/MOTOR1".into());
    cap.1.insert(("C99".into(), "2".into()), "VDD_3V3".into());
    assert!(
        motor_findings(&judge(&spec, &cap, &board)).is_empty(),
        "a capacitor to the rail is no second pull"
    );
    let mut pads = pads;
    pads.0.insert("R99".into(), "442k CRCW0603_F_K".into());
    pads.2
        .insert("R99".into(), "Resistor_SMD:R_0603_1608Metric".into());
    pads.1.insert(("R99".into(), "1".into()), "GND".into());
    pads.1
        .insert(("R99".into(), "2".into()), "/MOTOR1_PD".into());
    let out = judge(&spec, &pads, &board);
    let f = motor_findings(&out);
    assert_eq!(
        f.iter().map(|x| x.0).collect::<Vec<_>>(),
        ["motor_pull_count"],
        "{f:?}"
    );
    assert!(f[0].1.contains("R99") && f[0].1.contains("R6"), "{f:?}");
}

/// A resistor of another symbol (`R_Small`) in parallel with the pull counts
/// as a second path; a resistor from the line or the pull net to the rail (a
/// pull-up, a divider with the pull-down) is `motor_net_extra_resistor`,
/// and a resistor elsewhere is neither. The link, itself a `Device:R` on a
/// `Resistor_SMD` footprint, is never counted.
#[test]
fn another_resistor_on_a_motor_line_fails_by_name() {
    let (spec, pads, board, _) = motors();
    let with = |r: &str, from: &str, other: &str, symbol: &str, footprint: &str| {
        let mut p = pads.clone();
        p.0.insert(r.into(), "10k CRCW0603_F_K".into());
        p.2.insert(r.into(), footprint.into());
        p.1.insert((r.into(), "1".into()), from.into());
        p.1.insert((r.into(), "2".into()), other.into());
        let text = netlist_text(&p).replace(
            &format!("(comp (ref \"{r}\") (value \"10k CRCW0603_F_K\") (libsource (lib \"Device\") (part \"R\"))"),
            &format!("(comp (ref \"{r}\") (value \"10k CRCW0603_F_K\") (libsource (lib \"Device\") (part \"{symbol}\"))"),
        );
        assert!(
            text.contains(&format!("(part \"{symbol}\")")),
            "edit applied"
        );
        motor_findings(&judge_text(&spec, &text, &board))
            .iter()
            .map(|x| (x.0, x.1.to_owned()))
            .collect::<Vec<_>>()
    };
    let f = with(
        "R98",
        "/MOTOR1_PD",
        "GND",
        "R_Small",
        "Resistor_SMD:R_0603_1608Metric",
    );
    assert_eq!(
        f.iter().map(|x| x.0).collect::<Vec<_>>(),
        ["motor_pull_count"],
        "{f:?}"
    );
    assert!(f[0].1.contains("R98"), "{f:?}");
    // Each way the gate knows a resistor stands alone: the symbol family,
    // or the footprint class; on either net of the chain.
    for from in ["/MOTOR1", "/MOTOR1_PD"] {
        for (symbol, footprint) in [
            ("R", "Resistor_SMD:R_0603_1608Metric"),
            ("R_Small", "Other:Pad"),
            ("Odd", "Resistor_SMD:R_0603_1608Metric"),
        ] {
            let f = with("R99", from, "VDD_3V3", symbol, footprint);
            assert_eq!(
                f.iter().map(|x| x.0).collect::<Vec<_>>(),
                ["motor_net_extra_resistor"],
                "{from} {symbol}: {f:?}"
            );
            assert!(
                f[0].1.contains("R99") && f[0].1.contains("R6") && f[0].1.contains(LINK),
                "{f:?}"
            );
        }
    }
    assert!(
        with(
            "R99",
            "/MOTOR1",
            "/OTHER",
            "R",
            "Resistor_SMD:R_0603_1608Metric"
        )
        .iter()
        .any(|x| x.0 == "motor_net_extra_resistor")
    );
    assert!(
        with(
            "R99",
            "/OTHER",
            "VDD_3V3",
            "R",
            "Resistor_SMD:R_0603_1608Metric"
        )
        .is_empty()
    );
}

/// `AET-WO-4300j` D5: a declared link absent from the netlist is missing,
/// by name; the pull-down behind it is still judged.
#[test]
fn a_motor_link_absent_is_missing() {
    let (spec, mut pads, board, _) = motors();
    pads.0.remove(LINK);
    pads.1.retain(|(r, _), _| r != LINK);
    let f = judge(&spec, &pads, &board);
    let m = motor_findings(&f);
    assert_eq!(motor_kinds(&f), ["motor_link_missing"], "{:?}", f.findings);
    assert!(m[0].1.contains("MOTOR1") && m[0].1.contains(LINK), "{m:?}");
}

/// `AET-WO-4300j` §5: a drawn link whose nets disagree with the
/// declaration: across the motor net and `GND` (and so a second pull), on
/// two motors' nets, both ends on one net, or an end on a pin it lacks or
/// on none, each `motor_link_off_net`.
#[test]
fn a_motor_link_off_its_nets_fails_by_name() {
    let (spec, pads, board, _) = motors();
    let pin = |p: &str| (LINK.to_owned(), p.to_owned());
    assert_eq!(pads.1[&pin("1")], "/MOTOR1");
    assert_eq!(pads.1[&pin("2")], "/MOTOR1_PD");
    // On MOTOR2's nets it is also a resistor on MOTOR2's chain.
    let extra = ["motor_link_off_net", "motor_net_extra_resistor"].as_slice();
    let alone = ["motor_link_off_net"].as_slice();
    for (edit, why, want) in [
        (
            vec![(pin("2"), Some("/MOTOR2_PD"))],
            "its pull end on MOTOR2's",
            extra,
        ),
        (
            vec![(pin("1"), Some("/MOTOR1_PD"))],
            "both ends on the pull net",
            alone,
        ),
        (
            vec![(pin("1"), Some("/MOTOR2"))],
            "its motor end on MOTOR2",
            extra,
        ),
        (
            vec![(pin("2"), None), (pin("3"), Some("/MOTOR1_PD"))],
            "pin 3",
            alone,
        ),
        (vec![(pin("2"), None)], "an end on no net", alone),
    ] {
        let mut p = pads.clone();
        for (k, n) in edit {
            match n {
                Some(n) => p.1.insert(k, n.into()),
                None => p.1.remove(&k),
            };
        }
        let f = judge(&spec, &p, &board);
        let m = motor_findings(&f);
        assert_eq!(motor_kinds(&f), want, "{why}: {m:?}");
        let off = m.iter().find(|x| x.0 == "motor_link_off_net").unwrap();
        assert!(off.1.contains("MOTOR1") && off.1.contains(LINK), "{m:?}");
    }
    // Across the motor net and GND it is off its nets; the second-path
    // count leaves the declared link to its own finding.
    let mut p = pads.clone();
    p.1.insert(pin("2"), "GND".into());
    assert_eq!(
        motor_kinds(&judge(&spec, &p, &board)),
        ["motor_link_off_net"],
        "the link is excluded from the second-path count, and its own finding names it"
    );
}

/// `AET-WO-4300j` §5: a drawn link that is not the declared jumper: another
/// value, another symbol, or another footprint (the 0603 row, or none).
#[test]
fn a_motor_link_of_another_part_or_footprint_fails_by_name() {
    let (spec, pads, board, _) = motors();
    assert_eq!(pads.0[LINK], "0");
    assert_eq!(pads.2[LINK], "Resistor_SMD:R_0402_1005Metric");
    for value in ["10k", "0R", "0 CRCW0402_Z_0", ""] {
        let mut p = pads.clone();
        p.0.insert(LINK.into(), value.into());
        let f = judge(&spec, &p, &board);
        assert_eq!(motor_kinds(&f), ["motor_link_part"], "{value}");
    }
    let text = netlist_text(&pads);
    let lib =
        format!("(comp (ref \"{LINK}\") (value \"0\") (libsource (lib \"Device\") (part \"R\"))");
    assert!(text.contains(&lib), "{text}");
    let f = judge_text(
        &spec,
        &text.replace(&lib, &lib.replace("(part \"R\")", "(part \"C\")")),
        &board,
    );
    assert_eq!(motor_kinds(&f), ["motor_link_part"], "a capacitor's symbol");
    for (footprint, why) in [
        (Some("Resistor_SMD:R_0603_1608Metric"), "0603"),
        (None, "none"),
    ] {
        let mut p = pads.clone();
        match footprint {
            Some(fp) => p.2.insert(LINK.into(), fp.into()),
            None => p.2.remove(LINK),
        };
        let f = judge(&spec, &p, &board);
        let m = motor_findings(&f);
        assert_eq!(motor_kinds(&f), ["motor_link_footprint"], "{why}: {m:?}");
        assert!(m[0].1.contains(why) && m[0].1.contains(LINK), "{m:?}");
    }
}

/// `AET-WO-4300j` D7: the link's fitment follows the declaration. A
/// `fitted` output's link marked `dnp`, or taken off the BOM or the board,
/// fails; an `open` output's link populated, or marked anything but `dnp`
/// alone, fails; each `motor_link_fitment`, naming the link and the
/// declared fitment.
#[test]
fn a_motor_links_fitment_that_disagrees_with_the_declaration_fails_by_name() {
    let (spec, pads, board, _) = motors();
    let comp = format!("(comp (ref \"{LINK}\") (value \"0\")");
    let text = netlist_text(&pads);
    assert!(text.contains(&comp));
    for mark in ["dnp", "exclude_from_bom", "exclude_from_board"] {
        let edit = text.replace(&comp, &format!("{comp} (property (name \"{mark}\"))"));
        let f = judge_text(&spec, &edit, &board);
        let m = motor_findings(&f);
        assert_eq!(
            motor_kinds(&f),
            ["motor_link_fitment"],
            "fitted, {mark}: {m:?}"
        );
        assert!(
            m[0].1.contains(LINK) && m[0].1.contains("`fitted`") && m[0].1.contains(mark),
            "{m:?}"
        );
    }
    let (open, open_pads, open_board) = motors_open();
    // Populated.
    let mut p = open_pads.clone();
    p.3.remove(LINK);
    let f = judge(&open, &p, &open_board);
    let m = motor_findings(&f);
    assert_eq!(motor_kinds(&f), ["motor_link_fitment"], "{m:?}");
    assert!(m[0].1.contains(LINK) && m[0].1.contains("`open`"), "{m:?}");
    // `dnp` and something more.
    let text = netlist_text(&open_pads);
    let marked = format!(
        "{comp} (libsource (lib \"Device\") (part \"R\")) (footprint \"Resistor_SMD:R_0402_1005Metric\") (property (name \"dnp\"))"
    );
    assert!(text.contains(&marked), "{text}");
    for extra in ["exclude_from_bom", "exclude_from_board"] {
        let edit = text.replace(&marked, &format!("{marked} (property (name \"{extra}\"))"));
        assert_eq!(
            motor_kinds(&judge_text(&open, &edit, &open_board)),
            ["motor_link_fitment"],
            "open, dnp and {extra}"
        );
    }
}

/// `AET-WO-4300j` §5: a second part across the link (a resistor, a
/// capacitor or a jumper between the motor net and the pull net) bypasses
/// it: `motor_link_count`, naming it, in either fitment.
#[test]
fn a_second_part_across_a_motor_link_fails_by_name() {
    let (spec, pads, board, _) = motors();
    let (open, open_pads, open_board) = motors_open();
    for (r, v) in [("R99", "0"), ("C99", "100n"), ("JP99", "0")] {
        for (spec, pads, board) in [(&spec, &pads, &board), (&open, &open_pads, &open_board)] {
            let mut p = pads.clone();
            p.0.insert(r.into(), v.into());
            p.1.insert((r.into(), "1".into()), "/MOTOR1".into());
            p.1.insert((r.into(), "2".into()), "/MOTOR1_PD".into());
            let f = judge(spec, &p, board);
            let m = motor_findings(&f);
            assert_eq!(motor_kinds(&f), ["motor_link_count"], "{r}: {m:?}");
            assert!(m[0].1.contains(r) && m[0].1.contains(LINK), "{m:?}");
        }
    }
}
