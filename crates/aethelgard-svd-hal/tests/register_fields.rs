//! `AET-WO-4230` §1.7, contract D9: the witness ledger's field list covers
//! every register write the generated board crate makes. The check reads
//! the crate's own text, not the plan `register_fields` is built from: each
//! `r.write`, `r.write8` and `r.modify` line, its block and offset, and the
//! bits it sets or changes.
//!
//! - every written register has rows;
//! - a whole-word write (`r.write`) has a row for every CMSIS field of the
//!   register, so a field dropped from the list fails here (WO §5.4);
//! - a literal word's every set bit, and a read-modify-write's every changed
//!   bit, is inside a row.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use aethelgard_ir::{SolvedTopology, Specification};
use aethelgard_svd_hal::board::{FieldAccess, FieldUse, generate, register_fields};
use aethelgard_svd_hal::{cmsis, pin_table};
use sha2::{Digest, Sha256};

fn solve(path: &Path) -> (SolvedTopology, aethelgard_ir::PinTable) {
    let bytes = fs::read(path).unwrap();
    let spec: Specification = serde_json::from_slice(&bytes).unwrap();
    let sha: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let table = pin_table(&spec.mcu.part).unwrap();
    (aethelgard_ir::solve(&spec, &table, &sha).unwrap(), table)
}

