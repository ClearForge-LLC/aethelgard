# AET-WO-1323 — FEEDBACK: the router's backlog (measured, stopped at §1.2, ruled, built)

**Branch:** `wo/aet-1323-router-backlog` from `main` `cd79024c`.
**Builder:** Claude (Opus 5.5). Every measurement here was taken by me on Furnace; no worker's report is relied on. Written 2026-11-23.

**Status: built to the ruling** (`~/.cache/aet-bus/1323.md`, 2026-11-23 11:00Z), §5 onward. §0–§4 are the measurement and the stop as they stood at `f0997fc2`; the measurement is `spike/1323/RESULTS.md`, beside `PREDICTIONS.md` (committed at `456e0f48`, before any run).

**The answer at the stop, in six lines:**
- **No `fab/` byte would move** under any of the five. `examples/sentinel-01.json`, the package's specification, emits byte-identical under U1, under the pair fix, and under the keepout rule.
- **None of §7's other stops fires.** Every vendored keepout lies inside its own courtyard, and (5) can be cut without changing what the gate proves, but only by parallelism.
- **(1):** the heaviest tier-1 routing in the workspace suite does 142.9 M units of work, and 148.7 M under U1. U1's killer test would do 170.1 M. I propose a bound of **350 000 000** per routing if that killer is built, **300 000 000** if not, refused as **`RouterWorkExceeded`**. The adversary (a reconstruction) crosses them in 51 s and 37.5 s, where it took 203 s to `RIPUPS`.
- **(2) U1 moves 4 committed boards' `.kicad_pcb`** (d1, d3, Sentinel-03, `solve/stress`) and 3 of `d1319`'s drawn boards. Nothing else moves. It is a trade, not a strict gain: on shifted d3 boards it wins one seed and loses another.
- **(4) is `route_pair`'s own:** the breakout walks back along its own coupled run. Pruning the leaves clears all 152 stubs of a 1 232-placement sweep. No committed byte moves.
- **(5) is 86 % solve.** The time is CG (47 %) and the interval energy of each via's 1 024 hole-position boxes (35 %), per pin, 27 pins. Nothing in it recurs, so the only cut that keeps every bit is threads.

## 0. How it was made

- **Trees.** The base is a `git archive` of `cd79024c`. Each prototype is that archive plus one patch, built in its own target dir:
  - `u1`: (2);
  - `s4`: (4), switched by environment;
  - `k3`: (3);
  - `inst`: counters and timers only, `spike/1323/inst.py`, which emits all 75 files byte-identical to base;
  - `all`: shim + U1 + prune + the early pair judge, run as the workspace suite.

  Evidence is in `~/.cache/aet-1323/`.
- **Harnesses.**
  - `spike/1319/harness/run.sh` and `compare.sh`, unchanged: the sweep of 34 specifications, `d4diff` and `d1319`.
  - `spike/1323/work-sweep.sh`: a work log per specification.
  - `adv1307.rs`: the `1307` A4 adversary, **reconstructed**, because the original was not kept.
  - `stub1323.rs`: F2 and a sweep of 1 232 placements.
  - `u1hunt.rs`: shifted placements, U1 against bans alone.
- **Heavy runs** went in capped user scopes (`MemoryMax`, no swap), at `nice -n 10`, 6 test threads at most.
- **`perf` is denied on Furnace** (`perf_event_paranoid` 4), so (5)'s split is by timers.

## 1. What was measured, per piece

Each piece in one paragraph; the tables are in RESULTS.

