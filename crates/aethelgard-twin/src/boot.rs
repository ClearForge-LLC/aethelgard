//! The judge's half of a boot (`AET-WO-2100`, ruled on PR #56, R2 to R5).
//!
//! A firmware boots when three witnesses agree:
//! 1. the boot word, the first word of RAM, holds [`BOOT_MAGIC`];
//! 2. the registers the topology solves read as [`expected`] computes them;
//! 3. the core executed exactly MIPS × virtual time instructions.
//!
//! `AET-WO-2202` adds a fourth: each modelled I2C device's identity read, one
//! record per device after the boot word, judged against `aethelgard-ir`'s
//! part table ([`I2cPlan`]). `AET-WO-2204` witnesses each read by what Renode
//! records, not by what the firmware writes: the device model's own log lines
//! ([`StepCheck::witness`]); and requires the core parked in thread mode on a
//! self-loop ([`VECTACTIVE_MASK`], [`self_loops`]). `AET-WO-4210` grows the
//! one identity read into each device's script, one record per step
//! ([`StepCheck`]), and requires that the core enabled no interrupt
//! ([`SYST_CSR`], [`NVIC_ISER`]; `AET-WO-2204` A2, ruled D6).
//!
//! Every expected value here comes from the [`SolvedTopology`] and ST's
//! header, never from the firmware or from the generator that wrote it
//! (`INV-07`). The boot word's address and magic are this crate's contract,
//! which the generated linker script and runtime follow.

use std::collections::{BTreeMap, BTreeSet};

use aethelgard_ir::SolvedTopology;
use aethelgard_ir::parts;
use aethelgard_svd_hal::cmsis::{self, RegisterError};

use crate::{TwinError, kind, model, platform, split_pin};

// belt-and-suspenders: the generator holds its own copy of this constant
// (`aethelgard-svd-hal` `board/firmware.rs`), because the judge's expectations
// are never taken from the generator (`INV-07`); the boot test proves the two agree.
/// What a firmware writes to the boot word at its fixed point. Neither RAM's
/// reset value (0) nor a value the cold start writes.
pub const BOOT_MAGIC: u32 = 0x2100_B007;

// belt-and-suspenders: the generator holds its own copies of these three
// (`aethelgard-svd-hal` `board/firmware.rs`), for the reason above; the boot
// test proves the copies agree (`AET-WO-4240v` §1.4).
/// What a firmware writes to the boot word, then parks, when `PWR_CR3` reads
/// back supply bits other than the declared ones: another writer locked the
/// register since power-on (`AET-WO-4240v` §1.4).
pub const BOOT_SUPPLY_NOT_DECLARED: u32 = 0x2100_B0F1;
/// The same, when `PWR_CSR1.ACTVOSRDY` did not rise within the bound.
pub const BOOT_ACTVOSRDY_TIMEOUT: u32 = 0x2100_B0F2;
/// The same, when `PWR_D3CR.VOSRDY` did not rise within the bound.
pub const BOOT_VOSRDY_TIMEOUT: u32 = 0x2100_B0F3;

/// The name a failed boot carries when its word 0 is one of the PWR words
/// (`AET-WO-4240v` §1.4), with what the firmware found; `None` for any other
/// word. The twin's PWR never reads a wait's bit 0 and stores the first
/// `PWR_CR3` write ([`crate::platform::pwr_override`]), so a generated
/// firmware that names one in the twin has a fault of its own.
pub fn pwr_boot_word(word: u32) -> Option<(&'static str, &'static str)> {
    match word {
        BOOT_SUPPLY_NOT_DECLARED => Some((
            "boot_supply_not_declared",
            "PWR_CR3 read back supply bits other than the declared ones",
        )),
        BOOT_ACTVOSRDY_TIMEOUT => Some((
            "boot_actvosrdy_timeout",
            "PWR_CSR1.ACTVOSRDY did not rise within the firmware's bound",
        )),
        BOOT_VOSRDY_TIMEOUT => Some((
            "boot_vosrdy_timeout",
            "PWR_D3CR.VOSRDY did not rise within the firmware's bound",
        )),
        _ => None,
    }
}

/// The seed every boot gives `emulation SetSeed`, so the platform's random
/// initial values are the same in every run (spike F15).
pub const SEED: u32 = 2100;

/// The ruled bound on a boot's virtual time: 100 ms, 10⁷ instructions at
/// Renode's 100 MIPS (R5).
pub const MAX_VIRTUAL_NS: u64 = 100_000_000;

/// The length of a DShot output's evidence window, from its timer's anchor
/// (`AET-WO-2302`, ruled (a-i)): every board boots [`MAX_VIRTUAL_NS`], and
/// a DShot timer is witnessed from the first write that sets its `CEN` in
/// the boot the judge reads to this long after it, when the judge's own
/// scheduled action reads the timer back and clears
/// `CEN`. 2 ms holds four DShot600 channels' log (2.80 MB of the 8 MiB
/// budget) and memory (4.7 MiB of RSS per ms of DShot; `AET-WO-2302`
/// FEEDBACK §3); it was the board's whole boot under `AET-WO-4300a` D1.
pub const DSHOT_WINDOW_NS: u64 = 2_000_000;

/// The name of the constant that bounds every board's boot, as the verdict
/// and the campaign line state it (`AET-WO-2302`: the per-board boot window
/// is gone).
pub const WINDOW: &str = "MAX_VIRTUAL_NS";

/// The core's speed in the emitted platform: Renode 1.17.0's default, which
/// the platform never changes (spike S4: `cpu PerformanceInMips` reads 100,
/// and 10 ms executes exactly 1,000,000 instructions).
pub const MIPS: u64 = 100;

/// The quantum every boot runs at, in nanoseconds (`AET-WO-4300` D2): Renode
/// 1.17.0's own default (`TimeSourceBase.DefaultQuantum`, 100 µs), stated by
/// the CLI's composition. A line the CPU causes carries the start of its
/// quantum, unless the core has just accessed a timer, which brings the
/// clock to that instant (`AET-WO-4300a` FEEDBACK M4): an edge off this grid
/// is D4's third part of the witness, never proof alone.
pub const QUANTUM_NS: u64 = 100_000;

/// A half-open address range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub start: u64,
    pub end: u64,
}

