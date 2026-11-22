//! `AET-WO-1211b`: a pin device generated. D1 (`AET-WO-1211a`'s fixture: a
//! DRV8876 in PH/EN, nSLEEP, nFAULT and IPROPI) and variants of its solved
//! topology: the bridge's enable after its timer's `CEN`, woken only with its
//! row's `t_wake` waited; an idle the truth table marks as driving refused;
//! the ADC brought up at the declared resolution, and the resolutions no code
//! of both silicon revisions means refused by name.

use std::collections::BTreeMap;

use aethelgard_ir::motor::{Idle, Level};
use aethelgard_ir::{PinTable, SolvedTopology, Specification};
use aethelgard_svd_hal::board::generate;
use aethelgard_svd_hal::pin_table;

fn d1() -> (SolvedTopology, PinTable) {
    let path = format!(
        "{}/../aethelgard-ir/tests/fixtures/d1-drv8876.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let spec: Specification = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let table = pin_table(&spec.mcu.part).unwrap();
    (
        aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap(),
        table,
    )
}

fn files(t: &SolvedTopology, table: &PinTable) -> BTreeMap<String, String> {
    generate("d1-board", t, table).unwrap()
}

fn line_of(text: &str, needle: &str) -> usize {
    text.lines()
        .position(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("no `{needle}`"))
}

/// The enable (nSLEEP, PA1, declared `low`) is configured after TIM1, its
/// bridge's timer, starts, taking TIM1's `Running` token; it does not wake
/// the part, so no wait follows. The proof's image refuses any write to it
/// before TIM1's `CEN`.
#[test]
fn d1_s_enable_follows_its_bridge_s_cen_and_does_not_wake() {
    let (t, table) = d1();
    let f = files(&t, &table);
    let main = &f["firmware/src/main.rs"];
    let start = line_of(main, "let _tim1 = tim1.start_outputs(");
    let enable = line_of(main, "let _pa1 = gpioa.pa1.into_output(&mut r, &_tim1);");
    let magic = line_of(main, "write_volatile(BOOT_MAGIC)");
    assert!(start < enable && enable < magic, "{main}");
    // Configured once, there: never also with the pins, before any timer.
    assert_eq!(main.matches("gpioa.pa1.into_output(").count(), 1, "{main}");
    assert!(!main.contains("settle("), "nSLEEP low wakes nothing");
    assert!(f["src/board.rs"].contains(
        "pub fn into_output<R: Registers>(self, r: &mut R, _cen: &Tim1<Running>) -> PA1<Output> {"
    ));
    assert!(
        f["src/proofs.rs"]
            .contains("PA1, a bridge's enable, written before its inputs sat at their idle")
    );
}

/// Declared `high`, nSLEEP wakes the DRV8876: the row's t_wake (1 ms,
/// SLVSDS7B p. 5) is waited after the enable, before the boot word. The idle
/// (sleep off, EN 0) is brake, a non-driving row (SLVSDS7B Table 3 p. 11).
#[test]
fn a_waking_enable_waits_its_row_s_t_wake() {
    let (mut t, table) = d1();
    t.gpio.get_mut("DRV_NSLEEP").unwrap().initial = Some(Level::High);
    t.validate().unwrap();
    let f = files(&t, &table);
    let main = &f["firmware/src/main.rs"];
    let enable = line_of(main, "let _pa1 = gpioa.pa1.into_output(&mut r, &_tim1);");
    let wait = line_of(main, "    settle(");
    assert_eq!(wait, enable + 2, "{main}");
    assert!(main.contains("its row's t_wake, 1000 us"), "{main}");
}

/// An idle that drives: EN idles high (CCR ARR + 1) with nSLEEP awake, PH/EN
/// forward or reverse, a driving row: refused by name at generate.
#[test]
fn an_idle_the_truth_table_marks_as_driving_is_refused() {
    let (mut t, table) = d1();
    t.gpio.get_mut("DRV_NSLEEP").unwrap().initial = Some(Level::High);
    let m = t.motor_outputs.get_mut("DRV_EN").unwrap();
    m.pwm.as_mut().unwrap().idle = Idle::Level(Level::High);
    m.ccr = m.arr + 1;
    let e = generate("d1-board", &t, &table).unwrap_err();
    assert_eq!(e.kind(), "BridgeIdleNotSafe", "{e}");
    assert!(e.to_string().contains("driving"), "{e}");
}

/// D1's ADC3 is brought up at its declared 12 bits (`RES` `010`), on
/// channel 5 alone; 8 bits, which no code of both revisions means, is
/// refused by name, and so is a topology that carries no resolution.
#[test]
fn the_adc_takes_the_declared_resolution_or_is_refused_by_name() {
    let (t, table) = d1();
    let lib = &files(&t, &table)["src/board.rs"];
    assert!(
        lib.contains("0x80000008); // CFGR"),
        "RES 010 at bit 2, JQDIS"
    );
    assert!(lib.contains("0x00000020); // PCSEL"));
    assert!(lib.contains("0x00000140); // SQR1"));
    // The bounds as ruled (Q-B): twice DS12117's figures at 8 core cycles per
    // ADC cycle, and tADCVREG_STUP's 10 us at HSI's fastest.
    for bound in [
        "regulator: 650,",
        "calibration: 2640160,",
        "ready: 13104,",
        "conversion: 13108,",
    ] {
        assert!(lib.contains(bound), "{bound}");
    }
    // A pin device's input with no solved resolution is now refused by
    // `SolvedTopology::validate` first (`AET-WO-1211c` D7: its `transfer`
    // names a device, which requires its `bits`), before the generator's own
    // `AdcResolutionNotSolved` is reached.
    for (bits, kind) in [
        (Some(8), "AdcResolutionNotCommon"),
        (None, "InvalidTopology"),
    ] {
        let mut t = t.clone();
        t.adc_inputs.get_mut("DRV_IPROPI").unwrap().bits = bits;
        let e = generate("d1-board", &t, &table).unwrap_err();
        assert_eq!(e.kind(), kind, "{e}");
    }
}

/// The adversarial pass's F2: PH as a GPIO output (`1211` Q7, ruled
/// allowed) beside EN on TIM1 generates; PH joins TIM1's proof part, so the
/// enable's rule there reads both its inputs.
#[test]
fn a_bridge_with_a_gpio_input_generates_in_its_timer_s_part() {
    let path = format!(
        "{}/../aethelgard-ir/tests/fixtures/d1-drv8876.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let motors = v["motor_outputs"].as_array_mut().unwrap();
    motors.retain(|m| m["label"] != "DRV_PH");
    let device = &mut v["pin_devices"][0];
    device["signals"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "device_pin": "PH/IN2",
            "net": "DRV_PH",
            "pin": null,
            "direction": "output",
            "initial": "low"
        }));
    let spec: Specification = serde_json::from_value(v).unwrap();
    let table = pin_table(&spec.mcu.part).unwrap();
    let t = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    let ph = t.gpio["DRV_PH"].pin.to_ascii_lowercase();
    let f = files(&t, &table);
    let main = &f["firmware/src/main.rs"];
    assert!(
        main.contains(&format!(
            "let _pa1 = gpioa.pa1.into_output(&mut r, &_tim1, &_{ph});"
        )),
        "{main}"
    );
    let proofs = &f["src/proofs.rs"];
    let part = &proofs[proofs.find("fn every_valid_sequence_tim1").unwrap()..];
    assert!(
        part.contains(&format!("{ph}.take().unwrap()")),
        "{ph} is in TIM1's part"
    );
}

