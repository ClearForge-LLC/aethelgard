//! The text of a fabrication package's `FAB-NOTES.md` (`AET-WO-3300` §1.2).
//!
//! Every number comes from the specification's declared board
//! ([`crate::stack::Stack`], `AET-WO-4100`): the layers, the stackup and its
//! sources, the rule minima, and the conditions as [`crate::extract`] states
//! them. At 2 layers the words are `AET-WO-3300`'s, so the reference's notes
//! are unchanged (ruled D5). The text claims what the package's verdict shows
//! and nothing beyond it (`RUL-19`).
//!
//! It also states the drill tools the package's drill file must hold
//! ([`drills`], `AET-WO-4300d2` §1.6), from the board the gates judged, and
//! checks KiCad's drill file against them ([`holes`]).

use std::collections::BTreeMap;

use crate::extract::permille;
use crate::route::{Board, DRILL_NM, Routed, VIA_NM};
use crate::stack::Stack;

/// What a tool drills. KiCad 10.0.7 keeps a via's hole and a through-hole
/// pad's in separate tools even at one diameter, the via's first (measured,
/// `AET-WO-4300d2` FEEDBACK), so this is the order of the variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Hole {
    Via,
    Pad,
}

/// One kind of hole a tool drills.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Drilled {
    /// Routed vias of one pad size: the board's or a net class's.
    Routed { via_nm: i64 },
    /// The thermal-via array under a declared pad (`AET-WO-1309` §1.2).
    Thermal {
        reference: String,
        pad: String,
        via_nm: i64,
    },
    /// A part's plated through-hole pads of one number (empty for a pad with
    /// no number).
    Pad { reference: String, number: String },
}

/// One tool of the drill file, as the judged board predicts it: its
/// diameter, what it drills, how many holes of each kind, and every hole's
/// centre in board coordinates (nm, Y down), sorted (`AET-WO-1311` §1.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drill {
    pub drill_nm: i64,
    pub hole: Hole,
    pub what: Vec<(Drilled, usize)>,
    pub at: Vec<(i64, i64)>,
}

impl Drill {
    /// Its number of holes.
    pub fn hits(&self) -> usize {
        self.what.iter().map(|(_, n)| n).sum()
    }

    /// The header line KiCad 10.0.7 writes for it as tool `n` (measured:
    /// `T1C0.300`, millimetres to three places).
    pub fn tool_line(&self, n: usize) -> String {
        format!(
            "T{n}C{}.{:03}",
            self.drill_nm / 1_000_000,
            self.drill_nm % 1_000_000 / 1000
        )
    }

    /// What it drills, as the notes and refusals name it.
    fn named(&self) -> String {
        let parts: Vec<String> = self
            .what
            .iter()
            .map(|(d, n)| {
                let what = match d {
                    Drilled::Routed { via_nm } => format!("routed vias, {} mm pad", mm(*via_nm)),
                    Drilled::Thermal {
                        reference,
                        pad,
                        via_nm,
                    } => format!(
                        "the thermal-via array under {reference} pad {pad}, {} mm pad",
                        mm(*via_nm)
                    ),
                    Drilled::Pad { reference, number } if number.is_empty() => {
                        format!("through-hole pads of {reference} with no number")
                    }
                    Drilled::Pad { reference, number } => {
                        format!("through-hole pad {number} of {reference}")
                    }
                };
                format!("{what} ({n})")
            })
            .collect();
        parts.join(", ")
    }
}

/// The drill file's tools, in KiCad 10.0.7's order (measured: ascending
/// diameter, a via's tool before a pad's at one diameter; routed vias and an
/// array's vias of one drill share a tool), from the board `route::read`
/// returned:
///
/// - each routed via at its net's rule's drill, which `read` requires the
///   via to carry (`AET-WO-1308`: a class's declared via drill included);
/// - each thermal array's vias at its declared rule's drill, never the
///   board's own, so an array drilled otherwise disagrees with KiCad's file;
/// - each plated through-hole pad's drill, a pad with no number included.
///
/// `read` has already refused every other hole: an unplated one, an oval or
/// offset drill, a hole with no copper. A drill that is not a whole
/// micrometre cannot be stated in KiCad's three places, and is refused.
pub fn drills(board: &Board, routed: &Routed) -> Result<Vec<Drill>, String> {
    type Tool = (BTreeMap<Drilled, usize>, Vec<(i64, i64)>);
    let mut tools: BTreeMap<(i64, Hole), Tool> = BTreeMap::new();
    let mut add = |drill_nm: i64, hole: Hole, what: Drilled, at: &[(i64, i64)]| {
        let tool = tools.entry((drill_nm, hole)).or_default();
        *tool.0.entry(what).or_default() += at.len();
        tool.1.extend_from_slice(at);
    };
    for v in &routed.vias {
        let rule = board.stack.rule(&v.net);
        add(
            rule.drill_nm,
            Hole::Via,
            Drilled::Routed {
                via_nm: rule.via_nm,
            },
            &[v.at],
        );
    }
    for a in &routed.arrays {
        let rule = board
            .stack
            .thermal
            .iter()
            .find(|t| t.reference == a.reference && t.pad == a.pad)
            .ok_or_else(|| {
                format!(
                    "the thermal vias under {} pad {} have no declared rule",
                    a.reference, a.pad
                )
            })?;
        add(
            rule.drill_nm,
            Hole::Via,
            Drilled::Thermal {
                reference: a.reference.clone(),
                pad: a.pad.clone(),
                via_nm: rule.via_nm,
            },
            &a.at,
        );
    }
    for part in &board.parts {
        for pad in &part.pads {
            if let Some(d) = pad.drill_nm {
                // `route::read` holds a pad's drill at its centre, and lays its
                // box from there as `c − w/2 .. c + w − w/2` (`AET-WO-1311`
                // adversarial L: an odd width's centre is still exact).
                let r = pad.rect;
                add(
                    d,
                    Hole::Pad,
                    Drilled::Pad {
                        reference: part.reference.clone(),
                        number: pad.number.clone(),
                    },
                    &[(r.x0 + (r.x1 - r.x0) / 2, r.y0 + (r.y1 - r.y0) / 2)],
                );
            }
        }
    }
    let mut out = Vec::new();
    for ((drill_nm, hole), (what, mut at)) in tools {
        if drill_nm <= 0 || drill_nm % 1000 != 0 {
            return Err(format!(
                "a drill of {drill_nm} nm is not a whole micrometre, which KiCad's tool line (three places in mm) cannot state"
            ));
        }
        at.sort_unstable();
        out.push(Drill {
            drill_nm,
            hole,
            what: what.into_iter().collect(),
            at,
        });
    }
    Ok(out)
}

/// Whether `s` is a decimal KiCad writes in a hit: an optional `-`, digits,
/// a point and digits (measured: `X105.875Y-109.875`, `X102.0Y-106.0`).
fn decimal(s: &str) -> bool {
    let s = s.strip_prefix('-').unwrap_or(s);
    s.split_once('.').is_some_and(|(a, b)| {
        !a.is_empty()
            && !b.is_empty()
            && a.bytes().all(|c| c.is_ascii_digit())
            && b.bytes().all(|c| c.is_ascii_digit())
    })
}

/// A decimal KiCad writes in a hit, in millimetres, as nanometres: at most
/// six places.
fn nm(s: &str) -> Option<i64> {
    if !decimal(s) {
        return None;
    }
    let (neg, s) = s.strip_prefix('-').map_or((false, s), |r| (true, r));
    let (whole, frac) = s.split_once('.')?;
    if frac.len() > 6 {
        return None;
    }
    let v = whole
        .parse::<i64>()
        .ok()?
        .checked_mul(1_000_000)?
        .checked_add(format!("{frac:0<6}").parse::<i64>().ok()?)?;
    Some(if neg { -v } else { v })
}

/// The hole `line` places, in board coordinates (nm, Y down: the file's Y
/// is up, measured: a via at (122.375, 115.125) mm is `X122.375Y-115.125`),
/// when it is one round hit, `X<decimal>Y<decimal>`, and nothing else: a
/// slot (`…G85X…`) is not.
fn hit(line: &str) -> Option<(i64, i64)> {
    let (x, y) = line.strip_prefix('X')?.split_once('Y')?;
    Some((nm(x)?, nm(y)?.checked_neg()?))
}

