//! The `aethelgard` binary: subcommands over the crates and the external
//! tools, one JSON object out, documented exit codes (`docs/architecture.md`
//! `RUL-10`). The codes are listed in this crate's `README.md`.

#![forbid(unsafe_code)]

mod input;
mod kani;
mod kicad;
mod mcp;
mod renode;
mod run;
mod said;
mod verdict;

use std::ffi::OsString;
use std::fs::{self, DirBuilder, File};
use std::io::{self, Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use aethelgard_ir::Specification;
use clap::error::ErrorKind;
use clap::{Parser, Subcommand};
use serde::Serialize;
use serde_json::error::Category;

use input::MAX_INPUT_BYTES;

const EXIT_FAIL: u8 = 1;
const EXIT_INVALID_SPEC: u8 = 2;
const EXIT_UNREADABLE: u8 = 3;
const EXIT_TOOL_UNAVAILABLE: u8 = 4;
const EXIT_TOOL_ERROR: u8 = 5;
const EXIT_USAGE: u8 = 64;
const EXIT_INTERNAL: u8 = 70;

#[derive(Parser)]
#[command(name = "aethelgard", version, about, arg_required_else_help = false)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate one JSON Specification read from stdin, or from --file.
    Validate {
        /// Read the specification from this file instead of stdin.
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// Solve one Specification's pins and print the SolvedTopology.
    Solve {
        /// The specification file.
        #[arg(long)]
        file: PathBuf,
    },
    /// Validate and solve one Specification, then write its KiCad schematic, bill of materials and placed board into a directory.
    Emit {
        /// The specification file.
        #[arg(long)]
        file: PathBuf,
        /// The directory to write <project_name>.kicad_sch, <project_name>.bom.csv and <project_name>.kicad_pcb into; created if absent.
        #[arg(long)]
        out: PathBuf,
    },
    /// Run a KiCad check on a board or schematic and print its verdict.
    Check {
        #[command(subcommand)]
        check: CheckCommand,
    },
    /// Prove one Kani harness in a crate and print the verdict.
    Prove {
        /// The crate directory. It is copied, minus a top-level target/, and
        /// the copy is judged.
        #[arg(long = "crate")]
        crate_dir: PathBuf,
        /// The harness to prove: its name, or its path such as proofs::add_in_range.
        /// It must name exactly one harness.
        #[arg(long, value_parser = harness_name)]
        harness: String,
    },
    /// Load a platform in Renode, list the machine it built, and print the verdict.
    Simulate {
        /// A .repl file, or @path inside Renode's own tree, such as
        /// @platforms/boards/nucleo_h753zi.repl.
        #[arg(long)]
        platform: PathBuf,
    },
    /// Serve every other subcommand as an MCP tool on stdin and stdout.
    Mcp,
}

#[derive(Subcommand)]
enum CheckCommand {
    /// KiCad's design rule check on a board (kicad-cli pcb drc).
    Drc {
        /// The .kicad_pcb file.
        #[arg(long)]
        board: PathBuf,
        /// Judge by the rules this specification implies; without it, by KiCad's defaults. Either way the gate writes every rule file itself.
        #[arg(long)]
        spec: Option<PathBuf>,
        /// Also check the board against this .kicad_sch (KiCad's schematic parity). It is copied beside the board and judged with it.
        #[arg(long)]
        schematic: Option<PathBuf>,
    },
    /// KiCad's electrical rule check on a schematic (kicad-cli sch erc).
    Erc {
        /// The .kicad_sch file.
        #[arg(long)]
        schematic: PathBuf,
    },
}

#[derive(Serialize)]
struct ValidReport<'a> {
    ok: bool,
    project_name: &'a str,
    voltage_domains: usize,
    net_constraints: usize,
}

#[derive(Serialize)]
struct Refusal {
    ok: bool,
    error: RefusalBody,
}

#[derive(Serialize)]
struct RefusalBody {
    kind: &'static str,
    id: Option<String>,
    /// The JSON path of the refused field (`InvalidIdentifier` only); absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    message: String,
}

impl Refusal {
    fn new(kind: &'static str, id: Option<&str>, message: String) -> Self {
        Self {
            ok: false,
            error: RefusalBody {
                kind,
                id: id.map(str::to_owned),
                path: None,
                message,
            },
        }
    }

