//! The `aethelgard` binary: subcommands over the crates and the external
//! tools, one JSON object out, documented exit codes (`docs/architecture.md`
//! `RUL-10`). The codes are listed in this crate's `README.md`.

#![forbid(unsafe_code)]

mod analog;
mod anchor;
mod campaign;
mod doctor;
mod elf;
mod extract;
mod fab;
mod firmware;
mod hwdef;
mod input;
mod kani;
mod kicad;
mod mcp;
mod pinout;
mod pinwitness;
mod propose;
mod renode;
mod run;
mod said;
mod speeds;
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
    /// Validate and solve one Specification, then write everything it produces into an empty or absent directory: kicad/ (as emit writes it), and board/, the board crate with its firmware crate in board/firmware/ and the Renode platform board/<project_name>.repl. Starts no process. With a subcommand, generate one artifact instead (hwdef).
    #[command(args_conflicts_with_subcommands = true, subcommand_negates_reqs = true)]
    Generate {
        #[command(subcommand)]
        what: Option<GenerateCommand>,
        /// The specification file.
        #[arg(long, required = true)]
        spec: Option<PathBuf>,
        /// The directory to write into; it must be empty or absent.
        #[arg(long, required = true)]
        out: Option<PathBuf>,
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
    /// Bound each I2C line's capacitance, and each SPI bus's SCK and MOSI, from above from a routed board's copper, and judge each bound against the specification's declared bound. Runs no tool.
    Extract {
        /// The routed .kicad_pcb file.
        #[arg(long)]
        board: PathBuf,
        /// The specification whose lines are judged.
        #[arg(long)]
        spec: PathBuf,
    },
    /// Run a check and print its verdict: KiCad's on a board or schematic, or the output speed check on a specification.
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
        /// specification. Needs --spec, and --for or --foreign.
        #[arg(long, requires_all = ["spec", "window"])]
        firmware: Option<PathBuf>,
        /// The specification the boot is judged by. Needs --firmware.
        #[arg(long, requires = "firmware")]
        spec: Option<PathBuf>,
        /// The virtual time the boot runs: an integer with ns, us or ms, at
        /// most 100ms on every board, a whole number of 10 ns instructions.
        /// Needs --firmware.
        #[arg(long = "for", id = "for_time", value_name = "TIME", requires = "firmware", value_parser = virtual_time, group = "window")]
        for_time: Option<u64>,
        /// Judge the boot as a foreign firmware's, one this project did not
        /// generate (AET-WO-4400e): declared here, never inferred from the
        /// ELF; the verdict's boot says judge foreign. Its value is the
        /// virtual time the boot runs, as --for's, at most the foreign
        /// window. Needs --firmware; not with --for.
        #[arg(long, value_name = "TIME", requires = "firmware", value_parser = foreign_time, group = "window")]
        foreign: Option<u64>,
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
    /// Report whether this host can run every subcommand: the binary's version, the sandbox the tools would run in, the scratch directory, and each external tool's version, naming each missing piece and the subcommands it blocks.
    Doctor,
    /// Serve every other subcommand as an MCP tool on stdin and stdout.
    Mcp,
}

#[derive(Subcommand)]
enum GenerateCommand {
    /// Write ArduPilot's hwdef.dat for a SolvedTopology, from the topology and the parts tables alone (AET-WO-4400b). Starts no process.
    Hwdef {
        /// The SolvedTopology JSON.
        #[arg(long)]
        topology: PathBuf,
        /// Where to write hwdef.dat; it must not exist.
        #[arg(long)]
        out: PathBuf,
    },
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
    /// The MCU's support network (AET-WO-4300g): KiCad's netlist of the schematic (kicad-cli sch export netlist) judged against the MCU's row and the specification, each bounded capacitor measured on the board. Never reads the emitter's choices.
    Support {
        /// The .kicad_sch file, whose netlist KiCad exports.
        #[arg(long)]
        schematic: PathBuf,
        /// The .kicad_pcb file: where each pad sits, for the bounded distances.
        #[arg(long)]
        board: PathBuf,
        /// The specification: the MCU's part and supply, its domains and its connectors.
        #[arg(long)]
        spec: PathBuf,
    },
    /// Judge what a linked STM32H753 firmware writes to its GPIO registers at reset (gpio_default_config in the ELF) against a SolvedTopology: forward, reverse and package. Starts no process.
    Pinout {
        /// The SolvedTopology JSON the firmware was meant to realise.
        #[arg(long)]
        topology: PathBuf,
        /// The linked firmware ELF (ELF32, little-endian, EM_ARM).
        #[arg(long)]
        elf: PathBuf,
        /// ArduPilot's generated hwdef.h: its VAL_GPIOx_MODER/AFRL/AFRH must equal the ELF's, or the check refuses.
        #[arg(long = "hwdef-h")]
        hwdef_h: Option<PathBuf>,
        /// A declared allow-list: pins outside the topology the firmware may configure, each exactly as declared, and why.
        #[arg(long)]
        allow: Option<PathBuf>,
    },
    /// Each generated pin's output speed against DS12117 Table 149: SPI SCK and MOSI judged at their declared rate, supply and load; every other line named with why it is out of scope. No tool runs.
    Speeds {
        /// The specification.
        #[arg(long)]
        spec: PathBuf,
    },
}

