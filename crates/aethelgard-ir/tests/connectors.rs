//! Off-board connectors (`AET-WO-4300f`): the tables, every refusal of the
//! connectors' step of `validate`, their identifiers, the strict reader, and
//! the twin's blindness to them (`RUL-16`).

use std::collections::BTreeMap;

use aethelgard_ir::connectors::{PARTS, Series, part};
use aethelgard_ir::ds009::{PORTS, Role};
use aethelgard_ir::{
    IrValidationError as E, Pin, PinSource, PinTable, SolvedTopology, Specification, solve,
};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("../../aethelgard-cli/tests/fixtures/connectors.json");
const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const SOURCE: &str = "condition: a JST GH cable of at most 300 mm at 50 pF/m (15 pF), the mated contact (5 pF) and one remote pin of at most 10 pF (UM10204 Table 10 Ci)";

fn fixture() -> Value {
    serde_json::from_str(FIXTURE).expect("the fixture is JSON")
}

fn read(v: &Value) -> Result<Specification, serde_json::Error> {
    serde_json::from_value(v.clone())
}

fn spec_of(v: &Value) -> Specification {
    read(v).expect("the mutated specification still reads")
}

fn refusal(v: &Value) -> E {
    spec_of(v)
        .validate()
        .expect_err("the mutated specification is refused")
}

fn assert_refusal(err: &E, kind: &str, id: &str) {
    assert_eq!(err.kind(), kind, "{err}");
    assert_eq!(err.id(), Some(id), "{err}");
}

fn assert_path(err: &E, kind: &str, path: &str) {
    assert_eq!(err.kind(), kind, "{err}");
    assert_eq!(err.path(), Some(path), "{err}");
}

fn offboard() -> Value {
    json!({"capacitance_femtofarads": 30000, "source": SOURCE})
}

fn pin(n: &str, net: Option<&str>, budget: bool) -> Value {
    json!({"pin": n, "net": net, "offboard": if budget { offboard() } else { Value::Null }})
}

fn set_connector(v: &mut Value, i: usize, part: &str, standard: Option<&str>, pins: Vec<Value>) {
    v["connectors"][i]["part"] = json!(part);
    v["connectors"][i]["standard"] = json!(standard);
    v["connectors"][i]["pins"] = Value::Array(pins);
}

/// Pin number `n` (1-based) of connector `c`, to a net and a budget flag.
fn set_pin(v: &mut Value, c: usize, n: usize, net: Option<&str>, budget: bool) {
    v["connectors"][c]["pins"][n - 1] = pin(&n.to_string(), net, budget);
}

/// J1 as a plain explicit map, so the standard does not speak first.
fn explicit(v: &mut Value) {
    v["connectors"][0]["standard"] = Value::Null;
}

// 1. The fixture validates.

#[test]
fn fixture_validates() {
    assert_eq!(
        spec_of(&fixture()).validate(),
        Ok(()),
        "the fixture is valid"
    );
}

#[test]
fn empty_connector_list_validates() {
    let mut v = fixture();
    v["connectors"] = json!([]);
    let spec = spec_of(&v);
    assert_eq!(spec.validate(), Ok(()), "no connectors is valid");
    assert!(spec.declared_connectors().is_empty(), "none are declared");
}

#[test]
fn declared_connectors_are_in_document_order() {
    let spec = spec_of(&fixture());
    let refs: Vec<_> = spec
        .declared_connectors()
        .iter()
        .map(|c| c.reference.as_str())
        .collect();
    assert_eq!(refs, ["J1", "J2"], "document order is kept");
}

// 2. Refusals of the connectors' step.

#[test]
fn unknown_part_is_refused() {
    let mut v = fixture();
    v["connectors"][0]["part"] = json!("SM05B-GHS-TB");
    let err = refusal(&v);
    assert_refusal(&err, "UnknownConnectorPart", "J1");
    assert!(
        matches!(&err, E::UnknownConnectorPart { part, .. } if part == "SM05B-GHS-TB"),
        "the error names the part: {err}"
    );
}

#[test]
fn pin_zero_is_unknown() {
    let mut v = fixture();
    v["connectors"][0]["pins"][3]["pin"] = json!("0");
    let err = refusal(&v);
    assert_refusal(&err, "ConnectorPinUnknown", "J1");
    assert!(
        matches!(&err, E::ConnectorPinUnknown { pin, pins: 4, .. } if pin == "0"),
        "pin 0 is outside 1..=4: {err}"
    );
}

