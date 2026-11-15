//! `verify`: one verdict that binds a specification to every artifact it
//! produced (`AET-WO-3301` §1.3, `INV-01`, `INV-03`, `RUL-19` (2)).
//!
//! The tree is generated once, in memory, and every expected hash is taken
//! from those bytes. The tree is then written into a guarded scratch
//! directory and each gate runs as this binary's own subcommand on it, so
//! each gate copies, hashes and judges exactly as a user's call does. After
//! each gate, the hash the gate reports for what it judged must equal the
//! generated one, or `verify` refuses (`BindingMismatch`, exit 5): a file
//! swapped in `verify`'s own scratch, or a gate that reports judging other
//! bytes, is never named in a passing verdict. No hash in the verdict comes
//! from a second read of scratch.
//!
//! **What it does not bind (adversarial B1 to B4).** Each gate hashes its
//! own copy and then hands that copy's path to its tool. A process of the
//! same user that writes into a gate's private scratch between the two (a
//! source file before cargo reads it, a `.cargo/config.toml` in the tool's
//! `HOME`, an ELF before Renode loads it) changes what the tool judges while
//! the hash the gate reports stays the generated one. That is every gate's
//! window, not `verify`'s alone; closing it needs a boundary a same-user
//! process cannot cross (`RUL-19` (4)). See the FEEDBACK.
//!
//! The verdict names no scratch path, so two runs on one specification give
//! byte-identical verdicts.
//!
//! **The fabrication package (`AET-WO-3300`).** With `--fab-out D`, and only
//! when every gate passed, [`crate::fab`] exports the package and `verify`
//! writes it into `D` before printing the verdict, which names every file's
//! sha256 under `fab`. `MANIFEST.json` names the sha256 of the verdict line
//! the same call prints without `--fab-out`; that line is this verdict
//! without its final `fab` key. `--against-fab D` derives the package the
//! same way and compares it with `D`.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use serde::Serialize;
use serde_json::Value;

use aethelgard_cli::generate::{BOARD_DIR, KICAD_DIR};

use crate::campaign;
use crate::fab;
use crate::input::hex;
use crate::run;
use crate::tree::{self, Difference};
use crate::{EXIT_FAIL, EXIT_INTERNAL, EXIT_TOOL_ERROR, EXIT_UNREADABLE};

/// `RUL-19` (1), in the verdict itself.
const RISE_TIMES: &str = "computed from declared values";

#[derive(Serialize)]
struct Report {
    ok: bool,
    verdict: &'static str,
    check: &'static str,
    engine_version: &'static str,
    project_name: String,
    specification_sha256: String,
    topology_sha256: String,
    /// Every generated file's sha256, by its path in the tree.
    files: BTreeMap<String, String>,
    /// The lifecycle banner (`AET-WO-4210` D1): one line per `eol` or `nrnd`
    /// part the board uses, absent when it is empty. It is in the verdict
    /// that names the package, not in a file of the package: no board with an
    /// `eol` or `nrnd` part can be drawn today (FEEDBACK, adversarial A3).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    lifecycle: Vec<String>,
    /// `--against`: whether the directory matched the tree; absent without it.
    #[serde(skip_serializing_if = "Option::is_none")]
    against: Option<&'static str>,
    elf_sha256: Option<String>,
    rise_times: &'static str,
    gates: Vec<Gate>,
    /// `--scenarios`: each scenario's run; absent without it.
    #[serde(skip_serializing_if = "Option::is_none")]
    campaign: Option<Vec<Run>>,
    findings: Vec<Finding>,
    /// `--against-fab`: whether the directory matched the package. With
    /// `fab`, always the last keys, so the verdict without them is the line
    /// a call without the fab flags prints (the manifest's `verdict_sha256`).
    #[serde(skip_serializing_if = "Option::is_none")]
    against_fab: Option<&'static str>,
    /// `--fab-out` or `--against-fab`: the package.
    #[serde(skip_serializing_if = "Option::is_none")]
    fab: Option<FabReport>,
    /// The parts whose lifecycle the table cites but does not hold, which the
    /// package names (`AET-WO-4210` D1): never in the verdict without a fab
    /// flag.
    #[serde(skip)]
    lifecycle_unheld: Vec<String>,
    /// The `extract` gate's stated conditions, for the package's notes.
    #[serde(skip)]
    conditions: Vec<String>,
}

/// `--fab-out D` writes the package into `D`; `--against-fab D` compares it.
pub enum FabMode {
    Write(PathBuf),
    Compare(PathBuf),
}

#[derive(Serialize)]
struct FabReport {
    /// `written` (`--fab-out`, every gate passed), `derived` (`--against-fab`),
    /// or `not_written` / `not_derived` when a gate failed: nothing is written.
    status: &'static str,
    /// Each precondition of a package and its state (`AET-WO-3303` §1.1). A
    /// skipped harness was refused before any gate ran; campaign coverage is
    /// stated here and not yet enforced (FEEDBACK D1).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    preconditions: Vec<Precondition>,
    /// `kicad-cli`'s version, as the export ran it.
    tool_version: Option<String>,
    /// Whether every export ran under the separate uid (`AET-WO-1601`).
    uid_boundary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    uid_boundary_unmet: Option<run::Unmet>,
    /// The sha256 of the board the export copied, equal to the generated one.
    board_sha256: Option<String>,
    /// Every file of the package, `MANIFEST.json` included, by sha256.
    files: BTreeMap<String, String>,
    /// `--against-fab`: every file that differs, is missing or is extra.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    differences: Vec<Finding>,
    /// Each part on the board whose lifecycle no held vendor page states,
    /// named with the page it must be read from: a purchase never meets an
    /// unread lifecycle silently (`AET-WO-4210` D1).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    lifecycle_unheld: Vec<String>,
}

/// One precondition of a package: `met`, or `unmet` with what fails it.
#[derive(Serialize, Clone)]
struct Precondition {
    precondition: &'static str,
    state: &'static str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    unmet: Vec<String>,
}

impl Precondition {
    fn of(precondition: &'static str, unmet: Vec<String>) -> Self {
        let state = if unmet.is_empty() { "met" } else { "unmet" };
        Self {
            precondition,
            state,
            unmet,
        }
    }
}

/// Every precondition of a package, in a fixed order, on the gates that ran,
/// with [`uncovered`]'s list for the campaign's coverage.
fn preconditions(report: &Report, planned: &[String], uncovered: Vec<String>) -> Vec<Precondition> {
    let proved = |h: &String| {
        report
            .gates
            .iter()
            .any(|g| g.gate == "prove" && g.harness.as_ref() == Some(h) && g.verdict == "pass")
    };
    let failed = report
        .gates
        .iter()
        .filter(|g| g.verdict != "pass")
        .map(|g| match &g.harness {
            Some(h) => format!("{} {h}: {}", g.gate, g.verdict),
            None => format!("{}: {}", g.gate, g.verdict),
        })
        .chain(
            report
                .findings
                .iter()
                .filter(|f| f.severity == "error")
                .map(|f| format!("finding {}", f.kind)),
        )
        .collect();
    vec![
        Precondition::of(
            "every_harness_proved",
            planned
                .iter()
                .filter(|h| !proved(h))
                .map(|h| format!("{h} not proved"))
                .collect(),
        ),
        Precondition::of("campaign_faults_every_bus_and_rail", uncovered),
        Precondition::of("every_gate_passed", failed),
    ]
}