#[derive(Serialize)]
struct ValidReport<'a> {
    ok: bool,
    project_name: &'a str,
    voltage_domains: usize,
    net_constraints: usize,
    /// The lifecycle banner (`AET-WO-4210` D1); absent when it is empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    lifecycle: Vec<String>,
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
    /// Who is in conflict: on every refusal of the solver, empty or not, and
    /// on a validation refusal that names someone (`MissingBusConstraint`,
    /// which `solve` made before `AET-WO-1206`), so each prints the same bytes
    /// from every command. Absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    claimants: Option<Box<[String]>>,
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
                claimants: None,
                path: None,
                message,
            },
        }
    }

    fn at(mut self, path: Option<&str>) -> Self {
        self.error.path = path.map(str::to_owned);
        self
    }

    fn claimed(mut self, claimants: Vec<String>) -> Self {
        self.error.claimants = Some(claimants.into_boxed_slice());
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
        Command::Generate {
            what: Some(GenerateCommand::Hwdef { topology, out }),
            ..
        } => {
            return match hwdef::generate(&topology, &out) {
                Ok(r) => emit(&r, 0),
                Err(stop) => stopped(stop),
            };
        }
        Command::Generate {
            what: None,
            spec: Some(spec),
            out: Some(out),
        } => return generate_tree(&spec, &out),
        Command::Generate { .. } => {
            unreachable!("clap requires --spec and --out without a subcommand")
        }
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
        Command::Check {
            check:
                CheckCommand::Support {
                    schematic,
                    board,
                    spec,
                },
        } => check_support(&schematic, &board, &spec),
        Command::Check {
            check:
                CheckCommand::Pinout {
                    topology,
                    elf,
                    hwdef_h,
                    allow,
                },
        } => {
            return match pinout::check(&topology, &elf, hwdef_h.as_deref(), allow.as_deref()) {
                Ok(v) => emit(&v, if v.judgement.ok { 0 } else { EXIT_FAIL }),
                Err(stop) => stopped(stop),
            };
        }
        Command::Check {
            check: CheckCommand::Speeds { spec },
        } => return speeds::speeds(&spec),
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
            foreign,
        } => match (firmware, spec, for_time.or(foreign)) {
            (Some(firmware), Some(spec), Some(virtual_ns)) => {
                let (parsed, topology) = match solved(&spec) {
                    Ok((parsed, topology, _)) => (parsed, topology),
                    Err(code) => return code,
                };
                let pins = match aethelgard_twin::pins::plan(
                    &pinwitness::declared(&parsed),
                    &topology,
                    &Default::default(),
                ) {
                    Ok(p) => p,
                    Err(e) => return stopped(twin_stop(e)),
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
                        open: Default::default(),
                        rail: None,
                        foreign: foreign.is_some(),
                        pins,
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
        Command::Doctor => return doctor::doctor(),
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
    time_up_to(s, aethelgard_twin::boot::MAX_VIRTUAL_NS)
}

/// `--foreign`: as `--for`, up to the foreign window
/// ([`renode::foreign::FOREIGN_MAX_VIRTUAL_NS`], `AET-WO-4400e`).
fn foreign_time(s: &str) -> Result<u64, String> {
    time_up_to(s, renode::foreign::FOREIGN_MAX_VIRTUAL_NS)
}

fn time_up_to(s: &str, max: u64) -> Result<u64, String> {
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
    let mut read_spec_value = None;
    let (rules, spec_sha256) = match spec {
        Some(file) => {
            let (spec, sha256) = read_spec(Some(file)).map_err(|(refusal, code)| run::Stop {
                code,
                kind: refusal.error.kind,
                id: refusal.error.id,
                message: refusal.error.message,
            })?;
            let stack = aethelgard_eda::stack::Stack::of(&spec).map_err(undeclared)?;
            let rules = aethelgard_eda::rules::Rules::from_stack(&stack);
            declared = Some(stack);
            read_spec_value = Some((spec.clone(), sha256.clone()));
            (rules, Some(sha256))
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
    let mut pairs = None;
    if let (Some(stack), Some(tree)) = (&declared, &board) {
        aethelgard_eda::route::declared(tree, stack).map_err(|d| run::Stop {
            code: EXIT_UNREADABLE,
            kind: d.kind,
            id: None,
            message: d.message,
        })?;
        // Declared coupled pairs are judged on the board's copper, read and
        // joined by geometry first (`AET-WO-1308` §1.3): KiCad has no rule
        // for a pair it cannot recognise by name. Extraction's allowlist runs
        // first, so copper the pair judge does not read (an arc, a stray
        // shape) is refused as `extract` refuses it (`AET-WO-3303` §1.3).
        if !stack.pairs.is_empty() {
            let read = aethelgard_eda::extract::read_tree(tree, stack).map_err(|r| run::Stop {
                code: EXIT_UNREADABLE,
                kind: r.kind,
                id: r.id,
                message: r.message,
            })?;
            let reports =
                aethelgard_eda::route::pairs(&read.board, &read.routed).map_err(|message| {
                    run::Stop {
                        code: EXIT_UNREADABLE,
                        kind: "PairNotCoupled",
                        id: None,
                        message,
                    }
                })?;
            pairs = Some(reports.into_iter().map(verdict::PairRecord::from).collect());
        }
    }
    // A power tree's layout rules, judged from the same board (`AET-WO-4300d2`).
    let mut power = None;
    let mut power_findings = Vec::new();
    if let (Some(stack), Some(tree), Some((spec, sha256))) = (&declared, &board, &read_spec_value)
        && spec.power.is_some()
    {
        let (record, findings) = power_gate(spec, sha256, stack, tree)?;
        power = Some(record);
        power_findings = findings;
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
    let version = kicad::pinned(version(invocation(
        run::args(["version"]),
        "status-version",
    ))?)?;
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
    let mut findings = kicad::judge(check, code, &report, &version)?;
    findings.extend(power_findings);
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
        v.pairs = pairs;
        v.power = power;
        v.schematic = schematic.map(|s| verdict::InputReport {
            path: s.original.display().to_string(),
            sha256: s.sha256,
        });
    }
    Ok(v)
}

/// `check support` (`AET-WO-4300g` §1.5): the schematic is copied, refused
/// unless flat, and KiCad exports its netlist in the sandbox; the netlist is
/// read strictly and judged by `aethelgard_eda::support` against the MCU's row
/// and the specification, with the board read strictly for where each pad
/// sits. A finding fails the check; a file the gate cannot read is refused.
fn check_support(
    schematic: &Path,
    board: &Path,
    spec: &Path,
) -> Result<verdict::Verdict, run::Stop> {
    let (spec, sha256) = read_spec(Some(spec)).map_err(|(refusal, code)| run::Stop {
        code,
        kind: refusal.error.kind,
        id: refusal.error.id,
        message: refusal.error.message,
    })?;
    let unreadable = |kind: &'static str| {
        move |message: String| run::Stop {
            code: EXIT_UNREADABLE,
            kind,
            id: None,
            message,
        }
    };
    let scratch = Scratch::new()?;
    let input = input::file(schematic, &scratch.tool.join(kicad::Check::Erc.copy_name()))?;
    kicad::refuse_sheets(&input.copy)?;
    kicad::prepare(
        kicad::Check::Erc,
        &input.copy,
        &scratch.tool,
        &aethelgard_eda::rules::Rules::kicad_default(),
    )?;
    let board = input::file(board, &scratch.root.join("board.kicad_pcb"))?;
    let text = fs::read(&board.copy)
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .ok_or_else(|| unreadable("MalformedBoard")("the board is not UTF-8".to_owned()))?;
    let tree = aethelgard_eda::board::read(&text).map_err(|r| run::Stop {
        code: EXIT_UNREADABLE,
        kind: r.kind(),
        id: r.id().map(str::to_owned),
        message: r.to_string(),
    })?;
    let stack = aethelgard_eda::stack::Stack::of(&spec).map_err(undeclared)?;
    let (placed, _) =
        aethelgard_eda::route::read(&tree, &stack).map_err(unreadable("BoardUnreadable"))?;
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
    let net = scratch.tool.join("input.net");
    let mut args = run::args(["sch", "export", "netlist", "--format", "kicadsexpr", "-o"]);
    args.push(net.clone().into_os_string());
    args.push(input.copy.clone().into_os_string());
    let out = invocation(args, "status-run").run()?;
    let code = out.code("kicad-cli")?;
    if code != 0 {
        return Err(run::Stop::tool_error(
            "kicad-cli",
            format!("kicad-cli sch export netlist exited {code}"),
        ));
    }
    let netlist = fs::read(&net)
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
        .ok_or_else(|| {
            run::Stop::tool_error("kicad-cli", "the exported netlist is unreadable".to_owned())
        })?;
    let tree = aethelgard_eda::strict::netlist(&netlist).map_err(|r| run::Stop {
        code: EXIT_UNREADABLE,
        kind: r.kind(),
        id: r.id().map(str::to_owned),
        message: r.to_string(),
    })?;
    let netlist =
        aethelgard_eda::support::netlist(&tree).map_err(unreadable("MalformedNetlist"))?;
    let record =
        aethelgard_eda::support::judge(&spec, &netlist, &placed).map_err(|m| run::Stop {
            code: EXIT_INVALID_SPEC,
            kind: "SupportUnjudgeable",
            id: None,
            message: m,
        })?;
    let findings = record
        .findings
        .iter()
        .map(|f| verdict::Finding {
            kind: f.kind.to_owned(),
            severity: "error".to_owned(),
            message: f.message.clone(),
            location: Vec::new(),
        })
        .collect();
    let mut v = verdict::Verdict::new(
        "support",
        &input,
        &tool,
        version,
        code,
        out.sandbox,
        findings,
    );
    v.specification_sha256 = Some(sha256);
    v.support = Some(verdict::SupportRecord::from((
        record,
        verdict::InputReport {
            path: board.original.display().to_string(),
            sha256: board.sha256,
        },
    )));
    Ok(v)
}

/// `check drc --spec` on a specification with a power tree (`AET-WO-4300d2`
/// §1.3-§1.4): the board read and its labels proved by copper, each loop rule
/// measured, each domain's source and return pad on its net (`K5`), each
/// class named with its width, and every rule the board cannot show named.
/// A rule the board breaks is a finding; a board the rules cannot be read
/// from is refused.
fn power_gate(
    spec: &Specification,
    sha256: &str,
    stack: &aethelgard_eda::stack::Stack,
    tree: &aethelgard_eda::strict::Tree,
) -> Result<(verdict::PowerRecord, Vec<verdict::Finding>), run::Stop> {
    use aethelgard_eda::{loops, route};
    let stop = |code: u8, kind: &'static str| {
        move |message: String| run::Stop {
            code,
            kind,
            id: None,
            message,
        }
    };
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part)
        .map_err(|e| stop(EXIT_INVALID_SPEC, e.kind())(e.to_string()))?;
    let solved = aethelgard_ir::solve(spec, &table, sha256)
        .map_err(|e| stop(EXIT_INVALID_SPEC, e.kind())(e.to_string()))?;
    let rules = loops::rules(spec, &solved, &table)
        .map_err(|e| stop(EXIT_INVALID_SPEC, e.kind())(e.to_string()))?;
    let (b, r) = route::read(tree, stack).map_err(stop(EXIT_UNREADABLE, "BoardUnreadable"))?;
    route::connectivity(&b, &r).map_err(stop(EXIT_UNREADABLE, "NetLabelsDisagree"))?;
    // Each buck's thermal array against its rule (count, size, sites) and each
    // through-hole pad's join, as declared (`1309` D1, D2; §1.5): KiCad's
    // `thermal_*` rule bounds a via's size and drill, not the array (adversarial
    // pass A1).
    route::check(&b, &r).map_err(stop(EXIT_UNREADABLE, "ClearanceViolated"))?;
    let records = loops::judge(&b, &rules).map_err(stop(EXIT_UNREADABLE, "LoopPartMissing"))?;
    let finding = |kind: &str, message: String| verdict::Finding {
        kind: kind.to_owned(),
        severity: "error".to_owned(),
        message,
        location: Vec::new(),
    };
    let mut findings = Vec::new();
    if let Err(m) = loops::source_pads(&b, &solved) {
        findings.push(finding("source_pad", m));
    }
    for l in records.iter().filter(|l| l.verdict == "fail") {
        let worst = l.pads.iter().map(|g| g.gap_um).max().unwrap_or(0);
        findings.push(finding(
            "loop_rule",
            format!(
                "{} is {worst} um from {} (pads {}), over the {} um bound: {}",
                l.part,
                l.ic,
                l.pads
                    .iter()
                    .map(|g| format!("{} to {}", g.pad, g.pin))
                    .collect::<Vec<_>>()
                    .join(", "),
                l.bound_um,
                l.rule
            ),
        ));
    }
    let classes = stack
        .classes
        .iter()
        .map(|c| verdict::ClassRecord {
            name: c.name.clone(),
            width_um: c.rule.width_nm / 1000,
            via_um: c
                .via_declared
                .then_some((c.rule.via_nm / 1000, c.rule.drill_nm / 1000)),
            nets: c.nets.clone(),
        })
        .collect();
    Ok((
        verdict::PowerRecord {
            loops: records.into_iter().map(verdict::LoopRecord::from).collect(),
            classes,
            conditions: loops::conditions(spec),
        },
        findings,
    ))
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

/// What `prove` runs Kani with: cargo, `cargo-kani`, the variables they read
/// and the directories they may read, `target` their target directory. One
/// definition, which `doctor`'s version probe shares (`AET-WO-1604` §1.1).
struct KaniToolchain {
    cargo: run::Tool,
    cargo_kani: run::Tool,
    env: Vec<(&'static str, OsString)>,
    read_only: Vec<PathBuf>,
}

fn kani_toolchain(target: &Path) -> Result<KaniToolchain, run::Stop> {
    let cargo = run::Tool::find("cargo")?;
    let cargo_kani = run::Tool::find("cargo-kani")?;
    let kani_home = run::required_var("KANI_HOME", true)?;
    let rustup_home = run::required_var("RUSTUP_HOME", true)?;
    let path_var = run::required_var("PATH", false)?;
    let mut read_only = cargo.dirs();
    read_only.extend(cargo_kani.dirs());
    read_only.push(PathBuf::from(&kani_home));
    read_only.push(PathBuf::from(&rustup_home));
    let env = vec![
        ("KANI_HOME", kani_home),
        ("RUSTUP_HOME", rustup_home),
        ("PATH", path_var),
        ("CARGO_TARGET_DIR", target.to_path_buf().into_os_string()),
    ];
    Ok(KaniToolchain {
        cargo,
        cargo_kani,
        env,
        read_only,
    })
}

fn prove(
    crate_dir: &Path,
    harness: &str,
    spec: Option<&Path>,
) -> Result<verdict::Verdict, run::Stop> {
    input::dir_check(crate_dir)?;
    // A --spec that cannot be generated, and a crate the pre-flight refuses
    // (`AET-WO-1603` §1.2), are refused before any tool is looked for.
    let generated = spec.map(generated_board).transpose()?;
    let scratch = Scratch::new()?;
    let input = input::crate_dir(crate_dir, &scratch.tool.join("crate"))?;
    firmware::preflight(&input.copy, &input.copy)?;
    let KaniToolchain {
        cargo,
        cargo_kani,
        env,
        read_only,
    } = kani_toolchain(&scratch.tool.join("target"))?;
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
    let invocation = |args: Vec<OsString>, cwd: &Path, status: &str| run::Invocation {
        tool: &cargo,
        args,
        env: env.clone(),
        workdir: &scratch.tool,
        cwd: cwd.to_path_buf(),
        status: scratch.root.join(status),
        read_only: read_only.clone(),
        // Cargo starts its own children with stdin from /dev/null, which
        // every tool may read and write (`run::confine`).
        read_write: Vec::new(),
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
    /// The serial ports the scenario holds open on their RX line
    /// (`AET-WO-2302` D5), by label.
    open: std::collections::BTreeSet<String>,
    /// A campaign scenario's declared droop (`AET-WO-2301`); `None` otherwise.
    rail: Option<analog::Rail>,
    /// `simulate --foreign` (`AET-WO-4400e` D3): judged by the foreign judge.
    foreign: bool,
    /// What the twin drives and reads of the specification's pin devices,
    /// with the scenario's holds and values (`AET-WO-1211c`).
    pins: aethelgard_twin::pins::Pins,
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
    /// Each motor output's observer, solved values and static check
    /// (`AET-WO-4300a`).
    motors: Vec<renode::Motor>,
    /// Each serial port's expectations and silicon check (`AET-WO-4300b`).
    serial: Vec<renode::Serial>,
    /// The boot's fresh capture directory, inside the tool's own scratch;
    /// `None` without a serial port.
    capture: Option<PathBuf>,
    /// The undisturbed boot's, for a droop with a serial port
    /// (`AET-WO-2305` §1.2); `None` otherwise.
    capture_undisturbed: Option<PathBuf>,
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
    /// The foreign firmware's memory regions, when judged as one
    /// (`AET-WO-4400e`); `None` for a generated firmware.
    foreign: Option<Vec<(&'static str, aethelgard_twin::boot::Region)>>,
    /// The foreign firmware's loaded flash segments, read back after the run;
    /// empty for a generated firmware.
    flash: Vec<elf::FlashLoad>,
    /// The generated firmware's PWR rule (`AET-WO-4240v` §1.6); `None` for a
    /// foreign firmware, which is never given it.
    pwr: Option<aethelgard_twin::serial::PwrRule>,
    /// The generated firmware's HSE bring-up rule (`AET-WO-4240b` §1.6);
    /// `None` for a board with no HSE and for a foreign firmware.
    clock: Option<aethelgard_twin::hse::ClockRule>,
    /// What the twin drives and reads of the pin devices (`AET-WO-1211c`).
    pins: aethelgard_twin::pins::Pins,
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

/// The PWR rule a boot is judged by (`AET-WO-4240v` §1.6): the generated
/// firmware's, from the topology; none for a foreign firmware, which is
/// never given it.
fn pwr_of(
    foreign: bool,
    topology: &aethelgard_ir::SolvedTopology,
) -> Result<Option<aethelgard_twin::serial::PwrRule>, run::Stop> {
    match foreign {
        true => Ok(None),
        false => aethelgard_twin::serial::pwr_rule(topology)
            .map(Some)
            .map_err(twin_stop),
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
    // Each clock serves one run kind and never crosses (`AET-WO-4400g`,
    // ruled on PR #229): the solved clock is the generated firmware's, a
    // declared one a foreign firmware's. Refused first, before any input is
    // read, as `INV-06` refuses a missing fact.
    match (boot.foreign, &boot.topology.foreign_clock) {
        (true, None) => {
            return Err(run::Stop::refused(
                "ForeignClockUndeclared",
                "--foreign needs the firmware's clock declared in the specification (`mcu.foreign_clock`, AET-WO-4400g); without it the twin would build the timers and USARTs at the solved clock, which is the generated firmware's, not this firmware's (INV-06)".to_owned(),
            ));
        }
        (false, Some(_)) => {
            return Err(run::Stop::refused(
                "ForeignClockOnGeneratedRun",
                "the specification declares `mcu.foreign_clock`, a foreign firmware's clock; the generated firmware runs at the solved clock, so only `--foreign` takes this specification (AET-WO-4400g)".to_owned(),
            ));
        }
        _ => {}
    }
    if let Err(err) = aethelgard_cli::generate::simulable(&boot.topology) {
        return Err(run::Stop {
            code: EXIT_INVALID_SPEC,
            kind: err.kind(),
            id: None,
            message: err.to_string(),
        });
    }
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
    // A foreign firmware is held to its own memory rule and none of the
    // generated runtime's contract (`AET-WO-4400e`): no boot word, no
    // self-loop, no serial injection, no motor window.
    let foreign = match boot.foreign {
        true => Some(elf::foreign_memory()?),
        false => None,
    };
    let checked = match &foreign {
        Some(regions) => elf::check_foreign(&bytes, regions)?,
        None => elf::check(&bytes, &memory, boot_word)?,
    };
    // `foreign_memory` puts flash first.
    let flash = match &foreign {
        Some(regions) => elf::flash_loads(&bytes, &regions[0].1)?,
        None => Vec::new(),
    };
    let image = aethelgard_twin::boot::expected(&boot.topology).map_err(twin_stop)?;
    let pwr = pwr_of(boot.foreign, &boot.topology)?;
    let clock = match boot.foreign {
        true => None,
        false => aethelgard_twin::hse::clock_rule(&boot.topology).map_err(twin_stop)?,
    };
    let motors = match boot.foreign {
        true => Vec::new(),
        false => renode::motors(&boot.topology).map_err(twin_stop)?,
    };
    // `AET-WO-4300b`: each port's frame is derived from the firmware's own
    // sha256. It goes in one quantum after the last boot's boot word, port
    // after port (`AET-WO-2302` D4), by the CLI's own hooks; the boot word's
    // instant is known only after the run, so `simulate` refuses before it
    // only what no boot word can save, from the earliest instant, and the
    // judge names the rest after it.
    let sha = aethelgard_twin::serial::sha256_bytes(&firmware.sha256).ok_or_else(|| {
        run::Stop::unreadable(format!(
            "the firmware's sha256 `{}` is not hex",
            firmware.sha256
        ))
    })?;
    let mut serial = match boot.foreign {
        true => Vec::new(),
        false => renode::serials(&boot.topology, &sha).map_err(twin_stop)?,
    };
    for s in &mut serial {
        s.open = boot.open.contains(&s.port.label);
    }
    let capture = match serial.is_empty() {
        true => None,
        false => Some(capture_dir(scratch, "capture")?),
    };
    // The undisturbed boot's own, for a droop (`AET-WO-2305` §1.2).
    let capture_undisturbed = match (serial.is_empty(), boot.rail.is_some()) {
        (false, true) => Some(capture_dir(scratch, "capture-undisturbed")?),
        _ => None,
    };
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
            motors,
            serial,
            capture,
            capture_undisturbed,
            boot_word,
            specification_sha256: boot.topology.specification_sha256.clone(),
            virtual_ns: boot.virtual_ns,
            topology: boot.topology,
            scenario: boot.scenario,
            held: boot.held,
            absent: boot.absent,
            rail: boot.rail,
            foreign,
            flash,
            pwr,
            clock,
            pins: boot.pins,
        },
    ))
}

/// `RecoveryBoundPastRun` (`AET-WO-2305` §1.2, ruled 2026-11-20): every
/// release's probe, at the release, its nudge and the recovery bound, must
/// lie inside the boot and before the next reset is asserted, or the droop
/// is refused by name once the bound is known, never cut short. The rail's
/// own span was checked before any run (`check_droop`).
fn recovery_fits(
    resets: &[aethelgard_chaos::power::Reset],
    bound_ns: u64,
    virtual_ns: u64,
) -> Result<(), run::Stop> {
    for (k, r) in resets.iter().enumerate() {
        let Some(release) = r.released_ns else {
            continue;
        };
        let next = resets
            .get(k + 1)
            .map_or(virtual_ns, |n| n.asserted_ns.min(virtual_ns));
        match aethelgard_chaos::power::probe_ns(release, bound_ns) {
            Some(probe) if probe <= next => {}
            probe => {
                return Err(run::Stop::refused(
                    "RecoveryBoundPastRun",
                    format!(
                        "the release at {release} ns is probed at {} ns (its boot's start and the recovery bound, {bound_ns} ns: the undisturbed boot word and one quantum), past {next} ns, {}",
                        probe.map_or_else(|| "an instant past u64".to_owned(), |p| p.to_string()),
                        if next < virtual_ns {
                            "where the next reset is asserted"
                        } else {
                            "the boot's end"
                        }
                    ),
                ));
            }
        }
    }
    Ok(())
}

/// `SerialInjectionPastWindow`: the frames go in at `injection_ns`, and the
/// firmware needs the quantum after it to echo them, all inside the board's
/// window, `MAX_VIRTUAL_NS`, named by `window`. The frames'
/// later bytes, paced a character apart (adversarial pass 4, ADV4-2), are
/// the wire budget's ([`serial_wire_fits`]), which holds them all.
fn serial_fits(injection_ns: u64) -> Result<(), run::Stop> {
    let (window, window_ns) = (
        aethelgard_twin::boot::WINDOW,
        aethelgard_twin::boot::MAX_VIRTUAL_NS,
    );
    if aethelgard_twin::serial::fits(injection_ns, window_ns) {
        return Ok(());
    }
    Err(run::Stop::refused(
        "SerialInjectionPastWindow",
        format!(
            "the serial frames go in at {injection_ns} ns and the firmware echoes them in the {} ns after, past {window} of {window_ns} ns from the earliest boot word (AET-WO-4300b, AET-WO-2302)",
            aethelgard_twin::boot::QUANTUM_NS
        ),
    ))
}

/// `SerialWireTimePastWindow` (the PM's rulings on adversarial F5 and pass
/// 2's H1): every port's pattern and one echoed character at its solved
/// actual baud, and the frame as the composition paces it (pass 4, ADV4-2),
/// summed over the ports in the specification's order (the firmware serves
/// them one after another), from the injection at `injection_ns`, inside the
/// board's window. The twin renders no pattern's wire time, so this is
/// judged before the boot.
fn serial_wire_fits(serial: &[renode::Serial], injection_ns: u64) -> Result<(), run::Stop> {
    let (window, window_ns) = (
        aethelgard_twin::boot::WINDOW,
        aethelgard_twin::boot::MAX_VIRTUAL_NS,
    );
    let Some((s, times, end)) = renode::wire_past_window(serial, injection_ns, window_ns) else {
        return Ok(());
    };
    Err(run::Stop::refused(
        "SerialWireTimePastWindow",
        format!(
            "serial port `{}` ({}, {}): {}, from the earliest injection at {injection_ns} ns, past {window} of {window_ns} ns (AET-WO-4300b, AET-WO-2302)",
            s.port.label,
            s.port.peripheral,
            aethelgard_twin::serial::describe(&s.port.framing),
            match (times, end) {
                (Some((char_ns, wire, sum)), Some(end)) => format!(
                    "its pattern ({} characters), the frame ({}) and one echoed character take {wire} ns on the wire at {} mbaud ({char_ns} ns a character, the frame's {} ns apart), {}ending at {end} ns",
                    s.pattern.len(),
                    aethelgard_twin::serial::FRAME_LEN,
                    s.silicon.actual_millibaud,
                    s.spacing_ns.unwrap_or_default(),
                    if sum > wire {
                        format!(
                            "{sum} ns after the ports before it in the specification's order, which the firmware serves first, "
                        )
                    } else {
                        String::new()
                    }
                ),
                _ => format!(
                    "its wire time at {} mbaud cannot be computed",
                    s.silicon.actual_millibaud
                ),
            }
        ),
    ))
}

/// The boot's capture directory, made fresh inside the tool's scratch before
/// any tool runs there. As root, it is the one directory there the tool uid
/// may add to (`run::add_only`): Renode creates each port's file in it, and
/// can change nothing the CLI made (`AET-WO-1601` §1.2).
fn capture_dir(scratch: &Scratch, name: &str) -> Result<PathBuf, run::Stop> {
    let dir = scratch.tool.join(name);
    DirBuilder::new().mode(0o700).create(&dir).map_err(|e| {
        run::Stop::tool_error(
            "aethelgard",
            format!("cannot create the capture directory: {e}"),
        )
    })?;
    if rustix::process::geteuid().is_root() {
        run::add_only(&dir).map_err(|e| {
            run::Stop::sandbox(
                "uid",
                format!("cannot open the capture directory to the tool: {e}"),
            )
        })?;
    }
    Ok(dir)
}

/// The capture directory after the run, read by the CLI: each entry must be
/// a regular file beneath it, reached by no link, of at most
/// [`renode::CAPTURE_MAX_BYTES`]; anything else is named, never read past.
fn read_capture(dir: &Path) -> renode::Captured {
    let mut c = renode::Captured::default();
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            c.problems.push(format!("cannot be read: {e}"));
            return c;
        }
    };
    let mut names = Vec::new();
    for entry in entries {
        match entry {
            Ok(e) => names.push(e.file_name()),
            Err(e) => c.problems.push(format!("cannot be listed: {e}")),
        }
    }
    names.sort();
    for name in names {
        let shown = name.to_string_lossy().escape_debug().to_string();
        let Some(text) = name.to_str() else {
            c.problems.push(format!("`{shown}` is not a UTF-8 name"));
            continue;
        };
        match firmware::read_beneath(dir, &dir.join(text), renode::CAPTURE_MAX_BYTES) {
            Ok(bytes) if bytes.len() as u64 > renode::CAPTURE_MAX_BYTES => c.problems.push(
                format!("`{shown}` is past {} bytes", renode::CAPTURE_MAX_BYTES),
            ),
            Ok(bytes) => c.files.push((text.to_owned(), bytes)),
            Err(e) => c
                .problems
                .push(format!("`{shown}` is not a regular file beneath it: {e}")),
        }
    }
    c
}

/// Renode as `simulate` runs it, found on `PATH`: its installation's root,
/// and what it may read. One definition, which `doctor`'s version probe
/// shares (`AET-WO-1604` §1.1).
struct Renode {
    tool: run::Tool,
    root: PathBuf,
    read_only: Vec<PathBuf>,
}

fn renode_tool() -> Result<Renode, run::Stop> {
    let tool = run::Tool::find("renode")?;
    let root = fs::canonicalize(&tool.path)
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .ok_or_else(|| {
            run::Stop::unusable("renode", "cannot resolve renode's installation".to_owned())
        })?;
    let mut read_only = tool.dirs();
    read_only.extend([
        root.clone(),
        PathBuf::from("/proc"),
        PathBuf::from("/dev/urandom"),
    ]);
    Ok(Renode {
        tool,
        root,
        read_only,
    })
}

/// The variables every Renode run gets.
fn renode_env() -> Vec<(&'static str, OsString)> {
    // .NET would otherwise try to open its diagnostic IPC socket, which the
    // seccomp filter refuses; this removes even the attempt (PR #15, S2-F).
    // W^X's double mapping truncates a memfd to 2 TiB, which RLIMIT_FSIZE
    // refuses with SIGXFSZ; without it Renode starts under the cap
    // (AET-WO-1601 §1.6, measured; FEEDBACK).
    vec![
        ("DOTNET_EnableDiagnostics", "0".into()),
        ("DOTNET_EnableWriteXorExecute", "0".into()),
    ]
}

/// The Renode a boot needs. The stock table and the boot's measured facts
/// are Renode 1.17.0's; another Renode is a fact the twin does not have
/// (`INV-06`). Its build is the tools image's,
/// [`RENODE_BUILD`](aethelgard_twin::platform::RENODE_BUILD): a stock 1.17.0
/// judges nested interrupts wrongly with no warning (`AET-WO-1605`) and
/// hard-faults unmodified ArduPilot (`AET-WO-4400f` §2), so any other build,
/// or none named, is refused by name (`AET-WO-4400g` §1.5).
fn renode_pinned(version: &str, build: Option<&str>) -> Result<(), run::Stop> {
    let pinned = format!("Renode v{}", aethelgard_twin::platform::RENODE_VERSION);
    if version != pinned {
        return Err(run::Stop::unusable(
            "renode",
            format!("`{version}` is not the {pinned} the twin's platform is pinned to"),
        ));
    }
    let want = aethelgard_twin::platform::RENODE_BUILD;
    if build != Some(want) {
        return Err(run::Stop::renode_build_not_pinned(format!(
            "Renode's build is {}, not the tools image's `{want}`, the only build the twin's judges are measured on (AET-WO-1605, AET-WO-4400g)",
            build.map_or_else(|| "not named".to_owned(), |b| format!("`{b}`"))
        )));
    }
    Ok(())
}

/// Writes the configuration every Renode run reads into `dir`, the tool's
/// scratch, and returns its path.
fn renode_config(dir: &Path) -> Result<PathBuf, run::Stop> {
    let config = dir.join(renode::CONFIG_FILE);
    fs::write(&config, renode::config(dir)).map_err(|e| {
        run::Stop::unusable(
            "renode",
            format!("cannot write Renode's configuration: {e}"),
        )
    })?;
    Ok(config)
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
    let Renode {
        tool,
        root,
        read_only,
    } = renode_tool()?;
    let (input, ready) = match ready {
        Some((input, ready)) => (input, Some(ready)),
        None => (
            input::platform(platform, &root, &scratch.tool.join("platform.repl"))?,
            None,
        ),
    };
    let invocation = |args: Vec<OsString>, status: &str| run::Invocation {
        tool: &tool,
        args,
        env: renode_env(),
        workdir: &scratch.tool,
        cwd: scratch.tool.clone(),
        status: scratch.root.join(status),
        read_only: read_only.clone(),
        read_write: Vec::new(),
    };
    // The models of our own the loaded platform declares, each the CLI's own
    // copy, hashed again as written, before any tool runs, so the copies are
    // sealed with the rest of the scratch (`AET-WO-4220` D6).
    let loaded = match &analog {
        Some((_, booted)) => booted.path.as_path(),
        None => input.copy.as_path(),
    };
    let models = own_models(loaded, &scratch.tool)?;
    // Every Renode this call starts reads one configuration, the droop's
    // through the bridge included (`AET-WO-2204` D4, F6).
    let config = renode_config(&scratch.tool)?;
    let with_config = |rest: &[&str]| {
        let mut a = run::args(["--config".as_ref(), config.as_os_str()]);
        a.extend(run::args(rest));
        a
    };
    let reported = invocation(with_config(&["--version"]), "status-version").run()?;
    let version = version_of("renode", &reported)?;
    // Every verdict names the build Renode reported (`AET-WO-4400g` §1.5),
    // and a boot's archive does (`AET-WO-2205` §1.1); every call, a plain
    // load too, refuses any build but the pinned one.
    let build = renode::build(&reported.stdout);
    renode_pinned(&version, build.as_deref())?;
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
    let plan_with = |resets, recovery_bound_ns, capture| {
        ready
            .as_ref()
            .zip(booted.as_ref())
            .map(|(r, booted)| renode::BootPlan {
                platform: &booted.path,
                models: &models,
                firmware: &r.firmware.copy,
                virtual_ns: r.virtual_ns,
                boot_word: r.boot_word,
                vector_table: r.vector_table,
                image: &r.image,
                executable: &r.executable,
                self_loops: &r.self_loops,
                motors: &r.motors,
                instances: &booted.instances,
                resets,
                recovery_bound_ns,
                serial: &r.serial,
                capture,
                pwr: r.pwr.as_ref(),
                clock: r.clock.as_ref(),
                pins: &r.pins,
            })
    };
    let resets = droop
        .as_ref()
        .map_or(&[][..], |(_, _, resets)| resets.as_slice());
    // The recovery bound (`AET-WO-2305` §1.2, ruled 2026-11-20): when the
    // droop releases a reset, the same prepared boot runs first with no
    // reset, through the same bridge, and its boot word's instant plus one
    // quantum bounds each recovery. Its own capture directory, so the
    // droop's boot opens its files new.
    let undisturbed = match plan_with(
        &[],
        None,
        ready
            .as_ref()
            .and_then(|r| r.capture_undisturbed.as_deref()),
    ) {
        Some(base) if resets.iter().any(|r| r.released_ns.is_some()) => {
            let commands = base.commands();
            let args = with_config(&["--disable-gui", "--console", "--plain", "-e", &commands]);
            let out = analog::boot(
                &invocation(args, "status-undisturbed"),
                &tool.path,
                base.log_filter(),
            )?;
            Some(renode::undisturbed_boot_word_ns(&out)?)
        }
        _ => None,
    };
    let recovery_bound_ns = undisturbed
        .flatten()
        .and_then(aethelgard_chaos::power::recovery_bound);
    if let (Some(bound), Some(r)) = (recovery_bound_ns, &ready) {
        recovery_fits(resets, bound, r.virtual_ns)?;
    }
    let plan = plan_with(
        resets,
        recovery_bound_ns,
        ready.as_ref().and_then(|r| r.capture.as_deref()),
    );
    // A rendered release moves the injection (`AET-WO-4300b`): when even
    // its earliest instant, one quantum after the last boot's start, and the
    // ports' wire time from it pass `MAX_VIRTUAL_NS`, no boot word can save
    // it, and it is refused by name before the boot runs, never cut short
    // (the ruling's amendment 3). The real instant is the judge's
    // (`serial_wire_past_run`, `AET-WO-2302`).
    if let (Some(i), Some(r)) = (
        plan.as_ref().and_then(renode::BootPlan::injection_ns),
        &ready,
    ) {
        serial_fits(i)?;
        serial_wire_fits(&r.serial, i)?;
    }
    if let (Some(r), Some((pass, booted))) = (&ready, &analog) {
        booted_is_emitted(&booted.path, r, pass)?;
    }
    let foreign = ready.as_ref().and_then(|r| r.foreign.as_ref());
    let commands = match (&plan, foreign) {
        (Some(plan), Some(_)) => {
            renode::foreign::commands(plan, &ready.as_ref().expect("foreign is ready's").flash)?
        }
        (Some(plan), None) => plan.commands(),
        (None, _) => renode::commands(&input.copy, &models),
    };
    let args = with_config(&["--disable-gui", "--console", "--plain", "-e", &commands]);
    let run = invocation(args, "status-run");
    // Only a boot is bounded (`RUL-13` as amended, PR #56 B1); a load is not.
    // A boot's log is read through the filter, so a flood reaches a verdict;
    // a droop's boot is the bridge's, which owns Renode as its child.
    let out = match (&plan, &droop) {
        (Some(plan), Some(_)) => analog::boot(&run, &tool.path, plan.log_filter())?,
        (Some(_), None) if foreign.is_some() => {
            run.run_bounded_filtered(renode::BOOT_BOUNDS, renode::foreign::FOREIGN_LOG_FILTER)?
        }
        (Some(plan), None) => run.run_bounded_filtered(renode::BOOT_BOUNDS, plan.log_filter())?,
        (None, _) => run.run()?,
    };
    // The foreign judge's report (`AET-WO-4400e`): its firmware, its
    // specification, its memory regions and every device's part.
    let foreign_inputs = ready.as_ref().and_then(|r| {
        r.foreign.as_ref().map(|regions| {
            let parts: std::collections::BTreeMap<String, String> = r
                .topology
                .i2c_devices
                .iter()
                .map(|(k, d)| (k.clone(), d.part.clone()))
                .chain(
                    r.topology
                        .spi_devices
                        .iter()
                        .map(|(k, d)| (k.clone(), d.part.clone())),
                )
                .collect();
            (
                verdict::FirmwareReport {
                    path: r.firmware.original.display().to_string(),
                    sha256: r.firmware.sha256.clone(),
                    image_sha256: r.image_sha256.clone(),
                },
                r.specification_sha256.clone(),
                renode::foreign::regions_report(regions),
                parts,
                renode::foreign::ClockReport::declared(&r.topology)
                    .expect("ready refuses a foreign run with no declared clock"),
            )
        })
    });
    let mut foreign_report = None;
    let (code, (mut findings, machine, observed)) = match (&plan, &out.ended) {
        // Killed at a bound: a fail that names it, with nothing read. A killed
        // tool has no exit code, so the verdict records -1.
        (Some(plan), run::Ended::Bounded(bound)) => {
            if let Some((firmware, spec, regions, _, clock)) = foreign_inputs.clone() {
                foreign_report = Some(renode::foreign::report(
                    plan, firmware, spec, regions, clock,
                ));
            }
            (-1, (vec![renode::bounded(&out, *bound)], None, None))
        }
        (Some(plan), _) if foreign_inputs.is_some() => {
            let code = out.code("renode")?;
            let (firmware, spec, regions, parts, clock) =
                foreign_inputs.clone().expect("the guard holds it");
            let r = ready.as_ref().expect("foreign is ready's");
            let (findings, machine, report) = renode::foreign::judge(
                code,
                &out,
                plan,
                renode::foreign::report(plan, firmware, spec, regions, clock),
                &parts,
                &r.flash,
                r.topology.clock.hse_hz,
            )?;
            foreign_report = Some(report);
            (code, (findings, machine, None))
        }
        (Some(plan), _) => {
            let code = out.code("renode")?;
            let captured = plan.capture.map(read_capture).unwrap_or_default();
            (code, renode::judge_boot(code, &out, plan, &captured)?)
        }
        (None, _) => {
            let code = out.code("renode")?;
            let (findings, machine) = renode::judge(code, &out)?;
            (code, (findings, machine, None))
        }
    };
    // The motor outputs' static check judges the topology, never the run
    // (`4300` D3), so a boot killed at a bound reports it too.
    if let Some(plan) = &plan {
        findings.extend(renode::static_motor_findings(plan.motors));
        findings.extend(renode::static_serial_findings(plan.serial));
    }
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
                undisturbed,
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
    v.tool.build.clone_from(&build);
    // The bridge's tools are this verdict's too (AET-WO-1601, PR #96).
    if let Some(bridge) = bridge {
        v.sandbox.unmet(bridge.uid_boundary_unmet);
    }
    v.machine = machine;
    if let (Some(r), Some(plan)) = (&ready, &plan) {
        match foreign_report {
            Some(report) => v.foreign_boot = Some(report),
            None => v.boot = Some(boot_report(r, plan, observed.as_ref())),
        }
        v.memory = out
            .peak_rss_kib
            .map(|k| verdict::MemoryReport::of(k, renode::BOOT_BOUNDS.rss_kib));
    }
    if let Some(r) = &ready {
        // `validate` read the topology's parts already, so the table is there.
        v.lifecycle = r.topology.lifecycle_banner().unwrap_or_default();
    }
    // What judged the boot, as each tool reported it (`AET-WO-2205` §1.1).
    if let Some(build) = build.filter(|_| ready.is_some()) {
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

/// The models of our own `platform` declares (`AET-WO-4220` D6), each written
/// from `aethelgard_renode_models` into `dir/models/`, read back and hashed:
/// - `ModelUnpinned`: the header names a model or sha256 the twin does not
///   pin, or a device is declared as one the header does not name;
/// - `ModelPinMismatch`: the copy's bytes do not hash to the pin.
///
/// Returns the copies' paths, in the header's order.
fn own_models(platform: &Path, dir: &Path) -> Result<Vec<PathBuf>, run::Stop> {
    // Lossy: a header line and a device line are ASCII either way, and a
    // platform Renode would read is not refused here for another byte.
    let bytes = fs::read(platform)
        .map_err(|e| run::Stop::unreadable(format!("cannot read the platform: {e}")))?;
    let named = aethelgard_twin::platform::own_models(&String::from_utf8_lossy(&bytes))
        .map_err(|e| run::Stop::refused(e.kind(), e.to_string()))?;
    if named.is_empty() {
        return Ok(Vec::new());
    }
    let to = dir.join("models");
    let written = |e: io::Error| {
        run::Stop::tool_error(
            "aethelgard",
            format!("cannot write a model of our own into the scratch: {e}"),
        )
    };
    fs::create_dir(&to).map_err(written)?;
    let mut out = Vec::new();
    for m in named {
        // `own_models` named only pinned models, each with a source.
        let ((file, pin), Some(text)) = (m.own().expect("a pinned model"), m.source()) else {
            return Err(run::Stop::refused(
                "ModelUnpinned",
                format!("`{}` has no source in this build", m.renode()),
            ));
        };
        let path = to.join(file);
        fs::write(&path, text).map_err(written)?;
        let back = fs::read(&path).map_err(written)?;
        let sha = input::hex(<sha2::Sha256 as sha2::Digest>::digest(&back));
        // belt-and-suspenders: the bytes written and the pin come from one
        // build, and the twin's tier-1 test holds them equal, so no test
        // reaches this refusal (adversarial F8). Ruling D6 asks for the copy
        // hashed again as written: a scratch a tool can reach is not trusted.
        if sha != pin {
            return Err(run::Stop::refused(
                "ModelPinMismatch",
                format!(
                    "the copy of `{}/{file}` hashes to sha256 `{sha}`, not its pin `{pin}`",
                    aethelgard_twin::model::OWN_DIR
                ),
            ));
        }
        out.push(path);
    }
    Ok(out)
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
        window: aethelgard_twin::boot::WINDOW,
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
        brown_out: verdict::BrownOutReport {
            level: r.image.brown_out.level,
            address: word(r.image.brown_out.word_at),
            mask: word(r.image.brown_out.mask),
            expected: word(r.image.brown_out.expected()),
            observed: observed.map(|o| word(o.brown_out)),
            flash_access: observed.and_then(|o| o.flash_access.clone()),
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
                expected: word(
                    observed
                        .and_then(|o| o.register_expected.get(i).copied())
                        .unwrap_or(reg.value),
                ),
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
                    step: c.label,
                    register: bytes(&c.write),
                    width: c.width,
                    expected: format!("{:#x}", c.expected),
                    mask: format!("{:#x}", c.mask),
                    observed: observed.map(|o| format!("{:#x}", o.records[i].1)),
                    status: observed.map(|o| renode::status_name(&c.wire, o.records[i].0)),
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
                    step: c.label,
                    command: bytes(&c.write),
                    width: c.width,
                    expected: format!("{:#x}", c.expected),
                    mask: format!("{:#x}", c.mask),
                    observed: observed.map(|o| format!("{:#x}", o.records[i].1)),
                    status: observed.map(|o| renode::status_name(&c.wire, o.records[i].0)),
                }),
                aethelgard_twin::boot::Wire::I2c { .. } => None,
            })
            .collect(),
        not_witnessable: not_witnessable_report(plan, observed),
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
                step: c.label,
                device: c.device.clone(),
                expected: c.witness.clone().unwrap_or_default(),
                observed: observed.and_then(|o| o.witnesses[i].clone()),
                unrouted: observed.map(|o| o.unrouted[i].clone()).unwrap_or_default(),
            })
            .collect(),
        icsr: observed.map(|o| word(o.icsr)),
        motor: plan
            .motors
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let mut report = verdict::MotorReport::new(m, observed.map(|o| &o.motors[i]));
                report.level = m.idle_high.map(|high| {
                    let w = observed
                        .and_then(|o| o.levels.iter().find(|w| w.label == m.observer.label));
                    verdict::LevelReport {
                        idle: level_name(high),
                        settle_ns: w.and_then(|w| w.settle_ns),
                        rises_after: w.map(|w| w.rises_after),
                        falls_after: w.map(|w| w.falls_after),
                        end_level: w.and_then(|w| w.end_level).map(level_name),
                        status: w.map(|w| w.status),
                    }
                });
                report
            })
            .collect(),
        gpio: gpio_reports(&r.pins, observed),
        adc: r
            .pins
            .analog
            .iter()
            .enumerate()
            .map(|(k, a)| {
                let w = observed.and_then(|o| o.adc.get(k));
                verdict::AdcReport {
                    net: a.net.clone(),
                    pin: a.pin.clone(),
                    peripheral: a.peripheral.clone(),
                    channel: a.channel,
                    bits: a.bits,
                    microvolts: a.microvolts,
                    held: a.held,
                    expected: a.code,
                    status: w.map(|w| w.status),
                    reads: w.map(|w| w.reads),
                    observed: w.and_then(|w| w.code),
                    residual: verdict::ADC_RESIDUAL,
                }
            })
            .collect(),
        serial: plan
            .serial
            .iter()
            .enumerate()
            .map(|(i, s)| verdict::SerialReport::new(s, observed.map(|o| &o.serial[i])))
            .collect(),
        boot_word_ns: observed.and_then(|o| o.boot_word_ns),
        stops: observed
            .map(|o| {
                o.timers
                    .iter()
                    .filter_map(|t| Some((t, t.anchor?, t.stop.as_ref()?)))
                    .map(|(t, (_, _, anchor_ns), s)| {
                        let prefix = format!("TIM{}.", t.timer);
                        verdict::StopReport {
                            timer: format!("TIM{}", t.timer),
                            anchor_ns,
                            at_ns: s.at_ns,
                            read_back: r
                                .image
                                .registers
                                .iter()
                                .filter(|reg| reg.name.starts_with(&prefix))
                                .zip(s.words.iter().map(Some).chain(std::iter::repeat(None)))
                                .map(|(reg, got)| verdict::RegisterReport {
                                    name: reg.name.clone(),
                                    address: word(reg.address),
                                    mask: word(reg.mask),
                                    expected: word(reg.value),
                                    observed: got.map(|&g| word(g)),
                                })
                                .collect(),
                            cr1_after: s.cleared.map(word),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default(),
        residuals: plan
            .motors
            .iter()
            .any(|m| m.observer.protocol.bit_rate().is_some())
            .then_some(verdict::DSHOT_STOP_RESIDUAL)
            .into_iter()
            .chain(
                r.pwr
                    .is_some()
                    .then_some(aethelgard_twin::serial::PWR_RESIDUAL),
            )
            .chain(
                r.clock
                    .is_some()
                    .then_some(aethelgard_twin::hse::HSE_RESIDUAL),
            )
            .collect(),
        executable: plan.executable.to_vec(),
    }
}

fn level_name(high: bool) -> &'static str {
    if high { "high" } else { "low" }
}

/// A pin device's GPIO signals, outputs then inputs, as the plan and the
/// witnesses give them (`AET-WO-1211c`).
fn gpio_reports(
    pins: &aethelgard_twin::pins::Pins,
    observed: Option<&renode::Observed>,
) -> Vec<verdict::GpioReport> {
    let outputs = pins.outputs.iter().map(|o| {
        (
            o.net.as_str(),
            o.pin.as_str(),
            "output",
            o.high,
            None,
            false,
        )
    });
    let inputs = pins.inputs.iter().map(|i| {
        (
            i.net.as_str(),
            i.pin.as_str(),
            "input",
            i.high,
            Some(level_name(i.active_high)),
            i.held,
        )
    });
    outputs
        .chain(inputs)
        .enumerate()
        .map(|(k, (net, pin, direction, high, active, held))| {
            let w = observed.and_then(|o| o.gpio.get(k));
            verdict::GpioReport {
                net: net.to_owned(),
                pin: pin.to_owned(),
                direction,
                level: level_name(high),
                active,
                held,
                status: w.map(|w| w.status),
                bsrr: w.and_then(|w| w.bsrr).map(verdict::word),
                ordered: w.and_then(|w| w.ordered),
                read: w.and_then(|w| w.level).map(|l| level_name(l == 1)),
                residual: verdict::GPIO_RESIDUAL,
            }
        })
        .collect()
}

/// The boot's unmodelled identity reads (`AET-WO-4241`), or `None` when the
/// board has none.
fn not_witnessable_report(
    plan: &renode::BootPlan<'_>,
    observed: Option<&renode::Observed>,
) -> Option<verdict::NotWitnessableReport> {
    let reads: Vec<verdict::NotWitnessableRead> = plan
        .image
        .identity
        .unmodelled
        .iter()
        .enumerate()
        .filter_map(|(j, u)| {
            let n = u.identity.as_ref()?;
            let seen = observed.and_then(|o| o.unmodelled.get(j));
            let on = match &u.wire {
                aethelgard_twin::boot::Wire::I2c { address } => {
                    format!("{} {address:#04x}", u.peripheral)
                }
                aethelgard_twin::boot::Wire::Spi { select, .. } => {
                    format!("{} {select}", u.peripheral)
                }
            };
            Some(verdict::NotWitnessableRead {
                reference: u.reference.clone(),
                part: u.part.clone(),
                on,
                register: bytes(&n.write),
                width: n.width,
                table_value: format!("{:#x}", n.expected),
                status: seen.map(|s| renode::status_name(&u.wire, s.record.0)),
                observed: seen.map(|s| format!("{:#x}", s.record.1)),
                cost_ns: seen.and_then(renode::Unwitnessed::cost_ns),
            })
        })
        .collect();
    if reads.is_empty() {
        return None;
    }
    let total_ns = reads.iter().map(|r| r.cost_ns).sum::<Option<u64>>();
    Some(verdict::NotWitnessableReport {
        count: reads.len(),
        reads,
        total_ns,
        twin_limit: verdict::NOT_WITNESSABLE_LIMIT,
    })
}

/// Bytes as the verdict writes them: `0xef 0xc8`.
fn bytes(write: &[u8]) -> String {
    let each: Vec<String> = write.iter().map(|b| format!("{b:#04x}")).collect();
    each.join(" ")
}

fn validate(file: Option<&Path>) -> ExitCode {
    match read_spec(file).and_then(|(spec, sha256)| {
        drawable(&spec).map_err(|r| (r, EXIT_INVALID_SPEC))?;
        acknowledgements_current(&spec, &sha256).map_err(|r| (r, EXIT_INVALID_SPEC))?;
        drawing_holds(&spec, &sha256).map_err(|r| (r, EXIT_INVALID_SPEC))?;
        Ok((spec, sha256))
    }) {
        Ok((spec, _)) => emit(
            &ValidReport {
                ok: true,
                project_name: &spec.project_name,
                voltage_domains: spec.voltage_domains.len(),
                net_constraints: spec.net_constraints.len(),
                lifecycle: spec.lifecycle_banner().unwrap_or_default(),
            },
            0,
        ),
        Err((refusal, code)) => emit(&refusal, code),
    }
}

/// `AET-WO-1210` (`AET-WO-1209` §1.3, rule 3): on a drawn board, every
/// device's part is one the emitter has an arm for, read from the emitter's
/// own set (`aethelgard_eda::schematic::KNOWN_PARTS`, held to the arms by
/// `AET-WO-1316`'s tests), so `emit`'s `UnknownPart` is refused before any
/// board. A specification that declares `board.drawn: false` is not held.
fn drawable(spec: &Specification) -> Result<(), Refusal> {
    let arms: Vec<&str> = aethelgard_eda::schematic::KNOWN_PARTS.split(", ").collect();
    spec.drawable_parts(&arms)
        .map_err(|e| Refusal::new(e.kind(), e.id(), e.to_string()).at(e.path()))
}

/// `AET-WO-1210` §1.4: a specification that declares `board.drawn: false`
/// draws no board, so `command` (`emit`, `extract` or `verify`) refuses it by
/// name before it solves, draws or reads anything. `generate` writes its
/// firmware and twin artifacts and no `kicad/` (F1).
pub(crate) fn draws_a_board(spec: &Specification, command: &str) -> Result<(), Refusal> {
    if spec.board.drawn {
        return Ok(());
    }
    Err(Refusal::new(
        "DrawsNoBoard",
        Some(&spec.project_name),
        format!(
            "`{}` declares `board.drawn: false`, a twin stand-in no board is drawn from, so `{command}` refuses it",
            spec.project_name
        ),
    )
    .at(Some("board.drawn")))
}

/// The checks that judge the drawn design, [`outline_holds`] then
/// [`loads_drawn`], read one drawing: solved and drawn once, and only when
/// either has something to judge (`AET-WO-1316`). Drawing the parts needs
/// the solve: a specification that does not solve, or that the emitter
/// refuses, is left to `solve` and `emit` to refuse by their names, as
/// `validate` always has.
fn drawing_holds(spec: &Specification, sha256: &str) -> Result<(), Refusal> {
    let loads = spec.power.iter().flat_map(|p| &p.loads);
    if spec.board.max_outline_um.is_none() && loads.clone().all(|l| l.reference.is_none()) {
        return Ok(());
    }
    let Some((table, topology)) = quiet_solve(spec, sha256) else {
        return Ok(());
    };
    let Ok(drawing) = aethelgard_eda::schematic::Drawing::of(spec, &topology, &table) else {
        return Ok(());
    };
    outline_holds(&drawing)?;
    loads_drawn(spec, &drawing)
}

/// `AET-WO-1313` §2.1: a declared `board.max_outline_um` that cannot hold
/// the parts at all (one keep-out box, or their sum, against the bound less
/// its margins) is refused by name before any placement, by the emitter's
/// own judgement (`aethelgard_eda::schematic::Drawing::bound_holds`).
fn outline_holds(drawing: &aethelgard_eda::schematic::Drawing) -> Result<(), Refusal> {
    use aethelgard_eda::schematic::EmitError;
    match drawing.bound_holds() {
        Err(e @ EmitError::OutlineTooSmall { .. }) => {
            Err(Refusal::new(e.kind(), e.id(), e.to_string()).at(Some("board.max_outline_um")))
        }
        _ => Ok(()),
    }
}

/// The pin table and solve the drawing checks judge, or `None` when either
/// is refused: `solve` and `emit` then refuse by their own names.
fn quiet_solve(
    spec: &Specification,
    sha256: &str,
) -> Option<(aethelgard_ir::PinTable, aethelgard_ir::SolvedTopology)> {
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).ok()?;
    let topology = aethelgard_ir::solve(spec, &table, sha256).ok()?;
    Some((table, topology))
}

/// `AET-WO-1209` D3, as ruled: each `power.loads` reference names a part the
/// emitter draws for this specification (`aethelgard_eda::schematic::Drawing::drawn_nets`),
/// such as a removed crossing's CS pull-up no longer; step 20a holds only the
/// form. `AET-WO-1209b`: and the part, as drawn, has a pin on the entry's
/// rail, such as a renumbered strap now on `GND` no longer. This holds every
/// form a reference takes (declared, the MCU, the reset switch, an
/// emitter-numbered `R<n>`/`C<n>`), since each names a drawn part. A wrong
/// part that is also on the rail stays unseen (a renumbered `R7`, still on
/// `VDD_3V3`: a roadmap §7.1 row).
fn loads_drawn(
    spec: &Specification,
    drawing: &aethelgard_eda::schematic::Drawing,
) -> Result<(), Refusal> {
    let Some(power) = &spec.power else {
        return Ok(());
    };
    let drawn = drawing.drawn_nets();
    for (i, l) in power.loads.iter().enumerate() {
        let Some(r) = &l.reference else {
            continue;
        };
        let path = format!("power.loads[{i}].reference");
        let refuse = |kind, message| Err(Refusal::new(kind, Some(r), message).at(Some(&path)));
        match drawn.get(r) {
            None => {
                return refuse(
                    "LoadNamesUndrawnPart",
                    format!(
                        "{path} on `{}` names `{r}`, which the emitter does not draw for this specification",
                        l.domain
                    ),
                );
            }
            Some(nets) if !nets.contains(&l.domain) => {
                let on: Vec<String> = nets.iter().map(|n| format!("`{n}`")).collect();
                return refuse(
                    "LoadPartOffRail",
                    format!(
                        "{path} on `{d}` names `{r}`, which the emitter draws with no pin on `{d}`: its pins are on {}",
                        on.join(", "),
                        d = l.domain,
                    ),
                );
            }
            Some(_) => {}
        }
    }
    Ok(())
}

/// `AET-WO-4230` R1: every `acknowledge_unwitnessed` entry names only
/// fields the witness ledger has for its register and class, the ledger
/// computed as `verify` computes it (pin table, solve, the generator's field
/// list, the classification). With entries but no ledger (the part has no
/// pin table, the specification does not solve, or the generator refuses
/// it), the first entry stands for nothing and is refused.
fn acknowledgements_current(spec: &Specification, sha256: &str) -> Result<(), Refusal> {
    use aethelgard_cli::ledger;
    let Some(first) = spec.acknowledge_unwitnessed.first() else {
        return Ok(());
    };
    // A `bypass` board is a true declaration this engine does not build
    // (`AET-WO-4240v` D-1): `validate` and `solve` accept it, and `generate`,
    // `emit`, `simulate` and `verify` refuse it as `CoreSupplyNotGenerated`
    // before any ledger is read. So no package, and no ledger, can rest on its
    // acknowledgements, and they are left unjudged here (adversarial pass A1).
    // The same holds for a declared HSE whose tree is not H1
    // (`HseNotGenerated`, `AET-WO-4240b`); an H1 board's are judged.
    if spec.mcu.core_supply.mode == aethelgard_ir::vos::CoreSupplyMode::Bypass {
        return Ok(());
    }
    let built = aethelgard_svd_hal::pin_table(&spec.mcu.part)
        .map_err(|e| e.to_string())
        .and_then(|table| {
            let topology = aethelgard_ir::solve(spec, &table, sha256).map_err(|e| e.to_string())?;
            if aethelgard_cli::generate::generable(&topology).is_err() {
                return Ok(None);
            }
            ledger::build(&topology, &table)
                .map(Some)
                .map_err(|e| e.message())
        });
    match built {
        Ok(None) => Ok(()),
        Ok(Some(l)) => match ledger::stale(&l, spec) {
            None => Ok(()),
            Some(s) => Err(
                Refusal::new(ledger::Stale::KIND, Some(&s.register), s.message())
                    .at(Some(&s.path())),
            ),
        },
        Err(why) => Err(Refusal::new(
            ledger::Stale::KIND,
            Some(&first.register),
            format!(
                "`acknowledge_unwitnessed` acknowledges register `{}`, but no witness ledger exists for this specification ({why}): an acknowledgement stands only for the ledger's rows (AET-WO-4230 R1)",
                first.register
            ),
        )
        .at(Some("acknowledge_unwitnessed[0].fields"))),
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
        let refusal = Refusal::new(err.kind(), err.id(), err.to_string()).at(err.path());
        let claimants = err.claimants();
        if claimants.is_empty() {
            (refusal, EXIT_INVALID_SPEC)
        } else {
            (refusal.claimed(claimants), EXIT_INVALID_SPEC)
        }
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
            &Refusal::new(err.kind(), err.id(), err.to_string()).claimed(err.claimants()),
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
        /// The capacitance ladder's choice, on a board that declares the
        /// fabricator's tolerances (`AET-WO-1314`).
        #[serde(skip_serializing_if = "Option::is_none")]
        capacitance_ladder: Option<serde_json::Value>,
        /// What the via ring rule changed that the specification did not
        /// declare.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        notes: Vec<String>,
    }

    let (spec, sha256) = match read_spec(Some(file)) {
        Ok(s) => s,
        Err((refusal, code)) => return emit(&refusal, code),
    };
    if let Err(r) = draws_a_board(&spec, "emit") {
        return emit(&r, EXIT_INVALID_SPEC);
    }
    let (topology, table) = match solve_spec(&spec, &sha256) {
        Ok(solved) => solved,
        Err(code) => return code,
    };
    if let Err(err) = aethelgard_cli::generate::generable(&topology) {
        return emit(
            &Refusal::new(err.kind(), None, err.to_string()),
            EXIT_INVALID_SPEC,
        );
    }
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
            capacitance_ladder: emission.ladder.as_ref().map(|l| {
                let rung = aethelgard_eda::route::RUNGS[l.rung];
                let mut v = serde_json::json!({
                    "rung": l.rung,
                    "bounded_via_cost": rung.bounded_via_cost,
                    "bounded_first": rung.bounded_first,
                    "judged_by": "extract, run separately on the emitted board (RUL-20)",
                });
                if let Some((net, c, max)) = &l.smallest {
                    v["smallest_margin"] = serde_json::json!({
                        "net": net,
                        "c_bound_femtofarads": c,
                        "declared_femtofarads": max,
                    });
                }
                if !l.unrouted.is_empty() {
                    v["unrouted_rungs"] = serde_json::json!(l.unrouted);
                }
                if let Some((kind, message)) = &l.unjudged {
                    v["unjudged"] = serde_json::json!({"kind": kind, "message": message});
                }
                v
            }),
            notes: emission.notes.clone(),
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
        /// The lifecycle banner (`AET-WO-4210` D1); absent when it is empty.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        lifecycle: Vec<String>,
        /// Why no `kicad/` is written, for a twin stand-in (`AET-WO-1210`
        /// F1); absent for a drawn board.
        #[serde(skip_serializing_if = "Option::is_none")]
        kicad: Option<&'static str>,
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
            lifecycle: topology.lifecycle_banner().unwrap_or_default(),
            kicad: (!spec.board.drawn).then_some(
                "not written: `board.drawn: false` declares a twin stand-in no board is drawn from",
            ),
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

/// A declared `fab_tolerances` block that lacks `path`, which the ring rule
/// needs: refused by every reader of a board as `emit` refuses it
/// (`AET-WO-1315` §1.6), never read with the engine's via.
fn undeclared(path: &'static str) -> run::Stop {
    let e = aethelgard_eda::schematic::EmitError::FabToleranceUndeclared(path);
    run::Stop {
        code: EXIT_INVALID_SPEC,
        kind: e.kind(),
        id: e.id().map(str::to_owned),
        message: e.to_string(),
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

    /// `AET-WO-2305` §1.2: every release's probe lies inside the boot and
    /// before the next reset's assertion, or the droop is refused by name
    /// once the bound is known.
    #[test]
    fn a_probe_past_the_boot_or_into_the_next_reset_is_refused_by_name() {
        use aethelgard_chaos::power::Reset;
        let r = |asserted_ns, released_ns| Reset {
            asserted_ns,
            released_ns,
            cause: vec!["VBOR3".to_owned()],
        };
        let one = [r(1_018_330, Some(1_231_690))];
        assert!(recovery_fits(&one, 125_700, 100_000_000).is_ok());
        // Exactly at the end fits; one ns past does not.
        let at_end = 100_000_000 - 1_231_690 - 10;
        assert!(recovery_fits(&one, at_end, 100_000_000).is_ok());
        let e = recovery_fits(&one, at_end + 1, 100_000_000).unwrap_err();
        assert_eq!(e.kind, "RecoveryBoundPastRun");
        assert!(e.message.ends_with("the boot's end"), "{}", e.message);
        // A second reset asserted before the first release's probe.
        let two = [r(1_018_330, Some(1_231_690)), r(1_300_000, Some(1_500_000))];
        let e = recovery_fits(&two, 125_700, 100_000_000).unwrap_err();
        assert!(
            e.message.ends_with("where the next reset is asserted"),
            "{}",
            e.message
        );
        // A release that never comes, and an instant past u64, both named.
        assert!(recovery_fits(&[r(1, None)], u64::MAX, 100_000_000).is_ok());
        assert!(recovery_fits(&one, u64::MAX, 100_000_000).is_err());
    }

    /// `AET-WO-4240v` §1.6: a generated boot is given the PWR rule of its
    /// declared supply; a foreign boot never is.
    #[test]
    fn only_a_generated_boot_is_given_the_pwr_rule() {
        let spec = root().join("examples/sentinel-01-minimal.json");
        let (_, topology, _) = solved(&spec).unwrap();
        let rule = pwr_of(false, &topology).unwrap().unwrap();
        assert_eq!(rule.supply, "ldo");
        assert!(pwr_of(true, &topology).unwrap().is_none());
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
