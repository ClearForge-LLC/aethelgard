//! `simulate`: Renode loads the platform and lists what it built. Renode exits
//! 0 even when the load fails (`architecture.md` §2.4), so the exit code is
//! recorded but never trusted alone. Two witnesses in the output: a CPU in the
//! `peripherals` listing (an entry followed by a `Slot:` line) and the absence
//! of `There was an error executing command`. A CPU listed beside an `[ERROR]`
//! log line is a failed load too (`platform_load_error`).

use aethelgard_chaos::power;

use crate::run::{Bound, Bounds, LogFilter, Output, Stop};
use crate::said::{self, Style};
use crate::verdict::{Finding, Machine, Peripheral};

const TOOL: &str = "renode";
const LOAD_ERROR: &str = "There was an error executing command";
/// Renode's error log level; a CPU listed beside one is not a clean load.
const LOG_ERROR: &str = "[ERROR]";

/// The file name of the configuration every Renode this CLI starts reads,
/// written into the call's own scratch directory and passed as `--config`.
pub const CONFIG_FILE: &str = "renode.config";

/// That configuration (`AET-WO-2204` D4), for a call whose tool home is
/// `home`. It changes two keys from what Renode writes for itself when given
/// no file. `log-timestamps-type = Virtual` (`AET-WO-4300` D2, built in
/// `AET-WO-4300a`): each line carries the emulation's virtual time
/// (`Logger.cs` L592, `TimeInterval.ToString`), so a line a timer event
/// causes carries its exact time and a line the CPU causes the start of its
/// quantum ([`QUANTUM`]); the judge strips it as it stripped the host time,
/// and reads it only for a motor output's edges. `use-synchronous-logging = true`, so each log entry is written by
/// the thread that logs it, with no queue and no log thread (`Logger.cs`
/// L822–834); a line the firmware caused is on the console before `RunFor`
/// returns, never split below line level by the monitor's output (spike F2)
/// and never dropped by a level change (B10). Every other key Renode reads is
/// stated at the value it writes for itself (measured: the history under
/// `home`, repeated lines folded), because Renode writes a key it finds
/// missing back into the file, and the file is the CLI's, sealed read-only
/// for the tool uid (`AET-WO-1601`): given a file without them, Renode
/// aborted in the sandbox. Stating them all, it only reads it.
pub fn config(home: &std::path::Path) -> String {
    format!(
        "[general]\n\
         use-synchronous-logging = true\n\
         terminal = Termsharp\n\
         compiler-cache-enabled = False\n\
         serialization-mode = Generated\n\
         always-log-machine-name = False\n\
         collapse-repeated-log-entries = True\n\
         log-timestamps-type = Virtual\n\
         log-history-limit = 1000\n\
         history-path = {}/.config/renode/history\n\
         store-table-bits = 41\n\
         [monitor]\n\
         consume-exceptions-from-command = True\n\
         break-script-on-exception = True\n\
         number-format = Hexadecimal\n\
         [plugins]\n\
         enabled-plugins = \n\
         [translation]\n\
         min-tb-size = 33554432\n\
         max-tb-size = 536870912\n",
        home.display()
    )
}

/// The quantum every boot runs at, stated rather than left to Renode's
/// default (`AET-WO-4300` D2). Renode 1.17.0 reads no configuration key for
/// it (`ConfigurationManager` is asked for none); its default is
/// `TimeSourceBase.DefaultQuantum`, 100 µs (L814), and this command sets the
/// same value, [`aethelgard_twin::boot::QUANTUM_NS`], by the monitor.
pub const QUANTUM: &str = "emulation SetGlobalQuantum \"0.000100000\"";

/// The monitor commands. The platform is always the call's own copy, at a
/// path this CLI chose, so no part of the caller's input reaches the monitor.
pub fn commands(copy: &std::path::Path) -> String {
    format!(
        "mach create; machine LoadPlatformDescription @{}; peripherals; quit",
        copy.display()
    )
}

/// Renode's console lines, each with any stray carriage return at either end
/// removed (spike F14: the console can write `\r` before a line). A `\r`
/// inside a line is kept, so a value it splits does not parse.
fn clean_lines(text: &str) -> Vec<&str> {
    text.lines().map(|l| l.trim_matches('\r')).collect()
}

/// The `peripherals` listing: every entry, and the CPUs (an entry followed by
/// a `Slot:` line).
fn listing(lines: &[&str]) -> (Vec<String>, Vec<Peripheral>) {
    let mut peripherals = Vec::new();
    let mut cpus = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let branch = line.trim_start_matches([' ', '│']).trim_start();
        let Some(entry) = branch
            .strip_prefix("├── ")
            .or_else(|| branch.strip_prefix("└── "))
        else {
            continue;
        };
        let Some((name, kind)) = entry
            .trim_end()
            .strip_suffix(')')
            .and_then(|e| e.split_once(" ("))
        else {
            continue;
        };
        if lines.get(i + 1).is_some_and(|next| next.contains("Slot:")) {
            cpus.push(name.to_owned());
        }
        peripherals.push(Peripheral {
            name: name.to_owned(),
            kind: kind.to_owned(),
        });
    }
    (cpus, peripherals)
}

pub fn judge(code: i32, out: &Output) -> Result<(Vec<Finding>, Option<Machine>), Stop> {
    let text = out.text();
    if code != 0 {
        let said = said::suffix(&text, Style::Renode);
        return Err(Stop::tool_error(
            TOOL,
            format!("renode exited {code}{said}"),
        ));
    }
    let lines = clean_lines(&text);
    let (cpus, peripherals) = listing(&lines);
    let error = lines.iter().position(|l| l.contains(LOAD_ERROR));
    match (cpus.is_empty(), error) {
        (false, None) => match lines.iter().find(|l| l.contains(LOG_ERROR)) {
            None => Ok((Vec::new(), Some(Machine { cpus, peripherals }))),
            Some(line) => Ok((
                vec![Finding::plain(
                    "platform_load_error",
                    line.trim().to_owned(),
                )],
                None,
            )),
        },
        (true, Some(i)) => {
            let detail = lines.get(i + 1).map(|l| l.trim()).unwrap_or_default();
            Ok((
                vec![Finding::plain(
                    "platform_load_error",
                    format!("{} {detail}", lines[i].trim()).trim().to_owned(),
                )],
                None,
            ))
        }
        (false, Some(_)) => Err(Stop::tool_error(
            TOOL,
            format!(
                "renode listed a CPU and also reported a load error{}",
                said::suffix(&text, Style::Renode)
            ),
        )),
        (true, None) => Err(Stop::tool_error(
            TOOL,
            format!(
                "renode listed no CPU and reported no load error{}",
                said::suffix(&text, Style::Renode)
            ),
        )),
    }
}

/// The bounds on a boot's Renode tree (`RUL-13` as amended for `simulate
/// --firmware`, ruled on PR #56, B1). The reference boot at `--for 100ms`,
/// five runs each, took at most 2.16 s and 394,392 KiB of tree RSS on
/// Furnace, and 2.59 s and 404,620 KiB in the tools image. The ruling asks at
/// least 10x the slower wall time (26 s) and 2x the higher peak (791 MiB):
/// 30 s, since CI's boots under parallel tests took up to 6.84 s, and 1 GiB.
/// `RLIMIT_AS` cannot do this: CoreCLR will not start under any address-space
/// limit, 256 GiB included (measured), so the reaper polls instead.
pub const BOOT_BOUNDS: Bounds = Bounds {
    wall: std::time::Duration::from_secs(30),
    rss_kib: 1024 * 1024,
};

/// What Renode logs once its CPU has aborted; `RunFor` then never returns
/// (adversarial A6).
const ABORTED: &str = "This machine has aborted";

/// A boot's stdout is read through this (`AET-WO-2100` F23, `AET-WO-2301` P5):
/// a run whose output fits 8 MiB is read byte for byte; past that, Renode's
/// log lines are counted and dropped, but the first of each kind the judge
/// reads is kept, so a firmware flooding the log still reaches a verdict.
pub const LOG_FILTER: LogFilter = LogFilter {
    budget: 8 * 1024 * 1024,
    keep: &[ABORTED, LOAD_ERROR, LOG_ERROR, LOCKED[0], LOCKED[1]],
};

/// A log line without Renode's timestamp (`[00:00:00.019999000] `), which
/// is the run's own clock and so never belongs in a finding's text
/// (`INV-04`, `AET-WO-2301` F10, `AET-WO-4300a`).
fn log_text(line: &str) -> &str {
    let line = line.trim();
    log_body(line).unwrap_or(line).trim()
}

/// The finding for a boot the reaper killed at a bound: `cpu_aborted` if
/// Renode logged that its machine aborted, else the bound it exceeded.
pub fn bounded(out: &Output, bound: Bound) -> Finding {
    let text = out.text();
    if let Some(line) = text.lines().find(|l| l.contains(ABORTED)) {
        return Finding::plain(
            "cpu_aborted",
            format!(
                "the machine aborted and the run never ended; killed at the {bound:?} bound: {}",
                line.trim_matches(['\r', ' '])
            ),
        );
    }
    match bound {
        Bound::Wall => Finding::plain(
            "wall_bound_exceeded",
            format!(
                "the boot ran past the {} s wall-clock bound and was killed",
                BOOT_BOUNDS.wall.as_secs()
            ),
        ),
        Bound::Memory => Finding::plain(
            "memory_cap_exceeded",
            format!(
                "Renode's resident memory passed the {} MiB cap and it was killed",
                BOOT_BOUNDS.rss_kib / 1024
            ),
        ),
    }
}

/// The sentinels around a boot's reads. Each is matched as a whole line, so
/// the echo of the command line itself, which contains them inside quotes,
/// never matches. With synchronous logging, every log line before `BEGIN` is
/// the run's; one between the sentinels is the judge's own reads' (F3).
const BEGIN: &str = "AETHELGARD-BOOT-BEGIN";
const END: &str = "AETHELGARD-BOOT-END";
/// The warning Renode logs when a gated peripheral is touched (spike F10).
const LOCKED: [&str; 2] = [
    "Tried to write a locked peripheral",
    "Tried to read a locked peripheral",
];

/// One boot: the copies Renode reads, how long it runs, and what the judge
/// computed from the topology (`INV-07`).
pub struct BootPlan<'a> {
    pub platform: &'a std::path::Path,
    pub firmware: &'a std::path::Path,
    pub virtual_ns: u64,
    /// The board's window (`aethelgard_twin::boot::window`): shorter than
    /// `MAX_VIRTUAL_NS` for a board with a DShot output (`AET-WO-4300a` D1).
    pub window_ns: u64,
    pub boot_word: u32,
    /// Where the part boots from: `FLASH_BANK1_BASE`, the header's `FLASH_BASE`.
    pub vector_table: u32,
    pub image: &'a aethelgard_twin::boot::Image,
    /// The firmware's executable ranges, where the core must be at the end.
    pub executable: &'a [(u64, u64)],
    /// Every `yield; b.n` self-loop in the firmware's loaded bytes
    /// ([`aethelgard_twin::boot::self_loops`]): the core must end on one.
    pub self_loops: &'a [u64],
    /// Each motor output's observer, solved values and static check
    /// (`AET-WO-4300a`), in the specification's order.
    pub motors: &'a [Motor],
    /// Every solved peripheral as the listing must name it: (name, model).
    pub instances: &'a [(String, String)],
    /// The resets a declared droop renders, in time order; empty for every
    /// other boot (`AET-WO-2301`).
    pub resets: &'a [power::Reset],
}

/// The sentinels around the read after a release (`AET-WO-2301` §2.3), one
/// pair per release, numbered from 0; each is matched as a whole line.
const PROBE_BEGIN: &str = "AETHELGARD-PROBE-BEGIN-";
const PROBE_END: &str = "AETHELGARD-PROBE-END-";
/// The sentinel at each release, numbered as the probes are: the log lines
/// from it to the probe's `BEGIN` are the recovered boot's (`AET-WO-2204`).
const RELEASE: &str = "AETHELGARD-RELEASE-";

fn run_for(ns: u64) -> String {
    format!(
        "emulation RunFor \"{}.{:09}\"",
        ns / 1_000_000_000,
        ns % 1_000_000_000
    )
}

