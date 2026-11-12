//! `aethelgard propose --archive <run dir> --scenario <file> --out <dir>`
//! (`AET-WO-3200`, `RUL-07`, `RUL-18`).
//!
//! It checks first: `attribute`'s checks, and with them `replay`'s; the
//! scenario file is the run's (its sha256 and overrides are kind 5's); its
//! base resolves exactly as `campaign` resolves it, to the archived bytes;
//! and the solved topology is kind 6's, with kind 15 exactly what `campaign`
//! writes from it. Then it attributes the run, hands the rules proposer the
//! solved facts as integers, and writes `proposal.json`, with
//! `proposed-spec.json` for a proposal or `generator-report.json` for a
//! generator report, into an empty or absent `--out`. Nothing else is
//! written, no process is started, and nothing is applied to any
//! specification but the copy written there.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use aethelgard_chaos::record::{Override, Record};
use aethelgard_evolve::propose::{
    self, Answer, BusFacts, Expectation, GeneratorReport, HEADROOM, Input, Outcome, Rules, Sizing,
};
use aethelgard_evolve::{AttributeError, Attribution, EventRef, Evidence};
use aethelgard_ir::electrical::i2c_pull_up_floor_ohms;
use aethelgard_ir::{SolvedTopology, Specification};
use serde::Serialize;

use crate::campaign::{
    Effective, apply_overrides, checked, digest, effective, read_base, read_scenario, refuse,
    solved_buses,
};
use crate::input::hex;
use crate::{EXIT_FAIL, EXIT_GENERATOR_REPORT, EXIT_INTERNAL, EXIT_UNREADABLE, emit};

const PROPOSAL: &str = "proposal.json";
const PROPOSED_SPEC: &str = "proposed-spec.json";
const GENERATOR_REPORT: &str = "generator-report.json";

/// Said in `proposal.json`, because a reader takes the next decision on it
/// (`RUL-19`).
const WHAT_THIS_IS: &str = "A proposal is evidence about the specification, not the board. Every rise here is computed from declared values: ngspice arithmetic on the specification's pull-up and its max_capacitance bound, not a measurement of a routed board (RUL-19).";

#[derive(Serialize)]
struct EventLine {
    index: u64,
    kind: u64,
}

fn events(refs: &[EventRef]) -> Vec<EventLine> {
    refs.iter()
        .map(|e| EventLine {
            index: e.index,
            kind: e.kind,
        })
        .collect()
}

#[derive(Serialize)]
struct AttributionLine {
    domain: &'static str,
    rule: &'static str,
    evidence: Vec<EventLine>,
    /// Software evidence on the faulted bus, set aside by `E1`. When it is
    /// not empty, the run also shows a firmware defect the proposal does not
    /// fix, and a re-gate on regenerated firmware cannot see it
    /// (adversarial B2).
    masked: Vec<EventLine>,
}

#[derive(Serialize)]
struct ScenarioLine {
    name: String,
    sha256: String,
}

/// A change in the scenario override form, `{"bus", "pull_up_ohms"}`.
#[derive(Serialize)]
struct ChangeLine {
    bus: String,
    pull_up_ohms: u32,
}

#[derive(Serialize)]
struct SizingLine {
    bus: String,
    r_now_ohms: u32,
    binding_net: String,
    rise_worst_ps: u64,
    ceiling_ps: u64,
    headroom: String,
    r_target_ohms: u64,
    rp_min_ohms: u32,
    e12_ohms: Option<u32>,
    predicted_rise_ps: Option<u64>,
}

fn sizing(s: &Sizing) -> SizingLine {
    SizingLine {
        bus: s.bus.clone(),
        r_now_ohms: s.r_now_ohms,
        binding_net: s.binding_net.clone(),
        rise_worst_ps: s.rise_worst_ps,
        ceiling_ps: s.ceiling_ps,
        headroom: format!("{}/{}", HEADROOM.0, HEADROOM.1),
        r_target_ohms: s.r_target_ohms,
        rp_min_ohms: s.rp_min_ohms,
        e12_ohms: s.e12_ohms,
        predicted_rise_ps: s.predicted_rise_ps,
    }
}

