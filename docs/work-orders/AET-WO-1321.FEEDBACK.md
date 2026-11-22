# AET-WO-1321 — FEEDBACK: `DECOUPLERS` checked both ways, the bulk drawn by `two_pin`, two stale names

**Branch:** `wo/aet-1321-eda-backlog` from `main` `a41a8a8d`. **Builder:** Claude (Opus 5.5 orchestrating), with one fresh subagent for the targeted adversarial read. Its finding was checked by planting its mutant. Written 2026-11-22.

**Status: READY** (see §7 for CI). D1 was posted at the start and built as posted; **it was accepted at 14:18Z**, and so was M2 as reported (F1). **The answer:**
- **(1)** `check()` takes the decoupler table and refuses an entry with no bus row. `a_decoupler_entry_with_no_bus_row_is_refused` refuses three bad tables at compile time.
- **(2)** `two_pin` takes the BOM note, and `declared_bulk` calls it.
- **(4)** `spike/1319/harness/mutants.sh` names `schematic_arms.rs`. Re-run: M1 and M3 are killed by bytes; M2 now fails to compile (§2).
- **(5)** The moved message is stated in §5.
- **(3) is not built:** `1217` has not merged (§3).
- **Every committed specification's bytes are identical, and so are 1319's harnesses**, branch against base: 34 specifications and 280 files, d4diff 277 cases, d1319 84. The declared bulk, which the sweep never draws, is identical too (§2).
- No row, drawing or emitted byte moved. `TIER2_MIN` is unchanged. No file outside the fence changed.

## 0. How it was made

- **Predictions first:** `spike/1321/PREDICTIONS.md`, committed at `1efb48c2` before any run. Results beside them: `spike/1321/RESULTS.md`.
- **Bus:** D1 (§4) was posted at the start (`~/.cache/aet-bus/architect.md`, "from 1321", 13:30Z). **D1 and M2 were accepted at 14:18Z** (`~/.cache/aet-bus/1321.md`).
- **Kit:**
  - the null sweep is `1319`'s `run.sh` and `compare.sh`, unchanged;
  - `spike/1321/bulk.sh` draws the declared bulk (§2);
  - `spike/1321/mutants.sh` holds R1–R3 and N1.
  - Base and branch are each a `git archive`, each built in its own target dir, and each log names its own `Compiling aethelgard-eda (<tree>)`.
- **Evidence:** `~/.cache/aet-1321/`.

## 1. What was built

- **(1), `crates/aethelgard-eda/src/schematic_arms.rs`:**
  - `check(rows, counts)`. The forward count check (`decouplers_stated`) reads the table it is given.
  - A new last line refuses an entry for a part with no bus row in `rows` (`has_bus_row`): "a DECOUPLERS entry with no bus row". A pin device's row is not a bus row.
  - The `const` assertion is `check(ARMS, DECOUPLERS)`.
  - Tests:
    - `refused()` passes `DECOUPLERS`. The reverse line runs after every row's, so it takes no row's refusal.
    - `at_compile_time!` and the SPI `Ok` control pass the one row's own entry (`entry()`, a `core::slice::from_ref` of it, or `&[]`).
    - So every existing test keeps its expected result.
  - The new test has an extra entry:
    - first: `DRV8876.part`, a pin row's part;
    - in the middle: BMP582;
    - last: TMP116.
  - Its control is two bus rows with their entries in the other order, accepted.
  - One more assert holds the forward count to the table given: BME280 against `("BME280", 2)` is refused. Without it, a check that read the global `DECOUPLERS` would pass every test (R4).
- **(2), `schematic_power.rs` and `schematic_arms.rs`:**
  - `two_pin` is `pub(super)` and takes `note: Option<String>`.
  - `capacitor`, `resistor` and the buck's inductor pass `None`, as it was fixed before.
  - `declared_bulk` calls `two_pin` with its value and note in place of its own `Part` push. The `Some(row)` in its `let … else` always matched (the `match` above returns on `None`), so it went with the push.
- **(4), `spike/1319/harness/mutants.sh`:** line 10, the file name only.

## 2. Measurement and acceptance

- **Branch at `d514f7a0` against base `a41a8a8d`:**
  - `identical: 280 files` (34 specifications: exit codes, printed JSON, emitted files);
  - `d4diff identical: 277 cases, 926 files`;
  - `d1319 identical: 84 cases, 331 files`.
- **The declared bulk (`bulk.sh`): identical.**
  - The sweep's only specification with a `bulk`, `d1-drv8876.json`, declares `board.drawn: false`, so `emit` refuses it (`DrawsNoBoard`). **Without this check no byte measurement draws `declared_bulk`.** Predicted otherwise (B3, missed).
  - `bulk.sh` draws it on a drawn board, as `pin_devices_drawn_tier1.rs` does, with the ceramic bulk and with that test's electrolytic: `ceramic identical: emit exit 0, 3 files, the note in 1`, and the same for the electrolytic.
  - It can fail: N1 (no note) differs in both.