impl BootPlan<'_> {
    /// The instructions the core executes in the run: MIPS × time, or, after
    /// a rendered reset, MIPS × the time since the last release less the
    /// nudge (ruled D5; `AET-WO-2301` F4, F5): Renode's count restarts at a
    /// reset, and the release takes effect at the nudge's end. A reset never
    /// released executes nothing after it.
    pub fn instructions(&self) -> u64 {
        let ran_from = match self.resets.last() {
            None => 0,
            Some(power::Reset {
                released_ns: Some(at),
                ..
            }) => at + power::NUDGE_NS,
            Some(_) => return 0,
        };
        aethelgard_twin::boot::MIPS * self.virtual_ns.saturating_sub(ran_from) / 1000
    }

    /// Every read, in order, as the monitor command and what it reads.
    fn reads(&self) -> Vec<(String, Read)> {
        let mut reads = vec![
            (
                format!("sysbus ReadDoubleWord {:#010x}", self.boot_word),
                Read::BootWord,
            ),
            ("cpu ExecutedInstructions".to_owned(), Read::Instructions),
            ("cpu PerformanceInMips".to_owned(), Read::Mips),
            ("cpu PC".to_owned(), Read::Pc),
            (
                format!(
                    "sysbus ReadDoubleWord {:#010x}",
                    aethelgard_twin::boot::ICSR
                ),
                Read::Icsr,
            ),
        ];
        for (i, &at) in self.self_loops.iter().enumerate() {
            for (k, at) in [(0u8, at), (1, at + 2)] {
                reads.push((format!("sysbus ReadWord {at:#010x}"), Read::Loop(i, k)));
            }
        }
        for (i, r) in self.image.registers.iter().enumerate() {
            reads.push((
                format!("sysbus ReadDoubleWord {:#010x}", r.address),
                Read::Register(i),
            ));
        }
        for (i, c) in self.image.identity.checks.iter().enumerate() {
            for (word, at) in record_words(c).into_iter().enumerate() {
                reads.push((
                    format!("sysbus ReadDoubleWord {at:#010x}"),
                    Read::Record(i, word),
                ));
            }
        }
        reads
    }

    /// The monitor commands a declared droop's reset is rendered with
    /// ([`power::ASSERT_RENDERING`]): `machine Reset`, which alone never boots
    /// again (`AET-WO-2301` F3), then the cold writes and the vector table
    /// re-applied, the core held, and the boot word and records re-armed so
    /// that only the firmware's code can set them again.
    fn assert_reset(&self) -> Vec<String> {
        let mut c = vec!["machine Reset".to_owned()];
        c.extend(self.cold());
        c.push(self.vtor());
        c.push("cpu IsHalted true".to_owned());
        c.extend(self.rearm());
        c
    }

    /// The cold RCC writes, as the part's reset leaves those registers: each
    /// enable register at its reset value (`AET-WO-4300a` §1.6, 2100 F24).
    fn cold(&self) -> Vec<String> {
        self.image
            .cold
            .iter()
            .map(|r| format!("sysbus WriteDoubleWord {:#010x} {:#x}", r.address, r.value))
            .collect()
    }

    /// The core takes its vector table where the part boots, not where Renode
    /// guesses it from the ELF (adversarial A3; after a reset, `AET-WO-2301` F3).
    fn vtor(&self) -> String {
        format!("cpu VectorTableOffset {:#010x}", self.vector_table)
    }

    /// Only the firmware's code may set the boot word (adversarial A1, A2), or
    /// fill a record, so one preloaded in the image, or never written, reads as
    /// the judge left it (`AET-WO-2202` adversarial 1).
    fn rearm(&self) -> Vec<String> {
        let mut c = vec![format!(
            "sysbus WriteDoubleWord {:#010x} 0x0",
            self.boot_word
        )];
        for check in &self.image.identity.checks {
            for at in record_words(check) {
                c.push(format!(
                    "sysbus WriteDoubleWord {at:#010x} {:#010x}",
                    aethelgard_twin::boot::RECORD_UNSET
                ));
            }
        }
        c
    }

    /// The reads of a probe: the boot word, then the first identity record's
    /// status and value. The record is the firmware's own claim; what a
    /// transfer on the bus leaves is the device's witness lines since the
    /// release (`AET-WO-2301` adversarial F1, `AET-WO-2204`).
    fn probe_reads(&self) -> Vec<String> {
        let mut c = vec![format!("sysbus ReadDoubleWord {:#010x}", self.boot_word)];
        if let Some(check) = self.image.identity.checks.first() {
            for at in [check.status_at, check.value_at] {
                c.push(format!("sysbus ReadDoubleWord {at:#010x}"));
            }
        }
        c
    }

    /// Each modelled device's log at `Noisy`, set once before anything runs:
    /// a source's level, not the console's (`AET-WO-2204` D1, D4). Only a
    /// device the booted platform declares: an analog fault leaves its bus's
    /// devices off the platform, and a level set on no element is a failed
    /// command.
    ///
    /// Each device at its model's level (`AET-WO-4210` D2: the ICP-101xx at
    /// `Debug`), and each SPI controller's multiplexer at `Noisy`, whose
    /// chip-select lines are the per-read windows (D5): attribution only,
    /// never a witness.
    fn witness_levels(&self) -> Vec<String> {
        let mut c: Vec<String> = Vec::new();
        for check in &self.image.identity.checks {
            let present = self
                .instances
                .iter()
                .any(|(name, _)| check.device.rsplit('.').next() == Some(name.as_str()));
            let level = format!("logLevel {} sysbus.{}", check.level.0, check.device);
            if present && !c.contains(&level) {
                c.push(level);
            }
            if let aethelgard_twin::boot::Wire::Spi { .. } = check.wire {
                let mux = format!("logLevel -1 sysbus.{}", mux_of(check));
                if !c.contains(&mux) {
                    c.push(mux);
                }
            }
        }
        // Each motor output's observer (`AET-WO-4300a`).
        for m in self.motors {
            c.push(format!("logLevel -1 sysbus.{}", m.observer.source));
        }
        c
    }

    /// The stimulus of each data read whose device the platform declares,
    /// set by the judge before the run (`AET-WO-4210` D3): its own act,
    /// never the firmware's.
    fn stimuli(&self) -> Vec<String> {
        let mut c = Vec::new();
        for check in &self.image.identity.checks {
            let present = self
                .instances
                .iter()
                .any(|(name, _)| check.device.rsplit('.').next() == Some(name.as_str()));
            for (property, value) in check.stimulus.iter().flatten() {
                if present {
                    c.push(format!("sysbus.{} {property} {value}", check.device));
                }
            }
        }
        c
    }

    /// The droop's schedule as monitor commands, from virtual time 0 to the
    /// run's end: each reset asserted at its time, released at its time with
    /// the nudge, and probed at the recovery bound (ruled D1, D4).
    fn droop_schedule(&self) -> Vec<String> {
        let mut c = Vec::new();
        let mut now = 0u64;
        let mut at = |c: &mut Vec<String>, t: u64| {
            if t > now {
                c.push(run_for(t - now));
                now = t;
            }
        };
        for (k, r) in self.resets.iter().enumerate() {
            at(&mut c, r.asserted_ns);
            c.extend(self.assert_reset());
            let Some(release) = r.released_ns else { break };
            at(&mut c, release);
            c.push("cpu IsHalted false".to_owned());
            c.push(format!("echo \"{RELEASE}{k}\""));
            // The nudge (F5): the release takes effect at its end.
            at(&mut c, release + power::NUDGE_NS);
            at(&mut c, release + power::RECOVERY_BOUND_NS);
            // No level is changed anywhere (`AET-WO-2204` D4).
            c.push(format!("echo \"{PROBE_BEGIN}{k}\""));
            c.extend(self.probe_reads());
            c.push(format!("echo \"{PROBE_END}{k}\""));
        }
        at(&mut c, self.virtual_ns);
        c
    }

    /// The monitor commands: a fixed seed, the platform, the firmware, the
    /// cold RCC, each modelled device's log level, a fixed virtual time, then
    /// the reads between sentinels, then the listing. Both paths are this
    /// CLI's own copies. With a droop, the fixed virtual time is the droop's
    /// schedule instead. The console's level is never changed: with
    /// [`config`]'s synchronous logging, `BEGIN` is the boundary between the
    /// run's log and the judge's (`AET-WO-2204` D4).
    pub fn commands(&self) -> String {
        let mut c = vec![
            format!("emulation SetSeed {}", aethelgard_twin::boot::SEED),
            QUANTUM.to_owned(),
            "mach create".to_owned(),
            format!(
                "machine LoadPlatformDescription @{}",
                self.platform.display()
            ),
            format!("sysbus LoadELF @{}", self.firmware.display()),
        ];
        c.extend(self.cold());
        c.extend(self.rearm());
        c.push(self.vtor());
        c.extend(self.witness_levels());
        c.extend(self.stimuli());
        if self.resets.is_empty() {
            c.push(run_for(self.virtual_ns));
        } else {
            c.extend(self.droop_schedule());
        }
        c.push(format!("echo \"{BEGIN}\""));
        c.extend(self.reads().into_iter().map(|(cmd, _)| cmd));
        c.push(format!("echo \"{END}\""));
        c.push("peripherals".to_owned());
        c.push("quit".to_owned());
        c.join("; ")
    }
}

#[derive(Clone, Copy)]
enum Read {
    BootWord,
    Instructions,
    Mips,
    Pc,
    Icsr,
    /// Self-loop `.0`'s halfword `.1` (0 the `yield`, 1 the `b.n`), read back
    /// from the machine.
    Loop(usize, u8),
    Register(usize),
    /// Check `.0`'s record: word 0 is the status, then the value's words,
    /// most significant first.
    Record(usize, usize),
}

/// A boot's findings, the listing, and what the reads gave.
pub type Judged = (Vec<Finding>, Option<Machine>, Option<Observed>);

/// What the reads gave, for the verdict.
pub struct Observed {
    pub boot_word: u32,
    pub instructions: u64,
    pub mips: u64,
    pub pc: u32,
    pub icsr: u32,
    /// Each ELF self-loop's two halfwords as the machine held them after the
    /// run: a firmware can rewrite its flash in Renode (adversarial A1).
    pub loops: Vec<[u32; 2]>,
    pub registers: Vec<u32>,
    /// Each step check's status word and value.
    pub records: Vec<(u32, u64)>,
    /// Each step check's witness: the lines Renode logged for its device with
    /// a witness's shape that fall to this step ([`attribute`]), in order;
    /// `None` when the log could not be read whole (`AET-WO-2204` D2).
    pub witnesses: Vec<Option<Vec<String>>>,
    /// For each identity check, the addresses its bus's controller found no
    /// device at during the run (Renode's `Unknown slave at address N.`), in
    /// order: attribution's evidence of a transfer to a wrong address, never
    /// a witness (`AET-WO-2204`, the seeded set's `wrong-address`). Empty for
    /// an SPI check.
    pub absent: Vec<Vec<u8>>,
    /// For each motor output, what its observer showed (`AET-WO-4300a`).
    pub motors: Vec<MotorWitness>,
    /// For each step check on SPI, every line its controller's multiplexer
    /// logged when it routed a byte to no device, to several, or to an
    /// address with none (`SPIMultiplexer.cs` L56, L62, L69), whole and in
    /// order, inside the step's chip-select window; a line inside no window
    /// goes to every device's identity step (`AET-WO-4210` D5, `4200` F16):
    /// attribution's evidence, never a witness (`AET-WO-4200` D4). Empty for
    /// an I2C check.
    pub unrouted: Vec<Vec<String>>,
}

/// The words of a check's record in RAM, in the order the judge reads them:
/// the status, then the value's [`aethelgard_twin::boot::value_words`].
fn record_words(c: &aethelgard_twin::boot::StepCheck) -> Vec<u32> {
    let words = aethelgard_twin::boot::value_words(c.width);
    std::iter::once(c.status_at)
        .chain((0..words).map(|w| c.value_at + 4 * w))
        .collect()
}

/// The multiplexer an SPI check's device sits behind: `spi1.spi1mux`.
fn mux_of(c: &aethelgard_twin::boot::StepCheck) -> &str {
    c.device.rsplit_once('.').map_or("", |(m, _)| m)
}

/// A device's chip-select windows: each `[open, close)` from the
/// multiplexer's `GPIO #a set to False` (selected: the inputs are
/// active-low) to the next `GPIO #a set to True`, as positions in `log`
/// (`SPIMultiplexer.cs` L30; `AET-WO-4210` P3.1). A window still open at the
/// end runs to the end.
fn windows(log: &[&str], mux: &str, address: u8) -> Vec<(usize, usize)> {
    let edge = |level: &str| format!("[NOISY] {mux}: GPIO #{address} set to {level}");
    let (low, high) = (edge("False"), edge("True"));
    let mut out = Vec::new();
    let mut open = None;
    for (i, line) in log.iter().enumerate() {
        match log_body(line) {
            Some(b) if b == low && open.is_none() => open = Some(i),
            Some(b) if b == high => {
                if let Some(o) = open.take() {
                    out.push((o, i));
                }
            }
            _ => {}
        }
    }
    if let Some(o) = open {
        out.push((o, log.len()));
    }
    out
}

