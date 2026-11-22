//! `AET-WO-4240b`, the generator's half: an HSE board's bring-up (the PM's
//! contract, steps 0–11), its `SolvedClock` token, the kernel selects left at
//! reset, the waits sized at the source's fastest, the CSS NMI's hold, and
//! the boot words. Both fixtures (`hse-crystal-16`, `hse-external-8`) are
//! `sentinel-02-shaped` with `mcu.hse` declared; the shaped board, with no
//! HSE, is the control. Every expected word below is written here from the
//! contract and RM0433 Rev 9, never read from the generator.
//!
//! `AETHELGARD_DUMP=<dir>` writes each fixture's generated tree there.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use aethelgard_ir::{SolvedTopology, Specification};
use aethelgard_svd_hal::board::{
    BOOT_FLASH_ACR_NOT_TAKEN, BOOT_HSE_CSS_FAULT, BOOT_HSE_NOT_READY, BOOT_PLL1_NOT_LOCKED,
    BOOT_SW_NOT_TAKEN, generate_with,
};
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

fn answers(part: &str) -> bool {
    matches!(aethelgard_twin::model::model(part), Ok(Some(_)))
}

fn generated(name: &str) -> BTreeMap<String, String> {
    let (solved, table) = fixture(name);
    let files = generate_with("hse-board", &solved, &table, &answers)
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    if let Some(dir) = std::env::var_os("AETHELGARD_DUMP") {
        for (path, text) in &files {
            let p = PathBuf::from(&dir).join(name).join(path);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, text).unwrap();
        }
    }
    files
}

/// The text from `head` to the end of its item (the next line that is `}`
/// at `indent`).
fn item<'a>(text: &'a str, head: &str, end: &str) -> &'a str {
    assert_eq!(text.matches(head).count(), 1, "`{head}`");
    let at = text.find(head).unwrap();
    let rest = &text[at..];
    &rest[..rest.find(end).unwrap()]
}

/// The bring-up's register writes and reads, in order, as `Hse::start` has
/// them: `(register offset or "FLASH", word)`.
fn start_body(board: &str) -> &str {
    item(board, "    pub fn start<R: Registers>(", "\n    }\n")
}

/// The contract's words: `RCC_CR` from its reset `0x0000_0001` (RM0433 Rev 9
/// p. 383) with `HSEBYP` 18, `HSEON` 16, `HSECSSON` 19 and `PLL1ON` 24.
fn contract_cr(external: bool) -> Vec<u32> {
    if external {
        vec![0x0004_0001, 0x0005_0001, 0x000D_0001, 0x010D_0001]
    } else {
        vec![0x0001_0001, 0x0009_0001, 0x0109_0001]
    }
}

