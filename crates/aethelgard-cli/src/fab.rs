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

// The KiCad whose output this module is written against: its version is in
// the two lines `strip_clock` deletes and in the line before them.
use crate::kicad::{self, KICAD_VERSION};
use crate::run::{self, MAX_CAPTURE, Stop};
use crate::tree;

const TOOL: &str = "kicad-cli";

/// Each Gerber: the layer KiCad is asked for, the name KiCad gives its file
/// after the copy's stem (`input-`), the line its creator line is on, as
/// measured, and what the notes call it. The package names the file after the project instead.
///
/// The creator line follows KiCad's header attributes, and the outline
/// carries one fewer (no `%TF.FilePolarity`), so its creator line is line 8
/// (`GERBER_plotter.cpp` `StartPlot`; FEEDBACK F3).
pub const LAYERS: [(&str, &str, usize, &str); 9] = [
    ("F.Cu", "F_Cu.gtl", 9, "Gerber, top copper"),
    ("B.Cu", "B_Cu.gbl", 9, "Gerber, bottom copper"),
    ("F.Paste", "F_Paste.gtp", 9, "Gerber, top solder paste"),
    ("B.Paste", "B_Paste.gbp", 9, "Gerber, bottom solder paste"),
    ("F.SilkS", "F_Silkscreen.gto", 9, "Gerber, top silkscreen"),
    (
        "B.SilkS",
        "B_Silkscreen.gbo",
        9,
        "Gerber, bottom silkscreen",
    ),
    ("F.Mask", "F_Mask.gts", 9, "Gerber, top solder mask"),
    ("B.Mask", "B_Mask.gbs", 9, "Gerber, bottom solder mask"),
    ("Edge.Cuts", "Edge_Cuts.gm1", 8, "Gerber, board outline"),
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
    /// The conditions the `extract` gate's verdict stated, as it stated them.
    pub conditions: &'a [String],
    /// The verdict's lifecycle lines: its banner, then each part unheld.
    pub lifecycle: &'a [String],
    /// The support gate's verdict, as lines for the notes (`AET-WO-4300g`).
    pub support: &'a [String],
    /// What the campaign's verdicts do not claim, each once (`AET-WO-2304b`
    /// §1.7, §1.11): the notes state them verbatim.
    pub campaign: &'a [String],
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
    let stack = aethelgard_eda::stack::Stack::of(b.spec).map_err(crate::undeclared)?;
    let rules = aethelgard_eda::rules::Rules::from_stack(&stack);
    let tree =
        kicad::prepare(kicad::Check::Drc, &copy, &scratch.tool, &rules)?.ok_or_else(|| {
            Stop::tool_error("aethelgard", "no strict reading of the board".to_owned())
        })?;
    aethelgard_eda::route::declared(&tree, &stack).map_err(|d| {
        Stop::tool_error(
            "aethelgard",
            format!(
                "the board is not the declared one ({}): {}",
                d.kind, d.message
            ),
        )
    })?;
    // `read` refuses every hole but a routed via, a declared array's via and
    // a plated round through-hole pad: an unplated hole would share KiCad's
    // one drill file with no plating mark, and a slot is a `G85` line
    // (measured, `AET-WO-4300d2` FEEDBACK).
    let (board, routed) = aethelgard_eda::route::read(&tree, &stack)
        .map_err(|e| Stop::tool_error("aethelgard", format!("the board's copper: {e}")))?;
    let drills = aethelgard_eda::fab::drills(&board, &routed)
        .map_err(|e| Stop::tool_error("aethelgard", format!("the board's holes: {e}")))?;
    let layers = gerbers(&stack);

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
    for (layer, suffix, creator, _) in &layers {
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
    aethelgard_eda::fab::holes(&drill, &drills)
        .map_err(|m| Stop::tool_error(TOOL, format!("the drill file: {m}")))?;
    files.insert(format!("{}.drl", b.project), drill);
    let placement = read_output(TOOL, &pos)?;
    // Reads the placement file row by row (`aethelgard_eda::fab::placement`),
    // refusing one KiCad's unescaped quotes would shift.
    let (jlcpcb_bom, jlcpcb_cpl) = jlcpcb(b.bom, &placement)?;
    files.insert(format!("{}-pos.csv", b.project), placement);
    files.insert(format!("{}.bom.csv", b.project), b.bom.to_vec());
    files.insert(format!("{}-jlcpcb-bom.csv", b.project), jlcpcb_bom);
    files.insert(format!("{}-jlcpcb-cpl.csv", b.project), jlcpcb_cpl);
    files.insert(format!("{}.elf", b.project), elf);

    let notes = notes(
        b.project,
        env!("CARGO_PKG_VERSION"),
        b.specification_sha256,
        b.spec,
        &stack,
        &drills,
        &Stated {
            conditions: b.conditions,
            lifecycle: b.lifecycle,
            support: b.support,
            campaign: b.campaign,
        },
    );
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
pub fn gerbers(stack: &aethelgard_eda::stack::Stack) -> Vec<(String, String, usize, String)> {
    let mut out = Vec::new();
    for (layer, suffix, creator, what) in LAYERS {
        out.push((
            layer.to_owned(),
            suffix.to_owned(),
            creator,
            what.to_owned(),
        ));
        if layer == "B.Cu" {
            for (i, inner) in stack.copper[1..stack.copper.len() - 1].iter().enumerate() {
                out.push((
                    inner.clone(),
                    format!("{}.g{}", inner.replace('.', "_"), i + 1),
                    9,
                    format!(
                        "Gerber, inner copper layer {}, a plane (see Stackup)",
                        i + 1
                    ),
                ));
            }
        }
    }
    out
}

/// What the verdict states that the notes repeat: `extract`'s conditions,
/// the lifecycle lines, and the support gate's lines.
struct Stated<'a> {
    conditions: &'a [String],
    lifecycle: &'a [String],
    support: &'a [String],
    campaign: &'a [String],
}

/// `FAB-NOTES.md` for the package of `spec`'s board, whose drill file holds
/// `drills`, with the `extract` verdict's `conditions` and `lifecycle` lines
/// as it stated them (`AET-WO-3303`).
fn notes(
    project: &str,
    engine: &str,
    specification_sha256: &str,
    spec: &aethelgard_ir::Specification,
    stack: &aethelgard_eda::stack::Stack,
    drills: &[aethelgard_eda::fab::Drill],
    stated: &Stated<'_>,
) -> String {
    let layers = gerbers(stack);
    let described = describe(project, &layers);
    let mut text = aethelgard_eda::fab::notes(&aethelgard_eda::fab::Notes {
        project,
        engine_version: engine,
        specification_sha256,
        kicad_version: KICAD_VERSION,
        files: &described,
        stack,
        conditions: stated.conditions,
        lifecycle: stated.lifecycle,
        drills,
        power: &aethelgard_eda::loops::conditions(spec),
        support: stated.support,
        acknowledged: &spec.acknowledge_unwitnessed,
    });
    text.push_str(&campaign_section(stated.campaign));
    text.push_str(&brown_out_step(spec.mcu.bor_level));
    text.push_str(&aethelgard_eda::fab::jlcpcb_notes(project));
    text
}

/// The campaign's conditions (`AET-WO-2304b` §1.7, §1.11), each as its
/// verdict states it; none for a campaign that names none, so every other
/// board's notes keep their bytes.
fn campaign_section(conditions: &[String]) -> String {
    if conditions.is_empty() {
        return String::new();
    }
    let mut text = String::from(
        "\n## What the fault campaign does not claim\n\nEach as the campaign's verdicts state it:\n\n",
    );
    for c in conditions {
        text.push_str(&format!("- {c}\n"));
    }
    text
}

/// The three values of `FLASH_OPTSR_CUR` on record (`AET-WO-4230` R8, the
/// architect's ruling on Q-J), each with `BOR_LEV` `00`: they agree in the
/// low 16 bits only, so no fixed word is written.
/// - read on the bench: `AET-WO-3302`, a NUCLEO-H753ZI rev V, 2026-11-16
///   05:00 UTC;
/// - RM0433 Rev 9 Table 21 (p. 177), the factory word with the
///   part-dependent bits 28:27 and 20:19 (`X` in the table) at 0;
/// - Renode 1.17.0's model (`AET-WO-4230` FEEDBACK §2, measured).
const OPTSR_BENCH: u32 = 0x1BC6_AAF0;
const OPTSR_TABLE_21: u32 = 0x0006_AAF0;
const OPTSR_RENODE: u32 = 0x0406_AAF0;

/// The factory step that programs the declared brown-out level and reads it
/// back (`AET-WO-4230` §1.6): none for level 0, which is the factory value
/// (RM0433 Rev 9 Table 21, p. 177). The firmware never programs option
/// bytes; it reads `FLASH_OPTSR_CUR` into its boot record. The step is a
/// read-modify-write of the word as read, the only correct procedure (the
/// architect's ruling on Q-J), and cites the three values on record. Every address,
/// key and bit is RM0433 Rev 9's, at the section and page named in the
/// text; no programmer's command is cited, because ST's STM32CubeProgrammer
/// manual (UM2237) is not held, and the architect ruled it is not fetched
/// (Q-J).
fn brown_out_step(level: u8) -> String {
    if level == 0 {
        return String::new();
    }
    let bits = format!("{:02b}", level & 3);
    let (k1, k2) = (
        aethelgard_twin::boot::OPTKEY1,
        aethelgard_twin::boot::OPTKEY2,
    );
    let bench = OPTSR_BENCH | u32::from(level & 3) << 2;
    let lines = [
        String::new(),
        "## Brown-out level".to_owned(),
        String::new(),
        format!("The specification declares brown-out reset level {level} (`mcu.bor_level`). A"),
        "part leaves the factory at level 0 (RM0433 Rev 9 Table 21, p. 177), and the".to_owned(),
        "firmware reads the level but never programs option bytes, so each part is".to_owned(),
        "programmed once, before the board is used, and read back:".to_owned(),
        String::new(),
        "1. Read `FLASH_OPTSR_CUR` (0x5200201C; RM0433 Rev 9 §4.9.8, p. 212).".to_owned(),
        format!("2. Write 0x{k1:08X}, then 0x{k2:08X}, to `FLASH_OPTKEYR` (0x52002008) to"),
        "   unlock `FLASH_OPTCR` (§4.5.1, p. 182).".to_owned(),
        "3. Write to `FLASH_OPTSR_PRG` (0x52002020) the word read in step 1 with".to_owned(),
        format!("   bits 3:2 (`BOR_LEV`) set to `{bits}` and every other bit as read: the"),
        "   reserved bits must keep their reset value, and `RDP` (bits 15:8) must".to_owned(),
        "   stay 0xAA, since any other value raises the readout protection level".to_owned(),
        "   (§4.9.9, pp. 215-216). Read-modify-write of the word as read is the only".to_owned(),
        "   procedure: no fixed word is right for every part. Three values are on".to_owned(),
        "   record, each with `BOR_LEV` 00, and they agree only in bits 15:0:".to_owned(),
        format!("   - 0x{OPTSR_BENCH:08X}, read on the bench (AET-WO-3302, NUCLEO-H753ZI rev V,"),
        format!("     2026-11-16 05:00 UTC), for which the word to write is 0x{bench:08X};"),
        format!("   - 0x{OPTSR_TABLE_21:08X}, RM0433 Rev 9 Table 21 (p. 177), the factory word"),
        "     with its part-dependent bits 28:27 and 20:19 at 0;".to_owned(),
        format!("   - 0x{OPTSR_RENODE:08X}, the twin's model (Renode 1.17.0)."),
        "4. Set `OPTSTART` (`FLASH_OPTCR`, 0x52002018, bit 1), wait until `OPT_BUSY`".to_owned(),
        "   (`FLASH_OPTSR_CUR` bit 0) reads 0 (§4.4.3, p. 175), then set `OPTLOCK`".to_owned(),
        "   (`FLASH_OPTCR` bit 0).".to_owned(),
        format!("5. Read `FLASH_OPTSR_CUR` again: bits 3:2 must read `{bits}`. The firmware's"),
        format!("   boot record then holds the same word, and a level other than {level} is"),
        "   `BrownOutLevelNotProgrammed`.".to_owned(),
        String::new(),
        "No programmer's command is given: ST's STM32CubeProgrammer user manual".to_owned(),
        "(UM2237), which would state its option-byte syntax, is not held, and none".to_owned(),
        "is cited here.".to_owned(),
    ];
    lines.join("\n") + "\n"
}

/// Every file of the package but the notes and the manifest, with what it is.
fn describe<'a>(
    project: &str,
    layers: &'a [(String, String, usize, String)],
) -> Vec<(String, &'a str)> {
    let mut out: Vec<(String, &'a str)> = layers
        .iter()
        .map(|(_, suffix, _, what)| (format!("{project}-{suffix}"), what.as_str()))
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
            format!("{project}-jlcpcb-bom.csv"),
            "bill of materials in JLCPCB's upload format, its part number empty (below)",
        ),
        (
            format!("{project}-jlcpcb-cpl.csv"),
            "placement in JLCPCB's upload format, rotation in KiCad's frame (below)",
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

/// JLCPCB's BOM and placement file (`AET-WO-1604` §1.3, ruled on PR #199).
/// A part on the bottom side refuses the package by its reference, exit 3,
/// `BottomSidePart`: JLCPCB states no bottom-side convention, and no row is
/// written that cannot be filled (`INV-06`). Inputs that are not what KiCad
/// and the emitter write are `ToolError`.
fn jlcpcb(bom: &[u8], pos: &[u8]) -> Result<(Vec<u8>, Vec<u8>), Stop> {
    use aethelgard_eda::fab::JlcpcbRefusal;
    match aethelgard_eda::fab::jlcpcb(bom, pos) {
        Ok((b, c)) => Ok((b.into_bytes(), c.into_bytes())),
        Err(JlcpcbRefusal::BottomSide(reference)) => Err(Stop {
            code: crate::EXIT_UNREADABLE,
            kind: "BottomSidePart",
            message: format!(
                "`{reference}` is placed on the bottom side, and JLCPCB's assembly pages state no bottom-side convention (mirrored X, rotation seen from below): no placement row is written that cannot be filled, so no package is written"
            ),
            id: Some(reference),
        }),
        Err(JlcpcbRefusal::Malformed(m)) => {
            Err(Stop::tool_error(TOOL, format!("the JLCPCB files: {m}")))
        }
    }
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
    /// Each precondition of the package and its state (`AET-WO-3303` §1.1),
    /// as the verdict's `fab.preconditions` states them.
    preconditions: &'a [Precondition],
    verdict_sha256: String,
    files: BTreeMap<&'a str, String>,
}

/// One precondition of a package: `met`, or `unmet` with what fails it.
#[derive(Serialize, Clone)]
pub struct Precondition {
    pub precondition: &'static str,
    pub state: &'static str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unmet: Vec<String>,
    /// Coverage: each bus no device uses, named and not blocking
    /// (`AET-WO-3303` D1, ruled (c)).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub no_device: Vec<String>,
}

impl Precondition {
    pub fn of(precondition: &'static str, unmet: Vec<String>) -> Self {
        let state = if unmet.is_empty() { "met" } else { "unmet" };
        Self {
            precondition,
            state,
            unmet,
            no_device: Vec::new(),
        }
    }
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
    pub preconditions: &'a [Precondition],
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
        preconditions: n.preconditions,
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

    /// `AET-WO-2304b` §2: FAB-NOTES states every campaign condition
    /// verbatim, in a section of its own; a campaign with none adds no byte.
    #[test]
    fn the_campaign_s_conditions_are_stated_verbatim_and_only_when_there_are_some() {
        assert_eq!(campaign_section(&[]), "");
        let conditions = [
            "The battery's load is the tree's declared static load; no motor current is commanded, so no reboot loop is shown or excluded.".to_owned(),
            "`VDD_5V` below its band is rendered as its consumers' absence (CRSF and MAVLINK RX open).".to_owned(),
        ];
        let text = campaign_section(&conditions);
        assert!(text.starts_with("\n## What the fault campaign does not claim\n"));
        for c in &conditions {
            assert!(text.contains(&format!("- {c}\n")), "{text}");
        }
    }

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
            preconditions: &[
                Precondition::of("met_one", Vec::new()),
                Precondition::of("unmet_one", vec!["a".to_owned()]),
            ],
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
        // `AET-WO-3303` §1.1: the package itself names each precondition.
        assert_eq!(
            v["preconditions"],
            serde_json::json!([
                {"precondition": "met_one", "state": "met"},
                {"precondition": "unmet_one", "state": "unmet", "unmet": ["a"]}
            ])
        );
        assert_eq!(v["files"]["a"], tree::sha256(b"1"));
        assert_eq!(v["files"].as_object().unwrap().len(), 2);
        assert!(m.ends_with(b"}\n"));
    }

    /// `AET-WO-1604` §1.3, on KiCad 10.0.7's placement file and the
    /// emitter's BOM for Sentinel-01 as ordered (the committed package at
    /// `ac5f8c8`): the JLCPCB files carry exactly what those two say, with
    /// the part number empty for every kind of part and KiCad's rotation
    /// normalised, never offset (adversarial: no column holds a guess).
    #[test]
    fn the_jlcpcb_files_from_real_kicad_output_carry_nothing_invented() {
        let pos = include_bytes!("../tests/fixtures/fab/kicad-10.0.7-sentinel-01-bench-pos.csv");
        let bom = include_bytes!("../tests/fixtures/fab/emit-sentinel-01-bench.bom.csv");
        let (b, c) = jlcpcb(bom, pos).unwrap();
        let (b, c) = (String::from_utf8(b).unwrap(), String::from_utf8(c).unwrap());
        let b: Vec<&str> = b.strip_suffix("\r\n").unwrap().split("\r\n").collect();
        let c: Vec<&str> = c.strip_suffix("\r\n").unwrap().split("\r\n").collect();
        assert_eq!(
            b[0],
            "Comment,Designator,Footprint,JLCPCB Part #（optional）"
        );
        assert_eq!(c[0], "Designator,Mid X,Mid Y,Layer,Rotation");
        // 13 kinds of part over 39 references (FEEDBACK §4, measured).
        assert_eq!(b.len(), 1 + 13);
        assert_eq!(
            b[1],
            "100n,\"C1,C2,C3,C4,C5,C6,C7,C8,C9,C10,C11,C13,C15,C19,C20,C21,C22\",C_0402_1005Metric,"
        );
        for row in &b[1..] {
            assert!(row.ends_with(','), "a part number was written: {row}");
        }
        assert!(b.contains(&"TMP117xxDRV,\"U3\",WSON-6-1EP_2x2mm_P0.65mm_EP1x1.6mm,"));
        // Every placement row, one for one, in KiCad's order.
        let kicad = aethelgard_eda::fab::placement(pos).unwrap();
        assert_eq!(c.len(), 1 + kicad.len());
        assert_eq!(kicad.len(), 39);
        for (row, k) in c[1..].iter().zip(&kicad) {
            let r = match k.rotation {
                "-90.000000" => "270",
                "0.000000" => "0",
                "90.000000" => "90",
                "180.000000" => "180",
                other => panic!("a rotation this fixture does not hold: {other}"),
            };
            assert_eq!(
                *row,
                format!("\"{}\",{}mm,{}mm,T,{r}", k.reference, k.x, k.y),
                "{row}"
            );
        }
    }

    #[test]
    fn a_bottom_side_part_refuses_the_package_by_name() {
        let pos = b"Ref,Val,Package,PosX,PosY,Rot,Side\n\"R1\",\"1k\",\"R_0402_1005Metric\",1.000000,-2.000000,0.000000,bottom\n";
        let bom = b"Reference,Value,Footprint,Symbol,Note\r\nR1,1k,Resistor_SMD:R_0402_1005Metric,Device:R,\r\n";
        let stop = jlcpcb(bom, pos).unwrap_err();
        assert_eq!(
            (stop.code, stop.kind),
            (crate::EXIT_UNREADABLE, "BottomSidePart")
        );
        assert_eq!(stop.id.as_deref(), Some("R1"));
        assert!(
            stop.message.contains("bottom-side convention"),
            "{}",
            stop.message
        );
    }

    #[test]
    fn the_notes_describe_every_file_and_the_package_names_them_after_the_project() {
        let spec: aethelgard_ir::Specification =
            serde_json::from_str(&repo_file("examples/sentinel-01-minimal.json")).unwrap();
        let layers = gerbers(&aethelgard_eda::stack::Stack::of(&spec).unwrap());
        let d = describe("p", &layers);
        assert_eq!(d.len(), LAYERS.len() + 6);
        assert!(
            d.iter()
                .all(|(n, _)| n.starts_with("p-") || n.starts_with("p."))
        );
    }

    /// The tool `derive` expects of a board whose only holes are `n` routed
    /// vias at the board's own rule.
    fn vias_only(n: usize) -> Vec<aethelgard_eda::fab::Drill> {
        vias_at((0..n as i64).map(|i| (i * 1_000_000, 0)).collect())
    }

    /// The same, its vias at `at` (board coordinates, nm, sorted).
    fn vias_at(at: Vec<(i64, i64)>) -> Vec<aethelgard_eda::fab::Drill> {
        vec![aethelgard_eda::fab::Drill {
            drill_nm: aethelgard_eda::route::DRILL_NM,
            hole: aethelgard_eda::fab::Hole::Via,
            what: vec![(
                aethelgard_eda::fab::Drilled::Routed {
                    via_nm: aethelgard_eda::route::VIA_NM,
                },
                at.len(),
            )],
            at,
        }]
    }

    /// A drill file's hits as board coordinates (nm, Y down), read here
    /// independently of `fab::holes`: `X<mm>Y<mm>` with Y up.
    fn hits(drl: &[u8]) -> Vec<(i64, i64)> {
        let nm = |t: &str| -> i64 {
            let (neg, t) = t.strip_prefix('-').map_or((false, t), |r| (true, r));
            let (w, f) = t.split_once('.').unwrap();
            let v =
                w.parse::<i64>().unwrap() * 1_000_000 + format!("{f:0<6}").parse::<i64>().unwrap();
            if neg { -v } else { v }
        };
        let mut at: Vec<(i64, i64)> = std::str::from_utf8(drl)
            .unwrap()
            .lines()
            .filter_map(|l| l.strip_prefix('X')?.split_once('Y'))
            .map(|(x, y)| (nm(x), -nm(y)))
            .collect();
        at.sort_unstable();
        at
    }

    /// `AET-WO-4300d2` §2: every existing board's package is byte-identical.
    /// KiCad's files pass through unchanged, so only `FAB-NOTES.md` (and the
    /// manifest's hash of it) could move. Each golden is the notes the code
    /// before `AET-WO-4300d2` wrote for that specification (captured at
    /// 9ce769c, same project, engine version and specification hash), and
    /// today's notes for its via-only board are those bytes: 2 layers, 4
    /// layers, and a net class with its own via (the clause that sentence
    /// carries).
    #[test]
    fn a_via_only_boards_notes_are_byte_identical_to_before_4300d2() {
        for (spec, project, golden) in [
            (
                "examples/sentinel-01-minimal.json",
                "sentinel-01",
                include_str!("../tests/fixtures/fab/notes-before-4300d2-sentinel-01-minimal.md"),
            ),
            (
                "crates/aethelgard-cli/tests/fixtures/four-layer.json",
                "sentinel-01-4l",
                include_str!("../tests/fixtures/fab/notes-before-4300d2-four-layer.md"),
            ),
            (
                "crates/aethelgard-cli/tests/fixtures/four-layer-power.json",
                "p",
                include_str!("../tests/fixtures/fab/notes-before-4300d2-four-layer-power.md"),
            ),
        ] {
            let spec: aethelgard_ir::Specification =
                serde_json::from_str(&repo_file(spec)).unwrap();
            // The conditions the code before 4300d2 wrote for these boards
            // (none declares off-board capacitance), which an `extract`
            // verdict now hands the notes (`AET-WO-3303`); no lifecycle line.
            let stated = aethelgard_eda::extract::conditions(
                &aethelgard_eda::stack::Stack::of(&spec).unwrap(),
            );
            // `AET-WO-4230` §1.6: a board declaring a brown-out level adds
            // its factory step after the files, and nothing else moves.
            // `AET-WO-1604` §1.3: the two JLCPCB files join the file list,
            // which is sorted by name, and their section ends the notes.
            let pos = format!("- `{project}-pos.csv`:");
            let golden = golden.replace(
                &pos,
                &format!(
                    "- `{project}-jlcpcb-bom.csv`: bill of materials in JLCPCB's upload format, its part number empty (below)\n\
                     - `{project}-jlcpcb-cpl.csv`: placement in JLCPCB's upload format, rotation in KiCad's frame (below)\n{pos}"
                ),
            );
            let golden = format!(
                "{golden}{}{}",
                brown_out_step(spec.mcu.bor_level),
                aethelgard_eda::fab::jlcpcb_notes(project)
            );
            for n in [1, 64] {
                assert_eq!(
                    notes(
                        project,
                        "0.1.0",
                        "SPEC_SHA",
                        &spec,
                        &aethelgard_eda::stack::Stack::of(&spec).unwrap(),
                        &vias_only(n),
                        &Stated {
                            conditions: &stated,
                            lifecycle: &[],
                            support: &[],
                            campaign: &[],
                        },
                    ),
                    golden,
                    "{project}"
                );
            }
        }
    }

    /// `AET-WO-4230` §1.6, WO §5.3 (a package with no factory step): a level
    /// above 0 carries the step that programs it, its exact factory word and
    /// the read-back; level 0, the factory value, carries none. The step's
    /// addresses, keys and bits are the header's and the twin's.
    #[test]
    fn a_declared_brown_out_level_adds_its_factory_step_and_level_0_none() {
        assert_eq!(brown_out_step(0), "");
        let step = brown_out_step(3);
        assert!(step.starts_with(
            "\n## Brown-out level\n\nThe specification declares brown-out reset level 3"
        ));
        for want in [
            "bits 3:2 (`BOR_LEV`) set to `11`",
            // `AET-WO-4230` R8: the three values on record, and the one
            // procedure.
            "Read-modify-write of the word as read is the only\n   procedure",
            "   - 0x1BC6AAF0, read on the bench (AET-WO-3302, NUCLEO-H753ZI rev V,\n     2026-11-16 05:00 UTC), for which the word to write is 0x1BC6AAFC;",
            "   - 0x0006AAF0, RM0433 Rev 9 Table 21 (p. 177)",
            "   - 0x0406AAF0, the twin's model (Renode 1.17.0).",
            "bits 3:2 must read `11`",
            "a level other than 3 is\n   `BrownOutLevelNotProgrammed`",
            "Write 0x08192A3B, then 0x4C5D6E7F, to `FLASH_OPTKEYR` (0x52002008)",
            "No programmer's command is given",
        ] {
            assert!(step.contains(want), "{want}: {step}");
        }
        assert!(brown_out_step(1).contains("the word to write is 0x1BC6AAF4;"));
        assert!(!step.contains("0x0006AAFC"), "no fixed word from the table");
        assert!(brown_out_step(2).contains("set to `10`"));
        // The addresses the text states are the header's.
        let f = aethelgard_svd_hal::cmsis::register_facts().unwrap();
        let flash = f.layout("FLASH").unwrap();
        let base = f.base("FLASH_R").unwrap();
        for (reg, at) in [
            ("OPTKEYR", 0x5200_2008u32),
            ("OPTCR", 0x5200_2018),
            ("OPTSR_CUR", 0x5200_201C),
            ("OPTSR_PRG", 0x5200_2020),
        ] {
            assert_eq!(base + flash.offset(reg, 0).unwrap(), at, "{reg}");
            assert!(
                step.contains(&format!("{at:#010X}").replace("0X", "0x")),
                "{reg}"
            );
        }
        // Each line of the section fits the notes' width.
        assert!(step.lines().all(|l| l.len() <= 80), "{step}");
    }

    /// The reference's drill file as KiCad 10.0.7 wrote it from the emitted
    /// `sentinel-01` board (64 vias), pinned: the one tool and its hits, and
    /// an off-by-one either way refused; and each hole where the board has
    /// one: a via moved by a micrometre is refused (`AET-WO-1311` §1.3).
    #[test]
    fn todays_drill_file_holds_exactly_its_vias() {
        let drl = include_bytes!("../tests/fixtures/fab/kicad-10.0.7-vias.drl");
        let at = hits(drl);
        assert_eq!(at.len(), 64);
        assert_eq!(
            aethelgard_eda::fab::holes(drl, &vias_at(at.clone())),
            Ok(())
        );
        let mut moved = at.clone();
        moved[10].0 += 1_000;
        moved.sort_unstable();
        let e = aethelgard_eda::fab::holes(drl, &vias_at(moved)).unwrap_err();
        assert!(e.contains("tool T1 (0.3 mm) drills at"), "{e}");
        for n in [63, 65] {
            let mut at = at.clone();
            at.resize(n, (0, 0));
            at.sort_unstable();
            let e = aethelgard_eda::fab::holes(drl, &vias_at(at)).unwrap_err();
            assert!(
                e.contains(&format!("tool T1 (0.3 mm) has 64 hits, where the board holds {n}: routed vias, 0.6 mm pad ({n})")),
                "{e}"
            );
        }
        let e = aethelgard_eda::fab::holes(drl, &[]).unwrap_err();
        assert!(e.contains("no holes"), "{e}");
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
            gerbers(&aethelgard_eda::stack::Stack::of(&spec).unwrap())
        };
        let two = load(&repo_file("examples/sentinel-01-minimal.json"));
        let today: Vec<(String, String, usize, String)> = LAYERS
            .iter()
            .map(|(l, s, c, w)| ((*l).to_owned(), (*s).to_owned(), *c, (*w).to_owned()))
            .collect();
        assert_eq!(two, today);
        let four = load(&repo_file(
            "crates/aethelgard-cli/tests/fixtures/four-layer.json",
        ));
        let inner: Vec<(&str, &str, usize)> = four
            .iter()
            .filter(|(l, ..)| l.starts_with("In"))
            .map(|(l, s, c, _)| (l.as_str(), s.as_str(), *c))
            .collect();
        assert_eq!(
            inner,
            [("In1.Cu", "In1_Cu.g1", 9), ("In2.Cu", "In2_Cu.g2", 9)]
        );
        assert_eq!(four.len(), LAYERS.len() + 2);
    }
}
