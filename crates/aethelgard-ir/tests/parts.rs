//! The parts table (`AET-WO-4200` D3): the embedded table's values, its pin,
//! and one refusal per malformed field, each through `parse_parts`.

use aethelgard_ir::parts::{
    Bus, Held, MAX_PIN_FEMTOFARADS, MAX_SPI_CLOCK_HZ, MAX_SUPPLY_MILLIVOLTS, PARTS_JSON,
    parse_parts, table,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// The sha256 of `data/parts.json`. It moves only with the file, on purpose.
const PARTS_SHA256: &str = "744c769edb8bd1dd916503b4cd97cacefd7d42f4e08e165ebb5201e7fe5ca007";

fn embedded() -> Value {
    serde_json::from_str(PARTS_JSON).unwrap()
}

/// The index of `name` in `parts`.
fn at(name: &str) -> String {
    let i = embedded()["parts"]
        .as_array()
        .unwrap()
        .iter()
        .position(|p| p["part"] == name)
        .unwrap();
    format!("/parts/{i}")
}

/// Parses the embedded table after `mutate` edits the entry for `name`.
fn refused_entry(name: &str, mutate: impl FnOnce(&mut Value)) -> aethelgard_ir::IrValidationError {
    let mut doc = embedded();
    mutate(doc.pointer_mut(&at(name)).unwrap());
    parse_parts(&doc.to_string()).expect_err("the mutated table must be refused")
}

/// Asserts kind, the part `id()` names, and that the message names `field`.
fn assert_refused(err: &aethelgard_ir::IrValidationError, part: Option<&str>, field: &str) {
    assert_eq!(err.kind(), "PartsTableInvalid", "{err}");
    assert_eq!(err.id(), part, "{err}");
    assert!(err.to_string().contains(field), "`{field}` not in: {err}");
}

#[test]
fn the_embedded_table_is_pinned_by_its_sha256() {
    let hash: String = Sha256::digest(PARTS_JSON.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(hash, PARTS_SHA256);
}

#[test]
fn the_embedded_table_parses_and_holds_five_parts() {
    let t = table().unwrap();
    let names: Vec<&str> = t.parts().iter().map(|p| p.part.as_str()).collect();
    assert_eq!(
        names,
        ["BME280", "ICM20948", "ICP10111", "TMP117", "W25Q64JVxIQ"]
    );
    assert_eq!(parse_parts(PARTS_JSON).unwrap(), *t);
}

#[test]
fn only_the_flashs_lifecycle_is_pending() {
    // Every datasheet is held in ~/.cache/aet-4200/ds/ or ~/.cache/aet-4210/ds/,
    // recorded in the work orders' FEEDBACK. Winbond's catalog loads the
    // flash's status by script, so its lifecycle is not held (`AET-WO-4210`).
    assert_eq!(
        table().unwrap().pending(),
        [(
            "W25Q64JVxIQ",
            "lifecycle",
            "https://www.winbond.com/hq/new-online-purchasing-guide/?__locale=en&pLine=/product/code-storage-flash/qspi-nor/&pNo=W25Q64JV"
        )]
    );
}

#[test]
fn every_entry_has_all_four_citations_held() {
    for p in table().unwrap().parts() {
        for c in [
            &p.citations.bus,
            &p.citations.identity,
            &p.citations.supply,
            &p.citations.pin_capacitance,
        ] {
            assert!(!c.document.is_empty() && !c.revision.is_empty() && !c.section.is_empty());
            assert!(
                matches!(&c.held, Held::Sha256(h) if h.len() == 64),
                "{}",
                p.part
            );
        }
    }
}

#[test]
fn each_part_holds_its_datasheet_values() {
    let t = table().unwrap();
    let get = |n: &str| t.get(n).unwrap();

    let p = get("BME280");
    assert_eq!(
        (
            p.identity.write.as_slice(),
            p.identity.width,
            p.identity.value
        ),
        (&[0xD0][..], 1, 0x60)
    );
    assert_eq!(
        p.bus,
        Bus::I2c {
            addresses: vec![0x76, 0x77]
        }
    );
    assert_eq!(p.supply_millivolts, (1710, 3600));
    assert_eq!(p.renode.as_deref(), Some("I2C.BME280"));

    let p = get("TMP117");
    assert_eq!(
        (
            p.identity.write.as_slice(),
            p.identity.width,
            p.identity.value
        ),
        (&[0x0F][..], 2, 0x0117)
    );
    assert_eq!(
        p.bus,
        Bus::I2c {
            addresses: vec![0x48, 0x49, 0x4A, 0x4B]
        }
    );
    assert_eq!(p.supply_millivolts, (1800, 5500));
    assert_eq!(p.renode, None);

    let p = get("W25Q64JVxIQ");
    assert_eq!(
        (
            p.identity.write.as_slice(),
            p.identity.width,
            p.identity.value
        ),
        (&[0x9F][..], 3, 0xEF4017)
    );
    assert_eq!(
        p.bus,
        Bus::Spi {
            modes: vec![0, 3],
            max_clock_hz: 104_000_000
        }
    );
    assert_eq!(p.supply_millivolts, (2700, 3600));
    assert_eq!(p.renode.as_deref(), Some("SPI.GenericSpiFlash"));
    // `AET-WO-4300c` D3: Cout 8 pF, the larger of the two printed maxima.
    assert_eq!(p.pin_capacitance_femtofarads, Some(8_000));

    let p = get("ICM20948");
    assert_eq!(
        (
            p.identity.write.as_slice(),
            p.identity.width,
            p.identity.value
        ),
        (&[0x80][..], 1, 0xEA)
    );
    assert_eq!(
        p.bus,
        Bus::Spi {
            modes: vec![0, 3],
            max_clock_hz: 7_000_000
        }
    );
    assert_eq!(p.supply_millivolts, (1710, 3600));
    assert_eq!(p.renode.as_deref(), Some("Sensors.ICM20948"));
    // "< 10" pF is printed as a typical: no maximum (`AET-WO-4300c` D3).
    assert_eq!(p.pin_capacitance_femtofarads, None);
}

// ---- file level -------------------------------------------------------------

#[test]
fn refuses_a_duplicate_part() {
    let mut doc = embedded();
    let first = doc["parts"][0].clone();
    doc["parts"].as_array_mut().unwrap().push(first);
    let err = parse_parts(&doc.to_string()).unwrap_err();
    assert_refused(&err, Some("BME280"), "twice");
}

#[test]
fn refuses_an_unknown_key_at_top_level() {
    let mut doc = embedded();
    doc["extra"] = 1.into();
    let err = parse_parts(&doc.to_string()).unwrap_err();
    assert_refused(&err, None, "extra");
}

#[test]
fn refuses_schema_version_2_3_and_5() {
    for version in [2, 3, 5] {
        let mut doc = embedded();
        doc["schema_version"] = version.into();
        let err = parse_parts(&doc.to_string()).unwrap_err();
        assert_refused(&err, None, "schema_version");
    }
}

#[test]
fn refuses_text_that_is_not_json_without_panicking() {
    for text in ["", "{", "[]", "null", "{\"schema_version\":1}"] {
        let err = parse_parts(text).unwrap_err();
        assert_refused(&err, None, "");
    }
}

#[test]
fn refuses_a_non_identifier_part_name() {
    let err = refused_entry("BME280", |p| p["part"] = "BME 280".into());
    assert_refused(&err, Some("BME 280"), "part");
}

// ---- unknown keys in each level of an entry -----------------------------------

#[test]
fn refuses_an_unknown_key_in_an_entry() {
    let err = refused_entry("BME280", |p| p["extra"] = 1.into());
    assert_refused(&err, Some("BME280"), "extra");
}

#[test]
fn refuses_an_unknown_key_in_bus() {
    let err = refused_entry("BME280", |p| p["bus"]["i2c"]["extra"] = 1.into());
    assert_refused(&err, Some("BME280"), "extra");
    let err = refused_entry("W25Q64JVxIQ", |p| p["bus"]["spi"]["extra"] = 1.into());
    assert_refused(&err, Some("W25Q64JVxIQ"), "extra");
    let err = refused_entry("BME280", |p| p["bus"]["extra"] = json!({}));
    assert_refused(&err, Some("BME280"), "");
}

#[test]
fn refuses_an_unknown_key_in_identity() {
    let err = refused_entry("BME280", |p| p["identity"]["extra"] = 1.into());
    assert_refused(&err, Some("BME280"), "extra");
}

#[test]
fn refuses_an_unknown_key_in_a_citation() {
    let err = refused_entry("BME280", |p| p["citations"]["bus"]["extra"] = 1.into());
    assert_refused(&err, Some("BME280"), "extra");
}

// ---- bus ----------------------------------------------------------------------

#[test]
fn refuses_a_bus_with_both_i2c_and_spi_or_neither() {
    // Refused before serde reads the entry, naming `bus`.
    let err = refused_entry("BME280", |p| {
        p["bus"]["spi"] = json!({"modes": [0], "max_clock_hz": 1_000_000});
    });
    assert_refused(&err, Some("BME280"), "`bus`");
    let err = refused_entry("BME280", |p| p["bus"] = json!({}));
    assert_refused(&err, Some("BME280"), "`bus`");
}

#[test]
fn refuses_an_i2c_address_out_of_range_repeated_or_missing() {
    for bad in ["0x07", "0x00", "0x78", "0xFF", "0x1FF"] {
        let err = refused_entry("BME280", |p| p["bus"]["i2c"]["addresses"][0] = bad.into());
        assert_refused(&err, Some("BME280"), "bus.i2c.addresses");
    }
    let err = refused_entry("BME280", |p| {
        p["bus"]["i2c"]["addresses"][1] = "0x76".into()
    });
    assert_refused(&err, Some("BME280"), "bus.i2c.addresses");
    assert!(err.to_string().contains("twice"), "{err}");
    let err = refused_entry("BME280", |p| p["bus"]["i2c"]["addresses"] = json!([]));
    assert_refused(&err, Some("BME280"), "bus.i2c.addresses");
}

#[test]
fn refuses_a_malformed_i2c_address() {
    for bad in ["0x7a", "76", "", "0x", "0x076x"] {
        let err = refused_entry("BME280", |p| p["bus"]["i2c"]["addresses"][0] = bad.into());
        assert_refused(&err, Some("BME280"), "bus.i2c.addresses");
    }
}

#[test]
fn refuses_an_spi_mode_out_of_range_repeated_or_missing() {
    let err = refused_entry("W25Q64JVxIQ", |p| p["bus"]["spi"]["modes"] = json!([0, 4]));
    assert_refused(&err, Some("W25Q64JVxIQ"), "bus.spi.modes");
    let err = refused_entry("W25Q64JVxIQ", |p| p["bus"]["spi"]["modes"] = json!([3, 3]));
    assert_refused(&err, Some("W25Q64JVxIQ"), "bus.spi.modes");
    let err = refused_entry("W25Q64JVxIQ", |p| p["bus"]["spi"]["modes"] = json!([]));
    assert_refused(&err, Some("W25Q64JVxIQ"), "bus.spi.modes");
}

#[test]
fn refuses_a_max_clock_of_zero_or_above_the_ceiling() {
    for hz in [0, u64::from(MAX_SPI_CLOCK_HZ) + 1] {
        let err = refused_entry("W25Q64JVxIQ", |p| {
            p["bus"]["spi"]["max_clock_hz"] = hz.into()
        });
        assert_refused(&err, Some("W25Q64JVxIQ"), "bus.spi.max_clock_hz");
    }
    // The ceiling itself is accepted.
    let mut doc = embedded();
    doc.pointer_mut(&at("W25Q64JVxIQ")).unwrap()["bus"]["spi"]["max_clock_hz"] =
        MAX_SPI_CLOCK_HZ.into();
    parse_parts(&doc.to_string()).unwrap();
}

// ---- identity -----------------------------------------------------------------

#[test]
fn refuses_an_identity_read_of_zero_or_five() {
    for read in [0, 5] {
        let err = refused_entry("BME280", |p| p["identity"]["read"] = read.into());
        assert_refused(&err, Some("BME280"), "identity.read");
    }
}

#[test]
fn refuses_an_identity_value_wider_than_its_read() {
    let err = refused_entry("BME280", |p| p["identity"]["value"] = "0x100".into());
    assert_refused(&err, Some("BME280"), "identity.value");
    let err = refused_entry("TMP117", |p| p["identity"]["value"] = "0x10117".into());
    assert_refused(&err, Some("TMP117"), "identity.value");
}

#[test]
fn refuses_an_identity_write_above_ff_or_malformed() {
    for bad in ["0x100", "0xd0", "D0", "", "0x", "0xDG"] {
        let err = refused_entry("BME280", |p| p["identity"]["write"] = json!([bad]));
        assert_refused(&err, Some("BME280"), "identity.write");
    }
    for bad in [json!([]), json!(["0x1", "0x2", "0x3", "0x4", "0x5"])] {
        let err = refused_entry("BME280", |p| p["identity"]["write"] = bad.clone());
        assert_refused(&err, Some("BME280"), "identity.write");
    }
    let err = refused_entry("BME280", |p| p["identity"]["write"] = "0xD0".into());
    assert_refused(&err, Some("BME280"), "invalid type");
}

// ---- supply -------------------------------------------------------------------

#[test]
fn refuses_a_supply_with_zero_min_min_above_max_or_max_above_the_ceiling() {
    for (min, max) in [
        (0, 3600),
        (3601, 3600),
        (1710, u64::from(MAX_SUPPLY_MILLIVOLTS) + 1),
    ] {
        let err = refused_entry("BME280", |p| {
            p["supply_millivolts"] = json!({"min": min, "max": max});
        });
        assert_refused(&err, Some("BME280"), "supply_millivolts");
    }
    let mut doc = embedded();
    doc.pointer_mut(&at("BME280")).unwrap()["supply_millivolts"] =
        json!({"min": 1, "max": MAX_SUPPLY_MILLIVOLTS});
    parse_parts(&doc.to_string()).unwrap();
}

// ---- renode model ---------------------------------------------------------------

#[test]
fn refuses_an_unknown_renode_model() {
    let err = refused_entry("BME280", |p| p["renode"] = "I2C.NOPE".into());
    assert_refused(&err, Some("BME280"), "renode");
    assert!(err.to_string().contains("I2C.NOPE"), "{err}");
}

#[test]
fn refuses_a_known_renode_model_on_the_wrong_bus_kind() {
    let err = refused_entry("BME280", |p| p["renode"] = "SPI.GenericSpiFlash".into());
    assert_refused(&err, Some("BME280"), "renode");
    let err = refused_entry("W25Q64JVxIQ", |p| p["renode"] = "I2C.BME280".into());
    assert_refused(&err, Some("W25Q64JVxIQ"), "renode");
    // `Sensors.ICM20948` attaches to either kind.
    let mut doc = embedded();
    doc.pointer_mut(&at("BME280")).unwrap()["renode"] = "Sensors.ICM20948".into();
    parse_parts(&doc.to_string()).unwrap();
}

// ---- citations ------------------------------------------------------------------

#[test]
fn refuses_a_citation_with_both_sha256_and_pending_or_neither() {
    let err = refused_entry("BME280", |p| {
        p["citations"]["bus"]["pending"] = "https://example.com/ds.pdf".into();
    });
    assert_refused(&err, Some("BME280"), "citations.bus");
    let err = refused_entry("BME280", |p| {
        p["citations"]["supply"]
            .as_object_mut()
            .unwrap()
            .remove("sha256");
    });
    assert_refused(&err, Some("BME280"), "citations.supply");
}

#[test]
fn refuses_a_bad_citation_sha256() {
    let good = "a2ccdb449fec94380742fe8eec851a11d9bd4142252d332b34682b4deecd7d89";
    for bad in [&good[..63], &good.to_uppercase()[..], ""] {
        let err = refused_entry("BME280", |p| {
            p["citations"]["identity"]["sha256"] = bad.into()
        });
        assert_refused(&err, Some("BME280"), "citations.identity.sha256");
    }
}

#[test]
fn a_pending_citation_must_be_an_https_url_and_is_listed() {
    for bad in [
        "http://example.com/ds.pdf",
        "ftp://example.com",
        "example.com",
        "https://",
        "",
    ] {
        let err = refused_entry("TMP117", |p| {
            let c = p["citations"]["bus"].as_object_mut().unwrap();
            c.remove("sha256");
            c.insert("pending".into(), bad.into());
        });
        assert_refused(&err, Some("TMP117"), "citations.bus.pending");
    }
    let mut doc = embedded();
    let c = doc.pointer_mut(&at("TMP117")).unwrap()["citations"]["bus"]
        .as_object_mut()
        .unwrap();
    c.remove("sha256");
    c.insert("pending".into(), "https://example.com/ds.pdf".into());
    let t = parse_parts(&doc.to_string()).unwrap();
    assert_eq!(
        t.pending(),
        [
            ("TMP117", "bus", "https://example.com/ds.pdf"),
            (
                "W25Q64JVxIQ",
                "lifecycle",
                "https://www.winbond.com/hq/new-online-purchasing-guide/?__locale=en&pLine=/product/code-storage-flash/qspi-nor/&pNo=W25Q64JV"
            )
        ]
    );
}

/// `AET-WO-4200` adversarial A2: a key given twice inside an entry is
/// refused, never collapsed to the last one; so is an entry without
/// `renode` (only `null` says unmodelled), a part twice in another case,
/// and an empty table.
#[test]
fn a_key_twice_a_missing_renode_a_case_twin_or_no_parts_is_refused() {
    let text = PARTS_JSON;
    // `"part": "W25Q64JVxIQ"` given twice, the second renaming it.
    let twice = text.replacen(
        "\"part\": \"W25Q64JVxIQ\",",
        "\"part\": \"W25Q64JVxIQ\",\n      \"part\": \"W25Q128JV\",",
        1,
    );
    assert_ne!(twice, text);
    let err = parse_parts(&twice).unwrap_err();
    // Named by the entry's last `part`, as a lenient read gives it: the
    // refusal still says which entry, and that a key is twice.
    assert_refused(&err, Some("W25Q128JV"), "duplicate field `part`");
    // `"value"` given twice in an identity.
    let twice = text.replacen(
        "\"value\": \"0xEF4017\"",
        "\"value\": \"0xEF4017\",\n        \"value\": \"0xEF4018\"",
        1,
    );
    assert_ne!(twice, text);
    let err = parse_parts(&twice).unwrap_err();
    assert_refused(&err, Some("W25Q64JVxIQ"), "duplicate field `value`");
    // `renode` left out of an entry.
    let mut doc = embedded();
    doc.pointer_mut(&at("W25Q64JVxIQ"))
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("renode");
    let err = parse_parts(&doc.to_string()).unwrap_err();
    assert_refused(&err, Some("W25Q64JVxIQ"), "missing field `renode`");
    // The same part in another case.
    let mut doc = embedded();
    let mut copy = doc.pointer(&at("BME280")).unwrap().clone();
    copy["part"] = "bme280".into();
    doc["parts"].as_array_mut().unwrap().push(copy);
    let err = parse_parts(&doc.to_string()).unwrap_err();
    assert_refused(&err, Some("bme280"), "twice");
    // No parts at all.
    let err = parse_parts(r#"{"schema_version": 4, "parts": []}"#).unwrap_err();
    assert_refused(&err, None, "`parts` is empty");
}

/// `AET-WO-4300c` D3: `pin_capacitance` is required, its key too, and a
/// stated maximum is 1 fF to the ceiling; `null` is "no maximum printed".
#[test]
fn refuses_a_missing_or_out_of_range_pin_capacitance() {
    let err = refused_entry("BME280", |p| {
        p.as_object_mut().unwrap().remove("pin_capacitance");
    });
    assert_refused(&err, Some("BME280"), "pin_capacitance");
    let err = refused_entry("BME280", |p| p["pin_capacitance"] = json!({}));
    assert_refused(&err, Some("BME280"), "max_femtofarads");
    for bad in [0, MAX_PIN_FEMTOFARADS + 1] {
        let err = refused_entry("W25Q64JVxIQ", |p| {
            p["pin_capacitance"]["max_femtofarads"] = bad.into();
        });
        assert_refused(&err, Some("W25Q64JVxIQ"), "pin_capacitance.max_femtofarads");
    }
    let err = refused_entry("W25Q64JVxIQ", |p| {
        p["citations"]
            .as_object_mut()
            .unwrap()
            .remove("pin_capacitance");
    });
    assert_refused(&err, Some("W25Q64JVxIQ"), "pin_capacitance");
}

/// `AET-WO-4300c` F20: a `SPI.GenericSpiFlash` part's identity is a JEDEC
/// read (`9Fh`, 3 bytes) whose capacity code the model can build, or the
/// table is refused before it reaches `generate`.
#[test]
fn refuses_a_malformed_flash_identity() {
    for (field, bad) in [
        ("write", json!(["0x9E"])),
        ("read", json!(2)),
        ("value", json!("0xEF400F")),
        ("value", json!("0xEF401A")),
    ] {
        let err = refused_entry("W25Q64JVxIQ", |p| {
            p["identity"][field] = bad.clone();
            if field == "read" {
                p["identity"]["value"] = "0x4017".into();
            }
        });
        assert_refused(&err, Some("W25Q64JVxIQ"), "JEDEC identity");
    }
    // The edges of the capacity codes are accepted.
    for value in ["0xEF4010", "0xEF4019"] {
        let mut doc = embedded();
        doc.pointer_mut(&at("W25Q64JVxIQ")).unwrap()["identity"]["value"] = value.into();
        assert!(parse_parts(&doc.to_string()).is_ok(), "{value}");
    }
}

// ---- schema 3 (`AET-WO-4210`) -------------------------------------------------

#[test]
fn the_stand_ins_hold_their_scripts_and_lifecycles() {
    use aethelgard_ir::parts::{Lifecycle, StepKind};
    let t = table().unwrap();
    let life: Vec<(&str, Lifecycle, bool)> = t
        .parts()
        .iter()
        .map(|p| (p.part.as_str(), p.lifecycle, p.stand_in))
        .collect();
    assert_eq!(
        life,
        [
            ("BME280", Lifecycle::Active, false),
            ("ICM20948", Lifecycle::Eol, true),
            ("ICP10111", Lifecycle::Eol, true),
            ("TMP117", Lifecycle::Active, false),
            ("W25Q64JVxIQ", Lifecycle::Active, false),
        ]
    );
    let imu = t.get("ICM20948").unwrap();
    assert_eq!(imu.io_millivolts, Some((1710, 1950)));
    let steps = imu.steps();
    let shape: Vec<(StepKind, Vec<u8>, u8, u64, u64)> = steps
        .iter()
        .map(|s| (s.kind, s.write.clone(), s.width, s.expected, s.mask))
        .collect();
    // D3: 0 g, 0 g and 1 g at 16,384 LSB/g; F5: the wake is the configuration.
    assert_eq!(
        shape,
        [
            (StepKind::Identity, vec![0x80], 1, 0xEA, 0xFF),
            (StepKind::Configure, vec![0x06, 0x01], 0, 0, 0),
            (StepKind::Data, vec![0xAD], 6, 0x4000, 0xFFFF_FFFF_FFFF),
        ]
    );
    let baro = t.get("ICP10111").unwrap();
    assert_eq!(baro.io_range().0, (1710, 1890));
    assert!(!baro.identity.datasheet_value);
    let shape: Vec<(StepKind, Vec<u8>, u8, u64, u64)> = baro
        .steps()
        .iter()
        .map(|s| (s.kind, s.write.clone(), s.width, s.expected, s.mask))
        .collect();
    // F6: bits 5:0 only. 25 degrees C: (25 + 45) x 65536 / 175 = 26214.4.
    assert_eq!(
        shape,
        [
            (StepKind::Identity, vec![0xEF, 0xC8], 2, 0x0008, 0x3F),
            (StepKind::Configure, vec![0x80, 0x5D], 0, 0, 0),
            (StepKind::Data, vec![0x78, 0x66], 2, 26_214, 0xFFFF),
        ]
    );
    // A part with no script makes its identity read alone.
    assert_eq!(t.get("BME280").unwrap().steps().len(), 1);
}

#[test]
fn a_channel_rounds_half_away_from_zero_and_wraps_signed() {
    use aethelgard_ir::parts::{Channel, Quantity};
    let c = |stimulus, zero, lsb, signed| Channel {
        quantity: Quantity::AccelerationX,
        stimulus,
        zero,
        lsb,
        signed,
    };
    assert_eq!(c(1, 0, (3, 2), false).raw(), 2);
    assert_eq!(c(-1, 0, (3, 2), true).raw(), 0xFFFE);
    assert_eq!(c(-1, 0, (16384, 1), true).raw(), 0xC000);
    assert_eq!(c(1, 0, (1, 4), false).raw(), 0);
}

#[test]
fn refuses_a_lifecycle_outside_the_three() {
    for bad in ["Active", "obsolete", ""] {
        let err = refused_entry("TMP117", |p| p["lifecycle"] = bad.into());
        assert_refused(&err, Some("TMP117"), "lifecycle");
    }
    let err = refused_entry("TMP117", |p| {
        p.as_object_mut().unwrap().remove("lifecycle");
    });
    assert_refused(&err, Some("TMP117"), "lifecycle");
    let err = refused_entry("TMP117", |p| {
        p["citations"].as_object_mut().unwrap().remove("lifecycle");
    });
    assert_refused(&err, Some("TMP117"), "lifecycle");
}

#[test]
fn only_a_stand_in_may_carry_an_identity_no_datasheet_states() {
    let err = refused_entry("ICP10111", |p| p["stand_in"] = false.into());
    assert_refused(&err, Some("ICP10111"), "identity.datasheet_value");
    let err = refused_entry("BME280", |p| {
        p["identity"]["datasheet_value"] = false.into()
    });
    assert_refused(&err, Some("BME280"), "identity.datasheet_value");
}

#[test]
fn refuses_a_mask_that_is_zero_too_wide_or_leaves_the_value_unfixed() {
    for bad in ["0x0", "0x10000", "0x37"] {
        let err = refused_entry("ICP10111", |p| p["identity"]["mask"] = bad.into());
        assert_refused(&err, Some("ICP10111"), "identity.mask");
    }
    let err = refused_entry("ICP10111", |p| {
        p["identity"].as_object_mut().unwrap().remove("mask");
    });
    assert_refused(&err, Some("ICP10111"), "mask");
}

#[test]
fn an_io_range_and_its_citation_come_together() {
    let err = refused_entry("ICM20948", |p| p["io_millivolts"] = Value::Null);
    assert_refused(&err, Some("ICM20948"), "citations.io");
    let err = refused_entry("TMP117", |p| {
        p["io_millivolts"] = json!({"min": 1800, "max": 3600});
    });
    assert_refused(&err, Some("TMP117"), "citations.io");
    let err = refused_entry("ICM20948", |p| {
        p["io_millivolts"] = json!({"min": 1950, "max": 1710});
    });
    assert_refused(&err, Some("ICM20948"), "io_millivolts");
    let err = refused_entry("TMP117", |p| {
        p.as_object_mut().unwrap().remove("io_millivolts");
    });
    assert_refused(&err, Some("TMP117"), "io_millivolts");
}

/// `AET-WO-4300d2` K6: the output delay and its citation come together, the
/// key is required, and only an SPI part may carry one.
#[test]
fn an_spi_output_delay_and_its_citation_come_together() {
    let t = table().unwrap();
    let imu = t.get("ICM20948").unwrap();
    assert_eq!(imu.spi_output_delay_max_ns, Some(59));
    let c = imu.citations.spi_output_delay.as_ref().unwrap();
    assert!(c.section.contains("tVD.SDO max 59 ns at 20 pF"), "{c:?}");
    assert!(c.section.contains("characterization of 5 parts"), "{c:?}");
    for p in t.parts().iter().filter(|p| p.part != "ICM20948") {
        assert_eq!(p.spi_output_delay_max_ns, None, "{}", p.part);
        assert!(p.citations.spi_output_delay.is_none(), "{}", p.part);
    }
    // A value without its citation, and a citation without its value.
    let err = refused_entry("ICM20948", |p| {
        p["citations"]["spi_output_delay"] = Value::Null
    });
    assert_refused(&err, Some("ICM20948"), "citations.spi_output_delay");
    let err = refused_entry("ICM20948", |p| p["spi_output_delay_max_ns"] = Value::Null);
    assert_refused(&err, Some("ICM20948"), "citations.spi_output_delay");
    // Each key is required: an absent one is refused, never read as none.
    let err = refused_entry("ICM20948", |p| {
        p.as_object_mut().unwrap().remove("spi_output_delay_max_ns");
    });
    assert_refused(&err, Some("ICM20948"), "spi_output_delay_max_ns");
    let err = refused_entry("W25Q64JVxIQ", |p| {
        p["citations"]
            .as_object_mut()
            .unwrap()
            .remove("spi_output_delay");
    });
    assert_refused(&err, Some("W25Q64JVxIQ"), "spi_output_delay");
    // Out of range.
    for bad in [json!(0), json!(1001)] {
        let err = refused_entry("ICM20948", |p| p["spi_output_delay_max_ns"] = bad.clone());
        assert_refused(&err, Some("ICM20948"), "spi_output_delay_max_ns");
    }
    // A negative or fractional one is not a u32 (serde names the value).
    for bad in [json!(-1), json!(1.5)] {
        let err = refused_entry("ICM20948", |p| p["spi_output_delay_max_ns"] = bad.clone());
        assert_refused(&err, Some("ICM20948"), "expected u32");
    }
    // An I2C part may not carry one, with or without a citation.
    let cite = embedded()["parts"][1]["citations"]["spi_output_delay"].clone();
    let err = refused_entry("TMP117", |p| {
        p["spi_output_delay_max_ns"] = json!(59);
        p["citations"]["spi_output_delay"] = cite.clone();
    });
    assert_refused(&err, Some("TMP117"), "on an I2C part");
}

#[test]
fn a_script_and_its_two_citations_come_together() {
    let err = refused_entry("ICM20948", |p| p["script"] = Value::Null);
    assert_refused(&err, Some("ICM20948"), "citations.configure");
    let err = refused_entry("ICM20948", |p| p["citations"]["data"] = Value::Null);
    assert_refused(&err, Some("ICM20948"), "citations.data");
    let err = refused_entry("BME280", |p| {
        p.as_object_mut().unwrap().remove("script");
    });
    assert_refused(&err, Some("BME280"), "script");
}

#[test]
fn refuses_a_malformed_script() {
    type Mutate = fn(&mut Value);
    let cases: [(&str, Mutate); 9] = [
        ("script.configure.write", |p| {
            p["script"]["configure"]["write"] = json!([])
        }),
        ("script.data.write", |p| {
            p["script"]["data"]["write"] = json!(["0x1FF"])
        }),
        ("script.data.channels", |p| {
            p["script"]["data"]["channels"] = json!([])
        }),
        ("script.data.channels", |p| {
            let c = p["script"]["data"]["channels"][0].clone();
            p["script"]["data"]["channels"] =
                json!([c.clone(), c.clone(), c.clone(), c.clone(), c]);
        }),
        ("quantity", |p| {
            p["script"]["data"]["channels"][0]["quantity"] = "pressure".into()
        }),
        ("unit", |p| {
            p["script"]["data"]["channels"][0]["unit"] = "celsius".into()
        }),
        ("lsb", |p| {
            p["script"]["data"]["channels"][0]["lsb"] = json!([0, 1])
        }),
        ("16-bit word", |p| {
            p["script"]["data"]["channels"][2]["stimulus"] = 2.into()
        }),
        ("16-bit word", |p| {
            p["script"]["data"]["channels"][2]["signed"] = false.into();
            p["script"]["data"]["channels"][2]["stimulus"] = (-1).into();
        }),
    ];
    for (field, mutate) in cases {
        let err = refused_entry("ICM20948", mutate);
        assert_refused(&err, Some("ICM20948"), field);
    }
    let err = refused_entry("ICM20948", |p| p["script"]["data"]["extra"] = 1.into());
    assert_refused(&err, Some("ICM20948"), "extra");
}

/// Adversarial A5: a channel whose conversion overflows 64 bits is refused,
/// never a panic; A6: a stand-in must be `eol` or `nrnd`.
#[test]
fn an_overflowing_channel_or_an_active_stand_in_is_refused() {
    let err = refused_entry("ICM20948", |p| {
        let c = &mut p["script"]["data"]["channels"][2];
        c["stimulus"] = i32::MAX.into();
        c["zero"] = i32::MIN.into();
        c["lsb"] = json!([u32::MAX, 1]);
    });
    assert_refused(&err, Some("ICM20948"), "16-bit word");
    let err = refused_entry("ICP10111", |p| p["lifecycle"] = "active".into());
    assert_refused(&err, Some("ICP10111"), "stand_in");
}
