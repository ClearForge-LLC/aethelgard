//! `net_classes` and `diff_pairs` (`AET-WO-1308` §1.1 and §1.3, FEEDBACK F2 to
//! F4): each refusal is named, and nothing is defaulted (`INV-06`).

use aethelgard_ir::{IrValidationError, Specification};
use serde_json::{Value, json};

fn reference() -> Value {
    serde_json::from_str(include_str!("../../../examples/sentinel-01-minimal.json")).unwrap()
}

fn verdict(v: &Value) -> Result<(), IrValidationError> {
    let spec: Specification = serde_json::from_value(v.clone()).expect("the shape parses");
    spec.validate()
}

/// The reference with `classes` and `pairs` declared, refused.
fn refused(classes: Value, pairs: Value) -> IrValidationError {
    let mut v = reference();
    v["net_classes"] = classes;
    v["diff_pairs"] = pairs;
    verdict(&v).expect_err("refused")
}

fn power() -> Value {
    json!({"name": "power", "track_width_um": 250,
           "via": {"diameter_um": 700, "drill_um": 300}, "nets": ["VDD_3V3", "GND"]})
}

fn pair() -> Value {
    json!({"name": "USB", "nets": ["SPI_SCK", "SPI_MOSI"], "gap_um": 300, "width_um": 200})
}

fn class(name: &str, nets: Value) -> Value {
    json!({"name": name, "track_width_um": 250, "via": null, "nets": nets})
}

#[test]
fn a_power_class_and_a_pair_are_accepted() {
    let mut v = reference();
    verdict(&v).unwrap();
    v["net_classes"] = json!([power(), class("sig", json!(["I2C_SDA"]))]);
    v["diff_pairs"] = json!([pair()]);
    verdict(&v).unwrap();
}

#[test]
fn a_version_three_file_is_refused_by_version() {
    let mut v = reference();
    v["schema_version"] = json!(3);
    assert_eq!(verdict(&v).unwrap_err().kind(), "UnsupportedSchemaVersion");
}

#[test]
fn the_two_keys_are_required_and_closed() {
    for key in ["net_classes", "diff_pairs"] {
        let mut v = reference();
        v.as_object_mut().unwrap().remove(key);
        assert!(serde_json::from_value::<Specification>(v).is_err(), "{key}");
    }
    // `via` is required as a key, `null`; no key is unknown.
    let mut c = power();
    c.as_object_mut().unwrap().remove("via");
    let mut v = reference();
    v["net_classes"] = json!([c]);
    assert!(serde_json::from_value::<Specification>(v).is_err());
    let mut c = power();
    c["surprise"] = json!(1);
    let mut v = reference();
    v["net_classes"] = json!([c]);
    assert!(serde_json::from_value::<Specification>(v).is_err());
    let mut p = pair();
    p["surprise"] = json!(1);
    let mut v = reference();
    v["diff_pairs"] = json!([p]);
    assert!(serde_json::from_value::<Specification>(v).is_err());
}

#[test]
fn a_net_in_two_classes_is_refused_by_name() {
    let e = refused(
        json!([
            class("a", json!(["I2C_SDA"])),
            class("b", json!(["I2C_SCL", "I2C_SDA"]))
        ]),
        json!([]),
    );
    assert_eq!(e.kind(), "NetInTwoClasses");
    assert_eq!(e.id(), Some("I2C_SDA"));
    let m = e.to_string();
    assert!(m.contains("`a`") && m.contains("`b`"), "{m}");
}

#[test]
fn a_pair_is_its_nets_class() {
    let e = refused(json!([class("a", json!(["SPI_SCK"]))]), json!([pair()]));
    assert_eq!(e.kind(), "NetInTwoClasses");
    assert_eq!(e.id(), Some("SPI_SCK"));
    let mut other = pair();
    other["name"] = json!("OTHER");
    other["nets"] = json!(["SPI_MOSI", "SPI_MISO"]);
    let e = refused(json!([]), json!([pair(), other]));
    assert_eq!(e.kind(), "NetInTwoClasses");
    assert_eq!(e.id(), Some("SPI_MOSI"));
}

