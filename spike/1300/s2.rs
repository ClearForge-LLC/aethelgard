//! S2: a minimal schematic emitted from Rust (the MCU, one I2C sensor, two
//! pull-ups, decoupling, one rail), judged by ERC in five variants so each
//! finding is tied to what clears it, and emitted twice to show the bytes
//! are identical.

use std::collections::BTreeMap;
use std::fs;

use super::kilib::{self, Libs};
use super::sch::{self, Conn, GRID, Part, Sheet};
use super::sexpr::{self, Sx};
use super::{Ctx, answer, sha256_hex, summary, verdict_summary, write};

pub const VERSION: &str = "20231120";
pub const RAIL: &str = "VDD_3V3";

#[derive(Clone, Copy)]
pub struct Variant {
    pub name: &'static str,
    pub nc: bool,
    pub flags: bool,
    pub vcap_flag: bool,
}

pub const VARIANTS: [Variant; 5] = [
    Variant {
        name: "naive",
        nc: false,
        flags: false,
        vcap_flag: false,
    },
    Variant {
        name: "nc",
        nc: true,
        flags: false,
        vcap_flag: false,
    },
    Variant {
        name: "flags",
        nc: false,
        flags: true,
        vcap_flag: false,
    },
    Variant {
        name: "nc+flags",
        nc: true,
        flags: true,
        vcap_flag: false,
    },
    Variant {
        name: "nc+flags+vcap",
        nc: true,
        flags: true,
        vcap_flag: true,
    },
];

/// The library ids S2 uses, real or synthetic.
pub struct Ids {
    pub mcu: String,
    pub sensor: String,
    pub r: String,
    pub c: String,
    pub rail: String,
    pub gnd: String,
    pub flag: String,
    pub footprints: bool,
}

pub fn load(ctx: &Ctx) -> Result<(Ids, BTreeMap<String, Sx>), String> {
    if let Some(libs) = &ctx.libs {
        let pick = |exact: &str, contains: &str| -> Result<String, String> {
            ctx.found
                .get(exact)
                .cloned()
                .or_else(|| {
                    ctx.found
                        .iter()
                        .find(|(n, _)| n.contains(contains))
                        .map(|(_, id)| id.clone())
                })
                .ok_or_else(|| format!("no symbol like {contains}"))
        };
        let ids = Ids {
            mcu: pick("STM32H753ZITx", "STM32H753ZI")?,
            sensor: pick("BME280", "BME280")?,
            r: "Device:R".into(),
            c: "Device:C".into(),
            rail: "power:+3V3".into(),
            gnd: "power:GND".into(),
            flag: "power:PWR_FLAG".into(),
            footprints: true,
        };
        let syms = load_ids(libs, &ids)?;
        Ok((ids, syms))
    } else {
        let file = sexpr::parse(kilib::SYNTHETIC)?;
        let mut syms = BTreeMap::new();
        for name in ["R", "SENS", "+3V3", "PWR_FLAG"] {
            let id = format!("spike:{name}");
            syms.insert(
                id,
                kilib::embed(&|n: &str| kilib::top_symbol(&file, n), "spike", name)?,
            );
        }
        let ids = Ids {
            mcu: "spike:SENS".into(),
            sensor: "spike:SENS".into(),
            r: "spike:R".into(),
            c: "spike:R".into(),
            rail: "spike:+3V3".into(),
            gnd: "spike:+3V3".into(),
            flag: "spike:PWR_FLAG".into(),
            footprints: false,
        };
        Ok((ids, syms))
    }
}

fn load_ids(libs: &Libs, ids: &Ids) -> Result<BTreeMap<String, Sx>, String> {
    let mut syms = BTreeMap::new();
    for id in [
        &ids.mcu,
        &ids.sensor,
        &ids.r,
        &ids.c,
        &ids.rail,
        &ids.gnd,
        &ids.flag,
    ] {
        syms.insert(id.clone(), libs.symbol(id)?);
    }
    Ok(syms)
}

fn pin_named(sym: &Sx, want: &str) -> Vec<String> {
    kilib::pins(sym)
        .unwrap_or_default()
        .into_iter()
        .filter(|p| p.name.eq_ignore_ascii_case(want))
        .map(|p| p.number)
        .collect()
}

