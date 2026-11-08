//! `simulate`: Renode loads the platform and lists what it built. Renode exits
//! 0 even when the load fails (`architecture.md` §2.4), so the exit code is
//! recorded but never trusted alone. Two witnesses in the output: a CPU in the
//! `peripherals` listing (an entry followed by a `Slot:` line) and the absence
//! of `There was an error executing command`. A CPU listed beside an `[ERROR]`
//! log line is a failed load too (`platform_load_error`).

use crate::run::{Output, Stop};
use crate::said::{self, Style};
use crate::verdict::{Finding, Machine, Peripheral};

const TOOL: &str = "renode";
const LOAD_ERROR: &str = "There was an error executing command";
/// Renode's error log level; a CPU listed beside one is not a clean load.
const LOG_ERROR: &str = "[ERROR]";

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

/// The sentinels around a boot's reads. Each is matched as a whole line, so
/// the echo of the command line itself, which contains them inside quotes,
/// never matches.
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
    /// Every solved peripheral as the listing must name it: (name, model).
    pub instances: &'a [(String, String)],
}

impl BootPlan<'_> {
    /// The instructions a core that never stopped executes in the run.
    pub fn instructions(&self) -> u64 {
        aethelgard_twin::boot::MIPS * self.virtual_ns / 1000
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
        ];
        for (i, r) in self.image.registers.iter().enumerate() {
            reads.push((
                format!("sysbus ReadDoubleWord {:#010x}", r.address),
                Read::Register(i),
            ));
        }
        reads
    }

    /// The monitor commands: a fixed seed, the platform, the firmware, the
    /// cold RCC, a fixed virtual time, then the reads between sentinels under
    /// `logLevel 3`, then the listing. Both paths are this CLI's own copies.
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
        for r in &self.image.cold {
            c.push(format!("sysbus WriteDoubleWord {:#010x} 0x0", r.address));
        }
        // Only the firmware's code may set the boot word (adversarial A1, A2),
        // and the core takes its vector table where the part boots, not where
        // Renode guesses it from the ELF (adversarial A3).
        c.push(format!(
            "sysbus WriteDoubleWord {:#010x} 0x0",
            self.boot_word
        ));
        c.push(format!("cpu VectorTableOffset {:#010x}", self.vector_table));
        c.push(format!(
            "emulation RunFor \"{}.{:09}\"",
            self.virtual_ns / 1_000_000_000,
            self.virtual_ns % 1_000_000_000
        ));
        c.push("logLevel 3".to_owned());
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
    Register(usize),
}

/// A boot's findings, the listing, and what the reads gave.
pub type Judged = (Vec<Finding>, Option<Machine>, Option<Observed>);

/// What the reads gave, for the verdict.
pub struct Observed {
    pub boot_word: u32,
    pub instructions: u64,
    pub mips: u64,
    pub pc: u32,
    pub registers: Vec<u32>,
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
    let reads = plan.reads();
    let between = lines.get(begin + 1..end).unwrap_or_default();
    if end < begin || between.len() != reads.len() {
        return Err(untrusted(format!(
            "renode printed {} lines between the markers, not the {} reads",
            between.len(),
            reads.len()
        )));
    }
    let mut observed = Observed {
        boot_word: 0,
        instructions: 0,
        mips: 0,
        pc: 0,
        registers: vec![0; plan.image.registers.len()],
    };
    for ((cmd, read), line) in reads.iter().zip(between) {
        let v = value(line).ok_or_else(|| {
            let shown: String = line
                .chars()
                .take(120)
                .map(|c| {
                    if c.is_ascii_graphic() || c == ' ' {
                        c
                    } else {
                        '?'
                    }
                })
                .collect();
            untrusted(format!("`{cmd}` read `{shown}`, not a bare value"))
        })?;
        let word = || u32::try_from(v).map_err(|_| untrusted(format!("`{cmd}` read {v:#x}")));
        match *read {
            Read::BootWord => observed.boot_word = word()?,
            Read::Instructions => observed.instructions = v,
            Read::Mips => observed.mips = v,
            Read::Pc => observed.pc = word()?,
            Read::Register(i) => observed.registers[i] = word()?,
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
    if let Some(line) = lines.iter().find(|l| l.contains(LOG_ERROR)) {
        findings.push(Finding::plain("renode_error_log", line.trim().to_owned()));
    }
    let locked: Vec<&&str> = lines
        .iter()
        .filter(|l| LOCKED.iter().any(|w| l.contains(w)))
        .collect();
    if let Some(first) = locked.first() {
        findings.push(Finding::plain(
            "locked_peripheral_access",
            format!(
                "{} access(es) to a peripheral whose clock was off; the first: {}",
                locked.len(),
                first.trim()
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
    if !plan.executable.iter().any(|&(s, e)| pc >= s && pc < e) {
        findings.push(Finding::plain(
            "pc_outside_firmware",
            format!(
                "the core ended at {:#010x}, outside every executable segment of the firmware",
                observed.pc
            ),
        ));
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
    Ok((
        findings,
        Some(Machine { cpus, peripherals }),
        Some(observed),
    ))
}
