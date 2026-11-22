# AET-WO-4300h — the measurement (§1.1–§1.4), against PREDICTIONS.md

Run 2026-11-22 at `main` `f46c3c42` plus this spike. The binary is `aethelgard`, built from `f46c3c42` (release, `CARGO_TARGET_DIR` in scratch). The commit order:
- predictions first (`817ed9eb`);
- the readers and the rules before `judge.py` first ran (`201b42d6`);
- the measurement (`58d5825e`).

**The adversarial pass then corrected three rules** (`RULES.md`, "Corrections"; FEEDBACK §5):
- **note 3** applies to the pulls the engine writes;
- **unsolved rows** are named before any bound is looked up;
- **the battery input connector** is listed.

Everything below is the corrected run, except where a row says "as committed".

## The kit

| file | what it does |
|---|---|
| `run.sh` | `validate`, `solve` and `emit` on each board of `spike/1216/universe.sh` (the one home of board names; 34 files), then both walks. Writes `out/boards.tsv`, `out/facing/` and `out/inward/`. |
| `facing.py` | **Walk 1**, from the specification and its solved topology. It lists every `connectors[]` pin (`connector` on an MCU net, `connector_other` otherwise) and every `test_points[]` entry on an MCU net. For each it gives the pin, its Table 8 structure (`power-parts.json` `io.pins`; `RST`, `B`), the solved signal and AF, hand or solver, the MCU's direction, the reset pull, the pull the solved topology implies (`fw_pull`), the rails on the end and what is known about the far end. Three more classes: `power_input` (the battery connector), and two no option sees, `undeclared_offboard` (`sidecars/undeclared-offboard.tsv`) and `foreign_clock`. |
| `inward.py` | **Walk 2**, from the emitted `.kicad_pcb` only, inward from every `J`/`TP` pad. A two-pad R, L, FB or D passes (3 hops, signal nets only). Anything else ends the walk and is named (`ic_bridge` when its other pads carry MCU nets). Structure comes from the rendered-page transcription (`spike/1211e/out/table8-visual.tsv`). |
| `compare.py` | Walk 1 against walk 2 on every drawn board, both ways (`out/compare.tsv`), and the three reads of Table 8 (`out/table8.tsv`). |
| `pintable.py` | The solver's pin table, parsed without linking it from the open pin data `aethelgard-svd-hal` embeds (`out/pintable.tsv`), and checked against every solved assignment. |
| `alts.py` | Every pin the solver could take instead of each TT pin found, judged as `solve.rs` would (`out/alts.tsv`). |
| `moves.py` | Each move, run on a scratch copy: `validate`, `solve`, `emit`, and what moved (`out/moves.tsv`). |
| `judge.py`, `RULES.md`, `sidecars/drive.tsv` | The options as rules, on both branches of the motor-pull fork (`out/options-{internal,external}.tsv`, `out/judged-*.tsv`). |
| `summary.py` | One line per board (`out/summary.md`). |

The readers read the CLI's output. Nothing under `spike/4300h/` has a `Cargo.toml` (as in `1216` D6).

## 1. Every connector- and test-point-facing MCU pin (§1.2)

- **The run** (`out/boards.tsv`):
  - 25 of 34 boards draw.
  - Four draw none by declaration (`DrawsNoBoard`): D1, `hse-external-8`, `imu-baro-1v8` and `sentinel-02-bench-hse8`.
  - `validate` refuses two: D2 (`PartLimitUndeclared`) and `invalid-zero-domain`.
  - `solve` refuses the three `solve/` fixtures.
  - All 34 are walked from the specification.
