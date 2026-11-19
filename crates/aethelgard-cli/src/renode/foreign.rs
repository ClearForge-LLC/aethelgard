//! The foreign-firmware judge (`AET-WO-4400e`, ruled on PR #212, comment
//! 6958822008; D4 on PR #215, comment 6961943794): a second judge, for a
//! firmware this project did not generate, chosen by `simulate --foreign`
//! and never inferred from the ELF (`INV-06`).
//!
//! It boots the firmware on the same emitted platform, under the same bounds,
//! but with none of the generated runtime's contract: no boot word, no records
//! in RAM, no hooks of ours, no serial injection. What it says comes from
//! witnesses the firmware cannot write (`INV-07`): the models' own transaction
//! lines and the platform's own state. Every rule of today's judge is kept,
//! replaced or dropped, [`RULES`], and the verdict names each, with what is
//! weakened.

use serde::Serialize;

use super::*;
use crate::verdict::{Finding, FirmwareReport, Machine};

/// What `boot.judge` says on a foreign verdict.
pub const JUDGE: &str = "foreign";

/// The longest boot `--foreign` admits, in virtual nanoseconds, and its name
/// in the verdict (`AET-WO-4400e` D7, measured with the ICM-42688-P's FIFO
/// live; `AET-WO-4400g` §1.4 since the twin's timers run at ArduPilot's
/// declared 200 MHz, not 250): unmodified ArduPilot's last witness, the ICM's
/// first FIFO read, is at 4.394 s (it was 3.6105 s at 1.25× virtual time),
/// so 5 s leaves 0.61 s. Its RSS margin under concurrency is FEEDBACK's F1
/// measurement; the 1 GiB bound itself is a ruling of its own, never widened
/// here.
pub const FOREIGN_MAX_VIRTUAL_NS: u64 = 5_000_000_000;
pub const FOREIGN_WINDOW: &str = "FOREIGN_MAX_VIRTUAL_NS";

/// The rule table, in the order of the verdict: each of today's rules, what
/// a foreign boot does with it, and what that judges or weakens (`INV-07`).
/// Every row is ruled (PR #212, comments 6958822008, 6965511448 and
/// 6966155027); the bound is D7's measurement.
pub const RULES: &[(&str, &str, &str)] = &[
    (
        "firmware memory",
        "replaced",
        "every PT_LOAD, at both addresses, inside flash or one RAM region ST's header states (seven regions), not flash and DTCM alone",
    ),
    (
        "PT_LOAD overlap",
        "replaced",
        "no two segments' file bytes overlap; zero-filled segments may share a physical address (ruled on PR #212, comment 6965511448)",
    ),
    (
        "boot word",
        "replaced",
        "booted: the first identity transaction a declared device's model logs, with no core_in_exception and the final PC in an executable PT_LOAD. Weakens: witnesses that the firmware drove a device's bus, not that a fixed point was reached; the console banner is refused",
    ),
    (
        "instructions (cpu_halted)",
        "dropped",
        "the core may sleep (WFI). Weakens: no witness that the core executed for the whole run",
    ),
    (
        "core_in_exception",
        "kept",
        "ICSR.VECTACTIVE is 0 at the run's end",
    ),
    (
        "pc_outside_firmware",
        "kept",
        "the final PC is inside an executable PT_LOAD",
    ),
    (
        "self-loop (pc_not_on_self_loop)",
        "dropped",
        "an RTOS parks in its idle thread. Weakens: the parked witness",
    ),
    (
        "solved pins' MODER and AFR",
        "kept",
        "each solved pin's mode and alternate function, exactly",
    ),
    (
        "USART CR1, CR2",
        "replaced",
        "CR1's frame bits (UE, TE, PS, PCE, M0, M1) and CR2's STOP against the solved framing. Weakens: CR1.RE (receive enable) is not judged, as receive is held",
    ),
    (
        "USART CR3, BRR",
        "dropped",
        "Weakens: the baud and the DMA enables are unjudged (BRR depends on the firmware's clock tree; Renode's usart is fixed at 200 MHz)",
    ),
    (
        "cold RCC enable registers",
        "replaced",
        "at least the solved enable bits are set. Weakens: an unsolved peripheral clocked is not caught",
    ),
    (
        "SysTick.CSR",
        "dropped",
        "Weakens: an armed SysTick is not caught (a foreign RTOS ticks as it chooses)",
    ),
    (
        "NVIC.ISER0-4",
        "dropped",
        "Weakens: armed interrupts are not caught",
    ),
    (
        "--for at most 100 ms",
        "replaced",
        "--foreign's own bound, FOREIGN_MAX_VIRTUAL_NS, 5 s (the last witness of unmodified ArduPilot at 4.394 s at its declared clock, AET-WO-4400g)",
    ),
    (
        "device identity",
        "kept",
        "each modelled device's identity transaction, its model's own lines, found in the device's log in order",
    ),
    (
        "device configure",
        "dropped",
        "Weakens: the configure step is not judged for a foreign firmware (the firmware's own configuration)",
    ),
    (
        "device data",
        "replaced",
        "one of the part's data reads (its script's, or one its table lists) after its identity, by the model's own lines",
    ),
    (
        "records in RAM",
        "dropped",
        "a foreign firmware keeps no records; the witness lines stand alone, as they always were the evidence",
    ),
    (
        "SPI chip-select windows",
        "replaced",
        "a step is the first run of the device's own lines that matches it, after the step before. Weakens: extra transactions before a step are not refused",
    ),
    (
        "device warnings",
        "kept",
        "a [WARNING] of a declared device's model, but its model's listed artefacts, fails the run",
    ),
    (
        "renode_error_log",
        "kept, with D4",
        "the SPI model's slave-mode [ERROR] is exempt only directly after a CFG2 write with MASTER 0 while CR1 holds SPE 0, and only when that controller's next CFG2 write sets MASTER again before any of its devices logs a line (adversarial F1, accepted on PR #212, comment 6967529218), as a hook on each SPI controller's CFG2 writes logs it (D7 (b)); every exemption is counted in the verdict",
    ),
    (
        "renode_log_flood",
        "kept, with D7 (a)",
        "the 8 MiB budget stands; past it no witness is read whole: every step is unwitnessed. Renode's own Unhandled write warnings are summarised: each kind (peripheral and message, the written value elided) is one line with its count and first and last virtual time in boot.warnings (ruled on PR #212, comment 6965511448, D7 (a)); none is hidden, none floods; at most 1,024 kinds are summarised, and a line of a further kind is kept or dropped as any other, against the budget",
    ),
    (
        "locked_peripheral_access",
        "replaced",
        "an info finding naming each address with its first time and count (ruled on PR #212, comment 6965511448). Weakens: unclocked accesses are not judged (silicon ignores them)",
    ),
    (
        "brown-out record",
        "dropped",
        "Weakens: the BOR level is not judged in the twin (a foreign firmware keeps no record)",
    ),
    (
        "flash controller access",
        "replaced",
        "no write inside its own loaded flash (ruled on PR #212, comments 6965511448 and 6966155027): each loaded flash segment's sha256, from the ELF's file bytes on the host, equals Renode's read of it after the run; and no erase it starts (SER with SNB or BER on FLASH_CR1/CR2, or MER on FLASH_OPTCR at 0x18 or its alias 0x118, RM0433 Rev 9 §4.9.7, p. 211; each bit from ST's header) overlaps one; an option-byte change (OPTSTART) is info. Weakens: writes elsewhere in flash are not judged; a store that changes a word of an own segment and a later store that restores it, with no erase between, passes, which silicon cannot do: programming only clears bits, and a bit returns to 1 only by erasing its whole sector (RM0433 Rev 9 §4.3.9, p. 161), an erase the hook fails",
    ),
    (
        "motor outputs",
        "dropped",
        "Weakens: motor outputs are unjudged (their window is anchored on the generated bring-up's CEN; the flight-loop order's backlog)",
    ),
    (
        "serial pattern and echo",
        "dropped",
        "Weakens: no serial witness (an echo needs receive, which stays held)",
    ),
    (
        "wall, memory and abort bounds",
        "kept",
        "30 s, 1 GiB, an aborted machine",
    ),
    (
        "device_unmodelled",
        "kept",
        "a device with no model is named (info)",
    ),
];

/// One region a foreign firmware may load into.
#[derive(Serialize, Clone)]
pub struct RegionReport {
    pub name: &'static str,
    pub start: String,
    pub end: String,
}

