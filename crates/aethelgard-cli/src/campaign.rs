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
use aethelgard_chaos::record::{BusLine, Line, Override, Reading, Record};
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

/// Every scenario boots for the longest virtual time `simulate --firmware`
/// admits, so a read that times out still reaches the boot word (`AET-WO-2202`).
pub const VIRTUAL_NS: u64 = aethelgard_twin::boot::MAX_VIRTUAL_NS;

/// One scenario, read, applied and solved, ready to run.
struct Prepared {
    path: PathBuf,
    /// The base specification file's own bytes, as read.
    base_sha256: [u8; 32],
    scenario: Scenario,
    specification_sha256: [u8; 32],
    topology: SolvedTopology,
    held: Vec<HeldLow>,
    rail: Option<crate::analog::Rail>,
    run_id: [u8; 16],
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
    } = effective(&scenario, &base)?;
    let rail = match scenario.droop() {
        Some(d) => Some(resolve_droop(d, &spec, &topology).map_err(|e| {
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
        rail,
        run_id,
    })
}

/// A droop's domain, nominal, tolerance and decoupling, resolved against the
/// effective specification, then checked against the supervisor's table
/// (ruled D3, D7). Only the MCU's own supply can be drooped: its supervisor is
/// the one the campaign renders. Every refusal is the scenario's (exit 2).
fn resolve_droop(
    droop: &aethelgard_chaos::record::Droop,
    spec: &Specification,
    topology: &SolvedTopology,
) -> Result<crate::analog::Rail, ScenarioError> {
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
    rules::check_droop(
        droop,
        domain.nominal_millivolts,
        decoupling_picofarads,
        VIRTUAL_NS,
    )?;
    Ok(crate::analog::Rail {
        droop: droop.clone(),
        nominal_millivolts: domain.nominal_millivolts,
        tolerance_permille: u32::from(domain.tolerance_permille),
        decoupling_picofarads,
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
    Ok(Effective {
        spec,
        held,
        specification_sha256,
        topology,
    })
}

#[derive(Serialize)]
struct ScenarioLine {
    scenario: String,
    file: String,
    verdict: &'static str,
    expect: &'static str,
    matched: bool,
    findings: Vec<String>,
    expect_findings: Vec<String>,
    run_id: String,
    run: String,
    state_sequence_hash: String,
    event_count: u64,
    trace_bytes: u64,
    /// The boot's `uid_boundary` and, when false, why (`AET-WO-1601`).
    uid_boundary: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    uid_boundary_unmet: Option<run::Unmet>,
}

#[derive(Serialize)]
struct CampaignSummary {
    ok: bool,
    verdict: &'static str,
    check: &'static str,
    scenarios: usize,
    matched: usize,
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

    let (mut matched, mut total) = (0, 0u64);
    for p in prepared {
        let (line, ok) = match run_one(p, firmware, &firmware_sha256, archive) {
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
            virtual_time_ns: VIRTUAL_NS,
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
    let verdict = crate::simulate(
        &platform,
        Some(Boot {
            firmware: firmware.to_path_buf(),
            topology: p.topology,
            virtual_ns: VIRTUAL_NS,
            scenario: p.scenario.name.clone(),
            held: p.held,
            rail: p.rail,
        }),
    )
    .map_err(stop)?;
    drop(scratch);
    let (uid_boundary, uid_boundary_unmet) = (
        verdict.sandbox.uid_boundary,
        verdict.sandbox.uid_boundary_unmet,
    );
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
    records.extend(records_of(&verdict).map_err(internal)?);
    // The run must pass the rules replay will judge it by before it is written.
    let summary = rules::check_run(&records)
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
    };
    let dir = archive.join(hex(p.run_id));
    let archived = (|| {
        let mut writer = TraceWriter::create(&dir, &header)?;
        for record in records {
            let timestamp = record.timestamp(VIRTUAL_NS);
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
    Ok((
        ScenarioLine {
            scenario: summary.name,
            file: p.path.display().to_string(),
            verdict: pass_fail(summary.pass),
            expect: pass_fail(summary.expect_pass),
            matched: ok,
            findings: summary.failing_kinds,
            expect_findings: summary.expect_findings,
            run_id: hex(p.run_id),
            run: dir.display().to_string(),
            state_sequence_hash: hex(archived.state_hash),
            event_count: archived.event_count,
            trace_bytes: archived.trace_bytes,
            uid_boundary,
            uid_boundary_unmet,
        },
        ok,
    ))
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

/// Kinds 16 to 19 from a droop's report, the crossings, resets and
/// observations merged in the time order replay checks (`AET-WO-2301`).
fn droop_records(d: &crate::verdict::DroopReport) -> Result<Vec<Record>, String> {
    let r = &d.rail;
    let mut out = vec![Record::RailDeclared {
        droop: aethelgard_chaos::record::Droop {
            domain: r.domain.clone(),
            floor_millivolts: r.floor_millivolts,
            start_ns: r.start_ns,
            fall_ns: r.fall_ns,
            hold_ns: r.hold_ns,
            rise_ns: r.rise_ns,
            source_milliohms: r.source_milliohms,
            load_milliamps: r.load_milliamps,
            bor_level: r.bor_level,
        },
        nominal_millivolts: r.nominal_millivolts,
        tolerance_permille: r.tolerance_permille,
        decoupling_picofarads: r.decoupling_picofarads,
    }];
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
                witness: o
                    .witness
                    .as_ref()
                    .and_then(|w| w.observed.as_deref())
                    .map(archived_lines),
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

/// Picoseconds from the bridge's nanoseconds. A measured rise rounds up, so
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
fn records_of(v: &Verdict) -> Result<Vec<Record>, String> {
    let mut out = Vec::new();
    for a in v.analog.as_deref().unwrap_or_default() {
        let ceiling_ps = u64::from(a.ceiling_ns) * 1000;
        let record = match (
            a.rise_30_70_ns,
            a.rise_30_70_ns_lower_bound,
            a.volts_at_release,
        ) {
            (Some(rise), None, None) => Record::RiseMeasured {
                bus: a.bus.clone(),
                net: a.net.clone(),
                rise_30_70_ps: picoseconds(rise, true)?,
                complete: a.complete,
                ceiling_ps,
                exceeds: a.exceeds,
            },
            (None, Some(bound), None) => Record::RiseMeasured {
                bus: a.bus.clone(),
                net: a.net.clone(),
                rise_30_70_ps: picoseconds(bound, false)?,
                complete: a.complete,
                ceiling_ps,
                exceeds: a.exceeds,
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
        out.extend(droop_records(d)?);
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
            register: u8::try_from(hex_u64(&c.register)?).map_err(|e| e.to_string())?,
            expected: hex_u64(&c.expected)?,
            observed: c.observed.as_deref().map(hex_u64).transpose()?,
            status: c.status.map(str::to_owned),
        });
    }
    for w in &boot.i2c_witness {
        out.push(Record::I2cWitness {
            peripheral: w.peripheral.clone(),
            reference: w.reference.clone(),
            device: w.device.clone(),
            expected: w.expected.clone(),
            observed: w.observed.as_deref().map(archived_lines),
            absent: w
                .absent
                .iter()
                .take(aethelgard_chaos::record::MAX_WITNESS_LINES)
                .map(|a| u8::try_from(hex_u64(a)?).map_err(|e| e.to_string()))
                .collect::<Result<_, _>>()?,
        });
    }
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
    };
    let lines = |v: Vec<aethelgard_evolve::EventRef>| {
        v.into_iter()
            .map(|e| EventLine {
                index: e.index,
                kind: e.kind,
            })
            .collect()
    };
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
        },
        if determinate { 0 } else { EXIT_FAIL },
    )
}

#[cfg(test)]
mod tests {
    use super::{
        HeldLow, Record, Scenario, Specification, apply, archived_lines, picoseconds, solved_buses,
    };

    /// `AET-WO-2204`: an observation is archived as at most 16 lines of
    /// printable ASCII, each at most 256 bytes; a changed line still differs
    /// from every line a model logs, so the archive cannot make it match.
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
            "schema_version": 2,
            "specification_sha256": "acd60081f2d111b1b1ed0d974078fff45ccf2a967ec5f42317417f8ce68149d1",
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
            }
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

    /// A measured rise rounds up and a lower bound rounds down, so neither
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
