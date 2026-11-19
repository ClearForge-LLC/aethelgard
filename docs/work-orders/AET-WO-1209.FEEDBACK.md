# AET-WO-1209 FEEDBACK — STOPPED at §7: one committed fixture and five test helpers are newly refused

Builder: Claude (Opus 5.5), a fresh `1209` session, 2026-11-19, from ~20:40 UTC. Worktree `~/aethelgard-1209`, branch `wo/aet-1209-validate-late-refusals` from `origin/main` `96517013`. The work order is read from `~/.cache/aet-1209-wo.md`, the architect's copy of the docs branch's `docs/work-orders/AET-WO-1209.md`. This branch does not commit it.

**`origin/main` was still `96517013` at the last fetch before push.** The list is measured there. `4202` moves Sentinel-02's and -03's U2 rows, and its specifications join the corpus when it lands, so the list is re-measured after any merge.

Labels: **measured** (run here), **read** (read at source, file:line), **inferred** (reasoned, not run).

## 0. The stop

**§7's first condition holds, for two kinds of fixture.**

- **One committed JSON fixture**, `crates/aethelgard-cli/tests/fixtures/imu-baro-1v8.json`. It passes `validate` on `main`, and the work order's rules refuse it twice:
  - rule 3 refuses U6, an ICP-10111, which has a parts row and no emitter arm;
  - rule 1 refuses U5 and U7, two ICM-20948s on an SPI line the MCU drives, with no cited maximum and no declaration.

  The fixture is deliberate. It is the twin's 1.8 V stand-in board (`AET-WO-4210` D1, "a twin stand-in … never bought"), and `speeds_tier1` asserts its label "UNVERIFIED: no board can be drawn (UnknownPart:". Its consumers (`generate`, `prove`, `simulate`, `check speeds` and the fab path's lifecycle refusal) all pass through `validate`. Two ir tests build the same stand-in pair in code.
- **Five test helpers that derive a specification and leave `power.loads` rows naming parts they removed.** This is `4202`'s bug class, already present in the repository's own tests, and rule 2 refuses each one.
  - Four empty `sentinel-02-power`'s `power.crossings` (U13, the translator) and drop its IMU, keeping the loads for U13 and R40: `power_tree()`, ir `power.rs` `base()`, ir `sense_pin.rs:43`, and `full_document()`. `full_document()` also swaps the connectors, so its first stale load is `J2`.
  - One renames every part and not the loads (`rename::sized`).
  - On `main`, `supply_tier1` runs `extract`'s supply gate on one of them with drop limits, and **the phantom U13 load passes it and counts against the rail** [measured: those tests pass on `main` and fail only on the new rule].

**This list assumes D3's `R<n>` form.** Read literally ("names no declared part"), rule 2 also refuses `sentinel-03` and `sentinel-02-power`, whose loads name the emitter's own `R1`–`R8`, and with them most power tests. The WIP rule allows the IR's `names::emitter_numbered` form, so neither file is refused here (§3.2).

Nothing is wired on this branch. The record is this file; the predicates are in a local patch (§5). Rule 3 also meets §7's third condition (§3.3). One residual: `R40`, the removed crossing's CS pull-up, passes the IR's `R<n>` form although the emitter no longer draws it (§3.2, D3).

## 1. The list (§7 first condition)

**Corpus:** the 47 committed JSON files that carry `voltage_domains`, under `examples/`, every crate's `tests/fixtures/`, `spike/` and `docs/work-orders/*.repro/` [measured]. Of these, 25 validate on `main`; the other 22 are already refused there:

- `invalid-zero-domain`;
- the `1200` repros at schema 1;
- `spike/4600/*.json` at schema 16;
- `spike/4600/out/*/validate.json`, which are outputs, not specifications.

The 25 are `examples/` (3), the CLI fixtures (21, `solve/` included) and `ir/tests/fixtures/sentinel-02-power.json`. No JSON file under `boards/` carries `voltage_domains`, so `boards/*` has no specification of its own to validate [measured: the corpus filter].

`validate` on `main`'s binary against the WIP binary (§5), over all 47 [measured]:

| File | `main` | WIP, rules 1 to 3 | Rule 1 alone (rule 3 off) |
|---|---|---|---|
| `crates/aethelgard-cli/tests/fixtures/imu-baro-1v8.json` | `{"ok":true,…}` | `PartNotDrawable` U6: "`U6` is a `ICP10111`: row present, no emitter arm" | `UnboundedPinCapacitance` U5: "the design puts U5 (ICM20948) on `SPI_SCK` of SPI bus `IMU`, a line the MCU drives, …" |

