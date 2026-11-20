//! `AET-WO-1211a`: each reader of the directional `gpio`, `adc_inputs` and
//! `pin_devices` shapes refuses a topology it does not build, by name. The
//! topologies are the committed ones with a pin device, or a motor output
//! that idles high, inserted; each is judged valid first. The board
//! generator's, the twin's, hwdef's and pincheck's are in their own crates;
//! the schematic emitter's is here because `aethelgard-eda` holds no pin
//! table or solver to build its input.

use aethelgard_cli::generate::{GenerateError, generable};
use aethelgard_ir::motor::{Idle, Level};
use aethelgard_ir::pin_devices::{Direction, McuPull, SolvedGpio, SolvedPinDevice};
use aethelgard_ir::{SolvedTopology, Specification};

const SPEC: &str = "../../examples/sentinel-01-minimal.json";

fn spec() -> Specification {
    let bytes = std::fs::read(format!("{}/{SPEC}", env!("CARGO_MANIFEST_DIR"))).unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn solved() -> (Specification, SolvedTopology, aethelgard_ir::PinTable) {
    let spec = spec();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let topology = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    (spec, topology, table)
}

/// The topology with pin device `U9` (a wake switch) and its input `WAKE`.
fn with_pin_device() -> (Specification, SolvedTopology, aethelgard_ir::PinTable) {
    let (spec, mut t, table) = solved();
    t.pin_devices.insert(
        "U9".into(),
        SolvedPinDevice {
            part: "WAKE_SWITCH".into(),
        },
    );
    t.gpio.insert(
        "WAKE".into(),
        SolvedGpio {
            pin: "PB5".into(),
            direction: Direction::Input,
            initial: None,
            pull: Some(McuPull::Up),
        },
    );
    t.pin_speeds.insert("WAKE".into(), 0);
    t.validate().unwrap();
    (spec, t, table)
}

#[test]
fn generable_refuses_a_pin_device_by_reference_and_part() {
    let (_, t, _) = solved();
    generable(&t).unwrap();
    let (_, t, _) = with_pin_device();
    let err = generable(&t).unwrap_err();
    assert!(matches!(err, GenerateError::PinDeviceNotGenerated { .. }));
    assert_eq!(
        (err.kind(), err.id(), err.internal()),
        ("PinDeviceNotGenerated", Some("U9"), false)
    );
    assert!(err.to_string().contains("WAKE_SWITCH"), "{err}");
}

#[test]
fn generable_refuses_a_motor_output_that_idles_high() {
    let mut t: SolvedTopology = serde_json::from_str(include_str!(
        "../../aethelgard-svd-hal/tests/ui/motors-pwm/topology.json"
    ))
    .unwrap();
    generable(&t).unwrap();
    let net = t
        .motor_outputs
        .iter_mut()
        .find(|(_, m)| m.pwm.is_some())
        .map(|(net, m)| {
            m.pwm.as_mut().unwrap().idle = Idle::Level(Level::High);
            // An idle high's CCR is ARR + 1, as `solve` writes it.
            m.ccr = m.arr + 1;
            net.clone()
        })
        .unwrap();
    t.validate().unwrap();
    let err = generable(&t).unwrap_err();
    assert!(matches!(err, GenerateError::IdleHighNotGenerated { .. }));
    assert_eq!(
        (err.kind(), err.id(), err.internal()),
        ("IdleHighNotGenerated", Some(net.as_str()), false)
    );
}

#[test]
fn the_schematic_emitter_refuses_a_pin_device_by_reference_and_part() {
    let (spec, t, table) = with_pin_device();
    let err = aethelgard_eda::schematic::emit(&spec, &t, &table).unwrap_err();
    assert_eq!((err.kind(), err.id()), ("PinDeviceNotDrawn", Some("U9")));
    assert!(err.to_string().contains("WAKE_SWITCH"), "{err}");
}
