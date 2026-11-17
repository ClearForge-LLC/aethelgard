# AET-WO-1313 — FEEDBACK

## 0. Status: STOPPED at §1.5 for the architect's ruling

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
- **(a)** compaction: after today's placer, each non-MCU IC with every part anchored beside it moves as one rigid group, a 0.25 mm step at a time, toward the MCU's centre (x, then y), while every member keeps the gap the placer gave it and stays out of the approach strips. Groups go in reference order, until none moves.
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
