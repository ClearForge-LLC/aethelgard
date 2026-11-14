//! The fabrication package `verify --fab-out` writes and `--against-fab`
//! derives again (`AET-WO-3300` §1.2, §1.3, ruling on PR #103).
//!
//! It runs only after every gate of a `verify` has passed. The board is
//! copied from `verify`'s scratch into a scratch of its own, hashed and bound
//! to the generated board, and prepared exactly as `check drc` prepares it
//! (the specification's setup and rules, `RUL-03`). KiCad then exports, in
//! the sandbox:
//!
//! - the Gerbers of every layer the board uses, refilling every zone
//!   (`--check-zones`), so the copper plotted is the copper DRC judged;
//! - the drill file, Excellon with KiCad's minimal header;
//! - the placement file, CSV in millimetres.
//!
//! **The one change to KiCad's output.** KiCad 10.0.7 writes the wall clock
//! into lines 2 and 9 of every Gerber, and no option of its own stops it
//! (FEEDBACK §1.1). [`strip_clock`] deletes exactly those two lines and
//! nothing else, after checking each is where and what KiCad writes, and
//! that no other line carries a date. Any departure is a `ToolError`, exit 5,
//! and nothing is written.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use rustix::fs::OFlags;
use serde::Serialize;

use crate::kicad;
use crate::run::{self, MAX_CAPTURE, Stop};
use crate::tree;

/// The KiCad whose output this module is written against: its version is in
/// the two lines [`strip_clock`] deletes and in the line before them.
pub const KICAD_VERSION: &str = "10.0.7";

const TOOL: &str = "kicad-cli";

/// Each Gerber: the layer KiCad is asked for, the name KiCad gives its file
/// after the copy's stem (`input-`), and the line its creator line is on,
/// as measured. The package names the file after the project instead.
///
/// The creator line follows KiCad's header attributes, and the outline
/// carries one fewer (no `%TF.FilePolarity`), so its creator line is line 8
/// (`GERBER_plotter.cpp` `StartPlot`; FEEDBACK F3).
pub const LAYERS: [(&str, &str, usize); 9] = [
    ("F.Cu", "F_Cu.gtl", 9),
    ("B.Cu", "B_Cu.gbl", 9),
    ("F.Paste", "F_Paste.gtp", 9),
    ("B.Paste", "B_Paste.gbp", 9),
    ("F.SilkS", "F_Silkscreen.gto", 9),
    ("B.SilkS", "B_Silkscreen.gbo", 9),
    ("F.Mask", "F_Mask.gts", 9),
    ("B.Mask", "B_Mask.gbs", 9),
    ("Edge.Cuts", "Edge_Cuts.gm1", 8),
];

/// The package's manifest: written last, named in the verdict like every
/// other file.
pub const MANIFEST: &str = "MANIFEST.json";
pub const NOTES: &str = "FAB-NOTES.md";

/// The copy's stem, as `check drc` names it: KiCad names every output after it.
const STEM: &str = kicad::STEM;

/// What `verify` hands the package: everything it has already bound.
pub struct Bound<'a> {
    pub project: &'a str,
    pub spec: &'a aethelgard_ir::Specification,
    pub specification_sha256: &'a str,
    /// The generated board, in `verify`'s scratch, and its generated sha256.
    pub board: &'a Path,
    pub board_sha256: &'a str,
    /// The ELF `build` wrote, and the sha256 `build` reported, `simulate`
    /// booted and every campaign run was named by.
    pub elf: &'a Path,
    pub elf_sha256: &'a str,
    /// `emit`'s bill of materials, as generated.
    pub bom: &'a [u8],
}

/// What the export reports for the verdict's `fab` key: the tool it ran and
/// whether the uid boundary held, as every tool gate reports them.
pub struct Exported {
    /// Every file but the manifest, by its name in the package.
    pub files: BTreeMap<String, Vec<u8>>,
    pub tool_version: String,
    pub sandbox: run::SandboxReport,
}

