//! `AET-WO-1211a`: each reader of the directional `gpio`, `adc_inputs` and
//! `pin_devices` shapes refuses a topology it does not build, by name. The
//! topologies are the committed ones with a pin device, or a motor output
//! that idles high, inserted; each is judged valid first. The board
//! generator's, the twin's, hwdef's and pincheck's are in their own crates;
//! the schematic emitter's is here because `aethelgard-eda` holds no pin
//! table or solver to build its input. `AET-WO-1211b` lifts both at
//! `generate`, and `AET-WO-1211c` at `simulate`: what of a pin device the
//! twin cannot run is refused by its plan (`aethelgard-twin`'s `pins`), and
//! an idle high only where no settled window remains.

use aethelgard_cli::generate::{GenerateError, generable, simulable};
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
            bridges: Default::default(),
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
fn a_pin_device_is_generated_and_simulable() {
    let (_, t, _) = solved();
    simulable(&t).unwrap();
    let (_, t, _) = with_pin_device();
    generable(&t).unwrap();
    simulable(&t).unwrap();
}

/// An idle high is simulated (`AET-WO-1211c`, Q10): judged from one period
/// after its timer's `CEN`. It is refused by name only where two of its
/// periods exceed the boot, so no settled window would remain.
#[test]
fn a_motor_output_that_idles_high_is_refused_at_simulate_only_without_a_window() {
    let mut t: SolvedTopology = serde_json::from_str(include_str!(
        "../../aethelgard-svd-hal/tests/ui/motors-pwm/topology.json"
    ))
    .unwrap();
    simulable(&t).unwrap();
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
    generable(&t).unwrap();
    // 400 Hz: two periods are 5 ms of the boot's 100 ms.
    simulable(&t).unwrap();
    // 19 Hz: two periods are 105 ms.
    t.motor_outputs
        .get_mut(&net)
        .unwrap()
        .pwm
        .as_mut()
        .unwrap()
        .rate_hz = 19;
    let err = simulable(&t).unwrap_err();
    assert!(matches!(err, GenerateError::IdleHighNotSimulated { .. }));
    assert_eq!(
        (err.kind(), err.id(), err.internal()),
        ("IdleHighNotSimulated", Some(net.as_str()), false)
    );
}

/// D1 (`AET-WO-1211a`'s fixture) generates its board crate and its Renode
/// platform (`AET-WO-1211c`; until it, no platform), and no `kicad/`: its
/// board is not drawn.
#[test]
fn d1_generates_a_board_crate_and_its_platform() {
    let bytes = std::fs::read(format!(
        "{}/../aethelgard-ir/tests/fixtures/d1-drv8876.json",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let spec: Specification = serde_json::from_slice(&bytes).unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let t = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    let files = aethelgard_cli::generate::tree(&spec, &t, &table).unwrap();
    assert!(files.contains_key("board/src/board.rs"));
    assert_eq!(
        files.keys().filter(|k| k.ends_with(".repl")).count(),
        1,
        "{:?}",
        files.keys()
    );
    assert!(files.keys().all(|k| !k.starts_with("kicad/")));
}

#[test]
fn the_schematic_emitter_refuses_a_pin_device_by_reference_and_part() {
    let (spec, t, table) = with_pin_device();
    let err = aethelgard_eda::schematic::emit(&spec, &t, &table).unwrap_err();
    assert_eq!((err.kind(), err.id()), ("PinDeviceNotDrawn", Some("U9")));
    assert!(err.to_string().contains("WAKE_SWITCH"), "{err}");
}

/// `AET-WO-1211b` §1.1: a `power.sense` input is in `adc_inputs`
/// (`AET-WO-1211a`), but the generator brings up only a pin device's ADC
/// inputs, so a sensed board's firmware is the one `main` generated: no
/// analog pin, no ADC.
#[test]
fn a_power_sense_input_is_not_generated() {
    let bytes = std::fs::read(format!(
        "{}/tests/fixtures/sentinel-03.json",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let spec: Specification = serde_json::from_slice(&bytes).unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let t = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    assert!(!t.adc_inputs.is_empty(), "the fixture senses its battery");
    let files = aethelgard_cli::generate::tree(&spec, &t, &table).unwrap();
    for (path, text) in files.iter().filter(|(p, _)| p.ends_with(".rs")) {
        assert!(
            !text.contains("into_analog") && !text.contains("Adc"),
            "{path}"
        );
    }
}