#[test]
fn the_bring_up_writes_the_contract_s_words_in_order() {
    for (name, external) in [("hse-crystal-16", false), ("hse-external-8", true)] {
        let (solved, _) = fixture(name);
        let h = solved.clock.hse.as_ref().unwrap();
        let files = generated(name);
        let body = start_body(&files["src/board.rs"]);
        let cr = contract_cr(external);
        let mut want: Vec<String> = Vec::new();
        let w =
            |off: u32, word: u32| format!("r.write(TOKEN, Block::Rcc, {off:#05x}, {word:#010x});");
        let n = cr.len();
        for word in &cr[..n - 2] {
            want.push(w(0x000, *word));
        }
        want.push("if !clock_ready(r, 0x000, 0x00020000, 0x00020000, hse_polls) {".into());
        want.push("return Err(ClockFault::HseNotReady);".into());
        want.push(w(0x000, cr[n - 2]));
        want.push(w(0x028, h.pllckselr));
        want.push(w(0x02c, h.pllcfgr));
        want.push(w(0x030, h.pll1divr));
        want.push(w(0x000, cr[n - 1]));
        want.push("if !clock_ready(r, 0x000, 0x02000000, 0x02000000, pll_polls) {".into());
        want.push("return Err(ClockFault::Pll1NotLocked);".into());
        want.push(format!(
            "r.write(TOKEN, Block::Flash, 0x00, {:#010x});",
            h.flash_acr
        ));
        want.push(format!(
            "if (r.read(TOKEN, Block::Flash, 0x00) ^ {:#010x}) & 0x0000003f != 0 {{",
            h.flash_acr
        ));
        want.push("return Err(ClockFault::FlashAcrNotTaken);".into());
        want.push(w(0x010, 0x0000_0003));
        want.push("if !clock_ready(r, 0x010, 0x00000038, 0x00000018, sws_polls) {".into());
        want.push("return Err(ClockFault::SwNotTaken);".into());
        want.push("Ok(SolvedClock { _private: () })".into());
        let got: Vec<String> = body
            .lines()
            .map(|l| l.trim().split(" // ").next().unwrap().to_owned())
            .filter(|l| {
                l.starts_with("r.")
                    || l.starts_with("if ")
                    || l.starts_with("return")
                    || l.starts_with("Ok(")
            })
            .collect();
        assert_eq!(got, want, "{name}");
        // The solved words of the 16 MHz crystal (4240 FEEDBACK §4.5).
        if !external {
            assert_eq!(
                (h.pllckselr, h.pllcfgr, h.pll1divr, h.flash_acr),
                (0x52, 0x0003_0004, 0x0103_064F, 0x11)
            );
        }
    }
}

/// Bounds in reads at HSI's fastest, 64,958,976 Hz (contract "Bounds"), and
/// the waits after the switch at the source's fastest (contract "After the
/// switch").
#[test]
fn the_bounds_and_spins_are_the_contract_s() {
    for (name, settle, fast) in [
        ("hse-crystal-16", 1_280_131u64, 64_006_528u64),
        ("hse-external-8", 1_280_064, 64_003_200),
    ] {
        let files = generated(name);
        let main = &files["firmware/src/main.rs"];
        for (c, v) in [
            ("HSE_POLLS", 1_299_180),
            ("PLL_POLLS", 9_744),
            ("SWS_POLLS", 9_744),
        ] {
            assert!(
                main.contains(&format!("const {c}: u32 = {v};")),
                "{name} {c}"
            );
        }
        assert!(
            main.contains("    let clock = match hse.start(&mut r, clock, HSE_POLLS, PLL_POLLS, SWS_POLLS) {\n"),
            "{name}"
        );
        // Sentinel-02's 20 ms start-up wait (the IMU's), at the source's fastest.
        assert!(
            main.contains(&format!("    settle({settle});\n")),
            "{name}\n{main}"
        );
        assert!(
            main.contains(&format!("{fast} Hz (AET-WO-4240b)")),
            "{name}"
        );
    }
}

/// The control's counts do not move: `sentinel-02-shaped` with no HSE waits
/// at HSI's fastest, as before `AET-WO-4240b`.
#[test]
fn the_control_keeps_its_hsi_counts() {
    let main = &generated("sentinel-02-shaped")["firmware/src/main.rs"];
    assert!(main.contains("    settle(1299180);\n"));
    assert!(main.contains("fastest, 64958976 Hz (DS12117 Rev 10 Table 36"));
    assert!(!main.contains("HSE_POLLS"));
    assert!(!main.contains("nmi"));
}