    fn at(mut self, path: Option<&str>) -> Self {
        self.error.path = path.map(str::to_owned);
        self
    }
}

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().collect();
    match args.get(1).and_then(|a| a.to_str()) {
        Some(run::REAPER_ARG) => run::reaper_main(&args[2..]),
        Some(run::CONFINE_ARG) => run::confine_main(&args[2..]),
        Some(run::GUARD_ARG) => run::guard_main(&args[2..]),
        _ => {}
    }
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err)
            if matches!(
                err.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            // Help and version are the only output that is not JSON.
            return match err.print() {
                Ok(()) => ExitCode::SUCCESS,
                Err(_) => ExitCode::from(EXIT_INTERNAL),
            };
        }
        Err(err) => {
            let rendered = err.render().to_string();
            diagnose(rendered.trim_end());
            let first = rendered.lines().next().unwrap_or_default();
            let message = first.strip_prefix("error: ").unwrap_or(first).to_owned();
            return emit(&Refusal::new("UsageError", None, message), EXIT_USAGE);
        }
    };
    // Only a CLI that `mcp` started dies with its parent (ruled on PR #34): Linux
    // sends the signal when the parent *thread* ends, which would kill a CLI that
    // any other multi-threaded caller runs, and `mcp` starts every CLI from its
    // main thread. `mcp` never arms it on itself, nor do the reaper and guardian,
    // which must outlive this process to clean up after it.
    if let Some(named) = std::env::var_os(run::PARENT_PID_VAR)
        && !matches!(cli.command, Command::Mcp)
    {
        let Some(parent) = run::pid_arg(&named) else {
            let message = format!("{} is not a pid", run::PARENT_PID_VAR);
            return emit(&Refusal::new("UsageError", None, message), EXIT_USAGE);
        };
        if let Err(e) =
            rustix::process::set_parent_process_death_signal(Some(rustix::process::Signal::KILL))
        {
            diagnose(&format!("cannot set the parent-death signal: {e}"));
            return ExitCode::from(EXIT_INTERNAL);
        }
        // A parent that died before the signal was armed never sends it.
        if std::os::unix::process::parent_id() != parent {
            diagnose("the process that started this one is gone");
            return ExitCode::from(EXIT_INTERNAL);
        }
    }

    let judged = match cli.command {
        Command::Validate { file } => return validate(file.as_deref()),
        Command::Solve { file } => return solve(&file),
        Command::Emit { file, out } => return emit_design(&file, &out),
        Command::Check {
            check:
                CheckCommand::Drc {
                    board,
                    spec,
                    schematic,
                },
        } => check_kicad(
            kicad::Check::Drc,
            &board,
            spec.as_deref(),
            schematic.as_deref(),
        ),
        Command::Check {
            check: CheckCommand::Erc { schematic },
        } => check_kicad(kicad::Check::Erc, &schematic, None, None),
        Command::Prove { crate_dir, harness } => prove(&crate_dir, &harness),
        Command::Simulate { platform } => simulate(&platform),
        Command::Mcp => return mcp::serve(),
    };
    match judged {
        Ok(v) => {
            let code = if v.ok { 0 } else { EXIT_FAIL };
            emit(&v, code)
        }
        Err(stop) => {
            let code = stop.code;
            emit(
                &Refusal::new(stop.kind, stop.id.as_deref(), stop.message),
                code,
            )
        }
    }
}

/// A harness name is a Rust path: letters, digits, `_` and `::`. Anything
/// else could not name a harness, and could be read by Kani as something else.
fn harness_name(s: &str) -> Result<String, String> {
    let ok = !s.is_empty()
        && s.split("::").all(|part| {
            !part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        });
    if ok {
        Ok(s.to_owned())
    } else {
        Err("a harness is a Rust path such as add_in_range or proofs::add_in_range".to_owned())
    }
}

/// The call's private temporary directory: `root/status-*` for the reaper,
/// `root/tool/` for everything the tool may write. Removed on drop.
struct Scratch {
    root: PathBuf,
    tool: PathBuf,
}

