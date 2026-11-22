//! `AET-WO-4230` §1.1–§1.4, §1.10, review A5: the words the generated board
//! crate writes for every SPI and I2C controller are the independent rule's
//! (`aethelgard_ir::silicon`), on every fixture that solves; and the
//! generator refuses by name a solved controller the rule does not hold.
//!
//! The generator composes its words from the solved values and the CMSIS
//! header; the rule from RM0433 by its own bit positions. A mutant of either
//! side fails here without a Kani run (WO §5.1, §5.2).

use std::path::{Path, PathBuf};

use aethelgard_ir::{SolvedTopology, Specification, silicon};
use aethelgard_svd_hal::board::{check_silicon, generate};
use aethelgard_svd_hal::pin_table;
use sha2::{Digest, Sha256};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The committed specification in `bytes`, or `None` for JSON that is not one
/// (an object without both `mcu` and `board`): it must parse and carry the
/// current schema version, or this fails naming it, never skipping it
/// (`AET-WO-4400g` F5, `INV-06`). The CLI's tests hold the same rule in
/// `tests/common/spec.rs`; this crate cannot reach that file.
fn committed(path: &Path, bytes: &[u8]) -> Option<Specification> {
    let v: serde_json::Value = serde_json::from_slice(bytes).unwrap_or_else(|e| {
        panic!(
            "{}: committed JSON that does not parse: {e}",
            path.display()
        )
    });
    if v.get("mcu").is_none() || v.get("board").is_none() {
        return None;
    }
    let spec: Specification = serde_json::from_slice(bytes).unwrap_or_else(|e| {
        panic!(
            "{}: a specification that does not parse: {e}",
            path.display()
        )
    });
    assert_eq!(
        spec.schema_version,
        aethelgard_ir::SPECIFICATION_SCHEMA_VERSION,
        "{}: a committed specification at another schema version",
        path.display()
    );
    Some(spec)
}

/// Every specification under `examples/` and the CLI's fixtures that solves.
fn solved() -> Vec<(String, SolvedTopology)> {
    let table = pin_table("STM32H753ZITx").unwrap();
    let mut paths: Vec<PathBuf> = Vec::new();
    for dir in ["examples", "crates/aethelgard-cli/tests/fixtures"] {
        for e in std::fs::read_dir(root().join(dir)).unwrap() {
            let p = e.unwrap().path();
            if p.extension().is_some_and(|x| x == "json") {
                paths.push(p);
            }
        }
    }
    paths.sort();
    let mut out = Vec::new();
    for p in paths {
        let bytes = std::fs::read(&p).unwrap();
        let Some(spec) = committed(&p, &bytes) else {
            continue;
        };
        let sha: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if let Ok(t) = aethelgard_ir::solve(&spec, &table, &sha) {
            out.push((p.file_name().unwrap().to_string_lossy().into_owned(), t));
        }
    }
    out
}

/// `Spi1`, `I2c1`.
/// The declared speed of the bus `peripheral` serves, joined here from the
/// topology alone and never through the generator (`AET-WO-3402` §1.3, ruled
/// on PR #203: `RUL-20`, the judge shares no code with what it judges): each
/// I2C line's bus is its `analog_lines` entry, and its peripheral the
/// assignment of its net. Exactly one bus, with every line on `peripheral`.
fn own_bus_speed(t: &SolvedTopology, peripheral: &str) -> u32 {
    let on = |net: &str| t.assignments.get(net).map(|a| a.peripheral.clone());
    let mut buses: Vec<&str> = t
        .analog_lines
        .iter()
        .filter(|(net, _)| on(net).as_deref() == Some(peripheral))
        .map(|(_, line)| line.bus.as_str())
        .collect();
    buses.sort_unstable();
    buses.dedup();
    assert_eq!(buses.len(), 1, "{peripheral} serves buses {buses:?}");
    for (net, line) in &t.analog_lines {
        if line.bus == buses[0] {
            assert_eq!(
                on(net).as_deref(),
                Some(peripheral),
                "{net} of {}",
                buses[0]
            );
        }
    }
    t.i2c_speeds[buses[0]]
}

fn block(peripheral: &str) -> String {
    let (head, n) = peripheral.split_at(peripheral.len() - 1);
    format!("{}{}{n}", &head[..1], head[1..].to_ascii_lowercase())
}

