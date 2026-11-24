# AET-WO-1324 — FEEDBACK: meeting via junction regions, and the nearest-pad scan inside the work count (measured, stopped at §1.1, ruled, built)

**Branch:** `wo/aet-1324-router-vias-and-scan` from `main` `72d85eef` (`4410` merged; `main` `7bf6163d`, docs only, merged in before READY).
**Builder:** Claude (Opus 5.5). Every figure here was measured by me on Furnace unless marked *read*. Written 2026-11-24.

**Status: built to the ruling** (`~/.cache/aet-bus/1324.md`, 2026-11-24 10:00Z). The measurement is `spike/1324/RESULTS.md`, beside `PREDICTIONS.md`, which was committed at `65e98697` before any run. The stop is PR #386 at `f0591f41`.

**The answer, in six lines:**
- **(1) is a limit of the supply model, not a fabrication fault.** KiCad 10.0.7's DRC passes the board `extract` refuses. The router now keeps a judged supply net's vias one junction region apart, judged in `extract`'s own cells from `extract`'s own reach, so the certified model is unchanged.
- **`sentinel-02-power` on PC0 no longer fails `ViaRegionsOverlap`.** The gate reads it: VDD_3V3 16/16 bounded, and every return refused `GroundReturnUnread` (exit 1). That is the PF3 layout's pre-existing refusal, recorded and not chased (ruling).
- **(2):** the nearest-pad scan was 99.0 % of one 1,600-pad net's time and grew as N³. An exact incremental index replaces it, choosing the same pad and node, ties included.
- **What stays super-linear is now counted:** the seeds, the index's updates and picks, and the final check's pairs, all charged to `WORK`. One net of 400, 900 and 1,600 pads is charged 6,163,182, 31,193,212 and 98,575,542 units: N², in a test, counts only.
- **Every committed board's bytes are unchanged:** 78 emitted files of 26 specifications, against the base. Gated `sentinel-03` and `sentinel-02-power` route byte-identical too.
- **Headroom:** the heaviest committed routing is `sentinel-02-power`, 136.2 M (38.9 % of `WORK`). The suite's heaviest is 177.1 M (50.6 %).

## 1. The measurement and the stop

See `spike/1324/RESULTS.md`. In short:

- **(1)'s cause.** One VDD_3V3 connection dropped to B.Cu through a new via 0.707 mm, (4, 4) pitches, from an earlier via, to meet the tree's B.Cu run. It was a cost choice: with the rule, the same connection lays on F.Cu. The router's only same-net via rule was KiCad's hole gap, 0.56 mm.
- **The regions.** Reach 349,068 nm, 120 cells of 62.5 µm. They meet at 108 grid offsets, out to (5, 3), 0.729 mm. A disc at twice the reach (0.698 mm) misses (4, 4), so the rule is judged in cells.
- **Why the model refuses.**
  - Thomson's principle needs one flow on the copper (`flow.rs` L1–9).
  - A region carries a field written for one hole (`junction_core`; `1303b` FEEDBACK §B1). Its cells are absent from every sheet (`flow.rs` L1882).
  - Two regions on a cell would count that copper twice.
- **Tolerances.** At +0.13 mm and ±0.075 mm the web between these two holes is about 0.065 mm. P1.2's tolerance clause was wrong. But that is the hole gap's question: the gap already admits 0.045 mm (§6, B1).
- **(2)'s shares** (a reconstructed lattice; seconds noisy up to 2×, shares within 2 points):

  | | 400 pads | 900 pads | 1,600 pads |
  |---|---|---|---|
  | scan | 95.0 % | 98.0 % | 99.0 % |
  | seeds | 3.5 % | 1.6 % | 0.8 % |
  | final judges | 0.3 % | 0.1 % | 0.06 % |
  | route time, two runs | 0.98–2.11 s | 10.9–16.5 s | 70.4–71.8 s |

  Wall time goes in FEEDBACK only (§2 of the work order).

### Predictions beside results

