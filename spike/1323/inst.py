"""AET-WO-1323 §1.1 instrumentation: a measuring shim applied to a tree copy, never committed.

Usage: python3 -I inst.py <tree>

route.rs: thread-local counters of the router's work in one `route_with` call
  - `search` pops: each A* state `search()` expands (popped and not stale);
  - `pair` pops: each state `pair_run()` expands;
  - `flood` pops: each node `zone_blockers` and `components` visit;
  and the call's wall time and the time spent inside `search()`. With
  AET1323_WORK_LOG set, each `route_with` call appends one tab-separated line:
  thread name (libtest's test name), rung, outline in mm, parts, nets,
  outcome, search, pair, flood, wall ms, search ms, ripups, why (first 160 bytes).

supply.rs / cli extract.rs: with AET1323_TIME_LOG set, each phase's wall time.
The shim adds no branch that changes a search, a cost or an emitted byte.
"""
import sys

tree = sys.argv[1]


def edit(rel, pairs):
    p = f"{tree}/{rel}"
    s = open(p).read()
    for old, new in pairs:
        assert s.count(old) == 1, (rel, old[:80], s.count(old))
        s = s.replace(old, new)
    open(p, "w").write(s)


ROUTE_MOD = r'''
/// AET-WO-1323 measuring shim (never committed).
pub mod spike_work {
    use std::cell::Cell;
    thread_local! {
        pub static SEARCH: Cell<u64> = const { Cell::new(0) };
        pub static PAIR: Cell<u64> = const { Cell::new(0) };
        pub static FLOOD: Cell<u64> = const { Cell::new(0) };
        pub static SEARCH_NS: Cell<u128> = const { Cell::new(0) };
    }
    pub fn bump(c: &'static std::thread::LocalKey<Cell<u64>>) {
        c.with(|v| v.set(v.get() + 1));
    }
    pub fn log(line: &str) {
        if let Some(p) = std::env::var_os("AET1323_WORK_LOG") {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new().create(true).append(true).open(p).unwrap();
            f.write_all(line.as_bytes()).unwrap();
        }
    }
}
'''

edit("crates/aethelgard-eda/src/route.rs", [
    ("/// The grid's pitch.\npub const PITCH_NM: i64 = 125_000;",
     ROUTE_MOD + "\n/// The grid's pitch.\npub const PITCH_NM: i64 = 125_000;"),
    # search(): an expansion
    ("            if f != c + h(node) {\n                continue;\n            }\n            if goal(l, node) {",
     "            if f != c + h(node) {\n                continue;\n            }\n            spike_work::bump(&spike_work::SEARCH);\n            if goal(l, node) {"),
    # search(): its time
    ("        soft: bool,\n    ) -> Option<Vec<Node>> {\n        let n = self.grid.len();",
     "        soft: bool,\n    ) -> Option<Vec<Node>> {\n        let t0 = std::time::Instant::now();\n        let r = self.search_inner(net, sources, goal, h, b_moves, window, soft);\n        spike_work::SEARCH_NS.with(|v| v.set(v.get() + t0.elapsed().as_nanos()));\n        r\n    }\n\n    #[allow(clippy::too_many_arguments)]\n    fn search_inner(\n        &mut self,\n        net: usize,\n        sources: &[Node],\n        goal: &dyn Fn(Layer, usize) -> bool,\n        h: &dyn Fn(usize) -> u32,\n        b_moves: bool,\n        window: (usize, usize, usize, usize),\n        soft: bool,\n    ) -> Option<Vec<Node>> {\n        let n = self.grid.len();"),
    # pair_run(): an expansion
    ("        while let Some(Reverse((f, done, s, node))) = heap.pop() {\n",
     "        while let Some(Reverse((f, done, s, node))) = heap.pop() {\n            spike_work::bump(&spike_work::PAIR);\n"),
    # zone_blockers' flood
    ("            while let Some(n) = stack.pop() {\n                if std::mem::replace(&mut seen[n], true) {",
     "            while let Some(n) = stack.pop() {\n                spike_work::bump(&spike_work::FLOOD);\n                if std::mem::replace(&mut seen[n], true) {"),
    # components' flood
    ("            while let Some(n) = stack.pop() {\n                for d in 0..4 {\n                    if let Some(m) = grid.step(n, d)\n                        && core[m]",
     "            while let Some(n) = stack.pop() {\n                spike_work::bump(&spike_work::FLOOD);\n                for d in 0..4 {\n                    if let Some(m) = grid.step(n, d)\n                        && core[m]"),
    # route_with: wrap
    ("pub fn route_with(board: &Board, rung: Rung) -> Result<Routed, Unroutable> {\n",
     r'''pub fn route_with(board: &Board, rung: Rung) -> Result<Routed, Unroutable> {
    for c in [&spike_work::SEARCH, &spike_work::PAIR, &spike_work::FLOOD] {
        c.with(|v| v.set(0));
    }
    spike_work::SEARCH_NS.with(|v| v.set(0));
    let t0 = std::time::Instant::now();
    let r = route_with_inner(board, rung);
    let wall = t0.elapsed().as_millis();
    let get = |c: &'static std::thread::LocalKey<std::cell::Cell<u64>>| c.with(|v| v.get());
    let nets = net_pads(board).len();
    let (outcome, ripups, why) = match &r {
        Ok(routed) => ("ok", routed.ripups.to_string(), String::new()),
        Err(e) => ("err", "-".to_owned(), {
            let mut w = format!("{}: {}", e.net, e.why).replace(['\t', '\n'], " ");
            if w.len() > 160 {
                let mut k = 160;
                while !w.is_char_boundary(k) {
                    k -= 1;
                }
                w.truncate(k);
            }
            w
        }),
    };
    let o = board.outline;
    spike_work::log(&format!(
        "{}\t{}/{}\t{}x{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
        std::thread::current().name().unwrap_or("-"),
        rung.bounded_via_cost,
        rung.bounded_first,
        (o.x1 - o.x0) / 1_000_000,
        (o.y1 - o.y0) / 1_000_000,
        board.parts.len(),
        nets,
        outcome,
        get(&spike_work::SEARCH),
        get(&spike_work::PAIR),
        get(&spike_work::FLOOD),
        wall,
        spike_work::SEARCH_NS.with(|v| v.get()) / 1_000_000,
        ripups,
        why
    ));
    r
}

fn route_with_inner(board: &Board, rung: Rung) -> Result<Routed, Unroutable> {
'''),
])

