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
