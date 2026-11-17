# AET-WO-1313b — FEEDBACK

Labels: **[measured]** by a run recorded in `spike/1313b/`; **[read at source]** from a file named beside it; **[inferred]** from arithmetic or reasoning.

## 0. Status: STOPPED at §2 with a recommendation

- **Recommended for the build: (k)+(m)+(s)**, only under a declared bound:
  - a canonical, name-free order;
  - each part's target the centroid of its anchor's shared-net pads;
  - anchored parts in the emitter's draw sequence;
  - then compaction toward the MCU with a sidestep.
- **On the fixture at four layers [measured]:** **85.25 × 80.95 mm**, every gate passing, `extract`'s least slack 9.2 pF, `emit` 3.1 s.
  - Over 20 order-only renamings: 20/20 fit, 20/20 every gate, 20/20 the identical placement.
  - Over 20 of 1307's prefix renamings (parts 31 % larger): 20/20 fit (93.70 × 87.95), 20/20 every gate.
  - Shuffling the parts list (10 seeds) and reversing the last tie-break both give the identical placement.
  - With no bound declared, all 24 corpus specifications are byte-identical to `main`.
- **Headroom:** with the witness port, USB FS and E0 as blocks (495 mm² of keep-out), the board is **89.20 × 86.25 mm** and still fits [measured, outline only].
- **The test-pad rule is not recommended now (§2.4).** It helps Sentinel-01 a little, breaks the fit on Sentinel-03 under (s), and moves Sentinel-01's bytes.
- **Two findings about `main` (§2.6)**, one with a byte-neutral fix.

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

## 2. The spike, measured

