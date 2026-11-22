# AET-WO-1211c — FEEDBACK: pin devices simulated, faulted and verified

**Branch:** `wo/aet-1211c-pin-devices-twin` from `main` `7fc81640`; `main` merged at `3e2b33a7` (`171f2a60`, docs) and `b5dd68a9` (`122378c7`, 2307). **Builder:** Claude (Opus 5.5 orchestrating; three Sonnet readers mapping the twin, the chaos crate and `verify`, one Sonnet builder for the chaos crate's records and overrides, one Opus adversarial reviewer; each report checked at source before use). Written 2026-11-22.

**Status:** DRAFT. See §7 for the open ruling (D7).

## 0. The answer

- **D1 simulates, and D2g ("D2", D3 ruled) simulates:** every witness of the MCU side of a pin device named, every unwitnessable field named (§2).
- **A `pin_held` on nFAULT is detected by a gate:** the run fails `pin_device_input_asserted`, the input's item in the `gpio` gate. By D5 (ruled) it counts toward no coverage target: no generated firmware reads the input, so the gate reads back the twin's own stimulus (`INV-07`). Holding an output is refused by name (`PinHeldIsOutput`).
- **No existing verdict moves.** The null sweep (§1) at `main` `122378c7`: every committed specification's tree and `simulate` verdict, and every committed scenario's campaign lines and records, identical, but for the version-moved fields and kind 24's two new fields on motor boards (trace 18).
- **The motor rail:** stopped (§1.1), ruled (ii): a pin device's supply alone is named in `no_device`, never met, never blocking.

## 1. The measurement (§1.1) and the stop

Predictions `spike/1211c/PREDICTIONS.md`, committed (`4254de68`) before any run; results `spike/1211c/RESULTS.md` (`4a49fb37`), outputs `spike/1211c/out/`. In short:
- **1211 §4's G and T rows on `main`'s Renode pin:** 112 of 112 value lines identical (P1a holds).
- **D1 and D2 under `simulate`, before this work order:** D1 `PinDeviceNotSimulated`; D2 as committed `PartLimitUndeclared`; D2g `PinDeviceNotSimulated`; `verify` on D1 `DrawsNoBoard`.
- **The motor rail:** with a baseline, a detected open RX line and a BOR-crossing droop, `VMOTOR` is the only unmet target on D1 and D2g; a droop on it is refused `DroopNotMcuSupply` and a rail out of band `RailOutOfBandRefused`. Five committed rails carry the same "no consumer a judge observes" reason; none feeds a pin device.
- **D1's own firmware, measured:** BSRR BR1 before `MODER` (P4b); nFAULT held low reads as an undriven input would, held high it survives the firmware's port setup (M4, M4g); ADC3's `DR` is the 12-bit code at 1.5 V (P4e missed: 1861). A `Read` watchpoint's `value` is 0, not the word read (H-probe, measured during the build).
- **Kit corrections after the predictions,** each named in RESULTS.md: the log comparer, two scenario fields, the coverage edit, M4's platform (the stock PWR stub parks the firmware at `BOOT_SUPPLY_NOT_DECLARED`; rerun on the twin's own platform).

**Stopped on the motor rail with lean (ii); ruled 2026-11-22T04:55Z with D2–D6** (§6).

## 2. What was built

