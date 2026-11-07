//! S4: the leads for placement and routing (`AET-WO-1301`, `1302`): Specctra
//! DSN/SES in `kicad-cli` 10.0.7, a Python `pcbnew` module (looked for, never
//! imported), whether DRC refills zones, and a Rust grid router's proof of
//! concept judged with `--schematic-parity`.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::pcb::{self, Board, Fp, Rules, Zone};
use super::s3::{PCB_VERSION, SPEC_MM, clearance_count, dru, project, setup};
use super::sch::{self, Conn, GRID, Part, Sheet};
use super::sexpr::Sx;
use super::{Ctx, answer, run as run_cmd, sha256_hex, summary, write};

const MM: i64 = 1_000_000;
const R_FP: &str = "Resistor_SMD:R_0603_1608Metric";

fn help(ctx: &Ctx, args: &[&str]) -> Result<String, String> {
    let ran = ctx.kicad(args, &ctx.work)?;
    let text = format!("{}\n{}", ran.stdout, ran.stderr);
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    answer(
        "S4",
        format!(
            "`kicad-cli {}` exit {}: {}",
            args.join(" "),
            ran.code_text(),
            lines.join(" / ")
        ),
    );
    Ok(text)
}

pub fn run(ctx: &mut Ctx) -> Result<(), String> {
    if ctx.kicad.is_some() {
        leads(ctx)?;
    } else {
        answer(
            "S4",
            "leads 1-3 blocked: kicad-cli absent here (tools image only)",
        );
    }
    let src = match &ctx.libs {
        Some(libs) => libs.footprint(R_FP)?,
        None => {
            answer(
                "S4",
                "footprint: synthetic (no KiCad library here); router mechanics only",
            );
            super::sexpr::parse(super::kilib::SYNTHETIC_FP)?
        }
    };
    zones(ctx, &src)?;
    routed(ctx, &src)?;
    Ok(())
}

fn leads(ctx: &Ctx) -> Result<(), String> {
    let top = help(ctx, &["--help"])?;
    let pcb_help = help(ctx, &["pcb", "--help"])?;
    let export = help(ctx, &["pcb", "export", "--help"])?;
    let drc = help(ctx, &["pcb", "drc", "--help"])?;
    if pcb_help.contains("import") {
        help(ctx, &["pcb", "import", "--help"])?;
    }
    let all = format!("{top}{pcb_help}{export}").to_ascii_lowercase();
    answer(
        "S4",
        format!(
            "lead 1: `dsn` in kicad-cli help={}, `specctra`={}, `ses`={}",
            all.contains("dsn"),
            all.contains("specctra"),
            all.split(|c: char| !c.is_ascii_alphanumeric())
                .any(|w| w == "ses")
        ),
    );
    answer(
        "S4",
        format!(
            "lead 3: pcb drc options mention refill={}, zone={}",
            drc.to_ascii_lowercase().contains("refill"),
            drc.to_ascii_lowercase().contains("zone")
        ),
    );
    let found = run_cmd(
        "find",
        &[
            "/",
            "-xdev",
            "(",
            "-name",
            "pcbnew.py",
            "-o",
            "-name",
            "_pcbnew*.so",
            "-o",
            "-name",
            "pcbnew*.so",
            ")",
            "-print",
        ],
        Path::new("/"),
    )?;
    let hits: Vec<&str> = found.stdout.lines().collect();
    answer(
        "S4",
        format!(
            "lead 2: pcbnew Python module files: {:?} (not imported, RUL-12)",
            hits
        ),
    );
    for tool in ["python3", "python", "java"] {
        let r = run_cmd(
            "sh",
            &["-c", &format!("command -v {tool} || echo absent")],
            Path::new("/"),
        )?;
        answer("S4", format!("lead 2: {tool} on PATH: {}", r.stdout.trim()));
    }
    Ok(())
}

fn fp(src: &Sx, reference: &str, at: (i64, i64), nets: [&str; 2], path: Option<String>) -> Fp {
    Fp {
        lib_id: R_FP.into(),
        reference: reference.into(),
        value: "10k".into(),
        at,
        path,
        sheetfile: "s4.kicad_sch".into(),
        pad_nets: BTreeMap::from([
            ("1".to_owned(), nets[0].to_owned()),
            ("2".to_owned(), nets[1].to_owned()),
        ]),
        src: src.clone(),
        pad_clearance: None,
    }
}

fn board(key: &str, fps: Vec<Fp>) -> Board {
    Board {
        key: key.into(),
        version: PCB_VERSION.into(),
        setup: setup(SPEC_MM),
        outline: (100 * MM, 100 * MM, 125 * MM, 115 * MM),
        footprints: fps,
        segs: Vec::new(),
        vias: Vec::new(),
        zones: Vec::new(),
    }
}

fn gate_files(dir: &Path, stem: &str) -> Result<(), String> {
    write(
        &dir.join(format!("{stem}.kicad_pro")),
        &project(stem, SPEC_MM, ""),
    )?;
    write(
        &dir.join(format!("{stem}.kicad_dru")),
        &dru("gate_spec_clearance", SPEC_MM),
    )
}

