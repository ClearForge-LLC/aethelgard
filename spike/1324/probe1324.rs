//! AET-WO-1324 §1.1 (2): one unstitched net of N pads, timed by the shim.
//! Dropped into a tree copy as `route/probe1324.rs`; never committed so.
//! Run: AET1324_N=400,900,1600 cargo test --release -p aethelgard-eda probe1324 -- --nocapture

use super::*;

const MM: i64 = 1_000_000;

/// N one-pad parts on a square lattice at `pitch`, every pad on `/BIG`.
pub(super) fn lattice(n: usize, pitch: i64) -> Board {
    let side = (n as f64).sqrt().ceil() as i64;
    let margin = 3 * MM;
    let mut parts = Vec::new();
    for k in 0..n as i64 {
        let (i, j) = (k % side, k / side);
        let (x, y) = (margin + i * pitch, margin + j * pitch);
        parts.push(Part {
            rotation: 0,
            reference: format!("P{k}"),
            pads: vec![Pad {
                number: "1".into(),
                net: Some("/BIG".into()),
                rect: Rect { x0: x - 250_000, y0: y - 250_000, x1: x + 250_000, y1: y + 250_000 },
                corner_nm: 0,
                drill_nm: None,
                join: None,
            }],
            courtyard: Vec::new(),
            keepouts: Vec::new(),
        });
    }
    let w = 2 * margin + (side - 1) * pitch;
    Board {
        outline: Rect { x0: 0, y0: 0, x1: w, y1: w },
        parts,
        stack: crate::stack::reference(),
    }
}

#[test]
fn probe1324() {
    let ns: Vec<usize> = std::env::var("AET1324_N")
        .unwrap_or_else(|_| "400".into())
        .split(',')
        .map(|s| s.parse().unwrap())
        .collect();
    let pitch: i64 = std::env::var("AET1324_PITCH").map_or(2 * MM, |s| s.parse().unwrap());
    for n in ns {
        let b = lattice(n, pitch);
        spike_time::reset();
        let t = std::time::Instant::now();
        let out = route_measured(&b, Rung::TODAY, WORK);
        let wall = t.elapsed().as_secs_f64();
        let get = |c: &'static std::thread::LocalKey<std::cell::Cell<u128>>| c.with(|x| x.get()) as f64 / 1e9;
        let cnt = |c: &'static std::thread::LocalKey<std::cell::Cell<u64>>| c.with(|x| x.get());
        let (work, segs, vias) = match &out {
            Ok((r, w)) => (*w, r.segments.len(), r.vias.len()),
            Err(u) => panic!("n={n}: {}", u.why),
        };
        let (scan, seed, judge, route) = (get(&spike_time::SCAN), get(&spike_time::SEED), get(&spike_time::JUDGE), get(&spike_time::ROUTE));
        println!(
            "N\t{n}\tpitch_nm\t{pitch}\twall_s\t{wall:.3}\troute_s\t{route:.3}\tscan_s\t{scan:.3}\tseed_s\t{seed:.3}\tjudge_s\t{judge:.3}\tother_s\t{:.3}\twork\t{work}\tscan_pairs\t{}\tseeds\t{}\tindex\t{}\tsegments\t{segs}\tvias\t{vias}",
            route - scan - seed - judge,
            cnt(&spike_time::SCAN_PAIRS),
            cnt(&spike_time::SEEDS),
            cnt(&spike_time::INDEX),
        );
    }
}