/// The MCU's connection for a pin, by name.
fn mcu_conn(name: &str, ids: &Ids) -> Option<Conn> {
    let n = name.to_ascii_uppercase();
    let is_port_pin = |p: &str| {
        n == p
            || n.strip_prefix(p)
                .is_some_and(|rest| !rest.starts_with(|c: char| c.is_ascii_digit()))
    };
    if n.starts_with("VDD") || n == "VBAT" || n.starts_with("VREF+") || n == "PDR_ON" {
        Some(Conn::Power(ids.rail.clone(), RAIL.into()))
    } else if n.starts_with("VSS") || n.starts_with("VREF-") || n == "BOOT0" {
        Some(Conn::Power(ids.gnd.clone(), "GND".into()))
    } else if n.starts_with("VCAP") {
        Some(Conn::Label("VCAP".into()))
    } else if is_port_pin("PB8") {
        Some(Conn::Label("I2C_SCL".into()))
    } else if is_port_pin("PB9") {
        Some(Conn::Label("I2C_SDA".into()))
    } else {
        None
    }
}

fn sensor_conn(name: &str, ids: &Ids) -> Option<Conn> {
    match name.to_ascii_uppercase().as_str() {
        "VDD" | "VDDIO" | "CSB" => Some(Conn::Power(ids.rail.clone(), RAIL.into())),
        "GND" | "SDO" => Some(Conn::Power(ids.gnd.clone(), "GND".into())),
        "SDI" => Some(Conn::Label("I2C_SDA".into())),
        "SCK" => Some(Conn::Label("I2C_SCL".into())),
        _ => None,
    }
}

fn passive(
    ids: &Ids,
    lib_id: &str,
    reference: &str,
    value: &str,
    fp: &str,
    at: (i64, i64),
    one: Conn,
    two: Conn,
) -> Part {
    Part {
        lib_id: lib_id.to_owned(),
        reference: reference.into(),
        value: value.into(),
        footprint: ids.footprints.then(|| fp.to_owned()),
        at,
        conns: BTreeMap::from([("1".to_owned(), one), ("2".to_owned(), two)]),
    }
}

pub fn sheet(
    ids: &Ids,
    syms: &BTreeMap<String, Sx>,
    v: Variant,
    project: &str,
) -> Result<Sheet, String> {
    let conns = |lib_id: &str, rule: &dyn Fn(&str) -> Option<Conn>| -> Result<_, String> {
        let sym = syms.get(lib_id).ok_or("symbol not loaded")?;
        let mut m = BTreeMap::new();
        for p in kilib::pins(sym)? {
            match rule(&p.name) {
                Some(c) => {
                    m.insert(p.number, c);
                }
                None if v.nc => {
                    m.insert(p.number, Conn::NoConnect);
                }
                None => {}
            }
        }
        Ok(m)
    };
    let rail = || Conn::Power(ids.rail.clone(), RAIL.into());
    let gnd = || Conn::Power(ids.gnd.clone(), "GND".into());
    let label = |n: &str| Conn::Label(n.into());
    let r_fp = "Resistor_SMD:R_0402_1005Metric";
    let c_fp = "Capacitor_SMD:C_0402_1005Metric";
    let mut parts = vec![
        Part {
            lib_id: ids.mcu.clone(),
            reference: "U1".into(),
            value: "STM32H753ZITx".into(),
            footprint: None,
            at: (GRID * 300, GRID * 200),
            conns: conns(&ids.mcu, &|n| mcu_conn(n, ids))?,
        },
        Part {
            lib_id: ids.sensor.clone(),
            reference: "U2".into(),
            value: "BME280".into(),
            footprint: None,
            at: (GRID * 60, GRID * 60),
            conns: conns(&ids.sensor, &|n| sensor_conn(n, ids))?,
        },
        passive(
            ids,
            &ids.r,
            "R1",
            "2k2",
            r_fp,
            (GRID * 40, GRID * 120),
            rail(),
            label("I2C_SDA"),
        ),
        passive(
            ids,
            &ids.r,
            "R2",
            "2k2",
            r_fp,
            (GRID * 60, GRID * 120),
            rail(),
            label("I2C_SCL"),
        ),
        passive(
            ids,
            &ids.c,
            "C1",
            "100n",
            c_fp,
            (GRID * 80, GRID * 120),
            rail(),
            gnd(),
        ),
        passive(
            ids,
            &ids.c,
            "C2",
            "100n",
            c_fp,
            (GRID * 100, GRID * 120),
            rail(),
            gnd(),
        ),
        passive(
            ids,
            &ids.c,
            "C3",
            "2u2",
            c_fp,
            (GRID * 120, GRID * 120),
            label("VCAP"),
            gnd(),
        ),
    ];
    // The synthetic library has no MCU; keep the sheet the same shape.
    if !ids.footprints {
        parts.retain(|p| p.reference != "U1");
    }
    let mut flags = Vec::new();
    if v.flags {
        flags.push((ids.rail.clone(), RAIL.to_owned()));
        flags.push((ids.gnd.clone(), "GND".to_owned()));
    }
    let mut sheet = Sheet {
        project: project.into(),
        title: project.into(),
        version: VERSION.into(),
        paper: "A0".into(),
        parts,
        flags,
        flag_lib_id: ids.flag.clone(),
        loose_labels: Vec::new(),
    };
    if v.vcap_flag {
        // A PWR_FLAG on a label net: a flag symbol whose pin sits on a VCAP
        // label, emitted as a one-pin part.
        sheet.parts.push(Part {
            lib_id: ids.flag.clone(),
            reference: "#FLG100".into(),
            value: "PWR_FLAG".into(),
            footprint: None,
            at: (GRID * 140, GRID * 120),
            conns: BTreeMap::from([("1".to_owned(), Conn::Label("VCAP".into()))]),
        });
    }
    let _ = pin_named;
    Ok(sheet)
}

