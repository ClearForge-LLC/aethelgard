# AET-WO-1317 — The placer, monotone in parts: find why removing a part can stop a board fitting

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1317-placer-monotone`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P1 (eda, placement). Written 2026-11-20. **Cadence:** a measurement, then a build with one stop.

**Grounds** (`4202` F6 and its §2.4, ruled (i) 2026-11-19, #257; `1315` (c), ruled (a); roadmap §7.1):
- **Removing a part made a board stop fitting.**
  - When U2 left Sentinel-03, `4400d`'s forward-turned IMU variant went from fitting to `OutlineUnreachable`: best 103.500 × 67.100 mm, over the 100 mm bound by 3.500 mm.
  - The other three turns, and the declared board (95.80 × 67.60 mm), still fit.
  - The forward turn is pinned to that refusal in `outline_tier1::a_turned_imu_is_compacted_whole_with_its_keepouts_inside_the_bound`, as size samples 2, 6 and 8 are.
- **The placer** is `eda/src/pcb.rs` `place` (about L572). Under a declared bound it changes three things (`1313c`):
  - (k) the canonical rank orders the parts;
  - (m) each part targets the centroid of its anchor's pads on shared nets;
  - (s) compaction runs toward the MCU, bounded by `1313d`.
- **A related row:** anchoring a large passive moves its whole group. Sentinel-03's x headroom fell from 14.75 to 3.5 mm when C1 was anchored to U10 (`1315` (c)).
- **Sentinel-01 is ordered.** Its emitted board must not move without a ruling.

> **What this is:**
> - a measurement that names the mechanism by which removing a part grows the outline;
> - a sweep over committed bounded specifications, removing one part at a time, that counts how often it happens;
> - then, if a change makes the sweep monotone (or strictly better) without moving any committed board's bytes, that change, held by a test.
>
> It is NOT:
> - loosening a bound, a keep-out or a clearance to make something fit;
> - the router, the ladder, or any specification change.

## 1. Build

1. **Measure first,** in `spike/1317/`, with predictions committed before the runs.
   - **Reproduce:** the forward turn on Sentinel-03, with U2 and without. Diff the two placements step by step, and name the first step where they diverge, and which of (k), (m) and (s) decides it.
   - **The sweep:** for every committed specification that declares a bound, and for each of `outline_tier1`'s turned and size variants, remove each part that can be removed (not the MCU; say which others you exclude, and why), and place.
     - Record: whether it fits, the outline, and the outline with the part present.
     - A case is non-monotone when the full board fits and the reduced one does not, or when the outline grows by more than a stated tolerance.
     - Report the count, and wall time and peak.
   - **The related row:** re-measure C1's group move on Sentinel-03. Does the same mechanism explain it?
2. **Name the mechanism** in FEEDBACK, with the step diff. Then choose one:
   - **(a) A fix** inside the placer that makes the sweep monotone, or removes a stated share of the cases. It must move no committed board's bytes: `boards/`, `examples/`, and the bench package. Build it.
   - **(b) No such fix.** The row stays. Report the mechanism and what a fix would cost, and stop.
3. **If (a):**
   - The forward turn fits again, and its test returns to placed-outline and keep-out checks, like the other three turns. Size samples 2, 6 and 8 are re-judged, each explained.
   - The sweep, or the slice of it that runs in reasonable time, becomes a test that fails on `main`'s placer. Say which tier.
   - Determinism: the same input gives the same bytes on two runs, and no reference name or list position decides a placement (`1313c`).

## 2. Acceptance

- **The step diff, the mechanism and the sweep table** are in FEEDBACK, with predictions beside them.
- **If built:**
  - the forward turn fits;
  - the sweep test fails on `main` and passes on the branch;
  - **mutant:** revert the fix, and the test fails;
  - every committed board, the bench package and every passing `outline_tier1` case are byte-identical.
- **The CLAUDE.md gates pass, and CI is green.** `TIER2_MIN` rises by your own tier-2 count.

## 3. Invariants and fences

- **Invariants:** `INV-01` (the placement comes from the specification alone), `INV-06`.
- **Fence:**
  - `crates/aethelgard-eda/src/pcb.rs`, the placer only;
  - tests, and goldens for non-ordered fixtures moved by tooling, each move explained;
  - `spike/1317/`;
  - FEEDBACK.
  - **Not:** the router, the ladder, `stack.rs`, the IR, specifications, `boards/`, `examples/`, `fab/`, workflows, or `ci/` (except a weight row P1/P2 asks for).
- **Running beside you:**
  - `1316`: eda `schematic.rs`. You share no file with it.
  - `4240v` (#259), `2305` (#271), `2306` (#269). None touches the placer.

  Merge `main` before READY.
- **One container of yours at a time,** under `capped-run`, `--mem 8G` for `prove` and 16G for `verify`, at `nice -n 10`.
- One PR, left unmerged. Write `~/.cache/aet-1317/pr-title-body.md`.

## 5. Adversarial pass, full

- **A fix that fits by shrinking a gap,** a keep-out or a clearance, rather than by a better order.
- **A fix that is monotone on the sweep but not beyond it:** remove two parts, or add one.
- **A placement that a reference name or list position now decides,** against `1313c`.
- **Compaction's bound (`1313d`)** reached on a case that fitted before.

## 7. Flag-and-stop conditions

- **The only fix found moves a committed board's bytes,** Sentinel-01's above all. Stop with the diff; the architect rules, and Scotty is told if the ordered board would move.
- **The sweep does not finish** under `capped-run` in a few hours. Stop with the slice that ran, and its count.
