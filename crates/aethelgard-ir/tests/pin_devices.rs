//! `AET-WO-1211a`: a device with no bus (`pin_devices`), `pwm.idle` and
//! `pwm.resolution_ps`, judged by `validate` (step 24 and the motor timing).
//! Each refusal has its own kind and its own test, on D1 (DRV8876, PH/EN)
//! or D2 (DRV8833, two IN/IN bridges), the fixtures of `AET-WO-1211`'s spike
//! (`spike/1211a/fixtures/make.py` says what moved and why). The solve of
//! both on the real pin table is `aethelgard-cli`'s `pin_devices_solve.rs`.

use aethelgard_ir::parts::{PARTS_JSON, parse_parts};
use aethelgard_ir::{IrValidationError, Specification};
use serde_json::{Value, json};

fn d1() -> Value {
    serde_json::from_str(include_str!("fixtures/d1-drv8876.json")).unwrap()
}

fn d2() -> Value {
    serde_json::from_str(include_str!("fixtures/d2-drv8833.json")).unwrap()
}

fn spec(v: &Value) -> Specification {
    serde_json::from_value(v.clone()).expect("a specification")
}

fn refused(v: &Value) -> IrValidationError {
    spec(v).validate().expect_err("refused")
}

/// The kind, and the refusal's message, for the assertion's text.
fn kind(v: &Value) -> String {
    let e = refused(v);
    format!("{}: {e}", e.kind())
}

fn assert_kind(v: &Value, want: &str) {
    let got = kind(v);
    assert!(got.starts_with(&format!("{want}: ")), "{got}");
}

/// D2 with the limit SLVSAR1E does not state declared, so that
/// `PartLimitUndeclared` is shown to be its only refusal.
fn d2_declared() -> Value {
    let mut v = d2();
    v["pin_devices"][0]["part_limits"]["input_pwm_max_hz"] =
        json!({"value": 50000, "source": "a declaration for this test, no held document"});
    v
}

fn device(v: &mut Value) -> &mut Value {
    &mut v["pin_devices"][0]
}

fn motor<'v>(v: &'v mut Value, label: &str) -> &'v mut Value {
    v["motor_outputs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|m| m["label"] == label)
        .unwrap()
}

// ---------------------------------------------------------------- the two fixtures

#[test]
fn d1_validates() {
    spec(&d1()).validate().unwrap();
}

#[test]
fn d2_is_refused_part_limit_undeclared_and_only_that() {
    let e = refused(&d2());
    assert_eq!(e.kind(), "PartLimitUndeclared", "{e}");
    assert_eq!(
        e.path(),
        Some("pin_devices[0].part_limits.input_pwm_max_hz")
    );
    assert!(e.to_string().contains("SLVSAR1E"), "{e}");
    spec(&d2_declared()).validate().unwrap();
}

/// D1 as `1211`'s spike wrote it (2 A, 16 bits, no reservoir) is refused at
/// the full scale's error-band edge, and at 1.9 A by the narrower RAIN table
/// (`make.py`'s two findings).
#[test]
fn d1_as_the_spike_wrote_it_is_refused_twice() {
    let spike: Value =
        serde_json::from_str(include_str!("../../../spike/1211/fixtures/d1-drv8876.json")).unwrap();
    let mut v = d1();
    *device(&mut v)["analog"].get_mut(0).unwrap() = spike["pin_devices"][0]["analog"][0].clone();
    device(&mut v)["analog"][0]["channel"] = Value::Null;
    device(&mut v)["analog"][0]["transfer"]["current_output"]["reservoir"] = Value::Null;
    // 2 A: 2.1 mA at AERR's 5 %, through R2 at its highest, above 3135 mV;
    // at the typical gain alone (2 mA x 1500 ohms = 3.0 V) it would pass.
    let e = refused(&v);
    assert_eq!(e.kind(), "AdcFullScaleExceeded", "{e}");
    assert!(e.to_string().contains("3135 mV"), "{e}");
    // At 1.9 A the full scale fits; at 16 bits R2 is above RAIN's 170 ohms
    // (DS12117 Rev 10 Table 173 p. 273) and no reservoir is declared.
    device(&mut v)["analog"][0]["full_scale"] = json!({"milliamps": 1900});
    assert_kind(&v, "AdcSourceImpedance");
}

