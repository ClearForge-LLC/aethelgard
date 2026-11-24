# AET-WO-1324 §1.1 — the measurement (STOP)

Base: `main` 72d85eef (4410 merged: the PC0 sense). Predictions committed first
(`PREDICTIONS.md`, 65e98697). Measured 2026-11-24 on Furnace, release builds
unless named. Kit: `inst.py` (a timing shim applied to a tree copy, never
committed), `probe1324.rs` (the N-pad lattice), `sweep.sh` (every committed
specification through the shimmed binary), `patches/proto-a.diff` (a
throwaway prototype of option (a), behind an environment variable, never
committed).

## (1) The meeting junction regions

### Why the router places them

The test's board (`supply_tier1::the_two_layer_power_fixture_on_the_pc0_sense_meets_overlapping_vias`,
kept from the run) has two VDD_3V3 vias, 0.75/0.30 mm, at
**(92.75, 86.625)** and **(93.25, 87.125)** mm. That is (4, 4) pitches,
0.707 mm apart.

- **Via A (93.25, 87.125) comes first.** An earlier connection laid it; its
  B.Cu run feeds the tree, (95.25, 86.875) → (93.25, 86.875).
- **Via B (92.75, 86.625) comes second.** The connection to C28 searches from
  the whole tree. It meets the tree's B.Cu node (93.25, 86.875), runs 4
  pitches on B.Cu, and rises through a new via B to reach C28 on F.Cu.
- **The router allows it.** Its only rule between one net's vias is KiCad's
  hole gap: `hole_reach` = 0.15 + 0.15 + 0.25 + 0.01 = 0.56 mm, against laid
  vias (`claim_via`) and within a path (`close_vias`). 0.707 mm passes.
- **It was a cost choice, not a forced one** (measured). With the prototype
  rule, the same connection goes on F.Cu, (92.75, 86.625) → (92.75, 87.125)
  → (93.25, 87.125), and ends at via A's own node. The board has 130 vias
  instead of 131, and only 2 VDD_3V3 segments change. So the F.Cu path was
  open, and the B.Cu drop was cheaper by the router's own costs.
- **Nothing in the router knows the junction region.** `4410`'s PC0 sense
  changed the routing order and costs near C28; any board can make the same
  choice.

### Where the regions meet

`extract`'s region for a via is `flow::Region::new(supply::PITCH_NM, reach)`:
the 62.5 µm cells that meet the open disc of radius `reach` about the via's
cell corner (`supply.rs` L600–607). Here:

```
reach = r_shell_max + BETA_NM + ⌈e·√2⌉ + 1
      = ((drill + hole_size_plus + 1)/2 + plating_min) + 10 000 + ⌈75 000·√2⌉ + 1
      = (215 000 + 18 000) + 10 000 + 106 067 + 1 = 349 068 nm      (P1.1: held)
```

The region has 120 cells, from −6 to +5 on each axis (±0.375 mm). Measured
exactly, by cell sets, over the router's 125 µm grid: two such regions meet at
**108 offsets**. In pitches, these are every |a|, |b| ≤ 5 except (4, 5), (5, 4)
and (5, 5). The farthest is (5, 3), 0.729 mm. They are apart from (6, 0),
0.75 mm.

**A disc rule would not do.** `2·reach` is 0.698 mm, yet the regions at
(4, 4), 0.707 mm, do meet: the cells reach past the disc. A rule "from the same
constant" has to use the region's cells. The prototype computes them with
`flow::Region::new` itself.

### A fabrication fault, or the model only?

**The model only, at drawn geometry** (measured):

- `aethelgard check drc` on the very board `extract` refuses passes: KiCad
  10.0.7, exit 0, `"verdict":"pass"`, tools image `1e0ab105`, under
  `capped-run`.
- The holes are 0.707 − 0.30 = 0.41 mm apart edge to edge, against the 0.25 mm
  gap. Same-net pads may merge.

**At the fabricator's tolerances,** P1.2's clause was **wrong** (missed). At
+0.13 mm on each hole and ±0.075 mm per axis toward each other, the web
between these two holes is 0.707 − 0.43 − 2 × 0.106 ≈ **0.065 mm**.

- **That is the hole gap's question, not this pair's.** The router's hole-gap
  rule already admits closer same-net pairs: (5, 0), 0.625 mm, whose worst
  web is 0.625 − 0.43 − 0.15 ≈ 0.045 mm.
- **The specification holds no fabricator figure for hole-to-hole.** I supply
  none.
