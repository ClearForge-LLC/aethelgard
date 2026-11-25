//! `AET-WO-1211c`: a pin device's overrides (`pin_held` and `analog_value`,
//! scenario version 5, tags 8 and 9) and records (kinds 33 to 37: a GPIO
//! signal declared and witnessed, an ADC input declared and witnessed, a
//! motor output held at a static level and its witness): the parse, the
//! archive's round trip, each record's field rules, `adc_code`, and
//! `check_run`'s joins and its re-derivation of the verdict's GPIO, ADC and
//! motor level findings.

mod common;

use aethelgard_chaos::campaign::{
    ADC_MISMATCH_FINDING, ADC_UNWITNESSED_FINDING, GPIO_INPUT_ASSERTED_FINDING,
    GPIO_MISMATCH_FINDING, GPIO_UNWITNESSED_FINDING, MOTOR_LEVEL_FINDING, Scenario, check_run,
};
use aethelgard_chaos::cbor::Value;
use aethelgard_chaos::record::{
    ADC_STATUSES, ANALOG_MAX_MILLIVOLTS, GPIO_STATUSES, LAST_KIND, LEVEL_STATUSES, Level,
    MOTOR_STATUSES, Override, Reading, Record, adc_code,
};
use aethelgard_chaos::{Event, replay_dir};
use serde_json::{Value as J, json};

use common::*;

const BOOT_WORD: u32 = 0x2100_B007;

// ---- the scenario file ----

fn file(version: u64, overrides: J) -> J {
    json!({
        "schema_version": version,
        "name": "pins",
        "base": "b.json",
        "expect": "pass",
        "expect_findings": [],
        "overrides": overrides,
    })
}

fn parse(v: &J) -> Result<Scenario, aethelgard_chaos::campaign::ScenarioError> {
    Scenario::parse(v.to_string().as_bytes())
}

fn held(net: &str, level: &str) -> J {
    json!({"pin_held": {"net": net, "level": level}})
}

fn analog(net: &str, mv: u64) -> J {
    json!({"analog_value": {"net": net, "millivolts": mv}})
}

#[test]
fn version_5_admits_both_overrides_and_exposes_them() {
    let s = parse(&file(
        5,
        json!([
            held("BTN", "high"),
            held("SW", "low"),
            analog("VSENSE", 1500)
        ]),
    ))
    .unwrap();
    assert_eq!(s.pins_held(), [("BTN", Level::High), ("SW", Level::Low)]);
    assert_eq!(s.analog_values(), [("VSENSE", 1500)]);
    // Neither is a version-4 scenario's old override.
    assert_eq!(parse(&file(5, json!([]))).unwrap().pins_held(), []);
    // Another override of the same file is kept beside them.
    let both = parse(&file(
        5,
        json!([{"serial_rx_open": "CRSF"}, held("BTN", "low")]),
    ))
    .unwrap();
    assert_eq!(both.overrides.len(), 2);
}

#[test]
fn versions_1_to_4_refuse_each_override_by_name() {
    for version in 1..=4 {
        for o in [held("BTN", "high"), analog("VSENSE", 1500)] {
            let e = parse(&file(version, json!([o.clone()]))).unwrap_err();
            assert_eq!(e.kind, "SchemaViolation", "{version} {o}: {e}");
            assert!(e.message.contains("schema_version 5"), "{version} {o}: {e}");
        }
    }
}

#[test]
fn an_analog_value_is_bounded_by_the_adc_reference() {
    assert_eq!(ANALOG_MAX_MILLIVOLTS, 3300);
    for mv in [0, 1, 3300] {
        assert!(parse(&file(5, json!([analog("V", mv)]))).is_ok(), "{mv}");
    }
    for mv in [3301, u64::from(u32::MAX) + 1, u64::MAX] {
        let e = parse(&file(5, json!([analog("V", mv)]))).unwrap_err();
        assert_eq!(e.kind, "SchemaViolation", "{mv}");
        assert!(e.message.contains("0 to 3300"), "{mv}: {e}");
    }
    for bad in [json!(-1), json!(1.5), json!("3"), json!(null)] {
        let v = file(
            5,
            json!([{"analog_value": {"net": "V", "millivolts": bad.clone()}}]),
        );
        assert_eq!(parse(&v).unwrap_err().kind, "SchemaViolation", "{bad}");
    }
}

#[test]
fn a_level_is_low_or_high_and_nothing_else() {
    for level in [
        json!("medium"),
        json!("High"),
        json!("LOW"),
        json!(1),
        json!(true),
        json!(null),
        json!(""),
    ] {
        let v = file(
            5,
            json!([{"pin_held": {"net": "BTN", "level": level.clone()}}]),
        );
        assert_eq!(parse(&v).unwrap_err().kind, "SchemaViolation", "{level}");
    }
}

#[test]
fn the_keys_of_each_override_are_closed() {
    let bad: Vec<J> = vec![
        // The nested object's keys.
        json!({"pin_held": {"net": "BTN"}}),
        json!({"pin_held": {"level": "low"}}),
        json!({"pin_held": {"net": "BTN", "level": "low", "extra": 1}}),
        json!({"pin_held": {"net": "", "level": "low"}}),
        json!({"pin_held": {"net": 5, "level": "low"}}),
        json!({"pin_held": ["BTN", "low"]}),
        json!({"pin_held": "BTN"}),
        json!({"analog_value": {"net": "V"}}),
        json!({"analog_value": {"millivolts": 5}}),
        json!({"analog_value": {"net": "V", "millivolts": 5, "extra": 1}}),
        json!({"analog_value": {"net": "", "millivolts": 5}}),
        json!({"analog_value": [ "V", 5 ]}),
        // The top-level keys beside each: every old key, and the other new one.
        json!({"pin_held": {"net": "B", "level": "low"}, "net": "B"}),
        json!({"pin_held": {"net": "B", "level": "low"}, "bus": "I2C1"}),
        json!({"pin_held": {"net": "B", "level": "low"}, "serial_rx_open": "CRSF"}),
        json!({"pin_held": {"net": "B", "level": "low"}, "spi_device_absent": "U1"}),
        json!({"pin_held": {"net": "B", "level": "low"}, "rail_out_of_band": "V"}),
        json!({"pin_held": {"net": "B", "level": "low"}, "battery_sag": {"series_milliohms": 5, "start_ns": 1}}),
        json!({"pin_held": {"net": "B", "level": "low"}, "droop": null}),
        json!({"pin_held": {"net": "B", "level": "low"}, "pull_up_ohms": 5}),
        json!({"pin_held": {"net": "B", "level": "low"}, "analog_value": {"net": "V", "millivolts": 1}}),
        json!({"analog_value": {"net": "V", "millivolts": 1}, "net": "V"}),
        json!({"analog_value": {"net": "V", "millivolts": 1}, "serial_rx_open": "CRSF"}),
        json!({"analog_value": {"net": "V", "millivolts": 1}, "line_held_low": "sda"}),
        json!({"analog_value": {"net": "V", "millivolts": 1}, "rail_out_of_band": "V"}),
        // An old override beside a new key is dropped by none of them.
        json!({"serial_rx_open": "CRSF", "analog_value": {"net": "V", "millivolts": 1}}),
        json!({"rail_out_of_band": "V", "pin_held": {"net": "B", "level": "low"}}),
    ];
    for o in bad {
        let e = parse(&file(5, json!([o.clone()]))).unwrap_err();
        assert_eq!(e.kind, "SchemaViolation", "{o}: {e}");
    }
}

