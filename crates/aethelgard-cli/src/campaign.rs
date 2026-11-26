//! `aethelgard campaign` and `aethelgard replay` (`AET-WO-2300`).
//!
//! `campaign` reads every scenario first, applies its overrides to its base
//! specification and solves it; any refusal stops the campaign before a run.
//! Each scenario then runs exactly `simulate --firmware`'s path ([`crate::simulate`]),
//! with the lines it holds low, and every record of the verdict is archived as
//! integer-only events in a run directory of its own, committed before the
//! scenario's line is printed. `replay` reads one run back through
//! `aethelgard-chaos`'s strict reader alone: no solver, no tool, no scratch
//! directory, nothing outside the run directory.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use aethelgard_chaos::campaign::{self as rules, Scenario, ScenarioError};
use aethelgard_chaos::record::{BusLine, Line, Override, Reading, Record, Window};
use aethelgard_chaos::{Event, Header, TraceWriter};
use aethelgard_ir::{IdentifierKind, SolvedTopology, Specification, check_identifier};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::input::hex;
use crate::verdict::{RegisterReport, Verdict, Word};
use crate::{
    Boot, EXIT_FAIL, EXIT_INTERNAL, EXIT_INVALID_SPEC, EXIT_UNREADABLE, Refusal, Scratch, emit,
    read_input, run,
};

/// One scenario, read, applied and solved, ready to run.
struct Prepared {
    path: PathBuf,
    /// The base specification file's own bytes, as read.
    base_sha256: [u8; 32],
    scenario: Scenario,
    specification_sha256: [u8; 32],
    topology: SolvedTopology,
    held: Vec<HeldLow>,
    /// A droop, checked against its domain, to be placed from the
    /// undisturbed boot word (`AET-WO-2310`).
    droop: Option<PreparedDroop>,
    /// A battery sag, judged statically; its instant is placed from the boot
    /// word and the deck solves it after the boot (`AET-WO-2310`).
    sag: Option<SagStatic>,
    /// What its verdict does not claim (`AET-WO-2304b` §1.7, §1.11).
    conditions: Vec<String>,
    run_id: [u8; 16],
    /// What the twin drives and reads of the pin devices (`AET-WO-1211c`).
    pins: aethelgard_twin::pins::Pins,
}

/// A battery sag, judged statically and solved by the deck, ready to archive.
struct SagSolved {
    record: Record,
    tools: Vec<(&'static str, String)>,
    sandbox: run::SandboxReport,
}

pub(crate) fn refuse(kind: &'static str, id: Option<&str>, message: String, code: u8) -> ExitCode {
    emit(&Refusal::new(kind, id, message), code)
}

pub(crate) fn digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// The bytes of `base`, relative to the scenario file's directory: no
/// absolute path, and a regular file inside the current directory once every
/// symlink is resolved. The file is opened once, without following a symlink
/// or blocking on a FIFO, and the open descriptor itself is checked and read,
/// so nothing swapped in by name after the check is ever read (`AET-WO-2300`
/// adversarial F2). Every refusal is exit 3: `(kind, message)`.
pub(crate) fn read_base(scenario: &Path, base: &str) -> Result<Vec<u8>, (&'static str, String)> {
    use rustix::fs::{Mode, OFlags};
    let outside = |m: String| ("BaseOutsideRoot", m);
    let relative = Path::new(base);
    if relative
        .components()
        .any(|c| matches!(c, Component::RootDir | Component::Prefix(_)))
    {
        return Err(outside(format!("base `{base}` is absolute")));
    }
    let root = std::env::current_dir()
        .and_then(fs::canonicalize)
        .map_err(|e| outside(format!("cannot resolve the current directory: {e}")))?;
    let dir = scenario.parent().unwrap_or(Path::new("."));
    let resolved = fs::canonicalize(dir.join(relative))
        .map_err(|e| outside(format!("cannot resolve base `{base}`: {e}")))?;
    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let file = rustix::fs::open(&resolved, flags, Mode::empty())
        .map(fs::File::from)
        .map_err(|e| outside(format!("cannot open base `{base}`: {e}")))?;
    // Where the descriptor really is, after any swap of a directory above it.
    let opened = rustix::fd::AsRawFd::as_raw_fd(&file);
    let opened = fs::read_link(format!("/proc/self/fd/{opened}"))
        .map_err(|e| outside(format!("cannot read where base `{base}` was opened: {e}")))?;
    if !opened.starts_with(&root) {
        return Err(outside(format!(
            "base `{base}` resolves to {}, outside the current directory {}",
            opened.display(),
            root.display()
        )));
    }
    let meta = file
        .metadata()
        .map_err(|e| ("UnreadableInput", format!("cannot read base `{base}`: {e}")))?;
    if !meta.is_file() {
        return Err(outside(format!("base `{base}` is not a regular file")));
    }
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(
        &mut std::io::Read::take(file, crate::MAX_INPUT_BYTES + 1),
        &mut bytes,
    )
    .map_err(|e| ("UnreadableInput", format!("cannot read base `{base}`: {e}")))?;
    if bytes.len() as u64 > crate::MAX_INPUT_BYTES {
        return Err((
            "InputTooLarge",
            format!("base `{base}` exceeds {} bytes", crate::MAX_INPUT_BYTES),
        ));
    }
    Ok(bytes)
}

/// The SPI devices a scenario leaves absent behind their chip selects
/// (`AET-WO-4200` D4), each checked against the solved topology by [`effective`].
pub(crate) fn absent(overrides: &[Override]) -> std::collections::BTreeSet<String> {
    overrides
        .iter()
        .filter_map(|o| match o {
            Override::SpiDeviceAbsent { reference } => Some(reference.clone()),
            _ => None,
        })
        .collect()
}

/// The serial ports a scenario holds open on their RX line (`AET-WO-2302`
/// D5), by label, each checked against the solved topology by [`effective`].
pub(crate) fn open(overrides: &[Override]) -> std::collections::BTreeSet<String> {
    overrides
        .iter()
        .filter_map(|o| match o {
            Override::SerialRxOpen { port } => Some(port.clone()),
            _ => None,
        })
        .collect()
}

/// A line the scenario holds low: its bus and the line's net.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeldLow {
    pub bus: String,
    pub net: String,
}

/// Apply a scenario's overrides to `spec` and validate the result; return
/// the lines it holds low. The runner does this, not `aethelgard-chaos`,
/// which never reads a `Specification` (`RUL-16`). An id the specification
/// does not have, or a result the specification's own rules refuse, is
/// refused (`INV-06`).
fn apply(scenario: &Scenario, spec: &mut Specification) -> Result<Vec<HeldLow>, ScenarioError> {
    apply_overrides(&scenario.overrides, spec)
}

/// [`apply`]'s path for any list of overrides: `propose` applies a
/// proposal's changes through it (`AET-WO-3200` §1.3), so a proposed
/// specification is made exactly as a scenario's is.
pub(crate) fn apply_overrides(
    overrides: &[Override],
    spec: &mut Specification,
) -> Result<Vec<HeldLow>, ScenarioError> {
    let mut held = Vec::new();
    for o in overrides {
        match o {
            Override::PullUpOhms { bus, ohms } => {
                bus_of(spec, bus)?.pull_up_ohms = *ohms;
            }
            Override::MaxCapacitance { net, femtofarads } => {
                let c = spec
                    .net_constraints
                    .iter_mut()
                    .find(|c| &c.net_name == net)
                    .ok_or_else(|| {
                        ScenarioError::new(
                            "UnknownNet",
                            Some(net),
                            format!("the specification constrains no net `{net}`"),
                        )
                    })?;
                c.max_capacitance_femtofarads = *femtofarads;
            }
            Override::LineHeldLow { bus, line } => {
                let b = bus_of(spec, bus)?;
                let net = match line {
                    Line::Sda => b.sda.net.clone(),
                    Line::Scl => b.scl.net.clone(),
                };
                held.push(HeldLow {
                    bus: bus.clone(),
                    net,
                });
            }
            // A droop changes no value of the specification: the runner
            // resolves and checks it against the domain (`resolve_droop`).
            Override::Droop(_) => {}
            // Nor does an absent SPI device: the platform leaves it out
            // behind its chip select (`absent`, `AET-WO-4200` D4). [`effective`]
            // refuses one the solved topology has no SPI device for (`RUL-16`).
            Override::SpiDeviceAbsent { .. } => {}
            // Nor does an open RX line: the composition injects nothing into
            // that port (`open`, `AET-WO-2302` D5). [`effective`] refuses one
            // the solved topology has no serial port for.
            Override::SerialRxOpen { .. } => {}
            // Nor does a battery sag: the chain is resolved against the tree
            // (`resolve_battery_sag`), nor a rail out of band: its consumers'
            // absences are the overrides listed beside it (`rendered`).
            Override::BatterySag(_) | Override::RailOutOfBand { .. } => {}
            // Nor does a held input or a set ADC value (`AET-WO-1211c`): the
            // twin drives them (`aethelgard_twin::pins::plan`, which
            // [`effective`] runs, refusing a net no pin device declares).
            Override::PinHeld { .. } | Override::AnalogValue { .. } => {}
        }
    }
    spec.validate().map_err(|e| {
        ScenarioError::new(
            e.kind(),
            e.id(),
            format!("the overrides leave the specification invalid: {e}"),
        )
    })?;
    Ok(held)
}

fn bus_of<'a>(
    spec: &'a mut Specification,
    bus: &str,
) -> Result<&'a mut aethelgard_ir::I2cBus, ScenarioError> {
    spec.i2c_buses
        .iter_mut()
        .find(|b| b.id == bus)
        .ok_or_else(|| {
            ScenarioError::new(
                "UnknownBus",
                Some(bus),
                format!("the specification has no I2C bus `{bus}`"),
            )
        })
}

/// Read, apply and solve one scenario; a refusal is printed and its code returned.
fn prepare(path: &Path, firmware_sha256: &[u8; 32]) -> Result<Prepared, ExitCode> {
    let scenario = read_scenario(path)?;
    let base = read_base(path, &scenario.base)
        .map_err(|(kind, m)| refuse(kind, Some(&scenario.name), m, EXIT_UNREADABLE))?;
    let Effective {
        spec,
        held,
        specification_sha256,
        topology,
        chain,
        pins,
    } = effective(&scenario, &base)?;
    let conditions = conditions(&scenario, &spec).map_err(|e| {
        refuse(
            e.kind,
            e.id.as_deref(),
            format!("scenario `{}`: {e}", scenario.name),
            EXIT_INVALID_SPEC,
        )
    })?;
    let droop = match scenario.droop() {
        Some(d) => Some(prepare_droop(d, &spec, &topology).map_err(|e| {
            refuse(
                e.kind,
                e.id.as_deref(),
                format!("scenario `{}`: {e}", scenario.name),
                EXIT_INVALID_SPEC,
            )
        })?),
        None => None,
    };
    let run_id = rules::run_id(&scenario.sha256, &specification_sha256, firmware_sha256);
    Ok(Prepared {
        path: path.to_path_buf(),
        base_sha256: digest(&base),
        scenario,
        specification_sha256,
        topology,
        held,
        droop,
        sag: chain,
        conditions,
        run_id,
        pins,
    })
}

/// What a battery sag's verdict does not claim (`AET-WO-2304b` §1.11),
/// verbatim in FAB-NOTES.
pub(crate) const SAG_LOAD: &str = "The battery's load is the tree's declared static load; no motor current is commanded, so no reboot loop is shown or excluded.";
pub(crate) const SAG_ADMITTED: &str = "A battery sag is admitted only while every battery-fed regulator stays at or above its stated regulation floor (D-B9); no brownout is shown or excluded.";
pub(crate) const SAG_OUTPUTS: &str = "Each battery-fed regulator's output is in its stated band by rule (D-B9), not solved: the deck solves VBAT only.";

/// The conditions `scenario`'s verdict names (`AET-WO-2304b` §1.7, §1.11):
/// a rail out of band's modelling assumption; a battery sag's load and
/// admission, the healthy R's source, and each motor output's `esc`
/// declaration with its source. Empty for every other scenario.
pub(crate) fn conditions(
    scenario: &Scenario,
    spec: &Specification,
) -> Result<Vec<String>, ScenarioError> {
    let mut out = Vec::new();
    if let Some(domain) = scenario.rail_out_of_band() {
        out.push(rail_condition(domain, &rendered(spec, domain)?));
    }
    if scenario.battery_sag().is_some() {
        out.extend([SAG_LOAD, SAG_ADMITTED, SAG_OUTPUTS].map(str::to_owned));
        if let Some(r) = spec
            .power
            .as_ref()
            .and_then(|p| p.battery.series_milliohms.as_ref())
        {
            out.push(format!(
                "The healthy path R is declared {} mΩ, a lower bound on the path where its source says so: {}",
                r.value, r.source
            ));
        }
        for m in &spec.motor_outputs {
            if let Some(aethelgard_ir::motor::Esc::NoEsc(n)) = &m.esc {
                out.push(format!(
                    "Motor output `{}` declares no ESC on the board's path: {}",
                    m.label, n.source
                ));
            }
        }
    }
    Ok(out)
}

/// Kind 31 as a campaign line states it (`AET-WO-2304b` §2): the chain as
/// declared, the solved minimum and the binding floor.
#[derive(Serialize)]
struct SagLine {
    series_milliohms: u32,
    /// The scenario's offset from the undisturbed boot word, and the instant
    /// it placed (`AET-WO-2310`).
    from_boot_word_ns: i64,
    start_ns: u64,
    v_open_microvolts: u64,
    healthy_milliohms: u32,
    sinks: Vec<SinkLine>,
    capacitance_picofarads: u64,
    static_microvolts: u64,
    solved_minimum_microvolts: u64,
    binding: String,
    binding_floor_microvolts: u64,
}

#[derive(Serialize)]
struct SinkLine {
    name: String,
    input_milliwatts: u64,
    shutdown_millivolts: u64,
    regulation_millivolts: u64,
}

fn sag_line(r: &Record) -> Option<SagLine> {
    let Record::BatterySagDeclared {
        sag,
        start_ns,
        v_open_microvolts,
        healthy_milliohms,
        sinks,
        capacitance_picofarads,
        static_microvolts,
        solved_minimum_microvolts,
        binding,
    } = r
    else {
        return None;
    };
    Some(SagLine {
        series_milliohms: sag.series_milliohms,
        from_boot_word_ns: sag.from_boot_word_ns,
        start_ns: *start_ns,
        v_open_microvolts: *v_open_microvolts,
        healthy_milliohms: *healthy_milliohms,
        sinks: sinks
            .iter()
            .map(|k| SinkLine {
                name: k.name.clone(),
                input_milliwatts: k.input_milliwatts,
                shutdown_millivolts: k.shutdown_millivolts,
                regulation_millivolts: k.regulation_millivolts,
            })
            .collect(),
        capacitance_picofarads: *capacitance_picofarads,
        static_microvolts: *static_microvolts,
        solved_minimum_microvolts: *solved_minimum_microvolts,
        binding_floor_microvolts: aethelgard_chaos::battery::binding(sinks)
            .map_or(0, aethelgard_chaos::record::SagSink::floor_microvolts),
        binding: binding.clone(),
    })
}

