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

use std::path::Path;

use serde::Serialize;

use super::*;
use crate::run::Emit;
use crate::verdict::{Finding, FirmwareReport, Machine};

/// What `boot.judge` says on a foreign verdict.
pub const JUDGE: &str = "foreign";

/// The longest boot `--foreign` admits, in virtual nanoseconds, and its name
/// in the verdict (`AET-WO-4400e` D7, measured with the ICM-42688-P's FIFO
/// live; `AET-WO-4400g` §1.4 since the twin's timers run at ArduPilot's
/// declared 200 MHz, not 250): unmodified ArduPilot's last witness, the ICM's
/// first FIFO read, is at 4.394 s (it was 3.6105 s at 1.25× virtual time),
/// so 5 s leaves 0.61 s. Its RSS margin under concurrency is FEEDBACK's F1
/// measurement; the memory bound itself is a ruling of its own, never widened
/// here: 1.5 GiB since `AET-WO-2313` (`renode::BOOT_BOUNDS`).
pub const FOREIGN_MAX_VIRTUAL_NS: u64 = 5_000_000_000;
pub const FOREIGN_WINDOW: &str = "FOREIGN_MAX_VIRTUAL_NS";

/// The longest boot `--foreign` admits with a declared input (`AET-WO-4400j`
/// D3, ruled 2026-11-25T18:49Z): main's Sentinel-03 arms on `4400i`'s input
/// at 8.752 s (its armed heartbeat's first byte), and 12 s leaves 3.25 s.
/// Inside the bounds of `RUL-13` only since D1 (Renode's stdin held open:
/// 347-435 permille of the cap at 12 s with the input, against 1,756-1,861
/// before) and D2 (the flood summarised); `spike/4400j/RESULTS.md` §11. A run
/// with no input keeps [`FOREIGN_MAX_VIRTUAL_NS`], so no such verdict moves.
pub const FOREIGN_INPUT_MAX_VIRTUAL_NS: u64 = 12_000_000_000;
pub const FOREIGN_INPUT_WINDOW: &str = "FOREIGN_INPUT_MAX_VIRTUAL_NS";

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
        "kept, with AET-WO-4400j D4 (i)",
        "ICSR.VECTACTIVE is 0 at one of the run's four end reads, one quantum apart, the last at the window's end. Weakens: a core in handlers for three quanta at the end, then still in one at its end, is caught; one in handlers at every read is; one out of a handler for a single quantum among four passes (measured on ArduPilot: at most two quanta in a row, AET-WO-4400j)",
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
        "kept, with AET-WO-4400j D4 (i)",
        "each solved pin's mode and alternate function, exactly, at one of the run's four end reads one quantum apart (a register fails only if it fails at all four). Weakens: a register right for one quantum of the last four passes (an SPI controller stopped between transactions holds its pins at most two quanta, measured, AET-WO-4400j)",
    ),
    (
        "USART CR1, CR2",
        "replaced",
        "CR1's frame bits (UE, TE, PS, PCE, M0, M1) and CR2's STOP against the solved framing, read back; and CR2's SWAP, TXINV and RXINV, which Renode does not store, from each declared USART's access log (AET-WO-2311, ruled at §1.2): the last CR2 word written while CR1.UE was clear, the word silicon keeps (RM0433 Rev 9 §48.8.3, p. 2075), held to the specification alone: SWAP clear; on RC input TXINV clear; elsewhere TXINV and RXINV each as framing.inverted. A CR2 write with UE set that changes a judged bit fails (it does not take); a write into CR1 or CR2 that is not a word at its own offset, or no CR2 word written with UE clear, fails. Weakens: CR1.RE (receive enable) is not judged, as receive is held; an RC input port's RXINV is AP_RCProtocol's and unjudged; the words are the twin's sequence, not silicon's: a firmware that reads CR2 back before writing it reads every tag as 0 in the twin, so a clearing write silicon would make may be absent (ArduPilot's set_options, UARTDriver.cpp:1574, :1699-1709). At one of the four end reads, as the pins (AET-WO-4400j D4 (i)); on a declared RC input port, as rc_protocol_search says",
    ),
    (
        "rc_protocol_search",
        "added, AET-WO-4400j D4 (ii)",
        "on a declared RC input port, ArduPilot's protocol search (AP_RCProtocol.cpp L336-396) cycles six serial configurations, 1 s each; CR2.STOP and CR1.PCE, PS and M0 are its, and are not judged at the end read there, each named in boot.rc_search and an info finding. Weakens: an RC port left at a wrong stop, parity or word length at the run's end is not caught",
    ),
    (
        "serial pins' PUPDR",
        "replaced",
        "an info finding per declared port's TX and RX pin naming its pull, read back at the run's end, each in boot.serial_pulls. Weakens: the pull is not judged (roadmap §7.1 L1054 awaits its ruling)",
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
        "--foreign's own bound, FOREIGN_MAX_VIRTUAL_NS, 5 s, or with a declared input FOREIGN_INPUT_MAX_VIRTUAL_NS, 12 s (unmodified ArduPilot's armed heartbeat at 8.752 s on an input run, AET-WO-4400j D3) (the last witness of unmodified ArduPilot at 4.394 s at its declared clock, AET-WO-4400g)",
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
        "kept, with D7 (a) and AET-WO-4400j D2",
        "the 8 MiB budget stands; past it no witness is read whole: every step is unwitnessed. Renode's own Unhandled write warnings are summarised: each kind (peripheral and message, the written value elided) is one line with its count and first and last virtual time in boot.warnings (ruled on PR #212, comment 6965511448, D7 (a)); none is hidden, none floods; at most 1,024 kinds are summarised, and a line of a further kind is kept or dropped as any other, against the budget. AET-WO-4400j D2 (ruled 2026-11-25T18:49Z): each repeating per-transaction line is kept the first time and counted after, with its count and first and last time in boot.summarised, the judge reading the kept log as it would the whole one: an SPI controller's D4 cycle (CFG2 MASTER 0 with SPE 0, the slave-mode error directly after, then CFG2 MASTER 1) by its three lines, counted into spi_errors_exempted, and any other cycle kept as written with its next event; each declared device's and multiplexer's lines by exact text, only once the device's identity and every multi-line candidate are found; a listed read's single-line pattern (the ICM FIFO read), its values elided, after its first match; the ADC's start-while-busy warning by text; a device's warning never. At most 4,096 keys, then lines as written (boot.summary_capped). Weakens: a summarised line's later occurrences are counted, not kept, so a value in one that no rule reads is not in the verdict",
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
        "30 s, 1.5 GiB, an aborted machine",
    ),
    (
        "device_unmodelled",
        "kept",
        "a device with no model is named (info)",
    ),
    (
        "foreign_panic",
        "added (AET-WO-1609)",
        "an execution hook on AP_HAL::panic, found by its symbol in the ELF (a firmware whose symbol table does not name it is refused, ForeignPanicSymbolMissing), fails a run that enters it, whenever it did, with each entry's virtual time in boot.panic. Weakens: a fault, a hang or a reset that never calls it is judged only by the rules above (a thread-mode hang after the last witness passes; spike/1609); the symbol table that locates the hook is the judged ELF's own, so one that names a decoy function in place of the panic hides it (INV-07: it locates the hook, it sets no pass criterion)",
    ),
    (
        "foreign_fault",
        "added (AET-WO-2314)",
        "an execution hook on each fault handler (HardFault, MemManage, BusFault, UsageFault) that the ELF's vector table at FLASH_BANK1_BASE names (a table entry that is 0 or outside every executable PT_LOAD is refused, ForeignFaultVectorUnusable) fails a run whose core takes that exception (ICSR.VECTACTIVE is its number at the handler's entry), naming the vector, the PC the exception stacked and its virtual time, and halts the core there, so a handler's own loop never reaches the log budget or the memory cap (spike/2314). Weakens: only HardFault was exercised (an undefined instruction, escalated); a firmware that takes and handles a fault on purpose fails; a VTOR moved at run time to a table whose handlers lie elsewhere is not hooked, nor is a handler branched to without an exception (both are judged by the rules above: a crash handler run from thread mode still ends at the memory cap, spike/2314); a handler that is also code the core spins on (a shared default handler) runs the hook on each pass, which slows the run toward its wall bound; a PC stacked where nothing answers reads 0, and the message gives the frame's address",
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
    /// At the end read, the window's end.
    pub observed: String,
    /// At the end reads before it, one quantum apart, in order
    /// (`AET-WO-4400j` D4 (i)).
    pub earlier: Vec<String>,
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

/// One declared port's `USART_CR2` options as the foreign judge read them
/// (`AET-WO-2311`).
#[derive(Serialize)]
pub struct SerialOptionsReport {
    pub label: String,
    pub peripheral: String,
    pub role: &'static str,
    pub inverted: bool,
    /// Every `CR2` word write in the access log, `UE` set or clear.
    pub writes: usize,
    /// The last `CR2` word written with `UE` clear, and its virtual time;
    /// `null` when there is none.
    pub judged: Option<String>,
    pub judged_ns: Option<u64>,
    /// `SWAP`, `TXINV`, `RXINV`: each `clear`, `set` or `unjudged`.
    pub expected: Vec<OptionBitReport>,
}

/// One option bit's expectation.
#[derive(Serialize)]
pub struct OptionBitReport {
    pub field: &'static str,
    pub expected: &'static str,
}

/// One serial line's pull, read back at the run's end (`AET-WO-2311` (2)).
#[derive(Serialize)]
pub struct PullReport {
    pub label: String,
    pub line: &'static str,
    pub pin: String,
    pub pupdr: &'static str,
}

/// The finding kinds of the serial options (`AET-WO-2311` D4).
pub const SWAP_SET: &str = "serial_swap_set";
pub const TXINV_WRONG: &str = "serial_txinv_wrong";
pub const RXINV_WRONG: &str = "serial_rxinv_wrong";
pub const OPTIONS_IGNORED: &str = "serial_options_ignored";
pub const OPTIONS_UNJUDGEABLE: &str = "serial_options_unjudgeable";
pub const OPTIONS_UNWRITTEN: &str = "serial_options_unwritten";
pub const PULL: &str = "serial_pull";

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
    /// The fields above that a re-simulation of this same run may report
    /// otherwise (`AET-WO-4412` (ii), ruled 2026-11-26T15:27Z), each with why;
    /// empty, and absent, on a platform without the timer model.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub not_rerun_stable: Vec<Unstable>,
    pub icsr: Option<String>,
    /// ICSR at the end reads before the last, in order (`AET-WO-4400j` D4 (i)).
    pub icsr_earlier: Vec<String>,
    /// Each RC port's register whose search fields the end read left
    /// unjudged, and those bits ([`RC_SEARCH`], `AET-WO-4400j` D4 (ii)).
    pub rc_search: Vec<String>,
    pub booted: Option<Booted>,
    pub registers: Vec<RegisterReport>,
    pub devices: Vec<StepReport>,
    /// D4: the SPI model's slave-mode `[ERROR]` lines exempted, each directly
    /// after a `CFG2` write with `MASTER` 0 while `SPE` was 0.
    pub spi_errors_exempted: u64,
    /// Renode's `Unhandled write` warnings, each kind once (D7 (a)).
    pub warnings: Vec<WarningReport>,
    /// Each line the log's summary counted (`AET-WO-4400j` D2): its key (the
    /// line without its timestamp; a D4 cycle's three lines; a pattern's
    /// elided form), how often, and the first and last time.
    pub summarised: Vec<WarningReport>,
    /// Whether the summary reached [`SUMMARY_KEYS`] and kept the rest as
    /// written.
    pub summary_capped: bool,
    /// The bytes of Renode's output the log filter kept and counted against
    /// `log_budget_bytes`, and the log lines it dropped past it (D7 (c)).
    pub log_kept_bytes: u64,
    pub log_budget_bytes: u64,
    pub log_lines_dropped: u64,
    /// The firmware's loaded flash segments, read back after the run.
    pub flash: Vec<FlashReport>,
    /// Each flash erase the firmware started, as the erase hook logs it.
    pub erases: Vec<EraseReport>,
    /// Each declared serial port's `CR2` options (`AET-WO-2311`).
    pub serial_options: Vec<SerialOptionsReport>,
    /// Each declared serial line's pull at the run's end (`AET-WO-2311`).
    pub serial_pulls: Vec<PullReport>,
    /// The virtual time of each option-byte change it started (`info`).
    pub option_changes: Vec<Option<u64>>,
    /// Accesses to a peripheral whose clock was off (`info`).
    pub locked: Vec<LockedReport>,
    pub unobservable: Vec<String>,
    pub rules: Vec<RuleReport>,
    /// The clock the platform was built at, and what of it was checked
    /// (`AET-WO-4400g` §1.2).
    pub clock: ClockReport,
    /// ArduPilot's `AP_HAL::panic`, hooked where its symbol is, and every
    /// entry the hook logged (`AET-WO-1609`).
    pub panic: PanicReport,
    /// The DShot frames each paced transfer moved (`AET-WO-4412` D4), on a
    /// platform with the timer and DMA models; absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dshot: Option<DshotReport>,
    /// Each reset the firmware asked for (`SYSRESETREQ`), by its virtual time
    /// (`AET-WO-4412` (4)); absent from a run with none.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub resets_ns: Vec<Option<u64>>,
    /// A declared input's run (`AET-WO-4400i`); absent without `--input`, so
    /// a run without one reports as before.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<InputRun>,
    /// Without an input, each IMU at a witness stimulus that is not at rest,
    /// named (`AET-WO-4400i` §2); absent when none is.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub not_at_rest: Vec<String>,
}

/// The panic hook's evidence (`AET-WO-1609`): the symbol, the address the
/// hook was set at, and the virtual time of each entry it logged, in order.
#[derive(Serialize, Clone)]
pub struct PanicReport {
    pub symbol: &'static str,
    pub address: String,
    /// `null` until the judge has read the run's log (a run ended at a
    /// bound is never read), so "not read" is never "no entry".
    pub entered_ns: Option<Vec<Option<u64>>>,
}

/// ArduPilot's panic, `AP_HAL::panic(const char*, ...)` (`AP_HAL/system.h`;
/// `AP_HAL_ChibiOS/system.cpp:327` prints and sleeps forever in the calling
/// thread), by its mangled name in the ELF's symbol table.
pub const PANIC_SYMBOL: &str = "_ZN6AP_HAL5panicEPKcz";

/// The line the panic hook logs on each entry, with the PC.
const PANIC_MARK: &str = "AETHELGARD-PANIC ";

/// The execution hook on the panic's first instruction (`AET-WO-1609`, ruled
/// at §1.2): each entry logs a `[WARNING]` of the CPU's own, which no
/// firmware can write. A panic leaves the core in thread mode, parked, with
/// every witness already logged (`spike/1609`), so nothing else the judge
/// reads tells it from a healthy run.
fn panic_hook(address: u64) -> String {
    format!(
        "cpu AddHook {address:#x} \"__import__('Antmicro.Renode.Logging').Renode.Logging.Logger.Log(self, __import__('Antmicro.Renode.Logging').Renode.Logging.LogLevel.Warning, '{PANIC_MARK}%x' % pc)\""
    )
}

/// The virtual time of an entry the panic hook logged at `address`, as
/// `Some(stamp)`; `None` for any other line.
fn panic_entry(line: &str, address: u64) -> Option<Option<u64>> {
    let pc = log_text(line)
        .strip_prefix("[WARNING] cpu: ")?
        .strip_prefix(PANIC_MARK)?;
    (u64::from_str_radix(pc.trim(), 16).ok()? == address)
        .then(|| log_stamped(line).map(|(ns, _)| ns))
}

/// Renode 1.17.0's NVIC on a write of `SCB_AIRCR.SYSRESETREQ`
/// (`NVIC.cs`, measured on Sentinel-03's ArduPilot, `spike/4412`): it pauses
/// the machine, resets every peripheral, and runs the machine's `reset`
/// macro (`AET-WO-4412` (4)).
pub const RESET_MARK: &str = "[INFO] nvic: Resetting platform with SYSRESETREQ";

/// `RCC_RSR`'s offset in the RCC (RM0433 Rev 9 §8.7.? "RCC reset status
/// register (RCC_RSR)", p. 449: offset 0x0D0; ArduPilot's `watchdog.c` L39).
pub const RSR_OFFSET: u32 = 0xD0;
/// `RCC_RSR` after a system reset generated by the CPU (RM0433 Rev 9 Table
/// 56 row 4, p. 333): `SFTRSTF` (bit 24), `PINRSTF` (22) and `CPURSTF` (17),
/// the flags ArduPilot reads at boot (`stm32_was_software_reset`,
/// `watchdog.c` L131) and clears with `RMVF` (`board.c` L291–293) before it.
pub const RSR_SOFTWARE_RESET: u32 = (1 << 24) | (1 << 22) | (1 << 17);
/// `RCC_RSR.RMVF` (bit 16): a write of 1 clears every flag (§8.4.4, p. 333).
pub const RSR_RMVF: u32 = 1 << 16;

/// What the twin's reboot keeps and loses against silicon's system reset
/// (`AET-WO-4412` (4), ruled D5), named in every verdict of a run that
/// rebooted.
pub const REBOOT_RESIDUAL: &str = "foreign_reboot: the firmware reset the core by SCB_AIRCR.SYSRESETREQ; the twin's reset is Renode 1.17.0's (every peripheral to its model's reset, memories kept), then the judge's macro: the cold RCC writes, the vector table at the boot address, and RCC_RSR reading RM0433 Rev 9 Table 56 row 4 (SFTRSTF, PINRSTF, CPURSTF; p. 333) until RMVF clears it. Kept, as on silicon: flash (both banks), SRAM and the TCMs, the backup domain (RTC registers). Weakens: RCC_RSR after the reset is row 4's alone, as after ArduPilot's own RMVF at boot (board.c L291-293); a firmware that never clears it would read the power-on flags too, and before a reset RMVF is not modelled (Renode's RCC ignores it)";

/// The `reset` macro Renode runs on `SYSRESETREQ` (`AET-WO-4412` (4), ruled
/// D5): no `LoadELF` (flash survives Renode's reset, measured), the image's
/// cold RCC writes and the vector table at `vector_table` (without it the core
/// takes its initial SP and PC from address 0, measured), then the hooks that
/// make `RCC_RSR` read [`RSR_SOFTWARE_RESET`] until a write with
/// [`RSR_RMVF`] clears it, installed here so that no boot before a reset is
/// touched. Written to the scratch before any tool runs, and included.
pub fn reset_script(image: &aethelgard_twin::boot::Image, vector_table: u32) -> String {
    let state = "__import__('sys').aethelgard_rsr";
    let mut lines: Vec<String> = image
        .cold
        .iter()
        .map(|r| format!("sysbus WriteDoubleWord {:#010x} {:#x}", r.address, r.value))
        .collect();
    lines.push(format!("cpu VectorTableOffset {vector_table:#010x}"));
    lines.push(format!("python \"{state} = {RSR_SOFTWARE_RESET:#010x}\""));
    lines.push(format!(
        "sysbus SetHookAfterPeripheralRead sysbus.rcc \"if offset == {RSR_OFFSET:#x}: value = {state}\""
    ));
    lines.push(format!(
        "sysbus SetHookBeforePeripheralWrite sysbus.rcc \"if offset == {RSR_OFFSET:#x} and value & {RSR_RMVF:#x}: {state} = 0\""
    ));
    format!("macro reset\n\"\"\"\n{}\n\"\"\"\n", lines.join("\n"))
}

/// A field a re-simulation may report otherwise, and why.
#[derive(Serialize, Clone)]
pub struct Unstable {
    pub field: &'static str,
    pub why: &'static str,
}

/// The foreign verdict's fields that are not re-run stable on a platform with
/// the DShot timer and DMA models (`AET-WO-4412` (ii)): no committed golden,
/// CI assertion, package or expectation compares them (a tier-1 test holds
/// it). `pc_outside_firmware` still judges `boot.pc`; both outcomes measured
/// lie in the firmware's flash.
pub const NOT_RERUN_STABLE: [&str; 2] = ["boot.instructions", "boot.pc"];

pub const NOT_RERUN_STABLE_WHY: &str = "Renode 1.17.0 re-simulation is not bit-stable: on Sentinel-03 dshot12, 5 of 19 runs with the same inputs ended 6 instructions apart (the CPU at __port_switch, not __port_exit_from_isr, from 11.280174680 s), with no model state, transfer word, USART byte or stamp differing; stock Renode moved 18 log stamps by 10 ns in 1 of 20 (AET-WO-4412 spike/4412/proto/NOTES.md §9). A replay of this run's trace is deterministic (INV-04); a fresh re-simulation is not";

/// Whether the platform loads the DShot timer and DMA models.
fn loads_dshot_models(models: &[std::path::PathBuf]) -> bool {
    models.iter().any(|m| {
        m.file_name()
            .is_some_and(|f| f == "AethelgardSTM32Timer.cs" || f == "AethelgardSTM32DMA.cs")
    })
}

/// The DMA model's line for a paced stream in circular mode (`AET-WO-4412`
/// (a)).
pub const UNBOUNDED_MARK: &str = "AETHELGARD-DMA-UNBOUNDED";
/// A paced stream that would never end, refused by name.
pub const DMA_STREAM_UNBOUNDED: &str = "foreign_dma_stream_unbounded";
/// A DShot frame whose checksum fails, or that is not 16 pulses.
pub const DSHOT_FRAME_MALFORMED: &str = "foreign_dshot_frame_malformed";

/// Where the judge read the DShot frames (`AET-WO-4412` D4, D2 of `2302`):
/// stated in every verdict that has them.
pub const DSHOT_READ_AT: &str = "each paced transfer's words, from the DMA model's line as it ended (models/renode/AethelgardSTM32DMA.cs): the compare values the timer's update requests burst into CCRx through DMAR. The timer is never stopped by the judge: it runs on to the window's end, where its PSC and ARR are read";

/// How many malformed frames a position lists.
pub const MALFORMED_LISTED: usize = 8;

/// The DShot frames of a foreign run, per burst position.
#[derive(Serialize, Clone)]
pub struct DshotReport {
    pub read: &'static str,
    pub transfers: Vec<DshotStream>,
}

/// One paced stream's transfers.
#[derive(Serialize, Clone)]
pub struct DshotStream {
    pub timer: String,
    pub request: u32,
    pub stream: u32,
    pub count: u64,
    pub first_ns: u64,
    pub last_ns: u64,
    /// Per burst position (`CCR(DBA + k)`): frames, malformed ones, and the
    /// last good frame's throttle and telemetry request.
    pub positions: Vec<DshotPosition>,
}

#[derive(Serialize, Clone)]
pub struct DshotPosition {
    pub position: usize,
    pub frames: u64,
    /// Transfers whose words at this position are all 0: no pulse, the line
    /// held low, so no frame left (the firmware's own buffer; not malformed).
    pub empty: u64,
    pub malformed: u64,
    /// The first [`MALFORMED_LISTED`] malformed frames: where each transfer
    /// ended, and what was wrong.
    pub malformed_at: Vec<String>,
    pub last_throttle: Option<u16>,
    pub telemetry_requests: u64,
}