**(1) The work bound** (RESULTS §2).
- The counter: A* expansions in `search()` (states popped and not stale), plus the nodes the fill floods visit (`zone_blockers`, `components`), summed over one `route_with` call.
- `search` holds 64–92 % of a committed routing's wall time and 90 % of the adversary's; the floods hold most of the rest.
- The count is a function of the board alone: equal in release and dev, and on every repeat.
- The heaviest committed specification is `sentinel-02-power`: 86.4 M search + 32.6 M flood = 119.0 M, in 11.6 s.
- The heaviest routing in tier-1 `cargo test --workspace` (701 routings logged) is `power_board_tier1`'s `power_tree()` fixture through the ladder, rung 320/false: 118.7 M + 24.1 M = **142.9 M**. Under U1 it is a 204 × 129 mm board, at 148.7 M. Tier 2 is unmeasured.
- The suite's refused routings are tiny (≤ 146 k), so no such bound changes an asserted refusal.
- The adversary, a reconstruction (seed 1307: a 249 mm board, 4 LQFP-144, 288 random two-pad nets), fails by `RIPUPS` after 203 s and 353 MB, at 1 370.8 M. `1307` measured 616 s on its own board.

**(2) U1** (RESULTS §3).
- The prototype: a soft path's flattened hop passes where the hard maps are open, as the soft search judged it, with `BANS` at 16.
- It moves `d1-24-i2c`, `d3-24-i2c-12-spi`, `sentinel-03` and `solve/stress`, each `.kicad_pcb` only, plus `d1319`'s `d1-own-M1_OUT1-U2_VCP`, `d1-own-M1_OUT2-U2_CPL` and `d2-two`. `d4diff` is identical over 277 cases.
- No exit code moves.
- Shifted placements: the reference board, d1, `four-layer` and Sentinel-01 never differ (180 seeds). On d3, seed 13 routes only under U1 (the mutant's killer), and seed 16 routes only under bans alone.

**(3) The keepout** (RESULTS §4).
- Checked through the real reader three ways, with every byte and exit code identical to base:
  - `emit` of all 34 specifications, since `emit` builds parts through `route::part_of`;
  - `extract` on the 7 boards it reads that place a keepout footprint;
  - `emit` with the IMU turned 0°, 90°, 180° and 270°.

  Every keepout lies inside its courtyard.
- The B.Cu keepouts of the TDK LGA-14 and the Bosch LGA-10 lie outside the pad field, so the rule is "courtyard or field". `extract` is identical to base on all 8.
- The QFN-24's keepout is read from its footprint only; no emitting board places it.

**(4) The stub** (RESULTS §5).
- F2 reproduces on base. `route_pair` lays the stub on the bare board.
  - The `/N` breakout cannot climb the run's own first column, because `/P`'s breakout is too close.
  - So it joins the run three pitches in, and walks back along its own copper to the run's first node, the search's only goal. The run's head is left a leaf.
- Sweep of 1 232 `S` placements:

  | | route | stub | refused |
  |---|---|---|---|
  | base | 788 | 152 | 292 |
  | prune | 907 | 0 | 325 |

  Prune takes off every leaf of a pair conductor's copper that lies in none of its pads, then rebuilds the maps as `rip` does.
- Of the 152 former stubs, 119 route and 33 are refused by the pair judge.
- Run on the pairs' own copper straight after the pair loop, that judge refuses those 33 by name **before** `connectivity`.
- The whole sweep under both emits all 75 files byte-identical to base, `pair-standin.json` (the only committed specification with a pair) among them.

**(5) The supply gate** (RESULTS §6).
- `extract` on `s024l` takes 278–303 s release (568 MB); the test-side `emit` takes 0.98 s. Of that:
  - `copper()` 31.6 s;
  - network builds 10.8 s;
  - solves 260 s, of which CG is 138 s (124 iterations, 819 603 free nodes, per pin) and the interval energy 101 s. 98 % of the energy is the `Group` elements: 35 vias × 1 024 boxes × 336 cells × 4 corners, per pin.
- The 60 and 165 mV runs do the same work.
- No operand recurs: every (column, width) pair of a via's boxes is distinct. Neither pins nor boxes share solved state.

## 2. Predictions beside results

| | prediction | result |
|---|---|---|
| B1–B3 | 34 / 25 / 9; harness unchanged; a hash per file | **hit** |
| W1 | expansions per routing | **hit**; floods added to the unit (D1) |
| W2 | `search` > 80 % on the adversary | **hit**, 90 %; 64–92 % on committed boards |
| W3 | count equal in debug, release, repeats | **hit** |
| W4 | heaviest committed: Sentinel-03 or d3, ~10⁸ | **wrong in order** (`sentinel-02-power`), magnitude hit |
| W5 | suite's heaviest within 2× of W4 | **hit**, 1.37× |
| W6 | adversary ≥ 10× | **hit**, 15.9× |
| U1-a | d3 alone moves; Sentinel-01 identical | **half**: Sentinel-01 identical; d1, Sentinel-03, `solve/stress` move too |
| U1-b | no board flips | **hit** among committed boards |
| U1-c | maybe no killer | **wrong**: d3 seed 13; and U1 loses d3 seed 16 |
| S1–S2 | F2 reproduces; `route_pair` lays it | **hit** |
| S3 | a minority stub | **hit**, 152 / 1 232 |
| S4 | only `pair-standin` can move; it does not | **hit** |
| P1 | solve > 70 % | **hit**, 86 % |
| P2 | `emit` a minority | **hit**, 0.98 s of ~290 |
| P3 | shared work across pins of a net | **wrong**: nothing recurs; only parallelism keeps every bit |

## 3. Decisions for the architect

**D1 · (1) the bound, its unit and its name.**
- **Unit:** the router's work in one routing (`route_with`), counted as search expansions plus fill-flood nodes. It is a count, never a clock, so a run's answer cannot depend on the machine.
  - `pair_run`'s expansions are left out: each call is capped at `4 × MAX_NODES` states, and it runs at most twice per pair, before the rip-up loop (24 k on `pair-standin`).
  - Counting search alone (the alternative) leaves the floods unbounded except through `RIPUPS`. The suite's heaviest flood is 66.3 M, on a 204 × 129 mm board.
- **Number:** `route::WORK` = **350 000 000** if D2 builds U1 with its killer (d3 seed 13, itself 170.1 M: 2.06×); otherwise **300 000 000**. 300 M is:
  - 2.10× the suite's heaviest tier-1 routing (142.9 M);
  - 2.02× the heaviest under U1 (148.7 M, RESULTS §3.1);
  - 2.52× the heaviest committed specification (119.0 M).

  It is per routing, so the ladder's eight routings may spend eight bounds.
- **Name:** `EmitError::RouterWorkExceeded` (kind `RouterWorkExceeded`, exit 2, `id` the net being routed when it was crossed). The message names the bound and the rip-ups so far. `RIPUPS` stays.
- **Cost:** the adversary's test runs, on Furnace:
  - 300 M: 37.5 s release, 41.6 s dev;
  - 350 M: 50.8 s release, 57.2 s dev, on a busier box;
  - about 350 MB either way.

  CI takes about 1.7–2.8× that. A weight row moves.
- **Lean: as stated**, the number tied to D2.

**D2 · (2) U1.**
- **Build it, or not?** It is the rule `1310` called sounder, and it moves no `fab/` byte. But it is a trade on shifted d3 boards, not a gain.
- **If built:**
  - no tier-1 literal moves: the suite under all three router changes passes, `strict_fence`'s refusal of the spike's modules aside (RESULTS §3.1);
  - the tier-2 tests that judge the moved boards (`with_tools`' d3 and d1 DRC, `board_tier2`'s Sentinel-03) re-run in the image;
  - the killer test is d3 shifted by seed 13: under U1 it routes with 307 rip-ups in 16.5 s release (170.1 M of work). Under bans alone, it is refused.
- **Lean: build**, as the roadmap row asks, saying in the PR that it is a trade.

**D3 · (3) the keepout rule.** Built as `1310` FEEDBACK A7 states it, not as the row reads:
- **The rule:** `route::read` refuses a footprint keepout not wholly inside its footprint's courtyard outline (the exact polygon) or its pad field (the pads' box), by name.
- **No byte or verdict moves** on a committed board, nor at any of the IMU's four turns.
- **The roadmap row's wording** is for the docs PR.
- **Lean: as stated.**