/// A droop checked against its domain before the boot, to be placed from the
/// undisturbed boot word ([`place_droop`], `AET-WO-2310`).
struct PreparedDroop {
    droop: aethelgard_chaos::record::Droop,
    /// The specification's `mcu.bor_level`: the scenario states none
    /// (`AET-WO-2310`; `AET-WO-4300d` D-B1, K8).
    bor_level: u8,
    nominal_millivolts: u32,
    tolerance_permille: u32,
    decoupling_picofarads: u64,
}

/// A droop's domain, nominal, tolerance and decoupling, resolved against the
/// effective specification, then checked against the supervisor's table at
/// the specification's BOR level (ruled D3, D7), every clause but its
/// instant's, which waits for the undisturbed boot word (`AET-WO-2310`). Only
/// the MCU's own supply can be drooped: its supervisor is the one the
/// campaign renders. Every refusal is the scenario's (exit 2).
fn prepare_droop(
    droop: &aethelgard_chaos::record::Droop,
    spec: &Specification,
    topology: &SolvedTopology,
) -> Result<PreparedDroop, ScenarioError> {
    let refuse = |kind: &'static str, m: String| ScenarioError::new(kind, Some(&droop.domain), m);
    if droop.domain != spec.mcu.supply {
        return Err(refuse(
            "DroopNotMcuSupply",
            format!(
                "the droop is on `{}`, not the MCU's supply `{}`",
                droop.domain, spec.mcu.supply
            ),
        ));
    }
    // belt-and-suspenders: the specification's own validation makes the MCU's
    // supply one of its domains; a domain missing here is refused, not unwrapped.
    let domain = spec
        .voltage_domains
        .iter()
        .find(|d| d.id == droop.domain)
        .ok_or_else(|| {
            refuse(
                "UnknownDomain",
                format!("the specification has no domain `{}`", droop.domain),
            )
        })?;
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part)
        .map_err(|e| refuse(e.kind(), format!("the pin table is unusable: {e}")))?;
    let decoupling = aethelgard_eda::schematic::decoupling_picofarads(spec, topology, &table)
        .map_err(|e| refuse(e.kind(), format!("the emitter's decoupling: {e}")))?;
    let decoupling_picofarads = decoupling.get(&droop.domain).copied().unwrap_or(0);
    rules::check_droop_shape(
        droop,
        spec.mcu.bor_level,
        domain.nominal_millivolts,
        decoupling_picofarads,
    )?;
    Ok(PreparedDroop {
        droop: droop.clone(),
        bor_level: spec.mcu.bor_level,
        nominal_millivolts: domain.nominal_millivolts,
        tolerance_permille: u32::from(domain.tolerance_permille),
        decoupling_picofarads,
    })
}

/// The undisturbed boot's word, or, when it wrote none, that boot's own end
/// as its verdict names it (`AET-WO-2314`, ruled D1; `INV-06`): every
/// finding but the information-only ones, `renode_log_flood` (its log was
/// cut, so no hook was read) and, for a boot killed at a bound, the memory
/// it reached. A boot word that reads `BOOT_MAGIC` though no hook saw it
/// written has no finding of its own, so it is named here. Every other end
/// has one: a word not read is a bound's, and one read that is not
/// `BOOT_MAGIC` is `boot_word_missing`'s (`spike/2314`, the six planted).
fn undisturbed(v: &Verdict) -> Result<u64, String> {
    if let Some(word) = v.boot.as_ref().and_then(|b| b.boot_word_ns) {
        return Ok(word);
    }
    let mut end: Vec<String> = v
        .findings
        .iter()
        .filter(|f| f.severity != "info" || f.kind == "renode_log_flood")
        .map(|f| format!("{}: {}", f.kind, f.message))
        .collect();
    let bounded = v.findings.iter().any(|f| {
        [rules::CPU_ABORTED_FINDING, rules::MEMORY_CAP_FINDING].contains(&f.kind.as_str())
    });
    if let Some(m) = v.memory.as_ref().filter(|_| bounded) {
        end.push(format!(
            "memory: Renode peaked at {} KiB against the bound's {} KiB",
            m.peak_rss_kib, m.bound_kib
        ));
    }
    let magic = format!("{:#010x}", aethelgard_twin::boot::BOOT_MAGIC);
    let flood = v.findings.iter().any(|f| f.kind == "renode_log_flood");
    if !flood
        && v.boot
            .as_ref()
            .and_then(|b| b.boot_word.observed.as_deref())
            == Some(magic.as_str())
    {
        end.push(format!(
            "its boot word reads {magic} at the run's end, but its hook saw no write of it"
        ));
    }
    Err(end.join("; "))
}

/// A planted instant from the undisturbed boot word (`AET-WO-2310`, ruled
/// Q-A (a)): before reset, past the run, or with no boot word to place it
/// from, the run is refused by name (exit 2), never planted at a default.
/// With no word, `word` holds that boot's own end ([`undisturbed`]), which
/// the refusal carries (`AET-WO-2314`).
fn place(
    scenario: &str,
    what: &str,
    word: &Result<u64, String>,
    from_boot_word_ns: i64,
) -> Result<u64, ExitCode> {
    let unplaced = |m: String| {
        refuse(
            rules::PLANTED_UNPLACED,
            Some(scenario),
            format!("scenario `{scenario}`: {m}"),
            EXIT_INVALID_SPEC,
        )
    };
    let word = match word {
        Ok(word) => *word,
        Err(end) => {
            return Err(unplaced(format!(
                "its {what} is planted {from_boot_word_ns} ns from the boot word, and the undisturbed boot wrote no boot word to place it from: {end}"
            )));
        }
    };
    match aethelgard_chaos::record::resolve_instant(word, from_boot_word_ns) {
        Some(at) if at <= aethelgard_twin::boot::MAX_VIRTUAL_NS => Ok(at),
        _ => Err(unplaced(format!(
            "its {what}, {from_boot_word_ns} ns from the boot word at {word} ns, lies at or before reset or past the run's {} ns",
            aethelgard_twin::boot::MAX_VIRTUAL_NS
        ))),
    }
}

/// The prepared droop at its instant: the rail the boot renders, checked
/// whole, its instant included ([`rules::check_droop`]).
fn place_droop(
    scenario: &str,
    d: &PreparedDroop,
    word: &Result<u64, String>,
) -> Result<crate::analog::Rail, ExitCode> {
    let start_ns = place(scenario, "droop", word, d.droop.from_boot_word_ns)?;
    let droop = aethelgard_chaos::record::RailDroop {
        droop: d.droop.clone(),
        start_ns,
        bor_level: d.bor_level,
    };
    rules::check_droop(
        &droop,
        d.nominal_millivolts,
        d.decoupling_picofarads,
        // A droop past the boot is refused with its time, never cut short.
        aethelgard_twin::boot::MAX_VIRTUAL_NS,
    )
    .map_err(|e| {
        refuse(
            e.kind,
            e.id.as_deref(),
            format!("scenario `{scenario}`: {e}"),
            EXIT_INVALID_SPEC,
        )
    })?;
    Ok(crate::analog::Rail {
        droop,
        nominal_millivolts: d.nominal_millivolts,
        tolerance_permille: d.tolerance_permille,
        decoupling_picofarads: d.decoupling_picofarads,
    })
}

/// One scenario file, read and parsed as `campaign` reads it; a refusal is
/// printed and its code returned.
pub(crate) fn read_scenario(path: &Path) -> Result<Scenario, ExitCode> {
    let bytes = read_input(Some(path)).map_err(|r| emit(&r, EXIT_UNREADABLE))?;
    let scenario = Scenario::parse(&bytes).map_err(|e| {
        let code = if e.kind == "InvalidJson" {
            EXIT_UNREADABLE
        } else {
            EXIT_INVALID_SPEC
        };
        refuse(
            e.kind,
            e.id.as_deref(),
            format!("{}: {e}", path.display()),
            code,
        )
    })?;
    // `AET-WO-1102`: a scenario's name is an identifier like any other, and it
    // reaches the verdict, the archive and `MANIFEST.json`. `aethelgard-chaos`
    // may not depend on `aethelgard-ir` (`workspace_shape`), so the CLI checks.
    if let Err(rule) = check_identifier(IdentifierKind::Plain, &scenario.name) {
        return Err(refuse(
            "SchemaViolation",
            None,
            format!(
                "{}: scenario name `{}` is not an identifier: {rule}",
                path.display(),
                scenario.name.escape_debug()
            ),
            EXIT_INVALID_SPEC,
        ));
    }
    Ok(scenario)
}

/// A scenario's effective specification: its base with its overrides, as
/// `campaign` judges it, with that specification's hash and its solved
/// topology.
pub(crate) struct Effective {
    pub spec: Specification,
    pub held: Vec<HeldLow>,
    pub specification_sha256: [u8; 32],
    pub topology: SolvedTopology,
    /// A battery sag the static checks admitted (`AET-WO-2304b` §1.6).
    pub chain: Option<SagStatic>,
    /// What the twin drives and reads of the pin devices, with the
    /// scenario's holds and values (`AET-WO-1211c`).
    pub pins: aethelgard_twin::pins::Pins,
}

/// Apply `scenario`'s overrides to its base's bytes, then serialize, hash
/// and solve, as `campaign` does before a run; a refusal is printed and its
/// code returned.
pub(crate) fn effective(scenario: &Scenario, base: &[u8]) -> Result<Effective, ExitCode> {
    let (mut spec, _) = crate::parse_spec(base).map_err(|(mut r, code)| {
        r.error.message = format!("scenario `{}`, base: {}", scenario.name, r.error.message);
        emit(&r, code)
    })?;
    let held = apply(scenario, &mut spec).map_err(|e| {
        refuse(
            e.kind,
            e.id.as_deref(),
            format!("scenario `{}`: {e}", scenario.name),
            EXIT_INVALID_SPEC,
        )
    })?;
    // The specification judged is the base with the overrides, as serialized:
    // its hash names it in the topology, the run id and the archive.
    let overridden = serde_json::to_vec(&spec).map_err(|e| {
        crate::diagnose(&format!("cannot serialize a specification: {e}"));
        ExitCode::from(EXIT_INTERNAL)
    })?;
    let specification_sha256 = digest(&overridden);
    let (topology, _) = crate::solve_spec(&spec, &hex(specification_sha256))?;
    // A campaign runs the generated firmware (`AET-WO-4412` D1 (a)).
    let topology = aethelgard_cli::generate::generated(&topology);
    if let Some(r) = absent(&scenario.overrides)
        .into_iter()
        .find(|r| !topology.spi_devices.contains_key(r))
    {
        return Err(refuse(
            "UnknownDevice",
            Some(&r),
            format!(
                "scenario `{}`: the specification declares no SPI device `{r}`",
                scenario.name
            ),
            EXIT_INVALID_SPEC,
        ));
    }
    if let Some(p) = open(&scenario.overrides)
        .into_iter()
        .find(|p| !topology.serial_ports.contains_key(p))
    {
        return Err(refuse(
            "UnknownSerialPort",
            Some(&p),
            format!(
                "scenario `{}`: the specification declares no serial port `{p}`",
                scenario.name
            ),
            EXIT_INVALID_SPEC,
        ));
    }
    let scenario_refusal = |e: ScenarioError| {
        refuse(
            e.kind,
            e.id.as_deref(),
            format!("scenario `{}`: {e}", scenario.name),
            EXIT_INVALID_SPEC,
        )
    };
    if let Some(domain) = scenario.rail_out_of_band() {
        let r = rendered(&spec, domain).map_err(scenario_refusal)?;
        let want = (r.ports, r.devices);
        let have = (open(&scenario.overrides), absent(&scenario.overrides));
        if have != want {
            return Err(scenario_refusal(ScenarioError::new(
                RAIL_REFUSED,
                Some(domain),
                format!(
                    "the scenario renders {have:?} (serial ports, SPI devices) absent, and `{domain}`'s consumers are {want:?}: a rail out of band is rendered by exactly its consumers' absence (AET-WO-2304b §1.7)"
                ),
            )));
        }
    }
    let chain = match scenario.battery_sag() {
        Some(sag) => Some(resolve_battery_sag(sag, &spec).map_err(scenario_refusal)?),
        None => None,
    };
    // A held input and a set ADC value (`AET-WO-1211c`): only a pin device's
    // declared input may be held (`PinHeldIsOutput` for any net the MCU
    // drives, `1211` §4 G4), and only its ADC input set, as the twin's own
    // plan refuses them.
    let overrides = aethelgard_twin::pins::Overrides {
        held: scenario
            .pins_held()
            .into_iter()
            .map(|(n, l)| (n.to_owned(), l == aethelgard_chaos::record::Level::High))
            .collect(),
        millivolts: scenario
            .analog_values()
            .into_iter()
            .map(|(n, mv)| (n.to_owned(), mv))
            .collect(),
    };
    let pins = aethelgard_twin::pins::plan(&topology, &overrides).map_err(|e| {
        let s = crate::twin_stop(e);
        refuse(
            s.kind,
            s.id.as_deref(),
            format!("scenario `{}`: {}", scenario.name, s.message),
            s.code,
        )
    })?;
    Ok(Effective {
        spec,
        held,
        specification_sha256,
        topology,
        chain,
        pins,
    })
}

use aethelgard_chaos::campaign::RAIL_REFUSED;

/// What a rail out of band renders (`AET-WO-2304b` §1.7, D4 option 1, K1):
/// each serial port whose RX net is on a connector that also carries the
/// rail, or each SPI device the rail supplies, each by its existing
/// composition; and, named but never counted, everything else on such a
/// connector (`EXT` on Sentinel-03's J8: no device).
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Rendering {
    pub ports: BTreeSet<String>,
    pub devices: BTreeSet<String>,
    /// What is named and not rendered, as the condition states it.
    pub unrendered: Vec<String>,
    /// Who the assumption is about: the ports' peer, by the connectors'
    /// declared standard, or the devices.
    who: String,
    /// The domain's tolerance floor, mV.
    band_millivolts: u64,
}

