//! The `aethelgard` binary: subcommands over the crates and the external
//! tools, one JSON object out, documented exit codes (`docs/architecture.md`
//! `RUL-10`). The codes are listed in this crate's `README.md`.

#![forbid(unsafe_code)]

mod analog;
mod campaign;
mod elf;
mod extract;
mod fab;
mod firmware;
mod input;
mod kani;
mod kicad;
mod mcp;
mod propose;
mod renode;
mod run;
mod said;
mod tree;
mod verdict;
mod verify;

use std::ffi::OsString;
use std::fs::{self, DirBuilder, File};
use std::io::{self, Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
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
/// `propose`: the run's failure is a generator defect, and a generator report
/// was written (`AET-WO-3200`).
const EXIT_GENERATOR_REPORT: u8 = 6;
/// The subcommands that print one JSON line per record, then their verdict's
/// line; every other prints exactly one. `mcp` reads this, not a name of its own.
const LINE_PER_RECORD: [&str; 1] = ["campaign"];
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
    /// Validate and solve one Specification, then write everything it produces into an empty or absent directory: kicad/ (as emit writes it), and board/, the board crate with its firmware crate in board/firmware/ and the Renode platform board/<project_name>.repl. Starts no process.
    Generate {
        /// The specification file.
        #[arg(long)]
        spec: PathBuf,
        /// The directory to write into; it must be empty or absent.
        #[arg(long)]
        out: PathBuf,
    },
    /// Build a board crate's firmware/ crate offline for the part, in the sandbox, and print the verdict with the ELF's sha256.
    Build {
        /// The board crate directory, holding firmware/. It is copied, minus a
        /// top-level target/, and the copy is built.
        #[arg(long = "crate")]
        crate_dir: PathBuf,
        /// Where to write the ELF; it must not exist.
        #[arg(long)]
        out: PathBuf,
    },
    /// Regenerate everything from one Specification into scratch, run every gate on exactly those bytes, and print one verdict naming every artifact's hash and every gate's result.
    Verify {
        /// The specification file.
        #[arg(long)]
        spec: PathBuf,
        /// Fault scenarios whose base is --spec, run as campaign runs them.
        #[arg(long, num_args = 1.., action = clap::ArgAction::Append)]
        scenarios: Vec<PathBuf>,
        /// First compare this directory with the generated tree, byte for byte; any difference fails and no gate runs.
        #[arg(long)]
        against: Option<PathBuf>,
        /// Requires --scenarios (INV-03 counts the fault campaign). Only if every gate passes, write the fabrication package (Gerbers, drill, placement, BOM, firmware ELF, FAB-NOTES.md, MANIFEST.json) into this directory, which must be empty or absent; the verdict names every file's sha256.
        #[arg(long, conflicts_with = "against_fab", requires = "scenarios")]
        fab_out: Option<PathBuf>,
        /// Requires --scenarios. Derive the fabrication package again and compare it with this directory, file by file; any file that differs, is missing or is extra fails.
        #[arg(long, requires = "scenarios")]
        against_fab: Option<PathBuf>,
    },
    /// Bound each I2C line's capacitance from above from a routed board's copper, and judge each bound against the specification's declared bound. Runs no tool.
    Extract {
        /// The routed .kicad_pcb file.
        #[arg(long)]
        board: PathBuf,
        /// The specification whose I2C lines are judged.
        #[arg(long)]
        spec: PathBuf,
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
        /// The specification the crate is judged against: the verdict's
        /// harness_provenance is `generated` only when the crate is byte for
        /// byte the board crate `generate` writes for it.
        #[arg(long)]
        spec: Option<PathBuf>,
    },
    /// Load a platform in Renode, list the machine it built, and print the verdict.
    Simulate {
        /// A .repl file, or @path inside Renode's own tree, such as
        /// @platforms/boards/nucleo_h753zi.repl. With --firmware, it must be
        /// exactly the platform emitted for --spec.
        #[arg(long)]
        platform: PathBuf,
        /// Boot this ELF on the platform and judge the boot by the
        /// specification. Needs --spec and --for.
        #[arg(long, requires_all = ["spec", "for_time"])]
        firmware: Option<PathBuf>,
        /// The specification the boot is judged by. Needs --firmware.
        #[arg(long, requires = "firmware")]
        spec: Option<PathBuf>,
        /// The virtual time the boot runs: an integer with ns, us or ms, at
        /// most 100ms, a whole number of 10 ns instructions. Needs --firmware.
        #[arg(long = "for", id = "for_time", value_name = "TIME", requires = "firmware", value_parser = virtual_time)]
        for_time: Option<u64>,
    },
    /// Run each fault scenario through simulate --firmware's path, archive every record of each run, and print one JSON line per scenario, then a summary.
    Campaign {
        /// The scenario files, run in the order given. Each names its base
        /// specification relative to its own directory, inside the current
        /// directory.
        #[arg(long, required = true, num_args = 1.., action = clap::ArgAction::Append)]
        scenarios: Vec<PathBuf>,
        /// The firmware ELF every scenario boots.
        #[arg(long)]
        firmware: PathBuf,
        /// The archive directory; each run is a new directory inside it, named by its run id.
        #[arg(long)]
        archive: PathBuf,
    },
    /// Replay one archived campaign run from its trace alone, and print its state-sequence hash.
    Replay {
        /// The run directory.
        #[arg(long)]
        archive: PathBuf,
    },
    /// Attribute one archived campaign run to the electrical or the software half, from its trace alone, naming the rule and the evidence events.
    Attribute {
        /// The run directory.
        #[arg(long)]
        archive: PathBuf,
    },
    /// Propose a specification change for one archived campaign run, from its attribution, and write it (or a generator report, or why nothing is proposed) into an empty directory. Never applies a change or edits a gate.
    Propose {
        /// The run directory.
        #[arg(long)]
        archive: PathBuf,
        /// The scenario file the run ran: the same bytes, with its base resolved as campaign resolves it.
        #[arg(long)]
        scenario: PathBuf,
        /// The directory to write proposal.json, and proposed-spec.json or generator-report.json, into; it must be empty or absent.
        #[arg(long)]
        out: PathBuf,
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
        /// Judge by the rules this specification declares, and first refuse a board that is not its declared board (copper layers, planes, thickness, through vias); without it, by KiCad's defaults. Either way the gate writes every rule file itself.
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
        Command::Generate { spec, out } => return generate_tree(&spec, &out),
        Command::Build { crate_dir, out } => firmware::build(&crate_dir, &out),
        Command::Verify {
            spec,
            scenarios,
            against,
            fab_out,
            against_fab,
        } => {
            let fab = match (fab_out, against_fab) {
                (Some(dir), _) => Some(verify::FabMode::Write(dir)),
                (None, Some(dir)) => Some(verify::FabMode::Compare(dir)),
                (None, None) => None,
            };
            return verify::verify(&spec, &scenarios, against.as_deref(), fab.as_ref());
        }
        Command::Extract { board, spec } => return extract::extract(&board, &spec),
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
        Command::Prove {
            crate_dir,
            harness,
            spec,
        } => prove(&crate_dir, &harness, spec.as_deref()),
        Command::Simulate {
            platform,
            firmware,
            spec,
            for_time,
        } => match (firmware, spec, for_time) {
            (Some(firmware), Some(spec), Some(virtual_ns)) => {
                let topology = match solved(&spec) {
                    Ok((_, topology, _)) => topology,
                    Err(code) => return code,
                };
                simulate(
                    &platform,
                    Some(Boot {
                        firmware,
                        topology,
                        virtual_ns,
                        scenario: String::new(),
                        held: Vec::new(),
                        absent: Default::default(),
                        rail: None,
                    }),
                )
            }
            // clap's `requires` makes the three all-or-none.
            _ => simulate(&platform, None),
        },
        Command::Campaign {
            scenarios,
            firmware,
            archive,
        } => return campaign::run(&scenarios, &firmware, &archive),
        Command::Replay { archive } => return campaign::replay(&archive),
        Command::Attribute { archive } => return campaign::attribute(&archive),
        Command::Propose {
            archive,
            scenario,
            out,
        } => return propose::propose(&archive, &scenario, &out),
        Command::Mcp => return mcp::serve(),
    };
    match judged {
        Ok(v) => {
            let code = if v.ok { 0 } else { EXIT_FAIL };
            emit(&v, code)
        }
        Err(stop) => stopped(stop),
    }
}