impl FabReport {
    fn none(status: &'static str, lifecycle_unheld: Vec<String>) -> Self {
        Self {
            status,
            preconditions: Vec::new(),
            tool_version: None,
            uid_boundary: None,
            uid_boundary_unmet: None,
            board_sha256: None,
            files: BTreeMap::new(),
            differences: Vec::new(),
            lifecycle_unheld,
        }
    }
}

/// Every part on the board whose lifecycle the table cites as pending (not
/// held), named with the page it must be read from, so a purchase never
/// meets an unread lifecycle silently (`AET-WO-4210` D1). An `eol` or `nrnd`
/// part needs no rule here: `validate` refuses it unless the specification
/// acknowledges it, and the verdict's banner names it (Scotty's ruling on
/// D1). Devices in the specification's order, I2C then SPI.
fn lifecycle_unheld(topology: &aethelgard_ir::SolvedTopology) -> Vec<String> {
    let Ok(table) = aethelgard_ir::parts::table() else {
        return Vec::new();
    };
    let mut unheld = Vec::new();
    for (reference, part) in topology.parts_in_order() {
        let Some(p) = table.get(part) else { continue };
        if let aethelgard_ir::parts::Held::Pending(url) = &p.citations.lifecycle.held {
            let line = format!(
                "{part} ({reference}): lifecycle `{}` not held, read {url}",
                p.lifecycle.name()
            );
            if !unheld.contains(&line) {
                unheld.push(line);
            }
        }
    }
    unheld
}

/// `--fab-out` must be an empty directory or absent. One that exists must be
/// the caller's and writable by no group or other, or another uid could
/// replace the package after the verdict names it (adversarial A1). Checked
/// before any gate runs and again just before the write; `tree::write`
/// checks emptiness once more through the descriptor it writes through.
/// Nothing is created here.
fn fab_out_usable(dir: &Path) -> Result<(), run::Stop> {
    use std::os::unix::fs::MetadataExt;
    let unusable = |m: String| run::Stop::refused("UnwritableOutput", m);
    match std::fs::symlink_metadata(dir) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(unusable(format!("cannot read {}: {e}", dir.display()))),
        Ok(meta) if !meta.is_dir() => Err(unusable(format!(
            "{} exists and is not a directory",
            dir.display()
        ))),
        Ok(meta)
            if meta.uid() != rustix::process::geteuid().as_raw() || meta.mode() & 0o022 != 0 =>
        {
            Err(unusable(format!(
                "{} is not this user's alone: it must be owned by the caller and writable by no group or other",
                dir.display()
            )))
        }
        Ok(_) => match std::fs::read_dir(dir).map(|mut d| d.next()) {
            Ok(None) => Ok(()),
            Ok(Some(_)) => Err(unusable(format!(
                "{} is not empty: the package is written only into an empty or absent directory",
                dir.display()
            ))),
            Err(e) => Err(unusable(format!("cannot read {}: {e}", dir.display()))),
        },
    }
}

#[derive(Serialize)]
struct Gate {
    gate: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    harness: Option<String>,
    /// `pass`, `fail`, or `not_run` when a gate before it failed so it could not run.
    verdict: &'static str,
    exit_code: Option<i32>,
    tool_version: Option<String>,
    /// Whether the gate's tools ran under the separate uid (`AET-WO-1601`
    /// §1.2), as the gate's own verdict says; null when it did not run, and
    /// for `extract`, which runs no tool. For the campaign, true only when
    /// every run says so.
    uid_boundary: Option<bool>,
    /// When `uid_boundary` is false, the first condition that failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    uid_boundary_unmet: Option<String>,
    /// The sha256 the gate reports for what it judged, equal to the generated one.
    input_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    harness_provenance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cover_count: Option<u64>,
    findings: Vec<Value>,
}

#[derive(Serialize)]
struct Run {
    scenario: String,
    verdict: String,
    expect: String,
    matched: bool,
    findings: Value,
    run_id: String,
    state_sequence_hash: String,
}

#[derive(Serialize)]
struct Finding {
    kind: &'static str,
    severity: &'static str,
    message: String,
}

#[derive(Serialize)]
struct Refusal {
    ok: bool,
    error: RefusalBody,
}

#[derive(Serialize)]
struct RefusalBody {
    kind: String,
    id: Option<String>,
    message: String,
}

fn refuse(kind: &str, id: Option<&str>, message: String, code: u8) -> ExitCode {
    crate::emit(
        &Refusal {
            ok: false,
            error: RefusalBody {
                kind: kind.to_owned(),
                id: id.map(str::to_owned),
                message,
            },
        },
        code,
    )
}

fn mismatch(gate: &str, what: &str, want: &str, have: Option<&str>) -> ExitCode {
    refuse(
        "BindingMismatch",
        Some(gate),
        format!(
            "gate `{gate}` judged {what} with sha256 {}, not the generated {want}",
            have.unwrap_or("(none reported)")
        ),
        EXIT_TOOL_ERROR,
    )
}

/// One gate's output: its exit code and every stdout line as JSON.
struct Ran {
    code: i32,
    lines: Vec<Value>,
}

impl Ran {
    fn last(&self) -> &Value {
        self.lines.last().unwrap_or(&Value::Null)
    }
}

/// Runs this binary's own subcommand `args` and reads its lines. A gate that
/// refuses (any code but 0 or 1) stops `verify` with the gate's own refusal
/// and exit code (`INV-06`).
fn gate(name: &str, args: Vec<OsString>) -> Result<Ran, ExitCode> {
    let exe = std::env::current_exe().map_err(|e| {
        refuse(
            "ToolError",
            Some("aethelgard"),
            format!("cannot find this binary to run gate `{name}`: {e}"),
            EXIT_TOOL_ERROR,
        )
    })?;
    // The gate dies with this process (`run::PARENT_PID_VAR`), and its tool
    // with it, through the gate's own reaper.
    let out = Command::new(exe)
        .args(&args)
        .env(run::PARENT_PID_VAR, std::process::id().to_string())
        .stdin(Stdio::null())
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| {
            refuse(
                "ToolError",
                Some("aethelgard"),
                format!("cannot run gate `{name}`: {e}"),
                EXIT_TOOL_ERROR,
            )
        })?;
    let Some(code) = out.status.code() else {
        return Err(refuse(
            "ToolError",
            Some("aethelgard"),
            format!("gate `{name}` was killed by a signal"),
            EXIT_TOOL_ERROR,
        ));
    };
    let mut lines = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let value: Value = serde_json::from_str(line).map_err(|e| {
            refuse(
                "ToolError",
                Some("aethelgard"),
                format!("gate `{name}` printed a line that is not JSON: {e}"),
                EXIT_TOOL_ERROR,
            )
        })?;
        lines.push(value);
    }
    let ran = Ran { code, lines };
    match code {
        0 | 1 if !ran.lines.is_empty() => Ok(ran),
        2..=5 | 64 => {
            let error = &ran.last()["error"];
            let kind = error["kind"].as_str().unwrap_or("ToolError");
            let message = error["message"].as_str().unwrap_or("no message");
            Err(refuse(
                kind,
                error["id"].as_str(),
                format!("gate `{name}`: {message}"),
                u8::try_from(code).unwrap_or(EXIT_TOOL_ERROR),
            ))
        }
        EXIT_INTERNAL_I32 => {
            crate::diagnose(&format!("gate `{name}` exited 70"));
            Err(ExitCode::from(EXIT_INTERNAL))
        }
        other => Err(refuse(
            "ToolError",
            Some("aethelgard"),
            format!("gate `{name}` exited {other} with no verdict"),
            EXIT_TOOL_ERROR,
        )),
    }
}

