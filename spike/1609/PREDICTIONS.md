# AET-WO-1609 — predictions, before any run

Written 2026-11-23 on `e5984c86`, before any measurement. **Already read** (so these are not blind):
`crates/aethelgard-cli/src/verify.rs` (the `Report` and `Gate` structs, `scope_of`, `judged_nothing`,
the per-item `gpio`/`adc`/`bridge` gates, `not_run`, `not_applicable`, `board_gates`, the campaign and
`attribute` block), `crates/aethelgard-cli/src/renode/foreign.rs` (`RULES`, the read-back at
L1230–1320), `ci/foreign-judge.sh`, and review 3's B5, C2 and A15 with the architect's response.
Nothing has been run.

## P1. The gates in `verify`'s verdict

| Gate | Prediction: where on/off is decided | Prediction: the key |
|---|---|---|
| `erc`, `drc`, `support`, `extract` | `verify.rs` `run_gates`: `not_applicable` when the board is not drawn | `board.drawn` |
| `extract`'s supply gate (a part of `extract`) | inside eda (`supply.rs`): no gate when `power.drop_limits` is absent; `extract`'s `judged` just shrinks by the domains | `power.drop_limits` |
| `prove` | one row per harness; with no harness, no row at all | none named today: the harness list is the generated crate's |
| `build`, `simulate` | always run on a drawn or undrawn board | none (always on) |
| `gpio`, `adc`, `bridge` | `verify.rs` drops each when its item list is empty: a silent gate | the specification's pin devices (`pin_devices`), implicit in the data |
| `campaign` | the `--scenarios` flag, not a specification key | none: a flag |
| `attribute` | only for a seeded run that failed as expected | none: the run's outcome |
| `silicon`, `ledger`, `lifecycle`, `twin_limits` | judging structures outside `gates`, always computed | none |
| the via-ring rule (inside `drc`) | eda `stack.rs`, off without `fab_tolerances` | `board.fab_tolerances` (or wherever it sits) |

Prediction: **at least two gates' on/off cannot be named by a specification key today** (`campaign`, a flag;
`gpio`/`adc`/`bridge`, implicit in the data), which is a §7 stop condition to raise, and the supply gate's
"off" is decided outside the fence (eda), so a table derived from the gates' own results needs either eda to
report "off" or the CLI to re-derive it from the specification.

## P2. The verdict's version

Prediction: `verify`'s verdict carries no version field today; its format changes have been additive keys
(as `2306` added `judged_nothing`). The "repo's own versioning" for a verdict is to be found, not assumed.
Prediction: `MANIFEST.json`'s `verdict_sha256` pins the verdict line, so a new key moves every committed
package's manifest.

## P3. The verdict's readers

Prediction: the MCP `verify` tool (passes the line through), `fab.rs` (the manifest's `verdict_sha256`),
`against_fab` tests, the tier-2 verify tests (`verify_2k2_tier2.rs`, `verify_47k_tier2.rs`, `against_fab_tier2.rs`),
`ci/` scripts and workflow inline Python that grep the verdict, and the committed packages. `replay` and
`attribute` read the archive, not the verify verdict.

## P4. The late panic

Prediction: **`simulate --foreign` passes it.** ArduPilot's ChibiOS `panic()` prints and loops with
`hal.scheduler->delay(500)` in thread mode, so at the window's end `ICSR.VECTACTIVE` is 0 and the PC is in
an executable `PT_LOAD`; every witness before the panic is already in the log. Nothing in the judge reads
the console or a panic symbol. A `udf` (HardFault) after the last witness is predicted to **fail** by
`core_in_exception`, unless ArduPilot's fault handler resets the core (then predicted to pass, as a
watchdog reset would).