#[test]
fn every_fixture_s_controllers_are_written_the_rule_s_words() {
    let table = pin_table("STM32H753ZITx").unwrap();
    let all = solved();
    let mut checked = (0, 0);
    for (name, t) in &all {
        let files = generate("b", t, &table).unwrap_or_else(|e| panic!("{name}: {e}"));
        let board = &files["src/board.rs"];
        for p in t.spi_controllers.keys() {
            let w = silicon::spi_words(&t.spi_controllers, p).unwrap();
            let b = block(p);
            // A bus with no device has no transfer, so no enable or start.
            let transfer = board.contains(&format!("pub enum {b}Device {{"));
            let mut words = vec![
                (0x00, w.cr1_idle, "CR1 idle"),
                (0x08, w.cfg1, "CFG1"),
                (0x0c, w.cfg2, "CFG2"),
            ];
            if transfer {
                words.push((0x00, w.cr1_enable, "CR1 enable"));
                words.push((0x00, w.cr1_start, "CR1 start"));
            }
            for (offset, word, what) in words {
                let line = format!("r.write(TOKEN, Block::{b}, {offset:#04x}, {word:#010x});");
                assert!(board.contains(&line), "{name} {p} {what}: `{line}`");
            }
            // The 3302 bench (NUCLEO-H753ZI rev V): with `SSM` 0 the first
            // `SPE` raised `MODF` and cleared `MASTER`. So `init` writes `CFG2`
            // with `MASTER` and `SSM` before any write that sets `SPE`, and every
            // `CR1` word with `SPE` carries `SSI`. `AFCNTR` too (the architect's
            // ruling on Q-D, RM0433 Rev 9 §50.11.4): the master keeps driving
            // SCK and MOSI while `SPE` is 0 and a chip select is low.
            let cfg2 = format!("r.write(TOKEN, Block::{b}, 0x0c, ");
            let cfg2_at = board.find(&cfg2).unwrap_or_else(|| panic!("{name} {p}"));
            let cfg2_word =
                u32::from_str_radix(&board[cfg2_at + cfg2.len() + 2..][..8], 16).unwrap();
            assert_eq!(
                cfg2_word & (1 << 22 | 1 << 26 | 1 << 31),
                1 << 22 | 1 << 26 | 1 << 31,
                "{name} {p}"
            );
            let cr1 = format!("r.write(TOKEN, Block::{b}, 0x00, ");
            for (at, _) in board.match_indices(&cr1) {
                let word = u32::from_str_radix(&board[at + cr1.len() + 2..][..8], 16).unwrap();
                if word & 1 != 0 {
                    assert!(cfg2_at < at, "{name} {p}: SPE written before CFG2");
                    assert_ne!(word & 1 << 12, 0, "{name} {p}: SPE without SSI");
                }
            }
            // An HSE board writes no select: each stays at reset, which the
            // rule's word is (`AET-WO-4240b`, `4240` D6).
            let (offset, word) = w.kernel_select;
            let line = format!("r.write(TOKEN, Block::Rcc, {offset:#05x}, {word:#010x});");
            if t.clock.hse.is_some() {
                assert_eq!(word, 0, "{name} {p}: the rule's select is not at reset");
                assert!(
                    !board.contains(&line),
                    "{name} {p} select written: `{line}`"
                );
            } else {
                assert!(board.contains(&line), "{name} {p} select: `{line}`");
            }
            checked.0 += 1;
        }
        for (p, timing) in &t.i2c_timings {
            let b = block(p);
            let cr1 = silicon::i2c_cr1();
            for word in [cr1, cr1 | 1] {
                let line = format!("r.write(TOKEN, Block::{b}, 0x00, {word:#010x});");
                assert!(board.contains(&line), "{name} {p}: `{line}`");
            }
            // Each admitted mode's `TIMINGR` is the solved word, which the
            // rule holds. Each peripheral at its own bus's declared speed
            // (`AET-WO-3402` §1.3), with no admitted mode above it, read in
            // that peripheral's own `{m}_speed` module: two peripherals share
            // a mode's word, so a search of the whole file cannot tell them
            // apart.
            let speed = own_bus_speed(t, p);
            let m = p.to_ascii_lowercase();
            let module = board
                .split(&format!("pub mod {m}_speed {{"))
                .nth(1)
                .and_then(|rest| rest.split("\n}\n").next())
                .unwrap_or_else(|| panic!("{name} {p}: no {m}_speed module"));
            for (mode, hz) in [
                ("Standard100k", 100_000),
                ("Fast400k", 400_000),
                ("FastPlus1M", 1_000_000),
            ] {
                let word = timing.timingr[mode];
                let admitted = format!(
                    "impl AtMostCeiling for {mode} {{\n        const TIMINGR: u32 = {word:#010x};"
                );
                assert_eq!(module.contains(&admitted), hz <= speed, "{name} {p} {mode}");
                assert_eq!(
                    module.contains(&format!("impl AtMostCeiling for {mode} ")),
                    hz <= speed,
                    "{name} {p} {mode}: admitted with another word"
                );
                assert!(
                    silicon::check_timingr_on(t.clock.budget(), timing.kernel_hz, mode, word)
                        .unwrap()
                        .holds
                );
            }
            checked.1 += 1;
        }
        // No configuration register is read-modify-written (D5): the only
        // `modify` left is an RCC gate's, a pin field's, or `PWR_CR3`'s supply
        // bits, which keep every other bit as read (`AET-WO-4240v` §1.4:
        // reserved bits "must be kept at reset value", RM0433 Rev 9 §6.8.4).
        for line in board.lines().filter(|l| l.contains("r.modify(")) {
            assert!(
                line.contains("Block::Rcc")
                    || line.contains("Block::Gpio")
                    || line.contains("r.modify(TOKEN, Block::Pwr, 0x0c, "),
                "{name}: {line}"
            );
        }
        assert_eq!(board.matches("Block::Pwr, 0x0c, |v|").count(), 1, "{name}");
    }
    eprintln!(
        "{} fixtures, {} SPI and {} I2C controllers",
        all.len(),
        checked.0,
        checked.1
    );
    // A floor, so the test cannot pass by checking nothing. Measured: 15
    // fixtures, 20 SPI and 21 I2C controllers (A1's sweep also reads the
    // other crates' fixtures).
    assert!(
        all.len() >= 15 && checked.0 >= 20 && checked.1 >= 21,
        "{checked:?}"
    );
}