impl Region {
    /// True when `[address, address + len)` lies inside the region.
    pub fn holds(&self, address: u64, len: u64) -> bool {
        address >= self.start && address.checked_add(len).is_some_and(|e| e <= self.end)
    }
}

/// Where a firmware may load: the part's flash and its DTCM RAM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Memory {
    /// `FLASH_BANK1_BASE` to `FLASH_END`, both banks.
    pub flash: Region,
    /// `D1_DTCMRAM_BASE`, of the size its comment states.
    pub ram: Region,
}

// belt-and-suspenders: the generator reads the same macros for its linker
// script (`board/firmware.rs`); the judge reads them itself (`INV-07`).
/// The part's memory, from the header.
pub fn memory() -> Result<Memory, TwinError> {
    let f = cmsis::register_facts()?;
    let flash_start = u64::from(f.value("FLASH_BANK1_BASE")?);
    let flash_end = u64::from(f.value("FLASH_END")?) + 1;
    let ram_start = u64::from(f.value("D1_DTCMRAM_BASE")?);
    let ram_end = ram_start + u64::from(f.region_kib("D1_DTCMRAM_BASE")?) * 1024;
    Ok(Memory {
        flash: Region {
            start: flash_start,
            end: flash_end,
        },
        ram: Region {
            start: ram_start,
            end: ram_end,
        },
    })
}

/// The boot word's address: the first word of RAM.
pub fn boot_word() -> Result<u32, TwinError> {
    let start = memory()?.ram.start;
    Ok(u32::try_from(start).expect("D1_DTCMRAM_BASE is a 32-bit value"))
}

/// One register the judge reads, and what it must hold under `mask`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Register {
    /// `GPIOA.MODER`, `RCC.AHB4ENR`.
    pub name: String,
    pub address: u32,
    pub mask: u32,
    pub value: u32,
}

// belt-and-suspenders: the generator holds its own copy of the record layout
// and the status codes (`aethelgard-svd-hal` `board/firmware.rs`), so the
// judge reads the records as it defines them (`INV-07`); `tests/board.rs`
// proves the two agree.
/// Where the first I2C record starts, from the boot word: after the boot word
/// (word 0) and the brown-out word (word 1, [`BROWN_OUT_OFFSET`];
/// `AET-WO-4230` D10).
pub const RECORD_OFFSET: u32 = 8;

/// Where the brown-out word sits, from the boot word: `FLASH_OPTSR_CUR`
/// exactly as the firmware read it, once, early in `main` (`AET-WO-4230`
/// D7, D10).
pub const BROWN_OUT_OFFSET: u32 = 4;

/// One record is the status word, then the bytes read in as many words as
/// they need, at least one, the first byte on the wire as the most
/// significant of the first word: a read of up to four bytes is two words, as
/// every identity read is (`AET-WO-4210`).
pub const RECORD_BYTES: u32 = 8;

/// The words a record's value takes for a read of `width` bytes.
pub fn value_words(width: u8) -> u32 {
    u32::from(width).div_ceil(4).max(1)
}

/// What the judge writes to both words of every record before the core runs,
/// as it does the boot word, so a record is only ever what the firmware wrote.
pub const RECORD_UNSET: u32 = 0xFFFF_FFFF;

/// The statuses a firmware writes, each a word no reset value or stray write
/// is likely to be.
pub const STATUS_OK: u32 = 0x2202_0000;
pub const STATUS_TXIS: u32 = 0x2202_0001;
pub const STATUS_TC: u32 = 0x2202_0002;
pub const STATUS_RXNE: u32 = 0x2202_0003;
pub const STATUS_STOP: u32 = 0x2202_0004;
/// An SPI read whose bounded wait for `SR.RXP` expired (`AET-WO-4200`).
pub const STATUS_RXP: u32 = 0x2202_0005;
/// An SPI read whose bounded wait for `SR.EOT` expired before the firmware
/// cleared `SPE` (RM0433 Rev 9 §50.4.12; `AET-WO-4230` B0).
pub const STATUS_EOT: u32 = 0x2202_0006;

/// Renode's name of the embedded flash memory's controller in the stock
/// platform (`flashController: MTD.STM32H7_FlashController @ sysbus
/// 0x52002000`, [`platform::STOCK_FILE`] line 66): the composition logs its
/// accesses on every boot (`AET-WO-4230` D10).
pub const FLASH_RENODE: &str = "flashController";

/// The `FLASH_OPTCR` unlock keys, written in this order to `FLASH_OPTKEYR`
/// (RM0433 Rev 9 §4.5.1, p. 182).
pub const OPTKEY1: u32 = 0x0819_2A3B;
pub const OPTKEY2: u32 = 0x4C5D_6E7F;

/// What Renode 1.17.0's `FLASH_OPTSR_CUR` reads at reset (measured,
/// `AET-WO-4230` FEEDBACK §2; `STM32H7_FlashController.cs` L118–119):
/// `BOR_LEV` `00`. It is not RM0433 Rev 9 Table 21's factory word (p. 177),
/// whose bit 26 is 0 and whose bits 28:27 and 20:19 are part-dependent: the
/// presentation keeps the model's other bits and replaces `BOR_LEV` alone.
pub const RENODE_OPTSR_RESET: u32 = 0x0406_AAF0;

/// The brown-out level a boot of the topology must record, and where
/// (`AET-WO-4230` §1.6, D10): the declared level (`topology.bor_level`,
/// `RUL-16`), the word in RAM, and the flash controller's registers, every
/// address and position from ST's header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrownOut {
    pub level: u8,
    /// The boot record's word 1: the boot word plus [`BROWN_OUT_OFFSET`].
    pub word_at: u32,
    /// `FLASH_OPTSR_BOR_LEV_Msk` (bits 3:2) and its position.
    pub mask: u32,
    pub shift: u32,
    /// `FLASH_R_BASE`.
    pub base: u32,
    /// The offsets in `FLASH_TypeDef` of `OPTKEYR`, `OPTCR`, `OPTSR_CUR` and
    /// `OPTSR_PRG`.
    pub optkeyr: u32,
    pub optcr: u32,
    pub optsr_cur: u32,
    pub optsr_prg: u32,
    /// `FLASH_OPTCR_OPTSTART`.
    pub optstart: u32,
}

