use aethelgard_ir::{
    AnalogLine, I2cBus, I2cDevice, IrValidationError as E, Mcu, NetConstraint, PinAssignment,
    Signal, SolvedI2cDevice, SolvedTopology, Specification, SpiBus, SpiDevice, VoltageDomain,
};

fn sig(net: &str) -> Signal {
    Signal {
        net: net.into(),
        pin: None,
    }
}

fn valid() -> Specification {
    Specification {
        schema_version: 1,
        project_name: "sentinel-01".into(),
        mcu: Mcu {
            part: "STM32H753ZITx".into(),
            supply: "VDD_3V3".into(),
        },
        voltage_domains: vec![VoltageDomain {
            id: "VDD_3V3".into(),
            nominal_millivolts: 3300,
            tolerance_permille: 50,
        }],
        i2c_buses: vec![I2cBus {
            id: "SENSORS".into(),
            peripheral: None,
            speed_hz: 400_000,
            pull_up_ohms: 2200,
            domain: "VDD_3V3".into(),
            sda: sig("I2C_SDA"),
            scl: sig("I2C_SCL"),
        }],
        spi_buses: vec![SpiBus {
            id: "FLASH".into(),
            peripheral: None,
            speed_hz: 20_000_000,
            domain: "VDD_3V3".into(),
            sck: sig("SPI_SCK"),
            mosi: sig("SPI_MOSI"),
            miso: sig("SPI_MISO"),
        }],
        i2c_devices: vec![
            I2cDevice {
                reference: "U2".into(),
                part: "BME280".into(),
                supply: "VDD_3V3".into(),
                bus: "SENSORS".into(),
                address: 118,
            },
            I2cDevice {
                reference: "U3".into(),
                part: "TMP117".into(),
                supply: "VDD_3V3".into(),
                bus: "SENSORS".into(),
                address: 72,
            },
        ],
        spi_devices: vec![SpiDevice {
            reference: "U4".into(),
            part: "W25Q64JV".into(),
            supply: "VDD_3V3".into(),
            bus: "FLASH".into(),
            cs: sig("FLASH_CS"),
        }],
        net_constraints: vec![NetConstraint {
            net_name: "I2C_SDA".into(),
            max_capacitance_femtofarads: 400_000,
            max_rise_time_nanoseconds: 300,
            impedance_ohms: None,
        }],
    }
}

fn refusal(mutate: impl FnOnce(&mut Specification)) -> E {
    let mut spec = valid();
    mutate(&mut spec);
    spec.validate().expect_err("mutated spec must be refused")
}

/// Asserts the kind and `id()` of a refusal.
fn assert_refusal(err: E, kind: &str, id: Option<&str>) {
    assert_eq!(err.kind(), kind, "{err}");
    assert_eq!(err.id(), id, "{err}");
}

fn assert_invalid_identifier(err: E, path: &str) {
    assert_eq!(err.kind(), "InvalidIdentifier", "{err}");
    assert_eq!(err.id(), None);
    assert_eq!(err.path(), Some(path));
}

#[test]
fn valid_spec_passes() {
    assert_eq!(valid().validate(), Ok(()));
}

#[test]
fn empty_lists_other_than_domains_are_valid() {
    let mut spec = valid();
    spec.i2c_buses.clear();
    spec.spi_buses.clear();
    spec.i2c_devices.clear();
    spec.spi_devices.clear();
    spec.net_constraints.clear();
    assert_eq!(spec.validate(), Ok(()));
}

#[test]
fn refuses_unsupported_schema_version() {
    for version in [0, 2, u32::MAX] {
        let err = refusal(|s| s.schema_version = version);
        assert_refusal(err, "UnsupportedSchemaVersion", None);
    }
}

#[test]
fn refuses_invalid_identifiers_at_every_path() {
    type Mutator = fn(&mut Specification);
    let cases: Vec<(&str, Mutator)> = vec![
        ("project_name", |s| s.project_name = String::new()),
        ("mcu.part", |s| s.mcu.part = "a b".into()),
        ("mcu.supply", |s| s.mcu.supply = " ".into()),
        ("voltage_domains[0].id", |s| {
            s.voltage_domains[0].id = "".into()
        }),
        ("i2c_buses[0].id", |s| s.i2c_buses[0].id = "x y".into()),
        ("i2c_buses[0].peripheral", |s| {
            s.i2c_buses[0].peripheral = Some("".into())
        }),
        ("i2c_buses[0].domain", |s| {
            s.i2c_buses[0].domain = "a\tb".into()
        }),
        ("i2c_buses[0].sda.net", |s| {
            s.i2c_buses[0].sda.net = "".into()
        }),
        ("i2c_buses[0].sda.pin", |s| {
            s.i2c_buses[0].sda.pin = Some("P B".into())
        }),
        ("i2c_buses[0].scl.net", |s| {
            s.i2c_buses[0].scl.net = "".into()
        }),
        ("i2c_buses[0].scl.pin", |s| {
            s.i2c_buses[0].scl.pin = Some("".into())
        }),
        ("spi_buses[0].id", |s| s.spi_buses[0].id = "".into()),
        ("spi_buses[0].peripheral", |s| {
            s.spi_buses[0].peripheral = Some(" ".into())
        }),
        ("spi_buses[0].domain", |s| s.spi_buses[0].domain = "".into()),
        ("spi_buses[0].sck.net", |s| {
            s.spi_buses[0].sck.net = "".into()
        }),
        ("spi_buses[0].sck.pin", |s| {
            s.spi_buses[0].sck.pin = Some("".into())
        }),
        ("spi_buses[0].mosi.net", |s| {
            s.spi_buses[0].mosi.net = "".into()
        }),
        ("spi_buses[0].mosi.pin", |s| {
            s.spi_buses[0].mosi.pin = Some("".into())
        }),
        ("spi_buses[0].miso.net", |s| {
            s.spi_buses[0].miso.net = "".into()
        }),
        ("spi_buses[0].miso.pin", |s| {
            s.spi_buses[0].miso.pin = Some("".into())
        }),
        ("i2c_devices[1].reference", |s| {
            s.i2c_devices[1].reference = "".into()
        }),
        ("i2c_devices[0].part", |s| s.i2c_devices[0].part = "".into()),
        ("i2c_devices[0].supply", |s| {
            s.i2c_devices[0].supply = "".into()
        }),
        ("i2c_devices[0].bus", |s| s.i2c_devices[0].bus = "".into()),
        ("spi_devices[0].reference", |s| {
            s.spi_devices[0].reference = "".into()
        }),
        ("spi_devices[0].part", |s| s.spi_devices[0].part = "".into()),
        ("spi_devices[0].supply", |s| {
            s.spi_devices[0].supply = "".into()
        }),
        ("spi_devices[0].bus", |s| s.spi_devices[0].bus = "".into()),
        ("spi_devices[0].cs.net", |s| {
            s.spi_devices[0].cs.net = "".into()
        }),
        ("spi_devices[0].cs.pin", |s| {
            s.spi_devices[0].cs.pin = Some("".into())
        }),
        ("net_constraints[0].net_name", |s| {
            s.net_constraints[0].net_name = "".into()
        }),
    ];
    for (path, mutate) in cases {
        assert_invalid_identifier(refusal(mutate), path);
    }
}