/// KiCad's drill file (`kicad-cli pcb export drill --excellon-min-header`)
/// holds exactly the board's tools (`AET-WO-4300d2` §1.6). As measured from
/// KiCad 10.0.7, it is one file:
///
/// ```text
/// M48 / METRIC / T1C0.254 / T2C0.300 / … / % / G90 / G05 /
/// T1 / X…Y… … / T2 / X…Y… … / M30
/// ```
///
/// Each of `expected`'s tools must be in the header in order with its
/// diameter, and selected once, in order, in the body, followed by exactly
/// its holes, one round hit per line, at exactly its holes' positions in
/// any order (`AET-WO-1311` §1.3). A tool missing, an extra tool, a wrong
/// diameter, a hit count off by one, a hole moved or exchanged with another
/// tool's, a slot or any line not in that shape is refused, naming the tool
/// and what it drills. A board with no hole is refused: its drill file is
/// not measured.
pub fn holes(drill: &[u8], expected: &[Drill]) -> Result<(), String> {
    let text = std::str::from_utf8(drill)
        .ok()
        .filter(|t| t.is_ascii() && !t.contains('\r') && t.ends_with('\n'))
        .ok_or("it is not ASCII lines ending in a newline")?;
    if expected.is_empty() {
        return Err(
            "the board has no holes, and KiCad's drill file for none is not measured".to_owned(),
        );
    }
    let mut lines = text.lines();
    if lines.next() != Some("M48") || lines.next() != Some("METRIC") {
        return Err("it does not begin M48, METRIC".to_owned());
    }
    let header: Vec<&str> = lines.by_ref().take_while(|l| *l != "%").collect();
    let want: Vec<String> = expected
        .iter()
        .enumerate()
        .map(|(i, d)| d.tool_line(i + 1))
        .collect();
    for (i, d) in expected.iter().enumerate() {
        match header.get(i) {
            Some(got) if *got == want[i] => {}
            Some(got) => {
                return Err(format!(
                    "tool {} is {got}, where the board's is {} ({} mm, {})",
                    i + 1,
                    want[i],
                    mm(d.drill_nm),
                    d.named()
                ));
            }
            None => {
                return Err(format!(
                    "tool {} is missing: the board's {} ({} mm, {})",
                    i + 1,
                    want[i],
                    mm(d.drill_nm),
                    d.named()
                ));
            }
        }
    }
    if let Some(extra) = header.get(expected.len()) {
        return Err(format!("it has a tool the board does not: {extra}"));
    }
    if lines.next() != Some("G90") || lines.next() != Some("G05") {
        return Err("its body does not begin G90, G05".to_owned());
    }
    let mut lines = lines.peekable();
    for (i, d) in expected.iter().enumerate() {
        let select = format!("T{}", i + 1);
        if lines.next() != Some(select.as_str()) {
            return Err(format!(
                "tool {select} is not selected next, after every earlier tool's holes"
            ));
        }
        let mut at = Vec::new();
        while lines.peek().is_some_and(|l| l.starts_with('X')) {
            let line = lines.next().unwrap_or_default();
            at.push(
                hit(line)
                    .ok_or_else(|| format!("under {select}, `{line}` is not one round hit"))?,
            );
        }
        let hits = at.len();
        if hits != d.hits() {
            return Err(format!(
                "tool {select} ({} mm) has {hits} hits, where the board holds {}: {}",
                mm(d.drill_nm),
                d.hits(),
                d.named()
            ));
        }
        at.sort_unstable();
        if let Some((got, want)) = at.iter().zip(&d.at).find(|(g, w)| g != w) {
            return Err(format!(
                "tool {select} ({} mm) drills at ({}, {}) mm where the board's next hole is at ({}, {}) mm: {}",
                mm(d.drill_nm),
                signed(got.0),
                signed(got.1),
                signed(want.0),
                signed(want.1),
                d.named()
            ));
        }
    }
    match (lines.next(), lines.next()) {
        (Some("M30"), None) => Ok(()),
        (Some(line), _) => Err(format!(
            "after the board's last tool's holes, `{line}` where KiCad ends with M30"
        )),
        (None, _) => Err("it does not end with M30".to_owned()),
    }
}

/// What the notes name that the board does not carry.
pub struct Notes<'a> {
    pub project: &'a str,
    pub engine_version: &'a str,
    pub specification_sha256: &'a str,
    /// The KiCad version whose output the Gerbers, drill and placement are.
    pub kicad_version: &'a str,
    /// Every other file of the package, in order, with what it is.
    pub files: &'a [(String, &'a str)],
    /// The board the specification declares.
    pub stack: &'a Stack,
    /// The bound's scope and conditions exactly as the `extract` verdict
    /// that judged this board states them (`AET-WO-4300f` F2): never
    /// assembled a second time here, so the notes cannot state fewer.
    pub conditions: &'a [String],
    /// The lifecycle lines the verdict states for the board's parts: the
    /// banner of each acknowledged `eol` or `nrnd` part and each part whose
    /// lifecycle no held page states (`AET-WO-4210` F-B3). None, no section.
    pub lifecycle: &'a [String],
    /// The drill file's tools ([`drills`]).
    pub drills: &'a [Drill],
    /// What the design-rule gate's verdict names as not judged on a board
    /// with a power tree ([`crate::loops::conditions`]): the package states
    /// every condition its verdict states (`AET-WO-4300f` F2,
    /// `AET-WO-4300d2` §1.6). Empty, and no section, for any other board.
    pub power: &'a [String],
    /// What the support gate's verdict judged of the MCU's support network,
    /// then each condition it names (`AET-WO-4300g` §1.6), as the gate
    /// states them. Empty, and no section, when no support gate ran.
    pub support: &'a [String],
    /// The specification's `acknowledge_unwitnessed` entries, in its order
    /// (`AET-WO-3402` §1.2): each register, class, fields and reason exactly
    /// as the specification states them. Empty writes a line saying so.
    pub acknowledged: &'a [aethelgard_ir::UnwitnessedAcknowledgement],
}

/// The section naming every ledger row the specification acknowledges as
/// unwitnessed (`RUL-20`, `AET-WO-4230` R1): one line per entry, its
/// register, class, fields and reason as written, so a reader can match each
/// line to the specification's entry.
pub fn acknowledged_section(rows: &[aethelgard_ir::UnwitnessedAcknowledgement]) -> String {
    let mut out = String::from(
        "## Acknowledged ledger rows\n\nThe witness ledger classes these register fields as judged by no independent rule, model or bench reading. The specification acknowledges each by name, with its reason, and the bench reads them (`RUL-20`):\n\n",
    );
    if rows.is_empty() {
        out.push_str("- None: the specification acknowledges no unwitnessed ledger row.\n");
    }
    for r in rows {
        let fields: Vec<String> = r.fields.iter().map(|f| format!("`{f}`")).collect();
        out.push_str(&format!(
            "- `{}` ({}): {}. {}\n",
            r.register,
            r.class,
            fields.join(", "),
            r.reason
        ));
    }
    out.push('\n');
    out
}

/// `nm` in millimetres, with no trailing zeros: 1_600_000 is `1.6`.
/// [`mm`] of a coordinate, which may be negative.
fn signed(nm: i64) -> String {
    if nm < 0 {
        format!("-{}", mm(nm.saturating_neg()))
    } else {
        mm(nm)
    }
}

fn mm(nm: i64) -> String {
    let whole = nm / 1_000_000;
    let frac = format!("{:06}", nm % 1_000_000);
    let frac = frac.trim_end_matches('0');
    if frac.is_empty() {
        whole.to_string()
    } else {
        format!("{whole}.{frac}")
    }
}