/// How the boot record's brown-out word reads against the declared level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrownOutOutcome {
    Pass,
    /// Still the judge's fill: the firmware never recorded `OPTSR_CUR`. Its
    /// `BOR_LEV` bits read 3, so it is named before they are compared.
    Unset,
    /// The recorded `BOR_LEV` is not the declared level.
    Mismatch {
        recorded: u8,
    },
}

impl BrownOut {
    /// The `BOR_LEV` field the twin presents: `level` in place of the
    /// model's `00`.
    pub fn expected(&self) -> u32 {
        u32::from(self.level) << self.shift
    }

    /// The word the platform programs into `OPTSR_PRG` and so presents in
    /// `OPTSR_CUR`: [`RENODE_OPTSR_RESET`] with `BOR_LEV` set to the level.
    pub fn presented(&self) -> u32 {
        (RENODE_OPTSR_RESET & !self.mask) | self.expected()
    }

    /// Word 1 of the boot record, judged.
    pub fn judge(&self, word: u32) -> BrownOutOutcome {
        if word == RECORD_UNSET {
            BrownOutOutcome::Unset
        } else if word & self.mask == self.expected() {
            BrownOutOutcome::Pass
        } else {
            BrownOutOutcome::Mismatch {
                recorded: u8::try_from((word & self.mask) >> self.shift).unwrap_or(u8::MAX),
            }
        }
    }

    /// The platform's `init add:` block on the flash controller that leaves
    /// `OPTSR_CUR` at [`Self::presented`], as a factory step would
    /// (`AET-WO-4230` FEEDBACK §2, m1, measured): the two option keys, the
    /// word into `OPTSR_PRG`, `OPTSTART`, and last a wrong key, which leaves
    /// `OPTCR` locked (`OPTLOCK` reads 1), as silicon resets. Emitted for
    /// every level, 0 included.
    pub fn presentation(&self) -> String {
        format!(
            "\n// The brown-out level the specification declares (mcu.bor_level {level}), presented in\n\
             // FLASH_OPTSR_CUR as a factory step leaves it (AET-WO-4230 §1.6): the option keys, the\n\
             // word into FLASH_OPTSR_PRG, OPTSTART, then a wrong key to lock FLASH_OPTCR again.\n\
             {FLASH_RENODE}:\n    init add:\n\
             \x20       WriteDoubleWord 0x{keyr:02X} 0x{k1:08X}\n\
             \x20       WriteDoubleWord 0x{keyr:02X} 0x{k2:08X}\n\
             \x20       WriteDoubleWord 0x{prg:02X} 0x{word:08X}\n\
             \x20       WriteDoubleWord 0x{cr:02X} 0x{start:08X}\n\
             \x20       WriteDoubleWord 0x{keyr:02X} 0x00000000\n",
            level = self.level,
            keyr = self.optkeyr,
            k1 = OPTKEY1,
            k2 = OPTKEY2,
            prg = self.optsr_prg,
            word = self.presented(),
            cr = self.optcr,
            start = self.optstart,
        )
    }

    /// What the flash controller's access log of each boot shows that the
    /// bring-up never does, each once, in the order first seen
    /// (`AET-WO-4230` D10, WO §5.3): any write, at any offset or width (a
    /// firmware cannot program the twin's option bytes itself); any read but
    /// a word read of `OPTSR_CUR`; more than one such read in a boot; and,
    /// in the boot the reads judge (the last), not exactly one; and, where
    /// that boot read it once, a `word` (the boot record's word 1) other
    /// than, in all 32 bits, the value that read returned: a firmware that
    /// records a word it did not read, such as the declared level, would
    /// otherwise pass wherever the level is presented (the adversarial
    /// pass's F2). A `word` still at [`RECORD_UNSET`] is [`Self::judge`]'s
    /// to name. `boots` are [`crate::serial::Access`]es of [`FLASH_RENODE`]
    /// alone.
    pub fn judge_accesses(&self, boots: &[Vec<crate::serial::Access>], word: u32) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut note = |x: String| {
            if !out.contains(&x) {
                out.push(x);
            }
        };
        for (k, boot) in boots.iter().enumerate() {
            let mut reads = 0usize;
            let mut returned = 0u64;
            for a in boot.iter().filter(|a| a.source == FLASH_RENODE) {
                if a.write {
                    note(format!(
                        "the firmware wrote the flash controller ({} bits at {:#X}, value {:#X})",
                        a.bits, a.offset, a.value
                    ));
                } else if a.bits != 32 || a.offset != self.optsr_cur {
                    note(format!(
                        "the firmware read the flash controller other than FLASH_OPTSR_CUR whole ({} bits at {:#X})",
                        a.bits, a.offset
                    ));
                } else {
                    reads += 1;
                    returned = a.value;
                }
            }
            let last = k + 1 == boots.len();
            if last && reads == 1 && word != RECORD_UNSET && u64::from(word) != returned {
                note(format!(
                    "the boot record's word 1 is {word:#010X}, not {returned:#010X}, the value the firmware's one read of FLASH_OPTSR_CUR returned"
                ));
            }
            if reads > 1 || (last && reads != 1) {
                note(format!(
                    "the firmware read FLASH_OPTSR_CUR {reads} times in {}, not once",
                    if last {
                        "the boot the reads judge".to_owned()
                    } else {
                        format!("boot {k}")
                    }
                ));
            }
        }
        out
    }
}

