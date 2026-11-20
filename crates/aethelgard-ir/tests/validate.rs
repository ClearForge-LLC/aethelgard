mod common;

use common::devs;

use aethelgard_ir::{
    AnalogLine, I2cBus, I2cDevice, IrValidationError as E, Mcu, NetConstraint, PinAssignment,
    Signal, SolvedClock, SolvedI2cDevice, SolvedI2cTiming, SolvedSpiController, SolvedSpiDevice,
    SolvedTopology, Specification, SpiBus, SpiDevice, SpiKernel, SpiLine,
    UnwitnessedAcknowledgement, UnwitnessedClass, VoltageDomain,
};
use std::collections::BTreeMap;

/// The reference specification's declared board (`examples/sentinel-01-minimal.json`).
const BOARD: &str = r#"{"copper_layers": 2, "layers": [{"name": "F.Cu", "role": "signal", "net": null}, {"name": "B.Cu", "role": "signal", "net": null}], "stackup": {"finished_thickness_um": {"nominal": 1600, "min": 340, "max": 2000, "source": "AET-WO-1305 conditions: 1.6 mm declared; 0.34 mm = 2 x 70 um + 0.2 mm; 2.0 mm = 1.6 mm + 25 %"}, "dielectrics": [{"between": ["F.Cu", "B.Cu"], "min_thickness_um": 200, "source": "implied: the finished minimum 340 um less twice the copper maximum 70 um"}], "copper_max_um": {"value": 70, "source": "AET-WO-1305 condition: twice KiCad's default 35 um (2 oz)"}, "relative_permittivity_max_permille": {"value": 5400, "source": "the IPC-4101 FR-4 maximum, specified at 1 MHz"}}, "rules": {"clearance_um": 200, "track_width_um": 200, "via_drill_um": 300, "via_diameter_um": 600}}"#;

fn sig(net: &str) -> Signal {
    Signal {
        net: net.into(),
        pin: None,
    }
}

fn valid() -> Specification {
    // The device lists are private to the crate (`AET-WO-1204`), so the
    // fixture is built from the JSON the type reads.
    let json = serde_json::json!({
        "schema_version": aethelgard_ir::SPECIFICATION_SCHEMA_VERSION,
        // U4, the flash, is `nrnd` by its maker (`AET-WO-4211`).
        "accept_lifecycle": [{
            "part": "W25Q64JVxIQ",
            "status": "nrnd",
            "statement": "NRND by its maker; kept until AET-WO-4500 picks a flash (AET-WO-4211)"
        }],
        "acknowledge_unwitnessed": [],
        "net_classes": [],
        "diff_pairs": [],
        "power": null,
        "project_name": "sentinel-01",
        "board": serde_json::from_str::<serde_json::Value>(BOARD).unwrap(),
        "mcu": Mcu { part: "STM32H753ZITx".into(), supply: "VDD_3V3".into(), hse_hz: 0, bor_level: 0, supply_source: Some(aethelgard_ir::SupplySource { connector: "J2".into(), pin: "1".into() }), reset_switch: false, foreign_clock: None },
        "voltage_domains": [VoltageDomain {
            id: "VDD_3V3".into(),
            nominal_millivolts: 3300,
            tolerance_permille: 50,
        }],
        "i2c_buses": [I2cBus {
            id: "SENSORS".into(),
            peripheral: None,
            speed_hz: 400_000,
            pull_up_ohms: 2200,
            domain: "VDD_3V3".into(),
            sda: sig("I2C_SDA"),
            scl: sig("I2C_SCL"),
        }],
        "spi_buses": [SpiBus {
            id: "FLASH".into(),
            peripheral: None,
            speed_hz: 20_000_000,
            domain: "VDD_3V3".into(),
            sck: sig("SPI_SCK"),
            mosi: sig("SPI_MOSI"),
            miso: sig("SPI_MISO"),
        }],
        "i2c_devices": [
            I2cDevice {
                reference: "U2".into(),
                part: "BME280".into(),
                supply: "VDD_3V3".into(),
                bus: "SENSORS".into(),
                address: 118,
                pin_capacitance: None,
            },
            I2cDevice {
                reference: "U3".into(),
                part: "TMP117".into(),
                supply: "VDD_3V3".into(),
                bus: "SENSORS".into(),
                address: 72,
                pin_capacitance: None,
            },
        ],
        "spi_devices": [SpiDevice {
            reference: "U4".into(),
            part: "W25Q64JVxIQ".into(),
            supply: "VDD_3V3".into(),
            bus: "FLASH".into(),
            cs: sig("FLASH_CS"),
            pin_capacitance: None,
            orientation: None,
        }],
        "connectors": [
            {"reference": "J1", "part": "SM06B-SRSS-TB", "standard": null, "pins": [
                {"pin": "1", "net": "VDD_3V3", "offboard": null},
                {"pin": "2", "net": null, "offboard": null},
                {"pin": "3", "net": null, "offboard": null},
                {"pin": "4", "net": "SWDIO", "offboard": null},
                {"pin": "5", "net": "SWCLK", "offboard": null},
                {"pin": "6", "net": "GND", "offboard": null}]},
            {"reference": "J2", "part": "SM04B-GHS-TB", "standard": null, "pins": [
                {"pin": "1", "net": "VDD_3V3", "offboard": null},
                {"pin": "2", "net": null, "offboard": null},
                {"pin": "3", "net": null, "offboard": null},
                {"pin": "4", "net": "GND", "offboard": null}]}
        ],
        "net_constraints": [NetConstraint {
            net_name: "I2C_SDA".into(),
            max_capacitance_femtofarads: 400_000,
            max_rise_time_nanoseconds: 300,
            impedance_ohms: None,
        }, NetConstraint {
            net_name: "SPI_SCK".into(),
            max_capacitance_femtofarads: 30_000,
            max_rise_time_nanoseconds: 20,
            impedance_ohms: None,
        }, NetConstraint {
            net_name: "SPI_MOSI".into(),
            max_capacitance_femtofarads: 30_000,
            max_rise_time_nanoseconds: 20,
            impedance_ohms: None,
        }, NetConstraint {
            // Last, so no index above moves: `validate` requires both I2C
            // lines' constraints (step 19, `AET-WO-1206` §1.2).
            net_name: "I2C_SCL".into(),
            max_capacitance_femtofarads: 400_000,
            max_rise_time_nanoseconds: 300,
            impedance_ohms: None,
        }],
        "motor_outputs": [],
        "serial_ports": [],
    });
    serde_json::from_value(json).expect("the fixture is a specification")
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
    devs(&mut spec).i2c.clear();
    devs(&mut spec).spi.clear();
    // The flash is gone, so its acknowledgement would stand for no part.
    spec.accept_lifecycle.clear();
    spec.net_constraints.clear();
    assert_eq!(spec.validate(), Ok(()));
}

#[test]
fn refuses_unsupported_schema_version() {
    for version in [
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        10,
        11,
        12,
        13,
        14,
        15,
        16,
        17,
        19,
        u32::MAX,
    ] {
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
            devs(s).i2c[1].reference = "".into()
        }),
        ("i2c_devices[0].part", |s| devs(s).i2c[0].part = "".into()),
        ("i2c_devices[0].supply", |s| {
            devs(s).i2c[0].supply = "".into()
        }),
        ("i2c_devices[0].bus", |s| devs(s).i2c[0].bus = "".into()),
        ("spi_devices[0].reference", |s| {
            devs(s).spi[0].reference = "".into()
        }),
        ("spi_devices[0].part", |s| devs(s).spi[0].part = "".into()),
        ("spi_devices[0].supply", |s| {
            devs(s).spi[0].supply = "".into()
        }),
        ("spi_devices[0].bus", |s| devs(s).spi[0].bus = "".into()),
        ("spi_devices[0].cs.net", |s| {
            devs(s).spi[0].cs.net = "".into()
        }),
        ("spi_devices[0].cs.pin", |s| {
            devs(s).spi[0].cs.pin = Some("".into())
        }),
        ("net_constraints[0].net_name", |s| {
            s.net_constraints[0].net_name = "".into()
        }),
    ];
    for (path, mutate) in cases {
        assert_invalid_identifier(refusal(mutate), path);
    }
}

/// The base set, on a field of kind `Plain` (`mcu.part`): everything a base
/// identifier must refuse, one value each, and what it must admit.
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
        // KiCad does not escape it in a placement row (AET-WO-3300 F15).
        "U\"4,X",
        "\"",
        // KiCad rewrites these four in a Gerber net attribute (AET-WO-1102, CI).
        "A%B",
        "A*B",
        "A,B",
        "A\\B",
        // A date or a time of day, anywhere inside (AET-WO-3300 F14).
        "2026-11-13",
        "N2026-11-13X",
        "12:34:56",
        "AB12:34:56CD",
    ];
    for id in bad {
        let err = refusal(|s| s.mcu.part = id.into());
        assert_invalid_identifier(err, "mcu.part");
    }
    let long = "a".repeat(65);
    assert_invalid_identifier(refusal(|s| s.mcu.part = long), "mcu.part");
    for ok in [
        "a".repeat(64),
        "!".into(),
        "~".into(),
        "+3V3".into(),
        "Net-(U1-Pad3)".into(),
        // Every other printable character, S-expression specials included,
        // but `"`.
        "!#&'()+-.:;<=>?@^_`|~".into(),
        // Not shaped like a date or a time: too few digits, or a break.
        "2026-11-1".into(),
        "202-11-13".into(),
        "12:34:5".into(),
        "12:3456".into(),
        "1234-56_78".into(),
    ] {
        let mut spec = valid();
        spec.mcu.part = ok;
        assert_eq!(spec.validate(), Ok(()));
    }
}

/// `AET-WO-1102`: the project name is a crate name, and a device reference
/// is a Renode identifier, each refused at `validate` as `generate` refuses
/// it; the message names the rule broken.
#[test]
fn project_names_and_references_have_their_own_shapes() {
    for id in [
        "Sentinel",
        "sentinel_01",
        "1sentinel",
        "-sentinel",
        "sentinel-",
        "a--b",
        "a.b",
        "a+b",
        ".",
        "~",
    ] {
        let err = refusal(|s| s.project_name = id.into());
        assert_invalid_identifier(err.clone(), "project_name");
        assert!(err.to_string().contains("crate name"), "{err}");
    }
    // `generate` names the board crate `<project>-board`: 59 bytes is refused there.
    let long = refusal(|s| s.project_name = "a".repeat(59));
    assert_invalid_identifier(long, "project_name");
    for ok in ["sentinel-01", "a", "a1-b2", &"a".repeat(58)] {
        let mut spec = valid();
        spec.project_name = ok.into();
        assert_eq!(spec.validate(), Ok(()), "{ok}");
    }
    for id in [
        "U-2", "U.2", "2U", "_U", "U\"4", "U+2", "U 2", "#U2", "U:2", "U2,",
    ] {
        let err = refusal(|s| devs(s).i2c[0].reference = id.into());
        assert_invalid_identifier(err.clone(), "i2c_devices[0].reference");
        // The base set speaks first for a `"` or a space; the shape for the rest.
        let said = err.to_string();
        assert!(
            said.contains("Renode identifier") || said.contains("printable ASCII"),
            "{said}"
        );
        let err = refusal(|s| devs(s).spi[0].reference = id.into());
        assert_invalid_identifier(err, "spi_devices[0].reference");
    }
    for ok in ["U2", "u_2", "Ab_9", "S", "R_FLASH_0"] {
        let mut spec = valid();
        devs(&mut spec).i2c[0].reference = ok.into();
        assert_eq!(spec.validate(), Ok(()), "{ok}");
        let mut spec = valid();
        // The SPI device is `U4`; "U2" would be a duplicate of the I2C one.
        devs(&mut spec).spi[0].reference = format!("S{ok}");
        assert_eq!(spec.validate(), Ok(()), "{ok}");
    }
}

/// `AET-WO-2204` D5: a reference the emitter would lowercase onto a name the
/// Renode platform already holds is refused at `validate`, I2C and SPI alike.
#[test]
fn a_reference_that_lowercases_to_a_reserved_name_is_refused() {
    for id in [
        "nvic", "cpu", "crypto", "sysbus", "machine", "using", "init", "True", "NVIC", "Cpu",
        "I2C1", "SPI4",
    ] {
        let err = refusal(|s| devs(s).i2c[0].reference = id.into());
        assert_invalid_identifier(err.clone(), "i2c_devices[0].reference");
        assert!(err.to_string().contains("already holds"), "{err}");
        let err = refusal(|s| devs(s).spi[0].reference = id.into());
        assert_invalid_identifier(err.clone(), "spi_devices[0].reference");
        assert!(err.to_string().contains("already holds"), "{err}");
    }
    // Capitals that do not lowercase onto a stock name load: `gpioporta` is not `gpioPortA`.
    for ok in ["gpioPortA", "Nvic1", "U2", "usingx", "Machine1"] {
        let mut spec = valid();
        devs(&mut spec).i2c[0].reference = ok.into();
        assert_eq!(spec.validate(), Ok(()), "{ok}");
    }
}

#[test]
fn the_reserved_list_is_sorted_and_lowercase() {
    let list = aethelgard_ir::RESERVED_REFERENCES;
    assert!(list.windows(2).all(|w| w[0] < w[1]), "sorted, no repeats");
    assert!(list.iter().all(|n| {
        n.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    }));
}

#[test]
fn a_solved_topology_refuses_a_reserved_reference() {
    let err = solved_refusal(|t| {
        let device = t.i2c_devices.remove("U3").unwrap();
        t.i2c_devices.insert("NVIC".into(), device);
    });
    assert_invalid_identifier(err, "i2c_devices.NVIC");
}

#[test]
fn first_invalid_identifier_wins_in_key_order() {
    let err = refusal(|s| {
        s.net_constraints[0].net_name = "".into();
        devs(s).spi[0].cs.net = "".into();
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
        // At 500 permille the rail spans 1650 to 4950 mV, outside every
        // listed part's supply (`SupplyOutsidePart`, `AET-WO-4300c`) and
        // DS12117 Table 149's bands for an SPI bus on it
        // (`SupplyOutsideSpeedTable`, step 21, `AET-WO-1207`); only the
        // tolerance's own range is under test here, so the board has no
        // device (and no acknowledgement of one, `AET-WO-4211`) and no SPI
        // bus. At 500 permille the MCU's own range refuses the rail
        // (`DomainOutsideConsumer`, step 22, `AET-WO-1208` F1/F5), past the
        // tolerance's own check.
        let verdict = bare_rail(3300, value, false).validate();
        match value {
            1 => assert_eq!(verdict, Ok(())),
            _ => assert_eq!(verdict.unwrap_err().kind(), "DomainOutsideConsumer"),
        }
    }
}

#[test]
fn refuses_unknown_domain_at_each_use() {
    type Mutator = fn(&mut Specification);
    let cases: Vec<Mutator> = vec![
        |s| s.mcu.supply = "NOPE".into(),
        |s| s.i2c_buses[0].domain = "NOPE".into(),
        |s| s.spi_buses[0].domain = "NOPE".into(),
        |s| devs(s).i2c[0].supply = "NOPE".into(),
        |s| devs(s).spi[0].supply = "NOPE".into(),
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
        devs(s).i2c[1].supply = "VDD_1V8".into();
    });
    assert_refusal(err, "SupplyMismatch", Some("U3"));
    let err = refusal(|s| {
        with_second_domain(s);
        devs(s).spi[0].supply = "VDD_1V8".into();
    });
    assert_refusal(err, "SupplyMismatch", Some("U4"));
}

