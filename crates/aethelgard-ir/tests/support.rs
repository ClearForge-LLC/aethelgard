//! `AET-WO-4300g`: the two refusals at `validate` (`NoProgrammingPath`,
//! `SupplyHasNoSource`), the engine's debug nets, and the MCU row's support
//! network as the parts table parses it.

use aethelgard_ir::power_parts::{Kind, POWER_PARTS_JSON, SupportRole, parse_power_parts};
use aethelgard_ir::{IrValidationError, Specification};
use serde_json::{Value, json};

const REFERENCE: &str = include_str!("../../../examples/sentinel-01-minimal.json");
const POWER: &str = include_str!("fixtures/sentinel-02-power.json");

fn value(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

fn check(v: &Value) -> Result<(), IrValidationError> {
    let spec: Specification = serde_json::from_value(v.clone()).unwrap();
    spec.validate()
}

fn refused(v: &Value) -> &'static str {
    check(v).unwrap_err().kind()
}

#[test]
fn the_reference_declares_a_programming_path_and_a_source() {
    check(&value(REFERENCE)).unwrap();
}

/// §1.4: SWDIO, SWCLK, GND and the supply must reach one connector.
#[test]
fn a_connector_short_of_any_debug_line_is_no_programming_path() {
    for pin in 0..6 {
        let mut v = value(REFERENCE);
        let net = v["connectors"][0]["pins"][pin]["net"].clone();
        if net.is_null() {
            continue;
        }
        v["connectors"][0]["pins"][pin]["net"] = Value::Null;
        assert_eq!(
            refused(&v),
            "NoProgrammingPath",
            "J1 pin {} ({net})",
            pin + 1
        );
    }
    // Spread over two connectors, the four reach no one connector.
    let mut v = value(REFERENCE);
    v["connectors"][0]["pins"][3]["net"] = Value::Null;
    v["connectors"][1]["pins"][1]["net"] = json!("SWDIO");
    assert_eq!(refused(&v), "NoProgrammingPath");
    // No connector at all: the source's connector goes too, and the path is
    // judged first.
    let mut v = value(REFERENCE);
    v["connectors"] = json!([]);
    assert_eq!(refused(&v), "NoProgrammingPath");
}

/// NRST may reach the connector; it is not required.
#[test]
fn nrst_on_the_connector_is_admitted() {
    let mut v = value(REFERENCE);
    v["connectors"][0]["pins"][1]["net"] = json!("NRST");
    check(&v).unwrap();
}

/// The debug nets are the engine's: no declared net or domain takes one, in
/// any case.
#[test]
fn a_declared_net_or_domain_spelling_a_debug_net_is_refused() {
    let mut v = value(REFERENCE);
    v["i2c_buses"][0]["sda"]["net"] = json!("swdio");
    v["net_constraints"][0]["net_name"] = json!("swdio");
    assert_eq!(refused(&v), "DuplicateNetName");
    let mut v = value(REFERENCE);
    v["spi_devices"][0]["cs"]["net"] = json!("NRST");
    assert_eq!(refused(&v), "DuplicateNetName");
    let mut v = value(REFERENCE);
    v["voltage_domains"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": "SwClk", "nominal_millivolts": 1800, "tolerance_permille": 50}));
    assert_eq!(refused(&v), "NetShadowsDomain");
}

/// §1.4: the supply's one declared source.
#[test]
fn a_supply_without_one_declared_source_is_refused() {
    let cases: [(&str, Value); 4] = [
        ("null, and no power tree", Value::Null),
        (
            "an undeclared connector",
            json!({"connector": "J9", "pin": "1"}),
        ),
        (
            "a pin the connector does not list",
            json!({"connector": "J2", "pin": "7"}),
        ),
        ("a pin on GND", json!({"connector": "J2", "pin": "4"})),
    ];
    for (why, source) in cases {
        let mut v = value(REFERENCE);
        v["mcu"]["supply_source"] = source;
        let e = check(&v).unwrap_err();
        assert_eq!(e.kind(), "SupplyHasNoSource", "{why}");
        assert_eq!(e.id(), Some("VDD_3V3"), "{why}");
    }
    // The key is required.
    let mut v = value(REFERENCE);
    v["mcu"].as_object_mut().unwrap().remove("supply_source");
    assert!(serde_json::from_value::<Specification>(v).is_err());
}

/// A power tree's regulator is the source; a connector pin named beside it
/// is a second source, refused.
#[test]
fn a_regulated_supply_takes_no_connector_source() {
    let v = value(POWER);
    assert!(v["mcu"]["supply_source"].is_null());
    check(&v).unwrap();
    let mut two = v.clone();
    let debug = two["connectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| {
            c["pins"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["net"] == "SWDIO")
        })
        .unwrap()["reference"]
        .clone();
    two["mcu"]["supply_source"] = json!({"connector": debug, "pin": "1"});
    let e = check(&two).unwrap_err();
    assert_eq!(e.kind(), "SupplyHasNoSource");
    assert!(e.to_string().contains("outputs it too"), "{e}");
}