/// `FAB-NOTES.md`, byte-identical for the same `notes`.
pub fn notes(n: &Notes<'_>) -> String {
    let kicad = n.kicad_version;
    let st = n.stack;
    let um = st.copper_max_nm / 1000;
    let eps = permille(st.permittivity_permille);
    let eps_source = &st.permittivity_source;
    let stackup = stackup(st, n.drills);
    let mut files = String::new();
    for (name, what) in n.files {
        files.push_str(&format!("- `{name}`: {what}\n"));
    }
    let conditions: String = n.conditions.iter().map(|c| format!("- {c}\n")).collect();
    let lifecycle = if n.lifecycle.is_empty() {
        String::new()
    } else {
        let lines: String = n.lifecycle.iter().map(|l| format!("- {l}\n")).collect();
        format!(
            "## Lifecycle\n\nThe verdict that names this package states, for the board's parts:\n\n{lines}\n"
        )
    };
    format!(
        "# Fabrication notes: {project}

Generated by aethelgard {engine} from the specification whose sha256 is
`{spec}`.
`MANIFEST.json` names every other file of this package by sha256, and the
verdict of the `aethelgard verify` run that emitted the package names every
file, `MANIFEST.json` included. `aethelgard verify --spec <file> --scenarios
<the files MANIFEST.json names> --against-fab <this directory>`, with the
other arguments of the run that emitted it, derives the package again and
names every file that differs. `MANIFEST.json` also names that run's verdict
by sha256, and the verdict states the sandbox each tool ran in, so a run
under another uid or user namespace finds `MANIFEST.json` different while
every other file is the same.

## What this package is

It was emitted by a passing `aethelgard verify`: every gate that verdict
names passed on the design generated from the specification, and these
files were made from that design. The gates judged the design, not these
files. It claims nothing beyond that verdict. Emitting this package made
no board, and nothing in it was tested on hardware.

The Gerbers are KiCad {kicad}'s output (`kicad-cli pcb export gerbers
--check-zones`), exported from a copy of the generated board with the
sha256 the design-rule gate judged, prepared as that gate prepares its copy
(the specification's rules), with every zone refilled by KiCad. KiCad writes the time of export into two lines of every Gerber
(`%TF.CreationDate` and `G04 Created by KiCad ... date`). Those two lines
are removed so that the package is reproducible; nothing else in any file
is changed. The drill file (`kicad-cli pcb export drill
--excellon-min-header`) and the placement file (`kicad-cli pcb export pos
--format csv --units mm --side both`) are KiCad's output, unchanged. KiCad's
Gerber job file is not included; this file states the stackup.

## Stackup

{stackup}- {laminate}, with laminate and solder mask relative permittivity at
  most {eps} ({eps_source}).
- Declared thickness {declared} mm. Finished board from {min} mm to {max} mm thick.
- Finished copper at most {um} um thick on each layer.

A board made outside this stackup is not covered by the capacitance bound
of `aethelgard extract`. That bound's scope and conditions, as `extract`
states them:

{conditions}

## Holes

{holes}The
drill file is Excellon, metric, with decimal coordinates, and its minimal
header carries no plating comment.

## Rule minima

- Track width: {track} mm.
- Copper to copper: {space} mm.
- Via: {via} mm pad, {drill} mm drill.
{classes}{power}{support}
{lifecycle}{acknowledged}## Files

{files}
## Licence

Files generated by aethelgard carry no licence obligation from the
aethelgard project. The footprints are from KiCad's libraries, licensed
CC-BY-SA 4.0 with a design exception that covers designs made from them.
",
        project = n.project,
        engine = n.engine_version,
        spec = n.specification_sha256,
        laminate = if st.has_planes() {
            "Laminate: as the dielectrics' sources above state"
        } else {
            "Laminate: FR-4"
        },
        declared = mm(st.thickness.nominal_nm),
        min = mm(st.thickness.min_nm),
        max = mm(st.thickness.max_nm),
        drill = mm(DRILL_NM),
        via = mm(VIA_NM),
        track = mm(st.width_nm),
        space = mm(st.clearance_nm),
        holes = holes_text(st, n.drills),
        classes = classes(st),
        power = section(POWER_INTRO, n.power),
        support = section(SUPPORT_INTRO, n.support),
        lifecycle = lifecycle,
        acknowledged = acknowledged_section(n.acknowledged),
    )
}

/// The power tree's section, before the file list, when the verdict names
/// conditions: each as the gate states it.
const POWER_INTRO: &str = "\n## Power tree\n\nThe design-rule gate judged each loop part's distance to the pin it serves, each regulator's source and return pads, and each thermal array and through-hole join. Its verdict also states, and this package claims nothing beyond them:\n\n";

/// The MCU's support network, as the support gate's verdict states it.
const SUPPORT_INTRO: &str = "\n## MCU support network\n\nThe support gate judged the MCU's support network from KiCad's netlist of the schematic, against the MCU's datasheet as its parts row cites it, and each bounded capacitor's distance on the board. Its verdict states, and this package claims nothing beyond it:\n\n";

/// A notes section: `intro`, then each line as a bullet; nothing when there
/// are none.
fn section(intro: &str, lines: &[String]) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let mut out = intro.to_owned();
    for l in lines {
        out.push_str(&format!("- {l}\n"));
    }
    out
}

/// The Holes section's first paragraph, ending where "The drill file is
/// Excellon…" continues it. A board whose only holes are routed vias of the
/// router's drill, one tool, keeps `AET-WO-3300`'s sentence, so its notes are
/// unchanged; any other board's names every tool of the drill file, in its
/// order, with its diameter and what it drills (`AET-WO-4300d2` §1.6).
fn holes_text(st: &Stack, drills: &[Drill]) -> String {
    if let [d] = drills
        && d.drill_nm == DRILL_NM
        && d.what
            .iter()
            .all(|(w, _)| matches!(w, Drilled::Routed { .. }))
    {
        let class = if st.classes.iter().any(|c| c.via_declared) {
            ", but where a net class below declares its own via"
        } else {
            ""
        };
        return format!(
            "Every hole is a plated via, drilled {} mm, with a {} mm pad{class}. ",
            mm(DRILL_NM),
            mm(VIA_NM)
        );
    }
    let mut out = "Every hole is plated. The drill file has these tools, in its order,\neach with its diameter, its number of holes and what it drills:\n\n".to_owned();
    for (i, d) in drills.iter().enumerate() {
        out.push_str(&format!(
            "- T{}: {} mm, plated, {} holes: {}.\n",
            i + 1,
            mm(d.drill_nm),
            d.hits(),
            d.named()
        ));
    }
    out.push('\n');
    out
}

/// The declared net classes' and pairs' own rules (`AET-WO-1308`), one line
/// each; nothing for a specification that declares none.
fn classes(st: &Stack) -> String {
    let mut out = String::new();
    // A thermal array's vias are its rule's, under its pad, not the board's
    // via above (`AET-WO-1309` D1, `AET-WO-4300d2` §1.5).
    for t in &st.thermal {
        out.push_str(&format!(
            "- Thermal vias under {} pad {}: {} x {}, {} mm pad, {} mm drill, {} mm pitch.\n",
            t.reference,
            t.pad,
            t.columns,
            t.rows,
            mm(t.via_nm),
            mm(t.drill_nm),
            mm(t.pitch_nm)
        ));
    }
    for c in &st.classes {
        let via = if c.via_declared {
            format!(
                ", via {} mm pad, {} mm drill",
                mm(c.rule.via_nm),
                mm(c.rule.drill_nm)
            )
        } else {
            String::new()
        };
        out.push_str(&format!(
            "- Net class `{}` ({}): track width {} mm{via}.\n",
            c.name,
            c.nets.join(", "),
            mm(c.rule.width_nm)
        ));
    }
    for p in &st.pairs {
        out.push_str(&format!(
            "- Coupled pair `{}` ({}, {}): track width {} mm, {} mm apart.\n",
            p.name,
            p.nets[0],
            p.nets[1],
            mm(p.width_nm),
            mm(p.gap_nm)
        ));
    }
    out
}

/// The layer lines of the stackup section. At 2 layers, `AET-WO-3300`'s one
/// line. At 4, the layer order with each plane's net, every dielectric's
/// minimum with its source, the thickness's and copper's sources, and the
/// rounding rule (ruled D2): a fabricator builds from these.
fn stackup(st: &Stack, drills: &[Drill]) -> String {
    if !st.has_planes() {
        return "- Two copper layers: F.Cu (top) and B.Cu (bottom).\n".to_owned();
    }
    // Plated through-hole pads join the layers too (`AET-WO-4300d2`).
    let joined = if drills.iter().any(|d| d.hole == Hole::Pad) {
        "through vias and plated through-hole pads"
    } else {
        "through vias"
    };
    let mut out = format!(
        "- {} copper layers, top to bottom, joined only by {joined}:\n",
        st.layers()
    );
    for layer in &st.copper {
        match st.plane_of_layer(layer) {
            Some(net) => out.push_str(&format!("  - {layer}: plane, net {net}\n")),
            None => out.push_str(&format!("  - {layer}: signal\n")),
        }
    }
    out.push_str("- Each dielectric at least:\n");
    for (a, b, min, source) in &st.dielectrics {
        out.push_str(&format!("  - {a} to {b}: {} mm ({source})\n", mm(*min)));
    }
    out.push_str(&format!(
        "- Thickness source: {}. Copper source: {}.\n",
        st.thickness.source, st.copper_source
    ));
    out.push_str(
        "- Every minimum here is its source's value less its published tolerance, rounded down; every maximum, rounded up.\n",
    );
    out
}

/// The header of JLCPCB's BOM: its sample file `Sample-BOM_JLCSMT.xlsx`'s
/// four columns as it spells them, the part number's full-width parentheses
/// included (`AET-WO-1604` §1.3, ruled on PR #199: the sample files are the
/// format), then `Note`, the emitter's note on the part (`AET-WO-2308` (1)):
/// a stand-in is labelled wherever a person reads the package.
pub const JLCPCB_BOM_HEADER: &str = "Comment,Designator,Footprint,JLCPCB Part #（optional）,Note";

/// The header of JLCPCB's placement file (CPL), as `Sample-CPL_JLCSMT.xlsx`
/// spells it.
pub const JLCPCB_CPL_HEADER: &str = "Designator,Mid X,Mid Y,Layer,Rotation";

/// Why the JLCPCB files cannot be written.
#[derive(Debug, PartialEq, Eq)]
pub enum JlcpcbRefusal {
    /// A part KiCad places on the bottom side, by reference. JLCPCB's pages
    /// state no bottom-side convention (mirrored X, rotation seen from
    /// below), so no row is written for it, and no file at all (ruled U3).
    BottomSide(String),
    /// An input that is not what KiCad or the emitter writes.
    Malformed(String),
}

/// One row of KiCad's placement file (`kicad-cli pcb export pos --format csv
/// --units mm`): reference, value, package, X, Y, rotation and side, each as
/// KiCad printed it.
#[derive(Debug, PartialEq, Eq)]
pub struct Placed<'a> {
    pub reference: &'a str,
    pub value: &'a str,
    pub package: &'a str,
    pub x: &'a str,
    pub y: &'a str,
    pub rotation: &'a str,
    pub side: &'a str,
}

/// KiCad's placement file, row by row. KiCad writes each row's reference,
/// value and package in quotes without escaping a quote inside them, so a
/// name holding `"` shifts every later column (adversarial): every row must
/// be three quoted fields with no quote inside, then four unquoted ones;
/// commas inside quotes are CSV's own.
pub fn placement(csv: &[u8]) -> Result<Vec<Placed<'_>>, String> {
    let text = std::str::from_utf8(csv).map_err(|_| "it is not UTF-8".to_owned())?;
    let mut lines = text.lines();
    if lines.next() != Some("Ref,Val,Package,PosX,PosY,Rot,Side") {
        return Err("its header is not KiCad's CSV header".to_owned());
    }
    let mut rows = Vec::new();
    for (n, row) in lines.enumerate() {
        let bad = || {
            format!(
                "row {} is not three quoted fields and four plain ones",
                n + 1
            )
        };
        let mut rest = row;
        let mut quoted = Vec::new();
        for _ in 0..3 {
            let inner = rest.strip_prefix('"').ok_or_else(bad)?;
            let end = inner.find('"').ok_or_else(bad)?;
            quoted.push(&inner[..end]);
            rest = inner[end + 1..].strip_prefix(',').ok_or_else(bad)?;
        }
        let plain: Vec<&str> = rest.split(',').collect();
        if plain.len() != 4 || plain.iter().any(|f| f.is_empty() || f.contains('"')) {
            return Err(bad());
        }
        rows.push(Placed {
            reference: quoted[0],
            value: quoted[1],
            package: quoted[2],
            x: plain[0],
            y: plain[1],
            rotation: plain[2],
            side: plain[3],
        });
    }
    Ok(rows)
}