// ---------------------------------------------------------------- pwm.idle and resolution_ps

#[test]
fn idle_pulse_zero_is_idle_spelled_twice() {
    let mut v = d1();
    motor(&mut v, "SERVO1")["pwm"]["idle"] = json!({"pulse_us": 0});
    assert_kind(&v, "IdleSpelledTwice");
}

#[test]
fn an_idle_holds_exactly_one_key() {
    for idle in [
        json!({}),
        json!({"level": "low", "pulse_us": 3}),
        json!({"duty": 1}),
    ] {
        let mut v = d1();
        motor(&mut v, "SERVO1")["pwm"]["idle"] = idle.clone();
        assert!(
            serde_json::from_value::<Specification>(v).is_err(),
            "{idle}"
        );
    }
}

#[test]
fn a_resolution_no_prescaler_gives_is_refused() {
    let mut v = d1();
    // 1 ns is 0.064 of a 64 MHz cycle.
    motor(&mut v, "SERVO1")["pwm"]["resolution_ps"] = json!(1000);
    assert_kind(&v, "ResolutionNotDerivable");
    // 65,537 cycles is past a 16-bit prescaler.
    motor(&mut v, "SERVO1")["pwm"]["resolution_ps"] = json!(1_024_015_625u64);
    assert_kind(&v, "ResolutionNotDerivable");
}

#[test]
fn a_pulse_off_the_tick_is_refused() {
    let mut v = d1();
    // 3 us ticks: 1000 us is 333.3 of them.
    motor(&mut v, "SERVO1")["pwm"]["resolution_ps"] = json!(3_000_000);
    assert_kind(&v, "PulseNotOnTick");
}

#[test]
fn an_idle_off_the_tick_is_refused() {
    let mut v = d1();
    motor(&mut v, "SERVO1")["pwm"]["resolution_ps"] = json!(2_000_000);
    motor(&mut v, "SERVO1")["pwm"]["idle"] = json!({"pulse_us": 1501});
    assert_kind(&v, "IdleNotOnTick");
}

#[test]
fn an_idle_longer_than_the_period_is_refused() {
    let mut v = d1();
    motor(&mut v, "SERVO1")["pwm"]["idle"] = json!({"pulse_us": 20_001});
    assert_kind(&v, "IdleExceedsPeriod");
}

#[test]
fn idle_high_and_low_validate_and_the_commanded_clause_stands() {
    let mut v = d1();
    motor(&mut v, "SERVO1")["pwm"]["idle"] = json!({"level": "high"});
    spec(&v).validate().unwrap();
    // A commanded minimum one tick short of the period is still refused,
    // though the idle word ARR + 1 is accepted.
    motor(&mut v, "SERVO1")["pwm"]["pulse_min_us"] = json!(19_999);
    motor(&mut v, "SERVO1")["pwm"]["pulse_max_us"] = json!(19_999);
    let e = refused(&v);
    assert!(matches!(e.kind(), "PulseRange" | "MotorTiming"), "{e}");
}

// ---------------------------------------------------------------- the device

#[test]
fn a_bus_part_in_pin_devices_is_part_has_bus() {
    let mut v = d1();
    device(&mut v)["part"] = json!("BME280");
    assert_kind(&v, "PartHasBus");
}

#[test]
fn an_unknown_part_is_unknown_part() {
    let mut v = d1();
    device(&mut v)["part"] = json!("DRV9999");
    assert_kind(&v, "UnknownPart");
}

#[test]
fn a_busless_part_on_a_bus_is_part_without_bus() {
    let mut v: Value =
        serde_json::from_str(include_str!("fixtures/sentinel-02-power.json")).unwrap();
    v["i2c_devices"][0]["part"] = json!("DRV8833PWPR");
    let e = refused(&v);
    assert_eq!(e.kind(), "PartWithoutBus", "{e}");
    assert_eq!(e.path(), Some("i2c_devices[0].part"));
}

