//! A board's pads and a netlist written from them in KiCad 10.0.7's
//! `kicadsexpr` shape, for the support gate's tier-1 tests (`AET-WO-4300g`,
//! `1312`, `4240c`); the tier-2 tests read KiCad's own export.
#![allow(dead_code)]

use std::collections::BTreeMap;

use aethelgard_eda::strict;

/// Each footprint's value and each pad's net, from the board text.
pub type Pads = (BTreeMap<String, String>, BTreeMap<(String, String), String>);

pub fn pads_of(board: &str) -> Pads {
    let tree = strict::board(board).unwrap();
    let mut values = BTreeMap::new();
    let mut nets = BTreeMap::new();
    for fp in tree.root().all("footprint") {
        let reference = fp.field("Reference").unwrap().unwrap().to_owned();
        values.insert(
            reference.clone(),
            fp.field("Value").unwrap().unwrap().to_owned(),
        );
        for pad in fp.all("pad") {
            let Some(net) = pad.one("net").unwrap().and_then(|n| n.last()) else {
                continue;
            };
            nets.insert(
                (reference.clone(), pad.arg(1).unwrap().to_owned()),
                net.to_owned(),
            );
        }
    }
    (values, nets)
}

/// The netlist, each symbol named as KiCad writes it for the emitter's parts
/// by their reference's letters.
pub fn netlist_text((values, nets): &Pads) -> String {
    let mut by_net: BTreeMap<&str, Vec<(&str, &str)>> = BTreeMap::new();
    for ((r, p), n) in nets {
        by_net.entry(n).or_default().push((r, p));
    }
    let mut s = String::from("(export (version \"E\") (components\n");
    for (r, v) in values {
        let stem: String = r.chars().take_while(char::is_ascii_alphabetic).collect();
        let lib = match stem.as_str() {
            "C" => " (libsource (lib \"Device\") (part \"C\"))",
            "R" => " (libsource (lib \"Device\") (part \"R\"))",
            "Y" => " (libsource (lib \"Device\") (part \"Crystal_GND24\"))",
            "SW" => " (libsource (lib \"Switch\") (part \"SW_Push\"))",
            "TP" => " (libsource (lib \"Connector\") (part \"TestPoint\"))",
            _ => "",
        };
        s.push_str(&format!("(comp (ref \"{r}\") (value \"{v}\"){lib})\n"));
    }
    s.push_str(")\n(nets\n");
    for (i, (n, nodes)) in by_net.iter().enumerate() {
        s.push_str(&format!("(net (code \"{}\") (name \"{n}\")", i + 1));
        for (r, p) in nodes {
            s.push_str(&format!(" (node (ref \"{r}\") (pin \"{p}\"))"));
        }
        s.push_str(")\n");
    }
    s.push_str("))\n");
    s
}