/// The emitter's BOM (`crate::bom`), read back strictly: its header, then
/// CRLF records of five RFC 4180 fields. Returns reference, value,
/// footprint and note of each row.
fn bom_rows(csv: &[u8]) -> Result<Vec<[String; 4]>, String> {
    let text = std::str::from_utf8(csv).map_err(|_| "the BOM is not UTF-8".to_owned())?;
    let body = text
        .strip_prefix(crate::bom::HEADER)
        .and_then(|b| b.strip_prefix("\r\n"))
        .ok_or("the BOM's header is not the emitter's")?;
    let mut rows = Vec::new();
    let mut fields: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut chars = body.chars().peekable();
    let mut start = true;
    while let Some(c) = chars.next() {
        match c {
            '"' if start && field.is_empty() => {
                start = false;
                loop {
                    match chars.next() {
                        Some('"') if chars.peek() == Some(&'"') => {
                            chars.next();
                            field.push('"');
                        }
                        Some('"') => break,
                        Some(c) => field.push(c),
                        None => return Err("the BOM ends inside a quoted field".to_owned()),
                    }
                }
                if !matches!(chars.peek(), Some(',' | '\r')) {
                    return Err("the BOM has text after a quoted field".to_owned());
                }
            }
            '"' => return Err("the BOM has a quote inside an unquoted field".to_owned()),
            ',' => {
                fields.push(std::mem::take(&mut field));
                start = true;
            }
            '\r' => {
                if chars.next() != Some('\n') {
                    return Err("the BOM has a bare CR".to_owned());
                }
                fields.push(std::mem::take(&mut field));
                let [reference, value, footprint, _symbol, note]: [String; 5] =
                    std::mem::take(&mut fields)
                        .try_into()
                        .map_err(|f: Vec<String>| {
                            format!("a BOM row has {} fields, not 5", f.len())
                        })?;
                rows.push([reference, value, footprint, note]);
                start = true;
            }
            '\n' => return Err("the BOM has a bare LF".to_owned()),
            c => {
                start = false;
                field.push(c);
            }
        }
    }
    if !field.is_empty() || !fields.is_empty() {
        return Err("the BOM's last row has no CRLF".to_owned());
    }
    Ok(rows)
}

/// Every part of the emitter's BOM that carries a note (a symbol standing
/// in for the ordered part, a value its datasheet does not state), in the
/// references' natural order: reference, value and note (`AET-WO-2308`
/// (1)). The package's notes list each one.
pub fn noted(bom: &[u8]) -> Result<Vec<[String; 3]>, String> {
    let mut out: Vec<[String; 3]> = bom_rows(bom)?
        .into_iter()
        .filter(|[.., note]| !note.is_empty())
        .map(|[reference, value, _, note]| [reference, value, note])
        .collect();
    out.sort_by(|a, b| natural(&a[0]).cmp(&natural(&b[0])));
    Ok(out)
}

/// A reference's order among others: `C2` before `C10`. A reference that is
/// not letters then digits sorts by its bytes after every one that is.
fn natural(reference: &str) -> (bool, &str, u64, &str) {
    let digits = reference.trim_start_matches(|c: char| !c.is_ascii_digit());
    let prefix = &reference[..reference.len() - digits.len()];
    match digits.parse::<u64>() {
        Ok(n) if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) => {
            (false, prefix, n, reference)
        }
        _ => (true, reference, 0, reference),
    }
}

/// One CSV field, quoted by RFC 4180 when it holds a comma, a quote or a
/// line break, or always when `always`.
fn csv_field(s: &str, always: bool) -> String {
    if always || s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_owned()
    }
}

/// KiCad's rotation, in degrees as it prints them (`-90.000000`), normalised
/// to 0 up to but not including 360, counter-clockwise as both KiCad and
/// JLCPCB count it; whole degrees print without a fraction (`270`).
fn rotation(kicad: &str) -> Option<String> {
    const MICRO: i128 = 1_000_000;
    let (negative, digits) = match kicad.strip_prefix('-') {
        Some(d) => (true, d),
        None => (false, kicad),
    };
    let (whole, frac) = digits.split_once('.').unwrap_or((digits, ""));
    let numeric = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    if whole.is_empty() || whole.len() > 12 || frac.len() > 6 || !numeric(whole) || !numeric(frac) {
        return None;
    }
    let frac_micro = format!("{frac:0<6}").parse::<i128>().ok()?;
    let mut micro = whole.parse::<i128>().ok()? * MICRO + frac_micro;
    if negative {
        micro = -micro;
    }
    let turn = 360 * MICRO;
    let r = micro.rem_euclid(turn);
    let (deg, rest) = (r / MICRO, r % MICRO);
    Some(if rest == 0 {
        deg.to_string()
    } else {
        format!("{deg}.{}", format!("{rest:06}").trim_end_matches('0'))
    })
}

