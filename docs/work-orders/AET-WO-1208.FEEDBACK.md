# AET-WO-1208 FEEDBACK — `validate` judges the MCU's own supply, and refuses the rest of what `solve` refuses statically

Builder: Claude (Opus 5.5), the `1207` session continuing, 2026-11-16 ~19:50 UTC. Worktree `~/aethelgard-1207` (the `1207` branch merged as `d539bb1`), branch `wo/aet-1208-validate-mcu` from `origin/main` `d539bb1`. The work order is read from `origin/claude/argon-repo-docs-synthesis-rjrb0d` (docs PR #180); this branch does not commit it.

Labels: **measured** (run here), **read** (read at source, file:line), **inferred** (reasoned, not run).

## 0. Predictions (committed before any measured run)

Written at ~20:10 UTC, after reading the code and DS12117, before building or running anything.

| # | Prediction | Basis |
|---|---|---|
| P1 | **DS12117 at source.** Rev 10 (the PDF in `~/.cache/aet-4230/ds/`, whose sha256 begins `607b9eadaef1c54f`, as `power-parts.json`'s `DS12117` document does), Table 111 "General operating conditions", p. 209 by the document's own table of contents: VDD "Standard operating voltage" 1.62 V (note 1) to 3.6 V. Note 1: "When RESET is released functionality is guaranteed down to VBOR0 min". The MCU row already holds `kind.mcu.vdd_millivolts` 1620 to 3600, cited to that table and page. **So F2 needs no data and no `power_parts.json` schema change, and takes no version number.** | read |
| P2 | §1.1 finds **no** stop. Every corpus specification's MCU domain lies in 1620 to 3600 mV: 3135 to 3465 everywhere, `imu-baro-1v8` 1710 to 1890; `invalid-zero-domain` (0 mV) is already refused (`ZeroNominalVoltage`). F1's rules can refuse only what `solve` refuses already, and `generate` solves. | read: the corpus inventory |
| P3 | **F2 today:** the MCU row's range is read only by `power.rs` `consumers` (step 16), so only for a specification with a power tree. Without one, nothing judges the MCU's rail, which is `1207`'s measured 5.0 V board. The new refusal (`McuSupplyOutsideRange`) sits right after step 16 ("16a"). A power tree's existing refusal for the same rail keeps coming first, and the new one closes the no-tree path. It judges the domain's declared minimum and maximum, as `4230` did for the pull-up floor. | read: `power.rs:1076-1101` |
| P4 | **The BOR level is a second gap of the same kind.** Without a power tree, a rail whose minimum is below the selected BOR level's falling threshold (`bor_level` 3: 2.68 V) holds the MCU in reset, and nothing refuses it. `power.rs` takes that floor for a tree's MCU rail. The work order names the operating range only, so this is measured at §1.1 and reported, not built, unless §1.1 shows it costs nothing. | read: `power.rs:1091` |
| P5 | **F1, static in verdict:** `BusTooFast`, `PullUpBelowSinkFloor`, `BusCapacitanceOverLimit`, `MotorTiming`, `SerialBaudOutOfRange`, `SerialBaudTolerance` and `SpiSpeedBelowKernelRange`. The reset clock gives every timer, every USART/UART (`hclk / 1`) and every SPI kernel (HSI) one frequency, so none depends on the search's choice of peripheral. | read: `motor.rs:120-137`, `serial.rs:201-208`, `solve.rs:615-627` |
| P6 | **F1, static in bytes: all but `SpiSpeedBelowKernelRange`.** Its message names the peripheral the search chose ("the slowest `SPI1` reaches"), so `validate` cannot print `solve`'s bytes without the search. It stays at `solve`, reported for a ruling. That is not a §7 stop: no two error types need to stay apart. The serial and motor messages name only the port or output, the baud, the kernel clock and the table row. | read: `solve.rs:146-160`, `:512-566`, `:778-800` |
| P7 | The six move into step 21, in the order `solve` used before `1207`: per I2C bus its rise time, then `BusTooFast`; then per bus its pull-up and SDA and SCL capacitance; then the SPI buses (`1207`'s three); then each motor output's timing; then each serial port's BRR and tolerance. So `1207`'s recorded first-refusal change (`MissingSpiConstraint` or `SupplyOutsideSpeedTable` before `BusTooFast` and the electrical two) is undone. The motor and serial refusals now come before the table-dependent refusals (`PartMismatch`, the search's), which changes only multi-fault first refusals. | read |
| P8 | `solve` keeps no copy of the six: it calls `validate` first. `MotorTiming`, `SerialBaudOutOfRange` and `SerialBaudTolerance` become `IrValidationError` variants with the same messages, and `SolveError` loses them. The CLI needs no edit: the three name a claimant, which `1206`'s optional `claimants` carries. | read: `cli/src/main.rs:290-305` |
| P9 | **F3: (a), `schematic::emit` calls `Specification::validate` first,** is cheap. On the largest fixture (`sentinel-02-power`, or `solve/stress`) `validate` takes milliseconds, against seconds for `generate`. It closes the whole class (every name rule, not only F3's pairs) with no second copy of `validate`'s rules in eda, which (b) would be. The eda guards that only a library caller reached may become unreachable (`1206`'s `UnsupportedAddress`, `1207`'s emitter `spelled`); each is checked and recorded. | inferred, to be measured |
| P10 | Every emitted file of every corpus specification is byte-identical; `TIER2_MIN` stays at `main`'s 160. | inferred |

## 1. §1.1 measurement (before any rule was wired): no stop

Harness: `~/.cache/aet-1208/tools/` (`verdicts.sh`, `probe.py`); the baseline binary is `main` at `d539bb1` (`~/.cache/aet-1208/baseline/`), built in a capped scope.

- **The corpus:** 23 specification files and 26 scenario files, as in `1207` [measured]. No scenario override changes a domain, a rate, a baud or a pull-up past a limit; `droop` changes no specification value (`campaign.rs:178`) [read].
- **Baseline verdicts** [measured]:
  - 15 specifications generate;
  - `imu-baro-1v8` and `sentinel-02-own` are `UnknownPart` at `generate`;
  - three are `Infeasible`;
  - three are refused by `validate`.
- **Each candidate rule over the corpus** (`probe-1.1.txt`) [measured]:
  - **F2 (MCU rail outside 1620 to 3600 mV):** one hit, `invalid-zero-domain` (0 mV), which is already `ZeroNominalVoltage`. Every other MCU rail is 3135 to 3465 mV, or 1710 to 1890 mV (`imu-baro-1v8`).
  - **The BOR floor** (P4, measured but not built): the same single hit.
  - **F1:** no corpus specification is refused by any of the seven kinds at `solve`.

  No rule refuses a specification `generate` accepts.
- **DS12117 at source** [read]: Rev 10, sha256 `607b9ead…`, the document `power-parts.json` names.
  - **Table 111, "General operating conditions"** (p. 209 by the document's own table of contents; the text extraction falls between the footers `209/357` and `210/357`) gives "VDD Standard operating voltage" 1.62 V (note 1) to 3.6 V. Note 1: "When RESET is released functionality is guaranteed down to VBOR0 min".
  - **The MCU row** already holds `kind.mcu.vdd_millivolts` 1620 to 3600, cited to "p. 209, Table 111, VDD 1.62 to 3.6 V". **No data change, no `power_parts.json` schema change, and no version number.**
- **Reachability on the baseline** [measured]:
  - **F2 (`~/.cache/aet-1208/reach/`):** each of these passes `validate` and **generates a board**:
    - 5.0 V ± 0.1 % and 3.8 V ± 0.1 % with a lone TMP117 and no SPI bus;
    - 3.0 V ± 20.1 % (2397 to 3603 mV).
  - **The BOR floor:** a 1.9 V ± 1 % rail with `bor_level` 3 (VBOR3 falls at up to 2.68 V, DS12117 Table 115) generates a board too: P4's gap is real (F1 below).
  - **A power tree (step 16):** `DomainOutsideConsumer` already judges the MCU's rail against this row, BOR floor included (`pwr-bare-tol95`: "the domain's 2986 to 3614 mV is not inside STM32H753ZITx (MCU)'s 1620 to 3600 mV with its minimum above the consumer's floor of 2680 mV").
    - A connector-fed MCU rail cannot occur inside a tree: every domain needs a regulator (`DomainWithoutSource`, `pwr-conn-*`).
    - So F2 is exactly the specifications with no power tree.
  - **F1's bytes (`~/.cache/aet-1208/pins/`):**
    - **Static:** `MotorTiming` (PWM 1 Hz), `SerialBaudOutOfRange` (300 baud) and `SerialBaudTolerance` (a 50,000 ppm peer) print the same bytes from `solve`, `generate`, `emit`, `check speeds`, `verify` and `extract`.
    - **Not static:** `SpiSpeedBelowKernelRange` (a 1000 Hz bus) prints "the slowest `SPI1` reaches" when the search chooses and "`SPI4`" when the bus hand-sets SPI4.
  - **F3's cost (debug binary, the whole process, best of 7):** `validate` takes 5.7 ms on `solve/stress`, 6.0 ms on `sentinel-02-power` and 5.9 ms on `d3-24-i2c-12-spi`. `emit` takes 5.4 s, 9.2 s and 7.9 s on the same files.

## 2. Results beside the predictions

| # | Result | Label |
|---|---|---|
| P1 | Held: the row already carried the cited range, so no data or version change. | read |
| P2 | Held: no stop. | measured |
| P3 | **Placement changed.** `McuSupplyOutsideRange` is step 22, after step 21, not "16a". Step 21's `SupplyOutsideSpeedTable` judges the same 1620 to 3600 mV beside an SPI bus, and "16a" would have renamed that refusal for every such specification, a refusal `main` prints today (the advisor's catch, checked against `1207`'s pins). After step 21, a power tree's `DomainOutsideConsumer` and the SPI rule keep their bytes, and the new refusal fires only where nothing did. | read, measured |
| P4 | Half-known. The BOR floor is already the rule for a power tree's MCU rail (`DomainOutsideConsumer`, floor = max(1620 mV, VBOR falling max)); without a tree nothing applies it. Not built: the work order names the operating range. F1 below. | measured |
| P5 | Held. The reset clock gives every timer, every USART/UART (`hclk / 1`, all prescalers 1) and every SPI kernel (HSI) one frequency. | read |
| P6 | Held, measured: `SpiSpeedBelowKernelRange` names the chosen controller (`SPI1` against a hand-set `SPI4`), so it stays at `solve` (F2 below). | measured |
| P7 | Held. Step 21 now runs in `solve`'s old order, so `1207` §5's first-refusal change (an SPI refusal before `BusTooFast` and the electrical two) is undone. | read; §6 measures it |
| P8 | Held. `MotorTiming`, `SerialBaudOutOfRange` and `SerialBaudTolerance` are `IrValidationError` variants with the same messages; `SolveError` lost them. Nothing outside `aethelgard-ir` named them (grep), so the CLI needs no edit. | read, measured |
| P9 | Held: (a), validate-first, costs about 0.1 % of `emit` and closes the whole class. On a scratch build, eda's suites stay green, and four CLI tests change, each one that handed `emit` a specification `validate` refuses (§3.4). | measured |
| P10 | Held: §4.1. | measured |

## 3. What was built, by work-order item

1. **§1.1:** above.
2. **§1.2, the MCU's supply (F2).** *Superseded by the architect's ruling on F1 and F5 (§10): the no-tree path now runs step 16's consumer rule as `DomainOutsideConsumer`, and `McuSupplyOutsideRange` is deleted. This item records the first build.*
   - **Step 22, `Specification::mcu_supply`:** the MCU's supply domain, its declared minimum and maximum (`minimum_millivolts`, `maximum_millivolts`, as `4230` took the pull-up floor at the maximum), against the power-parts row's `kind.mcu.vdd_millivolts`.
   - **The refusal:** `McuSupplyOutsideRange`, whose `id` is the domain. The message names the part, the domain, both ranges, and the row's citation in step 16's form (`PowerPartsTable::cite`), so the two refusals cite one row alike.
   - **A part with no MCU row** is left to the emitter's `UnknownPart`, as before.
   - **The test** is the measured 5.0 V board, `aet_1208_the_mcu_rail_is_judged_against_its_row`: `validate` and `generate` print the pinned refusal, and nothing is written. Before (`bb72efc`), both passed and `generate` wrote a board.
3. **§1.3, the static refusals (F1).** Step 21 (`solve_static`, formerly `1207`'s `speed_inputs`) now holds, in the order `solve` judged them before `1207`:
   - per I2C bus, the rise time, then `BusTooFast`;
   - per I2C bus, `PullUpBelowSinkFloor`, then `BusCapacitanceOverLimit` for SDA and then SCL;
   - the SPI three;
   - each motor output's timing (`solve::motor_timing`, shared);
   - each serial port's `BRR` and worst case (`solve::serial_timing`). It runs on `serial::PERIPHERALS[0]` after asserting that all eight kernels are one frequency on the reset clock, as `motor_timing` asserts for the timers.

   **What `solve` keeps:**
   - **Deleted:** `solve`'s `i2c_electrical`, its `BusTooFast` branch, and the three `SolveError` variants.
   - **Kept, for their values only:** `i2c_ceilings` (infallible now), `motor_timing` and `port_on`. Their refusals are validation's variants inside `InvalidSpecification`, which nothing reaches after `validate`.
   - **Kept as `solve`'s own:** `SerialClock`, which needs the chosen peripheral, and `SpiSpeedBelowKernelRange` (F2 below).

   **Tests:**
   - `aet_1208_validate_refuses_what_solve_refused_byte_for_byte` pins the motor and both serial refusals from `validate` (with and without acknowledgements), `solve` and `generate`, with `d539bb1`'s bytes.
   - `aet_1206_one_error_set_keeps_solves_bytes` now asserts `validate` prints the bus three's bytes too; it was their old-behaviour test.
4. **§1.4, the emitter (F3): (a).** `schematic::emit_moved`, which `emit` calls, runs `Specification::validate` first, and returns its refusal as `EmitError::InvalidSpecification`, whose `kind()` and `id()` are the inner refusal's.
   - **Why (a):** it costs about 6 ms against 5 to 9 s, and it closes every name rule, not only F3's pairs. (b) would be a second copy, in eda, of `validate`'s net rules (`DuplicateNetName`, `NetShadowsDomain`, step 20's).
   - **Its cost in callers:** no valid specification changes, so the CLI's bytes are unchanged (it validates before `emit`).
   - **Four tests handed `emit` a specification `validate` refuses,** to reach an emitter guard. Each now asserts `validate`'s refusal:
     - 1206's address strap: `I2cAddressNotOffered`, which reached `UnsupportedAddress`;
     - 1207's four serial and motor rows: `ReservedName` and `NetShadowsDomain`, which reached the emitter's `ReservedName`;
     - power board K10, `vbat`: `ReservedName`, the same text;
     - a crossing on a two-device bus: `CrossingInvalid`, which reached `PowerNotDrawable`.
   - **The F3 test**, `aet_1208_the_emitter_draws_only_what_validate_passes`, renames a serial net to `VDD_3V3` and to `I2C_SDA`, and a motor output to `MOTOR4`. Each emitted a shorted board on `main` (`bb72efc`); `emit` now returns `validate`'s refusal (`NetShadowsDomain`, `DuplicateNetName`).
   - **Guards now unreachable through `emit`,** and not deleted (the fence is "the F3 change only"): eda's `UnsupportedAddress`, `PowerNotDrawable` for a crossed bus of two devices, and the emitter's own `ReservedName` and `spelled` collision for a name `validate` refuses (F3 below).

## 4. Evidence

### 4.1 Verdicts and bytes (measured: `~/.cache/aet-1208/v-base` against `v-new` at `7c9bb8f` and `v-new2` at `d5d600e`)
- **All 23 specifications** give byte-identical `validate`, `solve` and `generate` stdout and exit codes, and an identical sha256 for every generated file: 153 of 153 records, with the 15 generating specifications' 210 files, at both commits.
- **The F1 pins** (`~/.cache/aet-1208/pins/`): `solve`, `emit`, `check speeds`, `verify` and `extract` print byte-identical bytes on both binaries. The new `validate` prints exactly the baseline `solve`'s bytes for the motor and both serial pins. The two SPI pins are unchanged everywhere: `SpiSpeedBelowKernelRange` stays at `solve`.

### 4.2 Tier-1 tests, each committed first with the behaviour before (`bb72efc`)
- **`aet_1208_validate_refuses_what_solve_refused_byte_for_byte`** (cli `reserved_references_tier1.rs`): four rows, each printing `d539bb1`'s bytes from `solve`, `generate` and `validate` (with and without acknowledgements):
  - motor PWM at 1 Hz;
  - 300 baud;
  - a 50,000 ppm peer;
  - the adversarial pull-up at the nominal rail (§6).
- **`aet_1208_the_mcu_rail_is_judged_against_its_row`:** the 5.0 V board (§3.2).
- **`aet_1208_the_emitter_draws_only_what_validate_passes`:** §3.4.
- **Old-behaviour tests already on `main`, now flipped:**
  - `aet_1206_one_error_set_keeps_solves_bytes` (`validate` now prints the bus three's bytes);
  - `1207`'s `bare_rail(.., false)` assertions (`McuSupplyOutsideRange` at 1618 and 3603 mV);
  - `refuses_tolerance_out_of_range` at ±50 %.

### 4.3 Tests that encoded the old behaviour (test-built specifications, not corpus verdicts)
- `serial.rs`: `a_peer_deviation_is_cited_only_as_the_crsf_spread` (a declared 20,000 ppm, and a console at 8,002 ppm); the 416,666-baud CRSF case; the BRR and tolerance-edge cases. Each asserted `validate` passes, or `SolveError`'s variant; each now asserts step 21's refusal, or validation's variant inside `InvalidSpecification`.
- `validate.rs`: `refuses_unsupported_i2c_speed_…`. Its 1 MHz case had 300 ns lines, which step 21 refuses as `BusTooFast`; the case now declares 120 ns.
- **The four emitter-guard tests:** §3.4.

### 4.4 The CLI
- **No CLI source changed.** The three moved variants name a claimant, which the CLI's optional `claimants` carries (`1206` §4.3). The CLI validates before `emit`, so the emitter's validate-first changes none of its bytes [measured, §4.1].

### 4.5 Mutation (diff-sized: one mutant per guard this work order adds or changes)
- **Method:** 10 mutants, one at a time, against `aethelgard-ir` (every test) and cli `reserved_references_tier1` and `power_board_tier1` (`~/.cache/aet-1208/mut/`).
- **Result:** all 10 killed by a test failure [measured]. No log holds `could not compile`.

| Mutant | Killed by |
|---|---|
| `BusTooFast` off | ir, cli |
| pull-up at the maximum off | ir, cli |
| capacitance off | cli |
| motor timing off | cli |
| serial off | ir, cli |
| the MCU's minimum edge (`<` to `<=`) | ir |
| the MCU's maximum edge (`>` to `>=`) | ir |
| step 22 off | ir, cli |
| `emit` trusting its caller | cli |
| the nominal pull-up check off (the adversarial fix) | cli |

## 5. Behaviour changes a reviewer should know

- **A new refusal, licensed by the work order:** an MCU supply outside 1620 to 3600 mV, with no power tree and no SPI bus, is `McuSupplyOutsideRange`. Before, it generated a board. No corpus specification is affected.
- **`validate` with acknowledgements.** For the six moved kinds it now prints the refusal itself, where it printed `UnwitnessedAcknowledgementStale`: the CLI's `validate` solves only for the ledger, after `Specification::validate`.
- **First refusals of multi-fault specifications** (the adversarial pass's E table, 1,614 specifications; new `validate` and new `generate` always agree):
  - **The pre-`1207` order is restored, as P7 said.** An I2C electrical fault now comes before an SPI constraint or supply fault. `BusTooFast` on bus 0 now comes before `RiseTimeTooSlow` on bus 1 (per-bus interleaving). Under `1207`, the SPI three and every rise time came first.
  - **Before the pin table.** The static faults now come before `solve`'s pin-table refusals. An unknown MCU part (`UnknownPart`, no pin table) beside a static fault prints the static fault now.
  - **The nominal pull-up check** sits at the end of step 21, where `solve`'s topology check came last.
- **`campaign`** [read, not run: it needs a firmware ELF]: a base specification with one of the moved refusals is refused by `parse_spec`, with its "scenario `<name>`, base: " prefix, where before `solve_spec` refused it without one, as `1207` §5 recorded for its three.
- **`emit` for a library caller:** a specification `validate` refuses now returns `EmitError::InvalidSpecification`, whose kind, `id` and message are the validation refusal's.

## 6. Adversarial pass (targeted)

**What ran.** A fresh subagent, given both binaries and the work order's claims, not this document, at `7c9bb8f`. It generated 5,254 specifications and ran `validate`, `solve` and `generate` on both binaries for each: 31,524 invocations, every exit 0 or 2, no panic. Its families:
- A: the MCU rail;
- B: limits;
- E: 1 to 3 faults combined;
- F: 400 seeded mutations;
- G: the 23 repository specifications.

Harness, specifications, `REPORT.md` and `results/`: `~/.cache/aet-1208/adv/`.

**Results** [measured; I re-ran the 3a repro and all 84 of its specifications by hand]:
- **A, the MCU rail:**
  - **No device on the rail:** `McuSupplyOutsideRange` exactly when the rail leaves 1620 to 3600 mV.
    - Accepted: 1800 ± 10.0 % (1620 to 1980), 3000 ± 20.0 % (2400 to 3600), 3272 ± 10 % (2944 to 3600).
    - Refused: 1799 ± 10 % (1619), 3273 ± 10 % (3601), 1620 ± 0.1 % (1618 to 1622), and `u32::MAX` nominal (saturated).
    - Minimum in range with the maximum out, the reverse, and both out: all refused.
  - **A device on the rail:** its own `SupplyOutsidePart` comes first, as before.
  - **Beside an SPI bus:** `SupplyOutsideSpeedTable` comes first (55 specifications).
  - **In a power tree:** 160 lean trees, including a regulator output in range with its declared tolerance out. The first refusal was always one of the tree's own (`DomainOutsideConsumer`, `RegulatorOutsideDomain`, `SupplyFloorBelowBorRising`, `SenseFullScaleExceeded`, `SupplyOutsidePart`); `McuSupplyOutsideRange` was never printed there.
  - **A part with no MCU row:** passes `validate`, as designed, and is `UnknownPart` at `solve`.
  - **Rail consistency:** over all 5,254 specifications, 0 refused in range and 0 accepted out of range.
- **B, `validate` passes and `solve` or `generate` refuses: 108,** each identical on the baseline.
  - **The defect: 84 `PullUpBelowSinkFloor`.**
    - Step 21 judged the floor at the domain's maximum, as `solve`'s pre-search check did. `solve`'s final topology check (`SolvedTopology::validate`, `analog_lines.domain_millivolts` = the nominal, `solve.rs:834`) judges it at the nominal.
    - The floor jumps at 2.0 V (`VOL`) and 2.7 V (Fast-mode Plus 4 and 8 mA). So, for example, 2.6 V ± 10 % at 1 MHz with 308 Ω passes at 2860 mV (308 Ω floor) and fails at 2600 mV (550 Ω).
    - **Fixed** (`d5d600e`): step 21 ends with the nominal check, in the topology's bus order. All 84 now print the baseline `solve`'s bytes from `validate`, a pin row covers the case, and its mutant is killed. Which rail the floor belongs at is F4.
  - **The rest depend on the pin table, the vendored models or the search:**
    - 14 `UnknownPart`;
    - 4 `RateAbovePinSpeed` (each names the chosen pin);
    - 3 `Infeasible`;
    - 3 serial ports whose hand-set pins split or miss a peripheral.
  - **None was `MotorTiming`, `SerialBaud*`, `BusTooFast` or `BusCapacitanceOverLimit`.**
- **C, false refusals: none.**
  - 80 are `McuSupplyOutsideRange`, each with the rail outside the range, as intended.
  - 10 are `UnwitnessedAcknowledgementStale`, which the baseline `validate` prints identically.
- **D, bytes: 0 differences.**
  - Same command and kind on both binaries: 11,056 results compared, all identical.
  - Both binaries generated the same 11,480 files.
  - Baseline `solve` against new `validate`, identical bytes, by kind:

    | Kind | Identical / specifications |
    |---|---|
    | `BusTooFast` | 161 / 161 |
    | `BusCapacitanceOverLimit` | 102 / 102 |
    | `MotorTiming` | 230 / 230 |
    | `SerialBaudOutOfRange` | 102 / 102 |
    | `SerialBaudTolerance` | 172 / 172 |
    | `PullUpBelowSinkFloor` | 193 / 277 at `7c9bb8f`, the other 84 being the defect (all 84 identical after the fix) |
- **E, multi-fault:** §5.
- **Not run:** `SpiSpeedBelowKernelRange` (no specification produced it; §1 pins it by hand), `hse_hz` other than 0, and `emit`, `verify` and `prove` in the subagent (§4.1 covers `emit`, `verify`, `check speeds` and `extract` on the pins).

## 7. Reduction pass

The four questions over the diff:
- **(1) A fact stated twice:**
  - The six refusals' rules now live once, in step 21. `solve`'s `i2c_electrical`, its `BusTooFast` branch and three `SolveError` variants are deleted.
  - The motor reasons and the serial judgement (`port_on`) have one home, shared by `validate` and `solve`.
  - **Kept, reported:**
    - The topology's own pull-up and capacitance recheck (`SolvedTopology::validate`), which `RUL-16` requires of a hand-written topology.
    - Three names for one rail range (F5).
- **(2) Unused:**
  - The eda guards now unreachable through `emit` (§3.4) are outside the fence ("the F3 change only"), so they are F3, not deleted.
  - `solve`'s `InvalidSpecification` maps on `motor_timing` and `port_on` are typed, not dead: they keep `solve` safe if `validate` ever changes.
- **(3) Two mechanisms that could be one:** `solved_port` and `serial_timing` both wrap `port_on`; nothing further.
- **(4) A workaround for a gone problem:** none.

Reduction pass (diff): −48 lines net in `solve.rs` (`i2c_electrical`, the `BusTooFast` branch, three variants) against +170 in `lib.rs` (four variants, steps 21's growth and 22) · 3 duplicates removed (the bus electrical rules, the motor and serial refusals, the reasons) · 0 dead paths removed (the eda guards are a finding, outside the fence) · 0 protected kept · 2 findings below (F3, F5)

## 8. Findings and questions

| # | Finding | Where | Evidence | Disposition | Decision needed |
|---|---|---|---|---|---|
| F1 | **The BOR floor without a power tree.** A 1.9 V ± 1 % MCU rail with `bor_level` 3 (VBOR3 falls at up to 2.68 V, DS12117 Table 115) passes `validate` and generates a board whose MCU stays in reset. Inside a tree, step 16's `DomainOutsideConsumer` already takes floor = max(1620 mV, VBOR falling max); without one, nothing does. Repro: `~/.cache/aet-1208/reach/bor3-1900-nospi.json` | `lib.rs` step 22 | measured | not built: the work order names the operating range | yes: should step 22 take the BOR floor (by step 16's rule)? |
| F2 | **`SpiSpeedBelowKernelRange` stays at `solve`.** Its verdict is the specification's (every SPI kernel is HSI on the reset clock, one instance per bus), but its message names the controller the search chose: "the slowest `SPI1` reaches" against a hand-set bus's "`SPI4`" | `solve.rs` `spi_controller` | measured (`pins/spi-1000-*`) | a deviation from §1.3, which says each moves by `solve`'s bytes | yes: move with a message that names no controller (a byte change to an existing refusal), move only for a hand-set controller, or keep |
| F3 | **eda guards unreachable through `emit`** now that it validates first: `UnsupportedAddress`, `PowerNotDrawable` for a crossed bus of two devices, and the emitter's own `ReservedName` and `spelled` collisions for names `validate` refuses. Kept, outside "the F3 change only" | eda `schematic.rs`, `schematic_power.rs` | measured (the four tests of §3.4) | reported | yes: delete them in a reduction work order, or label them `belt-and-suspenders:` |
| F4 | **Which rail the pull-up floor is taken at.** `validate` (`4230` A8) and `solve`'s pre-search check took the domain's maximum; `solve`'s topology check, the nominal (the topology carries only the nominal). Because the floor jumps at 2.0 and 2.7 V, neither is the worst case over the declared range: the floor's maximum over [min, max] is. Step 21 now applies both checks, so `validate` refuses what `solve` refuses, by its bytes. That is not yet the physics | `lib.rs` step 21, `SolvedTopology::validate`, `electrical::i2c_pull_up_floor_ohms` | measured (adversarial 3a) | built to match `solve` | yes: judge the floor's maximum over the declared range (a rule change; the topology would need the tolerance) |
| F5 | **One range, three names:** `DomainOutsideConsumer` (with a tree, BOR floor included), `SupplyOutsideSpeedTable` (beside an SPI bus, DS12117 Table 149) and `McuSupplyOutsideRange` (otherwise, Table 111). Both tables give 1620 to 3600 mV for this part | `power.rs`, `lib.rs` steps 21 and 22 | read, measured | by the work order's literal reading (a new name), placed after step 21 so the two older names keep their bytes | yes: should the no-tree path run step 16's consumer rule instead (one name, BOR floor included, which also closes F1)? |
| F6 | §6's split-on-touch names `ir/src/lib.rs` (+184 −14 here: four variants, step 21's growth, step 22). Declined, as `1206` F8 and `1207` F6 did: a split is a change of its own, and step 21's code sits beside step 20's | roadmap §6 | read | declined | no |

**Forks for the architect to confirm:**
- **The CLI tests** (`reserved_references_tier1.rs`, `power_board_tier1.rs`) changed under "tests": no CLI source.
- **`EmitError::InvalidSpecification`,** a new public variant of eda's error.
- **`solve::serial_timing`** asserts that the reset clock gives all eight USART/UARTs one kernel clock, as `motor_timing` asserts for the timers.
- **The ir README:** steps 21 and 22, and `solve`'s table (rows 1a, 1b, 2a removed; 7a and 7b reworded).

## 9. Where everything ran, and the gates

- **Where:** host cargo in a capped user scope (`systemd-run --user --scope -p MemoryMax=32G -p MemorySwapMax=0`, with `memory.swap.max` read back as 0). The differentials and the adversarial harness ran in a 16 GiB, swap-0 scope with at most 6 workers. No container, Renode or KiCad was started.
- **This work order adds no tier-2 test,** so `TIER2_MIN` is `main`'s, 160. Its new tests are tier 1, in existing binaries, so it adds no `ci/` weight.
- **Gates on `d5d600e` plus this FEEDBACK** [measured]:
  - `cargo fmt --all -- --check`: clean;
  - `cargo clippy --workspace --all-targets --features aethelgard-cli/with-tools -- -D warnings`: clean;
  - CI's package include check, run locally on the packaged `aethelgard-ir` and `aethelgard-eda`: every target is inside its package;
  - `cargo test --workspace --no-fail-fast`: 117 test binaries, 0 failures.
- **The merge of `origin/main` at `3dcd242`** (`4403` #176, docs #180) brings only `container/`, `.github/` and `docs/`, and no crate source, so the gates above stand on the merged tree. The work order on `main` is identical to the one built from.

## 10. CI and the stop line

- **CI on PR #182 at `57d8295`** (run 43950671084): **green** [measured].
  - **Green jobs:** `gates` (fmt, clippy, tier 1, the package step), all eight `tools-shard` jobs, the `tools` aggregate (`tier2_tests_passed=160 floor=160`: `TIER2_MIN` is exact and unchanged), `shards`, `cli`, `pinned`, `pinout`, `source-rehearsal`, both images and the four `ardupilot` jobs.
  - **Shard times:**

    | Shard | Time |
    |---|---|
    | 0 | 27:59 |
    | 1 | 29:48 |
    | 2 | 26:57 |
    | 3 | 22:29 |
    | 4 | 27:02 |
    | 5 | 24:49 |
    | 6 | 24:35 |
    | 7 | 22:27 |

    `gates` took 14:58. Every shard is over the 20-minute clause, as are the same hour's other runs [measured, from their run timings]: `main`, run 43944349940, 23 to 27 min; `4403`, run 43950185531, 20 to 32 min. So this is the runners' load. This work order adds no tier-2 test and no `ci/` weight. The READY commit changes only this file, and its own run is the one the architect reads.
- **Disk:**
  - No agent worktree was created (the adversarial subagent ran without one). The mutants ran in this worktree, each restored.
  - **Kept, as the evidence this document cites:** `~/.cache/aet-1208/`. It holds:
    - `baseline/` and `new/`, the binaries;
    - `tools/`;
    - `v-base`, `v-new` and `v-new2`;
    - `pins/` and `reach/`;
    - `mut/`;
    - `adv/`: 5,254 specifications, `REPORT.md`, `results/`;
    - `pr-title-body.md`.
  - `1207`'s `~/.cache/aet-1207/` is kept for `1207`'s FEEDBACK.
- **Tokens:** minted per push (the branch and the PR; the READY commit) and revoked after each, every revocation `HTTP 204`. Three prep files from a refused fill (a typo in my own check fragment) held the token; they were deleted after its revocation.

### 10.1 The architect's rulings on §8 (PR #182, 2026-11-16 21:40 UTC), and what was built

| # | Ruling | Done |
|---|---|---|
| F1, F5 | Build in this PR. Without a power tree, judge the MCU domain by step 16's consumer rule: floor = max(1620 mV, the `bor_level`'s VBOR falling max), ceiling 3600 mV, one rule and one floor. Its name follows a stated test: `DomainOutsideConsumer` if that message reads true with no tree, else keep `McuSupplyOutsideRange` with the shared floor | **Built** (`88ca5b0` old behaviour, `74f8c42` build), below |
| F2 | Keep `SpiSpeedBelowKernelRange` at `solve`, recorded as a deviation from §1.3 for that reason; no backlog row | Kept. **Deviation from §1.3:** its message names the controller the search chose (`SPI1` against a hand-set `SPI4`, measured), so moving it would change an existing refusal's bytes for a cosmetic gain |
| F3 | No change here; a §7.1 row for the next eda reduction work order | None here (the architect's row) |
| F4 | Backlog: the floor's maximum over [min, max], with its own measured stop | None here (the architect's row) |
| F6 | Agreed, declined | — |
| Forks | All four confirmed | — |

**The name.**
- **The rule's test:** `DomainOutsideConsumer`'s message reads "`<domain>`: the domain's *min* to *max* mV is not inside STM32H753ZITx (MCU)'s 1620 to 3600 mV with its minimum above the consumer's floor of *floor* mV (Table 111 …; VBOR*n* falling max … Table 115 …)". It names the domain and the consumer, never a regulator, so it reads true with no tree.
- **So:** the no-tree path uses it, and `McuSupplyOutsideRange` (variant, `kind`, `id`, `mcu_supply`) is deleted.
- **One function computes the floor.** `power.rs` gains `mcu_consumer` (the MCU's consumer entry: range, floor, both citations) and `domain_inside_consumer` (one domain against one consumer), which step 16 now calls, and `mcu_without_tree`, which step 22 calls when `power` is `null`. Step 16's own behaviour and bytes are unchanged: the corpus and the power-tree tests show it [measured].
- **Fence:** this edits `power.rs`, outside the work order's named fence, as the ruling requires ("by step 16's consumer rule").

**A discrepancy in the ruling's tests, measured and built to the rule.** The ruling lists "`bor_level` 0 at 1.62 V passes" and "a domain exactly at the BOR maximum". Step 16's rule, which the ruling names, gives different results:
- **The floor at level 0 is not 1620 mV.** The row's `bor_falling_max_millivolts[0]` is VBOR0's 1680 mV (DS12117 Table 115). So at `bor_level` 0 the floor is 1680 mV, not 1620, and a rail whose minimum is 1620 mV is **refused**.
- **The rule is strict.** It requires the minimum to be *above* the floor (`dmin <= floor` refuses). So a minimum exactly at the BOR maximum is refused, and 1 mV above passes.
- **What I built and tested:** the rule as ruled, one rule and one floor, at its real edges (`aet_1208_without_a_tree_the_mcu_rail_is_held_above_its_bor_floor`):
  - at `bor_level` 3, minimums 2679 and 2680 mV refused, 2681 mV passes;
  - at `bor_level` 0, minimums 1620 and 1680 mV refused, 1681 mV passes.

  Before (`88ca5b0`), every one passed.
- **Not built, for your decision:** if `bor_level` 0 was meant as "no BOR", so that 1.62 V should pass, that is a change to step 16's rule for trees too.
- `1207`'s speed-table test edge at 1620 mV (`bare_rail(1800, 100, true)`, `bor_level` 0) moves the same way: step 21 passes the table's edge, and step 22's 1680 mV floor refuses it.

**F1's repro** (`aet_1208_a_rail_below_the_bor_floor_is_refused`, cli). 1.9 V ± 1 % at `bor_level` 3: `validate` and `generate` now print `DomainOutsideConsumer` (minimum 1881 mV against the 2680 mV floor) and write nothing. Before, both passed and `generate` wrote a board. The 5.0 V pin (`aet_1208_the_mcu_rail_is_judged_against_its_row`) now prints the same refusal's bytes.

**§1.1 re-run (measured): no stop.**
- The probe's BOR column again hits only `invalid-zero-domain` (already refused).
- **Corpus:** all 23 specifications give byte-identical `validate`, `solve` and `generate` output and generated files against `d539bb1`, 153 of 153 records (`v-new3`).
- **The adversary's 5,254 specifications, re-run on `validate`** (`adv/recheck-f1f5.txt`): 255 moved since `7c9bb8f`.
  - **171 follow the rule exactly**, each now `DomainOutsideConsumer`, each with no power tree and its minimum at or below max(1620, VBOR) or its maximum over 3600 mV:
    - 91 were `ok`;
    - 80 were `McuSupplyOutsideRange`.
  - **84 are the `d5d600e` nominal pull-up fix**, which `generate` already refused.
  - None is a corpus specification.


### 10.2 Mutation at `74f8c42` (after the ruling)
10 mutants, one at a time, against `aethelgard-ir` and cli `reserved_references_tier1` and `power_board_tier1`; all 10 killed by a test failure, none by compilation [measured, `~/.cache/aet-1208/mut/`]. The ruling's three replace `McuSupplyOutsideRange`'s:

| Mutant | Killed by |
|---|---|
| `BusTooFast` off | ir, cli |
| pull-up at the maximum off | ir, cli |
| capacitance off | cli |
| motor timing off | cli |
| serial off | ir, cli |
| step 22 off | ir, cli |
| the BOR floor ignored (in the shared `mcu_consumer`, so step 16 too) | ir, cli |
| the floor's edge loosened (`<=` to `<`, in the shared `domain_inside_consumer`) | ir |
| the nominal pull-up check off | cli |
| `emit` trusting its caller | cli |

### 10.3 Gates after the ruling (`74f8c42` plus this FEEDBACK) [measured]
- **Gates:**
  - `cargo fmt --all -- --check`: clean;
  - `cargo clippy --workspace --all-targets --features aethelgard-cli/with-tools -- -D warnings`: clean;
  - `cargo test --workspace --no-fail-fast`: 117 test binaries, 0 failures.
- No `include_str!` changed, so the package step stands as measured in §9.
- `TIER2_MIN` is unchanged (160).
- **The READY commit** is this file only; its own CI run is the one the architect reads.
- **Tokens:** minted for this push and revoked after it (`HTTP 204`).
