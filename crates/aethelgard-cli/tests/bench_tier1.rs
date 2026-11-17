//! `AET-WO-1312`, tier 1: the bench board's emitted parts, its support gate
//! (BOOT0's pull-down, the reset switch, each test point and connector pin
//! on its net) on a netlist written from the board's own pads in KiCad's
//! netlist format, `extract` with test pads on its lines, and the §5
//! adversarial cases. The tier-2 test reads KiCad's own netlist.

use std::collections::BTreeMap;

use aethelgard_eda::schematic;
use aethelgard_eda::stack::Stack;
use aethelgard_eda::{extract, route, strict, support};
use aethelgard_ir::{PinTable, SolvedTopology, Specification};
use serde_json::{Value, json};

const BENCH: &str = "crates/aethelgard-cli/tests/fixtures/sentinel-01-bench.json";
const V0: &str = "examples/sentinel-01-minimal.json";

fn repo(path: &str) -> String {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    std::fs::read_to_string(format!("{root}/{path}")).unwrap()
}

fn json_of(path: &str) -> Value {
    serde_json::from_str(&repo(path)).unwrap()
}

fn solved(v: &Value) -> (Specification, SolvedTopology, PinTable) {
    let spec: Specification = serde_json::from_value(v.clone()).unwrap();
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

/// A netlist in KiCad 10.0.7's `kicadsexpr` shape, each symbol named as
/// KiCad writes it for the emitter's parts by their reference's letters.
fn netlist_text((values, nets): &Pads) -> String {
    let mut by_net: BTreeMap<&str, Vec<(&str, &str)>> = BTreeMap::new();
    for ((r, p), n) in nets {
        by_net.entry(n).or_default().push((r, p));
    }
    let mut s = String::from("(export (version \"E\") (components\n");
    for (r, v) in values {
        let stem: String = r.chars().take_while(char::is_ascii_alphabetic).collect();
        let lib = match stem.as_str() {
            "C" => " (libsource (lib \"Device\") (part \"C\"))",
            "R" => " (libsource (lib \"Device\") (part \"R\"))",
            "SW" => " (libsource (lib \"Switch\") (part \"SW_Push\"))",
            "TP" => " (libsource (lib \"Connector\") (part \"TestPoint\"))",
            _ => "",
        };
        s.push_str(&format!("(comp (ref \"{r}\") (value \"{v}\"){lib})\n"));
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
    let netlist = support::netlist(&strict::netlist(&netlist_text(pads)).unwrap()).unwrap();
    let tree = strict::board(board).unwrap();
    let (b, _) = route::read(&tree, &Stack::of(spec)).unwrap();
    support::judge(spec, &netlist, &b).unwrap()
}

fn kinds(r: &support::Record) -> Vec<&'static str> {
    r.findings.iter().map(|f| f.kind).collect()
}

fn messages(r: &support::Record) -> String {
    r.findings
        .iter()
        .map(|f| f.message.clone())
        .collect::<Vec<_>>()
        .join(" | ")
}

/// The bench board, emitted, and its pads; it passes.
fn bench() -> (Specification, Pads, String) {
    let (spec, solved, table) = solved(&json_of(BENCH));
    let board = schematic::emit(&spec, &solved, &table).unwrap().board;
    let pads = pads_of(&board);
    (spec, pads, board)
}

/// The resistor on BOOT0, by its pads.
fn pull_down(pads: &Pads) -> String {
    pads.1
        .iter()
        .find(|((r, _), n)| r.starts_with('R') && n.as_str() == "/BOOT0")
        .map(|((r, _), _)| r.clone())
        .unwrap()
}

#[test]
fn the_bench_board_passes_the_support_gate() {
    let (spec, pads, board) = bench();
    let r = judge(&spec, &pads, &board);
    assert_eq!(kinds(&r), Vec::<&str>::new(), "{}", messages(&r));
    let pd = pull_down(&pads);
    assert_eq!(pads.0[&pd], "10k");
    assert!(r.straps.iter().any(|s| s.contains(&format!(
        "BOOT0 (pin 138) on `/BOOT0`, pulled to `GND` by {pd} (10k)"
    ))));
    // The switch, from NRST to GND, and nothing else on BOOT0 but the MCU,
    // its pull-down, J2's pin and TP12.
    assert_eq!(pads.0["SW1"], "PTS810 SJM 250 SMTR LFS");
    let on = |n: &str| -> Vec<String> {
        pads.1
            .iter()
            .filter(|(_, net)| net.as_str() == n)
            .map(|((r, p), _)| format!("{r}.{p}"))
            .collect()
    };
    assert_eq!(
        on("/BOOT0"),
        ["J2.2", &format!("{pd}.1"), "TP12.1", "U1.138"]
    );
    assert!(on("/NRST").contains(&"SW1.1".to_owned()));
    assert!(on("GND").contains(&"SW1.2".to_owned()));
}

#[test]
fn boot0_wired_or_floating_or_pulled_the_wrong_way_fails() {
    let (spec, pads, board) = bench();
    let pd = pull_down(&pads);
    // Tied straight to GND, as today's strap: nothing drives it high.
    let mut p = pads.clone();
    p.1.insert(("U1".into(), "138".into()), "GND".into());
    let r = judge(&spec, &p, &board);
    // J2's BOOT0 pin then sits off the boot pin's net, so J2 is no longer
    // the programming path, as for any connector pin off its net.
    assert_eq!(
        kinds(&r),
        [
            "strap_wrong",
            "connector_pin_off_net",
            "no_programming_path",
            "test_point_off_net"
        ],
        "{}",
        messages(&r)
    );
    // Floating: the pull-down gone.
    let mut p = pads.clone();
    p.0.remove(&pd);
    p.1.retain(|(r, _), _| *r != pd);
    let r = judge(&spec, &p, &board);
    assert_eq!(kinds(&r), ["strap_wrong"], "{}", messages(&r));
    assert!(
        messages(&r).contains("0 resistors join"),
        "{}",
        messages(&r)
    );
    // Pulled up to the supply instead.
    let mut p = pads.clone();
    p.1.insert((pd.clone(), "2".into()), "VDD_3V3".into());
    let r = judge(&spec, &p, &board);
    assert_eq!(kinds(&r), ["strap_wrong"], "{}", messages(&r));
    // A value DS12117 refuses either way: too weak to hold VIL, too strong
    // for a driver.
    for (value, why) in [
        ("100M", "VIL"),
        ("470", "4000 uA"),
        ("0", "unbounded"),
        ("10 k", "not a resistance"),
    ] {
        let mut p = pads.clone();
        p.0.insert(pd.clone(), value.into());
        let r = judge(&spec, &p, &board);
        assert_eq!(kinds(&r), ["strap_wrong"], "{value}: {}", messages(&r));
        assert!(messages(&r).contains(why), "{value}: {}", messages(&r));
    }
}

#[test]
fn boot0_shared_with_another_pin_fails() {
    let (spec, pads, board) = bench();
    let mut p = pads.clone();
    // PDR_ON (143) joined to BOOT0's net.
    p.1.insert(("U1".into(), "143".into()), "/BOOT0".into());
    let r = judge(&spec, &p, &board);
    assert!(kinds(&r).contains(&"strap_wrong"), "{}", messages(&r));
    assert!(
        messages(&r).contains("shared with another"),
        "{}",
        messages(&r)
    );
}

#[test]
fn a_missing_extra_or_misplaced_reset_switch_fails() {
    let (spec, pads, board) = bench();
    // Not on the board.
    let mut p = pads.clone();
    p.0.remove("SW1");
    p.1.retain(|(r, _), _| r != "SW1");
    assert_eq!(kinds(&judge(&spec, &p, &board)), ["reset_switch_wrong"]);
    // Across NRST and the supply.
    let mut p = pads.clone();
    p.1.insert(("SW1".into(), "2".into()), "VDD_3V3".into());
    assert_eq!(kinds(&judge(&spec, &p, &board)), ["reset_switch_wrong"]);
    // Drawn where the specification declares none.
    let mut v = json_of(BENCH);
    v["mcu"]["reset_switch"] = json!(false);
    let (no_switch, ..) = solved(&v);
    assert_eq!(
        kinds(&judge(&no_switch, &pads, &board)),
        ["reset_switch_wrong"]
    );
}

#[test]
fn a_test_point_off_its_net_or_bridging_two_fails() {
    let (spec, pads, board) = bench();
    // TP1 (I2C_SDA) moved to I2C_SCL: two pads on SCL, none on SDA.
    let mut p = pads.clone();
    p.1.insert(("TP1".into(), "1".into()), "/I2C_SCL".into());
    let r = judge(&spec, &p, &board);
    assert_eq!(kinds(&r), ["test_point_off_net"], "{}", messages(&r));
    assert!(messages(&r).contains("TP1"), "{}", messages(&r));
    // TP9 (VDD_3V3) on GND: a bridge the gate refuses by name.
    let mut p = pads.clone();
    p.1.insert(("TP9".into(), "1".into()), "GND".into());
    assert_eq!(kinds(&judge(&spec, &p, &board)), ["test_point_off_net"]);
    // TP11 (NRST) off the MCU's NRST.
    let mut p = pads.clone();
    p.1.insert(("TP11".into(), "1".into()), "/I2C_SDA".into());
    assert_eq!(kinds(&judge(&spec, &p, &board)), ["test_point_off_net"]);
}

#[test]
fn a_connector_boot0_pin_off_the_boot_pin_fails() {
    let (spec, pads, board) = bench();
    let mut p = pads.clone();
    p.1.insert(("J2".into(), "2".into()), "/NRST".into());
    assert_eq!(
        kinds(&judge(&spec, &p, &board)),
        ["connector_pin_off_net", "no_programming_path"]
    );
}

#[test]
fn a_board_without_bench_access_is_judged_as_before() {
    let (spec, solved, table) = solved(&json_of(V0));
    let board = schematic::emit(&spec, &solved, &table).unwrap().board;
    let pads = pads_of(&board);
    let r = judge(&spec, &pads, &board);
    assert_eq!(kinds(&r), Vec::<&str>::new());
    assert!(r.straps.iter().any(|s| s == "BOOT0 (pin 138) on VSS's net"));
    assert!(
        !pads
            .0
            .keys()
            .any(|r| r.starts_with("TP") || r.starts_with("SW"))
    );
}

#[test]
fn test_points_are_out_of_the_bill_of_materials_and_the_switch_is_in() {
    let (spec, solved, table) = solved(&json_of(BENCH));
    let e = schematic::emit(&spec, &solved, &table).unwrap();
    assert!(!e.bom.contains("TestPoint"), "{}", e.bom);
    assert!(
        e.bom.contains(
            "SW1,PTS810 SJM 250 SMTR LFS,Button_Switch_SMD:SW_SPST_PTS810,Switch:SW_Push,"
        )
    );
    assert!(e.bom.contains(",10k,"));
    // Each test point's symbol is out of the BOM, as its footprint is, so
    // KiCad's parity check finds them alike (measured in tier 2).
    assert_eq!(e.schematic.matches("(in_bom no)").count(), 12);
}

#[test]
fn extract_counts_a_test_pad_as_copper_and_never_as_a_pin() {
    let (spec, solved, table) = solved(&json_of(BENCH));
    let board = schematic::emit(&spec, &solved, &table).unwrap().board;
    let read = extract::read(&board, &Stack::of(&spec)).unwrap();
    let lines = extract::lines(&spec, &solved, &table).unwrap();
    let bounds = extract::bound(&read, &lines).unwrap();
    assert_eq!(bounds.len(), 4);
    for b in &bounds {
        assert!(!b.pins.iter().any(|p| p.starts_with("TP")), "{:?}", b.pins);
        // The I2C lines: MCU, two devices and the pull-up, plus the test
        // pad; the SPI lines: MCU and flash, plus the test pad.
        let want = if b.spi_signal.is_some() { 3 } else { 5 };
        assert_eq!(b.pad_count, want, "{}", b.net);
    }
}

/// §5: a test point on a net spelled like one of the emitter's own nets
/// (a VCAP net, a device's strap net) is refused by `validate`, and by the
/// library path, which validates first (`AET-WO-1208` F3).
#[test]
fn a_test_point_on_an_emitter_net_is_refused_on_every_path() {
    for net in ["U1_VCAP1", "U2_SDO", "U4_WP", "unconnected-x"] {
        let mut v = json_of(BENCH);
        v["test_points"][0]["net"] = json!(net);
        let spec: Specification = serde_json::from_value(v).unwrap();
        let err = spec.validate().unwrap_err();
        assert_eq!(err.kind(), "TestPointNetUnknown", "{net}: {err}");
        let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
        let (_, solved, _) = solved(&json_of(BENCH));
        assert!(
            schematic::emit(&spec, &solved, &table).is_err(),
            "{net}: emit drew a test point validate refuses"
        );
    }
}
