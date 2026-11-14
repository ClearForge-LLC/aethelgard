# AET-WO-1909 — FEEDBACK

**Status: built to the architect's ruling on PR #115 (comment 6873338632): A, B, C and D; `oq_03` removed; E declined. Adversarial and reduction passes done; three counted CI runs recorded below.**

**Branch:** `wo/aet-1909-ci-reduction` · **Base:** `a5be795` (`origin/main`, merged in) · **Builder:** Claude (Opus 5.5, orchestrating). Census rows and D8's assertion messages were done by Sonnet subagents, each checked against the source. The adversarial pass was run by a fresh Opus subagent.

# Part 2: the build (§2), as ruled

## What changed

- **`tests/common/verify.rs` (new).**
  - It holds the helpers `board_tier2` and the three new binaries share: `aethelgard`, `one`, `build`, `gate_reads`, `package_files`, `copy_tree`, `sha`, `root`, `board` and `SPEC`. Each was moved out of `board_tier2.rs`, not copied.
  - It also holds `Shared<T>` (F6): a `OnceLock<Result<T, String>>` whose maker runs under `catch_unwind`. A run that fails is stored once and never made again. Every test that reads it fails at once, naming its own property and the stored failure.
- **Three new tier-2 binaries (D1 (a)),** each reading one shared run. Every test keeps its name, and every original assertion is kept.

  | binary | tests (moved from `board_tier2.rs`) | the shared run | `verify` runs, before → after |
  |---|---|---|---|
  | `verify_2k2_tier2.rs` (B) | `verify_binds_the_reference_and_two_runs_print_the_same_bytes`, `verify_fab_out_writes_the_package_its_verdict_names_at_any_path` | V(2k2, `--against` tree, `--fab-out`), at two paths of different lengths | 4 → 2 |
  | `against_fab_tier2.rs` (A) | `verify_against_fab_finds_the_package_it_wrote_identical`, `verify_against_fab_names_every_changed_and_extra_file` | one V(2k2, `--fab-out`) package; each test compares its own copy, one `verify` at a time (`ONE_AT_A_TIME`) | 4 → 3 |
  | `verify_47k_tier2.rs` (C) | `verify_fails_the_47k_pull_up_with_the_campaigns_findings`, `verify_fab_out_writes_nothing_when_a_gate_fails` | V(47k, `--fab-out`) into an empty directory | 2 → 1 |
- **`attribute_seeded.rs`:**
  - `tier2::oq_03_the_reference_runs_bytes_with_kind_15` is removed (D7). `ran()` already prints its data for every case; the module comment now says so.
  - The two committed-proposal tests now log `proposals/ absent: nothing to check` as an `ANSWER 3200` line, instead of returning silently (F2).
- **D8, every assertion named.** Every assertion in the three F1 tests, and in every test that reads a shared run, now carries a message naming its property. That covers the new binaries, `ran()` and `REGATED` and their six readers, `first()` and its five readers plus the droop test, and `firmware()`/`two_bme()` and their seven readers.
  - The change is messages only, checked mechanically: string literals, whitespace and commas are stripped from the old and new file, and the two token streams are diffed (`~/.cache/aet-1909/msgonly.py`).
  - The residue is zero in `campaign_tier2.rs`. In `twin_tier2.rs` it is one hunk, a message change, read by hand. In `attribute_seeded.rs` it is my own D7 and F2 edits, plus two added `dir.display()` context arguments.
- **`ci/`:**
  - `TIER2_RE` names the three new binaries.
  - `ci/colocate.tsv` keeps each new binary and `campaign_tier2` (D6) whole on one shard.
  - `ci/test-weights.tsv` drops the seven lines for tests that moved or went.
  - `TIER2_MIN` is 90 (91 − `oq_03`; the six moved tests still count), its comment naming every moved and removed test and the two vacuous ones.
- **Not changed:** any production file, `ONE_PROOF` (E declined), `ci/SHARDS` (8), the guard and the canary.

## B and C: the evidence the ruling asked for, before the old invocations went

**1. Byte comparison** (`~/.cache/aet-1909/bytes/`; the pinned image; each verdict compared key by key with `cmp.py`):

| compared | keys only in one | shared keys that differ | equal apart from those |
|---|---|---|---|
| V(2k2, `--against`) vs V(2k2, `--against`, `--fab-out`) | `fab` | none | yes |
| V(2k2, `--fab-out`) vs V(2k2, `--against`, `--fab-out`) | `against` | `fab.files["MANIFEST.json"]` | yes, apart from those |
| V(47k) vs V(47k, `--fab-out`) | `fab` | none | yes; `--fab-out` held 0 entries |

- **The one shared key that differs is caused by the extra flag's key.** `MANIFEST.json` names the sha256 of the verdict line without `fab` (`verify.rs:709-722`), and with `--against` that line carries `"against":"identical"`.
- The other 14 package files are byte-identical between the two `--fab-out` runs (`pkg-fabout.sha`, `pkg-both.sha`).
- `verify_fab_out_writes…` still checks the manifest against the line its own run printed, so it checks the same binding.

**2. Independence**, from the source:
- `main.rs:340-353` passes `--against` and the fab mode to `verify::verify` as separate arguments.
- In `verify.rs`, `--fab-out` is checked usable before any gate (`:566-574`). `--against` only reads its directory and diffs it against the in-memory generated `files` (`:576-603`), setting `report.against`, or failing before any gate if the trees differ.
- Every gate runs on the scratch tree written from `files` (`:605-610`), never on the `--against` directory.
- `--fab-out` is written only in `package()`, after every gate passed (`:694-780`), from the scratch root's board and ELF.
- So neither reads what the other writes, and neither changes a gate's input. Their one coupling is the failure path: a differing `--against` writes no package (`:589-590`). B's shared runs pass `--against`, so that path is not shared.

## Mutants (§4): for each shared run, one property broken, caught by the named test only

Each mutant was a production edit in a scratch copy (`~/.cache/aet-1909/mutX`, `mutY`), run against all three binaries in the pinned image. Mutants on disjoint paths share a build.

| mutant | caught by | the assertion that fires | the other test reading the run |
|---|---|---|---|
| X: `fab.rs:100` FAB-NOTES "plated via, drilled" → "plated via drilled" | `verify_fab_out_writes_the_package_its_verdict_names_at_any_path` | `verify_2k2_tier2.rs:397` "FAB-NOTES.md states plated via, drilled 0.3 mm" | `verify_binds…` passes |
| Y: `verify.rs:602` `against` "identical" → "same" | `verify_binds_the_reference_and_two_runs_print_the_same_bytes` | `verify_2k2_tier2.rs:91` "--against finds the committed tree identical" | `verify_fab_out_writes…` passes |
| X: `verify.rs:762` `against_fab` "identical" → "same" | `verify_against_fab_finds_the_package_it_wrote_identical` | `against_fab_tier2.rs:83` "the derived package is identical" | `…names every changed…` passes |
| Y: `verify.rs:764` "differs" → "different" | `verify_against_fab_names_every_changed_and_extra_file` | `against_fab_tier2.rs:120` "the derived package differs" | `…finds…identical` passes |
| X: `verify.rs:687` "not_written" → "unwritten" | `verify_fab_out_writes_nothing_when_a_gate_fails` | `verify_47k_tier2.rs:103` "the package is not written" | `verify_fails_the_47k…` passes |
| Y: after `verify.rs:972`, the campaign gate's `exit_code` + 7 | `verify_fails_the_47k_pull_up_with_the_campaigns_findings` | `verify_47k_tier2.rs:67` "the scenario met its own expectation" | `…writes_nothing…` passes |

## Coverage map: every removed or merged test, what it guarded, and what guards it now

| test | what it guards | guarded now by | evidence |
|---|---|---|---|
| `oq_03_the_reference_runs_bytes_with_kind_15` (removed) | nothing of its own: `ran()`'s checks | `every_case_fails_the_way_its_manifest_says` and four more `ran()` readers, at the same assertion | mutant M1 (census §3): both died at `attribute_seeded.rs:433` |
| `verify_binds…` (moved to `verify_2k2_tier2.rs`; now with `--fab-out` as well) | `--against` binding and byte-identity | itself, on the shared runs | mutant Y `verify.rs:602` |
| `verify_fab_out_writes…` (moved; now with `--against` as well) | the package at two paths | itself | mutant X `fab.rs:100` |
| `verify_against_fab_finds…identical` (moved; shares the package) | `--against-fab` identical | itself | mutant X `verify.rs:762` |
| `verify_against_fab_names every changed…` (moved; shares the package) | `--against-fab` differences named | itself | mutant Y `verify.rs:764` |
| `verify_fails_the_47k…` (moved; now with `--fab-out`) | 47k fails with the campaign's findings | itself | mutant Y `verify.rs:972` |
| `verify_fab_out_writes_nothing…` (moved; shares the run) | nothing written on failure | itself | mutant X `verify.rs:687` |
| the dropped invocations V(2k2, `--against`) alone, V(2k2, `--fab-out`) alone and V(47k) alone | the same verdicts | the combined runs, equal apart from the named keys | the byte comparison and independence above |

