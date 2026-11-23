//! `AET-WO-4240e`, the generator's half: the kernel-clock switch's order
//! (D1 (a), the consistency judge the architect named beside the Kani
//! harness and Tviburi's guard), its fault arm (D6), and the ADC clock the
//! topology states (the ruling on the ADC). A kernel mux takes a new input
//! only while its old and new inputs both run (RM0433 Rev 9 pp. 371–372), so
//! PLL1 must be locked before the select and stopped only after it.
//!
//! Every expected word below is written here from RM0433 Rev 9 or read from
//! the topology (`clock.kernel_switch`, the solver's, which
//! `aethelgard_ir::kernel_clock` judges), never from the generator.

use std::fs;
use std::path::Path;

use aethelgard_ir::{SolvedTopology, Specification};
use aethelgard_svd_hal::board::{
    BOOT_PLL1_NOT_LOCKED, BoardError, FieldAccess, SWITCH_READS, generate_with, register_fields,
};
use aethelgard_svd_hal::pin_table;
use sha2::{Digest, Sha256};

/// `RCC_CR` (offset 0x000, reset `0x0000 0001`, p. 383): `HSION` 0,
/// `PLL1ON` 24, `PLL1RDY` 25, written whole (Q11 ruled); `RCC_PLLCKSELR`
/// 0x028, `RCC_PLLCFGR` 0x02C, `RCC_PLL1DIVR` 0x030 (§8.7.11–§8.7.13);
/// `RCC_D2CCIP1R` 0x050 (§8.7.20).
const CR_ON: u32 = 0x0100_0001;
const CR_OFF: u32 = 0x0000_0001;
const PLL1RDY: u32 = 1 << 25;

fn solved(path: &str) -> (SolvedTopology, aethelgard_ir::PinTable) {
    let bytes = fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join(path)).unwrap();
    let spec: Specification = serde_json::from_slice(&bytes).unwrap();
    let sha: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let table = pin_table(&spec.mcu.part).unwrap();
    (aethelgard_ir::solve(&spec, &table, &sha).unwrap(), table)
}

fn cli(name: &str) -> (SolvedTopology, aethelgard_ir::PinTable) {
    solved(&format!("../aethelgard-cli/tests/fixtures/{name}.json"))
}

fn answers(part: &str) -> bool {
    matches!(aethelgard_twin::model::model(part), Ok(Some(_)))
}

fn generate(
    solved: &SolvedTopology,
    table: &aethelgard_ir::PinTable,
) -> std::collections::BTreeMap<String, String> {
    generate_with("switch-board", solved, table, &answers).unwrap()
}

/// `select_spi_kernels`'s body, from its signature to its end.
fn select_body(board: &str) -> &str {
    let at = board
        .find("    pub fn select_spi_kernels<R: Registers>(")
        .expect("select_spi_kernels");
    let rest = &board[at..];
    &rest[..rest.find("\n    }\n").unwrap()]
}

/// The position of each needle in `text`, each found once and after the one
/// before it.
fn in_order(text: &str, needles: &[String]) {
    let mut from = 0;
    for n in needles {
        assert_eq!(text.matches(n.as_str()).count(), 1, "`{n}` once in\n{text}");
        let at = text.find(n.as_str()).unwrap();
        assert!(at >= from, "`{n}` out of order in\n{text}");
        from = at + n.len();
    }
}

#[test]
fn the_switch_locks_pll1_before_the_select_and_stops_it_after() {
    let (t, table) = cli("sentinel-01-bench");
    let k = t
        .clock
        .kernel_switch
        .clone()
        .expect("an HSI board with SPI1 on per_ck has the switch");
    let files = generate(&t, &table);
    let body = select_body(&files["src/board.rs"]);
    let w = |offset: u32, word: u32| {
        format!("r.write(TOKEN, Block::Rcc, {offset:#05x}, {word:#010x});")
    };
    in_order(
        body,
        &[
            w(0x028, k.pllckselr),
            w(0x02C, k.pllcfgr),
            w(0x030, k.pll1divr),
            w(0x000, CR_ON),
            "for _ in 0..pll_polls {".to_owned(),
            format!("r.read(TOKEN, Block::Rcc, 0x000) & {PLL1RDY:#010x} != 0"),
            "return Err(KernelClockFault::Pll1NotLocked);".to_owned(),
            // SPI123SEL `100`, `per_ck` (p. 414), every other field at reset.
            w(0x050, 0x0000_4000),
            w(0x000, CR_OFF),
            "Ok(SpiKernels { _private: () })".to_owned(),
        ],
    );
    // Exactly `SWITCH_READS` reads of `RCC_D2CCIP1R`, straight-line, between
    // the select and `PLL1ON` cleared.
    let select = body.find(&w(0x050, 0x0000_4000)).unwrap();
    let off = body.find(&w(0x000, CR_OFF)).unwrap();
    // No read-modify-write of `RCC_CR`: each word is the contract's.
    assert!(!body.contains("r.modify(TOKEN, Block::Rcc, 0x000,"));
    let settle = "let _ = r.read(TOKEN, Block::Rcc, 0x050); // the transition";
    assert_eq!(
        body[select..off].matches(settle).count(),
        SWITCH_READS as usize
    );
    assert_eq!(body.matches(settle).count(), SWITCH_READS as usize);
    // The transition's reads: 2 cycles of each clock (p. 371), `per_ck` the
    // HSI and `pll1_q_ck` faster, are under 4 HSI cycles at its slowest,
    // 62,342,784 Hz; each read is at least one core cycle at the HSI's
    // fastest, 64,958,976 Hz.
    assert!(u64::from(SWITCH_READS) * 62_342_784 >= 4 * 64_958_976);
    assert!(
        files["src/board.rs"].contains(&format!("pub const SWITCH_READS: u32 = {SWITCH_READS};"))
    );
}

