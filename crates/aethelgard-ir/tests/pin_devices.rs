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
    // VREF on its own 3.5 V rail (at least 3412 mV), so the trip holds and
    // the ADC rule is what refuses (A2 is judged first, `AET-WO-1217` F9).
    with_vref_rail(&mut v);
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

/// VREF tied to a 3.5 V ± 2.5 % rail of its own.
fn with_vref_rail(v: &mut Value) {
    v["voltage_domains"].as_array_mut().unwrap().push(json!({
        "id": "VREF_3V5", "nominal_millivolts": 3500, "tolerance_permille": 25
    }));
    device(v)["straps"]["VREF"] = json!({"tie": "VREF_3V5"});
}

/// F9 (`AET-WO-1217`): A2 is judged before the ADC rule, so a device both
/// refuse is named by its trip. With VREF on `GND` and a full scale above
/// VDDA, the trip's kind is reported, not the ADC rule's; with VREF on the
/// MCU's own rail, the trip below the full scale is.
#[test]
fn a_trip_is_named_before_the_adc_rule() {
    let mut v = d1();
    device(&mut v)["analog"][0]["transfer"]["current_output"]["resistor"]["ohms"] = json!(2000);
    assert_kind(&v, "TripBelowFullScale");
    device(&mut v)["straps"]["VREF"] = json!({"tie": "GND"});
    assert_kind(&v, "TripReferenceGrounded");
    // The ADC rule alone, with the trip met on its own rail: the same
    // resistor is the ADC rule's.
    with_vref_rail(&mut v);
    device(&mut v)["analog"][0]["transfer"]["current_output"]["resistor"]["ohms"] = json!(1600);
    assert_kind(&v, "AdcFullScaleExceeded");
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
        json!({"resistor": {"reference": "R19", "part": "CRCW0603_F_K", "ohms": 20000}});
    assert_kind(&v, "StrapLevelInvalid");
    // IMODE's resistor levels are 18.6-21.4 and 57.6-66.4 kOhm.
    let mut v = d1();
    device(&mut v)["straps"]["IMODE"] =
        json!({"resistor": {"reference": "R19", "part": "CRCW0603_F_K", "ohms": 40000}});
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
        json!({"resistor": {"reference": "R19", "part": "CRCW0603_F_K", "ohms": 20000}});
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
    // D1 without its nSLEEP signal: the bridge's idle reads an enable no
    // signal drives.
    let mut v = d1();
    device(&mut v)["signals"].as_array_mut().unwrap().remove(0);
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
    // D1's R2 (1.07 kΩ since `AET-WO-1216b`) is at most 1110 Ω, under RAIN's
    // 1150 Ω at 12 bits (Table 173): it needs no reservoir.
    spec(&v).validate().unwrap();
    // 1.5 kΩ is above it, and with no reservoir is refused.
    device(&mut v)["analog"][0]["transfer"]["current_output"]["resistor"]["ohms"] = json!(1500);
    assert_kind(&v, "AdcSourceImpedance");
    // At 10 bits RAIN is 5650 ohms: 1.5 kΩ needs none, and the refusal is
    // then step 25's (3 mA into 1555 Ω is above Table 10's 4.0 V).
    device(&mut v)["analog"][0]["adc_resolution_bits"] = json!(10);
    assert_kind(&v, "OffDomainAboveLimit");
}

