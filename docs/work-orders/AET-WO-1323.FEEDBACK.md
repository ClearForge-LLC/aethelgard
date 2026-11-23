# AET-WO-1323 — FEEDBACK: the router's backlog (measurement, STOPPED at §1.2 as planned)

**Branch:** `wo/aet-1323-router-backlog` from `main` `cd79024c`.
**Builder:** Claude (Opus 5.5). Every measurement here was taken by me on Furnace; no worker's report is relied on. Written 2026-11-23.

**Status: STOPPED at §1.2, the planned stop.** No crate, specification, `fab/`, workflow or `ci/` file changes; the branch carries `spike/1323/` and this file. The measurement is `spike/1323/RESULTS.md`, beside `PREDICTIONS.md` (committed at `456e0f48`, before any run).

**The answer, in six lines:**
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

## 4. Gates, fence

- **Fence:** `spike/1323/` and this file only.
- **Gates:** `cargo fmt --all -- --check` passes, and `rustfmt --check` passes on `spike/1323/*.rs`. No crate changed, so `clippy` and `cargo test --workspace` are as `main`'s. Two suites ran on prototype trees, and each passed everything except `strict_fence`'s refusal of the spike's own modules: the shim alone (RESULTS §2.1), and shim + U1 + prune + early judge (§3.1).
- **No adversarial or reduction pass yet:** both close the build, after the ruling.
