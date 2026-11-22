# AET-WO-4300h — FEEDBACK: what a connector or test point can put on an MCU pin (design spike, STOPPED at §7 for the ruling)

**Branch:** `wo/aet-4300h-offboard-pins` from `main` `f46c3c42`.
**Builder:** Claude (Opus 5.5 orchestrating), with one Opus adversarial reviewer. The reviewer was given the code and §5, not my conclusions. Every finding was checked at source before use. Written 2026-11-22.

**Status: STOPPED at §7 for the ruling.** No crate, specification, `fab/`, workflow or `ci/` file changes.

**First, the finding that changes the work order's premise (adversarial F1, confirmed first-hand):**
- **Every motor output carries an internal pull-down once the firmware's first GPIO init has run.** It is written by both generators, by the ruling on PR #161 Q-G (`silicon::motor_output_pupd`). The generated Sentinel-03 crate writes GPIOE `PUPDR` = `0x28880000`, a pull-down on PE9, PE11, PE13 and PE14, and never clears it.
- **DS12117 Table 13 note 3** (p. 105): above VDD + 0.3 V "the internal Pull-up and Pull-Down resistors must be disabled". So **with the firmware running, every motor pin takes 3435 mV, whether FT or TT.**
- **Moving MOTOR1 from PE9 (TT) to PA8 (FT) changes its powered limit only from reset to that first write.** `1216` F1's row ("connector-facing signals on FT pins") does not protect J5 while the pull stays internal.

