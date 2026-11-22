//! `AET-WO-1211c`: the twin's plan of a pin device (`pins::plan`), from the
//! solved topology and the facts the CLI reads from the specification
//! ([`pins::Declared`], `RUL-16`): D1's, and each refusal by its name. The
//! CLI's reading of the specification is tested beside it
//! (`aethelgard-cli` `pinwitness`).

use aethelgard_ir::{SolvedTopology, Specification};
use aethelgard_twin::pins::{
    self, Declared, DeclaredAnalog, DeclaredDevice, DeclaredSignal, Overrides,
};
use serde_json::Value;

fn d1() -> Value {
    let path = format!(
        "{}/../aethelgard-ir/tests/fixtures/d1-drv8876.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn solved(v: &Value) -> (Specification, SolvedTopology) {
    let spec: Specification = serde_json::from_value(v.clone()).unwrap();
    spec.validate().unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let t = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    (spec, t)
}

/// D1's facts as the specification declares them: nSLEEP an output; nFAULT
/// an input, active low, R3 to `VDD_3V3` holding it high; IPROPI a current
/// output of a device with a bridge, so 0 V at idle.
fn declared() -> Declared {
    let signal = |net: &str, output, active_high, pulled_high| DeclaredSignal {
        net: net.into(),
        output,
        active_high,
        pulled_high,
    };
    Declared {
        devices: vec![DeclaredDevice {
            reference: "U2".into(),
            part: "DRV8876PWPR".into(),
            signals: vec![
                signal("DRV_NSLEEP", true, None, None),
                signal("DRV_NFAULT", false, Some(false), Some(true)),
            ],
            analog: vec![DeclaredAnalog {
                net: "DRV_IPROPI".into(),
                bits: 12,
                idle_microvolts: Ok(0),
            }],
        }],
    }
}

fn plan(v: &Value, o: &Overrides) -> Result<pins::Pins, aethelgard_twin::TwinError> {
    let (_, t) = solved(v);
    pins::plan(&declared(), &t, o)
}

/// D1: nSLEEP an output at its declared `low`, its BSRR bit BR1; nFAULT an
/// input driven high, where R3 to `VDD_3V3` holds it, active low; IPROPI on
/// ADC3 INP5 at 12 bits, set to 0 V (the bridge idles non-driving), code 0.
#[test]
fn d1_s_plan_is_its_declaration() {
    let p = plan(&d1(), &Overrides::default()).unwrap();
    assert_eq!(p.devices, [("U2".to_owned(), "DRV8876PWPR".to_owned())]);
    let [o] = p.outputs.as_slice() else { panic!() };
    assert_eq!(
        (o.net.as_str(), o.pin.as_str(), o.high),
        ("DRV_NSLEEP", "PA1", false)
    );
    assert_eq!((o.bsrr_bit(), o.bsrr_other()), (1 << 17, 1 << 1));
    assert_eq!((o.bsrr_at, o.moder_at), (0x5802_0018, 0x5802_0000));
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
    assert_eq!(a.dr_at, 0x5802_6040);
}

/// A scenario's hold and value replace the declared idle's, marked held.
#[test]
fn a_hold_and_a_value_replace_the_idle() {
    let o = Overrides {
        held: [("DRV_NFAULT".to_owned(), false)].into(),
        millivolts: [("DRV_IPROPI".to_owned(), 1500)].into(),
    };
    let p = plan(&d1(), &o).unwrap();
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
        let e = plan(&d1(), &held(net)).unwrap_err();
        assert_eq!(e.kind(), "PinHeldIsOutput", "{net}: {e}");
    }
    assert_eq!(
        plan(&d1(), &held("NOPE")).unwrap_err().kind(),
        "PinHeldUnknown"
    );
    let set = |net: &str| Overrides {
        millivolts: [(net.to_owned(), 100)].into(),
        ..Default::default()
    };
    for net in ["DRV_NFAULT", "NOPE"] {
        assert_eq!(
            plan(&d1(), &set(net)).unwrap_err().kind(),
            "AnalogValueUnknown"
        );
    }
}

/// What the twin cannot run is refused `PinDeviceNotSimulated`, naming it:
/// an ADC input at a resolution other than the measured 12 bits, or on ADC2,
/// one with no stated idle value, an input with no stated level, and one
/// with no active level.
#[test]
fn what_the_twin_cannot_run_is_refused_by_name() {
    let (_, t) = solved(&d1());
    let refused = |d: &Declared, t: &SolvedTopology, want: &str| {
        let e = pins::plan(d, t, &Overrides::default()).unwrap_err();
        assert_eq!(e.kind(), "PinDeviceNotSimulated", "{e}");
        assert!(e.to_string().contains(want), "{e}");
        assert!(e.to_string().contains("`U2` (DRV8876PWPR)"), "{e}");
    };
    let mut bits = t.clone();
    bits.adc_inputs.get_mut("DRV_IPROPI").unwrap().bits = Some(16);
    refused(&declared(), &bits, "16 bits");
    let mut adc2 = t.clone();
    adc2.adc_inputs.get_mut("DRV_IPROPI").unwrap().peripheral = "ADC2".into();
    refused(&declared(), &adc2, "ADC2, which Renode does not model");
    let mut d = declared();
    d.devices[0].analog[0].idle_microvolts = Err("why".into());
    refused(&d, &t, "states no idle value: why");
    let mut d = declared();
    d.devices[0].signals[1].pulled_high = None;
    refused(&d, &t, "states no level its pull holds it at");
    let mut d = declared();
    d.devices[0].signals[1].active_high = None;
    refused(&d, &t, "declares no active level");
}

/// The plan of a specification with no pin device is empty, so its boot's
/// commands, reads and verdict are what they were before `1211c`.
#[test]
fn a_board_without_a_pin_device_has_an_empty_plan() {
    let path = format!(
        "{}/../../examples/sentinel-01-minimal.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let v: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let (_, t) = solved(&v);
    assert!(
        pins::plan(&Declared::default(), &t, &Overrides::default())
            .unwrap()
            .is_empty()
    );
}
