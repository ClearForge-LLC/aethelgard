# AET-WO-4300h: adversarial pass on the spike's readers

Reviewer: adversarial subagent (Opus), 2026-11-22. Worktree `/home/forge/aethelgard-4300h` at `58d5825e`, read only. Every probe, copy and output is under `/home/forge/.cache/aet-4300h/adv/`.

The reference outputs are the committed `spike/4300h/out/*`. `adv/kit/out/` was overwritten by my re-judged runs; its kept copies are `adv/kit/pull/`, `adv/kit/unsolved/` and `adv/kit/both/`.

## Summary

| # | Finding | Severity | Changes a pin or a count? |
|---|---|---|---|
| F1 | Every motor-output pin carries an internal pull-down that the generated firmware writes, and the solved topology implies it. Table 13 note 3 then caps the pin at VDD + 0.3 V, FT or TT. `judge.py` uses 5499 mV for FT motor pins. | **changes a result** (under note 3 applied to generator-written pulls; RULES.md:24 excludes them) | Yes. j5-5v-esc `OffBoardDriveAboveTolerance` goes from 1 to 4 in b-declared, b-required, c-declared and c-required. `c-declared(MOTOR1 on PA8)` j5-5v-esc gains `OffBoardDriveAboveTolerance 4`. Moving PE9 to PA8 does not clear the powered case. |
| F2 | The battery input connector (`power.input`: J4 on `sentinel-03`, J1 on `sentinel-02-power`) reaches PF3 (FT_ha) through R50, 100 kΩ in series. Walk 1 never reads `power.input`. Walk 2 sees the path but files it as `via_rail`, and `compare.py` drops `via_rail`. | weakens a claim | It adds 2 rows to the pin list (PF3, FT_ha). No TT pin and no option count change. Prediction 4 ("no connector reaches an MCU pin through a series resistor") is false as worded. |
| F3 | Walk 1 silently drops test points on non-MCU nets (rails and GND). | cosmetic | No. |
| F4 | `compare.py` labels a walk-2 `series` row "walk 2 only", even when walk 1 lists the same (end, pin). | cosmetic | No. It does not fire on any committed board, only on a mutant. |
| F5 | Table 11 (p. 104) allows **no** injected current on PA4 or PA5 (−0/0; every other I/O is −5/+0). The spike cites only Tables 8, 10 and 13 for PA5. | weakens a claim | No. It is one more reason for (a) on PA5. |
| F6 | On the Nucleo bench, the PB3 move meets the ST-LINK SWO input (SB32, default ON). PG11 meets RMII TX_EN (SB27). | weakens a claim (bench only) | No. `sentinel-02-bench` is never judged by (a). |
| F7 | `alts.tsv` marks TIM13_CH1 (PF8) and TIM14_CH1 (PF9) FREE for a dshot600 output. DShot by DMA likely cannot run on TIM12, TIM13 or TIM14 (no DMAMUX request; RM0433 not held, so not verified). | weakens a claim | No. No move uses them. |
| F8 | RULES.md:24 says firmware-enabled pulls are not in the solved topology. They are, for `gpio[].pull` as well as for motor outputs (F1). | cosmetic, beyond F1 | No. No connector-facing GPIO input exists today. |
| F10 | Unsolved rows (D2's CMD_TX, CMD_RX and SERVO1) are refused `OffBoardDriveUndeclared` under b-required and c-required. RULES.md's last line says such a row is "named, not judged". | **changes a result** (counts follow from RULES.md as written) | Yes. In every b-required and c-required row: `OffBoardDriveUndeclared` is 3 lower (164, 136, 160, 90), `rows_named` is 70 not 67, and probe-vtref `boards_refused` is 17 not 18. |
| F9 | `rows_refused_by_kind` counts a row refused at both VDD up and VDD = 0 once under each kind, so the kinds do not add up to rows. | cosmetic | No. |

**Which rule each number follows.**
- F10's numbers follow from RULES.md as written. The fix moves the `unsolved` check in `option_b` (`judge.py:127`) ahead of the bound lookup (`:120`).
- F1's numbers come from my **amended** rule: note 3 is applied to the pulls the generators write. `judge.py` does follow RULES.md:23-24 as written there. F1 says that RULES.md's premise is false, not that the judge miscounts it.
- I found no ruling that excludes firmware-enabled pulls. I grepped the `1211e` FEEDBACK, the `1216` FEEDBACK and RESULTS, and `1216b`: the only note-3 ruling is `1216b` D4, which is Table 11's note 3, a different note.
- So the exclusion is the spike's own premise, and it turns out to be material.

The §7 stop does not fire. j5-5v-esc is a hypothetical sidecar, and no committed board declares or draws a peer above any limit. F1 does change what the lean can claim (see the end).

---

## F1: motor pins are pulled, so FT gives them nothing with VDD up (changes a result)

**What RULES.md says.**
- RULES.md:23 limits only the four reset-pull pins (PA13, PA14, PA15, PB4) to VDD + 0.3 V.
- RULES.md:24 reads: "A pull the generated firmware enables is not in the solved topology. That case is not judged here, and the same is true of A1."
- `judge.py:34` applies VDD + 0.3 only to `RESET_PULL`.

**What the engine does.**

1. The rule's own function is `crates/aethelgard-ir/src/silicon.rs:834-875`.
   - `motor_output_pupd()` returns `PUPDR_PULL_DOWN`.
   - `idle_pupd()` returns pull-up for an idle "high" and pull-down otherwise, DShot included.
   - `implied_pupd(t, net)` says the pull is implied by the solved topology for every `t.motor_outputs` key.
2. The native board-crate generator is `crates/aethelgard-svd-hal/src/board.rs:952-978` and `:1052` (`TimCh1..4 => (MODE_AF, 0, MOTOR_PULL)`).
   - I ran `aethelgard generate --spec crates/aethelgard-cli/tests/fixtures/sentinel-03.json --out adv/gen-s03`.
   - `gen-s03/board/src/board.rs:795` contains `r.write(TOKEN, Block::GpioE, 0x0c, 0x28880000);`. That is GPIOE_PUPDR with fields `10` (pull-down) on PE9, PE11, PE13 and PE14.
   - It is the only write to GPIOE `PUPDR` in the crate (grep), so the pull is never cleared after `start_outputs`.
   - **The ruling's build, MOTOR1 on PA8:** I ran `generate --spec adv/moves-scratch/sentinel-03--motor1-on-pa8-tim1-ch1-ft-fha-3400-kept.json`.
     - `gen-s03-pa8/board/src/board.rs:701` writes `r.write(TOKEN, Block::GpioA, 0x0c, 0x64020000);`. That is PA8's field `10` (pull-down), next to the debug pulls `0x6400_0000`.
     - Line 796 writes GPIOE `0x28800000`, which keeps PE11, PE13 and PE14 pulled down.
   - **The all-TIM4 variant:** `gen-s03-tim4/board/src/board.rs:795` writes GPIOD `0xaa000000`, which pulls down PD12, PD13, PD14 and PD15.
3. The ArduPilot path is `crates/aethelgard-hwdef/src/lib.rs:529`: `"{} {} {} PWM({}) {} PUSHPULL PULLDOWN"`.
   - The generated `adv/hwdef-serial-dshot-9600.dat:37-40` shows `PA8 TIM1_CH1 TIM1 PWM(1) SPEED_VERYLOW PUSHPULL PULLDOWN` and the same for PA9, PA10 and PA11.

**What the datasheet says.**
- **Table 13 note 3 (rendered p. 105):** "For operation with voltage higher than Min(VDD, VDDA, VDD33USB) +0.3V, the internal Pull-up and Pull-Down resistors must be disabled."
- **Table 10 note 4 (rendered p. 104):** "To sustain a voltage higher than 4V the internal pull-up/pull-down resistors must be disabled."

So with the generated firmware running, every motor-output pin is limited to VDD + 0.3 V = 3435 mV, whatever its Table 8 structure.

**Measured.** I copied the kit to `adv/kit/` and checked that `judge.py` reproduces `out/options.tsv` and `out/judged.tsv` byte for byte. Then I made `adv/kit/judge_pull.py`: the same rule, with note 3 also applied to every `motor_outputs` net and every `gpio` input with a pull, read from the scratch solve JSONs. The `diff` against the committed `out/options.tsv`:

```
< b-declared  j5-5v-esc  1 0 OffBoardDriveAboveLimitUnpowered 4,OffBoardDriveAboveTolerance 1  230 sentinel-03
> b-declared  j5-5v-esc  1 0 OffBoardDriveAboveLimitUnpowered 4,OffBoardDriveAboveTolerance 4  230 sentinel-03
< c-declared(MOTOR1 on PA8)  j5-5v-esc  1 0 OffBoardDriveAboveLimitUnpowered 4  31 sentinel-03
> c-declared(MOTOR1 on PA8)  j5-5v-esc  1 0 OffBoardDriveAboveLimitUnpowered 4,OffBoardDriveAboveTolerance 4  31 sentinel-03
(and the same 1 -> 4 on b-required, c-declared and c-required j5-5v-esc)
```

The new rows are in `adv/kit/pull/judged-pull.tsv`, for example `J5.1 MOTOR1 PA8 FT_fha refused OffBoardDriveAboveTolerance 5250 mV > 3435 mV`. No other scenario moves: the DS-009 sidecars bound no motor pin.

**Reach.** 37 connector or test-point rows sit on a motor output, on 9 solved boards:
- `sentinel-03`: J5.1–J5.4 and TP19–TP22;
- `motors-dshot600`, `motors-pwm`, `serial-dshot-9600`, `sentinel-02-shaped`, `hse-crystal-16`, `hse-external-8`: J*.1–4;
- `driver-judges-nothing`: J4.1–J4.4;
- `d1-drv8876`: J4.1.

`d2-drv8833`'s SERVO1 (unsolved) would be one more. For all of these, the `structure` column overstates the VDD-up limit.

**What it does to the options.**
- **(a) does nothing for a motor output while the pull-down is written.**
  - With VDD up, FT and TT both take 3435 mV.
  - At VDD = 0, both take 4.0 V (Table 10, with note 4 for a pulled FT pin).
  - The prediction "(a) moves PE9 to FT, which clears the powered case" (PREDICTIONS 17) is false.
  - The ruling's build, MOTOR1 on PA8, buys a 5 V peer nothing at J5.
- **(a) still holds for the SPI1 test points (PA5, PA7)**, which the generators leave FLOATING (`hwdef-sentinel-01.dat:28-30`, `PUSHPULL FLOATING`). Moving them to PB3 and PB5 really does raise their VDD-up limit to 5499 mV.
- **An option that wants FT to help a motor line also needs a pull change:** an external pull-down declared in place of the internal one, or the pull cleared at `start_outputs`.
- **A1 shares part of the gap.** `pull_tolerated` (`crates/aethelgard-ir/src/pin_devices.rs:1466-1474`) applies note 3 only to `MCU_RESET_PULL_UPS`/`_DOWNS`. It never applies it to the signal's own declared MCU pull (`signals[].pull.mcu`, solved as `gpio[].pull`).
  - Motor outputs are not A1's subject.
  - A pin-device input with a declared internal pull-up and an off-domain external pull would pass A1 on an FT pin above VDD + 0.3 V.

---

## F2: the battery connector reaches PF3 through a series resistor (weakens a claim)

- **Specs.** I scanned every universe spec for J/P/CN/BT/TP/JP/SB references outside `connectors[]` and `test_points[]`. Two show up, both `power.input`:
  - `sentinel-03`: J4, an XT30PW-M;
  - `sentinel-02-power`: J1, an XT30PW-M.
- **Walk 1.** `facing.py` reads `connectors[]` and `test_points[]` only, so these connectors are absent from walk 1 (the end-set diff between the walks, run by me).
- **Walk 2.** Walk 2 has the path:
  - `out/inward/sentinel-03.tsv`: `J4.2  VBAT  VBAT-R50(100k CRCW0603_F_K)-/VBAT_SENSE  13  PF3  FT_ha  via_rail`;
  - `out/inward/sentinel-02-power.tsv`: the same, on J1.2.
- **Where it is lost.** `compare.py` keeps only `direct`, `series` and `ic_bridge` from walk 2, so the row never reaches `compare.tsv`. `inward.py`'s docstring calls `via_rail` "an on-board domain's path". VBAT is not an on-board domain: it is the pack, off the board.
- **Why it matters.** This is a connector that reaches an MCU pin through a series resistor, which is §5's first named case. Prediction 4 says none exists.
- **Effect.** The pin list misses PF3 on 2 boards: FT_ha, LQFP144 pin 13, rendered p. 65 (also ADC3_INP5, so Table 10 note 5 caps it at 4 V while the ADC reads it).  No TT pin and no option count changes:
  - (b) as shaped bounds `connectors[]` pins only;
  - the pack's maximum is already declared (`battery.cells.max` × `cell_max_millivolts`);
  - `1216` judged the path: 2536 mV on PF3 (RESULTS.md:36), which `1216b` carries.
- **Fix.** Either list `power.input` ends in walk 1 as their own class, or say in RULES.md why a `via_rail` row from VBAT is out of scope.

## F3: test points on non-MCU nets disappear from walk 1 (cosmetic)

`facing.py`'s test-point loop adds a row only when `tp["net"] in pin_of`. Its docstring says "so no pin is silently dropped", and that covers connector pins only. The test points walk 1 drops:

| Board | Dropped test points |
|---|---|
| `sentinel-03` | TP23 (VDD_3V3, which reaches PB6, PB7, PB10 and PB11 through the I2C pull-ups, `via_rail`), TP28 (VDD_5V), TP29 (GND) |
| `sentinel-01` and `sentinel-01-ext-bmp581` | TP1 (GND), TP4 (VDD_3V3) |
| `sentinel-01-bench` | TP9 (VDD_3V3), TP10 (GND) |

Walk 2 lists all of them, so nothing reachable is lost. Only the claim in the docstring is wider than the code.

## F4: `compare.py` mislabels a series path (cosmetic)

- **Mutant.** `adv/mut-series.kicad_pcb` is a copy of `sentinel-03`'s board with J5.1 moved to a new net `/J5_1_EXT`, and R99 (2k2) placed between `/J5_1_EXT` and `/MOTOR1`.
- **Walk 2 catches it.** `inward.py` returns `J5.1  /J5_1_EXT  /J5_1_EXT-R99(2k2)-/MOTOR1  60  PE9  TT_ha  series`.
- **But the label is wrong.** Run on that mutant, `compare.py` (in `adv/kit2`) emits `sentinel-03  J5.1  PE9  connector TT_ha  series TT_ha  walk 2 only`.
  - The cause is `compare.py:52-59`: it gives "walk 2 only" to any row that is not all `direct` or all `ic_bridge`.
  - The intended label was presumably "series".

## F5: PA5 also has a zero injection limit (weakens a claim)

- **The cell.** Rendered p. 104, Table 11 `I_INJ(PIN)`: "Injected current on FT_xxx, TT_xx, RST and B pins except PA4, PA5: −5/+0"; "Injected current on PA4, PA5: −0/0" mA.
- **Who is exposed.** A probe on TP3 or TP10 (the `sentinel-01` family) or the bench breakout on CN7.10 (`sentinel-02-bench`) may not pull PA5 below VSS either. Negative injection on PA5 has zero margin.
- **Effect.** The spike's limits cover only the positive side (Tables 10 and 13). This changes no count, but it belongs in the case for (a) on PA5.

## F6: the bench alternatives are not free on the Nucleo (weakens a claim, bench only)

- **PB3.** UM2407 Rev 6, held at `~/.cache/aet-3304/ds/um2407-rev6.txt:2186-2189`: "SB32 ON: SWO signal of the STM32H7 (PB3) is connected to the ST-LINK SWO input". The bold extract `um2407-p28-34-bold.txt:300` marks SB32 ON as the default (`B ... ON`).
- **PG11.** `um2407-rev6.txt:2137`: "PG11 RMII TX enable, SB27 ON".
- **Effect.** The `moves.tsv` row "IMU_SCK on PB3 (FT)" for `sentinel-02-bench` puts SPI1 SCK onto the ST-LINK's SWO input on the real bench. That is an undeclared load, and it kills SWO trace. No committed-board count changes, because (a) does not judge `undeclared_offboard` rows.

## F7: TIM12/13/14 as DShot alternatives (weakens a claim, not verified on a held page)

- **What alts.py does.** It accepts every channel in `motor::TIMERS` (`crates/aethelgard-ir/src/motor.rs:659`, 12 timers including 12, 13 and 14), as the solver does (`solve.rs:1613-1623`, `channels_for` at 2259). That marks PF8 (TIM13_CH1) and PF9 (TIM14_CH1) FREE for MOTOR1 (dshot600).
- **The doubt.** As I recall RM0433's DMAMUX1 request table, TIM12, TIM13 and TIM14 raise no DMA request, so DShot by burst DMA could not run there. The repository holds no copy of that table: `stm32h753xx.h` has no DMAMUX request list, and 4300 FEEDBACK:33 says DMA is not modelled.
- **Effect.** These are "FREE to the solver", not usable for dshot. No move and no count uses them. Record it as an open question, not a fact.
- **The same question covers the moves the spike does make:** MOTOR1 on PA8 (TIM1) or PD12 (TIM4), and the all-TIM4 set.
  - As I recall, TIM1 and TIM4 both have an `_UP` DMA request (TIM4 has no CH4 request, but DShot uses UP burst), so I expect them to be fine. That is not verified either.
  - The engine does build the all-TIM4 variant: `generate` returns 0 (`adv/gen-s03-tim4.out`).
  - DShot's DMA is unmodelled (4300 FEEDBACK:33, P1.11), so no engine gate would catch a timer without one.

## F10: unsolved rows are refused under b-required and c-required (changes a result)

- **What RULES.md says.** Its last line: "A row whose pin is `unsolved` is named, not judged."
- **What the code does.** `option_a` follows it (`judge.py:88-91`). `option_b` checks for a bound first (`:120-126`) and only reaches `if r["pin"] == "unsolved"` at `:127`. So an unsolved row with no bound is refused under `required`.
- **The evidence** is in the committed `out/judged.tsv`:

```
b-required  none  J1.1  CMD_TX  unsolved  unheld  refused OffBoardDriveUndeclared  no drive bound
b-required  none  J1.2  CMD_RX  unsolved  unheld  refused OffBoardDriveUndeclared  no drive bound
b-required  none  J4.1  SERVO1  unsolved  unheld  refused OffBoardDriveUndeclared  no drive bound
```

It is the same in every scenario. Under probe-vtref, these 3 rows are D2's only refusals: 3 rows, from `awk` on the committed file.

- **Re-run.** `adv/kit/judge_unsolved.py` is `judge.py` with the unsolved check moved first. Its output is `adv/kit/unsolved/options-unsolved.tsv`. The diff against the committed `options.tsv` changes only the b-required and c-required rows:

  | Scenario | `OffBoardDriveUndeclared` | `rows_named` | `boards_refused` |
  |---|---|---|---|
  | none | 167 → 164 | 67 → 70 | 33 |
  | ds009-nominal and ds009-plus5 | 139 → 136 | 67 → 70 | 33 |
  | j5-5v-esc | 163 → 160 | 67 → 70 | 33 |
  | probe-vtref | 93 → 90 | 67 → 70 | **18 → 17** (D2 drops out) |

  The b-declared rows are unchanged: unsolved rows were already named there.
- **F1 and F10 together:** `adv/kit/both/options-both.tsv`.
- **Context.** D2 is unsolved only because `validate` refuses `PartLimitUndeclared`. With a placeholder limit, `adv/d2-probe.json` solves these rows onto FT pins (PB14, PA10, PA0). So under b-required, D2 would be refused anyway once it solves. The count still does not follow RULES.md as written.

## F8: RULES.md:24 is wrong beyond F1 too (cosmetic)

- **The record.** `SolvedTopology.gpio[].pull` exists. `w/solve/d1-drv8876.json` has `DRV_NFAULT {"pin":"PA11","direction":"input","pull":"none",...}`.
- **A probe.** My `adv/d2-probe.json` is D2 with a placeholder `input_pwm_max_hz`, used only to get past `validate`. It solves with `DRV_NFAULT` on PA11 and `"pull": "up"`.
- **Effect.** No committed board has a connector-facing GPIO input today, so no count changes.

## F9: refusals counted per kind, not per row (cosmetic)

`judge.py:138-142` notes one row twice when it fails both limits: J5.1 on PE9 under j5-5v-esc. `rows_refused_by_kind` is labelled "by kind", so this is not a defect, but the kinds' sum is not a row count.

---

## Checked and found clean

### Target 1: a missed facing pin
- **Reference prefixes on the 25 drawn boards:**
  - every off-board footprint is `J<n>` or `TP<n>`, and every other part is C, R, L, S, SW, U or Y;
  - no JP, SB, FB, D, RN or CN appears;
  - the `S` parts on `d1-24-i2c` and `d3-24-i2c-12-spi` are sensor ICs (BME280, TMP117, W25Q64).
- **End sets agree between walk 1 and walk 2** on every drawn board, apart from:
  - `J*.MP` (GND);
  - F2 (the battery connectors);
  - F3 (the test points);
  - the `sentinel-02-bench` CN rows (the sidecar, by design).
- **Translator.** The `sentinel-02-power` U13 TXU0304 B-side nets (`/U13_B*`) reach U5 only. No J or TP touches them.
- **ic_bridge rows.** All 29 are sensors or flash sharing a bus with a facing pin, already listed as `direct`.
- **Series detection.** A series R is caught by walk 2 (F4 mutant).
- **Boards that refuse or draw nothing.**
  - D2 with a placeholder limit (`adv/d2-probe.json`) solves CMD_TX to PB14 (FT_u), CMD_RX to PA10 (FT_u) and SERVO1 to PA0 (FT_a). No TT pin.
  - The 3 `solve/` fixtures put only SWD on connectors.
  - `invalid-zero-domain` declares no connector.
  - The 4 DrawsNoBoard boards have no connector outside `connectors[]`.
  - `connector_other` "other" rows exist only for DRV8876 and DRV8833 bridge outputs (motor terminals behind the driver IC).
- **Hand column.** No connector or test-point row has hand `-`, so none is misjudged as solver-chosen.

### Target 2: Table 8 and the limits, read on the rendered pages
I read every structure below on the rendered page, in the LQFP144 column and the I/O-structure column. All agree with `power-parts.json`.

| Page | Pins and structures |
|---|---|
| 64 | PE5, PE6 FT_h |
| 65 | PF3 FT_ha (LQFP144 pin 13) |
| 66 | PF6–PF9 FT_ha; PH0 FT; NRST RST |
| 68 | PA0 FT_a, PA1 FT_ha, PA2 FT_a |
| 69 | PA3 FT_ha, **PA5 TT_ha** (pin 41), PA6 FT_a |
| 70 | **PA7 TT_a** (43), PB0 FT_a, **PB1 TT_a** |
| 72 | **PE9 TT_ha** (60), PE11 FT_ha, PE13 FT_h |
| 73 | PE14 FT_h, PB10 FT_f, PB11 FT_f |
| 76 | PD12 FT_fh, PD13 FT_fh, PD14 FT_h, PD15 FT_h (TIM4_CH1–4 listed) |
| 79 | PC6–PC8 FT_h, PC9 FT_fh |
| 80 | PA8 FT_fha (TIM1_CH1), PA9–PA12 FT_u, PA13 FT |
| 82 | PA15 FT |
| 83 | PD7 FT_h (SPI1_MOSI) |
| 84 | PG11 FT_h (SPI1_SCK) |
| 85 | PB3 FT (SPI1_SCK), PB4 FT |
| 86 | PB5 FT (SPI1_MOSI), PB6 FT_f, PB7 FT_fa, BOOT0 B, PB8 FT_fh |
| 87 | PB9 FT_fh |

The limits, also on the rendered pages:
- **Table 13, p. 105:** TT_xx VDD + 0.3; BOOT0 0–9; FT `Min(VDD, VDDA, VDD33USB) + 3.6 V < 5.5 V`.
  - `judge.py` uses VDD alone. That is sound here: pads 33 (VDDA), 95 (VDD33USB) and 6 (VBAT) are on VDD_3V3 on all 25 drawn boards (read from each `.kicad_pcb`).
- **Table 10, p. 103:** TT 4.0; BOOT0 9.0; other pins 4.0; FT `Min(Min(VDD, VDDA, VDD33USB, VBAT) + 4.0, 6 V)`, which is 4.0 at VDD = 0.
- **The NRST and BOOT0 treatment** follows these pages.
- **The only gap** is note 3 for pulls the generators write (F1).

### Target 3: alternatives
- **Reserved pins.** `solve.rs:21` (`RESERVED_PINS`), plus HSE PH0/PH1, match `alts.py`.
- **Held pins.** The held set covers assignments, gpio and adc_inputs.
- **Each FREE pin carries the signal**, per its Table 8 alternate-function cell (above).
- **Timer channels and keys.** Channel-in-use and timer keys are respected. Every move in `moves.tsv` solves with only the named nets moved: no other key of the solve JSON differs.
- **I re-ran `moves.py`** in scratch (`adv/kit`, `adv/moves-scratch`). `moves.tsv` reproduces byte for byte.
- **Generator pulls on the moved SPI pins.** The hwdef generator writes FLOATING on UART, SPI, I2C and CS lines. That overrides ArduPilot's default UART PULLUP (held `~/.cache/aet-4405/adv/src/chibios_hwdef.py:394-399`), so the DS-009 scenarios are not hit by F1's mechanism.

### Target 4: the counts
- **Not clean in one place:** F10.
- **Reproduced.** `judge.py` reproduces `options.tsv` and `judged.tsv` byte for byte.
- **Arithmetic checks out:**
  - 167 connector, 53 test-point, 12 undeclared and 2 foreign-clock rows;
  - b-declared none named = 167 + 53 + 12 + 2 = 234;
  - (a) named = 12 + 2 + 3 unsolved (D2) = 17;
  - ds009 bounds 28, so 167 − 28 = 139;
  - j5 bounds 4, so 163;
  - probe-vtref bounds 74, so 93 and 160.
- **The sidecars.**
  - All 134 `drive.tsv` rows match a real `connector` row.
  - Every DS-009 connector row has a ds009 bound.
  - Every SWD, NRST and BOOT0 connector row has a probe-vtref bound.
- **Moves.** (a)'s `first_ft` and `move_result` pick PB3 and PB5, matching `moves.tsv`.
- **(c)** adds (a)'s kinds once per scenario.

## What F1 means for the lean
The motor lines are why Sentinel-03 is in this work order, and on them (a) changes nothing until the motor-pin pull policy changes. With VDD up, a 5 V peer on J5 exceeds 3435 mV on PA8 exactly as it does on PE9. Only (b) and (c) see that, and only with a declared bound. If the lean is (c) "with MOTOR1 on PA8", it has to say that the move helps the probe-facing SPI pins, not J5.
