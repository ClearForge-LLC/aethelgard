//! The `board` section (`AET-WO-4100`, ruled on PR #120): a malformed board is
//! refused by name, for each field, and nothing is defaulted (`INV-06`).

use aethelgard_ir::{IrValidationError, Specification};
use serde_json::{Value, json};

fn reference() -> Value {
    serde_json::from_str(include_str!("../../../examples/sentinel-01-minimal.json")).unwrap()
}

/// The reference with the 4-layer board of the CLI's fixture.
fn four() -> Value {
    serde_json::from_str(include_str!(
        "../../aethelgard-cli/tests/fixtures/four-layer.json"
    ))
    .unwrap()
}

fn verdict(v: &Value) -> Result<(), IrValidationError> {
    let spec: Specification = serde_json::from_value(v.clone()).expect("the shape parses");
    spec.validate()
}

fn refused(mut base: Value, edit: impl FnOnce(&mut Value)) -> IrValidationError {
    edit(&mut base);
    verdict(&base).expect_err("refused")
}

#[test]
fn the_reference_and_the_four_layer_fixture_are_valid() {
    verdict(&reference()).unwrap();
    verdict(&four()).unwrap();
}

#[test]
fn a_missing_board_or_key_is_a_shape_error_not_a_default() {
    let mut v = reference();
    v.as_object_mut().unwrap().remove("board");
    assert!(serde_json::from_value::<Specification>(v).is_err());
    for key in ["copper_layers", "layers", "stackup", "rules"] {
        let mut v = reference();
        v["board"].as_object_mut().unwrap().remove(key);
        assert!(serde_json::from_value::<Specification>(v).is_err(), "{key}");
    }
    // A signal layer's `net` is required as a key, `null`.
    let mut v = reference();
    v["board"]["layers"][0].as_object_mut().unwrap().remove("net");
    assert!(serde_json::from_value::<Specification>(v).is_err());
    let mut v = reference();
    v["board"]["surprise"] = json!(1);
    assert!(serde_json::from_value::<Specification>(v).is_err());
}

#[test]
fn a_version_one_specification_is_refused_by_name() {
    let e = refused(reference(), |v| v["schema_version"] = json!(1));
    assert_eq!(e.kind(), "UnsupportedSchemaVersion");
}

#[test]
fn six_layers_and_odd_counts_are_refused_as_unsupported() {
    for n in [0, 1, 3, 6, 8] {
        let e = refused(reference(), |v| v["board"]["copper_layers"] = json!(n));
        assert_eq!(e.kind(), "UnsupportedLayerCount", "{n}");
    }
}

#[test]
fn a_layer_table_of_the_wrong_length_or_names_is_refused() {
    let e = refused(reference(), |v| v["board"]["copper_layers"] = json!(4));
    assert_eq!(e.kind(), "LayerTable");
    let e = refused(reference(), |v| v["board"]["layers"][1]["name"] = json!("In1.Cu"));
    assert_eq!((e.kind(), e.path()), ("LayerTable", Some("board.layers[1].name")));
    let e = refused(four(), |v| {
        let layers = v["board"]["layers"].as_array_mut().unwrap();
        layers.swap(1, 2);
    });
    assert_eq!(e.kind(), "LayerTable");
}

#[test]
fn each_wrong_role_is_refused() {
    // A signal layer naming a net; a plane naming none; an outer plane; an
    // inner signal layer at 4 layers.
    let e = refused(reference(), |v| v["board"]["layers"][0]["net"] = json!("GND"));
    assert_eq!((e.kind(), e.path()), ("LayerRole", Some("board.layers[0]")));
    let e = refused(four(), |v| v["board"]["layers"][1]["net"] = Value::Null);
    assert_eq!(e.kind(), "LayerRole");
    let e = refused(reference(), |v| {
        v["board"]["layers"][1] = json!({"name": "B.Cu", "role": "plane", "net": "GND"});
    });
    assert_eq!(e.kind(), "LayerRole");
    let e = refused(four(), |v| {
        v["board"]["layers"][2] = json!({"name": "In2.Cu", "role": "signal", "net": null});
    });
    assert_eq!(e.kind(), "LayerRole");
    // A role that is no role is a shape error.
    let mut v = four();
    v["board"]["layers"][1]["role"] = json!("power");
    assert!(serde_json::from_value::<Specification>(v).is_err());
}

