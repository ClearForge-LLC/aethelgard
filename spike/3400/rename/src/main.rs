//! AET-WO-3400 S2, the renaming check. `renamed`, `named_back`, `mix` and `hash` are copied verbatim
//! from `crates/aethelgard-cli/tests/escape_tier1.rs` (AET-WO-1307, ruled K3), seeds 0..20, with the
//! same assertion: each renamed board routes, and to the board's own copper once its names are put
//! back. One extension, named: the test's boards declare no class, pair, through-hole join or thermal
//! array, so its `renamed` renames only `stack.planes`. Every variant with a power tree declares all
//! four, keyed by net name (classes, pairs) or by reference (joins, thermal), so this copy renames
//! them by the same maps. Without it, a renamed net would lose its class and a renamed part its join.
//!
//! Usage: aet-3400-rename <spec.json> <emitted.kicad_pcb>
use std::collections::BTreeMap;
use std::fs;

use aethelgard_eda::route::{self, Board, Routed};
use aethelgard_eda::stack::Stack;
use aethelgard_eda::strict;

/// SplitMix64: a fixed, toolchain-independent hash for the renamings.
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn hash(seed: u64, s: &str) -> u64 {
    s.bytes().fold(mix(seed), |h, b| mix(h ^ u64::from(b)))
}

fn renamed(
    board: &Board,
    seed: u64,
) -> (Board, BTreeMap<String, String>, BTreeMap<String, String>) {
    let new_net = |n: &str| {
        if (!board.stack.has_planes() && n == route::GROUND) || n.starts_with("unconnected-") {
            n.to_owned()
        } else {
            format!("{:04x}_{n}", hash(seed, n) & 0xffff)
        }
    };
    let new_ref = |r: &str| format!("Q{:04x}_{}", hash(seed ^ 1, r) & 0xffff, r);
    let mut back = BTreeMap::new();
    let mut refs = BTreeMap::new();
    let mut parts = board.parts.clone();
    for part in &mut parts {
        let r = new_ref(&part.reference);
        refs.insert(r.clone(), part.reference.clone());
        part.reference = r;
        for pad in &mut part.pads {
            if let Some(n) = &pad.net {
                let m = new_net(n);
                back.insert(m.clone(), n.clone());
                pad.net = Some(m);
            }
        }
        part.pads.sort_by_key(|p| hash(seed ^ 2, &p.number));
    }
    parts.sort_by_key(|p| hash(seed ^ 3, &p.reference));
    let mut stack = board.stack.clone();
    for (_, n) in &mut stack.planes {
        let m = new_net(n);
        back.insert(m.clone(), n.clone());
        *n = m;
    }
    // The extension (module documentation): the same maps over the stack's other keyed names.
    // Negative control: AET_3400_SKIP_CLASSES=1 leaves the classes' nets unrenamed, so a renamed net
    // loses its class and its width; the comparison must then fail.
    let skip = std::env::var_os("AET_3400_SKIP_CLASSES").is_some();
    for c in stack.classes.iter_mut().filter(|_| !skip) {
        for n in &mut c.nets {
            let m = new_net(n);
            back.insert(m.clone(), n.clone());
            *n = m;
        }
    }
    for p in &mut stack.pairs {
        for n in &mut p.nets {
            let m = new_net(n);
            back.insert(m.clone(), n.clone());
            *n = m;
        }
    }
    for (r, _) in &mut stack.joins {
        *r = new_ref(r);
    }
    for t in &mut stack.thermal {
        t.reference = new_ref(&t.reference);
    }
    (
        Board {
            outline: board.outline,
            parts,
            stack,
        },
        back,
        refs,
    )
}

