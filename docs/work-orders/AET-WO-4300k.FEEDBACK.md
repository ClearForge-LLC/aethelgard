# AET-WO-4300k — FEEDBACK: Sentinel-03 fabrication readiness (specification 30, power-parts 12)

**Branch:** `wo/aet-4300k-sentinel-03-readiness`. **Base:** `0faaaec3`. **Builder:** Claude (Opus 5.5) as project manager, with subagents (Sonnet 5.5 unless noted) for measurement, datasheet reading, library vendoring, the acknowledgement reasons (Opus: judgement) and tests. Each subagent's report was spot-checked against its artifact before use (§0).

## 0. How it was made

- **Measure, stop, build.**
  - Predictions were committed first (`521dcd06`), then the measurement and the stop after §1.2 (`spike/4300k/RESULTS.md`, `STOP.md`).
  - Ruled 2026-11-24T09:45Z: every lean taken, with (b) OSH Park.
  - Two questions were asked during the build and ruled:
    - Q1 (the fuse), 10:00Z: (a), SF-1206F700-2.
    - Q2 (the thermal copper), 10:30Z: step 1.
- **Subagents, and what was checked of each:**
  - the key-deletion sweep (host), then its image extension for `check support/erc/drc`;
  - the 36-specification blast table;
  - thermal, fabricator and held pages;
  - the datasheet rows (spot-checked in the extracts: VGSS ±20 V, RDS(on) 33 mΩ at −4.5 V, SF-1206F400-2 4.00 A 16 mΩ 0.954 A²s);
  - the KiCad vendoring (every blob SHA checked against GitLab's API);
  - the JST mating sides;
  - the 46 acknowledgement reasons (Opus);
  - the new test files.
- **OSH Park's page** was fetched by me (sha256 `cf14c792…`) to confirm every value the worker read: drill size ±2.5 mil, position 2 mil, annular ring 4 mil, prepreg ±0.0202 mm, core ±0.0991 mm, and an edge keepout of 15 mil, inside the engine's 0.5 mm.
- **A scratch test file was committed by accident.** `crates/aethelgard-cli/tests/zz_scratch_acks.rs` went in with WIP commit `b52c57d5` and was removed in `01c65708`. It is in the branch's history only. A squash at merge removes it.

## 1. The measurement and the stop (§1.1–§1.2)

See `spike/4300k/RESULTS.md` (M0–M12, predictions scored) and `STOP.md` (decisions (a)–(e), D6–D14).

- **No §7 stop on documents:** every held hash matched; the crystal passes AN2867 Rev 25; the XT30 drawing puts the opening away from the tails.
- **Corrections the advisor review made before the push:**
  - the fuse current: the tree's `input_steady_milliamps` is the outputs' sum, not the input;
  - D6 and D13 are one decision;
  - (c) rests on how `--against-fab` compares;
  - the fabricator was not JLCPCB-by-format.

## 2. What was built

PENDING (written at the close).

## 3. The adversarial pass (§5, full)

PENDING.

## 4. Versions and what moved

- Specification 29 → 30 (`versions::CHANGES` row 30). Power-parts 11 → 12: new kinds `pfet`, `zener`, `fuse`, `supervisor`, `bor_falling_min_millivolts` on the MCU row, and LMR33630's cin minimum enforced (A6).
- `parts.json` stays at 13: `supply_by_read_clock` is optional, and only W25Q64JVxIQ carries it.
- `FAB-NOTES.md` moves with the bump (granted into the fence, ruled at 09:45Z).

## 5. Decisions the architect made, and how they were built

- **(a) Pours drawn, measured (ii-a); Q2 step 1.**
  - On a board with a buck, GND pours go on every outer layer no plane fills (`Stack::pours`).
  - `aethelgard_eda::pour::joined` measures a lower bound per layer:
    - 50 µm cells;
    - other-net copper grown by the clearance, and the edge by 0.5 mm;
    - the fill's 0.25 mm minimum width as an opening;
    - the component reached from the pad or its thermal vias;
    - a nearer-pad split (ties count for neither).
  - `extract.thermal` screens each buck by `power::junction_screen` at the least joined layer.
  - **Measured** on the drawn board (cm², F.Cu / In1.Cu / B.Cu):
    - U10: 20.10 / 38.51 / 36.59, needs 15;
    - U11: 13.14 / 24.77 / 23.82, needs 7.
    - With KiCad's default reliefs on every GND pad, U10 was 13.28 and U11 6.54. That fired the stop, and Q2 ruled step 1.
  - **A deviation from Q2's "keep reliefs" (reported 10:4xZ).**
    - With reliefs on the 0402/0603 passives, KiCad's DRC refused two starved reliefs (C2 and C38, one spoke against two).
    - Those pads are instead not joined to the pour (`zone_connect 0`) and keep their own vias to the GND plane. There is no relief to starve and no tombstoning exposure.
    - No small passive is joined solid, so `solid_gnd_join_small_passive` has nothing to list.
  - **(b) and (c) stay Scotty's options** (ruled 10:30Z): growing the outline toward `max_outline_um`, or lowering U10's 2 A (the DS-009 port budget). Use the numbers above.
  - **4300d2 D2 (a), the keepout's "no fill on its layer", is relaxed for pours, not planes.**
    - KiCad's refill keeps a pour out of U5's keepout, `pour` counts the keepout as no copper, and the DRC gate judges the filled board (it passes).
    - Planes keep the refusal.
  - **Assembly:** Scotty confirms the solid joins with the assembler before the order.
- **(b) OSH Park, declared by `board.stackup.fabricator`.**
  - Bounds cite one of its documents by id, a standard, or a condition by id.
  - Its two unpublished tolerances (track width; and thickness, whose min and max are derived from its published layer tolerances) are handled so: track width is a `{condition}` (`track-width-tolerance`), and thickness cites OSH Park as a derivation.
  - Copper max, permittivity max (IPC-4101 not held), the three copper and plating minima and copper temperature are `{condition}`s.
  - **Scotty confirms the fabricator before any order;** switching it is a data edit.
- **(c) `FabricationNotReady`** is a pre-gate refusal under `--fab-out` only, with no MANIFEST row. Under `--against-fab` the verdict's `fab` report names it "not judged". **The gap:** a later engine that adds a readiness key does not re-judge an existing package until it is re-written.
- **(d) The key list:**
  - `fab_tolerances`, `fabricator`; with `power`: `drop_limits`, `conditions.dc_bias_derating`, `loads` and `net_classes`. The image sweep added `net_classes`, whose emptiness drops DRC's power classes.
  - `diff_pairs` is not on the list; it is named where the USB pins are reserved (`usb_pair_undeclared`).
  - Condition ids come from the code's sites: `Ctx::screen` in `power.rs`, plus `supervisor::check`, `board::conditions`, `clock::conditions`, and the readiness module's spec-read ids.
  - 71 on Sentinel-03, each acknowledged with a reason (`spike/4300k/build/s03_spec.py`).
- **(e) and Q1:**
  - The protection is a fuse SF-1206F700-2, a P-FET DMP6023LSS-13, a 12 V zener MMSZ5242BT1G and a 100 kΩ gate resistor, between J4.2 and `VBAT`, with the sense divider after the FET.
  - The input current, computed from the engine's own losses, is 1,640 mA at 9.0 V. The input path drops 70 mV, counted in the bucks' headroom.
  - **"About 2×" was amended by Q1:** the hot-plug I²t bound (C·V²/2R_min, the fuse's own minimum the only R) clears only the 7 A part: 2.35 A²s against 3.25.
  - Condition `fuse_hot_plug_bound_fuse_only:F1`. The fuse's 32 VDC rating is judged against the steady 17.4 V.
- **DMP6023LSS's "ADVANCE INFORMATION"** is white text in the left margin, present in the text layer of every page and invisible on the rendered page. It is kept as a condition (`datasheet_advance_information:Q1`), as ruled.
- **D6:** the TLV803S's rising maximum is VIT− max (2.99 V) plus its typical hysteresis (40 mV), judged against VDD_3V3's solved minimum less its 110 mV drop limit (3,034 mV, the solved minimum floored to whole mV). The margin is 4 mV, with the condition `supervisor_hysteresis_typical:U12`.
- **D7:** the factor 2 is labelled a derivation in the refusal, the condition and the code.
- **D8:** two input ceramics per buck: C60 and C61, 10.87 µF effective each pair at 17.4 V, worst case.
- **D9:** `board.forward: top`, U5 `x: right`. After J4's turn, IMU_SCK is 21,478 fF and IMU_MOSI 21,268 fF against 30,000.
- **D10:**
  - Sentinel-03's 46 ledger register/class entries are in. Every row is acknowledged.
  - Four held reasons the crystal and the new pins made false were rewritten: USART1.CR1 (FIFOEN set), GPIOB.OTYPER (PB10/PB11 open-drain), RCC.CR ignored-by-model (the HSE bring-up writes it), RCC.D1CCIPR (SPI1/2 on pll1_q).
  - **The reasons, for review:** PENDING (listed at the close from the specification).
- **D11:** W25Q64JV's §9.2 rows are held by read clock (`supply_by_read_clock`); at 20 MHz, 2.7 V applies.
- **D12:** the flash floor is refused by `--fab-out` (`FabricationNotReady`, id `U4`) and is otherwise listed by `supervisor::floors_above_reset`. The Sentinel-01 bench row asks: did the board ever run under 2.7 V?
- **D13:** `DropLimitUndeclared` for the MCU's supply; every other regulated domain is `drop_not_judged`.
- **D14:** `sentinel-02-power` is 4S, with a 165 mV VDD_3V3 limit and the same FET, fuse, zener and gate resistor (`spike/4300k/build/s02p_spec.py`).

## 6. Rows (§7.1)

- **Q1 (c):** hold the pack's or the FET's minimum resistance, recompute the hot-plug I²t, and possibly step the fuse down to 4–5 A.
- **The supervisor as the reset floor in the twin:** with a supervisor on NRST, the reset is at its VIT−, not BOR3. The droop scenarios and the 2,680 mV MCU consumer floor still assume BOR3. This is for the twin's work order (D6).
- **(b) and (c) of Q2:** see §5 (a).
- **The supervisor row's board temperature** is not judged (its −40 to 125 °C covers the board's −20 to 85 °C); `supervisor::check` could judge it as `power` judges every power part.
- **1609 F2 closed for new packages:** `fab_tolerances` is a readiness key. Sentinel-01 is untouched, by (c).
- PENDING: rows from the adversarial pass.

## 7. Gates, CI, package

PENDING.
