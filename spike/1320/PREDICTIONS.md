# AET-WO-1320 — predictions, committed before any run

Written 2026-11-22 09:55Z, on `main` `f46c3c42` (`TIER2_MIN` unchanged by this work order), before the baseline is taken and before any line of `check()`.

## The baseline (§1.1)

- **B1.** `spike/1211d/measure/sweep.sh` selects **34** committed specifications at `f46c3c42`. That is `1211f`'s count (its commit message: 34 specs, 280 files). `1216` added specifications only under `spike/`, which the sweep skips, and `1606` added none.
- **B2.** `spike/1319/harness/run.sh` runs unchanged at `f46c3c42`. The `dump` shim's anchor is still in `schematic.rs`, and `d4diff.rs` and `d1319.rs` still compile.
  - `d4diff` writes **277** cases (`1319` F4) and `d1319` writes **84** (`1319` §5).
  - If either breaks, a copy patched under `spike/1320/` is applied to both trees alike and recorded as a finding. `spike/1319/` is not edited.
- **B3.** `spike/1319/harness/mutants.sh` names `schematic_pin_device.rs`, which `1319`'s Q1 ruling renamed to `schematic_arms.rs`. It cannot run at `main` as written. It is not needed here; it is recorded as a finding.

## The five checks (§1.2)

- **C1.** All five can be written in the `const fn` on stable Rust, with `while` loops over the slices and `same()` for strings, as `1319`'s are. §7's first stop does not fire.
- **C2.** Today's `ARMS` passes all five. The crate compiles, and no row is touched.
- **C3.** Every existing bad-table test passes unmodified, so no new line steals an earlier refusal.
- **C4.** Each new check has a test whose bad table is evaluated by `check()` in a `const` item, so the compiler refuses it. Each mutant deletes that one check in a scratch copy and is killed by that check's own test, named.
- **C5.** The decoupler count has no independent source in the row. Rail-pin counts do not predict it: ICM42688P has two rail pins and three decouplers, BME280 three and one. It is therefore stated once inside `check()` per bus part, and a bus part it does not name is refused. No `Arm` field is added (§"It is NOT": no change to a row).

## Re-run (§1.3)

- **R1.** **Identical:** the sweep (34 specifications: every exit code, every printed JSON normalised by `same.sh`, every emitted file), `d4diff` (277) and `d1319` (84), branch against base. `check()` runs only at compile time and in tests, so no drawn path changes.
- **R2.** `cargo test --workspace` passes. `TIER2_MIN` is unchanged.
- **R3.** Net change in `crates/`: **+80 to +200 lines**, mostly tests.
