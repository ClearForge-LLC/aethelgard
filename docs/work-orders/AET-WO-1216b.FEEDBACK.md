# AET-WO-1216b — FEEDBACK: the power sequence judged at VDD = 0

**Branch:** `wo/aet-1216b-off-domain-paths` from `main` `c9d4096b` (main's docs `c79b9820` merged in).
**Builder:** Claude (Opus 5.5 orchestrating). One Opus adversarial reviewer was given the code and §5, not my conclusions; its report was checked at source before use. Written 2026-11-22.

**Status: READY on the code; AWAITING-PACKAGE.** Specification 25 moves `fab/sentinel-01-bench/package`'s sha lines, and the new `twin_limits` rows move its verdict. The package needs a `write=true` dispatch (§9).

**The answer, in five lines:**
- **Step 25 is built in `aethelgard-ir` (`off_domain.rs`).** It walks every path onto an MCU net from off `mcu.supply`, by `1216`'s walk-1 classes. Each path whose rail can lead is judged against DS12117 Rev 10 Table 10 at VDD = 0: 4.0 V, and 9.0 V on BOOT0. The limits are data on the MCU row (`power-parts` 7, `io.absolute_maximum`).
- **The walker equals walk 1 on all 34 committed boards, path for path: 225 / 225.** Before the fixture edit it refuses D1 alone, `OffDomainAboveLimit` on `DRV_IPROPI`, 4665 > 4000 mV. The message names the 3 mA as a recommended operating condition, not a maximum (D5). After R2 became 1.07 kΩ, nothing is refused (3330 mV).
- **An unbounded path is named and never passed.** It is one `OFFDOMAIN` row in `verify`'s `twin_limits`: silicon-only, never acknowledged, never counted as judged. 222 such paths are on 33 boards. No `validate` byte moves (D2).
- **Every committed specification validates, solves and emits byte-identically except its sha.** The null sweep covers 34 specifications and 280 files; D1 is identical too, because the solved topology does not carry R2's ohms. Specification 25 adds `CHANGES` row 25 (`OffDomainAboveLimit`).
- **The adversarial pass found one real gap, now fixed and tested.** An EN/PG gate granted the exemption even when an off-board end named the EN net. Two further rulings are asked of you (D1, D2 below).

## 0. How it was made

- **Predictions:** `4e4d84f1`, before any run (`spike/1216b/PREDICTIONS.md`).
- **The walker at specification 24, measured before the fixture edit:** `934c5b84`.
- **D2's rows, R2 and the tests:** `de32a361`.
- **The bump:** `7586dc21`.
- **The adversarial fixes and the goldens:** `00a024f9`.
- **The reduction:** `7f221410`.
- **Results:** `spike/1216b/RESULTS.md`, with evidence in `spike/1216b/out/`.
- **The oracle:** `spike/1216`, re-run on `c9d4096b`. It is byte-identical to its record and still gives 1 / 34.

## 1. What was built (WO §1)

- **The walker** (`off_domain::paths`) lists every path onto an MCU net from off `mcu.supply`, in these classes:
  - the battery sense divider: the battery's declared maximum through the divider's largest ratio, which is step 11's computation, now shared as `power::sense_max_microvolts`;
  - an I2C bus's pull-ups, a crossing's A side and its chip-select pull;
  - a power-tree control wire on an MCU net;
  - a pin device's pull to a domain;
  - a pin device's current output: the row's `output_max_ua` into the resistor at its highest, by the ADC rule's own `resistor_error_ppm` and `band`;
  - each connector pin, test point and foreign clock on an MCU net.
- **The exemption** is read from drawn wiring only, never from a declaration (`INV-07`). A rail is exempt only if:
  - its chain passes through `mcu.supply`, or its regulator's `EN` is wired to the `PG` of the regulator outputting `mcu.supply` or one fed from it (the strict reading, ruled 08:58Z); and
  - no off-board end names the rail, a rail between it and `mcu.supply`, or the gate's EN net.

  A device with two supplies is judged when either supply leads. A control pin is judged by its own part's supplies.
- **The limits** are the MCU row's `io.absolute_maximum`:
  - FT is `Min(VDD + 4.0 V, 6 V)` and TT is 4.0 V;
  - BOOT0 is 9.0 V, and "any other pin", NRST among them, is 4.0 V;
  - each is cited to Table 10, p. 103;
  - an unplaced or unheld pin takes the least row.

  The parser refuses a Table 10 list that does not rate Table 13's families in order. `validate` reads every limit at VDD = 0. Note 3's reading admits a voltage at the limit (D4), and the test pins 4000 mV exactly.
- **D5 as data:** `output_max_basis` (`maximum` or `recommended_operating`) is now required on a current output (parts 13). The DRV8876's IPROPI is `recommended_operating`. Its `page` text now says "0 to 3 mA, recommended operating conditions, p. 4 §6.3: no maximum is stated", where it used to say "max 3 mA".
- **D2:** `ledger::off_domain_rows` (CLI) writes one `OFFDOMAIN.<declaration path>` row per unbounded path into `twin_limits`. Each row carries its pin (declared, or `solve`'s) and Table 10's limit there. The IR walks and the CLI writes the rows, so `RUL-16` holds.
- **D3:** R2 is now 1.07 kΩ.
  - At the fixture's own tolerance (CRCW0603: 1 % + 100 ppm/K × 65 K over D1's −20 to 85 °C + 2 % drift, 36 500 ppm) it is at most 1110 Ω.
  - The held 3 mA then gives 3330 mV.
  - The 3.5 A peak on the typical 1000 µA/A ratio gives 3885 mV. **That figure rests on the typical ratio:** no AERR is stated above 2 A. The test that judges R2 says so (`d1_s_r2_of_1k07_clears_4v0_on_the_held_figure_and_at_the_typical_peak`), as ruled. `power::Resistor` has no `source` field.
  - 1211e's A2 still holds: the full scale is 2214 mV against VREF's minimum of 3135 mV.
- **Specification 25:**
  - `CHANGES` row 25, which adds no key and the rule `OffDomainAboveLimit`;
  - the README's version prose and step 25's row;
  - 34 specifications moved by `spike/4240a/kit/renumber.py`;
  - the solved version stays at 17.
- **F7:** both walks were re-run on D1 drawn (`spike/1211f/d1-drawn.json`). Classified by the pin row's kind (`spike/1216b/f7.py`), EN, PH, nSLEEP and nFAULT are inputs or open drain, so none is a source, and the walks agree on every D1 row.

## 2. Acceptance (WO §2)

- **The walker's paths equal walk 1's, path for path,** on all 34 boards: 225 / 225 (`out/compare-pre.txt`). The comparison does not compare:
  - the pin, because walk 1 reads `solve`'s pins and the walker reads the specification's;
  - each lead's wording;
  - walk 1's five "no:" rows, Sentinel-02's crossing on `mcu.supply`, which are no path.
- **Refusals before and after the edit:**
  - **Before the edit, D1 alone is refused** by `OffDomainAboveLimit`.
  - **After it, none is.**
  - D2 keeps `PartLimitUndeclared`, and `invalid-zero-domain` keeps `ZeroNominalVoltage` (`out/walker-post/`).
  - After the edit, D1's figure is the only row that moves against walk 1's record: 4665 → 3330 mV (`out/compare-post.txt`).
- **The null sweep is identical** (`out/sweep-result.txt`): `c9d4096b`'s binary on its own tree against the branch's binary on the branch, with each specification's sha put back. That is 34 specifications and 280 files, with exit codes 86 × 0 and 16 × 2 on both.
- **Gates and CI:** §8.

## 3. Adversarial pass (WO §5, full)

The Opus reviewer worked on its own copy of the tree. Its cases are in `~/.cache/aet-1216b/adv/`. What it found:

| # | finding | done |
|---|---|---|
| A1 | **Bug.** An EN/PG gate granted the exemption although a connector pin also named the EN net, so a peer could raise EN with `VDD_3V3` down and put 5.25 V on PA1. The case validated and emitted. The emitter draws a control wire only to a net a connector or test point ends, so on a drawn board every gate has an off-board end on its EN net. | **Fixed:** an off-board end on the EN net voids the gate. The reviewer's case is now refused `OffDomainAboveLimit`. Tested, with a mutant. See D1. |
| A2 | A control wire's rail was the regulator's *output*, so `mcu.supply`'s own regulator's controls were never listed. | **Fixed:** a control pin's rails are its part's supplies, a regulator's input and a translator's two sides. Tested, with a mutant. |
| A3 | A test point on an exempt rail did not void the exemption, although a connector does. | **Applied:** a test point is an off-board end, as everywhere else in the walker. Tested, with a mutant. See D2. |
| A4 | **Pre-existing:** a specification with both pin devices and test points is refused `TestPointReferenceTaken` (`bench.rs:154` slices `names::references` assuming test points come last; `names.rs:125` chains pin devices after them). | Not in the fence. **F1.** |
| A5 | Walk 1 rounds R2 as a float (1109.06 Ω, 3328 mV); the engine rounds the ohm up first (1110 Ω, 3330 mV). The engine's figure is the safe one. P3.1 predicted 3328. | Recorded (RESULTS M5). |
| A6 | What the comparison does not compare; the exemption logic is exercised only by tests, because no committed board wires a PG. Walk 1's `spi_device_output`, `i2c_device_open_drain` and `pin_device_output` have no walker arm: steps 5a and 24 refuse each first, and they give 0 rows. | Stated in §2 and here. |
| A7 | One assertion was on constants only. | **Fixed:** it is now computed from the walker's figure. |
| A8 | Two silent skips, both unreachable after steps 5 and 24. | **Labelled** `belt-and-suspenders:`. |

**Checked clean by the reviewer:**
- Table 10 and Table 11 note 3 against the held DS12117 text.
- The limit read only at VDD = 0, and never from Table 13.
- The limits for BOOT0, NRST, PH0, TT pins and unplaced pins.
- Every figure rounds outward.
- Chain loops are refused at step 16.
- The walker's MCU net set equals `solve`'s assignment, GPIO and ADC keys on all 29 boards that solve.
- Unbounded paths appear only in `twin_limits`.

## 4. Tests and mutants (WO §1.5)

- **`tests/off_domain.rs`** has 16 tests:
  - one per path class;
  - the guards: downstream, a connector on a rail between, a connector on the rail, a test point, the gate net, an upstream PG, two supplies;
  - each limit: 4.0 V to the millivolt, BOOT0's 9.0 V, Table 10 not Table 13 with VDD up;
  - the unbounded rule;
  - the two tables' new fields.
- **`ledger_tier1`:** `every_unbounded_path_is_a_twin_limit_never_judged` (Sentinel-03: 35 rows, BOOT0's at 9000 mV, PE9's at 4000 mV), and the verify-line test, which now lists the four connector rows beside `IDENTITY.U3`.
- **1211e's tests updated, intent kept:**
  - a 5 V pull on an FT pin passes A1 and is now refused by step 25;
  - A2's literals were re-taken at R2's new band;
  - the RAIN test uses 1.5 kΩ explicitly.
- **Mutants:** one per guard this work order adds, against the suites they live in (`spike/1216b/mutants.py`, `out/mutants.json`): **28 of 28 killed** (the first run's 18 "did not compile" were the harness's copy excluding `spike/`, which a test `include_str!`s; RESULTS M7).
- **The arms only `paths()` can reach.** These are tested through `paths()`, because an earlier step refuses their input first:
  - an I2C bus or a crossing off `mcu.supply` (steps 5a and 16; labelled `belt-and-suspenders:`);
  - a battery divider above 4.0 V (step 11 refuses it above VREF+);
  - an exempt rail above 4.0 V (no held regulator boosts from `mcu.supply`).

## 5. Predictions beside results

The full table is in `RESULTS.md`.
- P1 (the oracle), P2 (the walker before the edit) and P4.1/P4.3 hold.
- **P3.1 is 2 mV low:** I predicted 3328 mV; the engine gives 3330 mV (A5).
- **P3.4:** no test outside `aethelgard-ir` moved with R2. Inside it, four did, plus three A1 tests that step 25 now refuses.
- **P4.1 holds more strongly than predicted:** D1's outputs did not move at all.

## 6. Unjudged, as the work order says

These are not judged:
- **Dynamic cases:** power-down, brown-out (including D1's nSLEEP between VIL and VIH), battery hot-plug.
- **VDD between 1 V and 1.62 V.**
- **A connector's drive bound:** a connectors work order, `1216` D2's row.
- **IPROPI's real maximum and clamp:** `1216` D5's row.

## 7. Decisions for the architect, and rows

**Decisions:**
- **D1 · A gate net an off-board end names grants nothing (A1, applied).**
  - With it, (b)'s EN/PG exemption can never hold on a drawn board today: the emitter draws a control wire only to a net a connector or test point ends (`schematic.rs:1607`, `schematic_power.rs:582`). It can hold only on a twin stand-in, and no committed board has one.
  - **Lean: keep it.** An EN a peer can drive is not an order the tree enforces. The day a pull on a control net can be declared (`1216` F2), the gate net no longer needs an off-board end.
- **D2 · A test point voids an exemption as a connector does (A3, applied).**
  - The ruling named connectors. The walker already treats a test point as an off-board end, and a bench probe can back-feed a rail.
  - It affects no committed board.
  - **Lean: keep it.** It is one line to revert if you rule connectors only.

**Rows for §7.1:**
- **F1 · Pre-existing: pin devices and test points cannot be combined** (A4): `TestPointReferenceTaken` on any test point beside a pin device. **Proposed row:** "`bench::validate` takes the test points' references by name, not by position in `names::references`."
- **F2 · Walk 1 and the engine round R2 differently** (A5). The engine's whole-ohm ceiling is the safe direction. **Proposed row:** none; the spike's float is the record.
- **F3 · D1's reservoir C1 is no longer needed at 12 bits.** R2 is at most 1110 Ω, under RAIN's 1150 Ω. It still validates, as a declared extra. **Proposed row:** "D1's fixture drops C1 or keeps it with a reason, when a work order next edits it."
- **F4 · `1216` D5's row stands:** hold IPROPI's stated maximum and the clamp's figure when TI states them. Until then the refusal rests on a recommended-operating figure, and says so.
- **F5 · `1216` D2's row stands:** a connector pin's off-board drive bound, so that the 222 `OFFDOMAIN` rows can be judged.

## 8. Gates, fence, reduction pass

- **Gates** (Furnace, target in the repository, at `7f221410` plus this file):
  - `cargo fmt --all -- --check` passes.
  - `cargo clippy --workspace --all-targets -- -D warnings` passes.
  - `cargo test --workspace`: 160 binaries, **2744 passed and 1 failed**. The failure is `bench_board_tier1::the_committed_package_is_bound_to_its_specification_scenarios_and_emit`: the committed package binds the specification-24 sha, and it waits on the dispatch (§9).
  - CI is the confirmation.
- **Fence:**
  - `crates/aethelgard-ir`: the walker, the limits data, `output_max_basis`, the refusal, versions, the README and tests;
  - the CLI: `ledger::off_domain_rows`, its one call in `verify` and its tests (D2 only);
  - the D1 fixture's R2;
  - the version literals and goldens the bump moves, each by its generator:
    - `renumber.py` for the specifications;
    - `generate` for `boards/sentinel-01*` (16 files);
    - `AETHELGARD_BLESS` for the svd-hal UI goldens;
    - `solve` and `generate hwdef` for the hwdef fixtures (6 files);
    - `solve_cli`'s and `order_2203`'s digests, each shown equal to main's with the specification-24 sha put back (`spike/1216b/order_back.rs`);
    - the CLI README's line;
  - `spike/1216b/` and this file.

  Nothing in the generator, the twin, chaos, eda, workflows or `ci/` changed. No `ci/test-weights.tsv` row was needed, because no tier-2 test was added.
- **Reduction pass (diff):** −8 net in `off_domain.rs` (22 removed, 14 added; `7f221410`).
  - `path()` now carries the no-path case. That removed `push_rail`'s own supply check and an unreachable fallback.
  - `Walk::new` is infallible, and step 25's redundant early return is gone.
  - Duplicates avoided at the source:
    - the divider's figure is step 11's `sense_max_microvolts`;
    - the resistor band is the ADC rule's;
    - the limit and citation are one `limit_at_zero`, for `validate` and the CLI alike.
  - **Proof:** `off_domain` and `pin_devices` were re-run green, and the mutants ran on the reduced code.
  - **Protected (`belt-and-suspenders:`), kept:** 4 arms (the I2C pull, the crossing, and two skips).

## 9. The package

`fab/sentinel-01-bench/package` must be re-written by a `write=true` dispatch. Two things move it:
- the specification's sha, in `FAB-NOTES.md` and `MANIFEST.json`, as in `1211e`;
- the verdict's `twin_limits`, which gains 20 `OFFDOMAIN` rows for `examples/sentinel-01.json`.

`bench_board_tier1::the_committed_package_is_bound_to_its_specification_scenarios_and_emit` fails until then, by the sha alone. The queue is 1606, then 2308. I commit the dispatch's package byte for byte when it is run.