impl Scratch {
    fn new() -> Result<Self, run::Stop> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!("aethelgard-{}-{nanos}", std::process::id()));
        let fail = |e: io::Error| {
            run::Stop::tool_error(
                "aethelgard",
                format!("cannot create a temporary directory: {e}"),
            )
        };
        // The guardian starts before the directory exists, so no instant of this
        // process's death leaves the directory unguarded. It removes the
        // directory if this process dies first.
        let mut guardian = std::env::current_exe()
            .and_then(|me| {
                std::process::Command::new(me)
                    .arg(run::GUARD_ARG)
                    .arg(run::CLI_PID_ARG)
                    .arg(std::process::id().to_string())
                    .arg(&root)
                    .env_clear()
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .process_group(0)
                    .spawn()
            })
            .map_err(fail)?;
        if let Err(e) = DirBuilder::new().mode(0o700).create(&root) {
            // Not ours, if it already existed: the guardian must not remove it.
            let _ = guardian.kill();
            let _ = guardian.wait();
            return Err(fail(e));
        }
        let scratch = Self {
            tool: root.join("tool"),
            root,
        };
        DirBuilder::new()
            .mode(0o700)
            .create(&scratch.tool)
            .map_err(fail)?;
        Ok(scratch)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Asks the tool its version, in the same sandbox as the judging run. A tool
/// that will not say is unusable (exit 4).
fn version(inv: run::Invocation<'_>) -> Result<String, run::Stop> {
    let name = inv.tool.name;
    let out = inv.run()?;
    match (out.ended, out.first_line()) {
        (run::Ended::Code(0), Some(v)) => Ok(v),
        (ended, _) => Err(run::Stop::unusable(
            name,
            format!("`{name}` did not report a version ({ended:?})"),
        )),
    }
}

fn check_kicad(
    check: kicad::Check,
    path: &Path,
    spec: Option<&Path>,
    schematic: Option<&Path>,
) -> Result<verdict::Verdict, run::Stop> {
    let original = input::file_check(path)?;
    let (rules, spec_sha256) = match spec {
        Some(file) => {
            let (spec, sha256) = read_spec(Some(file)).map_err(|(refusal, code)| run::Stop {
                code,
                kind: refusal.error.kind,
                id: refusal.error.id,
                message: refusal.error.message,
            })?;
            (
                aethelgard_eda::rules::Rules::from_specification(&spec),
                Some(sha256),
            )
        }
        None => (aethelgard_eda::rules::Rules::kicad_default(), None),
    };
    // The copy is scanned and prepared before the tool is looked for, so a
    // board carrying its own rules is refused whatever is installed, and the
    // bytes judged are the bytes scanned.
    let scratch = Scratch::new()?;
    let input = input::file(&original, &scratch.tool.join(check.copy_name()))?;
    kicad::prepare(check, &input.copy, &scratch.tool, &rules)?;
    // KiCad's parity check reads the schematic named like the board, beside
    // it; the copy is scanned like the board before any tool runs.
    let schematic = match schematic {
        Some(path) => {
            let copy = input::file(path, &scratch.tool.join(kicad::Check::Erc.copy_name()))?;
            kicad::prepare_schematic(&copy.copy)?;
            Some(copy)
        }
        None => None,
    };
    let tool = run::Tool::find("kicad-cli")?;
    let invocation = |args: Vec<OsString>, status: &str| run::Invocation {
        tool: &tool,
        args,
        env: Vec::new(),
        workdir: &scratch.tool,
        cwd: scratch.tool.clone(),
        status: scratch.root.join(status),
        read_only: tool.dirs(),
        read_write: Vec::new(),
    };
    let version = version(invocation(run::args(["version"]), "status-version"))?;
    let report = scratch.tool.join("report.json");
    let [noun, verb] = check.verb();
    let mut args = run::args([noun, verb, "--exit-code-violations", "--format", "json"]);
    // Zones are emitted unfilled; KiCad fills them in memory to judge, and
    // the board is never saved, because its saved fills are not
    // deterministic (`RUL-03` as amended 2026-11-08).
    if matches!(check, kicad::Check::Drc) {
        args.push("--refill-zones".into());
    }
    if schematic.is_some() {
        args.push("--schematic-parity".into());
    }
    args.push("-o".into());
    args.push(report.clone().into_os_string());
    args.push(input.copy.clone().into_os_string());
    let out = invocation(args, "status-run").run()?;
    let code = out.code("kicad-cli")?;
    let findings = kicad::judge(check, code, &report, &version)?;
    let mut v = verdict::Verdict::new(
        check.name(),
        &input,
        &tool,
        version,
        code,
        out.sandbox,
        findings,
    );
    v.included_severities = Some(&kicad::SEVERITIES);
    if matches!(check, kicad::Check::Drc) {
        v.rules = Some(rules.source.name());
        v.specification_sha256 = spec_sha256;
        v.schematic = schematic.map(|s| verdict::InputReport {
            path: s.original.display().to_string(),
            sha256: s.sha256,
        });
    }
    Ok(v)
}