#[test]
fn identifier_boundaries() {
    let bad = [
        "",
        " ",
        " VDD_3V3",
        "VDD_3V3 ",
        "VDD 3V3",
        "VDD_3V3\n",
        "VDD_\u{0412}V3",  // Cyrillic В
        "VDD_3V3\u{200b}", // zero-width space
        "VDD\x7f",
        "caf\u{e9}",
        // KiCad reads these as more than text (AET-WO-1300 S5, D6).
        "/SDA",
        "A/B",
        "A{slash}B",
        "{",
        "}",
        "${PROJECTNAME}",
        "A$B",
        "A[0..1]",
        "]",
    ];
    for id in bad {
        let err = refusal(|s| s.project_name = id.into());
        assert_invalid_identifier(err, "project_name");
    }
    let long = "a".repeat(65);
    assert_invalid_identifier(refusal(|s| s.project_name = long), "project_name");
    for ok in [
        "a".repeat(64),
        "!".into(),
        "~".into(),
        "+3V3".into(),
        // Every other printable character, S-expression specials included.
        "!\"#%&'()*+,-.:;<=>?@\\^_`|~".into(),
    ] {
        let mut spec = valid();
        spec.project_name = ok;
        assert_eq!(spec.validate(), Ok(()));
    }
}

#[test]
fn first_invalid_identifier_wins_in_key_order() {
    let err = refusal(|s| {
        s.net_constraints[0].net_name = "".into();
        s.spi_devices[0].cs.net = "".into();
        s.i2c_buses[0].scl.pin = Some("".into());
    });
    assert_invalid_identifier(err, "i2c_buses[0].scl.pin");
}

#[test]
fn refuses_no_voltage_domain() {
    assert_refusal(
        refusal(|s| s.voltage_domains.clear()),
        "NoVoltageDomain",
        None,
    );
}

fn second_domain(spec: &mut Specification, id: &str) {
    let mut domain = spec.voltage_domains[0].clone();
    domain.id = id.into();
    spec.voltage_domains.push(domain);
}

#[test]
fn refuses_duplicate_domain_id() {
    let err = refusal(|s| second_domain(s, "VDD_3V3"));
    assert_refusal(err, "DuplicateDomainId", Some("VDD_3V3"));
}

#[test]
fn padded_ids_are_invalid_identifiers_not_duplicates() {
    let err = refusal(|s| second_domain(s, " VDD_3V3\t"));
    assert_invalid_identifier(err, "voltage_domains[1].id");
    let err = refusal(|s| {
        s.net_constraints.push(s.net_constraints[0].clone());
        s.net_constraints[1].net_name = "I2C_SDA ".into();
    });
    assert_invalid_identifier(err, "net_constraints[1].net_name");
}

#[test]
fn refuses_zero_nominal_voltage() {
    let err = refusal(|s| s.voltage_domains[0].nominal_millivolts = 0);
    assert_refusal(err, "ZeroNominalVoltage", Some("VDD_3V3"));
}

#[test]
fn refuses_tolerance_out_of_range() {
    for value in [0, 501, u16::MAX] {
        let err = refusal(|s| s.voltage_domains[0].tolerance_permille = value);
        assert_eq!(
            err,
            E::ToleranceOutOfRange {
                id: "VDD_3V3".into(),
                value
            }
        );
        assert_eq!(err.id(), Some("VDD_3V3"));
    }
    for value in [1, 500] {
        let mut spec = valid();
        spec.voltage_domains[0].tolerance_permille = value;
        assert_eq!(spec.validate(), Ok(()));
    }
}

#[test]
fn refuses_unknown_domain_at_each_use() {
    type Mutator = fn(&mut Specification);
    let cases: Vec<Mutator> = vec![
        |s| s.mcu.supply = "NOPE".into(),
        |s| s.i2c_buses[0].domain = "NOPE".into(),
        |s| s.spi_buses[0].domain = "NOPE".into(),
        |s| s.i2c_devices[0].supply = "NOPE".into(),
        |s| s.spi_devices[0].supply = "NOPE".into(),
    ];
    for mutate in cases {
        assert_refusal(refusal(mutate), "UnknownDomain", Some("NOPE"));
    }
}

fn with_second_domain(s: &mut Specification) {
    let mut other = s.voltage_domains[0].clone();
    other.id = "VDD_1V8".into();
    s.voltage_domains.push(other);
}

#[test]
fn refuses_a_bus_off_the_mcu_rail() {
    let err = refusal(|s| {
        with_second_domain(s);
        s.i2c_buses[0].domain = "VDD_1V8".into();
    });
    assert_refusal(err, "SupplyMismatch", Some("SENSORS"));
    let err = refusal(|s| {
        with_second_domain(s);
        s.spi_buses[0].domain = "VDD_1V8".into();
    });
    assert_refusal(err, "SupplyMismatch", Some("FLASH"));
}

