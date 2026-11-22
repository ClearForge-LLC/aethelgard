# AET-WO-1217 — FEEDBACK: pin devices' IR backlog (a GND tie, the analog-switch pads, the reservoir's rating, step 24's order, an unknown MCU)

**Branch:** `wo/aet-1217-pin-device-ir-backlog`, from `main` `22a1b3c0`; merged in: `857525ad` (docs) and `a41a8a8d` (2308). **Builder:** Claude (Opus 5.5 orchestrating, with one Opus adversarial reviewer in its own worktree, given the code and §5, not my conclusions). Each finding was checked at source before use. Written 2026-11-22.

**Status: AWAITING-PACKAGE.** The bump moves `fab/sentinel-01-bench/package`'s sha lines, so `bench_board_tier1` fails until the write=true dispatch is committed (§9).

**The answer, in five lines:**
- **F2:** `straps.IPROPI: {"tie": "GND"}` is admitted only on the pin the row's `trip.sense` names, and it turns the trip off. VREF on `GND`, or on a domain whose minimum is 0 mV, is still refused. A tie plus an ADC input on the pin is `DevicePinBoundTwice`.
- **F4:** `PC2_C`/`PC3_C` are the MCU row's `io.analog_pads` (power-parts 8). No digital claimant is offered one. A digital hand pin there is `AnalogPadPin`, in `solve` and in `SolvedTopology::validate`. ADC inputs keep the pads.
- **F8, F9, F10:**
  - the reservoir is rated against VDDA's maximum;
  - the trip is judged before the ADC rule;
  - an unknown MCU is refused once, at `mcu.part`, at the head of step 24.
- **Specification 26**, a rules-only change row (`AnalogPadPin`, `PowerPartUnknown`). The null sweep covers 34 specifications: no exit code, output or emitted byte moved, and 232 generated files moved by their sha line alone.
- **Mutants: 18 of 18 killed**, one per guard. No committed board's pin moved: this was measured first, and a prototype that withheld both pads from every offer gave 280/280 files identical.

## 0. How it was made

- **Commits:**
  - predictions, committed before any run: `f70d6972`;
  - the measurement and the stop: `37d1347f` (`spike/1217/RESULTS.md`);
  - ruled by the architect at 12:25Z, Q1–Q4 all to the leans;
  - the build: `b6d24dd2` (F2, F8, F9, F10), `a97d8f5a` (F4), `beb2da1b` (specification 26 and its goldens);
  - `c2bc0943`, mutants and null sweep; `ade9c22f`, the reduction;
  - then the adversarial pass's fixes (§5).
- **The kit, in `spike/1217/`:**
  - `measure.sh` runs the baseline sweep, the PC2/PC3 scan (`pc23.py`), the probes (`probes.py`) and F10's scope;
  - `pads.py` writes `io.analog_pads` from Table 8's type column;
  - `order_back.rs` is `order_2203`'s put-back harness;
  - `mutants.py` runs the mutants;
  - `proto-f4-withheld.patch` is the bounding prototype;
  - every output is in `out/`.

## 1. The measurement and the stop (§1.1, §7)

Full record: `spike/1217/RESULTS.md`.
- **Baseline:** 34 committed specifications, 280 files, the count `1216b` recorded.
- **F4:** no committed specification places any signal on PC2 or PC3. The prototype withheld both from every offer, analog included, and left all 280 files identical. **No pinout stop.**
- **F9 confirmed:** D1 with VREF on `GND` and R2 at 2 kΩ reports `AdcFullScaleExceeded`, hiding `TripReferenceGrounded`.
- **F10:** today's refusal is `PowerPartUnknown` at `pin_devices[0].mcu`. With an unknown part, 23 of 34 committed specifications still validate.
- **Predictions:** 5 of 6 held. Prediction 3 failed on its probe: no-reservoir validates, since R2 1.07 kΩ is under RAIN. F9 was confirmed on another probe.
- **The stop asked four questions, all ruled to the leans:**
  - Q1: F2's shape;
  - Q2: F4's data and rule;
  - Q3: a test-only fence extension for `emit_tier1.rs`;
  - Q4: F10 narrow.
- **Correction to RESULTS §3:** "the 23 specifications that validate with an unknown part still do; only D1-like probes move" was wrong. Any board with a pin device and an unknown `mcu.part` is now refused, including one whose devices read nothing of the row: a D2 with its limit declared validated at 25 (adversarial A3). That is the ruled narrow scope, "only with a pin device", but my sentence understated it. It is now recorded in `CHANGES` row 26 and the README (§5).

## 2. What was built