/// `proposal.json`: every byte a function of the run, the scenario's bytes
/// and its base's bytes, never of a path.
#[derive(Serialize)]
struct ProposalFile {
    schema_version: u32,
    outcome: &'static str,
    proposer: Option<&'static str>,
    reason: Option<String>,
    run_id: String,
    state_sequence_hash: String,
    scenario: ScenarioLine,
    base_sha256: String,
    specification_sha256: String,
    topology_sha256: String,
    attribution: AttributionLine,
    changes: Vec<ChangeLine>,
    sizing: Vec<SizingLine>,
    proposed_specification_sha256: Option<String>,
    note: &'static str,
}

#[derive(Serialize)]
struct ExpectationLine {
    index: u64,
    kind: u64,
    generator: &'static str,
    what: String,
    address: Option<String>,
    mask: Option<String>,
    solved: String,
    observed: Option<String>,
}

fn expectation(e: &Expectation) -> ExpectationLine {
    // A register field is shown in hex, the instruction count in decimal.
    let show = |v: u64| match e.address {
        Some(_) => format!("0x{v:08x}"),
        None => v.to_string(),
    };
    ExpectationLine {
        index: e.event.index,
        kind: e.event.kind,
        generator: e.generator.name(),
        what: e.what.clone(),
        address: e.address.map(|a| format!("0x{a:08x}")),
        mask: e.mask.map(|m| format!("0x{m:08x}")),
        solved: show(e.solved),
        observed: e.observed.map(show),
    }
}

#[derive(Serialize)]
struct ReportFile {
    schema_version: u32,
    run_id: String,
    state_sequence_hash: String,
    rule: &'static str,
    evidence: Vec<EventLine>,
    generators: Vec<&'static str>,
    expectations: Vec<ExpectationLine>,
    note: &'static str,
}

const REPORT_NOTE: &str = "A software attribution in generated firmware is a generator defect. This report names the solved expectations the generated firmware did not meet; it carries no specification change (RUL-18).";

#[derive(Serialize)]
struct Proposed {
    ok: bool,
    check: &'static str,
    outcome: &'static str,
    proposer: Option<&'static str>,
    run_id: String,
    out: String,
    files: BTreeMap<&'static str, String>,
}

/// The solved facts of each bus, from the verified topology: the pull-up and
/// rail both lines carry, and the floor at that rail and the bus's declared
/// `speed_hz`, the speed `solve` judges the floor at. Never the topology's
/// `i2c_ceilings`, which is the fastest speed the rise ceiling allows, not
/// the bus's own (adversarial B1).
fn facts(spec: &Specification, t: &SolvedTopology) -> Result<BTreeMap<String, BusFacts>, String> {
    let mut out: BTreeMap<String, BusFacts> = BTreeMap::new();
    for (net, a) in &t.analog_lines {
        let speed = spec
            .i2c_buses
            .iter()
            .find(|b| b.id == a.bus)
            .map(|b| b.speed_hz)
            .ok_or_else(|| format!("bus `{}` is not in the specification", a.bus))?;
        let f = BusFacts {
            pull_up_ohms: a.pull_up_ohms,
            domain_millivolts: a.domain_millivolts,
            floor_ohms: i2c_pull_up_floor_ohms(speed, a.domain_millivolts),
        };
        if *out.entry(a.bus.clone()).or_insert(f) != f {
            return Err(format!(
                "line `{net}` of bus `{}` disagrees with its other line's pull-up or rail",
                a.bus
            ));
        }
    }
    Ok(out)
}