#[test]
fn pin_with_leading_zero_is_unknown() {
    let mut v = fixture();
    v["connectors"][0]["pins"][3]["pin"] = json!("07");
    let err = refusal(&v);
    assert_refusal(&err, "ConnectorPinUnknown", "J1");
    let mut v = fixture();
    v["connectors"][0]["pins"][3]["pin"] = json!("04");
    assert_refusal(&refusal(&v), "ConnectorPinUnknown", "J1");
}

#[test]
fn pin_past_the_part_is_unknown() {
    let mut v = fixture();
    v["connectors"][0]["pins"][3]["pin"] = json!("5");
    let err = refusal(&v);
    assert_refusal(&err, "ConnectorPinUnknown", "J1");
    assert!(
        matches!(&err, E::ConnectorPinUnknown { pin, pins: 4, .. } if pin == "5"),
        "pin 5 is past a 4-pin part: {err}"
    );
}

#[test]
fn non_numeric_pin_is_unknown() {
    let mut v = fixture();
    v["connectors"][0]["pins"][3]["pin"] = json!("A4");
    assert_refusal(&refusal(&v), "ConnectorPinUnknown", "J1");
}

#[test]
fn repeated_pin_is_refused() {
    let mut v = fixture();
    v["connectors"][0]["pins"][3]["pin"] = json!("1");
    let err = refusal(&v);
    assert_refusal(&err, "ConnectorPinRepeated", "J1");
    assert!(
        matches!(&err, E::ConnectorPinRepeated { pin, .. } if pin == "1"),
        "the repeated pin is named: {err}"
    );
}

#[test]
fn missing_pin_is_refused() {
    let mut v = fixture();
    v["connectors"][0]["pins"].as_array_mut().unwrap().pop();
    let err = refusal(&v);
    assert_refusal(&err, "ConnectorPinMissing", "J1");
    assert!(
        matches!(&err, E::ConnectorPinMissing { pin, .. } if pin == "4"),
        "the first missing pin is named: {err}"
    );
}

#[test]
fn pin_order_is_not_significant() {
    let mut v = fixture();
    v["connectors"][0]["pins"].as_array_mut().unwrap().reverse();
    assert_eq!(spec_of(&v).validate(), Ok(()), "pins are keyed by number");
}

#[test]
fn undeclared_net_is_refused() {
    let mut v = fixture();
    explicit(&mut v);
    set_pin(&mut v, 0, 1, Some("VDD_12V"), false);
    let err = refusal(&v);
    assert_refusal(&err, "ConnectorNetUnknown", "J1");
    assert!(
        matches!(&err, E::ConnectorNetUnknown { pin, net, .. } if pin == "1" && net == "VDD_12V"),
        "a connector cannot invent a net: {err}"
    );
}

#[test]
fn budget_null_on_i2c_line_is_missing() {
    let mut v = fixture();
    set_pin(&mut v, 0, 2, Some("I2C_SCL"), false);
    let err = refusal(&v);
    assert_refusal(&err, "ConnectorBudgetMissing", "J1");
    assert!(
        matches!(&err, E::ConnectorBudgetMissing { pin, .. } if pin == "2"),
        "{err}"
    );
}

#[test]
fn budget_null_on_spi_line_is_missing() {
    let mut v = fixture();
    set_pin(&mut v, 1, 2, Some("EXT_SCK"), false);
    let err = refusal(&v);
    assert_refusal(&err, "ConnectorBudgetMissing", "J2");
    assert!(
        matches!(&err, E::ConnectorBudgetMissing { pin, .. } if pin == "2"),
        "{err}"
    );
}

#[test]
fn budget_null_on_spi_chip_select_is_missing() {
    let mut v = fixture();
    set_pin(&mut v, 1, 5, Some("FLASH_CS"), false);
    let err = refusal(&v);
    assert_refusal(&err, "ConnectorBudgetMissing", "J2");
    assert!(
        matches!(&err, E::ConnectorBudgetMissing { pin, .. } if pin == "5"),
        "a chip select is a line: {err}"
    );
}

#[test]
fn budget_with_chip_select_is_accepted() {
    let mut v = fixture();
    set_pin(&mut v, 1, 5, Some("FLASH_CS"), true);
    assert_eq!(
        spec_of(&v).validate(),
        Ok(()),
        "a chip select takes a budget"
    );
}

#[test]
fn zero_budget_is_missing() {
    let mut v = fixture();
    v["connectors"][0]["pins"][1]["offboard"]["capacitance_femtofarads"] = json!(0);
    let err = refusal(&v);
    assert_refusal(&err, "ConnectorBudgetMissing", "J1");
}

#[test]
fn budget_on_ground_is_not_applicable() {
    let mut v = fixture();
    set_pin(&mut v, 0, 4, Some("GND"), true);
    let err = refusal(&v);
    assert_refusal(&err, "ConnectorBudgetNotApplicable", "J1");
    assert!(
        matches!(&err, E::ConnectorBudgetNotApplicable { pin, .. } if pin == "4"),
        "{err}"
    );
}

