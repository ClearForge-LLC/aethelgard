# AET-WO-1310 — FEEDBACK (spike, then stop, then build)

**Branch:** `wo/aet-1310-escape` from `origin/wo/aet-4300g-support` `4bb6814` (stacked: PR #159's ruling 6904419552) · **Work order:** `docs/work-orders/AET-WO-1310.md` on `main`, amended by PR #159's rulings 6904190430 and 6904419552 · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Cadence:** spike, stop for the ruling, build.

Each answer below is labelled **measured**, **read** (at source), or **inferred**. The predictions in §1 were committed before any measured run of a candidate, and they are not edited afterwards. One measured run came before them: the baseline of §0 R9, which only reproduced `4300g`'s §6 failures on this branch.

## 0. Readings taken before the predictions

- **R1 · The grid and the floor** [read]. `route.rs`: `PITCH_NM` 125 µm; a node is open to a net only when a track centred there keeps `clearance + width/2 + EPS_NM` (200 + 100 + 10 = 310 µm) from foreign copper, a pad's box included. Outside a pad, no F.Cu node inside a footprint's pad field is open. A pad's terminals are the nodes inside its `reach()` box that are open by pads and the edge alone.
- **R2 · The placer snaps origins to 50 µm** (`pcb.rs` `SNAP`), against the 125 µm grid. Where a part lands relative to the grid therefore varies in 25 µm steps.
- **R3 · The pad that failed in `4300d2` is not the TXU0304's.** `4300d2` FEEDBACK §5 D7 names "U5 pad 20" on "the QFN-24's pads … 0.2 mm wide at 0.4 mm pitch". That is the ICM-20948 (`InvenSense_QFN-24_3x3mm_P0.4mm`). The TXU0304 is vendored as `TXU0304PW` on `TSSOP-14_4.4x5mm_P0.65mm`, whose pads are 1.475 × 0.4 mm at 0.65 mm (gap 0.25 mm). The work order's grounds name the TXU0304; the spike measures both. (The work order is protected: noted here only.)
- **R4 · The held LGA parts** (`~/.cache/aet-ds/aet-4220/ds/`):
  - BMP581, DS Rev 1.13, §8.2 p. 69: land pads 0.325 × 0.3 mm (package pad + 25 µm per side), 0.5 mm pitch, centres at ±0.5/0 and ±0.7625 mm (centres **inferred** by the 4220 reader from the drawing; orientation of each pad's long side **uncertain**). "We do not recommend vias or traces under the BMP581."
  - ICM-42688-P, DS-000347 p. 55: lead width 0.25 mm nominal (0.2–0.3), length 0.475, pitch 0.5, D1 1.5 / E1 1.0. The datasheet prints no land pattern; AN-000393 does (held, not yet read).
- **R5 · The amended pad list** (ruling 6904419552): d1's S9, seed 22's U3 pad 5 and the edge test's R1 pad 2 join §1.1's parts. None of the three is a fine-pitch pad: S9 is a two-pad part behind the ring, U3 and R1 are shifted or rotated ordinary parts.
- **R6 · The spike starts from** `~/.cache/aet-4300g/spike-d5-levers.patch` (lever 1, stubs first; lever 2, the pair approach). `4300g` §12 measured lever 2 routing `pair-standin` and lever 1 changing no outcome.
- **R7 · `1307` A4's bound** (`AET-WO-1307.FEEDBACK.md` A4): a 249 mm adversarial board with four LQFP-144s fails closed after 500 rip-ups in 616 s (568 MB); a wall-time bound is an unowned backlog row (roadmap §7.1). `MAX_NODES` is 4 000 000.
- **R8 · Roadmap §7.1 gives `1310` one more row:** "the diff-pair path walk tests a track end with `inner().contains`, where connectivity and the router use `reach()`" (`4300d2` reduction 8).
- **R9 · Baseline on this branch at `4bb6814`** [measured]: the 8 tier-1 tests of `4300g` §6 fail with the same messages: `/SCL1` "cannot reach S9 pad 1" (three d1 tests), `pair USB_STANDIN cannot be coupled` (three `classes_tier1` tests), seed 22 `VDD_3V3` "cannot take U3 pad 5 to a via", and `/I2C_SDA` "R1 pad 2 has no node clear to end a track". `escape_tier1` takes 89 s.

## 1. Predictions (committed before any measured run of a candidate)

| # | Prediction | Basis |
|---|---|---|
| P1 | **A pad has a grid terminal only when a node in its reach box sits within `p − w/2 − 310 µm` of its centre across the gap** (pitch `p`, pad width `w` across the gap). For the ICM-20948's QFN-24 (0.4 mm, 0.2 mm) that is −10 µm: **no terminal at any offset, on any grid, while `EPS_NM` > 0.** A finer grid (candidates 2 and 3) cannot escape it at the 200 µm floor; only an exact, off-grid segment on the pad's own centreline can (200 µm exactly, which `check` admits as ≥). | inferred (R1) |
| P2 | **The TXU0304 is not a sub-grid pad:** 650 − 200 − 310 = 140 µm > 62.5 µm, so every pad has a terminal wherever it lands. | inferred (R3) |
| P3 | **BMP581 (0.3 mm across the gap):** a terminal only within 40 µm, so some 50 µm-snapped origins leave pads without one on the 125 µm grid; on a 62.5 µm grid (offset ≤ 31.25 µm) every pad has one. **ICM-42688-P:** depends on AN-000393's land width; at 0.35 mm it is 15 µm, blocked at most offsets on either grid. **A 0.5 mm QFN at 0.25 mm pads:** 65 µm, so it always has one at 125 µm, unless its rounding narrows the reach box. | inferred |
| P4 | **The three amended pads (S9, U3 pad 5, R1 pad 2) are not sub-grid pads.** Their failures are crowding: neighbouring parts' pads leave a channel narrower than a grid-centred track needs (600 µm + 2 × 10 µm + up to 125 µm of misalignment). The pin-centred escape, as the work order defines it, changes none of them. | inferred (R5) |
| P5 | **Lever 2 reproduces `4300g`'s result** (pair-standin routes), and lever 1 again changes no outcome. | inferred (R6) |
| P6 | **A global 62.5 µm grid** routes at least one of the three crowded terminals (a finer grid fits narrower channels), changes every board's bytes, costs 3–8× router time on today's boards, and refuses `1307` A4's 249 mm board at `MAX_NODES`. | inferred (R7) |
| P7 | **The pin-centred escape changes no board that routes today** (no vendored footprint before the ICM-20948 has a pad without a terminal, `4300d2` FEEDBACK §11), and it brings the ICM-20948's power fixture past "U5 pad 20". | inferred |
| P8 | **Not every one of the 12 failing tests routes with any single candidate plus lever 2;** the crowded terminals need a placement rule or an escape that is not on the pad's own axis. The spike therefore ends with a recommendation that pairs the pin-centred escape with a placement change for the crowded terminals. | inferred |

## 2. What fails, by cause (measured)

Each of `4300g`'s failing tests was probed on the bare board: its pad's geometry against the grid, its terminals, what blocks each node, and whether the hard maps alone (pads, fields, courtyards, the edge) join it to the rest of its net or to a via site. Probe and spike code: `~/.cache/aet-1310/spike-1310.patch` (`sha256 2901b03a…`) and `~/.cache/aet-1310/spike-tests/`. None of it is in the branch.

| failure | the tests | cause [measured] | what fixes it [measured] |
|---|---|---|---|
| `/SCL1` "cannot reach S9 pad 1" | d1: `stackups_tier1::d1…`, `escape_tier1::twenty_renamings…`, `adversarial_1301::attack_a_wide…`; tier 2 `with_tools::d3_and_d1…`, `adversarial_1301_tools::attack_wide_references…` | **Not an escape.** S9 pad 1 has 4 terminals, and its hard region joins every pad of `/SCL1`. The free searches find nothing (routed copper). Both soft searches find a path, but each path hops under copper with two vias closer than the hole gap. `unhop` bans one node and searches again, `BANS` = 4 times, and then the try returns nothing. With every try empty, the net is refused. | `BANS` 16 (also 64). Alternatively, `U1`: a soft search's flattened hop judged on the hard maps, as the soft search itself is. Both route d1; `U1` changes d3's bytes, `BANS` 16 changes none (§5) |
| pair `USB_STANDIN` "cannot be coupled" | pair-standin: three `classes_tier1` tests; tier 2 `with_tools::the_class_and_pair_fixtures…` | As `4300g` §6: the ring pushes the placement in front of the pair's pins | `4300g`'s lever 2 (the pair's approach kept clear by the placer) |
| seed 22 `VDD_3V3` "cannot take U3 pad 5 to a via" | `escape_tier1::shifted_placements…` | **A sealed pad.** The test's seeded shift puts C9 pad 2 (GND) 606.8 µm in front of U3 pad 5. Its hard region is 17 F.Cu states and no via site. A track down that channel needs 600 µm exactly, or 620 µm on the grid (`EPS_NM` both sides), plus up to 125 µm of misalignment. Measured: still sealed on the 62.5 µm grid with `EPS_NM` 10 µm. Inferred: at 62.5 µm the least `EPS_NM` the const assert admits leaves a 3.6 µm window, so a finer grid reaches it only by chance | `A′`, a channel escape (§4): an exact run down the channel's centreline (x = 101.7847 mm), then onto the grid |
| `/I2C_SDA` "R1 pad 2 has no node clear to end a track" | `classes_tier1::a_part_on_each_edge…`; tier 2 `with_tools::a_part_on_each_edge…` | **Not routing: R1 is off the board.** The test moves R1 to 2 mm below the plain board's bottom edge. With `4300g`'s connectors, `layout_of` sets the outline's bottom to the connectors' courtyard bottom (`pcb.rs:1206`) whatever was moved. So R1 (y 128.1 mm) lies below the outline (y1 126.1 mm), and its reach box holds no grid node at all | `O1`: a moved part also bounds the bottom edge (`emit_moved` has no production caller; tests only) |
| `adversarial_1301::attack_a_wide…`, once d1 routes | (the same d1 test) | **The test's own model.** It asserts that every footprint's `Reference` text is at angle 0, and judges each as silkscreen. `4300g`'s anchored placer turns parts (d1 now carries 6 references at 180 and 8 at 90 or 270). Their references are on `F.Fab` (`pcb.rs:1140`, `4300d2`'s rule), which KiCad does not print, and they meet U1's courtyard by design. Even the test's control (short references) then reports 19 collisions | The test judging `F.SilkS` references only, with an angle-aware box: then it **passes**, with no collision (every silkscreen reference is upright and clear). This is a test edit: D2 |

## 3. §1.1 — which pads the grid cannot escape (measured: synthetic boards)

Each part was placed alone at every 25 µm offset in x and y (`SNAP` is 50 µm, so these are the residues the placer can produce, and more) and at 0, 90, 180 and 270: 100 configurations each. Every pad was wired to its own partner 3 mm out. "Terminal" means a grid node in the pad's reach box that a track may end at.

| part (land) | pads with no terminal (configs) | routes today | routes with the escape (A) |
|---|---|---|---|
| ICM-20948, QFN-24 0.4 mm, 0.2 mm pads (vendored; **the pad `4300d2` measured**) | every pad, all 100 (P1) | 0 / 100 | every pad wired: 36 / 100; every 2nd: **100 / 100**; every 3rd: **100 / 100** |
| TXU0304PW, TSSOP-14 0.65 mm (vendored) | 0 | 100 / 100 | 100 / 100 |
| ICM-42688-P on AN-000393's land (land = pin, 0.25 × 0.475 mm: AN-000393 v2.4 Fig. 1 p. 6 [read, rendered]; DS-000347 p. 55) | 0 | 100 / 100 | 100 / 100 |
| ICM-42688-P on KiCad's stock `LGA-14_3x2.5mm_P0.5mm_LayoutBorder3x4y` (0.35 mm pads) | 96 | 0: the footprint's own pads are 150 µm apart, refused by `check` | 0 (the same) |
| BMP581, land of §8.2 (0.3 mm across) on the 4220 reader's inferred centres (±0.5, ±0.7625 mm); its corner pads 7 and 8 sit exactly 200.0 µm apart | 40 | 60 / 100 | **100 / 100** (every pad, every 2nd, every 3rd) |
| QFN-16 3 × 3 mm 0.5 mm (KiCad stock, 0.25 mm pads) | 0 | 100 / 100 | 100 / 100 |

Each part's centres are exact to the nanometre. A first run read half-µm centres (0.7625, 1.1625, 0.9125, 1.4625, 2.8625 mm) as whole µm; that put the BMP581's corner pads 199.5 µm apart and was this builder's error, caught in review before the stop was pushed. Every row above is the corrected run (`~/.cache/aet-1310/logs/sweep-final.txt`).

So:
- **The TXU0304 is not a sub-grid part** (P2 held). The work order's grounds name it; the failing pad was the ICM-20948's (§0 R3).
- **The ICM-42688-P on its application note's land needs no escape.**
- **The BMP581 needs one at 40 % of placements, and with it routes at every one.** On the inferred centres its corner gap is exactly the floor (200.0 µm), which `check` admits; `4220b` draws the land and must not draw it tighter.
- **§7's stop does not fire:** A escapes every held part whose land itself clears the floor. KiCad's stock LGA-14 (150 µm between pads) does not, and is not the land AN-000393 recommends.

## 4. The three candidates (measured)

| | A: pin-centred off-grid escape | B: half-pitch sub-grid in each courtyard | C: global 62.5 µm grid (levers 1 and 2 off, unless stated) |
|---|---|---|---|
| **What was built** | Prototype in `route.rs` (patch). For a routed SMD pad with no terminal: from its centre, along its own axis, out across its field's edge, to a grid row, then across to a grid node. A run of such pads gets **lanes 500 µm apart** (the least grid multiple a track's grid node needs from a neighbour's copper: 410 µm). The base lane keeps the largest jog least, and outer pads jog first. Every escape is validated against exact clearance, and its copper is claimed in every map before any net is routed | **Not built** (time box). Its terminals near a pad are a subset of C's 62.5 µm lattice, so C's terminal result bounds it | `PITCH_NM` 62.5 µm, windows, corridor depth and via cost scaled ×2 to keep distances; `EPS_NM` unchanged (subagent; spot-checked) |
| **The ICM-20948 board** (`sentinel-02-power`) | **routes** (9 escapes). Limit: with all 24 of the QFN's pads wired, 36 / 100 synthetic placements route (§3) | inferred: fails as C does | **fails**: "U5 pad 20 has no node clear to start a track" (P1) |
| **DRC with parity** (image, `capped-run`, swap 0) | d1 and pair-standin: pass, no finding. `sentinel-02-power`: no clearance, unconnected or parity finding. The one finding is `lib_footprint_mismatch` on U5 (D4) | — | not run (the power board does not route) |
| **`check`** | needs **an exact rounded-pad clearance**. By boxes, U5's corner pads 1 and 24 read under 200 µm and the routed board is refused. As rounded rectangles (`inner` box grown by the corner radius) they pass. Measured both ways | — | — |
| **Byte-identity of the 9 fixture boards that route today** | **identical**: 0 of 9 changed (with `BANS` 16 in place of U1, and lever 1 off); every other output of `emit-all` identical but the four newly routing fixtures' | inferred: identical only if applied where today's grid has no terminal | **none identical**: 9 of 9 changed |
| **Router wall time** (best of 3, release) | each routing board within noise of today; newly routing: power 6.6 s, stress 3.8 s, d1 0.8 s, pair-standin 0.3 s; all 19 fixtures 41 s against 23.6 s, the difference being the four boards that now route | — | 106.2 s for all fixtures (4.5×); d3 29.2 s against 6.1 s (4.8×). With levers 1 and 2 on (the patch's default, the subagent's first run): 229.6 s and d3 95.8 s. `MAX_NODES` caps a square board at 125 mm (250 mm today), so `1307` A4's 249 mm board would be refused (computed, P6) |
| **The 12 tests** | with the levers of §5: 7 of 8 tier 1 pass; the eighth needs D2 | — | with levers 1 and 2 on: 2 of 8 tier 1 (d1's two). Levers off: d1 routes and pair-standin and the power board do not (`emit`) |
| **What the strict reader and `extract` must learn** | nothing [read: no grid assumption in `strict.rs`, `extract.rs` or `route::read`; measured: the gate's reader took the off-grid copper in all three DRC runs]. `extract` refuses the power board by name (`TranslatorPinUnbounded`, `4300d2` D6) before it reaches copper, so no off-grid term was measured through `extract` | — | — |

## 5. The levers, measured on the 12 tests and every fixture

| lever | earns its place? [measured] |
|---|---|
| `4300g` lever 1 (stubs first) | **No.** With it off, every outcome is the same; with it on, every board's copper moves. Dropped |
| `4300g` lever 2 (pair approach) | **Yes.** Without it pair-standin's three tests fail again. It changes only boards with a declared pair (pair-standin alone) |
| `BANS` 4 → 16 | **Yes**, for d1 (S9). Every routing board stays byte-identical |
| `U1` (soft hop judged on the hard maps) | Routes d1 too, but changes d3's bytes. Not proposed: `BANS` 16 does the same with no change |
| `A`, the pin-centred escape with the lane plan | **Yes**, for the ICM-20948 board (not among the 12; the work order's §2 board) |
| exact rounded-pad clearance in `check` | **Yes**, the ICM-20948's corner pads |
| `A′`, the channel escape for a sealed pad | **Yes**, for seed 22. It applies only to a pad whose hard region (pads, fields, courtyards, the edge) reaches no via site and no other terminal of its own net within 4000 states. No routing board changed |
| `O1`, a moved part bounds the bottom edge | **Yes**, for the edge test. As prototyped it took every part's box; built, it takes only the moved parts' |
| `adversarial_1301`'s reference model | the eighth test: D2 |

**The 8 tier-1 tests, measured:**
- with A, exact `check`, A′, `BANS` 16, lever 2 and O1: 7 pass, and `attack_a_wide…` fails on the test's model (§2);
- with the test judging silkscreen references only: it passes too.

**Every fixture with that set** (`~/.cache/aet-1310/emit/b16` against `base`):
- every board that routes today is byte-identical;
- d1, pair-standin, `solve/stress` (unroutable on `main` too) and `sentinel-02-power` now route.

**The whole workspace suite under that set** (`cargo test --workspace --no-fail-fast`, the switches in the environment; `~/.cache/aet-1310/logs/ws-final.log`) [measured]: 1637 passed, 3 failed.
- `adversarial_1301::attack_a_wide…`: D2.
- `power_board_tier1::the_crossing_is_drawn_and_its_limits_are_refused_by_name`: it pins `4300d2`'s refusal "U5 pad 20 has no node clear", and the board now routes. That expected value is the build's to re-take, as the work order's §2 intends.
- `routing_tier1::the_routed_board_is_byte_identical_and_a_function_of_its_pads`: **a harness artifact.**
  - The test emits through the CLI with `env_clear()`, so the CLI ran with the patch's defaults (lever 1 on), while the in-process re-route ran with `AET_NO_STUBS`.
  - Bisected [measured]: with no switch, and with each of `AET_ESC`, `AET_EXACT`, `AET_CHAN` and `AET_BANS=16` alone, it passes; only `AET_NO_STUBS` fails it.
  - The build carries no switches.

**Tier 2: not run in the spike.** DRC with parity in the image was measured on d1, pair-standin and the power board (§4). The four tier-2 tests are the build's to run. Two risks are named, unmeasured:
- O1 moves the edge test's outline below the connectors, whose courtyard bottoms are the edge today (`with_tools::a_part_on_each_edge…`);
- `adversarial_1301_tools::attack_wide_references…` is D2's twin.

## 6. Predictions against the results

| # | held? |
|---|---|
| P1 | **Held.** No grid node escapes the ICM-20948 at any offset; the 62.5 µm grid fails it (measured) |
| P2 | **Held** (100 / 100 on today's grid) |
| P3 | **Held for the BMP581** (an escape at 40 % of placements; on 62.5 µm not measured). **Wrong for the ICM-42688-P:** on AN-000393's land (0.25 mm) it needs none. The 0.5 mm QFN always has a terminal, as predicted |
| P4 | **Held for the kind, wrong in the detail.** None of the three is a sub-grid pad. Only seed 22 is a narrow channel. S9 is a search giving up (`BANS`), and R1 is off the board |
| P5 | **Held** |
| P6 | **Partly.** C routes d1, but not by escaping a crowded terminal: d1's failure was `BANS`, not a channel. Every board changes. 4.5× in all and 4.8× on d3 with the levers off, within the predicted 3–8×. A4's board would be refused (computed) |
| P7 | **Held** |
| P8 | **Wrong.** With A, A′, `BANS` 16, lever 2 and O1, every routing failure among the 12 is fixed with no fixture edit. The one left is a test's model of reference text (D2), not routing |

## 7. Recommendation and decisions needed

**Recommendation: build A with the lane plan, the exact rounded-pad clearance in `check`, A′, `BANS` 16, lever 2 and O1 (moved parts only); drop lever 1 and U1.** It routes the work order's board and every routing failure among the 12 tests, and every board that routes today among the 19 fixtures stays byte-identical, and the workspace suite under it fails only as listed in §5. The tier-1 tests per pad class:
- an escape on its own axis;
- a lane-planned run;
- a channel escape;
- a pad the grid already reaches gets no escape;
- an escape that would cross a neighbour's clearance is refused;
- a rotated part;
- an anchored part's pair approach (lever 2);
- `BANS`.

Each has a killed mutant.

**Decisions for the architect:**
- **D1 · Scope.** The work order's escape is "on its own axis". Two measured additions go past it:
  - the **lane plan's jog**, a perpendicular segment of up to a few pitches at the end of the axis run, without which a run of adjacent connected pads at 0.4 mm cannot reach 500 µm lanes;
  - **A′, the channel escape**: the axis run, then a run along a channel's exact centreline, for a sealed pad (seed 22).

  **A's limit, measured:** a QFN-24 at 0.4 mm with all 24 pads wired routes at 36 of 100 synthetic placements (every 2nd or 3rd pad wired: 100 of 100). A full run of adjacent connected pads needs a fan-out wider than one lane per pad; the build would refuse such a pad by name rather than leave it to chance.

  The fence also widens to `pcb.rs`'s outline rule (O1) and `connect`'s `BANS`. Both are within "the router, the placer"; named because they are not escape work.
- **D2 · `adversarial_1301::attack_a_wide…`.** The acceptance is "pass with no fixture edit". This is a test edit, not a fixture edit: the test judges every `Reference` as silkscreen at angle 0, and `4300d2`'s rule puts anchored parts' references on `F.Fab`. Options:
  - (a) the test judges `F.SilkS` references only, with an angle-aware box (measured: it passes, no collision);
  - (b) the placer stops turning anchored parts (it would move every ring, and every board's bytes);
  - (c) something else.
- **D3 · `BANS` 16 in place of the soft-hop rule (U1).** Both route d1. U1 is arguably the sounder rule (a soft search's flattened hop judged as the soft search judges), but it changes d3's bytes; `BANS` 16 changes nothing.
- **D4 · The ICM-20948 board's DRC.** Routed with A, it fails DRC only on `lib_footprint_mismatch` (a warning) on U5.
  - **Measured:** the vendored footprint is byte-identical to the image's library file (sha256 `f8bb8114…` both). The board's U5 block carries the same pads, zone and keepout, with only placement fields added. No other board that routes today places this footprint (`imu-baro-1v8` is refused before `emit`).
  - **Not established:** the cause. Inferred: KiCad's comparison of a footprint that carries a keepout zone.
  - **For the ruling:** "passes every gate it reaches" needs one. The build can find the cause, or the warning can be admitted by name, or the gate stays as is and this board's DRC is named as failing.
- **D5 · Roadmap §7.1's row for `1310`** (the pair walk tests a track end with `inner().contains`). Take it in the build? It is small and in `route.rs`.

**Base and CI.** This PR's base is `wo/aet-4300g-support`. Its CI runs that branch's 12 failing tests, so the STOPPED head is red by inheritance, not by anything here; nothing but FEEDBACK changed.
