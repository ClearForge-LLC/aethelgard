# AET-WO-1207 FEEDBACK — `validate` refuses the rest of what `solve` refuses statically, and an SPI device whose modes the controller cannot drive

Builder: Claude (Opus 5.5), session started 2026-11-16 ~16:50 UTC, worktree `~/aethelgard-1207`, branch `wo/aet-1207-validate-rest` from `origin/main` `1178ca8`. The work order is read from `origin/claude/argon-repo-docs-synthesis-rjrb0d` (docs PR #174); this branch does not commit it.

Labels: **measured** (run here), **read** (read at source, file:line), **inferred** (reasoned, not run).

## 0. Predictions (committed before any measured run)

Written at ~17:20 UTC, after reading the code and before building or running anything.

| # | Prediction | Basis |
|---|---|---|
| P1 | §1.1 finds **no** stop. No specification `generate` accepts today carries any of the three solve refusals (each is `solve`'s, and `generate` solves). No project is named `com0` or `lpt0`. No serial or motor net is named for an emitter net (`validate`'s A5 already refuses one, `1206`). Every SPI part with a row (ICM20948, ICM42688P, W25Q64JVxIQ) lists mode 0. | read: `solve.rs:921-1000`, `names.rs:287`, `parts.json` rows |
| P2 | The corpus is 23 specifications (1206's 24, less `spike/4300f/s2/offboard.json`, deleted by `4402`) and 26 scenario files. The scenarios override only `pull_up_ohms`, `droop`, `max_capacitance_femtofarads`, `line_held_low` and `spi_device_absent`, so none can change a verdict of these rules. | measured: `git ls-files` (listing only, before any rule ran) |
| P3 | `RiseTimeTooSlow` and `MissingSpiConstraint` are reachable today from a specification that `validate` passes. `SupplyOutsideSpeedTable` may not be: every SPI bus is on the MCU's supply (step 5a), and a device on it is bounded by its part's supply and I/O range. It is reachable only where no device bounds the bus: a bus with no device (if `validate` allows one), or every device behind a crossing. If none is reachable, the move is recorded as such, not claimed. | inferred |
| P4 | The three move to a new last step of `Specification::validate` (step 21, after `1206`'s 20), in `solve`'s order: each I2C bus's rise time, then each SPI bus's supply and its SCK and MOSI constraints. Their kind, `id`, `claimants` (the bus) and message stay as before, so the CLI needs no edit: `1206`'s optional `claimants` already carries a validation refusal that names someone. `SupplyOutsideSpeedTable` fires only inside the SPI-bus loop, as `solve`'s does (never with no SPI bus). | read: `cli/src/main.rs:290-305`, `solve.rs:925-955` |
| P5 | `solve`'s three copies become unreachable: `solve_inner` calls `spec.validate()` first (`solve.rs:322`), so no library caller reaches them without `validate`. They are deleted from `SolveError`, with `expect`s where `solve` reads what step 21 checked. | read |
| P6 | First refusals change only for specifications with two faults: a step-21 refusal now comes before a `solve`-time refusal, including `BusTooFast` on an earlier bus (per-bus interleaving in `i2c_ceilings`) and `i2c_electrical`'s two (which `solve` ran before `spi_lines`). | read |
| P7 | The adversarial search finds other refusals `solve` makes from the specification alone, which `validate` passes: at least `BusTooFast`, `PullUpBelowSinkFloor` and `BusCapacitanceOverLimit` (validation's own variants, raised only by `solve` on a specification), and probably motor timing. They are findings for the architect, not builds: the work order names three. | read: `solve.rs:991-1048`, `motor_timing` |
| P8 | The driven SPI mode is one fixed value, **mode 0**: `svd-hal` `spi_code` (`board.rs:1898-1910`) composes `CFG2` from `MASTER`, `SSM` and `AFCNTR` only, so `CPOL` and `CPHA` are 0 for every controller, independent of the solve and the part. No §7 stop. `ir` cannot depend on `svd-hal`, so the constant lives in `ir` and an `svd-hal` test pins it to the `CFG2` word's `CPOL`/`CPHA` bits. | read |
| P9 | The mode refusal sits beside `check_spi_clock` in step 14, and cites the row's bus citation. No real row excludes mode 0, so the refusal is reachable only with a modified table; the test reaches it through a table parsed from mutated JSON. | read: `parts.rs:1029-1050`, `lib.rs:3078` |
| P10 | `spelled` reading `names::spelled_nets` (one list) closes F1: on `main`, a library caller of `schematic::emit` with a serial net `U1_VCAP1` gets a board (VCAP1 shorted to a UART pin); after, `ReservedName`. A motor label behaves the same. | read: `schematic.rs:669-684`; `1206` §5 |
| P11 | Every emitted file of every corpus specification is byte-identical; `TIER2_MIN` stays at `main`'s 160 (no tier-2 test added). | inferred |

## 1. §1.1 measurement (before any rule was wired): no stop

Harness: `~/.cache/aet-1207/tools/` (`verdicts.sh`, `probe.py`, `reach.py`). The baseline binary is `main` at `1178ca8`, built in a capped scope (`~/.cache/aet-1207/baseline/`).

- **The corpus:** 23 specification files (every tracked JSON with a `project_name`) and 26 scenario files [measured]. Since `1206`'s 24, only `spike/4300f/s2/offboard.json` is gone (deleted by `4402`). The scenarios override only `pull_up_ohms` (with `bus`), `droop`, `max_capacitance_femtofarads` (with `net`), `line_held_low` and `spi_device_absent`. None of those sets a rise time, removes a constraint, moves a rail, renames anything or changes a part. `spi_device_absent` keeps the device in the specification (`campaign.rs:396`) [read]. So no scenario can change a verdict of these rules.
- **Baseline verdicts** (`validate` / `solve` / `generate`) [measured]:
  - 15 generate;
  - `imu-baro-1v8` and `sentinel-02-own` are `UnknownPart` at `generate`;
  - three `solve/` fixtures are `Infeasible`;
  - three are refused by `validate` (the two `1200.repro` files for their schema version, `invalid-zero-domain` for `ZeroNominalVoltage`).
- **Each candidate rule over the corpus** (`probe.py`, `~/.cache/aet-1207/probe-1.1.txt`) [measured]:

  | Rule | Hits | Where |
  |---|---|---|
  | `RiseTimeTooSlow` (worse I2C line over 1000 ns) | 0 | |
  | `MissingSpiConstraint` | 24 | all in the two `1200.repro` files, already `UnsupportedSchemaVersion` |
  | `SupplyOutsideSpeedTable` | 1 | `invalid-zero-domain`, already `ZeroNominalVoltage` |
  | `com0` / `lpt0` project name | 0 | |
  | serial or motor net named for an emitter net | 0 | |
  | SPI device whose row lacks mode 0 | 0 | every SPI row (ICM20948, ICM42688P, W25Q64JVxIQ) lists `[0, 3]`; the 9 `X` parts are the `1200.repro` files, already refused |

  No rule refuses a specification `generate` accepts.
- **Reachability of the three, on the baseline** (`reach.py`, `~/.cache/aet-1207/reach*/`) [measured]. Each is reachable from a specification that `validate` passes and `solve` refuses:
  - `RiseTimeTooSlow`: a line at 1001 ns.
  - `MissingSpiConstraint`: SCK or MOSI with no constraint.
  - `SupplyOutsideSpeedTable`: reachable only where no device bounds the SPI bus's rail. Every device must share its bus's and the MCU's rail (`SupplyMismatch`), and each row's supply range lies inside 1620 to 3600 mV except TMP117's (1800 to 5500 mV). So the reachable cases are an SPI bus with no device (which `validate` and `solve` accept at 3.3 V) at 1600 or 3800 mV, or a lone TMP117 at 3800 mV.

## 2. Results beside the predictions

| # | Result | Label |
|---|---|---|
| P1 | Held: no stop. | measured |
| P2 | Held: 23 specifications, 26 scenarios, and the overrides as listed. | measured |
| P3 | Half right. All three are reachable, `SupplyOutsideSpeedTable` included: an SPI bus with no device, or with a lone TMP117 above 3.6 V, is not bounded by any part (§1). | measured |
| P4 | Held. Step 21 sits after step 20, in `solve`'s order. The CLI needed no edit: `1206`'s optional `claimants` carries the three, and `validate` prints `solve`'s bytes for each (§4.2). | measured |
| P5 | Held. `solve` calls `spec.validate()` first (`solve.rs:322`), so no library caller reaches its copies without `validate`. They are deleted, and `solve` `expect`s what step 21 checked. | read, measured |
| P6 | Held in kind (§5); the adversarial pass measured it (§6). | measured |
| P7 | See §6 and F1. | measured |
| P8 | Held: mode 0, one fixed value. `the_cfg2_word_drives_the_spi_mode_validate_judges` (svd-hal) pins it for both kernels and every `MBR`. | read, measured |
| P9 | Held, with one change: the mode judgement lives inside the SPI part check (`check_spi_device`, formerly `check_spi_clock`). So it runs at step 14 and again in `SolvedTopology::validate`'s part limits (`RUL-16`), as the clock check does. | measured |
| P10 | Held. On `main`, each of four library calls emitted a board (`4c61fad`); now each is `ReservedName`. | measured |
| P11 | Held for the bytes: 153 of 153 records and 210 of 210 generated-file hashes are identical (§4.1). `TIER2_MIN` stays at `main`'s 160. | measured |

## 3. What was built, by work-order item

1. **§1.1:** above.
2. **§1.2, the three refusals (F3).**
   - **Step 21, `Specification::speed_inputs`,** comes after step 20, so every existing first refusal stays where it was. In `solve`'s order:
     - each I2C bus's worse line (the larger of SDA's and SCL's `max_rise_time_nanoseconds`) is at most 1000 ns (`RiseTimeTooSlow`);
     - then, bus by bus, each SPI bus is on an MCU supply inside 1620 to 3600 mV across its whole tolerance (`SupplyOutsideSpeedTable`, judged only beside an SPI bus, as in `solve`);
     - and its SCK, then its MOSI, carries a net constraint (`MissingSpiConstraint`).
   - **Same JSON:** the three are new `IrValidationError` variants with `solve`'s messages, and `id()` and `claimants()` give the bus.
   - **Which applies (the work order's "record which applies"): `solve`'s guard is not kept.** A library caller cannot reach it without `validate`, because `solve_inner` runs `spec.validate()` first. `SolveError::{RiseTimeTooSlow, SupplyOutsideSpeedTable, MissingSpiConstraint}` are deleted. `i2c_ceilings` and `spi_lines` `expect` what step 21 checked, and `spi_lines` is now infallible.
3. **§1.3, Windows names (F7).** `com0` and `lpt0` join `names::WINDOWS_RESERVED` (now 24 names), under `1206`'s folding rule: the stem before the first `.`, trailing spaces dropped, any case.
   - The superscript forms are not ASCII, and every identifier already is.
   - A project name in upper case is refused earlier by the crate-name rule, so `windows_reserved` carries the case tests (`com0.txt`, `LPT0 `).
4. **§1.4, the emitter's names (F1).** `schematic.rs` `design` takes its `spelled` set from `aethelgard_ir::names::spelled_nets`: domains, I2C and SPI lines, chip selects, serial lines, motor outputs, sense nets. That is the one list step 20 reads, and it is now `pub`. eda's own partial copy (the inline list and `schematic_power::spelled`) is deleted.
   - Library-level test: `aet_1207_the_emitter_refuses_a_serial_or_motor_net_named_for_its_own` (cli `reserved_references_tier1.rs`). It solves a valid specification, renames one net in both the specification and the topology, and calls `schematic::emit`.
   - Its four cases each emitted a board on `main` (`4c61fad`), and each is `ReservedName` now:
     - serial `CONSOLE_TX` as `U1_VCAP1`;
     - serial `CONSOLE_RX` as `gnd` (`GND and gnd`);
     - motor `MOTOR1` as `U1_VCAP2`;
     - motor `MOTOR2` as `GND`.
5. **§1.5, SPI modes (Grok 5).**
   - **Where the driven mode is set** [read]: `aethelgard-svd-hal` `board.rs` `spi_code` composes `CFG2` as `MASTER | SSM | AFCNTR`. So `CPOL` (bit 25) and `CPHA` (bit 24) are 0, mode 0, for every controller, kernel and divider, independent of the solve and the part. No §7 stop.
   - `ir` cannot depend on `svd-hal` (`workspace_shape`), so `aethelgard_ir::parts::DRIVEN_SPI_MODE = 0` states it. The svd-hal unit test `the_cfg2_word_drives_the_spi_mode_validate_judges` reads `CPOL`/`CPHA` from the CMSIS header and asserts the word `spi_code` composes carries that mode.
   - **The refusal:** `SpiModeNotDriven`. Its `id` is the device reference, and its message names the part, the row's `modes` and the row's bus citation.
     - It is judged inside `check_spi_device` (step 14, after the clock), and again from the topology (`SolvedTopology::validate`'s part limits, `RUL-16`).
     - Today it is latent: every SPI row lists `[0, 3]`.
     - Tier-1 test: `aet_1207_a_device_whose_modes_exclude_the_driven_one_is_refused`, a unit test in `parts.rs` that drives the private `validate_parts` with a table parsed from mutated JSON. With `[1, 2, 3]`, `[3]` or `[2]` it is refused; with `[0]`, `[3, 0]` or `[0, 1, 2, 3]` it passes. Before, every one passed (`4c61fad`).
   - **Per-device mode:** a question, not built (F5).

## 4. Evidence

### 4.1 Verdicts and bytes (measured: `~/.cache/aet-1207/v-base` against `v-new`)
- **All 23 specifications** give byte-identical `validate`, `solve` and `generate` stdout and exit codes, and an identical sha256 for every generated file. That is 153 of 153 records, with the 15 generating specifications' 210 files among them.
- **The baseline binary is `1178ca8`.** The new binary was measured twice: at `7b403a4` (`v-new`) and again at `55b258b` (`v-new2`), after the reduction and the adversarial fix. Both gave 153 of 153.

### 4.1a The other commands that read a specification (measured: `~/.cache/aet-1207/sub/`)
- **The five pinned specifications,** through every other subcommand that takes one, on both binaries: `emit --file`, `check speeds --spec`, `verify --spec`, `extract --spec` (with the reference's routed board). All print byte-identical bytes, each the moved refusal with `solve`'s `claimants`.
- **`validate` on stdin** changes as `validate --file` does: the moved refusal now, where it printed `ok` or `UnwitnessedAcknowledgementStale` before.
- **Not run:** `campaign` (§5), and `mcp`, which serves the same subcommands.

### 4.2 Tier-1 tests, each committed first with the behaviour before (`4c61fad`)
- **`aet_1207_validate_refuses_what_solve_refused_byte_for_byte`** (cli `reserved_references_tier1.rs`). It has five rows: the rise time, SCK, MOSI, the rail at 1600 mV with no device, and at 3800 mV with a lone TMP117. Each holds the bytes `1178ca8` printed from `solve` and `generate`, which they still print.
  - **Before:** `validate` passed each specification that acknowledges nothing, and refused the reference's acknowledgements as `UnwitnessedAcknowledgementStale`.
  - **Now:** `validate` prints `solve`'s bytes, with or without acknowledgements.
- **`ir/tests/validate.rs`** adds three tests, for the three step-21 rules at each boundary, both ways:
  - 1000/1001 ns either line, and `u32::MAX`;
  - the rail at 1620 and 1618 mV minimum, 3600 and 3603 mV maximum;
  - no SPI bus, nothing judged;
  - SCK, then MOSI;
  - the order: I2C before SPI, and supply before a line.
- **Windows names:** `a_project_named_for_a_windows_device_is_refused` held `com0` and `lpt0` in its pass list on `main`; they are now refused.
- **The emitter:** §3.4. **The modes:** §3.5.

### 4.3 Tests that encoded the old behaviour (not corpus verdicts)
- **`refuses_tolerance_out_of_range`** (`validate.rs`) built a ±50 % rail with a bare SPI bus, to test the tolerance's own range. `validate` passed it before, and `solve` refused it as `SupplyOutsideSpeedTable`. The test's board now drops its SPI bus too, as it already dropped its devices for `SupplyOutsidePart`.
- **Two `ir/tests/solve.rs` assertions** expected `SolveError::RiseTimeTooSlow`. They now expect it as validation's variant inside `InvalidSpecification`, with the same kind, `id`, claimant and message.

### 4.4 The CLI envelope
- **No CLI source changed.** The CLI's refusal carries `claimants` for any validation refusal that names one (`1206` §4.3). So the three print `solve`'s bytes from `validate` with no CLI edit [measured, §4.2].
- **`cli/src/extract.rs:120`'s comment** still says "`solve` refused a line with no constraint (`MissingBusConstraint`, `MissingSpiConstraint`)". It is outside the fence, and true in substance (the refusal now comes from `validate` inside `solve`): F4.

### 4.5 Mutation (diff-sized: one mutant per guard this work order adds or changes)
- **Method:** 13 mutants, one at a time, run first at `7b403a4` and again at `bc9d039` (after the reduction, the adversarial fix and the merge), against `aethelgard-ir` (every test), cli `reserved_references_tier1`, and svd-hal `the_cfg2_word_drives_the_spi_mode_validate_judges`. Harness and logs: `~/.cache/aet-1207/mut/`.
- **Result:** all 13 killed, each by a test failure [measured]. No mutant log holds `could not compile`, so none was killed by compilation. Both runs gave the same table.

| Mutant | Killed by |
|---|---|
| step 21's rise check off | ir, cli |
| step 21's supply check off | ir, cli |
| its minimum edge (`<` to `<=`) | ir |
| its maximum edge (`>` to `>=`) | ir |
| the SPI constraint check off | ir, cli |
| the mode judgement off | ir |
| `DRIVEN_SPI_MODE` = 3 | ir, svd-hal |
| `com0` dropped | ir |
| `lpt0` dropped | ir |
| serial nets dropped from `spelled_nets` | cli |
| motor nets dropped from `spelled_nets` | cli |
| the emitter's `spelled` emptied | cli |
| `CFG2` composed with `CPOL` set | svd-hal |

## 5. Behaviour changes a reviewer should know

- **`validate` with acknowledgements.** When a specification acknowledges unwitnessed fields, the CLI's `validate` solves it for the ledger (`cli/src/main.rs` `acknowledgements_current`), and reported a failed solve as `UnwitnessedAcknowledgementStale`.
  - For the three moved refusals, it now prints the refusal itself: `Specification::validate` refuses before the ledger is built. This is the intended move.
  - The others still print `UnwitnessedAcknowledgementStale`, as on `main`: `BusTooFast`, `PullUpBelowSinkFloor`, `BusCapacitanceOverLimit` (F1).
- **A doubly faulty specification may report a different first refusal.** Step 21 now runs before `solve`'s electrical checks. The adversarial pass measured 54 of 258 multi-fault rows (§6 D) where `generate` used to print `BusTooFast`, `PullUpBelowSinkFloor` or `BusCapacitanceOverLimit` first and now prints `MissingSpiConstraint` or `SupplyOutsideSpeedTable`.
  - **The per-bus case** is measured on `d3-24-i2c-12-spi` (`~/.cache/aet-1207/twobus/`), with `BusTooFast` on bus `B0` and a 1001 ns line on `B2`. Baseline `solve` printed `BusTooFast` (`B0`), from `i2c_ceilings`, which interleaves the two per bus. `validate` and `solve` now print `RiseTimeTooSlow` (`B2`).
  - Among the three moved kinds the order is `solve`'s: rise time, then supply, then the SPI line.
  - New `validate` and new `generate` agree.
- **`campaign` over a base specification with one of the three** [read: `campaign.rs:361-364`, not run, since it needs a firmware ELF]. The refusal now comes from `parse_spec`, with campaign's "scenario `<name>`, base: " prefix, where before it came from `solve_spec` without one. Kind, `id` and `claimants` are unchanged. This is the shift `1206` §5 recorded for `MissingBusConstraint`. No scenario override can produce one of the three (§1).
- **The Windows rule's message** now lists `com0` to `com9` and `lpt0` to `lpt9`. It changes for every Windows name. No corpus specification has one.
- **The ir README** gains step 21, `SpiModeNotDriven` (step 14 and topology item 14), and the solve table's row 1. It also corrects `t_max` to the larger rise time (the worse line), which had said "smaller" since `1206` §1.7 changed the rule.

## 6. Adversarial pass (targeted)

**What ran.** Two parts, at `7b403a4`. After it came the reduction (test-only), the Windows-message fix and the docs merge. The corpus differential (153 of 153), the subcommand check (§4.1a) and the mutants were rerun on the final code; the 8,609-specification differential was not.
- **The CLI differential.** A fresh subagent was given both binaries and the work order's claims, not this document. It generated 8,609 specifications over the reference, `sentinel-02-power`, `motors-pwm`, `serial-four` and `d3-24-i2c-12-spi`, and ran `validate` and `solve` on both binaries for each, and `generate` for 2,156. Harness, specifications and report: `~/.cache/aet-1207/adv/` (`harness.py`, `specs/`, `results.json`, `REPORT.md`).
- **The library path and the `modes` forms.** Run here (`adv/library-path.txt`, `adv/modes-forms.txt`).

**Results** [measured; I spot-checked the supply edges and one stale-acknowledgement case against both binaries]:
- **A. `validate` passes and `solve` or `generate` refuses (the search).** 147 specifications, none with one of the three moved kinds, and each identical on the baseline. So the change introduced none.
  - **81 are static** (decidable from the specification): `BusTooFast` 40, `BusCapacitanceOverLimit` 14, `PullUpBelowSinkFloor` 13, `SerialBaudOutOfRange` 5, `SerialBaudTolerance` 3, `SpiSpeedBelowKernelRange` 2, `MotorTiming` 1, plus 3 `Infeasible` from the pin search. F1.
  - **66 depend on the pin table or the vendored models:** `RateAbovePinSpeed` 52, `LoadAboveSpeedTable` 9, `RiseTimeBelowPinSpeed` 3 (each names a chosen pin), and `UnknownPart` 2 (`1206` F10).
- **Boundaries.** On a 9×9 grid of SDA and SCL rise times, `validate` refuses exactly when the worse line is over 1000 ns. Over 809 rail variants it refuses exactly when there is an SPI bus and min < 1620 or max > 3600:
  - 1621 mV ± 0.1 % refused, 1622 accepted;
  - 3596 accepted, 3597 refused;
  - with no SPI bus, accepted, as on `main`.
- **Duplicate constraints on one net** are `DuplicateNetConstraint` on both binaries, so "which one is read" is unreachable.
- **B. False refusals:** only `com0` and `lpt0` (4 specifications), the intended rule. The 14 others that show here print `UnwitnessedAcknowledgementStale` from `validate` on both binaries; the change does not cause them.
- **C. Bytes:** 0 differences.
  - Each of 328 specifications refused by baseline `solve` and new `validate` with one of the three gives identical bytes.
  - Across both binaries, 11,056 same-command, same-kind refusals have identical bytes.
  - 656 specifications generated on both, and their 9,184 output files have identical sha256.
- **D. Multi-fault first refusals:** §5.
- **E. Project names:** 6,756 names tried, including every one-character edit of the 24 reserved names. Only `com0` and `lpt0` changed verdict.
  - **Found:** the refusal's message still listed `com1` to `com9` and `lpt1` to `lpt9`.
  - **Fixed** (`55b258b`), with an assertion on the listed range.
- **The library path** (§3.4's mechanism, 35 renames of a serial or a motor net, each handed to `schematic::emit`).
  - Every emitter-own net is now `ReservedName` from the emitter in any case: `GND`, `NRST`, `SWDIO`, `SWCLK`, `U1_VCAP1`, `U1_VCAP2`, `U4_WP`, `U4_HOLD`, `U2_SDO`, `unconnected-x`.
  - Names that are not emitter nets still emit, correctly: `VBAT` without a power tree, `U1_VCAP3`, `U3_SDO`, and `Unconnected-x` (KiCad's prefix is case-sensitive).
  - **Residual, F3:** a serial or motor net named for another net the specification spells (`VDD_3V3`, `I2C_SDA`, `MOTOR4`) still emits a shorted board through the library path. `validate` refuses each (`NetShadowsDomain`, `DuplicateNetName`), but the emitter never compared the specification's own names with each other.
- **`modes` in other forms** (the work order's empty list, the driven mode twice, another case or form). The table parser refuses each before any judgement reads the list, so only a well-formed list reaches `SpiModeNotDriven`:
  - empty;
  - `[0, 0]` and `[3, 3]`;
  - strings (`"0"`, `"mode0"`), floats, `-1`, `256`, `4`, `true`, `null`, a bare `0`, `[[0]]`, a map;
  - the key spelled `Modes`.
- **Not run:** `SpiModeNotDriven` from the CLI (no table row reaches it; the unit test reaches it through a mutated table), and `campaign` (it needs a firmware ELF).

## 7. Reduction pass

The four questions over the diff:
- **(1) A fact stated twice:**
  - The three refusals' messages now live only in `IrValidationError`: `solve`'s three variants are deleted.
  - The emitter's `spelled` is `names::spelled_nets`; eda's inline list and `schematic_power::spelled` are deleted.
  - The tolerance test's bare board is the new `bare_rail` (`702dcc1`, −9 lines).
  - **Kept, reported:** `parts::DRIVEN_SPI_MODE` restates what svd-hal's `spi_code` composes, because `ir` may not depend on `svd-hal`. A test pins the two together.
- **(2) Unused:** `spi_lines`' supply bounds and their band constants went with the check.
- **(3) Two mechanisms that could be one:** the clock and mode judgements are one `check_spi_device`, called at step 14 and from the topology.
- **(4) A workaround for a gone problem:** none.

Reduction pass (diff): −9 lines net in the pass (the build itself deleted 65 lines from `solve.rs` and 23 from eda, against 133 added in `lib.rs`) · 3 duplicates removed (solve's variants, eda's `spelled`, the tolerance test's board) · 0 dead paths removed · 0 protected kept · 1 finding below (F6)

## 8. Findings and questions

| # | Finding | Where | Evidence | Disposition | Decision needed |
|---|---|---|---|---|---|
| F1 | `validate` still passes specifications `solve` refuses from the specification alone: `BusTooFast`, `PullUpBelowSinkFloor`, `BusCapacitanceOverLimit` (validation's own variants, which `1206` §1.5 kept at `solve`, pinned in `aet_1206_one_error_set_keeps_solves_bytes`), and `MotorTiming`, `SerialBaudOutOfRange`, `SerialBaudTolerance`, `SpiSpeedBelowKernelRange` (each judged against the reset clock, which no specification field changes) | `solve.rs` `i2c_ceilings`, `i2c_electrical`, `motor_timing`, serial and SPI clock checks | measured (§6 A: 81 specifications) | not moved: the work order names three | yes: a row for the next `validate` work order, if these should move |
| F2 | The MCU's own supply is not judged. An MCU rail of 3.8 V or 5.0 V with one TMP117 (1.8 to 5.5 V) and no SPI bus passes `validate` and **generates a board** on `main` and on this branch; DS12117 operates the STM32H753 at 1.62 to 3.6 V. The only bound on that rail is the speed-table rule, which fires only beside an SPI bus | `lib.rs` step 5, `power_parts.json` MCU row | measured (`~/.cache/aet-1207/reach4/`) | outside the fence | yes: a §7.1 row (the MCU row's supply range, judged like a part's) |
| F3 | A library caller of `schematic::emit` with an unvalidated specification whose serial or motor net is named for a domain or another spelled net (`VDD_3V3`, `I2C_SDA`, `MOTOR4`) still gets a shorted board. The emitter holds its own nets against the spelled ones, never the spelled ones against each other; `validate` refuses them (`NetShadowsDomain`, `DuplicateNetName`) | eda `schematic.rs` `design` | measured (§6) | residual: the work order's F1 is the emitter's own nets | yes, if `emit` should refuse what `validate` refuses about names |
| F4 | `cli/src/extract.rs:120`'s comment names `solve` as refusing `MissingSpiConstraint`; `validate` does now (inside `solve`) | cli `extract.rs` | read | outside the fence (2302's crate is the CLI) | no |
| F5 | **Per-device SPI mode (the work order's question):** the generator could drive one. `CFG2`'s `CPOL` and `CPHA` belong to a controller, and devices on one bus share it. So a mode per bus is one word per controller (`spi_code` would read it from the devices' rows), and a mode per device on a shared bus means rewriting `CFG2` with `SPE` clear before each transfer. Today every row lists `[0, 3]`, so nothing needs it | svd-hal `spi_code`, `parts.json` | read | not built | yes, when a part needs mode 1, 2 or 3 alone |
| F6 | §6's split-on-touch names `ir/src/lib.rs`, touched here (+133 −19 lines: four variants, step 21, the SPI device check). **Declined:** a split is a change of its own, outside this fence, and `4211` (#173) has open hunks in `ir` (`parts.rs`, possibly the two `Lifecycle` enums), which a split would turn into a rewrite. This work order's code went beside the code it extends, as `1206` F8 did | roadmap §6 | read | declined with that reason | no |
| F7 | `SpiModeNotDriven` is latent: no row today lacks mode 0, so no CLI specification reaches it. It guards the next row; the unit test reaches it through a mutated table, and its mutant is killed | `parts.rs` tests | measured | as built | no |

**Forks for the architect to confirm** (each beyond the work order's literal list, made for the reason given):
- **`schematic_power::spelled` deleted.** That file is outside the named fence (`schematic.rs`, `spelled` only). It was the power tree's part of the list `spelled_nets` replaces, and nothing else read it.
- **`names::spelled_nets` made `pub`,** so eda reads the one list.
- **The mode judgement also runs on the topology** (`SolvedTopology::validate`'s part limits, `RUL-16`), because it lives in the one SPI part check.
- **The Windows rule's message** lists `com0` and `lpt0`, so it changes for every Windows name (adversarial E).
- **The ir README:** step 21, `SpiModeNotDriven`, the solve table, and the stale "smaller" corrected to "larger".
- **`refuses_tolerance_out_of_range` changed** (§4.3).

**Peers told:** `4211` (inbox, 18:40 UTC): where this branch's `parts.rs` hunks sit (the constant above `pub enum Bus`, the test at the end of the test module), and that the test builds its expected message from the W25Q64JVxIQ row's own citation.

## 9. Where everything ran, and the gates

- **Where:** host cargo in a capped user scope (`systemd-run --user --scope -p MemoryMax=32G -p MemorySwapMax=0`, with `memory.swap.max` read back as 0). The differentials and the adversarial harness ran in a 16 GiB, swap-0 scope. No container, Renode or KiCad was started.
- **Tier 2 here:** `cargo clippy --workspace --all-targets --features aethelgard-cli/with-tools -- -D warnings` is clean, so the tier-2 tests compile [measured]. None sets a rise time above 1000 ns, removes an SPI line's constraint, or moves the MCU rail; their net edits are consistent renames [read: grep over the `with-tools` test files]. CI's tier-2 run is the measurement.
- **This work order adds no tier-2 test,** so `TIER2_MIN` is `main`'s, 160. Its new tests are tier 1, in existing binaries (`validate`, `solve`, `reserved_references_tier1`, and the `ir` and `svd-hal` unit tests), so it adds no `ci/` weight.
- **Gates on the final tree** (`687026b`, after merging `origin/main` at `311398d`, docs only) [measured]:
  - `cargo fmt --all -- --check`: clean;
  - `cargo clippy --workspace --all-targets -- -D warnings`: clean;
  - `cargo test --workspace --no-fail-fast`: 117 test binaries, 0 failures.

## 10. CI and the stop line

- **First CI run (run 43920679872, at `97b181f`): `gates` failed in its package step** [measured]. The mode unit test (§3.5) read the reference specification with `include_str!("../../../examples/...")`, outside the `aethelgard-ir` crate, and the step refuses any include target missing from the package. The suite had not caught it, because the test passes in the workspace.
  - **Fix:** the test reads the crate's own `tests/fixtures/sentinel-02-power.json`, whose first SPI device is also `U4`, a W25Q64JVxIQ.
  - **Checked:** the step's include check, run locally on the packaged `ir` crate, finds all four targets.
  - That run's other jobs finished green at the time it was read: `cli`, `pinned`, `pinout`, both images, the four `ardupilot` jobs, and tools shards 1, 3, 4, 5 and 7 (19:33 to 24:32).

- **CI on PR #178 at `0019d70`** (run 43926160268): **green** [measured].
  - **Green jobs:** `gates` (fmt, clippy, tier 1, the package step), all eight `tools-shard` jobs, the `tools` aggregate (`tier2_tests_passed=160 floor=160`: `TIER2_MIN` is exact and unchanged), `shards`, `cli`, `pinned`, `pinout`, `engine-image`, `ardupilot-image` and its four `ardupilot` jobs.
  - **Shard times:**

    | Shard | Time |
    |---|---|
    | 0 | 23:01 |
    | 1 | 22:59 |
    | 2 | 28:41 |
    | 3 | 29:30 |
    | 4 | 31:16 |
    | 5 | 25:32 |
    | 6 | 24:25 |
    | 7 | 18:03 |

    `gates` took 25:07. Seven shards are over the 20-minute clause, so its one re-run does not apply to one shard. The same hour's runs of other branches are as slow [measured, from their run timings]:
    - `4211`: run 43920671965, shards 20 to 31 min, `gates` 23; run 43926463946, shards 18 to 28, `gates` 17.
    - `4403`: run 43923548434, shards 20 to 36, `gates` 23.

    So the slowness is the runners shared by four builders, not this change. This work order adds no tier-2 test and no `ci/` weight, and the `shards` job's plan is `main`'s. The READY commit changes only this file, and its own run is the one the architect reads.
- **Merge of `main` at `55ed4c4` (`4211`, #173), after that run.** The W25Q64JV row is NRND, and every specification placing it carries an `accept_lifecycle` entry.
  - **One conflict,** in `refuses_tolerance_out_of_range`, resolved to this branch's `bare_rail`. That helper, and the CLI test's `bare_spi_bus`, now clear `accept_lifecycle` with the devices they remove, as `4211`'s own edit of that test did: an acknowledgement of a part no longer placed is refused.
  - **After the merge** [measured]:
    - **Bytes:** against a baseline built from `55ed4c4` itself (`~/.cache/aet-1207/baseline2/`, `v-b2` against `v-n3`), 153 of 153 records are identical, including 210 generated files from 15 specifications.
    - **Tests:** the five pinned rows still print `1178ca8`'s bytes from `solve`, `generate` and `validate`.
    - **Gates** (`992611a`): `cargo fmt --all -- --check` clean; clippy `--workspace --all-targets --features aethelgard-cli/with-tools -D warnings` clean; `cargo test --workspace --no-fail-fast` 117 binaries, 0 failures.
  - The temporary `main` worktree and its 1.6 GB target are removed.
- **Disk:**
  - No agent worktree was created (the adversarial subagent ran without one), and the mutants ran in this worktree, each restored.
  - **Kept, as the evidence this document cites:** `~/.cache/aet-1207/` (538 MB). It holds:
    - `baseline/` and `new/`, the `1178ca8` and branch binaries;
    - `tools/`;
    - the verdict records `v-base`, `v-new` and `v-new2`;
    - `reach*/`, `twobus/` and `sub/`;
    - `mut/`, with both runs;
    - `adv/`: the 8,609 specifications, `results.json`, `REPORT.md` and the library and `modes` probes;
    - `pr-title-body.md`.
- **Tokens:** minted per push (the branch and the PR; the fix; the READY commit) and revoked after each, every revocation `HTTP 204`.
- **The READY commit** is this file only, on `992611a`. It is the last push, and its CI run is the one the architect reads.