/// The brown-out level a boot of `topology` must record (`AET-WO-4230`
/// D10), with the flash controller's registers from the header.
pub fn brown_out(topology: &SolvedTopology) -> Result<BrownOut, TwinError> {
    let f = cmsis::register_facts()?;
    let flash = f.layout("FLASH")?;
    let (shift, width) = f.field("FLASH_OPTSR_BOR_LEV")?;
    let mask = ((1u32 << width) - 1) << shift;
    if u32::from(topology.bor_level) << shift & !mask != 0 {
        return Err(TwinError::InvalidTopology(
            aethelgard_ir::IrValidationError::BorLevelOutOfRange {
                value: topology.bor_level,
            },
        ));
    }
    Ok(BrownOut {
        level: topology.bor_level,
        word_at: boot_word()? + BROWN_OUT_OFFSET,
        mask,
        shift,
        base: f.base("FLASH_R")?,
        optkeyr: flash.offset("OPTKEYR", 0)?,
        optcr: flash.offset("OPTCR", 0)?,
        optsr_cur: flash.offset("OPTSR_CUR", 0)?,
        optsr_prg: flash.offset("OPTSR_PRG", 0)?,
        optstart: 1 << f.position("FLASH_OPTCR_OPTSTART_Pos")?,
    })
}
/// SysTick's Control and Status Register (ARMv7-M Architecture Reference
/// Manual, B3.3.2) and the NVIC's Interrupt Set-Enable Registers (B3.4.4):
/// after the boot word, every one must read 0, because the generated
/// firmware arms no interrupt, so "parked" claims none runs
/// (`AET-WO-2204` A2, ruled D6 on PR #131; `RUL-19` (5)).
pub const SYST_CSR: u32 = 0xE000_E010;
pub const NVIC_ISER: u32 = 0xE000_E100;
/// The `SYST_CSR` bits judged: `ENABLE`, `TICKINT` and `COUNTFLAG`. Renode's
/// `CLKSOURCE` reads 1 whatever the firmware writes (`NVIC.cs` L1040, read at
/// `066a7f13`), so the whole word is never 0 in the twin; these three say no
/// SysTick is armed and none has counted to 0 (`AET-WO-4210` F-A2).
pub const SYST_CSR_ARMED: u32 = 0x0001_0003;

/// The Cortex-M7's Interrupt Control and State Register, and its `VECTACTIVE`
/// field: the exception the core is handling, 0 in thread mode (ARMv7-M
/// Architecture Reference Manual, B3.2.4). Renode's NVIC model computes it
/// from its own state (`NVIC.cs` L1077), so no firmware store sets it.
pub const ICSR: u32 = 0xE000_ED04;
pub const VECTACTIVE_MASK: u32 = 0x1FF;

/// Thumb `yield`, then `b.n` back to it: the two halfwords
/// `loop { core::hint::spin_loop() }` compiles to (`AET-WO-2204` S3, measured
/// at `0x08000594` in the reference firmware). A pass claims the core parked
/// on such a loop after the boot word, not that the loop is the generator's
/// fixed point: a firmware that writes the boot word and then parks on
/// another thread-mode self-loop also passes (ruled D3, a named residual).
/// Nor does it claim that no interrupt runs after the boot word (adversarial
/// A2: a SysTick handler that returns leaves the core on the loop).
pub const YIELD: u16 = 0xBF10;
pub const B_N_BACK: u16 = 0xE7FD;

/// The address of every `yield; b.n` self-loop in `code`, loaded at `base`:
/// the core is on the loop when its `PC` is one of these or two past it.
pub fn self_loops(base: u64, code: &[u8]) -> Vec<u64> {
    let half = |i: usize| u16::from_le_bytes([code[i], code[i + 1]]);
    (0..code.len().saturating_sub(3))
        .step_by(2)
        .filter(|&i| half(i) == YIELD && half(i + 2) == B_N_BACK)
        .filter_map(|i| base.checked_add(i as u64))
        .collect()
}

/// Where a device sits on its bus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wire {
    /// An I2C device at its 7-bit address.
    I2c { address: u8 },
    /// An SPI device at its multiplexer address, selected by the pin `select`
    /// (`PA0`), driven low (`AET-WO-4200` D1).
    Spi { address: u8, select: String },
}

impl Wire {
    /// How a finding names where the device is: `at I2C address 0x76`, or
    /// `on SPI1, chip select PA0`.
    pub fn describe(&self, peripheral: &str) -> String {
        match self {
            Wire::I2c { address } => format!("at I2C address {address:#04x}"),
            Wire::Spi { select, .. } => format!("on {peripheral}, chip select {select}"),
        }
    }
}

/// One step of a device's script the judge expects (`AET-WO-4210` S2), from
/// the parts table alone: the identity read, then a scripted part's
/// configuration write and data read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepCheck {
    pub reference: String,
    /// ST's name of the controller the device is on, such as `I2C1`.
    pub peripheral: String,
    pub wire: Wire,
    pub step: parts::StepKind,
    /// The step's name among the device's steps ([`parts::Step::label`]): what
    /// every verdict, record and finding calls it (`AET-WO-4220`).
    pub label: &'static str,
    /// The bytes written first: an I2C register or command, or an SPI command.
    pub write: Vec<u8>,
    pub width: u8,
    /// What the part answers, under `mask`: the identity, or the datasheet's
    /// conversion of the stimulus (D3). A configuration reads nothing.
    pub expected: u64,
    pub mask: u64,
    /// The record in RAM: its status word, then [`value_words`] words.
    pub status_at: u32,
    pub value_at: u32,
    /// The level the judge sets the device's log to, and its witness lines'
    /// prefix ([`model::Model::level`]).
    pub level: (i8, &'static str),
    /// The model's path in Renode: `i2c1.u2` (the bus's stock name, then the
    /// device's emitted name), or `spi1.spi1mux.u4` (with the multiplexer).
    pub device: String,
    /// What Renode itself records when the controller runs this read on the
    /// model (`AET-WO-2204` D1): the model's whole log lines, after the host
    /// timestamp, in order. Only a transfer the controller runs to the
    /// device's address calls the model, so no store writes these lines; with
    /// them, the `[WARNING]` lines a correct step makes the model write,
    /// pinned to source (`AET-WO-4210` D4). All
    /// of it comes from the part table and the emitter, never from the
    /// firmware (`INV-07`). `None` when the model has no known witness format:
    /// a named failure, never a skip (D2).
    pub witness: Option<Vec<String>>,
    /// The model's edge artefact, allowed only directly after the
    /// multiplexer's finish line for the device ([`model::Model::idle_warning`]).
    pub idle_warning: Option<String>,
    /// The model's `Noisy` lines that are not a transfer, which the judge
    /// skips; any other line from the device fails (`AET-WO-4200` D2).
    pub skipped: Vec<String>,
    /// What the judge sets on the model before the run, for a data read: each
    /// channel's property and value (D3). `None` when the model has no
    /// property for a channel, so the read cannot be judged.
    pub stimulus: Option<Vec<(&'static str, i32)>>,
}

/// A device the twin has no model for: reported, never passed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unmodelled {
    pub reference: String,
    pub part: String,
    pub peripheral: String,
    pub wire: Wire,
    /// Its identity read's record, when its part has a row in the table: the
    /// firmware makes the read whatever the twin models (`AET-WO-4241`),
    /// after every check's step, so no check's record moves for it (the
    /// architect's D3). `None` for a part with no row, which no firmware
    /// reads.
    pub identity: Option<NotWitnessable>,
}