#[test]
fn budget_on_domain_is_not_applicable() {
    let mut v = fixture();
    set_pin(&mut v, 0, 1, Some("VDD_5V"), true);
    assert_refusal(&refusal(&v), "ConnectorBudgetNotApplicable", "J1");
}

#[test]
fn budget_on_null_net_is_not_applicable() {
    let mut v = fixture();
    set_pin(&mut v, 1, 5, None, true);
    let err = refusal(&v);
    assert_refusal(&err, "ConnectorBudgetNotApplicable", "J2");
    assert!(
        matches!(&err, E::ConnectorBudgetNotApplicable { pin, .. } if pin == "5"),
        "{err}"
    );
}

#[test]
fn zero_budget_on_ground_is_not_applicable_not_missing() {
    let mut v = fixture();
    set_pin(&mut v, 0, 4, Some("GND"), true);
    v["connectors"][0]["pins"][3]["offboard"]["capacitance_femtofarads"] = json!(0);
    assert_refusal(&refusal(&v), "ConnectorBudgetNotApplicable", "J1");
}

#[test]
fn unknown_standard_is_refused() {
    let mut v = fixture();
    v["connectors"][0]["standard"] = json!("DS-009 NOPE");
    let err = refusal(&v);
    assert_refusal(&err, "UnknownConnectorStandard", "J1");
    assert!(
        matches!(&err, E::UnknownConnectorStandard { standard, .. } if standard == "DS-009 NOPE"),
        "{err}"
    );
}

#[test]
fn standard_is_matched_exactly() {
    let mut v = fixture();
    v["connectors"][0]["standard"] = json!("ds-009 i2c");
    assert_refusal(&refusal(&v), "UnknownConnectorStandard", "J1");
}

#[test]
fn gh_6_pin_part_named_i2c_contradicts_the_standard() {
    let mut v = fixture();
    v["connectors"][1]["standard"] = json!("DS-009 I2C");
    let err = refusal(&v);
    assert_refusal(&err, "ConnectorPartContradictsStandard", "J2");
    assert!(
        matches!(&err, E::ConnectorPartContradictsStandard { part, standard, .. }
            if part == "SM06B-GHS-TB" && standard == "DS-009 I2C"),
        "{err}"
    );
}

#[test]
fn sh_part_named_telem_contradicts_the_standard() {
    let mut v = fixture();
    set_connector(
        &mut v,
        1,
        "SM06B-SRSS-TB",
        Some("DS-009 TELEM"),
        (1..=6).map(|n| pin(&n.to_string(), None, false)).collect(),
    );
    assert_refusal(&refusal(&v), "ConnectorPartContradictsStandard", "J2");
}

#[test]
fn gh_part_named_debug_mini_contradicts_the_standard() {
    let mut v = fixture();
    v["connectors"][1]["standard"] = json!("DS-009 DEBUG MINI");
    assert_refusal(&refusal(&v), "ConnectorPartContradictsStandard", "J2");
}

#[test]
fn i2c_supply_on_3v3_domain_is_refused() {
    let mut v = fixture();
    set_pin(&mut v, 0, 1, Some("VDD_3V3"), false);
    let err = refusal(&v);
    assert_refusal(&err, "ConnectorSupplyContradictsStandard", "J1");
    assert!(
        matches!(&err, E::ConnectorSupplyContradictsStandard {
            pin, role: "VCC +5V", net: Some(n), required_millivolts: Some(5000), ..
        } if pin == "1" && n == "VDD_3V3"),
        "{err}"
    );
}

#[test]
fn i2c_supply_null_is_refused() {
    let mut v = fixture();
    set_pin(&mut v, 0, 1, None, false);
    let err = refusal(&v);
    assert_refusal(&err, "ConnectorSupplyContradictsStandard", "J1");
    assert!(
        matches!(
            &err,
            E::ConnectorSupplyContradictsStandard { net: None, .. }
        ),
        "{err}"
    );
}

#[test]
fn i2c_supply_on_ground_is_refused() {
    let mut v = fixture();
    set_pin(&mut v, 0, 1, Some("GND"), false);
    assert_refusal(&refusal(&v), "ConnectorSupplyContradictsStandard", "J1");
}