**D4 · (4) the stub.**
- **(a) Prune alone:** 152 stubs become 119 routes and 33 refusals by the pair judge, after `connectivity`.
- **(b) Prune plus the pair judge on the pairs' own copper right after the pair loop:** the same 33, refused before any other net routes and before `connectivity`. Today `pairs()` speaks last, so 12 other placements that end in an `/S` refusal now end in the pair's own refusal; their copper was already refused.
- **Movers:** none committed, either way. `check`, `connectivity` and `pairs` are unchanged; the final `pairs()` stays.
- **Lean: (b)**, because §2 (4) asks for "refused by name before `connectivity`".

**D5 · (5) the supply gate.**
- **No bit-identical cut except threads.** The cut is the 27 per-pin `bound()` calls on scoped threads, results collected in pin order. The alternative (a′) is each via's 1 024 boxes on threads, with the worst box still chosen in box order.
- **Effect:** wall time only; CPU time is unchanged, and memory grows by up to one network per thread (unmeasured until built).
- **Fit:** on CI's shared shard, the gain depends on free cores.
- **Is a thread "inside `extract`"** as the WO means it, and how many may it take (lean: at most 4)? Or is (5) left unbuilt, its time accepted?
- **Lean: (a), capped at 4 threads**, with the verdict bytes of both tests identical and both times measured.