#[test]
fn a_plane_on_an_undeclared_net_or_a_bad_layout_is_refused() {
    let e = refused(four(), |v| v["board"]["layers"][2]["net"] = json!("VDD_5V"));
    assert_eq!((e.kind(), e.id()), ("PlaneNet", Some("VDD_5V")));
    // A signal net is not a plane's.
    let e = refused(four(), |v| v["board"]["layers"][2]["net"] = json!("I2C_SDA"));
    assert_eq!(e.kind(), "PlaneNet");
    // Two planes on one net.
    let e = refused(four(), |v| v["board"]["layers"][2]["net"] = json!("GND"));
    assert_eq!(e.kind(), "PlaneLayout");
    // No GND plane at 4 layers.
    let e = refused(four(), |v| {
        v["voltage_domains"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id": "VDD_A", "nominal_millivolts": 3300, "tolerance_permille": 50}));
        v["board"]["layers"][1]["net"] = json!("VDD_A");
    });
    assert_eq!(e.kind(), "PlaneLayout");
    // A plane net is an identifier.
    let e = refused(four(), |v| v["board"]["layers"][2]["net"] = json!("VDD 3V3"));
    assert_eq!((e.kind(), e.path()), ("InvalidIdentifier", Some("board.layers[2].net")));
}

#[test]
fn a_dielectric_table_out_of_order_or_zero_is_refused() {
    let e = refused(four(), |v| {
        v["board"]["stackup"]["dielectrics"].as_array_mut().unwrap().pop();
    });
    assert_eq!(e.kind(), "DielectricTable");
    let e = refused(four(), |v| {
        v["board"]["stackup"]["dielectrics"][0]["between"] = json!(["In1.Cu", "F.Cu"]);
    });
    assert_eq!(
        (e.kind(), e.path()),
        ("DielectricTable", Some("board.stackup.dielectrics[0].between"))
    );
    let e = refused(four(), |v| {
        v["board"]["stackup"]["dielectrics"][1]["min_thickness_um"] = json!(0);
    });
    assert_eq!(
        (e.kind(), e.path()),
        ("StackupValue", Some("board.stackup.dielectrics[1].min_thickness_um"))
    );
}

#[test]
fn a_finished_thickness_that_contradicts_itself_is_refused() {
    let set = |field: &'static str, value: u32| {
        move |v: &mut Value| v["board"]["stackup"]["finished_thickness_um"][field] = json!(value)
    };
    for (field, value) in [("nominal", 300), ("nominal", 2100), ("min", 0), ("max", 20_000)] {
        let e = refused(reference(), set(field, value));
        assert_eq!(e.kind(), "FinishedThickness", "{field} {value}");
    }
    // At 4 layers, a minimum below the dielectric minima summed.
    let e = refused(four(), set("min", 1000));
    assert_eq!(e.kind(), "FinishedThickness");
    // At 2 layers, a dielectric minimum the finished minimum and the copper
    // maximum do not imply (the notes state only those two).
    let e = refused(reference(), |v| {
        v["board"]["stackup"]["dielectrics"][0]["min_thickness_um"] = json!(201);
    });
    assert_eq!(e.kind(), "FinishedThickness");
}

#[test]
fn copper_and_permittivity_out_of_range_are_refused() {
    let e = refused(reference(), |v| {
        v["board"]["stackup"]["copper_max_um"]["value"] = json!(0);
    });
    assert_eq!(
        (e.kind(), e.path()),
        ("StackupValue", Some("board.stackup.copper_max_um.value"))
    );
    for er in [0, 999, 100_001] {
        let e = refused(reference(), |v| {
            v["board"]["stackup"]["relative_permittivity_max_permille"]["value"] = json!(er);
        });
        assert_eq!(e.kind(), "StackupValue", "{er}");
    }
}

#[test]
fn a_source_that_is_empty_or_could_forge_a_line_is_refused() {
    for bad in ["", " lead", "trail ", "two\nlines", "tab\there", &"x".repeat(201)] {
        let e = refused(four(), |v| {
            v["board"]["stackup"]["dielectrics"][0]["source"] = json!(bad);
        });
        assert_eq!(
            (e.kind(), e.path()),
            ("InvalidSource", Some("board.stackup.dielectrics[0].source")),
            "{bad:?}"
        );
    }
    let e = refused(reference(), |v| {
        v["board"]["stackup"]["relative_permittivity_max_permille"]["source"] = json!("");
    });
    assert_eq!(e.kind(), "InvalidSource");
}

#[test]
fn rules_under_the_floor_or_other_vias_are_refused_by_name() {
    for (field, value) in [("clearance_um", 199), ("track_width_um", 150)] {
        let e = refused(reference(), |v| v["board"]["rules"][field] = json!(value));
        assert_eq!(e.kind(), "RuleBelowFloor", "{field}");
        assert_eq!(e.path(), Some(format!("board.rules.{field}").as_str()));
    }
    for (field, value) in [
        ("via_drill_um", 250),
        ("via_drill_um", 350),
        ("via_diameter_um", 500),
        ("via_diameter_um", 800),
    ] {
        let e = refused(reference(), |v| v["board"]["rules"][field] = json!(value));
        assert_eq!(e.kind(), "UnsupportedViaRule", "{field} {value}");
    }
    // Above the floor is accepted: the engine honours it.
    let mut v = reference();
    v["board"]["rules"]["clearance_um"] = json!(250);
    verdict(&v).unwrap();
}
