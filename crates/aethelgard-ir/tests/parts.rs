//! The parts table (`AET-WO-4200` D3): the embedded table's values, its pin,
//! and one refusal per malformed field, each through `parse_parts`.

use aethelgard_ir::parts::{
    Bus, Held, MAX_PIN_FEMTOFARADS, MAX_SPI_CLOCK_HZ, MAX_SUPPLY_MILLIVOLTS, PARTS_JSON,
    parse_parts, table,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// The sha256 of `data/parts.json`. It moves only with the file, on purpose.
const PARTS_SHA256: &str = "e3a0da9f407f028db403bbea1982f0311be82952cc7b49b54c3ad13aeec9ca62";

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
fn the_embedded_table_parses_and_holds_seven_parts() {
    let t = table().unwrap();
    let names: Vec<&str> = t.parts().iter().map(|p| p.part.as_str()).collect();
    assert_eq!(
        names,
        [
            "BME280",
            "BMP581",
            "ICM20948",
            "ICM42688P",
            "ICP10111",
            "TMP117",
            "W25Q64JVxIQ"
        ]
    );
    assert_eq!(parse_parts(PARTS_JSON).unwrap(), *t);
}

#[test]
fn no_citation_is_pending_and_the_flash_is_nrnd_by_its_makers_export() {
    use aethelgard_ir::parts::Lifecycle;
    // Every datasheet is held in ~/.cache/aet-4200/ds/ or ~/.cache/aet-4210/ds/,
    // recorded in the work orders' FEEDBACK. The flash's lifecycle, pending
    // since `AET-WO-4210`, is Winbond's own catalog export (`AET-WO-4211`).
    let t = table().unwrap();
    assert_eq!(t.pending(), []);
    let flash = t.get("W25Q64JVxIQ").unwrap();
    assert_eq!(flash.lifecycle, Lifecycle::Nrnd);
    let c = &flash.citations.lifecycle;
    assert!(
        matches!(&c.held, Held::Sha256(h) if h == "f10526aeaa7efbf7da5f358f1d1ac933275dcf1628546f136c9e39445ab42ac4")
    );
    assert!(
        c.document
            .contains("winbond-W25Q64JV-catalog-export-2026-11-16.csv"),
        "{c:?}"
    );
    for word in ["column 15 Status-Industrial = N", "Longevity Y", "DigiKey"] {
        assert!(c.section.contains(word), "{word}: {c:?}");
    }
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
fn refuses_schema_version_2_3_4_5_and_7() {
    for version in [2, 3, 4, 5, 7] {
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
        [("TMP117", "bus", "https://example.com/ds.pdf")]
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
    let err = parse_parts(r#"{"schema_version": 6, "parts": []}"#).unwrap_err();
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
            ("BMP581", Lifecycle::Active, false),
            ("ICM20948", Lifecycle::Eol, true),
            ("ICM42688P", Lifecycle::Active, false),
            ("ICP10111", Lifecycle::Eol, true),
            ("TMP117", Lifecycle::Active, false),
            ("W25Q64JVxIQ", Lifecycle::Nrnd, false),
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
        bytes: 2,
        big_endian: true,
    };
    assert_eq!(c(1, 0, (3, 2), false).raw(), 2);
    assert_eq!(c(-1, 0, (3, 2), true).raw(), 0xFFFE);
    assert_eq!(c(-1, 0, (16384, 1), true).raw(), 0xC000);
    assert_eq!(c(1, 0, (1, 4), false).raw(), 0);
    // `AET-WO-4220`: a 24-bit word wraps in 24 bits, and its bytes cross the
    // wire in the stated order.
    let wide = |stimulus, big_endian| Channel {
        quantity: Quantity::Pressure,
        stimulus,
        zero: 0,
        lsb: (64, 1),
        signed: true,
        bytes: 3,
        big_endian,
    };
    assert_eq!(wide(-1, false).raw(), 0xFF_FFC0);
    assert_eq!(wide(101_325, false).wire(), [0x40, 0xF3, 0x62]);
    assert_eq!(wide(101_325, true).wire(), [0x62, 0xF3, 0x40]);
    assert_eq!(c(1, 0, (16384, 1), true).wire(), [0x40, 0x00]);
}

/// `AET-WO-4220`: the two parts of our own, each step as the firmware makes
/// it and the judge expects it. The ICM-42688-P's temperature at 29 degrees C
/// is (29 - 25) x 132.48 = 529.92, so rounding (530, 0x0212) and truncation
/// (529) differ: the table rounds, as the datasheet's conversion does (the
/// `4210` D-A8 amendment).
#[test]
fn the_two_parts_of_our_own_make_their_cited_scripts() {
    use aethelgard_ir::parts::{Lifecycle, StepKind};
    let t = table().unwrap();
    let shape = |part: &str| -> Vec<(StepKind, Vec<u8>, u8, u64, u32)> {
        t.get(part)
            .unwrap()
            .steps()
            .iter()
            .map(|s| (s.kind, s.write.clone(), s.width, s.expected, s.settle_us))
            .collect()
    };
    assert_eq!(
        shape("ICM42688P"),
        [
            (StepKind::Identity, vec![0xF5], 1, 0x47, 0),
            (StepKind::Configure, vec![0x4C, 0xC0], 0, 0, 0),
            (StepKind::Configure, vec![0x50, 0x66], 0, 0, 0),
            (StepKind::Configure, vec![0x4E, 0x03], 0, 0, 20_000),
            (StepKind::Data, vec![0x9D], 6, 0x1202_0000_0040, 0),
        ]
    );
    assert_eq!(
        shape("BMP581"),
        [
            (StepKind::Identity, vec![0x01], 1, 0x50, 0),
            (StepKind::Configure, vec![0x36, 0x40, 0x81], 0, 0, 10_000),
            (StepKind::Data, vec![0x1D], 6, 0x0000_1940_F362, 0),
        ]
    );
    // Each of several configurations has its own name, a lone one `configure`.
    let labels = |part: &str| -> Vec<&str> {
        t.get(part)
            .unwrap()
            .steps()
            .iter()
            .map(|s| s.label)
            .collect()
    };
    assert_eq!(
        labels("ICM42688P"),
        [
            "identity",
            "configure 1",
            "configure 2",
            "configure 3",
            "data"
        ]
    );
    assert_eq!(labels("BMP581"), ["identity", "configure", "data"]);
    assert_eq!(labels("ICM20948"), ["identity", "configure", "data"]);
    let truncated = (29 - 25) * 13_248 / 100;
    assert_eq!(truncated, 529);
    for part in ["ICM42688P", "BMP581"] {
        let p = t.get(part).unwrap();
        assert_eq!(p.lifecycle, Lifecycle::Active, "{part}");
        assert!(!p.stand_in, "{part}");
        assert_eq!(p.pin_capacitance_femtofarads, None, "{part}: D10");
    }
    // The stand-ins wait their start-up too (`AET-WO-4230` §1.9): twice the
    // typical figure, the datasheets print no maximum.
    for (part, wait) in [("ICM20948", 40_000), ("ICP10111", 340)] {
        let waits: Vec<u32> = t
            .get(part)
            .unwrap()
            .steps()
            .iter()
            .map(|s| s.settle_us)
            .collect();
        assert_eq!(waits, [0, wait, 0], "{part}");
    }
}

/// `AET-WO-4220`: the configuration list, the wait and the wider words are
/// each refused when malformed.
#[test]
fn refuses_a_malformed_schema_4_script() {
    type Mutate = fn(&mut Value);
    let cases: [(&str, &str, Mutate); 9] = [
        ("BMP581", "settle_us", |p| {
            p["script"]["configure"]["settle_us"] = Value::Null
        }),
        ("BMP581", "settle_us", |p| {
            p["script"]["configure"]["settle_us"] = 0.into()
        }),
        ("BMP581", "settle_us", |p| {
            p["script"]["configure"]["settle_us"] = 40_001.into()
        }),
        ("BMP581", "settle_us", |p| {
            p["script"]["configure"]
                .as_object_mut()
                .unwrap()
                .remove("settle_us");
        }),
        ("ICM42688P", "script.configure.writes", |p| {
            let w = p["script"]["configure"]["writes"][0].clone();
            p["script"]["configure"]["writes"] =
                json!([w.clone(), w.clone(), w.clone(), w.clone(), w]);
        }),
        ("ICM42688P", "script.configure.writes[1]", |p| {
            p["script"]["configure"]["writes"][1] = json!([])
        }),
        ("BMP581", "bytes", |p| {
            p["script"]["data"]["channels"][0]["bytes"] = 4.into()
        }),
        ("BMP581", "order", |p| {
            p["script"]["data"]["channels"][0]["order"] = "middle".into()
        }),
        ("BMP581", "more than 8", |p| {
            let c = p["script"]["data"]["channels"][0].clone();
            p["script"]["data"]["channels"] = json!([c.clone(), c.clone(), c]);
        }),
    ];
    for (part, field, mutate) in cases {
        let err = refused_entry(part, mutate);
        assert_refused(&err, Some(part), field);
    }
    // A 24-bit signed word holds what 16 bits cannot, and no more: at 64
    // counts a pascal, -131,072 Pa is -2^23 and fits; one pascal either side
    // of the range does not (adversarial: the bound's mutants).
    let err = refused_entry("BMP581", |p| {
        p["script"]["data"]["channels"][1]["stimulus"] = 131_072.into()
    });
    assert_refused(&err, Some("BMP581"), "24-bit word");
    let err = refused_entry("BMP581", |p| {
        p["script"]["data"]["channels"][1]["stimulus"] = (-131_073).into()
    });
    assert_refused(&err, Some("BMP581"), "24-bit word");
    let mut doc = embedded();
    doc.pointer_mut(&at("BMP581")).unwrap()["script"]["data"]["channels"][1]["stimulus"] =
        (-131_072).into();
    let t = parse_parts(&doc.to_string()).unwrap();
    let data = t.get("BMP581").unwrap().steps().pop().unwrap();
    assert_eq!(
        data.expected & 0xFF_FFFF,
        0x00_0080,
        "-2^23 is 0x800000, XLSB first"
    );
    // A word of one byte is no channel this table reads.
    let err = refused_entry("BMP581", |p| {
        p["script"]["data"]["channels"][0]["bytes"] = 1.into()
    });
    assert_refused(&err, Some("BMP581"), "bytes");
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
        ("script.configure.writes", |p| {
            p["script"]["configure"]["writes"] = json!([])
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
            p["script"]["data"]["channels"][0]["quantity"] = "humidity".into()
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

// ---- schema 6 (`AET-WO-4230` §1.9): the waits, statically ----------------------

/// Parses the embedded table after `mutate` edits the entry for `name`'s
/// `script.configure`.
fn configure_refused(
    name: &str,
    mutate: impl FnOnce(&mut Value),
) -> aethelgard_ir::IrValidationError {
    refused_entry(name, |p| mutate(&mut p["script"]["configure"]))
}

#[test]
fn a_settle_below_the_cited_maximum_is_refused_and_the_maximum_itself_passes() {
    // BMP581: 4000 + 1050 + 1050 = 6100 us max.
    let err = configure_refused("BMP581", |c| c["settle_us"] = 6_099.into());
    assert_refused(&err, Some("BMP581"), "SettleBelowStartUp");
    assert_refused(&err, Some("BMP581"), "6099, below 6100");
    let mut doc = embedded();
    doc.pointer_mut(&at("BMP581")).unwrap()["script"]["configure"]["settle_us"] = 6_100.into();
    let t = parse_parts(&doc.to_string()).unwrap();
    assert_eq!(
        t.get("BMP581").unwrap().script.as_ref().unwrap().settle_us,
        Some(6_100)
    );
}

#[test]
fn a_maximum_outranks_a_typical_figure() {
    // A cited maximum binds even where twice the typical would be less.
    let err = configure_refused("BMP581", |c| {
        c["start_up"]["max_us"] = 9_000.into();
        c["start_up"]["typ_us"] = 1_000.into();
        c["settle_us"] = 8_999.into();
    });
    assert_refused(&err, Some("BMP581"), "SettleBelowStartUp");
    assert_refused(&err, Some("BMP581"), "cited start-up maximum");
}

#[test]
fn a_settle_below_twice_a_typical_only_figure_is_refused_by_its_named_condition() {
    // ICM42688P: 10,000 us typ, no maximum: 20,000 us is the least.
    let err = configure_refused("ICM42688P", |c| c["settle_us"] = 19_999.into());
    assert_refused(&err, Some("ICM42688P"), "SettleBelowStartUp");
    assert_refused(
        &err,
        Some("ICM42688P"),
        "settle covers twice the typical-only start-up",
    );
    let mut doc = embedded();
    doc.pointer_mut(&at("ICM42688P")).unwrap()["script"]["configure"]["settle_us"] = 20_000.into();
    parse_parts(&doc.to_string()).unwrap();
}

#[test]
fn a_null_settle_beside_a_start_up_figure_is_refused_even_on_a_stand_in() {
    for part in ["ICM20948", "ICP10111"] {
        let err = configure_refused(part, |c| c["settle_us"] = Value::Null);
        assert_refused(&err, Some(part), "SettleBelowStartUp");
        assert_refused(&err, Some(part), "null beside a cited start-up");
    }
}

#[test]
fn a_stand_in_with_no_printed_start_up_may_carry_both_null_and_no_other_part_may() {
    let both_null = |c: &mut Value| {
        c["start_up"]["max_us"] = Value::Null;
        c["start_up"]["typ_us"] = Value::Null;
    };
    for (part, wait) in [("ICM20948", Value::Null), ("ICP10111", 1.into())] {
        let mut doc = embedded();
        let c = &mut doc.pointer_mut(&at(part)).unwrap()["script"]["configure"];
        both_null(c);
        c["settle_us"] = wait;
        parse_parts(&doc.to_string()).unwrap();
    }
    let err = configure_refused("BMP581", both_null);
    assert_refused(&err, Some("BMP581"), "neither max_us nor typ_us");
}

#[test]
fn a_malformed_start_up_is_refused() {
    type Mutate = fn(&mut Value);
    let cases: [(&str, Mutate); 6] = [
        ("start_up", |c| {
            c.as_object_mut().unwrap().remove("start_up");
        }),
        ("max_us", |c| {
            c["start_up"].as_object_mut().unwrap().remove("max_us");
        }),
        ("max_us", |c| c["start_up"]["max_us"] = 0.into()),
        ("typ_us 7000 above max_us 6100", |c| {
            c["start_up"]["typ_us"] = 7_000.into()
        }),
        ("condition", |c| c["start_up"]["condition"] = "".into()),
        ("extra", |c| c["start_up"]["extra"] = 1.into()),
    ];
    for (field, mutate) in cases {
        let err = configure_refused("BMP581", mutate);
        assert_refused(&err, Some("BMP581"), field);
    }
}

#[test]
fn a_start_up_and_its_citation_come_together() {
    let err = refused_entry("BMP581", |p| p["citations"]["start_up"] = Value::Null);
    assert_refused(&err, Some("BMP581"), "citations.start_up");
    let err = refused_entry("BMP581", |p| {
        p["citations"].as_object_mut().unwrap().remove("start_up");
    });
    assert_refused(&err, Some("BMP581"), "start_up");
    let err = refused_entry("BME280", |p| {
        p["citations"]["start_up"] = p["citations"]["bus"].clone()
    });
    assert_refused(&err, Some("BME280"), "citations.start_up");
}

#[test]
fn every_shipped_script_row_cites_its_start_up_and_its_wait_covers_it() {
    let t = table().unwrap();
    let mut seen = Vec::new();
    for p in t.parts() {
        let Some(script) = &p.script else {
            assert!(p.citations.start_up.is_none(), "{}", p.part);
            continue;
        };
        let (need, _) = script.start_up.required_settle().unwrap_or_else(|| {
            panic!(
                "{}: the shipped table prints a start-up for every script",
                p.part
            )
        });
        let settle = script
            .settle_us
            .unwrap_or_else(|| panic!("{}: no wait", p.part));
        assert!(settle >= need, "{}: {settle} < {need}", p.part);
        assert!(
            matches!(p.citations.start_up.as_ref().unwrap().held, Held::Sha256(_)),
            "{}",
            p.part
        );
        seen.push((p.part.as_str(), need, settle));
    }
    assert_eq!(
        seen,
        [
            ("BMP581", 6_100, 10_000),
            ("ICM20948", 40_000, 40_000),
            ("ICM42688P", 20_000, 20_000),
            ("ICP10111", 340, 340),
        ]
    );
}
