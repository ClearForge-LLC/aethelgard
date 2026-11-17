# AET-WO-1313 — FEEDBACK

## 0. Status: READY, as ruled on the second stop (PR #197, comment 6944446645; §6)

- **The method is (r)+(c).** Under a declared `board.max_outline_um`, ICs owning anchored parts are placed first, and the connectors go on the left and right edges, cable side out, through one edge-plan function. (d), every cable on the bottom edge, is one line away.
  - (a), compaction, was dropped in a removal-only commit (ruled (i)).
  - A bound that cannot hold the parts is `OutlineTooSmall`, before placement. One the placer does not reach is `OutlineUnreachable`, and nothing is written.
  - With no bound declared, every existing board is byte for byte today's.
- **The fixture** (`sentinel-03.json`: no `SPI_*`/`IMU_*` test pads, IMU_CS on PB12, bound 100 000 × 100 000 µm):
  - **85.25 × 99.05 mm**;
  - every gate passes at four layers, in tier 1 and in the tier-2 test in the image;
  - `extract` passes with room: the SPI lines at 12.3–16.5 pF of 30.
- **Specification 15; `TIER2_MIN` 178,** counted in the image.
- **3401's fabrication package** is the artifact of the dispatched run 44058299670 (on `556b303`), committed unchanged. Nothing after `556b303` moves Sentinel-01's bytes (measured, §6).
- **For the quote:** ask for 100 × 100 mm, four layers, top-side assembly. Expect about 85 × 99 mm. The spike's 86 × 81 was the C1 target's (§5.3). In height there is no spare room for the witness port, USB FS and E0 [inferred].
- **(iii)**, a compaction that recovers the 80 mm heights, is `AET-WO-1313b`, the architect's spike.

### The §1.5 status, as written at `ede77a8`


- **The 100 × 100 mm target is reachable on top-side parts at four layers.** Recommended method, for the build: **(r)+(a)+(c)**: the bucks first, compaction, and every connector on the left and right edges. Its outline on the fixture is **85.25 × 80.80 mm**. Over 20 order-only renamings it is 84.75–87.70 × 80.05–82.80 mm, and all 20 fit. Under 1307's prefix renamer, which also grows the parts (the keep-out sum rises 31 %), 12 of 20 fit. Today's placer gives 100.05 × 115.75 [measured, §2.3, §2.4].
- **But on the fixture as committed, no method that fits passes `extract` on every renaming.**
  - Under the recommended method, the fixture's own names fail `extract`: IMU_SCK at 31.1 pF against 30 pF.
  - Across renamings it passes every gate on 15/20 order-only and 13/20 with 1307's prefix.
  - The test pads on the six 30 pF SPI lines, plus the router's via count, use up the copper budget.
  - Without those six test pads (a probe), the method passes all eight gates on 20/20 order-only and 19/20 with the prefix; it fits 20/20 and 20/20.
  - So §2.3's acceptance, the fixture passing every gate inside 100 × 100, needs a ruling on the fixture (D3).
- **K3 (1307's 16/18/15) no longer reproduces on `main`:** 20/20 on all three boards under every method [measured, §2.4].
- **Deviations:**
  - the fixture pins `IMU_CS` on PB12 (D2);
  - it has one `GND` test point, not two, because 1312 refuses a second (D5).
- Decisions D1–D5 and questions Q1–Q4 are in §3. §2 is not started.


Labels: **[measured]** by a run recorded in `spike/1313/`; **[read at source]** from a file or document named beside it; **[inferred]** from arithmetic or reasoning, not measured.

## 1. Predictions (committed before any measured run)

### 1.1 Read at source before predicting

- The placer (`pcb.rs` `place`, `place_connectors`, `layout_of`, at `2b43672`): the MCU at `ORIGIN`; ICs, then passives, in reference order, each at the first free `STEP` ring around its anchor's pad; gaps `GAP_NM` 0.5 mm, `IC_GAP_NM` 1 mm, a reserve of (an anchored part's longest side + 1 mm) for an IC that owns anchored parts; the declared pairs' approach strips kept clear; connectors last, in one row at the bottom, centred under the core, `IC_GAP_NM` apart; the outline is the keep-out boxes' union grown by `MARGIN_NM` 2 mm, snapped to 50 µm. Nothing scores the box.
- V5's connector row (`spike/3400/variants/v5-4l.json`, vendored courtyards): J1 9.46, J2 9.80, J3 9.46, J4 (XT30) 14.30, J5–J7 3 × 11.96, J8 9.46 mm wide; 88.36 mm of courtyard plus 7 × 1 mm gaps = 95.36 mm, plus 2 × 2 mm margin = 99.36 mm before silkscreen widens a keep-out box. V5's width is 100.05 mm [read at source; computed].
- So **V5's width is set by the connector row, not the core** [inferred]. Any option that does not shorten the row (fewer gaps, a second edge) cannot bring the width under 100.00 mm.
- The XT30's courtyard is 15.85 mm deep (−13.60 … +2.25 mm), so it sets the row's depth [read at source].

### 1.2 Predictions [all inferred]

