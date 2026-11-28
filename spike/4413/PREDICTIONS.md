# AET-WO-4413 part (1): predictions, committed before any probe

Written 2026-11-27, on `wo/aet-4413-s03-rf-reset-ground` at `86a1f280` (4310's READY head). Sources:
- two read-only surveys (`~/.cache/aet-4413/survey-reset-rail.md`, `survey-ground.md`), each spot-checked at source;
- the committed `spike/4310/out/s03-extract-3c.json`;
- three held datasheets: TLV803 SBVS157E; TPS3839 SBVS193D and TPS3840 SNVSB03E, newly held under `main`'s held-copy rule (`~/.cache/aet-held/`, each with a `SOURCE.txt`).

Each prediction is scored in `RESULTS.md` after the probes. None is edited here after this commit.

## P1. The reset

- **P1.1** TLV803S cannot be the radio rail's monitor: VIT− 2.87 / 2.93 / 2.99 V (SBVS157E p. 6, §7.5), so its minimum 2,870 < U30's 3,000 mV. No other TLV803 variant fits: Z 2.20, R 2.58 and M 4.28 V minimum, and M is above U30's 3.6 V maximum.
- **P1.2** TPS3840 cannot. Its 3.1 V option's minimum is 3,053.5 mV (−1.5 %, SNVSB03E p. 6), but its guaranteed hysteresis of 175–225 mV puts the release as high as 3,371.5 mV. A rail released there needs a solved minimum of at least 3,371.5 mV plus the drop, so a solved maximum above U30's 3,600 mV.
- **P1.3** **TPS3839G33 fits.**
  - VIT− 3.003 / 3.08 / 3.126 V at −40 to 85 °C (SBVS193D p. 7, §7.5); the board is declared −20 to 85 °C.
  - Vhys 31 mV, typical only, so the release (at most 3,157 mV) is a screen, as TLV803S's is on the MCU rail.
  - RESET is push-pull, active low (p. 4); the part is TPS3839G33DBZR, SOT-23.
  - Its minimum 3,003 ≥ U30's 3,000 mV, so the module is held whenever its rail is under its range.
- **P1.4** The IR cannot express it today. `Mcu.reset_supervisor` is one `Option`, and `supervisor::judged` holds only `spec.mcu.supply`. U30's `EN` is `RadioModule.enable {resistor, capacitor}`, with no driver.
  - The smallest change is an IR field: a rail monitor, its rail, its part and the enables it drives.
  - `judged` then counts a rail held when such a monitor drives every part on it it is credited for.
  - Specification version 31 → 32.
- **P1.5** U31.B is held by the same monitor only if its `OE` is driven by the monitor's RESET; strapped to VCCA, as now, it stays named. Lean: drive `OE` from it.

## P2. The rail

- **P2.1** At the present feedback (105k / 46.4k) the rail solves to 3,144.727–3,417.204 mV (the repo's formula, reproduced by the survey). The minimum at U30's pin, 3,000.7 mV, is under TPS3839G33's 3,157 mV release, so the module would be held in reset under load.
- **P2.2** **100k / 41.2k (E96, TNPW0603 B) solves to 3,303.185–3,589.075 mV.**
  - That is under U30's 3,600 mV absolute and recommended maximum (v1.7 p. 21, Tables 6-1 and 6-2), and under U31.B's 5.5 V operating maximum (SCES935A p. 6, §7.3).
  - The domain's declared 3,300 mV ± 5 % (3,135–3,465 mV) must change to contain it. The domain declaration moves, nothing else.
- **P2.3** VDD_3V3_RF's drop limit is re-derived as the solved minimum less the monitor's release: 3,303.185 − 3,157 = 146 mV.
  - That is the module's run requirement, not its safety: the monitor holds it below 3.0 V whatever the drop.
  - The monitor is placed at U30, and senses the rail at U30's pin (the 4310 F5 point).

## P3. The ground

- **P3.1** The specification cannot express the split today.
  - `GND` is one fixed net (IR `lib.rs:166`), the return net in `supply.rs` is the literal `"GND"` (`supply.rs:1676`), and a net-tie footprint is refused (RUL-03).
  - The split needs a second ground net with a single join, its regions on In1 and the outer layers, and the gate crediting a current whose source and sink are both past the single join.
  - That last item is a change to `4310`'s rule, which this work order's fence excludes. The rest is an estimated 900–1,850 lines, an engine work order.
- **P3.2** Drawn without the gate change, the split is worse: VDD_3V3 is about 334 mV, because a current with no `GND` entry pad is charged at R₁₁.
- **P3.3** Split, with the gate change, and nothing else:
  - VDD_3V3 = 85.7 + 22.3 + 10.0 = 118.0 mV against 110, which fails.
  - VDD_3V3_RF = 122.7 + 20.1 + 9.4 = 152.2 mV against 146, which fails.
  - U11's and U13's input currents stay: they enter at the gates' own return pins.
- **P3.4** **The one lever the spec has today, `net_classes`.** VDD_3V3_RF is routed at 500 µm (class `POWER`), and its supply side is 216.2 mΩ of its 122.7 mV.
  - A wider class (1,500 µm) cuts the supply side by about 3× to about 75 mΩ, so the own term falls to about 51 mV.
  - Alone, that leaves VDD_3V3_RF at about 169 mV (fails) and VDD_3V3 at 271 mV (unchanged, fails).
  - With the split, VDD_3V3_RF is about 81 mV (passes), and VDD_3V3 stays at 118 mV (fails by 8).
  - VDD_3V3's remaining lever is its own R₁₁ (65.3 mΩ) or the radio cluster's entry bound (44.1 mΩ), by placement or the ground via class. That is measured, not predicted.
- **P3.5** Widening VDD_3V3_RF moves routing, so the capacitance ladder may pick another rung. At 1303b a via change pushed `IMU_SCK` to 31.27 pF against 30 pF. Predicted: the board still routes, and the ladder still passes.

## P4. The cost

- **P4.1** Sentinel-03's `emit` takes 104 s on Furnace (the 4310 `INV-08` run). Its `extract` takes 1,078 s (4310 (3c)).
- **P4.2** `bench-package-s03` refuses at readiness in 6 min today (dispatch 46020469752). When it runs whole, it is about 1919's 28.0–33.6 min plus 4310's extract growth (4.4 → about 8.5 min in the side lane, under prove's 25 min), inside 1919's 45 min.

## P5. The stops this predicts

- No held supervisor fitted: true before TPS3839 was held. It fits now, with the rail raised (P1.3, P2.2).
- The split cannot meet VDD_3V3's limit on the copper as it stands (P3.3, P3.4). It also needs a change to `4310`'s gate, outside this fence (P3.1).
