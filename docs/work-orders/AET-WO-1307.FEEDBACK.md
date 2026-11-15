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
- Candidates (a), (b), (c) and S3 were built by four Sonnet subagents in their own worktrees, from one common brief. I re-ran (c1) and (c2) on D3, recomputed the S3 counts from the TSVs, and built and measured the combined probe myself. Rows (a) and (b) are the subagents' reported runs and were **not** re-run by me; `a.tsv` was hand-formatted by its subagent.

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

**Finding F1 (not this work order's, but it shapes §4.2).** `D1` and `2 × 5` route on `main` but **fail `extract`**: their I2C lines bound above their declared 100 pF. So "D1 routes and passes every gate" was never true of `extract`/`verify`. `4100` claimed DRC with parity only. The two D3 boards that route here (c2, and c2 with order B) fail `extract` the same way (§2.2). `4100`'s FEEDBACK records no `extract` result for D1, so this is new.

### 2.1 S1 — why D3 fails [measured]

Probe `spike/1307/s1-trace.patch` (env-gated: `AET_TRACE` logs each stuck search, rip-up and plane pad's via sites; `AET_PARTIAL` returns the copper at give-up; `AET_DUMP` writes it as JSON). Log: `~/.cache/aet-1307/runs/d3-trace.log`.

- **Contested escapes (P1.1 held).** 277 of the 358 stuck searches (77 %) are at U1's bottom-left corner:
  - plane pads, left side: **30 VDD_3V3, 31 GND, 32 VDD_3V3, 33 VDD_3V3** (four in a row);
  - CS escapes, left side: 34 `/CS0_0`, 35 `/CS0_1`, 36 `/CS1_2`;
  - round the corner, bottom side: 37 `/CS2_0`, **38 GND, 39 VDD_3V3**, 40 `/CS2_1`, 41 `/SCK0`, 42 `/MISO0`, 43 `/MOSI0`.

  The corner's stuck searches: "cannot take U1 pad 31 / 39 / 32 to a via" (70, 40, 37) and "cannot reach U1 pad 35 / 34 / 41 / 40" (33, 33, 25, 18).

  A second, smaller cluster is U1's plane pads 84 and 107 (23, 6 %), whose live sites also hit zero. The rest (58) are spread over flash pads (`S1000`–`S1010`) and single U1 pads. The corner is where the loop ends, and it is the cause stated below; pads 84 and 107 show the same mechanism with fewer neighbours. The decoupling capacitors placed by the supply pads take further sites.
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
- **(c) removes the cause.** Opening the LQFP's interior (the pad ring's inside, the field shrunk past the pads' inner ends) lets an escape go inward, then down a via, then out under the pads on B.Cu. Only (c2), the variant open to every net, routes D3; plane nets alone (c1) make CS0_1 statically unroutable. Alone, (c2) routes D3 at 496 of 500 rip-ups, which is luck, not margin. **With order B it routes D3 in 4.6 s with 151 rip-ups**, and it routes every one of the 20 board-level renamings (§2.3). On 2 × 5 the via count is equal (132); on the others it is lower.
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
- **The guard (P3.3).** An `aethelgard-eda` unit test reads the emitted reference, `four` and D1 (via `route::read`), renames every net and reference **and permutes the order of `parts`** by 8 fixed permutations, routes each, and asserts one name-erased digest (sorted segments and vias). A tier-1 CLI test over 20 board-level renamings of the reference, D1 and D3 is the work order's literal acceptance (§4), at about 0.5 s + 0.5 s + 4.6 s per renaming in release.

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
  - A KiCad 10.0.7 open-and-save of an emitted board is **not measured** (container denied). If KiCad rewrote the start corner or direction, the strict-order form would refuse it and the any-order form would not. **Decision K6.**

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

Steering read after the merge of `origin/main` (PR #124): architecture §2.12, §2.21 and `RUL-03` with every amendment, and roadmap §7.1.
- `RUL-03` sets the gate's rules as clearance and width from `board.rules`, plus refusals of local overrides and waivers. Nothing in it or §2.12 forbids a via inside a part's own courtyard, so K1 (i) contradicts no ruling; it needs one because it is new.
- §2.21 records D3's failure as "needs escape planning", which this spike answers.
- The S4 judge is `INV-06`'s fail-closed, applied in `route::read`.


- **S1. The cause.** Four adjacent plane pads at the LQFP's bottom-left corner need four vias at 0.8 mm pitch in the one outward strip that seven converging CS/SPI escapes must also cross, because the pad field allows only outward escapes. It is contention, not placement: sites exist against pads and fields.
- **S2. The strategy: (c2) inward escape, with order B.** Recommended. Open the dense part's interior, the inside of the pad ring, to every net's F.Cu tracks and through vias. The ring itself and every other part's field stay closed. D3 routes in 4.6 s at 151 rip-ups, and so do all 20 board-level renamings. The other four inputs route with no more vias and less track.
  - Not (a): infeasible at 0.5 mm pitch as written.
  - Not (b) alone: it cannot make sites that the strip does not have.
  - **The cost: the reference changes.** Its board, extraction bounds (−1.3 % and −6.1 %, still passing) and fabrication package all change. Gating (c) to boards with planes (`has_planes`) keeps the reference and 2 × 5 byte-identical under (c1p). But order B changes the reference anyway, so gating buys nothing once order B lands (K2).
- **S3. The order: order B.**
  - The key is `(!stitched, pad count, sorted pad centres, name)`. Nets are re-indexed by it once, so the queue, the blocker re-queue, the owner ids and the tree root all follow it.
  - A net's pads are ordered by `(y, x)`.
  - The guard is a digest test over renamings (§2.3).
  - It removes the router's dependence on net names and on the order of a net's pads: one geometry per board at board level. The board-level hook renamed references **after** `pcb.rs` had sorted the parts, so the order of `parts` was not exercised. The build should order parts by position where the router reads them, and the guard should also shuffle `parts`. It does not remove placement's: specification-level renamings still move parts (K3).
- **S4. The zone judge.** Each zone has exactly one `polygon` whose four points are the `Edge.Cuts` rectangle's corners. Anything else is refused by name; `filled_polygon` is not read. Nothing `emit` writes is refused. A deliberately pulled-back plane is refused, which is right while the specification cannot declare one.

## 5. Decisions needed (K1–K6; `D1`, `D3` are the fixtures)

- **K1. Escape strategy.**
  - (i) **(c2) inward escape under the dense part's own body, for every net, with order B.** Recommended.
  - (ii) (c1p), inward for plane nets only with planes. This keeps the reference identical, but does not route D3.
  - (iii) Something else, for example jogged planned escapes (a′), which were not measured.

  Under (i), a via inside the LQFP's own courtyard is new. WO §5.2's "a via site under another part's courtyard" stays an escape lie the gate must refuse, and "own part's interior" becomes the one admitted case.
- **K2. Sentinel-01 changes** (WO §2.4). Under (i) the reference's routed board changes: 70 → 58 vias, 348.5 → 301.6 mm, bounds 46,800/58,235 → 46,192/54,711, all passing. So do its fabrication package and the `46_800, 58_235` literal in `stackups_tier1.rs`. Accept the change? Recommended: yes. The second §7 stop is not hit: no bound passes its declared value.
- **K3. Placement's name dependence.** At spec level, order B + (c2) routes 20/20 of the reference, 19/20 of D1 and 13/20 of D3. The rest fail because `pcb::place` orders by reference. The work order's acceptance ("20 renamings each route the reference, D1 and D3") is met at **board level** only.
  - (i) Read the acceptance as board level (the router's guarantee), and backlog a geometric placement order. Recommended, as the fence is `route.rs` plus "`pcb.rs` where the escape is written".
  - (ii) Widen the fence to `pcb::place` and make it name-independent too. This changes every placed board.
- **K4. "Passes every gate" for D3** (WO §2.1, §4). D3 under (i) routes and passes `check`/`connectivity`. Its I2C lines bound at 109,001–142,244 fF against a declared 100,000, so `extract` and hence `verify` fail. D1 and 2 × 5 **already fail `extract` on `main`** (F1).
  - (i) Read "every gate" as DRC with parity, as `4100` did for D1, and backlog D1/D3's bound. Recommended.
  - (ii) Raise the fixtures' declared bound so they pass, which changes the fixtures `4100` committed.
  - (iii) Treat the bound as this work order's (it is not routing).
- **K5. KiCad DRC of the chosen strategy.** It was not run in this spike (container denied; operator ruled to skip). The build runs the tier-2 gate on D3, D1, `four` and the reference under (i) before anything else. If KiCad refuses a via under the body, that is a §7 stop.
- **K6. Zone judge vertex order.** Strict (emitter's order) or any cyclic order and direction? Recommended: any order, because a KiCad round trip (not measured) may rotate the start. Both refuse every shape other than the outline.
- **F2 (for the build, no ruling needed unless the architect wants it separate).** The grid's margin misses a diagonal track-end corner (GND/VDD stubs, 0.195 mm). The exact judge catches it and fails closed. Fix the margin, so the "no tested board reaches these" comment becomes true, and add the reproducing renaming as a test.

## 6. Files

`spike/1307/` (deleted by the build):
- **Probes:** `s1-trace.patch`, `s2-a-escape-first.patch`, `s2-b-plane-priority.patch`, `s2-c-inward.patch` (env `AET_C` = `1`, `2`, `1p`), `s3-rename-hook.patch`, `s3-order-b.patch`. The combined probe is the last two applied together, with `AET_C=2`.
- **Runners:** `run.sh`, `stats.py`, `gen.py` (2 × 5), `rename_spec.py`, `placement.py`, `s3-board-level.sh`, `s3-spec-level.sh`, `s4_zones.py`, `plot.py`.
- **Not run:** `img.sh`, the capped image runner (container denied).
- **Data:** `s2-results.tsv` (labels: `b` appears twice, v1 then v2; `s3order` is order A, `s3orderB` is order B; `a` rows are hand-formatted), `s3-renamings.tsv`, and the figure `s1-d3-u1-pads-28-44.png`.
- **Logs:** under `~/.cache/aet-1307/` (not committed).

---

# Build (§2), to the ruling (K1–K6, F2)

**Base:** `origin/main` `c621461` (AET-WO-4200) merged as `34b4f8a`; build commits `f798bc3` onward. The spike's sections above are unchanged. `spike/1307/` is deleted by the build; its probes stay in git history at `f4a09e7`, and its tables are the ones above.

Labels as above: **[measured]**, **[read]**, **[inferred]**.

## B1. K5: the KiCad gate, first [measured]

Before any build code, the spike's two probes (`s2-c-inward.patch` + `s3-order-b.patch`, `AET_C=2`) were built on `34b4f8a`. ERC, then `check drc --spec --schematic` (the specification's rules, schematic parity) ran in the pinned image on the reference, `four`, D1 and D3: **every board exits 0 with no finding of any kind**, `unconnected_items` included. KiCad 10.0.7 does not refuse a via under the LQFP's own body. No §7 stop.

The same gate on the final binary (`409d7b7`'s tree) also exits 0 with no finding on the reference, `four`, D1, D3 and D1 at 2 layers (B5). The board sha256 prefixes are `59a8f1dc`, `455def47`, `72a43bee`, `d039f6cb` and `4651cfb4`. It is now held by a tier-2 test (B7).

**Containers.** Every container this build ran used `--memory 8g --memory-swap 8g --pids-limit -1` and read back `memory.max=8589934592 memory.swap.max=0 pids.max=max`; `~/.cache/aet-1307/caps.log` holds each line. **Correction during the build:** until the architect's correction of kickoff rule 3 (2026-11-15), the runner also tried to write `0` into the container's `memory.swap.max` after start, as rule 3 then said. Furnace denies writes under `/sys/fs/cgroup`, and the correction removed the write. The runners (`~/.cache/aet-1307/{img,ci-img}.sh` and the adversary's) now only read back the three values and kill the container if `memory.swap.max` is not 0.
- **Containers started before the correction** (each with the write, each reading back `memory.swap.max=0`): the two K5 runs, the tenting export, the final K5, the CI-layout build, the bridge build, the timed tier-2 test, and the tier-2 suites.
- **After the correction, podman 5.7 does not set `memory.swap.max` to 0.** With equal `--memory 8g --memory-swap 8g` and no write, the adversary's only container read back `memory.max=8589934592 memory.swap.max=8589934592 pids.max=max`, and the runner killed it as the rule requires. This matches the Furnace global note (AET-WO-2101 F11: podman 5.7 writes `--memory-swap` into `memory.swap.max`).
- **So no container could run under the corrected rule.** I did not work around it. The architect then **replaced rule 3** (`~/.cache/aet-bus/RULE3.md`, 03:40 UTC): a capped systemd user scope (`MemoryMax=8G`, `MemorySwapMax=0`, `TasksMax=8192`), the container in it via `podman run --cgroups=disabled` with no memory or pids flags, and the caps read (never written) inside the scope before the container.
- **K5 again, under the replaced rule, on the final binary** (`~/.cache/aet-1307/img3.sh`). The scope read back `memory.max=8589934592 memory.swap.max=0 pids.max=8192`. ERC and DRC with parity exit 0 with no finding on the reference, `four`, D1, D3 and D1 at 2 layers, the boards after the A1 fix (board sha256 prefixes `59a8f1dc`, `ecd09af8`, `6d4e9d86`, `04b3f84f`, `4651cfb4`). Each DRC took 0.8–1.2 s.

## B2. K1: the inward escape and the via rules [read + measured]

- **The dense part** is chosen by rule, in `route::ring_interior`, with its reason in the doc comment. A part qualifies when it has pads on all four sides (pads near the field's centre lines), a free interior of at least `DENSE_INTERIOR_NM` = 4 mm each way, and no pad inside it, so a QFN's exposed pad excludes it.
  - On every tested board, only the LQFP-144 qualifies. The SOIC-8, LGA-8, WSON-6 (exposed pad) and 0402 parts do not.
  - `route.rs::tests::the_dense_part_is_chosen_by_rule` pins 4 mm as a literal, plus the 2-side, 3-side and exposed-pad cases.
- **Opened.** Inside a dense part's own ring interior, F.Cu tracks of every net and through vias are admitted (c2). The ring, its pads' gaps and every other part's field stay closed.
  - A part's interior opens only that part's restrictions. A spike-probe defect was not carried over: it marked the interior globally, so a later part's field could have been unblocked.
- **The courtyard rule (the ruling's "outside any other part's courtyard"). A fork, taken by measurement.**
  - `route::Part` now carries its F.Cu courtyard as straight edges, read from the footprint: `fp_line`, `fp_rect` and `fp_poly`; an `fp_arc`/`fp_circle` on F.CrtYd is refused by name, never approximated.
  - **No via's copper may enter any part's courtyard, except inside that part's own ring interior.** Exactly: the via's centre is neither inside the outline nor on it, and no outline edge passes nearer than `VIA_NM/2`, in i128 integer arithmetic. A tangent via passes; 1 nm nearer fails.
  - The router blocks such via sites, and `route::check`, the exact judge that `route()`, `extract` and `fab` run, refuses one: "via (x, y) (net) is in U4's courtyard".
  - **Why copper, not centre.** The ruling says "a via under another part's courtyard stays an escape lie the gate refuses". That is **not** what `main` did: nothing judged courtyards.
    - On `main`'s reference, one via centre sits inside U4's (SOIC-8) courtyard. My first count, by bounding box, said 2 to 34, but the SOIC's courtyard is a cross, and the exact polygon test gives 1. Many vias' copper overlaps passives' courtyard lines by under 0.3 mm.
    - A probe in the k5 tree measured both definitions over the four inputs:

| rule | ref | four | D1 | D3 |
|---|---|---|---|---|
| none (the ruled probe) | routed, 17 rip | routed, 3 | routed, 19 | routed, 151 |
| via centre outside other courtyards | routed, 17 | routed, 3 | routed, 19 | **Unroutable `/MOSI1`** |
| **via copper outside other courtyards** | routed, 27 | routed, 3 | routed, 25 | routed, 103 |

    The copper rule is the stricter one, and it routes everything (20/20 board-level renamings, one geometry each), so it was taken. The centre rule is the weaker one, and it breaks D3 (routing is chaotic in its constraints).
- **Vias in the ring touch no pad.** Each via site keeps the pads' clearance (`via_reach`), and `check` judges every pad against every via exactly.
- **Tenting [measured].** The emitter writes no `tenting` node and no board `setup`, so KiCad's defaults apply.
  - The F.Mask and B.Mask Gerbers of D3 and the reference were exported in the image (`kicad-cli pcb export gerbers --layers F.Mask,B.Mask`, the default plot settings, as `fab` uses). They carry **no** round 0.6 mm aperture.
  - D3's F.Mask has 574 flashes, all of pad apertures (rect and roundrect), and B.Mask has none. The vias, 525 on that probe board, are covered by solder mask on both sides.
  - Nothing was changed. `FAB-NOTES.md` does not need to say it: the mask Gerbers are the fabricator's statement of tenting.
  - `fab`'s own call is `kicad-cli pcb export gerbers --layers … --check-zones` [read, `fab.rs`]. It passes no option touching the masks or tenting, so the package's masks are the ones measured.

## B3. Order B and the guard (K3) [measured]

- Nets are indexed once by `(stitched first, pad count, pads' centres (y, x) sorted, name)`, and a net's pads are ordered by centre. Every index-ordered choice follows the index: the queue, the blockers taken up, owner ids, the stitched list and the tree root.
- The name decides only between two nets with identical pad centres, which `check` refuses. `Board.parts` order and each part's pad order are read only through these keys.
- **The guard is `escape_tier1.rs`.** For the reference, D1 and D3, the emitted board is read with `route::read` and renamed by 20 fixed SplitMix64 seeds:
  - every net (ground keeps its name at 2 layers, because that name is what makes it stitched there; plane nets are renamed in the stack too; `unconnected-` nets keep their name);
  - and every reference;
  - with `parts` and each part's `pads` permuted.

  Each is routed, and **after naming the nets back, the copper equals the board's own exactly**: 60/60.
- Mutants (B8): ordering by name (m8) and pads by reference (m9) each fail it.
- **Spec-level** renaming, end to end with placement (`rename_spec.py`, seeds 0–19), information for K3:

| routed of 20 | `main` (spike) | ruled probe (spike) | build `f798bc3` | **build, final (A1 fixed)** |
|---|---|---|---|---|
| reference | 11 | 20 | 16 | **16** |
| D1 | 9 | 19 | 18 | **18** |
| D3 | – (never routed) | 13 | 16 | **15** |

  The reference's four failures (seeds 9, 11, 12, 17) are all `VDD_3V3: cannot reach U4 pad 8`. Under the courtyard rule, U4's supply pad has fewer via sites. The remaining failures are SCL and MOSI nets at sensor and flash pads: final D3 fails at seeds 0, 7, 10, 11 and 16. None of the 60 reached the exact judge (`~/.cache/aet-1307/sweep-build2.tsv`). **The spec-level count for the reference is lower than the spike's ruled probe (20).** The cause is the courtyard rule (B2), so it is a finding for the backlog row on geometric placement (K3), not a stop.

## B4. The reproducers (K3) [measured]

`escape_tier1::the_reproducers_of_1102_a6_and_2204_a5_route` covers:
- 1102 A6: `FLASH_CS` renamed `nCS`, `cs`, `Z`, `a`, `~`, `_X`, `U`, plus 1102 §4 deviation 4's six nets renamed `N0`…`N5` in the reference's own order;
- 2204 A5: the references `Peripherals` and `Sysbus_` on the first I2C and the first SPI device. The adversary's original specifications are schema version 1, so they were regenerated from today's reference by the same rule.

On `main` (`34b4f8a`, release), the eight A6 cases and the two I2C A5 cases give `Unroutable`; the two SPI A5 cases route. **On the build, all 12 route.**

## B5. K4 and the fixtures [measured]

- **D3 routes at 4 layers** in 3.7 s (release) and passes DRC with parity (B1). It has 521 vias. Its I2C lines' `extract` bounds still exceed the fixtures' declared 100 pF: 109,657–130,500 fF on the final board. As ruled, the declared bound is unchanged; this is F1's backlog row.
- **New: D1 at 2 layers now routes** (2.2 s release), and it passes DRC with parity (B1). `4100`'s test asserted it failed closed there. The test is now `d1_routes_at_four_layers_and_at_two`. D3 at 2 layers still fails closed (`/SDA1: cannot reach R3 pad 2`, 11.6 s), which no test holds; its cost was not worth CI time.
- 2 × 5 (`4100`'s `many(2, 5)`) routes (126 vias, 2.7 s).

## B6. F2: the cause and the fix [measured]

- **The cause** was narrower than the spike's reading. `Router::search` seeded its sources unpriced:
  - a pad's terminal nodes, which are open by `hard_track`, the pads alone;
  - or a tree's nodes.

  Moves *into* a node were judged against laid copper, but a path's first node was not. A track could therefore start where a foreign track end already stood within clearance: a GND and a `VDD_3V3` F.Cu end at (0.125, 0.375) mm, 0.395 mm centre to centre, against 0.4 needed.
- **The fix:** a source is priced exactly as a move is (free; soft, at `SOFT_COST`; or barred), and `blockers` counts the source as a step.
- **The reproducer**, found by mutant: the build minus only that pricing (m1).
  - The ruling's candidates do not discriminate in the final tree. Board-level D1 seed 16 collapses into order B's single geometry. Spec-level D3 seed 19 now fails on `/SCL1`.
  - At `f798bc3`, D3 itself reached the exact judge under m1. After the A1 fix (B10) it no longer does, and m1 survived the D3 tests: a guard had lost its test.
  - The adversary's geometry sweep supplies the replacement. On the 4-layer fixture's board, each part of at most 8 pads is moved by a seeded offset (seeds 0–299, 57 skipped for overlap).
    - The final build reaches the exact judge **0** times (`four` 0/243, D1 0/239). m1 reaches it 11 and 3 times.
    - Every hit is a GND and a `VDD_3V3` F.Cu end within clearance (D1's: `/SCL3` against `/SDA3`).
  - `escape_tier1::shifted_placements_route_without_reaching_the_exact_judge` holds seeds 5, 6, 12, 22 and 48 (1.2 s). Under m1 it fails: "seed 5: Unroutable { net: "", … GND … VDD_3V3 }". Evidence: `~/.cache/aet-1307/f2probe/`.
- **The comment "no tested board reaches these" is true at the build:** no run of the sweep (60 spec-level renamings, 12 reproducers, 5 inputs) or of the tests reached the exact judge. The comment is kept.

## B7. K6: the zone judge, and the corpus [measured]

- `route::read` requires each zone to have exactly one `polygon` with one `pts` of exactly four `xy` points, which are the `Edge.Cuts` rectangle's corners in any cyclic order and direction (a diagonal walk of the corners is refused). Anything else gives "zone `NET` on LAYER: its outline is not the board's". `filled_polygon` is not read.
- `escape_tier1::a_zone_that_is_not_the_board_outline_is_refused_by_name` covers the `four` GND plane and the reference's B.Cu zone:
  - 8 accepted orders (4 rotations × 2 directions) and a saved fill;
  - refused: smaller, larger, offset, diagonal, extra vertex, three corners, a corner twice, and holed (a second polygon).
- **The two corpus recipes, re-measured** (`strict_tier1`, tier 1; the KiCad half in the image, B9):
  - `zone_fill_close` keeps `drc 0 pass; extract 3 UnmodelledCopper`;
  - `zone_clearance` keeps `drc 3 LocalClearance`.

  The judge refuses neither, because neither changes the polygon.
- **Nine other corpus recipes no longer applied.** They are anchored to text of the reference board that the new routing removed:
  - `escape/pintype_{hex,octal}_no_connect`;
  - `repeated-child/{big_plate, dup_end, dup_net_seg, dup_via_size, dup_width, segment_layer_and_layers}`;
  - `unread-child/via_padstack`.

  Each is re-anchored to the equivalent item of the new reference board:
  - the pintype recipes remove U1 pad 137's SDA branch, F.Cu, via, B.Cu, via, F.Cu, up to the junction at (100.125, 85.125);
  - the others use B.Cu SDA segment 51, F.Cu SDA segment 40 and SDA via 7.

  The recorded paths follow (`segment[53]` → `segment[51]`, `via[10]` → `via[7]`, `big_plate` stays `segment[40]`), and so does `dup_end`'s note. The `main:` history lines are left as written.

## B8. Mutation (sized to the diff) [measured]

One mutant per guard this build adds, each run against the tests it lives in (`~/.cache/aet-1307/{mutants.py,run-mutants.sh,mut-*.log}`):

| # | mutant | killed by |
|---|---|---|
| m1 | search sources unpriced (F2) | D3's emit (judge refuses) |
| m2 | router ignores courtyards | the reference's emit (`check`: via in U3's courtyard) |
| m3 | `check` ignores courtyards | `a_via_in_a_courtyard…` |
| m4 | courtyard rule by centre only | `a_via_in_a_courtyard…` (the copper case) |
| m5 | no own-ring exemption | `a_via_in_a_courtyard…` (U1's interior) |
| m6 | zone outline not judged | `a_zone_that_is_not…` |
| m7 | zone corners in any walk | `a_zone_that_is_not…` (diagonal) |
| m8 | nets ordered by name before centres | D1 and reference renamings |
| m9 | a net's pads by reference | D1 and reference renamings |
| m10 | dense interior 3 mm | `the_dense_part_is_chosen_by_rule`, **after a fix**: the first form compared against the constant itself and let m10 survive; the threshold is now a literal |
| m11 | exposed pad admitted | `the_dense_part_is_chosen_by_rule` |
| m12 | no inward escape | D3's emit (`/SCK1` after 500 rip-ups) |
| m13 | the router lets a ring via touch a pad (A1) | `a_via_in_the_ring_may_not_touch_even_its_own_nets_pad` |
| m14 | `check` lets a via touch a pad (A1) | the same |
| m15 | "touch" made strict (`<`, a tangent admitted) | the same (exact at the box) |
| m16 | dangling ends not reported (A3) | `a_via_in_a_courtyard…` (the U4 pad-2 stub) |
| m17 | an `fp_poly`'s non-`xy` points dropped (A2) | `reading_refuses_what_the_router_does_not_model` |
| m18 | a courtyard edge twice accepted (A2) | the same |
| m19 | an unclosed courtyard accepted (A2) | the same |
| m20 | a footprint without a courtyard accepted (A2) | the same |

**Re-run on the final tree** (`~/.cache/aet-1307/mutants2.out`): 17 killed at once. Three needed work:
- **m1 survived.** Its D3 reproducer had gone with A1's fix. It is killed by the new F2 test (B6).
- **m12 had to be re-anchored** after the reduction moved its line; it is then killed.
- **m17 did not compile as first written.** As written now, it does compile and is killed.

Every kill was checked to be a failed assertion, not a build failure.

## B9. Changed committed files, each with its cause

- **K2 (the reference's routed board changed: 70 → 64 vias, 348.5 → 331.6 mm):**
  - `crates/aethelgard-cli/tests/stackups_tier1.rs`: the `46_800, 58_235` literal is now `50_576, 43_749`, with a comment naming the ruling.
  - `crates/aethelgard-cli/tests/extract_tier1.rs`: the reference's terms, lengths, segment and via counts, the 20 pF messages, and the 50 pF case, which now fails SDA, not SCL.
  - The BME280 case of `a_foreign_pad_closer_than_the_clearance_to_a_line_is_refused` now moves the I2C pad, because the moved foreign pad's new track no longer reaches it, so connectivity refused first.
  - The nine corpus recipes (B7).
- **The ruled numbers differ from the spike's: cause, the K1 courtyard rule (B2).** The reference's bounds are **50,576 and 43,749 fF** (both pass 100,000), not the ruled 46,192 and 54,711. The fabrication package changes with the board; the repository commits none.
- **K1/K2, the 4-layer fixture:** `stackups_tier1::the_four_layer_board_is_the_declared_one_and_extracts`' bounds `45_857, 48_827` → `49_641, 45_671`.
- **K1, D1 at 2 layers:** `stackups_tier1`'s D1 test (B5); D3's former fail-closed test is removed (B6).
- **K1:** `routing_tier1::every_pad_is_left_straight_out_of_its_field` admits a dense part's own ring interior.
- **`crates/aethelgard-cli/README.md` line 578** (in scope by the architect's review): the example `extract` line is now the reference's measured SDA line, bound 50,576 fF, trace 12,922, length 33,750,000 nm, 28 segments. It was 46,800 / 9,146 / 24,250,000 / 19.
- **The merge of `origin/main` `50d7a6f` (AET-WO-4300c merged first) [measured on the merged tree, release].**
  - Two conflicts, both literals: `extract_tier1` and `stackups_tier1` now also bound SCK and MOSI. Resolved as agreed in the mailbox: D1 keeps specification schema 3 and 4300c's SCK/MOSI constraints, at 150 pF for I2C. Git combined 4300c's extract check with this work order's `d1_routes_at_four_layers_and_at_two` without a conflict.
  - Each literal re-measured:

| literal | `main` (4300c's routes) | merged (1307's routes) |
|---|---|---|
| reference SDA / SCL / SCK / MOSI bound (`the_old_and_new_paths_agree_on_the_reference`) | 46,800 / 58,235 / 18,521 / 17,188 | **50,576 / 43,749 / 16,972 / 13,772** |
| reference SCK terms (trace, vias, pads, pins, bound) | 5,920, 3,038, 1,563, 8,000, 18,521 | **4,371, 3,038, 1,563, 8,000, 16,972** |
| reference MOSI terms | 4,587, 3,038, 1,563, 8,000, 17,188 | **4,209, 0, 1,563, 8,000, 13,772** |
| `a_declared_spi_bound…`: under / at | 18,520 / 18,521 | **16,971 / 16,972** |
| 50 pF verdicts (SDA, SCL, SCK, MOSI) | pass, fail, pass, pass | **fail, pass, pass, pass** |
| `four` SDA / SCL / SCK / MOSI | 45,857 / 48,827 / 18,470 / 17,114 | **49,641 / 45,671 / 15,264 / 16,439** |

  - **D1 at 4 layers against 4300c's declarations:** every line passes, and the test holds it.
    - I2C (150,000 fF): SDA0 114,235, SCL0 103,483, SDA1 118,205, SCL1 98,854, SDA2 104,704, SCL2 116,644, SDA3 118,375, SCL3 113,015.
    - SCK0 15,264 and MOSI0 16,439 (30,000 fF).
  - The reference's I2C lines are unchanged by the merge. Main's reference specification changed (schema 3, the flash part), so its board bytes differ.
- **Protected documents, findings only:**
  - `docs/architecture.md` §2.21 records D3 as needing escape planning;
  - the router's order is described by name wherever the steering documents state it [inferred: not re-read line by line].

## B10. Adversarial pass (full; a fresh Opus subagent, on a frozen copy at `f798bc3`) [measured by the adversary; each fix measured by me]

- **Harness and evidence:** `~/.cache/aet-1307-adv/` (`probe/…/examples/adv.rs`, `lies*.py`, `logs/`).
- **KiCad did not run in the adversary's pass.** Its one container was stopped by the cap guard (B1), and a host `check drc` was denied by the permission classifier. Every "extract pass" below is therefore "DRC not run".

| # | §5 | finding | severity | disposition |
|---|---|---|---|---|
| A1 | 2 / K1 | The router put vias **into U1's own pads** inside the ring. On `four`, D1 and D3, three VDD_3V3 vias overlap pads 144, 108 and 72 by 0.25 / 0.16 / 0.16 mm, and the drill cuts into the pad. The ring's opening lifted the field's via ban, and a same-net pad's claim admits its own net. Nothing refused it, against the ruling's "they must not touch any pad" and the WO's "not … via-in-pad". | **high** | **Fixed.** The router closes every via site in the ring whose copper touches any pad (any net), and `route::check` refuses a via that touches any pad anywhere ("via … touches U1 pad 144"). Outside rings the field rule already keeps vias clear, so the reference is byte-identical. `four`, D1 and D3 change (D3: 514 vias, routes in 4.1 s), and no via of any of them touches a pad (re-measured). `four`'s extract bounds are unchanged. Held by `route.rs::tests::a_via_in_the_ring_may_not_touch_even_its_own_nets_pad` (exact at the pad box; the router's node closed). Mutants m13–m15 are killed. |
| A2 | 2 | Courtyard holes. A missing courtyard, every edge duplicated (even-odd cancels), an `fp_poly` with an `arc` in its `pts` (silently dropped, against `part()`'s own comment) and an unclosed outline all passed `extract`. A *shrunken* courtyard passed too. | medium | **Fixed, but for the shrunken case.** `part()` refuses: a footprint with copper pads and no courtyard; an edge twice (either direction); an outline that is not closed (a corner met an odd number of times); and anything but `xy` in an `fp_poly`'s points. Held by `reading_refuses_what_the_router_does_not_model`'s new cases; mutants m17–m20 are killed. **A shrunken but closed courtyard** can only be told from the library's, which is KiCad's `lib_footprint_mismatch` (unmeasured here). It is a named residual. |
| A3 | 2 | Dangling stubs (an F.Cu stub off U4 pad 2; B.Cu stubs off a via) passed `extract`. `connectivity` required only that copper be one piece. | medium (conditional on DRC) | **Fixed.** `connectivity` refuses a track whose end joins none of its net's pads, vias or tracks: "…ends at (x, y), joined to nothing". It is reported after every other judgement, so earlier refusals keep their reasons. Held by `escape_tier1::a_via_in_a_courtyard…` (the U4 pad-2 stub); mutant m16 is killed. |
| A4 | 3 | A board-level adversary (a 249 mm board, 4 LQFP-144 copies, 288 random two-pad nets, built as `route::Board`) fails closed after 500 rip-ups in 616 s (568 MB). | low/medium | **Residual, named.** It never hung, because `RIPUPS` bounds it. `emit` cannot produce this board: one MCU per specification, and the outline is the parts' box plus 2 mm. A wall-time bound on the router is a backlog question for the architect. |
| A5 | 2 | `ring_interior` admits parts that are not quad rings: four single pads round a 6 mm interior, or a side's one stray pad up to 30 mm away. | low | **Residual.** It is the ruled rule as written ("pads on all four sides, ≥ 4 mm, no exposed pad"). Only the LQFP-144 qualifies among the vendored footprints, and the router places only vendored parts. Tightening it (a pad count per side) would change the ruled rule, so it is a question for the architect. |
| A6 | 1 | Two same-net pads with one centre (nested) route under one `parts` order and fail closed at the exact judge under the other. | low | **Residual**, the case `net_pads`' comment names. It fails closed, and no vendored footprint overlaps pads. "No tested board reaches these" stays true of tested boards. |
| A7 | 1/F2 | With F2's `blockers` first-step hunk reverted as well, nothing tells it apart. Its geometry sweep (B6) showed the exact judge reached by the sources-unpriced mutant, never by the build. | low | **Kept, named.** It is the soft search's counterpart to the source pricing (a path may start where it is not free, and that node's owner is a blocker). No test separates it. The sweep's seeds became F2's test (B6), which the source-pricing hunk alone already passes. |
| A8 | 4 | `route::read` alone accepts a `keepout` zone, a `filled_polygon` and quoted numbers in `xy`. `extract` refuses the first two (UnmodelledCopper). | info | **No change.** K6 says `filled_polygon` is not read. Zone type and token form belong to the strict reader (AET-WO-1306) in front of every gate. |
| I1 | 1 | Spec-level random renamings: `Unroutable` for the reference 1 of 98, D1 2/45, D3 9/29, D3 net-only 1/22. | info | K3 (placement order, backlog). |

**Withstood** (the adversary's evidence):
- **§5.1, board level.** Renamings, reversed/empty/control-character/1 KB names, and parts, pads and pad numbers reversed or rotated give identical copper on the reference, `four`, D1 and D3.
- **F2.** Over geometry sweeps (`four` 300 seeds, D1 100), the build reached the exact judge 0 times; the sources-unpriced mutant 13 and 2 times.
- **The courtyard predicate.** It agreed with an independent exact oracle on 140,288 points, tangency ±1 nm included.
- **The ring.** Another part's courtyard inside U1's ring refuses a via there.
- **§5.3, the escape plan's own attacks** (alternating plane pins, plane/plane/signal patterns) fail closed in 6–15 s or route.
- **§5.4.** Every zone variant is refused by name (smaller, larger, offset, extra or duplicated points, zero area, bow-tie, diagonal, holed, `arc`, two `pts`, three-value `xy`), and the legitimate rotations and reversals are accepted.

## B11. Reduction pass (diff)

Reduction pass (diff): −3 net (`bae55aa`) · 1 duplicate removed (`ring_interior` computed in two per-part loops of `Router::new`; one loop now; boards byte-identical; m2 still killed) · 1 dead path dropped before commit (an in-fill exemption for dangling ends, written with A3's fix, which router output never reaches and no test told apart; refusing such a hand-made board is fail-closed) · 0 protected kept · 2 findings (`escape_tier1`'s helpers repeat `stackups_tier1`'s, the house pattern per test file, and `common::verify`'s helpers log 3301's `ANSWER` lines, so reuse would be a mismatch; `crates/aethelgard-cli/README.md` line 578, B9).

## B12. Gates and CI

- **Host gates** (every run under a capped user scope): `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --features aethelgard-cli/with-tools -- -D warnings`, `cargo test --workspace`.
- **The first build CI run, at `ccb8dbc`** (run 43667202095): green.
  - Shards: 14m30s, 13m49s, 11m49s, 13m57s, 13m17s, **15m28s** (shard 5), 13m28s, 14m31s; `gates` 10m03s.
  - The aggregate: `tier2_tests_passed=97 floor=97`. `TIER2_MIN` is exact.
- **Local tier-2 in the image** (before the cap correction): `with_tools` 24/24, the new test included (8.75 s alone). Also green: `against_fab_tier2`, `verify_2k2_tier2`, `verify_47k_tier2`, `adversarial_1301_tools` and `adversarial_1302_tools`, plus `board_tier2`'s corpus, four-layer, against-fab, swapped-board and identifier tests.

- **CI at `be06b33`** (the adversarial fixes, `origin/main` `5f47a5d` merged in; run 43673945956): green.
  - Shards: 16m09s, 13m57s, 11m17s, 14m55s, **16m58s** (shard 4, the slowest), 16m29s, 14m59s, 14m30s; `gates` 9m55s. Every shard is under 20 minutes.
  - The aggregate: `tier2_tests_passed=97 floor=97`. `TIER2_MIN` is exact.
- **`origin/main` `d5bc841`** (AET-WO-1303's spike: `spike/1303/` and its FEEDBACK only, no code) is merged in after that run.
- **Mailbox** (`~/.cache/aet-bus/`): notes sent to 1303 (routed boards and bounds changed) and 4300c (D1's 4-layer I2C bounds, at most 118,375 fF under their 150,000; the renamed D1 stackups test). Received from 4210 (the cap measurement, B1), 1303 and 4300c (their D1 fixture change: whoever merges second keeps schema 3 and their constraints).

- **After the architect's review (accepted on substance; rulings on A4, A5, A2's shrunken courtyard and K3's count):** `origin/main` `50d7a6f` (4300c, 4300f, 4210, 1303) merged as `a97ead2`. The literals were re-measured (B9) and the CLI README line fixed.
  - Gates green: fmt, clippy, and `cargo test --workspace` (80 binaries).
  - CI run 43677178229 green. Shards: 15m46s, **18m19s** (shard 1, the slowest), 14m43s, 11m26s, 13m21s, 13m56s, 14m58s, 14m46s; `gates` 7m34s. Every shard is under 20 minutes.
  - `tier2_tests_passed=97 floor=97`: `TIER2_MIN` is exact.

**AET-WO-1307 READY FOR REVIEW: PR #126.**