/// The device's own lines in `log`, with their positions: its witness-shaped
/// lines ([`device_line`]), but for its model's edge artefact directly after
/// the multiplexer's finish line for it, which is skipped (`AET-WO-4210` D4).
/// The judge's attribution and the droop's probe read a device through this
/// one filter.
fn own_lines<'l>(c: &aethelgard_twin::boot::StepCheck, log: &[&'l str]) -> Vec<(usize, &'l str)> {
    let finish = match c.wire {
        aethelgard_twin::boot::Wire::Spi { address, .. } => Some(format!(
            "[NOISY] {}: Finisihing transmission on device 0x{address:X}",
            mux_of(c)
        )),
        aethelgard_twin::boot::Wire::I2c { .. } => None,
    };
    let mut lines = Vec::new();
    for (i, line) in log.iter().enumerate() {
        let Some(body) = device_line(c, line) else {
            continue;
        };
        let after_finish = i > 0 && finish.is_some() && log_body(log[i - 1]) == finish.as_deref();
        if after_finish && c.idle_warning.as_deref() == Some(body) {
            continue;
        }
        lines.push((i, body));
    }
    lines
}

/// Each check's observed lines and, on SPI, the multiplexer lines of its
/// window (`AET-WO-4210` S2, D4, D5). A device's lines are its witness-shaped
/// lines ([`device_line`]), but for its model's edge artefact directly after
/// the multiplexer's finish line for it (D4), which is skipped. They go to
/// its steps:
/// - on SPI by chip-select window: the device's `k`-th window is its `k`-th
///   step's, a line in a later window or in none its last step's;
/// - on I2C in order, each step taking as many lines as it expects and the
///   last step the rest; an `[ERROR]` line is counted toward none, and goes
///   to the step under way, the last whose first line came before it (D-A1).
///
/// A line attributed to the wrong step only ever differs from what that step
/// expects: attribution never passes a witness.
fn attribute(
    checks: &[aethelgard_twin::boot::StepCheck],
    log: &[&str],
) -> (Vec<Vec<String>>, Vec<Vec<String>>) {
    let mut seen = vec![Vec::new(); checks.len()];
    let mut unrouted = vec![Vec::new(); checks.len()];
    // Each device's steps are consecutive, in its script's order.
    let mut groups: Vec<std::ops::Range<usize>> = Vec::new();
    for (i, c) in checks.iter().enumerate() {
        match groups.last_mut() {
            Some(g) if checks[g.start].reference == c.reference => g.end = i + 1,
            _ => groups.push(i..i + 1),
        }
    }
    let mut every_window: std::collections::BTreeMap<&str, Vec<(usize, usize)>> =
        Default::default();
    for g in &groups {
        let c = &checks[g.start];
        let last = g.end - 1;
        let spi = match c.wire {
            aethelgard_twin::boot::Wire::Spi { address, .. } => {
                let w = windows(log, mux_of(c), address);
                every_window.entry(mux_of(c)).or_default().extend(&w);
                Some((address, w))
            }
            aethelgard_twin::boot::Wire::I2c { .. } => None,
        };
        let lines = own_lines(c, log);
        match &spi {
            Some((_, w)) => {
                let step_of = |i: usize| {
                    w.iter()
                        .position(|&(o, e)| o < i && i < e)
                        .map_or(last, |k| (g.start + k).min(last))
                };
                for (i, body) in lines {
                    seen[step_of(i)].push(body.to_owned());
                }
                for (i, line) in log.iter().enumerate() {
                    if let Some(u) = unrouted_line(line, mux_of(c))
                        && w.iter().any(|&(o, e)| o < i && i < e)
                    {
                        unrouted[step_of(i)].push(u.to_owned());
                    }
                }
            }
            None => {
                let (errors, lines): (Vec<_>, Vec<_>) = lines
                    .into_iter()
                    .partition(|(_, b)| b.starts_with(LOG_ERROR));
                let mut at = 0;
                let mut begun = Vec::new();
                for k in g.clone() {
                    let n = if k == last {
                        lines.len().saturating_sub(at)
                    } else {
                        checks[k].witness.as_ref().map_or(0, Vec::len)
                    };
                    let end = (at + n).min(lines.len());
                    if let Some(&(i, _)) = lines.get(at).filter(|_| at < end) {
                        begun.push((i, k));
                    }
                    seen[k] = lines[at..end]
                        .iter()
                        .map(|(_, b)| (*b).to_owned())
                        .collect();
                    at = end;
                }
                for (i, body) in errors {
                    let k = begun
                        .iter()
                        .rev()
                        .find(|&&(b, _)| b < i)
                        .map_or(g.start, |&(_, k)| k);
                    seen[k].push(body.to_owned());
                }
            }
        }
    }
    // A multiplexer line inside no device's window is the controller's: it
    // goes to every device's identity step, as `4200` D4 collected it.
    for g in &groups {
        let c = &checks[g.start];
        if let aethelgard_twin::boot::Wire::Spi { .. } = c.wire {
            let all = every_window.get(mux_of(c)).map_or(&[][..], Vec::as_slice);
            for (i, line) in log.iter().enumerate() {
                if let Some(u) = unrouted_line(line, mux_of(c))
                    && !all.iter().any(|&(o, e)| o < i && i < e)
                {
                    unrouted[g.start].push(u.to_owned());
                }
            }
        }
    }
    (seen, unrouted)
}

/// One motor output as a boot judges it (`AET-WO-4300a` §1.4): the twin's
/// observer with the model's predicted timing, and the static check for
/// silicon on the topology's clock (`4300` D3), each from the topology alone
/// (`INV-07`).
#[derive(Debug, Clone)]
pub struct Motor {
    pub observer: aethelgard_twin::timer::Observer,
    pub psc: u32,
    pub arr: u32,
    pub ccr: u32,
    pub silicon: aethelgard_ir::motor::SiliconTiming,
}

/// Every motor output of `topology`, in the specification's order.
pub fn motors(
    topology: &aethelgard_ir::SolvedTopology,
) -> Result<Vec<Motor>, aethelgard_twin::TwinError> {
    let observers = aethelgard_twin::timer::observers(topology)?;
    Ok(observers
        .into_iter()
        .zip(topology.motor_outputs_in_order())
        .map(|(observer, (_, m, _))| Motor {
            silicon: aethelgard_ir::motor::check_silicon(&topology.clock, observer.timer, m),
            observer,
            psc: m.psc,
            arr: m.arr,
            ccr: m.ccr,
        })
        .collect())
}

/// The grain of a timer-caused stamp, in ns: one instruction at the core's
/// `MIPS` (measured: every stamp a multiple of 10 ns, FEEDBACK P7). Two
/// stamps of one exact sequence differ from it by less than this.
pub const STAMP_GRAIN_NS: u64 = 1000 / aethelgard_twin::boot::MIPS;

/// What an output's observer showed, judged against the model's
/// conventions (`aethelgard_twin::timer`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotorWitness {
    /// `witnessed`, `static_only`, `unwitnessed`, `unreadable`, `mismatch`,
    /// `batched` or `stopped`.
    pub status: &'static str,
    pub rises: u64,
    pub falls: u64,
    /// The first rise's stamp, the anchor every later edge is predicted from.
    pub first_ns: Option<u64>,
    /// The last rise's stamp, and the end of the window it must reach to
    /// within one period and one stamp grain (adversarial H1: a timer
    /// stopped after two periods).
    pub last_ns: Option<u64>,
    pub end_ns: u64,
    /// The widest spread of the edges' deviations from the predicted
    /// sequence, in ns: below [`STAMP_GRAIN_NS`] for an exact sequence.
    pub span_ns: Option<u64>,
    /// Edges whose stamp is off the quantum's grid. A store alone carries its
    /// quantum's start (FEEDBACK P2.1), but one right after a timer access
    /// carries an exact stamp (M4), so the grid is D4's third part, never the
    /// witness's whole: the exact sequence and the read-back are the rest.
    pub off_grid: u64,
    /// Consecutive edges with one stamp: a timer the core touched while it
    /// ran (FEEDBACK M1).
    pub batched: u64,
}

/// The observer's edges, `(stamp, rising)`, in the log's order; `None` when
/// a line of the observer is anything but a whole state change.
fn observer_edges(source: &str, lines: &[&str]) -> Option<Vec<(u64, bool)>> {
    let mut edges = Vec::new();
    for line in lines {
        let Some((ns, body)) = log_stamped(line) else {
            continue;
        };
        let Some(rest) = body
            .strip_prefix("[NOISY] ")
            .and_then(|r| r.strip_prefix(source))
            .and_then(|r| r.strip_prefix(": "))
        else {
            // Another source's line, or this source's at another level: a
            // LED logs its state changes at `Noisy` alone.
            if body.contains(&format!(" {source}: ")) {
                return None;
            }
            continue;
        };
        match rest {
            "LED state changed to True" => edges.push((ns, true)),
            "LED state changed to False" => edges.push((ns, false)),
            _ => return None,
        }
    }
    Some(edges)
}

/// Judges one output's edges against the model's sequence: rises at
/// `first + k·period` and falls `high` after each, alternating from a rise,
/// every deviation inside one stamp grain of every other (the model's
/// conventions, `aethelgard_twin::timer`). `None` is a log that could not be
/// read whole; `Some(None)` an observer line that was not a state change.
fn witness_motor(m: &Motor, edges: Option<Option<Vec<(u64, bool)>>>, end_ns: u64) -> MotorWitness {
    let mut w = MotorWitness {
        status: "unreadable",
        rises: 0,
        falls: 0,
        first_ns: None,
        last_ns: None,
        end_ns,
        span_ns: None,
        off_grid: 0,
        batched: 0,
    };
    let Some(edges) = edges else {
        return w;
    };
    let Some(edges) = edges else {
        w.status = "mismatch";
        return w;
    };
    w.rises = edges.iter().filter(|e| e.1).count() as u64;
    w.falls = edges.len() as u64 - w.rises;
    w.off_grid = edges
        .iter()
        .filter(|e| e.0 % aethelgard_twin::boot::QUANTUM_NS != 0)
        .count() as u64;
    w.batched = edges.windows(2).filter(|p| p[0].0 == p[1].0).count() as u64;
    if w.rises < 2 {
        w.status = "unwitnessed";
        return w;
    }
    let (period, high) = (
        i128::from(m.observer.period_ns),
        i128::from(m.observer.high_ns),
    );
    let first = edges[0].0;
    w.first_ns = Some(first);
    let (mut lo, mut hi) = (i128::MAX, i128::MIN);
    let mut alternating = edges[0].1;
    for (i, &(ns, rising)) in edges.iter().enumerate() {
        alternating &= rising == (i % 2 == 0);
        let k = (i / 2) as i128;
        let predicted = i128::from(first) + k * period + if rising { 0 } else { high };
        let d = i128::from(ns) - predicted;
        lo = lo.min(d);
        hi = hi.max(d);
    }
    let span = u64::try_from(hi - lo).unwrap_or(u64::MAX);
    w.span_ns = Some(span);
    let last = edges.iter().rev().find(|e| e.1).map(|e| e.0);
    w.last_ns = last;
    // The outputs run to the end of the window: a rise is due every period,
    // so the last one comes no earlier than a period and a grain before it.
    let covers = last.is_some_and(|l| l + m.observer.period_ns + STAMP_GRAIN_NS >= end_ns);
    w.status = if w.batched > 0 {
        "batched"
    } else if !alternating || span >= STAMP_GRAIN_NS {
        "mismatch"
    } else if !covers {
        "stopped"
    } else if w.off_grid == 0 {
        "static_only"
    } else {
        "witnessed"
    };
    w
}

fn motor_who(o: &aethelgard_twin::timer::Observer) -> String {
    format!("{} (TIM{}_CH{} on {})", o.label, o.timer, o.channel, o.pin)
}