/// Lead 3: a GND zone over two GND pads, written with no filled polygon.
fn zones(ctx: &Ctx, src: &Sx) -> Result<(), String> {
    let mut b = board(
        "s4zone",
        vec![fp(src, "R1", (112 * MM, 107 * MM), ["GND", "GND"], None)],
    );
    b.zones.push(Zone {
        net: "GND".into(),
        layer: "F.Cu".into(),
        pts: vec![
            (105 * MM, 104 * MM),
            (120 * MM, 104 * MM),
            (120 * MM, 111 * MM),
            (105 * MM, 111 * MM),
        ],
    });
    let text = b.emit()?;
    answer(
        "S4",
        format!(
            "lead 3: zone board emitted twice identical={} (no filled_polygon written)",
            text == b.emit()?
        ),
    );
    if ctx.kicad.is_none() {
        return Ok(());
    }
    let dir = ctx.dir("s4-zone")?;
    let pcb_path = dir.join("zone.kicad_pcb");
    write(&pcb_path, &text)?;
    gate_files(&dir, "zone")?;
    let drc_help = ctx.kicad(&["pcb", "drc", "--help"], &dir)?.stdout;
    let mut variants: Vec<(&str, Vec<&str>)> = vec![("no flag", vec![])];
    if drc_help.contains("--refill-zones") {
        variants.push(("--refill-zones", vec!["--refill-zones"]));
        if drc_help.contains("--save-board") {
            variants.push((
                "--refill-zones --save-board",
                vec!["--refill-zones", "--save-board"],
            ));
        }
    }
    for (name, flags) in variants {
        write(&pcb_path, &text)?;
        let (ran, by) = ctx.drc(&pcb_path, &flags)?;
        let flat: Vec<_> = by.values().flatten().cloned().collect();
        let after = fs::read_to_string(&pcb_path).unwrap_or_default();
        answer(
            "S4",
            format!(
                "lead 3 [{name}]: exit {}, unconnected {}, findings {}; board file changed={} filled_polygon in file={}",
                ran.code_text(),
                by["unconnected_items"].len(),
                summary(&flat),
                after != text,
                after.contains("filled_polygon")
            ),
        );
        if name.contains("save-board") {
            // Is KiCad's own fill deterministic? Save twice from the same input.
            let first = after;
            write(&pcb_path, &text)?;
            ctx.drc(&pcb_path, &flags)?;
            let second = fs::read_to_string(&pcb_path).unwrap_or_default();
            answer(
                "S4",
                format!(
                    "lead 3: two refills saved identical={} (sha256 {} / {})",
                    first == second,
                    &sha256_hex(first.as_bytes())[..16],
                    &sha256_hex(second.as_bytes())[..16]
                ),
            );
        }
    }
    Ok(())
}

/// The schematic the board must match: R1 and R2, two label nets, crossed so
/// a route needs a detour or the second layer.
fn schematic(
    ctx: &Ctx,
    values: [&str; 2],
) -> Result<Option<(String, BTreeMap<String, Sx>)>, String> {
    let (ids, all) = super::s2::load(ctx)?;
    let mut syms = BTreeMap::new();
    syms.insert("Device:R".to_owned(), all[&ids.r].clone());
    let r = |reference: &str, value: &str, x: i64, one: &str, two: &str| Part {
        lib_id: "Device:R".into(),
        reference: reference.into(),
        value: value.into(),
        footprint: Some(R_FP.into()),
        at: (GRID * x, GRID * 40),
        conns: BTreeMap::from([
            ("1".to_owned(), Conn::Label(one.into())),
            ("2".to_owned(), Conn::Label(two.into())),
        ]),
    };
    let sheet = Sheet {
        project: "s4".into(),
        title: "s4".into(),
        version: super::s2::VERSION.into(),
        paper: "A4".into(),
        parts: vec![
            r("R1", values[0], 40, "A", "B"),
            r("R2", values[1], 60, "B", "A"),
        ],
        flags: Vec::new(),
        flag_lib_id: String::new(),
        loose_labels: Vec::new(),
    };
    Ok(Some((sch::emit(&sheet, &syms)?, syms)))
}

