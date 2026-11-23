//! `simulate --firmware`'s analog pre-pass (`AET-WO-2200` §1.4, `RUL-17`).
//!
//! Before the boot, each I2C line of the solved topology is measured:
//! 1. `aethelgard-twin::analog` writes one netlist per line;
//! 2. the bridge, a sandboxed tool beside this executable, measures each
//!    line's 30 %–70 % rise time with libngspice;
//! 3. each rise is recorded as `analog.rise_time_computed` (from declared values, `RUL-19`);
//! 4. a line past its ceiling, still rising when the window ends, or never
//!    low when released faults its bus: `bridge.fault_state` is
//!    recorded, and the bus's devices are left out of the platform the boot
//!    loads, so the fault applies from the first instruction (ruling L5).
//!
//! The measurement and the firmware's effect are separate records, and the
//! effect is labelled as the model's rendering of a NACK, never as a measured
//! waveform (`AET-WO-2101` S6). A bridge that is missing, fails, is killed or
//! answers anything but one answer per line stops the call: nothing is read
//! as "no fault" (`INV-06`). What the design does to a line is never a stop.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::Path;

use aethelgard_chaos::campaign;
use aethelgard_chaos::power::{self, Crossing};
use aethelgard_chaos::record::RailDroop;
use aethelgard_ir::SolvedTopology;
use aethelgard_twin::analog::{self, Deck};
use serde_json::{Value, json};

use crate::run;
use crate::verdict::{AnalogRecord, FaultRecord, Finding};

/// The bridge binary, found in this executable's own directory: the name
/// the archive header gives it, too.
pub const BRIDGE: &str = campaign::BRIDGE;

/// The ngspice release the bridge is pinned to; an answer naming another is refused.
pub const NGSPICE: &str = "44.2";

/// What the firmware sees of a faulted bus, and why it is not a measurement.
pub const RENDERING: &str = "the bus's devices are absent from the platform. Renode 1.17.0 models no \
NACK, so the firmware sees the absence as a TXIS timeout: the model's rendering of the NACK an \
unreadable address draws, not a measured waveform (AET-WO-2101 S6)";