## Adversarial pass (§5, targeted, a fresh Opus subagent)

It ran in the pinned image (15 GiB cap, swap 0, read back in `~/.cache/aet-1909/adv/caps.log`), on a copy of the tree. Logs are under `~/.cache/aet-1909/adv/`. **No defect found in the build.**

1. **Lose a failure.**
   - Mutants X and Y (above) each go red naming their property.
   - The reviewer's three further mutants go red only in the expected test, each naming its property:
     - **KA:** a passing `--fab-out` without `--against` exits 1. Both A tests fail at once with "<property>: the shared run `verify 2k2 --fab-out, the package to compare` failed once…", and the log shows one `verify`: 145.5 s for the first test, 0.000 s for the second.
     - **KB:** the manifest's verdict line carries a trailing space. B's fab test fails at "the manifest names the verdict without its fab key"; `verify_binds…` stays green.
     - **KC:** a stray file is left after a failed `--fab-out`. C's `…writes_nothing…` fails at "a failing run wrote nothing into --fab-out"; `verify_fails_the_47k…` stays green.
2. **Order.** libtest sorts tests by name, so the reviewer ran each test executable directly with `RUSTC_BOOTSTRAP=1 … --test-threads=1 -Z unstable-options --shuffle-seed N`. Seeds giving both orders were confirmed in the output, and each test was also run alone with `--exact`. **All 12 runs pass** (`order-{A,B,C}.log`).
   - A, changed-copy first, then identical, passes: the copies are isolated.
   - A with default threads passes, its three `verify` runs one after another (`parallel-A.log`), so there is no deadlock between `ONE_AT_A_TIME` and the `OnceLock`.
3. **Poison.** Each maker was made to panic; in snapshot A it panics while holding `ONE_AT_A_TIME`. **All six dependent tests fail, in parallel, serial and alone runs, at 0.000 s each, with no hang.** The maker's panic appears once per process, and every message names its property and the stored failure (`poison.log`, `cur-poison-A.log` on the final A).

**Finding (the coverage trade B and C ruled): LP1 and LP2.** Two mutants that the pre-build tests killed now pass all of tier 2:
- **LP1** (`verify.rs:602`): `--against` alone reports "same" when no fab flag is given.
- **LP2** (`verify.rs:680`): a failing `verify` without a fab flag exits 0.

