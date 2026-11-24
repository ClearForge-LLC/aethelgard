"""AET-WO-1324 §1.1 (2): a timing shim applied to a tree copy, never committed.

Usage: python3 -I inst.py <tree>

Adds thread-local nanosecond timers to route.rs around:
  SCAN  - route_net's nearest-pad choice (every remaining pad against the tree);
  SEED  - building `sources` from the tree in route_net, plus each search's
          loop that prices and pushes every source;
  JUDGE - the final check, connectivity and pairs in route_measured;
  ROUTE - the whole route_measured call;
and counters: SCAN_PAIRS (pad x tree-node distance evaluations) and SEEDS
(sources priced). It adds no branch that changes a search, a cost or an
emitted byte. It also drops route/probe1324.rs in as a test module.
"""
import sys, shutil, os

tree = sys.argv[1]
here = os.path.dirname(os.path.abspath(__file__))


def edit(rel, pairs):
    p = f"{tree}/{rel}"
    s = open(p).read()
    for old, new in pairs:
        assert s.count(old) == 1, (rel, old[:80], s.count(old))
        s = s.replace(old, new)
    open(p, "w").write(s)


MOD = r'''
/// AET-WO-1324 measuring shim (never committed).
pub mod spike_time {
    use std::cell::Cell;
    thread_local! {
        pub static SCAN: Cell<u128> = const { Cell::new(0) };
        pub static SEED: Cell<u128> = const { Cell::new(0) };
        pub static JUDGE: Cell<u128> = const { Cell::new(0) };
        pub static ROUTE: Cell<u128> = const { Cell::new(0) };
        pub static SCAN_PAIRS: Cell<u64> = const { Cell::new(0) };
        pub static SEEDS: Cell<u64> = const { Cell::new(0) };
        pub static INDEX: Cell<u64> = const { Cell::new(0) };
    }
    pub fn add(c: &'static std::thread::LocalKey<Cell<u128>>, t: std::time::Instant) {
        c.with(|x| x.set(x.get() + t.elapsed().as_nanos()));
    }
    pub fn count(c: &'static std::thread::LocalKey<Cell<u64>>, n: u64) {
        c.with(|x| x.set(x.get() + n));
    }
    pub fn reset() {
        for c in [&SCAN, &SEED, &JUDGE, &ROUTE] { c.with(|x| x.set(0)); }
        for c in [&SCAN_PAIRS, &SEEDS, &INDEX] { c.with(|x| x.set(0)); }
    }
}
'''

# With AET1324_LOG set, each route_measured call appends one line: rung,
# outline, parts, outcome, counted work, scan pairs, seeds, and the
# nanosecond timers.
LOGGED = r'''pub fn route_measured(board: &Board, rung: Rung, limit: u64) -> Result<(Routed, u64), Unroutable> {
    spike_time::reset();
    let r = route_measured_inner(board, rung, limit);
    if let Some(p) = std::env::var_os("AET1324_LOG") {
        use std::io::Write;
        let get = |c: &'static std::thread::LocalKey<std::cell::Cell<u128>>| c.with(|x| x.get());
        let cnt = |c: &'static std::thread::LocalKey<std::cell::Cell<u64>>| c.with(|x| x.get());
        let o = board.outline;
        let (outcome, work) = match &r {
            Ok((_, w)) => ("ok".to_owned(), *w),
            Err(e) => (format!("err:{}", e.net), 0),
        };
        let line = format!(
            "{}/{}\t{}x{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            rung.bounded_via_cost, rung.bounded_first,
            (o.x1 - o.x0) / 1_000_000, (o.y1 - o.y0) / 1_000_000,
            board.parts.len(), outcome, work,
            cnt(&spike_time::SCAN_PAIRS), cnt(&spike_time::SEEDS),
            cnt(&spike_time::INDEX),
            get(&spike_time::ROUTE) / 1_000_000, get(&spike_time::SCAN) / 1_000_000,
            get(&spike_time::SEED) / 1_000_000, get(&spike_time::JUDGE) / 1_000_000,
        );
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(p).unwrap();
        f.write_all(line.as_bytes()).unwrap();
    }
    r
}

'''