/// `a`, `a and b`, `a, b and c`.
fn listed(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// mV as volts with no trailing zero: 4750 is `4.75`.
fn volts(mv: u64) -> String {
    let frac = format!("{:03}", mv % 1000);
    let frac = frac.trim_end_matches('0');
    if frac.is_empty() {
        format!("{}", mv / 1000)
    } else {
        format!("{}.{frac}", mv / 1000)
    }
}

/// The modelling assumption every verdict that uses a rail out of band
/// names, verbatim in FAB-NOTES (`AET-WO-2304b` §1.7).
pub(crate) fn rail_condition(domain: &str, r: &Rendering) -> String {
    let ports: Vec<String> = r.ports.iter().cloned().collect();
    let devices: Vec<String> = r.devices.iter().cloned().collect();
    let mut rendered = Vec::new();
    if !ports.is_empty() {
        rendered.push(format!("{} RX open", listed(&ports)));
    }
    if !devices.is_empty() {
        rendered.push(format!("{} absent", listed(&devices)));
    }
    rendered.extend(r.unrendered.iter().cloned());
    format!(
        "`{domain}` below its band is rendered as its consumers' absence ({}). How {} below {} V is not declared; absent is a modelling assumption (AET-WO-2304 D4).",
        rendered.join("; "),
        r.who,
        volts(r.band_millivolts)
    )
}

/// [`Rendering`] of `domain`, or why a rail out of band on it is refused: an
/// unknown domain, the MCU's supply (a droop's), a consumer with no
/// composition (an I2C device), consumers of both kinds, whose joint
/// attribution is not modelled, or no consumer a judge observes.
pub(crate) fn rendered(spec: &Specification, domain: &str) -> Result<Rendering, ScenarioError> {
    let refuse = |m: String| ScenarioError::new(RAIL_REFUSED, Some(domain), m);
    let Some(v) = spec.voltage_domains.iter().find(|v| v.id == domain) else {
        return Err(refuse(format!(
            "the specification has no domain `{domain}`"
        )));
    };
    if domain == spec.mcu.supply {
        return Err(refuse(format!(
            "`{domain}` is the MCU's supply: a droop renders it, with the supervisor's resets"
        )));
    }
    let c = aethelgard_ir::power::rail_consumers(spec, domain);
    // A rail that feeds a regulator would take that regulator's output with
    // it, which no composition renders (adversarial 2).
    if let Some(r) = c.regulators.first() {
        return Err(refuse(format!(
            "`{domain}` feeds regulator {r}: a rail out of band would take its output down too, which is not modelled"
        )));
    }
    if let Some(d) = c.i2c_devices.first() {
        return Err(refuse(format!(
            "{d} is an I2C device on `{domain}`, and no composition renders an I2C device's absence"
        )));
    }
    if !c.serial_rx.is_empty() && !c.spi_devices.is_empty() {
        return Err(refuse(format!(
            "`{domain}` supplies serial ports {:?} and SPI devices {:?}: their joint attribution is not modelled",
            c.serial_rx, c.spi_devices
        )));
    }
    if c.serial_rx.is_empty() && c.spi_devices.is_empty() {
        return Err(refuse(format!(
            "`{domain}` has no consumer a judge observes"
        )));
    }
    let mut unrendered: Vec<String> = c
        .i2c_buses
        .iter()
        .map(|(bus, used)| {
            if *used {
                format!("`{bus}`'s off-board peer is not rendered")
            } else {
                format!("`{bus}` has no device and is not rendered")
            }
        })
        .collect();
    unrendered.extend(
        c.tx_only
            .iter()
            .map(|p| format!("`{p}`'s peer (on its TX line only) is not rendered")),
    );
    unrendered.extend(
        c.other_nets
            .iter()
            .map(|(j, net)| format!("`{net}` on {j} is not rendered")),
    );
    // The peer's standard, its first word, when every port's connector
    // declares one and they agree (`DS-009 TELEM` is a DS-009 peer).
    let firsts: BTreeSet<Option<&str>> = c
        .standards
        .iter()
        .map(|s| s.as_deref().and_then(|s| s.split(' ').next()))
        .collect();
    let who = if !c.spi_devices.is_empty() {
        let d: Vec<String> = c.spi_devices.iter().cloned().collect();
        format!(
            "{} {}",
            listed(&d),
            if d.len() == 1 { "behaves" } else { "behave" }
        )
    } else {
        match firsts.into_iter().collect::<Vec<_>>().as_slice() {
            [Some(std)] => format!("a {std} peer behaves"),
            _ => "an off-board peer behaves".to_owned(),
        }
    };
    Ok(Rendering {
        ports: c.serial_rx,
        devices: c.spi_devices,
        unrendered,
        who,
        band_millivolts: rules::tolerance_floor_uv(
            v.nominal_millivolts,
            u32::from(v.tolerance_permille),
        ) / 1000,
    })
}

/// A battery sag judged against the tree (`AET-WO-2304b` §1.6): refused with
/// no tree, with `series_milliohms` undeclared (`SeriesResistanceUndeclared`),
/// with a motor output whose `esc` is undeclared (`EscUndeclared`, the
/// first), and by every clause of [`aethelgard_chaos::battery::check_battery_sag`].
pub(crate) struct SagStatic {
    pub sag: aethelgard_chaos::record::BatterySag,
    pub side: aethelgard_ir::power::BatterySide,
    pub chain: aethelgard_chaos::battery::SagChain,
    pub static_microvolts: u64,
}

pub(crate) fn resolve_battery_sag(
    sag: &aethelgard_chaos::record::BatterySag,
    spec: &Specification,
) -> Result<SagStatic, ScenarioError> {
    use aethelgard_chaos::battery;
    let side = aethelgard_ir::power::battery_side(spec)
        .map_err(|e| ScenarioError::new(e.kind(), e.id(), e.to_string()))?
        .ok_or_else(|| {
            ScenarioError::new(
                battery::REFUSED,
                None,
                "no power tree: the specification declares no battery to sag".to_owned(),
            )
        })?;
    let Some(healthy) = &side.healthy else {
        return Err(ScenarioError::new(
            "SeriesResistanceUndeclared",
            None,
            "power.battery.series_milliohms is null: the healthy path R is undeclared, and a sag is the step from it (AET-WO-2304 D3)".to_owned(),
        ));
    };
    if let Some(m) = spec.motor_outputs.iter().find(|m| m.esc.is_none()) {
        return Err(ScenarioError::new(
            "EscUndeclared",
            Some(&m.label),
            format!(
                "motor output `{}` declares no ESC (`esc` is null): an ESC fed through the board would load the battery, so its absence must be declared (AET-WO-2304 D2)",
                m.label
            ),
        ));
    }
    if let Some(m) = spec
        .motor_outputs
        .iter()
        .find(|m| matches!(m.esc, Some(aethelgard_ir::motor::Esc::BoardVbat(_))))
    {
        return Err(ScenarioError::new(
            "EscCurrentUnmodelled",
            Some(&m.label),
            format!(
                "motor output `{}` declares an ESC fed through the board's VBAT copper, whose current no part row models yet (AET-WO-4500), so the sag cannot be computed",
                m.label
            ),
        ));
    }
    let chain = battery::SagChain {
        v_open_microvolts: side.v_open_microvolts,
        healthy_milliohms: healthy.value,
        sinks: side
            .sinks
            .iter()
            .map(|k| aethelgard_chaos::record::SagSink {
                name: k.reference.clone(),
                input_milliwatts: k.input_milliwatts,
                shutdown_millivolts: k.shutdown_millivolts,
                regulation_millivolts: k.regulation_millivolts,
            })
            .collect(),
        capacitance_picofarads: side.capacitors.iter().map(|c| c.picofarads).sum(),
    };
    // Before the boot: every clause but the instant's (`AET-WO-2310`).
    let static_microvolts =
        battery::check_battery_sag(sag, None, &chain, aethelgard_twin::boot::MAX_VIRTUAL_NS)?;
    Ok(SagStatic {
        sag: sag.clone(),
        side,
        chain,
        static_microvolts,
    })
}

/// The deck's solve of a sag the static checks admitted (`AET-WO-2304b`
/// §1.8): a floor crossed in the solve refuses the run by name
/// (`BatterySagSolvedBelowFloor`), the dynamic counterpart of the static
/// clauses; a solved minimum off the static point by more than the stated
/// tolerance is an internal error, never a verdict.
fn solve_battery_sag(s: &SagStatic, start_ns: u64, scenario: &str) -> Result<SagSolved, ExitCode> {
    let stop = |e: run::Stop| emit(&Refusal::new(e.kind, e.id.as_deref(), e.message), e.code);
    // The clauses of its instant, now that the boot word placed it.
    aethelgard_chaos::battery::check_battery_sag(
        &s.sag,
        Some(start_ns),
        &s.chain,
        aethelgard_twin::boot::MAX_VIRTUAL_NS,
    )
    .map_err(|e| {
        refuse(
            e.kind,
            Some(scenario),
            format!("scenario `{scenario}`: {e}"),
            EXIT_INVALID_SPEC,
        )
    })?;
    let scratch = Scratch::new().map_err(stop)?;
    let solved = crate::analog::chain(
        &s.sag,
        start_ns,
        &s.side,
        &s.chain,
        aethelgard_twin::boot::MAX_VIRTUAL_NS,
        &scratch.root.join("chain"),
        &scratch.root.join("status-chain"),
    )
    .map_err(stop)?;
    if let Some(c) = solved
        .crossings
        .iter()
        .find(|c| c.at_ps.is_some() || solved.minimum_microvolts < c.floor_microvolts)
    {
        return Err(refuse(
            "BatterySagSolvedBelowFloor",
            Some(&c.name),
            format!(
                "scenario `{scenario}`: the deck takes the battery to {} µV, below {}'s {} µV floor{}; the static point was {} µV",
                solved.minimum_microvolts,
                c.name,
                c.floor_microvolts,
                c.at_ps
                    .map_or_else(String::new, |t| format!(", crossing it at {t} ps")),
                s.static_microvolts
            ),
            EXIT_INVALID_SPEC,
        ));
    }
    if solved.minimum_microvolts.abs_diff(s.static_microvolts)
        > aethelgard_chaos::battery::SOLVE_TOLERANCE_UV
    {
        crate::diagnose(&format!(
            "scenario `{scenario}`: the deck's minimum {} µV and the static point {} µV disagree by more than {} µV",
            solved.minimum_microvolts,
            s.static_microvolts,
            aethelgard_chaos::battery::SOLVE_TOLERANCE_UV
        ));
        return Err(ExitCode::from(EXIT_INTERNAL));
    }
    let binding = aethelgard_chaos::battery::binding(&s.chain.sinks)
        .map(|k| k.name.clone())
        .unwrap_or_default();
    Ok(SagSolved {
        record: Record::BatterySagDeclared {
            sag: s.sag.clone(),
            start_ns,
            v_open_microvolts: s.chain.v_open_microvolts,
            healthy_milliohms: s.chain.healthy_milliohms,
            sinks: s.chain.sinks.clone(),
            capacitance_picofarads: s.chain.capacitance_picofarads,
            static_microvolts: s.static_microvolts,
            solved_minimum_microvolts: solved.minimum_microvolts,
            binding,
        },
        tools: solved.tools,
        sandbox: solved.sandbox,
    })
}

#[derive(Serialize)]
struct ScenarioLine {
    scenario: String,
    file: String,
    verdict: &'static str,
    /// The scenario's boot's lifecycle banner (`AET-WO-4210` D1); absent when
    /// it is empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    lifecycle: Vec<String>,
    expect: &'static str,
    matched: bool,
    findings: Vec<String>,
    expect_findings: Vec<String>,
    run_id: String,
    run: String,
    state_sequence_hash: String,
    event_count: u64,
    trace_bytes: u64,
    /// The boot's virtual time (`AET-WO-4300a` D1). The line names no window:
    /// every board's is `MAX_VIRTUAL_NS` (`AET-WO-2310`, `2302` reduction 2).
    virtual_time_ns: u64,
    /// The boot's `uid_boundary` and, when false, why (`AET-WO-1601`).
    uid_boundary: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    uid_boundary_unmet: Option<run::Unmet>,
    /// A battery sag's chain (kind 31); absent otherwise (`AET-WO-2304b`).
    #[serde(skip_serializing_if = "Option::is_none")]
    battery_sag: Option<SagLine>,
    /// What the verdict does not claim; absent when nothing.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    conditions: Vec<String>,
    /// A droop's observations (`AET-WO-2306`): the releases whose recovery
    /// was judged against a device's witness, and `nothing_in_scope` when
    /// the boot reads no device; absent when the droop released nothing.
    #[serde(skip_serializing_if = "Option::is_none")]
    observation_witness: Option<crate::verdict::Scope>,
}

/// Why a droop's observations judged no device (`AET-WO-2306`).
pub(crate) const NO_WITNESSED_DEVICE: &str = "no witnessed device: the boot reads no modelled device's identity (an unmodelled one's is not witnessable), so each recovery is judged by the boot word alone";

/// The witness scope of a run's droop observations, from the records replay
/// judges: each release has one observation, judged against the first
/// identity read's device's steps (`first_script`: the first I2C witness
/// record's device, else the first SPI one's; `AET-WO-2305` §1.3), and with
/// neither there is no device to witness. A release whose probe a killed
/// boot never read, or that has no recovery bound (`AET-WO-2305` §1.2), is
/// in scope too: it is judged, as `brownout_unrecovered`. `None` when no reset was released, so
/// nothing was observed.
fn observation_witness(records: &[Record]) -> Option<crate::verdict::Scope> {
    let releases = records
        .iter()
        .filter(|r| matches!(r, Record::ResetRendered { released: true, .. }))
        .count() as u64;
    let device = records
        .iter()
        .any(|r| matches!(r, Record::I2cWitness { .. } | Record::SpiWitness { .. }));
    (releases > 0)
        .then(|| crate::verdict::Scope::of(if device { releases } else { 0 }, NO_WITNESSED_DEVICE))
}

#[derive(Serialize)]
struct CampaignSummary {
    ok: bool,
    verdict: &'static str,
    check: &'static str,
    scenarios: usize,
    matched: usize,
    /// Every scenario's boot, `MAX_VIRTUAL_NS` (`AET-WO-2302`).
    virtual_time_ns: u64,
    trace_bytes: u64,
}

fn pass_fail(pass: bool) -> &'static str {
    if pass { "pass" } else { "fail" }
}