#[test]
fn the_catch_all_names_the_new_overrides() {
    let e = parse(&file(5, json!([{}]))).unwrap_err();
    assert!(
        e.message.contains("{pin_held} or {analog_value}"),
        "{}",
        e.message
    );
}

#[test]
fn two_overrides_of_one_net_are_a_duplicate() {
    for pair in [
        json!([held("BTN", "high"), held("BTN", "low")]),
        json!([held("BTN", "high"), held("BTN", "high")]),
        json!([analog("V", 1), analog("V", 2)]),
    ] {
        let e = parse(&file(5, pair.clone())).unwrap_err();
        assert_eq!(e.kind, "DuplicateOverride", "{pair}");
        assert!(e.id.is_some());
    }
    // Different nets are two things.
    assert!(parse(&file(5, json!([held("A", "high"), held("B", "high")]))).is_ok());
    assert!(parse(&file(5, json!([analog("A", 1), analog("B", 1)]))).is_ok());
}

// ---- adc_code ----

#[test]
fn adc_code_is_the_conversion_renode_applies() {
    // `AET-WO-1211` §4: A3 at 1.65 V reads 0x8000 and A5 at 0.825 V 0x4000
    // at 16 bits; `spike/1211c/RESULTS.md` P4e: 1.5 V reads 0x745 = 1861 at
    // 12 bits.
    assert_eq!(adc_code(1_650_000, 16), 0x8000);
    assert_eq!(adc_code(825_000, 16), 0x4000);
    assert_eq!(adc_code(1_500_000, 12), 0x745);
    assert_eq!(adc_code(1_500_000, 12), 1861);
    // The ends of the range, and round-half-up at a tie.
    assert_eq!(adc_code(0, 12), 0);
    assert_eq!(adc_code(3_300_000, 12), 4095);
    assert_eq!(adc_code(3_300_000, 16), 65_535);
    assert_eq!(adc_code(1_650_000, 1), 1);
    // No input panics.
    assert_eq!(adc_code(u64::MAX, 255), u32::MAX);
    let _ = adc_code(u64::MAX, 12);
    let _ = adc_code(1, 0);
}

// ---- the records ----

fn gpio_out() -> Record {
    Record::PinGpioDeclared {
        net: "LED".into(),
        pin: "PA5".into(),
        output: true,
        level: 0,
        active: None,
        held: false,
    }
}

fn gpio_in(level: u8, active: u8, held: bool) -> Record {
    Record::PinGpioDeclared {
        net: "BTN".into(),
        pin: "PB2".into(),
        output: false,
        level,
        active: Some(active),
        held,
    }
}

fn gpio_witness(net: &str, status: &str) -> Record {
    Record::PinGpioWitness {
        net: net.into(),
        status: status.into(),
        bsrr: None,
        ordered: None,
        level: None,
    }
}

/// The output's witness for a declared low level on `PA5`: the reset bit
/// (bit 21), the first write, before its `MODER` write.
fn led_witness() -> Record {
    Record::PinGpioWitness {
        net: "LED".into(),
        status: "witnessed".into(),
        bsrr: Some(1 << 21),
        ordered: Some(true),
        level: None,
    }
}

fn btn_witness(level: u8) -> Record {
    Record::PinGpioWitness {
        net: "BTN".into(),
        status: "witnessed".into(),
        bsrr: None,
        ordered: None,
        level: Some(level),
    }
}

fn adc_decl(held: bool) -> Record {
    Record::PinAdcDeclared {
        net: "VSENSE".into(),
        pin: "PA0".into(),
        peripheral: "ADC1".into(),
        channel: 16,
        bits: 12,
        microvolts: 1_500_000,
        held,
        code: 1861,
    }
}

fn adc_witness(status: &str, reads: u64, code: Option<u32>) -> Record {
    Record::PinAdcWitness {
        net: "VSENSE".into(),
        status: status.into(),
        reads,
        code,
    }
}

fn level_witness(status: &str) -> Record {
    let (settle_ns, end_level) = match status {
        "unanchored" => (None, Some(0)),
        // An idle low settles at its anchor, kind 26's 6 ns (adversarial F1).
        "unreadable" => (Some(6), None),
        "mismatch" => (Some(6), Some(1)),
        _ => (Some(6), Some(0)),
    };
    Record::MotorLevelWitness {
        label: "M1".into(),
        level: 0,
        settle_ns,
        rises_after: 0,
        falls_after: 0,
        end_level,
        status: status.into(),
    }
}

#[test]
fn every_record_and_override_round_trips_and_has_its_kind() {
    let records = vec![
        Record::Scenario {
            name: "pins".into(),
            sha256: [1; 32],
            expect_pass: true,
            expect_findings: vec![],
            overrides: vec![
                Override::PinHeld {
                    net: "BTN".into(),
                    level: Level::High,
                },
                Override::PinHeld {
                    net: "SW".into(),
                    level: Level::Low,
                },
                Override::AnalogValue {
                    net: "VSENSE".into(),
                    millivolts: 3300,
                },
            ],
        },
        gpio_out(),
        gpio_in(1, 0, true),
        adc_decl(true),
        gpio_witness("LED", "unwitnessed"),
        led_witness(),
        btn_witness(1),
        adc_witness("witnessed", 3, Some(1861)),
        adc_witness("unwitnessed", 0, None),
        level_witness("witnessed"),
        level_witness("unanchored"),
        level_witness("unreadable"),
    ];
    let events: Vec<Event> = records
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, record)| Event::Record {
            timestamp: i as u64,
            record,
        })
        .collect();
    let (dir, _) = write_run("pins-roundtrip", 9, &events);
    assert_eq!(replay_dir(&dir).unwrap().state.records, records);
    // 38 to 40 since AET-WO-4400i: a foreign run's, round-tripped in archive.rs.
    assert_eq!(LAST_KIND, 40);
    let kinds: Vec<u64> = records[1..4].iter().map(Record::kind).collect();
    assert_eq!(kinds, [33, 33, 35]);
    assert_eq!(records[5].kind(), 34);
    assert_eq!(records[7].kind(), 36);
    assert_eq!(records[9].kind(), 37);
    // Each record's own fields are what the reader's decode gives back.
    for r in &records {
        assert_eq!(Record::from_fields(r.kind(), &r.fields()).as_ref(), Some(r));
        assert_eq!(r.check(), Ok(()), "{r:?}");
    }
}

#[test]
fn kinds_33_and_35_are_archived_before_the_boot_and_the_rest_after() {
    for r in [gpio_out(), adc_decl(false)] {
        assert!(r.before_boot(), "{r:?}");
        assert_eq!(r.timestamp(99), 0);
    }
    for r in [
        led_witness(),
        adc_witness("unwitnessed", 0, None),
        level_witness("witnessed"),
    ] {
        assert!(!r.before_boot(), "{r:?}");
        assert_eq!(r.timestamp(99), 99);
    }
}