- **236 rows on 33 boards** (all but `invalid-zero-domain`, which has no connector): 167 connector, 53 test point, 2 battery input, 12 bench header and 2 foreign clock. That is 202 distinct (board, pin). Per board: `out/summary.md`. Every row: `out/facing/`.
- **The reconciliation** (`out/compare.tsv`): on the 25 drawn boards walk 1 and walk 2 agree on **192 of 192** (end, pin) rows, structure included. Neither walk has a row the other lacks. 30 rows are on boards that do not draw (walk 1 only, by construction).
- **Through a series part:** only the battery input connector. On `sentinel-03` (J4.2) and `sentinel-02-power` (J1.2), VBAT reaches **PF3 (FT_ha)** through R50, 100 kΩ. This is `power.input`, which `connectors[]` does not list. It is `1216`'s battery divider (2536 mV), which `1216b` judges at VDD = 0. The first run missed it (adversarial F2).
- **Through a translator:** none. Walk 2's 29 `ic_bridge` rows are all bus peers on the same net (TMP117, BME280, W25Q64, BMP581). The one translator (U13 TXU0304, `sentinel-02-power`) is on no connector's net.
- **Through a rail** (`via_rail`): 169 rows from `VDD_3V3` (`mcu.supply` on every such board), through the I2C pull-ups.
- **`VDD_5V`:** six boards carry it on a connector. On the five that draw, it reaches no MCU pin (13 `no_mcu_pin` rows).
- **The structures** (`out/table8.tsv`): the three reads agree on **29 of 29** pins listed. The text read has no PA13 or PA14, which the PDF names `PA13(JTMS/SWDIO)`; the other two reads agree on them. PE9 was also read on the rendered page 72: LQFP144 pin 60, `TT_ha`, TIM1_CH1. The adversarial reviewer read every listed pin and every proposed alternative on its rendered page (FEEDBACK §5).

**The non-FT pins at an off-board end:**

| board | end | pin | structure | signal | hand | pull after first GPIO init |
|---|---|---|---|---|---|---|
| `sentinel-03` | **J5.1**, TP19 | **PE9** | **TT_ha** | MOTOR1, TIM1_CH1 AF1, dshot600, output | **hand** (`motor_outputs[0].pin`) | **down** (as on every motor output) |
| `sentinel-01`, `sentinel-01-ext-bmp581` | TP10 / TP9 | PA5 / PA7 | TT_ha / TT_a | SPI_SCK / SPI_MOSI, SPI1 AF5, outputs | solver | none (floating) |
| `sentinel-01-bench` | TP3 / TP4 | PA5 / PA7 | TT_ha / TT_a | the same | solver | none |
| `sentinel-02-bench`, `-hse8` | Nucleo CN7.10 (undeclared) | PA5 | TT_ha | IMU_SCK, SPI1 AF5, output | hand | none |
| `sentinel-01` family, `sentinel-03` | J2.3 and a TP | NRST | RST | reset, bidirectional, embedded pull-up | fixed | up (embedded) |
| the same | J2.2 and a TP | BOOT0 | B | boot mode input | fixed | none |
| D2 (`validate` refuses) | J1.1, J1.2, J4.1 | unsolved | — | CMD_TX, CMD_RX, SERVO1 | solver | — |

**Pins holding an internal pull at an off-board end:**
- **From reset:** PA13 (up) and PA14 (down), at all 34 SWD ends. PA15 and PB4 face none.
- **After the first GPIO init:** every motor output.
  - `fw_pull` down, written by the generators by the ruling on PR #161 Q-G (`silicon::motor_output_pupd`).
  - The generated Sentinel-03 crate writes GPIOE `PUPDR` = `0x28880000`, a pull-down on PE9, PE11, PE13 and PE14, read here first-hand.
  - That is 35 pulled nets on 9 boards, 37 off-board rows.
  - No GPIO input with a declared pull faces an off-board end.