/// The DShot report and its findings from the run's kept lines: each line of
/// the DMA model's, decoded at its timer's period (`ARR + 1`, as the end
/// read found it).
fn dshot_report(
    lines: &[&str],
    image: &aethelgard_twin::boot::Image,
    observed: &[[u32; END_READS]],
) -> (Option<DshotReport>, Vec<Finding>) {
    use aethelgard_twin::dshot::{Frame, frames, transfer};
    let mut streams: Vec<DshotStream> = Vec::new();
    let mut findings = Vec::new();
    for line in lines {
        let text = log_text(line);
        let Some(body) = text
            .strip_prefix("[NOISY] dma1: ")
            .or_else(|| text.strip_prefix("[NOISY] dma2: "))
        else {
            continue;
        };
        let Some(t) = transfer(body) else {
            continue;
        };
        let Some(&(timer, _)) = aethelgard_ir::motor::UP_REQUESTS
            .iter()
            .find(|(_, r)| *r == t.request)
        else {
            continue;
        };
        let arr = image
            .registers
            .iter()
            .position(|r| r.name == format!("TIM{timer}.ARR"))
            .map(|i| observed[i][END_READS - 1]);
        let width = if t.requests == 0 {
            0
        } else {
            t.words.len() / t.requests as usize
        };
        let s = match streams
            .iter_mut()
            .find(|s| s.stream == t.stream && s.request == t.request)
        {
            Some(s) => s,
            None => {
                streams.push(DshotStream {
                    timer: format!("TIM{timer}"),
                    request: t.request,
                    stream: t.stream,
                    count: 0,
                    first_ns: t.end_ns,
                    last_ns: t.end_ns,
                    positions: (0..width)
                        .map(|position| DshotPosition {
                            position,
                            frames: 0,
                            empty: 0,
                            malformed: 0,
                            malformed_at: Vec::new(),
                            last_throttle: None,
                            telemetry_requests: 0,
                        })
                        .collect(),
                });
                streams.last_mut().expect("just pushed")
            }
        };
        s.count += 1;
        s.last_ns = t.end_ns;
        let Some(arr) = arr else {
            continue;
        };
        for (k, f) in frames(&t, width, arr + 1).into_iter().enumerate() {
            let Some(p) = s.positions.get_mut(k) else {
                continue;
            };
            p.frames += 1;
            match f {
                Frame::Packet {
                    throttle,
                    telemetry,
                } => {
                    p.last_throttle = Some(throttle);
                    p.telemetry_requests += u64::from(telemetry);
                }
                Frame::Pulses { count: 0 } => p.empty += 1,
                bad => {
                    p.malformed += 1;
                    if p.malformed_at.len() < MALFORMED_LISTED {
                        p.malformed_at.push(format!("{} ns: {bad:?}", t.end_ns));
                    }
                    if p.malformed == 1 {
                        findings.push(Finding::plain(
                            DSHOT_FRAME_MALFORMED,
                            format!(
                                "TIM{timer}'s DShot frame at burst position {k}, the transfer ending at {} ns (DMA stream {}, request {}): {bad:?}",
                                t.end_ns, t.stream, t.request
                            ),
                        ));
                    }
                }
            }
        }
    }
    let report = (!streams.is_empty()).then_some(DshotReport {
        read: DSHOT_READ_AT,
        transfers: streams,
    });
    (report, findings)
}

/// The virtual time of a [`RESET_MARK`] line, or `None` for any other./// The virtual time of a [`RESET_MARK`] line, or `None` for any other.
fn reset_entry(line: &str) -> Option<Option<u64>> {
    (log_text(line) == RESET_MARK).then(|| log_stamped(line).map(|(ns, _)| ns))
}

/// Each fault exception the judge hooks (`AET-WO-2314`): its number, which
/// is its vector table index and `ICSR.VECTACTIVE` in its handler, and its
/// name (ARMv7-M ARM B1.5.2).
pub const FAULT_VECTORS: [(u32, &str); 4] = [
    (3, "HardFault"),
    (4, "MemManage"),
    (5, "BusFault"),
    (6, "UsageFault"),
];

/// The line a fault hook logs on its exception's entry: the exception's
/// number, the handler's address and the PC the exception stacked.
const FAULT_MARK: &str = "AETHELGARD-FAULT ";

/// Each fault vector's handler, read from the vector table at
/// `vector_table` in the firmware's loaded bytes, Thumb bit cleared
/// (`AET-WO-2314`, ruled D2 (i)): a table not in the loaded bytes, or an
/// entry that is 0 or outside every executable `PT_LOAD`, cannot be hooked,
/// so the firmware is refused by name before a boot (`INV-06`).
pub fn fault_handlers(
    bytes: &[u8],
    vector_table: u32,
    executable: &[(u64, u64)],
) -> Result<Vec<(u32, u64)>, Stop> {
    let unusable = |m: String| Stop::refused("ForeignFaultVectorUnusable", m);
    let first = u64::from(vector_table) + 4 * u64::from(FAULT_VECTORS[0].0);
    let words = crate::elf::loaded_words(bytes, first, FAULT_VECTORS.len())?.ok_or_else(|| {
        unusable(format!(
            "no single PT_LOAD's file bytes hold the vector table's fault entries at {first:#010x}, so the foreign judge cannot hook the firmware's fault handlers"
        ))
    })?;
    FAULT_VECTORS
        .iter()
        .zip(words)
        .map(|(&(n, name), word)| {
            let address = u64::from(word & !1);
            if word == 0 || !executable.iter().any(|&(s, e)| (s..e).contains(&address)) {
                return Err(unusable(format!(
                    "the vector table's {name} entry (vector {n}, at {:#010x}) reads {word:#010x}, {}, so the foreign judge cannot hook it",
                    u64::from(vector_table) + 4 * u64::from(n),
                    if word == 0 { "no handler" } else { "outside every executable PT_LOAD" }
                )));
            }
            Ok((n, address))
        })
        .collect()
}

/// The execution hooks on the fault handlers, one per distinct address
/// (`AET-WO-2314`, spike/2314): at a handler's first instruction, when
/// `ICSR.VECTACTIVE` is one of the vectors it serves, the CPU logs its own
/// `[WARNING]` with the exception, the handler, and the address of the PC
/// stacked on the frame (PSP when `EXC_RETURN` bit 2 is set, else MSP, at
/// +24) with what it reads (0 where nothing answers there), then
/// halts, so the handler's loop never runs. A handler entered any other way
/// (a call, or a moved table) logs nothing and runs on.
fn fault_hooks(handlers: &[(u32, u64)]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut seen: Vec<u64> = Vec::new();
    for &(_, address) in handlers {
        if seen.contains(&address) {
            continue;
        }
        seen.push(address);
        let vectors: String = handlers
            .iter()
            .filter(|(_, a)| *a == address)
            .map(|(n, _)| format!("{n}, "))
            .collect();
        out.push(format!(
            "cpu AddHook {address:#x} \"r = lambda x: int(x.RawValue) if hasattr(x, 'RawValue') else int(x); v = self.Bus.ReadDoubleWord({icsr:#x}) & {mask:#x}; ok = v in ({vectors}); lr = r(self.LR); f = r(self.OtherSP) if lr & 4 else r(self.SP); ok and __import__('Antmicro.Renode.Logging').Renode.Logging.Logger.Log(self, __import__('Antmicro.Renode.Logging').Renode.Logging.LogLevel.Warning, '{FAULT_MARK}%d %x %x %x' % (v, pc, f + 24, self.Bus.ReadDoubleWord(f + 24))); ok and setattr(self, 'IsHalted', True)\"",
            icsr = aethelgard_twin::boot::ICSR,
            mask = aethelgard_twin::boot::VECTACTIVE_MASK,
        ));
    }
    out
}

/// A fault hook's entry in `line`: the exception's number, where the PC was
/// stacked and what it read, and the virtual time, when the line is the
/// CPU's own mark at one of `handlers` for an exception that handler serves;
/// `None` for any other.
fn fault_entry(line: &str, handlers: &[(u32, u64)]) -> Option<(u32, u64, u64, Option<u64>)> {
    let rest = log_text(line)
        .strip_prefix("[WARNING] cpu: ")?
        .strip_prefix(FAULT_MARK)?;
    let mut f = rest.split(' ');
    let n: u32 = f.next()?.parse().ok()?;
    let handler = u64::from_str_radix(f.next()?, 16).ok()?;
    let frame = u64::from_str_radix(f.next()?, 16).ok()?;
    let stacked = u64::from_str_radix(f.next()?, 16).ok()?;
    if f.next().is_some() || !handlers.contains(&(n, handler)) {
        return None;
    }
    Some((n, frame, stacked, log_stamped(line).map(|(ns, _)| ns)))
}

/// `foreign_fault` for the first fault hook's entry in `text`, if any
/// (`AET-WO-2314`): a run that took a fault fails by name, with the vector
/// and the PC it stacked, whether or not it then reached a bound. Every
/// line: only the CPU writes the mark, which the filter keeps past its
/// budget.
pub fn fault_finding(text: &str, handlers: &[(u32, u64)]) -> Option<Finding> {
    let (n, frame, stacked, ns) = text.lines().find_map(|l| fault_entry(l, handlers))?;
    let name = FAULT_VECTORS
        .iter()
        .find(|v| v.0 == n)
        .map_or("a fault", |v| v.1);
    Some(Finding::plain(
        "foreign_fault",
        format!(
            "the core took {name} (exception {n}) at {}, from PC {stacked:#010x} (read from its stack frame at {frame:#010x}); its handler's hook halted the core there",
            ns.map_or("an unstamped time".to_owned(), |t| format!("{t} ns"))
        ),
    ))
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
    /// The declared voltage scale (`mcu.foreign_clock.vos`).
    #[serde(skip)]
    pub declared_vos: aethelgard_ir::vos::Vos,
    /// The voltage-scaling rule at the run's end (`AET-WO-4240v` D-4): each
    /// clock the decode reaches judged at the declared scale at its fastest;
    /// one over its maximum is a named condition in `conditions`
    /// (`vos_limit_at_boundary`), never a failure. `null` until the
    /// registers decode.
    pub vos: Option<aethelgard_ir::vos::ForeignVerdict>,
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
pub const STOCK_UNDECLARED: &str = "stock_frequency_undeclared: the instances in `stock` keep the stock file's frequencies (dwt 250 MHz, lptimer1-5 32 MHz, lpuart1 125 MHz), which no declaration sets; no judge reads any of them";

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
            declared_vos: c.vos,
            vos: None,
        }
    }

    /// The check at the run's end: `values`, the RCC registers in
    /// [`RccRegisters::OFFSETS`]' order, decoded with the solved `clock`'s
    /// declared crystal (RM0433 Rev 9, `SolvedClock::from_rcc`) and judged at
    /// its declared deviation on an HSE tree (`AET-WO-4412` D2), and each
    /// timer bus's decoded clock held to its declaration. Each disagreement
    /// is a failing finding with both figures.
    ///
    /// [`RccRegisters::OFFSETS`]: aethelgard_ir::motor::RccRegisters::OFFSETS
    fn check(
        &mut self,
        values: [u32; 6],
        hse_hz: u32,
        hse: Option<aethelgard_ir::clock::Budget>,
        d3cfgr: Option<u32>,
    ) -> Vec<Finding> {
        use aethelgard_ir::motor::{Apb, RccRegisters};
        self.registers = RccRegisters::OFFSETS
            .iter()
            .zip(values)
            .map(|(&(name, _), v)| RccRead {
                name,
                value: format!("{v:#010x}"),
            })
            .collect();
        let registers = RccRegisters::from_values(values);
        let decoded = match aethelgard_ir::SolvedClock::from_rcc(
            &registers,
            hse_hz,
            self.declared_vos,
        ) {
            Ok(c) => c,
            Err(e) => {
                return vec![Finding::plain(
                    CLOCK_UNDECODABLE,
                    format!(
                        "the RCC registers read back at the run's end do not decode to a timer kernel clock: {e}"
                    ),
                )];
            }
        };
        let usarts = self
            .declared_only
            .iter()
            .map(|d| (d.instance.clone(), d.declared_hz))
            .collect();
        let vos = aethelgard_ir::vos::judge_foreign(&registers, &decoded, &usarts, hse, d3cfgr);
        self.conditions.extend(vos.conditions.iter().cloned());
        self.vos = Some(vos);
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
    keep: &[
        ABORTED,
        LOAD_ERROR,
        LOG_ERROR,
        LOCKED[0],
        LOCKED[1],
        MODEL_LOAD_ERROR,
        PANIC_MARK,
        FAULT_MARK,
        RESET_MARK,
        UNBOUNDED_MARK,
    ],
    discard: unread_access_line,
    summarise: unhandled_write_key,
    ..LOG_FILTER
};

/// [`LOG_FILTER`]'s discard, and each USART write to its `TDR`, at any
/// width (`AET-WO-2311` A5, built in `AET-WO-2314`): no foreign rule reads
/// one (`judge_serial` reads `CR1` and `CR2` only), while the generated
/// judge does, so its filter keeps them. ArduPilot's console writes about
/// 600 in 5 s; this trims the log and does not move Renode's own memory.
fn unread_access_line(line: &str) -> bool {
    // Each `RunFor` of an input's timeline pauses and resumes the machine,
    // two `[INFO]` lines per byte sent, which no rule reads (`AET-WO-4400i`).
    let body = log_body(line).unwrap_or("");
    if body.starts_with("[INFO] machine-")
        && (body.ends_with(": Machine paused.") || body.ends_with(": Machine resumed."))
    {
        return true;
    }
    unjudged_access_line(line)
        || log_body(line)
            .and_then(aethelgard_twin::serial::access)
            .is_some_and(|a| {
                a.write
                    && usart_tdr().is_some_and(|tdr| a.offset & !3 == tdr)
                    && usarts().contains(&a.source.as_str())
            })
}

/// A USART's `TDR` offset, from the header, read once; `None` if the header
/// cannot be read (then no `TDR` write is discarded).
fn usart_tdr() -> Option<u32> {
    static TDR: std::sync::OnceLock<Option<u32>> = std::sync::OnceLock::new();
    *TDR.get_or_init(|| {
        let f = aethelgard_svd_hal::cmsis::register_facts().ok()?;
        f.layout("USART").ok()?.offset("TDR", 0).ok()
    })
}

/// Every stock USART's Renode name, read once.
fn usarts() -> &'static [&'static str] {
    static NAMES: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    NAMES.get_or_init(aethelgard_twin::serial::stock_uarts)
}

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
/// `CFG2` hook (D4), each declared serial port's USART access log
/// (`AET-WO-2311` D2), the run, then the reads between the markers, each
/// loaded flash segment's sha256, and the listing. No boot word, no record,
/// no hook on the firmware's own code and no injection: none of the
/// generated runtime's contract.
pub fn commands(
    plan: &BootPlan<'_>,
    flash: &[crate::elf::FlashLoad],
    serial: &[aethelgard_twin::serial::Options],
    panic: u64,
    faults: &[(u32, u64)],
    input: Option<(&Path, &[(u64, String)])>,
    reset: Option<&Path>,
) -> Result<String, Stop> {
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
    // The reboot (`AET-WO-4412` (4)): the macro Renode runs on SYSRESETREQ.
    c.extend(reset.map(|r| format!("include @{}", r.display())));
    // The DMA model's one line per paced transfer (`AET-WO-4412` D4), its
    // only `Noisy` line, where the platform loads it.
    if loads_dshot_models(plan.models) {
        c.extend([
            "logLevel -1 sysbus.dma1".to_owned(),
            "logLevel -1 sysbus.dma2".to_owned(),
        ]);
    }
    c.extend(plan.witness_levels());
    c.extend(spi_controllers(plan).iter().map(|s| cfg2_hook(s)));
    c.extend(usart_logs(serial));
    c.push(Erase::facts()?.hook());
    c.push(panic_hook(panic));
    c.extend(fault_hooks(faults));
    c.extend(plan.stimuli());
    // A declared input's timeline (`AET-WO-4400i`): its stimuli, bytes and
    // run to the first end read, from a script file; its events after that
    // instant are the tail, run between the end reads.
    let mut block = vec![format!("echo \"{BEGIN}\"")];
    block.extend(reads(plan, serial)?.into_iter().map(|(cmd, _)| cmd));
    block.push(format!("echo \"{END}\""));
    let (now, tail) = match input {
        Some((script, tail)) => {
            c.push(format!("include @{}", script.display()));
            (end_read_instants(plan.virtual_ns)[0], tail)
        }
        None => (0, &[][..]),
    };
    c.extend(end_commands(now, plan.virtual_ns, tail, &block));
    c.extend(flash.iter().map(flash_hash));
    c.push("peripherals".to_owned());
    c.push("quit".to_owned());
    Ok(c.join("; "))
}

/// How many times the end of the run is read (`AET-WO-4400j` D4 (i), ruled
/// 2026-11-25T18:49Z): four reads one quantum apart, the last at the
/// window's end, never past it. A register, or the core in an exception, is
/// judged at the end only if it fails at every one. Measured on
/// Sentinel-03's ArduPilot, per quantum over a second: an SPI controller
/// stopped between transactions, or the core in a handler, held at most two
/// quanta in a row (`spike/4400j/RESULTS.md` §4), and either fails a single
/// read about one end in fifty.
pub const END_READS: usize = 4;

/// The instants of the [`END_READS`] end reads, one quantum apart, the last
/// at `virtual_ns`; for a window shorter than three quanta, the earlier ones
/// are at 0, so no read is ever past the window.
pub fn end_read_instants(virtual_ns: u64) -> [u64; END_READS] {
    let q = aethelgard_twin::boot::QUANTUM_NS;
    std::array::from_fn(|k| virtual_ns.saturating_sub((END_READS - 1 - k) as u64 * q))
}

/// The run from `now` to the window's end with the end reads: each `tail`
/// event (an input's, after the first end read's instant) at its instant, and
/// `block` (the markers and the reads) at each of [`end_read_instants`],
/// after the events at the same instant.
fn end_commands(
    now: u64,
    virtual_ns: u64,
    tail: &[(u64, String)],
    block: &[String],
) -> Vec<String> {
    let mut out = Vec::new();
    let mut now = now;
    let mut events = tail.iter().cloned().peekable();
    for at in end_read_instants(virtual_ns) {
        let due = std::iter::from_fn(|| events.next_if(|(t, _)| *t <= at));
        run_to(&mut out, &mut now, due, at);
        out.extend(block.iter().cloned());
    }
    out
}

/// Each event at its instant from `now`, a `RunFor` before each later one,
/// then the run to `until`.
fn run_to(
    out: &mut Vec<String>,
    now: &mut u64,
    events: impl IntoIterator<Item = (u64, String)>,
    until: u64,
) {
    for (t, cmd) in events {
        if t > *now {
            out.push(run_for(t - *now));
            *now = t;
        }
        out.push(cmd);
    }
    if until > *now {
        out.push(run_for(until - *now));
        *now = until;
    }
}

/// Each declared serial port's USART, its accesses logged (`AET-WO-2311`
/// D2): the access log carries each write's width, which a write hook's
/// scope does not (measured, `spike/2311` P7), so a half-word write to `CR2`
/// is never read as a word. Once per USART.
fn usart_logs(serial: &[aethelgard_twin::serial::Options]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for o in serial {
        let c = format!("sysbus LogPeripheralAccess sysbus.{} true", o.renode);
        if !out.contains(&c) {
            out.push(c);
        }
    }
    out
}

/// Each `PUPDR` a declared serial line's pin is in, by name and address,
/// once each, in the ports' order (TX, then RX).
fn pull_registers(serial: &[aethelgard_twin::serial::Options]) -> Vec<(String, u32)> {
    let mut out: Vec<(String, u32)> = Vec::new();
    for pin in serial.iter().flat_map(|o| [&o.tx, &o.rx]) {
        let r = pin.pupdr();
        if !out.contains(&r) {
            out.push(r);
        }
    }
    out
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
    /// `RCC_D3CFGR`, for `fPCLK4` (`AET-WO-4412` D2).
    D3cfgr,
    /// The `PUPDR` at this index of [`pull_registers`].
    Pull(usize),
}

fn reads(
    plan: &BootPlan<'_>,
    serial: &[aethelgard_twin::serial::Options],
) -> Result<Vec<(String, ForeignRead)>, Stop> {
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
    for (i, (_, offset)) in aethelgard_ir::motor::RccRegisters::OFFSETS
        .iter()
        .enumerate()
    {
        r.push((
            format!("sysbus ReadDoubleWord {:#010x}", rcc + offset),
            ForeignRead::Rcc(i),
        ));
    }
    r.push((
        format!(
            "sysbus ReadDoubleWord {:#010x}",
            rcc + aethelgard_ir::motor::RCC_D3CFGR.1
        ),
        ForeignRead::D3cfgr,
    ));
    for (i, (_, address)) in pull_registers(serial).iter().enumerate() {
        r.push((
            format!("sysbus ReadDoubleWord {address:#010x}"),
            ForeignRead::Pull(i),
        ));
    }
    Ok(r)
}

/// A motor declared DShot whose timer, at the run's end, does not run at its
/// protocol's bit rate (`AET-WO-4412` D3, ruled 2026-11-26): the expectation
/// is the specification's (`motor_outputs[].protocol`, its bit rate), the
/// observation the end read's `PSC` and `ARR` at the frequency the platform
/// builds the timer at. Below half the bit rate the timer is not counting
/// DShot bits: unmodified ArduPilot runs PWM until a `MOT_PWM_TYPE` is set
/// (its default is PWM, and the generated hwdef sets none). Info, never a
/// failure, one per timer; `None` where the image does not read the timer.
pub const MOTOR_PROTOCOL_UNMET: &str = "foreign_motor_protocol_unmet";

pub(crate) fn motor_protocol_unmet(
    image: &aethelgard_twin::boot::Image,
    motors: &[crate::renode::Motor],
    observed: &[[u32; END_READS]],
) -> Vec<Finding> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for m in motors {
        let o = &m.observer;
        let Some(bit_rate) = o.protocol.bit_rate() else {
            continue;
        };
        if !seen.insert(o.timer) {
            continue;
        }
        let at = |r: &str| {
            let name = format!("TIM{}.{r}", o.timer);
            image
                .registers
                .iter()
                .position(|reg| reg.name == name)
                .map(|i| observed[i][END_READS - 1])
        };
        let (Some(psc), Some(arr)) = (at("PSC"), at("ARR")) else {
            continue;
        };
        let cycles = (u64::from(psc) + 1) * (u64::from(arr) + 1);
        let rate = o.kernel_hz / cycles.max(1);
        if rate * 2 >= u64::from(bit_rate) {
            continue;
        }
        let labels: Vec<&str> = motors
            .iter()
            .filter(|n| n.observer.timer == o.timer)
            .map(|n| n.observer.label.as_str())
            .collect();
        out.push(Finding::info(
            MOTOR_PROTOCOL_UNMET,
            format!(
                "ArduPilot runs PWM on a board declared DShot: {} on TIM{} are declared {} ({bit_rate} bit/s), but at the run's end TIM{} updates at {rate} Hz (PSC {psc:#x}, ARR {arr:#x} at {} Hz), not at a DShot bit rate. MOT_PWM_TYPE is an ArduPilot parameter, PWM by default: `generate hwdef` writes it into the board's defaults.parm (AET-WO-4412 (3)), so this ELF was built without that file, or did not embed it",
                labels.join(", "),
                o.timer,
                o.protocol.name(),
                o.timer,
                o.kernel_hz
            ),
        ));
    }
    out
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