/// `aethelgard campaign --scenarios <file>... --firmware <elf> --archive <dir>`.
pub fn run(scenarios: &[PathBuf], firmware: &Path, archive: &Path) -> ExitCode {
    let firmware_sha256 = match read_input(Some(firmware)) {
        Ok(bytes) => digest(&bytes),
        Err(r) => return emit(&r, EXIT_UNREADABLE),
    };
    let mut prepared = Vec::with_capacity(scenarios.len());
    for path in scenarios {
        match prepare(path, &firmware_sha256) {
            Ok(p) => prepared.push(p),
            Err(code) => return code,
        }
    }
    let mut ids = BTreeSet::new();
    for p in &prepared {
        if !ids.insert(p.run_id) {
            return refuse(
                "DuplicateScenario",
                Some(&p.scenario.name),
                format!(
                    "{} names the same run as an earlier scenario: the same file, specification and firmware",
                    p.path.display()
                ),
                EXIT_INVALID_SPEC,
            );
        }
    }
    if let Err(e) = fs::create_dir_all(archive) {
        return refuse(
            "UnwritableOutput",
            None,
            format!("cannot create the archive {}: {e}", archive.display()),
            EXIT_UNREADABLE,
        );
    }
    // Every run is new: one that exists is refused before anything runs.
    for p in &prepared {
        let dir = archive.join(hex(p.run_id));
        if fs::symlink_metadata(&dir).is_ok() {
            return refuse(
                "RunExists",
                Some(&hex(p.run_id)),
                format!(
                    "{} already exists: the archive is append-only",
                    dir.display()
                ),
                EXIT_UNREADABLE,
            );
        }
    }

    // The host's toolchain, once for the campaign (`AET-WO-2310`, Q-T T1).
    let toolchain = match Scratch::new().and_then(|s| toolchain_entry(&s)) {
        Ok(t) => t,
        Err(s) => return emit(&Refusal::new(s.kind, s.id.as_deref(), s.message), s.code),
    };
    let (mut matched, mut total) = (0, 0u64);
    for p in prepared {
        let (line, ok) = match run_one(p, firmware, &firmware_sha256, archive, &toolchain) {
            Ok(done) => done,
            Err(code) => return code,
        };
        total += line.trace_bytes;
        let code = emit(&line, 0);
        if code != ExitCode::SUCCESS {
            return code;
        }
        matched += usize::from(ok);
    }
    let all = matched == scenarios.len();
    emit(
        &CampaignSummary {
            ok: all,
            verdict: pass_fail(all),
            check: "campaign",
            scenarios: scenarios.len(),
            matched,
            virtual_time_ns: aethelgard_twin::boot::MAX_VIRTUAL_NS,
            trace_bytes: total,
        },
        if all { 0 } else { EXIT_FAIL },
    )
}

/// Boot one scenario, archive its records and commit the run.
fn run_one(
    p: Prepared,
    firmware: &Path,
    firmware_sha256: &[u8; 32],
    archive: &Path,
    toolchain: &(String, Option<[u8; 32]>),
) -> Result<(ScenarioLine, bool), ExitCode> {
    let stop = |s: run::Stop| {
        let code = s.code;
        emit(&Refusal::new(s.kind, s.id.as_deref(), s.message), code)
    };
    let scratch = Scratch::new().map_err(stop)?;
    let platform = scratch.tool.join("platform.repl");
    let text =
        aethelgard_twin::platform::emit(&p.topology).map_err(|e| stop(crate::twin_stop(e)))?;
    fs::write(&platform, text).map_err(|e| {
        stop(run::Stop::tool_error(
            "aethelgard",
            format!("cannot write the scenario's platform: {e}"),
        ))
    })?;
    let topology_sha256 = digest(&serde_json::to_vec(&p.topology).map_err(|e| {
        crate::diagnose(&format!("cannot serialize a topology: {e}"));
        ExitCode::from(EXIT_INTERNAL)
    })?);
    let buses = solved_buses(&p.topology).map_err(|m| {
        crate::diagnose(&m);
        ExitCode::from(EXIT_INTERNAL)
    })?;
    let spi_buses = solved_spi(&p.topology).map_err(|e| {
        crate::diagnose(&e.to_string());
        ExitCode::from(EXIT_INTERNAL)
    })?;
    let motors = solved_motors(&p.topology);
    let serial = solved_serial(&p.topology);
    let name = p.scenario.name.clone();
    let boot = |rail: Option<crate::analog::Rail>| {
        crate::simulate(
            &platform,
            Some(Boot {
                firmware: firmware.to_path_buf(),
                topology: p.topology.clone(),
                virtual_ns: aethelgard_twin::boot::MAX_VIRTUAL_NS,
                scenario: name.clone(),
                held: p.held.clone(),
                absent: absent(&p.scenario.overrides),
                open: open(&p.scenario.overrides),
                rail,
                foreign: false,
                pins: p.pins.clone(),
                // Never a test's hang guard: a campaign, and so `verify`,
                // boots under the constant (`AET-WO-1918` (2)).
                bounds: crate::renode::BOOT_BOUNDS,
            }),
        )
        .map_err(stop)
    };
    // A planted event is placed from the undisturbed boot word (`AET-WO-2310`,
    // ruled Q-A (a)). A droop's boot is disturbed, so the same prepared boot
    // runs first without it; a battery sag never reaches the boot, so the
    // run's own boot is the undisturbed one and the deck solves after it.
    let (verdict, reference) = match &p.droop {
        Some(d) => {
            let word = undisturbed(&boot(None)?);
            let rail = place_droop(&name, d, &word)?;
            (boot(Some(rail))?, word)
        }
        None => {
            let v = boot(None)?;
            let word = undisturbed(&v);
            (v, word)
        }
    };
    let sag = match &p.sag {
        Some(s) => {
            let at = place(&name, "battery sag", &reference, s.sag.from_boot_word_ns)?;
            Some(solve_battery_sag(s, at, &name)?)
        }
        None => None,
    };
    drop(scratch);
    let mut sandbox = verdict.sandbox;
    if let Some(s) = &sag {
        sandbox.unmet(s.sandbox.uid_boundary_unmet);
    }
    let (uid_boundary, uid_boundary_unmet) = (sandbox.uid_boundary, sandbox.uid_boundary_unmet);
    // The run id and the header name the firmware as first read; the boot
    // judged its own copy. A file changed between the two is refused.
    let booted = verdict.boot.as_ref().map(|b| b.firmware.sha256.as_str());
    if booted != Some(hex(firmware_sha256).as_str()) {
        return Err(refuse(
            "FirmwareChanged",
            Some(&p.scenario.name),
            format!(
                "{} changed while the campaign ran: the boot judged sha256 {}, the campaign read {}",
                firmware.display(),
                booted.unwrap_or("none"),
                hex(firmware_sha256)
            ),
            EXIT_UNREADABLE,
        ));
    }

    let internal = |m: String| {
        crate::diagnose(&m);
        ExitCode::from(EXIT_INTERNAL)
    };
    let mut records = vec![p.scenario.record()];
    records.extend(p.scenario.counts_record());
    records.extend([Record::Solved {
        specification_sha256: p.specification_sha256,
        topology_sha256,
    }]);
    records.extend(buses);
    records.extend(spi_buses);
    records.extend(motors);
    records.extend(serial);
    records.extend(pin_declared(&p.pins));
    let mut boot = records_of(&verdict, p.scenario.droop()).map_err(internal)?;
    // Kind 31 sits with the declared rail, after the analog records and the
    // bus faults, before the boot's (`check_run`'s rank 6).
    if let Some(s) = &sag {
        let at = boot
            .iter()
            .position(|r| {
                !matches!(
                    r,
                    Record::RiseComputed { .. }
                        | Record::LineStuckHigh { .. }
                        | Record::BusFault { .. }
                )
            })
            .unwrap_or(boot.len());
        boot.insert(at, s.record.clone());
    }
    // Kind 32, the reference every planted run holds (`AET-WO-2310`): a
    // droop's boot wrote it when it released a reset, from the same boot run
    // undisturbed, which must be the word the campaign placed from;
    // otherwise it is that word, beside the rail or the sag.
    if let Some(word) = reference
        .ok()
        .filter(|_| p.droop.is_some() || sag.is_some())
    {
        let want = Record::RecoveryBound {
            boot_word_ns: Some(word),
            margin_ns: aethelgard_chaos::power::RECOVERY_MARGIN_NS,
            bound_ns: aethelgard_chaos::power::recovery_bound(word),
        };
        match boot
            .iter()
            .position(|r| matches!(r, Record::RecoveryBound { .. }))
        {
            Some(i) if boot[i] == want => {}
            // belt-and-suspenders: the two undisturbed boots are one prepared
            // boot run twice by a deterministic Renode, so they agree; a
            // stub cannot make them differ under the sandbox (no state it
            // may write), so no test reaches this. If they ever differ, the
            // instant and the bound would come from two words: stop.
            Some(i) => {
                return Err(internal(format!(
                    "the droop's undisturbed boot gave {:?}, and the campaign placed its instant from the boot word at {word} ns",
                    boot[i]
                )));
            }
            None => {
                let at = boot
                    .iter()
                    .position(|r| {
                        matches!(
                            r,
                            Record::RailDeclared { .. } | Record::BatterySagDeclared { .. }
                        )
                    })
                    .map_or(boot.len(), |i| i + 1);
                boot.insert(at, want);
            }
        }
    }
    records.extend(boot);
    // The run must pass the rules replay will judge it by before it is written.
    let summary = rules::check_run(&records)
        .and_then(|s| rules::check_serial_frames(firmware_sha256, &records).map(|()| s))
        .map_err(|reason| internal(format!("the campaign built a run replay refuses: {reason}")))?;

    let header = Header {
        run_id: p.run_id,
        artifacts: vec![
            ("base".to_owned(), p.base_sha256),
            ("firmware".to_owned(), *firmware_sha256),
            ("scenario".to_owned(), p.scenario.sha256),
            ("specification".to_owned(), p.specification_sha256),
            ("topology".to_owned(), topology_sha256),
        ],
        tools: Vec::new(),
        tool_sha256: Vec::new(),
    };
    let header = header_tools(
        header,
        &verdict.tools,
        sag.as_ref().map_or(&[][..], |s| &s.tools),
        toolchain,
    )
    .map_err(internal)?;
    let observation_witness = observation_witness(&records);
    let dir = archive.join(hex(p.run_id));
    let archived = (|| {
        let mut writer = TraceWriter::create(&dir, &header)?;
        for record in records {
            let timestamp = record.timestamp(aethelgard_twin::boot::MAX_VIRTUAL_NS);
            writer.append(&Event::Record { timestamp, record })?;
        }
        writer.commit()
    })()
    .map_err(|e| {
        refuse(
            "ArchiveWriteFailed",
            Some(&hex(p.run_id)),
            e.to_string(),
            EXIT_UNREADABLE,
        )
    })?;
    let ok = summary.matched;
    let battery_sag = sag.as_ref().and_then(|s| sag_line(&s.record));
    Ok((
        ScenarioLine {
            scenario: summary.name,
            file: p.path.display().to_string(),
            verdict: pass_fail(summary.pass),
            lifecycle: verdict.lifecycle.clone(),
            expect: pass_fail(summary.expect_pass),
            matched: ok,
            findings: summary.failing_kinds,
            expect_findings: summary.expect_findings,
            run_id: hex(p.run_id),
            run: dir.display().to_string(),
            state_sequence_hash: hex(archived.state_hash),
            event_count: archived.event_count,
            trace_bytes: archived.trace_bytes,
            virtual_time_ns: aethelgard_twin::boot::MAX_VIRTUAL_NS,
            uid_boundary,
            uid_boundary_unmet,
            battery_sag,
            conditions: p.conditions,
            observation_witness,
        },
        ok,
    ))
}