/// The static check's findings for every motor output (`4300` D3): decided
/// from the topology alone, so a boot killed at a bound reports them too.
pub fn static_motor_findings(motors: &[Motor]) -> Vec<Finding> {
    motors
        .iter()
        .filter(|m| !m.silicon.holds)
        .map(|m| {
            let s = &m.silicon;
            Finding::plain(
                aethelgard_chaos::campaign::MOTOR_STATIC_FINDING,
                format!(
                    "{}: PSC {}, ARR {}, CCR {} on a {} Hz timer clock give a period of {} ps and a high time of {} ps (silicon counts ARR + 1); the declared timing is {} ps and {} ps",
                    motor_who(&m.observer),
                    m.psc,
                    m.arr,
                    m.ccr,
                    s.kernel_hz,
                    s.period_ps,
                    s.high_ps,
                    s.nominal_period_ps,
                    s.nominal_high_ps
                ),
            )
        })
        .collect()
}

/// One motor output's witness finding, once the boot reached its fixed
/// point (as an identity record is judged only then).
/// `window_ns` is the board's window (`aethelgard_twin::boot::window`), not
/// the boot's own length: a boot shorter than its board's window says
/// nothing about what a full one could show.
fn witness_findings_of(m: &Motor, w: &MotorWitness, window_ns: u64) -> Vec<Finding> {
    let o = &m.observer;
    let who = motor_who(o);
    let mut out = Vec::new();
    let model = format!(
        "the model's period {} ns and high time {} ns (ARR and CCR ticks at {} Hz)",
        o.period_ns,
        o.high_ns,
        aethelgard_twin::timer::TIMER_HZ
    );
    match w.status {
        "witnessed" => {}
        "static_only" => out.push(Finding::info(
            aethelgard_chaos::campaign::MOTOR_STATIC_ONLY_FINDING,
            format!(
                "{who}: {} rises at {model}, every edge on the {} ns quantum's grid, where a store can put one; judged by the static check alone",
                w.rises,
                aethelgard_twin::boot::QUANTUM_NS
            ),
        )),
        "unreadable" => out.push(Finding::plain(
            aethelgard_chaos::campaign::MOTOR_UNWITNESSED_FINDING,
            format!("{who}: Renode's log could not be read whole, so the output's edges were not"),
        )),
        "unwitnessed" => out.push(Finding::plain(
            aethelgard_chaos::campaign::MOTOR_UNWITNESSED_FINDING,
            format!(
                "{who}: the observer on {} logged {} rises and {} falls, too few to show a period{}",
                o.pin,
                w.rises,
                w.falls,
                // Two rises need two of the model's periods inside the window
                // (`AET-WO-4300a` D1, adversarial F5): name it when they cannot.
                if 2 * o.period_ns > window_ns {
                    format!(
                        "; the board's window of {window_ns} ns is shorter than two of its periods ({} ns), so no boot of this board can show one",
                        2 * o.period_ns
                    )
                } else {
                    String::new()
                }
            ),
        )),
        "stopped" => out.push(Finding::plain(
            aethelgard_chaos::campaign::MOTOR_MISMATCH_FINDING,
            format!(
                "{who}: {} rises follow {model}, but the last is at {} ns, more than a period before the window's end at {} ns: the output stopped",
                w.rises,
                w.last_ns.unwrap_or(0),
                w.end_ns
            ),
        )),
        "batched" => out.push(Finding::plain(
            aethelgard_chaos::campaign::MOTOR_MISMATCH_FINDING,
            format!(
                "{who}: {} pairs of edges share one stamp, so the timer's events were not delivered at their times",
                w.batched
            ),
        )),
        _ => out.push(Finding::plain(
            aethelgard_chaos::campaign::MOTOR_MISMATCH_FINDING,
            format!(
                "{who}: {} rises and {} falls do not follow {model} from the first rise: their deviations span {} ns, not under {STAMP_GRAIN_NS} ns, or they do not alternate from a rise",
                w.rises,
                w.falls,
                w.span_ns.map_or_else(|| "an unreadable".to_owned(), |s| s.to_string())
            ),
        )),
    }
    out
}

/// A line the multiplexer of `mux` (`spi1.spi1mux`) logs when a byte reaches
/// no single device: its whole text after the timestamp.
fn unrouted_line<'l>(line: &'l str, mux: &str) -> Option<&'l str> {
    log_body(line).filter(|b| {
        b.strip_prefix("[WARNING] ")
            .and_then(|r| r.strip_prefix(mux))
            .and_then(|r| r.strip_prefix(": Tried to transmit byte 0x"))
            .is_some()
    })
}

/// `i2c` or `spi`: the prefix of a check's finding kinds.
fn bus_word(c: &aethelgard_twin::boot::StepCheck) -> &'static str {
    match c.wire {
        aethelgard_twin::boot::Wire::I2c { .. } => "i2c",
        aethelgard_twin::boot::Wire::Spi { .. } => "spi",
    }
}

/// How a finding names a check's device: `U2 at I2C address 0x76`, or `U4 on
/// SPI1, chip select PA0`.
fn who(c: &aethelgard_twin::boot::StepCheck) -> String {
    format!("{} {}", c.reference, c.wire.describe(&c.peripheral))
}

/// An address the controller `bus` (`i2c1`) found no device at, from its
/// whole log line `[WARNING] i2c1: Unknown slave at address 119.`
/// (`STM32F7_I2C.cs` L349, the address in decimal).
fn absent_address(line: &str, bus: &str) -> Option<u8> {
    log_body(line)?
        .strip_prefix("[WARNING] ")?
        .strip_prefix(bus)?
        .strip_prefix(": Unknown slave at address ")?
        .strip_suffix('.')
        .filter(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))?
        .parse()
        .ok()
}

/// A bare hexadecimal value, the whole line: `0x2100B007`. Anything else on
/// the line, a log fragment included, is refused, never searched (R3).
fn value(line: &str) -> Option<u64> {
    let hex = line.strip_prefix("0x")?;
    if hex.is_empty() || hex.len() > 16 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u64::from_str_radix(hex, 16).ok()
}

/// The text of a whole Renode log line after its virtual timestamp
/// (`[00:00:00.019999000] `, `TimeInterval.ToString`), exactly: nothing
/// trimmed. `None` for any other line.
fn log_body(line: &str) -> Option<&str> {
    log_stamped(line).map(|(_, body)| body)
}

/// A whole Renode log line's virtual time in nanoseconds, and its text after
/// the timestamp. The stamp is `[hh:mm:ss.nnnnnnnnn] `, 21 bytes, and Renode
/// counts time in nanoseconds (`TimeInterval.TicksPerNanosecond = 1`).
pub(crate) fn log_stamped(line: &str) -> Option<(u64, &str)> {
    let b = line.as_bytes();
    let digits = |r: std::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
    let stamped = b.len() >= 21
        && b[0] == b'['
        && digits(1..3)
        && b[3] == b':'
        && digits(4..6)
        && b[6] == b':'
        && digits(7..9)
        && b[9] == b'.'
        && digits(10..19)
        && b[19] == b']'
        && b[20] == b' ';
    if !stamped {
        return None;
    }
    let n = |r: std::ops::Range<usize>| line[r].parse::<u64>().expect("digits");
    let ns = ((n(1..3) * 60 + n(4..6)) * 60 + n(7..9)) * 1_000_000_000 + n(10..19);
    Some((ns, &line[21..]))
}

/// Renode's log levels, as a line names them after its timestamp.
const LEVELS: [&str; 5] = ["[NOISY] ", "[DEBUG] ", "[INFO] ", "[WARNING] ", "[ERROR] "];

/// A whole Renode log line: a host timestamp, then a level.
fn is_log_line(line: &str) -> bool {
    log_body(line).is_some_and(|b| LEVELS.iter().any(|l| b.starts_with(l)))
}

/// A log line's text, if it has a witness's shape for `device` at `level`:
/// `[NOISY] i2c1.u2: …`, or a `[WARNING]` of the device (`AET-WO-4210` D4),
/// or an `[ERROR]` of it, which fails the step it falls in by name (D-A1).
/// The shape only ever fails a witness: a pass needs the whole text equal to
/// an expected line (`AET-WO-2204` D1).
fn witness_shaped<'l>(line: &'l str, device: &str, level: &str) -> Option<&'l str> {
    log_body(line).filter(|b| {
        [level, "[WARNING] ", "[ERROR] "].into_iter().any(|l| {
            b.strip_prefix(l)
                .and_then(|r| r.strip_prefix(device))
                .is_some_and(|r| r.starts_with(": "))
        })
    })
}

/// A log line's text if it is `c`'s device's and not one of the lines its
/// model writes that are not a transfer, which the judge skips, exactly as
/// the twin lists them (`AET-WO-4200` D2).
fn device_line<'l>(c: &aethelgard_twin::boot::StepCheck, line: &'l str) -> Option<&'l str> {
    witness_shaped(line, &c.device, c.level.1).filter(|b| !c.skipped.iter().any(|s| s == b))
}

/// The line index of each `{prefix}{k}` sentinel, `k` from 0 to `n` − 1: each
/// exactly once, as a whole line, in order, and none beyond `n`.
fn sentinels(lines: &[&str], prefix: &str, n: usize) -> Result<Vec<usize>, String> {
    let all = lines.iter().filter(|l| l.starts_with(prefix)).count();
    if all != n {
        return Err(format!("renode printed {all} `{prefix}` markers, not {n}"));
    }
    let mut at = Vec::with_capacity(n);
    for k in 0..n {
        let want = format!("{prefix}{k}");
        match (0..lines.len())
            .filter(|&i| lines[i] == want)
            .collect::<Vec<_>>()[..]
        {
            [i] if at.last().is_none_or(|&p| p < i) => at.push(i),
            _ => return Err(format!("renode did not print `{want}` once, in order")),
        }
    }
    Ok(at)
}

