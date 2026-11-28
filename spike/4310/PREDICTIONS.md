# AET-WO-4310 §1.1 — predictions, committed before the runs they predict

Written 2026-11-27T05:46Z by the 4310 builder, from main `adbd5c66` (4411 #395 and 4412 #410 merged; 1802's 0.3.0 version bytes not yet on main). Nothing below has been run with a 4310 probe or prototype. Each prediction is scored in `RESULTS.md`, right or wrong, and never edited after its run.

**Already read before writing, so not predicted:**
- **The code:** `ir/src/supervisor.rs`, `ir/src/radio.rs` `not_judged`, `ir/src/power.rs` `headroom`, `chaos/src/campaign.rs` `rail_levels`, `cli/src/verify.rs` `stress`/`classify`/`coverage`, `eda/src/supply.rs` `bound`/`drop_bound`, `cli/src/extract.rs`, `cli/src/fab.rs`, `cli/src/rebind.rs`, and `.github/workflows/ci.yml`'s `bench-package` and `bench-package-s03`.
- **The baseline emit of all 43 JSON files** in `examples/`, `cli/tests/fixtures/` and `ir/tests/fixtures/` (main's debug build, `out/base-emit.tsv`):
  - **27 draw:** 4411's 26, plus Sentinel-03 (103.5 s).
  - **16 are refused:**
    - 10 scenario files, `UnsupportedSchemaVersion`;
    - 4 `DrawsNoBoard`;
    - `d2-drv8833`, `PartLimitUndeclared`;
    - `invalid-zero-domain`, `ZeroNominalVoltage`.
  - **The ladder runs on four:**
    - `sentinel-01-ready`: rung 7, no step;
    - `sentinel-03-radio`: rung 5, step 0;
    - `sentinel-02-power`: rung 0, no step;
    - Sentinel-03: rung 7, step 1, 1000 ‰.
- **The committed scenario sets** (`scenarios/*`), both packages' `MANIFEST.json`, and Sentinel-03's `FAB-NOTES.md`.

**The boards named below:**
- **The 25 INV-08 boards** are the 27 that draw, less `sentinel-03-radio` and `examples/sentinel-03.json`. 4411 measured the module board as moving.
- **"Sentinel-01"** is `examples/sentinel-01.json`, whose package is `fab/sentinel-01-bench/package`.
- **"Sentinel-03"** is `examples/sentinel-03.json`, whose package is `fab/sentinel-03/package`.

## Four facts that change what (1) can measure (found in the reading)

- **F-a. 3303 F5 (the droop's loaded floor) is already built.**
  - `verify.rs` L1625–1646 and `chaos` `rail_levels` judge the floor less load × source resistance (2304b §1.10, L697; 2304b FEEDBACK L169, "L697 done (the loaded floor)").
  - Roadmap L1017 was never closed. What is open is the *source* of both numbers. Every committed droop scenario states `load_milliamps` 100 and `source_milliohms` 500 with no source.
- **F-b. K6 is half built.** `headroom` already subtracts the fuse and P-FET drop (`power.rs` L2743, 4300k (10)). It does not subtract `power.battery.series_milliohms` × the input current.
- **F-c. extract cannot recover the rung or the pour step from the board file.** They exist only in `emit`'s memory and JSON. The board's header carries `generator` and `generator_version` only.
- **F-d. `--against-fab` reads no committed package field.** It re-derives every file and byte-compares (`verify.rs` L2797, `tree::against`). The only readers of committed package text are `rebind.rs`'s (`RebindMoved`).

## (2) The supervisor floor for every powered part

**Today:** `devices()` takes I2C and SPI devices on `mcu.supply` only. A part with no `parts.json` row is skipped silently (L163). The result reaches only `readiness` under `--fab-out` (first floor only). No plain verdict names a floor, although the module's doc says "every other verdict names it (ruled D12)".

- **P2.1. A rail reading must be ruled**, because the outcome turns on it:
  - **(A)** every part with a declared minimum on the MCU's supply;
  - **(B)** every part against its own rail's reset (a rail with no reset names its parts);
  - **(C)** every powered part on any rail against the MCU's reset, as `radio::not_judged` compares today.
- **P2.2. Under (C) Sentinel-03 gains one new floor: U30 (ESP32-C3-WROOM-02-N4), 3000 mV against U12's 2870 mV.**
  - `--fab-out` is then refused, `FabricationNotReady` U30, so the architect's write dispatch on this head fails until it is fixed or ruled.
  - `--against-fab` does not judge readiness, so it is unchanged.
  - `sentinel-03-radio` gains the same floor. U30 is on `VDD_3V3` there.
- **P2.3. Under (A) U30 is not judged, since it is on `VDD_3V3_RF`, and Sentinel-03's new parts are:**
  - U1 (STM32H753ZITx, 1620 mV), below the reset;
  - U12 (TLV803SDBZR, VDD 1100 mV), below;
  - U31's A side (TXU0304PW), **named**, since its row holds no operating minimum in a field. The 1.08–5.5 V of SCES935A p. 6 §7.3 is in a citation string only, and the PDF is not in `~/.cache/aet-held/`.
  - If "named, never passed" blocks readiness, Sentinel-03's `--fab-out` is refused for U31 until that row carries the field.
- **P2.4. The MCU's own minimum is a new floor on every BOR-level-0 board.** VBOR0's falling minimum is 1580 mV, under the MCU's 1620 mV (DS12117 Table 111, p. 209).
  - Predicted on the eleven BOR-0 boards that draw: `connectors`, `d1-24-i2c`, `d3-24-i2c-12-spi`, `four-layer`, `four-layer-power`, `four-port`, `motors-pwm`, `pair-standin`, `serial-four`, `two-bme`, and `sentinel-02-bench`.
  - Each already has bus-device floors (1710 > 1580), so readiness was already refused on each. No package is written from any.
  - BOR levels 1–3 (1950–2540 mV) are above 1620: no MCU floor.
- **P2.5. The Sentinel-01 family is unchanged.** BOR3 is 2540 mV. U4's 2700 mV flash floor is pre-existing (`readiness_4300k.rs` L377). Sentinel-01's package is written with `--rebind`, which judges no readiness.
- **P2.6. Regulators, passives, connectors and off-board peers** (DS-009's 4.75 V, the EXT port) are not "powered parts with a minimum" in any held row. Predicted: named as out of the rule's scope, never judged.
- **P2.7. Bytes.**
  - If the floors reach only readiness (today's wiring), no plain-verify verdict moves on any board.
  - If (2) adds the D12 naming line to the plain verdict, every verdict of the 25 moves by that one field.

## (3) The supply bounds, loaded

- **P3a. The droop.**
  - **The loads are held:** the solved domain load, `checked.domains[d].load_milliamps`, is 750 mA on Sentinel-03's `VDD_3V3`. Sentinel-01 has no power tree, so it holds no load, which is named.
  - **The source resistance is held nowhere.** No regulator row states an output impedance or a load-regulation figure that gives one.
  - Parts and pages needed:
    - LMR33630ADDA (U10, U11), SNVSAN3F, not held as a PDF;
    - LMR33620ADDA (U13), SNVSAW1E, held;
    - TLV75518, SBVS320D;
    - Sentinel-01's `VDD_3V3`, whose source is off-board (the NUCLEO's or bench supply's), declared nowhere.
  - **This is a §7 stop.** The table says what the rule needs.
  - With the topology's 750 mA at the scenario's own 500 mΩ, Sentinel-03's loaded floors fall by 325 mV: `droop-tolerated` from 2950 to 2625 mV, `droop-bor3` from 2350 to 2025 mV. Both stay below the 3135 mV band, `droop-bor3` still crosses BOR3, and no coverage class moves.
  - The rendered levels move, so each droop run's trace and the verdict bytes move: Sentinel-01, Sentinel-01-bench, Sentinel-03 and `motors`.
- **P3b. Headroom.** The change is −(input current × `series_milliohms`).
  - Sentinel-03: 1906 mA × 1 mΩ = 1.906 mV off each battery-fed regulator's VIN minimum (U10, U11, U13). No verdict flips, since each regulator's margin is far above 2 mV (VIN minimum 3800 mV against about 8.9 V).
  - `sentinel-03-radio` and `sentinel-02-power` move by their own I × series.
  - Every board without a power tree is unchanged.
  - If `solve` prints the headroom figure, Sentinel-03's topology bytes and its package move.
- **P3c. Other domains' return currents** (the trigger, roadmap L1107, is met on Sentinel-03). Today's figures from 4411 FEEDBACK L113–114:
  - `VDD_3V3` is 85.7 of 110 mV (margin 24.3; worst VSS 65.3 mΩ, pin 31);
  - `VDD_3V3_RF` is 122.7 of 144 mV (margin 21.3).
  - **Charged as today's bound charges its own load,** at the worst VSS bound, `VDD_5V`'s 2000 mA alone adds 130.6 mV. Predicted: **`VDD_3V3` fails**, at about 216 mV against 110.
  - **A sounder bound** charges each other return at its transfer resistance, at most √(R₁₁·R₂₂), measured with `supply::bound` on the routed copper. Predicted:
    - `VDD_5V`'s term on U1 is still ≥ 25 mV, which still fails `VDD_3V3` (≥ 110.7 mV);
    - the battery's return (the bucks' GND pins to J4) is **named, not counted**, because J4's pads are through-hole (`TerminalDrilled`).
  - **On the other boards:**
    - `sentinel-03-radio` fails in the same way;
    - `sentinel-02-power`'s gate cannot run (`TranslatorPinUnbounded`), so it is unchanged;
    - no other board declares a drop limit, so the rest are unchanged.
  - **A new finding on Sentinel-03,** fixed only by layout: the 5 V loads' and the bucks' returns kept out of U1's return copper. That is a ground split or a star return at U11, and a placer or board work order, not a rule change.
- **P3-S2.** Roadmap L1276 (split each drop limit into its VDD and VSS shares, 4411 S2) is tagged slice D but is not in this work order's grounds. **Flagged, not built.**

## (4) The package says what it was built at

- **P4.1. How `extract` learns the rung and the step** must be ruled (F-c). The options:
  - **(a)** `emit` writes them into the board file. This moves the board bytes of the four laddered boards only, with the Gerbers unchanged, and costs nothing at run time. **Lean.**
  - **(b)** a sidecar beside the board that `extract` reads.
  - **(c)** `extract` re-runs the ladder, which adds about 103 s per Sentinel-03 extract.
- **P4.2.** "A package that lacks them is refused by name" needs an explicit absence on boards that run no ladder or no step (`not_run`, not a missing key). Then:
  - every `extract` verdict of the 25 moves by that field;
  - so does every `verify` verdict that carries extract's keys.
  - If extract's keys are carried into `verify`'s verdict (today `gate_report` copies only fixed keys), every `verdict_sha256` moves.
- **P4.3. FAB-NOTES gains one line each** from the board's `Edge.Cuts` rectangle:
  - Sentinel-03, 85.00 × 99.75 mm;
  - Sentinel-01-bench, 46.35 × 54.85 mm.
  - `FAB-NOTES.md` and `MANIFEST.json` move in both packages; the Gerbers, BOM, ELF and placement files do not.
  - No copper-extent code exists, so "from the copper" is read as the `Edge.Cuts` copper-layer outline.
- **P4.4. The Sentinel-01-bench write breaks.**
  - Its write runs `--fab-out … --rebind fab/sentinel-01-bench/package`, and `rebind`'s `notes_bound_alike` refuses a FAB-NOTES that gained a line (`RebindMoved`).
  - A write without `--rebind` judges readiness, which refuses U4 (P2.5).
  - **So Sentinel-01-bench cannot be re-written** until `rebind` learns (4)'s line or a ruling names the path.
- **P4.5. "Both strict readers" has no referent in today's code** (F-d). A package lacking the fields shows today as `against_differs` naming `FAB-NOTES.md` and `MANIFEST.json`, not a by-name refusal. A by-name refusal needs a package reader, which is new code. Which two readers are meant must be ruled.

## (5) Coverage that means what it says

- **P5.1. (i) Tolerance.**
  - No I2C pull-up declares a tolerance: `I2cBus` holds `pull_up_ohms` only, and the emitted R1/R2 are bare `Device:R 2k2`. So under 3303 F3's ruling, every pull-up "tolerated" cover becomes "needs a detected fault".
  - **Sentinel-01-bench loses I2C `SENSORS`.** Its only cover is `pullup-3k3`. The precondition goes `unmet`: plain verify's coverage is short, so `--against-fab` and the write both fail. The `bench-package` job goes red.
  - **Sentinel-01's seeded set keeps `SENSORS`** through `held-low-sda` and `held-low-scl`, detected beside the `pullup-2k2` baseline.
  - No committed pass scenario raises a capacitance.
  - **The fix is a scenario, not a rule:** add `held-low-sda` on `SENSORS` (the BME280 is modelled) to `scenarios/sentinel-01-bench`. Its MANIFEST scenario list and verdict move, and the campaign gains one run.
- **P5.2. (ii) Unmodelled-only buses.** TMP117 is the only unmodelled part.
  - The TMP117-only buses on drawing boards are `spi-only` `SENSORS` and `logger-4600` `TEMP`. Their tests' coverage moves; no package board is affected.
  - **I2C `EXT` (no device) on both packages** stays `no_device` (ruled (c)) only if (5)'s "named as uncovered" leaves a device-less bus non-blocking. If it blocks, both packages fail. That is a ruling.
- **P5.3. (iii) Motor outputs.**
  - **Sentinel-03's four DShot600 outputs become targets with no cover,** so its precondition goes `unmet` and `bench-package-s03` goes red until scenarios are added. The fix is one output-stuck scenario per output, four scenarios, plus the fault kind itself.
  - **The other drawing boards with motor outputs** are `driver-judges-nothing`, `hse-crystal-16`, `motors-dshot600`, `motors-pwm`, `sentinel-02-shaped`, `sentinel-03-radio` and `serial-dshot-9600`. Each test asserting their coverage moves.
  - The witness is today's `motor_output_unwitnessed` / `motor_timing_mismatch` "stopped".
- **P5.4.** On both packages `campaign_faults_every_bus_and_rail` is predicted `unmet` after (5) until P5.1's and P5.3's scenarios are added, and `met` with them.

## CI cost

- **(2), (3b):** negligible, pure IR arithmetic.
- **(3c):** one `supply::bound` per other domain per limited domain. Predicted under 30 s per board with drop limits, about 2 boards.
- **(4)(a):** none. **(4)(c):** about +103 s on `bench-package-s03`.
- **(5):** Sentinel-03 gains 4 campaign runs and Sentinel-01-bench 1. Predicted 20–40 s each on CI, so `bench-package-s03` grows 1.5–3 min, from 28.0–33.6 min on 1919's three runners, under 1919's 45 min target. The slowest runner, the Xeon 8573C at a projected 35.5 min, stays under it too.
- **New tests:** about 25 planted boards and scenarios, mostly tier 1. Predicted under 3 min of shard time in all.