### 2.1 The twin (`crates/aethelgard-twin`)
- **`pins::plan`**: from the solved topology and [`pins::Declared`] (§7, D7), each pin device's outputs (the declared level's `BSRR` bit, the port's `BSRR` and `MODER`), inputs (driven at the level the declared pull holds them at, or a scenario's hold; read at `IDR`), and ADC inputs (ADC1 → `adcM1S2`, ADC3 → `adc3`; set to the idle's value, 0 V for a current output of a bridged device, or a scenario's; the expected code from `aethelgard_chaos::record::adc_code`).
- **Refused by name, what the twin cannot run** (`PinDeviceNotSimulated`, with why): an input with no stated level or active level, an ADC input off ADC1/ADC3, at a resolution other than the measured 12 bits, with no stated idle value, or sharing its ADC with another input (the twin reads one conversion's code per ADC). `PinHeldIsOutput` (any declared output or peripheral pin), `PinHeldUnknown`, `AnalogValueUnknown`.
- **The platform** lists ADC1 and ADC3 for a pin device's input (`STOCK`, `solved()`); a `power.sense` input (no solved resolution) stays out, as before.
- **The image** holds a declared input at `MODER` 00, a pin device's ADC pin at 11, its ADC's clock gate, and `PCSEL` and `SQR1` (both stored by the model and read back).

### 2.2 The boot (`crates/aethelgard-cli`: `pinwitness.rs`, `renode.rs`, `anchor.rs`)
- **Stimuli** before the run, and again after each rendered reset: `gpioPortX OnGPIO n level`, `adcN SetVoltage µV ch`, the measured monitor forms.
- **Hooks:** each output's port's `BSRR` and `MODER` writes (`GPIO <address> <value> <before> <count>`), each ADC input's `DR` reads (`DR <input> <count>`); reads after the run: each input's `IDR`, each ADC input's `DR`.
- **Witnesses:** an output by its first `BSRR` word touching its pin (the declared level's bit alone) before the first `MODER` write making it an output; an input by its `IDR` bit (the driven level; at the declared active level, `pin_device_input_asserted`); an ADC input by the firmware's `DR` read and the code `DR` holds after it; a motor output idling at a static level by `witness_level` (still from its `CEN` anchor, or one period and a quantum after it for an idle high, Q10; kind 26 status `level`).
- **Findings** (`aethelgard_chaos::campaign`): `pin_device_input_asserted`, `gpio_mismatch`, `gpio_unwitnessed`, `adc_value_mismatch`, `adc_read_unwitnessed`, `motor_level_mismatch`; read once the boot reached its fixed point.
- **`simulate`** keeps `IdleHighNotSimulated` only where two of an idle high's periods exceed the boot; `generate` writes a pin device's `.repl`.
- **The verdict's `boot`** gains `gpio` and `adc`, and each static-level motor output a `level`, each absent when empty.

### 2.3 Chaos (`crates/aethelgard-chaos`; trace 18, scenario 5)
- **Overrides:** `pin_held {net, level}` (tag 8) and `analog_value {net, millivolts}` (tag 9, 0 to 3300 mV), scenario version 5 only, closed keys, duplicates refused.
- **Records:** kinds 33 (a GPIO signal as the twin drives it), 34 (its witness), 35 (an ADC input as set, with its expected code), 36 (its witness), 37 (a static level's witness); kind 26 gains status `level`. Replay joins each witness to its declaration and each override to its record (`PinHeldIsOutput` on a forged hold of an output), and holds every new finding's count to the records.
- **Kind 24 gains the declared idle and `resolution_ps`,** and replay re-judges kind 25 as `check_silicon` has since `1211a`. Found during the build: replay re-derived kind 25 from `pulse_min_us`, so any output whose idle is not its minimum pulse (D1's three) was refused before its archive was written. Every committed motor board's records gain these two fields and are otherwise identical (§3).
- The golden trace changed in its version byte only (17's hash pinned in the ladder).

### 2.4 `verify` (after `main` `122378c7`)
- **Per-item gates** from `simulate`'s line, present only on a board with a pin device: `gpio` (one item per GPIO signal), `adc` (one per ADC input), `bridge` (one per bridge: its inputs' static levels and its enable's write). Each item carries its verdict; `judged` counts the items (2306: never a whole input counted once).
- **Coverage:** `Target::Analog(net)`, met by a tolerated `analog_value` other than its idle's. Named in `no_device`, never met, never blocking, each with its reason: a motor rail (D1), a bridge (D2), an input (D5).
- **The ledger:** `PCSEL`, `SQR1` and `RES` observed; `ADCAL`, `JQDIS` and `SMP#` ignored by the model (unhandled tags, measured); the other ADC fields silicon-only with 1211 §4's reasons; a pin device's output `BSRR` observed by the GPIO witness. `twin_limits` gain `PIN.<net>`, `ADC.<net>` and `IDLE.<label>` rows, 1211 §4's fidelity notes as reasons.

## 4. Mutants (§1.5)

`spike/1211c/mutants/mutants.py`, one mutant per witness, fault kind, gate and refusal, each alone in a scratch export of HEAD: **32 of 32 killed** at `23e5431f` (`spike/1211c/out/mutants.txt`): 21 on the build's guards and 11 on the adversarial pass's fixes. Acted on along the way:
- **R4** (the motor rail ignoring a sourced domain) survived: no fixture reached the clause. Its regulator-output half is not in D1's ruled words (a regulated VMOTOR still feeds only pins nothing observes) and was cut; its load half is now tested on a constructed specification and killed.
- **R6** (a held input stressing as a target) was equivalent: `classify` treats every `pin_held` as intolerable and never reads its target. Replaced by a mutant on the hold's comparison with the pull's level, killed.
- **A7** (judging only the last `BSRR` word before `MODER`) survived F2's first fix, which judged every word: the pad is undriven until `MODER` (RM0433 Rev 9 p. 539), so only the last word before it decides the level. The rule became the last word, and A7 went.
- **W4** (no firmware read needed) became equivalent once F3 required a read after `ADSTART`; the redundant check and W4 were cut.

## 5. Adversarial pass (full)

A fresh Opus reviewer attacked `e0c7fae3` in its own worktree (`~/worktrees/aethelgard/1211c-adv`), with probes (`adv_*` tests, `~/.cache/aet-1211c/adv/probes.patch`) and three Renode mutants of D1's firmware. Report: `spike/1211c/out/adv/REPORT.md`. Each finding was checked at source before it was acted on.

| # | finding | done |
|---|---|---|
| F1 | replay never joined kind 26 `level`, kind 37, kind 24's idle and kind 26's anchor and edges: a live PWM relabelled `level` passed replay, archived too. The chaos tests' own fixture idled at a pulse and was judged by level | joined (`c4ae210c`): a level witness's output idles at a static level, its level is that idle, its settled instant is kind 26's anchor (plus a period and a quantum when high), its counts are within kind 26's, and witnessed it has no edge past its idle's and a period of window; the fixture idles low at CCR 0. Four tests, the forged pass refused archived |
| F2 | an output was witnessed at its declared level while the firmware set the other level before `MODER` (a second `BSRR` word; an `ODR` write): Renode mutants `twoset`, `odr` passed | the level is judged as of `MODER`: every `BSRR` word touching the pin before it carries the declared bit alone, and no `ODR` write precedes it (`ODR` hooked). Both mutants now fail `gpio_mismatch` (`spike/1211c/out/adv/*-sim-34a4.json`) |
| F3 | at the idle's 0 V the ADC's code is `DR`'s reset value, so a firmware that never converts passed (mutant `noconv`): 2306's defect | the conversion is witnessed: `ADSTART` written to its ADC's `CR` before the firmware's first `DR` read. `noconv` now fails `adc_read_unwitnessed`; D1's firmware passes |
| F4 | the `gpio` gate counted an input as judged on the twin's own stimulus, which D5 names `INV-07` | an input's item is `not_judged`, with D5's reason, outside `judged`; an asserted input still fails the gate by its finding. **Applied as the ruling's consequence; the architect may rule otherwise** |
| F5 | replay checked no pin across kinds | a pin device's pin is its own: not another signal's, an I2C line's, a chip select's, a motor output's, a serial pin's |
| F6 | replay accepted a `mismatch` whose reading was the declaration's (a forged failure) | an input's and an ADC input's mismatch must read otherwise |
| F7 | the plan took a signal's direction from the declaration, never the topology's, and fell back to the declared resolution | the declared and solved direction and resolution must agree, or `PinDeviceNotSimulated` names the disagreement |
| F8 | the null sweep was taken at `adeb3acd` and never runs `verify` | re-taken against `main` `122378c7` at `b5dd68a9` (§3); a final one at READY. `verify` is not swept whole (Kani, ERC, DRC on 27 boards): its new parts (`pin_gates`, coverage, the ledger rows) are absent by construction for a board with no pin device, which the reviewer's `adv_n` checked on all 27 specifications at library level, and D2g's `verify` shows them on a board with one |
| F9 | an idle high's settled window could be shorter than a period | a level is witnessed only with a period of window after settling, in the twin and in replay |
| F10 | `classify` called a hold the twin refuses `Baseline` | it changes the run; the campaign refuses it |
| F11 | with no `boot`, the `gpio` and `adc` gates vanish; nSLEEP counts in `gpio` and `bridge` | recorded: the first fails closed (`simulate`'s own gate fails); the bridge gate is one item per bridge as 1211 §2.7 (e) writes it, judging what `gpio` does not (the pair and its enable as a row) |
| F12 | a firmware `ODR` write clears a driven input (G4) | recorded; the generated firmware writes no `ODR`, and F2 now refuses an output's `ODR` write before `MODER` |

**Could not break:** `pin_held` by 19 other names (outputs, peripheral pins, supplies, references, case and space variants: `PinHeldIsOutput` or `PinHeldUnknown`, before any run); the stimuli after a real BOR reset on D2g; any judgement of a pull (G2); a move on a board without a pin device; ADC1's gate; `INV-07` but F4.
