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
    /// The support gate's record (`AET-WO-4300g`): each role, strap and
    /// condition as `check support` judged them; absent when it did not run.
    #[serde(skip_serializing_if = "Option::is_none")]
    support: Option<Value>,
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
        fab::Precondition::of(
            "mcu_support_network_judged",
            match report.gates.iter().find(|g| g.gate == "support") {
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
            },
        ),
        fab::Precondition::of("every_gate_passed", failed),
    ]
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
        scenario: None,
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
        scenario: None,
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
    Rail(String),
}

impl Target {
    fn name(&self) -> String {
        match self {
            Self::I2c(b) => format!("I2C bus `{b}`"),
            Self::Spi(b) => format!("SPI bus `{b}`"),
            Self::Rail(r) => format!("rail `{r}`"),
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
}

/// Each override kind's mechanism, a closed table: the engine's, never the
/// scenario's own `expect_findings`.
const RISE: &[&str] = &["analog_rise_exceeds_ceiling", "i2c_timeout"];
const HELD: &[&str] = &["i2c_line_held_low", "i2c_timeout"];
const ABSENT: &[&str] = &["spi_identity_mismatch", "spi_transaction_unwitnessed"];

/// What one scenario seeds, judged against the specification.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Seeded {
    /// Expects a pass and every override changes nothing: the no-fault baseline.
    Baseline,
    /// Expects a pass: the design must survive each of these.
    Tolerable(Vec<Target>),
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
                }),
            ))
        }
        Override::Droop(d) => {
            // Below the domain's tolerance band: inside it the rail stays in
            // its specification (§4 F3). A droop names no mechanism: an
            // expected-fail droop is judged as before.
            let v = spec.voltage_domains.iter().find(|v| v.id == d.domain)?;
            let band =
                u64::from(v.nominal_millivolts) * (1000 - u64::from(v.tolerance_permille)) / 1000;
            Some(if u64::from(d.floor_millivolts) < band {
                (Some(Target::Rail(d.domain.clone())), None)
            } else {
                (None, None)
            })
        }
    }
}