/// An unmodelled device's identity read, which the firmware makes and the
/// twin cannot witness (`AET-WO-4241`): no model answers it, so what the
/// record holds is neither a pass nor a failure. Its place comes from the
/// parts table and this crate's record layout, never from the firmware
/// (`INV-07`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotWitnessable {
    /// The bytes written first: the identity register or command.
    pub write: Vec<u8>,
    pub width: u8,
    /// What the part answers on silicon, under `mask` (the table's identity),
    /// which no model in the twin answers.
    pub expected: u64,
    pub mask: u64,
    /// The record in RAM: its status word, then [`value_words`] words.
    pub status_at: u32,
    pub value_at: u32,
    /// The path Renode would give its model (`i2c1.u3`, `spi1.spi1mux.u4`),
    /// derived as a check's [`StepCheck::device`]: no element is declared
    /// there, and the judge reads its controller or multiplexer from it.
    pub device: String,
}

/// What a boot must leave for the devices.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IdentityPlan {
    /// One per step of each device whose part has a model: every identity
    /// and configuration step, then every data read (`AET-WO-4230` R6, the
    /// architect's ruling on Q-E: one start-up wait between them), each half
    /// with the I2C devices in the order they are declared, then the SPI
    /// devices in theirs, and each device's steps in its script's order; the
    /// record index is the position.
    pub checks: Vec<StepCheck>,
    /// Every device with no model, in the order above; each identity read's
    /// record after the last check's, in that order (`AET-WO-4241`, D3).
    pub unmodelled: Vec<Unmodelled>,
}

/// How one record reads against its check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Pass,
    /// The transaction finished and the value is not the part's.
    Mismatch,
    /// The firmware's bounded wait expired; the stage names where.
    Timeout(&'static str),
    /// Neither a status the firmware writes nor the judge's own fill: the
    /// record was never written, or written with something else.
    Invalid,
}

impl StepCheck {
    /// The status word and value, judged. Pass needs both: a status of OK and
    /// exactly the table's value under its mask.
    pub fn judge(&self, status: u32, value: u64) -> Outcome {
        match self.wire.status(status) {
            Some(None)
                if value & !parts::width_mask(self.width) == 0
                    && value & self.mask == self.expected & self.mask =>
            {
                Outcome::Pass
            }
            Some(None) => Outcome::Mismatch,
            Some(Some(stage)) => Outcome::Timeout(stage),
            None => Outcome::Invalid,
        }
    }
}

impl Wire {
    /// What a record's status word says of a read on this bus: `Some(None)`
    /// when the transaction finished, `Some(Some(stage))` when the
    /// firmware's bounded wait for `stage` expired, and `None` for any word a
    /// read on this bus never writes. A status counts only on its own bus: an
    /// I2C read never writes `RXP`, and an SPI read none of the I2C waits.
    pub fn status(&self, status: u32) -> Option<Option<&'static str>> {
        let spi = matches!(self, Wire::Spi { .. });
        match (status, spi) {
            (STATUS_OK, _) => Some(None),
            (STATUS_TXIS, false) => Some(Some("TXIS")),
            (STATUS_TC, false) => Some(Some("TC")),
            (STATUS_RXNE, false) => Some(Some("RXNE")),
            (STATUS_STOP, false) => Some(Some("STOPF")),
            (STATUS_RXP, true) => Some(Some("RXP")),
            (STATUS_EOT, true) => Some(Some("EOT")),
            _ => None,
        }
    }
}