fn routed(ctx: &Ctx, src: &Sx) -> Result<(), String> {
    let Some((sch_text, _)) = schematic(ctx, ["10k", "10k"])? else {
        return Ok(());
    };
    let dir = ctx.dir("s4-route")?;
    let sch_path = dir.join("s4.kicad_sch");
    write(&sch_path, &sch_text)?;
    // The net names KiCad gives the schematic's label nets.
    let mut names = BTreeMap::from([
        ("A".to_owned(), "/A".to_owned()),
        ("B".to_owned(), "/B".to_owned()),
    ]);
    if ctx.kicad.is_some()
        && let Some(net) = super::s2::netlist(ctx, &sch_path)?
    {
        let got: Vec<String> = net
            .child("nets")
            .map(|n| {
                n.children("net")
                    .filter_map(|n| n.child("name").and_then(|x| x.arg(1)).map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        answer(
            "S4",
            format!("parity: the schematic's nets as KiCad names them: {got:?}"),
        );
        for g in got {
            let bare = g.trim_start_matches('/').to_owned();
            if names.contains_key(&bare) {
                names.insert(bare, g);
            }
        }
    }
    let path = |r: &str| Some(format!("/{}", sch::symbol_uuid("s4", r)));
    let rules = Rules {
        width: MM / 4,
        clearance: 200_000,
        via: 600_000,
        drill: 300_000,
        edge: 500_000,
        pitch: 100_000,
    };
    let build = |r2_ref: &str,
                 r2_lib: &str,
                 r2_path: Option<String>,
                 net_a: &str,
                 net_b: &str|
     -> Result<(Board, Vec<String>), String> {
        let mut r1 = fp(src, "R1", (108 * MM, 107 * MM), [net_a, net_b], path("R1"));
        let mut r2 = fp(src, r2_ref, (117 * MM, 107 * MM), [net_b, net_a], r2_path);
        r2.lib_id = r2_lib.into();
        r1.value = "10k".into();
        let mut b = board("s4", vec![r1, r2]);
        let report = pcb::route(&mut b, &[net_a, net_b], &rules)?;
        Ok((b, report))
    };
    let (a, bn) = (names["A"].clone(), names["B"].clone());
    let (good, report) = build("R2", R_FP, path("R2"), &a, &bn)?;
    let text = good.emit()?;
    let (again, _) = build("R2", R_FP, path("R2"), &a, &bn)?;
    answer(
        "S4",
        format!(
            "router: {}; {} segments, {} vias; two emissions identical={}; nets {a:?} {bn:?}",
            report.join("; "),
            good.segs.len(),
            good.vias.len(),
            text == again.emit()?
        ),
    );
    write(&dir.join("s4.kicad_pcb"), &text)?;
    if ctx.kicad.is_none() {
        return Ok(());
    }
    let mut cases: Vec<(&str, String, String)> = vec![("P0 matching", text, sch_text.clone())];
    let wrong_ref = build("R9", R_FP, path("R2"), &a, &bn)?.0.emit()?;
    cases.push(("P1 board reference R9", wrong_ref, sch_text.clone()));
    let wrong_lib = build("R2", "Resistor_SMD:R_0402_1005Metric", path("R2"), &a, &bn)?
        .0
        .emit()?;
    cases.push((
        "P2 footprint lib id 0402 on 0603 copper",
        wrong_lib,
        sch_text.clone(),
    ));
    let bare_a = a.trim_start_matches('/').to_owned();
    let bare_b = bn.trim_start_matches('/').to_owned();
    if bare_a != a {
        let wrong_net = build("R2", R_FP, path("R2"), &bare_a, &bare_b)?.0.emit()?;
        cases.push((
            "P3 board nets without the sheet prefix",
            wrong_net,
            sch_text.clone(),
        ));
    }
    let wrong_path = build(
        "R2",
        R_FP,
        Some(format!("/{}", super::uuid("not-a-symbol"))),
        &a,
        &bn,
    )?
    .0
    .emit()?;
    cases.push(("P4 R2 path names no symbol", wrong_path, sch_text.clone()));
    let (sch_1k, _) = schematic(ctx, ["10k", "1k"])?.ok_or("no schematic")?;
    let good_again = build("R2", R_FP, path("R2"), &a, &bn)?.0.emit()?;
    cases.push((
        "P5 schematic R2 value 1k, board 10k",
        good_again.clone(),
        sch_1k,
    ));
    cases.push(("P6 no .kicad_pro beside the board", good_again, sch_text));
    for (name, board_text, sch_t) in cases {
        let d = ctx.dir(&format!("s4-{}", &name[..2]))?;
        write(&d.join("s4.kicad_pcb"), &board_text)?;
        write(&d.join("s4.kicad_sch"), &sch_t)?;
        if !name.starts_with("P6") {
            gate_files(&d, "s4")?;
        }
        let (ran, by) = ctx.drc(&d.join("s4.kicad_pcb"), &["--schematic-parity"])?;
        answer(
            "S4",
            format!(
                "{name}: exit {}, violations {} (clearance {}), unconnected {}, parity {}",
                ran.code_text(),
                summary(&by["violations"]),
                clearance_count(&by),
                by["unconnected_items"].len(),
                summary(&by["schematic_parity"])
            ),
        );
        for f in by["schematic_parity"]
            .iter()
            .chain(by["violations"].iter())
            .take(4)
        {
            let items: Vec<&str> = f.items.iter().map(|i| i.0.as_str()).collect();
            answer(
                "S4",
                format!(
                    "{name}:   {} {}: {} {:?}",
                    f.kind, f.severity, f.description, items
                ),
            );
        }
        if ran.code != Some(0) && ran.code != Some(5) {
            answer("S4", format!("{name}:   stderr {}", ran.stderr.trim()));
        }
    }
    Ok(())
}