#[test]
fn i2c_supply_on_a_5v_domain_renamed_but_5100_mv_is_refused() {
    let mut v = fixture();
    v["voltage_domains"][1]["id"] = json!("VDD_5V1");
    v["voltage_domains"][1]["nominal_millivolts"] = json!(5100);
    set_pin(&mut v, 0, 1, Some("VDD_5V1"), false);
    // J2 pin 1 still names VDD_5V; J2 is an explicit map, so repoint it too.
    set_pin(&mut v, 1, 1, Some("VDD_5V1"), false);
    let err = refusal(&v);
    assert_refusal(&err, "ConnectorSupplyContradictsStandard", "J1");
    assert!(
        matches!(
            &err,
            E::ConnectorSupplyContradictsStandard {
                required_millivolts: Some(5000),
                ..
            }
        ),
        "D5: the nominal decides, not the name: {err}"
    );
}

#[test]
fn i2c_supply_on_a_5v_domain_by_another_name_is_accepted() {
    let mut v = fixture();
    v["voltage_domains"][1]["id"] = json!("RAIL_A");
    set_pin(&mut v, 0, 1, Some("RAIL_A"), false);
    set_pin(&mut v, 1, 1, Some("RAIL_A"), false);
    assert_eq!(spec_of(&v).validate(), Ok(()), "D5: the nominal decides");
}

#[test]
fn i2c_with_scl_and_sda_swapped_is_refused_at_pin_2() {
    let mut v = fixture();
    set_pin(&mut v, 0, 2, Some("I2C_SDA"), true);
    set_pin(&mut v, 0, 3, Some("I2C_SCL"), true);
    let err = refusal(&v);
    assert_refusal(&err, "ConnectorPinContradictsStandard", "J1");
    assert!(
        matches!(&err, E::ConnectorPinContradictsStandard { pin, role: "I2C SCL", .. } if pin == "2"),
        "{err}"
    );
}

#[test]
fn i2c_with_pin_4_not_ground_is_refused() {
    let mut v = fixture();
    set_pin(&mut v, 0, 4, None, false);
    let err = refusal(&v);
    assert_refusal(&err, "ConnectorPinContradictsStandard", "J1");
    assert!(
        matches!(&err, E::ConnectorPinContradictsStandard { pin, role: "GND", .. } if pin == "4"),
        "{err}"
    );
}

#[test]
fn i2c_with_pin_4_on_a_domain_is_refused() {
    let mut v = fixture();
    set_pin(&mut v, 0, 4, Some("VDD_3V3"), false);
    let err = refusal(&v);
    assert_refusal(&err, "ConnectorPinContradictsStandard", "J1");
}

#[test]
fn telem_with_floating_flow_control_is_refused_at_tx() {
    let mut v = fixture();
    set_connector(
        &mut v,
        1,
        "SM06B-GHS-TB",
        Some("DS-009 TELEM"),
        vec![
            pin("1", Some("VDD_5V"), false),
            pin("2", None, false),
            pin("3", None, false),
            pin("4", None, false),
            pin("5", None, false),
            pin("6", Some("GND"), false),
        ],
    );
    let err = refusal(&v);
    assert_refusal(&err, "ConnectorPinContradictsStandard", "J2");
    assert!(
        matches!(&err, E::ConnectorPinContradictsStandard { pin, role: "TX (OUT)", net: None, .. }
            if pin == "2"),
        "no serial net exists in this schema, so TX refuses null: {err}"
    );
}

#[test]
fn first_refusal_is_the_first_pin_in_pin_order() {
    // Pin 4 (CTS) is given a net, which a floating role refuses; pin 2 (TX)
    // refuses first, since pins are judged 1 to N.
    let mut v = fixture();
    set_connector(
        &mut v,
        1,
        "SM06B-GHS-TB",
        Some("DS-009 TELEM"),
        vec![
            pin("1", Some("VDD_5V"), false),
            pin("2", None, false),
            pin("3", None, false),
            pin("4", Some("VDD_3V3"), false),
            pin("5", None, false),
            pin("6", Some("GND"), false),
        ],
    );
    let err = refusal(&v);
    assert!(
        matches!(&err, E::ConnectorPinContradictsStandard { pin, role: "TX (OUT)", .. } if pin == "2"),
        "document order: {err}"
    );
}

#[test]
fn first_refusal_is_by_pin_order_not_listing_order() {
    // Pins listed in reverse: pin 1 (supply) is still judged before pin 2.
    let mut v = fixture();
    set_connector(
        &mut v,
        1,
        "SM06B-GHS-TB",
        Some("DS-009 TELEM"),
        vec![
            pin("6", Some("GND"), false),
            pin("5", None, false),
            pin("4", None, false),
            pin("3", None, false),
            pin("2", None, false),
            pin("1", Some("VDD_3V3"), false),
        ],
    );
    let err = refusal(&v);
    assert!(
        matches!(&err, E::ConnectorSupplyContradictsStandard { pin, .. } if pin == "1"),
        "role order, not listing order: {err}"
    );
}