#[test]
fn a_supply_pin_without_a_domain_is_refused() {
    let mut v = d1();
    device(&mut v)["supplies"] = json!({});
    assert_kind(&v, "SupplyPinUndeclared");
}

#[test]
fn a_supply_on_an_undeclared_domain_is_refused() {
    let mut v = d1();
    device(&mut v)["supplies"]["VM"] = json!("V_NONE");
    assert_kind(&v, "UnknownDomain");
}

#[test]
fn a_supply_outside_the_part_is_refused() {
    let mut v = d1();
    // 3.3 V is below VM's 4.5 V minimum (SLVSDS7B p. 4).
    device(&mut v)["supplies"]["VM"] = json!("VDD_3V3");
    assert_kind(&v, "SupplyOutOfRange");
}

#[test]
fn a_strap_left_out_is_refused() {
    let mut v = d1();
    device(&mut v)["straps"]
        .as_object_mut()
        .unwrap()
        .remove("PMODE");
    assert_kind(&v, "StrapUndeclared");
}

#[test]
fn a_strap_level_the_pin_lacks_is_refused() {
    // PMODE has no resistor level.
    let mut v = d1();
    device(&mut v)["straps"]["PMODE"] =
        json!({"resistor": {"reference": "R9", "part": "CRCW0603_F_K", "ohms": 20000}});
    assert_kind(&v, "StrapLevelInvalid");
    // IMODE's resistor levels are 18.6-21.4 and 57.6-66.4 kOhm.
    let mut v = d1();
    device(&mut v)["straps"]["IMODE"] =
        json!({"resistor": {"reference": "R9", "part": "CRCW0603_F_K", "ohms": 40000}});
    assert_kind(&v, "StrapLevelInvalid");
    // VREF tied to VMOTOR is above its 3.6 V.
    let mut v = d1();
    device(&mut v)["straps"]["VREF"] = json!({"tie": "VMOTOR"});
    assert_kind(&v, "StrapLevelInvalid");
}

#[test]
fn a_resistor_strap_inside_a_level_validates() {
    let mut v = d1();
    device(&mut v)["straps"]["IMODE"] =
        json!({"resistor": {"reference": "R9", "part": "CRCW0603_F_K", "ohms": 20000}});
    spec(&v).validate().unwrap();
}

#[test]
fn a_device_pin_the_row_lacks_is_refused() {
    let mut v = d1();
    device(&mut v)["signals"][0]["device_pin"] = json!("nRESET");
    assert_kind(&v, "UnknownDevicePin");
}

#[test]
fn a_device_pin_bound_twice_is_refused() {
    let mut v = d1();
    let mut second = device(&mut v)["signals"][0].clone();
    second["net"] = json!("DRV_NSLEEP2");
    device(&mut v)["signals"]
        .as_array_mut()
        .unwrap()
        .push(second);
    assert_kind(&v, "DevicePinBoundTwice");
}

#[test]
fn an_input_of_the_part_left_unbound_is_refused() {
    // D2 without its second bridge leaves BIN1 and BIN2 to their pulls.
    let mut v = d2_declared();
    device(&mut v)["bridges"].as_array_mut().unwrap().pop();
    // Its outputs' nets go with it, off the connector that carried them.
    for c in v["connectors"].as_array_mut().unwrap() {
        for p in c["pins"].as_array_mut().unwrap() {
            if p["net"].as_str().is_some_and(|n| n.starts_with("MB_")) {
                p["net"] = Value::Null;
            }
        }
    }
    let labels = ["B_IN1", "B_IN2"];
    v["motor_outputs"]
        .as_array_mut()
        .unwrap()
        .retain(|m| !labels.contains(&m["label"].as_str().unwrap()));
    assert_kind(&v, "DevicePinUnbound");
}

#[test]
fn a_signal_against_its_pin_s_kind_is_refused() {
    let mut v = d1();
    // An output on nFAULT, an open-drain output of the part.
    device(&mut v)["signals"][1] = json!({"device_pin": "nFAULT", "net": "DRV_NFAULT", "pin": null, "direction": "output", "initial": "high"});
    assert_kind(&v, "SignalKindMismatch");
}

