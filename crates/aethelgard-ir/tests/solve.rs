//! `solve` on a small synthetic `PinTable`: every refusal, the search order,
//! the reserved pins, determinism and the node budget. The real table is
//! tested through the CLI (`crates/aethelgard-cli/tests/solve_cli.rs`), since
//! this crate may depend on no other member.

use std::collections::BTreeMap;

use aethelgard_ir::{
    I2cBus, Mcu, Pin, PinSource, PinTable, RESERVED_PINS, Signal, SolveError, SolvedTopology,
    Specification, SpiBus, SpiDevice, VoltageDomain, solve, solve_with_limit,
};

const PART: &str = "TEST_PART";
const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// `(pin, [(signal, af)])`. Each reserved pin carries a signal the search
/// would otherwise take first, so a reserved pin assigned would show.
fn table_of(pins: &[(&str, &[(&str, u8)])]) -> PinTable {
    PinTable {
        part: PART.into(),
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
    table_of(&[
        ("PA0", &[]),
        ("PA1", &[]),
        ("PA13", &[("I2C1_SDA", 4), ("SPI1_SCK", 5)]),
        ("PA14", &[("I2C1_SCL", 4)]),
        ("PA5", &[("SPI1_SCK", 5), ("SPI3_SCK", 6)]),
        ("PA6", &[("SPI1_MISO", 5), ("SPI3_MISO", 6)]),
        ("PA7", &[("SPI1_MOSI", 5), ("SPI3_MOSI", 6)]),
        ("PB13", &[("SPI2_SCK", 5)]),
        ("PB14", &[("SPI2_MISO", 5)]),
        ("PB15", &[("SPI2_MOSI", 5)]),
        ("PB6", &[("I2C1_SCL", 4), ("I2C4_SCL", 6)]),
        ("PB7", &[("I2C1_SDA", 4), ("I2C4_SDA", 6)]),
        ("PB8", &[("I2C1_SCL", 4), ("I2C4_SCL", 6)]),
        ("PB9", &[("I2C1_SDA", 4), ("I2C4_SDA", 6)]),
        ("PC14", &[("I2C2_SDA", 4)]),
        ("PC15", &[("I2C2_SCL", 4)]),
        ("PF0", &[("I2C2_SDA", 4)]),
        ("PF1", &[("I2C2_SCL", 4)]),
        ("PH0", &[("I2C3_SDA", 4)]),
        ("PH1", &[("I2C3_SCL", 4)]),
        // I2C5 has an SDA and no SCL, so it is not offered.
        ("PJ0", &[("I2C5_SDA", 4)]),
    ])
}

fn signal(net: &str, pin: Option<&str>) -> Signal {
    Signal {
        net: net.into(),
        pin: pin.map(Into::into),
    }
}

fn i2c(id: &str) -> I2cBus {
    I2cBus {
        id: id.into(),
        peripheral: None,
        speed_hz: 400_000,
        pull_up_ohms: 2200,
        domain: "VDD_3V3".into(),
        sda: signal(&format!("{id}_SDA"), None),
        scl: signal(&format!("{id}_SCL"), None),
    }
}

fn spi(id: &str) -> SpiBus {
    SpiBus {
        id: id.into(),
        peripheral: None,
        speed_hz: 1_000_000,
        domain: "VDD_3V3".into(),
        sck: signal(&format!("{id}_SCK"), None),
        mosi: signal(&format!("{id}_MOSI"), None),
        miso: signal(&format!("{id}_MISO"), None),
    }
}

fn device(reference: &str, bus: &str) -> SpiDevice {
    SpiDevice {
        reference: reference.into(),
        part: "FLASH".into(),
        supply: "VDD_3V3".into(),
        bus: bus.into(),
        cs: signal(&format!("{reference}_CS"), None),
    }
}

fn spec(
    i2c_buses: Vec<I2cBus>,
    spi_buses: Vec<SpiBus>,
    spi_devices: Vec<SpiDevice>,
) -> Specification {
    Specification {
        schema_version: 1,
        project_name: "t".into(),
        mcu: Mcu {
            part: PART.into(),
            supply: "VDD_3V3".into(),
        },
        voltage_domains: vec![VoltageDomain {
            id: "VDD_3V3".into(),
            nominal_millivolts: 3300,
            tolerance_permille: 50,
        }],
        i2c_buses,
        spi_buses,
        i2c_devices: vec![],
        spi_devices,
        net_constraints: vec![],
    }
}

/// One I2C bus, one SPI bus with one device.
fn basic() -> Specification {
    spec(vec![i2c("I")], vec![spi("S")], vec![device("U1", "S")])
}

fn ok(s: &Specification) -> SolvedTopology {
    solve(s, &table(), HASH).expect("solvable")
}

fn refused(s: &Specification) -> SolveError {
    solve(s, &table(), HASH).expect_err("refused")
}

fn pin_of(t: &SolvedTopology, net: &str) -> String {
    t.assignments[net].pin.clone()
}

#[test]
fn basic_solves_in_the_fixed_order_and_validates() {
    let t = ok(&basic());
    assert_eq!(t.validate(), Ok(()));
    assert_eq!(t.specification_sha256, HASH);
    // I2C1 first; its first pins in key order, PA13/PA14 being reserved.
    let sda = &t.assignments["I_SDA"];
    assert_eq!(
        (
            sda.pin.as_str(),
            sda.peripheral.as_str(),
            sda.signal.as_str(),
            sda.alternate_function
        ),
        ("PB7", "I2C1", "I2C1_SDA", 4)
    );
    assert_eq!(pin_of(&t, "I_SCL"), "PB6");
    // SPI1 on PA5/PA7/PA6, never PA13.
    assert_eq!(t.assignments["S_SCK"].peripheral, "SPI1");
    assert_eq!(pin_of(&t, "S_SCK"), "PA5");
    assert_eq!(pin_of(&t, "S_MOSI"), "PA7");
    assert_eq!(pin_of(&t, "S_MISO"), "PA6");
    // cs last: the first free unreserved pin in key order.
    assert_eq!(t.gpio, BTreeMap::from([("U1_CS".into(), "PA0".into())]));
}

#[test]
fn two_solves_are_byte_identical() {
    let a = serde_json::to_string(&ok(&basic())).unwrap();
    let b = serde_json::to_string(&ok(&basic())).unwrap();
    assert_eq!(a, b);
}

#[test]
fn no_reserved_pin_is_ever_assigned() {
    // Enough I2C buses to use every offered instance, and cs devices to eat
    // through the free pins: still no reserved pin.
    let s = spec(
        vec![i2c("A"), i2c("B")],
        vec![spi("S"), spi("T")],
        vec![device("U1", "S"), device("U2", "S"), device("U3", "T")],
    );
    let t = ok(&s);
    let used: Vec<&str> = t
        .assignments
        .values()
        .map(|a| a.pin.as_str())
        .chain(t.gpio.values().map(String::as_str))
        .collect();
    for pin in RESERVED_PINS {
        assert!(!used.contains(&pin), "{pin} assigned: {used:?}");
    }
    // I2C2 and I2C3 live only on reserved pins (or I2C2 on PF0/PF1).
    assert_eq!(t.assignments["B_SDA"].peripheral, "I2C2");
    assert_eq!(pin_of(&t, "B_SDA"), "PF0");
}

#[test]
fn reserved_pins_are_the_debug_port_and_the_oscillators() {
    assert_eq!(
        RESERVED_PINS,
        ["PA13", "PA14", "PC14", "PC15", "PH0", "PH1"]
    );
}

#[test]
fn a_hand_pin_on_each_reserved_pin_refuses() {
    for pin in RESERVED_PINS {
        let mut s = basic();
        s.spi_devices[0].cs.pin = Some(pin.into());
        assert_eq!(
            refused(&s),
            SolveError::ReservedPin {
                pin: pin.into(),
                claimant: "spi_devices[0].cs".into()
            }
        );
    }
}

#[test]
fn hand_values_are_kept() {
    let mut s = basic();
    s.i2c_buses[0].sda.pin = Some("PB9".into());
    s.spi_buses[0].peripheral = Some("SPI3".into());
    s.spi_devices[0].cs.pin = Some("PA1".into());
    let t = ok(&s);
    assert_eq!(pin_of(&t, "I_SDA"), "PB9");
    assert_eq!(t.assignments["S_SCK"].peripheral, "SPI3");
    assert_eq!(t.assignments["S_SCK"].alternate_function, 6);
    assert_eq!(t.gpio["U1_CS"], "PA1");
}

#[test]
fn a_hand_cs_pin_is_not_given_to_a_bus() {
    let mut s = basic();
    s.spi_devices[0].cs.pin = Some("PB7".into());
    let t = ok(&s);
    assert_eq!(pin_of(&t, "I_SDA"), "PB9");
    assert_eq!(t.gpio["U1_CS"], "PB7");
}

#[test]
fn a_pin_claimed_twice_names_both_claimants_in_document_order() {
    let mut s = basic();
    s.i2c_buses[0].sda.pin = Some("PB9".into());
    s.spi_devices[0].cs.pin = Some("PB9".into());
    let err = refused(&s);
    assert_eq!(err.kind(), "PinClaimedTwice");
    assert_eq!(err.id(), Some("PB9"));
    assert_eq!(err.claimants(), ["i2c_buses[0].sda", "spi_devices[0].cs"]);
    assert_eq!(
        err.to_string(),
        "pin `PB9` is claimed by both `i2c_buses[0].sda` and `spi_devices[0].cs`"
    );
}

#[test]
fn an_unknown_pin_is_refused_by_key_never_by_case() {
    let mut s = basic();
    s.i2c_buses[0].scl.pin = Some("pb8".into());
    let err = refused(&s);
    assert_eq!(
        err,
        SolveError::UnknownPin {
            pin: "pb8".into(),
            claimant: "i2c_buses[0].scl".into()
        }
    );
    assert_eq!(err.claimants(), ["i2c_buses[0].scl"]);
}

#[test]
fn an_unknown_or_wrong_kind_peripheral_is_refused() {
    for name in ["I2C5", "I2C9", "SPI1"] {
        let mut s = basic();
        s.i2c_buses[0].peripheral = Some(name.into());
        let err = refused(&s);
        assert_eq!(
            err,
            SolveError::UnknownPeripheral {
                peripheral: name.into(),
                claimant: "i2c_buses[0]".into()
            }
        );
    }
}

#[test]
fn a_peripheral_claimed_twice_names_both_buses() {
    let mut s = spec(vec![i2c("A"), i2c("B")], vec![], vec![]);
    s.i2c_buses[0].peripheral = Some("I2C4".into());
    s.i2c_buses[1].peripheral = Some("I2C4".into());
    let err = refused(&s);
    assert_eq!(err.kind(), "PeripheralClaimedTwice");
    assert_eq!(err.claimants(), ["i2c_buses[0]", "i2c_buses[1]"]);
}

#[test]
fn a_bus_no_instance_can_serve_says_why_for_each_instance() {
    let mut s = basic();
    s.i2c_buses[0].sda.pin = Some("PB6".into());
    let err = refused(&s);
    assert_eq!(err.kind(), "NoInstance");
    assert_eq!(err.claimants(), ["I"]);
    assert_eq!(
        err.to_string(),
        "no instance can serve bus `I`: I2C1: `I2C1_SDA` is not on pin `PB6`; \
         I2C2: `I2C2_SDA` is not on pin `PB6`; I2C3: `I2C3_SDA` is not on pin `PB6`; \
         I2C4: `I2C4_SDA` is not on pin `PB6`"
    );

    let mut s = basic();
    s.i2c_buses[0].peripheral = Some("I2C3".into());
    assert_eq!(
        refused(&s).to_string(),
        "no instance can serve bus `I`: I2C3: the table has no unreserved pin for `I2C3_SDA`"
    );

    let mut s = basic();
    s.spi_buses[0].peripheral = Some("SPI2".into());
    s.spi_devices[0].cs.pin = Some("PB13".into());
    assert_eq!(
        refused(&s).to_string(),
        "no instance can serve bus `S`: SPI2: every pin for `SPI2_SCK` is held by hand \
         (`PB13` by `spi_devices[0].cs`)"
    );
}

#[test]
fn infeasible_names_a_minimal_set_of_buses() {
    // SPI1 and SPI3 share PA5, PA6 and PA7 here, so the buses forced onto
    // them conflict over pins, not instances. The free B is dropped.
    let mut s = spec(vec![], vec![spi("A"), spi("B"), spi("C")], vec![]);
    s.spi_buses[0].peripheral = Some("SPI1".into());
    s.spi_buses[2].peripheral = Some("SPI3".into());
    let err = refused(&s);
    assert_eq!(err.kind(), "Infeasible");
    assert_eq!(err.id(), None);
    assert_eq!(err.claimants(), ["A", "C"]);
    assert_eq!(
        err.to_string(),
        "buses `A`, `C` cannot all be placed at once (a minimal set: dropping any one makes the rest placeable)"
    );
}

#[test]
fn the_set_is_minimal_not_minimum() {
    // With a fourth bus, {A, D} is the smallest conflict, but deletion in
    // document order drops A first (B, C and D still cannot all fit, as one
    // of B and C must take SPI1 beside D's SPI3) and keeps {B, C, D}, which
    // is irreducible.
    let mut s = spec(vec![], vec![spi("A"), spi("B"), spi("C"), spi("D")], vec![]);
    s.spi_buses[0].peripheral = Some("SPI1".into());
    s.spi_buses[3].peripheral = Some("SPI3".into());
    assert_eq!(refused(&s).claimants(), ["B", "C", "D"]);
}

#[test]
fn a_pigeonhole_is_refused_at_once() {
    // Four I2C buses, three placeable instances (I2C3 lives only on reserved
    // pins): refused at the root, and every bus is in the minimal set.
    let s = spec(vec![i2c("A"), i2c("B"), i2c("C"), i2c("D")], vec![], vec![]);
    let err = solve_with_limit(&s, &table(), HASH, 1_000).unwrap_err();
    assert_eq!(err.claimants(), ["A", "B", "C", "D"], "{err}");
}

#[test]
fn the_budget_refuses_naming_the_bus_being_placed() {
    let s = basic();
    let err = solve_with_limit(&s, &table(), HASH, 3).unwrap_err();
    assert_eq!(
        err,
        SolveError::BudgetExceeded {
            bus: "S".into(),
            limit: 3
        }
    );
    assert_eq!(err.claimants(), ["S"]);
    assert_eq!(
        err.to_string(),
        "the search tried 3 values without an answer; it was placing bus `S`"
    );
    // The same specification within budget solves.
    assert!(solve_with_limit(&s, &table(), HASH, 100).is_ok());
}

#[test]
fn the_budget_covers_the_minimal_set_too() {
    let s = spec(vec![i2c("A"), i2c("B"), i2c("C"), i2c("D")], vec![], vec![]);
    // Enough for the first search's refusal, not for the deletions after it.
    let first = (1..1_000)
        .find(|&n| {
            !matches!(
                solve_with_limit(&s, &table(), HASH, n),
                Err(SolveError::BudgetExceeded { .. })
            )
        })
        .expect("a budget that finishes");
    assert_eq!(
        solve_with_limit(&s, &table(), HASH, first)
            .unwrap_err()
            .kind(),
        "Infeasible"
    );
    assert_eq!(
        solve_with_limit(&s, &table(), HASH, first - 1)
            .unwrap_err()
            .kind(),
        "BudgetExceeded"
    );
}

#[test]
fn a_cs_with_no_free_pin_left_is_refused() {
    let small = table_of(&[
        ("PA5", &[("SPI1_SCK", 5)]),
        ("PA6", &[("SPI1_MISO", 5)]),
        ("PA7", &[("SPI1_MOSI", 5)]),
        ("PB0", &[]),
        ("PA13", &[]),
    ]);
    let s = spec(
        vec![],
        vec![spi("S")],
        vec![device("U1", "S"), device("U2", "S")],
    );
    let err = solve(&s, &small, HASH).unwrap_err();
    assert_eq!(
        err,
        SolveError::NoFreePin {
            claimant: "spi_devices[1].cs".into()
        }
    );
}

#[test]
fn an_invalid_specification_hash_or_part_is_refused_before_the_search() {
    let mut s = basic();
    s.spi_devices[0].cs.net = "I_SDA".into();
    let err = refused(&s);
    assert_eq!(err.kind(), "DuplicateNetName");
    assert!(matches!(err, SolveError::InvalidSpecification(_)));

    let err = solve(&basic(), &table(), "not-a-hash").unwrap_err();
    assert_eq!(err.kind(), "MalformedSpecificationHash");

    let mut s = basic();
    s.mcu.part = "OTHER".into();
    assert_eq!(
        refused(&s),
        SolveError::PartMismatch {
            spec: "OTHER".into(),
            table: PART.into()
        }
    );
}

#[test]
fn refusals_come_in_the_readme_order() {
    // An unknown pin is found before a peripheral problem, which is found
    // before a bus no instance can serve.
    let mut s = basic();
    s.i2c_buses[0].peripheral = Some("I2C9".into());
    s.spi_buses[0].mosi.pin = Some("NOPE".into());
    assert_eq!(refused(&s).kind(), "UnknownPin");
    let mut s = basic();
    s.i2c_buses[0].peripheral = Some("I2C9".into());
    s.spi_buses[0].mosi.pin = Some("PB6".into());
    assert_eq!(refused(&s).kind(), "UnknownPeripheral");
}
