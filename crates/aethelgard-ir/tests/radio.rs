//! `AET-WO-4411`: a radio module as a part. Each refusal planted into the
//! radio fixture (Sentinel-03 with an ESP32-C3-WROOM-02 on a UART of its
//! own) and refused by name; the fixture itself passes.

use aethelgard_ir::Specification;
use serde_json::{Value, json};

fn fixture() -> Value {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
    let text = std::fs::read_to_string(format!(
        "{root}crates/aethelgard-cli/tests/fixtures/sentinel-03-radio.json"
    ))
    .unwrap();
    serde_json::from_str(&text).unwrap()
}

fn judged(v: &Value) -> Result<(), (String, Option<String>)> {
    let s: Specification = serde_json::from_value(v.clone()).expect("a specification");
    s.validate()
        .map_err(|e| (e.kind().to_owned(), e.path().map(str::to_owned)))
}

/// `(kind, path)` of the refusal `edit` plants.
fn refused(edit: impl FnOnce(&mut Value)) -> (String, Option<String>) {
    let mut v = fixture();
    edit(&mut v);
    judged(&v).expect_err("the plant is refused")
}

fn module(v: &mut Value) -> &mut Value {
    &mut v["radio_modules"][0]
}

fn pull(reference: &str, to: &str) -> Value {
    json!({ "resistor": { "reference": reference, "part": "CRCW0603_F_K", "ohms": 10000, "to": to } })
}

#[test]
fn the_radio_fixture_passes() {
    judged(&fixture()).unwrap();
}

#[test]
fn a_strap_at_the_wrong_level_is_refused() {
    // IO9 low at reset is joint download boot, not SPI boot (p. 13).
    let (kind, path) = refused(|v| module(v)["straps"]["IO9"] = pull("R93", "GND"));
    assert_eq!(kind, "RadioStrapWrongLevel");
    assert_eq!(path.as_deref(), Some("radio_modules[0].straps.IO9"));
    let (kind, _) = refused(|v| module(v)["straps"]["IO2"] = pull("R91", "GND"));
    assert_eq!(kind, "RadioStrapWrongLevel");
}

#[test]
fn a_floating_strap_that_boot_needs_is_refused() {
    let (kind, path) = refused(|v| module(v)["straps"]["IO2"] = json!("open"));
    assert_eq!(kind, "RadioStrapUndriven");
    assert_eq!(path.as_deref(), Some("radio_modules[0].straps.IO2"));
}

#[test]
fn a_strap_another_rail_pulls_is_refused() {
    let (kind, _) = refused(|v| module(v)["straps"]["IO2"] = pull("R91", "VDD_5V"));
    assert_eq!(kind, "RadioStrapPullTo");
}

#[test]
fn every_strap_is_declared_and_no_other() {
    let (kind, _) = refused(|v| {
        module(v)["straps"].as_object_mut().unwrap().remove("IO8");
    });
    assert_eq!(kind, "RadioStrapUndeclared");
    let (kind, _) = refused(|v| module(v)["straps"]["IO3"] = json!("open"));
    assert_eq!(kind, "RadioStrapUnknown");
}

#[test]
fn the_port_is_a_declared_mavlink_port_of_its_own() {
    let (kind, _) = refused(|v| module(v)["serial_port"] = json!("NOPE"));
    assert_eq!(kind, "RadioPortUndeclared");
    let (kind, _) = refused(|v| {
        for p in v["serial_ports"].as_array_mut().unwrap() {
            if p["label"] == "WIFI" {
                p["role"] = json!("gps");
                p["framing"]["baud"] = json!(115200);
            }
        }
    });
    assert_eq!(kind, "RadioPortRole");
    // The MAVLINK port leaves on J7: a module there is two drivers on RX.
    let (kind, _) = refused(|v| {
        module(v)["serial_port"] = json!("MAVLINK");
        v["serial_ports"]
            .as_array_mut()
            .unwrap()
            .retain(|p| p["label"] != "WIFI");
    });
    assert_eq!(kind, "RadioPortShared");
}

#[test]
fn a_port_with_no_module_still_ends_at_a_connector() {
    let mut v = fixture();
    v.as_object_mut().unwrap().remove("radio_modules");
    assert_eq!(judged(&v).unwrap_err().0, "SerialPortNotAtConnector");
}

