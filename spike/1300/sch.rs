//! The spike's schematic emitter. Every symbol sits at rotation 0. A
//! connected pin gets a 2.54 mm wire stub, outward from the body, that ends in
//! a label or a power symbol, so nothing relies on a label touching a pin
//! directly. An unused pin gets a no-connect on its point. Stacked pins (one
//! point, several numbers) are connected once. Every UUID is derived from a
//! content path (`super::uuid`), and every collection is walked in key order,
//! so the text is a function of the `Sheet` and the embedded symbols alone.

use std::collections::BTreeMap;

use super::kilib::{self, Pin};
use super::sexpr::{self, Sx, mm};
use super::uuid;

pub const GRID: i64 = 1_270_000;
const STUB: i64 = 2 * GRID;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Conn {
    Label(String),
    /// A power symbol: its library id and the net name (its Value).
    Power(String, String),
    NoConnect,
}

#[derive(Clone, Debug)]
pub struct Part {
    pub lib_id: String,
    pub reference: String,
    pub value: String,
    /// None keeps the library's Footprint field.
    pub footprint: Option<String>,
    pub at: (i64, i64),
    /// By pin number.
    pub conns: BTreeMap<String, Conn>,
}

#[derive(Clone, Debug, Default)]
pub struct Sheet {
    pub project: String,
    pub title: String,
    pub version: String,
    pub paper: String,
    pub parts: Vec<Part>,
    /// (power symbol lib id, net value): a power symbol and a PWR_FLAG on one point.
    pub flags: Vec<(String, String)>,
    pub flag_lib_id: String,
    /// Labels on their own, connected to nothing: (text, global?).
    pub loose_labels: Vec<(String, bool)>,
}

pub fn symbol_uuid(project: &str, reference: &str) -> String {
    uuid(&format!("{project}/symbol/{reference}"))
}

pub fn root_uuid(project: &str) -> String {
    uuid(&format!("{project}/root"))
}

fn at(x: i64, y: i64, angle: i64) -> Sx {
    Sx::list(vec![
        Sx::atom("at"),
        Sx::atom(mm(x)),
        Sx::atom(mm(y)),
        Sx::atom(angle.to_string()),
    ])
}

fn uuid_sx(key: &str) -> Sx {
    Sx::list(vec![Sx::atom("uuid"), Sx::str(uuid(key))])
}

fn justified(j: &str) -> Sx {
    sexpr::parse(&format!("(effects (font (size 1.27 1.27)) (justify {j}))")).unwrap()
}

/// Outward from the body, in schematic coordinates (Y down), from the pin's
/// library angle (the direction from its point into the body, Y up).
fn outward(angle: i64) -> Result<(i64, i64), String> {
    match angle.rem_euclid(360) {
        0 => Ok((-1, 0)),
        90 => Ok((0, 1)),
        180 => Ok((1, 0)),
        270 => Ok((0, -1)),
        a => Err(format!("pin angle {a} is not a multiple of 90")),
    }
}

/// A label's angle and justification for a stub that runs in `dir`.
fn label_dir(dir: (i64, i64)) -> (i64, &'static str) {
    match dir {
        (1, 0) => (0, "left bottom"),
        (-1, 0) => (180, "right bottom"),
        (0, -1) => (90, "left bottom"),
        _ => (270, "right bottom"),
    }
}

struct Emitter<'a> {
    sheet: &'a Sheet,
    syms: &'a BTreeMap<String, Sx>,
    root: String,
    items: Vec<Sx>,
    power_count: usize,
    flag_count: usize,
}

