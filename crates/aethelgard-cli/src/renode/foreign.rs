//! The foreign-firmware judge (`AET-WO-4400e`, ruled on PR #212, comment
//! 6958822008; D4 on PR #215, comment 6961943794): a second judge, for a
//! firmware this project did not generate, chosen by `simulate --foreign`
//! and never inferred from the ELF (`INV-06`).
//!
//! It boots the firmware on the same emitted platform, under the same bounds
//! and the same `RETTOBASE` correction, but with none of the generated
//! runtime's contract: no boot word, no records in RAM, no hooks of ours, no
//! serial injection. What it says comes from witnesses the firmware cannot
//! write (`INV-07`): the models' own transaction lines and the platform's
//! own state. Every rule of today's judge is kept, replaced or dropped,
//! [`RULES`], and the verdict names each, with what is weakened.

use serde::Serialize;

use super::*;
use crate::verdict::{Finding, FirmwareReport, Machine};

/// What `boot.judge` says on a foreign verdict.
pub const JUDGE: &str = "foreign";

/// The longest boot `--foreign` admits, in virtual nanoseconds, and its name
/// in the verdict. **Provisional** (`AET-WO-4400e` D7, deferred until B1 and
/// B2 resolve): 4 s holds the ICM-42688-P's start (3.57 s) under the CLI's
/// own script at about 10 s of wall and 500 MiB on Furnace (`AET-WO-4400e3`
/// acceptance), inside the 30 s and 1 GiB bounds. D7 measures it with the
/// IMU's reads and sets it.
pub const FOREIGN_MAX_VIRTUAL_NS: u64 = 4_000_000_000;
pub const FOREIGN_WINDOW: &str = "FOREIGN_MAX_VIRTUAL_NS";

/// The rule table, in the order of the verdict: each of today's rules, what
/// a foreign boot does with it, and what that judges or weakens (`INV-07`).
/// `provisional` marks a rule the ruled table did not name, decided here and
/// sent to the architect.
pub const RULES: &[(&str, &str, &str)] = &[
    (
        "firmware memory",
        "replaced",
        "every PT_LOAD, at both addresses, inside flash or one RAM region ST's header states (seven regions), not flash and DTCM alone",
    ),
    (
        "PT_LOAD overlap",
        "replaced (provisional)",
        "no two segments' file bytes overlap; zero-filled segments may share a physical address",
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
        "CR1's frame bits (UE, TE, PS, PCE, M0, M1) and CR2's STOP against the solved framing",
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
        "--foreign's own bound, FOREIGN_MAX_VIRTUAL_NS (provisional until D7)",
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
        "the SPI model's slave-mode [ERROR] is exempt only directly after a CFG2 write with MASTER 0 while the last CR1 write left SPE 0, from the same access log; every exemption is counted in the verdict",
    ),
    (
        "renode_log_flood",
        "kept",
        "past the budget no witness is read whole: every step is unwitnessed",
    ),
    (
        "locked_peripheral_access",
        "kept (provisional)",
        "an access to a peripheral whose clock was off",
    ),
    (
        "brown-out record",
        "dropped (provisional)",
        "Weakens: the BOR level is not judged in the twin (a foreign firmware keeps no record)",
    ),
    (
        "flash controller access",
        "dropped (provisional)",
        "Weakens: flash erase and program are unjudged (a foreign firmware may keep storage in flash)",
    ),
    (
        "motor outputs",
        "dropped (provisional)",
        "Weakens: motor outputs are unjudged (their window is anchored on the generated bring-up's CEN)",
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
    pub observed: Option<String>,
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
    pub unobservable: Vec<String>,
    pub rules: Vec<RuleReport>,
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

/// The foreign boot's monitor commands: the platform, the `RETTOBASE`
/// correction, the firmware, the cold RCC, each modelled device's log level
/// and stimulus, each SPI controller's access log (D4), the run, then the
/// reads between the markers and the listing. No boot word, no record, no
/// hook of ours and no injection: none of the generated runtime's contract.
pub fn commands(plan: &BootPlan<'_>) -> String {
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
        RETTOBASE_HOOK.to_owned(),
        format!("sysbus LoadELF @{}", plan.firmware.display()),
    ]);
    c.extend(plan.cold());
    c.push(plan.vtor());
    c.extend(plan.witness_levels());
    c.extend(
        spi_controllers(plan)
            .iter()
            .map(|s| format!("sysbus LogPeripheralAccess sysbus.{s} true")),
    );
    c.extend(plan.stimuli());
    c.push(run_for(plan.virtual_ns));
    c.push(format!("echo \"{BEGIN}\""));
    c.extend(reads(plan).into_iter().map(|(cmd, _)| cmd));
    c.push(format!("echo \"{END}\""));
    c.push("peripherals".to_owned());
    c.push("quit".to_owned());
    c.join("; ")
}

/// What one read gives.
#[derive(Clone, Copy)]
enum ForeignRead {
    Instructions,
    Mips,
    Pc,
    Icsr,
    Register(usize),
}