/// The pass's records and the findings that fail the verdict.
pub struct Pass {
    pub analog: Vec<AnalogRecord>,
    pub faults: Vec<FaultRecord>,
    pub findings: Vec<Finding>,
    /// The bridge's sandbox, when it ran: its `uid_boundary` joins the boot's.
    pub sandbox: Option<run::SandboxReport>,
    /// When the bridge ran: its identity digest and the ngspice its answers
    /// named, as it reported them (`AET-WO-2205` §1.1).
    pub tools: Vec<(&'static str, String)>,
}

impl Pass {
    /// The buses the boot's platform leaves devices out of, read from the
    /// recorded fault decision itself.
    pub fn faulted(&self) -> BTreeSet<String> {
        self.faults.iter().map(|f| f.bus.clone()).collect()
    }
}

/// The digest of the bridge's build inputs that this CLI expects its bridge
/// to name (`bridge/build.rs`): for `Cargo.toml`, `Cargo.lock`, `build.rs`,
/// then every file in `src/` by name, each one's path, a NUL, its length as 8
/// bytes little-endian, and its bytes. It is a constant, not read from
/// `bridge/` at build time, because a published CLI carries no file outside
/// its own package; a test recomputes it from the bridge's files on disk, so
/// an edit to the bridge fails the build's tests until this is updated with it.
pub const BRIDGE_SOURCE_SHA256: &str =
    "aacbc9b5cd6835e26c0ae27f75b3bc674031519cefc9d364d32a6b49ce2d7808";

/// The bridge's identity, as [`identified`] accepted it.
#[derive(Debug, PartialEq)]
pub struct Identity {
    /// The bridge's version.
    pub version: String,
    /// The digest of the sources it was built from, for the archive's header
    /// (`AET-WO-2205` §1.1).
    pub source_sha256: String,
    /// The ngspice release its library's banner named, in a run that started
    /// the library and accepted it (`AET-WO-1606`); none otherwise.
    pub ngspice: Option<String>,
    /// With it, the sha256 of the file the bridge's loader mapped the library
    /// from, lower-case hex (`AET-WO-2310`).
    pub ngspice_library_sha256: Option<String>,
}

/// The internal name the library's sha256 travels under beside
/// [`campaign::NGSPICE`] in a verdict's tools (`AET-WO-2310`): the campaign
/// folds it into ngspice's header entry, and a boot whose bridges mapped two
/// different files stops on it as two versions. Never a header name.
pub const NGSPICE_LIBRARY: &str = "ngspice-library";

/// The most bytes the CLI reads of the library file (libngspice 44.2 is
/// 8.9 MB): a larger file is refused, never hashed in part.
const MAX_LIBRARY_BYTES: u64 = 256 << 20;

/// The sha256 of the library file the bridge named, lower-case hex: an
/// absolute, canonical path to a regular file of at most
/// [`MAX_LIBRARY_BYTES`].
fn library_sha256(path: &str) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let p = Path::new(path);
    if !p.is_absolute() || fs::canonicalize(p).ok().as_deref() != Some(p) {
        return Err(format!("`{path}` is not an absolute, canonical path"));
    }
    let file = fs::File::open(p).map_err(|e| format!("`{path}`: {e}"))?;
    let meta = file.metadata().map_err(|e| format!("`{path}`: {e}"))?;
    if !meta.is_file() || meta.len() > MAX_LIBRARY_BYTES {
        return Err(format!(
            "`{path}` is not a regular file of at most {MAX_LIBRARY_BYTES} bytes"
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAX_LIBRARY_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("`{path}`: {e}"))?;
    if bytes.len() as u64 > MAX_LIBRARY_BYTES {
        return Err(format!(
            "`{path}` grew past {MAX_LIBRARY_BYTES} bytes while read"
        ));
    }
    Ok(Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

impl Identity {
    /// The ngspice a run that answered must have named: a bridge whose
    /// identity names none started no library, so its answers are refused.
    fn ran(&self) -> Result<(String, String), run::Stop> {
        self.ngspice
            .clone()
            .zip(self.ngspice_library_sha256.clone())
            .ok_or_else(|| {
                run::Stop::tool_error(
                    BRIDGE,
                    "it answered, but its identity names no ngspice".to_owned(),
                )
            })
    }
}

/// The bridge's first line (`AET-WO-2200` F15): its name, version and the
/// digest of the sources it was built from, which must be the ones this CLI
/// was built with, and, when it started libngspice, the banner the library
/// printed and the release read from it, which must be [`NGSPICE`]
/// (`AET-WO-1606`). A stale bridge, one from another tree, or one that says
/// anything else first is refused (`BridgeMismatch`, exit 4) before any of
/// its answer is read; the identity and the rest of its stdout are returned.
pub fn identified(stdout: &str) -> Result<(Identity, &str), run::Stop> {
    let (first, rest) = stdout.split_once('\n').unwrap_or((stdout, ""));
    let mismatch = |message: String| run::Stop {
        code: crate::EXIT_TOOL_UNAVAILABLE,
        kind: "BridgeMismatch",
        id: Some(BRIDGE.to_owned()),
        message,
    };
    let want = BRIDGE_SOURCE_SHA256;
    let v: Value = serde_json::from_str(first).map_err(|_| {
        mismatch(format!(
            "its first line is not its identity: `{}`",
            first.escape_debug()
        ))
    })?;
    const BARE: [&str; 3] = ["bridge", "source_sha256", "version"];
    const LOADED: [&str; 6] = [
        "bridge",
        "ngspice",
        "ngspice_banner",
        "ngspice_library",
        "source_sha256",
        "version",
    ];
    let o = v
        .as_object()
        .filter(|o| {
            let keys: Vec<&str> = o.keys().map(String::as_str).collect();
            keys == BARE || keys == LOADED
        })
        .ok_or_else(|| {
            mismatch(format!(
                "its identity is not its three fields, or six with ngspice: `{}`",
                first.escape_debug()
            ))
        })?;
    let field = |k: &str| o.get(k).and_then(Value::as_str);
    let (Some(BRIDGE), Some(version)) = (field("bridge"), field("version")) else {
        return Err(mismatch(format!(
            "it does not name itself: `{}`",
            first.escape_debug()
        )));
    };
    let source_sha256 = match field("source_sha256") {
        Some(named) if named == want => named.to_owned(),
        named => {
            return Err(mismatch(format!(
                "it was built from sources {}, not the {want} this CLI was built with: rebuild it",
                named.unwrap_or("(none)")
            )));
        }
    };
    // belt-and-suspenders: the bridge parses and pins its banner (`solver::pinned`); the CLI holds
    // the line it reports to the same form, since it cannot link the bridge's parser.
    let (ngspice, ngspice_library_sha256) = match (
        field("ngspice"),
        field("ngspice_banner"),
        field("ngspice_library"),
    ) {
        (None, None, None) if o.len() == BARE.len() => (None, None),
        (Some(release), Some(banner), Some(library))
            if release == NGSPICE && banner == format!("** ngspice-{release} shared library") =>
        {
            let hash = library_sha256(library).map_err(|e| {
                mismatch(format!(
                    "the ngspice library it names cannot be hashed: {e}"
                ))
            })?;
            (Some(release.to_owned()), Some(hash))
        }
        _ => {
            return Err(mismatch(format!(
                "its identity does not name the ngspice {NGSPICE} its banner says: `{}`",
                first.escape_debug()
            )));
        }
    };
    Ok((
        Identity {
            version: version.to_owned(),
            source_sha256,
            ngspice,
            ngspice_library_sha256,
        },
        rest,
    ))
}

/// The bridge beside this executable, or `BridgeUnavailable` (exit 4).
pub fn bridge() -> Result<run::Tool, run::Stop> {
    let unavailable = |message: String| run::Stop {
        code: crate::EXIT_TOOL_UNAVAILABLE,
        kind: "BridgeUnavailable",
        id: Some(BRIDGE.to_owned()),
        message,
    };
    let me = std::env::current_exe()
        .map_err(|e| unavailable(format!("cannot find this executable: {e}")))?;
    let path = me
        .parent()
        .map(|d| d.join(BRIDGE))
        .ok_or_else(|| unavailable("this executable has no directory".to_owned()))?;
    match fs::metadata(&path) {
        Ok(m) if m.is_file() => Ok(run::Tool { name: BRIDGE, path }),
        _ => Err(unavailable(format!(
            "{} is not beside this executable, at {}",
            BRIDGE,
            path.display()
        ))),
    }
}

/// Lines a campaign scenario holds low, and the scenario's name
/// (`AET-WO-2300`). `simulate --firmware` passes none.
pub struct Held<'a> {
    pub scenario: &'a str,
    pub lines: &'a [crate::campaign::HeldLow],
}

/// Measures every analog line of `topology` in `dir`, a directory of its own
/// that only the bridge writes; `status` is the reaper's, outside it. A held
/// line is not measured: the bridge's decision is the fault, from the
/// scenario, without a measurement.
pub fn pass(
    topology: &SolvedTopology,
    held: &Held<'_>,
    dir: &Path,
    status: &Path,
) -> Result<Pass, run::Stop> {
    let decks: Vec<Deck> = analog::decks(topology)
        .map_err(crate::twin_stop)?
        .into_iter()
        .filter(|d| !held.lines.iter().any(|h| h.net == d.net))
        .collect();
    // Each record states its bus's declared speed beside the deck's window
    // (`AET-WO-2205` §1.3).
    // belt-and-suspenders: `SolvedTopology::validate` (run by `decks`) already
    // refuses a line whose bus has no ceiling (`UnknownBus`) and speeds that
    // name other buses than the ceilings (`SpeedBusMismatch`), so this cannot
    // fire today; it keeps an IR change from becoming a panic at `declared[bus]`.
    if let Some(d) = decks
        .iter()
        .find(|d| !topology.i2c_speeds.contains_key(&d.bus))
    {
        return Err(run::Stop::tool_error(
            "aethelgard",
            format!(
                "bus `{}` of net `{}` declares no speed in the solved topology",
                d.bus, d.net
            ),
        ));
    }
    let mut pass = measure(&decks, &topology.i2c_speeds, dir, status)?;
    for line in held.lines {
        pass.findings.push(Finding::plain(
            campaign::HELD_LOW_FINDING,
            format!(
                "bus `{}`: net `{}` is held low by scenario `{}`; the bus is faulted without a measurement",
                line.bus, line.net, held.scenario
            ),
        ));
        pass.faults.push(FaultRecord {
            kind: "bridge.fault_state",
            source: format!("scenario {}", held.scenario),
            bus: line.bus.clone(),
            fault: "absent",
            from: vec![campaign::HELD_LOW],
            nets: vec![line.net.clone()],
            firmware_sees: RENDERING,
        });
    }
    Ok(pass)
}

/// Runs the bridge on `decks`; with none, there is nothing to measure.
fn measure(
    decks: &[Deck],
    declared: &BTreeMap<String, u32>,
    dir: &Path,
    status: &Path,
) -> Result<Pass, run::Stop> {
    if decks.is_empty() {
        return Ok(Pass {
            analog: Vec::new(),
            faults: Vec::new(),
            findings: Vec::new(),
            sandbox: None,
            tools: Vec::new(),
        });
    }
    let tool = bridge()?;
    fs::DirBuilder::new()
        .mode(0o700)
        .create(dir)
        .map_err(|e| run::Stop::tool_error(BRIDGE, format!("cannot create its directory: {e}")))?;
    let request = dir.join("request.json");
    fs::write(&request, request_json(decks).to_string())
        .map_err(|e| run::Stop::tool_error(BRIDGE, format!("cannot write its request: {e}")))?;
    let out = run::Invocation {
        tool: &tool,
        args: vec![request.into_os_string()],
        env: Vec::new(),
        workdir: dir,
        cwd: dir.to_path_buf(),
        status: status.to_path_buf(),
        read_only: tool.dirs(),
        read_write: Vec::new(),
    }
    .run_bounded(crate::renode::BOOT_BOUNDS)?;
    let code = out.code(BRIDGE)?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let (identity, stdout) = identified(&stdout)?;
    if code != 0 {
        return Err(run::Stop::tool_error(
            BRIDGE,
            format!(
                "exited {code}: {} {}",
                stdout.trim(),
                String::from_utf8_lossy(&out.stderr).trim()
            ),
        ));
    }
    let (ngspice, library) = identity.ran()?;
    let measured =
        answers(decks, stdout, &ngspice).map_err(|e| run::Stop::tool_error(BRIDGE, e))?;
    let mut pass = judge(decks, &measured, declared, &ngspice);
    pass.sandbox = Some(out.sandbox);
    pass.tools = vec![
        (BRIDGE, identity.source_sha256),
        (campaign::NGSPICE, ngspice),
        (NGSPICE_LIBRARY, library),
    ];
    Ok(pass)
}

/// A declared droop, with what the runner resolved for it: the domain's
/// nominal and tolerance, and the emitter's decoupling on it (ruled D7).
#[derive(Clone, Debug)]
pub struct Rail {
    /// The droop as rendered: its instant placed from the undisturbed boot
    /// word, its level the specification's (`AET-WO-2310`).
    pub droop: RailDroop,
    pub nominal_millivolts: u32,
    pub tolerance_permille: u32,
    pub decoupling_picofarads: u64,
}

/// The rail ngspice computed: every crossing of the active thresholds'
/// levels, in time order, and the rail's minimum.
pub struct Solved {
    pub crossings: Vec<Crossing>,
    pub minimum_microvolts: u64,
    pub sandbox: run::SandboxReport,
    /// The bridge's identity digest, and the ngspice its crossing lines
    /// named (none when the rail crossed no level), as it reported them.
    pub tools: Vec<(&'static str, String)>,
}

/// Solves `rail` once with the bridge (`--rail`, ruled D1), over `span_ns`,
/// in `dir`, a directory of its own; `status` is the reaper's. The request
/// is integers only; every answer line is checked and anything else stops
/// the call (`INV-06`).
pub fn rail(rail: &Rail, span_ns: u64, dir: &Path, status: &Path) -> Result<Solved, run::Stop> {
    let tool = bridge()?;
    fs::DirBuilder::new()
        .mode(0o700)
        .create(dir)
        .map_err(|e| run::Stop::tool_error(BRIDGE, format!("cannot create its directory: {e}")))?;
    let (r, d) = (&rail.droop, &rail.droop.droop);
    let levels = power::levels(r.bor_level);
    let request = dir.join("rail.json");
    let body = json!({
        "domain": d.domain,
        "nominal_microvolts": u64::from(rail.nominal_millivolts) * 1000,
        "floor_microvolts": u64::from(d.floor_millivolts) * 1000,
        "start_ns": r.start_ns,
        "fall_ns": d.fall_ns,
        "hold_ns": d.hold_ns,
        "rise_ns": d.rise_ns,
        "source_milliohms": d.source_milliohms,
        "load_microamps": u64::from(d.load_milliamps) * 1000,
        "decoupling_picofarads": rail.decoupling_picofarads,
        "span_ns": span_ns,
        "levels": levels.iter().map(|(name, uv)| json!({"name": name, "microvolts": uv})).collect::<Vec<_>>(),
    });
    fs::write(&request, body.to_string())
        .map_err(|e| run::Stop::tool_error(BRIDGE, format!("cannot write its request: {e}")))?;
    let out = run::Invocation {
        tool: &tool,
        args: vec!["--rail".into(), request.into_os_string()],
        env: Vec::new(),
        workdir: dir,
        cwd: dir.to_path_buf(),
        status: status.to_path_buf(),
        read_only: tool.dirs(),
        read_write: Vec::new(),
    }
    .run_bounded(crate::renode::BOOT_BOUNDS)?;
    let code = out.code(BRIDGE)?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let (identity, stdout) = identified(&stdout)?;
    let fail = |m: String| run::Stop::tool_error(BRIDGE, m);
    if code != 0 {
        return Err(fail(format!(
            "exited {code} solving the rail: {} {}",
            stdout.trim(),
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    let (ngspice, library) = identity.ran()?;
    let (crossings, minimum_microvolts) = rail_answer(stdout, &levels, &ngspice).map_err(fail)?;
    let mut tools = vec![(BRIDGE, identity.source_sha256)];
    // Every crossing line named the identity's ngspice or was refused; a rail
    // that crossed nothing named none.
    if !crossings.is_empty() {
        tools.push((campaign::NGSPICE, ngspice));
        tools.push((NGSPICE_LIBRARY, library));
    }
    Ok(Solved {
        crossings,
        minimum_microvolts,
        sandbox: out.sandbox,
        tools,
    })
}

/// One battery-fed sink's floor crossing, as the chain's deck answered it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SinkCrossing {
    pub name: String,
    pub floor_microvolts: u64,
    /// The first falling crossing of its floor, ps; `None` when none.
    pub at_ps: Option<u64>,
}

/// The battery chain as the deck solved it (`AET-WO-2304b` §1.8).
pub struct ChainSolved {
    pub minimum_microvolts: u64,
    pub crossings: Vec<SinkCrossing>,
    pub sandbox: run::SandboxReport,
    /// The bridge's identity digest and the ngspice its answer named.
    pub tools: Vec<(&'static str, String)>,
}

/// Solves the battery chain once with the bridge (`--chain`, `RUL-17`):
/// `V_open` behind the path R, which steps from the healthy R to the sag's
/// at its `start_ns`; the bulk capacitors with their ESR maximum and the
/// input ceramics as ideal; one constant-power sink per battery-fed
/// regulator, at its higher floor. The request is integers and checked
/// names; the bridge writes the deck. In `dir`, a directory of its own;
/// `status` is the reaper's.
pub fn chain(
    sag: &aethelgard_chaos::record::BatterySag,
    start_ns: u64,
    side: &aethelgard_ir::power::BatterySide,
    chain: &aethelgard_chaos::battery::SagChain,
    span_ns: u64,
    dir: &Path,
    status: &Path,
) -> Result<ChainSolved, run::Stop> {
    let tool = bridge()?;
    fs::DirBuilder::new()
        .mode(0o700)
        .create(dir)
        .map_err(|e| run::Stop::tool_error(BRIDGE, format!("cannot create its directory: {e}")))?;
    let bulk: Vec<Value> = side
        .capacitors
        .iter()
        .filter_map(|c| {
            c.esr_max_milliohms
                .map(|esr| json!({"picofarads": c.picofarads, "esr_milliohms": esr}))
        })
        .collect();
    let ceramics: u64 = side
        .capacitors
        .iter()
        .filter(|c| c.esr_max_milliohms.is_none())
        .map(|c| c.picofarads)
        .sum();
    let sinks: Vec<Value> = chain
        .sinks
        .iter()
        .map(|k| {
            json!({"name": k.name, "milliwatts": k.input_milliwatts,
                   "floor_microvolts": k.floor_microvolts()})
        })
        .collect();
    let body = json!({
        "v_open_microvolts": chain.v_open_microvolts,
        "healthy_milliohms": chain.healthy_milliohms,
        "fault_milliohms": sag.series_milliohms,
        "start_ns": start_ns,
        "span_ns": span_ns,
        "bulk": bulk,
        "ceramics_picofarads": ceramics,
        "sinks": sinks,
    });
    let request = dir.join("chain.json");
    fs::write(&request, body.to_string())
        .map_err(|e| run::Stop::tool_error(BRIDGE, format!("cannot write its request: {e}")))?;
    let out = run::Invocation {
        tool: &tool,
        args: vec!["--chain".into(), request.into_os_string()],
        env: Vec::new(),
        workdir: dir,
        cwd: dir.to_path_buf(),
        status: status.to_path_buf(),
        read_only: tool.dirs(),
        read_write: Vec::new(),
    }
    .run_bounded(crate::renode::BOOT_BOUNDS)?;
    let code = out.code(BRIDGE)?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let (identity, stdout) = identified(&stdout)?;
    let fail = |m: String| run::Stop::tool_error(BRIDGE, m);
    if code != 0 {
        return Err(fail(format!(
            "exited {code} solving the battery chain: {} {}",
            stdout.trim(),
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    let (ngspice, library) = identity.ran()?;
    let (minimum_microvolts, crossings) = chain_answer(stdout, chain, &ngspice).map_err(fail)?;
    Ok(ChainSolved {
        minimum_microvolts,
        crossings,
        sandbox: out.sandbox,
        tools: vec![
            (BRIDGE, identity.source_sha256),
            (campaign::NGSPICE, ngspice),
            (NGSPICE_LIBRARY, library),
        ],
    })
}

/// The bridge's chain answer: one line, every field the mode writes and no
/// other, its sinks the request's in its order, named by the ngspice its
/// identity named.
fn chain_answer(
    stdout: &str,
    chain: &aethelgard_chaos::battery::SagChain,
    ngspice: &str,
) -> Result<(u64, Vec<SinkCrossing>), String> {
    let lines: Vec<&str> = stdout.lines().collect();
    let [line] = lines.as_slice() else {
        return Err(format!(
            "the chain answer is {} lines, not one",
            lines.len()
        ));
    };
    let Ok(Value::Object(o)) = serde_json::from_str::<Value>(line) else {
        return Err(format!("`{}` is not a JSON object", line.escape_debug()));
    };
    const FIELDS: [&str; 9] = [
        "accepted_points",
        "chain",
        "final_microvolts",
        "initial_microvolts",
        "minimum_at_ns",
        "minimum_microvolts",
        "ngspice",
        "sinks",
        "step_ns",
    ];
    let keys: Vec<&str> = o.keys().map(String::as_str).collect();
    if keys != FIELDS
        || o["chain"] != "solved"
        || o["ngspice"] != ngspice
        || [
            "accepted_points",
            "final_microvolts",
            "initial_microvolts",
            "minimum_at_ns",
            "step_ns",
        ]
        .iter()
        .any(|k| o[*k].as_u64().is_none())
    {
        return Err(format!("`{line}` is not the chain's answer"));
    }
    let minimum = o["minimum_microvolts"]
        .as_u64()
        .ok_or_else(|| format!("`{line}` holds no minimum"))?;
    let sinks = o["sinks"].as_array().map_or(&[][..], Vec::as_slice);
    if sinks.len() != chain.sinks.len() {
        return Err(format!(
            "`{line}` answers {} sinks, not {}",
            sinks.len(),
            chain.sinks.len()
        ));
    }
    let mut out = Vec::new();
    for (v, k) in sinks.iter().zip(&chain.sinks) {
        let ok = v.as_object().is_some_and(|s| s.len() == 3)
            && v["name"] == k.name.as_str()
            && v["floor_microvolts"].as_u64() == Some(k.floor_microvolts())
            && (v["at_ps"].is_null() || v["at_ps"].is_u64());
        if !ok {
            return Err(format!("`{v}` is not the answer for sink `{}`", k.name));
        }
        out.push(SinkCrossing {
            name: k.name.clone(),
            floor_microvolts: k.floor_microvolts(),
            at_ps: v["at_ps"].as_u64(),
        });
    }
    Ok((minimum, out))
}

/// The bridge's rail answer: crossing lines in time order, each of a
/// requested level at its requested value and naming `ngspice`, then one
/// closing line.
fn rail_answer(
    stdout: &str,
    levels: &[(String, u64)],
    ngspice: &str,
) -> Result<(Vec<Crossing>, u64), String> {
    let lines: Vec<&str> = stdout.lines().collect();
    let Some((last, crossing_lines)) = lines.split_last() else {
        return Err("the rail answer is empty".to_owned());
    };
    let parse = |l: &str| -> Result<serde_json::Map<String, Value>, String> {
        match serde_json::from_str::<Value>(l) {
            Ok(Value::Object(o)) => Ok(o),
            _ => Err(format!("`{}` is not a JSON object", l.escape_debug())),
        }
    };
    let end = parse(last)?;
    let minimum = match (
        end.len(),
        end.get("rail"),
        end.get("minimum_microvolts").and_then(Value::as_u64),
        end.get("accepted_points").and_then(Value::as_u64),
    ) {
        (3, Some(Value::String(s)), Some(m), Some(_)) if s == "solved" => m,
        _ => return Err(format!("`{last}` is not the rail's closing line")),
    };
    let mut out = Vec::new();
    for l in crossing_lines {
        let o = parse(l)?;
        let text = |k: &str| o.get(k).and_then(Value::as_str);
        let level = text("level").ok_or_else(|| format!("`{l}` names no level"))?;
        let uv = o.get("microvolts").and_then(Value::as_u64);
        let at_ps = o.get("at_ps").and_then(Value::as_u64);
        let falling = o.get("falling").and_then(Value::as_bool);
        let ok = o.len() == 6
            && text("method") == Some("interpolated")
            && text("ngspice") == Some(ngspice)
            && levels.iter().any(|(n, v)| n == level && Some(*v) == uv);
        let (true, Some(microvolts), Some(at_ps), Some(falling)) = (ok, uv, at_ps, falling) else {
            return Err(format!("`{l}` is not a crossing of a requested level"));
        };
        out.push(Crossing {
            level: level.to_owned(),
            microvolts,
            at_ps,
            falling,
        });
    }
    if !out.windows(2).all(|w| w[0].at_ps <= w[1].at_ps) {
        return Err("the crossings are not in time order".to_owned());
    }
    // A declared sag crosses a level at most twice; more is a rail that
    // dithers across it, which `check_droop` keeps scenarios from asking for
    // (`AET-WO-2301` adversarial F4), and is never read as resets.
    for (name, _) in levels {
        if out.iter().filter(|c| &c.level == name).count() > 2 {
            return Err(format!("the rail crossed `{name}` more than twice"));
        }
    }
    Ok((out, minimum))
}

/// The boot of a droop scenario: the bridge owns Renode as its child
/// (`--renode`, ruled D1), with Renode's own arguments, environment and
/// grants; Renode's output is the bridge's after its identity line, read
/// through `filter`, the boot's own ([`crate::renode::LOG_FILTER`], or
/// [`crate::renode::LOG_FILTER_HSE`] on an HSE board).
pub fn boot(
    inv: &run::Invocation<'_>,
    renode: &Path,
    filter: run::LogFilter,
) -> Result<run::Output, run::Stop> {
    let tool = bridge()?;
    let mut args: Vec<std::ffi::OsString> = vec!["--renode".into(), renode.as_os_str().into()];
    args.extend(inv.args.iter().cloned());
    let mut read_only = inv.read_only.clone();
    read_only.extend(tool.dirs());
    let mut out = run::Invocation {
        tool: &tool,
        args,
        env: inv.env.clone(),
        workdir: inv.workdir,
        cwd: inv.cwd.clone(),
        status: inv.status.clone(),
        read_only,
        read_write: inv.read_write.clone(),
    }
    .run_bounded_filtered(crate::renode::BOOT_BOUNDS, filter)?;
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let rest = identified(&text)?.1.as_bytes().to_vec();
    out.stdout = rest;
    Ok(out)
}

/// What a probe must read for the firmware to have recovered: the boot word,
/// the first identity check's status OK and expected value, and that check's
/// device witness since the release, exactly the model's lines (`AET-WO-2301`
/// adversarial F1: the record alone is the firmware's own claim;
/// `AET-WO-2204`: Renode's log is not). A reading the image has no check for
/// is vacuous.
fn recovered_words(
    image: &aethelgard_twin::boot::Image,
    boot_word: u32,
    probe: Option<&crate::renode::Probe>,
) -> (
    [crate::verdict::Word; 3],
    Option<crate::verdict::WitnessReport>,
) {
    use crate::verdict::{WitnessReport, Word, word};
    let check = image.identity.checks.first();
    let w = |address: u32, expected: u32, observed: Option<u32>| Word {
        address: word(address),
        expected: word(expected),
        observed: observed.map(word),
    };
    let words = [
        w(
            boot_word,
            aethelgard_twin::boot::BOOT_MAGIC,
            probe.map(|p| p.boot_word),
        ),
        w(
            check.map_or(0, |c| c.status_at),
            check.map_or(0, |_| aethelgard_twin::boot::STATUS_OK),
            probe.map(|p| p.record.map_or(0, |r| r.0)),
        ),
        w(
            check.map_or(0, |c| c.value_at),
            // The first check is an identity read, at most four bytes. A
            // masked identity is compared whole here: it can fail a probe,
            // never pass one (`AET-WO-4210`, a named residual).
            check.map_or(0, |c| u32::try_from(c.expected).unwrap_or(u32::MAX)),
            probe.map(|p| p.record.map_or(0, |r| r.1)),
        ),
    ];
    let witness =
        check.map(|c| WitnessReport::of(c, probe.and_then(|p| p.witness.clone()), Vec::new()));
    (words, witness)
}

/// Whether a probe's witness shows the first device brought up again
/// ([`power::witness_recovers`], `AET-WO-2305` §1.3): its identity read's
/// lines, then only that device's later steps, in the image's order. No
/// witness (no identity read) is vacuous; lines not read whole, or a step
/// with no known format first, never show it.
fn witnessed(
    image: &aethelgard_twin::boot::Image,
    w: Option<&crate::verdict::WitnessReport>,
) -> bool {
    let Some(w) = w else { return true };
    let checks = &image.identity.checks;
    let script: Vec<&[String]> = checks
        .iter()
        .filter(|c| checks.first().is_some_and(|f| f.device == c.device))
        .map(|c| c.witness.as_deref().unwrap_or_default())
        .collect();
    w.observed
        .as_deref()
        .is_some_and(|o| power::witness_recovers(&script, o))
}

/// Whether a probe's reads show the firmware recovered: the boot word and
/// the first identity record's status and value each read as expected, and
/// its witness shows that device brought up again ([`witnessed`]). A wrong
/// identity fails twice over, by its value and by its line (`AET-WO-2305`
/// §1.3): still `brownout_unrecovered`, by name.
fn holds(
    image: &aethelgard_twin::boot::Image,
    words: [&crate::verdict::Word; 3],
    witness: Option<&crate::verdict::WitnessReport>,
) -> bool {
    words
        .iter()
        .all(|w| w.observed.as_deref() == Some(w.expected.as_str()))
        && witnessed(image, witness)
}

/// A droop's records and findings (`AET-WO-2301` §2.2, §2.3): the declared
/// rail, ngspice's crossings, the rendered resets, each release's probe;
/// `brownout_reset` (info) per reset, `brownout_unrecovered` per reset the
/// firmware did not recover from within the bound (or never released, or
/// whose probe a killed boot never read, or with no bound to probe at), and
/// `rail_below_tolerance` (info) when the rail left the domain's tolerance.
/// `probes` is `None` when the boot was killed at a bound. `undisturbed` is
/// the undisturbed boot's boot word instant (`AET-WO-2305` §1.2): `None`
/// when no reset was released, so none was run; `Some(None)` when it wrote
/// no boot word, so there is no bound.
pub fn droop_report(
    rail: &Rail,
    solved: &Solved,
    resets: &[power::Reset],
    probes: Option<&[crate::renode::Probe]>,
    image: &aethelgard_twin::boot::Image,
    boot_word: u32,
    undisturbed: Option<Option<u64>>,
) -> (crate::verdict::DroopReport, Vec<Finding>) {
    use crate::verdict::{
        CrossingReport, DroopReport, ObservedReport, RailReport, RecoveryBoundReport, ResetReport,
    };
    let bound = undisturbed.flatten().and_then(power::recovery_bound);
    let (placed, d) = (&rail.droop, &rail.droop.droop);
    let mut findings = Vec::new();
    let mut reports = Vec::new();
    for r in resets {
        let cause = r.cause.join(", ");
        reports.push(ResetReport {
            kind: "bridge.reset_rendered",
            source: "bridge",
            at_ns: r.asserted_ns,
            released: false,
            cause: r.cause.clone(),
            rendering: power::ASSERT_RENDERING,
        });
        findings.push(Finding::info(
            campaign::RESET_FINDING,
            match r.released_ns {
                Some(at) => format!(
                    "{cause} held the part in reset from {} ns to {at} ns: the model's rendering of the supervisor, not a measured reset",
                    r.asserted_ns
                ),
                None => format!("{cause} held the part in reset from {} ns to the run's end", r.asserted_ns),
            },
        ));
        if let Some(at) = r.released_ns {
            reports.push(ResetReport {
                kind: "bridge.reset_rendered",
                source: "bridge",
                at_ns: at,
                released: true,
                cause: r.cause.clone(),
                rendering: power::RELEASE_RENDERING,
            });
        } else {
            findings.push(Finding::plain(
                campaign::UNRECOVERED_FINDING,
                format!(
                    "{cause} never released the reset asserted at {} ns",
                    r.asserted_ns
                ),
            ));
        }
    }
    let mut observed = Vec::new();
    let releases: Vec<u64> = resets.iter().filter_map(|r| r.released_ns).collect();
    for (k, at) in releases.iter().enumerate() {
        let Some(probe_at) = bound.and_then(|b| power::probe_ns(*at, b)) else {
            findings.push(Finding::plain(
                campaign::UNRECOVERED_FINDING,
                format!(
                    "no recovery bound for the release at {at} ns: the same boot, run first with no reset, wrote no boot word that could be read, so recovery cannot be judged"
                ),
            ));
            continue;
        };
        let Some(probe) = probes.map(|p| p.get(k)) else {
            findings.push(Finding::plain(
                campaign::UNRECOVERED_FINDING,
                format!("the boot was killed before the read at {probe_at} ns, after the release at {at} ns"),
            ));
            continue;
        };
        let ([boot, status, value], witness) = recovered_words(image, boot_word, probe);
        if !holds(image, [&boot, &status, &value], witness.as_ref()) {
            findings.push(Finding::plain(
                campaign::UNRECOVERED_FINDING,
                format!(
                    "{} ns after the boot that the release at {at} ns started (the undisturbed boot's {} ns and one quantum) the firmware had not recovered: boot word {}, status {}, value {}, witness {}",
                    probe_at - at - power::NUDGE_NS,
                    undisturbed.flatten().unwrap_or_default(),
                    boot.observed.as_deref().unwrap_or("unread"),
                    status.observed.as_deref().unwrap_or("unread"),
                    value.observed.as_deref().unwrap_or("unread"),
                    match witness.as_ref().map(|w| &w.observed) {
                        None => "none".to_owned(),
                        Some(None) => "unread".to_owned(),
                        Some(Some(lines)) => format!("{lines:?}"),
                    }
                ),
            ));
        }
        observed.push(ObservedReport {
            kind: "firmware.observed",
            source: "renode",
            at_ns: probe_at,
            boot_word: boot,
            status,
            value,
            // The judge read the whole witness; the report keeps the lines
            // kind 19 archives (`AET-WO-2204` B4, `AET-WO-2205` §1.8).
            witness: witness.and_then(|w| w.observed),
        });
    }
    // belt-and-suspenders: `check_droop` refused a rail whose levels are not
    // stated before any run; a floor of 0 here would only add the finding.
    let (floor, _) = campaign::rail_levels(d, rail.nominal_millivolts).unwrap_or((0, 0));
    let tolerance_floor =
        campaign::tolerance_floor_uv(rail.nominal_millivolts, rail.tolerance_permille);
    if floor < tolerance_floor {
        findings.push(Finding::info(
            campaign::BELOW_TOLERANCE_FINDING,
            format!(
                "domain `{}`: the declared droop takes the rail to {floor} µV, below its tolerance, {tolerance_floor} µV",
                d.domain
            ),
        ));
    }
    let report = DroopReport {
        recovery_bound: undisturbed.map(|instant| RecoveryBoundReport {
            kind: "twin.recovery_bound",
            source: "renode",
            undisturbed_boot_word_ns: instant,
            margin_ns: power::RECOVERY_MARGIN_NS,
            bound_ns: bound,
        }),
        rail: RailReport {
            kind: "analog.rail_declared",
            source: "scenario",
            domain: d.domain.clone(),
            nominal_millivolts: rail.nominal_millivolts,
            tolerance_permille: rail.tolerance_permille,
            floor_millivolts: d.floor_millivolts,
            start_ns: placed.start_ns,
            fall_ns: d.fall_ns,
            hold_ns: d.hold_ns,
            rise_ns: d.rise_ns,
            source_milliohms: d.source_milliohms,
            load_milliamps: d.load_milliamps,
            decoupling_picofarads: rail.decoupling_picofarads,
            bor_level: placed.bor_level,
            minimum_microvolts: solved.minimum_microvolts,
            thresholds: "DS12117 Rev 10 Table 16, typical (read from st.com; equal to Rev 7's, AET-WO-2205)",
        },
        crossings: solved
            .crossings
            .iter()
            .map(|c| CrossingReport {
                kind: "analog.rail_crossing_computed",
                source: "ngspice",
                level: c.level.clone(),
                microvolts: c.microvolts,
                at_ps: c.at_ps,
                falling: c.falling,
            })
            .collect(),
        resets: reports,
        observed,
    };
    (report, findings)
}

fn request_json(decks: &[Deck]) -> Value {
    let lines: Vec<Value> = decks
        .iter()
        .map(|d| {
            json!({
                "net": d.net,
                "netlist": d.netlist,
                "source": analog::SOURCE,
                "stimulus": d.stimulus,
                "probe": analog::PROBE,
                "release_ps": d.release_ps,
                "low_uv": d.low_uv,
                "high_uv": d.high_uv,
                "stop_ps": d.stop_ps,
            })
        })
        .collect();
    json!({ "lines": lines })
}

/// What the bridge found on one line (ruled F7 (b) on PR #70).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Answer {
    /// Its 30 %–70 % rise, in ns.
    Rise(f64),
    /// Still rising when the window ended; at least this many ns.
    Incomplete(f64),
    /// At or above 30 % when the driver released it, in volts.
    StuckHigh(f64),
}

/// One answer per deck, in the request's order, each exactly one of the
/// bridge's three documented lines, naming `ngspice`, the release its
/// identity read from the library's banner; anything else is refused.
fn answers(decks: &[Deck], stdout: &str, ngspice: &str) -> Result<Vec<Answer>, String> {
    let lines: Vec<&str> = stdout.lines().collect();
    if lines.len() != decks.len() {
        return Err(format!(
            "answered {} lines for {} nets: {stdout}",
            lines.len(),
            decks.len()
        ));
    }
    decks
        .iter()
        .zip(lines)
        .map(|(deck, line)| {
            let v: Value =
                serde_json::from_str(line).map_err(|e| format!("`{line}` is not JSON: {e}"))?;
            answer(deck, &v, ngspice)
                .ok_or_else(|| format!("`{line}` is not an answer for `{}`", deck.net))
        })
        .collect()
}

fn answer(deck: &Deck, v: &Value, ngspice: &str) -> Option<Answer> {
    let o = v.as_object()?;
    let ours = o.get("net").and_then(Value::as_str) == Some(deck.net.as_str())
        && o.get("method").and_then(Value::as_str) == Some("interpolated")
        && o.get("ngspice").and_then(Value::as_str) == Some(ngspice);
    let value = |key: &str| o.get(key).and_then(Value::as_f64).filter(|x| x.is_finite());
    let positive = |key: &str| value(key).filter(|x| *x > 0.0);
    let answer = match o.len() {
        4 if o.contains_key("rise_30_70_ns") => Answer::Rise(positive("rise_30_70_ns")?),
        4 if o.contains_key("volts_at_release") => Answer::StuckHigh(value("volts_at_release")?),
        5 if o.get("complete") == Some(&Value::Bool(false)) => {
            Answer::Incomplete(positive("rise_30_70_ns_lower_bound")?)
        }
        _ => return None,
    };
    ours.then_some(answer)
}

/// Why a bus is faulted, as its fault record's `from` names it.
const PAST_CEILING: &str = "analog.rise_time_computed > ceiling_ns";
const STUCK_HIGH: &str = "analog.line_stuck_high";

/// `declared` holds each deck's bus: [`pass`] refuses a deck whose bus has
/// no declared speed before the bridge runs.
fn judge(
    decks: &[Deck],
    answers: &[Answer],
    declared: &BTreeMap<String, u32>,
    ngspice: &str,
) -> Pass {
    let mut analog_records = Vec::new();
    let mut findings = Vec::new();
    let mut faulted: Vec<(String, Vec<String>, Vec<&'static str>)> = Vec::new();
    for (deck, &answer) in decks.iter().zip(answers) {
        let (bus, net, ceiling) = (&deck.bus, &deck.net, deck.ceiling_ns);
        let mut record = AnalogRecord {
            kind: "analog.rise_time_computed",
            source: "ngspice",
            bus: bus.clone(),
            net: net.clone(),
            complete: true,
            rise_30_70_ns: None,
            rise_30_70_ns_lower_bound: None,
            volts_at_release: None,
            ceiling_ns: ceiling,
            exceeds: false,
            method: "interpolated",
            ngspice: ngspice.to_owned(),
            deck_hz: analog::DECK_HZ,
            declared_hz: declared[bus],
        };
        let cause = match answer {
            Answer::Rise(rise) => {
                record.rise_30_70_ns = Some(rise);
                record.exceeds = rise > f64::from(ceiling);
                record.exceeds.then(|| {
                    Finding::plain(
                        campaign::PAST_CEILING_FINDING,
                        format!(
                            "bus `{bus}`: net `{net}` rose from 30 % to 70 % in {rise} ns, past its {ceiling} ns ceiling"
                        ),
                    )
                })
                .map(|f| (f, PAST_CEILING))
            }
            // Always past the ceiling: a charging line slows as it rises, so a
            // rise unfinished 20 µs after the release is at least 4/7 of that,
            // 11.4 µs, against at most 1,000 ns for any I2C mode.
            Answer::Incomplete(bound) => {
                record.complete = false;
                record.rise_30_70_ns_lower_bound = Some(bound);
                record.exceeds = true;
                let f = Finding::plain(
                    campaign::PAST_CEILING_FINDING,
                    format!(
                        "bus `{bus}`: net `{net}` had not reached 70 % when the window ended: its 30 % to 70 % rise is more than {bound} ns (a lower bound), past its {ceiling} ns ceiling"
                    ),
                );
                Some((f, PAST_CEILING))
            }
            Answer::StuckHigh(volts) => {
                record.kind = STUCK_HIGH;
                record.complete = false;
                record.volts_at_release = Some(volts);
                let f = Finding::plain(
                    campaign::STUCK_HIGH_FINDING,
                    format!(
                        "bus `{bus}`: net `{net}` was at {volts} V when its driver released it, at or above 30 % of the rail: the driver cannot pull it low"
                    ),
                );
                Some((f, STUCK_HIGH))
            }
        };
        analog_records.push(record);
        if let Some((finding, from)) = cause {
            findings.push(finding);
            match faulted.iter_mut().find(|(b, _, _)| b == bus) {
                Some((_, nets, causes)) => {
                    nets.push(net.clone());
                    if !causes.contains(&from) {
                        causes.push(from);
                    }
                }
                None => faulted.push((bus.clone(), vec![net.clone()], vec![from])),
            }
        }
    }
    faulted.sort();
    let faults = faulted
        .into_iter()
        .map(|(bus, nets, from)| FaultRecord {
            kind: "bridge.fault_state",
            source: "aethelgard".to_owned(),
            bus,
            fault: "absent",
            from,
            nets,
            firmware_sees: RENDERING,
        })
        .collect();
    Pass {
        analog: analog_records,
        faults,
        findings,
        sandbox: None,
        tools: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    use super::*;

    /// `AET-WO-2200` F15: the digest this CLI expects is the one the bridge's
    /// build script computes from the bridge's files as they are now; an edit
    /// to the bridge fails here until [`BRIDGE_SOURCE_SHA256`] moves with it.
    #[test]
    fn the_expected_bridge_digest_is_the_bridges_build_inputs_now() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bridge");
        let mut files = vec![
            "Cargo.toml".to_owned(),
            "Cargo.lock".to_owned(),
            "build.rs".to_owned(),
        ];
        let mut src: Vec<String> = fs::read_dir(root.join("src"))
            .unwrap()
            .map(|e| format!("src/{}", e.unwrap().file_name().to_str().unwrap()))
            .collect();
        src.sort();
        files.extend(src);
        let mut h = Sha256::new();
        for f in &files {
            let bytes = fs::read(root.join(f)).unwrap();
            h.update(f.as_bytes());
            h.update([0u8]);
            h.update((bytes.len() as u64).to_le_bytes());
            h.update(&bytes);
        }
        assert_eq!(
            crate::input::hex(<[u8; 32]>::from(h.finalize())),
            BRIDGE_SOURCE_SHA256,
            "the bridge changed: update BRIDGE_SOURCE_SHA256"
        );
    }

    /// `AET-WO-2305` §1.3, on Sentinel-03's image (its first identity read is
    /// U7's, a BMP581 with a configuration and a data read): a probe after
    /// the bring-up recovers with the device's later lines; a wrong identity,
    /// a repeated identity read (a boot that started over) or a stray line
    /// does not, nor does an unset boot word.
    #[test]
    fn a_probe_recovers_by_its_reads_and_its_first_devices_own_lines() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bytes =
            fs::read(root.join("crates/aethelgard-cli/tests/fixtures/sentinel-03.json")).unwrap();
        let spec: aethelgard_ir::Specification = serde_json::from_slice(&bytes).unwrap();
        let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
        let topology = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
        let image = aethelgard_twin::boot::expected(&topology).unwrap();
        let checks = &image.identity.checks;
        let first = &checks[0];
        let script: Vec<String> = checks
            .iter()
            .filter(|c| c.device == first.device)
            .flat_map(|c| c.witness.clone().unwrap())
            .collect();
        assert!(
            script.len() > first.witness.as_ref().unwrap().len(),
            "U7 has later steps"
        );
        let boot_word = aethelgard_twin::boot::boot_word().unwrap();
        let identity = u32::try_from(first.expected).unwrap();
        let judge = |boot: u32, value: u32, lines: Vec<String>| {
            let probe = crate::renode::Probe {
                at_ns: 0,
                boot_word: boot,
                record: Some((aethelgard_twin::boot::STATUS_OK, value)),
                witness: Some(lines),
            };
            let ([b, s, v], w) = recovered_words(&image, boot_word, Some(&probe));
            holds(&image, [&b, &s, &v], w.as_ref())
        };
        let magic = aethelgard_twin::boot::BOOT_MAGIC;
        assert!(judge(magic, identity, script.clone()));
        assert!(judge(magic, identity, first.witness.clone().unwrap()));
        // A wrong identity: its value, and its line as the model would log it.
        assert!(!judge(magic, identity ^ 1, script.clone()));
        let mut wrong = script.clone();
        wrong[0] = format!("{}?", wrong[0]);
        assert!(!judge(magic, identity, wrong));
        let mut again = script.clone();
        again.extend(first.witness.clone().unwrap());
        assert!(!judge(magic, identity, again));
        let mut stray = script.clone();
        stray.insert(1, format!("{} stray", first.level.1));
        assert!(!judge(magic, identity, stray));
        assert!(!judge(0, identity, script));
    }

    /// A stale or impostor bridge is refused before any of its answer is read.
    #[test]
    fn a_bridge_that_does_not_name_this_trees_sources_first_is_refused() {
        let me = BRIDGE_SOURCE_SHA256;
        let ok =
            format!(r#"{{"bridge":"aethelgard-bridge","source_sha256":"{me}","version":"0.1.0"}}"#);
        assert_eq!(
            identified(&format!("{ok}\nrest\n")).unwrap(),
            (
                Identity {
                    version: "0.1.0".to_owned(),
                    source_sha256: me.to_owned(),
                    ngspice: None,
                    ngspice_library_sha256: None,
                },
                "rest\n"
            )
        );
        // `AET-WO-1606`: a run that started the library names its banner;
        // `AET-WO-2310`: and the file it was mapped from, whose bytes the CLI
        // hashes. This test binary stands in for the library's file.
        let file = std::fs::canonicalize(std::env::current_exe().unwrap()).unwrap();
        let library = file.to_str().unwrap();
        let want: String = {
            use sha2::{Digest, Sha256};
            Sha256::digest(std::fs::read(&file).unwrap())
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect()
        };
        let loaded = format!(
            r#"{{"bridge":"aethelgard-bridge","ngspice":"44.2","ngspice_banner":"** ngspice-44.2 shared library","ngspice_library":"{library}","source_sha256":"{me}","version":"0.1.0"}}"#
        );
        assert_eq!(
            identified(&format!("{loaded}\nrest\n")).unwrap().0,
            Identity {
                version: "0.1.0".to_owned(),
                source_sha256: me.to_owned(),
                ngspice: Some("44.2".to_owned()),
                ngspice_library_sha256: Some(want.clone()),
            }
        );

        // A run that answered must have named its ngspice; nothing stands in.
        let bare = identified(&format!("{ok}\n")).unwrap().0;
        assert_eq!(bare.ran().unwrap_err().kind, "ToolError");
        let refused = [
            // Another release, its banner agreeing or not.
            loaded.replace("44.2", "45.0"),
            loaded.replace(r#""ngspice":"44.2""#, r#""ngspice":"45.0""#),
            loaded.replace("-44.2 shared", "-45.0 shared"),
            loaded.replace(" shared library", " shared library (debug)"),
            // A release without its banner, or a banner without its release.
            loaded.replace(r#""ngspice":"44.2","#, ""),
            loaded.replace(r#""ngspice_banner":"** ngspice-44.2 shared library","#, ""),
            // `AET-WO-2310`: a library named without its file, or a file that is
            // not an absolute, canonical path to a regular file.
            loaded.replace(&format!(r#""ngspice_library":"{library}","#), ""),
            loaded.replace(library, "libngspice.so.0"),
            loaded.replace(library, "/nonexistent/libngspice.so.0"),
            loaded.replace(library, "/tmp"),
            // The same regular file, by a path that is not canonical (`..`:
            // `Path` equality ignores a `.` component, never a `..`).
            loaded.replace(
                library,
                &format!(
                    "{}/../{}/{}",
                    file.parent().unwrap().display(),
                    file.parent()
                        .unwrap()
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap(),
                    file.file_name().unwrap().to_str().unwrap()
                ),
            ),
            String::new(),
            "rest".to_owned(),
            ok.replace(me, &"0".repeat(64)),
            ok.replace("aethelgard-bridge", "aethelgard-bridgf"),
            ok.replace(r#","version":"0.1.0""#, ""),
            ok.replace('}', r#","extra":1}"#),
            format!("rest\n{ok}"),
        ];
        for r in refused {
            let e = identified(&format!("{r}\n")).unwrap_err();
            assert_eq!(
                (e.kind, e.code),
                ("BridgeMismatch", crate::EXIT_TOOL_UNAVAILABLE),
                "{r}"
            );
        }
    }

    /// `AET-WO-2301`: a rail answer is crossings of requested levels at their
    /// requested values, in time order, then one closing line.
    #[test]
    fn a_rail_answer_is_requested_crossings_in_order_then_the_close() {
        let levels = power::levels(3);
        let c = |level: &str, uv: u64, at: u64, falling: bool| {
            format!(
                r#"{{"at_ps":{at},"falling":{falling},"level":"{level}","method":"interpolated","microvolts":{uv},"ngspice":"44.2"}}"#
            )
        };
        let end = r#"{"accepted_points":100027,"minimum_microvolts":2350000,"rail":"solved"}"#;
        let good = [
            c("VBOR3_rising", 2_700_000, 1_012_422_222, true),
            c("VBOR3_falling", 2_610_000, 1_014_422_222, true),
            end.to_owned(),
        ]
        .join("\n");
        let (crossings, min) = rail_answer(&good, &levels, "44.2").unwrap();
        // Every crossing names the ngspice the identity named, not a constant.
        assert!(rail_answer(&good, &levels, "44.3").is_err());
        assert_eq!((crossings.len(), min), (2, 2_350_000));
        let refused = [
            String::new(),
            c("VBOR3_falling", 2_610_000, 1, true),
            [c("VBOR2_falling", 2_310_000, 1, true), end.to_owned()].join("\n"),
            [c("VBOR3_falling", 2_600_000, 1, true), end.to_owned()].join("\n"),
            [
                c("VBOR3_falling", 2_610_000, 2, true),
                c("VBOR3_rising", 2_700_000, 1, true),
                end.to_owned(),
            ]
            .join("\n"),
            [
                c("VBOR3_falling", 2_610_000, 1, true).replace("44.2", "45"),
                end.to_owned(),
            ]
            .join("\n"),
            [
                c("VBOR3_falling", 2_610_000, 1, true).replace('}', r#","x":1}"#),
                end.to_owned(),
            ]
            .join("\n"),
            [end.to_owned(), end.to_owned()].join("\n"),
            end.replace("solved", "done"),
            // A level crossed three times is a dithering rail (adversarial F4).
            [
                c("VBOR3_falling", 2_610_000, 1, true),
                c("VBOR3_falling", 2_610_000, 2, false),
                c("VBOR3_falling", 2_610_000, 3, true),
                end.to_owned(),
            ]
            .join("\n"),
        ];
        for r in refused {
            assert!(rail_answer(&r, &levels, "44.2").is_err(), "accepted: {r}");
        }
    }

    fn decks() -> Vec<Deck> {
        let topology: SolvedTopology = serde_json::from_str(
            r#"{"schema_version":17,"specification_sha256":"acd60081f2d111b1b1ed0d974078fff45ccf2a967ec5f42317417f8ce68149d1","assignments":{"I2C_SCL":{"pin":"PB6","peripheral":"I2C1","signal":"I2C1_SCL","alternate_function":4},"I2C_SDA":{"pin":"PB7","peripheral":"I2C1","signal":"I2C1_SDA","alternate_function":4}},"gpio":{},"adc_inputs":{},"i2c_ceilings":{"SENSORS":400000},"i2c_speeds":{"SENSORS":400000},"analog_lines":{"I2C_SCL":{"bus":"SENSORS","pull_up_ohms":47000,"capacitance_femtofarads":100000,"ceiling_ns":300,"domain_millivolts":3300},"I2C_SDA":{"bus":"SENSORS","pull_up_ohms":47000,"capacitance_femtofarads":100000,"ceiling_ns":300,"domain_millivolts":3300}},"i2c_devices":{"U2":{"index":0,"part":"BME280","bus":"SENSORS","peripheral":"I2C1","address":118},"U3":{"index":1,"part":"TMP117","bus":"SENSORS","peripheral":"I2C1","address":72}},"spi_devices":{},"spi_lines":{},"clock":{"source":"HSI","sysclk_hz":64000000,"hse_hz":0,"d1cpre":1,"hpre":1,"d1ppre":1,"d2ppre1":1,"d2ppre2":1,"d3ppre":1,"timpre":false,"vos":"scale2"},"motor_outputs":{},"power":null,"serial_ports":{},"spi_controllers":{},"i2c_timings":{"I2C1":{"kernel_hz":64000000,"cr1":0,"timingr":{"Fast400k":281481788,"FastPlus1M":11669037,"Standard100k":1356016192}}},"bor_level":0,"core_supply":"ldo","mcu":"STM32H753ZITx","pin_speeds":{"I2C_SCL":0,"I2C_SDA":0},"foreign_clock":null}"#,
        )
        .unwrap();
        analog::decks(&topology).unwrap()
    }

    fn speeds() -> BTreeMap<String, u32> {
        BTreeMap::from([("SENSORS".to_owned(), 100_000)])
    }

    fn line(net: &str, rise: &str) -> String {
        format!(
            r#"{{"method":"interpolated","net":"{net}","ngspice":"44.2","rise_30_70_ns":{rise}}}"#
        )
    }

    #[test]
    fn every_answer_but_one_documented_line_per_net_in_order_is_refused() {
        let d = decks();
        let good = format!("{}\n{}\n", line("I2C_SCL", "1.5"), line("I2C_SDA", "2.5"));
        assert_eq!(
            answers(&d, &good, "44.2").unwrap(),
            [Answer::Rise(1.5), Answer::Rise(2.5)]
        );
        // Each answer names the ngspice the identity named, not a constant.
        assert!(answers(&d, &good, "44.3").is_err());
        let other = format!(
            "{}\n{}\n",
            r#"{"complete":false,"method":"interpolated","net":"I2C_SCL","ngspice":"44.2","rise_30_70_ns_lower_bound":13298.0}"#,
            r#"{"method":"interpolated","net":"I2C_SDA","ngspice":"44.2","volts_at_release":1.65}"#
        );
        assert_eq!(
            answers(&d, &other, "44.2").unwrap(),
            [Answer::Incomplete(13298.0), Answer::StuckHigh(1.65)]
        );
        let incomplete = |extra: &str| {
            format!(
                r#"{{"complete":false,"method":"interpolated","net":"I2C_SCL","ngspice":"44.2"{extra}}}{}{}"#,
                "\n",
                line("I2C_SDA", "2.5")
            )
        };
        let refused = [
            // An incomplete line that says it completed, gives no bound, or a rise too.
            incomplete(r#","rise_30_70_ns_lower_bound":1.0"#).replace("false", "true"),
            incomplete(""),
            incomplete(r#","rise_30_70_ns_lower_bound":0"#),
            incomplete(r#","rise_30_70_ns":1.0"#),
            incomplete(r#","rise_30_70_ns_lower_bound":1.0,"rise_30_70_ns":1.0"#),
            // A stuck line with no voltage, or with a rise beside it.
            line("I2C_SCL", "1.5").replace("rise_30_70_ns\":1.5", "volts_at_release\":null")
                + "\n"
                + &line("I2C_SDA", "2.5"),
            line("I2C_SCL", "1.5").replace('}', r#","volts_at_release":1.0}"#)
                + "\n"
                + &line("I2C_SDA", "2.5"),
            String::new(),
            line("I2C_SCL", "1.5"),
            format!("{}\n{}\n", line("I2C_SDA", "2.5"), line("I2C_SCL", "1.5")),
            format!("{}\n{}\n", line("I2C_SCL", "1.5"), line("I2C_SCL", "1.5")),
            format!(
                "{}\n{}\n{}\n",
                line("I2C_SCL", "1.5"),
                line("I2C_SDA", "2.5"),
                line("I2C_SDA", "2.5")
            ),
            format!("{}\n{}\n", line("I2C_SCL", "0"), line("I2C_SDA", "2.5")),
            format!("{}\n{}\n", line("I2C_SCL", "-1"), line("I2C_SDA", "2.5")),
            format!("{}\n{}\n", line("I2C_SCL", "null"), line("I2C_SDA", "2.5")),
            format!("{}\n{}\n", line("I2C_SCL", "\"1\""), line("I2C_SDA", "2.5")),
            format!("{}\n{}\n", line("I2C_SCL", "1e400"), line("I2C_SDA", "2.5")),
            format!(
                "{}\n{}\n",
                line("I2C_SCL", "1.5").replace("44.2", "45"),
                line("I2C_SDA", "2.5")
            ),
            format!(
                "{}\n{}\n",
                line("I2C_SCL", "1.5").replace("interpolated", "meas"),
                line("I2C_SDA", "2.5")
            ),
            format!(
                "{}\n{}\n",
                line("I2C_SCL", "1.5").replace('}', r#","exceeds":false}"#),
                line("I2C_SDA", "2.5")
            ),
            format!(
                r#"{{"net":"I2C_SCL","error":"methods_disagree","message":"m"}}{}{}"#,
                "\n",
                line("I2C_SDA", "2.5")
            ),
        ];
        for (i, out) in refused.iter().enumerate() {
            assert!(
                answers(&d, out, "44.2").is_err(),
                "case {i} was accepted: {out}"
            );
        }
    }

    #[test]
    fn a_line_past_its_ceiling_faults_its_bus_once() {
        let d = decks();
        let rises = |a: f64, b: f64| [Answer::Rise(a), Answer::Rise(b)];
        let pass = judge(&d, &rises(3982.3, 186.4), &speeds(), "44.2");
        assert_eq!(pass.analog.len(), 2);
        // The deck's 400 kHz window, beside the bus's declared 100 kHz (`AET-WO-2205` §1.3).
        for r in &pass.analog {
            assert_eq!((r.deck_hz, r.declared_hz), (400_000, 100_000));
        }
        assert!(pass.analog[0].exceeds && !pass.analog[1].exceeds);
        assert_eq!(pass.faulted(), BTreeSet::from(["SENSORS".to_owned()]));
        assert_eq!(pass.faults[0].nets, ["I2C_SCL"]);
        assert_eq!(pass.findings.len(), 1);
        let pass = judge(&d, &rises(3982.3, 3982.3), &speeds(), "44.2");
        assert_eq!((pass.faults.len(), pass.findings.len()), (1, 2));
        assert_eq!(pass.faults[0].nets, ["I2C_SCL", "I2C_SDA"]);
        // At the ceiling exactly is not past it.
        let pass = judge(&d, &rises(300.0, 186.4), &speeds(), "44.2");
        assert!(pass.faults.is_empty() && pass.findings.is_empty());
        assert_eq!(pass.analog[0].rise_30_70_ns, Some(300.0));
    }

    /// Ruled F7 (b) on PR #70: a rise unfinished in the window and a line
    /// never low are faults of the design, never a stop.
    #[test]
    fn an_unfinished_rise_and_a_line_stuck_high_fault_the_bus() {
        let d = decks();
        let pass = judge(
            &d,
            &[Answer::Incomplete(13298.0), Answer::Rise(186.4)],
            &speeds(),
            "44.2",
        );
        let r = &pass.analog[0];
        assert_eq!(
            (r.kind, r.complete, r.exceeds),
            ("analog.rise_time_computed", false, true)
        );
        assert_eq!(
            (r.rise_30_70_ns, r.rise_30_70_ns_lower_bound),
            (None, Some(13298.0))
        );
        assert_eq!(pass.faults[0].from, [PAST_CEILING]);
        assert_eq!(pass.findings[0].kind, "analog_rise_exceeds_ceiling");
        assert!(
            pass.findings[0]
                .message
                .contains("more than 13298 ns (a lower bound)")
        );

        let pass = judge(
            &d,
            &[Answer::StuckHigh(1.65), Answer::Rise(3982.3)],
            &speeds(),
            "45.0",
        );
        let r = &pass.analog[0];
        // `AET-WO-1606`: the record names the ngspice the bridge named.
        assert_eq!(r.ngspice, "45.0");
        assert_eq!(
            (r.kind, r.complete, r.exceeds),
            ("analog.line_stuck_high", false, false)
        );
        assert_eq!(r.volts_at_release, Some(1.65));
        assert_eq!(pass.faults.len(), 1);
        assert_eq!(pass.faults[0].nets, ["I2C_SCL", "I2C_SDA"]);
        assert_eq!(pass.faults[0].from, [STUCK_HIGH, PAST_CEILING]);
        let kinds: Vec<&str> = pass.findings.iter().map(|f| f.kind.as_str()).collect();
        assert_eq!(
            kinds,
            ["analog_line_stuck_high", "analog_rise_exceeds_ceiling"]
        );
    }
}