**The answer, in six lines:**
- **Every off-board end on the 34 committed boards is listed:** 236 rows on 33 boards (`spike/4300h/out/facing/`, `out/summary.md`). The two walks agree on 192 of 192 rows on the 25 drawn boards.
  - **Series parts:** one series path, the battery input connector through R50 onto PF3 (FT_ha, `1216b`'s divider).
  - **Translators:** none.
- **The TT pins:**
  - **at a connector:** one, Sentinel-03's MOTOR1 on **PE9 (TT_ha, a hand pin, pulled down)**, at J5.1 and TP19;
  - **at a test point:** PA5 (TT_ha) and PA7 (TT_a) on the three `sentinel-01` boards. The solver chose them, and they float;
  - **at an undeclared end:** the bench's Nucleo header (PA5, hand, `sentinel-02-bench` and `-hse8`). No option sees it.
- **MOTOR1's FT alternative on its own channel is PA8 alone.**
  - **PA8:** moving there changes that one pin and nothing else in the solved topology. It is the solver's own choice on every other board, and the pin `3400` reserved for MCO1.
  - **TIM4 on PD12–PD15** is the only whole-set move off reserved pins.
- **The limits:**
  - **VDD up:** TT and any pulled pin take 3435 mV; an unpulled FT pin 5499 mV. A "3.3 V" peer at +5 % is over 3435 mV by 30 mV.
  - **VDD = 0:** every pin takes 4.0 V.
- **Measured, with the motor pull internal as the engine writes it:**

  | option | refused | moved |
  |---|---|---|
  | (a) | 1 board (Sentinel-03) | 3 (test points only) |
  | (b), required | 33 | — |
  | (b), declared, today | 0 | — |
  | a 3.3 V +5 % peer declared on J5 | all four J5 pins, **PE9 or PA8 alike** | — |
  | the same peer, pull external | PE9 alone; **0 after the PA8 re-pin** | — |

- **Lean:**
  - **The pull policy comes first (D0):** move the motor pull-down to an external resistor.
  - **Then (a) at connectors,** with MOTOR1 re-pinned to PA8.
  - **If the pull stays internal:** no re-pin is worth its cost now, and J5 is answered by (b) when its peer is declared.
  - **Test points:** named, not moved (D2).
  - **(b):** deferred to the first work order that declares a peer with a held page.

## 0. How it was made

- **Commits:**
  - predictions, before any reader ran: `817ed9eb`;
  - the readers and the options as rules (`RULES.md`, `judge.py`, the sidecars), before `judge.py` first ran: `201b42d6`;
  - the measurement: `58d5825e`;
  - the first RESULTS and FEEDBACK draft: `e7322d74`, `d73665b6`;
  - the adversarial corrections (RULES "Corrections", the re-run, this file): the commits after.
- **Readers:** in `spike/4300h/`. They read the CLI's output of `main`'s crates (`solve` JSON, emitted `.kicad_pcb`), built from `f46c3c42` into scratch. They do not link the crates, so there is no `Cargo.toml` (as `1216` D6). The kit is listed in `RESULTS.md`.
- **Structures:** DS12117 Rev 10 Table 8, three reads compared on every listed pin: `power-parts.json` `io.pins`, `1211e`'s text read and `1211e`'s rendered-page read. PE9 was read here on the rendered page 72. The reviewer read every listed pin and every proposed alternative on its rendered page (pp. 64–87).
- **Limits:** Table 13 p. 105 with note 3 (VDD up, A1's reading) and Table 10 p. 103 (VDD = 0, `1216b`'s), read on the rendered pages.
- **The pulls:** `silicon::implied_pupd`'s definition, read from each solve, and the `PUPDR` word in a crate generated from Sentinel-03.
- **The solver's alternatives:** from its own pin data, parsed independently (`pintable.py`; 279 of 279 solved assignments found in it). Each move was run on a scratch copy (`moves.py`).

## 1. The measurement (§1.2)

Its one home is `spike/4300h/RESULTS.md` §1: the run, the reconciliation, the non-FT table with each pin's pull, and the far ends. What the options turn on:

- **The TT pins at an off-board end are all MCU outputs:** TIM1_CH1, and SPI1 SCK and MOSI.
  - A far end puts a level on them only by driving back: bidirectional DShot telemetry (not declared), a mis-wire, or an instrument.
  - PA5 also takes no injected current in either direction (Table 11 p. 104, "PA4, PA5 −0/0").
- **The far ends:**
  - **DS-009 ports** print +3.3V nominal with no maximum (`1216` F5). Every DS-009 pin is FT and unpulled.
  - **J5** declares no ESC (`esc: none`, "a bench header").
  - **Every other connector** declares its off-board capacitance, or nothing.
- **Internally pulled pins at an off-board end:**
  - **motor outputs:** every one, 37 rows on 9 boards;
  - **from reset:** PA13 and PA14 at every SWD end;
  - **NRST:** its embedded pull-up.
- **§5's paths:**
  - **Series:** the battery input connector (`power.input`: J4 on Sentinel-03, J1 on `sentinel-02-power`) reaches PF3 through R50, 100 kΩ. `connectors[]` does not list it, so (a) and (b) cannot see it; `1216b` judges it (2536 mV). The first run filed it under `via_rail` and missed it (F2 below).
  - **Translators:** none. The 29 `ic_bridge` rows are bus peers.

## 2. The alternatives (§1.3)

**MOTOR1 (TIM1_CH1, dshot600, Sentinel-03), each move run on a scratch copy:**

| move | solve | what moves in the solved topology | drawn board (segments, vias) | cost beyond that |
|---|---|---|---|---|
| **PA8** (FT_fha, p. 80) | 0 | MOTOR1's pin. Timer, channel, AF 1 and timing are unchanged | 1298 → 1364, 337 → 337 | takes `3400`'s MCO1 pin (E5, T7, deferred "with pins reserved", L268). MCO2 on PC9, also kept free, can output HSE as MCO1 can (RM0433 Rev 9 p. 391) |
| `pin: null` | 0 | the solver picks **PA8**, as on every other board it places TIM1_CH1 | as PA8 | as PA8 |
| **TIM4, PD12–PD15** (FT_fh/FT_h, p. 76) | 0 | all four pins; TIM1 → TIM4 (APB2 → APB1) | 1298 → 1460, 337 → 379 | no reserved pin. The firmware's timer, and the DMA a flight stack uses for it, change (DShot DMA is unmodelled) |
| PD12 alone (TIM4_CH1) | 0 | MOTOR1 on a second timer | 1298 → 1322, 337 → 357 | the motors span two timers |
| all four `null` | 0 | MOTOR1 → PA8, **MOTOR4 → PA11 (USB DM)** | — | `3400`'s reason for the hand pins |

- **Where TIM1_CH1 is:** on PA8 and PE9 only, on the LQFP144.
- **Whole-set moves:** no other timer has four free FT channels off `3400`'s kept-free pins.
  - TIM3 and TIM8 need PC9 (MCO2) for CH4.
  - TIM12, TIM13 and TIM14 are free to the solver. Whether DShot's DMA can run on them is unverified (adversarial F7).
- **Under either pull, the moved pins stay internally pulled while the pull is internal:** GPIOA `PUPDR` holds PA8's pull-down, and GPIOD holds the four TIM4 pins' (the reviewer's generated variants).
- **What a Sentinel-03 re-pin moves:** its fixture, and the tier-2 tests and scenarios that read it. It has no committed `fab/` or `boards/` output.

**PA5/PA7 on the `sentinel-01` family (SPI1):**
- **The FT alternatives:** PB3 or PG11 for SCK, PB5 or PD7 for MOSI, all free.
- **The move solves** and moves only those two pins.
- **But `validate` then refuses `sentinel-01` and `sentinel-01-bench`** (`UnwitnessedAcknowledgementStale`). Their `acknowledge_unwitnessed[5]` names `GPIOA.AFRL` `AFSEL5`/`AFSEL7` (`4230` R1).
- **It also moves** `boards/sentinel-01`, `boards/sentinel-01-bench` and `fab/sentinel-01-bench/package`.
- **These pins float,** so the move would raise their powered limit from 3435 to 5499 mV. It is the one place (a) raises a powered limit today.
- **On the Nucleo bench,** PB3 meets the ST-LINK SWO (SB32 ON) and PG11 meets RMII TX_EN (SB27), from UM2407 Rev 6.

## 3. The options (§1.4), each with its cost and its measured refusals

The rules are in `spike/4300h/RULES.md` and `judge.py`. They were committed before they ran, and corrected after the adversarial pass (RULES "Corrections"). The results are in `out/options-internal.tsv` and `out/options-external.tsv`, with every row in `out/judged-*.tsv`.

**The limits** (RESULTS §3):

| | unpulled TT | unpulled FT | pulled (note 3): motor outputs, PA13/PA14, NRST | BOOT0 |
|---|---|---|---|---|
| VDD up (Table 13, VDD 3135 mV) | **3435 mV** | **5499 mV** | **3435 mV** | 9.0 V |
| VDD = 0 (Table 10) | 4.0 V | 4.0 V | 4.0 V | 9.0 V |

### D0 first: the motor-pull fork (PR #161 Q-G)

- **`internal` (the engine as it is):**
  - every motor line is held to 3435 mV powered, on any pin;
  - the pull holds the line low only from the first GPIO init. From reset to then, the pin floats (Table 7: "all I/Os are set as floating inputs during and after reset"; RM0433 p. 542 resets PORTE's `PUPDR` to 0).
- **`external` (a declared resistor instead, no internal pull):**
  - an FT motor pin takes 5499 mV powered;
  - the line is held low from power-up, not from the first init;
  - A1 judges the resistor as a declared pull.
- **Cost of `external`:**
  - a pull field on `motor_outputs` (the schema has none today);
  - both generators stop writing the motor `PUPDR` and hwdef's `PULLDOWN`;
  - `implied_pupd` and the pinout check follow;
  - 35 motor nets on 9 boards gain a resistor, with their goldens;
  - it reverses a ruling.

### (a) The solver prefers FT at an off-board end; a hand TT pin there is refused unless the specification states why

- **Refuses 1 / 34**, Sentinel-03 (`OffBoardPinNotFt`, J5.1 and TP19). No field states why.
- **Moves 3 / 34**, the `sentinel-01` family's test points to PB3/PB5. All three solve; two need their acknowledgement re-written, and the third moves a fabrication package.
- **Restricted to connectors:** refuses the same 1 and moves **0**.
- **What it buys:**
  - **`internal`:** nothing on J5 with the firmware running. A 3.3 V +5 % peer declared on J5 is refused on all four pins with MOTOR1 on PE9 or on PA8. It buys only the reset-to-init window on MOTOR1, and the powered limit of the floating test points if they move.
  - **`external`:** the powered case on J5 for any peer up to 5.5 V. The same 3.3 V +5 % peer is refused on PE9 alone, and on nothing after the PA8 re-pin.
- **What it cannot judge:**
  - anything at VDD = 0, where FT and TT are equal;
  - NRST and BOOT0, which cannot move;
  - the bench header, the foreign clock and the battery input, which no `connectors[]` or `test_points[]` entry declares.
- **Cost:**
  - the solver: off-board-facing nets take FT candidates first (the nets are already in the specification);
  - validation: one refusal for a hand pin;
  - with no escape (D5), no new field;
  - `CHANGES` and README rows, tests;
  - the Sentinel-03 re-pin (D3);
  - **with test points:** the `sentinel-01` family's costs above (D2).

  **On every other board, `solve` is byte-identical.**

### (b) A declared off-board drive bound per connector pin (`connectors[].pins[].offboard.drive_max_millivolts` with a source), judged by A1 with VDD up (note 3 included) and by `1216b`'s rule at VDD = 0

- **Today no specification declares a bound:**
  - **b-required** refuses **33 / 34** (164 rows);
  - **b-declared** refuses **0**.
- **On hand sidecars** (`sidecars/drive.tsv`, each marked nominal or hypothetical):

  | sidecar | refused |
  |---|---|
  | DS-009 3300 mV nominal, or 3465 mV | 0 |
  | a debug probe tracking VTref | 0 |
  | **a 3.3 V +5 % peer on J5** | **all four pins powered (`internal`)**; PE9 alone (`external`) |
  | **a 5 V peer on J5** | all four powered (`internal`) or PE9 (`external`), and **all four unpowered under either** |

- **The only option that judges the J5 hazard under `internal`, and the only one that judges any unpowered case.** It does so only where a bound is declared.
- **What it cannot judge:** test points (not `connectors[]` pins), the bench header, the foreign clock and the battery input.
- **Its bounds have no source today.** DS-009 prints a nominal, and J5's peer is undeclared, so every (b) refusal above rests on a hypothetical sidecar.
- **Cost:**
  - a field and a source on `offboard` (specification 26 or later);
  - Table 13's NRST and BOOT0 rows as data beside `1211e`'s FT/TT families (D6);
  - a judge that reuses A1's limit with note 3 on the implied pulls, and `1216b`'s limit;
  - `CHANGES`, README, tests.

### (c) Both

- **Today:** declared refuses 1 (a's) and moves 3; required refuses 33.
- **After the ruling's re-pin (MOTOR1 on PA8), declared refuses 0.**
- **With a peer declared on J5:**
  - a 3.3 V +5 % peer: all four pins refused under `internal`, none under `external`;
  - a 5 V peer: all four refused unpowered under either. **Only hardware clears that:** a buffer or translator powered from `mcu.supply`. Table 11 admits no positive injection, so a series resistor alone does not.
- **Cost:** (a)'s and (b)'s together.

## 4. The lean

**Lean: D0 `external` first; then (a) at connectors with no escape field, and MOTOR1 re-pinned to PA8; test points named, not moved; (b) deferred to the first work order that declares a peer with a held page. If D0 stays `internal`: no re-pin now, and J5 is answered by (b) when its peer is declared.**

**Reasons:**
- **D0 `external`:** it is the only branch in which a pin's structure protects J5 with the firmware running, which is what `1216` F1's row asks for.
  - It also does what Q-G's pull-down was for, holding the motor line low, from power-up instead of from the first GPIO init.
  - It costs a cross-cutting build (§3 D0), and it reverses a ruling. Under `internal`, a "3.3 V" peer at +5 % is already over the pin on paper.
- **(a) at connectors, given `external`:**
  - It changes the one named defect and needs no new field.
  - The 3.3 V +5 % peer then clears on PA8. A 5 V peer clears powered, and only hardware clears it unpowered.
- **No escape field:** no committed board needs one.
- **PA8 over TIM4:**
  - **PA8** moves one pin: same timer, channel, AF, timing and DMA. It is the solver's own choice everywhere else.
  - **TIM4** moves four pins and the timer.
  - **PA8's price** is MCO1's only pin, which `3400` reserved for E5. MCO2 on PC9 (still free) can output HSE. If E5 needs MCO1 itself (HSI, LSE, HSI48, `pll1_q_ck`), take TIM4 (D3).
- **Given `internal`, (a) is not worth a re-pin now.** It would refuse Sentinel-03 for a move that changes its powered limit only before the first GPIO init. The J5 question is the peer's level, which only (b) judges.
- **Test points named, not moved:**
  - a test point's far end is the operator's instrument, and each TT pin found is an MCU output;
  - moving them costs two acknowledgements, two generated crates and the `sentinel-01-bench` fabrication package.
  - **Against that:** they float, so a move raises their powered limit to 5.5 V, and PA5 takes no injection at all. If that package is not yet ordered, now is the cheapest time to move them. That is D2.
- **(b) deferred:** none of its bounds has a held source. b-required would refuse 33 boards for a figure nobody can supply (`INV-06`), and b-declared judges nothing. It earns its place when a real peer is chosen (Sentinel-03's ESCs, a GPS or radio). Its finding for a 5 V peer will be a hardware one.

**What it leaves unjudged:**
- every off-board end at VDD = 0 (named, by `1216` D2's ruling);
- test points;
- the bench header (F3);
- the battery input under (a)/(b) (`1216b` judges it);
- NRST and BOOT0;
- D2's unsolved pins (F5);
- the reset-to-init window under `internal`.

## 5. Adversarial pass (§5, targeted)

**The reviewer:** an Opus agent given the code, the WO and §5, not my conclusions. Its report and probes are in `~/.cache/aet-4300h/adv/`. Nothing in the worktree was touched.

**Reproduction:** `judge.py` and `moves.py` reproduced byte for byte.

**Clean:**
- **Table 8:** every TT pin and every proposed FT alternative, read on its rendered page (pp. 64–87). The limits on pp. 103–105.
- **The sidecars:** all 134 rows match a real connector pin.
- **No missed end on any drawn board:**
  - no JP, SB, FB, D, RN or CN reference;
  - every `ic_bridge` is a bus peer;
  - the TXU0304's B side reaches U5 only.
- **D2, solved with a placeholder limit:** its connector pins land on FT (PB14, PA10, PA0).
- **Its arithmetic, checked line by line.**

**Findings, each checked at source, and what was done:**

| # | finding | severity | done |
|---|---|---|---|
| F1 | Every motor output is internally pulled down after the first GPIO init (Q-G; GPIOE `PUPDR` `0x28880000`, read first-hand); note 3 caps it at 3435 mV, FT or TT. RULES had said firmware pulls were not in the solved topology | **changes the result and the lean** | walk 1 carries `fw_pull` (by `implied_pupd`); `judge.py` applies note 3 and runs both branches; the lean is rebuilt around D0 |
| F10 | Unsolved rows were refused under `required`, against RULES' last line | changes counts (167 → 164; probe-vtref 18 → 17 boards) | fixed; re-run |
| F2 | The battery input connector reaches PF3 through R50, filed as `via_rail` and dropped | weakens a claim (prediction 4 wrong) | walk 1 lists `power_input`; compare reconciles it (192/192) |
| F5 | Table 11: PA5 takes no injection either way | weakens a claim | cited (§1, D2) |
| F6 | On the Nucleo, PB3 meets SWO (SB32) and PG11 RMII TX_EN (SB27) | bench only | cited (RESULTS §2); checked in UM2407 |
| F7 | TIM12/13/14 FREE to the solver may lack DShot DMA | open, not verified | noted |
| F3, F4, F8, F9 | test points on rails not listed by walk 1 (walk 2 lists them); a series row labelled "walk 2 only"; firmware GPIO pulls (folded into F1); per-kind counts | cosmetic | F3 docstring, F4 label fixed; F9 noted in RESULTS |

**Not adopted, with reason:** the reviewer's side note that A1 would pass a pin-device input with both an internal MCU pull and an off-domain resistor. Validation refuses that shape: `pull` "states exactly one of `mcu` and `resistor`" (`pin_devices.rs` L1153, the open-drain branch). I did not probe the other pin kinds, so this is not claimed either way.

**§7's stop** was re-checked at the corrected limit. It does not fire: no off-domain on-board source lands on any of the 35 pulled nets (RESULTS §6).

## 6. Predictions beside results

The full table is in `RESULTS.md` §5. 13 of 18 hold, and one is incomplete. The others:
- **Prediction 2:** 25 boards draw, not 26.
- **Prediction 4:** the battery input reaches PF3 through R50.
- **Prediction 11:** the move solves, but there is a stale-acknowledgement cost.
- **Prediction 13:** note 3 also covers the motor outputs.
- **Predictions 16 and 17:** under the engine's pull, J5 is refused on all four pins powered, and PA8 clears nothing powered.

**F1 is the miss that matters.** I took the generators' pulls to be outside the solved topology without reading `silicon.rs`.

## 7. Decisions for the architect, and rows

**Decisions:**
- **D0 · The motor-line pull** (PR #161 Q-G).
  - **Lean:** an external, declared resistor; no internal pull.
  - **Alternative:** keep the internal pull-down. J5 is then a ≤ 3435 mV header, powered, on any pin.
- **D1 · The option.**
  - **Lean:** given D0 `external`, (a) at connectors; given `internal`, (b) at the work order that declares J5's peer.
  - **Alternatives:** (a) with test points; (c); b-declared now.
- **D2 · Test points under (a).**
  - **Lean:** named, not moved.
  - **Alternative:** moved, which raises PA5/PA7's powered limit (they float) and costs 2 acknowledgements, 2 `boards/` crates and `fab/sentinel-01-bench/package`. The cheapest moment is before that package is ordered.
- **D3 · Sentinel-03's re-pin target** (if D1 orders one).
  - **Lean:** PA8, with E5's MCO on PC9 (MCO2).
  - **Alternatives:** TIM4 on PD12–PD15 (every `3400` reservation kept; four pins and the timer move); PD12 alone (two timers).
- **D4 · (b).**
  - **Lean:** deferred.
  - **Proposed row:** "a connector pin's off-board drive maximum (`connectors[].pins[].offboard.drive_max_millivolts`, a held source), judged by A1 powered, with note 3 on every pull `implied_pupd` gives, and by `1216b`'s rule unpowered. Owner: the work order that declares a peer (Sentinel-03's ESCs, a GPS or radio). A 5 V peer's remedy is hardware (a buffer or translator on `mcu.supply`); Table 11 admits no positive injection."
- **D5 · A stated-reason escape for a hand TT pin at an off-board end.**
  - **Lean:** none.
- **D6 · NRST and BOOT0 as data.** `power-parts.json` `io.families` holds FT and TT only. NRST (Table 13: FT's row, but its embedded pull-up holds it to VDD + 0.3 V by note 3) and BOOT0 (0–9 V) are needed only if (b) is built.
  - **Lean:** with (b).

**Findings (rows for §7.1):**
- **F1 · A motor line floats from reset to the first GPIO init.** Q-G's pull-down is written in the first init (RM0433 p. 542 resets PORTE's `PUPDR` to 0; Table 7: floating "during and after reset"). An ESC sees an undriven line until then.
  - **Proposed row,** if D0 stays `internal`: "the motor line held low from power-up (an external pull-down), or the window named in the ledger."
- **F2 · The drawn battery input connector is not a `connectors[]` pin.** J4 (`sentinel-03`) and J1 (`sentinel-02-power`), `power.input`, reach PF3 through R50. `1216b` judges the divider. A reader of `connectors[]` alone misses it, and (b) as shaped could never bound it.
  - **Proposed row:** "every drawn off-board end is a `connectors[]` pin or named by the field that draws it (`power.input`), so one list covers them."
- **F3 · The bench's Nucleo header wiring is an off-board end no field declares.** `sentinel-02-bench` and `-hse8` put IMU_SCK on PA5 (TT_ha, hand) at CN7.10, plus five more signals on CN7/CN10 (`3304` bench §B4). The breakouts are on the MCU's own 3V3.
  - **Proposed row:** "declare a bench specification's header wiring as `connectors[]`, or name it, so the off-board rules see it."
- **F4 · NRST and BOOT0 face J2 and a test point on four boards.** No option can move them. NRST is held to VDD + 0.3 V powered (note 3, embedded pull-up).
- **F5 · D2's three connector pins are unsolved** (`validate` refuses D2 by design). Solved with a placeholder limit, they land on FT.
- **F6 · A re-pin of any pin named by `acknowledge_unwitnessed` fails `validate`** until the acknowledgement is re-written (`4230` R1). A solver preference that moves pins hits this on every specification carrying one.
- **F7 · `3400`'s kept-free pins are declared nowhere the solver sees.** Solved free, Sentinel-03's motors take PA8 (MCO1) and PA11 (USB DM). Only hand pins fence them, so (a)'s preference could move a signal onto a pin a later work order needs.
  - **Proposed row:** "a declared reservation (`mcu.reserved_pins` with a reason) the solver honours."
- **F8 · DShot DMA per timer is unmodelled.** The solver offers TIM12–14 channels for a DShot output, and whether those timers raise a DMA request is not held (adversarial F7). A move to TIM4 (D3) rests on the same unverified assumption for TIM4_UP.

## 8. Gates, fence, reduction pass

- **Gates not run.** This branch is spike-only: no crate, specification, `fab/`, workflow or `ci/` file changes, and the WO says the gates are not run on it. `git diff --stat` against the merge base `f46c3c42` lists only `spike/4300h/` and this file (85 files).
- **Fence:** `spike/4300h/` and this FEEDBACK only.
- **Reduction pass (diff):** −5 net · 2 duplicates removed · 0 dead paths removed · 0 protected kept · 1 finding below.
  - **The duplicates:**
    - `judge.py`'s copy of the reset-pull pins, now read from walk 1's `reset_pull`/`fw_pull` columns;
    - `judge.py`'s own `power-parts.json` loader, now PA8's structure from `alts.tsv`.
  - **Prose:** FEEDBACK §1 now cites `RESULTS.md` §1 instead of restating its table.
  - **The proof:** the cuts went in with the F1/F10 corrections, so no pre-cut baseline of the corrected rule exists. Instead:
    - the corrected `judge.py`'s refusals and moves (columns 1–5 of `options-internal.tsv`, less the new `j5-3v3-plus5` sidecar) equal the reviewer's independently amended judge (`adv/kit/both/options-both.tsv`) line for line;
    - a full re-run reproduces every output (`diff -r` empty);
    - `pyflakes` finds nothing unused.
  - **The finding (kept):** the four reset-pull pins are in `facing.py` and `alts.py` both. `alts.py` judges candidate pins walk 1 never lists, so it cannot read them from walk 1.
- **Scratch:** `~/.cache/aet-4300h`. The release target, the solve and emit outputs and the move copies are removed at close. The renders and the adversarial report are kept.