#[test]
fn refuses_a_device_off_its_buss_rail() {
    let err = refusal(|s| {
        with_second_domain(s);
        s.i2c_devices[1].supply = "VDD_1V8".into();
    });
    assert_refusal(err, "SupplyMismatch", Some("U3"));
    let err = refusal(|s| {
        with_second_domain(s);
        s.spi_devices[0].supply = "VDD_1V8".into();
    });
    assert_refusal(err, "SupplyMismatch", Some("U4"));
}

#[test]
fn a_device_with_an_unknown_bus_is_left_to_step_nine() {
    let err = refusal(|s| {
        with_second_domain(s);
        s.i2c_devices[0].bus = "NOPE".into();
        s.i2c_devices[0].supply = "VDD_1V8".into();
    });
    assert_refusal(err, "UnknownBus", Some("NOPE"));
}

#[test]
fn refuses_duplicate_bus_id_across_both_lists() {
    let err = refusal(|s| s.spi_buses[0].id = "SENSORS".into());
    assert_refusal(err, "DuplicateBusId", Some("SENSORS"));
    let err = refusal(|s| {
        s.i2c_buses.push(s.i2c_buses[0].clone());
    });
    assert_refusal(err, "DuplicateBusId", Some("SENSORS"));
}

#[test]
fn refuses_unsupported_i2c_speed_zero_pull_up_and_zero_spi_speed() {
    for value in [0, 99_999, 3_400_000] {
        let err = refusal(|s| s.i2c_buses[0].speed_hz = value);
        assert_eq!(
            err,
            E::UnsupportedI2cSpeed {
                id: "SENSORS".into(),
                value
            }
        );
        assert_eq!(err.id(), Some("SENSORS"));
    }
    for value in [100_000, 1_000_000] {
        let mut spec = valid();
        spec.i2c_buses[0].speed_hz = value;
        assert_eq!(spec.validate(), Ok(()));
    }
    assert_refusal(
        refusal(|s| s.i2c_buses[0].pull_up_ohms = 0),
        "ZeroPullUp",
        Some("SENSORS"),
    );
    assert_refusal(
        refusal(|s| s.spi_buses[0].speed_hz = 0),
        "ZeroSpiSpeed",
        Some("FLASH"),
    );
}

#[test]
fn refuses_duplicate_reference_across_both_lists() {
    let err = refusal(|s| s.spi_devices[0].reference = "U2".into());
    assert_refusal(err, "DuplicateReference", Some("U2"));
    let err = refusal(|s| s.i2c_devices[1].reference = "U2".into());
    assert_refusal(err, "DuplicateReference", Some("U2"));
}

#[test]
fn refuses_unknown_bus_and_a_bus_of_the_wrong_kind() {
    let err = refusal(|s| s.i2c_devices[0].bus = "NOPE".into());
    assert_refusal(err, "UnknownBus", Some("NOPE"));
    let err = refusal(|s| s.i2c_devices[0].bus = "FLASH".into());
    assert_refusal(err, "UnknownBus", Some("FLASH"));
    let err = refusal(|s| s.spi_devices[0].bus = "SENSORS".into());
    assert_refusal(err, "UnknownBus", Some("SENSORS"));
}

#[test]
fn refuses_i2c_address_out_of_range_and_duplicate_on_a_bus() {
    for value in [0, 7, 0x78, 255] {
        let err = refusal(|s| s.i2c_devices[0].address = value);
        assert_eq!(
            err,
            E::I2cAddressOutOfRange {
                id: "U2".into(),
                value
            }
        );
        assert_eq!(err.id(), Some("U2"));
    }
    for value in [0x08, 0x77] {
        let mut spec = valid();
        spec.i2c_devices[0].address = value;
        assert_eq!(spec.validate(), Ok(()));
    }
    let err = refusal(|s| s.i2c_devices[1].address = 118);
    assert_eq!(
        err,
        E::DuplicateI2cAddress {
            id: "U3".into(),
            value: 118
        }
    );
    assert_eq!(err.id(), Some("U3"));
}

#[test]
fn same_i2c_address_on_two_buses_is_valid() {
    let mut spec = valid();
    let mut second = spec.i2c_buses[0].clone();
    second.id = "OTHER".into();
    second.sda.net = "I2C2_SDA".into();
    second.scl.net = "I2C2_SCL".into();
    spec.i2c_buses.push(second);
    spec.i2c_devices[1].bus = "OTHER".into();
    spec.i2c_devices[1].address = 118;
    assert_eq!(spec.validate(), Ok(()));
}

#[test]
fn refuses_a_net_declared_twice_across_buses_and_cs() {
    let err = refusal(|s| s.i2c_buses[0].scl.net = "I2C_SDA".into());
    assert_refusal(err, "DuplicateNetName", Some("I2C_SDA"));
    let err = refusal(|s| s.spi_devices[0].cs.net = "SPI_SCK".into());
    assert_refusal(err, "DuplicateNetName", Some("SPI_SCK"));
    let err = refusal(|s| s.spi_buses[0].miso.net = "I2C_SCL".into());
    assert_refusal(err, "DuplicateNetName", Some("I2C_SCL"));
}

#[test]
fn refuses_a_signal_net_named_like_a_domain() {
    let err = refusal(|s| s.spi_devices[0].cs.net = "VDD_3V3".into());
    assert_refusal(err, "NetShadowsDomain", Some("VDD_3V3"));
}

#[test]
fn refuses_net_constraint_faults() {
    let err = refusal(|s| s.net_constraints.push(s.net_constraints[0].clone()));
    assert_refusal(err, "DuplicateNetConstraint", Some("I2C_SDA"));
    let err = refusal(|s| s.net_constraints[0].net_name = "NOPE".into());
    assert_refusal(err, "UnknownNet", Some("NOPE"));
    assert_refusal(
        refusal(|s| s.net_constraints[0].max_capacitance_femtofarads = 0),
        "ZeroCapacitance",
        Some("I2C_SDA"),
    );
    assert_refusal(
        refusal(|s| s.net_constraints[0].max_rise_time_nanoseconds = 0),
        "ZeroRiseTime",
        Some("I2C_SDA"),
    );
    assert_refusal(
        refusal(|s| s.net_constraints[0].impedance_ohms = Some(0)),
        "ZeroImpedance",
        Some("I2C_SDA"),
    );
}