/// What `scenario` seeds (`AET-WO-3303` D1, ruled (a)).
fn classify(
    spec: &aethelgard_ir::Specification,
    topology: &aethelgard_ir::SolvedTopology,
    scenario: &aethelgard_chaos::campaign::Scenario,
) -> Seeded {
    let changed: Vec<_> = scenario
        .overrides
        .iter()
        .filter_map(|o| stress(spec, topology, o))
        .collect();
    if scenario.expect_pass {
        if changed.is_empty() {
            return Seeded::Baseline;
        }
        // A scenario that also eases something covers nothing: its net change
        // may ease the target it stresses (adversary 2, F2). A held line or an
        // absent device is never a fault to survive: an absent IMU or flash
        // must be caught, not survived (ruled (a); adversary 2, F4).
        use aethelgard_chaos::record::Override;
        let intolerable = scenario.overrides.iter().any(|o| {
            matches!(
                o,
                Override::LineHeldLow { .. } | Override::SpiDeviceAbsent { .. }
            )
        });
        if intolerable || changed.iter().any(|(t, _)| t.is_none()) {
            return Seeded::Tolerable(Vec::new());
        }
        return Seeded::Tolerable(changed.into_iter().filter_map(|(t, _)| t).collect());
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

/// The campaign's coverage of every declared bus and rail.
#[derive(Debug, Default, PartialEq, Eq)]
struct Coverage {
    /// Each bus or rail no fault covers, named with why where no kind can.
    unmet: Vec<String>,
    /// Each bus no device uses: never `met`, and it does not block (ruled (c)).
    no_device: Vec<String>,
}

/// Coverage, in the specification's order: I2C buses, SPI buses, then rails
/// (each voltage domain, and the battery when a power tree is declared).
/// Before any gate `outcome` is `None` and a seeded fault counts as it is
/// declared; after the gates `outcome[i]` says whether scenario `i`'s run
/// passed the gate, and only those count. A tolerable fault covers its
/// target; a detected one covers its bus only when the campaign holds a
/// baseline (ruled (a)). Rails other than the MCU's supply and the battery
/// stay `unmet` until a fault kind reaches them (ruled (b)).
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
            Seeded::Tolerable(targets) if ran(i) => covered.extend(targets),
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
    for v in &spec.voltage_domains {
        let target = Target::Rail(v.id.clone());
        if v.id != spec.mcu.supply {
            c.unmet.push(format!(
                "{} (not the MCU's supply: no fault kind droops it)",
                target.name()
            ));
        } else if !covered.contains(&target) {
            c.unmet.push(target.name());
        }
    }
    if spec.power.is_some() {
        c.unmet
            .push("rail `battery` (no fault kind droops it)".to_owned());
    }
    c
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
        support: None,
        campaign: None,
        findings: Vec::new(),
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
    }

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
            support: &support_lines(report.support.as_ref()),
        };
        let scenarios: Vec<fab::Scenario> = expected
            .iter()
            .map(|e| fab::Scenario {
                scenario: e.name.clone(),
                sha256: hex(e.scenario),
            })
            .collect();
        let outcome = report.outcome.clone();
        let short = coverage(&spec, &topology, &seeded, Some(&outcome));
        let pre = preconditions(&report, &harnesses, short);
        if let Err(code) = package(&mut report, mode, &bound, &scenarios, &pre) {
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
    report.support = support.last().get("support").cloned();
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

    fn json_scenario(
        expect_pass: bool,
        findings: &[&str],
        overrides: Value,
    ) -> aethelgard_chaos::campaign::Scenario {
        aethelgard_chaos::campaign::Scenario::parse(
            json!({"schema_version": 2, "name": "x", "base": "b.json",
                "expect": if expect_pass { "pass" } else { "fail" },
                "expect_findings": findings, "overrides": overrides})
            .to_string()
            .as_bytes(),
        )
        .unwrap()
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
        }
    }

    const S1: &str = "examples/sentinel-01-minimal.json";
    const BASELINE: &str = "scenarios/sentinel-01/pullup-2k2.json";
    const TOLERATED_I2C: &str = "scenarios/sentinel-01/seeded/pullup-3k3.json";
    const RAIL: &str = "scenarios/sentinel-01/droop-bor3.json";
    const ABSENT_U4: &str = "scenarios/sentinel-01/seeded/spi-absent-u4.json";

    /// `AET-WO-3303` D1, ruled: each declared bus and rail is named until a
    /// fault covers it. A tolerable fault covers its target; a detected one
    /// its bus only beside a baseline; before the gates as declared, after
    /// them only when the run passed the gate. The reference's seeded set
    /// covers everything.
    #[test]
    fn coverage_needs_a_survived_or_a_detected_fault_beside_a_baseline() {
        let (spec, t) = solved(S1);
        let all = ["I2C bus `SENSORS`", "SPI bus `FLASH`", "rail `VDD_3V3`"];
        let unmet = |files: &[&str], outcome: Option<&[bool]>| {
            coverage(&spec, &t, &seeded(&spec, &t, files), outcome).unmet
        };
        assert_eq!(unmet(&[BASELINE], None), all);
        assert_eq!(unmet(&[TOLERATED_I2C, RAIL], None), ["SPI bus `FLASH`"]);
        // A detected fault without a baseline covers nothing.
        assert_eq!(
            unmet(&[TOLERATED_I2C, RAIL, ABSENT_U4], None),
            ["SPI bus `FLASH`"]
        );
        let set = [TOLERATED_I2C, RAIL, ABSENT_U4, BASELINE];
        assert!(unmet(&set, None).is_empty());
        assert!(unmet(&set, Some(&[true; 4])).is_empty());
        // After the gates, a detection that failed covers nothing, nor does
        // one beside a baseline that failed.
        assert_eq!(
            unmet(&set, Some(&[true, true, false, true])),
            ["SPI bus `FLASH`"]
        );
        assert_eq!(
            unmet(&set, Some(&[true, true, true, false])),
            ["SPI bus `FLASH`"]
        );
        // A droop inside the tolerance band covers nothing (§4 F3).
        assert_eq!(
            unmet(
                &["scenarios/sentinel-01/droop-tolerated.json", TOLERATED_I2C],
                None
            ),
            ["SPI bus `FLASH`", "rail `VDD_3V3`"]
        );
    }

    /// Ruled (b) and (c): a rail other than the MCU's supply stays `unmet`
    /// whatever is seeded; a bus no device uses is named as having nothing to
    /// fault and does not block.
    #[test]
    fn a_rail_no_kind_reaches_is_unmet_and_a_deviceless_bus_does_not_block() {
        let (spec, t) = solved("crates/aethelgard-cli/tests/fixtures/connectors.json");
        let faults = json_scenario(
            true,
            &[],
            json!([{"bus": "SENSORS", "pull_up_ohms": 3300},
                {"droop": {"domain": "VDD_3V3", "floor_millivolts": 2400, "start_ns": 1000000,
                    "fall_ns": 20000, "hold_ns": 200000, "rise_ns": 20000,
                    "source_milliohms": 500, "load_milliamps": 100, "bor_level": 0}}]),
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
        assert_eq!(
            c.unmet,
            ["rail `VDD_5V` (not the MCU's supply: no fault kind droops it)"]
        );
        assert_eq!(c.no_device, ["SPI bus `EXT` (no device: nothing to fault)"]);
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
            json!([{"droop": {"domain": "VDD_3V3", "floor_millivolts": 2400, "start_ns": 1000000,
                "fall_ns": 20000, "hold_ns": 200000, "rise_ns": 20000, "source_milliohms": 500,
                "load_milliamps": 100, "bor_level": 3}}]),
        );
        assert_eq!(c(&droop), Seeded::Other);
        let sda = &spec.i2c_buses[0].sda.net;
        let eased = json_scenario(
            true,
            &[],
            json!([{"net": sda, "max_capacitance_femtofarads": 1}]),
        );
        assert_eq!(c(&eased), Seeded::Tolerable(Vec::new()));
        // Adversary 2: a lowered pull-up only eases (F1); a stressing fault
        // beside an eased one covers nothing (F2); a held line or an absent
        // device expected to pass is never survived (F4).
        let lowered = json_scenario(true, &[], json!([{"bus": "SENSORS", "pull_up_ohms": 1000}]));
        assert_eq!(c(&lowered), Seeded::Tolerable(Vec::new()));
        let mixed = json_scenario(
            true,
            &[],
            json!([{"bus": "SENSORS", "pull_up_ohms": 3300},
                {"net": sda, "max_capacitance_femtofarads": 1}]),
        );
        assert_eq!(c(&mixed), Seeded::Tolerable(Vec::new()));
        for o in [
            json!([{"spi_device_absent": "U4"}]),
            json!([{"bus": "SENSORS", "line_held_low": "scl"}]),
        ] {
            assert_eq!(
                c(&json_scenario(true, &[], o)),
                Seeded::Tolerable(Vec::new())
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
            let Some(m) = [&rise, &held, &absent]
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

    #[test]
    fn harnesses_are_read_from_the_generated_proofs() {
        let text = "#[cfg(kani)]\n#[kani::proof]\n#[kani::unwind(4)]\nfn every_valid_sequence() {}\nfn helper() {}\n#[kani::proof]\nfn i2c1_transfer() {}\n";
        assert_eq!(
            harnesses(text),
            ["proofs::every_valid_sequence", "proofs::i2c1_transfer"]
        );
    }
}