impl Emitter<'_> {
    fn sym(&self, lib_id: &str) -> Result<&Sx, String> {
        self.syms
            .get(lib_id)
            .ok_or_else(|| format!("symbol `{lib_id}` not loaded"))
    }

    fn instance(
        &mut self,
        lib_id: &str,
        reference: &str,
        value: &str,
        footprint: Option<&str>,
        pos: (i64, i64),
        key: &str,
    ) -> Result<(), String> {
        let sym = self.sym(lib_id)?.clone();
        let project = &self.sheet.project;
        let mut v = vec![
            Sx::atom("symbol"),
            Sx::list(vec![Sx::atom("lib_id"), Sx::str(lib_id)]),
            at(pos.0, pos.1, 0),
            Sx::node("unit", &["1"]),
        ];
        for flag in ["exclude_from_sim", "in_bom", "on_board"] {
            let val = sym.child(flag).and_then(|f| f.arg(1)).unwrap_or("yes");
            v.push(Sx::node(flag, &[val]));
        }
        v.push(Sx::node("dnp", &["no"]));
        v.push(uuid_sx(key));
        for prop in sym.children("property") {
            let name = prop.arg(1).ok_or("property without name")?;
            let text = match name {
                "Reference" => reference.to_owned(),
                "Value" => value.to_owned(),
                "Footprint" => footprint
                    .map(str::to_owned)
                    .unwrap_or_else(|| prop.arg(2).unwrap_or("").to_owned()),
                _ => prop.arg(2).unwrap_or("").to_owned(),
            };
            let (px, py, pa) = match prop.child("at") {
                Some(a) => (
                    sexpr::nm(a.arg(1).unwrap_or("0"))?,
                    sexpr::nm(a.arg(2).unwrap_or("0"))?,
                    a.arg(3).unwrap_or("0").parse::<f64>().unwrap_or(0.0) as i64,
                ),
                None => (0, 0, 0),
            };
            let mut p = vec![
                Sx::atom("property"),
                Sx::str(name),
                Sx::str(text),
                at(pos.0 + px, pos.1 - py, pa),
            ];
            p.extend(
                prop.items()
                    .iter()
                    .filter(|c| matches!(c.head(), Some("effects" | "hide")))
                    .cloned(),
            );
            v.push(Sx::list(p));
        }
        let mut numbers: Vec<String> = kilib::pins(&sym)?.into_iter().map(|p| p.number).collect();
        numbers.sort();
        numbers.dedup();
        for n in numbers {
            v.push(Sx::list(vec![
                Sx::atom("pin"),
                Sx::str(n.clone()),
                uuid_sx(&format!("{key}/pin/{n}")),
            ]));
        }
        v.push(Sx::list(vec![
            Sx::atom("instances"),
            Sx::list(vec![
                Sx::atom("project"),
                Sx::str(project.clone()),
                Sx::list(vec![
                    Sx::atom("path"),
                    Sx::str(format!("/{}", self.root)),
                    Sx::list(vec![Sx::atom("reference"), Sx::str(reference)]),
                    Sx::node("unit", &["1"]),
                ]),
            ]),
        ]));
        self.items.push(Sx::list(v));
        Ok(())
    }

    fn power(&mut self, lib_id: &str, value: &str, pos: (i64, i64)) -> Result<(), String> {
        self.power_count += 1;
        let r = format!("#PWR{:03}", self.power_count);
        let key = format!("{}/power/{r}", self.sheet.project);
        self.instance(lib_id, &r, value, None, pos, &key)
    }

    fn label(&mut self, text: &str, global: bool, pos: (i64, i64), dir: (i64, i64), key: &str) {
        let (angle, j) = label_dir(dir);
        let mut v = vec![
            Sx::atom(if global { "global_label" } else { "label" }),
            Sx::str(text),
        ];
        if global {
            v.push(Sx::node("shape", &["passive"]));
        }
        v.push(at(pos.0, pos.1, angle));
        v.push(Sx::node("fields_autoplaced", &["yes"]));
        v.push(justified(j));
        v.push(uuid_sx(key));
        if global {
            v.push(Sx::list(vec![
                Sx::atom("property"),
                Sx::str("Intersheetrefs"),
                Sx::str("${INTERSHEET_REFS}"),
                at(pos.0, pos.1, 0),
                sexpr::parse("(effects (font (size 1.27 1.27)) (justify left) (hide yes))")
                    .unwrap(),
            ]));
        }
        self.items.push(Sx::list(v));
    }

    fn wire(&mut self, a: (i64, i64), b: (i64, i64), key: &str) {
        self.items.push(Sx::list(vec![
            Sx::atom("wire"),
            Sx::list(vec![
                Sx::atom("pts"),
                Sx::node("xy", &[&mm(a.0), &mm(a.1)]),
                Sx::node("xy", &[&mm(b.0), &mm(b.1)]),
            ]),
            sexpr::parse("(stroke (width 0) (type default))").unwrap(),
            uuid_sx(key),
        ]));
    }

    fn part(&mut self, part: &Part) -> Result<(), String> {
        let project = self.sheet.project.clone();
        let key = format!("{project}/symbol/{}", part.reference);
        self.instance(
            &part.lib_id,
            &part.reference,
            &part.value,
            part.footprint.as_deref(),
            part.at,
            &key,
        )?;
        let pins = kilib::pins(self.sym(&part.lib_id)?)?;
        for n in part.conns.keys() {
            if !pins.iter().any(|p| &p.number == n) {
                return Err(format!(
                    "{}: `{}` has no pin {n}",
                    part.reference, part.lib_id
                ));
            }
        }
        let mut groups: BTreeMap<(i64, i64), Vec<&Pin>> = BTreeMap::new();
        for p in &pins {
            groups.entry(p.at).or_default().push(p);
        }
        for (pt, mut group) in groups {
            group.sort_by(|a, b| a.number.cmp(&b.number));
            let conns: Vec<(&str, &Conn)> = group
                .iter()
                .filter_map(|p| part.conns.get(&p.number).map(|c| (p.number.as_str(), c)))
                .collect();
            let Some(&(first, conn)) = conns.first() else {
                continue;
            };
            if conns.iter().any(|(_, c)| *c != conn) {
                return Err(format!(
                    "{}: stacked pins at one point disagree: {conns:?}",
                    part.reference
                ));
            }
            let p = (part.at.0 + pt.0, part.at.1 - pt.1);
            let pkey = format!("{key}/pin/{first}");
            if *conn == Conn::NoConnect {
                self.items.push(Sx::list(vec![
                    Sx::atom("no_connect"),
                    Sx::node("at", &[&mm(p.0), &mm(p.1)]),
                    uuid_sx(&format!("{pkey}/nc")),
                ]));
                continue;
            }
            let dir = outward(group[0].angle)?;
            let end = (p.0 + dir.0 * STUB, p.1 + dir.1 * STUB);
            self.wire(p, end, &format!("{pkey}/wire"));
            match conn.clone() {
                Conn::Label(t) => self.label(&t, false, end, dir, &format!("{pkey}/label")),
                Conn::Power(lib, value) => self.power(&lib, &value, end)?,
                Conn::NoConnect => unreachable!(),
            }
        }
        Ok(())
    }
}