/// `AET-WO-4230` R3-fix (the adversarial pass's F3): in every fixture's
/// generated proofs, the schedule starts each I2C bus last at the mode the
/// independent rule gives its declared speed (`silicon::i2c_start_mode`,
/// which is the mode the firmware's `main` starts it at), and before every
/// `init` the oracle expects the topology's `TIMINGR` for that `init`'s
/// mode, so Kani can hold them. A generator that admits a mode above the
/// declared speed (mutant M3) starts the bus there while the oracle expects
/// the rule's mode's word: the pairing fails here, as the proof does.
#[test]
fn every_fixture_s_schedule_starts_each_i2c_bus_at_the_rule_s_mode() {
    let table = pin_table("STM32H753ZITx").unwrap();
    let mut checked = 0;
    for (name, t) in &solved() {
        if t.i2c_timings.is_empty() {
            continue;
        }
        let files = generate("b", t, &table).unwrap_or_else(|e| panic!("{name}: {e}"));
        let lines: Vec<&str> = files["src/proofs.rs"].lines().map(str::trim).collect();
        let main = &files["firmware/src/main.rs"];
        for (p, timing) in &t.i2c_timings {
            // The rule's start mode for this peripheral's own bus
            // (`AET-WO-3402` §1.3), not the lowest bus's.
            let rule = silicon::i2c_start_mode(u64::from(own_bus_speed(t, p))).unwrap();
            let m = p.to_ascii_lowercase();
            let set = format!("{m}_timingr = 0x");
            let speed = format!("{m}_speed::");
            let started: Vec<(u32, &str)> = lines
                .windows(2)
                .filter_map(|w| {
                    let word = w[0].strip_prefix(set.as_str())?.strip_suffix(';')?;
                    let mode = w[1].split(speed.as_str()).nth(1)?.split(',').next()?;
                    Some((u32::from_str_radix(word, 16).unwrap(), mode))
                })
                .collect();
            assert!(!started.is_empty(), "{name} {p}");
            for (word, mode) in &started {
                assert_eq!(
                    Some(word),
                    timing.timingr.get(*mode),
                    "{name} {p}: the oracle's TIMINGR before init at {mode}"
                );
            }
            assert_eq!(started.last().unwrap().1, rule, "{name} {p}");
            assert!(
                main.contains(&format!("{m}.init::<{m}_speed::{rule}, _>(")),
                "{name} {p}: main does not start at {rule}"
            );
            checked += 1;
        }
    }
    assert!(checked >= 2, "{checked}");
}

fn reference() -> SolvedTopology {
    let bytes = std::fs::read(root().join("examples/sentinel-01-minimal.json")).unwrap();
    let spec: Specification = serde_json::from_slice(&bytes).unwrap();
    let table = pin_table(&spec.mcu.part).unwrap();
    aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap()
}

