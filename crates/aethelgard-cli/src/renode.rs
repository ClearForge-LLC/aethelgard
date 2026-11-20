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

/// The foreign-firmware judge (`AET-WO-4400e`).
pub mod foreign;

const TOOL: &str = "renode";
const LOAD_ERROR: &str = "There was an error executing command";
/// Renode's error log level; a CPU listed beside one is not a clean load.
const LOG_ERROR: &str = "[ERROR]";
/// What Renode's monitor prints when an included model fails to compile or
/// load (`AET-WO-4220` spike F2, measured): untimestamped, on stdout, then the
/// diagnostics. The `-e` line stops there and Renode still exits 0, so a
/// load is judged by this line, never assumed (ruling D6).
const MODEL_LOAD_ERROR: &str = "Errors during compilation or loading:";

/// The monitor command that compiles one model of our own before the platform
/// that declares it is loaded (`AET-WO-4220` D6): the CLI's own sealed copy.
fn include(model: &std::path::Path) -> String {
    format!("include @{}", model.display())
}

/// `ModelLoadFailed` (`AET-WO-4220` D6) when Renode said a model of ours did
/// not compile or load: the line and the diagnostics after it, a few at most.
fn model_load_failed(lines: &[&str]) -> Option<Stop> {
    let i = lines.iter().position(|l| l.contains(MODEL_LOAD_ERROR))?;
    let said: Vec<&str> = lines[i..]
        .iter()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .take(6)
        .collect();
    Some(Stop::refused(
        "ModelLoadFailed",
        format!(
            "Renode did not compile or load a C# file (a model of our own, or one the platform included): {}",
            said.join(" | ")
        ),
    ))
}

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
/// path this CLI chose, so no part of the caller's input reaches the monitor;
/// so is each model of our own it declares, included first (`AET-WO-4220`).
pub fn commands(copy: &std::path::Path, models: &[std::path::PathBuf]) -> String {
    let mut c: Vec<String> = models.iter().map(|m| include(m)).collect();
    c.extend([
        "mach create".to_owned(),
        format!("machine LoadPlatformDescription @{}", copy.display()),
        "peripherals".to_owned(),
        "quit".to_owned(),
    ]);
    c.join("; ")
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
    if let Some(stop) = model_load_failed(&lines) {
        return Err(stop);
    }
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
/// Renode's access log of an access the judge never reads (a read, a write
/// to a GPIO word other than `MODER` and `AFR`) is discarded, uncounted,
/// wherever it falls (`AET-WO-4300b` fix 1): a port polling `ISR` until
/// `ECHO_POLLS` runs out writes 20 000 such lines, 2.3 MB (measured, P2; the
/// four-port fixture with no frame, 9.3 MB), and a firmware toggling a pin
/// of a logged port in its loop one per toggle.
pub const LOG_FILTER: LogFilter = LogFilter {
    budget: 8 * 1024 * 1024,
    keep: &[
        ABORTED,
        LOAD_ERROR,
        LOG_ERROR,
        LOCKED[0],
        LOCKED[1],
        MODEL_LOAD_ERROR,
    ],
    discard: unjudged_access_line,
    summarise: crate::run::summarise_none,
};

/// A whole log line of Renode's access log the judge never reads.
fn unjudged_access_line(line: &str) -> bool {
    log_body(line).is_some_and(aethelgard_twin::serial::is_unjudged_access)
}

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

/// The flash controller's access log, on for every boot before anything
/// runs (`AET-WO-4230` D10): its one read of `FLASH_OPTSR_CUR` and that it
/// never writes are judged from it. Logging is off without it (measured,
/// C1 m2), and the command answers with [`FLASH_LOG_ON`].
const FLASH_LOG: &str = "sysbus LogPeripheralAccess sysbus.flashController true";
/// What Renode 1.17.0 answers [`FLASH_LOG`] with, once, at the load
/// (measured, `AET-WO-4230` FEEDBACK §2 m0 and C1 m2: the CPU's accesses are
/// logged after it all the same): the judge's own line, never an access.
const FLASH_LOG_ON: &str = "[INFO] flashController: Logging is already enabled";
/// The warning Renode logs when a gated peripheral is touched (spike F10).
const LOCKED: [&str; 2] = [
    "Tried to write a locked peripheral",
    "Tried to read a locked peripheral",
];

/// One boot: the copies Renode reads, how long it runs, and what the judge
/// computed from the topology (`INV-07`).
pub struct BootPlan<'a> {
    pub platform: &'a std::path::Path,
    /// The CLI's sealed copy of each model of our own the platform declares,
    /// included before it loads (`AET-WO-4220` D6).
    pub models: &'a [std::path::PathBuf],
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
    /// Each motor output's observer, solved values and static check
    /// (`AET-WO-4300a`), in the specification's order.
    pub motors: &'a [Motor],
    /// Every solved peripheral as the listing must name it: (name, model).
    pub instances: &'a [(String, String)],
    /// The resets a declared droop renders, in time order; empty for every
    /// other boot (`AET-WO-2301`).
    pub resets: &'a [power::Reset],
    /// The droop's recovery bound, from the same boot run first with no
    /// reset (`AET-WO-2305` §1.2, kind 32): each release is probed at it
    /// ([`power::probe_ns`]). `None` for every other boot, and for a droop
    /// whose undisturbed boot wrote no boot word, which reads no probe.
    pub recovery_bound_ns: Option<u64>,
    /// Each serial port's expectations (`AET-WO-4300b`), in the
    /// specification's order; empty for a board without one, whose commands
    /// are then exactly what they were before.
    pub serial: &'a [Serial],
    /// The boot's fresh capture directory, which the CLI made and Renode may
    /// add to: one file backend per port. `None` without a serial port.
    pub capture: Option<&'a std::path::Path>,
}

/// The sentinels around the read after a release (`AET-WO-2301` §2.3), one
/// pair per release, numbered from 0; each is matched as a whole line.
const PROBE_BEGIN: &str = "AETHELGARD-PROBE-BEGIN-";
const PROBE_END: &str = "AETHELGARD-PROBE-END-";
/// The sentinel at each release, numbered as the probes are: the log lines
/// from it to the probe's `BEGIN` are the recovered boot's (`AET-WO-2204`).
const RELEASE: &str = "AETHELGARD-RELEASE-";

/// `emulation RunFor` for exactly `ns`. Renode 1.17.0 reads the decimals as a
/// fraction of a second through a double and truncates
/// (`TimeInterval.TryParse`: `(ulong)(double.Parse("0.<decimals>") *
/// TicksPerSecond)`, a tick a nanosecond), so `"0.001018380"` ran 1,018,379
/// ns: about 2 % of 10 ns durations ran 1 ns short, and every instant after
/// one was 1 ns early, a droop's boot start among them (`AET-WO-4202`,
/// measured). A tenth decimal of 5 puts the value half a nanosecond above
/// `ns`; the double's error below a second is under 1e-6 ns, so the
/// truncation lands on `ns`.
fn run_for(ns: u64) -> String {
    format!(
        "emulation RunFor \"{}.{:09}5\"",
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

    /// The start of the boot the judge reads: 0, or the last rendered
    /// release plus its nudge (`AET-WO-2301` F5; measured again in
    /// `AET-WO-2302`: a hook's count restarts there). `None` when the last
    /// reset is never released: no boot follows it.
    pub fn last_start(&self) -> Option<u64> {
        match self.resets.last() {
            None => Some(0),
            Some(power::Reset {
                released_ns: Some(at),
                ..
            }) => Some(at + power::NUDGE_NS),
            Some(_) => None,
        }
    }

    /// The earliest instant the serial frames can go in (`AET-WO-4300b`,
    /// `AET-WO-2302` D4): one quantum after the last boot's start, since its
    /// boot word comes no earlier. The frames go in one quantum after the
    /// boot word itself, which only the run shows; `main.rs` refuses before
    /// the boot only what no boot word can save. `None` without a serial
    /// port, or without a last boot.
    pub fn injection_ns(&self) -> Option<u64> {
        if self.serial.is_empty() {
            return None;
        }
        self.last_start().map(aethelgard_twin::serial::injection_ns)
    }

    /// Each motor timer as the composition hooks it (`AET-WO-2302`), in
    /// ascending number: its `CR1` and solved registers from the image
    /// (`INV-07`), and whether it is DShot. A timer
    /// whose `CR1` the image does not hold is not hooked, and its outputs
    /// fail closed on an anchor never observed.
    pub fn timer_hooks(&self) -> Vec<crate::anchor::TimerHook> {
        let mut timers: Vec<u32> = self.motors.iter().map(|m| m.observer.timer).collect();
        timers.sort_unstable();
        timers.dedup();
        timers
            .into_iter()
            .filter_map(|n| {
                let prefix = format!("TIM{n}.");
                let cr1 = self
                    .image
                    .registers
                    .iter()
                    .find(|r| r.name == format!("TIM{n}.CR1"))?
                    .address;
                Some(crate::anchor::TimerHook {
                    timer: n,
                    cr1,
                    registers: self
                        .image
                        .registers
                        .iter()
                        .filter(|r| r.name.starts_with(&prefix))
                        .map(|r| r.address)
                        .collect(),
                    dshot: self
                        .motors
                        .iter()
                        .any(|m| m.observer.timer == n && m.observer.protocol.bit_rate().is_some()),
                })
            })
            .collect()
    }

    /// Each serial port as the composition injects it, in the
    /// specification's order; none when a port's spacing cannot be computed
    /// (`main.rs` refuses such a board before the boot).
    pub fn port_hooks(&self) -> Vec<crate::anchor::PortHook> {
        self.serial
            .iter()
            .map(|p| {
                Some(crate::anchor::PortHook {
                    renode: p.port.renode,
                    tdr: p.tdr_at,
                    rx: (p.rx.moder.1, p.rx.afr.1),
                    frame: p.frame,
                    spacing_ns: p.spacing_ns?,
                    open: p.open,
                })
            })
            .collect::<Option<Vec<_>>>()
            .unwrap_or_default()
    }

    /// The hooks on the boot word and on each port's `TDR` but the last
    /// (`AET-WO-2302` D4), added once, before the boot the judge reads: a
    /// hook on RAM is not fired again after `machine Reset`, and added again
    /// it fires twice (measured, build probes P3 and P4).
    fn boot_hooks(&self) -> Vec<String> {
        let ports = self.port_hooks();
        let mut c = vec![crate::anchor::boot_word_hook(self.boot_word, &ports)];
        c.extend(
            (0..ports.len()).filter_map(|k| crate::anchor::echo_hook(k, &ports, self.boot_word)),
        );
        // Each I2C step's status word (`AET-WO-3305`): where its step ends.
        c.extend(
            self.image
                .identity
                .checks
                .iter()
                .enumerate()
                .filter(|(_, s)| matches!(s.wire, aethelgard_twin::boot::Wire::I2c { .. }))
                .map(|(k, s)| crate::anchor::step_hook(k, s.status_at)),
        );
        c
    }

    /// Before the first `RunFor`, with a serial port: Renode's access log on
    /// each port's USART, each of its pins' GPIO ports and the RCC
    /// (adversarial F1–F4, pass 2 A1, A2; measured, fix 1 P1, P2: one line per
    /// access, kept through `machine Reset`), then one file backend per stock
    /// USART and UART in the
    /// capture directory, the ports' last and in their order: Renode opens
    /// each file new (`UartFileBackend.cs`) and writes each byte the USART
    /// transmits, flushed (`true`). A byte on a USART no port names is
    /// forged (F12).
    fn capture_commands(&self) -> Vec<String> {
        let Some(dir) = self.capture else {
            return Vec::new();
        };
        let mut c: Vec<String> = Vec::new();
        for source in self.logged_sources() {
            c.push(format!("sysbus LogPeripheralAccess sysbus.{source} true"));
        }
        let ports: Vec<&str> = self.serial.iter().map(|p| p.port.renode).collect();
        let others = aethelgard_twin::serial::stock_uarts()
            .into_iter()
            .filter(|u| !ports.contains(u));
        for n in others.chain(ports.iter().copied()) {
            c.push(format!(
                "sysbus.{n} CreateFileBackend @{}/{n}.bin true",
                dir.display()
            ));
        }
        c
    }

    /// Every peripheral whose accesses Renode logs: each port's USART, then
    /// each TX and RX pin's GPIO port, once each, in the ports' order, then
    /// the RCC, whose writes may reset or reclock any of them; none without a serial
    /// port.
    fn logged_sources(&self) -> Vec<&'static str> {
        let mut out: Vec<&'static str> = self.serial.iter().map(|p| p.port.renode).collect();
        for p in self.serial {
            for source in [p.tx.source, p.rx.source] {
                if !out.contains(&source) {
                    out.push(source);
                }
            }
        }
        if !out.is_empty() {
            out.push(aethelgard_twin::serial::RCC_RENODE);
        }
        out
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
        reads.push((
            format!(
                "sysbus ReadDoubleWord {:#010x}",
                self.image.brown_out.word_at
            ),
            Read::BrownOut,
        ));
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
    ///
    /// With a serial port, the RCC's access log is off around the cold
    /// writes: they are the judge's, and a logged write with no `[cpu: …]`
    /// is never rebuilt (`AET-WO-4300b` adversarial pass 2, A1).
    fn assert_reset(&self) -> Vec<String> {
        let mut c = vec!["machine Reset".to_owned()];
        let rcc = self
            .logged_sources()
            .contains(&aethelgard_twin::serial::RCC_RENODE)
            .then(|| {
                let at = |on| {
                    format!(
                        "sysbus LogPeripheralAccess sysbus.{} {on}",
                        aethelgard_twin::serial::RCC_RENODE
                    )
                };
                (at(false), at(true))
            });
        c.extend(rcc.as_ref().map(|r| r.0.clone()));
        c.extend(self.cold());
        c.extend(rcc.map(|r| r.1));
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
        let mut c = vec![
            format!("sysbus WriteDoubleWord {:#010x} 0x0", self.boot_word),
            format!(
                "sysbus WriteDoubleWord {:#010x} {:#010x}",
                self.image.brown_out.word_at,
                aethelgard_twin::boot::RECORD_UNSET
            ),
        ];
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
        // Each motor output's observer at `Noisy` (`AET-WO-4300a`), never
        // changed after (`AET-WO-2302` adversarial pass 1, H1).
        c.extend(
            self.motors
                .iter()
                .map(|m| format!("logLevel -1 sysbus.{}", m.observer.source)),
        );
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
    /// the nudge, and probed at the recovery bound (ruled D1, D4). The boot
    /// word's and the ports' hooks go in at the last reset's release, before
    /// the core runs again: that boot is the one the judge reads, and the
    /// only one injected, as before (`AET-WO-2302`).
    fn droop_schedule(&self) -> Vec<String> {
        let mut c = Vec::new();
        let mut now = 0u64;
        let mut at = |c: &mut Vec<String>, t: u64| {
            if t > now {
                c.push(run_for(t - now));
                now = t;
            }
        };
        let last = self.resets.len().saturating_sub(1);
        for (k, r) in self.resets.iter().enumerate() {
            at(&mut c, r.asserted_ns);
            c.extend(self.assert_reset());
            let Some(release) = r.released_ns else { break };
            at(&mut c, release);
            if k == last {
                c.extend(self.boot_hooks());
            }
            c.push("cpu IsHalted false".to_owned());
            c.push(format!("echo \"{RELEASE}{k}\""));
            // The nudge (F5): the release takes effect at its end.
            at(&mut c, release + power::NUDGE_NS);
            // No level is changed anywhere (`AET-WO-2204` D4). With no
            // bound, nothing is probed (`AET-WO-2305` §1.2).
            if let Some(probe) = self.probe_at(release) {
                at(&mut c, probe);
                c.push(format!("echo \"{PROBE_BEGIN}{k}\""));
                c.extend(self.probe_reads());
                c.push(format!("echo \"{PROBE_END}{k}\""));
            }
        }
        at(&mut c, self.virtual_ns);
        c
    }

    /// Where a release's probe reads the machine: the release, its nudge and
    /// the recovery bound (`AET-WO-2305` §1.2); `None` with no bound.
    pub fn probe_at(&self, release_ns: u64) -> Option<u64> {
        self.recovery_bound_ns
            .and_then(|b| power::probe_ns(release_ns, b))
    }

    /// The number of probes the boot reads: one per release, or none with
    /// no bound.
    fn probe_count(&self) -> usize {
        match self.recovery_bound_ns {
            Some(_) => self
                .resets
                .iter()
                .filter(|r| r.released_ns.is_some())
                .count(),
            None => 0,
        }
    }

    /// The monitor commands: a fixed seed, the platform, the firmware, the
    /// cold RCC, each modelled device's log level, each motor timer's hook
    /// (`AET-WO-2302`), a fixed virtual time, then the reads between
    /// sentinels, then the listing. Both paths are this CLI's own copies.
    /// Without a droop, the boot word's and the ports' hooks go in before
    /// the run; with one, the fixed virtual time is the droop's schedule,
    /// which adds them at the last release. The console's level is never
    /// changed: with [`config`]'s synchronous logging, `BEGIN` is the
    /// boundary between the run's log and the judge's (`AET-WO-2204` D4).
    pub fn commands(&self) -> String {
        let mut c = vec![
            format!("emulation SetSeed {}", aethelgard_twin::boot::SEED),
            QUANTUM.to_owned(),
        ];
        c.extend(self.models.iter().map(|m| include(m)));
        c.extend([
            "mach create".to_owned(),
            format!(
                "machine LoadPlatformDescription @{}",
                self.platform.display()
            ),
        ]);
        c.extend([
            format!("sysbus LoadELF @{}", self.firmware.display()),
            FLASH_LOG.to_owned(),
        ]);
        c.extend(self.cold());
        c.extend(self.rearm());
        c.push(self.vtor());
        c.extend(self.witness_levels());
        c.extend(self.capture_commands());
        c.extend(self.stimuli());
        c.extend(
            self.timer_hooks()
                .iter()
                .flat_map(crate::anchor::timer_hooks),
        );
        if self.resets.is_empty() {
            c.extend(self.boot_hooks());
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
    /// The boot record's word 1, `FLASH_OPTSR_CUR` as the firmware read it.
    BrownOut,
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
    /// The boot record's word 1 (`AET-WO-4230` D10).
    pub brown_out: u32,
    /// What the flash controller's access log of each boot shows that the
    /// bring-up never does ([`aethelgard_twin::boot::BrownOut::judge_accesses`]),
    /// empty when nothing; `None` when the log could not be read whole.
    pub flash_access: Option<Vec<String>>,
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
    /// For each I2C check, the addresses its bus's controller found no device
    /// at during that check's own step ([`absent_by_step`], `AET-WO-3305`;
    /// Renode's `Unknown slave at address N.`), in order: attribution's
    /// evidence of a transfer to a wrong address, never a witness
    /// (`AET-WO-2204`, the seeded set's `wrong-address`). Empty for an SPI
    /// check, and for every check when the log could not be read whole.
    pub absent: Vec<Vec<u8>>,
    /// For each motor output, what its observer showed (`AET-WO-4300a`).
    pub motors: Vec<MotorWitness>,
    /// For each serial port, what its capture, pins and tags showed
    /// (`AET-WO-4300b`).
    pub serial: Vec<SerialWitness>,
    /// For each step check on SPI, every line its controller's multiplexer
    /// logged when it routed a byte to no device, to several, or to an
    /// address with none (`SPIMultiplexer.cs` L56, L62, L69), whole and in
    /// order, inside the step's chip-select window; a line inside no window
    /// goes to every device's identity step (`AET-WO-4210` D5, `4200` F16):
    /// attribution's evidence, never a witness (`AET-WO-4200` D4). Empty for
    /// an I2C check.
    pub unrouted: Vec<Vec<String>>,
    /// The instant of the boot word's write the hook observed in the boot
    /// the judge reads (`AET-WO-2302`): the serial frames' anchor.
    pub boot_word_ns: Option<u64>,
    /// What the judge expects each image register to hold at the end: the
    /// image's value, but `CEN` cleared in a DShot timer's `CR1` whose stop
    /// ran, the judge's own act (`AET-WO-2302` D2).
    pub register_expected: Vec<u32>,
    /// Each hooked timer's anchor and stop (`AET-WO-2302`).
    pub timers: Vec<TimerWitness>,
    /// Each port's injection (`AET-WO-2302` D4).
    pub injections: Vec<PortInjection>,
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
    // Each device's steps, in its script's order: not consecutive, since
    // every data read follows every other device's identity and
    // configuration (`AET-WO-4230` R6).
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for (i, c) in checks.iter().enumerate() {
        match groups
            .iter_mut()
            .find(|g| checks[g[0]].reference == c.reference)
        {
            Some(g) => g.push(i),
            None => groups.push(vec![i]),
        }
    }
    let mut every_window: std::collections::BTreeMap<&str, Vec<(usize, usize)>> =
        Default::default();
    for g in &groups {
        let c = &checks[g[0]];
        let last = g[g.len() - 1];
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
            Some((address, w)) => {
                // A model that logs a transaction when it ends (ours,
                // `AET-WO-4220`) logs it when the multiplexer finishes it:
                // after the closing edge `GPIO #a set to True`, directly
                // after `Finisihing transmission on device 0xa`, in that
                // edge's burst, before the multiplexer's next edge
                // (`SPIMultiplexer.cs` L30-32, L99-108, L111-129). The
                // release ends the transaction, so such a line is its
                // window's. Attribution only: it never passes a witness.
                let finish = format!(
                    "[NOISY] {}: Finisihing transmission on device 0x{address:X}",
                    mux_of(c)
                );
                let edge = format!("[NOISY] {}: GPIO #", mux_of(c));
                let closing: Vec<(usize, usize)> = w
                    .iter()
                    .map(|&(_, e)| {
                        let next = (e + 1..log.len())
                            .find(|&j| log_body(log[j]).is_some_and(|b| b.starts_with(&edge)))
                            .unwrap_or(log.len());
                        (e, next)
                    })
                    .collect();
                let step_of = |i: usize| {
                    w.iter()
                        .position(|&(o, e)| o < i && i < e)
                        .or_else(|| {
                            let finished = i > 0 && log_body(log[i - 1]) == Some(finish.as_str());
                            closing
                                .iter()
                                .position(|&(e, next)| finished && e < i && i < next)
                        })
                        .map_or(last, |k| g.get(k).copied().unwrap_or(last))
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
                for &k in g {
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
                        .map_or(g[0], |&(_, k)| k);
                    seen[k].push(body.to_owned());
                }
            }
        }
    }
    // A multiplexer line inside no device's window is the controller's: it
    // goes to every device's identity step, as `4200` D4 collected it.
    for g in &groups {
        let c = &checks[g[0]];
        if let aethelgard_twin::boot::Wire::Spi { .. } = c.wire {
            let all = every_window.get(mux_of(c)).map_or(&[][..], Vec::as_slice);
            for (i, line) in log.iter().enumerate() {
                if let Some(u) = unrouted_line(line, mux_of(c))
                    && !all.iter().any(|&(o, e)| o < i && i < e)
                {
                    unrouted[g[0]].push(u.to_owned());
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

/// One serial port as a boot judges it (`AET-WO-4300b` §1.5): what the twin
/// expects from the topology and the firmware's own sha256 (`INV-07`), and
/// the static check for silicon (`aethelgard_ir::serial::check_silicon`),
/// kept apart from the twin's judgement (`4300` D3, D8).
#[derive(Debug, Clone)]
pub struct Serial {
    pub port: aethelgard_twin::serial::Port,
    pub role: aethelgard_ir::SerialRole,
    /// The bytes the port must send at each boot's start.
    pub pattern: Vec<u8>,
    /// The bytes injected from [`BootPlan::injection_ns`], and echoed back
    /// by a firmware that reads them.
    pub frame: [u8; aethelgard_twin::serial::FRAME_LEN],
    /// How far apart the frame's bytes go in
    /// ([`aethelgard_twin::serial::spacing_ns`]); `None` where the solved
    /// baud gives no time.
    pub spacing_ns: Option<u64>,
    pub tx: aethelgard_twin::serial::PinMux,
    pub rx: aethelgard_twin::serial::PinMux,
    pub silicon: aethelgard_ir::serial::SiliconFraming,
    /// What the bring-up writes, from the topology and the header, against
    /// which the access log's writes are judged.
    pub bringup: aethelgard_twin::serial::Bringup,
    /// `TDR`'s address, from the header: the hook that counts the port's
    /// echo watches it (`AET-WO-2302` D4).
    pub tdr_at: u32,
    /// What the RCC's writes may be: its enable registers alone, never
    /// clearing the gate of the port's USART or of its pins' GPIO ports.
    pub rcc: aethelgard_twin::serial::RccRule,
    /// The scenario holds the port's RX line open (`AET-WO-2302` D5): no
    /// frame goes in, and the next port goes in after the port before it.
    pub open: bool,
}

/// Every serial port of `topology`, in the specification's order, for the
/// firmware whose file's sha256 is `firmware`; none held open (the boot
/// marks the ones its scenario holds open).
pub fn serials(
    topology: &aethelgard_ir::SolvedTopology,
    firmware: &[u8; 32],
) -> Result<Vec<Serial>, aethelgard_twin::TwinError> {
    let ports = aethelgard_twin::serial::ports(topology)?;
    ports
        .into_iter()
        .zip(topology.serial_ports_in_order())
        .map(|(port, (_, solved, _, _))| {
            let bringup = aethelgard_twin::serial::Bringup::of(&port.framing, solved.brr)?;
            let tdr_at = aethelgard_svd_hal::cmsis::register_facts()
                .and_then(|f| f.base(&port.peripheral))
                .map_err(aethelgard_twin::TwinError::from)?
                + bringup.tdr;
            Ok(Serial {
                role: solved.role,
                pattern: aethelgard_twin::serial::pattern(&port.label),
                frame: aethelgard_twin::serial::frame(firmware, &port.label),
                spacing_ns: aethelgard_twin::serial::spacing_ns(
                    &port.framing,
                    solved.actual_millibaud,
                ),
                tx: aethelgard_twin::serial::PinMux::of(&port.tx, port.tx_af)?,
                rx: aethelgard_twin::serial::PinMux::of(&port.rx, port.rx_af)?,
                silicon: aethelgard_ir::serial::check_silicon(&topology.clock, solved),
                bringup,
                tdr_at,
                rcc: aethelgard_twin::serial::rcc_rule(
                    topology,
                    &port.peripheral,
                    &[&port.tx, &port.rx],
                )?,
                port,
                open: false,
            })
        })
        .collect()
}

/// The capture directory as the CLI read it after the run: each regular
/// file's name and bytes, and what else was there.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Captured {
    pub files: Vec<(String, Vec<u8>)>,
    /// Each entry that is not a regular file of at most
    /// [`CAPTURE_MAX_BYTES`], or the directory itself unreadable, named.
    pub problems: Vec<String>,
}

/// The most bytes the CLI reads of one capture file: the patterns of every
/// boot a droop can render and the frame fit far inside it.
pub const CAPTURE_MAX_BYTES: u64 = 64 * 1024;

/// What one port's capture, pins and tags showed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialWitness {
    /// The capture file's bytes; `None` when Renode left none.
    pub bytes: Option<Vec<u8>>,
    pub capture: Option<aethelgard_twin::serial::Capture>,
    /// The first frame byte's instant, one quantum after the boot word for
    /// the first port and after the port before's last echoed byte for each
    /// later one (`AET-WO-2302` D4); `None` when no frame went in.
    pub injected_ns: Option<u64>,
    /// The frame's bytes that went in.
    pub injected: usize,
    /// The RX pin's `MODER` and `AFR` at the injection instant; `None` when
    /// no frame was injected.
    pub rx_at_injection: Option<(u32, u32)>,
    /// The TX pin's `MODER` and `AFR` at the end, from the image's reads.
    pub tx_at_end: Option<(u32, u32)>,
    /// The TX pin's `MODER` and `AFR` at the first `TDR` write of each boot,
    /// rebuilt from the access log: the first boot's that is not routed,
    /// else the first that left the AF after it was solved
    /// ([`Self::tx_after_solved`]), else the last's; `None` when no
    /// boot wrote `TDR`, a boot's could not be rebuilt, or the log could not
    /// be read whole (adversarial F4; pass 4, ADV4-4).
    pub tx_at_send: Option<(u32, u32)>,
    /// What the interval rule saw (adversarial pass 4, ADV4-4): the TX
    /// pin's `MODER` and `AFR` at the first write that took it off its AF
    /// after it was solved, when that is [`Self::tx_at_send`] (every boot's
    /// first `TDR` write saw it routed; `None` inside where the rebuild could
    /// not say), and the RX pin's state at the first write that changed it
    /// after it was solved; each `None` when no boot did or the log could
    /// not be read whole.
    pub tx_after_solved: Option<Option<(u32, u32)>>,
    pub rx_after_solved: Option<String>,
    /// The writes judged against the bring-up; `None` when the log could not
    /// be read whole.
    pub tags: Option<aethelgard_twin::serial::TagJudgement>,
    /// `witnessed`, `missing`, `partial`, `not_counted` (the frame came back
    /// with the RX pin off its AF at the injection, or changed after it was
    /// solved: [`Self::rx_after_solved`]) or `not_judged` (no boot word).
    pub echo: &'static str,
}

impl SerialWitness {
    /// The pattern counts when there is at least one and the TX pin is on its
    /// AF at the first `TDR` write of each boot and at the end.
    fn pattern_out(&self, s: &Serial) -> bool {
        self.capture.as_ref().is_some_and(|c| c.patterns > 0)
            && self.tx_at_send.is_some_and(|(m, a)| s.tx.routed(m, a))
            && self.tx_at_end.is_some_and(|(m, a)| s.tx.routed(m, a))
    }
}

fn serial_who(s: &Serial) -> String {
    format!(
        "{} ({}, TX {}, RX {})",
        s.port.label, s.port.peripheral, s.port.tx, s.port.rx
    )
}

/// A port at which the wire-time budget passes the window: the port, its
/// character's ns, its own budget's and the ports' budgets' sum through it,
/// and where that sum ends (`None` where its baud gives no time).
pub type WirePast<'a> = (&'a Serial, Option<(u64, u64, u64)>, Option<u64>);

/// The first port, in the specification's order, at which the ports' wire
/// time no longer fits between the injection at `injection_ns` and the
/// board's window `window_ns` (the PM's rulings on adversarial F5 and on
/// pass 2's H1): the firmware sends the patterns port after port and echoes
/// port after port, so the budget is the sum, in the specification's order,
/// of each port's own ([`aethelgard_twin::serial::wire_ns`]: its pattern,
/// the frame as the composition paces it and one echoed character). The port, its character's ns, its
/// budget's and the sum through it, and where the sum ends. A port whose baud
/// gives no time is the same.
pub fn wire_past_window(
    serial: &[Serial],
    injection_ns: u64,
    window_ns: u64,
) -> Option<WirePast<'_>> {
    past_window(serial.iter(), injection_ns, window_ns, |s| {
        aethelgard_twin::serial::wire_ns(&s.port.label, &s.port.framing, s.silicon.actual_millibaud)
    })
}

/// A port's wire time from the boot word (`AET-WO-2302` adversarial pass 2,
/// A2-3): its frame as the composition injects it, [`FRAME_LEN`] bytes each
/// [`aethelgard_twin::serial::spacing_ns`] apart, and one echoed
/// character. Not its pattern: the generated bring-up sends every port's
/// pattern before it writes the boot word, so after the boot word only the
/// frames and their echoes are on the wire. `None` where the baud gives no
/// time.
///
/// [`FRAME_LEN`]: aethelgard_twin::serial::FRAME_LEN
pub fn frame_echo_ns(s: &Serial) -> Option<u64> {
    let millibaud = s.silicon.actual_millibaud;
    let frame = u64::try_from(aethelgard_twin::serial::FRAME_LEN).ok()?;
    aethelgard_twin::serial::spacing_ns(&s.port.framing, millibaud)?
        .checked_mul(frame)?
        .checked_add(aethelgard_twin::serial::char_ns(
            &s.port.framing,
            millibaud,
        )?)
}

/// The first of `ports`, in order, at which the sum of each one's
/// `budget` from `injection_ns` passes `window_ns` (or whose budget its baud
/// cannot give), as [`wire_past_window`] names it.
fn past_window<'a>(
    ports: impl Iterator<Item = &'a Serial>,
    injection_ns: u64,
    window_ns: u64,
    budget: impl Fn(&Serial) -> Option<u64>,
) -> Option<WirePast<'a>> {
    let mut sum = Some(0u64);
    ports.into_iter().find_map(|s| {
        let char_ns = aethelgard_twin::serial::char_ns(&s.port.framing, s.silicon.actual_millibaud);
        let wire = budget(s);
        sum = sum.zip(wire).and_then(|(a, w)| a.checked_add(w));
        let end = sum.and_then(|t| injection_ns.checked_add(t));
        let times = char_ns.zip(wire).zip(sum).map(|((c, w), t)| (c, w, t));
        (!end.is_some_and(|e| e <= window_ns)).then_some((s, times, end))
    })
}

/// The silicon check's findings for every serial port (`4300` D8): decided
/// from the topology alone, so a boot killed at a bound reports them too.
pub fn static_serial_findings(serial: &[Serial]) -> Vec<Finding> {
    serial
        .iter()
        .filter(|s| !s.silicon.holds)
        .map(|s| {
            let f = &s.silicon;
            Finding::plain(
                aethelgard_chaos::campaign::SERIAL_STATIC_FINDING,
                format!(
                    "{}: BRR {} on a {} Hz kernel clock gives {} mbaud, {} ppm from the declared {}; {}",
                    serial_who(s),
                    f.brr,
                    f.kernel_hz,
                    f.actual_millibaud,
                    f.error_ppm,
                    aethelgard_twin::serial::describe(&s.port.framing),
                    match f.tolerance {
                        Some(row) => format!(
                            "{} M = {:02b} tolerates under {} ppm ({}), and BRR must be the nearest whole USARTDIV",
                            row.table,
                            row.m_bits,
                            row.tolerance_ppm,
                            aethelgard_ir::serial::TOLERANCE_SOURCE
                        ),
                        None => "no row of the receiver's tolerance applies".to_owned(),
                    }
                ),
            )
        })
        .collect()
}

/// One port's witness findings, once the boot reached its fixed point.
/// `releases` is the boots the droop rendered after the first; `why` why
/// no whole frame went in, when one did not ([`PortInjection::why`]).
fn serial_findings_of(
    s: &Serial,
    w: &SerialWitness,
    releases: usize,
    why: Option<&str>,
) -> Vec<Finding> {
    use aethelgard_chaos::campaign::{
        SERIAL_ECHO_FINDING, SERIAL_FORGED_FINDING, SERIAL_PATTERN_FINDING, SERIAL_TAG_FINDING,
    };
    let who = serial_who(s);
    let mut out = Vec::new();
    match w.tags.as_ref() {
        None => out.push(Finding::plain(
            SERIAL_TAG_FINDING,
            format!(
                "{who}: Renode's log could not be read whole, so the writes to {} were not judged",
                s.port.renode
            ),
        )),
        Some(t) if !t.holds() => out.push(Finding::plain(
            SERIAL_TAG_FINDING,
            format!(
                "{who}: the framing {} needs {} set at UE; {} writes to {} in the last boot (Renode's access log): tags missing at UE {}, and not the bring-up: {}",
                aethelgard_twin::serial::describe(&s.port.framing),
                if s.port.tags.is_empty() {
                    "no tag set".to_owned()
                } else {
                    s.port
                        .tags
                        .iter()
                        .map(|t| format!("{}.{}", t.register, t.field))
                        .collect::<Vec<_>>()
                        .join(", ")
                },
                t.writes,
                s.port.renode,
                if t.missing.is_empty() {
                    "none".to_owned()
                } else {
                    t.missing.join(", ")
                },
                if t.unexpected.is_empty() {
                    "none".to_owned()
                } else {
                    t.unexpected.join(", ")
                },
            ),
        )),
        Some(_) => {}
    }
    let Some(c) = w.capture.as_ref() else {
        out.push(Finding::plain(
            SERIAL_FORGED_FINDING,
            format!("{who}: Renode left no capture of {}", s.port.renode),
        ));
        return out;
    };
    if c.stray {
        out.push(Finding::plain(
            SERIAL_FORGED_FINDING,
            format!(
                "{who}: the capture's {} bytes are not its pattern {} time(s), each but the last perhaps cut short by a reset, then the frame's first bytes: {} whole pattern(s), then bytes the port was never sent",
                c.len,
                releases + 1,
                c.patterns
            ),
        ));
    } else if c.patterns + c.cut > releases + 1 {
        out.push(Finding::plain(
            SERIAL_FORGED_FINDING,
            format!(
                "{who}: {} whole and {} cut pattern(s) in a run of {} boot(s); each boot sends one, cut only by a reset",
                c.patterns,
                c.cut,
                releases + 1
            ),
        ));
    }
    if !w.pattern_out(s) {
        let sent = w.tx_at_send.filter(|&(m, a)| s.tx.routed(m, a));
        out.push(Finding::plain(
            SERIAL_PATTERN_FINDING,
            match w.tx_at_end {
                _ if c.patterns > 0
                    && sent.is_none()
                    && w.tx_after_solved.is_some() =>
                {
                    format!(
                        "{who}: the pattern was captured, but after it was solved the TX pin went to {}, not MODER 0b10 on AF{}: once solved a pin never changes in a boot, and on silicon bytes it was carrying never reached it",
                        w.tx_at_send.map_or_else(
                            || "a mux the access log does not rebuild (not a word write)".to_owned(),
                            |(m, a)| s.tx.describe(m, a)
                        ),
                        s.tx.alternate_function
                    )
                }
                _ if c.patterns > 0 && sent.is_none() => format!(
                    "{who}: the pattern was captured, but at a boot's first TDR write the TX pin was at {}, not MODER 0b10 on AF{}: it never reached the pin",
                    w.tx_at_send.map_or_else(
                        || "a mux the access log does not rebuild (no word write, or no TDR write)".to_owned(),
                        |(m, a)| s.tx.describe(m, a)
                    ),
                    s.tx.alternate_function
                ),
                Some((m, a)) if c.patterns > 0 && !s.tx.routed(m, a) => format!(
                    "{who}: the pattern was captured, but the TX pin ends at {}, not MODER 0b10 on AF{}: it never reached the pin",
                    s.tx.describe(m, a),
                    s.tx.alternate_function
                ),
                _ => format!(
                    "{who}: the capture does not start with the pattern ({} bytes: the label then {})",
                    s.pattern.len(),
                    hex_bytes(&aethelgard_twin::serial::PATTERN_TAIL)
                ),
            },
        ));
    }
    match w.echo {
        "witnessed" | "not_judged" => {}
        "not_counted" => out.push(Finding::plain(
            SERIAL_ECHO_FINDING,
            match (w.rx_at_injection, &w.rx_after_solved) {
                (Some((m, a)), Some(after)) if s.rx.routed(m, a) => format!(
                    "{who}: the frame came back, but after it was solved the RX pin went to {after}: once solved a pin never changes in a boot, and on silicon the frame's bytes, a character apart, need it solved throughout",
                ),
                _ => format!(
                    "{who}: the frame came back, but at the injection the RX pin was at {}, not MODER 0b10 on AF{}: no byte reached the receiver",
                    w.rx_at_injection
                        .map(|(m, a)| s.rx.describe(m, a))
                        .unwrap_or_default(),
                    s.rx.alternate_function
                ),
            },
        )),
        _ => out.push(Finding::plain(
            SERIAL_ECHO_FINDING,
            match (why, w.injected_ns) {
                (Some(why), None) => format!(
                    "{who}: no frame went in: {why}, so its echo cannot be witnessed"
                ),
                (why, _) => format!(
                    "{who}: {} of the frame's {} bytes ({}) came back{}",
                    c.echoed,
                    aethelgard_twin::serial::FRAME_LEN,
                    hex_bytes(&s.frame),
                    why.map(|w| format!("; {w}")).unwrap_or_default()
                ),
            },
        )),
    }
    out
}

/// `43 4F 4E`: bytes as a finding names them.
pub fn hex_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
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
    /// `batched`, `stopped`, or `unanchored` (edges with no anchor observed,
    /// `AET-WO-2302`).
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
    /// The window's anchor: its timer's first `CEN` rise in the boot the
    /// judge reads (`AET-WO-2302`); `None` when none was observed.
    pub anchor_ns: Option<u64>,
    /// Edges before the anchor, and after the window's end, in that boot:
    /// never judged, each named.
    pub edges_before: u64,
    pub edges_after: u64,
    /// Its timer's `CEN` rises in that boot, the anchor's first (kind 26
    /// carries it, so replay binds `motor_anchor_repeated`; adversarial
    /// pass 1, L3).
    pub cen_rises: u64,
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
        anchor_ns: None,
        edges_before: 0,
        edges_after: 0,
        cen_rises: 0,
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
/// point (as an identity record is judged only then), inside its own window
/// (`AET-WO-2302`): from its anchor, or the boot's start without one, to
/// its end. An output with edges and no anchor is the anchor's finding
/// ([`anchor_findings`]).
fn witness_findings_of(m: &Motor, w: &MotorWitness) -> Vec<Finding> {
    let window_ns = w.end_ns.saturating_sub(w.anchor_ns.unwrap_or(0));
    let o = &m.observer;
    let who = motor_who(o);
    let mut out = Vec::new();
    let model = format!(
        "the model's period {} ns and high time {} ns (ARR and CCR ticks at {} Hz, each realized in whole ns rounded up)",
        o.period_ns, o.high_ns, o.kernel_hz
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
                        "; its window of {window_ns} ns is shorter than two of its periods ({} ns), so it cannot show one",
                        2 * o.period_ns
                    )
                } else {
                    String::new()
                }
            ),
        )),
        "unanchored" => {}
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

