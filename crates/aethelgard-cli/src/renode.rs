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

/// That configuration (`AET-WO-2204` D4). It sets one key: Renode then writes
/// each log entry from the thread that logs it, with no queue and no log
/// thread (`Logger.cs` L822–834), so a line the firmware caused is on the
/// console before `RunFor` returns, never interleaved with the monitor's
/// output below line level (spike F2) and never dropped by a level change
/// (B10). Measured: Renode fills every other key with the defaults it writes
/// for itself when given no file, the history path and the folding of
/// repeated lines included; the files differ in this key alone.
pub const CONFIG: &str = "[general]\nuse-synchronous-logging = true\n";

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

/// A log line without Renode's host timestamp (`[21:06:58.1004] `), which
/// differs run to run and so never belongs in a finding (`INV-04`,
/// `AET-WO-2301` F10).
fn log_text(line: &str) -> &str {
    let line = line.trim();
    match line.strip_prefix('[').and_then(|r| r.split_once("] ")) {
        Some((stamp, rest))
            if stamp
                .bytes()
                .all(|b| b.is_ascii_digit() || b == b':' || b == b'.') =>
        {
            rest.trim()
        }
        _ => line,
    }
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
    pub boot_word: u32,
    /// Where the part boots from: `FLASH_BANK1_BASE`, the header's `FLASH_BASE`.
    pub vector_table: u32,
    pub image: &'a aethelgard_twin::boot::Image,
    /// The firmware's executable ranges, where the core must be at the end.
    pub executable: &'a [(u64, u64)],
    /// Every `yield; b.n` self-loop in the firmware's loaded bytes
    /// ([`aethelgard_twin::boot::self_loops`]): the core must end on one.
    pub self_loops: &'a [u64],
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
        for (i, r) in self.image.registers.iter().enumerate() {
            reads.push((
                format!("sysbus ReadDoubleWord {:#010x}", r.address),
                Read::Register(i),
            ));
        }
        for (i, c) in self.image.i2c.checks.iter().enumerate() {
            for (at, word) in [(c.status_at, 0), (c.value_at, 1)] {
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

    /// The cold RCC writes, as the part's reset leaves those registers.
    fn cold(&self) -> Vec<String> {
        self.image
            .cold
            .iter()
            .map(|r| format!("sysbus WriteDoubleWord {:#010x} 0x0", r.address))
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
        for check in &self.image.i2c.checks {
            for at in [check.status_at, check.value_at] {
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
        if let Some(check) = self.image.i2c.checks.first() {
            for at in [check.status_at, check.value_at] {
                c.push(format!("sysbus ReadDoubleWord {at:#010x}"));
            }
        }
        c
    }

    /// Each modelled device's log at `Noisy`, set once before anything runs:
    /// a source's level, not the console's (`AET-WO-2204` D1, D4).
    fn witness_levels(&self) -> Vec<String> {
        self.image
            .i2c
            .checks
            .iter()
            .map(|c| format!("logLevel -1 sysbus.{}", c.device))
            .collect()
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
    /// [`CONFIG`]'s synchronous logging, `BEGIN` is the boundary between the
    /// run's log and the judge's (`AET-WO-2204` D4).
    pub fn commands(&self) -> String {
        let mut c = vec![
            format!("emulation SetSeed {}", aethelgard_twin::boot::SEED),
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
    Register(usize),
    /// Check `.0`'s record: word 0 is the status, word 1 the value.
    Record(usize, u8),
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
    pub registers: Vec<u32>,
    /// Each identity check's (status, value) words.
    pub records: Vec<(u32, u32)>,
    /// Each identity check's witness: every line Renode logged for its
    /// device with a witness's shape, in order ([`witness_shaped`]); `None`
    /// when the log could not be read whole (`AET-WO-2204` D2).
    pub witnesses: Vec<Option<Vec<String>>>,
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

/// The text of a whole Renode log line after its host timestamp
/// (`[21:06:58.1004] `), exactly: nothing trimmed. `None` for any other line.
fn log_body(line: &str) -> Option<&str> {
    let b = line.as_bytes();
    let digits = |r: std::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
    let stamped = b.len() >= 16
        && b[0] == b'['
        && digits(1..3)
        && b[3] == b':'
        && digits(4..6)
        && b[6] == b':'
        && digits(7..9)
        && b[9] == b'.'
        && digits(10..14)
        && b[14] == b']'
        && b[15] == b' ';
    stamped.then(|| &line[16..])
}

/// Renode's log levels, as a line names them after its timestamp.
const LEVELS: [&str; 5] = ["[NOISY] ", "[DEBUG] ", "[INFO] ", "[WARNING] ", "[ERROR] "];

/// A whole Renode log line: a host timestamp, then a level.
fn is_log_line(line: &str) -> bool {
    log_body(line).is_some_and(|b| LEVELS.iter().any(|l| b.starts_with(l)))
}

/// A log line's text, if it has a witness's shape for `device`:
/// `[NOISY] i2c1.u2: …`. The shape only ever fails a witness: a pass needs
/// the whole text equal to an expected line (`AET-WO-2204` D1).
fn witness_shaped<'l>(line: &'l str, device: &str) -> Option<&'l str> {
    log_body(line).filter(|b| {
        b.strip_prefix("[NOISY] ")
            .and_then(|r| r.strip_prefix(device))
            .is_some_and(|r| r.starts_with(": "))
    })
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
        match (0..lines.len()).filter(|&i| lines[i] == want).collect::<Vec<_>>()[..] {
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
        return Err(untrusted("renode printed the end marker before the begin marker".into()));
    }
    // The droop's sentinels all come before the reads; the probes' own
    // blocks hold the judge's reads, not the run's log (F3).
    let released = plan.resets.iter().filter(|r| r.released_ns.is_some()).count();
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
                line.escape_debug().to_string().chars().take(120).collect::<String>()
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
    let checks = &plan.image.i2c.checks;
    // Past the filter's budget a log line may have been dropped, so no
    // witness can be read whole: every one is unwitnessed, never passed
    // (`AET-WO-2204` D2, ruled).
    let witnesses: Vec<Option<Vec<String>>> = checks
        .iter()
        .map(|c| {
            (out.dropped_log_lines == 0).then(|| {
                last_boot
                    .iter()
                    .filter_map(|l| witness_shaped(l, &c.device))
                    .map(str::to_owned)
                    .collect()
            })
        })
        .collect();
    let mut observed = Observed {
        boot_word: 0,
        instructions: 0,
        mips: 0,
        pc: 0,
        icsr: 0,
        registers: vec![0; plan.image.registers.len()],
        records: vec![(0, 0); checks.len()],
        witnesses,
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
            Read::Register(i) => observed.registers[i] = word()?,
            Read::Record(i, 0) => observed.records[i].0 = word()?,
            Read::Record(i, _) => observed.records[i].1 = word()?,
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
    if let Some(first) = run_log.iter().find(|l| LOCKED.iter().any(|w| l.contains(w))) {
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
            "pc_outside_firmware",
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
        let on_loop = plan.self_loops.iter().any(|&a| pc == a || pc == a + 2);
        if inside && !on_loop {
            findings.push(Finding::plain(
                "pc_not_on_self_loop",
                format!(
                    "the core ended at {:#010x}, which is not on a `yield; b.n` self-loop of the firmware",
                    observed.pc
                ),
            ));
        }
    }
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
            findings.extend(i2c_finding(c, status, value));
        }
        findings.extend(witness_findings(checks, &observed, &last_boot));
    }
    for d in &plan.image.i2c.unmodelled {
        findings.push(Finding::info(
            "device_unmodelled",
            format!(
                "{} ({}) at I2C address {:#04x} has no Renode {} model, so its identity was not read",
                d.reference,
                d.part,
                d.address,
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

/// Each record that claims its read completed, against what Renode logged
/// for its device (`AET-WO-2204` D1): exactly the model's expected lines, as
/// whole lines, in order, and across the devices in the records' order. A
/// record that says a wait expired claims nothing.
fn witness_findings(
    checks: &[aethelgard_twin::boot::I2cCheck],
    observed: &Observed,
    log: &[&str],
) -> Vec<Finding> {
    let claimed: Vec<usize> = (0..checks.len())
        .filter(|&i| observed.records[i].0 == aethelgard_twin::boot::STATUS_OK)
        .collect();
    let each: Vec<Finding> = claimed
        .iter()
        .filter_map(|&i| witness_finding(&checks[i], observed.witnesses[i].as_deref()))
        .collect();
    if !each.is_empty() {
        return each;
    }
    // Each device's own lines are right; the devices' order must be too.
    let devices: Vec<&str> = claimed.iter().map(|&i| checks[i].device.as_str()).collect();
    let seen: Vec<&str> = log
        .iter()
        .filter_map(|l| devices.iter().find_map(|d| witness_shaped(l, d)))
        .collect();
    let want: Vec<&str> = claimed
        .iter()
        .flat_map(|&i| checks[i].witness.iter().flatten().map(String::as_str))
        .collect();
    if seen == want {
        return Vec::new();
    }
    vec![Finding::plain(
        "i2c_transaction_unwitnessed",
        format!(
            "the devices' witness lines are not in the records' order: Renode logged {seen:?}, the records need {want:?}"
        ),
    )]
}

/// The finding for one record that claims its read completed, or none when
/// Renode logged exactly the model's lines for it.
fn witness_finding(
    c: &aethelgard_twin::boot::I2cCheck,
    seen: Option<&[String]>,
) -> Option<Finding> {
    let who = format!("{} at I2C address {:#04x}", c.reference, c.address);
    let Some(want) = &c.witness else {
        return Some(Finding::plain(
            "i2c_witness_unavailable",
            format!(
                "{who}: the twin knows no witness format for its model, so no read of it can be witnessed"
            ),
        ));
    };
    match seen {
        Some(seen) if seen == want.as_slice() => None,
        Some(seen) => Some(Finding::plain(
            "i2c_transaction_unwitnessed",
            format!(
                "{who}: the record says the read completed, but Renode logged {seen:?} for `{}`, not {want:?}",
                c.device
            ),
        )),
        None => Some(Finding::plain(
            "i2c_transaction_unwitnessed",
            format!(
                "{who}: the record says the read completed, but the log passed the filter's budget, so `{}`'s witness cannot be read whole",
                c.device
            ),
        )),
    }
}

/// The finding for one identity record, or none when the read passed. The
/// expectation is the check's, which comes from the part table, never from
/// the firmware (`INV-07`).
fn i2c_finding(c: &aethelgard_twin::boot::I2cCheck, status: u32, value: u32) -> Option<Finding> {
    use aethelgard_twin::boot::Outcome;
    let who = format!("{} at I2C address {:#04x}", c.reference, c.address);
    match c.judge(status, value) {
        Outcome::Pass => None,
        Outcome::Mismatch => Some(Finding::plain(
            "i2c_identity_mismatch",
            format!(
                "{who}: register {:#04x} read {value:#x}, the part table says {:#x}",
                c.register, c.expected
            ),
        )),
        Outcome::Timeout(stage) => Some(Finding::plain(
            "i2c_timeout",
            format!(
                "{who}: the firmware's wait for {stage} expired reading register {:#04x} (expected {:#x})",
                c.register, c.expected
            ),
        )),
        Outcome::Invalid => Some(Finding::plain(
            "i2c_record_invalid",
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
    let released = sentinels(&lines, RELEASE, n).map_err(&untrusted)?;
    let begins = sentinels(&lines, PROBE_BEGIN, n).map_err(&untrusted)?;
    let ends = sentinels(&lines, PROBE_END, n).map_err(&untrusted)?;
    let reads = plan.probe_reads();
    let check = plan.image.i2c.checks.first();
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
                    untrusted(format!("probe {k}: `{}` is not a word", line.escape_debug()))
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
            lines[r + 1..b]
                .iter()
                .filter_map(|l| witness_shaped(l, &c.device))
                .map(str::to_owned)
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

/// How a record's status reads in the verdict.
pub fn status_name(status: u32) -> &'static str {
    use aethelgard_twin::boot::{STATUS_OK, STATUS_RXNE, STATUS_STOP, STATUS_TC, STATUS_TXIS};
    match status {
        STATUS_OK => "ok",
        STATUS_TXIS => "timeout_txis",
        STATUS_TC => "timeout_tc",
        STATUS_RXNE => "timeout_rxne",
        STATUS_STOP => "timeout_stopf",
        _ => "invalid",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image() -> aethelgard_twin::boot::Image {
        let topology: aethelgard_ir::SolvedTopology = serde_json::from_str(
            r#"{"schema_version":2,"specification_sha256":"acd60081f2d111b1b1ed0d974078fff45ccf2a967ec5f42317417f8ce68149d1","assignments":{"I2C_SCL":{"pin":"PB6","peripheral":"I2C1","signal":"I2C1_SCL","alternate_function":4},"I2C_SDA":{"pin":"PB7","peripheral":"I2C1","signal":"I2C1_SDA","alternate_function":4}},"gpio":{},"i2c_ceilings":{"SENSORS":400000},"i2c_speeds":{"SENSORS":400000},"analog_lines":{"I2C_SCL":{"bus":"SENSORS","pull_up_ohms":2200,"capacitance_femtofarads":100000,"ceiling_ns":300,"domain_millivolts":3300},"I2C_SDA":{"bus":"SENSORS","pull_up_ohms":2200,"capacitance_femtofarads":100000,"ceiling_ns":300,"domain_millivolts":3300}},"i2c_devices":{"U2":{"index":0,"part":"BME280","bus":"SENSORS","peripheral":"I2C1","address":118},"U3":{"index":1,"part":"TMP117","bus":"SENSORS","peripheral":"I2C1","address":72}}}"#,
        )
        .unwrap();
        aethelgard_twin::boot::expected(&topology).unwrap()
    }

    fn plan<'a>(
        image: &'a aethelgard_twin::boot::Image,
        resets: &'a [power::Reset],
    ) -> BootPlan<'a> {
        BootPlan {
            platform: std::path::Path::new("/w/platform.repl"),
            firmware: std::path::Path::new("/w/firmware.elf"),
            virtual_ns: 100_000_000,
            boot_word: 0x2000_0000,
            vector_table: 0x0800_0000,
            image,
            executable: &[],
            instances: &[],
            resets,
        }
    }

    fn bor3() -> Vec<power::Reset> {
        vec![power::Reset {
            asserted_ns: 1_014_430,
            released_ns: Some(1_227_980),
            cause: vec!["VBOR3".to_owned()],
        }]
    }

    #[test]
    fn a_log_line_loses_its_host_timestamp_and_nothing_else() {
        assert_eq!(
            log_text("[21:06:58.1004] [WARNING] sysbus: Tried to write a locked peripheral"),
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
        assert!(static_.contains("emulation RunFor \"0.100000000\"; logLevel 3"));
        assert!(!static_.contains("machine Reset"));
        let rec = |at: u32| format!("sysbus WriteDoubleWord {at:#010x} 0xffffffff");
        let check = &image.i2c.checks[0];
        let want = [
            "cpu VectorTableOffset 0x08000000".to_owned(),
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
            "emulation RunFor \"0.000000010\"".to_owned(),
            "emulation RunFor \"0.000099990\"".to_owned(),
            "echo \"AETHELGARD-PROBE-BEGIN-0\"".to_owned(),
        ]
        .join("; ");
        assert!(droop.contains(&rest), "{droop}");
        assert!(droop.contains(
            "echo \"AETHELGARD-PROBE-END-0\"; emulation RunFor \"0.098672020\"; logLevel 3"
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
        let block = |rows: &[&str]| {
            format!(
                "x\nAETHELGARD-PROBE-BEGIN-0\n{}\nAETHELGARD-PROBE-END-0\n",
                rows.join("\n")
            )
        };
        let good = [
            "0x2100B007",
            "0x22020000",
            "0x00000060",
            "0x020104EC",
            "0x000000D0",
        ];
        let want = [Probe {
            at_ns: 1_327_980,
            boot_word: 0x2100_B007,
            record: Some((0x2202_0000, 0x60)),
            controller: Some((0x0201_04EC, 0xD0)),
        }];
        assert_eq!(probes_in(&block(&good), &p).unwrap(), want);
        // A whole Renode log line inside the block is the log, not a read
        // (the console keeps its level through a probe, adversarial F2).
        let mut logged = good.to_vec();
        logged.insert(
            2,
            "[21:06:58.1004] [WARNING] sysbus: Tried to write a locked peripheral",
        );
        assert_eq!(probes_in(&block(&logged), &p).unwrap(), want);
        let refused = [
            String::new(),
            block(&good[..4]),
            block(&[
                "0x2100B007",
                "0x22020000",
                "0x60 0x61",
                "0x020104EC",
                "0x000000D0",
            ]),
            // An echoed command's tail read as a value (`AET-WO-2301` F8).
            block(&[
                "0x2100B007",
                "0x22020000",
                "0x20000008\"",
                "0x020104EC",
                "0x000000D0",
            ]),
            block(&[
                "0x2100B007",
                "0x22020000",
                " [WARNING] 0x60",
                "0x020104EC",
                "0x000000D0",
            ]),
            format!("{}AETHELGARD-PROBE-BEGIN-1\n", block(&good)),
            "AETHELGARD-PROBE-END-0\n0x2100B007\nAETHELGARD-PROBE-BEGIN-0\n".to_owned(),
            block(&[
                "0x1FFFFFFFF",
                "0x22020000",
                "0x00000060",
                "0x020104EC",
                "0x000000D0",
            ]),
        ];
        for r in refused {
            assert!(probes_in(&r, &p).is_err(), "accepted: {r:?}");
        }
    }
}
