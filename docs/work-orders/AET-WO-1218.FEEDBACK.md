# AET-WO-1218 — FEEDBACK: test points beside pin devices, the analog-pad check on the MCU actually solved, the power tree's reservoir against VDDA's maximum, R4 and R5

**Branch:** `wo/aet-1218-ir-defects`, from `main` `2ccd2450`; merged in: `c16da411` (docs only). **Builder:** Claude (Opus 5.5), single-threaded: the diff is small and touches gate logic. Written 2026-11-22.

**Status: READY.** The package is committed from dispatch run 45045208580, byte for byte (§9).

**The answer, in five lines:**
- **(1)** A test point beside a pin device validates, solves and emits. Its references are stable: two emits are byte-identical, and they are the bare board's references plus the test point's. Step 15a judges each test point against every other reference by construction (`names::references_but_test_points`), not by slicing the end of `names::references`, which lists the pin devices last.
- **(2)** `check pinout` refuses a topology whose `mcu` is not its pin table's part, exactly (`PartMismatch`, exit 2). A pad claim cannot escape by renaming the MCU, nor by changing its case.
- **(3)** The battery sense's reservoir is rated by `pin_devices::reservoir_rated`, VDDA's maximum: one rule, one home.
- **R4** `NoInstance` and `NoSerialPeripheral` name each pin withheld as an analog pad. **R5** a declared reservoir is a capacitor row whether or not RAIN needs it, both in pin devices and in the power tree's sense (accepted at 16:10Z).
- **Specification 27** is a rules-only change row (`PowerPartUnknown`). The null sweep covers 34 specifications: no exit, output or emitted byte moved, and 232 generated files moved by their sha line alone. Mutants: 11 of 11 killed. No committed board carries both a pin device and a test point, so no committed reference moved.

## 0. How it was made

- **Commits:**
  - predictions, before any run: `9156a5e8`;
  - the build: `3eece126`;
  - the measurement, results and mutants: `f669d92e`;
  - specification 27 and its goldens: `528f4454`;
  - main merged: `a0bee897`.