/// For each check, the addresses its controller found no device at during
/// its own step (`AET-WO-3305`). The firmware ends each step by writing its
/// status word once ([`Hook::Step`]), so an `Unknown slave` line goes to the
/// I2C check on its controller whose status write first follows it. That
/// boundary is the firmware's, so it is trusted only as the generated
/// firmware draws it: when every I2C check on the controller wrote its status
/// exactly once, and a status write follows the line. Otherwise the line
/// goes to every I2C check on the controller, as the whole run's list did
/// before (adversarial pass, F1 to F3): a status written early, twice or
/// never moves no address off its step, it only keeps the old masking.
/// Empty for an SPI check. Attribution only, never a witness.
///
/// [`Hook::Step`]: crate::anchor::Hook::Step
fn absent_by_step(
    checks: &[aethelgard_twin::boot::StepCheck],
    log: &[&str],
    hooks: &[Seen],
) -> Vec<Vec<u8>> {
    fn controller(c: &aethelgard_twin::boot::StepCheck) -> Option<&str> {
        matches!(c.wire, aethelgard_twin::boot::Wire::I2c { .. })
            .then(|| c.device.split('.').next().unwrap_or_default())
    }
    // Each check's status writes, by their positions in the log.
    let mut writes: Vec<Vec<usize>> = vec![Vec::new(); checks.len()];
    for (i, h, _) in hooks {
        if let crate::anchor::Hook::Step { check, .. } = h
            && let Some(w) = writes.get_mut(*check)
        {
            w.push(*i);
        }
    }
    let mut absent = vec![Vec::new(); checks.len()];
    for (i, line) in log.iter().enumerate() {
        let Some((bus, address)) = checks.iter().find_map(|c| {
            let bus = controller(c)?;
            absent_address(line, bus).map(|a| (bus, a))
        }) else {
            continue;
        };
        let mine: Vec<usize> = (0..checks.len())
            .filter(|&j| controller(&checks[j]) == Some(bus))
            .collect();
        // The check whose one status write first follows the line.
        let owner = mine
            .iter()
            .all(|&j| writes[j].len() == 1)
            .then(|| {
                mine.iter()
                    .filter(|&&j| writes[j][0] > i)
                    .min_by_key(|&&j| writes[j][0])
            })
            .flatten();
        match owner {
            Some(&j) => absent[j].push(address),
            None => mine.iter().for_each(|&j| absent[j].push(address)),
        }
    }
    absent
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
pub fn judge_boot(
    code: i32,
    out: &Output,
    plan: &BootPlan<'_>,
    capture: &Captured,
) -> Result<Judged, Stop> {
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
    // A model that did not compile stops the line before the platform loads,
    // and Renode still exits 0: judged before any marker (`AET-WO-4220` F2).
    if let Some(stop) = model_load_failed(&lines) {
        return Err(stop);
    }
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
    let probed = plan.probe_count();
    let probe_begins = sentinels(&lines[..begin], PROBE_BEGIN, probed).map_err(untrusted)?;
    let probe_ends = sentinels(&lines[..begin], PROBE_END, probed).map_err(untrusted)?;
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
    // Its start, which places every hook's count (`AET-WO-2302`): 0, or the
    // last released reset's release plus the nudge.
    let start_ns = plan
        .resets
        .iter()
        .rev()
        .find_map(|r| r.released_ns)
        .map_or(0, |at| at + power::NUDGE_NS);
    // Its lines of ours, each read whole; past the filter's budget a line may
    // have been dropped, so none is read: every anchor and injection is then
    // unreadable, never absent (`AET-WO-2204` D2's rule).
    // A line its count does not place at its stamp is the core's sleep
    // (adversarial pass 1, M1): `cpu_halted`, never an error of the tool.
    let (hooks, slept) = match (out.dropped_log_lines == 0)
        .then(|| hooks_of(&last_boot, start_ns))
        .transpose()
        .map_err(untrusted)?
    {
        Some((h, slept)) => (Some(h), slept),
        None => (None, None),
    };
    let boot_word_hook = hooks.as_deref().and_then(|h| {
        h.iter().find_map(|(i, x, ns)| match x {
            crate::anchor::Hook::BootWord { value, before, .. }
                if *value == aethelgard_twin::boot::BOOT_MAGIC
                    && *before != aethelgard_twin::boot::BOOT_MAGIC =>
            {
                Some((*i, *ns))
            }
            _ => None,
        })
    });
    let timers = hooks
        .as_deref()
        .map(|h| timer_witnesses(plan, h, plan.virtual_ns))
        .unwrap_or_default();
    let injections = match hooks.as_deref() {
        Some(h) => port_injections(plan, h, boot_word_hook),
        None => vec![
            PortInjection {
                why: Some("Renode's log could not be read whole".to_owned()),
                ..Default::default()
            };
            plan.serial.len()
        ],
    };

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
        brown_out: 0,
        flash_access: None,
        loops: vec![[0, 0]; plan.self_loops.len()],
        registers: vec![0; plan.image.registers.len()],
        records: vec![(0, 0); checks.len()],
        witnesses,
        absent: hooks.as_deref().map_or_else(
            || vec![Vec::new(); checks.len()],
            |h| absent_by_step(checks, &last_boot, h),
        ),
        unrouted,
        serial: Vec::new(),
        // Past the filter's budget an edge may have been dropped, so no
        // output's edges can be read whole (`AET-WO-2204` D2's rule).
        motors: plan
            .motors
            .iter()
            .map(|m| {
                let edges = (out.dropped_log_lines == 0)
                    .then(|| observer_edges(&m.observer.source, &last_boot));
                let timer = timers.iter().find(|t| t.timer == m.observer.timer);
                witness_motor_in(m, edges, timer, plan.virtual_ns)
            })
            .collect(),
        boot_word_ns: boot_word_hook.map(|(_, ns)| ns),
        register_expected: plan.image.registers.iter().map(|r| r.value).collect(),
        timers: timers.clone(),
        injections: injections.clone(),
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
            Read::BrownOut => observed.brown_out = word()?,
            Read::Loop(i, k) => observed.loops[i][usize::from(k)] = word()?,
            Read::Register(i) => observed.registers[i] = word()?,
            Read::Record(i, 0) => observed.records[i].0 = word()?,
            Read::Record(i, _) => {
                let r = &mut observed.records[i].1;
                *r = (*r << 32) | u64::from(word()?);
            }
        }
    }
    // Each boot's accesses on the logged peripherals, in log order: the
    // first boot, then one per release (`AET-WO-4300b` adversarial F1–F4).
    let starts = std::iter::once(0).chain(releases.iter().map(|&r| r + 1));
    let ends = releases.iter().copied().chain(std::iter::once(begin));
    let boots: Vec<Vec<&str>> = starts
        .zip(ends)
        .map(|(s, e)| (s..e).filter(|&i| !in_probe(i)).map(|i| lines[i]).collect())
        .collect();
    let accesses = if out.dropped_log_lines == 0 {
        access_boots(&boots, &plan.logged_sources())
    } else {
        None
    };
    // The flash controller's accesses, each boot's, but for the line
    // Renode answers the judge's own command with (C1 m2).
    observed.flash_access = (out.dropped_log_lines == 0)
        .then(|| {
            let flash: Vec<Vec<&str>> = boots
                .iter()
                .map(|b| {
                    b.iter()
                        .copied()
                        .filter(|l| log_body(l) != Some(FLASH_LOG_ON))
                        .collect()
                })
                .collect();
            access_boots(&flash, &[aethelgard_twin::boot::FLASH_RENODE])
        })
        .flatten()
        .map(|b| plan.image.brown_out.judge_accesses(&b, observed.brown_out));
    // How many of the last boot's accesses precede each port's injection:
    // the RX pin's words there are rebuilt from them (adversarial pass 2,
    // A2), each port's at its own first byte (`AET-WO-2302` D4).
    let inject_at: Vec<Option<usize>> = injections
        .iter()
        .map(|j| {
            j.line
                .filter(|_| j.at_ns.is_some())
                .and_then(|l| access_boots(&[last_boot[..l].to_vec()], &plan.logged_sources()))
                .map(|v| v[0].len())
        })
        .collect();
    observed.serial = serial_witnesses(
        plan,
        &observed.registers,
        &injections,
        capture,
        accesses.as_deref(),
        &inject_at,
    );
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
    // The echo follows the boot word, so without it nothing is judged of it.
    if !booted {
        for w in &mut observed.serial {
            w.echo = "not_judged";
        }
    }
    let halted = observed.instructions < expected;
    if booted && halted {
        return Err(untrusted(format!(
            "the boot word is set but the core executed {} of {expected} instructions",
            observed.instructions
        )));
    }
    // A line of ours lagging its stamp is a core that executed fewer
    // instructions than its time held: with the whole run's executed, the
    // witnesses disagree.
    if let Some(s) = slept.as_deref().filter(|_| !halted) {
        return Err(untrusted(format!(
            "{s}, but the core executed all the run's {expected} instructions"
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
            aethelgard_chaos::campaign::BOOT_WORD_FINDING,
            format!(
                "the boot word at {:#010x} reads {:#010x}, not {:#010x}",
                plan.boot_word,
                observed.boot_word,
                aethelgard_twin::boot::BOOT_MAGIC
            ),
        ));
    }
    // One `cpu_halted`: the core executed fewer instructions than the run
    // holds, naming the first line of ours that shows it lagging its time
    // (it slept, `WFI`): what a hook or action anchored there is then
    // judged at its stamp, and an action whose count was due falls stale
    // and fails by its own name (adversarial pass 1, M1).
    if halted {
        let lag = slept
            .as_deref()
            .map(|s| format!("; {s}: the core slept or stopped before it"))
            .unwrap_or_default();
        findings.push(Finding::plain(
            "cpu_halted",
            format!(
                "the core executed {} of the run's {expected} instructions{lag}",
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
    // (`AET-WO-4210` D-B1, accepted). A DShot timer whose stop ran ends
    // with `CEN` cleared, the judge's own act (`AET-WO-2302` D2); what it
    // held at the window's end is the stop's read-back, judged apart.
    for t in &observed.timers {
        let stopped = t
            .stop
            .as_ref()
            .is_some_and(|s| s.cleared.is_some_and(|c| c & 1 == 0));
        let cr1 = format!("TIM{}.CR1", t.timer);
        if let Some(i) = plan.image.registers.iter().position(|r| r.name == cr1)
            && stopped
        {
            observed.register_expected[i] &= !1;
        }
    }
    for ((r, &got), &want) in plan
        .image
        .registers
        .iter()
        .zip(&observed.registers)
        .zip(&observed.register_expected)
    {
        if got & r.mask != want {
            findings.push(Finding::plain(
                "register_mismatch",
                format!(
                    "{} at {:#010x} reads {got:#010x}; under mask {:#010x} it must be {want:#010x}{}",
                    r.name,
                    r.address,
                    r.mask,
                    if want == r.value {
                        ""
                    } else {
                        " (CEN cleared by the judge's stop at its window's end)"
                    }
                ),
            ));
        }
    }
    // The brown-out level (`AET-WO-4230` D10), once the firmware reached its
    // fixed point, as an identity record is: word 1 against the declared
    // level, and the flash controller read once and never written.
    if booted {
        findings.extend(brown_out_findings(
            &plan.image.brown_out,
            observed.brown_out,
            observed.flash_access.as_deref(),
        ));
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
        findings.extend(timeouts_without_the_boot_word(checks, &observed.records));
    }
    // The motor outputs (`AET-WO-4300a` §1.4): the static check judges
    // silicon from the topology alone, the twin's witness the model's
    // conventions, never blended (`4300` D3). The witness is read only once
    // the firmware reached its fixed point, as an identity record is; each
    // inside its own window, anchored at its timer's `CEN` and bounded by
    // the judge (`AET-WO-2302`).
    if booted {
        for (m, w) in plan.motors.iter().zip(&observed.motors) {
            findings.extend(witness_findings_of(m, w));
        }
        if hooks.is_some() {
            let last_device = checks
                .iter()
                .flat_map(|c| own_lines(c, &last_boot))
                .map(|(i, _)| i)
                .max();
            findings.extend(anchor_findings(
                plan,
                &observed.timers,
                &observed.motors,
                last_device,
                boot_word_hook,
            ));
        }
    }
    // The serial ports (`AET-WO-4300b`): a capture directory holding what
    // no port's backend wrote is forged whether or not the boot finished;
    // each port's witness is read once the firmware reached its fixed point,
    // as an identity record is. The silicon check is the caller's, apart.
    findings.extend(capture_findings(plan, capture));
    if booted {
        let releases = plan
            .resets
            .iter()
            .filter(|r| r.released_ns.is_some())
            .count();
        for ((s, w), j) in plan
            .serial
            .iter()
            .zip(&observed.serial)
            .zip(&observed.injections)
        {
            findings.extend(serial_findings_of(s, w, releases, j.why.as_deref()));
        }
        // The ports' wire time from the frames' anchor, the boot word's
        // quantum, summed in the specification's order, inside the run: known
        // only now (`AET-WO-2302` D4).
        findings.extend(observed.boot_word_ns.and_then(|bw| wire_past_run(plan, bw)));
        // Each port in only after the one before it echoed its whole frame
        // (adversarial pass 1, L2).
        for (s, j) in plan.serial.iter().zip(&observed.injections) {
            if let Some(why) = &j.out_of_order {
                findings.push(Finding::plain(
                    aethelgard_chaos::campaign::SERIAL_INJECTION_ORDER_FINDING,
                    format!("{}: {why}", serial_who(s)),
                ));
            }
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

/// One line of ours in the boot the judge reads: its position among the
/// boot's lines, what it says, and its instant.
type Seen = (usize, crate::anchor::Hook, u64);

/// Each line of ours in `boot` ([`crate::anchor`]), with its position in
/// `boot` and its instant: its count placed in a boot started at
/// `start_ns`, 10 ns an instruction. A scheduled action's line (a stop, its
/// `CEN` cleared, an injected byte, a stale action) is stamped exactly
/// there, as every probe measured it; a hook's line at or before it, inside
/// one quantum (measured up to 60 ns early). A line of ours not exactly one
/// of its shapes is refused, never read past (exit 5), and so is a count
/// ahead of its stamp: no core runs ahead of its time. A count behind its
/// stamp is the core's: it executed fewer instructions than its time held
/// (it slept, `WFI`), so the line's instant is its stamp, and the first
/// such line is returned beside, for `cpu_halted` (adversarial pass 1, M1;
/// before it, exit 5).
fn hooks_of(boot: &[&str], start_ns: u64) -> Result<(Vec<Seen>, Option<String>), String> {
    let mut out = Vec::new();
    let mut slept = None;
    for (i, line) in boot.iter().enumerate() {
        let Some((stamp, body)) = log_stamped(line) else {
            continue;
        };
        let Some(hook) = crate::anchor::parse(body) else {
            continue;
        };
        let hook = hook.ok_or_else(|| {
            format!(
                "`{}` is the CLI's hook line in no shape it writes",
                body.escape_debug()
            )
        })?;
        let at = crate::anchor::instant(start_ns, hook.count());
        let placed = if hook.scheduled() {
            stamp == at
        } else {
            stamp <= at && at - stamp < aethelgard_twin::boot::QUANTUM_NS
        };
        let misplaced = || {
            format!(
                "`{}`: its count places it at {at} ns in a boot started at {start_ns} ns, but it is stamped {stamp} ns",
                body.escape_debug()
            )
        };
        // A count ahead of its time is no core's: refused.
        if !placed && stamp < at {
            return Err(misplaced());
        }
        if !placed && slept.is_none() {
            slept = Some(misplaced());
        }
        out.push((i, hook, if placed { at } else { stamp }));
    }
    Ok((out, slept))
}

/// What the hooks showed of one motor timer in the boot the judge reads
/// (`AET-WO-2302`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimerWitness {
    /// `1` for `TIM1`.
    pub timer: u32,
    pub dshot: bool,
    /// The first write that set `CEN` over a word without it: its position
    /// among the boot's lines, its count and its instant.
    pub anchor: Option<(usize, u64, u64)>,
    /// Every such rise's count, the anchor's first.
    pub rises: Vec<u64>,
    /// The anchor's window's end: `DSHOT_WINDOW_NS` after it for DShot, the
    /// boot's end for PWM.
    pub end_ns: Option<u64>,
    /// The judge's stop for the anchor, as it logged it; `None` when it did
    /// not run in this boot (or the timer is PWM).
    pub stop: Option<StopSeen>,
    /// Stops this boot logged as stale: an earlier boot's, which did nothing.
    pub stale: usize,
}

/// One judge's stop: its instant, the timer's solved registers it read back
/// (in the image's order), and `CR1` after it cleared `CEN` (`None` when the
/// clearing line is missing).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopSeen {
    pub at_ns: u64,
    pub words: Vec<u32>,
    pub cleared: Option<u32>,
}

/// A DShot output's evidence window's end, from its anchor (its timer's first
/// `CEN` rise in the boot): `DSHOT_WINDOW_NS` after it, where the judge's stop
/// runs. The one place the window's end is computed (the architect's ruling
/// R1 on PR #175).
fn dshot_window_end(anchor_ns: u64) -> u64 {
    anchor_ns + aethelgard_twin::boot::DSHOT_WINDOW_NS
}

/// Each hooked timer's witness from `hooks` (the boot's lines of ours) in a
/// boot that ends at `end_ns`. A DShot stop's instant is its window's end,
/// its anchor's plus `DSHOT_WINDOW_NS`: the count it ran at is exactly its
/// anchor's plus the window's, which the stale check holds.
fn timer_witnesses(plan: &BootPlan<'_>, hooks: &[Seen], end_ns: u64) -> Vec<TimerWitness> {
    use crate::anchor::{Hook, Stale};
    let window = crate::anchor::instructions(aethelgard_twin::boot::DSHOT_WINDOW_NS);
    plan.timer_hooks()
        .into_iter()
        .map(|t| {
            let rises: Vec<(usize, u64, u64)> = hooks
                .iter()
                .filter_map(|(i, h, ns)| match h {
                    Hook::Cr1 {
                        timer,
                        value,
                        before,
                        count,
                    } if *timer == t.timer && value & 1 == 1 && before & 1 == 0 => {
                        Some((*i, *count, *ns))
                    }
                    _ => None,
                })
                .collect();
            let anchor = rises.first().copied();
            let window_end = anchor.map(|(_, _, at)| match t.dshot {
                true => dshot_window_end(at),
                false => end_ns,
            });
            let stop = anchor.filter(|_| t.dshot).and_then(|(_, a, at)| {
                let due = a + window;
                let words = hooks.iter().find_map(|(_, h, _)| match h {
                    Hook::Stop {
                        timer,
                        anchor,
                        count,
                        words,
                    } if *timer == t.timer && *anchor == a && *count == due => Some(words.clone()),
                    _ => None,
                })?;
                let cleared = hooks.iter().find_map(|(_, h, _)| match h {
                    Hook::Cleared {
                        timer,
                        anchor,
                        count,
                        cr1,
                    } if *timer == t.timer && *anchor == a && *count == due => Some(*cr1),
                    _ => None,
                });
                Some(StopSeen {
                    at_ns: dshot_window_end(at),
                    words,
                    cleared,
                })
            });
            let stale = hooks
                .iter()
                .filter(|(_, h, _)| {
                    matches!(h, Hook::Stale { what: Stale::Stop { timer }, .. } if *timer == t.timer)
                })
                .count();
            TimerWitness {
                timer: t.timer,
                dshot: t.dshot,
                anchor,
                rises: rises.iter().map(|r| r.1).collect(),
                end_ns: window_end,
                stop,
                stale,
            }
        })
        .collect()
}

/// One output's witness inside its timer's window (`AET-WO-2302`): the
/// edges from its anchor to its window's end are judged as before
/// ([`witness_motor`]); edges before the anchor or after the end are
/// counted, and named by the caller, never judged and never dropped
/// silently. Edges with no anchor observed are `unanchored`.
fn witness_motor_in(
    m: &Motor,
    edges: Option<Option<Vec<(u64, bool)>>>,
    timer: Option<&TimerWitness>,
    run_end_ns: u64,
) -> MotorWitness {
    let anchor = timer.and_then(|t| t.anchor.map(|a| a.2));
    let end = timer.and_then(|t| t.end_ns).unwrap_or(run_end_ns);
    let cen_rises = timer.map_or(0, |t| t.rises.len() as u64);
    let Some(Some(all)) = edges else {
        let mut w = witness_motor(m, edges, end);
        w.anchor_ns = anchor;
        w.cen_rises = cen_rises;
        return w;
    };
    let Some(at) = anchor else {
        let mut w = witness_motor(m, Some(Some(Vec::new())), end);
        if !all.is_empty() {
            w.status = "unanchored";
            w.rises = all.iter().filter(|e| e.1).count() as u64;
            w.falls = all.len() as u64 - w.rises;
            w.off_grid = all
                .iter()
                .filter(|e| e.0 % aethelgard_twin::boot::QUANTUM_NS != 0)
                .count() as u64;
        }
        return w;
    };
    let before = all.iter().filter(|e| e.0 < at).count() as u64;
    let after = all.iter().filter(|e| e.0 > end).count() as u64;
    let inside: Vec<(u64, bool)> = all
        .into_iter()
        .filter(|e| e.0 >= at && e.0 <= end)
        .collect();
    let mut w = witness_motor(m, Some(Some(inside)), end);
    w.anchor_ns = Some(at);
    w.edges_before = before;
    w.edges_after = after;
    w.cen_rises = cen_rises;
    w
}

/// Each timer's findings and each output's window findings, once the boot
/// reached its fixed point (`AET-WO-2302`, the anchors' bounds, per boot,
/// the last boot's): the anchor after the boot's last device step's
/// witness and before its boot word, observed once, a DShot window inside
/// the run and its stop run, and every edge inside its window. `last_device`
/// is the last device witness line's position, `boot_word` the boot word's.
fn anchor_findings(
    plan: &BootPlan<'_>,
    timers: &[TimerWitness],
    witnesses: &[MotorWitness],
    last_device: Option<usize>,
    boot_word: Option<(usize, u64)>,
) -> Vec<Finding> {
    use aethelgard_chaos::campaign::{
        JUDGE_STOP_MISMATCH_FINDING, JUDGE_STOP_MISSING_FINDING, MOTOR_ANCHOR_ORDER_FINDING,
        MOTOR_ANCHOR_REPEATED_FINDING, MOTOR_ANCHOR_UNOBSERVED_FINDING, MOTOR_EDGE_AFTER_FINDING,
        MOTOR_EDGE_BEFORE_FINDING, MOTOR_WINDOW_PAST_RUN_FINDING,
    };
    let mut out = Vec::new();
    for t in timers {
        let name = format!("TIM{}", t.timer);
        let Some((at, count, at_ns)) = t.anchor else {
            continue;
        };
        if t.rises.len() > 1 {
            out.push(Finding::plain(
                MOTOR_ANCHOR_REPEATED_FINDING,
                format!(
                    "{name}: CEN rose {} times in the boot the judge reads (at instructions {}); a timer starts once, and its window anchors at the first",
                    t.rises.len(),
                    t.rises
                        .iter()
                        .map(u64::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }
        let order = match (last_device, boot_word) {
            (_, None) => Some("no write of the boot word was observed by its hook".to_owned()),
            (_, Some((b, _))) if at > b => {
                Some("after the boot word, which the bring-up writes last".to_owned())
            }
            (Some(d), _) if at < d => Some(
                "before the last device step's witness, which the bring-up completes first"
                    .to_owned(),
            ),
            _ => None,
        };
        if let Some(why) = order {
            out.push(Finding::plain(
                MOTOR_ANCHOR_ORDER_FINDING,
                format!(
                    "{name}: CEN rose at instruction {count} ({at_ns} ns), {why}: the anchor cannot be bounded"
                ),
            ));
        }
        if !t.dshot {
            continue;
        }
        let end = dshot_window_end(at_ns);
        if end > plan.virtual_ns {
            out.push(Finding::plain(
                MOTOR_WINDOW_PAST_RUN_FINDING,
                format!(
                    "{name}: its DShot window from its anchor at {at_ns} ns ends at {end} ns, past the boot's end at {} ns (DSHOT_WINDOW_NS {} ns)",
                    plan.virtual_ns,
                    aethelgard_twin::boot::DSHOT_WINDOW_NS
                ),
            ));
            continue;
        }
        match &t.stop {
            None => out.push(Finding::plain(
                JUDGE_STOP_MISSING_FINDING,
                format!(
                    "{name}: the judge's stop due at {end} ns (its anchor plus DSHOT_WINDOW_NS) did not run in the boot the judge reads{}",
                    if t.stale > 0 {
                        format!("; {} stale stop(s) of an earlier boot did nothing", t.stale)
                    } else {
                        String::new()
                    }
                ),
            )),
            Some(s) if s.cleared.is_none_or(|c| c & 1 != 0) => out.push(Finding::plain(
                JUDGE_STOP_MISSING_FINDING,
                format!(
                    "{name}: the judge's stop at {} ns read the timer back but did not leave CEN cleared ({})",
                    s.at_ns,
                    s.cleared
                        .map_or_else(|| "no read after it".to_owned(), |c| format!("CR1 {c:#010x}"))
                ),
            )),
            Some(s) => {
                let prefix = format!("TIM{}.", t.timer);
                let regs: Vec<&aethelgard_twin::boot::Register> = plan
                    .image
                    .registers
                    .iter()
                    .filter(|r| r.name.starts_with(&prefix))
                    .collect();
                if regs.len() != s.words.len() {
                    out.push(Finding::plain(
                        JUDGE_STOP_MISMATCH_FINDING,
                        format!(
                            "{name}: the judge's stop at {} ns read {} words back, not the timer's {} solved registers",
                            s.at_ns,
                            s.words.len(),
                            regs.len()
                        ),
                    ));
                }
                for (r, &got) in regs.iter().zip(&s.words) {
                    if got & r.mask != r.value {
                        out.push(Finding::plain(
                            JUDGE_STOP_MISMATCH_FINDING,
                            format!(
                                "{} at {:#010x} reads {got:#010x} at the judge's stop ({} ns, the window's end); under mask {:#010x} it must be {:#010x}",
                                r.name, r.address, s.at_ns, r.mask, r.value
                            ),
                        ));
                    }
                }
            }
        }
    }
    for (m, w) in plan.motors.iter().zip(witnesses) {
        let who = motor_who(&m.observer);
        if w.status == "unanchored" {
            out.push(Finding::plain(
                MOTOR_ANCHOR_UNOBSERVED_FINDING,
                format!(
                    "{who}: the observer logged {} rises and {} falls, but no CPU store of TIM{}.CR1 set CEN in the boot the judge reads, so the output's window has no anchor (a DMA write of CR1 is not observed)",
                    w.rises, w.falls, m.observer.timer
                ),
            ));
        }
        if w.edges_before > 0 {
            out.push(Finding::plain(
                MOTOR_EDGE_BEFORE_FINDING,
                format!(
                    "{who}: {} edge(s) before its timer's anchor at {} ns, in the same boot",
                    w.edges_before,
                    w.anchor_ns.unwrap_or_default()
                ),
            ));
        }
        if w.edges_after > 0 {
            out.push(Finding::plain(
                MOTOR_EDGE_AFTER_FINDING,
                format!(
                    "{who}: {} edge(s) after its window's end at {} ns, where the judge's stop stopped the timer: the firmware drove the pin or restarted the timer after its window",
                    w.edges_after, w.end_ns
                ),
            ));
        }
    }
    out
}

/// `serial_wire_past_run` (`AET-WO-2302` D4): the ports' wire time after
/// the boot word, summed in the specification's order over the ports the
/// chain injects (not those the scenario holds open), each its frame as
/// injected and one echoed character ([`frame_echo_ns`]; its pattern went
/// before the boot word, adversarial pass 2, A2-3), from one quantum after
/// the boot word at `boot_word_ns`, past the boot's end; known only after
/// the run. The first port at which it passes is named.
fn wire_past_run(plan: &BootPlan<'_>, boot_word_ns: u64) -> Option<Finding> {
    let (s, times, end) = past_window(
        plan.serial.iter().filter(|s| !s.open),
        boot_word_ns + aethelgard_twin::boot::QUANTUM_NS,
        plan.virtual_ns,
        frame_echo_ns,
    )?;
    Some(Finding::plain(
        aethelgard_chaos::campaign::SERIAL_WIRE_PAST_RUN_FINDING,
        format!(
            "{}: the ports' frames and one echoed character each through it, {}, from one quantum after the boot word at {boot_word_ns} ns, passes the boot's end at {} ns (the patterns go before the boot word)",
            serial_who(s),
            match (times, end) {
                (Some((char_ns, wire, sum)), Some(end)) => format!(
                    "its own {wire} ns ({char_ns} ns a character) of {sum} ns in all, ending at {end} ns"
                ),
                _ => "which its baud cannot give".to_owned(),
            },
            plan.virtual_ns
        ),
    ))
}

/// What the injection of one port showed (`AET-WO-2302` D4).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PortInjection {
    /// The first byte's instant; `None` when no byte went in, or the bytes
    /// that did are not one frame.
    pub at_ns: Option<u64>,
    /// The frame's bytes that went in, in order from the first.
    pub bytes: usize,
    /// The RX pin's `MODER` and `AFR`, read with the first byte.
    pub mux: Option<(u32, u32)>,
    /// The first byte's line among the boot's lines.
    pub line: Option<usize>,
    /// Why no whole frame went in, for the echo's finding.
    pub why: Option<String>,
    /// Why a byte of its frame went in before the port before it in the
    /// chain had echoed its whole frame (`serial_injection_out_of_order`,
    /// adversarial pass 1, L2); `None` when in order.
    pub out_of_order: Option<String>,
}

/// How many bytes port `j` wrote to its `TDR` that echo its `frame` in
/// order, from its frame's first byte's line on (`AET-WO-2302` adversarial
/// pass 1, L1/L2: a byte that is not the frame's next is never counted),
/// and the line and instant of the `FRAME_LEN`-th: recounted from the
/// logged bytes, never taken from the hook's own count.
fn frame_echo(hooks: &[Seen], j: usize, frame: &[u8]) -> (u64, Option<(usize, u64)>) {
    use crate::anchor::Hook;
    let Some(from) = hooks.iter().find_map(|(i, h, _)| match h {
        Hook::Inject { port, byte: 0, .. } if *port == j => Some(*i),
        _ => None,
    }) else {
        return (0, None);
    };
    let mut n = 0usize;
    for (i, h, ns) in hooks {
        if let Hook::Tdr { port, value, .. } = h
            && *port == j
            && *i > from
            && frame.get(n).is_some_and(|&b| u32::from(b) == *value)
        {
            n += 1;
            if n == frame.len() {
                return (n as u64, Some((*i, *ns)));
            }
        }
    }
    (n as u64, None)
}

/// Each port's injection from `hooks`: the first port not held open
/// anchored at the boot word, each later one at the `FRAME_LEN`-th byte
/// echoing its frame of the last port before it not held open
/// (`AET-WO-2302` D4, D5), each byte at its due count; a port in before
/// that byte is named ([`frame_echo`]). A port held
/// open gets nothing, and says so. Bytes that are not exactly one frame in
/// order (a boot word written twice), or any byte into a port held open,
/// are no injection, and say why.
fn port_injections(
    plan: &BootPlan<'_>,
    hooks: &[Seen],
    boot_word: Option<(usize, u64)>,
) -> Vec<PortInjection> {
    use crate::anchor::Hook;
    /// One `INJECT` line: its position, the byte's index, its anchor and
    /// count, and the mux read with the first.
    struct Byte {
        line: usize,
        index: usize,
        anchor: u64,
        count: u64,
        ns: u64,
        mux: Option<(u32, u32)>,
    }
    let ports = plan.port_hooks();
    let frame = aethelgard_twin::serial::FRAME_LEN;
    (0..plan.serial.len())
        .map(|k| {
            let bytes: Vec<Byte> = hooks
                .iter()
                .filter_map(|(i, h, ns)| match h {
                    Hook::Inject {
                        port,
                        byte,
                        anchor,
                        count,
                        mux,
                    } if *port == k => Some(Byte {
                        line: *i,
                        index: *byte,
                        anchor: *anchor,
                        count: *count,
                        ns: *ns,
                        mux: *mux,
                    }),
                    _ => None,
                })
                .collect();
            let Some(p) = ports.get(k) else {
                return PortInjection {
                    why: Some("its frame's spacing cannot be computed".to_owned()),
                    ..Default::default()
                };
            };
            // The port before it in the chain: the last before it not held
            // open (`AET-WO-2302` D5), and its frame's echo.
            let before = (0..k).rev().find(|&j| !ports[j].open);
            let (echoed, last) =
                before.map_or((0, None), |j| frame_echo(hooks, j, &ports[j].frame));
            // Any byte of this port's in before that echo was whole is out
            // of the chain's order (L2), whatever else is wrong with it.
            let out_of_order = before
                .zip(bytes.first())
                .filter(|(_, b)| last.is_none_or(|(l, _)| b.line < l))
                .map(|(j, b)| {
                    format!(
                        "its frame's first byte went in at {} ns, {}; each port goes in only once the one before it has echoed its whole frame",
                        b.ns,
                        match last {
                            Some((_, at)) => format!("before port {j}'s last echoed frame byte at {at} ns"),
                            None => format!("while port {j} before it had echoed {echoed} of its {frame} frame bytes"),
                        }
                    )
                });
            if p.open && bytes.is_empty() {
                return PortInjection {
                    why: Some(
                        "the scenario holds its RX line open (serial_rx_open)".to_owned(),
                    ),
                    ..Default::default()
                };
            }
            if bytes.is_empty() {
                let why = match (before, boot_word) {
                    (None, None) => "no write of the boot word was observed by its hook".to_owned(),
                    (None, Some(_)) => "the boot ended before the boot word's quantum".to_owned(),
                    (Some(j), _) if echoed < frame as u64 => format!(
                        "port {j} before it echoed {echoed} of its {frame} bytes, and each port goes in only once the one before it has echoed its whole frame",
                    ),
                    _ => "the boot ended before its quantum after the port before it".to_owned(),
                };
                return PortInjection {
                    why: Some(why),
                    ..Default::default()
                };
            }
            let anchor = bytes[0].anchor;
            let one_frame = bytes.iter().enumerate().all(|(j, b)| {
                b.index == j
                    && b.anchor == anchor
                    && b.count
                        == anchor
                            + crate::anchor::instructions(
                                aethelgard_twin::boot::QUANTUM_NS + j as u64 * p.spacing_ns,
                            )
            }) && bytes.len() <= frame
                && bytes[0].mux.is_some()
                && !p.open;
            if !one_frame {
                return PortInjection {
                    why: Some(format!(
                        "{} byte writes went in that are not one frame in order from one anchor (the boot word written over another word more than once?)",
                        bytes.len()
                    )),
                    out_of_order,
                    ..Default::default()
                };
            }
            PortInjection {
                at_ns: Some(bytes[0].ns),
                bytes: bytes.len(),
                mux: bytes[0].mux,
                line: Some(bytes[0].line),
                why: (bytes.len() < frame).then(|| {
                    format!(
                        "{} of its {frame} bytes went in before the boot's end",
                        bytes.len()
                    )
                }),
                out_of_order,
            }
        })
        .collect()
}

/// The brown-out word's finding and the flash controller's, from what the
/// boot read (`AET-WO-4230` D10): each names both the declared level and
/// what the firmware recorded; a log that could not be read whole is a
/// failure, never a pass.
pub fn brown_out_findings(
    b: &aethelgard_twin::boot::BrownOut,
    word: u32,
    flash: Option<&[String]>,
) -> Vec<Finding> {
    use aethelgard_chaos::campaign::{BROWN_OUT_FINDING, FLASH_ACCESS_FINDING};
    use aethelgard_twin::boot::BrownOutOutcome;
    let mut out = Vec::new();
    let at = format!(
        "the boot record's FLASH_OPTSR_CUR word at {:#010x}",
        b.word_at
    );
    match b.judge(word) {
        BrownOutOutcome::Pass => {}
        BrownOutOutcome::Unset => out.push(Finding::plain(
            BROWN_OUT_FINDING,
            format!(
                "{at} reads {word:#010x}, the judge's fill: the firmware never recorded BOR_LEV; the specification declares level {}",
                b.level
            ),
        )),
        BrownOutOutcome::Mismatch { recorded } => out.push(Finding::plain(
            BROWN_OUT_FINDING,
            format!(
                "{at} reads {word:#010x}: BOR_LEV {recorded}, and the specification declares level {}",
                b.level
            ),
        )),
    }
    match flash {
        Some([]) => {}
        Some(notes) => out.push(Finding::plain(FLASH_ACCESS_FINDING, notes.join("; "))),
        None => out.push(Finding::plain(
            FLASH_ACCESS_FINDING,
            "the flash controller's access log could not be read whole, so its one read of FLASH_OPTSR_CUR is unwitnessed"
                .to_owned(),
        )),
    }
    out
}

/// Each boot's accesses on `sources` ([`aethelgard_twin::serial::access`]),
/// in log order, each write a logged source's model says it ignored
/// ([`aethelgard_twin::serial::refused_by`]) marked refused; `None` when a
/// line of a logged source's own `INFO` log is not a whole access, or a
/// refusal follows no write of its source, so the writes cannot be rebuilt.
fn access_boots(
    boots: &[Vec<&str>],
    sources: &[&str],
) -> Option<Vec<Vec<aethelgard_twin::serial::Access>>> {
    boots
        .iter()
        .map(|lines| {
            let mut out: Vec<aethelgard_twin::serial::Access> = Vec::new();
            for body in lines.iter().filter_map(|l| log_body(l)) {
                if let Some(source) =
                    aethelgard_twin::serial::refused_by(body).filter(|s| sources.contains(s))
                {
                    let last = out.iter_mut().rev().find(|a| a.source == source)?;
                    if !last.write || last.refused {
                        return None;
                    }
                    last.refused = true;
                    continue;
                }
                if !sources
                    .iter()
                    .any(|s| aethelgard_twin::serial::logged_by(body, s))
                {
                    continue;
                }
                out.push(aethelgard_twin::serial::access(body)?);
            }
            Some(out)
        })
        .collect()
}

/// Each port's witness from what the run left: its capture file, its RX
/// pin's mux at its injection (`injection`, read by the first byte's
/// action), its TX pin's at the end (among the image's reads) and at the
/// first `TDR` write of each boot, and the writes of each boot
/// ([`access_boots`]; `None` when the log could not be read whole): its
/// USART's against the bring-up, the RCC's against its rule (enable registers only), its pins'
/// beyond their mux (`inject_at`: how many of the last boot's accesses
/// precede each port's injection). The echo is `not_judged` until the
/// caller knows the boot word; [`judge_boot`] sets it.
fn serial_witnesses(
    plan: &BootPlan<'_>,
    registers: &[u32],
    injection: &[PortInjection],
    capture: &Captured,
    boots: Option<&[Vec<aethelgard_twin::serial::Access>]>,
    inject_at: &[Option<usize>],
) -> Vec<SerialWitness> {
    let image = |address: u32| {
        plan.image
            .registers
            .iter()
            .position(|r| r.address == address)
            .and_then(|i| registers.get(i).copied())
    };
    plan.serial
        .iter()
        .enumerate()
        .map(|(k, s)| {
            let name = format!("{}.bin", s.port.renode);
            let bytes = capture
                .files
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, b)| b.clone());
            let parsed = bytes
                .as_deref()
                .map(|b| aethelgard_twin::serial::parse(b, &s.pattern, &s.frame));
            let rx_at_injection = injection
                .get(k)
                .filter(|j| j.at_ns.is_some())
                .and_then(|j| j.mux);
            let tx_at_end = image(s.tx.moder.1).zip(image(s.tx.afr.1));
            let at = inject_at
                .get(k)
                .copied()
                .flatten()
                .filter(|_| rx_at_injection.is_some());
            let rx_after_solved =
                boots.and_then(|b| aethelgard_twin::serial::rx_changed_after_solved(&s.rx, b));
            let tx_after_solved =
                boots.and_then(|b| aethelgard_twin::serial::tx_unrouted_after_solved(&s.tx, b));
            let echo = match (&parsed, rx_at_injection) {
                (Some(c), Some((m, a)))
                    if c.echoed == aethelgard_twin::serial::FRAME_LEN && !c.stray =>
                {
                    if s.rx.routed(m, a) && rx_after_solved.is_none() {
                        "witnessed"
                    } else {
                        "not_counted"
                    }
                }
                (Some(c), Some(_)) if c.echoed > 0 && !c.stray => "partial",
                _ => "missing",
            };
            // The first boot's mux that is not routed at its first `TDR`
            // write, else the mux that first left the AF once solved (ADV4-4),
            // else the last boot's; `None` when a boot's could not be rebuilt
            // or no boot wrote `TDR`.
            let mut tx_after_solved = tx_after_solved;
            let tx_at_send = boots.and_then(|b| {
                let sent =
                    aethelgard_twin::serial::tx_at_send(&s.tx, s.port.renode, s.bringup.tdr, b);
                match sent
                    .iter()
                    .find(|m| !m.is_some_and(|(m, a)| s.tx.routed(m, a)))
                {
                    Some(bad) => {
                        // Named at its first `TDR` write: the later change
                        // is the tags' alone.
                        tx_after_solved = None;
                        *bad
                    }
                    None => tx_after_solved.unwrap_or_else(|| sent.last().copied().flatten()),
                }
            });
            SerialWitness {
                bytes,
                capture: parsed,
                injected_ns: injection.get(k).and_then(|j| j.at_ns),
                injected: injection.get(k).map_or(0, |j| j.bytes),
                rx_at_injection,
                tx_at_end,
                tx_at_send,
                tx_after_solved,
                rx_after_solved,
                tags: boots.map(|b| {
                    use aethelgard_twin::serial::{judge_pins, judge_rcc, judge_writes};
                    let mut j = judge_writes(&s.bringup, s.port.renode, b);
                    let tdr = s.bringup.tdr;
                    for x in judge_rcc(&s.rcc, b).into_iter().chain(judge_pins(
                        &s.tx,
                        &s.rx,
                        s.port.renode,
                        tdr,
                        at,
                        b,
                    )) {
                        j.note(x);
                    }
                    j
                }),
                echo,
            }
        })
        .collect()
}

/// A capture directory holding anything but the stock USARTs' own regular
/// files, a file the CLI could not read whole, or a byte on a USART no port
/// names (adversarial F12: a pattern sent there and everything restored):
/// forged, never read past.
fn capture_findings(plan: &BootPlan<'_>, capture: &Captured) -> Vec<Finding> {
    let mut out: Vec<Finding> = capture
        .problems
        .iter()
        .map(|p| {
            Finding::plain(
                aethelgard_chaos::campaign::SERIAL_FORGED_FINDING,
                format!("the capture directory: {p}"),
            )
        })
        .collect();
    let stock = aethelgard_twin::serial::stock_uarts();
    for (name, bytes) in &capture.files {
        let port = plan
            .serial
            .iter()
            .any(|s| format!("{}.bin", s.port.renode) == *name);
        let backend = stock.iter().any(|u| format!("{u}.bin") == *name);
        if !backend {
            out.push(Finding::plain(
                aethelgard_chaos::campaign::SERIAL_FORGED_FINDING,
                format!(
                    "the capture directory holds `{}`, which no file backend writes",
                    name.escape_debug()
                ),
            ));
        } else if !port && !bytes.is_empty() {
            out.push(Finding::plain(
                aethelgard_chaos::campaign::SERIAL_FORGED_FINDING,
                format!(
                    "`{}` holds {} byte(s) ({}{}), sent on a USART no serial port of the topology names",
                    name.escape_debug(),
                    bytes.len(),
                    hex_bytes(&bytes[..bytes.len().min(16)]),
                    if bytes.len() > 16 { " …" } else { "" }
                ),
            ));
        }
    }
    out
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
                    c.label,
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
    let who = format!("{}, its {} step", who(c), c.label);
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

/// With no boot word, each read whose record says its wait expired is still
/// named, as the booted path names it (`AET-WO-3201`, `3100` F15): the
/// record is the firmware's own, so the timeout is never left for
/// attribution to find unnamed. Nothing else of the record is judged before
/// the fixed point.
fn timeouts_without_the_boot_word(
    checks: &[aethelgard_twin::boot::StepCheck],
    records: &[(u32, u64)],
) -> Vec<Finding> {
    use aethelgard_twin::boot::Outcome;
    checks
        .iter()
        .zip(records)
        .filter(|(c, r)| matches!(c.judge(r.0, r.1), Outcome::Timeout(_)))
        .filter_map(|(c, r)| identity_finding(c, r.0, r.1))
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
        _ => format!("{}, its {} step", who(c), c.label),
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
        // Named by its step (`AET-WO-4210` F-B10): a data read's mismatch
        // is not the identity's.
        Outcome::Mismatch => Some(Finding::plain(
            &format!("{bus}_{}_mismatch", c.step.name()),
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

/// The instant the undisturbed boot wrote its boot word (`AET-WO-2305`
/// §1.2): the droop's own prepared boot, run first with no reset, its hook's
/// count from 0, which is exact (count × 10 ns at the twin's MIPS). `None`
/// when that boot was killed at a bound, when its log could not be read
/// whole, or when it wrote none: no bound then, which the judge names. An
/// output that cannot be trusted is a `Stop`, as for any boot.
pub fn undisturbed_boot_word_ns(out: &Output) -> Result<Option<u64>, Stop> {
    let code = match out.ended {
        crate::run::Ended::Bounded(_) => return Ok(None),
        _ => out.code(TOOL)?,
    };
    let text = out.text();
    let untrusted = |what: String| {
        Stop::tool_error(
            TOOL,
            format!(
                "the undisturbed boot: {what}{}",
                said::suffix(&text, Style::Renode)
            ),
        )
    };
    if code != 0 {
        return Err(untrusted(format!("renode exited {code}")));
    }
    let lines = clean_lines(&text);
    let begins: Vec<usize> = (0..lines.len()).filter(|&i| lines[i] == BEGIN).collect();
    let &[begin] = begins.as_slice() else {
        return Err(untrusted(format!(
            "renode printed {} begin markers, not one",
            begins.len()
        )));
    };
    if out.dropped_log_lines > 0 {
        return Ok(None);
    }
    let (hooks, _) = hooks_of(&lines[..begin], 0).map_err(untrusted)?;
    Ok(hooks.iter().find_map(|(_, h, ns)| match h {
        crate::anchor::Hook::BootWord { value, before, .. }
            if *value == aethelgard_twin::boot::BOOT_MAGIC
                && *before != aethelgard_twin::boot::BOOT_MAGIC =>
        {
            Some(*ns)
        }
        _ => None,
    }))
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
    let released = sentinels(&lines, RELEASE, releases.len()).map_err(untrusted)?;
    let n = plan.probe_count();
    let begins = sentinels(&lines, PROBE_BEGIN, n).map_err(untrusted)?;
    let ends = sentinels(&lines, PROBE_END, n).map_err(untrusted)?;
    let Some(bound) = plan.recovery_bound_ns else {
        return Ok(Vec::new());
    };
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
            at_ns: power::probe_ns(*release, bound)
                .ok_or_else(|| untrusted(format!("probe {k}: its instant overflows")))?,
            boot_word: words[0],
            record: check.map(|_| (words[1], words[2])),
            witness,
        });
    }
    Ok(out)
}

/// The build `renode --version` names on its `build:` line, as Renode prints
/// it (`1.17.0+20260907gitf1dd1b4af.aet1605` in the pinned image): `None` unless
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
        Outcome::Timeout("EOT") => "timeout_eot",
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
            label: "identity",
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
            r#"{"schema_version":11,"specification_sha256":"acd60081f2d111b1b1ed0d974078fff45ccf2a967ec5f42317417f8ce68149d1","assignments":{"I2C_SCL":{"pin":"PB6","peripheral":"I2C1","signal":"I2C1_SCL","alternate_function":4},"I2C_SDA":{"pin":"PB7","peripheral":"I2C1","signal":"I2C1_SDA","alternate_function":4}},"gpio":{},"i2c_ceilings":{"SENSORS":400000},"i2c_speeds":{"SENSORS":400000},"analog_lines":{"I2C_SCL":{"bus":"SENSORS","pull_up_ohms":2200,"capacitance_femtofarads":100000,"ceiling_ns":300,"domain_millivolts":3300},"I2C_SDA":{"bus":"SENSORS","pull_up_ohms":2200,"capacitance_femtofarads":100000,"ceiling_ns":300,"domain_millivolts":3300}},"i2c_devices":{"U2":{"index":0,"part":"BME280","bus":"SENSORS","peripheral":"I2C1","address":118},"U3":{"index":1,"part":"TMP117","bus":"SENSORS","peripheral":"I2C1","address":72}},"spi_devices":{},"spi_lines":{},"clock":{"source":"HSI","sysclk_hz":64000000,"hse_hz":0,"d1cpre":1,"hpre":1,"d1ppre":1,"d2ppre1":1,"d2ppre2":1,"d3ppre":1,"timpre":false},"motor_outputs":{},"power":null,"serial_ports":{},"spi_controllers":{},"i2c_timings":{"I2C1":{"kernel_hz":64000000,"cr1":0,"timingr":{"Fast400k":281481788,"FastPlus1M":11669037,"Standard100k":1356016192}}},"bor_level":0,"mcu":"STM32H753ZITx","pin_speeds":{"I2C_SCL":0,"I2C_SDA":0},"foreign_clock":null}"#,
        )
        .unwrap();
        aethelgard_twin::boot::expected(&topology).unwrap()
    }

    /// Each port's injection `port_injections` reads from `log`, a boot of
    /// no sleep whose first boot word is the chain's anchor.
    fn injections(p: &BootPlan<'_>, log: &[String]) -> Vec<PortInjection> {
        let lines: Vec<&str> = log.iter().map(String::as_str).collect();
        let (hooks, slept) = hooks_of(&lines, 0).unwrap();
        assert_eq!(slept, None);
        let bw = hooks.iter().find_map(|(i, h, ns)| match h {
            crate::anchor::Hook::BootWord { .. } => Some((*i, *ns)),
            _ => None,
        });
        port_injections(p, &hooks, bw)
    }

    /// A tool run that ended with code 0 and printed `stdout`, `dropped`
    /// log lines lost.
    fn output(stdout: Vec<u8>, dropped: u64) -> Output {
        Output {
            ended: crate::run::Ended::Code(0),
            stdout,
            stderr: Vec::new(),
            sandbox: crate::run::SandboxReport {
                landlock_abi: 0,
                tcp_confined: false,
                scoped: false,
                sockets_denied: true,
                uid_boundary: false,
                uid_boundary_unmet: None,
            },
            dropped_log_lines: dropped,
            peak_rss_kib: None,
            summaries: Vec::new(),
        }
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
            models: &[],
            firmware: std::path::Path::new("/w/firmware.elf"),
            virtual_ns: 100_000_000,
            boot_word: 0x2000_0000,
            vector_table: 0x0800_0000,
            image,
            executable: &[],
            self_loops: &[],
            motors: &[],
            instances: &INSTANCES,
            resets,
            // Each probe at release + 100 µs, as these tests were written:
            // they judge the probe's reading, not the bound's derivation
            // (`AET-WO-2305` §1.2, tested in `aethelgard-chaos`).
            recovery_bound_ns: Some(100_000 - power::NUDGE_NS),
            serial: &[],
            capture: None,
        }
    }

    /// `AET-WO-4400g` §1.6, `INV-07` residue (a)(3): under a declared clock
    /// the DWT keeps the stock file's 250 MHz (a condition the foreign
    /// verdict states), so no judge may read it. Neither boot's commands
    /// name the DWT or an address of its block (`0xE0001000`, 4 KiB, ARMv7-M
    /// ARM DDI 0403E.b §C1.8).
    #[test]
    fn no_judge_reads_the_dwt() {
        let image = image();
        let p = plan(&image, &[]);
        for c in [p.commands(), foreign::commands(&p, &[]).unwrap()] {
            assert!(!c.to_ascii_lowercase().contains("dwt"), "{c}");
            for word in c.split(|ch: char| !ch.is_ascii_alphanumeric()) {
                if let Some(hex) = word.strip_prefix("0x") {
                    let a = u64::from_str_radix(hex, 16).unwrap_or(0);
                    assert!(!(0xE000_1000..0xE000_2000).contains(&a), "{word} in {c}");
                }
            }
        }
    }

    /// `AET-WO-4220` D6: each model of our own is included from the CLI's own
    /// copy before the platform loads, in a boot and in a load alike.
    #[test]
    fn models_of_our_own_are_included_before_the_platform_loads() {
        let image = image();
        let models = [
            std::path::PathBuf::from("/w/models/BMP581.cs"),
            std::path::PathBuf::from("/w/models/ICM42688P.cs"),
        ];
        let mut p = plan(&image, &[]);
        p.models = &models;
        let c = p.commands();
        let at = |what: &str| c.find(what).unwrap_or_else(|| panic!("{what} not in {c}"));
        assert!(at("include @/w/models/BMP581.cs") < at("include @/w/models/ICM42688P.cs"));
        assert!(at("include @/w/models/ICM42688P.cs") < at("mach create"));
        assert!(at("mach create") < at("machine LoadPlatformDescription"));
        assert!(!plan(&image, &[]).commands().contains("include"));
        assert_eq!(
            commands(std::path::Path::new("/w/p.repl"), &models),
            "include @/w/models/BMP581.cs; include @/w/models/ICM42688P.cs; mach create; machine LoadPlatformDescription @/w/p.repl; peripherals; quit"
        );
        assert_eq!(
            commands(std::path::Path::new("/w/p.repl"), &[]),
            "mach create; machine LoadPlatformDescription @/w/p.repl; peripherals; quit"
        );
    }

    /// `AET-WO-4220` F2, D6: a model that does not compile stops Renode's
    /// line with exit 0 and no marker; the CLI refuses it as
    /// `ModelLoadFailed`, carrying the diagnostics, in a boot and in a load.
    #[test]
    fn a_model_that_does_not_compile_is_model_load_failed() {
        // The spike's measured text (`AET-WO-4220` P1.5).
        let text = "Errors during compilation or loading:\n\
                    Could not compile assembly from: /w/models/BMP581.cs\n\
                    /w/models/BMP581.cs(13,27): error CS1002: ; expected\n\
                    [INFO] machine-0: Disposed.\n";
        let out = output(text.as_bytes().to_vec(), 0);
        let image = image();
        for stop in [
            judge_boot(0, &out, &plan(&image, &[]), &Captured::default())
                .err()
                .unwrap(),
            judge(0, &out).err().unwrap(),
        ] {
            assert_eq!(stop.kind, "ModelLoadFailed");
            assert!(
                stop.message.contains("error CS1002: ; expected"),
                "{}",
                stop.message
            );
        }
        assert!(LOG_FILTER.keep.contains(&MODEL_LOAD_ERROR));
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

    /// The PWM fixture's first output on a timer built at 250 MHz, the
    /// stock file's figure: the model's 639,744 ns period and 256,000 ns
    /// high time (PSC 63, ARR 2499, CCR 1000 at 4 ns a tick). The judge's
    /// mechanics read the observer's period, not a clock, so these tests'
    /// logs keep the 250 MHz spacing they were written at (`AET-WO-4400f`:
    /// they do not move under the realized period).
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
                kernel_hz: 250_000_000,
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
        assert!(witness_findings_of(&m, &w).is_empty());
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
        let f = witness_findings_of(&m, &w);
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
            witness_findings_of(&m, &w)[0].kind,
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
        let f = witness_findings_of(&m, &w);
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
        assert_eq!(witness_findings_of(&m, &w)[0].kind, "motor_timing_mismatch");
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

    /// The reference topology with its console (`AET-WO-4300b`): USART1 on
    /// PA9/PA10 at AF7, 115200 8N1.
    fn console_topology() -> aethelgard_ir::SolvedTopology {
        serde_json::from_str(CONSOLE_TOPOLOGY).unwrap()
    }

    const CONSOLE_TOPOLOGY: &str = r#"{"schema_version":11,"specification_sha256":"c1072b716ed55a19167f94225fb387617c643e5a5ba79fd59db4d12924752e3e","assignments":{"CONSOLE_RX":{"pin":"PA10","peripheral":"USART1","signal":"USART1_RX","alternate_function":7},"CONSOLE_TX":{"pin":"PA9","peripheral":"USART1","signal":"USART1_TX","alternate_function":7},"I2C_SCL":{"pin":"PB6","peripheral":"I2C1","signal":"I2C1_SCL","alternate_function":4},"I2C_SDA":{"pin":"PB7","peripheral":"I2C1","signal":"I2C1_SDA","alternate_function":4},"SPI_MISO":{"pin":"PA6","peripheral":"SPI1","signal":"SPI1_MISO","alternate_function":5},"SPI_MOSI":{"pin":"PA7","peripheral":"SPI1","signal":"SPI1_MOSI","alternate_function":5},"SPI_SCK":{"pin":"PA5","peripheral":"SPI1","signal":"SPI1_SCK","alternate_function":5}},"gpio":{"FLASH_CS":"PA0"},"i2c_ceilings":{"SENSORS":400000},"i2c_speeds":{"SENSORS":400000},"analog_lines":{"I2C_SCL":{"bus":"SENSORS","pull_up_ohms":2200,"capacitance_femtofarads":100000,"ceiling_ns":300,"domain_millivolts":3300},"I2C_SDA":{"bus":"SENSORS","pull_up_ohms":2200,"capacitance_femtofarads":100000,"ceiling_ns":300,"domain_millivolts":3300}},"i2c_devices":{"U2":{"index":0,"part":"BME280","bus":"SENSORS","peripheral":"I2C1","address":118},"U3":{"index":1,"part":"TMP117","bus":"SENSORS","peripheral":"I2C1","address":72}},"spi_devices":{"U4":{"index":0,"part":"W25Q64JVxIQ","bus":"FLASH","peripheral":"SPI1","cs":"FLASH_CS"}},"spi_lines":{"SPI_MOSI":{"bus":"FLASH","signal":"MOSI","rate_hz":20000000,"capacitance_femtofarads":30000,"rise_time_ns":20,"vdd_min_millivolts":3135},"SPI_SCK":{"bus":"FLASH","signal":"SCK","rate_hz":20000000,"capacitance_femtofarads":30000,"rise_time_ns":20,"vdd_min_millivolts":3135}},"clock":{"source":"HSI","sysclk_hz":64000000,"hse_hz":0,"d1cpre":1,"hpre":1,"d1ppre":1,"d2ppre1":1,"d2ppre2":1,"d3ppre":1,"timpre":false},"motor_outputs":{},"power":null,"serial_ports":{"CONSOLE":{"index":0,"peripheral":"USART1","tx":"CONSOLE_TX","rx":"CONSOLE_RX","role":"console","framing":{"baud":115200,"data_bits":8,"parity":"none","stop_bits":1,"inverted":false},"negotiated":null,"peer_deviation_ppm":5000,"kernel_hz":64000000,"brr":556,"actual_millibaud":115107913,"error_ppm":-799}},"spi_controllers":{"SPI1":{"kernel":"per_ck","kernel_hz":64000000,"declared_hz":20000000,"mbr":1,"achieved_hz":16000000,"achieved_max_hz":16239744}},"i2c_timings":{"I2C1":{"kernel_hz":64000000,"cr1":0,"timingr":{"Fast400k":281481788,"FastPlus1M":11669037,"Standard100k":1356016192}}},"bor_level":3,"mcu":"STM32H753ZITx","pin_speeds":{"CONSOLE_RX":0,"CONSOLE_TX":0,"FLASH_CS":0,"I2C_SCL":0,"I2C_SDA":0,"SPI_MISO":0,"SPI_MOSI":1,"SPI_SCK":1},"foreign_clock":null}"#;

    /// The console's expectations for a firmware whose sha256 is all `0x11`.
    fn console() -> Vec<Serial> {
        serials(&console_topology(), &[0x11; 32]).unwrap()
    }

    fn serial_plan<'a>(
        image: &'a aethelgard_twin::boot::Image,
        resets: &'a [power::Reset],
        serial: &'a [Serial],
    ) -> BootPlan<'a> {
        BootPlan {
            serial,
            capture: Some(std::path::Path::new("/w/capture")),
            ..plan(image, resets)
        }
    }

    /// `AET-WO-4300b`: one file backend per port before anything runs.
    /// `AET-WO-2302` D4: the frame goes in by the CLI's own hooks, one
    /// quantum after the boot word, the RX pin's mux read in the first
    /// byte's action; the boot is one `RunFor` of `MAX_VIRTUAL_NS`; with a
    /// droop the boot word's hook is added at the last release, before the
    /// core runs again, and never without a released last reset.
    #[test]
    fn a_serial_boot_captures_first_and_injects_from_the_boot_word() {
        let t = console_topology();
        let image = aethelgard_twin::boot::expected(&t).unwrap();
        let serial = console();
        let p = serial_plan(&image, &[], &serial);
        // The earliest instant, for the static refusals.
        assert_eq!(p.injection_ns(), Some(100_000));
        let c = p.commands();
        // The access log on the port's USART, its pins' GPIO port and the RCC
        // (adversarial pass 2, A1), then a backend on every stock USART, the
        // port's last (adversarial F12), then the boot word's hook, then the
        // whole boot.
        let others: String = [
            "usart2", "usart3", "uart4", "uart5", "usart6", "uart7", "uart8", "lpuart1",
        ]
        .iter()
        .map(|u| format!("sysbus.{u} CreateFileBackend @/w/capture/{u}.bin true; "))
        .collect();
        let hook = crate::anchor::boot_word_hook(0x2000_0000, &p.port_hooks());
        // `AET-WO-3305`: then the one I2C step's status hook.
        let step = crate::anchor::step_hook(0, image.identity.checks[0].status_at);
        assert!(
            c.contains(&format!(
                "logLevel -1 sysbus.i2c1.u2; logLevel -1 sysbus.spi1.spi1mux; sysbus LogPeripheralAccess sysbus.usart1 true; sysbus LogPeripheralAccess sysbus.gpioPortA true; sysbus LogPeripheralAccess sysbus.rcc true; {others}sysbus.usart1 CreateFileBackend @/w/capture/usart1.bin true; {hook}; {step}; emulation RunFor \"0.1000000005\"; echo \"AETHELGARD-BOOT-BEGIN\""
            )),
            "{c}"
        );
        assert_eq!(c.matches("CreateFileBackend").count(), 9);
        // The three, and the flash controller's on every boot (`AET-WO-4230`).
        assert_eq!(c.matches("LogPeripheralAccess").count(), 4);
        // The frame's bytes, each scheduled from the boot word's hook at a
        // quantum and a character apart, rounded up to a quantum
        // (adversarial pass 4, ADV4-2: 86 876 ns at 115 200 8N1).
        assert_eq!(c.matches("WriteChar").count(), 8);
        for (j, b) in serial[0].frame.iter().enumerate() {
            let us = 100 + 100 * j;
            assert!(c.contains(&format!("FromMicroseconds({us}), ")), "{j}");
            let start = if j == 0 {
                ", __import__('sys').aethelgard_echo.__setitem__(0, 0)"
            } else {
                ""
            };
            assert!(
                c.contains(&format!(
                    "sysbus.usart1 WriteChar {b:#04x}'){start}) if cpu.ExecutedInstructions == a + {}",
                    us * 100
                )),
                "{j}"
            );
        }
        assert!(c.contains("self.ReadDoubleWord(0x58020000), self.ReadDoubleWord(0x58020024)"));
        assert!(!c.contains("INJECT-BEGIN"));
        assert_eq!(c.matches("emulation RunFor").count(), 1);
        let resets = bor3();
        let p = serial_plan(&image, &resets, &serial);
        assert_eq!(p.injection_ns(), Some(1_227_980 + 10 + 100_000));
        assert_eq!(p.last_start(), Some(1_227_990));
        let c = p.commands();
        // The reset's cold writes are the judge's: the RCC's log is off
        // around them, and on again before the core runs.
        let cold = p.cold().join("; ");
        assert!(
            c.contains(&format!(
                "machine Reset; sysbus LogPeripheralAccess sysbus.rcc false; {cold}; sysbus LogPeripheralAccess sysbus.rcc true; cpu VectorTableOffset"
            )),
            "{c}"
        );
        // The boot word's hook once, at the last release, before the core
        // runs again: a hook on RAM is not fired after a reset, and added
        // twice fires twice (measured, build probes P3, P4).
        assert_eq!(c.matches("AddWatchpointHook 0x20000000").count(), 1);
        assert!(
            c.contains(&format!(
                "emulation RunFor \"0.0002135505\"; {hook}; {step}; cpu IsHalted false; echo \"AETHELGARD-RELEASE-0\""
            )),
            "{c}"
        );
        // Without a serial port only the flash controller's is on
        // (`AET-WO-4230` D10), and no reset turns it off.
        let no_serial = plan(&image, &resets).commands();
        assert_eq!(no_serial.matches("LogPeripheralAccess").count(), 1);
        assert!(no_serial.contains(FLASH_LOG));
        // A reset never released: no boot follows it, and no hook on RAM
        // goes in.
        let held = vec![power::Reset {
            asserted_ns: 1_014_430,
            released_ns: None,
            cause: vec!["VBOR3".to_owned()],
        }];
        let p = serial_plan(&image, &held, &serial);
        assert_eq!((p.injection_ns(), p.last_start()), (None, None));
        assert!(!p.commands().contains("AddWatchpointHook"));
        // A board without a port: no capture, no injection; the boot word's
        // hook only logs, beside its one I2C step's (`AET-WO-3305`).
        let p = plan(&image, &[]);
        assert_eq!(p.injection_ns(), None);
        let c = p.commands();
        assert_eq!(c.matches("AddWatchpointHook").count(), 2);
        assert_eq!(c.matches("AETHELGARD-STEP").count(), 1);
        assert!(!c.contains("WriteChar") && !c.contains("ScheduleAction"));
    }

    /// `AET-WO-2302` D4, sequential: the boot word schedules the first
    /// port's frame alone; each later port goes in a quantum after the port
    /// before has echoed its `FRAME_LEN`-th byte, counted by a hook on that
    /// port's `TDR`, each byte at its own spacing. A spacing that cannot be
    /// computed injects nothing.
    #[test]
    fn a_later_port_goes_in_once_the_port_before_has_echoed() {
        let t = console_topology();
        let image = aethelgard_twin::boot::expected(&t).unwrap();
        let mut serial = console();
        let mut slow = serial[0].clone();
        slow.port.label = "SLOW".to_owned();
        slow.port.renode = "usart2";
        slow.tdr_at = 0x4000_4428;
        slow.frame = [0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7];
        slow.spacing_ns = Some(1_100_000);
        serial.push(slow);
        assert_eq!(serial[0].tdr_at, 0x4001_1028);
        let p = serial_plan(&image, &[], &serial);
        let c = p.commands();
        let bw = crate::anchor::boot_word_hook(0x2000_0000, &p.port_hooks());
        let echo = crate::anchor::echo_hook(0, &p.port_hooks(), 0x2000_0000).unwrap();
        let step = crate::anchor::step_hook(0, image.identity.checks[0].status_at);
        assert!(
            c.contains(&format!(
                "{bw}; {echo}; {step}; emulation RunFor \"0.1000000005\""
            )),
            "{c}"
        );
        assert!(!bw.contains("usart2"));
        assert!(echo.starts_with("sysbus AddWatchpointHook 0x40011028 DoubleWord Write"));
        for j in 0..8u64 {
            let us = 100 + 1100 * j;
            assert!(echo.contains(&format!("FromMicroseconds({us}), ")), "{j}");
        }
        assert!(echo.contains("sysbus.usart2 WriteChar 0xa7"));
        assert_eq!(c.matches("WriteChar").count(), 16);
        // The last port's TDR is not hooked.
        assert!(!c.contains("AddWatchpointHook 0x40004428"));
        serial[1].spacing_ns = None;
        let p = serial_plan(&image, &[], &serial);
        assert!(p.port_hooks().is_empty());
        assert!(!p.commands().contains("WriteChar"));
    }

    /// `AET-WO-2302`: each motor timer's `CR1` is hooked before the run, a
    /// DShot timer's with the judge's stop and a PWM timer's only logged.
    /// The observers' level is set once, before the run, and never changed:
    /// the stop silences nothing (adversarial pass 1, H1), so a reset sets
    /// no level again.
    #[test]
    fn each_motor_timer_is_hooked_and_its_observers_never_silenced() {
        let image = image();
        let pwm = motor();
        let mut dshot = motor();
        dshot.observer.protocol = aethelgard_ir::MotorProtocol::Dshot600;
        dshot.observer.timer = 2;
        dshot.observer.source = "gpioPortA.pa9obs".into();
        let mut regs = image.clone();
        for (n, at) in [(1u32, 0x4001_0000u32), (2, 0x4000_0000)] {
            for (r, off) in [("CR1", 0), ("ARR", 0x2c)] {
                regs.registers.push(aethelgard_twin::boot::Register {
                    name: format!("TIM{n}.{r}"),
                    address: at + off,
                    mask: 0xFF,
                    value: 0x81,
                });
            }
        }
        let motors = [pwm, dshot];
        let resets = bor3();
        let p = BootPlan {
            motors: &motors,
            ..plan(&regs, &resets)
        };
        let hooks = p.timer_hooks();
        assert_eq!(
            hooks
                .iter()
                .map(|t| (t.timer, t.cr1, t.dshot, t.registers.len()))
                .collect::<Vec<_>>(),
            [(1, 0x4001_0000, false, 2), (2, 0x4000_0000, true, 2)]
        );
        let c = p.commands();
        let pwm_hooks = crate::anchor::timer_hooks(&hooks[0]);
        let dshot_hooks = crate::anchor::timer_hooks(&hooks[1]);
        assert!(pwm_hooks.iter().all(|h| !h.contains("ScheduleAction")));
        assert!(
            dshot_hooks
                .iter()
                .all(|h| h.contains("ScheduleAction") && !h.contains("logLevel"))
        );
        // Before the first `RunFor`, once each, every width: a peripheral's
        // hook survives `machine Reset` (measured, F5 and build probe P3).
        let first = c.find("emulation RunFor").unwrap();
        for h in pwm_hooks.iter().chain(&dshot_hooks) {
            assert_eq!(c.matches(h.as_str()).count(), 1);
            assert!(c.find(h.as_str()).unwrap() < first);
        }
        let reset = c.split("machine Reset").nth(1).unwrap();
        let reset = reset.split("cpu IsHalted false").next().unwrap();
        assert!(!reset.contains("logLevel"), "{reset}");
        assert!(!c.contains("logLevel 3"), "{c}");
        assert_eq!(c.matches("logLevel -1 sysbus.gpioPortA.pa9obs").count(), 1);
    }

    // ------------------------------------------------------- AET-WO-2302

    /// The reference image with TIM2's `CR1` and `ARR` added (both 0x81
    /// under 0xFF), and one DShot600 output on TIM2 at the model's 424 ns
    /// period and 160 ns high time.
    fn dshot_parts() -> (aethelgard_twin::boot::Image, Vec<Motor>) {
        let mut image = image();
        for (r, at) in [("CR1", 0x4000_0000), ("ARR", 0x4000_002c)] {
            image.registers.push(aethelgard_twin::boot::Register {
                name: format!("TIM2.{r}"),
                address: at,
                mask: 0xFF,
                value: 0x81,
            });
        }
        let mut m = motor();
        m.observer.protocol = aethelgard_ir::MotorProtocol::Dshot600;
        m.observer.timer = 2;
        m.observer.pin = "PA9".into();
        m.observer.source = "gpioPortA.pa9obs".into();
        (m.observer.period_ns, m.observer.high_ns) = (424, 160);
        (image, vec![m])
    }

    fn at(ns: u64, body: &str) -> String {
        format!("[00:00:00.{:09}] {body}", ns)
    }

    /// A DShot boot as Renode logs it (build probe P5's shape): U2's
    /// identity, `CEN` at instruction 3067, the boot word at 3075, the edges
    /// from 490 ns after the anchor to the window's end, and the judge's stop
    /// 200 000 instructions after the anchor.
    fn dshot_log() -> Vec<String> {
        let mut log = vec![
            at(10_000, "[NOISY] i2c1.u2: Write D0"),
            at(10_000, "[NOISY] i2c1.u2: Read 60"),
            at(30_620, "[WARNING] sysbus: AETHELGARD-CR1 2 81 0 3067"),
            at(
                30_750,
                "[WARNING] sysbus: AETHELGARD-BOOTWORD 2100b007 0 3075",
            ),
        ];
        for k in 0..4716u64 {
            let rise = 30_670 + 490 + k * 424;
            log.push(at(
                rise,
                "[NOISY] gpioPortA.pa9obs: LED state changed to True",
            ));
            log.push(at(
                rise + 160,
                "[NOISY] gpioPortA.pa9obs: LED state changed to False",
            ));
        }
        log.push(at(
            2_030_670,
            "[WARNING] sysbus: AETHELGARD-STOP 2 3067 203067 81 81",
        ));
        log.push(at(
            2_030_670,
            "[WARNING] sysbus: AETHELGARD-CLEARED 2 3067 203067 80",
        ));
        log
    }

    /// The judge's anchor findings on `log`, a boot started at `start_ns`
    /// and read to 100 ms, and the output's witness; a line of ours its
    /// count does not place at its stamp adds `cpu_halted`, as the judge's
    /// boot does.
    fn judge_anchors(
        log: &[String],
        start_ns: u64,
    ) -> Result<(Vec<Finding>, MotorWitness), String> {
        let (image, motors) = dshot_parts();
        let p = BootPlan {
            motors: &motors,
            ..plan(&image, &[])
        };
        let lines: Vec<&str> = log.iter().map(String::as_str).collect();
        let (hooks, slept) = hooks_of(&lines, start_ns)?;
        let timers = timer_witnesses(&p, &hooks, p.virtual_ns);
        let w = witness_motor_in(
            &motors[0],
            Some(observer_edges(&motors[0].observer.source, &lines)),
            timers.first(),
            p.virtual_ns,
        );
        let last_device = image
            .identity
            .checks
            .iter()
            .flat_map(|c| own_lines(c, &lines))
            .map(|(i, _)| i)
            .max();
        let bw = hooks.iter().find_map(|(i, h, ns)| match h {
            crate::anchor::Hook::BootWord { value, before, .. }
                if *value == aethelgard_twin::boot::BOOT_MAGIC && *before == 0 =>
            {
                Some((*i, *ns))
            }
            _ => None,
        });
        let mut f: Vec<Finding> = slept
            .into_iter()
            .map(|s| Finding::plain("cpu_halted", s))
            .collect();
        f.extend(witness_findings_of(&motors[0], &w));
        f.extend(anchor_findings(
            &p,
            &timers,
            std::slice::from_ref(&w),
            last_device,
            bw,
        ));
        Ok((f, w))
    }

    fn kinds_of(f: &[Finding]) -> Vec<&str> {
        f.iter().map(|f| f.kind.as_str()).collect()
    }

    fn messages(f: &[Finding]) -> Vec<&str> {
        f.iter().map(|f| f.message.as_str()).collect()
    }

    /// `AET-WO-2302` (a-i): the DShot window is the anchor plus
    /// `DSHOT_WINDOW_NS`, its edges judged inside it, its stop read back.
    #[test]
    fn a_dshot_output_is_witnessed_inside_its_anchored_window() {
        let (f, w) = judge_anchors(&dshot_log(), 0).unwrap();
        assert_eq!(kinds_of(&f), Vec::<&str>::new(), "{:?}", messages(&f));
        assert_eq!(w.status, "witnessed");
        assert_eq!((w.anchor_ns, w.end_ns), (Some(30_670), 2_030_670));
        assert_eq!((w.rises, w.edges_before, w.edges_after), (4716, 0, 0));
    }

    /// Each bound fails by name: the anchor once, after the last device
    /// step's witness and before the boot word, observed, its window inside
    /// the run, its stop run and read back, and every edge inside it.
    #[test]
    fn each_anchor_bound_fails_by_name() {
        let kinds = |log: Vec<String>| {
            let (f, _) = judge_anchors(&log, 0).unwrap();
            f.into_iter().map(|f| f.kind).collect::<Vec<_>>()
        };
        let base = dshot_log();
        let with = |f: &dyn Fn(&mut Vec<String>)| {
            let mut l = base.clone();
            f(&mut l);
            l
        };
        // An edge after the window's end: the stop silences nothing, so it
        // is the firmware's own.
        assert_eq!(
            kinds(with(&|l| l.push(at(
                2_030_700,
                "[NOISY] gpioPortA.pa9obs: LED state changed to True"
            )))),
            ["motor_edge_after_window"]
        );
        // An edge before the anchor, in the same boot (the firmware emits
        // before its anchor, WO §5).
        assert_eq!(
            kinds(with(&|l| l.insert(
                2,
                at(
                    20_000,
                    "[NOISY] gpioPortA.pa9obs: LED state changed to True"
                )
            ))),
            ["motor_edge_before_anchor"]
        );
        // A second rise of CEN in one boot.
        assert_eq!(
            kinds(with(&|l| l.push(at(
                2_500_000,
                "[WARNING] sysbus: AETHELGARD-CR1 2 81 80 250000"
            )))),
            ["motor_anchor_repeated"]
        );
        // The anchor before the last device step's witness, after the boot
        // word, or with no boot word observed.
        assert_eq!(
            kinds(with(&|l| l.swap(0, 2))),
            ["motor_anchor_out_of_order"]
        );
        assert_eq!(
            kinds(with(&|l| l.swap(2, 3))),
            ["motor_anchor_out_of_order"]
        );
        assert_eq!(
            kinds(with(&|l| {
                l.remove(3);
            })),
            ["motor_anchor_out_of_order"]
        );
        // No stop in the boot, a stop that left CEN set, a stop's read-back
        // off the solved values.
        let no_stop = with(&|l| {
            l.truncate(l.len() - 2);
        });
        assert_eq!(kinds(no_stop), ["judge_stop_missing"]);
        assert_eq!(
            kinds(with(&|l| {
                let n = l.len();
                l[n - 1] = at(
                    2_030_670,
                    "[WARNING] sysbus: AETHELGARD-CLEARED 2 3067 203067 81",
                );
            })),
            ["judge_stop_missing"]
        );
        assert_eq!(
            kinds(with(&|l| {
                let n = l.len();
                l[n - 2] = at(
                    2_030_670,
                    "[WARNING] sysbus: AETHELGARD-STOP 2 3067 203067 81 82",
                );
            })),
            ["judge_stop_mismatch"]
        );
        // A stop that read fewer words back than its timer has registers,
        // and a stop of another anchor than the timer's (no stop of its own).
        assert_eq!(
            kinds(with(&|l| {
                let n = l.len();
                l[n - 2] = at(
                    2_030_670,
                    "[WARNING] sysbus: AETHELGARD-STOP 2 3067 203067 81",
                );
            })),
            ["judge_stop_mismatch"]
        );
        assert_eq!(
            kinds(with(&|l| {
                let n = l.len();
                l[n - 2] = at(
                    2_030_670,
                    "[WARNING] sysbus: AETHELGARD-STOP 2 3066 203067 81 81",
                );
            })),
            ["judge_stop_missing"]
        );
        // Edges with no anchor observed (a CR1 written by DMA).
        let (f, w) = judge_anchors(
            &with(&|l| {
                l.remove(2);
            }),
            0,
        )
        .unwrap();
        assert_eq!(w.status, "unanchored");
        assert_eq!(kinds_of(&f), ["motor_anchor_unobserved"]);
    }

    /// F2's late anchor (`AET-WO-2302` §5): `start_outputs` delayed to
    /// 99.07 ms; 0.93 ms of DShot would pass a 100 ms window, but the
    /// anchored window ends past the run, named, and the output stopped
    /// short of its window's end.
    #[test]
    fn f2_a_late_anchor_fails_by_name() {
        let mut log = vec![
            at(10_000, "[NOISY] i2c1.u2: Write D0"),
            at(10_000, "[NOISY] i2c1.u2: Read 60"),
            at(
                99_072_300,
                "[WARNING] sysbus: AETHELGARD-CR1 2 81 0 9907235",
            ),
            at(
                99_072_430,
                "[WARNING] sysbus: AETHELGARD-BOOTWORD 2100b007 0 9907243",
            ),
        ];
        for k in 0..2187u64 {
            let rise = 99_072_350 + 490 + k * 424;
            log.push(at(
                rise,
                "[NOISY] gpioPortA.pa9obs: LED state changed to True",
            ));
            log.push(at(
                rise + 160,
                "[NOISY] gpioPortA.pa9obs: LED state changed to False",
            ));
        }
        let (f, w) = judge_anchors(&log, 0).unwrap();
        assert_eq!(w.end_ns, 99_072_350 + 2_000_000);
        assert_eq!(
            kinds_of(&f),
            ["motor_timing_mismatch", "motor_window_past_run"],
            "{:?}",
            messages(&f)
        );
    }

    /// F5 (`AET-WO-2302` §5): a reset restarts the count, and the earlier
    /// boot's stop fires in the later one, stale, doing nothing; the boot
    /// the judge reads is anchored and stopped by its own.
    #[test]
    fn f5_a_stale_stop_of_an_earlier_boot_does_nothing() {
        let start = 331_700;
        let shift = |ns: u64| ns + start;
        let mut log = vec![
            at(shift(10_000), "[NOISY] i2c1.u2: Write D0"),
            at(shift(10_000), "[NOISY] i2c1.u2: Read 60"),
            at(
                shift(21_620),
                "[WARNING] sysbus: AETHELGARD-CR1 2 81 0 2167",
            ),
            at(
                shift(21_750),
                "[WARNING] sysbus: AETHELGARD-BOOTWORD 2100b007 0 2175",
            ),
        ];
        for k in 0..4716u64 {
            let rise = shift(21_670 + 490 + k * 424);
            log.push(at(
                rise,
                "[NOISY] gpioPortA.pa9obs: LED state changed to True",
            ));
            log.push(at(
                rise + 160,
                "[NOISY] gpioPortA.pa9obs: LED state changed to False",
            ));
        }
        // Measured: due at 202 167 instructions, it fired at 168 997.
        let stale = at(
            2_021_670,
            "[WARNING] sysbus: AETHELGARD-STALE STOP 2 2167 168997",
        );
        let i = log
            .iter()
            .position(|l| l.as_str() > stale.as_str())
            .unwrap();
        log.insert(i, stale);
        log.push(at(
            shift(2_021_670),
            "[WARNING] sysbus: AETHELGARD-STOP 2 2167 202167 81 81",
        ));
        log.push(at(
            shift(2_021_670),
            "[WARNING] sysbus: AETHELGARD-CLEARED 2 2167 202167 80",
        ));
        let (f, w) = judge_anchors(&log, start).unwrap();
        assert_eq!(kinds_of(&f), Vec::<&str>::new(), "{:?}", messages(&f));
        assert_eq!(w.anchor_ns, Some(start + 21_670));
        // Without its own stop, the stale one never stands in for it.
        log.truncate(log.len() - 2);
        let (f, _) = judge_anchors(&log, start).unwrap();
        assert_eq!(kinds_of(&f), ["judge_stop_missing"]);
        assert!(
            f[0].message
                .contains("1 stale stop(s) of an earlier boot did nothing")
        );
    }

    /// Every line of ours is read whole: one in no shape it writes, or
    /// whose count is ahead of its stamp, is refused (exit 5). One whose
    /// count lags its stamp is the core's lag, not the tool's:
    /// `cpu_halted`, naming the line (adversarial pass 1, M1).
    #[test]
    fn a_hook_line_out_of_shape_is_refused_and_one_lagging_is_cpu_halted() {
        let halted = |log: &[String], start: u64| -> Vec<String> {
            let (f, _) = judge_anchors(log, start).unwrap();
            f.into_iter()
                .filter(|f| f.kind == "cpu_halted")
                .map(|f| f.message)
                .collect()
        };
        let mut log = dshot_log();
        log[2] = at(30_620, "[WARNING] sysbus: AETHELGARD-CR1 2 81 0");
        assert!(
            judge_anchors(&log, 0)
                .err()
                .unwrap()
                .contains("in no shape")
        );
        let mut log = dshot_log();
        log[2] = at(130_620, "[WARNING] sysbus: AETHELGARD-CR1 2 81 0 3067");
        let h = halted(&log, 0);
        assert_eq!(h.len(), 1, "{h:?}");
        assert!(h[0].contains("stamped 130620 ns"), "{h:?}");
        // Its anchor falls back to its stamp: the edges from 30 670 ns are
        // then before it, named, never judged.
        let (f, w) = judge_anchors(&log, 0).unwrap();
        assert_eq!(w.anchor_ns, Some(130_620));
        assert!(
            kinds_of(&f).contains(&"motor_edge_before_anchor"),
            "{:?}",
            messages(&f)
        );
        // The same boot read from the wrong start: every count ahead of
        // its stamp, refused.
        assert!(
            judge_anchors(&dshot_log(), 100_000)
                .err()
                .unwrap()
                .contains("stamped 30620 ns")
        );
        // A hook's line may be stamped up to a quantum early (the `CR1`
        // line above is 50 ns early); a scheduled action's line is exact
        // (the PM's decision on S1's open question 3): one grain early, a
        // stop, its clearing, a stale stop or an injected byte is refused;
        // one grain late, it is the core's lag.
        assert_eq!(halted(&dshot_log(), 0), Vec::<String>::new());
        let n = dshot_log().len();
        for (i, ns, body) in [
            (n - 2, 2_030_660, "STOP 2 3067 203067 81 81"),
            (n - 1, 2_030_660, "CLEARED 2 3067 203067 80"),
            (n - 1, 2_030_660, "STALE STOP 2 3067 203067"),
            (n - 1, 2_030_660, "INJECT 0 1 3075 203067"),
        ] {
            let mut log = dshot_log();
            log[i] = at(ns, &format!("[WARNING] sysbus: AETHELGARD-{body}"));
            let e = judge_anchors(&log, 0).err().unwrap_or_default();
            assert!(e.contains(&format!("stamped {ns} ns")), "{body}: {e}");
        }
        for (i, ns, body) in [
            (n - 2, 2_030_680, "STOP 2 3067 203067 81 81"),
            (n - 1, 2_030_680, "CLEARED 2 3067 203067 80"),
            (n - 1, 2_030_680, "INJECT 0 1 3075 203067"),
        ] {
            let mut log = dshot_log();
            log[i] = at(ns, &format!("[WARNING] sysbus: AETHELGARD-{body}"));
            let h = halted(&log, 0);
            assert!(
                h.len() == 1 && h[0].contains(&format!("stamped {ns} ns")),
                "{body}: {h:?}"
            );
        }
        let mut log = dshot_log();
        log[n - 1] = at(
            2_030_660,
            "[WARNING] sysbus: AETHELGARD-STALE STOP 2 3067 203066",
        );
        log[n - 2] = at(
            2_030_660,
            "[WARNING] sysbus: AETHELGARD-INJECT 0 1 3075 203066",
        );
        assert_eq!(
            halted(&log, 0),
            Vec::<String>::new(),
            "exact action lines of any shape are read"
        );
    }

    /// Adversarial pass 1, M1: a core that slept (`WFI`) before its timer's
    /// `CEN` lags its time by the sleep. Every line of ours after it is
    /// placed at its stamp: the anchor, the stop that ran at its due count,
    /// and the window between them are consistent (the stop's instant is
    /// the window's end), the one finding is `cpu_halted`. A sleep inside
    /// the window makes the stop stale: it does nothing and fails by name.
    #[test]
    fn a_core_that_slept_is_cpu_halted_and_its_anchors_fall_back_to_their_stamps() {
        let lag = 1_000_000u64;
        let shift = |log: &[String], from: u64, by: u64| -> Vec<String> {
            log.iter()
                .map(|l| match log_stamped(l) {
                    Some((ns, body)) if ns >= from => at(ns + by, body),
                    _ => l.clone(),
                })
                .collect()
        };
        // Everything from the anchor on is 1 ms later than its count.
        let slept = shift(&dshot_log(), 30_000, lag);
        let (f, w) = judge_anchors(&slept, 0).unwrap();
        assert_eq!(kinds_of(&f), ["cpu_halted"], "{:?}", messages(&f));
        assert_eq!(w.status, "witnessed");
        assert_eq!(w.anchor_ns, Some(30_620 + lag));
        assert_eq!(w.end_ns, 30_620 + lag + 2_000_000);
        // A sleep between the anchor and its stop: the stop's count is not
        // its due one, so it logged itself stale and did nothing.
        let mut log = shift(&dshot_log(), 1_000_000, lag);
        let n = log.len();
        log[n - 2] = at(
            2_030_670 + lag,
            "[WARNING] sysbus: AETHELGARD-STALE STOP 2 3067 103067",
        );
        log.pop();
        let (f, _) = judge_anchors(&log, 0).unwrap();
        let k = kinds_of(&f);
        assert!(
            k.contains(&"cpu_halted") && k.contains(&"judge_stop_missing"),
            "{k:?}"
        );
    }

    /// `AET-WO-2302` D4: each port's injection from the lines: port 0 from
    /// the boot word, port 1 once port 0 has echoed its eighth byte, each
    /// byte at its due count; anything else is no injection, and says why.
    #[test]
    fn each_port_goes_in_whole_or_is_named_why_not() {
        let t = console_topology();
        let image = aethelgard_twin::boot::expected(&t).unwrap();
        let mut serial = console();
        let mut slow = serial[0].clone();
        slow.port.renode = "usart2";
        slow.spacing_ns = Some(1_100_000);
        serial.push(slow);
        let p = serial_plan(&image, &[], &serial);
        let line = |count: u64, body: String| {
            at(count * 10, &format!("[WARNING] sysbus: AETHELGARD-{body}"))
        };
        let frame0 = p.port_hooks()[0].frame;
        let mut log = vec![line(3075, "BOOTWORD 2100b007 0 3075".into())];
        for j in 0..8u64 {
            let c = 3075 + 10_000 + 10_000 * j;
            let mux = if j == 0 { " 28a801 770" } else { "" };
            log.push(line(c, format!("INJECT 0 {j} 3075 {c}{mux}")));
            let b = frame0[j as usize];
            log.push(line(c + 50, format!("TDR 0 {b:x} {}", c + 50)));
        }
        let echoed = 3075 + 10_000 + 70_000 + 50;
        for j in 0..8u64 {
            let c = echoed + 10_000 + 110_000 * j;
            let mux = if j == 0 { " 28a801 770" } else { "" };
            log.push(line(c, format!("INJECT 1 {j} {echoed} {c}{mux}")));
        }
        let read = |log: &[String]| injections(&p, log);
        let j = read(&log);
        assert_eq!(
            j.iter()
                .map(|j| (
                    j.at_ns,
                    j.bytes,
                    j.mux,
                    j.why.clone(),
                    j.out_of_order.clone()
                ))
                .collect::<Vec<_>>(),
            [
                (Some(130_750), 8, Some((0x28_a801, 0x770)), None, None),
                (Some(931_250), 8, Some((0x28_a801, 0x770)), None, None),
            ]
        );
        // Port 0's eighth echoed byte, the line port 1 is anchored at.
        let last = format!("TDR 0 {:x} {echoed}", frame0[7]);
        // Port 0 echoed seven: port 1 never went in.
        let seven: Vec<String> = log
            .iter()
            .filter(|l| !l.ends_with(&last) && !l.contains("INJECT 1 "))
            .cloned()
            .collect();
        let j = read(&seven);
        assert_eq!(j[1].at_ns, None);
        assert!(
            j[1].why
                .as_deref()
                .unwrap()
                .contains("echoed 7 of its 8 bytes"),
            "{:?}",
            j[1]
        );
        // Adversarial pass 1, L1: bytes written to port 0's `TDR` that do
        // not echo its frame in order (its frame before its first byte went
        // in, a pattern byte, junk) are never counted.
        let mut noisy = log.clone();
        for (i, b) in frame0.iter().enumerate() {
            noisy.insert(
                1 + i,
                line(3_080 + i as u64, format!("TDR 0 {b:x} {}", 3_080 + i)),
            );
        }
        let at = noisy.iter().position(|l| l.ends_with(&last)).unwrap();
        noisy.insert(at, line(echoed - 10, format!("TDR 0 4d {}", echoed - 10)));
        let seen = |j: Vec<PortInjection>| {
            j.into_iter()
                .map(|j| (j.at_ns, j.bytes, j.mux, j.why, j.out_of_order))
                .collect::<Vec<_>>()
        };
        assert_eq!(seen(read(&noisy)), seen(read(&log)));
        // L2: port 1 in before port 0's last echoed frame byte (that line
        // moved after it), or while port 0 had echoed seven, is named.
        let mut early: Vec<String> = log
            .iter()
            .filter(|l| !l.ends_with(&last))
            .cloned()
            .collect();
        early.push(line(
            echoed + 900_000,
            format!("TDR 0 {:x} {}", frame0[7], echoed + 900_000),
        ));
        let j = read(&early);
        assert_eq!(j[1].at_ns, Some(931_250));
        assert!(
            j[1].out_of_order
                .as_deref()
                .is_some_and(|w| w.contains("before port 0's last echoed frame byte at 9831250 ns")),
            "{:?}",
            j[1]
        );
        let unechoed: Vec<String> = log
            .iter()
            .filter(|l| !l.ends_with(&last))
            .cloned()
            .collect();
        let j = read(&unechoed);
        assert!(
            j[1].out_of_order.as_deref().is_some_and(
                |w| w.contains("while port 0 before it had echoed 7 of its 8 frame bytes")
            ),
            "{:?}",
            j[1]
        );
        assert_eq!(j[0].out_of_order, None);
        // No boot word observed: nothing went in, by name.
        let j = read(&log[1..1]);
        assert!(
            j[0].why
                .as_deref()
                .unwrap()
                .contains("no write of the boot word")
        );
        // A byte twice (a boot word written over another word twice): not
        // one frame, so no injection.
        let mut twice = log.clone();
        twice.insert(3, log[1].clone());
        assert_eq!(read(&twice)[0].at_ns, None);
        // The boot ended after three bytes: an injection of three, named.
        let three: Vec<String> = log.iter().take(7).cloned().collect();
        let j = read(&three);
        assert_eq!((j[0].at_ns, j[0].bytes), (Some(130_750), 3));
        assert!(
            j[0].why
                .as_deref()
                .unwrap()
                .contains("3 of its 8 bytes went in")
        );
        // The ports' wire time from the boot word's quantum, past the run:
        // named after it (`serial_wire_past_run`); inside it, not.
        assert!(wire_past_run(&p, 30_750).is_none());
        let named = wire_past_run(&p, 99_000_000).unwrap();
        assert_eq!(named.kind, "serial_wire_past_run");
        assert!(
            named
                .message
                .contains("from one quantum after the boot word at 99000000 ns")
        );
    }

    /// `AET-WO-2302` D5: a port held open gets nothing and says why; the
    /// port after it goes in at the echo of the last port before it not held
    /// open (or at the boot word); a byte into a port held open is no
    /// injection.
    #[test]
    fn a_port_held_open_is_skipped_by_name() {
        let t = console_topology();
        let image = aethelgard_twin::boot::expected(&t).unwrap();
        let mut serial = console();
        for renode in ["usart2", "uart4"] {
            let mut p = serial[0].clone();
            p.port.renode = renode;
            p.spacing_ns = Some(1_100_000);
            serial.push(p);
        }
        let line = |count: u64, body: String| {
            at(count * 10, &format!("[WARNING] sysbus: AETHELGARD-{body}"))
        };
        let bytes = serial[0].frame;
        let frame = |log: &mut Vec<String>, k: usize, anchor: u64, spacing: u64, echo: bool| {
            for j in 0..8u64 {
                let c = anchor + 10_000 + spacing * j;
                let mux = if j == 0 { " 28a801 770" } else { "" };
                log.push(line(c, format!("INJECT {k} {j} {anchor} {c}{mux}")));
                if echo {
                    let b = bytes[j as usize];
                    log.push(line(c + 50, format!("TDR {k} {b:x} {}", c + 50)));
                }
            }
        };
        let read = |serial: &[Serial], log: &[String]| {
            injections(&serial_plan(&image, &[], serial), log)
                .into_iter()
                .map(|j| (j.at_ns, j.bytes, j.why))
                .collect::<Vec<_>>()
        };
        let open = Some("the scenario holds its RX line open (serial_rx_open)".to_owned());
        // The middle port open: port 2 goes in at port 0's echo.
        serial[1].open = true;
        let mut log = vec![line(3075, "BOOTWORD 2100b007 0 3075".into())];
        frame(&mut log, 0, 3075, 10_000, true);
        let echoed = 3075 + 10_000 + 70_000 + 50;
        frame(&mut log, 2, echoed, 110_000, false);
        assert_eq!(
            read(&serial, &log),
            [
                (Some(130_750), 8, None),
                (None, 0, open.clone()),
                (Some(931_250), 8, None),
            ]
        );
        // Port 0 echoed seven: port 2 never went in, and names port 0.
        let seven: Vec<String> = log
            .iter()
            .filter(|l| {
                !l.ends_with(&format!("TDR 0 {:x} {echoed}", bytes[7])) && !l.contains("INJECT 2 ")
            })
            .cloned()
            .collect();
        let j = read(&serial, &seven);
        assert_eq!(j[1].2, open);
        assert!(
            j[2].2
                .as_deref()
                .is_some_and(|w| w.starts_with("port 0 before it echoed 7 of its 8 bytes")),
            "{j:?}"
        );
        // The first port open: port 1 goes in at the boot word.
        serial[1].open = false;
        serial[0].open = true;
        let mut log = vec![line(3075, "BOOTWORD 2100b007 0 3075".into())];
        frame(&mut log, 1, 3075, 110_000, false);
        let j = read(&serial, &log);
        assert_eq!(j[0], (None, 0, open.clone()));
        assert_eq!((j[1].0, j[1].1, &j[1].2), (Some(130_750), 8, &None));
        // A byte into the port held open: no injection, said so.
        let mut forged = log.clone();
        frame(&mut forged, 0, 3075, 10_000, false);
        let j = read(&serial, &forged);
        assert_eq!(j[0].0, None);
        assert!(
            j[0].2
                .as_deref()
                .is_some_and(|w| w.contains("not one frame")),
            "{j:?}"
        );
    }

    /// `AET-WO-2302`, through `judge_boot` whole: a DShot plan's boot read
    /// from its transcript. Passing, the stop's read-back holds and the end
    /// read of `CR1` expects `CEN` cleared, the judge's own act; with no stop
    /// in the boot, `CR1` is expected set and `judge_stop_missing` named;
    /// past the filter's budget nothing of ours is read, every output is
    /// unreadable, and no anchor finding is named (fail closed, never absent).
    #[test]
    fn judge_boot_reads_a_dshot_boot_s_window_and_stop() {
        let (image, motors) = dshot_parts();
        let loops = [0x0800_0010u64];
        let executable = [(0x0800_0000u64, 0x0801_0000u64)];
        let p = BootPlan {
            motors: &motors,
            self_loops: &loops,
            executable: &executable,
            ..plan(&image, &[])
        };
        let flash = "[00:00:00.000000000] [INFO] flashController: [cpu: 0x8000102] ReadUInt32 from 0x1C (OptionStatusCurrent), returned 0x406AAF0";
        let judge = |log: &[String], cr1: u32, dropped: u64| {
            let mut text = format!("{flash}\n");
            for l in log {
                text.push_str(l);
                text.push('\n');
            }
            text.push_str("AETHELGARD-BOOT-BEGIN\n");
            for (_, read) in p.reads() {
                let v = match read {
                    Read::BootWord => u64::from(aethelgard_twin::boot::BOOT_MAGIC),
                    Read::Instructions => 10_000_000,
                    Read::Mips => 100,
                    Read::Pc => 0x0800_0010,
                    Read::Icsr => 0x800,
                    Read::Loop(_, 0) => 0xBF10,
                    Read::Loop(_, _) => 0xE7FD,
                    Read::BrownOut => 0x0406_AAF0,
                    Read::Register(i) if image.registers[i].name == "TIM2.CR1" => u64::from(cr1),
                    Read::Register(i) => u64::from(image.registers[i].value),
                    Read::Record(_, 0) => u64::from(aethelgard_twin::boot::STATUS_OK),
                    Read::Record(_, _) => 0x60,
                };
                text.push_str(&format!("{v:#x}\n"));
            }
            text.push_str("AETHELGARD-BOOT-END\nAvailable peripherals:\n  \u{251c}\u{2500}\u{2500} cpu (CortexM)\n  \u{2502}       Slot: 0\n  \u{251c}\u{2500}\u{2500} u2 (BME280)\n");
            let out = output(text.into_bytes(), dropped);
            let (f, _, o) = judge_boot(0, &out, &p, &Captured::default()).unwrap();
            let failing: Vec<String> = f
                .iter()
                .filter(|f| f.severity != "info")
                .map(|f| f.kind.clone())
                .collect();
            (failing, o.unwrap())
        };
        let cr1 = image
            .registers
            .iter()
            .position(|r| r.name == "TIM2.CR1")
            .unwrap();
        // Passing: the window's witness, the stop, and CR1 cleared at the end.
        let (failing, o) = judge(&dshot_log(), 0x80, 0);
        assert!(failing.is_empty(), "{failing:?}");
        assert_eq!(o.motors[0].status, "witnessed");
        assert_eq!(o.register_expected[cr1], 0x80);
        assert_eq!(o.boot_word_ns, Some(30_750));
        assert_eq!(o.timers[0].stop.as_ref().map(|s| s.at_ns), Some(2_030_670));
        // CR1 read with CEN still set after the stop ran: the judge's own
        // clear did not hold, named as a register mismatch.
        let (failing, _) = judge(&dshot_log(), 0x81, 0);
        assert_eq!(failing, ["register_mismatch"]);
        // No stop in the boot: CEN is expected set, and the stop is named.
        let mut no_stop = dshot_log();
        no_stop.truncate(no_stop.len() - 2);
        let (failing, o) = judge(&no_stop, 0x81, 0);
        assert_eq!(failing, ["judge_stop_missing"]);
        assert_eq!(o.register_expected[cr1], 0x81);
        // Past the filter's budget: nothing of ours is read.
        let (failing, o) = judge(&dshot_log(), 0x80, 1);
        assert!(o.timers.is_empty() && o.boot_word_ns.is_none());
        assert_eq!(o.motors[0].status, "unreadable");
        assert!(
            failing.iter().all(|k| !k.starts_with("motor_anchor")
                && !k.starts_with("judge_stop")
                && !k.starts_with("motor_edge")
                && k != "motor_window_past_run"),
            "{failing:?}"
        );
        assert!(
            failing.contains(&"motor_output_unwitnessed".to_owned()),
            "{failing:?}"
        );
    }

    /// `AET-WO-2302` D4, through `judge_boot` whole: the two serial findings
    /// only the run decides are named by it. A boot whose frames and echoes
    /// from the boot word run past the boot's end names
    /// `serial_wire_past_run`; a second port's frame written before the
    /// first has echoed its eighth byte names `serial_injection_out_of_order`
    /// (adversarial pass 1, L2); a boot in order and inside its end names
    /// neither.
    #[test]
    fn judge_boot_names_a_wire_past_the_run_and_an_injection_out_of_order() {
        let t = console_topology();
        let image = aethelgard_twin::boot::expected(&t).unwrap();
        let mut serial = console();
        let mut slow = serial[0].clone();
        slow.port.renode = "usart2";
        slow.spacing_ns = Some(1_100_000);
        serial.push(slow);
        let judge = |p: &BootPlan<'_>, log: &[String]| {
            let mut text = String::new();
            for l in log {
                text.push_str(l);
                text.push('\n');
            }
            text.push_str("AETHELGARD-BOOT-BEGIN\n");
            for (_, read) in p.reads() {
                let v = match read {
                    Read::BootWord => u64::from(aethelgard_twin::boot::BOOT_MAGIC),
                    Read::Instructions => aethelgard_twin::boot::MIPS * p.virtual_ns / 1000,
                    Read::Mips => 100,
                    Read::Pc => 0x0800_0010,
                    Read::Icsr => 0x800,
                    Read::Loop(_, 0) => 0xBF10,
                    Read::Loop(_, _) => 0xE7FD,
                    Read::BrownOut => 0x0406_AAF0,
                    Read::Register(i) => u64::from(image.registers[i].value),
                    Read::Record(_, 0) => u64::from(aethelgard_twin::boot::STATUS_OK),
                    Read::Record(_, _) => 0x60,
                };
                text.push_str(&format!("{v:#x}\n"));
            }
            text.push_str("AETHELGARD-BOOT-END\nAvailable peripherals:\n  \u{251c}\u{2500}\u{2500} cpu (CortexM)\n  \u{2502}       Slot: 0\n  \u{251c}\u{2500}\u{2500} u2 (BME280)\n");
            let out = output(text.into_bytes(), 0);
            let (f, _, _) = judge_boot(0, &out, p, &Captured::default()).unwrap();
            f.into_iter().map(|f| f.kind).collect::<Vec<_>>()
        };
        let line = |count: u64, body: String| {
            at(count * 10, &format!("[WARNING] sysbus: AETHELGARD-{body}"))
        };
        let p = serial_plan(&image, &[], &serial);
        let frame0 = p.port_hooks()[0].frame;
        // Port 0 in from the boot word and echoed whole, port 1 after it.
        let mut log = vec![line(3075, "BOOTWORD 2100b007 0 3075".into())];
        for j in 0..8u64 {
            let c = 3075 + 10_000 + 10_000 * j;
            let mux = if j == 0 { " 28a801 770" } else { "" };
            log.push(line(c, format!("INJECT 0 {j} 3075 {c}{mux}")));
            log.push(line(
                c + 50,
                format!("TDR 0 {:x} {}", frame0[j as usize], c + 50),
            ));
        }
        let echoed = 3075 + 10_000 + 70_000 + 50;
        for j in 0..8u64 {
            let c = echoed + 10_000 + 110_000 * j;
            let mux = if j == 0 { " 28a801 770" } else { "" };
            log.push(line(c, format!("INJECT 1 {j} {echoed} {c}{mux}")));
        }
        let named = |kinds: &[String], kind: &str| kinds.iter().any(|k| k == kind);
        let kinds = judge(&p, &log);
        assert!(!named(&kinds, "serial_wire_past_run"), "{kinds:?}");
        assert!(!named(&kinds, "serial_injection_out_of_order"), "{kinds:?}");
        // The same boot ending before the frames and echoes can: 1 ms.
        let short = BootPlan {
            virtual_ns: 1_000_000,
            ..serial_plan(&image, &[], &serial)
        };
        assert!(named(&judge(&short, &log), "serial_wire_past_run"));
        // Port 0 echoes only its first byte before port 1's frame goes in.
        let mut early = vec![log[0].clone()];
        early.extend(log[1..3].iter().cloned());
        early.extend(log[17..].iter().cloned());
        let kinds = judge(&p, &early);
        assert!(named(&kinds, "serial_injection_out_of_order"), "{kinds:?}");
    }

    /// `4300` D8: the silicon check is the topology's, reported apart; a port
    /// whose check fails is named with its row and source.
    #[test]
    fn a_serial_port_failing_silicon_s_check_is_named_with_its_row() {
        let mut s = console();
        assert!(static_serial_findings(&s).is_empty());
        s[0].silicon.holds = false;
        let f = static_serial_findings(&s);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind, "serial_framing_static_mismatch");
        assert!(
            f[0].message
                .starts_with("CONSOLE (USART1, TX PA9, RX PA10): BRR 556")
                && f[0].message.contains("Table 407")
                && f[0]
                    .message
                    .contains(aethelgard_ir::serial::TOLERANCE_SOURCE),
            "{}",
            f[0].message
        );
    }

    /// The witness from forged inputs alone: the echo counts only with the RX
    /// pin routed at the injection and the whole frame after the patterns;
    /// the tags are unreadable without the whole log; a capture past the
    /// boots a droop rendered is forged.
    #[test]
    fn a_serial_witness_is_read_from_its_capture_pins_and_log() {
        let kinds = |f: Vec<Finding>| f.into_iter().map(|f| f.kind).collect::<Vec<_>>();
        let names = |f: &[Finding]| f.iter().map(|f| f.kind.clone()).collect::<Vec<_>>();
        let t = console_topology();
        let image = aethelgard_twin::boot::expected(&t).unwrap();
        let serial = console();
        let p = serial_plan(&image, &[], &serial);
        let regs: Vec<u32> = image.registers.iter().map(|r| r.value).collect();
        let whole = [serial[0].pattern.clone(), serial[0].frame.to_vec()].concat();
        let captured = |bytes: &[u8]| Captured {
            files: vec![("usart1.bin".to_owned(), bytes.to_vec())],
            problems: Vec::new(),
        };
        let routed = [PortInjection {
            at_ns: Some(130_750),
            bytes: 8,
            mux: Some((0x0028_A801, 0x0000_0770)),
            line: Some(0),
            why: None,
            out_of_order: None,
        }];
        let none = [PortInjection::default()];
        let boot = console_boot();
        let one: &[Vec<aethelgard_twin::serial::Access>] = std::slice::from_ref(&boot);
        let w = serial_witnesses(&p, &regs, &routed, &captured(&whole), Some(one), &[None]);
        assert_eq!(w[0].echo, "witnessed");
        assert_eq!(w[0].tx_at_send, Some((0x0028_A801, 0x0000_0770)));
        assert!(serial_findings_of(&serial[0], &w[0], 0, None).is_empty());
        // Adversarial F4: PA9 routed only after the first TDR write.
        let late: Vec<_> = boot[3..8]
            .iter()
            .chain(&boot[..3])
            .chain(&boot[8..])
            .cloned()
            .collect();
        let w = serial_witnesses(
            &p,
            &regs,
            &routed,
            &captured(&whole),
            Some(&[late]),
            &[None],
        );
        assert_eq!(w[0].tx_at_send, Some((0, 0)));
        let f = serial_findings_of(&serial[0], &w[0], 0, None);
        assert_eq!(names(&f), ["serial_pattern_missing"]);
        assert!(
            f[0].message
                .contains("at a boot's first TDR write the TX pin was at PA9 MODER 0b00, AF0"),
            "{}",
            f[0].message
        );
        // Adversarial F3: a PRESC write, judged by the access log.
        let mut presc = boot.clone();
        presc.insert(4, aethelgard_twin::serial::access(PRESC).unwrap());
        let w = serial_witnesses(
            &p,
            &regs,
            &routed,
            &captured(&whole),
            Some(&[presc]),
            &[None],
        );
        let f = serial_findings_of(&serial[0], &w[0], 0, None);
        assert_eq!(names(&f), ["serial_tag_mismatch"]);
        assert!(
            f[0].message.ends_with("not the bring-up: 0x1 to 0x2C"),
            "{}",
            f[0].message
        );
        // Adversarial pass 2, A1 (ADV2-1): GPIOA reset through the RCC after
        // the bring-up, every word written back; A2 (ADV2-2): PA9 open-drain
        // with a pull-down before the pattern; and PA10 pulled up at the
        // injection (eight accesses in), released after it.
        let line = |l: &str| aethelgard_twin::serial::access(l).unwrap();
        let mut reset = boot.clone();
        reset.splice(
            3..3,
            [
                line("[INFO] rcc: [cpu: 0x8000710] WriteUInt32 to 0x88 (AHB4Reset), value 0x1"),
                line("[INFO] rcc: [cpu: 0x8000714] WriteUInt32 to 0x88 (AHB4Reset), value 0x0"),
            ],
        );
        reset.splice(5..5, boot[..3].iter().cloned());
        let open_drain = {
            let mut b = boot.clone();
            b.splice(
                7..7,
                [
                    line("[INFO] gpioPortA: [cpu: 0x8000A56] WriteUInt32 to 0x4 (OutputType), value 0x200"),
                    line("[INFO] gpioPortA: [cpu: 0x8000A56] WriteUInt32 to 0xC (PullUpPullDown), value 0x80000"),
                ],
            );
            b
        };
        let pulled = {
            let mut b = boot.clone();
            b.insert(7, line("[INFO] gpioPortA: [cpu: 0x8000A56] WriteUInt32 to 0xC (PullUpPullDown), value 0x100000"));
            b.push(line(
                "[INFO] gpioPortA: [cpu: 0x8000A56] WriteUInt32 to 0xC (PullUpPullDown), value 0x0",
            ));
            b
        };
        // Adversarial pass 4, ADV4-4: a pin the interval rule names is never
        // witnessed: PA9 off AF7 once solved is `tx_at_send` (the pattern's
        // finding too); PA10 changed once solved is not counted (the echo's).
        let routed_tx = Some((0x0028_A801, 0x0000_0770));
        for (accesses, inject_at, unexpected, tx, kinds) in [
            (
                reset,
                None,
                // Written back as the bring-up wrote them, AFRH's first word
                // unroutes PA9 in the rebuild: the interval rule sees it too.
                "RCC.AHB4RSTR 0x1 written, not an enable register, RCC.AHB4RSTR 0x0 written, not an enable register, PA9 MODER 0b10, AF0, OTYPER 0, OSPEEDR 0b00, PUPDR 0b00 after it was solved",
                Some((0x0028_A801, 0x0000_0700)),
                &["serial_tag_mismatch", "serial_pattern_missing"][..],
            ),
            (
                open_drain,
                None,
                "PA9 OT set in OTYPER, a bit Renode does not store, PA9 MODER 0b10, AF7, OTYPER 1, OSPEEDR 0b00, PUPDR 0b00 after it was solved, PA9 OTYPER 1, OSPEEDR 0b00, PUPDR 0b10 at the first TDR write, PA9 OTYPER 1, OSPEEDR 0b00, PUPDR 0b10 at the end",
                routed_tx,
                &["serial_tag_mismatch"][..],
            ),
            (
                pulled,
                Some(8),
                "PA10 MODER 0b10, AF7, OTYPER 0, OSPEEDR 0b00, PUPDR 0b01 after it was solved, PA10 OTYPER 0, OSPEEDR 0b00, PUPDR 0b01 at the injection",
                routed_tx,
                &["serial_tag_mismatch", "serial_echo_missing"][..],
            ),
        ] {
            let w = serial_witnesses(
                &p,
                &regs,
                &routed,
                &captured(&whole),
                Some(&[accesses]),
                &[inject_at],
            );
            assert_eq!(w[0].tx_at_send, tx);
            let f = serial_findings_of(&serial[0], &w[0], 0, None);
            assert_eq!(names(&f), kinds);
            assert!(
                f[0].message
                    .ends_with(&format!("not the bring-up: {unexpected}")),
                "{}",
                f[0].message
            );
        }
        // No frame injected: the RX pin is not judged at the injection.
        let w = serial_witnesses(&p, &regs, &none, &captured(&whole), Some(one), &[Some(8)]);
        assert!(w[0].tags.as_ref().is_some_and(|t| t.holds()));
        // Off its AF at the injection: the frame came back, not counted.
        let off = [PortInjection {
            mux: Some((0x0028_A801, 0x0000_0870)),
            ..routed[0].clone()
        }];
        let w = serial_witnesses(&p, &regs, &off, &captured(&whole), Some(one), &[None]);
        assert_eq!(w[0].echo, "not_counted");
        assert_eq!(
            kinds(serial_findings_of(&serial[0], &w[0], 0, None)),
            ["serial_echo_missing"]
        );
        // No injection read: never witnessed.
        let w = serial_witnesses(&p, &regs, &none, &captured(&whole), Some(one), &[None]);
        assert_eq!(w[0].echo, "missing");
        // The log not whole: neither the writes nor the TX pin at its first
        // TDR write are judged, and both fail.
        let w = serial_witnesses(&p, &regs, &routed, &captured(&whole), None, &[None]);
        assert_eq!(
            kinds(serial_findings_of(&serial[0], &w[0], 0, None)),
            ["serial_tag_mismatch", "serial_pattern_missing"]
        );
        // Two patterns: forged in one boot, the droop's two boots in one.
        let two = [serial[0].pattern.clone(), whole.clone()].concat();
        let w = serial_witnesses(&p, &regs, &routed, &captured(&two), Some(one), &[None]);
        assert_eq!(
            kinds(serial_findings_of(&serial[0], &w[0], 0, None)),
            ["serial_capture_forged"]
        );
        assert!(serial_findings_of(&serial[0], &w[0], 1, None).is_empty());
        // A pattern cut short (the ruling on the parser): a reset's doing
        // in a droop's two boots; forged where no reset happened, and where
        // cut and whole patterns pass the boots.
        let pattern = &serial[0].pattern;
        let cut = [&pattern[..3], &whole[..]].concat();
        let w = serial_witnesses(&p, &regs, &routed, &captured(&cut), Some(one), &[None]);
        assert_eq!(w[0].echo, "witnessed");
        assert_eq!(
            w[0].capture.as_ref().map(|c| (c.patterns, c.cut)),
            Some((1, 1))
        );
        assert!(serial_findings_of(&serial[0], &w[0], 1, None).is_empty());
        assert_eq!(
            kinds(serial_findings_of(&serial[0], &w[0], 0, None)),
            ["serial_capture_forged"]
        );
        let three = [&pattern[..3], &pattern[..], &whole[..]].concat();
        let w = serial_witnesses(&p, &regs, &routed, &captured(&three), Some(one), &[None]);
        assert_eq!(
            kinds(serial_findings_of(&serial[0], &w[0], 1, None)),
            ["serial_capture_forged"]
        );
        assert!(serial_findings_of(&serial[0], &w[0], 2, None).is_empty());
        // A stock USART no port names, empty: its backend's own file.
        let mut c = captured(&whole);
        c.files.push(("usart2.bin".to_owned(), Vec::new()));
        assert!(capture_findings(&p, &c).is_empty());
        // Adversarial F12: a byte on it is forged; so are a file no backend
        // writes and a problem the CLI met.
        c.files[1].1 = serial[0].pattern.clone();
        c.files.push(("usart9.bin".to_owned(), Vec::new()));
        c.problems
            .push("`x` is not a regular file beneath it".to_owned());
        let f = capture_findings(&p, &c);
        assert_eq!(
            names(&f),
            [
                "serial_capture_forged",
                "serial_capture_forged",
                "serial_capture_forged"
            ]
        );
        assert!(
            f[1].message.starts_with(
                "`usart2.bin` holds 12 byte(s) (43 4F 4E 53 4F 4C 45 00 FE 7F 80 FF), sent on a USART no serial port"
            ),
            "{}",
            f[1].message
        );
    }

    /// A boot of the reference console as Renode 1.17.0's access log shows
    /// it (fix 1, P1's `log.out`, the cpu field kept): PA10's then PA9's AF,
    /// then their modes, the bring-up, two bytes of the pattern.
    const CONSOLE_BOOT: [&str; 9] = [
        "[INFO] gpioPortA: [cpu: 0x80009BC] WriteUInt32 to 0x24 (AlternateFunctionHigh), value 0x700",
        "[INFO] gpioPortA: [cpu: 0x80009BC] WriteUInt32 to 0x24 (AlternateFunctionHigh), value 0x770",
        "[INFO] gpioPortA: [cpu: 0x80009BC] WriteUInt32 to 0x0 (Mode), value 0x28A801",
        "[INFO] usart1: [cpu: 0x80009BC] WriteUInt32 to 0xC (BaudRate), value 0x22C",
        "[INFO] usart1: [cpu: 0x80009BC] WriteUInt32 to 0x4 (ControlRegister2), value 0x0",
        "[INFO] usart1: [cpu: 0x80009BC] WriteUInt32 to 0x0 (ControlRegister1), value 0x2000000C",
        "[INFO] usart1: [cpu: 0x80009BC] WriteUInt32 to 0x0 (ControlRegister1), value 0x2000000D",
        "[INFO] usart1: [cpu: 0x80009BC] WriteUInt32 to 0x28 (TransmitData), value 0x43",
        "[INFO] usart1: [cpu: 0x80009BC] WriteUInt32 to 0x28 (TransmitData), value 0x4F",
    ];
    const PRESC: &str =
        "[INFO] usart1: [cpu: 0x800070A] WriteUInt32 to 0x2C (TransmitData+0x4), value 0x1";

    fn console_boot() -> Vec<aethelgard_twin::serial::Access> {
        CONSOLE_BOOT
            .iter()
            .map(|l| aethelgard_twin::serial::access(l).unwrap())
            .collect()
    }

    /// Each boot's accesses from the log's lines, by line, the logged sources
    /// only; a line of a logged source's `INFO` log that is not a whole access
    /// leaves none (fail closed); a read line goes to the filter's discard.
    #[test]
    fn a_boot_s_accesses_are_its_logged_sources_whole_lines() {
        let stamp = |l: &str| format!("[00:00:00.000100000] {l}");
        let lines: Vec<String> = CONSOLE_BOOT.iter().map(|l| stamp(l)).collect();
        let mut boot: Vec<&str> = lines.iter().map(String::as_str).collect();
        let other = stamp(
            "[INFO] usart2: [cpu: 0x80009BC] WriteUInt32 to 0x2C (TransmitData+0x4), value 0x1",
        );
        let warning = stamp("[WARNING] usart1: Unhandled write to offset 0x2C, value 0x1.");
        boot.extend([other.as_str(), warning.as_str(), "(monitor) usart1"]);
        let got = access_boots(&[boot.clone()], &["usart1", "gpioPortA"]).unwrap();
        assert_eq!(got, [console_boot()]);
        // A locked pin's write refused (adversarial pass 2, L2): the write
        // before it from its source is marked, once; a refusal after no
        // write of its source leaves none; one from an unlogged source is
        // another line.
        let refused = stamp(
            "[WARNING] gpioPortA: Ignoring attempt to change MODER configuration of the locked pin #9",
        );
        let mut locked = boot.clone();
        locked.insert(3, refused.as_str());
        let got = access_boots(&[locked.clone()], &["usart1", "gpioPortA"]).unwrap();
        let mut want = console_boot();
        want[2].refused = true;
        assert_eq!(got, [want]);
        locked.insert(3, refused.as_str());
        assert_eq!(access_boots(&[locked], &["usart1", "gpioPortA"]), None);
        assert_eq!(
            access_boots(&[vec![refused.as_str()]], &["gpioPortA"]),
            None
        );
        assert_eq!(
            access_boots(&[vec![refused.as_str()]], &["usart1"]),
            Some(vec![vec![]])
        );
        let odd = stamp("[INFO] usart1: something else");
        boot.push(odd.as_str());
        assert_eq!(access_boots(&[boot], &["usart1", "gpioPortA"]), None);
        let read = stamp(
            "[INFO] usart1: [cpu: 0x8000990] ReadUInt32 from 0x1C (InterruptAndStatus), returned 0x206000C0",
        );
        assert!(unjudged_access_line(&read) && !unjudged_access_line(&lines[3]));
        assert!(!unjudged_access_line(
            &CONSOLE_BOOT[3].replace("Write", "Read")
        ));
    }

    /// The PM's rulings on adversarial F5 and pass 2's H1, from the
    /// topology alone: each port's pattern and one echoed character, and the
    /// frame's eight bytes as the composition paces them, a character
    /// rounded up to a whole quantum apart (pass 4, ADV4-2), summed over the
    /// ports in the specification's order, from the injection, inside a
    /// bound: the run's end from one quantum after the boot word since
    /// `AET-WO-2302` (`serial_wire_past_run`), 2 ms here for the arithmetic.
    /// The console at 115 200 8N1 (BRR 556): 13 x 86 876 + 8 x 100 000 =
    /// 1 929 388 ns; from an injection at 100 000 ns it ends at
    /// 2 029 388 ns, past 2 000 000. It fits from an injection at 70 612 ns
    /// or earlier. A `GPS` port after it at 115 200 8N1: 9 x 86 876 +
    /// 800 000 = 1 581 884 ns, 3 511 272 ns with the console's: past the
    /// bound, though each alone fits. At 9600 8N1 (BRR 6667, 9 599.520
    /// baud): 13 x 1 041 719 + 8 x 1 100 000 = 22 342 347 ns, ending at
    /// 22 442 347 ns: inside `MAX_VIRTUAL_NS` (100 000 000), which every
    /// board now boots.
    #[test]
    fn a_port_s_wire_time_must_fit_its_bound_from_the_injection() {
        let dshot = aethelgard_twin::boot::DSHOT_WINDOW_NS;
        let full = aethelgard_twin::boot::MAX_VIRTUAL_NS;
        let fast = console();
        assert_eq!(fast[0].spacing_ns, Some(100_000));
        let one = wire_past_window(&fast, 100_000, dshot).unwrap();
        assert_eq!(
            (one.1, one.2),
            (Some((86_876, 1_929_388, 1_929_388)), Some(2_029_388))
        );
        assert!(wire_past_window(&fast, 100_000, full).is_none());
        // At its edge: a budget ending at the window's last ns fits.
        assert!(wire_past_window(&fast, 70_612, dshot).is_none());
        let late = wire_past_window(&fast, 70_613, dshot).unwrap();
        assert_eq!(
            (late.1, late.2),
            (Some((86_876, 1_929_388, 1_929_388)), Some(2_000_001))
        );
        // H1: the ports are served one after another. A second port, alone
        // inside the window, passes it after the console.
        let mut gps = fast[0].clone();
        gps.port.label = "GPS".to_owned();
        gps.pattern = aethelgard_twin::serial::pattern("GPS");
        assert!(wire_past_window(std::slice::from_ref(&gps), 100_000, dshot).is_none());
        let two = [fast[0].clone(), gps];
        let (port, times, end) = wire_past_window(&two, 70_612, dshot).unwrap();
        assert_eq!(port.port.label, "GPS");
        assert_eq!(
            (times, end),
            (Some((86_876, 1_581_884, 3_511_272)), Some(3_581_884))
        );
        assert!(wire_past_window(&two, 100_000, full).is_none());
        let slow_json = CONSOLE_TOPOLOGY
            .replace("\"baud\":115200", "\"baud\":9600")
            .replace(
                "\"brr\":556,\"actual_millibaud\":115107913,\"error_ppm\":-799",
                "\"brr\":6667,\"actual_millibaud\":9599520,\"error_ppm\":-50",
            );
        let slow_topology: aethelgard_ir::SolvedTopology =
            serde_json::from_str(&slow_json).unwrap();
        let slow = serials(&slow_topology, &[0x11; 32]).unwrap();
        assert_eq!(slow[0].silicon.actual_millibaud, 9_599_520);
        assert_eq!(slow[0].spacing_ns, Some(1_100_000));
        let (port, times, end) = wire_past_window(&slow, 100_000, dshot).unwrap();
        assert_eq!(port.port.label, "CONSOLE");
        assert_eq!(
            (times, end),
            (Some((1_041_719, 22_342_347, 22_342_347)), Some(22_442_347))
        );
        assert!(wire_past_window(&slow, 100_000, full).is_none());
        // After the run (adversarial pass 2, A2-3): from the boot word only
        // the frame as injected and one echoed character, 8 x 1 100 000 +
        // 1 041 719 = 9 841 719 ns, since the bring-up sends its pattern
        // before the boot word. A boot word at 87 030 760 ns (the
        // adversary's 9600 board, its echo witnessed) is inside the run;
        // the last that fits is 100 000 000 - 100 000 - 9 841 719 =
        // 90 058 281 ns.
        assert_eq!(frame_echo_ns(&slow[0]), Some(9_841_719));
        let image = aethelgard_twin::boot::expected(&slow_topology).unwrap();
        let p = serial_plan(&image, &[], &slow);
        assert_eq!(p.virtual_ns, full);
        for bw in [79_530_760, 87_030_760, 90_058_281] {
            assert!(wire_past_run(&p, bw).is_none(), "{bw}");
        }
        let named = wire_past_run(&p, 90_058_282).unwrap();
        assert!(
            named.message.contains(
                "its own 9841719 ns (1041719 ns a character) of 9841719 ns in all, ending at 100000001 ns"
            ),
            "{}",
            named.message
        );
        assert!(wire_past_run(&p, 94_530_760).is_some());
        // Two ports in the chain sum; a port held open gets no frame and
        // adds nothing (`AET-WO-2302` D5).
        let mut two = vec![slow[0].clone(), slow[0].clone()];
        two[1].port.label = "GPS".to_owned();
        let p = serial_plan(&image, &[], &two);
        let named = wire_past_run(&p, 80_216_563).unwrap();
        assert!(named.message.starts_with("GPS"), "{}", named.message);
        assert!(wire_past_run(&p, 80_216_562).is_none());
        two[0].open = true;
        let p = serial_plan(&image, &[], &two);
        assert!(wire_past_run(&p, 90_058_281).is_none());
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
        let f = witness_findings_of(&m, &short);
        assert_eq!(f.len(), 1);
        assert!(
            f[0].message
                .contains("shorter than two of its periods (1279488 ns)"),
            "{}",
            f[0].message
        );
        // `AET-WO-2302`: the window is the output's own, from its anchor to
        // its end; one long enough for two periods gives no reason.
        let long = witness_motor(
            &m,
            Some(Some(vec![(639_750, true)])),
            aethelgard_twin::boot::MAX_VIRTUAL_NS,
        );
        assert!(
            !witness_findings_of(&m, &long)[0]
                .message
                .contains("shorter than two")
        );
        let mut anchored = long.clone();
        anchored.anchor_ns = Some(aethelgard_twin::boot::MAX_VIRTUAL_NS - 1_000_000);
        assert!(
            witness_findings_of(&m, &anchored)[0]
                .message
                .contains("its window of 1000000 ns is shorter than two of its periods")
        );
    }

    /// `AET-WO-3201` (`3100` F15): with no boot word, a record that says its
    /// wait expired is still named, as the booted path names it; a mismatch,
    /// the fill or a passing read is not judged before the fixed point.
    #[test]
    fn a_timed_out_read_is_named_without_the_boot_word() {
        use aethelgard_twin::boot::{RECORD_UNSET, STATUS_OK, STATUS_TXIS};
        let image = image();
        let checks = &image.identity.checks;
        let i2c = checks
            .iter()
            .position(|c| matches!(c.wire, aethelgard_twin::boot::Wire::I2c { .. }))
            .unwrap();
        let mut records = vec![(RECORD_UNSET, u64::from(RECORD_UNSET)); checks.len()];
        assert!(timeouts_without_the_boot_word(checks, &records).is_empty());
        records[i2c] = (STATUS_OK, 0x1);
        assert!(timeouts_without_the_boot_word(checks, &records).is_empty());
        records[i2c] = (STATUS_TXIS, 0);
        let named = timeouts_without_the_boot_word(checks, &records);
        assert_eq!(named.len(), 1);
        assert_eq!(named[0].kind, "i2c_timeout");
        assert_eq!(
            named[0].message,
            identity_finding(&checks[i2c], STATUS_TXIS, 0)
                .unwrap()
                .message
        );
    }

    /// `AET-WO-3201` (`4210` F-B10): a mismatch is named by its step, so a
    /// data read's is never dressed as the identity's.
    #[test]
    fn a_mismatch_is_named_by_its_step() {
        use aethelgard_ir::parts::StepKind;
        use aethelgard_twin::boot::STATUS_OK;
        let image = image();
        let mut c = image.identity.checks[0].clone();
        let bus = bus_word(&c);
        for (step, name) in [
            (StepKind::Identity, "identity"),
            (StepKind::Configure, "configure"),
            (StepKind::Data, "data"),
        ] {
            c.step = step;
            let value = !c.expected & aethelgard_ir::parts::width_mask(c.width);
            let f = identity_finding(&c, STATUS_OK, value).unwrap();
            assert_eq!(f.kind, format!("{bus}_{name}_mismatch"));
        }
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
        let hook = crate::anchor::boot_word_hook(0x2000_0000, &[]);
        // `AET-WO-3305`: the I2C step's status hook beside the boot word's.
        let step = crate::anchor::step_hook(0, image.identity.checks[0].status_at);
        assert!(static_.contains(&format!(
            "cpu VectorTableOffset 0x08000000; logLevel -1 sysbus.i2c1.u2; {hook}; {step}; emulation RunFor \"0.1000000005\"; echo \"AETHELGARD-BOOT-BEGIN\""
        )));
        assert!(!droop.contains("logLevel 3") && !static_.contains("logLevel 3"));
        assert_eq!(static_.matches("logLevel").count(), 1);
        assert!(static_.contains("sysbus ReadDoubleWord 0xe000ed04"));
        // `AET-WO-4230` D10: the flash controller's access log is on from the
        // firmware's load, on a board without a serial port too, and the
        // brown-out word is armed with the judge's fill and read back.
        let elf = static_.split("sysbus LoadELF @").nth(1).unwrap();
        assert!(
            elf.split_once("; ")
                .unwrap()
                .1
                .starts_with(&format!("{FLASH_LOG}; ")),
            "{static_}"
        );
        assert_eq!(static_.matches("LogPeripheralAccess").count(), 1);
        let word = image.brown_out.word_at;
        assert!(static_.contains(&format!("sysbus WriteDoubleWord {word:#010x} 0xffffffff")));
        let after_begin = static_.split("AETHELGARD-BOOT-BEGIN").nth(1).unwrap();
        assert!(after_begin.contains(&format!("; sysbus ReadDoubleWord {word:#010x}; ")));
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
            "emulation RunFor \"0.0010144305\"".to_owned(),
            "machine Reset".to_owned(),
        ]
        .join("; ");
        assert!(droop.contains(&want), "{droop}");
        let rest = [
            "cpu IsHalted true".to_owned(),
            "sysbus WriteDoubleWord 0x20000000 0x0".to_owned(),
            // The brown-out word (`AET-WO-4230` D10).
            rec(image.brown_out.word_at),
            rec(check.status_at),
            rec(check.value_at),
            "emulation RunFor \"0.0002135505\"".to_owned(),
            hook.clone(),
            step.clone(),
            "cpu IsHalted false".to_owned(),
            "echo \"AETHELGARD-RELEASE-0\"".to_owned(),
            "emulation RunFor \"0.0000000105\"".to_owned(),
            "emulation RunFor \"0.0000999905\"".to_owned(),
            "echo \"AETHELGARD-PROBE-BEGIN-0\"".to_owned(),
        ]
        .join("; ");
        assert!(droop.contains(&rest), "{droop}");
        assert!(droop.contains(
            "echo \"AETHELGARD-PROBE-END-0\"; emulation RunFor \"0.0986720205\"; echo \"AETHELGARD-BOOT-BEGIN\""
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
        assert_eq!(
            droop.matches("emulation RunFor \"0.0000000105\"").count(),
            1
        );
        assert_eq!(plan(&image, &resets).instructions(), 9_877_201);
        assert_eq!(plan(&image, &[]).instructions(), 10_000_000);
    }

    /// The nanoseconds Renode 1.17.0 runs for a `RunFor` argument, as its
    /// `TimeInterval.TryParse` computes them (renode-infrastructure
    /// `cd4b002aac2398994c4a61a34a8b31b5700facf2`, the pinned source,
    /// `src/Emulator/Main/Time/TimeInterval.cs` L53-59, a tick a nanosecond
    /// at L292): whole seconds exactly, the decimals as
    /// `(ulong)(double.Parse("0.<decimals>") * TicksPerSecond)`, truncated.
    fn renode_runs(arg: &str) -> u64 {
        let (seconds, decimals) = arg.split_once('.').unwrap();
        let fraction: f64 = format!("0.{decimals}").parse().unwrap();
        seconds.parse::<u64>().unwrap() * 1_000_000_000 + (fraction * 1e9) as u64
    }

    /// `AET-WO-4202`: `run_for` renders `ns` as `ns + 0.5` ns. Why that
    /// never truncates to anything but `ns`: the decimals are a fraction `f`
    /// below 1; `double.Parse` rounds it correctly (error at most 2^-53 f),
    /// and the product by 1e9 (exact in a double) rounds once more (2^-53
    /// of the product). The result is `ns + 0.5` within 2^-52 × 1e9 < 2.3e-7
    /// ns, so it lies strictly between `ns` and `ns + 1`, and the truncation
    /// is `ns`. The sweep checks it on every value the reasoning covers.
    #[test]
    fn every_run_for_runs_exactly_its_nanoseconds_in_renode() {
        let runs = |ns: u64| {
            let c = run_for(ns);
            renode_runs(
                c.strip_prefix("emulation RunFor \"")
                    .unwrap()
                    .strip_suffix('"')
                    .unwrap(),
            )
        };
        // The measured fault, by name: the nine-digit form runs 1,018,380 ns
        // 1 ns short; the rendering runs it exactly.
        assert_eq!(renode_runs("0.001018380"), 1_018_379);
        assert_eq!(runs(1_018_380), 1_018_380, "{}", run_for(1_018_380));
        // Every nanosecond of the first 2 ms, then a stride to the longest
        // `RunFor` any boot renders (the foreign window's bound).
        let longest = foreign::FOREIGN_MAX_VIRTUAL_NS;
        assert!(longest >= aethelgard_twin::boot::MAX_VIRTUAL_NS);
        for ns in (0..=2_000_000u64)
            .chain((2_000_000..=longest).step_by(9_973))
            .chain([longest])
        {
            assert_eq!(runs(ns), ns, "{}", run_for(ns));
        }
    }

    /// `AET-WO-4202` §5: the judge stays exact. A scheduled line (here the
    /// first injected byte, as the bench's boot logged it) is placed only at
    /// `start_ns + 10 ns × count`; stamped 1 ns before, it is refused (a
    /// core ahead of its time), and 1 ns after, it is the core's sleep.
    #[test]
    fn a_scheduled_line_is_placed_only_at_its_count_s_exact_instant() {
        let (start, count) = (1_231_740, 1_962_227);
        let line = |stamp| {
            let l = at(
                stamp,
                &format!("[WARNING] sysbus: AETHELGARD-INJECT 0 0 1952227 {count} 28a801 770"),
            );
            hooks_of(&[l.as_str()], start)
        };
        let exact = start + 10 * count;
        let (hooks, slept) = line(exact).unwrap();
        assert_eq!((hooks.len(), slept), (1, None));
        assert_eq!(hooks[0].2, exact);
        let early = line(exact - 1).unwrap_err();
        assert!(early.contains("but it is stamped 20854009 ns"), "{early}");
        let (_, slept) = line(exact + 1).unwrap();
        assert!(slept.is_some());
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

    /// `AET-WO-1603` §1.3: output a firmware reaches is tagged by its source,
    /// which is never a device's: semihosting's line is `cpu`'s (measured,
    /// FEEDBACK M16), a USART analyzer's is the USART's. A witness's text
    /// inside such a line is never a witness, wherever it falls in it.
    #[test]
    fn firmware_reachable_output_is_never_parsed_as_a_witness() {
        let forged = "[NOISY] i2c1.u2: Read 60 (2)";
        for line in [
            "[00:00:00.000100000] [WARNING] cpu: Semihosting handler is not registered; R0=0x4 R1=0x80000E6".to_owned(),
            format!("[00:00:00.000100000] [WARNING] cpu: {forged}"),
            format!("[00:00:00.000100000] [INFO] usart1: [host: 0.1s (+0.1s)|virt: 0s (+0s)] {forged}"),
            format!("[00:00:00.000100000] [INFO] usart1: {forged}"),
            format!("[00:00:00.000100000] [WARNING] usart1: Unhandled read from offset 0x3FC. {forged}"),
            forged.to_owned(),
            format!(" [00:00:00.000100000] {forged}"),
        ] {
            assert_eq!(witness_shaped(&line, "i2c1.u2", "[NOISY] "), None, "{line}");
        }
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
            label: step.name(),
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

    /// `AET-WO-3305`: each address found absent goes to the I2C step whose
    /// status write first follows it. The logs are the measured two-bme runs
    /// (`spike/3305`), whole lines with their stamps: held-low SDA, where
    /// each step meets its own device's absence, and U2's read sent to 0x75,
    /// where U5's step meets none.
    fn absent_of(checks: &[aethelgard_twin::boot::StepCheck], log: &[&str]) -> Vec<Vec<u8>> {
        let (hooks, slept) = hooks_of(log, 0).unwrap();
        assert_eq!(
            slept, None,
            "every measured hook line is placed at its stamp"
        );
        absent_by_step(checks, log, &hooks)
    }

    fn two_bme() -> Vec<aethelgard_twin::boot::StepCheck> {
        use aethelgard_ir::parts::StepKind::Identity;
        let i2c = |address| aethelgard_twin::boot::Wire::I2c { address };
        let spi = aethelgard_twin::boot::Wire::Spi {
            address: 0,
            select: "PA0".into(),
        };
        vec![
            step(i2c(0x76), "i2c1.u2", Identity, &[]),
            step(i2c(0x77), "i2c1.u5", Identity, &[]),
            step(spi, "spi1.spi1mux.u4", Identity, &[]),
        ]
    }

    const STEP0: &str = "[00:00:00.003600000] [WARNING] sysbus: AETHELGARD-STEP 0 22020001 361033";
    const ICR: &str = "[00:00:00.003600000] [WARNING] i2c1: Unhandled write to offset 0x1C. Unhandled bits: [4] when writing value 0x30. Tags: NACKCF (0x1).";

    #[test]
    fn an_absent_address_belongs_to_the_step_whose_status_write_follows_it() {
        let held = [
            "[00:00:00.000000000] [WARNING] i2c1: Unknown slave at address 118.",
            ICR,
            STEP0,
            "[00:00:00.003610340] [WARNING] i2c1: Unknown slave at address 119.",
            "[00:00:00.007200000] [WARNING] i2c1: Unhandled write to offset 0x1C. Unhandled bits: [4] when writing value 0x30. Tags: NACKCF (0x1).",
            "[00:00:00.007200000] [WARNING] sysbus: AETHELGARD-STEP 1 22020001 721093",
            "[00:00:00.007210940] [WARNING] sysbus: AETHELGARD-BOOTWORD 2100b007 0 721647",
        ];
        assert_eq!(
            absent_of(&two_bme(), &held),
            [vec![118], vec![119], vec![]],
            "on a faulted bus each step holds only its own device's address"
        );
        let wrong = [
            "[00:00:00.000000000] [WARNING] i2c1: Unknown slave at address 117.",
            ICR,
            STEP0,
            "[00:00:00.003610340] [NOISY] i2c1.u5: Write D0",
            "[00:00:00.003610340] [NOISY] i2c1.u5: Read 60",
            "[00:00:00.003610340] [WARNING] sysbus: AETHELGARD-STEP 1 22020000 361194",
            "[00:00:00.003611970] [WARNING] sysbus: AETHELGARD-BOOTWORD 2100b007 0 361750",
        ];
        assert_eq!(
            absent_of(&two_bme(), &wrong),
            [vec![117], vec![], vec![]],
            "the wrong address is U2's step's alone; U5's step met no absence"
        );
        // A status write directly after its step's line (mutant H3).
        let adjacent = [held[0], held[2], held[3], held[5]];
        assert_eq!(
            absent_of(&two_bme(), &adjacent),
            [vec![118], vec![119], vec![]]
        );
    }

    /// The adversarial pass (F1, F1b, F2, F3): the boundary is the
    /// firmware's, so where it is not drawn as the generated firmware draws
    /// it (each step's status once, after its transfer) a line goes to every
    /// step on its controller, as main's whole-run list did: the address can
    /// still mask, never hide a fault by moving to another device's step.
    #[test]
    fn an_absent_address_goes_to_every_step_when_the_boundary_is_not_one_write_each() {
        let both = |a: Vec<u8>| vec![a.clone(), a, vec![]];
        let u2 = "[00:00:00.000000000] [WARNING] i2c1: Unknown slave at address 117.";
        let early = "[00:00:00.000000000] [WARNING] sysbus: AETHELGARD-STEP 0 ffffffff 10";
        let late0 = "[00:00:00.003600000] [WARNING] sysbus: AETHELGARD-STEP 0 22020001 361033";
        let step1 = "[00:00:00.003610340] [WARNING] sysbus: AETHELGARD-STEP 1 22020000 361194";
        // F1: U2's status written before its transfer and after it.
        assert_eq!(
            absent_of(&two_bme(), &[early, u2, late0, step1]),
            both(vec![117])
        );
        // F2: U2's status never written; the line is not U5's alone.
        assert_eq!(absent_of(&two_bme(), &[u2, step1]), both(vec![117]));
        // F1b: every record marked before its transfer, then written after.
        let early1 = "[00:00:00.003600000] [WARNING] sysbus: AETHELGARD-STEP 1 ffffffff 361040";
        let held = [
            early,
            "[00:00:00.000000000] [WARNING] i2c1: Unknown slave at address 118.",
            late0,
            early1,
            "[00:00:00.003610340] [WARNING] i2c1: Unknown slave at address 119.",
            "[00:00:00.007200000] [WARNING] sysbus: AETHELGARD-STEP 1 22020001 721093",
        ];
        assert_eq!(absent_of(&two_bme(), &held), both(vec![118, 119]));
        // F3: a line after every step has ended is no one step's.
        let after = "[00:00:00.003611970] [WARNING] i2c1: Unknown slave at address 117.";
        assert_eq!(
            absent_of(&two_bme(), &[late0, step1, after]),
            both(vec![117])
        );
        // No status write read at all: every line to every step.
        assert_eq!(absent_by_step(&two_bme(), &[u2], &[]), both(vec![117]));
        // Another controller's line is none of these steps'.
        let other = "[00:00:00.000000000] [WARNING] i2c2: Unknown slave at address 118.";
        assert_eq!(absent_of(&two_bme(), &[other, late0, step1]), both(vec![]));
    }

    #[test]
    fn a_status_write_on_another_controller_ends_no_step_of_this_one() {
        // Adversarial mutant M7: an absent address on I2C1 followed only by
        // the status write of I2C2's step is still I2C1's last step's.
        use aethelgard_ir::parts::StepKind::Identity;
        let i2c = |address| aethelgard_twin::boot::Wire::I2c { address };
        let checks = [
            step(i2c(0x76), "i2c1.u2", Identity, &[]),
            step(i2c(0x47), "i2c2.u5", Identity, &[]),
        ];
        let log = [
            "[00:00:00.000000000] [WARNING] i2c1: Unknown slave at address 118.",
            "[00:00:00.003610340] [WARNING] sysbus: AETHELGARD-STEP 1 22020000 361034",
        ];
        assert_eq!(absent_of(&checks, &log), [vec![118], vec![]]);
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
            brown_out: 0,
            flash_access: None,
            loops: Vec::new(),
            registers: Vec::new(),
            records: vec![(aethelgard_twin::boot::STATUS_OK, 0); checks.len()],
            witnesses: seen.into_iter().map(Some).collect(),
            absent: vec![Vec::new(); checks.len()],
            motors: Vec::new(),
            serial: Vec::new(),
            unrouted,
            boot_word_ns: None,
            register_expected: Vec::new(),
            timers: Vec::new(),
            injections: Vec::new(),
        };
        witness_findings(checks, &observed)
    }

    /// `AET-WO-4220`: a model of our own logs a transaction when the
    /// multiplexer finishes it, after the window's closing edge and directly
    /// after `Finisihing transmission on device 0xa` (measured in the pinned
    /// image, `SPIMultiplexer.cs` L30-32, L99-108). That line is its window's;
    /// one after another address's finish, after the next edge, or with no
    /// finish line before it is not, and fails.
    #[test]
    fn a_line_logged_at_its_own_release_belongs_to_that_window() {
        use aethelgard_ir::parts::StepKind::{Configure, Identity};
        let at = |b: &str| format!("[00:00:00.010040000] {b}");
        let spi = || aethelgard_twin::boot::Wire::Spi {
            address: 1,
            select: "PA1".into(),
        };
        let d = "spi1.spi1mux.u7";
        let id = "[NOISY] spi1.spi1mux.u7: bank 0 read 0x75 -> 0x47";
        let cfg = "[NOISY] spi1.spi1mux.u7: bank 0 write 0x4E <- 0x03";
        let checks = [
            step(spi(), d, Identity, &[id]),
            step(spi(), d, Configure, &[cfg]),
        ];
        let (low, high) = (
            "[NOISY] spi1.spi1mux: GPIO #1 set to False",
            "[NOISY] spi1.spi1mux: GPIO #1 set to True",
        );
        let finish0 = "[NOISY] spi1.spi1mux: Finisihing transmission on device 0x0";
        let finish1 = "[NOISY] spi1.spi1mux: Finisihing transmission on device 0x1";
        let other = "[NOISY] spi1.spi1mux: GPIO #0 set to True";
        let judge = |log: Vec<&str>| -> Vec<String> {
            let log: Vec<String> = log.into_iter().map(at).collect();
            let log: Vec<&str> = log.iter().map(String::as_str).collect();
            step_findings(&checks, &log)
                .iter()
                .map(|f| f.kind.clone())
                .collect()
        };
        // As measured: each release finishes every deselected input in turn.
        assert_eq!(
            judge(vec![
                low, high, finish0, finish1, id, low, high, finish0, finish1, cfg
            ]),
            Vec::<String>::new()
        );
        // After another address's finish line: not this window's.
        assert!(
            !judge(vec![
                low, high, finish1, finish0, id, low, high, finish1, cfg
            ])
            .is_empty()
        );
        // After the multiplexer's next edge: not the closed window's, though a
        // finish line for its address comes before it.
        assert!(!judge(vec![low, high, other, finish1, id, low, high, finish1, cfg]).is_empty());
        // Swapped steps still fail.
        assert!(!judge(vec![low, high, finish1, cfg, low, high, finish1, id]).is_empty());
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