- **A backlog row:** the hole gap is judged on drawn drills only (FEEDBACK).

**Why the model refuses it** (read):

- `flow.rs` L1–9 states Thomson's principle: a bound needs **one**
  divergence-free current density confined to the copper.
- Each via is `flow::via`, whose region carries a field written for **one**
  hole: a radial band and the plating (`junction_core`; flow.rs L1098–1113;
  `1303b` FEEDBACK §B1, "Via junctions: per grid face of a square around the
  hole, κ·F²"). Its contract is that the region's cells are absent from every
  sheet (flow.rs L1882).
- Two regions on one cell would place two such fields on the same copper. That
  counts the copper twice, and the energy of their sum is not the sum of their
  energies, so the result is no longer a certified upper bound.
- `supply.rs` L700–715 therefore refuses rather than price it.
- `supply.rs` L990–996 records the companion assumption: a hole inside a
  junction region is what DRC's hole-to-hole spacing is relied on to forbid.

### What else gets a region

`extract` prices more than routed vias (`supply.rs` L532–562):

- each **through-hole pad** of the net, with its own drill;
- each **thermal-array via** of the net that lands on the cell grid (most are
  off it and left out).

A rule must keep a routed via's region clear of those too (adversarial:
via-in-pad, or a via beside a through-hole pad). The prototype checks only
routed vias. On this board, `extract` passed its region loop with it on
(read: no `ViaRegionsOverlap`).

### What the router needs, and from where

Every input is an existing specification field, so this is **not** §7's stop
(no new IR field):

| Input | Where it is today | Where the router would get it |
|---|---|---|
| `drill` | the net's rule (`Stack::rule`) | already there |
| `hole_size_plus_um`, `hole_position_um` | `board.stackup.fab_tolerances` (`RingRule` reads them only with `via_ring_um`) | carried on `Stack` |
| `plating_min_um` | `board.stackup` (read only by `supply::supplies`) | carried on `Stack` |
| `BETA_NM`, `PITCH_NM` | `supply.rs` constants | read from `supply` (already `pub`) |
| the judged nets | each `power.drop_limits` domain, and `GND` | carried on `Stack`, like `bounded` |

This touches `stack.rs` (one field, set in `Stack::of`). It also needs one
reach function shared with `supply.rs`. Today the formula is inline at L604;
it would move into a `pub fn`, called by both, with `supply`'s pricing
unchanged. Both files are outside §3's fence for (a), hence **Q1**.

- **When an input is undeclared,** the gate never runs (`SupplyInputUndeclared`),
  so the rule is off: no region to keep apart. A board with no
  `power.drop_limits` gets no rule.

### Scope, measured

No committed specification declares `power.drop_limits` or `plating_min_um`.
Only `sentinel-02-power` and `sentinel-03` declare the hole tolerances (both
JLCPCB: +130 µm, ±75 µm; one 0.30 mm via drill on the supply nets). With the
prototype's region on those two committed specifications as they stand:

| Rule applied to | sentinel-03 (4 layers, 4300k's board) | sentinel-02-power (2 layers) |
|---|---|---|
| none (base) | `da09ab13…` | `02e2d864…` |
| VDD_3V3, GND, VBAT, VDD_5V (rails + ground) | `da09ab13…` **identical** | `02e2d864…` **identical** |
| every net | `da09ab13…` **identical** | `874381d2…` **moves** |

- **A rule keyed on the gate moves no committed board.** Its nets are the
  limited domains and `GND`, and it is on only when the gate's inputs are
  declared. That held under P1.3, and it holds even at the rails-wide scope.
- **The every-net scope moves `sentinel-02-power`,** so it is not offered.
- **`supply_tier1`'s pattern** ("Sentinel-03's board, emitted once: the gate's
  inputs never change it") emits from the ungated specification and extracts
  with gated copies. Under (a) keyed on the gate, an emitted board can depend
  on `drop_limits` and `plating_min_um`. For Sentinel-03 that changes nothing
  (identical even with the rule on every net), so those tests stand. The
  comment would need a clause; FEEDBACK will say so.

### With the rule: what `extract` says next (P1.4: held)

`sentinel-02-power` on PC0, prototype (a) on VDD_3V3 and GND, debug build:
`emit` exit 0 (8 ladder rungs, 108 offsets per net). Then `extract`:

- **exit 1, `"verdict":"fail"`, not an exit-2 refusal;**
- VDD_3V3: **16/16 pins bounded**, 0.578–1.625 mΩ;
- every return pin refused **`GroundReturnUnread`** (11 pins, as
  `supply_pin_unbounded` findings): "U1.16's return runs through a GND zone
  the emitted board leaves unfilled: the gate reads no plane for GND";
- `ran`: `extract.supply` judged 1.

So the next gate by name is **`GroundReturnUnread`**, the PF3 layout's refusal.
`4300k` draws GND pours in `emit`, which may change it after that merges.

### Options for (1)

- **(a) A routing rule from `extract`'s own region** (the architect's prior;
  **lean**). On a net the supply gate judges, a via may not stand where
  `flow::Region::new(supply::PITCH_NM, reach)` would meet the region of:
  - one of the net's laid vias (`via_open`, every search);
  - another of the path's own new vias (`close_vias`, then `unhop` or a ban,
    as today's hole gap);
  - a through-hole pad or on-grid thermal-array via of the net (fixed, at
    `Router::new`).

  The meeting set is computed once per drill pair, as cell-set intersections,
  so it is exact and needs no disc. A rip-up rebuilds the claims from laid
  vias (`remap`), so a via cannot move back.
  - The certified solve is unchanged.
  - Committed boards: none move (measured above).
  - `sentinel-02-power` on PC0 then fails by name, `GroundReturnUnread`.
