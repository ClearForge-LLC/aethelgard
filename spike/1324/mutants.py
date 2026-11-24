"""AET-WO-1324: one mutant per guard the work order adds, each run against the
test that must kill it, in a copy of the tree (never the worktree).

Usage: python3 -I mutants.py <tree copy> <cargo target dir>
Each mutant replaces one exact text (asserted unique) in a fresh copy of the
file, runs its test, and restores the file (copy back, then touch, so cargo
rebuilds: the mutant-restore lesson). KILLED means the test failed.
"""
import os, shutil, subprocess, sys

tree, target = sys.argv[1], sys.argv[2]
R = "crates/aethelgard-eda/src/route.rs"
S = "crates/aethelgard-eda/src/stack.rs"
P = "crates/aethelgard-eda/src/supply.rs"
lib = lambda name: ["cargo", "test", "-q", "-p", "aethelgard-eda", "--lib", "--", f"route::tests::{name}", "--exact"]
cli = lambda name: ["cargo", "test", "-q", "-p", "aethelgard-cli", "--test", "supply_tier1", "--", name, "--exact"]

MUTANTS = [
    # (row offset ja - jb -> jb - ja survives as an equivalent mutant: every
    # Region::new region is symmetric under j -> -j - 1, so the set is the same)
    ("M1 meeting: the merge skips an offset", R, "lo.max(done.saturating_add(1))", "lo.max(done.saturating_add(2))",
     lib("the_meeting_offsets_by_runs_are_every_cell_difference")),
    ("M2 meet keeps (0, 0)", R, "&& (i, j) != (0, 0))", ")",
     lib("two_junction_regions_meet_where_their_cells_do_not_where_a_disc_does")),
    ("M3 junction only on judged nets: every net", R, ".contains(name)", ".contains(name).then_some(true).unwrap_or(true)",
     lib("two_junction_regions_meet_where_their_cells_do_not_where_a_disc_does")),
    ("M4 via_open ignores the junction", R, "&& !self.junction[net].as_ref().is_some_and(|j| j.near[node]))", ")",
     lib("a_judged_nets_via_stays_out_of_its_laid_vias_regions")),
    ("M5 claim_via marks nothing", R, "j.near[m] = true;", "let _ = (m, &j);",
     lib("a_judged_nets_via_stays_out_of_its_laid_vias_regions")),
    ("M6 remap keeps the laid vias' marks", R, "j.near.clone_from(&j.fixed);", "let _ = &j;",
     lib("a_judged_nets_via_stays_out_of_its_laid_vias_regions")),
    ("M7 close_vias ignores the junction", R, "|| meet.is_some_and(|m| m.contains(&((x - px) / PITCH_NM, (y - py) / PITCH_NM)))", "|| meet.is_some_and(|_| false)",
     lib("a_paths_own_vias_whose_regions_meet_are_a_close_pair")),
    ("M8 through-hole pads hold no region", R, "            .filter(|p| p.net.as_deref() == Some(net))\n            .filter_map(|p| {", "            .filter(|_| false)\n            .filter_map(|p| {",
     lib("a_judged_nets_through_hole_pad_and_array_via_hold_their_regions")),
    ("M9 array vias hold no region", R, "            .filter(|a| a.net == net)\n            .flat_map(|a| a.at.iter()", "            .filter(|_| false)\n            .flat_map(|a| a.at.iter()",
     lib("a_judged_nets_through_hole_pad_and_array_via_hold_their_regions")),
    ("M10 index compares distance only", R, "if d < *b {", "if d.0 < b.0 {",
     lib("the_nearest_index_picks_what_the_whole_scan_picked_ties_included")),
    ("M11 index tie-break: last pad", R, ".min_by_key(|&(k, &(d, _))| (d, k))", ".min_by_key(|&(k, &(d, _))| (d, std::cmp::Reverse(k)))",
     lib("the_nearest_index_picks_what_the_whole_scan_picked_ties_included")),
    ("M12 seeds uncharged", R, "self.work += sources.len() as u64;", "let _ = sources.len();",
     lib("a_search_charges_every_source_it_seeds")),
    ("M13 index updates uncharged", R, "self.work += nearest.add(&self.grid, &grown);", "nearest.add(&self.grid, &grown);",
     lib("one_net_of_1600_pads_is_charged_its_quadratic_work")),
    ("M14 picks uncharged", R, "            self.work += work;\n            if self.over() {\n                return Err(Stuck::Work);", "            let _ = work;\n            if self.over() {\n                return Err(Stuck::Work);",
     lib("one_net_of_1600_pads_is_charged_its_quadratic_work")),
    ("M15 final check uncharged", R, "r.work += judge_work(board, &routed);", "let _ = judge_work(board, &routed);",
     lib("the_final_check_is_charged_its_pairs")),
    ("M16 rule leaves GND out", S, "        nets.push(crate::route::GROUND.to_owned());\n", "",
     cli("the_routers_junction_rule_is_on_where_the_gate_runs")),
    ("M17 rule on with an empty limit list", S, "        if limits.is_empty() {\n            return None;\n        }\n        let st", "        let st",
     cli("the_routers_junction_rule_is_on_where_the_gate_runs")),
    ("M18 reach one short", P, "+ hole_radius(0, hole_offset_nm) + 1\n", "+ hole_radius(0, hole_offset_nm)\n",
     lib("two_junction_regions_meet_where_their_cells_do_not_where_a_disc_does")),
    ("M19 the rule is never built", S, "            junction: JunctionRule::of(spec),", "            junction: None,",
     cli("the_two_layer_power_fixture_on_the_pc0_sense_keeps_its_vias_regions_apart")),
    ("M20 unhop flattens a pair with a via between", R, "            if path[j].0 != outer {\n                self.banned.insert(path[j].1);\n                return None;\n            }\n", "",
     lib("a_close_pair_with_a_via_between_is_banned_not_flattened")),
    ("M21 meeting runs: one end wrong", R, ".push((lo_a - hi_b, hi_a - lo_b));", ".push((lo_a - hi_b, hi_a - hi_b));",
     lib("the_meeting_offsets_by_runs_are_every_cell_difference")),
    ("M22 rule ignores the gate's other inputs", S, "            read.as_ref()?;", "            let _ = read;",
     cli("the_routers_junction_rule_is_on_where_the_gate_runs")),
]

only = set(sys.argv[3].split(",")) if len(sys.argv) > 3 else None
env = dict(os.environ, CARGO_TARGET_DIR=target)
env["PATH"] = os.path.expanduser("~/.cargo/bin") + ":" + env["PATH"]
for name, rel, old, new, cmd in MUTANTS:
    if only and name.split()[0] not in only:
        continue
    path = os.path.join(tree, rel)
    src = open(path).read()
    assert src.count(old) == 1, (name, src.count(old))
    shutil.copy(path, path + ".orig")
    open(path, "w").write(src.replace(old, new))
    try:
        r = subprocess.run(cmd, cwd=tree, env=env, capture_output=True, text=True)
        out = r.stdout + r.stderr
        built = "error[" not in out and "could not compile" not in out
        ran = "running 1 test" in out
        verdict = ("KILLED" if r.returncode != 0 else "SURVIVED") if built and ran else ("NOBUILD" if not built else "NOTRUN")
        print(f"{verdict}\t{name}", flush=True)
    finally:
        shutil.move(path + ".orig", path)
        os.utime(path)
