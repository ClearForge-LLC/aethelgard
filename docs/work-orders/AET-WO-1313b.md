# AET-WO-1313b — Spike: a compaction that does not depend on part order, and the test pads placed beside the nets they serve

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1313b-compaction`. A spike: its prototypes stay under `spike/1313b/` as patches and scripts, and FEEDBACK is the deliverable.
**Author:** Claude (architect) · **Builder:** strong tier (the `1313` builder, reused).
**Phase:** P1 board capability, for Sentinel-03 (roadmap §5). Written 2026-11-17, after `AET-WO-1313`'s second stop (PR #197, ruling comment 6944446645). **Cadence: spike only, ending in a stop.** Scotty's fabrication quote waits on this order.

**Grounds:**
- **`1313` FEEDBACK §5.3, measured.** The spike's 85.25 × 80.80 mm came from a compaction whose target was whatever part came first in the emitter's item order. That was C1 on the fixture, not the MCU's centre. The board would therefore depend on the order of its parts, which `pcb.rs`'s `the_board_does_not_depend_on_the_parts_order` forbids.
  - Compaction toward the MCU's centre moves nothing. `1313` ships (r)+(c) at **85.25 × 99.05 mm**, with 0.95 mm of height to spare.
  - Under 1307's prefix renamer, which grows the parts' keep-out sum by 31 %, 1 of 20 renamings fits.
  - The witness port, USB FS and E0's capacitor, estimated by area in `1313` §2, will not fit in that last millimetre.
- **`3402` F1 (PR #203), measured.** A `GND` test pad lands in the same place, 25.8–31.5 mm from Sentinel-01's I2C pads, wherever it sits in the specification's list and whatever it is called. The placer puts test pads by its own rule.
- **Scotty, 2026-11-17:** the quote comes after "the compaction/density system".

> **What this is:** the measurement a build needs. Is there a deterministic, order-independent placement that recovers about 80 mm of height on Sentinel-03 inside 100 × 100 mm, holds up under renaming, and passes every gate? If so, which method, and what does it cost? Separately: can a test pad be placed beside the nets it serves, by a rule rather than by list order?
>
> It is NOT:
> - a merged placer change (prototypes stay under `spike/1313b/`);
> - a router change;
> - bottom-side parts (ruled out by `1313` D1).

## 1. Spike

1. **Predictions first,** committed to FEEDBACK before any run.
2. **At least three order-independent methods,** each prototyped as a patch kept under `spike/1313b/` against `aethelgard-eda`'s placer. A method is order-independent when its result is the same under every renaming and reordering of parts with the same footprints and nets. Candidates, not a limit:
   - **(s)** compaction toward a geometric target defined by the board, such as the MCU's centre or the core box, with a sidestep: when blocked, move to the nearest column that frees a step down;
   - **(k)** a canonical order computed from footprints and the net graph alone (area, then net-graph distance to the MCU, then a hash of the pad-net signature), then today's greedy rings;
   - **(p)** shelf or skyline packing of the decoupled groups, in that canonical order.
3. **Measure each on Sentinel-03's fixture at bound 100 × 100, four layers:**
   - the outline;
   - fits and every gate passing, over 20 order-only renamings and over 20 under 1307's prefix renamer, with `1313`'s scripts;
   - `the_board_does_not_depend_on_the_parts_order` on the prototype;
   - `emit`'s wall time;
   - every existing board's bytes with no bound declared, which must not move.

   Label every figure measured, read at source or inferred.
4. **Test pads beside their nets.** Prototype one rule, for example "each test pad goes at the free spot nearest to the copper of the net it carries", and measure:
   - each pad's distance to its net's nearest other pad, on Sentinel-01 and Sentinel-03;
   - whether Sentinel-01 then routes and passes every gate at two layers with a second `GND` pad near TP11–TP14.
5. **Headroom for what Sentinel-03 still lacks.** With the best method, add `1313` §2's area estimates for the witness port, USB FS and E0's capacitor as keep-out blocks, and say whether the board still fits 100 × 100.

## 2. Stop

**STOP** with FEEDBACK holding:
- the predictions;
- the methods table;
- the test-pad measurements;
- the headroom figure;
- a recommendation for the build, with its cost in code.

Push an `AET-WO-1313b STOPPED:` commit and open a PR. It is merged as the record: FEEDBACK plus `spike/1313b/`, with no crate changed.

## 3. Invariants and fences

- **Invariants:** `INV-01`, `INV-03`, `INV-06`; `RUL-03`, `RUL-16`.
- **Fence:**
  - `spike/1313b/`;
  - FEEDBACK.

  Read anything. Change no crate on the branch: prototypes are patches applied in scratch trees.
- **Running beside you:**
  - `3402` (Sentinel-01's package);
  - `1604` (the CLI);
  - `4400c2` (the twin);
  - `2302b` (evolve, chaos).

  You run KiCad: one container of yours at a time, under `capped-run`.
- One PR. Write `~/.cache/aet-1313b/pr-title-body.md`.

## 7. Flag-and-stop conditions

- **§2 is the stop.**
- **No method fits 100 × 100 with every gate passing.** Stop with the closest one and its numbers.