**Findings, no decision needed:**
- **F1.** On base, 78 of the 1 232 sweep placements are refused by `pairs()` after routing ("… uncoupled at an end"): the router lays pair copper its own judge refuses. They fail closed. (b) moves them before the other nets, text unchanged.
- **F2.** `1307` A4's adversary was not kept; `spike/1323/adv1307.rs` is the reconstruction that the build's test would use.
- **F3.** `spike/1323/inst.py`'s shim adds a module file. The suite's only shim failure is `strict_fence`, which refuses any unlisted module, by design.

## 4. Gates at the stop

- **Fence:** `spike/1323/` and this file only.
- **Gates:** `cargo fmt --all -- --check` passes, and `rustfmt --check` passes on `spike/1323/*.rs`. No crate changed, so `clippy` and `cargo test --workspace` are as `main`'s. Two suites ran on prototype trees, and each passed everything except `strict_fence`'s refusal of the spike's own modules: the shim alone (RESULTS §2.1), and shim + U1 + prune + early judge (§3.1).
- **No adversarial or reduction pass yet:** both close the build, after the ruling.

## 5. The ruling, and the build

**The ruling** (11:00Z): build all five. D1 at 350 M with U1 built, a planted test at bound+1 and the committed heaviest board under the bound. D2 built, naming d3 seed 16's trade. D3 and D4 as leaned. D5 at most 4 scoped threads with byte-identity shown by a test, or dropped. `main` (`7dec1d7b`: 1916, docs) merged first as `7bddbe3b`; it changed no `eda` file.

