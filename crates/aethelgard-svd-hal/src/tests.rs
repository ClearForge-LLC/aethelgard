use super::*;

/// The pinned sha256 of the table's JSON, computed on Furnace and reproduced in
/// `gates`. A mismatch between the two is a stop, not a constant to update.
const TABLE_JSON_SHA256: &str = "80dcd2751a3a0036f7609c06929a33b6446885b5ac04a380f34afcf0f61d9e2c";

fn source() -> PinSource {
    PinSource {
        repository: "r".into(),
        commit: "c".into(),
        files: BTreeMap::new(),
    }
}

fn mcu(pins: &str) -> String {
    format!(
        r#"<Mcu Package="LQFP144" RefName="STM32H753ZITx" xmlns="http://dummy.com">{pins}</Mcu>"#
    )
}

fn gpio(pins: &str) -> String {
    format!(r#"<IP Name="GPIO" xmlns="http://dummy.com">{pins}</IP>"#)
}

fn af(signal: &str, value: &str) -> String {
    format!(
        r#"<PinSignal Name="{signal}"><SpecificParameter Name="GPIO_AF"><PossibleValue>{value}</PossibleValue></SpecificParameter></PinSignal>"#
    )
}

fn io(name: &str, position: &str, signals: &[&str]) -> String {
    let signals: String = signals
        .iter()
        .map(|s| format!(r#"<Signal Name="{s}"/>"#))
        .collect();
    format!(r#"<Pin Name="{name}" Position="{position}" Type="I/O">{signals}</Pin>"#)
}

fn gpio_pin(name: &str, signals: &str) -> String {
    format!(r#"<GPIO_Pin PortName="PB" Name="{name}">{signals}</GPIO_Pin>"#)
}

fn run(mcu_pins: &str, gpio_pins: &str) -> Result<PinTable, PinTableError> {
    parse(
        mcu(mcu_pins).as_bytes(),
        gpio(gpio_pins).as_bytes(),
        source(),
    )
}

fn malformed_detail(result: Result<PinTable, PinTableError>) -> String {
    match result {
        Err(PinTableError::MalformedSource { detail }) => detail,
        other => panic!("expected MalformedSource, got {other:?}"),
    }
}

fn real() -> PinTable {
    pin_table("STM32H753ZITx").unwrap()
}

#[test]
fn golden_facts() {
    let table = real();
    assert_eq!(table.part, "STM32H753ZITx");
    assert_eq!(table.package, "LQFP144");
    assert_eq!(table.pins.len(), 114);
    let pb8 = &table.pins["PB8"];
    assert_eq!(pb8.position, 139);
    assert_eq!(pb8.alternate_functions["I2C1_SCL"], 4);
    assert!(table.pins.contains_key("PC14"));
    assert_eq!(table.validate(), Ok(()));
    for key in table.pins.keys() {
        assert!(key.len() <= 4 && key.starts_with('P'), "{key}");
    }
}

#[test]
fn the_eleven_suffixed_names_reduce_to_canonical_ones() {
    let table = real();
    for name in [
        "PC14", "PC15", "PH0", "PH1", "PC2", "PC3", "PA13", "PA14", "PA15", "PB3", "PB4",
    ] {
        assert!(table.pins.contains_key(name), "{name}");
    }
}

#[test]
fn source_records_the_pinned_files() {
    let table = real();
    assert_eq!(table.source.commit, COMMIT);
    assert_eq!(table.source.repository, REPOSITORY);
    assert_eq!(table.source.files[MCU_FILE], MCU_SHA256);
    assert_eq!(table.source.files[GPIO_FILE], GPIO_SHA256);
}

#[test]
fn only_this_packages_functions_are_kept() {
    // PB9 carries SPI2_NSS and I2C functions in the GPIO file; every kept signal
    // is one the MCU file lists for that pin.
    let doc = text(MCU_BYTES, MCU_FILE).unwrap();
    let mcu_doc = document(&doc, MCU_FILE).unwrap();
    for pin in children(mcu_doc.root_element(), "Pin") {
        let Some(name) = pin.attribute("Name") else {
            continue;
        };
        if pin.attribute("Type") != Some("I/O") {
            continue;
        }
        let listed: BTreeSet<&str> = children(pin, "Signal")
            .filter_map(|s| s.attribute("Name"))
            .collect();
        for signal in real().pins[canonical(name).unwrap()]
            .alternate_functions
            .keys()
        {
            assert!(listed.contains(signal.as_str()), "{name} {signal}");
        }
    }
}

#[test]
fn table_json_is_deterministic() {
    let json = serde_json::to_string(&real()).unwrap();
    assert_eq!(json, serde_json::to_string(&real()).unwrap());
    assert_eq!(sha256_hex(json.as_bytes()), TABLE_JSON_SHA256);
}

#[test]
fn sha256_hex_matches_a_known_vector() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn an_unknown_part_is_refused() {
    let err = pin_table("STM32F407VGTx").unwrap_err();
    assert_eq!(err.kind(), "UnknownPart");
    assert_eq!(pin_table("").unwrap_err().kind(), "UnknownPart");
    assert_eq!(
        pin_table("stm32h753zitx").unwrap_err().kind(),
        "UnknownPart"
    );
}

#[test]
fn a_changed_byte_in_either_file_is_refused_by_name() {
    for (which, expected) in [(0, MCU_FILE), (1, GPIO_FILE)] {
        for edit in ["flip", "append", "truncate"] {
            let (mut m, mut g) = (MCU_BYTES.to_vec(), GPIO_BYTES.to_vec());
            let bytes = if which == 0 { &mut m } else { &mut g };
            match edit {
                "flip" => bytes[1000] ^= 1,
                "append" => bytes.push(b'\n'),
                _ => {
                    bytes.pop();
                }
            }
            match build(&m, &g) {
                Err(PinTableError::SourceHashMismatch { file, .. }) => assert_eq!(file, expected),
                other => panic!("{expected} {edit}: {other:?}"),
            }
        }
    }
    assert_eq!(build(MCU_BYTES, GPIO_BYTES).unwrap(), real());
}

#[test]
fn a_clean_fixture_parses_and_leaves_out_what_it_should() {
    let table = run(
        &format!(
            "{}{}{}",
            io("PB8 (X)", "139", &["I2C1_SCL", "ADC_IN1", "OTHER"]),
            r#"<Pin Name="VDD" Position="1" Type="Power"/>"#,
            io("PB9", "140", &[]),
        ),
        &format!(
            "{}{}",
            gpio_pin(
                "PB8 (X)",
                &format!(
                    "{}{}{}<PinSignal Name=\"ADC_IN1\"/>",
                    af("I2C1_SCL", "GPIO_AF4_I2C1"),
                    af("OTHER", "GPIO_AF15_X"),
                    af("NOT_LISTED", "GPIO_AF0_X"),
                )
            ),
            gpio_pin("PB9", ""),
        ),
    )
    .unwrap();
    assert_eq!(table.pins.len(), 2);
    let pb8 = &table.pins["PB8"];
    assert_eq!(pb8.position, 139);
    let expected: BTreeMap<String, u8> = [("I2C1_SCL".into(), 4), ("OTHER".into(), 15)].into();
    assert_eq!(pb8.alternate_functions, expected);
    assert!(table.pins["PB9"].alternate_functions.is_empty());
}

#[test]
fn names_without_a_canonical_run_are_refused() {
    for bad in ["", "BOOT0", "PL1", "PA", "pa1", "PA123", "XPA1", "P1"] {
        let detail = malformed_detail(run(&io(bad, "1", &[]), &gpio_pin(bad, "")));
        assert!(detail.contains(bad), "{bad}: {detail}");
    }
}

#[test]
fn two_pins_with_one_canonical_name_are_refused() {
    let detail = malformed_detail(run(
        &format!("{}{}", io("PC2", "1", &[]), io("PC2_C", "2", &[])),
        &format!("{}{}", gpio_pin("PC2", ""), gpio_pin("PC2_C", "")),
    ));
    assert!(detail.contains("PC2"), "{detail}");
}

#[test]
fn two_pins_at_one_position_are_refused() {
    let detail = malformed_detail(run(
        &format!("{}{}", io("PA1", "7", &[]), io("PA2", "7", &[])),
        &format!("{}{}", gpio_pin("PA1", ""), gpio_pin("PA2", "")),
    ));
    assert!(detail.contains("position 7"), "{detail}");
    for bad in ["x", "", "-1", "70000"] {
        malformed_detail(run(&io("PA1", bad, &[]), &gpio_pin("PA1", "")));
    }
}

#[test]
fn bad_af_forms_are_refused_naming_the_pin_and_signal() {
    for bad in [
        "",
        "AF4",
        "GPIO_AF16_X",
        "GPIO_AF_X",
        "GPIO_AF4",
        "GPIO_AF4_",
        "GPIO_AF04_X",
        "GPIO_AFx_X",
        "GPIO_AF+4_X",
        "GPIO_AF-1_X",
        "gpio_af4_x",
        "GPIO_AF256_X",
    ] {
        let detail = malformed_detail(run(
            &io("PA1", "1", &["S"]),
            &gpio_pin("PA1", &af("S", bad)),
        ));
        assert!(
            detail.contains("PA1") && detail.contains('S'),
            "{bad}: {detail}"
        );
    }
    for ok in ["GPIO_AF0_X", "GPIO_AF15_X", "GPIO_AF4_I2C1", "GPIO_AF4_A_B"] {
        run(&io("PA1", "1", &["S"]), &gpio_pin("PA1", &af("S", ok))).unwrap();
    }
}

#[test]
fn a_pin_missing_from_the_gpio_file_is_refused() {
    let detail = malformed_detail(run(&io("PA1", "1", &[]), &gpio_pin("PA2", "")));
    assert!(detail.contains("PA1"), "{detail}");
}

#[test]
fn a_gpio_pin_listed_twice_is_refused() {
    malformed_detail(run(
        &io("PA1", "1", &[]),
        &format!("{}{}", gpio_pin("PA1", ""), gpio_pin("PA1", "")),
    ));
}

#[test]
fn one_signal_with_two_af_numbers_is_refused() {
    let both = format!("{}{}", af("S", "GPIO_AF4_X"), af("S", "GPIO_AF5_X"));
    let detail = malformed_detail(run(&io("PA1", "1", &["S"]), &gpio_pin("PA1", &both)));
    assert!(detail.contains("PA1 S "), "{detail}");
    // The same number twice is not a conflict.
    let same = format!("{}{}", af("S", "GPIO_AF4_X"), af("S", "GPIO_AF4_Y"));
    let table = run(&io("PA1", "1", &["S"]), &gpio_pin("PA1", &same)).unwrap();
    assert_eq!(table.pins["PA1"].alternate_functions["S"], 4);
}

#[test]
fn a_signal_name_outside_rul_14_is_refused() {
    let long = "S".repeat(65);
    for bad in [long.as_str(), "A B"] {
        malformed_detail(run(
            &io("PA1", "1", &[bad]),
            &gpio_pin("PA1", &af(bad, "GPIO_AF1_X")),
        ));
    }
}

#[test]
fn the_wrong_mcu_or_broken_xml_is_refused() {
    let other = r#"<Mcu Package="P" RefName="OTHER"/>"#;
    malformed_detail(parse(other.as_bytes(), gpio("").as_bytes(), source()));
    let none = r#"<Mcu RefName="STM32H753ZITx"/>"#;
    malformed_detail(parse(none.as_bytes(), gpio("").as_bytes(), source()));
    malformed_detail(parse(b"<Mcu", gpio("").as_bytes(), source()));
    malformed_detail(parse(&[0xff, 0xfe], gpio("").as_bytes(), source()));
    malformed_detail(parse(mcu("").as_bytes(), &[0xff], source()));
}

#[test]
fn a_dtd_is_refused() {
    let dtd = format!("<!DOCTYPE Mcu [<!ENTITY x \"y\">]>{}", mcu(""));
    malformed_detail(parse(dtd.as_bytes(), gpio("").as_bytes(), source()));
}
