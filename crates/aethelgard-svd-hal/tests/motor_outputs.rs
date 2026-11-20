//! `AET-WO-4300a`, the board crate's motor outputs: the two fixtures
//! (Sentinel-01 plus four outputs on TIM1 CH1..CH4, PA8..PA11 at AF1) generate,
//! and their board crate and firmware carry exactly the start the work order
//! fixes, in order, behind the reset-clock check.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use aethelgard_ir::{SolvedTopology, Specification};
use aethelgard_svd_hal::board::{MOTOR_OUTPUT, SpeedCheck, generate_with, pin_speeds};
use aethelgard_svd_hal::pin_table;
use sha2::{Digest, Sha256};

fn fixture(name: &str) -> (SolvedTopology, aethelgard_ir::PinTable) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../aethelgard-cli/tests/fixtures/{name}.json"));
    let bytes = fs::read(path).unwrap();
    let spec: Specification = serde_json::from_slice(&bytes).unwrap();
    let sha: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let table = pin_table(&spec.mcu.part).unwrap();
    (aethelgard_ir::solve(&spec, &table, &sha).unwrap(), table)
}

/// What the CLI passes: the twin's model table.
fn answers(part: &str) -> bool {
    matches!(aethelgard_twin::model::model(part), Ok(Some(_)))
}

fn generated(name: &str) -> BTreeMap<String, String> {
    let (solved, table) = fixture(name);
    generate_with("motors-board", &solved, &table, &answers).unwrap()
}

/// The text from `head` to the end of its item (the next line that is `}`).
fn item<'a>(text: &'a str, head: &str) -> &'a str {
    assert_eq!(text.matches(head).count(), 1, "`{head}`");
    let at = text.find(head).unwrap();
    let rest = &text[at..];
    &rest[..rest.find("\n}\n").unwrap()]
}

/// The `r.write` lines of `TIM1`'s start, as (offset, value, register).
fn tim1_writes(board: &str) -> Vec<String> {
    item(board, "impl Tim1<Clocked> {")
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("r."))
        .map(str::to_owned)
        .collect()
}

fn expected(psc: u32, arr: u32, ccr: u32) -> Vec<String> {
    let w = |offset: u32, value: u32, name: &str| {
        format!("r.write(TOKEN, Block::Tim1, {offset:#04x}, {value:#010x}); // {name}")
    };
    vec![
        w(0x28, psc, "PSC"),
        w(0x2c, arr, "ARR"),
        w(0x34, ccr, "CCR1"),
        w(0x38, ccr, "CCR2"),
        w(0x3c, ccr, "CCR3"),
        w(0x40, ccr, "CCR4"),
        w(0x18, 0x6868, "CCMR1"),
        w(0x1c, 0x6868, "CCMR2"),
        w(0x20, 0x1111, "CCER"),
        w(0x44, 0x8000, "BDTR"),
        w(0x14, 0x1, "EGR"),
        w(0x00, 0x81, "CR1"),
    ]
}

#[test]
fn pwm_starts_tim1_with_exactly_the_solved_writes_in_order() {
    let files = generated("motors-pwm");
    assert_eq!(
        tim1_writes(&files["src/board.rs"]),
        expected(63, 2499, 1000)
    );
}

#[test]
fn dshot600_starts_tim1_with_its_own_values_and_the_same_order() {
    let files = generated("motors-dshot600");
    assert_eq!(tim1_writes(&files["src/board.rs"]), expected(0, 106, 40));
}