The originals go red, at `board_tier2.rs:405` and `:504` of `cfafdb4`.
- **Why:** no test at any tier now runs a full `verify` without a fab flag. A regression that tied `against` or the exit code to the fab mode would pass CI.
- **This is exactly the trade the ruling accepted on the byte comparison and the independence argument above.** I did not add a guard. One in tier 2 costs a full `verify` (about 215 CI s, much of B and C's saving). One in tier 1 would need a `#[cfg(test)]` in `verify.rs`, a production file outside the fence.
- **Backlog, for the architect:** a cheap guard that `against` and the exit code do not depend on the fab mode, for example a unit test over `verify`'s report assembly, in a work order whose fence allows it.

**Smaller findings:**
- The shared runs' scratch directories are not removed, as the per-test ones were: about 288 KB per A directory, in the container's `/tmp`. Removing a shared directory needs every reader to have finished, which a `OnceLock` cannot tell. Left as is.
- `Shared::get` cannot catch an abort, only an unwinding panic. A panic payload that is not a string still fails, as "a panic without a message".

## CI: before and after, measured

- **Suite seconds** are the sum, over the eight `tools-shard` jobs, of the step "cargo test inside the image, this shard's tests", from the GitHub API (`~/.cache/aet-1909/runtimes.sh RUN`).
- **A shard's time** is its job, start to end. Spread is the slowest shard minus the fastest.

**Before** (`main`, the same base code, eight shards):

| run | commit | suite test-step s | slowest | fastest | spread |
|---|---|---|---|---|---|
| 43564483946 | `a5be795` | 4,622 | 16:14 | 9:18 | 6:56 |
| 43562913696 | `79bdfe1` | 4,838 | 15:36 | 9:20 | 6:16 |

**After:**

| run | commit | plan | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | suite test-step s | slowest | spread | `tier2_tests_passed` |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 (43570762262) | `ad432a3` | provisional block weights | 8:23 | 9:58 | 11:24 | 8:41 | 9:15 | 8:08 | 8:49 | 10:50 | 3,471 | 11:24 | 3:16 | 90 (floor 90) |
| 2 (43573333731), counted 1 of 3 | `af2ec01` | refitted | 8:09 | 9:58 | 12:10 | 9:37 | 8:29 | 10:49 | 10:26 | 8:35 | 3,684 | 12:10 | 4:01 | 90 (floor 90) |
| 3 (43574423322), counted 2 of 3 | `94488c7` | refitted | 8:38 | 9:46 | 11:59 | 10:54 | 8:16 | 8:14 | 10:00 | 11:04 | 3,722 | 11:59 | 3:45 | 90 (floor 90) |
__CI_ROWS__

- **Run 1 measured the blocks in one process** (libtest's `finished in`): `attribute_seeded` 473.4 s, `against_fab_tier2` 562.2 s, `verify_2k2_tier2` 404.6 s, `verify_47k_tier2` 197.3 s, `campaign_tier2` 36.9 s.
- **The refit** (`ci/shard.sh` REFIT RULE step 3: a colocated block at its in-process wall time) puts the new blocks at those values. `attribute_seeded` stays at 720, still its worst sample.
- No per-test weight changed: no test that stays in a shard by name changed its work. The moved tests' lines left with them.
- The refitted plan: the 720 s block, the 562 s `--against-fab` block, and six shards of 465 s. Run 1 does not count toward the three; runs 2 to 4 are on the refitted plan, and each commit after a run records it here.

__CI_RESULT__

## Reduction pass (diff)

Run by a fresh subagent, read-only. It proposed; I applied what was inside the diff or a copy of a fact the diff touched.

- **Done:**
  - `attribute_seeded.rs`'s own `copy_tree` and `root`, and `campaign_tier2.rs`'s `root`, were byte-for-byte `common::verify`'s, so they now use it.
  - `against_fab_tier2.rs` no longer takes `ONE_AT_A_TIME` inside the package's maker. No `--against-fab` comparison can start before the package exists, so that lock guarded nothing. The two comparisons still take it.
- **Not done, and why:**
  - Shortening `TIER2_MIN`'s comment: §2.3 and the ruling require every moved and removed test named there.
  - The `scratch` doc line and `colocate.tsv`'s per-block comments: they carry the why, and the refit evidence the rule asks for.
- **Proof:** the three touched binaries pass in the pinned image after the change, and mutants X and Y are still each caught by `against_fab_tier2`'s named test, and only by it (`against_fab_tier2` 2 passed in 432.4 s, `campaign_tier2` 8 passed, `attribute_seeded` 11 passed in the image, and its 5 tier-1 tests on the host; X still fails only `…finds…identical` and Y only `…names every changed…`; `~/.cache/aet-1909/reduce-run.log`, `mutX/reduce.log`, `mutY/reduce.log`).
- **Findings, outside this diff:**
  - Ten or so tier-1 files carry their own `root`, `sha`, `SPEC` or `scratch`. A phase-gate pass could move the identical ones to `common`.
  - Three places list the tier-2 binaries: `TIER2_RE`, `ci.yml`'s comment and `colocate.tsv`'s. A guard comparing `TIER2_RE` with the files under `tests/` would let two of them go.
  - `twin_tier2` and `campaign_tier2` build the reference firmware by the same cargo invocation, in two places.
- **`common::verify::scratch` uses `temp_dir()`, as the moved tests did.** Inside the image that is the container's `/tmp`, not Furnace's tmpfs.

Reduction pass (diff): −12 net (9 added, 21 removed) · 3 duplicates removed · 1 dead lock removed · 0 protected kept · 3 findings above.

# Part 1: the census (§1), as stopped for the ruling

## Answer first

1. **Every guarded failure the suite pays for is distinct, with three exceptions.** I found no test whose failure another test already catches. The exceptions assert nothing of their own:
   - `attribute_seeded::tier2::oq_03_the_reference_runs_bytes_with_kind_15`, which repeats `ran()`'s checks (§3, with its mutant);
   - the two committed-proposal tests, which return early because `proposals/` is absent (F2).
2. **Where CI pays twice is the same `verify` pipeline on the same reference.**
   - Six `board_tier2` tests make ten full `verify` runs: eight on 2k2, two on 47k.
   - Every variant costs the same: 122 to 125 s on Furnace, 191 to 258 CI s. Kani dominates each run. A flag adds nothing.
   - One shared `--fab-out` run is the same invocation that two tests repeat today; it is bookable (A, 1 run).
   - Three more runs can go only by merging invocations that differ in a flag (B, 2 runs; C, 1 run). That is the architect's call, not mine.
3. **`attribute_seeded` already shares its expensive work in one process** (`RAN`, `REGATED`). Nothing there is booked as a saving.
   - On a CI-sized container the block's wall time is `p3_gate_clause_2`'s own chain: 303.8 s alone, 305.9 s for the whole block.
   - That chain is `ran()` (68 s), then two re-gates of about 115 s each. Each re-gate is mostly one Kani proof (108 to 109 s).
   - The re-gates run one after the other because `p3_gate_clause_2` calls them in one loop on one thread, holding `REGATED`'s lock across `every_gate` (`:699`, `:744`, `:848-857`). Today `ONE_PROOF` never sees two callers.
   - Run concurrently, the two proofs take 122.8 s against 217.3 s, but the container's memory peaks at 11.3 GB against 5.5 GB (E, a decision).
4. **Colocation saves little.** CI splits `campaign_tier2`'s `first()` (6.1 s) across four shards, so each pays it again; keeping the binary whole saves about 31 CI s (D). `twin_tier2`'s `firmware()` costs about a second; nothing to gain.
5. **Predictions (§1.5), as the eight-shard plan by `ci/shard.sh assign`:**
   - Today: maximum 720 s, total 5,166 planned s.
   - Booking only A: 720 and 4,936.
   - B, C, D and E: every shard 524 planned s, total 4,192.
   - **Only E moves the maximum.** It is the `attribute_seeded` block, and nothing else reaches it.
6. **Decisions needed:** D1 to D8, under "Decisions needed" below. **Findings:** F1 to F6 (F4 withdrawn).

## Method

- **Rows:** every test that `cargo test --workspace --features aethelgard-cli/with-tools -- --list` prints, in the pinned image (`container/IMAGE`, `sha256:c69ff9bc…`). That gives 1,171 workspace tests. **91 of them are in the seven tier-2 binaries of `TIER2_RE`, which is exactly `TIER2_MIN` (91).** The canary counts every pass in those binaries, so the five tier-1 tests at the top of `attribute_seeded.rs` are among the 91.
- **CI seconds** are `ci/test-weights.tsv` (`AET-WO-1908`, each test alone on a GitHub runner). A test that file does not list took under 0.5 s there, written `<0.5`. I re-timed the `with_tools` Kani tests serially to check this (F4, withdrawn).
- **Furnace measurements** were taken in the pinned image under rootless podman with `--memory 15g --memory-swap 15g --pids-limit -1 --cpus 4`, so the container is about a GitHub runner's size (4 vCPU, 16 GB).
  - After start, 0 was written into the container's `memory.swap.max`, and each run's `memory.max=16106127360 memory.swap.max=0 pids.max=max cpu.max=400000 100000` was read back and logged (`~/.cache/aet-1909/caps.log`, every run).
  - TMPDIR was unset inside the container, as on CI. Runs were one at a time, with nothing else in the container.
- **Furnace seconds to CI seconds** use a ratio per pipeline, never one scale (`AET-WO-1908` deleted `CI_SCALE` for that reason). Each ratio is a CI-measured test that runs exactly one such invocation, over the Furnace time of that invocation. Every CI-second figure derived this way is marked *est.* and printed beside its Furnace measurement.
- **Rows were read, not guessed.** Six read-only subagents each wrote rows for one binary group to one schema: guarded failure, class, normalised invocations, shared state, defining assertion with file:line. They did not judge redundancy. I checked their counts and claims against the source: 22 seeded cases, 10 firmwares, `oq_03`'s body, `proposals/` absent, `verify`'s flags (`main.rs:106-121`), and `ONE_AT_A_TIME`.

### Measurements on Furnace (pinned image, 4 CPUs, 15 GiB, swap 0)

**Each expensive invocation, alone, from the repository root as the tests run it** (`~/.cache/aet-1909/verify-times.sh`). The variants were timed for cost; I did not compare their verdict bytes across flags.

| invocation | Furnace s | CI s of a test that runs exactly this | ratio |
|---|---|---|---|
| V(2k2) | 125.2 | — | — |
| V(2k2, `--against` tree) | 123.5 | `verify_binds…`: 382.4 for ×2, plus `build`, `campaign` and `emit` | 1.52 |
| V(2k2, `--fab-out`) | 123.6 | `verify_fab_out_writes…`: 515.7 for ×2, plus `emit` | 2.08 |
| V(2k2, `--against-fab`) | 123.2 | `…finds…identical`: 416.1 and `…names every changed…`: 453.5 (each one `--fab-out` plus one `--against-fab`) | 1.69, 1.84 |
| V(2k2, `--against` and `--fab-out` together) | 123.3 (exit 0) | no test runs it today | — |
| V(47k) | 122.4 (exit 1) | `verify_fails_the_47k…`: 218.2 | 1.78 |
| V(47k, `--fab-out`) | 122.4 (exit 1) | `verify_fab_out_writes_nothing…`: 199.6 | 1.63 |
| P(boards/sentinel-01) | 106.4 | `the_generated_board_proves`: 183.2 | 1.72 |
| `build` (reference) | 0.2 | — | — |
| `campaign` (2k2, reference ELF) | 3.0 | — | — |
| `emit` | 1.2 | — | — |

- **One V costs 191 to 258 CI s, about 215 on average,** read from the CI-measured tests: (382.4 + 515.7 + 416.1 + 453.5 − about 6 s of `build`/`campaign`/`emit`) / 8 = 220 for 2k2; 218.2 and 199.6 for 47k.
- **The flags add nothing:** each variant is within 2 s of plain V.
- A V is ERC, DRC, extract, two Kani proofs, `build`, `simulate` and a one-scenario `campaign`, yet `build` is 0.2 s and `campaign` 3.0 s. **Kani dominates:** one proof alone is 106.4 s of V's 123 s. I did not measure how V's two prove gates split that time.
- The ratio varies from 1.52 to 2.08 between tests run on different CI runners. That is the runner-to-runner variance `AET-WO-1908` measured (1.5×), not a pipeline difference.
- **Savings below are booked in CI s from the CI-measured tests (one V = 215), never from Furnace × a ratio.**

**`attribute_seeded`, in one process, every output line stamped** (`~/.cache/aet-1909/attr-times.sh`; the `ANSWER 3100`/`3200` lines give the split):

| run | Furnace wall s | CI s (1908) |
|---|---|---|
| the whole block (12 tests, 4 threads, as its shard runs it) | 305.9 | 475, 505, 575, 721 (four runs) |
| `p3_gate_clause_2` alone (`--exact`) | 303.8 | 631.9 |
| `ran()` alone, through `oq_03` | 67.8 | 146.0 |

The block's timeline:
- 0 to 68.0 s: `ran()`, 10 firmware builds and 10 campaigns. The unmutated firmware's 11 scenarios end at 33.2 s, and each mutant's single scenario takes about 3.2 s.
- 68.0 to 72.4 s: the other five `ran()` tests finish, in parallel.
- Re-gate 1 (spec `54aaef1c…`, the 2,700 Ω proposal shared by six cases): 68.4 to 182.8 s, of which Kani is 72.4 to 179.5 s (107 s).
- Re-gate 2 (spec `aa4d7c62…`, 1,200 Ω): 184.8 to 299.4 s, of which Kani is 187.5 to 296.1 s (109 s).
- The 44 `propose` calls fill the gaps, to 305.9 s.

**The two re-gate proofs, alone, in series and together** (`~/.cache/aet-1909/two-proofs.sh`; the crates `generate` writes from the two proposed specifications, with `prove --spec`):

| | wall s | container memory peak |
|---|---|---|
| one after the other | 109.2 + 108.1 = 217.3 | 5.48 GB (`memory.peak`; sampled `memory.current` max 5.31 GB) |
| at the same time | 122.8 | 11.28 GB (`memory.peak`; sampled max 10.42 GB) |

Both pass with `harness_provenance: generated` and `cover_count: 9`. The peaks include page cache. The cap was 15 GiB, read back as `memory.max=16106127360`. A GitHub runner has 16 GB for everything.

**Whole binaries in one process** (`~/.cache/aet-1909/bin-times.sh`; `--test-threads=1` gives each test's own share; the worktree's `target/` is removed first, so `firmware()` builds cold, as on a fresh runner):

| binary | 4 threads, s | serial, s | CI alone sum (1908), s | the shared fixture, measured |
|---|---|---|---|---|
| `twin_tier2` | 37.7 | 101.8 | 211.7 | `firmware()`: its first user takes 2.9 s, the same as a mutant-build test, so the build is about 1 s |
| `campaign_tier2` | 20.7 | 39.9 | 110.4 | `first()`: 6.1 s for its first user, then 0.0 s for the next three |
| `with_tools` | — | 36.1 | 44.8 listed | none. Every `kani-smoke` proof takes under 1 s, and the heaviest test is `prove_of_an_unresolvable_dependency…` at 11.2 s |
| `adversarial_1301_tools` | — | 74.4 | 111.6 | none |
| `adversarial_1302_tools` | — | 4.2 | 6.6 | none |

"Alone sum minus in-process wall" is **not** a saving. A shard already runs its tests on four threads in one process, so the difference is parallelism every shard gets anyway. **A colocation saves only the shared fixture that each extra shard pays again.** That is what is booked below.

## 1. The census table (§1.1)

Abbreviations for the expensive work:
- **V(2k2)**: one `aethelgard verify --spec examples/sentinel-01-minimal.json --scenarios scenarios/sentinel-01/pullup-2k2.json`. It runs the whole pipeline: ERC, DRC (KiCad), extract, Kani `prove` ×2, `build` (a cargo firmware build), `simulate` (Renode), and a one-scenario `campaign` (Renode and the bridge).
- **V(47k)**: the same with `pullup-47k.json`.
- A flag after V, for example V(2k2, `--fab-out`), adds that flag. `--fab-out` and `--against-fab` also run kicad-cli's Gerber, drill and placement export.
- **P(x)**: one Kani `prove --crate x --harness proofs::every_valid_sequence`.
- **RAN**: `attribute_seeded`'s fixture: 10 firmware builds and 10 campaigns over 22 scenarios (22 Renode boots).
- Class: pass (pins a pass), fail (a seeded bad input fails, naming its finding), forgery, residual, determinism, refusal.

### `board_tier2` (18 tests, all tier 2; heavy tests serialise on `ONE_AT_A_TIME`)

| test | CI s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|
| `the_generated_board_proves` | 183.2 | `prove` without `--spec` on the committed reference crate gives `pass`, no finding, Kani 0.68.0 (`:109-110`). | pass | P(boards/sentinel-01) ×1 |
| `a_wrong_alternate_function_fails` | 127.1 | Seeded AF 4→5 in `board.rs` fails as `proof_failed` "GpioB word 8: a solved pin's field is not what the specification fixes" (`:103`). | fail | P(seeded copy) ×1 |
| `a_missing_clock_enable_fails` | 179.7 | Deleted RCC_APB2ENR bit 12 fails as `proof_failed` "Spi1 accessed before RCC_APB2ENR bit 12 is set" (`:103`). | fail | P(seeded copy) ×1 |
| `a_write_past_the_block_fails` | 178.9 | TXDR offset 0x28→0x2c fails as `proof_failed` "register access past the end of its block" (`:103`). | fail | P(seeded copy) ×1 |
| `a_vacuous_harness_fails` | 101.8 | A seeded `kani::assume(false)` (Kani exits 0) is `cover_unsatisfied`, verdict fail (`:159-161`). | forgery | P(seeded copy) ×1 |
| `build_gives_one_sha256_at_two_paths` | 1.2 | One crate built at two path lengths gives one ELF sha256, named in the verdict (`:319-322`); a build script, an outside `include_bytes!` and a bad cargo config are refused. | determinism | `build` ×5 |
| `prove_with_spec_tells_a_hand_written_harness_from_the_generated_one` | 16.1 | With `--spec`, a hand-written vacuous harness that passes Kani is `harness_provenance: supplied, cover_count 0`, and the committed crate's generated harness `i2c1_transfer_is_bounded_and_ordered` is `generated, 7`. | forgery | `prove --spec` ×2, each on a small harness (a vacuous one; `i2c1_transfer_is_bounded_and_ordered`), not `every_valid_sequence` |
| `verify_binds_the_reference_and_two_runs_print_the_same_bytes` | 382.4 | **MULTI (F1):** `verify --against` the committed tree prints the same bytes twice, binds every file's hash, gate order and verdicts, uid boundaries, the ELF `build` gives, and the run id `campaign` gives. | pass, determinism | V(2k2, `--against`) ×2; `build` ×1; `campaign` ×1; `emit` ×1 |
| `verify_fails_the_47k_pull_up_with_the_campaigns_findings` | 218.2 | `verify` on 47k exits 1, with the campaign gate failing on exactly `analog_rise_exceeds_ceiling` and `i2c_timeout` (its own exit 0) and every earlier gate passing (`:509-528`). | fail | V(47k) ×1 |
| `as_root_a_third_uid_cannot_change_what_build_reads` | <0.5 | While `build` runs, a third uid cannot overwrite, plant into or rename what it reads (every try `EACCES`), and the ELF is the reference's (`:636-658`). | forgery | `build` ×2 |
| `a_cargo_configuration_another_uid_plants_in_tmp_is_not_read` | 0.5 | A `/tmp/.cargo/config.toml` planted by another uid neither fails `build` nor changes the ELF. | forgery | `build` ×2 |
| `verify_refuses_a_board_swapped_before_extract` | 4.2 | A board swapped in verify's scratch between DRC and extract is `BindingMismatch`, exit 5. | forgery | `verify` (no scenarios), ERC and DRC only |
| `every_corpus_file_the_reader_accepts_gets_its_stated_kicad_verdict` | 51.3 | Every corpus file the strict reader accepts gets exactly its stated exit and verdict from `check drc` and the parity gate. | fail/refusal table | `check drc` × each corpus entry |
| `verify_fab_out_writes_the_package_its_verdict_names_at_any_path` | 515.7 | **MULTI (F1):** `--fab-out` writes exactly the 15 files its verdict names by sha256, the same bytes at two paths. The BOM is `emit`'s and the manifest binds the verdict without `fab`. The package carries the B.Cu GND pour and no Gerber clock, and FAB-NOTES lists every file. | pass, determinism | V(2k2, `--fab-out`) ×2; `emit` ×1 |
| `verify_against_fab_finds_the_package_it_wrote_identical` | 416.1 | `--against-fab` on a package `--fab-out` just wrote is `identical`, `derived`, same `fab.files`, no `differences`. | pass | V(2k2, `--fab-out`) ×1; V(2k2, `--against-fab`) ×1 |
| `verify_against_fab_names_every_changed_and_extra_file` | 453.5 | One flipped byte in each of 15 files, plus one extra file, gives exit 1 with 15 `against_differs` and 1 `against_extra`, each named. | forgery | V(2k2, `--fab-out`) ×1; V(2k2, `--against-fab`) ×1 |
| `verify_fab_out_writes_nothing_when_a_gate_fails` | 199.6 | A failing `verify` (47k) writes nothing into `--fab-out`: `not_written`, `files {}`, the directory empty. | fail | V(47k, `--fab-out`) ×1 |
| `every_admitted_identifier_character_survives_every_output` | 251.1 | One specification carrying all 83 admitted identifier characters passes `verify --fab-out`, and every output (BOM, placement, Gerbers, manifest, notes) carries each name intact. | pass | V(renamed spec, `--fab-out`) ×1 |

### `attribute_seeded` (12 tests in the tier-2 binary; whole on one shard by `ci/colocate.tsv`)

| test | CI s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|
| `the_manifest_seeds_twenty_failing_cases_eight_per_domain_and_the_three_traps` (tier 1) | <0.5 | The seeded manifest keeps ≥ 20 failing cases, ≥ 8 per domain, unique names, and the three traps with their stated domain and rule. | other | none |
| `every_mutant_applies_exactly_once_to_the_generated_board_as_committed` (tier 1) | <0.5 | Each of the 9 mutants' `find` text occurs exactly once in the committed board, so a generator change cannot un-seed a case. | other | none |
| `a_patch_that_matches_zero_or_two_times_is_refused_naming_the_mutant` (tier 1) | <0.5 | `patch()` refuses zero, two and comment-only matches, naming the mutant. | refusal | none |
| `every_unmutated_case_is_a_scenario_whose_own_expectation_is_the_manifests` (tier 1) | <0.5 | Each unmutated case's scenario expects what the manifest says, and every seeded scenario is some case's. | other | none |
| `every_committed_proposal_is_pull_up_changes_named_for_its_scenario` (tier 1) | <0.5 | **Asserts nothing today (F2):** `proposals/` is absent, so it returns at `:256-258`. | other | none |
| `tier2::every_case_fails_the_way_its_manifest_says` | 146.9 | Each of the 22 runs reports exactly its manifest's finding list (`:468`). | fail | RAN |
| `tier2::p3_gate_clause_1_attribute_names_every_case_with_evidence_its_rule_allows` | 151.2 | `attribute` names each run's manifest domain and rule, with evidence only of the kinds that rule allows (`:499-523`). | fail | RAN; `attribute` ×22 |
| `tier2::the_trap_the_wrong_address_shares_i2c_timeout_with_47k_and_is_told_apart` | 146.4 | The wrong-address 3k3 trap is software/S1, wrong-address 47k masks kind 13, and unmutated 47k masks nothing (`:552-565`). | fail | RAN; `attribute` ×4 |
| `tier2::attribute_twice_is_identical_and_a_one_byte_change_is_refused_as_replay_refuses_it` | 154.7 | `attribute` is byte-identical twice on all 22 runs, and a one-byte trace change is refused exactly as `replay` refuses it (`HashMismatch`, exit 3). | determinism, forgery | RAN; `attribute` ×45; `replay` ×1 |
| `tier2::oq_03_the_reference_runs_bytes_with_kind_15` | 146.0 | **None of its own (§3):** it looks up two cases and prints what `ran()` already prints. | other | RAN |
| `tier2::p3_gate_clause_2_propose_answers_every_seeded_case_as_its_rule_says` | 631.9 | `propose` answers each of the 22 cases as its rule says (proposal, generator report, model slot, below floor, INV-05), byte-identical twice, and each proposal re-gates through every gate with its rise within 2 %. | fail, pass, determinism | RAN; `propose` ×44; `every_gate` × distinct proposed specs (validate, solve, generate, ERC, DRC, P(generated, `--spec`), `build`, `campaign`, `attribute`) |
| `tier2::every_committed_proposal_reruns_to_its_own_bytes_and_regates` | <0.5 | **Asserts nothing today (F2):** `proposals/` is absent, so it returns at `:793-796` without calling `ran()`. | other | none |

### `twin_tier2` (22 tests, all tier 2; `firmware()` and `two_bme()` are `OnceLock`s)

| test | CI s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|
| `renode_s_files_are_the_pinned_ones_and_declare_the_stock_table` | <0.5 | Renode's installed platform files hash to `RENODE_FILES`, and the stock file declares each `STOCK` peripheral at its pinned model and address. | pass | none |
| `the_emitted_platform_loads_and_lists_every_solved_peripheral` | 3.7 | `simulate --platform` of the committed platform lists every solved peripheral. | pass | Renode load ×1 |
| `the_reference_firmware_boots_and_two_boots_agree` | 12.4 | The reference firmware boots to the fixed point with its identity read witnessed, and two boots give identical verdicts. | pass, determinism | `firmware()`; `simulate` ×2 |
| `a_boot_too_short_to_reach_the_fixed_point_fails` | 6.4 | A 10 µs boot is `boot_word_missing`, not `cpu_halted`. | fail | `firmware()`; `simulate` ×1 |
| `a_firmware_with_no_vector_table_at_the_boot_address_fails` | 3.9 | A firmware linked at 0x08010000 is `boot_word_missing` and `cpu_halted`. | forgery | mutant build ×1; `simulate` ×1 |
| `a_core_that_leaves_the_firmware_after_the_boot_word_fails` | 4.4 | A jump to FMC after the boot word is exactly `pc_outside_firmware`. | forgery | mutant build ×1; `simulate` ×1 |
| `an_aborted_machine_returns_inside_the_bound_as_cpu_aborted` | 32.4 | A jump into a hole aborts Renode, and `simulate` returns inside 35 s as exactly `cpu_aborted`. | fail | mutant build ×1; `simulate` ×1, until the wall bound |
| `p2_gate_clause_2_analog_half_47k_fails_and_2k2_passes` | 25.7 | On real Renode with the bridge, 47k is `analog_rise_exceeds_ceiling` with `i2c_timeout`, and 2k2 passes, twice the same. | fail, pass | `firmware()`; `emit` ×2; `simulate` ×4 |
| `a_read_no_model_answers_fails_as_a_timeout_at_txis` | 6.4 | A read of 0x77 is exactly `i2c_timeout` at `timeout_txis`, naming U2 and 0x76. | fail | mutant build ×1; `simulate` ×1 |
| `a_value_one_bit_off_fails_as_an_identity_mismatch` | 6.4 | A recorded 0x160 is exactly `i2c_identity_mismatch` (expected 0x60). | forgery | mutant build ×1; `simulate` ×1 |
| `a_platform_without_the_bme280_leaves_a_txis_timeout_in_the_record` | 6.6 | Run on Renode directly, the platform with the BME280 records OK 0x60, and without it records TXIS. | pass/fail pair | `firmware()`; Renode ×2 |
| `a_firmware_that_skips_the_read_or_its_record_cannot_pass` | 8.8 | A firmware that skips the read, or one that skips its record, cannot pass. | forgery | mutant build ×2; `simulate` ×2 |
| `a_forged_record_without_a_transaction_fails_unwitnessed` | 6.4 | A perfect record with no `transfer` is exactly `i2c_transaction_unwitnessed`. | forgery | mutant build ×1; `simulate` ×1 |
| `a_firmware_that_writes_the_controller_registers_by_hand_fails_unwitnessed` | 5.3 | Hand-written CR2 and TXDR with a forged record is `i2c_transaction_unwitnessed`. | forgery | mutant build ×1; `simulate` ×1 |
| `a_forgery_through_every_register_or_a_wrong_address_fails_unwitnessed` | 13.2 | Every I2C1 register hand-set to the reference's values, or a real transfer to 0x77, is `i2c_transaction_unwitnessed`. | forgery | mutant build ×2; `simulate` ×2 |
| `on_a_bus_with_two_devices_each_read_is_witnessed` | 25.9 | With two BME280s, the honest firmware has one witness per device, and forging either read is `i2c_transaction_unwitnessed` naming that device. | pass, forgery | `two_bme()`; mutant build ×3; `simulate` ×3 |
| `a_firmware_parked_in_its_fault_handler_fails_naming_vectactive` | 6.5 | A core parked in its fault handler is `core_in_exception` "exception 15". | forgery | mutant build ×1; `simulate` ×1 |
| `a_firmware_that_rewrites_its_loop_in_flash_is_not_parked` | 8.3 | A firmware that rewrites its `yield` loop in flash is `pc_not_on_self_loop`. | forgery | mutant build ×1; `simulate` ×1 |
| `the_judge_s_own_reads_of_a_gated_peripheral_are_never_the_firmware_s` | 6.2 | The judge's own reads after BEGIN never raise `locked_peripheral_access`. | fail (negative) | `firmware()`; `simulate` ×1 |
| `a_locked_access_late_in_a_flood_reaches_the_judge_whole` | 10.2 | A locked GPIOC read late in a 37.7 MB flood is still `locked_peripheral_access`, with `renode_log_flood` a verdict, not exit 5. | forgery | mutant build ×1; `simulate` ×1 |
| `a_device_named_for_a_renode_instance_or_keyword_breaks_the_load` | 4.7 | A device named `nvic` or `using` is Renode's `platform_load_error` (E02/E00), the measurement behind `validate`'s refusal. | residual | Renode load ×2 |
| `renode_s_configuration_differs_from_its_default_in_synchronous_logging_alone` | 8.0 | Renode's default configuration equals `RENODE_DEFAULT_CONFIG`, and the stated configuration is read and never rewritten. | pass | `firmware()`; Renode ×2 |

### `campaign_tier2` (8 tests, all tier 2; `first()` is a `OnceLock`)

| test | CI s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|
| `p2_gate_clause_2_the_campaign_passes_2k2_and_fails_47k_as_each_expects` | 8.6 | One campaign of 2k2 and 47k on the reference firmware matches each scenario's expectation, 47k with exactly its two findings. | pass, fail | `first()`: `build` ×1; campaign(2k2, 47k) ×1 |
| `p2_gate_clause_2_the_47k_trace_shows_the_rise_the_fault_and_the_timeout_as_three_events` | 13.5 | The archived 47k trace shows the rise, the bus fault and the TXIS timeout as three events in order. | fail | `first()` |
| `p2_gate_clause_3_replay_twice_is_identical_and_equals_the_committed_hash` | 13.2 | `replay` of the 47k run is identical twice and equals the manifest's committed `state_hash`. | determinism | `first()`; `replay` ×2 |
| `p2_gate_clause_3_a_one_byte_change_is_refused_even_with_the_manifest_resealed` | 9.0 | One changed trace byte is `HashMismatch`, and with the manifest resealed is `ReplayDiverged`. | forgery | `first()`; `replay` ×2 |
| `p2_gate_clause_3_the_same_campaign_twice_gives_identical_hashes_per_scenario` | 17.4 | A second campaign into a fresh archive gives the same hash, run id and trace bytes per scenario. | determinism | `first()`; campaign(2k2, 47k) ×1 more |
| `the_two_reference_droops_behave_as_predicted_twice_byte_identical_and_replay` | 22.5 | **MULTI (F1):** the tolerated droop passes and BOR3 resets and recovers as `AET-WO-2301` predicted, twice byte-identical, and both replay. | pass, fail, determinism | `build` ×1; campaign(droop ×2) ×2; `replay` ×4 |
| `renodes_reset_recipe_is_pinned_step_by_step` | 12.3 | Renode's reset recipe behaves as pinned, step by step (run on Renode directly). | residual | `build` ×1; Renode ×4 |
| `a_flooding_firmware_reaches_a_verdict_and_its_message_is_the_same_twice` | 13.8 | A log-flooding firmware reaches a verdict, and its message is the same twice. | forgery | flood fixture build ×1; `simulate` ×2 |

### `with_tools`, `adversarial_1301_tools`, `adversarial_1302_tools` (23 + 6 + 2 tests, all tier 2; no shared state)

| test | CI s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|
| `with_tools::drc_clean_board_passes` | <0.5 | `check drc` on the clean fixture is pass, no findings, input hash and severities echoed. | pass | KiCad DRC ×1 |
| `with_tools::drc_violation_board_fails_with_exactly_one_clearance_finding` | 0.6 | The violation fixture is exactly one `clearance` naming NET_A and NET_B. | fail | KiCad DRC ×1 |
| `with_tools::erc_clean_schematic_passes` | <0.5 | `check erc` on the clean fixture passes with no findings. | pass | KiCad ERC ×1 |
| `with_tools::erc_dangling_label_fails_with_one_finding` | 0.5 | The dangling label is exactly `label_dangling`. | fail | KiCad ERC ×1 |
| `with_tools::prove_verifies_the_in_range_harness` | <0.5 | Kani passes `add_in_range` with no findings. | pass | P(kani-smoke) ×1 |
| `with_tools::prove_fails_the_seeded_overflow` | <0.5 | `add_overflows` is exactly `proof_failed` "attempt to add with overflow". | fail | P(kani-smoke) ×1 |
| `with_tools::prove_reports_a_compile_error` | <0.5 | A crate that does not compile is exactly `compile_error`. | fail | P(kani-compile-error) ×1 |
| `with_tools::prove_reports_a_missing_harness` | <0.5 | A missing harness is exactly `harness_not_found`. | refusal | P(kani-smoke) ×1 |
| `with_tools::prove_of_an_unresolvable_dependency_names_the_package` | 11.1 | An unresolvable dependency is `ToolError` exit 5, naming the package and the tool's error. | refusal | P(fixture) ×1 |
| `with_tools::prove_leaves_the_crate_untouched` | <0.5 | `prove` leaves nothing new beside its input crate. | other | P(kani-smoke) ×1 |
| `with_tools::simulate_loads_the_reference_board` | 2.2 | `simulate` of the Nucleo platform passes on Renode 1.17.0 with one `cpu`. | pass | Renode ×1 |
| `with_tools::simulate_fails_a_platform_that_does_not_load` | 1.4 | A platform with a missing include is `platform_load_error`, although Renode exits 0. | fail | Renode ×1 |
| `with_tools::each_tool_removed_from_path_is_named` | <0.5 | With a tool off `PATH`, the CLI is exit 4 `ToolMissing` naming exactly that tool. | refusal | none |
| `with_tools::mcp_check_drc_passes_the_clean_board_and_fails_the_violation_without_is_error` | 1.2 | Through `mcp`, `check_drc` gives pass and fail as exit codes, never `isError`. | pass, fail | KiCad DRC ×2 |
| `with_tools::mcp_prove_fails_the_seeded_overflow_without_is_error` | <0.5 | Through `mcp`, a failing proof is a verdict, not `isError`. | fail | P(kani-smoke) ×1 |
| `with_tools::mcp_simulate_loads_the_nucleo_board` | 2.3 | Through `mcp`, `simulate` passes without `isError`. | pass | Renode ×1 |
| `with_tools::the_emitted_reference_schematic_passes_erc` | 2.1 | The schematic `emit` writes for the reference passes ERC. | pass | `emit` ×1; ERC ×1 |
| `with_tools::identifiers_with_s_expression_characters_survive_kicad` | 1.9 | S-expression-significant names pass ERC and read back verbatim from KiCad's own netlist. | pass | `emit`; ERC; netlist |
| `with_tools::a_board_whose_own_files_are_looser_is_judged_by_the_gate` | 1.5 | A board whose own project files allow 0.05 mm still fails DRC on the gate's floor. | forgery | KiCad DRC ×2 |
| `with_tools::with_a_token_free_setup_the_project_step_is_load_bearing` | 1.2 | Without the project step, a looser pad escapes DRC, and with it the pad is caught (D5's measurement). | residual | KiCad DRC ×2 |
| `with_tools::p1_gate_clause_1_the_reference_board_passes_erc_and_drc_with_no_finding` | 7.1 | The reference's emitted schematic and routed board pass ERC and parity DRC with no finding. | pass | `emit`; ERC; DRC |
| `with_tools::every_pad_carries_the_net_kicads_netlist_gives_its_pin` | 2.9 | Every pad's net is the net KiCad's netlist gives its pin. | pass | `emit`; netlist |
| `with_tools::parity_fails_a_board_that_does_not_match_its_schematic` | 8.6 | Each of five board/schematic mismatches fails parity with KiCad's finding for it. | fail | DRC ×6 |
| `adversarial_1301_tools::attack_a_board_only_footprint_does_not_pass_parity` | 1.6 | A board-only footprint never passes parity. | forgery | `emit`; DRC ×2 |
| `adversarial_1301_tools::attack_two_unconnected_pins_joined_on_one_net_do_not_pass_parity` | 1.7 | Two unconnected pins joined on one net never pass parity. | forgery | `emit`; DRC ×1 |
| `adversarial_1301_tools::attack_wide_references_leave_the_emitted_board_clean` | 77.7 | 24 parts with 64-byte references emit a board KiCad's DRC finds clean, beyond `unconnected_items`. | forgery | `emit` of 24 parts ×2; DRC ×2 |
| `adversarial_1301_tools::kicad_parity_does_not_see_metadata_only_differences` | 8.2 | Five metadata-only differences still pass parity; red if KiCad starts to see one. | residual | `emit`; DRC ×5 |
| `adversarial_1301_tools::parity_fails_every_other_mismatch_with_kicads_finding` | 16.2 | Each mismatch in the table fails with exactly the finding kinds KiCad gave when measured. | fail table | DRC × 21 |
| `adversarial_1301_tools::a_schematic_kicad_cannot_load_is_no_verdict` | 6.2 | A schematic the reader accepts but KiCad cannot load is `ToolError`, never a pass. | forgery | `emit`; DRC ×5 |
| `adversarial_1302_tools::kicad_does_not_judge_the_net_a_track_is_labelled_with` | 3.0 | Relabelling VDD copper as GND still gives no finding (residual A2). | residual | `emit`; DRC ×1 |
| `adversarial_1302_tools::kicad_does_not_see_a_floating_loop_on_a_net_no_pad_carries` | 3.6 | A floating loop on a pad-less net still gives no finding (residual A3). | residual | `emit`; DRC ×1 |

### Tier-1 tests of 10 CI seconds or more

| test | CI s | the distinct failure it guards | where the time goes |
|---|---|---|---|
| `adversarial_1301::attack_a_wide_reference_stays_clear_of_every_other_part` | 60.5 | No 64-character reference text leaves the outline or overlaps another part, by a Rust model of KiCad's glyph widths. | two `emit`s of 24 parts (placer, router). The tier-2 twin above judges the same boards with KiCad's DRC instead: a different oracle, not redundant. |
| `generate_tier1::verify_against_fails_on_one_changed_byte_in_any_file_naming_it` | 43.6 | One changed byte in any of 14 files, a missing file or an extra file fails `--against`, naming it. | 15 `verify --against` runs, each regenerating the tree in memory (about 2.7 s). |
| `board_ui::misuse_does_not_compile_and_the_driver_does` (`aethelgard-svd-hal`) | 41.7 | 12 misuses of the typed driver do not compile, and the valid driver does. | trybuild compiles of 13 programs. |
| `boot_tier1::a_boot_past_the_wall_bound_fails_and_names_it` | 32.3 | A boot still running at the 30 s wall bound is `wall_bound_exceeded`. | It waits out the production 30 s bound (`renode.rs:166`). Shorter needs a production change (§7). |
| `boot_tier1::an_aborted_machine_killed_at_a_bound_is_cpu_aborted` | 11.0 | A boot killed at a bound whose log says the machine aborted is `cpu_aborted`, not the bound's kind. | A stub fills 1.3 GB to pass the 1 GiB memory cap. |
| `generate_tier1::two_generations_are_identical_and_a_used_directory_is_refused_untouched` | 10.8 | Two generations are byte-identical, and a non-empty directory, a symlink or a file is refused untouched. | 5 full `generate` runs; refusal comes after generation. |
| `boot_tier1::a_record_the_firmware_never_wrote_or_wrote_oddly_fails` | 10.1 | Five malformed identity records are each `i2c_record_invalid`. | 5 stub `simulate` runs. |

## 2. Overlap groups (§1.2)

A group is tests that run the same expensive work on the same input. Each group is keyed on the exact invocation: subcommand, specification, scenario and flags.

**A shared run saves only inside one process.** `ci/shard.sh` assigns tests by name to eight jobs, and a `OnceLock` shared by tests on four shards runs four times. So every proposal below names its colocation unit, and that unit's serial time, which `ONE_AT_A_TIME` makes a sum. How a unit is kept whole is decision D1.

| group | tests (runs today) | could share one run in one binary? | today, CI s | shared, CI s | saving, CI s |
|---|---|---|---|---|---|
| **G1.** V(2k2, `--fab-out`), identical but for an output path no assertion reads | `fab_package()` in `…finds…identical` (×1) and `…names every changed…` (×1); `verify_fab_out_writes…` (×2, two paths) | Yes. One `OnceLock` package, copied for each `--against-fab` test, gives **proposal A** (the two `--against-fab` tests). Taking `verify_fab_out_writes…`'s run `a` too gives A+. | A: 869.6; A+: 1,385.3 | A: 3 V ≈ 640; A+: 4 V ≈ 870 | **A: 1 V ≈ 215 (bookable)**; A+: 2 V ≈ 430 |
| **G2.** V(2k2) twice for byte-identity: `--against` vs `--fab-out` | `verify_binds…` (V(`--against`) ×2); `verify_fab_out_writes…` (V(`--fab-out`) ×2) | Only by merging flags: run V(`--against`, `--fab-out` a) and V(`--against`, `--fab-out` b), and assert both tests' properties on those two runs (**proposal B**). The combined invocation passes (123.3 s, measured). It drops two distinct invocations, V(`--against`) alone and V(`--fab-out`) alone. | 898.1 | 2 V + `build`, `campaign`, `emit` ≈ 385 | 2 V ≈ 513 (**decision D2**) |
| **G3.** V(47k) with and without `--fab-out` | `verify_fails_the_47k…` (V(47k) ×1); `verify_fab_out_writes_nothing…` (V(47k, `--fab-out`) ×1) | Only by merging: one V(47k, `--fab-out`) with both tests' assertions (**proposal C**). It drops plain V(47k). | 417.8 | 1 V ≈ 218 | 1 V ≈ 200 (**decision D3**) |
| **G4.** `attribute_seeded`'s `RAN` and `REGATED` | six tests share `RAN`; `p3_gate_clause_2` alone uses `REGATED` | Already shared in one process, already colocated whole. | block 720 (planned) | — | **none bookable** (§4) |
| **G5.** The two re-gate proofs inside `REGATED` | `p3_gate_clause_2` (P(generated, `--spec`) ×2, one after the other: one loop on one thread, with `REGATED` locked across `every_gate`, `:699`, `:744`, `:848-857`) | They are not the same invocation: the two specifications differ, and the two generated crates differ only in the provenance comment naming the specification's sha256 (measured with `diff -r`). **Proposal E** restructures the test so the distinct specifications' `every_gate` runs on two threads, without holding `REGATED` across it, and `ONE_PROOF` admits two proofs. Measured: 122.8 s against 217.3 s, peak memory 11.3 GB against 5.5 GB. | block 720 | ≈ 497 (est.: 720 × (305.9 − 217.3 + 122.8) / 305.9) | ≈ 223 off the critical path (**decision D5**) |
| **G6.** `campaign_tier2`'s `first()` (campaign of 2k2 and 47k on the reference) | five tests (`…passes_2k2_and_fails_47k…`, `…three_events`, `…replay_twice…`, `…one_byte_change…`, `…same_campaign_twice…`) | Already a `OnceLock`. Colocating the binary (**proposal D**) makes CI pay it once instead of on up to five shards. | 110.4 (alone sum) | ≈ 69 (est.: serial 39.9 Furnace s × 1.72) | ≈ 31 (`first()`'s five tests sit on 4 shards of today's plan: 3 × 6.1 Furnace s × 1.72; est.) |
| **G7.** `twin_tier2`'s `firmware()` | six tests | Already a `OnceLock`; the build costs about 1 s. | — | — | about 5 s: **not proposed** |
| **G8.** `emit` of the reference specification | 10 calls across `with_tools`, `adversarial_1301_tools` and `adversarial_1302_tools` | 1.2 s each. | — | — | about 12 s across three binaries: **not proposed** |
| **G9.** Kani on the board crate | `the_generated_board_proves` and four seeded `prove`s in `board_tier2`; two in every V; two in `p3_gate_clause_2` | Five distinct crates in `board_tier2`. The V and re-gate proofs carry `--spec` and run on other bytes. **No two are the same invocation.** A proof cache would be a production change (§7). | — | — | **none** |

What each proposal keeps:
- **A** keeps every invocation and every assertion. Two tests read one package, each from its own copy, so `…names every changed…`'s byte flips cannot touch `…identical`'s input.
- **B** and **C** keep every test and every assertion by name, but each asserts on a run that carries one flag more than today.
- **D** changes no test.
- **E** restructures `p3_gate_clause_2`'s re-gate loop to run the distinct specifications on two threads, and lets `ONE_PROOF` admit two. Every assertion stays.

## 3. Redundancy (§1.3)

**One candidate; nothing else claimed.**

**`attribute_seeded::tier2::oq_03_the_reference_runs_bytes_with_kind_15`: redundant except for one lookup, which guards no product failure.**
- **Its body** (`:595-604`) looks up the cases `unmutated-2k2` and `pullup-47k` and prints `trace_bytes` and `event_count`. It asserts nothing itself.
- **Its only assertions are `ran()`'s** (`:418`, `:433`, `:439`, `:443`). Every other `ran()` test runs the same assertions on the same output.
- **Mutant M1** (production, `campaign.rs:465`, `check: "campaign"` → `"campaign!"`) is killed by both `oq_03` and `every_case_fails_the_way_its_manifest_says`, at the same assertion: `attribute_seeded.rs:433:17` (`left: "campaign!"`, `right: "campaign"`). Run in a scratch copy, `~/.cache/aet-1909/mut/m1.log`.
- **Its one catch no other test makes:** renaming the case `unmutated-2k2` in the manifest. No other code names it (`grep -rn unmutated-2k2`). That is a lookup, not a product failure.
- **What it prints is already printed.** `ran()` already writes `ANSWER 3100: case <name> trace_bytes=… event_count=…` for all 22 cases (`:446-449`), the two it prints included.
- **Removing it saves no wall time,** because `ran()` is shared in its block. It would lower `TIER2_MIN` to 90. **Decision D7.**

**Candidates examined and rejected; no redundancy claimed:**
- **`the_generated_board_proves` against the prove gates inside V.** It is the only pass-pin of `prove` without `--spec`, on the committed crate in place, with the Kani version asserted. V proves the generated crate with `--spec` in scratch.
- **`verify_fails_the_47k…` against `verify_fab_out_writes_nothing…`.** Different assertions: the campaign's findings and every earlier gate passing, against nothing written. It is an overlap (G3), not redundancy.
- **The cross-binary pairs, each with a different oracle or input:**
  - `adversarial_1301`'s wide reference (a Rust model of KiCad's glyph extents) and `adversarial_1301_tools`'s (KiCad's DRC). Same boards, different oracle.
  - `campaign_tier1` against `campaign_tier2`: stub Renode, stub bridge and a 64-byte ELF, against the real ones. The tier-1 tamper tests edit a stub run's instruction count; the tier-2 one edits a real trace byte and the last bit of a real rise.
  - `boot_tier1`'s aborted and wall-bound stubs against `twin_tier2`'s real abort. Only the tier-2 test asserts `cpu_aborted` together with the elapsed bound.
  - The four identifier-character tests:
    - `adversarial_1300::withstood_…` takes 83 characters through the schematic and BOM.
    - `with_tools::identifiers_with_s_expression_characters_survive_kicad` reads 11 names back through KiCad's netlist.
    - `p1_gate_clause_1` uses 7 of them through ERC and DRC parity.
    - `board_tier2::every_admitted_identifier_character_survives_every_output` takes 83 through every fabrication output.
    - Each reaches an output the others do not.
- **Every forgery and residual test** is distinct by construction (§1.3), and none pins the same forgery as another.

## 4. `attribute_seeded` (§1.4)

- **Why its tests overlap in one process:**
  - Six of its seven tier-2 tests start with `ran()`, a `OnceLock` of 10 firmware builds and 10 campaigns (22 scenarios). The first caller pays and the rest wait.
  - `p3_gate_clause_2` then runs 44 `propose` calls and, through `REGATED`, `every_gate` once per distinct proposed specification: two (2,700 Ω for six cases, 1,200 Ω for `cap-220p-2k2`).
  - Each `every_gate` is validate, solve, generate, ERC, DRC, a Kani proof under `ONE_PROOF`, `build`, a campaign and `attribute`.
  - Run alone, each of the six would pay `ran()` again. That is why 1,377 s alone becomes 475 to 721 s together.
- **What a shared fixture would save:** nothing more. It is already shared, and the block is already kept whole by `ci/colocate.tsv`.
- **`p3_gate_clause_2` inside the block and alone:**
  - Measured on Furnace: 303.8 s alone and 305.9 s as the whole block, the block ending when it ends.
  - So the block's wall time is `p3_gate_clause_2`'s own chain: `ran()` 68 s, then two re-gates of about 115 s each, 107 to 109 s of each being Kani, then `propose`.
  - The other five tests add nothing to the wall time.
  - **This differs from `AET-WO-1908` FEEDBACK's "about 382 s inside the block".** That FEEDBACK does not say where the figure came from. My inference is libtest's per-test `--report-time` in a shared process (its run 1), which `AET-WO-1908` itself says not to trust for weights. On CI the block took 475 to 721 s and the test alone 632 s, so the same holds there within runner variance.
- **The one lever that shortens it without dropping an invocation is E:** the two re-gates at once, which means restructuring the test's loop and letting `ONE_PROOF` admit two. It would bring the block to about 211 Furnace s (est. 497 CI s), but the measured memory peak doubles to 11.3 GB on a 16 GB runner. `ONE_PROOF`'s comment ("each proof peaks at about 3.5 GB") is why one proof at a time is the rule.
- **Not a lever:** the two re-gate crates differ only in a provenance comment, but `prove --spec` checks provenance against the specification, so one proof cannot stand for the other.

## 5. Predictions (§1.5)

The eight-shard plan by `ci/shard.sh assign`, unchanged, with `ci/test-weights.tsv` as committed:
- Every moved test is set in a colocated unit weighed at the shared run's CI seconds above.
- The `attribute_seeded` block is 720, or 497 under E.
- `ci/SHARDS` stays 8.
- The script is `~/.cache/aet-1909/predict.sh`. It re-plans 1,171 entries from the image's own test list.

| proposals taken | plan maximum, CI s | planned total, CI s | change in total |
|---|---|---|---|
| none (today) | 720 | 5,166 | — |
| A | 720 | 4,936 | −230 |
| B | 720 | 4,654 | −512 |
| C | 720 | 4,969 | −197 |
| D | 720 | 5,130 | −36 (the planner books the alone sum, 110, against the block, 69; the measured saving is about 31, because `first()`'s tests sit on four shards today) |
| E | 618 | 4,944 | −222 |
| B + C + D | 720 | 4,416 | −750 |
| A + B + C + D | 720 | 4,186 | −980 |
| B + C + D + E | **524** (all eight shards) | 4,192 | −974 |
| A + B + C + D + E | 640 (the A block alone on a shard) | 3,963 | −1,203 |

- **The suite's CI seconds** (the plan's total, which is what the eight shards run) fall from 5,166 to about 4,190 with B, C and D (and A or E), that is 16 to 19 %.
- **A shard's wall time is its planned seconds plus setup** (102 to 145 s, `AET-WO-1908`).
  - Shards holding the serial `verify` tests ran 12 to 20 % over plan.
  - The block shard varied 1.5× between runners.
- **Predicted slowest shard:**
  - Today's plan: about 11.4 to 14.2 minutes. The three accepted runs measured 13:01, 14:48 and 14:12.
  - Without E: unchanged, because the block is the maximum.
  - With B, C, D and E: 524 × 1.2 + 145 ≈ 774 s, about 13 minutes at worst and 11 typical.
  - With A as well, the 640 s block becomes the maximum again: about 15 minutes at worst.
- **These are predictions from a plan, not measurements.** §4 asks for three CI runs at the build.

## Findings for the architect

- **F1 (§7: a guarded failure that one sentence cannot state).** Three tests each pin several unrelated properties of one run. I give each a sentence in the table, but each sentence is a list:
  - `board_tier2::verify_binds_the_reference_and_two_runs_print_the_same_bytes`: ten properties at `:402-487`, including byte-identity, file hashes, gate order, uid boundaries, the ELF and the run id.
  - `board_tier2::verify_fab_out_writes_the_package_its_verdict_names_at_any_path`: eleven, at `:862-979`.
  - `campaign_tier2::the_two_reference_droops_behave_as_predicted_twice_byte_identical_and_replay`: about nine groups.

  This is a finding, not a deletion. Under B, the first two share their runs and keep every assertion by name. It bears on §2.1 of the build ("every property still asserted by name"): splitting them into named tests over one shared run would make each property its own test, and raise `TIER2_MIN` with no new CI cost.
- **F2. Two counted tests assert nothing in this tree.** `proposals/` does not exist, so both return before any assertion:
  - `attribute_seeded::every_committed_proposal_is_pull_up_changes_named_for_its_scenario` (tier 1, counted by the canary, `:256-258`);
  - `tier2::every_committed_proposal_reruns_to_its_own_bytes_and_regates` (`:793-796`).

  `TIER2_MIN` 91 therefore counts two vacuous passes and `oq_03`. **The distinct failures the 91 tests guard number 88.** I have not removed or changed them: they guard committed proposals if any are ever added. Whether `proposals/` should exist is the architect's.
- **F3. `oq_03` (§3).**
- **F4. Withdrawn.** A first reading put about 11 s of Kani compile on the first `kani-smoke` proof. It came from a parser that dropped interleaved result lines (the trap `ci/shard.sh`'s refit rule names). Re-parsed, every smoke proof takes under 1 s, and their weights of under 0.5 s stand.
- **F6. A shared run that panics is not poisoned; it runs again.** Under M1, `ran()`'s `OnceLock` init panicked in the first test, and `oq_03` then re-ran the whole init, waiting over 60 s, before failing at the same assertion.
  - `std::sync::OnceLock::get_or_init` leaves the cell empty when its closure panics, so each dependent test re-runs the fixture and fails.
  - That satisfies §5.3's "every dependent test must fail, none pass". But a failing fixture costs its run once per dependent test, and a fixture that fails late could take the shard past 20 minutes.
  - A build that adds shared runs (A, B, C) should either accept this, or record the first failure (for example a `OnceLock<Result<…, String>>`), so each dependent test fails at once, naming the property.
- **F5. The census's own limits:**
  - Furnace-to-CI ratios range from 1.52 to 2.08 across tests (runner variance), so every *est.* figure carries that spread.
  - The verdict bytes of the flag variants were not compared: they ran in the container's `/tmp`, which went with it. B and C should compare them at the build before relying on them.
  - The memory peaks include page cache.

## Decisions needed

| # | decision | my recommendation |
|---|---|---|
| D1 | **How a group stays in one process.** Either (a) the group's tests move to a new tier-2 test file, colocated in `ci/colocate.tsv`, with `TIER2_RE` in `ci/shard.sh` naming it; or (b) `ci/colocate.tsv` learns to name a set of tests within a binary (a `ci/` change). Both are inside the fence. | (a): no planner change, and binaries in one shard run one after another, so `ONE_AT_A_TIME`'s memory cap holds across them. |
| D2 | **B:** `verify_binds…` and `verify_fab_out_writes…` share two runs of V(`--against`, `--fab-out`), dropping V(`--against`) alone and V(`--fab-out`) alone. 513 CI s. | Yes, if F5's byte comparison shows the combined verdict equals each single-flag verdict apart from its own keys. |
| D3 | **C:** the two 47k tests share one V(47k, `--fab-out`), dropping plain V(47k). About 200 CI s. | Yes, on the same condition. |
| D4 | **A:** the two `--against-fab` tests share one `--fab-out` package. About 215 CI s; bookable. | Yes without E. With E, A makes a 640 s block that becomes the maximum: take A only if the architect weighs total seconds over the slowest shard. |
| D5 | **E:** `p3_gate_clause_2` re-gates its two distinct specifications on two threads (not holding `REGATED` across `every_gate`), and `ONE_PROOF` admits two proofs. It shortens the critical path by about 223 CI s, but the peak is 11.3 GB of a 16 GB runner. | The architect's call on the memory risk. Without E, nothing moves the maximum, and the run stays about 13 to 15 minutes at worst. |
| D6 | **D:** colocate `campaign_tier2`. About 31 CI s; no test change. | Yes: small, and free. |
| D7 | **`oq_03`:** remove it (`TIER2_MIN` 90, its data already printed by `ran()`), or keep it as an `OQ-03` record. | Remove. |
| D8 | **F1:** split the multi-property tests into named properties over one shared run (`TIER2_MIN` rises), or leave them whole. | The architect's: it is §2.1's "every property asserted by name". |

## Not done

- No test, fixture, `ci/` or production change (§1). The mutant ran in a scratch copy, `~/.cache/aet-1909/mut/repo`, made with `git archive`. The worktree is untouched apart from this file and the gitignored `target/` the twin tests build into.
- No CI run. Every number is Furnace-measured in the pinned image, or CI seconds from `ci/test-weights.tsv`.
- The adversarial pass (§5) and the reduction pass belong to the build.

## Where the evidence is

`~/.cache/aet-1909/`:
- the runner `img.sh`, and `caps.log`, which holds every run's caps read back;
- `verify-times.log`, `attr-times.log`, `twin-times.log`, `campaign-times.log` and `two-proofs.log`;
- `*-serial.log`, the serial runs;
- `mut/m1.log`, the M1 mutant;
- `entries.tsv`, the image's test list; `plan/`, every prediction's shard files;
- `census/*.md`, the per-binary rows with file:line for every claim.