**What is known about the far ends:**
- **DS-009 ports** (TELEM, GPS, I2C; 28 connector rows): DS-009 pp. 6–9 prints **+3.3V nominal**, with no maximum, and says nothing on driving while the autopilot is unpowered (`1216` F5). Every DS-009 pin is FT and unpulled.
- **J5 on Sentinel-03:** `esc: none`, "a bench header, … ESCs, when added, are fed from the pack" (the specification's own source). No ESC is declared, so no drive level is declared either.
- **Every other connector** declares only its off-board capacitance (`offboard`), or nothing.
- **Each TT pin found is an MCU output.** A far end puts a level on it only by driving back: bidirectional DShot telemetry (not declared), a fault, a mis-wire, or an instrument.
- **PA5 also takes no injected current in either direction** (Table 11 p. 104: "PA4, PA5 −0/0" mA; every other I/O −5/+0).
- **The bench header ends** (`3304` bench §B4):
  - the breakouts are fed from the Nucleo's own 3V3 (CN8.7), the MCU's supply;
  - the FX2 analyzer is an input with a 1.6 V threshold.

## 2. The alternatives (§1.3)

- **The pin table** (`pintable.py`): 839 (pin, signal, AF) rows. **279 of 279** solved assignments on the committed boards are found in it.
- **MOTOR1 (TIM1_CH1).** On the LQFP144, TIM1_CH1 is on **PA8 and PE9 only**.
  - **PA8 (FT_fha, p. 80)** is the one same-channel FT alternative. `3400` kept it free: it is MCO1's only pin, reserved for E5 (capture input and `MCO`, T7; `3400` FEEDBACK L265, L268).
  - **Every board where the solver placed a TIM1_CH1 output, it chose PA8:**
    - MOTOR1 on `hse-crystal-16`, `hse-external-8`, `motors-dshot600`, `motors-pwm`, `sentinel-02-shaped` and `serial-dshot-9600`;
    - the TIM1_CH1 outputs of D1 and `driver-judges-nothing`.
  - **Other timers:** `out/alts.tsv` lists **21 free FT pins carrying 27 free timer channels** on Sentinel-03. PA15 and PB4 among them hold a reset pull-up.
  - **TIM12, TIM13 and TIM14** are free to the solver. Whether DShot can run there (a DMA request) is not verified on a held page (adversarial F7). No move uses them.
  - **Whole-set moves:** only **TIM4 on PD12–PD15** has all four channels on free FT pins that `3400` did not keep free.
    - TIM1 would need PA8 (MCO1), and PA11 (USB DM) or PE14 for CH4.
    - TIM3 and TIM8 would need PC9 (MCO2) for CH4.
- **PA5/PA7 (SPI1 SCK/MOSI).** The FT alternatives on SPI1 are **PB3** (FT, p. 85) and **PG11** (FT_h) for SCK, and **PB5** (FT, p. 86) and **PD7** (FT_h) for MOSI.
  - All are free on the three `sentinel-01` boards.
  - The solver takes PA5 and PA7 because they come first in its order (pin name).
  - **On the Nucleo bench, PB3 and PG11 are not free:** PB3 meets the ST-LINK SWO input (SB32 ON by default), and PG11 meets RMII TX_EN (SB27). Source: UM2407 Rev 6, held at `~/.cache/aet-3304/ds/um2407-rev6.txt` L2186–2189 and L2137 (adversarial F6).

**The moves, run** (`out/moves.tsv`; each a scratch copy, hand-pinned):

| board | move | `solve` | what moved in the solved topology | the drawn board | `validate` |
|---|---|---|---|---|---|
| `sentinel-03` | MOTOR1 `pin: null` | 0 | MOTOR1 PE9 → **PA8**, TIM1_CH1 | segments 1298 → 1364, vias 337 → 337 | 0 |
| `sentinel-03` | MOTOR1 on PA8 | 0 | the same, and **nothing else**: timer, channel, AF 1 and timing unchanged | the same board | 0 |
| `sentinel-03` | MOTOR1 on PD12 | 0 | MOTOR1 → TIM4_CH1 (two timers carry the motors) | 1298 → 1322, vias 337 → 357 | 0 |
| `sentinel-03` | all four on TIM4, PD12–PD15 | 0 | four pins; TIM1 → TIM4 (APB2 → APB1) | 1298 → 1460, vias 337 → 379 | 0 |
| `sentinel-03` | all four `null` | 0 | MOTOR1 → PA8, MOTOR4 → **PA11** (USB DM), as `3400` saw | 1298 → 1300, vias 337 → 319 | 0 |
| `sentinel-01` | SCK PB3, MOSI PB5 | 0 | those two pins only | 729 → 839, vias 136 → 155 | **2 `UnwitnessedAcknowledgementStale`** |
| `sentinel-01-bench` | SCK PB3, MOSI PB5 | 0 | those two only | 679 → 638, vias 130 → 132 | **2, the same** |
| `sentinel-01-ext-bmp581` | SCK PB3, MOSI PB5 | 0 | those two only | 824 → 829, vias 156 → 159 | 0 |
| the three | SCK PG11, MOSI PD7 | 0 | those two only | moved | the same 2 / 2 / 0 |
| `sentinel-02-bench` | IMU_SCK PB3 | 0 | that pin only | 459 → 470, vias 77 → 89 | 0 (on the bench it meets SB32) |

- **What the stale acknowledgement is:** `sentinel-01.json` and `sentinel-01-bench.json` carry `acknowledge_unwitnessed[5]`, naming `GPIOA.AFRL` `AFSEL5`, `AFSEL7` (`4230` R1). Moving SCK and MOSI off port A empties those fields. The specification must re-name them (GPIOB `AFSEL3`, `AFSEL5`) before `validate` passes.
- **What else moves with the `sentinel-01` family:** the committed generated crates `boards/sentinel-01` and `boards/sentinel-01-bench`, and `fab/sentinel-01-bench/package` (a fabrication package `ORDER.md` prepares for ordering).
- **What moves with a Sentinel-03 re-pin:** it has no committed `fab/` or `boards/` output. Its tests and scenarios read the fixture (`board_tier2.rs` L1800–1837, `twin_tier2.rs` L4956, `scenarios/sentinel-03/`).
- **MCO2 as MCO1's stand-in if PA8 is taken** (RM0433 Rev 9, RCC_CFGR, p. 391): both can output `hse_ck`. MCO2 also gives `sys_ck`, `pll2_p_ck`, `pll1_p_ck`, CSI and LSI. MCO1 alone gives HSI, LSE, `pll1_q_ck` and HSI48. PC9 is MCO2's pin, which `3400` also kept free.

## 3. The limits the options judge by

**With VDD up** (Table 13 p. 105; `mcu.supply` minimum 3135 mV on 32 of 34 boards):

| pin | limit |
|---|---|
| TT | **3435 mV** |
| FT | **5499 mV** |
| BOOT0 | 9.0 V |
| **any pin holding an internal pull** (note 3: "for operation with voltage higher than Min(VDD, VDDA, VDD33USB) + 0.3 V, the internal Pull-up and Pull-Down resistors must be disabled"): NRST, PA13/PA14 from reset, and **every motor output once its pull-down is written** | **VDD + 0.3 V = 3435 mV**, FT or TT |

**With VDD = 0** (Table 10 p. 103): **4.0 V on FT and TT alike**, BOOT0 9.0 V. Note 4 there requires pulls off above 4 V.

So:
- **A motor output takes 3435 mV with firmware running, whatever its structure.** PE9 → PA8 changes its powered limit only while no pull is written: from reset to the first GPIO init, a blank board, NRST held by a probe, and BOOT0 high (whether the ROM bootloader writes a pull is not stated). In those states, the `external` branch's limits are the ones that hold.
- **A "3.3 V" peer at +5 % (3465 mV) is over 3435 mV by 30 mV** on a TT pin, or on any pulled pin.
- **An unpulled FT pin takes a 5 V peer powered.** Every pin is over its limit with a 5 V peer unpowered.
- **Moving a pin from TT to FT changes nothing at VDD = 0.**

## 4. The options, run (`out/options-internal.tsv`, `out/options-external.tsv`)

The fork: `internal` is the engine as it is (motor pull-down written, PR #161 Q-G). `external` is a motor line pulled by a resistor instead, with no internal pull.

| option | input (`sidecars/drive.tsv`) | internal pull: boards refused | rows refused | external pull: rows refused | moved |
|---|---|---|---|---|---|
| **(a)** | — | **1** (`sentinel-03`) | `OffBoardPinNotFt` 2 (J5.1, TP19) | the same | **3** (`sentinel-01` family, test points only) |
| (a), connectors only (read from `judged-*.tsv`) | — | 1 | J5.1 | the same | **0** |
| **b-required** | none declared (today) | **33** | `Undeclared` 164 | the same | 0 |
| b-required | `probe-vtref` | 17 | 90 | the same | 0 |
| **b-declared** | none declared (today) | **0** | — (236 rows named) | the same | 0 |
| b-declared | `ds009-nominal` 3300, `ds009-plus5` 3465 | 0 | — (every DS-009 pin is FT and unpulled) | the same | 0 |
| b-declared | **`j5-3v3-plus5` 3465** | **1** | `AboveTolerance` **4** (J5.1–J5.4) | `AboveTolerance` **1** (PE9) | 0 |
| b-declared | **`j5-5v-esc` 5250** | **1** | `AboveTolerance` **4**, `AboveLimitUnpowered` 4 | `AboveTolerance` 1, `AboveLimitUnpowered` 4 | 0 |
| **(c)**, declared | none, `ds009-*`, `probe-vtref` | 1 | (a)'s 2 | the same | 3 |
| (c), required | none | 33 | 164 + (a)'s 2 | the same | 3 |
| **(c) after the re-pin, MOTOR1 on PA8** | none, `ds009-*`, `probe-vtref` | **0** | — | the same | 0 |
| (c) after the re-pin | **`j5-3v3-plus5`** | **1** | `AboveTolerance` **4** | **0: PA8 clears it** | 0 |
| (c) after the re-pin | `j5-5v-esc` | 1 | `AboveTolerance` 4, `AboveLimitUnpowered` 4 | `AboveLimitUnpowered` 4 | 0 |

`rows_refused_by_kind` counts a row that fails both limits once under each kind (adversarial F9).

**As committed at `58d5825e`, before the corrections:**
- `j5-5v-esc` gave `AboveTolerance` 1, not 4;
- the PA8 rows gave 0 `AboveTolerance`;
- b-required counted D2's three unsolved rows (`Undeclared` 167, probe-vtref 18 boards).

**Not judged by any option** (named in `judged-*.tsv`):
- the 12 bench-header rows;
- the 2 foreign clocks;
- D2's 3 unsolved rows;
- the 2 battery-input rows (`1216b` judges the divider);
- every test point under (b).

## 5. Predictions beside results

| # | predicted | measured | |
|---|---|---|---|
| 1 | 34 boards | 34 | holds |
| 2 | 26 draw (D1 drawable after `1211f`) | **25**: D1 still declares `board.drawn: false` (`DrawsNoBoard`) | **wrong** |
| 3 | 33 of 34 with an off-board pin | 33 | holds |
| 4 | every end direct; no series part or translator | no translator; **but the battery input connector reaches PF3 through R50** on 2 boards (`power.input`, which `connectors[]` does not list) | **wrong** |
| 5 | PE9 (hand) on Sentinel-03; PA5/PA7 (solver) on the `sentinel-01` family; no other TT | exactly | holds |
| 6 | 2 bench boards, PA5 on a header | 2; the breakouts are on the MCU's own 3V3 | holds |
| 7 | NRST, BOOT0 on the `sentinel-01` family and Sentinel-03 | the same 4 boards | holds |
| 8 | PA13/PA14 at every SWD end; PA15/PB4 at none | 34 / 34; 0 | holds |
| 9 | three Table 8 reads agree | 29 / 29 (two reads for PA13/PA14) | holds |
| 10 | TIM1_CH1 on PA8 and PE9 only; `null` → PA8; PA8 moves only the pin; TIM4 PD12–15 free | all four | holds |
| 11 | PB3/PG11, PB5/PD7; solve on all three, moving only those pins | they solve, and only those pins move; **`validate` refuses 2 of 3 on a stale acknowledgement** | **holds in `solve`; missed a cost** |
| 12 | every solved assignment in the parsed table | 279 / 279 | holds |
| 13 | TT 3435, FT 5499, note-3 pins 3435; 3.3 V +5 % over TT by 30 mV | the same, **but note 3 also covers every motor output** (its written pull-down), which I did not predict | **holds, incomplete** |
| 14 | FT = TT = 4.0 V at VDD = 0 | the same | holds |
| 15 | (a) refuses 1, moves 3 | 1, 3 | holds |
| 16 | b-required 33; b-declared 0; DS-009 sidecars 0; J5 sidecar: PE9 alone with VDD up, 4 unpowered | 33, 0, 0; **J5 with VDD up refuses all four** (the pulls) | **wrong on J5 powered** |
| 17 | (c) the union; (a) moving PE9 to FT clears the powered case | the union; **PA8 clears nothing powered while the pull-down is written** | **wrong** |
| 18 | §7's stop does not fire | it does not (§6) | holds |

## 6. §7's immediate stop

**It does not fire.** No committed board puts a stated level above Table 13's limit on an MCU pin with VDD up by an on-board path that no option judges.
- **Checked again at the corrected limit**, 3435 mV on every internally pulled pin: `1216`'s walk 1 (`spike/1216/spec.py`), run on this run's solves, lists no off-domain source on any of the 35 pulled nets.
- A1 judges every declared pull and pin-device level with VDD up (`1211e`).
- The I2C pulls are on `mcu.supply`, and a source on `mcu.supply` never exceeds VDD (A1's same-rail exemption).
- The battery divider gives 2536 mV on an unpulled FT_ha pin.
- Every off-board end has no stated level, so none exceeds a rating by a stated figure.
- The J5 sidecars are hypothetical.
