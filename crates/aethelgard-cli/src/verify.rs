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

/// The virtual time `simulate` boots for: the campaign's.
const BOOT_TIME: &str = "100ms";

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
}

impl FabReport {
    fn none(status: &'static str) -> Self {
        Self {
            status,
            tool_version: None,
            uid_boundary: None,
            uid_boundary_unmet: None,
            board_sha256: None,
            files: BTreeMap::new(),
            differences: Vec::new(),
        }
    }
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
        against: None,
        elf_sha256: None,
        rise_times: RISE_TIMES,
        gates: Vec::new(),
        campaign: None,
        findings: Vec::new(),
        against_fab: None,
        fab: None,
    };

    // Every scenario must start from exactly `--spec`, or its campaign would
    // judge another specification.
    let mut expected = Vec::new();
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
    let board = tree::subtree(&files, &format!("{BOARD_DIR}/"));
    let board_sha256 = tree::crate_sha256(&board);
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
            harnesses: harnesses(board.get("src/proofs.rs").map_or("", String::as_str)),
        },
        &expected,
    ) {
        return code;
    }
    report.ok = report.findings.is_empty() && report.gates.iter().all(|g| g.verdict == "pass");
    report.verdict = if report.ok { "pass" } else { "fail" };
    if let Some(mode) = fab_mode {
        let elf_sha256 = report.elf_sha256.clone().unwrap_or_default();
        let bound = fab::Bound {
            project: &project,
            spec: &spec,
            specification_sha256: &spec_sha256,
            board: &root.join(&pcb),
            board_sha256: &hashes[&pcb],
            elf: &scratch.tool.join("firmware.elf"),
            elf_sha256: &elf_sha256,
            bom: files[&format!("{KICAD_DIR}/{project}.bom.csv")].as_bytes(),
        };
        let scenarios: Vec<fab::Scenario> = expected
            .iter()
            .map(|e| fab::Scenario {
                scenario: e.name.clone(),
                sha256: hex(e.scenario),
            })
            .collect();
        if let Err(code) = package(&mut report, mode, &bound, &scenarios) {
            return code;
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
    report.fab = Some(FabReport::none(status));
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
        tool_version: Some(exported.tool_version.clone()),
        uid_boundary: Some(exported.sandbox.uid_boundary),
        uid_boundary_unmet: exported.sandbox.uid_boundary_unmet,
        board_sha256: Some(bound.board_sha256.to_owned()),
        files,
        differences: Vec::new(),
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
            &BOOT_TIME,
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