| # | Claim | Predicted |
|---|---|---|
| P1 | The fixture (V5 + ICM-42688-P + BMP581 + ~30 test points + reset switch + BOOT0 on J2, at 4L) under today's placer | width **100.05 mm** (the same row); height **110–120 mm**; every gate passes, `extract` included |
| P2 | Packing efficiency (sum of keep-out boxes / outline area) | V0 **35–45 %**; V5 **25–35 %**; fixture **28–38 %** |
| P3 | The connector row's share of the outline's area (row band from the core's last margin to the edge, full width) | V0 **15–20 %**; V5 and fixture **10–15 %** |
| P4 | The largest empty rectangle inside the outline | V5 and fixture: at least one of **≥ 15 × 15 mm**, beside the bucks or under the MCU's ring |
| P5 | (a) compaction alone | width unchanged (**100.05**, row-bound), height down **10–20 %**: misses 100 × 100 on width alone, by 0.05 mm |
| P6 | (b) bounded placer alone | as (a), within ±3 mm of it in height; also row-bound in width |
| P7 | (c) connectors on two edges alone | width **80–92 mm** (the core sets it); height unchanged or larger (**108–122 mm**); misses on height |
| P8 | (d) smaller gaps alone (the row's 1 mm → 0.5 mm, the core's IC gap kept unless the router proves less) | width **~96.5 mm**; height down **2–5 %**; misses on height |
| P9 | (e) passives on the bottom (arithmetic only) | frees **20–30 %** of the top-side keep-out area; not built |
| P10 | The combination that first fits 100 × 100 at 4L | **(a) + (c)** (or (b) + (c)) at about **85–95 × 85–97 mm**, every gate passing |
| P11 | Router on the compacted fixture at 4L | track length **down 5–15 %** against today's placer; vias within ±15 %; rip-ups up but every net routes |
| P12 | `emit` wall time on the fixture at 4L (release, host) | **20–90 s**; under three minutes for any option |
| P13 | K3 under the recommended method, spec-level renamings (1307's harness, seeds 0–19) of the reference, D1, D3 | with no bound declared, **16/20, 18/20, 15/20 unchanged** (no byte moves by construction); with a bound declared, **within ±3** of those, since the order is still by reference |
| P14 | Decoupling capacitors after compaction | each stays within its anchor's ring (no capacitor moves away from its pin by more than a few millimetres), parity holds |

## 2. The spike, measured

Host: release CLI built at `9a37ae6` (`main` with 1312; `ec0fcdd`'s engine code, which equals `main`'s but for an svd-hal README) with `spike/1313/placer.patch` applied, in a scratch worktree. Nothing in it is pushed as code. With no `AET_1313_*` variable set, the patched binary emits V0, V5-4L and the fixture byte-identical to the unpatched one [measured]. KiCad steps ran in the pinned tools image through `~/bin/capped-run` (caps read back each run: memory.max 8 GiB, memory.swap.max 0, pids.max 8192; `*.stderr` in the scratch results). Every step's exit is in `spike/1313/results/`.

### 2.1 The fixture (§1.1): `crates/aethelgard-cli/tests/fixtures/sentinel-03.json`

Written by `spike/1313/make_fixture.py`: V5 regenerated from `2b43672`'s `make_variants.py` (read from git), plus:
- the flight sensors as `sentinel-02-own.json` declares them: ICM-42688-P U5 on SPI `IMU` (SCK pinned PB13 there), BMP581 U7 on `SENSORS`, the IMU's net constraints;
- bench access (1312): 29 test points (every declared net, 28, and one `GND`), BOOT0 on J2 pin 2, `mcu.reset_switch`;
- four layers, `four-layer.json`'s `board`;
- **one pin added (a deviation, D2):** `IMU_CS` on PB12, SPI2's NSS, beside SCK/MISO/MOSI on PB13–PB15 (LQFP-144 pads 73–76). Left to the solver (`probes/sentinel-03-cs-free.json`), `IMU_CS` lands on pad 35, across the MCU. The placer seats U5 by that pad, because it is the first shared label net by name. `IMU_SCK` and `IMU_MOSI` then exceed their 30 pF in `extract` under every option, today's placer included: 43.1 and 36.3 pF [measured, `results/options.tsv`, third block]. This is the 3400 practice: a constraint added, none dropped.

`validate` passes it. **What `main` cannot say yet:**

| Missing | Why | Area, estimated (never a part on the board) |
|---|---|---|
| A second `GND` test point | 1312 refuses a net's second test point (`TestPointNetRepeated`) [measured] | one test pad, about 16 mm² with its reference text [inferred from the fixture's 29 test points, 473 mm² together] |
| The witness's capture port (W1) | no capture-port role; a connector cannot carry an undeclared net (3400 §10.1) | 2 × TSSOP-24 66.7 mm², a 2 × 10 header 160.8 mm² (2 × 12: 191.7 mm²), from 3400 §10.1's courtyards. The TPD8E003 ESD part has no KiCad footprint, so it is unmeasured |
| USB FS (W3) | `4300e1` unwritten | USB4105 95.1 mm² + USBLC6-2SC6 (SOT-23-6) 14.5 mm² = 109.6 mm², from 3400 §10.1 |
| E0's hot-plug capacitor (`power.input.hot_plug`, declared and not drawn) | 3400 F1 | UCZ1H470MCL1GS is `Capacitor_SMD:CP_Elec_8x10` (`power-parts.json`), which is not vendored, so there is no courtyard to read. About 9 × 9.5 mm ≈ 85 mm² for an 8 mm can and its pads [inferred, **not** from 3400 §10] |

Together about 420–455 mm² of keep-out boxes [inferred], against the fixture's 2934 mm². The two connectors (the probe header, the USB receptacle) would join an edge row.

### 2.2 Where the area goes (§1.2) [measured: `results/area-v0-v5.txt`, `results/area-fixture.txt`]

| Board (today's placer) | Outline | Keep-out sum | Packing efficiency | Connector row's band | Largest empty rectangles |
|---|---|---|---|---|---|
| V0 | 44.10 × 48.50 mm = 2139 mm² | 964 mm² | **45.0 %** | 352 mm² = 16.5 % | 13.50 × 29.25 (395 mm²); 4.50 × 22.25; 9.25 × 10.50 |
| V5-4L | 100.05 × 107.60 = 10765 mm² | 2346 mm² | **21.8 %** | 1746 mm² = 16.2 % | **21.00 × 99.75 and 21.00 × 99.50 mm** (2095 and 2090 mm²), one each side of the core; 40.75 × 22.75 |
| Fixture | 100.05 × 115.75 = 11581 mm² | 2934 mm² | **25.3 %** | 1746 mm² = 15.1 % | **21.00 × 107.75 and 20.75 × 107.75 mm**, again each side; 17.75 × 47.50 |

- **The width of V5 and of the fixture is the connector row's** [measured]: the core (every part but the connectors) is 57.71 mm wide on V5 and 57.94 mm on the fixture, while the row of eight is 100.05 mm. So a strip about 21 mm wide runs empty, top to bottom, on each side.
- **The height is the core's** [measured]: on the fixture, the two bucks and their inductors stack upward from the MCU (95.23 mm of core), and the XT30's 17.43 mm-deep keep-out box sets the row's band.
- The fixture's passives and test points take 1332 mm² (45 % of the keep-out sum), the ICs 772 and the connectors 830 [measured].

### 2.3 The options on the fixture at four layers (§1.3) [measured: `results/options.tsv`]

The spike's letters (`placer.patch`; every option runs only when its letter is set, with `AET_1313_BOUND=100,100`):
- **(a)** compaction: after today's placer, each non-MCU IC with every part anchored beside it moves as one rigid group, a 0.25 mm step at a time, toward the MCU's centre (x, then y), while every member keeps the gap the placer gave it and stays out of the approach strips. Groups go in reference order, until none moves. **Correction (§5.3): the spike's code moved them toward the first placed part's centre (C1 on this fixture), not the MCU's; every (a) row below measures that.**
- **(b)** bounded placer: each free candidate, from the first free ring out to 40 rings (10 mm) beyond it, is scored by how far the core's box would overflow the bound (less each used edge's connector band, the margins and two snaps), then by the box's area, then by ring order. Anchored parts (`place_anchored`) are not scored.
- **(B)** the same, scored by overflow only: the first free spot that keeps the core inside the bound.
- **(c)** connectors on the left and right edges, cable side out (rotation 270 left, 90 right). Each goes, in reference order, to the side with less length so far. **(C)** the same on the bottom and top.
- **(d)** the connector row's gap at `GAP_NM` 0.5 mm, not `IC_GAP_NM` 1 mm. The core's gaps are unchanged.
- **(o)** ICs in geometric order: those owning anchored parts (the bucks) first, then the larger keep-out box first; the reference only between equals. Its halves, measured apart: **(r)** owners first only, **(s)** larger first only.
- **(e)** is arithmetic only (§2.6).

Every row passed `validate`, `generate`, `emit`, `check erc`, `check drc --spec`, DRC with parity and `check support`, with `findings == []`: **seven of the eight gates pass under every option.** Only `extract` differs. Fit means inside 100 × 100 mm.

| Option | Outline (mm) | Fits | `extract` (fF over the declared 30000, or 100000 for EXT I2C) | Segments / track mm / vias / rip-ups | `emit` s |
|---|---|---|---|---|---|
| today's placer | 100.05 × 115.75 | no | SPI_MOSI 30776 | 1176 / 2067.5 / 277 / 10 | 1.3 |
| (a) | 100.05 × 97.25 | no (width) | SPI_MOSI 30776 | 1184 / 2049.1 / 283 / 10 | 1.2 |
| (b) | 100.05 × 65.90 | no (width) | SPI_SCK 33129, SPI_MOSI 32366, IMU_SCK 34152 | 1353 / 2322.1 / 359 / 23 | 2.3 |
| (B) | 100.05 × 111.75 | no | pass | 1202 / 2081.4 / 298 / 10 | 4.2 |
| (c) | **85.25 × 99.30** | yes | **pass** | 1262 / 2176.1 / 308 / 7 | 2.2 |
| (C) | 62.00 × 122.55 | no | IMU_SCK 30014 | 1245 / 2260.2 / 297 / 6 | 2.0 |
| (d) | 96.55 × 115.75 | no | SPI_MOSI 30776 | 1181 / 2054.9 / 277 / 10 | 1.3 |
| (a)+(d) | 96.55 × 97.25 | yes | SPI_MOSI 30776 | 1189 / 2036.5 / 283 / 10 | 1.2 |
| (a)+(c) | 85.25 × 80.80 | yes | IMU_SCK 31087 | 1348 / 2163.1 / 340 / 27 | 2.7 |
| (b)+(c) | **91.50 × 60.45** | yes | **pass** | 1466 / 2341.1 / 382 / 23 | 2.7 |
| (b)+(d) | 99.80 × 65.90 | yes | SPI_SCK 30152, SPI_MOSI 31186, IMU_SCK 30026 | 1363 / 2338.1 / 361 / 18 | 1.9 |
| (c)+(d) | 85.25 × 99.30 | yes | pass | 1260 / 2174.4 / 305 / 7 | 2.3 |
| (a)+(c)+(d) | 85.25 × 80.80 | yes | IMU_SCK 33158 | 1344 / 2147.4 / 338 / 27 | 3.4 |
| (o) | 100.05 × 115.75 | no | pass | 1241 / 2100.4 / 302 / 8 | 1.3 |
| (o)+(a) | 100.05 × 97.25 | no (width) | pass | 1249 / 2082.0 / 308 / 8 | 1.3 |
| (o)+(c) | 85.25 × 99.30 | yes | IMU_SCK 33879 | 1261 / 2217.0 / 330 / 19 | 3.2 |
| (o)+(a)+(c) | 85.25 × 80.80 | yes | IMU_SCK 34056 | 1287 / 2136.8 / 335 / 22 | 3.0 |
| (o)+(b)+(c) | 93.50 × 60.70 | yes | SPI_SCK 33761 | 1448 / 2303.4 / 406 / 29 | 2.9 |
| (o)+(a)+(d) | **96.55 × 97.25** | yes | **pass** | 1253 / 2069.9 / 301 / 8 | 1.2 |
| (o)+(b)+(d) | 99.80 × 65.65 | yes | SPI_SCK 30724, SPI_MOSI 32221, IMU_SCK 31463 | 1393 / 2393.2 / 391 / 19 | 2.4 |
| (r)+(a)+(c) | 85.25 × 80.80 | yes | IMU_SCK 31087 | as (a)+(c) | 2.7 |
| (s)+(a)+(c) | 85.25 × 80.80 | yes | IMU_SCK 34056 | as (o)+(a)+(c) | 3.0 |

- **The same table on two probes** (`results/options.tsv`, blocks 2 and 3):
  - **Without test pads on the six SPI lines** (`probes/sentinel-03-nospitp.json`, 22 test points), **every option passes `extract` too**: all eight gates under all 22 options. The outlines are within 1.4 mm of the fixture's, except (B) at 3.5 mm.
  - **With IMU_CS left to the solver** (`probes/sentinel-03-cs-free.json`), **no option passes `extract`**: IMU_SCK is 34–49 pF.
- **What decides `extract` is the router's via count, not the outline.** The IMU lines' budget is 30 pF, less U5's declared 10 pF and 1.77 pF of pads, which leaves about 18 pF for track and vias. At four layers a via is 1.43 pF here. The failing IMU_SCK under (o)+(a)+(c) has 8 vias (11.4 pF) on a 25.4 mm route; under (c) it has 4 vias on 17.9 mm and passes. Every SPI failure in the table is between 0.01 and 4.2 pF over [measured: `extract`'s terms per line].
- **The router is not the limit**, at four layers: every option routes, with rip-ups 6–29 against 10 today.
- **`emit` takes at most 4.2 s** for any option (today 1.3 s). P12's 20–90 s was wrong by an order of magnitude; the CI-cost stop is nowhere near.
- **(c) alone takes the width from 100.05 to 85.25 mm**, because the row was the width (§2.2). (d) alone takes it to 96.55. Neither touches the height.
- **(a) alone takes the height from 115.75 to 97.25 mm**: the bucks' groups slide down toward the MCU. (b) takes it to 65.90 by spreading the core sideways into the empty strips.
- **Two emits of the same option are byte-identical** ((r)+(a)+(c), (o)+(a)+(c) and (o)+(b)+(c), checked). A second machine was not tried.
- **Geometry** (`results/geom.txt`): in every layout of every run, every keep-out box is inside the outline, none overlap, and every connector's cable side is on its own edge within 50 µm.