#[test]
fn a_constraint_may_name_a_voltage_domain() {
    let mut spec = valid();
    spec.net_constraints[0].net_name = "VDD_3V3".into();
    assert_eq!(spec.validate(), Ok(()));
}

#[test]
fn checks_run_in_table_order() {
    // Step 3 before step 4/5: no domain beats an unknown domain reference.
    let err = refusal(|s| {
        s.voltage_domains.clear();
        s.mcu.supply = "NOPE".into();
    });
    assert_eq!(err.kind(), "NoVoltageDomain");
    // Step 5 before step 6.
    let err = refusal(|s| {
        s.spi_buses[0].id = "SENSORS".into();
        s.i2c_devices[0].supply = "NOPE".into();
    });
    assert_eq!(err.kind(), "UnknownDomain");
    // Step 9 before step 10.
    let err = refusal(|s| {
        s.i2c_devices[0].bus = "NOPE".into();
        s.i2c_devices[1].address = 0;
    });
    assert_eq!(err.kind(), "UnknownBus");
    // Step 11 before step 12.
    let err = refusal(|s| {
        s.spi_devices[0].cs.net = "SPI_SCK".into();
        s.net_constraints[0].net_name = "NOPE".into();
    });
    assert_eq!(err.kind(), "DuplicateNetName");
}

#[test]
fn a_pinned_signal_is_valid() {
    let mut spec = valid();
    spec.i2c_buses[0].peripheral = Some("I2C1".into());
    spec.i2c_buses[0].sda.pin = Some("PB9".into());
    assert_eq!(spec.validate(), Ok(()));
}

// ---- serde ----------------------------------------------------------------

fn spec_json() -> serde_json::Value {
    serde_json::to_value(valid()).unwrap()
}

fn parse(value: &serde_json::Value) -> Result<Specification, String> {
    serde_json::from_str::<Specification>(&value.to_string()).map_err(|e| e.to_string())
}

#[test]
fn round_trip_is_equal_and_byte_stable() {
    let mut spec = valid();
    spec.net_constraints[0].impedance_ohms = Some(50);
    spec.i2c_buses[0].sda.pin = Some("PB9".into());
    let json = serde_json::to_string(&spec).unwrap();
    let back: Specification = serde_json::from_str(&json).unwrap();
    assert_eq!(back, spec);
    assert_eq!(serde_json::to_string(&back).unwrap(), json);
    assert_eq!(back.validate(), Ok(()));
}

#[test]
fn document_order_is_preserved_and_unsorted_input_is_accepted() {
    let mut spec = valid();
    spec.i2c_devices.reverse();
    let json = serde_json::to_string(&spec).unwrap();
    let back: Specification = serde_json::from_str(&json).unwrap();
    assert_eq!(back.i2c_devices[0].reference, "U3");
    assert_eq!(back.validate(), Ok(()));
    assert_eq!(serde_json::to_string(&back).unwrap(), json);
}

/// Every object path in a valid specification, as a JSON pointer.
const OBJECT_POINTERS: &[&str] = &[
    "",
    "/mcu",
    "/voltage_domains/0",
    "/i2c_buses/0",
    "/i2c_buses/0/sda",
    "/spi_buses/0",
    "/spi_buses/0/sck",
    "/i2c_devices/0",
    "/spi_devices/0",
    "/spi_devices/0/cs",
    "/net_constraints/0",
];

#[test]
fn unknown_field_in_each_struct_is_refused() {
    for pointer in OBJECT_POINTERS {
        let mut doc = spec_json();
        doc.pointer_mut(pointer).unwrap()["extra"] = 1.into();
        let err = parse(&doc).unwrap_err();
        assert!(err.contains("unknown field `extra`"), "{pointer}: {err}");
    }
}

#[test]
fn missing_key_in_each_struct_is_refused() {
    for pointer in OBJECT_POINTERS {
        let keys: Vec<String> = spec_json()
            .pointer(pointer)
            .unwrap()
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        for key in keys {
            let mut doc = spec_json();
            doc.pointer_mut(pointer)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(&key);
            let err = parse(&doc).unwrap_err();
            assert!(
                err.contains(&format!("missing field `{key}`")),
                "{pointer} without {key}: {err}"
            );
        }
    }
}

#[test]
fn missing_optional_keys_are_refused() {
    for (pointer, key) in [
        ("/i2c_buses/0", "peripheral"),
        ("/spi_buses/0", "peripheral"),
        ("/i2c_buses/0/sda", "pin"),
        ("/spi_buses/0/miso", "pin"),
        ("/spi_devices/0/cs", "pin"),
        ("/net_constraints/0", "impedance_ohms"),
    ] {
        let mut doc = spec_json();
        doc.pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(key);
        let err = parse(&doc).unwrap_err();
        assert!(err.contains(&format!("missing field `{key}`")), "{err}");
    }
}

#[test]
fn positional_sequence_form_is_refused_for_each_struct() {
    for pointer in OBJECT_POINTERS {
        let mut doc = spec_json();
        *doc.pointer_mut(pointer).unwrap() = serde_json::json!([]);
        let err = parse(&doc).unwrap_err();
        assert!(err.contains("object"), "{pointer}: {err}");
    }
}