const EXIT_INTERNAL_I32: i32 = EXIT_INTERNAL as i32;

/// A gate's report for the verdict, from its one verdict line.
fn gate_report(name: &'static str, harness: Option<String>, ran: &Ran) -> Gate {
    let v = ran.last();
    Gate {
        gate: name,
        harness,
        verdict: if ran.code == 0 && v["verdict"] == "pass" {
            "pass"
        } else {
            "fail"
        },
        exit_code: Some(ran.code),
        tool_version: v["tool"]["version"].as_str().map(str::to_owned),
        uid_boundary: Some(v["sandbox"]["uid_boundary"] == true),
        uid_boundary_unmet: v["sandbox"]["uid_boundary_unmet"]
            .as_str()
            .map(str::to_owned),
        input_sha256: v["input"]["sha256"].as_str().map(str::to_owned),
        harness_provenance: v["harness_provenance"].as_str().map(str::to_owned),
        cover_count: v["cover_count"].as_u64(),
        findings: v["findings"].as_array().cloned().unwrap_or_default(),
    }
}

/// The campaign's `uid_boundary`: true only when every run line says `true`.
fn every_run_separated(runs: &[Value]) -> bool {
    runs.iter().all(|line| line["uid_boundary"] == true)
}

fn not_run(name: &'static str) -> Gate {
    Gate {
        gate: name,
        harness: None,
        verdict: "not_run",
        exit_code: None,
        tool_version: None,
        uid_boundary: None,
        uid_boundary_unmet: None,
        input_sha256: None,
        harness_provenance: None,
        cover_count: None,
        findings: Vec::new(),
    }
}

/// What one gate must report about what it judged, as (what, JSON pointer,
/// value): its binding to the generated bytes. `elf` is the ELF `build`
/// reported; `harness` the one `prove` was asked for.
fn expectations(
    gate: &str,
    w: &Want<'_>,
    elf: &str,
    harness: &str,
) -> Vec<(&'static str, &'static str, String)> {
    let e = |what, pointer, want: &str| (what, pointer, want.to_owned());
    match gate {
        "erc" => vec![e("the schematic", "/input/sha256", w.sch)],
        "drc" => vec![
            e("the board", "/input/sha256", w.pcb),
            e("the parity schematic", "/schematic/sha256", w.sch),
            e("the specification", "/specification_sha256", w.spec),
        ],
        "extract" => vec![
            e("the board", "/input/sha256", w.pcb),
            e("the specification", "/specification_sha256", w.spec),
        ],
        "prove" => vec![
            e("the board crate", "/input/sha256", w.board),
            e("the harness", "/harness", harness),
            e("a harness it calls", "/harness_provenance", "generated"),
        ],
        "build" => vec![e("the board crate", "/input/sha256", w.board)],
        "simulate" => vec![
            e("the platform", "/input/sha256", w.repl),
            e("the firmware", "/boot/firmware/sha256", elf),
            e("the specification", "/boot/specification_sha256", w.spec),
        ],
        _ => Vec::new(),
    }
}

/// The first expectation `v` does not meet, as (what, wanted, reported).
fn unbound(
    gate: &str,
    v: &Value,
    w: &Want<'_>,
    elf: &str,
    harness: &str,
) -> Option<(&'static str, String, Option<String>)> {
    expectations(gate, w, elf, harness)
        .into_iter()
        .find_map(|(what, pointer, want)| {
            let have = v.pointer(pointer).and_then(Value::as_str);
            (have != Some(want.as_str())).then(|| (what, want.clone(), have.map(str::to_owned)))
        })
}

/// Refuses (`BindingMismatch`) unless `ran`'s verdict meets every
/// expectation of `gate`.
fn bound(gate: &str, ran: &Ran, w: &Want<'_>, elf: &str, harness: &str) -> Result<(), ExitCode> {
    match unbound(gate, ran.last(), w, elf, harness) {
        None => Ok(()),
        Some((what, want, have)) => Err(mismatch(gate, what, &want, have.as_deref())),
    }
}

/// Requires `have` (a field the gate reported) to be `want`.
fn bind(gate: &str, what: &str, want: &str, have: &Value) -> Result<(), ExitCode> {
    if have.as_str() == Some(want) {
        Ok(())
    } else {
        Err(mismatch(gate, what, want, have.as_str()))
    }
}

/// The whole-schedule harness, which does not finish on a board with motor
/// outputs (`AET-WO-4300a` FEEDBACK Q5).
const WHOLE_SCHEDULE: &str = "proofs::every_valid_sequence";

/// The harnesses to prove, and whether [`WHOLE_SCHEDULE`] was left out. Only
/// on a motor board and only that exact harness is dropped; the rest run.
fn harness_plan(all: Vec<String>, motor_board: bool) -> (Vec<String>, bool) {
    if motor_board && all.iter().any(|h| h == WHOLE_SCHEDULE) {
        (
            all.into_iter().filter(|h| h != WHOLE_SCHEDULE).collect(),
            true,
        )
    } else {
        (all, false)
    }
}

/// The info finding that says which harnesses ran and that
/// [`WHOLE_SCHEDULE`] did not.
fn skipped_finding(ran: &[String]) -> Finding {
    Finding {
        kind: "whole_schedule_harness_not_run",
        severity: "info",
        message: format!(
            "harnesses run: {}; {WHOLE_SCHEDULE} did not run: a motor board: the whole-schedule harness does not finish (AET-WO-4300a FEEDBACK Q5); the targeted harnesses prove the timer start and the clock check; the split is AET-WO-4500's backlog",
            ran.join(", ")
        ),
    }
}