fn prove(crate_dir: &Path, harness: &str) -> Result<verdict::Verdict, run::Stop> {
    input::dir_check(crate_dir)?;
    let cargo = run::Tool::find("cargo")?;
    let cargo_kani = run::Tool::find("cargo-kani")?;
    let kani_home = run::required_var("KANI_HOME", true)?;
    let rustup_home = run::required_var("RUSTUP_HOME", true)?;
    let path_var = run::required_var("PATH", false)?;
    let scratch = Scratch::new()?;
    let input = input::crate_dir(crate_dir, &scratch.tool.join("crate"))?;
    let mut read_only = cargo.dirs();
    read_only.extend(cargo_kani.dirs());
    read_only.push(PathBuf::from(&kani_home));
    read_only.push(PathBuf::from(&rustup_home));
    let env = vec![
        ("KANI_HOME", kani_home.clone()),
        ("RUSTUP_HOME", rustup_home.clone()),
        ("PATH", path_var.clone()),
        (
            "CARGO_TARGET_DIR",
            scratch.tool.join("target").into_os_string(),
        ),
    ];
    let invocation = |args: Vec<OsString>, cwd: &Path, status: &str| run::Invocation {
        tool: &cargo,
        args,
        env: env.clone(),
        workdir: &scratch.tool,
        cwd: cwd.to_path_buf(),
        status: scratch.root.join(status),
        read_only: read_only.clone(),
        // Cargo starts its own children with stdin from /dev/null.
        read_write: vec![PathBuf::from("/dev/null")],
    };
    let version = version(invocation(
        run::args(["kani", "--version"]),
        &scratch.tool,
        "status-version",
    ))?;
    let mut run_args = run::args(["kani", "--harness"]);
    run_args.push(harness.into());
    // A path names exactly one harness only with --exact, because --harness
    // alone matches substrings; a bare name is checked against what ran.
    if harness.contains("::") {
        run_args.push("--exact".into());
    }
    let out = invocation(run_args, &input.copy, "status-run").run()?;
    let code = out.code("cargo-kani")?;
    let findings = kani::judge(code, &out, harness)?;
    let mut v = verdict::Verdict::new(
        "prove",
        &input,
        &cargo_kani,
        version,
        code,
        out.sandbox,
        findings,
    );
    v.harness = Some(harness.to_owned());
    Ok(v)
}

fn simulate(platform: &Path) -> Result<verdict::Verdict, run::Stop> {
    let at_path = platform.as_os_str().as_encoded_bytes().starts_with(b"@");
    if !at_path {
        input::file_check(platform)?;
    }
    let tool = run::Tool::find("renode")?;
    let root = fs::canonicalize(&tool.path)
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .ok_or_else(|| {
            run::Stop::unusable("renode", "cannot resolve renode's installation".to_owned())
        })?;
    let scratch = Scratch::new()?;
    let input = input::platform(platform, &root, &scratch.tool.join("platform.repl"))?;
    let mut read_only = tool.dirs();
    read_only.extend([
        root.clone(),
        PathBuf::from("/proc"),
        PathBuf::from("/dev/urandom"),
    ]);
    let invocation = |args: Vec<OsString>, status: &str| run::Invocation {
        tool: &tool,
        args,
        // .NET would otherwise try to open its diagnostic IPC socket, which the
        // seccomp filter refuses; this removes even the attempt (PR #15, S2-F).
        env: vec![("DOTNET_EnableDiagnostics", "0".into())],
        workdir: &scratch.tool,
        cwd: scratch.tool.clone(),
        status: scratch.root.join(status),
        read_only: read_only.clone(),
        read_write: Vec::new(),
    };
    let version = version(invocation(run::args(["--version"]), "status-version"))?;
    let args = run::args([
        "--disable-gui",
        "--console",
        "--plain",
        "-e",
        &renode::commands(&input.copy),
    ]);
    let out = invocation(args, "status-run").run()?;
    let code = out.code("renode")?;
    let (findings, machine) = renode::judge(code, &out)?;
    let mut v = verdict::Verdict::new(
        "simulate",
        &input,
        &tool,
        version,
        code,
        out.sandbox,
        findings,
    );
    v.machine = machine;
    Ok(v)
}