/// The header's tools: the boot's, the bridge and ngspice the battery chain's
/// deck ran (`AET-WO-2304b` §1.9), and the host's toolchain, sorted, each
/// once; ngspice's library hash and the toolchain's sysroot digest beside
/// their names (`AET-WO-2310`). A tool that named two versions in one run,
/// or a library hash with no ngspice, is an internal error, never a choice.
fn header_tools(
    mut header: Header,
    boot: &[(&'static str, String)],
    chain: &[(&'static str, String)],
    toolchain: &(String, Option<[u8; 32]>),
) -> Result<Header, String> {
    let mut tools: Vec<(String, String)> = boot
        .iter()
        .chain(chain)
        .map(|(n, v)| ((*n).to_owned(), v.clone()))
        .chain([(rules::RUST_TOOLCHAIN.to_owned(), toolchain.0.clone())])
        .collect();
    tools.sort();
    tools.dedup();
    if let Some(w) = tools.windows(2).find(|w| w[0].0 == w[1].0) {
        return Err(format!(
            "{} reported two versions in one run: `{}`, `{}`",
            w[0].0, w[0].1, w[1].1
        ));
    }
    let library = tools
        .iter()
        .position(|(n, _)| n == crate::analog::NGSPICE_LIBRARY)
        .map(|i| tools.remove(i).1);
    let ngspice = tools.iter().any(|(n, _)| n == rules::NGSPICE);
    let mut hashes = Vec::new();
    match (library, ngspice) {
        (Some(hex), true) => hashes.push((rules::NGSPICE.to_owned(), unhex(&hex)?)),
        (None, false) => {}
        _ => return Err("ngspice and its library's hash do not come together".to_owned()),
    }
    if let Some(h) = toolchain.1 {
        hashes.push((rules::RUST_TOOLCHAIN.to_owned(), h));
    }
    hashes.sort();
    header.tools = tools;
    header.tool_sha256 = hashes;
    Ok(header)
}

/// 64 lower-case hex digits as 32 bytes.
fn unhex(s: &str) -> Result<[u8; 32], String> {
    let bad = || format!("`{s}` is not a sha256 in lower-case hex");
    if s.len() != 64
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(bad());
    }
    let mut out = [0u8; 32];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).map_err(|_| bad())?;
    }
    Ok(out)
}

/// The host's Rust toolchain for the header (`AET-WO-2310`, ruled Q-T T1):
/// `rustc -V`'s version, `rustc ` cut, and [`tree_sha256`] of the sysroot
/// `rustc --print sysroot` names, both run as `build` runs cargo (the same
/// variables, from a scratch directory, so the same toolchain is active). It
/// is the host's, not proven to be the one that built the ELF. On a host
/// with no `rustc` on `PATH` or no `RUSTUP_HOME`, it is
/// [`rules::TOOLCHAIN_ABSENT`] with no hash; any other failure stops.
fn toolchain_entry(scratch: &Scratch) -> Result<(String, Option<[u8; 32]>), run::Stop> {
    let absent = || Ok((rules::TOOLCHAIN_ABSENT.to_owned(), None));
    let rustc = match run::Tool::find("rustc") {
        Ok(t) => t,
        Err(s) if s.kind == "ToolMissing" => return absent(),
        Err(s) => return Err(s),
    };
    let Some(rustup_home) = std::env::var_os("RUSTUP_HOME").filter(|v| !v.is_empty()) else {
        return absent();
    };
    let mut env = vec![
        ("RUSTUP_HOME", rustup_home.clone()),
        ("PATH", std::env::var_os("PATH").unwrap_or_default()),
    ];
    if let Some(t) = std::env::var_os("RUSTUP_TOOLCHAIN").filter(|v| !v.is_empty()) {
        env.push(("RUSTUP_TOOLCHAIN", t));
    }
    let mut read_only = rustc.dirs();
    read_only.push(PathBuf::from(&rustup_home));
    let ask = |args: &[&str], status: &str| -> Result<String, run::Stop> {
        let out = run::Invocation {
            tool: &rustc,
            args: run::args(args),
            env: env.clone(),
            workdir: &scratch.tool,
            cwd: scratch.tool.clone(),
            status: scratch.root.join(status),
            read_only: read_only.clone(),
            read_write: Vec::new(),
        }
        .run()?;
        if out.code("rustc")? != 0 {
            return Err(run::Stop::tool_error(
                "rustc",
                format!(
                    "`rustc {}` exited non-zero: {}",
                    args.join(" "),
                    out.text().trim()
                ),
            ));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
    };
    let said = ask(&["-V"], "status-rustc-version")?;
    let version = said
        .strip_prefix("rustc ")
        .filter(|v| {
            aethelgard_chaos::event::text_ok(v, 1, aethelgard_chaos::event::MAX_TOOL_VERSION_BYTES)
        })
        .ok_or_else(|| {
            run::Stop::tool_error(
                "rustc",
                format!("`rustc -V` said `{}`", said.escape_debug()),
            )
        })?
        .to_owned();
    let sysroot = PathBuf::from(ask(&["--print", "sysroot"], "status-rustc-sysroot")?);
    let digest = tree_sha256(&sysroot).map_err(|e| {
        run::Stop::tool_error("rustc", format!("its sysroot {}: {e}", sysroot.display()))
    })?;
    Ok((version, Some(digest)))
}

/// The sha256 of a directory tree (`AET-WO-2310`), the bridge source
/// digest's recipe (`bridge/build.rs`) over every entry under `root`, sorted
/// by its path relative to `root` (byte order, `/`): for a regular file, the
/// path, one 0x00 byte, its length as 8 bytes little-endian, then its bytes;
/// for a symlink, the path, one 0x01 byte, its target's length as 8 bytes
/// little-endian, then the target, then, when it resolves to a regular file
/// (inside the tree or not), that file's bytes as a regular file's are
/// hashed, so a link cannot carry changed bytes past the digest.
/// Directories are walked, never followed through a link; any other kind
/// of entry is refused.
pub(crate) fn tree_sha256(root: &Path) -> Result<[u8; 32], String> {
    use std::io::Read;
    use std::os::unix::ffi::OsStrExt;
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
        for entry in fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))? {
            let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
            let kind = fs::symlink_metadata(&path)
                .map_err(|e| format!("{}: {e}", path.display()))?
                .file_type();
            if kind.is_dir() {
                walk(&path, out)?;
            } else if kind.is_file() || kind.is_symlink() {
                out.push(path);
            } else {
                return Err(format!(
                    "{} is not a file, link or directory",
                    path.display()
                ));
            }
        }
        Ok(())
    }
    let mut paths = Vec::new();
    walk(root, &mut paths)?;
    let mut rel: Vec<(Vec<u8>, PathBuf)> = paths
        .into_iter()
        .map(|p| {
            let r = p
                .strip_prefix(root)
                .unwrap_or(&p)
                .as_os_str()
                .as_bytes()
                .to_vec();
            (r, p)
        })
        .collect();
    rel.sort();
    let mut hash = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    // A regular file's bytes: 0x00, its length, then the bytes.
    let mut file = |hash: &mut Sha256, p: &Path, len: u64| -> Result<(), String> {
        let mut f = fs::File::open(p).map_err(|e| format!("{}: {e}", p.display()))?;
        hash.update([0u8]);
        hash.update(len.to_le_bytes());
        let mut read = 0u64;
        loop {
            let n = f
                .read(&mut buf)
                .map_err(|e| format!("{}: {e}", p.display()))?;
            if n == 0 {
                break;
            }
            hash.update(&buf[..n]);
            read += n as u64;
        }
        if read != len {
            return Err(format!("{} changed while it was read", p.display()));
        }
        Ok(())
    };
    for (r, p) in rel {
        let meta = fs::symlink_metadata(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        hash.update(&r);
        if meta.file_type().is_symlink() {
            let target = fs::read_link(&p).map_err(|e| format!("{}: {e}", p.display()))?;
            let t = target.as_os_str().as_bytes();
            hash.update([1u8]);
            hash.update((t.len() as u64).to_le_bytes());
            hash.update(t);
            // The bytes a link resolves to, inside the tree or not.
            if let Ok((true, len)) = fs::metadata(&p).map(|m| (m.is_file(), m.len())) {
                file(&mut hash, &p, len)?;
            }
            continue;
        }
        file(&mut hash, &p, meta.len())?;
    }
    Ok(hash.finalize().into())
}

/// Kind 15 for every solved I2C bus, sorted by bus (`AET-WO-3100` R1): its
/// controller, its SCL and SDA lines and its devices, from the solved
/// topology alone (`INV-07`, `RUL-16`). A topology whose lines, controller
/// and devices do not agree is an internal error, never a guess.
pub(crate) fn solved_buses(t: &SolvedTopology) -> Result<Vec<Record>, String> {
    let mut lines: BTreeMap<&str, Vec<(&aethelgard_ir::PinAssignment, BusLine)>> = BTreeMap::new();
    for (net, a) in &t.analog_lines {
        let assignment = t
            .assignments
            .get(net)
            .ok_or_else(|| format!("analog line `{net}` has no pin assignment"))?;
        lines.entry(a.bus.as_str()).or_default().push((
            assignment,
            BusLine {
                signal: assignment.signal.clone(),
                net: net.clone(),
                pin: assignment.pin.clone(),
                alternate_function: assignment.alternate_function,
                ceiling_ps: u64::from(a.ceiling_ns) * 1000,
            },
        ));
    }
    let mut out = Vec::new();
    for (bus, mut pairs) in lines {
        let peripheral = pairs[0].0.peripheral.clone();
        if pairs.iter().any(|(a, _)| a.peripheral != peripheral) {
            return Err(format!("bus `{bus}`'s lines are on two controllers"));
        }
        // SCL, then SDA, by the solver's signal names (`I2C1_SCL`).
        let order = |l: &BusLine| l.signal.strip_prefix(peripheral.as_str()) != Some("_SCL");
        pairs.sort_by_key(|(_, l)| order(l));
        let devices: Vec<(String, u8)> = t
            .i2c_devices
            .iter()
            .filter(|(_, d)| d.bus == bus)
            .map(|(r, d)| {
                if d.peripheral == peripheral {
                    Ok((r.clone(), d.address))
                } else {
                    Err(format!(
                        "device `{r}` is on bus `{bus}` but not on `{peripheral}`"
                    ))
                }
            })
            .collect::<Result<_, _>>()?;
        out.push(Record::I2cBusSolved {
            bus: bus.to_owned(),
            peripheral,
            lines: pairs.into_iter().map(|(_, l)| l).collect(),
            devices,
        });
    }
    if let Some((r, d)) = t
        .i2c_devices
        .iter()
        .find(|(_, d)| !t.analog_lines.values().any(|a| a.bus == d.bus))
    {
        return Err(format!(
            "device `{r}` is on bus `{}`, which has no solved line",
            d.bus
        ));
    }
    Ok(out)
}

/// Kind 23, one per SPI controller with a device, from the solved topology
/// alone, as the twin wires it (`aethelgard_twin::platform::spi_wiring`):
/// each device's multiplexer address and chip select (`AET-WO-4200`
/// adversarial A3).
pub(crate) fn solved_spi(t: &SolvedTopology) -> Result<Vec<Record>, aethelgard_twin::TwinError> {
    let mut by: BTreeMap<String, Vec<(String, u8, String)>> = BTreeMap::new();
    for s in aethelgard_twin::platform::spi_wiring(t)? {
        by.entry(s.peripheral)
            .or_default()
            .push((s.reference, s.address, s.select));
    }
    Ok(by
        .into_iter()
        .map(|(peripheral, mut devices)| {
            devices.sort_by_key(|d| d.1);
            Record::SpiBusSolved {
                peripheral,
                devices,
            }
        })
        .collect())
}

/// Kinds 33 and 35 (`AET-WO-1211c`): each pin device's GPIO signal and ADC
/// input as the twin drives it, before the boot; none for a board without.
fn pin_declared(pins: &aethelgard_twin::pins::Pins) -> Vec<Record> {
    let outputs = pins.outputs.iter().map(|o| Record::PinGpioDeclared {
        net: o.net.clone(),
        pin: o.pin.clone(),
        output: true,
        level: u8::from(o.high),
        active: None,
        held: false,
    });
    let inputs = pins.inputs.iter().map(|i| Record::PinGpioDeclared {
        net: i.net.clone(),
        pin: i.pin.clone(),
        output: false,
        level: u8::from(i.high),
        active: Some(u8::from(i.active_high)),
        held: i.held,
    });
    let analog = pins.analog.iter().map(|a| Record::PinAdcDeclared {
        net: a.net.clone(),
        pin: a.pin.clone(),
        peripheral: a.peripheral.clone(),
        channel: a.channel,
        bits: a.bits,
        microvolts: a.microvolts,
        held: a.held,
        code: a.code,
    });
    outputs.chain(inputs).chain(analog).collect()
}

/// Kinds 24 and 25, one of each per motor output, from the solved topology
/// alone (`AET-WO-4300a`): the output as solved, and the static check for
/// silicon the judge reports, by the same function (`motor::check_silicon`).
pub(crate) fn solved_motors(t: &SolvedTopology) -> Vec<Record> {
    let mut solved = Vec::new();
    let mut checks = Vec::new();
    for (label, m, a) in t.motor_outputs_in_order() {
        // `SolvedTopology::validate` gave every output a `TIMn_CHk`.
        let Some((timer, channel)) = aethelgard_ir::motor::timer_channel(&a.signal) else {
            continue;
        };
        solved.push(Record::MotorOutputSolved {
            label: label.to_owned(),
            timer: a.peripheral.clone(),
            channel: u8::try_from(channel).expect("1 to 4"),
            pin: a.pin.clone(),
            alternate_function: a.alternate_function,
            protocol: m.protocol.name().to_owned(),
            psc: m.psc,
            arr: m.arr,
            ccr: m.ccr,
            pwm: m
                .pwm
                .as_ref()
                .map(|p| (p.rate_hz, p.pulse_min_us, p.pulse_max_us)),
            idle: m.pwm.as_ref().map(|p| match p.idle {
                aethelgard_ir::motor::Idle::Level(aethelgard_ir::motor::Level::Low) => {
                    aethelgard_chaos::record::MotorIdle::Low
                }
                aethelgard_ir::motor::Idle::Level(aethelgard_ir::motor::Level::High) => {
                    aethelgard_chaos::record::MotorIdle::High
                }
                aethelgard_ir::motor::Idle::PulseUs(us) => {
                    aethelgard_chaos::record::MotorIdle::PulseUs(us)
                }
            }),
            resolution_ps: m.pwm.as_ref().map(|p| p.resolution_ps),
        });
        let s = aethelgard_ir::motor::check_silicon(&t.clock, timer, m);
        checks.push(Record::MotorTimingCheck {
            label: label.to_owned(),
            kernel_hz: s.kernel_hz,
            period_ps: s.period_ps,
            high_ps: s.high_ps,
            nominal_period_ps: s.nominal_period_ps,
            nominal_high_ps: s.nominal_high_ps,
            holds: s.holds,
        });
    }
    solved.extend(checks);
    solved
}

/// Kind 27, one per serial port in the specification's order, from the
/// solved topology alone (`AET-WO-4300b`).
pub(crate) fn solved_serial(t: &SolvedTopology) -> Vec<Record> {
    t.serial_ports_in_order()
        .into_iter()
        .map(|(label, p, tx, rx)| Record::SerialPortSolved {
            label: label.to_owned(),
            index: p.index,
            peripheral: p.peripheral.clone(),
            tx: tx.pin.clone(),
            tx_af: tx.alternate_function,
            rx: rx.pin.clone(),
            rx_af: rx.alternate_function,
            role: p.role.name().to_owned(),
            framing: aethelgard_chaos::record::SerialFraming {
                baud: p.framing.baud,
                data_bits: p.framing.data_bits,
                parity: p.framing.parity.name().to_owned(),
                stop_bits: p.framing.stop_bits,
                inverted: p.framing.inverted,
            },
            kernel_hz: p.kernel_hz,
            brr: p.brr,
            actual_millibaud: p.actual_millibaud,
            error_ppm: p.error_ppm,
        })
        .collect()
}

/// Kinds 28 and 29 from the boot's serial reports (`AET-WO-4300b`): every
/// port's static framing judgement, its silicon half from the topology and
/// its tags once Renode has run; then every port's witness, once Renode has
/// run. A longer list of unexpected writes than kind 28 holds is cut: it
/// fails the port's tags all the same.
fn serial_records(serial: &[crate::verdict::SerialReport]) -> Vec<Record> {
    use aethelgard_chaos::record::{MAX_SERIAL_UNEXPECTED, SerialCapture, SerialTags};
    let mut out: Vec<Record> = serial
        .iter()
        .map(|s| {
            let (f, w) = (&s.silicon, &s.twin);
            Record::SerialFramingCheck {
                label: s.label.clone(),
                kernel_hz: f.kernel_hz,
                brr: f.brr,
                actual_millibaud: f.actual_millibaud,
                error_ppm: f.error_ppm,
                tolerance: f
                    .tolerance
                    .map(|r| (r.table.to_owned(), r.m_bits, r.tolerance_ppm)),
                holds: f.holds,
                tags_needed: w.tags_needed.clone(),
                tags: match (w.usart_writes, &w.tags_missing, &w.tags_unexpected) {
                    (Some(n), Some(missing), Some(unexpected)) => Some(SerialTags {
                        writes: n as u64,
                        missing: missing.clone(),
                        unexpected: unexpected
                            .iter()
                            .take(MAX_SERIAL_UNEXPECTED)
                            .cloned()
                            .collect(),
                    }),
                    _ => None,
                },
            }
        })
        .collect();
    for s in serial {
        let w = &s.twin;
        let Some(echo) = w.echo else {
            continue;
        };
        let capture = match (w.patterns, w.cut, w.echoed, w.stray, w.capture_bytes) {
            (Some(patterns), Some(cut), Some(echoed), Some(stray), Some(len)) => {
                Some(SerialCapture {
                    patterns: patterns as u64,
                    cut: cut as u64,
                    echoed: echoed as u64,
                    stray,
                    len: len as u64,
                })
            }
            _ => None,
        };
        out.push(Record::SerialWitness {
            label: s.label.clone(),
            capture,
            frame: w.frame_bytes,
            echo: echo.to_owned(),
            rx_at_injection: w.rx_fields,
            tx_at_end: w.tx_fields,
            tx_at_send: w.tx_send_fields,
            injected_ns: w.injected_ns.filter(|_| w.rx_fields.is_some()),
        });
    }
    out
}