#[test]
fn duplicate_key_is_refused() {
    let json = serde_json::to_string(&valid())
        .unwrap()
        .replacen('{', r#"{"project_name":"q","#, 1);
    let err = serde_json::from_str::<Specification>(&json).unwrap_err();
    assert!(err.to_string().contains("duplicate field"), "{err}");
}

// ---- SolvedTopology -------------------------------------------------------

const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn assignment(pin: &str, af: u8) -> PinAssignment {
    PinAssignment {
        pin: pin.into(),
        peripheral: "I2C1".into(),
        signal: "I2C1_SCL".into(),
        alternate_function: af,
    }
}

fn solved() -> SolvedTopology {
    SolvedTopology {
        schema_version: 1,
        specification_sha256: HASH.into(),
        assignments: [
            ("I2C_SCL".to_owned(), assignment("PB8", 4)),
            ("I2C_SDA".to_owned(), assignment("PB9", 4)),
        ]
        .into(),
        gpio: [("FLASH_CS".to_owned(), "PA0".to_owned())].into(),
        i2c_ceilings: [("SENSORS".to_owned(), 400_000)].into(),
        analog_lines: [
            ("I2C_SCL".to_owned(), analog_line()),
            ("I2C_SDA".to_owned(), analog_line()),
        ]
        .into(),
        // `U10` sorts before `U3`, so the map's order is not the index order.
        i2c_devices: [
            ("U3".to_owned(), device(0, 0x48)),
            ("U10".to_owned(), device(1, 0x76)),
        ]
        .into(),
    }
}

fn device(index: u32, address: u8) -> SolvedI2cDevice {
    SolvedI2cDevice {
        index,
        part: "BME280".into(),
        bus: "SENSORS".into(),
        peripheral: "I2C1".into(),
        address,
    }
}

fn analog_line() -> AnalogLine {
    AnalogLine {
        bus: "SENSORS".into(),
        pull_up_ohms: 2200,
        capacitance_femtofarads: 100_000,
        ceiling_ns: 300,
        domain_millivolts: 3300,
    }
}

fn solved_refusal(mutate: impl FnOnce(&mut SolvedTopology)) -> E {
    let mut topo = solved();
    mutate(&mut topo);
    topo.validate()
        .expect_err("mutated topology must be refused")
}

#[test]
fn valid_solved_topology_passes() {
    assert_eq!(solved().validate(), Ok(()));
}

#[test]
fn solved_refuses_version_and_hash() {
    let err = solved_refusal(|t| t.schema_version = 2);
    assert_refusal(err, "UnsupportedSchemaVersion", None);
    for hash in [
        "",
        &HASH[..63],
        &format!("{HASH}0"),
        &HASH.to_uppercase(),
        &format!("g{}", &HASH[1..]),
        &format!("{}\u{e9}", &HASH[..62]),
    ] {
        let err = solved_refusal(|t| t.specification_sha256 = hash.into());
        assert_refusal(err, "MalformedSpecificationHash", None);
    }
}

#[test]
fn solved_refuses_non_identifiers_with_paths() {
    let err = solved_refusal(|t| {
        let a = t.assignments.remove("I2C_SDA").unwrap();
        t.assignments.insert("I2C SDA".into(), a);
    });
    assert_invalid_identifier(err, "assignments.I2C SDA");
    let err = solved_refusal(|t| t.assignments.get_mut("I2C_SDA").unwrap().pin = "".into());
    assert_invalid_identifier(err, "assignments.I2C_SDA.pin");
    let err = solved_refusal(|t| t.assignments.get_mut("I2C_SDA").unwrap().peripheral = " ".into());
    assert_invalid_identifier(err, "assignments.I2C_SDA.peripheral");
    let err = solved_refusal(|t| {
        t.assignments.get_mut("I2C_SCL").unwrap().signal = "\u{0412}".into();
    });
    assert_invalid_identifier(err, "assignments.I2C_SCL.signal");
}

#[test]
fn solved_refuses_alternate_function_above_15_and_duplicate_pin() {
    let err = solved_refusal(|t| t.assignments.get_mut("I2C_SDA").unwrap().alternate_function = 16);
    assert_eq!(
        err,
        E::AlternateFunctionOutOfRange {
            id: "I2C_SDA".into(),
            value: 16
        }
    );
    assert_eq!(err.id(), Some("I2C_SDA"));
    let mut ok = solved();
    ok.assignments
        .get_mut("I2C_SDA")
        .unwrap()
        .alternate_function = 15;
    assert_eq!(ok.validate(), Ok(()));
    let err = solved_refusal(|t| t.assignments.get_mut("I2C_SDA").unwrap().pin = "PB8".into());
    assert_refusal(err, "DuplicatePin", Some("PB8"));
}

#[test]
fn solved_round_trip_is_equal_and_byte_stable() {
    let topo = solved();
    let json = serde_json::to_string(&topo).unwrap();
    let back: SolvedTopology = serde_json::from_str(&json).unwrap();
    assert_eq!(back, topo);
    assert_eq!(serde_json::to_string(&back).unwrap(), json);
}

#[test]
fn solved_serialization_is_sorted_whatever_the_input_order() {
    let a = r#"{"schema_version":1,"specification_sha256":"h","assignments":{"B":{"pin":"p","peripheral":"q","signal":"s","alternate_function":1},"A":{"pin":"r","peripheral":"q","signal":"s","alternate_function":1}},"gpio":{"Z":"g","C":"c"},"i2c_ceilings":{"Y":100000,"B":400000},"analog_lines":{},"i2c_devices":{}}"#;
    let b = r#"{"schema_version":1,"specification_sha256":"h","assignments":{"A":{"pin":"r","peripheral":"q","signal":"s","alternate_function":1},"B":{"pin":"p","peripheral":"q","signal":"s","alternate_function":1}},"gpio":{"C":"c","Z":"g"},"i2c_ceilings":{"B":400000,"Y":100000},"analog_lines":{},"i2c_devices":{}}"#;
    let a: SolvedTopology = serde_json::from_str(a).unwrap();
    let b: SolvedTopology = serde_json::from_str(b).unwrap();
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        serde_json::to_string(&b).unwrap()
    );
}

fn solved_json() -> serde_json::Value {
    serde_json::to_value(solved()).unwrap()
}

fn parse_solved(value: &serde_json::Value) -> Result<SolvedTopology, String> {
    serde_json::from_str::<SolvedTopology>(&value.to_string()).map_err(|e| e.to_string())
}

