# AET-WO-4300h — FEEDBACK: what a connector or test point can put on an MCU pin (design spike, STOPPED at §7 for the ruling)

**Branch:** `wo/aet-4300h-offboard-pins` from `main` `f46c3c42`.
**Builder:** Claude (Opus 5.5 orchestrating), with one Opus adversarial reviewer. The reviewer was given the code and §5, not my conclusions, and every finding was checked at source before use. Written 2026-11-22.

**Status: STOPPED at §7 for the ruling.** No crate, specification, `fab/`, workflow or `ci/` file changes.

**The answer, in six lines:**
- **Every off-board end on the 34 committed boards is listed:** 234 rows on 33 boards (`spike/4300h/out/facing/`, `out/summary.md`). The two walks agree on 190 of 190 rows on the 25 drawn boards. No connector or test point reaches an MCU pin through a series part or a translator.
- **One connector-facing TT pin:** Sentinel-03's MOTOR1 on **PE9 (TT_ha), a hand pin**, at J5.1 and TP19.
  - **Test points on TT pins:** PA5 (TT_ha) and PA7 (TT_a) on the three `sentinel-01` boards, solver-chosen.
  - **One class no option sees:** the bench's Nucleo header wiring puts PA5 (TT_ha, hand) off-board on `sentinel-02-bench` and `-hse8`, and `connectors[]` does not declare it.
- **MOTOR1's FT alternative on its own channel is PA8 alone** (TIM1_CH1 is on PA8 and PE9 only). Moved there, the solved topology changes that one pin and nothing else. PA8 is where the solver puts MOTOR1 on every other board, and the pin `3400` kept free for MCO1. The only whole-set move off reserved pins is **TIM4 on PD12–PD15**.
- **The limits decide the options:**
  - With VDD up, TT takes 3435 mV at `mcu.supply`'s minimum, so a "3.3 V" peer at +5 % is over it by 30 mV. FT takes 5499 mV.
  - **At VDD = 0, FT and TT both take 4.0 V**, so a pin move changes nothing unpowered.
- **Measured:**

  | option | refused | moved |
  |---|---|---|
  | (a) | 1 (Sentinel-03) | 3 (`sentinel-01` family, test points only) |
  | (a), connectors only | 1 | 0 |
  | (b) required | 33 | 0 |
  | (b) declared, today | 0 | 0 |
  | (b) declared, a 5 V peer on J5 | 1: PE9 powered, J5.1–J5.4 unpowered | 0 |
  | (c) | (a) ∪ (b) | 3 |
  | (c) after MOTOR1 → PA8, 5 V on J5 | 1: J5.1–J5.4 unpowered only | 0 |

- **Lean: (a) at connectors now, with no stated-reason escape. Re-pin MOTOR1 to PA8.** Test points are named, not moved (D2). (b) is deferred until a held page states a peer's drive maximum: no committed far end has one, and (b) is the only option that can judge the unpowered case.

## 0. How it was made

- Predictions were committed before any reader ran: `817ed9eb`.
- The readers and the options as rules (`RULES.md`, `judge.py`, the sidecars) were committed before `judge.py` first ran: `201b42d6`.
- The measurement: `58d5825e`. RESULTS: `e7322d74`. The adversarial corrections, the reduction pass and this file: the commits after.
- **Readers:** in `spike/4300h/`. They read the CLI's output of `main`'s crates (`solve` JSON, emitted `.kicad_pcb`), built from `f46c3c42` into scratch. They do not link the crates; there is no `Cargo.toml` (as `1216` D6). The kit and its outputs are listed in `RESULTS.md`.
- **Structures:** DS12117 Rev 10 Table 8, three reads compared on every listed pin:
  - `power-parts.json` `io.pins` (`1211e`);
  - `1211e`'s text read;
  - `1211e`'s rendered-page read.

  PE9 was also read on the rendered page 72 here.
- **Limits:** Table 13 p. 105 (VDD up, A1's reading) and Table 10 p. 103 (VDD = 0, `1216b`'s), read in the held text.
- **The solver's alternatives:** from its own pin data, parsed independently (`pintable.py`; 279 of 279 solved assignments found in it). Each move was run on a scratch copy (`moves.py`).