#[test]
fn the_supply_is_the_mcus_and_in_range() {
    let (kind, _) = refused(|v| module(v)["supply"] = json!("VDD_5V"));
    assert_eq!(kind, "RadioSupplyOutOfRange");
    let (kind, _) = refused(|v| module(v)["supply"] = json!("VDD_9V"));
    assert_eq!(kind, "RadioSupplyUnknown");
}

/// Step 27 alone (`radio::validate`): a second 3.3 V domain needs a whole
/// power tree, whose own checks would refuse first; the rule is the
/// module's.
#[test]
fn a_supply_in_range_but_not_the_mcus_is_refused() {
    let mut v = fixture();
    v["voltage_domains"]
        .as_array_mut()
        .unwrap()
        .push(json!({ "id": "VDD_RF", "nominal_millivolts": 3300, "tolerance_permille": 50 }));
    module(&mut v)["supply"] = json!("VDD_RF");
    module(&mut v)["enable"]["resistor"]["to"] = json!("VDD_RF");
    module(&mut v)["straps"]["IO2"] = pull("R91", "VDD_RF");
    let s: Specification = serde_json::from_value(v).unwrap();
    let e = aethelgard_ir::radio::validate(&s).unwrap_err();
    assert_eq!(e.kind(), "RadioSupplyNotMcu");
}

#[test]
fn the_enable_is_pulled_to_the_module_supply() {
    let (kind, _) = refused(|v| module(v)["enable"]["resistor"]["to"] = json!("GND"));
    assert_eq!(kind, "RadioEnableNotPulled");
}

#[test]
fn the_passives_are_declared_and_held() {
    let (kind, _) = refused(|v| module(v)["decoupling"] = json!([]));
    assert_eq!(kind, "RadioDecouplingUndeclared");
    let (kind, path) = refused(|v| module(v)["decoupling"][0]["part"] = json!("CRCW0603_F_K"));
    assert_eq!(kind, "RadioPassivePartUnknown");
    assert_eq!(path.as_deref(), Some("radio_modules[0].decoupling[0].part"));
    let (kind, _) =
        refused(|v| module(v)["enable"]["resistor"]["part"] = json!("GRM188R71C105KA12"));
    assert_eq!(kind, "RadioPassivePartUnknown");
    let (kind, _) = refused(|v| module(v)["part"] = json!("ESP32-WROOM-32E"));
    assert_eq!(kind, "RadioPartUnknown");
}

/// D5: the row's 500 mA is counted on the module's rail. On the fixture U11
/// (3,000 mA) has 1,753 mA left with it counted: 1,753 more fits, 1,754 does
/// not. Without the module's 500 mA, 2,253 would fit.
#[test]
fn the_rail_counts_the_supply_requirement() {
    let load = |ma: u32| {
        move |v: &mut Value| {
            v["power"]["loads"]
                .as_array_mut()
                .unwrap()
                .push(json!({ "domain": "VDD_3V3", "milliamps": ma, "source": "planted" }));
        }
    };
    let mut fits = fixture();
    load(1753)(&mut fits);
    judged(&fits).unwrap();
    let (kind, _) = refused(load(1754));
    assert_eq!(kind, "RegulatorOverloaded");
}

/// An average or peak declared beside the row's count would hide or double
/// it: a `power.loads` row naming the module is refused.
#[test]
fn a_load_naming_the_module_is_refused() {
    let (kind, _) = refused(|v| {
        v["power"]["loads"].as_array_mut().unwrap().push(
            json!({ "domain": "VDD_3V3", "milliamps": 345, "source": "planted", "reference": "U30" }),
        );
    });
    assert_eq!(kind, "LoadNamesCountedPart");
}

#[test]
fn the_modules_references_are_distinct() {
    let (kind, _) = refused(|v| module(v)["decoupling"][0]["reference"] = json!("R90"));
    assert_eq!(kind, "DuplicateReference");
}

#[test]
fn the_verdict_line_names_firmware_and_radio() {
    let s: Specification = serde_json::from_value(fixture()).unwrap();
    let lines = aethelgard_ir::radio::not_judged(&s);
    assert_eq!(lines.len(), 1);
    assert!(
        lines[0].starts_with("U30 ESP32-C3-WROOM-02-N4: firmware not judged; radio not simulated")
    );
}