Host: release CLI from `f845b6f` (`main`) with `spike/1313b/placer.patch` applied in a scratch tree; KiCad steps in the pinned image through `~/bin/capped-run` (caps read back each run, swap 0). The methods run only when `AET_1313B` holds their letters and a bound is declared; `t` (the pad rule) and `q` (§2.6) run without one. `gnd-exemption-3402.patch` (3402 §1.1's exemption) is applied in the scratch tree for the two-`GND` probe only. No crate is changed on this branch.

### 2.1 The methods, as built

The base, shared by every method: **anchored parts in the emitter's draw sequence.**
- Each `Anchor` gets `seq`, its place among the parts anchored to its IC, counted when it is drawn (13 lines in `schematic.rs`, 2 each in `schematic_power.rs` and `schematic_support.rs`).
- Without it, the first canonical order put the buck's inductor and bulk capacitor before its input capacitor, CBOOT and VCC bypass. That failed eight loop rules on every renaming (`C11 is 3856 um from U10 … over the 2000 um bound`) [measured, superseded].
- The draw order is the emitter's engineering priority. It was carried only by list position, which a parts shuffle changes.

The methods:
- **(k) canonical order** (`canon_ranks`, 133 lines in the prototype):
  - **Colours.** Each part's colour is its footprint, refined three times over the net graph (each net coloured by the (part colour, pad number) pairs on it). Ownership is mixed in too: the owner's colour, whether it decouples, `seq` and the anchor's pad pairs. Unconnected pads, whose nets carry a reference, are skipped.
  - **The rank, in order:** the MCU; then ICs owning anchored parts; then larger courtyard; then net-graph distance to the MCU over label nets; then colour. The reference only breaks what still ties.
  - **Uses of the rank:** the anchor's tie-break, connectors to edges, and bench parts.
  - **The target:** the anchor's lowest-numbered pad on any shared net (name-free).
- **(m) centroid target:** instead, the centroid of the anchor's pads on every shared net. On today's placer that is where a name decides: the first net by name was IMU_CS's pad, across the MCU (`1313` D2).
- **(s) sidestep compaction** (`compact_sidestep`, 91 lines, plus `Rule` and `groups`, about 35):
  - Each non-MCU IC's group (the IC and every non-IC part anchored to it), in rank order, steps toward the MCU's courtyard centre in y.
  - When blocked, it may jump sideways (left first) up to 10 mm, together with the y step.
  - Otherwise it steps in x. No step overshoots the centre.
  - The potential (|dy|, |dx|) falls lexicographically with every move, so it ends.
  - Every member keeps the gap the placer gave it, stays out of the approach strips, and stays inside the core's x-range as it stood before.
- **(p) skyline repacking** (`skyline` and `pack`, about 160):
  - Each group is kept rigid and repacked by its keep-out box into a strip as wide as the bound less the side connector bands, under and beside the MCU's group.
  - Groups go tallest first; each takes the lowest level, then the column nearest the MCU pads it shares nets with.
  - Variant `w` searches the strip's width.

### 2.2 The methods on the fixture (Sentinel-03, 100 × 100 mm, four layers) [measured]

Files: `results-independence.txt`, `results-perm.txt`, `results-prefix.txt`, `results-emit-time.txt`. A method is order-independent when shuffling the parts (10 seeds), reversing the last tie-break, and the 20 order-only renamings all give its own unrenamed placement (`sig.py`: the outline plus the sorted footprint, origin and rotation).

| Method | Outline (mm) | Order-independent | Order-only renamings: fit / every gate | Prefix renamings (parts +31 %): fit / every gate | Least `extract` slack | `emit` (median) |
|---|---|---|---|---|---|---|
| `main` (r)+(c) | 85.25 × 99.05 | **no**: shuffled 0/10, renamings 0/20 identical | 20 / 20 | **1 / 1** (19 `OutlineUnreachable`) | 8.8 pF | 1.9 s |
| (k) | 89.20 × 95.55 | yes | 20 / **0** | 20 / 20 (99.45 × 99.40) | −3.9 pF | 2.7 s |
| (k)+(m) | 85.25 × 99.45 | yes | 20 / 20 | **0 / 0** (93.70 × 105.45) | 9.2 pF | 3.4 s |
| (k)+(s) | 89.20 × 77.05 | yes | 20 / **0** | 20 / 0 | −2.6 pF | 2.6 s |
| **(k)+(m)+(s)** | **85.25 × 80.95** | **yes** | **20 / 20** | **20 / 20** (93.70 × 87.95) | **9.2 pF** (9.5 prefixed) | 3.1 s |
| (k)+(p) | 93.95 × 87.20 | yes | 20 / **0** | 20 / 20 | −8.0 pF | 1.8 s |
| (k)+(m)+(p) | 88.85 × 83.90 | yes | 20 / **0** | 20 / 0 | −19.6 pF | 2.9 s |

- **Every failure in the table is `extract`,** on the I2C and SPI lines; or `OutlineUnreachable` (refused by name, nothing written).
- **(p)'s outcome swings with details.** Centring its strip on the MCU's courtyard rather than its group's box moved (k)+(m)+(p) from 98.60 × 77.95 with every gate passing to 88.85 × 83.90 failing `extract` by 19.6 pF.
  - Its width search (`w`) reached 77.05 × 89.20 and failed by 24.6 pF.
  - Repacking groups by their boxes pulls them off the MCU pins they serve: the I2C lines run 136–175 mm with up to 21 vias. That is 1313's (b) again: the tightest packer fails `extract` most.
  - I report (p) at its spread and do not pick the draw that passed.
- **(s) needed (m).** With the lowest-pad target, the compacted board fails `extract` by 2.6 pF on every renaming.

### 2.3 Headroom (§1.5) [measured, outline only: blocks are placed, never drawn or routed]

| Method | Without the blocks | With the six blocks (495 mm² of keep-out) |
|---|---|---|
| `main` | 85.25 × 99.05 | 91.40 × **103.05**: `OutlineUnreachable` |
| (k)+(m) | 85.25 × 99.45 | 89.20 × **104.75**: `OutlineUnreachable` |
| **(k)+(m)+(s)** | 85.25 × 80.95 | **89.20 × 86.25: fits** (10.8 mm of width and 13.75 mm of height to spare) |
| (k)+(p) | 93.95 × 87.20 | 98.25 × 98.25: fits |
| (k)+(m)+(p) | 88.85 × 83.90 | 97.90 × 95.30: fits |

The blocks:
- **In the core:** two TSSOP-24 (8.04 × 8.30 each), the 2 × 10 header (6.09 × 26.40), USBLC6-2SC6 (4.10 × 3.54), and E0's capacitor (9.0 × 9.5 [inferred]).
- **In a side connector row:** the USB-C receptacle (8.94 along the edge × 10.64 deep).

`1313` §2.1's estimate (420–455 mm²) counted one TSSOP-24 for two.

### 2.4 Test pads beside their nets (§1.4) [measured: `results-testpads.txt`]

**The rule `t`:**
- A signal test pad goes at the free spot nearest any placed pad of its net, minimising its distance to that pad.
- A `GND` test pad goes beside the signal test pad that is farthest from every `GND` test pad so far. The first goes beside the signal pad nearest all the others.

**The measurements:**

| Board | Signal pads: median / max distance to their net's nearest other pad | `GND` pad(s) |
|---|---|---|
| Sentinel-01, today | 6.4 / 13.3 mm | TP1: 20.5–31.0 mm from TP11–TP14 |
| Sentinel-01, `t` | **5.0 / 8.8 mm** | TP1 3.8 mm from TP13; 20.5–33.3 mm from the others |
| Sentinel-01, `t`, a second `GND` pad (TP15) | 5.0 / 8.8 mm | TP15 3.8 mm from a signal pad, but 21.9–45.2 mm from TP11–TP14 |
| Sentinel-03, (k)+(m)+(s) | 10.1 / 16.3 mm | TP29 6.3 mm from TP13 |
| Sentinel-03, (k)+(m)+(s) with `t` | 8.5 / 24.8 mm | **the outline goes to 85.25 × 104.70: `OutlineUnreachable`** |

- **Sentinel-01 passes every gate at two layers with the rule,** with one `GND` pad or two (`extract`'s least slack 6.7 and 6.8 pF).
- **The coverage rule does not put the second pad beside TP11–TP14:** it serves the signal pad farthest from TP1. I did not tune it to that board.
- **Under (s) the rule breaks the fit.** Pads seated by their nets sit inside other groups' paths, and compaction then stalls.
- **Its cost:** with the methods unset, `t` moves Sentinel-01 and its bench fixture (2 of 24 corpus specifications move; `results-nobound.txt`). So it would need a re-dispatch of 3401's package, or gating behind a bound.

### 2.5 No-bound bytes and the parts-order test [measured]

- **The corpus:** 24 specifications without a bound, `emit` and `generate`, `main`'s binary against the prototype (`corpus.sh`).
  - With `AET_1313B` unset: 24/24 byte-identical.
  - With `kms`: 24/24, because the methods run only under a bound.
  - With `kmst`: 22/24. Sentinel-01 as ordered and its bench fixture move, by the pad rule.
- **`the_board_does_not_depend_on_the_parts_order`** passes on the prototype, with and without `kms`. It declares no bound, so it never reaches the methods; the bounded proof is §2.2's shuffles.

### 2.6 Findings about `main`

- **F1. `main`'s placer depends on the order of the parts list,** even with no bound.
  - Shuffling the parts inside the emitter (10 seeds) gives the same placement only 1–3 times in 10 on Sentinel-01, Sentinel-01 as ordered, the Sentinel-02-own fixture, `four-layer` and `four-layer-power` (`results-main-order.txt`).
  - Today's unit test passes only because its synthetic design has no anchored parts.
  - **The cause:** `place` takes an IC's anchored parts in list order.
  - **The fix, measured:** order them by the draw sequence the emitter attaches (`seq`, letter `q`). All five boards then give the same placement 10/10, and all 24 corpus specifications stay byte-identical. The parts list's order is fixed today, so no board is wrong. But any future change that reorders parts would move bytes for no reason.
- **F2. On `main`, under a bound, the outline depends on names.** Order-only renamings give 20 distinct placements (84.75–87.70 × 97.40–99.55), and 1307's prefix renamings fit 1 in 20 (confirming `1313` §5.3).

### 2.7 Predictions scored

| # | Predicted | Measured | Score |
|---|---|---|---|
| P1 | (k): 84–88 × 92–100 | 89.20 × 95.55, `extract` fails | outline about right; gates wrong |
| P2 | (s): 84–88 × 78–88 | (k)+(m)+(s) 85.25 × 80.95; (s) on (k) alone fails `extract` | right with (m) |
| P3 | (p): 84–90 × 72–85, longer routes | 88.85–93.95 × 83.90–87.20; `extract` fails by 8–20 pF | routes right; outline worse |
| P4 | renamings: identical, 20/20 | 20/20 for every method; ownership and `seq` had to enter the key | right, after two fixes |
| P5 | prefix: (s), (p) ≥ 15/20; (k) 5–12 | (k)+(m)+(s) 20/20; (k)+(p) 20/20; (k) 20/20; (k)+(m) 0/20 | right for (s) and (p); (k) better |
| P6 | (k), (s) pass every gate; (p) maybe not `extract` | (k)+(m)+(s) passes; (k) and (k)+(s) fail `extract`; (p) fails | half right |
| P7 | the parts-order test passes | passes; shuffles 10/10 for every method | right |
| P8 | `emit` under 5 s | 1.8–3.4 s | right |
| P9 | no-bound bytes unchanged | 24/24 | right |
| P10 | pad median 2–5 mm | 5.0 mm (Sentinel-01), 8.5 (Sentinel-03) | **wrong**, short of it |
| P11 | Sentinel-01 with a second `GND` pad passes | passes every gate | right |
| P12 | headroom fits, about 88 × 85–92 | 89.20 × 86.25 | right |

## 3. Recommendation for the build

1. **Build (k)+(m)+(s) under a declared bound** as `board.max_outline_um`'s placement. It replaces (r)'s owners-first rule, which the canonical rank includes. With no bound declared, today's placer stays, and every byte with it.
   - **Cost in code [inferred from the prototype's line counts]:** about 350 lines in `pcb.rs`:
     - the canonical rank, about 135;
     - the centroid target, about 25;
     - the sidestep compaction with its gap rule and groups, about 125;
     - anchored ordering and the connector rank, about 20;
     - the rest wiring.
   - Plus `Anchor::seq` (about 17 lines across `schematic.rs`, `schematic_power.rs` and `schematic_support.rs`), and tests:
     - the shuffle and reversed-tie checks as unit tests under a bound;
     - the 20 order-only renamings as an identity test;
     - the tier-2 gate.
   - **Moved bytes:** the fixture's board only.
2. **Fix F1 in the same work order or before it:** the anchored parts' order by `seq` on the unbounded path. It is byte-neutral on the whole corpus [measured]. Extend `the_board_does_not_depend_on_the_parts_order` to a design with anchored parts, so the test means what it says.
3. **Do not build the test-pad rule now.**
   - It moves Sentinel-01's bytes for 1.4 mm of median.
   - It breaks the fit under (s).
   - Its `GND` coverage does not give 3402's pairing.
   - A pad rule that cooperates with compaction (placing pads after compaction, by their nets) is a next spike if the bench needs it.
4. **For Scotty's quote:**
   - **Size:** ask for 100 × 100 mm, four layers, top-side assembly.
   - **Expected outline** under the recommendation: about **86 × 81 mm** now (85.25 × 80.95). With the witness port, USB FS and E0 placed by the same method, about **90 × 87 mm** (89.20 × 86.25).