| piece | built | tests (each kills the mutant beside it, §5.2) |
|---|---|---|
| **D1** work bound | `route::WORK` = 350 000 000. `Router` counts each state `search` expands and each node the router's own fill floods visit (`zone_blockers`, and `components` inside `route_ground` and `zone_blockers`; the final `connectivity` judge's flood is not the router's work and is not counted). A count past the limit stops the search, and `connect`, `route_ground`, the pair loop and `zone_blockers` each return it as `Unroutable { work_bound: Some(limit) }`, which `pcb::unroutable` maps to `EmitError::RouterWorkExceeded` (`id` the net). `route_measured(board, rung, limit)` returns the work done; `route_with` is it at `WORK`. `pair_run` is not counted (§1, D1). A ladder rung refused for its work is a rung that did not route, as an `Unroutable` rung is today | `route::tests::a_routing_one_unit_over_its_bound_is_refused_by_name` (two boards, one stitching ground; Ok at `w`, refused at `w − 1` with the bound named); `route::tests::the_1307_adversary_is_refused_by_the_work_bound` (54 s dev); `stackups_tier1::the_heaviest_committed_board_routes_within_half_the_work_bound` (`sentinel-02-power`); `pcb::tests::a_run_with_no_escape_keeps_its_shape` (the kind's shape) |
| **D2** U1 | `unhop` takes `soft`; a soft path's flattened hop is judged by `hard_open`, as the soft search judged it. `BANS` stays 16 | `stackups_tier1::a_shifted_d3_routes_with_its_soft_hops_judged_on_the_hard_maps` (d3, seed 13). **The trade, named as ruled:** d3 shifted by seed 16 routes with bans alone and is refused under U1 (`/SDA1: still blocked by ["VDD_3V3", "/SCL1"] after 500 rip-ups`); RESULTS §3 |
| **D3** keepout | `route::read`'s `part` refuses a footprint keepout not wholly inside the courtyard outline (exact: no outline edge enters the keepout's open interior and its centre is inside, at doubled coordinates) or inside the pad field (`route::field`), by name: "`<ref>`'s keepout on F.Cu lies outside its own courtyard and pad field". `emit` builds parts through the same reader (`route::part_of`) | `route::tests::a_keepout_outside_its_own_footprint_is_refused_at_read` (on the edge; moved off; 0.1 mm past; turned 90°; an L-shaped courtyard's legs and notch; the pad field beyond a small courtyard); `route::tests::a_board_past_the_item_cap_is_refused` (its keepout lies exactly on its courtyard) |
| **D4** stub | after a pair's breakouts, `prune` takes off each conductor's ends that are in none of its pads and at no escape's far end, until none is left, and `remap` (now shared with `rip`) rebuilds the maps; then `pairs` judges the pairs' own copper before any other net is laid | `route::tests::a_pairs_breakout_leaves_no_stub` (F2's board); `route::tests::no_placement_of_a_crossing_part_leaves_a_pair_stub` (624 placements); `route::tests::a_pair_the_judge_refuses_is_refused_by_its_own_name_first`; `route::tests::a_pair_on_escaped_pads_keeps_its_breakouts` (the adversarial pass's A1) |
| **D5** threads | `supply::bound_all` bounds a net's pins on `min(available_parallelism, SOLVE_THREADS = 4)` scoped threads, each pin computed exactly as one call of `bound`, kept at its index; one thread takes the serial path | `supply::tests::the_pins_bounded_on_threads_are_the_serial_pins_byte_for_byte` (5 pins, 1 thread against 2, 3 and 4, every result's `{:?}`) |

### 5.1 The bytes, re-run (§1.4)

`run.sh` on an archive of the head against base (`compare.sh`), release:
- **sweep:** exactly U1's movers. `d1-24-i2c`, `d3-24-i2c-12-spi`, `sentinel-03` and `solve/stress` move, each `.kicad_pcb` (and the `emit` JSON that carries its hash); the other 71 files are identical, every exit code the same. The head's 75 hashes equal the U1 prototype's (`spike/1323/sha256-branch.txt`).
- **`d4diff`:** identical, 277 cases, 926 files. **`d1319`:** 3 cases move (`d1-own-M1_OUT1-U2_VCP`, `d1-own-M1_OUT2-U2_CPL`, `d2-two`), each `out.kicad_pcb`, equal to the prototype's.
- **`examples/sentinel-01.json` is identical: no `fab/` byte moves.**
- **Tier 2, in the pinned image** (capped, offline), on the moved boards: `with_tools::d3_and_d1_pass_erc_and_drc_with_parity_at_four_layers` and `board_tier2::the_sentinel_03_fixture_passes_every_gate_inside_its_bound` pass (12.6 s and 20.4 s, peak 3.3 GB). CI's tier-2 shards are the record.

### 5.2 Mutants (`spike/1323/mutants.sh`, on an archive of `cd4b59f4`)