/// Prints a `Stop` as the refusal it is and returns its exit code.
fn stopped(stop: run::Stop) -> ExitCode {
    let code = stop.code;
    emit(
        &Refusal::new(stop.kind, stop.id.as_deref(), stop.message),
        code,
    )
}

/// `--for`: an integer, no leading zero, then `ns`, `us` or `ms`; at most the
/// ruled 100 ms (`aethelgard_twin::boot::MAX_VIRTUAL_NS`), and a whole number
/// of instructions at the platform's 100 MIPS, so the instruction witness is
/// exact.
fn virtual_time(s: &str) -> Result<u64, String> {
    let (digits, scale) = if let Some(d) = s.strip_suffix("ns") {
        (d, 1)
    } else if let Some(d) = s.strip_suffix("us") {
        (d, 1_000)
    } else if let Some(d) = s.strip_suffix("ms") {
        (d, 1_000_000)
    } else {
        return Err("a virtual time is an integer then ns, us or ms, such as 10ms".into());
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) || digits.starts_with('0') {
        return Err(format!(
            "`{digits}` is not a positive integer without a leading zero"
        ));
    }
    let max = aethelgard_twin::boot::MAX_VIRTUAL_NS;
    let ns = digits
        .parse::<u64>()
        .ok()
        .and_then(|n| n.checked_mul(scale))
        .filter(|&ns| ns <= max)
        .ok_or_else(|| format!("{s} is more than the ruled {} ms", max / 1_000_000))?;
    let per = 1000 / aethelgard_twin::boot::MIPS;
    if ns % per != 0 {
        return Err(format!(
            "{s} is not a whole number of {per} ns instructions"
        ));
    }
    Ok(ns)
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

/// Where a root CLI keeps scratch when `TMPDIR` is not set (`AET-WO-1601` §1.4).
const ROOT_SCRATCH: &str = "/var/lib/aethelgard";

/// Where any other CLI keeps scratch when `TMPDIR` is not set, under `HOME`.
const USER_SCRATCH: &str = ".cache/aethelgard";

/// `ScratchUnsafe`, exit 4: no tool runs in a scratch directory that breaks
/// the rule (`AET-WO-1601` §1.4, `INV-06`).
fn scratch_unsafe(id: &Path, message: String) -> run::Stop {
    run::Stop {
        code: EXIT_TOOL_UNAVAILABLE,
        kind: "ScratchUnsafe",
        id: Some(id.display().to_string()),
        message,
    }
}

/// The directory every call's scratch directory is made in (`AET-WO-1601`
/// §1.4): `TMPDIR` if it is set, which must exist; otherwise
/// [`ROOT_SCRATCH`] for root and `$HOME/`[`USER_SCRATCH`] for anyone else,
/// created if missing. Cargo and rustup read configuration from every
/// ancestor of a tool's working directory (FEEDBACK §1.1, S9 and S10), so the
/// base and each of its ancestors must be a directory owned by root or by this
/// process's effective uid and writable by no group and no other user: a
/// file another uid could create there could be read as configuration.
/// `/tmp` breaks the rule. As root, each must also be searchable by others,
/// or the tool uid could not reach its own directory. The real path must be
/// spelled in `[A-Za-z0-9/._-]` alone, so no tool's command language (the
/// Renode monitor's `@path`) reads a space or a `;` in it (`AET-WO-2100` F25).
/// Anything else is refused; nothing else is tried.
fn scratch_base() -> Result<PathBuf, run::Stop> {
    let euid = rustix::process::geteuid();
    let root = euid.is_root();
    let base = match std::env::var_os("TMPDIR").filter(|v| !v.is_empty()) {
        Some(tmpdir) => PathBuf::from(tmpdir),
        None => {
            let base = if root {
                PathBuf::from(ROOT_SCRATCH)
            } else {
                match std::env::var_os("HOME").map(PathBuf::from) {
                    Some(home) if home.is_absolute() => home.join(USER_SCRATCH),
                    _ => {
                        return Err(scratch_unsafe(
                            Path::new("HOME"),
                            "neither TMPDIR nor an absolute HOME is set, so there is no private place for scratch"
                                .to_owned(),
                        ));
                    }
                }
            };
            let mode = if root { 0o711 } else { 0o700 };
            DirBuilder::new()
                .recursive(true)
                .mode(mode)
                .create(&base)
                .map_err(|e| {
                    scratch_unsafe(&base, format!("cannot create {}: {e}", base.display()))
                })?;
            base
        }
    };
    let real = fs::canonicalize(&base)
        .map_err(|e| scratch_unsafe(&base, format!("{}: {e}", base.display())))?;
    if !real
        .as_os_str()
        .as_encoded_bytes()
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || b"/._-".contains(b))
    {
        return Err(scratch_unsafe(
            &real,
            format!(
                "{} holds a character outside [A-Za-z0-9/._-], which a tool's command language could read as its own",
                real.display()
            ),
        ));
    }
    for dir in real.ancestors() {
        let meta = fs::symlink_metadata(dir)
            .map_err(|e| scratch_unsafe(dir, format!("{}: {e}", dir.display())))?;
        let mode = meta.permissions().mode();
        let why = if !meta.is_dir() {
            Some("is not a directory".to_owned())
        } else if meta.uid() != 0 && meta.uid() != euid.as_raw() {
            Some(format!(
                "belongs to uid {}, neither root nor this user",
                meta.uid()
            ))
        } else if mode & 0o022 != 0 {
            Some(format!(
                "is writable by its group or by others (mode {:o})",
                mode & 0o7777
            ))
        } else if root && mode & 0o001 == 0 {
            Some(format!(
                "cannot be searched by the tool uid {} (mode {:o})",
                run::TOOL_UID,
                mode & 0o7777
            ))
        } else {
            None
        };
        if let Some(why) = why {
            return Err(scratch_unsafe(
                dir,
                format!(
                    "scratch would live under {}, and {} {why}: another user could plant configuration a tool reads. Set TMPDIR to a directory only you or root can write",
                    real.display(),
                    dir.display()
                ),
            ));
        }
    }
    Ok(real)
}