/// Kinds 16 to 19 from a droop's report, the crossings, resets and
/// observations merged in the time order replay checks (`AET-WO-2301`).
fn droop_records(
    d: &crate::verdict::DroopReport,
    droop: Option<&aethelgard_chaos::record::Droop>,
) -> Result<Vec<Record>, String> {
    let r = &d.rail;
    // The scenario's droop, as the report rendered it, at the instant and the
    // level the report placed it at (`AET-WO-2310`).
    let droop = droop
        .filter(|o| {
            o.domain == r.domain
                && o.floor_millivolts == r.floor_millivolts
                && o.fall_ns == r.fall_ns
                && o.hold_ns == r.hold_ns
                && o.rise_ns == r.rise_ns
                && o.source_milliohms == r.source_milliohms
                && o.load_milliamps == r.load_milliamps
        })
        .ok_or("the verdict reports a rail that is not the scenario's droop")?;
    let mut out = vec![Record::RailDeclared {
        droop: aethelgard_chaos::record::RailDroop {
            droop: droop.clone(),
            start_ns: r.start_ns,
            bor_level: r.bor_level,
        },
        nominal_millivolts: r.nominal_millivolts,
        tolerance_permille: r.tolerance_permille,
        decoupling_picofarads: r.decoupling_picofarads,
    }];
    // Kind 32 beside the rail, decided before the droop's boot
    // (`AET-WO-2305` §1.2).
    out.extend(d.recovery_bound.as_ref().map(|b| Record::RecoveryBound {
        boot_word_ns: b.undisturbed_boot_word_ns,
        margin_ns: b.margin_ns,
        bound_ns: b.bound_ns,
    }));
    let mut timed: Vec<((u64, u8), Record)> = Vec::new();
    for c in &d.crossings {
        timed.push((
            (c.at_ps, 0),
            Record::RailCrossing {
                domain: r.domain.clone(),
                level: c.level.clone(),
                microvolts: c.microvolts,
                at_ps: c.at_ps,
                falling: c.falling,
            },
        ));
    }
    for x in &d.resets {
        timed.push((
            (x.at_ns.saturating_mul(1000), 1),
            Record::ResetRendered {
                at_ns: x.at_ns,
                released: x.released,
                cause: x.cause.clone(),
                rendering: x.rendering.to_owned(),
            },
        ));
    }
    for o in &d.observed {
        timed.push((
            (o.at_ns.saturating_mul(1000), 2),
            Record::FirmwareObserved {
                at_ns: o.at_ns,
                boot_word: word(&o.boot_word)?,
                status: word(&o.status)?,
                value: word(&o.value)?,
                witness: o.witness.as_deref().map(archived_lines),
            },
        ));
    }
    timed.sort_by_key(|(k, _)| *k);
    out.extend(timed.into_iter().map(|(_, r)| r));
    Ok(out)
}

/// Witness lines as a record archives them: the first
/// [`aethelgard_chaos::record::MAX_WITNESS_LINES`], each as printable ASCII of
/// at most 256 bytes. A line that is not is escaped and cut; the judge
/// compared the lines whole, and a changed line still differs from what it
/// expects, so this cannot make an observation match.
fn archived_lines(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .take(aethelgard_chaos::record::MAX_WITNESS_LINES)
        .map(|l| {
            if aethelgard_chaos::event::text_ok(l, 1, 256) {
                l.clone()
            } else {
                let mut e: String = l.escape_default().collect();
                e.truncate(256);
                if e.is_empty() { "?".to_owned() } else { e }
            }
        })
        .collect()
}

/// A kind 13 or kind 22 with its tail cut until the event, at the largest
/// timestamp, encodes within the reader's 4,096 bytes (`AET-WO-4200` A4;
/// `AET-WO-4210` F18: a script's step and a 64-byte reference size it, not
/// one identity read). A kind 22's multiplexer lines are cut first, then the
/// observation's last lines, so the first lines kept are the step's own. The
/// judge compared them whole first, so a cut observation still differs; the
/// expected lines, the twin's, are never cut, and an event they alone
/// overfill is the archive's refusal.
fn fit(mut record: Record) -> Record {
    let size = |r: &Record| {
        aethelgard_chaos::cbor::to_bytes(
            &aethelgard_chaos::event::Event::Record {
                timestamp: u64::MAX,
                record: r.clone(),
            }
            .to_value(),
        )
        .len()
    };
    while size(&record) > aethelgard_chaos::archive::MAX_EVENT_BYTES {
        let cut = match &mut record {
            Record::SpiWitness {
                unrouted, observed, ..
            } => unrouted.pop().is_some() || observed.as_mut().is_some_and(|o| o.pop().is_some()),
            Record::I2cWitness { observed, .. } => {
                observed.as_mut().is_some_and(|o| o.pop().is_some())
            }
            _ => false,
        };
        if !cut {
            break;
        }
    }
    record
}

/// The bytes the verdict writes as `0xef 0xc8`, each 1 to 4 of them.
fn write_bytes(text: &str) -> Result<Vec<u8>, String> {
    text.split(' ')
        .map(|b| u8::try_from(hex_u64(b)?).map_err(|e| e.to_string()))
        .collect()
}

fn hex_u64(s: &str) -> Result<u64, String> {
    s.strip_prefix("0x")
        .and_then(|h| u64::from_str_radix(h, 16).ok())
        .ok_or_else(|| format!("`{s}` is not a 0x hex value"))
}

fn hex_u32(s: &str) -> Result<u32, String> {
    u32::try_from(hex_u64(s)?).map_err(|_| format!("`{s}` is over 32 bits"))
}

pub(crate) fn sha(s: &str) -> Result<[u8; 32], String> {
    let bytes: Result<Vec<u8>, _> = (0..s.len())
        .step_by(2)
        .map(|i| s.get(i..i + 2).and_then(|b| u8::from_str_radix(b, 16).ok()))
        .map(|b| b.ok_or(()))
        .collect();
    bytes
        .ok()
        .and_then(|b| <[u8; 32]>::try_from(b).ok())
        .ok_or_else(|| format!("`{s}` is not a sha256"))
}

fn reading(r: &RegisterReport) -> Result<Reading, String> {
    Ok(Reading {
        address: hex_u32(&r.address)?,
        mask: hex_u32(&r.mask)?,
        expected: hex_u32(&r.expected)?,
        observed: r.observed.as_deref().map(hex_u32).transpose()?,
    })
}

fn word(w: &Word) -> Result<Reading, String> {
    Ok(Reading {
        address: hex_u32(&w.address)?,
        mask: u32::MAX,
        expected: hex_u32(&w.expected)?,
        observed: w.observed.as_deref().map(hex_u32).transpose()?,
    })
}

/// Picoseconds from the bridge's nanoseconds. A computed rise rounds up, so
/// a rise past its whole-nanosecond ceiling stays past it in picoseconds; a
/// lower bound rounds down, so it stays a lower bound.
fn picoseconds(ns: f64, up: bool) -> Result<u64, String> {
    let ps = ns * 1000.0;
    let ps = if up { ps.ceil() } else { ps.floor() };
    if ps.is_finite() && (0.0..=u64::MAX as f64).contains(&ps) {
        // In range and whole, so the cast is exact.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Ok(ps as u64)
    } else {
        Err(format!("{ns} ns is not a time in picoseconds"))
    }
}

/// Every record of a `simulate --firmware` verdict, as integers (`RUL-06`).
fn records_of(
    v: &Verdict,
    droop: Option<&aethelgard_chaos::record::Droop>,
) -> Result<Vec<Record>, String> {
    let mut out = Vec::new();
    for a in v.analog.as_deref().unwrap_or_default() {
        let ceiling_ps = u64::from(a.ceiling_ns) * 1000;
        let window = Window {
            deck_hz: a.deck_hz,
            declared_hz: a.declared_hz,
        };
        let record = match (
            a.rise_30_70_ns,
            a.rise_30_70_ns_lower_bound,
            a.volts_at_release,
        ) {
            (Some(rise), None, None) => Record::RiseComputed {
                bus: a.bus.clone(),
                net: a.net.clone(),
                rise_30_70_ps: picoseconds(rise, true)?,
                complete: a.complete,
                ceiling_ps,
                exceeds: a.exceeds,
                window,
            },
            (None, Some(bound), None) => Record::RiseComputed {
                bus: a.bus.clone(),
                net: a.net.clone(),
                rise_30_70_ps: picoseconds(bound, false)?,
                complete: a.complete,
                ceiling_ps,
                exceeds: a.exceeds,
                window,
            },
            (None, None, Some(volts)) => {
                let uv = (volts * 1e6).round();
                if !(uv.is_finite() && (i64::MIN as f64..=i64::MAX as f64).contains(&uv)) {
                    return Err(format!("{volts} V is not a voltage in microvolts"));
                }
                Record::LineStuckHigh {
                    bus: a.bus.clone(),
                    net: a.net.clone(),
                    // In range and whole, so the cast is exact.
                    #[allow(clippy::cast_possible_truncation)]
                    microvolts_at_release: uv as i64,
                    ceiling_ps,
                    window,
                }
            }
            _ => return Err(format!("analog record on `{}` has no single value", a.net)),
        };
        out.push(record);
    }
    for f in v.faults.as_deref().unwrap_or_default() {
        let mut nets = f.nets.clone();
        nets.sort();
        out.push(Record::BusFault {
            bus: f.bus.clone(),
            fault: f.fault.to_owned(),
            source: f.source.clone(),
            from: f.from.iter().map(|s| (*s).to_owned()).collect(),
            nets,
            firmware_sees: f.firmware_sees.to_owned(),
        });
    }
    if let Some(d) = &v.droop {
        out.extend(droop_records(d, droop)?);
    }
    let boot = v
        .boot
        .as_ref()
        .ok_or("a firmware verdict has no boot record")?;
    out.push(Record::BootObserved {
        image_sha256: sha(&boot.firmware.image_sha256)?,
        boot_word: word(&boot.boot_word)?,
        instructions_expected: boot.instructions.expected,
        instructions_observed: boot.instructions.observed,
        pc: boot.pc.as_deref().map(hex_u32).transpose()?,
        executable: boot
            .executable
            .iter()
            .map(|&(s, e)| match (u32::try_from(s), u32::try_from(e)) {
                (Ok(s), Ok(e)) => Ok((s, e)),
                _ => Err(format!("segment [{s:#x}, {e:#x}) is not in 32 bits")),
            })
            .collect::<Result<_, String>>()?,
        brown_out: Reading {
            address: hex_u32(&boot.brown_out.address)?,
            mask: hex_u32(&boot.brown_out.mask)?,
            expected: hex_u32(&boot.brown_out.expected)?,
            observed: boot
                .brown_out
                .observed
                .as_deref()
                .map(hex_u32)
                .transpose()?,
        },
        // At most as many as a record carries: a cut list still fails.
        flash_access: boot.brown_out.flash_access.as_ref().map(|v| {
            v.iter()
                .take(aethelgard_chaos::record::MAX_WITNESS_LINES)
                .cloned()
                .collect()
        }),
    });
    for r in &boot.registers {
        out.push(Record::RegisterObserved {
            name: r.name.clone(),
            reading: reading(r)?,
        });
    }
    for c in &boot.i2c {
        out.push(Record::I2cCheck {
            reference: c.reference.clone(),
            address: u8::try_from(hex_u64(&c.address)?).map_err(|e| e.to_string())?,
            step: c.step.to_owned(),
            write: write_bytes(&c.register)?,
            width: c.width,
            expected: hex_u64(&c.expected)?,
            mask: hex_u64(&c.mask)?,
            observed: c.observed.as_deref().map(hex_u64).transpose()?,
            status: c.status.map(str::to_owned),
        });
    }
    for w in &boot.i2c_witness {
        out.push(fit(Record::I2cWitness {
            peripheral: w.peripheral.clone(),
            reference: w.reference.clone(),
            device: w.device.clone(),
            step: w.step.to_owned(),
            expected: w.expected.clone(),
            observed: w.observed.as_deref().map(archived_lines),
            absent: w
                .absent
                .iter()
                .take(aethelgard_chaos::record::MAX_WITNESS_LINES)
                .map(|a| u8::try_from(hex_u64(a)?).map_err(|e| e.to_string()))
                .collect::<Result<_, _>>()?,
        }));
    }
    for c in &boot.spi {
        out.push(Record::SpiCheck {
            reference: c.reference.clone(),
            peripheral: c.peripheral.clone(),
            select: c.select.clone(),
            address: u8::try_from(hex_u64(&c.address)?).map_err(|e| e.to_string())?,
            step: c.step.to_owned(),
            write: write_bytes(&c.command)?,
            width: c.width,
            expected: hex_u64(&c.expected)?,
            mask: hex_u64(&c.mask)?,
            observed: c.observed.as_deref().map(hex_u64).transpose()?,
            status: c.status.map(str::to_owned),
        });
    }
    for w in &boot.spi_witness {
        out.push(fit(Record::SpiWitness {
            peripheral: w.peripheral.clone(),
            reference: w.reference.clone(),
            device: w.device.clone(),
            step: w.step.to_owned(),
            expected: w.expected.clone(),
            // Kind 22 holds a witness and a multiplexer's lines, both of a
            // length the firmware drives: [`fit`] cuts them so the event stays
            // inside the reader's 4,096 bytes (adversarial A4).
            observed: w.observed.as_deref().map(archived_lines),
            unrouted: archived_lines(&w.unrouted),
        }));
    }
    // Kind 26, each motor output's twin witness, once Renode has run
    // (`AET-WO-4300a`).
    for m in &boot.motor {
        let w = &m.twin;
        let (
            Some(status),
            Some(rises),
            Some(falls),
            Some(off_grid),
            Some(batched),
            Some(end_ns),
            Some(edges_before),
            Some(edges_after),
            Some(cen_rises),
        ) = (
            w.status,
            w.rises,
            w.falls,
            w.off_grid,
            w.batched,
            w.end_ns,
            w.edges_before,
            w.edges_after,
            w.cen_rises,
        )
        else {
            continue;
        };
        out.push(Record::MotorWitness {
            label: m.label.clone(),
            observer: w.observer.clone(),
            period_ns: w.period_ns,
            high_ns: w.high_ns,
            grain_ns: w.grain_ns,
            quantum_ns: w.quantum_ns,
            status: status.to_owned(),
            rises,
            falls,
            first_ns: w.first_ns,
            last_ns: w.last_ns,
            end_ns,
            span_ns: w.span_ns,
            off_grid,
            batched,
            anchor_ns: w.anchor_ns,
            edges_before,
            edges_after,
            cen_rises,
        });
    }
    // Kinds 37, 34 and 36 (`AET-WO-1211c`): each output at a static level,
    // each pin device's GPIO signal and ADC input, once Renode has run.
    for m in &boot.motor {
        let Some(l) = &m.level else { continue };
        let (Some(status), Some(rises_after), Some(falls_after)) =
            (l.status, l.rises_after, l.falls_after)
        else {
            continue;
        };
        out.push(Record::MotorLevelWitness {
            label: m.label.clone(),
            level: u8::from(l.idle == "high"),
            settle_ns: l.settle_ns,
            rises_after,
            falls_after,
            end_level: l.end_level.map(|e| u8::from(e == "high")),
            status: status.to_owned(),
        });
    }
    for g in &boot.gpio {
        let Some(status) = g.status else { continue };
        out.push(Record::PinGpioWitness {
            net: g.net.clone(),
            status: status.to_owned(),
            bsrr: g.bsrr.as_deref().map(hex_u32).transpose()?,
            ordered: g.ordered,
            level: g.read.map(|r| u8::from(r == "high")),
        });
    }
    for a in &boot.adc {
        let (Some(status), Some(reads)) = (a.status, a.reads) else {
            continue;
        };
        out.push(Record::PinAdcWitness {
            net: a.net.clone(),
            status: status.to_owned(),
            reads,
            code: a.observed,
        });
    }
    // Kind 30, each judge's stop (`AET-WO-2302` D2).
    for stop in &boot.stops {
        out.push(Record::JudgeStop {
            timer: stop.timer.clone(),
            anchor_ns: stop.anchor_ns,
            at_ns: stop.at_ns,
            read_back: stop
                .read_back
                .iter()
                .map(reading)
                .collect::<Result<_, _>>()?,
            cr1_after: stop.cr1_after.as_deref().map(hex_u32).transpose()?,
        });
    }
    out.extend(serial_records(&boot.serial));
    let mut findings: BTreeMap<(String, String), u64> = BTreeMap::new();
    for f in &v.findings {
        *findings
            .entry((f.kind.clone(), f.severity.clone()))
            .or_default() += 1;
    }
    out.push(Record::Verdict {
        pass: v.ok,
        findings: findings.into_iter().map(|((k, s), n)| (k, s, n)).collect(),
    });
    Ok(out)
}

