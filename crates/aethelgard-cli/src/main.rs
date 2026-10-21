//! The `aethelgard` binary: subcommands over the crates and the external
//! tools, one JSON object out, documented exit codes (`docs/architecture.md`
//! `RUL-10`). The codes are listed in this crate's `README.md`.

#![forbid(unsafe_code)]

mod input;
mod kani;
mod kicad;
mod renode;
mod run;
mod verdict;

use std::ffi::OsString;
use std::fs::{self, DirBuilder, File};
use std::io::{self, Read, Write};
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use aethelgard_ir::TopologyGraph;
use clap::error::ErrorKind;
use clap::{Parser, Subcommand};
use serde::Serialize;
use serde_json::error::Category;

const EXIT_FAIL: u8 = 1;
const EXIT_INVALID_SPEC: u8 = 2;
const EXIT_UNREADABLE: u8 = 3;
/// Exit 3 also covers a tool subcommand's input path.
const EXIT_INPUT: u8 = EXIT_UNREADABLE;
const EXIT_TOOL_UNAVAILABLE: u8 = 4;
const EXIT_TOOL_ERROR: u8 = 5;
const EXIT_USAGE: u8 = 64;
const EXIT_INTERNAL: u8 = 70;

/// Larger input is refused unread, so hostile input cannot exhaust memory.
const MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Parser)]
#[command(name = "aethelgard", version, about, arg_required_else_help = false)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate one JSON TopologyGraph read from stdin, or from --file.
    Validate {
        /// Read the specification from this file instead of stdin.
        #[arg(long)]
        file: Option<PathBuf>,
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
        /// The harness to prove, as `cargo kani --harness` takes it.
        #[arg(long)]
        harness: String,
    },
    /// Load a platform in Renode, list the machine it built, and print the verdict.
    Simulate {
        /// A .repl file, or @path inside Renode's own tree, such as
        /// @platforms/boards/nucleo_h753zi.repl.
        #[arg(long)]
        platform: PathBuf,
    },
}

#[derive(Subcommand)]
enum CheckCommand {
    /// KiCad's design rule check on a board (kicad-cli pcb drc).
    Drc {
        /// The .kicad_pcb file.
        #[arg(long)]
        board: PathBuf,
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
    message: String,
}

impl Refusal {
    fn new(kind: &'static str, id: Option<&str>, message: String) -> Self {
        Self {
            ok: false,
            error: RefusalBody {
                kind,
                id: id.map(str::to_owned),
                message,
            },
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().collect();
    match args.get(1).and_then(|a| a.to_str()) {
        Some(run::REAPER_ARG) => run::reaper_main(&args[2..]),
        Some(run::CONFINE_ARG) => run::confine_main(&args[2..]),
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

    let judged = match cli.command {
        Command::Validate { file } => return validate(file.as_deref()),
        Command::Check {
            check: CheckCommand::Drc { board },
        } => check_kicad(kicad::Check::Drc, &board),
        Command::Check {
            check: CheckCommand::Erc { schematic },
        } => check_kicad(kicad::Check::Erc, &schematic),
        Command::Prove { crate_dir, harness } => prove(&crate_dir, &harness),
        Command::Simulate { platform } => simulate(&platform),
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
        DirBuilder::new().mode(0o700).create(&root).map_err(fail)?;
        let tool = root.join("tool");
        DirBuilder::new().mode(0o700).create(&tool).map_err(fail)?;
        Ok(Self { root, tool })
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

fn check_kicad(check: kicad::Check, path: &Path) -> Result<verdict::Verdict, run::Stop> {
    let original = input::file_check(path)?;
    let tool = run::Tool::find("kicad-cli")?;
    let scratch = Scratch::new()?;
    let input = input::file(&original, &scratch.tool.join(check.copy_name()))?;
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
    let mut args = run::args([
        noun,
        verb,
        "--exit-code-violations",
        "--format",
        "json",
        "-o",
    ]);
    args.push(report.clone().into_os_string());
    args.push(input.copy.clone().into_os_string());
    let out = invocation(args, "status-run").run()?;
    let (findings, severities) = kicad::judge(check, &out, &report, &version)?;
    let code = verdict::code_of(out.ended).unwrap_or_default();
    let mut v = verdict::Verdict::new(
        check.name(),
        &input,
        &tool,
        version,
        code,
        out.sandbox,
        findings,
    );
    v.included_severities = Some(severities);
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
    let out = invocation(run_args, &input.copy, "status-run").run()?;
    let findings = kani::judge(&out, harness)?;
    let code = verdict::code_of(out.ended).unwrap_or_default();
    let mut v = verdict::Verdict::new(
        "prove",
        &input,
        &cargo,
        version,
        code,
        out.sandbox,
        findings,
    );
    // The judge is Kani; `cargo` only fronts it.
    v.tool.name = "cargo-kani";
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
        env: Vec::new(),
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
    let (findings, machine) = renode::judge(&out)?;
    let code = verdict::code_of(out.ended).unwrap_or_default();
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
    let input = match read_input(file) {
        Ok(input) => input,
        Err(refusal) => return emit(&refusal, EXIT_UNREADABLE),
    };

    let graph: TopologyGraph = match serde_json::from_slice(&input) {
        Ok(graph) => graph,
        // Well-formed JSON of the wrong shape is an invalid specification;
        // anything that is not one JSON document is unreadable input.
        Err(err) if err.classify() == Category::Data => {
            let refusal = Refusal::new("SchemaViolation", None, err.to_string());
            return emit(&refusal, EXIT_INVALID_SPEC);
        }
        Err(err) => {
            let refusal = Refusal::new("InvalidJson", None, err.to_string());
            return emit(&refusal, EXIT_UNREADABLE);
        }
    };

    match graph.validate() {
        Ok(()) => emit(
            &ValidReport {
                ok: true,
                project_name: &graph.project_name,
                voltage_domains: graph.voltage_domains.len(),
                net_constraints: graph.net_constraints.len(),
            },
            0,
        ),
        Err(err) => emit(
            &Refusal::new(err.kind(), err.id(), err.to_string()),
            EXIT_INVALID_SPEC,
        ),
    }
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
