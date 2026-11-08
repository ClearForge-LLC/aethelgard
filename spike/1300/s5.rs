//! S5: escaping (`AET-WO-1100` FEEDBACK A3). For every printable ASCII
//! character `RUL-14` admits that is not a letter or digit, and a few
//! KiCad markup shapes, a name carrying it is emitted in each identifier
//! class as the thing it becomes, and read back from ERC's report and the
//! netlist:
//! - net name: a local label, and a loose local label (ERC: dangling);
//! - net name, global: a loose global label;
//! - domain id: a power symbol's value (the power net);
//! - reference and part: a resistor's Reference and Value (ERC: its
//!   unconnected pin names the reference);
//! - project name: the instances' project and the title block, one file each.

use std::collections::BTreeMap;

use super::sch::{self, Conn, GRID, Part, Sheet};
use super::sexpr::Sx;
use super::{Ctx, answer, write};

pub fn names() -> Vec<String> {
    let mut v: Vec<String> = (0x21u8..=0x7e)
        .filter(|b| !b.is_ascii_alphanumeric())
        .map(|b| format!("A{}B", b as char))
        .collect();
    for extra in [
        "~{X}",
        "^{X}",
        "_{X}",
        "${X}",
        "${PROJECTNAME}",
        "A{slash}B",
        "A{backslash}B",
        "A{colon}B",
        "A{brace}B",
        "A{B}",
        "A[0..1]",
        "\\\"",
        "\\\\",
        ")(",
        "9A",
        "A//B",
        "/A",
    ] {
        v.push(extra.to_owned());
    }
    v.push("Z".repeat(63));
    v
}

fn sheet(names: &[String], power: &str) -> Sheet {
    let mut parts = Vec::new();
    let mut loose = Vec::new();
    for (i, n) in names.iter().enumerate() {
        let x = GRID * (20 + 16 * (i as i64 % 16));
        let y = GRID * (40 + 30 * (i as i64 / 16));
        parts.push(Part {
            lib_id: "Device:R".into(),
            reference: format!("R{n}"),
            value: format!("V{n}"),
            footprint: None,
            at: (x, y),
            conns: BTreeMap::from([("1".to_owned(), Conn::Label(format!("L{n}")))]),
        });
        parts.push(Part {
            lib_id: "Device:R".into(),
            reference: format!("P{i}"),
            value: "x".into(),
            footprint: None,
            at: (x + GRID * 6, y),
            conns: BTreeMap::from([
                ("1".to_owned(), Conn::Power(power.into(), format!("D{n}"))),
                ("2".to_owned(), Conn::NoConnect),
            ]),
        });
        loose.push((format!("L{n}"), false));
        loose.push((format!("G{n}"), true));
    }
    Sheet {
        project: "s5".into(),
        title: "s5".into(),
        version: super::s2::VERSION.into(),
        paper: "A0".into(),
        parts,
        flags: Vec::new(),
        flag_lib_id: String::new(),
        loose_labels: loose,
    }
}

