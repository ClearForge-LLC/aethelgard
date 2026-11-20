//! `AET-WO-1211a` §2: D1 validates and solves on the real STM32H753ZITx pin
//! table, and every word the solver writes for it is judged by
//! `check_silicon`; D2 solves once its input PWM limit is declared, and an
//! IN/IN pair split across two timers is refused by name.

use aethelgard_ir::motor::{Level, check_silicon};
use aethelgard_ir::pin_devices::{Direction, McuPull, SolvedGpio};
use aethelgard_ir::{SolveError, SolvedTopology, Specification, solve};
use serde_json::{Value, json};

const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn fixture(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../aethelgard-ir/tests/fixtures")
        .join(name);
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn solved(v: &Value) -> Result<SolvedTopology, SolveError> {
    let spec: Specification = serde_json::from_value(v.clone()).unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    solve(&spec, &table, HASH)
}

fn d2_declared() -> Value {
    let mut v = fixture("d2-drv8833.json");
    v["pin_devices"][0]["part_limits"]["input_pwm_max_hz"] =
        json!({"value": 50000, "source": "a declaration for this test, no held document"});
    v
}

/// Every motor output's words hold on the topology's clock.
fn silicon_holds(t: &SolvedTopology) {
    for (label, m) in &t.motor_outputs {
        let timer: u32 = t.assignments[label].peripheral["TIM".len()..]
            .parse()
            .unwrap();
        let s = check_silicon(&t.clock, timer, m);
        assert!(s.holds, "{label}: {s:?}");
    }
}

#[test]
fn d1_solves_and_its_words_hold() {
    let t = solved(&fixture("d1-drv8876.json")).unwrap();
    t.validate().unwrap();
    assert_eq!(t.pin_devices["U2"].part, "DRV8876PWPR");
    assert_eq!(t.gpio["DRV_NSLEEP"].direction, Direction::Output);
    assert_eq!(t.gpio["DRV_NSLEEP"].initial, Some(Level::Low));
    // nFAULT's pull is the 10 kOhm resistor: the MCU's own is none.
    assert_eq!(
        (t.gpio["DRV_NFAULT"].direction, t.gpio["DRV_NFAULT"].pull),
        (Direction::Input, Some(McuPull::None))
    );
    let ipropi = &t.adc_inputs["DRV_IPROPI"];
    assert!(
        ["ADC1", "ADC3"].contains(&ipropi.peripheral.as_str()),
        "{ipropi:?}"
    );
    // 20 kHz at one 64 MHz tick (15,625 ps): PSC 0, ARR 3199, idle low CCR 0.
    for l in ["DRV_EN", "DRV_PH"] {
        let m = &t.motor_outputs[l];
        assert_eq!((m.psc, m.arr, m.ccr), (0, 3199, 0), "{l}");
    }
    // The servo at 1 us ticks, idling at its 1500 us centre.
    let m = &t.motor_outputs["SERVO1"];
    assert_eq!((m.psc, m.arr, m.ccr), (63, 19999, 1500));
    silicon_holds(&t);
}

#[test]
fn d1_with_an_idle_high_input_solves_to_arr_plus_one() {
    let mut v = fixture("d1-drv8876.json");
    let m = v["motor_outputs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|m| m["label"] == "DRV_PH")
        .unwrap();
    m["pwm"]["idle"] = json!({"level": "high"});
    let t = solved(&v).unwrap();
    let m = &t.motor_outputs["DRV_PH"];
    assert_eq!((m.arr, m.ccr), (3199, 3200));
    silicon_holds(&t);
    // A word off the declared idle fails the silicon check.
    let mut off = t.motor_outputs["DRV_PH"].clone();
    off.ccr = 3199;
    let timer: u32 = t.assignments["DRV_PH"].peripheral[3..].parse().unwrap();
    assert!(!check_silicon(&t.clock, timer, &off).holds);
}

#[test]
fn d2_declared_solves_on_its_hand_timers() {
    let t = solved(&d2_declared()).unwrap();
    t.validate().unwrap();
    for (l, tim) in [
        ("A_IN1", "TIM1"),
        ("A_IN2", "TIM1"),
        ("B_IN1", "TIM3"),
        ("B_IN2", "TIM3"),
    ] {
        assert_eq!(t.assignments[l].peripheral, tim, "{l}");
        assert_eq!(t.motor_outputs[l].ccr, 0, "{l}");
    }
    assert!(t.adc_inputs.is_empty());
    silicon_holds(&t);
}

#[test]
fn an_in_in_pair_on_two_timers_is_refused() {
    let mut v = d2_declared();
    for m in v["motor_outputs"].as_array_mut().unwrap() {
        match m["label"].as_str().unwrap() {
            "A_IN2" => m["pin"] = json!("PC7"),
            "B_IN2" => m["pin"] = json!("PA9"),
            _ => {}
        }
    }
    let e = solved(&v).unwrap_err();
    assert_eq!(e.kind(), "BridgeSplitAcrossTimers", "{e}");
    assert_eq!(e.id(), Some("pin_devices[0].bridges[0]"));
}

#[test]
fn a_pin_device_s_gpio_is_never_a_chip_select_s_shape() {
    let t = solved(&fixture("d1-drv8876.json")).unwrap();
    let not_cs: Vec<&String> = t
        .gpio
        .iter()
        .filter(|(_, g)| **g != SolvedGpio::output(g.pin.clone(), Level::High))
        .map(|(n, _)| n)
        .collect();
    assert_eq!(not_cs, ["DRV_NFAULT", "DRV_NSLEEP"]);
}