- **(b) Merge meeting regions** into one, with its own certified solve: a
  field converging on two holes over a union of cells. That is a new
  derivation (`1303b`'s `junction_core` is written for one hole), and a
  change to the certified model. Not recommended.
- **(c) `extract` leaves one of two meeting vias out**, as it does an off-grid
  via: Rayleigh, holes kept clear. This is sound in principle.
  - But it edits `supply.rs`'s pricing (fenced to (b)).
  - It weakens the bound silently on any board the router draws so.
  - It needs a new argument that the left-out via's hole stays outside the
    kept via's region (`supply.rs` L990's assumption).

  Not recommended.

## (2) The scan inside the count

The board (`probe1324.rs`): N one-pad parts on a square lattice at 2 mm, one
net `/BIG` (not stitched), the reference 2-layer stack, `route_measured` at
`WORK`, one thread. **A reconstruction:** `1323`'s board was not kept. Its
1,600-pad figures were 348,036 units and 130.2 s; this lattice does 113,919
units in 70–72 s.

| N | counted work | scan pairs (pad × tree node) | seeds (sources priced) | index updates † | route s | scan | seeds | judges | other |
|---|---|---|---|---|---|---|---|---|---|
| 400 | 27,759 | 383,119,800 | 2,868,411 | 2,858,436 | 0.98–2.11 | 95.0 % | 3.5 % | 0.3 % | 1.4 % |
| 900 | 63,539 | 4,369,544,550 | 14,553,911 | 14,531,436 | 10.9–16.5 | 98.0 % | 1.6 % | 0.1 % | 0.3 % |
| 1,600 | 113,919 | 24,561,919,200 | 46,033,611 | 45,993,636 | 70.4–71.8 | **99.0 %** | 0.8 % | 0.06 % | 0.1 % |

† what an exact incremental index would do: each node added to the tree,
against every pad still remaining.

Shares are the second run's. The first run's agree within 2 points (400 pads: scan 93.2 % against 95.0 %). Wall seconds vary up to 2×
between runs on the shared box, so seconds go in FEEDBACK only.

- **P2.1 (≥ N²): held, and worse.** The scan is about **N³**: 383 M → 4.37 G
  → 24.6 G pairs, since each step compares every remaining pad with a tree that
  grows with every step.
- **P2.2 (scan ≥ 70 %): held**, at 95–99 %.
- **P2.3 (seeds 10–25 %): missed.** Seeds are 0.8–3.5 %.
- **P2.4 (judges < 5 %): held**, at 0.06–0.3 %. `check` is quadratic in items
  (pads + segments + vias); 0.039 s at 1,600 pads.
- **P2.5 (counted work ~linear): held.** 27.8 K → 63.5 K → 113.9 K (×4.1 for
  ×4 pads).

### Every committed specification under the shim (`sweep.sh`; 26 specifications emitted, 33 routings counting Sentinel-03's eight ladder rungs)

The five heaviest by today's count, and the two charged columns:

