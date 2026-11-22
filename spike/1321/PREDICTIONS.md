# AET-WO-1321 — predictions, committed before any run

Written 2026-11-22 13:30Z on `main` `a41a8a8d`, before the baseline is taken and before any line of the build. `1217` has not merged, so item (3) is not built (WO: "only if `1217` has merged"). If it merges before READY, (3) is re-examined at the merge of `main`.

## The baseline (§1.1)

- **B1.** `spike/1211d/measure/sweep.sh` selects **34** committed specifications at `a41a8a8d`, as at `1320` (34 specifications, 280 files). Since then `1216b` and `2308` added no specification outside `spike/`.
- **B2.** `spike/1319/harness/run.sh` runs unchanged. `d4diff` writes **277** cases and `d1319` **84**, as at `1320`.
- **B3.** `crates/aethelgard-ir/tests/fixtures/d1-drv8876.json` is in the sweep, its `emit` exits 0, and its BOM carries CVM2's note "CVM2 declared, TI SLVSDS7B DRV8876 states no value: …". So the sweep draws `declared_bulk`, and the fold (2) is measured by bytes, not only by `pin_devices_drawn_tier1.rs`.

## (1) The reverse check

- **C1.** `check` takes the decoupler table as an argument: `check(rows, decouplers)`. The forward count check reads it, and one new last line refuses an entry with no bus row in `rows`. The `const` assertion passes `ARMS` and `DECOUPLERS` and compiles: today's table names six bus parts, each with a row, so no row is touched.
- **C2.** Every existing bad-table test passes with its expected message unchanged. The tests' `at_compile_time!` and `refused()` pass the one row's own `DECOUPLERS` entry (`core::slice::from_ref`, `&[]` for a pin row or an unnamed part). So each one-row table, and each `Ok` control, has the outcome it has today.
- **C3.** The new test's bad tables are refused at compile time with the new message: an extra entry first (a pin part), in the middle, and last (a part with no row). Its control is two bus rows whose entries come in the other order, accepted.
- **C4.** Mutants, planted by `spike/1321/mutants.sh` in a copy:
  - **R1** deletes the line and is killed by the new test only.
  - **R2** accepts a pin row as an entry's row and is killed by the first bad table.
  - **R3** skips the last entry and is killed by the third.

## (2) The fold

- **F1.** `two_pin` takes `note: Option<String>`. The power tree's three callers pass `None`, and `declared_bulk` calls it in place of its own push. The output is byte-identical (R1 below).
- **F2.** Fold mutant **N1**, `declared_bulk` passing `None`, is killed by the sweep (d1-drv8876's BOM and schematic move) and by `pin_devices_drawn_tier1::d1_is_drawn_with_its_declared_bulk`.

## (4) `1319`'s mutants

- **M.** With the path corrected, `spike/1319/harness/mutants.sh` plants M1–M3 at this branch:
  - **M1** (W25Q64JVxIQ's WP and HOLD swapped) builds and is killed by the harness's bytes.
  - **M3** (DRV8833's CVCP and CVINT reordered) builds and is killed by the harness's bytes.
  - **M2** (BMP581's second decoupler dropped) does **not** build: since `1320`, the `const` assertion over `ARMS` refuses its count. `run.sh` exits non-zero and `compare.sh` fails on the missing output, so the script prints `M2 KILLED`. That is a compile-time refusal, not a byte difference, and it will be reported as one. Each mutant is judged by its `run exit` line and its `compare-*.txt`, not the KILLED line.

## Re-run (§1.3)

- **R1.** **Identical,** branch against base: the sweep (34 specifications, every exit code, printed JSON and emitted file), `d4diff` (277) and `d1319` (84). The final compare is the merged head against the `main` merged in.
- **R2.** `cargo test --workspace` passes. `TIER2_MIN` is unchanged.
- **R3.** Net change in `crates/`: **+30 to +90 lines**, mostly the new test.
