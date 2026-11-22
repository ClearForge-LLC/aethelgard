# AET-WO-4300h — the measurement (§1.1–§1.4), against PREDICTIONS.md

Run 2026-11-22 at `main` `f46c3c42` plus this spike. The binary is `aethelgard` built from `f46c3c42` (release, `CARGO_TARGET_DIR` in scratch). Predictions were committed first (`817ed9eb`); the readers and the rules before `judge.py` first ran (`201b42d6`); the measurement after (`58d5825e`).

## The kit

| file | what it does |
|---|---|
| `run.sh` | `validate`, `solve` and `emit` on each board of `spike/1216/universe.sh` (the one home of board names; 34 files), then both walks. Writes `out/boards.tsv`, `out/facing/`, `out/inward/`. |
| `facing.py` | **Walk 1**, from the specification and its solved topology. Every `connectors[]` pin (on an MCU net: `connector`; otherwise `connector_other`, so none is dropped) and every `test_points[]` entry on an MCU net, with the pin, its Table 8 structure (`power-parts.json` `io.pins`; `RST`, `B`), the solved signal and AF, hand or solver, the MCU's direction, the reset pull, the rails on the end and what is known about the far end. Two classes no option sees: `undeclared_offboard` (`sidecars/undeclared-offboard.tsv`) and `foreign_clock`. |
| `inward.py` | **Walk 2**, from the emitted `.kicad_pcb` only, inward from every `J`/`TP` pad. A two-pad R, L, FB or D passes (3 hops, signal nets only); anything else ends the walk and is named (`ic_bridge` when its other pads carry MCU nets). Structure from the rendered-page transcription (`spike/1211e/out/table8-visual.tsv`). |
| `compare.py` | Walk 1 against walk 2 on every drawn board, both ways (`out/compare.tsv`), and the three reads of Table 8 (`out/table8.tsv`). |
| `pintable.py` | The solver's pin table, parsed from the open pin data `aethelgard-svd-hal` embeds, without linking it (`out/pintable.tsv`), checked against every solved assignment. |
| `alts.py` | Every pin the solver could take instead of each TT pin found, judged as `solve.rs` would (`out/alts.tsv`). |
| `moves.py` | Each move run on a scratch copy: `validate`, `solve`, `emit`, and what moved (`out/moves.tsv`). |
| `judge.py`, `RULES.md`, `sidecars/drive.tsv` | The options as rules (`out/options.tsv`, `out/judged.tsv`). |
| `summary.py` | One line per board (`out/summary.md`). |

The readers read the CLI's output; nothing under `spike/4300h/` has a `Cargo.toml` (as `1216` D6).

## 1. Every connector- and test-point-facing MCU pin (§1.2)

- **The run** (`out/boards.tsv`): 25 of 34 boards draw. Four draw none by declaration (`DrawsNoBoard`: D1, `hse-external-8`, `imu-baro-1v8`, `sentinel-02-bench-hse8`). `validate` refuses two (D2 `PartLimitUndeclared`, `invalid-zero-domain`). `solve` refuses three `solve/` fixtures. All 34 are walked from the specification.
- **234 rows on 33 boards** (all but `invalid-zero-domain`, which has no connector): 167 connector, 53 test point, 12 bench header, 2 foreign clock. 200 distinct (board, pin). Per board: `out/summary.md`; every row: `out/facing/`.
- **The reconciliation** (`out/compare.tsv`): on the 25 drawn boards walk 1 and walk 2 agree on **190 of 190** (end, pin) rows, structure included. Neither walk has a row the other lacks. 30 rows are on boards that do not draw (walk 1 only, by construction).
- **No connector or test point reaches an MCU pin through a series part or a translator**: walk 2 found **0 `series` rows**. Its 29 `ic_bridge` rows are all bus peers on the same net (TMP117, BME280, W25Q64, BMP581), not translators. The one translator (U13 TXU0304, `sentinel-02-power`) is on no connector's net.
- **Through a rail** (`via_rail`), from the drawn ends: 169 rows from `VDD_3V3` (= `mcu.supply` on every such board) through the I2C pull-ups; and **2 from `VBAT`**: the battery connector **J4.2**, which the board draws from `power.battery` and `connectors[]` does not list, through R50 100 k onto **PF3 (FT_ha)** on `sentinel-03` and `sentinel-02-power`. That is `1216`'s battery divider (2536 mV), which `1216b` judges at VDD = 0. `VDD_5V` on a connector (six boards) reaches no MCU pin on the five that draw (13 `no_mcu_pin` rows).
- **The structures** (`out/table8.tsv`): the three reads agree on **28 of 28** pins listed (the text read has no PA13/PA14, named `PA13(JTMS/SWDIO)` in the PDF; the other two agree). PE9 was also read on the rendered page 72: LQFP144 pin 60, `TT_ha`, TIM1_CH1.

