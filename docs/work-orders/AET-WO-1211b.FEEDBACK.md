# AET-WO-1211b — FEEDBACK: pin devices generated (idle words, GPIO and ADC bring-up, bridge enables after `CEN`, Kani's oracles, the ledger's classes)

**Branch:** `wo/aet-1211b-pin-devices-gen` from `main` `25203c58`; `main` merged at `349c75f5` (4240c, 1213), then at `ddfa64d6` (4240b, solved 15), then renumbered to solved 16. **Builder:** Claude (Opus 5.5 orchestrating; one Opus reader for the ADC contract, one Opus adversarial reviewer, each report checked at source before use). Written 2026-11-20.

**Status: READY.** Built, merged with `main` `ddfa64d6`, solved 16; the package committed byte for byte from the architect's write=true dispatch, run 44976609648 (§9). **The answer, in six lines:**
- **D1 generates, builds and proves.** Every harness of its board crate proves in the tools image (10; the slowest `every_valid_sequence_tim1`, 48 s, which now holds the bridge's enable; ADC3's bring-up 4 s); its ELF builds. A tier-2 test runs it in CI. D2 stays refused at validate (`PartLimitUndeclared`).
- **No existing byte moves.** All 25 committed specifications that generate on `main` generate byte-identical trees, against a baseline re-taken at each `main` merge (§1).
- **The generator writes:** a GPIO output's declared level before `MODER`; an input's declared pull; a motor pad's pull from its idle (Q9); a pin device's ADC1/ADC3 inputs enabled, calibrated and converted once (§2.3); a bridge's enable after its timer's `CEN`, taking `&Tim<Running>`, with the row's `t_wake` waited only when its declared level wakes the part (§2.4). `PSC` and the idle `CCR` were already the solved words (1211a); the proof's expected `CCR` now comes from the declared idle (`oracle_idle_word`).
- **Kani's oracles come from the specification:** the idle words, the GPIO levels and pulls (`silicon::implied_pupd`), the ADC's words, and the bridge's enable after `CEN` as a rule of the proof's image, independent of the typestate. The bridge's static idle is judged at generate time against the part's truth table (`BridgeIdleNotSafe`).
- **`generate` accepts a pin device and an idle high;** `simulate` refuses both (`PinDeviceNotSimulated`, `IdleHighNotSimulated`) until 1211c, `emit` refuses a pin device by eda's `PinDeviceNotDrawn`, and a pin device's tree carries no `.repl` (S2, as ruled).
- **Mutants: 21 of 21 killed**, including the work order's four (§4).

## 0. How it was made