#[test]
fn solved_serde_edge_cases_are_refused() {
    for pointer in ["", "/assignments/I2C_SDA"] {
        let mut doc = solved_json();
        doc.pointer_mut(pointer).unwrap()["extra"] = 1.into();
        assert!(parse_solved(&doc).unwrap_err().contains("unknown field"));

        let mut doc = solved_json();
        *doc.pointer_mut(pointer).unwrap() = serde_json::json!([]);
        assert!(parse_solved(&doc).unwrap_err().contains("object"));

        let mut doc = solved_json();
        let object = doc.pointer_mut(pointer).unwrap().as_object_mut().unwrap();
        let key = object.keys().next().unwrap().clone();
        object.remove(&key);
        assert!(parse_solved(&doc).unwrap_err().contains("missing field"));
    }
    let mut doc = solved_json();
    *doc.pointer_mut("/assignments").unwrap() = serde_json::json!([]);
    assert!(parse_solved(&doc).is_err());
}

#[test]
fn solved_repeated_assignment_key_is_refused() {
    let json = r#"{"schema_version":1,"specification_sha256":"h","assignments":{"N":{"pin":"p","peripheral":"q","signal":"s","alternate_function":1},"N":{"pin":"x","peripheral":"q","signal":"s","alternate_function":1}},"gpio":{},"i2c_ceilings":{}}"#;
    let err = serde_json::from_str::<SolvedTopology>(json).unwrap_err();
    assert!(
        err.to_string().contains("duplicate assignment for net `N`"),
        "{err}"
    );
}

#[test]
fn a_solved_field_in_a_specification_and_the_reverse_are_refused() {
    let mut doc = spec_json();
    doc["assignments"] = serde_json::json!({});
    assert!(
        parse(&doc)
            .unwrap_err()
            .contains("unknown field `assignments`")
    );
    let mut doc = spec_json();
    doc["i2c_buses"][0]["sda"]["alternate_function"] = 4.into();
    assert!(
        parse(&doc)
            .unwrap_err()
            .contains("unknown field `alternate_function`")
    );

    let mut doc = solved_json();
    doc["project_name"] = "x".into();
    assert!(
        parse_solved(&doc)
            .unwrap_err()
            .contains("unknown field `project_name`")
    );
    assert!(serde_json::from_value::<Specification>(solved_json()).is_err());
    assert!(serde_json::from_value::<SolvedTopology>(spec_json()).is_err());
}

// ---- SolvedTopology.gpio (AET-WO-1200 S4) ----------------------------------

#[test]
fn solved_refuses_gpio_non_identifiers_with_paths() {
    let err = solved_refusal(|t| {
        t.gpio.insert("CS 2".into(), "PA1".into());
    });
    assert_invalid_identifier(err, "gpio.CS 2");
    let err = solved_refusal(|t| {
        t.gpio.insert("FLASH_CS".into(), "".into());
    });
    assert_invalid_identifier(err, "gpio.FLASH_CS.pin");
    // assignments are checked before gpio
    let err = solved_refusal(|t| {
        t.gpio.insert("CS 2".into(), "PA1".into());
        t.assignments.get_mut("I2C_SDA").unwrap().pin = "".into();
    });
    assert_invalid_identifier(err, "assignments.I2C_SDA.pin");
}

#[test]
fn solved_refuses_a_net_in_both_maps() {
    let err = solved_refusal(|t| {
        t.gpio.insert("I2C_SDA".into(), "PA1".into());
    });
    assert_refusal(err, "NetAssignedTwice", Some("I2C_SDA"));
    // before a duplicate pin
    let err = solved_refusal(|t| {
        t.gpio.insert("I2C_SDA".into(), "PB8".into());
    });
    assert_refusal(err, "NetAssignedTwice", Some("I2C_SDA"));
}

#[test]
fn solved_refuses_a_pin_used_twice_across_or_within_gpio() {
    let err = solved_refusal(|t| {
        t.gpio.insert("FLASH_CS".into(), "PB9".into());
    });
    assert_refusal(err, "DuplicatePin", Some("PB9"));
    let err = solved_refusal(|t| {
        t.gpio.insert("OTHER_CS".into(), "PA0".into());
    });
    assert_refusal(err, "DuplicatePin", Some("PA0"));
}

#[test]
fn solved_refuses_a_repeated_or_missing_gpio() {
    let json = r#"{"schema_version":1,"specification_sha256":"h","assignments":{},"gpio":{"N":"p","N":"q"},"i2c_ceilings":{}}"#;
    let err = serde_json::from_str::<SolvedTopology>(json).unwrap_err();
    assert!(
        err.to_string().contains("duplicate assignment for net `N`"),
        "{err}"
    );
    let json =
        r#"{"schema_version":1,"specification_sha256":"h","assignments":{},"i2c_ceilings":{}}"#;
    let err = serde_json::from_str::<SolvedTopology>(json).unwrap_err();
    assert!(err.to_string().contains("missing field `gpio`"), "{err}");
    let json = r#"{"schema_version":1,"specification_sha256":"h","assignments":{},"gpio":{"N":4},"i2c_ceilings":{}}"#;
    assert!(serde_json::from_str::<SolvedTopology>(json).is_err());
}

// ---- SolvedTopology.i2c_ceilings (AET-WO-1202) ------------------------------

#[test]
fn solved_accepts_each_mode_and_refuses_any_other_ceiling() {
    for hz in [100_000, 400_000, 1_000_000] {
        let mut t = solved();
        t.i2c_ceilings.insert("SENSORS".into(), hz);
        assert_eq!(t.validate(), Ok(()), "{hz}");
    }
    for hz in [
        0,
        99_999,
        100_001,
        399_999,
        400_001,
        999_999,
        1_000_001,
        3_400_000,
        u32::MAX,
    ] {
        let err = solved_refusal(|t| {
            t.i2c_ceilings.insert("SENSORS".into(), hz);
        });
        assert_refusal(err, "InvalidCeiling", Some("SENSORS"));
    }
}

#[test]
fn solved_refuses_a_ceiling_key_that_is_not_an_identifier() {
    let err = solved_refusal(|t| {
        t.i2c_ceilings.insert("BUS 2".into(), 400_000);
    });
    assert_invalid_identifier(err, "i2c_ceilings.BUS 2");
    // gpio is checked before the ceilings
    let err = solved_refusal(|t| {
        t.i2c_ceilings.insert("BUS 2".into(), 400_000);
        t.gpio.insert("CS 2".into(), "PA1".into());
    });
    assert_invalid_identifier(err, "gpio.CS 2");
}

