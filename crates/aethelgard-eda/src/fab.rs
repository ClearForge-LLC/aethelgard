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

use crate::extract::{conditions, permille};
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
/// diameter, what it drills, and how many holes of each kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drill {
    pub drill_nm: i64,
    pub hole: Hole,
    pub what: Vec<(Drilled, usize)>,
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
    let mut tools: BTreeMap<(i64, Hole), BTreeMap<Drilled, usize>> = BTreeMap::new();
    let mut add = |drill_nm: i64, hole: Hole, what: Drilled, n: usize| {
        *tools
            .entry((drill_nm, hole))
            .or_default()
            .entry(what)
            .or_default() += n;
    };
    for v in &routed.vias {
        let rule = board.stack.rule(&v.net);
        add(
            rule.drill_nm,
            Hole::Via,
            Drilled::Routed {
                via_nm: rule.via_nm,
            },
            1,
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
            a.at.len(),
        );
    }
    for part in &board.parts {
        for pad in &part.pads {
            if let Some(d) = pad.drill_nm {
                add(
                    d,
                    Hole::Pad,
                    Drilled::Pad {
                        reference: part.reference.clone(),
                        number: pad.number.clone(),
                    },
                    1,
                );
            }
        }
    }
    let mut out = Vec::new();
    for ((drill_nm, hole), what) in tools {
        if drill_nm <= 0 || drill_nm % 1000 != 0 {
            return Err(format!(
                "a drill of {drill_nm} nm is not a whole micrometre, which KiCad's tool line (three places in mm) cannot state"
            ));
        }
        out.push(Drill {
            drill_nm,
            hole,
            what: what.into_iter().collect(),
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

/// Whether `line` is one round hit, `X<decimal>Y<decimal>`, and nothing
/// else: a slot (`…G85X…`) is not.
fn hit(line: &str) -> bool {
    line.strip_prefix('X')
        .and_then(|r| r.split_once('Y'))
        .is_some_and(|(x, y)| decimal(x) && decimal(y))
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
/// its holes, one round hit per line. A tool missing, an extra tool, a wrong
/// diameter, a hit count off by one, a slot or any line not in that shape is
/// refused, naming the tool and what it drills. A board with no hole is
/// refused: its drill file is not measured.
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
        let mut hits = 0;
        while lines.peek().is_some_and(|l| l.starts_with('X')) {
            let line = lines.next().unwrap_or_default();
            if !hit(line) {
                return Err(format!("under {select}, `{line}` is not one round hit"));
            }
            hits += 1;
        }
        if hits != d.hits() {
            return Err(format!(
                "tool {select} ({} mm) has {hits} hits, where the board holds {}: {}",
                mm(d.drill_nm),
                d.hits(),
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
    /// Whether its connectors declare off-board capacitance, which the
    /// bound adds ([`crate::extract::OFFBOARD`]).
    pub offboard: bool,
    /// The drill file's tools ([`drills`]).
    pub drills: &'a [Drill],
}

/// `nm` in millimetres, with no trailing zeros: 1_600_000 is `1.6`.
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
    let stackup = stackup(st);
    let mut files = String::new();
    for (name, what) in n.files {
        files.push_str(&format!("- `{name}`: {what}\n"));
    }
    let mut conditions = conditions(st);
    if n.offboard {
        conditions.push(crate::extract::OFFBOARD.to_owned());
    }
    let conditions: String = conditions.iter().map(|c| format!("- {c}\n")).collect();
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
{classes}
## Files

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
    )
}

/// The Holes section's first paragraph, ending where "The drill file is
/// Excellon…" continues it. A board whose only holes are routed vias of the
/// router's drill, one tool, keeps `AET-WO-3300`'s sentence, so its notes are
/// unchanged; any other board's names every tool of the drill file, in its
/// order, with its diameter and what it drills (`AET-WO-4300d2` §1.6).
fn holes_text(st: &Stack, drills: &[Drill]) -> String {
    if let [d] = drills
        && d.hole == Hole::Via
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
fn stackup(st: &Stack) -> String {
    if !st.has_planes() {
        return "- Two copper layers: F.Cu (top) and B.Cu (bottom).\n".to_owned();
    }
    let mut out = format!(
        "- {} copper layers, top to bottom, joined only by through vias:\n",
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        sample_on(&crate::stack::reference())
    }

    fn sample_on(stack: &Stack) -> String {
        notes_with(stack, &[routed(DRILL_NM, VIA_NM, 64)])
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
            offboard: false,
            drills,
        })
    }

    fn routed(drill_nm: i64, via_nm: i64, n: usize) -> Drill {
        Drill {
            drill_nm,
            hole: Hole::Via,
            what: vec![(Drilled::Routed { via_nm }, n)],
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
        assert_eq!(
            drills(&b, &r).unwrap(),
            [routed(DRILL_NM, VIA_NM, 1), routed(400_000, 800_000, 2)]
        );
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
        for c in conditions(&crate::stack::reference()) {
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
