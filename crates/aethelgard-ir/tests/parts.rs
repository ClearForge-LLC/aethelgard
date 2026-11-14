//! The parts table (`AET-WO-4200` D3): the embedded table's values, its pin,
//! and one refusal per malformed field, each through `parse_parts`.

use aethelgard_ir::parts::{
    Bus, Held, MAX_SPI_CLOCK_HZ, MAX_SUPPLY_MILLIVOLTS, PARTS_JSON, parse_parts, table,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// The sha256 of `data/parts.json`. It moves only with the file, on purpose.
const PARTS_SHA256: &str = "bbf2c111038a6217ce24a18f339df5a401766e5789a7403997f51a387d32f7a6";

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
fn the_embedded_table_parses_and_holds_four_parts() {
    let t = table().unwrap();
    let names: Vec<&str> = t.parts().iter().map(|p| p.part.as_str()).collect();
    assert_eq!(names, ["BME280", "ICM20948", "TMP117", "W25Q64JV"]);
    assert_eq!(parse_parts(PARTS_JSON).unwrap(), *t);
}

#[test]
fn no_citation_is_pending() {
    // Every citation's document is held in ~/.cache/aet-4200/ds/, recorded in
    // the work order's FEEDBACK.
    assert_eq!(table().unwrap().pending(), Vec::<(&str, &str, &str)>::new());
}

#[test]
fn every_entry_has_all_three_citations_held() {
    for p in table().unwrap().parts() {
        for c in [&p.citations.bus, &p.citations.identity, &p.citations.supply] {
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
        (p.identity.write, p.identity.width, p.identity.value),
        (0xD0, 1, 0x60)
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
        (p.identity.write, p.identity.width, p.identity.value),
        (0x0F, 2, 0x0117)
    );
    assert_eq!(
        p.bus,
        Bus::I2c {
            addresses: vec![0x48, 0x49, 0x4A, 0x4B]
        }
    );
    assert_eq!(p.supply_millivolts, (1800, 5500));
    assert_eq!(p.renode, None);

    let p = get("W25Q64JV");
    assert_eq!(
        (p.identity.write, p.identity.width, p.identity.value),
        (0x9F, 3, 0xEF4017)
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

    let p = get("ICM20948");
    assert_eq!(
        (p.identity.write, p.identity.width, p.identity.value),
        (0x80, 1, 0xEA)
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
fn refuses_schema_version_2() {
    let mut doc = embedded();
    doc["schema_version"] = 2.into();
    let err = parse_parts(&doc.to_string()).unwrap_err();
    assert_refused(&err, None, "schema_version");
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
    let err = refused_entry("W25Q64JV", |p| p["bus"]["spi"]["extra"] = 1.into());
    assert_refused(&err, Some("W25Q64JV"), "extra");
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
    let err = refused_entry("W25Q64JV", |p| p["bus"]["spi"]["modes"] = json!([0, 4]));
    assert_refused(&err, Some("W25Q64JV"), "bus.spi.modes");
    let err = refused_entry("W25Q64JV", |p| p["bus"]["spi"]["modes"] = json!([3, 3]));
    assert_refused(&err, Some("W25Q64JV"), "bus.spi.modes");
    let err = refused_entry("W25Q64JV", |p| p["bus"]["spi"]["modes"] = json!([]));
    assert_refused(&err, Some("W25Q64JV"), "bus.spi.modes");
}

#[test]
fn refuses_a_max_clock_of_zero_or_above_the_ceiling() {
    for hz in [0, u64::from(MAX_SPI_CLOCK_HZ) + 1] {
        let err = refused_entry("W25Q64JV", |p| p["bus"]["spi"]["max_clock_hz"] = hz.into());
        assert_refused(&err, Some("W25Q64JV"), "bus.spi.max_clock_hz");
    }
    // The ceiling itself is accepted.
    let mut doc = embedded();
    doc.pointer_mut(&at("W25Q64JV")).unwrap()["bus"]["spi"]["max_clock_hz"] =
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
        let err = refused_entry("BME280", |p| p["identity"]["write"] = bad.into());
        assert_refused(&err, Some("BME280"), "identity.write");
    }
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
    let err = refused_entry("W25Q64JV", |p| p["renode"] = "I2C.BME280".into());
    assert_refused(&err, Some("W25Q64JV"), "renode");
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