#[test]
fn a_class_naming_no_existing_net_is_refused() {
    let e = refused(json!([class("a", json!(["NOT_A_NET"]))]), json!([]));
    assert_eq!(e.kind(), "ClassUnknownNet");
    assert_eq!(e.id(), Some("NOT_A_NET"));
    assert!(e.to_string().contains("`a`"), "{e}");
    let mut p = pair();
    p["nets"] = json!(["SPI_SCK", "NOPE"]);
    let e = refused(json!([]), json!([p]));
    assert_eq!(e.kind(), "ClassUnknownNet");
    assert_eq!(e.id(), Some("NOPE"));
    let e = refused(json!([class("a", json!([]))]), json!([]));
    assert_eq!(e.kind(), "ClassNoNets");
}

#[test]
fn a_width_under_the_floor_or_the_rules_is_refused() {
    for (width, ok) in [(199, false), (200, true)] {
        let mut c = class("a", json!(["I2C_SDA"]));
        c["track_width_um"] = json!(width);
        let mut v = reference();
        v["net_classes"] = json!([c]);
        assert_eq!(verdict(&v).is_ok(), ok, "{width}");
    }
    let mut c = class("a", json!(["I2C_SDA"]));
    c["track_width_um"] = json!(150);
    let e = refused(json!([c]), json!([]));
    assert_eq!(e.kind(), "ClassBelowMinimum");
    assert_eq!(e.path(), Some("net_classes[0].track_width_um"));
    assert!(
        e.to_string().contains("150") && e.to_string().contains("200"),
        "{e}"
    );
    // The declared rule raises the minimum above the floor.
    let mut v = reference();
    v["board"]["rules"]["track_width_um"] = json!(250);
    let mut c = class("a", json!(["I2C_SDA"]));
    c["track_width_um"] = json!(220);
    v["net_classes"] = json!([c]);
    let e = verdict(&v).unwrap_err();
    assert_eq!(e.kind(), "ClassBelowMinimum");
    assert!(e.to_string().contains("250"), "{e}");
    // A pair's width is held to the same minimum, and a class to the maximum.
    let mut p = pair();
    p["width_um"] = json!(100);
    let e = refused(json!([]), json!([p]));
    assert_eq!(e.kind(), "ClassBelowMinimum");
    assert_eq!(e.path(), Some("diff_pairs[0].width_um"));
    let mut c = class("a", json!(["I2C_SDA"]));
    c["track_width_um"] = json!(10_001);
    assert_eq!(refused(json!([c]), json!([])).kind(), "ClassAboveMaximum");
}

#[test]
fn a_via_whose_ring_is_under_the_rules_is_refused() {
    // The rules' ring is (600 - 300) / 2 = 150 um.
    let via = |diameter: u32, drill: u32| {
        let mut c = class("a", json!(["I2C_SDA"]));
        c["via"] = json!({"diameter_um": diameter, "drill_um": drill});
        refused(json!([c]), json!([]))
    };
    let e = via(500, 300);
    assert_eq!(e.kind(), "ViaRingUnderRules");
    assert_eq!(e.id(), Some("a"));
    let m = e.to_string();
    assert!(m.contains("100 um") && m.contains("150 um"), "{m}");
    // A larger drill on a pad that keeps no ring, and a pad at or under its drill.
    assert_eq!(via(700, 500).kind(), "ViaRingUnderRules");
    assert_eq!(via(300, 300).kind(), "ViaRingUnderRules");
    assert_eq!(via(200, 300).kind(), "ViaRingUnderRules");
    // Exactly the rules' ring is accepted, as is a wider pad on a larger drill.
    for (d, dr) in [(600, 300), (700, 400)] {
        let mut c = class("a", json!(["I2C_SDA"]));
        c["via"] = json!({"diameter_um": d, "drill_um": dr});
        let mut v = reference();
        v["net_classes"] = json!([c]);
        verdict(&v).unwrap();
    }
    // The drill is at least the rules', and the pad at most the board bound.
    let e = via(500, 200);
    assert_eq!(e.kind(), "ClassBelowMinimum");
    assert_eq!(e.path(), Some("net_classes[0].via.drill_um"));
    let e = via(10_001, 300);
    assert_eq!(e.kind(), "ClassAboveMaximum");
}