/// Every declared bus and rail the campaign seeds no fault on (`INV-03`,
/// `AET-WO-3303` §1.1), each named with why, in the specification's order:
/// I2C buses, SPI buses, then rails. A fault counts only where it can change
/// what the campaign judges: on a bus some device uses, with a value other
/// than the specification's own (a pull-up or a line's capacitance limit), a
/// held line, an SPI device left absent, or a droop whose floor is below its
/// domain's nominal. A rail is each voltage domain, and the battery when the
/// specification declares a power tree. The campaign droops only the MCU's
/// supply (`DroopNotMcuSupply`) and nothing names an SPI bus but through a
/// device, so such a rail or bus is named as one no fault kind reaches.
pub(crate) fn uncovered(
    spec: &aethelgard_ir::Specification,
    topology: &aethelgard_ir::SolvedTopology,
    scenarios: &[aethelgard_chaos::campaign::Scenario],
) -> Vec<String> {
    use aethelgard_chaos::record::Override;
    use std::collections::BTreeSet;
    let mut i2c = BTreeSet::new();
    let mut spi = BTreeSet::new();
    let mut rails = BTreeSet::new();
    for o in scenarios.iter().flat_map(|s| &s.overrides) {
        match o {
            Override::PullUpOhms { bus, ohms } => {
                if spec
                    .i2c_buses
                    .iter()
                    .any(|b| &b.id == bus && b.pull_up_ohms != *ohms)
                {
                    i2c.insert(bus.as_str());
                }
            }
            Override::MaxCapacitance { net, femtofarads } => {
                let changed = spec
                    .net_constraints
                    .iter()
                    .any(|c| &c.net_name == net && c.max_capacitance_femtofarads != *femtofarads);
                if let Some(b) = spec
                    .i2c_buses
                    .iter()
                    .find(|b| changed && (&b.sda.net == net || &b.scl.net == net))
                {
                    i2c.insert(b.id.as_str());
                }
            }
            Override::LineHeldLow { bus, .. } => {
                i2c.insert(bus.as_str());
            }
            Override::Droop(d) => {
                if spec
                    .voltage_domains
                    .iter()
                    .any(|v| v.id == d.domain && d.floor_millivolts < v.nominal_millivolts)
                {
                    rails.insert(d.domain.as_str());
                }
            }
            Override::SpiDeviceAbsent { reference } => {
                if let Some(d) = topology.spi_devices.get(reference) {
                    spi.insert(d.bus.as_str());
                }
            }
        }
    }
    let mut out = Vec::new();
    let unused = "no device uses it, so a fault there judges nothing";
    for b in &spec.i2c_buses {
        if !topology.i2c_devices.values().any(|d| d.bus == b.id) {
            out.push(format!("I2C bus `{}` ({unused})", b.id));
        } else if !i2c.contains(b.id.as_str()) {
            out.push(format!("I2C bus `{}`", b.id));
        }
    }
    for b in &spec.spi_buses {
        if !topology.spi_devices.values().any(|d| d.bus == b.id) {
            out.push(format!(
                "SPI bus `{}` ({unused}; no fault kind names an SPI bus but through a device)",
                b.id
            ));
        } else if !spi.contains(b.id.as_str()) {
            out.push(format!("SPI bus `{}`", b.id));
        }
    }
    for v in &spec.voltage_domains {
        if v.id != spec.mcu.supply {
            out.push(format!(
                "rail `{}` (not the MCU's supply: no fault kind droops it)",
                v.id
            ));
        } else if !rails.contains(v.id.as_str()) {
            out.push(format!("rail `{}`", v.id));
        }
    }
    if spec.power.is_some() {
        out.push("rail `battery` (no fault kind droops it)".to_owned());
    }
    out
}