#[test]
fn a_signal_of_the_wrong_shape_is_refused() {
    let mut v = d1();
    device(&mut v)["signals"][0]["pull"] = json!({"mcu": "down", "source": "x"});
    assert_kind(&v, "SignalShape");
    let mut v = d1();
    device(&mut v)["signals"][1]
        .as_object_mut()
        .unwrap()
        .remove("active");
    assert_kind(&v, "SignalShape");
}

#[test]
fn an_output_the_pin_cannot_read_is_refused() {
    // D2's nSLEEP reads high from 2.5 V; on a 3.0 V +-5 % supply the MCU's
    // VOH is 2850 - 400 = 2450 mV at least.
    let mut v = d2_declared();
    v["voltage_domains"][0]["nominal_millivolts"] = json!(3000);
    assert_eq!(v["voltage_domains"][0]["id"], "VDD_3V3");
    assert_kind(&v, "LogicLevelIncompatible");
}

#[test]
fn an_open_drain_input_with_no_pull_up_is_refused() {
    let mut v = d2_declared();
    device(&mut v)["signals"][1]["pull"]["mcu"] = json!("none");
    assert_kind(&v, "OpenDrainUnpulled");
    device(&mut v)["signals"][1]["pull"]["mcu"] = json!("down");
    assert_kind(&v, "OpenDrainUnpulled");
}

#[test]
fn a_pull_up_the_part_cannot_sink_is_refused() {
    let mut v = d1();
    device(&mut v)["signals"][1]["pull"]["resistor"]["ohms"] = json!(330);
    assert_kind(&v, "PullOverDrive");
}

#[test]
fn a_limit_the_row_states_declared_again_is_refused() {
    let mut v = d1();
    device(&mut v)["part_limits"]["input_pwm_max_hz"] = json!({"value": 1, "source": "x"});
    assert_kind(&v, "PartLimitDeclaredTwice");
}

#[test]
fn a_resistor_judged_over_no_temperature_is_refused() {
    let mut v = d1();
    device(&mut v)["temperature_celsius"] = Value::Null;
    assert_kind(&v, "TemperatureUndeclared");
}

#[test]
fn a_temperature_declared_beside_power_s_is_refused() {
    // D1 on Sentinel-02's power tree, which declares the board temperature.
    let power: Value =
        serde_json::from_str(include_str!("fixtures/sentinel-02-power.json")).unwrap();
    let mut v = d1();
    v["power"] = power["power"].clone();
    // Judged before the tree's own checks need the rest of Sentinel-02.
    let e = aethelgard_ir::pin_devices::validate_against(
        &spec(&v),
        aethelgard_ir::parts::table().unwrap(),
    )
    .unwrap_err();
    assert_eq!(e.kind(), "TemperatureDeclaredTwice", "{e}");
}

#[test]
fn a_resistor_the_power_table_lacks_is_refused() {
    let mut v = d1();
    device(&mut v)["analog"][0]["transfer"]["current_output"]["resistor"]["part"] = json!("R_NONE");
    assert_kind(&v, "PowerPartUnknown");
}

#[test]
fn a_full_scale_no_error_band_covers_is_refused() {
    let mut v = d1();
    // AERR is stated to 2 A (SLVSDS7B p. 6).
    device(&mut v)["analog"][0]["full_scale"] = json!({"milliamps": 2100});
    assert_kind(&v, "PartLimitUndeclared");
}

#[test]
fn a_current_output_above_its_maximum_is_refused() {
    // The DRV8876's 3 mA is above any banded full scale (2.1 mA at 2 A), so
    // the row's maximum is lowered in a copy of the table.
    let mut doc: Value = serde_json::from_str(PARTS_JSON).unwrap();
    let rows = doc["pin_parts"].as_array_mut().unwrap();
    let row = rows
        .iter_mut()
        .find(|r| r["part"] == "DRV8876PWPR")
        .unwrap();
    row["pins"]["IPROPI"]["output_max_ua"] = json!(1500);
    let table = parse_parts(&doc.to_string()).unwrap();
    let e = aethelgard_ir::pin_devices::validate_against(&spec(&d1()), &table).unwrap_err();
    assert_eq!(e.kind(), "AdcOutputCurrentExceeded", "{e}");
}

