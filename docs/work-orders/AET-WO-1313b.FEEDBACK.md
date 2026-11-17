# AET-WO-1313b — FEEDBACK

Labels: **[measured]** by a run recorded in `spike/1313b/`; **[read at source]** from a file named beside it; **[inferred]** from arithmetic or reasoning.

## 1. Predictions (committed before any run)

### 1.1 Read at source before predicting

- **What makes today's placer depend on names or list order** (`pcb.rs` at `f845b6f`):
  1. `placing_order`: ICs, passives and bench parts each in reference order. (r) puts IC owners first, but still in reference order.
  2. The anchor's tie-break (the lower reference).
  3. The target: the anchor's lowest-numbered pad on the *first shared net by name*, label nets first.
  4. Anchored parts in drawing (list) order.
  5. Connectors assigned to an edge in reference order.
  6. Test points: bench parts in reference order, beside the IC sharing most nets. A `GND` pad shares none, so it goes beside the tie-break IC's courtyard centre: 3402 F1's fixed spot.
- **So an order-independent method needs an order-independent base,** whatever it does afterwards. The prototypes share one: **(k)**. (s) and (p) are built on it.
- **`1313` §2.1's headroom estimate undercounted** [read at source: `2b43672:spike/3400/sources/courtyards-w.txt`]. One TSSOP-24 is 8.04 × 8.30 mm = 66.7 mm², so the witness port's two take 133.4 mm², not 66.7. The corrected blocks:
  - two TSSOP-24 (8.04 × 8.30 each);
  - a 2 × 10 header (6.09 × 26.40);
  - USBLC6-2SC6 (SOT-23-6, 4.10 × 3.54);
  - the USB-C receptacle (10.64 × 8.94; a connector, on an edge);
  - E0's capacitor (CP_Elec_8x10, not vendored, about 9 × 9.5 [inferred]).
  - Together about 500 mm² of courtyard.

### 1.2 The methods, as they will be prototyped

- **(k) canonical:** every part ordered by a key computed from footprints and the net graph alone:
  - the MCU first;
  - then ICs owning anchored parts;
  - then larger keep-out box first;
  - then the net-graph distance to the MCU;
  - then a hash of the part's pad signature: its footprint, and for each netted pad its pad number and the size of its net. Net names and references never enter.
  - **Anchor ties** go by the same key.
  - **The target** is the centroid of the anchor's pads on every shared net, not the first by name.
  - **Anchored parts and connectors** go in canonical order.
  - Then today's greedy rings.
- **(s) sidestep compaction on (k):** each IC group, in canonical order, slides a step at a time toward the MCU's centre (x, then y). When blocked, it moves sideways to the nearest column (either direction, up to 10 mm) from which it can step toward the MCU in y. Every gap holds, until no group moves.
- **(p) shelves on (k):** each IC group is kept rigid as (k) laid it out, and repacked by its keep-out box:
  - the MCU's group stays put;
  - the other groups go in shelves above and below it, tallest first, in canonical order;
  - each shelf is filled left to right up to the core width the bound allows;
  - the gaps between groups are `IC_GAP_NM`.

### 1.3 Predictions [inferred]

| # | Claim | Predicted |
|---|---|---|
| P1 | (k) on the fixture | 84–88 × 92–100 mm; height about today's (85.25 × 99.05), since (k) only removes name dependence |
| P2 | (s) on the fixture | 84–88 × **78–88** mm; the sidestep recovers what the C1 target did by accident |
| P3 | (p) on the fixture | 84–90 × **72–85** mm; the tightest, but with longer routes |
| P4 | Order-only renamings (20) | (k), (s), (p): 20/20 identical outlines. Byte-identical boards after renaming the names back [inferred: the methods never read a name] |
| P5 | 1307's prefix renamings (20; parts grow 31 %) | (s) and (p) fit at least 15/20; (k) 5–12/20 |
| P6 | Every gate on the fixture | (k) and (s) pass. (p) passes all but perhaps `extract` (longer SPI routes; margins today 12–16 pF of 30) |
| P7 | `the_board_does_not_depend_on_the_parts_order`, with a bound | (k), (s), (p) pass, shuffled parts giving identical placements |
| P8 | `emit` on the fixture | under 5 s for each |
| P9 | No bound declared | every existing board byte-identical: each method runs only under a bound |
| P10 | The test-pad rule (nearest free spot to the copper of its net; `GND` pads by coverage of the signal pads) | median pad-to-net distance on Sentinel-01 from about 10–20 mm today to **2–5 mm**; on Sentinel-03 likewise |
| P11 | Sentinel-01 with a second `GND` pad by the rule, at two layers | routes, and every gate passes |
| P12 | Headroom: the ~500 mm² of blocks added under the best method | still fits 100 × 100, at about 88 × 85–92 mm |