#[test]
fn a_pair_needs_two_distinct_nets() {
    for nets in [
        json!(["SPI_SCK"]),
        json!(["SPI_SCK", "SPI_SCK"]),
        json!(["SPI_SCK", "SPI_MOSI", "SPI_MISO"]),
    ] {
        let mut p = pair();
        p["nets"] = nets;
        assert_eq!(refused(json!([]), json!([p])).kind(), "PairNets");
    }
}

#[test]
fn a_pair_on_a_rail_or_ground_is_refused() {
    for rail in ["VDD_3V3", "GND"] {
        let mut p = pair();
        p["nets"] = json!(["SPI_SCK", rail]);
        let e = refused(json!([]), json!([p]));
        assert_eq!(e.kind(), "PairOnRail");
        assert_eq!(e.id(), Some("USB"));
        assert!(e.to_string().contains(rail), "{e}");
    }
}

#[test]
fn a_pair_gap_is_at_least_the_clearance() {
    let mut p = pair();
    p["gap_um"] = json!(199);
    let e = refused(json!([]), json!([p]));
    assert_eq!(e.kind(), "ClassBelowMinimum");
    assert_eq!(e.path(), Some("diff_pairs[0].gap_um"));
    let mut p = pair();
    p["gap_um"] = json!(200);
    let mut v = reference();
    v["diff_pairs"] = json!([p]);
    verdict(&v).unwrap();
    let mut p = pair();
    p["gap_um"] = json!(10_001);
    assert_eq!(refused(json!([]), json!([p])).kind(), "ClassAboveMaximum");
}

#[test]
fn names_are_identifiers_and_unique_across_classes_and_pairs() {
    let e = refused(
        json!([
            class("a", json!(["I2C_SDA"])),
            class("a", json!(["I2C_SCL"]))
        ]),
        json!([]),
    );
    assert_eq!(e.kind(), "DuplicateClassName");
    assert_eq!(e.id(), Some("a"));
    let mut p = pair();
    p["name"] = json!("a");
    let e = refused(json!([class("a", json!(["I2C_SDA"]))]), json!([p]));
    assert_eq!(e.kind(), "DuplicateClassName");
    let e = refused(json!([class("a b", json!(["I2C_SDA"]))]), json!([]));
    assert_eq!(e.kind(), "InvalidIdentifier");
    assert_eq!(e.path(), Some("net_classes[0].name"));
    let mut p = pair();
    p["name"] = json!("");
    let e = refused(json!([]), json!([p]));
    assert_eq!(e.kind(), "InvalidIdentifier");
    assert_eq!(e.path(), Some("diff_pairs[0].name"));
}

#[test]
fn a_class_net_holding_a_quote_or_a_wildcard_is_refused() {
    // RUL-14 admits `'` in a net name; the gate's KiCad rules quote with it.
    let text =
        include_str!("../../../examples/sentinel-01-minimal.json").replace("I2C_SDA", "I2C'SDA");
    let mut v: Value = serde_json::from_str(&text).unwrap();
    verdict(&v).unwrap();
    v["net_classes"] = json!([class("sig", json!(["I2C'SDA"]))]);
    let e = verdict(&v).unwrap_err();
    assert_eq!(e.kind(), "ClassNetNotLiteral");
    assert_eq!(e.id(), Some("I2C'SDA"));
    // `?` is KiCad's one-character wildcard in `==` (adversarial pass: a class
    // on `SPI_SC?` bound `SPI_SCK` too).
    let text =
        include_str!("../../../examples/sentinel-01-minimal.json").replace("I2C_SCL", "SPI_SC?");
    let mut v: Value = serde_json::from_str(&text).unwrap();
    verdict(&v).unwrap();
    v["net_classes"] = json!([class("sig", json!(["SPI_SC?"]))]);
    assert_eq!(verdict(&v).unwrap_err().kind(), "ClassNetNotLiteral");
    v["net_classes"] = json!([]);
    v["diff_pairs"] =
        json!([{"name": "P", "nets": ["SPI_SC?", "SPI_MOSI"], "gap_um": 300, "width_um": 200}]);
    assert_eq!(verdict(&v).unwrap_err().kind(), "ClassNetNotLiteral");
}