- **`1319`'s mutants, re-run with the path corrected:**
  - **M1** (WP and HOLD swapped): killed by all three, the sweep (23 specifications), d4diff (73 cases) and d1319 (6).
  - **M3** (CVCP and CVINT reordered): killed by d1319 (32 cases).
  - **M2** (BMP581's second decoupler dropped) **no longer builds.** `1320`'s count check refuses it (`error[E0080]: … a bus row whose decoupler count is not its part's`, run exit 101). The script prints `M2 KILLED` because `compare.sh` fails over missing output, not over bytes. Predicted (M). It is now a compile-time refusal, and the harness's bytes do not see it. `mutants.sh` is fenced to its file name, so M2 was left as written (F1).
- **The reverse check is refused at compile time** by its test, which fails when the line is deleted (R1).

## 3. Item (3), not built

`1217` had not merged when this was written: `origin/main` was still `a41a8a8d`, this branch's base, at the last fetch before READY, so there was nothing to merge in. So `PinDeviceNotDrawable`'s guard text, which says an undeclared IPROPI is the only case, is unchanged. It is for whoever builds after `1217`, against its ruling (Q1, the GND tie).

## 4. Decisions

- **D1, the form of the reverse check** (posted at the start; built as posted; **accepted 14:18Z**).
  - A line over the global `DECOUPLERS` inside `check(rows)` would refuse every one-row bad table, and `1320`'s `Ok` controls with them. Those tables name one part, and the table names six.
  - So the decoupler table is `check`'s second argument, held both ways against `rows`.
  - The production call is the one `const` assertion. The tests give a one-row table its own entry.

## 5. Adversarial pass (§5), targeted

- **A fold that changes a drawn byte, a note or a reference order: none.**
  - Bytes: identical in the sweep, both harnesses and `bulk.sh` (§2).
  - The reviewer read it field by field:
    - `lib_id`, reference, value text, footprint, conns 1 then 2, the note, and owner, offboard and seat are all the same;
    - `two_pin`'s `row()` does the same lookup on the same table, so it cannot fail where the earlier one passed;
    - the refusals come in the same order.
- **A reverse check that refuses today's table: none.** The `const` assertion compiles. All six named parts have I2C or SPI rows, spelled as the rows spell them.
- **One finding, CONFIRMED and fixed:** the first bad entry named `"DRV8876"`, but the pin row's part is `"DRV8876PWPR"`.
  - The entry had no row of any kind, so the test never exercised the pin-row exclusion.
  - Mutant R2 (pin rows count as bus rows) survived, found both by `mutants.sh` and by the reviewer.
  - Fixed at `d514f7a0` with `DRV8876.part`.
- **A second gap, from the close-out review:** no test gave the forward check a table other than `DECOUPLERS`, so R4 (the forward count reads `DECOUPLERS`) would have survived. The assert was added at `0bad145d`.
- **Mutants at the head `0bad145d`:** R1–R4 and N1 are all killed (`spike/1321/mutants.sh`).
- **`1320`'s kit, re-run unedited at the head,** because `refused()` and `at_compile_time!` changed: K1–K5 and W1–W5 are all still killed, each by its own test.
- **The message `1319`'s hazard table names, which has moved (`1320` F3, item 5).**
  - `1319` FEEDBACK §5 says `check()` refuses "an own net named twice" (`an_own_net_capacitor_names_its_own_pin`).
  - Since `1320` D3 that line is gone. Its `Own`/`Own` table is refused by **"one own-net name on two pins of a row"**, the general own-name line, and the test expects that message.
  - The record stands as written; this is the correction. No other file in `spike/1319/harness/` names the old message.

## 6. Findings, not fixed here

- **F1, `1319`'s M2 is killed at compile time, not by the harness's bytes** (§2). **Accepted as reported at 14:18Z.** Being caught earlier is stronger, and the harness stays as it is past the file name. A §7.1 row will ask for a byte-level replacement mutant if anyone wants the harness to measure that case again.
- **F2, a `DECOUPLERS` entry stated twice for one part** is not refused. The forward lookup takes the first match, so a second entry with another count is shadowed. The work order asks for an entry with no row, not this. One more line of `check()` would hold it; by the scope test, backlog.

## 7. Gates, fence, reduction pass

- **Gates at `1fc5ecd6`** (the code is `d514f7a0`'s, as measured in §2):
  - `cargo fmt --all -- --check`: clean;
  - `cargo clippy --workspace --all-targets -- -D warnings`: clean;
  - `cargo test --workspace` (target in the repository, as CI builds it; `TMPDIR` unset): **2760 passed, 0 failed**, 160 `test result: ok` lines.
- **At the head `0bad145d`:** the one later code change is the R4 assert. It is inside `#[cfg(test)]` (`git diff -U0 d514f7a0 0bad145d -- crates` touches only line 1888, in `mod tests` from line 1393), so no emitted byte can move. fmt and clippy are clean, and `cargo test -p aethelgard-eda` gives 227 passed, 0 failed. The tests that read `docs/` (`readme_status`, `strict_fence`, `board_names`, `workspace_shape`) pass with this file in place.
- **CI:** see the PR's checks. READY is posted only on green. `TIER2_MIN` is unchanged.

- **Fence:** `crates/aethelgard-eda/src/schematic_arms.rs` and `schematic_power.rs`, `spike/1319/harness/mutants.sh` (line 10), `spike/1321/` and this file.
- **Reduction pass (diff): +68 net in `crates/` (+103 −35) · 1 duplicate removed · 1 dead path removed · 1 protected kept · 2 findings above.**
  - **Removed:** `declared_bulk`'s copy of `two_pin`'s `Part` push (the work order's item 2), and its `Some(row)`, which always matched.
  - **Protected:** `DECOUPLERS`, labelled `belt-and-suspenders:`.
  - **Proof:** after the fold, the bytes are identical and N1 is killed.
  - Run inline over the diff, not by a separate subagent. The adversarial reviewer had already read the whole diff.