### 2.4 Name dependence: §1.4 (K3), and the fixture under renamings [measured]

**1307's boards** (`results/k3-1307-boards.tsv`). This uses 1307's spec-level renamer (`rename_spec.py`, its code unchanged: a seeded prefix on device references and on bus and CS nets, seeds 0–19), and emits end to end:

| Method | reference | D1 | D3 |
|---|---|---|---|
| today's placer, (c), (b)+(c), (r)+(a)+(c), (o)+(a)+(c), (o)+(b)+(c), (o)+(a)+(d), (o)+(b)+(d) | 20/20 | 20/20 | 20/20 |

- **1307's 16/20, 18/20 and 15/20 no longer reproduce on `main`**, even with today's placer.
- **Placement is still name-dependent.** Today's placer gives 11 distinct layouts (multisets of footprint positions) of the reference over the 20 seeds, and 20 of D1 and of D3. The router now routes every one.
- **So under every method measured, K3's failure is gone at the routing level.**

**The fixture, two renamers, every gate on each renaming:**
- `rename_fixture.py` is 1307's renamer, carried to connector pins and test points, and also prefixing connector and test-point references.
- `rename_perm.py` (new) keeps every reference's length: it permutes references among those of the same prefix and length, and prefixes only net names.
- **The difference matters.** A reference is drawn on the silkscreen, and the keep-out box includes its text (`pcb.rs`, 4300g). 1307's prefix ("TP1" becomes "Qcbc2_TP1") makes every renamed part larger.
  - Measured: the fixture's keep-out sum rises from 2934 to **3845 mm² (+31 %)**. TP1 grows 13.4 → 38.4 mm², U2 15.3 → 51.3 mm², J1 73.9 → 85.3 mm². `rename_perm` leaves it at 2934.
  - So the first renamer measures size as well as order. The second measures order alone.