- **Predictions first:** `spike/1211b/PREDICTIONS.md` (B1–B6), committed (`e34b6677`) before any build line, with the kit and `main`'s baseline rows.
- **The measurement:** `spike/1211b/kit/sweep.sh` in the tools image (`img.sh`, capped-run): validate, solve, generate and build every committed specification with `main`'s binary; `base-rows-25203c58.tsv` holds each exit and ELF sha256. After each generator commit, a host diff of every generated tree against the baseline (`gen-diff.sh`); the baseline was re-taken with `main`'s binary at each merge.
- **The ADC contract:** an Opus reader wrote the register contract from RM0433 Rev 9 ch. 25 and DS12117 Rev 10, page by page (`spike/1211b/adc-contract.md`); two of its citations were checked at source before use (the revision V /2, p. 1039; the `RES` codes, p. 1013).
- **Scope forks stopped and ruled:** S1 (the topology's fields), S2 (no `.repl`), S3 (`power.sense`), Q-A (calibration), Q-B (poll bounds), all ruled 2026-11-20 (§3).

## 1. The measurement (§1.1)

| run | specifications | generate | trees | ELFs |
|---|---|---|---|---|
| `main` `25203c58`, image (`kit/sweep.sh`) | 31 | 25 generate and build; 5 refused before generate; D1 refused `PinDeviceNotGenerated` | baseline | baseline (`base-rows-25203c58.tsv`) |
| branch after each generator commit, host (`kit/gen-diff.sh`) | 31 | 25 + D1 | **25/25 identical** at every commit | — |
| branch at `3132793a` (`main` `349c75f5` merged), host, against a baseline re-taken with `349c75f5`'s binary | 31 | 25 + D1 | **25/25 identical** | — |
| branch at `2f0f3496` (`main` `ddfa64d6` merged, solved 16), host, against `ddfa64d6`'s binary | 33 | 27 + D1 | **27/27 identical** | — |
| the same two heads in the image (`kit/sweep.sh`, generate and build) | 33 | 27 + D1 | — | **27/27 ELFs identical**; D1 builds (`05720792…`) |

- **The first build moved two trees** (B1's premise was wrong, `PREDICTIONS.md`'s correction): `sentinel-03.json` and `sentinel-02-power.json` carry a `power.sense` input in `adc_inputs`, and the generator configured its pin. It now brings up a pin device's inputs only (S3, as ruled).
- **No stored topology moved** beyond the new fields and the version line: the only topology literal with a pin device is hwdef's test, which gains `"bridges": {}`; the renumber (`spike/4240a/kit/renumber.py . 23 23 15 16`) moved 5 stored topologies' version lines and 4 Rust pins; no committed specification byte moved.
- **The solved bump's pins, by measurement:** `solve_cli`'s two digests, which with version 15 put back hash to `main`'s exactly; the refused-version list (15 is now the file before `bits` and `bridges`); both READMEs' version lines. No generated file and no ELF moved: none carries the solved version.
- **A merge defect, found by the image sweep:** after `ddfa64d6`, every RCC enable takes 4240b's `Scale2` token, and the ADC's did not (D1's firmware failed to build; fixed in `2f0f3496`, D1 builds and proves again). The host suites cannot see it (they do not compile the firmware); the tier-2 test does.

## 2. What was built (§1.2–§1.4)

### 2.1 GPIO
- An output's `BSRR` write is its declared level's bit, set for `high` and reset for `low`, before `MODER` = 01 (RM0433 Rev 9 p. 534). D1's nSLEEP (PA1, `low`): `0x00020000`, BR1. A chip select is `high`, as before.
- An input is `MODER` 00 alone (no speed, no alternate function), its declared pull in the port's whole `PUPDR`. D1's nFAULT (PA11) is `none` (the 10 kΩ is external).
- Typestates `Input` and `Analog`, emitted only where a pin takes one. `check speeds` says why it judges neither (`GPIO_INPUT`, `ANALOG_INPUT`).
- **Oracle:** the proof's `PUPDR` is `silicon::implied_pupd`'s (extended to a declared input pull: **the one IR oracle helper this work order changes**, named here). An output's ready clause takes the declared level's `BSRR` word (`oracle_level_bit`, from its own copy of `solved.gpio`). The final state holds a pin device's output at its declared level; a chip select keeps its both-ways exercise.

### 2.2 The motor pad and the idle words
- The pad's pull follows the idle (Q9): the generator's `motor_pull` (pull-up `01` for `high`, else `10`), the oracle's `implied_pupd`.
- `PSC` and the idle `CCR` were the solved words already (1211a's solver: `PSC` from `resolution_ps`, `CCR` 0 / ARR + 1 / the pulse). **The proof's expected `CCR` now comes from the declared idle** (`oracle_idle_word`), not the topology's copy; DShot keeps its solved word.

### 2.3 ADC1/ADC3 (`board/adc.rs`; contract `spike/1211b/adc-contract.md`)
- Per ADC with a pin device's input: `ADCx_CCR` = CKMODE 11 (adc_sclk / 4; `ADCSEL` resets to `pll2_p_ck`, p. 419, which no firmware starts); `CR` 0 (out of deep power-down), then `ADVREGEN` 1 and `BOOST` 01; tADCVREG_STUP as a fixed count of reads (650); `CFGR` (`JQDIS`, `RES` from the declared bits); `PCSEL`, `SMPR` (810.5 cycles), `SQR1`; linearity calibration (`ADCALLIN`, then `ADCAL`, polled); `ISR.ADRDY` cleared, `ADEN` read-modify-write, `ADRDY` polled; `ADSTART` read-modify-write; each conversion's `EOC` polled and `DR` read.
- **Every setting holds on both silicon revisions.** fADC is 15.6–16.2 MHz (rev Y, 36 MHz maximum) or 7.8–8.1 MHz (rev V, 12.5 MHz with `BOOST` 01). `RES` 010 means 12 bits on both. **8 bits has no common code** (100 rev Y, 111 rev V) and is refused by name (`AdcResolutionNotCommon`).
- **Faults:** `AdcFault` (`CalibrationTimeout` = the ruled `AdcCalibrationTimeout`, `ReadyTimeout`, `ConversionTimeout`) writes boot word `0x2100B0A1`–`A3` and parks; no twin judge reads them until 1211c.
- **Oracle:** `adcN_bring_up_is_in_order_and_bounded` on a spy: the oracle's words in order (from its own reading of `solved.adc_inputs`, `adc::oracle_of`), the calibration's hardware-set bits kept by the `ADEN` and `ADSTART` writes (Q-A), no `ADC_CR` write with two command bits (Q-A), each wait bounded and each expiry its own fault.
- Refused by name: `AdcNotGenerated` (ADC2, a channel past INP19), `AdcResolutionNotSolved` (a topology with no bits), `AdcResolutionsDiffer`, `AdcSequenceTooLong` (more than `SQ1`–`SQ4`).

### 2.4 The bridge's enable
- **Generator:** an enable is configured after its bridge's timer starts, its method taking `&Tim<Running>` (and `&P<Output>` for a GPIO input): an enable before `CEN` does not compile (`INV-02`). Where its declared level wakes the part, the row's `t_wake` (DRV8876 1 ms, SLVSDS7B p. 5) is waited (`settle`, sized at HSI's fastest). D1 declares `low`: no wait.
- **Proof:** the enable sits in its timer's part, with its GPIO inputs. The image refuses any write to it (`BSRR`, and its `MODER` move) before the timer's `CEN` is set and each GPIO input is an output: **the oracle's rule, not the typestate's** (the tier-2 test seeds a wake before `CEN` and the proof fails on it). The final state judges the bridge's idle against its part's truth table (`(0, _, _) | (1, 0, _)` for PH/EN: asleep, or brake), reading each input's level as silicon holds it (CCR 0 low, above ARR high, else a pulse no row holds).
- **At generate:** the idle tuple is looked up in the row (`BridgeIdleNotSafe` for a driving or unlisted row), and `BridgeAcrossParts` (an enable following inputs on two timers) and `WakeTimeUnstated` are refused by name.

### 2.5 The ledger
- A declared input's `PUPDR`: independent-rule (`implied_pupd`), refused if the generator's value differs. An idle-high motor pad's `PUPDR`: independent-rule (`implied_pupd`, Q9). Existing rows text for text.
- A pin device's output's `BSRR` rows and every ADC field: **silicon-only**, since `simulate` refuses a pin device until 1211c (1211c re-classes `PCSEL` and `SQR1` as observed once the twin runs them, 1211 §2.7 (c)). RCC gates `ADC12EN` (`STM32H7_RCC.cs` L736) and `ADC3EN` (L834).
- **An unclassified field stays a hard error** (`LedgerFieldUnclassified`; mutant WO4).

## 3. Decisions, as ruled

- **S1 · the topology's fields (ruled 19:06Z):** `SolvedAdcInput.bits` (device inputs only); `SolvedPinDevice.bridges {mode, inputs, enable {net, active}}`. **`mode` was added by me** (the ruled lookup by part and mode needs it) and **accepted 19:30Z**. No datasheet value is copied into the topology. Solved version 16, after 4240b's 15.
- **S2 · no `.repl` for a pin device; `generable` split** (`simulable` refuses `PinDeviceNotSimulated`, `IdleHighNotSimulated`; `emit` reaches eda's `PinDeviceNotDrawn`): as leaned.
- **S3 · `power.sense` not generated:** as leaned (a §7.1 row).
- **Q-A · linearity calibration**, `ADEN`/`ADSTART` read-modify-write, cited at the write (RM0433 pp. 931–932).
- **Q-B · bounds:** 165,010 ADC cycles is tCAL (offset + linearity): **DS12117 Table 173 p. 273, rev V's Min column**; Table 76 p. 167 gives rev Y one value spanning Min/Typ/Max. It is not a stated maximum that covers rev V, so each bound is **twice** the stated figure at rev V's 8 core cycles per ADC cycle, labelled `ruled`: calibration 2,640,160 reads, `ADRDY` 13,104, `EOC` 13,108.

## 4. Mutants (§1.5)

`spike/1211b/mutants/mutants.py`, one mutant per guard, in a detached copy. **21 of 21 killed.**

| the work order's four | killed by |
|---|---|
| WO1 drop a GPIO initial level | `a_pin_device_s_signals_are_generated_at_their_declared_pull_and_level` |
| WO2 write the idle after `CEN` | `pwm_starts_tim1_with_exactly_the_solved_writes_in_order`, `dshot600_starts_…` |
| WO3 enable a bridge before its `CEN` | `d1_s_enable_follows_its_bridge_s_cen_and_does_not_wake` (first survived: the test now also holds "configured once"); and at proof level, the tier-2 test's seeded wake before `CEN` fails the image's rule |
| WO4 an unclassified ledger field | `a_pin_device_s_input_pull_and_output_level_are_named` |

The other 17: the motor pull's idle, an input's pull (generator and oracle), the oracle's idle-high word, an input's speed write, `power.sense` generated, `simulate`'s two refusals, the `.repl`, the ledger's two rules, the image's enable rule, a driving idle, a waking enable unwaited, the 12-bit code, the calibration bound at the stated minimum.

## 5. Adversarial pass (§5)

A fresh Opus reviewer attacked `b9b37a12` in its own worktree (evidence `~/.cache/aet-1211b/adv-scratch/EVIDENCE.md`). **It found no way to make a generated board drive its motor at boot.** Its findings, each checked at source:

| # | finding | done |
|---|---|---|
| F1 | the ADC proof's unwind (4) too small for 4 inputs | `unwind(max(n, 2) + 2)`; a 4-input variant proves (§6) |
| F2 | `BridgeAcrossParts` refused PH as a GPIO (Q7, allowed) | a bridge's GPIO inputs join its timer's part; PH as a GPIO generates and proves. Two timers sharing an enable (D2's shape on two timers) stay refused by name (§7) |
| F3 | no proof-side truth-table judgement of the idle | the final state asserts a non-driving row, from the oracle's rows |
| F4 | the enable rule's and the ADC oracle's inputs came from the generator's plan | `oracle_bridges` and `adc::oracle_of` read the topology themselves; a missing ADC oracle fails closed |
| F5 | `t_wake`'s wait is held by a test, not a proof | recorded (§7): no proof models time |
| F6 | D1's enable `BSRR` rows classed observed | silicon-only, as the ADC's |
| F7 | `bridge_plans` skipped a bridge with no enable | refused by name |
| F8 | the `ADEN` read-modify-write writes `LINCALRDYW` back (as read) while `ADEN` 0; p. 931 allows writing them only with `ADEN` 1 | recorded: the ruled Q-A; writing a bit its own value is no toggle, but the page's letter is not met (§7) |
| F9 | an asleep idle accepts any input tuple, and `set_high` on the enable has no row check | recorded (§7): no drive API exists (1211 §2.5) |

Held: byte stability over 63 JSON files (25 identical, 37 refused on both, D1 alone differs); the idle words and pulls; the enable's order in firmware and both images; driving idles refused; the ADC's words, bounds, bases and the 8-bit refusal; every variant's firmware and proofs type-check.

## 6. Predictions beside results

| # | prediction | result |
|---|---|---|
| B1 | 25 trees and ELFs unchanged | **trees: holds** (after S3's correction, which the premise missed); ELFs: at READY |
| B2 | the ledger of every existing specification unchanged | **holds** (new rows and rules only; `ledger_tier1` green) |
| B3 | D1 generates, builds and proves; D2 refused at validate | **holds** |
| B4 | D1's words | **holds:** TIM1 PSC 0 ARR 3199 CCR 0/0; TIM2 PSC 63 ARR 19999 CCR 1500; PA1 BR1 before MODER; PA11 MODER 00, PUPDR 00 |
| B5 | each harness under 600 s | **holds:** slowest 48 s on Furnace, 58 s in the tier-2 run |
| B6 | the four mutants killed | **holds** (WO3 after the test was strengthened) |

## 7. Findings, not fixed here

- **F5:** `t_wake`'s wait is held by a host test only; no proof models time.
- **F8:** the `ADEN`/`ADSTART` read-modify-write writes `LINCALRDYW1`–`6` back as read while `ADEN` is 0 (`ADEN`'s write); RM0433 p. 931 allows writing them only with `ADEN` 1. As ruled (Q-A); the bench should read `ADC_CR` after the bring-up.
- **F9:** a sleeping part's idle accepts any input tuple, and the board crate's `set_high` on an enable has no row check: a later wake drives at once. No drive API exists (1211 §2.5); the drive API's work order should own it.
- **Renode's ADC model reads `RES` as 2 bits at bit 2** (`STM32_ADC_Common.cs` L551), where the H7's is 3: `010` reads as its enum's value 2. 1211c's, when the twin runs an ADC.
- **The ledger's value column for a read-modify-write field** (`ADC3.CR.ADEN`, `LINCALRDYW#`) reads the generator's known bits, 0 where silicon leaves 1 (`ADEN` is `rs`; `LINCALRDYW` hardware-set). The rows are silicon-only and outside any package today.
- **`BridgeAcrossParts`:** an enable shared by bridges on two timers (D2 with its inputs on two timers) is refused by name; merging the timers' parts is the fix, once D2 can validate.

### 7.1 Rows for the roadmap
- **ADC bounds:** DS12117 Rev 10 states rev V's tCAL, tSTAB and tCONV only as minimums (Table 173 p. 273); the three bounds are the architect's 2× (Q-B). A stated maximum, or a bench measurement, replaces them.
- **`power.sense`'s bring-up** (S3): solved into `adc_inputs` but generated by no firmware; generating it moves the bench package.

## 8. Gates, fence, reduction pass

- **`cargo fmt --all -- --check`:** clean. **`cargo clippy --workspace --all-targets -- -D warnings`:** clean (at `2f0f3496`'s parent; no lint since).
- **`cargo test --workspace`** (host, in-tree target, `main` `ddfa64d6` merged): **2,597 passed, 5 failed**; four were the solved bump's pins, fixed and green in `2f0f3496` (`solve_cli` 18/18, `readme_versions`, `validate` 129/129); the fifth, `bench_board_tier1::the_committed_package_is_bound_to_its_specification_scenarios_and_emit`, waits on the package (§9). svd-hal 133/133 and the CLI's pin-device and ledger suites green after the last fix.
- **Kani at `2f0f3496`** (image, one harness at a time): D1's 10 harnesses pass, the slowest 42 s; the PH-as-GPIO variant's 10 and the 4-input ADC's bring-up pass (before the merge, `3132793a`).
- **`TIER2_MIN`:** 200 → 201 (one tier-2 test, `board_tier2::d1_s_pin_device_generates_builds_and_proves`, 96.19 s in the image; its weight row an estimate × 2.8, kept beside 1913's refit).
- **Fence:** `crates/aethelgard-svd-hal`; `crates/aethelgard-cli/src` (generate, ledger, simulate's gate); `crates/aethelgard-ir/src` for S1's fields as ruled (`pin_devices.rs`, `solve.rs`, `lib.rs`'s version) and the one oracle helper `silicon::implied_pupd`; tests and stored topology fixtures (version lines, hwdef's literal); `ci/test-weights.tsv` (one row) and `TIER2_MIN`; `spike/1211b/`; this file. **Edges, flagged:** `crates/aethelgard-cli/README.md` (the refusal sentence, made true) and `crates/aethelgard-ir/README.md` (the solved version line).
- **Reduction pass (diff):** +7 net (`87b45a2d`: 32 in, 25 out; the two new labels are 7 comment lines, the table about even) · 1 duplicate removed (the ADC's names: four functions and a string-built ledger copy, now one table) · 0 dead paths removed · 7 protected kept (labelled `belt-and-suspenders:`: `oracle_level_bit`, `oracle_idle_word`, `oracle_bridges`, `adc::oracle_of`, `adc::oracle_res`, `adc::oracle`, and the generator's `motor_pull`/`input_pull`, labelled in this pass) · 1 finding: `bridge_plans` and `oracle_bridges` read one bridge twice on purpose (`INV-07`), which is why both exist.
- **Scratch:** `~/.cache/aet-1211b` 9.8 GiB after freeing 12.4 GiB (bus 2026-11-22); evidence kept (`adv-scratch`, logs, baselines).

## 9. The package

From the architect's write=true dispatch, **run 44976609648** (on `3e30777d`, PR #307's head), artifact `bench-package`, committed byte for byte. It moved exactly as predicted and ruled:
- `MANIFEST.json`: `topology_sha256` `6d93b01a…3597` → `ce2d02a1…9c93` (solved 16) and `verdict_sha256` `09552a0d…c97b` → `c1741912…bdec`; nothing else in it.
- `FAB-NOTES.md`, `sentinel-01-bench.elf` and every Gerber, drill, placement and BOM file: **byte-identical** (16 of 17 files).
- The binding test (`bench_board_tier1`) passes with it, 2/2.
