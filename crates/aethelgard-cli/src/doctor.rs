//! `aethelgard doctor`: whether this host can run every subcommand, and what
//! blocks the ones it cannot (`AET-WO-1604` §1.1), so an exit `4` is explained
//! without reading `run.rs`.
//!
//! Nothing here is a second implementation of a check:
//!
//! - **The scratch** is judged by `scratch_base`, the rule every tool call
//!   applies before it makes its directory.
//! - **The sandbox** is what the trampoline achieved when it confined a real
//!   process: this executable, asked `--version` through
//!   [`run::Invocation`], exactly as every tool is run. Landlock's ABI and
//!   the seccomp filter are learned only by installing them (`run::confine`,
//!   `run::seccomp_filter`); a layer that cannot be installed is the
//!   `SandboxUnavailable` the run refuses with.
//! - **Each tool** is found, given its variables and read grants, asked its
//!   version and judged against its pin by the same functions the subcommand
//!   that runs it calls, in the same sandbox. No tool runs beyond that probe.
//!
//! Exit `0` when nothing is missing; otherwise exit `1` and `missing` names
//! each piece and the subcommands it blocks.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use serde::Serialize;

use crate::run::{self, SandboxReport, Stop};
use crate::{EXIT_FAIL, Scratch, analog, firmware, kicad};

/// Every subcommand that makes a scratch directory and runs a tool in the
/// sandbox (each caller of `Scratch::new`).
const TOOL_SUBCOMMANDS: [&str; 8] = [
    "check drc",
    "check erc",
    "check support",
    "build",
    "prove",
    "simulate",
    "campaign",
    "verify",
];

#[derive(Serialize)]
struct Report {
    ok: bool,
    verdict: &'static str,
    check: &'static str,
    engine_version: &'static str,
    /// The scratch base, when it passes the rule.
    scratch: Option<String>,
    /// What the sandbox achieved for this executable's own probe; absent
    /// when it could not run.
    sandbox: Option<SandboxReport>,
    tools: Vec<ToolLine>,
    missing: Vec<Missing>,
}

#[derive(Serialize)]
struct ToolLine {
    tool: &'static str,
    /// Where it was found; absent when it was not.
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    /// What its probe reported; absent when it was not probed.
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    /// The sandbox its probe ran in, which `uid_boundary`'s toolchain
    /// condition judges on the tool's own read grants.
    #[serde(skip_serializing_if = "Option::is_none")]
    sandbox: Option<SandboxReport>,
    /// Set for the bridge alone: its `--version` started libngspice and
    /// accepted the library's banner.
    #[serde(skip_serializing_if = "Option::is_none")]
    ngspice_loaded: Option<bool>,
    /// Set for the bridge alone: the ngspice release that banner named
    /// (`AET-WO-1606`).
    #[serde(skip_serializing_if = "Option::is_none")]
    ngspice: Option<String>,
}

/// One missing piece: the refusal a blocked subcommand would print, and the
/// subcommands it blocks.
#[derive(Serialize)]
struct Missing {
    kind: &'static str,
    id: Option<String>,
    message: String,
    blocks: Vec<&'static str>,
}

