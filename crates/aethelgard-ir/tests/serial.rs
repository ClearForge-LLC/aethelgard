//! `AET-WO-4300b` §1.1–§1.3 and adversarial items 2 and 3: serial ports
//! declared, validated, placed on USARTs, their BRR solved and checked, and
//! their topology judged, on a small synthetic `PinTable` (this crate may
//! depend on no other member; the real table is `solve_cli.rs`'s).

use std::collections::BTreeMap;

use aethelgard_ir::serial::{
    CRSF_ELRS_SOURCE, CRSF_SOURCE, PERIPHERALS, SolvedSerialPort, check_silicon, solve_brr,
    tolerance, within,
};
use aethelgard_ir::{
    Framing, IrValidationError, Parity, Pin, PinSource, PinTable, SerialRole, SolveError,
    SolvedClock, SolvedTopology, Specification, solve,
};
use serde_json::{Value, json};

const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// The reference specification's declared board (`examples/sentinel-01-minimal.json`).
const BOARD: &str = r#"{"copper_layers": 2, "layers": [{"name": "F.Cu", "role": "signal", "net": null}, {"name": "B.Cu", "role": "signal", "net": null}], "stackup": {"finished_thickness_um": {"nominal": 1600, "min": 340, "max": 2000, "source": "AET-WO-1305 conditions: 1.6 mm declared; 0.34 mm = 2 x 70 um + 0.2 mm; 2.0 mm = 1.6 mm + 25 %"}, "dielectrics": [{"between": ["F.Cu", "B.Cu"], "min_thickness_um": 200, "source": "implied: the finished minimum 340 um less twice the copper maximum 70 um"}], "copper_max_um": {"value": 70, "source": "AET-WO-1305 condition: twice KiCad's default 35 um (2 oz)"}, "relative_permittivity_max_permille": {"value": 5400, "source": "the IPC-4101 FR-4 maximum, specified at 1 MHz"}}, "rules": {"clearance_um": 200, "track_width_um": 200, "via_drill_um": 300, "via_diameter_um": 600}}"#;

/// An edit of a port or its framing.
type Edit = fn(&mut Value);