/// Judges one boot. A finding means `fail`; a `Stop` means the output cannot
/// be trusted (exit 5) or the tool exited nonzero. The exit code is never a
/// witness (`architecture.md` §2.4).
pub fn judge_boot(code: i32, out: &Output, plan: &BootPlan<'_>) -> Result<Judged, Stop> {
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
    let lines = clean_lines(&text);
    let begins: Vec<usize> = (0..lines.len()).filter(|&i| lines[i] == BEGIN).collect();
    let ends: Vec<usize> = (0..lines.len()).filter(|&i| lines[i] == END).collect();
    let errors: Vec<usize> = (0..lines.len())
        .filter(|&i| lines[i].contains(LOAD_ERROR))
        .collect();

    // Renode runs no command after one fails, so a load error leaves no reads.
    if let Some(&i) = errors.first() {
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
        return Ok((vec![Finding::plain(kind, message)], None, None));
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
    // The droop's sentinels all come before the reads; the probes' own
    // blocks hold the judge's reads, not the run's log (F3).
    let released = plan
        .resets
        .iter()
        .filter(|r| r.released_ns.is_some())
        .count();
    let releases = sentinels(&lines[..begin], RELEASE, released).map_err(untrusted)?;
    let probe_begins = sentinels(&lines[..begin], PROBE_BEGIN, released).map_err(untrusted)?;
    let probe_ends = sentinels(&lines[..begin], PROBE_END, released).map_err(untrusted)?;
    let in_probe = |i: usize| {
        probe_begins
            .iter()
            .zip(&probe_ends)
            .any(|(&b, &e)| b <= i && i <= e)
    };
    // Every log line the run caused: with synchronous logging, all of them
    // are on the console before `RunFor` returns, so before `BEGIN`.
    let run_log: Vec<&str> = (0..begin)
        .filter(|&i| !in_probe(i))
        .map(|i| lines[i])
        .collect();
    // The boot the reads judge: after the last release, if the droop had one.
    let last_boot: Vec<&str> = (releases.last().map_or(0, |&r| r + 1)..begin)
        .filter(|&i| !in_probe(i))
        .map(|i| lines[i])
        .collect();

    // Between the sentinels: the reads, each a bare value, and whole log
    // lines the judge's own reads caused, which are never the firmware's
    // (`AET-WO-2204` D4). Anything else is refused, never read.
    let reads = plan.reads();
    let mut values = Vec::with_capacity(reads.len());
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
    if values.len() != reads.len() {
        return Err(untrusted(format!(
            "renode printed {} values between the markers, not the {} reads",
            values.len(),
            reads.len()
        )));
    }
    let checks = &plan.image.identity.checks;
    // Past the filter's budget a log line may have been dropped, so no
    // witness can be read whole: every one is unwitnessed, never passed
    // (`AET-WO-2204` D2, ruled). A line the model writes that is not a
    // transfer, exactly as the twin lists it, is skipped; nothing else is
    // (`AET-WO-4200` D2).
    let (seen, unrouted) = attribute(checks, &last_boot);
    let witnesses: Vec<Option<Vec<String>>> = seen
        .into_iter()
        .map(|w| (out.dropped_log_lines == 0).then_some(w))
        .collect();
    let mut observed = Observed {
        boot_word: 0,
        instructions: 0,
        mips: 0,
        pc: 0,
        icsr: 0,
        loops: vec![[0, 0]; plan.self_loops.len()],
        registers: vec![0; plan.image.registers.len()],
        records: vec![(0, 0); checks.len()],
        witnesses,
        absent: checks
            .iter()
            .map(|c| match c.wire {
                aethelgard_twin::boot::Wire::I2c { .. } => {
                    let bus = c.device.split('.').next().unwrap_or_default();
                    last_boot
                        .iter()
                        .filter_map(|l| absent_address(l, bus))
                        .collect()
                }
                aethelgard_twin::boot::Wire::Spi { .. } => Vec::new(),
            })
            .collect(),
        unrouted,
        // Past the filter's budget an edge may have been dropped, so no
        // output's edges can be read whole (`AET-WO-2204` D2's rule).
        motors: plan
            .motors
            .iter()
            .map(|m| {
                let edges = (out.dropped_log_lines == 0)
                    .then(|| observer_edges(&m.observer.source, &last_boot));
                witness_motor(m, edges, plan.virtual_ns)
            })
            .collect(),
    };
    for ((cmd, read), line) in reads.iter().zip(values) {
        let v = value(line).ok_or_else(|| untrusted(format!("`{cmd}` read no value")))?;
        let word = || u32::try_from(v).map_err(|_| untrusted(format!("`{cmd}` read {v:#x}")));
        match *read {
            Read::BootWord => observed.boot_word = word()?,
            Read::Instructions => observed.instructions = v,
            Read::Mips => observed.mips = v,
            Read::Pc => observed.pc = word()?,
            Read::Icsr => observed.icsr = word()?,
            Read::Loop(i, k) => observed.loops[i][usize::from(k)] = word()?,
            Read::Register(i) => observed.registers[i] = word()?,
            Read::Record(i, 0) => observed.records[i].0 = word()?,
            Read::Record(i, _) => {
                let r = &mut observed.records[i].1;
                *r = (*r << 32) | u64::from(word()?);
            }
        }
    }
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
    let expected = plan.instructions();
    if observed.instructions > expected {
        return Err(untrusted(format!(
            "the core executed {} instructions in a run that holds {expected}",
            observed.instructions
        )));
    }
    let booted = observed.boot_word == aethelgard_twin::boot::BOOT_MAGIC;
    let halted = observed.instructions < expected;
    if booted && halted {
        return Err(untrusted(format!(
            "the boot word is set but the core executed {} of {expected} instructions",
            observed.instructions
        )));
    }
    let mut findings = Vec::new();
    if out.dropped_log_lines > 0 {
        findings.push(Finding::info(
            "renode_log_flood",
            format!(
                "Renode's output passed {} MiB; its log lines past that were counted and dropped, the first of each kind the judge reads kept",
                LOG_FILTER.budget / (1024 * 1024)
            ),
        ));
    }
    // Only the run's own log is the firmware's: a line the judge's reads
    // caused (a read of a peripheral whose clock is off) comes after `BEGIN`
    // or inside a probe's block, and is never attributed (F3).
    if let Some(line) = run_log.iter().find(|l| l.contains(LOG_ERROR)) {
        findings.push(Finding::plain(
            "renode_error_log",
            log_text(line).to_owned(),
        ));
    }
    // The count of such lines differs run to run under a flood (`AET-WO-2301`
    // F10), and Renode's timestamp always does: neither is in the message.
    if let Some(first) = run_log
        .iter()
        .find(|l| LOCKED.iter().any(|w| l.contains(w)))
    {
        findings.push(Finding::plain(
            "locked_peripheral_access",
            format!(
                "one or more accesses to a peripheral whose clock was off; the first: {}",
                log_text(first)
            ),
        ));
    }
    if !booted {
        findings.push(Finding::plain(
            "boot_word_missing",
            format!(
                "the boot word at {:#010x} reads {:#010x}, not {:#010x}",
                plan.boot_word,
                observed.boot_word,
                aethelgard_twin::boot::BOOT_MAGIC
            ),
        ));
    }
    if halted {
        findings.push(Finding::plain(
            "cpu_halted",
            format!(
                "the core executed {} of the run's {expected} instructions",
                observed.instructions
            ),
        ));
    }
    // Thumb code runs at even addresses; the PC Renode reports has no Thumb bit.
    let pc = u64::from(observed.pc);
    let inside = plan.executable.iter().any(|&(s, e)| pc >= s && pc < e);
    if !inside {
        findings.push(Finding::plain(
            aethelgard_chaos::campaign::PC_OUTSIDE_FINDING,
            format!(
                "the core ended at {:#010x}, outside every executable segment of the firmware",
                observed.pc
            ),
        ));
    }
    // After the boot word, the core must be parked in thread mode on a
    // self-loop (`AET-WO-2204` D3). That is all a pass claims: not that the
    // loop is the generator's fixed point (a named residual).
    if booted {
        let active = observed.icsr & aethelgard_twin::boot::VECTACTIVE_MASK;
        if active != 0 {
            findings.push(Finding::plain(
                "core_in_exception",
                format!(
                    "the core ended handling exception {active} (ICSR.VECTACTIVE), not in thread mode"
                ),
            ));
        }
        // On a loop the ELF holds, and that the machine still holds there:
        // a firmware can rewrite its own flash in Renode (adversarial A1).
        let intact = [
            u32::from(aethelgard_twin::boot::YIELD),
            u32::from(aethelgard_twin::boot::B_N_BACK),
        ];
        let on = plan.self_loops.iter().position(|&a| pc == a || pc == a + 2);
        let rewritten = on.is_some_and(|i| observed.loops[i] != intact);
        if inside && (on.is_none() || rewritten) {
            findings.push(Finding::plain(
                aethelgard_chaos::campaign::SELF_LOOP_FINDING,
                if rewritten {
                    format!(
                        "the core ended at {:#010x}, on a `yield; b.n` self-loop of the firmware that the machine no longer holds",
                        observed.pc
                    )
                } else {
                    format!(
                        "the core ended at {:#010x}, which is not on a `yield; b.n` self-loop of the firmware",
                        observed.pc
                    )
                },
            ));
        }
    }
    // Each register under its mask. `SysTick.CSR` is judged under
    // `SYST_CSR_ARMED`, 0x00010003 (`ENABLE`, `TICKINT`, `COUNTFLAG`), since
    // Renode reads `CLKSOURCE` as 1 whatever is written (`NVIC.cs` L1040 at
    // `066a7f13`), and `NVIC.ISER0`-`4` whole: nothing armed, nothing counted
    // (`AET-WO-4210` D-B1, accepted).
    for (r, &got) in plan.image.registers.iter().zip(&observed.registers) {
        if got & r.mask != r.value {
            findings.push(Finding::plain(
                "register_mismatch",
                format!(
                    "{} at {:#010x} reads {got:#010x}; under mask {:#010x} it must be {:#010x}",
                    r.name, r.address, r.mask, r.value
                ),
            ));
        }
    }
    // The identity reads. A record is only meaningful once the firmware has
    // reached its fixed point, which it does after writing every record;
    // before that `boot_word_missing` already says the run failed.
    if booted {
        for (c, &(status, value)) in checks.iter().zip(&observed.records) {
            findings.extend(identity_finding(c, status, value));
        }
        findings.extend(witness_findings(checks, &observed));
    } else {
        findings.extend(reads_outside_window(
            checks,
            &observed.records,
            plan.virtual_ns,
            plan.window_ns,
        ));
    }
    // The motor outputs (`AET-WO-4300a` §1.4): the static check judges
    // silicon from the topology alone, the twin's witness the model's
    // conventions, never blended (`4300` D3). The witness is read only once
    // the firmware reached its fixed point, as an identity record is.
    if booted {
        for (m, w) in plan.motors.iter().zip(&observed.motors) {
            findings.extend(witness_findings_of(m, w, plan.window_ns));
        }
    }
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
    Ok((
        findings,
        Some(Machine { cpus, peripherals }),
        Some(observed),
    ))
}

/// Each record that claims its step completed, against what Renode logged
/// for its device in that step (`AET-WO-2204` D1, `AET-WO-4210` S2):
/// exactly the model's expected lines, as whole lines, in order. Each
/// device's steps are judged in its script's order, by device, never across
/// devices, so two devices' steps may interleave (`2204` A4, closed). A
/// record that says a wait expired claims nothing. Whatever its record says,
/// a step in which the device logged an `[ERROR]` fails by name, beside the
/// run's `renode_error_log` (`AET-WO-4210` D-A1, ruled).
fn witness_findings(
    checks: &[aethelgard_twin::boot::StepCheck],
    observed: &Observed,
) -> Vec<Finding> {
    let mut out = Vec::new();
    for (i, c) in checks.iter().enumerate() {
        let seen = observed.witnesses[i].as_deref();
        if let Some(e) = seen.and_then(|s| s.iter().find(|l| l.starts_with(LOG_ERROR))) {
            out.push(Finding::plain(
                &format!("{}_step_error", bus_word(c)),
                format!(
                    "{}, its {} step: Renode logged an error for `{}` inside the step: {e}",
                    who(c),
                    c.step.name(),
                    c.device
                ),
            ));
        } else if observed.records[i].0 == aethelgard_twin::boot::STATUS_OK {
            out.extend(witness_finding(c, seen));
        }
    }
    out
}

/// The finding for one record that claims its read completed, or none when
/// Renode logged exactly the model's lines for it.
fn witness_finding(
    c: &aethelgard_twin::boot::StepCheck,
    seen: Option<&[String]>,
) -> Option<Finding> {
    let who = format!("{}, its {} step", who(c), c.step.name());
    let bus = bus_word(c);
    let Some(want) = &c.witness else {
        return Some(Finding::plain(
            &format!("{bus}_witness_unavailable"),
            format!(
                "{who}: the twin knows no witness format for this step of its model, so it cannot be witnessed"
            ),
        ));
    };
    match seen {
        Some(seen) if seen == want.as_slice() => None,
        Some(seen) => Some(Finding::plain(
            &format!("{bus}_transaction_unwitnessed"),
            format!(
                "{who}: the record says the read completed, but Renode logged {seen:?} for `{}`, not {want:?}",
                c.device
            ),
        )),
        None => Some(Finding::plain(
            &format!("{bus}_transaction_unwitnessed"),
            format!(
                "{who}: the record says the read completed, but the log passed the filter's budget, so `{}`'s witness cannot be read whole",
                c.device
            ),
        )),
    }
}

/// On a board whose window is shorter than `MAX_VIRTUAL_NS` (a DShot board,
/// `AET-WO-4300a` D1, ruled (a)), every declared identity read whose record
/// is still unset when the window ends did not complete inside it, and is
/// named; the run already fails on its missing boot word. A record the
/// firmware wrote, however it reads, is not named here.
fn reads_outside_window(
    checks: &[aethelgard_twin::boot::StepCheck],
    records: &[(u32, u64)],
    boot_ns: u64,
    window_ns: u64,
) -> Vec<Finding> {
    if window_ns >= aethelgard_twin::boot::MAX_VIRTUAL_NS {
        return Vec::new();
    }
    checks
        .iter()
        .zip(records)
        .filter(|(_, r)| r.0 == aethelgard_twin::boot::RECORD_UNSET)
        .map(|(c, _)| {
            // A script's later steps say which step (`AET-WO-4210`).
            let what = match c.step {
                aethelgard_ir::parts::StepKind::Identity => "its identity read".to_owned(),
                step => format!("its {} step's read", step.name()),
            };
            Finding::plain(
                "identity_read_outside_window",
                format!(
                    "{}: {what} did not complete inside this boot of {boot_ns} ns (a board with a DShot output boots inside its window of {window_ns} ns, AET-WO-4300a D1)",
                    who(c)
                ),
            )
        })
        .collect()
}

/// The finding for one identity record, or none when the read passed. The
/// expectation is the check's, which comes from the part table, never from
/// the firmware (`INV-07`).
fn identity_finding(
    c: &aethelgard_twin::boot::StepCheck,
    status: u32,
    value: u64,
) -> Option<Finding> {
    use aethelgard_twin::boot::Outcome;
    // An identity step is named as every identity read always was; a
    // script's later steps say which step (`AET-WO-4210`).
    let who = match c.step {
        aethelgard_ir::parts::StepKind::Identity => who(c),
        step => format!("{}, its {} step", who(c), step.name()),
    };
    let bus = bus_word(c);
    let what = if bus == "i2c" { "register" } else { "command" };
    let wrote: Vec<String> = c.write.iter().map(|b| format!("{b:#04x}")).collect();
    let wrote = wrote.join(" ");
    let under = if c.mask == u64::MAX >> (64 - 8 * u32::from(c.width.clamp(1, 8))) {
        String::new()
    } else {
        format!(" under mask {:#x}", c.mask)
    };
    match c.judge(status, value) {
        Outcome::Pass => None,
        Outcome::Mismatch => Some(Finding::plain(
            &format!("{bus}_identity_mismatch"),
            format!(
                "{who}: {what} {wrote} read {value:#x}, the part table says {:#x}{under}",
                c.expected
            ),
        )),
        Outcome::Timeout(stage) => Some(Finding::plain(
            &format!("{bus}_timeout"),
            format!(
                "{who}: the firmware's wait for {stage} expired reading {what} {wrote} (expected {:#x})",
                c.expected
            ),
        )),
        Outcome::Invalid => Some(Finding::plain(
            &format!("{bus}_record_invalid"),
            format!(
                "{who}: the record reads status {status:#010x} and value {value:#010x}, which no completed or timed-out transfer writes"
            ),
        )),
    }
}

/// What a probe read after a release (`AET-WO-2301` §2.3): the boot word,
/// then the first identity record's status and value, if the image has one,
/// and what Renode logged for that record's device since the release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Probe {
    pub at_ns: u64,
    pub boot_word: u32,
    pub record: Option<(u32, u32)>,
    /// The first check's witness-shaped lines from the release to the probe;
    /// `None` without a check, or when the log could not be read whole.
    pub witness: Option<Vec<String>>,
}