/// `AET-WO-1218` (`1217` R5): a declared reservoir is a capacitor row
/// whether or not RAIN needs it. D1's R2 is under RAIN, so before this a
/// part no row holds, or a resistor row, validated and `emit` refused it.
#[test]
fn a_reservoir_rain_does_not_need_is_still_a_capacitor_row() {
    for part in ["NOPE", "CRCW0603_F_K"] {
        let mut v = d1();
        device(&mut v)["analog"][0]["transfer"]["current_output"]["reservoir"]["part"] =
            json!(part);
        let e = refused(&v);
        assert_eq!(
            (e.kind(), e.path()),
            (
                "PowerPartUnknown",
                Some("pin_devices[0].analog[0].transfer.current_output.reservoir")
            ),
            "{part}: {e}"
        );
        assert!(e.to_string().contains("is not a capacitor row"), "{e}");
    }
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
            active: Some(aethelgard_ir::motor::Level::Low),
            pulled: Some(aethelgard_ir::motor::Level::High),
        },
    );
    bad.pin_speeds.insert("DRV_NFAULT".into(), 0);
    assert_eq!(bad.validate().unwrap_err().kind(), "PinDeviceUndeclared");
    bad.pin_devices.insert(
        "U2".into(),
        aethelgard_ir::pin_devices::SolvedPinDevice {
            part: "DRV8876PWPR".into(),
            bridges: Default::default(),
            supplies: Default::default(),
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
        "bulk": {"reference": "C9", "part": "GCM32EC71H106KA03",
                 "source": "a test stand-in, not a design (AET-WO-1211e)"},
        "temperature_celsius": null
    }]);
    // IPROPI terminated as D1 terminates it (`TripSenseUnbound`, ruled D-A2).
    let mut ipropi = d1()["pin_devices"][0]["analog"][0].clone();
    ipropi["net"] = json!("HB_IPROPI");
    ipropi["transfer"]["current_output"]["resistor"]["reference"] = json!("R90");
    ipropi["transfer"]["current_output"]["reservoir"]["reference"] = json!("C90");
    device(&mut v)["analog"] = json!([ipropi]);
    device(&mut v)["temperature_celsius"] = d1()["pin_devices"][0]["temperature_celsius"].clone();
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

/// Adversarial pass D2: an input pair driven outside a declared bridge has
/// no idle judgement, so every channel input of the part's modes is a
/// bridge's.
#[test]
fn a_channel_input_outside_a_bridge_is_refused() {
    let mut v = d1();
    device(&mut v)["bridges"] = json!([]);
    for c in v["connectors"].as_array_mut().unwrap() {
        for p in c["pins"].as_array_mut().unwrap() {
            if p["net"].as_str().is_some_and(|n| n.starts_with("M1_")) {
                p["net"] = Value::Null;
            }
        }
    }
    v["motor_outputs"]
        .as_array_mut()
        .unwrap()
        .retain(|m| m["label"] == "SERVO1");
    for (pin, net) in [("EN/IN1", "G_EN"), ("PH/IN2", "G_PH")] {
        device(&mut v)["signals"].as_array_mut().unwrap().push(json!(
            {"device_pin": pin, "net": net, "pin": null, "direction": "output", "initial": "high"}
        ));
    }
    assert_kind(&v, "BridgeUndeclared");
}

/// Adversarial pass D4: SLVSDS7B states AERR only for 5.5 V <= VVM <= 37 V
/// (p. 6); on a 5 V motor supply no error is stated.
#[test]
fn an_error_band_outside_its_supply_condition_is_undeclared() {
    let mut v = d1();
    assert_eq!(v["voltage_domains"][1]["id"], "VMOTOR");
    v["voltage_domains"][1]["nominal_millivolts"] = json!(5000);
    let e = refused(&v);
    assert_eq!(e.kind(), "PartLimitUndeclared", "{e}");
    assert!(e.to_string().contains("5500"), "{e}");
}

/// Adversarial pass D3: a pin device's net is no battery sense's.
#[test]
fn a_device_net_named_as_a_sense_is_refused() {
    let power: Value =
        serde_json::from_str(include_str!("fixtures/sentinel-02-power.json")).unwrap();
    let sense = power["power"]["sense"][0]["net"].clone();
    let mut v = d1();
    device(&mut v)["analog"][0]["net"] = sense.clone();
    v["power"] = power["power"].clone();
    device(&mut v)["temperature_celsius"] = Value::Null;
    let e = aethelgard_ir::pin_devices::validate_against(
        &spec(&v),
        aethelgard_ir::parts::table().unwrap(),
    )
    .unwrap_err();
    assert_eq!(
        e,
        IrValidationError::DuplicateNetName {
            id: sense.as_str().unwrap().into()
        }
    );
}

/// Adversarial pass D5: an idle pulse of the whole period is `high`'s CCR,
/// a second spelling; one tick short of it validates and solves (CCR = ARR).
#[test]
fn an_idle_pulse_of_the_whole_period_is_spelled_twice() {
    let mut v = d1();
    motor(&mut v, "SERVO1")["pwm"]["idle"] = json!({"pulse_us": 20_000});
    assert_kind(&v, "IdleSpelledTwice");
    motor(&mut v, "SERVO1")["pwm"]["idle"] = json!({"pulse_us": 19_999});
    spec(&v).validate().unwrap();
}