## 1. The measurement (§1.2)

Its one home is `spike/4300h/RESULTS.md` §1: the run, the reconciliation, the non-FT table, the far ends. This section keeps what the options turn on.

**The non-FT pins at an off-board end:**

| board | end | pin | structure | signal | placed by |
|---|---|---|---|---|---|
| `sentinel-03` | J5.1, TP19 | PE9 | TT_ha | MOTOR1, TIM1_CH1, output | hand |
| `sentinel-01`, `-ext-bmp581`, `-bench` | TP | PA5, PA7 | TT_ha, TT_a | SPI1 SCK, MOSI, outputs | solver |
| `sentinel-02-bench`, `-hse8` | Nucleo CN7.10 (undeclared) | PA5 | TT_ha | IMU_SCK, output | hand |
| `sentinel-01` family, `sentinel-03` | J2, TP | NRST, BOOT0 | RST, B | reset, boot mode | fixed |

**The far ends:**
- **DS-009 ports** print +3.3V nominal, with no maximum (`1216` F5). Every DS-009 pin on every board is FT.
- **J5 on Sentinel-03** declares no ESC (`esc: none`, "a bench header").
- **Every other connector** declares only its off-board capacitance, or nothing.
- **Each TT pin found is an MCU output.** A far end puts a level on it only by driving back: bidirectional DShot telemetry (not declared), a mis-wire, or an instrument.

**Paths the WO's §5 asked about:**
- **Through a series part:** walk 2 found none from a `connectors[]` pin or test point. It found one from a connector that `connectors[]` does not list: the battery connector **J4.2** (drawn from `power.battery`) reaches **PF3 (FT_ha)** through R50 on `sentinel-03` and `sentinel-02-power`. That is `1216`'s divider (2536 mV), which `1216b` judges at VDD = 0 (F1).
- **Through a translator:** none. The 29 `ic_bridge` rows are bus peers (TMP117, BME280, W25Q64, BMP581).
- **`VDD_5V`:** six boards carry it on a connector. On the five that draw, it reaches no MCU pin (13 `no_mcu_pin` rows); `hse-external-8` draws no board.

## 2. The alternatives (§1.3)

**MOTOR1 (TIM1_CH1, dshot600, Sentinel-03):**

| move | solve | what moves in the solved topology | drawn board (segments, vias) | cost beyond that |
|---|---|---|---|---|
| **PA8** (FT_fha, p. 80) | 0 | MOTOR1's pin. Timer, channel, AF 1 and timing are unchanged | 1298 → 1364, 337 → 337 | takes `3400`'s MCO1 pin (E5, T7, deferred "with pins reserved", L268). MCO2 on PC9, also kept free, can output HSE as MCO1 can (RM0433 Rev 9 p. 391) |
| `pin: null` | 0 | the solver picks **PA8**, as on every other motor board | the same as PA8 | the same |
| **TIM4, PD12–PD15** (FT_fh/FT_h, p. 76) | 0 | all four pins; TIM1 → TIM4 (APB2 → APB1) | 1298 → 1460, 337 → 379 | no reserved pin. The firmware's timer, and the DMA a flight stack uses for it, change |
| PD12 alone (TIM4_CH1) | 0 | MOTOR1 on a second timer | 1298 → 1322, 337 → 357 | the motors span two timers |
| all four `null` | 0 | MOTOR1 → PA8, **MOTOR4 → PA11 (USB DM)** | — | `3400`'s reason for the hand pins |

- **TIM1_CH1 exists only on PA8 and PE9** on the LQFP144.
- **No other timer has four free FT channels** off `3400`'s kept-free pins except TIM4:
  - TIM3 and TIM8 need PC9 (MCO2) for CH4;
  - TIM1 needs PA8.

  The other free FT channel pins are in `out/alts.tsv`: 21 pins, 27 channels.
- **Sentinel-03 has no committed `fab/` or `boards/` output.** A re-pin moves its fixture and the tier-2 tests and scenarios that read it.