/// Each release's probe, read between its own whole-line sentinels: exactly
/// one pair per release and none other, and between them exactly the
/// probe's reads, each a bare value (`value`), and whole log lines the
/// judge's reads caused, or the output is not trusted. A boot killed at a
/// bound has read none; its probes are absent, never guessed.
pub fn probes(out: &Output, plan: &BootPlan<'_>) -> Result<Vec<Probe>, Stop> {
    probes_in(&out.text(), out.dropped_log_lines > 0, plan)
}

fn probes_in(text: &str, dropped: bool, plan: &BootPlan<'_>) -> Result<Vec<Probe>, Stop> {
    let untrusted = |what: String| {
        Stop::tool_error(TOOL, format!("{what}{}", said::suffix(text, Style::Renode)))
    };
    let lines = clean_lines(text);
    let releases: Vec<u64> = plan.resets.iter().filter_map(|r| r.released_ns).collect();
    let n = releases.len();
    let released = sentinels(&lines, RELEASE, n).map_err(untrusted)?;
    let begins = sentinels(&lines, PROBE_BEGIN, n).map_err(untrusted)?;
    let ends = sentinels(&lines, PROBE_END, n).map_err(untrusted)?;
    let reads = plan.probe_reads();
    let check = plan.image.identity.checks.first();
    let mut out = Vec::new();
    for (k, release) in releases.iter().enumerate() {
        let (r, b, e) = (released[k], begins[k], ends[k]);
        if !(r < b && b < e) {
            return Err(untrusted(format!(
                "probe {k}: its release and markers are out of order"
            )));
        }
        // A whole Renode log line inside the block is the judge's reads',
        // not a read; anything else that is not a bare value is refused.
        let mut words = Vec::new();
        for line in &lines[b + 1..e] {
            match value(line) {
                Some(v) => words.push(u32::try_from(v).map_err(|_| {
                    untrusted(format!(
                        "probe {k}: `{}` is not a word",
                        line.escape_debug()
                    ))
                })?),
                None if is_log_line(line) => {}
                None => {
                    return Err(untrusted(format!(
                        "probe {k}: `{}` is neither a read nor a whole log line",
                        line.escape_debug()
                    )));
                }
            }
        }
        if words.len() != reads.len() {
            return Err(untrusted(format!(
                "probe {k}: {} values between its markers, not the {} reads",
                words.len(),
                reads.len()
            )));
        }
        let witness = check.filter(|_| !dropped).map(|c| {
            own_lines(c, &lines[r + 1..b])
                .into_iter()
                .map(|(_, l)| l.to_owned())
                .collect()
        });
        out.push(Probe {
            at_ns: release + power::RECOVERY_BOUND_NS,
            boot_word: words[0],
            record: check.map(|_| (words[1], words[2])),
            witness,
        });
    }
    Ok(out)
}

/// The build `renode --version` names on its `build:` line, as Renode prints
/// it (`1.17.0+20260907gitf1dd1b4af` in the pinned image): `None` unless
/// exactly one such line holds 1 to 256 printable ASCII bytes and no space.
pub fn build(version_stdout: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(version_stdout);
    let mut builds = text
        .lines()
        .filter_map(|l| l.trim().strip_prefix("build:"))
        .map(str::trim);
    let build = builds.next()?;
    let ok = builds.next().is_none()
        && (1..=aethelgard_chaos::event::MAX_TOOL_VERSION_BYTES).contains(&build.len())
        && build.bytes().all(|b| (0x21..=0x7e).contains(&b));
    ok.then(|| build.to_owned())
}

