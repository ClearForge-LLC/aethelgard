//! `AET-WO-4400i` §2 and D4: `sentinel-02-own`'s U5 (an ICM-42688-P, x right
//! and z up) at the parts table's witness stimulus, 1 g on y, is named not at
//! rest by its declared orientation; without an orientation, its rest is
//! named unknown; and the rest an input run starts from puts 1 g on z.

use aethelgard_ir::{SolvedTopology, Specification};
use aethelgard_twin::{boot, rest};
use std::fs;
use std::path::Path;

fn topology(orientation: bool) -> SolvedTopology {
    let bytes = fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../aethelgard-cli/tests/fixtures/sentinel-02-own.json"),
    )
    .unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    if !orientation {
        for d in v["spi_devices"].as_array_mut().unwrap() {
            d.as_object_mut().unwrap().remove("orientation");
        }
    }
    let spec: Specification = serde_json::from_value(v).unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap()
}

#[test]
fn a_witness_stimulus_that_is_not_at_rest_is_named() {
    let t = topology(true);
    let checks = boot::expected(&t).unwrap().identity.checks;
    assert_eq!(
        rest::not_at_rest(&t, &checks),
        [
            "U5: the parts table's witness stimulus (X 0, Y 1, Z 0 g) is not at rest by its declared orientation (X 0, Y 0, Z 1 g)"
        ]
    );
    // The rest an input run starts from: 1 g on z, and the barometer moved.
    let s = rest::stimuli(&t, &checks, 1_000_000_000).unwrap();
    let u5: Vec<(&str, &str)> = s
        .iter()
        .filter(|x| x.reference == "U5")
        .map(|x| (x.property, x.value.as_str()))
        .collect();
    assert_eq!(
        u5,
        [
            ("AccelerationX", "0"),
            ("AccelerationY", "0"),
            ("AccelerationZ", "1")
        ]
    );
    assert!(
        s.iter()
            .any(|x| x.reference == "U7" && x.value == "101325.080")
    );
    let t = topology(false);
    let checks = boot::expected(&t).unwrap().identity.checks;
    assert_eq!(
        rest::not_at_rest(&t, &checks),
        [
            "U5: the parts table's witness stimulus (X 0, Y 1, Z 0 g); no orientation is declared, so its rest is unknown"
        ]
    );
}