#[derive(Serialize)]
struct Replayed {
    ok: bool,
    check: &'static str,
    run_id: String,
    scenario: String,
    verdict: &'static str,
    expect: &'static str,
    matched: bool,
    findings: Vec<String>,
    expect_findings: Vec<String>,
    artifacts: BTreeMap<String, String>,
    /// The tools that judged the run, as the archive names them; never
    /// required equal to this host's (`AET-WO-2205` §1.1).
    tools: BTreeMap<String, String>,
    /// The content hash the archive holds beside a tool's name: ngspice's
    /// library and the host toolchain's sysroot (`AET-WO-2310`).
    tool_sha256: BTreeMap<String, String>,
    event_count: u64,
    trace_bytes: u64,
    state_sequence_hash: String,
}

/// `replay`'s checks, in its order: the strict reader, the run directory's
/// name, then the campaign's run rules. `attribute` applies exactly these
/// (`AET-WO-3100` §1.3); a refusal is printed and its code returned.
pub(crate) fn checked(
    dir: &Path,
) -> Result<(aethelgard_chaos::Replayed, rules::RunSummary), ExitCode> {
    let replayed = match aethelgard_chaos::replay_dir(dir) {
        Ok(r) => r,
        Err(e) => return Err(refuse(e.kind(), None, e.to_string(), EXIT_UNREADABLE)),
    };
    // A run directory is named by its run id, so a run copied under another
    // run's name is not that run (`AET-WO-2300` adversarial F3).
    // `replay_dir` has refused a symlinked run directory, so its canonical
    // name is its own (and `--archive .` from inside it works).
    let named = fs::canonicalize(dir)
        .ok()
        .and_then(|d| d.file_name().map(std::ffi::OsStr::to_owned));
    if named.as_deref() != Some(std::ffi::OsStr::new(&hex(replayed.header.run_id))) {
        return Err(refuse(
            "RunIdMismatch",
            Some(&hex(replayed.header.run_id)),
            format!(
                "{} holds run {}: a run directory is named by its run id",
                dir.display(),
                hex(replayed.header.run_id)
            ),
            EXIT_UNREADABLE,
        ));
    }
    match rules::check_archived(&replayed.header, &replayed.state) {
        Ok(summary) => Ok((replayed, summary)),
        Err(reason) => Err(refuse(
            "NotACampaignRun",
            None,
            format!("{}: {reason}", dir.display()),
            EXIT_UNREADABLE,
        )),
    }
}

/// `aethelgard replay --archive <run dir>`: the strict reader, then the
/// campaign's run rules, from the run directory alone.
pub fn replay(dir: &Path) -> ExitCode {
    let (replayed, summary) = match checked(dir) {
        Ok(run) => run,
        Err(code) => return code,
    };
    emit(
        &Replayed {
            ok: true,
            check: "replay",
            run_id: hex(replayed.header.run_id),
            scenario: summary.name,
            verdict: pass_fail(summary.pass),
            expect: pass_fail(summary.expect_pass),
            matched: summary.matched,
            findings: summary.failing_kinds,
            expect_findings: summary.expect_findings,
            artifacts: replayed
                .header
                .artifacts
                .iter()
                .map(|(n, h)| (n.clone(), hex(h)))
                .collect(),
            tools: replayed.header.tools.iter().cloned().collect(),
            tool_sha256: replayed
                .header
                .tool_sha256
                .iter()
                .map(|(n, h)| (n.clone(), hex(h)))
                .collect(),
            event_count: replayed.manifest.event_count,
            trace_bytes: replayed.manifest.trace_bytes,
            state_sequence_hash: hex(replayed.manifest.state_hash),
        },
        0,
    )
}

#[derive(Serialize)]
struct EventLine {
    index: u64,
    kind: u64,
}

#[derive(Serialize)]
struct Attributed {
    ok: bool,
    check: &'static str,
    run_id: String,
    state_sequence_hash: String,
    domain: &'static str,
    rule: &'static str,
    evidence: Vec<EventLine>,
    symptoms: Vec<EventLine>,
    masked: Vec<EventLine>,
    unexplained: Vec<String>,
    /// What each evidence and symptom event is about, sorted and unique
    /// ([`subject`]): `verify` holds a detected fault to its own subject
    /// (`AET-WO-3303` D1, ruled (a)).
    subjects: Vec<String>,
}

/// What one record is about: `bus:<id>`, `device:<reference>`, `rail:<id>`,
/// `register:<name>`, `output:<label>`, `port:<label>` (a serial port),
/// `controller:<peripheral>`, `boot`, or `kind:<n>` for a record that names
/// none.
fn subject(r: &Record) -> String {
    match r {
        Record::RiseComputed { bus, .. }
        | Record::LineStuckHigh { bus, .. }
        | Record::BusFault { bus, .. }
        | Record::I2cBusSolved { bus, .. } => format!("bus:{bus}"),
        Record::I2cCheck { reference, .. }
        | Record::I2cWitness { reference, .. }
        | Record::SpiCheck { reference, .. }
        | Record::SpiWitness { reference, .. } => format!("device:{reference}"),
        Record::RailDeclared { droop, .. } => format!("rail:{}", droop.droop.domain),
        Record::RailCrossing { domain, .. } => format!("rail:{domain}"),
        Record::RegisterObserved { name, .. } => format!("register:{name}"),
        Record::MotorOutputSolved { label, .. }
        | Record::MotorTimingCheck { label, .. }
        | Record::MotorWitness { label, .. } => format!("output:{label}"),
        Record::JudgeStop { timer, .. } => format!("timer:{timer}"),
        Record::SerialPortSolved { label, .. }
        | Record::SerialFramingCheck { label, .. }
        | Record::SerialWitness { label, .. } => format!("port:{label}"),
        Record::SpiBusSolved { peripheral, .. } => format!("controller:{peripheral}"),
        Record::BootObserved { .. }
        | Record::ResetRendered { .. }
        | Record::FirmwareObserved { .. }
        | Record::RecoveryBound { .. } => "boot".to_owned(),
        other => format!("kind:{}", other.kind()),
    }
}

/// `aethelgard attribute --archive <run dir>`: `replay`'s checks, then
/// `aethelgard_evolve::attribute`, from the run directory alone. Exit 0 for
/// `electrical`, `software` or `none`, 1 for `indeterminate`.
pub fn attribute(dir: &Path) -> ExitCode {
    use aethelgard_evolve::{AttributeError, Domain};
    let (replayed, _) = match checked(dir) {
        Ok(run) => run,
        Err(code) => return code,
    };
    let a = match aethelgard_evolve::attribute(&replayed) {
        Ok(a) => a,
        // `checked` has applied the same rules, so this is not reached.
        Err(AttributeError::NotACampaignRun(reason)) => {
            return refuse(
                "NotACampaignRun",
                None,
                format!("{}: {reason}", dir.display()),
                EXIT_UNREADABLE,
            );
        }
        Err(AttributeError::RegisterFacts(e)) => {
            return refuse(
                e.kind(),
                None,
                format!("{}: a register fact the run names: {e}", dir.display()),
                EXIT_UNREADABLE,
            );
        }
        Err(AttributeError::TooManyRecords { records, limit }) => {
            return refuse(
                "TooManyRecords",
                None,
                format!(
                    "{}: the run holds {records} records; attribution is bounded at {limit}",
                    dir.display()
                ),
                EXIT_UNREADABLE,
            );
        }
    };
    let lines = |v: Vec<aethelgard_evolve::EventRef>| {
        v.into_iter()
            .map(|e| EventLine {
                index: e.index,
                kind: e.kind,
            })
            .collect()
    };
    let records = &replayed.state.records;
    let subjects: BTreeSet<String> = a
        .evidence
        .iter()
        .chain(&a.symptoms)
        .filter_map(|e| usize::try_from(e.index).ok().and_then(|i| records.get(i)))
        .map(subject)
        .collect();
    let determinate = a.domain != Domain::Indeterminate;
    emit(
        &Attributed {
            ok: determinate,
            check: "attribute",
            run_id: hex(a.run_id),
            state_sequence_hash: hex(a.state_hash),
            domain: a.domain.name(),
            rule: a.rule.id(),
            evidence: lines(a.evidence),
            symptoms: lines(a.symptoms),
            masked: lines(a.masked),
            unexplained: a.unexplained,
            subjects: subjects.into_iter().collect(),
        },
        if determinate { 0 } else { EXIT_FAIL },
    )
}

#[cfg(test)]
mod tests {
    use super::{
        HeldLow, NO_WITNESSED_DEVICE, Record, Scenario, Specification, apply, archived_lines,
        observation_witness, picoseconds, solved_buses, subject,
    };