| Method | Fit ≤ 100 × 100, of 20: order only (`rename_perm`) | Every gate, of 20 | Outline range, order only (mm) | Fit, of 20: 1307's prefix (`rename_fixture`) | Every gate, of 20 |
|---|---|---|---|---|---|
| today's placer | 0 | 10 | 100.05 × 115.00–117.75 | 0 | 5 |
| (c) | 18 | 14 | 84.75–87.70 × 98.55–101.30 | 1 | 6 |
| (b)+(c) | 20 | 5 | 89.50–93.80 × 59.05–61.95 | 8 | 5 |
| (a)+(c) | 20 | 15 | 84.75–87.70 × 80.05–82.80 | 3 | 7 |
| **(r)+(a)+(c)** | **20** | **15** | **84.75–87.70 × 80.05–82.80** | **12** | **13** |
| (o)+(a)+(c) | 20 | 13 | 84.75–87.70 × 80.05–81.55 | 5 | 12 |
| (o)+(b)+(c) | 20 | 5 | 89.70–93.80 × 59.05–61.45 | 20 | 0 |
| (a)+(d) (bottom edge only) | 20 | 12 | 96.55 × 96.50–99.25 | – | – |
| (o)+(a)+(d) (bottom edge only) | 20 | 13 | 96.55 × 96.50–98.00 | 0 (width 101.90–102.05) | 9 |
| (o)+(b)+(d) | 20 | 3 | 99.70–99.80 × 66.15–69.90 | 0 (width) | 3 |
| (s)+(a)+(c) | – | – | | 0 | 7 |
| *probe without SPI test pads:* **(r)+(a)+(c)** | **20** | **20** | | **20** | **19** |
| *probe:* (o)+(a)+(c) | 20 | 20 | | 16 | 20 |
| *probe:* (o)+(b)+(c) | 20 | 20 | | 20 | 18 |
| *probe:* (o)+(a)+(d) | 20 | 20 | | 0 (width) | 20 |
| *probe:* today's placer | | | | 0 | 19 |

- **Every failure in either column is `extract`** (on SPI lines, and on the external I2C lines EXT_SDA/EXT_SCL in 10 renamings with 1307's prefix and 2 order-only, 100.2–113.8 pF against 100 pF). No other gate failed in 653 runs (53 option runs, 320 renamings with 1307's prefix, 280 order-only).
- **Order-only, compaction alone is enough.** (a)+(c) fits all 20 order-only renamings, within ±1.5 mm in width and ±1.4 mm in height, with or without a geometric order. Under (c) alone, two of 20 go over.
- **Under 1307's prefix renamer, where parts grow by 31 %, reserve owners first helps and larger first hurts.**
  - (r)+(a)+(c) fits 12/20.
  - (a)+(c) fits 3/20, (o)+(a)+(c) 5/20, (s)+(a)+(c) 0/20.
  - So the (o) of §2.3 is dropped for its (r) half.
  - Only (b) keeps every prefixed outline in, and (b) is the method `extract` fails most.
- **`extract` is where the methods differ.** Compaction (a) keeps the greedy placer's neighbourhoods and so its short SPI routes: 12–15/20. The bounded placer (b) packs tighter and lengthens them: 3–5/20. With no test pads on the six SPI lines, (o)+(a)+(c) passes every gate on 20/20 under both renamers, and (r)+(a)+(c) on 20/20 and 19/20.
- **Connector edges under (c) are assigned in reference order.** Renaming the connectors moves them between the left and right edges. Every cable side still lands on its edge (`results/geom.txt`).

### 2.5 (e) passives on the bottom: arithmetic only [inferred from measured keep-out boxes]

- **The fixture's keep-out sum is 2934 mm²** [measured]. Its 61 `Device:` passives take 830 mm² (28.3 %) of that, of which the capacitors are 346 mm². The 29 test points take 473 mm².
- **Top-side area left:**
  - with the passives on the bottom, **2104 mm²** (−28.3 %);
  - with the test points there too, **1631 mm²** (−44 %).
- **What that would buy:** at the packing efficiency (o)+(a)+(c) reached on top (42.6 %), 2104 mm² is an outline of about 4940 mm², roughly 70 × 70 mm.
- **It is not needed for 100 × 100:** top-side options fit (§2.3). It is also not free:
  - `pcb.rs`, the router's reader, parity and `extract` model top-side parts only;
  - two-sided assembly is Scotty's cost and his hand-assembly question.
- **Not built, as the work order says.**

### 2.6 Predictions scored (§1.2)

| # | Predicted | Measured | Score |
|---|---|---|---|
| P1 | fixture today: 100.05 wide, 110–120 tall, every gate passing | 100.05 × 115.75; `extract` fails (SPI_MOSI 30.8 pF; unpinned IMU_CS: IMU lines 36–43 pF) | outline right; gates **wrong** |
| P2 | efficiency V0 35–45 %, V5 25–35 %, fixture 28–38 % | 45.0 %, **21.8 %**, **25.3 %** | V0 right; V5 and fixture lower than predicted |
| P3 | connector band V0 15–20 %, V5 and fixture 10–15 % | 16.5 %, 16.2 %, 15.1 % | V0 right; the others just above |
| P4 | an empty rectangle ≥ 15 × 15 mm | two 21 × 100 mm strips | right |
| P5 | (a): width 100.05, height −10–20 %, misses by 0.05 mm | 100.05 × 97.25 (−16 %) | right |
| P6 | (b) within ±3 mm of (a) in height | 65.90 against 97.25 | **wrong**: (b) spreads the core into the side strips |
| P7 | (c): width 80–92, height 108–122, misses | 85.25 × **99.30**, fits | width right; height **wrong** |
| P8 | (d): width ~96.5, height −2–5 % | 96.55 × 115.75 (0 %) | width right; height wrong |
| P9 | (e) frees 20–30 % of the top-side area | 28.3 % (passives), 44 % with the test points | right |
| P10 | first fit: (a)+(c) at 85–95 × 85–97, every gate passing | (a)+(c) at 85.25 × 80.80, `extract` failing (IMU_SCK 31.1 pF); (c) alone fits with every gate | **wrong** on both counts |
| P11 | compacted: track −5–15 %, vias ±15 % | (o)+(a)+(c): track +3.4 %, vias +21 %, rip-ups 22 vs 10 | **wrong**: compaction lengthens routes here |
| P12 | `emit` 20–90 s | 0.8–4.2 s | **wrong**, by an order of magnitude |
| P13 | K3 16/18/15 unchanged without a bound, ±3 with one | 20/20 on all three boards under all eight methods | **wrong**: `main` has moved since 1307 |
| P14 | decoupling capacitors stay by their pins | `check support` (and every gate but `extract`) passes in all 653 runs: 53 option runs and 600 renamings | right |