/// No kernel select is written on an HSE board (`4240` D6), and every SPI
/// controller's `MBR` is the control's.
#[test]
fn no_select_is_written_and_no_mbr_moves() {
    let control = generated("sentinel-02-shaped");
    for name in ["hse-crystal-16", "hse-external-8"] {
        let files = generated(name);
        let board = &files["src/board.rs"];
        let main = &files["firmware/src/main.rs"];
        assert!(!board.contains("select_spi_kernels"), "{name}");
        assert!(!board.contains("SpiKernels"), "{name}");
        assert!(!main.contains("select_spi_kernels"), "{name}");
        // No write to RCC_D2CCIP1R (0x050) or RCC_D3CCIPR (0x058) anywhere.
        for off in ["0x050", "0x058"] {
            assert!(
                !board.contains(&format!("r.write(TOKEN, Block::Rcc, {off},")),
                "{name} {off}"
            );
        }
        let (solved, _) = fixture(name);
        let (shaped, _) = fixture("sentinel-02-shaped");
        for (p, c) in &solved.spi_controllers {
            assert_eq!(c.mbr, shaped.spi_controllers[p].mbr, "{name} {p}");
            let cfg1 = |b: &str| {
                b.lines()
                    .find(|l| {
                        l.contains(&format!(
                            "Block::{}{}, 0x08,",
                            &p[..1],
                            p[1..].to_ascii_lowercase()
                        ))
                    })
                    .map(str::trim)
                    .map(str::to_owned)
            };
            assert_eq!(
                cfg1(board),
                cfg1(&control["src/board.rs"]),
                "{name} {p} CFG1"
            );
        }
        // SPI clock enables take the solved clock.
        assert!(main.contains("let spi1 = rcc.enable_spi1(&mut r, spi1, scale2, &clock);"));
    }
}

/// The token: produced only by `Hse::start`, after `SWS`; every start takes
/// it, and none takes the reset clock's.
#[test]
fn every_start_takes_the_solved_clock() {
    for name in ["hse-crystal-16", "hse-external-8"] {
        let board = &generated(name)["src/board.rs"];
        assert_eq!(board.matches("SolvedClock { _private: () }").count(), 1);
        assert!(!board.contains("_clock: &ResetClock"), "{name}");
        assert!(board.contains("_clock: ResetClock,"), "{name}");
        for head in [
            "pub fn init<S: i2c1_speed::AtMostCeiling, R: Registers>(",
            "pub fn start_outputs<R: Registers>(",
            "pub fn configure<R: Registers>(",
        ] {
            let at = board.find(head).unwrap_or_else(|| panic!("{head}"));
            assert!(
                board[at..at + 400].contains("_clock: &SolvedClock,"),
                "{name} {head}"
            );
        }
    }
}

/// The boot words are the contract's, distinct from every other, and each
/// fault of the bring-up writes its own in `main`.
#[test]
fn each_fault_writes_its_own_boot_word() {
    assert_eq!(
        [
            BOOT_HSE_NOT_READY,
            BOOT_PLL1_NOT_LOCKED,
            BOOT_SW_NOT_TAKEN,
            BOOT_HSE_CSS_FAULT,
            BOOT_FLASH_ACR_NOT_TAKEN
        ],
        [
            0x2100_B0F4,
            0x2100_B0F5,
            0x2100_B0F6,
            0x2100_B0F7,
            0x2100_B0F8
        ]
    );
    for name in ["hse-crystal-16", "hse-external-8"] {
        let main = &generated(name)["firmware/src/main.rs"];
        for (fault, word, value) in [
            ("HseNotReady", "BOOT_HSE_NOT_READY", 0x2100_B0F4u32),
            ("Pll1NotLocked", "BOOT_PLL1_NOT_LOCKED", 0x2100_B0F5),
            ("FlashAcrNotTaken", "BOOT_FLASH_ACR_NOT_TAKEN", 0x2100_B0F8),
            ("SwNotTaken", "BOOT_SW_NOT_TAKEN", 0x2100_B0F6),
        ] {
            assert!(
                main.contains(&format!("ClockFault::{fault} => {word},")),
                "{name} {fault}"
            );
            assert!(main.contains(&format!("const {word}: u32 = {value:#010x};")));
        }
        // The bring-up right after the reset clock, before any enable.
        let read = main.find("rcc.reset_clock(&mut r, &scale2)").unwrap();
        let start = main.find("hse.start(&mut r, clock,").unwrap();
        let first = main.find("rcc.enable_").unwrap();
        let optsr = main.find("flash.option_status(").unwrap();
        assert!(read < start && start < optsr && optsr < first, "{name}");
    }
}