#[test]
fn a_source_above_rain_with_no_reservoir_is_refused() {
    let mut v = d1();
    device(&mut v)["analog"][0]["transfer"]["current_output"]["reservoir"] = Value::Null;
    assert_kind(&v, "AdcSourceImpedance");
    // At 10 bits RAIN is 5650 ohms (Table 173): R2 needs no reservoir.
    device(&mut v)["analog"][0]["adc_resolution_bits"] = json!(10);
    spec(&v).validate().unwrap();
}

// ---------------------------------------------------------------- the bridge

#[test]
fn a_mode_the_part_lacks_is_refused() {
    let mut v = d1();
    device(&mut v)["bridges"][0]["mode"] = json!("half");
    assert_kind(&v, "BridgeModeUnknown");
}

#[test]
fn a_mode_its_strap_does_not_select_is_refused() {
    // PMODE tied to ground selects PH/EN, not IN/IN.
    let mut v = d1();
    device(&mut v)["bridges"][0]["mode"] = json!("in_in");
    assert_kind(&v, "BridgeModeStrapMismatch");
    // Tied high, it selects IN/IN, and the PH/EN bridge is refused.
    let mut v = d1();
    device(&mut v)["straps"]["PMODE"] = json!({"tie": "VDD_3V3"});
    assert_kind(&v, "BridgeModeStrapMismatch");
}

#[test]
fn pins_that_are_no_channel_are_refused() {
    let mut v = d1();
    device(&mut v)["bridges"][0]["outputs"] = json!({"OUT1": "M1_OUT1", "VM": "M1_OUT2"});
    assert_kind(&v, "BridgeChannelUnknown");
}

#[test]
fn an_input_that_is_no_output_of_the_mcu_is_refused() {
    let mut v = d1();
    device(&mut v)["bridges"][0]["inputs"]["PH/IN2"] = json!("DRV_NFAULT");
    assert_kind(&v, "BridgeInputUnknown");
}

#[test]
fn one_motor_output_on_two_inputs_is_refused() {
    let mut v = d2_declared();
    device(&mut v)["bridges"][1]["inputs"]["BIN1"] = json!("A_IN1");
    v["motor_outputs"]
        .as_array_mut()
        .unwrap()
        .retain(|m| m["label"] != "B_IN1");
    assert_kind(&v, "BridgeInputShared");
}

#[test]
fn a_pulsed_bridge_input_is_refused() {
    let mut v = d1();
    motor(&mut v, "DRV_EN")["pwm"]["idle"] = json!({"pulse_us": 1});
    let e = refused(&v);
    assert_eq!(e.kind(), "BridgeIdlePulsed", "{e}");
    assert_eq!(e.path(), Some("pin_devices[0].bridges[0].inputs.EN/IN1"));
}

#[test]
fn a_driving_idle_is_refused() {
    // nSLEEP high wakes the part; EN high and PH low is reverse (Table 3).
    let mut v = d1();
    device(&mut v)["signals"][0]["initial"] = json!("high");
    motor(&mut v, "DRV_EN")["pwm"]["idle"] = json!({"level": "high"});
    let e = refused(&v);
    assert_eq!(e.kind(), "BridgeIdleDrives", "{e}");
    assert!(e.to_string().contains("reverse"), "{e}");
    // EN low is brake whatever PH: awake and idle, not driving.
    motor(&mut v, "DRV_EN")["pwm"]["idle"] = json!({"level": "low"});
    motor(&mut v, "DRV_PH")["pwm"]["idle"] = json!({"level": "high"});
    spec(&v).validate().unwrap();
}

/// The enable is read: the same driving inputs with nSLEEP low are sleep.
#[test]
fn a_sleeping_part_idles_whatever_its_inputs() {
    let mut v = d1();
    motor(&mut v, "DRV_EN")["pwm"]["idle"] = json!({"level": "high"});
    spec(&v).validate().unwrap();
}