| Prediction | Result |
|---|---|
| P1.1 reach 349,068; regions apart from 0.75 mm | **held** |
| P1.2 not a fabrication fault | **held at drawn geometry** (DRC passes); its tolerance clause **missed** (0.065 mm web) |
| P1.3 no committed board moves under a gate-keyed rule | **held** (measured with the prototype, then with the build) |
| P1.4 next refusal `GroundReturnUnread` | **held**, as findings (exit 1), not an exit-2 refusal |
| P2.1 wall ≥ N² | **held; worse:** the scan is N³ |
| P2.2 scan ≥ 70 % | **held** (95–99 %) |
| P2.3 seeds 10–25 % | **missed** (0.8–3.5 %) |
| P2.4 judges < 5 % | **held** (0.06–0.3 %) |
| P2.5 counted work ~linear | **held** (27.8 K, 63.5 K, 113.9 K) |
| P2.6 an index leaves bytes unchanged | **held** (78/78 files) |

## 2. The ruling, and what was built

The ruling (Q1–Q5) takes every lean:

- **Q1, rule (a).**
  - `stack::JunctionRule` is one new `Stack` field, `junction`. It is set in `Stack::of` only where the gate runs: `power.drop_limits` non-empty, with `plating_min_um`, `fab_tolerances.hole_size_plus_um` and `hole_position_um` declared.
  - Its nets are each limited domain and `GND`.
  - `supply::junction_reach` is the reach formula moved out of `copper` and `pad_needed`, which call it unchanged. `JunctionRule::reach` calls it at `supply::BETA_NM`.
  - In `route.rs`, `Junction` holds, per judged net:
    - the 108 meeting offsets, as cell-set intersections of `flow::Region::new(supply::PITCH_NM, reach)`;
    - the nodes its fixed items block;
    - the nodes its laid vias block.
  - `via_open` refuses a node a region would meet. `close_vias` treats two of a path's new vias whose regions meet as a close pair (unhop, or ban). `claim_via` marks, and `remap` rebuilds from the fixed marks, so a rip-up cannot bring a via back.
  - The doc comments on `JunctionRule` and `Junction` say what the ruling asked: the regions are a limit of the supply model, KiCad's DRC passes such vias, and the router keeps the certified model unchanged by routing around it.
- **Q2.** The net's through-hole pads (with their own drill) and its array vias hold their regions too. An item off the gate's cell grid marks no node, since every point a cell step from it is off the router's grid as well. That is exactly the item `extract` leaves out.
- **Q3, (i).** `Nearest` keeps each remaining pad's least `(distance, node)` over the tree and is updated with each node added. The pick is the least `(distance, pad index)`.
  - **Charges:** each (pad, new node) update, each pad at each pick, and each source a search seeds (in `search`). A charge past the bound is refused as before (`Stuck::Work`).
- **Q4.** `judge_work` is `check`'s two pair loops (items, and holes). It is charged before `check` runs. A board too large for it is refused `RouterWorkExceeded` "at the final check of the routed board", with no net.
- **After the adversarial pass** (§5): `unhop` bans a close pair with a via between (A1); `meeting` works by row runs (A2); the rule needs every gate input (A3).
- **Q5.** `supply_tier1`'s comment now says the gate's inputs can change a routing, and that Sentinel-03 gated routes byte-identical (measured, §3).

### Tests (each with the mutant it kills, §4)