/// D4: the footprint holds its keepout on each of a 4-layer board's copper
/// layers; a module on a 2-layer board is refused by name.
#[test]
fn a_radio_module_needs_a_four_layer_board() {
    let mut v = fixture();
    v["board"]["copper_layers"] = json!(2);
    let s: Specification = serde_json::from_value(v).unwrap();
    let e = aethelgard_ir::radio::validate(&s).unwrap_err();
    assert_eq!(e.kind(), "RadioBoardLayers");
    assert_eq!(e.path(), Some("board.copper_layers"));
}

#[test]
fn one_radio_module_per_board() {
    let (kind, path) = refused(|v| {
        let mut second = v["radio_modules"][0].clone();
        second["reference"] = json!("U31");
        second["enable"]["resistor"]["reference"] = json!("R95");
        second["enable"]["capacitor"]["reference"] = json!("C95");
        second["straps"]["IO2"]["resistor"]["reference"] = json!("R96");
        second["decoupling"][0]["reference"] = json!("C96");
        second["decoupling"][1]["reference"] = json!("C97");
        v["radio_modules"].as_array_mut().unwrap().push(second);
    });
    assert_eq!(kind, "RadioModuleTwice");
    assert_eq!(path.as_deref(), Some("radio_modules[1]"));
}

/// Adversarial pass: with no regulator on the module's supply, nothing
/// judges its 500 mA (`INV-06`), so step 27 refuses it.
#[test]
fn a_supply_no_regulator_outputs_is_refused() {
    let mut v = fixture();
    v["power"]["regulators"]
        .as_array_mut()
        .unwrap()
        .retain(|r| r["output"] != "VDD_3V3");
    let s: Specification = serde_json::from_value(v).unwrap();
    let e = aethelgard_ir::radio::validate(&s).unwrap_err();
    assert_eq!(e.kind(), "RadioSupplyUnsourced");
}

/// Adversarial pass (review M1): a pull whose value the pin's own input
/// current defeats holds no level; EN's or a strap's.
#[test]
fn a_pull_too_weak_to_hold_its_level_is_refused() {
    let (kind, path) =
        refused(|v| module(v)["enable"]["resistor"]["ohms"] = json!(4_294_967_295u32));
    assert_eq!(kind, "RadioPullTooWeak");
    assert_eq!(
        path.as_deref(),
        Some("radio_modules[0].enable.resistor.ohms")
    );
    let (kind, path) =
        refused(|v| module(v)["straps"]["IO2"]["resistor"]["ohms"] = json!(20_000_000));
    assert_eq!(kind, "RadioPullTooWeak");
    assert_eq!(
        path.as_deref(),
        Some("radio_modules[0].straps.IO2.resistor.ohms")
    );
    // 15.6 MOhm holds on VDD_3V3 (3,135 mV at its lowest): a quarter is 783 mV,
    // 783 mV / 50 nA = 15.675 MOhm.
    let mut v = fixture();
    module(&mut v)["straps"]["IO2"]["resistor"]["ohms"] = json!(15_600_000);
    judged(&v).unwrap();
}

/// Review M1: EN's RC must keep it under the release voltage for tSTBL.
#[test]
fn an_enable_rc_faster_than_the_supply_settles_is_refused() {
    let (kind, path) = refused(|v| module(v)["enable"]["resistor"]["ohms"] = json!(0));
    assert_eq!(kind, "RadioEnableTooFast");
    assert_eq!(path.as_deref(), Some("radio_modules[0].enable"));
    // 1 uF retained at 50 %: 70 ohm gives 70 x 500,000 pF x 1.386 = 48.5 us,
    // under 50; 73 ohm gives 50.6 us.
    let (kind, _) = refused(|v| module(v)["enable"]["resistor"]["ohms"] = json!(70));
    assert_eq!(kind, "RadioEnableTooFast");
    let mut v = fixture();
    module(&mut v)["enable"]["resistor"]["ohms"] = json!(73);
    judged(&v).unwrap();
    // No derating declared for the capacitor's dielectric: unbounded.
    let (kind, _) = refused(|v| v["power"]["conditions"]["dc_bias_derating"] = json!([]));
    assert_eq!(kind, "RadioEnableTooFast");
}

/// Review M1: the module's own schematic puts 10 uF on 3V3 (p. 33).
#[test]
fn decoupling_under_the_modules_schematic_is_refused() {
    let (kind, path) = refused(|v| {
        module(v)["decoupling"] = json!([{ "reference": "C92", "part": "GCM188R71H104KA57" }]);
    });
    assert_eq!(kind, "RadioDecouplingShort");
    assert_eq!(path.as_deref(), Some("radio_modules[0].decoupling"));
}