## 3. §1.5: the recommendation, decisions and questions

### 3.1 The options, in one line each (fixture at four layers; fit means inside 100 × 100 mm)

| Option | Outline on the fixture (mm) | Fits, of 20 order-only renamings | Every gate, of 20 | Every gate on the fixture's own names | Cables |
|---|---|---|---|---|---|
| today's placer | 100.05 × 115.75 | 0 | 10 | no (SPI_MOSI 30.8 pF) | bottom |
| (a) compaction | 100.05 × 97.25 | – | – | no | bottom; misses by 0.05 mm in width |
| (b) bounded placer | 100.05 × 65.90 | – | – | no | bottom; misses in width (row-bound) |
| (c) two side edges | 85.25 × 99.30 | 18 | 14 | **yes** | left and right |
| (d) row gap 0.5 mm | 96.55 × 115.75 | – | – | no | bottom |
| (e) passives on the bottom | about 70 × 70 [inferred] | – | – | not built | – |
| (b)+(c) | 91.50 × 60.45 | 20 | 5 | **yes** | left and right |
| (a)+(c) | 85.25 × 80.80 | 20 | 15 | no (IMU_SCK 31.1 pF) | left and right |
| **(r)+(a)+(c)** | **85.25 × 80.80** | **20** (12 with 1307's prefix) | **15** (13) | no (IMU_SCK 31.1 pF) | left and right |
| (o)+(a)+(c) | 85.25 × 80.80 | 20 (5) | 13 (12) | no (IMU_SCK 34.1 pF) | left and right |
| (o)+(b)+(c) | 93.50 × 60.70 | 20 | 5 | no | left and right |
| (o)+(a)+(d) | 96.55 × 97.25 | 20 | 13 | **yes** | bottom |
| (o)+(b)+(d) | 99.80 × 65.65 | 20 | 3 | no | bottom |

### 3.2 Recommendation: (r)+(a)+(c), only when `board.max_outline_mm` is declared

**Corrected in §5.3:** (a)'s measured gain is the C1 target's; as described, (a) changes nothing on the fixture.

1. **(r)** Place ICs that own anchored parts (the bucks) before other ICs; otherwise today's order. Their loop reserve then never has to clear parts already placed.
2. **(a)** Compact: each IC's group slides rigidly toward the MCU while every gap holds. Decoupling capacitors move with their IC; `check support` passed in all 653 runs.
3. **(c)** Split the connectors between the left and right edges, cable side out.
4. **Then check the bound.** If the outline exceeds it, refuse with `OutlineUnreachable`, naming the outline reached and the shortfall. The bound does not steer the placement, so the refusal is honest: what is reported is what was placed.

Why:
- **It fits with room:** at least 12.3 mm in width and 17.2 mm in height, on every order-only renaming.
- **It is the most robust method measured that does not pack with (b).** It fits 12/20 even when every part grows by 31 % (1307's prefix renamer), against 3–5/20 for the other compaction variants.
- **It has the best `extract` rate of any method that always fits:** 15/20, against 3–5/20 for the (b) variants. Compaction keeps the greedy placer's short routes.
- **It is the smallest change that does all this.** (s) is not needed, and (o) does worse.
- **It is deterministic** (two emits byte-identical) and cheap (`emit` 3.0 s on the fixture).
- **It leaves edge room for what `main` cannot say yet:**
  - The connectors use 45.7 and 43.0 mm of the two 80.8 mm side edges, which leaves about 28–31 mm a side, after gaps and margins, for the witness header (2 × 10, 160.8 mm² courtyard) and the USB-C receptacle (95.1 mm²).
  - (o)+(a)+(d) puts every cable on the bottom edge and uses 88.8 of 96.5 mm of it. Adding those two connectors would take that row past 100 mm [inferred from their courtyards].
- **The cost is the cables' exit:** left and right, not bottom. That is Scotty's mechanical call (Q1).

**If the cables must stay on the bottom edge:** (o)+(a)+(d) or (a)+(d), at 96.55 × 97.25 mm. They fit 20/20 order-only (13 and 12 every gate), and (o)+(a)+(d) passes every gate on the fixture's own names. But under 1307's prefix renamer they fit 0/20 (width 101.9 mm), with 3.45 mm of width margin and no room for more connectors.

### 3.3 Decisions for the architect

- **D1. The method.** Recommended: (r)+(a)+(c) as §3.2. Alternative: (o)+(a)+(d) if cables stay on the bottom.
  - Also in the build: the MCU's gap rule in compaction. The spike's first version let an IC slide to 0.5 mm of the MCU instead of 1 mm. It was fixed before the final runs, and changed no measured layout.
- **D2. The fixture pins `IMU_CS` on PB12** (a constraint added, none dropped).
  - The cause is placement: `place` targets an anchor's lowest pad on the first shared label net by name, which for U5 is `IMU_CS`.
  - An engine fix (target the pads of most shared nets) would make the pin unnecessary. It is outside this work order's §1, so the build keeps the pin unless ruled otherwise (Q2).
- **D3. The six SPI test pads in the fixture** (30 pF lines). On the committed fixture, the recommended method passes every gate on 15/20 order-only renamings (13/20 with the prefix), and not on the fixture's own names. Without those six pads: 20/20 and 19/20.
  - **(i) Recommended.** The build's fixture drops the test pads on `SPI_*` and `IMU_*` (22 test points remain). §2.3's acceptance then holds as measured. The bench reaches those lines at the flash's and the IMU's pins, as 1312's Q2 ruling already asks `BENCH.md` to add a probe's own capacitance there.
  - (ii) Keep them, and the build stops at §7 on `extract`.
  - (iii) Keep them, and a router work order makes via count a cost on nets with a declared bound (Q3).
- **D4. Nothing moves without a bound.** Every option runs only when `board.max_outline_mm` is declared; absent, the placer is today's, byte for byte (checked on V0, V5-4L and the fixture with the spike binary).
- **D5. One `GND` test point, not two:** 1312 refuses a net's second test point (`TestPointNetRepeated`). If two are wanted, that is a 1312 follow-up, not this work order.

### 3.4 Questions

- **Q1 (Scotty).** Are connectors on the left and right edges acceptable? Today they are assigned in reference order. A declared edge per connector would be a specification field, outside this work order.
- **Q2 (backlog).** Should the placer's target for a part be the anchor pad of the most shared nets, not the first by name? That would remove D2's pin.
- **Q3 (backlog).** Should the router count vias as a cost on nets with a declared capacitance? Every SPI `extract` failure here is 0.01–4.2 pF over, and they follow the via count.
- **Q4 (guard).** 1307's spec-level renamer prefixes references, which widens each part's silkscreen keep-out box, so it measures size as well as order. A future name-independence guard should keep lengths (`spike/1313/rename_perm.py`).

### 3.5 For Scotty's quote (no prices guessed)

**Corrected in §5.3:** the 100 × 100 ask stands. Under the method as built the expected outline is about 85 × 99 mm, not 86 × 81, and the spare room below no longer holds.

- **Outline to ask for:** **100 × 100 mm**, the bound the build will declare. The board will be at most that, or `emit` refuses.
  - The expected outline under the recommendation is about **86 × 81 mm**: 85.25 × 80.80 measured, 84.75–87.70 × 80.05–82.80 over order-only renamings.
  - The parts `main` cannot say yet (§2.1, about 420–455 mm² [inferred]) are expected to grow it while staying inside 100 × 100 [inferred]. The three largest empty rectangles of the recommended layout are 526, 455 and 405 mm², and the side edges have about 28–31 mm free each.
- **Open questions for the quote:**
  - layer count: **four**, `four-layer.json`'s stackup (OSH Park's 4-layer);
  - size: up to 100 × 100, expected about 86 × 81;
  - quantity: Scotty's;
  - assembly: **single-sided** (top only; (e) is not needed);
  - the cables leave from the left and right edges (Q1).

## 4. Evidence and rerun

- `spike/1313/` holds the generator, the patch, the runners, three probes and `results/`. Its `README.md` names each file. Emitted boards and per-step JSON were kept only in the scratch tree (3.5 GB with the patched build), which was removed at the stop. The summaries in `results/` hold every number above.
- **The branch's own gates,** on the merge with `main` `f57d9b6`: `cargo fmt --all -- --check` passes; `cargo clippy --workspace --all-targets -- -D warnings` passes; `cargo test --workspace` gives 1999 passed, 0 failed.
  - Run first with `CARGO_TARGET_DIR` outside the worktree, `fab_tier1`'s `a_rail_or_bus_no_fault_kind_reaches_is_named` fails with `BaseOutsideRoot`: its scratch then lies outside the repository. Run in place, it passes. This is an environmental note, not a defect.
- **After merging `main` `d3a0aa3`** (4400b: parts schema 8, solved schema 9; `pcb.rs`, the router and `extract` unchanged), the fixture still validates and emits on a fresh build. CI runs the suite on the head.
- The branch adds no engine code, so it moves no emitted byte.

## 5. The build (§2), as ruled (PR #197, comment 6942817989)

### 5.1 Predictions, committed before any build run [inferred]

| # | Claim | Predicted |
|---|---|---|
| B1 | Every existing emitted file with no bound declared, before the version bump | byte-identical (the bound's code paths do not run) |
| B2 | The bump to 15 | moves only specification hashes, as 1312's did |
| B3 | The fixture (no `SPI_*`/`IMU_*` test pads, IMU_CS on PB12, bound 100 × 100) | **85.25 × 80.55 mm**, every gate passing at four layers with no finding (the spike's `rac` row on the no-SPI-pad probe) |
| B4 | `emit` on the fixture | under 5 s on Furnace; the tier-2 test under 60 s |
| B5 | A bound just above the keep-out sum | `validate` accepts it; `emit` refuses it with `OutlineUnreachable`, naming the outline reached, and writes nothing |

### 5.2 What is built

- **`aethelgard-ir`** (`board.rs`, `lib.rs`): `board.max_outline_mm: { width, height }`, in whole millimetres.
  - Optional under #191 Q1. Absent or `null` is no bound, and is not written back.
  - Each side must be 1 to 1000 mm (`MAX_OUTLINE_MM`, the placer's metre of reach), or it is refused by name with `OutlineBound` and its path.
- **`aethelgard-eda`** (`pcb.rs`, `schematic.rs`):
  - **The pre-placement check** (`pcb::bound_holds`, run in `emit` before placement, `OutlineTooSmall`). Every keep-out box must fit, at some right angle, inside the bound less `MARGIN_NM` a side, and their areas must fit together.
  - **Under a bound, the placer:**
    - (r) puts ICs owning anchored parts first;
    - (a) compacts toward the MCU's centre, with the MCU's 1 mm IC gap (`Rule`);
    - places the connectors by one edge-plan function. `BOUNDED_PLAN = EdgePlan::Sides`, and (d) is `EdgePlan::Bottom { gap: GAP_NM }`, one line away, as ruled.
  - **Then the outline is checked.** Every keep-out box must be inside it, or the emitter reports an internal error. If the outline exceeds the bound, `OutlineUnreachable` names the outline reached and the shortfall in µm, and nothing is written.
  - **With no bound,** every new path is skipped: the bottom row at `IC_GAP_NM`, today's order.
- **CLI `validate`:** runs the same check when a bound is declared and the specification solves (`OutlineTooSmall`, exit 2, at `board.max_outline_mm`).
- **The fixture:** the measured no-SPI-pad probe, renamed `sentinel-03`, with `max_outline_mm` 100 × 100 (D3 (i), 22 test points, IMU_CS on PB12).
- **Tests:**
  - `aethelgard-ir/tests/board.rs`: the bound's range, shape and round trip;
  - `pcb.rs` unit tests: both refusals, the side edges cable out, nothing outside the outline or overlapping, and no bound giving the bottom row;
  - `aethelgard-cli/tests/outline_tier1.rs`: the fixture inside its bound, `validate` refusing 40 × 40 (the sum) and 20 × 200 (U1, named), `emit` refusing 60 × 60 by name and writing nothing.

### 5.3 The finding: (a)'s target [measured]

**Diagnosis.** The spike's `compact` took its target as `placed.iter().flatten().next()`. That is the first placed part in the emitter's item order, which on the fixture is C1. I swapped that target into the build and emitted the fixture: **85.25 × 80.55**, the spike's `rac` figure on the no-pad probe, exactly. Restored to the MCU's centre, as §2.3 describes and the ruling built on, the build gives 85.25 × 99.05.

| On the fixture, bound 100 × 100, four layers | Outline (mm) | Every gate |
|---|---|---|
| **as built: (r)+(a)+(c), (a) toward the MCU's centre** | **85.25 × 99.05** | **8/8**, no finding; `emit` 1.5 s |
| the same, (a) toward the spike's target (C1) | 85.25 × 80.55 | not run as a build |
| the same, (a) toward the core's box centre (one candidate, ordered by nothing) | 85.25 × 99.05 | not run |
| (c) alone (spike, §2.3) | 85.25 × 99.05 on the probe | 8/8 |

**As built, under the 20 + 20 renamings** (`spike/1313/renamings.sh` with the build binary; outlines read from the emitted boards):
- **Order only** (`rename_perm`): 20/20 fit, at 84.75–87.70 × 97.40–99.55 mm, and 20/20 pass all eight gates.
- **1307's prefix,** where the parts grow 31 %: 1/20 fits (92.80 × 99.80). The other 19 are refused by name with `OutlineUnreachable`, over by 1.30–8.45 mm in height. None writes a board.

**Why the spike's target cannot be built as it ran.** It is whichever part comes first in the emitter's list. So the board would depend on the order of the parts, which `pcb.rs`'s `the_board_does_not_depend_on_the_parts_order` holds it never does today [inferred from the property; not run with the C1 target].

**The fork, for the architect:**
- **(i) As described.** On this fixture compaction changes no outline. **Corrected in §6:** it did move one group (U5, the ICM-42688-P, with its capacitors C41–C43) by 2.25 mm inside the outline, so the board's bytes moved when (a) was removed. If ruled, I recommend dropping (a), leaving (r)+(c), rather than shipping code that does nothing here.
  - It fits the fixture at 85.25 × 99.05: 14.75 mm of width margin, 0.95 mm of height.
  - It fits every order-only renaming.
  - A grown board is refused by name, never written.
- **(ii) As the spike measured.** Not buildable verbatim, because of the part-order dependence above.
- **(iii) A compaction defined without part order** that recovers the 80 mm heights: for example, slide toward the MCU and, when blocked, step sideways to the nearest column that frees a step down. **Unmeasured**: a new spike, not adopted unruled. The one other target tried, the core's box centre, also moved nothing.
- **The quote** (§3.5): ask for 100 × 100 still. The expected outline under (i) is about 85 × 99 mm. §3.5's spare room for the witness port, USB FS and E0 no longer holds in height: about 1 mm [inferred].

### 5.4 Other forks, for the architect

- **F2. Whole millimetres.** The work order names the key `max_outline_mm`. The board section's other values are micrometres by `4100`'s D8. I followed the work order. If micrometres are wanted, the key becomes `max_outline_um`.
- **F3. `validate` and an unsolvable specification.** `validate` runs the area check only when the specification solves and the emitter can draw it. Otherwise it returns ok, as it always has for those, and `solve` and `emit` refuse them by their own names. `emit` always runs the check before placement. Is that acceptable under `INV-06`, or should `validate` refuse a declared bound it cannot judge?
- **F4. `null` is no bound,** as an absent key is (tested). The alternative is to refuse `null`.

### 5.5 Measured on the build

- **D4, before any bump.** Every specification in the corpus (the two examples, and every fixture under `crates/` with a `board` and an `mcu`) was run with `validate`, `emit` and `generate`, by `main`'s binary and this branch's. All are **byte-identical**, once the output paths each binary writes are normalised [measured]:
  - first with the fixture still unbounded: 24 specifications, 467 files, 38 boards;
  - again on the final code: the 23 existing specifications. The bounded fixture differs, as it should: `main` refuses its new key.
- **The fixture** (KiCad steps in the pinned image through `~/bin/capped-run`): validate, generate, emit, ERC, DRC `--spec`, DRC with parity, support and `extract` all exit 0 with no finding; `emit` 1.5 s.
- **The bump to 15 [measured].**
  - The constant and its documentation (`lib.rs`, the IR README) moved, and the 23 specification files (22 existing and the fixture) moved from `"schema_version": 14` to `15` and nothing else.
  - Every committed golden that carries a specification's sha256 moved by that hash alone, old hash for new: `boards/sentinel-01/**` (8 files), the hwdef fixtures (3), the svd-hal UI boards and the topology, the CLI README's example line, and `verify_2k2_tier2`'s FAB-NOTES prefix (`1963b05d` to `b9e95cdc`).
  - **The pinned digests moved, and with the old hash put back they reproduce the old digests exactly:**
    - `solve_cli`: `929dabce…`, `3ec179d2…`;
    - `order_2203`: `a60bdcac…`, `a81a65b3…`, `292791aa…`; the boot image did not move.
  - **The differential.** `main`'s binary ran on `main`'s version-14 files and this branch's on the version-15 files: `validate`, `solve`, `emit` and `generate`, with the old hash substituted by the new.
    - All 23 existing specifications produce equal files: 563 files, 44 boards.
    - Only `generate`'s report lines differ, because they digest files that carry the hash.
  - **Version tests:**
    - a version-14 file is refused by its version, with or without the bound, in the library and the CLI;
    - 14 joined the refused list;
    - the version-13 message expects 15;
    - `prove`'s malformed-file test writes version 15.
- **The tier-2 test** (`board_tier2.rs`: the fixture inside its bound, every gate at four layers, no finding) passes in the pinned image, in 8.28 s on Furnace.
  - `TIER2_MIN` is 178, counted in the image: 176, plus 3401's test (merged), plus this one.
  - Its weight, 23.184, is a Furnace estimate (× 2.8, as 1312's was).
  - `verify_2k2_tier2` (its prefix moved) was not run locally: about 20 minutes a test. CI runs it.
- **Predictions (§5.1):**
  - B1 right.
  - B2 right: only hashes moved.
  - B3 **wrong**: 85.25 × 99.05, not 80.55, which is the finding.
  - B4 right: `emit` 1.5 s; the tier-2 test 8.28 s.
  - B5 right: `outline_tier1.rs`, 60 × 60.
- **Gates on this branch.**
  - Before 3401 merged (main `19741d2`): fmt and clippy pass, and `cargo test --workspace` gives 2066 passed, 0 failed, run in place. The 12 mutants were re-run after the reduction pass, with the same result.
  - After merging `main` `a323aea` (3401 and the 2303 record):
    - fmt passes;
    - clippy passes, both plain and with `aethelgard-cli/with-tools`;
    - `cargo test --workspace` gives **2068 passed, 1 failed**: the package binding of §5.8.

### 5.6 Adversarial pass (WO §5) [measured]

- **A part that leaves the board, sits over the edge, or overlaps another.**
  - Every placement's keep-out boxes are checked against the outline under a bound. The guard is labelled `belt-and-suspenders:`, because no plan can trip it; its mutant survives (M5).
  - The unit tests assert containment, no overlap, and each connector's cable side within 50 µm of its edge.
  - The spike's geometry check covered 653 layouts.
- **DRC passes but parity breaks, or a decoupling capacitor moves.** Compaction moves an IC's group rigidly, its capacitors with it. DRC with parity and `check support` pass on the fixture, in tier 1 and in tier 2.
- **Nondeterminism.** The fixture was emitted twice on Furnace, and once by the same binary inside the pinned image (another OS and file system). All three are byte-identical (board `05bbef2c…`). A second machine of another kind was not tried; CI's tier-2 run is the next one.
- **A bound just above the keep-out sum.** At 60 × 60 the area check passes and the placer cannot reach it. `emit` refuses it with `OutlineUnreachable`, exit 2, and writes nothing (`outline_tier1.rs`).
- **A connector moved off the edge its cable needs.** Rotations 270 left and 90 right, cable side on its edge (unit test). With no bound, every connector stays at rotation 0 on the bottom (tier 1).
- **Mutants: one per guard this work order adds, 12; 11 killed.**
  - M1 the bound check, M2 the area sum, M3 the per-part fit, M4 the IR range, M6 the MCU's 1 mm in compaction, M7 (r), M8 the side balance, M9 `validate`'s call, M10 the left edge, M11 placement ignoring the bound, M12 the IC gap in compaction: all killed.
  - M5 survives, the labelled guard above.

### 5.7 Reduction pass

Reduction pass (diff): −1 copy (one `bound_nm` for two mm-to-nm conversions) · `spike/1313/` deleted (the spike's record stays at `ede77a8`; builds delete `spike/`) · 1 protected kept (M5's labelled guard) · 1 finding: (a) itself, which changes no outline on the fixture as ruled (§5.3, fork (i); dropped by ruling, §6).

### 5.8 The stop: 3401's CI-written fabrication package [measured]

- **The merge.** `main` `a323aea` (3401 READY, #190; the 2303 spike record, #200) was merged with a merge commit.
  - `ci.yml`'s `TIER2_MIN` became 178; both comments were kept.
  - `board_tier2.rs` is `main`'s file with this work order's test appended. No line of `main`'s was removed.
- **The bump, for 3401's new files.**
  - `examples/sentinel-01.json` moved from 14 to 15: its hash went from `e1e4f6a5…7615` to `411cef0c…e437`.
  - `boards/sentinel-01-bench/**` (8 files) carries that hash and moved by it alone. `aethelgard-svd-hal/tests/board.rs` regenerates the crate and passes.
- **The package.** `fab/sentinel-01-bench/package/` carries the old hash in `MANIFEST.json` (`specification_sha256`, and `topology_sha256` through the solve) and in `FAB-NOTES.md`. It also binds `verdict_sha256`, the digest of `verify`'s whole verdict, sandbox included, which no substitution can reproduce.
  - The ruling on #190: "the committed package is the one this job writes (dispatch with `write`, then commit the artifact's files), never one written on another machine".
  - So I left it untouched. It is the one red test, until the dispatch or a ruling.

## 6. After the ruling on the second stop (PR #197, comment 6944446645)

### 6.1 What each ruling became

- **§5.8, the package.**
  - The architect dispatched `ci.yml` with `write` on `wo/aet-1313-outline`: run **44058299670**, on `556b303`.
  - Its `bench-package` artifact (id **14318433213**; zip sha256 `a5558a81…`, 51 427 bytes, as the architect gave them) was downloaded with the read-only token and committed unchanged into `fab/sentinel-01-bench/package/`.
  - The artifact holds the same 15 files. Only `FAB-NOTES.md` and `MANIFEST.json` moved (the specification's sha256, now `411cef0c…`); the Gerbers, drill, positions, BOM and ELF are byte-identical.
  - The run's red jobs were the stale binding test alone (`gates`, `tools-shard (3)`, and `tools` aggregating it), as the architect said; I read their logs. With the package committed, `bench_board_tier1` passes.
  - Nothing committed after `556b303` moves Sentinel-01's bytes. Measured: `emit` and `generate` of `examples/sentinel-01.json` and `examples/sentinel-01-minimal.json` are byte-identical between `556b303`'s binary and this branch's, after both changes below. So no re-dispatch is needed.
- **§5.3 → (i): (a) dropped,** removal only: 0 lines added, 148 deleted from `pcb.rs` (the compaction, its gap bookkeeping, and its test). (r)+(c) remain.
  - **A correction to §5.3** [measured]. The fixture's outline did not change (85.25 × 99.05). But its board did: compaction had moved one group, U5 (the ICM-42688-P) with its capacitors C41–C43, by 2.25 mm in y inside the outline, and removing it moved them back. Those 4 of the 100 footprints were the only ones that moved. So "(a) moves nothing" in §5.3 was true of the outline, not of the bytes.
  - The new placement passes every gate: tier 1, and the tier-2 test in the image (7.63 s). `extract` passes, the SPI lines at 12.3–16.5 pF of 30.
- **F2 → `board.max_outline_um`**, in micrometres, 1 to 1 000 000 a side (`MAX_OUTLINE_UM`).
  - The refusals name the bound and the outline in millimetres for reading (`OutlineTooSmall`, `OutlineUnreachable`: `width_um`, `height_um`).
  - The fixture declares 100 000 × 100 000.
- **F3, accepted.** The IR README now says it beside the key: `validate` judges the bound only when the specification solves; `emit` always judges it before placement.
- **F4: an explicit `null` is refused.**
  - That is what the existing optional keys do. `test_points` (a list) and `mcu.reset_switch` (a bool) both refuse `null` as the wrong type: tested, in `a_malformed_outline_bound_is_a_shape_error`.
  - So the bound takes `default` with a `present` deserializer. Absent is no bound; `null` is a shape error (`SchemaViolation`).
- **M5, the reduction and `TIER2_MIN` 178:** accepted, unchanged.

### 6.2 Gates on the final head

- **Mutants for the guards this round touched,** re-run as a set of 12: 11 killed.
  - M1 the bound check, M2 the area sum, M3 the per-part fit, M4 the micrometre range, M7 (r), M8 the side balance, M9 `validate`'s call, M10 the left edge, M11 placement ignoring the bound, M13 `null` read as absent, M14 micrometres read as millimetres: all killed.
  - M5, the labelled guard, survives.
  - M6 and M12 went with (a).
- **fmt; clippy, plain and with `aethelgard-cli/with-tools`; `cargo test --workspace`:** fmt and both clippy runs are clean; **2068 passed, 0 failed**, run in place on the final code. (One test fewer than before: the compaction test went with (a).)
- **The tier-2 test** passes in the pinned image on the final placement (7.63 s; caps read back, swap 0).
- **`TIER2_MIN` 178,** counted in the image.

