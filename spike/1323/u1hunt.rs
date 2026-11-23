//! AET-WO-1323 §1.1 (2), U1-c: a board that bans alone fails and U1 routes.
//! Each small committed board, emitted, read back as `route::Board`, its
//! parts of at most 8 pads shifted by a seeded offset as `escape_tier1`'s
//! `shifted_placements…` does; each routed with U1 and with bans alone
//! (`route::BANS_ONLY`, a spike-only switch the prototype's `unhop` reads). Dropped into a tree
//! copy's `crates/aethelgard-cli/tests/`; never committed so.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use aethelgard_eda::route::{self, Board};
use aethelgard_eda::stack::Stack;
use aethelgard_eda::strict;
use serde_json::Value;

fn root() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

fn emitted(label: &str, spec: &str) -> Board {
    let dir = std::env::temp_dir().join(format!("aet-1323-hunt-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args([
            "emit",
            "--file",
            spec,
            "--out",
            dir.join("out").to_str().unwrap(),
        ])
        .current_dir(root())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let text = fs::read_to_string(v["board"]["path"].as_str().unwrap()).unwrap();
    let bytes = fs::read(root().join(spec)).unwrap();
    let stack = Stack::of(&serde_json::from_slice::<aethelgard_ir::Specification>(&bytes).unwrap())
        .unwrap();
    fs::remove_dir_all(&dir).unwrap();
    route::read(&strict::board(&text).unwrap(), &stack)
        .unwrap()
        .0
}

fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn shifted(board: &Board, seed: u64) -> Board {
    let mut b = board.clone();
    let mut z = seed * 7919 + 1;
    for p in &mut b.parts {
        z = mix(z);
        if p.pads.len() > 8 {
            continue;
        }
        let dx = ((z % 9) as i64 - 4) * 125_000;
        let dy = (((z >> 16) % 9) as i64 - 4) * 125_000;
        for pad in &mut p.pads {
            pad.rect = aethelgard_eda::pcb::Rect {
                x0: pad.rect.x0 + dx,
                y0: pad.rect.y0 + dy,
                x1: pad.rect.x1 + dx,
                y1: pad.rect.y1 + dy,
            };
        }
        for e in &mut p.courtyard {
            *e = ((e.0.0 + dx, e.0.1 + dy), (e.1.0 + dx, e.1.1 + dy));
        }
        for k in &mut p.keepouts {
            k.rect = aethelgard_eda::pcb::Rect {
                x0: k.rect.x0 + dx,
                y0: k.rect.y0 + dy,
                x1: k.rect.x1 + dx,
                y1: k.rect.y1 + dy,
            };
        }
    }
    b
}

fn outcome(b: &Board) -> String {
    match route::route(b) {
        Ok(_) => "ok".into(),
        Err(e) => format!(
            "refused {}: {}",
            e.net,
            e.why.chars().take(100).collect::<String>()
        ),
    }
}

/// AET1323_HUNT=<spec>,<first seed>,<seeds>
#[test]
#[ignore]
fn hunt() {
    let arg = std::env::var("AET1323_HUNT").unwrap();
    let v: Vec<&str> = arg.split(',').collect();
    let (spec, first, n): (&str, u64, u64) = (v[0], v[1].parse().unwrap(), v[2].parse().unwrap());
    let board = emitted("b", spec);
    let (mut same, mut differ) = (0, 0);
    for seed in first..first + n {
        let b = shifted(&board, seed);
        use std::sync::atomic::Ordering::SeqCst;
        route::BANS_ONLY.store(false, SeqCst);
        let u1 = outcome(&b);
        route::BANS_ONLY.store(true, SeqCst);
        let bans = outcome(&b);
        route::BANS_ONLY.store(false, SeqCst);
        if u1 == bans {
            same += 1;
        } else {
            differ += 1;
            println!("{spec} seed {seed}: U1 {u1} | bans {bans}");
        }
    }
    println!("{spec}: {same} same outcome, {differ} differ");
}