/// A name `back` lacks is kept as it is, so a name the renaming missed stays renamed and fails the
/// comparison (1307's copy indexes and panics instead; with an empty map this also names `own`).
fn named_back(
    routed: &Routed,
    back: &BTreeMap<String, String>,
    refs: &BTreeMap<String, String>,
) -> Routed {
    let mut out = routed.clone();
    // The second extension: `Routed.arrays` (AET-WO-1309) postdates 1307's copy and carries a
    // reference and a net, so both are put back too.
    for a in &mut out.arrays {
        a.reference = refs.get(&a.reference).unwrap_or(&a.reference).clone();
        a.net = back.get(&a.net).unwrap_or(&a.net).clone();
    }
    out.arrays
        .sort_by(|x, y| (&x.reference, &x.pad).cmp(&(&y.reference, &y.pad)));
    for s in &mut out.segments {
        s.net = back.get(&s.net).unwrap_or(&s.net).clone();
    }
    for v in &mut out.vias {
        v.net = back.get(&v.net).unwrap_or(&v.net).clone();
    }
    for (n, _) in &mut out.zones {
        *n = back.get(n).unwrap_or(n).clone();
    }
    out.segments.sort();
    out.vias.sort();
    out.zones.sort();
    out.ripups = 0;
    out
}

fn length_mm(r: &Routed) -> f64 {
    r.segments
        .iter()
        .map(|s| {
            let (dx, dy) = ((s.end.0 - s.start.0) as f64, (s.end.1 - s.start.1) as f64);
            (dx * dx + dy * dy).sqrt() / 1e6
        })
        .sum()
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let spec: aethelgard_ir::Specification =
        serde_json::from_slice(&fs::read(&a[1]).unwrap()).unwrap();
    let text = fs::read_to_string(&a[2]).unwrap();
    let (board, emitted) = route::read(&strict::board(&text).unwrap(), &Stack::of(&spec)).unwrap();
    let mut own = named_back(&emitted, &BTreeMap::new(), &BTreeMap::new());
    own.zones.sort();
    let o = board.outline;
    println!(
        "board: outline {:.2} x {:.2} mm, {} parts, {} pads; emitted copper: {} segments, {:.1} mm of track, {} vias, {} zones; classes {}, pairs {}, joins {}, thermal {}",
        (o.x1 - o.x0) as f64 / 1e6,
        (o.y1 - o.y0) as f64 / 1e6,
        board.parts.len(),
        board.parts.iter().map(|p| p.pads.len()).sum::<usize>(),
        emitted.segments.len(),
        length_mm(&emitted),
        emitted.vias.len(),
        emitted.zones.len(),
        board.stack.classes.len(),
        board.stack.pairs.len(),
        board.stack.joins.len(),
        board.stack.thermal.len()
    );
    match route::route(&board) {
        Ok(r) => println!(
            "unrenamed reroute: {} segments, {} vias, {} rip-ups, same copper as emitted: {}",
            r.segments.len(),
            r.vias.len(),
            r.ripups,
            named_back(&r, &BTreeMap::new(), &BTreeMap::new()) == own
        ),
        Err(e) => println!("unrenamed reroute: UNROUTABLE {e:?}"),
    }
    let mut pass = 0;
    for seed in 0..20u64 {
        let (b, back, refs) = renamed(&board, seed);
        assert_ne!(b.parts, board.parts, "seed {seed}: nothing renamed");
        match route::route(&b) {
            Ok(r) => {
                let nb = named_back(&r, &back, &refs);
                let same = nb == own;
                println!(
                    "seed {seed:2}: routes, {} rip-ups, own copper: {same}",
                    r.ripups
                );
                if !same && seed == 0 {
                    let only = |a: &Routed, b: &Routed| -> (usize, usize, Vec<String>) {
                        let s: Vec<_> = a
                            .segments
                            .iter()
                            .filter(|x| !b.segments.contains(x))
                            .collect();
                        let v: Vec<_> = a.vias.iter().filter(|x| !b.vias.contains(x)).collect();
                        let mut ex: Vec<String> =
                            s.iter().take(4).map(|x| format!("{x:?}")).collect();
                        ex.extend(v.iter().take(4).map(|x| format!("{x:?}")));
                        (s.len(), v.len(), ex)
                    };
                    println!("  only in renamed: {:?}", only(&nb, &own));
                    println!("  only in emitted: {:?}", only(&own, &nb));
                    println!("  zones renamed {:?} emitted {:?}", nb.zones, own.zones);
                }
                if same {
                    pass += 1;
                }
            }
            Err(e) => println!("seed {seed:2}: UNROUTABLE {e:?}"),
        }
    }
    println!("renamings routed to their own copper: {pass}/20");
}
