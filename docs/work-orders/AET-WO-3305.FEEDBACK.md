# AET-WO-3305 — FEEDBACK (built to the architect's ruling of 23:10Z on D1–D3)

**Branch and inputs.**
- Branch `wo/aet-3305-witness-elsewhere`, from `main` at `35d23eae`.
- The work order is read from `~/.cache/aet-3305-wo.md`. The ruling is in `~/.cache/aet-bus/3305.md` (23:10Z). The grounds are `2304b`'s F-MASK (#254).

Each statement is labelled:
- **measured:** run on Furnace in the pinned tools image, under `capped-run`, nice 10;
- **read at source:** file:line;
- **inferred.**

## 0. Status

- **`main` merged:** `52a2d62e` (3303b) cleanly, then `03d89554` (4202), then `3341dc6b` (1911, 2304b, 1314b), then `ff31328d` (documents only: `docs/roadmap.md` and a bench note; `readme_status`, the one test that reads the roadmap, passes on the merged tree).
  - 4202's conflicts: its exact `RunFor` in the pinned monitor-script tests (§4.5), each keeping the step hook, and one `TIER2_MIN` line.
  - 2304b's: the trace number (16 on top of its 15, both docs kept) and the golden. 2304b changed the version byte alone, so this order's hashes at 16 stand (the golden test passes).
  - 1911's: `ci/test-weights.tsv`, resolved as its refit plus this order's one row.
- **Re-measured on top of each merge:** §6, the tier-2 tests and `verify`.
  - At `3341dc6b` every ELF was rebuilt on spec 18, and the differential grew to 44 runs with Sentinel-03 and sentinel-02-power (§6).
  - two-bme's `verify` is refused on `main` (campaign gate: `attribute masks [{index:28,kind:13},{index:29,kind:13}] on the faulted bus`) and passes on the branch (117 s, 7.68 GB peak under 16G).
