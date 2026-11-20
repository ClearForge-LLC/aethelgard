# AET-WO-1317 — predictions, committed before the first run

Written 2026-11-20, before any placement was run on this branch. Nothing below is tuned after the runs; FEEDBACK scores each line.

## Definitions, fixed now

- **A removal** is one non-MCU part, together with every part it owns (`owner.ic` names it). That is what U2's leaving did. A part that owns others cannot leave alone: its owned parts would be orphaned, and the placer refuses them ("decouples X, which is not placed").
- **Excluded:** the MCU only. Every part anchors to it, and the placer refuses a design with none. Connectors, test points, the reset switch and every passive are removed in turn.
- **Where the removal happens:** at the parts list `design` returns, before placement. No other part is renamed or renumbered. The spec-level removal (4202's) renumbered R6–R8. Calibration: the parts-level removal of U2 must reproduce 4202's forward refusal. If it does not, the sweep moves to spec level for the parts with a spec entry.
- **Outcome classes:**
  - `fit`;
  - `OutlineUnreachable` (it carries the reached size);
  - `CompactionUnbounded`;
  - any other refusal, its own class;
  - for `OutlineTooSmall`, `bound_holds` refused before placement.
- **Non-monotone, hard:** the full board fits and the reduced one does not (any refusal).
- **Non-monotone, soft:** the reduced outline (its reached size when refused) is wider or taller than the full one's by more than **1.000 mm** (20 snaps). Also reported at 0 mm.
- **Variants:**
  - the committed Sentinel-03 fixture;
  - `outline_tier1`'s four turns (`board.forward` "top", U5 x right/forward/left/aft), built with the test's own edit;
  - the size samples 1, 2, 6 and 8 (`common::rename::sized`, the test's own code).

## Predictions

- **P1 (calibration).** With U2 re-added (03d89554's fixture hunk reversed onto today's fixture), the fixture places at 96.50 × 69.35 mm and all four turns fit. Without U2, today's fixture, the forward turn is refused at 103.500 × 67.100 mm. Certain if nothing but U2 moved the placer since 4202.
- **P2 (first divergence).** The first step at which the two placements differ, with parts matched by a name-free identity, is in the placing order (k), before U2's own step.
  - Why: colour refinement hashes every pad on GND and the rails, so removing any part recolours nearly every other part. Parts equal in class, area and distance then reorder.
  - p = 0.6. The alternative is that the order is the same and the first difference is a part taking the spot U2 freed: p = 0.3.
- **P3 (what decides).**
  - The 3.5 mm is in x, and the core is already that wide before compaction (s). Compaction never widens x, and it does not pull an x outlier back in: p = 0.65.
  - Transplanting the with-U2 run's order onto the without-U2 parts restores the fit: p = 0.55.
  - So (k), through first-fit `spot` packing, decides. (m) does not: the targets of parts that share no net with U2 are unchanged.
- **P4 (sweep).**
  - Across the nine variants, at least 3 hard non-monotone cases: p = 0.7.
  - At 1 mm, at least 10 % of removals are soft non-monotone: p = 0.6.
  - The outline both grows and shrinks across removals.
  - With (k) knocked out (reference order under the bound), non-monotone cases persist: p = 0.7. Greedy first-fit packing is order-sensitive whichever order is used, and (k) only picks which order.
- **P5 (C1, the related row).** Not the same mechanism. Anchoring C1 to U10 grows U10's reserve, the longest side of its largest anchored part plus 1 mm, which U10 keeps from every part placed before it. That is a gap, not an order: p = 0.6.
- **P6 (a fix).**
  - No change to (k), (m) or (s) both makes the sweep monotone and leaves the five passing bounded cases byte-identical: the base, right, left, aft and size 1.
  - What can leave them identical by construction is a fallback that runs only when today's placement misses the bound. It removes hard cases, not soft growth inside the bound.
  - The work ends in (a), with such a fallback: p = 0.5.
- **P7 (cost).** The sweep is under 30 min wall on the host (release build), with peak RSS under 2 GiB.
