//! S3: the gate's own rules (`RUL-03`, `INV-07`). A board whose own files
//! carry a 0.05 mm clearance (its setup block, its `.kicad_pro` and its
//! `.kicad_dru`) and two tracks 0.1 mm apart, which the specification's
//! 0.2 mm catches. Every subset of the gate's three steps is measured, so the
//! load-bearing ones are read off the table rather than argued.

use std::collections::BTreeMap;
use std::path::Path;

use super::pcb::{Board, Fp, Seg};
use super::{Ctx, Finding, answer, verdict_summary, write};

pub const PCB_VERSION: &str = "20240108";
pub const SETUP_PLAIN: &str = "(setup (pad_to_mask_clearance 0))";
pub const SPEC_MM: &str = "0.2";
const LOOSE_MM: &str = "0.05";

pub fn setup(clearance: &str) -> String {
    format!(
        "(setup (pad_to_mask_clearance 0) (trace_clearance {clearance}) (clearance_min {clearance}))"
    )
}

/// A project file whose Default net class and board minimum carry `clearance`.
pub fn project(name: &str, clearance: &str, severities: &str) -> String {
    format!(
        r#"{{
  "board": {{
    "design_settings": {{
      "meta": {{ "version": 2 }},
      "rules": {{ "min_clearance": {clearance}, "min_track_width": 0.0 }},
      "rule_severities": {{ {severities} }}
    }}
  }},
  "meta": {{ "filename": "{name}.kicad_pro", "version": 1 }},
  "net_settings": {{
    "classes": [
      {{ "name": "Default", "clearance": {clearance}, "track_width": 0.25, "via_diameter": 0.6, "via_drill": 0.3, "microvia_diameter": 0.3, "microvia_drill": 0.1, "diff_pair_gap": 0.25, "diff_pair_via_gap": 0.25, "diff_pair_width": 0.2, "wire_width": 6, "bus_width": 12, "line_style": 0, "pcb_color": "rgba(0, 0, 0, 0.000)", "schematic_color": "rgba(0, 0, 0, 0.000)", "priority": 2147483647 }}
    ],
    "meta": {{ "version": 3 }}
  }}
}}
"#
    )
}

pub fn dru(name: &str, clearance: &str) -> String {
    format!("(version 1)\n(rule \"{name}\"\n\t(constraint clearance (min {clearance}mm)))\n")
}

const MM: i64 = 1_000_000;

fn tracks_board(setup: String) -> Board {
    // Width 0.25 mm, centres 0.35 mm apart: a 0.1 mm gap.
    let seg = |y: i64, net: &str| Seg {
        a: (105 * MM, y),
        b: (125 * MM, y),
        width: MM / 4,
        layer: "F.Cu".into(),
        net: net.into(),
    };
    Board {
        key: "s3".into(),
        version: PCB_VERSION.into(),
        setup,
        outline: (100 * MM, 100 * MM, 130 * MM, 120 * MM),
        footprints: Vec::new(),
        segs: vec![seg(110 * MM, "NET_A"), seg(110 * MM + 350_000, "NET_B")],
        vias: Vec::new(),
        zones: Vec::new(),
    }
}

pub fn clearance_count(by: &BTreeMap<String, Vec<Finding>>) -> usize {
    by["violations"]
        .iter()
        .filter(|f| f.kind == "clearance")
        .count()
}

fn all(by: &BTreeMap<String, Vec<Finding>>) -> String {
    let flat: Vec<Finding> = by.values().flatten().cloned().collect();
    super::summary(&flat)
}

/// One DRC row: the board text, and the sidecars present in its directory.
fn row(
    ctx: &Ctx,
    name: &str,
    board: &str,
    pro: Option<&str>,
    rules: Option<&str>,
) -> Result<(), String> {
    let dir = ctx.dir(&format!("s3-{name}"))?;
    let pcb = dir.join("input.kicad_pcb");
    write(&pcb, board)?;
    if let Some(p) = pro {
        write(&dir.join("input.kicad_pro"), p)?;
    }
    if let Some(d) = rules {
        write(&dir.join("input.kicad_dru"), d)?;
    }
    let (ran, by) = ctx.drc(&pcb, &[])?;
    answer(
        "S3",
        format!(
            "{name}: exit {}, clearance violations {}, all {}",
            ran.code_text(),
            clearance_count(&by),
            all(&by)
        ),
    );
    for f in by["violations"]
        .iter()
        .filter(|f| f.kind == "clearance")
        .take(1)
    {
        answer("S3", format!("{name}:   {}", f.description));
    }
    if ran.code != Some(0) && ran.code != Some(5) {
        answer("S3", format!("{name}:   stderr {}", ran.stderr.trim()));
    }
    Ok(())
}