/// One rule of [`RULES`], in the verdict.
#[derive(Serialize)]
pub struct RuleReport {
    pub rule: &'static str,
    pub disposition: &'static str,
    pub judges: &'static str,
}

/// One register the foreign judge reads, and how.
#[derive(Serialize)]
pub struct RegisterReport {
    pub name: String,
    pub address: String,
    /// `equal` under `mask`, or `at_least` (every bit of `expected` set).
    pub rule: &'static str,
    pub mask: String,
    pub expected: String,
    pub observed: String,
}

/// One device step the foreign judge looks for, and where it found it.
#[derive(Serialize)]
pub struct StepReport {
    pub reference: String,
    pub device: String,
    /// `identity`, or `data` with the read it is (`script` or `reads[k]`).
    pub step: String,
    pub expected: Vec<String>,
    /// The virtual time of the step's first line, `null` when not found.
    pub observed_ns: Option<u64>,
}

/// One kind of Renode `Unhandled write` warning, summarised (D7 (a)).
#[derive(Serialize)]
pub struct WarningReport {
    pub line: String,
    pub count: u64,
    pub first_ns: Option<u64>,
    pub last_ns: Option<u64>,
}

/// A loaded flash segment, read back after the run.
#[derive(Serialize)]
pub struct FlashReport {
    pub address: String,
    pub bytes: u64,
    pub expected_sha256: String,
    pub observed_sha256: String,
}

/// A flash erase the firmware started: when, and the flash it erases.
#[derive(Serialize)]
pub struct EraseReport {
    pub at_ns: Option<u64>,
    pub start: String,
    pub end: String,
}

/// An access to a peripheral whose clock was off: its address, the first
/// time and how often.
#[derive(Serialize)]
pub struct LockedReport {
    pub access: String,
    pub first_ns: Option<u64>,
    pub count: u64,
}

/// What booted the firmware, as D5 rules it.
#[derive(Serialize)]
pub struct Booted {
    pub reference: String,
    pub device: String,
    pub at_ns: u64,
}

/// A foreign boot's report, the verdict's `boot` (`AET-WO-4400e` D3).
#[derive(Serialize)]
pub struct ForeignBoot {
    pub judge: &'static str,
    pub firmware: FirmwareReport,
    pub specification_sha256: String,
    pub seed: u32,
    pub virtual_time_ns: u64,
    pub window: &'static str,
    pub memory_regions: Vec<RegionReport>,
    pub mips: Option<u64>,
    /// Executed instructions, for a reader: no rule reads it.
    pub instructions: Option<u64>,
    pub pc: Option<String>,
    pub icsr: Option<String>,
    pub booted: Option<Booted>,
    pub registers: Vec<RegisterReport>,
    pub devices: Vec<StepReport>,
    /// D4: the SPI model's slave-mode `[ERROR]` lines exempted, each directly
    /// after a `CFG2` write with `MASTER` 0 while `SPE` was 0.
    pub spi_errors_exempted: u64,
    /// Renode's `Unhandled write` warnings, each kind once (D7 (a)).
    pub warnings: Vec<WarningReport>,
    /// The bytes of Renode's output the log filter kept and counted against
    /// `log_budget_bytes`, and the log lines it dropped past it (D7 (c)).
    pub log_kept_bytes: u64,
    pub log_budget_bytes: u64,
    pub log_lines_dropped: u64,
    /// The firmware's loaded flash segments, read back after the run.
    pub flash: Vec<FlashReport>,
    /// Each flash erase the firmware started, as the erase hook logs it.
    pub erases: Vec<EraseReport>,
    /// The virtual time of each option-byte change it started (`info`).
    pub option_changes: Vec<Option<u64>>,
    /// Accesses to a peripheral whose clock was off (`info`).
    pub locked: Vec<LockedReport>,
    pub unobservable: Vec<String>,
    pub rules: Vec<RuleReport>,
    /// The clock the platform was built at, and what of it was checked
    /// (`AET-WO-4400g` §1.2).
    pub clock: ClockReport,
}

/// The foreign firmware's clock in the verdict (`AET-WO-4400g` §1.2, Q2
/// ruled on PR #229): the timers' kernel clocks declared and checked
/// against the RCC registers read back at the run's end, each USART's
/// declared and not checked, and what neither sets.
#[derive(Serialize, Clone)]
pub struct ClockReport {
    /// `declared`: `mcu.foreign_clock` built every timer and USART.
    pub source: &'static str,
    /// The RCC registers read back at the run's end, each `{name, value}`;
    /// empty when the run never reached its end.
    pub registers: Vec<RccRead>,
    /// The timers' kernel clocks, declared and decoded.
    pub checked: Vec<ClockCheck>,
    /// The USARTs' kernel clocks: declared, never read back.
    pub declared_only: Vec<DeclaredClock>,
    /// The instances at the stock file's frequency, which nothing declares.
    pub stock: Vec<&'static str>,
    /// What the check rests on, each a named sentence.
    pub conditions: Vec<String>,
}

/// One RCC register as read back.
#[derive(Serialize, Clone)]
pub struct RccRead {
    pub name: &'static str,
    pub value: String,
}

/// One timer bus's kernel clock: declared, decoded from the RCC registers
/// (`null` when they were not read or do not decode), and whether the two
/// agree.
#[derive(Serialize, Clone)]
pub struct ClockCheck {
    pub clock: &'static str,
    pub declared_hz: u64,
    pub decoded_hz: Option<u64>,
    pub holds: bool,
}

/// A clock declared and not read back.
#[derive(Serialize, Clone)]
pub struct DeclaredClock {
    pub instance: String,
    pub declared_hz: u64,
}

/// The conditions every foreign verdict states (`AET-WO-4400g` §1.2, §1.6).
pub const USART_UNCHECKED: &str = "usart_clock_declared_unchecked: each USART's kernel clock is the declared one, not read back: Renode 1.17.0 drops RCC_D2CCIP2R, so no register says what the firmware selected (AET-WO-4400f F2)";
pub const PLL1_FRACTION_UNREAD: &str = "pll1_fraction_not_read: the decode is PLL1's integer divider: Renode 1.17.0 tags PLL1FRACEN and FRACN1, so a fractional PLL is not read back";
pub const STOCK_UNDECLARED: &str = "stock_frequency_undeclared: dwt and lpuart1 keep the stock file's 250 MHz and 125 MHz, which no declaration sets; no judge reads either";

/// The finding kinds of the clock check.
pub const CLOCK_CONTRADICTED: &str = "foreign_clock_contradicted";
pub const CLOCK_UNDECODABLE: &str = "foreign_clock_undecodable";

impl ClockReport {
    /// The report before the run: what was declared, nothing decoded.
    pub fn declared(topology: &aethelgard_ir::SolvedTopology) -> Option<Self> {
        let c = topology.foreign_clock.as_ref()?;
        Some(Self::of(c, aethelgard_twin::platform::unclocked(topology)))
    }

    fn of(c: &aethelgard_ir::motor::SolvedForeignClock, stock: Vec<&'static str>) -> Self {
        Self {
            source: "declared",
            registers: Vec::new(),
            checked: [
                ("apb1_timers_hz", c.apb1_timers_hz),
                ("apb2_timers_hz", c.apb2_timers_hz),
            ]
            .into_iter()
            .map(|(clock, declared_hz)| ClockCheck {
                clock,
                declared_hz,
                decoded_hz: None,
                holds: false,
            })
            .collect(),
            declared_only: c
                .usarts
                .iter()
                .map(|(instance, hz)| DeclaredClock {
                    instance: instance.clone(),
                    declared_hz: *hz,
                })
                .collect(),
            stock,
            conditions: vec![
                USART_UNCHECKED.to_owned(),
                PLL1_FRACTION_UNREAD.to_owned(),
                STOCK_UNDECLARED.to_owned(),
            ],
        }
    }

