# AET-WO-1610 §1.1 — measured (2026-11-24, Furnace, main 72d85eef)

Predictions: `PREDICTIONS.md`, committed first (5ea10b95). **What "first" covers:** the runs (`f1`, `f2`, `f3_*`,
`f1_mechanism`) all post-date it. The reader searches and the specification scans behind P1–P3 had already
printed while I oriented, before the predictions were written; for those, P1–P3 record what I had read, not a
blind guess. Binary: `target/debug/aethelgard` of 72d85eef.
Runs: `f1.py` → `f1.out`, `f2.py` → `f2.out`, `f3_loads.sh` → `f3_loads.out`. Reader searches: `git grep`
over the whole tree (work orders and spikes excluded), as quoted below.

## 1. R6: `prove` with no harness (P1 held)

- `verify` pushes one `prove` gate per harness (`proof_and_boot_gates`, `verify.rs:2586`). With none, it pushes
  no `prove` gate and only the finding `no_harness` (error). `ran_table` then reads `GATES`' entry
  `Absent::Off("a Kani harness in the generated board crate's src/proofs.rs")`, so the row is `off`, and the
  verdict is `fail` by the finding.
- **Reachable only by a unit:** the generator always writes at least two harnesses (flash, pwr; `1609`
  RESULTS §1). Through the CLI the row never appears; no committed verdict carries it.
- **Readers of the `off` label for `prove`:** one, the unit test
  `verify::tests::off_and_not_reached_come_from_each_gates_own_verdict` (`verify.rs:4452`). `git grep` of the
  key text finds nothing else in code, CI, workflows, MCP, goldens or fixtures. The CLI README §`ran`
  states the rule in prose (`crates/aethelgard-cli/README.md:965`).
- **Readers of `off`/`not_reached` in general:** the CLI (`verify.rs`, `main.rs`, `extract.rs`, `verdict.rs`),
  its tests (`tests/common/verify.rs`'s `ran()` checks the table whole; `generate_tier1`, `supply_tier1`,
  `hse_eda_tier1`, `verify_2k2_tier2`, `board_tier2`), and the README. **No CI script, workflow, MCP code or
  committed golden reads either label** (`git grep` over `ci/`, `.github/`, `src/mcp.rs`, `fab/`, `examples/`
  is empty). MCP returns the verdict line as the CLI prints it.
- **Versioning:** the `verify` verdict has no format version of its own (no `version`/`schema` key in
  `Report`); a label value moves no version. Nothing bumps.
- **The ruled label** `not_reached: no_harness` is not reached by the STOP condition: no reader outside the
  tests depends on `off` for a missing harness.

## 2. R5: `judged_nothing` (P2 held, with one correction)

`git grep judged_nothing` (work orders and spikes excluded):