fn reads(plan: &BootPlan<'_>) -> Vec<(String, ForeignRead)> {
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
    r
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

/// An SPI access log's write: the controller, the offset and the value.
fn spi_write(body: &str) -> Option<(&str, u64, u64)> {
    let (source, rest) = body.split_once(": [cpu: ")?;
    let source = source.rsplit(' ').next()?;
    let (_, rest) = rest.split_once("] Write")?;
    let (_, rest) = rest.split_once(" to 0x")?;
    let (offset, rest) = rest.split_once(' ')?;
    let (_, value) = rest.rsplit_once(", value 0x")?;
    Some((
        source,
        u64::from_str_radix(offset, 16).ok()?,
        u64::from_str_radix(value.trim(), 16).ok()?,
    ))
}

/// The run's first `[ERROR]` line D4 does not exempt, and how many it did:
/// the SPI model's slave-mode line directly after a `CFG2` (offset `0xC`)
/// write with `MASTER` (bit 22) 0, while the controller's last `CR1`
/// (offset 0) write left `SPE` (bit 0) 0. The bits are RM0433 Rev 9
/// §50.11.1 and §50.11.4's (`AET-WO-4400e2` D4).
fn error_lines<'l>(run_log: &[&'l str]) -> (Option<&'l str>, u64) {
    let mut cr1: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    let mut exempt = 0u64;
    let mut first = None;
    for (i, line) in run_log.iter().enumerate() {
        let body = log_text(line);
        if let Some((source, offset, value)) = spi_write(body) {
            if offset == 0 {
                cr1.insert(source.to_owned(), value);
            }
            continue;
        }
        if !line.contains(LOG_ERROR) {
            continue;
        }
        let exempted = body
            .strip_prefix("[ERROR] ")
            .and_then(|b| b.split_once(": "))
            .filter(|(source, text)| source.starts_with("spi") && *text == SLAVE_MODE)
            .is_some_and(|(source, _)| {
                i > 0
                    && spi_write(log_text(run_log[i - 1])).is_some_and(|(s, offset, value)| {
                        s == source
                            && offset == 0xC
                            && value & (1 << 22) == 0
                            && cr1.get(source).copied().unwrap_or(0) & 1 == 0
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

/// A foreign boot's report before anything is read: what it is judged by,
/// and the rule table. A boot killed at a bound reports this.
pub fn report(
    plan: &BootPlan<'_>,
    firmware: FirmwareReport,
    specification_sha256: String,
    regions: Vec<RegionReport>,
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
        unobservable: plan.image.unobservable.clone(),
        rules: RULES
            .iter()
            .map(|&(rule, disposition, judges)| RuleReport {
                rule,
                disposition,
                judges,
            })
            .collect(),
    }
}

/// The foreign judge: the run's findings, the listing, and the report.
pub fn judge(
    code: i32,
    out: &Output,
    plan: &BootPlan<'_>,
    firmware: FirmwareReport,
    specification_sha256: String,
    regions: Vec<RegionReport>,
    parts: &std::collections::BTreeMap<String, String>,
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
    let mut report = report(plan, firmware, specification_sha256, regions);
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
    let wanted = reads(plan);
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

    let mut findings = Vec::new();
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
            observed: Some(format!("{got:#010x}")),
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
    match &booted {
        Some(_) => {}
        None => findings.push(Finding::plain(
            "foreign_not_booted",
            "no declared device's model logged its identity transaction: nothing the firmware cannot write witnesses that it ran (D5; the console is refused)".to_owned(),
        )),
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
        let cr1 = |v: &str| {
            format!(
                "[00:00:00.100000000] [INFO] spi1: [cpu: 0x8100F7C] WriteUInt32 to 0x0 (Control1), value {v}"
            )
        };
        let cfg2 = |v: &str| {
            format!(
                "[00:00:00.100000000] [INFO] spi1: [cpu: 0x8100F82] WriteUInt32 to 0xC (Configuration2), value {v}"
            )
        };
        let err = format!("[00:00:00.100000000] [ERROR] spi1: {SLAVE_MODE}");
        let other = "[00:00:00.200000000] [ERROR] spi2: something else".to_owned();
        let run = |lines: &[String]| {
            let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
            let (first, n) = error_lines(&refs);
            (first.map(str::to_owned), n)
        };
        // spi_lld_stop: SPE cleared, CR1 = 0, CFG2 = 0: exempt.
        assert_eq!(run(&[cr1("0x0"), cfg2("0x0"), err.clone()]), (None, 1));
        // SPE still set: not exempt.
        assert_eq!(run(&[cr1("0x1"), cfg2("0x0"), err.clone()]).1, 0);
        // MASTER written 1 (the model logs nothing then, but if it did): not exempt.
        assert_eq!(run(&[cr1("0x0"), cfg2("0x400000"), err.clone()]).1, 0);
        // Not directly after the write: not exempt.
        assert_eq!(
            run(&[cr1("0x0"), cfg2("0x0"), other.clone(), err.clone()]).1,
            0
        );
        // Another controller's write: not exempt.
        let cfg2_spi2 = "[00:00:00.100000000] [INFO] spi2: [cpu: 0x8100F82] WriteUInt32 to 0xC (Configuration2), value 0x0".to_owned();
        assert_eq!(run(&[cr1("0x0"), cfg2_spi2, err.clone()]).1, 0);
        // Any other error is the first reported, exempt ones counted.
        assert_eq!(
            run(&[cr1("0x0"), cfg2("0x0"), err.clone(), other.clone()]),
            (Some(other), 1)
        );
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
}