fn debug_mini(v: &mut Value, supply: Option<&str>) {
    set_connector(
        v,
        1,
        "SM06B-SRSS-TB",
        Some("DS-009 DEBUG MINI"),
        vec![
            pin("1", supply, false),
            pin("2", None, false),
            pin("3", None, false),
            pin("4", None, false),
            pin("5", None, false),
            pin("6", Some("GND"), false),
        ],
    );
}

#[test]
fn debug_mini_accepts_the_mcu_supply_at_pin_1_and_refuses_pin_2() {
    let mut v = fixture();
    debug_mini(&mut v, Some("VDD_3V3"));
    let err = refusal(&v);
    assert_refusal(&err, "ConnectorPinContradictsStandard", "J2");
    assert!(
        matches!(&err, E::ConnectorPinContradictsStandard { pin, role: "CONSOLE TX (OUT)", .. }
            if pin == "2"),
        "pin 1 holds, so the refusal is at pin 2: {err}"
    );
}

#[test]
fn debug_mini_refuses_a_5v_supply_at_pin_1() {
    let mut v = fixture();
    debug_mini(&mut v, Some("VDD_5V"));
    let err = refusal(&v);
    assert_refusal(&err, "ConnectorSupplyContradictsStandard", "J2");
    assert!(
        matches!(&err, E::ConnectorSupplyContradictsStandard {
            pin, role: "TARGET PROCESSOR VOLTAGE", ..
        } if pin == "1"),
        "the supply is the MCU's, not the 5 V rail: {err}"
    );
    assert!(
        matches!(
            &err,
            E::ConnectorSupplyContradictsStandard {
                required_millivolts: Some(3300),
                ..
            }
        ),
        "the required supply is the MCU's nominal, 3300 mV: {err}"
    );
}