impl Missing {
    fn of(stop: Stop, blocks: &[&'static str]) -> Self {
        Self {
            kind: stop.kind,
            id: stop.id,
            message: stop.message,
            blocks: blocks.to_vec(),
        }
    }
}

/// Adds `m`, or, when the same kind and id is already named (cargo, which
/// `build` and `prove` both need), adds the subcommands it blocks to it.
fn note(missing: &mut Vec<Missing>, m: Missing) {
    match missing
        .iter_mut()
        .find(|o| o.kind == m.kind && o.id == m.id)
    {
        Some(o) => {
            for b in m.blocks {
                // `simulate` already covers `simulate --firmware`.
                let covered = o
                    .blocks
                    .iter()
                    .any(|x| b == *x || b.strip_prefix(*x).is_some_and(|r| r.starts_with(' ')));
                if !covered {
                    o.blocks.push(b);
                }
            }
        }
        None => missing.push(m),
    }
}

const KICAD: [&str; 4] = ["check drc", "check erc", "check support", "verify"];
/// What a Renode missing, or other than the pinned version and build,
/// blocks: every `simulate`, a plain load too (`AET-WO-4400g` §1.5).
const RENODE: [&str; 3] = ["simulate", "campaign", "verify"];
const BRIDGE: [&str; 3] = ["simulate --firmware", "campaign", "verify"];
const CARGO: [&str; 3] = ["build", "prove", "verify"];
const KANI: [&str; 2] = ["prove", "verify"];

/// `aethelgard doctor`.
pub fn doctor() -> ExitCode {
    let mut missing = Vec::new();
    let (scratch, sandbox, tools) = match crate::scratch_base() {
        Ok(base) => {
            let (sandbox, tools) = probe(&mut missing);
            (Some(base.display().to_string()), sandbox, tools)
        }
        Err(stop) => {
            note(&mut missing, Missing::of(stop, &TOOL_SUBCOMMANDS));
            (None, None, unprobed())
        }
    };
    let ok = missing.is_empty();
    let report = Report {
        ok,
        verdict: if ok { "pass" } else { "fail" },
        check: "doctor",
        engine_version: env!("CARGO_PKG_VERSION"),
        scratch,
        sandbox,
        tools,
        missing,
    };
    crate::emit(&report, if ok { 0 } else { EXIT_FAIL })
}

/// With no scratch nothing can run, sandbox included: each tool is only
/// looked for on `PATH` (or beside this executable, the bridge).
fn unprobed() -> Vec<ToolLine> {
    let found = |tool: &'static str, r: Result<run::Tool, Stop>| absent(tool, r.ok().as_ref());
    vec![
        found("kicad-cli", run::Tool::find("kicad-cli")),
        found("renode", run::Tool::find("renode")),
        found(analog::BRIDGE, analog::bridge()),
        found("cargo", run::Tool::find("cargo")),
        found("cargo-kani", run::Tool::find("cargo-kani")),
    ]
}

/// A fresh scratch for one probe, as each subcommand makes one for its tool:
/// nothing one probe's tool leaves in its `HOME` reaches another's, and root
/// writes a tool's files (Renode's configuration) before that tool's
/// directory is handed over, never after another tool had it.
fn fresh(missing: &mut Vec<Missing>) -> Option<Scratch> {
    match Scratch::new() {
        Ok(s) => Some(s),
        Err(stop) => {
            note(missing, Missing::of(stop, &TOOL_SUBCOMMANDS));
            None
        }
    }
}

/// The sandbox, then each tool, each probe in a scratch of its own.
fn probe(missing: &mut Vec<Missing>) -> (Option<SandboxReport>, Vec<ToolLine>) {
    let own = match fresh(missing) {
        Some(scratch) => own(&scratch),
        None => return (None, unprobed()),
    };
    let sandbox = match own {
        Ok(p) => Some(p.sandbox),
        Err(stop) => {
            // A layer that cannot be installed refuses every tool call; any
            // other failure leaves the sandbox unknown, which blocks nothing
            // by itself but is not a pass.
            let blocks: &[&'static str] = if stop.kind == "SandboxUnavailable" {
                &TOOL_SUBCOMMANDS
            } else {
                &[]
            };
            note(missing, Missing::of(stop, blocks));
            None
        }
    };
    let tools = vec![
        kicad_cli(missing),
        renode(missing),
        bridge(missing),
        cargo(missing),
        kani(missing),
    ];
    (sandbox, tools)
}

/// A probe that ran: the version its tool printed and the sandbox it ran in.
struct Probed {
    version: String,
    /// The `build:` line of the output, which only Renode prints
    /// (`AET-WO-4400g` §1.5).
    build: Option<String>,
    sandbox: SandboxReport,
}

fn invocation<'a>(
    scratch: &'a Scratch,
    tool: &'a run::Tool,
    args: Vec<OsString>,
    env: Vec<(&'static str, OsString)>,
    read_only: Vec<PathBuf>,
    status: &str,
) -> run::Invocation<'a> {
    run::Invocation {
        tool,
        args,
        env,
        workdir: &scratch.tool,
        cwd: scratch.tool.clone(),
        status: scratch.root.join(status),
        read_only,
        read_write: Vec::new(),
    }
}

/// Runs a version probe and reads it as every subcommand reads its tool's.
fn ran(inv: run::Invocation<'_>) -> Result<Probed, Stop> {
    let out = inv.run()?;
    probed(inv.tool.name, &out)
}