/// The steps a boot of the topology's devices must make: every identity and
/// configuration step, then every data read (`AET-WO-4230` R6: the firmware
/// waits once, for the largest start-up, between the two), each half with
/// the I2C devices in the specification's order, then the SPI devices in
/// theirs, each device's script in order ([`parts::Part::steps`]); and the
/// devices it cannot check. The steps are the parts table's; the device is
/// modelled when [`model::kind`] says so.
pub fn identity(topology: &SolvedTopology) -> Result<IdentityPlan, TwinError> {
    let base = boot_word()?;
    let mut plan = IdentityPlan::default();
    let push = |plan: &mut IdentityPlan,
                reference: &str,
                peripheral: &str,
                part: &str,
                wire: Wire,
                device: &dyn Fn() -> Result<String, TwinError>|
     -> Result<(), TwinError> {
        let table = parts::table().map_err(TwinError::InvalidTopology)?;
        let Some(model) = model::kind(part)? else {
            // The firmware reads only the identity of a part with no model
            // (`AET-WO-4241`), and nothing of a part with no row.
            let identity = match table.get(part) {
                Some(p) => Some((p, device()?)),
                None => None,
            };
            let identity = identity
                .and_then(|(p, device)| {
                    p.steps()
                        .into_iter()
                        .find(|s| s.kind == parts::StepKind::Identity)
                        .map(|id| (id, device))
                })
                .map(|(id, device)| NotWitnessable {
                    write: id.write,
                    width: id.width,
                    expected: id.expected,
                    mask: id.mask,
                    // Placed below, with the checks.
                    status_at: 0,
                    value_at: 0,
                    device,
                });
            plan.unmodelled.push(Unmodelled {
                reference: reference.to_owned(),
                part: part.to_owned(),
                peripheral: peripheral.to_owned(),
                wire,
                identity,
            });
            return Ok(());
        };
        let Some(p) = table.get(part) else {
            return Ok(());
        };
        let device = device()?;
        for step in p.steps() {
            let stimulus = match (&p.script, step.kind) {
                (Some(s), parts::StepKind::Data) => s
                    .channels
                    .iter()
                    .map(|c| model.stimulus(c.quantity).map(|prop| (prop, c.stimulus)))
                    .collect::<Option<Vec<_>>>(),
                _ => Some(Vec::new()),
            };
            plan.checks.push(StepCheck {
                reference: reference.to_owned(),
                peripheral: peripheral.to_owned(),
                wire: wire.clone(),
                step: step.kind,
                label: step.label,
                width: step.width,
                expected: step.expected,
                mask: step.mask,
                // Placed below, once the steps are in the boot's order.
                status_at: 0,
                value_at: 0,
                level: model.level(),
                // A data read whose stimulus the model cannot be set to has
                // no expected value the twin can witness (D3).
                witness: model.witness(&device, &step).filter(|_| stimulus.is_some()),
                idle_warning: model.idle_warning(&device),
                skipped: model.skipped(&device),
                stimulus,
                device: device.clone(),
                write: step.write,
            });
        }
        Ok(())
    };
    for (reference, d) in topology.i2c_devices_in_order() {
        push(
            &mut plan,
            reference,
            &d.peripheral,
            &d.part,
            Wire::I2c { address: d.address },
            &|| platform::device_path(&d.peripheral, reference),
        )?;
    }
    for s in platform::spi_wiring(topology)? {
        push(
            &mut plan,
            &s.reference,
            &s.peripheral,
            &s.part,
            Wire::Spi {
                address: s.address,
                select: s.select.clone(),
            },
            &|| platform::spi_device_path(&s.peripheral, &s.reference),
        )?;
    }
    // Every identity and configuration step, then every data read, each half
    // in the order above (a stable sort), then each record after the last;
    // then each unmodelled device's identity record, after every check's
    // (`AET-WO-4241`, the architect's D3).
    plan.checks.sort_by_key(|c| c.step == parts::StepKind::Data);
    let mut next = RECORD_OFFSET;
    let mut place = |reference: &str, width: u8| -> Result<u32, TwinError> {
        let overflow = || TwinError::InvalidDeviceName {
            reference: reference.to_owned(),
        };
        let at = next;
        next = at
            .checked_add(4 * (1 + value_words(width)))
            .ok_or_else(overflow)?;
        base.checked_add(at).ok_or_else(overflow)
    };
    for c in &mut plan.checks {
        c.status_at = place(&c.reference, c.width)?;
        c.value_at = c.status_at + 4;
    }
    for u in &mut plan.unmodelled {
        if let Some(n) = u.identity.as_mut() {
            n.status_at = place(&u.reference, n.width)?;
            n.value_at = n.status_at + 4;
        }
    }
    Ok(plan)
}

/// What a boot of `topology` must leave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    /// Every RCC enable register the header defines (`AET-WO-4300a` §1.6,
    /// 2100 F24), each with silicon's reset value as `value`: the boot writes
    /// it before it starts, because Renode resets them with every clock on
    /// (spike F10).
    pub cold: Vec<Register>,
    /// Every register to compare after the run, in address order: the mode
    /// and alternate function of each solved pin, and each cold register in
    /// full, which must hold exactly the solved enable bits.
    pub registers: Vec<Register>,
    /// Solved fields Renode 1.17.0 does not store, so no read can judge them
    /// (spike F11): `OTYPER` of every port with a solved pin, which the
    /// bring-up writes whole (`AET-WO-4230` D5, B0), an I2C's `TIMINGR`;
    /// a timer's `BDTR.MOE` and each channel's `OCxPE` (`AET-WO-4300a`);
    /// on a generated platform, `PWR_CSR1.ACTVOSRDY` and `PWR_D3CR.VOSRDY`,
    /// which its PWR reads 1 whatever happens (`AET-WO-4240v` §1.6).
    pub unobservable: Vec<String>,
    /// The identity reads of the devices (`AET-WO-2202`, `AET-WO-4200`).
    pub identity: IdentityPlan,
    /// The brown-out word the boot must record (`AET-WO-4230` D10).
    pub brown_out: BrownOut,
}

/// The enable registers whose reset value is not 0 (RM0433 Rev 9 Tables
/// 66–74): `RCC_APB4ENR` resets with `RTCAPBEN` (bit 16) set. Every other
/// `*ENR` of the header resets to 0.
pub const ENR_RESET: [(&str, u32); 1] = [("APB4ENR", 0x0001_0000)];