#[test]
fn debug_mini_follows_the_mcu_supply_by_name() {
    // A second 3.3 V domain is not the MCU's supply, though its nominal matches.
    let mut v = fixture();
    v["voltage_domains"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": "VDD_3V3B", "nominal_millivolts": 3300, "tolerance_permille": 50}));
    debug_mini(&mut v, Some("VDD_3V3B"));
    let err = refusal(&v);
    assert!(
        matches!(&err, E::ConnectorSupplyContradictsStandard { pin, .. } if pin == "1"),
        "the MCU's own supply by name: {err}"
    );
}

#[test]
fn i2c_pins_on_two_different_buses_are_refused_at_pin_3() {
    let mut v = fixture();
    v["voltage_domains"][0]["id"] = json!("VDD_3V3");
    v["i2c_buses"].as_array_mut().unwrap().push(json!({
        "id": "OTHER",
        "peripheral": null,
        "speed_hz": 400000,
        "pull_up_ohms": 2200,
        "domain": "VDD_3V3",
        "sda": {"net": "I2C2_SDA", "pin": null},
        "scl": {"net": "I2C2_SCL", "pin": null}
    }));
    for net in ["I2C2_SDA", "I2C2_SCL"] {
        v["net_constraints"].as_array_mut().unwrap().push(json!({
            "net_name": net,
            "max_capacitance_femtofarads": 100000,
            "max_rise_time_nanoseconds": 300,
            "impedance_ohms": null
        }));
    }
    assert_eq!(
        spec_of(&v).validate(),
        Ok(()),
        "two buses and J1 on SENSORS"
    );
    set_pin(&mut v, 0, 3, Some("I2C2_SDA"), true);
    let err = refusal(&v);
    assert_refusal(&err, "ConnectorPinContradictsStandard", "J1");
    assert!(
        matches!(&err, E::ConnectorPinContradictsStandard { pin, role: "I2C SDA", .. } if pin == "3"),
        "SCL is SENSORS's, SDA is OTHER's: {err}"
    );
    // The other bus whole is fine.
    set_pin(&mut v, 0, 2, Some("I2C2_SCL"), true);
    assert_eq!(spec_of(&v).validate(), Ok(()), "both pins on one bus");
}

#[test]
fn explicit_map_is_judged_by_no_standard() {
    let mut v = fixture();
    explicit(&mut v);
    set_pin(&mut v, 0, 1, None, false);
    set_pin(&mut v, 0, 4, Some("VDD_3V3"), false);
    assert_eq!(
        spec_of(&v).validate(),
        Ok(()),
        "a null standard judges nothing"
    );
}

#[test]
fn part_errors_come_before_pin_errors() {
    let mut v = fixture();
    v["connectors"][0]["part"] = json!("NOPE");
    v["connectors"][0]["pins"][0]["pin"] = json!("0");
    assert_refusal(&refusal(&v), "UnknownConnectorPart", "J1");
}

#[test]
fn second_connector_is_checked_after_the_first() {
    let mut v = fixture();
    v["connectors"][1]["part"] = json!("NOPE");
    assert_refusal(&refusal(&v), "UnknownConnectorPart", "J2");
    set_pin(&mut v, 0, 1, Some("VDD_3V3"), false);
    assert_refusal(&refusal(&v), "ConnectorSupplyContradictsStandard", "J1");
}

// 3. Identifiers.

#[test]
fn reference_with_a_hyphen_is_invalid() {
    let mut v = fixture();
    v["connectors"][0]["reference"] = json!("J-1");
    assert_path(&refusal(&v), "InvalidIdentifier", "connectors[0].reference");
}

#[test]
fn reference_reserved_name_is_invalid() {
    let mut v = fixture();
    v["connectors"][0]["reference"] = json!("sysbus");
    let err = refusal(&v);
    assert_eq!(err.kind(), "InvalidIdentifier", "{err}");
}

#[test]
fn part_with_a_space_is_invalid() {
    let mut v = fixture();
    v["connectors"][0]["part"] = json!("SM04B GHS-TB");
    assert_path(&refusal(&v), "InvalidIdentifier", "connectors[0].part");
}

#[test]
fn pin_with_a_space_is_invalid() {
    let mut v = fixture();
    v["connectors"][1]["pins"][4]["pin"] = json!("5 ");
    assert_path(
        &refusal(&v),
        "InvalidIdentifier",
        "connectors[1].pins[4].pin",
    );
}

#[test]
fn net_with_a_space_is_invalid() {
    let mut v = fixture();
    v["connectors"][0]["pins"][1]["net"] = json!("I2C SCL");
    assert_path(
        &refusal(&v),
        "InvalidIdentifier",
        "connectors[0].pins[1].net",
    );
}

#[test]
fn offboard_source_with_a_leading_space_is_invalid() {
    let mut v = fixture();
    v["connectors"][0]["pins"][1]["offboard"]["source"] = json!(format!(" {SOURCE}"));
    assert_path(
        &refusal(&v),
        "InvalidSource",
        "connectors[0].pins[1].offboard.source",
    );
}

#[test]
fn offboard_source_empty_is_invalid() {
    let mut v = fixture();
    v["connectors"][0]["pins"][1]["offboard"]["source"] = json!("");
    assert_path(
        &refusal(&v),
        "InvalidSource",
        "connectors[0].pins[1].offboard.source",
    );
}

#[test]
fn standard_with_a_space_is_not_an_identifier_error() {
    let spec = spec_of(&fixture());
    assert_eq!(
        spec.declared_connectors()[0].standard.as_deref(),
        Some("DS-009 I2C"),
        "the fixture's standard holds a space"
    );
    assert_eq!(spec.validate(), Ok(()), "a space in a standard is fine");
}

#[test]
fn identifiers_come_before_the_connector_step() {
    // An invalid identifier on J1 and an unknown part on J1: step 2 first.
    let mut v = fixture();
    v["connectors"][0]["part"] = json!("NOPE X");
    assert_path(&refusal(&v), "InvalidIdentifier", "connectors[0].part");
}

// 4. References.

#[test]
fn connector_reference_equal_to_a_device_is_a_duplicate() {
    let mut v = fixture();
    v["connectors"][0]["reference"] = json!("U2");
    let err = refusal(&v);
    assert_refusal(&err, "DuplicateReference", "U2");
}

#[test]
fn connector_reference_equal_to_a_device_in_another_case_is_a_duplicate() {
    let mut v = fixture();
    v["connectors"][0]["reference"] = json!("u2");
    let err = refusal(&v);
    assert_refusal(&err, "DuplicateReference", "u2");
    assert!(
        matches!(&err, E::DuplicateReference { first, .. } if first == "U2"),
        "the device came first: {err}"
    );
}

#[test]
fn two_connectors_with_one_reference_are_a_duplicate() {
    let mut v = fixture();
    v["connectors"][1]["reference"] = json!("j1");
    assert_refusal(&refusal(&v), "DuplicateReference", "j1");
}

// 5. The strict reader.

fn read_err(v: &Value) -> String {
    read(v).expect_err("the reader refuses").to_string()
}

#[test]
fn connectors_key_is_required() {
    let mut v = fixture();
    v.as_object_mut().unwrap().remove("connectors");
    assert!(
        read_err(&v).contains("connectors"),
        "a missing key is no default"
    );
}

#[test]
fn connector_without_standard_is_refused() {
    let mut v = fixture();
    v["connectors"][0]
        .as_object_mut()
        .unwrap()
        .remove("standard");
    assert!(
        read_err(&v).contains("standard"),
        "standard is a required key"
    );
}

#[test]
fn pin_without_offboard_is_refused() {
    let mut v = fixture();
    v["connectors"][0]["pins"][0]
        .as_object_mut()
        .unwrap()
        .remove("offboard");
    assert!(
        read_err(&v).contains("offboard"),
        "offboard is a required key"
    );
}

#[test]
fn pin_without_net_is_refused() {
    let mut v = fixture();
    v["connectors"][0]["pins"][0]
        .as_object_mut()
        .unwrap()
        .remove("net");
    assert!(read_err(&v).contains("net"), "net is a required key");
}

#[test]
fn pin_without_pin_number_is_refused() {
    let mut v = fixture();
    v["connectors"][0]["pins"][0]
        .as_object_mut()
        .unwrap()
        .remove("pin");
    assert!(read_err(&v).contains("pin"), "pin is a required key");
}

#[test]
fn offboard_without_source_is_refused() {
    let mut v = fixture();
    v["connectors"][0]["pins"][1]["offboard"]
        .as_object_mut()
        .unwrap()
        .remove("source");
    assert!(read_err(&v).contains("source"), "source is a required key");
}

#[test]
fn unknown_key_in_a_pin_is_refused() {
    let mut v = fixture();
    v["connectors"][0]["pins"][0]["role"] = json!("VCC");
    assert!(
        read_err(&v).contains("role"),
        "an unknown pin key is refused"
    );
}

#[test]
fn unknown_key_in_a_connector_is_refused() {
    let mut v = fixture();
    v["connectors"][0]["mounting"] = json!("GND");
    assert!(
        read_err(&v).contains("mounting"),
        "an unknown connector key is refused"
    );
}

#[test]
fn unknown_key_in_offboard_is_refused() {
    let mut v = fixture();
    v["connectors"][0]["pins"][1]["offboard"]["note"] = json!("x");
    assert!(
        read_err(&v).contains("note"),
        "an unknown offboard key is refused"
    );
}

#[test]
fn pin_as_a_positional_array_is_refused() {
    let mut v = fixture();
    v["connectors"][0]["pins"][0] = json!(["1", "VDD_5V", null]);
    assert!(read(&v).is_err(), "a pin is an object only");
}

#[test]
fn connector_as_a_positional_array_is_refused() {
    let mut v = fixture();
    v["connectors"][0] = json!(["J1", "SM04B-GHS-TB", null, []]);
    assert!(read(&v).is_err(), "a connector is an object only");
}

#[test]
fn offboard_as_a_positional_array_is_refused() {
    let mut v = fixture();
    v["connectors"][0]["pins"][1]["offboard"] = json!([30000, SOURCE]);
    assert!(read(&v).is_err(), "an offboard is an object only");
}

#[test]
fn negative_capacitance_is_refused() {
    let mut v = fixture();
    v["connectors"][0]["pins"][1]["offboard"]["capacitance_femtofarads"] = json!(-1);
    assert!(read(&v).is_err(), "the capacitance is unsigned");
}

#[test]
fn schema_version_2_is_refused() {
    let mut v = fixture();
    v["schema_version"] = json!(2);
    let err = refusal(&v);
    assert_eq!(err.kind(), "UnsupportedSchemaVersion", "{err}");
}

#[test]
fn connectors_round_trip_through_json() {
    let spec = spec_of(&fixture());
    let json = serde_json::to_value(&spec).unwrap();
    assert_eq!(
        json["connectors"],
        fixture()["connectors"],
        "written as read"
    );
    assert_eq!(spec_of(&json), spec, "round trip");
}

// 6. Table consistency.

#[test]
fn every_port_fits_a_part_of_its_series() {
    for port in &PORTS {
        assert!(
            PARTS
                .iter()
                .any(|p| p.series == port.series && usize::from(p.pins) == port.roles.len()),
            "{} has a vendored part of its series and pin count",
            port.name
        );
    }
}

#[test]
fn every_part_is_used_by_a_port_or_is_a_plain_header() {
    // Not a requirement: this documents which parts no port uses.
    let unused: Vec<_> = PARTS
        .iter()
        .filter(|p| {
            !PORTS
                .iter()
                .any(|q| q.series == p.series && q.roles.len() == usize::from(p.pins))
        })
        .map(|p| p.name)
        .collect();
    assert_eq!(unused, Vec::<&str>::new(), "every part carries some port");
}

#[test]
fn part_names_are_unique() {
    let mut names: Vec<_> = PARTS.iter().map(|p| p.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), PARTS.len(), "part names are unique");
}