| id | mutant | killed by |
|---|---|---|
| W1 | the bound's check removed (`over()` false) | `a_routing_one_unit_over_its_bound_is_refused_by_name` |
| W2 | the bound off by one (`>=`) | `a_routing_one_unit_over_its_bound_is_refused_by_name` |
| U1 | U1 reverted to bans alone | `a_shifted_d3_routes_with_its_soft_hops_judged_on_the_hard_maps` |
| K1 | the keepout's containment removed | `a_keepout_outside_its_own_footprint_is_refused_at_read` |
| K2 | the pad field's branch removed | `a_keepout_outside_its_own_footprint_is_refused_at_read` |
| S1 | the stub fix reverted | `a_pairs_breakout_leaves_no_stub` |
| S2 | the early pair judge removed | `a_pair_the_judge_refuses_is_refused_by_its_own_name_first` |
| S3 | an escape's end pruned (A1) | `a_pair_on_escaped_pads_keeps_its_breakouts` |
| T1 | a thread's pins kept at the wrong index | `the_pins_bounded_on_threads_are_the_serial_pins_byte_for_byte` |

9 of 9 killed. A bound check deleted only inside `search` would still be caught by `connect` one step later, so W1 makes `over()` false, which nothing else catches.

### 5.3 (5), before and after

`extract` on `s024l_board()` (release, Furnace, back to back; the box was shared, load 9–25):

| limit | base | head | verdict bytes |
|---|---|---|---|
| 165 mV | 294.3 s wall, 291 s CPU, 568 MB | **114.2 s** wall, 341 s CPU, **2.43 GB** | identical (exit 0) |
| 60 mV | 306.1 s wall, 303 s CPU, 568 MB | **138.1 s** wall, 411 s CPU, **2.43 GB** | identical (exit 1) |

The cut is wall time only; CPU time rises a little and the peak memory 4.3×, one network and its AMG hierarchy per live thread. After the adversarial pass's S1 (§6), each pin's bound is converted inside its thread: re-measured below. `supply_tier1`'s whole binary on 4 CPUs: §8.

## 6. Adversarial pass (full; a fresh Opus worker, frozen at `40081ff4`)

The worker had the code, the diff and WO §5, not my conclusions; it worked on a copy under `~/.cache/aet-1323/adv/` with its own target, capped. Its report is a claim: A1 and D1 I reproduced and fixed with a test that fails before the fix; the rest I checked at source as stated.

| id | severity | what got through | disposition |
|---|---|---|---|
| **A1** | high, **this WO's regression** | `prune` ate a declared pair whose pads needed an escape: an escaped pad is joined at its escape's far end, outside the pad's `reach()`, so the breakout's first node was a leaf `prune` removed, then the next, until the early judge refused the pair ("its copper is not one path"). 6 configurations routed with prune off and were refused with it on (measured) | **fixed**: `prune` keeps the net's escape ends. Test `route::tests::a_pair_on_escaped_pads_keeps_its_breakouts` (fails before: "it stops at (7625000, 5375000)"); mutant S3. Over the worker's 72 escaped-pair boards (`spike/1323/escaped_sweep.py`), prune on and prune off now give identical outcomes, every one (measured) |
| **A2** | medium, **pre-existing** (read: unchanged by this WO) | **The work count does not bound wall time** on one huge net that is not stitched: work outside the count grows with pad count squared. One net of 1 600 pads did 348 036 units (0.1 % of `WORK`) in 130.2 s; 900 pads, 20.9 s; 400 pads, 1.9 s (measured). The candidates (read): `route_net`'s nearest-pad scan (each remaining pad against every tree node), each search seeded from the whole tree, and the final judges. | **for the architect** (§7, D6). The bound holds as a count and refuses the `1307` adversary as ruled; it is not a runtime guard for every board. No committed board is near (the heaviest committed board's 119.0 M units take 11.6 s). Counting the nearest-pad scan, or indexing the tree, is a router change beyond D1's ruled unit |
| A3 | low | `pair_run` is uncounted, as stated; its time grows with the pair's length (0.24 s at 30 mm, 1.38 s at 70 mm, counted work 522 in both; measured) | each call is capped at `4 × MAX_NODES` states and runs at most twice per pair; the number of pairs is the specification's. Stated in D1; no change |
| S1 | low–medium | the supply threads raise peak memory (measured by the worker: 2.44 GB threaded, 785 MB serial under `taskset -c 0`) | **reduced**: `bound_all` now hands each pin's whole bound to its conversion as soon as it is made, so none outlives its pin (it held all 27 until the last; that was the serial path's 785 MB against base's 568 MB). Re-measured in §5.3 |
| K1 | low | a keepout inside the overlap of two courtyard loops is refused (`held` is even-odd, and the reader admits overlapping loops) | fails closed; no vendored footprint has such a courtyard (all 27 read at every turn, measured). No change |
| K2 | low | a keepout outside a circular courtyard but inside its bounding square is read: the reader takes an `fp_circle` courtyard as its square (`1312` Q5) | no vendored footprint pairs a circular courtyard with a keepout. No change; a finding for the backlog |
| K3 | info | the pad field is all the pads' box, so a part with pads far apart reads a keepout far from its courtyard | the rule as ruled (D3) |
| D1 | low, this WO's | the new supply test was inserted between an old test's doc comment and its `#[test]` | **fixed** |
| O1 | open | tier 2's routings are not measured against the bound | as RESULTS says; CI's tier-2 shards route every tier-2 board under the bound and would refuse by name |

