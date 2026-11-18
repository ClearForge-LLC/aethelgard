//! `AET-WO-1303b`'s measured stop (ruling D5 (b), PR #222): for each pad of
//! the load on a net, the certified bound from the source pad
//! ([`aethelgard_eda::supply`]). A measurement, not a gate: it judges
//! nothing and is not part of the CLI.
//!
//! Usage: `supply_stop <board.kicad_pcb> <spec.json> <board net> <source
//! ref.pad> <load ref> <h_nm> <crop_mm> [<pad> ...]`

use std::time::Instant;

use aethelgard_eda::flow::Iv;
use aethelgard_eda::supply::{self, Model};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let text = std::fs::read_to_string(&a[1]).expect("board");
    let spec: aethelgard_ir::Specification =
        serde_json::from_slice(&std::fs::read(&a[2]).expect("spec")).expect("spec json");
    let stack = aethelgard_eda::stack::Stack::of(&spec);
    let read =
        aethelgard_eda::extract::read(&text, &stack).unwrap_or_else(|r| panic!("refused: {r:?}"));
    // the net as the board writes it (a rail is bare, a signal `/`-prefixed)
    let net = a[3].clone();
    let (sref, spad) = a[4].split_once('.').expect("source ref.pad");
    let load = &a[5];
    let h_nm: i64 = a[6].parse().unwrap();
    let crop_mm: i64 = a[7].parse().unwrap();
    let only: Vec<&String> = a[8..].iter().collect();
    let rho = Iv::ratio(17_241, 1_000_000_000)
        .mul(Iv::exact(1.0).add(Iv::ratio(393, 100_000).scale(105.0)))
        .mul(Iv::ratio(1181, 1000));
    let m = Model {
        h_nm,
        rho,
        outer_nm: (30_000, stack.copper_max_nm),
        plane_nm: (15_000, stack.copper_max_nm),
        plating_nm: 18_000,
        beta_nm: 10_000,
        hole_offset_nm: 75_000,
        edge_nm: 625_000,
        crop_nm: crop_mm * 1_000_000,
        fill_clearance_nm: 202_500,
        opening_nm: 125_000,
    };
    let board = &read.board;
    let source = supply::pad(board, &net, sref, spad).expect("source pad");
    let mut sinks: Vec<_> = board
        .parts
        .iter()
        .filter(|p| &p.reference == load)
        .flat_map(|p| p.pads.iter())
        .filter(|p| p.net.as_deref() == Some(net.as_str()))
        .filter(|p| only.is_empty() || only.iter().any(|o| **o == p.number))
        .collect();
    sinks.sort_by_key(|p| p.number.parse::<u32>().unwrap_or(u32::MAX));
    let mut terminals = vec![source];
    terminals.extend(sinks.iter().copied());
    let t = Instant::now();
    let nc = supply::copper(board, &read.routed, &net, &terminals, &m)
        .unwrap_or_else(|r| panic!("refused: {r:?}"));
    let cells: usize = nc.layers.iter().map(|l| l.cells.len()).sum();
    println!(
        "== {} {net} from {sref}.{spad} to {load} ({} pads): h {h_nm} nm, crop {crop_mm} mm, rho {:.5e} ohm*mm; {cells} cells {:?}; {} vias, joins {:?}; copper in {:?}",
        a[1].rsplit('/').next().unwrap_or(""),
        sinks.len(),
        rho.hi,
        nc.layers
            .iter()
            .map(|l| (l.name.as_str(), l.cells.len()))
            .collect::<Vec<_>>(),
        nc.vias.len(),
        {
            let mut c = std::collections::BTreeMap::new();
            for u in &nc.vias {
                *c.entry(u.joins.len()).or_insert(0) += 1;
            }
            c
        },
        t.elapsed()
    );
    // AET_DUMP_CELLS=<file>: every copper cell (`cell <layer> <x0> <y0>
    // <side>`), nm, for the copper checks against KiCad's refill and an
    // independent reading of the board
    if let Ok(path) = std::env::var("AET_DUMP_CELLS") {
        let mut out = String::new();
        for l in &nc.layers {
            for &(i, j) in &l.cells {
                out.push_str(&format!(
                    "cell {} {} {} {h_nm}\n",
                    l.name,
                    nc.origin.0 + i * h_nm,
                    nc.origin.1 + j * h_nm
                ));
            }
        }
        std::fs::write(path, out).expect("dump");
        return;
    }
    println!(
        "   rows (mm): {:?}",
        nc.rows
            .iter()
            .map(|(l, h)| (l.lo, h.hi))
            .collect::<Vec<_>>()
    );
    if std::env::var("AET_VIAS").is_ok() {
        for u in &nc.vias {
            let j: Vec<String> = u
                .joins
                .iter()
                .map(|(l, p)| format!("{}:{}", nc.layers[*l].name, p.len()))
                .collect();
            println!(
                "   via ({:.3},{:.3}) {}",
                u.via.at.0 as f64 / 1e6,
                u.via.at.1 as f64 / 1e6,
                j.join(" ")
            );
        }
    }
    for n in &nc.notes {
        println!("   note: {n}");
    }
    for p in sinks {
        let t = Instant::now();
        match supply::bound(&nc, &m, source, p, 400) {
            Ok(b) => {
                let mut used: Vec<_> = b
                    .via_transfer
                    .iter()
                    .filter(|(_, q)| *q * 100 >= aethelgard_eda::flow::QUANTA)
                    .collect();
                used.sort_by_key(|(_, q)| -q);
                println!(
                    "  {load}.{:>4}: {:9.3} mOhm (sheets {:.3}, junctions {:.3}, barrels {:.3}; iterations {}, residual {:.1e}, {:?}); vias carrying >= 1 %: {}",
                    p.number,
                    b.bound.energy.hi * 1e3,
                    b.shares[0] * 1e3,
                    b.shares[1] * 1e3,
                    b.shares[2] * 1e3,
                    b.bound.iterations,
                    b.bound.residual,
                    t.elapsed(),
                    used.iter()
                        .map(|(at, q)| format!(
                            "({:.3},{:.3}) {:.0}%",
                            at.0 as f64 / 1e6,
                            at.1 as f64 / 1e6,
                            *q as f64 * 100.0 / aethelgard_eda::flow::QUANTA as f64
                        ))
                        .collect::<Vec<_>>()
                        .join(" ")
                );
            }
            Err(r) => println!("  {load}.{:>4}: REFUSED {r:?}", p.number),
        }
    }
}
