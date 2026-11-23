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
//!
//! **The witness ledger (`AET-WO-4230` §1.7, R1).** Built before any gate,
//! each unwitnessed row marked acknowledged or not by the specification; a
//! stale acknowledgement is refused as `validate` refuses it, and either fab
//! flag refuses an unacknowledged row before any gate
//! (`UnwitnessedFieldNotAcknowledged`), so no package is written with a row
//! nobody acknowledged.

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
use crate::verdict::Scope;
use crate::{
    EXIT_FAIL, EXIT_INTERNAL, EXIT_INVALID_SPEC, EXIT_NOT_VERIFIED, EXIT_TOOL_ERROR,
    EXIT_UNREADABLE,
};

/// `RUL-19` (1), in the verdict itself.
const RISE_TIMES: &str = "computed from declared values";

#[derive(Serialize)]
struct Report {
    ok: bool,
    /// `pass`, `fail`, or `not_verified` (see [`outcome`]).
    verdict: &'static str,
    /// Why a verdict is `not_verified`; absent on every other.
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'static str>,
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
    /// Every gate whose scope held nothing, by name, and each campaign run
    /// whose droop observations witnessed no device, as `campaign:<scenario>`
    /// (`AET-WO-2306`): a reader sees them without reading every line.
    judged_nothing: Vec<String>,
    /// The support gate's record (`AET-WO-4300g`): each role, strap and
    /// condition as `check support` judged them; absent when it did not run.
    #[serde(skip_serializing_if = "Option::is_none")]
    support: Option<Value>,
    /// `--scenarios`: each scenario's run; absent without it.
    #[serde(skip_serializing_if = "Option::is_none")]
    campaign: Option<Vec<Run>>,
    findings: Vec<Finding>,
    /// Every serial port whose peer deviation is declared, not cited, with
    /// its figure (`AET-WO-4230` R2, the architect's ruling on Q-A).
    declared_peers: Vec<aethelgard_cli::ledger::DeclaredPeer>,
    /// Each SPI and I2C controller's independent rule and each USART's `BRR`
    /// rule (`AET-WO-4230` §1.7, R3); one that does not hold is also an error
    /// finding by name.
    silicon: aethelgard_cli::ledger::Silicon,
    /// The witness ledger (`AET-WO-4230` §1.7, `RUL-20`): every register
    /// field the generated firmware writes or leaves at reset, classed, each
    /// unwitnessed row marked acknowledged or not (R1).
    ledger: aethelgard_cli::ledger::Ledger,
    /// What the twin cannot judge outside the register fields (the
    /// architect's ruling on Q-I): never acknowledged, never blocking.
    twin_limits: Vec<aethelgard_cli::ledger::LedgerRow>,
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
    /// Whether each scenario's run passed the campaign gate, in order.
    #[serde(skip)]
    outcome: Vec<bool>,
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
    /// skipped harness and a campaign short of coverage were refused before
    /// any gate ran; here each is judged again on the gates (D1, ruled).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    preconditions: Vec<fab::Precondition>,
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

/// Every precondition of a package, in a fixed order, on the gates that ran,
/// with [`coverage`] after the gates for the campaign's.
fn preconditions(report: &Report, planned: &[String], covered: Coverage) -> Vec<fab::Precondition> {
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
        fab::Precondition::of(
            "every_harness_proved",
            planned
                .iter()
                .filter(|h| !proved(h))
                .map(|h| format!("{h} not proved"))
                .collect(),
        ),
        fab::Precondition {
            no_device: covered.no_device,
            ..fab::Precondition::of("campaign_faults_every_bus_and_rail", covered.unmet)
        },
        support_judged(&report.gates),
        fab::Precondition::of("every_gate_passed", failed),
    ]
}

/// `AET-WO-4300g` §1.5: met only when the support gate ran and passed; each
/// finding's kind is named otherwise, or that it did not run.
fn support_judged(gates: &[Gate]) -> fab::Precondition {
    let unmet = match gates.iter().find(|g| g.gate == "support") {
        Some(g) if g.verdict == "pass" => Vec::new(),
        Some(g) => g
            .findings
            .iter()
            .map(|f| f["kind"].as_str().unwrap_or("finding").to_owned())
            .chain(
                g.findings
                    .is_empty()
                    .then(|| format!("support: {}", g.verdict)),
            )
            .collect(),
        None => vec!["support: not run".to_owned()],
    };
    fab::Precondition::of("mcu_support_network_judged", unmet)
}

