//! AET-WO-1323 §1.1 (4): `1311` F2 reproduced and swept. `route.rs`'s
//! `paired(12 mm)` board (copied: the test module's helpers are private),
//! plus a part `S` whose two `/S` pads stand either side of the pair.
//! Dropped into a tree copy as `route/stub1323.rs`; never committed so.

use super::*;

const MM: i64 = 1_000_000;

fn rect(x0: i64, y0: i64, x1: i64, y1: i64) -> Rect {
    Rect { x0, y0, x1, y1 }
}

fn pad(number: &str, net: Option<&str>, r: Rect) -> Pad {
    Pad {
        number: number.into(),
        net: net.map(Into::into),
        rect: r,
        corner_nm: 0,
        drill_nm: None,
        join: None,
    }
}

fn at(x: i64, y: i64) -> Rect {
    rect(x - 150_000, y - 150_000, x + 150_000, y + 150_000)
}

pub(super) fn paired(n_far: i64) -> Board {
    let mut stack = crate::stack::reference();
    stack.pairs.push(crate::stack::Pair {
        name: "X".into(),
        nets: ["/P".into(), "/N".into()],
        gap_nm: 300_000,
        width_nm: 200_000,
    });
    let part = |reference: &str, x: i64, xn: i64| Part {
        rotation: 0,
        reference: reference.into(),
        pads: vec![
            pad("1", Some("/P"), at(x, 5 * MM)),
            pad("2", Some("/N"), at(xn, 5 * MM + MM / 2)),
        ],
        courtyard: Vec::new(),
        keepouts: Vec::new(),
    };
    Board {
        outline: rect(0, 0, 20 * MM, 10 * MM),
        parts: vec![part("A", 2 * MM, 2 * MM), part("B", 12 * MM, n_far)],
        stack,
    }
}

/// `paired(12 mm)` with `S`'s pads at (x, y1) and (x, y2), in µm.
pub(super) fn with_s(x: i64, y1: i64, y2: i64) -> Board {
    let mut b = paired(12 * MM);
    b.parts.push(Part {
        rotation: 0,
        reference: "S".into(),
        pads: vec![
            pad("1", Some("/S"), at(x * 1000, y1 * 1000)),
            pad("2", Some("/S"), at(x * 1000, y2 * 1000)),
        ],
        courtyard: Vec::new(),
        keepouts: Vec::new(),
    });
    b
}

fn class(r: &Result<Routed, Unroutable>) -> String {
    match r {
        Ok(_) => "ok".into(),
        Err(e) if e.why.contains("joined to nothing") => format!("STUB {}", e.why),
        Err(e) => format!("refused {}: {}", e.net, e.why),
    }
}

/// F2's own placement, then a sweep of `S` over the board: x from 0.5 to
/// 19.5 mm by 0.25 mm, the pads at every (y1, y2) of {1, 2, 3, 4} × {6.5,
/// 7, 8, 9} mm. Prints each outcome; AET1323_STUB_ONE=x,y1,y2 runs one.
#[test]
#[ignore]
fn stub_sweep() {
    if let Ok(one) = std::env::var("AET1323_STUB_ONE") {
        let v: Vec<i64> = one.split(',').map(|s| s.parse().unwrap()).collect();
        println!("{one}: {}", class(&route(&with_s(v[0], v[1], v[2]))));
        return;
    }
    println!(
        "F2 (7000, 3000, 8000): {}",
        class(&route(&with_s(7000, 3000, 8000)))
    );
    let (mut ok, mut stub, mut other) = (0, 0, 0);
    for xi in 0..77 {
        let x = 500 + 250 * xi;
        for y1 in [1000, 2000, 3000, 4000] {
            for y2 in [6500, 7000, 8000, 9000] {
                let c = class(&route(&with_s(x, y1, y2)));
                if c == "ok" {
                    ok += 1;
                } else if c.starts_with("STUB") {
                    stub += 1;
                    println!("{x},{y1},{y2}: {c}");
                } else {
                    other += 1;
                    println!("{x},{y1},{y2}: {c}");
                }
            }
        }
    }
    println!(
        "sweep: {ok} ok, {stub} stub, {other} other refusal, of {}",
        ok + stub + other
    );
}