**The non-FT pins at an off-board end:**

| board | end | pin | structure | signal | hand |
|---|---|---|---|---|---|
| `sentinel-03` | **J5.1**, TP19 | **PE9** | **TT_ha** | MOTOR1, TIM1_CH1 AF1, dshot600, output | **hand** (`motor_outputs[0].pin`) |
| `sentinel-01`, `sentinel-01-ext-bmp581` | TP10 / TP9 | PA5 / PA7 | TT_ha / TT_a | SPI_SCK / SPI_MOSI, SPI1 AF5, outputs | solver |
| `sentinel-01-bench` | TP3 / TP4 | PA5 / PA7 | TT_ha / TT_a | the same | solver |
| `sentinel-02-bench`, `-hse8` | Nucleo CN7.10 (undeclared) | PA5 | TT_ha | IMU_SCK, SPI1 AF5, output | hand |
| the `sentinel-01` family, `sentinel-03` | J2.3 and a TP | NRST | RST | reset, bidirectional, embedded pull-up | fixed |
| the same | J2.2 and a TP | BOOT0 | B | boot mode input | fixed |
| D2 (`validate` refuses) | J1.1, J1.2, J4.1 | unsolved | — | CMD_TX, CMD_RX, SERVO1 | solver |

**Pins holding an internal pull from reset:** PA13 (up) and PA14 (down), at all 34 SWD ends. PA15 and PB4 face none.

