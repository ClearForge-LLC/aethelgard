# AET-WO-1324 §1.1 — predictions, committed before the measurement

Base: `main` 72d85eef (4410 merged). Written 2026-11-24 before any timing run.

## (1) The meeting junction regions

**Already read before these predictions** (so not predictions): the emitted
`sentinel-02-power` board (PC0 sense, the supply_tier1 test's spec) has two
VDD_3V3 vias, 0.75/0.30 mm, at (92.75, 86.625) and (93.25, 87.125) mm,
0.707 mm apart on the diagonal. They are the two ends of one path's 1 mm hop
on B.Cu. The router keeps a net's vias apart only by KiCad's hole gap
(`hole_reach`: 0.15 + 0.15 + 0.25 + 0.01 = 0.56 mm), within a path
(`close_vias`) and against laid vias (`claim_via`).

Predictions:

- P1.1 The junction region's reach on this board's gate model is
  r_shell_max + beta + e·√2 + 1 nm = 233 000 + 10 000 + 106 067 + 1
  ≈ 349 068 nm, so a region's cells reach ±0.375 mm along an axis. Two
  regions on the router's grid meet at axis offsets ≤ 0.625 mm and at the
  diagonal (0.5, 0.5) mm; they are apart from about 0.75 mm.
- P1.2 Two same-net vias 0.707 mm apart, 0.75 mm pads, 0.30 mm drills are
  **not a fabrication fault**: their holes keep the 0.25 mm hole gap (0.41 mm
  web at the drawn drill, more than 0.25 mm even at +0.13 mm and ±0.075 mm
  position), and same-net pads may merge. KiCad's DRC on the board passes.
  It is only the supply model that cannot represent them: each junction
  region is an element whose cells no sheet holds (`supply.rs` L700–715), a
  flow built for one hole; two flows on one copper cell would count that
  copper twice, which a Thomson (upper) bound may not.
- P1.3 No committed specification declares `power.drop_limits`, so a rule
  scoped to the nets the supply gate judges moves no committed board.
- P1.4 With such a rule, `sentinel-02-power` on PC0 either extracts, or
  `extract` refuses it by the next name, which I predict is
  `GroundReturnUnread` (the refusal the PF3 layout had).

## (2) The scan inside the count

The board: one net (not stitched, not ground) of N pads on a square lattice,
N = 400, 900, 1 600, release build, one thread.

- P2.1 Wall time grows about as N² or faster; the 1323 figures were 1.9 s,
  20.9 s and 130.2 s (N^2.7 between 900 and 1 600).
- P2.2 The nearest-pad scan in `route_net` (each remaining pad against every
  tree node, every step) is the largest share at 1 600 pads: **≥ 70 %**.
- P2.3 The whole-tree seeds (every tree node pushed into each search's heap,
  and the `sources` vector built from the tree) are the second share:
  10–25 %.
- P2.4 The final judges (`check`, `connectivity`, `pairs`) are under 5 %:
  `check` is quadratic in items but with a small constant.
- P2.5 The counted work (`WORK` units) grows about linearly in N
  (1323: 348 036 at 1 600).
- P2.6 Indexing the tree (an exact nearest query with the same (distance,
  node) tie-break, and then (distance, pad order) across pads) leaves every
  committed board byte-identical, since it returns the same choice.
