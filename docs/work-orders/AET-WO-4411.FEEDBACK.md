# AET-WO-4411: FEEDBACK

A pre-certified 2.4 GHz radio module, an ESP32-C3-WROOM-02-N4 (U30), on Sentinel-03 (specification 31). It is on USART6 (TX PG14, RX PG9), with the antenna hung off the board's top-right corner. Since the 2026-11-25T21:17Z ruling it is fed by **a 3.3 V regulator of its own** (U13, LMR33620ADDA), from the battery. Since (c1) (2026-11-26T04:03Z) its UART runs through **U31, a TXU0304PW** between the two rails. The measurement, the stops and every ruling are in `spike/4411/`:
- `STOP.md`, `RESULTS.md` and `FACTS.md`: §1.1.
- `out/lever-iv-sweep.md` and `out/lever-1-sweep.md`: the escape levers.
- `out/fallback-2.md`: a lower-threshold supervisor, on paper.
- `out/option-1.md`: the module on its own rail.
- `out/option-1e.md`: F2's lever (e), on paper, which has no window.
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
  - **`RadioLevelOverSupply`**, compared at the regulators: replaced in (c1) by `LineLevelUnmet`, at the pins (below).
  - **`RadioSupplyBelowMinimum`:** the module rail's guaranteed minimum, less its drop limit, under the module's 3.0 V. A 144 mV limit passes; 145 mV is refused.
  - **`DropLimitUndeclared` on the module's rail.**
  - **`ControlAfterInvalid`:** no regulator, itself, no open-drain PG, a PG already holding an enable, a loop, or a pin that is no enable. A PG also declared in its own `controls` is `PowerValueInvalid`.
- **The drop gate.** A limited domain the MCU has no pin on takes the radio module there as its load.
- **Step 25.** `radio_module_tx` is a new step-25 path class. Under (iii), `VDD_3V3_RF` leads `VDD_3V3` at every power-up, so a module driving U1's RX directly is judged at VDD = 0: the rail's declared maximum, 3465 mV, against DS12117 Table 10's 4.0 V (ruled 1216 D4). Planted over the limit at a 220 per mille tolerance (4026 mV, `OffDomainAboveLimit`).
  - Since (c1), Sentinel-03 has no such path (below). The class judges a module wired directly.
- **The campaign.** A module on a rail counts as that rail's observed consumer (its port's RX open). A new scenario, `scenarios/sentinel-03/vdd3v3rf-out-of-band.json`, covers `VDD_3V3_RF`.

**F2 and (c1) (ruled 2026-11-26T03:22Z, 04:03Z, 04:51Z and 06:25Z)**
- **F2 was real:** at U30's pins, U1's high could stand 116.5 mV over the module's VIH maximum (VDD + 0.3 V).
  - (d) was refused.
  - **(e), raising U13 by its divider, has no window** (`spike/4411/out/option-1e.md`, 081a463b): VIH maximum needs U13's minimum at least 3,261.2 mV, and VIH minimum (0.75 x VDD against U1's VDD − 0.4 V) needs its maximum at most 3,512.9 mV. That is 251.7 mV of room against a band of about 285 mV.
