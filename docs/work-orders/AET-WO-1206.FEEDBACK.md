# AET-WO-1206 FEEDBACK — `validate` refuses what `generate` refuses

Builder: Claude (Opus 5.5), session started 2026-11-16 ~08:55 UTC, worktree `~/aethelgard-1206`, branch `wo/aet-1206-validate` from `origin/main` `8030fa5`. The work order is read from `origin/claude/argon-repo-docs-synthesis-rjrb0d` (docs PR #162). This branch does not commit it.

Labels: **measured** (run here), **read** (read at source, file:line), **inferred** (reasoned, not run).

## 0. Predictions (committed before any measured run)

Written at 09:10 UTC, after reading the code and before building or running anything.

| # | Prediction | Basis |
|---|---|---|
| P1 | §1.1 finds **no** example, fixture or scenario that `generate` accepts today and any candidate rule (A2, A4, A5, A7) would refuse. So no §7 stop on §1.1. | inferred: the corpus uses upper-case `U<n>`-style or descriptive references, and nets like `I2C_SDA` |
| P2 | Every JSON specification `validate` accepts today and the new `MissingBusConstraint` rule refuses is one `solve` already refuses with `MissingBusConstraint`. No such file is an *accepted* `generate` input, so this rule cannot trigger §7. Rust-built specs in `ir/tests/validate.rs` that declare I2C buses without net constraints will start failing; they are test edits, not corpus verdicts. | read: `solve.rs` `i2c_ceilings` runs on every I2C bus, after `validate` |
| P3 | A2 is mostly done already: `RESERVED_REFERENCES` (`lib.rs:1680`) refuses `i2c1`–`i2c4`, `spi1`–`spi6`, the USART/UART names and `timer<n>` at `validate` as `InvalidIdentifier`. The residue `generate` refuses and `validate` passes is `gpioa`–`gpiok` and `tim<n>` (motor timers), each refused by the twin only when the solve uses that port or timer. | read: `twin/src/platform.rs:787` `solved()`, `:500` |
| P4 | A4's reserved set is the pattern `[urc]<n>`, lower-case first letter, `n` with no leading zero. The emitter skips a reference spelled exactly `U<n>` (`schematic.rs:369-388`) and refuses only another spelling. The set depends on the specification, not on the solve: which `n` the emitter reaches is a function of the declared parts. The pattern is therefore wider than the exact set, so §1.3 licenses it only if §1.1 finds no use. | read |
| P5 | A5's reserved nets are `GND` (folded), `BATTERY` (`power::BATTERY_NET`, folded, when a power tree exists), any net starting `unconnected-` (case-sensitive, `pcb.rs:398`), and `<ref>_<suffix>` for each emitter-owned strap or support net. All are static in the specification: MCU `U<n>_VCAP1..2`, BME280 `_SDO`, W25Q64JVxIQ `_WP`/`_HOLD`, ICM20948 `_REGOUT`, the translator lines, and the buck's `_FB`/`_VCC`/`_BOOT`/`_SW`. | read: `schematic.rs`, `schematic_power.rs` |
| P6 | Keeping JSON byte-identical needs **no** second error type. `SolveError::claimants()` must delegate to the wrapped validation error for the four bus refusals, and the CLI's spec-validation refusal must carry `claimants` for `MissingBusConstraint`, because `solve`, `emit` and `generate` print `claimants` for it today. | read: `cli/src/main.rs:2045-2090`, `:272-303` |
| P7 | Of the three pre-empted guards: eda `UnsupportedAddress` is reachable from a library caller of `schematic::emit` that hands it an unvalidated `Specification` (the type has public fields), so it is **kept**. The twin's `ModelOffItsBus` is reachable from a hand-written `SolvedTopology` (`simulate --topology`, a library caller of `platform::emit`), so it is **kept**. `InvalidDeviceName` for `spi1`–`spi6` has no code of its own; it is the shared `taken` check, so there is nothing to delete. | inferred |
| P8 | Every emitted file stays byte-identical; `TIER2_MIN` does not change (no tier-2 test added). | inferred |
| P10 | **§1.8 (amendment 2, 09:35 UTC; prediction written 10:05 UTC before reading the table's rows or any §1.8 run).** Every spec `generate` accepts today uses only parts with a `parts.json` row (BME280, TMP117, W25Q64JVxIQ), so §1.8 triggers no stop. The specs it newly refuses are ones `generate` already refuses. That includes the four `solve/` fixtures and the `1200` repro if they use synthetic parts, which would move their first refusal from `Infeasible` (solve) to the new validate refusal. If so, the solver's own CLI tests (`solve_cli.rs`) lose their inputs; that is a finding, not a stop. The refusal's kind is `UnknownPart` with the emitter's message (`known:` list). Rust-built test specs with synthetic parts (`ir/tests/solve.rs`, `validate.rs`) start failing, and that is the bulk of the test edits. | inferred |
| P9 | **§1.7 (amendment, 09:25 UTC; prediction written 09:40 UTC before any §1.7 run).** No JSON example, fixture or scenario changes its solve: each declares SDA and SCL with equal `max_rise_time_nanoseconds`, so min and max agree. `SolvedTopology::validate` has the same `min` (`lib.rs:3115`, "slowest" over `ceiling_ns`) and gets the same fix. Some Rust-built tests in `ir/tests/solve.rs` pass unequal lines (`i2c_spec(100_000, 1001, 2000)` and similar); their expected refusals may move, and each move is a test that encoded the bug, to be listed. | read: the amendment; `solve.rs:749-777`; `lib.rs:3109-3122` |

## 1. §1.1 measurement (before any rule was wired) — no stop

Harness: `~/.cache/aet-1206/tools/` (`verdicts.sh`, `names.py`, `mutate.py`, `table.sh`). The baseline binary is `main` at `8030fa5`, built under a capped scope (`~/.cache/aet-1206/baseline/`).

- **The corpus:** 24 specification files (every tracked JSON with a `project_name`) and 26 scenario files (each over one of those bases; their overrides set only `pull_up_ohms`, `max_capacitance_femtofarads`, `droop`, `line_held_low` and `spi_device_absent`, never a name or a rise time) [measured]. Specifications built inside Rust tests were measured by wiring the rules and running the suite (§3).
- **Names in the corpus:** 195 references, 554 nets, 30 domains, 24 project names (`~/.cache/aet-1206/names-inventory.txt`) [measured]. Each candidate rule was run over them:
  - **A2:** 0 hits.
  - **A4:** 0 hits (every reference is canonical: `U2`…, `J1`…).
  - **A5:** 0 hits on a name the specification *gives*. The 15 `GND` hits are connector pins and plane layers that refer to ground, which the rule does not read.
  - **A7:** 0 hits.
  - **§1.8:** 0 hits. The table holds BME280, BMP581, ICM20948, ICM42688P, ICP10111, TMP117 and W25Q64JVxIQ, and every corpus part is one of them. The one exception, `X` in the two `docs/work-orders/AET-WO-1200.repro/` files, is already refused for its schema version.
  - **`MissingBusConstraint`:** 0 hits.
  - **§1.7:** 0 specifications declare SDA and SCL with unequal rise times.
- **Baseline verdicts** (`validate` / `solve` / `generate`) [measured]:
  - 13 specs generate;
  - `imu-baro-1v8` and `sentinel-02-own` are `UnknownPart` at `generate` (parts with a row but no vendored model, A9, untouched);
  - three `solve/` fixtures are `Infeasible`; `stress` and `sentinel-02-power` are `Unroutable`;
  - four are refused by `validate` (three for their schema version, one `ZeroNominalVoltage`).

## 2. Results beside the predictions

| # | Result | Label |
|---|---|---|
| P1 | Held: no rule refuses an existing specification `generate` accepts. | measured |
| P2 | Held for the JSON corpus (0 hits). The Rust-built test specs that missed a constraint were the `valid()` fixture of `ir/tests/validate.rs` (no `I2C_SCL` constraint: 11 tests) and one second bus; each now declares its lines. | measured |
| P3 | Held, and more precise than predicted. The twin refuses only *solved* ports and timers: `gpioa` and `GpioB` were refused on the reference while `gpioe`, `gpioh`–`gpiok`, `tim1`, `tim2`, `TIM8` and `tim17` generated. The static rule refuses all of them (the licensed widening, §5). | measured |
| P4 | Half right. The emitter draws **one** `U`, the MCU, as the first `U<n>` the specification does not spell, so that set is exact and static; A4 refuses `U` only there (`u2` passes until `U1` is spelled). `R<n>` and `C<n>` depend on how many passives the emitter draws (`r5` refused and `r30` generated on the reference), so the pattern is held for them. A leading zero also collides (`r01` against `R1`: KiCad's key, `pcb.rs` `reference_key`). | measured |
| P5 | Held, with one correction: the emitter's own nets are exact and static, as predicted, but **`unconnected-` is case-sensitive** (`Unconnected-x` generates), and `VBAT` is reserved only beside a power tree. | measured |
| P6 | Held. No second error type was needed; the CLI envelope gained an optional `claimants` (§4.3). | measured |
| P7 | Partly wrong. Kept: eda `UnsupportedAddress`, and the twin's SPI `ModelOffItsBus`. **Deleted: the twin's I2C `ModelOffItsBus`**, which nothing can reach. `spi1`–`spi6` have no code of their own. See §4.6. | read, measured |
| P8 | Held: every emitted file of every corpus specification is byte-identical (157 of 157 records), and `TIER2_MIN` stays 155. | measured |
| P9 | Held for the corpus (0 unequal-line specifications). Five tests encoded the bug and now assert the worse line (§3). | measured |
| P10 | Wrong on the `solve/` fixtures: they use real parts (TMP117, W25Q64JVxIQ), so their `Infeasible` refusals stand. Right on the rest: no stop; kind `UnknownPart` with the emitter's message; the test edits are synthetic parts (`FLASH`, `HDC1080`, `UNLISTED`). | measured |

## 3. What was built, by work-order item

All new rules are steps of `Specification::validate`, after every existing step, so a specification `validate` refused before is refused with the same refusal. The new code is a new module, `crates/aethelgard-ir/src/names.rs`; `lib.rs` gains only the variants, steps 17 and 18, the §1.8 refusal and the Windows rule.

1. **§1.1:** above.
2. **§1.2 `MissingBusConstraint`:** step 17 (`Specification::bus_constraints`), the same check `solve` made, by the same name and message. `solve`'s copy is deleted: `i2c_ceilings` now `expect`s the constraint, as `i2c_electrical` already did.
3. **§1.3, the names:** step 18 (`names::check`), in `generate`'s order (the emitter's references, its nets, then the twin's device names).
   - **A2:** a device reference that case-folds to an instance of `names::peripheral_instances()` is refused as `InvalidDeviceName`. The table it reads is static: GPIO ports A–K (the stock platform's, which bound any pin's port), `I2C1`–`I2C4` and `SPI1`–`SPI6` (the H753's), `serial::PERIPHERALS`, and `TIM<n>` for `motor::TIMERS`. Its `id` is `null` because the twin prints none.
   - **A4, the emitter's naming rule, as read** (`schematic.rs` `Builder::reference`, `pcb.rs` `items`):
     - The emitter numbers `U<n>`, `R<n>`, `C<n>` from 1, skips a reference the specification spells exactly, and refuses one that differs only in case.
     - The board writer then refuses any two references with one KiCad key (case folded, leading zeros of the trailing number dropped).
     - **`U` is exact:** the MCU is `names::mcu_reference`.
     - **`R` and `C` follow the solve-independent pattern:** the set depends on the drawing, not on the solve, so §7's second stop does not fire. The pattern is held because §1.1 found no use.
     - The `id` is the sorted pair, as both checks print it (`U1 and u1`, `R1 and r01`).
   - **A5:** a net or domain (domains, bus lines, chip selects, serial lines, motor outputs, sense nets) that case-folds to a net in `names::own_nets(spec)`, or that starts with `unconnected-`, is refused as `ReservedName`. The `id` is the name, or `<own> and <spelled>` as the emitter's `collision` prints it.
   - **The moved list:** `own_nets` is the one list both stages read. It holds `GND`, `VBAT` beside a power tree, `<mcu>_VCAP1..2`, each part's straps and filter (`device_nets`: BME280 `SDO`, W25Q64JVxIQ `WP` and `HOLD`, ICM20948 `REGOUT`), a buck's `FB`, `VCC`, `BOOT` and `SW`, and a crossing's four lines (`Translator::crossing_lines`, moved from eda's `lines`).
     - The emitter's `own_net` refuses, as `EmitterError`, any net the list lacks.
     - `power::references` (eda `schematic_power.rs`), `reference_key` (eda `pcb.rs`), `UNCONNECTED` (eda `parity.rs`) and `VENDORED_PARTS` (eda's `KNOWN_PARTS`) moved to `ir` unchanged; eda reads them.
4. **§1.4, A7:** `check_identifier` refuses a project name whose stem (before the first `.`, without trailing spaces) case-folds to `con`, `prn`, `aux`, `nul`, `com1`–`com9` or `lpt1`–`lpt9`, as `InvalidIdentifier` at `project_name`.
   - **Other identifiers that become file names:** none [read]. `generate` names `kicad/<project>.*`, `board/<project>.repl` and the crate `<project>-board`; `fab` names `<project>-*`, `<project>.drl`, `.bom.csv`, `.elf`. Campaign archives use the run id in hex; scenario names reach no path.
5. **§1.5, one error set:**
   - `SolveError::{MissingBusConstraint, BusTooFast, PullUpBelowSinkFloor, BusCapacitanceOverLimit}` are deleted. `solve` raises validation's variants inside `InvalidSpecification`.
   - The new `IrValidationError::claimants()` gives the bus, or the line, for those four, and `SolveError::claimants()` delegates to it.
   - The JSON is pinned byte for byte from the baseline binary, one test per variant (`aet_1206_one_error_set_keeps_solves_bytes`, `aet_1206_a_missing_bus_constraint_prints_solves_bytes_from_every_command`).
6. **§1.6, the three pre-empted refusals:** §4.6.
7. **§1.7 (amendment):** `i2c_ceilings` takes the max; so does `SolvedTopology::validate`'s `CeilingNotDerived` re-derivation.
   - Tests: worse SDA and worse SCL, each boundary both ways (120/121, 300/301, 1000/1001), the amendment's 1000/120 at 1 MHz (`BusTooFast`, ceiling 100 kHz), and the topology side both ways.
   - **Five tests encoded the bug and now assert the worse line:**
     - `the_stricter_line_sets_the_ceiling_whichever_it_is` (whose own comment said "if that is ruled, every assertion here inverts");
     - `a_rise_time_slower_than_every_mode_is_refused` (it now names 2000 ns, the worse line, not 1001);
     - `solved_derives_the_ceiling_from_the_smallest_bound_of_the_bus`;
     - `solved_refuses_a_rise_time_slower_than_every_mode`;
     - `solved_refuses_a_zero_analog_value`'s huge-values tail (the `u32::MAX` rise time is now refused; width stays covered by the preceding test).
8. **§1.8 (amendment 2):** step 14 refuses a device part with no row, I2C devices first, before any limit, as the emitter's `UnknownPart` (`id` the reference, the same message). The emitter's `known:` list is `names::VENDORED_PARTS`, and each vendored part has a row, so the message stays true.

## 4. Evidence

### 4.1 Verdicts and bytes (measured, `~/.cache/aet-1206/v-base`, `v-new2`)
- **The corpus:** after the change, all 24 specifications give byte-identical `validate`, `solve` and `generate` stdout and exit codes, and an identical sha256 for every generated file (157 of 157 records).
- **76 name mutations of the reference:** every one `generate` refused before (57) now gets the same bytes from `validate`, and `generate` prints exactly what it printed before. Old against new: `~/.cache/aet-1206/mut1-base.table`, `mut1-new.table`.
- **The three F1 refusals and `MissingBusConstraint`:** byte-identical from `solve`, `emit` and `generate`, measured on both binaries.

### 4.2 Tier-1 tests (each with the behaviour before)
- `reserved_references_tier1.rs`:
  - `aet_1206_validate_refuses_what_generate_refuses_byte_for_byte` holds 18 rows (A2, A4 with the KiCad-key pairs, A5, the serial short, §1.8, `MissingBusConstraint`). On each, `validate`'s stdout equals `generate`'s byte for byte.
  - The old behaviour (`validate` exit 0, `generate` refusing with this kind and `id`) was committed first, as `58c2438`. Its rows other than §1.8 and the serial short are the same; those two were measured on the baseline binary (§1, §5).
  - It also holds the F1 pins and the eda guard test.
- `ir/tests/validate.rs` gains a test per rule (A2, A4, A5, A7, `MissingBusConstraint`, §1.8), each with the names that must still pass.
- `ir/tests/solve.rs` gains the §1.7 tests.
- `power_board_tier1.rs` gains `the_nets_validate_reserves_are_the_emitters_own` (on the reference and the power tree, every listed net is drawn, and every net the emitter draws that the specification does not spell is listed) and `a_power_reference_the_emitter_refuses_is_refused_at_validate` (`#U10`, and `unconnected-x` making `unconnected-x_FB`, each with the emitter's kind, `id` and message).

### 4.3 The CLI envelope (a fork; outside the named fence)
`MissingBusConstraint` printed `claimants` from `solve`, `emit` and `generate`. To keep those bytes once `validate` refuses it, the CLI's `Refusal` gained an optional `claimants` (between `id` and `path`), set for a validation refusal that names one.
- `validate` therefore prints `solve`'s shape for it.
- The reduction pass then made `solve`'s private envelope this one (`claimants` always present on a solve refusal, as before).
- The fence names `ir`, the emitter's list, the guards and tests. `cli/src/main.rs` is touched only here, because byte-identity required it.

### 4.4 Gates
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean. The boxed `claimants` keeps the refusal under `result_large_err`.
- `cargo test --workspace --no-fail-fast` in a 32 GiB, swap-0 user scope: 110 test binaries, 0 failures [measured].
- No container was started.

### 4.5 Mutation (diff-sized: one mutant per guard this work order adds)
19 mutants, run one at a time against `ir` validate/solve, twin `spi_platform`, and cli `reserved_references_tier1`, `power_board_tier1` and `solve_cli` (`~/.cache/aet-1206/mutants/`). Three are for the adversarial fixes, run after `dee28d9`. **18 killed** by test failures (none by compilation).
- **The survivor is mutant 12** (the emitter's `own_net` guard off). It fires only when `own_nets` and the emitter disagree, which no correct tree does.
- Mutant 13 (the list drops a buck's nets) is killed by that very guard: `Internal("the emitter names net U10_FB, which aethelgard_ir::names::own_nets does not list")`. So the guard is load-bearing exactly when the two lists drift.

### 4.6 §1.6, the three pre-empted refusals
- **eda `UnsupportedAddress`: kept.** `validate` refuses the strap first (`I2cAddressNotOffered`). But `schematic::emit` is public and takes a `Specification`, which deserializes without validating. A library caller reaches the guard: `aet_1206_the_emitters_address_guard_is_reachable_by_a_library_caller` (BME280 and TMP117 at `0x50`).
- **twin `ModelOffItsBus`, SPI site: kept.** `platform::spi_wiring` is public and does not validate its topology. A hand-written topology with a BME280 as an SPI device reaches it: `an_i2c_model_on_spi_reaches_the_kept_guard_through_spi_wiring`.
- **twin `ModelOffItsBus`, I2C site (`split_devices`): deleted.** Both of its callers (`instances`, `emit_faulted`) validate the topology first, through `stock_instances`, which refuses a listed part off its row's bus (`PartOnWrongBus`). Every modelled part has a row, since the model is read from the row. Every row sits on its twin model's one bus, which the new table test `every_modelled_part_s_row_is_on_its_model_s_bus` pins. So no path reaches the check.
- **Emitter `InvalidDeviceName` for `spi1`–`spi6`: nothing to delete.** There is no code of its own: it is the twin's shared `taken` check (solved peripherals, lower-cased). `SolvedTopology::validate` refuses `spi1`–`spi6` first, as `InvalidIdentifier` (`RESERVED_REFERENCES`). The shared check stays: a hand-written topology still reaches it with `gpioa`.

## 5. Behaviour changes a reviewer should know

- **Refusals wider than `generate`'s, each licensed by the work order, none used in the corpus** [measured on the reference]:
  - A2's static table (`gpioe`, `gpioh`–`gpiok`, `tim1`, `tim2`, `TIM8`, `tim17` generated before, since the solve did not use them);
  - A4's `R<n>` and `C<n>` (`r30`, `c99` generated before);
  - A7 (a new rule; `generate` accepted every Windows name).
- **A damage path closed (measured on `main`):** a serial net named for an emitter net generated a shorted board.
  - `CONSOLE_TX` renamed `U1_VCAP1` put J1 pad 1 and U1 pad 101 on the MCU's VCAP1 net with U1 pad 71 and C1. The emitter compares its own nets only with the names it "spells" (domains, bus lines, chip selects, sense nets), not serial or motor nets (`schematic.rs` `design`).
  - A5 reads every net, so `validate`, and with it `generate`, refuses it.
  - The emitter's own list is unchanged (moved, not changed): backlog F1.
- **A doubly faulty specification** may now report a different first refusal: a new step 17/18 refusal comes before a `solve`-time one (`PartMismatch`, the search), and within step 18 the order approximates `generate`'s. Each is still refused.
- **§1.7 moves two messages:** `RiseTimeTooSlow` now names the worse line's rise time, and a mixed bus's ceiling is the slower mode.
- **`campaign` over a base specification missing a bus constraint** [read, `campaign.rs:362`, not run: it needs a firmware ELF]. The refusal now comes from `parse_spec`, with campaign's `scenario \`<name>\`, base: ` message prefix, where before it came from `solve_spec` without one. Kind, `id` and `claimants` are unchanged.

## 6. Adversarial pass (targeted)

A fresh subagent (no stake in the code, given the code and both binaries, not this document) generated 5,422 specifications over the reference and the power-tree fixture. It ran `validate` and `generate` on both binaries for each, plus the work order's targeted list: Unicode case (`İ`, `ſ`, the Kelvin sign `K`, fullwidth, NBSP, ZWSP), trailing dots, `CON.`, `con .txt`, `Gnd`/`gNd`/`Vbat`/`UNCONNECTED-x`/`Unconnected-x`, `U01`/`u001`/`R0`/`U0`, buck and translator nets, and every 1-, 2- and 3-fault combination of 11 faults. Harness and specifications: `~/.cache/aet-1206/adv/` (`drive.py`; each class's report in `rep/`, the rerun in `rep3/`; repros in `repro/`, `run.sh <spec>`).

- **Unicode, dots, Windows spellings:**
  - Every non-ASCII name is `InvalidIdentifier` on both binaries; the Kelvin sign alone is ASCII `K`.
  - `CON.` and `con .txt` are `InvalidIdentifier` on both (the crate rule refuses `.` and space before the Windows rule).
  - `GND.` passes everywhere.
  - Mixed-case `gnd`, `Gnd`, `gNd` and `Vbat` give the same refusal from all four runs. `Unconnected-x`, `UNCONNECTED-x` and `uNconnected-x` pass on every binary (KiCad's prefix match is case-sensitive) [measured].
- **Escapes found against `47de363`, all measured with both binaries:**
  - **A1.** Two of the specification's references with one KiCad key by a leading zero: `J1` and `J01`, `U1` and `U01`, `U010` beside the power tree's `U10`; over 100 such specifications. Step 8 folds case only; the board writer's pair check refused them. **Fixed** (`dee28d9`): step 18 holds every reference to KiCad's key, printing the writer's sorted pair.
  - **A2.** A power-tree reference starting `#` (KiCad's virtual-symbol prefix), on every power reference slot but `hot_plug`, which the emitter does not draw. **Fixed:** `ReservedName`, the reference.
  - **A3.** A power reference starting `unconnected-`, which makes the emitter's own nets start with it (`unconnected-x_FB`). **Fixed:** `ReservedName`, the own net.
  - **A4.** A part with a table row but no vendored model (`ICM42688P`, `ICP10111`, `BMP581`, and the repository's own `imu-baro-1v8` fixture), and an `mcu.part` other than `STM32H753ZITx`. Both are `UnknownPart` at `generate` only. **Not moved:** the work order's "It is NOT" keeps the closed vocabularies with their tables (`1102` A9), and §1.8 is worded "no row". Finding F10.
- **The rerun after the fix** (`dee28d9`, all 5,422) [measured]:
  - **Escapes:** 9 left, all `UnknownPart` (A4).
  - **False refusals:** 101, every one a licensed widening:
    - `R7`/`c99` (6);
    - A2's table (51);
    - Windows names (13);
    - serial nets named for an emitter net, in any case (31).
  - **New `validate` against new `generate`:** byte-identical wherever both refuse.
  - **New `validate` against baseline `generate`, same kind:** 25 differ in bytes. 23 are multi-fault specifications that name a different fault first. Two are one power reference named `unconnected-x`:
    - on a buck whose sense net also takes one of its own nets' names, which is two faults (F12);
    - on the translator, where the board writer names the first offending pad in its symbol's pin order (`_B4`) and `validate` names the first in `crossing_lines`' order (`_B1Y`). Kind and message form agree; residual F11.
- **First refusals of multi-fault specifications:** of the 231 combinations, 84 report a different first kind than baseline `generate` did. Most are a Windows project name, a §1.8 part or a step 18 name that `validate` now meets first; new `validate` and new `generate` always agree. A measurement, not a defect (§5).
- **A hand-written topology reaching a deleted refusal's path:** the deleted twin I2C site is reached only through `instances` and `emit_faulted`. Both validate the topology first; `platform::emit` with a BME280 declared on SPI, or a W25Q64JVxIQ on I2C, is `PartOnWrongBus` (`spi_platform.rs`, `a_model_on_the_wrong_kind_of_bus_is_refused`). The table test pins every modelled row on its model's bus [measured].

## 7. Reduction pass

The four questions over the diff:
- **(1) A fact stated twice:**
  - The emitter's naming facts now have one home (`names.rs`). eda reads `reference_key`, `UNCONNECTED`, `VENDORED_PARTS`, `power::references` and `crossing_lines` from `ir`, and its `own_net` is held to `own_nets`. These moves are the work order's.
  - The variant messages `validate` shares with the emitter and the twin (`ReservedName`, `DuplicateReference`, `InvalidDeviceName`, `UnknownPart`) stay as two literals per message, across crates that cannot share a `#[error]` format. Each pair is pinned equal by the byte-identity test: finding F5.
  - `power.rs`'s inline reference list is **not** a copy of `power::references`. It also holds `hot_plug` references, which the emitter never draws (no `hot_plug` in eda).
- **(2) Unused:** the twin's I2C `ModelOffItsBus` site (§4.6) was deleted.
- **(3) Two mechanisms that could be one:** `solve_spec`'s private `SolveRefusal` and `SolveRefusalBody` became `Refusal` with `claimants` (`1bfe45f`, −19 lines). Proven by the differential: every corpus record and each F1 pin is byte-identical.
- **(4) A workaround for a gone problem:** `solve`'s `MissingBusConstraint` branch (now an `expect`, as `i2c_electrical`'s).
- **Kept, reported:** `solve`'s early bus checks still duplicate `validate_i2c_speeds` (`1205` F1's second half). Deleting them would move every bus refusal after the pin search, changing first refusals and time: F6.

Reduction pass (diff): −19 lines net (the envelope merge; the guard deletion is 0 net, its 6 lines replaced by the 6-line reason) · 2 duplicates removed (the solve envelope; `SolveError`'s four variants) · 2 dead paths removed (twin I2C `ModelOffItsBus`, `solve`'s `MissingBusConstraint` branch) · 0 protected kept · 2 findings below (F5, F6)

## 8. Findings and questions

| # | Finding | Where | Evidence | Disposition | Decision needed |
|---|---|---|---|---|---|
| F1 | The emitter compares its own nets with domains, bus lines, chip selects and sense nets, not serial or motor nets, so on `main` a serial net named `U1_VCAP1` generated a board shorting VCAP1 to a UART pin. `validate` now refuses it (A5 reads every net), so `generate` cannot reach it. A library caller of `schematic::emit` with an unvalidated specification still can | `eda/src/schematic.rs` `design` (`spelled`) | measured (§5) | backlog: add serial and motor nets to the emitter's `spelled` (outside this fence: "moved, not changed") | no |
| F2 | Two specification references with one KiCad key by a leading zero (`U2` and `U02`) passed `validate`, and `generate` refused them (`DuplicateReference`, `U02 and U2`, the board writer's pair check). The adversarial pass found the same (§6 A1) | `names.rs` step 18 | measured | **fixed** in step 18 with the writer's bytes, as part of A4's family (a fork: the work order names the emitter's references; this is the specification's against each other, the same check and kind) | confirm the fork |
| F3 | `MissingSpiConstraint`, `RiseTimeTooSlow` and `SupplyOutsideSpeedTable` are still `solve`'s alone. Each is static in the specification as `MissingBusConstraint` was, so `validate` passes a specification `solve` refuses for them. Not listed in the work order, so not moved | `solve.rs` `spi_lines`, `i2c_ceilings` | read | backlog row, the same pattern as §1.2 | yes |
| F4 | A4's `R<n>`/`C<n>` refusal, where the emitter would not reach that number (`r30` on the reference), prints the emitter's message, "`R30 and r30` is used twice as a reference", about a part not yet drawn. A distinct message would break byte-identity where the emitter does reach it | `names.rs` `emitter_numbered` | measured | kept for byte-identity | no |
| F5 | Four messages live as one literal in `ir` and one in eda or the twin; the byte-identity test pins each pair equal | `lib.rs`, `eda/schematic.rs`, `twin/lib.rs` | read | the test is the single source; a shared type would be a cross-crate error redesign | no |
| F6 | `solve`'s `i2c_ceilings` and `i2c_electrical` restate `validate_i2c_speeds`'s rules (`1205` F1's second half) | `solve.rs` | read | kept: they order the bus refusals before the search | no |
| F7 | A7 refuses the work order's list. Microsoft's naming page also lists `COM0`, `LPT0` and the superscript forms; the superscripts are non-ASCII and already refused, and `com0`/`lpt0` pass | `names.rs` `WINDOWS_RESERVED` | read | as listed in the work order | yes (trivial) |
| F8 | §6's split-on-touch rule names `ir/src/lib.rs` and `ir/src/power.rs`, both touched here. **Declined:** `4300g` (#159) and `4230` (#161) both have open hunks in `lib.rs`, so a split now turns both of their merges into rewrites. This work order's new code went to `names.rs`, adding 98 lines net to `lib.rs` (variants, steps 17 and 18, §1.8, the Windows rule) | roadmap §6 | read | declined with that reason | no |
| F9 | `IrValidationError::InvalidDeviceName` has `id: null` at `validate`, because the twin prints none and byte-identity rules. Every other new refusal names its reference | `names.rs` A2 | measured | kept for byte-identity | no |
| F10 | `validate` passes a part with a table row but no vendored model (`ICM42688P`, `ICP10111`, `BMP581`; the fixture `imu-baro-1v8.json`) and an `mcu.part` other than `STM32H753ZITx`; `generate` refuses both as `UnknownPart` | eda `KNOWN_PARTS` (`names::VENDORED_PARTS`), svd-hal pin table | measured (§6 A4) | not moved: the work order's "It is NOT" (`1102` A9). Moving the first would refuse two committed fixtures at `validate` (`imu-baro-1v8`, `sentinel-02-own`), which six test files use past `validate` (`board_tier2`, `models_tier2`, `twin_tier2`, `own_models_tier1`, `speeds_tier1`, `fab_tier1`) | yes, if it should move |
| F11 | A crossing (translator) reference starting `unconnected-`: `validate` names `<ref>_B1Y`, while the board writer named `<ref>_B4`, the first offending pad in the KiCad symbol's pin order, which `ir` does not hold. Kind and message form agree, and `generate` now prints `validate`'s refusal | `names.rs` A3 fix | measured | residual: exotic, one fault | no |
| F12 | A multi-fault specification can report a different first refusal than `generate` did: 84 of 231 combinations measured, and 23 byte differences of the same kind. New `validate` and new `generate` always agree | step 17/18 order | measured | a measurement (§5) | no |

**Peers told (inbox, 09:45 UTC):**
- `4300g`: `own_nets` must list its support network's nets when the second of us merges.
- `4230`: the deleted `SolveError` variants sit beside its `id()` and `claimants()` hunks.

## 9. Where everything ran

- Host cargo, in a capped user scope (`systemd-run --user --scope -p MemoryMax=32G -p MemorySwapMax=0`, `memory.swap.max` read back 0).
- The baseline and differential runs used the debug binaries under `~/.cache/aet-1206/`.
- No container, Renode or KiCad was started; `TIER2_MIN` stays 155 (no tier-2 test added).
