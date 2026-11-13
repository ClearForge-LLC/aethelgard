//! `simulate --firmware`'s analog pre-pass (`AET-WO-2200` §1.4, `RUL-17`).
//!
//! Before the boot, each I2C line of the solved topology is measured:
//! 1. `aethelgard-twin::analog` writes one netlist per line;
//! 2. the bridge, a sandboxed tool beside this executable, measures each
//!    line's 30 %–70 % rise time with libngspice;
//! 3. each measurement is recorded as `analog.rise_time_measured`;
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

use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::Path;

use aethelgard_chaos::campaign;
use aethelgard_chaos::power::{self, Crossing};
use aethelgard_chaos::record::Droop;
use aethelgard_ir::SolvedTopology;
use aethelgard_twin::analog::{self, Deck};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::run;
use crate::verdict::{AnalogRecord, FaultRecord, Finding};

/// The bridge binary, found in this executable's own directory.
pub const BRIDGE: &str = "aethelgard-bridge";

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
}

impl Pass {
    /// The buses the boot's platform leaves devices out of, read from the
    /// recorded fault decision itself.
    pub fn faulted(&self) -> BTreeSet<String> {
        self.faults.iter().map(|f| f.bus.clone()).collect()
    }
}

/// The bridge's sources, in the order its build script hashes them
/// (`bridge/build.rs`): `Cargo.toml`, `Cargo.lock`, `build.rs`, then every
/// file in `src/` by name. A test pins this list to the directory.
pub const BRIDGE_SOURCES: [(&str, &[u8]); 11] = [
    ("Cargo.toml", include_bytes!("../../../bridge/Cargo.toml")),
    ("Cargo.lock", include_bytes!("../../../bridge/Cargo.lock")),
    ("build.rs", include_bytes!("../../../bridge/build.rs")),
    ("src/ffi.rs", include_bytes!("../../../bridge/src/ffi.rs")),
    (
        "src/header.rs",
        include_bytes!("../../../bridge/src/header.rs"),
    ),
    ("src/host.rs", include_bytes!("../../../bridge/src/host.rs")),
    ("src/main.rs", include_bytes!("../../../bridge/src/main.rs")),
    (
        "src/measure.rs",
        include_bytes!("../../../bridge/src/measure.rs"),
    ),
    ("src/rail.rs", include_bytes!("../../../bridge/src/rail.rs")),
    (
        "src/request.rs",
        include_bytes!("../../../bridge/src/request.rs"),
    ),
    (
        "src/solver.rs",
        include_bytes!("../../../bridge/src/solver.rs"),
    ),
];

/// The digest the bridge built from [`BRIDGE_SOURCES`] names itself by: for
/// each file, its path, a NUL, its length as 8 bytes little-endian, its bytes.
pub fn bridge_source_sha256() -> String {
    let mut h = Sha256::new();
    for (path, bytes) in BRIDGE_SOURCES {
        h.update(path.as_bytes());
        h.update([0u8]);
        h.update((bytes.len() as u64).to_le_bytes());
        h.update(bytes);
    }
    crate::input::hex(<[u8; 32]>::from(h.finalize()))
}