/// The CSS NMI: exception 2 on an HSE board, its own word, and every motor
/// driven low as a GPIO output (TIM2's included: no break holds it).
#[test]
fn the_nmi_holds_every_motor_low() {
    for name in ["hse-crystal-16", "hse-external-8"] {
        let (solved, _) = fixture(name);
        let files = generated(name);
        let main = &files["firmware/src/main.rs"];
        assert!(main.contains("    Reset, nmi, fault, fault,"));
        let nmi = item(main, "unsafe extern \"C\" fn nmi() -> ! {", "\n}\n");
        let word = nmi.find("write_volatile(BOOT_HSE_CSS_FAULT)").unwrap();
        let hold = nmi.find("css_hold(&mut unsafe { Mmio::new() })").unwrap();
        assert!(word < hold);
        let board = &files["src/board.rs"];
        let drive = item(board, "fn css_drive<R: Registers>(r: &mut R) {", "\n}");
        let mut timers = std::collections::BTreeSet::new();
        for (net, a) in &solved.assignments {
            if !a.peripheral.starts_with("TIM") {
                continue;
            }
            timers.insert(a.peripheral.clone());
            let (port, n) = (&a.pin[1..2], a.pin[2..].parse::<u32>().unwrap());
            let block = format!("Gpio{port}");
            let low = format!(
                "r.write(TOKEN, Block::{block}, 0x18, {:#010x}); // {}",
                1u32 << (16 + n),
                a.pin
            );
            let out = format!(
                "r.modify(TOKEN, Block::{block}, 0x00, |v| (v & !{:#010x}) | {:#010x}); // {}",
                0b11u32 << (2 * n),
                0b01u32 << (2 * n),
                a.pin
            );
            let (l, o) = (
                drive
                    .find(&low)
                    .unwrap_or_else(|| panic!("{name} {net}: {low}\n{drive}")),
                drive
                    .find(&out)
                    .unwrap_or_else(|| panic!("{name} {net}: {out}")),
            );
            assert!(l < o, "{name} {net}: BSRR before MODER");
            assert!(
                board.contains(&format!("/// - `{}`, net `{net}`:", a.pin)),
                "{name} {net}"
            );
        }
        assert!(
            timers.contains("TIM2"),
            "{name}: the fixture puts motors on TIM2"
        );
    }
    // A board without HSE keeps its vector table and has no hold.
    let control = generated("sentinel-02-shaped");
    assert!(control["firmware/src/main.rs"].contains("    Reset, fault, fault, fault,"));
    assert!(!control["src/board.rs"].contains("css_hold"));
}