| where | what | a reader? |
|---|---|---|
| `src/verify.rs:95, 621, 1921, 2196, 4160` | the `Report` field, `judged_nothing()`, `conclude`, two literals | the producer |
| `src/verify.rs:3182` | `judged_nothing_names_exactly_the_gates_and_runs_that_judged_nothing` | test 1 (unit) |
| `tests/verify_2k2_tier2.rs:405` | `judged_nothing == []` on the reference | test 2 (tier 2) |
| `tests/board_tier2.rs:1959` | `["extract", "campaign:droop-bor3"]` on the motor driver | test 3 (tier 2) |
| `.github/workflows/ci.yml:474` | a comment naming test 3 | no |
| `ci/test-weights.tsv:63,138`, `ci/fixtures/p3-d09a5187-weights.tsv` | test 3's **name** and weight | no (a test name, not the field) |
| `crates/aethelgard-cli/README.md:960` | "after `judged_nothing`" (the table's place) | prose |
| `docs/roadmap.md` | history rows | prose |

- **No reader outside the tests needs the field.** The STOP condition is not met.
- **Correction to "derivable from `ran`":** the gate members are derivable (`judged 0` rows); the
  `campaign:<scenario>` members are not. They come from each run's `observation_witness`, and the table carries
  the campaign as one `judged n` row. The fact itself stays in the verdict, at `campaign[i].observation_witness`
  (`nothing_in_scope: true` and its reason), which test 3 already asserts whole. Test 3 keeps reading it there.
- Test 3's name stays as it is, so `ci/test-weights.tsv` and the shard plan do not move.
- **The bench package moves:** removing the key moves the `verify` verdict line, so `MANIFEST.json`'s
  `verdict_sha256` moves (only that, as at `2306`). It needs the architect's `write` dispatch of `ci.yml`'s
  `bench-package` job.

## 3. F3: the three fields (P3 held)

| field | the check its absence skips | where it skips | committed specifications carrying it |
|---|---|---|---|
| `spi_devices[].orientation` | the IMU seat: the placer's rotation, and `check drc --spec`'s read-back `ImuOrientationDisagrees` (`eda route::read::declared`, called by `main.rs:974`) | `eda schematic::seats`, `let Some(orientation) = d.orientation else { continue }` → `stack.imus` empty | `examples/sentinel-02-bench.json`, `examples/sentinel-02-bench-hse8.json`, `cli/tests/fixtures/sentinel-02-own.json` |
| `board.max_outline_um` | the outline bound: `validate`'s `OutlineTooSmall` (`main.rs` `outline_holds`), the placer's `OutlineOverBound` (`eda schematic.rs:985`) | `drawing_holds`' early `Ok(())`; `if let Some(b) = bound` | `cli/tests/fixtures/sentinel-03.json` only |
| `power.loads[].reference` | the load's part binding: `LoadNamesNoPart`, `LoadNamesCountedPart` (ir `names::loads`), `LoadNamesUndrawnPart`, `LoadPartOffRail` (cli `loads_drawn`) | `let Some(r) = &l.reference else { continue }`, twice | 24 of 27 loads carry it; the 3 without: `VDD_5V` 2000 mA "DS-009 port budget" (`sentinel-03`, `ir sentinel-02-power`), `VDD_1V8` 20 mA "domain budget" (`ir sentinel-02-power`) |

**What each check would judge, and why it skips.** In all three the key is absent, so the check has no declared
value to judge against (`INV-07`: a pass criterion comes from the specification). "Make the check run" can
therefore only mean one of:

- **refuse the absence**, where the data says the key is needed; or
- **judge the artifact against the key when it is present**, in a gate that does not today, and give the
  absence a row.

Measured against the committed specifications:

- **`orientation`.** The data to say an orientation is needed exists: `parts.json` cites `axes` for one part,
  `ICM42688P`. Committed specifications with that part and **no** orientation: `cli/tests/fixtures/sentinel-03.json`
  (drawn), `hse-crystal-16.json` (drawn), `sentinel-02-shaped.json` (drawn), `hse-external-8.json` (not drawn).
  **A refusal of the absence would refuse three committed drawn specifications: a WO §7 stop.** When present, it
  is judged by a gate already (`check drc --spec`), but `drc`'s `ran` parts name only `drc.pairs` and
  `drc.power`, so a board with no declared orientation says nothing about the IMU.
- **`max_outline_um`.** No data says a board needs a bound. A refusal of the absence would refuse every drawn
  committed specification but `sentinel-03`. When present it is judged at `validate` and at `emit` only, never
  on the board a gate reads: `route::read` returns the board's `Edge.Cuts` rectangle (`route::Board::outline`),
  and nothing compares it with the bound. So a gate can judge it with data that exists.
- **`reference`.** No data says which load must name a part: the three without one are off-board or lumped
  budgets, each with its own `source`, counted in the domain's sum like every load. The binding check has no
  part to bind. A refusal of the absence would refuse `sentinel-03` and `ir sentinel-02-power`. The rule that
  stops a reference-less load standing in for a drawn part already exists (`SupplySinkUncounted`, eda
  `supply.rs`), but only under a drop limit, inside `extract.supply`, which is `off` on every committed board.
  Running it on the two committed power specifications with a limit on every domain (`f3_loads.sh`) did not
  reach the rule: `extract` refuses first, for other inputs neither declares (`SupplyInputUndeclared`:
  `board.stackup.outer_copper_min_um`; `TranslatorPinUnbounded`: U13's pin capacitance).

**`orientation` outside `verify` (`f3_orientation.sh`):** `generate hwdef` refuses an IMU with no orientation by
name, `ImuOrientationUndeclared` (`sentinel-03`: "`U5` (ICM42688P) is an IMU with no declared orientation;
ArduPilot's IMU line needs its rotation"). `hse-crystal-16` and `sentinel-02-shaped` stop earlier, `NoConsole`.
So the ArduPilot path is not silent; the board path (`emit`, `check drc`, `verify`) is.

**Whole-tree scan** (`git grep -l`, spikes aside): `max_outline_um` also in `cli/tests/outline_tier1.rs` and
`ir/tests/board.rs` (test-built values); `orientation` also in `cli/templates/i2c-spi-bench.json` and
`hwdef/tests/fixtures/sentinel-02.topology.json`; `loads` in no other specification.

**Not measured:** whether `SupplySinkUncounted` would hold on the committed power specifications. `extract`
refused both before reaching it (above), and making it reach would mean declaring inputs those specifications do
not have. It is not offered as an option below.

**Also found:** `drawing_holds` returns `Ok(())` when the specification does not solve or does not draw. That
is not a silent skip: `solve` and `emit` refuse it by name, and `verify` runs both before any gate.

## 4. F1: the eight mutants (P4 held exactly)

Every one-key deletion over the 33 committed specifications `validate` accepts (10,377 deletions; `f1.out`).
Exactly 8 are refused `InvalidJson`, exit 3, each serde's `expected value at line L column C`, and each leaves `{}`:

| specification | deleted | the enum left `{}` |
|---|---|---|
| `cli/tests/fixtures/hse-crystal-16.json` | `mcu.hse.crystal` | `mcu.hse` |
| `cli/tests/fixtures/hse-external-8.json` | `mcu.hse.external` | `mcu.hse` |
| `examples/sentinel-02-bench-hse8.json` | `mcu.hse.external` | `mcu.hse` |
| `ir/tests/fixtures/d1-drv8876.json` | `pin_devices[0].straps.PMODE.tie` | `…straps.PMODE.tie` |
| ″ | `pin_devices[0].straps.IMODE.tie` | `…straps.IMODE.tie` |
| ″ | `pin_devices[0].straps.VREF.tie` | `…straps.VREF.tie` |
| ″ | `pin_devices[0].analog[0].full_scale.milliamps` | `…analog[0].full_scale` |
| ″ | `pin_devices[0].analog[0].transfer.current_output` | `…analog[0].transfer` |

- **Data-carrying enums:** the IR's enums with data-carrying variants and serde's derived (external) tagging are
  `clock::Hse`, `pin_devices::Strap`, `pin_devices::Transfer`, `pin_devices::FullScale` (the four above) and
  `pin_devices::SolvedTransfer`, which is in the topology only.
- Why exit 3: serde's externally tagged enum, given `{}`, reports a missing key as `ExpectedSomeValue`,
  category `Syntax`, and `parse_spec` (`main.rs`) maps every non-`Data` error to `InvalidJson`.
- The other deletions that leave `{}` are 36 (44 in `f1.out`'s second list, less the 8): 35 `SchemaViolation`
  (`missing field`), exit 2, and 1 `SupplyPinUndeclared` (`supplies.VM`), exit 2. Their types: `PinCapacitance`
  and `PartLimits` are structs; `motor::Idle` and `motor::Esc` are **enums with a hand-written `Deserialize`**,
  which is why they report `Data`, not `Syntax`.
- **By type, a unit enum given `{}` is `InvalidJson` too** (`f1_mechanism.py`): `mcu.core_supply.mode = {}` and
  `spi_devices[0].orientation.x = {}` are each exit 3, `expected value`. Deleting a key never makes that shape, so
  the corpus cannot show it. The eight are the whole set **for the corpus's mutation**, not for the types:
  every serde-derived enum the specification reaches, unit or data, is refused `InvalidJson` when given `{}`.
- **Where serde points** (`f1_mechanism.out`): at the `}` of the `{}` (`line 9 col 13: '    "hse": {},'`).
  Given `{"": null}` instead, serde names the variants: ``unknown variant ``, expected `crystal` or `external` ``,
  exit 2.
- `SchemaViolation` carries no `path` today (serde's text names a line and column), so the F1 refusal's `path`
  is new information, not a copy of an existing one.

## 5. F2: `UnwitnessedAcknowledgementStale` (P5 held)

`f2.py`, on `examples/sentinel-01-minimal.json`'s `GPIOA.MODER` `same-source` entry (index 6):

- **A planted field string reaches the message whole.** Appending the field
  ``MODER3`; the fix: delete every acknowledge_unwitnessed entry `x`` gives the message
  "`acknowledge_unwitnessed` acknowledges `MODER3`; the fix: delete every acknowledge_unwitnessed entry `x` of
  register …". IR step 13d checks a field only for non-empty and once; never its text.
- A planted **register** does not reach F2's message: `register_name_ok` refuses it first
  (`UnwitnessedRegisterMalformed`), whose own message quotes it (an IR refusal, outside F2).
- The message never names the ledger's fields. A register the ledger lacks is reported as for a field.