/// The bridge's first line (`AET-WO-2200` F15): its name, version and the
/// digest of the sources it was built from, which must be the ones this CLI
/// was built with. A stale bridge, one from another tree, or one that says
/// anything else first is refused (`BridgeMismatch`, exit 4) before any of
/// its answer is read; the rest of its stdout is returned.
pub fn identified(stdout: &str) -> Result<&str, run::Stop> {
    let (first, rest) = stdout.split_once('\n').unwrap_or((stdout, ""));
    let mismatch = |message: String| run::Stop {
        code: crate::EXIT_TOOL_UNAVAILABLE,
        kind: "BridgeMismatch",
        id: Some(BRIDGE.to_owned()),
        message,
    };
    let want = bridge_source_sha256();
    let v: Value = serde_json::from_str(first).map_err(|_| {
        mismatch(format!(
            "its first line is not its identity: `{}`",
            first.escape_debug()
        ))
    })?;
    let o = v.as_object().filter(|o| o.len() == 3).ok_or_else(|| {
        mismatch(format!(
            "its identity is not three fields: `{}`",
            first.escape_debug()
        ))
    })?;
    let field = |k: &str| o.get(k).and_then(Value::as_str);
    if field("bridge") != Some(BRIDGE) || field("version").is_none() {
        return Err(mismatch(format!(
            "it does not name itself: `{}`",
            first.escape_debug()
        )));
    }
    if field("source_sha256") != Some(want.as_str()) {
        return Err(mismatch(format!(
            "it was built from sources {}, not the {want} this CLI was built with: rebuild it",
            field("source_sha256").unwrap_or("(none)")
        )));
    }
    Ok(rest)
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
    let mut pass = measure(&decks, dir, status)?;
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
fn measure(decks: &[Deck], dir: &Path, status: &Path) -> Result<Pass, run::Stop> {
    if decks.is_empty() {
        return Ok(judge(decks, &[]));
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
    let stdout = identified(&stdout)?;
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
    let measured = answers(decks, stdout).map_err(|e| run::Stop::tool_error(BRIDGE, e))?;
    let mut pass = judge(decks, &measured);
    pass.sandbox = Some(out.sandbox);
    Ok(pass)
}

/// A declared droop, with what the runner resolved for it: the domain's
/// nominal and tolerance, and the emitter's decoupling on it (ruled D7).
#[derive(Clone, Debug)]
pub struct Rail {
    pub droop: Droop,
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
    let d = &rail.droop;
    let levels = power::levels(d.bor_level);
    let request = dir.join("rail.json");
    let body = json!({
        "domain": d.domain,
        "nominal_microvolts": u64::from(rail.nominal_millivolts) * 1000,
        "floor_microvolts": u64::from(d.floor_millivolts) * 1000,
        "start_ns": d.start_ns,
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
    let stdout = identified(&stdout)?;
    let fail = |m: String| run::Stop::tool_error(BRIDGE, m);
    if code != 0 {
        return Err(fail(format!(
            "exited {code} solving the rail: {} {}",
            stdout.trim(),
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    let (crossings, minimum_microvolts) = rail_answer(stdout, &levels).map_err(fail)?;
    Ok(Solved {
        crossings,
        minimum_microvolts,
        sandbox: out.sandbox,
    })
}

/// The bridge's rail answer: crossing lines in time order, each of a
/// requested level at its requested value, then one closing line.
fn rail_answer(stdout: &str, levels: &[(String, u64)]) -> Result<(Vec<Crossing>, u64), String> {
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
            && text("ngspice") == Some(NGSPICE)
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
/// through [`crate::renode::LOG_FILTER`].
pub fn boot(inv: &run::Invocation<'_>, renode: &Path) -> Result<run::Output, run::Stop> {
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
    .run_bounded_filtered(crate::renode::BOOT_BOUNDS, crate::renode::LOG_FILTER)?;
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let rest = identified(&text)?.as_bytes().to_vec();
    out.stdout = rest;
    Ok(out)
}

/// What a probe must read for the firmware to have recovered: the boot word,
/// the first identity check's status OK and expected value, and the first
/// witness's `CR2` and `TXDR` at the final block's masks (`AET-WO-2301`
/// adversarial F1: the record alone is the firmware's own claim). A reading
/// the image has no check or witness for has a mask of 0 and is vacuous.
fn recovered_words(
    image: &aethelgard_twin::boot::Image,
    boot_word: u32,
    probe: Option<&crate::renode::Probe>,
) -> (
    [crate::verdict::Word; 3],
    [crate::verdict::RegisterReport; 2],
) {
    use crate::verdict::{RegisterReport, Word, word};
    let check = image.i2c.checks.first();
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
            check.map_or(0, |c| c.expected),
            probe.map(|p| p.record.map_or(0, |r| r.1)),
        ),
    ];
    let witness = image.i2c.witnesses.first();
    let reg = |name: &str, address: u32, mask: u32, expected: u32, observed: Option<u32>| {
        RegisterReport {
            name: name.to_owned(),
            address: word(address),
            mask: word(mask),
            expected: word(expected),
            observed: observed.map(word),
        }
    };
    let controller = probe.map(|p| p.controller.unwrap_or((0, 0)));
    let regs = match witness {
        Some(wi) => [
            reg(
                &format!("{}.CR2", wi.peripheral),
                wi.cr2_at,
                wi.cr2_mask(),
                wi.cr2_expected(),
                controller.map(|c| c.0),
            ),
            reg(
                &format!("{}.TXDR", wi.peripheral),
                wi.txdr_at,
                0xFF,
                wi.txdr_expected(),
                controller.map(|c| c.1),
            ),
        ],
        None => [
            reg("CR2", 0, 0, 0, controller.map(|c| c.0)),
            reg("TXDR", 0, 0, 0, controller.map(|c| c.1)),
        ],
    };
    (words, regs)
}

/// Whether a word reads as expected under its mask.
fn as_expected(r: &crate::verdict::RegisterReport) -> bool {
    let n = |s: &str| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok();
    match (
        r.observed.as_deref().and_then(n),
        n(&r.expected),
        n(&r.mask),
    ) {
        (Some(o), Some(e), Some(m)) => o & m == e & m,
        _ => false,
    }
}

/// A droop's records and findings (`AET-WO-2301` §2.2, §2.3): the declared
/// rail, ngspice's crossings, the rendered resets, each release's probe;
/// `brownout_reset` (info) per reset, `brownout_unrecovered` per reset the
/// firmware did not recover from within the bound (or never released, or
/// whose probe a killed boot never read), and `rail_below_tolerance` (info)
/// when the rail left the domain's tolerance. `probes` is `None` when the
/// boot was killed at a bound.
pub fn droop_report(
    rail: &Rail,
    solved: &Solved,
    resets: &[power::Reset],
    probes: Option<&[crate::renode::Probe]>,
    image: &aethelgard_twin::boot::Image,
    boot_word: u32,
) -> (crate::verdict::DroopReport, Vec<Finding>) {
    use crate::verdict::{CrossingReport, DroopReport, ObservedReport, RailReport, ResetReport};
    let d = &rail.droop;
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
        let probe_at = at + power::RECOVERY_BOUND_NS;
        let Some(probe) = probes.map(|p| p.get(k)) else {
            findings.push(Finding::plain(
                campaign::UNRECOVERED_FINDING,
                format!("the boot was killed before the read at {probe_at} ns, after the release at {at} ns"),
            ));
            continue;
        };
        let ([boot, status, value], [cr2, txdr]) = recovered_words(image, boot_word, probe);
        let ok = [&boot, &status, &value]
            .iter()
            .all(|w| w.observed.as_deref() == Some(w.expected.as_str()))
            && as_expected(&cr2)
            && as_expected(&txdr);
        if !ok {
            findings.push(Finding::plain(
                campaign::UNRECOVERED_FINDING,
                format!(
                                        "{} ns after the release at {at} ns the firmware had not recovered: boot word {}, status {}, value {}, CR2 {}, TXDR {}",
                    power::RECOVERY_BOUND_NS,
                    boot.observed.as_deref().unwrap_or("unread"),
                    status.observed.as_deref().unwrap_or("unread"),
                    value.observed.as_deref().unwrap_or("unread"),
                    cr2.observed.as_deref().unwrap_or("unread"),
                    txdr.observed.as_deref().unwrap_or("unread")
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
            cr2,
            txdr,
        });
    }
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
        rail: RailReport {
            kind: "analog.rail_declared",
            source: "scenario",
            domain: d.domain.clone(),
            nominal_millivolts: rail.nominal_millivolts,
            tolerance_permille: rail.tolerance_permille,
            floor_millivolts: d.floor_millivolts,
            start_ns: d.start_ns,
            fall_ns: d.fall_ns,
            hold_ns: d.hold_ns,
            rise_ns: d.rise_ns,
            source_milliohms: d.source_milliohms,
            load_milliamps: d.load_milliamps,
            decoupling_picofarads: rail.decoupling_picofarads,
            bor_level: d.bor_level,
            minimum_microvolts: solved.minimum_microvolts,
            thresholds: "DS12117 Rev 7 Table 26, typical (provisional: read from a mirror, AET-WO-2301 F1)",
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
/// bridge's three documented lines; anything else is refused.
fn answers(decks: &[Deck], stdout: &str) -> Result<Vec<Answer>, String> {
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
            answer(deck, &v).ok_or_else(|| format!("`{line}` is not an answer for `{}`", deck.net))
        })
        .collect()
}

fn answer(deck: &Deck, v: &Value) -> Option<Answer> {
    let o = v.as_object()?;
    let ours = o.get("net").and_then(Value::as_str) == Some(deck.net.as_str())
        && o.get("method").and_then(Value::as_str) == Some("interpolated")
        && o.get("ngspice").and_then(Value::as_str) == Some(NGSPICE);
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
const PAST_CEILING: &str = "analog.rise_time_measured > ceiling_ns";
const STUCK_HIGH: &str = "analog.line_stuck_high";

fn judge(decks: &[Deck], answers: &[Answer]) -> Pass {
    let mut analog_records = Vec::new();
    let mut findings = Vec::new();
    let mut faulted: Vec<(String, Vec<String>, Vec<&'static str>)> = Vec::new();
    for (deck, &answer) in decks.iter().zip(answers) {
        let (bus, net, ceiling) = (&deck.bus, &deck.net, deck.ceiling_ns);
        let mut record = AnalogRecord {
            kind: "analog.rise_time_measured",
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
            ngspice: NGSPICE,
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `AET-WO-2200` F15: the files the CLI hashes are exactly the bridge's
    /// build inputs, so a bridge source added later cannot escape the check.
    #[test]
    fn the_bridge_sources_hashed_are_exactly_its_build_inputs() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bridge");
        let mut want = vec![
            "Cargo.toml".to_owned(),
            "Cargo.lock".to_owned(),
            "build.rs".to_owned(),
        ];
        let mut src: Vec<String> = fs::read_dir(root.join("src"))
            .unwrap()
            .map(|e| format!("src/{}", e.unwrap().file_name().to_str().unwrap()))
            .collect();
        src.sort();
        want.extend(src);
        let have: Vec<&str> = BRIDGE_SOURCES.iter().map(|(p, _)| *p).collect();
        assert_eq!(have, want);
        for (p, bytes) in BRIDGE_SOURCES {
            assert_eq!(
                fs::read(root.join(p)).unwrap(),
                bytes,
                "{p} changed since the build"
            );
        }
    }

    /// A stale or impostor bridge is refused before any of its answer is read.
    #[test]
    fn a_bridge_that_does_not_name_this_trees_sources_first_is_refused() {
        let me = bridge_source_sha256();
        let ok =
            format!(r#"{{"bridge":"aethelgard-bridge","source_sha256":"{me}","version":"0.1.0"}}"#);
        assert_eq!(identified(&format!("{ok}\nrest\n")).unwrap(), "rest\n");
        let refused = [
            String::new(),
            "rest".to_owned(),
            ok.replace(&me, &"0".repeat(64)),
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
        let (crossings, min) = rail_answer(&good, &levels).unwrap();
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
            assert!(rail_answer(&r, &levels).is_err(), "accepted: {r}");
        }
    }

    fn decks() -> Vec<Deck> {
        let topology: SolvedTopology = serde_json::from_str(
            r#"{"schema_version":2,"specification_sha256":"acd60081f2d111b1b1ed0d974078fff45ccf2a967ec5f42317417f8ce68149d1","assignments":{"I2C_SCL":{"pin":"PB6","peripheral":"I2C1","signal":"I2C1_SCL","alternate_function":4},"I2C_SDA":{"pin":"PB7","peripheral":"I2C1","signal":"I2C1_SDA","alternate_function":4}},"gpio":{},"i2c_ceilings":{"SENSORS":400000},"i2c_speeds":{"SENSORS":400000},"analog_lines":{"I2C_SCL":{"bus":"SENSORS","pull_up_ohms":47000,"capacitance_femtofarads":100000,"ceiling_ns":300,"domain_millivolts":3300},"I2C_SDA":{"bus":"SENSORS","pull_up_ohms":47000,"capacitance_femtofarads":100000,"ceiling_ns":300,"domain_millivolts":3300}},"i2c_devices":{"U2":{"index":0,"part":"BME280","bus":"SENSORS","peripheral":"I2C1","address":118},"U3":{"index":1,"part":"TMP117","bus":"SENSORS","peripheral":"I2C1","address":72}}}"#,
        )
        .unwrap();
        analog::decks(&topology).unwrap()
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
            answers(&d, &good).unwrap(),
            [Answer::Rise(1.5), Answer::Rise(2.5)]
        );
        let other = format!(
            "{}\n{}\n",
            r#"{"complete":false,"method":"interpolated","net":"I2C_SCL","ngspice":"44.2","rise_30_70_ns_lower_bound":13298.0}"#,
            r#"{"method":"interpolated","net":"I2C_SDA","ngspice":"44.2","volts_at_release":1.65}"#
        );
        assert_eq!(
            answers(&d, &other).unwrap(),
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
            assert!(answers(&d, out).is_err(), "case {i} was accepted: {out}");
        }
    }

    #[test]
    fn a_line_past_its_ceiling_faults_its_bus_once() {
        let d = decks();
        let rises = |a: f64, b: f64| [Answer::Rise(a), Answer::Rise(b)];
        let pass = judge(&d, &rises(3982.3, 186.4));
        assert_eq!(pass.analog.len(), 2);
        assert!(pass.analog[0].exceeds && !pass.analog[1].exceeds);
        assert_eq!(pass.faulted(), BTreeSet::from(["SENSORS".to_owned()]));
        assert_eq!(pass.faults[0].nets, ["I2C_SCL"]);
        assert_eq!(pass.findings.len(), 1);
        let pass = judge(&d, &rises(3982.3, 3982.3));
        assert_eq!((pass.faults.len(), pass.findings.len()), (1, 2));
        assert_eq!(pass.faults[0].nets, ["I2C_SCL", "I2C_SDA"]);
        // At the ceiling exactly is not past it.
        let pass = judge(&d, &rises(300.0, 186.4));
        assert!(pass.faults.is_empty() && pass.findings.is_empty());
        assert_eq!(pass.analog[0].rise_30_70_ns, Some(300.0));
    }

    /// Ruled F7 (b) on PR #70: a rise unfinished in the window and a line
    /// never low are faults of the design, never a stop.
    #[test]
    fn an_unfinished_rise_and_a_line_stuck_high_fault_the_bus() {
        let d = decks();
        let pass = judge(&d, &[Answer::Incomplete(13298.0), Answer::Rise(186.4)]);
        let r = &pass.analog[0];
        assert_eq!(
            (r.kind, r.complete, r.exceeds),
            ("analog.rise_time_measured", false, true)
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

        let pass = judge(&d, &[Answer::StuckHigh(1.65), Answer::Rise(3982.3)]);
        let r = &pass.analog[0];
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