| Specification (rung) | work today | scan pairs | seeds | index updates | work + seeds + index | work + scan + seeds |
|---|---|---|---|---|---|---|
| ir `sentinel-02-power` (24) | 116.9 M | 189.7 M | 8.1 M | 6.6 M | **131.6 M** (37.6 % of `WORK`) | **314.7 M** (89.9 %) |
| cli `sentinel-03` (320/false) | 126.1 M | 0.48 M | 0.32 M | 0.17 M | 126.6 M | 126.9 M |
| cli `sentinel-03` (320/true) | 114.7 M | 0.54 M | 0.41 M | 0.16 M | 115.3 M | 115.7 M |
| cli `sentinel-03` (96/false) | 101.5 M | 0.76 M | 0.42 M | 0.19 M | 102.1 M | 102.6 M |
| cli `solve/stress` (24) | 36.7 M | 116.2 M | 3.9 M | 4.7 M | 45.3 M | 156.9 M |

The full table is in `~/.cache/aet-1324/sweep-base/table.tsv`; FEEDBACK will
carry it.

- **The headroom covers committed specifications only.** It does not cover:
  - the suite's own routings (`1323`: the heaviest is 170.1 M, d3 shifted
    under U1);
  - a test that pins a work figure.

  Both are measured at the build.
- **Charging the scan as it is would put `sentinel-02-power` at 90 % of
  `WORK`.**

### Options for (2)

- **(i) Index the tree, and charge what stays super-linear** (**lean**). Keep,
  for each remaining pad, its nearest tree node as `(distance, node)`. Update
  it with each node added (the tree only grows inside `route_net`). Remove a
  pad's entry with the pad, at the same index `k`.
  - **The same choice by construction.** The minimum of `(d, n)` over a
    growing set is associative. The pick stays `min_by_key((d, k))` over
    `remaining`, in the same order. So the choice is identical, ties included.
    The goldens prove it.
  - **The scan falls from N³ to N·T.** What stays super-linear is the index
    updates and each search's whole-tree seeds, each about N² (46 M each at
    1,600 pads). Both are charged to `WORK`, 1 unit per update and per seed,
    so the count rises with them.
  - **Charged totals:** 5.75 M, 29.1 M and 92.1 M at 400, 900 and 1,600 pads
    (measured, summed above).
  - **The scaling test states** those three charged counts, and two bounds:
    the 1,600-pad net finishes under `WORK`, and its charge is at most
    1.5 × (1600/400)² = 24 times the 400-pad charge (measured: 16.0). It runs
    counts only, no clock.
  - Committed headroom: `sentinel-02-power` 131.6 M (37.6 %). Bytes are
    unchanged unless a board crosses `WORK`; none is near.
- **(ii) Charge the scan to `WORK` as it is** (the work order's fallback).
  The 1,600-pad net is refused by name, since 24.6 G pairs is far over
  350 M. But `sentinel-02-power` sits at 314.7 M (90 %), and the suite's
  heavier routings are unmeasured. That is too tight; not recommended.
- **(iii) Index only, and charge nothing new.** Bytes unchanged. But the
  seeds and updates (about N²) stay outside the count. At the router's
  largest grid (250 mm, about 15,000 pads at 2 mm) they extrapolate to
  roughly 8 G uncounted operations, so the count would still not bound wall
  time. Not recommended.

## Questions for the architect

- **Q1 (1), fence.** Rule (a) as above, scoped to the nets the supply gate
  judges and on only when the gate's inputs are declared. Lean **yes**. It
  needs the fence widened by:
  - `crates/aethelgard-eda/src/stack.rs`: one `Stack` field, set in
    `Stack::of` from existing specification fields;
  - `supply.rs`: the inline reach at L604 moved into one `pub fn` that both
    call, with the pricing unchanged.

  Or name another home.
- **Q2 (1), through-hole pads and array vias.** Is the rule also held
  against the net's through-hole pads and on-grid array vias, which `extract`
  prices too? Lean **yes**: exactly the regions `extract` builds.
- **Q3 (2).** Option (i): index, and charge the index updates and the seeds.
  Lean **(i)**.
- **Q4 (2), the judges.** Leave the final `check`, `connectivity` and `pairs`
  uncounted, as `1323` D1 ruled: 0.06 % at 1,600 pads, quadratic in items. Or
  charge `check`'s pair loop (items·(items − 1)/2) before it runs. Lean
  **charge it**: it closes §5's "a judge run outside `route_net`", costs no
  byte, and is cheap to state.
- **Q5, the test's comment.** `supply_tier1`'s "the gate's inputs never change
  it" needs a clause under (a), since an emitted board can depend on
  `drop_limits`. Sentinel-03's bytes are unchanged (measured), so no test
  moves. Lean: reword the comment in the build.
