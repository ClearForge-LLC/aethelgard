//! `AET-WO-4300d`: the Sentinel-02 power fixture as the CLI tests use it.

use serde_json::{Value, json};

/// The fixture without its crossing and the 1.8 V IMU behind it, which wait
/// for the sensor table's output delay (`K6`).
pub fn power_tree() -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../aethelgard-ir/tests/fixtures/sentinel-02-power.json");
    let mut v: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    v["power"]["crossings"] = json!([]);
    for key in ["spi_devices", "spi_buses", "net_constraints"] {
        let kept: Vec<Value> = v[key]
            .as_array()
            .unwrap()
            .iter()
            .filter(|x| {
                !(x["reference"] == "U5"
                    || x["id"] == "IMU"
                    || x["net_name"].as_str().is_some_and(|n| n.starts_with("IMU")))
            })
            .cloned()
            .collect();
        v[key] = Value::Array(kept);
    }
    v
}