#[test]
fn the_decode_refuses_what_is_not_a_level_or_a_flag() {
    // A level byte of 2 in an override.
    let scenario = |o: Value| {
        Record::from_fields(
            5,
            &[
                Value::Text("pins".into()),
                Value::Bytes(vec![1; 32]),
                Value::Uint(1),
                Value::Array(vec![]),
                Value::Array(vec![o]),
            ],
        )
    };
    let tag = |n: u64, a: &str, b: u64| {
        Value::Array(vec![Value::Uint(n), Value::Text(a.into()), Value::Uint(b)])
    };
    assert!(scenario(tag(8, "BTN", 0)).is_some());
    assert!(scenario(tag(8, "BTN", 1)).is_some());
    assert!(scenario(tag(8, "BTN", 2)).is_none());
    assert!(scenario(tag(9, "V", u64::from(u32::MAX))).is_some());
    assert!(scenario(tag(9, "V", u64::from(u32::MAX) + 1)).is_none());
    assert!(scenario(tag(10, "V", 0)).is_none());
    // `ordered` is a flag: 0 or 1.
    let ordered = |n: u64| {
        Record::from_fields(
            34,
            &[
                Value::Text("LED".into()),
                Value::Text("witnessed".into()),
                Value::Array(vec![Value::Uint(1)]),
                Value::Array(vec![Value::Uint(n)]),
                Value::Array(vec![]),
            ],
        )
    };
    assert!(ordered(0).is_some() && ordered(1).is_some());
    assert!(ordered(2).is_none());
    // `held` and `output` are flags too.
    let declared = |output: u64| {
        Record::from_fields(
            33,
            &[
                Value::Text("LED".into()),
                Value::Text("PA5".into()),
                Value::Uint(output),
                Value::Uint(0),
                Value::Array(vec![]),
                Value::Uint(0),
            ],
        )
    };
    assert!(declared(1).is_some() && declared(2).is_none());
}

fn refused(r: &Record) -> &'static str {
    r.check().unwrap_err()
}

#[test]
fn a_declared_gpio_signal_is_refused_for_each_malformed_field() {
    let edit = |f: &dyn Fn(&mut Record)| {
        let mut r = gpio_in(1, 0, false);
        f(&mut r);
        refused(&r)
    };
    let net = "a declared GPIO signal's net or pin is malformed";
    assert_eq!(edit(&|r| set_net(r, "")), net);
    for pin in ["", "PA16", "pa5", "A5", "PA05", "P5"] {
        assert_eq!(
            edit(&|r| {
                if let Record::PinGpioDeclared { pin: p, .. } = r {
                    *p = pin.into();
                }
            }),
            net,
            "{pin}"
        );
    }
    let levels = "a declared GPIO signal's level or active level is not 0 or 1";
    assert_eq!(
        edit(&|r| {
            if let Record::PinGpioDeclared { level, .. } = r {
                *level = 2;
            }
        }),
        levels
    );
    assert_eq!(
        edit(&|r| {
            if let Record::PinGpioDeclared { active, .. } = r {
                *active = Some(2);
            }
        }),
        levels
    );
    assert_eq!(
        edit(&|r| {
            if let Record::PinGpioDeclared { active, .. } = r {
                *active = None;
            }
        }),
        "a declared GPIO input has no active level"
    );
    let out = "a declared GPIO output has an active level or is held";
    let mut r = gpio_out();
    if let Record::PinGpioDeclared { active, .. } = &mut r {
        *active = Some(0);
    }
    assert_eq!(refused(&r), out);
    let mut r = gpio_out();
    if let Record::PinGpioDeclared { held, .. } = &mut r {
        *held = true;
    }
    assert_eq!(refused(&r), out);
}

fn set_net(r: &mut Record, to: &str) {
    match r {
        Record::PinGpioDeclared { net, .. }
        | Record::PinGpioWitness { net, .. }
        | Record::PinAdcDeclared { net, .. }
        | Record::PinAdcWitness { net, .. } => *net = to.into(),
        Record::MotorLevelWitness { label, .. } => *label = to.into(),
        _ => unreachable!("no net"),
    }
}

#[test]
fn a_gpio_witness_is_refused_for_each_malformed_field() {
    let mut r = led_witness();
    set_net(&mut r, "");
    assert_eq!(refused(&r), "a GPIO witness's net is malformed");
    for status in ["", "Witnessed", "static_only", "level"] {
        assert_eq!(
            refused(&gpio_witness("LED", status)),
            "a GPIO witness's status is not one the judge gives",
            "{status}"
        );
    }
    for status in GPIO_STATUSES {
        assert_eq!(gpio_witness("LED", status).check(), Ok(()), "{status}");
    }
    assert_eq!(
        refused(&btn_witness(2)),
        "a GPIO witness's level is not 0 or 1"
    );
    // An output's fields and an input's in one witness.
    for (bsrr, ordered) in [(Some(1), None), (None, Some(true)), (Some(1), Some(false))] {
        let r = Record::PinGpioWitness {
            net: "X".into(),
            status: "mismatch".into(),
            bsrr,
            ordered,
            level: Some(1),
        };
        assert_eq!(
            refused(&r),
            "a GPIO witness holds both an output's fields and an input's"
        );
    }
    // Unreadable holds nothing.
    for (bsrr, ordered, level) in [
        (Some(1), None, None),
        (None, Some(true), None),
        (None, None, Some(1)),
    ] {
        let r = Record::PinGpioWitness {
            net: "X".into(),
            status: "unreadable".into(),
            bsrr,
            ordered,
            level,
        };
        assert_eq!(refused(&r), "an unreadable GPIO witness holds a reading");
    }
}

#[test]
fn a_declared_adc_input_is_refused_for_each_malformed_field() {
    let edit = |f: &dyn Fn(&mut Record)| {
        let mut r = adc_decl(false);
        f(&mut r);
        refused(&r)
    };
    assert_eq!(adc_decl(false).check(), Ok(()));
    assert_eq!(adc_decl(true).check(), Ok(()));
    let net = "a declared ADC input's net or pin is malformed";
    assert_eq!(edit(&|r| set_net(r, "")), net);
    assert_eq!(
        edit(&|r| {
            if let Record::PinAdcDeclared { pin, .. } = r {
                *pin = "ADC".into();
            }
        }),
        net
    );
    for peripheral in ["ADC2", "adc1", "ADC", ""] {
        assert_eq!(
            edit(&|r| {
                if let Record::PinAdcDeclared { peripheral: p, .. } = r {
                    *p = peripheral.into();
                }
            }),
            "a declared ADC input's peripheral is not ADC1 or ADC3",
            "{peripheral}"
        );
    }
    let mut ok = adc_decl(false);
    if let Record::PinAdcDeclared { peripheral, .. } = &mut ok {
        *peripheral = "ADC3".into();
    }
    assert_eq!(ok.check(), Ok(()));
    assert_eq!(
        edit(&|r| {
            if let Record::PinAdcDeclared { channel, .. } = r {
                *channel = 20;
            }
        }),
        "a declared ADC input's channel is above 19"
    );
    let mut ok = adc_decl(false);
    if let Record::PinAdcDeclared { channel, .. } = &mut ok {
        *channel = 19;
    }
    assert_eq!(ok.check(), Ok(()));
    for bits in [0, 8, 11, 13, 16] {
        assert_eq!(
            edit(&|r| {
                if let Record::PinAdcDeclared { bits: b, .. } = r {
                    *b = bits;
                }
            }),
            "a declared ADC input's resolution is not 12 bits",
            "{bits}"
        );
    }
    assert_eq!(
        edit(&|r| {
            if let Record::PinAdcDeclared {
                microvolts, code, ..
            } = r
            {
                *microvolts = 3_300_001;
                *code = adc_code(3_300_001, 12);
            }
        }),
        "a declared ADC input's voltage is above the ADC's 3300 mV reference"
    );
    let mut top = adc_decl(false);
    if let Record::PinAdcDeclared {
        microvolts, code, ..
    } = &mut top
    {
        *microvolts = 3_300_000;
        *code = 4095;
    }
    assert_eq!(top.check(), Ok(()));
    for code in [0, 1860, 1862, 4095] {
        assert_eq!(
            edit(&|r| {
                if let Record::PinAdcDeclared { code: c, .. } = r {
                    *c = code;
                }
            }),
            "a declared ADC input's code is not the one its voltage reads as",
            "{code}"
        );
    }
}