/// IN/IN both high is brake, which slow-decay PWM uses: not refused
/// (`1211` Q3 withdrew "never driven high together").
#[test]
fn both_inputs_high_on_an_in_in_bridge_is_brake_and_validates() {
    let mut v = d2_declared();
    device(&mut v)["signals"][0]["initial"] = json!("high");
    for l in ["A_IN1", "A_IN2"] {
        motor(&mut v, l)["pwm"]["idle"] = json!({"level": "high"});
    }
    spec(&v).validate().unwrap();
    motor(&mut v, "A_IN2")["pwm"]["idle"] = json!({"level": "low"});
    assert_kind(&v, "BridgeIdleDrives");
}

#[test]
fn a_bridge_input_may_be_a_gpio_signal() {
    // `1211` Q7: PH as a GPIO output at `low`.
    let mut v = d1();
    v["motor_outputs"]
        .as_array_mut()
        .unwrap()
        .retain(|m| m["label"] != "DRV_PH");
    device(&mut v)["signals"].as_array_mut().unwrap().push(
        json!({"device_pin": "PH/IN2", "net": "DRV_PH", "pin": null, "direction": "output", "initial": "low"}),
    );
    spec(&v).validate().unwrap();
}

/// A row with a forbidden state (a gate-driver leg's shoot-through) in a
/// copy of the table: an idle that reaches it is refused by its own kind.
#[test]
fn an_idle_in_a_forbidden_state_is_refused() {
    let mut doc: Value = serde_json::from_str(PARTS_JSON).unwrap();
    let rows = doc["pin_parts"].as_array_mut().unwrap();
    let row = rows
        .iter_mut()
        .find(|r| r["part"] == "DRV8876PWPR")
        .unwrap();
    row["modes"][0]["forbidden"] = json!(["sleep"]);
    let table = parse_parts(&doc.to_string()).unwrap();
    let e = aethelgard_ir::pin_devices::validate_against(&spec(&d1()), &table).unwrap_err();
    assert_eq!(e.kind(), "BridgeForbiddenState", "{e}");
}

/// A driven input with no internal pull, in a copy of the table: from reset
/// to the first write nothing holds it (`1211` §2.6).
#[test]
fn an_input_no_pull_holds_from_reset_is_refused() {
    let mut doc: Value = serde_json::from_str(PARTS_JSON).unwrap();
    let rows = doc["pin_parts"].as_array_mut().unwrap();
    let row = rows
        .iter_mut()
        .find(|r| r["part"] == "DRV8876PWPR")
        .unwrap();
    row["pins"]["EN/IN1"]
        .as_object_mut()
        .unwrap()
        .remove("pull_down_ohms_typical");
    let table = parse_parts(&doc.to_string()).unwrap();
    let e = aethelgard_ir::pin_devices::validate_against(&spec(&d1()), &table).unwrap_err();
    assert_eq!(e.kind(), "UnheldDuringReset", "{e}");
}

#[test]
fn an_input_rate_above_the_part_is_refused() {
    let mut v = d1();
    // 200 kHz with a 4 us longest pulse fits the period; the DRV8876 takes
    // 100 kHz (SLVSDS7B p. 4).
    for l in ["DRV_EN", "DRV_PH"] {
        motor(&mut v, l)["pwm"]["rate_hz"] = json!(200_000);
        motor(&mut v, l)["pwm"]["pulse_max_us"] = json!(4);
    }
    assert_kind(&v, "InputRateAbovePart");
}

// ---------------------------------------------------------------- names and the topology

#[test]
fn a_device_s_nets_and_references_join_the_board_s() {
    let mut v = d1();
    device(&mut v)["reference"] = json!("J1");
    assert_kind(&v, "DuplicateReference");
    let mut v = d1();
    device(&mut v)["bridges"][0]["outputs"]["OUT1"] = json!("DRV_NSLEEP");
    assert_kind(&v, "DuplicateNetName");
}