/// INV-02 in the text: the start takes the token by reference and exactly the
/// timer's four solved pins, each in its function, from `Clocked`, and returns
/// a state with no method; the token comes only from `Rcc::reset_clock`.
#[test]
fn the_start_takes_the_clock_token_and_only_its_own_configured_pins() {
    let files = generated("motors-pwm");
    let board = &files["src/board.rs"];
    let start = item(board, "impl Tim1<Clocked> {");
    assert!(start.contains(
        "        _clock: &ResetClock,\n        \
         _ch1: PA8<AlternateFunction<1>>,\n        \
         _ch2: PA9<AlternateFunction<1>>,\n        \
         _ch3: PA10<AlternateFunction<1>>,\n        \
         _ch4: PA11<AlternateFunction<1>>,\n    \
         ) -> Tim1<Running> {"
    ));
    assert!(!start.contains("r.read") && !start.contains("r.modify"));
    assert!(!board.contains("impl Tim1<Running>"));
    assert!(board.contains("pub struct ResetClock {\n    _private: (),\n}"));
    assert_eq!(board.matches("ResetClock { _private: () }").count(), 1);
    let rcc = item(board, "impl Rcc {");
    assert!(rcc.contains(
        "    pub fn reset_clock<R: Registers>(&self, r: &mut R, _scale2: &Scale2) -> Result<ResetClock, NotResetClock> {\n        \
         let cr = r.read(TOKEN, Block::Rcc, 0x000);\n        \
         let cfgr = r.read(TOKEN, Block::Rcc, 0x010);\n        \
         let d1cfgr = r.read(TOKEN, Block::Rcc, 0x018);\n        \
         let d2cfgr = r.read(TOKEN, Block::Rcc, 0x01c);\n        \
         let d1ccipr = r.read(TOKEN, Block::Rcc, 0x04c);\n        \
         let d2ccip1r = r.read(TOKEN, Block::Rcc, 0x050);\n        \
         let d2ccip2r = r.read(TOKEN, Block::Rcc, 0x054);\n        \
         if cr & 0x00000018 == 0 && cfgr & 0x00008038 == 0 && d1cfgr & 0x00000f7f == 0 && d2cfgr & 0x00000770 == 0 && d1ccipr & 0x30000000 == 0 && d2ccip1r & 0x00077000 == 0 && d2ccip2r & 0x0000303f == 0 {\n            \
         Ok(ResetClock { _private: () })\n"
    ));
    // TIM1's clock gate is RCC_APB2ENR bit 0, and its block is at 0x40010000.
    assert!(rcc.contains(
        "pub fn enable_tim1<R: Registers>(&mut self, r: &mut R, _gated: Tim1<Gated>) -> Tim1<Clocked> {\n        \
         r.modify(TOKEN, Block::Rcc, 0x0f0, |v| v | 0x00000001);\n        \
         let _ = r.read(TOKEN, Block::Rcc, 0x0f0);\n        \
         Tim1 { _state: PhantomData }"
    ));
    assert!(board.contains("            Block::Tim1 => 0x40010000,"));
    // Each pin's own AF1 (AFRH, pins 8 to 11) and MODER, push-pull, no pull.
    let pa8 = item(board, "impl PA8<Unconfigured> {");
    assert!(pa8.contains(
        "pub fn into_tim1_ch1<R: Registers>(self, r: &mut R) -> PA8<AlternateFunction<1>> {"
    ));
    assert!(
        pa8.contains("r.modify(TOKEN, Block::GpioA, 0x24, |v| (v & !0x0000000f) | 0x00000001);")
    );
    assert!(
        pa8.contains("r.modify(TOKEN, Block::GpioA, 0x00, |v| (v & !0x00030000) | 0x00020000);")
    );
    assert!(pa8.contains("r.modify(TOKEN, Block::GpioA, 0x08, |v| v & !0x00030000);"));
}

/// §1.3 and the WO's "stops": the firmware obtains the token first, before
/// any write (`AET-WO-4230` D6), or parks without the boot word; after the
/// identity reads and before the boot word it starts TIM1 on its four pins and
/// never names it again.
#[test]
fn the_firmware_reads_the_clock_first_and_starts_the_timer_after_the_reads() {
    let files = generated("motors-pwm");
    let main = &files["firmware/src/main.rs"];
    let last_read = main.rfind("record(").unwrap();
    let clock = main
        .find(
            "    let Ok(clock) = rcc.reset_clock(&mut r, &scale2) else {\n        \
             loop {\n            \
             core::hint::spin_loop();\n        \
             }\n    \
             };\n",
        )
        .unwrap();
    let start = main
        .find("    let _tim1 = tim1.start_outputs(&mut r, &clock, pa8, pa9, pa10, pa11);\n")
        .unwrap();
    let boot = main.find(".write_volatile(BOOT_MAGIC)").unwrap();
    let first_write = main.find("rcc.enable_").unwrap();
    assert!(clock < first_write && last_read < start && start < boot);
    assert_eq!(main.matches("tim1.").count(), 1);
    assert!(main.contains("    let tim1 = rcc.enable_tim1(&mut r, tim1);\n"));
    for (pin, k) in [("pa8", 1), ("pa9", 2), ("pa10", 3), ("pa11", 4)] {
        let port = "gpioa";
        assert!(main.contains(&format!(
            "    let {pin} = {port}.{pin}.into_tim1_ch{k}(&mut r);\n"
        )));
    }
}