#[test]
fn an_adc_witness_is_refused_for_each_malformed_field() {
    assert_eq!(adc_witness("witnessed", 1, Some(1861)).check(), Ok(()));
    assert_eq!(adc_witness("mismatch", 2, Some(7)).check(), Ok(()));
    assert_eq!(adc_witness("unwitnessed", 0, None).check(), Ok(()));
    assert_eq!(adc_witness("unwitnessed", 4, None).check(), Ok(()));
    let mut r = adc_witness("witnessed", 1, Some(1861));
    set_net(&mut r, "");
    assert_eq!(refused(&r), "an ADC witness's net is malformed");
    for status in ["", "unreadable", "Witnessed"] {
        assert_eq!(
            refused(&adc_witness(status, 1, Some(1))),
            "an ADC witness's status is not one the judge gives",
            "{status}"
        );
    }
    for status in ADC_STATUSES {
        let code = (status != "unwitnessed").then_some(1);
        assert_eq!(adc_witness(status, 1, code).check(), Ok(()));
    }
    let code = "an ADC witness has a code exactly when it is not unwitnessed";
    assert_eq!(refused(&adc_witness("unwitnessed", 1, Some(1))), code);
    assert_eq!(refused(&adc_witness("witnessed", 1, None)), code);
    assert_eq!(refused(&adc_witness("mismatch", 1, None)), code);
    assert_eq!(
        refused(&adc_witness("witnessed", 0, Some(1861))),
        "an ADC witness matched a code from no read"
    );
}

type Break<'a> = &'a dyn Fn(&mut Record);

#[test]
fn a_level_witness_is_refused_for_each_malformed_field() {
    for status in LEVEL_STATUSES {
        assert_eq!(level_witness(status).check(), Ok(()), "{status}");
    }
    let mut r = level_witness("witnessed");
    set_net(&mut r, "");
    assert_eq!(refused(&r), "a level witness's label is malformed");
    let edit = |f: &dyn Fn(&mut u8, &mut Option<u8>)| {
        let mut r = level_witness("mismatch");
        if let Record::MotorLevelWitness {
            level, end_level, ..
        } = &mut r
        {
            f(level, end_level);
        }
        refused(&r)
    };
    let levels = "a level witness's level or end level is not 0 or 1";
    assert_eq!(edit(&|l, _| *l = 2), levels);
    assert_eq!(edit(&|_, e| *e = Some(2)), levels);
    let mut r = level_witness("witnessed");
    if let Record::MotorLevelWitness { status, .. } = &mut r {
        *status = "static_only".into();
    }
    assert_eq!(
        refused(&r),
        "a level witness's status is not one the judge gives"
    );
    // `witnessed` exactly when each of its four conditions holds.
    let witnessed = "a level witness is witnessed exactly when it settled, saw no edge after and ended at its level";
    let breaks: [(&str, Break); 4] = [
        ("settle", &|r| {
            if let Record::MotorLevelWitness { settle_ns, .. } = r {
                *settle_ns = None;
            }
        }),
        ("rises", &|r| {
            if let Record::MotorLevelWitness { rises_after, .. } = r {
                *rises_after = 1;
            }
        }),
        ("falls", &|r| {
            if let Record::MotorLevelWitness { falls_after, .. } = r {
                *falls_after = 1;
            }
        }),
        ("end", &|r| {
            if let Record::MotorLevelWitness { end_level, .. } = r {
                *end_level = Some(1);
            }
        }),
    ];
    for (what, f) in breaks {
        let mut r = level_witness("witnessed");
        f(&mut r);
        assert_eq!(refused(&r), witnessed, "{what}");
    }
    let mut r = level_witness("witnessed");
    if let Record::MotorLevelWitness { end_level, .. } = &mut r {
        *end_level = None;
    }
    assert_eq!(refused(&r), witnessed, "no end level");
    // And the other way: every condition holding is `witnessed` alone.
    for status in ["mismatch", "unanchored", "unreadable"] {
        let mut r = level_witness("witnessed");
        if let Record::MotorLevelWitness { status: s, .. } = &mut r {
            *s = status.into();
        }
        assert_eq!(refused(&r), witnessed, "{status}");
    }
    let mut r = level_witness("mismatch");
    if let Record::MotorLevelWitness {
        status, settle_ns, ..
    } = &mut r
    {
        *status = "unanchored".into();
        *settle_ns = Some(1);
    }
    assert_eq!(refused(&r), "an unanchored level witness has a settle time");
    let mut r = level_witness("mismatch");
    if let Record::MotorLevelWitness {
        status, end_level, ..
    } = &mut r
    {
        *status = "unreadable".into();
        *end_level = Some(1);
    }
    assert_eq!(refused(&r), "an unreadable level witness has an end level");
}

#[test]
fn a_scenario_record_refuses_a_malformed_override_id_or_voltage() {
    let with = |o: Override| Record::Scenario {
        name: "pins".into(),
        sha256: [1; 32],
        expect_pass: true,
        expect_findings: vec![],
        overrides: vec![o],
    };
    let held = |net: &str| {
        with(Override::PinHeld {
            net: net.into(),
            level: Level::Low,
        })
    };
    let analog = |net: &str, millivolts| {
        with(Override::AnalogValue {
            net: net.into(),
            millivolts,
        })
    };
    assert_eq!(held("BTN").check(), Ok(()));
    assert_eq!(analog("V", 3300).check(), Ok(()));
    let ids = "an override's id is not 1 to 64 printable ASCII bytes";
    assert_eq!(refused(&held("")), ids);
    assert_eq!(refused(&analog("", 1)), ids);
    assert_eq!(
        refused(&analog("V", 3301)),
        "an analog_value override is above the ADC's 3300 mV reference"
    );
}

#[test]
fn the_motor_statuses_gain_level_alone() {
    assert_eq!(MOTOR_STATUSES.len(), 9);
    assert_eq!(MOTOR_STATUSES[8], "level");
}

// ---- check_run ----

const MOTOR_REFUSED: &str = "the verdict's motor findings do not match the motor records";
const PIN_REFUSED: &str = "the verdict's GPIO and ADC findings do not match the pin device records";
const PERIOD: u64 = 2_499_000;

fn boot(word: Option<u32>) -> Record {
    Record::BootObserved {
        image_sha256: [4; 32],
        boot_word: Reading {
            address: 0x2000_0000,
            mask: u32::MAX,
            expected: BOOT_WORD,
            observed: word,
        },
        instructions_expected: 10_000_000,
        instructions_observed: Some(10_000_000),
        pc: Some(0x0800_0100),
        executable: vec![(0x0800_0000, 0x0801_0000)],
        brown_out: Reading {
            address: 0x2000_0004,
            mask: 0xC,
            expected: 0,
            observed: Some(0),
        },
        flash_access: Some(Vec::new()),
    }
}