**What is known about the far ends:**
- **DS-009 ports** (TELEM, GPS, I2C; 28 connector rows on the boards that declare a standard): DS-009 pp. 6–9 prints **+3.3V nominal**, no maximum, and nothing on driving while the autopilot is unpowered (`1216` F5).
- **Every other connector:** declares only the off-board capacitance (`offboard`), or nothing.
- **J5 on Sentinel-03:** `esc: none`, "a bench header, … ESCs, when added, are fed from the pack" (the specification's own source). No ESC, and so no drive level, is declared.
- **Every MCU end is an output except** RX, MISO, SWDIO, SWCLK, NRST, BOOT0, I2C and the foreign clock.
  - Each TT pin found is an MCU **output** (TIM1_CH1, SPI1 SCK and MOSI). A far end puts a level on it only if it drives back: bidirectional DShot telemetry (not declared), a fault, a mis-wire, or an instrument.
- **The bench header ends** (`3304` bench §B4): the breakouts are fed from the Nucleo's own 3V3 (CN8.7), the MCU's supply. The FX2 analyzer is an input with a 1.6 V threshold.

## 2. The alternatives (§1.3)

- **The pin table** (`pintable.py`): 839 (pin, signal, AF) rows. **279 of 279** solved assignments on the committed boards are found in it.
- **MOTOR1 (TIM1_CH1).** On the LQFP144, TIM1_CH1 is on **PA8 and PE9 only**.
  - **PA8 (FT_fha, page 80)** is the one same-channel FT alternative. `3400` kept it free: it is MCO1's only pin, reserved for E5 (capture input and `MCO`, T7; `3400` FEEDBACK L265, L268).
  - **Every board where the solver placed MOTOR1, it placed it on PA8**: `hse-crystal-16`, `hse-external-8`, `motors-dshot600`, `motors-pwm`, `sentinel-02-shaped`, `serial-dshot-9600`, and D1's and `driver-judges-nothing`'s TIM1_CH1 outputs.
  - Other timers: `out/alts.tsv` lists **21 free FT pins carrying 27 free timer channels** on Sentinel-03 (PA15 and PB4 among them hold a reset pull-up).
  - **Whole-set moves:** only **TIM4 on PD12–PD15** has all four channels on free FT pins that `3400` did not keep free.
    - TIM1 would need PA8 (MCO1) and has PA11 (USB DM) or PE14 for CH4.
    - TIM3 and TIM8 would need PC9 (MCO2) for CH4.
- **PA5/PA7 (SPI1 SCK/MOSI):** the FT alternatives on SPI1 are **PB3** (FT, p. 85) and **PG11** (FT_h) for SCK, and **PB5** (FT, p. 86) and **PD7** (FT_h) for MOSI. All are free on the three `sentinel-01` boards. In the solver's order (pin name), PA5 and PA7 come first, which is why it takes them.

**The moves, run** (`out/moves.tsv`; each a scratch copy, hand-pinned):

| board | move | `solve` | what moved in the solved topology | the drawn board | `validate` |
|---|---|---|---|---|---|
| `sentinel-03` | MOTOR1 `pin: null` | 0 | MOTOR1 PE9 → **PA8**, TIM1_CH1 | segments 1298 → 1364, vias 337 → 337 | 0 |
| `sentinel-03` | MOTOR1 on PA8 | 0 | the same, **nothing else**: timer, channel, AF 1, timing unchanged | the same board | 0 |
| `sentinel-03` | MOTOR1 on PD12 | 0 | MOTOR1 → TIM4_CH1 (two timers carry the motors) | 1298 → 1322, vias 337 → 357 | 0 |
| `sentinel-03` | all four on TIM4, PD12–PD15 | 0 | four pins, TIM1 → TIM4 (APB2 → APB1) | 1298 → 1460, vias 337 → 379 | 0 |
| `sentinel-03` | all four `null` | 0 | MOTOR1 → PA8, MOTOR4 → **PA11** (USB DM), as `3400` saw | 1298 → 1300, vias 337 → 319 | 0 |
| `sentinel-01` | SCK PB3, MOSI PB5 | 0 | those two pins only | 729 → 839, vias 136 → 155 | **2 `UnwitnessedAcknowledgementStale`** |
| `sentinel-01-bench` | SCK PB3, MOSI PB5 | 0 | those two only | 679 → 638, vias 130 → 132 | **2, the same** |
| `sentinel-01-ext-bmp581` | SCK PB3, MOSI PB5 | 0 | those two only | 824 → 829, vias 156 → 159 | 0 |
| the three | SCK PG11, MOSI PD7 | 0 | those two only | moved | the same 2 / 2 / 0 |
| `sentinel-02-bench` | IMU_SCK PB3 | 0 | that pin only | 459 → 470, vias 77 → 89 | 0 |

- **What the stale acknowledgement is:** `sentinel-01.json` and `sentinel-01-bench.json` carry `acknowledge_unwitnessed[5]`, naming `GPIOA.AFRL` `AFSEL5`, `AFSEL7` (`4230` R1). Moving SCK and MOSI off port A empties those fields. The specification must re-name them (GPIOB `AFSEL3`, `AFSEL5`) before `validate` passes.
- **What else moves with the `sentinel-01` family:** the committed generated crates `boards/sentinel-01` and `boards/sentinel-01-bench`, and `fab/sentinel-01-bench/package` (a fabrication package, through the package queue).
- **What moves with a Sentinel-03 re-pin:** it has no committed `fab/` or `boards/` output. Its tests and scenarios read the fixture (`board_tier2.rs` L1800–1837, `twin_tier2.rs` L4956, `scenarios/sentinel-03/`).
- **MCO2 as MCO1's stand-in, if PA8 is taken:** both can output `hse_ck`. MCO2 also gives `sys_ck`, `pll2_p_ck`, `pll1_p_ck`, CSI and LSI. MCO1 alone gives HSI, LSE, `pll1_q_ck` and HSI48 (RM0433 Rev 9, RCC_CFGR, p. 391). PC9 is MCO2's pin, which `3400` also kept free.

## 3. The limits the options judge by

- **With VDD up** (Table 13 p. 105, `mcu.supply` minimum 3135 mV on 32 of 34 boards):
  - TT: **3435 mV**;
  - FT: **5499 mV**;
  - BOOT0: 9.0 V;
  - NRST, and PA13 or PA14 while their reset pull holds: VDD + 0.3 V (note 3).
- **With VDD = 0** (Table 10 p. 103): **4.0 V on FT and TT alike**, BOOT0 9.0 V.

So:
- **a "3.3 V" peer at +5 % (3465 mV) is over TT by 30 mV** with the MCU powered;
- a 5 V peer (5250 mV) is inside FT powered and over every pin unpowered;
- **moving a pin from TT to FT changes nothing at VDD = 0.**

## 4. The options, run (`out/options.tsv`)

| option | scenario (`sidecars/drive.tsv`) | boards refused | boards moved | rows refused |
|---|---|---|---|---|
| **(a)** | — | **1** (`sentinel-03`) | **3** (the `sentinel-01` family) | `OffBoardPinNotFt` 2 (J5.1, TP19) |
| (a), connectors only (read from `judged.tsv`) | — | 1 | **0**: every moved row is a test point | J5.1 |
| **b-required** | none declared (today) | **33** | 0 | `OffBoardDriveUndeclared` 167 |
| b-required | `probe-vtref` | 18 | 0 | 93 |
| **b-declared** | none declared (today) | **0** | 0 | — (234 rows named) |
| b-declared | `ds009-nominal` 3300 | 0 | 0 | — |
| b-declared | `ds009-plus5` 3465 | 0 | 0 | — (every DS-009 pin is FT) |
| b-declared | **`j5-5v-esc` 5250** | **1** | 0 | `AboveTolerance` 1 (PE9), `AboveLimitUnpowered` **4** (J5.1–J5.4) |
| **(c)**, declared | none / `ds009-*` / `probe-vtref` | 1 | 3 | (a)'s 2 |
| (c), declared | `j5-5v-esc` | 1 | 3 | (a)'s 2 + 1 + 4 |
| (c), required | none | 33 | 3 | 167 + (a)'s 2 |
| **(c) after the re-pin, MOTOR1 on PA8** | none / `ds009-*` / `probe-vtref` | **0** | 0 | — |
| (c) after the re-pin | `j5-5v-esc` | 1 | 0 | `AboveLimitUnpowered` **4**: PA8 is FT, so the powered case clears, and the unpowered case stands on every J5 pin |

**Not judged by any option** (named in `judged.tsv`):
- the 12 bench-header rows;
- the 2 foreign clocks;
- D2's 3 unsolved rows;
- the battery connector J4 (drawn, not in `connectors[]`; `1216b` judges its divider);
- every test point under (b).

## 5. Predictions beside results

| # | predicted | measured | |
|---|---|---|---|
| 1 | 34 boards | 34 | holds |
| 2 | 26 draw (D1 drawable after `1211f`) | **25**: D1 still declares `board.drawn: false` (`DrawsNoBoard`) | **wrong** |
| 3 | 33 of 34 with an off-board pin | 33 | holds |
| 4 | every end direct; no series part or translator | 0 `series`, 0 translator; **but the drawn battery connector J4 reaches PF3 through R50** (`via_rail`, 2 boards), which `connectors[]` does not list | **partly wrong** |
| 5 | PE9 (hand) on Sentinel-03; PA5/PA7 (solver) on the `sentinel-01` family; no other TT | exactly | holds |
| 6 | 2 bench boards, PA5 on a header | 2; the breakouts are on the MCU's own 3V3 | holds |
| 7 | NRST, BOOT0 on the `sentinel-01` family and Sentinel-03 | the same 4 boards | holds |
| 8 | PA13/PA14 at every SWD end; PA15/PB4 at none | 34 / 34; 0 | holds |
| 9 | three Table 8 reads agree | 28 / 28 (two reads for PA13/PA14) | holds |
| 10 | TIM1_CH1 on PA8 and PE9 only; `null` → PA8; PA8 moves only the pin; TIM4 PD12–15 free | all four | holds |
| 11 | PB3/PG11, PB5/PD7; solve on all three, moving only those pins | solve, and only those pins move; **`validate` refuses 2 of 3 by a stale acknowledgement** | **holds in `solve`, missed a cost** |
| 12 | every solved assignment in the parsed table | 279 / 279 | holds |
| 13 | TT 3435, FT 5499, note-3 pins 3435; 3.3 V +5 % over TT by 30 mV | the same | holds |
| 14 | FT = TT = 4.0 V at VDD = 0 | the same | holds |
| 15 | (a) refuses 1, moves 3 | 1, 3 | holds |
| 16 | b-required 33; b-declared 0; DS-009 sidecars 0; J5 sidecar PE9 up, 4 unpowered | the same | holds |
| 17 | (c) the union | the same | holds |
| 18 | §7's stop does not fire | it does not (§6) | holds |

## 6. §7's immediate stop

**It does not fire.** No committed board puts a stated level above Table 13's limit on an MCU pin with VDD up by an on-board path that no option judges:
- A1 judges every declared pull and pin-device level with VDD up (`1211e`);
- the I2C pulls are on `mcu.supply` (169 `via_rail` rows);
- the battery divider gives 2536 mV on an FT_ha pin.

Every off-board end has no stated level, so none exceeds a rating by a stated figure.
