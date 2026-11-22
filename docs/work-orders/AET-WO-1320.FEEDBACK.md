# AET-WO-1320 — FEEDBACK: the arm table's `check()` holds five more facts

**Branch:** `wo/aet-1320-arms-check` from `main` `f46c3c42`. **Builder:** Claude (Opus 5.5 orchestrating), with one fresh subagent for the adversarial read. Its report was checked by planting its mutants. Written 2026-11-22.

**Status: READY** (see §6 for CI). D1 and D2 were accepted at 10:36Z; D3, a fold the reduction pass made, is for your ruling. **The answer, in five lines:**
- **Five new lines in `check()`** (`crates/aethelgard-eda/src/schematic_arms.rs`). Each refuses a bad table when the crate compiles:
  1. two unstated passives on one pin;
  2. an I2C row with no address strap;
  3. a bus row whose decoupler count is not its part's;
  4. one own-net name on two pins of a row;
  5. a pin listed twice in a row.
- **Each is refused at compile time by a test over a bad table.** The test evaluates `check()` in a `const` item (`at_compile_time!`), as the crate does over `ARMS`.
- **Every committed specification's bytes are identical, and so are 1319's harnesses**, branch against base: 34 specifications and 280 files, d4diff 277 cases, d1319 84.
- **Mutants: 10 of 10 killed.** K1–K5 delete each check, and each is killed by its own test. W1–W5 are the reviewer's weakening mutants; all five survived the first tests and are killed now.
- **No row, drawing or emitted byte moved.** `TIER2_MIN` is unchanged. No file outside the fence changed.

## 0. How it was made

- **Predictions first:** `spike/1320/PREDICTIONS.md`, committed at `bb3b2661` before any run. Results beside them: `spike/1320/RESULTS.md`.
- **Bus:** at the start I posted the decoupler-count form and my reading of "per pin" (`~/.cache/aet-bus/architect.md`, "from 1320"). **Both were accepted at 10:36Z** (`~/.cache/aet-bus/1320.md`) and are built as posted (§3, D1 and D2).
- **Kit:** the null sweep is `1319`'s `spike/1319/harness/run.sh` and `compare.sh`, unchanged. Base is a `git archive` of `f46c3c42`; the branch is an rsync copy. Each tree has its own target dir, and each log names its own `Compiling aethelgard-eda (<tree>)`. The mutants are `spike/1320/mutants.sh`.
- **Evidence:** `~/.cache/aet-1320/`.

## 1. What was built

- **`check()`, five lines,** each placed after the `1319` refusals a bad table could also trip, so no earlier refusal is stolen. Every `1319` bad-table test passed unmodified at the build commit `a10b8e72`.
  1. **Pin rows:** two `value: None` passives that share a pin end (`share_a_pin`, all four pairs of ends) are refused. Two on different pins are allowed (D2).
  2. **I2C rows:** `addresses == 0` is refused, beside the existing `addresses > 1`.
  3. **Bus rows:** the decoupler count must be the one `DECOUPLERS` states for the part. A bus part it does not name is refused (D1).
  4. **Bus rows, pairwise:** two pins whose roles draw an own net of one name are refused. The roles are `Strap(n)`, `Own(n)` and `Addr(Some(n), _)` (`own_name`).
  5. **Bus rows, pairwise:** two entries for one pin number are refused.
- **Tests:** five new tests, each over bad one-row tables evaluated in a `const` item:
  - `two_unstated_passives_on_one_pin_are_refused`
  - `an_i2c_row_with_no_address_strap_is_refused`
  - `a_decoupler_dropped_or_added_is_refused`
  - `one_own_net_name_on_two_pins_is_refused`
  - `a_pin_listed_twice_is_refused`

  Each bus-row bad table is a real row with one change. The pin-device tables are short passive lists over DRV8876's or DRV8833's row: CVM2 twice, or CVM2 beside one other passive. Each test also carries a control the check must accept where a weaker check would pass by luck:
  - unstated passives on two different pins;
  - an unstated passive beside a stated one on its pin, as DRV8876's own row has.

## 2. Measurement and acceptance

- **Baseline at `f46c3c42`:** `run.sh` ran unchanged (B2). The `dump` shim's anchor is still in place, and both harnesses compile.
- **Branch against base:**
  - **at the build commit `a10b8e72`:** `identical: 280 files`; `d4diff identical: 277 cases, 926 files`; `d1319 identical: 84 cases, 328 files`.
  - **at the final head `f3cc1b2f`** (`compare-final.txt`): the same three lines, identical.
- **Each of the five is refused at compile time** by its test (§1), and each test fails when its check is deleted (§4).
- **No §7 stop fired.** All five are plain `const fn` on stable, written as `1319`'s are (`while` loops and `same()`). No emitted byte moved.

## 3. Decisions

- **D1, where the decoupler count lives** (posted at the start; **accepted 10:36Z**).
  - No other fact of a row predicts it. Rail pins don't: ICM42688P has two rail pins and three decouplers, BME280 three and one.
  - An `Arm` field would edit all eight rows, which the WO's "NOT any change to a row" excludes.
  - So the count is stated once more, in `const DECOUPLERS: &[(&str, usize)]` beside `check()`, and labelled `belt-and-suspenders:` at the site.
  - **It states each bus part's decoupler count a second time, by design, so a row that drops or adds a decoupler fails to compile.**
  - A bus part it does not name is refused, so a new bus row must state its count there.
  - The reverse (an entry for a part with no row) is not checked (F2).