/// Each solved timer's registers after the boot, from the topology alone
/// (`AET-WO-4300a` §1.4, `INV-07`): the fields Renode 1.17.0's
/// `STM32_Timer` stores, each with the value the bring-up writes. `CR1`:
/// `CEN | ARPE` under the stored bits 0–7 (L143–165; `CKD` is a tag). `DIER`:
/// 0 under its stored interrupt enables (bits 0–4 and 6; the DMA enables
/// are tags), so a firmware that arms an update interrupt fails here.
/// `CCMR1` and `CCMR2`: each used channel `OCxM` = 110 (PWM mode 1) and
/// `CCxS` = 00 (output), under the stored `CCxS` and `OCxM` bits (L425–445;
/// `OCxFE`, `OCxPE` and `OCxCE` are tags). `CCER`: each used channel's
/// `CCxE`, polarity 0, under the stored `CCxE`, `CCxP` and `CCxNP` bits
/// (`CCxNE` is a tag). `PSC`, `ARR` and each used channel's `CCRx` in full.
/// `CR2.TI1S`, `SMCR`'s `SMS` and `TS`, and `RCR` (where the timer has one)
/// at their reset value 0, which the bring-up never changes. `BDTR` is all
/// tags, so `MOE` and `BKE` are listed as unobservable, with `SMCR.ECE`. An unused
/// channel's `CCRx` is not written and not read: Renode resets it to
/// `0xFFFF` where silicon resets it to 0 (measured, FEEDBACK P8.1).
fn timer_registers(
    topology: &SolvedTopology,
    f: &cmsis::RegisterFacts,
    field: &mut impl FnMut(u32, String, u32, u32),
) -> Result<(), TwinError> {
    let tim = f.layout("TIM")?;
    // Each timer's shared values, and each used channel with its CCR.
    let mut timers = BTreeMap::new();
    for (net, m, a) in topology.motor_outputs_in_order() {
        let (timer, channel) = aethelgard_ir::motor::timer_channel(&a.signal).ok_or_else(|| {
            TwinError::UnsupportedPeripheral {
                peripheral: format!("{} ({net})", a.signal),
            }
        })?;
        timers
            .entry(timer)
            .or_insert((m, Vec::new()))
            .1
            .push((channel, m.ccr));
    }
    for (timer, (m, channels)) in timers {
        let name = format!("TIM{timer}");
        let base = f.base(&name)?;
        let mut reg = |r: &str, mask: u32, value: u32| -> Result<(), TwinError> {
            field(base + tim.offset(r, 0)?, format!("{name}.{r}"), mask, value);
            Ok(())
        };
        reg("CR1", 0xFF, CR1_ARPE | CR1_CEN)?;
        // `CR2.TI1S`, and `SMCR`'s slave mode and trigger (L175, L186–207),
        // at reset: a gated or trigger slave mode keeps the model counting
        // while it stops silicon's counter, or stops the model's with `CEN`
        // still set (adversarial H1, H2).
        reg("CR2", 0x80, 0)?;
        reg("SMCR", 0x77, 0)?;
        reg("DIER", 0x5F, 0)?;
        let (mut ccmr, mut ccer) = ([0u32; 2], 0u32);
        for &(c, _) in &channels {
            let half = (c - 1) % 2;
            ccmr[((c - 1) / 2) as usize] |= OCM_PWM1 << (4 + 8 * half);
            ccer |= 1 << (4 * (c - 1));
        }
        reg("CCMR1", 0x7373, ccmr[0])?;
        reg("CCMR2", 0x7373, ccmr[1])?;
        reg("CCER", 0xBBBB, ccer)?;
        reg("PSC", u32::MAX, m.psc)?;
        reg("ARR", u32::MAX, m.arr)?;
        // The repetition counter, at reset, on the timers that have one (the
        // same set as `MOE`'s, RM0433 Rev 9 §38.4.15, §41.5.13, §41.6.12).
        if aethelgard_ir::motor::has_main_output_enable(timer) {
            reg("RCR", 0xFF, 0)?;
        }
        for &(c, ccr) in &channels {
            reg(&format!("CCR{c}"), u32::MAX, ccr)?;
        }
    }
    Ok(())
}

/// Each serial port's USART after the boot, from the topology alone
/// (`AET-WO-4300b` §1.5, `INV-07`): the fields Renode 1.17.0's
/// `UART.STM32F7_USART` stores, each with the value the bring-up leaves.
/// `CR1`: `UE | RE | TE`, `PCE` with any parity and `PS` for odd, under the
/// stored bits (`UE`, `RE`, `TE`, `RXNEIE`, `TCIE`, `TXEIE`, `PEIE`, `PS`,
/// `PCE`, `OVER8`, `RTOIE`; `STM32F7_USART.cs` L144–160, L317–318), so an
/// interrupt enable or `OVER8` set fails here. `CR2`: `STOP` (L175, the only
/// stored field). `CR3`: 0 under `DMAR` and `DMAT` (L186–187). `BRR`: the
/// solved value in its 16 stored bits (L210). Positions from ST's header.
/// The tags the framing sets or relies on (`CR1.M0`, `M1`, `CR2.TXINV`,
/// `RXINV`, `CR3.ONEBIT`; L162, L167, L177–178, L347), and `CR1.FIFOEN`,
/// which the bring-up sets (bits 29–31 reserved in the model; measured, fix
/// 2 f1), are stored nowhere and listed as unobservable here: the judge sees
/// them only in the words Renode's access log shows written
/// (`crate::serial::judge_writes`).
fn serial_registers(
    topology: &SolvedTopology,
    f: &cmsis::RegisterFacts,
    field: &mut impl FnMut(u32, String, u32, u32),
    unobservable: &mut Vec<String>,
) -> Result<(), TwinError> {
    let usart = f.layout("USART")?;
    let bit = |name: &str| -> Result<u32, TwinError> { Ok(1 << f.position(name)?) };
    let [cr1_stored, cr2_stored, cr3_stored] = crate::serial::usart_stored(f)?;
    for (_, port, _, _) in topology.serial_ports_in_order() {
        let name = &port.peripheral;
        kind(name)?;
        let base = f.base(name)?;
        let mut reg = |r: &str, mask: u32, value: u32| -> Result<(), TwinError> {
            field(
                base + usart.offset(r, 0)?,
                format!("{name}.{r}"),
                mask,
                value,
            );
            Ok(())
        };
        let (pce, ps) = crate::serial::parity_bits(&port.framing);
        let mut cr1 =
            bit("USART_CR1_UE_Pos")? | bit("USART_CR1_RE_Pos")? | bit("USART_CR1_TE_Pos")?;
        if pce {
            cr1 |= bit("USART_CR1_PCE_Pos")?;
        }
        if ps {
            cr1 |= bit("USART_CR1_PS_Pos")?;
        }
        reg("CR1", cr1_stored, cr1)?;
        let stop = port.framing.stop_field().ok_or_else(|| {
            TwinError::InvalidTopology(aethelgard_ir::IrValidationError::SerialPortMismatch {
                id: name.clone(),
            })
        })?;
        reg(
            "CR2",
            cr2_stored,
            u32::from(stop) << f.field("USART_CR2_STOP")?.0,
        )?;
        reg("CR3", cr3_stored, 0)?;
        reg("BRR", 0xFFFF, port.brr)?;
        for tag in [
            "CR1.FIFOEN",
            "CR1.M0",
            "CR1.M1",
            "CR2.RXINV",
            "CR2.TXINV",
            "CR3.ONEBIT",
        ] {
            unobservable.push(format!("{name}.{tag}"));
        }
    }
    Ok(())
}

/// `TIMx_CR1.CEN` and `ARPE` (RM0433 Rev 9 §38.4.1).
const CR1_CEN: u32 = 1;
const CR1_ARPE: u32 = 1 << 7;
/// `OCxM` = 110, PWM mode 1 (RM0433 Rev 9 §38.4.8, the output-compare view).
const OCM_PWM1: u32 = 0b110;

// belt-and-suspenders: the generator's `board.rs` states these too; the
// judge computes the expected image without the generator's code (`INV-07`).
/// MODER values (RM0433 §11.4.1), the two roles the topology solves.
const MODE_OUTPUT: u32 = 0b01;
const MODE_AF: u32 = 0b10;