fn comps(net: &Sx) -> Vec<(String, String)> {
    net.child("components")
        .map(|c| {
            c.children("comp")
                .map(|c| {
                    (
                        c.child("ref")
                            .and_then(|r| r.arg(1))
                            .unwrap_or("")
                            .to_owned(),
                        c.child("value")
                            .and_then(|r| r.arg(1))
                            .unwrap_or("")
                            .to_owned(),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn run(ctx: &mut Ctx) -> Result<(), String> {
    let all = names();
    let (ids, syms_all) = super::s2::load(ctx)?;
    let mut syms = BTreeMap::new();
    let r_id = if ids.footprints {
        "Device:R"
    } else {
        "spike:R"
    };
    syms.insert("Device:R".to_owned(), syms_all[r_id].clone());
    syms.insert(ids.rail.clone(), syms_all[&ids.rail].clone());
    let s = sheet(&all, &ids.rail);
    let text = sch::emit(&s, &syms)?;
    let reparsed = super::sexpr::parse(&text).map(|_| "ok").unwrap_or("FAILED");
    answer(
        "S5",
        format!(
            "{} names x 6 classes emitted; our own reader re-parses the file: {reparsed}; identical on re-emission={}",
            all.len(),
            text == sch::emit(&s, &syms)?
        ),
    );
    if ctx.kicad.is_none() {
        answer("S5", "blocked: kicad-cli absent here (tools image only)");
        return Ok(());
    }
    let dir = ctx.dir("s5")?;
    let path = dir.join("s5.kicad_sch");
    write(&path, &text)?;
    let (ran, findings) = ctx.erc(&path)?;
    answer(
        "S5",
        format!(
            "ERC exit {}, findings {}",
            ran.code_text(),
            super::summary(&findings)
        ),
    );
    let descriptions: Vec<String> = findings
        .iter()
        .flat_map(|f| f.items.iter().map(|i| i.0.clone()))
        .collect();
    for f in findings.iter().filter(|f| {
        matches!(
            f.kind.as_str(),
            "bus_to_net_conflict" | "multiple_net_names" | "unresolved_variable"
        )
    }) {
        let items: Vec<&str> = f.items.iter().map(|i| i.0.as_str()).collect();
        answer(
            "S5",
            format!(
                "ERC {} {}: {} {:?}",
                f.kind, f.severity, f.description, items
            ),
        );
    }
    let net = super::s2::netlist(ctx, &path)?;
    let components = net.as_ref().map(comps).unwrap_or_default();
    // Each net by the nodes on it: the label net of name i is the one holding
    // R<name> pin 1, its power net the one holding P<i> pin 1.
    let mut by_node: BTreeMap<(String, String), String> = BTreeMap::new();
    if let Some(nl) = &net
        && let Some(nets) = nl.child("nets")
    {
        for n in nets.children("net") {
            let name = n
                .child("name")
                .and_then(|x| x.arg(1))
                .unwrap_or("")
                .to_owned();
            for node in n.children("node") {
                let r = node
                    .child("ref")
                    .and_then(|x| x.arg(1))
                    .unwrap_or("")
                    .to_owned();
                let p = node
                    .child("pin")
                    .and_then(|x| x.arg(1))
                    .unwrap_or("")
                    .to_owned();
                by_node.insert((r, p), name.clone());
            }
        }
    }
    let mut bad = 0;
    for (i, n) in all.iter().enumerate() {
        // ERC: the exact text inside the item descriptions' quotes.
        let erc = |want: &str| -> String {
            let quoted = [
                format!("'{want}'"),
                format!("\"{want}\""),
                format!(" {want} "),
            ];
            if descriptions
                .iter()
                .any(|d| quoted.iter().any(|q| d.contains(q.as_str())))
            {
                "ok".into()
            } else {
                "MISSING".into()
            }
        };
        let net_at = |r: &str, want: &str| -> String {
            match by_node.get(&(r.to_owned(), "1".to_owned())) {
                Some(got) if got == want => "ok".into(),
                Some(got) => format!("CHANGED to {got:?}"),
                None => format!("MISSING (no node {r}.1)"),
            }
        };
        let comp = components.iter().find(|(r, _)| *r == format!("R{n}"));
        let fields = match comp {
            Some((_, v)) if *v == format!("V{n}") => "ok".to_owned(),
            Some((_, v)) => format!("value CHANGED to {v:?}"),
            None => "ref MISSING".to_owned(),
        };
        let line = format!(
            "name {n:?}: label-erc {} label-net {} global-erc {} domain-net {} ref-erc {} ref+value-netlist {}",
            erc(&format!("L{n}")),
            net_at(&format!("R{n}"), &format!("/L{n}")),
            erc(&format!("G{n}")),
            net_at(&format!("P{i}"), &format!("D{n}")),
            erc(&format!("R{n}")),
            fields
        );
        if line.contains("MISSING") || line.contains("CHANGED") {
            bad += 1;
        }
        answer("S5", line);
    }
    answer(
        "S5",
        format!("{bad} of {} names changed or lost in some class", all.len()),
    );
    // Project names: one file each, read back from the netlist's title block.
    let mut proj_bad = Vec::new();
    for (i, n) in all.iter().enumerate() {
        let d = ctx.dir(&format!("s5p-{i}"))?;
        let s = Sheet {
            project: format!("P{n}"),
            title: format!("T{n}"),
            version: super::s2::VERSION.into(),
            paper: "A4".into(),
            parts: vec![Part {
                lib_id: "Device:R".into(),
                reference: "R1".into(),
                value: "1k".into(),
                footprint: None,
                at: (GRID * 40, GRID * 40),
                conns: BTreeMap::from([
                    ("1".to_owned(), Conn::NoConnect),
                    ("2".to_owned(), Conn::NoConnect),
                ]),
            }],
            flags: Vec::new(),
            flag_lib_id: String::new(),
            loose_labels: Vec::new(),
        };
        let p = d.join("p.kicad_sch");
        write(&p, &sch::emit(&s, &syms)?)?;
        let got = super::s2::netlist(ctx, &p)?;
        let title = got
            .as_ref()
            .and_then(|g| g.child("design"))
            .and_then(|d| d.child("sheet"))
            .and_then(|s| s.child("title_block"))
            .and_then(|t| t.child("title"))
            .and_then(|t| t.arg(1))
            .map(str::to_owned);
        let refs = got.as_ref().map(comps).unwrap_or_default();
        let ok = title.as_deref() == Some(&format!("T{n}")) && refs.iter().any(|(r, _)| r == "R1");
        if !ok {
            proj_bad.push(format!("{n:?}: title {title:?} refs {refs:?}"));
        }
    }
    answer(
        "S5",
        format!(
            "project-name class: {} of {} files read back with title and R1 intact; failures {:?}",
            all.len() - proj_bad.len(),
            all.len(),
            proj_bad
        ),
    );
    Ok(())
}
