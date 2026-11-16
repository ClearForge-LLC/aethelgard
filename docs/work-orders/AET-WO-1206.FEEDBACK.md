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

## 1. §1.1 measurement

(pending)