- **(c1), built: U31, a TXU0304PW,** with VCCA = `VDD_3V3` and VCCB = `VDD_3V3_RF`.
  - A1 hears `WIFI_TX`, and B1Y drives U30's RXD on `U31_B1Y`. B4 hears U30's TXD on `U31_B4`, and A4Y drives `WIFI_RX`.
  - **OE is strapped to VCCA** (ruled 04:51Z). Under (iii), `VDD_3V3` is the rail that rises last, so OE is low through power-up, as SCES935A 9.1 asks ("left floating or externally pulled down … during power up or power down"). 9.3.5's isolation holds the outputs while either supply is under 100 mV.
    - The first bus note said VCCB. The adversarial pass showed that VCCB holds OE high while VCCA ramps from 0, against 9.1, so it moved to VCCA.
    - A wired OE is refused (`PowerValueInvalid`): a wire's driver and level through partial power-down are not judged.
  - **A2 and A3 are tied to `GND`** (9.3.1.1's pull-down rule made explicit; ruled: no floating input). B2Y and B3Y are left open as outputs, and NC 6 and 9 have no internal connection (p. 4, Table 6-1).
  - **Bypasses:** C93 on VCCA and C94 on VCCB, 0.1 µF each (p. 25, 12.1).
  - **Loads:** 1 mA rows on both rails (ICCA and ICCB 6 µA max, rounded up). U11's loss is restated at 750 mA (82.81 % read) and U13's at 505 mA (79.09 % read). Both losses hold: 563 and 487 mW, under the declared 570 and 490.
- **The row (`power-parts.json`):** the translator's timing moved into `supplies`, one row per supply pair: the VCCB = 1.8 V row as before, and the VCCB = 3.3 V row (p. 14, 7.11: tpd 11 ns each way). A crossing reads the first row whose bands hold both its domains. Added beside it:
  - **`levels`:** VOH ≥ VCCO − 0.1 V and VOL ≤ 0.1 V at |IO| ≤ 0.1 mA (p. 8); VT+ 640 and VT− 296 per mille of VCCI; II 2 µA; VI 5.5 V.
  - **`unused_inputs_at`:** the citation for the rule that ties every input a crossing does not carry to `GND`.
  - **The MCU row** gains Table 50's CMOS output (VDD − 0.4 V, 0.4 V, at 8 mA).
  - **The radio row** gains Table 6-3's VIH, VIL, VOH and VOL.
- **Two SCES935A gaps (ruled 04:51Z):**
  - **G1 (a), an inference.** VT+ and VT− are stated at VCCI = 3, 4.5 and 5.5 V, not 3.3 V (p. 7). The 3 V row is taken as a fraction of VCCI: 640 and 296 per mille, which bounds every stated row above it.
    - It is named in the verdict (`translator_threshold_inferred:U31`) with its bench row: VT+ and VT− measured at the board's rails at bring-up (`RUL-20` (3); the ruling's `RUL-18` was corrected at 06:25Z).
    - (b), the 4.5 V row's 2.74 V, is a valid bound but about 550 mV loose. It would refuse the board and was not taken.
  - **G2, a screen.** No TMAX is stated for 3.0-3.6 V to 3.0-3.6 V (p. 9). The port's 57,600 bit/s is judged against 10 Mbit/s, the lowest row stated from a 3.0-3.6 V input (`translator_rate_inferred:U31`). tpd is 11 ns as stated.
  - **S1 (ruled): UART only.** The 3.3 V row is `uart_only`, so an SPI crossing still reads the 1.8 V row. An SPI crossing between two 3.3 V rails is refused as before, on VCCB's band (tested).
- **The rule, `LineLevelUnmet`** (it replaces `RadioLevelOverSupply`, `crates/aethelgard-ir/src/levels.rs`):
  - each line's driver VOH and VOL at its own pin are judged against the receiver's VIH, VIL and maximum input at its own pin, and the receiver's input current against the load the driver's levels are stated at;
  - two pins on one rail share one source voltage, less each pin's drop limit, so the band is not counted twice;
  - two rails are judged at each band's ends;
  - it runs for every module, directly or through a UART crossing, so F2's lesson holds inside the rule.
- **Sentinel-03's four lines** (worst margins, mV):

| Line | Driver to receiver | High | Low | Under the receiver's maximum |
|---|---|---|---|---|
| `WIFI_TX` | U1 PG14 to U31 A1 | 622.1 | 498.3 | 2,082.8 |
| `U31_B1Y` | U31 B1Y to U30 RXD | 542.2 | 650.2 | 156.0 |
| `U31_B4` | U30 TXD to U31 B4 | 388.0 | 573.7 | 2,082.8 |
| `WIFI_RX` | U31 A4Y to U1 PG9 | 733.4 | 810.4 | (Table 10, step 25) |