/// `AET-WO-1210`'s rule 3: no emitter arm draws a pin device until
/// `AET-WO-1211d`, so D1 and D2 declare `board.drawn: false`; on a drawn
/// board the device is `PartNotDrawable`.
#[test]
fn a_pin_device_on_a_drawn_board_is_not_drawable() {
    let arms = ["BME280"];
    spec(&d1()).drawable_parts(&arms).unwrap();
    let mut v = d1();
    v["board"].as_object_mut().unwrap().remove("drawn");
    let e = spec(&v).drawable_parts(&arms).unwrap_err();
    assert_eq!(
        (e.kind(), e.path()),
        ("PartNotDrawable", Some("pin_devices[0].part"))
    );
}

/// Mutants G7 and G9: a topology whose idle `low` output's CCR is not 0 is
/// refused, and `check_silicon` fails a word whose prescaler does not give
/// the declared `resolution_ps`.
#[test]
fn the_idle_word_and_the_tick_are_judged_in_the_topology() {
    let t: aethelgard_ir::SolvedTopology = serde_json::from_str(include_str!(
        "../../aethelgard-svd-hal/tests/ui/motors-pwm/topology.json"
    ))
    .unwrap();
    let label = t
        .motor_outputs
        .iter()
        .find(|(_, m)| m.pwm.is_some())
        .map(|(l, _)| l.clone())
        .unwrap();
    let mut low = t.clone();
    let m = low.motor_outputs.get_mut(&label).unwrap();
    m.pwm.as_mut().unwrap().idle =
        aethelgard_ir::motor::Idle::Level(aethelgard_ir::motor::Level::Low);
    assert_eq!(low.validate().unwrap_err().kind(), "MotorOutputMismatch");
    low.motor_outputs.get_mut(&label).unwrap().ccr = 0;
    low.validate().unwrap();
    let timer: u32 = t.assignments[&label].peripheral[3..].parse().unwrap();
    let m = &t.motor_outputs[&label];
    assert!(aethelgard_ir::motor::check_silicon(&t.clock, timer, m).holds);
    let mut coarse = m.clone();
    coarse.pwm.as_mut().unwrap().resolution_ps = 2_000_000;
    assert!(!aethelgard_ir::motor::check_silicon(&t.clock, timer, &coarse).holds);
}

/// Ruled F6 (PR #292): with no rated IOD, a pull-up is held to the current
/// the sheet guarantees VOL at, and the refusal says so.
#[test]
fn a_pull_up_past_the_vol_test_current_is_named_as_such() {
    let mut v = d2_declared();
    device(&mut v)["temperature_celsius"] =
        json!({"min": -20, "max": 85, "source": "a declaration for this test"});
    device(&mut v)["signals"][1]["pull"] = json!({"resistor": {"reference": "R19",
        "part": "CRCW0603_F_K", "ohms": 600, "to": "VDD_3V3"}, "source": "a declaration for this test"});
    let e = refused(&v);
    assert_eq!(e.kind(), "PullOverDrive", "{e}");
    assert!(e.to_string().contains("not a rated IOD"), "{e}");
}

// ---------------------------------------------------------------- AET-WO-1211e

