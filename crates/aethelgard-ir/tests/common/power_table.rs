//! The power tests' shared helpers (`tests/power.rs`, `tests/readiness_4300k.rs`):
//! a specification from JSON, its refusal, a power refusal by kind and id, and
//! the embedded power parts table as JSON for a copy with one limit changed.

#![allow(dead_code)]

use aethelgard_ir::power::Refusal;
use aethelgard_ir::power_parts::POWER_PARTS_JSON;
use aethelgard_ir::{IrValidationError as E, Specification};
use serde_json::Value;

pub fn spec(v: &Value) -> Specification {
    serde_json::from_value(v.clone()).expect("a specification")
}

pub fn refused(v: &Value) -> E {
    spec(v).validate().expect_err("refused")
}

/// A power refusal of `kind` naming `id`, its `kind()` the refusal's name.
pub fn assert_power(err: &E, kind: Refusal, id: &str) {
    match err {
        E::Power { kind: k, id: i, .. } => {
            assert_eq!((*k, i.as_str()), (kind, id), "{err}");
            assert_eq!(err.kind(), kind.name(), "{err}");
        }
        other => panic!("not a power refusal: {other}"),
    }
}

/// The embedded table as JSON, for a copy with one limit changed.
pub fn table_json() -> Value {
    serde_json::from_str(POWER_PARTS_JSON).unwrap()
}

pub fn row<'v>(t: &'v mut Value, part: &str) -> &'v mut Value {
    t["parts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["part"] == part)
        .unwrap()
}