- **Rows this closes, by name:**
  - `radio_power_down_order:U30` and `radio_rail_ioff:U30` no longer fire once a crossing carries the port, and their acknowledgements are removed.
  - A condition names them as closed by U31: each U31 output is powered from its receiver's own rail, and its Ioff rows hold either side at 0 V (p. 8; 9.3.4).
  - **Step 25:** the module's TX now ends at U31's B4 on its own rail, and A4Y drives `WIFI_RX` from `VDD_3V3`. So no path reaches the MCU from another rail, and `radio_module_tx` is no longer a path on Sentinel-03. Without U31 it is judged as before (planted).
- **Rows it names, acknowledged:**
  - `translator_supply_drop:U31`: U31's pins are taken at each rail's drop limit, and the gate measures the rails at their loads.
  - `uart_internal_pulls:U31`: the 0.1 mA rows need both receivers' internal pull-ups off. U1's PG9 is written `FLOATING` by the generated hwdef. U30's RXD pull-up is its firmware's, not judged.
  - `radio_output_load:U30`: Table 6-3 states VOH and VOL at a high-impedance load, and U31's B4 draws 2 µA.
  - `translator_partial_power:U31`: 9.3.5 states isolation under 100 mV only. Between 100 mV and 1.08 V it states nothing, and it recommends inputs low before a supply floats, while a UART idles high. While U31 isolates, `WIFI_RX` and `U31_B1Y` are driven by nothing.
  - `translator_threshold_inferred:U31` and `translator_rate_inferred:U31` (G1, G2).
  - `rated_only:U31.bypass_a` and `rated_only:U31.bypass_b`.
  - A UART crossing named by no load on a rail is screened `translator_supply_not_counted`, as an SPI crossing always is.
- **(iii) is kept as ruled.** `RadioRailUnsequenced`'s reason now reads, with a crossing, as the ruled order rather than the RXD drive the crossing removed.

## 2. The built board (Sentinel-03, specification 31)