#[test]
fn a_topology_with_a_device_s_gpio_and_no_device_is_refused() {
    let t: aethelgard_ir::SolvedTopology = serde_json::from_str(include_str!(
        "../../aethelgard-svd-hal/tests/ui/motors-pwm/topology.json"
    ))
    .unwrap();
    t.validate().unwrap();
    let mut bad = t.clone();
    bad.gpio.insert(
        "DRV_NFAULT".into(),
        aethelgard_ir::pin_devices::SolvedGpio {
            pin: "PD0".into(),
            direction: aethelgard_ir::pin_devices::Direction::Input,
            initial: None,
            pull: Some(aethelgard_ir::pin_devices::McuPull::Up),
        },
    );
    bad.pin_speeds.insert("DRV_NFAULT".into(), 0);
    assert_eq!(bad.validate().unwrap_err().kind(), "PinDeviceUndeclared");
    bad.pin_devices.insert(
        "U2".into(),
        aethelgard_ir::pin_devices::SolvedPinDevice {
            part: "DRV8876PWPR".into(),
        },
    );
    bad.validate().unwrap();
}

/// M5 (`AET-WO-1211a` §1.1): the 4600 driver (`driver-judges-nothing.json`,
/// as `spike/1211a/migrate.py` moved it: each idle the old `pulse_min_us`,
/// 1 us) with its `HB_IN1`/`HB_IN2` pair declared as an IN/IN bridge is
/// refused `BridgeIdlePulsed`: 4600 F2's hazard, an IN/IN pair chopping from
/// boot, now caught. Idled `low`, the same declaration validates.
#[test]
fn the_4600_driver_s_pair_as_a_bridge_is_refused_pulsed() {
    let mut v: Value = serde_json::from_str(include_str!(
        "../../aethelgard-cli/tests/fixtures/driver-judges-nothing.json"
    ))
    .unwrap();
    assert_eq!(
        motor(&mut v, "HB_IN1")["pwm"]["idle"],
        json!({"pulse_us": 1})
    );
    v["voltage_domains"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": "VMOTOR", "nominal_millivolts": 12000, "tolerance_permille": 50}));
    v["pin_devices"] = json!([{
        "reference": "U9",
        "part": "DRV8876PWPR",
        "supplies": {"VM": "VMOTOR"},
        "straps": {"PMODE": {"tie": "VDD_3V3"}, "IMODE": {"tie": "GND"}, "VREF": {"tie": "VDD_3V3"}},
        "signals": [{"device_pin": "nSLEEP", "net": "HB_NSLEEP", "pin": null,
                     "direction": "output", "initial": "low"}],
        "analog": [],
        "bridges": [{"id": "HB", "mode": "in_in",
                     "inputs": {"EN/IN1": "HB_IN1", "PH/IN2": "HB_IN2"},
                     "outputs": {"OUT1": "HB_OUT1", "OUT2": "HB_OUT2"}}],
        "part_limits": {"input_pwm_max_hz": null},
        "temperature_celsius": null
    }]);
    let e = refused(&v);
    assert_eq!(e.kind(), "BridgeIdlePulsed", "{e}");
    assert!(e.to_string().contains("HB_IN1"), "{e}");
    for l in ["HB_IN1", "HB_IN2"] {
        motor(&mut v, l)["pwm"]["idle"] = json!({"level": "low"});
    }
    spec(&v).validate().unwrap();
}

#[test]
fn a_device_s_identifiers_are_step_2_s() {
    for (path, value) in [
        ("/pin_devices/0/reference", json!("U 2")),
        ("/pin_devices/0/signals/0/net", json!("DRV NSLEEP")),
        (
            "/pin_devices/0/analog/0/transfer/current_output/resistor/reference",
            json!("R/2"),
        ),
        ("/pin_devices/0/bridges/0/outputs/OUT1", json!("M1 OUT1")),
    ] {
        let mut v = d1();
        *v.pointer_mut(path).unwrap() = value;
        let e = refused(&v);
        assert_eq!(e.kind(), "InvalidIdentifier", "{path}: {e}");
    }
    let mut v = d1();
    device(&mut v)["temperature_celsius"]["source"] = json!(" padded ");
    assert_kind(&v, "InvalidSource");
}