/// Kind 24 and 25 of a PWM output idling at a static level, and its kind 26
/// with status `level`: the timing and the window are the ordinary output's
/// (`level` relaxes only what the counts must be).
fn motor_records() -> (Record, Record, Record) {
    (
        Record::MotorOutputSolved {
            label: "M1".into(),
            timer: "TIM1".into(),
            channel: 1,
            pin: "PA8".into(),
            alternate_function: 1,
            protocol: "pwm".into(),
            psc: 63,
            arr: 2499,
            // Idling low (adversarial F1: this fixture idled at a pulse).
            ccr: 0,
            pwm: Some((400, 1000, 2000)),
            idle: Some(aethelgard_chaos::record::MotorIdle::Low),
            resolution_ps: Some(1_000_000),
        },
        Record::MotorTimingCheck {
            label: "M1".into(),
            kernel_hz: 64_000_000,
            period_ps: 2_500_000_000,
            high_ps: 0,
            nominal_period_ps: 2_500_000_000,
            nominal_high_ps: 0,
            holds: true,
        },
        Record::MotorWitness {
            label: "M1".into(),
            observer: "gpioPortA.pa8obs".into(),
            period_ns: PERIOD,
            high_ns: 0,
            grain_ns: 10,
            quantum_ns: 100_000,
            status: "level".into(),
            rises: 0,
            falls: 0,
            first_ns: None,
            last_ns: None,
            end_ns: 100_000_000,
            span_ns: None,
            off_grid: 0,
            batched: 0,
            anchor_ns: Some(6),
            edges_before: 0,
            edges_after: 0,
            cen_rises: 1,
        },
    )
}

// Where each record sits in `base()`.
const SCENARIO: usize = 0;
const LED: usize = 2;
const BTN: usize = 3;
const ADC: usize = 4;
const LED_W: usize = 9;
const BTN_W: usize = 10;
const ADC_W: usize = 11;
const MOTOR_W: usize = 8;
const LEVEL_W: usize = 12;

/// A consistent run: an output `LED` declared low and written low, an input
/// `BTN` held high (its active level low) and read high, an ADC input held
/// at 1.5 V and read as 1861, and a motor output idling low, settled.
fn base() -> Vec<Record> {
    let (solved, check, motor_w) = motor_records();
    vec![
        Record::Scenario {
            name: "pins".into(),
            sha256: [1; 32],
            expect_pass: true,
            expect_findings: vec![],
            overrides: vec![
                Override::PinHeld {
                    net: "BTN".into(),
                    level: Level::High,
                },
                Override::AnalogValue {
                    net: "VSENSE".into(),
                    millivolts: 1500,
                },
            ],
        },
        Record::Solved {
            specification_sha256: [2; 32],
            topology_sha256: [3; 32],
        },
        gpio_out(),
        gpio_in(1, 0, true),
        adc_decl(true),
        solved,
        check,
        boot(Some(BOOT_WORD)),
        motor_w,
        led_witness(),
        btn_witness(1),
        adc_witness("witnessed", 3, Some(1861)),
        level_witness("witnessed"),
        Record::Verdict {
            pass: true,
            findings: vec![],
        },
    ]
}

/// `base()` edited, with the verdict naming `findings` (each an error unless
/// `info`), and the scenario expecting what they fail.
fn run(edit: impl FnOnce(&mut Vec<Record>), findings: &[(&str, &str, u64)]) -> Vec<Record> {
    let mut r = base();
    edit(&mut r);
    let failing: Vec<String> = findings
        .iter()
        .filter(|f| f.1 != "info")
        .map(|f| f.0.to_owned())
        .collect();
    let pass = failing.is_empty();
    if let Record::Scenario {
        expect_pass,
        expect_findings,
        ..
    } = &mut r[SCENARIO]
    {
        *expect_pass = pass;
        *expect_findings = failing;
        expect_findings.sort();
    }
    let last = r.len() - 1;
    r[last] = Record::Verdict {
        pass,
        findings: findings
            .iter()
            .map(|(k, s, n)| ((*k).to_owned(), (*s).to_owned(), *n))
            .collect(),
    };
    r
}

fn judged(edit: impl FnOnce(&mut Vec<Record>)) -> Result<(), &'static str> {
    check_run(&run(edit, &[])).map(|_| ())
}

#[test]
fn a_consistent_pin_device_run_passes() {
    let s = check_run(&base()).unwrap();
    assert!(s.pass && s.matched, "{s:?}");
    // The records the run holds are each valid on their own.
    for r in base() {
        assert_eq!(r.check(), Ok(()), "{r:?}");
    }
}

#[test]
fn the_records_are_ranked_before_and_after_the_boot() {
    // A declaration after the boot, or a witness before it, is out of order.
    let mut r = base();
    let d = r.remove(LED);
    r.insert(7, d);
    assert_eq!(
        check_run(&r).unwrap_err(),
        "the records are not in the campaign's order"
    );
    let mut r = base();
    let w = r.remove(LED_W);
    r.insert(1, w);
    assert_eq!(
        check_run(&r).unwrap_err(),
        "the records are not in the campaign's order"
    );
    // The declarations sit with the solved ones, in any order among them.
    let mut r = base();
    r.swap(LED, ADC);
    assert!(check_run(&r).is_ok());
    let mut r = base();
    r.swap(LED_W, LEVEL_W);
    assert!(check_run(&r).is_ok());
}

/// Each finding: how a witness is made to give it, and the count the
/// verdict must name. One too few and one too many are refused, and so is
/// the finding named as `info`.
type Cause = fn(&mut Vec<Record>);

fn finding_cases() -> Vec<(&'static str, &'static str, Cause)> {
    vec![
        (GPIO_INPUT_ASSERTED_FINDING, PIN_REFUSED, |r| {
            // Active high, held high, read high.
            r[BTN] = gpio_in(1, 1, true);
        }),
        (GPIO_UNWITNESSED_FINDING, PIN_REFUSED, |r| {
            r[BTN_W] = gpio_witness("BTN", "unwitnessed");
        }),
        (GPIO_UNWITNESSED_FINDING, PIN_REFUSED, |r| {
            r[LED_W] = gpio_witness("LED", "unreadable");
        }),
        (GPIO_MISMATCH_FINDING, PIN_REFUSED, |r| {
            r[BTN_W] = Record::PinGpioWitness {
                net: "BTN".into(),
                status: "mismatch".into(),
                bsrr: None,
                ordered: None,
                level: Some(0),
            };
        }),
        (GPIO_MISMATCH_FINDING, PIN_REFUSED, |r| {
            r[LED_W] = Record::PinGpioWitness {
                net: "LED".into(),
                status: "mismatch".into(),
                bsrr: Some(1 << 5),
                ordered: Some(false),
                level: None,
            };
        }),
        (ADC_UNWITNESSED_FINDING, PIN_REFUSED, |r| {
            r[ADC_W] = adc_witness("unwitnessed", 0, None);
        }),
        (ADC_MISMATCH_FINDING, PIN_REFUSED, |r| {
            r[ADC_W] = adc_witness("mismatch", 2, Some(1000));
        }),
        (MOTOR_LEVEL_FINDING, MOTOR_REFUSED, |r| {
            r[LEVEL_W] = level_witness("mismatch");
        }),
        (MOTOR_LEVEL_FINDING, MOTOR_REFUSED, |r| {
            // Unanchored: kind 26 observed no CEN rise either (F1's join).
            if let Record::MotorWitness {
                anchor_ns,
                cen_rises,
                ..
            } = &mut r[MOTOR_W]
            {
                (*anchor_ns, *cen_rises) = (None, 0);
            }
            r[LEVEL_W] = level_witness("unanchored");
        }),
        (MOTOR_LEVEL_FINDING, MOTOR_REFUSED, |r| {
            r[LEVEL_W] = level_witness("unreadable");
        }),
    ]
}