#[test]
fn a_lock_that_does_not_come_writes_its_boot_word_before_any_enable() {
    let (t, table) = cli("sentinel-01-bench");
    let files = generate(&t, &table);
    let main = &files["firmware/src/main.rs"];
    in_order(
        main,
        &[
            format!("const BOOT_PLL1_NOT_LOCKED: u32 = {BOOT_PLL1_NOT_LOCKED:#010x};"),
            // 150 us (DS12117 Rev 10 Table 39) at 64,958,976 Hz, rounded up.
            "const PLL_POLLS: u32 = 9744;".to_owned(),
            "let Ok(clock) = rcc.reset_clock(&mut r, &scale2) else {".to_owned(),
            "let Ok(kernels) = rcc.select_spi_kernels(&mut r, &clock, PLL_POLLS) else {".to_owned(),
            ".write_volatile(BOOT_PLL1_NOT_LOCKED) };".to_owned(),
            "let spi1 = rcc.enable_spi1(".to_owned(),
        ],
    );
    assert!(main.find("rcc.enable_").unwrap() > main.find("select_spi_kernels").unwrap());
}

#[test]
fn a_select_between_live_inputs_starts_no_pll() {
    // SPI6 alone: `SPI6SEL` from `rcc_pclk4` to `hsi_ker_ck` (p. 418), both
    // running from reset.
    let (t, table) = cli("pair-standin");
    assert!(t.clock.kernel_switch.is_none());
    let files = generate(&t, &table);
    let body = select_body(&files["src/board.rs"]);
    for absent in [
        "0x028,",
        "0x02c,",
        "0x030,",
        "Block::Rcc, 0x000,",
        "pll_polls {",
    ] {
        assert!(!body.contains(absent), "`{absent}` in\n{body}");
    }
    assert!(body.contains("_pll_polls: u32,"));
    let main = &files["firmware/src/main.rs"];
    assert!(main.contains("rcc.select_spi_kernels(&mut r, &clock, 0) else {"));
    assert!(!main.contains("BOOT_PLL1_NOT_LOCKED"));
}

#[test]
fn an_hse_board_writes_no_select_and_no_switch() {
    let (t, table) = cli("hse-crystal-16");
    assert!(t.clock.kernel_switch.is_none());
    let files = generate(&t, &table);
    assert!(!files["src/board.rs"].contains("fn select_spi_kernels"));
    assert!(!files["src/board.rs"].contains("SWITCH_READS"));
}

#[test]
fn the_adc_converts_on_the_clock_the_topology_states() {
    let (mut t, table) = solved("../aethelgard-ir/tests/fixtures/d1-drv8876.json");
    // `CKMODE` `11`, `adc_sclk` / 4 (RM0433 Rev 9 p. 1039).
    assert_eq!(t.clock.adc.as_ref().map(|a| a.ckmode), Some(0b11));
    let files = generate(&t, &table);
    assert!(files["src/board.rs"].contains("0x08, 0x00030000); // CCR: CKMODE 11"));
    for found in [None, Some(0), Some(0b01)] {
        t.clock.adc = found.map(|ckmode| aethelgard_ir::clock::SolvedAdcClock { ckmode });
        match generate_with("switch-board", &t, &table, &answers) {
            Err(BoardError::AdcClockNotGenerated { found: f, .. }) => assert_eq!(f, found),
            other => panic!("{found:?}: {:?}", other.map(|_| ())),
        }
    }
}

/// Adversarial pass 4: `SWITCH_READS` is sized for a `pll1_q_ck` at least as
/// fast as `per_ck`; a legal but slower one (`DIVQ1` 127, ÷128: 1.56 MHz at
/// the HSI's slowest) is refused, not emitted with too short a wait.
#[test]
fn a_switch_slower_than_per_ck_is_refused() {
    let (mut t, table) = cli("sentinel-01-bench");
    let k = t.clock.kernel_switch.as_mut().unwrap();
    k.pll1divr = (k.pll1divr & !(0x7F << 16)) | 127 << 16;
    match generate_with("switch-board", &t, &table, &answers) {
        Err(e @ BoardError::SwitchQBelowPerCk { .. }) => assert_eq!(e.kind(), "SwitchQBelowPerCk"),
        other => panic!("{:?}", other.map(|_| ())),
    }
}

/// The ledger's rows for the switch (`AET-WO-4240e`): `RCC_CR` written whole
/// (Q11 ruled), so each of its fields has a written row, `HSION` 1 and the
/// rest 0 as the second word leaves them; and `PLL1RDY` polled, which the
/// bench reads 0 after boot, PLL1 stopped (adversarial pass 2).
#[test]
fn the_switch_s_ledger_rows_are_cr_whole_and_pll1rdy_read_zero() {
    let (t, table) = cli("sentinel-01-bench");
    let rows = register_fields(&t, &table).unwrap();
    let cr: Vec<(&str, FieldAccess, Option<u32>)> = rows
        .iter()
        .filter(|r| r.register == "RCC.CR")
        .map(|r| (r.field.as_str(), r.access, r.value))
        .collect();
    assert!(
        cr.contains(&("PLL1RDY", FieldAccess::Reset, Some(0))),
        "{cr:?}"
    );
    for (field, value) in [
        ("HSION", 1),
        ("PLL1ON", 0),
        ("CSSHSEON", 0),
        ("HSIKERON", 0),
    ] {
        assert!(
            cr.contains(&(field, FieldAccess::Written, Some(value))),
            "{field}: {cr:?}"
        );
    }
}
