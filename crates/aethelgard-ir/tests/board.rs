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
fn an_unknown_board_key_is_a_shape_error() {
    // Missing keys are `validate.rs`'s `removing_any_key_of_any_object_is_refused`.
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
    let e = refused(reference(), |v| {
        v["board"]["layers"][1]["name"] = json!("In1.Cu")
    });
    assert_eq!(
        (e.kind(), e.path()),
        ("LayerTable", Some("board.layers[1].name"))
    );
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
    let e = refused(reference(), |v| {
        v["board"]["layers"][0]["net"] = json!("GND")
    });
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
    let e = refused(four(), |v| {
        v["board"]["layers"][2]["net"] = json!("I2C_SDA")
    });
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
    let e = refused(four(), |v| {
        v["board"]["layers"][2]["net"] = json!("VDD 3V3")
    });
    assert_eq!(
        (e.kind(), e.path()),
        ("InvalidIdentifier", Some("board.layers[2].net"))
    );
}

#[test]
fn a_dielectric_table_out_of_order_or_zero_is_refused() {
    let e = refused(four(), |v| {
        v["board"]["stackup"]["dielectrics"]
            .as_array_mut()
            .unwrap()
            .pop();
    });
    assert_eq!(e.kind(), "DielectricTable");
    let e = refused(four(), |v| {
        v["board"]["stackup"]["dielectrics"][0]["between"] = json!(["In1.Cu", "F.Cu"]);
    });
    assert_eq!(
        (e.kind(), e.path()),
        (
            "DielectricTable",
            Some("board.stackup.dielectrics[0].between")
        )
    );
    let e = refused(four(), |v| {
        v["board"]["stackup"]["dielectrics"][1]["min_thickness_um"] = json!(0);
    });
    assert_eq!(
        (e.kind(), e.path()),
        (
            "StackupValue",
            Some("board.stackup.dielectrics[1].min_thickness_um")
        )
    );
}