/// Every library id the sheet uses.
pub fn lib_ids(sheet: &Sheet) -> Vec<String> {
    let mut ids: Vec<String> = sheet.parts.iter().map(|p| p.lib_id.clone()).collect();
    for p in &sheet.parts {
        for c in p.conns.values() {
            if let Conn::Power(lib, _) = c {
                ids.push(lib.clone());
            }
        }
    }
    for (lib, _) in &sheet.flags {
        ids.push(lib.clone());
    }
    if !sheet.flags.is_empty() {
        ids.push(sheet.flag_lib_id.clone());
    }
    ids.sort();
    ids.dedup();
    ids
}

pub fn emit(sheet: &Sheet, syms: &BTreeMap<String, Sx>) -> Result<String, String> {
    let root = root_uuid(&sheet.project);
    let mut e = Emitter {
        sheet,
        syms,
        root: root.clone(),
        items: Vec::new(),
        power_count: 0,
        flag_count: 0,
    };
    let mut lib = vec![Sx::atom("lib_symbols")];
    for id in lib_ids(sheet) {
        lib.push(e.sym(&id)?.clone());
    }
    let mut parts: Vec<&Part> = sheet.parts.iter().collect();
    parts.sort_by(|a, b| a.reference.cmp(&b.reference));
    for w in parts.windows(2) {
        if w[0].reference == w[1].reference {
            return Err(format!("reference `{}` twice", w[0].reference));
        }
    }
    for part in parts {
        e.part(part)?;
    }
    let mut flags = sheet.flags.clone();
    flags.sort();
    for (i, (lib_id, value)) in flags.iter().enumerate() {
        let pos = (GRID * 20 * (i as i64 + 1), GRID * 10);
        e.power(lib_id, value, pos)?;
        e.flag_count += 1;
        let r = format!("#FLG{:03}", e.flag_count);
        let key = format!("{}/flag/{r}", sheet.project);
        let flag_lib = sheet.flag_lib_id.clone();
        e.instance(&flag_lib, &r, "PWR_FLAG", None, pos, &key)?;
    }
    for (i, (text, global)) in sheet.loose_labels.iter().enumerate() {
        let pos = (GRID * 600, GRID * (20 + 4 * i as i64));
        let key = format!("{}/loose/{i}", sheet.project);
        e.label(text, *global, pos, (1, 0), &key);
    }
    let mut top = vec![
        Sx::atom("kicad_sch"),
        Sx::node("version", &[&sheet.version]),
        Sx::list(vec![Sx::atom("generator"), Sx::str("aethelgard")]),
        Sx::list(vec![Sx::atom("generator_version"), Sx::str("0.1")]),
        Sx::list(vec![Sx::atom("uuid"), Sx::str(root)]),
        Sx::list(vec![Sx::atom("paper"), Sx::str(sheet.paper.clone())]),
        Sx::list(vec![
            Sx::atom("title_block"),
            Sx::list(vec![Sx::atom("title"), Sx::str(sheet.title.clone())]),
        ]),
        Sx::list(lib),
    ];
    top.extend(e.items);
    top.push(sexpr::parse(r#"(sheet_instances (path "/" (page "1")))"#).unwrap());
    Ok(sexpr::write(&Sx::list(top)))
}