/// The call's private temporary directory: `root/status-*` for the reaper,
/// `root/tool/` for everything the tool may write. Removed on drop. As root,
/// `root` is root's with group [`run::TOOL_GID`] and mode 0710, so the tool
/// uid alone can pass through it, and `tool/` is root's (0700) until a tool
/// first runs in it (`run::Invocation`, `AET-WO-1601` §1.2).
struct Scratch {
    root: PathBuf,
    tool: PathBuf,
}

impl Scratch {
    fn new() -> Result<Self, run::Stop> {
        let base = scratch_base()?;
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let root = base.join(format!("aethelgard-{}-{nanos}", std::process::id()));
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
        if rustix::process::geteuid().is_root() {
            // The tool uid may pass through to its own directory, and no one
            // else may (AET-WO-1601 §1.2). The directory is new and its parent
            // is writable by root alone, so no one can have swapped it.
            let unavailable = |e: io::Error| {
                run::Stop::sandbox(
                    "uid",
                    format!("cannot give the tool uid its way into scratch: {e}"),
                )
            };
            std::os::unix::fs::lchown(&scratch.root, None, Some(run::TOOL_GID))
                .map_err(unavailable)?;
            fs::set_permissions(&scratch.root, fs::Permissions::from_mode(0o710))
                .map_err(unavailable)?;
        }
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
    version_of(name, &inv.run()?)
}

/// The first line of a tool's `--version` output, which must have exited 0.
fn version_of(name: &str, out: &run::Output) -> Result<String, run::Stop> {
    match (&out.ended, out.first_line()) {
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
    let mut declared = None;
    let (rules, spec_sha256) = match spec {
        Some(file) => {
            let (spec, sha256) = read_spec(Some(file)).map_err(|(refusal, code)| run::Stop {
                code,
                kind: refusal.error.kind,
                id: refusal.error.id,
                message: refusal.error.message,
            })?;
            declared = Some(aethelgard_eda::stack::Stack::of(&spec));
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
    if matches!(check, kicad::Check::Erc) {
        kicad::refuse_sheets(&input.copy)?;
    }
    let board = kicad::prepare(check, &input.copy, &scratch.tool, &rules)?;
    // With `--spec` the board is judged against the declared board before
    // KiCad runs (`AET-WO-4100`): its copper layers, planes, thickness and
    // vias, by name, never by the board's own claims (`RUL-03`).
    if let (Some(stack), Some(tree)) = (&declared, &board) {
        aethelgard_eda::route::declared(tree, stack).map_err(|d| run::Stop {
            code: EXIT_UNREADABLE,
            kind: d.kind,
            id: None,
            message: d.message,
        })?;
    }
    // KiCad's parity check reads the schematic named like the board, beside
    // it; the copy is scanned like the board before any tool runs, and the
    // board is judged on the strict reading `prepare` made of it.
    let schematic = match (schematic, &board) {
        (Some(path), Some(board)) => {
            let copy = input::file(path, &scratch.tool.join(kicad::Check::Erc.copy_name()))?;
            kicad::prepare_parity(board, &copy.copy)?;
            Some(copy)
        }
        _ => None,
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

/// The board crate `generate` writes for the specification in `file`, its
/// paths relative to the crate; every refusal as a `Stop`, with `solve`'s and
/// `generate`'s kinds and codes.
fn generated_board(file: &Path) -> Result<std::collections::BTreeMap<String, String>, run::Stop> {
    let (spec, sha256) = read_spec(Some(file)).map_err(|(refusal, code)| run::Stop {
        code,
        kind: refusal.error.kind,
        id: refusal.error.id,
        message: refusal.error.message,
    })?;
    let invalid = |kind: &'static str, id: Option<&str>, message: String| run::Stop {
        code: EXIT_INVALID_SPEC,
        kind,
        id: id.map(str::to_owned),
        message,
    };
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).map_err(|err| match err {
        aethelgard_svd_hal::PinTableError::UnknownPart { .. } => {
            invalid(err.kind(), Some(&spec.mcu.part), err.to_string())
        }
        _ => run::Stop {
            code: EXIT_INTERNAL,
            kind: err.kind(),
            id: None,
            message: format!("the pin table is unusable: {err}"),
        },
    })?;
    let topology = aethelgard_ir::solve(&spec, &table, &sha256)
        .map_err(|err| invalid(err.kind(), err.id(), err.to_string()))?;
    let files =
        aethelgard_cli::generate::tree(&spec, &topology, &table).map_err(|err| run::Stop {
            code: if err.internal() {
                EXIT_INTERNAL
            } else {
                EXIT_INVALID_SPEC
            },
            kind: err.kind(),
            id: err.id().map(str::to_owned),
            message: err.to_string(),
        })?;
    Ok(tree::subtree(
        &files,
        &format!("{}/", aethelgard_cli::generate::BOARD_DIR),
    ))
}

/// `generated` when the crate copied at `copy` is byte for byte `generated`
/// (every file, none missing and none extra); `supplied` otherwise, and
/// always without `--spec` (`AET-WO-3301` §1.4). The whole crate, not only
/// `src/proofs.rs`: a harness file that matches while the code it proves was
/// rewired is not the generated harness.
fn provenance(
    copy: &Path,
    generated: Option<&std::collections::BTreeMap<String, String>>,
) -> Result<&'static str, run::Stop> {
    Ok(match generated {
        Some(board) if tree::diff(copy, board, &[])?.is_empty() => "generated",
        _ => "supplied",
    })
}

fn prove(
    crate_dir: &Path,
    harness: &str,
    spec: Option<&Path>,
) -> Result<verdict::Verdict, run::Stop> {
    input::dir_check(crate_dir)?;
    // A --spec that cannot be generated is refused before any tool is looked for.
    let generated = spec.map(generated_board).transpose()?;
    let cargo = run::Tool::find("cargo")?;
    let cargo_kani = run::Tool::find("cargo-kani")?;
    let kani_home = run::required_var("KANI_HOME", true)?;
    let rustup_home = run::required_var("RUSTUP_HOME", true)?;
    let path_var = run::required_var("PATH", false)?;
    let scratch = Scratch::new()?;
    let input = input::crate_dir(crate_dir, &scratch.tool.join("crate"))?;
    // The copy, as copied and before Kani touches it, against the generated
    // crate: every file, none missing and none extra (`AET-WO-3301` §1.4).
    let provenance = provenance(&input.copy, generated.as_ref())?;
    // Cargo writes the Cargo.lock a crate without one needs into the crate's
    // root, so as root that one directory takes new files from the tool uid;
    // nothing in it can be changed (AET-WO-1601 adversarial F4a).
    if rustix::process::geteuid().is_root() {
        run::add_only(&input.copy).map_err(|e| {
            run::Stop::sandbox("uid", format!("cannot open the crate to its lockfile: {e}"))
        })?;
    }
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
    v.harness_provenance = Some(provenance);
    v.cover_count = Some(kani::cover_count(&out));
    Ok(v)
}

/// `simulate --firmware`'s inputs, once `--spec` is solved.
struct Boot {
    firmware: PathBuf,
    topology: aethelgard_ir::SolvedTopology,
    virtual_ns: u64,
    /// The campaign scenario's name and the lines it holds low; `simulate
    /// --firmware` has neither (`AET-WO-2300`).
    scenario: String,
    held: Vec<campaign::HeldLow>,
    /// The SPI devices the scenario leaves absent (`AET-WO-4200` D4).
    absent: std::collections::BTreeSet<String>,
    /// A campaign scenario's declared droop (`AET-WO-2301`); `None` otherwise.
    rail: Option<analog::Rail>,
}

/// A boot ready to run: every input copied and checked, every expectation
/// computed from the topology.
struct Ready {
    firmware: input::Input,
    image_sha256: String,
    executable: Vec<(u64, u64)>,
    self_loops: Vec<u64>,
    vector_table: u32,
    image: aethelgard_twin::boot::Image,
    boot_word: u32,
    specification_sha256: String,
    virtual_ns: u64,
    topology: aethelgard_ir::SolvedTopology,
    scenario: String,
    held: Vec<campaign::HeldLow>,
    /// The SPI devices a campaign scenario leaves absent behind their chip
    /// selects (`AET-WO-4200` D4).
    absent: std::collections::BTreeSet<String>,
    rail: Option<analog::Rail>,
}

/// The topology asks for what the twin cannot lower (exit 2), or the
/// vendored header failed its own check (a corrupted build, as for `solve`).
fn twin_stop(err: aethelgard_twin::TwinError) -> run::Stop {
    // A name the header does not define is the topology's fault (adversarial
    // C3); a header that fails its own checks is a corrupted build.
    let code = match err {
        aethelgard_twin::TwinError::Register(
            aethelgard_svd_hal::cmsis::RegisterError::Undefined { .. },
        ) => EXIT_INVALID_SPEC,
        aethelgard_twin::TwinError::Register(_) => EXIT_INTERNAL,
        _ => EXIT_INVALID_SPEC,
    };
    run::Stop {
        code,
        kind: err.kind(),
        id: None,
        message: err.to_string(),
    }
}

/// Copies and checks a boot's inputs before any tool is looked for: the
/// platform must be exactly the one emitted for the topology (R6, F1), and
/// the firmware must pass the ELF checks (R2).
fn ready(
    platform: &Path,
    boot: Boot,
    scratch: &Scratch,
) -> Result<(input::Input, Ready), run::Stop> {
    if platform.as_os_str().as_encoded_bytes().starts_with(b"@") {
        return Err(run::Stop::refused(
            "PlatformNotEmitted",
            "with --firmware, --platform is the emitted .repl file, not a path inside Renode's tree"
                .to_owned(),
        ));
    }
    let platform = input::file(platform, &scratch.tool.join("platform.repl"))?;
    let emitted = aethelgard_twin::platform::emit(&boot.topology).map_err(twin_stop)?;
    let copied = fs::read(&platform.copy)
        .map_err(|e| run::Stop::unreadable(format!("cannot read the platform's copy: {e}")))?;
    if copied != emitted.as_bytes() {
        return Err(run::Stop::refused(
            "PlatformNotEmitted",
            format!(
                "{} is not the platform aethelgard-twin emits for specification {}",
                platform.original.display(),
                boot.topology.specification_sha256
            ),
        ));
    }
    let firmware = input::file(&boot.firmware, &scratch.tool.join("firmware.elf"))?;
    let bytes = fs::read(&firmware.copy)
        .map_err(|e| run::Stop::unreadable(format!("cannot read the firmware's copy: {e}")))?;
    let memory = aethelgard_twin::boot::memory().map_err(twin_stop)?;
    let boot_word = aethelgard_twin::boot::boot_word().map_err(twin_stop)?;
    let checked = elf::check(&bytes, &memory, boot_word)?;
    let image = aethelgard_twin::boot::expected(&boot.topology).map_err(twin_stop)?;
    let vector_table =
        u32::try_from(memory.flash.start).expect("FLASH_BANK1_BASE is a 32-bit value");
    Ok((
        platform,
        Ready {
            firmware,
            image_sha256: checked.sha256,
            executable: checked.executable,
            self_loops: checked.self_loops,
            vector_table,
            image,
            boot_word,
            specification_sha256: boot.topology.specification_sha256.clone(),
            virtual_ns: boot.virtual_ns,
            topology: boot.topology,
            scenario: boot.scenario,
            held: boot.held,
            absent: boot.absent,
            rail: boot.rail,
        },
    ))
}

fn simulate(platform: &Path, boot: Option<Boot>) -> Result<verdict::Verdict, run::Stop> {
    let scratch = Scratch::new()?;
    let ready = match boot {
        Some(boot) => Some(ready(platform, boot, &scratch)?),
        None => {
            if !platform.as_os_str().as_encoded_bytes().starts_with(b"@") {
                input::file_check(platform)?;
            }
            None
        }
    };
    // The analog pre-pass runs before Renode is looked for, and its fault
    // decision fixes the platform the boot loads (AET-WO-2200 §1.4).
    let analog = match &ready {
        Some((_, r)) => Some(analog_pass(r, &scratch)?),
        None => None,
    };
    let tool = run::Tool::find("renode")?;
    let root = fs::canonicalize(&tool.path)
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .ok_or_else(|| {
            run::Stop::unusable("renode", "cannot resolve renode's installation".to_owned())
        })?;
    let (input, ready) = match ready {
        Some((input, ready)) => (input, Some(ready)),
        None => (
            input::platform(platform, &root, &scratch.tool.join("platform.repl"))?,
            None,
        ),
    };
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
        // W^X's double mapping truncates a memfd to 2 TiB, which RLIMIT_FSIZE
        // refuses with SIGXFSZ; without it Renode starts under the cap
        // (AET-WO-1601 §1.6, measured; FEEDBACK).
        env: vec![
            ("DOTNET_EnableDiagnostics", "0".into()),
            ("DOTNET_EnableWriteXorExecute", "0".into()),
        ],
        workdir: &scratch.tool,
        cwd: scratch.tool.clone(),
        status: scratch.root.join(status),
        read_only: read_only.clone(),
        read_write: Vec::new(),
    };
    // Every Renode this call starts reads one configuration, the droop's
    // through the bridge included (`AET-WO-2204` D4, F6).
    let config = scratch.tool.join(renode::CONFIG_FILE);
    fs::write(&config, renode::config(&scratch.tool)).map_err(|e| {
        run::Stop::unusable(
            "renode",
            format!("cannot write Renode's configuration: {e}"),
        )
    })?;
    let with_config = |rest: &[&str]| {
        let mut a = run::args(["--config".as_ref(), config.as_os_str()]);
        a.extend(run::args(rest));
        a
    };
    let reported = invocation(with_config(&["--version"]), "status-version").run()?;
    let version = version_of("renode", &reported)?;
    // The stock table and the boot's measured facts are Renode 1.17.0's;
    // another Renode is a fact the twin does not have (`INV-06`).
    let pinned = format!("Renode v{}", aethelgard_twin::platform::RENODE_VERSION);
    if ready.is_some() && version != pinned {
        return Err(run::Stop::unusable(
            "renode",
            format!("`{version}` is not the {pinned} the twin's platform is pinned to"),
        ));
    }
    // A boot's archive names the build Renode reported (`AET-WO-2205` §1.1).
    let build = match &ready {
        Some(_) => Some(renode::build(&reported.stdout).ok_or_else(|| {
            run::Stop::unusable("renode", "`renode --version` named no build".to_owned())
        })?),
        None => None,
    };
    // A declared droop: the rail is solved once, before the boot, and its
    // crossings become the resets the boot renders (`AET-WO-2301`, ruled D1).
    let droop = match ready.as_ref().and_then(|r| r.rail.as_ref()) {
        Some(rail) => {
            let solved = analog::rail(
                rail,
                ready.as_ref().map_or(0, |r| r.virtual_ns),
                &scratch.root.join("rail"),
                &scratch.root.join("status-rail"),
            )?;
            let resets = aethelgard_chaos::power::schedule(rail.droop.bor_level, &solved.crossings)
                .map_err(|e| run::Stop::tool_error(analog::BRIDGE, e.to_owned()))?;
            Some((rail, solved, resets))
        }
        None => None,
    };
    let booted = analog.as_ref().map(|(_, booted)| booted.clone());
    let plan = ready
        .as_ref()
        .zip(booted.as_ref())
        .map(|(r, booted)| renode::BootPlan {
            platform: &booted.path,
            firmware: &r.firmware.copy,
            virtual_ns: r.virtual_ns,
            boot_word: r.boot_word,
            vector_table: r.vector_table,
            image: &r.image,
            executable: &r.executable,
            self_loops: &r.self_loops,
            instances: &booted.instances,
            resets: droop
                .as_ref()
                .map_or(&[][..], |(_, _, resets)| resets.as_slice()),
        });
    if let (Some(r), Some((pass, booted))) = (&ready, &analog) {
        booted_is_emitted(&booted.path, r, pass)?;
    }
    let commands = match &plan {
        Some(plan) => plan.commands(),
        None => renode::commands(&input.copy),
    };
    let args = with_config(&["--disable-gui", "--console", "--plain", "-e", &commands]);
    let run = invocation(args, "status-run");
    // Only a boot is bounded (`RUL-13` as amended, PR #56 B1); a load is not.
    // A boot's log is read through the filter, so a flood reaches a verdict;
    // a droop's boot is the bridge's, which owns Renode as its child.
    let out = match (&plan, &droop) {
        (Some(_), Some(_)) => analog::boot(&run, &tool.path)?,
        (Some(_), None) => run.run_bounded_filtered(renode::BOOT_BOUNDS, renode::LOG_FILTER)?,
        (None, _) => run.run()?,
    };
    let (code, (findings, machine, observed)) = match (&plan, &out.ended) {
        // Killed at a bound: a fail that names it, with nothing read. A killed
        // tool has no exit code, so the verdict records -1.
        (Some(_), run::Ended::Bounded(bound)) => {
            (-1, (vec![renode::bounded(&out, *bound)], None, None))
        }
        (Some(plan), _) => {
            let code = out.code("renode")?;
            (code, renode::judge_boot(code, &out, plan)?)
        }
        (None, _) => {
            let code = out.code("renode")?;
            let (findings, machine) = renode::judge(code, &out)?;
            (code, (findings, machine, None))
        }
    };
    // The analog findings first: they are the evidence the boot's effect follows.
    let bridge = analog.as_ref().and_then(|(pass, _)| pass.sandbox);
    let mut tools: Vec<(&'static str, String)> = analog
        .as_ref()
        .map(|(pass, _)| pass.tools.clone())
        .unwrap_or_default();
    let (analog, findings) = match analog {
        Some((pass, _)) => {
            let mut all = pass.findings;
            all.extend(findings);
            (Some((pass.analog, pass.faults)), all)
        }
        None => (None, findings),
    };
    // The droop's records and findings, after the boot's (`AET-WO-2301`).
    let mut findings = findings;
    let droop_report = match (&droop, &ready, &plan) {
        (Some((rail, solved, resets)), Some(r), Some(plan)) => {
            let probes = match out.ended {
                run::Ended::Bounded(_) => None,
                _ => Some(renode::probes(&out, plan)?),
            };
            let (report, more) = analog::droop_report(
                rail,
                solved,
                resets,
                probes.as_deref(),
                &r.image,
                r.boot_word,
            );
            findings.extend(more);
            Some((solved, report))
        }
        _ => None,
    };
    let mut v = verdict::Verdict::new(
        "simulate",
        &input,
        &tool,
        version,
        code,
        out.sandbox,
        findings,
    );
    // The bridge's tools are this verdict's too (AET-WO-1601, PR #96).
    if let Some(bridge) = bridge {
        v.sandbox.unmet(bridge.uid_boundary_unmet);
    }
    v.machine = machine;
    if let (Some(r), Some(plan)) = (&ready, &plan) {
        v.boot = Some(boot_report(r, plan, observed.as_ref()));
    }
    // What judged the boot, as each tool reported it (`AET-WO-2205` §1.1).
    if let Some(build) = build {
        use aethelgard_chaos::campaign::{ENGINE, RENODE};
        v.tools.push((ENGINE, env!("CARGO_PKG_VERSION").to_owned()));
        v.tools.push((RENODE, build));
    }
    if let Some((records, faults)) = analog {
        v.analog = Some(records);
        v.faults = Some(faults);
    }
    if let Some((solved, report)) = droop_report {
        v.droop = Some(report);
        v.sandbox.unmet(solved.sandbox.uid_boundary_unmet);
        tools.extend(solved.tools.iter().cloned());
    }
    // One bridge ran each step: a step that named another version is a
    // stop, never a choice between them.
    tools.sort();
    tools.dedup();
    if let Some(w) = tools.windows(2).find(|w| w[0].0 == w[1].0) {
        return Err(run::Stop::tool_error(
            w[0].0,
            format!(
                "reported two versions in one run: `{}`, `{}`",
                w[0].1, w[1].1
            ),
        ));
    }
    v.tools.extend(tools);
    v.tools.sort();
    Ok(v)
}

/// The platform the boot loads, and what Renode must list once it has.
#[derive(Clone)]
struct Booted {
    path: PathBuf,
    /// Every solved peripheral, and each device the fault decision left in.
    instances: Vec<(String, String)>,
}

/// Measures every I2C line, then writes the platform the boot loads: the
/// emission for the specification with the pass's fault decision, which
/// declares no device on a faulted bus. The firmware still reads those
/// devices, so the boot's expected reads keep them (`AET-WO-2202`'s judge
/// then sees the timeout). The bridge works in a directory of its own, so
/// it can touch none of the copies Renode reads.
fn analog_pass(r: &Ready, scratch: &Scratch) -> Result<(analog::Pass, Booted), run::Stop> {
    let held = analog::Held {
        scenario: &r.scenario,
        lines: &r.held,
    };
    let pass = analog::pass(
        &r.topology,
        &held,
        &scratch.root.join("bridge"),
        &scratch.root.join("status-bridge"),
    )?;
    let faulted = pass.faulted();
    let text = aethelgard_twin::platform::emit_faulted(&r.topology, &faulted, &r.absent)
        .map_err(twin_stop)?;
    let instances = aethelgard_twin::platform::instances(&r.topology, &faulted, &r.absent)
        .map_err(twin_stop)?
        .into_iter()
        .map(|i| (i.name, i.model))
        .collect();
    let booted = scratch.tool.join("platform-booted.repl");
    fs::write(&booted, text).map_err(|e| {
        run::Stop::tool_error(
            "aethelgard",
            format!("cannot write the booted platform: {e}"),
        )
    })?;
    Ok((
        pass,
        Booted {
            path: booted,
            instances,
        },
    ))
}

/// `PlatformNotEmitted` for the file the boot loads: it must be the
/// emission for the specification plus the fault decision as recorded in
/// the verdict's `faults`, read again just before Renode starts.
fn booted_is_emitted(booted: &Path, r: &Ready, pass: &analog::Pass) -> Result<(), run::Stop> {
    let topology = &r.topology;
    let want = aethelgard_twin::platform::emit_faulted(topology, &pass.faulted(), &r.absent)
        .map_err(twin_stop)?;
    let have = fs::read(booted)
        .map_err(|e| run::Stop::unreadable(format!("cannot read the booted platform: {e}")))?;
    if have != want.as_bytes() {
        return Err(run::Stop::refused(
            "PlatformNotEmitted",
            format!(
                "the platform the boot would load is not the emission for specification {} with the recorded fault decision",
                topology.specification_sha256
            ),
        ));
    }
    Ok(())
}

fn boot_report(
    r: &Ready,
    plan: &renode::BootPlan<'_>,
    observed: Option<&renode::Observed>,
) -> verdict::Boot {
    use verdict::word;
    verdict::Boot {
        firmware: verdict::FirmwareReport {
            path: r.firmware.original.display().to_string(),
            sha256: r.firmware.sha256.clone(),
            image_sha256: r.image_sha256.clone(),
        },
        specification_sha256: r.specification_sha256.clone(),
        seed: aethelgard_twin::boot::SEED,
        virtual_time_ns: r.virtual_ns,
        mips: observed.map(|o| o.mips),
        instructions: verdict::Expected {
            expected: plan.instructions(),
            observed: observed.map(|o| o.instructions),
        },
        pc: observed.map(|o| word(o.pc)),
        boot_word: verdict::Word {
            address: word(r.boot_word),
            expected: word(aethelgard_twin::boot::BOOT_MAGIC),
            observed: observed.map(|o| word(o.boot_word)),
        },
        registers: r
            .image
            .registers
            .iter()
            .enumerate()
            .map(|(i, reg)| verdict::RegisterReport {
                name: reg.name.clone(),
                address: word(reg.address),
                mask: word(reg.mask),
                expected: word(reg.value),
                observed: observed.map(|o| word(o.registers[i])),
            })
            .collect(),
        unobservable: r.image.unobservable.clone(),
        i2c: r
            .image
            .identity
            .checks
            .iter()
            .enumerate()
            .filter_map(|(i, c)| match c.wire {
                aethelgard_twin::boot::Wire::I2c { address } => Some(verdict::I2cReport {
                    reference: c.reference.clone(),
                    address: format!("{address:#04x}"),
                    register: format!("{:#04x}", c.write),
                    expected: format!("{:#x}", c.expected),
                    observed: observed.map(|o| format!("{:#x}", o.records[i].1)),
                    status: observed.map(|o| renode::status_name(c, o.records[i].0)),
                }),
                aethelgard_twin::boot::Wire::Spi { .. } => None,
            })
            .collect(),
        i2c_witness: r
            .image
            .identity
            .checks
            .iter()
            .enumerate()
            .filter(|(_, c)| matches!(c.wire, aethelgard_twin::boot::Wire::I2c { .. }))
            .map(|(i, c)| {
                verdict::WitnessReport::of(
                    c,
                    observed.and_then(|o| o.witnesses[i].clone()),
                    observed
                        .map(|o| o.absent[i].iter().map(|a| format!("{a:#04x}")).collect())
                        .unwrap_or_default(),
                )
            })
            .collect(),
        spi: r
            .image
            .identity
            .checks
            .iter()
            .enumerate()
            .filter_map(|(i, c)| match &c.wire {
                aethelgard_twin::boot::Wire::Spi { select, address } => Some(verdict::SpiReport {
                    reference: c.reference.clone(),
                    peripheral: c.peripheral.clone(),
                    select: select.clone(),
                    address: format!("{address:#04x}"),
                    command: format!("{:#04x}", c.write),
                    expected: format!("{:#x}", c.expected),
                    observed: observed.map(|o| format!("{:#x}", o.records[i].1)),
                    status: observed.map(|o| renode::status_name(c, o.records[i].0)),
                }),
                aethelgard_twin::boot::Wire::I2c { .. } => None,
            })
            .collect(),
        spi_witness: r
            .image
            .identity
            .checks
            .iter()
            .enumerate()
            .filter(|(_, c)| matches!(c.wire, aethelgard_twin::boot::Wire::Spi { .. }))
            .map(|(i, c)| verdict::SpiWitnessReport {
                peripheral: c.peripheral.clone(),
                reference: c.reference.clone(),
                device: c.device.clone(),
                expected: c.witness.clone().unwrap_or_default(),
                observed: observed.and_then(|o| o.witnesses[i].clone()),
                unrouted: observed.map(|o| o.unrouted[i].clone()).unwrap_or_default(),
            })
            .collect(),
        icsr: observed.map(|o| word(o.icsr)),
        executable: plan.executable.to_vec(),
    }
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
    parse_spec(&input)
}

/// Parses and validates one `Specification` from bytes already read under the
/// cap, as `read_spec` does.
fn parse_spec(input: &[u8]) -> Result<(Specification, String), (Refusal, u8)> {
    // A specification of another version is refused by name before its shape
    // is judged: a version-1 file has no `board`, and nothing supplies one
    // (`AET-WO-4100`, `RUL-14`).
    #[derive(serde::Deserialize)]
    struct Version {
        schema_version: u32,
    }
    if let Ok(Version { schema_version }) = serde_json::from_slice::<Version>(input)
        && schema_version != aethelgard_ir::SPECIFICATION_SCHEMA_VERSION
    {
        let err = aethelgard_ir::IrValidationError::UnsupportedSchemaVersion {
            version: schema_version,
            expected: aethelgard_ir::SPECIFICATION_SCHEMA_VERSION,
        };
        return Err((
            Refusal::new(err.kind(), err.id(), err.to_string()),
            EXIT_INVALID_SPEC,
        ));
    }
    // Well-formed JSON of the wrong shape is an invalid specification;
    // anything that is not one JSON document is unreadable input.
    let spec: Specification = serde_json::from_slice(input).map_err(|err| {
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
        input::hex(<sha2::Sha256 as sha2::Digest>::digest(input)),
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
    let (spec, sha256) = read_spec(Some(file)).map_err(|(refusal, code)| emit(&refusal, code))?;
    let (topology, table) = solve_spec(&spec, &sha256)?;
    Ok((spec, topology, table))
}

/// Loads the pin table and solves a validated specification whose bytes
/// hash to `sha256`. Every refusal is printed here; the caller gets its
/// exit code.
fn solve_spec(
    spec: &Specification,
    sha256: &str,
) -> Result<(aethelgard_ir::SolvedTopology, aethelgard_ir::PinTable), ExitCode> {
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
    match aethelgard_ir::solve(spec, &table, sha256) {
        Ok(topology) => Ok((topology, table)),
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

/// Everything `generate` writes for a solved specification, and the sha256
/// of the topology. Every refusal is printed here; the caller gets its exit
/// code.
fn generated(
    spec: &Specification,
    topology: &aethelgard_ir::SolvedTopology,
    table: &aethelgard_ir::PinTable,
) -> Result<(std::collections::BTreeMap<String, String>, String), ExitCode> {
    let files = match aethelgard_cli::generate::tree(spec, topology, table) {
        Ok(files) => files,
        Err(err) if err.internal() => {
            diagnose(&format!("a vendored generator input is unusable: {err}"));
            return Err(ExitCode::from(EXIT_INTERNAL));
        }
        Err(err) => {
            return Err(emit(
                &Refusal::new(err.kind(), err.id(), err.to_string()),
                EXIT_INVALID_SPEC,
            ));
        }
    };
    match serde_json::to_vec(topology) {
        Ok(bytes) => Ok((files, tree::sha256(&bytes))),
        Err(err) => {
            diagnose(&format!("cannot serialize the topology: {err}"));
            Err(ExitCode::from(EXIT_INTERNAL))
        }
    }
}

/// `aethelgard generate --spec --out`: solve as `solve` does, then write
/// every file `aethelgard_cli::generate::tree` gives under `out`, which must
/// be empty or absent (`AET-WO-3301` §1.1). Nothing else is written and no
/// process is started.
fn generate_tree(file: &Path, out: &Path) -> ExitCode {
    #[derive(Serialize)]
    struct GenerateReport {
        ok: bool,
        project_name: String,
        specification_sha256: String,
        topology_sha256: String,
        /// Every file written, by its path under `--out`, with its sha256.
        files: std::collections::BTreeMap<String, String>,
    }

    let (spec, topology, table) = match solved(file) {
        Ok(solved) => solved,
        Err(code) => return code,
    };
    let (files, topology_sha256) = match generated(&spec, &topology, &table) {
        Ok(generated) => generated,
        Err(code) => return code,
    };
    if let Err(stop) = tree::write(out, &files) {
        return stopped(stop);
    }
    emit(
        &GenerateReport {
            ok: true,
            project_name: spec.project_name.clone(),
            specification_sha256: topology.specification_sha256.clone(),
            topology_sha256,
            files: files
                .iter()
                .map(|(path, text)| (path.clone(), tree::sha256(text.as_bytes())))
                .collect(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    /// `AET-WO-3301` §1.4 in tier 1: the committed board is the generated
    /// crate for the reference; a copy whose harness is hand-written, whose
    /// code is rewired under an unchanged harness, or with a file added is
    /// `supplied`; and without `--spec` every crate is `supplied`.
    #[test]
    fn provenance_is_generated_only_for_the_generated_crate() {
        let spec = root().join("examples/sentinel-01-minimal.json");
        let board = generated_board(&spec).unwrap();
        let committed = root().join("boards/sentinel-01");
        assert_eq!(provenance(&committed, Some(&board)).unwrap(), "generated");
        assert_eq!(provenance(&committed, None).unwrap(), "supplied");

        let copy = std::env::temp_dir().join(format!("aet-3301-provenance-{}", std::process::id()));
        for (file, from, to) in [
            (
                "src/proofs.rs",
                "kani::cover!(",
                "kani::assume(false); kani::cover!(",
            ),
            ("src/board.rs", "pub fn ", "pub(crate) fn "),
            ("src/extra.rs", "", ""),
        ] {
            let _ = fs::remove_dir_all(&copy);
            tree::write(&copy, &board).unwrap();
            assert_eq!(provenance(&copy, Some(&board)).unwrap(), "generated");
            let path = copy.join(file);
            let text = fs::read_to_string(&path).unwrap_or_default();
            let edited = if from.is_empty() {
                "// added\n".to_owned()
            } else {
                text.replacen(from, to, 1)
            };
            assert_ne!(text, edited, "{file}");
            fs::write(&path, edited).unwrap();
            assert_eq!(
                provenance(&copy, Some(&board)).unwrap(),
                "supplied",
                "{file}"
            );
        }
        fs::remove_dir_all(&copy).unwrap();
    }
}