    /// The check at the run's end: `values`, the RCC registers in
    /// [`RccRegisters::OFFSETS`]' order, decoded with the declared
    /// crystal `hse_hz` (RM0433 Rev 9, `SolvedClock::from_rcc`), and each
    /// timer bus's decoded clock held to its declaration. Each disagreement
    /// is a failing finding with both figures.
    ///
    /// [`RccRegisters::OFFSETS`]: aethelgard_ir::motor::RccRegisters::OFFSETS
    fn check(&mut self, values: [u32; 6], hse_hz: u32) -> Vec<Finding> {
        use aethelgard_ir::motor::{Apb, RccRegisters};
        self.registers = RccRegisters::OFFSETS
            .iter()
            .zip(values)
            .map(|(&(name, _), v)| RccRead {
                name,
                value: format!("{v:#010x}"),
            })
            .collect();
        let decoded = match aethelgard_ir::SolvedClock::from_rcc(
            &RccRegisters::from_values(values),
            hse_hz,
        ) {
            Ok(c) => c,
            Err(e) => {
                return vec![Finding::plain(
                    CLOCK_UNDECODABLE,
                    format!("the RCC registers read back at the run's end do not decode to a timer kernel clock: {e}"),
                )];
            }
        };
        let mut out = Vec::new();
        for (check, apb) in self.checked.iter_mut().zip([Apb::Apb1, Apb::Apb2]) {
            let hz = decoded.timer_kernel_hz(apb);
            check.decoded_hz = Some(hz);
            check.holds = hz == check.declared_hz;
            if !check.holds {
                out.push(Finding::plain(
                    CLOCK_CONTRADICTED,
                    format!(
                        "mcu.foreign_clock.{} declares {} Hz; the RCC registers read back at the run's end decode to {hz} Hz (RM0433 Rev 9 Table 57, sys_ck {} Hz from {})",
                        check.clock, check.declared_hz, decoded.sysclk_hz, decoded.source
                    ),
                ));
            }
        }
        out
    }
}