fn reference() -> (SolvedTopology, aethelgard_ir::PinTable) {
    solve(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sentinel-01-minimal.json"))
}

fn fixture(name: &str) -> (SolvedTopology, aethelgard_ir::PinTable) {
    solve(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../aethelgard-cli/tests/fixtures/{name}.json")),
    )
}

/// One write of the generated crate's text.
#[derive(Debug)]
struct Written {
    /// `SPI1`, `GPIOA`, `RCC`; or `port` / `block` for a chip select's or a
    /// frame's write through a variable.
    instance: String,
    offset: u32,
    /// `Some(word)` for a literal whole word, `None` for a run-time one.
    word: Option<u32>,
    /// A read-modify-write's changed bits.
    changed: Option<u32>,
    line: String,
}

fn hex(s: &str) -> Option<u32> {
    u32::from_str_radix(s.trim().strip_prefix("0x")?, 16).ok()
}

/// Every write in `board.rs`.
fn writes(board: &str) -> Vec<Written> {
    let mut out = Vec::new();
    for line in board.lines().map(str::trim) {
        let (args, modify) = if let Some(rest) = line
            .strip_prefix("r.write(TOKEN, ")
            .or_else(|| line.strip_prefix("r.write8(TOKEN, "))
        {
            (rest, false)
        } else if let Some(rest) = line.strip_prefix("r.modify(TOKEN, ") {
            (rest, true)
        } else {
            continue;
        };
        let parts: Vec<&str> = args.splitn(3, ", ").collect();
        let [block, offset, rest] = parts.as_slice() else {
            panic!("{line}");
        };
        let instance = block
            .strip_prefix("Block::")
            .unwrap_or(block)
            .to_ascii_uppercase();
        let offset = hex(offset).unwrap_or_else(|| panic!("{line}"));
        let (word, changed) = if modify {
            // `|v| v | BIT`, `|v| v & !MASK`, `|v| (v & !MASK) | VALUE`.
            let body = rest.strip_prefix("|v| ").unwrap().trim_end_matches(");");
            let mask = if let Some(bit) = body.strip_prefix("v | ") {
                hex(bit).unwrap()
            } else {
                let at = body.find("!0x").unwrap() + 1;
                hex(&body[at..at + 10]).unwrap()
            };
            (None, Some(mask))
        } else {
            let value = rest.split(')').next().unwrap();
            (hex(value), None)
        };
        out.push(Written {
            instance,
            offset,
            word,
            changed,
            line: line.to_owned(),
        });
    }
    out
}

/// The CMSIS prefix of a row's register: `SPI1.CFG1` is `SPI_CFG1`.
fn prefix(row: &FieldUse) -> String {
    let (_, reg) = row.register.split_once('.').unwrap();
    format!("{}_{reg}", row.family)
}

fn mask(row: &FieldUse) -> u32 {
    (((1u64 << row.width) - 1) << row.pos) as u32
}

/// The failures of `rows` against the crate `files`, empty when it covers it.
fn uncovered(rows: &[FieldUse], files: &BTreeMap<String, String>) -> Vec<String> {
    let facts = cmsis::register_facts().unwrap();
    let written: Vec<&FieldUse> = rows
        .iter()
        .filter(|r| r.access == FieldAccess::Written)
        .collect();
    let mut failures = Vec::new();
    for w in writes(&files["src/board.rs"]) {
        // A chip select's (`port`) and a frame's (`block`) block is a
        // variable: every GPIO's, every SPI's register at that offset.
        let ours: Vec<&&FieldUse> = written
            .iter()
            .filter(|r| {
                let inst = r.register.split_once('.').unwrap().0;
                r.offset == w.offset
                    && match w.instance.as_str() {
                        "PORT" => r.family == "GPIO",
                        "BLOCK" => r.family == "SPI",
                        i => inst == i,
                    }
            })
            .collect();
        if ours.is_empty() {
            failures.push(format!("no row: {}", w.line));
            continue;
        }
        let covered = ours.iter().fold(0u32, |m, r| m | mask(r));
        let bits = w.word.or(w.changed).unwrap_or(0);
        if bits & !covered != 0 {
            failures.push(format!(
                "bits {:#010x} uncovered: {}",
                bits & !covered,
                w.line
            ));
        }
        if w.changed.is_none() {
            // A whole word: every field of the register has a row.
            let prefixes: BTreeSet<String> = ours.iter().map(|r| prefix(r)).collect();
            for p in prefixes {
                let all = facts
                    .fields_of(&p)
                    .unwrap()
                    .iter()
                    .fold(0u32, |m, (_, _, mask)| m | mask);
                let have = ours
                    .iter()
                    .filter(|r| prefix(r) == p)
                    .fold(0u32, |m, r| m | mask(r));
                if all & !have != 0 {
                    failures.push(format!(
                        "{p}: bits {:#010x} of a whole-word write have no row: {}",
                        all & !have,
                        w.line
                    ));
                }
            }
        }
    }
    failures
}

#[test]
fn every_write_of_the_reference_and_the_motor_board_has_its_rows() {
    for (solved, table) in [reference(), fixture("motors-pwm")] {
        let files = generate("b", &solved, &table).unwrap();
        let rows = register_fields(&solved, &table).unwrap();
        assert_eq!(uncovered(&rows, &files), Vec::<String>::new());
        // Each row once.
        let ids: BTreeSet<(String, FieldAccess)> =
            rows.iter().map(|r| (r.id(), r.access)).collect();
        assert_eq!(ids.len(), rows.len());
    }
}

/// The negative control (WO §5.4): a field the firmware writes, dropped from
/// the list, fails the check; a whole word's field written 0 and a
/// read-modify-written field alike.
#[test]
fn a_field_dropped_from_the_list_fails_the_check() {
    let (solved, table) = reference();
    let files = generate("b", &solved, &table).unwrap();
    let rows = register_fields(&solved, &table).unwrap();
    for id in [
        "SPI1.CFG1.MBR",
        "SPI1.CFG2.CPOL",
        "GPIOA.MODER.MODER5",
        "RCC.D2CCIP1R.SPI45SEL",
        "PWR.CR3.LDOEN",
        "PWR.D3CR.VOS",
    ] {
        let dropped: Vec<FieldUse> = rows
            .iter()
            .filter(|r| r.id() != id || r.access != FieldAccess::Written)
            .cloned()
            .collect();
        assert_eq!(dropped.len() + 1, rows.len(), "{id}");
        assert!(!uncovered(&dropped, &files).is_empty(), "{id}");
    }
}

/// The values: the solved words as the bring-up leaves them, the reset fields
/// at 0, and `BOR_LEV` at the declared level.
#[test]
fn the_reference_rows_carry_the_solved_values() {
    let (solved, table) = reference();
    let rows = register_fields(&solved, &table).unwrap();
    // `AETHELGARD_PRINT_ROWS=1` prints the list, for the ledger's count.
    if std::env::var_os("AETHELGARD_PRINT_ROWS").is_some() {
        for r in &rows {
            println!(
                "{:?}\t{}\t{}\t{}\t{}\t{}\t{:?}\t{}",
                r.access,
                r.family,
                r.id(),
                r.offset,
                r.pos,
                r.width,
                r.value,
                r.whole_word
            );
        }
    }
    let value = |id: &str, access: FieldAccess| {
        rows.iter()
            .find(|r| r.id() == id && r.access == access)
            .unwrap_or_else(|| panic!("{id}"))
            .value
    };
    let written = FieldAccess::Written;
    assert_eq!(value("SPI1.CFG1.MBR", written), Some(1));
    assert_eq!(value("SPI1.CFG1.DSIZE", written), Some(7));
    assert_eq!(value("SPI1.CFG1.CRCSIZE", written), Some(7));
    assert_eq!(value("SPI1.CFG2.SSM", written), Some(1));
    assert_eq!(value("SPI1.CFG2.MASTER", written), Some(1));
    assert_eq!(value("SPI1.CR1.SSI", written), Some(1));
    assert_eq!(value("SPI1.CR1.SPE", written), Some(0));
    assert_eq!(value("SPI1.CR2.TSIZE", written), None);
    assert_eq!(value("RCC.D2CCIP1R.SPI123SEL", written), Some(0b100));
    assert_eq!(value("RCC.D2CCIP1R.SPI123SEL", FieldAccess::Reset), Some(0));
    assert_eq!(value("I2C1.CR1.PE", written), Some(1));
    assert_eq!(value("I2C1.CR1.ANFOFF", written), Some(0));
    // Fast400k, the mode the firmware starts I2C1 at.
    assert_eq!(value("I2C1.TIMINGR.SCLL", written), Some(0x3C));
    assert_eq!(value("I2C1.TIMINGR.PRESC", written), Some(1));
    assert_eq!(value("GPIOB.OTYPER.OT6", written), Some(1));
    assert_eq!(value("GPIOA.OTYPER.OT5", written), Some(0));
    assert_eq!(value("RCC.APB2ENR.SPI1EN", written), Some(1));
    assert_eq!(
        value("FLASH.OPTSR_CUR.BOR_LEV", FieldAccess::Reset),
        Some(3)
    );
    // `AET-WO-4230` R4: `AFCNTR` on the master.
    assert_eq!(value("SPI1.CFG2.AFCNTR", written), Some(1));
    // R5: `PUPDR` whole, by port: the debug pulls kept (PA13 pull-up, PA14
    // pull-down, PB4 pull-up), no solved pin of the reference pulled.
    assert_eq!(value("GPIOA.PUPDR.PUPD13", written), Some(0b01));
    assert_eq!(value("GPIOA.PUPDR.PUPD14", written), Some(0b10));
    assert_eq!(value("GPIOB.PUPDR.PUPD4", written), Some(0b01));
    assert_eq!(value("GPIOA.PUPDR.PUPD5", written), Some(0));
    assert!(
        rows.iter()
            .filter(|r| r.register == "GPIOA.PUPDR")
            .all(|r| r.whole_word)
    );
    // R7: `D2CKRDY`, a dependency the firmware never polls (RM0433 Rev 9
    // §8.5.10 step c), 1 once a D2 peripheral is enabled.
    let d2 = rows
        .iter()
        // `AET-WO-4240e`: the kernel-clock switch writes `RCC_CR` whole, so the
        // read-only flag also has a written row (0); the dependency is the
        // reset row's.
        .find(|r| r.id() == "RCC.CR.D2CKRDY" && r.access == FieldAccess::Reset)
        .expect("RCC.CR.D2CKRDY");
    assert_eq!(
        (d2.access, d2.value, d2.pos, d2.width, d2.offset),
        (FieldAccess::Reset, Some(1), 15, 1, 0)
    );
    // `AET-WO-4240v` §1.7: the supply bits (a read-modify-write), `D3CR`
    // whole (its read-only `VOSRDY` written 0), the two polled ready bits and
    // `FLASH_ACR` at reset.
    let row = |id: &str, access: FieldAccess| {
        let r = rows
            .iter()
            .find(|r| r.id() == id && r.access == access)
            .unwrap_or_else(|| panic!("{id}"));
        (r.offset, r.pos, r.width, r.value, r.whole_word)
    };
    assert_eq!(row("PWR.CR3.SCUEN", written), (0x0C, 2, 1, Some(0), false));
    assert_eq!(row("PWR.CR3.LDOEN", written), (0x0C, 1, 1, Some(1), false));
    assert_eq!(row("PWR.CR3.BYPASS", written), (0x0C, 0, 1, Some(0), false));
    assert_eq!(
        row("PWR.D3CR.VOS", written),
        (0x18, 14, 2, Some(0b10), true)
    );
    assert_eq!(
        row("PWR.D3CR.VOSRDY", written),
        (0x18, 13, 1, Some(0), true)
    );
    let reset = FieldAccess::Reset;
    assert_eq!(
        row("PWR.CSR1.ACTVOSRDY", reset),
        (0x04, 13, 1, Some(1), false)
    );
    assert_eq!(row("PWR.D3CR.VOSRDY", reset), (0x18, 13, 1, Some(1), false));
    assert_eq!(
        row("FLASH.ACR.LATENCY", reset),
        (0x00, 0, 4, Some(7), false)
    );
    assert_eq!(
        row("FLASH.ACR.WRHIGHFREQ", reset),
        (0x00, 4, 2, Some(0b11), false)
    );
    // `PWR_CR3`'s bits no header field names, written back as read (RM0433
    // Rev 9 p. 308, "kept at reset value"; bit 6 resets to 1 on revision V),
    // one run-time row per run, so the twin's completeness test covers them.
    assert_eq!(row("PWR.CR3.RES[7:3]", written), (0x0C, 3, 5, None, false));
    assert_eq!(
        row("PWR.CR3.RES[23:10]", written),
        (0x0C, 10, 14, None, false)
    );
    assert_eq!(
        row("PWR.CR3.RES[31:27]", written),
        (0x0C, 27, 5, None, false)
    );
    assert_eq!(rows.iter().filter(|r| r.family == "PWR").count(), 10);
    // No other `PWR_CR3` field is a row: every named bit but the supply
    // configuration's is written as read too, as every read-modify-write's.
    assert!(
        rows.iter()
            .filter(|r| r.register == "PWR.CR3")
            .all(|r| ["SCUEN", "LDOEN", "BYPASS"].contains(&r.field.as_str())
                || r.field.starts_with("RES["))
    );
}