- **Kit** (`spike/1218/`):
  - `measure.sh` runs the baseline sweep, then every probe: validate, solve, emit twice, the emits compared, the references read;
  - `probes.py` holds the scratch boards, the R5 probes and the adversarial probes;
  - `refs.py` reads a board's references from its `.kicad_pcb` footprints;
  - `mutants.py` and `order_back.rs` (`order_2203`'s put-back);
  - outputs are in `out/`, and `RESULTS.md` sets each result beside its prediction.

## 1. The measurement (§1.1)

- **The baseline:** 34 committed specifications, 280 emitted files.
  - 2 carry pin devices (D1 and D2, both `board.drawn: false`), 4 carry test points, and none carry both.
  - Two runs of main's binary were identical.
- **The scratch board, today (main's binary):** D2, drawn, with its input PWM limit declared as `pin_devices_solve.rs` declares it, plus `TP1` on `GND`.
  - It is refused at validate, solve and emit with `TestPointReferenceTaken`: "test point `TP1` takes a reference that `TP1` already holds".
  - D1, a test point on `DRV_NFAULT` or `BOOT0`, and two test points give the same refusal.
- **After:** each of those validates, solves and emits, and two emits are byte-identical (`out/probes-after.tsv`).
  - The references are the bare board's plus `TP1` (`out/refs-after.tsv`).
  - On `BOOT0` there is also the bench's pull-down `R1`, drawn after every other part, so no other reference moves.
- **The §7 stop did not fire.** No committed board's reference moved: the null sweep found 0 emitted files moved, before and after the bump.

## 2. What was built

- **(1) `names.rs`, `bench.rs`.**
  - `names::references` and `names::references_but_test_points` are one chain (`declared(spec, test_points)`), so their order is one fact.
  - Step 15a builds its key map from the second.
  - `names::check`, `mcu_reference`, `switch_reference` and eda's claim chain read the list as a set, so the order moves none of them.
  - The collisions stay `TestPointReferenceTaken` at step 15a. A test point `drv01` beside device `DRV1` now names `DRV1`; on main it named itself.
- **(2) `aethelgard-pincheck`.**
  - `Refusal::PartMismatch { topology, table }` is the first check in `judge` after the table's own part.
  - In the CLI it is mapped to exit 2, with one match arm and the README's exit row. Both are outside the named fence, and both were accepted at 16:10Z.
  - `pinout.rs`' module comment no longer says the topology names no part.
- **(3) `power.rs` `sense()`** rates the reservoir with `!pin_devices::reservoir_rated(rated, vdda)`, now `pub(crate)`. The kind stays `ExternalPartOutOfRange`, and the message names VDDA's maximum. `vref_min`, the full-scale check's, is unchanged.
- **R4 `solve.rs`.**
  - `Model::withheld` holds the analog pads carrying each (instance, signal).
  - `withheld_note` appends "; `PC2` withheld as an analog pad (the MCU row's `io.analog_pads`)" to the "held by hand" and "no unreserved pin" reasons, for buses (step 5) and serial ports (step 5b).
  - Nothing is appended when no pad carries the signal, so every existing message is unchanged.
- **R5 `pin_devices.rs` `analog()` and `power.rs` `sense()`.** The reservoir's row is looked up, and refused unless it is a capacitor row, before the RAIN branch.
  - On the power tree, `NOPE` was already refused by the tree's part list. A resistor row was not, and `emit` then refused it (`PowerNotDrawable`).
- **Specification 27.**
  - `versions::CHANGES` row 27 (`AET-WO-1218`, rules `[PowerPartUnknown]`), the README's version paragraph, `lib.rs`' version comment.
  - All 34 committed specifications moved by their version line alone (checked line by line against `2ccd2450`).
  - (3) is not in the row: no held capacitor is rated between a legal VDDA's minimum and maximum (the lowest is 10 V), so it cannot refuse a file version 26 admitted. The README says so.
- **The tests**, each failing without its change (§4):
  - `ir/tests/bench.rs`: a test point beside a pin device validates on four nets, and a test point named for the device, exactly or by key, names the device;
  - `cli/tests/emit_tier1.rs`: validate, solve, two byte-identical emits, references bare plus `TP1`;
  - `pincheck/tests/check.rs` and `cli/tests/pinout_tier1.rs`: another part, and the part in lower case; `PC2` claimed, `AnalogPadPin` on the H753, `PartMismatch` otherwise;
  - `ir/tests/power.rs`: the band test, refused at 3135 and 3464 mV and admitted at 3465 mV, through a test row; and R5 on the sense;
  - `ir/tests/pin_devices.rs`: R5;
  - `ir/tests/solve.rs` and `ir/tests/serial.rs`: R4, on the synthetic tables relabelled as the H753. No UART signal is on a pad of the real table.

## 3. Rulings applied

- 16:10Z: R5 at both sites accepted under its row. `PartMismatch`'s CLI arm and README row accepted outside the named fence.

## 4. Mutants (sized to the diff)

11 of 11 killed (`spike/1218/out/mutants.json`). Each mutant ran against the suites that hold it, and the unmutated tree passes them all.

- (1): the positional slice restored; the test points left in "every other reference"; the pin devices left out of it.
- (2): the part check off.
- (3): the minimum restored at the call site. `reservoir_rated`'s own unit tests cannot see this, so the mutant is at the call.
- R4: each of the three notes dropped, and the withheld pads not collected.
- R5: each site's lookup again gated on RAIN.

## 5. Adversarial pass (§5, targeted)

- **A reference that two readers read differently.** None found.
  - The bench, step 20 and eda claim one set. The reset switch numbers `SW1` with and without a test point on `NRST`.
  - Test points on a bridge output (`MA_OUT1`) and on `VMOTOR` emit beside `U2`.
  - `generate` on D1 with `TP1` moves its KiCad files by the test point alone, and every other generated file by its sha line only. D2 does not generate, bare or not (`BridgeAcrossParts`, the generator's own refusal).
  - `check pinout` reads no reference.
- **A renamed MCU that still escapes a check.** None found.
  - Only `check pinout` and `generate hwdef` read a topology file. `generate hwdef` refuses `UnknownMcu` for `STM32H743ZITx` and `stm32h753zitx`.
  - The H753 is the only MCU row, so a rename never lands on another row's pads.
  - Every other topology is solved in-process from a specification, where `solve` refuses a table mismatch.
- **Order (my prediction P4.1 was wrong):** `PartMismatch` is raised in `judge`, so it comes after the CLI reads the ELF, and a malformed ELF is reported first. Both are refusals.

## 6. Predictions beside results

`spike/1218/RESULTS.md`. Every prediction held, except two:
- P2.3's collisions (`U2`, `R2`) are caught by `emitter_prefix`, not by the slice. Non-numbered collisions (`DRV1`, `RSENSE`) were added.
- P4.1's order (§5).

## 7. Findings, not fixed here (§7.1 rows proposed)

- **F1 · One rule, two homes:** "a declared reservoir is a capacitor row".
  - It is in `pin_devices::analog` and `power::sense`, with two error builders (`PassivePartUnknown`→`PowerPartUnknown` at a path, and `ctx.kind_err` at a reference).
  - Merging them is a refactor across the two refusal types.
  - **Proposed row:** "the reservoir's row lookup is one helper both judges call".
- **F2 · eda's reference claim chain is a hand copy of `names::references`' order** (`schematic.rs`, `design`).
  - Today it claims the same set, so nothing differs. A new kind added to one list and not the other would.
  - **Proposed row:** "eda claims `names::references` (made public)". This is outside the fence (eda).
- **F3 · An unneeded reservoir is checked only for its kind, not its rating.** A capacitor rated under VDDA's maximum, fitted where RAIN does not need it, still validates. R5's row asked for the kind only.
  - **Proposed row:** "a declared reservoir is rated for VDDA's maximum whether or not RAIN needs it".
- **F4 · `check pinout`'s refusal order.** The topology's part is judged after the ELF is read (§5). A CLI reorder would put every topology refusal first.

## 8. Gates, fence, reduction pass

- **Gates (Furnace, target in the repository), on the READY tree (the package of run 45045208580 committed; main `c16da411` merged):** `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass. `cargo test --workspace`, one whole run, gives 160 binaries, 2784 passed, 0 failed.
  - At `a0bee897`, before the package, the count was 2782 passed and 2 failed. One failure was the package test. The other was `readme_versions`, on the IR README's step-1 line, which the next commit fixed.
  - Before the bump, at `3eece126`: fmt and clippy pass, and `cargo test --workspace` gives 160 binaries, 2784 passed, 0 failed.
- **The null sweep** (`out/null-sweep.json`): main's binary on main's tree against the branch's on the branch's.
  - 34 specifications. No exit code, `validate` output, `solve` output or emitted file moved; 232 generated files each moved by one sha line.
  - Before the bump (`out/null-sweep-prebump.json`), nothing moved at all.
- **Goldens moved by the bump, by their generators, each by its sha line alone** (masked diffs pair line for line):
  - `boards/sentinel-01*` and the svd-hal UI goldens (`AETHELGARD_BLESS=1`);
  - the hwdef fixtures (`solve`, then `generate hwdef`);
  - `solve_cli`'s two digests (each with the specification-24 sha put back still equals its `_24` digest);
  - `order_2203`'s three texts (with the specification-26 sha put back, each equals main's `BEFORE`; the boot image did not move);
  - the CLI README's example line.
- **Fence:**
  - `crates/aethelgard-ir` (names, bench, power, pin devices, solve, versions, README, tests) and `crates/aethelgard-pincheck`;
  - the CLI's `PartMismatch` arm and README row (accepted);
  - the test-only additions in `aethelgard-cli/tests/emit_tier1.rs` and `pinout_tier1.rs`, for acceptance;
  - the version literals and goldens the bump moves; `spike/1218/`; this file.
  - Nothing in the generator, the twin, chaos, eda, `verify` or workflows changed.
- **Reduction pass (diff): −0 net · 0 duplicates removed · 0 dead paths removed · 0 protected kept · 3 findings below.**
  - The work order's own change (3) removed one duplicated rule: the power tree's own copy of the reservoir rating.
  - Findings:
    - F1 above;
    - F2 above;
    - `d2_declared` (D2 with its PWM limit declared) now exists in four test files across two crates' test binaries (`ir/tests/pin_devices.rs`, `ir/tests/bench.rs`, `cli/tests/pin_devices_solve.rs`, `cli/tests/emit_tier1.rs`). No helper is shared between those binaries, and a shared module would warn dead code in the binaries that do not call it. It is left as is.

## 9. The package

`fab/sentinel-01-bench/package` is the `bench-package` artifact of the architect's write=true dispatch, **run 45045208580** on `4481e7c3`, committed byte for byte.
- The artifact holds the same 17 files.
- Only `FAB-NOTES.md` and `MANIFEST.json` moved, each by sha lines alone (masked, the lines pair; 0 other lines), as predicted.
- `bench_board_tier1` passes with it.