    /// `AET-WO-2306` §1.4: a droop's observations count the releases judged
    /// against a device's witness; with no I2C or SPI witness record (no
    /// identity read on the boot) the count is 0 and says so; with no
    /// release there is no observation and no field.
    /// `AET-WO-2310` adversarial: the toolchain digest changes whenever a byte
    /// the toolchain runs does, a link's target's included, and is the same
    /// for the same tree.
    #[test]
    fn the_tree_digest_moves_with_every_byte_name_and_link_target() {
        use std::fs;
        use std::os::unix::fs::symlink;
        let base = std::env::temp_dir().join(format!("aet-2310-tree-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let (tree, outside) = (base.join("tree"), base.join("outside"));
        fs::create_dir_all(tree.join("lib")).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(tree.join("bin-rustc"), b"stub").unwrap();
        fs::write(tree.join("lib/librustc_driver.so"), b"the compiler").unwrap();
        fs::write(outside.join("libLLVM.so"), b"llvm").unwrap();
        symlink(outside.join("libLLVM.so"), tree.join("lib/libLLVM.so")).unwrap();
        let d = || super::tree_sha256(&tree).unwrap();
        let first = d();
        assert_eq!(d(), first, "the same tree, the same digest");
        let mut seen = vec![first];
        let mut moved = |what: &str, edit: &dyn Fn()| {
            edit();
            let now = d();
            assert!(!seen.contains(&now), "{what} did not move the digest");
            seen.push(now);
        };
        moved("a byte of the compiler", &|| {
            fs::write(tree.join("lib/librustc_driver.so"), b"the compilex").unwrap()
        });
        moved("the bytes behind a link", &|| {
            fs::write(outside.join("libLLVM.so"), b"LLVM").unwrap()
        });
        moved("a name", &|| {
            fs::rename(tree.join("bin-rustc"), tree.join("bin-rustd")).unwrap()
        });
        moved("an empty file", &|| {
            fs::write(tree.join("lib/empty"), b"").unwrap()
        });
        moved("a link's target", &|| {
            fs::remove_file(tree.join("lib/libLLVM.so")).unwrap();
            symlink(tree.join("lib/empty"), tree.join("lib/libLLVM.so")).unwrap();
        });
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_droop_observation_with_no_device_witness_says_so() {
        let reset = |released| Record::ResetRendered {
            at_ns: 1,
            released,
            cause: vec!["BOR".into()],
            rendering: "r".into(),
        };
        let i2c = Record::I2cWitness {
            peripheral: "I2C1".into(),
            reference: "U2".into(),
            device: "i2c1.u2".into(),
            step: "identity".into(),
            expected: vec!["line".into()],
            observed: None,
            absent: vec![],
        };
        let spi = Record::SpiWitness {
            peripheral: "SPI1".into(),
            reference: "U4".into(),
            device: "spi1.spi1mux.u4".into(),
            step: "identity".into(),
            expected: vec![],
            observed: None,
            unrouted: vec![],
        };
        let released = [reset(false), reset(true), reset(false), reset(true)];
        assert_eq!(observation_witness(&[reset(false)]), None);
        assert_eq!(observation_witness(std::slice::from_ref(&i2c)), None);
        for device in [i2c, spi] {
            let mut records = released.to_vec();
            records.push(device);
            let s = observation_witness(&records).unwrap();
            assert_eq!((s.judged, s.nothing_in_scope), (2, None), "{s:?}");
            assert_eq!(s.nothing_in_scope_reason, None);
        }
        let s = observation_witness(&released).unwrap();
        assert_eq!((s.judged, s.nothing_in_scope), (0, Some(true)), "{s:?}");
        assert_eq!(
            s.nothing_in_scope_reason.as_deref(),
            Some(NO_WITNESSED_DEVICE)
        );
    }

    /// A serial port's records are about the port, by label, as a motor
    /// output's are about the output (`AET-WO-4300b` merged with `3303` D1).
    #[test]
    fn a_serial_port_s_records_name_the_port() {
        let witness = Record::SerialWitness {
            label: "CONSOLE".into(),
            capture: None,
            frame: [0; aethelgard_chaos::record::SERIAL_FRAME_LEN],
            echo: "missing".into(),
            rx_at_injection: None,
            tx_at_end: None,
            tx_at_send: None,
            injected_ns: None,
        };
        assert_eq!(subject(&witness), "port:CONSOLE");
    }

    /// `AET-WO-2204`: an observation is archived as at most 16 lines of
    /// printable ASCII, each at most 256 bytes; a changed line still differs
    /// from every line a model logs, so the archive cannot make it match.
    #[test]
    fn kinds_13_and_22_stay_inside_the_event_bound_however_long_their_lines() {
        // `AET-WO-4200` adversarial A4: a firmware that reads with eight
        // frames and then clocks 16 bytes with no chip select made a kind 22
        // of 16 witness and 16 multiplexer lines, over the reader's 4,096
        // bytes, and the campaign stopped with an archive error. `AET-WO-4210`
        // F18: a script's step with a 64-byte reference, kind 13 too.
        let lines =
            |n: usize, len: usize| -> Vec<String> { (0..n).map(|_| "x".repeat(len)).collect() };
        let long = "u".repeat(64);
        let device = format!("spi1.spi1mux.{long}");
        // The largest step the twin expects: 14 lines, each naming the device.
        let expected = lines(14, 70 + device.len());
        let size = |r: &Record| {
            aethelgard_chaos::cbor::to_bytes(
                &aethelgard_chaos::event::Event::Record {
                    timestamp: u64::MAX,
                    record: r.clone(),
                }
                .to_value(),
            )
            .len()
        };
        let w = super::fit(Record::SpiWitness {
            peripheral: "SPI1".into(),
            reference: long.to_uppercase(),
            device: device.clone(),
            step: "data".into(),
            expected: expected.clone(),
            observed: Some(archived_lines(&lines(16, 256))),
            unrouted: archived_lines(&lines(16, 256)),
        });
        assert!(size(&w) <= 4096, "{}", size(&w));
        let Record::SpiWitness {
            observed, unrouted, ..
        } = &w
        else {
            unreachable!()
        };
        // The multiplexer's lines go first; the observation keeps its first
        // lines, the step's own.
        assert!(unrouted.is_empty() && !observed.as_ref().unwrap().is_empty());
        let w = super::fit(Record::I2cWitness {
            peripheral: "I2C1".into(),
            reference: long.to_uppercase(),
            device: format!("i2c1.{long}"),
            step: "data".into(),
            expected,
            observed: Some(archived_lines(&lines(16, 256))),
            absent: vec![0x7F; 16],
        });
        assert!(size(&w) <= 4096, "{}", size(&w));
        // What fits is untouched.
        let small = Record::I2cWitness {
            peripheral: "I2C1".into(),
            reference: "U2".into(),
            device: "i2c1.u2".into(),
            step: "identity".into(),
            expected: lines(2, 30),
            observed: Some(lines(2, 30)),
            absent: Vec::new(),
        };
        assert_eq!(super::fit(small.clone()), small);
    }

    /// Every step of every modelled part's script, with a 64-byte reference,
    /// expects lines that leave room in its event (`AET-WO-4210` F18).
    #[test]
    fn every_table_steps_expected_lines_fit_an_event_with_room() {
        let long = "u".repeat(64);
        for p in aethelgard_ir::parts::table().unwrap().parts() {
            let Some(m) = p
                .renode
                .as_deref()
                .and_then(aethelgard_twin::model::Model::from_renode)
            else {
                continue;
            };
            let device = match m.bus() {
                aethelgard_ir::parts::BusKind::I2c => format!("i2c4.{long}"),
                aethelgard_ir::parts::BusKind::Spi => format!("spi6.spi6mux.{long}"),
            };
            for step in p.steps() {
                let w = m.witness(&device, &step).unwrap_or_default();
                let bytes: usize = w.iter().map(String::len).sum();
                assert!(
                    w.len() <= 16 && bytes <= 2_000,
                    "{} {:?}: {bytes}",
                    p.part,
                    step.kind
                );
            }
        }
    }

    #[test]
    fn witness_lines_are_archived_bounded_and_printable() {
        let many: Vec<String> = (0..20)
            .map(|i| format!("[NOISY] i2c1.u2: Read {i:X}"))
            .collect();
        let kept = archived_lines(&many);
        assert_eq!(kept.len(), aethelgard_chaos::record::MAX_WITNESS_LINES);
        assert_eq!(kept[..], many[..16]);
        let odd = archived_lines(&["[NOISY] i2c1.u2: Read 60\u{e9}".to_owned(), "x".repeat(300)]);
        assert_eq!(odd[0], "[NOISY] i2c1.u2: Read 60\\u{e9}");
        assert_eq!(odd[1].len(), 256);
        assert!(
            odd.iter()
                .all(|l| aethelgard_chaos::event::text_ok(l, 1, 256))
        );
    }
    use serde_json::{Value, json};

    fn spec() -> Specification {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/sentinel-01-minimal.json"
        );
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    }

    fn scenario_with(overrides: Value) -> Scenario {
        let doc = json!({"schema_version": 1, "name": "t", "base": "b.json",
            "expect": "pass", "expect_findings": [], "overrides": overrides});
        Scenario::parse(&serde_json::to_vec(&doc).unwrap()).unwrap()
    }

    #[test]
    fn apply_sets_the_bus_pull_up() {
        let mut s = spec();
        assert_ne!(s.i2c_buses[0].pull_up_ohms, 47000);
        let held = apply(
            &scenario_with(json!([{ "bus": "SENSORS", "pull_up_ohms": 47000 }])),
            &mut s,
        )
        .unwrap();
        assert!(held.is_empty());
        let bus = s.i2c_buses.iter().find(|b| b.id == "SENSORS").unwrap();
        assert_eq!(bus.pull_up_ohms, 47000);
    }

    #[test]
    fn apply_sets_the_net_capacitance() {
        let mut s = spec();
        let before = s.clone();
        apply(
            &scenario_with(json!([{ "net": "I2C_SDA", "max_capacitance_femtofarads": 50000 }])),
            &mut s,
        )
        .unwrap();
        let c = |s: &Specification, n: &str| {
            s.net_constraints
                .iter()
                .find(|c| c.net_name == n)
                .unwrap()
                .max_capacitance_femtofarads
        };
        assert_eq!(c(&s, "I2C_SDA"), 50000);
        assert_eq!(c(&s, "I2C_SCL"), c(&before, "I2C_SCL"));
    }

    #[test]
    fn apply_refuses_an_unknown_bus() {
        let mut s = spec();
        let e = apply(
            &scenario_with(json!([{ "bus": "NOPE", "pull_up_ohms": 2200 }])),
            &mut s,
        )
        .unwrap_err();
        assert_eq!(e.kind, "UnknownBus");
        assert_eq!(e.id.as_deref(), Some("NOPE"));
        let e = apply(
            &scenario_with(json!([{ "bus": "NOPE", "line_held_low": "sda" }])),
            &mut spec(),
        )
        .unwrap_err();
        assert_eq!(e.kind, "UnknownBus");
    }

    #[test]
    fn apply_refuses_an_unknown_net() {
        let mut s = spec();
        let e = apply(
            &scenario_with(json!([{ "net": "NOPE", "max_capacitance_femtofarads": 1 }])),
            &mut s,
        )
        .unwrap_err();
        assert_eq!(e.kind, "UnknownNet");
        assert_eq!(e.id.as_deref(), Some("NOPE"));
    }

    #[test]
    fn apply_refuses_a_zero_pull_up_by_the_specifications_own_rule() {
        let mut s = spec();
        let e = apply(
            &scenario_with(json!([{ "bus": "SENSORS", "pull_up_ohms": 0 }])),
            &mut s,
        )
        .unwrap_err();
        assert_eq!(e.kind, "ZeroPullUp");
        assert_eq!(e.id.as_deref(), Some("SENSORS"));
    }

    #[test]
    fn line_held_low_reports_the_line_and_leaves_the_spec_alone() {
        let mut s = spec();
        let before = s.clone();
        let held = apply(
            &scenario_with(json!([{ "bus": "SENSORS", "line_held_low": "sda" }])),
            &mut s,
        )
        .unwrap();
        assert_eq!(
            held,
            vec![HeldLow {
                bus: "SENSORS".into(),
                net: "I2C_SDA".into()
            }]
        );
        assert_eq!(s, before);
        let held = apply(
            &scenario_with(json!([{ "bus": "SENSORS", "line_held_low": "scl" }])),
            &mut s,
        )
        .unwrap();
        assert_eq!(held[0].net, "I2C_SCL");
        assert_eq!(s, before);
    }

    /// The reference board's solved topology, as `solve` gives it.
    fn topology() -> aethelgard_ir::SolvedTopology {
        serde_json::from_value(json!({
            "schema_version": 3,
            "specification_sha256": "acd60081f2d111b1b1ed0d974078fff45ccf2a967ec5f42317417f8ce68149d1",
            // No stand-in clock or supply under any version (AET-WO-4240v §1.1).
            "clock": aethelgard_ir::SolvedClock::reset(),
            "core_supply": "ldo",
            "assignments": {
                "I2C_SCL": {"pin": "PB6", "peripheral": "I2C1", "signal": "I2C1_SCL", "alternate_function": 4},
                "I2C_SDA": {"pin": "PB7", "peripheral": "I2C1", "signal": "I2C1_SDA", "alternate_function": 4}
            },
            "gpio": {},
            "i2c_ceilings": {"SENSORS": 400000},
            "i2c_speeds": {"SENSORS": 400000},
            "analog_lines": {
                "I2C_SCL": {"bus": "SENSORS", "pull_up_ohms": 2200, "capacitance_femtofarads": 100000, "ceiling_ns": 300, "domain_millivolts": 3300},
                "I2C_SDA": {"bus": "SENSORS", "pull_up_ohms": 2200, "capacitance_femtofarads": 100000, "ceiling_ns": 250, "domain_millivolts": 3300}
            },
            "i2c_devices": {
                "U2": {"index": 0, "part": "BME280", "bus": "SENSORS", "peripheral": "I2C1", "address": 118},
                "U3": {"index": 1, "part": "TMP117", "bus": "SENSORS", "peripheral": "I2C1", "address": 72}
            },
            "spi_devices": {}
        }))
        .unwrap()
    }

    #[test]
    fn kind_15_is_the_solved_bus_scl_first_with_each_lines_own_ceiling() {
        let [
            Record::I2cBusSolved {
                bus,
                peripheral,
                lines,
                devices,
            },
        ] = <[Record; 1]>::try_from(solved_buses(&topology()).unwrap()).unwrap()
        else {
            panic!()
        };
        assert_eq!((bus.as_str(), peripheral.as_str()), ("SENSORS", "I2C1"));
        let got: Vec<(&str, &str, u64)> = lines
            .iter()
            .map(|l| (l.signal.as_str(), l.pin.as_str(), l.ceiling_ps))
            .collect();
        assert_eq!(
            got,
            [("I2C1_SCL", "PB6", 300_000), ("I2C1_SDA", "PB7", 250_000)]
        );
        assert_eq!(devices, [("U2".to_owned(), 0x76), ("U3".to_owned(), 0x48)]);
    }

    #[test]
    fn scl_comes_first_even_when_its_net_sorts_after_sdas() {
        let mut t = topology();
        let a = t.assignments.remove("I2C_SCL").unwrap();
        let l = t.analog_lines.remove("I2C_SCL").unwrap();
        t.assignments.insert("Z_SCL".into(), a);
        t.analog_lines.insert("Z_SCL".into(), l);
        let [Record::I2cBusSolved { lines, .. }] =
            <[Record; 1]>::try_from(solved_buses(&t).unwrap()).unwrap()
        else {
            panic!()
        };
        assert_eq!(lines[0].net, "Z_SCL");
        assert_eq!(lines[1].net, "I2C_SDA");
    }

    #[test]
    fn a_topology_whose_lines_and_devices_disagree_is_refused_never_guessed() {
        let refused = |edit: &dyn Fn(&mut aethelgard_ir::SolvedTopology)| {
            let mut t = topology();
            edit(&mut t);
            solved_buses(&t).unwrap_err()
        };
        let e = refused(&|t| t.assignments.get_mut("I2C_SDA").unwrap().peripheral = "I2C2".into());
        assert!(e.contains("two controllers"), "{e}");
        let e = refused(&|t| t.i2c_devices.get_mut("U2").unwrap().peripheral = "I2C2".into());
        assert!(e.contains("device `U2`"), "{e}");
        let e = refused(&|t| t.i2c_devices.get_mut("U3").unwrap().bus = "NOPE".into());
        assert!(e.contains("no solved line"), "{e}");
        let e = refused(&|t| {
            t.assignments.remove("I2C_SDA");
        });
        assert!(e.contains("no pin assignment"), "{e}");
    }

    /// A computed rise rounds up and a lower bound rounds down, so neither
    /// crosses a whole-nanosecond ceiling in the wrong direction.
    #[test]
    fn a_rise_rounds_up_and_a_lower_bound_rounds_down() {
        assert_eq!(picoseconds(3982.300059495949, true), Ok(3_982_301));
        assert_eq!(picoseconds(3982.300059495949, false), Ok(3_982_300));
        // Just past a 300 ns ceiling stays past it; exactly at it stays at it.
        assert_eq!(picoseconds(300.0001, true), Ok(300_001));
        assert_eq!(picoseconds(300.0, true), Ok(300_000));
        assert_eq!(picoseconds(13298.0277, false), Ok(13_298_027));
        for bad in [f64::NAN, f64::INFINITY, -1.0, 1e30] {
            assert!(picoseconds(bad, true).is_err(), "{bad}");
        }
    }
}