#[test]
fn port_names_are_unique() {
    let mut names: Vec<_> = PORTS.iter().map(|p| p.name).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), PORTS.len(), "port names are unique");
}

#[test]
fn part_lookup_finds_every_row_and_only_rows() {
    for p in &PARTS {
        assert_eq!(part(p.name), Some(p), "{} is found by name", p.name);
    }
    assert_eq!(part("sm04b-ghs-tb"), None, "names match exactly");
}

#[test]
fn every_part_has_signal_pins_within_a_connector_width() {
    for p in &PARTS {
        assert!(p.pins >= 1, "{} has a signal pin", p.name);
        assert!(
            matches!(p.series, Series::JstGh | Series::JstSh),
            "{} is a known series",
            p.name
        );
    }
}

#[test]
fn every_citation_hash_is_64_lowercase_hex() {
    let hex = |s: &str| s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
    for p in &PARTS {
        assert!(hex(p.cited.sha256), "{} cites a sha256", p.name);
        assert!(!p.cited.document.is_empty(), "{} names a document", p.name);
        assert!(!p.cited.section.is_empty(), "{} names a section", p.name);
    }
    assert!(hex(aethelgard_ir::ds009::SHA256), "DS-009's sha256");
    assert!(hex(aethelgard_ir::connectors::JST_GH_SHA256), "GH sha256");
    assert!(hex(aethelgard_ir::connectors::JST_SH_SHA256), "SH sha256");
}

