//! `measure <runs> <spec.json>...`: for each specification and each pruning,
//! solve `runs` times and print timings, node counts, the output's sha256 and
//! whether every run gave the same bytes. Times are probe-side only.

use std::time::{Duration, Instant};

use aethelgard_spike_solver::{Pruning, Stats, sha256_hex, solve};

fn main() {
    let mut args = std::env::args().skip(1);
    let runs: usize = args.next().and_then(|n| n.parse().ok()).expect("runs");
    // PRUNING=fc,im,pm+memo: a comma list of settings, each `+`-joined flags.
    let spec = std::env::var("PRUNING").unwrap_or_else(|_| "fc,im".into());
    let prunings: Vec<Pruning> = spec
        .split(',')
        .map(|one| {
            let flags: Vec<&str> = one.split('+').collect();
            Pruning {
                instance_matching: flags.contains(&"im"),
                pin_matching: flags.contains(&"pm"),
                memo: flags.contains(&"memo"),
                node_limit: std::env::var("NODE_LIMIT")
                    .ok()
                    .and_then(|n| n.parse().ok())
                    .unwrap_or(0),
            }
        })
        .collect();
    let mut loads: Vec<Duration> = (0..21)
        .map(|_| {
            let t = Instant::now();
            let _ = aethelgard_svd_hal::pin_table("STM32H753ZITx").expect("table");
            t.elapsed()
        })
        .collect();
    loads.sort();
    println!(
        "# pin_table(\"STM32H753ZITx\") load, hash check and parse, 21 runs: min={:.1}us median={:.1}us (not inside the solve times below)",
        loads[0].as_secs_f64() * 1e6,
        loads[10].as_secs_f64() * 1e6
    );
    let table = aethelgard_svd_hal::pin_table("STM32H753ZITx").expect("table");
    for path in args {
        let bytes = std::fs::read(&path).expect("read");
        for &pruning in &prunings {
            let mut times = Vec::with_capacity(runs);
            let mut outputs = Vec::with_capacity(runs);
            let mut stats = Stats::default();
            for _ in 0..runs {
                stats = Stats::default();
                let t = Instant::now();
                let r = solve(&bytes, &table, pruning, &mut stats);
                times.push(t.elapsed());
                outputs.push(match r {
                    Ok(topo) => serde_json::to_string(&topo).expect("serialize"),
                    Err(e) => format!("REFUSED {}: {}", e.kind(), e.message()),
                });
            }
            times.sort();
            let identical = outputs.windows(2).all(|w| w[0] == w[1]);
            let us = |d: Duration| d.as_secs_f64() * 1e6;
            println!(
                "{path} {}: runs={runs} min={:.1}us median={:.1}us max={:.1}us nodes={} searches={} identical={identical} out_sha256={}",
                pruning.label(),
                us(times[0]),
                us(times[runs / 2]),
                us(times[runs - 1]),
                stats.nodes,
                stats.searches,
                sha256_hex(outputs[0].as_bytes()),
            );
            println!("  {}", outputs[0]);
        }
    }
}