/// What the end read leaves unjudged on a port the specification declares
/// as an RC input (`AET-WO-4400j` D4 (ii), ruled 2026-11-25T18:49Z).
pub const RC_SEARCH: &str = "rc_protocol_search: on a port the specification declares as an RC input (rc_crsf, rc_sbus), unmodified ArduPilot searches for the protocol: Copter-4.7.2's AP_RCProtocol.cpp (serial_configs, L336-352; check_added_uart, L359-396) applies the next of six serial configurations every 1 s until one is detected, two of them 8E2, so a run with no receiver leaves the port at two stop bits for 2 s of every 6 (spike/4400j/RESULTS.md section 4). At the end read, that port's CR2.STOP and CR1.PCE, PS and M0 are not judged; every other field of its CR1 and CR2, and every other port, is";

/// The fields [`RC_SEARCH`] leaves unjudged, as `(CR1 mask, CR2 mask)`, each
/// position from ST's header (`RUL-16`): `CR1`'s `PCE`, `PS` and `M0`, and
/// `CR2`'s `STOP`.
fn rc_search_fields() -> Result<(u32, u32), Stop> {
    let hal = |e: aethelgard_svd_hal::cmsis::RegisterError| {
        Stop::tool_error("aethelgard-svd-hal", e.to_string())
    };
    let f = aethelgard_svd_hal::cmsis::register_facts().map_err(hal)?;
    let mut cr1 = 0u32;
    for name in ["PCE", "PS", "M0"] {
        cr1 |= 1 << f.position(&format!("USART_CR1_{name}_Pos")).map_err(hal)?;
    }
    let (at, width) = f.field("USART_CR2_STOP").map_err(hal)?;
    Ok((cr1, ((1u32 << width) - 1) << at))
}

/// The mask the end read judges register `name` (`USART3.CR2`) under, and
/// the bits D4 (ii) took out of `mask`: [`rc_search_fields`]' (`rc`), on a
/// port the specification declares as an RC input, of its `CR1` and `CR2`
/// alone; 0 everywhere else.
fn end_mask(
    name: &str,
    mask: u32,
    serial: &[aethelgard_twin::serial::Options],
    rc: (u32, u32),
) -> (u32, u32) {
    let Some((instance, register)) = name.split_once('.') else {
        return (mask, 0);
    };
    let is_rc = serial.iter().any(|o| {
        o.peripheral == instance
            && matches!(
                o.role,
                aethelgard_ir::SerialRole::RcCrsf | aethelgard_ir::SerialRole::RcSbus
            )
    });
    let fields = match (is_rc, register) {
        (true, "CR1") => rc.0,
        (true, "CR2") => rc.1,
        _ => 0,
    };
    (mask & !fields, mask & fields)
}