#[test]
fn no_name_or_citation_uses_the_mark() {
    let mut strings: Vec<String> = Vec::new();
    for p in &PARTS {
        strings
            .extend([p.name, p.cited.document, p.cited.sha256, p.cited.section].map(str::to_owned));
    }
    for p in &PORTS {
        strings.extend([p.name, p.section].map(str::to_owned));
        strings.extend(p.roles.iter().map(|r| r.name().to_owned()));
    }
    strings.extend(
        [
            aethelgard_ir::ds009::DOCUMENT,
            aethelgard_ir::connectors::JST_GH,
            aethelgard_ir::connectors::JST_SH,
        ]
        .map(str::to_owned),
    );
    // The needle is built, so the mark is not spelt in the source either.
    let mark = ["pix", "hawk"].concat();
    for s in strings {
        assert!(
            !s.to_lowercase().contains(&mark),
            "D2: the mark is never used (`{s}`)"
        );
    }
}

#[test]
fn every_port_ends_in_ground_and_has_one_ground() {
    for port in &PORTS {
        assert_eq!(
            port.roles.last(),
            Some(&Role::Ground),
            "{} ends in GND",
            port.name
        );
        let n = port.roles.iter().filter(|r| **r == Role::Ground).count();
        assert_eq!(n, 1, "{} has one GND", port.name);
    }
}

// 7. Solve is blind to connectors.

fn table() -> PinTable {
    let pins: &[(&str, &[(&str, u8)])] = &[
        ("PA0", &[]),
        ("PA1", &[]),
        ("PA5", &[("SPI1_SCK", 5)]),
        ("PA6", &[("SPI1_MISO", 5)]),
        ("PA7", &[("SPI1_MOSI", 5)]),
        ("PB13", &[("SPI2_SCK", 5)]),
        ("PB14", &[("SPI2_MISO", 5)]),
        ("PB15", &[("SPI2_MOSI", 5)]),
        ("PB6", &[("I2C1_SCL", 4)]),
        ("PB7", &[("I2C1_SDA", 4)]),
    ];
    PinTable {
        part: "STM32H753ZITx".into(),
        package: "TEST".into(),
        source: PinSource {
            repository: "test".into(),
            commit: "test".into(),
            files: BTreeMap::new(),
        },
        pins: pins
            .iter()
            .enumerate()
            .map(|(i, (name, afs))| {
                let pin = Pin {
                    position: i as u16 + 1,
                    alternate_functions: afs.iter().map(|(s, af)| ((*s).into(), *af)).collect(),
                };
                ((*name).into(), pin)
            })
            .collect(),
    }
}

fn solved(v: &Value) -> SolvedTopology {
    solve(&spec_of(v), &table(), HASH).expect("the fixture solves on the test table")
}

#[test]
fn solve_is_blind_to_connectors() {
    let with = solved(&fixture());
    let mut v = fixture();
    v["connectors"] = json!([]);
    let without = solved(&v);
    assert_eq!(
        with, without,
        "RUL-16: the solved topology is the same with and without connectors"
    );
    let json = serde_json::to_string(&with).unwrap();
    assert!(!json.contains("connectors"), "the topology carries none");
    assert!(
        !json.contains("J1"),
        "the topology does not name a connector"
    );
}