#[test]
fn solved_refuses_a_repeated_or_missing_ceilings_map() {
    let json = r#"{"schema_version":1,"specification_sha256":"h","assignments":{},"gpio":{},"i2c_ceilings":{"B":400000,"B":100000}}"#;
    let err = serde_json::from_str::<SolvedTopology>(json).unwrap_err();
    assert!(
        err.to_string().contains("duplicate assignment for net `B`"),
        "{err}"
    );
    let json = r#"{"schema_version":1,"specification_sha256":"h","assignments":{},"gpio":{}}"#;
    let err = serde_json::from_str::<SolvedTopology>(json).unwrap_err();
    assert!(
        err.to_string().contains("missing field `i2c_ceilings`"),
        "{err}"
    );
    let json = r#"{"schema_version":1,"specification_sha256":"h","assignments":{},"gpio":{},"i2c_ceilings":{"B":-1}}"#;
    assert!(serde_json::from_str::<SolvedTopology>(json).is_err());
}

#[test]
fn an_invalid_ceiling_is_refused_after_alternate_functions_and_before_duplicates() {
    let err = solved_refusal(|t| {
        t.i2c_ceilings.insert("SENSORS".into(), 1);
        t.assignments.get_mut("I2C_SDA").unwrap().alternate_function = 16;
    });
    assert_eq!(err.kind(), "AlternateFunctionOutOfRange");
    let err = solved_refusal(|t| {
        t.i2c_ceilings.insert("SENSORS".into(), 1);
        t.gpio.insert("FLASH_CS".into(), "PB9".into());
    });
    assert_eq!(err.kind(), "InvalidCeiling");
}

// ---- SolvedTopology.analog_lines (AET-WO-2200 §1.1) -------------------------

#[test]
fn solved_refuses_a_zero_analog_value() {
    type Edit = fn(&mut AnalogLine);
    let zero: [(&str, Edit); 4] = [
        ("ZeroPullUp", |l| l.pull_up_ohms = 0),
        ("ZeroCapacitance", |l| l.capacitance_femtofarads = 0),
        ("ZeroRiseTime", |l| l.ceiling_ns = 0),
        ("ZeroNominalVoltage", |l| l.domain_millivolts = 0),
    ];
    for (kind, edit) in zero {
        let err = solved_refusal(|t| edit(t.analog_lines.get_mut("I2C_SDA").unwrap()));
        assert_refusal(err, kind, Some("I2C_SDA"));
    }
    // Huge values are the bridge's to measure or refuse, not validate's.
    let mut t = solved();
    let line = t.analog_lines.get_mut("I2C_SDA").unwrap();
    line.pull_up_ohms = u32::MAX;
    line.capacitance_femtofarads = u64::MAX;
    line.ceiling_ns = u32::MAX;
    line.domain_millivolts = u32::MAX;
    assert_eq!(t.validate(), Ok(()));
}

#[test]
fn solved_refuses_an_analog_key_that_is_no_solved_i2c_line() {
    // Not assigned at all, a GPIO net, and an assignment that is not an I2C line.
    let err = solved_refusal(|t| {
        t.analog_lines.insert("NOWHERE".into(), analog_line());
    });
    assert_refusal(err, "AnalogLineNotI2c", Some("NOWHERE"));
    let err = solved_refusal(|t| {
        t.analog_lines.insert("FLASH_CS".into(), analog_line());
    });
    assert_refusal(err, "AnalogLineNotI2c", Some("FLASH_CS"));
    for (peripheral, signal) in [
        ("SPI1", "SPI1_SCK"),
        ("I2C1", "I2C1_SMBA"),
        ("I2C", "I2C_SDA"),
        ("I2C1", "I2C2_SDA"),
    ] {
        let err = solved_refusal(|t| {
            let a = t.assignments.get_mut("I2C_SDA").unwrap();
            a.peripheral = peripheral.into();
            a.signal = signal.into();
        });
        assert_refusal(err, "AnalogLineNotI2c", Some("I2C_SDA"));
    }
}

#[test]
fn solved_refuses_a_missing_analog_line_or_a_bus_without_a_ceiling() {
    let err = solved_refusal(|t| {
        t.analog_lines.remove("I2C_SCL");
    });
    assert_refusal(err, "MissingAnalogLine", Some("I2C_SCL"));
    let err = solved_refusal(|t| {
        t.analog_lines.get_mut("I2C_SCL").unwrap().bus = "OTHER".into();
    });
    assert_refusal(err, "UnknownBus", Some("OTHER"));
    let err = solved_refusal(|t| {
        t.i2c_ceilings.insert("OTHER".into(), 400_000);
        t.analog_lines.get_mut("I2C_SCL").unwrap().bus = "OTHER".into();
    });
    assert_refusal(err, "AnalogBusMismatch", Some("I2C_SDA"));
    // Adversarial A2: two peripherals' lines under one bus name.
    let err = solved_refusal(|t| {
        for (net, pin, signal) in [("B_SCL", "PB10", "I2C2_SCL"), ("B_SDA", "PB11", "I2C2_SDA")] {
            let mut a = t.assignments["I2C_SCL"].clone();
            a.pin = pin.into();
            a.peripheral = "I2C2".into();
            a.signal = signal.into();
            t.assignments.insert(net.into(), a);
            t.analog_lines.insert(net.into(), analog_line());
        }
    });
    // `B_SCL` sorts first and claims the bus for I2C2.
    assert_refusal(err, "AnalogBusMismatch", Some("I2C_SCL"));
    let err = solved_refusal(|t| {
        t.analog_lines.get_mut("I2C_SCL").unwrap().bus = "BUS 2".into();
    });
    assert_invalid_identifier(err, "analog_lines.I2C_SCL.bus");
}