/// The regions as the verdict names them.
pub fn regions_report(
    regions: &[(&'static str, aethelgard_twin::boot::Region)],
) -> Vec<RegionReport> {
    regions
        .iter()
        .map(|(name, r)| RegionReport {
            name,
            start: format!("{:#010x}", r.start),
            end: format!("{:#010x}", r.end),
        })
        .collect()
}

/// The foreign boot's log filter (D7 (a)): today's, with Renode's own
/// `Unhandled write` warnings summarised by [`unhandled_write_key`].
pub const FOREIGN_LOG_FILTER: LogFilter = LogFilter {
    summarise: unhandled_write_key,
    ..LOG_FILTER
};

/// An `Unhandled write` warning's summary key: the line after its timestamp
/// with every written value elided, so writes of other values to the same
/// register are one kind; `None` for every other line.
pub fn unhandled_write_key(line: &str) -> Option<String> {
    let body = log_body(line)?;
    let (source, text) = body.strip_prefix("[WARNING] ")?.split_once(": ")?;
    if !text.starts_with("Unhandled write to offset ") {
        return None;
    }
    let mut key = format!("[WARNING] {source}: ");
    let mut rest = text;
    while let Some(at) = rest.find("value 0x") {
        key.push_str(&rest[..at + "value 0x".len()]);
        rest = &rest[at + "value 0x".len()..];
        let hex = rest.bytes().take_while(u8::is_ascii_hexdigit).count();
        key.push('…');
        rest = &rest[hex..];
    }
    key.push_str(rest);
    Some(key)
}

/// The line D7 (b)'s hook logs for a write to an SPI controller's `CFG2`
/// (offset `0xC`): the value written and `CR1` as it stands, in hexadecimal.
const CFG2_MARK: &str = "AETHELGARD-CFG2 ";

/// The hook on an SPI controller's writes that logs each `CFG2` write with
/// `CR1` (D4's evidence, D7 (b)); every other write passes unlogged.
fn cfg2_hook(controller: &str) -> String {
    log_hook(
        controller,
        "offset == 0xC",
        CFG2_MARK,
        "value, self.ReadDoubleWord(0)",
    )
}

/// A write hook on `peripheral` that, when `condition` holds, logs `mark`
/// and two hexadecimal `values` as a `[WARNING]` of the peripheral's own;
/// every write passes unchanged.
fn log_hook(peripheral: &str, condition: &str, mark: &str, values: &str) -> String {
    format!(
        "sysbus SetHookBeforePeripheralWrite sysbus.{peripheral} \"({condition}) and __import__('Antmicro.Renode.Logging').Renode.Logging.Logger.Log(self, __import__('Antmicro.Renode.Logging').Renode.Logging.LogLevel.Warning, '{mark}%x %x' % ({values}))\""
    )
}

/// The monitor command that prints a loaded flash segment's sha256 as Renode
/// holds it after the run, read by the judge: `AETHELGARD-FLASH <address> <sha256>`.
fn flash_hash(f: &crate::elf::FlashLoad) -> String {
    format!(
        "python \"print('AETHELGARD-FLASH {:x} ' + (__import__('clr').AddReference('System.Security.Cryptography') or __import__('System').BitConverter.ToString(__import__('System.Security.Cryptography').Security.Cryptography.SHA256.Create().ComputeHash(self.Machine.SystemBus.ReadBytes({:#x}, {:#x}))).replace('-', '').lower()))\"",
        f.address, f.address, f.len
    )
}

/// The line the erase hook logs for a write to a flash bank's control
/// register that sets `START`: the register's offset and the value written.
const ERASE_MARK: &str = "AETHELGARD-ERASE ";

/// The flash controller's erase facts, every one from ST's header
/// (`RUL-16`): each bank's control register offset in `FLASH_TypeDef` with
/// the bank's base; `FLASH_OPTCR`'s offset; the bits `SER`, `BER`, `START`,
/// `MER` and `OPTSTART`, the field `SNB`, and the sector size; a bank is
/// `FLASH_SECTOR_TOTAL` sectors.
pub struct Erase {
    banks: [(u32, u64); 2],
    /// `FLASH_OPTCR`'s offset in `FLASH_TypeDef`, and its alias in bank 2's
    /// half of the register map, as far from it as `CR2` is from `CR1`:
    /// "Address offset: 0x018 or 0x118" (RM0433 Rev 9 §4.9.7, p. 211; the
    /// header's struct has only the first). Renode's model decodes only the
    /// first, so a mass erase at the alias would erase nothing in the twin
    /// and everything on silicon (adversarial pass, F2).
    optcr: [u32; 2],
    ser: u32,
    ber: u32,
    start: u32,
    mer: u32,
    optstart: u32,
    snb: (u32, u32),
    sector: u64,
    bank: u64,
}

/// What a logged flash-controller write starts.
#[derive(Debug, PartialEq, Eq)]
enum Started {
    /// An erase of `[start, end)`.
    Erase(u64, u64),
    /// An option-byte change (`OPTSTART`).
    Options,
}

impl Erase {
    pub fn facts() -> Result<Self, Stop> {
        let hal = |e: aethelgard_svd_hal::cmsis::RegisterError| {
            Stop::tool_error("aethelgard-svd-hal", e.to_string())
        };
        let f = aethelgard_svd_hal::cmsis::register_facts().map_err(hal)?;
        let layout = f.layout("FLASH").map_err(hal)?;
        let sector = u64::from(f.value("FLASH_SECTOR_SIZE").map_err(hal)?);
        Ok(Self {
            banks: [
                (
                    layout.offset("CR1", 0).map_err(hal)?,
                    u64::from(f.value("FLASH_BANK1_BASE").map_err(hal)?),
                ),
                (
                    layout.offset("CR2", 0).map_err(hal)?,
                    u64::from(f.value("FLASH_BANK2_BASE").map_err(hal)?),
                ),
            ],
            optcr: {
                let optcr = layout.offset("OPTCR", 0).map_err(hal)?;
                let bank2 =
                    layout.offset("CR2", 0).map_err(hal)? - layout.offset("CR1", 0).map_err(hal)?;
                [optcr, optcr + bank2]
            },
            ser: f.position("FLASH_CR_SER_Pos").map_err(hal)?,
            ber: f.position("FLASH_CR_BER_Pos").map_err(hal)?,
            start: f.position("FLASH_CR_START_Pos").map_err(hal)?,
            mer: f.position("FLASH_OPTCR_MER_Pos").map_err(hal)?,
            optstart: f.position("FLASH_OPTCR_OPTSTART_Pos").map_err(hal)?,
            snb: f.field("FLASH_CR_SNB").map_err(hal)?,
            sector,
            bank: sector * u64::from(f.value("FLASH_SECTOR_TOTAL").map_err(hal)?),
        })
    }

    /// The hook on the flash controller's writes that logs each write to a
    /// bank's control register with `START` set, and each write to
    /// `FLASH_OPTCR` with `MER` or `OPTSTART` set; every other write passes
    /// unlogged and unchanged.
    pub fn hook(&self) -> String {
        let condition = format!(
            "(offset in ({:#x}, {:#x})) and (value >> {}) & 1 or offset in ({:#x}, {:#x}) and ((value >> {}) & 1 or (value >> {}) & 1)",
            self.banks[0].0,
            self.banks[1].0,
            self.start,
            self.optcr[0],
            self.optcr[1],
            self.mer,
            self.optstart
        );
        log_hook("flashController", &condition, ERASE_MARK, "offset, value")
    }

    /// What a logged write starts. On a bank's control register: the bank
    /// with `BER`, else the sector `SNB` names with `SER`, else nothing. On
    /// `FLASH_OPTCR`: both banks with `MER` (which sets `BER1`, `BER2`,
    /// `START1` and `START2`, RM0433 Rev 9 `FLASH_OPTCR`), and an option-byte
    /// change with `OPTSTART`.
    fn started(&self, body: &str) -> Vec<Started> {
        let Some(text) = body
            .strip_prefix("[WARNING] flashController: ")
            .and_then(|t| t.strip_prefix(ERASE_MARK))
        else {
            return Vec::new();
        };
        let Some((offset, value)) = text.split_once(' ').and_then(|(o, v)| {
            Some((
                u32::from_str_radix(o, 16).ok()?,
                u32::from_str_radix(v.trim(), 16).ok()?,
            ))
        }) else {
            return Vec::new();
        };
        let bit = |at: u32| value >> at & 1 == 1;
        let mut out = Vec::new();
        if self.optcr.contains(&offset) {
            if bit(self.mer) {
                out.push(Started::Erase(self.banks[0].1, self.banks[1].1 + self.bank));
            }
            if bit(self.optstart) {
                out.push(Started::Options);
            }
            return out;
        }
        let Some(&(_, base)) = self.banks.iter().find(|(o, _)| *o == offset) else {
            return out;
        };
        if !bit(self.start) {
            return out;
        }
        if bit(self.ber) {
            out.push(Started::Erase(base, base + self.bank));
        } else if bit(self.ser) {
            let (at, width) = self.snb;
            let at = base + u64::from(value >> at & ((1 << width) - 1)) * self.sector;
            out.push(Started::Erase(at, at + self.sector));
        }
        out
    }
}

/// Each SPI controller a modelled device is on, as Renode names it (`spi2`).
fn spi_controllers(plan: &BootPlan<'_>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for c in &plan.image.identity.checks {
        if let aethelgard_twin::boot::Wire::Spi { .. } = c.wire {
            let name = c.device.split('.').next().unwrap_or_default().to_owned();
            if !out.contains(&name) {
                out.push(name);
            }
        }
    }
    out
}

/// The foreign boot's monitor commands: the platform, the firmware, the cold
/// RCC, each modelled device's log level and stimulus, each SPI controller's
/// `CFG2` hook (D4), the run, then the reads between the markers, each loaded flash segment's sha256, and the
/// listing. No boot word, no record, no hook on the firmware's own code and
/// no injection: none of the generated runtime's contract.
pub fn commands(plan: &BootPlan<'_>, flash: &[crate::elf::FlashLoad]) -> Result<String, Stop> {
    let mut c = vec![
        format!("emulation SetSeed {}", aethelgard_twin::boot::SEED),
        QUANTUM.to_owned(),
    ];
    c.extend(plan.models.iter().map(|m| include(m)));
    c.extend([
        "mach create".to_owned(),
        format!(
            "machine LoadPlatformDescription @{}",
            plan.platform.display()
        ),
        format!("sysbus LoadELF @{}", plan.firmware.display()),
    ]);
    c.extend(plan.cold());
    c.push(plan.vtor());
    c.extend(plan.witness_levels());
    c.extend(spi_controllers(plan).iter().map(|s| cfg2_hook(s)));
    c.push(Erase::facts()?.hook());
    c.extend(plan.stimuli());
    c.push(run_for(plan.virtual_ns));
    c.push(format!("echo \"{BEGIN}\""));
    c.extend(reads(plan)?.into_iter().map(|(cmd, _)| cmd));
    c.push(format!("echo \"{END}\""));
    c.extend(flash.iter().map(flash_hash));
    c.push("peripherals".to_owned());
    c.push("quit".to_owned());
    Ok(c.join("; "))
}

/// What one read gives.
#[derive(Clone, Copy)]
enum ForeignRead {
    Instructions,
    Mips,
    Pc,
    Icsr,
    Register(usize),
    /// The RCC register at this index of `RccRegisters::OFFSETS`.
    Rcc(usize),
}

fn reads(plan: &BootPlan<'_>) -> Result<Vec<(String, ForeignRead)>, Stop> {
    let rcc = aethelgard_svd_hal::cmsis::register_facts()
        .and_then(|f| f.base("RCC"))
        .map_err(|e| Stop::tool_error("aethelgard-svd-hal", e.to_string()))?;
    let mut r = vec![
        (
            "cpu ExecutedInstructions".to_owned(),
            ForeignRead::Instructions,
        ),
        ("cpu PerformanceInMips".to_owned(), ForeignRead::Mips),
        ("cpu PC".to_owned(), ForeignRead::Pc),
        (
            format!(
                "sysbus ReadDoubleWord {:#010x}",
                aethelgard_twin::boot::ICSR
            ),
            ForeignRead::Icsr,
        ),
    ];
    for (i, reg) in plan.image.registers.iter().enumerate() {
        r.push((
            format!("sysbus ReadDoubleWord {:#010x}", reg.address),
            ForeignRead::Register(i),
        ));
    }
    for (i, (_, offset)) in aethelgard_ir::motor::RccRegisters::OFFSETS.iter().enumerate() {
        r.push((
            format!("sysbus ReadDoubleWord {:#010x}", rcc + offset),
            ForeignRead::Rcc(i),
        ));
    }
    Ok(r)
}

/// How the foreign judge reads a register of the image, by its name, or
/// `None` when the rule table drops it.
fn register_rule(name: &str, frame: u32) -> Option<(&'static str, Option<u32>)> {
    let (instance, register) = name.split_once('.')?;
    if instance.starts_with("GPIO") {
        return Some(("equal", None));
    }
    if instance == "RCC" {
        return Some(("at_least", None));
    }
    if instance.starts_with("USART")
        || instance.starts_with("UART")
        || instance.starts_with("LPUART")
    {
        return match register {
            "CR1" => Some(("equal", Some(frame))),
            "CR2" => Some(("equal", None)),
            _ => None,
        };
    }
    if instance == "SysTick" || instance == "NVIC" || instance.starts_with("TIM") {
        return None;
    }
    Some(("equal", None))
}

/// `USART_CR1`'s frame bits, each position from ST's header: UE, TE, PS, PCE,
/// M0 and M1.
fn frame_bits() -> Result<u32, Stop> {
    let f = aethelgard_svd_hal::cmsis::register_facts()
        .map_err(|e| Stop::tool_error("aethelgard-svd-hal", e.to_string()))?;
    let mut mask = 0u32;
    for name in ["UE", "TE", "PS", "PCE", "M0", "M1"] {
        let at = f
            .position(&format!("USART_CR1_{name}_Pos"))
            .map_err(|e| Stop::tool_error("aethelgard-svd-hal", e.to_string()))?;
        mask |= 1 << at;
    }
    Ok(mask)
}

/// One expected line of a step: the model's whole line, or a listed read's
/// pattern.
enum Line {
    Exact(String),
    Pattern(aethelgard_twin::model::WitnessLine),
}

impl Line {
    fn matches(&self, body: &str) -> bool {
        match self {
            Line::Exact(t) => body == t,
            Line::Pattern(p) => p.matches(body),
        }
    }

    fn describe(&self) -> String {
        match self {
            Line::Exact(t) => t.clone(),
            Line::Pattern(aethelgard_twin::model::WitnessLine::Exact(t)) => t.clone(),
            Line::Pattern(aethelgard_twin::model::WitnessLine::Bytes { prefix, at_least }) => {
                format!("{prefix}<at least {at_least} bytes>")
            }
        }
    }
}

/// The first run of `own` from `from` whose lines match `want` in order, as
/// the index of its first line in `own`.
fn find(own: &[(usize, &str)], want: &[Line], from: usize) -> Option<usize> {
    if want.is_empty() || own.len() < want.len() {
        return None;
    }
    (from..=own.len() - want.len())
        .find(|&k| want.iter().zip(&own[k..]).all(|(w, (_, b))| w.matches(b)))
}

/// The SPI model's `[ERROR]` on a `CFG2` write that clears `MASTER`
/// (`STM32H7_SPI.cs` L187–194, `AET-WO-4400e2` D4).
const SLAVE_MODE: &str =
    "Attempted to set peripheral into SPI slave mode. Only master mode is supported";

/// A `CFG2` write as D7 (b)'s hook logs it: the controller, the value
/// written and `CR1` as it stood.
fn cfg2_write(body: &str) -> Option<(&str, u64, u64)> {
    let (source, text) = body.strip_prefix("[WARNING] ")?.split_once(": ")?;
    let (value, cr1) = text.strip_prefix(CFG2_MARK)?.split_once(' ')?;
    Some((
        source,
        u64::from_str_radix(value, 16).ok()?,
        u64::from_str_radix(cr1, 16).ok()?,
    ))
}

/// The run's first `[ERROR]` line D4 does not exempt, and how many it did:
/// the SPI model's slave-mode line directly after a `CFG2` write with `MASTER`
/// (bit 22) 0 while `CR1` held `SPE` (bit 0) 0, as the hook logs it, and only
/// when that controller's next `CFG2` write sets `MASTER` again before any of
/// its devices logs a line (or none does to the end): ChibiOS's
/// `spi_lld_stop`, then its next `spi_lld_start`. A firmware that leaves the
/// controller a slave and goes on using it is not exempt, for Renode's model
/// stays a master (`STM32H7_SPI.cs` L187-194) where silicon would not clock.
/// The bits are RM0433 Rev 9 §50.11.1 and §50.11.4's (`AET-WO-4400e2` D4;
/// adversarial pass, F1).
fn error_lines<'l>(run_log: &[&'l str]) -> (Option<&'l str>, u64) {
    // Backwards, per controller: whether its next event after each line is a
    // `CFG2` write with `MASTER` 1 (or the end), not a device's line.
    let mut restored: std::collections::BTreeMap<&str, bool> = std::collections::BTreeMap::new();
    let mut restored_after = vec![true; run_log.len()];
    for (i, line) in run_log.iter().enumerate().rev() {
        restored_after[i] = match error_source(line) {
            Some(source) => restored.get(source).copied().unwrap_or(true),
            None => true,
        };
        let body = log_text(line);
        if let Some((s, value, _)) = cfg2_write(body) {
            if value & (1 << 22) != 0 {
                restored.insert(s, true);
            }
        } else if let Some(source) = log_source(body)
            && let Some((controller, _)) = source.split_once('.')
            && controller.starts_with("spi")
        {
            restored.insert(controller, false);
        }
    }
    let mut exempt = 0u64;
    let mut first = None;
    for (i, line) in run_log.iter().enumerate() {
        if !line.contains(LOG_ERROR) {
            continue;
        }
        let exempted = error_source(line).is_some_and(|source| {
            i > 0
                && restored_after[i]
                && cfg2_write(log_text(run_log[i - 1])).is_some_and(|(s, value, cr1)| {
                    s == source && value & (1 << 22) == 0 && cr1 & 1 == 0
                })
        });
        if exempted {
            exempt += 1;
        } else if first.is_none() {
            first = Some(*line);
        }
    }
    (first, exempt)
}