/// As [`ran`], under the hang guard `simulate` puts on its own `renode
/// --version` (`AET-WO-1919` (2c)): a Renode that never answers is killed
/// and refused, never waited for.
fn ran_renode(inv: run::Invocation<'_>) -> Result<Probed, Stop> {
    let out = inv.run_bounded(crate::renode::LOAD_BOUNDS)?;
    if matches!(out.ended, run::Ended::Bounded(_)) {
        return Err(crate::renode::hung(
            "its `--version`",
            crate::renode::LOAD_BOUNDS.wall,
        ));
    }
    probed(inv.tool.name, &out)
}

fn probed(name: &str, out: &run::Output) -> Result<Probed, Stop> {
    Ok(Probed {
        version: crate::version_of(name, out)?,
        build: crate::renode::build(&out.stdout),
        sandbox: out.sandbox,
    })
}

/// This executable, asked its version in the sandbox every tool runs in.
fn own(scratch: &Scratch) -> Result<Probed, Stop> {
    let me = std::env::current_exe()
        .map_err(|e| Stop::tool_error("aethelgard", format!("cannot find this executable: {e}")))?;
    let tool = run::Tool {
        name: "aethelgard",
        path: me,
    };
    ran(invocation(
        scratch,
        &tool,
        run::args(["--version"]),
        Vec::new(),
        tool.dirs(),
        "status-sandbox",
    ))
}

fn absent(tool: &'static str, path: Option<&run::Tool>) -> ToolLine {
    ToolLine {
        tool,
        path: path.map(|t| t.path.display().to_string()),
        version: None,
        sandbox: None,
        ngspice_loaded: None,
        ngspice: None,
    }
}

/// A found tool's line: its probe, then its pin. A probe that failed blocks
/// `blocks`; a version off its pin blocks `pinned_for`.
fn finish(
    tool: &'static str,
    found: &run::Tool,
    probed: Result<Probed, Stop>,
    pin: impl FnOnce(&str, Option<&str>) -> Result<(), Stop>,
    (blocks, pinned_for): (&[&'static str], &[&'static str]),
    missing: &mut Vec<Missing>,
) -> ToolLine {
    match probed {
        Err(stop) => {
            note(missing, Missing::of(stop, blocks));
            absent(tool, Some(found))
        }
        Ok(p) => {
            if let Err(stop) = pin(&p.version, p.build.as_deref()) {
                note(missing, Missing::of(stop, pinned_for));
            }
            ToolLine {
                version: Some(p.version),
                sandbox: Some(p.sandbox),
                ..absent(tool, Some(found))
            }
        }
    }
}

/// `kicad-cli version`, as every subcommand that runs it asks and pins it:
/// an unpinned version blocks all of [`KICAD`] (`check support` since
/// `AET-WO-1607` (1)).
fn kicad_cli(missing: &mut Vec<Missing>) -> ToolLine {
    let tool = match run::Tool::find("kicad-cli") {
        Ok(t) => t,
        Err(stop) => {
            note(missing, Missing::of(stop, &KICAD));
            return absent("kicad-cli", None);
        }
    };
    let Some(scratch) = fresh(missing) else {
        return absent("kicad-cli", Some(&tool));
    };
    let scratch = &scratch;
    let probed = ran(invocation(
        scratch,
        &tool,
        run::args(["version"]),
        Vec::new(),
        tool.dirs(),
        "status-kicad-cli",
    ));
    let pin = |v: &str| kicad::pinned(v.to_owned()).map(drop);
    finish(
        "kicad-cli",
        &tool,
        probed,
        |v, _| pin(v),
        (&KICAD, &KICAD),
        missing,
    )
}

/// `renode --config <file> --version`, as `simulate` asks it, with its
/// variables, grants and hang guard; pinned as a boot pins it.
fn renode(missing: &mut Vec<Missing>) -> ToolLine {
    let r = match crate::renode_tool() {
        Ok(r) => r,
        Err(stop) => {
            note(missing, Missing::of(stop, &RENODE));
            return absent("renode", run::Tool::find("renode").ok().as_ref());
        }
    };
    let Some(scratch) = fresh(missing) else {
        return absent("renode", Some(&r.tool));
    };
    let scratch = &scratch;
    let probed = crate::renode_config(&scratch.tool).and_then(|config| {
        let mut args = run::args(["--config".as_ref(), config.as_os_str()]);
        args.push("--version".into());
        ran_renode(invocation(
            scratch,
            &r.tool,
            args,
            crate::renode_env(),
            r.read_only.clone(),
            "status-renode",
        ))
    });
    finish(
        "renode",
        &r.tool,
        probed,
        crate::renode_pinned,
        (&RENODE, &RENODE),
        missing,
    )
}

