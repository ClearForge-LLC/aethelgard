//! `AET-WO-4400d` (ruled on PR #207): an IMU's orientation, declared once in
//! the specification (version 16), and each refusal by its name. `solve`'s
//! copy (solved 10) is held where the MCU's pin table is, in the CLI's
//! `orientation_tier1`.

use aethelgard_ir::orientation::{Edge, Orientation, Toward};
use aethelgard_ir::{IrValidationError, Specification};
use serde_json::{Value, json};

/// `sentinel-02-own`, which declares the board's forward edge `top` and U5,
/// its ICM-42688-P, with X toward the right and Z up (ruled D5).
const FIXTURE: &str = include_str!("../../aethelgard-cli/tests/fixtures/sentinel-02-own.json");

fn declared() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}

fn spec(v: &Value) -> Specification {
    serde_json::from_value(v.clone()).expect("the fixture is a specification")
}

fn refused(v: &Value) -> IrValidationError {
    spec(v)
        .validate()
        .expect_err("the declaration must be refused")
}

fn u5(v: &mut Value) -> &mut Value {
    v["spi_devices"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["reference"] == "U5")
        .unwrap()
}

#[test]
fn a_declared_orientation_validates() {
    let s = spec(&declared());
    s.validate().unwrap();
    assert_eq!(s.board.forward, Some(Edge::Top));
    let u5 = s
        .declared_spi_devices()
        .iter()
        .find(|d| d.reference == "U5")
        .unwrap();
    assert_eq!(
        u5.orientation,
        Some(Orientation {
            x: Toward::Right,
            z: Toward::Up
        })
    );
}

#[test]
fn an_undeclared_imu_still_validates() {
    let mut v = declared();
    u5(&mut v).as_object_mut().unwrap().remove("orientation");
    v["board"].as_object_mut().unwrap().remove("forward");
    spec(&v).validate().unwrap();
}

#[test]
fn an_orientation_without_a_forward_edge_is_refused_by_name() {
    let mut v = declared();
    v["board"].as_object_mut().unwrap().remove("forward");
    let e = refused(&v);
    assert_eq!((e.kind(), e.id()), ("ForwardUndeclared", Some("U5")), "{e}");
}

#[test]
fn axes_not_at_right_angles_are_refused_never_rounded() {
    for (x, z) in [
        ("up", "up"),
        ("up", "down"),
        ("forward", "aft"),
        ("left", "left"),
    ] {
        let mut v = declared();
        u5(&mut v)["orientation"] = json!({"x": x, "z": z});
        let e = refused(&v);
        assert_eq!(
            (e.kind(), e.id()),
            ("ImuAxesNotPerpendicular", Some("U5")),
            "{x} {z}: {e}"
        );
        assert!(
            e.to_string().contains(x) && e.to_string().contains(z),
            "{e}"
        );
    }
}

#[test]
fn an_orientation_on_a_part_that_is_not_an_imu_is_refused_by_name() {
    // The flash, and an ICM-20948, which no `ardupilot` row names an IMU.
    let mut v = declared();
    v["spi_devices"][0]["orientation"] = json!({"x": "forward", "z": "up"});
    assert_eq!(v["spi_devices"][0]["reference"], "U4");
    let e = refused(&v);
    assert_eq!(
        (e.kind(), e.id()),
        ("OrientationOnNonImu", Some("U4")),
        "{e}"
    );
    let mut v = declared();
    u5(&mut v)["part"] = json!("ICM20948");
    let e = refused(&v);
    assert_eq!(
        (e.kind(), e.id()),
        ("OrientationOnNonImu", Some("U5")),
        "{e}"
    );
}

#[test]
fn an_unknown_word_or_a_rotation_by_angle_is_refused_at_parse() {
    for bad in [
        json!({"x": "north", "z": "up"}),
        json!({"x": "right"}),
        json!({"x": "right", "z": "up", "y": "forward"}),
        json!("ROTATION_ROLL_180_YAW_90"),
        json!({"x": "right", "z": "up", "yaw_deg": 45}),
    ] {
        let mut v = declared();
        u5(&mut v)["orientation"] = bad.clone();
        assert!(
            serde_json::from_value::<Specification>(v).is_err(),
            "{bad} parsed"
        );
    }
    for bad in [json!("nose"), json!("TOP"), json!(0)] {
        let mut v = declared();
        v["board"]["forward"] = bad.clone();
        assert!(
            serde_json::from_value::<Specification>(v).is_err(),
            "{bad} parsed"
        );
    }
}

/// An explicit `null` is refused, as `board.max_outline_um` refuses it.
#[test]
fn an_explicit_null_is_refused_not_read_as_absent() {
    let mut v = declared();
    v["board"]["forward"] = Value::Null;
    assert!(serde_json::from_value::<Specification>(v).is_err());
    let mut v = declared();
    u5(&mut v)["orientation"] = Value::Null;
    assert!(serde_json::from_value::<Specification>(v).is_err());
}

/// Specification 16: a version-15 file is refused by its version, with or
/// without the new keys, never read as today's board.
#[test]
fn a_version_15_file_is_refused_by_its_version() {
    assert_eq!(aethelgard_ir::SPECIFICATION_SCHEMA_VERSION, 29);
    let mut with = declared();
    with["schema_version"] = json!(15);
    let mut without = with.clone();
    u5(&mut without)
        .as_object_mut()
        .unwrap()
        .remove("orientation");
    without["board"].as_object_mut().unwrap().remove("forward");
    for v in [with, without] {
        let e = refused(&v);
        assert_eq!(e.kind(), "UnsupportedSchemaVersion", "{e}");
    }
}