fn validate(file: Option<&Path>) -> ExitCode {
    match read_spec(file) {
        Ok((spec, _)) => emit(
            &ValidReport {
                ok: true,
                project_name: &spec.project_name,
                voltage_domains: spec.voltage_domains.len(),
                net_constraints: spec.net_constraints.len(),
            },
            0,
        ),
        Err((refusal, code)) => emit(&refusal, code),
    }
}

/// `aethelgard solve --file`: read and validate as `validate` does, load the
/// part's pin table, solve, and print the `SolvedTopology` on one line (exit
/// 0). A specification the solver cannot realize is exit 2, with `claimants`.
fn solve(file: &Path) -> ExitCode {
    match solved(file) {
        Ok((_, topology, _)) => emit(&topology, 0),
        Err(code) => code,
    }
}

/// Reads one `Specification` from a file, or stdin when `file` is `None`,
/// under the input cap, and validates it. Returns it with the sha256 of the bytes as read, or the
/// refusal and its exit code (2 or 3).
fn read_spec(file: Option<&Path>) -> Result<(Specification, String), (Refusal, u8)> {
    let input = read_input(file).map_err(|r| (r, EXIT_UNREADABLE))?;
    // Well-formed JSON of the wrong shape is an invalid specification;
    // anything that is not one JSON document is unreadable input.
    let spec: Specification = serde_json::from_slice(&input).map_err(|err| {
        if err.classify() == Category::Data {
            (
                Refusal::new("SchemaViolation", None, err.to_string()),
                EXIT_INVALID_SPEC,
            )
        } else {
            (
                Refusal::new("InvalidJson", None, err.to_string()),
                EXIT_UNREADABLE,
            )
        }
    })?;
    spec.validate().map_err(|err| {
        (
            Refusal::new(err.kind(), err.id(), err.to_string()).at(err.path()),
            EXIT_INVALID_SPEC,
        )
    })?;
    Ok((
        spec,
        input::hex(<sha2::Sha256 as sha2::Digest>::digest(&input)),
    ))
}

/// Reads, validates, loads the pin table and solves. Every refusal is
/// printed here; the caller gets its exit code.
fn solved(
    file: &Path,
) -> Result<
    (
        Specification,
        aethelgard_ir::SolvedTopology,
        aethelgard_ir::PinTable,
    ),
    ExitCode,
> {
    #[derive(Serialize)]
    struct SolveRefusal {
        ok: bool,
        error: SolveRefusalBody,
    }
    #[derive(Serialize)]
    struct SolveRefusalBody {
        kind: &'static str,
        id: Option<String>,
        claimants: Vec<String>,
        message: String,
    }

    let (spec, sha256) = read_spec(Some(file)).map_err(|(refusal, code)| emit(&refusal, code))?;
    let table = match aethelgard_svd_hal::pin_table(&spec.mcu.part) {
        Ok(table) => table,
        Err(err @ aethelgard_svd_hal::PinTableError::UnknownPart { .. }) => {
            let refusal = Refusal::new(err.kind(), Some(&spec.mcu.part), err.to_string());
            return Err(emit(&refusal, EXIT_INVALID_SPEC));
        }
        // The vendored table failed its own check: a corrupted build, not a
        // bad specification. Nothing on stdout, as for exit 70.
        Err(err) => {
            diagnose(&format!("the pin table is unusable: {err}"));
            return Err(ExitCode::from(EXIT_INTERNAL));
        }
    };
    match aethelgard_ir::solve(&spec, &table, &sha256) {
        Ok(topology) => Ok((spec, topology, table)),
        Err(err) => Err(emit(
            &SolveRefusal {
                ok: false,
                error: SolveRefusalBody {
                    kind: err.kind(),
                    id: err.id().map(str::to_owned),
                    claimants: err.claimants(),
                    message: err.to_string(),
                },
            },
            EXIT_INVALID_SPEC,
        )),
    }
}