/// The bridge's `--version`: it starts libngspice and prints its identity,
/// which must be the one this CLI was built with and name the library's
/// banner (`analog::identified`), then exits `0`. It runs no circuit.
fn bridge(missing: &mut Vec<Missing>) -> ToolLine {
    let tool = match analog::bridge() {
        Ok(t) => t,
        Err(stop) => {
            note(missing, Missing::of(stop, &BRIDGE));
            return absent(analog::BRIDGE, None);
        }
    };
    let Some(scratch) = fresh(missing) else {
        return absent(analog::BRIDGE, Some(&tool));
    };
    let scratch = &scratch;
    let probed = invocation(
        scratch,
        &tool,
        run::args(["--version"]),
        Vec::new(),
        tool.dirs(),
        "status-bridge",
    )
    .run_bounded(analog::BRIDGE_BOUNDS)
    .and_then(|out| identity(&out).map(|names| (names, out.sandbox)));
    match probed {
        Err(stop) => {
            note(missing, Missing::of(stop, &BRIDGE));
            absent(analog::BRIDGE, Some(&tool))
        }
        Ok(((version, ngspice), sandbox)) => ToolLine {
            version: Some(version),
            sandbox: Some(sandbox),
            ngspice_loaded: Some(true),
            ngspice: Some(ngspice),
            ..absent(analog::BRIDGE, Some(&tool))
        },
    }
}

/// The bridge's version and the ngspice it named, from an identity
/// `analog::identified` accepted, after a `--version` that exited `0`.
fn identity(out: &run::Output) -> Result<(String, String), Stop> {
    let code = out.code(analog::BRIDGE)?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    if stdout.trim().is_empty() {
        let said = String::from_utf8_lossy(&out.stderr);
        let said: String = said
            .lines()
            .next()
            .unwrap_or_default()
            .chars()
            .filter(|c| c.is_ascii_graphic() || *c == ' ')
            .take(240)
            .collect();
        return Err(Stop::unusable(
            analog::BRIDGE,
            format!("it printed nothing and exited {code}; it said: {said}"),
        ));
    }
    let (named, rest) = analog::identified(&stdout)?;
    let said = rest.lines().next().unwrap_or_default().escape_debug();
    match (code, named.ngspice) {
        (0, Some(ngspice)) if rest.is_empty() => Ok((named.version, ngspice)),
        _ => Err(Stop::unusable(
            analog::BRIDGE,
            format!(
                "its --version exited {code}, not 0 with its identity naming ngspice and nothing else; it said: {said}"
            ),
        )),
    }
}

/// `cargo --version`, with `build`'s variables and grants.
fn cargo(missing: &mut Vec<Missing>) -> ToolLine {
    let found = || run::Tool::find("cargo").ok();
    let Some(scratch) = fresh(missing) else {
        return absent("cargo", found().as_ref());
    };
    let scratch = &scratch;
    let t = match firmware::toolchain(&scratch.tool.join("target")) {
        Ok(v) => v,
        Err(stop) => {
            note(missing, Missing::of(stop, &CARGO));
            return absent("cargo", found().as_ref());
        }
    };
    let probed = ran(invocation(
        scratch,
        &t.cargo,
        run::args(["--version"]),
        t.env.clone(),
        t.read_only.clone(),
        "status-cargo",
    ));
    finish(
        "cargo",
        &t.cargo,
        probed,
        |_, _| Ok(()),
        (&CARGO, &[]),
        missing,
    )
}

/// `cargo kani --version`, with `prove`'s variables and grants.
fn kani(missing: &mut Vec<Missing>) -> ToolLine {
    let found = || run::Tool::find("cargo-kani").ok();
    let Some(scratch) = fresh(missing) else {
        return absent("cargo-kani", found().as_ref());
    };
    let scratch = &scratch;
    let k = match crate::kani_toolchain(&scratch.tool.join("target")) {
        Ok(v) => v,
        Err(stop) => {
            note(missing, Missing::of(stop, &KANI));
            return absent("cargo-kani", found().as_ref());
        }
    };
    let probed = ran(invocation(
        scratch,
        &k.cargo,
        run::args(["kani", "--version"]),
        k.env.clone(),
        k.read_only.clone(),
        "status-kani",
    ));
    finish(
        "cargo-kani",
        &k.cargo_kani,
        probed,
        |_, _| Ok(()),
        (&KANI, &[]),
        missing,
    )
}