#[test]
fn a_device_with_an_unknown_bus_is_left_to_step_nine() {
    let err = refusal(|s| {
        with_second_domain(s);
        devs(s).i2c[0].bus = "NOPE".into();
        devs(s).i2c[0].supply = "VDD_1V8".into();
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
        // 1 MHz needs lines Fast-mode Plus allows (120 ns), or step 21
        // refuses it as `BusTooFast` (`AET-WO-1208` §1.3).
        rise_times(&mut spec, 120, 120);
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
    let err = refusal(|s| devs(s).spi[0].reference = "U2".into());
    assert_refusal(err, "DuplicateReference", Some("U2"));
    let err = refusal(|s| devs(s).i2c[1].reference = "U2".into());
    assert_refusal(err, "DuplicateReference", Some("U2"));
}

#[test]
fn references_that_differ_only_in_case_are_duplicates() {
    // KiCad's ERC refuses them (`AET-WO-1204`); the error names both spellings.
    let err = refusal(|s| devs(s).i2c[1].reference = "u2".into());
    assert_refusal(err.clone(), "DuplicateReference", Some("u2"));
    assert!(err.to_string().contains("`U2`"), "{err}");
    let err = refusal(|s| devs(s).spi[0].reference = "u3".into());
    assert_refusal(err, "DuplicateReference", Some("u3"));
}

#[test]
fn refuses_unknown_bus_and_a_bus_of_the_wrong_kind() {
    let err = refusal(|s| devs(s).i2c[0].bus = "NOPE".into());
    assert_refusal(err, "UnknownBus", Some("NOPE"));
    let err = refusal(|s| devs(s).i2c[0].bus = "FLASH".into());
    assert_refusal(err, "UnknownBus", Some("FLASH"));
    let err = refusal(|s| devs(s).spi[0].bus = "SENSORS".into());
    assert_refusal(err, "UnknownBus", Some("SENSORS"));
}

#[test]
fn refuses_i2c_address_out_of_range_and_duplicate_on_a_bus() {
    for value in [0, 7, 0x78, 255] {
        let err = refusal(|s| devs(s).i2c[0].address = value);
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
        // The range is the point: the address passes step 10, and the
        // refusal is step 14's, for a part with no row and so no straps
        // (`AET-WO-1206` §1.8), never `I2cAddressOutOfRange`.
        devs(&mut spec).i2c[0].part = "UNLISTED".into();
        devs(&mut spec).i2c[0].address = value;
        assert_eq!(spec.validate().unwrap_err().kind(), "UnknownPart");
    }
    let err = refusal(|s| devs(s).i2c[1].address = 118);
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
    for net in ["I2C2_SDA", "I2C2_SCL"] {
        let mut c = spec.net_constraints[0].clone();
        c.net_name = net.into();
        spec.net_constraints.push(c);
    }
    devs(&mut spec).i2c[1].bus = "OTHER".into();
    devs(&mut spec).i2c[1].address = 118;
    // A second BME280, strapped as the first, on the other bus.
    devs(&mut spec).i2c[1].part = "BME280".into();
    assert_eq!(spec.validate(), Ok(()));
}

#[test]
fn refuses_a_net_declared_twice_across_buses_and_cs() {
    let err = refusal(|s| s.i2c_buses[0].scl.net = "I2C_SDA".into());
    assert_refusal(err, "DuplicateNetName", Some("I2C_SDA"));
    let err = refusal(|s| devs(s).spi[0].cs.net = "SPI_SCK".into());
    assert_refusal(err, "DuplicateNetName", Some("SPI_SCK"));
    let err = refusal(|s| s.spi_buses[0].miso.net = "I2C_SCL".into());
    assert_refusal(err, "DuplicateNetName", Some("I2C_SCL"));
}

#[test]
fn refuses_a_signal_net_named_like_a_domain() {
    let err = refusal(|s| devs(s).spi[0].cs.net = "VDD_3V3".into());
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
    let mut rail = spec.net_constraints[0].clone();
    rail.net_name = "VDD_3V3".into();
    spec.net_constraints.push(rail);
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
        devs(s).i2c[0].supply = "NOPE".into();
    });
    assert_eq!(err.kind(), "UnknownDomain");
    // Step 9 before step 10.
    let err = refusal(|s| {
        devs(s).i2c[0].bus = "NOPE".into();
        devs(s).i2c[1].address = 0;
    });
    assert_eq!(err.kind(), "UnknownBus");
    // Step 11 before step 12.
    let err = refusal(|s| {
        devs(s).spi[0].cs.net = "SPI_SCK".into();
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
    devs(&mut spec).i2c.reverse();
    let json = serde_json::to_string(&spec).unwrap();
    let back: Specification = serde_json::from_str(&json).unwrap();
    assert_eq!(back.declared_i2c_devices()[0].reference, "U3");
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

/// A document with every section of the specification populated, and one
/// entry of each variant a section has: `sentinel-02-power.json` (power,
/// lifecycle, devices, board) plus the connectors, net classes, pair and
/// motor outputs and serial ports it lacks. A new section earns its place in the walk by being
/// added here; `the_full_document_is_valid_and_populates_every_section` fails
/// until it is.
fn full_document() -> serde_json::Value {
    use serde_json::{Value, json};
    let mut v: Value = serde_json::from_str(include_str!("fixtures/sentinel-02-power.json"))
        .expect("the power fixture");
    // Without the 1.8 V IMU and the crossing to it, which wait for `K6`
    // (the sensor table has no output delay for it yet): `power.crossings`
    // is the one empty list, named in `EMPTY_LISTS`. Its lifecycle
    // acknowledgement goes with it (`AET-WO-4210`); the flash's stays
    // (`AET-WO-4211`).
    v["power"]["crossings"] = json!([]);
    // The translator's and its CS pull-up's loads go with the crossing, and
    // J2's with J2, which the connectors below replace (`AET-WO-1209` D4): a
    // load naming a part the board lacks is refused.
    v["power"]["loads"]
        .as_array_mut()
        .unwrap()
        .retain(|l| !["U13", "R40", "J2"].iter().any(|r| l["reference"] == *r));
    v["accept_lifecycle"]
        .as_array_mut()
        .unwrap()
        .retain(|a| a["part"] != "ICM20948");
    for key in ["spi_devices", "spi_buses", "net_constraints"] {
        let list = v[key].as_array().unwrap().clone();
        v[key] = Value::Array(
            list.into_iter()
                .filter(|x| {
                    !(x["reference"] == "U5"
                        || x["id"] == "IMU"
                        || x["net_name"].as_str().is_some_and(|n| n.starts_with("IMU")))
                })
                .collect(),
        );
    }
    let c: Value = serde_json::from_str(include_str!(
        "../../aethelgard-cli/tests/fixtures/connectors.json"
    ))
    .expect("the connectors fixture");
    // The power fixture's J1 is the battery input; the two connectors are
    // moved to J5 and J6, J2's pins onto the first SPI bus's nets.
    let text = c["connectors"]
        .to_string()
        .replace("\"J1\"", "\"J5\"")
        .replace("\"J2\"", "\"J6\"")
        .replace("EXT_", "SPI_");
    v["connectors"] = serde_json::from_str(&text).unwrap();
    v["net_classes"] = json!([
        {"name": "power", "track_width_um": 250,
         "via": {"diameter_um": 700, "drill_um": 300}, "nets": ["VDD_3V3", "GND"]},
        {"name": "sig", "track_width_um": 250, "via": null, "nets": ["I2C_SDA"]}
    ]);
    v["diff_pairs"] = json!([
        {"name": "USB", "nets": ["SPI_SCK", "SPI_MOSI"], "gap_um": 300, "width_um": 200}
    ]);
    v["motor_outputs"] = json!([
        {"label": "M1", "pin": null, "protocol": "pwm",
         "pwm": {"rate_hz": 400, "pulse_min_us": 1000, "pulse_max_us": 2000}, "esc": null},
        {"label": "M2", "pin": null, "protocol": "dshot600", "pwm": null,
         "esc": {"none": {"source": "a bench header with no power pin"}}}
    ]);
    // `AET-WO-4300b`: a port left to the solver, and one with its peripheral
    // and pins named, parity, two stop bits and inversion.
    v["serial_ports"] = json!([
        {"label": "CONSOLE", "peripheral": null,
         "tx": {"net": "CONSOLE_TX", "pin": null}, "rx": {"net": "CONSOLE_RX", "pin": null},
         "role": "console",
         "framing": {"baud": 115200, "data_bits": 8, "parity": "none", "stop_bits": 1, "inverted": false},
         "negotiated": null, "peer_deviation_ppm": 5000, "peer_deviation_source": "declared"},
        {"label": "SBUS", "peripheral": "USART2",
         "tx": {"net": "SBUS_TX", "pin": "PD5"}, "rx": {"net": "SBUS_RX", "pin": "PD6"},
         "role": "rc_sbus",
         "framing": {"baud": 100000, "data_bits": 8, "parity": "even", "stop_bits": 2, "inverted": true},
         "negotiated": null, "peer_deviation_ppm": 5000, "peer_deviation_source": "declared"}
    ]);
    // Each serial line leaves the board at a connector.
    for (reference, port) in [("J7", "CONSOLE"), ("J8", "SBUS")] {
        v["connectors"].as_array_mut().unwrap().push(json!(
            {"reference": reference, "part": "SM04B-GHS-TB", "standard": null, "pins": [
                {"pin": "1", "net": format!("{port}_TX"), "offboard": null},
                {"pin": "2", "net": format!("{port}_RX"), "offboard": null},
                {"pin": "3", "net": null, "offboard": null},
                {"pin": "4", "net": "GND", "offboard": null}]}
        ));
    }
    // `AET-WO-4230` R1: an acknowledgement's shape is judged here; whether
    // the ledger has these fields is the CLI's `validate`.
    v["acknowledge_unwitnessed"] = json!([
        {"register": "SPI1.CFG2", "class": "ignored-by-model", "fields": ["CPHA", "CPOL"],
         "reason": "Renode stores neither (a tag): the bench reads them back"}
    ]);
    v
}

#[test]
fn the_full_document_is_valid_and_populates_every_section() {
    let doc = full_document();
    let spec: Specification = serde_json::from_value(doc.clone()).expect("parses");
    assert_eq!(spec.validate(), Ok(()));
    // Every key the type writes is in the document.
    let written = serde_json::to_value(&spec).unwrap();
    assert_eq!(
        written.as_object().unwrap().keys().collect::<Vec<_>>(),
        doc.as_object().unwrap().keys().collect::<Vec<_>>(),
        "the document and the type disagree on the sections"
    );
    // No list is empty and no section null (an empty list walks nothing),
    // but for the named ones.
    fn unpopulated(v: &serde_json::Value, at: &str, out: &mut Vec<String>) {
        match v {
            serde_json::Value::Null if at.matches('/').count() == 1 => out.push(at.to_owned()),
            serde_json::Value::Array(a) if a.is_empty() => out.push(at.to_owned()),
            serde_json::Value::Array(a) => {
                for (i, c) in a.iter().enumerate() {
                    unpopulated(c, &format!("{at}/{i}"), out);
                }
            }
            serde_json::Value::Object(o) => {
                for (k, c) in o {
                    unpopulated(c, &format!("{at}/{k}"), out);
                }
            }
            _ => {}
        }
    }
    let mut empty = Vec::new();
    unpopulated(&doc, "", &mut empty);
    let named: Vec<&str> = EMPTY_LISTS.iter().map(|(p, _)| *p).collect();
    assert_eq!(empty, named, "the lists the full document leaves empty");
}

/// The lists the full document cannot populate, and why. Each is covered
/// where it can be (`an_acknowledgement_s_keys_are_each_required`) or waits
/// for the work that makes it buildable.
const EMPTY_LISTS: &[(&str, &str)] = &[(
    "/power/crossings",
    "the 1.8 V IMU crossing waits for K6: the sensor table has no output delay \
         for ICM20948 (`OutputDelayMissing`)",
)];

#[test]
fn an_acknowledgement_s_keys_are_each_required() {
    use aethelgard_ir::LifecycleAcceptance;
    let whole = serde_json::json!({"part": "ICM20948", "status": "eol", "statement": "why"});
    assert!(serde_json::from_value::<LifecycleAcceptance>(whole.clone()).is_ok());
    for key in ["part", "status", "statement"] {
        let mut cut = whole.clone();
        cut.as_object_mut().unwrap().remove(key);
        let err = serde_json::from_value::<LifecycleAcceptance>(cut).unwrap_err();
        assert!(
            err.to_string().contains(&format!("missing field `{key}`")),
            "{err}"
        );
    }
}

/// Keys whose removal is legitimately accepted: JSON pointer to the object,
/// key, reason. None today: every key of every object is required, the
/// nullable ones included (`INV-06`). A key that becomes optional is listed
/// here with the reason, or the walk fails.
/// Why a load's `reference` may be absent (`AET-WO-1303b`).
const OPTIONAL_LOAD_REFERENCE: &str = "AET-WO-1303b: a load's part, optional; the supply gate needs it only on a domain with a drop limit";

/// Why the fabricator's tolerances may be absent (`AET-WO-1303b`).
const OPTIONAL_FAB_TOLERANCE: &str = "AET-WO-1303b: a fabricator's tolerance, optional; the supply gate refuses by name one it reads and the specification lacks";

/// Each key of the full document that may be absent: the fixture's VDD_3V3
/// loads (`power.loads[2..=12]`) carry the optional part `reference`, and
/// its stackup the fabricator's tolerances.
const OPTIONAL_KEYS: &[(&str, &str, &str)] = &[
    ("/board/stackup", "fab_tolerances", OPTIONAL_FAB_TOLERANCE),
    (
        "/board/stackup/fab_tolerances",
        "hole_position_um",
        OPTIONAL_FAB_TOLERANCE,
    ),
    (
        "/board/stackup/fab_tolerances",
        "hole_size_plus_um",
        OPTIONAL_FAB_TOLERANCE,
    ),
    (
        "/board/stackup/fab_tolerances",
        "hole_size_minus_um",
        OPTIONAL_FAB_TOLERANCE,
    ),
    (
        "/board/stackup/fab_tolerances",
        "track_width_permille",
        OPTIONAL_FAB_TOLERANCE,
    ),
    (
        "/board/stackup/fab_tolerances",
        "via_ring_um",
        OPTIONAL_FAB_TOLERANCE,
    ),
    ("/power/loads/2", "reference", OPTIONAL_LOAD_REFERENCE),
    ("/power/loads/3", "reference", OPTIONAL_LOAD_REFERENCE),
    ("/power/loads/4", "reference", OPTIONAL_LOAD_REFERENCE),
    ("/power/loads/5", "reference", OPTIONAL_LOAD_REFERENCE),
    ("/power/loads/6", "reference", OPTIONAL_LOAD_REFERENCE),
    ("/power/loads/7", "reference", OPTIONAL_LOAD_REFERENCE),
    ("/power/loads/8", "reference", OPTIONAL_LOAD_REFERENCE),
    ("/power/loads/9", "reference", OPTIONAL_LOAD_REFERENCE),
    ("/power/loads/10", "reference", OPTIONAL_LOAD_REFERENCE),
    ("/power/loads/11", "reference", OPTIONAL_LOAD_REFERENCE),
    ("/power/loads/12", "reference", OPTIONAL_LOAD_REFERENCE),
];

/// Every JSON object in `v`, as a pointer.
fn object_pointers(v: &serde_json::Value, at: &str, out: &mut Vec<String>) {
    match v {
        serde_json::Value::Object(o) => {
            out.push(at.to_owned());
            for (k, child) in o {
                let key = k.replace('~', "~0").replace('/', "~1");
                object_pointers(child, &format!("{at}/{key}"), out);
            }
        }
        serde_json::Value::Array(a) => {
            for (i, child) in a.iter().enumerate() {
                object_pointers(child, &format!("{at}/{i}"), out);
            }
        }
        _ => {}
    }
}

/// Removing any one key of any object of the full document is refused, by
/// name: `missing field`, not a default (`INV-06`). This is every
/// per-section missing-key test in one walk.
#[test]
fn removing_any_key_of_any_object_is_refused() {
    let doc = full_document();
    let mut pointers = Vec::new();
    object_pointers(&doc, "", &mut pointers);
    let mut walked = 0;
    for pointer in &pointers {
        let keys: Vec<String> = doc
            .pointer(pointer)
            .unwrap()
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        for key in keys {
            let mut cut = doc.clone();
            cut.pointer_mut(pointer)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(&key);
            let outcome = parse(&cut).and_then(|s| s.validate().map_err(|e| e.to_string()));
            if let Some((_, _, why)) = OPTIONAL_KEYS
                .iter()
                .find(|(p, k, _)| p == pointer && *k == key)
            {
                assert!(outcome.is_ok(), "{pointer}/{key} ({why}) is refused");
                continue;
            }
            let err = outcome.expect_err(&format!("{pointer} without `{key}` is accepted"));
            assert!(
                err.contains(&format!("missing field `{key}`")),
                "{pointer} without `{key}`: {err}"
            );
            walked += 1;
        }
    }
    assert!(walked > 250, "the walk only reached {walked} keys");
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
        schema_version: aethelgard_ir::SOLVED_SCHEMA_VERSION,
        specification_sha256: HASH.into(),
        assignments: [
            ("I2C_SCL".to_owned(), assignment("PB8", 4)),
            (
                "SPI_SCK".to_owned(),
                PinAssignment {
                    pin: "PA5".into(),
                    peripheral: "SPI1".into(),
                    signal: "SPI1_SCK".into(),
                    alternate_function: 5,
                },
            ),
            ("I2C_SDA".to_owned(), {
                let mut sda = assignment("PB9", 4);
                sda.signal = "I2C1_SDA".into();
                sda
            }),
        ]
        .into(),
        gpio: [("FLASH_CS".to_owned(), "PA0".to_owned())].into(),
        i2c_ceilings: [("SENSORS".to_owned(), 400_000)].into(),
        i2c_speeds: [("SENSORS".to_owned(), 400_000)].into(),
        analog_lines: [
            ("I2C_SCL".to_owned(), analog_line()),
            ("I2C_SDA".to_owned(), analog_line()),
        ]
        .into(),
        // `U10` sorts before `U3`, so the map's order is not the index order.
        i2c_devices: [
            ("U3".to_owned(), {
                let mut tmp = device(0, 0x48);
                tmp.part = "TMP117".into();
                tmp
            }),
            ("U10".to_owned(), device(1, 0x76)),
        ]
        .into(),
        spi_devices: [("U4".to_owned(), spi_device(0))].into(),
        spi_lines: [("SPI_SCK".to_owned(), spi_line("SCK"))].into(),
        clock: SolvedClock::reset(),
        motor_outputs: BTreeMap::new(),
        power: None,
        serial_ports: BTreeMap::new(),
        // Hand values (`silicon`'s tests): SPI1 at 20 MHz is MBR 1; I2C1's
        // three words at 64 MHz.
        spi_controllers: [(
            "SPI1".to_owned(),
            SolvedSpiController {
                kernel: SpiKernel::PerCk,
                kernel_hz: 64_000_000,
                declared_hz: 20_000_000,
                mbr: 1,
                achieved_hz: 16_000_000,
                achieved_max_hz: 16_239_744,
            },
        )]
        .into(),
        i2c_timings: [(
            "I2C1".to_owned(),
            SolvedI2cTiming {
                kernel_hz: 64_000_000,
                cr1: 0,
                timingr: [
                    ("Standard100k".to_owned(), 0x50D3_2A40),
                    ("Fast400k".to_owned(), 0x10C7_123C),
                    ("FastPlus1M".to_owned(), 0x00B2_0E2D),
                ]
                .into(),
            },
        )]
        .into(),
        bor_level: 0,
        mcu: "STM32H753ZITx".into(),
        pin_speeds: [
            ("FLASH_CS".to_owned(), 0),
            ("I2C_SCL".to_owned(), 0),
            ("I2C_SDA".to_owned(), 0),
            ("SPI_SCK".to_owned(), 1),
        ]
        .into(),
        foreign_clock: None,
    }
}

fn spi_line(signal: &str) -> SpiLine {
    SpiLine {
        bus: "FLASH".into(),
        signal: signal.into(),
        rate_hz: 20_000_000,
        capacitance_femtofarads: 30_000,
        rise_time_ns: 20,
        vdd_min_millivolts: 3135,
    }
}

fn spi_device(index: u32) -> SolvedSpiDevice {
    SolvedSpiDevice {
        index,
        part: "W25Q64JVxIQ".into(),
        bus: "FLASH".into(),
        peripheral: "SPI1".into(),
        cs: "FLASH_CS".into(),
        orientation: None,
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
    // Version 1 is the file before `i2c_speeds` (`AET-WO-1205`), 2 the file
    // before `spi_devices` (`AET-WO-4200`), 3 the file before `spi_lines`
    // (`AET-WO-4300c`), 4 the file before `clock` and `motor_outputs`
    // (`AET-WO-4300a`), 5 the file before `power` (`AET-WO-4300d`), 6 the
    // file before `serial_ports` (`AET-WO-4300b`), 7 the file before
    // `spi_controllers`, `i2c_timings` and `bor_level` (`AET-WO-4230`), 8 the
    // file before `mcu` and `pin_speeds` (`AET-WO-4400b`), 9 the file before
    // an SPI device's `orientation` (`AET-WO-4400d`), 10 the file before
    // `foreign_clock` (`AET-WO-4400g`), 12 one this build does not know.
    for version in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12] {
        let err = solved_refusal(|t| t.schema_version = version);
        assert_refusal(err, "UnsupportedSchemaVersion", None);
    }
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
    let a = r#"{"schema_version":4,"specification_sha256":"h","assignments":{"B":{"pin":"p","peripheral":"q","signal":"s","alternate_function":1},"A":{"pin":"r","peripheral":"q","signal":"s","alternate_function":1}},"gpio":{"Z":"g","C":"c"},"i2c_ceilings":{"Y":100000,"B":400000},"i2c_speeds":{"Y":100000,"B":400000},"analog_lines":{},"i2c_devices":{},"spi_devices":{},"spi_lines":{}}"#;
    let b = r#"{"schema_version":4,"specification_sha256":"h","assignments":{"A":{"pin":"r","peripheral":"q","signal":"s","alternate_function":1},"B":{"pin":"p","peripheral":"q","signal":"s","alternate_function":1}},"gpio":{"C":"c","Z":"g"},"i2c_ceilings":{"B":400000,"Y":100000},"i2c_speeds":{"B":400000,"Y":100000},"analog_lines":{},"i2c_devices":{},"spi_devices":{},"spi_lines":{}}"#;
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
    // Each mode with a rise-time bound that derives it (`AET-WO-1204`).
    for (hz, ns) in [(100_000, 1000), (400_000, 300), (1_000_000, 120)] {
        let mut t = solved();
        t.i2c_ceilings.insert("SENSORS".into(), hz);
        t.i2c_speeds.insert("SENSORS".into(), hz);
        for line in t.analog_lines.values_mut() {
            line.ceiling_ns = ns;
        }
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

// ---- A hand-written topology holds only the ceiling `solve` derives
// (AET-WO-1204; AET-WO-2200 F14) ----------------------------------------------

fn assert_not_derived(err: E, bus: &str, ceiling_hz: u32, derived_hz: Option<u32>) {
    assert_eq!(
        err,
        E::CeilingNotDerived {
            bus: bus.into(),
            ceiling_hz,
            derived_hz
        },
        "{err}"
    );
    assert_eq!(err.kind(), "CeilingNotDerived");
    assert_eq!(err.id(), Some(bus));
}

#[test]
fn solved_refuses_a_rise_time_slower_than_every_mode() {
    // F14: `ceiling_ns: u32::MAX` validated; `solve` refuses it (`RiseTimeTooSlow`).
    let err = solved_refusal(|t| {
        for line in t.analog_lines.values_mut() {
            line.ceiling_ns = u32::MAX;
        }
    });
    assert_not_derived(err, "SENSORS", 400_000, None);
    // The worse line decides (`AET-WO-1206` §1.7): one line past every mode
    // beside a 300 ns line derives no mode, as `solve` refuses it.
    let err = solved_refusal(|t| {
        t.analog_lines.get_mut("I2C_SDA").unwrap().ceiling_ns = 1001;
    });
    assert_not_derived(err, "SENSORS", 400_000, None);
}

#[test]
fn solved_refuses_a_ceiling_faster_or_slower_than_its_lines_derive() {
    let err = solved_refusal(|t| {
        for line in t.analog_lines.values_mut() {
            line.ceiling_ns = 120;
        }
    });
    assert_not_derived(err, "SENSORS", 400_000, Some(1_000_000));
    let err = solved_refusal(|t| {
        t.i2c_ceilings.insert("SENSORS".into(), 1_000_000);
    });
    assert_not_derived(err, "SENSORS", 1_000_000, Some(400_000));
}

#[test]
fn solved_derives_the_ceiling_from_the_worse_line_of_the_bus() {
    // One line at 120 ns and one at 300 ns: the larger bound decides, as in
    // `solve` (`AET-WO-1206` §1.7), so the bus derives 400 kHz, whichever
    // line is the faster, and a 1 MHz ceiling is refused.
    for fast in ["I2C_SDA", "I2C_SCL"] {
        let mut t = solved();
        t.analog_lines.get_mut(fast).unwrap().ceiling_ns = 120;
        assert_eq!(t.validate(), Ok(()), "{fast}");
        t.i2c_ceilings.insert("SENSORS".into(), 1_000_000);
        assert_not_derived(
            t.validate().unwrap_err(),
            "SENSORS",
            1_000_000,
            Some(400_000),
        );
    }
}

#[test]
fn solved_refuses_a_half_bus_whose_one_line_would_set_the_ceiling() {
    // Adversarial B1: only SDA at 100 ns, so the ceiling would derive 1 MHz
    // from one line where `solve` reads both.
    let mut t = solved();
    t.assignments.remove("I2C_SCL");
    t.analog_lines.remove("I2C_SCL");
    t.analog_lines.get_mut("I2C_SDA").unwrap().ceiling_ns = 100;
    t.i2c_ceilings.insert("SENSORS".into(), 1_000_000);
    assert_not_derived(t.validate().unwrap_err(), "SENSORS", 1_000_000, None);
}

#[test]
fn solved_refuses_a_ceiling_for_a_bus_with_no_line() {
    let err = solved_refusal(|t| {
        t.i2c_ceilings.insert("EXTRA".into(), 400_000);
    });
    assert_not_derived(err, "EXTRA", 400_000, None);
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
    // Huge values are the bridge's to measure or refuse, not validate's, but
    // capacitance now has the declared speed's limit (`AET-WO-1205`), and a
    // line's rise time bounds its bus's ceiling, the worse line deciding
    // (`AET-WO-1206` §1.7): 300 ns is the largest that keeps 400 kHz.
    // `u32::MAX` is `solved_refuses_a_rise_time_slower_than_every_mode`'s.
    let mut t = solved();
    let line = t.analog_lines.get_mut("I2C_SDA").unwrap();
    line.pull_up_ohms = u32::MAX;
    line.capacitance_femtofarads = 400_000;
    line.ceiling_ns = 300;
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
        r#""U3":{"address":72,"bus":"SENSORS","index":0,"part":"TMP117","peripheral":"I2C1"}"#;
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
    let err = refusal(|s| devs(s).spi[0].cs.net = "gnd".into());
    assert_refusal(err, "NetShadowsDomain", Some("gnd"));
    let err = refusal(|s| devs(s).spi[0].cs.net = "i2c_scl".into());
    assert_refusal(err, "DuplicateNetName", Some("i2c_scl"));
    let err = refusal(|s| devs(s).spi[0].cs.net = "vdd_3v3".into());
    assert_refusal(err, "NetShadowsDomain", Some("vdd_3v3"));
}

#[test]
fn refuses_voltage_domains_that_differ_only_in_case() {
    // Adversarial pass of AET-WO-1900: power names fold like net names do.
    let err = refusal(|s| {
        let mut d = s.voltage_domains[0].clone();
        d.id = "vdd_3v3".into();
        s.voltage_domains.push(d);
    });
    assert_refusal(err, "DuplicateDomainId", Some("vdd_3v3"));
    let err = refusal(|s| {
        let mut d = s.voltage_domains[0].clone();
        d.id = "gnd".into();
        s.voltage_domains.push(d);
    });
    assert_refusal(err, "DuplicateDomainId", Some("gnd"));
}

// ---- SolvedTopology.i2c_speeds (AET-WO-1205) --------------------------------

fn set_line_pull_up(t: &mut SolvedTopology, ohms: u32) {
    for line in t.analog_lines.values_mut() {
        line.pull_up_ohms = ohms;
    }
}

#[test]
fn a_version_1_file_is_refused_by_version_not_by_a_missing_key() {
    let mut v = solved_json();
    v["schema_version"] = 1.into();
    v.as_object_mut().unwrap().remove("i2c_speeds");
    let t: SolvedTopology = serde_json::from_value(v).unwrap();
    assert_refusal(t.validate().unwrap_err(), "UnsupportedSchemaVersion", None);
    // Under the current version the key is required, never defaulted.
    let mut v = solved_json();
    v.as_object_mut().unwrap().remove("i2c_speeds");
    let err = serde_json::from_value::<SolvedTopology>(v).unwrap_err();
    assert!(
        err.to_string().contains("missing field `i2c_speeds`"),
        "{err}"
    );
    // A repeated key is refused like every keyed collection.
    let json = r#"{"schema_version":4,"specification_sha256":"h","assignments":{},"gpio":{},"i2c_ceilings":{},"i2c_speeds":{"B":400000,"B":100000},"analog_lines":{},"i2c_devices":{},"spi_devices":{},"spi_lines":{}}"#;
    assert!(serde_json::from_str::<SolvedTopology>(json).is_err());
}

#[test]
fn solved_refuses_a_speed_key_missing_or_extra() {
    let err = solved_refusal(|t| {
        t.i2c_speeds.clear();
    });
    assert_refusal(err, "SpeedBusMismatch", Some("SENSORS"));
    let err = solved_refusal(|t| {
        t.i2c_speeds.insert("EXTRA".into(), 100_000);
    });
    assert_refusal(err, "SpeedBusMismatch", Some("EXTRA"));
    let err = solved_refusal(|t| {
        t.i2c_speeds.insert("BUS 2".into(), 100_000);
    });
    assert_invalid_identifier(err, "i2c_speeds.BUS 2");
}

#[test]
fn solved_refuses_a_speed_no_mode_admits_or_above_the_ceiling() {
    for hz in [
        0,
        1,
        99_999,
        100_001,
        399_999,
        1_000_001,
        1_500_000,
        u32::MAX,
    ] {
        let err = solved_refusal(|t| {
            t.i2c_speeds.insert("SENSORS".into(), hz);
        });
        assert_refusal(err, "UnsupportedI2cSpeed", Some("SENSORS"));
    }
    // 1 MHz under the 400 kHz ceiling.
    let err = solved_refusal(|t| {
        t.i2c_speeds.insert("SENSORS".into(), 1_000_000);
    });
    assert_eq!(
        err,
        E::BusTooFast {
            bus: "SENSORS".into(),
            speed_hz: 1_000_000,
            ceiling_hz: 400_000
        }
    );
    assert_eq!(err.kind(), "BusTooFast");
    // Below the ceiling is fine.
    let mut t = solved();
    t.i2c_speeds.insert("SENSORS".into(), 100_000);
    assert_eq!(t.validate(), Ok(()));
}

#[test]
fn solved_holds_the_pull_up_to_the_floor_at_the_declared_speed() {
    // 3.3 V, 400 kHz: ceil(2900 / 3 mA) = 967 ohms.
    let err = solved_refusal(|t| set_line_pull_up(t, 966));
    assert_eq!(
        err,
        E::PullUpBelowSinkFloor {
            bus: "SENSORS".into(),
            pull_up_ohms: 966,
            floor_ohms: 967
        }
    );
    assert_eq!(err.kind(), "PullUpBelowSinkFloor");
    assert_eq!(err.id(), Some("SENSORS"));
    let mut t = solved();
    set_line_pull_up(&mut t, 967);
    assert_eq!(t.validate(), Ok(()));
    // One line alone below the floor refuses.
    let err = solved_refusal(|t| t.analog_lines.get_mut("I2C_SCL").unwrap().pull_up_ohms = 966);
    assert_refusal(err, "PullUpBelowSinkFloor", Some("SENSORS"));
    // The floor follows the declared speed, not the ceiling: at 1 MHz it is
    // 8 mA (DS12117 Rev 10 Table 50, `AET-WO-4230` D8), ceil(2900 / 8 mA) =
    // 363 ohms at the topology's rail, so 362 refuses and 363 passes.
    let at_1m = |t: &mut SolvedTopology, ohms| {
        t.i2c_ceilings.insert("SENSORS".into(), 1_000_000);
        t.i2c_speeds.insert("SENSORS".into(), 1_000_000);
        for line in t.analog_lines.values_mut() {
            line.ceiling_ns = 120;
            line.pull_up_ohms = ohms;
        }
    };
    let err = solved_refusal(|t| at_1m(t, 362));
    assert_refusal(err, "PullUpBelowSinkFloor", Some("SENSORS"));
    let mut t = solved();
    at_1m(&mut t, 363);
    assert_eq!(t.validate(), Ok(()));
    // A rail at the 2 V boundary reads 0.2 x VDD at 2 mA: ceil(1600 / 2 mA) = 800.
    // One millivolt above it is VOL 0.4 V at 3 mA: ceil(1601 / 3 mA) = 534.
    for (mv, ohms, ok) in [
        (2000, 799, false),
        (2000, 800, true),
        (2001, 533, false),
        (2001, 534, true),
    ] {
        let mut t = solved();
        for line in t.analog_lines.values_mut() {
            line.domain_millivolts = mv;
            line.pull_up_ohms = ohms;
        }
        assert_eq!(t.validate().is_ok(), ok, "{mv} mV {ohms} ohm");
    }
}

#[test]
fn solved_holds_the_capacitance_to_the_limit_at_the_declared_speed() {
    let err = solved_refusal(|t| {
        t.analog_lines
            .get_mut("I2C_SDA")
            .unwrap()
            .capacitance_femtofarads = 400_001
    });
    assert_eq!(
        err,
        E::BusCapacitanceOverLimit {
            net: "I2C_SDA".into(),
            capacitance_femtofarads: 400_001,
            limit_femtofarads: 400_000
        }
    );
    assert_eq!(err.kind(), "BusCapacitanceOverLimit");
    assert_eq!(err.id(), Some("I2C_SDA"));
    let mut t = solved();
    t.analog_lines
        .get_mut("I2C_SDA")
        .unwrap()
        .capacitance_femtofarads = 400_000;
    assert_eq!(t.validate(), Ok(()));
    // At 1 MHz the limit is 550 pF.
    for (pf, ok) in [(550_000, true), (550_001, false)] {
        let mut t = solved();
        t.i2c_ceilings.insert("SENSORS".into(), 1_000_000);
        t.i2c_speeds.insert("SENSORS".into(), 1_000_000);
        for line in t.analog_lines.values_mut() {
            line.ceiling_ns = 120;
            line.capacitance_femtofarads = pf;
        }
        assert_eq!(t.validate().is_ok(), ok, "{pf} fF");
    }
}

#[test]
fn the_floor_and_the_limit_follow_the_declared_speed_not_the_ceiling() {
    // A 1 MHz ceiling over a 400 kHz bus: the floor is the 3 mA one (967 ohms),
    // not the 20 mA one (145), and the limit is 400 pF, not 550 pF.
    let under_1m = |t: &mut SolvedTopology| {
        t.i2c_ceilings.insert("SENSORS".into(), 1_000_000);
        for line in t.analog_lines.values_mut() {
            line.ceiling_ns = 120;
        }
    };
    let err = solved_refusal(|t| {
        under_1m(t);
        set_line_pull_up(t, 500);
    });
    assert_refusal(err, "PullUpBelowSinkFloor", Some("SENSORS"));
    let err = solved_refusal(|t| {
        under_1m(t);
        t.analog_lines
            .get_mut("I2C_SDA")
            .unwrap()
            .capacitance_femtofarads = 550_000;
    });
    assert_refusal(err, "BusCapacitanceOverLimit", Some("I2C_SDA"));
    let mut t = solved();
    under_1m(&mut t);
    assert_eq!(t.validate(), Ok(()));
}

// ---- SolvedTopology.spi_devices (AET-WO-4200) --------------------------------

fn rename_i2c(t: &mut SolvedTopology, from: &str, to: &str) {
    let d = t.i2c_devices.remove(from).unwrap();
    t.i2c_devices.insert(to.into(), d);
}

fn rename_spi(t: &mut SolvedTopology, from: &str, to: &str) {
    let d = t.spi_devices.remove(from).unwrap();
    t.spi_devices.insert(to.into(), d);
}

#[test]
fn a_version_2_file_without_spi_devices_is_refused_by_version_not_by_a_missing_key() {
    let mut v = solved_json();
    v["schema_version"] = 2.into();
    v.as_object_mut().unwrap().remove("spi_devices");
    let t: SolvedTopology = serde_json::from_value(v).unwrap();
    assert_refusal(t.validate().unwrap_err(), "UnsupportedSchemaVersion", None);
    // Under the current version the key is required, never defaulted.
    let mut v = solved_json();
    v.as_object_mut().unwrap().remove("spi_devices");
    let err = serde_json::from_value::<SolvedTopology>(v).unwrap_err();
    assert!(
        err.to_string().contains("missing field `spi_devices`"),
        "{err}"
    );
    // A repeated key is refused like every keyed collection.
    let json = r#"{"schema_version":4,"specification_sha256":"h","assignments":{},"gpio":{},"i2c_ceilings":{},"i2c_speeds":{},"analog_lines":{},"i2c_devices":{},"spi_devices":{"U4":{"index":0,"part":"P","bus":"B","peripheral":"SPI1","cs":"C"},"U4":{"index":0,"part":"P","bus":"B","peripheral":"SPI1","cs":"C"}},"spi_lines":{}}"#;
    assert!(serde_json::from_str::<SolvedTopology>(json).is_err());
}

#[test]
fn spi_devices_come_out_in_index_order_not_reference_order() {
    let mut t = solved();
    t.spi_devices.clear();
    t.spi_devices.insert("U3".into(), spi_device(0));
    t.spi_devices.insert("U10".into(), spi_device(1));
    let refs: Vec<&str> = t.spi_devices.keys().map(String::as_str).collect();
    assert_eq!(refs, ["U10", "U3"]);
    let order: Vec<&str> = t.spi_devices_in_order().iter().map(|d| d.0).collect();
    assert_eq!(order, ["U3", "U10"]);
}

#[test]
fn solved_refuses_an_spi_device_off_an_spi_peripheral_or_with_no_signal_on_it() {
    // Not an `SPIn` at all, even though a signal is solved to it.
    let err = solved_refusal(|t| t.spi_devices.get_mut("U4").unwrap().peripheral = "I2C1".into());
    assert_refusal(err, "SpiDevicePeripheral", Some("U4"));
    // An `SPIn` no solved signal is on.
    let err = solved_refusal(|t| t.spi_devices.get_mut("U4").unwrap().peripheral = "SPI2".into());
    assert_refusal(err, "SpiDevicePeripheral", Some("U4"));
}

#[test]
fn solved_refuses_an_spi_chip_select_that_is_not_a_gpio() {
    let err = solved_refusal(|t| t.spi_devices.get_mut("U4").unwrap().cs = "NOPE".into());
    assert_refusal(err, "SpiDeviceSelect", Some("U4"));
    // A net solved to an alternate function is not a GPIO.
    let err = solved_refusal(|t| t.spi_devices.get_mut("U4").unwrap().cs = "SPI_SCK".into());
    assert_refusal(err, "SpiDeviceSelect", Some("U4"));
}

#[test]
fn solved_refuses_spi_indices_that_are_not_exactly_zero_to_n() {
    // Repeated, out of range, and a gap, as for the I2C devices.
    let err = solved_refusal(|t| t.spi_devices.get_mut("U4").unwrap().index = 1);
    assert_refusal(err, "SpiDeviceIndex", Some("U4"));
    for (a, b) in [(0, 0), (1, 1), (0, 2), (1, 2), (5, 6)] {
        let err = solved_refusal(|t| {
            t.spi_devices.insert("U5".into(), spi_device(b));
            t.spi_devices.get_mut("U4").unwrap().index = a;
        });
        assert_eq!(err.kind(), "SpiDeviceIndex", "{a} {b}");
    }
    let mut ok = solved();
    ok.spi_devices.insert("U5".into(), spi_device(1));
    assert_eq!(ok.validate(), Ok(()));
}

#[test]
fn solved_refuses_an_spi_key_or_field_that_is_not_an_identifier() {
    let err = solved_refusal(|t| rename_spi(t, "U4", "U 4"));
    assert_invalid_identifier(err, "spi_devices.U 4");
    let err = solved_refusal(|t| t.spi_devices.get_mut("U4").unwrap().part = "".into());
    assert_invalid_identifier(err, "spi_devices.U4.part");
    let err = solved_refusal(|t| t.spi_devices.get_mut("U4").unwrap().bus = "".into());
    assert_invalid_identifier(err, "spi_devices.U4.bus");
    let err = solved_refusal(|t| t.spi_devices.get_mut("U4").unwrap().peripheral = "".into());
    assert_invalid_identifier(err, "spi_devices.U4.peripheral");
    let err = solved_refusal(|t| t.spi_devices.get_mut("U4").unwrap().cs = "".into());
    assert_invalid_identifier(err, "spi_devices.U4.cs");
}

// ---- D7: reference collisions (AET-WO-4200) ---------------------------------

const MUXES: [&str; 6] = [
    "spi1mux", "spi2mux", "spi3mux", "spi4mux", "spi5mux", "spi6mux",
];

#[test]
fn a_reference_equal_to_a_mux_name_is_reserved_in_any_case() {
    for name in ["SPI1MUX", "spi4mux", "Spi6Mux", "spi1mux"] {
        let err = refusal(|s| devs(s).i2c[0].reference = name.into());
        assert_invalid_identifier(err.clone(), "i2c_devices[0].reference");
        assert!(err.to_string().contains("already holds"), "{err}");
        let err = refusal(|s| devs(s).spi[0].reference = name.into());
        assert_invalid_identifier(err.clone(), "spi_devices[0].reference");
        assert!(err.to_string().contains("already holds"), "{err}");
        let err = solved_refusal(|t| rename_i2c(t, "U3", name));
        assert_invalid_identifier(err, &format!("i2c_devices.{name}"));
        let err = solved_refusal(|t| rename_spi(t, "U4", name));
        assert_invalid_identifier(err, &format!("spi_devices.{name}"));
    }
    for mux in MUXES {
        assert!(aethelgard_ir::RESERVED_REFERENCES.contains(&mux), "{mux}");
    }
}

#[test]
fn a_reference_plus_memory_that_is_another_reference_is_a_collision() {
    // The error's `id` is the reference whose array name collides, `with` the
    // other reference as spelled.
    let err = refusal(|s| devs(s).i2c[0].reference = "u4_memory".into());
    assert_refusal(err.clone(), "ReferenceCollision", Some("U4"));
    assert!(matches!(&err, E::ReferenceCollision { with, .. } if with == "u4_memory"));
    let err = refusal(|s| devs(s).i2c[0].reference = "U4_MEMORY".into());
    assert!(
        matches!(&err, E::ReferenceCollision { id, with }
        if id == "U4" && with == "U4_MEMORY"),
        "{err}"
    );
    let err = refusal(|s| devs(s).spi[0].reference = "u2_memory".into());
    assert!(
        matches!(&err, E::ReferenceCollision { id, with }
        if id == "U2" && with == "u2_memory"),
        "{err}"
    );
    // Within one list.
    let err = refusal(|s| devs(s).i2c[1].reference = "u2_memory".into());
    assert!(
        matches!(&err, E::ReferenceCollision { id, with }
        if id == "U2" && with == "u2_memory"),
        "{err}"
    );

    let err = solved_refusal(|t| rename_i2c(t, "U3", "u4_memory"));
    assert!(
        matches!(&err, E::ReferenceCollision { id, with }
        if id == "U4" && with == "u4_memory"),
        "{err}"
    );
    let err = solved_refusal(|t| rename_spi(t, "U4", "u3_memory"));
    assert!(
        matches!(&err, E::ReferenceCollision { id, with }
        if id == "U3" && with == "u3_memory"),
        "{err}"
    );
    let err = solved_refusal(|t| rename_i2c(t, "U10", "u3_memory"));
    assert!(
        matches!(&err, E::ReferenceCollision { id, with }
        if id == "U3" && with == "u3_memory"),
        "{err}"
    );
    assert_eq!(err.kind(), "ReferenceCollision");
    assert_eq!(err.id(), Some("U3"));
}

/// The cell "a reference whose lowercase plus `_memory` is a reserved name" is
/// empty by construction: no reserved name ends in `_memory`, so
/// `references_distinct` can only match the suffix against another reference.
/// If a reserved name ever does, this fails and the cell needs its own test.
#[test]
fn no_reserved_name_ends_in_memory() {
    for name in aethelgard_ir::RESERVED_REFERENCES {
        assert!(
            !name.ends_with(aethelgard_ir::MEMORY_SUFFIX),
            "{name} is reserved and ends in the memory suffix: add the D7 cell"
        );
    }
    assert_eq!(aethelgard_ir::MEMORY_SUFFIX, "_memory");
}

#[test]
fn solved_refuses_a_reference_equal_in_any_case_across_both_maps() {
    // The later key in i2c-then-spi order is the one named; `first` is the earlier.
    let err = solved_refusal(|t| rename_i2c(t, "U3", "u4"));
    assert!(
        matches!(&err, E::DuplicateReference { id, first }
        if id == "U4" && first == "u4"),
        "{err}"
    );
    assert_eq!(err.id(), Some("U4"));
    let err = solved_refusal(|t| rename_spi(t, "U4", "u3"));
    assert!(
        matches!(&err, E::DuplicateReference { id, first }
        if id == "u3" && first == "U3"),
        "{err}"
    );
    assert_eq!(err.id(), Some("u3"));
    // The same spelling in both maps.
    let err = solved_refusal(|t| rename_spi(t, "U4", "U3"));
    assert_refusal(err, "DuplicateReference", Some("U3"));
}

// ---- Part limits and SPI lines (AET-WO-4300c) -----------------------------

#[test]
fn the_bare_flash_name_is_refused_as_ambiguous_naming_both_ordering_codes() {
    let err = refusal(|s| devs(s).spi[0].part = "W25Q64JV".into());
    assert_refusal(err.clone(), "AmbiguousPartName", Some("U4"));
    let text = err.to_string();
    assert!(text.contains("W25Q64JVxIQ"), "{text}");
    assert!(text.contains("EF 70 17"), "{text}");
}

#[test]
fn a_flash_bus_above_the_parts_clock_is_refused_and_at_it_accepted() {
    let err = refusal(|s| s.spi_buses[0].speed_hz = 104_000_001);
    assert_refusal(err.clone(), "SpiClockAbovePart", Some("U4"));
    assert!(err.to_string().contains("9.6"), "{err}");
    let mut spec = valid();
    spec.spi_buses[0].speed_hz = 104_000_000;
    assert_eq!(spec.validate(), Ok(()));
}

#[test]
fn a_part_on_the_wrong_kind_of_bus_is_refused() {
    let err = refusal(|s| devs(s).spi[0].part = "BME280".into());
    assert_refusal(err.clone(), "PartOnWrongBus", Some("U4"));
    let text = err.to_string();
    assert!(text.contains("BME280") && text.contains("6.2"), "{text}");
    let err = refusal(|s| devs(s).i2c[0].part = "W25Q64JVxIQ".into());
    assert_refusal(err.clone(), "PartOnWrongBus", Some("U2"));
    let text = err.to_string();
    assert!(
        text.contains("W25Q64JVxIQ") && text.contains("9.6"),
        "{text}"
    );
}

#[test]
fn an_i2c_address_the_part_does_not_offer_is_refused() {
    let err = refusal(|s| {
        // U3 moves too, so the refusal is the part's and not a duplicate's.
        devs(s).i2c[1].address = 0x4B;
        devs(s).i2c[0].address = 0x48;
    });
    assert_refusal(err.clone(), "I2cAddressNotOffered", Some("U2"));
    let text = err.to_string();
    assert!(text.contains("0x76") && text.contains("6.2"), "{text}");
}

#[test]
/// `AET-WO-1206` §1.8: a part with no row in the table is refused by name,
/// I2C then SPI in document order, by the emitter's kind, `id` and message.
/// Before, it passed every limit unheld (and `generate` refused it).
fn a_part_the_table_does_not_hold_is_refused() {
    let mut spec = valid();
    devs(&mut spec).spi[0].part = "UNLISTED".into();
    spec.spi_buses[0].speed_hz = 200_000_000;
    let err = spec.validate().unwrap_err();
    assert_eq!(
        err,
        E::PartWithoutRow {
            reference: "U4".into(),
            part: "UNLISTED".into()
        }
    );
    assert_refusal(err.clone(), "UnknownPart", Some("U4"));
    assert_eq!(
        err.to_string(),
        "`U4` is a `UNLISTED`, which has no row in the parts table; known: BME280, BMP581, ICM20948, ICM42688P, ICP10111, TMP117, W25Q64JVxIQ"
    );
    devs(&mut spec).i2c[1].part = "LM75".into();
    assert_refusal(spec.validate().unwrap_err(), "UnknownPart", Some("U3"));
}

#[test]
fn solved_holds_the_flash_to_its_clock_after_a_bus_is_sped_up() {
    // The bus and its controller sped up together: above 32,479,488 Hz (the
    // fastest kernel / 2) MBR is 0.
    let sped_up = |t: &mut SolvedTopology, rate_hz: u32| {
        for l in t.spi_lines.values_mut() {
            l.rate_hz = rate_hz;
        }
        let c = t.spi_controllers.get_mut("SPI1").unwrap();
        (c.declared_hz, c.mbr, c.achieved_hz, c.achieved_max_hz) =
            (rate_hz, 0, 32_000_000, 32_479_488);
    };
    // At the part's own 104 MHz the part passes, and the stored speed is
    // refused: no selectable speed serves it (`AET-WO-4400b2` F4).
    let mut ok = solved();
    sped_up(&mut ok, 104_000_000);
    assert_eq!(ok.validate().unwrap_err().kind(), "RateAbovePinSpeed");
    let err = solved_refusal(|t| sped_up(t, 104_000_001));
    assert_refusal(err.clone(), "SpiClockAbovePart", Some("U4"));
    assert!(err.to_string().contains("9.6"), "{err}");
}

#[test]
fn solved_holds_an_i2c_device_to_the_addresses_its_part_offers() {
    let err = solved_refusal(|t| t.i2c_devices.get_mut("U10").unwrap().address = 0x40);
    assert_refusal(err.clone(), "I2cAddressNotOffered", Some("U10"));
    assert!(err.to_string().contains("6.2"), "{err}");
}

#[test]
fn solved_holds_a_device_to_its_kind_of_bus_after_a_move() {
    let err = solved_refusal(|t| t.spi_devices.get_mut("U4").unwrap().part = "BME280".into());
    assert_refusal(err, "PartOnWrongBus", Some("U4"));
    let err = solved_refusal(|t| t.i2c_devices.get_mut("U10").unwrap().part = "W25Q64JVxIQ".into());
    assert_refusal(err, "PartOnWrongBus", Some("U10"));
}

#[test]
fn spi_lines_must_match_the_solved_spi_signals() {
    // An SCK assignment with no line.
    let err = solved_refusal(|t| {
        t.spi_lines.clear();
    });
    assert_refusal(err, "SpiLineMismatch", Some("SPI_SCK"));
    // A line on a net that is not an SPI SCK or MOSI assignment.
    let err = solved_refusal(|t| {
        t.spi_lines.insert("I2C_SDA".into(), spi_line("SCK"));
    });
    assert_refusal(err, "SpiLineMismatch", Some("I2C_SDA"));
    // A line on a net with no assignment.
    let err = solved_refusal(|t| {
        t.spi_lines.insert("GHOST".into(), spi_line("MOSI"));
    });
    assert_refusal(err, "SpiLineMismatch", Some("GHOST"));
    // A line naming the other signal than its assignment's.
    let err = solved_refusal(|t| t.spi_lines.get_mut("SPI_SCK").unwrap().signal = "MOSI".into());
    assert_refusal(err, "SpiLineMismatch", Some("SPI_SCK"));
    // A zero value.
    let err = solved_refusal(|t| t.spi_lines.get_mut("SPI_SCK").unwrap().rate_hz = 0);
    assert_refusal(err.clone(), "SpiLineMismatch", Some("SPI_SCK"));
    assert!(err.to_string().contains("no zero value"), "{err}");
}

#[test]
fn a_version_4_file_without_spi_lines_is_a_missing_field_and_a_version_3_one_is_refused_by_version()
{
    let mut v = solved_json();
    v.as_object_mut().unwrap().remove("spi_lines");
    let err = parse_solved(&v).unwrap_err();
    assert!(err.contains("missing field `spi_lines`"), "{err}");
    v["schema_version"] = 3.into();
    let t = serde_json::from_value::<SolvedTopology>(v).expect("an older file still reads");
    assert_refusal(t.validate().unwrap_err(), "UnsupportedSchemaVersion", None);
}

/// Adversarial 2: each device's supply, nominal less and plus its tolerance,
/// within its part's range, citing the datasheet.
#[test]
fn a_supply_outside_a_part_s_range_is_refused() {
    let mut spec = valid();
    spec.voltage_domains[0].nominal_millivolts = 3_600;
    let err = spec.validate().unwrap_err();
    assert_refusal(err.clone(), "SupplyOutsidePart", Some("U2"));
    assert!(err.to_string().contains("3420 to 3780 mV"), "{err}");
    // The flash alone, at 2.8 V less 5 %: 2660 mV, under its 2.7 V.
    let mut spec = valid();
    spec.voltage_domains[0].nominal_millivolts = 2_800;
    devs(&mut spec).i2c.clear();
    let err = spec.validate().unwrap_err();
    assert_refusal(err.clone(), "SupplyOutsidePart", Some("U4"));
    assert!(err.to_string().contains("9.2"), "{err}");
}

/// `AET-WO-4210` D7: a device whose part separates its I/O range from its
/// supply is refused by name when its bus's domain leaves that range: the
/// ICM-20948's `VDDIO` tops out at 1.95 V, so on a 3.3 V bus it is refused
/// though its supply range holds 3.3 V. At 1.8 V the whole board passes.
#[test]
fn a_device_whose_io_range_excludes_its_bus_domain_is_refused() {
    let mut spec = valid();
    devs(&mut spec).spi[0].part = "ICM20948".into();
    let err = spec.validate().unwrap_err();
    assert_refusal(err.clone(), "IoOutsidePart", Some("U4"));
    let m = err.to_string();
    assert!(
        m.contains("3135 to 3465 mV")
            && m.contains("1710 to 1950 mV")
            && m.contains("3.4, table 4"),
        "{m}"
    );
    let mut spec = valid();
    devs(&mut spec).spi[0].part = "ICM20948".into();
    devs(&mut spec).i2c.truncate(1);
    devs(&mut spec).i2c[0].part = "ICP10111".into();
    devs(&mut spec).i2c[0].address = 0x63;
    let v = serde_json::to_string(&spec)
        .unwrap()
        .replace("VDD_3V3", "VDD_1V8");
    let mut spec: Specification = serde_json::from_str(&v).unwrap();
    spec.voltage_domains[0].nominal_millivolts = 1_800;
    spec.spi_buses[0].speed_hz = 7_000_000;
    spec.accept_lifecycle = acknowledge(&["ICP10111", "ICM20948"]);
    spec.validate().unwrap();
    // The BME280 separates VDDIO (1.2 to 3.6 V): at 1.2 V its supply refuses
    // it first, at its own minimum the I/O range alone would not.
    let mut spec = valid();
    spec.voltage_domains[0].nominal_millivolts = 1_200;
    assert_refusal(
        spec.validate().unwrap_err(),
        "SupplyOutsidePart",
        Some("U2"),
    );
}

/// Adversarial 3: a table part named in another case, or the bare flash in
/// any case, is refused rather than read as an unlisted part.
#[test]
fn a_part_name_in_another_case_is_refused() {
    let mut spec = valid();
    devs(&mut spec).spi[0].part = "W25Q64JVXIQ".into();
    assert_refusal(spec.validate().unwrap_err(), "PartNameCase", Some("U4"));
    let mut spec = valid();
    devs(&mut spec).spi[0].part = "w25q64jv".into();
    assert_refusal(
        spec.validate().unwrap_err(),
        "AmbiguousPartName",
        Some("U4"),
    );
}

/// F-a and adversarial 5: a net constraint on MISO or a chip select is
/// refused, since nothing reads it.
#[test]
fn a_constraint_on_miso_or_a_chip_select_is_refused() {
    for net in ["SPI_MISO", "FLASH_CS"] {
        let mut spec = valid();
        let mut c = spec.net_constraints[0].clone();
        c.net_name = net.into();
        spec.net_constraints.push(c);
        assert_refusal(
            spec.validate().unwrap_err(),
            "UnjudgedSpiConstraint",
            Some(net),
        );
    }
}

/// Adversarial 1: a relabelled line cannot give one peripheral two rates or
/// two labels.
#[test]
fn a_peripheral_s_lines_share_one_label_and_one_rate() {
    for forge in [
        |t: &mut aethelgard_ir::SolvedTopology| {
            let l = t.spi_lines.get_mut("SPI_MOSI").unwrap();
            l.bus = "GHOST".into();
            l.rate_hz = 1_000_000;
        },
        |t: &mut aethelgard_ir::SolvedTopology| {
            t.spi_lines.get_mut("SPI_SCK").unwrap().bus = "GHOST".into();
        },
    ] {
        let mut t = solved();
        t.assignments.insert(
            "SPI_MOSI".to_owned(),
            PinAssignment {
                pin: "PA7".into(),
                peripheral: "SPI1".into(),
                signal: "SPI1_MOSI".into(),
                alternate_function: 5,
            },
        );
        t.spi_lines.insert("SPI_MOSI".to_owned(), spi_line("MOSI"));
        t.pin_speeds.insert("SPI_MOSI".to_owned(), 1);
        assert_eq!(t.validate(), Ok(()));
        forge(&mut t);
        assert_eq!(t.validate().unwrap_err().kind(), "SpiLineMismatch");
    }
}

fn acknowledge(parts: &[&str]) -> Vec<aethelgard_ir::LifecycleAcceptance> {
    parts
        .iter()
        .map(|p| aethelgard_ir::LifecycleAcceptance {
            part: (*p).into(),
            status: "eol".into(),
            statement: "a twin stand-in, never bought (AET-WO-4210 D1)".into(),
        })
        .collect()
}

/// The reference with U2 an ICP-10111 at 0x63 and U4 an ICM-20948, on a
/// 1.8 V rail: the two stand-ins, both `eol`.
fn stand_ins() -> Specification {
    let mut spec = valid();
    devs(&mut spec).spi[0].part = "ICM20948".into();
    devs(&mut spec).i2c.truncate(1);
    devs(&mut spec).i2c[0].part = "ICP10111".into();
    devs(&mut spec).i2c[0].address = 0x63;
    let v = serde_json::to_string(&spec)
        .unwrap()
        .replace("VDD_3V3", "VDD_1V8");
    let mut spec: Specification = serde_json::from_str(&v).unwrap();
    spec.voltage_domains[0].nominal_millivolts = 1_800;
    spec.spi_buses[0].speed_hz = 7_000_000;
    // The flash is gone, so its acknowledgement would stand for no part.
    spec.accept_lifecycle.clear();
    spec
}

/// `AET-WO-4210` D1, ruled by Scotty: an `eol` or `nrnd` part is used only
/// when `accept_lifecycle` acknowledges it, once, at the table's status, with
/// a statement; an acknowledgement stands only for a part the board uses as
/// such. Each refusal is by name.
#[test]
fn an_eol_part_is_refused_unless_acknowledged_at_its_status() {
    let spec = stand_ins();
    let err = spec.validate().unwrap_err();
    assert_refusal(err.clone(), "LifecycleNotAccepted", Some("U2"));
    let m = err.to_string();
    assert!(
        m.contains("ICP10111") && m.contains("`eol`") && m.contains("EOL badge"),
        "{m}"
    );

    let mut spec = stand_ins();
    spec.accept_lifecycle = acknowledge(&["ICP10111"]);
    assert_refusal(
        spec.validate().unwrap_err(),
        "LifecycleNotAccepted",
        Some("U4"),
    );

    let mut spec = stand_ins();
    spec.accept_lifecycle = acknowledge(&["ICP10111", "ICM20948"]);
    spec.accept_lifecycle[1].status = "nrnd".into();
    let err = spec.validate().unwrap_err();
    assert_refusal(err.clone(), "LifecycleStatusMismatch", Some("U4"));
    assert!(err.to_string().contains("as `nrnd`"), "{err}");

    let mut spec = stand_ins();
    spec.accept_lifecycle = acknowledge(&["ICP10111", "ICM20948", "ICP10111"]);
    assert_refusal(
        spec.validate().unwrap_err(),
        "LifecycleAcceptedTwice",
        Some("ICP10111"),
    );

    for bad in ["", "   ", "tab\there", &"x".repeat(513)] {
        let mut spec = stand_ins();
        spec.accept_lifecycle = acknowledge(&["ICP10111", "ICM20948"]);
        spec.accept_lifecycle[0].statement = bad.to_owned();
        assert_refusal(
            spec.validate().unwrap_err(),
            "LifecycleStatementInvalid",
            Some("ICP10111"),
        );
    }

    // An acknowledgement of a part the board does not use, or uses `active`.
    for extra in ["BME280", "W25Q128"] {
        let mut spec = stand_ins();
        spec.accept_lifecycle = acknowledge(&["ICP10111", "ICM20948", extra]);
        assert_refusal(
            spec.validate().unwrap_err(),
            "LifecycleAcceptanceUnused",
            Some(extra),
        );
    }

    let mut spec = stand_ins();
    spec.accept_lifecycle = acknowledge(&["ICM20948", "ICP10111"]);
    spec.validate().unwrap();
    // The banner: one line per part, in the devices' order, each naming its
    // status, devices and vendor citation.
    let banner = spec.lifecycle_banner().unwrap();
    assert_eq!(banner.len(), 2, "{banner:?}");
    assert!(
        banner[0].starts_with("LIFECYCLE EOL: ICP10111 (U2) is end of life"),
        "{banner:?}"
    );
    assert!(
        banner[1].starts_with("LIFECYCLE EOL: ICM20948 (U4) is end of life"),
        "{banner:?}"
    );
    assert!(banner.iter().all(|l| l.contains("EOL badge")), "{banner:?}");
    // A board of `active` parts has no banner: the reference without its
    // `nrnd` flash.
    let mut active = valid();
    devs(&mut active).spi.clear();
    assert!(active.lifecycle_banner().unwrap().is_empty());
}

/// `AET-WO-4211`: the W25Q64JV is `nrnd` by Winbond's catalog export. The
/// reference without its acknowledgement is refused by name, citing the
/// export; with it, it passes with the NRND banner. §5's adversarial cases
/// each meet `4210`'s rules: another case does not acknowledge the part, a
/// second entry is refused, and an entry for a part the board does not place
/// stands for nothing.
#[test]
fn the_nrnd_flash_is_refused_unless_acknowledged_and_bannered_when_it_is() {
    let mut spec = valid();
    spec.accept_lifecycle.clear();
    let err = spec.validate().unwrap_err();
    assert_refusal(err.clone(), "LifecycleNotAccepted", Some("U4"));
    let m = err.to_string();
    for word in [
        "W25Q64JVxIQ",
        "`nrnd`",
        "winbond-W25Q64JV-catalog-export-2026-11-16.csv",
        "Status-Industrial = N",
    ] {
        assert!(m.contains(word), "{word}: {m}");
    }

    let spec = valid();
    spec.validate().unwrap();
    let banner = spec.lifecycle_banner().unwrap();
    assert_eq!(banner.len(), 1, "{banner:?}");
    assert!(
        banner[0].starts_with(
            "LIFECYCLE NRND: W25Q64JVxIQ (U4) is not recommended for new designs by its vendor (Winbond Parts Catalog export"
        ),
        "{banner:?}"
    );

    // The part named in another case, or with a space: not the part.
    for name in ["w25q64jvxiq", "W25Q64JVXIQ", "W25Q64JVxIQ ", "W25Q64JVSSIQ"] {
        let mut spec = valid();
        spec.accept_lifecycle[0].part = name.into();
        assert_refusal(
            spec.validate().unwrap_err(),
            "LifecycleNotAccepted",
            Some("U4"),
        );
    }
    // The status in another case, or another status.
    for status in ["NRND", "eol", "active", ""] {
        let mut spec = valid();
        spec.accept_lifecycle[0].status = status.into();
        assert_refusal(
            spec.validate().unwrap_err(),
            "LifecycleStatusMismatch",
            Some("U4"),
        );
    }
    // Named twice, even with the same words.
    let mut spec = valid();
    let again = spec.accept_lifecycle[0].clone();
    spec.accept_lifecycle.push(again);
    assert_refusal(
        spec.validate().unwrap_err(),
        "LifecycleAcceptedTwice",
        Some("W25Q64JVxIQ"),
    );
    // A board that does not place it: the stand-ins' board, and the
    // reference with no device.
    let mut spec = stand_ins();
    spec.accept_lifecycle = acknowledge(&["ICP10111", "ICM20948"]);
    spec.accept_lifecycle
        .push(valid().accept_lifecycle[0].clone());
    assert_refusal(
        spec.validate().unwrap_err(),
        "LifecycleAcceptanceUnused",
        Some("W25Q64JVxIQ"),
    );
    let mut spec = valid();
    devs(&mut spec).i2c.clear();
    devs(&mut spec).spi.clear();
    spec.i2c_buses.clear();
    spec.spi_buses.clear();
    spec.net_constraints.clear();
    assert_refusal(
        spec.validate().unwrap_err(),
        "LifecycleAcceptanceUnused",
        Some("W25Q64JVxIQ"),
    );
}

/// An entry of `acknowledge_unwitnessed` for `register` and `class`.
fn unwitnessed(register: &str, class: &str, fields: &[&str]) -> UnwitnessedAcknowledgement {
    UnwitnessedAcknowledgement {
        register: register.into(),
        class: class.into(),
        fields: fields.iter().map(|f| (*f).into()).collect(),
        reason: "Renode does not store it (FEEDBACK s2): the bench row reads it".into(),
    }
}

/// `AET-WO-4230` R1 (the architect's ruling on PR #161, comment 6911424608):
/// `acknowledge_unwitnessed` is required, and each entry's shape is refused
/// by name at `validate`, in document order. The ledger's own register and
/// field names, and all three classes, pass.
#[test]
fn an_unwitnessed_acknowledgement_s_shape_is_refused_by_name() {
    let ok = [
        unwitnessed("SPI1.CFG1", "same-source", &["MBR", "DSIZE"]),
        unwitnessed("SPI1.CFG1", "ignored-by-model", &["CRCSIZE"]),
        unwitnessed("FLASH.OPTSR_CUR", "silicon-only", &["BOR_LEV"]),
        unwitnessed("SCB.CPACR", "same-source", &["CP10", "CP11"]),
        unwitnessed("USART1.BRR", "same-source", &["DIV_MANTISSA"]),
    ];
    let mut spec = valid();
    spec.acknowledge_unwitnessed = ok.to_vec();
    assert_eq!(spec.validate(), Ok(()));
    assert_eq!(
        spec.unwitnessed_acknowledgement("SPI1.CFG1", UnwitnessedClass::IgnoredByModel),
        Some(&ok[1])
    );
    assert_eq!(
        spec.unwitnessed_acknowledgement("SPI1.CFG1", UnwitnessedClass::SiliconOnly),
        None
    );
    for c in UnwitnessedClass::ALL {
        assert_eq!(UnwitnessedClass::parse(c.name()), Some(c));
        assert_eq!(serde_json::to_value(c).unwrap(), c.name());
    }

    let refused =
        |entries: Vec<UnwitnessedAcknowledgement>| refusal(|s| s.acknowledge_unwitnessed = entries);
    // A register not of the ledger's shape.
    for bad in [
        "",
        "SPI1",
        "spi1.CFG1",
        "SPI1.",
        ".CFG1",
        "SPI1.CFG1.MBR",
        "SPI1 .CFG1",
        "1SPI.CFG1",
        "SPI1.CFG-1",
    ] {
        let e = refused(vec![unwitnessed(bad, "same-source", &["MBR"])]);
        assert_refusal(e, "UnwitnessedRegisterMalformed", Some(bad));
    }
    // A class outside the three, the two witnessed classes included.
    for bad in [
        "independent-rule",
        "observed",
        "Same-Source",
        "same_source",
        "",
    ] {
        let e = refused(vec![unwitnessed("SPI1.CFG1", bad, &["MBR"])]);
        assert_refusal(e.clone(), "UnwitnessedClassUnknown", Some("SPI1.CFG1"));
        assert!(e.to_string().contains(&format!("class `{bad}`")), "{e}");
    }
    // No fields.
    let e = refused(vec![unwitnessed("SPI1.CFG1", "same-source", &[])]);
    assert_refusal(e, "UnwitnessedFieldsEmpty", Some("SPI1.CFG1"));
    // A field twice.
    let e = refused(vec![unwitnessed(
        "SPI1.CFG1",
        "same-source",
        &["MBR", "DSIZE", "MBR"],
    )]);
    assert_refusal(e.clone(), "UnwitnessedFieldTwice", Some("SPI1.CFG1"));
    assert!(e.to_string().contains("field `MBR`"), "{e}");
    // A reason by the repository's rule for a source (`board::source`): 1 to
    // 200 bytes of printable ASCII, no space at either end.
    for bad in [
        "",
        " ",
        "   ",
        " why",
        "why ",
        "tab\there",
        "µs",
        &"x".repeat(201),
    ] {
        let mut a = unwitnessed("SPI1.CFG1", "same-source", &["MBR"]);
        a.reason = bad.to_owned();
        let e = refused(vec![ok[0].clone(), a]);
        assert_eq!(e.kind(), "InvalidSource", "{bad:?}: {e}");
        assert_eq!(
            e.path(),
            Some("acknowledge_unwitnessed[1].reason"),
            "{bad:?}"
        );
    }
    let mut a = unwitnessed("SPI1.CFG1", "same-source", &["MBR"]);
    a.reason = "x".repeat(200);
    let mut spec = valid();
    spec.acknowledge_unwitnessed = vec![a];
    assert_eq!(spec.validate(), Ok(()));
    // Two entries for one register and class, even with other fields.
    let e = refused(vec![
        unwitnessed("SPI1.CFG1", "same-source", &["MBR"]),
        unwitnessed("SPI1.CFG2", "same-source", &["MBR"]),
        unwitnessed("SPI1.CFG1", "same-source", &["DSIZE"]),
    ]);
    assert_refusal(e.clone(), "UnwitnessedAcknowledgedTwice", Some("SPI1.CFG1"));
    assert!(e.to_string().contains("`same-source`"), "{e}");
}

/// `acknowledge_unwitnessed` and each entry's four keys are required
/// (`INV-06`); an unknown key is refused.
#[test]
fn an_unwitnessed_acknowledgement_s_keys_are_each_required() {
    let whole = serde_json::json!({"register": "SPI1.CFG1", "class": "same-source",
                                   "fields": ["MBR"], "reason": "why"});
    assert!(serde_json::from_value::<UnwitnessedAcknowledgement>(whole.clone()).is_ok());
    for key in ["register", "class", "fields", "reason"] {
        let mut cut = whole.clone();
        cut.as_object_mut().unwrap().remove(key);
        let err = serde_json::from_value::<UnwitnessedAcknowledgement>(cut).unwrap_err();
        assert!(
            err.to_string().contains(&format!("missing field `{key}`")),
            "{err}"
        );
    }
    let mut extra = whole;
    extra["note"] = serde_json::json!("x");
    assert!(serde_json::from_value::<UnwitnessedAcknowledgement>(extra).is_err());
    let mut v = serde_json::to_value(valid()).unwrap();
    v.as_object_mut().unwrap().remove("acknowledge_unwitnessed");
    let err = serde_json::from_value::<Specification>(v).unwrap_err();
    assert!(
        err.to_string()
            .contains("missing field `acknowledge_unwitnessed`"),
        "{err}"
    );
}

/// `AET-WO-4230`: the version-8 fields, each refused by name, never defaulted.
#[test]
fn solved_holds_the_controllers_to_what_solve_derives() {
    let err = solved_refusal(|t| t.bor_level = 4);
    assert_refusal(err, "BorLevelOutOfRange", None);
    for b in 0..=3 {
        let mut ok = solved();
        ok.bor_level = b;
        assert_eq!(ok.validate(), Ok(()));
    }
    // An SPI peripheral of `assignments` with no controller, and a
    // controller with no peripheral.
    let err = solved_refusal(|t| {
        t.spi_controllers.clear();
    });
    assert_refusal(err, "SpiControllerMismatch", Some("SPI1"));
    let err = solved_refusal(|t| {
        let c = t.spi_controllers["SPI1"].clone();
        t.spi_controllers.insert("SPI2".into(), c);
    });
    assert_refusal(err, "SpiControllerMismatch", Some("SPI2"));
    let spi = |edit: fn(&mut SolvedSpiController)| {
        solved_refusal(|t| edit(t.spi_controllers.get_mut("SPI1").unwrap()))
    };
    for err in [
        spi(|c| c.mbr = 0),
        spi(|c| c.mbr = 2),
        spi(|c| c.mbr = 9),
        spi(|c| c.kernel = SpiKernel::HsiKerCk),
        spi(|c| c.kernel_hz = 32_000_000),
        spi(|c| c.declared_hz = 19_999_999),
        spi(|c| c.achieved_hz = 16_239_744),
        spi(|c| c.achieved_max_hz = 16_000_000),
    ] {
        assert_refusal(err, "SpiControllerMismatch", Some("SPI1"));
    }
    let err = solved_refusal(|t| {
        t.i2c_timings.clear();
    });
    assert_refusal(err, "I2cTimingMismatch", Some("I2C1"));
    let i2c = |edit: fn(&mut SolvedI2cTiming)| {
        solved_refusal(|t| edit(t.i2c_timings.get_mut("I2C1").unwrap()))
    };
    for err in [
        i2c(|t| t.cr1 = 1 << 12),
        i2c(|t| t.kernel_hz = 32_000_000),
        i2c(|t| *t.timingr.get_mut("Fast400k").unwrap() += 1),
        i2c(|t| {
            t.timingr.remove("FastPlus1M");
        }),
        i2c(|t| {
            t.timingr.insert("HighSpeed3M4".into(), 0);
        }),
    ] {
        assert_refusal(err, "I2cTimingMismatch", Some("I2C1"));
    }
}

#[test]
fn a_version_7_file_is_refused_and_a_version_8_file_needs_every_new_key() {
    let mut v = serde_json::to_value(solved()).unwrap();
    let o = v.as_object_mut().unwrap();
    for key in ["spi_controllers", "i2c_timings", "bor_level"] {
        let mut missing = o.clone();
        missing.remove(key);
        let err = serde_json::from_value::<SolvedTopology>(missing.clone().into())
            .expect_err("a missing key");
        assert!(err.to_string().contains(key), "{err}");
        // The same file as version 7 parses and is refused by version.
        missing.insert("schema_version".into(), 7.into());
        let old: SolvedTopology = serde_json::from_value(missing.into()).unwrap();
        assert_refusal(
            old.validate().unwrap_err(),
            "UnsupportedSchemaVersion",
            None,
        );
    }
    // A repeated mode key is refused when parsing.
    let text = serde_json::to_string(&solved()).unwrap().replace(
        "\"Fast400k\":281481788",
        "\"Fast400k\":281481788,\"Fast400k\":281481788",
    );
    assert!(
        text.contains("\"Fast400k\":281481788,\"Fast400k\""),
        "{text}"
    );
    assert!(serde_json::from_str::<SolvedTopology>(&text).is_err());
}

/// `AET-WO-1206` §1.2 (`AET-WO-4100` F1): step 19 refuses an I2C line with
/// no net constraint, SDA before SCL, by `solve`'s kind, `id`, claimant and
/// message.
#[test]
fn an_i2c_line_with_no_constraint_is_refused_at_validate() {
    for (line, missing) in [("I2C_SDA", "I2C_SDA"), ("I2C_SCL", "I2C_SCL")] {
        let err = refusal(|s| s.net_constraints.retain(|c| c.net_name != line));
        assert_eq!(
            err,
            E::MissingBusConstraint {
                bus: "SENSORS".into(),
                net: missing.into()
            }
        );
        assert_refusal(err.clone(), "MissingBusConstraint", Some("SENSORS"));
        assert_eq!(err.claimants(), ["SENSORS"]);
    }
    let err = refusal(|s| s.net_constraints.retain(|c| !c.net_name.starts_with("I2C")));
    assert_eq!(
        err.to_string(),
        "I2C bus `SENSORS` has no net constraint on `I2C_SDA`, so it has no rise time to bound its speed"
    );
    // An SPI-only board needs none.
    let mut spec = valid();
    spec.i2c_buses.clear();
    devs(&mut spec).i2c.clear();
    spec.net_constraints
        .retain(|c| !c.net_name.starts_with("I2C"));
    assert_eq!(spec.validate(), Ok(()));
}

/// A2: a device named for a peripheral instance a solve can use, in any
/// case, is the twin's `InvalidDeviceName`, with no `id` as the twin prints
/// none. A timer the motors cannot use, or a port the MCU lacks, is free.
#[test]
fn a_device_named_for_a_peripheral_instance_is_refused() {
    for name in ["gpioa", "GPIOE", "Gpiok", "tim1", "TIM8", "tim17"] {
        for spi in [false, true] {
            let err = refusal(|s| {
                let mut d = devs(s);
                if spi {
                    d.spi[0].reference = name.into();
                } else {
                    d.i2c[0].reference = name.into();
                }
            });
            assert_refusal(err.clone(), "InvalidDeviceName", None);
            assert_eq!(
                err.to_string(),
                format!(
                    "device `{name}` cannot be a Renode name (a lower-case letter, then letters, digits or `_`) that no stock instance already has"
                )
            );
        }
    }
    for name in ["tim6", "tim7", "gpiol", "gpio", "tim"] {
        let mut spec = valid();
        devs(&mut spec).i2c[0].reference = name.into();
        assert_eq!(spec.validate(), Ok(()), "{name}");
    }
}

/// A4: a reference KiCad would take for one the emitter numbers. The MCU is
/// the first `U<n>` the specification does not spell, so `u2` is free until
/// `U1` is spelled; any `R<n>` or `C<n>` from 1 in another spelling is held.
/// The `id` is the pair, as the emitter's and the board writer's refusals
/// name it.
#[test]
fn a_reference_the_emitter_numbers_in_another_spelling_is_refused() {
    for (name, id) in [
        ("u1", "U1 and u1"),
        ("U01", "U01 and U1"),
        ("r1", "R1 and r1"),
        ("R01", "R01 and R1"),
        ("c007", "C7 and c007"),
        ("r99", "R99 and r99"),
    ] {
        let err = refusal(|s| devs(s).i2c[0].reference = name.into());
        assert_refusal(err.clone(), "DuplicateReference", Some(id));
        assert_eq!(
            err.to_string(),
            format!("`{id}` is used twice as a reference")
        );
    }
    for name in ["u2", "U1", "R1", "C5", "r0", "R00", "U0", "Ux1"] {
        let mut spec = valid();
        devs(&mut spec).i2c[0].reference = name.into();
        assert_eq!(spec.validate(), Ok(()), "{name}");
    }
    // With `U1` spelled, the emitter numbers the MCU `U2`, which `u2` then
    // meets; `u5` it never reaches.
    let err = refusal(|s| {
        let mut d = devs(s);
        d.i2c[0].reference = "U1".into();
        d.i2c[1].reference = "u2".into();
    });
    assert_refusal(err, "DuplicateReference", Some("U2 and u2"));
    let mut spec = valid();
    {
        let mut d = devs(&mut spec);
        d.i2c[0].reference = "U1".into();
        d.i2c[1].reference = "u5".into();
    }
    assert_eq!(spec.validate(), Ok(()));
}

/// A5: a net or domain the emitter names for itself, in any case, or one
/// starting with KiCad's `unconnected-` (case-sensitive, as KiCad's prefix
/// match is), is the emitter's `ReservedName`.
#[test]
fn a_net_or_domain_the_emitter_names_is_refused() {
    fn rename(spec: &mut Specification, old: &str, new: &str) {
        let v = serde_json::to_string(&*spec)
            .unwrap()
            .replace(&format!("\"{old}\""), &format!("\"{new}\""));
        *spec = serde_json::from_str(&v).unwrap();
    }
    // The fixture's MCU is `U1`; U2 is a BME280 and U4 the flash.
    for (old, new, id) in [
        ("VDD_3V3", "GND", "GND"),
        ("I2C_SDA", "U1_VCAP1", "U1_VCAP1"),
        ("I2C_SDA", "u1_vcap2", "U1_VCAP2 and u1_vcap2"),
        ("SPI_MISO", "U2_SDO", "U2_SDO"),
        ("FLASH_CS", "U4_WP", "U4_WP"),
        ("VDD_3V3", "u4_hold", "U4_HOLD and u4_hold"),
        ("SPI_MISO", "unconnected-x", "unconnected-x"),
    ] {
        let mut spec = valid();
        rename(&mut spec, old, new);
        let err = spec.validate().unwrap_err();
        assert_refusal(err.clone(), "ReservedName", Some(id));
        assert_eq!(
            err.to_string(),
            format!("`{id}` names a net, rail or reference the emitter uses for itself; rename it")
        );
    }
    // Not the emitter's: a third VCAP, a pin of another part, another U's
    // VCAP, the battery net without a power tree, KiCad's prefix in another
    // case.
    for (old, new) in [
        ("I2C_SDA", "U1_VCAP3"),
        ("I2C_SDA", "U3_SDO"),
        ("I2C_SDA", "U2_REGOUT"),
        ("I2C_SDA", "U5_VCAP1"),
        ("I2C_SDA", "VBAT"),
        ("I2C_SDA", "Unconnected-x"),
    ] {
        let mut spec = valid();
        rename(&mut spec, old, new);
        assert_eq!(spec.validate(), Ok(()), "{new}");
    }
}

/// A7: a project name whose stem is a Windows reserved device name, in any
/// case, names files Windows cannot open. Only the project name becomes a
/// file name (`generate`'s `kicad/<project>.*` and `board/<project>.repl`,
/// the fabrication package's `<project>-*`).
#[test]
fn a_project_named_for_a_windows_device_is_refused() {
    // `com0` and `lpt0` (`AET-WO-1207` §1.3, `1206` F7) passed before, in
    // the list below (`4c61fad`).
    for name in [
        "con", "prn", "aux", "nul", "com1", "com9", "lpt1", "lpt9", "com0", "lpt0",
    ] {
        let err = refusal(|s| s.project_name = name.into());
        assert_invalid_identifier(err.clone(), "project_name");
        assert!(err.to_string().contains("Windows"), "{err}");
        // The message lists the range each name is in (adversarial: it
        // listed `com1` to `com9` while refusing `com0`).
        assert!(
            err.to_string()
                .contains("`con`, `prn`, `aux`, `nul`, `com0` to `com9`, `lpt0` to `lpt9`"),
            "{err}"
        );
    }
    for name in [
        "com10",
        "com00",
        "lpt01",
        "con-x",
        "console",
        "nul1",
        "aux-board",
    ] {
        let mut spec = valid();
        spec.project_name = name.into();
        assert_eq!(spec.validate(), Ok(()), "{name}");
    }
    // The stem is what Windows matches: before the first `.`, without
    // trailing spaces, in any case.
    for name in [
        "CON",
        "Con.",
        "con.txt",
        "con .txt",
        "nul.tar.gz",
        "COM1 ",
        "com0.txt",
        "LPT0 ",
    ] {
        assert!(aethelgard_ir::names::windows_reserved(name), "{name}");
    }
    for name in ["cons", ".con", "com", "lpt10", "con_x"] {
        assert!(!aethelgard_ir::names::windows_reserved(name), "{name}");
    }
}

/// The adversarial pass's F1: two of the specification's references that
/// KiCad takes for one, differing in more than case (a leading zero), are
/// the board writer's `DuplicateReference`, by its sorted pair.
#[test]
fn two_references_with_one_kicad_key_are_refused() {
    let err = refusal(|s| devs(s).i2c[1].reference = "U02".into());
    assert_refusal(err, "DuplicateReference", Some("U02 and U2"));
    let err = refusal(|s| devs(s).spi[0].reference = "u002".into());
    assert_refusal(err, "DuplicateReference", Some("U2 and u002"));
    // Case alone is step 8's refusal, as before.
    let err = refusal(|s| devs(s).i2c[1].reference = "u2".into());
    assert_eq!(
        err,
        E::DuplicateReference {
            id: "u2".into(),
            first: "U2".into()
        }
    );
}

/// Sets the I2C lines' declared rise times.
fn rise_times(spec: &mut Specification, sda: u32, scl: u32) {
    for c in &mut spec.net_constraints {
        match c.net_name.as_str() {
            "I2C_SDA" => c.max_rise_time_nanoseconds = sda,
            "I2C_SCL" => c.max_rise_time_nanoseconds = scl,
            _ => {}
        }
    }
}

/// `AET-WO-1207` §1.2 (`1206` F3): step 21 refuses an I2C bus whose worse
/// line no mode allows, by `solve`'s kind, `id`, claimant and message, at the
/// boundary both ways. Before, `validate` passed it and `solve` refused it
/// (`4c61fad`; the CLI pins hold `1178ca8`'s bytes).
#[test]
fn aet_1207_step_21_refuses_a_rise_time_no_mode_allows() {
    for (sda, scl, worse) in [
        (1000, 1001, 1001),
        (1001, 1000, 1001),
        (300, u32::MAX, u32::MAX),
    ] {
        let err = refusal(|s| rise_times(s, sda, scl));
        assert_refusal(err.clone(), "RiseTimeTooSlow", Some("SENSORS"));
        assert_eq!(err.claimants(), ["SENSORS"]);
        assert_eq!(
            err.to_string(),
            format!(
                "I2C bus `SENSORS` allows a rise time of {worse} ns, slower than any I2C mode (1000 ns at most)"
            )
        );
    }
    let mut spec = valid();
    spec.i2c_buses[0].speed_hz = 100_000;
    rise_times(&mut spec, 1000, 1000);
    assert_eq!(spec.validate(), Ok(()));
}

/// The MCU rail at `nominal` and `tolerance`, with no device (each part's
/// supply would bound it first, `SupplyOutsidePart`), so no acknowledgement
/// of one (`AET-WO-4211`), and, unless `spi`, no SPI bus either.
fn bare_rail(nominal: u32, tolerance: u16, spi: bool) -> Specification {
    let mut spec = valid();
    spec.voltage_domains[0].nominal_millivolts = nominal;
    spec.voltage_domains[0].tolerance_permille = tolerance;
    devs(&mut spec).i2c.clear();
    devs(&mut spec).spi.clear();
    spec.accept_lifecycle.clear();
    if !spi {
        spec.spi_buses.clear();
        spec.net_constraints
            .retain(|c| !c.net_name.starts_with("SPI_"));
    }
    spec
}

/// Step 21 refuses an SPI bus on an MCU supply outside DS12117 Table 149's
/// bands, 1620 to 3600 mV, across the whole tolerance (the minimum rounded
/// down, the maximum up), at each edge both ways; with no SPI bus there is
/// nothing to judge, as in `solve`. Before, `validate` passed each.
#[test]
fn aet_1207_step_21_refuses_an_spi_bus_off_the_speed_table() {
    // 1800 mV ± 10.0 % is 1620 to 1980; ± 10.1 % is 1618 to 1982.
    // 3000 mV ± 20.0 % is 2400 to 3600; ± 20.1 % is 2397 to 3603.
    assert_eq!(bare_rail(3000, 200, true).validate(), Ok(()));
    // 1620 mV passes the speed table's edge (step 21), and the MCU's own
    // floor refuses it: `bare_rail`'s `bor_level` 0 sets it at VBOR0's
    // 1680 mV (step 22, `AET-WO-1208` F1/F5).
    let err = bare_rail(1800, 100, true).validate().unwrap_err();
    assert_refusal(err, "DomainOutsideConsumer", Some("VDD_3V3"));
    for (nominal, tolerance, min, max) in [(1800, 101, 1618, 1982), (3000, 201, 2397, 3603)] {
        let err = bare_rail(nominal, tolerance, true).validate().unwrap_err();
        assert_refusal(err.clone(), "SupplyOutsideSpeedTable", Some("FLASH"));
        assert_eq!(err.claimants(), ["FLASH"]);
        assert_eq!(
            err.to_string(),
            format!(
                "SPI bus `FLASH` is on a supply of {min} to {max} mV, outside DS12117 Table 149's 1620 to 3600 mV, so no output speed can be judged for it"
            )
        );
        // With no SPI bus, step 22 judges the same rail by the MCU's own
        // consumer rule (`AET-WO-1208` F1/F5); before, it passed (`d539bb1`).
        let err = bare_rail(nominal, tolerance, false).validate().unwrap_err();
        assert_refusal(err, "DomainOutsideConsumer", Some("VDD_3V3"));
    }
}

/// Step 21 refuses an SPI bus with no net constraint on its SCK, then its
/// MOSI, naming the net; in `solve`'s order, after the I2C buses' rise times
/// and, on one bus, after its supply. Before, `validate` passed each.
#[test]
fn aet_1207_step_21_refuses_an_spi_line_with_no_constraint() {
    for net in ["SPI_SCK", "SPI_MOSI"] {
        let err = refusal(|s| s.net_constraints.retain(|c| c.net_name != net));
        assert_refusal(err.clone(), "MissingSpiConstraint", Some("FLASH"));
        assert_eq!(err.claimants(), ["FLASH"]);
        assert_eq!(
            err.to_string(),
            format!(
                "SPI bus `FLASH` has no net constraint on `{net}`, so its pin's output speed has no declared load to be chosen for (`AET-WO-4300c` D2)"
            )
        );
    }
    let err = refusal(|s| {
        s.net_constraints.retain(|c| c.net_name != "SPI_SCK");
        rise_times(s, 300, 1001);
    });
    assert_eq!(err.kind(), "RiseTimeTooSlow", "{err}");
    let mut spec = bare_rail(3000, 201, true);
    spec.net_constraints.retain(|c| c.net_name != "SPI_SCK");
    assert_eq!(
        spec.validate().unwrap_err().kind(),
        "SupplyOutsideSpeedTable"
    );
}

/// `bare_rail`'s board (no device, no SPI bus) with the MCU at `bor_level`.
fn bor_rail(nominal: u32, tolerance: u16, bor_level: u8) -> Specification {
    let mut spec = bare_rail(nominal, tolerance, false);
    spec.mcu.bor_level = bor_level;
    spec
}

/// `AET-WO-1208` F1/F5 (the architect's ruling on PR #182): without a power
/// tree, step 22 judges the MCU's rail by step 16's consumer rule, its
/// minimum above floor = max(1620 mV, the `bor_level`'s VBOR falling
/// maximum: VBOR3 2680 mV, VBOR0 1680 mV, DS12117 Table 115), its maximum at
/// most 3600 mV, as `DomainOutsideConsumer`. The rule is strict ("its minimum
/// above the floor"): a minimum exactly at the floor is refused. Before
/// (`88ca5b0`), every one of these passed.
#[test]
fn aet_1208_without_a_tree_the_mcu_rail_is_held_above_its_bor_floor() {
    // bor_level 3: minimums 2679 and 2680 refused, 2681 passes (2707, 2708,
    // 2709 mV ± 1 %); bor_level 0: 1620 (1800 ± 10 %) and 1680 refused, 1681
    // passes (1697, 1698 ± 1 %).
    for (nominal, tolerance, level, min, floor) in [
        (2707, 10, 3, 2679, 2680),
        (2708, 10, 3, 2680, 2680),
        (1800, 100, 0, 1620, 1680),
        (1697, 10, 0, 1680, 1680),
    ] {
        let err = bor_rail(nominal, tolerance, level).validate().unwrap_err();
        assert_refusal(err.clone(), "DomainOutsideConsumer", Some("VDD_3V3"));
        let text = err.to_string();
        assert!(text.contains(&format!("the domain's {min} to ")), "{text}");
        assert!(
            text.contains(&format!("the consumer's floor of {floor} mV")),
            "{text}"
        );
        assert!(
            text.contains("Table 111") && text.contains(&format!("VBOR{level}")),
            "{text}"
        );
    }
    for (nominal, level) in [(2709, 3), (1698, 0)] {
        assert_eq!(bor_rail(nominal, 10, level).validate(), Ok(()), "{nominal}");
    }
}

/// `AET-WO-4400b` D2: `pin_speeds` holds exactly the nets of `assignments`
/// and `gpio`, each `0` or `1`; a stray net, a missing one and a value of
/// `2` or `3` are each refused by name.
#[test]
fn pin_speeds_hold_every_solved_pin_once_at_0_or_1() {
    assert_eq!(solved().validate(), Ok(()));
    let err = solved_refusal(|t| {
        t.pin_speeds.remove("FLASH_CS");
    });
    assert_refusal(err, "PinSpeedMismatch", Some("FLASH_CS"));
    let err = solved_refusal(|t| {
        t.pin_speeds.insert("GHOST".to_owned(), 0);
    });
    assert_refusal(err, "PinSpeedMismatch", Some("GHOST"));
    for v in [2, 3] {
        let err = solved_refusal(|t| {
            t.pin_speeds.insert("I2C_SCL".to_owned(), v);
        });
        assert_refusal(err, "PinSpeedMismatch", Some("I2C_SCL"));
    }
    // `AET-WO-4400b2` F4: a stored speed is `solve`'s, re-derived; the
    // reference SCK at 20 MHz and 30 pF is `01`.
    let err = solved_refusal(|t| {
        t.pin_speeds.insert("SPI_SCK".to_owned(), 0);
    });
    assert_refusal(err, "PinSpeedNotSolved", Some("SPI_SCK"));
    let err = solved_refusal(|t| {
        t.pin_speeds.insert("FLASH_CS".to_owned(), 1);
    });
    assert_refusal(err, "PinSpeedNotSolved", Some("FLASH_CS"));
}

/// `AET-WO-4400b2` F4, moved from svd-hal's
/// `a_line_no_selectable_speed_serves_is_refused`: a solved SPI line whose
/// rate needs `10` or `11`, or whose load is above the table, is refused by
/// the topology itself, by the speed table's own kind, at every reader;
/// nothing is sped up.
#[test]
fn a_line_no_selectable_speed_serves_is_refused_by_the_topology() {
    // The line and its controller sped up together, so the controller
    // agrees and the speed is what is judged.
    let err = solved_refusal(|t| {
        t.spi_lines.get_mut("SPI_SCK").unwrap().rate_hz = 80_000_001;
        let c = t.spi_controllers.get_mut("SPI1").unwrap();
        (c.declared_hz, c.mbr, c.achieved_hz, c.achieved_max_hz) =
            (80_000_001, 0, 32_000_000, 32_479_488);
    });
    assert_refusal(err.clone(), "RateAbovePinSpeed", Some("SPI_SCK"));
    assert!(err.to_string().contains("`SPI_SCK` on `PA5`"), "{err}");
    let err = solved_refusal(|t| {
        t.spi_lines
            .get_mut("SPI_SCK")
            .unwrap()
            .capacitance_femtofarads = 50_001;
    });
    assert_refusal(err, "LoadAboveSpeedTable", Some("SPI_SCK"));
    let err = solved_refusal(|t| {
        t.spi_lines.get_mut("SPI_SCK").unwrap().rise_time_ns = 4;
    });
    assert_refusal(err, "RiseTimeBelowPinSpeed", Some("SPI_SCK"));
    let err = solved_refusal(|t| {
        t.spi_lines.get_mut("SPI_SCK").unwrap().vdd_min_millivolts = 1600;
    });
    assert_refusal(err, "SupplyOutsideSpeedTable", Some("SPI_SCK"));
}

// ------------------------------------------------- AET-WO-4220b: declared pin capacitance

fn declared(femtofarads: u64) -> Option<aethelgard_ir::PinCapacitance> {
    Some(aethelgard_ir::PinCapacitance {
        declared: aethelgard_ir::DeclaredPinCapacitance {
            femtofarads,
            condition: "each pin at most UM10204's Ci".into(),
            source: "UM10204 rev. 7.0 Table 10, p. 43".into(),
        },
    })
}

/// §1.2: a declaration within the parts table's own range (1 to 100 000 fF)
/// is accepted on a device whose part cites no maximum (on I2C, from
/// UM10204's `Ci` up); outside it, refused by name (step 14).
#[test]
fn a_declared_pin_capacitance_is_within_the_tables_range() {
    for ff in [10_000, 12_000, 100_000] {
        let mut spec = valid();
        devs(&mut spec).i2c[0].pin_capacitance = declared(ff);
        assert_eq!(spec.validate(), Ok(()), "{ff}");
    }
    for ff in [0, 100_001, u64::MAX] {
        let e = refusal(|s| devs(s).i2c[1].pin_capacitance = declared(ff));
        assert_refusal(e.clone(), "PinCapacitanceOutOfRange", Some("U3"));
        assert!(
            e.to_string()
                .contains(&format!("{ff} fF, not 1 to 100000 fF")),
            "{e}"
        );
    }
}

/// Adversarial A1: on an I2C device a declaration below UM10204's `Ci`
/// would lower the line's bound under what every undeclared I2C pin is
/// counted at, by assertion alone; it is refused.
#[test]
fn an_i2c_declaration_never_falls_below_um10204_s_ci() {
    for ff in [1, 5_000, 9_999] {
        let e = refusal(|s| devs(s).i2c[0].pin_capacitance = declared(ff));
        assert_refusal(e.clone(), "PinCapacitanceBelowI2cCi", Some("U2"));
        assert!(
            e.to_string()
                .contains(&format!("{ff} fF, below UM10204's Ci of 10000 fF")),
            "{e}"
        );
    }
}

/// §1.2 (`AET-WO-4300c` D3): a part whose row cites a maximum is counted at
/// it, and a declaration beside it, smaller or larger, is refused.
#[test]
fn a_declaration_never_stands_beside_a_cited_maximum() {
    for ff in [1_000, 8_000, 20_000] {
        let e = refusal(|s| devs(s).spi[0].pin_capacitance = declared(ff));
        assert_refusal(e.clone(), "PinCapacitanceAlreadyCited", Some("U4"));
        assert!(e.to_string().contains("cites a maximum of 8000 fF"), "{e}");
    }
}

/// Step 2: a declaration's condition and source are each held to the
/// repository's rule for a source, by path.
#[test]
fn a_declarations_texts_are_sources_by_path() {
    for bad in [
        "",
        " ",
        " lead",
        "trail ",
        "tab\there",
        "µF",
        &"x".repeat(201),
    ] {
        for (field, path) in [
            (
                "condition",
                "i2c_devices[0].pin_capacitance.declared.condition",
            ),
            ("source", "i2c_devices[0].pin_capacitance.declared.source"),
        ] {
            let e = refusal(|s| {
                let mut c = declared(10_000);
                let d = &mut c.as_mut().unwrap().declared;
                if field == "condition" {
                    d.condition = bad.to_owned();
                } else {
                    d.source = bad.to_owned();
                }
                devs(s).i2c[0].pin_capacitance = c;
            });
            assert_eq!(e.kind(), "InvalidSource", "{bad:?}: {e}");
            assert_eq!(e.path(), Some(path), "{bad:?}");
        }
    }
    let e = refusal(|s| {
        let mut c = declared(10_000);
        c.as_mut().unwrap().declared.source = String::new();
        devs(s).spi[0].pin_capacitance = c;
    });
    assert_eq!(
        e.path(),
        Some("spi_devices[0].pin_capacitance.declared.source")
    );
}

/// Version 13: every device carries the key, `null` or a declaration, and
/// nothing supplies it; a version-12 file is refused by its version, with
/// or without the key; a declaration holds exactly its three fields.
#[test]
fn every_device_carries_pin_capacitance_and_a_version_12_file_is_refused() {
    let json = serde_json::to_value(valid()).unwrap();
    for (list, i) in [("i2c_devices", 0), ("spi_devices", 0)] {
        let mut v = json.clone();
        v[list][i]
            .as_object_mut()
            .unwrap()
            .remove("pin_capacitance");
        assert!(
            serde_json::from_value::<Specification>(v).is_err(),
            "{list}"
        );
    }
    let mut v = json.clone();
    v["i2c_devices"][0]["pin_capacitance"] = serde_json::json!({"declared": {"femtofarads": 1, "condition": "c", "source": "s", "maximum": 1}});
    assert!(serde_json::from_value::<Specification>(v).is_err());
    let mut v = json.clone();
    v["i2c_devices"][0]["pin_capacitance"] = serde_json::json!({"maximum_femtofarads": 1});
    assert!(serde_json::from_value::<Specification>(v).is_err());
    let mut v = json;
    v["schema_version"] = 12.into();
    let e = serde_json::from_value::<Specification>(v)
        .unwrap()
        .validate()
        .unwrap_err();
    assert!(
        matches!(
            e,
            E::UnsupportedSchemaVersion {
                version: 12,
                expected: 18
            }
        ),
        "{e}"
    );
}

/// `AET-WO-4220b` H2 (ruled on #192, Q2): on any bus, a declaration below
/// the part's cited typical (`parts.json` `pin_capacitance.typical_femtofarads`)
/// is refused by name, citing the row; at or above it, accepted.
#[test]
fn a_declaration_never_falls_below_the_parts_cited_typical() {
    // The flash becomes an ICM-42688-P, so its `nrnd` acknowledgement goes.
    let icm = |s: &mut Specification, ff: u64| {
        s.accept_lifecycle.clear();
        let mut d = devs(s);
        d.spi[0].part = "ICM42688P".into();
        d.spi[0].pin_capacitance = declared(ff);
    };
    for ff in [1, 9_999] {
        let e = refusal(|s| icm(s, ff));
        assert_refusal(e.clone(), "PinCapacitanceBelowTypical", Some("U4"));
        assert!(
            e.to_string().contains(&format!(
                "{ff} fF, below the typical of 10000 fF its part `ICM42688P` cites"
            )) && e.to_string().contains("DS-000347"),
            "{e}"
        );
    }
    for ff in [10_000, 20_000] {
        let mut spec = valid();
        icm(&mut spec, ff);
        assert_eq!(spec.validate(), Ok(()), "{ff}");
    }
}