#[test]
fn a_finished_thickness_that_contradicts_itself_is_refused() {
    let set = |field: &'static str, value: u32| {
        move |v: &mut Value| v["board"]["stackup"]["finished_thickness_um"][field] = json!(value)
    };
    for (field, value) in [
        ("nominal", 300),
        ("nominal", 2100),
        ("min", 0),
        ("max", 20_000),
    ] {
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

/// `AET-WO-1303b`: the supply gate's inputs, each optional; where declared,
/// a copper minimum may not exceed `copper_max_um` (the §7.1 row), the
/// plating is above 0, the temperature 1 to 300 °C, a dielectric's maximum
/// at least its minimum, and every source valid. Absent, the specification
/// is as before, and serializes as before.
#[test]
fn the_supply_inputs_are_judged_where_declared() {
    let with = |v: &mut Value, key: &str, value: u32| {
        v["board"]["stackup"][key] = json!({"value": value, "source": "a source"});
    };
    assert!(verdict(&reference()).is_ok());
    let mut ok = reference();
    for (k, value) in [
        ("outer_copper_min_um", 30),
        ("plane_copper_min_um", 15),
        ("plating_min_um", 18),
        ("copper_temperature_max_c", 125),
    ] {
        with(&mut ok, k, value);
    }
    ok["board"]["stackup"]["dielectrics"][0]["max_thickness_um"] = json!(1940);
    assert!(verdict(&ok).is_ok());
    // absent keys stay absent when written back
    let spec: Specification = serde_json::from_value(reference()).unwrap();
    let back = serde_json::to_value(&spec).unwrap();
    assert!(
        back["board"]["stackup"]
            .get("outer_copper_min_um")
            .is_none()
    );
    assert!(
        back["board"]["stackup"]["dielectrics"][0]
            .get("max_thickness_um")
            .is_none()
    );
    for (key, value, path) in [
        (
            "outer_copper_min_um",
            71,
            "board.stackup.outer_copper_min_um.value",
        ),
        (
            "plane_copper_min_um",
            71,
            "board.stackup.plane_copper_min_um.value",
        ),
        (
            "outer_copper_min_um",
            0,
            "board.stackup.outer_copper_min_um.value",
        ),
        ("plating_min_um", 0, "board.stackup.plating_min_um.value"),
        (
            "copper_temperature_max_c",
            0,
            "board.stackup.copper_temperature_max_c.value",
        ),
        (
            "copper_temperature_max_c",
            301,
            "board.stackup.copper_temperature_max_c.value",
        ),
    ] {
        let e = refused(ok.clone(), |v| with(v, key, value));
        assert_eq!(
            (e.kind(), e.path()),
            ("StackupValue", Some(path)),
            "{key} {value}"
        );
    }
    // the minimum may equal the maximum
    let mut at = ok.clone();
    with(&mut at, "outer_copper_min_um", 70);
    assert!(verdict(&at).is_ok());
    let e = refused(ok.clone(), |v| {
        v["board"]["stackup"]["dielectrics"][0]["max_thickness_um"] = json!(199);
    });
    assert_eq!(
        (e.kind(), e.path()),
        (
            "StackupValue",
            Some("board.stackup.dielectrics[0].max_thickness_um")
        )
    );
    let e = refused(ok.clone(), |v| {
        v["board"]["stackup"]["plating_min_um"]["source"] = json!("");
    });
    assert_eq!(
        (e.kind(), e.path()),
        ("InvalidSource", Some("board.stackup.plating_min_um.source"))
    );
    // a present key holds a value: null is refused as the wrong type
    let mut null = ok.clone();
    null["board"]["stackup"]["plating_min_um"] = Value::Null;
    assert!(serde_json::from_value::<Specification>(null).is_err());
}

/// The fabricator's tolerances (`AET-WO-1303b`, ruling 6974760231): each
/// entry optional, judged where declared, its source required.
#[test]
fn the_fabricators_tolerances_are_judged_where_declared() {
    let tol = |hp: u32, plus: u32, minus: u32, track: u32| {
        json!({
            "hole_position_um": {"value": hp, "source": "s"},
            "hole_size_plus_um": {"value": plus, "source": "s"},
            "hole_size_minus_um": {"value": minus, "source": "s"},
            "track_width_permille": {"value": track, "source": "s"},
            "via_ring_um": {"value": 50, "source": "s"},
        })
    };
    let mut ok = reference();
    ok["board"]["stackup"]["fab_tolerances"] = tol(75, 130, 80, 200);
    assert!(verdict(&ok).is_ok());
    // an empty block is a declaration of nothing, and accepted
    let mut empty = reference();
    empty["board"]["stackup"]["fab_tolerances"] = json!({});
    assert!(verdict(&empty).is_ok());
    for (t, path) in [
        (tol(1001, 130, 80, 200), "hole_position_um"),
        (tol(75, 1001, 80, 200), "hole_size_plus_um"),
        (tol(75, 130, 1001, 200), "hole_size_minus_um"),
        (tol(75, 130, 80, 1000), "track_width_permille"),
        (
            {
                let mut t = tol(75, 130, 80, 200);
                t["via_ring_um"]["value"] = json!(1001);
                t
            },
            "via_ring_um",
        ),
    ] {
        let e = refused(reference(), |v| {
            v["board"]["stackup"]["fab_tolerances"] = t.clone()
        });
        assert_eq!(
            (e.kind(), e.path()),
            (
                "StackupValue",
                Some(format!("board.stackup.fab_tolerances.{path}.value").as_str())
            )
        );
    }
    let e = refused(ok.clone(), |v| {
        v["board"]["stackup"]["fab_tolerances"]["hole_position_um"]["source"] = json!("");
    });
    assert_eq!(
        (e.kind(), e.path()),
        (
            "InvalidSource",
            Some("board.stackup.fab_tolerances.hole_position_um.source")
        )
    );
    let mut unknown = ok.clone();
    unknown["board"]["stackup"]["fab_tolerances"]["drill_um"] = json!({"value": 1, "source": "s"});
    assert!(serde_json::from_value::<Specification>(unknown).is_err());
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
    for bad in [
        "",
        " lead",
        "trail ",
        "two\nlines",
        "tab\there",
        &"x".repeat(201),
    ] {
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

/// `AET-WO-1313` §2.1: `board.max_outline_um` is optional (PR #191 Q1): its
/// absence is today's placer and is not written back; a bound of 1 to
/// 1 000 000 micrometres a side is valid; anything else is refused by name.
#[test]
fn an_outline_bound_is_optional_and_refused_by_name_out_of_range() {
    let spec: Specification = serde_json::from_value(reference()).unwrap();
    assert_eq!(spec.board.max_outline_um, None);
    assert!(
        serde_json::to_value(&spec).unwrap()["board"]
            .get("max_outline_um")
            .is_none()
    );
    let mut v = reference();
    v["board"]["max_outline_um"] = json!({"width": 1, "height": 1_000_000});
    verdict(&v).unwrap();
    let spec: Specification = serde_json::from_value(v).unwrap();
    assert_eq!(
        serde_json::to_value(&spec).unwrap()["board"]["max_outline_um"],
        json!({"width": 1, "height": 1_000_000})
    );
    for (field, value) in [
        ("width", 0),
        ("height", 0),
        ("width", 1_000_001),
        ("height", 4_096_000),
    ] {
        let e = refused(reference(), |v| {
            v["board"]["max_outline_um"] = json!({"width": 100_000, "height": 100_000});
            v["board"]["max_outline_um"][field] = json!(value);
        });
        assert_eq!(e.kind(), "OutlineBound", "{field} {value}");
        assert_eq!(
            e.path(),
            Some(format!("board.max_outline_um.{field}").as_str())
        );
    }
}

/// A malformed bound is a shape error; `null` too, as `test_points` and
/// `mcu.reset_switch` refuse it (ruled F4).
#[test]
fn a_malformed_outline_bound_is_a_shape_error() {
    for bad in [
        json!({"width": 100_000}),
        json!({"width": 100_000, "height": 100_000, "depth": 1}),
        json!({"width": 99_999.5, "height": 100_000}),
        json!({"width": -1, "height": 100_000}),
        json!([100_000, 100_000]),
        json!(null),
    ] {
        let mut v = reference();
        v["board"]["max_outline_um"] = bad.clone();
        assert!(
            serde_json::from_value::<Specification>(v).is_err(),
            "{bad} parsed"
        );
    }
    // The keys whose behaviour this follows refuse `null` the same way.
    for (path, key) in [("", "test_points"), ("mcu", "reset_switch")] {
        let mut v = reference();
        let target = if path.is_empty() {
            &mut v
        } else {
            &mut v[path]
        };
        target[key] = json!(null);
        assert!(serde_json::from_value::<Specification>(v).is_err(), "{key}");
    }
}

/// Specification 15 (`AET-WO-1313`): a version-14 file is refused by its
/// version, with or without `board.max_outline_um`, never read as today's
/// board.
#[test]
fn a_version_14_file_is_refused_by_its_version() {
    assert_eq!(aethelgard_ir::SPECIFICATION_SCHEMA_VERSION, 18);
    for bound in [None, Some(json!({"width": 100_000, "height": 100_000}))] {
        let e = refused(reference(), |v| {
            v["schema_version"] = json!(14);
            if let Some(b) = bound {
                v["board"]["max_outline_um"] = b;
            }
        });
        assert!(
            matches!(
                e,
                IrValidationError::UnsupportedSchemaVersion {
                    version: 14,
                    expected: 18
                }
            ),
            "{e}"
        );
    }
}