| Measure | Result |
|---|---|
| Outline | **85.00 x 99.75 mm** since (c1) (85.65 x 98.75 before U31); approved up to the declared 100 x 100 mm (2026-11-25T21:44Z) |
| U1's drop, `VDD_3V3` (limit 110 mV, 750 mA) | 85.7 mV (82.6 before U31). Worst VDD pin 33, 49.0 mOhm; worst VSS pin 31, 65.3 mOhm |
| The module rail's drop, `VDD_3V3_RF` (limit 144 mV, 505 mA) | 122.7 mV. Supply U30.1 216.2 mOhm; return U30.9 26.8, U30.19 18.1 mOhm |
| U10 / U11 / U13 copper, F.Cu / B.Cu / In1.Cu (need 1500 / 700 / 700 mm²) | 2082 / 2750 / 3382; 1310 / 2354 / 2763; 1326 / 1578 / 1649. All pass |
| Capacitance ladder | rung 7, bounded via cost 320, bounded nets first, nearest EXT_SDA at 84,703 of 100,000 fF |
| Pour ladder | **step 1** (squares at 1.0 x each buck's need) |
| `extract` | verdict pass, no finding |
| `check drc --spec` (tier 2) | pass. `drc.power` judges 33 loop rules: the 31 before, plus C93 and C94 at U31's VCCA and VCCB pins (`radio_tier1`'s `the_crossings_bypasses_are_judged_loops`). `drc.radio` checks U30's pads on `U31_B4` and `U31_B1Y`. The raw DRC is as `board_tier2` holds it |
| Tier 2 (`board_tier2`, in the image) | Sentinel-03 passes every gate inside its bound (ERC, DRC, `check drc --spec`, extract), and the radio fixture and its planted boards pass (850 s) |
| Battery input current | 1906 mA (1869 before the module); J4's fault 14,100 mA |
| UART lines (four, through U31) | high 388.0-733.4 mV, low 498.3-810.4 mV; under U30's VDD + 0.3 V by 156.0 mV |

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
- **(c1), re-measured on the working tree against 3a35be0f's outputs:** the same 25 are byte-identical, and the 16 refusals keep their kinds. Their messages differ in the version-31 rules list only (`LineLevelUnmet`).
  - `sentinel-03-radio` (a module, no U31) is byte-identical against 081a463b's build, so (c1) moves no board but Sentinel-03.
  - **SPI crossings:** `sentinel-02-power`'s board is byte-identical. Its `solve` against 3a35be0f differs only in condition texts this WO changed before (c1) (the inductor wording and the drop-limit note).

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
- **(c1): 32 mutants of its own guards, one each, all killed** (the same harness, `crates/` only). Each was run against the suite the guard lives in: `radio_rail` for the IR's, and `radio_tier1` for the emitter's and the loops'.
  - **The level judge (C01-C05):** high, low, the receiver's maximum, the stated load, and the one-rail corner model.
  - **The crossing's shape (C06-C11):** no module on the port, twice on a port, no B-to-A channel, the rails, TMAX, and the levels' supply window.
  - **The off-domain gate (C12)** and the closed rows (C13).
  - **The supply-row selection (C14)** and its UART-only filter (C31).
  - **The emitter (C15, C16, C18):** A2/A3 tied to GND, the module on the B-side nets, and `own_nets`.
  - **The screens and refusals (C20-C30, C32, C33):**
    - the screens: the firmware's pulls, U30's load, U31's drop, partial power-down, the inferred thresholds and TMAX;
    - the uncounted translator, the `after` and wired-OE refusals, the loops' LINES condition, and the readers' order and empty-supplies checks.
  - C01 was re-run by hand to confirm its kill, by `each_line_level_through_the_translator_is_judged_at_its_receivers_pin`.
  - **`drc.radio` on the crossing's nets (C17):** killed in the image by `board_tier2`'s Sentinel-03 test (`RadioUartMiswired`, U30 pad 12 on `U31_B4`).
  - **C12 and C16 were re-run after the reduction** moved their code into `uart_b_nets` and `module_uart_nets`. Both are killed.
  - **C19** (`RadioRailUnsequenced`'s reason, which reads differently with a crossing) is wording, not a guard, so it was not planted.

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

**(c1), a second fresh read-only pass** over the (c1) working tree against 081a463b. No blockers. Six should-fixes:
- **Fixed:**
  - **(1)** An OE wire could float, and step 25's control-wire owners missed UART crossings. Now a UART crossing's OE takes a strap only (planted).
  - **(2)** OE on VCCB was high while VCCA ramps, against 9.1. Moved to VCCA (ruled).
  - **(3)** The two closed rows were dropped silently, and 9.3.5's residuals lost with them. Now a condition names them closed, and `translator_partial_power:U31` holds the residuals.
  - **(6)** `translator_supply_not_counted` scanned SPI crossings only. A UART crossing no load names is now screened (planted).
  - **(7)** The radio row's UART levels were not checked in order. Now they are (`RadioTableMalformed`, planted).
  - **(9)** U31's load source said 250 nA for U1's RX. It is 350 nA, the row's.
- **The close-out review found one more, now fixed.** "PUPDR is 0" had not been checked. ArduPilot's `chibios_hwdef.py` gives every UART TX and RX pin PULLUP unless its hwdef line names another pull. Aethelgard's hwdef writes every serial-port pin `PUSHPULL FLOATING` (`aethelgard-hwdef`'s UART lines). Sentinel-03's generated hwdef has `PG9 USART6_RX USART6 SPEED_VERYLOW PUSHPULL FLOATING`, so U1's side is a generated fact.
  - `uart_internal_pulls:U31` now names only U30's RXD pull-up: the module's firmware, not judged.
  - Its earlier "over 0.1 mA at 3.0 V" was wrong (3.0 V over 30 kΩ is exactly 0.1 mA), and it is gone.
- **Sent for a ruling:**
  - **(4)** The 3.3 V row would have admitted a 3.3 V SPI crossing. Ruled S1: UART only, now `uart_only`.
  - **(5)** The drop limit is taken wholly on the VDD side. Ruled S2, a row (§9).
- **Named, not changed:**
  - **(8)** Table 50's VOH row holds for 2.7 V ≤ VDD; U31's levels window keeps every judged pin at 3.0 V or more.
  - **(10)** `line_supply`'s fallback to a domain's declared range is reachable (a connector-sourced domain has no regulator), so it is kept.
- **The pass checked and found sound:** the rounding directions, the corner model (linear levels, independent drops), every U31 pin against the pinout, the same-rail module fixture by hand (smallest margins 190 and 226 mV), the off-domain gate, and every new citation against its page.

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
- **(c1), diff-sized:** a fresh read-only pass over 081a463b..HEAD (crates only, main's merge excluded). Reduction pass (diff): 4 duplicates removed · 1 dead path removed · 0 protected kept · 4 findings below.
  - **Removed while writing (c1), before the pass:** the UART drawer's copy of the SPI drawer. The two now share `sides` (supplies, ground, controls), `numbers` (pin numbers by name) and `bypasses`, in the SPI drawer's call order, and `sentinel-02-power`'s board is byte-identical. (No line count: the unshared drawer was never committed.)
  - **Duplicates removed:**
    - the module's UART nets: one `power::module_uart_nets` for the emitter and `drc.radio`;
    - the B-side net names: one `b_nets`, behind `uart_b_nets`, `own_nets` and the line judge;
    - the off-domain gate's own scan for a crossing: now `uart_b_nets`;
    - the translator's power-row reader: one `Supplies` list for both crossings.
  - **Dead path removed:** `radio_lines`' refusal for a crossing with no two channels, which `uart_crossings_shape` refuses first.
  - **Field put to use:** `Translator::unused_inputs_at` was read nowhere. It now cites the GND tie in a condition.
  - **Findings, not acted on:**
    - `radio_lines` re-reads the translator row that the shape pass judged;
    - `levels::corners` could be an array;
    - `UartCrossing` and `Crossing` copy six fields into `TranslatorUse`. Merging them changes the input's JSON shape;
    - the UART crossing's value checks repeat the SPI loop's common ones. Merging them reorders an SPI crossing's first refusal.
- **Findings, not acted on:**
  - 4300k's MinusY-to-180 turn mapping is written twice (`items`, `place_connectors`);
  - the joined doc comments above `rank_apart`;
  - the rotation list `[0, 90, 180, 270]` three times (taste);
  - `laddered`'s `short` evaluated twice per rung.

## 9. Rows and follow-ups

- **The rung and the pour step reach neither the verdict nor FAB-NOTES** (ruled 2026-11-26T01:31Z, a row). Today only `emit`'s JSON carries them. A later work order carries both through `extract`.
- **Q-SUP on the new rail (a row):** no Espressif document states a brown-out detector. The HDG's monitor, "around 3.0 V" (PDF p. 10), is not fitted, so R90/C90 is U30's only reset (`radio_rail_monitor:U30`, acknowledged).
- **Power-down order and the Ioff buffer:** closed by U31 ((c1), §1). Its residual, 9.3.5's window between 100 mV and 1.08 V and the undriven idle, is `translator_partial_power:U31`.
- **G1's bench row (`RUL-20` (3)):** U31's VT+ and VT− measured at the board's rails at bring-up (`translator_threshold_inferred:U31`).
- **S2 (ruled 04:51Z, a row):** split each drop limit into its VDD and VSS shares. The level judge takes the whole limit on the VDD side, which leaves a VSS rise unmodelled, worth at most about 36 mV on U30's high check. Every Sentinel-03 margin is 156 mV or more.
- **Q-R (ruled 06:25Z (a), a row): the placer is not monotone at the outline bound.** Removing a test point stops size 1 fitting 100 x 100 on all nine seeds: least-over 88.450 x 100.100 mm without Q_TP14, and 88.950 x 100.250 without Q_TP28.
  - The two are pinned exactly in `outline_tier1`'s removal sweep, so a third still fails.
  - A refused placement writes no board, so which part moves is not seen without a build. A placer work order takes it.
- **The `sentinel-03-radio` fixture** (the module on U1's own rail, wired directly) now produces `radio_output_load:U30`, which its acknowledgements do not name. No test packages it; a package of it would need the entry.
- **Size sample 2** no longer places whole (least-over 89.900 x 100.350 mm) and leaves the sweep's variants, by the size-6 precedent. Size 6 is now 100.600 x 103.100, and size 8 104.850 x 105.500.
- **U11 follows U13's PG** (`control_after:U11`): U1's rail drops whenever U13's PG pulls low, for example on an out-of-window output, current limit or thermal shutdown (SNVSAW1E p. 11).
- **The module rail's supply bound is 216 mOhm,** U13 being about 36 mm from pin 1. That passes at 122.2 of 144 mV. A rule that holds the block nearer pin 1 is a refinement, not needed today.
- **Hot-plug margin:** with U13's 20 µF more input ceramics, F1's hot-plug I²t is 3.06 against its 3.25 A²s melting figure. At 4.75 V cells it is over, and `readiness_4300k`'s doubled-pack edge test now judges against a table whose fuse melts later.
- **The battery sag edge** moves to 823/824 mOhm (6,999,388 µV at 824, measured; 824/825 before U31's two 1 mA rows). The twin campaign's sag point is 7,928,264 µV (it was 7,928,702).
- **Size sample 6** (`outline_tier1`) no longer places inside 100 x 100 (98.100 x 102.500 mm before (c1)). With size 2 also out, the removal sweep keeps six variants.
- **`supply_tier1`'s ring-0 plant** now emits Sentinel-03 without U31, as it placed before (c1). With U31 at a ring of 0, no pour step keeps U10's B.Cu at its need (341 of 1500 mm²), so the board is refused before the gate the plant reaches.
  - The plant's module rail declares 20 mV, since without U31 the direct lines pass only under 27.5 mV.
  - s024l was tried as a carrier, and a ring of 0 trips nothing there.
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

On the final head (the (c1) commit 8057a3da, main 649e81a8 merged as bd42ea94, then the close-out fixes and reduction):
- `cargo fmt --all -- --check`: pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- **`cargo test --workspace`: 3261 passed, 1 failed.** The failure is `bench_board_tier1`'s `the_committed_package_is_bound_to_its_specification_scenarios_and_emit`: Sentinel-01's bench package is bound to specification 30, and the write dispatch re-binds it.
- **The CI guards:** `ci/test-shard.sh`, `ci/test-route.sh`, `ci/route.sh check ci/docs-tests.tsv` and `ci/test-ceiling.sh` pass.
- **Tier 2,** in the pinned image under `capped-run` (16 GiB, swap 0), on the final code:
  - `board_tier2`: Sentinel-03 passes every gate inside its bound, and the radio fixture and its planted boards pass (840 s).
  - `twin_tier2`: Sentinel-03's battery-sag campaign meets every expectation, replays and repeats, with the sag point at 7,928,264 µV (388 s).
- **Mutants:** 32 of 32 (c1) mutants killed, §6.
- **INV-08 and identities, with the final binary:**
  - the 25 boards are byte-identical against 3a35be0f's outputs, and the 16 refusals keep their kinds;
  - `sentinel-03-radio` is byte-identical against 081a463b;
  - Sentinel-03's board is the same bytes as the board §2 measured.