/// Each harness the board crate declares that `planned` does not name: every
/// `kani::proof` attribute in every file of the crate is counted, however it
/// is written, so a harness the line reader in [`harnesses`] misses is never
/// silently left unproved (`AET-WO-3303` §1.1).
fn unplanned(board: &BTreeMap<String, String>, planned: usize) -> Option<String> {
    let declared: usize = board
        .values()
        .map(|text| {
            text.match_indices("kani::proof")
                .filter(|(i, m)| {
                    !text[i + m.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_')
                })
                .count()
        })
        .sum();
    (declared != planned).then(|| {
        format!(
            "the board crate declares {declared} Kani harnesses and the plan names {planned}: a harness the plan misses would not be proved"
        )
    })
}

/// The harnesses of a generated `src/proofs.rs`, as `proofs::<name>`: each
/// `fn` after a `#[kani::proof]`. The text is the generator's, not a user's.
pub fn harnesses(proofs: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut armed = false;
    for line in proofs.lines().map(str::trim) {
        if line == "#[kani::proof]" {
            armed = true;
        } else if armed && let Some(rest) = line.strip_prefix("fn ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            out.push(format!("proofs::{name}"));
            armed = false;
        }
    }
    out
}

fn os(p: &Path) -> OsString {
    p.as_os_str().to_owned()
}

/// What each scenario's run must be named: its file's hash, its effective
/// specification's hash, with `--spec` as its base.
struct Expected {
    path: PathBuf,
    name: String,
    scenario: [u8; 32],
    specification: [u8; 32],
}

/// `aethelgard verify --spec <file> [--scenarios <file>...] [--against <dir>]`.
pub fn verify(
    spec_path: &Path,
    scenarios: &[PathBuf],
    against: Option<&Path>,
    fab_mode: Option<&FabMode>,
) -> ExitCode {
    let bytes = match crate::read_input(Some(spec_path)) {
        Ok(b) => b,
        Err(r) => return crate::emit(&r, EXIT_UNREADABLE),
    };
    let (spec, spec_sha256) = match crate::parse_spec(&bytes) {
        Ok(s) => s,
        Err((r, code)) => return crate::emit(&r, code),
    };
    let (topology, table) = match crate::solve_spec(&spec, &spec_sha256) {
        Ok(s) => s,
        Err(code) => return code,
    };
    let lifecycle_unheld = match fab_mode {
        Some(_) => lifecycle_unheld(&topology),
        None => Vec::new(),
    };
    let (files, topology_sha256) = match crate::generated(&spec, &topology, &table) {
        Ok(generated) => generated,
        Err(code) => return code,
    };
    let hashes: BTreeMap<String, String> = files
        .iter()
        .map(|(p, t)| (p.clone(), tree::sha256(t.as_bytes())))
        .collect();
    let project = spec.project_name.clone();
    let mut report = Report {
        ok: false,
        verdict: "fail",
        check: "verify",
        engine_version: env!("CARGO_PKG_VERSION"),
        project_name: project.clone(),
        specification_sha256: spec_sha256.clone(),
        topology_sha256,
        files: hashes.clone(),
        lifecycle: topology.lifecycle_banner().unwrap_or_default(),
        against: None,
        elf_sha256: None,
        rise_times: RISE_TIMES,
        gates: Vec::new(),
        campaign: None,
        findings: Vec::new(),
        against_fab: None,
        fab: None,
        lifecycle_unheld,
        conditions: Vec::new(),
    };

    // Every scenario must start from exactly `--spec`, or its campaign would
    // judge another specification.
    let mut expected = Vec::new();
    let mut read = Vec::new();
    for path in scenarios {
        let scenario = match campaign::read_scenario(path) {
            Ok(s) => s,
            Err(code) => return code,
        };
        let base = match campaign::read_base(path, &scenario.base) {
            Ok(b) => b,
            Err((kind, m)) => return refuse(kind, Some(&scenario.name), m, EXIT_UNREADABLE),
        };
        if tree::sha256(&base) != spec_sha256 {
            return refuse(
                "ScenarioBaseMismatch",
                Some(&scenario.name),
                format!(
                    "{} starts from a base whose sha256 is {}, not --spec's {spec_sha256}",
                    path.display(),
                    tree::sha256(&base)
                ),
                EXIT_UNREADABLE,
            );
        }
        let effective = match campaign::effective(&scenario, &bytes) {
            Ok(e) => e,
            Err(code) => return code,
        };
        expected.push(Expected {
            path: path.clone(),
            name: scenario.name.clone(),
            scenario: scenario.sha256,
            specification: effective.specification_sha256,
        });
        read.push(scenario);
    }

    // An unusable `--fab-out` or `--against-fab` is refused before any gate.
    let fab_checked = match fab_mode {
        Some(FabMode::Write(dir)) => fab_out_usable(dir),
        Some(FabMode::Compare(dir)) => crate::input::dir_check(dir),
        None => Ok(()),
    };
    if let Err(s) = fab_checked {
        return crate::stopped(s);
    }

    let board = tree::subtree(&files, &format!("{BOARD_DIR}/"));
    let board_sha256 = tree::crate_sha256(&board);
    let (harnesses, skipped) = harness_plan(
        harnesses(board.get("src/proofs.rs").map_or("", String::as_str)),
        !topology.motor_outputs.is_empty(),
    );
    // A skipped harness is refused by name before any gate runs (`AET-WO-3303`
    // §1.1); the other preconditions are judged on the gates.
    if fab_mode.is_some() {
        let left_out = if skipped {
            Some(format!(
                "{WHOLE_SCHEDULE} is not proved on a board with motor outputs"
            ))
        } else {
            unplanned(&board, harnesses.len())
        };
        if let Some(m) = left_out {
            return refuse(
                "HarnessSkipped",
                None,
                format!("no package: {m}"),
                EXIT_UNREADABLE,
            );
        }
    }
    // Coverage is judged and stated in the package's preconditions, not yet
    // enforced: no fault kind a design survives reaches an SPI bus, so no
    // board with one could be packaged (`AET-WO-3303` FEEDBACK D1, open).
    let short = match fab_mode {
        Some(_) => uncovered(&spec, &topology, &read),
        None => Vec::new(),
    };

    // `--against` first: a directory that is not the tree fails, naming
    // every difference, and no gate runs; with a fab flag, nothing is
    // written or compared, and the verdict says so.
    if let Some(dir) = against {
        if let Err(s) = crate::input::dir_check(dir) {
            return crate::stopped(s);
        }
        let differences = match tree::diff(dir, &files, &[&format!("{BOARD_DIR}/target")]) {
            Ok(d) => d,
            Err(s) => return crate::stopped(s),
        };
        if !differences.is_empty() {
            report.against = Some("differs");
            report.findings = differences
                .iter()
                .map(|d| Finding {
                    kind: d.kind(),
                    severity: "error",
                    message: describe(d),
                })
                .collect();
            if let Some(mode) = fab_mode {
                not_packaged(&mut report, mode);
            }
            return crate::emit(&report, EXIT_FAIL);
        }
        report.against = Some("identical");
    }

    let scratch = match crate::Scratch::new() {
        Ok(s) => s,
        Err(s) => return crate::stopped(s),
    };
    let root = scratch.tool.join("tree");
    if let Err(s) = tree::write(&root, &files) {
        return crate::stopped(s);
    }
    // `parse_spec` accepted the bytes, so they are UTF-8.
    let spec_text = String::from_utf8_lossy(&bytes).into_owned();
    let spec_dir = scratch.tool.join("spec");
    if let Err(s) = tree::write(
        &spec_dir,
        &BTreeMap::from([("specification.json".to_owned(), spec_text)]),
    ) {
        return crate::stopped(s);
    }
    let spec_copy = spec_dir.join("specification.json");

    let sch = format!("{KICAD_DIR}/{project}.kicad_sch");
    let pcb = format!("{KICAD_DIR}/{project}.kicad_pcb");
    let repl = format!("{BOARD_DIR}/{project}.repl");
    let path = |rel: &str| os(&root.join(rel));
    let spec_arg = os(&spec_copy);

    if let Err(code) = run_gates(
        &mut report,
        &Paths {
            sch: path(&sch),
            pcb: path(&pcb),
            repl: path(&repl),
            board: path(BOARD_DIR),
            spec: spec_arg,
            elf: os(&scratch.tool.join("firmware.elf")),
            archive: os(&scratch.tool.join("archive")),
        },
        &Want {
            sch: &hashes[&sch],
            pcb: &hashes[&pcb],
            repl: &hashes[&repl],
            board: &board_sha256,
            spec: &spec_sha256,
            harnesses: harnesses.clone(),
            skipped,
            boot_time: format!("{}ns", aethelgard_twin::boot::window(&topology).0),
        },
        &expected,
    ) {
        return code;
    }
    report.ok = report.findings.iter().all(|f| f.severity != "error")
        && report.gates.iter().all(|g| g.verdict == "pass");
    report.verdict = if report.ok { "pass" } else { "fail" };
    if let Some(mode) = fab_mode {
        let elf_sha256 = report.elf_sha256.clone().unwrap_or_default();
        let lifecycle: Vec<String> = report
            .lifecycle
            .iter()
            .chain(&report.lifecycle_unheld)
            .cloned()
            .collect();
        let conditions = std::mem::take(&mut report.conditions);
        let bound = fab::Bound {
            project: &project,
            spec: &spec,
            specification_sha256: &spec_sha256,
            board: &root.join(&pcb),
            board_sha256: &hashes[&pcb],
            elf: &scratch.tool.join("firmware.elf"),
            elf_sha256: &elf_sha256,
            bom: files[&format!("{KICAD_DIR}/{project}.bom.csv")].as_bytes(),
            conditions: &conditions,
            lifecycle: &lifecycle,
        };
        let scenarios: Vec<fab::Scenario> = expected
            .iter()
            .map(|e| fab::Scenario {
                scenario: e.name.clone(),
                sha256: hex(e.scenario),
            })
            .collect();
        let pre = preconditions(&report, &harnesses, short);
        if let Err(code) = package(&mut report, mode, &bound, &scenarios) {
            return code;
        }
        if let Some(f) = report.fab.as_mut() {
            f.preconditions = pre;
        }
    }
    let code = if report.ok { 0 } else { EXIT_FAIL };
    crate::emit(&report, code)
}

/// A run that failed writes and compares nothing, and says so.
fn not_packaged(report: &mut Report, mode: &FabMode) {
    let (status, against) = match mode {
        FabMode::Write(_) => ("not_written", None),
        FabMode::Compare(_) => ("not_derived", Some("not_compared")),
    };
    report.fab = Some(FabReport::none(status, report.lifecycle_unheld.clone()));
    report.against_fab = against;
}

/// The package, once every gate has passed: derived, named in `report.fab`,
/// then written into `--fab-out` (the manifest last) or compared with
/// `--against-fab`. A failed run writes nothing and says so. A refusal
/// writes nothing and prints no verdict.
fn package(
    report: &mut Report,
    mode: &FabMode,
    bound: &fab::Bound<'_>,
    scenarios: &[fab::Scenario],
) -> Result<(), ExitCode> {
    if !report.ok {
        not_packaged(report, mode);
        return Ok(());
    }
    let exported = fab::derive(bound).map_err(crate::stopped)?;
    // The verdict as a call without the fab flags prints it.
    let line = serde_json::to_string(&*report).map_err(|e| {
        crate::diagnose(&format!("cannot serialize the verdict: {e}"));
        ExitCode::from(EXIT_INTERNAL)
    })?;
    let manifest = fab::manifest(
        &fab::Named {
            project: &report.project_name,
            specification_sha256: &report.specification_sha256,
            topology_sha256: &report.topology_sha256,
            scenarios,
            verdict_line: &line,
        },
        &exported.files,
    );
    let mut files: BTreeMap<String, String> = exported
        .files
        .iter()
        .map(|(name, bytes)| (name.clone(), tree::sha256(bytes)))
        .collect();
    files.insert(fab::MANIFEST.to_owned(), tree::sha256(&manifest));
    let mut fab_report = FabReport {
        status: "derived",
        preconditions: Vec::new(),
        tool_version: Some(exported.tool_version.clone()),
        uid_boundary: Some(exported.sandbox.uid_boundary),
        uid_boundary_unmet: exported.sandbox.uid_boundary_unmet,
        board_sha256: Some(bound.board_sha256.to_owned()),
        files,
        differences: Vec::new(),
        lifecycle_unheld: report.lifecycle_unheld.clone(),
    };
    match mode {
        FabMode::Write(dir) => {
            fab_out_usable(dir).map_err(crate::stopped)?;
            let ordered = exported
                .files
                .iter()
                .map(|(n, b)| (n.as_str(), b.as_slice()))
                .chain([(fab::MANIFEST, manifest.as_slice())]);
            tree::write(dir, ordered).map_err(|mut s| {
                s.message = format!(
                    "{}; {} may hold part of the package, and no verdict names it",
                    s.message,
                    dir.display()
                );
                crate::stopped(s)
            })?;
            fab_report.status = "written";
        }
        FabMode::Compare(dir) => {
            let mut package = exported.files;
            package.insert(fab::MANIFEST.to_owned(), manifest);
            let differences = tree::diff(dir, &package, &[]).map_err(crate::stopped)?;
            report.against_fab = Some(if differences.is_empty() {
                "identical"
            } else {
                "differs"
            });
            if !differences.is_empty() {
                report.ok = false;
                report.verdict = "fail";
            }
            fab_report.differences = differences
                .iter()
                .map(|d| Finding {
                    kind: d.kind(),
                    severity: "error",
                    message: describe(d).replace("generated", "derived"),
                })
                .collect();
        }
    }
    report.fab = Some(fab_report);
    Ok(())
}

fn describe(d: &Difference) -> String {
    match d {
        Difference::Missing(p) => format!("{p} is generated but missing from the directory"),
        Difference::Differs(p) => format!("{p} differs from the generated file"),
        Difference::Extra(p) => format!("{p} is in the directory but is not generated"),
    }
}

struct Paths {
    sch: OsString,
    pcb: OsString,
    repl: OsString,
    board: OsString,
    spec: OsString,
    elf: OsString,
    archive: OsString,
}

struct Want<'a> {
    sch: &'a str,
    pcb: &'a str,
    repl: &'a str,
    board: &'a str,
    spec: &'a str,
    harnesses: Vec<String>,
    /// Whether the whole-schedule harness is left out (a motor board).
    skipped: bool,
    /// The `--for` `simulate` boots for: the board's window, as the
    /// campaign's (`AET-WO-4300a` D1).
    boot_time: String,
}

