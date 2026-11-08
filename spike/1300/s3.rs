//! S3: the gate's own rules (`RUL-03`, `INV-07`). A board whose own files
//! carry a 0.05 mm clearance (its setup block, its `.kicad_pro` and its
//! `.kicad_dru`) and two tracks 0.1 mm apart, which the specification's
//! 0.2 mm catches. Every subset of the gate's three steps is measured, so the
//! load-bearing ones are read off the table rather than argued.

use std::collections::BTreeMap;

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
    footprint_rows(ctx, &spec_pro, &spec_dru, &loose_pro, &loose_dru)?;
    round3(ctx, &spec_dru, &loose_dru)?;
    Ok(())
}

/// §5.1's "rules embedded in a footprint": a NET_B track 0.1 mm from a pad
/// whose footprint or pad carries a 0.05 mm local clearance, against each
/// gate step alone and both together. Round 1 found the pro's board minimum,
/// not the gate's custom rule, catching a pad-local override.
fn footprint_rows(
    ctx: &Ctx,
    spec_pro: &str,
    spec_dru: &str,
    loose_pro: &str,
    loose_dru: &str,
) -> Result<(), String> {
    let Some(libs) = &ctx.libs else {
        answer("S3", "footprint rows blocked: no footprint library");
        return Ok(());
    };
    let lib_id = "Resistor_SMD:R_0603_1608Metric";
    let src = libs.footprint(lib_id)?;
    let overrides: [(&str, Option<i64>, Option<i64>); 3] = [
        ("fp-pad-default", None, None),
        ("fp-pad-local-0.05", Some(50_000), None),
        ("fp-footprint-local-0.05", None, Some(50_000)),
    ];
    let gates: [(&str, &str, &str); 3] = [
        ("gate-pro+dru", spec_pro, spec_dru),
        ("gate-dru-only", loose_pro, spec_dru),
        ("gate-pro-only", spec_pro, loose_dru),
    ];
    for (name, pad_cl, fp_cl) in overrides {
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
            pad_clearance: pad_cl,
            fp_clearance: fp_cl,
            extra_props: Vec::new(),
        };
        let mut b = Board {
            key: "s3fp".into(),
            version: PCB_VERSION.into(),
            setup: setup(SPEC_MM),
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
        for (g, pro, rules) in gates {
            if name == "fp-pad-default" && g != "gate-pro+dru" {
                continue;
            }
            row(ctx, &format!("{name}-{g}"), &text, Some(pro), Some(rules))?;
        }
    }
    Ok(())
}

/// The first demo project in the image, as KiCad 10 wrote it.
pub fn demo_pro() -> Result<(String, serde_json::Value), String> {
    let found = super::run(
        "find",
        &["/usr/share/kicad/demos", "-name", "*.kicad_pro"],
        std::path::Path::new("/"),
    )?;
    let mut files: Vec<&str> = found.stdout.lines().collect();
    files.sort();
    let first = files.first().ok_or("no demo .kicad_pro")?;
    Ok((
        first.to_string(),
        super::read_json(std::path::Path::new(first))?,
    ))
}

/// A project in the demo's own shape, with the Default net class's
/// clearance, the board minimum, and rule severities set to ignore.
pub fn pro_like_demo(
    demo: &serde_json::Value,
    name: &str,
    netclass: f64,
    min: f64,
    ignore: &[&str],
) -> Result<String, String> {
    let mut v = demo.clone();
    v["meta"]["filename"] = format!("{name}.kicad_pro").into();
    let ds = &mut v["board"]["design_settings"];
    ds["rules"]["min_clearance"] = min.into();
    for k in ignore {
        ds["rule_severities"][*k] = "ignore".into();
    }
    let classes = v["net_settings"]["classes"]
        .as_array_mut()
        .ok_or("demo has no net classes")?;
    classes.retain(|c| c["name"] == "Default");
    let default = classes.first_mut().ok_or("demo has no Default class")?;
    default["clearance"] = netclass.into();
    v["net_settings"]["netclass_assignments"] = serde_json::Value::Null;
    v["net_settings"]["netclass_patterns"] = serde_json::json!([]);
    serde_json::to_string_pretty(&v).map_err(|e| e.to_string())
}

/// The round-1 project with no `meta` in its design settings.
fn project_no_meta(name: &str, netclass: &str, min: &str, severities: &str) -> String {
    project(name, netclass, severities)
        .replace("\"meta\": { \"version\": 2 },\n      ", "")
        .replace(
            &format!("\"min_clearance\": {netclass},"),
            &format!("\"min_clearance\": {min},"),
        )
}