/// `aethelgard emit --file --out`: solve as `solve` does, emit the schematic,
/// bill of materials and placed board (`aethelgard_eda::schematic::emit`),
/// and write them into `out` as `<project_name>.kicad_sch`,
/// `<project_name>.bom.csv` and `<project_name>.kicad_pcb`.
fn emit_design(file: &Path, out: &Path) -> ExitCode {
    #[derive(Serialize)]
    struct Written {
        path: String,
        sha256: String,
    }
    #[derive(Serialize)]
    struct EmitReport {
        ok: bool,
        project_name: String,
        specification_sha256: String,
        schematic: Written,
        bill_of_materials: Written,
        board: Written,
    }

    let (spec, topology, table) = match solved(file) {
        Ok(solved) => solved,
        Err(code) => return code,
    };
    let emission = match aethelgard_eda::schematic::emit(&spec, &topology, &table) {
        Ok(e) => e,
        Err(err @ aethelgard_eda::schematic::EmitError::Library(_)) => {
            diagnose(&format!("the vendored library is unusable: {err}"));
            return ExitCode::from(EXIT_INTERNAL);
        }
        Err(err) => {
            return emit(
                &Refusal::new(err.kind(), err.id(), err.to_string()),
                EXIT_INVALID_SPEC,
            );
        }
    };
    let unwritable = |message: String| {
        emit(
            &Refusal::new("UnwritableOutput", None, message),
            EXIT_UNREADABLE,
        )
    };
    if let Err(e) = DirBuilder::new().recursive(true).mode(0o755).create(out) {
        return unwritable(format!("cannot create {}: {e}", out.display()));
    }
    let mut written = Vec::new();
    for (suffix, text) in [
        ("kicad_sch", &emission.schematic),
        ("bom.csv", &emission.bom),
        ("kicad_pcb", &emission.board),
    ] {
        let path = out.join(format!("{}.{suffix}", spec.project_name));
        // A symlink is never followed and a FIFO never waited on: only a new
        // or existing regular file is written.
        let opened = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o644)
            .custom_flags(
                (rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32,
            )
            .open(&path)
            .and_then(|f| {
                if f.metadata()?.is_file() {
                    Ok(f)
                } else {
                    Err(io::Error::other("not a regular file"))
                }
            });
        let result = opened.and_then(|mut f| f.write_all(text.as_bytes()));
        if let Err(e) = result {
            return unwritable(format!("cannot write {}: {e}", path.display()));
        }
        written.push(Written {
            path: path.display().to_string(),
            sha256: input::hex(<sha2::Sha256 as sha2::Digest>::digest(text.as_bytes())),
        });
    }
    let board = written.pop().unwrap_or(Written {
        path: String::new(),
        sha256: String::new(),
    });
    let bill_of_materials = written.pop().unwrap_or(Written {
        path: String::new(),
        sha256: String::new(),
    });
    let schematic = written.pop().unwrap_or(Written {
        path: String::new(),
        sha256: String::new(),
    });
    emit(
        &EmitReport {
            ok: true,
            project_name: spec.project_name.clone(),
            specification_sha256: topology.specification_sha256.clone(),
            schematic,
            bill_of_materials,
            board,
        },
        0,
    )
}

/// Reads at most one byte past the limit, so oversized input is detected
/// without being held in memory.
fn read_input(file: Option<&Path>) -> Result<Vec<u8>, Refusal> {
    let mut input = Vec::new();
    let read = match file {
        Some(path) => {
            File::open(path).and_then(|f| f.take(MAX_INPUT_BYTES + 1).read_to_end(&mut input))
        }
        None => io::stdin()
            .lock()
            .take(MAX_INPUT_BYTES + 1)
            .read_to_end(&mut input),
    };
    let source = file.map_or_else(|| "stdin".to_owned(), |p| p.display().to_string());
    match read {
        Err(err) => Err(Refusal::new(
            "UnreadableInput",
            None,
            format!("cannot read {source}: {err}"),
        )),
        Ok(_) if input.len() as u64 > MAX_INPUT_BYTES => Err(Refusal::new(
            "InputTooLarge",
            None,
            format!("{source} exceeds {MAX_INPUT_BYTES} bytes"),
        )),
        Ok(_) => Ok(input),
    }
}

/// Writes exactly one JSON line to stdout. A report that cannot be written
/// exits 70, never the code the report would have carried.
fn emit(report: &impl Serialize, code: u8) -> ExitCode {
    let mut out = io::stdout().lock();
    let written = serde_json::to_writer(&mut out, report)
        .map_err(io::Error::from)
        .and_then(|()| out.write_all(b"\n"))
        .and_then(|()| out.flush());
    match written {
        Ok(()) => ExitCode::from(code),
        Err(err) => {
            diagnose(&format!("cannot write report to stdout: {err}"));
            ExitCode::from(EXIT_INTERNAL)
        }
    }
}

/// Diagnostics go to stderr; a closed stderr is not worth a panic.
fn diagnose(message: &str) {
    let _ = writeln!(io::stderr(), "aethelgard: {message}");
}