type Pins<'a> = &'a [(&'a str, &'a [(&'a str, u8)])];

/// USART1 on PA9/PA10 and PB6/PB7 (beside I2C1 there; PA13 reserved),
/// LPUART1 also on PA9/PA10, USART2 on PA2/PA3, USART3 and UART4 sharing
/// PC10/PC11, UART4 alone on PA0/PA1; I2C1 also on PB8/PB9 and SPI1 on
/// PA5..PA7, neither with a USART. No UART5, USART6, UART7 or UART8.
const PINS: Pins = &[
    ("PA0", &[("UART4_TX", 8)]),
    ("PA1", &[("UART4_RX", 8)]),
    ("PA10", &[("USART1_RX", 7), ("LPUART1_RX", 3)]),
    ("PA13", &[("USART1_TX", 7)]),
    ("PA2", &[("USART2_TX", 7)]),
    ("PA3", &[("USART2_RX", 7)]),
    ("PA5", &[("SPI1_SCK", 5)]),
    ("PA6", &[("SPI1_MISO", 5)]),
    ("PA7", &[("SPI1_MOSI", 5)]),
    ("PA9", &[("USART1_TX", 7), ("LPUART1_TX", 3)]),
    ("PB6", &[("I2C1_SCL", 4), ("USART1_TX", 7)]),
    ("PB7", &[("I2C1_SDA", 4), ("USART1_RX", 7)]),
    ("PB8", &[("I2C1_SCL", 4)]),
    ("PB9", &[("I2C1_SDA", 4)]),
    ("PC10", &[("USART3_TX", 7), ("UART4_TX", 8)]),
    ("PC11", &[("USART3_RX", 7), ("UART4_RX", 8)]),
];

fn table_of(pins: Pins) -> PinTable {
    PinTable {
        part: "STM32H753ZITx".into(),
        package: "TEST".into(),
        source: PinSource {
            repository: "test".into(),
            commit: "test".into(),
            files: BTreeMap::new(),
        },
        pins: pins
            .iter()
            .enumerate()
            .map(|(i, (name, afs))| {
                let pin = Pin {
                    position: i as u16 + 1,
                    alternate_functions: afs.iter().map(|(s, af)| ((*s).into(), *af)).collect(),
                    adc_channels: Default::default(),
                };
                ((*name).into(), pin)
            })
            .collect(),
    }
}

fn table() -> PinTable {
    table_of(PINS)
}

/// A port at 115200 8N1, not inverted, role `console`, nets `<label>_TX`
/// and `<label>_RX`.
fn port(label: &str, peripheral: Option<&str>, tx: Option<&str>, rx: Option<&str>) -> Value {
    json!({"label": label, "peripheral": peripheral,
           "tx": {"net": format!("{label}_TX"), "pin": tx},
           "rx": {"net": format!("{label}_RX"), "pin": rx},
           "role": "console",
           "framing": {"baud": 115200, "data_bits": 8, "parity": "none", "stop_bits": 1, "inverted": false},
           "negotiated": null, "peer_deviation_ppm": 0, "peer_deviation_source": "declared"})
}

/// A specification holding `ports`, each port's TX and RX on pins 1 and 2 of
/// its own four-pin connector `J10<n>` with an explicit map: each serial port
/// ends at a connector pin (the architect's ruling 1; adversarial F9).
fn spec_json(ports: Vec<Value>) -> Value {
    let mut connectors: Vec<Value> = ports
        .iter()
        .enumerate()
        .map(|(i, p)| {
            json!({"reference": format!("J{}", 100 + i), "part": "SM04B-GHS-TB", "standard": null,
                "pins": [
                    {"pin": "1", "net": p["tx"]["net"], "offboard": null},
                    {"pin": "2", "net": p["rx"]["net"], "offboard": null},
                    {"pin": "3", "net": null, "offboard": null},
                    {"pin": "4", "net": "GND", "offboard": null}]})
        })
        .collect();
    // The programming path and the supply input every board declares
    // (`AET-WO-4300g`), after the ports' own connectors.
    connectors.push(
        json!({"reference": "J90", "part": "SM06B-SRSS-TB", "standard": null,
        "pins": [
            {"pin": "1", "net": "VDD_3V3", "offboard": null},
            {"pin": "2", "net": null, "offboard": null},
            {"pin": "3", "net": null, "offboard": null},
            {"pin": "4", "net": "SWDIO", "offboard": null},
            {"pin": "5", "net": "SWCLK", "offboard": null},
            {"pin": "6", "net": "GND", "offboard": null}]}),
    );
    connectors.push(
        json!({"reference": "J91", "part": "SM04B-GHS-TB", "standard": null,
        "pins": [
            {"pin": "1", "net": "VDD_3V3", "offboard": null},
            {"pin": "2", "net": null, "offboard": null},
            {"pin": "3", "net": null, "offboard": null},
            {"pin": "4", "net": "GND", "offboard": null}]}),
    );
    json!({
        "schema_version": aethelgard_ir::SPECIFICATION_SCHEMA_VERSION,
        "accept_lifecycle": [],
        "acknowledge_unwitnessed": [],
        "project_name": "t",
        "board": serde_json::from_str::<Value>(BOARD).unwrap(),
        "mcu": {"part": "STM32H753ZITx", "supply": "VDD_3V3", "hse_hz": 0, "bor_level": 0,
            "core_supply": {"mode": "ldo", "source": "DS12117 Rev 10 sec. 3.5.1 p. 27"},
            "supply_source": {"connector": "J91", "pin": "1"}, "reserved_pins": []},
        "voltage_domains": [
            {"id": "VDD_3V3", "nominal_millivolts": 3300, "tolerance_permille": 50},
            {"id": "VDD_5V", "nominal_millivolts": 5000, "tolerance_permille": 50}],
        "i2c_buses": [], "spi_buses": [], "i2c_devices": [], "spi_devices": [],
        "net_constraints": [],
        "connectors": connectors,
        "net_classes": [],
        "diff_pairs": [],
        "motor_outputs": [],
        "power": null,
        "serial_ports": ports,
    })
}

/// Adds I2C bus `SENS` (nets `I2C_SDA`, `I2C_SCL`) with its constraints.
fn with_i2c(mut v: Value, sda: Option<&str>, scl: Option<&str>) -> Value {
    v["i2c_buses"] = json!([{"id": "SENS", "peripheral": null, "speed_hz": 400000,
        "pull_up_ohms": 2200, "domain": "VDD_3V3",
        "sda": {"net": "I2C_SDA", "pin": sda}, "scl": {"net": "I2C_SCL", "pin": scl}}]);
    let line = |net: &str| {
        json!({"net_name": net, "max_capacitance_femtofarads": 100000,
        "max_rise_time_nanoseconds": 300, "impedance_ohms": null})
    };
    v["net_constraints"] = json!([line("I2C_SDA"), line("I2C_SCL")]);
    v
}

fn parse(v: Value) -> Specification {
    serde_json::from_value(v).expect("a specification")
}

fn spec(ports: Vec<Value>) -> Specification {
    parse(spec_json(ports))
}

fn refused(v: Value) -> IrValidationError {
    parse(v).validate().expect_err("refused")
}

fn refused_ports(ports: Vec<Value>) -> IrValidationError {
    refused(spec_json(ports))
}

fn solved(v: Value) -> SolvedTopology {
    solve(&parse(v), &table(), HASH).expect("solved")
}

fn solve_refused(v: Value) -> SolveError {
    solve(&parse(v), &table(), HASH).expect_err("refused")
}

/// The peripheral and the TX and RX pins of port `label`.
fn placed(t: &SolvedTopology, label: &str) -> (String, String, String) {
    let p = &t.serial_ports[label];
    (
        p.peripheral.clone(),
        t.assignments[&p.tx].pin.clone(),
        t.assignments[&p.rx].pin.clone(),
    )
}

#[test]
fn every_key_of_a_serial_port_is_required() {
    for key in [
        "label",
        "peripheral",
        "tx",
        "rx",
        "role",
        "framing",
        "peer_deviation_ppm",
        "peer_deviation_source",
    ] {
        let mut p = port("C", None, None, None);
        p.as_object_mut().unwrap().remove(key);
        let e = serde_json::from_value::<Specification>(spec_json(vec![p])).unwrap_err();
        assert!(e.to_string().contains(key), "{key}: {e}");
    }
    for key in ["baud", "data_bits", "parity", "stop_bits", "inverted"] {
        let mut p = port("C", None, None, None);
        p["framing"].as_object_mut().unwrap().remove(key);
        let e = serde_json::from_value::<Specification>(spec_json(vec![p])).unwrap_err();
        assert!(e.to_string().contains(key), "{key}: {e}");
    }
    let mut v = spec_json(vec![]);
    v.as_object_mut().unwrap().remove("serial_ports");
    assert!(serde_json::from_value::<Specification>(v).is_err());
    for (field, bad) in [("role", json!("sbus")), ("role", json!("Console"))] {
        let mut p = port("C", None, None, None);
        p[field] = bad;
        assert!(serde_json::from_value::<Specification>(spec_json(vec![p])).is_err());
    }
    // `peer_deviation_source` is `cited` or `declared`, exactly.
    for bad in ["Cited", "datasheet", ""] {
        let mut p = port("C", None, None, None);
        p["peer_deviation_source"] = json!(bad);
        assert!(serde_json::from_value::<Specification>(spec_json(vec![p])).is_err());
    }
    let mut p = port("C", None, None, None);
    p["peer_deviation_source"] = json!(null);
    assert!(serde_json::from_value::<Specification>(spec_json(vec![p])).is_err());
    let mut p = port("C", None, None, None);
    p["framing"]["parity"] = json!("mark");
    assert!(serde_json::from_value::<Specification>(spec_json(vec![p])).is_err());
    let mut p = port("C", None, None, None);
    p["framing"]["extra"] = json!(1);
    assert!(serde_json::from_value::<Specification>(spec_json(vec![p])).is_err());
}

#[test]
fn a_serial_port_s_identifiers_are_refused_with_their_paths() {
    let cases: [(Edit, &str); 6] = [
        (|p| p["label"] = json!("C/1"), "serial_ports[0].label"),
        (
            |p| p["peripheral"] = json!("US ART1"),
            "serial_ports[0].peripheral",
        ),
        (|p| p["tx"]["net"] = json!("A{B}"), "serial_ports[0].tx.net"),
        (|p| p["tx"]["pin"] = json!("PA 9"), "serial_ports[0].tx.pin"),
        (|p| p["rx"]["net"] = json!(""), "serial_ports[0].rx.net"),
        (|p| p["rx"]["pin"] = json!("P%1"), "serial_ports[0].rx.pin"),
    ];
    for (edit, path) in cases {
        let mut p = port("C", None, None, None);
        edit(&mut p);
        let e = refused_ports(vec![p]);
        assert_eq!(e.kind(), "InvalidIdentifier", "{path}: {e}");
        assert_eq!(e.path(), Some(path));
    }
}

#[test]
fn labels_are_unique_with_case_folded() {
    let mut second = port("console", None, None, None);
    second["tx"]["net"] = json!("B_TX");
    second["rx"]["net"] = json!("B_RX");
    let e = refused_ports(vec![port("CONSOLE", None, None, None), second]);
    assert_eq!(
        e,
        IrValidationError::DuplicateSerialLabel {
            id: "console".into()
        }
    );
}

#[test]
fn adversarial_3_lpuart1_is_refused_by_its_own_name_in_any_case() {
    for name in ["LPUART1", "lpuart1", "LpUart1"] {
        let e = refused_ports(vec![port("C", Some(name), None, None)]);
        assert_eq!(e.kind(), "SerialPeripheralLpuart", "{name}");
        assert_eq!(e.id(), Some("C"));
        assert!(e.to_string().contains("RCC_APB4ENR"), "{e}");
        // The solver never sees it.
        let s = solve(
            &spec(vec![port("C", Some(name), None, None)]),
            &table(),
            HASH,
        );
        assert_eq!(s.unwrap_err().kind(), "SerialPeripheralLpuart");
    }
}

#[test]
fn a_peripheral_outside_the_list_is_refused_as_written() {
    for name in ["usart2", "USART9", "UART9", "SPI1", "USART"] {
        let e = refused_ports(vec![port("C", Some(name), None, None)]);
        assert_eq!(
            e,
            IrValidationError::UnknownSerialPeripheral {
                id: "C".into(),
                peripheral: name.into()
            }
        );
    }
    for name in PERIPHERALS {
        assert_eq!(
            spec(vec![port("C", Some(name), None, None)]).validate(),
            Ok(())
        );
    }
}

#[test]
fn adversarial_3_two_ports_on_one_usart_are_refused() {
    let e = refused_ports(vec![
        port("A", Some("USART2"), None, None),
        port("B", Some("USART2"), None, None),
    ]);
    assert_eq!(
        e,
        IrValidationError::SerialPeripheralTwice {
            id: "B".into(),
            first: "A".into(),
            peripheral: "USART2".into()
        }
    );
    // Two ports whose fixed pins leave each only USART1 are refused by the
    // solver, though each alone can be placed.
    let e = solve_refused(spec_json(vec![
        port("A", None, Some("PA9"), None),
        port("B", None, Some("PB6"), None),
    ]));
    assert_eq!(e.kind(), "SerialPortsInfeasible", "{e}");
    assert_eq!(e.claimants(), ["A", "B"]);
}

#[test]
fn a_framing_outside_what_the_firmware_writes_is_refused_by_field() {
    let cases: [(Edit, &str, u32); 7] = [
        (|f| f["baud"] = json!(0), "baud", 0),
        (|f| f["data_bits"] = json!(6), "data_bits", 6),
        (|f| f["data_bits"] = json!(10), "data_bits", 10),
        (|f| f["data_bits"] = json!(255), "data_bits", 255),
        (
            |f| {
                f["data_bits"] = json!(9);
                f["parity"] = json!("even");
            },
            "parity",
            10,
        ),
        (|f| f["stop_bits"] = json!(0), "stop_bits", 0),
        (|f| f["stop_bits"] = json!(3), "stop_bits", 3),
    ];
    for (edit, field, value) in cases {
        let mut p = port("C", None, None, None);
        edit(&mut p["framing"]);
        let e = refused_ports(vec![p]);
        assert!(
            matches!(&e, IrValidationError::SerialFramingShape { id, field: f, value: v, .. }
                if id == "C" && *f == field && *v == value),
            "{field}: {e}"
        );
    }
    // 7 data bits with parity is an 8-bit word; 9 without, a 9-bit word.
    for (data, parity) in [(7, "odd"), (9, "none"), (7, "none")] {
        let mut p = port("C", None, None, None);
        p["framing"]["data_bits"] = json!(data);
        p["framing"]["parity"] = json!(parity);
        assert_eq!(spec(vec![p]).validate(), Ok(()), "{data}{parity}");
    }
}

fn crsf(edit: fn(&mut Value)) -> Value {
    let mut p = port("RC", None, None, None);
    p["role"] = json!("rc_crsf");
    p["framing"]["baud"] = json!(420000);
    p["peer_deviation_ppm"] = json!(8002);
    p["peer_deviation_source"] = json!("cited");
    edit(&mut p);
    p
}

/// The PR #152 ruling on §9 Q1: an `rc_crsf` port's baud is one of an
/// enumerated set, each cited (416666, TBS; 420000, ExpressLRS and
/// Betaflight), or a rate the port declares `negotiated` with its source
/// (frame 0x70). Anything else is refused by name, 432000 among them; the
/// declaration is refused off `rc_crsf`, with an empty source, and at a cited
/// baud; a framing that contradicts TBS's text is refused citing it.
#[test]
fn a_crsf_port_s_baud_is_a_cited_one_or_declared_negotiated_and_the_rest_is_refused() {
    let negotiated = |p: &mut Value| {
        p["framing"]["baud"] = json!(432000);
        p["negotiated"] =
            json!({"source": "the receiver's CRSF 0x70 speed proposal, vendor manual section 4"});
    };
    for edit in [(|_: &mut Value| {}) as Edit, negotiated] {
        let p = crsf(edit);
        assert_eq!(spec(vec![p.clone()]).validate(), Ok(()), "{p}");
        // The declaration reaches the solved port.
        let t = solved(spec_json(vec![p.clone()]));
        assert_eq!(
            t.serial_ports["RC"].negotiated,
            serde_json::from_value(p["negotiated"].clone()).unwrap()
        );
        assert_eq!(t.validate(), Ok(()));
    }
    // 416666 validates, but no `BRR` on the 64 MHz HSI clock leaves room for
    // HSI's worst case and the cited 8,002 ppm of spread: 154 is 2,596 ppm
    // slow, HSI's 25,894 takes it to 28,423 ppm, and with 8,002 of spread that is
    // 36,425, not below Table 407's
    // 33,300. `validate` refuses it, naming each term (step 21,
    // `AET-WO-1208` §1.3; before, it passed and `solve` refused it), and so
    // does `solve`, which calls it first.
    let p = crsf(|p| p["framing"]["baud"] = json!(416666));
    let v = spec(vec![p.clone()]).validate().unwrap_err();
    let e = solve_refused(spec_json(vec![p]));
    assert_eq!(e, SolveError::InvalidSpecification(v.clone()));
    assert!(
        matches!(&v, IrValidationError::SerialBaudTolerance { port, quantization_ppm: 2_596,
            hsi_ppm: 25_894, receiver_ppm: 28_423, peer_deviation_ppm: 8_002,
            worst_case_ppm: 36_425,
            tolerance_ppm: 33_300, .. } if port == "RC"),
        "{e}"
    );
    // The solved topology is held to the same: a negotiated port's
    // declaration dropped, or its source emptied, disagrees with it.
    let good = solved(spec_json(vec![crsf(negotiated)]));
    for edit in [
        (|p: &mut SolvedSerialPort| p.negotiated = None) as fn(&mut SolvedSerialPort),
        |p| p.negotiated.as_mut().unwrap().source = String::new(),
    ] {
        let mut t = good.clone();
        edit(t.serial_ports.get_mut("RC").unwrap());
        let e = t.validate().unwrap_err();
        assert_eq!(
            (e.kind(), e.id()),
            ("SerialPortMismatch", Some("RC")),
            "{e}"
        );
    }
    // A rate neither cited nor declared negotiated: refused, citing every source.
    let e = refused_ports(vec![crsf(|p| p["framing"]["baud"] = json!(432000))]);
    assert!(
        matches!(&e, IrValidationError::SerialFramingContradictsRole {
            id, role: "rc_crsf", field: "baud", required, citation
        } if id == "RC" && required.contains("not 432000")
            && citation.contains(CRSF_SOURCE) && citation.contains(CRSF_ELRS_SOURCE)),
        "{e}"
    );
    // A framing TBS's text contradicts, negotiated or not.
    let cases: [(Edit, &str); 5] = [
        (|p| p["framing"]["baud"] = json!(400000), "baud"),
        (
            |p| {
                p["framing"]["baud"] = json!(400000);
                p["negotiated"] = json!({"source": "s"});
            },
            "baud",
        ),
        (|p| p["framing"]["parity"] = json!("even"), "framing"),
        (|p| p["framing"]["stop_bits"] = json!(2), "framing"),
        (|p| p["framing"]["inverted"] = json!(true), "inverted"),
    ];
    for (edit, field) in cases {
        let e = refused_ports(vec![crsf(edit)]);
        assert!(
            matches!(&e, IrValidationError::SerialFramingContradictsRole {
                id, role: "rc_crsf", field: f, citation, ..
            } if id == "RC" && *f == field && citation == CRSF_SOURCE),
            "{field}: {e}"
        );
    }
    // The declaration itself, each refusal by name.
    let declared: [(Edit, &str); 3] = [
        (
            |p| {
                p["role"] = json!("console");
                p["framing"]["baud"] = json!(115200);
                p["negotiated"] = json!({"source": "s"});
            },
            "SerialNegotiatedOffCrsf",
        ),
        (
            |p| p["negotiated"] = json!({"source": "s"}),
            "SerialNegotiatedAtCitedBaud",
        ),
        (
            |p| {
                p["framing"]["baud"] = json!(416666);
                p["negotiated"] = json!({"source": "s"});
            },
            "SerialNegotiatedAtCitedBaud",
        ),
    ];
    for (edit, kind) in declared {
        let e = refused_ports(vec![crsf(edit)]);
        assert_eq!((e.kind(), e.id()), (kind, Some("RC")), "{e}");
    }
    // The key is required, and its object takes only `source`.
    let mut missing = spec_json(vec![crsf(|_| {})]);
    missing["serial_ports"][0]
        .as_object_mut()
        .unwrap()
        .remove("negotiated");
    let e = serde_json::from_value::<Specification>(missing).unwrap_err();
    assert!(e.to_string().contains("missing field `negotiated`"), "{e}");
    let extra = spec_json(vec![crsf(|p| {
        p["framing"]["baud"] = json!(432000);
        p["negotiated"] = json!({"source": "s", "baud": 432000});
    })]);
    let e = serde_json::from_value::<Specification>(extra).unwrap_err();
    assert!(e.to_string().contains("unknown field `baud`"), "{e}");
    // SBUS has no vendor-published source: its usual framing, and any other,
    // stands as declared.
    let mut sbus = port("SB", None, None, None);
    sbus["role"] = json!("rc_sbus");
    sbus["framing"] =
        json!({"baud": 100000, "data_bits": 8, "parity": "even", "stop_bits": 2, "inverted": true});
    assert_eq!(spec(vec![sbus.clone()]).validate(), Ok(()));
    sbus["framing"]["baud"] = json!(115200);
    assert_eq!(spec(vec![sbus]).validate(), Ok(()));
}

#[test]
fn a_serial_net_is_a_net_and_takes_no_constraint() {
    let mut p = port("C", None, None, None);
    p["rx"]["net"] = json!("C_TX");
    assert_eq!(
        refused_ports(vec![p]),
        IrValidationError::DuplicateNetName { id: "C_TX".into() }
    );
    let mut p = port("C", None, None, None);
    p["tx"]["net"] = json!("I2C_SDA");
    assert_eq!(
        refused(with_i2c(spec_json(vec![p]), None, None)),
        IrValidationError::DuplicateNetName {
            id: "I2C_SDA".into()
        }
    );
    for net in ["GND", "gnd", "Gnd", "VDD_3V3", "vdd_5v"] {
        let mut p = port("C", None, None, None);
        p["rx"]["net"] = json!(net);
        assert_eq!(
            refused_ports(vec![p]),
            IrValidationError::NetShadowsDomain { id: net.into() },
            "{net}"
        );
    }
    let mut v = spec_json(vec![port("C", None, None, None)]);
    v["net_constraints"] = json!([{"net_name": "C_RX", "max_capacitance_femtofarads": 50000,
        "max_rise_time_nanoseconds": 100, "impedance_ohms": null}]);
    let e = refused(v);
    assert_eq!(
        e,
        IrValidationError::UnjudgedSerialConstraint { id: "C_RX".into() }
    );
}

/// A connector `J1` on `part` with `standard`, pins `1..` holding `nets`,
/// in place of [`spec_json`]'s connector of each port it holds a line of.
fn with_connector(
    mut v: Value,
    part: &str,
    standard: Option<&str>,
    nets: &[Option<&str>],
) -> Value {
    let offboard = |net: Option<&str>| {
        if net.is_some_and(|n| n.starts_with("I2C_")) {
            json!({"capacitance_femtofarads": 20000, "source": "condition: test"})
        } else {
            Value::Null
        }
    };
    let pins: Vec<Value> = nets
        .iter()
        .enumerate()
        .map(|(i, net)| json!({"pin": (i + 1).to_string(), "net": net, "offboard": offboard(*net)}))
        .collect();
    let mut connectors: Vec<Value> = v["connectors"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| {
            !c["pins"].as_array().unwrap().iter().any(|p| {
                p["net"].as_str().is_some_and(|n| {
                    (n.ends_with("_TX") || n.ends_with("_RX")) && nets.contains(&Some(n))
                })
            })
        })
        .cloned()
        .collect();
    // First, so the tests read it as `connectors[0]` beside the programming
    // path and supply input `spec_json` declares (`AET-WO-4300g`).
    connectors.insert(
        0,
        json!({"reference": "J1", "part": part, "standard": standard, "pins": pins}),
    );
    v["connectors"] = Value::Array(connectors);
    v
}

fn telem(tx: Option<&str>, rx: Option<&str>) -> Value {
    let ports = vec![port("A", None, None, None), port("B", None, None, None)];
    let nets = [Some("VDD_5V"), tx, rx, None, None, Some("GND")];
    with_connector(
        spec_json(ports),
        "SM06B-GHS-TB",
        Some("DS-009 TELEM"),
        &nets,
    )
}

#[test]
fn a_connector_ends_a_serial_net_with_no_off_board_capacitance() {
    let v = with_connector(
        spec_json(vec![port("C", None, None, None)]),
        "SM04B-GHS-TB",
        None,
        &[Some("C_TX"), Some("C_RX"), None, Some("GND")],
    );
    assert_eq!(parse(v.clone()).validate(), Ok(()));
    let mut v = v;
    v["connectors"][0]["pins"][1]["offboard"] =
        json!({"capacitance_femtofarads": 1000, "source": "condition: test"});
    let e = refused(v);
    assert!(
        matches!(&e, IrValidationError::ConnectorBudgetNotApplicable { id, pin } if id == "J1" && pin == "2"),
        "{e}"
    );
}

#[test]
fn ds_009_telem_takes_one_port_s_tx_out_and_rx_in() {
    assert_eq!(parse(telem(Some("A_TX"), Some("A_RX"))).validate(), Ok(()));
    let role = |v: Value| match refused(v) {
        IrValidationError::ConnectorPinContradictsStandard { id, pin, role, .. } if id == "J1" => {
            (pin, role)
        }
        e => panic!("{e}"),
    };
    // Swapped: the autopilot's TX (OUT) holds the port's RX.
    assert_eq!(
        role(telem(Some("A_RX"), Some("A_TX"))),
        ("2".into(), "TX (OUT)")
    );
    // Two ports' lines on one connector.
    assert_eq!(
        role(telem(Some("A_TX"), Some("B_RX"))),
        ("3".into(), "RX (IN)")
    );
    // A serial pin may not float.
    assert_eq!(role(telem(Some("A_TX"), None)), ("3".into(), "RX (IN)"));
}

#[test]
fn ds_009_gps_takes_a_port_and_the_bus_s_i2c_lines() {
    let gps = |scl: &str, sda: &str| {
        let mut g = port("G", None, None, None);
        g["role"] = json!("gps");
        let v = with_i2c(spec_json(vec![g]), None, None);
        let nets = [
            Some("VDD_5V"),
            Some("G_TX"),
            Some("G_RX"),
            Some(scl),
            Some(sda),
            Some("GND"),
        ];
        with_connector(v, "SM06B-GHS-TB", Some("DS-009 GPS"), &nets)
    };
    assert_eq!(parse(gps("I2C_SCL", "I2C_SDA")).validate(), Ok(()));
    assert!(matches!(
        refused(gps("I2C_SDA", "I2C_SCL")),
        IrValidationError::ConnectorPinContradictsStandard {
            role: "I2C SCL",
            ..
        }
    ));
}

/// Adversarial F8: DS-009's GPS ports carry a GPS, by their names; TELEM is
/// "any serial port". A `mavlink` port on DS-009 GPS or GPS FULL is refused
/// by name, citing the pin and the port's section; on TELEM every role
/// validates.
#[test]
fn adversarial_f8_ds_009_s_gps_ports_take_only_a_gps() {
    let on = |standard: &str, part: &str, role: &str| {
        let mut g = port("G", None, None, None);
        g["role"] = json!(role);
        // Each role's own framing, so only the connector can refuse it.
        match role {
            "rc_crsf" => {
                g["framing"]["baud"] = json!(420000);
                g["peer_deviation_ppm"] = json!(8002);
            }
            "rc_sbus" => {
                g["framing"] = json!({"baud": 100000, "data_bits": 8, "parity": "even",
                    "stop_bits": 2, "inverted": true});
            }
            _ => {}
        }
        let v = with_i2c(spec_json(vec![g]), None, None);
        let mut nets = vec![
            Some("VDD_5V"),
            Some("G_TX"),
            Some("G_RX"),
            Some("I2C_SCL"),
            Some("I2C_SDA"),
        ];
        if standard == "DS-009 GPS FULL" {
            nets.extend([None, None, Some("VDD_3V3"), None]);
        }
        nets.push(Some("GND"));
        with_connector(v, part, Some(standard), &nets)
    };
    assert_eq!(
        parse(on("DS-009 GPS", "SM06B-GHS-TB", "gps")).validate(),
        Ok(())
    );
    for standard in ["DS-009 GPS", "DS-009 GPS FULL"] {
        let part = if standard == "DS-009 GPS" {
            "SM06B-GHS-TB"
        } else {
            "SM10B-GHS-TB"
        };
        for role in ["console", "mavlink", "rc_crsf", "rc_sbus"] {
            let e = refused(on(standard, part, role));
            assert_eq!(e.kind(), "ConnectorSerialRoleContradictsStandard", "{e}");
            assert_eq!(e.id(), Some("J1"));
            let text = e.to_string();
            assert!(
                text.contains("pin `2`")
                    && text.contains(&format!("`{role}`"))
                    && text.contains("`gps`")
                    && text.contains(&format!("`{standard}`")),
                "{text}"
            );
        }
    }
    for role in ["console", "mavlink", "gps", "rc_crsf", "rc_sbus"] {
        let mut v = on("DS-009 TELEM", "SM06B-GHS-TB", role);
        // TELEM's pins 4 and 5 are CTS and RTS, which float.
        v["connectors"][0]["pins"][3] = json!({"pin": "4", "net": null, "offboard": null});
        v["connectors"][0]["pins"][4] = json!({"pin": "5", "net": null, "offboard": null});
        assert_eq!(parse(v).validate(), Ok(()), "{role}");
    }
}

/// Adversarial F9: each serial port's TX and RX end at a declared connector
/// pin (the architect's ruling 1). A port on no connector, or with one line
/// on none, is refused by name, naming the line.
#[test]
fn adversarial_f9_a_port_s_tx_and_rx_each_end_at_a_connector_pin() {
    let mut v = spec_json(vec![port("C", None, None, None)]);
    v["connectors"] = json!([]);
    let e = refused(v.clone());
    assert_eq!(
        e,
        IrValidationError::SerialPortNotAtConnector {
            label: "C".into(),
            line: "TX",
            net: "C_TX".into()
        }
    );
    assert_eq!((e.kind(), e.id()), ("SerialPortNotAtConnector", Some("C")));
    let v = with_connector(
        v,
        "SM04B-GHS-TB",
        None,
        &[Some("C_TX"), None, None, Some("GND")],
    );
    assert_eq!(
        refused(v),
        IrValidationError::SerialPortNotAtConnector {
            label: "C".into(),
            line: "RX",
            net: "C_RX".into()
        }
    );
}

/// The refusal says what the role takes: a serial pin's rule, never "a role
/// this schema cannot yet declare" (adversarial F8's stale message).
#[test]
fn a_serial_pin_s_refusal_states_the_serial_rule() {
    let text = refused(telem(Some("A_RX"), Some("A_TX"))).to_string();
    assert!(
        text.contains("a `TX (OUT)` pin takes a serial port's `tx` net")
            && !text.contains("cannot yet declare"),
        "{text}"
    );
}

#[test]
fn ds_009_debug_mini_takes_a_console_but_still_refuses_swd() {
    let v = with_connector(
        spec_json(vec![port("C", None, None, None)]),
        "SM06B-SRSS-TB",
        Some("DS-009 DEBUG MINI"),
        &[
            Some("VDD_3V3"),
            Some("C_TX"),
            Some("C_RX"),
            None,
            None,
            Some("GND"),
        ],
    );
    assert!(
        matches!(&refused(v), IrValidationError::ConnectorPinContradictsStandard { pin, role: "SWDIO", .. }
            if pin == "4"),
    );
}

#[test]
fn a_null_port_takes_the_first_usart_with_free_pins_and_its_solved_brr() {
    let t = solved(spec_json(vec![port("CONSOLE", None, None, None)]));
    assert_eq!(
        placed(&t, "CONSOLE"),
        ("USART1".into(), "PA9".into(), "PA10".into())
    );
    let tx = &t.assignments["CONSOLE_TX"];
    assert_eq!(
        (tx.signal.as_str(), tx.alternate_function),
        ("USART1_TX", 7)
    );
    assert_eq!(t.assignments["CONSOLE_RX"].signal, "USART1_RX");
    assert_eq!(
        t.serial_ports["CONSOLE"],
        SolvedSerialPort {
            index: 0,
            peripheral: "USART1".into(),
            tx: "CONSOLE_TX".into(),
            rx: "CONSOLE_RX".into(),
            role: SerialRole::Console,
            framing: Framing {
                baud: 115_200,
                data_bits: 8,
                parity: Parity::None,
                stop_bits: 1,
                inverted: false
            },
            negotiated: None,
            peer_deviation_ppm: 0,
            kernel_hz: 64_000_000,
            brr: 556,
            actual_millibaud: 115_107_913,
            error_ppm: -799,
        }
    );
    assert!(check_silicon(&t.clock, &t.serial_ports["CONSOLE"]).holds);
}

#[test]
fn ports_take_distinct_peripherals_in_order_and_a_declared_one_is_honoured() {
    let ports = (0..4)
        .map(|i| port(&format!("P{i}"), None, None, None))
        .collect();
    let t = solved(spec_json(ports));
    let got: Vec<_> = t
        .serial_ports_in_order()
        .into_iter()
        .map(|(label, p, tx, rx)| {
            (
                label,
                p.peripheral.as_str(),
                tx.pin.as_str(),
                rx.pin.as_str(),
            )
        })
        .collect();
    assert_eq!(
        got,
        [
            ("P0", "USART1", "PA9", "PA10"),
            ("P1", "USART2", "PA2", "PA3"),
            ("P2", "USART3", "PC10", "PC11"),
            ("P3", "UART4", "PA0", "PA1"),
        ]
    );
    let t = solved(spec_json(vec![
        port("A", Some("UART4"), None, None),
        port("B", None, None, None),
    ]));
    // UART4 takes its first pins; B, USART1.
    assert_eq!(
        placed(&t, "A"),
        ("UART4".into(), "PA0".into(), "PA1".into())
    );
    assert_eq!(placed(&t, "B").0, "USART1");
    // A pin two peripherals share takes the AF of the one placed: USART3's
    // when the pin alone decides, UART4's when the port names it.
    let t = solved(spec_json(vec![port("B", None, Some("PC10"), None)]));
    assert_eq!(
        placed(&t, "B"),
        ("USART3".into(), "PC10".into(), "PC11".into())
    );
    assert_eq!(t.assignments["B_TX"].alternate_function, 7);
    let t = solved(spec_json(vec![port(
        "B",
        Some("UART4"),
        Some("PC10"),
        None,
    )]));
    assert_eq!(
        placed(&t, "B"),
        ("UART4".into(), "PC10".into(), "PA1".into())
    );
    assert_eq!(t.assignments["B_TX"].alternate_function, 8);
}

#[test]
fn adversarial_3_tx_and_rx_on_different_peripherals_are_refused_by_name() {
    let e = solve_refused(spec_json(vec![port("C", None, Some("PA9"), Some("PA3"))]));
    assert_eq!(
        e,
        SolveError::SerialPinsSplit {
            port: "C".into(),
            tx_pin: "PA9".into(),
            rx_pin: "PA3".into()
        }
    );
    assert_eq!(e.claimants(), ["C"]);
    // A pin that carries LPUART1 only through PA9/PA10's second AF still
    // takes USART1, never LPUART1.
    let t = solved(spec_json(vec![port("C", None, Some("PA9"), Some("PA10"))]));
    assert_eq!(placed(&t, "C").0, "USART1");
}

#[test]
fn adversarial_3_a_port_on_an_i2c_or_spi_pin_is_refused_by_name() {
    // `AET-WO-4300i`: `C_TX` reaches J100, and `PA5` is `TT_ha`, so
    // `validate`'s step 26 refuses it at the connector before the search.
    let e = solve_refused(spec_json(vec![port("C", None, Some("PA5"), None)]));
    assert_eq!(
        (e.kind(), e.id()),
        ("ConnectorPinNotFt", Some("C_TX")),
        "{e}"
    );
    let e = solve_refused(spec_json(vec![port("C", None, Some("PB8"), None)]));
    let SolveError::NoSerialPeripheral { port: who, reasons } = &e else {
        panic!("PB8: {e}");
    };
    assert_eq!(who, "C");
    assert!(
        reasons.contains(&"USART1: `USART1_TX` is not on pin `PB8`".to_owned()),
        "{e}"
    );
    assert_eq!(reasons.len(), 4, "one reason per offered peripheral: {e}");
    // A pin an I2C line fixes is claimed twice.
    let v = with_i2c(
        spec_json(vec![port("C", None, Some("PB6"), None)]),
        None,
        Some("PB6"),
    );
    let e = solve_refused(v);
    assert_eq!(e.kind(), "PinClaimedTwice");
    assert_eq!(e.claimants(), ["i2c_buses[0].scl", "serial_ports[0].tx"]);
    // A pin the bus would take, fixed by the port: the bus moves.
    let t = solved(with_i2c(
        spec_json(vec![port("C", None, Some("PB6"), None)]),
        None,
        None,
    ));
    assert_eq!(
        placed(&t, "C"),
        ("USART1".into(), "PB6".into(), "PA10".into())
    );
    assert_eq!(t.assignments["I2C_SCL"].pin, "PB8");
}

/// `AET-WO-1218` (`1217` R4): on the H753, whose row withholds `PC2` as an
/// analog pad, with `USART2_TX` also on `PC2` in the test table, the reason
/// names the pad beside the pin held by hand.
#[test]
fn a_port_refusal_names_a_pin_withheld_as_an_analog_pad() {
    const H753: &str = "STM32H753ZITx";
    let mut t = table();
    t.part = H753.into();
    let mut pc2 = t.pins["PA2"].clone();
    pc2.position = 900;
    t.pins.insert("PC2".into(), pc2);
    let mut v = spec_json(vec![
        port("A", Some("USART2"), None, None),
        port("B", None, Some("PA2"), None),
    ]);
    v["mcu"]["part"] = json!(H753);
    let e = solve(&parse(v), &t, HASH).expect_err("refused");
    let SolveError::NoSerialPeripheral { reasons, .. } = &e else {
        panic!("{e}");
    };
    assert_eq!(
        reasons,
        &[
            "USART2: every pin for `USART2_TX` is held by hand (`PA2` by `serial_ports[1].tx`); \
           `PC2` withheld as an analog pad (the MCU row's `io.analog_pads`)"
        ]
    );
}

#[test]
fn a_port_with_every_pin_held_names_the_holders() {
    let e = solve_refused(spec_json(vec![
        port("A", Some("USART2"), None, None),
        port("B", None, Some("PA2"), None),
    ]));
    let SolveError::NoSerialPeripheral { port: who, reasons } = &e else {
        panic!("{e}");
    };
    assert_eq!(who, "A");
    assert_eq!(
        reasons,
        &["USART2: every pin for `USART2_TX` is held by hand (`PA2` by `serial_ports[1].tx`)"]
    );
    // `AET-WO-4300i`: `C_RX` reaches J100, so a pin with no held structure
    // is `validate`'s step 26 (`McuPinStructureUnheld`), before `solve`'s
    // `UnknownPin`.
    let e = solve_refused(spec_json(vec![port("C", None, None, Some("PA13"))]));
    assert_eq!(e.kind(), "ReservedPin");
    assert_eq!(e.claimants(), ["serial_ports[0].rx"]);
    let e = solve_refused(spec_json(vec![port("C", None, None, Some("PZ1"))]));
    assert_eq!(
        (e.kind(), e.id()),
        ("McuPinStructureUnheld", Some("C_RX")),
        "{e}"
    );
    assert_eq!(
        e.to_string().split('`').nth(1),
        Some("serial_ports[0].rx.pin")
    );
    let e = solve_refused(spec_json(vec![port("C", Some("UART7"), None, None)]));
    assert_eq!(
        e,
        SolveError::UnknownPeripheral {
            peripheral: "UART7".into(),
            claimant: "serial_ports[0]".into()
        }
    );
}

#[test]
fn more_ports_than_peripherals_is_a_minimal_set() {
    let ports = (0..5)
        .map(|i| port(&format!("P{i}"), None, None, None))
        .collect();
    let e = solve_refused(spec_json(ports));
    assert_eq!(e.kind(), "SerialPortsInfeasible", "{e}");
    assert_eq!(e.claimants(), ["P0", "P1", "P2", "P3", "P4"]);
    assert_eq!(e.id(), None);
}

/// [`PINS`] with USART1's RX only on PB7, and I2C1's SDA on PB9 only when
/// `spare_sda`.
fn narrow_table(spare_sda: bool) -> PinTable {
    let mut t = table();
    t.pins
        .get_mut("PA10")
        .unwrap()
        .alternate_functions
        .remove("USART1_RX");
    if !spare_sda {
        t.pins.get_mut("PB9").unwrap().alternate_functions.clear();
    }
    t
}

#[test]
fn the_search_backtracks_into_the_buses_for_a_port() {
    // The bus's first choice (SDA on PB7) leaves USART1 no RX; the search
    // moves it to PB9.
    let v = with_i2c(
        spec_json(vec![port("C", Some("USART1"), None, None)]),
        None,
        None,
    );
    let t = solve(&parse(v.clone()), &narrow_table(true), HASH).unwrap();
    assert_eq!(
        placed(&t, "C"),
        ("USART1".into(), "PA9".into(), "PB7".into())
    );
    assert_eq!(t.assignments["I2C_SDA"].pin, "PB9");
    // With no other SDA pin, the bus and the port are named together.
    let e = solve(&parse(v), &narrow_table(false), HASH).unwrap_err();
    assert_eq!(e.kind(), "SerialPortsInfeasible", "{e}");
    assert_eq!(e.claimants(), ["SENS", "C"]);
}

#[test]
fn a_port_s_tx_and_rx_never_share_a_pin() {
    // A table whose PA2 carries USART2's TX and RX both: RX takes PA3.
    let mut t = table();
    let pa2 = &mut t.pins.get_mut("PA2").unwrap().alternate_functions;
    pa2.insert("USART2_RX".into(), 7);
    let s = spec(vec![port("C", Some("USART2"), None, None)]);
    let t = solve(&s, &t, HASH).unwrap();
    assert_eq!(
        placed(&t, "C"),
        ("USART2".into(), "PA2".into(), "PA3".into())
    );
}

#[test]
fn a_baud_no_brr_reaches_is_refused_by_name() {
    for baud in [900, 4_200_000] {
        // An `rc_sbus` port: 900 is refused first at step 13c on the console
        // (`ConsoleBaudNotRoundTrip`, `AET-WO-1213`) and on a `mavlink` or
        // `gps` port (`SerialBaudReadAsKbaud`, `AET-WO-1214`). Step 13c leaves
        // an RC port's baud to AP_RCProtocol, and step 21 judges every role.
        let mut p = port("C", None, None, None);
        p["role"] = json!("rc_sbus");
        p["framing"]["baud"] = json!(baud);
        assert_eq!(
            solve_refused(spec_json(vec![p])),
            SolveError::InvalidSpecification(IrValidationError::SerialBaudOutOfRange {
                port: "C".into(),
                baud,
                kernel_hz: 64_000_000
            })
        );
    }
}

#[test]
fn adversarial_2_the_tolerance_is_strict_at_its_edge_in_exact_integers() {
    // BRR 17 (Table 407), a 9-bit word: 30 300 ppm. 17 x 1 000 000 baud is
    // 17 000 000; an error of exactly 3.03 % is a kernel of 17 515 100 or
    // 16 484 900.
    let row = tolerance(17, 0b01).unwrap();
    assert_eq!((row.table, row.tolerance_ppm), ("Table 407", 30_300));
    assert!(within(17_515_099, 17, 1_000_000, 30_300));
    assert!(!within(17_515_100, 17, 1_000_000, 30_300));
    assert!(within(16_484_901, 17, 1_000_000, 30_300));
    assert!(!within(16_484_900, 17, 1_000_000, 30_300));
    // BRR 16 (Table 406), an 8-bit word: 37 500 ppm, 16 000 000 = 16 x 1 M.
    let row = tolerance(16, 0b00).unwrap();
    assert_eq!((row.table, row.tolerance_ppm), ("Table 406", 37_500));
    assert!(within(16_599_999, 16, 1_000_000, 37_500));
    assert!(!within(16_600_000, 16, 1_000_000, 37_500));
}

#[test]
fn adversarial_2_the_worst_case_never_understates_and_is_judged_strictly_on_the_reset_clock() {
    // Each BRR's lowest and highest baud on 64 MHz, for every word: the
    // nearest BRR's error is at most 1 / (2 BRR), below every row; HSI's
    // deviation then takes the small BRRs past it.
    let clock = SolvedClock::reset();
    let k: u64 = 64_000_000;
    let mut checked = 0;
    let mut held = 0;
    for brr in 16u32..=0xFFFF {
        let n = u64::from(brr);
        let (lo, hi) = (2 * k / (2 * n + 1) + 1, 2 * k / (2 * n - 1));
        for baud in [lo, hi].into_iter().filter(|_| lo <= hi) {
            let baud = u32::try_from(baud).unwrap();
            assert_eq!(solve_brr(k, baud), Some(brr), "{baud}");
            for (data_bits, parity) in [(7, Parity::None), (8, Parity::None), (8, Parity::Even)] {
                let framing = Framing {
                    baud,
                    data_bits,
                    parity,
                    stop_bits: 1,
                    inverted: false,
                };
                let p = SolvedSerialPort {
                    index: 0,
                    peripheral: "USART2".into(),
                    tx: "T".into(),
                    rx: "R".into(),
                    role: SerialRole::Console,
                    framing,
                    negotiated: None,
                    peer_deviation_ppm: 0,
                    kernel_hz: k,
                    brr,
                    actual_millibaud: 0,
                    error_ppm: 0,
                };
                let c = check_silicon(&clock, &p);
                let row = c.tolerance.unwrap();
                // The quantization alone is inside the row (as before)...
                assert!(within(k, brr, baud, row.tolerance_ppm), "{baud}");
                // ...and the verdict is the strict comparison of the worst
                // case at HSI's two extremes, recomputed here (peer 0).
                let d = u128::from(brr) * u128::from(baud);
                let at = |ppm: u128| (u128::from(k) * ppm).abs_diff(d * 1_000_000).div_ceil(d);
                let exact = at(1_000_000 + 14_984).max(at(1_000_000 - 25_894));
                assert_eq!(u128::from(c.worst_case_ppm), exact, "{baud}");
                assert_eq!(
                    c.holds,
                    c.worst_case_ppm < u64::from(row.tolerance_ppm),
                    "{baud} {data_bits}"
                );
                held += usize::from(c.holds);
                checked += 1;
            }
        }
    }
    assert!(checked > 50_000, "{checked}");
    // HSI's worst case alone is 2.6 %: the small BRRs, whose rounding error is
    // largest, no longer hold; the large ones do.
    assert!(held > 0 && held < checked, "{held} of {checked}");
}

#[test]
fn adversarial_2_a_brr_rounded_the_wrong_way_is_refused() {
    let t = solved(spec_json(vec![port("C", None, None, None)]));
    let mut p = t.serial_ports["C"].clone();
    for brr in [555, 557] {
        p.brr = brr;
        let check = check_silicon(&t.clock, &p);
        assert!(!check.holds, "{brr}");
        // Inside the row: the rounding alone refuses it.
        let row = check.tolerance.unwrap();
        assert!(within(p.kernel_hz, brr, 115_200, row.tolerance_ppm));
    }
}

/// CONSOLE (115200 8N1) and SBUS (100000 8E2 inverted), solved.
fn topology() -> SolvedTopology {
    let mut sbus = port("SBUS", None, None, None);
    sbus["role"] = json!("rc_sbus");
    sbus["framing"] =
        json!({"baud": 100000, "data_bits": 8, "parity": "even", "stop_bits": 2, "inverted": true});
    solved(spec_json(vec![port("CONSOLE", None, None, None), sbus]))
}

#[test]
fn the_solved_ports_round_trip_in_order() {
    let t = topology();
    assert_eq!(t.validate(), Ok(()));
    let back: SolvedTopology = serde_json::from_str(&serde_json::to_string(&t).unwrap()).unwrap();
    assert_eq!(back, t);
    let order: Vec<_> = t
        .serial_ports_in_order()
        .into_iter()
        .map(|(label, p, tx, rx)| (label, p.index, tx.signal.as_str(), rx.signal.as_str()))
        .collect();
    assert_eq!(
        order,
        [
            ("CONSOLE", 0, "USART1_TX", "USART1_RX"),
            ("SBUS", 1, "USART2_TX", "USART2_RX")
        ]
    );
    let sbus = &t.serial_ports["SBUS"];
    assert_eq!((sbus.brr, sbus.error_ppm), (640, 0));
    assert_eq!(sbus.framing.m_bits(), Some(0b01));
    assert_eq!(sbus.framing.stop_field(), Some(0b10));
}

#[test]
fn the_topology_refuses_a_port_that_disagrees_with_its_assignments() {
    let kind = |edit: &dyn Fn(&mut SolvedTopology)| {
        let mut t = topology();
        edit(&mut t);
        t.validate()
            .map_err(|e| (e.kind(), e.id().map(str::to_owned)))
    };
    let mismatch = |id: &str| Err(("SerialPortMismatch", Some(id.to_owned())));
    assert_eq!(kind(&|_| {}), Ok(()));
    let set = |t: &mut SolvedTopology, f: &dyn Fn(&mut SolvedSerialPort)| {
        f(t.serial_ports.get_mut("SBUS").unwrap());
    };
    // An assignment on a USART without its port, or on LPUART1.
    assert_eq!(
        kind(&|t| {
            t.serial_ports.remove("SBUS");
        }),
        mismatch("SBUS_RX")
    );
    assert_eq!(
        kind(&|t| {
            let mut a = t.assignments["CONSOLE_TX"].clone();
            a.pin = "PB6".into();
            a.peripheral = "LPUART1".into();
            a.signal = "LPUART1_TX".into();
            t.assignments.insert("X".into(), a);
        }),
        mismatch("X")
    );
    // The port's nets on another signal or peripheral.
    assert_eq!(
        kind(&|t| t.assignments.get_mut("SBUS_TX").unwrap().signal = "USART2_RX".into()),
        mismatch("SBUS")
    );
    assert_eq!(
        kind(&|t| set(t, &|p| p.peripheral = "USART3".into())),
        mismatch("SBUS_RX")
    );
    assert_eq!(
        kind(&|t| set(t, &|p| p.rx = "SBUS_TX".into())),
        mismatch("SBUS")
    );
    assert_eq!(
        kind(&|t| set(t, &|p| p.tx = "CONSOLE_TX".into())),
        mismatch("SBUS")
    );
    // Indices exactly 0..n.
    assert_eq!(kind(&|t| set(t, &|p| p.index = 2)), mismatch("SBUS"));
    assert_eq!(kind(&|t| set(t, &|p| p.index = 0)), mismatch("SBUS"));
    // Every solved value is what `serial` recomputes.
    assert_eq!(kind(&|t| set(t, &|p| p.brr = 641)), mismatch("SBUS"));
    assert_eq!(
        kind(&|t| set(t, &|p| p.kernel_hz = 32_000_000)),
        mismatch("SBUS")
    );
    assert_eq!(
        kind(&|t| set(t, &|p| p.actual_millibaud += 1)),
        mismatch("SBUS")
    );
    assert_eq!(kind(&|t| set(t, &|p| p.error_ppm = 1)), mismatch("SBUS"));
    assert_eq!(
        kind(&|t| set(t, &|p| p.framing.baud = 100_001)),
        mismatch("SBUS")
    );
    // A shape `validate` refuses in a specification, and no panic on 255.
    assert_eq!(
        kind(&|t| set(t, &|p| p.framing.data_bits = 255)),
        mismatch("SBUS")
    );
    assert_eq!(
        kind(&|t| set(t, &|p| p.framing.stop_bits = 3)),
        mismatch("SBUS")
    );
    assert_eq!(
        kind(&|t| set(t, &|p| p.role = SerialRole::RcCrsf)),
        mismatch("SBUS")
    );
    // Two ports on one peripheral.
    assert_eq!(
        kind(&|t| {
            for net in ["SBUS_TX", "SBUS_RX"] {
                let a = t.assignments.get_mut(net).unwrap();
                a.peripheral = "USART1".into();
                a.signal = a.signal.replace("USART2", "USART1");
            }
            set(t, &|p| p.peripheral = "USART1".into());
        }),
        mismatch("SBUS")
    );
    // A label is an identifier.
    assert_eq!(
        kind(&|t| {
            let p = t.serial_ports.remove("SBUS").unwrap();
            t.serial_ports.insert("S/B".into(), p);
        }),
        Err(("InvalidIdentifier", None))
    );
    // The clock the values were solved for.
    assert_eq!(
        kind(&|t| t.clock.d2ppre1 = 2),
        Err(("UnsupportedClock", None))
    );
}

#[test]
fn a_version_6_topology_without_serial_ports_is_refused_as_a_version() {
    let mut v = serde_json::to_value(topology()).unwrap();
    v["schema_version"] = json!(6);
    v.as_object_mut().unwrap().remove("serial_ports");
    let t: SolvedTopology = serde_json::from_value(v.clone()).unwrap();
    assert_eq!(t.validate().unwrap_err().kind(), "UnsupportedSchemaVersion");
    v["schema_version"] = json!(aethelgard_ir::SOLVED_SCHEMA_VERSION);
    assert!(serde_json::from_value::<SolvedTopology>(v).is_err());
}

/// Adversarial pass 4, ADV4-3 (the PM's ruling): a negotiated rate's source
/// is held to the repository's rule for a source (the stackup's: 1 to 200
/// bytes of printable ASCII, no space at either end), in `validate` and in
/// `SolvedTopology::validate`. The adversary's inputs were accepted: a
/// zero-width space, a NUL, a newline and tab, and a megabyte; and the
/// rule's edges besides. Each is refused as `InvalidSource` naming the
/// path, never the value; a plain citation, 200 bytes, is accepted and
/// solves.
#[test]
fn adversarial_4_a_negotiated_source_is_held_to_the_source_rule() {
    let at = |source: &str| {
        let mut p = crsf(|p| p["framing"]["baud"] = json!(432000));
        p["negotiated"] = json!({ "source": source });
        p
    };
    let long = "a".repeat(1_000_000);
    let over = "a".repeat(201);
    for source in [
        "\u{200B}",
        "\u{0}",
        "\n\t-",
        long.as_str(),
        over.as_str(),
        "",
        " ",
        " ExpressLRS",
        "ExpressLRS ",
        "caf\u{e9}",
        "a\u{7f}",
    ] {
        let e = refused_ports(vec![at(source)]);
        assert_eq!(
            (e.kind(), e.path()),
            ("InvalidSource", Some("serial_ports[0].negotiated.source")),
            "{source:?}: {e}"
        );
        // The message is the rule's, with the path and nothing of the value.
        assert_eq!(
            e.to_string(),
            "`serial_ports[0].negotiated.source` is not a source: 1 to 200 bytes of printable ASCII, no space at either end"
        );
    }
    let cited = "ExpressLRS options.cpp rcvr-uart-baud, the receiver's 432000 setting";
    let edge = "a".repeat(200);
    for source in [cited, "x", edge.as_str()] {
        let v = spec_json(vec![at(source)]);
        let t = solved(v);
        assert_eq!(
            t.serial_ports["RC"]
                .negotiated
                .as_ref()
                .map(|n| n.source.as_str()),
            Some(source)
        );
        assert!(t.validate().is_ok());
        // The solved topology re-checks it: the same source made invalid
        // there is refused as a mismatch of the port.
        let mut bad = serde_json::to_value(&t).unwrap();
        bad["serial_ports"]["RC"]["negotiated"]["source"] = json!("\u{200B}");
        let bad: SolvedTopology = serde_json::from_value(bad).unwrap();
        let e = bad.validate().unwrap_err();
        assert_eq!(
            (e.kind(), e.id()),
            ("SerialPortMismatch", Some("RC")),
            "{e}"
        );
    }
}

/// `AET-WO-4230` §1.8: the tolerance is judged on the worst case, quantization
/// plus HSI's deviation in the direction that adds plus the peer's declared
/// deviation, strictly below the row (RM0433 Rev 9 Table 407, 33,300 ppm for
/// an 8-bit word at `BRR[3:0]` ≠ 0). 115200 on 64 MHz is `BRR` 556, 799 ppm
/// slow, so the slow HSI (25,894) adds.
#[test]
fn the_serial_check_sums_quantization_hsi_and_the_peer_strictly_below_the_row() {
    let held = |peer: u32| {
        let mut p = port("C", None, None, None);
        p["peer_deviation_ppm"] = json!(peer);
        let t = solved_or_err(spec_json(vec![p]));
        t.map(|t| check_silicon(&t.clock, &t.serial_ports["C"]))
    };
    // The row is 33,300: 33,300 − 26,673 = 6,627 ppm of peer fills it.
    let c = held(5_000).unwrap();
    assert_eq!(
        (c.quantization_ppm, c.hsi_ppm, c.peer_deviation_ppm),
        (800, 25_894, 5_000)
    );
    // The slow HSI and the slow quantization share a sign: they add, less
    // their product (799.3 × 25,894 / 10⁶ = 20.7): 26,673 ppm of receiver.
    assert_eq!(c.receiver_ppm, 26_673);
    assert_eq!((c.worst_case_ppm, c.margin_ppm), (31_673, Some(1_627)));
    assert_eq!(c.tolerance.unwrap().tolerance_ppm, 33_300);
    assert!(c.holds);
    assert_eq!(held(6_626).unwrap().margin_ppm, Some(1));
    // One more ppm reaches the row, which is not below it: `solve` refuses.
    let e = held(6_627).unwrap_err();
    assert!(
        matches!(&e, SolveError::InvalidSpecification(IrValidationError::SerialBaudTolerance {
            port, quantization_ppm: 800, hsi_ppm: 25_894, receiver_ppm: 26_673,
            peer_deviation_ppm: 6_627, worst_case_ppm: 33_300, tolerance_ppm: 33_300, .. })
            if port == "C"),
        "{e}"
    );
}

/// The worse of HSI's two extremes is taken, in exact integers. At 57600
/// (`BRR` 1111, 100 ppm fast) the fast HSI gives 15,086 ppm but the slow one
/// 25,797 (HSI pulls the baud back across the declared one, so its
/// 25,894 shrinks by the 100 and the product): the slow bound is the worse.
/// At 3,878,788 on `BRR` 16 (31,250 ppm fast, the solver's own rounding limit)
/// the fast HSI adds its 14,984 and the product to the error: 46,703 ppm, the
/// slow one only 4,547. An exact hit (100000 at `BRR` 640) takes the slow
/// bound, the larger.
#[test]
fn the_check_takes_the_worse_of_hsi_s_two_extremes() {
    let t = solved(spec_json(vec![port("C", None, None, None)]));
    let at = |baud: u32, brr: u32| {
        let mut p = t.serial_ports["C"].clone();
        p.framing.baud = baud;
        p.brr = brr;
        check_silicon(&t.clock, &p)
    };
    let c = at(57_600, 1111);
    assert_eq!(
        (c.quantization_ppm, c.hsi_ppm, c.receiver_ppm),
        (101, 25_894, 25_797)
    );
    let c = at(3_878_788, 16);
    assert_eq!(
        (c.quantization_ppm, c.hsi_ppm, c.receiver_ppm),
        (31_250, 14_984, 46_703)
    );
    // 46,703 is over Table 406's 37,500: refused, though the quantization
    // alone (31,250) is inside it.
    assert!(!c.holds);
    assert_eq!(c.margin_ppm, Some(37_500 - 46_703));
    let c = at(100_000, 640);
    assert_eq!(
        (c.quantization_ppm, c.hsi_ppm, c.receiver_ppm),
        (0, 25_894, 25_894)
    );
}

/// An `rc_crsf` port declares a peer deviation of at least the spread of the
/// two bauds `CRSF_BAUDS` cites: 3,334 / 416,666 = 8,001.6, 8,002 ppm.
#[test]
fn a_crsf_port_s_peer_deviation_is_at_least_the_cited_spread() {
    assert_eq!(aethelgard_ir::serial::crsf_cited_spread_ppm(), 8_002);
    let e = refused_ports(vec![crsf(|p| p["peer_deviation_ppm"] = json!(8_001))]);
    assert!(
        matches!(&e, IrValidationError::PeerDeviationBelowCited { id, declared_ppm: 8_001,
            cited_ppm: 8_002, citation } if id == "RC"
            && citation.contains(CRSF_SOURCE) && citation.contains(CRSF_ELRS_SOURCE)),
        "{e}"
    );
    assert_eq!((e.kind(), e.id()), ("PeerDeviationBelowCited", Some("RC")));
    assert_eq!(parse(spec_json(vec![crsf(|_| {})])).validate(), Ok(()));
    // Only an `rc_crsf` port is held to it.
    let mut p = port("C", None, None, None);
    p["peer_deviation_ppm"] = json!(0);
    assert_eq!(parse(spec_json(vec![p])).validate(), Ok(()));
    // The solved topology is re-checked: a port edited under the cited spread.
    let mut t = solved(spec_json(vec![crsf(|_| {})]));
    assert_eq!(t.validate(), Ok(()));
    t.serial_ports.get_mut("RC").unwrap().peer_deviation_ppm = 8_001;
    assert!(matches!(
        t.validate(),
        Err(IrValidationError::SerialPortMismatch { .. })
    ));
}

/// `AET-WO-4230` R2 (the architect's ruling on Q-A): a figure is `cited`
/// only on an `rc_crsf` port and only as the cited spread, 8,002 ppm; any
/// other `cited` figure is refused by name (`PeerDeviationNotCited`). Every
/// other figure is `declared`, the CRSF spread included.
#[test]
fn a_peer_deviation_is_cited_only_as_the_crsf_spread() {
    // Cited at the spread, on CRSF: held.
    assert_eq!(parse(spec_json(vec![crsf(|_| {})])).validate(), Ok(()));
    // Declared, on CRSF at or above the spread, and on any other role.
    for ppm in [8_002, 8_003] {
        let mut p = crsf(|p| p["peer_deviation_source"] = json!("declared"));
        p["peer_deviation_ppm"] = json!(ppm);
        assert_eq!(parse(spec_json(vec![p])).validate(), Ok(()), "{ppm}");
    }
    // 20,000 declared passes the citation rule, and its worst case (43,453
    // ppm) is over the receiver's 33,300: `validate`'s step 21 refuses it
    // (`AET-WO-1208` §1.3; before, it passed and `solve` refused it).
    let mut p = crsf(|p| p["peer_deviation_source"] = json!("declared"));
    p["peer_deviation_ppm"] = json!(20_000);
    let e = parse(spec_json(vec![p])).validate().unwrap_err();
    assert_eq!(e.kind(), "SerialBaudTolerance", "{e}");
    // Cited on CRSF at a figure that is not the spread: above it (below it
    // is `PeerDeviationBelowCited`, judged first).
    for ppm in [8_003, 10_000, u32::MAX] {
        let mut p = crsf(|_| {});
        p["peer_deviation_ppm"] = json!(ppm);
        let e = refused_ports(vec![p]);
        assert!(
            matches!(&e, IrValidationError::PeerDeviationNotCited { id, role: "rc_crsf",
                declared_ppm, cited_ppm: 8_002 } if id == "RC" && *declared_ppm == ppm),
            "{ppm}: {e}"
        );
        assert_eq!((e.kind(), e.id()), ("PeerDeviationNotCited", Some("RC")));
    }
    let e = refused_ports(vec![crsf(|p| p["peer_deviation_ppm"] = json!(8_001))]);
    assert_eq!(e.kind(), "PeerDeviationBelowCited", "{e}");
    // Cited on every other role, even at 8,002 ppm: the repository holds no
    // citation for the peer of a console, MAVLink, GPS or SBUS port.
    for role in ["console", "mavlink", "gps", "rc_sbus"] {
        for ppm in [0, 5_000, 8_002] {
            let mut p = port("C", None, None, None);
            p["role"] = json!(role);
            if role == "rc_sbus" {
                p["framing"] = json!({"baud": 100000, "data_bits": 8, "parity": "even", "stop_bits": 2, "inverted": true});
            }
            p["peer_deviation_ppm"] = json!(ppm);
            // Declared, each passes the citation rule. At 8,002 ppm an 8N1
            // 115,200 baud port's worst case (34,675 ppm) is over the
            // receiver's 33,300, which step 21 refuses (`AET-WO-1208` §1.3;
            // before, `solve` did).
            match parse(spec_json(vec![p.clone()])).validate() {
                Ok(()) => {}
                Err(e) => assert!(
                    ppm == 8_002 && e.kind() == "SerialBaudTolerance",
                    "{role} {ppm}: {e}"
                ),
            }
            p["peer_deviation_source"] = json!("cited");
            let e = refused_ports(vec![p]);
            assert!(
                matches!(&e, IrValidationError::PeerDeviationNotCited { id, role: r,
                    declared_ppm, .. } if id == "C" && *r == role && *declared_ppm == ppm),
                "{role} {ppm}: {e}"
            );
        }
    }
    // The second port is judged too, in document order.
    let mut second = port("D", None, None, None);
    second["peer_deviation_source"] = json!("cited");
    let e = refused_ports(vec![port("C", None, None, None), second]);
    assert_eq!((e.kind(), e.id()), ("PeerDeviationNotCited", Some("D")));
}

/// A version-9 file, which has no `peer_deviation_ppm`, is refused, never
/// defaulted; a version-9 file that has it is refused by its version.
#[test]
fn a_version_9_specification_is_refused() {
    assert_eq!(aethelgard_ir::SPECIFICATION_SCHEMA_VERSION, 30);
    let mut v = spec_json(vec![port("C", None, None, None)]);
    v["schema_version"] = json!(9);
    let e = refused(v.clone());
    assert!(
        matches!(
            e,
            IrValidationError::UnsupportedSchemaVersion {
                changes: _,
                version: 9,
                expected: 30
            }
        ),
        "{e}"
    );
    v["serial_ports"][0]
        .as_object_mut()
        .unwrap()
        .remove("peer_deviation_ppm");
    assert!(serde_json::from_value::<Specification>(v).is_err());
}

fn solved_or_err(v: Value) -> Result<SolvedTopology, SolveError> {
    solve(&parse(v), &table(), HASH)
}

/// A version-10 file (`AET-WO-4300d3`'s, which has no `peer_deviation_ppm`) is
/// refused by its version, never defaulted; one that has the key is refused
/// by its version too.
#[test]
fn a_version_10_specification_is_refused() {
    let mut v = spec_json(vec![port("C", None, None, None)]);
    v["schema_version"] = json!(10);
    let e = refused(v.clone());
    assert!(
        matches!(
            e,
            IrValidationError::UnsupportedSchemaVersion {
                changes: _,
                version: 10,
                expected: 30
            }
        ),
        "{e}"
    );
    v["serial_ports"][0]
        .as_object_mut()
        .unwrap()
        .remove("peer_deviation_ppm");
    assert!(serde_json::from_value::<Specification>(v).is_err());
}

/// A version-11 file (`AET-WO-4300g`'s: `mcu.supply_source`, but no
/// `peer_deviation_ppm`, `peer_deviation_source` or `acknowledge_unwitnessed`)
/// is refused by its version, never defaulted; one that has every key is
/// refused by its version too (`AET-WO-4230` renumbered to 12).
#[test]
fn a_version_11_specification_is_refused() {
    let mut v = spec_json(vec![port("C", None, None, None)]);
    v["schema_version"] = json!(11);
    assert!(v["mcu"].get("supply_source").is_some());
    let e = refused(v.clone());
    assert!(
        matches!(
            e,
            IrValidationError::UnsupportedSchemaVersion {
                changes: _,
                version: 11,
                expected: 30
            }
        ),
        "{e}"
    );
    for key in ["peer_deviation_ppm", "peer_deviation_source"] {
        let mut old = v.clone();
        old["serial_ports"][0].as_object_mut().unwrap().remove(key);
        assert!(
            serde_json::from_value::<Specification>(old).is_err(),
            "{key}"
        );
    }
    let mut old = v;
    old.as_object_mut()
        .unwrap()
        .remove("acknowledge_unwitnessed");
    assert!(serde_json::from_value::<Specification>(old).is_err());
}

/// `AET-WO-4230` R3 (Q-H): every serial port of every specification in the
/// repository, at the reset clock's 64 MHz USART kernel, gets from the
/// solver's `solve_brr` a `BRR` the independent `silicon::check_brr` holds,
/// and the word one off either way fails it.
#[test]
fn check_brr_holds_the_solver_s_brr_on_every_fixture_s_port() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let files = [
        "examples/sentinel-01-minimal.json",
        "crates/aethelgard-cli/tests/fixtures/serial-dshot-9600.json",
        "crates/aethelgard-cli/tests/fixtures/serial-four.json",
    ];
    let clock = SolvedClock::reset();
    let kernel = aethelgard_ir::serial::kernel_hz(&clock, "USART1").unwrap();
    let mut seen = Vec::new();
    for file in files {
        let spec: Specification =
            serde_json::from_slice(&std::fs::read(root.join(file)).unwrap()).unwrap();
        for p in &spec.serial_ports {
            let baud = p.framing.baud;
            let brr = solve_brr(kernel, baud).unwrap();
            let f = aethelgard_ir::silicon::check_brr("USART1", &clock, kernel, baud, brr);
            assert!(f.holds, "{file} {}: {f:?}", p.label);
            for wrong in [brr - 1, brr + 1] {
                let f = aethelgard_ir::silicon::check_brr("USART1", &clock, kernel, baud, wrong);
                assert!(!f.holds, "{file} {} {wrong}: {f:?}", p.label);
            }
            seen.push((p.label.clone(), baud, brr));
        }
    }
    let seen: Vec<(&str, u32, u32)> = seen.iter().map(|(l, b, r)| (l.as_str(), *b, *r)).collect();
    assert_eq!(
        seen,
        [
            ("CONSOLE", 115_200, 556),
            ("CONSOLE", 9_600, 6_667),
            ("MAVLINK", 57_600, 1_111),
            ("GPS", 115_200, 556),
            ("CRSF", 420_000, 152),
            ("SBUS", 100_000, 640),
        ]
    );
}

/// `AET-WO-1213`: a console's baud `b` is one ArduPilot's `SERIAL0_BAUD`
/// maps back to, `map_baudrate(b / 1000) == b` (`AP_SerialManager.cpp:172`,
/// `:419`, `:737-770` at `Copter-4.7.2`). Each committed board's 115200
/// and 9600 passes, and so do 100000 and 250000, the work order's examples:
/// 100 is a listed code and 250 is kbaud. A baud that maps elsewhere is
/// refused by name, the mapped rate in the error; any other role is not
/// SERIAL0 and is not held to it.
#[test]
fn a_console_baud_serial0_cannot_hold_is_refused_by_name() {
    let at = |baud: u32, role: &str| {
        let mut p = port("CONSOLE", None, None, None);
        p["framing"]["baud"] = json!(baud);
        p["role"] = json!(role);
        spec(vec![p]).validate()
    };
    for baud in [115_200, 9600, 57_600, 100_000, 250_000, 2_000_000] {
        assert_eq!(at(baud, "console"), Ok(()), "{baud}");
    }
    for (baud, mapped) in [
        (76_800, 76_000),
        (111_000, 111_100),
        (115_000, 115_200),
        (3_000_000, 3000),
        (999, 57_600),
        (2_001_000, 2001),
    ] {
        let e = at(baud, "console").expect_err("refused");
        assert_eq!(
            e,
            IrValidationError::ConsoleBaudNotRoundTrip {
                id: "CONSOLE".into(),
                baud,
                mapped
            },
            "{baud}"
        );
        assert_eq!(
            (e.kind(), e.id()),
            ("ConsoleBaudNotRoundTrip", Some("CONSOLE"))
        );
        // `1_BAUD` to `9_BAUD` hold their define as it stands: above 2000,
        // it maps to itself (999 is `AET-WO-1214`'s, below).
        if baud > 2000 {
            assert_eq!(at(baud, "mavlink"), Ok(()), "{baud} on mavlink");
            assert_eq!(at(baud, "gps"), Ok(()), "{baud} on gps");
        }
    }
    // The rule comes after the framing's shape: a zero baud is still that.
    assert_eq!(at(0, "console").unwrap_err().kind(), "SerialFramingShape");
}

/// The same rule in `SolvedTopology::validate`, as each 13c rule is: a
/// console moved to 76800 with its solved values recomputed is a mismatch.
#[test]
fn the_topology_refuses_a_console_baud_serial0_cannot_hold() {
    use aethelgard_ir::serial::{actual_millibaud, error_ppm};
    let mut t = topology();
    let at = |t: &mut SolvedTopology, baud: u32| {
        let p = t.serial_ports.get_mut("CONSOLE").unwrap();
        p.framing.baud = baud;
        p.brr = solve_brr(p.kernel_hz, baud).unwrap();
        p.actual_millibaud = actual_millibaud(p.kernel_hz, p.brr);
        p.error_ppm = error_ppm(p.kernel_hz, p.brr, baud);
    };
    at(&mut t, 250_000);
    assert_eq!(t.validate(), Ok(()));
    at(&mut t, 76_800);
    let e = t.validate().expect_err("refused");
    assert_eq!((e.kind(), e.id()), ("SerialPortMismatch", Some("CONSOLE")));
}

/// `AET-WO-1214`: a `mavlink` or `gps` port is SERIAL1 to 9, whose
/// `SERIALn_BAUD` holds the define as it stands (`AP_SerialManager.cpp:199`)
/// and is opened at `map_baudrate` of it (`:473`, `:490`). A baud of 2000 or
/// less is a listed code or kbaud there, and is refused by name with the rate
/// ArduPilot would open; above 2000 the value is the baud. RC input sets its
/// own rate and the console has its own rule, so neither is held to this one.
#[test]
fn a_serial_baud_ardupilot_reads_as_kbaud_is_refused_by_name() {
    let at = |baud: u32, role: &str| {
        let mut p = port("TELEM", None, None, None);
        p["framing"]["baud"] = json!(baud);
        p["role"] = json!(role);
        spec(vec![p]).validate()
    };
    for role in ["mavlink", "gps"] {
        for (baud, mapped) in [
            (2000, 2_000_000),
            (1200, 1_200_000),
            (1, 1200),
            (999, 999_000),
        ] {
            let e = at(baud, role).expect_err("refused");
            assert_eq!(
                e,
                IrValidationError::SerialBaudReadAsKbaud {
                    id: "TELEM".into(),
                    role: if role == "gps" { "gps" } else { "mavlink" },
                    baud,
                    mapped
                },
                "{baud} on {role}"
            );
            assert_eq!((e.kind(), e.id()), ("SerialBaudReadAsKbaud", Some("TELEM")));
        }
        for baud in [2001, 9600, 57_600, 115_200] {
            assert_eq!(at(baud, role), Ok(()), "{baud} on {role}");
        }
    }
    // The generator's guard, `aethelgard-hwdef`'s `SerialBaud`, is `b <= 2000`
    // on every non-console port; the IR's rule is `map_baudrate(b) != b`. Over
    // the boundary they agree, with no shared code (`RUL-20`).
    for baud in 1..=4000 {
        let kind = at(baud, "mavlink").err().map(|e| e.kind());
        assert_eq!(
            kind == Some("SerialBaudReadAsKbaud"),
            baud <= 2000,
            "{baud}"
        );
    }
    // Not the rule's: the console keeps its own (1200 holds as `1` and maps
    // back; 2000 holds as `2`, 2400), RC input is AP_RCProtocol's (13c's
    // `rc_crsf` floor refuses first; `rc_sbus` passes step 13c).
    assert_eq!(at(1200, "console"), Ok(()));
    assert_eq!(
        at(2000, "console").unwrap_err().kind(),
        "ConsoleBaudNotRoundTrip"
    );
    assert_eq!(
        at(1200, "rc_crsf").unwrap_err().kind(),
        "SerialFramingContradictsRole"
    );
    assert_eq!(at(1200, "rc_sbus"), Ok(()));
    // A baud `int32_t` cannot hold is no `SERIALn_BAUD`; step 21 refuses it.
    for baud in [i32::MAX.unsigned_abs(), i32::MAX.unsigned_abs() + 1] {
        assert_eq!(
            at(baud, "mavlink").unwrap_err().kind(),
            "SerialBaudOutOfRange",
            "{baud}"
        );
    }
}

/// The same rule in `SolvedTopology::validate`, as each 13c rule is: a
/// `mavlink` port moved to 1200 with its solved values recomputed is a
/// mismatch, and at 2001 it is not.
#[test]
fn the_topology_refuses_a_serial_baud_ardupilot_reads_as_kbaud() {
    use aethelgard_ir::serial::{actual_millibaud, error_ppm};
    let mut mav = port("TELEM", None, None, None);
    mav["role"] = json!("mavlink");
    mav["framing"]["baud"] = json!(57_600);
    let mut t = solved(spec_json(vec![port("CONSOLE", None, None, None), mav]));
    let at = |t: &mut SolvedTopology, baud: u32| {
        let p = t.serial_ports.get_mut("TELEM").unwrap();
        p.framing.baud = baud;
        p.brr = solve_brr(p.kernel_hz, baud).unwrap();
        p.actual_millibaud = actual_millibaud(p.kernel_hz, p.brr);
        p.error_ppm = error_ppm(p.kernel_hz, p.brr, baud);
    };
    assert_eq!(t.validate(), Ok(()));
    at(&mut t, 2001);
    assert_eq!(t.validate(), Ok(()));
    at(&mut t, 1200);
    let e = t.validate().expect_err("refused");
    assert_eq!((e.kind(), e.id()), ("SerialPortMismatch", Some("TELEM")));
}
