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

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use serde::Serialize;
use serde_json::Value;

use aethelgard_cli::generate::{BOARD_DIR, KICAD_DIR};

use crate::campaign;
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
        input_sha256: v["input"]["sha256"].as_str().map(str::to_owned),
        harness_provenance: v["harness_provenance"].as_str().map(str::to_owned),
        cover_count: v["cover_count"].as_u64(),
        findings: v["findings"].as_array().cloned().unwrap_or_default(),
    }
}

fn not_run(name: &'static str) -> Gate {
    Gate {
        gate: name,
        harness: None,
        verdict: "not_run",
        exit_code: None,
        tool_version: None,
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
    scenario: [u8; 32],
    specification: [u8; 32],
}

/// `aethelgard verify --spec <file> [--scenarios <file>...] [--against <dir>]`.
pub fn verify(spec_path: &Path, scenarios: &[PathBuf], against: Option<&Path>) -> ExitCode {
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
            scenario: scenario.sha256,
            specification: effective.specification_sha256,
        });
    }

    // `--against` first: a directory that is not the tree fails, naming
    // every difference, and no gate runs.
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
    let code = if report.ok { 0 } else { EXIT_FAIL };
    crate::emit(&report, code)
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

/// ERC, DRC with parity, `prove` of every harness, `build`, `simulate
/// --firmware` and the campaign, in that order, each bound to the generated
/// hashes. A refusal or a binding mismatch stops the run.
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
        assert_eq!(fields, 11);
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