/// Timers start in ascending number, not in name order (`TIM12` sorts
/// before `TIM2`): TIM1, TIM3 and TIM12 here, each with its own token use.
#[test]
fn timers_start_in_ascending_number() {
    let (mut solved, table) = fixture("motors-pwm");
    // Move outputs 3 and 4 to TIM3 CH1 (PA6, AF2) and TIM12 CH1 (PB14, AF2).
    let moved = [("PA6", "TIM3", 2), ("PB14", "TIM12", 2)];
    let nets: Vec<String> = solved
        .motor_outputs_in_order()
        .iter()
        .skip(2)
        .map(|(net, _, _)| (*net).to_owned())
        .collect();
    for (net, (pin, timer, af)) in nets.iter().zip(moved) {
        let a = solved.assignments.get_mut(net).unwrap();
        a.pin = pin.into();
        a.peripheral = timer.into();
        a.signal = format!("{timer}_CH1");
        a.alternate_function = af;
    }
    // PA6 was SPI1's MISO in the fixture: give SPI1 another free MISO.
    let miso = solved
        .assignments
        .iter()
        .find(|(_, a)| a.signal == "SPI1_MISO")
        .map(|(n, _)| n.clone())
        .unwrap();
    solved.assignments.get_mut(&miso).unwrap().pin = "PG9".into();
    solved.validate().unwrap();
    let files = generate_with("b", &solved, &table, &answers).unwrap();
    let main = &files["firmware/src/main.rs"];
    let at = |s: &str| main.find(s).unwrap_or_else(|| panic!("{s}\n{main}"));
    let (t1, t3, t12) = (
        at("let _tim1 = tim1.start_outputs(&mut r, &clock, pa8, pa9);"),
        at("let _tim3 = tim3.start_outputs(&mut r, &clock, pa6);"),
        at("let _tim12 = tim12.start_outputs(&mut r, &clock, pb14);"),
    );
    assert!(t1 < t3 && t3 < t12);
    assert_eq!(main.matches("rcc.reset_clock(").count(), 1);
    // TIM3 and TIM12 have no main output enable: no BDTR write.
    let board = &files["src/board.rs"];
    assert!(!item(board, "impl Tim3<Clocked> {").contains("// BDTR"));
    assert!(!item(board, "impl Tim12<Clocked> {").contains("// BDTR"));
    assert!(item(board, "impl Tim1<Clocked> {").contains("0x00008000); // BDTR"));
}

/// The timer pins are out of the output speed check's scope, `00`, with
/// their own reason; the rest of the report is unchanged.
#[test]
fn timer_pins_take_00_with_their_own_reason() {
    let (solved, table) = fixture("motors-pwm");
    let speeds = pin_speeds(&solved, &table).unwrap();
    for pin in ["PA8", "PA9", "PA10", "PA11"] {
        let p = speeds.iter().find(|p| p.pin == pin).unwrap();
        assert_eq!(p.ospeedr, 0, "{pin}");
        assert_eq!(
            p.check,
            SpeedCheck::OutOfScope {
                reason: MOTOR_OUTPUT
            }
        );
        assert!(p.signal.starts_with("TIM1_CH"), "{}", p.signal);
    }
    let timers = speeds
        .iter()
        .filter(|p| p.signal.starts_with("TIM"))
        .count();
    assert_eq!(timers, 4);
}

/// A timer pin on an AF the pin table does not give that pin is refused by
/// name, as any other solved function is.
#[test]
fn a_timer_pin_on_an_af_the_table_does_not_give_is_refused() {
    let (mut solved, table) = fixture("motors-pwm");
    let net = solved.motor_outputs_in_order()[0].0.to_owned();
    solved.assignments.get_mut(&net).unwrap().alternate_function = 2;
    solved.validate().unwrap();
    let err = generate_with("b", &solved, &table, &answers).unwrap_err();
    assert_eq!(err.kind(), "AssignmentMismatch", "{err}");
    assert!(
        err.to_string().contains("`PA8` carries `TIM1_CH1` on AF"),
        "{err}"
    );
}

/// The trybuild cases include this generated `board.rs` by path; it must be
/// exactly what the generator writes for `motors-pwm`.
/// `AETHELGARD_BLESS=1` rewrites it.
#[test]
fn the_ui_board_is_the_generated_one() {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/ui/motors-pwm/board.rs");
    let text = generated("motors-pwm")["src/board.rs"].clone();
    if std::env::var_os("AETHELGARD_BLESS").is_some() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &text).unwrap();
    }
    assert!(
        fs::read_to_string(&path).unwrap() == text,
        "{} differs",
        path.display()
    );
}

/// The unit tests of the schedule proof's split read the PWM fixture's
/// solved topology from this file; it must be exactly what `solve` gives.
/// `AETHELGARD_BLESS=1` rewrites it.
#[test]
fn the_ui_topology_is_the_solved_one() {
    let path: PathBuf =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/ui/motors-pwm/topology.json");
    let (solved, _) = fixture("motors-pwm");
    let text = serde_json::to_string(&solved).unwrap() + "\n";
    if std::env::var_os("AETHELGARD_BLESS").is_some() {
        fs::write(&path, &text).unwrap();
    }
    assert!(
        fs::read_to_string(&path).unwrap() == text,
        "{} differs",
        path.display()
    );
}