/// The MCU row's support network as cited: 11 VDD pins (N), each role once,
/// the four straps.
#[test]
fn the_mcu_row_holds_figure_14s_network() {
    let t = parse_power_parts(POWER_PARTS_JSON).unwrap();
    let Kind::Mcu(m) = &t.get("STM32H753ZITx").unwrap().kind else {
        panic!("the MCU row is not an MCU")
    };
    let s = &m.support;
    assert_eq!(s.group("VDD").len(), 11);
    assert_eq!(s.group("VCAP"), ["71", "106"]);
    let roles: Vec<SupportRole> = s.capacitors.iter().map(|c| c.role).collect();
    assert_eq!(roles.len(), 8);
    let vdd = s
        .capacitors
        .iter()
        .find(|c| c.role == SupportRole::VddBypass)
        .unwrap();
    assert!(vdd.per_pin);
    assert_eq!(vdd.class(), (80_000, 120_000));
    let straps: Vec<(&str, &str)> = s
        .straps
        .iter()
        .map(|x| (x.pin.as_str(), x.to.as_str()))
        .collect();
    assert_eq!(
        straps,
        [
            ("BOOT0", "VSS"),
            ("PDR_ON", "VDD"),
            ("VBAT", "VDD"),
            ("VREF+", "VDDA")
        ]
    );
}

/// A malformed support network is refused by name, never read in part.
#[test]
fn a_malformed_support_network_is_refused() {
    type Edit = (&'static str, fn(&mut Value));
    let edits: [Edit; 7] = [
        ("a pin in two groups", |s| s["pins"]["VBAT"] = json!(["17"])),
        ("a group out of order", |s| {
            s["pins"]["VCAP"] = json!(["106", "71"])
        }),
        ("not a package position", |s| {
            s["pins"]["NRST"] = json!(["025"])
        }),
        ("an unknown owner", |s| {
            s["capacitors"][0]["owner"] = json!("VDDX")
        }),
        ("a strap of two pins", |s| {
            s["straps"][0]["pin"] = json!("VCAP")
        }),
        ("a tolerance above 500", |s| {
            s["capacitors"][0]["tolerance_permille"] = json!(501)
        }),
        ("a role twice", |s| {
            let first = s["capacitors"][0].clone();
            s["capacitors"].as_array_mut().unwrap().push(first);
        }),
    ];
    for (why, edit) in edits {
        let mut t = value(POWER_PARTS_JSON);
        edit(support_of(&mut t));
        let e = parse_power_parts(&t.to_string()).unwrap_err();
        assert_eq!(e.kind(), "PowerPartsTableInvalid", "{why}");
    }
    let mut t = value(POWER_PARTS_JSON);
    t["schema_version"] = json!(1);
    assert!(parse_power_parts(&t.to_string()).is_err());
}

/// The MCU row's support network inside the table `t`, mutably.
fn support_of(t: &mut Value) -> &mut Value {
    let rows = t["parts"].as_array_mut().unwrap();
    let row = rows
        .iter_mut()
        .find(|r| r["part"] == "STM32H753ZITx")
        .unwrap();
    &mut row["kind"]["mcu"]["support"]
}

/// Adversarial A3: the programming connector's supply pin is the debugger's
/// sense line, not a source.
#[test]
fn the_debug_ports_supply_pin_is_not_a_source() {
    let mut v = value(REFERENCE);
    v["connectors"].as_array_mut().unwrap().truncate(1);
    v["mcu"]["supply_source"] = json!({"connector": "J1", "pin": "1"});
    let e = check(&v).unwrap_err();
    assert_eq!(e.kind(), "SupplyHasNoSource");
    assert!(e.to_string().contains("programs the part"), "{e}");
}

/// §5.3: a regulator's output renamed leaves the supply sourceless, which the
/// power tree's own check refuses first (step 16, before step 18).
#[test]
fn a_supply_whose_regulator_output_is_renamed_is_refused() {
    let v = value(POWER);
    let supply = v["mcu"]["supply"].as_str().unwrap().to_owned();
    let regs = v["power"]["regulators"].as_array().unwrap();
    let i = regs
        .iter()
        .position(|r| r["output"] == supply.as_str())
        .unwrap();
    for (to, kind) in [
        ("VDD_3V3X", "UnknownDomain"),
        ("VDD_5V", "DomainWithoutSource"),
    ] {
        let mut w = v.clone();
        w["power"]["regulators"][i]["output"] = json!(to);
        assert_eq!(refused(&w), kind, "{to}");
    }
}