#[test]
fn solved_refuses_a_repeated_missing_or_malformed_analog_map() {
    let mut doc = solved_json();
    doc.as_object_mut().unwrap().remove("analog_lines");
    assert!(
        parse_solved(&doc)
            .unwrap_err()
            .contains("missing field `analog_lines`")
    );
    for field in [
        "bus",
        "pull_up_ohms",
        "capacitance_femtofarads",
        "ceiling_ns",
        "domain_millivolts",
    ] {
        let mut doc = solved_json();
        doc["analog_lines"]["I2C_SDA"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        let err = parse_solved(&doc).unwrap_err();
        assert!(err.contains(&format!("missing field `{field}`")), "{err}");
    }
    for bad in [
        serde_json::json!(-1),
        serde_json::json!(1.5),
        serde_json::json!("2200"),
        serde_json::json!(null),
    ] {
        let mut doc = solved_json();
        doc["analog_lines"]["I2C_SDA"]["pull_up_ohms"] = bad;
        assert!(parse_solved(&doc).is_err());
    }
    let mut doc = solved_json();
    doc["analog_lines"]["I2C_SDA"]["extra"] = 1.into();
    assert!(parse_solved(&doc).unwrap_err().contains("unknown field"));
    let text = solved_json().to_string();
    let line = r#""I2C_SDA":{"bus":"SENSORS","capacitance_femtofarads":100000,"ceiling_ns":300,"domain_millivolts":3300,"pull_up_ohms":2200}"#;
    assert!(text.contains(line), "{text}");
    let twice = text.replace(line, &format!("{line},{line}"));
    let err = serde_json::from_str::<SolvedTopology>(&twice).unwrap_err();
    assert!(
        err.to_string()
            .contains("duplicate assignment for net `I2C_SDA`"),
        "{err}"
    );
}

// ---- SolvedTopology.i2c_devices (AET-WO-2203 §1.1) ---------------------------

#[test]
fn devices_come_out_in_index_order_not_reference_order() {
    let t = solved();
    let refs: Vec<&str> = t.i2c_devices.keys().map(String::as_str).collect();
    assert_eq!(refs, ["U10", "U3"]);
    let order: Vec<&str> = t.i2c_devices_in_order().iter().map(|d| d.0).collect();
    assert_eq!(order, ["U3", "U10"]);
}

#[test]
fn solved_refuses_a_device_address_outside_the_range() {
    for address in [0x00, 0x07, 0x78, 0x80, 0xFF] {
        let err = solved_refusal(|t| t.i2c_devices.get_mut("U3").unwrap().address = address);
        assert_eq!(
            err,
            E::I2cAddressOutOfRange {
                id: "U3".into(),
                value: address
            }
        );
    }
}

#[test]
fn solved_refuses_a_device_on_an_unknown_bus_or_the_wrong_peripheral() {
    let err = solved_refusal(|t| t.i2c_devices.get_mut("U3").unwrap().bus = "NOPE".into());
    assert_refusal(err, "UnknownBus", Some("NOPE"));
    let err = solved_refusal(|t| t.i2c_devices.get_mut("U3").unwrap().peripheral = "I2C2".into());
    assert_refusal(err, "I2cDevicePeripheral", Some("U3"));
}

#[test]
fn solved_refuses_a_key_or_field_that_is_not_an_identifier() {
    let err = solved_refusal(|t| {
        let d = t.i2c_devices.remove("U3").unwrap();
        t.i2c_devices.insert("U 3".into(), d);
    });
    assert_invalid_identifier(err, "i2c_devices.U 3");
    let err = solved_refusal(|t| t.i2c_devices.get_mut("U3").unwrap().part = "".into());
    assert_invalid_identifier(err, "i2c_devices.U3.part");
}

#[test]
fn solved_refuses_indices_that_are_not_exactly_zero_to_n() {
    // Repeated, out of range, and a gap are each ambiguous about the order.
    for (a, b) in [(0, 0), (1, 1), (0, 2), (1, 2), (5, 6)] {
        let err = solved_refusal(|t| {
            t.i2c_devices.get_mut("U3").unwrap().index = a;
            t.i2c_devices.get_mut("U10").unwrap().index = b;
        });
        assert_eq!(err.kind(), "I2cDeviceIndex", "{a} {b}");
    }
}

#[test]
fn solved_refuses_a_missing_or_repeated_device() {
    let mut doc = solved_json();
    doc.as_object_mut().unwrap().remove("i2c_devices");
    assert!(
        parse_solved(&doc)
            .unwrap_err()
            .contains("missing field `i2c_devices`")
    );
    let mut doc = solved_json();
    doc["i2c_devices"]["U3"]["extra"] = 1.into();
    assert!(parse_solved(&doc).unwrap_err().contains("unknown field"));
    let mut doc = solved_json();
    doc["i2c_devices"]["U3"]
        .as_object_mut()
        .unwrap()
        .remove("index");
    assert!(
        parse_solved(&doc)
            .unwrap_err()
            .contains("missing field `index`")
    );
    let text = solved_json().to_string();
    let entry =
        r#""U3":{"address":72,"bus":"SENSORS","index":0,"part":"BME280","peripheral":"I2C1"}"#;
    assert!(text.contains(entry), "{text}");
    let twice = text.replace(entry, &format!("{entry},{entry}"));
    let err = serde_json::from_str::<SolvedTopology>(&twice).unwrap_err();
    assert!(
        err.to_string()
            .contains("duplicate assignment for net `U3`"),
        "{err}"
    );
}

#[test]
fn refuses_names_that_differ_only_in_case() {
    // The review's A10.1 inputs: `emit` accepted both and `check erc` refused them.
    let err = refusal(|s| s.spi_devices[0].cs.net = "gnd".into());
    assert_refusal(err, "NetShadowsDomain", Some("gnd"));
    let err = refusal(|s| s.spi_devices[0].cs.net = "i2c_scl".into());
    assert_refusal(err, "DuplicateNetName", Some("i2c_scl"));
    let err = refusal(|s| s.spi_devices[0].cs.net = "vdd_3v3".into());
    assert_refusal(err, "NetShadowsDomain", Some("vdd_3v3"));
}