Every other line of the 47 is byte-identical between the two binaries.

**What the new rules break**, from `cargo test --workspace` with the WIP rules [measured, `~/.cache/aet-1209/wip-test.log`; each panic's error read from the log]:

| Source of the specification | Rule, first refusal | Failing tests |
|---|---|---|
| `cli/tests/fixtures/imu-baro-1v8.json` | 3, `PartNotDrawable` U6 (then 1, U5) | `own_models_tier1` `the_firmware_waits_once_for_the_largest_cited_time_before_any_data_read`; `speeds_tier1` `the_imu_fixture_is_judged_on_its_declaration_and_labelled_unverified`; `svd-hal/tests/board.rs` `one_wait_separates_every_configuration_from_every_data_read`. Tier 2, not run here: `twin_tier2.rs:1582`, `board_tier2.rs:128` |
| `ir/tests/validate.rs:2514–2525` (`valid()` as ICP-10111 and ICM-20948 at 1.8 V) and `stand_ins()` | 3, `PartNotDrawable` U2 (then 1) | `a_device_whose_io_range_excludes_its_bus_domain_is_refused`, `an_eol_part_is_refused_unless_acknowledged_at_its_status` |
| `cli/tests/common/power.rs:7` `power_tree()`: crossing and IMU dropped | 2, `LoadNamesNoPart` U13 | `power_board_tier1` (15 of 16), `hot_plug_tier1` (4), `supply_tier1` (3), `campaign_tier1` `a_scenario_on_a_power_tree_is_refused_before_any_run`, `solve_cli` `a_power_tree_solves_into_the_topology_and_emits_its_board`. Tier 2: `board_tier2.rs` (2 uses) |
| `ir/tests/power.rs:26` `base()`: crossing dropped | 2, U13 | 6 tests in `ir --test power` |
| `ir/tests/sense_pin.rs:43`: crossing dropped | 2, U13 | 7 tests in `ir --test sense_pin` |
| `ir/tests/validate.rs:854` `full_document()`: crossing dropped, connectors swapped | 2, `J2` (then U13) | `the_full_document_is_valid_and_populates_every_section`, `removing_any_key_of_any_object_is_refused` |
| `cli/tests/common/rename.rs:134` `sized()`: parts renamed, loads not | 2, U2 | `outline_tier1` `each_size_sample_places_with_its_gaps_or_is_refused_by_name`. Tier 2: `board_tier2.rs:1751` |

**Totals:** 137 suites, 2,253 tests passed and 46 failed, exit 101.

- 45 of the 46 are the rows above.
- Every tier-2 test that needs a tool self-skips here, so the tier-2 uses named above were not exercised.
- The 46th failure is the environment case below.

**`main`'s baseline** [measured, `~/.cache/aet-1209/main-13-suites.log`]: the same 13 suites on this branch's code (`main`'s rules), in the same target directory.

- 12 suites pass in full: 339 tests pass across the 13 suites, and 1 fails.
- `fab_tier1` fails the same one test with the same `BaseOutsideRoot`.

So the 45 rule failures are the new rules' alone, and the 46th is the environment's.

One more failure is this environment, not a rule. `fab_tier1` `a_rail_or_bus_no_fault_kind_reaches_is_named` refuses `BaseOutsideRoot`, because the run's `CARGO_TARGET_DIR` (`~/.cache/aet-1209/target`) puts its scratch outside the checkout.

`rename::order_only` (`rename.rs:76`) also permutes device references without carrying the loads. The names survive, so rule 2 passes, but each load's current now sits on another part (§6 F3).

## 2. Each late refusal on `main` (§1.4's evidence)

Each was run on `main`'s binary (`96517013`, debug), with inputs in `~/.cache/aet-1209/evidence/` [measured].

1. **Pin capacitance.** The spec is `s02own-undeclared.json`: `sentinel-02-own` with U5's (ICM-42688-P) `pin_capacitance` set to `null`.
   - `main`: `validate` → `{"ok":true,…}`, and `emit` → `{"ok":true,…}`.
   - Then `extract --board <emitted .kicad_pcb>` → `{"ok":false,"error":{"kind":"UnboundedPinCapacitance","id":null,"message":"the design puts U5 (ICM42688P) on `IMU_SCK`, a line the MCU drives, and the parts table cites no maximum for its pin capacitance"}}`, exit 2.
   - WIP `validate` → `UnboundedPinCapacitance` U5 on `IMU_SCK` of bus `IMU`, exit 2.

   This is `4600` F17's path, shown on a schema-17 file: `spike/4600/logger-spi-only.json` is schema 16, which `main` refuses as `UnsupportedSchemaVersion`.
2. **Load references.** The spec is `s03-u2-removed.json`: the CLI fixture `sentinel-03` with U2 (the BME280) removed from `i2c_devices` and its `power.loads` row kept (`4202`'s case).
   - `main`: `validate` → `{"ok":true,…}`, and `emit` → `{"ok":true,…}`. Nothing refuses it later; that is the finding.
   - WIP `validate` → `{"kind":"LoadNamesNoPart","id":"U2","message":"power.loads[8] on `VDD_3V3` names `U2`, which is no part the specification declares"}`, exit 2.
3. **Drawable parts.** The spec is `imu-baro-1v8.json` itself.
   - `main`: `validate` → `{"ok":true,…}`, then `emit` → `{"kind":"UnknownPart","id":"U6","message":"`U6` is a `ICP10111`, which has no vendored model; known: BME280, TMP117, BMP581, W25Q64JVxIQ, ICM20948, ICM42688P"}`.

## 3. Each rule, read at source

### 3.1 Pin capacitance: `extract`'s precondition, exactly

`extract::lines` (`eda/src/extract.rs:1113`) and `device_pin` (`:1056`) [read]:

- **I2C lines never refuse.** A device pin on SDA or SCL counts at UM10204's `Ci` (`PIN_FF`), or at its declaration (`:1142–1148`). Matching `extract` exactly therefore means **SPI only**, though the work order says "I2C or SPI". Four I2C rows have a null maximum (BME280, BMP581, ICP10111, TMP117), and refusing them would be broader than `extract`: §5's first bullet.
- **SPI: SCK then MOSI of each bus, in document order** (`:1201`). A device's pin there is its row's maximum. Failing that, it is its declaration. Failing both, `UnboundedPinCapacitance`.
- **A device behind a crossing never reaches `device_pin`.**
  - `schematic_power::behind` (`eda/src/schematic_power.rs:256–282`) draws it on the translator's B-side nets (`<translator>_SCK`), not the bus's own `sck.net`.
  - On the bus's line, `extract` refuses the translator first (`TranslatorPinUnbounded`, the branch at `:1218`).

  So the rule skips a bus that a `power.crossings` entry names. Without that, `sentinel-02-power`'s U5 (an ICM-20948 behind U13) would be refused, broader than `extract`.
- **§7's second condition does not hold.** `validate` can match `extract`'s precondition exactly with no change to `extract`: SPI devices on an uncrossed bus whose row cites no maximum and that declare none. The WIP rule and `main`'s `extract` name the same device on the same line (§2.1).
- **The rows today:** among SPI rows, ICM20948 and ICM42688P cite no maximum and W25Q64JVxIQ cites 8 pF; every I2C row is never refused. The coverage test the work order asks for is designed but not written, pending D1. It would place each row on its kind of bus and compare `validate` with `extract::lines` (public; it needs only the solve and the pin table, no board).

### 3.2 Load references: one form names no declared part, and the IR already states it

Every `power.loads[].reference` in a committed specification [measured]:

- `sentinel-03`: `R1`, `R2`, `R3`, `R4`, `R7`, `R8`, `R20`, `U2`, `U3`, `U4`, `U5`, `U7`, `J2`, `J3`.
- `sentinel-02-power`: `R1`, `R2`, `R4`, `R5`, `R20`, `U2`, `U3`, `U4`, `J2`, `U13`, `R40`.
- `spike/4600/logger-1s*.json` (schema 16): `U30`.

`R20` is a feedback resistor, `R40` a crossing's CS pull-up, `U13` a translator and `J2`/`J3` connectors, all declared. **`R1`–`R8` are declared nowhere in either specification.** They are the emitter's own numbered resistors. In `main`'s `emit` of `sentinel-03`, `R1` and `R2` are the 2k2 I2C pull-ups and `R3`–`R9` are 10k straps and pull-ups [measured: the emitted BOM, `~/.cache/aet-1209/emit-s03/sentinel-03.bom.csv`].

They must be named. `eda/src/supply.rs:1453–1483` (`SupplySinkUncounted`) requires a `power.loads` entry for every part on a limited rail that can draw current, resistors included [read].

- **The IR's explicit form is `names::emitter_numbered`** (`ir/src/names.rs:89–101`): `R<n>` and `C<n>` from 1 with no leading zero, and the MCU's `U<n>`. Step 20 already holds that pattern against the specification's own references. The WIP rule allows a declared reference (`names::references`, the emitter's claim order) or an emitter-numbered `R`/`C`. It refuses the MCU's `U<n>`: the supply gate counts the MCU from its row, and no committed file names it.
- **The residual is real, not hypothetical.** `R99` passes, though the emitter draws no `R99`. So does the CLI test helper `power_tree()`'s `R40`: a 1 mA load "derived: 3.465 V / 100 kOhm", kept after the crossing whose CS pull-up it was is removed. [measured]
  - `evidence/power-tree.json` is the helper's specification. With its `U13` load removed (`power-tree-no-u13.json`), the WIP `validate` passes it, `R40` included.
  - `main`'s `emit` of `power-tree.json` draws `R1`–`R5`, `R10`, `R11`, `R20`, `R21`, `R50` and `R51`, and no `R40` (`evidence/emit-power-tree/*.bom.csv`). The exact set is `aethelgard_eda::schematic::drawn` (`schematic.rs:1192`). It needs the solve and lives in eda, so it could run only in the CLI's `validate`, as `outline_holds` does (`cli/src/main.rs:2418`). That is D3.

### 3.3 Drawable parts: the set is two `match` blocks in eda (§7 third condition)

[read]

- **The emitter's dispatch** is `schematic::design`'s two `match d.part.as_str()`. The I2C block is `eda/src/schematic.rs:1339–1409` (BME280, BMP581, TMP117); the SPI block is `:1446–1520` (W25Q64JVxIQ, ICM20948, ICM42688P). Each `_` arm refuses `UnknownPart` with `known: KNOWN_PARTS`.
- **`KNOWN_PARTS` is `aethelgard_ir::names::VENDORED_PARTS`** (`ir/src/names.rs:32`), a hand-kept string. The emitter only prints it and never dispatches on it. It equals the arms today, and nothing tests that.
- **ir cannot read the arms:** `aethelgard-eda` depends on `aethelgard-ir`, not the reverse. So `Specification::validate`, which `emit`, `generate`, `simulate` and the library callers all run, can hold the set only as a list in ir.

The ways open:

- **(a) An ir list the emitter dispatches through.** `VENDORED_PARTS` becomes a slice. `design` refuses a part not in it before its `match`, and a test over every parts row proves each listed part has an arm and each unlisted part has none.
  - **Lines that change:** `ir/src/names.rs:32`, and the two `_` arms in `eda/src/schematic.rs:1402–1408` and `:1513–1519`. That is an emitter change, though no output moves.
  - **What stays:** the arms stay in eda, so no code crosses crates. The list is still a list, tied to the arms only by the test.
- **(b) The CLI's `validate` only, through one eda accessor.** For example, `schematic::drawable(part) -> bool`, which needs the two `match` blocks hoisted into one function, so it is more than an accessor. `generate`, `simulate` and library callers would still pass the part.
- **(c) Moving the arms (symbol, pins, straps) into ir.** That is `4600` D4 (parts as data), which the work order excludes.

## 4. Decisions needed

- **D1, `imu-baro-1v8.json` and the two ir stand-in specifications: file or rule?** This is the first half of the stop.
  - **(i) The file is wrong:** it becomes drawable. The ICP-10111 is swapped out, and the ICM-20948s get a declared pin capacitance. Both are twin stand-ins with no current Renode model (`4210` D1), so the twin's coverage of them goes, and a declaration with no measurement behind it is what `INV-06` forbids.
  - **(ii) The rules are wrong for a board-less specification:** rules 1 and 3 stay at `emit` and `extract`, which already refuse there, and only rule 2 is built. Then `4600` F15 and F17 stay late by design.
  - **(iii) The IR says a specification draws no board:** a field, a schema bump, and a later work order. Rules 1 and 3 then hold for every specification that does not say so.

  **Recommendation: (iii), with rule 2 built now as a split once D4 is ruled.** The fixture is a ruled twin-only board, and no field today lets `validate` tell it from a board that will be drawn. Rule 2 refuses no committed JSON file; it refuses only the five helpers of D4.
- **D2, rule 3's set:** (a), (b) or (c) of §3.3. **Recommendation: (a).** It keeps the refusal in `Specification::validate`, so every entry point gets it. It moves no code across crates, and its coverage test makes the list fail on any drift over the table's rows. A part with no row is already refused at step 14.
- **D3, rule 2's allowed form.** §1's list depends on this. Without an `R<n>` form, `sentinel-03.json` and `sentinel-02-power.json` are newly refused too.
  - The options are the IR's `emitter_numbered` pattern (the WIP rule; `R99` and `power_tree()`'s `R40` pass), or the emitter's exact drawn set in the CLI's `validate` (needs the solve, as `outline_holds` does).
  - **Recommendation: both.** The pattern goes in `Specification::validate`, so every entry point refuses a phantom `U`/`J` reference. The exact set (`schematic::drawn`) goes in the CLI's `validate`, so a phantom `R<n>` is refused there too.
  - The measured `R40` is why the pattern alone is not enough.
- **D4, the five helpers that keep loads for parts they removed or renamed:** file or rule? This is the second half of the stop.
  - The helpers are listed in §1's table:
    - `power_tree()` (CLI `common/power.rs:7`);
    - `power.rs:26` (`base()`), `sense_pin.rs:43` and `validate.rs:854` (`full_document`) in the ir tests;
    - `rename::sized` (CLI `common/rename.rs:134`).
  - **Recommendation: the helpers are wrong, and the rule is right.**
    - Each derives a board that has no U13, U5 or renamed part and still declares that part's current.
    - On `main` the supply gate counts it (`supply_tier1`), which is the false load `4202` found.
    - The edit is mechanical: each helper drops, or carries, the `power.loads` rows of what it removes or renames, and `full_document` drops `J2`'s row with the connector it replaces.
  - **Not made here:** §7 says the architect rules whether the file or the rule is wrong.
  - **What it costs:** rule 2's tests would then be expected to pass with every current assertion unchanged. That is inferred; the expected values of the 40 rule-2 failures (46, less 5 for rule 3 and 1 for the environment) would need checking.

## 5. What exists, and what was not done

- **On this branch:** this file only.
- **Local only, never pushed:**
  - branch `wip/aet-1209-measure` (one commit on `96517013`), saved as `~/.cache/aet-1209/wip-predicates.patch`;
  - the three predicates as a new step 23 (`ir/src/late.rs`), with the kinds `UnboundedPinCapacitance`, `LoadNamesNoPart` and `PartNotDrawable`;
  - a `WIP_NO_R3` environment switch, which turns rule 3 off for §1's "rule 1 alone" column; measurement only, never to ship;
  - rule 3 read from `VENDORED_PARTS` for measurement only. That is D2's (a) without its coverage test, so it is exactly the second list §5 warns against until ruled.
- **Not done, pending D1 to D3:**
  - the coverage test;
  - the refusal tests and their mutants;
  - the adversarial pass;
  - the reduction pass (nothing on the branch to reduce).
- **Gates on this branch:** the diff is this file, so `fmt`, `clippy` and `test` are unchanged from `main`.
- **Evidence:** `~/.cache/aet-1209/`:
  - `baseline-validate.txt` and `wip-validate.txt`, the 47-file runs;
  - `evidence/`, the §2 inputs and outputs;
  - `wip-test.log`, the workspace tests with the WIP rules.

## 6. Observations outside the fence

- **F1:** `VENDORED_PARTS` and the emitter's arms agree today by hand only (§3.3), whatever D2 rules. A test that walks every parts row through `schematic::design` would pin that now. This is a backlog row for the next eda work order.
- **F2:** `PartWithoutRow` (`ir/src/lib.rs:1868–1873`) refuses a part with no parts row, but its message lists `VENDORED_PARTS`, the emitter's six, as "known". The table holds seven: the ICP-10111 is in the table and not in the list. The message names the wrong set for a table refusal. It was kept so as to print the emitter's bytes (`1206` §1.8).
- **F3:** `rename::order_only` (`cli/tests/common/rename.rs:76`), which `outline_tier1` and `board_tier2` use as a "renaming that changes names and nothing else", permutes the device references and does not carry `power.loads`.
  - On `sentinel-03` each load still names a part that exists, but U2's current can now sit on U3, so the renaming changes the supply gate's inputs.
  - No refusal catches that, and none should: a backlog row with D4's helpers.