/// Each `#[kani::proof]` harness of a generated `proofs.rs`, by name.
fn harnesses(proofs: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut armed = false;
    for line in proofs.lines().map(str::trim) {
        if line == "#[kani::proof]" {
            armed = true;
        } else if armed && let Some(rest) = line.strip_prefix("fn ") {
            out.push(&rest[..rest.find('(').unwrap()]);
            armed = false;
        }
    }
    out
}

fn schedule_parts(proofs: &str) -> Vec<&str> {
    harnesses(proofs)
        .into_iter()
        .filter(|h| h.starts_with("every_valid_sequence"))
        .collect()
}

/// `AET-WO-3303` §1.2: a board with motor outputs proves its schedule in one
/// part per unit, and keeps every targeted harness; the whole-schedule
/// harness is not emitted.
#[test]
fn a_motor_board_proves_its_schedule_in_one_part_per_unit() {
    for name in ["motors-pwm", "motors-dshot600"] {
        let files = generated(name);
        let proofs = &files["src/proofs.rs"];
        assert_eq!(
            schedule_parts(proofs),
            [
                "every_valid_sequence_i2c1",
                "every_valid_sequence_spi1",
                "every_valid_sequence_tim1"
            ],
            "{name}"
        );
        for kept in [
            "i2c1_transfer_is_bounded_and_ordered",
            "spi1_transfer_is_bounded_and_ordered",
            "tim1_start_writes_the_solved_values",
            "reset_clock_is_ok_exactly_at_reset",
        ] {
            assert!(harnesses(proofs).contains(&kept), "{name}: {kept}");
        }
        // Each part starts its own unit and no other, under the rely.
        let starts = ["I2C1 started", "SPI1 started", "TIM1 started"];
        for (part, own) in ["i2c1", "spi1", "tim1"].into_iter().zip(starts) {
            let body = item(proofs, &format!("fn every_valid_sequence_{part}() {{"));
            assert!(body.contains(own), "{name} {part}");
            for other in starts.iter().filter(|o| **o != own) {
                assert!(!body.contains(other), "{name} {part} schedules {other}");
            }
            assert!(body.contains("img.rely();"), "{name} {part}");
        }
    }
}

/// A GPIO output no unit owns gets its own part, which schedules its port's
/// enable and its configure.
#[test]
fn an_output_no_unit_owns_is_proved_in_the_gpio_part() {
    let (mut solved, table) = fixture("motors-pwm");
    solved.gpio.insert(
        "LED".into(),
        aethelgard_ir::pin_devices::SolvedGpio::output("PB0", aethelgard_ir::motor::Level::High),
    );
    solved.pin_speeds.insert("LED".into(), 0);
    let files = generate_with("motors-board", &solved, &table, &answers).unwrap();
    let proofs = &files["src/proofs.rs"];
    assert_eq!(
        schedule_parts(proofs),
        [
            "every_valid_sequence_i2c1",
            "every_valid_sequence_spi1",
            "every_valid_sequence_tim1",
            "every_valid_sequence_gpio"
        ]
    );
    let body = item(proofs, "fn every_valid_sequence_gpio() {");
    assert!(body.contains("rcc.enable_gpiob(&mut img"), "{body}");
    assert!(
        body.contains("pb0.take().unwrap().into_output(&mut img)"),
        "{body}"
    );
    assert!(
        body.contains("the schedule did not configure PB0"),
        "{body}"
    );
    assert!(!body.contains("enable_gpioa"), "{body}");
}

/// `AET-WO-4230` R5 (the architect's ruling on Q-G): every motor-output pin
/// is pulled down, `PUPDR` `10`, from port A's first init: `split` writes the
/// whole word, the debug port's reset pulls (`0x6400_0000`, RM0433 Rev 9
/// §11.4.4, p. 542) and `10` at PA8..PA11 (bits 23:16, `0x00AA_0000`); no
/// pin method writes `PUPDR` any more, and the proofs hold the word whole.
#[test]
fn motor_pins_are_pulled_down_from_the_first_port_init() {
    for name in ["motors-pwm", "motors-dshot600"] {
        let files = generated(name);
        let board = &files["src/board.rs"];
        let split = item(board, "impl GpioA<Clocked> {");
        assert!(
            split.contains("r.write(TOKEN, Block::GpioA, 0x0c, 0x64aa0000);"),
            "{name}: {split}"
        );
        assert_eq!(board.matches("Block::GpioA, 0x0c").count(), 1, "{name}");
        let proofs = &files["src/proofs.rs"];
        assert!(
            proofs.contains(
                "kani::assert(value == 0x64aa0000, \"GpioA PUPDR is not the solved word, whole\");"
            ),
            "{name}"
        );
    }
}