pub fn run(ctx: &mut Ctx) -> Result<(), String> {
    let (ids, syms) = load(ctx)?;
    if !ids.footprints {
        answer(
            "S2",
            "library: synthetic (no KiCad library here); emission mechanics and determinism only",
        );
    }
    let mcu_pins = kilib::pins(&syms[&ids.mcu])?;
    let mut by_type: BTreeMap<&str, usize> = BTreeMap::new();
    for p in &mcu_pins {
        *by_type.entry(p.etype.as_str()).or_default() += 1;
    }
    let mut points: Vec<(i64, i64)> = mcu_pins.iter().map(|p| p.at).collect();
    points.sort();
    points.dedup();
    answer(
        "S2",
        format!(
            "{}: {} pins at {} points, hidden {}, by type {:?}",
            ids.mcu,
            mcu_pins.len(),
            points.len(),
            mcu_pins.iter().filter(|p| p.hidden).count(),
            by_type
        ),
    );
    for v in VARIANTS {
        let dir = ctx.dir(&format!("s2-{}", v.name.replace('+', "-")))?;
        let s = sheet(&ids, &syms, v, "s2")?;
        let text = sch::emit(&s, &syms)?;
        let path = dir.join("s2.kicad_sch");
        write(&path, &text)?;
        // Determinism: a second emission, with the parts given in reverse.
        let mut s2 = sheet(&ids, &syms, v, "s2")?;
        s2.parts.reverse();
        s2.flags.reverse();
        let again = sch::emit(&s2, &syms)?;
        let nc = text.matches("(no_connect").count();
        answer(
            "S2",
            format!(
                "{}: emitted {} bytes sha256 {} ({} no_connect); second emission (inputs reversed) identical={}",
                v.name,
                text.len(),
                sha256_hex(text.as_bytes()),
                nc,
                text == again
            ),
        );
        if ctx.kicad.is_none() {
            continue;
        }
        let (ran, findings) = ctx.erc(&path)?;
        answer(
            "S2",
            format!(
                "{}: kicad-cli sch erc exit {}, {} ms, findings {}",
                v.name,
                ran.code_text(),
                ran.ms,
                summary(&findings)
            ),
        );
        let mut kinds_shown: BTreeMap<String, usize> = BTreeMap::new();
        for f in &findings {
            let n = kinds_shown.entry(f.kind.clone()).or_default();
            *n += 1;
            if *n <= 2 {
                let items: Vec<&str> = f.items.iter().map(|i| i.0.as_str()).collect();
                answer(
                    "S2",
                    format!(
                        "{}:   e.g. {} {}: {} {:?}",
                        v.name, f.kind, f.severity, f.description, items
                    ),
                );
            }
        }
        let item_uuids: Vec<&str> = findings
            .iter()
            .flat_map(|f| f.items.iter().map(|i| i.1.as_str()))
            .collect();
        let ours = item_uuids.iter().filter(|u| text.contains(**u)).count();
        answer(
            "S2",
            format!(
                "{}: finding item uuids present in the emitted file: {ours} of {}",
                v.name,
                item_uuids.len()
            ),
        );
        if matches!(v.name, "naive" | "nc+flags+vcap" | "nc+flags") {
            match ctx.check("erc", "--schematic", &path) {
                Ok((ran, verdict)) => answer(
                    "S2",
                    format!(
                        "{}: aethelgard check erc exit {}: {}",
                        v.name,
                        ran.code_text(),
                        verdict_summary(&verdict)
                    ),
                ),
                Err(e) => answer("S2", format!("{}: aethelgard check erc: {e}", v.name)),
            }
        }
        if v.name == "nc+flags+vcap" {
            netlist(ctx, &path)?;
            upgrade(ctx, &path, &text)?;
        }
    }
    demo_header(ctx);
    Ok(())
}