**PA5/PA7 on the `sentinel-01` family (SPI1):**
- **The FT alternatives:** PB3 or PG11 for SCK, and PB5 or PD7 for MOSI. All are free. The solver takes PA5/PA7 because they come first in pin order.
- **The move solves on all three boards** and moves only those two pins.
- **But `validate` then refuses `sentinel-01` and `sentinel-01-bench`** (`UnwitnessedAcknowledgementStale`). Their `acknowledge_unwitnessed[5]` names `GPIOA.AFRL` `AFSEL5`/`AFSEL7` (`4230` R1), so each must be re-written for GPIOB.
- **The move also changes** `boards/sentinel-01`, `boards/sentinel-01-bench` and **`fab/sentinel-01-bench/package`**, the package `ORDER.md` prepares for Scotty to order.

## 3. The options (§1.4), each with its cost and its measured refusals

The rules are in `spike/4300h/RULES.md` and `judge.py`, committed before they ran. The results are in `out/options.tsv`, and every row in `out/judged.tsv`.

**The limits** (RESULTS §3):

| | TT | FT | NRST, and a pin holding a reset pull (PA13, PA14) | BOOT0 |
|---|---|---|---|---|
| VDD up (Table 13 p. 105, VDD 3135 mV) | **3435 mV** | **5499 mV** | VDD + 0.3 V (note 3; NRST's pull-up is embedded) | 9.0 V |
| VDD = 0 (Table 10 p. 103) | **4.0 V** | **4.0 V** | 4.0 V | 9.0 V |

### (a) The solver prefers FT at an off-board end; a hand TT pin there is refused unless the specification states why

- **Refuses: 1 / 34**, Sentinel-03 (`OffBoardPinNotFt` on J5.1 and TP19). No field can state why, so this holds for every hand TT pin.
- **Moves: 3 / 34**, the `sentinel-01` family's PA5/PA7 test points to PB3/PB5. All three solve. Two need their acknowledgement re-written, and the third moves a fabrication package (§2).
- **Restricted to connectors:** it refuses the same 1 and moves **0**. Every moved row is a test point.
- **What it judges:** the powered case, for any peer up to 5.5 V on an FT pin. A 3.3 V peer at +5 % passes on FT and fails on TT.
- **What it cannot judge:**
  - anything at VDD = 0, where FT and TT are equal;
  - NRST and BOOT0, which cannot move;
  - the bench header and foreign clock rows, which no `connectors[]` or `test_points[]` entry declares.
- **Cost:**
  - **in the solver:** off-board-facing nets (the `connectors[]` and `test_points[]` nets, already in the specification) take FT candidates first;
  - **in validation:** one refusal for a hand pin;
  - **with no escape** (D5): no new field, `CHANGES` and README rows only, tests;
  - **the Sentinel-03 re-pin** (D3);
  - **with test points included:** the `sentinel-01` family's two acknowledgements, two `boards/` crates and the `fab/sentinel-01-bench` package (D2).
  - **On every other board, solve is byte-identical:** no other off-board end is on a TT pin.

### (b) A declared off-board drive bound per connector pin (`connectors[].pins[].offboard.drive_max_millivolts` with a source), judged by A1 with VDD up and by `1216b`'s rule at VDD = 0

- **Today no specification declares a bound:**
  - **b-required** (an undeclared bound refused) refuses **33 / 34**, 167 rows;
  - **b-declared** (an undeclared bound named) refuses **0**.
- **On hand sidecars** (`sidecars/drive.tsv`, each marked nominal or hypothetical):

  | sidecar | bound | refused |
  |---|---|---|
  | DS-009 nominal | 3300 mV | 0 |
  | DS-009 nominal +5 % | 3465 mV | 0: every DS-009 pin is FT |
  | a debug probe tracking VTref | — | 0 |
  | **a 5 V-logic peer on J5** | **5250 mV** | **PE9 with VDD up, and J5.1–J5.4 at VDD = 0** |

- **The only option that judges the unpowered case,** and only where a bound is declared.
- **What it cannot judge:**
  - **test points:** a test point is not a `connectors[]` pin, so (b) bounds none of the 53 test-point rows;
  - **undeclared ends:** the bench header and the foreign clock.
- **Its bounds have no source today.** DS-009 prints a nominal with no maximum, and J5's peer is undeclared. Every (b) refusal above rests on a hypothetical sidecar.
- **Cost:**
  - a field and a source on `offboard`, specification 26 or later;
  - Table 13's NRST and BOOT0 rows as data beside `1211e`'s FT/TT families (D6);
  - a judge reusing A1's limit and `1216b`'s;
  - `CHANGES`, README, tests.

### (c) Both

- **Refused, by input:**
  - **declared, no sidecar:** **1** (a's), with **3** moved;
  - **required:** 33, with 3 moved;
  - **with a 5 V peer on J5:** 1, on 7 rows.
- **After the ruling's build (MOTOR1 on PA8):**
  - **(c) declared refuses 0;**
  - with a 5 V peer on J5 it still refuses J5.1–J5.4 at VDD = 0. **FT does not help unpowered.** Only hardware can: a series resistor sized to the injection rule, a buffer, or a translator powered from `mcu.supply`.
- **Cost:** (a)'s and (b)'s together.

## 4. The lean

**Lean: (a) at connectors now, with no stated-reason escape; MOTOR1 re-pinned to PA8 by the build the ruling orders; test points named, not moved; (b) deferred to the first work order that declares a peer with a held page.**

**Reasons:**
- **(a) is the only option that changes a committed board, and it changes the one named defect** (`1216` F1).
  - With VDD up it puts every connector-facing signal on a structure that takes up to 5.5 V. That covers a 5 V-logic peer, and a "3.3 V" peer at +5 %, which TT does not.
  - It needs no new field. The off-board nets are already in the specification.
- **No escape field:** no committed board needs one. Sentinel-03's PE9 has a free FT alternative on its own channel, and a field adds surface no board uses.
- **PA8 over TIM4:**
  - **PA8:** moves one pin, with the same timer, channel, AF, timing and DMA. It is the solver's own choice on every other motor board.
  - **TIM4:** moves four pins and the timer.
  - **The price:** PA8 is MCO1's only pin, which `3400` reserved for E5. MCO2 on PC9 (still kept free) can output HSE as MCO1 can. If E5 needs MCO1 itself (HSI, LSE, HSI48 or `pll1_q_ck`), TIM4 on PD12–PD15 is the clean alternative (D3).
- **Test points named, not moved:**
  - a test point's far end is the operator's instrument, and each TT pin found is an MCU output;
  - moving them costs two acknowledgements, two generated crates and the `sentinel-01-bench` fabrication package, for a level nothing states;
  - (b) cannot bound a test point either.
- **(b) deferred:**
  - **No source:** none of its bounds has a held source today, so b-required refuses 33 boards for a figure nobody can supply (`INV-06`), and b-declared judges nothing.
  - **It is the only answer to the unpowered case,** so it earns its place when a real peer is chosen: Sentinel-03's ESCs, or a GPS or radio with a datasheet.
  - **Its finding will be a hardware one** (series resistance or a buffer), not a pin choice (§3 (c)).

**What it leaves unjudged:**
- every off-board end at VDD = 0 (named, by `1216` D2's ruling);
- test points;
- the bench header wiring (F2);
- NRST and BOOT0, which no option can move (F3);
- D2's unsolved connector pins (F4);
- a pull the generated firmware enables on an FT pin (note 3; neither A1 nor this spike reads firmware pull state, F6).

## 5. Adversarial pass (§5, targeted)

(Pending: filled from the reviewer's report, each finding checked at source.)

## 6. Predictions beside results

The full table is in `RESULTS.md` §5. 15 of 18 hold. Three do not:
- **Prediction 2:** 25 boards draw, not 26. D1 still declares `board.drawn: false`.
- **Prediction 4:** the drawn battery connector J4 reaches PF3 through R50 (F1). I had read only `connectors[]`.
- **Prediction 11:** the SPI move solves as predicted, but `validate` refuses two of the three boards on a stale acknowledgement. That cost was not predicted.

## 7. Decisions for the architect, and rows

**Decisions:**
- **D1 · The option.**
  - **Lean:** (a) at connectors.
  - **Alternatives:** (a) with test points; (c); b-declared alone.
- **D2 · Test points under (a).**
  - **Lean:** named, not moved.
  - **Alternative:** moved. That refuses 0 more, moves 3 boards, and costs 2 acknowledgements, 2 `boards/` crates and `fab/sentinel-01-bench/package`.
- **D3 · Sentinel-03's re-pin target, for the build.**
  - **Lean:** PA8, with E5's MCO on PC9 (MCO2).
  - **Alternatives:** TIM4 on PD12–PD15 (keeps every `3400` reservation, moves four pins and the timer); PD12 alone (two timers).
- **D4 · (b).**
  - **Lean:** deferred.
  - **Proposed row:** "a connector pin's off-board drive maximum (`connectors[].pins[].offboard.drive_max_millivolts`, a held source), judged by A1 powered and `1216b`'s rule unpowered; owner: the work order that declares a peer (Sentinel-03's ESCs, a GPS or radio). Expect its remedy to be hardware (series resistance, a buffer or a translator), since FT does not help at VDD = 0."
- **D5 · A stated-reason escape for a hand TT pin at an off-board end.**
  - **Lean:** none.
  - **Alternative:** a reason field beside each hand pin.
- **D6 · NRST and BOOT0 as data.** `power-parts.json` `io.families` holds FT and TT only. NRST (Table 13: FT's row, but its embedded pull-up holds it to VDD + 0.3 V by note 3) and BOOT0 (0–9 V) are needed only if (b) is built.
  - **Lean:** with (b), not before.

**Findings (rows for §7.1):**
- **F1 · The drawn battery connector is not a `connectors[]` pin.**
  - J4 on `sentinel-03` and `sentinel-02-power` is drawn from `power.battery`. J4.2 VBAT reaches PF3 through R50.
  - `1216b` judges the divider. (b) as shaped could never bound J4, and a reader of `connectors[]` alone misses it.
  - **Proposed row:** "every drawn off-board end is either a `connectors[]` pin or named by the field that draws it (`power.battery`), so one list covers them."
- **F2 · The bench's Nucleo header wiring is an off-board end no field declares.**
  - `sentinel-02-bench` and `-hse8` put IMU_SCK on PA5 (TT_ha, hand) on CN7.10, plus five more signals on CN7 and CN10 (`3304` bench §B4). The breakouts are on the MCU's own 3V3.
  - **Proposed row:** "declare a bench specification's header wiring as `connectors[]`, or name it in the specification, so the off-board rules see it."
- **F3 · NRST and BOOT0 face J2 and a test point on four boards.** No option can move them. NRST is held to VDD + 0.3 V with VDD up (note 3, embedded pull-up), which is TT's limit.
- **F4 · D2's three connector pins are unsolved** (`validate` refuses D2 by design). No option can judge them until D2 validates.
- **F5 · A re-pin of any pin named by `acknowledge_unwitnessed` fails `validate`** until the acknowledgement is re-written (`4230` R1). A solver preference that moves pins will hit this on every specification carrying one.
- **F6 · Note 3 and firmware pulls.** Table 13 note 3 holds any pin with an internal pull enabled to VDD + 0.3 V. A1 knows only the reset pulls (PA13, PA14, PA15, PB4). A pull the generated firmware enables on an FT connector pin (an RX line, say) is not in the solved topology, so no rule sees it.
  - **Proposed row:** "the solved topology names each pin's pull state, so A1 and any off-board rule judge note 3 on it."
- **F7 · `3400`'s kept-free pins are declared nowhere the solver sees.** Solved free, Sentinel-03's motors take PA8 (MCO1) and PA11 (USB DM). Only hand pins fence them, so a solver preference (a) can move a signal onto a pin a later work order needs.
  - **Proposed row:** "a declared reservation (`mcu.reserved_pins` with a reason) the solver honours."

## 8. Gates, fence, reduction pass

- **Gates not run.** This branch is spike-only: no crate, specification, `fab/`, workflow or `ci/` file changes. The WO says the gates are not run on it. `git diff --stat origin/main` lists only `spike/4300h/` and this file.
- **Fence:** `spike/4300h/` and this FEEDBACK only.
- **Reduction pass:** (pending).
- **Scratch:** `~/.cache/aet-4300h`. The release target, the solve and emit outputs and the move copies are removed at close; the renders are kept.
