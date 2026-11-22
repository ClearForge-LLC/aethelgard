//! `AET-WO-1211c`: the twin's plan of a pin device (`pins::plan`), from the
//! solved topology alone (`RUL-16`; `AET-WO-1211c` D7: an input's `active`
//! and `pulled`, an ADC input's `transfer` are solved): D1's, and each
//! refusal by its name.

use aethelgard_ir::SolvedTopology;
use aethelgard_twin::pins::{self, Overrides};
use serde_json::Value;

fn d1() -> Value {
    let path = format!(
        "{}/../aethelgard-ir/tests/fixtures/d1-drv8876.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn solved(v: &Value) -> SolvedTopology {
    let spec: aethelgard_ir::Specification = serde_json::from_value(v.clone()).unwrap();
    spec.validate().unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap()
}

fn plan(o: &Overrides) -> Result<pins::Pins, aethelgard_twin::TwinError> {
    pins::plan(&solved(&d1()), o)
}

/// D1: nSLEEP an output at its declared `low`, its BSRR bit BR1; nFAULT an
/// input driven high, where R3 to `VDD_3V3` holds it (solved `pulled`),
/// active low; IPROPI on ADC3 INP5 at 12 bits, a current output of U2,
/// whose bridge idles non-driving, so 0 V, code 0.
#[test]
fn d1_s_plan_is_its_solved_topology() {
    let p = plan(&Overrides::default()).unwrap();
    let [o] = p.outputs.as_slice() else { panic!() };
    assert_eq!(
        (o.net.as_str(), o.pin.as_str(), o.high),
        ("DRV_NSLEEP", "PA1", false)
    );
    assert_eq!((o.bsrr_bit(), o.bsrr_other()), (1 << 17, 1 << 1));
    assert_eq!(
        (o.bsrr_at, o.moder_at, o.odr_at),
        (0x5802_0018, 0x5802_0000, 0x5802_0014)
    );
    let [i] = p.inputs.as_slice() else { panic!() };
    assert_eq!(
        (
            i.net.as_str(),
            i.renode,
            i.bit,
            i.high,
            i.active_high,
            i.held
        ),
        ("DRV_NFAULT", "gpioPortA", 11, true, false, false)
    );
    assert_eq!(i.idr_at, 0x5802_0010);
    let [a] = p.analog.as_slice() else { panic!() };
    assert_eq!(
        (
            a.peripheral.as_str(),
            a.renode,
            a.channel,
            a.bits,
            a.microvolts,
            a.code
        ),
        ("ADC3", "adc3", 5, 12, 0, 0)
    );
    assert_eq!((a.dr_at, a.cr_at), (0x5802_6040, 0x5802_6008));
}

/// A scenario's hold and value replace the declared idle's, marked held.
#[test]
fn a_hold_and_a_value_replace_the_idle() {
    let o = Overrides {
        held: [("DRV_NFAULT".to_owned(), false)].into(),
        millivolts: [("DRV_IPROPI".to_owned(), 1500)].into(),
    };
    let p = plan(&o).unwrap();
    assert_eq!((p.inputs[0].high, p.inputs[0].held), (false, true));
    assert_eq!(
        (p.analog[0].microvolts, p.analog[0].held, p.analog[0].code),
        (1_500_000, true, 1861)
    );
}

/// Holding a declared output, or a pin a peripheral drives, is refused
/// `PinHeldIsOutput` (`1211` §4 G4); a net no signal declares,
/// `PinHeldUnknown`; a value on anything but a pin device's ADC input,
/// `AnalogValueUnknown`.
#[test]
fn a_hold_or_a_value_off_a_declared_input_is_refused_by_name() {
    let held = |net: &str| Overrides {
        held: [(net.to_owned(), true)].into(),
        ..Default::default()
    };
    for net in ["DRV_NSLEEP", "DRV_EN", "CMD_TX", "CMD_RX"] {
        let e = plan(&held(net)).unwrap_err();
        assert_eq!(e.kind(), "PinHeldIsOutput", "{net}: {e}");
    }
    assert_eq!(plan(&held("NOPE")).unwrap_err().kind(), "PinHeldUnknown");
    let set = |net: &str| Overrides {
        millivolts: [(net.to_owned(), 100)].into(),
        ..Default::default()
    };
    for net in ["DRV_NFAULT", "NOPE"] {
        assert_eq!(plan(&set(net)).unwrap_err().kind(), "AnalogValueUnknown");
    }
}

/// What the twin cannot run is refused `PinDeviceNotSimulated`, naming the
/// signal: an ADC input at a resolution other than the measured 12 bits, on
/// ADC2, or a current output of a device with no bridge; an input with no
/// stated level, or no active level.
#[test]
fn what_the_twin_cannot_run_is_refused_by_name() {
    let t = solved(&d1());
    let refused = |t: &SolvedTopology, signal: &str, want: &str| {
        let e = pins::plan(t, &Overrides::default()).unwrap_err();
        assert_eq!(e.kind(), "PinDeviceNotSimulated", "{e}");
        assert!(e.to_string().contains(want), "{e}");
        assert!(e.to_string().contains(&format!("`{signal}`")), "{e}");
    };
    let with = |f: &dyn Fn(&mut SolvedTopology)| {
        let mut t = t.clone();
        f(&mut t);
        t
    };
    refused(
        &with(&|t| t.adc_inputs.get_mut("DRV_IPROPI").unwrap().bits = Some(16)),
        "DRV_IPROPI",
        "16 bits",
    );
    refused(
        &with(&|t| t.adc_inputs.get_mut("DRV_IPROPI").unwrap().peripheral = "ADC2".into()),
        "DRV_IPROPI",
        "ADC2, which Renode does not model",
    );
    refused(
        &with(&|t| t.pin_devices.get_mut("U2").unwrap().bridges.clear()),
        "DRV_IPROPI",
        "drives no bridge",
    );
    refused(
        &with(&|t| t.gpio.get_mut("DRV_NFAULT").unwrap().pulled = None),
        "DRV_NFAULT",
        "states no level its pull holds it at",
    );
    refused(
        &with(&|t| t.gpio.get_mut("DRV_NFAULT").unwrap().active = None),
        "DRV_NFAULT",
        "declares no active level",
    );
}

/// The plan of a specification with no pin device is empty, so its boot's
/// commands, reads and verdict are what they were before `1211c`; an SPI
/// chip select is never a pin device's output.
#[test]
fn a_board_without_a_pin_device_has_an_empty_plan() {
    let path = format!(
        "{}/../../examples/sentinel-01-minimal.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let v: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let t = solved(&v);
    assert!(!t.gpio.is_empty(), "the board's flash chip select");
    assert_eq!(
        pins::plan(&t, &Overrides::default()).unwrap(),
        pins::Pins::default()
    );
}