/// The controller of an SPI model's slave-mode `[ERROR]` line (`spi2`).
fn error_source(line: &str) -> Option<&str> {
    log_text(line)
        .strip_prefix("[ERROR] ")
        .and_then(|b| b.split_once(": "))
        .filter(|(source, text)| source.starts_with("spi") && *text == SLAVE_MODE)
        .map(|(source, _)| source)
}

/// A log line's source after its level (`spi2.spi2mux.u5`).
fn log_source(body: &str) -> Option<&str> {
    body.strip_prefix('[')
        .and_then(|b| b.split_once("] "))
        .and_then(|(_, rest)| rest.split_once(": "))
        .map(|(source, _)| source)
}

/// A foreign boot's report before anything is read: what it is judged by,
/// and the rule table. A boot killed at a bound reports this.
pub fn report(
    plan: &BootPlan<'_>,
    firmware: FirmwareReport,
    specification_sha256: String,
    regions: Vec<RegionReport>,
    clock: ClockReport,
) -> ForeignBoot {
    ForeignBoot {
        judge: JUDGE,
        firmware,
        specification_sha256,
        seed: aethelgard_twin::boot::SEED,
        virtual_time_ns: plan.virtual_ns,
        window: FOREIGN_WINDOW,
        memory_regions: regions,
        mips: None,
        instructions: None,
        pc: None,
        icsr: None,
        booted: None,
        registers: Vec::new(),
        devices: Vec::new(),
        spi_errors_exempted: 0,
        warnings: Vec::new(),
        log_kept_bytes: 0,
        log_budget_bytes: FOREIGN_LOG_FILTER.budget as u64,
        log_lines_dropped: 0,
        flash: Vec::new(),
        erases: Vec::new(),
        option_changes: Vec::new(),
        locked: Vec::new(),
        unobservable: plan.image.unobservable.clone(),
        rules: RULES
            .iter()
            .map(|&(rule, disposition, judges)| RuleReport {
                rule,
                disposition,
                judges,
            })
            .collect(),
        clock,
    }
}