/// A solved `MBR` one off either way is refused by name, naming the
/// peripheral and the finding (`SiliconRuleFails`).
#[test]
fn an_mbr_the_rule_refuses_is_refused_by_name() {
    for delta in [1i8, -1] {
        let mut t = reference();
        let c = t.spi_controllers.get_mut("SPI1").unwrap();
        c.mbr = c.mbr.checked_add_signed(delta).unwrap();
        c.achieved_hz = c.kernel_hz >> (c.mbr + 1);
        c.achieved_max_hz = silicon::fast_hz(c.kernel_hz) >> (c.mbr + 1);
        let e = check_silicon(&t).unwrap_err();
        assert_eq!(e.kind(), "SiliconRuleFails", "{e}");
        let text = e.to_string();
        assert!(text.contains("`SPI1`") && text.contains("mbr_ok"), "{text}");
    }
}

/// A `TIMINGR` field one off either way is refused by name, with its mode.
#[test]
fn a_timingr_the_rule_refuses_is_refused_by_name() {
    for delta in [1i64, -1] {
        let mut t = reference();
        let w = t
            .i2c_timings
            .get_mut("I2C1")
            .unwrap()
            .timingr
            .get_mut("Fast400k")
            .unwrap();
        *w = u32::try_from(i64::from(*w) + delta).unwrap();
        let e = check_silicon(&t).unwrap_err();
        assert_eq!(e.kind(), "SiliconRuleFails", "{e}");
        let text = e.to_string();
        assert!(
            text.contains("`I2C1`") && text.contains("Fast400k TIMINGR"),
            "{text}"
        );
    }
    // And an I2C `CR1` that is not the rule's.
    let mut t = reference();
    t.i2c_timings.get_mut("I2C1").unwrap().cr1 = 1 << 12;
    let text = check_silicon(&t).unwrap_err().to_string();
    assert!(text.contains("cr1_ok"), "{text}");
    // The reference holds.
    assert_eq!(check_silicon(&reference()), Ok(()));
}

/// `AET-WO-4230` R3 (the architect's ruling on Q-H): the generation-time rule
/// check (`check_silicon`, which `generate` runs first) refuses a serial
/// port whose `BRR` is not the nearest whole divider by the independent
/// inequality (`silicon::check_brr`), naming the port's peripheral
/// (`SiliconRuleFails`). The reference's USART1 runs 115200 baud on 64 MHz at
/// `BRR` 556 (2·|556·115200 − 64e6| = 102,400 ≤ 115,200); 555 and 557 are off
/// by more than half a baud (128,000 and 332,800). Through `generate` the
/// hand-edited topology is refused earlier, by `SolvedTopology::validate`
/// (it recomputes `BRR` with the solver's arithmetic); the rule check is what
/// stands when the solver itself is wrong.
#[test]
fn a_brr_the_rule_refuses_is_refused_by_name_at_generation() {
    let table = pin_table("STM32H753ZITx").unwrap();
    let t = reference();
    let port = t.serial_ports.values().next().expect("a serial port");
    assert_eq!(
        (port.peripheral.as_str(), port.brr, port.kernel_hz),
        ("USART1", 556, 64_000_000)
    );
    generate("b", &t, &table).unwrap();
    for brr in [555, 557] {
        let mut t = reference();
        let port = t.serial_ports.values_mut().next().unwrap();
        port.brr = brr;
        assert_eq!(
            generate("b", &t, &table).unwrap_err().kind(),
            "InvalidTopology"
        );
        let e = check_silicon(&t).unwrap_err();
        assert_eq!(e.kind(), "SiliconRuleFails", "{e}");
        let text = e.to_string();
        assert!(
            text.contains("`USART1`") && text.contains(&format!("check_brr: BRR {brr} ")),
            "{text}"
        );
    }
}

/// `AET-WO-4230` R3-fix (the adversarial pass's F1, mutant M1): the
/// generation-time rule derives each USART's kernel clock itself (RM0433
/// Rev 9 §8.7.21, `rcc_pclk2` for USART1 and USART6 and `rcc_pclk1` for the
/// rest, both 64 MHz at the reset clock) and refuses a topology whose
/// kernel is not it, even when its `BRR` divides that kernel exactly as the
/// inequality wants: 32 MHz at `BRR` 278 is 230,400 baud on silicon.
#[test]
fn a_usart_kernel_the_rule_does_not_derive_is_refused_at_generation() {
    let mut t = reference();
    let port = t.serial_ports.values_mut().next().unwrap();
    port.kernel_hz = 32_000_000;
    port.brr = 278;
    let e = check_silicon(&t).unwrap_err();
    assert_eq!(e.kind(), "SiliconRuleFails", "{e}");
    let text = e.to_string();
    assert!(
        text.contains("`USART1`")
            && text.contains(
                "check_brr: the topology's kernel clock 32000000 Hz is not the rule's 64000000 Hz"
            ),
        "{text}"
    );
}
