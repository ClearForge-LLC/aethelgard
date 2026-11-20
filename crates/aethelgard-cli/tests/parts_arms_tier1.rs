//! `AET-WO-1316` §1.2: the parts table and the emitter's per-part arms agree,
//! by a test that fails on drift either way, naming the part.
//!
//! Every row of the table is drawn through `schematic::design` (one call, by
//! [`Drawing::of`]) on a minimal specification built from the row itself,
//! which `validate` must pass first, so a refusal is the arm's and not the
//! fixture's. A part in `schematic::KNOWN_PARTS` (the arms, as the emitter's
//! `UnknownPart` lists them) must draw; a row in [`ROWS_WITHOUT_ARM`] must be
//! refused by `UnknownPart` naming its device. So:
//! - a row added with no arm is refused, and is in neither list: it fails;
//! - an arm's part removed from the table: its list entry has no row;
//! - an arm added for a row, its list not updated: the row draws, unlisted.
//!
//! An arm written for a part neither the table nor the list holds is the
//! source scan's in `schematic.rs` (`the_arms_are_the_known_parts`).

mod common;

use aethelgard_eda::schematic::{Drawing, EmitError, KNOWN_PARTS};
use aethelgard_ir::Specification;
use aethelgard_ir::parts::{self, Bus, Lifecycle, Part};
use serde_json::{Value, json};

/// The rows the emitter has no arm for, each refused when drawn: ICP-10111,
/// the twin's stand-in, whose arm is `AET-WO-4600` D4's own row.
const ROWS_WITHOUT_ARM: [&str; 1] = ["ICP10111"];

fn arms() -> Vec<&'static str> {
    KNOWN_PARTS.split(", ").collect()
}

/// Sentinel-01's minimal board with `row` its one device, as `U2` on the I2C
/// bus or `U4` on the SPI bus, fitted so `validate` passes it: the first
/// address the row's straps offer, a bus clocked no faster than the part, its
/// lifecycle accepted, and the board at 1.8 V (brown-out reset off, so the
/// MCU's floor admits it) where the row's supply or I/O range leaves out
/// 3.3 V ± 5 %. Returns it with the device's reference.
fn minimal(row: &Part) -> (Specification, &'static str) {
    let bytes = std::fs::read(common::example("sentinel-01-minimal.json")).unwrap();
    let mut v: Value = serde_json::from_slice(&bytes).unwrap();
    v["acknowledge_unwitnessed"] = json!([]);
    v["accept_lifecycle"] = match row.lifecycle {
        Lifecycle::Active => json!([]),
        l => json!([{ "part": row.part, "status": l.name(), "statement": "AET-WO-1316 test" }]),
    };
    let (reference, prefix) = match &row.bus {
        Bus::I2c { addresses } => {
            v["spi_buses"] = json!([]);
            v["spi_devices"] = json!([]);
            v["i2c_devices"] = json!([{
                "reference": "U2", "part": row.part, "supply": "VDD_3V3",
                "bus": "SENSORS", "address": addresses[0], "pin_capacitance": null
            }]);
            ("U2", "I2C_")
        }
        Bus::Spi { max_clock_hz, .. } => {
            v["i2c_buses"] = json!([]);
            v["i2c_devices"] = json!([]);
            let devices = v["spi_devices"].as_array().unwrap();
            assert_eq!(devices.len(), 1, "the minimal board's SPI devices");
            assert_eq!(devices[0]["reference"], "U4");
            v["spi_devices"][0]["part"] = json!(row.part);
            let hz = v["spi_buses"][0]["speed_hz"].as_u64().unwrap();
            v["spi_buses"][0]["speed_hz"] = json!(hz.min(u64::from(*max_clock_hz)));
            ("U4", "SPI_")
        }
    };
    v["net_constraints"]
        .as_array_mut()
        .unwrap()
        .retain(|c| c["net_name"].as_str().unwrap().starts_with(prefix));
    let (lo, hi) = row.io_millivolts.map_or(row.supply_millivolts, |io| {
        (
            row.supply_millivolts.0.max(io.0),
            row.supply_millivolts.1.min(io.1),
        )
    });
    if lo > 3135 || hi < 3465 {
        v = serde_json::from_str(&v.to_string().replace("VDD_3V3", "VDD_1V8")).unwrap();
        v["voltage_domains"][0]["nominal_millivolts"] = json!(1800);
        v["mcu"]["bor_level"] = json!(0);
    }
    let spec: Specification = serde_json::from_value(v).unwrap();
    if let Err(e) = spec.validate() {
        panic!(
            "{}: its minimal specification is refused by validate, before any arm: {e}",
            row.part
        );
    }
    (spec, reference)
}

#[test]
fn every_row_draws_by_its_arm_or_is_refused_and_every_arm_has_a_row() {
    let table = parts::table().unwrap();
    let rows: Vec<&str> = table.parts().iter().map(|p| p.part.as_str()).collect();
    let arms = arms();
    assert_eq!(arms.len(), 6, "{KNOWN_PARTS}");
    for arm in &arms {
        assert!(
            rows.contains(arm),
            "{arm}: the emitter has an arm for it, and the parts table has no row"
        );
    }
    for r in ROWS_WITHOUT_ARM {
        assert!(
            rows.contains(&r),
            "{r}: pinned as a row with no arm, and the table has no row"
        );
        assert!(
            !arms.contains(&r),
            "{r}: pinned as a row with no arm, and listed as an arm"
        );
    }
    for row in table.parts() {
        let part = row.part.as_str();
        let (spec, reference) = minimal(row);
        let pins = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
        let solved = aethelgard_ir::solve(&spec, &pins, &"0".repeat(64)).unwrap();
        let drawing = Drawing::of(&spec, &solved, &pins);
        match (
            drawing,
            arms.contains(&part),
            ROWS_WITHOUT_ARM.contains(&part),
        ) {
            (Ok(d), true, false) => {
                let nets = d.drawn_nets();
                let on = nets.get(reference);
                assert!(
                    on.is_some_and(|n| n.contains(spec.mcu.supply.as_str())),
                    "{part}: drawn, and `{reference}` is not drawn on the supply: {on:?}"
                );
            }
            (
                Err(EmitError::UnknownPart {
                    reference: r,
                    part: p,
                    known,
                }),
                false,
                true,
            ) => {
                assert_eq!((r.as_str(), p.as_str()), (reference, part));
                assert_eq!(known, KNOWN_PARTS);
            }
            (got, true, _) => panic!("{part}: listed as an arm, and not drawn: {:?}", got.err()),
            (Ok(_), false, _) => panic!("{part}: drawn, and not in KNOWN_PARTS: an arm unlisted"),
            (Err(e), false, false) => {
                panic!(
                    "{part}: a row the emitter has no arm for, not pinned in ROWS_WITHOUT_ARM: {e}"
                )
            }
            (Err(e), false, true) => {
                panic!("{part}: pinned as a row with no arm, refused otherwise: {e}")
            }
        }
    }
}