/// The support gate's record as the notes' lines (`AET-WO-4300g` §1.6): each
/// role with its part and distance, each strap, the programming connector,
/// the supply's source, then each condition the gate names.
fn support_lines(record: Option<&Value>) -> Vec<String> {
    let Some(r) = record else {
        return Vec::new();
    };
    let s = |v: &Value| v.as_str().unwrap_or("").to_owned();
    let mut out = Vec::new();
    for role in r["roles"].as_array().into_iter().flatten() {
        let gap = match (role["gap_um"].as_i64(), role["bound_um"].as_i64()) {
            (Some(g), Some(b)) => format!(", {g} um from the pin (bound {b} um)"),
            (Some(g), None) => format!(", {g} um from the pin (no bound: a condition below)"),
            _ => String::new(),
        };
        out.push(format!(
            "{} at {}: {} ({} pF){gap}, {}",
            s(&role["role"]),
            s(&role["pin"]),
            role["part"].as_str().unwrap_or("none"),
            role["picofarads"].as_i64().unwrap_or(0),
            s(&role["rule"])
        ));
    }
    for strap in r["straps"].as_array().into_iter().flatten() {
        out.push(format!("strap: {}", s(strap)));
    }
    match r["programming"].as_object() {
        Some(p) => out.push(format!(
            "programming: {} carries SWDIO, SWCLK, GND and the supply{}",
            p.get("connector").map(s).unwrap_or_default(),
            if p.get("nrst") == Some(&Value::Bool(true)) {
                ", and NRST"
            } else {
                "; NRST does not reach it (not required: DS-009's debug ports carry none)"
            }
        )),
        None => out.push("programming: none".to_owned()),
    }
    if let Some(src) = r["supply_source"].as_str() {
        out.push(format!("supply source: {src}"));
    }
    for c in r["conditions"].as_array().into_iter().flatten() {
        out.push(format!("condition: {}", s(c)));
    }
    out
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

/// Each twin stand-in the board draws, by reference and part, I2C then SPI
/// in the specification's order (`AET-WO-2308` (1)): a part the table holds
/// `stand_in`, there to exercise the twin and never for a purchase
/// (`aethelgard_ir::parts`, `AET-WO-4210` D1). The emitter's BOM gives it no
/// note and the banner names only its lifecycle, so `verify --fab-out` and
/// `--against-fab` refuse it. A vendored table that cannot be read is an
/// internal error, never an empty list.
fn stand_ins(topology: &aethelgard_ir::SolvedTopology) -> Result<Vec<(String, String)>, String> {
    let table = aethelgard_ir::parts::table()
        .map_err(|e| format!("the parts table, read for twin stand-ins: {e}"))?;
    Ok(topology
        .parts_in_order()
        .filter(|(_, part)| table.get(part).is_some_and(|p| p.stand_in))
        .map(|(r, p)| (r.to_owned(), p.to_owned()))
        .collect())
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
    /// `attribute`: the scenario whose run it attributed (`AET-WO-3303` D1).
    #[serde(skip_serializing_if = "Option::is_none")]
    scenario: Option<String>,
    /// `pass`, `fail`, `not_run` when a gate before it failed so it could not
    /// run, or `not_applicable` for a board gate of a specification that
    /// draws no board (`AET-WO-1607` (5)).
    verdict: &'static str,
    /// Why a `not_applicable` gate did not run; absent on every other gate.
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'static str>,
    /// What the gate judged (`AET-WO-2306`); absent when it did not run.
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    scope: Option<Scope>,
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
    /// A per-item gate's items, each with its own verdict (`AET-WO-1211c`,
    /// `2306`'s rule: `judged` counts them); absent on every other gate.
    #[serde(skip_serializing_if = "Option::is_none")]
    items: Option<Vec<Value>>,
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
    /// A battery sag's chain as its campaign line states it (`AET-WO-2304b`).
    #[serde(skip_serializing_if = "Value::is_null")]
    battery_sag: Value,
    /// What the run's verdict does not claim (`AET-WO-2304b` §1.7, §1.11).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    conditions: Vec<String>,
    /// A droop's observations' witness scope, as its campaign line states it
    /// (`AET-WO-2306`).
    #[serde(skip_serializing_if = "Value::is_null")]
    observation_witness: Value,
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
    /// The entries `acknowledge_unwitnessed` lacks
    /// (`UnwitnessedFieldNotAcknowledged` only, `AET-WO-2307`); absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    entries: Option<Vec<aethelgard_ir::UnwitnessedAcknowledgement>>,
}

fn refuse(kind: &str, id: Option<&str>, message: String, code: u8) -> ExitCode {
    crate::emit(
        &Refusal {
            ok: false,
            error: RefusalBody {
                kind: kind.to_owned(),
                id: id.map(str::to_owned),
                message,
                entries: None,
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

/// Why a gate whose line counts nothing judged nothing, when the line does
/// not say (`AET-WO-2306`).
const NOTHING_LISTED: &str = "the gate's verdict line lists no item it judged";

/// What a gate judged, from its own verdict line (`AET-WO-2306`, units
/// accepted by the architect 2026-11-20). The rule: `judged` counts the
/// items that each carry a verdict on the gate's own line, and a gate whose
/// input is judged whole counts 1. So: the line's `judged` where it counts
/// (`extract`: lines + supply domains), with its reason; `support`'s roles
/// plus its straps; and 1 for `erc`, `drc`, `build`, `simulate`, each
/// `prove` harness and each `attribute` run. The campaign's is its runs.
fn scope_of(name: &str, v: &Value) -> Scope {
    let len = |p: &str| v.pointer(p).and_then(Value::as_array).map_or(0, Vec::len) as u64;
    let judged = match (v["judged"].as_u64(), name) {
        (Some(n), _) => n,
        (None, "support") => len("/support/roles") + len("/support/straps"),
        (None, _) => 1,
    };
    Scope::of(
        judged,
        v["nothing_in_scope_reason"]
            .as_str()
            .unwrap_or(NOTHING_LISTED),
    )
}

/// The gates and campaign runs that judged nothing, in the verdict's order
/// (`AET-WO-2306`). Only `prove` and `attribute` repeat, and each judges 1.
fn judged_nothing(gates: &[Gate], runs: &[Run]) -> Vec<String> {
    let gates = gates
        .iter()
        .filter(|g| g.scope.as_ref().and_then(|s| s.nothing_in_scope) == Some(true))
        .map(|g| g.gate.to_owned());
    let runs = runs
        .iter()
        .filter(|r| r.observation_witness["nothing_in_scope"] == true)
        .map(|r| format!("campaign:{}", r.scenario));
    gates.chain(runs).collect()
}

/// A gate's report for the verdict, from its one verdict line.
fn gate_report(name: &'static str, harness: Option<String>, ran: &Ran) -> Gate {
    let v = ran.last();
    Gate {
        gate: name,
        harness,
        scenario: None,
        verdict: if ran.code == 0 && v["verdict"] == "pass" {
            "pass"
        } else {
            "fail"
        },
        reason: None,
        scope: Some(scope_of(name, v)),
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
        items: None,
    }
}

/// The finding kinds each per-item gate carries from `simulate`'s line.
const GPIO_KINDS: [&str; 3] = [
    aethelgard_chaos::campaign::GPIO_INPUT_ASSERTED_FINDING,
    aethelgard_chaos::campaign::GPIO_MISMATCH_FINDING,
    aethelgard_chaos::campaign::GPIO_UNWITNESSED_FINDING,
];
const ADC_KINDS: [&str; 2] = [
    aethelgard_chaos::campaign::ADC_MISMATCH_FINDING,
    aethelgard_chaos::campaign::ADC_UNWITNESSED_FINDING,
];
const BRIDGE_KINDS: [&str; 3] = [
    aethelgard_chaos::campaign::MOTOR_LEVEL_FINDING,
    aethelgard_chaos::campaign::GPIO_MISMATCH_FINDING,
    aethelgard_chaos::campaign::GPIO_UNWITNESSED_FINDING,
];

/// A pin device's per-item gates (`AET-WO-1211c`), from `simulate`'s line:
/// `gpio`, one item per GPIO signal; `adc`, one per ADC input; `bridge`, one
/// per bridge, its inputs' static levels and its enable's write. Each item
/// carries its own verdict, and `judged` counts the items (`2306`: never a
/// whole input counted once). A gate is present only when it has an item,
/// so a board without a pin device reports as before. Each takes
/// `simulate`'s exit, tools, sandbox and input: it reads that run.
fn pin_gates(sim: &Ran, bridges: &[Bridge]) -> Vec<Gate> {
    let v = sim.last();
    let boot = &v["boot"];
    let ok = |w: &Value| w["status"] == "witnessed";
    let gpio: Vec<Value> = boot["gpio"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|g| {
            // An input is named, never judged (D5, ruled; adversarial F4):
            // no firmware path reads it, so its read-back is the twin's own
            // stimulus. An input read at its active level still fails the
            // gate, by its finding.
            if g["direction"] == "input" {
                return serde_json::json!({
                    "net": g["net"],
                    "direction": "input",
                    "status": g["status"],
                    "verdict": "not_judged",
                    "reason": INPUT_NOT_JUDGED,
                });
            }
            serde_json::json!({
                "net": g["net"],
                "direction": g["direction"],
                "status": g["status"],
                "verdict": if ok(g) { "pass" } else { "fail" },
            })
        })
        .collect();
    let adc: Vec<Value> = boot["adc"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|a| {
            serde_json::json!({
                "net": a["net"],
                "status": a["status"],
                "expected": a["expected"],
                "observed": a["observed"],
                "verdict": if ok(a) { "pass" } else { "fail" },
            })
        })
        .collect();
    // A bridge input is a motor output (its level witness) or a GPIO output
    // (its write's witness); the enable is a GPIO output.
    let status_of = |net: &str| -> Value {
        let motor = boot["motor"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|m| m["label"] == net)
            .map(|m| m["level"]["status"].clone());
        let gpio = boot["gpio"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|g| g["net"] == net)
            .map(|g| g["status"].clone());
        motor.or(gpio).unwrap_or(Value::Null)
    };
    let bridge: Vec<Value> = bridges
        .iter()
        .map(|b| {
            let inputs: Vec<Value> = b
                .inputs
                .iter()
                .map(|n| serde_json::json!({"net": n, "status": status_of(n)}))
                .collect();
            let enable = b
                .enable
                .as_ref()
                .map(|n| serde_json::json!({"net": n, "status": status_of(n)}));
            let pass = inputs
                .iter()
                .chain(&enable)
                .all(|i| i["status"] == "witnessed");
            serde_json::json!({
                "id": b.id,
                "inputs": inputs,
                "enable": enable,
                "verdict": if pass { "pass" } else { "fail" },
            })
        })
        .collect();
    let findings = |kinds: &[&str]| -> Vec<Value> {
        v["findings"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|f| f["kind"].as_str().is_some_and(|k| kinds.contains(&k)))
            .cloned()
            .collect()
    };
    [
        ("gpio", gpio, findings(&GPIO_KINDS)),
        ("adc", adc, findings(&ADC_KINDS)),
        ("bridge", bridge, findings(&BRIDGE_KINDS)),
    ]
    .into_iter()
    .filter(|(_, items, _)| !items.is_empty())
    .map(|(name, items, findings)| Gate {
        gate: name,
        harness: None,
        scenario: None,
        verdict: if items.iter().all(|i| i["verdict"] != "fail")
            && findings.iter().all(|f| f["severity"] != "error")
        {
            "pass"
        } else {
            "fail"
        },
        reason: None,
        scope: Some(Scope::of(
            items
                .iter()
                .filter(|i| i["verdict"] != "not_judged")
                .count() as u64,
            INPUT_NOT_JUDGED,
        )),
        exit_code: Some(sim.code),
        tool_version: v["tool"]["version"].as_str().map(str::to_owned),
        uid_boundary: Some(v["sandbox"]["uid_boundary"] == true),
        uid_boundary_unmet: v["sandbox"]["uid_boundary_unmet"]
            .as_str()
            .map(str::to_owned),
        input_sha256: v["input"]["sha256"].as_str().map(str::to_owned),
        harness_provenance: None,
        cover_count: None,
        findings,
        items: Some(items),
    })
    .collect()
}

/// Why a pin device's input is never a judged item (D5, ruled).
const INPUT_NOT_JUDGED: &str = "no generated firmware path reads a pin device's input, so its read-back is the twin's own stimulus (INV-07; D5, ruled)";

/// One bridge of a pin device, as the per-item gate judges it: its id, its
/// inputs' nets (motor outputs or GPIO outputs) and its enable's net.
#[derive(Debug, Clone)]
struct Bridge {
    id: String,
    inputs: Vec<String>,
    enable: Option<String>,
}

/// Every bridge of `topology`'s pin devices, by device and id.
fn bridges(topology: &aethelgard_ir::SolvedTopology) -> Vec<Bridge> {
    topology
        .pin_devices
        .values()
        .flat_map(|d| d.bridges.iter())
        .map(|(id, b)| Bridge {
            id: id.clone(),
            inputs: b.inputs.clone(),
            enable: b.enable.as_ref().map(|e| e.net.clone()),
        })
        .collect()
}

/// The campaign's `uid_boundary`: true only when every run line says `true`.
fn every_run_separated(runs: &[Value]) -> bool {
    runs.iter().all(|line| line["uid_boundary"] == true)
}

fn not_run(name: &'static str) -> Gate {
    Gate {
        gate: name,
        harness: None,
        scenario: None,
        verdict: "not_run",
        reason: None,
        scope: None,
        exit_code: None,
        tool_version: None,
        uid_boundary: None,
        uid_boundary_unmet: None,
        input_sha256: None,
        harness_provenance: None,
        cover_count: None,
        findings: Vec::new(),
        items: None,
    }
}

/// The gates that judge the drawn board: on a specification that declares
/// `board.drawn: false` there is none, so each is reported `not_applicable`
/// and none runs (`AET-WO-1607` (5), `1210` F2).
const BOARD_GATES: [&str; 4] = ["erc", "drc", "support", "extract"];

/// Why a board gate is `not_applicable`.
const NO_BOARD: &str =
    "the specification declares `board.drawn: false`, so no board is drawn for this gate to judge";

fn not_applicable(name: &'static str) -> Gate {
    Gate {
        verdict: "not_applicable",
        reason: Some(NO_BOARD),
        ..not_run(name)
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
        "support" => vec![
            e("the schematic", "/input/sha256", w.sch),
            e("the board", "/support/board/sha256", w.pcb),
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

/// What a scenario's fault stresses (`AET-WO-3303` D1).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Target {
    I2c(String),
    Spi(String),
    /// A serial port, by label: reached by `serial_rx_open` (`AET-WO-2302`
    /// D5), detected by its mechanism.
    Serial(String),
    Rail(String),
    /// The battery (`AET-WO-2304b` §1.10): a variant of its own, so no
    /// domain id can collide with it.
    Battery,
    /// A pin device's ADC input, by net (`AET-WO-1211c`): reached by an
    /// `analog_value` other than its idle's, which the run must survive.
    Analog(String),
}

impl Target {
    fn name(&self) -> String {
        match self {
            Self::I2c(b) => format!("I2C bus `{b}`"),
            Self::Spi(b) => format!("SPI bus `{b}`"),
            Self::Serial(l) => format!("serial port `{l}`"),
            Self::Rail(r) => format!("rail `{r}`"),
            Self::Battery => "rail `battery`".to_owned(),
            Self::Analog(n) => format!("ADC input `{n}`"),
        }
    }
}

/// An intolerable fault's named mechanism: the only finding kinds its run
/// may fail by, the half and rule `attribute` must name, and every subject
/// the attribution may cite (ruled (a)).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Mechanism {
    kinds: &'static [&'static str],
    rule: &'static str,
    subjects: std::collections::BTreeSet<String>,
    /// The subjects that must each be attributed: a rail out of band is
    /// detected only when each consumer it renders is (`AET-WO-2304b`,
    /// adversarial 5), by its `port:` or `device:` subject; a controller
    /// stays admissible, never required. Empty for every other mechanism.
    required: std::collections::BTreeSet<String>,
}

/// Each override kind's mechanism, a closed table: the engine's, never the
/// scenario's own `expect_findings`.
const RISE: &[&str] = &["analog_rise_exceeds_ceiling", "i2c_timeout"];
const HELD: &[&str] = &["i2c_line_held_low", "i2c_timeout"];
/// An absent device fails every read the firmware makes of it: its identity,
/// and its data where the bring-up reads data (an IMU's, `AET-WO-4220c`);
/// `attribute` decides each by the same E2 evidence (`AET-WO-2304b` F-IMU,
/// measured on Sentinel-03's U5 and the power fixture's).
const ABSENT: &[&str] = &[
    "spi_data_mismatch",
    "spi_identity_mismatch",
    "spi_transaction_unwitnessed",
];
const OPEN: &[&str] = &["serial_echo_missing"];

/// What one scenario seeds, judged against the specification.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Seeded {
    /// Expects a pass and every override changes nothing: the no-fault baseline.
    Baseline,
    /// Expects a pass: the design must survive each of these. The second
    /// list says, for each droop on the MCU's supply that crossed no active
    /// BOR threshold, why it covers nothing (`AET-WO-2305` §1.4).
    Tolerable(Vec<Target>, Vec<String>),
    /// Expects a fail by one fault's mechanism, on one target.
    Detect(Target, Mechanism),
    /// Anything else: it covers nothing, and its run is judged as before.
    Other,
}

/// One override: `None` when it changes nothing the campaign judges (a value
/// equal to the specification's own); otherwise what it stresses, with its
/// mechanism when it is intolerable by kind. An eased value (a lower line
/// capacitance, a droop inside its band) stresses nothing yet changes the
/// specification: `Some((None, None))`.
fn stress(
    spec: &aethelgard_ir::Specification,
    topology: &aethelgard_ir::SolvedTopology,
    o: &aethelgard_chaos::record::Override,
) -> Option<(Option<Target>, Option<Mechanism>)> {
    use aethelgard_chaos::record::Override;
    let bus_subjects = |bus: &str| -> std::collections::BTreeSet<String> {
        std::iter::once(format!("bus:{bus}"))
            .chain(
                topology
                    .i2c_devices
                    .iter()
                    .filter(|(_, d)| d.bus == bus)
                    .map(|(r, _)| format!("device:{r}")),
            )
            .collect()
    };
    let i2c = |bus: &str, kinds: &'static [&'static str]| {
        (
            Some(Target::I2c(bus.to_owned())),
            Some(Mechanism {
                kinds,
                rule: "E1",
                subjects: bus_subjects(bus),
                required: Default::default(),
            }),
        )
    };
    match o {
        Override::PullUpOhms { bus, ohms } => {
            let b = spec.i2c_buses.iter().find(|b| &b.id == bus)?;
            // Raised, the line rises slower; lowered, it only rises faster
            // (adversary 2, F1; `solve` refuses one below the sink floor).
            match ohms.cmp(&b.pull_up_ohms) {
                std::cmp::Ordering::Equal => None,
                std::cmp::Ordering::Greater => Some(i2c(bus, RISE)),
                std::cmp::Ordering::Less => Some((None, None)),
            }
        }
        Override::MaxCapacitance { net, femtofarads } => {
            let c = spec.net_constraints.iter().find(|c| &c.net_name == net)?;
            if c.max_capacitance_femtofarads == *femtofarads {
                return None;
            }
            // Raised, a line rises slower; lowered, it only eases (§4 F3).
            let bus = spec
                .i2c_buses
                .iter()
                .find(|b| &b.sda.net == net || &b.scl.net == net);
            Some(match bus {
                Some(b) if *femtofarads > c.max_capacitance_femtofarads => i2c(&b.id, RISE),
                _ => (None, None),
            })
        }
        Override::LineHeldLow { bus, .. } => Some(i2c(bus, HELD)),
        Override::SpiDeviceAbsent { reference } => {
            let d = topology.spi_devices.get(reference)?;
            Some((
                Some(Target::Spi(d.bus.clone())),
                Some(Mechanism {
                    kinds: ABSENT,
                    rule: "E2",
                    subjects: [
                        format!("device:{reference}"),
                        format!("controller:{}", d.peripheral),
                    ]
                    .into(),
                    required: Default::default(),
                }),
            ))
        }
        Override::SerialRxOpen { port } => {
            let p = spec.serial_ports.iter().find(|p| &p.label == port)?;
            Some((
                Some(Target::Serial(p.label.clone())),
                Some(Mechanism {
                    kinds: OPEN,
                    rule: "E3",
                    subjects: [format!("port:{port}")].into(),
                    required: Default::default(),
                }),
            ))
        }
        Override::Droop(d) => {
            // Below the domain's tolerance band: inside it the rail stays in
            // its specification (§4 F3). Judged at the loaded floor, the
            // declared floor less the load's I·R drop, which is the floor the
            // scheduler renders (`AET-WO-2304b` §1.10, L697). A droop names
            // no mechanism: an expected-fail droop is judged as before.
            let v = spec.voltage_domains.iter().find(|v| v.id == d.domain)?;
            let band = aethelgard_chaos::campaign::tolerance_floor_uv(
                v.nominal_millivolts,
                u32::from(v.tolerance_permille),
            );
            let loaded = aethelgard_chaos::campaign::rail_levels(d, v.nominal_millivolts)
                .map(|(floor, _)| floor);
            Some(
                if loaded.is_none_or(|f| f < band) && uncrossed(spec, d).is_none() {
                    (Some(Target::Rail(d.domain.clone())), None)
                } else {
                    (None, None)
                },
            )
        }
        // A battery sag the checks admit stresses the battery when its
        // static operating point lies below the battery's declared minimum
        // (`AET-WO-2304b` §1.10, L704's principle). IR counts no path drop
        // (K6), so every admitted sag lies below it, even 1 mΩ above the
        // healthy R: what keeps a non-fault from covering is the clause "its R
        // exceeds the healthy R" (§1.6), as K6 ruled. A sag the checks refuse
        // stresses nothing.
        Override::BatterySag(sag) => {
            let s = crate::campaign::resolve_battery_sag(sag, spec).ok()?;
            Some(if s.static_microvolts < s.chain.v_open_microvolts {
                (Some(Target::Battery), None)
            } else {
                (None, None)
            })
        }
        // A rail out of band is detected by its consumers' mechanism: E3 for
        // serial ports, E2 for SPI devices, its subjects theirs
        // (`AET-WO-2304b` §1.10). A rail the checks refuse stresses nothing.
        // A held input (`AET-WO-1211c`): at the level its pull holds it, it
        // changes nothing; at the other it changes the run but reaches no
        // target (D5, ruled: no generated firmware path reads an input, so
        // its gate would read back the twin's own stimulus, `INV-07`).
        Override::PinHeld { net, level } => {
            let plan = aethelgard_twin::pins::plan(topology, &Default::default()).ok();
            // A hold the twin's plan refuses (an output, an unknown net) is
            // no baseline: the campaign refuses its run (adversarial F10).
            let Some(i) = plan
                .as_ref()
                .and_then(|p| p.inputs.iter().find(|i| &i.net == net))
            else {
                return Some((None, None));
            };
            (i.high != (*level == aethelgard_chaos::record::Level::High)).then_some((None, None))
        }
        // A set ADC value other than the idle's stresses its input: the run
        // must convert and read it (`AET-WO-1211c`).
        Override::AnalogValue { net, millivolts } => {
            let plan = aethelgard_twin::pins::plan(topology, &Default::default()).ok();
            let Some(a) = plan
                .as_ref()
                .and_then(|p| p.analog.iter().find(|a| &a.net == net))
            else {
                return Some((None, None));
            };
            (u64::from(*millivolts) * 1000 != a.microvolts)
                .then(|| (Some(Target::Analog(net.clone())), None))
        }
        Override::RailOutOfBand { domain } => {
            let r = crate::campaign::rendered(spec, domain).ok()?;
            let (kinds, rule, subjects) = if r.devices.is_empty() {
                (
                    OPEN,
                    "E3",
                    r.ports.iter().map(|p| format!("port:{p}")).collect(),
                )
            } else {
                let mut subjects = std::collections::BTreeSet::new();
                for d in &r.devices {
                    let t = topology.spi_devices.get(d)?;
                    subjects.insert(format!("device:{d}"));
                    subjects.insert(format!("controller:{}", t.peripheral));
                }
                (ABSENT, "E2", subjects)
            };
            let required = r
                .ports
                .iter()
                .map(|p| format!("port:{p}"))
                .chain(r.devices.iter().map(|d| format!("device:{d}")))
                .collect();
            Some((
                Some(Target::Rail(domain.clone())),
                Some(Mechanism {
                    kinds,
                    rule,
                    subjects,
                    required,
                }),
            ))
        }
    }
}

/// Why a droop below its domain's band covers no rail (`AET-WO-2305` §1.4):
/// on the MCU's supply at the board's BOR level above 0, a loaded floor at
/// or above that level's falling threshold renders no reset, so no recovery
/// is ever judged. `None` for any other droop, and for one inside the band,
/// which covers nothing already (§4 F3). The level is the specification's
/// (`mcu.bor_level`), which the campaign holds the droop's to, so no droop
/// declares its way past this; the threshold is the typical value the
/// scheduler renders at, and `check_droop` refuses a floor inside its band.
fn uncrossed(
    spec: &aethelgard_ir::Specification,
    d: &aethelgard_chaos::record::Droop,
) -> Option<String> {
    let level = spec.mcu.bor_level;
    if d.domain != spec.mcu.supply || level == 0 {
        return None;
    }
    let v = spec.voltage_domains.iter().find(|v| v.id == d.domain)?;
    let (floor, _) = aethelgard_chaos::campaign::rail_levels(d, v.nominal_millivolts)?;
    let band = aethelgard_chaos::campaign::tolerance_floor_uv(
        v.nominal_millivolts,
        u32::from(v.tolerance_permille),
    );
    let t = aethelgard_chaos::power::BOR.get(usize::from(level))?;
    (floor < band && floor >= t.falling()).then(|| {
        format!(
            "at BOR level {level} a droop must cross {}'s falling level, {} mV; its loaded floor is {} mV",
            t.name,
            t.falling() / 1000,
            floor / 1000
        )
    })
}

/// What `scenario` seeds (`AET-WO-3303` D1, ruled (a)).
fn classify(
    spec: &aethelgard_ir::Specification,
    topology: &aethelgard_ir::SolvedTopology,
    scenario: &aethelgard_chaos::campaign::Scenario,
) -> Seeded {
    use aethelgard_chaos::record::Override;
    // A rail out of band is one fault: the absences listed beside it are its
    // rendering, not faults of their own (`AET-WO-2304b` §1.7).
    let rail = scenario.rail_out_of_band().is_some();
    let changed: Vec<_> = scenario
        .overrides
        .iter()
        .filter(|o| {
            !(rail
                && matches!(
                    o,
                    Override::SerialRxOpen { .. } | Override::SpiDeviceAbsent { .. }
                ))
        })
        .filter_map(|o| stress(spec, topology, o))
        .collect();
    if scenario.expect_pass {
        if changed.is_empty() {
            return Seeded::Baseline;
        }
        // A scenario that also eases something covers nothing: its net change
        // may ease the target it stresses (adversary 2, F2). A held line, an
        // absent device or an open RX line is never a fault to survive: an
        // absent IMU or flash must be caught, not survived (ruled (a);
        // adversary 2, F4), and so must a port that hears nothing, and a
        // rail whose consumers are gone.
        let intolerable = scenario.overrides.iter().any(|o| {
            matches!(
                o,
                Override::LineHeldLow { .. }
                    | Override::SpiDeviceAbsent { .. }
                    | Override::SerialRxOpen { .. }
                    | Override::RailOutOfBand { .. }
                    | Override::PinHeld { .. }
            )
        });
        // A droop that crossed no BOR threshold covers nothing, as an eased
        // override does, and says why (`AET-WO-2305` §1.4).
        let lacks: Vec<String> = scenario
            .overrides
            .iter()
            .filter_map(|o| match o {
                Override::Droop(d) => {
                    uncrossed(spec, d).map(|why| format!("`{}` crosses none: {why}", scenario.name))
                }
                _ => None,
            })
            .collect();
        if intolerable || changed.iter().any(|(t, _)| t.is_none()) {
            return Seeded::Tolerable(Vec::new(), lacks);
        }
        return Seeded::Tolerable(changed.into_iter().filter_map(|(t, _)| t).collect(), lacks);
    }
    // One target, one mechanism, every changed override stressing it, and the
    // declared findings within that mechanism's kinds.
    let mut seeded = None;
    for c in changed {
        let (Some(t), Some(m)) = c else {
            return Seeded::Other;
        };
        match &seeded {
            None => seeded = Some((t, m)),
            Some((t0, m0)) if *t0 == t && *m0 == m => {}
            Some(_) => return Seeded::Other,
        }
    }
    match seeded {
        Some((t, m))
            if !scenario.expect_findings.is_empty()
                && scenario
                    .expect_findings
                    .iter()
                    .all(|k| m.kinds.contains(&k.as_str())) =>
        {
            Seeded::Detect(t, m)
        }
        _ => Seeded::Other,
    }
}

/// The campaign's coverage of every declared bus, serial port and rail.
#[derive(Debug, Default, PartialEq, Eq)]
struct Coverage {
    /// Each bus, port or rail no fault covers, named with why where no kind
    /// can.
    unmet: Vec<String>,
    /// Each bus no device uses: never `met`, and it does not block (ruled (c)).
    no_device: Vec<String>,
}

/// Coverage, in the specification's order: I2C buses, SPI buses, serial
/// ports, then rails (each voltage domain, and the battery when a power tree
/// is declared). Before any gate `outcome` is `None` and a seeded fault
/// counts as it is declared; after the gates `outcome[i]` says whether
/// scenario `i`'s run passed the gate, and only those count. A tolerable
/// fault covers its target; a detected one covers its bus or port only when
/// the campaign holds a baseline (ruled (a)). A serial port is covered only
/// by a detected `serial_rx_open` (`AET-WO-2302` D5, ending the exemption
/// ruled on #155). A rail other than the MCU's supply is met by a detected
/// `rail_out_of_band` beside a baseline, and unmet otherwise, with why when
/// no such fault can reach it; the battery is met by a tolerated
/// `battery_sag` (`AET-WO-2304b` §1.10).
fn coverage(
    spec: &aethelgard_ir::Specification,
    topology: &aethelgard_ir::SolvedTopology,
    seeded: &[Seeded],
    outcome: Option<&[bool]>,
) -> Coverage {
    use std::collections::BTreeSet;
    let ran = |i: usize| outcome.is_none_or(|o| o.get(i) == Some(&true));
    let baseline = seeded
        .iter()
        .enumerate()
        .any(|(i, s)| *s == Seeded::Baseline && ran(i));
    let mut covered: BTreeSet<&Target> = BTreeSet::new();
    for (i, s) in seeded.iter().enumerate() {
        match s {
            Seeded::Tolerable(targets, _) if ran(i) => covered.extend(targets),
            Seeded::Detect(t, _) if baseline && ran(i) => {
                covered.insert(t);
            }
            _ => {}
        }
    }
    let mut c = Coverage::default();
    let nothing = "no device: nothing to fault";
    for (used, target) in spec
        .i2c_buses
        .iter()
        .map(|b| {
            let used = topology.i2c_devices.values().any(|d| d.bus == b.id);
            (used, Target::I2c(b.id.clone()))
        })
        .chain(spec.spi_buses.iter().map(|b| {
            let used = topology.spi_devices.values().any(|d| d.bus == b.id);
            (used, Target::Spi(b.id.clone()))
        }))
    {
        if !used {
            c.no_device.push(format!("{} ({nothing})", target.name()));
        } else if !covered.contains(&target) {
            c.unmet.push(target.name());
        }
    }
    for p in &spec.serial_ports {
        let target = Target::Serial(p.label.clone());
        if !covered.contains(&target) {
            c.unmet.push(target.name());
        }
    }
    // A pin device's ADC inputs and inputs (`AET-WO-1211c`), in the
    // specification's order, from the twin's plan; a plan the twin refuses
    // leaves `simulate` to refuse the board, and covers nothing here. An
    // input is no target (D5, ruled): no generated firmware path reads it,
    // so a held input's gate would read back the twin's own stimulus
    // (`INV-07`); it is named, as a bus no device uses (ruled (c)).
    if let Ok(plan) = aethelgard_twin::pins::plan(topology, &Default::default()) {
        for a in &plan.analog {
            let target = Target::Analog(a.net.clone());
            if !covered.contains(&target) {
                c.unmet.push(target.name());
            }
        }
        for i in &plan.inputs {
            c.no_device.push(format!(
                "input `{}` (no generated firmware path reads it, so a held input's gate would read back the twin's own stimulus, INV-07; a target when firmware responds to it)",
                i.net
            ));
        }
    }
    // A bridge (D2, ruled): no fault kind reaches its outputs, so it is
    // named, never met, and does not block, as a bus no device uses.
    for (reference, d) in &topology.pin_devices {
        for id in d.bridges.keys() {
            c.no_device.push(format!(
                "bridge `{id}` of `{reference}` (no fault kind reaches a bridge's outputs: the twin models no pin device; AET-WO-4600 F3)"
            ));
        }
    }
    for v in &spec.voltage_domains {
        let target = Target::Rail(v.id.clone());
        if covered.contains(&target) {
            continue;
        }
        // A motor rail (D1, ruled (ii)): named, never met, never blocking.
        if let Some(why) = motor_rail(spec, topology, &v.id) {
            c.no_device.push(format!("{} ({why})", target.name()));
            continue;
        }
        // A rail no fault can reach says why (`AET-WO-2304b` §1.10); the
        // MCU's supply names each droop that crossed no BOR threshold
        // (`AET-WO-2305` §1.4).
        let lacks: Vec<&str> = seeded
            .iter()
            .enumerate()
            .filter(|&(i, _)| ran(i))
            .flat_map(|(_, s)| match s {
                Seeded::Tolerable(_, lacks) => lacks.as_slice(),
                _ => &[],
            })
            .map(String::as_str)
            .collect();
        match crate::campaign::rendered(spec, &v.id) {
            Err(e) if v.id != spec.mcu.supply => {
                c.unmet.push(format!("{} ({})", target.name(), e.message));
            }
            _ if v.id == spec.mcu.supply && !lacks.is_empty() => {
                c.unmet
                    .push(format!("{} ({})", target.name(), lacks.join("; ")));
            }
            _ => c.unmet.push(target.name()),
        }
    }
    if spec.power.is_some() && !covered.contains(&Target::Battery) {
        c.unmet.push(Target::Battery.name());
    }
    c
}

/// Why `domain` is a motor rail no fault can reach (`AET-WO-1211c` D1,
/// ruled (ii)): a pin device's supply pin is on it, and nothing else consumes
/// it but connector pins: no serial port, SPI or I2C device, I2C bus,
/// regulator or declared load, and it is not the MCU's supply.
/// `None` for every other domain.
fn motor_rail(
    spec: &aethelgard_ir::Specification,
    topology: &aethelgard_ir::SolvedTopology,
    domain: &str,
) -> Option<String> {
    if domain == spec.mcu.supply {
        return None;
    }
    // The pin devices' supply pins, as solved (`AET-WO-1211c` D7, `RUL-16`).
    let pins: Vec<String> = topology
        .pin_devices
        .iter()
        .flat_map(|(reference, d)| {
            d.supplies
                .iter()
                .filter(|(_, v)| *v == domain)
                .map(move |(pin, _)| format!("{reference}.{pin}"))
        })
        .collect();
    if pins.is_empty() {
        return None;
    }
    let r = aethelgard_ir::power::rail_consumers(spec, domain);
    let loaded = spec
        .power
        .iter()
        .any(|p| p.loads.iter().any(|l| l.domain == domain));
    (r.serial_rx.is_empty()
        && r.tx_only.is_empty()
        && r.spi_devices.is_empty()
        && r.i2c_devices.is_empty()
        && r.i2c_buses.is_empty()
        && r.regulators.is_empty()
        && !loaded)
        .then(|| {
            format!(
                "it supplies only pin devices' supply pins ({}), and the twin models no pin device (device_unmodelled): no fault on it reaches anything the twin observes; judged on the bench",
                pins.join(", ")
            )
        })
}

/// Whether a failed run is its seeded fault detected by the named mechanism
/// (ruled (a)): it failed and met its expectation, every finding kind is the
/// mechanism's, and `attribute` (bound to the run) names the electrical half,
/// the mechanism's rule, nothing unexplained or masked, and only the seeded
/// subjects.
/// `Err` says why not.
fn detection(m: &Mechanism, run: &Run, attributed: &Value) -> Result<(), String> {
    if run.verdict != "fail" || !run.matched {
        return Err(format!(
            "verdict {}, its expectation {}",
            run.verdict,
            if run.matched { "met" } else { "unmet" }
        ));
    }
    let kinds: Vec<&str> = run
        .findings
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    if kinds.is_empty() || kinds.iter().any(|k| !m.kinds.contains(k)) {
        return Err(format!(
            "it failed by {kinds:?}, not only by its fault's {:?}",
            m.kinds
        ));
    }
    let text = |k: &str| attributed[k].as_str().unwrap_or_default();
    if attributed["ok"] != true || text("domain") != "electrical" || text("rule") != m.rule {
        return Err(format!(
            "attribute names `{}` by rule `{}`, not `electrical` by `{}`",
            text("domain"),
            text("rule"),
            m.rule
        ));
    }
    if attributed["unexplained"]
        .as_array()
        .is_none_or(|u| !u.is_empty())
    {
        return Err(format!(
            "attribute leaves {} unexplained",
            attributed["unexplained"]
        ));
    }
    // Software evidence on the faulted bus is something else failing in the
    // same run (a wrong address behind a weak pull-up, `cases.json`'s trap).
    if attributed["masked"]
        .as_array()
        .is_none_or(|m| !m.is_empty())
    {
        return Err(format!(
            "attribute masks {} on the faulted bus: something else fails too",
            attributed["masked"]
        ));
    }
    let subjects: Vec<&str> = attributed["subjects"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let other: Vec<&&str> = subjects
        .iter()
        .filter(|s| !m.subjects.contains(**s))
        .collect();
    if subjects.is_empty() || !other.is_empty() {
        return Err(format!(
            "attribute cites {other:?} beyond the seeded {:?}",
            m.subjects
        ));
    }
    if let Some(missing) = m.required.iter().find(|r| !subjects.contains(&r.as_str())) {
        return Err(format!(
            "attribute cites {subjects:?}, not every rendered consumer: {missing} is missing"
        ));
    }
    Ok(())
}

/// Each harness the board crate declares that `planned` does not name: every
/// `kani::proof` in every file of the crate is counted, however it is written
/// (`kani::proof_for_contract` too, which Kani also runs), so a harness the line reader in [`harnesses`] misses is never
/// silently left unproved (`AET-WO-3303` §1.1).
fn unplanned(board: &BTreeMap<String, String>, planned: usize) -> Option<String> {
    let declared: usize = board
        .values()
        .map(|text| text.matches("kani::proof").count())
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
    /// What it seeds (`AET-WO-3303` D1).
    seeded: Seeded,
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
    // No fabrication package is written or compared for a board never drawn
    // (`AET-WO-1210` §1.4): `generate::tree` draws none for it (F1). Plain
    // `verify` runs, and reports each of [`BOARD_GATES`] `not_applicable`
    // (`AET-WO-1607` (5)).
    let command = match fab_mode {
        Some(FabMode::Write(_)) => "verify --fab-out",
        Some(FabMode::Compare(_)) => "verify --against-fab",
        None => "verify",
    };
    if fab_mode.is_some()
        && let Err(r) = crate::draws_a_board(&spec, command)
    {
        return crate::emit(&r, EXIT_INVALID_SPEC);
    }
    let drawn = spec.board.drawn;
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
    // The ledger before any gate: a field it cannot class is refused, never
    // guessed (`AET-WO-4230` §1.7, §5.4). A stale acknowledgement is refused
    // as `validate` refuses it; every unwitnessed row is then marked (R1).
    let (mut ledger, twin_limits) =
        match aethelgard_cli::ledger::build(&topology, &table).and_then(|l| {
            let mut limits = aethelgard_cli::ledger::twin_limits(&topology)?;
            limits.extend(aethelgard_cli::ledger::off_domain_rows(&spec, &topology)?);
            Ok((l, limits))
        }) {
            Ok(l) => l,
            Err(e) => return refuse(e.kind(), e.id(), e.message(), EXIT_INTERNAL),
        };
    if let Some(s) = aethelgard_cli::ledger::stale(&ledger, &spec) {
        return refuse(
            aethelgard_cli::ledger::Stale::KIND,
            Some(&s.register),
            s.message(),
            EXIT_INVALID_SPEC,
        );
    }
    let unacknowledged = aethelgard_cli::ledger::acknowledge(&mut ledger, &spec);
    let silicon = aethelgard_cli::ledger::silicon(&topology);
    let project = spec.project_name.clone();
    let mut report = Report {
        ok: false,
        verdict: "fail",
        reason: None,
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
        judged_nothing: Vec::new(),
        support: None,
        campaign: None,
        findings: silicon_findings(&silicon),
        declared_peers: aethelgard_cli::ledger::declared_peers(&spec),
        silicon,
        ledger,
        twin_limits,
        against_fab: None,
        fab: None,
        lifecycle_unheld,
        conditions: Vec::new(),
        outcome: Vec::new(),
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
            seeded: classify(&spec, &topology, &scenario),
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
    let harnesses = harnesses(board.get("src/proofs.rs").map_or("", String::as_str));
    // A harness the plan leaves out is refused by name before any gate runs
    // (`AET-WO-3303` §1.1); the other preconditions are judged on the gates.
    if fab_mode.is_some() {
        let left_out = unplanned(&board, harnesses.len());
        if let Some(m) = left_out {
            return refuse(
                "HarnessSkipped",
                None,
                format!("no package: {m}"),
                EXIT_UNREADABLE,
            );
        }
    }
    // Every bus and rail a fault must cover, as the scenarios declare them,
    // before any gate (`AET-WO-3303` D1, ruled): the gates then decide
    // whether each run survived or detected its fault.
    let seeded: Vec<Seeded> = expected.iter().map(|e| e.seeded.clone()).collect();
    if fab_mode.is_some() {
        let short = coverage(&spec, &topology, &seeded, None);
        if !short.unmet.is_empty() {
            return refuse(
                "CampaignCoverageShort",
                None,
                format!(
                    "no package: the campaign covers no fault on {}",
                    short.unmet.join(", ")
                ),
                EXIT_UNREADABLE,
            );
        }
        // A twin stand-in drawn on the board is refused by name (`AET-WO-2308`
        // (1), shape (b)): no package file can label it. After the coverage
        // check, so a board refused for its campaign keeps that verdict.
        let drawn = match stand_ins(&topology) {
            Ok(d) => d,
            Err(m) => {
                crate::diagnose(&m);
                return ExitCode::from(EXIT_INTERNAL);
            }
        };
        if let Some((reference, _)) = drawn.first() {
            let named: Vec<String> = drawn.iter().map(|(r, p)| format!("{r} ({p})")).collect();
            return refuse(
                "StandInInFabPackage",
                Some(reference),
                format!(
                    "no package: {} is a twin stand-in (`stand_in` in the parts table), there to exercise the twin and never for a purchase, and the emitter's BOM carries no label for it: draw the part the board is for, or run `verify` without a fab flag",
                    named.join(", ")
                ),
                EXIT_INVALID_SPEC,
            );
        }
        // Every unwitnessed ledger row acknowledged by name (`AET-WO-4230`
        // R1, the architect's ruling): none is fabricated unread.
        // `entries` is each missing entry, its reason left for a person
        // (`AET-WO-2307`).
        if let Some(first) = unacknowledged.first() {
            let entries = aethelgard_cli::ledger::missing_entries(&report.ledger, &spec);
            return crate::emit(
                &Refusal {
                    ok: false,
                    error: RefusalBody {
                        kind: "UnwitnessedFieldNotAcknowledged".to_owned(),
                        id: Some(first.clone()),
                        message: format!(
                            "no package: {} unwitnessed ledger row(s) no `acknowledge_unwitnessed` entry names for its register and class: {}. `entries` holds the {} entry(ies) they need, one per register and class with exactly the fields to cover: each replaces any entry for its register and class (at most one entry per register and class), and an entry the specification already held comes back with its fields completed and its `reason` emptied, to be written for the whole set. Write each `reason`: 1 to {} bytes of printable ASCII with no space at either end; `validate` refuses an empty one (`InvalidSource`)",
                            unacknowledged.len(),
                            unacknowledged.join(", "),
                            entries.len(),
                            aethelgard_ir::board::MAX_SOURCE,
                        ),
                        entries: Some(entries),
                    },
                },
                EXIT_INVALID_SPEC,
            );
        }
    }

    // `--against` first: a directory that is not the tree fails, naming
    // every difference, and no gate runs; with a fab flag, nothing is
    // written or compared, and the verdict says so.
    if let Some(dir) = against {
        if let Err(s) = crate::input::dir_check(dir) {
            return crate::stopped(s);
        }
        let differences = match tree::against(dir, &files, &[&format!("{BOARD_DIR}/target")]) {
            Ok(d) => d,
            Err(s) => return crate::stopped(s),
        };
        if !differences.is_empty() {
            report.against = Some("differs");
            report.findings.extend(differences.iter().map(|d| Finding {
                kind: d.kind(),
                severity: "error",
                message: describe(d),
            }));
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
    // A board's files are generated only when it is drawn. Never a sha256,
    // so a board gate bound to it would fail closed (`BindingMismatch`); on
    // a board not drawn no board gate runs, and no fab flag gets this far.
    let board_file = |rel: &str| hashes.get(rel).map_or("", String::as_str);

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
            drawn,
            sch: board_file(&sch),
            pcb: board_file(&pcb),
            repl: &hashes[&repl],
            board: &board_sha256,
            spec: &spec_sha256,
            harnesses: harnesses.clone(),
            boot_time: format!("{}ns", aethelgard_twin::boot::MAX_VIRTUAL_NS),
            bridges: bridges(&topology),
        },
        &expected,
    ) {
        return code;
    }
    let tail = Tail {
        project: &project,
        spec: &spec,
        spec_sha256: &spec_sha256,
        topology: &topology,
        board: root.join(&pcb),
        board_sha256: board_file(&pcb),
        elf: scratch.tool.join("firmware.elf"),
        // Read only with a fab flag, which a board not drawn never reaches.
        bom: files
            .get(&format!("{KICAD_DIR}/{project}.bom.csv"))
            .map_or(&[][..], |t| t.as_bytes()),
        expected: &expected,
        seeded: &seeded,
        harnesses: &harnesses,
    };
    match conclude(&mut report, fab_mode, &tail) {
        Ok(code) => crate::emit(&report, code),
        Err(code) => code,
    }
}

/// What [`conclude`] needs of a `verify` call besides its report.
struct Tail<'a> {
    project: &'a str,
    spec: &'a aethelgard_ir::Specification,
    spec_sha256: &'a str,
    topology: &'a aethelgard_ir::SolvedTopology,
    /// The generated board and ELF in `verify`'s scratch, and the board's
    /// generated sha256.
    board: PathBuf,
    board_sha256: &'a str,
    elf: PathBuf,
    /// `emit`'s bill of materials, as generated.
    bom: &'a [u8],
    expected: &'a [Expected],
    seeded: &'a [Seeded],
    harnesses: &'a [String],
}

/// `verify`'s last steps once its gates have run: the verdict, then the
/// package for a fab flag, then the exit code to print the verdict with.
/// The code is the verdict's alone ([`outcome`]),
/// whatever the fab mode (`AET-WO-2308` LP2, `AET-WO-1909` LP2); `Err` is a
/// refusal already printed.
fn conclude(report: &mut Report, fab_mode: Option<&FabMode>, t: &Tail<'_>) -> Result<u8, ExitCode> {
    report.judged_nothing = judged_nothing(
        &report.gates,
        report.campaign.as_deref().unwrap_or_default(),
    );
    let drawn = t.spec.board.drawn;
    (report.ok, report.verdict, report.reason) =
        outcome(passes(&report.findings, &report.gates, drawn), drawn);
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
            project: t.project,
            spec: t.spec,
            specification_sha256: t.spec_sha256,
            board: &t.board,
            board_sha256: t.board_sha256,
            elf: &t.elf,
            elf_sha256: &elf_sha256,
            bom: t.bom,
            conditions: &conditions,
            lifecycle: &lifecycle,
            support: &support_lines(report.support.as_ref()),
            campaign: &campaign_conditions(report.campaign.as_deref().unwrap_or_default()),
        };
        let scenarios: Vec<fab::Scenario> = t
            .expected
            .iter()
            .map(|e| fab::Scenario {
                scenario: e.name.clone(),
                sha256: hex(e.scenario),
            })
            .collect();
        let outcome = report.outcome.clone();
        let short = coverage(t.spec, t.topology, t.seeded, Some(&outcome));
        let pre = preconditions(report, t.harnesses, short);
        package(report, mode, &bound, &scenarios, &pre)?;
        if let Some(f) = report.fab.as_mut() {
            f.preconditions = pre;
        }
    }
    // Read after the package: `--against-fab` fails a verdict whose
    // package differs (`package`).
    Ok(exit_code(report.ok, report.verdict))
}

/// The verdict and its reason, from whether every gate passed and whether
/// the board is drawn (`AET-WO-1607` D2, ruled): `INV-03`'s "verified" needs
/// ERC, so a board not drawn is never `pass`. Its run is `not_verified`,
/// reason `DrawsNoBoard`, when every gate that could run passed; a gate that
/// failed is `fail`, drawn or not.
fn outcome(passed: bool, drawn: bool) -> (bool, &'static str, Option<&'static str>) {
    match (passed, drawn) {
        (true, true) => (true, "pass", None),
        (true, false) => (false, "not_verified", Some("DrawsNoBoard")),
        (false, _) => (false, "fail", None),
    }
}

/// The exit code of the verdict as it is finally stated: `0` on a pass,
/// [`EXIT_NOT_VERIFIED`] on `not_verified`, and [`EXIT_FAIL`] otherwise.
fn exit_code(ok: bool, verdict: &str) -> u8 {
    match (ok, verdict) {
        (true, _) => 0,
        (false, "not_verified") => EXIT_NOT_VERIFIED,
        (false, _) => EXIT_FAIL,
    }
}

/// Every condition the campaign's runs name, each once, in the order the
/// runs first name it (`AET-WO-2304b` §1.7, §1.11): FAB-NOTES states them.
fn campaign_conditions(runs: &[Run]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for c in runs.iter().flat_map(|r| &r.conditions) {
        if !out.contains(c) {
            out.push(c.clone());
        }
    }
    out
}

/// The verdict passes when no finding is an error and every gate passed; on
/// a board not drawn, each of [`BOARD_GATES`] may instead be
/// `not_applicable` (`AET-WO-1607` (5)). On a drawn board nothing is.
fn passes(findings: &[Finding], gates: &[Gate], drawn: bool) -> bool {
    let excused =
        |g: &Gate| !drawn && g.verdict == "not_applicable" && BOARD_GATES.contains(&g.gate);
    findings.iter().all(|f| f.severity != "error")
        && gates.iter().all(|g| g.verdict == "pass" || excused(g))
}

/// Each controller whose independent rule does not hold, as an error by name
/// (`AET-WO-4230` §1.7). `generate` refuses such a topology first
/// (svd-hal `check_silicon`), so through the CLI it is empty; the verdict
/// judges it again on its own.
fn silicon_findings(s: &aethelgard_cli::ledger::Silicon) -> Vec<Finding> {
    aethelgard_cli::ledger::silicon_failures(s)
        .into_iter()
        .map(|message| Finding {
            kind: "silicon_rule_fails",
            severity: "error",
            message,
        })
        .collect()
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
    preconditions: &[fab::Precondition],
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
            preconditions,
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
            let differences = tree::against(dir, &package, &[]).map_err(crate::stopped)?;
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

/// The campaign gate's tool version (`AET-WO-2308` L3, `AET-WO-3301` F14):
/// the Renode build each run's own archive header names (`AET-WO-2205`
/// §1.1), read back by replay, never `simulate`'s. Every run must name the
/// same one; an archive that does not replay, or names none or another, is a
/// `ToolError`.
fn campaign_renode(archive: &Path, runs: &[Run]) -> Result<String, ExitCode> {
    let fail = |m: String| {
        refuse(
            "ToolError",
            Some("aethelgard"),
            format!("gate `campaign`: {m}"),
            EXIT_TOOL_ERROR,
        )
    };
    let mut build: Option<String> = None;
    for run in runs {
        let replayed = aethelgard_chaos::replay_dir(&archive.join(&run.run_id)).map_err(|e| {
            fail(format!(
                "the archive of run {} does not replay: {e}",
                run.run_id
            ))
        })?;
        let named = replayed
            .header
            .tools
            .iter()
            .find(|(name, _)| name == aethelgard_chaos::campaign::RENODE)
            .map(|(_, version)| version.clone())
            .ok_or_else(|| {
                fail(format!(
                    "the archive of run {} names no Renode build",
                    run.run_id
                ))
            })?;
        match &build {
            Some(b) if *b != named => {
                return Err(fail(format!(
                    "the runs name two Renode builds, `{b}` and `{named}`"
                )));
            }
            _ => build = Some(named),
        }
    }
    build.ok_or_else(|| fail("no run to read a Renode build from".to_owned()))
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
    /// `board.drawn`: false runs none of [`BOARD_GATES`].
    drawn: bool,
    sch: &'a str,
    pcb: &'a str,
    repl: &'a str,
    board: &'a str,
    spec: &'a str,
    harnesses: Vec<String>,
    /// The `--for` `simulate` boots for: the board's window, as the
    /// campaign's (`AET-WO-4300a` D1).
    boot_time: String,
    /// The pin devices' bridges (`AET-WO-1211c`), for the bridge gate.
    bridges: Vec<Bridge>,
}

fn args(items: &[&dyn AsRef<std::ffi::OsStr>]) -> Vec<OsString> {
    items.iter().map(|i| (**i).as_ref().to_owned()).collect()
}

/// ERC, DRC with parity, `extract`, `prove` of every harness, `build`,
/// `simulate --firmware` and the campaign, in that order, each bound to the
/// generated hashes. A refusal or a binding mismatch stops the run. On a
/// board not drawn, each of [`BOARD_GATES`] is `not_applicable` instead.
fn run_gates(
    report: &mut Report,
    p: &Paths,
    want: &Want<'_>,
    expected: &[Expected],
) -> Result<(), ExitCode> {
    if want.drawn {
        board_gates(report, p, want)?;
    } else {
        report
            .gates
            .extend(BOARD_GATES.into_iter().map(not_applicable));
    }
    proof_and_boot_gates(report, p, want, expected)
}

/// ERC, DRC with parity, `support` and `extract`: [`BOARD_GATES`].
fn board_gates(report: &mut Report, p: &Paths, want: &Want<'_>) -> Result<(), ExitCode> {
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

    // The MCU's support network, from the schematic's netlist (`AET-WO-4300g`).
    let support = gate(
        "support",
        args(&[
            &"check",
            &"support",
            &"--schematic",
            &p.sch,
            &"--board",
            &p.pcb,
            &"--spec",
            &p.spec,
        ]),
    )?;
    bound("support", &support, want, "", "")?;
    // The record, less the board's scratch path, which differs every run: the
    // verdict names the board by its sha256, as every gate's binding does.
    report.support = support.last().get("support").cloned().map(|mut s| {
        if let Some(b) = s.get_mut("board").and_then(Value::as_object_mut) {
            b.remove("path");
        }
        s
    });
    report.gates.push(gate_report("support", None, &support));

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
    Ok(())
}

/// `prove` of every harness, `build`, `simulate --firmware` and the campaign.
fn proof_and_boot_gates(
    report: &mut Report,
    p: &Paths,
    want: &Want<'_>,
    expected: &[Expected],
) -> Result<(), ExitCode> {
    if want.harnesses.is_empty() {
        report.findings.push(Finding {
            kind: "no_harness",
            severity: "error",
            message: "the generated board crate has no Kani harness".to_owned(),
        });
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
    report.gates.extend(pin_gates(&sim, &want.bridges));

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
            battery_sag: line["battery_sag"].clone(),
            observation_witness: line["observation_witness"].clone(),
            conditions: line["conditions"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|c| c.as_str().map(str::to_owned))
                .collect(),
        });
    }
    let mut g = gate_report("campaign", None, &ran);
    g.tool_version = Some(campaign_renode(Path::new(&p.archive), &out)?);
    g.scope = Some(Scope::of(out.len() as u64, NOTHING_LISTED));
    g.uid_boundary = Some(every_run_separated(runs));
    g.uid_boundary_unmet = runs
        .iter()
        .find_map(|line| line["uid_boundary_unmet"].as_str().map(str::to_owned));
    // `AET-WO-3303` D1, ruled (a), reversing `AET-WO-3301` §3.2: a run
    // passes the campaign gate when it survived (its verdict `pass`, every
    // expectation met), or when it is its seeded intolerable fault detected by
    // the named mechanism, with the no-fault baseline in the campaign passing.
    // A failure for any other reason fails the gate.
    let survived = |r: &Run| r.verdict == "pass" && r.matched;
    let baseline = out
        .iter()
        .zip(expected)
        .any(|(r, e)| e.seeded == Seeded::Baseline && survived(r));
    let mut why: Vec<Option<String>> = Vec::new();
    for (r, e) in out.iter().zip(expected) {
        why.push(match &e.seeded {
            _ if survived(r) => None,
            Seeded::Detect(_, m) if !baseline => Some(format!(
                "it seeds a fault by {:?}, and the campaign holds no passing no-fault baseline",
                m.kinds
            )),
            Seeded::Detect(_, m) if r.verdict == "fail" && r.matched => {
                let dir = OsString::from(Path::new(&p.archive).join(&r.run_id));
                let attributed = gate("attribute", args(&[&"attribute", &"--archive", &dir]))?;
                let a = attributed.last();
                bind(
                    "attribute",
                    "the run it attributes",
                    &r.run_id,
                    &a["run_id"],
                )?;
                bind(
                    "attribute",
                    "the run's state sequence",
                    &r.state_sequence_hash,
                    &a["state_sequence_hash"],
                )?;
                let mut g = gate_report("attribute", None, &attributed);
                g.scenario = Some(r.scenario.clone());
                g.uid_boundary = None;
                g.verdict = if attributed.code == 0 && a["ok"] == true {
                    "pass"
                } else {
                    "fail"
                };
                report.gates.push(g);
                detection(m, r, a).err()
            }
            _ => Some(format!("verdict {}, expected {}", r.verdict, r.expect)),
        });
    }
    report.outcome = why.iter().map(Option::is_none).collect();
    g.verdict = if ran.code == 0 && why.iter().all(Option::is_none) {
        "pass"
    } else {
        "fail"
    };
    g.findings = out
        .iter()
        .zip(&why)
        .filter_map(|(r, w)| w.as_ref().map(|w| (r, w)))
        .flat_map(|(r, w)| {
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
                    "message": format!("scenario `{}`: {w}", r.scenario),
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

    /// `AET-WO-1607` D2, ruled: a run on a board not drawn never exits 0
    /// and is never `pass`. With every gate passed it is `not_verified`
    /// (`DrawsNoBoard`, exit 7), and with one failed it is `fail` (exit 1).
    /// A drawn board's outcome is unchanged.
    #[test]
    fn a_board_not_drawn_never_exits_0() {
        for passed in [true, false] {
            let (ok, verdict, _) = outcome(passed, false);
            assert!(!ok && verdict != "pass", "{passed}");
            assert_ne!(exit_code(ok, verdict), 0, "{passed}");
        }
        assert_eq!(
            outcome(true, false),
            (false, "not_verified", Some("DrawsNoBoard"))
        );
        assert_eq!(exit_code(false, "not_verified"), EXIT_NOT_VERIFIED);
        assert_eq!(outcome(false, false), (false, "fail", None));
        assert_ne!(EXIT_NOT_VERIFIED, EXIT_FAIL);
        assert_eq!(outcome(true, true), (true, "pass", None));
        assert_eq!(exit_code(true, "pass"), 0);
        assert_eq!(outcome(false, true), (false, "fail", None));
        // A pass that `--against-fab` turned into a fail exits 1 (the
        // `four_layer_fab_tier2` and `against_fab_tier2` cases).
        assert_eq!(exit_code(false, "fail"), EXIT_FAIL);
    }

    /// `AET-WO-1607` (5): a board gate reports `not_applicable` with its
    /// reason and nothing it did not measure. The verdict may pass with the
    /// four board gates `not_applicable` only on a board not drawn; on a
    /// drawn board, or for any other gate, it fails.
    #[test]
    fn a_board_gate_is_not_applicable_only_on_a_board_not_drawn() {
        let v = serde_json::to_value(not_applicable("erc")).unwrap();
        assert_eq!(
            v,
            json!({"gate": "erc", "verdict": "not_applicable", "reason": NO_BOARD,
                   "exit_code": null, "tool_version": null, "uid_boundary": null,
                   "input_sha256": null, "findings": []})
        );
        assert!(
            serde_json::to_value(not_run("erc"))
                .unwrap()
                .get("reason")
                .is_none()
        );
        let passed = |name| Gate {
            verdict: "pass",
            ..not_run(name)
        };
        let board: Vec<Gate> = BOARD_GATES.into_iter().map(not_applicable).collect();
        let mut gates = board;
        gates.extend(["prove", "build", "simulate"].map(passed));
        assert!(passes(&[], &gates, false));
        assert!(!passes(&[], &gates, true), "a drawn board's gates must run");
        let mut prove = BOARD_GATES.map(passed).into_iter().collect::<Vec<_>>();
        prove.push(not_applicable("prove"));
        assert!(!passes(&[], &prove, false), "only a board gate is excused");
    }

    /// `AET-WO-4230` §1.7: a controller whose independent rule does not hold
    /// fails the verdict by name, every gate passing; one that holds adds
    /// nothing. A hand-edited controller (MBR one too fast), since `generate`
    /// refuses such a topology before the verdict sees it.
    #[test]
    fn a_silicon_rule_that_does_not_hold_fails_the_verdict_by_name() {
        let controller = |mbr: u8| aethelgard_ir::SolvedSpiController {
            kernel: aethelgard_ir::SpiKernel::PerCk,
            kernel_hz: 64_000_000,
            declared_hz: 20_000_000,
            mbr,
            achieved_hz: 64_000_000 >> (mbr + 1),
            achieved_max_hz: 64_958_976 >> (mbr + 1),
        };
        // `AET-WO-4230` R3: the console's BRR, 556 the nearest whole divider
        // of 64 MHz at 115200 baud, 557 one off.
        let silicon_brr = |mbr, brr| {
            let spi = vec![aethelgard_ir::silicon::check_spi(
                "SPI1",
                &controller(mbr),
                &aethelgard_ir::SolvedClock::reset(),
            )];
            let usart = vec![aethelgard_cli::ledger::UsartFinding {
                port: "CONSOLE".into(),
                peripheral: "USART1".into(),
                brr: aethelgard_ir::silicon::check_brr(
                    "USART1",
                    &aethelgard_ir::SolvedClock::reset(),
                    64_000_000,
                    115_200,
                    brr,
                ),
            }];
            let holds = spi.iter().all(|f| f.holds) && usart.iter().all(|f| f.brr.holds);
            aethelgard_cli::ledger::Silicon {
                spi,
                i2c: Vec::new(),
                usart,
                kernel_clock: Vec::new(),
                holds,
            }
        };
        let silicon = |mbr| silicon_brr(mbr, 556);
        let mut gate = not_run("erc");
        gate.verdict = "pass";
        let gates = [gate];
        assert!(silicon_findings(&silicon(1)).is_empty());
        assert!(passes(&silicon_findings(&silicon(1)), &gates, true));
        let brr = silicon_findings(&silicon_brr(1, 557));
        assert_eq!(brr.len(), 1);
        assert!(
            brr[0].message.starts_with("USART1 (CONSOLE): ")
                && brr[0].message.contains("check_brr")
                && brr[0].message.contains("nearest false"),
            "{}",
            brr[0].message
        );
        assert!(!passes(&brr, &gates, true));
        let failing = silicon_findings(&silicon(0));
        assert_eq!(failing.len(), 1);
        assert_eq!(failing[0].kind, "silicon_rule_fails");
        assert!(
            failing[0].message.starts_with("SPI1: "),
            "{}",
            failing[0].message
        );
        assert!(
            failing[0].message.contains("mbr_ok"),
            "{}",
            failing[0].message
        );
        assert!(!passes(&failing, &gates, true));
    }

    /// `AET-WO-4300g`: the package's support precondition is met only by a
    /// support gate that ran and passed.
    #[test]
    fn the_support_precondition_needs_the_gate_passed() {
        let state = |gates: &[Gate]| {
            let p = support_judged(gates);
            (p.state, p.unmet)
        };
        let mut passed = not_run("support");
        passed.verdict = "pass";
        assert_eq!(state(&[passed]), ("met", vec![]));
        let mut failed = not_run("support");
        failed.verdict = "fail";
        failed.findings = vec![json!({"kind": "support_capacitor_missing"})];
        assert_eq!(
            state(&[failed]),
            ("unmet", vec!["support_capacitor_missing".to_owned()])
        );
        assert_eq!(
            state(&[not_run("support")]),
            ("unmet", vec!["support: not_run".to_owned()])
        );
        assert_eq!(state(&[]), ("unmet", vec!["support: not run".to_owned()]));
    }

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
        // A contract harness is a harness Kani runs (adversarial F6).
        let contract = BTreeMap::from([(
            "src/proofs.rs".to_owned(),
            "#[kani::proof]\nfn a() {}\n#[kani::proof_for_contract(f)]\nfn b() {}\n".to_owned(),
        )]);
        assert!(unplanned(&contract, 1).is_some());
    }

    fn seeded(
        spec: &aethelgard_ir::Specification,
        t: &aethelgard_ir::SolvedTopology,
        files: &[&str],
    ) -> Vec<Seeded> {
        files
            .iter()
            .map(|f| classify(spec, t, &scenario_file(f)))
            .collect()
    }

    /// A droop's offset in these classifications, which never place it: the
    /// minimal board's (`AET-WO-2310`, `spike/2310/results.md`), 1 ms from reset.
    const PLANTED: i64 = -2_627_610;

    fn json_scenario(
        expect_pass: bool,
        findings: &[&str],
        overrides: Value,
    ) -> aethelgard_chaos::campaign::Scenario {
        aethelgard_chaos::campaign::Scenario::parse(
            // 6: a droop is planted from the boot word (`AET-WO-2310`).
            json!({"schema_version": 6, "name": "x", "base": "b.json",
                "expect": if expect_pass { "pass" } else { "fail" },
                "expect_findings": findings, "overrides": overrides})
            .to_string()
            .as_bytes(),
        )
        .unwrap()
    }

    /// L697 (`AET-WO-2304b` §1.10): a droop covers its rail when its loaded
    /// floor, the declared floor less the load's I·R drop (the floor the
    /// scheduler renders), lies below the band. 3,140 mV with 100 mA through
    /// 500 mΩ is 3,090 mV loaded, below 3,135 mV: covered, though the
    /// declared floor is above it; 3,200 mV is 3,150 mV loaded: not. On a
    /// board at BOR level 0, where the band alone decides (`AET-WO-2305`
    /// §1.4 moved this test off Sentinel-01, whose level is 3).
    #[test]
    fn a_droop_covers_its_rail_by_its_loaded_floor() {
        let (spec, t) = solved(FOUR_LAYER);
        let droop = |floor: u32| {
            json_scenario(
                true,
                &[],
                json!([{"droop": {"domain": "VDD_3V3", "floor_millivolts": floor,
                    "from_boot_word_ns": PLANTED, "fall_ns": 20000, "hold_ns": 200000,
                    "rise_ns": 20000, "source_milliohms": 500, "load_milliamps": 100}}]),
            )
        };
        assert_eq!(
            classify(&spec, &t, &droop(3140)),
            Seeded::Tolerable(vec![Target::Rail("VDD_3V3".into())], vec![])
        );
        assert_eq!(
            classify(&spec, &t, &droop(3200)),
            Seeded::Tolerable(vec![], vec![])
        );
    }

    /// `AET-WO-2305` §1.4: on the MCU's supply at BOR level 3, a droop below
    /// the band covers the rail only by crossing VBOR3's falling level,
    /// 2,610 mV. 2,900 mV is 2,850 mV loaded: below the band, above the
    /// threshold, so no reset and no recovery is judged; it covers nothing,
    /// and the rail's row names the crossing it lacks. 2,400 mV covers. A
    /// droop on the same rail at BOR level 0 is judged by the band alone.
    #[test]
    fn a_droop_covers_the_mcu_rail_only_by_crossing_its_bor_level() {
        let (spec, t) = solved(S1);
        let droop = |floor: u32| {
            json_scenario(
                true,
                &[],
                json!([{"droop": {"domain": "VDD_3V3", "floor_millivolts": floor,
                    "from_boot_word_ns": PLANTED, "fall_ns": 20000, "hold_ns": 200000,
                    "rise_ns": 20000, "source_milliohms": 500, "load_milliamps": 100}}]),
            )
        };
        let why = "`x` crosses none: at BOR level 3 a droop must cross VBOR3's falling level, 2610 mV; its loaded floor is 2850 mV";
        assert_eq!(
            classify(&spec, &t, &droop(2900)),
            Seeded::Tolerable(vec![], vec![why.to_owned()])
        );
        assert_eq!(
            classify(&spec, &t, &droop(2400)),
            Seeded::Tolerable(vec![Target::Rail("VDD_3V3".into())], vec![])
        );
        // The board's level decides: since `AET-WO-2310` a droop states none.
        let (four, t4) = solved(FOUR_LAYER);
        assert_eq!(
            classify(&four, &t4, &droop(2900)),
            Seeded::Tolerable(vec![Target::Rail("VDD_3V3".into())], vec![])
        );
        let s = |d: aethelgard_chaos::campaign::Scenario| {
            let mut v = seeded(
                &spec,
                &t,
                &[TOLERATED_I2C, ABSENT_U4, OPEN_CONSOLE, BASELINE],
            );
            v.push(classify(&spec, &t, &d));
            coverage(&spec, &t, &v, None).unmet
        };
        assert_eq!(s(droop(2900)), [format!("rail `VDD_3V3` ({why})")]);
        assert!(s(droop(2400)).is_empty());
    }

    /// Adversarial 5 (`AET-WO-2304b`): a rail out of band is detected only
    /// when every consumer it renders is attributed, not one of them.
    #[test]
    fn a_rail_is_detected_only_with_every_rendered_consumer_attributed() {
        let attributed = |rule: &str, subjects: Value| {
            json!({"ok": true, "domain": "electrical", "rule": rule, "unexplained": [],
                   "masked": [], "subjects": subjects})
        };
        // Sentinel-03's VDD_5V: both ports.
        let (spec, t) = solved("crates/aethelgard-cli/tests/fixtures/sentinel-03.json");
        let rail = |d: &str| match stress(
            &spec,
            &t,
            &aethelgard_chaos::record::Override::RailOutOfBand { domain: d.into() },
        ) {
            Some((Some(_), Some(m))) => m,
            other => panic!("{other:?}"),
        };
        let m = rail("VDD_5V");
        let run = failed(json!(["serial_echo_missing"]));
        assert!(detection(&m, &run, &attributed("E3", json!(["port:CRSF"]))).is_err());
        assert!(
            detection(
                &m,
                &run,
                &attributed("E3", json!(["port:CRSF", "port:MAVLINK"]))
            )
            .is_ok()
        );
        // The power fixture's VDD_1V8, against its measured attribution
        // (E2, `device:U5` alone): the controller is admissible, not required.
        let (spec, t) = solved("crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json");
        let m = match stress(
            &spec,
            &t,
            &aethelgard_chaos::record::Override::RailOutOfBand {
                domain: "VDD_1V8".into(),
            },
        ) {
            Some((Some(_), Some(m))) => m,
            other => panic!("{other:?}"),
        };
        let run = failed(json!([
            "spi_data_mismatch",
            "spi_identity_mismatch",
            "spi_transaction_unwitnessed"
        ]));
        assert!(detection(&m, &run, &attributed("E2", json!(["device:U5"]))).is_ok());
        assert!(detection(&m, &run, &attributed("E2", json!(["controller:SPI2"]))).is_err());
    }

    fn failed(findings: Value) -> Run {
        Run {
            scenario: "x".into(),
            verdict: "fail".into(),
            expect: "fail".into(),
            matched: true,
            findings,
            run_id: String::new(),
            state_sequence_hash: String::new(),
            battery_sag: Value::Null,
            conditions: Vec::new(),
            observation_witness: Value::Null,
        }
    }

    /// `AET-WO-2306` §1.2, §1.3: each gate's entry carries what its own line
    /// says it judged (the line's `judged` and reason, `support`'s roles and
    /// straps, else 1 for one input judged whole); `judged_nothing` names
    /// exactly the gates with nothing in scope, in order, and each run whose
    /// droop witnessed no device; a gate that did not run carries no scope.
    #[test]
    fn judged_nothing_names_exactly_the_gates_and_runs_that_judged_nothing() {
        let ran = |line: Value| Ran {
            code: 0,
            lines: vec![line],
        };
        let extract = gate_report(
            "extract",
            None,
            &ran(
                json!({"verdict": "pass", "judged": 0, "nothing_in_scope": true,
                        "nothing_in_scope_reason": "no bounded line"}),
            ),
        );
        let support = gate_report(
            "support",
            None,
            &ran(json!({"verdict": "pass", "support": {"roles": [1, 2, 3], "straps": [4]}})),
        );
        let erc = gate_report("erc", None, &ran(json!({"verdict": "pass"})));
        let empty_support = gate_report("support", None, &ran(json!({"verdict": "pass"})));
        let scope = |g: &Gate| serde_json::to_value(g.scope.as_ref()).unwrap();
        assert_eq!(
            scope(&extract),
            json!({"judged": 0, "nothing_in_scope": true, "nothing_in_scope_reason": "no bounded line"})
        );
        assert_eq!(scope(&support), json!({"judged": 4}));
        assert_eq!(scope(&erc), json!({"judged": 1}));
        assert_eq!(
            scope(&empty_support),
            json!({"judged": 0, "nothing_in_scope": true, "nothing_in_scope_reason": NOTHING_LISTED})
        );
        assert!(not_run("simulate").scope.is_none());
        let v = serde_json::to_value(&extract).unwrap();
        assert_eq!(v["nothing_in_scope"], true, "flattened into the entry: {v}");
        assert!(
            serde_json::to_value(not_run("simulate"))
                .unwrap()
                .get("judged")
                .is_none()
        );

        let mut droop = failed(json!([]));
        droop.scenario = "droop-bor3".into();
        droop.observation_witness = json!({"judged": 0, "nothing_in_scope": true});
        let mut witnessed = failed(json!([]));
        witnessed.observation_witness = json!({"judged": 1});
        assert_eq!(
            judged_nothing(
                &[erc, extract, support, not_run("campaign")],
                &[failed(json!([])), witnessed, droop]
            ),
            vec!["extract".to_owned(), "campaign:droop-bor3".to_owned()]
        );
        assert!(judged_nothing(&[empty_support], &[]) == vec!["support".to_owned()]);
    }

    /// `AET-WO-4241` §1.3 against `AET-WO-2306`'s fields: an unmodelled
    /// device's read, named `identity_not_witnessable`, never counts toward
    /// a gate's `judged`. The boot judges its input whole (1) however many
    /// reads its line names not witnessable, a boot that names only those is
    /// still 1 (the boot word, the registers, the PWR are judged), and its
    /// info rows are no item of the line's own `judged`.
    #[test]
    fn a_not_witnessable_read_never_counts_toward_judged() {
        let ran = |line: Value| Ran {
            code: 0,
            lines: vec![line],
        };
        let row = json!({"kind": "identity_not_witnessable", "severity": "info", "message": "U3"});
        let read = json!({"reference": "U3", "status": "timeout_txis", "cost_ns": 3600710});
        let scope = |line: Value| {
            serde_json::to_value(gate_report("simulate", None, &ran(line)).scope).unwrap()
        };
        let modelled = json!({"verdict": "pass", "findings": [],
            "boot": {"i2c": [{"reference": "U2"}], "spi": [{"reference": "U4"}]}});
        let with_one = json!({"verdict": "pass", "findings": [row],
            "boot": {"i2c": [{"reference": "U2"}], "spi": [{"reference": "U4"}],
                     "not_witnessable": {"count": 1, "reads": [read], "total_ns": 3600710}}});
        let only = json!({"verdict": "pass", "findings": [row, row, row],
            "boot": {"i2c": [], "spi": [],
                     "not_witnessable": {"count": 3, "reads": [read, read, read], "total_ns": 10802130}}});
        for line in [modelled, with_one, only] {
            assert_eq!(scope(line.clone()), json!({"judged": 1}), "{line}");
        }
    }

    const S1: &str = "examples/sentinel-01-minimal.json";
    const FOUR_LAYER: &str = "crates/aethelgard-cli/tests/fixtures/four-layer.json";
    const BASELINE: &str = "scenarios/sentinel-01/pullup-2k2.json";
    const TOLERATED_I2C: &str = "scenarios/sentinel-01/seeded/pullup-3k3.json";
    const RAIL: &str = "scenarios/sentinel-01/droop-bor3.json";
    const ABSENT_U4: &str = "scenarios/sentinel-01/seeded/spi-absent-u4.json";
    const OPEN_CONSOLE: &str = "scenarios/sentinel-01/seeded/serial-rx-open-console.json";
    const CONSOLE: &str = "serial port `CONSOLE`";

    /// `AET-WO-3303` D1, ruled: each declared bus, serial port and rail is
    /// named until a fault covers it. A tolerable fault covers its target; a
    /// detected one its bus or port only beside a baseline; before the
    /// gates as declared, after them only when the run passed the gate. The
    /// reference's seeded set covers everything.
    #[test]
    fn coverage_needs_a_survived_or_a_detected_fault_beside_a_baseline() {
        let (spec, t) = solved(S1);
        let all = [
            "I2C bus `SENSORS`",
            "SPI bus `FLASH`",
            CONSOLE,
            "rail `VDD_3V3`",
        ];
        let unmet = |files: &[&str], outcome: Option<&[bool]>| {
            coverage(&spec, &t, &seeded(&spec, &t, files), outcome).unmet
        };
        assert_eq!(unmet(&[BASELINE], None), all);
        assert_eq!(
            unmet(&[TOLERATED_I2C, RAIL], None),
            ["SPI bus `FLASH`", CONSOLE]
        );
        // A detected fault without a baseline covers nothing.
        assert_eq!(
            unmet(&[TOLERATED_I2C, RAIL, ABSENT_U4, OPEN_CONSOLE], None),
            ["SPI bus `FLASH`", CONSOLE]
        );
        let set = [TOLERATED_I2C, RAIL, ABSENT_U4, OPEN_CONSOLE, BASELINE];
        assert!(unmet(&set, None).is_empty());
        assert!(unmet(&set, Some(&[true; 5])).is_empty());
        // After the gates, a detection that failed covers nothing, nor does
        // one beside a baseline that failed.
        assert_eq!(
            unmet(&set, Some(&[true, true, false, true, true])),
            ["SPI bus `FLASH`"]
        );
        assert_eq!(
            unmet(&set, Some(&[true, true, true, false, true])),
            [CONSOLE]
        );
        assert_eq!(
            unmet(&set, Some(&[true, true, true, true, false])),
            ["SPI bus `FLASH`", CONSOLE]
        );
        // A droop inside the tolerance band covers nothing (§4 F3).
        assert_eq!(
            unmet(
                &["scenarios/sentinel-01/droop-tolerated.json", TOLERATED_I2C],
                None
            ),
            ["SPI bus `FLASH`", CONSOLE, "rail `VDD_3V3`"]
        );
    }

    /// Ruled (b) and (c), with `AET-WO-2304b` §1.10: a rail other than the
    /// MCU's supply that no `rail_out_of_band` covers stays `unmet`, named
    /// with why (here, no consumer a judge observes); a bus no device uses is
    /// named as having nothing to fault and does not block.
    #[test]
    fn a_rail_no_kind_reaches_is_unmet_and_a_deviceless_bus_does_not_block() {
        let (spec, t) = solved("crates/aethelgard-cli/tests/fixtures/connectors.json");
        let faults = json_scenario(
            true,
            &[],
            json!([{"bus": "SENSORS", "pull_up_ohms": 3300},
                {"droop": {"domain": "VDD_3V3", "floor_millivolts": 2400, "from_boot_word_ns": PLANTED,
                    "fall_ns": 20000, "hold_ns": 200000, "rise_ns": 20000,
                    "source_milliohms": 500, "load_milliamps": 100}}]),
        );
        let absent = json_scenario(
            false,
            &["spi_identity_mismatch"],
            json!([{"spi_device_absent": "U4"}]),
        );
        let base = json_scenario(true, &[], json!([]));
        let s: Vec<Seeded> = [faults, absent, base]
            .iter()
            .map(|x| classify(&spec, &t, x))
            .collect();
        let c = coverage(&spec, &t, &s, None);
        // `AET-WO-2304b` §1.10: a non-MCU rail no fault reaches says why.
        assert_eq!(
            c.unmet,
            ["rail `VDD_5V` (`VDD_5V` has no consumer a judge observes)"]
        );
        assert_eq!(c.no_device, ["SPI bus `EXT` (no device: nothing to fault)"]);
    }

    /// `AET-WO-2302` D5, ending the exemption ruled on #155: each serial
    /// port is `unmet`, in the specification's order, until a detected
    /// `serial_rx_open` on it stands beside a baseline; one port's open line
    /// covers that port alone, and nothing is named outside `unmet`.
    #[test]
    fn a_serial_port_is_covered_only_by_its_own_detected_open_rx_line() {
        let (spec, t) = solved(S1);
        let set = [TOLERATED_I2C, RAIL, ABSENT_U4, BASELINE];
        let c = coverage(&spec, &t, &seeded(&spec, &t, &set), None);
        assert_eq!(c.unmet, [CONSOLE]);
        assert!(c.no_device.is_empty());
        let row = fab::Precondition {
            no_device: c.no_device,
            ..fab::Precondition::of("campaign_faults_every_bus_and_rail", c.unmet)
        };
        assert_eq!(
            serde_json::to_value(&row).unwrap(),
            json!({"precondition": "campaign_faults_every_bus_and_rail", "state": "unmet",
                "unmet": [CONSOLE]})
        );
        let (spec, t) = solved("crates/aethelgard-cli/tests/fixtures/serial-four.json");
        let ports = ["MAVLINK", "GPS", "CRSF", "SBUS"].map(|l| format!("serial port `{l}`"));
        let serial_unmet = |s: &[Seeded]| -> Vec<String> {
            coverage(&spec, &t, s, None)
                .unmet
                .into_iter()
                .filter(|u| u.starts_with("serial"))
                .collect()
        };
        assert_eq!(serial_unmet(&[]), ports);
        let open = |port: &str| {
            json_scenario(
                false,
                &["serial_echo_missing"],
                json!([{ "serial_rx_open": port }]),
            )
        };
        let gps = classify(&spec, &t, &open("GPS"));
        assert!(
            matches!(&gps, Seeded::Detect(Target::Serial(p), m)
                if p == "GPS" && m.rule == "E3" && m.kinds == ["serial_echo_missing"]
                    && m.subjects == ["port:GPS".to_owned()].into()),
            "{gps:?}"
        );
        let base = classify(&spec, &t, &json_scenario(true, &[], json!([])));
        assert_eq!(base, Seeded::Baseline);
        assert_eq!(
            serial_unmet(&[gps.clone(), base.clone()]),
            ["MAVLINK", "CRSF", "SBUS"].map(|l| format!("serial port `{l}`"))
        );
        // Without the baseline, the detection covers nothing (ruled (a)).
        assert_eq!(serial_unmet(&[gps]), ports);
        // An open line expected to pass is never survived; one declaring a
        // kind outside its mechanism, or opening two ports, counts for
        // nothing.
        let c = |s: &aethelgard_chaos::campaign::Scenario| classify(&spec, &t, s);
        assert_eq!(
            c(&json_scenario(
                true,
                &[],
                json!([{"serial_rx_open": "GPS"}])
            )),
            Seeded::Tolerable(Vec::new(), Vec::new())
        );
        assert_eq!(
            c(&json_scenario(
                false,
                &["serial_echo_missing", "serial_pattern_missing"],
                json!([{"serial_rx_open": "GPS"}])
            )),
            Seeded::Other
        );
        assert_eq!(
            c(&json_scenario(
                false,
                &["serial_echo_missing"],
                json!([{"serial_rx_open": "GPS"}, {"serial_rx_open": "CRSF"}])
            )),
            Seeded::Other
        );
    }

    /// What a scenario seeds: a value equal to the specification's is no
    /// fault (so the 2.2 kΩ scenario is the baseline); a fault is detectable
    /// only by its own kinds; two targets, a declared kind outside the
    /// mechanism, or a droop make an expected fail count for nothing; an
    /// eased value is no baseline.
    #[test]
    fn a_scenario_seeds_one_fault_by_its_own_mechanism_or_counts_for_nothing() {
        let (spec, t) = solved(S1);
        let c = |s: &aethelgard_chaos::campaign::Scenario| classify(&spec, &t, s);
        assert_eq!(c(&scenario_file(BASELINE)), Seeded::Baseline);
        assert!(
            matches!(c(&scenario_file(ABSENT_U4)), Seeded::Detect(Target::Spi(b), _) if b == "FLASH")
        );
        for f in [
            "scenarios/sentinel-01/seeded/held-low-scl.json",
            "scenarios/sentinel-01/seeded/cap-220p-2k2.json",
            "scenarios/sentinel-01/pullup-47k.json",
        ] {
            assert!(
                matches!(c(&scenario_file(f)), Seeded::Detect(Target::I2c(b), _) if b == "SENSORS"),
                "{f}"
            );
        }
        let wrong = json_scenario(
            false,
            &["register_mismatch"],
            json!([{"spi_device_absent": "U4"}]),
        );
        assert_eq!(c(&wrong), Seeded::Other);
        let two = json_scenario(
            false,
            &["i2c_timeout"],
            json!([{"bus": "SENSORS", "pull_up_ohms": 47000}, {"spi_device_absent": "U4"}]),
        );
        assert_eq!(c(&two), Seeded::Other);
        let droop = json_scenario(
            false,
            &["brownout_unrecovered"],
            json!([{"droop": {"domain": "VDD_3V3", "floor_millivolts": 2400, "from_boot_word_ns": PLANTED,
                "fall_ns": 20000, "hold_ns": 200000, "rise_ns": 20000, "source_milliohms": 500,
                "load_milliamps": 100}}]),
        );
        assert_eq!(c(&droop), Seeded::Other);
        let sda = &spec.i2c_buses[0].sda.net;
        let eased = json_scenario(
            true,
            &[],
            json!([{"net": sda, "max_capacitance_femtofarads": 1}]),
        );
        assert_eq!(c(&eased), Seeded::Tolerable(Vec::new(), Vec::new()));
        // Adversary 2: a lowered pull-up only eases (F1); a stressing fault
        // beside an eased one covers nothing (F2); a held line or an absent
        // device expected to pass is never survived (F4).
        let lowered = json_scenario(true, &[], json!([{"bus": "SENSORS", "pull_up_ohms": 1000}]));
        assert_eq!(c(&lowered), Seeded::Tolerable(Vec::new(), Vec::new()));
        let mixed = json_scenario(
            true,
            &[],
            json!([{"bus": "SENSORS", "pull_up_ohms": 3300},
                {"net": sda, "max_capacitance_femtofarads": 1}]),
        );
        assert_eq!(c(&mixed), Seeded::Tolerable(Vec::new(), Vec::new()));
        for o in [
            json!([{"spi_device_absent": "U4"}]),
            json!([{"bus": "SENSORS", "line_held_low": "scl"}]),
        ] {
            assert_eq!(
                c(&json_scenario(true, &[], o)),
                Seeded::Tolerable(Vec::new(), Vec::new())
            );
        }
        let lowered_fail = json_scenario(
            false,
            &["i2c_timeout"],
            json!([{"bus": "SENSORS", "pull_up_ohms": 1000}]),
        );
        assert_eq!(c(&lowered_fail), Seeded::Other);
    }

    /// Ruled (a), driven by `cases.json`: each seeded case's measured
    /// attribution, given to the mechanism its finding kinds name. Only an
    /// unmutated electrical case is a detection: a mutant behind a weak
    /// pull-up (masked evidence), a software cause with the same kinds
    /// (`cs-never`, `S3`) and an indeterminate case are not. A subject beyond
    /// the seeded one is something else failing.
    #[test]
    fn only_an_unmutated_electrical_case_of_the_seeded_set_is_a_detection() {
        let (spec, t) = solved(S1);
        let mech = |f: &str| match classify(&spec, &t, &scenario_file(f)) {
            Seeded::Detect(_, m) => m,
            other => panic!("{f}: {other:?}"),
        };
        let rise = mech("scenarios/sentinel-01/pullup-47k.json");
        let held = mech("scenarios/sentinel-01/seeded/held-low-scl.json");
        let absent = mech(ABSENT_U4);
        let open = mech(OPEN_CONSOLE);
        let cases: Value = serde_json::from_slice(
            &std::fs::read(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../scenarios/sentinel-01/seeded/cases.json"),
            )
            .unwrap(),
        )
        .unwrap();
        let mut accepted = Vec::new();
        for case in cases["cases"].as_array().unwrap() {
            let findings: Vec<&str> = case["findings"]
                .as_array()
                .unwrap()
                .iter()
                .map(|f| f.as_str().unwrap())
                .collect();
            let Some(m) = [&rise, &held, &absent, &open]
                .into_iter()
                .find(|m| !findings.is_empty() && findings.iter().all(|k| m.kinds.contains(k)))
            else {
                continue;
            };
            let attributed = json!({
                "ok": case["domain"] != "indeterminate",
                "domain": case["domain"],
                "rule": case["rule"],
                "unexplained": [],
                "masked": case["masked"],
                "subjects": m.subjects.iter().collect::<Vec<_>>(),
            });
            let ok = detection(m, &failed(json!(findings)), &attributed).is_ok();
            let electrical = case["mutant"].is_null() && case["domain"] == "electrical";
            assert_eq!(ok, electrical, "{}", case["name"]);
            if ok {
                accepted.push(case["name"].as_str().unwrap());
            }
        }
        for want in [
            "spi-absent-u4",
            "serial-rx-open-console",
            "held-low-scl",
            "pullup-47k",
            "cap-220p-2k2",
        ] {
            assert!(accepted.contains(&want), "{want}: {accepted:?}");
        }
        let run = failed(json!(["spi_identity_mismatch"]));
        let a = |subjects: Value| {
            json!({"ok": true, "domain": "electrical", "rule": "E2", "unexplained": [],
                "masked": [], "subjects": subjects})
        };
        assert!(detection(&absent, &run, &a(json!(["device:U4"]))).is_ok());
        // An open RX line is detected by `E3`, with its port alone as subject.
        let echo = failed(json!(["serial_echo_missing"]));
        let e3 = |subjects: Value| {
            json!({"ok": true, "domain": "electrical", "rule": "E3", "unexplained": [],
                "masked": [], "subjects": subjects})
        };
        assert!(detection(&open, &echo, &e3(json!(["port:CONSOLE"]))).is_ok());
        assert!(detection(&open, &echo, &e3(json!(["port:CONSOLE", "port:GPS"]))).is_err());
        assert!(
            detection(
                &open,
                &echo,
                &json!({"ok": false, "domain": "indeterminate", "rule": "I6",
                    "unexplained": [], "masked": [], "subjects": ["port:CONSOLE"]})
            )
            .is_err()
        );
        assert!(detection(&absent, &run, &a(json!(["device:U4", "device:U2"]))).is_err());
        assert!(detection(&absent, &run, &a(json!([]))).is_err());
        assert!(detection(&rise, &run, &a(json!(["device:U4"]))).is_err());
        // A run that passed, or failed unmatched, is not a detection.
        let mut passed = failed(json!([]));
        passed.verdict = "pass".into();
        assert!(detection(&absent, &passed, &a(json!(["device:U4"]))).is_err());
        let mut unmatched = run;
        unmatched.matched = false;
        assert!(detection(&absent, &unmatched, &a(json!(["device:U4"]))).is_err());
    }

    /// `AET-WO-3305`: two-bme's held-low SDA as `attribute` measured it, on
    /// `main` (both witnesses masked: each read the other device's absent
    /// address as its own wrong one) and once each witness holds its own
    /// step's addresses (nothing masked; `spike/3305`). The first is refused,
    /// the second is the fault detected by its mechanism.
    #[test]
    fn a_bus_fault_with_two_devices_is_detected_once_nothing_is_masked() {
        let fixtures = "crates/aethelgard-cli/tests/fixtures";
        let (spec, t) = solved(&format!("{fixtures}/two-bme.json"));
        let held = match classify(
            &spec,
            &t,
            &scenario_file(&format!("{fixtures}/two-bme-held-low-sda.json")),
        ) {
            Seeded::Detect(_, m) => m,
            other => panic!("{other:?}"),
        };
        let run = failed(json!(["i2c_line_held_low", "i2c_timeout"]));
        let measured = |masked: Value| {
            json!({"ok": true, "domain": "electrical", "rule": "E1", "unexplained": [],
                "masked": masked, "subjects": ["bus:SENSORS", "device:U2", "device:U5"]})
        };
        let masked = json!([{"index": 28, "kind": 13}, {"index": 29, "kind": 13}]);
        assert!(
            detection(&held, &run, &measured(masked))
                .is_err_and(|e| e.contains("something else fails too"))
        );
        assert_eq!(detection(&held, &run, &measured(json!([]))), Ok(()));
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

    /// `AET-WO-1314b` §1.3: the extract gate's line in the verdict, when the
    /// gate fails a bound on a specification routed with no ladder, carries
    /// the finding's `cause` as `extract` printed it; a finding with none
    /// gains none.
    #[test]
    fn the_extract_gate_line_carries_the_ladder_cause_when_it_fails() {
        let finding = |cause: Option<&str>| {
            let mut f = json!({
                "kind": "capacitance_over_declared_bound",
                "severity": "error",
                "message": "`LOG_SCK`: bound 37508 fF (trace 15793, vias 12152, pads 1563, pins 8000) exceeds the declared 30000 fF",
            });
            if let Some(c) = cause {
                f["cause"] = c.into();
            }
            f
        };
        let line = |f: Value| {
            let g = gate_report(
                "extract",
                None,
                &Ran {
                    code: 1,
                    lines: vec![json!({"verdict": "fail", "findings": [f]})],
                },
            );
            assert_eq!(g.verdict, "fail");
            serde_json::to_value(&g).unwrap()
        };
        let cause = aethelgard_eda::extract::LADDER_NOT_RUN;
        let v = line(finding(Some(cause)));
        assert_eq!(v["findings"][0]["cause"], cause, "{v}");
        assert!(cause.contains("`board.stackup.fab_tolerances`"));
        let v = line(finding(None));
        assert!(v["findings"][0].get("cause").is_none(), "{v}");
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
            drawn: true,
            sch: "sch",
            pcb: "pcb",
            repl: "repl",
            board: "board",
            spec: "spec",
            harnesses: Vec::new(),
            boot_time: "100000000ns".to_owned(),
            bridges: Vec::new(),
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

    #[test]
    fn harnesses_are_read_from_the_generated_proofs() {
        let text = "#[cfg(kani)]\n#[kani::proof]\n#[kani::unwind(4)]\nfn every_valid_sequence() {}\nfn helper() {}\n#[kani::proof]\nfn i2c1_transfer() {}\n";
        assert_eq!(
            harnesses(text),
            ["proofs::every_valid_sequence", "proofs::i2c1_transfer"]
        );
    }

    // ---- AET-WO-1211c: a pin device's coverage and per-item gates.

    const D1: &str = "crates/aethelgard-ir/tests/fixtures/d1-drv8876.json";

    fn v5(
        expect_pass: bool,
        findings: &[&str],
        overrides: Value,
    ) -> aethelgard_chaos::campaign::Scenario {
        aethelgard_chaos::campaign::Scenario::parse(
            json!({"schema_version": 5, "name": "x", "base": "b.json",
                "expect": if expect_pass { "pass" } else { "fail" },
                "expect_findings": findings, "overrides": overrides})
            .to_string()
            .as_bytes(),
        )
        .unwrap()
    }

    /// A held nFAULT seeds no target at its active level (low): D5, ruled, no
    /// firmware reads it, so its gate would read back the twin's own
    /// stimulus. It changes nothing at its pull's level (high, R3 to
    /// VDD_3V3). A set IPROPI other than its idle's 0 V is a tolerable stress
    /// of its ADC input, and 0 mV changes nothing.
    #[test]
    fn a_held_input_seeds_nothing_and_a_set_adc_value_its_input() {
        let (spec, t) = solved(D1);
        let held = |level: &str, pass: bool, f: &[&str]| {
            classify(
                &spec,
                &t,
                &v5(
                    pass,
                    f,
                    json!([{"pin_held": {"net": "DRV_NFAULT", "level": level}}]),
                ),
            )
        };
        assert_eq!(
            held("low", false, &["pin_device_input_asserted"]),
            Seeded::Other
        );
        assert_eq!(held("high", true, &[]), Seeded::Baseline);
        // A hold or a value the twin refuses is no baseline (adversarial
        // F10): the campaign refuses its run.
        for o in [
            json!({"pin_held": {"net": "DRV_NSLEEP", "level": "high"}}),
            json!({"pin_held": {"net": "NOPE", "level": "low"}}),
            json!({"analog_value": {"net": "NOPE", "millivolts": 1}}),
        ] {
            assert_ne!(
                classify(&spec, &t, &v5(true, &[], json!([o.clone()]))),
                Seeded::Baseline,
                "{o}"
            );
        }
        // Held low and expected to pass: never a fault to survive.
        assert_eq!(
            held("low", true, &[]),
            Seeded::Tolerable(Vec::new(), Vec::new())
        );
        let set = |mv: u32| {
            classify(
                &spec,
                &t,
                &v5(
                    true,
                    &[],
                    json!([{"analog_value": {"net": "DRV_IPROPI", "millivolts": mv}}]),
                ),
            )
        };
        assert_eq!(
            set(2850),
            Seeded::Tolerable(vec![Target::Analog("DRV_IPROPI".into())], Vec::new())
        );
        assert_eq!(set(0), Seeded::Baseline);
    }

    /// D1's coverage: its ADC input is a target, unmet until a fault covers
    /// it; its input, its bridge and its motor rail are named and never
    /// block (D5, D2, D1, ruled); the rest is as every board's.
    #[test]
    fn a_pin_device_s_adc_input_is_a_target_and_its_input_bridge_and_motor_rail_are_named() {
        let (spec, t) = solved(D1);
        let base = || classify(&spec, &t, &v5(true, &[], json!([])));
        let c = coverage(&spec, &t, &[base()], None);
        assert_eq!(
            c.unmet,
            [
                "serial port `CMD`",
                "ADC input `DRV_IPROPI`",
                "rail `VDD_3V3`"
            ]
        );
        assert_eq!(c.no_device.len(), 3, "{:?}", c.no_device);
        for (k, start) in [
            "input `DRV_NFAULT` (no generated firmware path reads it",
            "bridge `M1` of `U2`",
            "rail `VMOTOR` (it supplies only pin devices' supply pins (U2.VM)",
        ]
        .into_iter()
        .enumerate()
        {
            assert!(c.no_device[k].starts_with(start), "{:?}", c.no_device);
        }
        let held = classify(
            &spec,
            &t,
            &v5(
                false,
                &["pin_device_input_asserted"],
                json!([{"pin_held": {"net": "DRV_NFAULT", "level": "low"}}]),
            ),
        );
        let set = classify(
            &spec,
            &t,
            &v5(
                true,
                &[],
                json!([{"analog_value": {"net": "DRV_IPROPI", "millivolts": 2850}}]),
            ),
        );
        let c = coverage(&spec, &t, &[base(), held.clone(), set.clone()], None);
        assert_eq!(c.unmet, ["serial port `CMD`", "rail `VDD_3V3`"]);
        // A survived value covers its input with no baseline too (ruled (a)).
        let c = coverage(&spec, &t, &[held, set], None);
        assert_eq!(c.unmet, ["serial port `CMD`", "rail `VDD_3V3`"]);
    }

    /// The motor rail's predicate catches a pin device's supply alone: none
    /// of the five committed rails with no consumer a judge observes (none
    /// feeds a pin device; `spike/1211c/RESULTS.md` P3d), and not a domain a
    /// serial port's RX line, a regulator or an SPI device is on.
    #[test]
    fn the_motor_rail_is_a_pin_device_s_supply_alone() {
        let (spec, d1) = solved(D1);
        assert!(motor_rail(&spec, &d1, "VMOTOR").is_some());
        assert!(motor_rail(&spec, &d1, "VDD_3V3").is_none());
        for (path, domain) in [
            (
                "crates/aethelgard-cli/tests/fixtures/connectors.json",
                "VDD_5V",
            ),
            (
                "crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json",
                "VDD_5V",
            ),
            (
                "crates/aethelgard-cli/tests/fixtures/hse-crystal-16.json",
                "VDD_1V8",
            ),
            (
                "crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json",
                "VDD_5V",
            ),
            (
                "crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json",
                "VDD_1V8",
            ),
        ] {
            let (spec, t) = solved(path);
            assert!(motor_rail(&spec, &t, domain).is_none(), "{path} {domain}");
        }
        // The same rail with an SPI device on it is no motor rail.
        let mut v: Value = serde_json::from_slice(
            &std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(D1)).unwrap(),
        )
        .unwrap();
        v["mcu"]["supply"] = json!("VDD_3V3");
        let mut spec: aethelgard_ir::Specification = serde_json::from_value(v).unwrap();
        assert!(motor_rail(&spec, &d1, "VMOTOR").is_some());
        spec.mcu.supply = "VMOTOR".into();
        assert!(
            motor_rail(&spec, &d1, "VMOTOR").is_none(),
            "the MCU's supply"
        );
        // A declared load on it is a consumer: no motor rail.
        let read = |path: &str| -> Value {
            serde_json::from_slice(
                &std::fs::read(
                    Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("../..")
                        .join(path),
                )
                .unwrap(),
            )
            .unwrap()
        };
        let mut v = read(D1);
        v["power"] =
            read("crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json")["power"].clone();
        let unloaded: aethelgard_ir::Specification = serde_json::from_value(v.clone()).unwrap();
        assert!(
            motor_rail(&unloaded, &d1, "VMOTOR").is_some(),
            "no load on it yet"
        );
        v["power"]["loads"][0]["domain"] = json!("VMOTOR");
        let loaded: aethelgard_ir::Specification = serde_json::from_value(v).unwrap();
        assert!(motor_rail(&loaded, &d1, "VMOTOR").is_none(), "a load on it");
    }

    fn sim_line(gpio: Value, adc: Value, motor: Value, findings: Value) -> Ran {
        Ran {
            code: 0,
            lines: vec![json!({"verdict": "pass", "tool": {"version": "r"},
                "sandbox": {"uid_boundary": true}, "input": {"sha256": "e"},
                "boot": {"gpio": gpio, "adc": adc, "motor": motor}, "findings": findings})],
        }
    }

    /// The per-item gates count one per item, never the whole input once
    /// (`2306`), pass only when every item does, and are absent with no item.
    #[test]
    fn the_per_item_gates_count_each_signal_and_fail_on_one() {
        let gpio = json!([
            {"net": "DRV_NSLEEP", "direction": "output", "status": "witnessed"},
            {"net": "DRV_NFAULT", "direction": "input", "status": "witnessed", "read": "high", "active": "low"}
        ]);
        let adc =
            json!([{"net": "DRV_IPROPI", "status": "witnessed", "expected": 0, "observed": 0}]);
        let motor = json!([
            {"label": "DRV_EN", "level": {"status": "witnessed"}},
            {"label": "DRV_PH", "level": {"status": "witnessed"}}
        ]);
        let bridge = [Bridge {
            id: "M1".into(),
            inputs: vec!["DRV_EN".into(), "DRV_PH".into()],
            enable: Some("DRV_NSLEEP".into()),
        }];
        let gates = pin_gates(
            &sim_line(gpio.clone(), adc.clone(), motor.clone(), json!([])),
            &bridge,
        );
        let summary: Vec<(&str, &str, u64)> = gates
            .iter()
            .map(|g| (g.gate, g.verdict, g.scope.as_ref().unwrap().judged))
            .collect();
        assert_eq!(
            summary,
            [
                ("gpio", "pass", 1),
                ("adc", "pass", 1),
                ("bridge", "pass", 1)
            ]
        );
        // nFAULT is named, never judged (D5; adversarial F4).
        let items = gates[0].items.as_ref().unwrap();
        assert_eq!(items[1]["verdict"], "not_judged");
        assert!(items[1]["reason"].as_str().unwrap().contains("INV-07"));
        // A gate whose only GPIO signal is an input judged nothing, and says
        // why (2306).
        let only = pin_gates(
            &sim_line(
                json!([gpio[1].clone()]),
                adc.clone(),
                motor.clone(),
                json!([]),
            ),
            &[],
        );
        let scope = serde_json::to_value(only[0].scope.as_ref().unwrap()).unwrap();
        assert_eq!(
            (scope["judged"].clone(), scope["nothing_in_scope"].clone()),
            (json!(0), json!(true))
        );
        // nFAULT read at its active level fails the gate, by its finding.
        let mut asserted = gpio.clone();
        asserted[1]["read"] = json!("low");
        let f = json!([{"kind": "pin_device_input_asserted", "severity": "error", "message": "m"},
            {"kind": "boot_word_missing", "severity": "error", "message": "m"}]);
        let gates = pin_gates(&sim_line(asserted, adc.clone(), motor.clone(), f), &bridge);
        assert_eq!(gates[0].verdict, "fail");
        let items = gates[0].items.as_ref().unwrap();
        assert_eq!(
            (items[0]["verdict"].clone(), items[1]["verdict"].clone()),
            (json!("pass"), json!("not_judged"))
        );
        assert_eq!(gates[0].findings.len(), 1, "its own kinds only");
        // A bridge input's level unwitnessed fails the bridge.
        let mut still = motor.clone();
        still[1]["level"]["status"] = json!("mismatch");
        let gates = pin_gates(&sim_line(gpio, adc, still, json!([])), &bridge);
        assert_eq!(gates[2].verdict, "fail");
        // No pin device: no gate.
        assert!(
            pin_gates(
                &sim_line(json!(null), json!(null), json!([]), json!([])),
                &[]
            )
            .is_empty()
        );
    }

    /// `AET-WO-2308` LP2 (`AET-WO-1909` LP2): the tail `verify` runs once
    /// its gates have run gives a failing verdict `EXIT_FAIL` with no fab
    /// flag, with `--fab-out` and with `--against-fab` alike, and the same
    /// verdict but for the package's keys; a passing one without a fab flag
    /// exits 0. The report is the reference's, as `verify` builds it, with
    /// one gate that did not pass. Nothing is written for the failing run.
    #[test]
    fn the_exit_code_is_the_verdicts_whatever_the_fab_mode() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sentinel-01-minimal.json");
        let Ok((spec, spec_sha256)) = crate::parse_spec(&std::fs::read(path).unwrap()) else {
            panic!("the reference parses");
        };
        let (topology, table) = crate::solve_spec(&spec, &spec_sha256).unwrap();
        let (files, topology_sha256) = crate::generated(&spec, &topology, &table).unwrap();
        let ledger = aethelgard_cli::ledger::build(&topology, &table).unwrap();
        let fresh = |gates: Vec<Gate>| Report {
            ok: false,
            verdict: "fail",
            reason: None,
            check: "verify",
            engine_version: env!("CARGO_PKG_VERSION"),
            project_name: spec.project_name.clone(),
            specification_sha256: spec_sha256.clone(),
            topology_sha256: topology_sha256.clone(),
            files: BTreeMap::new(),
            lifecycle: Vec::new(),
            against: None,
            elf_sha256: None,
            rise_times: RISE_TIMES,
            gates,
            judged_nothing: Vec::new(),
            support: None,
            campaign: None,
            findings: Vec::new(),
            declared_peers: Vec::new(),
            silicon: aethelgard_cli::ledger::silicon(&topology),
            ledger: ledger.clone(),
            twin_limits: Vec::new(),
            against_fab: None,
            fab: None,
            lifecycle_unheld: Vec::new(),
            conditions: Vec::new(),
            outcome: Vec::new(),
        };
        let bom = format!("{KICAD_DIR}/{}.bom.csv", spec.project_name);
        let scratch = std::env::temp_dir().join(format!("aet-2308-lp2-{}", std::process::id()));
        let tail = Tail {
            project: &spec.project_name,
            spec: &spec,
            spec_sha256: &spec_sha256,
            topology: &topology,
            board: scratch.join("board.kicad_pcb"),
            board_sha256: "",
            elf: scratch.join("firmware.elf"),
            bom: files[&bom].as_bytes(),
            expected: &[],
            seeded: &[],
            harnesses: &[],
        };
        let gate = |verdict: &'static str| {
            let mut g = not_run("erc");
            g.verdict = verdict;
            g
        };
        let out = scratch.join("pkg");
        let against = scratch.join("against");
        std::fs::create_dir_all(&against).unwrap();
        let mut lines = Vec::new();
        for mode in [
            None,
            Some(FabMode::Write(out.clone())),
            Some(FabMode::Compare(against.clone())),
        ] {
            let mut report = fresh(vec![gate("fail")]);
            let code = conclude(&mut report, mode.as_ref(), &tail)
                .unwrap_or_else(|_| panic!("a failing verdict is concluded, not refused"));
            assert_eq!(
                code, EXIT_FAIL,
                "a failing verdict exits EXIT_FAIL in every fab mode"
            );
            assert_eq!(report.verdict, "fail");
            report.fab = None;
            report.against_fab = None;
            lines.push(serde_json::to_string(&report).unwrap());
        }
        assert!(
            lines.windows(2).all(|w| w[0] == w[1]),
            "one verdict but for the package"
        );
        assert!(!out.exists(), "a failing run writes nothing");
        assert_eq!(std::fs::read_dir(&against).unwrap().count(), 0);
        let mut report = fresh(vec![gate("pass")]);
        assert_eq!(conclude(&mut report, None, &tail).ok(), Some(0));
        std::fs::remove_dir_all(&scratch).unwrap();
    }

    /// `AET-WO-2308` L3: the campaign gate's version is the Renode build its
    /// runs' archives name, read back by replay. Two runs naming one build
    /// give it; two builds, a run naming none, no run, and a run with no
    /// archive are each a `ToolError`, never a choice or a gap.
    #[test]
    fn the_campaign_names_the_renode_build_every_run_archive_names() {
        let dir = std::env::temp_dir().join(format!("aet-2308-l3-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let renode = aethelgard_chaos::campaign::RENODE;
        let run = |n: u8| Run {
            scenario: format!("s{n}"),
            verdict: "pass".to_owned(),
            expect: "pass".to_owned(),
            matched: true,
            findings: Value::Null,
            run_id: hex([n; 16]),
            state_sequence_hash: String::new(),
            battery_sag: Value::Null,
            observation_witness: Value::Null,
            conditions: Vec::new(),
        };
        for (n, tools) in [
            (1, vec![("aethelgard", "0.2.0"), (renode, "build-a")]),
            (2, vec![(renode, "build-a")]),
            (3, vec![(renode, "build-b")]),
            (4, vec![("aethelgard", "0.2.0")]),
        ] {
            let header = aethelgard_chaos::Header {
                run_id: [n; 16],
                artifacts: Vec::new(),
                tools: tools
                    .iter()
                    .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
                    .collect(),
                tool_sha256: Vec::new(),
            };
            aethelgard_chaos::TraceWriter::create(&dir.join(hex([n; 16])), &header)
                .unwrap()
                .commit()
                .unwrap();
        }
        assert_eq!(
            campaign_renode(&dir, &[run(1), run(2)]).ok(),
            Some("build-a".to_owned())
        );
        for runs in [
            vec![run(1), run(3)],
            vec![run(2), run(4)],
            vec![run(4)],
            Vec::new(),
            vec![run(9)],
        ] {
            assert!(campaign_renode(&dir, &runs).is_err(), "refused");
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