- **F2** (`pin_devices.rs`, `straps()` and `trip()`):
  - `(CurrentOutput, Tie(GND))` is admitted only where `row.trip.sense` names the pin. That is the same arm shape as `sense_return`'s precedent.
  - `trip()` refuses VREF on `GND` first. With the sense tied, it then refuses:
    - VREF on a domain whose minimum is 0 mV (A1);
    - an ADC entry on the tied pin (`DevicePinBoundTwice`, before any ADC rule reads it).
  - Otherwise it skips `TripSenseUnbound` and `TripBelowFullScale`.
  - Any other strap on the sense keeps `SignalKindMismatch`, with a message naming the one strap it takes (A6).
  - No new key. eda draws the tie by its generic `Strap::Tie` arm: U2's IPROPI pad is on GND, R2 and the IPROPI net are gone, and `generate` drops only the ADC input (adversarial "held").
- **F4:**
  - The data is `data/power-parts.json`, schema 8: `kind.mcu.io.analog_pads: {pins: ["PC2", "PC3"], at}`. It is written by `spike/1217/pads.py` from `spike/1211e/out/table8-text.tsv`'s type column (rows typed `ANA`), and cites DS12117 Rev 10:
    - Table 8, p. 67, type `ANA`, and note 4 (the alternate functions reach Pxy_C only through the analog switch);
    - Table 11, p. 104 (a Px_C pin sinks at most 1 mA).

    Both pages were read again at source: page markers `67/357` and `104/357` in the held text. The parser refuses a pad that is not a held `io` pin, or a pad named twice.
  - In `solve` (`analog_pads`, `Model::new`'s `hand` with a `digital` flag), no pad is offered to:
    - bus and serial candidates;
    - timer channels;
    - chip selects;
    - a pin device's free GPIO.

    A digital hand pin on a pad is `AnalogPadPin {pin, claimant}`, after `ReservedPin` and before `PinClaimedTwice`. A pin device's ADC input may take one, by hand or unpinned.
  - `SolvedTopology::validate` refuses a digital assignment or GPIO on a pad: `IrValidationError::AnalogPadPin`, rule 7a.
  - **An MCU with no power-parts row has no pads.** Only a test's own pin table (part `"T"`) reaches that case: svd-hal holds no table for a part without a row. This is labelled at `analog_pads`. See A2 in §5.
  - **Q3, the CLI test:** `emit_tier1::a_solved_pin_is_drawn_by_its_package_position` now witnesses draw-by-position with D1's IPROPI ADC input on `PC2`: label `DRV_IPROPI`, pad 28 on `/DRV_IPROPI`. The MISO-on-PC2 case is `a_digital_signal_on_an_analog_pad_is_refused` (`AnalogPadPin`, `spi_buses[0].miso`, nothing written).
- **F8:** `reservoir_rated(rated_mv, vdda)`, rated at least `maximum_millivolts` (rounded up). There is an in-crate `#[cfg(test)]` test at the edge: 3464 refused and 3465 passed at 3.3 V ± 5 %; 3466 refused and 3467 passed at 3301 mV. No held capacitor is rated under 10 V, so no specification reaches the edge.
- **F9:** `trip()` runs before `analog()`. With the trip met on its own rail, the ADC rule still names its own fault (`a_trip_is_named_before_the_adc_rule`). With the sense unterminated, each ADC entry on another pin is judged by its own kind first (A4).
- **F10:** at step 24's head, only when `pin_devices` is non-empty, the MCU row is looked up once: `PowerPartUnknown` at `mcu.part`, with id = the part. It is passed to every device. `Device::mcu_row` and its `pin_devices[0].mcu` path are gone.
- **Specification 26:**
  - by `spike/4240a/kit/renumber.py . 25 26 17 17` (34 specifications);
  - `versions::CHANGES` row 26 `rules: ["AnalogPadPin", "PowerPartUnknown"]`;
  - the README's version paragraph, step 24, the solver table row 3 and row 10, topology rule 7a, and "Analog pads".

## 3. Rulings applied

- **Q1 (a):** no new key; specification 26, solved 17. eda's stale guard text is row R1 below.
- **Q2:** power-parts 8 from `table8.py`'s type column; `AnalogPadPin` in `solve` and `SolvedTopology::validate`; ADC inputs stay offered.
- **Q3:** the test-only fence extension, as built.
- **Q4:** narrow.

## 4. Mutants (sized to the diff)

`spike/1217/mutants.py` makes 18 mutants, one per guard this work order adds or changes. Each runs against `cargo test -p aethelgard-ir --lib --test pin_devices --test pin_devices_solve`, and the file is restored from its saved bytes afterwards. **18 of 18 are killed, each by a failing test and none by a build break** (`out/mutants-result.json`).
- **F2 (5):**
  - the tie on any current output;
  - the tie to any rail;
  - the trip judged despite the tie;
  - a tied and read sense not refused (killed only once the test also gave the read entry a full scale the ADC rule refuses);
  - the tie skipping the grounded reference.
- **A1, A4 (2):** the 0 mV reference; a misplaced ADC entry reported as no entry.
- **F8, F9, F10 (3):** the reservoir at VDDA's minimum; the ADC rule before the trip; the MCU row looked up with no pin device.
- **F4 (8):**
  - no digital hand pin refused;
  - an ADC hand pin refused;
  - bus and serial candidates on a pad;
  - timer channels on a pad;
  - a chip select on a pad;
  - a device GPIO on a pad;
  - the topology check;
  - a pad not held, or named twice.

## 5. Adversarial pass (§5)

The reviewer worked in `~/.cache/aet-1217/adv-src` at `beb2da1b`. Its report and probes are `~/.cache/aet-1217/adv/adversarial.md` and `adv/probes/`, and a copy of the report is at `spike/1217/out/adversarial.md`.
- **Attack 3, committed specifications:** all 34 files × {validate, solve} are identical in kind and path between main and the branch. 29 solved topologies agree pin for pin.
- **Fixed here:**
  - **A1 (defect, F2):** with the tie, VREF on a domain whose minimum is 0 mV (1 mV ± 500 ‰) was admitted. It is now `TripReferenceGrounded`. 2 mV ± 500 ‰ (minimum 1 mV) is above GND and validates.
  - **A3 (gap, F10):** the refusal of content specification 25 admitted is now recorded: `PowerPartUnknown` is in row 26, in the README's version paragraph, and in the `lib.rs` version note. Row 12 is the precedent for listing a kind that already existed.
  - **A4 (gap, F9):** a misspelt or wrong-kind `analog[].device_pin` was reported as `TripSenseUnbound`, with a false message. It is now refused by its own kind, which is main's (`UnknownDevicePin`, `SignalKindMismatch`); all three of its probes match main exactly.
  - **A6 (note):** IPROPI tied to a rail or through a resistor keeps `SignalKindMismatch`, and its message now says IPROPI takes only a tie to `GND`.
- **Held, by its own probes:**
  - VREF on GND with the tie;
  - the tie plus an analog input;
  - straps in any key order; a duplicate key;
  - a GND tie on every other pin;
  - two devices with one tied;
  - `emit` and `generate` on a tied D1;
  - every digital hand pin kind on PC2/PC3;
  - ADC hand pins on the pads;
  - case games on pin names;
  - shared nets (`DuplicateNetName`);
  - a hand-edited topology with the true `mcu`, refused by `generate hwdef` and `check pinout`;
  - F10 with two devices;
  - no panic in 296 runs.
- **Refusal kinds that change for inputs that already refused** (the full table is in the report). Each is the ruled intent:
  - F10's path moves to `mcu.part`, ahead of any device rule;
  - F9: the trip is named first;
  - F2's kinds, with the tie now admitted;
  - F4: a hand pin on a pad is `AnalogPadPin`, where main gave `NoInstance`, `NoSerialPeripheral`, `NoTimerChannel` or `PinClaimedTwice`, or solved onto the pad.
- **Not fixed here (rows R2–R6 below):**
  - A2: a topology whose `mcu` names no row escapes the pad check;
  - A5: the power tree's sense reservoir;
  - A7: the `NoInstance` wording;
  - A8: an unneeded reservoir never looked up;
  - A9: test points beside a pin device.

## 6. Predictions beside results

The table is `spike/1217/RESULTS.md` §2. The predictions file says "solved 16"; main is at solved 17.

## 7. Findings, not fixed here (§7.1 rows proposed)

- **R1 · eda's IPROPI guard text is stale (ruled Q1: a row, outside the fence).** `aethelgard-eda/src/schematic_arms.rs:1354–1359` says "a pin device cannot declare that tie". With the embedded table, `validate` refuses an unbound trip sense first, so the line is unreachable. A copied row with a second `current_output` that no trip names would reach it.
  - **Proposed row:** "eda's `CurrentOutput` arm message names the tie to `GND` that `validate` now admits (`1217` F2)."
- **R2 · A2: a hand-edited topology escapes `AnalogPadPin` by renaming `mcu`.** `SolvedTopology::validate` takes the pads from `topology.mcu`, which names no row after the edit. `check pinout` (`aethelgard-pincheck`, the CLI's `pinout.rs`) never reads `mcu`, and judges against the STM32H753 package either way.
  - Probe `a2-topo-assign-PC2-mcu743.json`: the run passes topology validation and stops only at the ELF. `generate hwdef` stops it by its own `UnknownMcu`.
  - The end-to-end case, with a real ELF, was not run.
  - **Proposed row:** "`check pinout` refuses a topology whose `mcu` is not its pin table's part (as `solve`'s `PartMismatch`), so a topology's analog pads cannot be dropped by renaming it." The owner is pincheck's, outside this fence.
- **R3 · A5: the battery sense's reservoir is still rated against VDDA's minimum.** That is `power.rs` `sense()`, about line 2196: the same rule as F8, in a second file. It is outside F8's named scope and unreachable with the held rows.
  - **Proposed row:** "the power tree's sense reservoir judged by `pin_devices::reservoir_rated` (one rule, one home)."
- **R4 · A7: the `NoInstance` wording.** "every pin for `SPI2_MISO` is held by hand" lists the pins `solve` offers, and leaves out the pad it no longer offers. It is accurate about the offer and silent about the pad.
  - **Proposed row:** "`NoInstance`/`NoSerialPeripheral` name a pin withheld as an analog pad."
- **R5 · A8, from before this work order: an unneeded reservoir is never looked up.** When R is under RAIN, `analog()` never reads the reservoir's part. A part `NOPE`, or a resistor row, validates, and `emit` then refuses it (`PowerNotDrawable`). That is an `INV-06` gap, identical at main.
  - **Proposed row:** "a declared reservoir is a capacitor row whether or not RAIN needs it."
- **R6 · A9, a defect from before this work order (since `1211a`): no test point validates on a board with a pin device.**
  - **The cause:** `bench.rs` step 15a assumes `names::references` lists the test points last, but `names.rs:125` chains the pin devices' references after them. Every test point then collides with itself: `TestPointReferenceTaken`, "test point `TP1` takes a reference that `TP1` already holds".
  - **The evidence:** probe `a2-tp-TP1.json`, identical on main's binary (re-run here). No committed board has both.
  - **Proposed row:** "step 15a's reference split does not depend on `names::references`' order (test points beside pin devices)." It is outside this fence (bench, names), and it is a one-line order fix.

## 8. Gates, fence, reduction pass

- **Gates (Furnace, target in the repository):**
  - `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
  - `cargo test --workspace`: 160 binaries, 2772 passed and 1 failed (at `cf22dbf5`, main `a41a8a8d` (2308) merged). The failure is `bench_board_tier1::the_committed_package_is_bound_to_its_specification_scenarios_and_emit`, which waits on the package (§9).
  - With the target in the repository, `1211e` F11's `fab_tier1` failure does not appear.
- **The null sweep** (`spike/1211a/measure/run.sh` and `spike/1211e/compare.py`, `out/null-sweep.json`): main's binary on main's tree against the branch's on the branch's. 34 specifications; 0 exit codes and 0 `validate`/`solve` outputs moved once each sha is masked; every emitted file byte-identical; 232 generated files, each moved by one sha line.
- **Goldens moved by the bump, by their generators, each by its sha line alone** (masked diffs pair line for line):
  - `boards/sentinel-01*` and the svd-hal UI goldens (`AETHELGARD_BLESS=1`);
  - the hwdef fixtures (`solve`, then `generate hwdef`);
  - `solve_cli`'s two digests: with the specification-25 sha put back, each equals main's;
  - `order_2203`'s three texts: with the old sha put back, each equals main's `BEFORE`. The boot image did not move;
  - the CLI README's example line.
- **Data pins and literals:** `power.rs`'s `TABLE_SHA256` (the data change); `validate.rs`'s version list; the version-26 literals in `versions.rs` and `stackups_tier1.rs`.
- **Fence:**
  - `crates/aethelgard-ir` (the rules, the data, the solver's offer, versions, README, tests);
  - the version literals and goldens the bump moves;
  - `spike/1217/`; this file;
  - the one test-only edit in `aethelgard-cli/tests/emit_tier1.rs` granted by Q3.

  Nothing in the generator, the twin, chaos, eda, `verify` or workflows changed.
- **Reduction pass (diff): −6 net, 1 duplicate removed, 0 dead paths removed, 0 protected kept, 3 findings below.**
  - The duplicate: `mcu_row`'s two `refuse` calls became one, differing only in why.
  - The diff otherwise adds guards, each held by its mutant.
  - The three findings, outside the scope:
    - `off_domain::mcu_io`, `power.rs`'s `mcu` and `pin_devices::mcu_row` each look up the MCU row, with three kinds (`UnknownPart`, `PowerPartUnknown`, a context error). One lookup with one kind is a gate-sized change.
    - R3, a duplicated reservoir rule.
    - `reserved_pins` and `analog_pads` are each computed in both `solve_inner` and `Model::new`, the first from before this work order. That is two computations of one source, not two copies of a fact, so it is left alone.

## 9. The package

`fab/sentinel-01-bench/package` moves by its sha lines (the specification's, and what embeds it) and waits for the architect's write=true dispatch. On the dispatch I commit its artifact byte for byte, then push the READY commit.