/// The adversarial pass's F3: the proof's final state judges the bridge's
/// idle against its part's truth table (PH/EN: asleep, or EN 0, brake;
/// SLVSDS7B Table 3 p. 11), the rows the oracle reads from `pin_parts`.
#[test]
fn the_proof_judges_the_bridge_s_idle_against_the_truth_table() {
    let (t, table) = d1();
    let proofs = &files(&t, &table)["src/proofs.rs"];
    assert!(proofs.contains(
        "kani::assert(matches!((en, i0, i1), (0, _, _) | (1, 0, _)), \"`U2`'s `M1` idles in a driving or unlisted state of its truth table\");"
    ));
}

/// The adversarial pass's F1: four inputs on one ADC (the sequence's most)
/// need an unwind past the conversions' loop.
#[test]
fn four_adc_inputs_unwind_past_their_conversions() {
    use aethelgard_ir::pin_devices::SolvedAdcInput;
    let (mut t, table) = d1();
    for (net, pin, ch) in [
        ("A2", "PF4", "INP9"),
        ("A3", "PF5", "INP4"),
        ("A4", "PF6", "INP8"),
    ] {
        t.adc_inputs.insert(
            net.into(),
            SolvedAdcInput {
                pin: pin.into(),
                peripheral: "ADC3".into(),
                channel: ch.into(),
                bits: Some(12),
                transfer: Some(aethelgard_ir::pin_devices::SolvedTransfer::CurrentOutput {
                    device: "U2".into(),
                }),
            },
        );
    }
    t.validate().unwrap();
    let proofs = &files(&t, &table)["src/proofs.rs"];
    let h = &proofs[..proofs
        .find("fn adc3_bring_up_is_in_order_and_bounded")
        .unwrap()];
    assert!(
        h.ends_with("#[kani::unwind(6)]\n"),
        "{}",
        &h[h.len() - 60..]
    );
}
