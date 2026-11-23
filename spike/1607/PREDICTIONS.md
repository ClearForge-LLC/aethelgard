# AET-WO-1607 — predictions, committed before the baseline is read

Written 2026-11-23 00:30Z on `main` `28fba951`, before any line of the build.

Two things were measured before this file, and are recorded as measured, not predicted:
- **D1** (on the bus at 00:23Z): a `solve` output given a top-level `lifecycle` key is refused by `generate hwdef --topology` and `check pinout --topology` (`SchemaViolation`, exit 2), because `SolvedTopology` is `deny_unknown_fields`. CI pipes `solve` into both (`ardupilot-image.yml` 288–289, 410). Evidence: `~/.cache/aet-1607/d1/`.
- The base test run for (2) and (3) had started (00:28Z) but none of its output had been read.

## The baseline (§1.1)

- **B1.** `spike/1211d/measure/sweep.sh`'s selection gives **34** committed specifications, as `4300i` measured at this commit: `validate` exits 0 on 32, `solve` on 29, `emit` on 25. Four refuse `emit` with `DrawsNoBoard`.
- **B2.** The parts tables hold three non-`active` parts: `ICM20948` (eol), `ICP10111` (eol), `W25Q64JVxIQ` (nrnd). The specifications whose `validate` line carries `lifecycle` are those placing any of them; `validate` is the oracle for the banner set, and no other command's banner may differ from it.
- **B3.** A repeat sweep with one binary is byte-identical (every exit code, stdout line and emitted file).

## Which verdict lines (6) moves

- **V1.** For each specification in B2's set, these lines gain `"lifecycle":[…]`, equal to that specification's `validate` banner, placed as `simulate --spec` places it (after `check`), and nothing else in the line moves: `emit`; `check drc --board … --spec …`; `check support`; `prove … --spec …`; and `check pinout` on a topology placing one of the parts.
- **V2.** `check drc` with no `--spec`, `check erc`, and `prove` with no `--spec` are unchanged: they read no specification.
- **V3.** For every specification outside B2's set, every line is byte-identical (the key is skipped when empty).
- **V4.** `solve` waits on D1. If (a) is ruled, `solve`'s line gains the key for B2's set, and the `hwdef`/`pinout` verdicts' `topology_sha256` move only on such files.
- **V5.** No emitted file moves: `emit/`'s `.kicad_sch`, `.kicad_pcb` and `.bom.csv`, `generate`'s tree, and the ELF. No `fab/` byte moves.
- **V6.** `verify`'s line on a drawn board is unchanged: its gates' `lifecycle` keys are not copied into `gates[]` (`gate_report` picks named fields), and its own `lifecycle` already carries the banner.

## (1) `check support` pinned

- **P1.** `check support` calls `kicad::pinned` on `kicad-cli version`. With a stub printing `10.0.6`, `9.0.7` or `10.0.7-1` it is refused `ToolUnusable` (exit 4, id `kicad-cli`); on base it runs on. With `10.0.7` it gets past the version. Mutant **M1** (the call removed) is killed by the new test.

## (2) Tier-1 scratch off `$HOME`

- **P2.** On base, with `HOME` a private directory holding `.cargo/config.toml` (and `CARGO_HOME`/`RUSTUP_HOME` at the real ones), the tier-1 tests that take `build` or `prove` past the pre-flight fail with `CargoConfigRefused`. I expect them in `generate_tier1`, `lifecycle_tier1` and `tools_tier1` (the `prove` stubs), and possibly `boot_tier1` and `mcp_tier1`.
- **P3.** No other directory outside `$HOME` passes the CLI's scratch rule for a non-root user except `/run/user/<euid>`: `/tmp` and `/var/tmp` are world-writable, and anything under the repository has a `Cargo.toml` above it (`WorkspaceAboveCrate`). Root keeps `/var/lib/aethelgard`. `private_tmp` uses these, refuses a base under the canonical `$HOME`, and says so by name.
- **P4.** Tests that leave `TMPDIR` unset and pass `HOME` through (such as `tools_tier1`'s) take the CLI's own default under `$HOME`. Fixing them means setting `TMPDIR` in those files, which are outside the fence's file list. If any is in P2's set, it is flagged.
- **Mutant M2:** `private_tmp` back under `$HOME` is killed by the new test (with a config present).

## (3) `fab_tier1` scratch under the repository

- **P5.** On base with `CARGO_TARGET_DIR` outside the repository, `a_rail_or_bus_no_fault_kind_reaches_is_named` fails with `BaseOutsideRoot`. The two other `fab_tier1` tests that write a scenario with a `base` (lines 350 and 602) fail the same way, if their run reaches `read_base` under that scratch.
- **P6.** Other test files that write scenarios under `CARGO_TARGET_TMPDIR` and run from the repository (`campaign_tier1`, `attribute_seeded`) may fail the same way. They are outside this fence and are reported, not changed.
- **Mutant M3:** the scratch back under `CARGO_TARGET_TMPDIR` is killed only when `CARGO_TARGET_DIR` is outside; the test asserts its own scratch is under the repository, so it also dies in the ordinary layout.

## (4) `readme_status.rs`

- **P7.** Roadmap §5 has five phases, P0–P4; P0–P2 are marked passed, P3 and P4 carry an intent gate. The README names each: P0–P2 "complete", P3 "its exit gate waits", P4 "under way". The stricter test passes on today's README.
- **Mutants:** **M4** (a phase the README does not name is accepted), **M5** (an open phase stated as complete or passed is accepted), each killed by a synthetic README in the test.

## (5) `verify` on a no-board specification

- **P8.** Plain `verify --spec imu-baro-1v8.json` (and the other three `DrawsNoBoard` specifications) runs: `erc`, `drc`, `support` and `extract` each appear with verdict `not_applicable`; `prove`, `build`, `simulate` (and `campaign`, given scenarios) run as on a drawn board. Its `ok` follows the gates that ran. `--fab-out` and `--against-fab` still refuse `DrawsNoBoard`.
- **P9.** On a drawn board, `not_applicable` is never written, and `passes` treats it as a failure. Mutant **M6** (`passes` accepts `not_applicable` on a drawn board) and **M7** (the refusal restored) are killed.
- **P10.** The `verify` row of `emit_tier1`'s stand-in test must move from "refused" to "runs". That file is outside the listed fence.

## Gates and the rest

- **G1.** `cargo fmt`, `clippy -D warnings` and `cargo test --workspace` pass. No tier-2 test is added, so `TIER2_MIN` and `ci/test-weights.tsv` are unchanged.
- **G2.** Net change in `crates/`: +250 to +600 lines, most of it tests.