fn pretty<T: Serialize>(v: &T) -> Result<Vec<u8>, ExitCode> {
    let mut bytes = serde_json::to_vec_pretty(v).map_err(|e| {
        crate::diagnose(&format!("cannot serialize: {e}"));
        ExitCode::from(EXIT_INTERNAL)
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Write `files` into `out`, which must be absent or an empty directory. The
/// directory is opened once without following a symlink, and every file is
/// created new inside it, never followed or replaced.
fn write_out(out: &Path, files: &[(&'static str, Vec<u8>)]) -> Result<(), ExitCode> {
    use rustix::fs::{Mode, OFlags};
    let unwritable = |m: String| mismatch("UnwritableOutput", m);
    match fs::symlink_metadata(out) {
        Ok(meta) if meta.is_dir() => {
            let mut entries = fs::read_dir(out)
                .map_err(|e| unwritable(format!("cannot read {}: {e}", out.display())))?;
            if entries.next().is_some() {
                return Err(unwritable(format!(
                    "{} is not empty: propose writes only into an empty or absent directory",
                    out.display()
                )));
            }
        }
        Ok(_) => {
            return Err(unwritable(format!(
                "{} exists and is not a directory",
                out.display()
            )));
        }
        Err(_) => fs::create_dir_all(out)
            .map_err(|e| unwritable(format!("cannot create {}: {e}", out.display())))?,
    }
    let dir = rustix::fs::open(
        out,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|e| unwritable(format!("cannot open {}: {e}", out.display())))?;
    for (name, bytes) in files {
        let fd = rustix::fs::openat(
            &dir,
            *name,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::from_raw_mode(0o644),
        )
        .map_err(|e| unwritable(format!("cannot create {name} in {}: {e}", out.display())))?;
        std::io::Write::write_all(&mut fs::File::from(fd), bytes)
            .map_err(|e| unwritable(format!("cannot write {name} in {}: {e}", out.display())))?;
    }
    Ok(())
}

/// Whether `out` resolves outside the run directory `dir`, judged at its
/// nearest existing ancestor with every symlink resolved: the run directory
/// exists, so a component still to be created never leads into it. A path
/// with no such ancestor (`absent/..`) is not shown to be outside
/// (adversarial B3).
fn outside_run(out: &Path, dir: &Path) -> bool {
    let Ok(run) = fs::canonicalize(dir) else {
        return false;
    };
    let mut existing = out;
    loop {
        if let Ok(resolved) = fs::canonicalize(existing) {
            return !resolved.starts_with(&run);
        }
        match (existing.file_name(), existing.parent()) {
            (Some(_), Some(parent)) if parent.as_os_str().is_empty() => existing = Path::new("."),
            (Some(_), Some(parent)) => existing = parent,
            _ => return false,
        }
    }
}

/// A refusal at exit 3: the input is not this run's, no longer gives the
/// run, or the output cannot be written.
fn mismatch(kind: &'static str, message: String) -> ExitCode {
    refuse(kind, None, message, EXIT_UNREADABLE)
}

/// `aethelgard propose`. Exit 0 with a proposal, 6 with a generator report,
/// 1 with no proposal.
pub fn propose(dir: &Path, scenario_path: &Path, out: &Path) -> ExitCode {
    run(dir, scenario_path, out).unwrap_or_else(|code| code)
}

fn run(dir: &Path, scenario_path: &Path, out: &Path) -> Result<ExitCode, ExitCode> {
    let (replayed, _) = checked(dir)?;
    let records = &replayed.state.records;
    let header = &replayed.header;
    let artifact = |name: &str| {
        header
            .artifacts
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, h)| *h)
            .expect("check_archived pins the five artifacts")
    };
    let (
        Record::Scenario {
            sha256, overrides, ..
        },
        Record::Solved {
            specification_sha256,
            topology_sha256,
        },
    ) = (&records[0], &records[1])
    else {
        unreachable!("check_run puts the scenario and the topology first");
    };

    let scenario = read_scenario(scenario_path)?;
    if scenario.sha256 != *sha256 || scenario.overrides != *overrides {
        return Err(mismatch(
            "ScenarioMismatch",
            format!(
                "{} has sha256 {}, but run {} ran scenario sha256 {}",
                scenario_path.display(),
                hex(scenario.sha256),
                hex(header.run_id),
                hex(sha256)
            ),
        ));
    }
    let base = read_base(scenario_path, &scenario.base)
        .map_err(|(kind, m)| refuse(kind, Some(&scenario.name), m, EXIT_UNREADABLE))?;
    if digest(&base) != artifact("base") {
        return Err(mismatch(
            "BaseChanged",
            format!(
                "base `{}` has sha256 {}, but the run's base had {}",
                scenario.base,
                hex(digest(&base)),
                hex(artifact("base"))
            ),
        ));
    }
    let Effective {
        spec,
        specification_sha256: spec_sha,
        topology,
        ..
    } = effective(&scenario, &base)?;
    let topology_sha = digest(&serde_json::to_vec(&topology).map_err(|e| {
        crate::diagnose(&format!("cannot serialize a topology: {e}"));
        ExitCode::from(EXIT_INTERNAL)
    })?);
    if spec_sha != *specification_sha256 || topology_sha != *topology_sha256 {
        return Err(mismatch(
            "TopologyChanged",
            format!(
                "the scenario now gives specification {} and topology {}, but the run judged {} and {}",
                hex(spec_sha),
                hex(topology_sha),
                hex(specification_sha256),
                hex(topology_sha256)
            ),
        ));
    }
    let solved = records
        .iter()
        .filter(|r| matches!(r, Record::I2cBusSolved { .. }));
    let written = solved_buses(&topology).map_err(|m| {
        crate::diagnose(&m);
        ExitCode::from(EXIT_INTERNAL)
    })?;
    if !solved.eq(&written) {
        return Err(mismatch(
            "TopologyChanged",
            format!(
                "{}: the run's solved buses (kind 15) are not the ones its topology gives",
                dir.display()
            ),
        ));
    }

    let a = aethelgard_evolve::attribute(&replayed).map_err(|e| register_refusal(dir, &e))?;
    let view = aethelgard_evolve::view(&replayed).map_err(|e| register_refusal(dir, &e))?;
    let evidence: Vec<Evidence> = a
        .evidence
        .iter()
        .map(|e| {
            view.event(*e)
                .expect("attribute names only events of the run")
        })
        .collect();
    let buses = view.buses();
    let facts = facts(&spec, &topology).map_err(|m| {
        crate::diagnose(&m);
        ExitCode::from(EXIT_INTERNAL)
    })?;
    let answer = propose::propose(
        &[&Rules],
        &Input {
            attribution: &a,
            evidence: &evidence,
            buses: &buses,
            facts: &facts,
        },
    );

    let mut files: Vec<(&'static str, Vec<u8>)> = Vec::new();
    let mut proposed_sha = None;
    if let Outcome::Proposal(p) = &answer.outcome {
        let changes: Vec<Override> = p
            .changes
            .iter()
            .map(|c| Override::PullUpOhms {
                bus: c.bus.clone(),
                ohms: c.pull_up_ohms,
            })
            .collect();
        let mut proposed = spec.clone();
        apply_overrides(&changes, &mut proposed).map_err(|e| {
            refuse(
                e.kind,
                e.id.as_deref(),
                format!("the proposal leaves the specification invalid: {e}"),
                crate::EXIT_INVALID_SPEC,
            )
        })?;
        let bytes = pretty(&proposed)?;
        proposed_sha = Some(hex(digest(&bytes)));
        files.push((PROPOSED_SPEC, bytes));
    }
    if let Outcome::GeneratorReport(g) = &answer.outcome {
        files.push((GENERATOR_REPORT, pretty(&report(&a, g))?));
    }
    let proposal = proposal_file(
        &answer,
        &a,
        &scenario.name,
        ArtifactHashes {
            scenario: *sha256,
            base: artifact("base"),
            specification: *specification_sha256,
            topology: *topology_sha256,
        },
        proposed_sha,
    );
    files.insert(0, (PROPOSAL, pretty(&proposal)?));
    if !outside_run(out, dir) {
        return Err(mismatch(
            "UnwritableOutput",
            format!(
                "{} is inside the run directory {}, or cannot be shown to be outside it: a run is never written to",
                out.display(),
                dir.display()
            ),
        ));
    }
    write_out(out, &files)?;

    let (outcome, code) = match answer.outcome {
        Outcome::Proposal(_) => ("proposal", ExitCode::SUCCESS),
        Outcome::GeneratorReport(_) => ("generator_report", ExitCode::from(EXIT_GENERATOR_REPORT)),
        Outcome::NoProposal { .. } => ("no_proposal", ExitCode::from(EXIT_FAIL)),
    };
    let line = Proposed {
        ok: code == ExitCode::SUCCESS,
        check: "propose",
        outcome,
        proposer: answer.proposer,
        run_id: hex(header.run_id),
        out: out.display().to_string(),
        files: files.iter().map(|(n, b)| (*n, hex(digest(b)))).collect(),
    };
    let printed = emit(&line, 0);
    Ok(if printed == ExitCode::SUCCESS {
        code
    } else {
        printed
    })
}

fn register_refusal(dir: &Path, e: &AttributeError) -> ExitCode {
    match e {
        // `checked` has applied the same rules, so this is not reached.
        AttributeError::NotACampaignRun(reason) => {
            mismatch("NotACampaignRun", format!("{}: {reason}", dir.display()))
        }
        AttributeError::RegisterFacts(e) => mismatch(
            e.kind(),
            format!("{}: a register fact the run names: {e}", dir.display()),
        ),
    }
}

struct ArtifactHashes {
    scenario: [u8; 32],
    base: [u8; 32],
    specification: [u8; 32],
    topology: [u8; 32],
}

fn proposal_file(
    answer: &Answer,
    a: &Attribution,
    scenario: &str,
    h: ArtifactHashes,
    proposed: Option<String>,
) -> ProposalFile {
    let (outcome, reason, changes, sizes) = match &answer.outcome {
        Outcome::Proposal(p) => (
            "proposal",
            None,
            p.changes
                .iter()
                .map(|c| ChangeLine {
                    bus: c.bus.clone(),
                    pull_up_ohms: c.pull_up_ohms,
                })
                .collect(),
            p.sizing.iter().map(sizing).collect(),
        ),
        Outcome::GeneratorReport(_) => ("generator_report", None, vec![], vec![]),
        Outcome::NoProposal { reason, sizing: s } => (
            "no_proposal",
            Some(reason.clone()),
            vec![],
            s.iter().map(sizing).collect(),
        ),
    };
    ProposalFile {
        schema_version: 1,
        outcome,
        proposer: answer.proposer,
        reason,
        run_id: hex(a.run_id),
        state_sequence_hash: hex(a.state_hash),
        scenario: ScenarioLine {
            name: scenario.to_owned(),
            sha256: hex(h.scenario),
        },
        base_sha256: hex(h.base),
        specification_sha256: hex(h.specification),
        topology_sha256: hex(h.topology),
        attribution: AttributionLine {
            domain: a.domain.name(),
            rule: a.rule.id(),
            evidence: events(&a.evidence),
            masked: events(&a.masked),
        },
        changes,
        sizing: sizes,
        proposed_specification_sha256: proposed,
        note: WHAT_THIS_IS,
    }
}

fn report(a: &Attribution, g: &GeneratorReport) -> ReportFile {
    let mut generators: Vec<&'static str> =
        g.expectations.iter().map(|e| e.generator.name()).collect();
    generators.sort_unstable();
    generators.dedup();
    ReportFile {
        schema_version: 1,
        run_id: hex(a.run_id),
        state_sequence_hash: hex(a.state_hash),
        rule: a.rule.id(),
        evidence: events(&a.evidence),
        generators,
        expectations: g.expectations.iter().map(expectation).collect(),
        note: REPORT_NOTE,
    }
}