#[test]
fn each_finding_is_named_exactly_as_often_as_the_records_give_it() {
    for (i, (kind, refusal, cause)) in finding_cases().into_iter().enumerate() {
        let right = run(cause, &[(kind, "error", 1)]);
        assert!(check_run(&right).unwrap().matched, "{i} {kind}");
        // One too few, one too many, another kind, and the right one as
        // `info`: each refused.
        for wrong in [
            vec![],
            vec![(kind, "error", 2)],
            vec![(kind, "info", 1)],
            vec![(kind, "error", 1), (kind, "info", 1)],
            vec![("gpio_unwitnessed_x", "error", 1)],
        ] {
            let e = check_run(&run(cause, &wrong)).unwrap_err();
            assert!(
                e == refusal
                    || e == "a motor finding has the wrong severity"
                    || e == "the verdict's GPIO and ADC findings do not match the pin device records",
                "{i} {kind} {wrong:?}: {e}"
            );
            assert_ne!(
                check_run(&run(cause, &wrong)).map(|s| s.matched),
                Ok(true),
                "{i} {kind} {wrong:?}"
            );
        }
        // The verdict names the finding with no record that gives it.
        assert_eq!(
            check_run(&run(|_| {}, &[(kind, "error", 1)])).unwrap_err(),
            refusal,
            "{i} {kind}: a finding with no record"
        );
    }
}

#[test]
fn a_status_the_judge_does_not_count_gives_no_finding() {
    // A mismatched output's witness is `mismatch` and gives gpio_mismatch;
    // the same output unwitnessed gives gpio_unwitnessed, never both.
    let both = run(
        |r| r[LED_W] = gpio_witness("LED", "unwitnessed"),
        &[
            (GPIO_UNWITNESSED_FINDING, "error", 1),
            (GPIO_MISMATCH_FINDING, "error", 1),
        ],
    );
    assert_eq!(check_run(&both).unwrap_err(), PIN_REFUSED);
    // An input read at its inactive level is no assertion.
    let inactive = run(|r| r[BTN] = gpio_in(1, 0, true), &[]);
    assert!(check_run(&inactive).is_ok());
    // Nor is an output's `witnessed` one, whatever its declared active.
    let asserted_output = run(|_| {}, &[(GPIO_INPUT_ASSERTED_FINDING, "error", 1)]);
    assert_eq!(check_run(&asserted_output).unwrap_err(), PIN_REFUSED);
}

#[test]
fn a_pin_witness_is_judged_only_once_the_boot_word_is_set() {
    let unbooted = |findings: &[(&str, &str, u64)]| {
        run(
            |r| {
                r[7] = boot(Some(0));
                // The motor outputs' kind 26 unwitnessed before the boot word.
                r[LED_W] = gpio_witness("LED", "unwitnessed");
                // A mismatch reads a level other than the driven one (F6).
                r[BTN_W] = Record::PinGpioWitness {
                    net: "BTN".into(),
                    status: "mismatch".into(),
                    bsrr: None,
                    ordered: None,
                    level: Some(0),
                };
                r[ADC_W] = adc_witness("unwitnessed", 0, None);
                r[LEVEL_W] = level_witness("mismatch");
            },
            findings,
        )
    };
    let none = [("boot_word_missing", "error", 1)];
    assert!(check_run(&unbooted(&none)).is_ok());
    for kind in [
        GPIO_UNWITNESSED_FINDING,
        GPIO_MISMATCH_FINDING,
        ADC_UNWITNESSED_FINDING,
    ] {
        let named = [("boot_word_missing", "error", 1), (kind, "error", 1)];
        assert_eq!(
            check_run(&unbooted(&named)).unwrap_err(),
            PIN_REFUSED,
            "{kind}"
        );
    }
    let named = [
        ("boot_word_missing", "error", 1),
        (MOTOR_LEVEL_FINDING, "error", 1),
    ];
    assert_eq!(check_run(&unbooted(&named)).unwrap_err(), MOTOR_REFUSED);
    // Before the boot word, witnesses need not be there at all.
    let mut r = unbooted(&none);
    r.retain(|x| {
        !matches!(
            x,
            Record::PinGpioWitness { .. } | Record::PinAdcWitness { .. }
        )
    });
    assert!(check_run(&r).is_ok());
}

#[test]
fn a_declaration_without_its_witness_is_refused_once_the_boot_word_is_set() {
    let gone = "a declared GPIO signal or ADC input has no witness, though the boot word is set";
    assert_eq!(
        judged(|r| {
            r.remove(LED_W);
        }),
        Err(gone)
    );
    assert_eq!(
        judged(|r| {
            r.remove(ADC_W);
        }),
        Err(gone)
    );
    assert_eq!(
        judged(|r| {
            r.remove(BTN_W);
        }),
        Err(gone)
    );
}

#[test]
fn each_witness_joins_one_declaration_by_net() {
    assert_eq!(judged(|_| {}), Ok(()));
    assert_eq!(
        judged(|r| set_net(&mut r[LED_W], "NOPE")),
        Err("a GPIO witness names no declared GPIO signal")
    );
    assert_eq!(
        judged(|r| set_net(&mut r[ADC_W], "NOPE")),
        Err("an ADC witness names no declared ADC input")
    );
    // A witness with no declaration at all.
    assert_eq!(
        judged(|r| {
            r.remove(LED);
        }),
        Err("a GPIO witness names no declared GPIO signal")
    );
    // Two witnesses of one net, two declarations of one net.
    assert_eq!(
        judged(|r| r.insert(LED_W, led_witness())),
        Err("a GPIO signal has two witnesses")
    );
    assert_eq!(
        judged(|r| r.insert(ADC_W, adc_witness("witnessed", 3, Some(1861)))),
        Err("an ADC input has two witnesses")
    );
    assert_eq!(
        judged(|r| r.insert(LED, gpio_out())),
        Err("a GPIO signal is declared twice")
    );
    assert_eq!(
        judged(|r| r.insert(ADC, adc_decl(true))),
        Err("an ADC input is declared twice")
    );
}

#[test]
fn a_witness_is_shaped_as_its_signal_s_direction() {
    let shape = "a GPIO witness is not shaped as its signal's direction";
    // An input's witness with an output's fields, and an output's with a level.
    assert_eq!(
        judged(|r| {
            r[BTN_W] = Record::PinGpioWitness {
                net: "BTN".into(),
                status: "mismatch".into(),
                bsrr: Some(1),
                ordered: Some(true),
                level: None,
            };
        }),
        Err(shape)
    );
    assert_eq!(
        judged(|r| {
            r[LED_W] = Record::PinGpioWitness {
                net: "LED".into(),
                status: "mismatch".into(),
                bsrr: None,
                ordered: None,
                level: Some(0),
            };
        }),
        Err(shape)
    );
}

#[test]
fn a_witnessed_gpio_signal_is_what_its_declaration_says() {
    let not_so = "a witnessed GPIO signal is not what its declaration says: an output's first BSRR word before its MODER write setting its declared level, an input's level its declared one";
    let out = |bsrr: Option<u32>, ordered: Option<bool>| {
        judged(|r| {
            r[LED_W] = Record::PinGpioWitness {
                net: "LED".into(),
                status: "witnessed".into(),
                bsrr,
                ordered,
                level: None,
            };
        })
    };
    // Declared low on PA5: the reset bit, bit 21, alone of the pin's two.
    assert_eq!(out(Some(1 << 21), Some(true)), Ok(()));
    // Other pins' bits in the word are no matter.
    assert_eq!(out(Some((1 << 21) | 1 | (1 << 30)), Some(true)), Ok(()));
    for (bsrr, ordered) in [
        (Some(1 << 5), Some(true)),
        (Some((1 << 5) | (1 << 21)), Some(true)),
        (Some(1 << 4), Some(true)),
        (Some(1 << 22), Some(true)),
        (Some(0), Some(true)),
        (None, Some(true)),
        (Some(1 << 21), Some(false)),
        (Some(1 << 21), None),
    ] {
        assert_eq!(out(bsrr, ordered), Err(not_so), "{bsrr:?} {ordered:?}");
    }
    // Declared high: the set bit.
    let high = |bsrr: u32| {
        judged(|r| {
            r[LED] = Record::PinGpioDeclared {
                net: "LED".into(),
                pin: "PA5".into(),
                output: true,
                level: 1,
                active: None,
                held: false,
            };
            r[LED_W] = Record::PinGpioWitness {
                net: "LED".into(),
                status: "witnessed".into(),
                bsrr: Some(bsrr),
                ordered: Some(true),
                level: None,
            };
        })
    };
    assert_eq!(high(1 << 5), Ok(()));
    assert_eq!(high(1 << 21), Err(not_so));
    assert_eq!(high((1 << 5) | (1 << 21)), Err(not_so));
    // The pin's own number: PA5 is not PA0.
    assert_eq!(high(1), Err(not_so));
    // An input: its level is its declared one.
    assert_eq!(judged(|r| r[BTN_W] = btn_witness(0)), Err(not_so));
    assert_eq!(
        judged(|r| r[BTN_W] = gpio_witness("BTN", "witnessed")),
        Err(not_so)
    );
}