/// A coordinate as KiCad printed it, in millimetres: a plain decimal.
fn coordinate(kicad: &str) -> Option<&str> {
    let digits = kicad.strip_prefix('-').unwrap_or(kicad);
    let (whole, frac) = digits.split_once('.').unwrap_or((digits, ""));
    (!whole.is_empty()
        && whole.bytes().all(|b| b.is_ascii_digit())
        && frac.bytes().all(|b| b.is_ascii_digit()))
    .then_some(kicad)
}

/// JLCPCB's BOM and placement file (`AET-WO-1604` §1.3, ruled on PR #199),
/// from the emitter's BOM and KiCad's placement file, adding nothing either
/// lacks (`INV-06`):
///
/// - **BOM**, one row per kind of part, grouped by value, footprint and
///   note, in the order of each kind's first reference: `Comment` the value,
///   `Designator` every reference of the kind, quoted, `Footprint` the KiCad
///   footprint's name without its library, the part number **empty for
///   every part**, since no input names one, and `Note` the emitter's note,
///   so a part with one never shares a row with a part without
///   (`AET-WO-2308` (1));
/// - **CPL**, one row per placed part in KiCad's order: `Mid X` and `Mid Y`
///   KiCad's coordinates with `mm`, `Layer` `T`, and `Rotation` KiCad's,
///   normalised to 0–359, in KiCad's frame with no per-footprint offset.
///
/// Each line ends CRLF. A part on the bottom side refuses both files by its
/// reference. The two files must name exactly the same parts.
pub fn jlcpcb(bom: &[u8], pos: &[u8]) -> Result<(String, String), JlcpcbRefusal> {
    use JlcpcbRefusal::{BottomSide, Malformed};
    let placed = placement(pos).map_err(|e| Malformed(format!("the placement file: {e}")))?;
    let mut cpl = format!("{JLCPCB_CPL_HEADER}\r\n");
    for p in &placed {
        let layer = match p.side {
            "top" => "T",
            "bottom" => return Err(BottomSide(p.reference.to_owned())),
            other => return Err(Malformed(format!("`{}` is on side `{other}`", p.reference))),
        };
        let (Some(x), Some(y)) = (coordinate(p.x), coordinate(p.y)) else {
            return Err(Malformed(format!(
                "`{}` is at ({}, {}), not two decimals",
                p.reference, p.x, p.y
            )));
        };
        let r = rotation(p.rotation).ok_or_else(|| {
            Malformed(format!(
                "`{}`'s rotation `{}` is not a decimal",
                p.reference, p.rotation
            ))
        })?;
        cpl.push_str(&format!(
            "{},{x}mm,{y}mm,{layer},{r}\r\n",
            csv_field(p.reference, true)
        ));
    }
    let rows = bom_rows(bom).map_err(Malformed)?;
    let mut kinds: Vec<((String, String, String), Vec<String>)> = Vec::new();
    for [reference, value, footprint, note] in rows {
        let name = footprint
            .split_once(':')
            .map_or(footprint.as_str(), |(_, n)| n)
            .to_owned();
        let key = (value, name, note);
        match kinds.iter_mut().find(|(k, _)| *k == key) {
            Some((_, refs)) => refs.push(reference),
            None => kinds.push((key, vec![reference])),
        }
    }
    for (_, refs) in &mut kinds {
        refs.sort_by(|a, b| natural(a).cmp(&natural(b)));
    }
    kinds.sort_by(|a, b| natural(&a.1[0]).cmp(&natural(&b.1[0])));
    let mut in_bom: Vec<&str> = kinds
        .iter()
        .flat_map(|(_, r)| r.iter().map(String::as_str))
        .collect();
    let mut in_cpl: Vec<&str> = placed.iter().map(|p| p.reference).collect();
    in_bom.sort_unstable();
    in_cpl.sort_unstable();
    if in_bom != in_cpl {
        return Err(Malformed(format!(
            "the BOM names {in_bom:?} and the placement file {in_cpl:?}"
        )));
    }
    let mut out = format!("{JLCPCB_BOM_HEADER}\r\n");
    for ((value, footprint, note), refs) in &kinds {
        out.push_str(&format!(
            "{},{},{},,{}\r\n",
            csv_field(value, false),
            csv_field(&refs.join(","), true),
            csv_field(footprint, false),
            csv_field(note, false),
        ));
    }
    Ok((out, cpl))
}

