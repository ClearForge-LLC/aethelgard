# AET-WO-4411: FEEDBACK

A pre-certified 2.4 GHz radio module, an ESP32-C3-WROOM-02-N4 (U30), on Sentinel-03 (specification 31). It is on USART6 (TX PG14, RX PG9), with the antenna hung off the board's top-right corner. Since the 2026-11-25T21:17Z ruling it is fed by **a 3.3 V regulator of its own** (U13, LMR33620ADDA), from the battery. The measurement, the stops and every ruling are in `spike/4411/`:
- `STOP.md`, `RESULTS.md` and `FACTS.md`: §1.1.
- `out/lever-iv-sweep.md` and `out/lever-1-sweep.md`: the escape levers.
- `out/fallback-2.md`: a lower-threshold supervisor, on paper.
- `out/option-1.md`: the module on its own rail.
- `PREDICTIONS.md`: every prediction, committed before its run and scored.

## 1. What was built, against the rulings

**The module (§1.1, D1–D8)**
- **D1.** The ESP32-C3-WROOM-02-N4. Its -02U (u.FL) is named in the row (`u_fl_variant`) and not drawn.
- **§7 and D3.** Held by sha256 in `crates/aethelgard-ir/data/radio-parts.json`:
  - the datasheet v1.7;
  - the ESP32-C3 Hardware Design Guidelines (a rolling master build, held as retrieved 2026-11-24);
  - the product selector's JSON (D8).
