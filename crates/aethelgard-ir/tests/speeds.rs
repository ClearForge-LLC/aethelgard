//! The output speed table (`AET-WO-4300c` §1.1, §1.2): the embedded table's
//! pin and values, one refusal per way the table can lie (§4.3), and the
//! choice at every edge §4.1 names.

use aethelgard_ir::speeds::{
    OUTPUT_SPEEDS_JSON, OutputSpeeds, Refusal, parse_output_speeds, table,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// The sha256 of `data/output-speeds.json`. It moves only with the file.
const SHA256: &str = "13fd5c7e4aca8f4f16868e585884a09904486961c0cb7dac0dafc2c7451ee098";

fn embedded() -> Value {
    serde_json::from_str(OUTPUT_SPEEDS_JSON).unwrap()
}

fn refused(mutate: impl FnOnce(&mut Value), names: &str) {
    let mut doc = embedded();
    mutate(&mut doc);
    let err = parse_output_speeds(&doc.to_string()).expect_err("the mutated table is refused");
    assert_eq!(err.kind(), "OutputSpeedTableInvalid", "{err}");
    assert!(err.to_string().contains(names), "`{names}` not in: {err}");
}

/// The index of the row for a speed, band (by its minimum) and load.
fn at(doc: &Value, speed: &str, vdd_min: u32, load: u32) -> usize {
    doc["rows"]
        .as_array()
        .unwrap()
        .iter()
        .position(|r| {
            r["ospeedr"] == speed && r["vdd_min_millivolts"] == vdd_min && r["load_picofarads"] == load
        })
        .unwrap()
}

#[test]
fn the_embedded_table_is_pinned_by_its_sha256() {
    let hash: String = Sha256::digest(OUTPUT_SPEEDS_JSON.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(hash, SHA256);
}

/// Every cell of DS12117 Rev 10 Table 149 (pp. 245-246), as printed: Fmax in
/// MHz and tr/tf in ns, per speed, at 50, 30 and 10 pF, 2.7-3.6 V then
/// 1.62-2.7 V. Table 52 (rev Y) prints the same values.
#[test]
fn the_table_holds_every_cell_as_printed() {
    let printed: [(u8, [(f64, f64); 6]); 4] = [
        (0, [(12.0, 16.6), (3.0, 33.3), (12.0, 13.3), (3.0, 25.0), (16.0, 10.0), (4.0, 20.0)]),
        (1, [(60.0, 5.2), (15.0, 10.0), (80.0, 4.2), (15.0, 7.5), (110.0, 2.8), (20.0, 5.2)]),
        (2, [(85.0, 3.8), (35.0, 6.9), (110.0, 2.8), (40.0, 5.2), (166.0, 1.8), (85.0, 3.3)]),
        (3, [(100.0, 3.3), (50.0, 6.6), (133.0, 2.4), (66.0, 4.5), (220.0, 1.5), (100.0, 2.7)]),
    ];
    let t = table().unwrap();
    assert_eq!(t.rows().len(), 24);
    assert_eq!(t.mcu, "STM32H753ZITx");
    for (speed, cells) in printed {
        for (k, (fmax, tr)) in cells.iter().enumerate() {
            let load = [50, 50, 30, 30, 10, 10][k];
            let band = if k % 2 == 0 { (2700, 3600) } else { (1620, 2700) };
            let r = t.row(speed, band, load).unwrap();
            assert_eq!(f64::from(r.fmax_hz), fmax * 1e6, "{speed} {band:?} {load}");
            assert_eq!(f64::from(r.rise_fall_picoseconds), (tr * 1000.0_f64).round());
            assert_eq!(r.requires_compensation_cell, speed >= 2);
        }
    }
    assert_eq!(t.vbat_pins, ["PC13", "PC14", "PC15", "PI8"]);
    assert_eq!(t.vbat_fmax_hz, 2_000_000);
    assert!(t.open_drain.contains("6.3.32"), "{}", t.open_drain);
}

#[test]
fn a_table_that_lies_is_refused_by_name() {
    refused(|d| d["schema_version"] = 2.into(), "schema_version");
    refused(|d| d["extra"] = 1.into(), "extra");
    refused(
        |d| {
            d["citations"].as_object_mut().unwrap().remove("rev_y");
        },
        "rev_y",
    );
    refused(
        |d| {
            d["citations"]["rev_v"].as_object_mut().unwrap().remove("sha256");
        },
        "citations.rev_v",
    );
    refused(
        |d| {
            let i = at(d, "10", 2700, 30);
            d["rows"][i]["requires_compensation_cell"] = false.into();
        },
        "note 5",
    );
    refused(
        |d| {
            let i = at(d, "01", 2700, 30);
            d["rows"][i]["requires_compensation_cell"] = true.into();
        },
        "note 5",
    );
    refused(
        |d| {
            let i = at(d, "01", 2700, 30);
            d["rows"][i]["ospeedr"] = "1".into();
        },
        "`ospeedr`",
    );
    refused(
        |d| {
            let i = at(d, "00", 2700, 30);
            d["rows"][i]["load_picofarads"] = 20.into();
        },
        "load_picofarads",
    );
    refused(
        |d| {
            let i = at(d, "00", 2700, 30);
            d["rows"][i]["vdd_min_millivolts"] = 2500.into();
        },
        "band",
    );
    refused(
        |d| {
            let i = at(d, "00", 2700, 30);
            d["rows"][i]["fmax_hz"] = 0.into();
        },
        "fmax_hz",
    );
    refused(
        |d| {
            let i = at(d, "00", 2700, 30);
            d["rows"].as_array_mut().unwrap().remove(i);
        },
        "no row for speed 00",
    );
    refused(
        |d| {
            let copy = d["rows"][0].clone();
            d["rows"].as_array_mut().unwrap().push(copy);
        },
        "twice",
    );
    // 50 pF given more than 30 pF at `01`: a transcription the shape passes.
    refused(
        |d| {
            let i = at(d, "01", 2700, 50);
            d["rows"][i]["fmax_hz"] = 90_000_000.into();
        },
        "higher Fmax",
    );
    refused(|d| d["vbat_pins"]["pins"] = json!([]), "vbat_pins.pins");
    refused(|d| d["open_drain"] = "".into(), "open_drain");
    for text in ["", "{", "[]", "null"] {
        assert_eq!(
            parse_output_speeds(text).unwrap_err().kind(),
            "OutputSpeedTableInvalid"
        );
    }
}

/// The reference: 20 MHz, VDD 3300 mV less 5 % (3135), 30 pF declared.
#[test]
fn the_reference_spi_line_takes_01_at_the_30_pf_row() {
    let c = table().unwrap().choose("PA5", 20_000_000, 3135, 30_000, 20).unwrap();
    assert_eq!((c.ospeedr, c.row.load_picofarads, c.fmax_hz), (1, 30, 80_000_000));
    // At 7 MHz (the ICM-20948 fixture) `00` suffices: 12 MHz.
    let c = table().unwrap().choose("PA5", 7_000_000, 3135, 30_000, 20).unwrap();
    assert_eq!((c.ospeedr, c.fmax_hz), (0, 12_000_000));
}

/// §4.1: a capacitance is rounded up, never down.
#[test]
fn a_capacitance_rounds_up_to_the_next_row() {
    assert_eq!(OutputSpeeds::load(0), Ok(10));
    assert_eq!(OutputSpeeds::load(10_000), Ok(10));
    assert_eq!(OutputSpeeds::load(10_001), Ok(30));
    assert_eq!(OutputSpeeds::load(30_001), Ok(50));
    assert_eq!(OutputSpeeds::load(50_000), Ok(50));
    assert_eq!(
        OutputSpeeds::load(50_001),
        Err(Refusal::LoadAboveTable {
            capacitance_femtofarads: 50_001
        })
    );
    // 16 MHz passes `00` at 10 pF but not at 10.001 pF (the 30 pF row).
    let t = table().unwrap();
    assert_eq!(t.choose("PA5", 16_000_000, 3135, 10_000, 20).unwrap().ospeedr, 0);
    assert_eq!(t.choose("PA5", 16_000_000, 3135, 10_001, 20).unwrap().ospeedr, 1);
}

/// §4.1: the band is the supply's minimum's, and 2.7 V exactly is the
/// lower band, which the table also prints it in.
#[test]
fn the_band_is_the_minimum_s_and_2_7_v_takes_the_lower() {
    assert_eq!(OutputSpeeds::band(2701), Ok((2700, 3600)));
    assert_eq!(OutputSpeeds::band(2700), Ok((1620, 2700)));
    assert_eq!(OutputSpeeds::band(1620), Ok((1620, 2700)));
    for v in [1619, 3601, 0] {
        assert_eq!(
            OutputSpeeds::band(v),
            Err(Refusal::SupplyOutsideTable {
                vdd_min_millivolts: v
            })
        );
    }
    // 20 MHz at 30 pF: `01` gives 80 MHz above 2.7 V, 15 MHz at or below.
    let t = table().unwrap();
    assert_eq!(t.choose("PA5", 20_000_000, 2701, 30_000, 20).unwrap().ospeedr, 1);
    assert_eq!(
        t.choose("PA5", 20_000_000, 2700, 30_000, 20),
        Err(Refusal::TooFastForPin {
            rate_hz: 20_000_000,
            fmax_hz: 15_000_000
        })
    );
}

/// §1.4: a line whose rate needs `10` or `11` is refused, never sped up.
#[test]
fn a_rate_only_10_or_11_meets_is_refused() {
    let t = table().unwrap();
    assert_eq!(t.choose("PA5", 80_000_000, 3135, 30_000, 20).unwrap().ospeedr, 1);
    assert_eq!(
        t.choose("PA5", 80_000_001, 3135, 30_000, 20),
        Err(Refusal::TooFastForPin {
            rate_hz: 80_000_001,
            fmax_hz: 80_000_000
        })
    );
}

/// Note 2: a VBAT-capable pin is held to 2 MHz whatever its row says.
#[test]
fn a_vbat_pin_is_held_to_2_mhz() {
    let t = table().unwrap();
    assert_eq!(t.choose("PC13", 2_000_000, 3135, 10_000, 20).unwrap().fmax_hz, 2_000_000);
    assert_eq!(
        t.choose("PC13", 2_000_001, 3135, 10_000, 20),
        Err(Refusal::TooFastForPin {
            rate_hz: 2_000_001,
            fmax_hz: 2_000_000
        })
    );
}

/// `AET-WO-4300c` F-b: the declared rise time is a requirement on the edge.
/// The slowest speed meeting both it and the rate is chosen.
#[test]
fn the_declared_rise_time_is_met_or_refused() {
    let t = table().unwrap();
    // 1 MHz at 30 pF: `00`'s 13.3 ns meets 14 ns but not 13.
    assert_eq!(t.choose("PA5", 1_000_000, 3135, 30_000, 14).unwrap().ospeedr, 0);
    assert_eq!(t.choose("PA5", 1_000_000, 3135, 30_000, 13).unwrap().ospeedr, 1);
    // `01`'s 4.2 ns does not meet 4 ns.
    assert_eq!(
        t.choose("PA5", 1_000_000, 3135, 30_000, 4),
        Err(Refusal::RiseFasterThanPin {
            rise_time_ns: 4,
            rise_fall_picoseconds: 4_200
        })
    );
}