fn args(items: &[&dyn AsRef<std::ffi::OsStr>]) -> Vec<OsString> {
    items.iter().map(|i| (**i).as_ref().to_owned()).collect()
}

/// ERC, DRC with parity, `extract`, `prove` of every harness, `build`,
/// `simulate --firmware` and the campaign, in that order, each bound to the
/// generated hashes. A refusal or a binding mismatch stops the run.
fn run_gates(
    report: &mut Report,
    p: &Paths,
    want: &Want<'_>,
    expected: &[Expected],
) -> Result<(), ExitCode> {
    let erc = gate("erc", args(&[&"check", &"erc", &"--schematic", &p.sch]))?;
    bound("erc", &erc, want, "", "")?;
    report.gates.push(gate_report("erc", None, &erc));

    let drc = gate(
        "drc",
        args(&[
            &"check",
            &"drc",
            &"--board",
            &p.pcb,
            &"--spec",
            &p.spec,
            &"--schematic",
            &p.sch,
        ]),
    )?;
    bound("drc", &drc, want, "", "")?;
    report.gates.push(gate_report("drc", None, &drc));

    let extract = gate(
        "extract",
        args(&[&"extract", &"--board", &p.pcb, &"--spec", &p.spec]),
    )?;
    bound("extract", &extract, want, "", "")?;
    // `extract` runs no external tool, so its verdict has no `sandbox` and no
    // uid boundary to report: null, never a value it did not measure.
    report.conditions = extract.last()["conditions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| c.as_str().map(str::to_owned))
        .collect();
    let mut g = gate_report("extract", None, &extract);
    g.uid_boundary = None;
    report.gates.push(g);

    if want.harnesses.is_empty() {
        report.findings.push(Finding {
            kind: "no_harness",
            severity: "error",
            message: "the generated board crate has no Kani harness".to_owned(),
        });
    }
    if want.skipped {
        report.findings.push(skipped_finding(&want.harnesses));
    }
    for harness in &want.harnesses {
        let h = OsString::from(harness);
        let ran = gate(
            "prove",
            args(&[
                &"prove",
                &"--crate",
                &p.board,
                &"--harness",
                &h,
                &"--spec",
                &p.spec,
            ]),
        )?;
        bound("prove", &ran, want, "", harness)?;
        report
            .gates
            .push(gate_report("prove", Some(harness.clone()), &ran));
    }

    let built = gate(
        "build",
        args(&[&"build", &"--crate", &p.board, &"--out", &p.elf]),
    )?;
    bound("build", &built, want, "", "")?;
    let elf = built.last()["elf"]["sha256"].as_str().map(str::to_owned);
    report.gates.push(gate_report("build", None, &built));
    let Some(elf) = elf.filter(|_| built.code == 0) else {
        report.gates.push(not_run("simulate"));
        if !expected.is_empty() {
            report.gates.push(not_run("campaign"));
        }
        return Ok(());
    };
    report.elf_sha256 = Some(elf.clone());

    let sim = gate(
        "simulate",
        args(&[
            &"simulate",
            &"--platform",
            &p.repl,
            &"--firmware",
            &p.elf,
            &"--spec",
            &p.spec,
            &"--for",
            &want.boot_time,
        ]),
    )?;
    bound("simulate", &sim, want, &elf, "")?;
    report.gates.push(gate_report("simulate", None, &sim));

    if expected.is_empty() {
        return Ok(());
    }
    let mut a = vec![OsString::from("campaign"), OsString::from("--scenarios")];
    a.extend(expected.iter().map(|e| e.path.clone().into_os_string()));
    a.extend([
        OsString::from("--firmware"),
        p.elf.clone(),
        OsString::from("--archive"),
        p.archive.clone(),
    ]);
    let ran = gate("campaign", a)?;
    let firmware = campaign::sha(&elf).map_err(|m| {
        refuse(
            "ToolError",
            Some("aethelgard"),
            format!("gate `build` reported a bad hash: {m}"),
            EXIT_TOOL_ERROR,
        )
    })?;
    let runs = &ran.lines[..ran.lines.len().saturating_sub(1)];
    if runs.len() != expected.len() {
        return Err(refuse(
            "ToolError",
            Some("aethelgard"),
            format!(
                "gate `campaign` reported {} runs for {} scenarios",
                runs.len(),
                expected.len()
            ),
            EXIT_TOOL_ERROR,
        ));
    }
    let mut out = Vec::new();
    for (line, e) in runs.iter().zip(expected) {
        let id = hex(aethelgard_chaos::campaign::run_id(
            &e.scenario,
            &e.specification,
            &firmware,
        ));
        bind(
            "campaign",
            &format!("the run of {}", e.path.display()),
            &id,
            &line["run_id"],
        )?;
        let text = |k: &str| line[k].as_str().unwrap_or_default().to_owned();
        out.push(Run {
            scenario: text("scenario"),
            verdict: text("verdict"),
            expect: text("expect"),
            matched: line["matched"] == true,
            findings: line["findings"].clone(),
            run_id: id,
            state_sequence_hash: text("state_sequence_hash"),
        });
    }
    let mut g = gate_report("campaign", None, &ran);
    g.uid_boundary = Some(every_run_separated(runs));
    g.uid_boundary_unmet = runs
        .iter()
        .find_map(|line| line["uid_boundary_unmet"].as_str().map(str::to_owned));
    // The design must survive every scenario: each run's own verdict is
    // `pass`, and the campaign matched every expectation. A scenario that
    // expects a fail can never let `verify` pass (`AET-WO-3301` §3.2).
    let survived = out.iter().all(|r| r.verdict == "pass" && r.matched);
    g.verdict = if ran.code == 0 && ran.last()["ok"] == true && survived {
        "pass"
    } else {
        "fail"
    };
    g.findings = out
        .iter()
        .filter(|r| r.verdict != "pass" || !r.matched)
        .flat_map(|r| {
            let kinds: Vec<String> = r
                .findings
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|f| f.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default();
            let kinds = if kinds.is_empty() {
                vec!["expectation_unmatched".to_owned()]
            } else {
                kinds
            };
            kinds.into_iter().map(move |kind| {
                serde_json::json!({
                    "kind": kind,
                    "severity": "error",
                    "message": format!(
                        "scenario `{}`: verdict {}, expected {}",
                        r.scenario, r.verdict, r.expect
                    ),
                })
            })
        })
        .collect();
    report.gates.push(g);
    report.campaign = Some(out);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// `AET-WO-3303` §1.1: every `kani::proof` attribute in the crate is
    /// counted, so a harness the line reader misses (its attribute and `fn`
    /// on one line, or in another file) refuses the package.
    #[test]
    fn a_harness_the_plan_misses_is_counted() {
        let proofs = "#[kani::proof]\nfn a() {}\n#[kani::proof] fn b() {}\n";
        let crate_ = BTreeMap::from([("src/proofs.rs".to_owned(), proofs.to_owned())]);
        assert_eq!(harnesses(proofs), ["proofs::a"]);
        assert!(unplanned(&crate_, 1).is_some());
        assert!(unplanned(&crate_, 2).is_none());
        let other = BTreeMap::from([
            (
                "src/proofs.rs".to_owned(),
                "#[kani::proof]\nfn a() {}\n".to_owned(),
            ),
            (
                "src/lib.rs".to_owned(),
                "#[kani::proof]\nfn c() {}\n".to_owned(),
            ),
        ]);
        assert!(unplanned(&other, 1).is_some());
        // A longer path that only starts with the attribute's is not one.
        let contract = BTreeMap::from([(
            "src/proofs.rs".to_owned(),
            "#[kani::proof]\nfn a() {}\n#[kani::proof_for_contract(f)]\n".to_owned(),
        )]);
        assert!(unplanned(&contract, 1).is_none());
    }

    fn solved(path: &str) -> (aethelgard_ir::Specification, aethelgard_ir::SolvedTopology) {
        let bytes = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(path),
        )
        .unwrap();
        let spec: aethelgard_ir::Specification = serde_json::from_slice(&bytes).unwrap();
        let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
        let topology = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
        (spec, topology)
    }

    fn scenario_file(path: &str) -> aethelgard_chaos::campaign::Scenario {
        aethelgard_chaos::campaign::Scenario::parse(
            &std::fs::read(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../..")
                    .join(path),
            )
            .unwrap(),
        )
        .unwrap()
    }

    /// `AET-WO-3303` §1.1: each declared bus and rail is named until a
    /// scenario seeds a fault on it. The reference's survivable faults
    /// (`pullup-3k3`, `droop-tolerated`) cover `SENSORS` and `VDD_3V3`;
    /// `FLASH` is covered only by `spi-absent-u4`, which no design survives
    /// (3301 §3.2), so a passing package always names it (FEEDBACK D1).
    #[test]
    fn each_bus_and_rail_is_named_until_a_fault_covers_it() {
        let (spec, topology) = solved("examples/sentinel-01-minimal.json");
        let named = |files: &[&str]| {
            let read: Vec<_> = files.iter().map(|f| scenario_file(f)).collect();
            uncovered(&spec, &topology, &read)
        };
        let all = ["I2C bus `SENSORS`", "SPI bus `FLASH`", "rail `VDD_3V3`"];
        assert_eq!(named(&["scenarios/sentinel-01/pullup-2k2.json"]), all);
        assert_eq!(
            named(&[
                "scenarios/sentinel-01/seeded/pullup-3k3.json",
                "scenarios/sentinel-01/droop-tolerated.json"
            ]),
            ["SPI bus `FLASH`"]
        );
        assert_eq!(
            named(&[
                "scenarios/sentinel-01/pullup-47k.json",
                "scenarios/sentinel-01/seeded/spi-absent-u4.json",
                "scenarios/sentinel-01/droop-bor3.json"
            ]),
            Vec::<String>::new()
        );
    }

    /// `AET-WO-3303` §7 (D1): a rail other than the MCU's supply and an SPI
    /// bus no device uses are named whatever the campaign seeds, each with
    /// why: no override kind reaches them.
    #[test]
    fn a_rail_or_bus_no_fault_kind_reaches_is_named_with_why() {
        let (spec, topology) = solved("crates/aethelgard-cli/tests/fixtures/connectors.json");
        let faults = aethelgard_chaos::campaign::Scenario::parse(
            json!({"schema_version": 2, "name": "x", "base": "b.json", "expect": "fail",
                "expect_findings": ["i2c_timeout"], "overrides": [
                    {"bus": "SENSORS", "line_held_low": "scl"},
                    {"spi_device_absent": "U4"},
                    {"droop": {"domain": "VDD_3V3", "floor_millivolts": 3200, "start_ns": 1000000,
                        "fall_ns": 20000, "hold_ns": 200000, "rise_ns": 20000,
                        "source_milliohms": 500, "load_milliamps": 100, "bor_level": 0}}]})
            .to_string()
            .as_bytes(),
        )
        .unwrap();
        assert_eq!(
            uncovered(&spec, &topology, &[faults]),
            [
                "SPI bus `EXT` (no device uses it, so a fault there judges nothing; no fault kind names an SPI bus but through a device)",
                "rail `VDD_5V` (not the MCU's supply: no fault kind droops it)"
            ]
        );
    }

    /// `AET-WO-3303` §5.2: a fault equal to the no-fault case covers
    /// nothing: a capacitance limit at its declared value, a droop whose
    /// floor is the nominal; nor does a fault on a bus no device uses.
    #[test]
    fn a_fault_that_changes_nothing_covers_nothing() {
        let bytes = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sentinel-01-minimal.json"),
        )
        .unwrap();
        let spec: aethelgard_ir::Specification = serde_json::from_slice(&bytes).unwrap();
        let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
        let topology = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
        let sda = &spec.i2c_buses[0].sda.net;
        let declared = spec
            .net_constraints
            .iter()
            .find(|c| &c.net_name == sda)
            .unwrap()
            .max_capacitance_femtofarads;
        let scenario = |overrides: Value| {
            aethelgard_chaos::campaign::Scenario::parse(
                json!({"schema_version": 2, "name": "x", "base": "b.json", "expect": "pass",
                    "expect_findings": [], "overrides": overrides})
                .to_string()
                .as_bytes(),
            )
            .unwrap()
        };
        let droop = |floor: u32| {
            json!({"droop": {"domain": "VDD_3V3", "floor_millivolts": floor, "start_ns": 1000000,
                "fall_ns": 20000, "hold_ns": 200000, "rise_ns": 20000, "source_milliohms": 500,
                "load_milliamps": 100, "bor_level": 3}})
        };
        let short = |o: Value| uncovered(&spec, &topology, &[scenario(o)]);
        let all =
            short(json!([{"net": sda, "max_capacitance_femtofarads": declared}, droop(3300)]));
        assert_eq!(all.len(), 3, "{all:?}");
        let covered =
            short(json!([{"net": sda, "max_capacitance_femtofarads": declared + 1}, droop(3299)]));
        assert_eq!(covered, ["SPI bus `FLASH`"]);
    }

    /// `uid_boundary` is the gate's own `sandbox.uid_boundary`, and true only
    /// when that is the JSON `true`; a gate that did not run has none; the
    /// campaign's is true only when every run's is (`AET-WO-1601` §1.2).
    #[test]
    fn each_gate_names_the_uid_boundary_its_own_verdict_reports() {
        let ran = |v: Value| Ran {
            code: 0,
            lines: vec![v],
        };
        let boundary = |v: Value| gate_report("erc", None, &ran(v)).uid_boundary;
        assert_eq!(
            boundary(json!({"sandbox": {"uid_boundary": true}})),
            Some(true)
        );
        assert_eq!(
            boundary(json!({"sandbox": {"uid_boundary": false}})),
            Some(false)
        );
        assert_eq!(
            boundary(json!({"sandbox": {"uid_boundary": "true"}})),
            Some(false)
        );
        assert_eq!(boundary(json!({"sandbox": {}})), Some(false));
        assert_eq!(boundary(json!({"uid_boundary": true})), Some(false));
        assert_eq!(not_run("build").uid_boundary, None);
        let t = json!({"uid_boundary": true});
        assert!(every_run_separated(&[t.clone(), t.clone()]));
        assert!(!every_run_separated(&[
            t.clone(),
            json!({"uid_boundary": false})
        ]));
        assert!(!every_run_separated(&[t, json!({})]));
    }

    fn want() -> Want<'static> {
        Want {
            sch: "sch",
            pcb: "pcb",
            repl: "repl",
            board: "board",
            spec: "spec",
            harnesses: Vec::new(),
            skipped: false,
            boot_time: "100000000ns".to_owned(),
        }
    }

    /// Each gate's verdict, as the gate prints it, with every bound field
    /// at its generated value.
    fn verdicts() -> Vec<(&'static str, Value)> {
        vec![
            ("erc", json!({"input": {"sha256": "sch"}})),
            (
                "drc",
                json!({"input": {"sha256": "pcb"}, "schematic": {"sha256": "sch"}, "specification_sha256": "spec"}),
            ),
            (
                "extract",
                json!({"input": {"sha256": "pcb"}, "specification_sha256": "spec"}),
            ),
            (
                "prove",
                json!({"input": {"sha256": "board"}, "harness": "proofs::h", "harness_provenance": "generated"}),
            ),
            ("build", json!({"input": {"sha256": "board"}})),
            (
                "simulate",
                json!({"input": {"sha256": "repl"}, "boot": {"firmware": {"sha256": "elf"}, "specification_sha256": "spec"}}),
            ),
        ]
    }

    /// §5.2: a gate that reports judging anything but the generated bytes,
    /// in any one bound field, is refused; so is one that reports nothing.
    /// The firmware `simulate` reports it copied must be the ELF `build` gave
    /// (§3.3). This binds what each gate reports, not what its tool read
    /// after the copy (adversarial B4).
    #[test]
    fn every_bound_field_of_every_gate_is_checked() {
        let w = want();
        let mut fields = 0;
        for (gate, v) in verdicts() {
            assert_eq!(unbound(gate, &v, &w, "elf", "proofs::h"), None, "{gate}");
            assert!(
                unbound(gate, &json!({}), &w, "elf", "proofs::h").is_some(),
                "{gate}"
            );
            for (what, pointer, _) in expectations(gate, &w, "elf", "proofs::h") {
                let mut other = v.clone();
                *other.pointer_mut(pointer).unwrap() = json!("other");
                let (named, ..) = unbound(gate, &other, &w, "elf", "proofs::h").expect(pointer);
                assert_eq!(named, what, "{gate} {pointer}");
                fields += 1;
            }
        }
        assert_eq!(fields, 13);
        let firmware: Vec<_> = expectations("simulate", &w, "elf", "")
            .into_iter()
            .map(|(_, p, want)| (p, want))
            .collect();
        assert!(firmware.contains(&("/boot/firmware/sha256", "elf".to_owned())));
        assert!(
            expectations("prove", &w, "", "h")
                .iter()
                .any(|(_, p, want)| *p == "/harness_provenance" && want == "generated")
        );
    }

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn a_motor_board_drops_exactly_the_whole_schedule_harness_and_names_it() {
        let all = names(&[
            "proofs::every_valid_sequence",
            "proofs::tim1_start_writes_the_solved_values",
            "proofs::reset_clock_is_ok_exactly_at_reset",
        ]);
        let (ran, skipped) = harness_plan(all, true);
        assert_eq!(
            ran,
            [
                "proofs::tim1_start_writes_the_solved_values",
                "proofs::reset_clock_is_ok_exactly_at_reset"
            ]
        );
        assert!(skipped);
        let f = skipped_finding(&ran);
        assert_eq!(f.kind, "whole_schedule_harness_not_run");
        assert_eq!(f.severity, "info");
        assert!(
            f.message
                .contains("proofs::tim1_start_writes_the_solved_values")
        );
        assert!(
            f.message
                .contains("proofs::reset_clock_is_ok_exactly_at_reset")
        );
        assert!(
            f.message
                .contains("proofs::every_valid_sequence did not run")
        );
        assert!(f.message.contains("AET-WO-4500"));
    }

    #[test]
    fn a_board_without_motors_keeps_every_harness() {
        let all = names(&["proofs::every_valid_sequence", "proofs::i2c1_transfer"]);
        let (ran, skipped) = harness_plan(all.clone(), false);
        assert_eq!(ran, all);
        assert!(!skipped);
    }

    #[test]
    fn a_motor_board_without_the_whole_schedule_harness_names_nothing() {
        let all = names(&["proofs::tim1_start_writes_the_solved_values"]);
        let (ran, skipped) = harness_plan(all.clone(), true);
        assert_eq!(ran, all);
        assert!(!skipped);
    }

    #[test]
    fn harnesses_are_read_from_the_generated_proofs() {
        let text = "#[cfg(kani)]\n#[kani::proof]\n#[kani::unwind(4)]\nfn every_valid_sequence() {}\nfn helper() {}\n#[kani::proof]\nfn i2c1_transfer() {}\n";
        assert_eq!(
            harnesses(text),
            ["proofs::every_valid_sequence", "proofs::i2c1_transfer"]
        );
    }
}