SUPPLY_LOG = r'''
/// AET-WO-1323 measuring shim (never committed).
pub fn spike_time(what: &str, t0: std::time::Instant) {
    if let Some(p) = std::env::var_os("AET1323_TIME_LOG") {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(p).unwrap();
        writeln!(f, "{}\t{what}\t{}", std::process::id(), t0.elapsed().as_micros()).unwrap();
    }
}
'''

edit("crates/aethelgard-eda/src/supply.rs", [
    ("pub fn supplies(\n", SUPPLY_LOG + "\npub fn supplies(\n"),
    # copper per net
    ("            let nc = copper(board, &read.routed, net, &terminals, &model).map_err(to_refused)?;\n            let notes",
     "            let tc = std::time::Instant::now();\n            let nc = copper(board, &read.routed, net, &terminals, &model).map_err(to_refused)?;\n            spike_time(&format!(\"copper {net}\"), tc);\n            let notes"),
    # bound: build vs solve
    ("    let h = nc.h_nm;\n    let mut net = Network::default();",
     "    let tb = std::time::Instant::now();\n    let h = nc.h_nm;\n    let mut net = Network::default();"),
    ("    let b = net\n        .bound(src, snk, max_iter, 1e-10)\n        .map_err(Refusal::Flow)?;",
     "    spike_time(&format!(\"build {}\", sink.number), tb);\n    let ts = std::time::Instant::now();\n    let b = net\n        .bound(src, snk, max_iter, 1e-10)\n        .map_err(Refusal::Flow)?;\n    spike_time(&format!(\"solve {}\", sink.number), ts);"),
])

edit("crates/aethelgard-cli/src/extract.rs", [
    ("pub fn extract(board_path: &Path, spec_path: &Path) -> ExitCode {\n    let (spec, spec_sha256)",
     "pub fn extract(board_path: &Path, spec_path: &Path) -> ExitCode {\n    let t_all = std::time::Instant::now();\n    let r = extract_inner(board_path, spec_path);\n    aethelgard_eda::supply::spike_time(\"extract total\", t_all);\n    r\n}\n\nfn extract_inner(board_path: &Path, spec_path: &Path) -> ExitCode {\n    let t_pre = std::time::Instant::now();\n    let (spec, spec_sha256)"),
    ("    let bounded = bound::read(text, &stack)\n        .and_then(|read| bound::bound(&read, &wanted).map(|lines| (read, lines)));",
     "    aethelgard_eda::supply::spike_time(\"spec, solve, lines\", t_pre);\n    let t_b = std::time::Instant::now();\n    let bounded = bound::read(text, &stack)\n        .and_then(|read| bound::bound(&read, &wanted).map(|lines| (read, lines)));\n    aethelgard_eda::supply::spike_time(\"read and capacitance bound\", t_b);\n    let t_s = std::time::Instant::now();"),
    ("        Err(r) => return refuse(r.kind, r.message, EXIT_INVALID_SPEC),\n    };\n    // The oscillator nets",
     "        Err(r) => return refuse(r.kind, r.message, EXIT_INVALID_SPEC),\n    };\n    aethelgard_eda::supply::spike_time(\"supplies\", t_s);\n    // The oscillator nets"),
])
print("shim applied to", tree)