/// A device's data candidates, as the judge looks for each after its
/// identity (D5, D6 (i)): its script's read, the same read at each value a
/// declared input's rest set (`AET-WO-4400i` D4), and each read its parts
/// row lists, as its model logs it; with a finding for each listed read the
/// twin knows no witness format for. One list, read by the judge and by the
/// log's summary (`AET-WO-4400j` D2), which never drops a line a candidate
/// not yet found would match.
fn data_candidates(
    reference: &str,
    identity: &aethelgard_twin::boot::StepCheck,
    checks: &[&aethelgard_twin::boot::StepCheck],
    rest: &std::collections::BTreeMap<String, Vec<Vec<String>>>,
    parts: &std::collections::BTreeMap<String, String>,
    table: &aethelgard_ir::parts::PartsTable,
    bus: &str,
) -> (Vec<(String, Vec<Line>)>, Vec<Finding>) {
    let mut findings = Vec::new();
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
    // A declared input's rest moves the device's values (`AET-WO-4400i`
    // D4): the same read, at each value the rest set.
    for (i, w) in rest.get(reference).into_iter().flatten().enumerate() {
        candidates.push((
            format!("data (rest {i})"),
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
    (candidates, findings)
}

/// One expected line of a step: the model's whole line, or a listed read's
/// pattern.
#[derive(Clone)]
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

/// Each declared serial port's `CR2` options, from the access log of `run_log`
/// (read whole, or every port is unjudgeable), held to the specification by
/// [`aethelgard_twin::serial::judge_options`]; and each line's pull from
/// `pulls`, the [`pull_registers`] read back, as info (`AET-WO-2311`, ruled
/// at §1.2), with the verdict's `serial_options` and `serial_pulls`.
fn judge_serial(
    run_log: &[&str],
    whole: bool,
    serial: &[aethelgard_twin::serial::Options],
    pulls: &[u32],
) -> (Vec<Finding>, Vec<SerialOptionsReport>, Vec<PullReport>) {
    let (mut options, mut pull_reports) = (Vec::new(), Vec::new());
    use aethelgard_twin::serial::{Expect, access, judge_options, pull_name};
    let mut out = Vec::new();
    let (at, accesses): (Vec<usize>, Vec<aethelgard_twin::serial::Access>) = run_log
        .iter()
        .enumerate()
        .filter_map(|(k, l)| access(log_text(l)).map(|a| (k, a)))
        .unzip();
    let ns = |i: usize| log_stamped(run_log[at[i]]).map(|(ns, _)| ns);
    let when = |i: usize| {
        ns(i).map_or_else(
            || "at an unstamped line".to_owned(),
            |t| format!("at {t} ns"),
        )
    };
    for o in serial {
        let port = format!("{} ({}, {})", o.label, o.peripheral, o.role.name());
        let j = judge_options(o, &accesses);
        options.push(SerialOptionsReport {
            label: o.label.clone(),
            peripheral: o.peripheral.clone(),
            role: o.role.name(),
            inverted: o.inverted,
            writes: j.writes,
            judged: j.judged.map(|(w, _)| format!("{w:#010x}")),
            judged_ns: j.judged.and_then(|(_, i)| ns(i)),
            expected: o
                .bits
                .iter()
                .map(|(field, _, e)| OptionBitReport {
                    field,
                    expected: e.name(),
                })
                .collect(),
        });
        if !whole {
            out.push(Finding::plain(
                OPTIONS_UNJUDGEABLE,
                format!("{port}: its USART's access log could not be read whole, so the CR2 word silicon keeps is not known"),
            ));
            continue;
        }
        for (i, what) in &j.unjudgeable {
            out.push(Finding::plain(
                OPTIONS_UNJUDGEABLE,
                format!("{port}: {what} {}, a write into CR1 or CR2 that is not a word at the register's own offset, which no rule reads (fail closed, AET-WO-2311 D3)", when(*i)),
            ));
        }
        for (i, word, fields) in &j.ignored {
            out.push(Finding::plain(
                OPTIONS_IGNORED,
                format!("{port}: CR2 {word:#010x} written {} with CR1.UE set changes {}, which silicon ignores: SWAP, TXINV and RXINV can only be written when the USART is disabled (RM0433 Rev 9 §48.8.3, p. 2075)", when(*i), fields.join(" and ")),
            ));
        }
        let Some((word, i)) = j.judged else {
            out.push(Finding::plain(
                OPTIONS_UNWRITTEN,
                format!("{port}: no CR2 word was written with CR1.UE clear before the window's end, so SWAP, TXINV and RXINV were never set as the specification declares"),
            ));
            continue;
        };
        for (field, want) in &j.wrong {
            let (kind, why) = match (*field, want) {
                ("SWAP", _) => (SWAP_SET, "serial_ports declares no swap".to_owned()),
                ("TXINV", Expect::Clear) if o.bits[2].2 == Expect::Unjudged => (
                    TXINV_WRONG,
                    "on RC input TXINV is clear (RXINV is AP_RCProtocol's, unjudged; AET-WO-4404 FEEDBACK §2.5)".to_owned(),
                ),
                ("TXINV", _) => (TXINV_WRONG, format!("framing.inverted is {}", o.inverted)),
                _ => (RXINV_WRONG, format!("framing.inverted is {}", o.inverted)),
            };
            let does = match want {
                Expect::Set => "clears",
                _ => "sets",
            };
            out.push(Finding::plain(
                kind,
                format!("{port}: CR2 {word:#010x}, the last word written with CR1.UE clear before the window's end ({}), {does} {field}; {why}", when(i)),
            ));
        }
    }
    let registers = pull_registers(serial);
    for o in serial {
        for (line, pin) in [("TX", &o.tx), ("RX", &o.rx)] {
            let k = registers.iter().position(|r| *r == pin.pupdr());
            let word = pulls[k.expect("pull_registers holds every serial pin's PUPDR")];
            let field = pin.config_fields([0, 0, word]).2;
            out.push(Finding::info(
                PULL,
                format!("{} {line} ({}, {}): PUPDR {} ({field:02b}) at the run's end, read back; not judged (roadmap §7.1 L1054)", o.label, pin.pin, o.peripheral, pull_name(field)),
            ));
            pull_reports.push(PullReport {
                label: o.label.clone(),
                line: if line == "TX" { "tx" } else { "rx" },
                pin: pin.pin.clone(),
                pupdr: pull_name(field),
            });
        }
    }
    (out, options, pull_reports)
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
    panic: u64,
    input: bool,
) -> ForeignBoot {
    ForeignBoot {
        judge: JUDGE,
        firmware,
        specification_sha256,
        seed: aethelgard_twin::boot::SEED,
        virtual_time_ns: plan.virtual_ns,
        window: if input {
            FOREIGN_INPUT_WINDOW
        } else {
            FOREIGN_WINDOW
        },
        memory_regions: regions,
        mips: None,
        instructions: None,
        pc: None,
        not_rerun_stable: Vec::new(),
        icsr: None,
        icsr_earlier: Vec::new(),
        rc_search: Vec::new(),
        booted: None,
        registers: Vec::new(),
        devices: Vec::new(),
        spi_errors_exempted: 0,
        warnings: Vec::new(),
        summarised: Vec::new(),
        summary_capped: false,
        log_kept_bytes: 0,
        log_budget_bytes: FOREIGN_LOG_FILTER.budget as u64,
        log_lines_dropped: 0,
        flash: Vec::new(),
        erases: Vec::new(),
        serial_options: Vec::new(),
        serial_pulls: Vec::new(),
        option_changes: Vec::new(),
        panic: PanicReport {
            symbol: PANIC_SYMBOL,
            address: format!("{panic:#010x}"),
            entered_ns: None,
        },
        dshot: None,
        resets_ns: Vec::new(),
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
        input: None,
        not_at_rest: Vec::new(),
    }
}

/// The foreign judge: the run's findings, the listing, and `report` (as
/// [`report`] starts it) filled in.
#[allow(clippy::too_many_arguments)]
pub fn judge(
    code: i32,
    out: &Output,
    plan: &BootPlan<'_>,
    mut report: ForeignBoot,
    parts: &std::collections::BTreeMap<String, String>,
    flash: &[crate::elf::FlashLoad],
    clock: &aethelgard_ir::SolvedClock,
    serial: &[aethelgard_twin::serial::Options],
    faults: &[(u32, u64)],
    rest: &std::collections::BTreeMap<String, Vec<Vec<String>>>,
    motors: &[crate::renode::Motor],
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
    // `AET-WO-4400j` D4: the reads are taken [`END_READS`] times, one quantum
    // apart, each between its own pair of markers, the last at the end.
    if begins.len() != END_READS || ends.len() != END_READS {
        return Err(untrusted(format!(
            "renode printed {} begin and {} end markers, not {END_READS} of each",
            begins.len(),
            ends.len()
        )));
    }
    if (0..END_READS).any(|k| ends[k] < begins[k] || (k > 0 && begins[k] < ends[k - 1])) {
        return Err(untrusted(
            "renode printed the begin and end markers out of order".into(),
        ));
    }
    let end = ends[END_READS - 1];
    // The run's log: every line outside the read blocks, up to the last.
    let mut run_log: Vec<&str> = lines[..begins[0]].to_vec();
    for k in 1..END_READS {
        run_log.extend_from_slice(&lines[ends[k - 1] + 1..begins[k]]);
    }
    let wanted = reads(plan, serial)?;
    let mut blocks: Vec<Vec<&str>> = Vec::with_capacity(END_READS);
    for k in 0..END_READS {
        let mut values = Vec::with_capacity(wanted.len());
        for line in &lines[begins[k] + 1..ends[k]] {
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
        blocks.push(values);
    }
    // Each image register and ICSR at every end read; the rest at the end's.
    let mut observed = vec![[0u32; END_READS]; plan.image.registers.len()];
    let mut icsr = [0u32; END_READS];
    let mut rcc = [0u32; 6];
    let mut d3cfgr: Option<u32> = None;
    let mut pulls = vec![0u32; pull_registers(serial).len()];
    let mut pc = 0u32;
    for (k, values) in blocks.iter().enumerate() {
        let last = k == END_READS - 1;
        for ((cmd, read), line) in wanted.iter().zip(values) {
            let v = value(line).ok_or_else(|| untrusted(format!("`{cmd}` read no value")))?;
            let word = || u32::try_from(v).map_err(|_| untrusted(format!("`{cmd}` read {v:#x}")));
            match *read {
                ForeignRead::Instructions if last => report.instructions = Some(v),
                ForeignRead::Mips if last => report.mips = Some(v),
                ForeignRead::Instructions | ForeignRead::Mips => {}
                ForeignRead::Pc => {
                    let w = word()?;
                    if last {
                        pc = w;
                    }
                }
                ForeignRead::Icsr => icsr[k] = word()?,
                ForeignRead::Register(i) => observed[i][k] = word()?,
                ForeignRead::Rcc(i) => {
                    let w = word()?;
                    if last {
                        rcc[i] = w;
                    }
                }
                ForeignRead::D3cfgr => {
                    let w = word()?;
                    if last {
                        d3cfgr = Some(w);
                    }
                }
                ForeignRead::Pull(i) => {
                    let w = word()?;
                    if last {
                        pulls[i] = w;
                    }
                }
            }
        }
    }
    report.pc = Some(format!("{pc:#010x}"));
    if loads_dshot_models(plan.models) {
        report.not_rerun_stable = NOT_RERUN_STABLE
            .iter()
            .map(|&field| Unstable {
                field,
                why: NOT_RERUN_STABLE_WHY,
            })
            .collect();
    }
    report.icsr = Some(format!("{:#010x}", icsr[END_READS - 1]));
    report.icsr_earlier = icsr[..END_READS - 1]
        .iter()
        .map(|i| format!("{i:#010x}"))
        .collect();
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
    let mut findings = report.clock.check(
        rcc,
        clock.hse_hz,
        clock
            .hse
            .as_ref()
            .map(aethelgard_ir::clock::SolvedHse::budget),
        d3cfgr,
    );
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
    // D2 (i): each exempt cycle the summary counted and dropped, beside those
    // the kept log shows.
    let summarised_cycles: u64 = out
        .noted
        .iter()
        .filter(|n| n.key.contains(CFG2_MARK) && n.key.contains(SLAVE_MODE))
        .map(|n| n.count - 1)
        .sum();
    report.spi_errors_exempted = exempt + summarised_cycles;
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
    let reports = |v: &[crate::run::Summary]| -> Vec<WarningReport> {
        v.iter()
            .map(|w| WarningReport {
                line: w.key.clone(),
                count: w.count,
                first_ns: stamp(&w.first),
                last_ns: stamp(&w.last),
            })
            .collect()
    };
    report.warnings = reports(&out.summaries);
    // D2: every summarised line, by its key, with its count and first and
    // last time; and whether the summary reached its cap.
    report.summarised = reports(&out.noted);
    report.summary_capped = out.summary_capped;
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
    // In an exception at every end read (D4 (i)); the message is the end's.
    let vectactive = icsr[END_READS - 1] & 0x1FF;
    if icsr.iter().all(|i| i & 0x1FF != 0) {
        findings.push(Finding::plain(
            "core_in_exception",
            format!(
                "the run ended in exception {vectactive} (ICSR {:#010x}), not in thread mode, at each of its {END_READS} end reads one quantum apart",
                icsr[END_READS - 1]
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
    findings.extend(motor_protocol_unmet(plan.image, motors, &observed));
    let rc_fields = rc_search_fields()?;
    for (i, reg) in plan.image.registers.iter().enumerate() {
        let Some((rule, narrowed)) = register_rule(&reg.name, frame) else {
            continue;
        };
        let (mask, unjudged) = end_mask(
            &reg.name,
            narrowed.map_or(reg.mask, |n| reg.mask & n),
            serial,
            rc_fields,
        );
        if unjudged != 0 {
            report
                .rc_search
                .push(format!("{}: {unjudged:#010x} unjudged", reg.name));
        }
        let expected = reg.value & mask;
        let holds = |got: u32| match rule {
            "at_least" => got & expected == expected,
            _ => got & mask == expected,
        };
        // It fails only if it fails at every end read (D4 (i)); the message
        // and `observed` are the end's.
        let got = observed[i][END_READS - 1];
        let ok = observed[i].iter().any(|&g| holds(g));
        report.registers.push(RegisterReport {
            name: reg.name.clone(),
            address: format!("{:#010x}", reg.address),
            rule,
            mask: format!("{mask:#010x}"),
            expected: format!("{expected:#010x}"),
            observed: format!("{got:#010x}"),
            earlier: observed[i][..END_READS - 1]
                .iter()
                .map(|g| format!("{g:#010x}"))
                .collect(),
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

    // D4 (ii), by name, with what it left unjudged.
    if !report.rc_search.is_empty() {
        findings.push(Finding::info(
            "rc_protocol_search",
            format!(
                "{}; here: {}",
                RC_SEARCH.trim_start_matches("rc_protocol_search: "),
                report.rc_search.join(", ")
            ),
        ));
    }

    // Each declared serial port's options and pulls (`AET-WO-2311`).
    let (serial_findings, options, serial_pulls) = judge_serial(&run_log, whole, serial, &pulls);
    findings.extend(serial_findings);
    report.serial_options = options;
    report.serial_pulls = serial_pulls;

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
        let (candidates, unavailable) =
            data_candidates(reference, identity, &checks, rest, parts, table, bus);
        findings.extend(unavailable);
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
    // The firmware's panic, as its hook logs each entry (`AET-WO-1609`):
    // a run that panicked fails by name, whenever it panicked.
    let address = u64::from_str_radix(report.panic.address.trim_start_matches("0x"), 16)
        .map_err(|_| untrusted(format!("the panic hook's address {}", report.panic.address)))?;
    // Every line, not only the run's log before the begin marker: only the
    // CPU writes the hook's line, so one Renode printed late still counts
    // (adversarial review, finding 3). The first is kept past the log budget
    // (`FOREIGN_LOG_FILTER`).
    let entered: Vec<Option<u64>> = lines
        .iter()
        .filter_map(|line| panic_entry(line, address))
        .collect();
    if let Some(first) = entered.first() {
        let last_witness = report.devices.iter().filter_map(|d| d.observed_ns).max();
        let at =
            |ns: &Option<u64>| ns.map_or("an unstamped time".to_owned(), |n| format!("{n} ns"));
        findings.push(Finding::plain(
            "foreign_panic",
            format!(
                "the firmware entered {PANIC_SYMBOL} at {:#010x} {} time(s), first at {}, the last device witness at {}",
                address,
                entered.len(),
                at(first),
                at(&last_witness),
            ),
        ));
    }
    report.panic.entered_ns = Some(entered);
    // A reboot (`AET-WO-4412` (4)): each reset the firmware asked for, and
    // what the twin's reset keeps and loses, named.
    let resets: Vec<Option<u64>> = lines.iter().filter_map(|l| reset_entry(l)).collect();
    if !resets.is_empty() {
        let at =
            |ns: &Option<u64>| ns.map_or("an unstamped time".to_owned(), |n| format!("{n} ns"));
        findings.push(Finding::info(
            "foreign_reboot",
            format!(
                "the firmware reset the core {} time(s), first at {}: {}",
                resets.len(),
                at(&resets[0]),
                REBOOT_RESIDUAL.trim_start_matches("foreign_reboot: ")
            ),
        ));
    }
    report.resets_ns = resets;
    // A paced stream in circular mode, refused by the DMA model (`AET-WO-4412`
    // (a)): it would run until the wall bound.
    for line in lines
        .iter()
        .filter(|l| log_text(l).contains(UNBOUNDED_MARK))
    {
        findings.push(Finding::plain(
            DMA_STREAM_UNBOUNDED,
            format!(
                "{}: the DMA model refused it, not started (CIRC, RM0433 Rev 9 p. 664; §15.3.10, p. 647: NDTR is reloaded and the requests go on being served)",
                log_text(line)
            ),
        ));
    }
    let (dshot, malformed) = dshot_report(&lines, plan.image, &observed);
    findings.extend(malformed);
    report.dshot = dshot;
    // A fault exception, as its hook logs the entry that halted the core
    // (`AET-WO-2314`), before the handler's own loop could reach a bound.
    findings.extend(fault_finding(&text, faults));
    Ok((findings, Some(machine), report))
}

// ---------------------------------------------------------------------------
// A declared input (`AET-WO-4400i`, ruled on PR #396): timed MAVLink frames
// for one declared serial port, sent at their stated virtual times, the board
// at rest, the witness read from the firmware's own transmission, and the run
// archived so replay needs no file (`INV-04`).
// ---------------------------------------------------------------------------

use aethelgard_twin::{mavlink, rest};

/// The input file's format version.
pub const INPUT_VERSION: u64 = 1;

/// The gap every input run names (ruling D3): what the twin cannot do that
/// silicon does, and what the input's declared fill stands in for.
pub const IDLE_GAP: &str = "usart_idle_unmodelled: Renode 1.17.0's USART raises no IDLE (STM32F7_USART.cs:154, :231, a tag), and its DMA neither ends a stream with TCIF when software clears EN nor restarts the next at M0AR (STM32DMA.cs:237-262; RM0433 Rev 9 §15.3.16), so a firmware that receives by DMA reads a frame only when its buffer fills; an input fills it with the zero bytes it declares as `fill`, which silicon would not see (AET-WO-4400i D3)";

/// What a foreign run's `replay` says its `input_verdict` is.
pub const INPUT_VERDICT_IS: &str = "the input's witness re-judged from the trace (the input, the stimuli and the firmware's transmission), not the run's verdict: the trace does not hold the run's other findings";

/// A `PARAM_SET` the firmware answered with `PARAM_ERROR` (ruling D9).
pub const PARAM_REJECTED: &str = "foreign_param_rejected";
/// The input's port transmitted nothing the judge could read.
pub const CAPTURE_UNREADABLE: &str = "foreign_capture_unreadable";

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct InputFile {
    input_version: u64,
    port: String,
    frames: Vec<FrameFile>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct FrameFile {
    at_ns: u64,
    mavlink: String,
    #[serde(default)]
    fill: u32,
}

/// A declared input, read and checked before any tool is looked for.
pub struct Declared {
    pub path: String,
    pub sha256: String,
    /// The file's bytes, verbatim: the trace holds them (kind 38).
    pub file: Vec<u8>,
    pub label: String,
    pub peripheral: String,
    pub renode: String,
    pub spacing_ns: u64,
    pub frames: Vec<mavlink::InputFrame>,
    pub parsed: Vec<mavlink::Frame>,
    pub stimuli: Vec<rest::Stimulus>,
}

fn invalid(kind: &'static str, message: String) -> Stop {
    Stop {
        code: crate::EXIT_INVALID_SPEC,
        kind,
        id: None,
        message,
    }
}

fn twin_refused(e: aethelgard_twin::TwinError) -> Stop {
    invalid(e.kind(), e.to_string())
}

/// An input with no frame sends nothing, so a run under it is the no-input
/// run with a declared input's window and archive (`AET-WO-4400j` F11): it is
/// refused by its field, as the reader and replay read it (`AET-WO-1919` (3),
/// `INV-06`).
fn frames_present(f: &InputFile) -> Result<(), Stop> {
    if f.frames.is_empty() {
        return Err(Stop {
            code: crate::EXIT_INVALID_SPEC,
            kind: "InputMalformed",
            id: Some("frames".to_owned()),
            message: "the input's `frames` is empty: a declared input sends at least one frame"
                .to_owned(),
        });
    }
    Ok(())
}

/// The input file's bytes: a regular file, still one once open (no swap to
/// a FIFO between the check and the open), read to at most
/// [`aethelgard_chaos::record::MAX_FOREIGN_BYTES`].
pub fn read_input(path: &Path) -> Result<(String, Vec<u8>), Stop> {
    use std::io::Read;
    let original = crate::input::file_check(path)?;
    let unreadable = |e: std::io::Error| Stop::unreadable(format!("{}: {e}", original.display()));
    let file = std::fs::File::open(&original).map_err(unreadable)?;
    if !file.metadata().map_err(unreadable)?.is_file() {
        return Err(Stop::unreadable(format!(
            "{} is not a regular file",
            original.display()
        )));
    }
    let max = aethelgard_chaos::record::MAX_FOREIGN_BYTES;
    let mut bytes = Vec::new();
    file.take(max as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(unreadable)?;
    if bytes.len() > max {
        return Err(invalid(
            "InputMalformed",
            format!(
                "{} is over the {max} bytes an input may be",
                original.display()
            ),
        ));
    }
    Ok((original.display().to_string(), bytes))
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

/// An input's port label, its USART, the time between two of its bytes, and
/// its frames, as written and as parsed.
type Frames = (
    String,
    String,
    u64,
    Vec<mavlink::InputFrame>,
    Vec<mavlink::Frame>,
);

/// The port's label, its USART, the time between two of its bytes, and the
/// input's frames, as the file states them: every refusal but the rest's.
pub fn frames_of(
    file: &[u8],
    ports: &std::collections::BTreeMap<String, aethelgard_ir::serial::SolvedSerialPort>,
    window_ns: u64,
) -> Result<Frames, Stop> {
    let f: InputFile = serde_json::from_slice(file)
        .map_err(|e| invalid("InputMalformed", format!("the input file: {e}")))?;
    if f.input_version != INPUT_VERSION {
        return Err(invalid(
            "InputMalformed",
            format!(
                "the input file is version {}; this reader is version {INPUT_VERSION}",
                f.input_version
            ),
        ));
    }
    frames_present(&f)?;
    let port = ports.get(&f.port).ok_or_else(|| {
        invalid(
            "InputPortUndeclared",
            format!(
                "the input names port `{}`, which the specification does not declare (it declares {})",
                f.port,
                ports.keys().cloned().collect::<Vec<_>>().join(", ")
            ),
        )
    })?;
    let spacing_ns = aethelgard_twin::serial::spacing_ns(&port.framing, port.actual_millibaud)
        .ok_or_else(|| {
            invalid(
                "InputMalformed",
                format!("port `{}` has no character time", f.port),
            )
        })?;
    let mut frames = Vec::with_capacity(f.frames.len());
    for (index, fr) in f.frames.iter().enumerate() {
        let bytes = unhex(&fr.mavlink).ok_or_else(|| {
            invalid(
                "InputMalformed",
                format!("frame {index}'s `mavlink` is not an even number of hex digits"),
            )
        })?;
        frames.push(mavlink::InputFrame {
            at_ns: fr.at_ns,
            bytes,
            fill: fr.fill,
        });
    }
    let table = mavlink::messages().map_err(twin_refused)?;
    let parsed = mavlink::check_input(&frames, spacing_ns, window_ns, &table)
        .map_err(|e| invalid(e.kind(), e.to_string()))?;
    Ok((f.port, port.peripheral.clone(), spacing_ns, frames, parsed))
}

/// A declared input read, checked and given its rest stimuli.
pub fn declare(
    path: String,
    file: Vec<u8>,
    topology: &aethelgard_ir::SolvedTopology,
    checks: &[aethelgard_twin::boot::StepCheck],
    window_ns: u64,
) -> Result<Declared, Stop> {
    let (label, peripheral, spacing_ns, frames, parsed) =
        frames_of(&file, &topology.serial_ports, window_ns)?;
    let stimuli = rest::stimuli(topology, checks, window_ns).map_err(twin_refused)?;
    use sha2::Digest;
    Ok(Declared {
        path,
        sha256: crate::input::hex(sha2::Sha256::digest(&file)),
        renode: peripheral.to_ascii_lowercase(),
        file,
        label,
        peripheral,
        spacing_ns,
        frames,
        parsed,
        stimuli,
    })
}

/// The most keys the foreign log's summary keeps (`AET-WO-4400j` D2): past
/// them, every line is kept as written, and the verdict says the cap was hit
/// (`boot.summary_capped`). Measured: a 12 s armed run of Sentinel-03 keys
/// 1,488 lines (975 of them the flash model's data and address texts) and
/// keeps 895,449 bytes; at 1,024 keys it was capped and kept 900,373, so the
/// cap bounds the CLI's memory, not the log.
pub const SUMMARY_KEYS: usize = 4096;

/// The key of a D4 cycle's summary: the controller's three lines, each
/// without its timestamp.
fn cycle_key(s0: &str, e: &str, m: &str) -> String {
    format!("{} | {} | {}", log_text(s0), log_text(e), log_text(m))
}

/// A Renode model warning of a peripheral no rule reads, summarised by its
/// exact text (D2 (iv)): the ADC's (`STM32_ADC`) start-while-busy warning,
/// one per conversion ArduPilot starts early on Sentinel-03, 92 KB per virtual
/// second (`spike/4400j/RESULTS.md` §1.1).
fn unread_model_warning(body: &str) -> bool {
    body.strip_prefix("[WARNING] adc")
        .and_then(|r| r.split_once(": "))
        .is_some_and(|(n, text)| {
            !n.is_empty()
                && n.bytes().all(|b| b.is_ascii_digit())
                && text == "Issued a start event before the last sequence finished"
        })
}

/// One SPI controller's D4 cycle, as the summary holds it.
#[derive(Default)]
enum Cycle {
    #[default]
    Idle,
    /// A `CFG2` write with `MASTER` 0 while `CR1.SPE` was 0, its line.
    Stopped(String),
    /// That write and the slave-mode `[ERROR]` directly after it, and the
    /// queue slot that holds their place.
    Errored(String, String, u64),
}

/// One controller's summary state.
#[derive(Default)]
struct Controller {
    cycle: Cycle,
    /// A kept `[ERROR]` whose exemption the judge reads from this
    /// controller's next event: that event is kept, whatever it is.
    owed: bool,
}

/// One declared device's summary state: its lines are summarised by exact
/// text only once its identity and every multi-line candidate are found.
struct Device {
    check: aethelgard_twin::boot::StepCheck,
    identity: Vec<Line>,
    candidates: Vec<(Vec<Line>, bool)>,
    found_identity: bool,
    /// The device's own line bodies since its identity, as many as its
    /// longest step.
    recent: std::collections::VecDeque<String>,
    longest: usize,
}

impl Device {
    fn summarising(&self) -> bool {
        self.found_identity
            && self
                .candidates
                .iter()
                .all(|(want, found)| *found || want.len() == 1)
    }

    /// Whether `want` matches the last `want.len()` own lines.
    fn matches_recent(recent: &std::collections::VecDeque<String>, want: &[Line]) -> bool {
        want.len() <= recent.len()
            && want
                .iter()
                .zip(recent.iter().skip(recent.len() - want.len()))
                .all(|(w, b)| w.matches(b))
    }
}

/// One place in the summary's output: decided, or a held pair's.
enum Slot {
    Ready(Vec<Emit>),
    Pair(u64),
}

/// The most lines the summary queues behind an undecided pair: past it, the
/// oldest pair is kept as written (its next event owed), so the CLI's memory
/// stays bounded whatever the firmware does.
const SUMMARY_QUEUE: usize = 65_536;

/// The foreign log's summary (`AET-WO-4400j` D2, ruled 2026-11-25T18:49Z):
/// each repeating per-transaction line is kept the first time and counted
/// after, with its first and last time. It never reorders: the kept log is
/// the whole log less the lines it drops, so every line the judge reads
/// beside another (an `[ERROR]` after its `CFG2` write, a warning after a
/// finish line, across read blocks too, since the judge joins the run's log
/// around them) is beside the same line, or the dropped one is restored.
/// - (i) the D4 cycle, per SPI controller: a `CFG2` write with `MASTER` 0
///   and `SPE` 0, the slave-mode `[ERROR]` on the very next line, then the
///   controller's next event. Their place is held in the queue. A `CFG2` with
///   `MASTER` 1 makes it an exempt cycle, kept the first time its three lines
///   occur and counted after; any other event, a device warning straight
///   after the error, a marker or the end releases the pair as written, and
///   then the controller's next event is kept, so every kept `[ERROR]` is
///   judged by the event that follows it in the whole log;
/// - (ii) each declared device's lines and its multiplexer's, by exact text,
///   only once its identity and every multi-line candidate are found, so a
///   step's lines stay together and each candidate's first line after the
///   identity is kept;
/// - (iii) a candidate's single-line pattern (U5's FIFO read), its values
///   elided, after its first match;
/// - (iv) [`unread_model_warning`]s by exact text.
///
/// A device's `[WARNING]` or `[ERROR]` is never summarised, and the line
/// before it, if dropped, is restored first. A line a [`LogFilter`]'s own
/// summary dropped (a repeated `Unhandled write`) never reaches it; log lines
/// between a
/// read block's markers pass untouched, outside every rule, as the judge
/// reads none of them.
pub struct ForeignSummary {
    devices: Vec<Device>,
    /// The multiplexers of SPI devices (`spi2.spi2mux`).
    muxes: Vec<String>,
    controllers: std::collections::BTreeMap<String, Controller>,
    keys: std::collections::HashSet<String>,
    capped: bool,
    /// The previous line of the run's log, if it was dropped.
    dropped: Option<String>,
    /// The previous line of the run's log, whatever became of it.
    prev: String,
    /// The controller whose held `[ERROR]` was the previous line.
    held_error: Option<String>,
    queue: std::collections::VecDeque<Slot>,
    next_slot: u64,
    /// Between a read block's markers.
    inside: bool,
    /// A lone `CFG2` write was just kept as written: the next line of the
    /// run's log is kept too, so the write never sits beside a later error it
    /// was apart from.
    pin_next: bool,
}

impl ForeignSummary {
    /// The summary of a foreign boot of `plan`, its candidates as the judge's
    /// ([`data_candidates`]).
    pub fn of(
        plan: &BootPlan<'_>,
        parts: &std::collections::BTreeMap<String, String>,
        rest: &std::collections::BTreeMap<String, Vec<Vec<String>>>,
    ) -> Result<Self, Stop> {
        let table = aethelgard_ir::parts::table()
            .map_err(|e| Stop::tool_error("aethelgard-ir", e.to_string()))?;
        let mut devices = Vec::new();
        let mut muxes = Vec::new();
        let mut seen: Vec<&str> = Vec::new();
        for c in &plan.image.identity.checks {
            if seen.contains(&c.reference.as_str()) {
                continue;
            }
            seen.push(&c.reference);
            let checks: Vec<&aethelgard_twin::boot::StepCheck> = plan
                .image
                .identity
                .checks
                .iter()
                .filter(|x| x.reference == c.reference)
                .collect();
            let Some(identity) = checks
                .iter()
                .find(|x| x.step == aethelgard_ir::parts::StepKind::Identity)
            else {
                continue;
            };
            let bus = match identity.wire {
                aethelgard_twin::boot::Wire::I2c { .. } => "i2c",
                aethelgard_twin::boot::Wire::Spi { .. } => {
                    let mux = mux_of(identity).to_owned();
                    if !muxes.contains(&mux) {
                        muxes.push(mux);
                    }
                    "spi"
                }
            };
            // No witness: the judge finds nothing, so nothing is summarised.
            let Some(want) = identity.witness.as_ref() else {
                continue;
            };
            let identity_lines: Vec<Line> = want.iter().cloned().map(Line::Exact).collect();
            let (candidates, _) =
                data_candidates(&c.reference, identity, &checks, rest, parts, table, bus);
            let longest = candidates
                .iter()
                .map(|(_, w)| w.len())
                .chain([identity_lines.len()])
                .max()
                .unwrap_or(1);
            devices.push(Device {
                check: (*identity).clone(),
                identity: identity_lines,
                candidates: candidates.into_iter().map(|(_, w)| (w, false)).collect(),
                found_identity: false,
                recent: std::collections::VecDeque::new(),
                longest,
            });
        }
        Ok(Self {
            devices,
            muxes,
            controllers: std::collections::BTreeMap::new(),
            keys: std::collections::HashSet::new(),
            capped: false,
            dropped: None,
            prev: String::new(),
            held_error: None,
            queue: std::collections::VecDeque::new(),
            next_slot: 0,
            inside: false,
            pin_next: false,
        })
    }

    /// Emits in order: straight out with nothing queued, else behind the
    /// queue.
    fn emit(&mut self, e: Vec<Emit>, out: &mut Vec<Emit>) {
        if self.queue.is_empty() {
            out.extend(e);
        } else {
            self.queue.push_back(Slot::Ready(e));
            self.bound(out);
        }
    }

    /// Past [`SUMMARY_QUEUE`] places, the oldest held pair (always the
    /// queue's front: decided places there are emitted at once) is kept as
    /// written, its next event owed, until the queue is within the bound.
    fn bound(&mut self, out: &mut Vec<Emit>) {
        while self.queue.len() > SUMMARY_QUEUE {
            let Some(Slot::Pair(front)) = self.queue.front() else {
                return;
            };
            let front = *front;
            let Some(owner) = self.controllers.iter().find_map(|(n, c)| {
                matches!(c.cycle, Cycle::Errored(_, _, id) if id == front).then(|| n.clone())
            }) else {
                return;
            };
            self.release(&owner, out);
        }
    }

    /// Fills a held pair's place, then emits every decided slot at the
    /// queue's front.
    fn fill(&mut self, slot: u64, e: Vec<Emit>, out: &mut Vec<Emit>) {
        if let Some(s) = self
            .queue
            .iter_mut()
            .find(|s| matches!(s, Slot::Pair(id) if *id == slot))
        {
            *s = Slot::Ready(e);
        }
        while let Some(Slot::Ready(_)) = self.queue.front() {
            if let Some(Slot::Ready(e)) = self.queue.pop_front() {
                out.extend(e);
            }
        }
    }

    /// Releases a controller's held lines as written; a released `[ERROR]`
    /// owes its next event.
    fn release(&mut self, name: &str, out: &mut Vec<Emit>) {
        let Some(c) = self.controllers.get_mut(name) else {
            return;
        };
        match std::mem::take(&mut c.cycle) {
            Cycle::Idle => {}
            Cycle::Stopped(s0) => {
                self.pin_next = true;
                self.emit(vec![Emit::Keep(s0)], out);
            }
            Cycle::Errored(s0, e, slot) => {
                c.owed = true;
                self.fill(slot, vec![Emit::Keep(s0), Emit::Keep(e)], out);
            }
        }
    }

    /// Releases every controller's held lines.
    fn release_all(&mut self, out: &mut Vec<Emit>) {
        let names: Vec<String> = self.controllers.keys().cloned().collect();
        for n in names {
            self.release(&n, out);
        }
    }

    /// A key's occurrence: the line dropped and counted, or kept as written
    /// the first time (and past the cap).
    fn note(&mut self, key: String, line: &str, out: &mut Vec<Emit>) {
        let stamp = line.get(1..19).unwrap_or_default().to_owned();
        if self.keys.contains(&key) {
            self.emit(vec![Emit::Note { key, stamp }], out);
            self.dropped = Some(line.to_owned());
            return;
        }
        let mut e = Vec::new();
        if self.keys.len() >= SUMMARY_KEYS {
            self.capped = true;
        } else {
            self.keys.insert(key.clone());
            e.push(Emit::Note { key, stamp });
        }
        e.push(Emit::Keep(line.to_owned()));
        self.emit(e, out);
    }

    /// What a declared device's or multiplexer's line may be, once the
    /// devices have learnt from it: `None` if it is neither's, `Some(None)` to
    /// keep it as written, `Some(Some(key))` to summarise it by `key`.
    fn learn(&mut self, line: &str, source: &str) -> Option<Option<String>> {
        if self.muxes.iter().any(|m| m == source) {
            return Some(Some(log_text(line).to_owned()));
        }
        let prev = log_body(&self.prev).unwrap_or_default().to_owned();
        for d in &mut self.devices {
            let Some(own) = super::device_line(&d.check, line) else {
                continue;
            };
            // As the judge's own lines: the model's idle warning directly
            // after its multiplexer's finish line is not the device's.
            if let aethelgard_twin::boot::Wire::Spi { address, .. } = d.check.wire
                && d.check.idle_warning.as_deref() == Some(own)
                && prev
                    == format!(
                        "[NOISY] {}: Finisihing transmission on device 0x{address:X}",
                        mux_of(&d.check)
                    )
            {
                return Some(None);
            }
            let own = own.to_owned();
            if !d.found_identity {
                d.recent.push_back(own);
                while d.recent.len() > d.identity.len() {
                    d.recent.pop_front();
                }
                if Device::matches_recent(&d.recent, &d.identity) {
                    d.found_identity = true;
                    d.recent.clear();
                }
                return Some(None);
            }
            let already = d.summarising();
            d.recent.push_back(own.clone());
            while d.recent.len() > d.longest {
                d.recent.pop_front();
            }
            let mut newly = false;
            for (want, found) in &mut d.candidates {
                if !*found && Device::matches_recent(&d.recent, want) {
                    *found = true;
                    newly = true;
                }
            }
            if newly || !already {
                return Some(None);
            }
            // (iii) a found single-line pattern: its values elided.
            let pattern = d
                .candidates
                .iter()
                .find_map(|(want, found)| match want.as_slice() {
                    [Line::Pattern(p @ aethelgard_twin::model::WitnessLine::Bytes { .. })]
                        if *found && p.matches(&own) =>
                    {
                        Some(format!(
                            "{}: {}",
                            d.check.device,
                            Line::Pattern(p.clone()).describe()
                        ))
                    }
                    _ => None,
                });
            return Some(Some(pattern.unwrap_or_else(|| log_text(line).to_owned())));
        }
        None
    }
}

impl crate::run::Summarise for ForeignSummary {
    fn line(&mut self, line: &str, out: &mut Vec<Emit>) {
        // Between a read block's markers: none of the run's log, untouched.
        if self.inside {
            out.push(Emit::Keep(line.to_owned()));
            return;
        }
        let dropped = self.dropped.take();
        let held_error = self.held_error.take();
        let body = log_text(line);
        let source = log_source(body).unwrap_or_default().to_owned();
        let controller = source.split('.').next().unwrap_or_default().to_owned();
        let on_spi = controller.starts_with("spi");
        let device_level = source.contains('.')
            && (body.starts_with("[WARNING] ") || body.starts_with("[ERROR] "));
        let cfg2 = cfg2_write(body).map(|(s, v, c)| (s.to_owned(), v, c));
        let error = error_source(line).map(str::to_owned);
        // A device's warning or error: the line before it as in the whole log,
        // a held error released, a dropped line restored.
        if device_level {
            if let Some(name) = held_error {
                self.release(&name, out);
            }
            if let Some(d) = dropped {
                self.emit(vec![Emit::Keep(d)], out);
            }
        }
        // A lone CFG2 write this line is not the error of: released, and this
        // line kept, so the write stays apart from any later error.
        let mut force = std::mem::take(&mut self.pin_next);
        let stopped: Vec<String> = self
            .controllers
            .iter()
            .filter(|(n, c)| {
                matches!(c.cycle, Cycle::Stopped(_)) && error.as_deref() != Some(n.as_str())
            })
            .map(|(n, _)| n.clone())
            .collect();
        for n in stopped {
            self.release(&n, out);
            force = true;
        }
        // That release pinned this line, not the next.
        self.pin_next = false;
        // The devices learn from every line of theirs.
        let learnt = self.learn(line, &source);
        self.prev = line.to_owned();
        if let Some((spi, value, cr1)) = cfg2 {
            let master = value >> 22 & 1 == 1;
            if !master && cr1 & 1 == 0 {
                self.release(&spi, out);
                if force {
                    // Kept by force: never held, so never dropped with a cycle;
                    // a lone write kept as written pins the next line.
                    self.pin_next = true;
                    return self.emit(vec![Emit::Keep(line.to_owned())], out);
                }
                self.controllers.entry(spi).or_default().cycle = Cycle::Stopped(line.to_owned());
                return;
            }
            if !master {
                // A CFG2 write that is neither: as written, after what is held.
                self.release(&spi, out);
                return self.emit(vec![Emit::Keep(line.to_owned())], out);
            }
            let c = self.controllers.entry(spi.clone()).or_default();
            if let Cycle::Errored(..) = c.cycle
                && !c.owed
                && !force
            {
                let Cycle::Errored(s0, e, slot) = std::mem::take(&mut c.cycle) else {
                    unreachable!("matched just above");
                };
                let key = cycle_key(&s0, &e, line);
                let stamp = e.get(1..19).unwrap_or_default().to_owned();
                if self.keys.contains(&key) {
                    self.fill(slot, vec![Emit::Note { key, stamp }], out);
                    self.dropped = Some(line.to_owned());
                    return;
                }
                let mut pair = Vec::new();
                if self.keys.len() < SUMMARY_KEYS {
                    self.keys.insert(key.clone());
                    pair.push(Emit::Note { key, stamp });
                } else {
                    self.capped = true;
                }
                pair.extend([Emit::Keep(s0), Emit::Keep(e)]);
                self.fill(slot, pair, out);
                return self.emit(vec![Emit::Keep(line.to_owned())], out);
            }
            // An earlier kept error owes this event: what is held, then this
            // line, as written.
            let owed = c.owed || matches!(c.cycle, Cycle::Errored(..));
            self.release(&spi, out);
            if let Some(c) = self.controllers.get_mut(&spi) {
                c.owed = false;
            }
            if owed || force {
                return self.emit(vec![Emit::Keep(line.to_owned())], out);
            }
            return self.note(log_text(line).to_owned(), line, out);
        }
        if let Some(spi) = error {
            let c = self.controllers.entry(spi.clone()).or_default();
            if let Cycle::Stopped(_) = c.cycle {
                let Cycle::Stopped(s0) = std::mem::take(&mut c.cycle) else {
                    unreachable!("matched just above");
                };
                let slot = self.next_slot;
                self.next_slot += 1;
                self.queue.push_back(Slot::Pair(slot));
                if let Some(c) = self.controllers.get_mut(&spi) {
                    c.cycle = Cycle::Errored(s0, line.to_owned(), slot);
                }
                self.held_error = Some(spi);
                self.bound(out);
                return;
            }
            // Not directly after its write: never exempt; kept, and its next
            // event owed.
            self.release(&spi, out);
            if let Some(c) = self.controllers.get_mut(&spi) {
                c.owed = true;
            }
            return self.emit(vec![Emit::Keep(line.to_owned())], out);
        }
        if on_spi && source.contains('.') {
            // A device's or multiplexer's line on a controller: the event that
            // resolves what is held there, kept as written if it does.
            let c = self.controllers.entry(controller.clone()).or_default();
            if c.owed || !matches!(c.cycle, Cycle::Idle) {
                self.release(&controller, out);
                if let Some(c) = self.controllers.get_mut(&controller) {
                    c.owed = false;
                }
                return self.emit(vec![Emit::Keep(line.to_owned())], out);
            }
        }
        let key = match learnt {
            Some(Some(key)) if !device_level => Some(key),
            Some(_) => None,
            // (iv) a model warning no rule reads.
            None if unread_model_warning(body) => Some(body.to_owned()),
            None => None,
        };
        match key {
            Some(key) if !force => self.note(key, line, out),
            _ => self.emit(vec![Emit::Keep(line.to_owned())], out),
        }
    }

    fn other(&mut self, line: &str, out: &mut Vec<Emit>) {
        // Nothing held passes a line that is not the log's.
        self.release_all(out);
        let line = line.trim_end();
        if line == BEGIN {
            self.inside = true;
        } else if line == END {
            self.inside = false;
        } else if !self.inside {
            // A line of the run's log the judge reads beside the next.
            self.prev = line.to_owned();
            self.dropped = None;
            self.held_error = None;
        }
    }

    fn flush(&mut self, out: &mut Vec<Emit>) {
        self.release_all(out);
        self.dropped = None;
    }

    fn capped(&self) -> bool {
        self.capped
    }
}

/// The run's timeline from virtual time 0, as a Renode script: the port's
/// transmission into `capture`, then each rest stimulus and each byte at its
/// instant (a stimulus first at a shared instant), then the run to the first
/// end read ([`end_read_instants`]); the rest is [`timeline_tail`]'s. A file,
/// never the `-e` argument: one command per byte would pass the 128 KiB a
/// single argument may be.
pub fn timeline(d: &Declared, capture: &Path, virtual_ns: u64) -> String {
    let first_read = end_read_instants(virtual_ns)[0];
    let mut lines = vec![format!(
        "sysbus.{0} CreateFileBackend @{1}/{0}.bin true",
        d.renode,
        capture.display()
    )];
    let mut now = 0u64;
    let due = events(d).into_iter().filter(|(t, _)| *t <= first_read);
    run_to(&mut lines, &mut now, due, first_read);
    lines.join("\n") + "\n"
}

/// The input's events after the first end read's instant (`AET-WO-4400j`
/// D4): [`commands`] runs them between the end reads, since the reads are
/// known only once the boot is planned, after the timeline is sealed.
pub fn timeline_tail(d: &Declared, virtual_ns: u64) -> Vec<(u64, String)> {
    let first_read = end_read_instants(virtual_ns)[0];
    events(d)
        .into_iter()
        .filter(|(t, _)| *t > first_read)
        .collect()
}

/// The input's stimuli and bytes, each at its instant, in order; at a shared
/// instant, the stimuli first.
fn events(d: &Declared) -> Vec<(u64, String)> {
    let mut events: Vec<(u64, String)> = d.stimuli.iter().map(|s| (s.at_ns, s.command())).collect();
    events.extend(
        mavlink::schedule(&d.frames, d.spacing_ns)
            .into_iter()
            .map(|(t, b)| (t, format!("sysbus.{} WriteChar 0x{b:02X}", d.renode))),
    );
    // Stable: at a shared instant, the stimuli (pushed first) go first.
    events.sort_by_key(|e| e.0);
    events
}

/// What the verdict says of an input run.
#[derive(Serialize)]
pub struct InputRun {
    pub input: InputReport,
    pub rest: Vec<RestReport>,
    pub witness: WitnessReport,
    pub gap: &'static str,
    /// The archived run's id, with `--archive`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
}

#[derive(Serialize)]
pub struct InputReport {
    pub path: String,
    pub sha256: String,
    pub port: String,
    pub peripheral: String,
    pub spacing_ns: u64,
    pub frames: usize,
    pub fill_bytes: u64,
}

/// One stimulus of the rest, over the run: its values in order of first
/// use, how many times it was set, and when first and last.
#[derive(Serialize)]
pub struct RestReport {
    pub reference: String,
    pub device: String,
    pub property: &'static str,
    pub values: Vec<String>,
    pub set: u64,
    pub first_ns: u64,
    pub last_ns: u64,
    pub citation: String,
}

#[derive(Serialize)]
pub struct WitnessReport {
    pub captured_bytes: u64,
    pub heartbeats: u64,
    /// The first heartbeat with `MAV_MODE_FLAG_SAFETY_ARMED`, from the
    /// firmware's own transmission; `null` when none.
    pub armed: Option<ArmedReport>,
    pub acks: Vec<AckReport>,
    /// Each parameter the input set, with the firmware's last answer.
    pub params: Vec<ParamReport>,
    /// Each parameter the input only read (`PARAM_REQUEST_READ`), with the
    /// firmware's last `PARAM_VALUE` for it, `null` where it sent none
    /// (`AET-WO-4412` (3): `MOT_PWM_TYPE` from the board's defaults.parm).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reads: Vec<ReadReport>,
    pub texts: Vec<TextReport>,
}

#[derive(Serialize)]
pub struct ReadReport {
    pub id: String,
    pub value: Option<f32>,
}

#[derive(Serialize)]
pub struct ArmedReport {
    pub offset: usize,
    pub system: u8,
    pub component: u8,
    pub base_mode: u8,
}

#[derive(Serialize, PartialEq)]
pub struct AckReport {
    pub command: u16,
    pub result: u8,
    pub count: u64,
}

#[derive(Serialize)]
pub struct ParamReport {
    pub id: String,
    pub set: f32,
    pub echoed: Option<f32>,
    pub error: Option<u8>,
    /// With a reboot the firmware acknowledged (`AET-WO-4412` (4)): its last
    /// `PARAM_VALUE` before the first reboot and after the last, each `null`
    /// where it sent none; absent, both, from a run with no reboot.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_reboot: Option<Option<f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_reboot: Option<Option<f32>>,
}

/// A parameter the firmware read back after a reboot it acknowledged with a
/// value other than the one it read back before (`AET-WO-4412` (4)): on
/// silicon a stored parameter survives a system reset in flash.
pub const PARAM_LOST_ACROSS_REBOOT: &str = "foreign_param_lost_across_reboot";

#[derive(Serialize)]
pub struct TextReport {
    pub text: String,
    pub count: u64,
}

/// The rest stimuli as the verdict names them.
pub fn rest_report(stimuli: &[rest::Stimulus]) -> Vec<RestReport> {
    let mut out: Vec<RestReport> = Vec::new();
    for s in stimuli {
        match out
            .iter_mut()
            .find(|r| r.device == s.device && r.property == s.property)
        {
            Some(r) => {
                if !r.values.contains(&s.value) {
                    r.values.push(s.value.clone());
                }
                r.set += 1;
                r.last_ns = s.at_ns;
            }
            None => out.push(RestReport {
                reference: s.reference.clone(),
                device: s.device.clone(),
                property: s.property,
                values: vec![s.value.clone()],
                set: 1,
                first_ns: s.at_ns,
                last_ns: s.at_ns,
                citation: s.citation.clone(),
            }),
        }
    }
    out
}

/// The witness in `captured`, the firmware's transmission on the input's
/// port, and its findings: each parameter the input set that the firmware
/// answered with `PARAM_ERROR` (ruling D9).
pub fn witness_report(
    captured: &[u8],
    frames: &[mavlink::Frame],
) -> Result<(WitnessReport, Vec<Finding>), Stop> {
    let table = mavlink::messages().map_err(twin_refused)?;
    let w = mavlink::witness(captured, frames, &table);
    let mut acks: Vec<AckReport> = Vec::new();
    for &(command, result) in &w.acks {
        match acks
            .iter_mut()
            .find(|a| (a.command, a.result) == (command, result))
        {
            Some(a) => a.count += 1,
            None => acks.push(AckReport {
                command,
                result,
                count: 1,
            }),
        }
    }
    let mut reads: Vec<ReadReport> = Vec::new();
    for f in frames {
        if let mavlink::Decoded::ParamRead { id } = f.decode()
            && !reads.iter().any(|r| r.id == id)
        {
            reads.push(ReadReport {
                value: w
                    .values
                    .iter()
                    .rev()
                    .find(|(i, _)| *i == id)
                    .map(|(_, v)| *v),
                id,
            });
        }
    }
    let mut params: Vec<ParamReport> = Vec::new();
    for f in frames {
        if let mavlink::Decoded::ParamSet { id, value } = f.decode() {
            params.retain(|p| p.id != id);
            params.push(ParamReport {
                echoed: w
                    .values
                    .iter()
                    .rev()
                    .find(|(i, _)| *i == id)
                    .map(|(_, v)| *v),
                error: w
                    .errors
                    .iter()
                    .rev()
                    .find(|(i, _)| *i == id)
                    .map(|(_, e)| *e),
                before_reboot: w.reboots.first().map(|&first| {
                    w.values
                        .iter()
                        .zip(&w.value_offsets)
                        .rev()
                        .find(|((i, _), o)| *i == id && **o < first)
                        .map(|((_, v), _)| *v)
                }),
                after_reboot: w.reboots.last().map(|&last| {
                    w.values
                        .iter()
                        .zip(&w.value_offsets)
                        .rev()
                        .find(|((i, _), o)| *i == id && **o > last)
                        .map(|((_, v), _)| *v)
                }),
                id,
                set: value,
            });
        }
    }
    let lost = params.iter().filter_map(|p| match (p.before_reboot, p.after_reboot) {
        (Some(Some(before)), Some(Some(after))) if before.to_bits() != after.to_bits() => {
            Some(Finding::plain(
                PARAM_LOST_ACROSS_REBOOT,
                format!(
                    "the firmware read `{}` back as {before} before the reboot it acknowledged and as {after} after it",
                    p.id
                ),
            ))
        }
        _ => None,
    });
    let findings = lost
        .chain(params.iter().filter_map(|p| {
            p.error.map(|e| {
                Finding::plain(
                    PARAM_REJECTED,
                    format!(
                        "the input set `{}` to {}, and the firmware answered PARAM_ERROR error {e}{}",
                        p.id,
                        p.set,
                        if e == 1 {
                            " (MAV_PARAM_ERROR_DOES_NOT_EXIST)"
                        } else {
                            ""
                        }
                    ),
                )
            })
        }))
        .collect();
    let mut texts: Vec<TextReport> = Vec::new();
    for t in &w.texts {
        match texts.iter_mut().find(|r| r.text == *t) {
            Some(r) => r.count += 1,
            None => texts.push(TextReport {
                text: t.clone(),
                count: 1,
            }),
        }
    }
    Ok((
        WitnessReport {
            captured_bytes: captured.len() as u64,
            heartbeats: w.heartbeats,
            armed: w
                .armed
                .map(|(offset, system, component, base_mode)| ArmedReport {
                    offset,
                    system,
                    component,
                    base_mode,
                }),
            acks,
            params,
            reads,
            texts,
        },
        findings,
    ))
}

/// The input's report.
pub fn input_report(d: &Declared) -> InputReport {
    InputReport {
        path: d.path.clone(),
        sha256: d.sha256.clone(),
        port: d.label.clone(),
        peripheral: d.peripheral.clone(),
        spacing_ns: d.spacing_ns,
        frames: d.frames.len(),
        fill_bytes: d.frames.iter().map(|f| u64::from(f.fill)).sum(),
    }
}

fn digest(hex: &str) -> Option<[u8; 32]> {
    unhex(hex)?.try_into().ok()
}

/// The run archived under `archive` (trace 20, kinds 38 to 40), named by
/// its id, a function of the input, the specification and the firmware.
#[allow(clippy::too_many_arguments)]
pub fn archive(
    archive: &Path,
    d: &Declared,
    captured: &[u8],
    firmware_sha256: &str,
    specification_sha256: &str,
    platform_sha256: &str,
    tools: Vec<(String, String)>,
    virtual_ns: u64,
) -> Result<String, Stop> {
    use aethelgard_chaos::event::{Event, Header};
    use aethelgard_chaos::record::Record;
    let bad = |what: &str| Stop::tool_error("aethelgard", format!("{what} is not a sha256"));
    let (input, spec, firmware, platform) = (
        digest(&d.sha256).ok_or_else(|| bad("the input's hash"))?,
        digest(specification_sha256).ok_or_else(|| bad("the specification's hash"))?,
        digest(firmware_sha256).ok_or_else(|| bad("the firmware's hash"))?,
        digest(platform_sha256).ok_or_else(|| bad("the platform's hash"))?,
    );
    let run_id = aethelgard_chaos::campaign::run_id(&input, &spec, &firmware);
    let header = Header {
        run_id,
        artifacts: vec![
            ("firmware".to_owned(), firmware),
            ("input".to_owned(), input),
            ("platform".to_owned(), platform),
            ("specification".to_owned(), spec),
        ],
        tools,
        tool_sha256: Vec::new(),
    };
    use aethelgard_chaos::record::{FOREIGN_PART_BYTES, FOREIGN_PART_STIMULI};
    let mut records = Vec::new();
    let file: Vec<&[u8]> = d.file.chunks(FOREIGN_PART_BYTES).collect();
    for (k, part) in file.iter().enumerate() {
        records.push(Record::ForeignInput {
            label: d.label.clone(),
            peripheral: d.peripheral.clone(),
            spacing_ns: d.spacing_ns,
            part: k as u64,
            parts: file.len() as u64,
            file: part.to_vec(),
        });
    }
    let entries: Vec<(u64, String, String, String)> = d
        .stimuli
        .iter()
        .map(|s| {
            (
                s.at_ns,
                s.device.clone(),
                s.property.to_owned(),
                s.value.clone(),
            )
        })
        .collect();
    let stimuli: Vec<&[(u64, String, String, String)]> = match entries.is_empty() {
        true => vec![&[]],
        false => entries.chunks(FOREIGN_PART_STIMULI).collect(),
    };
    for (k, part) in stimuli.iter().enumerate() {
        records.push(Record::Stimuli {
            part: k as u64,
            parts: stimuli.len() as u64,
            entries: part.to_vec(),
        });
    }
    let capture: Vec<&[u8]> = match captured.is_empty() {
        true => vec![&[]],
        false => captured.chunks(FOREIGN_PART_BYTES).collect(),
    };
    for (k, part) in capture.iter().enumerate() {
        records.push(Record::SerialCaptured {
            label: d.label.clone(),
            end_ns: virtual_ns,
            part: k as u64,
            parts: capture.len() as u64,
            bytes: part.to_vec(),
        });
    }
    let id = crate::input::hex(run_id);
    let dir = archive.join(&id);
    (|| {
        let mut writer = aethelgard_chaos::TraceWriter::create(&dir, &header)?;
        for record in records {
            let timestamp = record.timestamp(virtual_ns);
            writer.append(&Event::Record { timestamp, record })?;
        }
        writer.commit()
    })()
    .map_err(|e| Stop {
        code: crate::EXIT_UNREADABLE,
        kind: "ArchiveWriteFailed",
        id: Some(id.clone()),
        message: e.to_string(),
    })?;
    Ok(id)
}

/// What `replay` prints of a foreign run (ruling D6, R1): re-judged from the
/// archive alone, with no file and no Renode. `input_verdict` is the input's
/// witness re-judged (`fail` with a `foreign_param_rejected`): the trace holds
/// the input, the stimuli and the transmission, not the run's other findings,
/// so it is not the run's verdict.
#[derive(Serialize)]
pub struct Replayed {
    pub ok: bool,
    pub check: &'static str,
    pub judge: &'static str,
    pub run_id: String,
    pub input_verdict: &'static str,
    /// What `input_verdict` is, in the output itself (ruled 2026-11-24).
    pub input_verdict_is: &'static str,
    pub input_sha256: String,
    pub port: String,
    pub peripheral: String,
    pub frames: usize,
    pub stimuli: usize,
    pub witness: WitnessReport,
    pub findings: Vec<Finding>,
    pub artifacts: std::collections::BTreeMap<String, String>,
    pub tools: std::collections::BTreeMap<String, String>,
    pub event_count: u64,
    pub state_sequence_hash: String,
}

/// `replay --archive` of a foreign input run (ruling D6, R1, granted
/// 2026-11-24T19:13Z): `None` when the directory is not one (its records
/// hold no kind 38, or the strict reader refuses it), which `replay` reads
/// exactly as before; else the run re-judged, or why not.
pub fn replay_archive(dir: &Path) -> Option<Result<Replayed, Stop>> {
    use aethelgard_chaos::record::Record;
    let replayed = aethelgard_chaos::replay_dir(dir).ok()?;
    if !replayed
        .state
        .records
        .iter()
        .any(|r| matches!(r, Record::ForeignInput { .. }))
    {
        return None;
    }
    // A run directory is named by its run id, as a campaign's is.
    let id = crate::input::hex(replayed.header.run_id);
    let named = std::fs::canonicalize(dir)
        .ok()
        .and_then(|d| d.file_name().map(std::ffi::OsStr::to_owned));
    if named.as_deref() != Some(std::ffi::OsStr::new(&id)) {
        return Some(Err(Stop {
            code: crate::EXIT_UNREADABLE,
            kind: "RunIdMismatch",
            id: Some(id.clone()),
            message: format!(
                "{} holds run {id}: a run directory is named by its run id",
                dir.display()
            ),
        }));
    }
    Some(replay(&replayed))
}

/// A foreign run's records joined: the input's (label, peripheral, spacing,
/// file), the number of stimuli, the capture and the run's end.
type ForeignRecords = ((String, String, u64, Vec<u8>), usize, Vec<u8>, u64);

/// The parts of a foreign run joined, or why they cannot be: every record is
/// one of kinds 38 to 40, in that order, each kind's parts numbered 0 to
/// `parts` − 1 in order with one count, and the fields every part repeats
/// equal across them.
fn foreign_records(replayed: &aethelgard_chaos::Replayed) -> Result<ForeignRecords, String> {
    use aethelgard_chaos::record::{MAX_FOREIGN_BYTES, MAX_STIMULI, Record};
    let mut input: Option<(String, String, u64, Vec<u8>, u64, u64)> = None;
    let mut stimuli: Option<(usize, u64, u64)> = None;
    let mut captured: Option<(String, u64, Vec<u8>, u64, u64)> = None;
    // The kinds in order, 38 then 39 then 40, none skipped: each record's
    // kind is the current one or the next.
    let mut phase = 38u64;
    // Each kind's next part and its count: a part out of order, or a count
    // that changes, is refused.
    fn next(seen: Option<(u64, u64)>, part: u64, parts: u64) -> Result<(), String> {
        let want = seen.map_or((0, parts), |(p, n)| (p + 1, n));
        if (part, parts) == want {
            Ok(())
        } else {
            Err(format!(
                "a part {part} of {parts} where part {} of {} was due",
                want.0, want.1
            ))
        }
    }
    for r in &replayed.state.records {
        let kind = r.kind();
        if kind != phase && kind != phase + 1 || kind < 38 {
            return Err(
                "it holds a record that is not a foreign run's, or one out of order".to_owned(),
            );
        }
        phase = kind;
        match r {
            Record::ForeignInput {
                label,
                peripheral,
                spacing_ns,
                part,
                parts,
                file,
            } => {
                next(input.as_ref().map(|i| (i.4, i.5)), *part, *parts)?;
                match &mut input {
                    None => {
                        input = Some((
                            label.clone(),
                            peripheral.clone(),
                            *spacing_ns,
                            file.clone(),
                            *part,
                            *parts,
                        ));
                    }
                    Some(i) => {
                        if (&i.0, &i.1, i.2) != (label, peripheral, *spacing_ns) {
                            return Err("the input's parts name two ports".to_owned());
                        }
                        i.3.extend_from_slice(file);
                        i.4 = *part;
                    }
                }
            }
            Record::Stimuli {
                part,
                parts,
                entries,
            } => {
                next(stimuli.map(|s| (s.1, s.2)), *part, *parts)?;
                let n = stimuli.map_or(0, |s| s.0) + entries.len();
                stimuli = Some((n, *part, *parts));
            }
            Record::SerialCaptured {
                label,
                end_ns,
                part,
                parts,
                bytes,
            } => {
                next(captured.as_ref().map(|c| (c.3, c.4)), *part, *parts)?;
                match &mut captured {
                    None => captured = Some((label.clone(), *end_ns, bytes.clone(), *part, *parts)),
                    Some(c) => {
                        if (&c.0, c.1) != (label, *end_ns) {
                            return Err("the capture's parts name two ports or two ends".to_owned());
                        }
                        c.2.extend_from_slice(bytes);
                        c.3 = *part;
                    }
                }
            }
            _ => {
                return Err(
                    "it holds a record that is not a foreign run's, or one out of order".to_owned(),
                );
            }
        }
    }
    let (label, peripheral, spacing, file, last, parts) = input.ok_or("it holds no input")?;
    let (n, slast, sparts) = stimuli.ok_or("it holds no stimuli")?;
    let (clabel, end_ns, bytes, clast, cparts) = captured.ok_or("it holds no capture")?;
    if last + 1 != parts || slast + 1 != sparts || clast + 1 != cparts {
        return Err("a kind's parts end before its count".to_owned());
    }
    if clabel != label {
        return Err("the capture names another port than the input".to_owned());
    }
    if file.len() > MAX_FOREIGN_BYTES || bytes.len() > MAX_FOREIGN_BYTES || n > MAX_STIMULI {
        return Err("its input, capture or stimuli are over their bounds".to_owned());
    }
    Ok(((label, peripheral, spacing, file), n, bytes, end_ns))
}

/// Re-judges a foreign run from its archive: the input's frames re-read and
/// re-refused from the file the trace holds, its hash against the header's,
/// and the witness from the transmitted bytes the trace holds.
pub fn replay(replayed: &aethelgard_chaos::Replayed) -> Result<Replayed, Stop> {
    let ((label, peripheral, spacing_ns, file), stimuli, captured, end_ns) =
        foreign_records(replayed).map_err(|why| {
            invalid(
                "NotAForeignRun",
                format!(
                    "the archive is not one foreign input run (kinds 38 to 40, in order): {why}"
                ),
            )
        })?;
    let (label, peripheral, file, captured) = (
        label.as_str(),
        peripheral.as_str(),
        file.as_slice(),
        captured.as_slice(),
    );
    use sha2::Digest;
    let sha: [u8; 32] = sha2::Sha256::digest(file).into();
    let named = replayed
        .header
        .artifacts
        .iter()
        .find(|(n, _)| n == "input")
        .map(|(_, h)| *h);
    if named != Some(sha) {
        return Err(invalid(
            "InputHashMismatch",
            "the input the trace holds is not the one its header names".to_owned(),
        ));
    }
    // The port as the run declared it: its label, USART and spacing, so the
    // file's frames are refused exactly as they were.
    let f: InputFile = serde_json::from_slice(file)
        .map_err(|e| invalid("InputMalformed", format!("the archived input: {e}")))?;
    frames_present(&f)?;
    if f.port != label {
        return Err(invalid(
            "InputHashMismatch",
            format!(
                "the archived input names port `{}`, its record `{label}`",
                f.port
            ),
        ));
    }
    let mut frames = Vec::with_capacity(f.frames.len());
    for (index, fr) in f.frames.iter().enumerate() {
        frames.push(mavlink::InputFrame {
            at_ns: fr.at_ns,
            bytes: unhex(&fr.mavlink).ok_or_else(|| {
                invalid(
                    "InputMalformed",
                    format!("archived frame {index}'s `mavlink` is not hex"),
                )
            })?,
            fill: fr.fill,
        });
    }
    let table = mavlink::messages().map_err(twin_refused)?;
    let parsed = mavlink::check_input(&frames, spacing_ns, end_ns, &table)
        .map_err(|e| invalid(e.kind(), e.to_string()))?;
    let (witness, findings) = witness_report(captured, &parsed)?;
    Ok(Replayed {
        ok: true,
        check: "replay",
        judge: JUDGE,
        input_verdict: if findings.is_empty() { "pass" } else { "fail" },
        input_verdict_is: INPUT_VERDICT_IS,
        run_id: crate::input::hex(replayed.header.run_id),
        input_sha256: crate::input::hex(sha),
        port: label.to_owned(),
        peripheral: peripheral.to_owned(),
        frames: frames.len(),
        stimuli,
        witness,
        findings,
        artifacts: replayed
            .header
            .artifacts
            .iter()
            .map(|(n, h)| (n.clone(), crate::input::hex(h)))
            .collect(),
        tools: replayed.header.tools.iter().cloned().collect(),
        event_count: replayed.manifest.event_count,
        state_sequence_hash: crate::input::hex(replayed.manifest.state_hash),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `AET-WO-4400i`: an input's timeline pauses and resumes the machine at
    /// every byte; the filter drops those two lines and nothing like them.
    #[test]
    fn the_timelines_pause_lines_are_dropped_and_nothing_else() {
        assert!(unread_access_line(
            "[00:00:01.000000000] [INFO] machine-0: Machine paused."
        ));
        assert!(unread_access_line(
            "[00:00:01.000000000] [INFO] machine-0: Machine resumed."
        ));
        assert!(!unread_access_line(
            "[00:00:01.000000000] [INFO] machine-0: Machine started."
        ));
        assert!(!unread_access_line(
            "[00:00:01.000000000] [WARNING] machine-0: Machine paused."
        ));
    }

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
                ["kept", "replaced", "dropped", "added"]
                    .iter()
                    .any(|d| disposition.starts_with(d)),
                "{rule}: {disposition}"
            );
            if disposition.starts_with("added") {
                assert!(
                    judges.contains("Weakens"),
                    "{rule}: an added rule says what it leaves"
                );
            }
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
        let regs = [0x3F00_3025, 0x801B, 0x48, 0x440, 0x40_8040, 0x0109_0231];
        let declared = |hz: u64| aethelgard_ir::motor::SolvedForeignClock {
            apb1_timers_hz: hz,
            apb2_timers_hz: hz,
            usarts: aethelgard_ir::serial::PERIPHERALS
                .iter()
                .map(|p| ((*p).to_owned(), 72_000_000))
                .collect(),
            vos: aethelgard_ir::vos::Vos::Scale1,
        };
        let mut r = ClockReport::of(&declared(200_000_000), vec!["dwt", "lpuart1"]);
        assert!(r.check(regs, 0, None, None).is_empty());
        assert!(
            r.checked
                .iter()
                .all(|c| c.holds && c.decoded_hz == Some(200_000_000))
        );
        assert_eq!(r.registers[1].name, "RCC_CFGR");
        assert_eq!(r.registers[1].value, "0x0000801b");
        assert_eq!(r.declared_only.len(), 8);
        assert!(r.conditions[0].starts_with("usart_clock_declared_unchecked:"));
        let mut r = ClockReport::of(&declared(250_000_000), vec![]);
        let f = r.check(regs, 0, None, None);
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
        let f = r.check(
            [regs[0], 0x8023, regs[2], regs[3], regs[4], regs[5]],
            0,
            None,
            None,
        );
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind, CLOCK_UNDECODABLE);
        assert!(r.checked.iter().all(|c| !c.holds && c.decoded_hz.is_none()));
    }

    /// One MAVLink 2 frame of message `id` from system 1, component 1, its
    /// CRC over the table's `CRC_EXTRA` (the encoding `parse_exact` reads).
    fn mav(id: u32, payload: &[u8]) -> Vec<u8> {
        let table = mavlink::messages().unwrap();
        let extra = table.iter().find(|m| m.id == id).unwrap().crc_extra;
        let mut f = vec![0xFD, payload.len() as u8, 0, 0, 0, 1, 1];
        f.extend_from_slice(&id.to_le_bytes()[..3]);
        f.extend_from_slice(payload);
        let crc = mavlink::x25(&[extra], mavlink::x25(&f[1..], 0xFFFF));
        f.extend_from_slice(&crc.to_le_bytes());
        f
    }

    fn param_id(name: &str) -> [u8; 16] {
        let mut id = [0u8; 16];
        id[..name.len()].copy_from_slice(name.as_bytes());
        id
    }

    /// `PARAM_VALUE`: value, count, index, id, type (MAVLink's size order).
    fn param_value(name: &str, v: f32) -> Vec<u8> {
        let mut p = v.to_le_bytes().to_vec();
        p.extend_from_slice(&900u16.to_le_bytes());
        p.extend_from_slice(&u16::MAX.to_le_bytes());
        p.extend_from_slice(&param_id(name));
        p.push(2);
        mav(22, &p)
    }

    /// `AET-WO-4412` (4): a parameter the firmware reads back before the
    /// reboot it acknowledged and after it is reported on both sides; the
    /// same value is no finding, another is `foreign_param_lost_across_reboot`
    /// by name; with no acknowledged reboot the report carries neither side.
    #[test]
    fn a_parameter_read_back_otherwise_after_a_reboot_is_named() {
        let table = mavlink::messages().unwrap();
        let mut set = 1f32.to_le_bytes().to_vec();
        set.extend_from_slice(&[1, 1]);
        set.extend_from_slice(&param_id("FRAME_CLASS"));
        set.push(9);
        let input = [mavlink::parse_exact(&mav(23, &set), &table).unwrap()];
        let ack = mav(77, &[246, 0, 0]);
        let run = |after: f32| {
            let mut tx = param_value("FRAME_CLASS", 1.0);
            tx.extend(ack.clone());
            tx.extend(param_value("FRAME_CLASS", after));
            witness_report(&tx, &input).unwrap()
        };
        let (w, f) = run(1.0);
        assert!(
            f.is_empty(),
            "{:?}",
            f.iter().map(|f| &f.message).collect::<Vec<_>>()
        );
        assert_eq!(
            (w.params[0].before_reboot, w.params[0].after_reboot),
            (Some(Some(1.0)), Some(Some(1.0)))
        );
        let (_, f) = run(0.0);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind, PARAM_LOST_ACROSS_REBOOT);
        assert!(
            f[0].message.contains("`FRAME_CLASS` back as 1 before"),
            "{}",
            f[0].message
        );
        let (w, f) = witness_report(&param_value("FRAME_CLASS", 0.0), &input).unwrap();
        assert!(f.is_empty());
        assert_eq!(
            (w.params[0].before_reboot, w.params[0].after_reboot),
            (None, None)
        );
    }

    /// `AET-WO-4412` (ii): a platform that loads the DShot timer or DMA
    /// model names the verdict's instruction count and PC not re-run stable;
    /// one with only sensor models, or none, does not.
    #[test]
    fn the_dshot_models_name_the_end_reads_not_rerun_stable() {
        let p = |f: &str| std::path::PathBuf::from(format!("/m/{f}"));
        assert!(loads_dshot_models(&[
            p("BMP581.cs"),
            p("AethelgardSTM32Timer.cs")
        ]));
        assert!(loads_dshot_models(&[p("AethelgardSTM32DMA.cs")]));
        assert!(!loads_dshot_models(&[p("BMP581.cs"), p("ICM42688P.cs")]));
        assert!(!loads_dshot_models(&[]));
        assert_eq!(NOT_RERUN_STABLE, ["boot.instructions", "boot.pc"]);
    }

    /// `AET-WO-4412` (3): a parameter the input only reads
    /// (`PARAM_REQUEST_READ`, index -1, by name) is reported with the
    /// firmware's last `PARAM_VALUE` for it, `null` where it sent none; it is
    /// not among the parameters set.
    #[test]
    fn a_parameter_the_input_only_reads_is_reported_with_its_value() {
        let table = mavlink::messages().unwrap();
        let mut read = (-1i16).to_le_bytes().to_vec();
        read.extend_from_slice(&[1, 1]);
        read.extend_from_slice(&param_id("MOT_PWM_TYPE"));
        let input = [mavlink::parse_exact(&mav(20, &read), &table).unwrap()];
        let mut tx = param_value("MOT_PWM_TYPE", 0.0);
        tx.extend(param_value("MOT_PWM_TYPE", 6.0));
        let (w, f) = witness_report(&tx, &input).unwrap();
        assert!(f.is_empty() && w.params.is_empty());
        assert_eq!(w.reads.len(), 1);
        assert_eq!(
            (w.reads[0].id.as_str(), w.reads[0].value),
            ("MOT_PWM_TYPE", Some(6.0))
        );
        let (w, _) = witness_report(&param_value("FRAME_CLASS", 1.0), &input).unwrap();
        assert_eq!(w.reads[0].value, None);
    }

    /// `AET-WO-4412` (4), ruled D5: the reset macro reloads nothing, writes
    /// the cold RCC and the vector table, and installs the `RCC_RSR` hooks:
    /// Table 56 row 4's flags, cleared by `RMVF`.
    #[test]
    fn the_reset_macro_is_the_boot_lines_without_loadelf_and_the_rsr_hooks() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/motors-dshot600.json");
        let spec: aethelgard_ir::Specification =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
        let topology = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
        let image = aethelgard_twin::boot::expected(&topology).unwrap();
        let m = reset_script(&image, 0x0800_0000);
        assert!(
            m.starts_with("macro reset\n\"\"\"\n") && m.ends_with("\n\"\"\"\n"),
            "{m}"
        );
        assert!(!m.contains("LoadELF"), "{m}");
        assert_eq!(
            m.matches("sysbus WriteDoubleWord").count(),
            image.cold.len()
        );
        assert!(m.contains("cpu VectorTableOffset 0x08000000"), "{m}");
        assert_eq!(RSR_SOFTWARE_RESET, 0x0142_0000);
        assert!(m.contains("aethelgard_rsr = 0x01420000"), "{m}");
        assert!(
            m.contains("SetHookAfterPeripheralRead sysbus.rcc \"if offset == 0xd0: value = "),
            "{m}"
        );
        assert!(
            m.contains(
                "SetHookBeforePeripheralWrite sysbus.rcc \"if offset == 0xd0 and value & 0x10000: "
            ),
            "{m}"
        );
    }

    /// `AET-WO-4412` D4: the DShot report from the DMA model's lines, on
    /// `motors-dshot600` (four outputs on TIM1) with TIM1's `ARR` read at 7
    /// (DShot600 on Sentinel-03, measured): a measured line of packet 0, a
    /// line with position 0's last bit flipped (its checksum fails, named
    /// once), where the judge read them, and nothing for a line of no paced
    /// request or no DMA.
    #[test]
    fn dshot_frames_are_judged_from_the_dma_models_lines() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/motors-dshot600.json");
        let spec: aethelgard_ir::Specification =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
        let topology = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
        let image = aethelgard_twin::boot::expected(&topology).unwrap();
        let observed: Vec<[u32; END_READS]> = image
            .registers
            .iter()
            .map(|r| {
                if r.name == "TIM1.ARR" {
                    [7; END_READS]
                } else {
                    [r.value; END_READS]
                }
            })
            .collect();
        let zero = format!("{}{}{}", "0 ".repeat(4), "3 ".repeat(64), "0 ".repeat(8));
        let mut bad: Vec<&str> = zero.split_whitespace().collect();
        bad[4 + 15 * 4] = "6";
        let line = |words: &str, end: u64| {
            format!(
                "[00:00:06.180697230] [NOISY] dma2: stream 1 req 15 NDTR 76 EN 6180670750 end {end} reqs 19: {}",
                words.trim_end()
            )
        };
        let good = line(&zero, 6_180_697_224);
        let flipped = line(&bad.join(" "), 6_180_997_224);
        let other =
            "[00:00:06.180697230] [NOISY] dma1: stream 0 req 41 NDTR 4 EN 1 end 2 reqs 4: 1 2 3 4"
                .to_owned();
        let empty = line(&"0 ".repeat(76), 6_180_397_224);
        let lines = [
            empty.as_str(),
            good.as_str(),
            flipped.as_str(),
            other.as_str(),
        ];
        let (report, findings) = dshot_report(&lines, &image, &observed);
        let r = report.unwrap();
        assert_eq!(r.read, DSHOT_READ_AT);
        assert_eq!(r.transfers.len(), 1);
        let s = &r.transfers[0];
        assert_eq!(
            (s.timer.as_str(), s.stream, s.request, s.count),
            ("TIM1", 1, 15, 3)
        );
        assert_eq!(s.positions.len(), 4);
        assert_eq!((s.positions[0].frames, s.positions[0].malformed), (3, 1));
        assert_eq!(s.positions[0].empty, 1);
        assert_eq!(s.positions[0].malformed_at.len(), 1);
        assert!(s.positions[0].malformed_at[0].starts_with("6180997224 ns: BadChecksum"));
        assert_eq!(s.positions[1].last_throttle, Some(0));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, DSHOT_FRAME_MALFORMED);
        assert!(
            findings[0].message.contains("6180997224"),
            "{}",
            findings[0].message
        );
        let (none, f) = dshot_report(&[other.as_str()], &image, &observed);
        assert!(none.is_none() && f.is_empty());
    }

    /// `AET-WO-4412` D2: on Sentinel-03's crystal tree (its RCC registers as
    /// ArduPilot leaves them, measured, `PLLSRC` `10`), the check passes the
    /// declared crystal's deviation to the clock rule, which judges every
    /// clock but `fPCLK4`; without one it judges none.
    #[test]
    fn an_hse_tree_is_judged_at_the_declared_crystals_deviation() {
        let regs = [0x3F03_3025, 0x801B, 0x48, 0x440, 0x0040_2022, 0x0109_0263];
        let declared = aethelgard_ir::motor::SolvedForeignClock {
            apb1_timers_hz: 200_000_000,
            apb2_timers_hz: 200_000_000,
            usarts: aethelgard_ir::serial::PERIPHERALS
                .iter()
                .map(|p| ((*p).to_owned(), 100_000_000))
                .collect(),
            vos: aethelgard_ir::vos::Vos::Scale1,
        };
        let crystal = aethelgard_ir::clock::Budget {
            fast_ppm: 102,
            slow_ppm: 102,
        };
        let mut r = ClockReport::of(&declared, vec![]);
        assert!(
            r.check(regs, 16_000_000, Some(crystal), Some(0x40))
                .is_empty()
        );
        let v = r.vos.as_ref().unwrap();
        assert_eq!(v.clocks.len(), 16);
        assert!(v.not_judged.is_empty());
        let mut r = ClockReport::of(&declared, vec![]);
        assert!(r.check(regs, 16_000_000, None, None).is_empty());
        assert!(r.vos.as_ref().unwrap().clocks.is_empty());
    }

    /// The four-port fixture (`AET-WO-4408`), solved, with `edit` applied to
    /// its specification first: SERIAL0–3 are MAVLINK (console, USART1),
    /// GPS (USART2), CRSF (rc_crsf, USART3) and GPS2 (UART4).
    fn four_port(edit: impl Fn(&mut serde_json::Value)) -> Vec<aethelgard_twin::serial::Options> {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/four-port.json");
        let mut v: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        edit(&mut v);
        let spec: aethelgard_ir::Specification = serde_json::from_value(v).unwrap();
        let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
        let topology = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
        aethelgard_twin::serial::options(&topology).unwrap()
    }

    /// `AET-WO-4400j` D4 (i): four end reads one quantum apart, the last at
    /// the window's end and none past it; a window under three quanta reads
    /// its earlier ones at 0. An input's events after the first read run
    /// between the reads, each at its instant, before a read at the same
    /// instant.
    #[test]
    fn the_end_is_read_four_times_ending_at_the_window_and_never_past_it() {
        let q = aethelgard_twin::boot::QUANTUM_NS;
        assert_eq!(
            end_read_instants(10_000_000),
            [9_700_000, 9_800_000, 9_900_000, 10_000_000]
        );
        assert_eq!(end_read_instants(150_000), [0, 0, 50_000, 150_000]);
        assert_eq!(end_read_instants(0), [0; END_READS]);
        let block = vec!["R".to_owned()];
        assert_eq!(
            end_commands(0, 10_000_000, &[], &block),
            [
                run_for(9_700_000),
                "R".into(),
                run_for(q),
                "R".into(),
                run_for(q),
                "R".into(),
                run_for(q),
                "R".into()
            ]
        );
        let tail = [
            (9_750_000, "A".to_owned()),
            (9_800_000, "B".to_owned()),
            (10_000_000, "C".to_owned()),
        ];
        assert_eq!(
            end_commands(9_700_000, 10_000_000, &tail, &block),
            [
                "R".into(),
                run_for(50_000),
                "A".into(),
                run_for(50_000),
                "B".into(),
                "R".into(),
                run_for(q),
                "R".into(),
                run_for(q),
                "C".into(),
                "R".into()
            ]
        );
    }

    /// `AET-WO-4400j` D4 (ii): on the four-port board, the CRSF port
    /// (USART3, `rc_crsf`) leaves exactly CR2's STOP and CR1's PCE, PS and M0
    /// to ArduPilot's protocol search at the end read: its two stop bits and
    /// 8E2 pass, named; the same on the GPS port (USART2) fails; another
    /// field of the RC port (CR1's UE) is still judged, as is any other
    /// register.
    #[test]
    fn only_an_rc_ports_search_fields_are_left_unjudged_at_the_end() {
        let serial = four_port(|_| {});
        let rc = rc_search_fields().unwrap();
        // RM0433 Rev 9 §53.8.1-2: PCE 10, PS 9, M0 12; STOP [13:12].
        assert_eq!(rc, (1 << 10 | 1 << 9 | 1 << 12, 0x3000));
        let frame = frame_bits().unwrap();
        let judged = |name: &str, mask: u32, expected: u32, got: u32| {
            let (m, unjudged) = end_mask(name, mask, &serial, rc);
            (got & m == expected & m, unjudged)
        };
        // Two stop bits (CR2 0x2000) against the solved one stop bit.
        assert_eq!(judged("USART3.CR2", 0x3000, 0, 0x2000), (true, 0x3000));
        assert_eq!(judged("USART2.CR2", 0x3000, 0, 0x2000), (false, 0));
        // 8E2's CR1: PCE, PS and M0 set, as ArduPilot writes 0x2000153D.
        let e81 = 0x0000_000D & frame;
        assert_eq!(
            judged("USART3.CR1", frame, e81, 0x2000_153D),
            (true, frame & rc.0)
        );
        assert!(!judged("USART2.CR1", frame, e81, 0x2000_153D).0);
        assert!(!judged("USART1.CR1", frame, e81, 0x2000_153D).0);
        // UE clear on the RC port is still a mismatch; so is a GPIO register.
        assert!(!judged("USART3.CR1", frame, e81, 0x0000_000C).0);
        assert_eq!(judged("GPIOA.MODER", 0xFFFF, 0x1234, 0x1235), (false, 0));
    }

    /// An access line in Renode 1.17.0's measured shape (`spike/2311/out`,
    /// run m4): `width` `UInt32`, `UInt16` or `Byte`.
    fn acc(t: &str, usart: &str, width: &str, offset: u32, value: u32) -> String {
        let name = ["ControlRegister1", "ControlRegister2"][(offset / 4).min(1) as usize];
        format!(
            "[00:00:0{t}] [INFO] {usart}: [cpu: 0x80FDE5C] Write{width} to {offset:#X} ({name}), value {value:#X}"
        )
    }

    /// Unmodified ArduPilot on the four-port board, as measured
    /// (`spike/2311/out/m1.txt`, `m4`): each port's bring-up writes CR2 with
    /// UE clear, then CR1 with UE; USART1 rewrites CR2 with UE set; the RC
    /// port's last search group leaves RXINV in force; and the `PUPDR` words
    /// read back at 5 s, GPIOA then GPIOB.
    fn arducopter() -> (Vec<String>, [u32; 2]) {
        let log = vec![
            acc("0.003200000", "usart1", "UInt32", 4, 0x40),
            acc("0.003200000", "usart1", "UInt32", 0, 0x2000_013D),
            acc("0.009100000", "usart1", "UInt32", 4, 0x40),
            acc("0.095300000", "usart2", "UInt32", 4, 0x40),
            acc("0.095300000", "usart2", "UInt32", 0, 0x2000_010D),
            acc("0.095472850", "uart4", "UInt32", 4, 0x40),
            acc("0.095472850", "uart4", "UInt32", 0, 0x2000_010D),
            acc("4.209467430", "usart3", "UInt32", 0, 0x10C),
            acc("4.209467430", "usart3", "UInt32", 4, 0),
            acc("4.209467430", "usart3", "UInt32", 4, 0x1_2040),
            acc("4.209467430", "usart3", "UInt32", 4, 0),
            acc("4.209467430", "usart3", "UInt32", 4, 0x1_0040),
            acc("4.209500000", "usart3", "UInt32", 0, 0x10D),
            acc("4.209500000", "usart3", "UInt32", 4, 0x40),
        ];
        (log, [0x0014_0055, 0x0050_0000])
    }

    fn judged(
        log: &[String],
        serial: &[aethelgard_twin::serial::Options],
        pulls: &[u32],
    ) -> (Vec<Finding>, Vec<SerialOptionsReport>, Vec<PullReport>) {
        let lines: Vec<&str> = log.iter().map(String::as_str).collect();
        judge_serial(&lines, true, serial, pulls)
    }

    fn failing(f: &[Finding]) -> Vec<(String, String)> {
        f.iter()
            .filter(|f| f.severity != "info")
            .map(|f| (f.kind.clone(), f.message.clone()))
            .collect()
    }

    /// `AET-WO-2311` (1): the expectation comes from the specification
    /// alone (`INV-07`): SWAP clear everywhere; on RC input TXINV clear and
    /// RXINV unjudged; elsewhere TXINV and RXINV as `framing.inverted`, and
    /// they follow it when it changes.
    #[test]
    fn the_options_expected_come_from_the_specification() {
        use aethelgard_twin::serial::Expect::{Clear, Set, Unjudged};
        let want = |o: &aethelgard_twin::serial::Options| -> Vec<_> {
            o.bits.iter().map(|b| (b.0, b.2)).collect()
        };
        let ports = four_port(|_| {});
        let names: Vec<_> = ports.iter().map(|o| (o.label.as_str(), o.renode)).collect();
        assert_eq!(
            names,
            [
                ("MAVLINK", "usart1"),
                ("GPS", "usart2"),
                ("CRSF", "usart3"),
                ("GPS2", "uart4")
            ]
        );
        for o in [&ports[0], &ports[1], &ports[3]] {
            assert_eq!(
                want(o),
                [("SWAP", Clear), ("TXINV", Clear), ("RXINV", Clear)]
            );
        }
        assert_eq!(
            want(&ports[2]),
            [("SWAP", Clear), ("TXINV", Clear), ("RXINV", Unjudged)]
        );
        let inverted = four_port(|v| v["serial_ports"][1]["framing"]["inverted"] = true.into());
        assert_eq!(
            want(&inverted[1]),
            [("SWAP", Clear), ("TXINV", Set), ("RXINV", Set)]
        );
        // An SBUS port, inverted as SBUS is: RC input, so still TXINV clear and
        // RXINV unjudged.
        let rc = four_port(|v| {
            v["serial_ports"][2]["role"] = "rc_sbus".into();
            v["serial_ports"][2]["peer_deviation_ppm"] = 5000.into();
            v["serial_ports"][2]["peer_deviation_source"] = "declared".into();
            v["serial_ports"][2]["framing"] = serde_json::json!(
                {"baud": 100000, "data_bits": 8, "parity": "even", "stop_bits": 2, "inverted": true}
            );
        });
        assert!(rc[2].inverted);
        assert_eq!(want(&rc[2]), want(&ports[2]));
    }

    /// Unmodified ArduPilot passes; each line's pull is info, named, and the
    /// verdict's report says what was judged.
    #[test]
    fn unmodified_arducopter_passes_and_its_pulls_are_info() {
        let ports = four_port(|_| {});
        let (log, pulls) = arducopter();
        let (f, options, pull_reports) = judged(&log, &ports, &pulls);
        assert_eq!(failing(&f), []);
        assert_eq!(f.len(), 8);
        assert!(f.iter().all(|f| f.severity == "info"));
        assert_eq!(
            f[2].message,
            "GPS TX (PA2, USART2): PUPDR pull-up (01) at the run's end, read back; not judged (roadmap §7.1 L1054)"
        );
        assert!(pull_reports.iter().all(|p| p.pupdr == "pull-up"));
        let o: Vec<_> = options
            .iter()
            .map(|o| (o.writes, o.judged.as_deref(), o.judged_ns))
            .collect();
        assert_eq!(
            o,
            [
                (2, Some("0x00000040"), Some(3_200_000)),
                (1, Some("0x00000040"), Some(95_300_000)),
                (5, Some("0x00010040"), Some(4_209_467_430)),
                (1, Some("0x00000040"), Some(95_472_850)),
            ]
        );
    }

    /// The WO's §2 planted words, each refused by name on a recorded log:
    /// SWAP set; TXINV set on an RC port; RXINV missing on an inverted port.
    #[test]
    fn a_planted_cr2_word_is_refused_by_name() {
        let ports = four_port(|_| {});
        let (mut log, pulls) = arducopter();
        // SWAP on GPS (USART2), as the ELF mutant writes it at 2.599 s.
        log.push(acc("2.599100000", "usart2", "UInt32", 0, 0x2000_010C));
        log.push(acc("2.599100000", "usart2", "UInt32", 4, 0x8000));
        let (f, ..) = judged(&log, &ports, &pulls);
        assert_eq!(
            failing(&f),
            [(
                SWAP_SET.to_owned(),
                "GPS (USART2, gps): CR2 0x00008000, the last word written with CR1.UE clear before the window's end (at 2599100000 ns), sets SWAP; serial_ports declares no swap".to_owned()
            )]
        );
        // TXINV on the RC port's last UE-clear word.
        let (mut log, _) = arducopter();
        log[11] = acc("4.209467430", "usart3", "UInt32", 4, 0x3_0040);
        log[13] = acc("4.209500000", "usart3", "UInt32", 4, 0x2_0040);
        let (f, ..) = judged(&log, &ports, &pulls);
        assert_eq!(
            failing(&f),
            [(
                TXINV_WRONG.to_owned(),
                "CRSF (USART3, rc_crsf): CR2 0x00030040, the last word written with CR1.UE clear before the window's end (at 4209467430 ns), sets TXINV; on RC input TXINV is clear (RXINV is AP_RCProtocol's, unjudged; AET-WO-4404 FEEDBACK §2.5)".to_owned()
            )]
        );
        // GPS declared inverted, ArduPilot's word not: RXINV and TXINV missing.
        let inverted = four_port(|v| v["serial_ports"][1]["framing"]["inverted"] = true.into());
        let (log, _) = arducopter();
        let (f, ..) = judged(&log, &inverted, &pulls);
        let k: Vec<_> = failing(&f)
            .into_iter()
            .map(|(k, m)| (k, m.contains("clears")))
            .collect();
        assert_eq!(
            k,
            [
                (TXINV_WRONG.to_owned(), true),
                (RXINV_WRONG.to_owned(), true)
            ]
        );
        assert!(
            failing(&f)[1]
                .1
                .ends_with("clears RXINV; framing.inverted is true")
        );
    }

    /// Ruled at §1.2: a CR2 write with UE set that changes a judged bit
    /// fails as its own finding; a narrow write fails closed; a port with no
    /// UE-clear word, or a log not read whole, fails by name.
    #[test]
    fn ignored_unjudgeable_and_unwritten_cr2_fail_by_name() {
        let ports = four_port(|_| {});
        let (mut log, pulls) = arducopter();
        log.push(acc("4.900000000", "usart1", "UInt32", 4, 0x8040));
        log.push(acc("4.900000000", "uart4", "UInt16", 4, 0x40));
        let (f, ..) = judged(&log, &ports, &pulls);
        let k: Vec<_> = failing(&f).into_iter().map(|(k, _)| k).collect();
        assert_eq!(k, [OPTIONS_IGNORED, OPTIONS_UNJUDGEABLE]);
        assert!(
            failing(&f)[0]
                .1
                .contains("CR2 0x00008040 written at 4900000000 ns with CR1.UE set changes SWAP")
        );
        assert!(
            failing(&f)[1]
                .1
                .starts_with("GPS2 (UART4, gps): WriteUInt16 to 0x4, value 0x40 at 4900000000 ns")
        );
        // GPS2 never written: unwritten.
        let (log, _) = arducopter();
        let log: Vec<String> = log.into_iter().filter(|l| !l.contains("uart4")).collect();
        let (f, options, _) = judged(&log, &ports, &pulls);
        assert_eq!(
            failing(&f).into_iter().map(|(k, _)| k).collect::<Vec<_>>(),
            [OPTIONS_UNWRITTEN]
        );
        assert_eq!(options[3].judged, None);
        // A log not read whole: every port unjudgeable.
        let (log, _) = arducopter();
        let lines: Vec<&str> = log.iter().map(String::as_str).collect();
        let (f, ..) = judge_serial(&lines, false, &ports, &pulls);
        assert_eq!(failing(&f).len(), 4);
        assert!(failing(&f).iter().all(|(k, _)| k == OPTIONS_UNJUDGEABLE));
    }

    /// `AET-WO-2314` D2: one hook per distinct handler, gated on
    /// `ICSR.VECTACTIVE` being a vector it serves, that logs the CPU's own
    /// mark with where the PC was stacked (PSP when `EXC_RETURN` bit 2 is
    /// set) and what it reads, and
    /// halts the core; only that mark, at a hooked handler for a vector it
    /// serves, is an entry.
    #[test]
    fn a_fault_entry_is_the_cpus_own_mark_at_a_hooked_handler_for_its_vector() {
        let handlers = [
            (3, 0x0800_0340),
            (4, 0x080c_02bc),
            (5, 0x080c_02b4),
            (6, 0x080c_02bc),
        ];
        let hooks = fault_hooks(&handlers);
        assert_eq!(hooks.len(), 3, "{hooks:?}");
        assert!(
            hooks[0].starts_with("cpu AddHook 0x8000340 \""),
            "{}",
            hooks[0]
        );
        assert!(hooks[0].contains("ok = v in (3, );"), "{}", hooks[0]);
        assert!(hooks[1].contains("ok = v in (4, 6, );"), "{}", hooks[1]);
        assert!(hooks[2].contains("ok = v in (5, );"), "{}", hooks[2]);
        for h in &hooks {
            assert!(
                h.contains("v = self.Bus.ReadDoubleWord(0xe000ed04) & 0x1ff;"),
                "{h}"
            );
            assert!(
                h.contains("f = r(self.OtherSP) if lr & 4 else r(self.SP);"),
                "{h}"
            );
            assert!(
                h.contains(&format!(
                    "'{FAULT_MARK}%d %x %x %x' % (v, pc, f + 24, self.Bus.ReadDoubleWord(f + 24))"
                )),
                "{h}"
            );
            assert!(
                h.ends_with("; ok and setattr(self, 'IsHalted', True)\""),
                "{h}"
            );
            assert_eq!(h.matches('"').count(), 2, "{h}");
        }
        let line = |body: &str| format!("[00:00:04.392440830] {body}");
        assert_eq!(
            fault_entry(
                &line("[WARNING] cpu: AETHELGARD-FAULT 3 8000340 30027328 80360aa"),
                &handlers
            ),
            Some((3, 0x3002_7328, 0x0803_60aa, Some(4_392_440_830)))
        );
        assert_eq!(
            fault_entry(
                &line("[WARNING] cpu: AETHELGARD-FAULT 6 80c02bc 30027328 8001000"),
                &handlers
            ),
            Some((6, 0x3002_7328, 0x0800_1000, Some(4_392_440_830)))
        );
        for other in [
            "[WARNING] cpu: AETHELGARD-FAULT 4 8000340 30027328 80360aa",
            "[WARNING] cpu: AETHELGARD-FAULT 3 8000342 30027328 80360aa",
            "[WARNING] usart1: AETHELGARD-FAULT 3 8000340 30027328 80360aa",
            "[INFO] cpu: AETHELGARD-FAULT 3 8000340 30027328 80360aa",
            "[WARNING] cpu: AETHELGARD-FAULT 3 8000340 30027328 80360aa 1",
            "[WARNING] cpu: AETHELGARD-FAULT 3 8000340 30027328",
        ] {
            assert_eq!(fault_entry(&line(other), &handlers), None, "{other}");
        }
        assert!(FOREIGN_LOG_FILTER.keep.contains(&FAULT_MARK));
    }

    /// `AET-WO-2314` D3: the foreign filter discards each USART's `TDR`
    /// writes, at any width, and what the default filter discards; it keeps
    /// the writes its rules read (`CR1`, `CR2`) and the word after `TDR`.
    /// The default filter, which the generated judge reads, keeps `TDR`.
    #[test]
    fn the_foreign_filter_discards_tdr_writes_the_generated_filter_keeps() {
        let at = |body: &str| format!("[00:00:04.000000000] [INFO] {body}");
        for tdr in [
            "usart1: [cpu: 0x8001434] WriteByte to 0x28 (TransmitData), value 0x41",
            "uart7: [cpu: 0x8001434] WriteUInt16 to 0x28 (TransmitData), value 0x41",
            "usart3: [cpu: 0x8001434] WriteUInt32 to 0x28 (TransmitData), value 0x41",
            "usart1: [cpu: 0x8001434] WriteByte to 0x29 (TransmitData+0x1), value 0x0",
        ] {
            assert!((FOREIGN_LOG_FILTER.discard)(&at(tdr)), "{tdr}");
            assert!(!(LOG_FILTER.discard)(&at(tdr)), "{tdr}");
        }
        for kept in [
            "usart1: [cpu: 0x80FDE5C] WriteUInt32 to 0x4 (ControlRegister2), value 0x40",
            "usart1: [cpu: 0x80FDE6E] WriteUInt32 to 0x0 (ControlRegister1), value 0x2000013D",
            "usart1: [cpu: 0x80FDE6E] WriteUInt32 to 0xC (BaudRate), value 0x271",
            "usart1: [cpu: 0x80FDE6E] WriteUInt32 to 0x2C (TransmitData+0x4), value 0x0",
            "usart1: [cpu: 0x80FDE6E] ReadUInt32 from 0x28 (TransmitData), returned 0x0",
            "spi1: [cpu: 0x80FDE6E] WriteUInt32 to 0x28 (unknown), value 0x0",
        ] {
            assert!(!(FOREIGN_LOG_FILTER.discard)(&at(kept)), "{kept}");
        }
        let isr =
            at("usart1: [cpu: 0x8001434] ReadUInt32 from 0x1C (InterruptAndStatus), returned 0xC0");
        assert!((FOREIGN_LOG_FILTER.discard)(&isr) && (LOG_FILTER.discard)(&isr));
    }

    /// `AET-WO-1609`: the hook is set at the symbol's address, and only the
    /// CPU's own line carrying that address is an entry, with its virtual
    /// time; a line of another source, another address or no mark is not.
    #[test]
    fn a_panic_entry_is_the_cpus_own_hook_line_at_the_symbols_address() {
        let hook = panic_hook(0x080c_033c);
        assert!(hook.starts_with("cpu AddHook 0x80c033c \""), "{hook}");
        assert!(hook.contains(&format!("'{PANIC_MARK}%x' % pc")), "{hook}");
        let line = |body: &str| format!("[00:00:04.392440850] {body}");
        assert_eq!(
            panic_entry(
                &line("[WARNING] cpu: AETHELGARD-PANIC 80c033c"),
                0x080c_033c
            ),
            Some(Some(4_392_440_850))
        );
        for other in [
            "[WARNING] cpu: AETHELGARD-PANIC 80c0340",
            "[WARNING] usart1: AETHELGARD-PANIC 80c033c",
            "[INFO] cpu: AETHELGARD-PANIC 80c033c",
            "[WARNING] cpu: AETHELGARD-PANICK 80c033c",
            "[WARNING] cpu: 80c033c",
        ] {
            assert_eq!(panic_entry(&line(other), 0x080c_033c), None, "{other}");
        }
        assert_eq!(
            panic_entry("[WARNING] cpu: AETHELGARD-PANIC 80c033c", 0x080c_033c),
            Some(None),
            "an unstamped entry still counts"
        );
        // Past the log budget, the first entry is still kept (review
        // finding 4); every other kind the default filter keeps stays kept.
        assert!(FOREIGN_LOG_FILTER.keep.contains(&PANIC_MARK));
        for k in LOG_FILTER.keep {
            assert!(FOREIGN_LOG_FILTER.keep.contains(k), "{k}");
        }
    }

    /// D2: each declared USART's accesses are logged once, and each serial
    /// pin's `PUPDR` is read once per port after the run.
    #[test]
    fn the_usarts_are_logged_and_the_pulls_read() {
        let ports = four_port(|_| {});
        assert_eq!(
            usart_logs(&ports),
            ["usart1", "usart2", "usart3", "uart4"]
                .map(|u| format!("sysbus LogPeripheralAccess sysbus.{u} true"))
        );
        assert_eq!(
            pull_registers(&ports),
            [
                ("GPIOA.PUPDR".to_owned(), 0x5802_000C),
                ("GPIOB.PUPDR".to_owned(), 0x5802_040C)
            ]
        );
    }

    // ------------------------------------------------ D2: the summary's oracle

    /// A step check on SPI for the summary's tests: `device` behind its
    /// multiplexer at `address`, its identity `want`, `idle` its model's edge
    /// artefact.
    fn spi_check(
        device: &str,
        address: u8,
        want: &[&str],
        idle: Option<&str>,
    ) -> aethelgard_twin::boot::StepCheck {
        aethelgard_twin::boot::StepCheck {
            reference: device.rsplit('.').next().unwrap().to_uppercase(),
            peripheral: "SPI2".into(),
            wire: aethelgard_twin::boot::Wire::Spi {
                address,
                select: "PB12".into(),
            },
            step: aethelgard_ir::parts::StepKind::Identity,
            label: "identity",
            write: vec![0x80],
            width: 1,
            expected: 0,
            mask: 0,
            status_at: 0,
            value_at: 0,
            level: (-1, "[NOISY] "),
            device: device.into(),
            witness: Some(want.iter().map(|w| (*w).to_owned()).collect()),
            idle_warning: idle.map(str::to_owned),
            skipped: Vec::new(),
            stimulus: None,
        }
    }

    /// The summary of the two devices below, with `candidates` each.
    fn summary_of(
        devices: &[(aethelgard_twin::boot::StepCheck, Vec<Vec<Line>>)],
    ) -> ForeignSummary {
        ForeignSummary {
            devices: devices
                .iter()
                .map(|(c, cands)| Device {
                    check: c.clone(),
                    identity: c
                        .witness
                        .clone()
                        .unwrap()
                        .into_iter()
                        .map(Line::Exact)
                        .collect(),
                    longest: cands
                        .iter()
                        .map(Vec::len)
                        .chain([c.witness.as_ref().unwrap().len()])
                        .max()
                        .unwrap(),
                    candidates: cands.iter().map(|w| (w.clone(), false)).collect(),
                    found_identity: false,
                    recent: std::collections::VecDeque::new(),
                })
                .collect(),
            muxes: devices
                .iter()
                .map(|(c, _)| mux_of(c).to_owned())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect(),
            controllers: std::collections::BTreeMap::new(),
            keys: std::collections::HashSet::new(),
            capped: false,
            dropped: None,
            prev: String::new(),
            held_error: None,
            queue: std::collections::VecDeque::new(),
            next_slot: 0,
            inside: false,
            pin_next: false,
        }
    }

    /// The log as the capture keeps it through the summary (no budget), and
    /// the exempt cycles it counted and dropped.
    fn summarised(s: &mut ForeignSummary, raw: &[String]) -> (Vec<String>, u64) {
        use crate::run::Summarise;
        let (mut kept, mut cycles, mut out) = (Vec::new(), 0u64, Vec::new());
        let mut unhandled = std::collections::HashSet::new();
        // A D4 cycle's key holds its write and its error (as the judge counts);
        // each note after a key's first is a cycle dropped.
        let mut noted = std::collections::HashSet::new();
        fn take(
            out: &mut Vec<Emit>,
            kept: &mut Vec<String>,
            cycles: &mut u64,
            noted: &mut std::collections::HashSet<String>,
        ) {
            for e in out.drain(..) {
                match e {
                    Emit::Keep(k) => kept.push(k),
                    Emit::Note { key, .. } => {
                        if key.contains(CFG2_MARK) && key.contains(SLAVE_MODE) && !noted.insert(key)
                        {
                            *cycles += 1;
                        }
                    }
                }
            }
        }
        for l in raw {
            if !l.starts_with('[') {
                s.other(l, &mut out);
            } else if let Some(key) = unhandled_write_key(l) {
                // The filter's own summary (D7 (a)), before this one: its first
                // line kept, the rest never reach the summary.
                if unhandled.insert(key) {
                    s.line(l, &mut out);
                }
            } else {
                s.line(l, &mut out);
            }
            take(&mut out, &mut kept, &mut cycles, &mut noted);
            if !l.starts_with('[') {
                kept.push(l.clone());
            }
        }
        // The end releases what is held, notes included.
        s.flush(&mut out);
        take(&mut out, &mut kept, &mut cycles, &mut noted);
        (kept, cycles)
    }

    /// What the judge reads of one device: its first warning, its identity's
    /// time, and each candidate's time after it.
    type DeviceView = (Option<String>, Option<u64>, Vec<Option<u64>>);
    /// What the judge reads of a log ([`view`]).
    type View = (Option<String>, u64, Vec<DeviceView>);
    /// A planted difference, by name.
    type Plant<'a> = (&'a str, Box<dyn Fn(&mut Vec<String>)>);

    /// What the judge reads of a log: the first unexempted error and the
    /// exempt count (with `cycles` the summary dropped); for each device, its
    /// first warning, its identity's time and each candidate's time after it.
    fn view(
        log: &[String],
        devices: &[(aethelgard_twin::boot::StepCheck, Vec<Vec<Line>>)],
        cycles: u64,
    ) -> View {
        // As the judge's `run_log`: every line outside the read blocks, and
        // none after the last.
        let last = log.iter().rposition(|l| l == END).unwrap_or(log.len());
        let mut inside = false;
        let lines: Vec<&str> = log[..last.min(log.len())]
            .iter()
            .map(String::as_str)
            .filter(|l| {
                let marker = *l == BEGIN || *l == END;
                if *l == BEGIN {
                    inside = true;
                }
                let keep = !inside && !marker;
                if *l == END {
                    inside = false;
                }
                keep
            })
            .collect();
        let (first, exempt) = error_lines(&lines);
        let mut per = Vec::new();
        for (c, cands) in devices {
            let own = own_lines(c, &lines);
            let stamp = |k: usize| log_stamped(lines[own[k].0]).map(|(ns, _)| ns);
            let warning = own
                .iter()
                .find(|(_, b)| b.starts_with("[WARNING]"))
                .map(|(_, b)| (*b).to_owned());
            let want: Vec<Line> = c
                .witness
                .clone()
                .unwrap()
                .into_iter()
                .map(Line::Exact)
                .collect();
            let at = find(&own, &want, 0);
            let after = at.map(|k| k + want.len());
            let found = cands
                .iter()
                .map(|w| after.and_then(|a| find(&own, w, a)).and_then(stamp))
                .collect();
            per.push((warning, at.and_then(stamp), found));
        }
        (first.map(|l| log_text(l).to_owned()), exempt + cycles, per)
    }

    /// Sentinel-03's shape, per `AET-WO-4400j` §1.2: U5 (the ICM-42688-P) on
    /// spi2 and U4 (the W25Q64JV) on spi1, each transaction the D4 cycle
    /// (`CFG2` MASTER 1, the select, the device's lines, the finish, the
    /// deselect, `CFG2` MASTER 0, the slave-mode error); U5's FIFO reads
    /// carrying changing samples; U4 programming bytes with its idle warning
    /// directly after each finish; the ADC's warning between; `n`
    /// transactions.
    fn shaped_log(n: usize) -> Vec<String> {
        let mut log = Vec::new();
        let mut t = 100_000_000u64;
        let mut at = |log: &mut Vec<String>, body: String| {
            t += 37_000;
            log.push(format!(
                "[00:00:{:02}.{:09}] {body}",
                t / 1_000_000_000,
                t % 1_000_000_000
            ));
        };
        let cycle = |log: &mut Vec<String>,
                     at: &mut dyn FnMut(&mut Vec<String>, String),
                     spi: &str,
                     mux: &str,
                     lines: &[String]| {
            at(log, format!("[WARNING] {spi}: {CFG2_MARK}20400000 0"));
            at(log, format!("[NOISY] {mux}: GPIO #0 set to False"));
            for l in lines {
                at(log, l.clone());
            }
            at(
                log,
                format!("[NOISY] {mux}: Finisihing transmission on device 0x0"),
            );
            // The flash model's edge artefact, directly after its finish line.
            if spi == "spi1" {
                at(log, "[WARNING] spi1.spi1mux.u4: Transmission finished in the unexpected state: RecognizeOperation".into());
            }
            at(log, format!("[NOISY] {mux}: GPIO #0 set to True"));
            at(log, format!("[WARNING] {spi}: {CFG2_MARK}0 0"));
            at(log, format!("[ERROR] {spi}: {SLAVE_MODE}"));
        };
        let u5 = |b: &str| format!("[NOISY] spi2.spi2mux.u5: {b}");
        let u4 = |b: &str| format!("[NOISY] spi1.spi1mux.u4: {b}");
        cycle(
            &mut log,
            &mut at,
            "spi1",
            "spi1.spi1mux",
            &[
                u4("Transmitting data 0x9F, current state: RecognizeOperation"),
                u4("Handled command: ReadID"),
            ],
        );
        cycle(
            &mut log,
            &mut at,
            "spi2",
            "spi2.spi2mux",
            &[u5("bank 0 read 0x75 -> 0x47")],
        );
        for k in 0..n {
            let fifo = format!(
                "bank 0 read 0x30 -> {}",
                (0..16)
                    .map(|i| format!("0x{:X}", (k * 7 + i * 13) % 251))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            cycle(
                &mut log,
                &mut at,
                "spi2",
                "spi2.spi2mux",
                &[u5("bank 0 read 0x2E -> 0x0 0x10"), u5(&fifo)],
            );
            if k % 5 == 0 {
                at(
                    &mut log,
                    "[WARNING] adc3: Issued a start event before the last sequence finished".into(),
                );
                cycle(
                    &mut log,
                    &mut at,
                    "spi1",
                    "spi1.spi1mux",
                    &[u4(&format!(
                        "Transmitting data 0x{:X}, current state: HandleCommand",
                        k % 256
                    ))],
                );
            }
        }
        log
    }

    fn shaped_devices() -> Vec<(aethelgard_twin::boot::StepCheck, Vec<Vec<Line>>)> {
        let fifo = Line::Pattern(aethelgard_twin::model::WitnessLine::Bytes {
            prefix: "[NOISY] spi2.spi2mux.u5: bank 0 read 0x30 -> ".into(),
            at_least: 16,
        });
        let exact = |s: &str| Line::Exact(s.to_owned());
        vec![
            (
                spi_check(
                    "spi2.spi2mux.u5",
                    0,
                    &["[NOISY] spi2.spi2mux.u5: bank 0 read 0x75 -> 0x47"],
                    None,
                ),
                vec![
                    vec![exact(
                        "[NOISY] spi2.spi2mux.u5: bank 0 read 0x1D -> 0x12 0x02",
                    )],
                    vec![fifo],
                ],
            ),
            (
                spi_check(
                    "spi1.spi1mux.u4",
                    0,
                    &[
                        "[NOISY] spi1.spi1mux.u4: Transmitting data 0x9F, current state: RecognizeOperation",
                        "[NOISY] spi1.spi1mux.u4: Handled command: ReadID",
                    ],
                    Some(
                        "[WARNING] spi1.spi1mux.u4: Transmission finished in the unexpected state: RecognizeOperation",
                    ),
                ),
                // Two lines each seen before (k = 10 and 5), never in this
                // order: a summary by text from the identity would drop both.
                vec![vec![
                    exact(
                        "[NOISY] spi1.spi1mux.u4: Transmitting data 0xA, current state: HandleCommand",
                    ),
                    exact(
                        "[NOISY] spi1.spi1mux.u4: Transmitting data 0x5, current state: HandleCommand",
                    ),
                ]],
            ),
        ]
    }

    /// The judge's reading of the raw log and of the summarised one.
    fn both(raw: &[String]) -> (Vec<String>, View, View) {
        let devices = shaped_devices();
        let (kept, cycles) = summarised(&mut summary_of(&devices), raw);
        // The whole log as the judge reads it without this summary: the
        // filter's own `Unhandled write` summary applied (D7 (a)).
        let mut seen = std::collections::HashSet::new();
        let filtered: Vec<String> = raw
            .iter()
            .filter(|l| unhandled_write_key(l).is_none_or(|k| seen.insert(k)))
            .cloned()
            .collect();
        let whole = view(&filtered, &devices, 0);
        let summary = view(&kept, &devices, cycles);
        (kept, whole, summary)
    }

    /// `AET-WO-4400j` D2 (ruled 2026-11-25T18:49Z), every source at once:
    /// the summary keeps a small share of a Sentinel-03-shaped log, and the
    /// judge reads the same first error, exempt count, warnings and step
    /// times from it as from the whole log.
    #[test]
    fn the_summarised_log_reads_as_the_whole_one() {
        let raw = shaped_log(1000);
        let (kept, whole, summary) = both(&raw);
        assert_eq!(summary, whole);
        assert_eq!(whole.0, None);
        assert!(whole.1 >= 1000, "{whole:?}");
        assert!(
            whole
                .2
                .iter()
                .all(|(w, id, c)| w.is_none() && id.is_some() && !c.is_empty()),
            "{whole:?}"
        );
        // Each flash data text once, and U4's idle warnings (never summarised)
        // with the finish line before each: about a sixteenth.
        assert!(
            kept.len() * 10 < raw.len(),
            "kept {} of {}",
            kept.len(),
            raw.len()
        );
        // Each summarised line is kept the first time: the cycle on each
        // controller, a FIFO read, the ADC's warning once.
        for once in [
            SLAVE_MODE,
            "bank 0 read 0x30 -> ",
            "adc3: Issued a start event",
        ] {
            assert!(kept.iter().any(|l| l.contains(once)), "{once}");
        }
        assert_eq!(
            kept.iter().filter(|l| l.contains("adc3: Issued")).count(),
            1
        );
        // U5's FIFO reads, 251 distinct samples here, elided after the first.
        let fifo = kept
            .iter()
            .filter(|l| l.contains("bank 0 read 0x30 -> "))
            .count();
        assert!(fifo < 5, "{fifo} FIFO reads kept");
    }

    /// WO §5, a summary that hides a finding: each source's later line
    /// differing in what the judge reads. For each, the judge's reading of the
    /// summarised log equals its reading of the whole one, and differs from
    /// the clean log's (the planted difference is a finding, or moves one).
    #[test]
    fn a_planted_difference_in_any_source_reads_as_in_the_whole_log() {
        let clean = view(&shaped_log(1000), &shaped_devices(), 0);
        let device = |l: &str| l.contains("] [NOISY] spi2.spi2mux.u5: bank 0 read 0x2E");
        let plants: Vec<Plant> = vec![
            // A later cycle left a slave while a device speaks: not exempt.
            (
                "a device line between an error and the next MASTER",
                Box::new(|log: &mut Vec<String>| {
                    let i = log
                        .iter()
                        .enumerate()
                        .filter(|(_, l)| l.contains(SLAVE_MODE))
                        .nth(700)
                        .unwrap()
                        .0;
                    let stamp = log[i][..21].to_owned();
                    log.insert(
                        i + 1,
                        format!("{stamp}[NOISY] spi2.spi2mux.u5: bank 0 read 0x2E -> 0x0 0x10"),
                    );
                }),
            ),
            // A later CFG2 write with SPE still set: never exempt.
            (
                "SPE set in a later CFG2 write",
                Box::new(|log: &mut Vec<String>| {
                    let i = log
                        .iter()
                        .enumerate()
                        .filter(|(_, l)| {
                            l.ends_with(&format!("{CFG2_MARK}0 0")) && l.contains("spi2")
                        })
                        .nth(800)
                        .unwrap()
                        .0;
                    log[i] = log[i].replace(&format!("{CFG2_MARK}0 0"), &format!("{CFG2_MARK}0 1"));
                }),
            ),
            // A later error not directly after its write.
            (
                "a line between a write and its error",
                Box::new(|log: &mut Vec<String>| {
                    let i = log
                        .iter()
                        .enumerate()
                        .filter(|(_, l)| l.contains(SLAVE_MODE))
                        .nth(600)
                        .unwrap()
                        .0;
                    let stamp = log[i][..21].to_owned();
                    log.insert(i, format!("{stamp}[WARNING] adc3: Issued a start event before the last sequence finished"));
                }),
            ),
            // A device warning not after its multiplexer's finish line.
            (
                "a device warning after a deselect",
                Box::new(|log: &mut Vec<String>| {
                    let i = log
                        .iter()
                        .enumerate()
                        .filter(|(_, l)| l.contains("spi1.spi1mux: GPIO #0 set to True"))
                        .nth(150)
                        .unwrap()
                        .0;
                    let stamp = log[i][..21].to_owned();
                    log.insert(i + 1, format!("{stamp}[WARNING] spi1.spi1mux.u4: Transmission finished in the unexpected state: RecognizeOperation"));
                }),
            ),
            // A data candidate first seen late, among many duplicates.
            (
                "a candidate first seen at the 900th read",
                Box::new(move |log: &mut Vec<String>| {
                    let i = log
                        .iter()
                        .enumerate()
                        .filter(|(_, l)| device(l))
                        .nth(900)
                        .unwrap()
                        .0;
                    log[i] = log[i].replace(
                        "bank 0 read 0x2E -> 0x0 0x10",
                        "bank 0 read 0x1D -> 0x12 0x02",
                    );
                }),
            ),
            // A multi-line candidate completed late.
            (
                "a multi-line candidate at the 150th flash transaction",
                Box::new(|log: &mut Vec<String>| {
                    let i = log
                        .iter()
                        .enumerate()
                        .filter(|(_, l)| l.contains("current state: HandleCommand"))
                        .nth(150)
                        .unwrap()
                        .0;
                    log[i] = format!(
                        "{}[NOISY] spi1.spi1mux.u4: Transmitting data 0xA, current state: HandleCommand",
                        &log[i][..21]
                    );
                    let stamp = log[i][..21].to_owned();
                    log.insert(i + 1, format!("{stamp}[NOISY] spi1.spi1mux.u4: Transmitting data 0x5, current state: HandleCommand"));
                }),
            ),
            // Lines of other sources between a held pair and its release.
            (
                "other sources between an error and its MASTER",
                Box::new(|log: &mut Vec<String>| {
                    let i = log
                        .iter()
                        .enumerate()
                        .filter(|(_, l)| l.contains(SLAVE_MODE) && l.contains("spi2"))
                        .nth(500)
                        .unwrap()
                        .0;
                    let stamp = log[i][..21].to_owned();
                    for _ in 0..3 {
                        log.insert(i + 1, format!("{stamp}[WARNING] adc3: Issued a start event before the last sequence finished"));
                    }
                }),
            ),
            // A pair released at a marker owes its next event, the MASTER;
            // dropped, the pair would read up to a later new device line.
            (
                "a marker after a pair, and a new device text later",
                Box::new(|log: &mut Vec<String>| {
                    let i = log
                        .iter()
                        .enumerate()
                        .filter(|(_, l)| l.contains(SLAVE_MODE) && l.contains("spi2"))
                        .nth(400)
                        .unwrap()
                        .0;
                    log.insert(i + 1, BEGIN.to_owned());
                    log.insert(i + 2, "0x00000000".to_owned());
                    log.insert(i + 3, END.to_owned());
                    let j = log
                        .iter()
                        .enumerate()
                        .filter(|(_, l)| l.contains("] [NOISY] spi2.spi2mux.u5: bank 0 read 0x2E"))
                        .nth(950)
                        .unwrap()
                        .0;
                    log[j] = log[j].replace(
                        "bank 0 read 0x2E -> 0x0 0x10",
                        "bank 0 read 0x2E -> 0x0 0x11",
                    );
                }),
            ),
            // Adversarial review F1: two controllers' cycles interleaved; spi2's
            // error comes straight after spi1's, so it is not exempt.
            (
                "two controllers' cycles interleaved",
                Box::new(|log: &mut Vec<String>| {
                    let i = log
                        .iter()
                        .enumerate()
                        .filter(|(_, l)| l.contains(SLAVE_MODE) && l.contains("spi2"))
                        .nth(300)
                        .unwrap()
                        .0;
                    let st = log[i][..21].to_owned();
                    // spi2: CFG2 0 0 at i-1, its error at i; put spi1's pair between.
                    log.insert(i, format!("{st}[ERROR] spi1: {SLAVE_MODE}"));
                    log.insert(i, format!("{st}[WARNING] spi1: {CFG2_MARK}0 0"));
                    log.insert(i + 4, format!("{st}[WARNING] spi1: {CFG2_MARK}20400000 0"));
                }),
            ),
            // F1: a device's warning straight after another controller's error.
            (
                "a device warning straight after another controller's error",
                Box::new(|log: &mut Vec<String>| {
                    // The first finish line, kept: only this pair stands between
                    // it and the warning.
                    let i = log
                        .iter()
                        .position(|l| l.contains("spi1.spi1mux: Finisihing"))
                        .unwrap();
                    let st = log[i][..21].to_owned();
                    // finish, [idle warning], deselect...: put spi2's pair between the
                    // finish line and the idle warning.
                    log.insert(i + 1, format!("{st}[ERROR] spi2: {SLAVE_MODE}"));
                    log.insert(i + 1, format!("{st}[WARNING] spi2: {CFG2_MARK}0 0"));
                    // And one spi2 cycle first, so that pair is a repeat, which a
                    // summary would drop unless the warning releases it.
                    let first = log[0][..21].to_owned();
                    for (k, l) in [
                        format!("[WARNING] spi2: {CFG2_MARK}0 0"),
                        format!("[ERROR] spi2: {SLAVE_MODE}"),
                        format!("[WARNING] spi2: {CFG2_MARK}20400000 0"),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        log.insert(k, format!("{first}{l}"));
                    }
                }),
            ),
            // F2: another source's error between a pair and its device line.
            (
                "another error between a pair and its device line",
                Box::new(|log: &mut Vec<String>| {
                    let i = log
                        .iter()
                        .enumerate()
                        .filter(|(_, l)| l.contains(SLAVE_MODE) && l.contains("spi2"))
                        .nth(200)
                        .unwrap()
                        .0;
                    let st = log[i][..21].to_owned();
                    log.insert(
                        i + 1,
                        format!("{st}[NOISY] spi2.spi2mux: GPIO #0 set to False"),
                    );
                    log.insert(i + 1, format!("{st}[ERROR] i2c1: x"));
                }),
            ),
            // F3: a dropped finish line, a read block, then the idle warning.
            (
                "a read block between a finish line and its idle warning",
                Box::new(|log: &mut Vec<String>| {
                    let i = log
                        .iter()
                        .enumerate()
                        .filter(|(_, l)| l.contains("spi1.spi1mux: Finisihing"))
                        .nth(120)
                        .unwrap()
                        .0;
                    log.insert(i + 1, END.to_owned());
                    log.insert(i + 1, "0x00000000".to_owned());
                    log.insert(i + 1, BEGIN.to_owned());
                }),
            ),
            // F4: the filter's own summary between a write and its error.
            (
                "an Unhandled write between a write and its error",
                Box::new(|log: &mut Vec<String>| {
                    let i = log
                        .iter()
                        .enumerate()
                        .filter(|(_, l)| l.contains(SLAVE_MODE) && l.contains("spi2"))
                        .nth(250)
                        .unwrap()
                        .0;
                    let st = log[i][..21].to_owned();
                    log.insert(
                        i,
                        format!("{st}[WARNING] rcc: Unhandled write to offset 0x0, value 0x1"),
                    );
                }),
            ),
            // A lone CFG2 write released by another controller's write kept
            // by force: that controller's cycle, dropped, would leave the lone
            // write beside a later error of its own.
            (
                "a lone write, another controller's cycle, then the lone write's error",
                Box::new(|log: &mut Vec<String>| {
                    let i = log
                        .iter()
                        .enumerate()
                        .filter(|(_, l)| l.contains("spi2.spi2mux: GPIO #0 set to True"))
                        .nth(500)
                        .unwrap()
                        .0;
                    let st = log[i][..21].to_owned();
                    for (k, l) in [
                        format!("[WARNING] spi1: {CFG2_MARK}0 0"),
                        format!("[WARNING] spi2: {CFG2_MARK}0 0"),
                        format!("[ERROR] spi2: {SLAVE_MODE}"),
                        format!("[WARNING] spi2: {CFG2_MARK}20400000 0"),
                        format!("[ERROR] spi1: {SLAVE_MODE}"),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        log.insert(i + 1 + k, format!("{st}{l}"));
                    }
                }),
            ),
            // A lone CFG2 write released by a MASTER that resolves another
            // controller's exempt cycle: that MASTER is kept by force.
            (
                "a lone write before another controller's MASTER, then its error",
                Box::new(|log: &mut Vec<String>| {
                    let i = log
                        .iter()
                        .enumerate()
                        .filter(|(_, l)| l.contains(SLAVE_MODE) && l.contains("spi2"))
                        .nth(450)
                        .unwrap()
                        .0;
                    let st = log[i][..21].to_owned();
                    log.insert(i + 1, format!("{st}[WARNING] spi1: {CFG2_MARK}0 0"));
                    // i + 2 is spi2's MASTER: spi1's error after it.
                    log.insert(i + 3, format!("{st}[ERROR] spi1: {SLAVE_MODE}"));
                }),
            ),
            // The reads at the end, then the flash hash: a pair still held there
            // must be in the run's log, before the first marker.
            (
                "a pair held to the end, then the reads",
                Box::new(|log: &mut Vec<String>| {
                    for _ in 0..4 {
                        log.push(BEGIN.to_owned());
                        log.push("0x00000000".to_owned());
                        log.push(END.to_owned());
                    }
                    log.push("AETHELGARD-FLASH 8000000 00".to_owned());
                }),
            ),
            // Adversarial review G2: a lone write kept as written (by force:
            // a third controller's lone write released), then a repeated line
            // the summary drops, then the lone write's error.
            (
                "a lone write kept by force, a dropped line, then its error",
                Box::new(|log: &mut Vec<String>| {
                    let i = log
                        .iter()
                        .enumerate()
                        .filter(|(_, l)| l.contains("spi2.spi2mux: GPIO #0 set to True"))
                        .nth(600)
                        .unwrap()
                        .0;
                    let st = log[i][..21].to_owned();
                    for (k, l) in [
                        format!("[WARNING] spi3: {CFG2_MARK}0 0"),
                        format!("[WARNING] spi1: {CFG2_MARK}0 0"),
                        "[NOISY] spi2.spi2mux: GPIO #0 set to True".to_owned(),
                        format!("[ERROR] spi1: {SLAVE_MODE}"),
                        format!("[WARNING] spi1: {CFG2_MARK}20400000 0"),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        log.insert(i + 1 + k, format!("{st}{l}"));
                    }
                }),
            ),
            // G2 across a read block: the lone write the last line before it,
            // a repeated line first after it, then the write's error.
            (
                "a lone write before a read block, a dropped line after it, then its error",
                Box::new(|log: &mut Vec<String>| {
                    let i = log
                        .iter()
                        .enumerate()
                        .filter(|(_, l)| l.contains("spi2.spi2mux: GPIO #0 set to True"))
                        .nth(650)
                        .unwrap()
                        .0;
                    let st = log[i][..21].to_owned();
                    for (k, l) in [
                        format!("{st}[WARNING] spi1: {CFG2_MARK}0 0"),
                        BEGIN.to_owned(),
                        "0x00000000".to_owned(),
                        END.to_owned(),
                        format!("{st}[NOISY] spi2.spi2mux: GPIO #0 set to True"),
                        format!("{st}[ERROR] spi1: {SLAVE_MODE}"),
                        format!("{st}[WARNING] spi1: {CFG2_MARK}20400000 0"),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        log.insert(i + 1 + k, l);
                    }
                }),
            ),
            // A marker (a read) between a write and its error, as at the end.
            (
                "a marker between a pair and its MASTER",
                Box::new(|log: &mut Vec<String>| {
                    let i = log
                        .iter()
                        .enumerate()
                        .filter(|(_, l)| l.contains(SLAVE_MODE) && l.contains("spi2"))
                        .nth(400)
                        .unwrap()
                        .0;
                    log.insert(i + 1, BEGIN.to_owned());
                    log.insert(i + 2, "0x00000000".to_owned());
                    log.insert(i + 3, END.to_owned());
                }),
            ),
        ];
        for (name, plant) in plants {
            let mut raw = shaped_log(1000);
            plant(&mut raw);
            // A run's log ends in its four reads and the flash hash, as every
            // real one does; nothing after the last read is the run's.
            if !raw
                .last()
                .is_some_and(|l| l.starts_with("AETHELGARD-FLASH"))
            {
                for _ in 0..4 {
                    raw.extend([BEGIN.to_owned(), "0x00000000".to_owned(), END.to_owned()]);
                }
                raw.push("AETHELGARD-FLASH 8000000 00".to_owned());
            }
            let (_, whole, summary) = both(&raw);
            assert_eq!(summary, whole, "{name}");
            if !name.starts_with("other sources")
                && !name.starts_with("a marker")
                && !name.starts_with("a pair held to the end")
                && !name.starts_with("a read block")
                && !name.starts_with("another error")
            {
                assert_ne!(whole, clean, "{name}: the plant moves the judge");
            }
        }
    }

    /// D2, ruled: a device's `[WARNING]` is never summarised, even repeated
    /// once its device's lines are: every one stays in the log.
    #[test]
    fn a_devices_warnings_are_never_summarised() {
        let mut raw = shaped_log(300);
        for k in [2000, 2400, 2800] {
            let stamp = raw[k][..21].to_owned();
            raw.insert(
                k,
                format!("{stamp}[WARNING] spi2.spi2mux.u5: Read from an unknown register"),
            );
        }
        let (kept, whole, summary) = both(&raw);
        assert_eq!(summary, whole);
        assert_eq!(
            kept.iter()
                .filter(|l| l.contains("u5: Read from an unknown register"))
                .count(),
            3
        );
    }

    /// The queue is bounded wherever it grows: a pair held while 70,000 lines
    /// of other sources pass is kept as written once the queue reaches
    /// [`SUMMARY_QUEUE`], and the judge reads the kept log as the whole one.
    #[test]
    fn the_queue_behind_a_held_pair_is_bounded() {
        use crate::run::Summarise;
        let devices = shaped_devices();
        let mut raw = shaped_log(5);
        let st = "[00:00:01.000000000] ";
        raw.push(format!("{st}[WARNING] spi2: {CFG2_MARK}0 0"));
        raw.push(format!("{st}[ERROR] spi2: {SLAVE_MODE}"));
        for k in 0..70_000 {
            raw.push(format!("{st}[INFO] usart1: line {k}"));
        }
        raw.push(format!("{st}[WARNING] spi2: {CFG2_MARK}20400000 0"));
        let mut s = summary_of(&devices);
        let mut out = Vec::new();
        let mut most = 0;
        for l in &raw {
            s.line(l, &mut out);
            out.clear();
            most = most.max(s.queue.len());
        }
        assert!(most <= SUMMARY_QUEUE, "{most}");
        let (_, whole, summary) = both(&raw);
        assert_eq!(summary, whole);
    }
}