- **`route::tests` (eda):**
  - `two_junction_regions_meet_where_their_cells_do_not_where_a_disc_does`: 108 offsets; (4, 4) and (5, 3) in; (4, 5), (6, 0) and (0, 0) out; a disc would miss (4, 4); unjudged nets, and boards without a gate, hold none.
  - `a_judged_nets_via_stays_out_of_its_laid_vias_regions`: (4, 4) from a laid via refused where the hole gap alone admits it; (6, 0) open; open again after the net is taken up.
  - `a_paths_own_vias_whose_regions_meet_are_a_close_pair`.
  - `a_judged_nets_through_hole_pad_and_array_via_hold_their_regions`: a 1 mm hole's region meets a via 1 mm off and not 1.25 mm off; an array via keeps a routed via off (4, 4).
  - `the_nearest_index_picks_what_the_whole_scan_picked_ties_included`: 40 pads on whole millimetres, nodes added out of order and twice, each step against a brute-force scan.
  - `a_search_charges_every_source_it_seeds`.
  - `the_final_check_is_charged_its_pairs`: 10 one-pad nets do 45 units, and are refused at 44 at the final check.
  - `one_net_of_1600_pads_is_charged_its_quadratic_work`: the acceptance. It states the three counts, under a third of `WORK`, and 8–24 times the 400-pad count (measured 16.0). About 1 s in a debug build.
  - `the_meeting_offsets_by_runs_are_every_cell_difference` and `a_close_pair_with_a_via_between_is_banned_not_flattened`: the adversarial fixes A2 and A1 (§5).
  - `a_routing_one_unit_over_its_bound_is_refused_by_name` (`1323`'s), extended: `w − 1` is refused at the final check; one under the routing's own work is still refused by a net. No case removed.
- **`supply_tier1` (cli):**
  - `the_two_layer_power_fixture_on_the_pc0_sense_keeps_its_vias_regions_apart` replaces `…_meets_overlapping_vias`. It expects exit 1, VDD 16/16 bounded, and 11 returns `GroundReturnUnread`. It also checks, independently, that no two judged items' regions share a cell on VDD_3V3 or GND (A4).
  - `the_routers_junction_rule_is_on_where_the_gate_runs`: nets, tolerances, reach 349,068, `Stack` carrying it; off with no limit, an empty list, or each of the three inputs missing.

## 3. The bytes

- **Every committed specification,** release, against the base sweep (`spike/1324/sweep.sh`): 78 emitted files of 26 specifications, **byte-identical**, at the build (`a7b7ec82`) and again after the adversarial fixes. Only the output paths in `emit`'s stdout differ.
- **Gated** (the test's `gated()`: plating 18 µm, drop limit on VDD_3V3) `sentinel-03` and `sentinel-02-power` emit byte-identical to their ungated boards (`~/.cache/aet-1324/gated`).
- **The test-only PC0 board moves, by (1)'s rule:** VDD_3V3 25 → 24 vias, and 2 segments change (B.Cu hop → F.Cu). It is not committed.

### Headroom under the new charges (release; `route_measured` per call)

| Routing | Before (`1323`) | Now | Share of `WORK` |
|---|---|---|---|
| `sentinel-02-power` (committed, heaviest) | 116.9 M | 136.2 M | 38.9 % |
| `sentinel-03` (rung 320/false) | 126.1 M | 129.6 M | 37.0 % |
| d3 shifted under U1 (`stackups_tier1`, suite's heaviest) | 170.1 M | 177.1 M | 50.6 % |

The ruling quoted 131.6 M, my stop's estimate. It left out the final check's pairs and the picks; 136.2 M is measured.

## 4. Mutants (`spike/1324/mutants.py`, on an archive of `a7b7ec82`; one per guard this work order adds)

**22 of 22 killed**, plus one equivalent mutant recorded. M1–M19 ran on `a7b7ec82`. After the reduction (§7), the eight whose guards it touched were run again and killed (M3–M6, M8, M9, M16, M17). M20–M22 guard the adversarial fixes. M1 was retargeted to the row-run `meeting` and run on `5972c182`, as was M21.

| # | Mutant | Killed by |
|---|---|---|
| M1 | the merge in `meeting` skips an offset | `the_meeting_offsets_by_runs_are_every_cell_difference` |
| M2 | the meeting set keeps (0, 0) | `two_junction_regions_meet_…` |
| M3 | the junction on every net, not only judged ones | `two_junction_regions_meet_…` |
| M4 | `via_open` ignores the junction | `a_judged_nets_via_stays_out_…` |
| M5 | `claim_via` marks nothing | `a_judged_nets_via_stays_out_…` |
| M6 | `remap` keeps the laid vias' marks | `a_judged_nets_via_stays_out_…` |
| M7 | `close_vias` ignores the junction | `a_paths_own_vias_whose_regions_meet_…` |
| M8 | through-hole pads hold no region | `a_judged_nets_through_hole_pad_and_array_via_…` |
| M9 | array vias hold no region | `a_judged_nets_through_hole_pad_and_array_via_…` |
| M10 | the index compares distance only | `the_nearest_index_picks_…` |
| M11 | the index's tie goes to the last pad | `the_nearest_index_picks_…` |
| M12 | seeds uncharged | `a_search_charges_every_source_it_seeds` |
| M13 | index updates uncharged | `one_net_of_1600_pads_is_charged_…` |
| M14 | picks uncharged | `one_net_of_1600_pads_is_charged_…` |
| M15 | the final check uncharged | `the_final_check_is_charged_its_pairs` |
| M16 | the rule leaves `GND` out | `the_routers_junction_rule_is_on_where_the_gate_runs` |
| M17 | the rule on with an empty limit list | the same |
| M18 | the reach one nm short | `two_junction_regions_meet_…` |
| M19 | the rule never built | `the_two_layer_power_fixture_on_the_pc0_sense_keeps_…` |
| M20 | `unhop` flattens a pair with a via between | `a_close_pair_with_a_via_between_is_banned_not_flattened` |
| M21 | a run's offset interval one end wrong | `the_meeting_offsets_by_runs_…` |
| M22 | the rule ignores the gate's other inputs | `the_routers_junction_rule_is_on_…` |

- **The equivalent mutant.** `meeting`'s row offset `ja − jb` → `jb − ja` survives. Every `Region::new` region is symmetric under j → −j − 1, so the set is unchanged. No test can kill it.
- **A first M1 survived** (the merge skipping an offset). Disc regions' intervals nest, so the merge's start was never exercised. The test now also holds `meeting` exact on ragged cell sets, and M1 is killed.

## 5. Adversarial pass (full; a fresh Opus worker, on `a7b7ec82`)

The worker built its own oracle at the end of `route_measured`: the regions `supply::copper` builds, including geometries the gate leaves unpriced, so it is stricter than the gate.

**What it confirmed:**
- **Rule on:** 0 meeting regions on every rung of gated s02-pc0, gated `sentinel-03` and gated s024l (VDD_3V3 and GND, 2 and 4 layers, stitched GND).
- **Rule off,** as a positive control: s02-pc0's known pair; s024l VDD_3V3 at (96.75, 87.5)/(96.125, 87.625); `sentinel-03` GND on 3 rungs.
- **The real gate,** on boards routed with the rule: `extract` passes gated `sentinel-03` (16 pins and 10 returns bounded) and gated s024l (16 and 11).
- **The index** is the scan's choice.
- **The counts** are flat per unit across 400/900/1,600 pads (9.8/8.3/6.3 ns per unit).
- **All 36 committed specifications** are byte-identical, base against head.

| # | Severity | Whose | Finding | Disposition |
|---|---|---|---|---|
| A1 | medium | pre-existing; this WO widened it (108 offsets against about 60) | `close_vias` can pair a path's first and third vias. Flattened, the run left a layer change `commit` laid as a via `via_open` never judged. It happened in a synthetic path only, never on a suite or gated board | **fixed:** `unhop` bans the second via's node when the pair does not return to the layer the first left. Test `a_close_pair_with_a_via_between_is_banned_not_flattened`, under both the rule and the hole gap; mutant M20 |
| A2 | low–medium | this WO's | `meeting` cost cells², uncounted, before routing: 6.6 s per call at the 3 mm plating minimum `validate` admits | **fixed:** offsets by row runs. Exact on discs and ragged sets (test; M1, M21). At 3 mm the emit takes 12.8 s to its refusal (the worker had 57.5 s); at the 10 mm ceiling, 0.55 s (refused, `Unroutable`). Release, measured |
| A3 | low | this WO's | the rule was on with `outer_copper_min_um` missing, which the gate refuses | **fixed:** `JunctionRule::of` requires every stackup and tolerance input the gate reads; the test removes each of eight (M22) |
| A4 | low | test gap | no suite test checks GND's regions: the gate stops at `GroundReturnUnread` before pricing GND | **fixed:** the PC0 test checks every judged item's region on VDD_3V3 and GND from the emitted board, independently. Positive control: on the base board it fails on the known pair. A 4-layer gated extract (the worker's, 6–7 min each in debug) is not added to the suite; it is recorded here |
| A5 | low | pre-existing | a soft search may cross a junction-blocked node at the soft price and name no net, returning "no free path" before the whole-board free search. Today's own-net hole gap does the same | backlog (§6, B3): routability, never safety |
| A6 | info | as ruled | `connectivity`, `pairs`, `pair_run`, `keepouts` and `blockers`/`near` stay uncounted. None showed on the lattice | recorded |

No `unwrap` or panic reachable from input in the new code (read by the worker).

## 6. Findings for the backlog

- **B1 (pre-existing; read and measured).** The router's hole gap is judged on drawn drills only. At the fabricator's worst hole size and position, two same-net holes at the gap's minimum (0.625 mm on an axis) keep a web of about 0.045 mm. The specification holds no fabricator hole-to-hole figure. A rules work order could state one.
- **B2.** `sentinel-02-power` on PC0 ends at `GroundReturnUnread`: the two-layer ground zone is left unfilled, so the gate reads no plane. This is pre-existing, recorded by ruling, and `4300k`'s GND pours may move it.
- **B3 (A5).** A soft search may cross a node blocked by the net's own via spacing (hole gap or junction) and name no net, so `connect` says "no free path" before trying the whole-board free search. Blocking those nodes on the hard map too would let the soft search go round them. It costs routability on judged nets only.
- **B4 (A6).** The router's remaining uncounted loops (`keepouts` × items, `blockers`/`near` × edges, `connectivity`, `pair_run`) did not show on the lattice. A later count could name them, if a board finds one.
- **B5.** `RouterWorkExceeded` at the final check carries an empty net, so `emit`'s refusal `id` is `""`. A board-level kind, or a `None` id, would read better (`schematic.rs`, outside this fence).

## 7. Reduction pass (diff)

Reduction pass (diff): −6 net (+4 −10, `77f4a62f`) · 1 duplicate removed (`grid_node`'s bounds now through `node_at`) · 2 dead paths removed (`JunctionRule::of`'s `sort` and `dedup`: the nets are only searched; `Junction::new`'s off-grid skip, before the first commit, since such an item marks no node anyway) · 0 protected kept · 1 finding: the through-hole pad's centre, `(x0 + x1) / 2`, is written in `supply::copper` and in `Junction::new` (and in the PC0 test's check). A shared `Pad` method would cost more lines than it saves, and `pcb.rs`'s `Rect::centre` differs on odd negative sums, so it is left as two copies, named here.

After the reduction, the eight mutants through the code it touched were rerun and killed (§4).

## 8. Merges, gates, close

- **`main` merged:** `7bf6163d` (docs only, `2314`) as `59367d4e`, before the adversarial fixes. `main` had not moved again at the last fetch.
- **Gates on `5972c182`,** Furnace, capped scope: `cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo test --workspace --no-fail-fast` **3,063 passed, 0 failed**. An earlier full run on `77f4a62f` gave 3,061/0.
- **Tier 2 / CI:** this branch changes no container, workflow or tier-2 test. CI runs the shards.
- **The PR:** #386, left unmerged. Its title and body are in `~/.cache/aet-1324/pr-title-body.md`; `gh-write` cannot edit a PR's text, so the architect applies them.
- **Beside me:** `4300k` (the supply gate and power) and `1610` (cli). I added to `supply.rs` (`junction_reach`, its two callers) and to `stack.rs` (one field), and rewrote no one's code. A merge with `4300k` meets in `supply.rs` near `pad_needed` and the reach line.