#[test]
fn a_witnessed_adc_input_read_the_code_its_declaration_gives() {
    let wrong = "a witnessed ADC input's code is not the one its declaration gives";
    assert_eq!(
        judged(|r| r[ADC_W] = adc_witness("witnessed", 3, Some(1860))),
        Err(wrong)
    );
    assert_eq!(
        judged(|r| r[ADC_W] = adc_witness("witnessed", 3, None)),
        Err(wrong)
    );
    // `mismatch` and `unwitnessed` carry no such claim: their findings.
    assert!(
        check_run(&run(
            |r| r[ADC_W] = adc_witness("mismatch", 1, Some(1860)),
            &[(ADC_MISMATCH_FINDING, "error", 1)]
        ))
        .is_ok()
    );
}

fn set_overrides(r: &mut [Record], o: Vec<Override>) {
    if let Record::Scenario { overrides, .. } = &mut r[SCENARIO] {
        *overrides = o;
    }
}

fn held_at(net: &str, level: Level) -> Override {
    Override::PinHeld {
        net: net.into(),
        level,
    }
}

fn analog_at(net: &str, millivolts: u32) -> Override {
    Override::AnalogValue {
        net: net.into(),
        millivolts,
    }
}

#[test]
fn each_pin_held_override_is_a_held_input_at_its_level() {
    let analog = analog_at("VSENSE", 1500);
    assert_eq!(
        judged(|r| set_overrides(r, vec![held_at("NOPE", Level::High), analog.clone()])),
        Err("a pin_held override names no declared GPIO signal")
    );
    // A pin_held on an output names `PinHeldIsOutput`.
    let e = judged(|r| set_overrides(r, vec![held_at("LED", Level::High), analog.clone()]))
        .unwrap_err();
    assert!(e.contains("PinHeldIsOutput"), "{e}");
    // The declaration's level is not the override's, or it is not held.
    let level = "a pin_held override is not held at its level by its signal's declaration";
    assert_eq!(
        judged(|r| set_overrides(r, vec![held_at("BTN", Level::Low), analog.clone()])),
        Err(level)
    );
    assert_eq!(judged(|r| r[BTN] = gpio_in(1, 0, false)), Err(level));
    // A held declaration with no override, and an override twice.
    assert_eq!(
        judged(|r| set_overrides(r, vec![analog.clone()])),
        Err("a held GPIO signal has no pin_held override")
    );
    assert_eq!(
        judged(|r| set_overrides(
            r,
            vec![
                held_at("BTN", Level::High),
                held_at("BTN", Level::High),
                analog.clone()
            ]
        )),
        Err("a scenario holds one net twice")
    );
    // Held low, declared low (its active level high): the other level agrees.
    assert_eq!(
        judged(|r| {
            set_overrides(r, vec![held_at("BTN", Level::Low), analog.clone()]);
            r[BTN] = gpio_in(0, 1, true);
            r[BTN_W] = btn_witness(0);
        }),
        Ok(())
    );
}

#[test]
fn each_analog_value_override_is_an_adc_input_held_at_its_voltage() {
    let held = held_at("BTN", Level::High);
    let no_adc = "an analog_value override names no ADC input held at its voltage";
    for o in [analog_at("NOPE", 1500), analog_at("VSENSE", 1400)] {
        assert_eq!(
            judged(|r| set_overrides(r, vec![held.clone(), o.clone()])),
            Err(no_adc),
            "{o:?}"
        );
    }
    assert_eq!(judged(|r| r[ADC] = adc_decl(false)), Err(no_adc));
    assert_eq!(
        judged(|r| set_overrides(r, vec![held.clone()])),
        Err("a held ADC input has no analog_value override")
    );
    assert_eq!(
        judged(|r| set_overrides(
            r,
            vec![
                held.clone(),
                analog_at("VSENSE", 1500),
                analog_at("VSENSE", 1500)
            ]
        )),
        Err("a scenario sets one analog net twice")
    );
    // An ADC input driven at its declared pull needs no override.
    let mut r = base();
    r[ADC] = adc_decl(false);
    set_overrides(&mut r, vec![held]);
    assert!(check_run(&r).is_ok());
}

#[test]
fn a_motor_witness_judged_by_level_has_its_level_witness_and_back() {
    let missing = "a motor witness judged by level has no level witness";
    assert_eq!(
        judged(|r| {
            r.remove(LEVEL_W);
        }),
        Err(missing)
    );
    let unjoined = "a level witness names no kind 26 judged by level, or one twice";
    assert_eq!(judged(|r| set_net(&mut r[LEVEL_W], "M9")), Err(unjoined));
    assert_eq!(
        judged(|r| r.insert(LEVEL_W, level_witness("witnessed"))),
        Err(unjoined)
    );
    // A kind 37 for an output judged by another status.
    let other = run(
        |r| {
            if let Record::MotorWitness {
                status,
                rises,
                falls,
                first_ns,
                last_ns,
                span_ns,
                off_grid,
                ..
            } = &mut r[MOTOR_W]
            {
                *status = "unwitnessed".into();
                let _ = (rises, falls, first_ns, last_ns, span_ns, off_grid);
            }
        },
        &[("motor_output_unwitnessed", "error", 1)],
    );
    assert_eq!(check_run(&other).unwrap_err(), unjoined);
}

#[test]
fn level_relaxes_only_the_counts_of_a_motor_witness() {
    // Any counts, so long as they are consistent with each other.
    let busy = |r: &mut Vec<Record>| {
        if let Record::MotorWitness {
            rises,
            falls,
            first_ns,
            last_ns,
            span_ns,
            off_grid,
            ..
        } = &mut r[MOTOR_W]
        {
            (*rises, *falls) = (5, 2);
            (*first_ns, *last_ns, *span_ns) = (Some(10), Some(20), Some(5));
            *off_grid = 7;
        }
    };
    // Witnessed beside edges past an idle low's: refused (adversarial F1).
    assert_eq!(
        judged(busy),
        Err(
            "a witnessed level's motor witness shows an edge past its idle's, or its window holds no period after the settled instant"
        )
    );
    // The timing, the grain and the window still bind it.
    let timing = "a motor witness's status is not what its counts and its output's timing give";
    assert_eq!(
        judged(|r| {
            busy(r);
            if let Record::MotorWitness { period_ns, .. } = &mut r[MOTOR_W] {
                *period_ns += 1;
            }
        }),
        Err(timing)
    );
    assert_eq!(
        judged(|r| {
            if let Record::MotorWitness { grain_ns, .. } = &mut r[MOTOR_W] {
                *grain_ns += 1;
            }
        }),
        Err(timing)
    );
    assert_eq!(
        judged(|r| {
            if let Record::MotorWitness { end_ns, .. } = &mut r[MOTOR_W] {
                *end_ns += 1;
            }
        }),
        Err("a motor witness's window is not its anchor's, or its first rise is before it")
    );
    // Edges out of order are no counts at all.
    assert_eq!(
        judged(|r| {
            busy(r);
            if let Record::MotorWitness {
                first_ns, last_ns, ..
            } = &mut r[MOTOR_W]
            {
                (*first_ns, *last_ns) = (Some(30), Some(20));
            }
        }),
        Err(timing)
    );
}

