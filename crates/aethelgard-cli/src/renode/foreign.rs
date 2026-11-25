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
        "CR1's frame bits (UE, TE, PS, PCE, M0, M1) and CR2's STOP against the solved framing, read back; and CR2's SWAP, TXINV and RXINV, which Renode does not store, from each declared USART's access log (AET-WO-2311, ruled at §1.2): the last CR2 word written while CR1.UE was clear, the word silicon keeps (RM0433 Rev 9 §48.8.3, p. 2075), held to the specification alone: SWAP clear; on RC input TXINV clear; elsewhere TXINV and RXINV each as framing.inverted. A CR2 write with UE set that changes a judged bit fails (it does not take); a write into CR1 or CR2 that is not a word at its own offset, or no CR2 word written with UE clear, fails. Weakens: CR1.RE (receive enable) is not judged, as receive is held; an RC input port's RXINV is AP_RCProtocol's and unjudged; the words are the twin's sequence, not silicon's: a firmware that reads CR2 back before writing it reads every tag as 0 in the twin, so a clearing write silicon would make may be absent (ArduPilot's set_options, UARTDriver.cpp:1574, :1699-1709). At one of the four end reads, as the pins (AET-WO-4400j D4 (i)); on a port the specification declares as an RC input, CR2.STOP and CR1.PCE, PS and M0 are not judged at the end read, rc_protocol_search (D4 (ii)). Weakens: an RC port left at a wrong frame is not caught at the end",
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
        let vos = aethelgard_ir::vos::judge_foreign(&registers, &decoded, &usarts);
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
    let mut events = tail.iter().peekable();
    for at in end_read_instants(virtual_ns) {
        while let Some((t, cmd)) = events.next_if(|(t, _)| *t <= at) {
            if *t > now {
                out.push(run_for(t - now));
                now = *t;
            }
            out.push(cmd.clone());
        }
        if at > now {
            out.push(run_for(at - now));
            now = at;
        }
        out.extend(block.iter().cloned());
    }
    out
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
    for (i, (_, address)) in pull_registers(serial).iter().enumerate() {
        r.push((
            format!("sysbus ReadDoubleWord {address:#010x}"),
            ForeignRead::Pull(i),
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
        icsr_earlier: Vec::new(),
        rc_search: Vec::new(),
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
        serial_options: Vec::new(),
        serial_pulls: Vec::new(),
        option_changes: Vec::new(),
        panic: PanicReport {
            symbol: PANIC_SYMBOL,
            address: format!("{panic:#010x}"),
            entered_ns: None,
        },
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
    hse_hz: u32,
    serial: &[aethelgard_twin::serial::Options],
    faults: &[(u32, u64)],
    rest: &std::collections::BTreeMap<String, Vec<Vec<String>>>,
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
    for (t, cmd) in events(d).into_iter().filter(|(t, _)| *t <= first_read) {
        if t > now {
            lines.push(run_for(t - now));
            now = t;
        }
        lines.push(cmd);
    }
    if first_read > now {
        lines.push(run_for(first_read - now));
    }
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
    pub texts: Vec<TextReport>,
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
}

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
                id,
                set: value,
            });
        }
    }
    let findings = params
        .iter()
        .filter_map(|p| {
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
        })
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
        assert!(r.check(regs, 0).is_empty());
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
}