/// The foreign judge: the run's findings, the listing, and `report` (as
/// [`report`] starts it) filled in.
pub fn judge(
    code: i32,
    out: &Output,
    plan: &BootPlan<'_>,
    mut report: ForeignBoot,
    parts: &std::collections::BTreeMap<String, String>,
    flash: &[crate::elf::FlashLoad],
    hse_hz: u32,
) -> Result<(Vec<Finding>, Option<Machine>, ForeignBoot), Stop> {
    let text = out.text();
    let untrusted = |what: String| {
        Stop::tool_error(
            TOOL,
            format!("{what}{}", said::suffix(&text, Style::Renode)),
        )
    };
    if code != 0 {
        return Err(untrusted(format!("renode exited {code}")));
    }
    let frame = frame_bits()?;
    let lines = clean_lines(&text);
    if let Some(stop) = model_load_failed(&lines) {
        return Err(stop);
    }
    let begins: Vec<usize> = (0..lines.len()).filter(|&i| lines[i] == BEGIN).collect();
    let ends: Vec<usize> = (0..lines.len()).filter(|&i| lines[i] == END).collect();
    if let Some(i) = (0..lines.len()).find(|&i| lines[i].contains(LOAD_ERROR)) {
        if !begins.is_empty() || !ends.is_empty() {
            return Err(untrusted(
                "renode reported a command error and also read the machine".into(),
            ));
        }
        let detail = lines.get(i + 1).map(|l| l.trim()).unwrap_or_default();
        let message = format!("{} {detail}", lines[i].trim()).trim().to_owned();
        let kind = if lines[i].contains("error executing command 'machine LoadPlatformDescription")
        {
            "platform_load_error"
        } else if lines[i].contains("error executing command 'sysbus LoadELF") {
            "firmware_load_error"
        } else {
            return Err(untrusted(format!("a monitor command failed: {message}")));
        };
        return Ok((vec![Finding::plain(kind, message)], None, report));
    }
    let (&[begin], &[end]) = (begins.as_slice(), ends.as_slice()) else {
        return Err(untrusted(format!(
            "renode printed {} begin and {} end markers, not one of each",
            begins.len(),
            ends.len()
        )));
    };
    if end < begin {
        return Err(untrusted(
            "renode printed the end marker before the begin marker".into(),
        ));
    }
    let run_log: Vec<&str> = lines[..begin].to_vec();
    let wanted = reads(plan)?;
    let mut values = Vec::with_capacity(wanted.len());
    for line in &lines[begin + 1..end] {
        if value(line).is_some() {
            values.push(*line);
        } else if !is_log_line(line) {
            return Err(untrusted(format!(
                "`{}` between the markers is neither a read nor a whole log line",
                line.escape_debug()
                    .to_string()
                    .chars()
                    .take(120)
                    .collect::<String>()
            )));
        }
    }
    if values.len() != wanted.len() {
        return Err(untrusted(format!(
            "renode printed {} values between the markers, not the {} reads",
            values.len(),
            wanted.len()
        )));
    }
    let mut observed = vec![0u32; plan.image.registers.len()];
    let mut rcc = [0u32; 6];
    let (mut pc, mut icsr) = (0u32, 0u32);
    for ((cmd, read), line) in wanted.iter().zip(values) {
        let v = value(line).ok_or_else(|| untrusted(format!("`{cmd}` read no value")))?;
        let word = || u32::try_from(v).map_err(|_| untrusted(format!("`{cmd}` read {v:#x}")));
        match *read {
            ForeignRead::Instructions => report.instructions = Some(v),
            ForeignRead::Mips => report.mips = Some(v),
            ForeignRead::Pc => pc = word()?,
            ForeignRead::Icsr => icsr = word()?,
            ForeignRead::Register(i) => observed[i] = word()?,
            ForeignRead::Rcc(i) => rcc[i] = word()?,
        }
    }
    report.pc = Some(format!("{pc:#010x}"));
    report.icsr = Some(format!("{icsr:#010x}"));
    let (cpus, peripherals) = listing(&lines[end + 1..]);
    if cpus.is_empty() {
        return Err(untrusted("renode listed no CPU after the reads".into()));
    }
    for (name, model) in plan.instances {
        if !peripherals
            .iter()
            .any(|p| &p.name == name && &p.kind == model)
        {
            return Err(untrusted(format!(
                "the platform's listing lacks the solved `{name}` ({model})"
            )));
        }
    }
    let machine = Machine { cpus, peripherals };
    // Each loaded flash segment's sha256 as Renode holds it after the run,
    // printed after the end marker, one line each, in order.
    let hashes: Vec<&str> = lines[end + 1..]
        .iter()
        .filter_map(|l| l.strip_prefix("AETHELGARD-FLASH "))
        .collect();
    if hashes.len() != flash.len() {
        return Err(untrusted(format!(
            "renode printed {} flash hashes, not the {} loaded flash segments'",
            hashes.len(),
            flash.len()
        )));
    }

    // The timers' declared clock held to the RCC registers (`AET-WO-4400g`).
    let mut findings = report.clock.check(rcc, hse_hz);
    let whole = out.dropped_log_lines == 0;
    if !whole {
        findings.push(Finding::info(
            "renode_log_flood",
            format!(
                "Renode's output passed {} MiB; its log lines past that were counted and dropped, so no witness is read whole",
                LOG_FILTER.budget / (1024 * 1024)
            ),
        ));
    }
    let (error, exempt) = error_lines(&run_log);
    report.spi_errors_exempted = exempt;
    if let Some(line) = error {
        findings.push(Finding::plain(
            "renode_error_log",
            log_text(line).to_owned(),
        ));
    }
    report.log_kept_bytes = out.stdout.len() as u64;
    report.log_lines_dropped = out.dropped_log_lines;
    // D7 (a): each kind of Renode's `Unhandled write` warning, once.
    let stamp = |t: &str| log_stamped(&format!("[{t}] x")).map(|(ns, _)| ns);
    report.warnings = out
        .summaries
        .iter()
        .map(|w| WarningReport {
            line: w.key.clone(),
            count: w.count,
            first_ns: stamp(&w.first),
            last_ns: stamp(&w.last),
        })
        .collect();
    // An access to an unclocked peripheral: info, each address with its first
    // time and count (ruled on PR #212, comment 6965511448).
    for line in run_log
        .iter()
        .filter(|l| LOCKED.iter().any(|w| l.contains(w)))
    {
        let text = log_text(line);
        let access = text
            .split_once(": ")
            .map_or(text, |(_, t)| t)
            .split(", value")
            .next()
            .unwrap_or(text)
            .trim_end_matches('.')
            .to_owned();
        match report.locked.iter_mut().find(|r| r.access == access) {
            Some(r) => r.count += 1,
            None => report.locked.push(LockedReport {
                access,
                first_ns: log_stamped(line).map(|(ns, _)| ns),
                count: 1,
            }),
        }
    }
    if !report.locked.is_empty() {
        findings.push(Finding::info(
            "locked_peripheral_access",
            format!(
                "{} distinct accesses to a peripheral whose clock was off, each in boot.locked; silicon ignores them, so the foreign judge reports them and does not fail",
                report.locked.len()
            ),
        ));
    }
    // No erase of the firmware's own loaded flash, as the erase hook logs it
    // (a sector erased and reprogrammed alike leaves the hash unchanged).
    let erase = Erase::facts()?;
    for line in &run_log {
        let at_ns = log_stamped(line).map(|(ns, _)| ns);
        for started in erase.started(log_text(line)) {
            let (start, end) = match started {
                Started::Erase(start, end) => (start, end),
                Started::Options => {
                    report.option_changes.push(at_ns);
                    continue;
                }
            };
            if let Some(f) = flash
                .iter()
                .find(|f| f.address < end && start < f.address + f.len)
            {
                findings.push(Finding::plain(
                    "flash_overwritten",
                    format!(
                        "the firmware started an erase of {start:#010x}..{end:#010x}, which holds its own loaded flash bytes at {:#010x}",
                        f.address
                    ),
                ));
            }
            report.erases.push(EraseReport {
                at_ns,
                start: format!("{start:#010x}"),
                end: format!("{end:#010x}"),
            });
        }
    }
    if !report.option_changes.is_empty() {
        findings.push(Finding::info(
            "flash_option_change",
            format!(
                "the firmware started {} option-byte changes (OPTSTART), each time in boot.option_changes; their effect on its own segments' protection is not judged",
                report.option_changes.len()
            ),
        ));
    }
    // The firmware's own loaded flash bytes, unchanged after the run.
    for (f, line) in flash.iter().zip(&hashes) {
        let observed = line
            .split_once(' ')
            .filter(|(at, _)| u64::from_str_radix(at, 16).ok() == Some(f.address))
            .map(|(_, h)| h.trim().to_owned())
            .ok_or_else(|| {
                untrusted(format!(
                    "`AETHELGARD-FLASH {line}` is not segment {:#010x}'s",
                    f.address
                ))
            })?;
        if observed != f.sha256 {
            findings.push(Finding::plain(
                "flash_overwritten",
                format!(
                    "the firmware's own loaded flash bytes at {:#010x} ({} bytes) changed during the run",
                    f.address, f.len
                ),
            ));
        }
        report.flash.push(FlashReport {
            address: format!("{:#010x}", f.address),
            bytes: f.len,
            expected_sha256: f.sha256.clone(),
            observed_sha256: observed,
        });
    }
    let vectactive = icsr & 0x1FF;
    if vectactive != 0 {
        findings.push(Finding::plain(
            "core_in_exception",
            format!(
                "the run ended in exception {vectactive} (ICSR {icsr:#010x}), not in thread mode"
            ),
        ));
    }
    if !plan
        .executable
        .iter()
        .any(|&(start, end)| (start..end).contains(&u64::from(pc)))
    {
        findings.push(Finding::plain(
            "pc_outside_firmware",
            format!(
                "the core ended at {pc:#010x}, outside every executable PT_LOAD of the firmware"
            ),
        ));
    }
    for (i, reg) in plan.image.registers.iter().enumerate() {
        let Some((rule, narrowed)) = register_rule(&reg.name, frame) else {
            continue;
        };
        let mask = narrowed.map_or(reg.mask, |n| reg.mask & n);
        let expected = reg.value & mask;
        let got = observed[i];
        let ok = match rule {
            "at_least" => got & expected == expected,
            _ => got & mask == expected,
        };
        report.registers.push(RegisterReport {
            name: reg.name.clone(),
            address: format!("{:#010x}", reg.address),
            rule,
            mask: format!("{mask:#010x}"),
            expected: format!("{expected:#010x}"),
            observed: format!("{got:#010x}"),
        });
        if !ok {
            findings.push(Finding::plain(
                "register_mismatch",
                match rule {
                    "at_least" => format!(
                        "{} reads {got:#010x}, which lacks the solved bits {:#010x} of {expected:#010x}",
                        reg.name,
                        expected & !got
                    ),
                    _ => format!(
                        "{} reads {:#010x} under {mask:#010x}, not the solved {expected:#010x}",
                        reg.name,
                        got & mask
                    ),
                },
            ));
        }
    }

    // The devices, each by its model's own lines (D5, D6 (i)).
    let table = aethelgard_ir::parts::table()
        .map_err(|e| Stop::tool_error("aethelgard-ir", e.to_string()))?;
    let mut references: Vec<&str> = Vec::new();
    for c in &plan.image.identity.checks {
        if !references.contains(&c.reference.as_str()) {
            references.push(&c.reference);
        }
    }
    let mut booted: Option<Booted> = None;
    for reference in references {
        let checks: Vec<&aethelgard_twin::boot::StepCheck> = plan
            .image
            .identity
            .checks
            .iter()
            .filter(|c| c.reference == reference)
            .collect();
        let Some(identity) = checks
            .iter()
            .find(|c| c.step == aethelgard_ir::parts::StepKind::Identity)
        else {
            continue;
        };
        let bus = match identity.wire {
            aethelgard_twin::boot::Wire::I2c { .. } => "i2c",
            aethelgard_twin::boot::Wire::Spi { .. } => "spi",
        };
        let own = own_lines(identity, &run_log);
        let stamp = |k: usize| log_stamped(run_log[own[k].0]).map(|(ns, _)| ns);
        // A device's warning that is not its model's listed artefact fails.
        if let Some((_, w)) = own.iter().find(|(_, b)| b.starts_with("[WARNING]")) {
            findings.push(Finding::plain(
                &format!("{bus}_step_error"),
                format!("{reference} ({}): its model warned: {w}", identity.device),
            ));
        }
        // Identity.
        let Some(want) = identity.witness.as_ref() else {
            findings.push(Finding::plain(
                &format!("{bus}_witness_unavailable"),
                format!(
                    "{reference} ({}): the twin knows no witness format for its identity",
                    identity.device
                ),
            ));
            continue;
        };
        let want: Vec<Line> = want.iter().cloned().map(Line::Exact).collect();
        let at = whole.then(|| find(&own, &want, 0)).flatten();
        let at_ns = at.and_then(stamp);
        report.devices.push(StepReport {
            reference: reference.to_owned(),
            device: identity.device.clone(),
            step: "identity".to_owned(),
            expected: want.iter().map(Line::describe).collect(),
            observed_ns: at_ns,
        });
        let Some(k) = at else {
            findings.push(Finding::plain(
                &format!("{bus}_transaction_unwitnessed"),
                format!(
                    "{reference} ({}): its identity transaction is not in its model's log{}",
                    identity.device,
                    if whole {
                        ""
                    } else {
                        ", which could not be read whole"
                    }
                ),
            ));
            continue;
        };
        if let Some(ns) = at_ns
            && booted.as_ref().is_none_or(|b| ns < b.at_ns)
        {
            booted = Some(Booted {
                reference: reference.to_owned(),
                device: identity.device.clone(),
                at_ns: ns,
            });
        }
        // Data: one of the part's reads, after the identity.
        let after = k + want.len();
        let mut candidates: Vec<(String, Vec<Line>)> = Vec::new();
        if let Some(data) = checks
            .iter()
            .find(|c| c.step == aethelgard_ir::parts::StepKind::Data)
            && let Some(w) = &data.witness
        {
            candidates.push((
                "data (script)".to_owned(),
                w.iter().cloned().map(Line::Exact).collect(),
            ));
        }
        let part = parts
            .get(reference)
            .and_then(|p| table.get(p))
            .map(|p| (p.reads.clone(), p.renode.clone()));
        if let Some((reads, Some(renode))) = part
            && let Some(model) = aethelgard_twin::model::Model::from_renode(&renode)
        {
            for (i, r) in reads.iter().enumerate() {
                match model.read_witness(&identity.device, r) {
                    Some(w) => candidates.push((
                        format!("data (reads[{i}])"),
                        w.into_iter().map(Line::Pattern).collect(),
                    )),
                    None => findings.push(Finding::plain(
                        &format!("{bus}_witness_unavailable"),
                        format!(
                            "{reference} ({}): the twin knows no witness format for its listed read reads[{i}]",
                            identity.device
                        ),
                    )),
                }
            }
        }
        // A data step always has a witness: every modelled part's data
        // channels are ones its model can be set to (the twin's test
        // `every_modelled_part_s_data_channels_can_be_stimulated`; adversarial
        // pass, F3). No data step and no listed read: nothing to judge.
        if candidates.is_empty() {
            continue;
        }
        let mut found = false;
        for (step, want) in &candidates {
            let at = whole.then(|| find(&own, want, after)).flatten();
            let ns = at.and_then(stamp);
            found |= at.is_some();
            report.devices.push(StepReport {
                reference: reference.to_owned(),
                device: identity.device.clone(),
                step: step.clone(),
                expected: want.iter().map(Line::describe).collect(),
                observed_ns: ns,
            });
        }
        if !found {
            findings.push(Finding::plain(
                &format!("{bus}_transaction_unwitnessed"),
                format!(
                    "{reference} ({}): none of its data reads follows its identity in its model's log{}",
                    identity.device,
                    if whole { "" } else { ", which could not be read whole" }
                ),
            ));
        }
    }
    if booted.is_none() {
        findings.push(Finding::plain(
            "foreign_not_booted",
            "no declared device's model logged its identity transaction: nothing the firmware cannot write witnesses that it ran (D5; the console is refused)".to_owned(),
        ));
    }
    report.booted = booted;
    for d in &plan.image.identity.unmodelled {
        findings.push(Finding::info(
            "device_unmodelled",
            format!(
                "{} ({}) {} has no Renode {} model, so its identity was not read",
                d.reference,
                d.part,
                d.wire.describe(&d.peripheral),
                aethelgard_twin::platform::RENODE_VERSION
            ),
        ));
    }
    Ok((findings, Some(machine), report))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D4: the slave-mode `[ERROR]` is exempt only directly after a `CFG2`
    /// write with `MASTER` 0 while the last `CR1` write left `SPE` 0, as
    /// `spi_lld_stop` writes them (`AET-WO-4400e2` D4, measured).
    #[test]
    fn the_slave_mode_error_is_exempt_only_after_a_disabled_cfg2_write_of_master_0() {
        let cfg2 = |value: &str, cr1: &str| {
            format!("[00:00:00.100000000] [WARNING] spi1: {CFG2_MARK}{value} {cr1}")
        };
        let err = format!("[00:00:00.100000000] [ERROR] spi1: {SLAVE_MODE}");
        let other = "[00:00:00.200000000] [ERROR] spi2: something else".to_owned();
        let run = |lines: &[String]| {
            let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
            let (first, n) = error_lines(&refs);
            (first.map(str::to_owned), n)
        };
        // spi_lld_stop: SPE cleared, then CFG2 = 0: exempt.
        assert_eq!(run(&[cfg2("0", "0"), err.clone()]), (None, 1));
        // SPE still set: not exempt.
        assert_eq!(run(&[cfg2("0", "1"), err.clone()]).1, 0);
        // MASTER written 1: not exempt.
        assert_eq!(run(&[cfg2("400000", "0"), err.clone()]).1, 0);
        // Not directly after the write: not exempt.
        assert_eq!(run(&[cfg2("0", "0"), other.clone(), err.clone()]).1, 0);
        // Another controller's write: not exempt.
        let spi2 = format!("[00:00:00.100000000] [WARNING] spi2: {CFG2_MARK}0 0");
        assert_eq!(run(&[spi2, err.clone()]).1, 0);
        // Left a slave while its device goes on: not exempt; restored first:
        // exempt.
        let device =
            "[00:00:00.300000000] [NOISY] spi1.spi1mux.u4: Transmitting data 0x9F".to_owned();
        assert_eq!(run(&[cfg2("0", "0"), err.clone(), device.clone()]).1, 0);
        assert_eq!(
            run(&[
                cfg2("0", "0"),
                err.clone(),
                cfg2("400000", "0"),
                device.clone()
            ])
            .1,
            1
        );
        // Another controller's device does not count against it.
        let other_device = device.replace("spi1.spi1mux", "spi2.spi2mux");
        assert_eq!(run(&[cfg2("0", "0"), err.clone(), other_device]).1, 1);
        // Any other error is the first reported, exempt ones counted.
        assert_eq!(
            run(&[cfg2("0", "0"), err.clone(), other.clone()]),
            (Some(other), 1)
        );
    }

    /// The erase hook's lines, read with the header's facts: a sector erase
    /// names its 128 KiB sector in its bank, a bank erase the whole bank, and
    /// a `START` write with neither `SER` nor `BER` erases nothing.
    #[test]
    fn an_erase_names_the_flash_it_erases_from_the_header_s_facts() {
        let e = Erase::facts().unwrap();
        let line = |offset: &str, value: &str| {
            format!("[WARNING] flashController: {ERASE_MARK}{offset} {value}")
        };
        let erase = |a, b| vec![Started::Erase(a, b)];
        assert_eq!(
            e.started(&line("c", "184")),
            erase(0x0802_0000, 0x0804_0000)
        );
        assert_eq!(
            e.started(&line("10c", "784")),
            erase(0x081E_0000, 0x0820_0000)
        );
        assert_eq!(
            e.started(&line("10c", "88")),
            erase(0x0810_0000, 0x0820_0000)
        );
        // MER erases both banks; OPTSTART is an option-byte change.
        assert_eq!(
            e.started(&line("18", "10")),
            erase(0x0800_0000, 0x0820_0000)
        );
        assert_eq!(e.started(&line("18", "2")), [Started::Options]);
        // Its alias at 0x118 (RM0433 §4.9.7) the same.
        assert_eq!(
            e.started(&line("118", "10")),
            erase(0x0800_0000, 0x0820_0000)
        );
        assert_eq!(e.started(&line("118", "2")), [Started::Options]);
        assert_eq!(
            e.started(&line("18", "12")),
            [Started::Erase(0x0800_0000, 0x0820_0000), Started::Options]
        );
        // START without SER or BER, SER without START, another register: nothing.
        assert!(e.started(&line("c", "80")).is_empty());
        assert!(e.started(&line("c", "104")).is_empty());
        assert!(e.started(&line("14", "184")).is_empty());
        assert!(
            e.started("[WARNING] spi1: AETHELGARD-ERASE c 184")
                .is_empty()
        );
        let hook = e.hook();
        assert!(
            hook.contains("((offset in (0xc, 0x10c)) and (value >> 7) & 1 or offset in (0x18, 0x118) and ((value >> 4) & 1 or (value >> 1) & 1))"),
            "{hook}"
        );
    }

    /// D7 (a): an `Unhandled write` warning's key elides every written value,
    /// so writes of other values to one register are one kind; any other line
    /// has no key.
    #[test]
    fn unhandled_writes_are_one_kind_per_peripheral_and_message() {
        let a = unhandled_write_key(
            "[00:00:01.000000000] [WARNING] nvic: Unhandled write to offset 0xF5C (DCacheInvalidateByMVAToPoCAddress), value 0x30012340.",
        );
        let b = unhandled_write_key(
            "[00:00:02.000000000] [WARNING] nvic: Unhandled write to offset 0xF5C (DCacheInvalidateByMVAToPoCAddress), value 0x3001FFE0.",
        );
        assert_eq!(a, b);
        assert_eq!(
            a.unwrap(),
            "[WARNING] nvic: Unhandled write to offset 0xF5C (DCacheInvalidateByMVAToPoCAddress), value 0x…."
        );
        let other = unhandled_write_key(
            "[00:00:02.000000000] [WARNING] nvic: Unhandled write to offset 0xF70 (DCacheCleanAndInvalidateByMVAToPoCAddress), value 0x1.",
        );
        assert_ne!(other, b);
        for none in [
            "[00:00:01.000000000] [ERROR] spi1: Unhandled write to offset 0x1, value 0x2.",
            "[00:00:01.000000000] [NOISY] i2c1.u2: Write D0",
            "[00:00:01.000000000] [WARNING] spi1: AETHELGARD-CFG2 0 0",
        ] {
            assert!(unhandled_write_key(none).is_none(), "{none}");
        }
    }

    /// The rule table names each of today's rules once, and every dropped or
    /// replaced rule says what it weakens or judges.
    #[test]
    fn every_rule_is_named_once_with_its_disposition() {
        let mut seen = std::collections::BTreeSet::new();
        for (rule, disposition, judges) in RULES {
            assert!(seen.insert(*rule), "{rule} twice");
            assert!(!judges.is_empty(), "{rule}");
            assert!(
                ["kept", "replaced", "dropped"]
                    .iter()
                    .any(|d| disposition.starts_with(d)),
                "{rule}: {disposition}"
            );
            if disposition.starts_with("dropped") {
                assert!(
                    judges.contains("Weakens") || rule == &"records in RAM",
                    "{rule}"
                );
            }
        }
    }

    /// The register rules: pins exact, RCC at least, USART frame bits, the
    /// rest of the USART, SysTick, NVIC and timers dropped.
    #[test]
    fn each_register_family_takes_its_ruled_rule() {
        let frame = frame_bits().unwrap();
        assert_eq!(frame, 1 | 1 << 3 | 1 << 9 | 1 << 10 | 1 << 12 | 1 << 28);
        assert_eq!(register_rule("GPIOA.MODER", frame), Some(("equal", None)));
        assert_eq!(
            register_rule("RCC.AHB4ENR", frame),
            Some(("at_least", None))
        );
        assert_eq!(
            register_rule("USART1.CR1", frame),
            Some(("equal", Some(frame)))
        );
        assert_eq!(register_rule("USART1.CR2", frame), Some(("equal", None)));
        for dropped in [
            "USART1.CR3",
            "USART1.BRR",
            "SysTick.CSR",
            "NVIC.ISER0",
            "TIM1.CR1",
        ] {
            assert_eq!(register_rule(dropped, frame), None, "{dropped}");
        }
    }

    #[test]
    fn a_step_is_the_first_run_of_the_device_s_lines_that_matches_it() {
        let own: Vec<(usize, &str)> = vec![(0, "a"), (1, "b"), (2, "a"), (3, "c")];
        let want =
            |l: &[&str]| -> Vec<Line> { l.iter().map(|s| Line::Exact((*s).to_owned())).collect() };
        assert_eq!(find(&own, &want(&["a", "c"]), 0), Some(2));
        assert_eq!(find(&own, &want(&["a", "b"]), 0), Some(0));
        assert_eq!(find(&own, &want(&["a", "b"]), 1), None);
        assert_eq!(find(&own, &want(&["c", "a"]), 0), None);
        assert_eq!(find(&own, &[], 0), None);
    }

    /// `AET-WO-4400g` §1.2: the timers' declaration held to the RCC registers
    /// ArduPilot leaves (`AET-WO-4400f` §2): 200 MHz holds on both buses; the
    /// stock file's 250 MHz declared is contradicted, by name, with both
    /// figures; registers that do not decode fail by name; and the USARTs
    /// are listed as declared only, with the condition that says so.
    #[test]
    fn the_declared_timer_clock_is_held_to_the_rcc_registers() {
        let regs = [0x0300_7025, 0x801B, 0x48, 0x440, 0x40_8040, 0x0109_0231];
        let declared = |hz: u64| aethelgard_ir::motor::SolvedForeignClock {
            apb1_timers_hz: hz,
            apb2_timers_hz: hz,
            usarts: aethelgard_ir::serial::PERIPHERALS
                .iter()
                .map(|p| ((*p).to_owned(), 72_000_000))
                .collect(),
        };
        let mut r = ClockReport::of(&declared(200_000_000), vec!["dwt", "lpuart1"]);
        assert!(r.check(regs, 0).is_empty());
        assert!(r.checked.iter().all(|c| c.holds && c.decoded_hz == Some(200_000_000)));
        assert_eq!(r.registers[1].name, "RCC_CFGR");
        assert_eq!(r.registers[1].value, "0x0000801b");
        assert_eq!(r.declared_only.len(), 8);
        assert!(r.conditions[0].starts_with("usart_clock_declared_unchecked:"));
        let mut r = ClockReport::of(&declared(250_000_000), vec![]);
        let f = r.check(regs, 0);
        assert_eq!(
            f.iter().map(|f| f.kind.as_str()).collect::<Vec<_>>(),
            [CLOCK_CONTRADICTED, CLOCK_CONTRADICTED]
        );
        assert!(
            f[0].message.contains("declares 250000000 Hz")
                && f[0].message.contains("decode to 200000000 Hz"),
            "{}",
            f[0].message
        );
        assert!(r.checked.iter().all(|c| !c.holds));
        // SWS 100 is reserved: no clock to hold the declaration to.
        let mut r = ClockReport::of(&declared(200_000_000), vec![]);
        let f = r.check([regs[0], 0x8023, regs[2], regs[3], regs[4], regs[5]], 0);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind, CLOCK_UNDECODABLE);
        assert!(r.checked.iter().all(|c| !c.holds && c.decoded_hz.is_none()));
    }

}