R = "crates/aethelgard-eda/src/route.rs"
edit(R, [
    ("mod read;\n", "mod read;\n" + MOD + "#[cfg(test)]\nmod probe1324;\n"),
    # the scan
    ("            let (k, (_, from)) = remaining\n                .iter()\n",
     "            let t_scan = std::time::Instant::now();\n"
     "            spike_time::count(&spike_time::SCAN_PAIRS, (remaining.len() * tree.len()) as u64);\n"
     "            let (k, (_, from)) = remaining\n                .iter()\n"),
    ("                .ok_or_else(|| Stuck::Unroutable(\"no pad left\".into()))?;\n",
     "                .ok_or_else(|| Stuck::Unroutable(\"no pad left\".into()))?;\n"
     "            spike_time::add(&spike_time::SCAN, t_scan);\n"),
    # an incremental index's updates: each node added to the tree, against
    # every pad still remaining
    ("            tree.extend(states);\n        }\n        Ok(())\n",
     "            tree.extend(states);\n"
     "            spike_time::count(&spike_time::INDEX, ((tree.len() - before) * remaining.len()) as u64);\n"
     "        }\n        Ok(())\n"),
    ("            self.commit(net, &path);\n            tree.extend(path.iter().copied());\n",
     "            self.commit(net, &path);\n            let before = tree.len();\n            tree.extend(path.iter().copied());\n"),
    # the sources vector
    ("            let sources: Vec<Node> = tree.iter().copied().collect();\n            let mut window_ends",
     "            let t_src = std::time::Instant::now();\n"
     "            let sources: Vec<Node> = tree.iter().copied().collect();\n"
     "            spike_time::add(&spike_time::SEED, t_src);\n"
     "            let mut window_ends"),
    # the seed loop in search
    ("        let mut heap = BinaryHeap::new();\n        // A source is priced",
     "        let mut heap = BinaryHeap::new();\n"
     "        let t_seed = std::time::Instant::now();\n"
     "        spike_time::count(&spike_time::SEEDS, sources.len() as u64);\n"
     "        // A source is priced"),
    ("        let (wx0, wy0, wx1, wy1) = window;\n        let mut moves",
     "        spike_time::add(&spike_time::SEED, t_seed);\n"
     "        let (wx0, wy0, wx1, wy1) = window;\n        let mut moves"),
    # judges
    ("    check(board, &routed).map_err(|why| fail(\"\", why))?;\n    connectivity(board, &routed).map_err(|why| fail(\"\", why))?;\n    pairs(board, &routed).map_err(|why| fail(\"\", why))?;\n    Ok((routed, r.work))\n",
     "    let t_judge = std::time::Instant::now();\n"
     "    check(board, &routed).map_err(|why| fail(\"\", why))?;\n    connectivity(board, &routed).map_err(|why| fail(\"\", why))?;\n    pairs(board, &routed).map_err(|why| fail(\"\", why))?;\n"
     "    spike_time::add(&spike_time::JUDGE, t_judge);\n"
     "    spike_time::add(&spike_time::ROUTE, t_route);\n"
     "    Ok((routed, r.work))\n"),
    ("pub fn route_measured(board: &Board, rung: Rung, limit: u64) -> Result<(Routed, u64), Unroutable> {\n",
     LOGGED + "fn route_measured_inner(board: &Board, rung: Rung, limit: u64) -> Result<(Routed, u64), Unroutable> {\n"
     "    let t_route = std::time::Instant::now();\n"),
])
shutil.copy(f"{here}/probe1324.rs", f"{tree}/crates/aethelgard-eda/src/route/probe1324.rs")
print("ok")
