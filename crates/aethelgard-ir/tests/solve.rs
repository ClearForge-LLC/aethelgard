//! `solve` on a small synthetic `PinTable`: every refusal, the search order,
//! the reserved pins, determinism and the node budget. The real table is
//! tested through the CLI (`crates/aethelgard-cli/tests/solve_cli.rs`), since
//! this crate may depend on no other member.

use std::collections::BTreeMap;

mod common;

use common::devs;

use aethelgard_ir::{
    AnalogLine, I2cBus, Mcu, NetConstraint, Pin, PinSource, PinTable, RESERVED_PINS, Signal,
    SolveError, SolvedTopology, Specification, SpiBus, SpiDevice, VoltageDomain, solve,
    solve_with_limit,
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

fn constraint(net: &str, rise_time_ns: u32) -> NetConstraint {
    NetConstraint {
        net_name: net.into(),
        max_capacitance_femtofarads: 100_000,
        max_rise_time_nanoseconds: rise_time_ns,
        impedance_ohms: None,
    }
}

/// Every I2C line gets a 300 ns constraint, so each bus's ceiling is 400 kHz,
/// its declared speed, unless a test changes it.
fn spec(
    i2c_buses: Vec<I2cBus>,
    spi_buses: Vec<SpiBus>,
    spi_devices: Vec<SpiDevice>,
) -> Specification {
    let net_constraints: Vec<NetConstraint> = i2c_buses
        .iter()
        .flat_map(|b| [constraint(&b.sda.net, 300), constraint(&b.scl.net, 300)])
        .collect();
    // The device lists are private to the crate (`AET-WO-1204`), so the
    // specification is built from the JSON it reads.
    let json = serde_json::json!({
        "schema_version": 1,
        "project_name": "t",
        "mcu": Mcu { part: PART.into(), supply: "VDD_3V3".into() },
        "voltage_domains": [VoltageDomain {
            id: "VDD_3V3".into(),
            nominal_millivolts: 3300,
            tolerance_permille: 50,
        }],
        "i2c_buses": i2c_buses,
        "spi_buses": spi_buses,
        "i2c_devices": [],
        "spi_devices": spi_devices,
        "net_constraints": net_constraints,
    });
    serde_json::from_value(json).expect("the fixture is a specification")
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
        devs(&mut s).spi[0].cs.pin = Some(pin.into());
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
    devs(&mut s).spi[0].cs.pin = Some("PA1".into());
    let t = ok(&s);
    assert_eq!(pin_of(&t, "I_SDA"), "PB9");
    assert_eq!(t.assignments["S_SCK"].peripheral, "SPI3");
    assert_eq!(t.assignments["S_SCK"].alternate_function, 6);
    assert_eq!(t.gpio["U1_CS"], "PA1");
}

#[test]
fn a_hand_cs_pin_is_not_given_to_a_bus() {
    let mut s = basic();
    devs(&mut s).spi[0].cs.pin = Some("PB7".into());
    let t = ok(&s);
    assert_eq!(pin_of(&t, "I_SDA"), "PB9");
    assert_eq!(t.gpio["U1_CS"], "PB7");
}

#[test]
fn a_pin_claimed_twice_names_both_claimants_in_document_order() {
    let mut s = basic();
    s.i2c_buses[0].sda.pin = Some("PB9".into());
    devs(&mut s).spi[0].cs.pin = Some("PB9".into());
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
    devs(&mut s).spi[0].cs.pin = Some("PB13".into());
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

/// The nodes a solve spends: the smallest budget it finishes within.
fn nodes(s: &Specification) -> u64 {
    (0..100_000)
        .find(|&n| {
            !matches!(
                solve_with_limit(s, &table(), HASH, n),
                Err(SolveError::BudgetExceeded { .. })
            )
        })
        .expect("finishes within 100000 nodes")
}

#[test]
fn the_budget_covers_the_whole_solve_minimal_set_included() {
    // Four buses over three placeable instances: refused at the root before
    // any node, so every node is spent in the minimal-set searches, one per
    // bus, each feasible and identical to solving the other three alone.
    let buses = ["A", "B", "C", "D"];
    let s = spec(buses.iter().map(|b| i2c(b)).collect(), vec![], vec![]);
    assert_eq!(refused(&s).kind(), "Infeasible");
    let alone: Vec<u64> = (0..buses.len())
        .map(|drop| {
            let rest = buses.iter().enumerate().filter(|(i, _)| *i != drop);
            nodes(&spec(rest.map(|(_, b)| i2c(b)).collect(), vec![], vec![]))
        })
        .collect();
    assert!(alone.iter().filter(|n| **n > 0).count() > 1, "{alone:?}");
    // A budget per search would make this the maximum, not the sum.
    assert_eq!(nodes(&s), alone.iter().sum::<u64>(), "{alone:?}");
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
    devs(&mut s).spi[0].cs.net = "I_SDA".into();
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
fn a_malformed_hash_is_refused_before_the_search_finds_infeasibility() {
    let s = spec(vec![i2c("A"), i2c("B"), i2c("C"), i2c("D")], vec![], vec![]);
    assert_eq!(refused(&s).kind(), "Infeasible");
    let err = solve(&s, &table(), "not-a-hash").unwrap_err();
    assert_eq!(err.kind(), "MalformedSpecificationHash");
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

// ---- I2C ceilings (AET-WO-1202) ---------------------------------------------

/// One I2C bus at `speed_hz` whose two lines declare these rise times.
fn i2c_spec(speed_hz: u32, sda_ns: u32, scl_ns: u32) -> Specification {
    let mut s = spec(vec![i2c("I")], vec![], vec![]);
    s.i2c_buses[0].speed_hz = speed_hz;
    s.net_constraints = vec![constraint("I_SDA", sda_ns), constraint("I_SCL", scl_ns)];
    s
}

#[test]
fn each_i2c_bus_gets_its_ceiling_and_an_spi_only_spec_gets_none() {
    let t = ok(&basic());
    assert_eq!(t.i2c_ceilings, BTreeMap::from([("I".into(), 400_000)]));
    let t = ok(&spec(vec![], vec![spi("S")], vec![]));
    assert!(t.i2c_ceilings.is_empty());
    assert!(t.i2c_speeds.is_empty());
}

#[test]
fn each_i2c_bus_carries_its_declared_speed_not_its_ceiling() {
    // AET-WO-1205: a 100 kHz bus under a 400 kHz ceiling.
    let t = ok(&i2c_spec(100_000, 300, 300));
    assert_eq!(t.schema_version, aethelgard_ir::SOLVED_SCHEMA_VERSION);
    assert_eq!(t.i2c_ceilings["I"], 400_000);
    assert_eq!(t.i2c_speeds, BTreeMap::from([("I".into(), 100_000)]));
    assert_eq!(t.validate(), Ok(()));
}

/// `AET-WO-2200` §1.1: each I2C line carries its bus's pull-up and domain
/// voltage and its own constraint's capacitance and rise-time bound, keyed by
/// net; an SPI line carries none.
#[test]
fn each_i2c_line_gets_its_analog_facts_from_the_specification() {
    let mut s = spec(vec![i2c("I"), i2c("J")], vec![spi("S")], vec![]);
    s.i2c_buses[1].pull_up_ohms = 47_000;
    s.net_constraints[1].max_capacitance_femtofarads = 250_000;
    s.net_constraints[2].max_rise_time_nanoseconds = 1000;
    s.i2c_buses[1].speed_hz = 100_000;
    s.voltage_domains[0].nominal_millivolts = 1800;
    let t = ok(&s);
    let line = |bus: &str, ohms, ff, ns| AnalogLine {
        bus: bus.into(),
        pull_up_ohms: ohms,
        capacitance_femtofarads: ff,
        ceiling_ns: ns,
        domain_millivolts: 1800,
    };
    assert_eq!(
        t.analog_lines,
        BTreeMap::from([
            ("I_SDA".into(), line("I", 2200, 100_000, 300)),
            ("I_SCL".into(), line("I", 2200, 250_000, 300)),
            ("J_SDA".into(), line("J", 47_000, 100_000, 1000)),
            ("J_SCL".into(), line("J", 47_000, 100_000, 300)),
        ])
    );
    assert!(
        ok(&spec(vec![], vec![spi("S")], vec![]))
            .analog_lines
            .is_empty()
    );
}

#[test]
fn the_ceiling_follows_the_mapping_at_each_boundary() {
    for (ns, hz) in [
        (120, 1_000_000),
        (121, 400_000),
        (300, 400_000),
        (301, 100_000),
        (1000, 100_000),
    ] {
        let t = ok(&i2c_spec(100_000, ns, ns));
        assert_eq!(t.i2c_ceilings["I"], hz, "{ns} ns");
    }
}

/// As ruled (WO §1.2, §5.3): the smaller declared rise time sets the ceiling.
/// AET-WO-1202 FEEDBACK F1 argues the bus is as slow as its worse line, i.e.
/// the larger value; if that is ruled, every assertion here inverts.
#[test]
fn the_stricter_line_sets_the_ceiling_whichever_it_is() {
    assert_eq!(
        ok(&i2c_spec(100_000, 120, 1000)).i2c_ceilings["I"],
        1_000_000
    );
    assert_eq!(
        ok(&i2c_spec(100_000, 1000, 120)).i2c_ceilings["I"],
        1_000_000
    );
    // At 1 MHz, the stricter line (120 ns) beats the looser one (300 ns).
    assert_eq!(
        ok(&i2c_spec(1_000_000, 120, 300)).i2c_ceilings["I"],
        1_000_000
    );
    assert_eq!(
        ok(&i2c_spec(1_000_000, 300, 120)).i2c_ceilings["I"],
        1_000_000
    );
}

#[test]
fn a_line_with_no_constraint_is_refused_naming_the_first_in_document_order() {
    let mut s = i2c_spec(100_000, 300, 300);
    s.net_constraints.clear();
    let err = refused(&s);
    assert_eq!(
        err,
        SolveError::MissingBusConstraint {
            bus: "I".into(),
            net: "I_SDA".into()
        }
    );
    assert_eq!(err.kind(), "MissingBusConstraint");
    assert_eq!(err.id(), Some("I"));
    assert_eq!(err.claimants(), ["I"]);
    // Only SCL missing: SCL is named; one line never stands for both.
    let mut s = i2c_spec(100_000, 300, 300);
    s.net_constraints.retain(|c| c.net_name == "I_SDA");
    assert_eq!(
        refused(&s),
        SolveError::MissingBusConstraint {
            bus: "I".into(),
            net: "I_SCL".into()
        }
    );
}

#[test]
fn a_rise_time_slower_than_every_mode_is_refused() {
    let err = refused(&i2c_spec(100_000, 1001, 2000));
    assert_eq!(
        err,
        SolveError::RiseTimeTooSlow {
            bus: "I".into(),
            rise_time_ns: 1001
        }
    );
    assert_eq!(
        err.to_string(),
        "I2C bus `I` allows a rise time of 1001 ns, slower than any I2C mode (1000 ns at most)"
    );
}

#[test]
fn a_bus_above_its_ceiling_is_refused() {
    let err = refused(&i2c_spec(1_000_000, 300, 300));
    assert_eq!(err.kind(), "BusTooFast");
    assert_eq!(
        err.to_string(),
        "I2C bus `I` runs at 1000000 Hz, above its rise-time ceiling of 400000 Hz"
    );
    // At its ceiling it solves.
    assert_eq!(ok(&i2c_spec(400_000, 300, 300)).i2c_ceilings["I"], 400_000);
}

#[test]
fn ceiling_refusals_come_before_the_pin_search_and_after_validation() {
    // A spec the search could not place still refuses on its ceiling first.
    let mut s = spec(vec![i2c("A"), i2c("B"), i2c("C"), i2c("D")], vec![], vec![]);
    assert_eq!(refused(&s).kind(), "Infeasible");
    s.net_constraints.retain(|c| c.net_name != "D_SCL");
    assert_eq!(refused(&s).kind(), "MissingBusConstraint");
    // An invalid spec still refuses on validation first.
    s.net_constraints.clear();
    s.voltage_domains[0].nominal_millivolts = 0;
    assert_eq!(refused(&s).kind(), "ZeroNominalVoltage");
}

/// A pull-up far above the floor and a capacitance at the limit leave the
/// ceiling to the rise time alone (`AET-WO-1203`: the rules only refuse).
#[test]
fn pull_up_and_capacitance_within_the_rules_do_not_move_the_ceiling() {
    let mut s = i2c_spec(400_000, 300, 300);
    s.i2c_buses[0].pull_up_ohms = 47_000;
    for c in &mut s.net_constraints {
        c.max_capacitance_femtofarads = 400_000;
    }
    assert_eq!(ok(&s).i2c_ceilings["I"], 400_000);
}

fn with_pull_up(speed_hz: u32, ohms: u32) -> Specification {
    let mut s = i2c_spec(speed_hz, 120, 120);
    s.i2c_buses[0].pull_up_ohms = ohms;
    s
}

/// `AET-WO-1203` §3: at 3.3 V and 400 kHz, 966 ohms is refused and 967 solves.
#[test]
fn the_pull_up_floor_at_3v3_and_400khz() {
    assert_eq!(
        refused(&with_pull_up(400_000, 966)),
        SolveError::PullUpBelowSinkFloor {
            bus: "I".into(),
            pull_up_ohms: 966,
            floor_ohms: 967
        }
    );
    ok(&with_pull_up(400_000, 967));
    ok(&basic());
}

/// The review's A2 inputs.
#[test]
fn the_reviews_24_50_and_100_ohm_pull_ups_are_refused() {
    for ohms in [24, 50, 100] {
        let err = refused(&with_pull_up(400_000, ohms));
        assert_eq!(err.kind(), "PullUpBelowSinkFloor", "{ohms}");
        assert_eq!(err.id(), Some("I"));
        assert_eq!(err.claimants(), vec!["I".to_owned()]);
    }
    assert_eq!(
        refused(&with_pull_up(400_000, 24)).to_string(),
        "I2C bus `I` has a pull-up of 24 ohms, below the 967 ohm floor its rail and mode set (UM10204 sink current)"
    );
}

#[test]
fn the_pull_up_floor_follows_the_bus_rail_and_mode() {
    // Fast-mode Plus sinks 20 mA: 145 ohms at 3.3 V.
    ok(&with_pull_up(1_000_000, 145));
    assert_eq!(
        refused(&with_pull_up(1_000_000, 144)).kind(),
        "PullUpBelowSinkFloor"
    );
    // 1.8 V reads VOL as 0.2 x VDD at 2 mA, in every mode: 720 ohms.
    for speed in [100_000, 400_000, 1_000_000] {
        let mut s = with_pull_up(speed, 720);
        s.voltage_domains[0].nominal_millivolts = 1800;
        ok(&s);
        s.i2c_buses[0].pull_up_ohms = 719;
        assert_eq!(refused(&s).kind(), "PullUpBelowSinkFloor", "{speed}");
    }
}

fn with_capacitance(speed_hz: u32, sda_ff: u64, scl_ff: u64) -> Specification {
    let mut s = i2c_spec(speed_hz, 120, 120);
    s.net_constraints[0].max_capacitance_femtofarads = sda_ff;
    s.net_constraints[1].max_capacitance_femtofarads = scl_ff;
    s
}

/// `AET-WO-1203` §3: 400 pF solves at 400 kHz and 401 pF is refused; 550 pF
/// solves at 1 MHz.
#[test]
fn the_capacitance_limit_per_mode() {
    ok(&with_capacitance(400_000, 400_000, 400_000));
    assert_eq!(
        refused(&with_capacitance(400_000, 401_000, 100_000)),
        SolveError::BusCapacitanceOverLimit {
            net: "I_SDA".into(),
            capacitance_femtofarads: 401_000,
            limit_femtofarads: 400_000
        }
    );
    let err = refused(&with_capacitance(100_000, 100_000, 401_000));
    assert_eq!(err.id(), Some("I_SCL"));
    ok(&with_capacitance(1_000_000, 550_000, 550_000));
    assert_eq!(
        refused(&with_capacitance(1_000_000, 550_001, 1)).kind(),
        "BusCapacitanceOverLimit"
    );
    assert_eq!(
        refused(&with_capacitance(400_000, u64::MAX, 1)).kind(),
        "BusCapacitanceOverLimit"
    );
}

/// The rules sit after the ceilings and before the pin search.
#[test]
fn the_electrical_rules_come_after_the_ceilings() {
    let mut s = with_pull_up(1_000_000, 24);
    for c in &mut s.net_constraints {
        c.max_rise_time_nanoseconds = 300;
    }
    assert_eq!(refused(&s).kind(), "BusTooFast");
    let mut s = with_pull_up(400_000, 24);
    s.net_constraints[0].max_capacitance_femtofarads = 500_000;
    assert_eq!(refused(&s).kind(), "PullUpBelowSinkFloor");
}

/// `AET-WO-2203` §1.1: `solve` copies each device, on the peripheral its bus
/// was solved to, and its place in the specification's order, which the map
/// (sorted by reference: `U10` before `U3`) does not keep.
#[test]
fn devices_are_copied_with_their_peripheral_and_their_place_in_the_order() {
    let mut s = basic();
    let dev = |reference: &str, address: u8| aethelgard_ir::I2cDevice {
        reference: reference.into(),
        part: "BME280".into(),
        supply: "VDD_3V3".into(),
        bus: "I".into(),
        address,
    };
    devs(&mut s).i2c = vec![dev("U10", 0x76), dev("U3", 0x77), dev("U2", 0x40)];
    let t = ok(&s);
    t.validate().unwrap();
    let peripheral = t.assignments["I_SDA"].peripheral.clone();
    let got: Vec<(&str, u32, u8, &str)> = t
        .i2c_devices_in_order()
        .into_iter()
        .map(|(r, d)| (r, d.index, d.address, d.peripheral.as_str()))
        .collect();
    assert_eq!(
        got,
        [
            ("U10", 0, 0x76, peripheral.as_str()),
            ("U3", 1, 0x77, peripheral.as_str()),
            ("U2", 2, 0x40, peripheral.as_str())
        ]
    );
    assert_eq!(t.i2c_devices["U3"].bus, "I");
    assert_eq!(t.i2c_devices["U3"].part, "BME280");
}

#[test]
fn a_specification_with_no_device_solves_to_none() {
    assert!(ok(&basic()).i2c_devices.is_empty());
}