#[test]
fn an_archived_run_with_a_pin_device_is_judged_by_its_records_alone() {
    let events: Vec<Event> = base()
        .into_iter()
        .enumerate()
        .map(|(i, record)| Event::Record {
            timestamp: i as u64,
            record,
        })
        .collect();
    let (dir, _) = write_run("pins-archived", 11, &events);
    let state = replay_dir(&dir).unwrap().state;
    assert!(check_run(&state.records).unwrap().matched);
}

// ---- AET-WO-1211c adversarial pass: each forgery refused by name.

fn kind26(r: &mut [Record]) -> &mut Record {
    &mut r[MOTOR_W]
}

/// F1: kind 26 says `level` and kind 37 `witnessed` on an output kind 24
/// says idles at a pulse (a live PWM): refused.
#[test]
fn a_level_witness_on_a_pulse_idle_output_is_refused() {
    let r = run(
        |r| {
            if let Record::MotorOutputSolved { idle, ccr, .. } = &mut r[5] {
                (*idle, *ccr) = (
                    Some(aethelgard_chaos::record::MotorIdle::PulseUs(1000)),
                    1000,
                );
            }
            // Kinds 25 and 26 consistent with that pulse, so only the level
            // witness can be refused.
            for rec in r.iter_mut() {
                match rec {
                    Record::MotorTimingCheck {
                        high_ps,
                        nominal_high_ps,
                        ..
                    } => (*high_ps, *nominal_high_ps) = (1_000_000_000, 1_000_000_000),
                    Record::MotorWitness { high_ns, .. } => *high_ns = 1_000_000,
                    _ => {}
                }
            }
        },
        &[],
    );
    assert_eq!(
        check_run(&r).unwrap_err(),
        "a level witness's output does not idle at a static level"
    );
}

/// F1: a failing PWM witness (38 rises to 95 ms) relabelled `level` beside a
/// witnessed kind 37 with its finding dropped: refused, not a forged pass.
#[test]
fn a_live_pwm_forged_into_a_level_pass_is_refused() {
    let forged = run(
        |r| {
            if let Record::MotorWitness {
                rises,
                falls,
                first_ns,
                last_ns,
                span_ns,
                ..
            } = kind26(r)
            {
                (*rises, *falls) = (38, 38);
                (*first_ns, *last_ns, *span_ns) = (Some(3_000_000), Some(95_000_000), Some(5));
            }
        },
        &[],
    );
    assert!(check_run(&forged).is_err());
    // Written as an archive and read back by the strict reader: the same.
    let events: Vec<Event> = forged
        .into_iter()
        .enumerate()
        .map(|(i, record)| Event::Record {
            timestamp: i as u64,
            record,
        })
        .collect();
    let (dir, _) = write_run("forged-level", 12, &events);
    assert!(check_run(&replay_dir(&dir).unwrap().state.records).is_err());
}

/// F1: kind 37's level is not kind 24's idle, or its settled instant is not
/// kind 26's anchor's: each refused by its own name.
#[test]
fn a_level_witness_s_level_and_settled_instant_are_its_output_s() {
    let with = |level: u8, settle: Option<u64>| {
        run(
            |r| {
                if let Record::MotorLevelWitness {
                    level: l,
                    end_level,
                    settle_ns,
                    ..
                } = &mut r[LEVEL_W]
                {
                    (*l, *end_level, *settle_ns) = (level, Some(level), settle);
                }
            },
            &[],
        )
    };
    assert_eq!(
        check_run(&with(1, Some(6))).unwrap_err(),
        "a level witness's level is not its output's declared idle"
    );
    assert_eq!(
        check_run(&with(0, Some(99_999_999))).unwrap_err(),
        "a level witness's settled instant is not its anchor's, plus a period and a quantum when high"
    );
    // F9: a window holding less than one period after the settled instant.
    let short = run(
        |r| {
            if let Record::MotorWitness { end_ns, .. } = kind26(r) {
                *end_ns = 6 + PERIOD - 1;
            }
        },
        &[],
    );
    assert!(check_run(&short).is_err());
}

/// F5: a held input, an output and an ADC input on a motor output's pin, or
/// on one another's: refused.
#[test]
fn a_pin_device_s_pin_is_its_own() {
    let on = |k: usize, at: &str| {
        run(
            |r| match &mut r[k] {
                Record::PinGpioDeclared { pin, .. } | Record::PinAdcDeclared { pin, .. } => {
                    *pin = at.into()
                }
                _ => unreachable!(),
            },
            &[],
        )
    };
    let why = "a pin device's pin is another signal's or a peripheral's";
    assert_eq!(check_run(&on(BTN, "PA8")).unwrap_err(), why, "M1's pin");
    let led = match &base()[LED] {
        Record::PinGpioDeclared { pin, .. } => pin.clone(),
        _ => unreachable!(),
    };
    assert_eq!(check_run(&on(ADC, &led)).unwrap_err(), why, "LED's pin");
}

/// F6: a mismatch whose reading is the declaration's is a forged failure:
/// refused for an ADC input and for an input.
#[test]
fn a_mismatch_reads_other_than_its_declaration() {
    let adc = run(
        |r| r[ADC_W] = adc_witness("mismatch", 3, Some(1861)),
        &[("adc_value_mismatch", "error", 1)],
    );
    assert_eq!(
        check_run(&adc).unwrap_err(),
        "an ADC input's mismatch reads no code, or the declared one"
    );
    let input = run(
        |r| {
            if let Record::PinGpioWitness { status, .. } = &mut r[BTN_W] {
                *status = "mismatch".into();
            }
        },
        &[("gpio_mismatch", "error", 1)],
    );
    assert_eq!(
        check_run(&input).unwrap_err(),
        "an input's mismatch reads no level, or the driven one"
    );
}

/// `AET-WO-2309` (ruled D3): a boot that read nothing holds none of what only
/// a boot Renode read gives. Unread whole and naming its bound, the run
/// replays without them; any one kind of 26, 34, 36 and 37 put back beside
/// it is refused by name.
#[test]
fn a_boot_that_read_nothing_holds_no_pin_motor_or_level_witness() {
    let killed = |keep: Option<u64>| {
        let mut r = base();
        unread_boot(&mut r);
        r.retain(|x| {
            !matches!(
                x,
                Record::MotorWitness { .. }
                    | Record::PinGpioWitness { .. }
                    | Record::PinAdcWitness { .. }
                    | Record::MotorLevelWitness { .. }
            ) || Some(x.kind()) == keep
        });
        let last = r.len() - 1;
        r[last] = Record::Verdict {
            pass: false,
            findings: vec![("cpu_aborted".into(), "error".into(), 1)],
        };
        check_run(&r).map(|_| ())
    };
    assert_eq!(killed(None), Ok(()));
    for kind in [26, 34, 36, 37] {
        assert_eq!(killed(Some(kind)), Err(RENODE_READ), "kind {kind}");
    }
}