/// How a record's status reads in the verdict.
pub fn status_name(c: &aethelgard_twin::boot::StepCheck, status: u32) -> &'static str {
    use aethelgard_twin::boot::Outcome;
    // The check's own judgement names it, so a status counts only on its own
    // bus (`AET-WO-4200`): an I2C record holding `RXP`'s word is invalid.
    match c.judge(status, c.expected) {
        Outcome::Pass | Outcome::Mismatch => "ok",
        Outcome::Timeout("TXIS") => "timeout_txis",
        Outcome::Timeout("TC") => "timeout_tc",
        Outcome::Timeout("RXNE") => "timeout_rxne",
        Outcome::Timeout("STOPF") => "timeout_stopf",
        Outcome::Timeout("RXP") => "timeout_rxp",
        Outcome::Timeout(_) | Outcome::Invalid => "invalid",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `AET-WO-4200` D2: of a device's own lines, exactly those its model
    /// lists as not a transfer are skipped; every other line of the device is
    /// kept, so it can fail the witness; a line of another device is not this
    /// one's, its path a prefix or not.
    #[test]
    fn a_device_s_line_is_kept_unless_its_model_lists_it_as_skipped() {
        let c = aethelgard_twin::boot::StepCheck {
            reference: "U5".into(),
            peripheral: "SPI1".into(),
            wire: aethelgard_twin::boot::Wire::Spi {
                address: 1,
                select: "PA1".into(),
            },
            step: aethelgard_ir::parts::StepKind::Identity,
            write: vec![0x80],
            width: 1,
            expected: 0xEA,
            mask: 0xFF,
            status_at: 0x2000_0004,
            value_at: 0x2000_0008,
            level: (-1, "[NOISY] "),
            device: "spi1.spi1mux.u5".into(),
            witness: None,
            idle_warning: None,
            skipped: vec!["[NOISY] spi1.spi1mux.u5: Finishing transmission, going idle".into()],
            stimulus: Some(Vec::new()),
        };
        let at = |b: &str| format!("[00:00:00.040010000] {b}");
        let idle = at("[NOISY] spi1.spi1mux.u5: Finishing transmission, going idle");
        assert_eq!(device_line(&c, &idle), None);
        let read = at("[NOISY] spi1.spi1mux.u5: Received byte 0x80, returning 0x0");
        assert_eq!(
            device_line(&c, &read),
            Some("[NOISY] spi1.spi1mux.u5: Received byte 0x80, returning 0x0")
        );
        // A skip line with anything appended is not the skip line.
        let longer = at("[NOISY] spi1.spi1mux.u5: Finishing transmission, going idle (2)");
        assert!(device_line(&c, &longer).is_some());
        for other in [
            "[NOISY] spi1.spi1mux.u50: Received byte 0x80, returning 0x0",
            "[NOISY] spi1.spi1mux: GPIO #1 set to False",
        ] {
            assert_eq!(device_line(&c, &at(other)), None, "{other}");
        }
    }

    fn image() -> aethelgard_twin::boot::Image {
        let topology: aethelgard_ir::SolvedTopology = serde_json::from_str(
            r#"{"schema_version":6,"specification_sha256":"acd60081f2d111b1b1ed0d974078fff45ccf2a967ec5f42317417f8ce68149d1","assignments":{"I2C_SCL":{"pin":"PB6","peripheral":"I2C1","signal":"I2C1_SCL","alternate_function":4},"I2C_SDA":{"pin":"PB7","peripheral":"I2C1","signal":"I2C1_SDA","alternate_function":4}},"gpio":{},"i2c_ceilings":{"SENSORS":400000},"i2c_speeds":{"SENSORS":400000},"analog_lines":{"I2C_SCL":{"bus":"SENSORS","pull_up_ohms":2200,"capacitance_femtofarads":100000,"ceiling_ns":300,"domain_millivolts":3300},"I2C_SDA":{"bus":"SENSORS","pull_up_ohms":2200,"capacitance_femtofarads":100000,"ceiling_ns":300,"domain_millivolts":3300}},"i2c_devices":{"U2":{"index":0,"part":"BME280","bus":"SENSORS","peripheral":"I2C1","address":118},"U3":{"index":1,"part":"TMP117","bus":"SENSORS","peripheral":"I2C1","address":72}},"spi_devices":{},"spi_lines":{},"clock":{"source":"HSI","sysclk_hz":64000000,"hse_hz":0,"d1cpre":1,"hpre":1,"d1ppre":1,"d2ppre1":1,"d2ppre2":1,"d3ppre":1,"timpre":false},"motor_outputs":{},"power":null}"#,
        )
        .unwrap();
        aethelgard_twin::boot::expected(&topology).unwrap()
    }

    /// The booted platform's U2, as the listing names it.
    static INSTANCES: std::sync::LazyLock<Vec<(String, String)>> =
        std::sync::LazyLock::new(|| vec![("u2".to_owned(), "BME280".to_owned())]);

    fn plan<'a>(
        image: &'a aethelgard_twin::boot::Image,
        resets: &'a [power::Reset],
    ) -> BootPlan<'a> {
        BootPlan {
            platform: std::path::Path::new("/w/platform.repl"),
            firmware: std::path::Path::new("/w/firmware.elf"),
            virtual_ns: 100_000_000,
            window_ns: 100_000_000,
            boot_word: 0x2000_0000,
            vector_table: 0x0800_0000,
            image,
            executable: &[],
            self_loops: &[],
            motors: &[],
            instances: &INSTANCES,
            resets,
        }
    }

    /// The witness of a run whose window ends right after the edges given.
    fn witness_motor_at(m: &Motor, edges: Option<Option<Vec<(u64, bool)>>>) -> MotorWitness {
        let end = edges
            .as_ref()
            .and_then(|e| e.as_ref())
            .and_then(|e| e.iter().map(|x| x.0).max())
            .unwrap_or(0);
        witness_motor(m, edges, end)
    }

    /// The PWM fixture's first output: the model's 639,744 ns period and
    /// 256,000 ns high time (PSC 63, ARR 2499, CCR 1000 at 4 ns a tick).
    fn motor() -> Motor {
        Motor {
            observer: aethelgard_twin::timer::Observer {
                label: "MOTOR1".into(),
                protocol: aethelgard_ir::MotorProtocol::Pwm,
                timer: 1,
                channel: 1,
                pin: "PA8".into(),
                port: 'A',
                index: 8,
                alternate_function: 1,
                name: "pa8obs".into(),
                source: "gpioPortA.pa8obs".into(),
                period_ns: 639_744,
                high_ns: 256_000,
            },
            psc: 63,
            arr: 2499,
            ccr: 1000,
            silicon: aethelgard_ir::motor::check_silicon(
                &aethelgard_ir::SolvedClock::reset(),
                1,
                &aethelgard_ir::SolvedMotorOutput {
                    index: 0,
                    protocol: aethelgard_ir::MotorProtocol::Pwm,
                    pwm: Some(aethelgard_ir::PwmTiming {
                        rate_hz: 400,
                        pulse_min_us: 1000,
                        pulse_max_us: 2000,
                    }),
                    psc: 63,
                    arr: 2499,
                    ccr: 1000,
                },
            ),
        }
    }

    /// Edges as the probes measured them (`pwm400-63`): each the exact time
    /// plus a deviation under the 10 ns grain, from the first rise at 639,750.
    fn measured(n: u64, jitter: &[i64]) -> Vec<(u64, bool)> {
        let mut out = Vec::new();
        for k in 0..n {
            let d = |i: u64| jitter[(i as usize) % jitter.len()];
            let rise = 639_750 + k * 639_744;
            out.push(((rise as i64 + d(2 * k)) as u64, true));
            out.push(((rise as i64 + 256_000 + d(2 * k + 1)) as u64, false));
        }
        out
    }

    #[test]
    fn a_motor_s_edges_on_the_model_s_sequence_are_witnessed() {
        let m = motor();
        let w = witness_motor_at(&m, Some(Some(measured(150, &[0, -6, 2, -4]))));
        assert_eq!(w.status, "witnessed", "{w:?}");
        assert_eq!((w.rises, w.falls, w.first_ns), (150, 150, Some(639_750)));
        assert_eq!(w.span_ns, Some(8));
        assert!(witness_findings_of(&m, &w, aethelgard_twin::boot::MAX_VIRTUAL_NS).is_empty());
    }

    #[test]
    fn adversarial_2_a_correct_output_one_tick_off_or_the_silicon_period_fails() {
        let m = motor();
        // Silicon's ARR + 1 period, 256 ns longer each period.
        let mut e = measured(10, &[0]);
        for (i, edge) in e.iter_mut().enumerate() {
            edge.0 += 256 * (i as u64 / 2);
        }
        assert_eq!(witness_motor_at(&m, Some(Some(e))).status, "mismatch");
        // One grain off on one edge alone.
        let mut e = measured(10, &[0]);
        e[7].0 += 10;
        let w = witness_motor_at(&m, Some(Some(e)));
        assert_eq!((w.status, w.span_ns), ("mismatch", Some(10)));
        let f = witness_findings_of(&m, &w, aethelgard_twin::boot::MAX_VIRTUAL_NS);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind, "motor_timing_mismatch");
    }

    #[test]
    fn adversarial_1_store_made_edges_fail_and_a_single_pulse_is_too_few() {
        let m = motor();
        // `BSRR` edges (and the `UG` + `BSRR` pair) sit on the 100 us grid,
        // 6 ms apart, high 400 us: not the model's sequence.
        let store: Vec<(u64, bool)> = (0..4)
            .flat_map(|k| [(k * 6_000_000, true), (k * 6_000_000 + 400_000, false)])
            .collect();
        assert_eq!(witness_motor_at(&m, Some(Some(store))).status, "mismatch");
        // A store that copies the period exactly still cannot leave the grid:
        // the model's period here is not a multiple of 100 us, so two rises
        // on the grid cannot be one period apart.
        let on_grid = vec![(0, true), (300_000, false), (700_000, true)];
        assert_eq!(witness_motor_at(&m, Some(Some(on_grid))).status, "mismatch");
        let one = vec![(639_750, true), (895_750, false)];
        let w = witness_motor_at(&m, Some(Some(one)));
        assert_eq!(w.status, "unwitnessed");
        assert_eq!(
            witness_findings_of(&m, &w, aethelgard_twin::boot::MAX_VIRTUAL_NS)[0].kind,
            "motor_output_unwitnessed"
        );
        assert_eq!(
            witness_motor_at(&m, Some(Some(vec![]))).status,
            "unwitnessed"
        );
    }

    #[test]
    fn a_timing_whose_edges_all_fall_on_the_grid_is_judged_static_only() {
        let mut m = motor();
        // A period and high time that are whole quanta: every edge on the grid.
        m.observer.period_ns = 2_000_000;
        m.observer.high_ns = 1_000_000;
        let e: Vec<(u64, bool)> = (0..5)
            .flat_map(|k| [(k * 2_000_000, true), (k * 2_000_000 + 1_000_000, false)])
            .collect();
        let w = witness_motor_at(&m, Some(Some(e)));
        assert_eq!(w.status, "static_only");
        let f = witness_findings_of(&m, &w, aethelgard_twin::boot::MAX_VIRTUAL_NS);
        assert_eq!(f.len(), 1);
        assert_eq!(
            (f[0].kind.as_str(), f[0].severity.as_str()),
            ("motor_witness_static_only", "info")
        );
    }

    #[test]
    fn batched_edges_unreadable_logs_and_odd_lines_never_pass() {
        let m = motor();
        let mut e = measured(10, &[0]);
        e[4].0 = e[3].0;
        assert_eq!(witness_motor_at(&m, Some(Some(e))).status, "batched");
        assert_eq!(witness_motor_at(&m, None).status, "unreadable");
        assert_eq!(witness_motor_at(&m, Some(None)).status, "mismatch");
        // Starting on a fall is not the model's sequence.
        let mut e = measured(10, &[0]);
        e.remove(0);
        assert_eq!(witness_motor_at(&m, Some(Some(e))).status, "mismatch");
        let lines = [
            "[00:00:00.000639750] [NOISY] gpioPortA.pa8obs: LED state changed to True",
            "[00:00:00.000639750] [NOISY] gpioPortA.pa9obs: LED state changed to True",
            "[00:00:00.000895750] [NOISY] gpioPortA.pa8obs: LED state changed to False",
            "[00:00:00.000895750] [NOISY] timer1: IRQ pending",
        ];
        assert_eq!(
            observer_edges("gpioPortA.pa8obs", &lines),
            Some(vec![(639_750, true), (895_750, false)])
        );
        let odd = ["[00:00:00.000639750] [INFO] gpioPortA.pa8obs: LED state changed to True"];
        assert_eq!(observer_edges("gpioPortA.pa8obs", &odd), None);
    }

    /// Adversarial H1: a timer stopped after two periods (or started late,
    /// which the coverage rule does not judge) passes no witness once the
    /// window runs on past its last rise.
    #[test]
    fn adversarial_h1_edges_that_stop_before_the_window_s_end_fail() {
        let m = motor();
        let two = measured(2, &[0]);
        let w = witness_motor(&m, Some(Some(two.clone())), 100_000_000);
        assert_eq!(w.status, "stopped");
        assert_eq!(
            witness_findings_of(&m, &w, aethelgard_twin::boot::MAX_VIRTUAL_NS)[0].kind,
            "motor_timing_mismatch"
        );
        // Within a period and a grain of the end, it covers the window.
        let end = two[2].0 + 639_744 + 10;
        assert_eq!(witness_motor(&m, Some(Some(two)), end).status, "witnessed");
    }

    #[test]
    fn a_static_mismatch_is_its_own_finding() {
        let mut m = motor();
        m.silicon.holds = false;
        let f = static_motor_findings(std::slice::from_ref(&m));
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind, "motor_timing_static_mismatch");
    }

    fn bor3() -> Vec<power::Reset> {
        vec![power::Reset {
            asserted_ns: 1_014_430,
            released_ns: Some(1_227_980),
            cause: vec!["VBOR3".to_owned()],
        }]
    }

    /// `AET-WO-4300a` D1 and adversarial F5: a correct slow output in a
    /// window shorter than two of its periods fails closed, and says why.
    #[test]
    fn an_output_too_slow_for_its_window_is_named_with_the_reason() {
        let m = motor();
        let short = witness_motor(&m, Some(Some(vec![(639_750, true)])), 1_000_000);
        assert_eq!(short.status, "unwitnessed");
        let f = witness_findings_of(&m, &short, 1_000_000);
        assert_eq!(f.len(), 1);
        assert!(
            f[0].message
                .contains("shorter than two of its periods (1279488 ns)"),
            "{}",
            f[0].message
        );
        // A boot shorter than a long enough window: the reason is not given
        // (third adversarial review, F3: `--for 1ms` on a 100 ms board).
        let long = witness_motor(&m, Some(Some(vec![(639_750, true)])), 1_000_000);
        assert!(
            !witness_findings_of(&m, &long, aethelgard_twin::boot::MAX_VIRTUAL_NS)[0]
                .message
                .contains("shorter than two")
        );
    }

    /// `AET-WO-4300a` D1 (a): on a DShot board's short window, a declared
    /// identity read whose record is still unset is named; one the firmware
    /// wrote is not, and on the 100 ms window nothing is (the missing boot
    /// word already fails the run).
    #[test]
    fn a_read_left_unset_by_a_short_window_is_named() {
        use aethelgard_twin::boot::{DSHOT_WINDOW_NS, MAX_VIRTUAL_NS, RECORD_UNSET, STATUS_OK};
        let image = image();
        let checks = &image.identity.checks;
        assert!(!checks.is_empty());
        let unset = vec![(RECORD_UNSET, u64::from(RECORD_UNSET)); checks.len()];
        let named = reads_outside_window(checks, &unset, DSHOT_WINDOW_NS, DSHOT_WINDOW_NS);
        assert_eq!(named.len(), checks.len());
        assert!(
            named
                .iter()
                .all(|f| f.kind == "identity_read_outside_window")
        );
        assert!(
            named[0].message.contains(&who(&checks[0])),
            "{}",
            named[0].message
        );
        assert!(
            named[0].message.contains("2000000 ns"),
            "{}",
            named[0].message
        );
        let written = vec![(STATUS_OK, 0x60); checks.len()];
        assert!(
            reads_outside_window(checks, &written, DSHOT_WINDOW_NS, DSHOT_WINDOW_NS).is_empty()
        );
        assert!(reads_outside_window(checks, &unset, MAX_VIRTUAL_NS, MAX_VIRTUAL_NS).is_empty());
    }

    #[test]
    fn a_log_line_loses_its_host_timestamp_and_nothing_else() {
        assert_eq!(
            log_text("[00:00:00.010040000] [WARNING] sysbus: Tried to write a locked peripheral"),
            "[WARNING] sysbus: Tried to write a locked peripheral"
        );
        assert_eq!(log_text("  plain  "), "plain");
        assert_eq!(log_text("[WARNING] x"), "[WARNING] x");
    }

    /// `AET-WO-2301` D1, D4, F3, F5: the droop's schedule is the measured
    /// recipe, at the schedule's times, and a static boot's commands are
    /// unchanged by it.
    #[test]
    fn a_droop_boot_runs_the_measured_recipe_at_the_schedules_times() {
        let image = image();
        let resets = bor3();
        let droop = plan(&image, &resets).commands();
        let static_ = plan(&image, &[]).commands();
        // `AET-WO-2204` D4: no level is ever changed; the device's log is set
        // to `Noisy` once, before the run (D1).
        assert!(static_.contains(
            "cpu VectorTableOffset 0x08000000; logLevel -1 sysbus.i2c1.u2; emulation RunFor \"0.100000000\"; echo \"AETHELGARD-BOOT-BEGIN\""
        ));
        assert!(!droop.contains("logLevel 3") && !static_.contains("logLevel 3"));
        assert_eq!(static_.matches("logLevel").count(), 1);
        assert!(static_.contains("sysbus ReadDoubleWord 0xe000ed04"));
        assert!(!static_.contains("machine Reset"));
        // A device the booted platform does not declare (a faulted bus) gets
        // no level: a level on no element is a failed command.
        let mut bare = plan(&image, &[]);
        bare.instances = &[];
        assert!(!bare.commands().contains("logLevel"));
        let rec = |at: u32| format!("sysbus WriteDoubleWord {at:#010x} 0xffffffff");
        let check = &image.identity.checks[0];
        let want = [
            "cpu VectorTableOffset 0x08000000".to_owned(),
            "logLevel -1 sysbus.i2c1.u2".to_owned(),
            "emulation RunFor \"0.001014430\"".to_owned(),
            "machine Reset".to_owned(),
        ]
        .join("; ");
        assert!(droop.contains(&want), "{droop}");
        let rest = [
            "cpu IsHalted true".to_owned(),
            "sysbus WriteDoubleWord 0x20000000 0x0".to_owned(),
            rec(check.status_at),
            rec(check.value_at),
            "emulation RunFor \"0.000213550\"".to_owned(),
            "cpu IsHalted false".to_owned(),
            "echo \"AETHELGARD-RELEASE-0\"".to_owned(),
            "emulation RunFor \"0.000000010\"".to_owned(),
            "emulation RunFor \"0.000099990\"".to_owned(),
            "echo \"AETHELGARD-PROBE-BEGIN-0\"".to_owned(),
        ]
        .join("; ");
        assert!(droop.contains(&rest), "{droop}");
        assert!(droop.contains(
            "echo \"AETHELGARD-PROBE-END-0\"; emulation RunFor \"0.098672020\"; echo \"AETHELGARD-BOOT-BEGIN\""
        ));
        // After `machine Reset` alone the core never runs (F3): the cold
        // writes and the vector table follow it.
        let after = droop.split("machine Reset; ").nth(1).unwrap();
        assert!(after.starts_with("sysbus WriteDoubleWord"));
        assert!(
            after
                .split("; cpu IsHalted true")
                .next()
                .unwrap()
                .ends_with("cpu VectorTableOffset 0x08000000")
        );
        assert_eq!(droop.matches("emulation RunFor \"0.000000010\"").count(), 1);
        assert_eq!(plan(&image, &resets).instructions(), 9_877_201);
        assert_eq!(plan(&image, &[]).instructions(), 10_000_000);
    }

    #[test]
    fn a_probe_is_read_only_between_its_own_whole_line_markers() {
        let image = image();
        let resets = bor3();
        let p = plan(&image, &resets);
        let lines = |l: &[&str]| l.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        let w = "[00:00:00.010040000] [NOISY] i2c1.u2: Write D0\n[00:00:00.010050000] [NOISY] i2c1.u2: Read 60";
        let block = |log: &str, rows: &[&str]| {
            format!(
                "x\nAETHELGARD-RELEASE-0\n{log}\nAETHELGARD-PROBE-BEGIN-0\n{}\nAETHELGARD-PROBE-END-0\n",
                rows.join("\n")
            )
        };
        let good = ["0x2100B007", "0x22020000", "0x00000060"];
        let want = [Probe {
            at_ns: 1_327_980,
            boot_word: 0x2100_B007,
            record: Some((0x2202_0000, 0x60)),
            witness: Some(lines(&[
                "[NOISY] i2c1.u2: Write D0",
                "[NOISY] i2c1.u2: Read 60",
            ])),
        }];
        assert_eq!(probes_in(&block(w, &good), false, &p).unwrap(), want);
        // A whole Renode log line inside the block is the judge's reads',
        // not a read and not the witness (`AET-WO-2204` F3).
        let mut logged = good.to_vec();
        logged.insert(
            2,
            "[00:00:00.010040000] [WARNING] sysbus: Tried to write a locked peripheral",
        );
        assert_eq!(probes_in(&block(w, &logged), false, &p).unwrap(), want);
        // Past the filter's budget the witness is unread, never guessed.
        assert_eq!(
            probes_in(&block(w, &good), true, &p).unwrap()[0].witness,
            None
        );
        // Lines before the release are the boot the reset ended, not this one.
        let early = format!("{w}\n{}", block("", &good));
        assert_eq!(
            probes_in(&early, false, &p).unwrap()[0].witness,
            Some(Vec::new())
        );
        let refused = [
            String::new(),
            block(w, &good[..2]),
            block(w, &["0x2100B007", "0x22020000", "0x60 0x61"]),
            // An echoed command's tail read as a value (`AET-WO-2301` F8).
            block(w, &["0x2100B007", "0x22020000", "0x20000008\""]),
            block(w, &["0x2100B007", "0x22020000", " [WARNING] 0x60"]),
            // A log line split below line level (F2) is not a whole line.
            block(
                w,
                &[
                    "0x2100B007",
                    "0x22020000",
                    "0x000000[00:00:00.010040000] [WARNING] x",
                ],
            ),
            format!("{}AETHELGARD-PROBE-BEGIN-1\n", block(w, &good)),
            "AETHELGARD-PROBE-END-0\n0x2100B007\nAETHELGARD-PROBE-BEGIN-0\n".to_owned(),
            // No release marker, or two.
            block(w, &good).replace("AETHELGARD-RELEASE-0\n", ""),
            format!("AETHELGARD-RELEASE-0\n{}", block(w, &good)),
            block(w, &["0x1FFFFFFFF", "0x22020000", "0x00000060"]),
        ];
        for r in refused {
            assert!(probes_in(&r, false, &p).is_err(), "accepted: {r:?}");
        }
    }

    /// `AET-WO-2204` D4: the configuration is Renode 1.17.0's own default
    /// (measured, and pinned on Renode by `twin_tier2::renode_s_configuration_
    /// ...`) with `use-synchronous-logging` and, since `AET-WO-4300a` (D2),
    /// `log-timestamps-type` changed, and the history under the call's home.
    #[test]
    fn the_configuration_is_renode_s_default_with_synchronous_logging() {
        let c = config(std::path::Path::new("/w/tool"));
        let mut lines: Vec<&str> = c.lines().collect();
        lines.sort_unstable();
        let mut want = vec![
            "[general]",
            "use-synchronous-logging = true",
            "terminal = Termsharp",
            "compiler-cache-enabled = False",
            "serialization-mode = Generated",
            "always-log-machine-name = False",
            "collapse-repeated-log-entries = True",
            "log-timestamps-type = Virtual",
            "log-history-limit = 1000",
            "history-path = /w/tool/.config/renode/history",
            "store-table-bits = 41",
            "[monitor]",
            "consume-exceptions-from-command = True",
            "break-script-on-exception = True",
            "number-format = Hexadecimal",
            "[plugins]",
            "enabled-plugins = ",
            "[translation]",
            "min-tb-size = 33554432",
            "max-tb-size = 536870912",
        ];
        want.sort_unstable();
        assert_eq!(lines, want);
    }

    #[test]
    fn a_log_line_s_virtual_stamp_is_its_time_in_nanoseconds() {
        assert_eq!(
            log_stamped("[01:02:03.019999010] [NOISY] gpioPortA.m1: x"),
            Some((3_723_019_999_010, "[NOISY] gpioPortA.m1: x"))
        );
        assert_eq!(log_stamped("[00:00:00.01999901] [NOISY] x"), None);
        assert_eq!(log_stamped("[00:00:00.0199990100] [NOISY] x"), None);
    }

    #[test]
    fn a_log_body_is_exactly_the_text_after_a_whole_virtual_timestamp() {
        assert_eq!(
            log_body("[00:00:00.010040000] [NOISY] i2c1.u2: Read 60"),
            Some("[NOISY] i2c1.u2: Read 60")
        );
        for not in [
            "[NOISY] i2c1.u2: Read 60",
            "x[00:00:00.010040000] [NOISY] i2c1.u2: Read 60",
            "[21:06:58.104] [NOISY] i2c1.u2: Read 60",
            "[00:00:00.010040000][NOISY] i2c1.u2: Read 60",
            "[2a:06:58.1004] [NOISY] i2c1.u2: Read 60",
        ] {
            assert_eq!(log_body(not), None, "{not}");
        }
        assert_eq!(
            witness_shaped(
                "[00:00:00.010040000] [NOISY] i2c1.u2: Read 60 (2)",
                "i2c1.u2",
                "[NOISY] "
            ),
            Some("[NOISY] i2c1.u2: Read 60 (2)")
        );
        assert_eq!(
            witness_shaped(
                "[00:00:00.010040000] [NOISY] i2c1.u20: Read 60",
                "i2c1.u2",
                "[NOISY] "
            ),
            None
        );
        assert_eq!(
            witness_shaped(
                "[00:00:00.010040000] [INFO] i2c1.u2: Read 60",
                "i2c1.u2",
                "[NOISY] "
            ),
            None
        );
        assert!(is_log_line("[00:00:00.010040000] [WARNING] sysbus: x"));
        assert!(!is_log_line("[00:00:00.010040000] WARNING sysbus: x"));
    }

    /// One step of a scripted device, witnessed by `witness`.
    fn step(
        wire: aethelgard_twin::boot::Wire,
        device: &str,
        step: aethelgard_ir::parts::StepKind,
        witness: &[&str],
    ) -> aethelgard_twin::boot::StepCheck {
        aethelgard_twin::boot::StepCheck {
            reference: "U7".into(),
            peripheral: "I2C1".into(),
            wire,
            step,
            write: vec![0x00],
            width: 1,
            expected: 0,
            mask: 0xFF,
            status_at: 0x2000_0004,
            value_at: 0x2000_0008,
            level: (-1, "[NOISY] "),
            device: device.into(),
            witness: Some(witness.iter().map(|&w| w.to_owned()).collect()),
            idle_warning: None,
            skipped: Vec::new(),
            stimulus: Some(Vec::new()),
        }
    }

    /// The findings of `checks` against `log`, every record `ok`.
    fn step_findings(checks: &[aethelgard_twin::boot::StepCheck], log: &[&str]) -> Vec<Finding> {
        let (seen, unrouted) = attribute(checks, log);
        let observed = Observed {
            boot_word: 0,
            instructions: 0,
            mips: 0,
            pc: 0,
            icsr: 0,
            loops: Vec::new(),
            registers: Vec::new(),
            records: vec![(aethelgard_twin::boot::STATUS_OK, 0); checks.len()],
            witnesses: seen.into_iter().map(Some).collect(),
            absent: vec![Vec::new(); checks.len()],
            motors: Vec::new(),
            unrouted,
        };
        witness_findings(checks, &observed)
    }

    /// `AET-WO-4210` D-A1 (ruled): a scripted device's `[ERROR]` line inside a
    /// step fails that step by name. On I2C the line counts toward no step's
    /// expected lines, so the steps around it keep their witnesses; on SPI it
    /// falls in its chip-select window.
    #[test]
    fn a_device_s_error_inside_a_step_fails_that_step_by_name() {
        use aethelgard_ir::parts::StepKind::{Configure, Data, Identity};
        let at = |b: &str| format!("[00:00:00.010040000] {b}");
        let i2c = || aethelgard_twin::boot::Wire::I2c { address: 0x63 };
        let d = "i2c1.u7";
        let checks = [
            step(i2c(), d, Identity, &["[NOISY] i2c1.u7: id"]),
            step(i2c(), d, Configure, &["[NOISY] i2c1.u7: cfg"]),
            step(i2c(), d, Data, &["[NOISY] i2c1.u7: data"]),
        ];
        let error = "[ERROR] i2c1.u7: Unknown command 0x80";
        for (log, named) in [
            (
                vec![
                    "[NOISY] i2c1.u7: id",
                    "[NOISY] i2c1.u7: cfg",
                    "[NOISY] i2c1.u7: data",
                ],
                None,
            ),
            (
                vec![
                    "[NOISY] i2c1.u7: id",
                    "[NOISY] i2c1.u7: cfg",
                    error,
                    "[NOISY] i2c1.u7: data",
                ],
                Some("configure"),
            ),
            (
                vec![
                    "[NOISY] i2c1.u7: id",
                    "[NOISY] i2c1.u7: cfg",
                    "[NOISY] i2c1.u7: data",
                    error,
                ],
                Some("data"),
            ),
            (
                vec![
                    error,
                    "[NOISY] i2c1.u7: id",
                    "[NOISY] i2c1.u7: cfg",
                    "[NOISY] i2c1.u7: data",
                ],
                Some("identity"),
            ),
        ] {
            let log: Vec<String> = log.into_iter().map(at).collect();
            let log: Vec<&str> = log.iter().map(String::as_str).collect();
            let f = step_findings(&checks, &log);
            let kinds: Vec<&str> = f.iter().map(|f| f.kind.as_str()).collect();
            match named {
                None => assert!(f.is_empty(), "{kinds:?}"),
                Some(name) => {
                    assert_eq!(kinds, ["i2c_step_error"], "{log:?}");
                    let m = &f[0].message;
                    assert!(
                        m.contains(&format!("U7 at I2C address 0x63, its {name} step")),
                        "{m}"
                    );
                    assert!(m.ends_with(error), "{m}");
                }
            }
        }
        // On SPI, the error falls in its step's chip-select window.
        let spi = || aethelgard_twin::boot::Wire::Spi {
            address: 1,
            select: "PA1".into(),
        };
        let d = "spi1.spi1mux.u7";
        let checks = [
            step(spi(), d, Identity, &["[NOISY] spi1.spi1mux.u7: id"]),
            step(spi(), d, Configure, &["[NOISY] spi1.spi1mux.u7: cfg"]),
        ];
        let (low, high) = (
            "[NOISY] spi1.spi1mux: GPIO #1 set to False",
            "[NOISY] spi1.spi1mux: GPIO #1 set to True",
        );
        let error = "[ERROR] spi1.spi1mux.u7: Unknown register";
        let log: Vec<String> = [
            low,
            "[NOISY] spi1.spi1mux.u7: id",
            error,
            high,
            low,
            "[NOISY] spi1.spi1mux.u7: cfg",
            high,
        ]
        .into_iter()
        .map(at)
        .collect();
        let log: Vec<&str> = log.iter().map(String::as_str).collect();
        let f = step_findings(&checks, &log);
        let kinds: Vec<&str> = f.iter().map(|f| f.kind.as_str()).collect();
        assert_eq!(kinds, ["spi_step_error"]);
        assert!(
            f[0].message.contains("its identity step"),
            "{}",
            f[0].message
        );
    }
}