/// Exports and assembles every file of the package but the manifest.
pub fn derive(b: &Bound<'_>) -> Result<Exported, Stop> {
    let mut files = BTreeMap::new();
    // The ELF first: read without following a link, and bound to the hash
    // the gates were bound to.
    let elf = read_output("aethelgard", b.elf)?;
    if tree::sha256(&elf) != b.elf_sha256 {
        return Err(Stop {
            code: crate::EXIT_TOOL_ERROR,
            kind: "BindingMismatch",
            id: Some("fab".to_owned()),
            message: format!(
                "the firmware read for the package has sha256 {}, not the {} build reported",
                tree::sha256(&elf),
                b.elf_sha256
            ),
        });
    }

    let scratch = crate::Scratch::new()?;
    let copy = scratch.tool.join(kicad::Check::Drc.copy_name());
    let input = crate::input::file(b.board, &copy)?;
    if input.sha256 != b.board_sha256 {
        return Err(Stop {
            code: crate::EXIT_TOOL_ERROR,
            kind: "BindingMismatch",
            id: Some("fab".to_owned()),
            message: format!(
                "the board copied for export has sha256 {}, not the generated {}",
                input.sha256, b.board_sha256
            ),
        });
    }
    let rules = aethelgard_eda::rules::Rules::from_specification(b.spec);
    let tree =
        kicad::prepare(kicad::Check::Drc, &copy, &scratch.tool, &rules)?.ok_or_else(|| {
            Stop::tool_error("aethelgard", "no strict reading of the board".to_owned())
        })?;
    let stack = aethelgard_eda::stack::Stack::of(b.spec);
    aethelgard_eda::route::declared(&tree, &stack).map_err(|d| {
        Stop::tool_error(
            "aethelgard",
            format!(
                "the board is not the declared one ({}): {}",
                d.kind, d.message
            ),
        )
    })?;
    let (_, routed) = aethelgard_eda::route::read(&tree, &stack)
        .map_err(|e| Stop::tool_error("aethelgard", format!("the board's copper: {e}")))?;
    let layers = gerbers(&stack);
    no_drilled_pad(&tree)?;

    let tool = run::Tool::find(TOOL)?;
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
    // Reported, not compared: `strip_clock` refuses a Gerber from any other
    // version, which KiCad writes into line 1 and the creator line.
    let version = crate::version(invocation(run::args(["version"]), "status-version"))?;
    let mut out_dir = scratch.tool.clone().into_os_string();
    out_dir.push("/");
    let names: Vec<&str> = layers.iter().map(|(l, ..)| l.as_str()).collect();
    let pos = scratch.tool.join(format!("{STEM}-pos.csv"));
    let exports = [
        (
            "status-gerbers",
            run::args([
                "pcb",
                "export",
                "gerbers",
                "--layers",
                &names.join(","),
                "--check-zones",
            ]),
            &out_dir,
        ),
        (
            "status-drill",
            run::args(["pcb", "export", "drill", "--excellon-min-header"]),
            &out_dir,
        ),
        (
            "status-pos",
            run::args([
                "pcb", "export", "pos", "--format", "csv", "--units", "mm", "--side", "both",
            ]),
            &pos.clone().into_os_string(),
        ),
    ];
    let mut sandbox = None;
    for (status, mut args, out) in exports {
        args.push("-o".into());
        args.push(out.clone());
        args.push(copy.clone().into_os_string());
        let ran = invocation(args, status).run()?;
        let code = ran.code(TOOL)?;
        if code != 0 {
            return Err(Stop::tool_error(
                TOOL,
                format!("`{TOOL}` exited {code} exporting ({status})"),
            ));
        }
        // The weakest of the three runs is the one the verdict reports.
        sandbox = Some(match sandbox {
            None => ran.sandbox,
            Some(s) => weaker(s, ran.sandbox),
        });
    }

    let mut copper = BTreeMap::new();
    for (layer, suffix, creator) in &layers {
        let (layer, creator) = (layer.as_str(), *creator);
        let raw = read_output(TOOL, &scratch.tool.join(format!("{STEM}-{suffix}")))?;
        let stripped = strip_clock(&raw, creator).map_err(|m| {
            Stop::tool_error(TOOL, format!("the {layer} Gerber is not as measured: {m}"))
        })?;
        if layer.ends_with(".Cu") {
            copper.insert(layer, stripped.clone());
        }
        files.insert(format!("{}-{suffix}", b.project), stripped);
    }
    for (net, layer) in &routed.zones {
        let plotted = copper
            .get(layer.as_str())
            .map(Vec::as_slice)
            .unwrap_or_default();
        if !pours(plotted, net) {
            return Err(Stop::tool_error(
                TOOL,
                format!(
                    "the board declares a {net} zone on {}, and its Gerber carries no region of {net}: the copper plotted is not the copper DRC judged",
                    layer
                ),
            ));
        }
    }
    let drill = read_output(TOOL, &scratch.tool.join(format!("{STEM}.drl")))?;
    holes(&drill, routed.vias.len())
        .map_err(|m| Stop::tool_error(TOOL, format!("the drill file: {m}")))?;
    files.insert(format!("{}.drl", b.project), drill);
    let placement = read_output(TOOL, &pos)?;
    placement_rows(&placement)
        .map_err(|m| Stop::tool_error(TOOL, format!("the placement file: {m}")))?;
    files.insert(format!("{}-pos.csv", b.project), placement);
    files.insert(format!("{}.bom.csv", b.project), b.bom.to_vec());
    files.insert(format!("{}.elf", b.project), elf);

    let described = describe(b.project, &layers);
    let notes = aethelgard_eda::fab::notes(&aethelgard_eda::fab::Notes {
        project: b.project,
        engine_version: env!("CARGO_PKG_VERSION"),
        specification_sha256: b.specification_sha256,
        kicad_version: KICAD_VERSION,
        files: &described,
        stack: &stack,
    });
    files.insert(NOTES.to_owned(), notes.into_bytes());
    Ok(Exported {
        files,
        tool_version: version,
        sandbox: sandbox.expect("three exports ran"),
    })
}