**Held** (the worker's, measured unless read): every limit from 0 to `w − 1` on six boards refused with `work_bound == Some(limit)` and never another kind (about 1 230 routings); every bound path reaches a check (read); the adversary refused at 350 M in 55.3 s release; a soft path is never committed, so a U1 hop `check` refuses has no path (read); all 27 vendored footprints read at 0°, 90°, 180° and 270°, at two origins, one off-grid; every keepout shape in its list (edge, off-centre at four turns, clockwise polygon, the base and mouth of a U, a corner touch, a ring's hole, two loops' gap); prune on and off identical on unescaped SOIC-like pairs; `extract` at 60 mV identical threaded and serial (sha256 `39d221e2…`).

## 7. Reduction pass (diff)

**Reduction pass (diff): ±0 net · 1 duplicate removed · 0 dead paths removed · 1 protected kept · 1 finding below.**
- Removed: the keepout check's own pad-field box. It now reads `route::field`, the router's one definition (10 lines in, 10 out). `rip`'s map rebuild was shared as `remap` with `prune` from the start, never copied, so it is not counted here.
- Kept: `bound_all`'s one-thread path, which looks like the threaded path at one thread. It is the reference the byte-identity test (and mutant T1) judges the threads against; without it a misplaced index would misorder both alike.
- Finding (out of the fence): SplitMix64 is now in three test files (`escape_tier1`, `stackups_tier1`, `spike/1323/u1hunt.rs`); sharing it is `tests/common`'s.

## 8. (5) on a CI-shaped machine, and what remains for the architect

**After S1's fix** (each pin converted in its thread), `extract` at 165 mV, release, the same board:

| mode | wall | CPU | peak RSS | verdict bytes |
|---|---|---|---|---|
| base | 294.3 s | 291 s | 568 MB | — |
| head, threaded (unpinned, load ~12) | 115.8 s | 349 s | 2.27 GB | identical to base |
| head, serial (`taskset -c 0`: `available_parallelism` 1) | 538.0 s | 289 s | 571 MB | identical to base |

The serial path's memory is base's again (it was 785 MB before the fix); the threaded path's 2.27 GB is the live networks themselves.

**`supply_tier1`'s whole binary on 4 CPUs** (`taskset -c 0-3`, dev profile, 4 test threads, as a CI runner has them): base **403.5 s** (load 3.9), head **382.0 s** (load 16.7), all 10 tests passing in both; peak RSS of one process 620 MB against 2.48 GB. The binary already runs its extracts side by side, which fills four cores, so the threads inside `extract` buy almost nothing there. `ci/colocate.tsv`'s `supply_tier1` row (721) is left as it is.

**D7 (new) · keep D5?** It is built as ruled and byte-identical by test and by measurement. On a 4-CPU runner it saves about 5 % of `supply_tier1`'s wall time, inside the noise of a shared box, and multiplies an extract's peak memory by about 4. Up to four `s024l` and Sentinel-03 extracts may run at once in that binary, so its total peak could approach 10 GB (inferred: `/usr/bin/time` reports one process's peak, not the sum). Removing D5 is four lines in `supplies` (the thread count set to 1). **Lean: drop the threads, keep `bound_all`'s per-pin conversion** (which alone restores base's memory and changes no byte), unless CI's run on this head shows the gates job faster.

**D6 (new) · A2: the count is not a runtime guard for every board.** One unstitched net of 1 600 pads takes 130 s at 0.1 % of `WORK`, through work outside the count (the nearest-pad scan in `route_net` and searches seeded from the whole tree). Pre-existing (read). Options: (a) a backlog row: count the nearest-pad scan, or index the tree; (b) accept, since no specification draws such a net. **Lean: (a), a §7.1 row**, since the docs PR owns the roadmap.

**Fence crossings, told here:** `crates/aethelgard-eda/src/pcb.rs` (`unroutable`'s one new arm for `RouterWorkExceeded`, and that function's shape test); its placement is not touched. `ci/test-weights.tsv`: five rows (the fence allows weight rows).

**Backlog rows for the docs PR:** the roadmap's `1310` A7 row rewords to what was built (a keepout refused outside its footprint, not admitted anywhere); A2 (D6); K2 (a circular courtyard's keepout is judged against the circle's square).

## 9. Merges, gates, close

- **`main` merged twice:** `7dec1d7b` (1916, docs) as `7bddbe3b` before the build, and `75da8053` (4408: a four-port board, the tools image re-pinned; the roadmap) as `2ed704f1`. Neither changes an `eda` file. 4408 adds a committed specification, `cli/…/fixtures/four-port.json`: base and head emit it byte-identical (release, 1.3 s each, far under the bound), so §5.1's movers stand. The tier-2 pre-check of §5.1 ran in the image before the re-pin; CI runs the new one.
- **No tier-2 test added:** `TIER2_MIN` unchanged.
- **Gates on the head** (after both merges, from a clean target; the worktree's copied `target/` had stale test binaries baked with `~/aethelgard-1322`'s path, which failed 28 unrelated tests until `cargo clean`): `cargo fmt --all -- --check` passes; `cargo clippy --workspace --all-targets -- -D warnings` passes, and with `--features aethelgard-cli/with-tools`; `cargo test --workspace`: **2 902 passed, 0 failed** (15 min 22 s, in a capped scope).
- **Disk:** the measurement's build trees removed at the architect's 10:58Z note (43 GB to under 1 GB); `~/.cache/aet-1323` keeps the logs, outputs and binaries this file cites.
- **Title and body** for the architect: `~/.cache/aet-1323/pr-title-body.md`.

## 10. D7 and D6, ruled (bus 13:10Z), and applied

- **D7: the threads dropped, the per-pin conversion kept.** `main`'s own loop in `supplies` already converts each pin's bound to its `PinResult` as it is made, which is why base peaks at 568 MB, so keeping the conversion and dropping the threads is `supply.rs` exactly as on `main`. `bound_all`, `SOLVE_THREADS`, the byte-identity test, `PinBound`'s `Debug` (added for that test), its weight row and mutant T1 all go. **(5) therefore changes nothing in the build:** the measurement (RESULTS §6) found no bit-identical cut but threads, and threads buy about 5 % on a 4-CPU runner for ×4 peak memory (§8). `supply_tier1`'s time is base's.
- **D6:** the architect adds the §7.1 row (a 1 600-pad unstitched net takes 130 s at 0.1 % of `WORK`).
- **`main` `cc99de04` merged** (2312: chaos, evolve and the CLI's campaign; no `eda`, specification or `ci/` file), so §5.1's bytes stand, and `route.rs`/`read.rs` are as at `dfb304ae`, so §5.2's mutants W1–S3 stand (8 of 8).