/// D2 declared, nFAULT pulled by a declared 10 kΩ to `to` on `pin`, with
/// D1's temperature (a resistor is judged over one). A domain other than
/// D2's own is added at `nominal` mV and `tol` ‰.
fn d2_pulled(to: &str, nominal: u32, tol: u32, pin: Value) -> Value {
    let mut v = d2_declared();
    if to != "VDD_3V3" && to != "VMOTOR" {
        v["voltage_domains"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id": to, "nominal_millivolts": nominal, "tolerance_permille": tol}));
    }
    let s = &mut device(&mut v)["signals"][1];
    assert_eq!(s["device_pin"], "nFAULT");
    s["pin"] = pin;
    s["pull"] = json!({
        "resistor": {"reference": "R19", "part": "CRCW0603_F_K", "ohms": 10000, "to": to},
        "source": "AET-WO-1211e test: a declared pull-up"
    });
    device(&mut v)["temperature_celsius"] = d1()["pin_devices"][0]["temperature_celsius"].clone();
    v
}

fn assert_refused_at(v: &Value, want: &str, path: &str, says: &str) {
    let e = refused(v);
    assert_eq!((e.kind(), e.path()), (want, Some(path)), "{e}");
    assert!(e.to_string().contains(says), "`{says}` not in: {e}");
}

const NFAULT_PULL: &str = "pin_devices[0].signals[1].pull";

/// A1: `1211d`'s board, D2's nFAULT by 10 kΩ to a 6.0 V ±5 % `VMOTOR`, puts
/// up to 6.3 V on the pin, above what any pin takes in operation (DS12117
/// Rev 10 Table 13 p. 105: FT below 5.5 V). Unpinned, the solver may place
/// it anywhere; on PA1, as `1211d`'s solve did, its FT_ha structure and the
/// table are named.
#[test]
fn a_pull_to_the_motor_domain_is_refused_pinned_or_not() {
    assert_refused_at(
        &d2_pulled("VMOTOR", 0, 0, Value::Null),
        "PullAboveMcuTolerance",
        NFAULT_PULL,
        "the solver may place it",
    );
    assert_refused_at(
        &d2_pulled("VMOTOR", 0, 0, json!("PA1")),
        "PullAboveMcuTolerance",
        NFAULT_PULL,
        "`PA1` is FT_ha (ST DS12117",
    );
    assert!(
        kind(&d2_pulled("VMOTOR", 0, 0, json!("PA1"))).contains("p. 105, Table 13"),
        "the refusal names Table 13"
    );
}

/// The same pull to the MCU's own 3.3 V passes, pinned or not, even on a TT
/// pin: on the same rail VIN never exceeds VDD. So does the MCU's own
/// pull-up (D2 as committed).
#[test]
fn the_same_pull_to_the_mcu_s_supply_validates() {
    for pin in [Value::Null, json!("PA1"), json!("PA4")] {
        spec(&d2_pulled("VDD_3V3", 0, 0, pin)).validate().unwrap();
    }
    spec(&d2_declared()).validate().unwrap();
}

/// A TT pin takes VDD + 0.3 V: 5 V ±5 % on PA4 (TT_a, Table 8 p. 69) is
/// refused, and so is a second 3.3 V ±5 % rail (3465 mV, above 3135 + 300),
/// unpinned or on PA4; the same rail on PA1 (FT) validates. 5 V on PA1
/// passes A1 and is refused by step 25: no tree orders `VDD_5V` after the
/// MCU's supply, and at VDD = 0 the pin takes 4.0 V (`AET-WO-1216b`).
#[test]
fn a_tt_pin_on_another_rail_is_refused_and_an_ft_pin_is_not() {
    assert_refused_at(
        &d2_pulled("VDD_5V", 5000, 50, json!("PA4")),
        "PullAboveMcuTolerance",
        NFAULT_PULL,
        "`PA4` is TT_a",
    );
    for pin in [Value::Null, json!("PA4")] {
        assert_eq!(
            refused(&d2_pulled("VDD_3V3B", 3300, 50, pin)).kind(),
            "PullAboveMcuTolerance"
        );
    }
    spec(&d2_pulled("VDD_3V3B", 3300, 50, json!("PA1")))
        .validate()
        .unwrap();
    assert_refused_at(
        &d2_pulled("VDD_5V", 5000, 50, json!("PA1")),
        "OffDomainAboveLimit",
        "pin_devices[0].signals[1].pull.resistor",
        "up to 5250 mV",
    );
}

/// At or below VDD's minimum + 0.3 V (3435 mV), the least any held structure
/// takes, an unpinned pull passes; 1 mV above, it is refused.
#[test]
fn an_unpinned_pull_is_judged_against_the_least_structure_to_the_millivolt() {
    // 3367 mV +2 % is 3434.34, up to 3435; 3368 is 3435.36, up to 3436.
    spec(&d2_pulled("VDD_3V4", 3367, 20, Value::Null))
        .validate()
        .unwrap();
    assert_refused_at(
        &d2_pulled("VDD_3V4", 3368, 20, Value::Null),
        "PullAboveMcuTolerance",
        NFAULT_PULL,
        "up to 3436 mV, above 3435 mV",
    );
}

/// An FT pin takes less than 5.5 V in operation: up to 5499 mV passes A1
/// (and is refused by step 25 at VDD = 0, `AET-WO-1216b`), 5500 is refused
/// by A1.
#[test]
fn an_ft_pin_is_judged_below_5v5_to_the_millivolt() {
    // 5444 mV +1 % is 5498.44, up to 5499; 5445 is 5499.45, up to 5500.
    assert_eq!(
        refused(&d2_pulled("VDD_5V5", 5444, 10, json!("PA1"))).kind(),
        "OffDomainAboveLimit"
    );
    assert_refused_at(
        &d2_pulled("VDD_5V5", 5445, 10, json!("PA1")),
        "PullAboveMcuTolerance",
        NFAULT_PULL,
        "takes 5499 mV",
    );
}

/// FT takes `Min(VDD + 3.6 V, 5.5 V)`, least at VDD's minimum: with the MCU
/// at 1.9 V ±5 % (1805 to 1995 mV) an FT pin takes 5405 mV, and a pull up to
/// 5454 mV is refused, though it is under 5499 mV, what VDD's maximum gives.
#[test]
fn ft_takes_its_supply_at_the_minimum() {
    let mut v = d2_pulled("VDD_5V4", 5400, 10, json!("PA1"));
    v["voltage_domains"][0]["nominal_millivolts"] = json!(1900);
    v["mcu"]["bor_level"] = json!(0);
    // nFAULT first: at 1.9 V the MCU drives no input of the part high
    // (`LogicLevelIncompatible`), which this pull is judged before.
    device(&mut v)["signals"].as_array_mut().unwrap().swap(0, 1);
    let e = refused(&v);
    assert_eq!(e.kind(), "PullAboveMcuTolerance", "{e}");
    assert!(
        e.to_string()
            .contains("takes 5405 mV in operation with `VDD_3V3` at its minimum of 1805 mV"),
        "{e}"
    );
}

/// A pin holding an internal pull from reset to the first write (PB4 up,
/// PA14 down; RM0433 Rev 9 p. 542) takes no pull above VDD + 0.3 V (Table
/// 13 note 3): 3876 mV is refused there, and PB5, FT with none, takes it.
#[test]
fn a_reset_pulled_pin_above_vdd_plus_0v3_is_refused() {
    for pin in ["PB4", "PA14"] {
        assert_refused_at(
            &d2_pulled("VDD_3V8", 3800, 20, json!(pin)),
            "PullAboveMcuTolerance",
            NFAULT_PULL,
            "note 3",
        );
    }
    spec(&d2_pulled("VDD_3V8", 3800, 20, json!("PB5")))
        .validate()
        .unwrap();
}

/// A hand pin with no held structure (PI0 is no LQFP144 pin) is refused
/// above the floor, and no structure is assumed; on the MCU's own rail it is
/// left to `solve`.
#[test]
fn a_pin_with_no_held_structure_is_refused_above_the_floor() {
    assert_refused_at(
        &d2_pulled("VDD_5V", 5000, 50, json!("PI0")),
        "McuPinStructureUnheld",
        "pin_devices[0].signals[1].pin",
        "no structure is assumed",
    );
    spec(&d2_pulled("VDD_3V3", 0, 0, json!("PI0")))
        .validate()
        .unwrap();
}

/// A2: a DRV8876 whose IPROPI no analog input terminates is refused, even
/// with a VREF that would pass (SLVSDS7B pp. 12-13: IPROPI to ground through
/// R_IPROPI, or tied to GND; ruled D-A2).
#[test]
fn a_trip_sense_no_analog_input_terminates_is_refused() {
    let mut v = d1();
    device(&mut v)["analog"] = json!([]);
    assert_refused_at(&v, "TripSenseUnbound", "pin_devices[0].analog", "`IPROPI`");
}

/// D1 with VREF on a domain of its own: `vref` mV at `tol` ‰.
fn d1_vref(nominal: u32, tol: u32) -> Value {
    let mut v = d1();
    v["voltage_domains"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": "VREF_D", "nominal_millivolts": nominal, "tolerance_permille": tol}));
    device(&mut v)["straps"]["VREF"] = json!({"tie": "VREF_D"});
    v
}

/// A2: VREF on `GND` chops at no current (SLVSDS7B p. 13, Eq. 3).
#[test]
fn vref_on_ground_is_refused() {
    let mut v = d1();
    device(&mut v)["straps"]["VREF"] = json!({"tie": "GND"});
    assert_refused_at(
        &v,
        "TripReferenceGrounded",
        "pin_devices[0].straps.VREF",
        "chop at no current",
    );
}

/// A2: at D1's 1.9 A full scale IPROPI reaches up to 2,214,450 µV (1995 µA
/// at AERR's 5 %, R2 1.07 kΩ at most 1110 Ω, `AET-WO-1216b`). A 0.5 V VREF
/// trips near 0.3 A; a 3.3 V ±1 % one (3267 mV at least) passes; 2.25 V ±2 %
/// (2205 to 2295 mV) straddles it and is refused at its minimum.
#[test]
fn a_vref_below_the_full_scale_is_refused_and_one_above_it_passes() {
    assert_refused_at(
        &d1_vref(500, 50),
        "TripBelowFullScale",
        "pin_devices[0].straps.VREF",
        "up to 2214450 uV across at most 1110 ohms, above `VREF_D`'s minimum of 475 mV",
    );
    spec(&d1_vref(3300, 10)).validate().unwrap();
    assert_refused_at(
        &d1_vref(2250, 20),
        "TripBelowFullScale",
        "pin_devices[0].straps.VREF",
        "minimum of 2205 mV",
    );
}

/// A2 to the microvolt: R2 at 1157 Ω is at most 1200 Ω, so 1995 µA makes
/// exactly 2,394,000 µV. A VREF whose minimum is 2394 mV (2520 mV −5 %)
/// passes (it chops at the full scale, not below it); 2393 mV is refused.
/// At 3 mA, 1200 Ω is 3.6 V, under step 25's 4.0 V (`AET-WO-1216b`).
#[test]
fn a_vref_equal_to_the_full_scale_passes_and_one_millivolt_less_is_refused() {
    let at = |nominal: u32| {
        let mut v = d1_vref(nominal, 50);
        device(&mut v)["analog"][0]["transfer"]["current_output"]["resistor"]["ohms"] = json!(1157);
        v
    };
    spec(&at(2520)).validate().unwrap();
    assert_refused_at(
        &at(2519),
        "TripBelowFullScale",
        "pin_devices[0].straps.VREF",
        "up to 2394000 uV",
    );
}

/// `bulk`: D1 without it is refused (CVM2 has no stated value, SLVSDS7B
/// p. 10 Table 1, §9.1 p. 31); with the fixture's stand-in it validates.
#[test]
fn d1_without_bulk_is_refused_and_with_it_validates() {
    let mut v = d1();
    device(&mut v)["bulk"] = Value::Null;
    assert_refused_at(
        &v,
        "BulkUndeclared",
        "pin_devices[0].bulk",
        "`VM`'s bulk capacitor",
    );
    spec(&d1()).validate().unwrap();
}

/// A `bulk` on the DRV8833, whose sheet states its VM capacitor, is refused.
#[test]
fn bulk_on_a_drv8833_is_refused() {
    let mut v = d2_declared();
    device(&mut v)["bulk"] = d1()["pin_devices"][0]["bulk"].clone();
    assert_refused_at(&v, "BulkNotAllowed", "pin_devices[0].bulk", "DRV8833PWPR");
}

/// A `bulk` rated at or below the VM domain's maximum is refused: 10 V
/// against 12.6 V, and 16 V against exactly 16000 mV; 15999 mV passes.
#[test]
fn a_bulk_rated_at_or_below_the_domain_is_refused() {
    let mut v = d1();
    device(&mut v)["bulk"]["part"] = json!("GRM32ER71A226KE20");
    assert_refused_at(
        &v,
        "BulkUnderRated",
        "pin_devices[0].bulk.part",
        "rated 10000 mV",
    );
    // 15238 mV +5 % is 15999.9, up to 16000; 15237 is 15998.85, up to 15999.
    let vm = |nominal: u32| {
        let mut v = d1();
        device(&mut v)["bulk"]["part"] = json!("GRM188R71C105KA12");
        v["voltage_domains"][1]["nominal_millivolts"] = json!(nominal);
        assert_eq!(v["voltage_domains"][1]["id"], "VMOTOR");
        v
    };
    assert_refused_at(
        &vm(15238),
        "BulkUnderRated",
        "pin_devices[0].bulk.part",
        "not above `VMOTOR`'s maximum of 16000 mV",
    );
    spec(&vm(15237)).validate().unwrap();
}

/// A `bulk` part that is no capacitor row, a reference another part has,
/// and a malformed reference or source are each refused by name.
#[test]
fn a_bulk_s_part_reference_and_source_are_judged() {
    let mut v = d1();
    device(&mut v)["bulk"]["part"] = json!("CRCW0603_F_K");
    assert_refused_at(
        &v,
        "PowerPartUnknown",
        "pin_devices[0].bulk.part",
        "not a capacitor or bulk_capacitor row",
    );
    // An electrolytic row is a bulk: the Nichicon 47 uF, 50 V.
    let mut v = d1();
    device(&mut v)["bulk"]["part"] = json!("UCZ1H470MCL1GS");
    spec(&v).validate().unwrap();
    let mut v = d1();
    device(&mut v)["bulk"]["reference"] = json!("C1");
    assert_eq!(refused(&v).kind(), "DuplicateReference");
    let mut v = d1();
    device(&mut v)["bulk"]["reference"] = json!("C 2");
    assert_eq!(refused(&v).path(), Some("pin_devices[0].bulk.reference"));
    let mut v = d1();
    device(&mut v)["bulk"]["source"] = json!(" padded");
    assert_eq!(
        (refused(&v).kind(), refused(&v).path()),
        ("InvalidSource", Some("pin_devices[0].bulk.source"))
    );
}

/// `bulk` is a required key: a device without it does not parse.
#[test]
fn bulk_is_a_required_key() {
    let mut v = d2_declared();
    device(&mut v).as_object_mut().unwrap().remove("bulk");
    let e = serde_json::from_value::<Specification>(v).unwrap_err();
    assert!(e.to_string().contains("bulk"), "{e}");
}

/// A pin part's `trip` names a strap_analog reference and a current_output
/// sense, and its `bulk_unstated` a supply pin (`AET-WO-1211e`); a row
/// naming any other is refused by name.
#[test]
fn a_trip_or_bulk_row_on_the_wrong_pin_is_refused() {
    for (field, pin, says) in [
        ("/trip/reference", "IMODE", "`trip`"),
        ("/trip/sense", "nFAULT", "`trip`"),
        ("/bulk_unstated/supply", "VCP", "`bulk_unstated`"),
    ] {
        let mut doc: Value = serde_json::from_str(PARTS_JSON).unwrap();
        let rows = doc["pin_parts"].as_array_mut().unwrap();
        let row = rows
            .iter_mut()
            .find(|r| r["part"] == "DRV8876PWPR")
            .unwrap();
        *row.pointer_mut(field).unwrap() = json!(pin);
        let e = parse_parts(&doc.to_string()).unwrap_err();
        assert!(e.to_string().contains(says), "{field}: {e}");
    }
}

// ---------------------------------------------------------------- AET-WO-1217

/// D1 with IPROPI tied to `GND` and no ADC input on it: SLVSDS7B p. 13's
/// way to turn current regulation off (F2).
fn d1_ipropi_tied() -> Value {
    let mut v = d1();
    device(&mut v)["straps"]["IPROPI"] = json!({"tie": "GND"});
    device(&mut v)["analog"] = json!([]);
    v
}

/// F2: IPROPI on `GND` with VREF above it validates, and no trip is judged.
#[test]
fn ipropi_tied_to_gnd_with_vref_above_it_validates() {
    spec(&d1_ipropi_tied()).validate().unwrap();
    // No trip is judged: a full scale the trip could not meet is not read.
    let mut v = d1_ipropi_tied();
    v["voltage_domains"].as_array_mut().unwrap().push(json!({
        "id": "VREF_LOW", "nominal_millivolts": 500, "tolerance_permille": 50
    }));
    device(&mut v)["straps"]["VREF"] = json!({"tie": "VREF_LOW"});
    spec(&v).validate().unwrap();
}

/// F2: with the tie, VREF on `GND` still chops at no current.
#[test]
fn ipropi_tied_to_gnd_with_vref_on_gnd_is_refused() {
    let mut v = d1_ipropi_tied();
    device(&mut v)["straps"]["VREF"] = json!({"tie": "GND"});
    let e = refused(&v);
    assert_eq!(e.kind(), "TripReferenceGrounded", "{e}");
    assert_eq!(e.path(), Some("pin_devices[0].straps.VREF"));
}

/// F2: the tie leaves no current to read, so an ADC input on IPROPI as well
/// is the pin bound twice.
#[test]
fn ipropi_tied_and_read_is_bound_twice() {
    let mut v = d1();
    device(&mut v)["straps"]["IPROPI"] = json!({"tie": "GND"});
    let e = refused(&v);
    assert_eq!(e.kind(), "DevicePinBoundTwice", "{e}");
    assert_eq!(e.path(), Some("pin_devices[0].analog[0]"));
    // Before any ADC rule reads it: a full scale above VDDA as well is still
    // the pin bound twice.
    device(&mut v)["analog"][0]["transfer"]["current_output"]["resistor"]["ohms"] = json!(2000);
    assert_kind(&v, "DevicePinBoundTwice");
}

/// F2: only a tie to `GND` is admitted, and only on the pin the row's trip
/// senses: IPROPI tied to a rail, or on a row with no trip, stays refused.
#[test]
fn a_current_output_takes_only_the_trips_gnd_tie() {
    let mut v = d1_ipropi_tied();
    device(&mut v)["straps"]["IPROPI"] = json!({"tie": "VDD_3V3"});
    let e = refused(&v);
    assert_eq!(e.kind(), "SignalKindMismatch", "{e}");
    assert_eq!(e.path(), Some("pin_devices[0].straps.IPROPI"));
    assert!(e.to_string().contains("only a tie to `GND`"), "{e}");

    let mut doc: Value = serde_json::from_str(PARTS_JSON).unwrap();
    let rows = doc["pin_parts"].as_array_mut().unwrap();
    let row = rows
        .iter_mut()
        .find(|r| r["part"] == "DRV8876PWPR")
        .unwrap();
    row.as_object_mut().unwrap().remove("trip");
    let table = parse_parts(&doc.to_string()).unwrap();
    let e =
        aethelgard_ir::pin_devices::validate_against(&spec(&d1_ipropi_tied()), &table).unwrap_err();
    assert_eq!(e.kind(), "SignalKindMismatch", "{e}");
    assert_eq!(e.path(), Some("pin_devices[0].straps.IPROPI"));
}

/// F10: an MCU with no power-parts row is refused once, at `mcu.part`,
/// naming the part, before any pin device is judged against it; a
/// specification with no pin device is not refused here.
#[test]
fn an_unknown_mcu_is_refused_at_mcu_part() {
    for part in ["STM32H743ZITx", "GRM32ER71A226KE20"] {
        let mut v = d1();
        v["mcu"]["part"] = json!(part);
        let e = refused(&v);
        assert_eq!(e.kind(), "PowerPartUnknown", "{part}: {e}");
        assert_eq!(e.path(), Some("mcu.part"), "{part}: {e}");
        assert_eq!(e.id(), Some(part), "{part}: {e}");
    }
    let mut v: Value =
        serde_json::from_str(include_str!("../../../examples/sentinel-02-bench.json")).unwrap();
    v["mcu"]["part"] = json!("STM32H743ZITx");
    spec(&v).validate().unwrap();
}

/// Adversarial A1 (`AET-WO-1217`): with the tie, VREF must be above `GND` at
/// its minimum; a domain that reaches 0 mV is the reference on `GND`.
#[test]
fn ipropi_tied_with_vref_reaching_0_mv_is_refused() {
    let mut v = d1_ipropi_tied();
    v["voltage_domains"].as_array_mut().unwrap().push(json!({
        "id": "VREF_1MV", "nominal_millivolts": 1, "tolerance_permille": 500
    }));
    device(&mut v)["straps"]["VREF"] = json!({"tie": "VREF_1MV"});
    let e = refused(&v);
    assert_eq!(e.kind(), "TripReferenceGrounded", "{e}");
    assert_eq!(e.path(), Some("pin_devices[0].straps.VREF"));
    v["voltage_domains"][2]["nominal_millivolts"] = json!(2);
    spec(&v).validate().unwrap();
}

/// Adversarial A4 (`AET-WO-1217`): with the trip judged first, an ADC entry
/// on the wrong pin is still refused by its own kind, not as the sense left
/// unterminated.
#[test]
fn a_misplaced_adc_entry_is_named_by_its_own_kind() {
    let mut v = d1();
    device(&mut v)["analog"][0]["device_pin"] = json!("ipropi");
    assert_kind(&v, "UnknownDevicePin");
    device(&mut v)["analog"][0]["device_pin"] = json!("nFAULT");
    let e = refused(&v);
    assert_eq!(e.kind(), "SignalKindMismatch", "{e}");
    assert_eq!(e.path(), Some("pin_devices[0].analog[0]"));
}