/// The expected register image of `topology`.
pub fn expected(topology: &SolvedTopology) -> Result<Image, TwinError> {
    topology.validate().map_err(TwinError::InvalidTopology)?;
    let identity = identity(topology)?;
    let f = cmsis::register_facts()?;
    let gpio = f.layout("GPIO")?;
    let rcc = f.layout("RCC")?;
    let rcc_base = f.base("RCC")?;

    // Pin name to (port, index, Some(af) for a peripheral pin, None for GPIO).
    let mut pins: BTreeMap<&str, (char, u32, Option<u32>)> = BTreeMap::new();
    let mut gates: BTreeSet<String> = BTreeSet::new();
    let mut unobservable = Vec::new();
    for a in topology.assignments.values() {
        let (port, index) = split_pin(&a.pin)?;
        pins.insert(&a.pin, (port, index, Some(u32::from(a.alternate_function))));
        kind(&a.peripheral)?;
        gates.insert(a.peripheral.clone());
        gates.insert(format!("GPIO{port}"));
        if a.peripheral.starts_with("I2C") {
            unobservable.push(format!("{}.TIMINGR", a.peripheral));
        }
        if let Some((timer, channel)) = aethelgard_ir::motor::timer_channel(&a.signal) {
            let ccmr = if channel <= 2 { 1 } else { 2 };
            unobservable.push(format!("{}.CCMR{ccmr}.OC{channel}PE", a.peripheral));
            // External clock mode 2 (`ECE`, L211) would leave silicon's
            // counter waiting on `ETR`; a motor pin's output type is a tag
            // too (adversarial H2).
            unobservable.push(format!("{}.SMCR.ECE", a.peripheral));
            if aethelgard_ir::motor::has_main_output_enable(timer) {
                unobservable.push(format!("{}.BDTR.MOE", a.peripheral));
                unobservable.push(format!("{}.BDTR.BKE", a.peripheral));
            }
        }
    }
    for pin in topology.gpio.values() {
        let (port, index) = split_pin(pin)?;
        pins.insert(pin, (port, index, None));
        gates.insert(format!("GPIO{port}"));
    }

    // (address, name) to (mask, value), so the list comes out in address order.
    let mut regs: BTreeMap<(u32, String), (u32, u32)> = BTreeMap::new();
    let mut field = |address: u32, name: String, mask: u32, value: u32| {
        let e = regs.entry((address, name)).or_insert((0, 0));
        e.0 |= mask;
        e.1 |= value;
    };
    for &(port, index, af) in pins.values() {
        // Every configured port's `OTYPER` is written whole (`AET-WO-4230`
        // D5, B0), and Renode stores none of its bits (spike F11).
        unobservable.push(format!("GPIO{port}.OTYPER"));
        let base = f.base(&format!("GPIO{port}"))?;
        let mode = if af.is_some() { MODE_AF } else { MODE_OUTPUT };
        field(
            base + gpio.offset("MODER", 0)?,
            format!("GPIO{port}.MODER"),
            0b11 << (2 * index),
            mode << (2 * index),
        );
        if let Some(af) = af {
            let word = index / 8;
            let shift = 4 * (index % 8);
            field(
                base + gpio.offset("AFR", word)?,
                format!("GPIO{port}.{}", ["AFRL", "AFRH"][word as usize]),
                0xF << shift,
                af << shift,
            );
        }
    }
    timer_registers(topology, &f, &mut field)?;
    serial_registers(topology, &f, &mut field, &mut unobservable)?;
    // The two bits the generated bring-up waits on, which the emitted
    // platform's PWR answers 1 whatever happens (`AET-WO-4240v` §1.6,
    // [`platform::pwr_override`]); a foreign platform keeps the stock stub.
    if topology.foreign_clock.is_none() {
        unobservable.extend([
            "PWR.CSR1.ACTVOSRDY".to_owned(),
            "PWR.D3CR.VOSRDY".to_owned(),
        ]);
    }
    unobservable.sort();
    unobservable.dedup();
    // Every enable register (`*ENR`, not `*LPENR`) of the header's
    // `RCC_TypeDef`, cold at its reset value and judged in full: the reset
    // value and exactly the solved enable bits (2100 F24).
    let mut cold: BTreeMap<(u32, String), (u32, u32)> = BTreeMap::new();
    for reg in rcc
        .fields
        .iter()
        .filter(|r| r.name.ends_with("ENR") && !r.name.ends_with("LPENR"))
    {
        let reset = ENR_RESET
            .iter()
            .find(|(name, _)| *name == reg.name)
            .map_or(0, |(_, v)| *v);
        cold.insert(
            (rcc_base + reg.offset, format!("RCC.{}", reg.name)),
            (reset, 0),
        );
    }
    for peripheral in &gates {
        let (register, bit) = f.enable_bit(peripheral)?;
        let address = rcc_base + rcc.offset(&register, 0)?;
        cold.get_mut(&(address, format!("RCC.{register}")))
            .ok_or_else(|| RegisterError::Undefined {
                name: format!("RCC_TypeDef.{register}"),
            })?
            .1 |= 1 << bit;
    }
    let cold: Vec<Register> = cold
        .into_iter()
        .map(|((address, name), (reset, bits))| {
            field(address, name.clone(), u32::MAX, reset | bits);
            Register {
                name,
                address,
                mask: u32::MAX,
                value: reset,
            }
        })
        .collect();
    // No interrupt armed (`AET-WO-2204` A2, ruled D6): SysTick's control
    // word, and one set-enable word per 32 of the part's interrupt lines
    // (the header's highest `IRQn`).
    field(SYST_CSR, "SysTick.CSR".to_owned(), SYST_CSR_ARMED, 0);
    for i in 0..=f.irq_max()? / 32 {
        field(NVIC_ISER + 4 * i, format!("NVIC.ISER{i}"), u32::MAX, 0);
    }
    let registers = regs
        .into_iter()
        .map(|((address, name), (mask, value))| Register {
            name,
            address,
            mask,
            value,
        })
        .collect();
    Ok(Image {
        cold,
        registers,
        unobservable,
        identity,
        brown_out: brown_out(topology)?,
    })
}