/// The Gerbers of the declared board (`AET-WO-4100`): today's nine, and each
/// inner copper layer after `B.Cu`, named as KiCad 10.0.7 names it
/// (`In1_Cu.g1`, `In2_Cu.g2`), its creator line on line 9 as on every copper
/// layer (measured, S1). The list follows the specification, never the
/// board, so a package cannot lose a declared layer.
pub fn gerbers(stack: &aethelgard_eda::stack::Stack) -> Vec<(String, String, usize)> {
    let mut out = Vec::new();
    for (layer, suffix, creator) in LAYERS {
        out.push((layer.to_owned(), suffix.to_owned(), creator));
        if layer == "B.Cu" {
            for (i, inner) in stack.copper[1..stack.copper.len() - 1].iter().enumerate() {
                out.push((
                    inner.clone(),
                    format!("{}.g{}", inner.replace('.', "_"), i + 1),
                    9,
                ));
            }
        }
    }
    out
}

/// Every file of the package but the notes and the manifest, with what it is.
fn describe(project: &str, layers: &[(String, String, usize)]) -> Vec<(String, &'static str)> {
    let mut out: Vec<(String, &'static str)> = layers
        .iter()
        .map(|(layer, suffix, _)| {
            let what = match layer.as_str() {
                "F.Cu" => "Gerber, top copper",
                "B.Cu" => "Gerber, bottom copper",
                "In1.Cu" => "Gerber, inner copper layer 1, a plane (see Stackup)",
                "In2.Cu" => "Gerber, inner copper layer 2, a plane (see Stackup)",
                "F.Paste" => "Gerber, top solder paste",
                "B.Paste" => "Gerber, bottom solder paste",
                "F.SilkS" => "Gerber, top silkscreen",
                "B.SilkS" => "Gerber, bottom silkscreen",
                "F.Mask" => "Gerber, top solder mask",
                "B.Mask" => "Gerber, bottom solder mask",
                _ => "Gerber, board outline",
            };
            (format!("{project}-{suffix}"), what)
        })
        .collect();
    out.extend([
        (format!("{project}.drl"), "Excellon drill file"),
        (
            format!("{project}-pos.csv"),
            "placement: reference, value, package, X and Y in mm, rotation, side",
        ),
        (
            format!("{project}.bom.csv"),
            "bill of materials, as `aethelgard emit` writes it",
        ),
        (
            format!("{project}.elf"),
            "firmware: the ELF `build` produced, whose sha256 is the verdict's `elf_sha256`",
        ),
    ]);
    out.sort();
    out
}

/// The sandbox report naming the weaker of two runs: a boundary holds for
/// the package only if it held for every export.
fn weaker(a: run::SandboxReport, b: run::SandboxReport) -> run::SandboxReport {
    if a.uid_boundary && !b.uid_boundary {
        b
    } else {
        a
    }
}

/// A file a tool wrote: a regular file, not followed through a link, at most
/// [`MAX_CAPTURE`] bytes.
fn read_output(tool: &str, path: &Path) -> Result<Vec<u8>, Stop> {
    let name = path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let missing = |e: std::io::Error| Stop::tool_error(tool, format!("no {name}: {e}"));
    let file = OpenOptions::new()
        .read(true)
        .custom_flags((OFlags::NOFOLLOW | OFlags::NONBLOCK).bits() as i32)
        .open(path)
        .map_err(missing)?;
    if !file.metadata().map_err(missing)?.is_file() {
        return Err(Stop::tool_error(
            tool,
            format!("{name} is not a regular file"),
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAX_CAPTURE as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(missing)?;
    if bytes.len() > MAX_CAPTURE {
        return Err(Stop::tool_error(
            tool,
            format!("{name} exceeds {MAX_CAPTURE} bytes"),
        ));
    }
    Ok(bytes)
}

/// `s` has exactly the shape `pattern` gives: `d` is an ASCII digit, `s` a
/// sign (`+` or `-`), and any other character itself.
fn shaped(s: &str, pattern: &str) -> bool {
    s.len() == pattern.len()
        && s.bytes().zip(pattern.bytes()).all(|(c, p)| match p {
            b'd' => c.is_ascii_digit(),
            b's' => c == b'+' || c == b'-',
            _ => c == p,
        })
}

/// Whether `line` carries anything that reads as a date or a time of day:
/// `dddd-dd-dd` or `dd:dd:dd` anywhere in it.
fn dated(line: &str) -> bool {
    let b = line.as_bytes();
    (0..b.len()).any(|i| {
        let at = |pat: &str| b.len() >= i + pat.len() && shaped(&line[i..i + pat.len()], pat);
        at("dddd-dd-dd") || at("dd:dd:dd")
    })
}

/// KiCad's two clock lines, deleted (ruling on PR #103, conditions 1 and 2).
///
/// The file must be ASCII with `\n` line ends. Line 1 must be
/// `%TF.GenerationSoftware,KiCad,Pcbnew,10.0.7*%`; line 2
/// `%TF.CreationDate,<YYYY-MM-DDThh:mm:ss±hh:mm>*%`; line `creator` (9, or
/// 8 for the outline, [`LAYERS`])
/// `G04 Created by KiCad (PCBNEW 10.0.7) date <YYYY-MM-DD hh:mm:ss>*`. No
/// other line may name a creation date or KiCad's creator line, or carry
/// anything that reads as a date or a time of day. The two lines are
/// deleted, never rewritten; every other byte is kept.
pub fn strip_clock(raw: &[u8], creator: usize) -> Result<Vec<u8>, String> {
    let text = std::str::from_utf8(raw)
        .ok()
        .filter(|t| t.is_ascii())
        .ok_or("it is not ASCII")?;
    if text.contains('\r') {
        return Err("it has a carriage return".to_owned());
    }
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let at = creator
        .checked_sub(1)
        .filter(|i| *i > 1)
        .ok_or("no creator line")?;
    if lines.len() <= at + 1 || lines.iter().any(|l| !l.ends_with('\n')) {
        return Err("its header is not complete lines".to_owned());
    }
    let body = |i: usize| lines[i].strip_suffix('\n').unwrap_or(lines[i]);
    let software = format!("%TF.GenerationSoftware,KiCad,Pcbnew,{KICAD_VERSION}*%");
    if body(0) != software {
        return Err(format!("line 1 is not {software}"));
    }
    let created = body(1)
        .strip_prefix("%TF.CreationDate,")
        .and_then(|r| r.strip_suffix("*%"));
    if !created.is_some_and(|d| shaped(d, "dddd-dd-ddTdd:dd:ddsdd:dd")) {
        return Err("line 2 is not %TF.CreationDate,<ISO 8601 with offset>*%".to_owned());
    }
    let prefix = format!("G04 Created by KiCad (PCBNEW {KICAD_VERSION}) date ");
    let date = body(at)
        .strip_prefix(prefix.as_str())
        .and_then(|r| r.strip_suffix('*'));
    if !date.is_some_and(|d| shaped(d, "dddd-dd-dd dd:dd:dd")) {
        return Err(format!(
            "line {creator} is not {prefix}<YYYY-MM-DD hh:mm:ss>*"
        ));
    }
    let mut out = Vec::with_capacity(raw.len());
    for (i, line) in lines.iter().enumerate() {
        if i == 1 || i == at {
            continue;
        }
        if line.contains("CreationDate") || line.contains("Created by") || dated(line) {
            return Err(format!(
                "line {} carries a date or a time of day (a name from the specification can put one there)",
                i + 1
            ));
        }
        out.extend_from_slice(line.as_bytes());
    }
    Ok(out)
}

/// Whether a copper Gerber carries a region (`G36`…`G37`) of `net`: KiCad
/// writes a filled zone as `%TO.N,<net>*%` then `G36*`, its outline, and
/// `G37*` (ruling condition 8).
pub fn pours(gerber: &[u8], net: &str) -> bool {
    let text = String::from_utf8_lossy(gerber);
    let lines: Vec<&str> = text.lines().collect();
    let attribute = format!("%TO.N,{net}*%");
    lines
        .windows(2)
        .enumerate()
        .any(|(i, w)| w[0] == attribute && w[1] == "G36*" && lines[i + 2..].contains(&"G37*"))
}

/// The notes say every hole is a plated via of the router's drill
/// (condition 6): no pad of the board carries a drill.
fn no_drilled_pad(tree: &aethelgard_eda::strict::Tree) -> Result<(), Stop> {
    for fp in tree.root().all("footprint") {
        for pad in fp.all("pad") {
            // A drill, or two (which `one` refuses), is a drilled pad.
            if !matches!(pad.one("drill"), Ok(None)) {
                return Err(Stop::tool_error(
                    "aethelgard",
                    "a pad of the board is drilled; the package states that every hole is a via"
                        .to_owned(),
                ));
            }
        }
    }
    Ok(())
}

/// The drill file has exactly one tool, the router's via drill in mm, and
/// one hit per via of the board.
pub fn holes(drill: &[u8], vias: usize) -> Result<(), String> {
    let text = std::str::from_utf8(drill).map_err(|_| "it is not UTF-8".to_owned())?;
    let drill_nm = aethelgard_eda::route::DRILL_NM;
    let tool = format!(
        "T1C{}.{:03}",
        drill_nm / 1_000_000,
        drill_nm % 1_000_000 / 1000
    );
    let header: Vec<&str> = text.lines().take_while(|l| *l != "%").collect();
    if !header.contains(&"METRIC") {
        return Err("it is not metric".to_owned());
    }
    let tools: Vec<&str> = header
        .iter()
        .copied()
        .filter(|l| l.starts_with('T'))
        .collect();
    if tools != [tool.as_str()] {
        return Err(format!("its tools are {tools:?}, not [{tool}]"));
    }
    let hits = text.lines().filter(|l| l.starts_with('X')).count();
    if hits != vias {
        return Err(format!("it has {hits} hits for the board's {vias} vias"));
    }
    Ok(())
}

/// KiCad writes each placement row's reference, value and package in quotes
/// without escaping a quote inside them, so a name holding `"` shifts every
/// later column (adversarial). Every row must be three quoted fields with no
/// quote inside, then four unquoted ones; commas inside quotes are CSV's own.
pub fn placement_rows(csv: &[u8]) -> Result<(), String> {
    let text = std::str::from_utf8(csv).map_err(|_| "it is not UTF-8".to_owned())?;
    let mut lines = text.lines();
    if lines.next() != Some("Ref,Val,Package,PosX,PosY,Rot,Side") {
        return Err("its header is not KiCad's CSV header".to_owned());
    }
    for (n, row) in lines.enumerate() {
        let bad = || {
            format!(
                "row {} is not three quoted fields and four plain ones",
                n + 1
            )
        };
        let mut rest = row;
        for _ in 0..3 {
            let inner = rest.strip_prefix('"').ok_or_else(bad)?;
            let end = inner.find('"').ok_or_else(bad)?;
            rest = inner[end + 1..].strip_prefix(',').ok_or_else(bad)?;
        }
        let plain: Vec<&str> = rest.split(',').collect();
        if plain.len() != 4 || plain.iter().any(|f| f.is_empty() || f.contains('"')) {
            return Err(bad());
        }
    }
    Ok(())
}

/// `MANIFEST.json`: every other file's sha256, the specification's and the
/// topology's, the scenarios the run was given, and the sha256 of the
/// verdict line the same `verify` call prints without `--fab-out` or
/// `--against-fab`. That verdict carries each gate's sandbox report, so a run
/// under another uid or user namespace prints another (adversarial A2).
#[derive(Serialize)]
struct Manifest<'a> {
    project: &'a str,
    specification_sha256: &'a str,
    topology_sha256: &'a str,
    /// Each `--scenarios` file, in the order given: its name and sha256.
    scenarios: &'a [Scenario],
    verdict_sha256: String,
    files: BTreeMap<&'a str, String>,
}

/// One scenario the emitting run was given.
#[derive(Serialize)]
pub struct Scenario {
    pub scenario: String,
    pub sha256: String,
}

/// What the manifest names beside the files.
pub struct Named<'a> {
    pub project: &'a str,
    pub specification_sha256: &'a str,
    pub topology_sha256: &'a str,
    pub scenarios: &'a [Scenario],
    /// The verdict line the same call prints without the fab flags.
    pub verdict_line: &'a str,
}

/// The manifest's bytes.
pub fn manifest(n: &Named<'_>, files: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let m = Manifest {
        project: n.project,
        specification_sha256: n.specification_sha256,
        topology_sha256: n.topology_sha256,
        scenarios: n.scenarios,
        verdict_sha256: tree::sha256(n.verdict_line.as_bytes()),
        files: files
            .iter()
            .map(|(name, bytes)| (name.as_str(), tree::sha256(bytes)))
            .collect(),
    };
    let mut out = serde_json::to_vec_pretty(&m).expect("a manifest serializes");
    out.push(b'\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The first 20 lines of two layers, as KiCad 10.0.7 wrote them in the
    /// §1.1 spike (run A-1). Only header lines are committed, never a whole
    /// layer. A KiCad bump that changes them fails here.
    const REAL: &str = include_str!("../tests/fixtures/fab/kicad-10.0.7-F_Cu-header.gtl");
    const OUTLINE: &str = include_str!("../tests/fixtures/fab/kicad-10.0.7-Edge_Cuts-header.gm1");

    #[test]
    fn a_real_header_loses_exactly_its_two_clock_lines() {
        for (text, creator) in [(REAL, 9), (OUTLINE, 8)] {
            let out = strip_clock(text.as_bytes(), creator).unwrap();
            let want: String = text
                .split_inclusive('\n')
                .enumerate()
                .filter(|(i, _)| *i != 1 && *i != creator - 1)
                .map(|(_, l)| l)
                .collect();
            assert_eq!(String::from_utf8(out).unwrap(), want);
            assert_eq!(want.lines().count() + 2, text.lines().count());
        }
        // Each layer's creator line is where it was measured, and nowhere else.
        assert!(strip_clock(OUTLINE.as_bytes(), 9).is_err());
        assert!(strip_clock(REAL.as_bytes(), 8).is_err());
        assert_eq!(
            LAYERS.iter().map(|l| l.2).collect::<Vec<_>>(),
            [9, 9, 9, 9, 9, 9, 9, 9, 8]
        );
    }

    fn with(edit: impl Fn(&mut Vec<String>)) -> Vec<u8> {
        let mut lines: Vec<String> = REAL.split_inclusive('\n').map(str::to_owned).collect();
        edit(&mut lines);
        lines.concat().into_bytes()
    }

    /// Ruling condition 2: every departure is refused.
    #[test]
    fn every_departure_from_the_measured_header_is_refused() {
        let date2 = REAL.lines().nth(1).unwrap().to_owned() + "\n";
        let date9 = REAL.lines().nth(8).unwrap().to_owned() + "\n";
        let cases: Vec<(&str, Vec<u8>)> = vec![
            ("line 2 missing", with(|l| drop(l.remove(1)))),
            ("line 9 missing", with(|l| drop(l.remove(8)))),
            ("line 2 moved", with(|l| l.swap(1, 2))),
            ("line 9 moved", with(|l| l.swap(8, 9))),
            ("a second line 2", with(|l| l.insert(12, date2.clone()))),
            ("a second line 9", with(|l| l.insert(12, date9.clone()))),
            (
                "a date in a comment",
                with(|l| l.insert(12, "G04 2026-11-13*\n".to_owned())),
            ),
            (
                "a time in a comment",
                with(|l| l.insert(12, "G04 at 03:32:32*\n".to_owned())),
            ),
            (
                "another version on line 9",
                with(|l| l[8] = l[8].replace("10.0.7", "10.0.8")),
            ),
            (
                "another version on line 1",
                with(|l| l[0] = l[0].replace("10.0.7", "10.0.8")),
            ),
            (
                "line 2 without its offset",
                with(|l| l[1] = l[1].replace("+00:00", "")),
            ),
            (
                "line 2 as a comment (--no-x2)",
                with(|l| {
                    l[1] = l[1]
                        .replace("%TF.CreationDate,", "G04 #@! TF.CreationDate,")
                        .replace("*%", "*")
                }),
            ),
            (
                "line 9 with text after its date",
                with(|l| l[8] = l[8].replace("32*", "32 X*")),
            ),
            (
                "a carriage return",
                with(|l| l[3] = l[3].replace('\n', "\r\n")),
            ),
            (
                "not ASCII",
                with(|l| l[3] = l[3].replace("Original", "Originál")),
            ),
            ("no final newline", REAL.trim_end().as_bytes().to_vec()),
            (
                "too short",
                REAL.lines()
                    .take(9)
                    .collect::<Vec<_>>()
                    .join("\n")
                    .into_bytes(),
            ),
        ];
        for (what, bytes) in cases {
            assert!(strip_clock(&bytes, 9).is_err(), "{what} was accepted");
        }
        assert!(strip_clock(REAL.as_bytes(), 0).is_err());
        assert!(strip_clock(REAL.as_bytes(), 2).is_err());
    }

    #[test]
    fn a_date_or_time_anywhere_in_a_line_is_found() {
        for line in ["2026-11-13", "x 1999-01-02 y", "03:32:32", "aT12:00:00Z"] {
            assert!(dated(line), "{line}");
        }
        for line in [
            "X112625000Y-97625000D01*",
            "%TF.ProjectId,input,696e7075-742e-46b6-9963-61645f706362,rev?*%",
            "%ADD10RoundRect,0.100000X-0.437500X-0.300000*%",
            "12:34",
        ] {
            assert!(!dated(line), "{line}");
        }
    }

    /// Condition 8: the pour is a region of the zone's net.
    #[test]
    fn a_zone_is_poured_only_as_a_region_of_its_net() {
        let poured = b"%TA.AperFunction,Conductor*%\n%TO.N,GND*%\nG36*\nX1Y1D02*\nG37*\n";
        assert!(pours(poured, "GND"));
        assert!(!pours(poured, "VDD_3V3"));
        assert!(!pours(b"%TO.N,GND*%\nX1Y1D03*\n", "GND"));
        assert!(!pours(b"%TO.N,GND*%\nX1Y1D03*\nG37*\n", "GND"));
        assert!(!pours(b"%TO.N,GND*%\nG36*\nX1Y1D02*\n", "GND"));
        assert!(!pours(b"G36*\n%TO.N,GND*%\nG37*\n", "GND"));
    }

    #[test]
    fn the_drill_file_must_be_every_via_and_nothing_else() {
        let drl = "M48\nMETRIC\nT1C0.300\n%\nG90\nG05\nT1\nX76.25Y-108.75\nX77.5Y-111.25\nM30\n";
        assert_eq!(holes(drl.as_bytes(), 2), Ok(()));
        assert!(holes(drl.as_bytes(), 3).is_err());
        assert!(holes(drl.replace("T1C0.300", "T1C0.400").as_bytes(), 2).is_err());
        assert!(
            holes(
                drl.replace("T1C0.300\n", "T1C0.300\nT2C1.000\n").as_bytes(),
                2
            )
            .is_err()
        );
        assert!(holes(drl.replace("METRIC", "INCH").as_bytes(), 2).is_err());
    }

    #[test]
    fn a_placement_row_with_a_quote_inside_a_field_is_refused() {
        let ok = "Ref,Val,Package,PosX,PosY,Rot,Side\n\"U4\",\"W25Q64JVSS\",\"SOIC-8\",82.55,-108.4,0.0,top\n";
        assert_eq!(placement_rows(ok.as_bytes()), Ok(()));
        let comma = ok.replace("W25Q64JVSS", "A,B");
        assert_eq!(placement_rows(comma.as_bytes()), Ok(()));
        for bad in [
            ok.replace("\"U4\"", "\"U\"4,X\""),
            ok.replace("\"U4\"", "U4"),
            ok.replace(",top", ",top,x"),
            ok.replace(",top", ""),
            ok.replace(",0.0,", ",\"0\","),
            ok.replace("Ref,", "Reference,"),
        ] {
            assert!(placement_rows(bad.as_bytes()).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_manifest_is_byte_stable_and_names_every_file() {
        let files = BTreeMap::from([
            ("b".to_owned(), b"2".to_vec()),
            ("a".to_owned(), b"1".to_vec()),
        ]);
        let scenarios = [Scenario {
            scenario: "s1".to_owned(),
            sha256: "ab".to_owned(),
        }];
        let named = Named {
            project: "p",
            specification_sha256: "s",
            topology_sha256: "t",
            scenarios: &scenarios,
            verdict_line: "{\"ok\":true}",
        };
        let m = manifest(&named, &files);
        assert_eq!(m, manifest(&named, &files));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&m).unwrap()["scenarios"],
            serde_json::json!([{"scenario": "s1", "sha256": "ab"}])
        );
        let v: serde_json::Value = serde_json::from_slice(&m).unwrap();
        assert_eq!(v["verdict_sha256"], tree::sha256(b"{\"ok\":true}"));
        assert_eq!(v["files"]["a"], tree::sha256(b"1"));
        assert_eq!(v["files"].as_object().unwrap().len(), 2);
        assert!(m.ends_with(b"}\n"));
    }

    /// Condition 6 rests on this: a drilled pad, or a pad with two drills,
    /// is refused; an SMD pad is not.
    #[test]
    fn a_drilled_pad_is_refused() {
        let board = |pad: &str| {
            aethelgard_eda::strict::board(&format!(
                "(kicad_pcb (version 20260206) (generator \"t\") (footprint \"t\" (layer \"F.Cu\") (at 0 0) {pad}))"
            ))
            .unwrap()
        };
        let smd = "(pad \"1\" smd rect (at 0 0) (size 1 1) (layers \"F.Cu\"))";
        assert!(no_drilled_pad(&board(smd)).is_ok());
        let drilled =
            "(pad \"1\" thru_hole circle (at 0 0) (size 1 1) (drill 0.5) (layers \"*.Cu\"))";
        assert!(no_drilled_pad(&board(drilled)).is_err());
        let npth =
            "(pad \"\" np_thru_hole circle (at 0 0) (size 1 1) (drill 1) (layers \"F.Mask\"))";
        assert!(no_drilled_pad(&board(npth)).is_err());
    }

    #[test]
    fn the_notes_describe_every_file_and_the_package_names_them_after_the_project() {
        let spec: aethelgard_ir::Specification =
            serde_json::from_str(&repo_file("examples/sentinel-01-minimal.json")).unwrap();
        let d = describe("p", &gerbers(&aethelgard_eda::stack::Stack::of(&spec)));
        assert_eq!(d.len(), LAYERS.len() + 4);
        assert!(
            d.iter()
                .all(|(n, _)| n.starts_with("p-") || n.starts_with("p."))
        );
    }

    /// A repository file, read when the test runs: the package carries no file
    /// outside the crate, so nothing here may be `include_str!`ed from one.
    fn repo_file(path: &str) -> String {
        std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(path),
        )
        .unwrap()
    }

    /// The Gerber list follows the declared layers (`AET-WO-4100`): at 2
    /// layers exactly today's nine; at 4 both inner layers after `B.Cu`,
    /// named as KiCad 10.0.7 names them, creator line 9 (measured, S1).
    #[test]
    fn the_gerbers_are_the_declared_layers() {
        let load = |text: &str| {
            let spec: aethelgard_ir::Specification = serde_json::from_str(text).unwrap();
            gerbers(&aethelgard_eda::stack::Stack::of(&spec))
        };
        let two = load(&repo_file("examples/sentinel-01-minimal.json"));
        let today: Vec<(String, String, usize)> = LAYERS
            .iter()
            .map(|(l, s, c)| ((*l).to_owned(), (*s).to_owned(), *c))
            .collect();
        assert_eq!(two, today);
        let four = load(&repo_file(
            "crates/aethelgard-cli/tests/fixtures/four-layer.json",
        ));
        let inner: Vec<&(String, String, usize)> =
            four.iter().filter(|(l, ..)| l.starts_with("In")).collect();
        assert_eq!(
            inner,
            [
                &("In1.Cu".to_owned(), "In1_Cu.g1".to_owned(), 9),
                &("In2.Cu".to_owned(), "In2_Cu.g2".to_owned(), 9)
            ]
        );
        assert_eq!(four.len(), LAYERS.len() + 2);
    }
}
