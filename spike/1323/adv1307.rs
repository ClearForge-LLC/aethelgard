//! AET-WO-1323 §1.1: `1307` A4's adversary, RECONSTRUCTED (the original was
//! not kept): a 249 mm board, four LQFP-144 copies (the vendored footprint's
//! pads, roundrect 0.25; its courtyard taken as the ±11.65 mm square), and
//! 288 two-pad nets, a random perfect matching of the 576 pads from a fixed
//! seed. Built as `route::Board`, which `emit` cannot produce. Dropped into
//! a tree copy as `route/adv1307.rs`, included from `route.rs` under
//! `#[cfg(test)]`; never committed in this form.

use super::*;

const MM: i64 = 1_000_000;

fn lqfp(reference: &str, cx: i64, cy: i64, nets: &[Option<String>]) -> Part {
    let mut pads = Vec::new();
    let (far, step, first) = (10_662_500, 500_000, -8_750_000);
    for k in 0..144usize {
        let (side, i) = (k / 36, (k % 36) as i64);
        let along = first + i * step;
        let (x, y, w, h) = match side {
            0 => (-far, along, 1_475_000, 300_000),
            1 => (along, far, 300_000, 1_475_000),
            2 => (far, -along, 1_475_000, 300_000),
            _ => (-along, -far, 300_000, 1_475_000),
        };
        let (x, y) = (cx + x, cy + y);
        pads.push(Pad {
            number: (k + 1).to_string(),
            net: nets[k].clone(),
            rect: Rect {
                x0: x - w / 2,
                y0: y - h / 2,
                x1: x + w / 2,
                y1: y + h / 2,
            },
            corner_nm: 75_000,
            drill_nm: None,
            join: None,
        });
    }
    let c = 11_650_000;
    let q = [
        (cx - c, cy - c),
        (cx + c, cy - c),
        (cx + c, cy + c),
        (cx - c, cy + c),
    ];
    Part {
        reference: reference.into(),
        rotation: 0,
        pads,
        courtyard: (0..4).map(|i| (q[i], q[(i + 1) % 4])).collect(),
        keepouts: Vec::new(),
    }
}

/// The adversary's board.
pub(super) fn adversary(seed: u64) -> Board {
    let mut order: Vec<usize> = (0..576).collect();
    let mut s = seed;
    let mut next = || {
        s = s
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        s >> 33
    };
    for i in (1..order.len()).rev() {
        let j = (next() % (i as u64 + 1)) as usize;
        order.swap(i, j);
    }
    let mut nets: Vec<Option<String>> = vec![None; 576];
    for (k, w) in order.chunks(2).enumerate() {
        for &p in w {
            nets[p] = Some(format!("/N{k:03}"));
        }
    }
    let at = [(62, 62), (187, 62), (62, 187), (187, 187)];
    let parts = at
        .iter()
        .enumerate()
        .map(|(i, &(x, y))| {
            lqfp(
                &format!("U{}", i + 1),
                x * MM,
                y * MM,
                &nets[i * 144..(i + 1) * 144],
            )
        })
        .collect();
    Board {
        outline: Rect {
            x0: 0,
            y0: 0,
            x1: 249 * MM,
            y1: 249 * MM,
        },
        parts,
        stack: crate::stack::reference(),
    }
}

/// Run with `--ignored --nocapture`; AET1323_SEED picks the seed (1307).
#[test]
#[ignore]
fn adversary_1307_a4() {
    let seed = std::env::var("AET1323_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1307);
    let board = adversary(seed);
    let t = std::time::Instant::now();
    let r = route(&board);
    println!(
        "seed {seed}: {:?} in {:.1} s",
        r.as_ref().map(|r| r.ripups).map_err(|e| (&e.net, &e.why)),
        t.elapsed().as_secs_f64()
    );
}