pub fn run(ctx: &mut Ctx) -> Result<(), String> {
    let loose_setup = tracks_board(setup(LOOSE_MM)).emit()?;
    let spec_setup = tracks_board(setup(SPEC_MM)).emit()?;
    let plain = tracks_board(SETUP_PLAIN.into()).emit()?;
    let loose_pro = project("input", LOOSE_MM, "");
    let spec_pro = project("input", SPEC_MM, "");
    let loose_dru = dru("board_own_loose", LOOSE_MM);
    let spec_dru = dru("gate_spec_clearance", SPEC_MM);
    let again = tracks_board(setup(LOOSE_MM)).emit()?;
    answer(
        "S3",
        format!(
            "board emitted twice identical={}; gap 0.1 mm between NET_A and NET_B; board's own rules {LOOSE_MM} mm, specification {SPEC_MM} mm",
            loose_setup == again
        ),
    );
    if ctx.kicad.is_none() {
        answer("S3", "blocked: kicad-cli absent here (tools image only)");
        return Ok(());
    }
    // Baselines: what each file alone does.
    row(ctx, "base-plain-setup-no-sidecars", &plain, None, None)?;
    row(
        ctx,
        "base-loose-setup-no-sidecars",
        &loose_setup,
        None,
        None,
    )?;
    row(ctx, "base-spec-setup-no-sidecars", &spec_setup, None, None)?;
    row(
        ctx,
        "base-plain-setup-loose-pro",
        &plain,
        Some(&loose_pro),
        None,
    )?;
    row(
        ctx,
        "base-plain-setup-loose-dru",
        &plain,
        None,
        Some(&loose_dru),
    )?;
    // The matrix: the board's own three files are loose; the gate replaces
    // the ones its applied steps cover.
    for mask in 0..8u8 {
        let (g_setup, g_pro, g_dru) = (mask & 1 != 0, mask & 2 != 0, mask & 4 != 0);
        let name = format!(
            "gate[setup={} pro={} dru={}]",
            u8::from(g_setup),
            u8::from(g_pro),
            u8::from(g_dru)
        );
        row(
            ctx,
            &name.replace(['[', ']', ' ', '='], "_"),
            if g_setup { &spec_setup } else { &loose_setup },
            Some(if g_pro { &spec_pro } else { &loose_pro }),
            Some(if g_dru { &spec_dru } else { &loose_dru }),
        )?;
    }
    // Today's check drc copies only the board; what does it judge by?
    let dir = ctx.dir("s3-check-drc")?;
    let pcb = dir.join("loose.kicad_pcb");
    write(&pcb, &loose_setup)?;
    write(&dir.join("loose.kicad_pro"), &loose_pro)?;
    write(&dir.join("loose.kicad_dru"), &loose_dru)?;
    match ctx.check("drc", "--board", &pcb) {
        Ok((ran, v)) => answer(
            "S3",
            format!(
                "aethelgard check drc on the loose board with its own .kicad_pro and .kicad_dru beside it: exit {}: {}",
                ran.code_text(),
                verdict_summary(&v)
            ),
        ),
        Err(e) => answer("S3", format!("aethelgard check drc: {e}")),
    }
    footprint_rows(ctx, &spec_setup_fn(), &spec_pro, &spec_dru)?;
    Ok(())
}

fn spec_setup_fn() -> String {
    setup(SPEC_MM)
}

/// Scope-informing extra (§5.1's "rules embedded in a footprint"): a pad
/// carrying its own 0.05 mm local clearance, a NET_B track 0.1 mm from it,
/// with every gate step applied.
fn footprint_rows(ctx: &Ctx, setup: &str, pro: &str, rules: &str) -> Result<(), String> {
    let Some(libs) = &ctx.libs else {
        answer("S3", "footprint rows blocked: no footprint library");
        return Ok(());
    };
    let lib_id = "Resistor_SMD:R_0603_1608Metric";
    let src = libs.footprint(lib_id)?;
    for (name, local) in [
        ("fp-pad-default", None),
        ("fp-pad-local-0.05", Some(50_000)),
    ] {
        let fp = Fp {
            lib_id: lib_id.into(),
            reference: "R1".into(),
            value: "1k".into(),
            at: (115 * MM, 112 * MM),
            path: None,
            sheetfile: String::new(),
            pad_nets: BTreeMap::from([
                ("1".to_owned(), "NET_A".to_owned()),
                ("2".to_owned(), "NET_A".to_owned()),
            ]),
            src: src.clone(),
            pad_clearance: local,
        };
        let mut b = Board {
            key: "s3fp".into(),
            version: PCB_VERSION.into(),
            setup: setup.into(),
            outline: (100 * MM, 100 * MM, 130 * MM, 120 * MM),
            footprints: vec![fp],
            segs: Vec::new(),
            vias: Vec::new(),
            zones: Vec::new(),
        };
        let pads = b.pads()?;
        let pad = pads
            .iter()
            .find(|p| p.number == "1" && p.layer == "F.Cu")
            .ok_or("no pad 1")?;
        // Top edge of pad 1, then a 0.1 mm gap, then half the 0.25 mm track.
        let y = pad.centre.1 - pad.half.1 - 100_000 - 125_000;
        b.segs.push(Seg {
            a: (110 * MM, y),
            b: (120 * MM, y),
            width: MM / 4,
            layer: "F.Cu".into(),
            net: "NET_B".into(),
        });
        let text = b.emit()?;
        row(ctx, name, &text, Some(pro), Some(rules))?;
    }
    let _ = Path::new("");
    Ok(())
}
