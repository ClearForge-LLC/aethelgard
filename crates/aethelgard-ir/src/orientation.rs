//! An IMU's orientation, declared once (`AET-WO-4400d`, ruled on PR #207).
//!
//! **The board frame** is the engine's constant (ruled D3): x points to the
//! declared `board.forward` edge, y to its right, and z down, through F.Cu to
//! B.Cu. The board is mounted with F.Cu up. A board mounted any other way is
//! `AHRS_ORIENTATION`'s, a flight-controller parameter outside this engine.
//!
//! **An IMU device declares** where the chip's own +X and +Z axes point in
//! that frame, in board words. The pair fixes the rotation, and Y is Z × X.
//! The placer turns the footprint so that the part's cited axes (`parts.json`
//! `axes`) agree. ArduPilot's `ROTATION_…` is computed from the same pair by
//! the generator and, independently, by the judge (`INV-01`, `RUL-20`).

use serde::de::value::MapAccessDeserializer;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// The board edge the vehicle's nose points to, as the board file draws it
/// seen from F.Cu: `top` is the edge at the least y (KiCad's y points down).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Edge {
    Top,
    Right,
    Bottom,
    Left,
}

/// A direction in the board frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Toward {
    Forward,
    Aft,
    Right,
    Left,
    Up,
    Down,
}

impl Toward {
    /// The word the specification writes.
    pub fn name(self) -> &'static str {
        match self {
            Self::Forward => "forward",
            Self::Aft => "aft",
            Self::Right => "right",
            Self::Left => "left",
            Self::Up => "up",
            Self::Down => "down",
        }
    }

    /// The same line, the other way.
    pub fn opposite(self) -> Self {
        match self {
            Self::Forward => Self::Aft,
            Self::Aft => Self::Forward,
            Self::Right => Self::Left,
            Self::Left => Self::Right,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
        }
    }
}

/// Where the chip's own +X and +Z axes point in the board frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, remote = "Self")]
pub struct Orientation {
    pub x: Toward,
    pub z: Toward,
}

impl Orientation {
    /// Whether the two axes are at right angles: neither the same line nor
    /// its opposite.
    pub fn perpendicular(&self) -> bool {
        self.x != self.z && self.x != self.z.opposite()
    }
}

crate::object_only!(Orientation);

#[cfg(test)]
mod tests {
    use crate::Specification;
    use crate::parts::{PARTS_JSON, parse_parts};

    /// `ImuAxesNotCited`: a declared orientation on an IMU whose row cites no
    /// `axes`, or no `ardupilot.frame`, is refused by name, never assumed
    /// (the work order's §5). The embedded table cites both, so each is
    /// removed from a copy. The specification is the crate's own power-board
    /// fixture with U5 made the ICM-42688-P as ruled (D5), inside the package:
    /// CI's package gate refuses an `include_str!` reaching outside the crate.
    #[test]
    fn an_imu_without_cited_axes_or_frame_is_refused_by_name() {
        let mut doc: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/sentinel-02-power.json")).unwrap();
        doc["board"]["forward"] = serde_json::json!("top");
        let u5 = doc["spi_devices"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|d| d["reference"] == "U5")
            .unwrap();
        u5["part"] = serde_json::json!("ICM42688P");
        u5["supply"] = serde_json::json!("VDD_3V3");
        u5["orientation"] = serde_json::json!({"x": "right", "z": "up"});
        // The ICM20948's end-of-life acceptance would now name no device.
        doc["accept_lifecycle"]
            .as_array_mut()
            .unwrap()
            .retain(|a| a["part"] != "ICM20948");
        let spec: Specification = serde_json::from_value(doc).unwrap();
        assert_eq!(spec.validate_parts(crate::parts::table().unwrap()), Ok(()));
        for (missing, pointer) in [("axes", "/axes"), ("ardupilot.frame", "/ardupilot/frame")] {
            let mut doc: serde_json::Value = serde_json::from_str(PARTS_JSON).unwrap();
            let row = doc["parts"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|p| p["part"] == "ICM42688P")
                .unwrap();
            *row.pointer_mut(pointer).unwrap() = serde_json::Value::Null;
            let table = parse_parts(&doc.to_string()).unwrap();
            let e = spec.validate_parts(&table).unwrap_err();
            assert_eq!((e.kind(), e.id()), ("ImuAxesNotCited", Some("U5")), "{e}");
            assert!(e.to_string().contains(missing), "{e}");
        }
    }
}
