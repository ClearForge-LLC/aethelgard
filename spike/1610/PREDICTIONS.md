# AET-WO-1610 §1.1 — predictions (written before any run, 2026-11-24, main 72d85eef)

Each is checked in `MEASURED.md`. Reading the code is not a run: these are what the runs should show.

## P1. Readers of `off` for a missing harness (R6)

- `prove` with no harness has no `prove` gate in `gates`, so `ran_table` falls to `GATES`' `Absent::Off("a Kani
  harness in the generated board crate's src/proofs.rs")` and the row reads `off: a Kani harness …`, while
  `findings` carries `no_harness` and the verdict is `fail`.
- The one reader of that row is the unit test `verify::tests::off_and_not_reached_come_from_each_gates_own_verdict`.
  The README describes the rule (`crates/aethelgard-cli/README.md` §`ran`). No CI script, no workflow, no MCP code
  and no committed golden or fixture reads the label: MCP passes the verdict through.
- No committed verdict has a no-harness `prove`, so the bench package does not move for (1).
- Versioning: the verify verdict carries no format version of its own; nothing bumps for a label value.

## P2. Readers of `judged_nothing` (R5)

- Code: `verify.rs` only (the `Report` field, `judged_nothing()`, `conclude`, two `Report` literals).
- Tests: three — `verify::tests::judged_nothing_names_exactly_the_gates_and_runs_that_judged_nothing` (unit),
  `verify_2k2_tier2` (asserts `[]`), `board_tier2::verify_names_what_judged_nothing_on_the_motor_driver`.
- Not code: a comment in `ci.yml`, the test's name in `ci/test-weights.tsv`, the CLI README, work orders.
  No reader outside the tests needs the field: the STOP condition is not met.
- **Not wholly derivable from `ran`:** `campaign:<scenario>` comes from a run's `observation_witness`, which the
  table does not carry (the campaign is one `judged n` row). The fact stays in `campaign[].observation_witness`.
- Removing the key moves the verify verdict line, so the bench package's `MANIFEST.json` `verdict_sha256` moves
  and needs the architect's `write` dispatch, as `2306` did.

## P3. The three F3 fields

| field | the check its absence skips | where | committed specifications carrying it |
|---|---|---|---|
| `spi_devices[].orientation` | the IMU seat: placement rotation and `check drc`'s read-back `ImuOrientationDisagrees` | `eda` `schematic::seats` (`continue`) → `stack.imus` → `route::read::declared` | `examples/sentinel-02-bench.json`, `-hse8`, `cli/tests/fixtures/sentinel-02-own.json` |
| `board.max_outline_um` | the outline bound: `validate`'s `OutlineTooSmall`, the placer's bound | `cli` `drawing_holds`, `eda` `schematic.rs` `if let Some(b)` | `cli/tests/fixtures/sentinel-03.json` only |
| `power.loads[].reference` | the load's part binding: `LoadNamesNoPart`, `LoadNamesCountedPart` (ir), `LoadNamesUndrawnPart`, `LoadPartOffRail` (cli) | `ir` `names::loads`, `cli` `loads_drawn` (`continue`) | 24 of 27 loads carry it; the 3 without are lumped budgets (a 5 V port budget, a 1.8 V domain budget) |

- With each key absent there is no declared value to judge against (`INV-07`), so "make the check run" can only
  mean refusing the absence. Each such refusal would refuse a committed specification:
  - an ICM-42688-P without `orientation`: `sentinel-03`, `hse-crystal-16`, `hse-external-8`, `sentinel-02-shaped`;
  - a drawn board without `max_outline_um`: every drawn committed board but `sentinel-03`;
  - a load without `reference`: `sentinel-03`, `ir/tests/fixtures/sentinel-02-power.json`.
- So each field's choice is predicted to be the table row. The table already writes an absent key as
  `off: <key>` (`extract.supply off: power.drop_limits`, `drc.pairs off: diff_pairs`), and that is the label that
  fits; the WO's text says `not_reached`. A decision for the architect.
- `orientation` has a gate that judges it when present (`check drc --spec`), so its row is a `drc` part.
  `max_outline_um` and `reference` are judged only by `validate` and `emit`, which `verify` runs before any gate.

## P4. The eight F1 mutants

- Each deletes the only key of an externally tagged enum object, leaving `{}`: `mcu.hse.external`,
  `mcu.hse.crystal`, a strap's `tie`, an analog's `full_scale` and `transfer` key.
- Each is refused today `InvalidJson`, exit 3, with serde's `expected value at line L column C`.
- Re-running `spike/1608/corpus.py`'s deletions (the 1608 rows were not kept) finds exactly 8, on the same paths.

## P5. `UnwitnessedAcknowledgementStale` (F2)

- The message quotes the entry's stale `fields` verbatim. `validate` shape-checks the register
  (`register_name_ok`) but not a field's text, so a planted field string (spaces, backticks, a fake "the fix:")
  reaches the message today.
- The message never lists the ledger's own fields for that register and class.