/// The notes' section on the JLCPCB files: their sources, what each column
/// is from, and what no input of this engine can fill.
pub fn jlcpcb_notes(project: &str) -> String {
    format!(
        "
## JLCPCB assembly files

`{project}-jlcpcb-bom.csv` and `{project}-jlcpcb-cpl.csv` are the BOM and the
placement file in JLCPCB's assembly upload formats, laid out as JLCPCB's
sample files `Sample-BOM_JLCSMT.xlsx` and `Sample-CPL_JLCSMT.xlsx` lay them
out, as read on 2026-11-17 from:

- https://jlcpcb.com/help/article/bill-of-materials-for-pcb-assembly (last updated 2026-09-09);
- https://jlcpcb.com/help/article/pick-place-file-for-pcb-assembly (last updated 2026-09-09);
- https://jlcpcb.com/help/article/how-to-generate-the-bom-and-centroid-file-from-kicad (last updated 2026-11-10).

They are made from `{project}.bom.csv` and `{project}-pos.csv` and add nothing
to them:

- The BOM has one row per kind of part, grouped by value, footprint and
  note. `Comment` is the value, `Designator` every reference of the kind, and
  `Footprint` the KiCad footprint's name without its library.
- **`Note` is `{project}.bom.csv`'s note on the part, and a fifth column:
  JLCPCB's sample has four.** A part whose symbol stands in for the ordered
  part, or whose value its datasheet does not state, carries one, and never
  shares a row with a part that does not. Every such part is listed in the
  section on parts with a note, below.
- **`JLCPCB Part #（optional）` is empty for every part.** No input of this
  engine names a fabricator's part number, so none is written. Choose each
  part in JLCPCB's library before ordering. A passive's value alone (`100n`)
  does not choose a part: its voltage rating, dielectric and tolerance are
  in no input.
- `Mid X` and `Mid Y` are KiCad's placement coordinates, in mm: each
  footprint's anchor, in the Gerbers' own frame. `Layer` is `T`.
- **`Rotation` is KiCad's, normalised to 0 to 359 degrees, counter-clockwise,
  in KiCad's frame.** No per-footprint offset into JLCPCB's library
  orientation is applied, because none is known. Check every part's
  orientation in JLCPCB's placement preview, by eye, before ordering.
- Test pads are in neither file: nothing is placed on them.
- A part on the bottom side is never written: JLCPCB's pages state no
  bottom-side convention, so `verify --fab-out` refuses such a board by name.
"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_placement_row_with_a_quote_inside_a_field_is_refused() {
        let ok = "Ref,Val,Package,PosX,PosY,Rot,Side\n\"U4\",\"W25Q64JVSS\",\"SOIC-8\",82.55,-108.4,0.0,top\n";
        assert_eq!(placement(ok.as_bytes()).map(|r| r.len()), Ok(1));
        let comma = ok.replace("W25Q64JVSS", "A,B");
        assert_eq!(placement(comma.as_bytes()).map(|r| r[0].value), Ok("A,B"));
        for bad in [
            ok.replace("\"U4\"", "\"U\"4,X\""),
            ok.replace("\"U4\"", "U4"),
            ok.replace(",top", ",top,x"),
            ok.replace(",top", ""),
            ok.replace(",0.0,", ",\"0\","),
            ok.replace("Ref,", "Reference,"),
        ] {
            assert!(placement(bad.as_bytes()).is_err(), "{bad}");
        }
    }

    const POS_HEAD: &str = "Ref,Val,Package,PosX,PosY,Rot,Side\n";
    const BOM_HEAD: &str = "Reference,Value,Footprint,Symbol,Note\r\n";

    fn pos_row(r: &str, rot: &str, side: &str) -> String {
        format!("\"{r}\",\"1k\",\"R_0402_1005Metric\",1.500000,-2.250000,{rot},{side}\n")
    }

    #[test]
    fn rotation_is_kicads_normalised_to_a_turn() {
        for (kicad, want) in [
            ("0.000000", "0"),
            ("-90.000000", "270"),
            ("-180.000000", "180"),
            ("360.000000", "0"),
            ("-0.500000", "359.5"),
            ("45.250000", "45.25"),
            ("720", "0"),
        ] {
            assert_eq!(rotation(kicad).as_deref(), Some(want), "{kicad}");
        }
        for bad in ["", "-", "1e3", "90.0000001", "+90", "9 0", ".5"] {
            assert_eq!(rotation(bad), None, "{bad}");
        }
    }

    #[test]
    fn the_bom_groups_by_value_footprint_and_note_in_natural_order_and_quotes_designators() {
        let bom = format!(
            "{BOM_HEAD}C10,100n,Capacitor_SMD:C_0402_1005Metric,Device:C,\r\nC2,100n,Capacitor_SMD:C_0402_1005Metric,Device:C,\r\n\"A,\"\"B\",\"1k, 1%\",Resistor_SMD:R_0402_1005Metric,Device:R,note\r\nC3,1u,Capacitor_SMD:C_0402_1005Metric,Device:C,\r\n"
        );
        let pos = format!(
            "{POS_HEAD}{}{}{}{}",
            pos_row("C10", "0", "top"),
            pos_row("C2", "0", "top"),
            pos_row("A,\"B", "0", "top").replace("\\\"", "\""),
            pos_row("C3", "0", "top")
        );
        // KiCad would not quote-escape `A,"B`; that row fails `placement`.
        assert!(matches!(
            jlcpcb(bom.as_bytes(), pos.as_bytes()),
            Err(JlcpcbRefusal::Malformed(_))
        ));
        let pos = format!(
            "{POS_HEAD}{}{}{}{}",
            pos_row("C10", "0", "top"),
            pos_row("C2", "0", "top"),
            pos_row("C3", "0", "top"),
            pos_row("C4", "0", "top")
        );
        // C4 is C2's and C10's kind but for its note (`AET-WO-2308` (1)):
        // it keeps a row of its own, its note quoted as RFC 4180 quotes it.
        let bom = format!(
            "{BOM_HEAD}C10,100n,Capacitor_SMD:C_0402_1005Metric,Device:C,\r\nC2,100n,Capacitor_SMD:C_0402_1005Metric,Device:C,\r\nC3,\"1u, X7R\",Capacitor_SMD:C_0402_1005Metric,Device:C,\r\nC4,100n,Capacitor_SMD:C_0402_1005Metric,Device:C,\"stand-in, not a design: \"\"A\"\"\"\r\n"
        );
        let (b, c) = jlcpcb(bom.as_bytes(), pos.as_bytes()).unwrap();
        assert_eq!(
            b,
            "Comment,Designator,Footprint,JLCPCB Part #（optional）,Note\r\n100n,\"C2,C10\",C_0402_1005Metric,,\r\n\"1u, X7R\",\"C3\",C_0402_1005Metric,,\r\n100n,\"C4\",C_0402_1005Metric,,\"stand-in, not a design: \"\"A\"\"\"\r\n"
        );
        assert_eq!(
            c,
            "Designator,Mid X,Mid Y,Layer,Rotation\r\n\"C10\",1.500000mm,-2.250000mm,T,0\r\n\"C2\",1.500000mm,-2.250000mm,T,0\r\n\"C3\",1.500000mm,-2.250000mm,T,0\r\n\"C4\",1.500000mm,-2.250000mm,T,0\r\n"
        );
        assert_eq!(
            noted(bom.as_bytes()),
            Ok(vec![[
                "C4".to_owned(),
                "100n".to_owned(),
                "stand-in, not a design: \"A\"".to_owned()
            ]])
        );
    }

    /// `AET-WO-2308` (1): every noted part, in the references' natural order
    /// (`C2` before `C10`), and none without a note; a BOM the emitter did
    /// not write is refused as `jlcpcb` refuses it.
    #[test]
    fn noted_lists_every_part_with_a_note_in_natural_order() {
        let bom = format!(
            "{BOM_HEAD}C10,100n,Capacitor_SMD:C_0402_1005Metric,Device:C,b\r\nC3,1u,Capacitor_SMD:C_0402_1005Metric,Device:C,\r\nC2,100n,Capacitor_SMD:C_0402_1005Metric,Device:C,a\r\n"
        );
        let refs: Vec<String> = noted(bom.as_bytes())
            .unwrap()
            .into_iter()
            .map(|[r, _, n]| format!("{r}:{n}"))
            .collect();
        assert_eq!(refs, ["C2:a", "C10:b"]);
        assert_eq!(noted(format!("{BOM_HEAD}").as_bytes()), Ok(Vec::new()));
        assert!(noted(b"Reference,Value\r\n").is_err());
    }

    #[test]
    fn a_bottom_side_part_is_refused_by_name_and_nothing_is_written() {
        let pos = format!(
            "{POS_HEAD}{}{}",
            pos_row("R1", "0", "top"),
            pos_row("R2", "90", "bottom")
        );
        let bom = format!(
            "{BOM_HEAD}R1,1k,Resistor_SMD:R_0402_1005Metric,Device:R,\r\nR2,1k,Resistor_SMD:R_0402_1005Metric,Device:R,\r\n"
        );
        assert_eq!(
            jlcpcb(bom.as_bytes(), pos.as_bytes()),
            Err(JlcpcbRefusal::BottomSide("R2".into()))
        );
    }

    #[test]
    fn the_two_files_must_name_the_same_parts_and_be_what_their_writers_write() {
        let pos = format!("{POS_HEAD}{}", pos_row("R1", "0", "top"));
        let bom = |rows: &str| format!("{BOM_HEAD}{rows}");
        let r1 = "R1,1k,Resistor_SMD:R_0402_1005Metric,Device:R,\r\n";
        let r2 = "R2,1k,Resistor_SMD:R_0402_1005Metric,Device:R,\r\n";
        assert!(jlcpcb(bom(r1).as_bytes(), pos.as_bytes()).is_ok());
        for (b, p) in [
            (bom(&format!("{r1}{r2}")), pos.clone()),
            (bom(""), pos.clone()),
            (bom(r1).replace("\r\n", "\n"), pos.clone()),
            (bom("R1,1k,F,S\r\n"), pos.clone()),
            (bom("R1,\"1k\"x,F,S,\r\n"), pos.clone()),
            (bom(r1).replace("Reference", "Ref"), pos.clone()),
            (bom(r1), pos.replace("top", "both")),
            (bom(r1), pos.replace("1.500000", "1,5")),
            (bom(r1), pos.replace("-2.250000", "x")),
        ] {
            assert!(
                matches!(
                    jlcpcb(b.as_bytes(), p.as_bytes()),
                    Err(JlcpcbRefusal::Malformed(_))
                ),
                "{b:?} {p:?}"
            );
        }
    }

    #[test]
    fn the_notes_section_cites_the_pages_and_names_what_is_not_filled() {
        let n = jlcpcb_notes("p");
        for want in [
            "`p-jlcpcb-bom.csv`",
            "`p-jlcpcb-cpl.csv`",
            "https://jlcpcb.com/help/article/bill-of-materials-for-pcb-assembly",
            "https://jlcpcb.com/help/article/pick-place-file-for-pcb-assembly",
            "https://jlcpcb.com/help/article/how-to-generate-the-bom-and-centroid-file-from-kicad",
            "is empty for every part",
            "`Note` is `p.bom.csv`'s note on the part, and a fifth column",
            "the\n  section on parts with a note, below",
            "in KiCad's frame",
            "by eye",
            "bottom side",
        ] {
            assert!(n.contains(want), "{want}");
        }
    }

    fn sample() -> String {
        sample_on(&crate::stack::reference())
    }

    /// `AET-WO-4300d2`: a board with a thermal array and a power tree's
    /// conditions states both; a board with neither keeps its bytes (the
    /// goldens in `cli/src/fab.rs` pin that).
    #[test]
    fn the_notes_state_a_thermal_array_and_every_power_condition() {
        let mut stack = crate::stack::reference();
        stack.thermal.push(crate::stack::ThermalPad {
            reference: "U10".into(),
            pad: "9".into(),
            columns: 3,
            rows: 4,
            pitch_nm: 900_000,
            drill_nm: 254_000,
            via_nm: 554_000,
            layers: vec!["B.Cu".into()],
        });
        let conditions = ["LAYOUT: one".to_owned(), "CLASSES: two".to_owned()];
        let text = notes(&Notes {
            project: "p",
            engine_version: "0.1.0",
            specification_sha256: "ab",
            kicad_version: "10.0.7",
            files: &[],
            stack: &stack,
            conditions: &stated(),
            lifecycle: &[],
            drills: &[routed(DRILL_NM, VIA_NM, 64)],
            power: &conditions,
            support: &[],
            acknowledged: &[],
        });
        assert!(
            text.contains("- Thermal vias under U10 pad 9: 3 x 4, 0.554 mm pad, 0.254 mm drill, 0.9 mm pitch.\n"),
            "{text}"
        );
        assert!(text.contains("## Power tree\n"), "{text}");
        for c in conditions {
            assert!(text.contains(&format!("- {c}\n")), "{text}");
        }
        assert!(!sample().contains("## Power tree"));
    }

    fn sample_on(stack: &Stack) -> String {
        sample_with(stack, &[])
    }

    fn sample_with(stack: &Stack, lifecycle: &[String]) -> String {
        let files = [("p-F_Cu.gtl".to_owned(), "F.Cu")];
        notes(&Notes {
            project: "p",
            engine_version: "0.1.0",
            specification_sha256: "ab",
            kicad_version: "10.0.7",
            files: &files,
            stack,
            conditions: &stated(),
            lifecycle,
            drills: &[routed(DRILL_NM, VIA_NM, 64)],
            power: &[],
            support: &[],
            acknowledged: &[],
        })
    }

    /// The conditions as the reference's `extract` verdict states them.
    fn stated() -> Vec<String> {
        let mut c = crate::extract::conditions(&crate::stack::reference());
        c.push(crate::extract::SPI_CONDITION.to_owned());
        c
    }

    /// `AET-WO-4300f` F2: every condition the `extract` verdict states,
    /// `SPI_CONDITION` among them, is in the notes, in its order.
    #[test]
    fn the_notes_state_every_condition_the_extract_verdict_states() {
        let text = sample();
        let listed: String = stated().iter().map(|c| format!("- {c}\n")).collect();
        assert!(text.contains(&listed), "{text}");
    }

    /// `AET-WO-4210` F-B3: a lifecycle line the verdict states is in the
    /// package's notes; with none, the notes have no such section.
    #[test]
    fn the_notes_carry_the_verdicts_lifecycle_lines() {
        let line = "LIFECYCLE EOL: W25Q64JV (U4) is end of life by its vendor".to_owned();
        let text = sample_with(&crate::stack::reference(), std::slice::from_ref(&line));
        assert!(text.contains(&format!("## Lifecycle\n\nThe verdict that names this package states, for the board's parts:\n\n- {line}\n\n## Acknowledged ledger rows")), "{text}");
        assert!(!sample().contains("Lifecycle"));
    }

    fn notes_with(stack: &Stack, drills: &[Drill]) -> String {
        let files = [("p-F_Cu.gtl".to_owned(), "F.Cu")];
        notes(&Notes {
            project: "p",
            engine_version: "0.1.0",
            specification_sha256: "ab",
            kicad_version: "10.0.7",
            files: &files,
            stack,
            conditions: &stated(),
            lifecycle: &[],
            drills,
            power: &[],
            support: &[],
            acknowledged: &[],
        })
    }

    /// `AET-WO-3402` §1.2: each acknowledged row is one line carrying its
    /// register, class, fields and reason exactly as written; none says so.
    #[test]
    fn the_acknowledged_rows_are_written_as_the_specification_states_them() {
        let none = acknowledged_section(&[]);
        assert!(
            none.starts_with("## Acknowledged ledger rows\n\n"),
            "{none}"
        );
        assert!(
            none.contains("- None: the specification acknowledges no unwitnessed ledger row.\n"),
            "{none}"
        );
        let rows = [
            aethelgard_ir::UnwitnessedAcknowledgement {
                register: "SPI1.SR".into(),
                class: "ignored-by-model".into(),
                fields: vec!["MODF".into()],
                reason: "MODF is a tag in Renode.".into(),
            },
            aethelgard_ir::UnwitnessedAcknowledgement {
                register: "GPIOB.MODER".into(),
                class: "same-source".into(),
                fields: vec!["MODER6".into(), "MODER7".into()],
                reason: "PB6/PB7 alternate.".into(),
            },
        ];
        let text = acknowledged_section(&rows);
        assert!(
            text.contains(
                "- `SPI1.SR` (ignored-by-model): `MODF`. MODF is a tag in Renode.\n- `GPIOB.MODER` (same-source): `MODER6`, `MODER7`. PB6/PB7 alternate.\n"
            ),
            "{text}"
        );
        assert!(!text.contains("- None:"), "{text}");
    }

    fn routed(drill_nm: i64, via_nm: i64, n: usize) -> Drill {
        Drill {
            drill_nm,
            hole: Hole::Via,
            what: vec![(Drilled::Routed { via_nm }, n)],
            at: (0..n as i64).map(|i| (i * 1_000_000, 0)).collect(),
        }
    }

    fn pad(number: &str) -> crate::route::Pad {
        crate::route::Pad {
            number: number.to_owned(),
            net: None,
            rect: crate::pcb::Rect {
                x0: 0,
                y0: 0,
                x1: 1_000_000,
                y1: 1_000_000,
            },
            corner_nm: 0,
            drill_nm: None,
            join: None,
        }
    }

    /// A board of `pads` (reference, number, drill) with `vias` routed on
    /// `nets` and `arrays` read under declared pads.
    fn board(
        stack: Stack,
        pads: &[(&str, &str, Option<i64>)],
        vias: &[&str],
        arrays: Vec<crate::route::Array>,
    ) -> (Board, Routed) {
        let mut parts: Vec<crate::route::Part> = Vec::new();
        for &(reference, number, drill) in pads {
            let p = crate::route::Pad {
                drill_nm: drill,
                ..pad(number)
            };
            match parts.iter_mut().find(|q| q.reference == reference) {
                Some(q) => q.pads.push(p),
                None => parts.push(crate::route::Part {
                    reference: reference.to_owned(),
                    rotation: 0,
                    pads: vec![p],
                    courtyard: Vec::new(),
                    keepouts: Vec::new(),
                }),
            }
        }
        let routed = Routed {
            vias: vias
                .iter()
                .enumerate()
                .map(|(i, n)| crate::route::Via {
                    net: (*n).to_owned(),
                    at: (i as i64, 0),
                })
                .collect(),
            arrays,
            ..Routed::default()
        };
        let outline = crate::pcb::Rect {
            x0: 0,
            y0: 0,
            x1: 1,
            y1: 1,
        };
        (
            Board {
                outline,
                parts,
                stack,
            },
            routed,
        )
    }

    fn array(drill_nm: i64, n: usize) -> crate::route::Array {
        crate::route::Array {
            reference: "U5".into(),
            pad: "9".into(),
            net: "GND".into(),
            via_nm: 554_000,
            drill_nm,
            at: (0..n as i64).map(|i| (i, 0)).collect(),
        }
    }

    fn thermal_stack() -> Stack {
        let mut st = crate::stack::reference();
        st.thermal = vec![crate::stack::ThermalPad {
            reference: "U5".into(),
            pad: "9".into(),
            columns: 3,
            rows: 4,
            pitch_nm: 1_000_000,
            drill_nm: 254_000,
            via_nm: 554_000,
            layers: vec!["B.Cu".into()],
        }];
        st
    }

    /// `AET-WO-4300d2` §1.6, in KiCad 10.0.7's measured order: ascending
    /// diameter; at one diameter the vias' tool, then the pads'; routed and
    /// array vias of one drill in one tool; a pad with no number counted.
    #[test]
    fn the_tools_are_the_judged_boards_holes_in_kicads_order() {
        let (b, r) = board(
            thermal_stack(),
            &[
                ("J1", "", Some(1_000_000)),
                ("J1", "", Some(1_000_000)),
                ("J1", "1", Some(1_900_000)),
                ("J1", "2", Some(1_900_000)),
                ("U6", "1", Some(300_000)),
                ("R1", "1", None),
            ],
            &["GND", "GND", "VBAT"],
            vec![array(254_000, 12)],
        );
        let d = drills(&b, &r).unwrap();
        let lines: Vec<(String, usize)> = d
            .iter()
            .enumerate()
            .map(|(i, t)| (t.tool_line(i + 1), t.hits()))
            .collect();
        assert_eq!(
            lines,
            [
                ("T1C0.254".to_owned(), 12),
                ("T2C0.300".to_owned(), 3),
                ("T3C0.300".to_owned(), 1),
                ("T4C1.000".to_owned(), 2),
                ("T5C1.900".to_owned(), 2),
            ]
        );
        assert_eq!(d[3].named(), "through-hole pads of J1 with no number (2)");
        assert_eq!(
            d[4].named(),
            "through-hole pad 1 of J1 (1), through-hole pad 2 of J1 (1)"
        );
        // An array of the same drill as the routed vias shares their tool.
        let mut st = thermal_stack();
        st.thermal[0].drill_nm = DRILL_NM;
        let (b, r) = board(st, &[], &["GND"], vec![array(DRILL_NM, 12)]);
        let d = drills(&b, &r).unwrap();
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].hits(), 13);
    }

    /// A routed via is at its net class's declared drill; an array's vias
    /// at its rule's drill, whatever the board's array says, so an array
    /// drilled otherwise disagrees with KiCad's file.
    /// `AET-WO-1311` adversarial L: a plated pad of an odd width in
    /// nanometres is drilled at its centre, which `route::read` keeps exact.
    #[test]
    fn a_pad_of_an_odd_width_is_drilled_at_its_centre() {
        let part = crate::route::part_of(
            &crate::sexpr::parse(
                "(footprint \"x\" (at 10 20) (property \"Reference\" \"J1\") (pad \"1\" thru_hole circle (at 0 0) (size 1.000001 1.000001) (drill 0.5) (layers \"*.Cu\")) (fp_rect (start -1 -1) (end 1 1) (layer \"F.CrtYd\")))",
            )
            .unwrap(),
        )
        .unwrap();
        let b = Board {
            outline: crate::pcb::Rect {
                x0: 0,
                y0: 0,
                x1: 40_000_000,
                y1: 40_000_000,
            },
            parts: vec![part],
            stack: crate::stack::reference(),
        };
        let d = drills(&b, &Routed::default()).unwrap();
        assert_eq!(d[0].at, [(10_000_000, 20_000_000)]);
    }

    #[test]
    fn each_via_is_counted_at_its_rules_drill() {
        let mut st = crate::stack::reference();
        st.classes = vec![crate::stack::Class {
            name: "power".into(),
            rule: crate::stack::Rule {
                width_nm: 500_000,
                via_nm: 800_000,
                drill_nm: 400_000,
            },
            via_declared: true,
            nets: vec!["VBAT".into()],
        }];
        let (b, r) = board(st, &[], &["GND", "VBAT", "VBAT"], vec![]);
        let mut want = [routed(DRILL_NM, VIA_NM, 1), routed(400_000, 800_000, 2)];
        // Each via at its own position (`board` lays via `i` at `(i, 0)`).
        want[1].at = vec![(1, 0), (2, 0)];
        assert_eq!(drills(&b, &r).unwrap(), want);
        let (b, r) = board(thermal_stack(), &[], &[], vec![array(300_000, 12)]);
        assert_eq!(drills(&b, &r).unwrap()[0].drill_nm, 254_000);
    }

    #[test]
    fn a_hole_the_drill_file_cannot_state_is_refused() {
        let (b, r) = board(
            crate::stack::reference(),
            &[("J1", "1", Some(1_000_500))],
            &[],
            vec![],
        );
        let e = drills(&b, &r).unwrap_err();
        assert!(e.contains("1000500 nm is not a whole micrometre"), "{e}");
        let (b, r) = board(crate::stack::reference(), &[], &[], vec![array(254_000, 1)]);
        let e = drills(&b, &r).unwrap_err();
        assert!(e.contains("under U5 pad 9 have no declared rule"), "{e}");
    }

    /// Today's boards (only routed vias at the router's drill) keep
    /// `AET-WO-3300`'s sentence; any other board's notes name every tool.
    #[test]
    fn the_holes_section_names_every_tool_once_a_board_has_more_than_vias() {
        let st = crate::stack::reference();
        let today =
            "Every hole is a plated via, drilled 0.3 mm, with a 0.6 mm pad. The\ndrill file";
        assert!(sample().contains(today));
        let (b, r) = board(
            thermal_stack(),
            &[("J1", "1", Some(1_900_000)), ("J1", "2", Some(1_900_000))],
            &["GND", "GND"],
            vec![array(254_000, 12)],
        );
        let text = notes_with(&st, &drills(&b, &r).unwrap());
        assert!(!text.contains("Every hole is a plated via"));
        for want in [
            "Every hole is plated. The drill file has these tools, in its order,\n",
            "- T1: 0.254 mm, plated, 12 holes: the thermal-via array under U5 pad 9, 0.554 mm pad (12).\n",
            "- T2: 0.3 mm, plated, 2 holes: routed vias, 0.6 mm pad (2).\n",
            "- T3: 1.9 mm, plated, 2 holes: through-hole pad 1 of J1 (1), through-hole pad 2 of J1 (1).\n\nThe\ndrill file is Excellon",
        ] {
            assert!(text.contains(want), "missing {want:?} in {text}");
        }
        // A board of vias at a class's other drill is not today's either.
        let other = notes_with(
            &st,
            &[routed(DRILL_NM, VIA_NM, 1), routed(400_000, 800_000, 2)],
        );
        assert!(other.contains("- T2: 0.4 mm, plated, 2 holes: routed vias, 0.8 mm pad (2).\n"));
        // One tool that is not only routed vias at the router's drill: an
        // array merged into it, pads of that drill, or vias of another drill.
        let array = Drilled::Thermal {
            reference: "U5".into(),
            pad: "9".into(),
            via_nm: VIA_NM,
        };
        let pads = Drilled::Pad {
            reference: "U5".into(),
            number: "9".into(),
        };
        for one in [
            Drill {
                what: vec![(Drilled::Routed { via_nm: VIA_NM }, 2), (array, 12)],
                ..routed(DRILL_NM, VIA_NM, 0)
            },
            Drill {
                hole: Hole::Pad,
                what: vec![(pads, 8)],
                ..routed(DRILL_NM, VIA_NM, 0)
            },
            routed(400_000, VIA_NM, 2),
        ] {
            let text = notes_with(&st, std::slice::from_ref(&one));
            assert!(!text.contains("Every hole is a plated via"), "{one:?}");
            assert!(text.contains("- T1: "), "{one:?}");
        }
    }

    /// `AET-WO-1305` F3's conditions, the stackup, the rule minima and the
    /// provenance of the Gerbers, each stated from its constant.
    #[test]
    fn the_notes_state_every_condition_the_bound_rests_on() {
        let text = sample();
        for want in [
            "Finished board from 0.34 mm to 2 mm thick.",
            "Finished copper at most 70 um thick on each layer.",
            "Declared thickness 1.6 mm.",
            "FR-4",
            "most 5.4",
            "Two copper layers: F.Cu (top) and B.Cu (bottom).",
            "Every hole is a plated via, drilled 0.3 mm, with a 0.6 mm pad.",
            "Track width: 0.2 mm.",
            "Copper to copper: 0.2 mm.",
            "KiCad 10.0.7's output",
            "--check-zones",
            "Those two lines\nare removed",
            "metric, with decimal coordinates",
            "`ab`",
            "- `p-F_Cu.gtl`: F.Cu\n",
            "emitted by a passing `aethelgard verify`",
            "The gates judged the design, not these\nfiles.",
            "names every other file of this package by sha256",
            "Emitting this package made\nno board, and nothing in it was tested on hardware.",
        ] {
            assert!(text.contains(want), "missing {want:?}");
        }
        for c in crate::extract::conditions(&crate::stack::reference()) {
            assert!(text.contains(&format!("- {c}\n")), "{c}");
        }
        assert_eq!(text, sample());
    }

    /// `RUL-19`: no word claims measurement, simulation of the board, or
    /// a campaign the verdict may not hold.
    #[test]
    fn the_notes_claim_nothing_beyond_the_verdict() {
        let text = sample().to_lowercase();
        for word in [
            "measured",
            "campaign",
            "simulat",
            "certif",
            "guarantee",
            "verified on",
        ] {
            assert!(!text.contains(word), "{word}");
        }
    }

    #[test]
    fn millimetres_have_no_trailing_zeros() {
        assert_eq!(mm(1_600_000), "1.6");
        assert_eq!(mm(2_000_000), "2");
        assert_eq!(mm(340_000), "0.34");
        assert_eq!(mm(70_000), "0.07");
    }
}
