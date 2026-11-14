# AET-WO-1307 FEEDBACK — escape planning and name-independent order: spike

**Builder:** Claude (strong tier), worktree `~/aethelgard-1307`, branch `wo/aet-1307-escape-planning` from `main` `d00c965`.
**Cadence:** spike (§1), then STOP for the architect's ruling, then build (§2).

The predictions in §1 were committed before any probe was built or run. They are not edited afterwards; corrections go in §3.

Labels:
- **[read]**: read at source in this repository at `d00c965`.
- **[measured]**: run. Host cargo runs under a capped user scope. KiCad runs in the pinned image under rootless podman, with its caps read back.
- **[inferred]**: neither; reasoning from the two.

## 0. Source reads that shape the predictions

**[read]** at `d00c965`: `route.rs` (`route`, `Router`, `check`, `connectivity`), `pcb.rs` (`place`, `copper`), `rules.rs`; except where an item is marked **[inferred]**.

- **Order.** `route::route` sorts by `(!stitched, pad count, net name)`. The stitched nets (at 4 layers, each plane's net: `GND` and `VDD_3V3` on `D3`) are laid first, each pad joined by F.Cu to a through via. The net index is the net's rank by name (`BTreeMap` keys), so names also decide:
  - the order of `Stuck::By`'s `BTreeSet<usize>`, which is the order blockers are pushed back on the queue;
  - which of two equal-count nets goes first.
- **The tree root is a name.** `net_pads` orders a net's pads by **reference**, then pad number, and `route_net` grows the tree from `pads[0]`. `route_ground` stitches pads in that order too, so the first pad by reference claims the nearest via site.
- **Escape.** On F.Cu no track crosses a footprint's pad field (the box around its pads) except inside its own pad. No via stands within `VIA/2 + s` of the field. So at the LQFP every pad leaves straight outward. The whole interior of the pad ring is closed to tracks and vias on F.Cu. It is empty: `pcb::place` keeps every part clear of the MCU's keep-out box (courtyard and silkscreen) by 1 mm for ICs and 0.5 mm for passives. A pad meeting exactly one side of its field gets a soft *corridor* (cost 2 per pitch, 16 pitches deep). The corridor is a price, never a reservation.
- **Rip-up.** A soft search names the blockers. The stuck net and its blockers are ripped and re-queued at the front, with a `HISTORY_COST` of 40 on the contested nodes. The budget is `RIPUPS` = 500 nets in all.
- **Geometry at 0.5 mm pitch** (rules 0.2/0.2, via 0.6/0.3, hole gap 0.25 mm): two vias need 0.8 mm centre to centre. So adjacent LQFP plane pads (0.5 mm apart) cannot drop vias side by side. The second via must stagger outward, and its stub then runs beside the first via, in a neighbour's escape lane.
- **Placement depends on reference names.** `pcb::place` places the MCU, then the other ICs **in reference order**, then the passives in reference order. Each goes beside the IC it shares most nets with, with ties to the lower reference. Each decoupling capacitor goes in the first free spot in rings out from the pad its rail names. So renaming references at the specification level moves parts before the router runs. The decoupling capacitors sit just outside the LQFP's pad ring, by its supply pads, and their pad fields block via sites there too.
- **The gate's rules** (`rules.rs`): a clearance and a track-width minimum, as net class and as `.kicad_dru` custom rules. `route::check` judges clearance, edge, axis and hole gap. Neither has a courtyard rule for vias.
- **Zones.** `pcb::copper` writes each zone's polygon as the board outline's four corners exactly (x0,y0 → x1,y0 → x1,y1 → x0,y1). The emitter does **not** inset it by the edge clearance. KiCad's fill applies that clearance **[inferred]**: `route.rs` says so in `EDGE_NM`'s comment, measured as a board setting, not read in KiCad's fill source. `route::read` keeps only `(net, layer)` per zone and reads no polygon.

## 1. Predictions (committed before any run)

Probe plan: patches to `route.rs` under `spike/1307/`, applied to detached worktrees of this branch's base, with a release build, a runner script and a renaming script. The inputs are D3 and D1 (fixtures), the reference (`examples/sentinel-01-minimal.json`), 2 × 5 (`4100`'s `many(n_i2c=2, per_bus=5)` on the reference's 2-layer board) and the 4-layer fixture.

### S1. Why D3 fails
- **P1.1** The failing pads are U1 (LQFP-144) pads where a `VDD_3V3` pad and a `GND` pad are adjacent or one apart, on the side(s) where the SPI and CS signals leave. `4100` named U1 31 and 39. Every contested escape is within 3 pads of a plane pad.
- **P1.2** The second plane via of an adjacent pair has a free site, but only by taking the escape lane of the next signal pad (a CS or HOLD line). The site is not missing; it sits on another pad's only way out. At least one plane pad has **no** free via site within 16 pitches when the signals are laid.
- **P1.3** At give-up the rip-up loop is **cycling**: fewer than 8 distinct nets make up at least 80 % of the 500 rip-ups. `VDD_3V3` (or `GND`) and the same two or three CS/HOLD nets take turns, and the history cost does not break the cycle.
- **P1.4** The cause, in one sentence (to be confirmed): *at 0.5 mm pitch two adjacent plane pads need staggered vias, and the outer one sits in a neighbouring signal pad's only escape lane, because every LQFP escape is forced outward into the same strip.*

### S2. Candidate escape strategies (each measured on D3, D1, reference, 2 × 5, four-layer)
- **(a) Escape first** (one planned stub and via site per LQFP pad, outward, alternating rows, plane vias claimed first):
  - D3 routes, 55 %;
  - D1, 2 × 5 and four-layer route;
  - the reference's board changes (not byte-identical);
  - via count rises by 0 to 10 % on boards that already route;
  - time within 2× of today's.
- **(b) Plane-pad priority** (each plane pad claims its geometrically nearest free site before any signal escape; that is today's order, made geometric): **D3 does not route**, 70 %, because today's order already lays the planes first. The rest route as today. The reference is byte-identical or nearly so.
- **(c) Inward plane escape** (my third, from P1.4): a plane-net pad of the dense part escapes **inward** on F.Cu, under its own body, to a via inside the pad ring. Signals keep the whole outward strip. The interior of an LQFP-144 is about 17 mm square and empty.
  - **D3 routes, 75 %**: the fastest candidate, with the fewest rip-ups.
  - D1, 2 × 5 and four-layer route.
  - The reference changes at its LQFP plane pads, unless it is limited to 4 layers.
  - KiCad DRC passes: a via under a part's own body breaks no rule the gate writes. The gate writes only clearance and width, and `check` has no courtyard rule. This is **[inferred]** for KiCad's built-in checks (no `rule_severities` entry is written for a via in a courtyard).
  - **Decision flag:** WO §5.2 lists "a via site under another part's courtyard" as an escape lie. A via under the dense part's **own** body is not that case, but it is new, and it needs a ruling.
- **P2.5** **The reference's bytes and bound.** (b) leaves the reference byte-identical, 60 %; (a) and (c) do not. Where the reference's routed board changes, its two extraction bounds (46,800 and 58,235 fF today) move by under 10 % and stay under their declared limits (100,000 fF per line). The second §7 stop is not hit.
- **P2.4** No candidate needs a change to the rules or the stackup (the §7 stop is not hit).

### S3. Name independence
- **Two levels of renaming.** *Specification level* (end to end, what a user sees): device references and bus nets only. `U1`, `C*`, `R*`, `GND`, domains and the emitter's derived nets cannot be renamed there. It moves placement (§0), so it measures the placer and the router together. *Board level* (the router alone, placement fixed): every net and every reference in a `route::Board`, through a probe. Every one of the 20 renamed specifications must reach the router; a refusal other than `Unroutable` is reported as not a routing datum.
- **P3.1** Under today's order the 20 fixed renamings (seeded hash prefixes) give, at both levels:
  - reference 15 to 19 of 20;
  - D1 (4 layers) 10 to 18 of 20;
  - D3 0 of 20.
- **P3.2** A proposed order:
  - stitched first;
  - then pad count;
  - then the bounding box's half-perimeter;
  - then the pads' coordinates in lexicographic order;
  - the name only as the last tie-break.

  With the tree root and stitching order taken from geometry (lowest y, then x) and blockers re-queued by rank, not index, the renamed boards give **the same copper modulo names**: 20/20 identical geometry for the reference and D1.
- **P3.3** The guard (board level): a unit test that routes a `Board` and the same `Board` with every net and reference renamed by a fixed permutation, and asserts equal geometry (segments and vias with names erased). It is stronger and cheaper than counting 20 CLI runs. A tier-1 CLI test over 20 renamings of the reference sits beside it.
- **P3.4** Placement **does** depend on reference names (§0, read). So the proposed order makes the *board-level* renamings 20/20 identical. The *specification-level* ones still differ in copper, because parts move, and some may still fail. Whether `place` is in this work order's fence ("`pcb.rs` where the escape is written") is a decision. Probe check: compare the placed parts (positions with references erased) before routing.

### S4. Zone outlines
- **P4.1** The reader can require each zone's polygon to be exactly one `polygon` with four `pts` equal to the board outline's corners, in the emitter's order. Nothing `emit` writes is refused. All 5 fixtures plus sentinel-01's committed board pass.
- **P4.2** The work order says "minus the edge clearance, as the emitter writes it". The emitter writes the outline itself, not an inset. So the judge is equality with the outline, and the inset is KiCad's at fill time. Saying this is a correction to the work order's wording, not a change.
- **P4.3** A board re-saved by KiCad 10.0.7 keeps the four points in order, so a KiCad round trip of an emitted board is not refused. The only thing refused that someone might call legitimate is a hand-redrawn zone, which `check drc --spec` already refuses as "not the declared board" in spirit.

## 2. Measurements

**Method.** Host only.
- Each probe is a patch to `route.rs` (S3 also patches `pcb.rs`) on `40951e4`, built `--release` in a detached worktree under `~/.cache/aet-1307`, with `systemd-run --user --scope -p MemoryMax=12–16G -p MemorySwapMax=0`.
- `spike/1307/run.sh` measures one binary over the five inputs: emit, time, vias and track length (`stats.py`), rip-ups, board sha, byte-identity with today's board, and `extract`'s bounds.
- Candidates (a), (b), (c) and S3 were built by four Sonnet subagents in their own worktrees, from one common brief. I spot-checked each claim the table relies on by re-running its binary (§2.2).

**KiCad was not run.** The auto-mode classifier denied the capped `podman run` of the pinned image (`spike/1307/img.sh`). The operator ruled "skip KiCad this spike". So:
- the S1 figure is a PNG rendered from the router's own dump (`plot.py`), not a KiCad plot;
- the DRC column below says **not run**;
- P4.3, the KiCad round trip, is **not measured**.

The router's exact judges (`check`, `connectivity`) ran on every board marked routed.

**Baseline (today's router, `40951e4`)** [measured]:

| input | result | s | vias | track mm | extract |
|---|---|---|---|---|---|
| reference (2 layers) | routed | 0.90 | 70 | 348.5 | pass 46,800 / 58,235 |
| 2 × 5 (2 layers) | routed | 5.38 | 132 | 650.0 | **fail** 106,609 / 93,600 / 101,943 / 91,276 |
| four-layer | routed | 0.14 | 70 | 201.4 | pass 45,857 / 48,827 |
| D1 (4 layers) | routed | 0.56 | 239 | 766.0 | **fail**: 7 of 8 lines over 100,000 fF (max 119,603) |
| D3 (4 layers) | `Unroutable VDD_3V3` after 500 rip-ups | 16.33 | – | – | – |

**Finding F1 (not this work order's, but it shapes §4.2).** `D1` and `2 × 5` route on `main` but **fail `extract`**: their I2C lines bound above their declared 100 pF. So "D1 routes and passes every gate" was never true of `extract`/`verify`. `4100` claimed DRC with parity only. Any D3 that routes will fail `extract` the same way (§2.2, c2).

### 2.1 S1 — why D3 fails [measured]

Probe `spike/1307/s1-trace.patch` (env-gated: `AET_TRACE` logs each stuck search, rip-up and plane pad's via sites; `AET_PARTIAL` returns the copper at give-up; `AET_DUMP` writes it as JSON). Log: `~/.cache/aet-1307/runs/d3-trace.log`.

- **Contested escapes (P1.1 held).** All 358 rip-up events and the last stuck searches are at U1's bottom-left corner:
  - plane pads, left side: **30 VDD_3V3, 31 GND, 32 VDD_3V3, 33 VDD_3V3** (four in a row);
  - CS escapes, left side: 34 `/CS0_0`, 35 `/CS0_1`, 36 `/CS1_2`;
  - round the corner, bottom side: 37 `/CS2_0`, **38 GND, 39 VDD_3V3**, 40 `/CS2_1`, 41 `/SCK0`, 42 `/MISO0`, 43 `/MOSI0`.

  The stuck searches are "cannot take U1 pad 32 / 31 / 39 to a via" and "cannot reach U1 pad 34 / 35 / 36 / 41". The decoupling capacitors placed by those pads take further sites.
- **Via sites (P1.2 held, with a nuance).** Counted within 16 pitches of each plane pad, in the plane's fill:
  - against pads, fields and edge alone (`hard_via`), every plane pad has free sites: at least 19 (U1 84), 22 (U1 33) and 39 (U1 31);
  - against live copper, U1 31 had **zero** in 12 of 166 attempts, U1 84 in 14 of 114, and U1 32, 33 and 107 at times;

  So it is contention and order, **not** a placement fact. The first §7 stop is not indicated.
- **Rip-up loop (P1.3 missed narrowly).** At give-up it is cycling. 32 distinct nets appear, but **8** nets make 82.5 % of the 503 blocker events (VDD_3V3 99, GND 85, `/CS0_0` 63, `/CS0_1` 43, `/S1000_HOLD` 39, `/CS2_1` 34, `/SCK0` 32, `/MISO0` 20); 7 make 79.1 %. The prediction said fewer than 8. The last cycle is VDD_3V3 (pad 32) ↔ `/CS0_0`, `/CS0_1`, `/S1000_HOLD`, GND (pad 31): each ripping the other back.
- **Cause (P1.4 held, sharpened):** *four adjacent plane pads at the LQFP's bottom-left corner need four vias at 0.8 mm pitch in the one outward strip that the seven CS/SPI escapes converging on that corner must also cross, because the pad field forbids every escape but outward.*
- **Figure:** `spike/1307/s1-d3-u1-pads-28-44.png`, the copper at give-up around U1 pads 28–44 (outlined), 40 px/mm.
  - Pads: GND dark green, VDD_3V3 orange, other nets grey.
  - Tracks: F.Cu red, B.Cu blue.
  - Vias: GND green, VDD_3V3 yellow, others magenta.

### 2.4 S4 — zone outlines [read + measured]

- **What the emitter writes (P4.1 held).** `spike/1307/s4_zones.py` over every board emitted at baseline (reference, 2 × 5, four-layer, D1) shows each zone with exactly one `polygon` of 4 `pts`, **exactly** the outline's corners in the emitter's order. No `filled_polygon` appears. The repository commits no `.kicad_pcb` with a zone (`container/fixtures/drc-*.kicad_pcb` have none). The strict corpus has two zone recipes [read]. Neither changes the polygon, so the proposed judge leaves their recorded verdicts as they are:
  - `board/unread-child/zone_fill_close.edit`: a saved fill; `drc` passes, `extract` refuses it `UnmodelledCopper`.
  - `zone_clearance.edit`: `drc` refuses it `LocalClearance`.

  The build must re-measure both.
- **The wording (P4.2 held).** The emitter writes the outline itself, not the outline minus the edge clearance. The judge is therefore **equality with the `Edge.Cuts` rectangle**. The edge clearance is KiCad's at fill time, and the router's `zone_core` already models it.
- **Proposal.** In `route::read`, each zone must have:
  - exactly one `polygon` child and exactly one `pts`;
  - exactly 4 `xy` points, which are the outline's four corners in the emitter's order (or, if the architect prefers, in any cyclic order and direction).

  Anything else is refused by name: "zone `GND` on In1.Cu: its outline is not the board's". That covers smaller, larger, offset, holed (a second polygon) and extra-vertex outlines. `filled_polygon` children are not read, so a board filled and saved in KiCad is not refused for its fill.
- **What legitimate input is refused:**
  - a plane deliberately pulled back by a user (for example from a mounting hole or an antenna). No specification field can declare that today, so it is not the declared board, and refusing it is `INV-06`'s fail-closed.
  - A KiCad 10.0.7 open-and-save of an emitted board is **not measured** (container denied). If KiCad rewrote the start corner or direction, the strict-order form would refuse it and the any-order form would not. **Decision D6.**

### 2.2 S2 — escape strategies [measured]

Each row is `run.sh` on that probe's binary.
- "same" means byte-identical with today's board.
- "ext" is `extract` (pass, or fail with its worst bound).
- Rip-ups are the run's total.
- KiCad DRC was **not run** (§2, method); the router's exact `check` and `connectivity` passed on every board marked routed.

| probe | ref (2 L) | 2 × 5 (2 L) | four-layer | D1 (4 L) | D3 (4 L) |
|---|---|---|---|---|---|
| **today** | 0.90 s, 70 v, 348.5 mm, ext pass 46,800/58,235 | 5.38 s, 132 v, 650.0 mm, ext fail | 0.14 s, 70 v, 201.4 mm, pass | 0.56 s, 239 v, 766.0 mm, ext fail | **Unroutable `VDD_3V3`**, 500 rip-ups, 16.3 s |
| **(a) escape first**, straight stubs, alternating rows | **Unroutable `/SPI_MISO`**, 0.02 s | **Unroutable `/MISO0`** | **Unroutable `GND`** | **Unroutable `GND`** | **Unroutable `GND`** (U1 107), 0.16 s, no rip-up |
| **(b) plane-pad priority**, joint, most-constrained first (v2) | same, 37 rip-ups | same | 0.20 s, 67 v, 186.6 mm, 20 rip, pass | 1.38 s, 225 v, 773.6 mm, 46 rip, ext fail | **Unroutable `GND`**, blocked by `VDD_3V3`, 500 rip-ups |
| **(c1) inward**, plane nets only | differs, 62 v, pass 46,410/43,741 | **Unroutable `VDD_3V3`** | 69 v, 171.2 mm, 0 rip | 231 v, 11 rip, ext fail | **Unroutable `/CS0_1`**, 0.25 s, no free path |
| **(c2) inward**, every net | differs, 58 v, 325.5 mm, pass 46,410/43,741 | 3.02 s, 138 v, 101 rip, ext fail | 69 v, 176.5 mm, 5 rip, pass | 230 v, 718.0 mm, 11 rip, ext fail | **routed**, 16.6 s, 519 v, 2,480 mm, **496 of 500 rip-ups**, ext fail (max 134,459) |
| **(c1p)** c1 only with planes | **same** | **same** | as c1 | as c1 | as c1 |
| **order B** alone (§2.3) | differs, 66 v, 352.4 mm, 48 rip, pass 45,085/54,793 | 3.21 s, 119 v, ext fail | 71 v, 14 rip, pass | 4.94 s, 238 v, 201 rip, ext fail | **Unroutable `/SCK2`**, 20.9 s |
| **order B + (c2)** (combined) | differs, 0.74 s, 58 v, 301.6 mm, 17 rip, pass 46,192/54,711 | 1.47 s, 132 v, 613.5 mm, 43 rip, ext fail | 0.05 s, 66 v, 170.2 mm, 3 rip, pass 49,558/45,671 | 0.44 s, 236 v, 727.4 mm, 19 rip, ext fail | **routed, 4.62 s, 525 v, 2,295 mm, 151 rip-ups**, ext fail (max 142,244) |

**What each probe taught.**
- **(a) is infeasible as written.** The cause is geometric, not a matter of tuning:
  - adjacent stubs are 0.5 mm apart, but a via needs 0.585 mm to a foreign track (0.3 + 0.2 + 0.075 + 0.01);
  - so the deeper of two adjacent stubs always passes beside the shallower one's via, whatever the row;
  - a planned escape must jog sideways, which is no longer "straight out".

  The subagent stopped there. It also broke boards that route today.
- **(b) moves the contention without removing it.** Joint, most-constrained-first stitching is better on `four` and D1 (fewer vias, 46 rip-ups against 259 for its own v1). But D3's two plane nets block each other at the same corner. Ordering cannot help when the strip has fewer via sites than the corner's pads need.
- **(c) removes the cause.** Opening the LQFP's interior (the pad ring's inside, the field shrunk past the pads' inner ends) lets an escape go inward, then down a via, then out under the pads on B.Cu. Only (c2), the variant open to every net, routes D3; plane nets alone (c1) make CS0_1 statically unroutable. Alone, (c2) routes D3 at 496 of 500 rip-ups, which is luck, not margin. **With order B it routes D3 in 4.6 s with 151 rip-ups**, and it routes every one of the 20 board-level renamings (§2.3).
- **Spot checks.**
  - I re-ran (c2) and (c1) on D3 myself (496 rip-ups, routed; `/CS0_1` in 0.24 s).
  - I recomputed the S3 counts from the subagent's TSVs.
  - I built and measured the combined probe myself (`s2-c-inward.patch` + `s3-order-b.patch`, `AET_C=2`).
- **Under (c) the dense part is chosen by rule.** It is a part whose pad field has pads on all four sides and no pad inside a ≥ 4 mm interior: the LQFP-144 on these boards. A QFN's exposed pad excludes it.

### 2.3 S3 — name independence [measured]

Renamings: seeds 0–19, each name given a seeded hash prefix (`rename_spec.py`, and the `AET_RENAME_SEED` hook in `pcb.rs`, `s3-rename-hook.patch`).
- *Board level* renames every net and reference in the `route::Board` after placement, so it measures the router alone.
- *Spec level* renames device references and bus nets in the specification, so it measures placement and router together.

No spec-level renaming was refused for anything but `Unroutable`.

| routed of 20 | today's order | order A (with half-perimeter) | **order B** | **order B + (c2)** |
|---|---|---|---|---|
| board: reference | 19 (19 distinct geometries) | 20 | **20, one geometry** | **20, one geometry** |
| board: D1 | 18 (18 distinct) | **0** | **20, one geometry** | **20, one geometry** |
| board: D3 | 0 | 0 | not run | **20, one geometry** |
| spec: reference | 11 | – | 13 | **20** |
| spec: D1 | 9 | – | 15 | 19 |
| spec: D3 | – | – | – | 13 |

- **Order B.** Nets are re-indexed once by `(!stitched, pad count, sorted pad centres (y, x), name)`, and a net's pads are ordered by centre `(y, x)`. Every index-ordered structure follows that key: the queue, the blockers re-queued after a rip-up, the owner ids, the stitched list and the tree root. The name is reached only if two nets have identical pad centres, which `check` refuses, so in practice the name is never consulted.
  - Order A added the pads' bounding-box half-perimeter after pad count. It is just as name-independent, but it **fails D1 every time** (`/SDA0`).
  - So a fixed geometric order makes routability a fact about the board, **not** a better fact: whether a board routes under it is one draw, the same for every renaming. Today, D1 routes under about 90 % of name orders.
- **Placement is the remaining dependence (P3.4 held).** `pcb::place` places in reference order with ties to the lower reference. Spec-level renaming moves parts: 7 distinct placements among 13 routed reference seeds under order B, and all distinct for D1. Hence order B + (c2) gives 20 / 19 / 13 at spec level, not 20.
- **Finding F2: the grid's margin claim is false.** `route.rs` says no tested board reaches the exact judge behind the grid ("belt-and-suspenders … no tested board reaches these"). Renamings reach it 5 times (an `Unroutable` with no net id). The cases:
  - today's order: board-level D1 seed 16, and spec-level D1 seeds 0, 12 and 16 (their messages were not kept);
  - order B + (c2): spec-level D3 seed 19.

  The two messages read are each a GND F.Cu track against a VDD_3V3 F.Cu track, ending diagonally 1 × 3 pitches apart: "track F (…) (GND) is within 200000 nm of track F (…) (VDD_3V3)". With 0.2 mm tracks that is 0.395 − 0.2 = 0.195 mm, under the 0.2 mm clearance.

  It fails closed (`Unroutable`, no id), so nothing ships. But a tracked end's corner is not covered by the node-to-node margin, and the stitched nets' F.Cu stubs hit it. The build should fix the margin (claim the end caps) rather than rely on the judge.
- **The guard (P3.3).** An `aethelgard-eda` unit test reads the emitted reference, `four` and D1 (via `route::read`), renames every net and reference by 8 fixed permutations, routes each, and asserts one name-erased digest (sorted segments and vias). A tier-1 CLI test over 20 board-level renamings of the reference, D1 and D3 is the work order's literal acceptance (§4), at about 0.5 s + 0.5 s + 4.6 s per renaming in release.

## 3. Predictions against measurement

| # | prediction | outcome |
|---|---|---|
| P1.1 | contest at U1 plane pads beside the SPI/CS escapes | **held**: U1 30–43, the bottom-left corner |
| P1.2 | sites exist but sit in signal lanes; at least one pad has none live | **held**: ≥ 19 hard sites everywhere; U1 31 and 84 at zero live |
| P1.3 | fewer than 8 nets make ≥ 80 % of rip-ups | **missed narrowly**: 8 nets make 82.5 %, 7 make 79.1 % |
| P1.4 | the cause sentence | **held**, sharpened (§2.1) |
| P2 (a) | D3 routes 55 %; the rest route | **wrong**: straight escapes break all five (§2.2) |
| P2 (b) | D3 does not route 70 %; the reference is identical | **held** |
| P2 (c) | D3 routes 75 %, the fastest; the others route | **half**: only c2 routes D3, at 496 rip-ups (slowest, not fastest); with order B, 4.6 s. c1 breaks 2 × 5 |
| P2 (c) DRC | KiCad passes vias under the body | **not measured** (container denied) |
| P2.4 | no rules or stackup change needed | **held** for (c2) + order B |
| P2.5 | (b) keeps the reference identical; bounds move < 10 % and stay under limits | **held** for (b). Under order B + (c2) the reference's bounds go 46,800 → 46,192 (−1.3 %) and 58,235 → 54,711 (−6.1 %), and still pass |
| P3.1 | today, ref 15–19 and D1 10–18 of 20; D3 0 | **held at board level** (19, 18, 0); **spec level lower** (ref 11, D1 9) |
| P3.2 | a geometric order gives one geometry for ref and D1 | **held for order B**; order A was name-independent but failed D1 |
| P3.3 | the guard is a renaming digest test | proposed (§2.3) |
| P3.4 | placement depends on names, so spec level still differs | **held**: placements move, spec-level counts < 20 |
| P4.1 | every emitted zone is exactly the outline | **held** (4 emitted boards, 6 zones) |
| P4.2 | the work order's "minus the edge clearance" is KiCad's, not the emitter's | **held** [read] |
| P4.3 | a KiCad round trip keeps the points | **not measured** (container denied) |

## 4. Recommendation per question

- **S1. The cause.** Four adjacent plane pads at the LQFP's bottom-left corner need four vias at 0.8 mm pitch in the one outward strip that seven converging CS/SPI escapes must also cross, because the pad field allows only outward escapes. It is contention, not placement: sites exist against pads and fields.
- **S2. The strategy: (c2) inward escape, with order B.** Recommended. Open the dense part's interior, the inside of the pad ring, to every net's F.Cu tracks and through vias. The ring itself and every other part's field stay closed. D3 routes in 4.6 s at 151 rip-ups, and so do all 20 board-level renamings. The other four inputs route with fewer vias and less track.
  - Not (a): infeasible at 0.5 mm pitch as written.
  - Not (b) alone: it cannot make sites that the strip does not have.
  - **The cost: the reference changes.** Its board, extraction bounds (−1.3 % and −6.1 %, still passing) and fabrication package all change. Gating (c) to boards with planes (`has_planes`) keeps the reference and 2 × 5 byte-identical under (c1p). But order B changes the reference anyway, so gating buys nothing once order B lands (D2).
- **S3. The order: order B.**
  - The key is `(!stitched, pad count, sorted pad centres, name)`. Nets are re-indexed by it once, so the queue, the blocker re-queue, the owner ids and the tree root all follow it.
  - A net's pads are ordered by `(y, x)`.
  - The guard is a digest test over renamings (§2.3).
  - It removes the router's name dependence entirely (one geometry per board at board level). It does not remove placement's: specification-level renamings still move parts (D3).
- **S4. The zone judge.** Each zone has exactly one `polygon` whose four points are the `Edge.Cuts` rectangle's corners. Anything else is refused by name; `filled_polygon` is not read. Nothing `emit` writes is refused. A deliberately pulled-back plane is refused, which is right while the specification cannot declare one.

## 5. Decisions needed

- **D1. Escape strategy.**
  - (i) **(c2) inward escape under the dense part's own body, for every net, with order B.** Recommended.
  - (ii) (c1p), inward for plane nets only with planes. This keeps the reference identical, but does not route D3.
  - (iii) Something else, for example jogged planned escapes (a′), which were not measured.

  Under (i), a via inside the LQFP's own courtyard is new. WO §5.2's "a via site under another part's courtyard" stays an escape lie the gate must refuse, and "own part's interior" becomes the one admitted case.
- **D2. Sentinel-01 changes** (WO §2.4). Under (i) the reference's routed board changes: 70 → 58 vias, 348.5 → 301.6 mm, bounds 46,800/58,235 → 46,192/54,711, all passing. So do its fabrication package and the `46_800, 58_235` literal in `stackups_tier1.rs`. Accept the change? Recommended: yes. The second §7 stop is not hit: no bound passes its declared value.
- **D3. Placement's name dependence.** At spec level, order B + (c2) routes 20/20 of the reference, 19/20 of D1 and 13/20 of D3. The rest fail because `pcb::place` orders by reference. The work order's acceptance ("20 renamings each route the reference, D1 and D3") is met at **board level** only.
  - (i) Read the acceptance as board level (the router's guarantee), and backlog a geometric placement order. Recommended, as the fence is `route.rs` plus "`pcb.rs` where the escape is written".
  - (ii) Widen the fence to `pcb::place` and make it name-independent too. This changes every placed board.
- **D4. "Passes every gate" for D3** (WO §2.1, §4). D3 under (i) routes and passes `check`/`connectivity`. Its I2C lines bound at 109,001–142,244 fF against a declared 100,000, so `extract` and hence `verify` fail. D1 and 2 × 5 **already fail `extract` on `main`** (F1).
  - (i) Read "every gate" as DRC with parity, as `4100` did for D1, and backlog D1/D3's bound. Recommended.
  - (ii) Raise the fixtures' declared bound so they pass, which changes the fixtures `4100` committed.
  - (iii) Treat the bound as this work order's (it is not routing).
- **D5. KiCad DRC of the chosen strategy.** It was not run in this spike (container denied; operator ruled to skip). The build runs the tier-2 gate on D3, D1, `four` and the reference under (i) before anything else. If KiCad refuses a via under the body, that is a §7 stop.
- **D6. Zone judge vertex order.** Strict (emitter's order) or any cyclic order and direction? Recommended: any order, because a KiCad round trip (not measured) may rotate the start. Both refuse every shape other than the outline.
- **F2 (for the build, no ruling needed unless the architect wants it separate).** The grid's margin misses a diagonal track-end corner (GND/VDD stubs, 0.195 mm). The exact judge catches it and fails closed. Fix the margin, so the "no tested board reaches these" comment becomes true, and add the reproducing renaming as a test.

## 6. Files

`spike/1307/` (deleted by the build):
- **Probes:** `s1-trace.patch`, `s2-a-escape-first.patch`, `s2-b-plane-priority.patch`, `s2-c-inward.patch` (env `AET_C` = `1`, `2`, `1p`), `s3-rename-hook.patch`, `s3-order-b.patch`. The combined probe is the last two applied together, with `AET_C=2`.
- **Runners:** `run.sh`, `stats.py`, `gen.py` (2 × 5), `rename_spec.py`, `placement.py`, `s3-board-level.sh`, `s3-spec-level.sh`, `s4_zones.py`, `plot.py`.
- **Not run:** `img.sh`, the capped image runner (container denied).
- **Data:** `s2-results.tsv`, `s3-renamings.tsv`, and the figure `s1-d3-u1-pads-28-44.png`.
- **Logs:** under `~/.cache/aet-1307/` (not committed).