- **D2.** The antenna overhangs the top edge at the top-right corner (HDG-C3 Fig. 19, position 3). The placer's margin exception covers the antenna region only (`Antenna::on_board`).
- **D4.** The footprint is our own (`Aethelgard_RF`), derived from KiCad's, with a keepout on each of the four copper layers. Pad 19's vias come from the engine's thermal array.
- **D5.** The row's 500 mA supply requirement is counted. The 345 mA typical transmit peak is recorded, never used as a bound. A `power.loads` row naming the module is refused (`LoadNamesCountedPart`).
- **D6.** Sentinel-03 itself. **D7.** A UART of its own, USART6 PG14/PG9 (ruled 03:03Z). **D8.** Active (the selector's live field).

**The supply (the 1 (a') ruling, lever (iv), lever (1), fallback (2), option 1)**
- With the module on `VDD_3V3`, U1's drop could not be held under the 110 mV that D6's supervisor arithmetic leaves. Measured:
  - lever (iv): 113.0 mV;
  - lever (1)'s directional escapes: 118.3 mV;
  - fallback (2): no TLV803 variant both clears the floor rule and allows the drop, because the W25Q64JV's 2.7 V binds R and Z.
- **Option 1 (ruled 2026-11-25T20:10Z, built to the 21:17Z ruling):** U13 = LMR33620ADDA from the battery, on `VDD_3V3_RF`.
  - **The datasheet** is TI SNVSAW1E, fetched 2026-11-25 and held at sha256 `865af73e…3daf2b`. Its row is in `power-parts.json`, and KiCad 10.0.7's `LMR33620ADDA` symbol is vendored.
  - **J4's fault current** becomes 3 x 4.0 A ISC max = 14,100 mA, under the XT30's 15 A.
  - **Loss:** 490 mW at 504 mA (Figure 9-8 at 78 %, read from the PDF's vectors).
  - **Inductor:** the held 8.2 µH meets SNVSAW1E's Equation 5 (L_MIN ≥ 0.36 · VOUT / fSW = 3.49 µH).
  - **The alternative, LMR33610ADDA** (SNVSBI9A, sha256 `777b3966…`): the same 4.0 A ISC and a half-size output bank, but KiCad ships no symbol for it.
- **(a) Placement.** U13 belongs to the module's group. It sits at the module's foot, inside the module's strip, turned so the pin its inductor serves faces the open board. The rule applies only to a regulator whose output is a module's supply, so no other board moves. `check drc --spec` passes all 31 loops.
- **(b) The pour ladder** (ruled 2026-11-26T00:22Z), described in §3.
- **(c) Pad 19.**
  - The array's pitch is 1.125 mm, 18 of the supply gate's 62.5 µm cells. Each via sits 12.5 µm per axis from the land pattern's gap crossing, inside OSH Park's ±50.8 µm.
  - A module-scoped 10 µm snap puts the pad's centre on the grid, so all four vias are read.
  - The module rail's return gate is now judged with no refusal left: pad 19 measures 18.0 mOhm.
- **(iii) Sequencing.** U11's `EN` follows U13's open-drain `PG` (a new `after` control, its pull-up R32 to `VDD_3V3_RF`), so U1's rail rises only after the module's.
- **New refusals, each planted (`crates/aethelgard-ir/tests/radio_rail.rs`):**
  - **`RadioRailUnsequenced`:** a module on its own rail that the MCU's regulator does not follow.
  - **`RadioLevelOverSupply`:** the MCU's rail at its highest more than VIH's VDD + 0.3 V over the module's rail at its lowest. Edge measured: R20 at 106.0 kOhm passes, 106.3 kOhm is refused.
  - **`RadioSupplyBelowMinimum`:** the module rail's guaranteed minimum, less its drop limit, under the module's 3.0 V. A 144 mV limit passes; 145 mV is refused.
  - **`DropLimitUndeclared` on the module's rail.**
  - **`ControlAfterInvalid`:** no regulator, itself, no open-drain PG, a PG already holding an enable, a loop, or a pin that is no enable. A PG also declared in its own `controls` is `PowerValueInvalid`.
- **The drop gate.** A limited domain the MCU has no pin on takes the radio module there as its load.
- **Step 25.** `radio_module_tx` is a new step-25 path class. Under (iii), `VDD_3V3_RF` leads `VDD_3V3` at every power-up, so U30's TX onto U1's `WIFI_RX` is judged at VDD = 0: the rail's declared maximum, 3465 mV, against DS12117 Table 10's 4.0 V (ruled 1216 D4). Planted over the limit at a 220 per mille tolerance (4026 mV, `OffDomainAboveLimit`).
- **The campaign.** A module on a rail counts as that rail's observed consumer (its port's RX open). A new scenario, `scenarios/sentinel-03/vdd3v3rf-out-of-band.json`, covers `VDD_3V3_RF`.

## 2. The built board (Sentinel-03, specification 31)

| Measure | Result |
|---|---|
| Outline | **85.65 x 98.75 mm**: Scotty's approved width; approved up to the declared 100 x 100 mm (2026-11-25T21:44Z) |
| U1's drop, `VDD_3V3` (limit 110 mV, 749 mA) | 82.6 mV. Worst VDD pin 30, 64.4 mOhm; worst VSS pin 107, 45.8 mOhm |
| The module rail's drop, `VDD_3V3_RF` (limit 144 mV, 504 mA) | 122.2 mV. Supply U30.1 216.1 mOhm; return U30.9 26.5, U30.19 18.0 mOhm |
| U10 / U11 / U13 copper, F.Cu / B.Cu / In1.Cu (need 1500 / 700 / 700 mm²) | 1590 / 2551 / 3062; 2061 / 2824 / 3311; 1115 / 1386 / 1489. All pass |
| Capacitance ladder | rung 3, bounded via cost 320, nearest EXT_SCL at 89,557 of 100,000 fF |
| Pour ladder | **step 1** (squares at 1.0 x each buck's need) |
| `extract` | verdict pass, no finding |
| Raw DRC | 36 `drill_out_of_range`: the three bucks' 0.254 mm thermal drills, as before with two |
| `check drc --spec` | pass: `drc.power` 31, `drc.radio` 1, `drc.outline` 1 |
| Battery input current | 1905 mA (1869 before); J4's fault 14,100 mA |

## 3. The pour ladder (b)

- **What it does.** After routing, `emit` meters `pour::joined` and holds each buck's `power::junction_need_cm2`, the θJA figure's smallest area at which its declared loss keeps the junction at 125 °C, on every joined layer.
  - **The steps:** step 0 is today's routing. Step k (`route::POUR_STEPS`, area ratios 1.0, 1.25, 1.5 and 1.75 of the need) prices a passing track at 40 a node on F.Cu within a square about each buck's thermal pad. A net with every pad within 6 mm of the buck's courtyard is the block's own and is not priced.
  - **The choice:** a rung fits only if it holds every capacitance bound and every buck's copper. The first step with a fitting rung settles, and the capacitance ladder chooses among its fitting rungs as before.
  - **The refusal:** when no step clears, `BuckPourShort` names each buck, layer and area at the nearest rung.
- **Why a ladder.** A single setting was a knife-edge. The sweep (`spike/4411/`, the 2026-11-26 bus table) found one point of nine passing all nine areas: every neighbour lost some buck's B.Cu as the rung's routing moved.
  - Pricing both outer layers made F.Cu the cheaper again inside the square, and the pad's ends closed (U11 measured 36 mm²).
  - Hard F.Cu exits did nothing with F-only pricing.
  - Both are therefore left out.
- **Sentinel-03:** step 0 left every rung with U11's F.Cu at 36 mm², because its own `U13_PG` net capped the package's ends. Step 1, rung 3 holds all nine areas.
- **A board with no buck runs step 0 alone,** today's ladder, and its emit names no pour step. Measured:
  - Sentinel-01 (sha256 `401b9e1e…`), Sentinel-01-minimal, Sentinel-02-bench, and the laddered `sentinel-01-ready` fixture are byte-identical against 3a35be0f's build.
  - Every other committed board is in §5.
- **Cost per step:** about 35 s per `emit` step in the debug build (eight rungs, each on its own thread). Sentinel-03 takes two steps, about 78 s. A board that no step clears pays five, about 3.5 min (`pour_ladder_tier1`'s refusal test).
  - Every test that emits Sentinel-03 pays the second step. `outline_tier1` took 201 s, and `supply_tier1` and `via_ring_tier1` about 10 minutes each, mostly their extract.
- **Ruled (1), 2026-11-26T01:31Z:** the step is in `emit`'s JSON (`pour_step`, `pour_square_permille`, `buck_copper` with each buck's need and area), beside the rung.

## 4. Predictions, scored (option 1 build, committed b069c134)

| Prediction | Measured | Score |
|---|---|---|
| Outline inside 100 x 100, within about 3 mm of board c's 97.65 x 99.6 | 85.65 x 98.75 | inside: right; the width 12 mm narrower than predicted |
| U1's drop about 80 mV (75-90) | 82.6 mV | right |
| Module rail: supply at most about 40 mOhm, drop at most about 40 mV | 216.1 mOhm, 122.2 mV | **wrong**: U13 sits at the module's foot, about 36 mm from pin 1 |
| Pad 19 bounded, at most pin 9's | 18.0 mOhm against pin 9's 26.5 | right |
| U10, U11, U13 at least their need | all pass, at the ladder's step 1 | right, after (b) became a ladder |
| U13's junction at most 106.2 °C | 490 mW at 36.0 °C/W (Figure 9-3's 10 cm² point, for 11.15 cm² least, F.Cu): about 102.6 °C | right |
| J4's fault 14,100 mA | 14,100 mA | right |
| Sentinel-01 and every board without a module byte for byte as before | §5 | right |

## 5. INV-08

Every committed specification but Sentinel-03 was emitted by 3a35be0f's build (this WO before option 1) and by 7cde17de's, and the outputs compared byte for byte. That is 42 specifications: `examples/`, `crates/aethelgard-cli/tests/fixtures/` and `crates/aethelgard-ir/tests/fixtures/`.
- **26 emit.** 25 are byte-identical: schematic, BOM and board. The one that moves is `tests/fixtures/sentinel-03-radio.json`, which carries the module (its pad 19's pitch and snap, and the pour ladder). INV-08 does not cover a board with a module.
- **16 are refused by both builds** with the same kind. Ten differ in their message only, because each is `UnsupportedSchemaVersion` on an old-version fixture and lists version 31's rules: `RadioSupplyNotMcu` is gone, and `RadioRailUnsequenced`, `RadioLevelOverSupply` and `RadioSupplyBelowMinimum` are added.
- **Against main (1cc2d01c), before this WO:** measured at the version bump, 41 committed specifications moved by their version line only.

## 6. Mutants

This stage ran 28 mutants of its own guards, one each, `crates/` only, each against the suite it lives in (scratch harness: patch, run, restore and touch). **27 were killed.**

| Mutants | Guards | Result |
|---|---|---|
| B01-B05 | `radio_rails`: sequenced, level, floor, the module rail's limit, the range left to step 27 | killed |
| B06-B12, B27 | `after`: exactly one, no regulator, itself, open-drain PG, one enable per PG, a loop, on an enable, a loop through a supply | killed |
| B13 | step 25's `radio_module_tx` | killed |
| B14 | `junction_need_cm2` | killed |
| B15 | the campaign's radio-module consumer | killed |
| B16-B21, B28 | the radio snap, the regulator's turn, its place in the group, the band below, the module's foot, the module owning it, no owner on the MCU's rail | killed |
| B22-B24 | the ladder: a rung must hold the pour, `BuckPourShort`, step 0 prices nothing | killed |
| B26 | the drop gate's load on the module's rail | killed |
| **B25** | the block-local exemption in the pour pricing | **survived** |

- **Why B25 survived:** without the exemption, Sentinel-03 settles at the same step and rung (1, 3, nearest EXT_SCL at 89,557 fF) but with different board bytes (sha256 `e46b9d1a…` against `9b6cfc4e…`). No test can tell the two apart.
- **Why it is kept:** it is the pricing's design, so a block's own SW, FB, VCC and BOOT tracks are never pushed off F.Cu. It is named here.
- **Not planted:** the ladder's treatment of a buck with no measured layer (adversarial F6). No board reaches that case.
- **Earlier phases:** 40 mutants of the radio rules and placement, 39 killed. M23 (references distinct) survived, named in §9.

## 7. Adversarial pass

The pass ran as a fresh read-only review of 3a35be0f..7cde17de, with the WO's §5 questions.

**Fixed in bd957c84:**
- **S1:** `VERIFY_S03` in `ci.yml` did not list `vdd3v3rf-out-of-band.json`. Sentinel-03's committed package no longer binds the specification, which is the ruled path: the architect's write dispatch.
- **F1:** the `after` loop check followed `after` edges only. A lead whose input is downstream of its follower deadlocked power-up unrefused. The check now follows supplies too (planted).
- **F3:** off-board ends carrying `VDD_3V3` (J2, J3) can power U1 while the module's rail is down. The power-down condition now names them.
- **F4:** a module on the MCU's own rail made the MCU's regulator the module's, pulling U11's block into the corner. That moved the `sentinel-03-radio` fixture. Fixed and planted.
- **F5:** `after` pull-ups were missing from every-part's checks (temperature and lifecycle). The PG low level against EN's threshold is now named in `control_after`.
- **F6:** a buck with no measured layer counted as fitting. It is now short at 0.
- **F12:** the loop conditions cited SNVSAN3F only. SNVSAW1E's items are now cited where an LMR33620 is.

**Sent for a ruling:**
- **F2:** VIH is compared at the regulators, not at U30's pins. At the pins U1's high can sit 416.6 mV over the module's VDD (144 mV drop limit), against +300 mV.

**Named, not changed:**
- **F7:** the local-net exemption is one flag per net, not per buck.
- **F8:** the feed-edge headroom is now 0-10 µm, since the 10 µm snap can leave up to 240 µm.
- **F9:** an unbounded board with a module has no keep-in check.
- **F10:** the rail's drop limit leaves no margin for a TX burst's dip, and under (iii) a dip that trips U13's PG turns U11 off.
- **F11:** `off_domain` does not read `after` as ordering, which errs toward refusal.
- **Ladder cost and errors:** an `extract` refusal at a later step aborts; `CapacitanceOverBound` reports step 0's figures.
- **Coverage:** size sample 6 left the removal sweep.
- **Hot-plug:** the fuse margin has narrowed.

## 8. Reduction pass

Reduction pass (diff): about −30 lines net of the doc added · 5 duplicates removed · 2 dead paths removed · 0 protected kept · 4 findings below. A fresh read-only pass over fa12abf4..bd957c84 proposed the changes; applied in 043162cb:
- **Duplicates removed:**
  - `shape()`'s hand-built reference list now reads `power::references` (same order, so the first duplicate a check names is unchanged);
  - `names::pg_net` for the three spellings of `<reg>_PG`;
  - `loops::from_snvsaw1e` for the two datasheet predicates;
  - one θJA clearing search (`clearing`) for step 9's screen and `junction_need_cm2`;
  - one Sentinel-03 loader for the power tests.
- **Dead paths removed:**
  - `radio_corner`'s fallback snap when the ground pad is missing: it can't be missing, so it is now a refusal;
  - the router's step-0 default under its ratio: now an `if let`.
- **Mechanism and copies merged:** the radio group's three hand-rolled anchor predicates now use `anchored_to`; the six SNVSAN3F `*_RULE` consts are inlined into their one use; one snap closure; the radio table read once.
- **Added with the pass, for the gap it found:** an `after` on a crossing's control is refused.
- **Findings, not acted on:**
  - 4300k's MinusY-to-180 turn mapping is written twice (`items`, `place_connectors`);
  - the joined doc comments above `rank_apart`;
  - the rotation list `[0, 90, 180, 270]` three times (taste);
  - `laddered`'s `short` evaluated twice per rung.

## 9. Rows and follow-ups

- **The rung and the pour step reach neither the verdict nor FAB-NOTES** (ruled 2026-11-26T01:31Z, a row). Today only `emit`'s JSON carries them. A later work order carries both through `extract`.
- **Q-SUP on the new rail (a row):** no Espressif document states a brown-out detector. The HDG's monitor, "around 3.0 V" (PDF p. 10), is not fitted, so R90/C90 is U30's only reset (`radio_rail_monitor:U30`, acknowledged).
- **Power-down order (a named condition):** U1's TX may drive U30's RXD above VDD + 0.3 V while `VDD_3V3_RF` falls first (`radio_power_down_order:U30`). An Ioff buffer on that line is a row (`radio_rail_ioff:U30`).
- **U11 follows U13's PG** (`control_after:U11`): U1's rail drops whenever U13's PG pulls low, for example on an out-of-window output, current limit or thermal shutdown (SNVSAW1E p. 11).
- **The module rail's supply bound is 216 mOhm,** U13 being about 36 mm from pin 1. That passes at 122.2 of 144 mV. A rule that holds the block nearer pin 1 is a refinement, not needed today.
- **Hot-plug margin:** with U13's 20 µF more input ceramics, F1's hot-plug I²t is 3.06 against its 3.25 A²s melting figure. At 4.75 V cells it is over, and `readiness_4300k`'s doubled-pack edge test now judges against a table whose fuse melts later.
- **The battery sag edge** moves to 824/825 mOhm (6,996,978 µV at 825, measured).
- **Size sample 6** (`outline_tier1`) no longer places inside 100 x 100 (least-over 98.100 x 102.500 mm). The removal sweep keeps seven variants.
- **Fixed in passing:** two LMR33630-only texts that a second buck part exposed.
  - The inductor condition's hard-coded "Table 9-2 gives 8 uH at 5 V and 6.8 uH at 3.3 V".
  - The loop rules' SNVSAN3F citations, now per datasheet.
- **Earlier rows, carried:**
  - The supervisor floor rule covers I2C and SPI devices only (Q-SUP (a)).
  - L7: the corner group can pull the outline's left edge past left-side connectors, which can't be tested on Sentinel-03.
  - The tests' `sentinel-03-radio.json` fixture duplicates the example.
  - The inner-keepout fill check lacks 4300k's pour exemption.
  - D8's lifecycle gate reads the row, not the held JSON.
  - M23 survived.
- **The Sentinel-01 bench package** is bound to the version-30 specification. CI's write dispatch re-binds it (`ci.yml`: `--rebind fab/sentinel-01-bench/package`) beside Sentinel-03's package.

## 10. Gates

PENDING: fmt, clippy, `cargo test --workspace`, the CI guards, tier 2.