/// Round 3. Round 2's override rows were all caught by "board minimum
/// clearance", even with the board's own project saying 0.05, and its
/// severities project did not silence a warning. So: which project shape
/// does KiCad 10 read the board minimum and rule severities from, and with
/// a shape that is read, which gate step catches a local override?
fn round3(ctx: &Ctx, spec_dru: &str, loose_dru: &str) -> Result<(), String> {
    let (path, demo) = demo_pro()?;
    let ds = &demo["board"]["design_settings"];
    let keys = |v: &serde_json::Value| -> Vec<String> {
        v.as_object()
            .map(|o| o.keys().cloned().collect())
            .unwrap_or_default()
    };
    answer(
        "S3",
        format!(
            "round3: template {path}: top keys {:?}; meta {}; board.design_settings keys {:?}; design_settings.meta {}; rules.min_clearance {}; rule_severities has {} keys; net_settings.meta {}",
            keys(&demo),
            demo["meta"],
            keys(ds),
            ds["meta"],
            ds["rules"]["min_clearance"],
            keys(&ds["rule_severities"]).len(),
            demo["net_settings"]["meta"]
        ),
    );
    // Which shape is read: netclass 0.05 (hides the 0.1 mm gap), board
    // minimum 0.2 (catches it if read), track_dangling ignored (silences
    // the two dangling warnings if read). Plain setup, no .kicad_dru.
    let plain = tracks_board(SETUP_PLAIN.into()).emit()?;
    let sev = "\"track_dangling\": \"ignore\"";
    let shapes = [
        (
            "fmtA-round1",
            project("input", LOOSE_MM, sev).replace(
                &format!("\"min_clearance\": {LOOSE_MM},"),
                &format!("\"min_clearance\": {SPEC_MM},"),
            ),
        ),
        (
            "fmtB-no-meta",
            project_no_meta("input", LOOSE_MM, SPEC_MM, sev),
        ),
        (
            "fmtD-demo",
            pro_like_demo(&demo, "input", 0.05, 0.2, &["track_dangling"])?,
        ),
    ];
    for (name, pro) in &shapes {
        row(ctx, &format!("r3-read-{name}"), &plain, Some(pro), None)?;
    }
    // Local overrides against each gate step, with a project shape that the
    // rows above show is read (fmtD) and with round 1's (fmtA).
    let Some(libs) = &ctx.libs else { return Ok(()) };
    let src = libs.footprint("Resistor_SMD:R_0603_1608Metric")?;
    let pros: [(&str, String, String); 2] = [
        (
            "fmtD",
            pro_like_demo(&demo, "input", 0.2, 0.2, &[])?,
            pro_like_demo(&demo, "input", 0.05, 0.05, &[])?,
        ),
        (
            "fmtA",
            project("input", SPEC_MM, ""),
            project("input", LOOSE_MM, ""),
        ),
    ];
    let setups = [
        ("spec", setup(SPEC_MM)),
        ("loose", setup(LOOSE_MM)),
        ("plain", SETUP_PLAIN.to_owned()),
    ];
    for (over, pad_cl, fp_cl) in [
        ("pad-local", Some(50_000), None),
        ("fp-local", None, Some(50_000)),
        ("none", None, None),
    ] {
        for (fmt, gate_pro, loose_pro) in &pros {
            // (setup, gate pro?, gate dru?)
            let combos: [(&str, bool, bool); 7] = [
                ("spec", true, true),
                ("loose", true, true),
                ("plain", true, true),
                ("spec", false, true),
                ("spec", true, false),
                ("loose", false, true),
                ("loose", false, false),
            ];
            for (s, gp, gd) in combos {
                let text = override_board(
                    &src,
                    &setups.iter().find(|x| x.0 == s).unwrap().1,
                    pad_cl,
                    fp_cl,
                )?;
                let name = format!(
                    "r3-{over}-{fmt}-setup_{s}-pro_{}-dru_{}",
                    if gp { "gate" } else { "own" },
                    if gd { "gate" } else { "own" }
                );
                row(
                    ctx,
                    &name,
                    &text,
                    Some(if gp { gate_pro } else { loose_pro }),
                    Some(if gd { spec_dru } else { loose_dru }),
                )?;
            }
        }
    }
    Ok(())
}

/// A NET_B track 0.1 mm from pad 1 of a NET_A R_0603 carrying the given
/// local clearances.
fn override_board(
    src: &super::sexpr::Sx,
    setup: &str,
    pad_cl: Option<i64>,
    fp_cl: Option<i64>,
) -> Result<String, String> {
    let fp = Fp {
        lib_id: "Resistor_SMD:R_0603_1608Metric".into(),
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
        pad_clearance: pad_cl,
        fp_clearance: fp_cl,
        extra_props: Vec::new(),
    };
    let mut b = Board {
        key: "s3r3".into(),
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
    let y = pad.centre.1 - pad.half.1 - 100_000 - 125_000;
    b.segs.push(Seg {
        a: (110 * MM, y),
        b: (120 * MM, y),
        width: MM / 4,
        layer: "F.Cu".into(),
        net: "NET_B".into(),
    });
    b.emit()
}