/// The trybuild `valid_hse_bringup` case includes this generated `board.rs`
/// by path; it must be exactly what the generator writes for the 16 MHz
/// crystal fixture. `AETHELGARD_BLESS=1` rewrites it.
#[test]
fn the_ui_hse_board_is_the_generated_one() {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/ui/hse-crystal/board.rs");
    let text = generated("hse-crystal-16")["src/board.rs"].clone();
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

/// The ledger (`AET-WO-4240b`): a row for every field the bring-up writes
/// (each register whole, so every field of it) or polls, `FLASH_ACR` written
/// rather than at reset, no select written, and no id twice.
#[test]
fn the_ledger_lists_every_field_the_bring_up_writes_or_polls() {
    use FieldAccess::{Reset, Written};
    use aethelgard_svd_hal::board::{FieldAccess, register_fields};
    for (name, bypass) in [("hse-crystal-16", 0), ("hse-external-8", 1)] {
        let (solved, table) = fixture(name);
        let h = solved.clock.hse.as_ref().unwrap();
        let rows = register_fields(&solved, &table).unwrap();
        if std::env::var_os("AETHELGARD_PRINT_ROWS").is_some() {
            for r in &rows {
                println!("{name}\t{}\t{:?}\t{:?}", r.id(), r.access, r.value);
            }
        }
        let get = |id: &str, access: FieldAccess| {
            let found: Vec<_> = rows
                .iter()
                .filter(|r| r.id() == id && r.access == access)
                .collect();
            assert_eq!(found.len(), 1, "{name}: {id} {access:?}");
            found[0].value
        };
        for (id, value) in [
            ("RCC.CR.HSEON", 1),
            ("RCC.CR.HSEBYP", bypass),
            ("RCC.CR.CSSHSEON", 1),
            ("RCC.CR.PLL1ON", 1),
            ("RCC.CR.HSION", 1),
            // The header's value names `PLLSRC_CSI` (bit 0) and `PLLSRC_HSE` (bit 1)
            // are its fields: the smaller overlap is kept (`fields_of`).
            ("RCC.PLLCKSELR.PLLSRC_HSE", (h.pllckselr >> 1) & 1),
            ("RCC.PLLCKSELR.DIVM1", (h.pllckselr >> 4) & 0x3F),
            ("RCC.PLLCFGR.PLL1RGE", (h.pllcfgr >> 2) & 0b11),
            ("RCC.PLLCFGR.DIVP1EN", (h.pllcfgr >> 16) & 1),
            ("RCC.PLL1DIVR.N1", h.pll1divr & 0x1FF),
            ("RCC.CFGR.SW", 0b011),
            ("FLASH.ACR.LATENCY", h.flash_acr & 0xF),
            ("FLASH.ACR.WRHIGHFREQ", (h.flash_acr >> 4) & 0b11),
        ] {
            assert_eq!(get(id, Written), Some(value), "{name}: {id}");
        }
        for id in ["RCC.CR.HSERDY", "RCC.CR.PLL1RDY"] {
            assert_eq!(get(id, Reset), Some(1), "{name}: {id} polled");
        }
        assert_eq!(get("RCC.CFGR.SWS", Reset), Some(0), "{name}: read at reset");
        assert!(
            !rows
                .iter()
                .any(|r| r.register == "FLASH.ACR" && r.access == Reset),
            "{name}: FLASH_ACR is written, not at reset"
        );
        assert!(
            !rows.iter().any(|r| r.access == Written
                && (r.register == "RCC.D2CCIP1R" || r.register == "RCC.D3CCIPR")),
            "{name}: a select written"
        );
        let mut ids: Vec<(String, FieldAccess)> = rows.iter().map(|r| (r.id(), r.access)).collect();
        let n = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), n, "{name}: an id twice");
    }
}

/// A start-up bound whose reads a `u32` cannot count is refused by name,
/// never wrapped: at 64,958,976 Hz the most is 66,118,149 µs (4,294,967,255 reads) (contract
/// "Bounds"); one more is refused.
#[test]
fn a_bound_past_a_u32_of_reads_is_refused() {
    use aethelgard_ir::clock::Hse;
    for (us, ok) in [
        (66_118_149u32, true),
        (66_118_150, false),
        (u32::MAX, false),
    ] {
        let (mut solved, table) = fixture("hse-crystal-16");
        let Hse::Crystal(c) = &mut solved.clock.hse.as_mut().unwrap().declared else {
            panic!("a crystal");
        };
        c.startup_max_us = us;
        let got = generate_with("hse-board", &solved, &table, &answers);
        match (ok, got) {
            (true, Ok(files)) => {
                assert!(
                    files["firmware/src/main.rs"].contains("const HSE_POLLS: u32 = 4294967255;")
                );
            }
            (false, Err(e)) => {
                assert_eq!(e.kind(), "HsePollsOverflow", "{us}");
                assert!(e.to_string().contains("HSERDY"), "{e}");
            }
            (ok, got) => panic!("{us}: expected ok={ok}, got {:?}", got.map(|_| ())),
        }
    }
}
