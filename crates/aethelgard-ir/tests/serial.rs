//! `AET-WO-4300b` §1.1–§1.3 and adversarial items 2 and 3: serial ports
//! declared, validated, placed on USARTs, their BRR solved and checked, and
//! their topology judged, on a small synthetic `PinTable` (this crate may
//! depend on no other member; the real table is `solve_cli.rs`'s).

use std::collections::BTreeMap;

use aethelgard_ir::serial::{
    self, CRSF_SOURCE, PERIPHERALS, SolvedSerialPort, check_silicon, solve_brr, tolerance, within,
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
        part: "TEST_PART".into(),
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
           "framing": {"baud": 115200, "data_bits": 8, "parity": "none", "stop_bits": 1, "inverted": false}})
}

fn spec_json(ports: Vec<Value>) -> Value {
    json!({
        "schema_version": 7,
        "project_name": "t",
        "board": serde_json::from_str::<Value>(BOARD).unwrap(),
        "mcu": {"part": "TEST_PART", "supply": "VDD_3V3", "hse_hz": 0},
        "voltage_domains": [
            {"id": "VDD_3V3", "nominal_millivolts": 3300, "tolerance_permille": 50},
            {"id": "VDD_5V", "nominal_millivolts": 5000, "tolerance_permille": 50}],
        "i2c_buses": [], "spi_buses": [], "i2c_devices": [], "spi_devices": [],
        "net_constraints": [],
        "connectors": [],
        "net_classes": [],
        "diff_pairs": [],
        "motor_outputs": [],
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
    for key in ["label", "peripheral", "tx", "rx", "role", "framing"] {
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
    edit(&mut p["framing"]);
    p
}

#[test]
fn a_crsf_port_that_contradicts_tbs_s_text_is_refused_with_the_citation() {
    assert_eq!(spec(vec![crsf(|_| {})]).validate(), Ok(()));
    assert_eq!(
        spec(vec![crsf(|f| f["baud"] = json!(416666))]).validate(),
        Ok(())
    );
    let cases: [(Edit, &str); 4] = [
        (|f| f["baud"] = json!(400000), "baud"),
        (|f| f["parity"] = json!("even"), "framing"),
        (|f| f["stop_bits"] = json!(2), "framing"),
        (|f| f["inverted"] = json!(true), "inverted"),
    ];
    for (edit, field) in cases {
        let e = refused_ports(vec![crsf(edit)]);
        assert!(
            matches!(&e, IrValidationError::SerialFramingContradictsRole {
                id, role: "rc_crsf", field: f, citation, ..
            } if id == "RC" && *f == field && *citation == CRSF_SOURCE),
            "{field}: {e}"
        );
    }
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

/// A connector `J1` on `part` with `standard`, pins `1..` holding `nets`.
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
    v["connectors"] =
        json!([{"reference": "J1", "part": part, "standard": standard, "pins": pins}]);
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
        let v = with_i2c(spec_json(vec![port("G", None, None, None)]), None, None);
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
    for pin in ["PB8", "PA5"] {
        let e = solve_refused(spec_json(vec![port("C", None, Some(pin), None)]));
        let SolveError::NoSerialPeripheral { port: who, reasons } = &e else {
            panic!("{pin}: {e}");
        };
        assert_eq!(who, "C");
        assert!(
            reasons.contains(&format!("USART1: `USART1_TX` is not on pin `{pin}`")),
            "{e}"
        );
        assert_eq!(reasons.len(), 4, "one reason per offered peripheral: {e}");
    }
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
    for (pin, kind) in [("PA13", "ReservedPin"), ("PZ1", "UnknownPin")] {
        let e = solve_refused(spec_json(vec![port("C", None, None, Some(pin))]));
        assert_eq!(e.kind(), kind, "{pin}");
        assert_eq!(e.claimants(), ["serial_ports[0].rx"]);
    }
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
        let mut p = port("C", None, None, None);
        p["framing"]["baud"] = json!(baud);
        assert_eq!(
            solve_refused(spec_json(vec![p])),
            SolveError::SerialBaudOutOfRange {
                port: "C".into(),
                baud,
                kernel_hz: 64_000_000
            }
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
fn adversarial_2_no_baud_a_brr_reaches_fails_the_tolerance_on_the_reset_clock() {
    // Each BRR's lowest and highest baud on 64 MHz, for every word: the
    // nearest BRR's error is at most 1 / (2 BRR), below every row.
    let clock = SolvedClock::reset();
    let k: u64 = 64_000_000;
    let mut checked = 0;
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
                    kernel_hz: k,
                    brr,
                    actual_millibaud: 0,
                    error_ppm: 0,
                };
                assert!(check_silicon(&clock, &p).holds, "{baud} {data_bits}");
                checked += 1;
            }
        }
    }
    assert!(checked > 50_000, "{checked}");
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
fn a_version_5_topology_without_serial_ports_is_refused_as_a_version() {
    let mut v = serde_json::to_value(topology()).unwrap();
    v["schema_version"] = json!(5);
    v.as_object_mut().unwrap().remove("serial_ports");
    let t: SolvedTopology = serde_json::from_value(v.clone()).unwrap();
    assert_eq!(t.validate().unwrap_err().kind(), "UnsupportedSchemaVersion");
    v["schema_version"] = json!(aethelgard_ir::SOLVED_SCHEMA_VERSION);
    assert!(serde_json::from_value::<SolvedTopology>(v).is_err());
}

#[test]
fn the_kernel_clock_names_its_select_field() {
    for p in PERIPHERALS {
        let field = serial::kernel_select(p);
        let expected = if matches!(p, "USART1" | "USART6") {
            "USART16SEL"
        } else {
            "USART234578SEL"
        };
        assert_eq!(field, expected, "{p}");
    }
}
