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
//! An arm written for a part neither the table nor the list holds is
//! refused by `schematic_arms.rs`'s rows-against-`KNOWN_PARTS` test.

mod common;

use aethelgard_eda::schematic::{Drawing, EmitError, KNOWN_PARTS};
use aethelgard_ir::Specification;
use aethelgard_ir::parts::{self, Bus, Lifecycle, Part};
use serde_json::{Value, json};

/// The rows the emitter has no arm for, each refused when drawn: ICP-10111,
/// the twin's stand-in, whose arm is `AET-WO-4600` D4's own row.
const ROWS_WITHOUT_ARM: [&str; 1] = ["ICP10111"];

/// Sentinel-01's minimal board with `row` its one device, as `U2` on the I2C
/// bus or `U4` on the SPI bus, fitted so `validate` passes it: the first
/// address the row's straps offer, a bus clocked no faster than the part, its
/// lifecycle accepted, and the board at 1.8 V (brown-out reset off, so the
/// MCU's floor admits it) where the row's supply or I/O range leaves out
/// 3.3 V ± 5 %. The device declares its pin capacitance at the row's
/// typical value where the row cites no maximum (`AET-WO-1210`: a drawn
/// board's SPI device needs one or the other). Returns it with the device's
/// reference.
fn minimal(row: &Part) -> (Specification, &'static str) {
    let (mut v, reference) = fitted(row);
    if let (Bus::Spi { .. }, None, Some(ff)) = (
        &row.bus,
        row.pin_capacitance_femtofarads,
        row.pin_capacitance_typical_femtofarads,
    ) {
        v["spi_devices"][0]["pin_capacitance"] = json!({ "declared": {
            "femtofarads": ff,
            "condition": "AET-WO-1210 test: the row's typical value",
            "source": "the parts table's typical pin capacitance"
        }});
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

/// [`minimal`]'s board before its declaration and before `validate`: the
/// device's `pin_capacitance` is `null`.
fn fitted(row: &Part) -> (Value, &'static str) {
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
    v["spi_devices"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .for_each(|d| d["pin_capacitance"] = Value::Null);
    (v, reference)
}

#[test]
fn every_row_draws_by_its_arm_or_is_refused_and_every_arm_has_a_row() {
    let table = parts::table().unwrap();
    // A pin device's row is a busless part's (`pin_parts`, `AET-WO-1211a`);
    // its arm draws it from the row and the declaration, and
    // `pin_devices_drawn_tier1.rs` draws each (`AET-WO-1211d`).
    let rows: Vec<&str> = table
        .parts()
        .iter()
        .map(|p| p.part.as_str())
        .chain(table.pin_parts().iter().map(|p| p.part.as_str()))
        .collect();
    let arms: Vec<&str> = KNOWN_PARTS.split(", ").collect();
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
                assert_eq!((r.as_str(), p.as_str()), (reference, part), "{part}");
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

/// `AET-WO-1210` §1.3, the coverage test (`AET-WO-1209` §1.1): for every row
/// of the parts table, on a drawn board with the device declaring nothing,
/// `validate`'s rule 1 refuses exactly when `extract`'s own precondition
/// does (`extract::lines`, `UnboundedPinCapacitance`, the same device on the
/// same line), and the CLI's rule 3 (`drawable_parts`, read from
/// `KNOWN_PARTS`) exactly when `emit`'s draw does (`UnknownPart`). The topology
/// is solved from the same board declared `board.drawn: false`, which neither
/// rule holds, so `extract` is reached on a board `validate` refuses.
#[test]
fn validate_refuses_a_drawn_board_exactly_where_extract_and_emit_would() {
    let arms: Vec<&str> = KNOWN_PARTS.split(", ").collect();
    let (mut early, mut late) = (Vec::new(), Vec::new());
    for row in parts::table().unwrap().parts() {
        let part = row.part.as_str();
        let (v, reference) = fitted(row);
        let spec: Specification = serde_json::from_value(v.clone()).unwrap();
        let mut stand_in = v;
        stand_in["board"]["drawn"] = json!(false);
        let stand_in: Specification = serde_json::from_value(stand_in).unwrap();
        let pins = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
        let solved = aethelgard_ir::solve(&stand_in, &pins, &"0".repeat(64))
            .unwrap_or_else(|e| panic!("{part}: the stand-in does not solve: {e}"));

        let rule_1 = match spec.validate() {
            Ok(()) => None,
            Err(e) if e.kind() == "UnboundedPinCapacitance" => Some(e.to_string()),
            Err(e) => panic!("{part}: refused by validate otherwise: {e}"),
        };
        let extract = match aethelgard_eda::extract::lines(&spec, &solved, &pins) {
            Err(r) if r.kind == "UnboundedPinCapacitance" => Some(r.message),
            _ => None,
        };
        assert_eq!(
            rule_1.is_some(),
            extract.is_some(),
            "{part}: validate {rule_1:?}, extract {extract:?}"
        );
        if let (Some(v), Some(x)) = (&rule_1, &extract) {
            let named = format!("{reference} ({part}) on `SPI_SCK`");
            assert!(
                v.contains(&named) && x.contains(&named),
                "{part}: {v} | {x}"
            );
            early.push(part);
        }

        let rule_3 = spec.drawable_parts(&arms).err();
        let emit = Drawing::of(&spec, &solved, &pins).err();
        match (&rule_3, &emit) {
            (None, None) => {}
            (Some(e), Some(EmitError::UnknownPart { reference: r, .. })) => {
                assert_eq!(e.kind(), "PartNotDrawable", "{part}");
                assert_eq!((e.id(), r.as_str()), (Some(reference), reference), "{part}");
                late.push(part);
            }
            _ => panic!("{part}: drawable_parts {rule_3:?}, the emitter {emit:?}"),
        }
    }
    // Neither side is vacuous on today's table.
    assert_eq!(early, ["ICM20948", "ICM42688P"]);
    assert_eq!(late, ["ICP10111"]);
}