- **`TIER2_MIN` is 194:** this order's one tier-2 test on top of `main`'s 193 (2304b's), recomputed at the merge as ruled at 00:10Z.
- **History.** The work order stopped at §1.3 under §7's first condition (record pushed at `93f9a1b2`, §1 to §3 below). D1 (A) was ruled at 23:10Z with the fence widened, and is now built.
- **The bus fault is detected.** A two-device bus fault counts as detected by its named mechanism: two-bme's held-low SDA is E1 with nothing masked. `verify` passes the campaign gate on the branch and refuses it on `main` (measured, §5).
- **The wrong address is still caught.** U2's read sent to 0x75 is S1 on two-bme, and the one-device seeded set is unchanged (§5, §6).
- **The reproduction holds.** At the last merge, 44 committed and fixture runs were compared with `main`. 41 are byte-identical apart from the version byte. 3 move, and each is explained (§6): two predicted in writing before the run, and Sentinel-03's held-low, whose group joined the differential only once 2304b made it runnable.
- **D3 is a §7.1 row,** not built (§9).

## 1. Read first (§1.1, read at source at `35d23eae`)

- **The window `absent` is filled over.**
  - Renode's controller logs `[WARNING] i2c1: Unknown slave at address N.` (`absent_address`, `crates/aethelgard-cli/src/renode.rs:1827-1840`).
  - Every I2C check on a bus got the same list: the whole last boot's lines of its controller (`renode.rs:2119-2129`).
  - kind 13 archived that list as is (`campaign.rs:1243-1257`).
  - I2C witness lines go to steps by count, with no window (`renode.rs:960-967`, `:1043-1078`).
- **What the record carried.**
  - kind 13: `peripheral`, `reference`, `device`, `step`, `expected`, `observed`, `absent`. kind 12: the plan's `address`, `step`, `write`, `width`, `expected`, `mask`, `observed`, `status`.
  - Both are stamped at the boot's end (`record.rs:1042-1049`).
  - No I2C register access is archived (`renode.rs:519-531`).
  - So the record could not tell this witness's transfers from another step's. That was the §7 stop.

## 2. Measured on `main` before the change (§1.2)

**Why not Sentinel-03.** It cannot run a campaign on `main` (`PowerNotSimulated`, measured). The fixture is the committed `crates/aethelgard-cli/tests/fixtures/two-bme.json`: two modelled BME280s on SENSORS (U2 at 0x76, U5 at 0x77), plus an unmodelled U3. The scripts are `spike/3305/sh/` and the replayable archives are `spike/3305/out/archives/`.

| Run (`main`) | `attribute` | kind 13 `absent` (U2, U5) |
|---|---|---|
| baseline | N1 | `[]`, `[]` |
| held-low SDA | E1, **masked `[28,13]`, `[29,13]`** | `[118,119]`, `[118,119]` |
| U2 sent to 0x75 (`2204`'s patch shape; 0x77 is U5 here) | S1, U2's witness | `[117]`, `[117]` |
| U2 sent to 0x77, where U5 answers (F-COLLIDE) | **I4** | `[]`, `[]` |

- **`verify` on `main` refuses the bus fault** (measured, §5): "attribute masks [28,13], [29,13] on the faulted bus: something else fails too".
- **The healthy U5's address field was marked wrong too** in the 0x75 run. It was not cited only because S1 filters software evidence to the symptom's device (`evolve/src/lib.rs:1876-1877`).
- **The raw log** (`spike/3305/out/raw/`, from a scratch build that printed the log; never in a crate) holds no line that names a step.

## 3. The stop and its ruling

- **The stop** (`93f9a1b2`): evolve-only rules were all refused.
  - (C), "no declared device owns it", was refused by §1.3.
  - (D), "own address also absent", hides faults.
  - The stop's adversarial subagent found no evolve-only rule that is "this step's transfers". Its best, aligning failed steps with `absent` entries, trusts the firmware under test.
  - So (A) was recommended: a step boundary that the twin records.
- **Ruled, 23:10Z.** D1 (A) is accepted.
  - The fence covers the twin's I2C step hook and the per-witness attribution (`renode.rs`), the trace record with a bump, `evolve`, and tests and goldens.
  - Trace: the next number after `2304b`'s 15, so 16.
  - D2: the §7.1 row stands and this order owns it.
  - D3: built if small, else a §7.1 row.

## 4. The build

1. **The hook** (`renode.rs` `boot_hooks`; `anchor.rs` `Hook::Step`, its parse arm and `step_hook`).
   - Each I2C step check's status word gets a `DoubleWord` write hook that logs `AETHELGARD-STEP <check> <value> <count>`.
   - It is added with the boot word's hook: once, before the boot the judge reads, at the last release under a droop.
   - Every line goes through `hooks_of`, so it is parsed strictly and placed at its stamp. Malformed STEP lines (a missing or extra field, a leading zero, `0x`, a sign, a non-number, a value past 32 bits) are refused, as the other shapes are (`anchor.rs` `every_line_is_read_whole_and_strictly`, ruled 00:10Z).
   - **Probe (measured, `spike/3305/build/raw/`):**
     - one STEP line per I2C check, each after its own step's `Unknown slave` line and before the next step's;
     - no STEP line from the judge's `RECORD_UNSET` fill;
     - the BOOTWORD instruction counts are unchanged (721647 held-low, 361750 wrong-address).
   - **The stamps change.** After the first hook fires, Renode's log stamps become instruction-precise rather than quantum-coarse (7.210940 ms against 7.200000). No committed trace moved for it (§6).
2. **The attribution** (`renode.rs` `absent_by_step`).
   - An `Unknown slave` line goes to the I2C check on its controller whose status write first follows it.
   - The boundary is the firmware's, so it is trusted only as the generated firmware draws it: each I2C check on the controller writes its status exactly once, and a write follows the line. The generated `record()` writes each status once, as a word, after its transfer (read in the generated `main.rs`).
   - Otherwise the line goes to every I2C check on the controller, which is `main`'s whole-run behaviour. A status written early, twice or never therefore never moves an address onto another device's step: adversarial F1–F3, §8.
   - With no hook line readable (the filter dropped lines), every line goes to every step. The witness is then unreadable anyway.
3. **The trace.**
   - `TRACE_VERSION` 16 in `event.rs`, on top of 2304b's 15 since the merge of `3341dc6b`; 14 → 16 before it.
   - kind 13's `absent` doc now says "this step", `record.rs`.
   - The shape is unchanged (`Vec<u8>`). `envelope` accepts only the exact version (`event.rs:424`), so no older archive is read under the new meaning.
4. **`evolve`: no logic change.** `elsewhere = absent.find(|a| a != address)` over a per-step list is the ruled rule exactly, so the "one-line fix" is the doc of `witness_wrong`. The fixture that fails on `main` therefore exercises the fill (tier 2), and "restore the old rule" is mutant T1.
5. **Tests.**
   - **Tier 1, `renode.rs`:** attribution over the measured probe lines (held-low and wrong address); every untrusted case (F1, F1b, F2, F3, no hook, another controller's line); a status write directly after its line; a write on another controller.
   - **Tier 1, the pinned monitor scripts:** three tests (`a_serial_boot_…`, `a_later_port_…`, `a_droop_boot_…`) now carry the step hook. It is a move of the expected command text, not a weakening.
   - **Tier 1, `verify.rs`:** `detection` refuses `main`'s measured attribution and accepts the branch's. It pins `detection` on the two measured inputs and passes on `main` too: the fix itself is shown by tier 2 and §5.
   - **Tier 2, `twin_tier2.rs`** `a_fault_on_a_bus_with_two_devices_is_not_masked_and_a_wrong_address_still_is_software`, with two scenario fixtures (`two-bme-held-low-sda.json`, `two-bme-i2c-timeout.json`):
     - held-low: E1, nothing masked, subjects as seeded, `absent` U2 `[118]` and U5 `[119]`;
     - 0x75: S1 with U2's witness alone, U5 `[]`.
   - **The existing wrong-address tests are unchanged:** `evolve` `a_transfer_the_controller_found_no_device_for_is_evidence_unless_it_is_the_devices_own`, and the seeded set's three wrong-address cases.
   - **The golden trace** is re-pinned (`7eccc893…5d31`). Set back to 14 it is `4400g`'s `bcd082c2…8383`, which is asserted.

## 5. Measured after (`75d73745`'s code; the hardening changes no well-formed run)

| two-bme run | `main` | branch |
|---|---|---|
| held-low SDA, `attribute` | E1, masked `[28,13]`, `[29,13]` | **E1, masked `[]`** |
| held-low SDA, kind 13 | `[118,119]`, `[118,119]` | `[118]`, `[119]` |
| 0x75, `attribute` | S1, U2's witness | S1, U2's witness |
| 0x75, kind 13 | `[117]`, `[117]` | `[117]`, **`[]`** (U5 no longer marked wrong) |
| 0x77 (collide), `attribute` | I4 | I4, kind 13s identical to `main`'s |
| `verify --spec two-bme.json` with a baseline and held-low (16G) | **fail:** campaign gate refuses, "attribute masks … something else fails too"; 309 s, 7.37 GB peak | **pass:** campaign gate passes; 306 s, 8.07 GB peak |

The `verify` scenario files are `spike/3305/verify/two-bme-baseline.json` and the held-low fixture; the script is `spike/3305/sh/verify.sh`.

## 6. Reproduction (§1.4)

- **Predictions first:** `spike/3305/build/predictions.md`, written before the branch's run.
- **Method:** `spike/3305/sh/diff.sh` and `cmp.py`.
  - One firmware per group, built by `main`'s CLI (the generator is unchanged).
  - Every committed scenario through `main`'s CLI and through the branch's, each with replay and attribute.
  - Trace bytes compared with the version byte normalised.
  - The groups: sentinel-01-minimal (16: its committed scenarios and the seeded set's unmutated ones), sentinel-01-bench (5), motors-dshot600 (2), motors-pwm (1), serial-four (3), and two fixture groups no committed scenario covers. The fixture groups are `sentinel-01-ext-bmp581` (its EXT BMP581 has three steps) and two-bme, each with a baseline and held-low SDA. Every group has at least one I2C step, so the hook is installed on every board.
- **Results:**
  - **Hook alone** (before the attribution; `cmp-hook`): 31 of 31 byte-identical, trace included. Neither the hook nor the finer stamps move anything, on motors, serial and droop boards alike.
  - **Branch:** 29 of 31 byte-identical apart from the version byte. Two moved, both as predicted:
    - **two-bme held-low SDA:** kind 13 `[118,119]` twice became `[118]` and `[119]`. The attribution moves from masked `[28,13]`, `[29,13]` to none. This is the masked result the work order exists for.
    - **ext-bmp581 held-low SDA:** U5's three steps each carried `[71, 71, 71]` on `main` and carry `[71]` on the branch (12 bytes). The attribution is unchanged: E1, nothing masked before or after, since every address was the device's own.
- **After each merge of `main`** (`main`'s CLI rebuilt each time from a `git archive` of `main`):
  - at `52a2d62e` and at `03d89554`: the same 29 byte-identical runs and the same 2 moves;
  - at `3341dc6b`: every ELF was rebuilt on spec 18, and two groups were added, 2304b's committed `scenarios/sentinel-03` (10) and `scenarios/sentinel-02-power` (3). 41 of 44 are byte-identical apart from the version byte (15 → 16). Three moved:
    - the two above, unchanged;
    - **Sentinel-03 held-low SDA** (not predicted in writing: the group is new). U7's (BMP581's) three step witnesses each carried `[70, 70, 70]` on `main` and carry `[70]` on the branch: 3 × 4 bytes, found by matching the CBOR array bytes in both traces. The attribution is unchanged, E1 with nothing masked before and after. The shape is ext-bmp581's: SENSORS carries a TMP117 too, but it has no I2C step witness, so every address on the list was U7's own.
- **The seeded set at tier 2** (`attribute_seeded`, pinned exact evidence indices, in the image, at `e6847099`): **11 passed**, 640 s. That covers the three one-device wrong-address cases (`wrong-address-2k2`, the two traps) with their exact evidence and masks.

## 7. Mutants (one per guard this build adds; each restored, the tree clean after)

- **Tier 1.** A subagent ran these at `575d33a3` in its own worktree; I spot-checked them.
  - M1 (every line to the fallback), M2 (the whole-boot list per check), M3 (`max` for `min`), M4a (`e > i + 1`), M5 (a later write overwrites), M6 (fallback to the first check), M8 (the STEP parse arm), M9 (no address check in `evolve`), M10 (address field never wrong), M11 (`detection`'s masked refusal removed): **all killed**.
  - M4b (`e >= i`) is equivalent: a hook line and an `Unknown slave` line are never the same line.
  - M7 (any controller's write ends the step) **survived**. A two-controller test was added, and M7 is now killed (measured).
- **After the hardening** (`75d73745`, host):
  - H1 (any write count trusted), H2 (`all` → `any`), H4 (`max_by_key`), H5 (any controller), H6 (untrusted lines dropped), H7 (a trusted line to every step): **killed**.
  - H3 (a write directly after the line excluded) survived, so the adjacent case was added, and H3 is now killed.
- **Tier 2, in the image** (`spike/3305/sh/mut2.sh` shape, host-side edits):
  - T1, the call site restored to the whole-boot list, which is `main`'s rule: **killed**. U2 and U5 got `[118,119]` each, exactly `main`'s measurement, so the fixture fails on `main`.
  - T2, no step hook installed: **killed** (every line to every step).
  - T0, unmutated: passes.
  - The first T1/T2 run failed for the wrong reason (`BridgeUnavailable`: no bridge beside the mutant binary). It was not counted; it was re-run with the bridge.

## 8. Adversarial pass (full; a fresh Opus subagent at `575d33a3`, given the code and the work order, not my conclusions)

- **F1 (medium, new, run):** a status written before the transfer moved the line to the other device's step, giving U2 I4 instead of `main`'s S1. **Fixed** by the trust rule (§4.2), with a test.
- **F1b (derived):** a firmware that marks each record before its transfer would bring F-MASK back with no firmware defect. **Fixed** by the same rule: two writes each makes the bus untrusted, and both steps carry `[118,119]`, `main`'s outcome. Tested.
- **F2 (low–medium, new, run):** a step that never writes its status sent its line to the next step. **Fixed**: untrusted. Tested.
- **F3 (low):** the fallback named "the step under way", which was wrong for any step but the last. **Fixed**: a line no write follows goes to every step. Tested.
- **F4 (pre-existing):** F-COLLIDE has no disposition yet. It is now §9 D3.
- **F5 (low):** the hook watches `DoubleWord` writes only. A byte status write gives no STEP line, so the bus is untrusted and `main`'s behaviour applies: safe by the rule. A STEP index that is out of range or names an SPI check is ignored (the firmware cannot forge a log line).
- **F6 (test quality):**
  - The `verify.rs` test passes on `main` too. It pins `detection` on the two measured inputs and is said so in §4.5.
  - The unit tests quoted STEP lines that were not committed; they are now in `spike/3305/build/raw/`.
- **Held, per the subagent:**
  - The line shift (M4a killed, M4b equivalent).
  - Both line types index the same `last_boot`.
  - Hook placement under resets.
  - Strict parsing.
  - No new `cpu_halted`: the sleep deficit only grows.
  - The finer stamps, with the differential identical.
  - The 16-line cap, now per step.
  - The exact-version envelope.
- **Residual, inferred:** a firmware that writes each step's status once, but before its transfer, still shifts lines by one step. Such a record claims a result before the transfer, which is the forged-record case the boot judge already fails as unwitnessed (`2204`).

## 9. Rulings and decisions

- **D1:** built (§4).
- **D2:** the §7.1 row is this order's.
  - Before this build, any board with two modelled devices on one I2C bus had every bus fault refused by `detection`.
  - After it, two-bme is detected.
  - **Sentinel-03, re-run after 2304b and 4202 (measured):** its committed held-low SDA through `verify` with its baseline passes on `main` (393 s) and on the branch (377 s, 7.68 GB peak under 16G).
    - It was never masked: only U7 is witnessed on SENSORS, so no witness reads another device's address.
    - The row's case, two witnessed devices on one bus, is two-bme. It is a fixture, and no committed board has one today.
- **D3, F-COLLIDE: a §7.1 row, not built.**
  - With (A), collide's kind 13s are identical to `main`'s (measured, §5), so the per-step data gives `evolve` nothing for this case.
  - What the case needs is a different change: judge a witness's whole `observed` against `expected`, not only its first write and read (`lib.rs:987-989`). U5's witness carries U2's transaction as two extra lines.
  - That is an `evolve` rule change whose effect on every committed attribution must be measured, which is outside this fence.
- **For the architect:**
  - `TIER2_MIN`: 194, this order's one tier-2 test on top of `main`'s 193. That one line is inside the fence by the 00:10Z ruling.
  - **The bench package.** `fab/sentinel-01-bench/package/MANIFEST.json` pins `verdict_sha256`. The verdict carries each campaign run's state hash, which the version byte moves (inferred), so CI's `bench-package` job may report AWAITING-PACKAGE. It failed on `f71a7323` and `bfad4c96`. I reproduced it locally: CI's `VERIFY` with `--against-fab`, in the image, with the branch's CLI. The result is `against_fab: differs`, and **only `MANIFEST.json`** differs from the derived package. Every Gerber, drill, BOM, CPL and FAB-NOTES file is byte-identical (measured; the field that moves is inferred, since `--against-fab` writes no derived manifest). The tier-1 `bench_board_tier1` passed in the gates set. CI decides it: if the job reports a move, it waits on the architect's `write=true` dispatch, never a hand edit.
  - **The trace number:** 2304b merged first with 15, and this order holds 16.

## 10. Owed and fence edges

- **Done at `3341dc6b`:** `main` merged after 2304b, §6 re-run, Sentinel-03's held-low through `verify` (§9 D2), trace 16 kept.
- **Owed:** the bench package's `write=true` dispatch (the architect's).
- **Fence edges, said on the bus:**
  - `anchor.rs`'s `Hook::Step`: every `AETHELGARD-` line must parse, or the run is refused;
  - `verdict.rs`: one doc line;
  - `verify.rs`: one unit test;
  - two scenario fixtures in `cli/tests/fixtures`;
  - `spike/3305/verify/two-bme-baseline.json`: a spike input.
  - `ci/test-weights.tsv`: one row and its provenance line for the new tier-2 test. twin_tier2 is lock-serialised, and `ci/shard.sh plan` refuses any of its tests without a row; CI's first run on `f71a7323` failed every tools shard on exactly that. The row is 8.4 s, a Furnace estimate by AET-WO-2302's rule, until the refit rule measures it. Said on the bus.

## Gates

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`, with and without `with-tools`: clean (host).
- `ci/gates.sh run` (the `gates` job's set, host, target inside the repository as CI has it): **green** at `6d61a87b`, at the 3303b merge, at the 4202 merge `84279860` (132 result lines each), and at the 2304b/1911 merge `556affbe` (134), all ok.
  - A first run with the target outside the repository failed `fab_tier1`'s `a_rail_or_bus_no_fault_kind_reaches_is_named` with `BaseOutsideRoot`, a test-scratch layout artefact: it passes with the target inside the repository (measured).
  - The first run also caught three pinned monitor-script tests, moved in §4.5.
- Tier 2 in the pinned image: the new two-bme test passes (T0, and again after each merge of `main`; 6–7 s, under the 60 s weight-row threshold), and the seeded set passes (11 of 11).
- After the 2304b merge (`556affbe`), in the image, all passed (measured):
  - this order's two-bme test (5.90 s);
  - 2304b's `battery_sag::sentinel_03_s_campaign_meets_every_expectation_replays_and_repeats` (132.84 s) and `battery_sag::the_power_fixture_s_campaign_…` (41.52 s);
  - `the_sentinel_02_fixture_with_parts_of_our_own_runs_every_step_witnessed`;
  - 4202's `campaign_tier2` test.
- After the 4202 merge, the tier-2 tests on the boards 4202 moved were run on the branch in the image. Those boards carry TMP117 and BMP581 on one I2C bus, so `absent_by_step` reaches them. The tests: `twin_tier2`'s `the_sentinel_02_fixture_with_parts_of_our_own_runs_every_step_witnessed`, `the_fixture_s_campaign_archives_each_configuration_and_replays` and `the_sentinel_02_shaped_board_is_witnessed_whole`; 4202's `campaign_tier2` `a_reset_on_an_instant_renode_would_run_short_boots_where_the_judge_places_it`; and `models_tier2` (12). All passed (measured).
- Reduction pass (diff): nothing to remove in a crate. 0 duplicates: the bus name is derived once (`absent_by_step`'s `controller`), and the whole-run fill is replaced, not kept beside. 0 dead paths, 0 protected kept, 0 findings below. In the spike, earlier: −3 files (scenario copies), and the stop record is condensed into §1–§3 here.

## What moved

- **Traces:** the version byte 15 → 16 on every archive (14 → 16 before 2304b). Content moved in three runs (§6).
- **Golden:** trace `bcd082c2…8383` → `7eccc893…5d31`; state `ada1b631…d8f7` → `f4c133fa…c693`.
- **Monitor scripts:** every board with an I2C step check gains one hook per I2C step.
- **The bench package:** may report AWAITING-PACKAGE (inferred, §9).
