//! The `aethelgard` binary: subcommands over the crates, JSON in, one JSON
//! object out, documented exit codes (`docs/architecture.md` `RUL-10`). The
//! codes are listed in this crate's `README.md`.

#![forbid(unsafe_code)]

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use aethelgard_ir::TopologyGraph;
use clap::error::ErrorKind;
use clap::{Parser, Subcommand};
use serde::Serialize;
use serde_json::error::Category;

const EXIT_INVALID_SPEC: u8 = 2;
const EXIT_UNREADABLE: u8 = 3;
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

    match cli.command {
        Command::Validate { file } => validate(file.as_deref()),
    }
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