/// Net names and component references as KiCad's netlist exporter reads them.
pub fn netlist(ctx: &Ctx, sch: &std::path::Path) -> Result<Option<Sx>, String> {
    let out = sch.with_extension("net");
    let ran = ctx.kicad(
        &[
            "sch",
            "export",
            "netlist",
            "-o",
            &out.to_string_lossy(),
            &sch.to_string_lossy(),
        ],
        sch.parent().unwrap(),
    )?;
    let Ok(text) = fs::read_to_string(&out) else {
        answer(
            "S2",
            format!(
                "netlist export exit {}: no file; {}",
                ran.code_text(),
                ran.stderr.trim()
            ),
        );
        return Ok(None);
    };
    let net = sexpr::parse(&text)?;
    if sch.file_stem().is_some_and(|s| s == "s2") {
        let nets: Vec<String> = net
            .child("nets")
            .map(|n| {
                n.children("net")
                    .map(|n| {
                        format!(
                            "{}({})",
                            n.child("name").and_then(|x| x.arg(1)).unwrap_or("?"),
                            n.children("node").count()
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        let refs: Vec<String> = net
            .child("components")
            .map(|c| {
                c.children("comp")
                    .map(|c| {
                        c.child("ref")
                            .and_then(|r| r.arg(1))
                            .unwrap_or("?")
                            .to_owned()
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut named: Vec<&String> = nets
            .iter()
            .filter(|n| !n.starts_with("unconnected-"))
            .collect();
        named.sort();
        answer(
            "S2",
            format!(
                "netlist: {} nets ({} named, the rest unconnected-*): {:?}; components {:?}",
                nets.len(),
                named.len(),
                named,
                refs
            ),
        );
    }
    Ok(Some(net))
}

/// What KiCad 10 writes when it re-saves the file: its format version, and
/// whether every UUID the emitter chose survives.
fn upgrade(ctx: &Ctx, sch: &std::path::Path, text: &str) -> Result<(), String> {
    let help = ctx.kicad(&["sch", "--help"], sch.parent().unwrap())?;
    answer(
        "S2",
        format!(
            "kicad-cli sch subcommands: {}",
            help.stdout
                .lines()
                .chain(help.stderr.lines())
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
                .join(" / ")
        ),
    );
    if !help.stdout.contains("upgrade") {
        return Ok(());
    }
    let copy = sch.with_file_name("upgraded.kicad_sch");
    fs::copy(sch, &copy).map_err(|e| e.to_string())?;
    let ran = ctx.kicad(
        &["sch", "upgrade", "--force", &copy.to_string_lossy()],
        sch.parent().unwrap(),
    )?;
    let up = fs::read_to_string(&copy).unwrap_or_default();
    let ours: Vec<&str> = text
        .match_indices("(uuid \"")
        .map(|(i, _)| &text[i + 7..i + 43])
        .collect();
    let kept = ours.iter().filter(|u| up.contains(**u)).count();
    let version = up
        .lines()
        .find(|l| l.contains("(version"))
        .unwrap_or("")
        .trim()
        .to_owned();
    answer(
        "S2",
        format!(
            "kicad-cli sch upgrade exit {}: {} -> {} bytes, {version}, identical={}, our uuids kept {kept} of {}; stderr {}",
            ran.code_text(),
            text.len(),
            up.len(),
            up == text,
            ours.len(),
            ran.stderr.trim()
        ),
    );
    Ok(())
}

fn demo_header(ctx: &Ctx) {
    let Some(libs) = &ctx.libs else { return };
    let root = libs.sym_dir.parent().unwrap_or(std::path::Path::new("/"));
    let Ok(found) = super::run(
        "find",
        &[
            &root.to_string_lossy(),
            "-name",
            "*.kicad_sch",
            "-path",
            "*demos*",
        ],
        std::path::Path::new("/"),
    ) else {
        return;
    };
    let mut files: Vec<&str> = found.stdout.lines().collect();
    files.sort();
    answer(
        "S2",
        format!("demo schematics in the image: {}", files.len()),
    );
    if let Some(first) = files.first() {
        let text = fs::read_to_string(first).unwrap_or_default();
        let head: Vec<&str> = text.lines().take(6).map(str::trim).collect();
        answer("S2", format!("demo {first} header: {}", head.join(" ")));
    }
}