- **D2, "one unstated passive per pin"** (**accepted 10:36Z**: the WO's wording).
  - The WO's title says "per pin"; `1211f` F2's proposed row says "per pin row". I built the title's reading: two unstated passives sharing any pin end are refused, whatever their other ends.
  - This is a superset of the hazard. The hazard is two from one supply pin to GND, which one `bulk` would answer twice.
  - Two unstated passives on different pins stay allowed. The bulk answers one, and the other is refused at run time by name as `PassiveValueUnstated`, as today.
  - The per-row reading would refuse that pair too. Today's table passes all three readings.

- **D3, a fold the reduction pass made (not asked by the WO; for your ruling).**
  - **Removed:** `1319`'s line in `check()`'s `Own` branch, "an own net named twice in one row". The new own-name line refuses every table it did, and more.
  - **Moved:** `1319`'s `an_own_net_capacitor_names_its_own_pin` now expects "one own-net name on two pins of a row" for its `Own`/`Own` table.
  - **Revert:** three lines back in, and one expected message.
  - The WO asked for five new lines; this also deletes an old one, so it is named here and in the READY note.

## 4. Adversarial pass (§5), targeted

- **A check that refuses a row in today's table: none.** The `const` assertion over `ARMS` compiles with all five checks.
  - The fresh reviewer also read the logic for faults today's data hides: off-by-one starts, the first-match lookup, `share_a_pin`'s pairs, Ground ends. It found none.
- **A check whose test passes with the check deleted: none.** K1–K5 each delete one check (its `return Err` becomes `()`) in a copy. Each is killed by that check's own test.
  - At the build commit, each was killed by its own test only.
  - Since the reduction (D3), K4 is also killed by `1319`'s `an_own_net_capacitor_names_its_own_pin`, whose `Own`/`Own` table the new line now refuses.
- **The reviewer's weakening mutants** (it ran 20; five survived the first tests, each CONFIRMED by planting). All five are killed now (`mutants.txt`):
  - **W1, W2:** `share_a_pin` without the `a.two`–`b.one`, or the `a.two`–`b.two`, term. The first tests only met at `a.one`. Added: CVCP then CVM2, which meet at the first's second end; and CVCP beside a copy of CVCP moved to CPH, which meet only at both second ends. A `[CVCP, CVCP]` table did not kill W2: its first ends meet too, a mistake caught by the mutant.
  - **W3:** the pairwise loop compares adjacent pins only. Every bad table had adjacent duplicates. Added: a pin and a name, each two or more rows apart.
  - **W4:** `own_name` without `Addr(Some(n))`. Added: BME280's SDO address strap and a strap named SDO.
  - **W5:** the check without `b.value.is_none()` would over-refuse. Added: DRV8876's CVM2 then CVM1, unstated beside stated on VM, accepted.
- **Placement** (reviewer, and my own run): no `1319` refusal is stolen. Every existing test passed unmodified at `a10b8e72`, and no other crate's test names these messages.

## 5. Findings, not fixed here

- **F1, `spike/1319/harness/mutants.sh` names `crates/aethelgard-eda/src/schematic_pin_device.rs`** (line 10). `1319`'s Q1 ruling renamed it `schematic_arms.rs`, so that script cannot plant M1–M3 at `main`.
  - `run.sh` and `compare.sh` are unaffected.
  - `spike/1319/` is outside this fence. One path, for whoever next runs it.
- **F2, `DECOUPLERS` has no reverse check.** An entry for a part with no row goes unnoticed.
  - Harmless: an extra entry refuses nothing and draws nothing.
  - A line of `check()` would hold it if wanted. By the scope test it is backlog.
- **F3, `1319` FEEDBACK §5's hazard table** says `check()` refuses "an own net named twice" in its own line. Since D3 the same table is refused by the general own-name line. The record stands; the message moved.

## 6. Gates, fence, reduction pass

- **Gates:** `cargo fmt --all -- --check` passes, and `cargo clippy --workspace --all-targets -- -D warnings` passes. `cargo test --workspace` passes: 159 `test result: ok` lines, doc-tests included.
  - The gates ran at `f3cc1b2f`. The later commits are docs only: this record, and `main` `b1fbc851` merged in. `git diff f3cc1b2f HEAD -- crates Cargo.toml Cargo.lock ci .github` is empty.
  - The three tests that read `docs/` (`readme_status`, `strict_fence`, `board_names`) were re-run at the merged head and pass.
  - A first run with the target dir under `~/.cache` failed one CLI test, `fab_tier1::a_rail_or_bus_no_fault_kind_reaches_is_named`, with `BaseOutsideRoot`. The test writes its scratch inside the target dir, and that dir was outside the repo.
  - That failure was environmental. Re-run with the target dir in the repository, as CI builds.
- **CI:** see the PR's checks; READY is posted only on green. `TIER2_MIN` is unchanged.
- **Fence:** only `crates/aethelgard-eda/src/schematic_arms.rs` (`check()`, its helpers, its tests), `spike/1320/` and this file.
- **Reduction pass (diff): −5 net · 2 duplicates removed · 0 dead paths removed · 1 protected kept · 1 finding (F3).**
  - **Removed:** `1319`'s `Own` branch line "an own net named twice in one row" (D3, for your ruling).
  - **Removed:** the `seen` set in `each_bus_rows_pins_are_its_symbols`. A pin listed twice is now a compile error over `ARMS`.
  - **Protected:** `DECOUPLERS`, labelled `belt-and-suspenders:` (D1).
  - **Proof:** after the removals, the arm tests stay green and K1–K5 and W1–W5 are all still killed.
- **Line count:** +282 in `crates/` (code +76, tests +206), against the +80 to +200 predicted (R3, missed). Each bad table is written out row by row, one entry per line.
