# AET-WO-1910 — FEEDBACK

**Status: census done (§1), STOPPED for the ruling on PR #233.**
- No test, fixture, `ci/` file or workflow changed. This file is the branch's only change.
- Every saving below is measured: from CI's own logs of three runs on post-`1314` code, or from runs in a scratch copy, never from the worktree.
- One task came in between: `AET-WO-1605c` (main red through `bench-package`), done on its own PR, #232.

**Branch:** `wo/aet-1910-ci-reduction` · **Base:** `b14be5bf` (`origin/main`; `565b32c5` is the last code change) · **Builder:** Claude (Opus 5.5, orchestrating).
- The census rows were written by five Sonnet subagents, each on one schema, and checked against the source and the measured emit log (Method).
- `AET-WO-4400g` has not merged. Its files (the twin, chaos, motor tests) are in the census as main has them, and the build would leave them alone.

# Part 1: the census (§1)

## Answer first

1. **Where CI pays twice is mostly one thing: the heavy tier-1 tests run in both jobs.**
   - Seven tier-1 binaries (`supply_tier1`, `power_board_tier1`, `outline_tier1`, `boot_tier1`, `escape_tier1`, `via_ring_tier1`, `campaign_tier1`) cost 3,229–3,699 s of the `tools` shards over three runs.
   - The same seven cost 1,647–2,866 s of the unsharded `gates` job.
   - The jobs build the same code with the same compiler but run it as different users: root in the image, the runner's user on the host (§1.4).
     - **For the five that compute through `emit` and `extract`, the second run buys only their figures on the host's glibc.**
     - `boot_tier1` and `campaign_tier1` drive the sandbox, which takes a different path as root, so each job tests a path the other does not.
   - **Running the five once, in `tools` only (D5 (a)), takes `gates` from 38–66 min to about 15–27 min** and leaves the shards where they are.
2. **Inside one process, three groups repeat an expensive run on the same input** (§1.2). Each was measured in a scratch prototype or from CI's own per-call times:
   - **A:** `power_board_tier1` routes `power_tree()` through the ladder ten times; two runs suffice. Measured on 4 CPUs: the binary goes from 271.6 to 184.4 s, −32 %.
   - **B:** `outline_tier1` emits the committed Sentinel-03 three times; one suffices. 182.7 to 153.1 s, −16 %.
   - **C:** two `board_tier2` tests each make the same four-layer `verify --fab-out`. One shared package saves one `verify`, 372–530 CI s measured.
   - Each needs its tests kept in one process (`ci/colocate.tsv`), as `1909` did.
3. **Redundancy: two tests, both tier 1, each with mutant evidence** (§1.3).
   - `campaign_tier1`'s `a_one_byte_change_to_the_trace_is_a_hash_mismatch` and `an_extra_file_in_the_run_directory_is_refused` pin what `attribute_refuses_whatever_replay_refuses_with_the_same_bytes_and_code` pins, on the same corruption.
   - Four mutants each fail both. The saving is small, about 5 CI s per test per job.
   - Nothing else was found redundant: every other same-input pair asserts another property or uses another oracle.
4. **The planner's silence is wider than `1314` found** (§1.5).
   - Today 1,876 listed tests have no weight row. The plan books them at 94 s; they took up to 702 s on one run.
   - 17 of them take 10 s or more, the heaviest 58.9 s (`1303b`'s `supply::tests` units).
   - 7 weight rows name tests that no longer exist: renamed tests, whose new names fell to the default.
   - **Proposed:**
     - `plan` refuses a row that names no listed test;
     - `result` refuses an unweighted test that libtest itself reports as running over 60 seconds.
     - Dry-run on CI's logs, the second fires by name on `a9b79f43` and is silent on 21 of today's 24 shard logs. The other three are one test that waits in a lock queue (§1.5).
5. **Predictions** (§1.6, the eight-shard plan by `ci/shard.sh assign` on CI's own test list):
   - today: plan 2,141 s per shard;
   - A + B + C, with the refit's honest rows for every measured test (§2.4): 2,137;
   - with the heavy tier-1 out of `tools` instead: 1,656.
   - **The honest weights the refit adds (162 rows, +689 s) cost about what A, B and C save.** The one lever that moves a shard is where the heavy tier-1 runs (D5).
6. **The cheapest saving is outside this fence (F2, D7).**
   - The stub `simulate` and `campaign` calls cost 4.5–10 s each because the dev-built CLI parses and hashes ST's 2.1 MB header on every call.
   - Building `aethelgard-svd-hal` and `sha2` at `opt-level = 2` in dev builds, as `aethelgard-eda` already is, took `boot_tier1` from 120.8 to 31.5 s and `campaign_tier1` from 47.1 to 9.1 s (measured, 4 CPUs).
   - It is a root `Cargo.toml` change.
7. **Findings, in Findings below:**
   - two savings that need production or build-profile changes (§7: proposed, not built);
   - 42 tests whose guarded failure is a list, not a sentence (§7);
   - `bench-package`'s blind spot on PRs (the architect's finding, D8);
   - smaller items.
8. **Decisions needed:** D1 to D10 (below).

## Method

**Baseline: three CI runs on post-`1314` code**
- `739b4a24` and `e6679f12` are `1314`'s last two heads; `e6679f12` adds FEEDBACK only. `565b32c5` is main after `1314` merged, the same code.
- Suite seconds are, as in `1909`, the sum of the eight `tools-shard` jobs' test step ("cargo test inside the image, this shard's tests"), from the GitHub API.

| run | head | suite s (test steps) | shard jobs, min | `gates` job | its `cargo test` step |
|---|---|---|---|---|---|
| 44426540986 | `739b4a24` | 16,910 | 32.7–49.3 | 38.2 min | 2,156 s |
| 44433378021 | `e6679f12` | 14,415 | 30.0–40.0 | 65.9 min | 3,740 s |
| 44441889769 | `565b32c5` | 16,312 | 32.0–45.3 | 51.4 min | 2,925 s |

- The plan is 2,141 s on every shard (8 × 2,141 = 17,128 planned). The measured suite is 0.84–0.99 of plan.
- **`gates` varies 1.7× on the same code.** Every binary in it was 1.6–2.0× slower on `e6679f12`'s runner, so the spread is the runner, not a test.
- **The kickoff's "66 min on the READY head" is that runner.** The other two runs took 38.2 and 51.4 min.
- `gates` runs its binaries one after another, so its test step is the sum of their `finished in`. It was 2,051, 3,579 and 2,807 s against steps of 2,156, 3,740 and 2,925 s; compiling is the rest.

**Per-test CI seconds.** These come from the runs' own logs (`~/.cache/aet-1910/tools/pertest.py`), not from `ci/test-weights.tsv`, which mixes alone times, in-shard worsts and Furnace estimates.
- **How a test's time is reconstructed:**
  - libtest starts a binary's tests in name order on 4 threads, and each completion starts the next queued test. So test *j* starts at completion *j* − 4, and its time is its `ok` less that start.
  - The parse accounts for every test: 2,470 per `tools` run and 2,258 per `gates` run, each equal to the job's own pass count.
  - **Checked per binary run:** the reconstructed times sum to at most 4 × its `finished in`, and its last `ok` falls at its `finished in`. That holds for 2,128 of 2,130 binary runs. The two exceptions are `adversarial_1301` on run 44426540986's shard 1, whose lines reached the log in one flush (no per-test times; the other runs carry it), and one `routing_tier1` run, 2 s off.
  - In a binary that serialises on a lock (`board_tier2`, `verify_2k2_tier2` and `against_fab_tier2` on `ONE_AT_A_TIME`; `twin_tier2` on `RENODE_TURN`), a test is its `ok` less the previous completion in its process: REFIT RULE 5's "less the previous lock-holder's".
  - **`twin_tier2` takes its lock around each tool call, not each test, so its per-test figures still include queueing.** Its binary total is exact.
  - Where tests wait on one shared run (`attribute_seeded`'s `ran()`, `supply_tier1`'s `OnceLock`s, the two `verify_47k_tier2` tests), each waiter's figure includes the wait.
- **A test is in the census at 10 s or more** in either job's worst run: 236 tests over the three runs of each job.

**What each test does, and how many times.** Five read-only subagents each wrote one group's rows to one schema (`~/.cache/aet-1910/census/BRIEF.md`): the guarded failure in one sentence with its `file:line`, class, expensive work, shared state, same-input groups and same-assertion candidates. I checked their claims against the source:
- the four-layer verifies (`board_tier2.rs:983`, `:1035`);
- the reference's `--fab-out` runs (`verify_2k2_tier2.rs:78-79`, `against_fab_tier2.rs:42`);
- `RENODE_TURN` (`twin_tier2.rs:69`);
- `pin_table`'s parse;
- the campaign pairs (`campaign_tier1.rs:1185-1232`, `:1598-1640`);
- and every emit count, against the measured emit log below.

**Overlap, measured, not read.** In a scratch copy (`git archive`; `~/.cache/aet-1910/scratch/repo`), `schematic::emit_moved` and `schematic::placement` were made to log, per call:
- the sha256 of the specification's `Debug` form;
- the number of moves;
- the result (the board's sha256, ladder or not, or the error's kind);
- the wall time and the process.

The whole tier-1 suite then ran as `gates` runs it (4 test threads, capped; 2,258 passed, 0 failed): 315 calls. The ladder binaries were run again pinned to 4 CPUs (`taskset -c 0-3`), as a runner has, because the ladder routes eight rungs on threads and Furnace's 20 cores hide that. Groups are keyed on that hash, so a mutated specification, a size sample or a renaming is a different input automatically.

**Mutants** ran in fresh `git archive` copies under `~/.cache/aet-1910/mut/`, each capped. Each filter was checked to match its tests: the unmutated base passes all three.

## 1. The census table (§1.1)

The full rows are in **Part 3**: 270 tests, one sentence each, with `file:line`. They cover the 236, plus 34 `twin_tier2` tests that fall under 10 s once its lock is accounted for (Method). Where the time goes, by binary:

| binary | `tools` s, summed over its shards (3 runs) | `gates` s (3 runs) | the expensive work |
|---|---|---|---|
| `board_tier2` | 3,888–4,180 | — | `verify`, Kani, KiCad, one at a time on `ONE_AT_A_TIME` |
| `against_fab_tier2` | 1,900–2,356 | — | three reference `verify`s (one shared `--fab-out`) |
| `verify_2k2_tier2` | 1,908–2,159 | — | four reference `verify`s |
| `attribute_seeded` | 933–1,429 | 18–34 (tier 1 only) | `ran()`: 14 builds, 14 campaigns; two re-gates |
| `twin_tier2` | 1,126–1,364 | — | Renode boots, 9 Kani proofs, firmware builds, on `RENODE_TURN` |
| `supply_tier1` | 1,165–1,287 | 391–795 | the supply gate (`extract`), about 250–390 CI s per run, four runs; two ladder emits behind `OnceLock`s |
| `power_board_tier1` | 632–745 | 475–797 | 17 ladder emits, 10 of one input |
| `outline_tier1` | 477–520 | 311–536 | 10 ladder emits, 3 of one input; 21 placements |
| `verify_47k_tier2` | 429–530 | — | one 47k `verify` |
| `escape_tier1` | 326–395 | 121–200 | 21 renamed-order emits (no ladder) |
| `boot_tier1` | 335–375 | 183–370 | stub `simulate` calls, about 4.5–10 s each; one 30 s wall bound |
| `via_ring_tier1` | 182–230 | 98–173 | 2 ladder emits and an over-bound ladder |
| `campaign_tier1` | 112–147 | 70–148 | stub campaigns: `a_run()` 16 times, `a_47k_run()` 5 |

**Tier 1 against tier 2 in `tools`.** The tier-2 binaries took 10,413–12,294 s and the tier-1 binaries 3,965–4,571 s, of which the seven heavy ones 3,229–3,699 s.

## 2. Overlap groups (§1.2)

**A shared run saves only inside one process.**
- `gates` runs each binary whole, so a `OnceLock` pays in full there.
- `tools` assigns tests by name, so a binary's tests sit on several shards and each shard pays the shared run again, unless the binary is colocated (`ci/colocate.tsv`).
- Each proposal below names its unit and that unit's size against the plan's 2,141 s mean and the 2,400 s stop.

| group | the repeated work (measured) | what can share it | saving, measured | proposal |
|---|---|---|---|---|
| **G1 `power_board_tier1`** | `power_tree()` through the ladder (input `fc7f9ed3`) 10 times in 9 tests, plus once in `solve_cli` (another binary). | 8 of the 10. **Two must stay independent:** `every_loop_rule_holds_on_the_fixtures_own_board` asserts that two emits are equal (`:68`), so its second emit is the property itself. `the_crossing_is_drawn_and_its_limits_are_refused_by_name`'s pair (`:339-343`) is likewise left alone. | Prototype `OnceLock` per input (scratch), 4 CPUs: binary **271.6 → 184.4 s (−32 %)**, 16/16 pass. In CI s: `gates` 475–797 → about 323–542; `tools` 632–745 (split) → one block of about 542 | **A**: shared emit, binary colocated (block 542) |
| **G2 `outline_tier1`** | committed Sentinel-03 through the ladder (`5d219835`) 3 times: `order_only…`, `the_fixture_fits…`, `the_fixture_places…` | all 3. `order_only…`'s one full emit (ruling 6987450886) can be the shared one: it compares that emit with placements, not with a second emit. | Prototype, 4 CPUs: **182.7 → 153.1 s (−16 %)**, 9/9 pass. `gates` 311–536 → about 261–450 | **B**: shared emit, colocated (block 450) |
| **G3 Sentinel-03 across binaries** | the same input in `outline_tier1` (3), `supply_tier1` (1, `OnceLock`) and `via_ring_tier1` (1, `OnceLock`): 65 s per emit on 4 CPUs | only by moving the tests into one binary, or by a cross-process cache | 2 emits, about 130 s on 4 CPUs | **not proposed** (D9): tests are filed by work order, and a cross-process cache is a new mechanism |
| **G4 four-layer `verify --fab-out`** | `four_layer_fixture_passes_every_gate_and_its_package_holds` and `against_fab_names_a_missing_inner_layer_and_a_stackup_that_lies` each run `four_verify(dir, "--fab-out")` (`board_tier2.rs:983`, `:1035`): the same spec, scenarios and flag, only the output directory differs. Then each runs one `--against-fab` on its own copy. | `1909`'s A pattern: one `--fab-out` package, copied per test, in a new tier-2 binary under `ONE_AT_A_TIME` | **One `verify` of 372–530 CI s** (each run's `ANSWER 3301 … wall_s`: 529.4, 514.2, 514.3 and 372.1, 440.3, 455.9) | **C**: new binary, colocated (block about 1,590 = 3 × 530) |
| **G5 the reference's `--fab-out`** | three identical runs: `verify_fab_out_writes_the_package_its_verdict_names_at_any_path`'s run *a* and *b* (two paths, which is its property) and `against_fab_tier2`'s shared package | `1909`'s A+: the shared package from run *a* | one `verify`, about 500 CI s | **not proposed**: the block would be about 4 `verify`s ≈ 2,100 s, at the plan mean. `against_fab_tier2` left the colocated blocks in `4230` for that reason |
| **G6 the supply gate on one board at two limits** | `extract` with the supply gate on `s024l_board()` at 60 mV (`a_drop_over_its_limit…`) and at 165 mV (`the_four_layer_board_bounds…`). The network solved is the same and only the limit differs: about 250–390 CI s each | only inside `extract` (a production cache) | about 300 CI s per job | **§7, not proposed** (F1) |
| **G7 the full power fixture** | 3 ladder-path emits (`6a3bb08d`): 2 in the crossing test (its equality pair) and 1 in `via_ring_tier1` | none: the pair is a determinism check | — | none |
| **G8 `twin_tier2`'s builds** | 11 firmware builds of 4 distinct inputs (serial-four ×3, dshot600 ×3, pwm ×3, sentinel-02-own ×2); the reference `simulate` at 10 ms ×3 | in one process only, and `twin_tier2` is split over the shards | **not measured.** `1909` measured `firmware()`'s build at about 1 s; under the noise | **not proposed** |
| **G9 `campaign_tier1`'s stub runs** | `a_run()` 16 times, `a_47k_run()` 5 times, no `OnceLock`. Most tests then corrupt their run directory. | a shared run copied per test (as G4) | **not measured.** The whole binary is 70–148 s in `gates` | **not proposed**: small, and a copy changes what the corruption tests corrupt (an in-place run against a copy) |
| **G10 the reference's plain emit** | `sentinel-01` (`4af75ab6`) 135 times across 22 binaries, 0.8 s each on Furnace; most inside `verify` and `generate` subprocesses whose own pipeline is the test's subject | none without changing what a test runs | about 108 s on Furnace, spread over 22 binaries | none |

**The ladder's share, measured.**
- Of the tier-1 suite's 935 s of emit time on Furnace, 689 s is in four binaries, nearly all of it ladder emits:
  - `power_board_tier1` 274;
  - `outline_tier1` 258;
  - `via_ring_tier1` 93;
  - `supply_tier1` 64.
- On 4 CPUs the same four spend 931, 613, 179 and 125 s of call time. The calls overlap, because the test threads run in parallel.
- 619 of `power_board_tier1`'s 931 s, and 189 of `outline_tier1`'s 613 s, repeat an input the same process already emitted.

## 3. Redundancy (§1.3)

**R1. `campaign_tier1::a_one_byte_change_to_the_trace_is_a_hash_mismatch`** is caught wholly by `attribute_refuses_whatever_replay_refuses_with_the_same_bytes_and_code`.
- **The narrow test** (`:1185-1194`): one `a_run()`; the byte at `INSTRUCTION_COUNT_CBOR + 4` of `trace.cbor` flipped in place; `replay` must refuse with exit 3 and kind `HashMismatch` (`expect_refusal`, `:422-427`).
- **The attribute test** makes the same flip in a copy of an `a_run()` run, under the same run name (`:1617-1622`).
  - Through `same(&flipped, "HashMismatch")`, it asserts that `attribute`'s refusal is exit 3 and `HashMismatch`.
  - It also asserts that `replay`'s stdout and exit code equal `attribute`'s (`:1600-1604`), so `replay`'s refusal is pinned too.
- **Mutants, each failing both tests at the refusal assertion:**
  - `chaos/src/lib.rs:111`, the kind `"HashMismatch"` → `"HashMismatchX"`;
  - `chaos/src/archive.rs:359`, the trace hash check removed.
- **What the narrow test alone has:** the run is corrupted in place, not as a copy.
  - The copy holds `trace.cbor` and `manifest.cbor` only (`:1612-1614`); the run in place keeps its other entries.
  - Nothing in `replay` reads where a run lives (`attribute_needs_nothing_but_the_run_directory` pins that location does not matter).
  - The other entries are `mirror.cbor` (and its `.tmp`). `check_entries` admits them and `replay` reads nothing else of them (`archive.rs:20-22`, `:273`), so the path to the hash check is the same with or without them.

**R2. `campaign_tier1::an_extra_file_in_the_run_directory_is_refused`** is caught wholly by the same test, through `same(&extra, "UnexpectedEntry")`: `notes.txt` added to a copy of the run (`:1626-1628`).
- **Mutants, each failing both tests:**
  - `lib.rs:101`, the kind → `"UnexpectedEntryX"`;
  - `archive.rs:280`, the unexpected entry let through (`Some(_) => Ok(())`).

The logs are `~/.cache/aet-1910/mut/{base,kindhash,skiphash,kindentry,skipentry}.log`; the base passes all three tests.

**Saving:** about one stub campaign per test, roughly 5 CI s each in `tools` and in `gates`, about 20 s a run in all. Both are tier 1, so `TIER2_MIN` does not move (D4).

**Examined and rejected; no redundancy claimed:**
- **`supply_tier1.rs:135` and `via_ring_tier1.rs:127`:** both read `extract`'s exit 0 on the Sentinel-03 board. Each test's own property is something else (no supplies without a limit; the ladder judged by `extract` alone).
- **`outline_tier1.rs:93` (the bound) and `:330` (exact size):** different tests. The first also pins the connectors on the side edges.
- **`outline_tier1.rs:331-335` (the name-free digest) and `:359-363` (emit equals placement):** the same output against two references; each catches a different change.
- **`boot_tier1.rs:681` and `:1539`:** different inputs (`good()` against the SPI witness set), and both are one call inside a larger test.
- **The cross-binary pairs `1909` examined** (a Rust glyph model against KiCad's DRC; stub against real Renode; four identifier-character tests through different outputs): unchanged. Each reaches an output, or uses an oracle, the others do not.
- **Every forgery and residual test.** None pins the same forgery as another; `campaign_tier1`'s pair above pins one forgery twice through the same oracle, which is why it is listed.

## 4. `gates` against `tools` (§1.4)

**What differs between the two jobs for a tier-1 test:**
- **The code: nothing.**
  - No production source has `cfg(feature = "with-tools")`. The feature gates only the tier-2 test files and `attribute_seeded`'s `tier2` module.
  - Both jobs build with rustc 1.99.1: `rust-toolchain.toml` on the host, and `container/Dockerfile:171-177` in the image.
- **The environment:**
  - the host runner (Ubuntu 26.04) or the image (Debian);
  - no tools on the host, where the image has KiCad, Kani and Renode;
  - descriptors above 2 closed by the step on the host;
  - each system's glibc. The router is integer arithmetic, but `extract`, `supply` and `flow` call glibc's `ln`, `exp`, `atan` and `hypot` (`flow.rs`, `extract.rs`, `supply.rs`). So the capacitance and supply figures that tier-1 goldens pin run under two libms;
  - **the uid.** `tools` runs the tests as root (`docker run` with no `--user`); `gates` runs them as the runner's user.
    - **The CLI takes different paths as root** (`AET-WO-1601`): a tool runs under its own uid (`run.rs:1283`), work directories are handed to it (`run.rs:514`, `main.rs:1306`, `:1614`), and the scratch base moves (`main.rs:647-660`). Tests assert on that path: `campaign_tier1.rs:616` expects `uid_boundary_unmet: tool_uid` when not root.
    - **`emit` and `extract` reach none of it:** `emit_design` and `extract::extract` never touch the scratch or `run::Invocation`.

**Every one of the 2,258 tier-1 tests runs in both.** What the second run buys:
- **`gates` buys:** "passes on a stock host with no tools, as a user, descriptors closed, on the host's glibc". This is what a contributor's plain `cargo test --workspace` meets.
- **`tools` buys:** "passes inside the pinned image, as root". A tier-1 test whose outcome changes with a tool present, or with the tool uid, fails there.

**The heavy seven split in two:**
- **Five are pure computation over committed fixtures,** through `emit`, `extract` and the library: `supply_tier1`, `power_board_tier1`, `outline_tier1`, `escape_tier1` and `via_ring_tier1`.
  - For them, "host or image" reduces to the libm figures.
  - The light tier-1 tests that exercise `extract`'s and `supply`'s arithmetic directly still run on the host (`aethelgard_eda`'s unit tests, `extract_tier1`, `support_tier1`).
- **Two drive the sandbox:** `boot_tier1` and `campaign_tier1` run `simulate` and `campaign` against stubs through `run.rs`. As root they take the tool-uid path, and as a user the other.
  - **Each job tests a path the other does not, so neither is a repeat.** They should stay in both.
  - D7 makes them cheap: −74 % and −81 %.

| option | `tools` plan (A + B + C + refit rows) | `gates`, predicted from each run's job less the moved binaries | lost |
|---|---|---|---|
| **(a) the five computation binaries in `tools` only; `boot_tier1` and `campaign_tier1` in both** | 2,137 | **about 15–27 min** (898, 1,605, 1,078 s); about 12–24 min with D7 | the five's figures on the host's glibc |
| (a7) all seven in `tools` only | 2,137 | about 11–18 min (645, 1,087, 777 s) | the above, and `boot_tier1`'s and `campaign_tier1`'s user-uid path |
| (b) the five in `gates` only | 1,767 | unchanged, 38–66 min, and the critical path | the five inside the image, as root, where they reach no uid path |
| (b7) all seven in `gates` only | 1,656 | unchanged | the above, and the two's root path |
| (c) both, as today | 2,137 | 38–66 min | nothing |

- **Recommended: (a).** `gates` is unsharded and has no guard of its own; at 66 min it was the slowest job in CI. Under (a), CI's wall time is about the slowest shard's, each uid path is still tested, and every light tier-1 test still proves itself on the host.
- **The mechanism is for the build.** For example:
  - `ci/` lists the moved binaries, and `gates` runs every other test target by name;
  - a guard checks that the union is the workspace's list.
  - That is a `ci.yml` change beyond `TIER2_MIN`, which the fence allows only as ruled (D5).

## 5. The planner's silence (§1.5)

**Measured on the baseline runs:**
- **1,876 listed tests have no weight row.** The plan books them at 0.05 s each, 94 s in all. They measured up to 702 s on one run.
- **17 take 10 s or more:**
  - 9 are `aethelgard_eda` units: 8 of `1303b`'s `supply::tests`, the heaviest 58.9 s (`a_relieved_pad_joins_no_plane`), and one `flow::tests`;
  - 6 are in `boot_tier1`;
  - 1 is `orientation_tier1::the_placed_imu_agrees_with_the_declared_orientation`, 40.3 s;
  - 1 is a `main.rs` renode unit, 10.4 s.
  - None takes 60 s.
- **7 weight rows name no listed test.** They are renamed tests whose new names fell to the default:
  - `boot_tier1`'s `the_injection_s_reads_are_one_whole_block_of_bare_values`, 12.9 s, is now `the_injection_s_lines_are_read_whole_and_in_place`, measured at 29.9 s;
  - three of `boot_tier1`'s and three of `main.rs`'s renode units.
- **Its plan-time arithmetic is honest.** Applied to CI's own list (run 44426540986's `entries.tsv`), `ci/shard.sh assign` reproduces CI's 2,141 s per shard exactly.

**Proposed** (each refusal names the test and exits non-zero; the 0.05 s default stays for light tests):
- **P1, at plan time:** `plan` refuses a weight or colocate row that names no listed test. It is deterministic and cheap.
  - It would fire today on the 7 rows above.
  - It catches a rename the moment it lands, instead of after a shard runs over.
- **P2, after the run, in `result`:** refuse a test that libtest itself reports as running "for over 60 seconds" (a stable, built-in line) and that has neither a weight row nor a colocated binary.
  - **Dry run on CI's logs** (`~/.cache/aet-1910/tools/rule60.py`, each commit's own weights):
    - It fires on `a9b79f43` (run 44372757248) in 7 of 8 shards, naming the 11 tests behind `1314`'s overrun: 8 in `supply_tier1`, 2 in `via_ring_tier1`, and `outline_tier1`'s `the_fixture_fits…`.
    - It is silent on 21 of the three baseline runs' 24 shard logs.
  - **The three firings are one test, and it is not heavy:** `board_tier2::the_ordered_board_passes_every_board_gate_with_no_finding` (5.8 s of its own) waits behind `ONE_AT_A_TIME` past 60 s.
    - So the rule needs every test of a lock-serialised binary to carry a row: `ONE_AT_A_TIME`'s three binaries and `twin_tier2`'s `RENODE_TURN`. The build adds them.
    - Silence on today's plan does not carry over to another plan: a light test with no row can queue past 60 s wherever a re-plan puts it beside a lock holder.
- **P3, optional and finer:** stamp each log line with bash's `$EPOCHREALTIME` and reconstruct each test's time in `result`, as `pertest.py` does. Then refuse an unweighted test over 10 s.
  - It would catch the 17 tests under 60 s that P2 does not.
  - It costs an awk port of the reconstruction (D6).
- **What P1 and P2 force is small:** the 7 stale rows, renamed, and rows for the lock-serialised binaries' unweighted tests.
  - **The 162 rows of 0.5 s or more (689 s) are not the refusal's cost.** They are §2.4's refit, the honest weights for tests that are measured and unweighted today, under any rule.
  - With them the plan rises from 2,141 to 2,226 s per shard before any saving (§1.6). P3 would refuse the 17 of them over 10 s.

**What the guard proves (§2.3, at the build):** its fixtures are the `a9b79f43` and baseline log excerpts. One test fires on the first, by name, and is silent on the second. P1's fixture is a weight file with one stale row.

## 6. Predictions (§1.6)

These are plans, by `ci/shard.sh assign` unchanged, on run 44426540986's own test list (`~/.cache/aet-1910/plan/predict.py`).
- **Block weights are measured CI seconds:**
  - A: `power_board_tier1`'s worst whole-binary 797.5 × 0.68 = 542;
  - B: `outline_tier1`'s 535.9 × 0.84 = 450;
  - C: three four-layer `verify`s at the worst measured 530 = 1,590.
- **"Refit rows"** adds the 162 rows of 0.5 s or more that §2.4's refit would write: measured today, and unweighted.

| proposals taken | plan per shard, s | planned total, s |
|---|---|---|
| today | 2,141 | 17,128 |
| the refit rows alone | 2,226 | 17,808 |
| A / B / C alone | 2,093 / 2,122 / 2,118 | 16,744 / 16,976 / 16,942 |
| A + B + C | 2,052 | 16,416 |
| **A + B + C + refit rows** | **2,137** | **17,096** |
| the heavy seven out of `tools` (D5 (b7)) | 1,616 | 12,928 |
| A + B + C + refit rows, the heavy seven out of `tools` (b7) | 1,656 | 13,248 |
| A + B + C + refit rows, the five computation binaries out of `tools` (b) | 1,767 | 14,136 |

**The suite's CI seconds, measured and predicted** (the mean of three runs is 15,879; the predictions are measured before less the measured savings):
- **A + B + C:**
  - A: `power_board_tier1` in `tools` 632–745 → one block of about 542;
  - B: `outline_tier1` 477–520 → about 450;
  - C: one four-layer `verify`, 372–530.
  - That is −640 to −1,060 s, about **14,800–15,200 s (−4 to −7 %)**.
- **With D5 (b7) as well:** a further −3,229 to −3,699 s, about **11,300 s (−29 %)**. D5 (b) would remove the five's share of that.
- **D5 (a)** moves no `tools` seconds. It removes 1,394–2,348 s from `gates`; (a7) removes 1,647–2,866 s.
- **D7 (F2)** would take about another 350 s from `tools` and 200–400 s from `gates`, from `boot_tier1` and `campaign_tier1` alone. It is not in the plans above, because those binaries' rows would need re-weighing by the refit rule after the change.

**Shard wall time.** Measured against plan, a shard's test step ran 0.75–1.27× its planned seconds over the three runs. Setup took another 190–235 s.
- At a 2,137 s plan: about 30–49 min, as today.
- At a 1,656 s plan (D5 (b7)): about 24–39 min.
- §7's line is a plan over 2,400 s or a measured shard over 50 min. A 2,137 s plan passes the plan line, but its 1.27× tail, at 49 min, is a minute from the measured one. D5 (b), D5 (b7) or D7 is what gives that margin back.

**`gates`, predicted:** D5 (a) takes it to about 15–27 min (each run's job less the five's time: 898, 1,605 and 1,078 s), and (a7) to about 11–18 min. Without D5 (a), A and B take about 200–340 s off it (G1 and G2 in full, `gates` running each binary whole): about 35–60 min.

## Findings for the architect

- **F1 (§7: a production change).** The supply gate runs twice on one board, `s024l_board()` at 60 and at 165 mV, about 250–390 CI s each, in both jobs. Only a change inside `extract` could share the network it solves. Not proposed; it is a saving the fence does not allow.
- **F2 (§7: a build-profile change, measured).** The stub `simulate` calls of `boot_tier1` and `campaign_tier1` cost about 4.5–10 s each, with no sleep or build to explain them (the group A rows).
  - The CLI parses ST's 2.1 MB `stm32h753xx.h` and hashes it on every `pin_table` and `register_facts` call, unoptimised in dev builds (`aethelgard-svd-hal/src/lib.rs:67`, `cmsis.rs:105`).
  - `aethelgard-eda` already builds at `opt-level = 2` in dev builds for this reason (`Cargo.toml`, 1302 B9).
  - Measured on 4 CPUs in scratch, `[profile.dev.package.aethelgard-svd-hal]` and `[profile.dev.package.sha2]` at `opt-level = 2` (`scratch/opt.txt`; every test passes before and after):

    | binary | before | after | change |
    |---|---|---|---|
    | `boot_tier1` (58 tests) | 120.8 s | 31.5 s | −74 % |
    | `campaign_tier1` (67) | 47.1 s | 9.1 s | −81 % |
    | `generate_tier1` (10) | 23.1 s | 14.3 s | −38 % |
    | `fab_tier1` (11) | 20.7 s | 12.6 s | −39 % |

  - **Each stanza alone, on `boot_tier1`:** `sha2` 74.2 s (−39 %) and `aethelgard-svd-hal` 64.7 s (−46 %), against 31.5 s for both (`scratch/opt-split.txt`). So both the hashing and the parse cost, and together they are most of what the stub tests' seconds were; the stubs themselves are not.
  - **The share, roughly:** In CI terms, `boot_tier1` and `campaign_tier1` cost 447–522 s in `tools` and 253–518 s in `gates` (§1.1). About three quarters of that would go, roughly 350 s in `tools` and 200–400 s in `gates` per run.
  - Every other test that runs the dev-built CLI as a subprocess gains too: `generate_tier1` and `fab_tier1` above, and the `verify`s, where Kani dominates.
  - Debug assertions and overflow checks stay on, as for `aethelgard-eda`. The release profile and every generated file are untouched. Rust does no fast-math at any opt-level, so no figure changes.
  - The root `Cargo.toml` is outside this fence (D7).
- **F3 (§7: guarded failures that one sentence cannot state).** 42 of the 270 rows are MULTI: their tests pin several properties, listed in Part 3. A3, B7, C9, D15, E8.
  - It is a finding, not a deletion. Splitting a MULTI test into named tests over one shared run keeps every assertion and raises the count; `1909` D8 ruled the same question for three tests.
- **F4 (the architect's): a change to the verdict's recorded fields is unseen on its PR.**
  - `bench-package` runs only on main's push. So `1605`'s change to every bench scenario's `state_sequence_hash` merged green and turned main red (fixed forward by `1605c`, #232).
  - The options are in D8.
- **F5. `twin_tier2`'s per-test CI seconds include `RENODE_TURN` queueing** (group B). Its REFIT RULE 5 rows ("`ok` less the binary's `Running`") are upper bounds, not costs. They do not mislead the plan, which needs them as upper bounds, but they cannot be compared test against test.
- **F6. `1909`'s FEEDBACK is stale in places** (group C): a `verify` runs 7 Kani `prove` gates and `check support`, not 2; the package has 17 files, not 15; `attribute_seeded` has 30 cases and 13 mutants.
- **F7, smaller:**
  - `kani_proves_every_part_of_both_motor_fixtures` builds two firmware ELFs it never uses (`motor_board` always builds, `twin_tier2.rs:1799`).
  - `a_stale_or_impostor_bridge_is_refused` copies the CLI binary twice instead of hard-linking it (`campaign_tier1.rs:2548-2552`).
  - The two committed-proposal tests are still vacuous while `proposals/` is absent (`1909` F2).
- **F8. main's `bench-package` was red** from `a8cdc3f8` to `b14be5bf`. The cause and the fix are in #232 (`AET-WO-1605c`).

## Decisions needed

| # | decision | my recommendation |
|---|---|---|
| D1 | **A:** `power_board_tier1` shares one ladder emit of `power_tree()` per process. The two equality pairs keep their own second emit. The binary is colocated (block 542). | Yes. |
| D2 | **B:** `outline_tier1` shares one emit of the committed Sentinel-03 per process; colocated (block 450). | Yes. |
| D3 | **C:** the two four-layer `--fab-out` tests move to a new tier-2 binary that shares one package, each test comparing its own copy, as `against_fab_tier2` does. `TIER2_RE` names the binary; `TIER2_MIN` does not move; colocated (block about 1,590). | Yes. |
| D4 | **R1, R2:** remove `a_one_byte_change_to_the_trace_is_a_hash_mismatch` and `an_extra_file_in_the_run_directory_is_refused` (mutants above). About 20 s a run. | Yes, on the mutant evidence. Scotty asked for redundancy found and removed. |
| D5 | **Where the heavy tier-1 binaries run** (§1.4's table): (a) the five computation binaries in `tools` only, with `boot_tier1` and `campaign_tier1` in both because each job tests a uid path the other does not; (a7) all seven in `tools`; (b) or (b7) in `gates`; (c) as today. | (a). `gates` falls to about 15–27 min, and about 12–24 min with D7. It needs a `ci.yml` change beyond `TIER2_MIN`, which the fence allows only as ruled. |
| D6 | **The planner:** P1 (stale rows, at plan time) and P2 (libtest's 60 s line, after the run, every lock-serialised binary's tests weighed); and P3 (time-stamped lines, a 10 s threshold), yes or no. The refit's 162 rows come with §2.4 in any case. | P1 and P2. P3 only if the architect wants the 10–60 s band refused as well. |
| D7 | **F2:** `opt-level = 2` for `aethelgard-svd-hal` and `sha2` in dev builds. It is two stanzas in the root `Cargo.toml`, outside this fence. | **Yes**, as a fence widening for this build or as its own small work order. It is the cheapest saving in the census (−74 % and −81 % on the two stub binaries) and touches no production behaviour. A test that pins it would compare the dev profile's stanzas with the measured binaries. |
| D8 | **F4:** how a PR that moves the verdict's recorded fields learns before merge: (i) `bench-package` on every PR (one `verify`, about 24–35 min, in parallel; about 1,460 runner s a run); (ii) on PRs whose paths touch `container/IMAGE`, the twin, the CLI's Renode or campaign code, or `chaos` (a changed-files step); (iii) as a tier-2 test in the shards (about 600–1,000 s more planned). | (i). It is the simplest and adds no wall time while shards take 30+ min, and it is honest: the package is compared on the PR that changes it. |
| D9 | **G3:** Sentinel-03's ladder emit shared across `outline_tier1`, `supply_tier1` and `via_ring_tier1`, by moving tests into one binary or by a cross-process cache. | No. |
| D10 | **F3:** split the 42 MULTI tests into named properties over one shared run, or leave them whole. | Leave them whole here. A split is its own work order if wanted. |

## Not done

- No test, fixture, `ci/` or workflow change (§1). The prototypes and mutants ran in scratch copies made with `git archive`. The worktree holds only this file.
- No CI run of a proposal: every "after" figure is a scratch measurement or a plan.
- The adversarial and reduction passes belong to the build.

## Where the evidence is

`~/.cache/aet-1910/`:
- `logs/`: the job logs of the three baseline runs and `a9b79f43`; `census-seconds.tsv` (every test, each run); `pertest-*.tsv`.
- `tools/`: `pertest.py`, `table.py`, `rule60.py`, `steps.sh`.
- `census/`: the brief, each group's input, and the five row files (`rows-A.md` to `rows-E.md`).
- `scratch/`: the instrumented copy, `emit-full.log` (the tier-1 suite), `emit-cpu4.log` and `cpu4.txt` (4 CPUs), `emit-proto.log` and `proto.txt` (prototypes A and B), `opt.txt` (F2).
- `mut/`: `run.sh` and each mutant's log.
- `plan/`: `predict.py` and every scenario's shard files.

# Part 3: the census rows

Each group's table follows, as its row-writer wrote it and as I checked it (Method).
- The row-writers worked from the first two runs, before `twin_tier2` was counted as lock-serialised. That set was 269 tests:
  - 34 of its `twin_tier2` rows are under 10 s on the three-run measure, and are kept;
  - one test reached 10 s only on the third run, and I added its row (`campaign_tier2`, marked).
- The seconds in the rows are the worst over runs 44426540986 and 44433378021 (`tools`), and over `739b4a24` and `e6679f12` (`gates`).
- `census-seconds.tsv` has the third run.
- Each group's shared state, same-input groups and same-assertion candidates are in its file under `census/`.
- A claim a row-writer marked "inferred" stays marked. Where I measured since, Part 1 governs:
  - Group A's per-call cost, inferred as the unoptimised dev-profile CLI, is measured in F2: 74–81 % of it goes with `aethelgard-svd-hal` and `sha2` at `opt-level = 2`.
  - Group D's ladder counts are confirmed by the emit log (§1.2).


## Census rows, group A (boot_tier1, campaign_tier1)

Repository read only, main 565b32c5. Paths are under `crates/aethelgard-cli/` unless given in full.
Test files: `tests/boot_tier1.rs` (B) and `tests/campaign_tier1.rs` (C). Times are CI seconds from the input.

### Reading guide

**FC (full call)** means one `aethelgard simulate --firmware ...` subprocess that runs to its end against the stubs. It is `boot()` at B:412, `pre_pass_on()` at B:910, `foreign()` at B:2427 and `boot_capture()` at B:1725, which all end in `run_cli` (B:380). Inside the CLI an FC is `simulate` (`src/main.rs:1734`): `ready()` (`main.rs:1431`: spec solve, ELF check, twin `platform::emit`), `analog_pass` (`main.rs:2055`: the bridge stub in the sandbox), `own_models`, a `renode --version` stub run, then the boot stub run through the reaper (`src/run.rs:510`).

A campaign scenario runs the same `crate::simulate` in process (`src/campaign.rs:601`). So in C one **campaign call** is one FC per scenario. `replay`, `attribute` and `propose` read the archive only (`campaign.rs:1428`; `propose.rs` has no tool run: grep found no `simulate`, `Invocation` or `renode`), so they are cheap.

**No stub in this group sleeps, builds or waits, except the ones named in section 5.** Every stub is a few lines of `/bin/sh` that prints a canned file (B:312-348, C:286-333). There is no cargo build of a stub: `cli()` hard-links the already built CLI (B:351-374, C:334-357).

**Where the time goes (derived from the input, not measured here):** the input gives a per-FC cost.
- `a_bridge_that_fails_or_answers_wrong_stops_the_call` is 6 FCs that stop at the bridge: 31.5 s tools, about 5.3 s each.
- `the_bridge_cannot_write_what_renode_reads` is 1 FC: 8.4 s tools, 10.1 s gates.
- `inputs_are_refused_before_renode_is_looked_for` is 6 refusals at `ready()` plus 1 FC: 12.0 s.

So an FC costs about 4.5 to 7 s in `tools` and about 8 to 10 s in `gates`. A call refused at `ready()` costs under 2 s (inferred). A test's time is about (FC count) x (per-FC cost). The code gives no sleep that explains the per-FC cost. My best reading (inferred, not measured): an unoptimised dev-profile CLI (only `aethelgard-eda` is opt-level 2, `Cargo.toml:62`), and each FC starts several sandboxed children by re-executing the CLI (reaper, then trampoline, `run.rs:525-535`), plus a guardian process (`run.rs:1217`), under CI core contention. The column "expensive work" gives the FC count so the cost can be summed.

### 1. Tables

### boot_tier1

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| any_witness_but_the_model_s_exact_lines_fails | 65.3 | 112.9 | A U2 witness that is not exactly the model's two lines, or is not a whole `[NOISY] i2c1.u2:` log line, fails as `i2c_transaction_unwitnessed` (`boot_tier1.rs:1364`, `:1383`). | fail | 13 FCs (9 table cases B:1319-1362 plus 4 shaped-text cases B:1375-1384), each `boot(&with_log(GOOD_RECORD, log))`, new stub and temp dirs each time. No shared value. Largest count in the group. |
| a_capture_that_is_not_the_pattern_then_the_frame_fails_by_name | 38.2 | 76.5 | A USART1 capture that is not its own pattern then the firmware's frame fails by a named `serial_*` kind (`boot_tier1.rs:1786`). | fail | 9 FCs (8 cases in `cases` B:1745-1779 plus the 3-byte case again B:1788), `boot_capture(&good(), bytes, "")`. |
| the_injection_s_lines_are_read_whole_and_in_place | 29.9 | 68.2 | MULTI: a missing, doubled or short injection fails `serial_echo_missing` by message (`:1932-1938`); a malformed or mis-stamped hook line is exit 5 `ToolError` (`:1961-1962`); a whole foreign log line among them is skipped and the boot passes (`:1971-1973`). | fail | 8 FCs (3 at B:1924, 4 at B:1958, 1 at B:1970), `boot_capture(&text, &capture_of(&elf()), "")`. |
| a_core_not_on_a_self_loop_fails_and_both_of_its_halfwords_pass | 48.8 | 61.7 | A PC off the `yield; b.n` loop, or a loop the machine no longer holds, fails `pc_not_on_self_loop` (`:1536`, `:1555`); both halfwords pass (`:1531`). | fail | 7 FCs through the local `at()` (B:1516): 2 passing PCs, 3 failing PCs, `0x8000040` (B:1539), and the `0x6008` rewrite (B:1550). |
| anything_beside_the_ports_files_in_the_capture_directory_is_forged | 40.5 | 59.8 | A planted file, link or directory beside the capture, a replaced port file, or a capture past 64 KiB is `serial_capture_forged`, even without the boot word (`:1814`, `:1824`, `:1833`, `:1852`). | forgery | 7 FCs (3 stub `then` plants B:1810, link-replace B:1821, 64 KiB+1 B:1830, unbooted B:1846 and B:1851). |
| a_foreign_boot_s_flash_locked_accesses_and_warnings_are_reported | 29.8 | 51.2 | MULTI: changed flash bytes or an own-sector or mass erase is `flash_overwritten` (`:2556`, `:2564`, `:2584`); another sector's erase and an option-byte change pass, the latter as `info` (`:2575`, `:2589-2599`); a missing hash line is exit-5 ToolError (`:2604`); locked accesses and `Unhandled write` warnings are counted once each (`:2614-2640`). | other | 7 FCs of `foreign()` (B:2427), each writing a new `toolbox_foreign` stub. |
| a_record_the_firmware_never_wrote_or_wrote_oddly_fails | 37.5 | 45.8 | A U2 record that is the fill, a skipped read, an unknown status or swapped words fails `i2c_record_invalid` (`:1241`). | fail | 5 FCs, one per `record` in the loop at B:1233-1245. |
| the_judge_s_own_log_after_begin_is_never_the_run_s | 29.1 | 42.7 | Log lines the judge's own reads cause, after `BEGIN`, never witness or fail the run, and a split line there is refused (`:1415`, `:1425-1427`, `:1439`). | fail | 5 FCs (B:1412, B:1422, 3 split lines at B:1436). |
| an_armed_interrupt_after_the_boot_word_fails | 39.7 | 42.6 | SysTick armed or counted to 0, or any NVIC set-enable bit after the boot word, is `register_mismatch` naming the register (`:653-655`). | fail | 5 FCs (4 loop cases B:639-657, 1 passing B:663). |
| a_write_the_bring_up_never_makes_fails_and_another_source_s_does_not_count | 32.5 | 42.0 | An unexpected write from USART1 itself, or an unparseable USART1 line, is `serial_tag_mismatch`; another peripheral's write is not counted (`:2007`, `:2021`, `:2035`). | fail | 5 FCs (3 loop B:1999, `usart2` B:2012, bad line B:2024). |
| a_boot_past_the_wall_bound_fails_and_names_it | 39.8 | 39.6 | A boot still running at 30 s wall is killed and fails `wall_bound_exceeded` (`:882`), and the kill came before the stub's 60 s sleep ended (`:886`). | fail | 1 FC whose stub ends with `/bin/sleep 60` (B:880). The reaper waits for the bound `wall: 30 s` (`src/renode.rs:219-222`), so this test takes at least 30 s by construction (`:886` asserts at least 30 s). The test calls itself "the one slow test" (B:879). |
| a_brown_out_word_that_is_not_the_declared_level_fails_naming_both | 35.3 | 39.2 | A boot-record word 1 that is another `BOR_LEV`, or the judge's fill, fails `brown_out_level_not_programmed` naming both (`:2221`, `:2224-2228`). | fail | 5 FCs (the passing `boot(&good())` B:2187, 3 loop B:2216, 1 before-boot-word B:2233). |
| the_flash_controller_is_read_once_and_never_written_or_the_boot_fails | 22.2 | 35.8 | A firmware write to the flash controller, no read, two reads, or a log not read whole is `flash_controller_access` with the exact message (`:2275-2278`). | fail | 4 FCs (the 4 `(edit, says)` cases at B:2258-2271). |
| markers_must_appear_once_each_as_whole_lines | 27.6 | 34.2 | A missing or doubled marker, a short read block, or no listing is exit 5 `ToolError` (`:539-547`). | refusal | 4 FCs, each `untrusted(boot(...))` (B:539, 540, 545, 547). |
| a_value_that_is_not_the_part_tables_fails_as_an_identity_mismatch | 28.2 | 34.1 | A U2 value other than 0x60 fails `i2c_identity_mismatch` naming U2 while its status stays `ok` (`:1202-1204`). | fail | 4 FCs (4 values B:1199, via `with_record`, B:1146). |
| a_wait_that_expired_fails_as_a_timeout_naming_the_wait | 26.5 | 34.1 | A status word of an expired wait (TXIS, TC, RXNE, STOPF) fails `i2c_timeout` naming the stage, not an identity mismatch (`:1220-1223`). | fail | 4 FCs (4 stages B:1211-1224). |
| a_bridge_that_fails_or_answers_wrong_stops_the_call | 31.5 | 30.1 | Six bridge stubs (exit 1, one line short, nothing, killed, another ngspice, nets swapped) each stop the call as exit 5 `ToolError`, id `aethelgard-bridge` (`:1018-1020`). | refusal | 6 FCs through `pre_pass()` that stop at the bridge: no `renode --version` and no boot run happen. 5.3 s each, about as long as an FC that runs the boot, so the cost is before the Renode stub (inferred from the numbers). |
| a_pin_off_its_af_does_not_count_its_bytes | 20.0 | 27.3 | An RX pin off AF7 or not in AF mode at the injection, or a TX pin off its AF at the end, makes the echo or pattern not count (`:1872`, `:1884`). | fail | 3 FCs (2 loop B:1866, 1 B:1881). |
| a_console_too_slow_for_the_boot_is_refused_before_it | 25.8 | 24.5 | A 1200-baud console whose wire time passes 100 ms is refused by name `SerialWireTimePastWindow`, exit 3 (`:2169-2170`), while the 9600-baud board is not (`:2165-2166`). | refusal | 2 `generate --spec` calls (B:2135-2146; `generate_tree` runs `aethelgard_eda::schematic::emit` plus board emission, `src/generate.rs:94-107`; on the `serial-dshot-9600` fixture and its 1200-baud copy) and 2 `simulate --for 100ms` calls, the first of which reaches the stub. The two `generate` runs are probably the heavy part (inferred; not timed). |
| a_witness_that_shows_the_read_passes_and_is_reported | 22.9 | 25.6 | A U2 witness of the model's two lines passes and is reported with `absent` addresses, also with other log lines around it (`:1288`, `:1294-1297`, `:1311`, `:1313-1316`). | pass | 3 FCs (`boot(&good())` B:1287, around B:1303, stray B:1310). |
| log_text_inside_a_value_is_refused | 24.8 | 25.6 | Log text inside a read value, a bare CR, or a `value ` prefix is exit 5 (`:530`, `:532`, `:534`). | refusal | 3 FCs. |
| a_core_ending_outside_the_firmware_fails | 22.4 | 25.5 | A final PC outside the firmware segment is `pc_outside_firmware` alone (`:674`), and one past the last loop instruction is `pc_not_on_self_loop` (`:678`). | fail | 3 FCs (B:672, B:677, B:680; the third is the same input as `at("0x8000040")` at B:1539). |
| a_failed_firmware_load_fails_and_names_itself | 23.2 | 24.5 | A Renode `LoadELF` error fails `firmware_load_error` (`:722`); the same beside reads, or an error in another command, is exit 5 (`:725`, `:727`). | fail | 3 FCs. |
| the_virtual_time_is_an_integer_unit_up_to_100_ms | 14.6 | 24.0 | A `--for` that is not an integer with a unit up to 100 ms is a usage error, exit 64 (`:802`), and a legal one is not (`:807`). | refusal | 12 CLI calls: 9 refused in argument parsing (exit 64, B:799) and 3 FCs that run to the stub (B:804). |
| a_rise_unfinished_in_the_window_or_a_line_stuck_high_is_a_fault | 14.4 | 21.4 | An unfinished rise or a line stuck high is a design fault with verdict `fail`, never exit 5 (`:1087-1090`, `:1123-1125`). | fail | 2 `solve --file` calls (`variant()` B:1040-1052, `cli(None)`) and 2 FCs through `pre_pass_on` (B:1086, B:1122), plus 2 in-process `platform::emit` calls. |
| the_foreign_judge_is_declared_and_its_memory_is_the_header_s_regions | 19.4 | 21.0 | MULTI: `--for` with `--foreign`, or a foreign window past its bound, is exit 64 (`:2676`, `:2678`); a generated boot's verdict names no judge (`:2681`); an ELF in D2 SRAM is `FirmwareOutsideMemory` without `--foreign` (`:2685`) and loads with it (`:2694-2695`); an ELF outside every region is refused either way (`:2698-2705`). | other | 6 CLI calls, 2 of them FCs (`boot(&good())` B:2679, foreign d2 B:2693), 2 refused at `ready()`, 2 refused in argument parsing. `boot(&good())` here is the same input as B:1287 and B:2187 (section 3). |
| a_boot_past_the_memory_cap_fails_and_names_it | 20.9 | 19.8 | A boot tree over 1 GiB is killed and fails `memory_cap_exceeded` with its peak RSS above the bound (`:851-862`), before the stub's 60 s sleep ends (`:866`). | fail | 1 FC whose stub runs `HOG` (B:845): `yes \| head -c 1300000000` into a shell variable. The reaper polls the tree's RSS every 100 ms with a full `/proc` scan (`src/run.rs:1075-1076`, `:1110-1135`). Time is the shell filling about 1 GiB plus the scan (inferred). |
| a_listing_without_a_solved_peripheral_is_untrusted | 11.7 | 19.5 | A peripheral listing missing a solved peripheral, or naming the wrong model type, is exit 5 (`:687`, `:689`). | refusal | 2 FCs. |
| an_aborted_machine_killed_at_a_bound_is_cpu_aborted | 16.7 | 19.5 | A machine that logs `This machine has aborted` and is then killed at a bound fails `cpu_aborted` (`:874`). | fail | 1 FC with the same `HOG` stub as B:849 (B:845, B:871). |
| a_register_off_the_solved_image_fails | 17.4 | 19.3 | A register not on the solved image (PA7 on AF6, an unsolved clock) is `register_mismatch` twice (`:617`), bits outside a pin mask do not count (`:628`). | fail | 2 FCs. |
| more_instructions_than_the_run_holds_or_another_speed_is_untrusted | 15.9 | 18.4 | More executed instructions than the run holds, or a speed other than the platform's 100 MIPS, is exit 5 (`:587`, `:595`). | refusal | 2 FCs. |
| a_halted_core_fails_and_a_set_word_beside_one_is_untrusted | 17.4 | 18.0 | A halted core fails `boot_word_missing` and `cpu_halted` (`:575`); a set word beside a count one short is exit 5 (`:576`). | fail | 2 FCs. |
| a_core_handling_an_exception_fails_naming_vectactive | 13.7 | 17.0 | ICSR `VECTACTIVE` 15 fails `core_in_exception` naming it (`:1500-1503`), and bits above the field do not (`:1511`). | fail | 2 FCs. |
| an_spi_read_whose_eot_wait_expired_fails_as_a_timeout_naming_eot | 16.4 | 17.0 | SPI status `0x22020006` fails `spi_timeout` naming EOT (`:2320-2322`), and the same word on I2C is `i2c_record_invalid` (`:2325`). | fail | 2 FCs. |
| two_devices_witness_their_own_reads_in_either_order | 12.0 | 16.4 | On a bus with two modelled devices each read is judged against its own device's lines, in either order (`:1650`, `:1656`), and one device's lines alone leave U2 unwitnessed (`:1660`). | pass | 1 `generate --spec two-bme.json --out` (B:1573, the eda schematic and board emit) and 3 FCs from `boot_two` (B:1645), each with its own stub. |
| inputs_are_refused_before_renode_is_looked_for | 12.0 | 15.5 | Six bad inputs (x86 ELF, junk, far, boot-word load, edited platform, `@` platform) are refused with exit 3 and their kinds before a missing Renode is looked for (`:740-741`); with good inputs the missing tool is exit 4 (`:782`). | refusal | 7 CLI calls: 6 refused in `ready()` (no bridge run) and 1 that reaches the missing-tool exit after the bridge pre-pass. |
| a_dshot_board_boots_the_whole_100_ms | 15.0 | 14.7 | A DShot board is not refused at 100, 3 or 2 ms (reaches the missing tool, exit 4, `:1713`) and `--for 101ms` is refused as on any board (`:1717-1718`). | pass | In process: spec parse, `pin_table`, `ir::solve`, `platform::emit` once (B:1683-1700). Then 4 CLI calls: 3 that go through `ready()` and the bridge to exit 4, and 1 refused in argument parsing. |
| a_foreign_boot_no_model_witnesses_fails_by_name | 9.1 | 14.3 | A foreign boot with no declared device's identity line fails `foreign_not_booted` and `i2c_transaction_unwitnessed` (`:2503-2504`), and a data read before its identity does not count (`:2512-2513`). | fail | 2 foreign FCs. |
| the_slave_mode_error_is_exempt_only_after_spi_lld_stop_s_writes | 11.3 | 14.2 | The SPI slave-mode `[ERROR]` is exempt only after a `CFG2` write with MASTER 0 (`:2531`), any other error is `renode_error_log` (`:2538`). | fail | 2 foreign FCs. |
| a_log_past_the_filter_s_budget_leaves_every_read_unwitnessed | 7.3 | 10.7 | A log past the 8 MiB filter budget leaves every witness unread and fails the six listed kinds plus `renode_log_flood` (`:1460-1471`). | fail | 1 FC whose stub output holds `.repeat(160_000)` log lines, about 10.4 MB (B:1451), read through `LOG_FILTER` (`src/renode.rs:238`). |
| the_bridge_cannot_write_what_renode_reads | 8.4 | 10.1 | A bridge that can write the firmware copy, platform copy, tool dir or status file exits 7; under the sandbox none can, so the call passes (`:1035`). | pass | 1 FC through `pre_pass` with the `WRITER` stub (B:1028-1032), which attempts 4 writes under Landlock. |
| a_line_past_its_ceiling_fails_names_its_bus_and_reaches_the_platform | 5.3 | 10.0 | A line past its 300 ns ceiling fails `analog_rise_exceeds_ceiling` naming bus and net, and the faulted platform reaches Renode (`:966-970`, `:974-989`). | fail | 1 FC through `pre_pass(Some(SLOW_SCL), true)`. |
| the_expected_value_is_the_tables_whatever_the_record_says_it_expects | 7.1 | 10.0 | The judge's `expected` is the part table's 0x60 whatever the firmware's record says (`:1253`). | forgery | 1 FC, `with_record(["0x22020000","0x00000061"])`, the same input as the first value in B:1199. |

### campaign_tier1

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| a_failure_for_a_superset_or_a_wrong_finding_kind_is_a_campaign_failure | 28.5 | 29.2 | A failing run whose scenario names a superset or a different finding set is not matched: exit 1, `matched` false (`campaign_tier1.rs:798-800`). | fail | 3 campaign calls (3 FCs) from `spec_run("fail", expected, PULL_UP_47K, &faulted())` (C:797), each a new `Work` and stub. |
| a_firmware_changed_while_the_campaign_runs_is_refused | 21.6 | 27.3 | A firmware file changed between two scenarios is exit 3 `FirmwareChanged`, id `pullup-47k` (`:2294-2299`). | fail | 1 campaign process with 2 scenarios: the first is one FC. `BRIDGE_SLOW` runs `/bin/sleep 3` per bridge call (C:2253). The test waits with `wait_for_the_slow_bridge`, a full `/proc` scan every 20 ms (C:2311-2330). The second scenario is refused before its bridge. About 3 s is a deliberate sleep. |
| a_droop_boot_killed_at_a_bound_is_a_verdict_and_replays | 10.5 | 23.1 | A droop boot killed at the memory cap is a verdict `brownout_unrecovered` and `memory_cap_exceeded`, exit 1, and replays as `fail` (`:2667-2674`). | fail | 1 droop campaign: rail solve through `DROOP_BRIDGE --rail`, the boot through `--renode`, the stub's `HOG` (C:2649, same as B:845) filling to the cap; 1 replay (in process). |
| a_campaign_with_one_matching_and_one_unmatched_scenario_exits_1 | 18.5 | 22.9 | One matched and one unmatched scenario make the campaign exit 1 with `matched` 1 of 2 and both runs archived (`:1357-1367`). | fail | 1 campaign call, 2 FCs (`pullup 2k2` twice with two scenario names, C:1342-1350). |
| a_record_that_claims_ok_without_the_device_logging_a_transfer_is_unrecovered | 16.7 | 21.6 | A record claiming OK without a device-logged transfer since the release is `brownout_unrecovered`, and with no transfer at all also `i2c_transaction_unwitnessed` (`:2598-2601`). | fail | 2 droop campaigns (loop at C:2589, new `droop_toolbox` each) and 2 replays. |
| the_header_names_the_renode_build_the_stub_printed_and_replay_reports_it | 15.3 | 21.0 | The archive header names the Renode build the stub printed, and replay reports it unchanged (`:550`, `:559`); a Renode naming no build, or a blank one, is exit 4 (`:574-580`). | pass | 3 campaign calls (1 good C:523, 2 refused at the build C:565, each after the bridge and the version run) and 1 replay. |
| propose_never_writes_into_the_run_it_reads | 12.1 | 17.0 | An `--out` inside the run, through a symlink into it, or with no existing ancestor is exit 3 `UnwritableOutput`, the run stays two files and replays (`:2217-2238`). | refusal | `a_47k_run()` (1 campaign call, 1 FC, C:1704) then 6 `propose` calls and 1 replay. |
| a_stale_or_impostor_bridge_is_refused | 16.5 | 16.5 | A bridge that names another source hash, or does not name itself first, is exit 4 `BridgeMismatch` before any answer is read (`:2566-2567`). | refusal | 2 campaign calls refused at the bridge (no version or boot run). Each iteration first copies the whole CLI binary: `fs::read(CARGO_BIN_EXE_aethelgard)` then `write_executable` through a `sh cat` child (C:2548-2552), so 2 full-binary copies of an unoptimised binary (size not measured). This is not a hard link. |
| the_same_campaign_into_a_fresh_archive_gives_the_same_hash | 14.3 | 14.9 | The same campaign into a fresh archive gives the same run id, state hash and trace size (`:720`). | determinism | 2 campaign calls, 2 FCs (loop `0..2` at C:700), each runs `scenarios/sentinel-01/pullup-2k2.json`. |
| propose_on_a_passing_or_indeterminate_run_proposes_nothing_and_names_the_rule | 11.4 | 13.3 | `propose` on a passing run proposes nothing and names rule N1 (`:1880-1890`); on an indeterminate run names I1 and INV-05 (`:1904-1912`). | other | `a_run()` (1 FC) plus 2 `propose` calls and a `reseal`. |
| propose_on_the_47k_run_changes_only_the_pull_up_to_2700_in_identical_bytes_twice | 11.5 | 13.3 | `propose` on the 47 kOhm run changes only `pull_up_ohms` to 2700 and writes identical bytes twice (`:1751`, `:1780`, `:1861-1873`). | pass | `a_47k_run()` (1 FC), 2 `propose`, 1 `validate`, 1 replay; the second `propose` runs with an empty PATH (C:1858). |
| propose_writes_only_into_an_empty_or_absent_directory_never_through_a_symlink | 10.4 | 12.9 | A full directory, a file or a symlink as `--out` is `UnwritableOutput` and writes nothing (`:2050-2071`); an empty directory is written (`:2076`). | refusal | `a_47k_run()` (1 FC) and 4 `propose` calls. |
| a_generator_report_names_each_generator_once_in_order | 10.9 | 12.4 | `propose` on a forged run names `board_crate` then `firmware` once each, rule S1, exit 6 (`:2122-2125`, `:2140-2155`). | other | `a_run()` (1 FC), 1 `reseal`, 1 `attribute`, 1 `propose`. |
| a_failing_run_that_matches_its_expectation_exits_0 | 8.6 | 12.3 | A failing run that matches its `expect` and findings exits 0 with `matched` true (`:750-752`). | pass | 1 campaign call (1 FC), `spec_run` (C:743). |
| a_run_with_a_consistently_resealed_but_changed_header_is_not_a_campaign_run | 11.6 | 10.2 | A run resealed with a flipped firmware hash in the header is exit 3 `NotACampaignRun` (`:1454`). | forgery | `a_run()` (1 FC), `reseal`, 1 replay. |
| attribute_names_the_47k_run_electrical_with_its_rises_and_fault | 11.5 | 10.4 | `attribute` on the 47 kOhm run is domain `electrical`, rule E1, evidence kinds 7, 7, 9 (`:1511-1519`), with the record times and the kind-15 join (`:1522-1565`). | other | 1 campaign call (1 FC) on `scenarios/sentinel-01/pullup-47k.json` with `faulted()` (C:1497), 1 `attribute`. |
| an_extra_file_in_the_run_directory_is_refused | 8.7 | 11.4 | A file `notes.txt` in a run directory is exit 3 `UnexpectedEntry` on replay (`:1225`). | refusal | `a_run()` (1 FC), 1 replay. |
| attribute_names_a_passing_run_none_twice_in_identical_bytes | 6.6 | 11.4 | `attribute` on a passing run is domain `none`, rule N1, no evidence (`:1475-1482`), and gives identical bytes twice (`:1489`). | determinism | `a_run()` (1 FC), 2 `attribute`, 1 replay. |
| attribute_refuses_whatever_replay_refuses_with_the_same_bytes_and_code | 7.8 | 11.2 | `attribute` prints replay's bytes and exit code for each refusal: `Io`, `HashMismatch`, `RunIdMismatch`, `UnexpectedEntry`, `NotACampaignRun` (`:1602-1604`, calls at `:1607-1635`). | refusal | `a_run()` (1 FC), then 6 pairs of `attribute` and `replay` (12 CLI calls), 1 `reseal`. |
| the_floor_is_judged_at_the_buss_declared_speed_not_its_ceilings | 8.5 | 11.2 | The sink floor is 1022 ohm at the declared 400 kHz, not 384 ohm at the ceiling's 1 MHz: `no_proposal`, `rp_min_ohms` 1022 (`:2190-2195`); `solve` takes 1022 and refuses 1021 `PullUpBelowSinkFloor` (`:2204-2208`). | residual | 1 campaign call (1 FC, `BRIDGE_500NS`, C:2173), 1 `propose`, 3 `solve`. Pins a ruled sizing floor from an adversarial finding (B1, C:2161-2165). |
| a_firmware_that_does_not_recover_in_the_bound_fails_and_replays | 11.1 | 10.7 | A firmware that has not rewritten its boot word at the bound fails `brownout_unrecovered`, replays as `fail`, and `attribute` says `indeterminate` (`:2510-2527`). | fail | 1 droop campaign (rail solve, `--renode` exec of the stub), 1 replay, 1 `attribute`. |
| a_run_directory_that_is_a_symlink_to_a_real_run_is_refused | 9.6 | 11.1 | A symlink to a real run is exit 3 `UnexpectedEntry` while the real one replays (`:1376-1377`). | refusal | `a_run()` (1 FC), 2 replays. |
| a_droop_scenario_renders_its_reset_through_the_bridge_in_one_command_line | 9.9 | 11.0 | A droop scenario passes through the bridge with Renode handed exactly one `-e` line (the stub exits 98 otherwise, `:2425`): exit 0, `matched` true, replay exit 0 (`:2472-2486`). | pass | 1 droop campaign, 1 replay. |
| a_held_low_sda_line_is_never_sent_to_the_bridge_and_faults_the_bus | 11.0 | 10.7 | A held-low SDA line is never sent to the bridge (the stub exits 9 if asked) and gives `i2c_line_held_low` and `i2c_timeout` (`:832-838`), replaying equal (`:841-844`). | pass | 1 campaign call (1 FC), 1 replay. |
| a_failing_run_under_an_expect_pass_scenario_exits_1_unmatched | 10.0 | 10.9 | A failing run under an `expect: pass` scenario exits 1, `matched` false, and the run is archived (`:758-769`). | fail | 1 campaign call (1 FC), `spec_run` (C:757). |
| propose_on_a_software_run_writes_a_generator_report_and_no_specification | 10.8 | 10.5 | A run whose boot word is missing yields `generator_report` rule S2, exit 6, and no proposed specification (`:1940-1953`). | other | `a_run()` (1 FC), `reseal`, 1 `propose`. |
| a_failure_for_a_subset_of_the_expected_findings_is_a_campaign_failure | 7.9 | 10.7 | A failing run whose scenario names only a subset of the findings is exit 1, `matched` false (`:780-783`). | fail | 1 campaign call (1 FC), `spec_run` (C:774). |
| a_trace_that_is_a_symlink_to_an_identical_copy_is_refused | 8.6 | 10.6 | A `trace.cbor` that is a symlink to an identical copy is exit 3 `UnexpectedEntry` (`:1387`). | refusal | `a_run()` (1 FC), 1 replay. |
| holding_both_lines_low_needs_no_bridge_at_all | 7.6 | 10.6 | Holding both lines low needs no bridge (`cli(None)`) and gives `i2c_line_held_low` and `i2c_timeout`, replaying `matched` (`:866-874`). | pass | 1 campaign call with no bridge (1 FC with both decks held, so no bridge run, `src/analog.rs:160-163`), 1 replay. |
| a_manifest_that_is_a_fifo_is_refused_without_hanging | 10.5 | 9.6 | A manifest that is a FIFO is exit 3 `UnexpectedEntry` and does not hang (`:1401`). | refusal | `a_run()` (1 FC), 1 replay under `run_guarded` (C:1254-1278: a 20 s kill guard and a 20 ms poll loop; the guard is not reached when the tool returns). |
| a_run_resealed_with_the_same_header_replays_so_the_next_test_is_not_vacuous | 8.0 | 10.5 | A run resealed unchanged replays ok with the original's state hash (`:1435-1439`). | pass | `a_run()` (1 FC), `reseal`, 2 replays. |
| a_second_campaign_into_the_same_archive_is_refused_and_the_run_is_untouched | 6.2 | 10.5 | A second campaign into the same archive is exit 3 `RunExists` and the run's two files are unchanged (`:1117-1120`). | refusal | 1 campaign call (1 FC) and 1 refused before any boot (`src/campaign.rs:521-534`). |
| a_passing_run_under_an_expect_fail_scenario_exits_1 | 9.3 | 10.4 | A passing run under an `expect: fail` scenario exits 1, verdict `pass`, `matched` false (`:807-810`). | fail | 1 campaign call (1 FC), `spec_run` (C:806). |
| an_indeterminate_run_exits_1_with_its_unexplained_finding_named | 6.0 | 10.3 | `attribute` on a run whose verdict is `renode_error_log` exits 1, domain `indeterminate`, rule I1, unexplained `renode_error_log` (`:1585-1590`). | fail | `a_run()` (1 FC), `reseal`, 1 `attribute`. |
| attribute_needs_nothing_but_the_run_directory | 5.7 | 10.3 | `attribute` on a copied run, from another cwd with an empty PATH, gives the same bytes and exit 0 (`:1656-1657`). | determinism | `a_run()` (1 FC), 2 `attribute`. |
| propose_refuses_a_scenario_or_base_that_is_not_the_runs_and_writes_nothing | 6.8 | 10.3 | `propose` refuses another scenario (`ScenarioMismatch`), an edited base (`BaseChanged`), and a base outside the cwd (`BaseOutsideRoot`), writing nothing; the same files elsewhere work (`:1960-1997`). | refusal | `a_47k_run()` (1 FC), 5 `propose` calls. |
| a_one_byte_change_with_the_manifest_resealed_diverges_on_replay | 9.4 | 10.0 | A one-byte trace change with the manifest resealed is exit 3 `ReplayDiverged` (`:1217`). | forgery | `a_run()` (1 FC), 1 replay. |


## AET-WO-1910 census, group B: twin_tier2, models_tier2, campaign_tier2

Read-only census of main at 565b32c5. Test sources: `crates/aethelgard-cli/tests/`. In a binary's own table, a bare `:N` is a line in that binary's file. All times are tools-job CI seconds from `in-B.tsv`; the gates job runs none of these (`-`).

Abbreviations. **BUILD** = one `cargo build --release --offline --locked` of a firmware crate in a fresh target dir (`mutant_of`, twin_tier2.rs:187-229; every call is a full compile, since the target dir is new). **BOOT(t)** = one `aethelgard simulate --platform --firmware --spec --for t` call: one Renode boot, plus the CLI's own `renode --version` and the analog pre-pass (main.rs:1734-1795; the Renode count per call is read from `simulate`, the pre-pass cost is inferred). **motor_board(fx)** = library solve and `board::generate_with` of `tests/fixtures/fx.json`, `platform::emit`, then ONE BUILD (:1768-1801, the build is at :1799 even when the caller only wants the platform). **adv_reference / mutant_of_board_rs / mutant** = copy of `boards/sentinel-01`, one edit, ONE BUILD (:182, :2947, :2975). **Kani** = `aethelgard prove --crate board --harness h`.

Key fact for twin_tier2 times: every `aethelgard(...)` call (solve, simulate, prove, replay, attribute, generate) takes the global `RENODE_TURN` mutex (:69, :90), as do `run_campaign` (:115) and the direct Renode runs (:860, :1432, :3800, :3944). Tests run on all CPUs, but the CLI/Renode/Kani calls are strictly one at a time. A test's CI seconds therefore include its wait in that queue. Only the cargo builds (`mutant_of`, `firmware()`) run in parallel. The file says so itself: a boot alone takes about 5 s (:64-66).

### 1. Tables

### `twin_tier2` (79 tests in this group; 87 `#[test]` in the file)

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `kani_proves_every_part_of_both_motor_fixtures` | 458.0 | - | MULTI: each motor fixture's board crate holds exactly the harness parts `_i2c1,_spi1,_tim1` (:2504), and every part proves `pass` with exit 0 (:2518-2519). | pass | 2 x motor_board (motors-pwm, motors-dshot600, label `parts`; the ELFs are built and unused) = 2 BUILD; 6 Kani `every_valid_sequence_{i2c1,spi1,tim1}` (2 fixtures x 3). No OnceLock. |
| `kani_proves_the_pwm_fixture_s_timer_start_and_its_clock_check` | 253.4 | - | MULTI: TIM1-start and reset-clock harnesses prove `pass` (:2545-2546); a board copy without the HSIDIV check fails as `proof_failed` only, exit 1 (:2571-2580). | pass + fail | 2 x motor_board(motors-pwm) (labels `prove`, `prove-no-hsidiv`) = 2 BUILD; 3 Kani (`tim1_start_writes_the_solved_values`, `reset_clock_is_ok_exactly_at_reset` x2: one on the clean, one on the edited board). |
| `on_a_bus_with_two_devices_each_read_is_witnessed` | 129.4 | - | The real firmware on the two-BME board passes with both devices witnessed (:1142-1160); a forged read of U2 or of U5 fails `i2c_transaction_unwitnessed` naming that device (:1171-1177). | forgery | `two_bme()` OnceLock: one CLI `generate` of `fixtures/two-bme.json` (:1094). 3 BUILD (real, forged-first, forged-last, :1112-1140). 3 BOOT(10ms). |
| `a2_3_a_9600_console_s_wire_time_is_judged_from_its_boot_word` | 123.0 | - | A 9600-baud console is witnessed whole with the boot word at 79 and 87 ms (:1977-1985) and fails `serial_wire_past_run` plus `serial_echo_missing`, with the exact wire-time text, at 94 ms (:1991-2000). | other (boundary: 2 pass, 1 fail) | 3 x motor_board(serial-dshot-9600) with a spin edit (:1957), 3 BUILD, 3 BOOT(100ms). |
| `the_judge_s_own_reads_of_a_gated_peripheral_are_never_the_firmware_s` | 110.5 | - | A 1 us boot reports `boot_word_missing`, and no locked access is attributed to the firmware: every finding is `boot_word_missing` or `register_mismatch` (:1303-1309). | refusal | `firmware()` OnceLock (:152; one BUILD of the reference firmware, paid by whichever test first calls it) and BOOT(1us). The test's own work is one 1 us boot (inferred: its 110 s is queue time, section 5). |
| `the_reference_firmware_boots_and_two_boots_agree` | 99.8 | - | MULTI: reference boot passes (:367-372), boot word 0x2100b007 (:375), 1,000,000 instructions (:379), seed 2100 (:382), registers (:386), i2c record U2 0x60 (:393), only finding is `device_unmodelled` U3 (:407-441), i2c witness (:418), ICSR thread mode (:426); a second boot gives the same boot and machine records (:445-451). | pass, determinism | `firmware()` OnceLock; 2 x BOOT(10ms) of the reference (`boot`, :366, :442). |
| `a_forged_spi_record_fails_unwitnessed` | 91.5 | - | A forged U4 record with no transfer fails `spi_transaction_unwitnessed` (:1541) and a transfer of another command fails `spi_step_error` + `renode_error_log` (:1559-1572). | forgery | 2 BUILD (`spi-forged`, `spi-wrong-command`), 2 BOOT(10ms). |
| `an_interrupt_armed_after_the_boot_word_fails_naming_it` | 89.2 | - | A counting SysTick or one enabled NVIC line after the boot word fails `register_mismatch` naming `SysTick.CSR` / `NVIC.ISER0` (:1249-1252). | fail | 2 BUILD (`systick-counting`, `nvic-line`), 2 BOOT(10ms). |
| `an_open_rx_line_is_detected_by_name_and_attributed_e3` | 84.2 | - | An open RX line fails by `serial_echo_missing` alone on the open port only, others witnessed (:2788-2820); `attribute` says electrical, rule E3, subject `port:<open>`, evidence kinds 28, 29 (:2827-2842). | fail | motor_board(serial-four, `serial-open`) = 1 BUILD; 1 `campaign` of 2 scenarios (open-mavlink, open-crsf) = 2 boots; per scenario `replay` + `attribute` (4 CLI calls) and `replay_dir` (library). |
| `adversarial_1_store_made_and_ug_plus_bsrr_motor_edges_fail` | 83.5 | - | Motor edges made by `BSRR` stores (timer never started) or `UG`+`BSRR` fail: `motor_anchor_unobserved`, `register_mismatch` naming `TIM1.CR1` / `TIM1.DIER`, every output `unanchored` (:2398-2415). | fail | 2 x motor_board(motors-pwm, `bsrr`, `ug-bsrr`, RAW_ADV edit) = 2 BUILD, 2 BOOT(100ms). |
| `the_four_port_fixture_s_campaign_passes_and_replays` | 70.1 | - | The serial-four campaign passes and matches (:2740-2744), archives kinds 27/28/29 four times each (:2751), replays matched (:2759), `attribute` names rule N1 (:2762). | pass | motor_board(serial-four, `serial-campaign`) = 1 BUILD; 1 `campaign` (1 scenario, 1 boot); 1 `replay`, 1 `attribute`, `replay_dir`. |
| `the_dshot600_fixture_s_campaign_passes_inside_its_window` | 66.5 | - | The DShot600 campaign and its droop scenario pass, matched, window `MAX_VIRTUAL_NS`, one judge stop at anchor+2 ms after any release, replay exit 0, `attribute` N1 (:2602-2640). | pass | motor_board(motors-dshot600, `campaign`) = 1 BUILD; 1 `campaign` of 2 scenarios (dshot600, dshot600-droop) = 2 boots; per scenario `replay` + `attribute` + `replay_dir`. |
| `the_emitted_platform_loads_and_lists_every_solved_peripheral` | 64.9 | - | The committed platform loads (exit 0) and lists every solved peripheral by name and type (:340-361). | pass | 1 CLI `solve` (:143) and 1 `simulate --platform` with no firmware (one Renode load). No build. |
| `m1_a_core_that_slept_without_its_boot_word_is_cpu_halted` | 64.0 | - | A core that slept before its outputs with no boot word is `cpu_halted` + `boot_word_missing`, naming the line (:2339-2346); its campaign run fails and replays exit 0 (:2350-2352). | fail | motor_board(motors-dshot600, `m1-wfi-no-boot-word`) = 1 BUILD; 1 BOOT(100ms); 1 `campaign` (1 scenario `dshot600`, same ELF) + 1 `replay`. |
| `every_bit_the_reference_writes_is_covered_by_a_ledger_row` | 63.9 | - | Every bit the reference firmware writes in a 100 ms boot is covered by a `written` ledger row; no uncovered or unknown peripheral (:4043-4048). | pass | `firmware()` OnceLock; 1 CLI `solve`; library `ledger::build`; 1 direct Renode 100 ms run with `LogAllPeripheralsAccess` (:3938-3956, takes the turn). |
| `an_aborted_machine_returns_inside_the_bound_as_cpu_aborted` | 63.6 | - | A jump into a hole gives exactly `cpu_aborted`, exit 1 (:550-554), and returns in under 60 s (:560-563). | fail | 1 BUILD (`hole`), 1 BOOT(10ms) that waits for the reaper (`BOOT_BOUNDS` 30 s, comment :555-559; inferred). |
| `p2_gate_clause_2_analog_half_47k_fails_and_2k2_passes` | 62.9 | - | MULTI: per pull-up, SPICE rise within tolerance, ceiling, `exceeds`, bus `SENSORS`, ngspice 44.2 (:641-663), boot word and 10,000,000 instructions (:668-674); 2k2 passes with U2 `ok` 0x60 and no faults (:729-742); 47k fails with one `bridge.fault_state`, `timeout_txis`, findings two `analog_rise_exceeds_ceiling` + `i2c_timeout` (:689-725); a second run repeats analog, faults and i2c (:746-756). | pass + fail, determinism | `firmware()` OnceLock; 2 CLI `solve`; library `emit` x2; 4 x BOOT(100ms) (2 pull-ups x 2 runs, :607-619). |
| `m1_a_core_that_slept_and_wrote_its_boot_word_is_the_ruled_disagreement` | 59.5 | - | A core that sleeps but writes its boot word is exit 5 "the boot word is set but the core executed ..." with no `AETHELGARD-` text (:2363-2369). | refusal | `dshot_mutant` on motors-dshot600: 1 BUILD, 1 BOOT(100ms). |
| `l1_bytes_written_before_the_frame_never_count_as_its_echo` | 59.2 | - | Stray `TDR` bytes before MAVLINK's frame give `serial_capture_forged` (:2293), with no `serial_injection_out_of_order` and MAVLINK injected at boot word + 100 us (:2266-2274). | forgery | `sentinel_02_chain`: motor_board(sentinel-02-shaped) = 1 BUILD, 1 BOOT(100ms). |
| `the_fixture_s_platform_lists_its_models_and_an_unpinned_one_is_refused` | 57.7 | - | The own-model platform loads and lists `ICM42688P` and `BMP581` (:3761-3768); a wrong pin hash or a missing header exits 3 `ModelUnpinned` (:3785-3786). | refusal | library `generated(sentinel-02-own.json)` + `emit` (no build); 3 `simulate --platform` (1 loads in Renode; 2 refused before Renode loads, comment :3753-3754). |
| `the_fixture_s_campaign_archives_each_configuration_and_replays` | 56.5 | - | The campaign passes and the archive holds U5's five steps `identity`, `configure 1..3`, `data`, replaying exit 0 (:3830, :3846-3855). | pass | own_board(`campaign`) library + `mutant_of` (no edit) = 1 BUILD; 1 `campaign` (`sentinel-02-own-base`, 1 boot); 1 `replay`; `replay_dir`. |
| `a_forgery_through_every_register_or_a_wrong_address_fails_unwitnessed` | 55.9 | - | A forged U2 record, after writing every I2C1 register or after a real transfer to 0x77, fails `i2c_transaction_unwitnessed` (`assert_unwitnessed` :1007-1031, called :1082). | forgery | 2 BUILD (`forge-all`, `wrong-addr-forge`), 2 BOOT(10ms). |
| `the_sentinel_02_shaped_board_is_witnessed_whole` | 55.5 | - | The Sentinel-02-shaped board is witnessed whole in one 100 ms boot: DShot windows, anchor after 39 ms and before the boot word, two ports in chain order, every I2C/SPI step `ok` (:2015-2039). | pass | motor_board(sentinel-02-shaped, `sentinel-02`) = 1 BUILD, 1 BOOT(100ms). |
| `m2_a_16_bit_or_byte_store_of_cen_anchors_and_is_stopped` | 52.1 | - | A 16-bit or byte store of `CR1` still anchors and stops the window and the board passes (`windowed_dshot`, :2200-2201). | pass | 2 x `dshot_mutant` (board.rs edit): 2 BUILD, 2 BOOT(100ms). |
| `the_serial_dshot_9600_board_is_witnessed_in_its_100_ms_boot` | 48.1 | - | The DShot600 + 9600 console board passes, outputs windowed, echo `witnessed`, frame injected at boot word + 100 us, 8 bytes (:1934-1941). | pass | motor_board(serial-dshot-9600, no edit) = 1 BUILD, 1 BOOT(100ms). |
| `the_imu_and_barometer_fixture_runs_every_script_step_witnessed` | 47.8 | - | MULTI: every I2C and SPI step has its observed value (:1635-1654), each witness observed == expected with the stated line counts (:1655-1667), two LIFECYCLE EOL banners in the verdict and in nine generated files (:1686-1709). | pass | `imu_baro_board` (library solve/generate/emit) + `mutant_of` no edit = 1 BUILD; 1 BOOT(100ms). |
| `a_platform_without_the_bme280_leaves_a_txis_timeout_in_the_record` | 47.6 | - | Direct Renode: with the BME280 declared the record holds status ok, 0x60 (:902-911); with its line removed it holds the TXIS timeout status (:915-919). | fail | `firmware()` OnceLock; 2 direct Renode 100 ms runs (`record_on`, :836-874). |
| `the_sentinel_02_fixture_with_parts_of_our_own_runs_every_step_witnessed` | 46.2 | - | MULTI: own models listed (:3662-3667), steps with datasheet-converted values (:3672-3691), witnesses 1 line for U5/U7 (:3692-3700), the one NRND lifecycle line (:3701-3709). | pass | own_board(`fixture`) library + 1 BUILD (no edit); 1 BOOT(100ms). |
| `the_four_port_fixture_witnesses_every_port_and_sbus_s_tags` | 45.9 | - | All four ports have one pattern and `witnessed` echo, silicon holds, no missing or unexpected tags, SBUS needs `CR1.M0, CR2.RXINV, CR2.TXINV`, USART write counts (:2699-2725). | pass | motor_board(serial-four, `serial-four`, no edit) = 1 BUILD, 1 BOOT(100ms). |
| `the_console_s_rx_on_the_wrong_af_keeps_the_pattern_and_loses_the_echo` | 45.0 | - | RX on AF8 keeps the pattern and loses the echo: kinds `register_mismatch` + `serial_echo_missing`, echo `not_counted`, `PA10 MODER 0b10, AF8` (:2938-2942). | fail | `mutant_of_board_rs` = 1 BUILD, 1 BOOT(100ms). |
| `m2_a_dma_start_of_cen_fails_closed_by_name` | 44.9 | - | A DMA start of `CEN` gives four `motor_anchor_unobserved`, and no flood/wall/memory bound finding, no stop (:2229-2244). | fail | `dshot_mutant` (board.rs and START edit): 1 BUILD, 1 BOOT(100ms). |
| `a_device_named_for_a_renode_instance_or_keyword_breaks_the_load` | 44.2 | - | A device named `nvic` or `using` gives exactly `platform_load_error` with Renode's E02/E00 text (:1384-1392). | refusal | 2 `simulate --platform` of edited copies of the platform (2 Renode loads that fail). No build. |
| `the_reference_reads_u4_s_jedec_id_witnessed_behind_spi1_s_multiplexer` | 44.1 | - | The reference's U4 JEDEC read is `ok` 0xef4017, witnessed by 8 lines behind `spi1.spi1mux.u4`, none unrouted (:1498-1508). | pass | `firmware()` OnceLock; 1 BOOT(10ms) (`boot`, :1496). |
| `adv_3_sbus_s_m0_cleared_before_ue_fails` | 42.7 | - | SBUS `CR1` with M0 cleared before UE: exactly `serial_tag_mismatch`, `tags_missing` = `CR1.M0` (:3111-3115). | fail | motor_board(serial-four, board.rs edit) = 1 BUILD, 1 BOOT(100ms). |
| `adv_2_a_presc_write_in_the_bring_up_fails` | 41.9 | - | A PRESC write in the bring-up: exactly `serial_tag_mismatch`, `tags_unexpected` `0x1 to 0x2C` (:3060-3064). | fail | `adv_reference` = 1 BUILD, 1 BOOT(100ms). |
| `adv2_3_a_software_reset_at_start_fails` | 41.5 | - | A SYSRESETREQ at start leaves the boot word unset: `boot_word_missing` (:3329), exit 1. | fail | `adv_reference` = 1 BUILD, 1 BOOT(100ms). |
| `a_pattern_on_the_wrong_port_fails` | 41.3 | - | MAVLINK sending GPS's label: port 0 has 0 patterns, other ports witnessed, kinds hold `serial_pattern_missing` and `serial_capture_forged` (:2913-2920). | forgery | motor_board(serial-four, firmware edit) = 1 BUILD, 1 BOOT(100ms). |
| `an_echo_the_firmware_never_received_is_forged` | 40.9 | - | An echo of a constant (no `RDR` read) gives `serial_capture_forged` and `serial_echo_missing`, not `serial_pattern_missing` (:2882-2884). | forgery | `mutant("echo-const")` = 1 BUILD, 1 BOOT(100ms). |
| `adversarial_1_a_timer_at_another_rate_with_the_claim_kept_fails` | 40.8 | - | PSC 31 under a PSC 63 claim gives `motor_timing_mismatch` and `TIM1.PSC` reads 0x1f against 0x3f (:2443-2453). | fail | motor_board(motors-pwm, board.rs edit) = 1 BUILD, 1 BOOT(100ms). |
| `l1_a_pattern_sent_after_the_boot_word_never_counts_as_the_echo` | 40.8 | - | The assertions are in `sentinel_02_chain`: no `serial_injection_out_of_order`, MAVLINK injected at boot word + 100 us, CRSF after 8 frame bytes (:2266-2277); the test body itself only prints the kinds (:2304). Not VACUOUS: the helper asserts. | forgery | `sentinel_02_chain`: motor_board(sentinel-02-shaped) = 1 BUILD, 1 BOOT(100ms). |
| `h1c_a_dma_restart_after_the_window_fails_by_name` | 40.6 | - | A DMA restart of TIM1 after the window: four `motor_edge_after_window`, edges after > 0, before 0 (`edges_after_window` :2117-2130). | fail | `dshot_mutant` = 1 BUILD, 1 BOOT(100ms). |
| `adv_4_sbus_s_inversion_written_after_ue_fails` | 39.0 | - | SBUS inversion written after UE: exactly `serial_tag_mismatch`, `tags_missing` RXINV/TXINV, `tags_unexpected` `CR2 0x32000 after UE` (:3146-3157). | fail | motor_board(serial-four, board.rs edits) = 1 BUILD, 1 BOOT(100ms). |
| `a_seeded_imu_fault_is_named_by_its_step` | 38.1 | - | An IMU never woken (transfer deleted, record claimed): only `spi_transaction_unwitnessed`, naming U5 and its `configure` step (:1731-1738). | forgery | `imu_baro_board` library + `mutant_of` with edit = 1 BUILD, 1 BOOT(100ms). |
| `adv3_2_sbus_s_cr2_read_back_and_xored_fails` | 36.4 | - | `CR2` read back and XORed before UE: exactly `serial_tag_mismatch`, `tags_unexpected` `ReadUInt32 from CR2 (0x4)` (:3521-3528). | fail | motor_board(serial-four, board.rs edit) = 1 BUILD, 1 BOOT(100ms). |
| `adv4_2_an_echo_that_hardly_waits_after_the_first_byte_fails` | 36.4 | - | An echo polling 2 times after byte 1: exactly `serial_echo_missing`, echo `partial`, 1 of 8 bytes (:3883-3895). | fail | `adv_reference` = 1 BUILD, 1 BOOT(100ms). |
| `adv_2b_a_presc_write_from_main_after_the_bring_up_fails` | 36.2 | - | PRESC written from `main.rs`: exactly `serial_tag_mismatch`, three `tags_unexpected` (:3083-3088). | fail | `adv_reference` = 1 BUILD, 1 BOOT(100ms). |
| `the_pwm_fixture_s_outputs_are_witnessed_off_the_grid_and_read_back` | 36.2 | - | Four PWM outputs `witnessed` with period 639744 ns, high 256000 ns, off-grid edges, silicon holds, 14 TIM1 registers read back (:1816-1846). | pass | motor_board(motors-pwm, no edit) = 1 BUILD, 1 BOOT(100ms). |
| `a_firmware_that_skips_the_read_or_its_record_cannot_pass` | 36.0 | - | A skipped read gives `i2c_identity_mismatch` + `i2c_transaction_unwitnessed` (:940-943); a missing record gives `i2c_record_invalid`, status `invalid` (:959-967). | forgery | 2 BUILD (`skip-read`, `no-record`), 2 BOOT(10ms). |
| `adv_5_a_pattern_on_a_usart_no_port_names_is_forged` | 36.0 | - | A pattern on an unnamed USART2: exactly `serial_capture_forged`, message names `usart2.bin` and its 12 bytes (:3176-3183). | forgery | `adv_reference` = 1 BUILD, 1 BOOT(100ms). |
| `a_value_one_bit_off_fails_as_an_identity_mismatch` | 35.9 | - | A record one bit off (0x160) fails exactly `i2c_identity_mismatch`, expected 0x60 (:817-828). | fail | `mutant("one-bit")` = 1 BUILD, 1 BOOT(10ms). |
| `adv3_5_an_rx_pin_unrouted_after_the_first_byte_fails` | 35.8 | - | RX pin made an input after byte 1: `serial_tag_mismatch` + `serial_echo_missing`, echo `not_counted`, message names PA10 (:3596-3611). | fail | `adv_reference` = 1 BUILD, 1 BOOT(100ms). |
| `a_seeded_barometer_fault_is_named_by_its_step` | 34.8 | - | A pressure-first command: `i2c_transaction_unwitnessed` naming U6 at 0x63 and its `data` step (:1756-1761). | forgery | `imu_baro_board` library + `mutant_of` with edit = 1 BUILD, 1 BOOT(100ms). |
| `h1b_a_16_bit_restart_after_the_window_fails_by_name` | 34.5 | - | A 16-bit `CEN` restart after the window: four `motor_edge_after_window` (:2158) and `motor_anchor_repeated` (:2160). | fail | `dshot_mutant` = 1 BUILD, 1 BOOT(100ms). |
| `h1a_a_pin_bit_banged_after_the_window_fails_by_name` | 34.4 | - | Pins bit-banged after the window: four `motor_edge_after_window` (:2144). | fail | `dshot_mutant` = 1 BUILD, 1 BOOT(100ms). |
| `f2_a_late_dshot_anchor_fails_by_name` | 34.2 | - | A late `CEN` (anchor over 98 ms) gives `motor_window_past_run`, every motor `stopped` (:2065-2082). | fail | motor_board(sentinel-02-shaped, `late-anchor`) = 1 BUILD, 1 BOOT(100ms). |
| `adv_1b_a_tx_pin_unrouted_after_the_first_byte_fails` | 33.7 | - | TX pin unrouted after byte 1: `serial_tag_mismatch` + `serial_pattern_missing`, `tx_at_send` `MODER 0b00`, the interval-rule text (:3214-3230). | fail | `adv_reference` = 1 BUILD, 1 BOOT(100ms). |
| `motor_output_dead_leaves_its_pin_silent_and_reads_its_wrong_af` | 33.4 | - | PA8 on AF2: kinds `motor_output_unwitnessed` + `register_mismatch`, MOTOR1 alone `unwitnessed` (:2478-2486). | fail | motor_board(motors-pwm, `dead`, board.rs edit) = 1 BUILD, 1 BOOT(100ms). |
| `without_the_cited_wait_each_data_read_answers_its_reset_value` | 32.1 | - | Without the wait, data reads give the reset values (U7 0x7f7f7f7f7f7f, U5 0x0), kinds `i2c_data_mismatch` and `spi_data_mismatch`, never an identity mismatch (:3733-3749). | fail | own_board(`no-wait`) library + `mutant_of` with edit = 1 BUILD, 1 BOOT(100ms). |
| `renode_s_configuration_differs_from_its_default_in_synchronous_logging_and_virtual_time_alone` | 31.1 | - | Renode's default config equals the pinned table (:1463-1467) and a given config is read, never written (:1484-1488). | other | `firmware()` OnceLock; 2 direct Renode runs (1 ms boot + `machine Reset` + 10 us, :1456-1483). |
| `adv3_1_an_apb2_prescaler_after_the_bring_up_fails` | 30.8 | - | A `D2CFGR` write after the bring-up: `serial_tag_mismatch`, `RCC.D2CFGR 0x400 written, not an enable register` (:3471-3476). | fail | `adv3_after_bring_up` -> `adv_reference` = 1 BUILD, 1 BOOT(100ms). |
| `a_firmware_that_rewrites_its_loop_in_flash_is_not_parked` | 29.0 | - | A rewritten self-loop gives exactly `pc_not_on_self_loop` with "no longer holds" (:1282-1288). | forgery | 1 BUILD (`rewritten-loop`), 1 BOOT(10ms). |
| `a_core_that_leaves_the_firmware_after_the_boot_word_fails` | 28.8 | - | A jump to 0x70000001 gives exactly `pc_outside_firmware`, boot word still set (:521-529). | fail | 1 BUILD (`leave`), 1 BOOT(10ms). |
| `adv3_1b_a_usart_kernel_clock_selection_after_the_bring_up_fails` | 28.2 | - | A `D2CCIP2R` write: `serial_tag_mismatch`, `RCC.D2CCIP2R 0x20 written, not an enable register` (:3490-3495). | fail | `adv3_after_bring_up` = 1 BUILD, 1 BOOT(100ms). |
| `adv3_4_a_tx_pin_unrouted_between_tdr_writes_fails` | 28.2 | - | TX pin unrouted between `TDR` writes: `serial_tag_mismatch` + `serial_pattern_missing`, `tx_at_send` `MODER 0b00` (:3560-3566). | fail | `adv_reference` = 1 BUILD, 1 BOOT(100ms). |
| `adv2_5_a_locked_tx_pin_fails` | 28.0 | - | A `LCKR`-locked PA9: four named kinds and `gpioPortA refused a write to 0x0 (a locked pin)`, `tx_at_send` `AF0` (:3394-3412). | fail | `adv_reference` = 1 BUILD, 1 BOOT(100ms). |
| `adv_1_a_tx_pin_routed_only_after_the_pattern_fails` | 27.9 | - | TX routed after the pattern: exactly `serial_pattern_missing`, `tx_at_send` `MODER 0b00` (:3039-3042). | fail | `adv_reference` (board and firmware edits) = 1 BUILD, 1 BOOT(100ms). |
| `adv2_1_a_gpio_reset_through_the_rcc_fails` | 27.8 | - | A GPIOA reset through `RCC_AHB4RSTR`: `serial_tag_mismatch`, two `... not an enable register` lines (:3272-3280). | fail | `adv_reference` = 1 BUILD, 1 BOOT(100ms). |
| `adv2_2_an_open_drain_pulled_down_tx_fails` | 25.1 | - | Open-drain, pulled-down PA9: `serial_tag_mismatch` with four exact unexpected lines (:3301-3311). | fail | `adv_reference` = 1 BUILD, 1 BOOT(100ms). |
| `adv2_4_an_unaligned_store_into_afrh_fails` | 24.4 | - | An unaligned store into `AFRH`: three kinds (`serial_tag_mismatch`, `serial_pattern_missing`, `serial_echo_missing`), echo `not_counted`, `PA9 MODER/AFR not rebuilt` (:3358-3372). | fail | `adv_reference` = 1 BUILD, 1 BOOT(100ms). |
| `the_dshot600_fixture_s_outputs_are_witnessed_in_their_window_of_a_100_ms_boot` | 24.0 | - | The DShot600 board boots 100 ms, window `MAX_VIRTUAL_NS`, 100,000,000 ns, outputs windowed (:1861-1863, `windowed_dshot` :1871-1923). | pass | motor_board(motors-dshot600, `dshot`, no edit) = 1 BUILD, 1 BOOT(100ms). |
| `adv2_6_an_open_drain_tx_read_modify_written_fails` | 19.7 | - | An OTYPER read-modify-write: `serial_tag_mismatch`, two exact unexpected lines (:3433-3440). | fail | `adv_reference` = 1 BUILD, 1 BOOT(100ms). |
| `a_read_no_model_answers_fails_as_a_timeout_at_txis` | 18.7 | - | A read of 0x77: exactly `i2c_timeout`, status `timeout_txis`, message names U2/0x76/TXIS/0x60 (:776-795). | fail | 1 BUILD (`absent`), 1 BOOT(100ms). |
| `a_locked_access_late_in_a_flood_reaches_the_judge_whole` | 18.3 | - | A locked read 183,081 pairs into a 37.7 MB flood still reaches the judge: `locked_peripheral_access` with 0x58020800, `i2c_transaction_unwitnessed`, `renode_log_flood` 8 MiB (:1343-1357). | fail | 1 BUILD (`flood-locked`), 1 BOOT(10ms) with a 37.7 MB Renode log. |
| `a_firmware_with_no_vector_table_at_the_boot_address_fails` | 17.5 | - | Flash at 0x08010000: `boot_word_missing` and `cpu_halted` (:502-505). | fail | 1 BUILD (`flash-offset`), 1 BOOT(10ms). |
| `a_witness_line_sent_through_every_firmware_channel_never_reaches_the_judge` | 17.5 | - | A forged witness line sent through every channel: fails with `i2c_record_invalid` (:4108) and "C1603" is nowhere in the verdict (:4111-4114). | forgery | 1 cargo build of `tests/fixtures/forge-log` (:4061), a patch of the ELF's string tables, 1 BOOT(10ms). |
| `a_firmware_parked_in_its_fault_handler_fails_naming_vectactive` | 16.5 | - | A SysTick tick entering `fault`: kinds `core_in_exception` + `register_mismatch` (SysTick.CSR), "exception 15" (:1198-1210). | fail | 1 BUILD (`systick`), 1 BOOT(10ms). |
| `a_forged_record_without_a_transaction_fails_unwitnessed` | 16.4 | - | A record written with no `transfer` fails `i2c_transaction_unwitnessed` naming U2/0x76/`i2c1.u2` (`assert_unwitnessed` :1007-1031). | forgery | `forged` -> 1 BUILD, 1 BOOT(10ms). |
| `the_console_sends_its_pattern_and_echoes_its_frame` | 14.9 | - | The reference console sends its pattern once and echoes 8 bytes: `witnessed`, `usart_writes` 24, `PA10 ... AF7`, USART1 registers read back (:2660-2685). | pass | `firmware()` OnceLock; 1 BOOT(100ms) of the reference. |
| `a_tdr_store_with_ue_clear_sends_no_pattern` | 13.1 | - | A `TDR` store with UE clear: `serial_pattern_missing`, `register_mismatch`, 0 patterns (:2860-2862). | fail | `mutant_of_board_rs` = 1 BUILD, 1 BOOT(100ms). |

### `campaign_tier2` (7 tests in this group; 9 `#[test]` in the file; no mutex, so tests run in parallel)

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `the_two_reference_droops_behave_as_predicted_twice_byte_identical_and_replay` | 44.5 | - | MULTI: two droop campaigns give byte-identical traces (:555-558) and replay matched (:566-571); tolerated droop has no crossing/reset (:585-592) and 10,000,000 instructions (:604-611); bor3 has four crossings at fixed times (:636-648), two resets (:661-668), one firmware observation (:673-718), 9,876,830 instructions, findings (:742-749). | pass, determinism | `firmware()` OnceLock (:42); `droop_campaign` x2 (:537-538), each 1 `campaign` of 2 scenarios = 4 scenario boots; 4 `replay` CLI calls + `replay_dir` x6. |
| `a_flooding_firmware_reaches_a_verdict_and_its_message_is_the_same_twice` | 27.1 | - | A flooding firmware gets a verdict (exit 0 or 1) with `locked_peripheral_access` and `renode_log_flood` at info (:1016-1036), and its findings and messages are the same on a second run (:1052-1056). | determinism | `flood_firmware()` OnceLock (:460; BUILD of `tests/fixtures/flood`); 2 x `simulate` reference platform+spec 10 ms (:978-996) = 2 boots. |
| `p2_gate_clause_3_the_same_campaign_twice_gives_identical_hashes_per_scenario` | 27.0 | - | Two campaigns of pullup-2k2 and pullup-47k give the same state hash, run id, trace length and trace bytes (:396-412). | determinism | `firmware()`; `campaign("b")` (:387) = 1 `campaign`, 2 boots; then `first()` (:394), which runs `campaign("a")` = 2 more boots if no other test has (OnceLock, :135). |
| `renodes_reset_recipe_is_pinned_step_by_step` | 24.7 | - | Renode reset recipe: `machine Reset` alone leaves boot word, instructions and VTOR 0 (:909-912); the recipe boots (:926-929); without the 10 ns nudge 0 instructions, with it 1000 (:940, :951); OPTSR/OPTCR values (:968-971). | other | `firmware()`; 5 direct Renode runs (a, b, c-no-nudge, c-nudge, d), each re-booting the reference 10 ms (`Renode::booted`, :796-815). |
| `p2_gate_clause_2_the_47k_trace_shows_the_rise_the_fault_and_the_timeout_as_three_events` | 23.4 | - | The 47k trace holds two rises of 3,982,300 ps +/- 500 over a 300,000 ps ceiling, one bus fault, U2 `timeout_txis`, in that order (:196-259), verdict counts 2 and 1 (:269-273). | pass | `first()` OnceLock (:135) = the shared 2-scenario campaign "a"; `replay_dir` library. |
| `p2_gate_clause_2_the_campaign_passes_2k2_and_fails_47k_as_each_expects` | 23.4 | - | The campaign exits 0, 3 lines, 2k2 passes, 47k fails with exactly `analog_rise_exceeds_ceiling` + `i2c_timeout`, summary ok, matched 2, 100,000,000 ns (:146-184). | pass + fail | `first()` OnceLock = campaign "a". |
| `a_droop_on_a_board_whose_first_read_is_on_spi_recovers_and_replays` | 14.3 | - | On a SPI-first board the droop probe reads U4's value 0x00EF4017 with 8 witness lines, verdict pass (:1108-1125), and replay exits 0 (:1127). | pass | CLI `generate` of `spi-only.json` (:1073); 1 cargo BUILD in its own target dir (:1082); 1 `campaign` (1 scenario, 1 boot); 1 `replay`. |
| `p2_gate_clause_3_a_one_byte_change_is_refused_even_with_the_manifest_resealed` (row added by the builder: it reached 10 s only on the third run, 13.5 s) | 13.5 | - | A flipped trace byte is `HashMismatch` with exit 3 (`campaign_tier2.rs:342-349`), and one bit of a 3,982,xxx ps rise changed with the manifest resealed is `ReplayDiverged` with exit 3 (`:374-381`). | forgery | `first()`'s 47k run (`OnceLock`), copied twice by `tampered` (`:316-322`); `replay` ×2 plus one `replay_dir` |

### `models_tier2` (6 tests in this group; 12 `#[test]` in the file). No firmware, no CLI call: each test runs `bench()` (library solve/generate/emit of `sentinel-02-own.json` and 2 model files) then ONE direct Renode run (`run`, :88-133).

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `the_fifo_timestamp_steps_as_the_datasheet_gives_it_at_1_khz` | 15.7 | - | At 1 kHz the FIFO timestamp steps by 937 or 938 (:690-691), gyro Z is 0 at 29 ms and 1640 at 30 ms (:694-695), temperature -40/+100 saturate to 0x81/0x7F (:672-674), count 31 (:682). | pass | `bench("fifo-1k")`; 1 Renode run, ~41.5 ms virtual, a 496-byte burst read (:652-668). |
| `the_fifo_fills_at_the_odr_in_virtual_time_with_the_datasheets_packet` | 15.6 | - | The FIFO count grows at the ODR `[0,1,61,59,0,0]` (:614), packets match the datasheet bytes (:624), a read past the count gives 0xFF (:627), timestamp after soft reset 0x28A6 (:629-632). | pass | `bench("fifo-2k")`; 1 Renode run, ~63 ms virtual. |
| `our_models_answer_their_datasheets_reset_values` | 15.4 | - | Every register read from reset logs its datasheet value (14 IMU, 11 baro) plus the read-only warning; line count and order exact (:258-261). | pass | `bench("resets")`; 1 Renode run of about 90 transactions, no `RunFor`. |
| `an_access_the_datasheets_forbid_warns` | 13.3 | - | Writes within 200 us of OFF, and accesses within 1 ms / 2 ms of a soft reset, log a `[WARNING]` (:322-341). | pass | `bench("timing")`; 1 Renode run, 2.4 ms virtual. |
| `our_models_round_half_away_from_zero` | 13.3 | - | 29 C reads `0x02 0x12` and 100000.0078125 Pa reads `0x01 0xA8 0x61` (:422-423). | pass | `bench("rounding")`; 1 Renode run, 30 ms virtual. |
| `a_big_endian_packet_with_an_invalid_gyro_slot_follows_the_datasheet` | 13.1 | - | Big-endian packets, count 0x02A0 (:846), packet bytes at 30.0/30.5 ms (:859-860), `HEADER_ODR_ACCEL` 0x6A and `HEADER_ODR_GYRO` 0x69 (:864-865). | pass | `bench("fifo-be")`; 1 Renode run, ~32 ms virtual, a 672-byte burst. |


## AET-WO-1910 census rows, group C (tier-2 verify / prove / KiCad tests)

Repository: `/home/forge/aethelgard-1910`, main at 565b32c5. All paths below are under `crates/aethelgard-cli/tests/` unless they start with `crates/` or a top-level dir. Read only; nothing was run. "Inferred" marks a reading not seen in code or a log.

### 0. Facts every row relies on

- **The fixture is not `examples/sentinel-03.json`.** There is no such file (`ls examples`: `invalid-zero-domain.json`, `sentinel-01-minimal.json`, `sentinel-01.json`). Sentinel-03 is `crates/aethelgard-cli/tests/fixtures/sentinel-03.json` (`board_tier2.rs:1822`, `:1852`).
- **The power fixtures.** `common::power::power_tree()` (`common/power.rs:7`) reads `crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json`, drops the crossing, the IMU and its rows. `board_tier2.rs:1152` uses that tree. `board_tier2.rs:1258` uses the committed file whole ("full" fixture).
- **What one `verify` runs today** (`crates/aethelgard-cli/src/verify.rs:1530-1805`, `run_gates`): `check erc`, `check drc --spec --schematic`, `check support` (new since 1909), `extract`, then `prove --spec` once per harness (`:1607`), `build`, `simulate`, ONE `campaign` call over all scenarios (`:1663-1672`), and one `attribute` per scenario whose seeded fault is "detected" (`:1745-1764`). With `--fab-out` or `--against-fab` it then runs kicad-cli's Gerber, drill and placement export. `verify` generates in-process (`:1099`); it does not spawn `emit`.
- **The reference has 7 Kani harnesses** (`boards/sentinel-01/src/proofs.rs` has 7 `#[kani::proof]`; the 14-gate list is asserted at `verify_2k2_tier2.rs:152-175`: erc, drc, support, extract, 7 x prove, build, simulate, campaign). 1909 said "Kani prove x2": stale. Every `verify(reference)` below therefore runs 7 `prove` gates.
- **The reference's package has 17 files**, not 15 (`verify_2k2_tier2.rs:326-348`, `against_fab_tier2.rs:148-150`). 1909 said 15: stale.
- **V(x) notation used below.** `V(ref, S, flag)` = one `aethelgard verify --spec examples/sentinel-01-minimal.json --scenarios S flag`, with the 14 gates above. `SEEDED` = 5 scenarios (`common/verify.rs:25-31`: pullup-3k3, droop-bor3, spi-absent-u4, pullup-2k2, serial-rx-open-console); two of them are "detected" (spi-absent-u4, serial-rx-open-console), so each `V(ref, SEEDED)` also runs 2 `attribute` gates (`verify_2k2_tier2.rs:431-447`).
- **P(x)** = `aethelgard prove` of one Kani harness. `every_valid_sequence` takes minutes in the image (228 s measured, comment at `board_tier2.rs:20-23`). The `*_transfer_*` harnesses take about 8-14 s (older logs: `~/.cache/aet-4230/t2/smoke2/board_tier2.log`, lines `ANSWER 4220-PB10`).
- **The "tools s" column is wall time inside a parallel run.** A test that takes a lock or waits on a `OnceLock` includes the wait (inferred from `board_tier2.rs:36`, `attribute_seeded.rs:564`, `common/verify.rs:196`). It is not the test's own CPU cost.

### Which tests hold `ONE_AT_A_TIME`

A `static ONE_AT_A_TIME: Mutex<()>` exists in three binaries. It serialises only within its own binary.

| binary | declared | tests that take it |
|---|---|---|
| `board_tier2` | `board_tier2.rs:36` | the `prove()` helper (`:39`) for `the_generated_board_proves`, `a_wrong_alternate_function_fails`, `a_write_past_the_block_fails`, `a_missing_clock_enable_fails`, `a_vacuous_harness_fails`; per harness in `the_imu_fixture_s_transfers_prove_within_its_scripts` (`:157`) and `the_own_parts_fixture_s_transfers_prove_within_its_scripts` (`:213`); the whole body of `prove_with_spec_tells_a_hand_written_harness_from_the_generated_one` (`:403`), `verify_refuses_a_board_swapped_before_extract` (`:587`), `every_admitted_identifier_character_survives_every_output` (`:707`), `four_layer_fixture_passes_every_gate_and_its_package_holds` (`:979`), `against_fab_names_a_missing_inner_layer_and_a_stackup_that_lies` (`:1031`), `connectors_pass_erc_and_drc_with_parity_at_two_and_four_layers` (`:1075`), `the_power_fixture_passes_erc_drc_its_loop_rules_and_extract` (`:1148`), `the_full_power_fixture_routes_and_passes_every_gate_it_reaches` (`:1253`), `the_reference_passes_the_support_gate_and_a_capacitor_moved_off_fails` (`:1321`), `the_bench_board_passes_every_gate_at_two_and_four_layers` (`:1408`), `the_own_parts_fixture_passes_erc_drc_with_parity_and_extract` (`:1511`), `the_ordered_board_passes_every_board_gate_with_no_finding` (`:1629`), `the_sentinel_03_fixture_passes_every_gate_inside_its_bound` (`:1818`), `each_size_sample_passes_every_gate_or_is_refused_by_name` (`:1847`). **Not locked:** `every_corpus_file_the_reader_accepts_gets_its_stated_kicad_verdict`, `build_gives_one_sha256_at_two_paths`, `as_root_a_third_uid_cannot_change_what_build_reads`, `a_cargo_configuration_another_uid_plants_in_tmp_is_not_read`. |
| `verify_2k2_tier2` | `verify_2k2_tier2.rs:24` | both tests, whole body (`:97`, `:268`). |
| `against_fab_tier2` | `against_fab_tier2.rs:19` | both tests, but only around their `--against-fab` verify (`:69`, `:109`); the shared `--fab-out` run is made outside the lock (`:42`, inside `PACKAGE.get`). |
| `verify_47k_tier2` | none | one `Shared` run (`verify_47k_tier2.rs:24`). |
| `attribute_seeded` | none named `ONE_AT_A_TIME`. `ONE_PROOF` (`:1282`) wraps only the prove in `every_gate` (`:1333`); `REGATED` (`:1178`) is held for the whole of `every_gate` (`:1195-1199`). | see section 2. |
| `with_tools`, `adversarial_1301_tools` | none | none |

---

### 1. Tables, one per binary

### `board_tier2` (15 of its 25 tests are in the input)

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `four_layer_fixture_passes_every_gate_and_its_package_holds` | 1057.6 | - | **MULTI:** (a) `verify` of `four-layer.json` with its 4 scenarios passes, every gate `pass` (`board_tier2.rs:985-988`); (b) both inner Gerbers carry their plane region and no clock line (`:991-1002`); (c) `FAB-NOTES.md` states the layer order and every dielectric (`:1004-1015`); (d) `--against-fab` on that package is `identical` (`:1019-1022`). | pass, determinism | V(four-layer, 4 scenarios `:955-960`, `--fab-out`) x1 at `:983`; V(four-layer, same 4, `--against-fab`) x1 at `:1019`. Each runs 7(?) `prove` gates, `build`, `simulate`, 1 campaign of 4 scenarios, kicad-cli export (the 7 is inferred for the four-layer crate; its harness count was not read). Locked (`:979`). No shared value. |
| `against_fab_names_a_missing_inner_layer_and_a_stackup_that_lies` | 846.1 | - | A 4-layer package with an inner Gerber deleted and a changed stackup line in `FAB-NOTES.md` fails `--against-fab`, exit 1, `differs`, naming exactly those two files (`:1044-1062`). | forgery | V(four-layer, 4 scenarios, `--fab-out`) x1 at `:1035`; V(four-layer, 4 scenarios, `--against-fab`) x1 at `:1042`. Same input as the first test (section 3, group G3). Locked (`:1031`). |
| `every_admitted_identifier_character_survives_every_output` | 551.6 | - | One specification carrying all 83 admitted identifier characters passes `verify --fab-out` (`:844-847`) and every output carries each name byte for byte (`:938`, the `damage` list) with every precondition met (`:948-949`). | pass | V(renamed reference copy at `base/spec.json`, 5 renamed scenarios, `--fab-out`) x1 at `:825-841`, run from the spec's directory. Renamed, so no other test's input. Locked (`:707`). |
| `a_write_past_the_block_fails` | 455.0 | - | TXDR offset 0x28 to 0x2c in a copy of `boards/sentinel-01` fails `every_valid_sequence` as `proof_failed` "register access past the end of its block" (`:98-111`, called at `:269-282`). | fail | P(seeded copy of boards/sentinel-01, `proofs::every_valid_sequence`, no `--spec`) x1 (`:42-48` via `prove()`). Locked per call (`:39`). Time includes lock waits (inferred). |
| `the_generated_board_proves` | 358.7 | - | `prove` of the committed `boards/sentinel-01` without `--spec` gives `pass` and no findings, Kani 0.68.0 (`:114-118`, version at `:62-65`). | pass | P(boards/sentinel-01, `every_valid_sequence`, no `--spec`) x1 at `:115`. Locked per call (`:39`). |
| `a_wrong_alternate_function_fails` | 325.9 | - | AF 4 to 5 in a copy of `board.rs` fails `every_valid_sequence` as `proof_failed` "GpioB word 8: a solved pin's field is not what the specification fixes" (`:245-253`, assertion `:110`). | fail | P(seeded copy, `every_valid_sequence`) x1. Locked per call (`:39`). |
| `each_size_sample_passes_every_gate_or_is_refused_by_name` | 174.5 | - | **MULTI:** for each of 3 renamed copies of `sentinel-03.json` (prefix 2, 6, 8 characters, `common/rename.rs:134`), the outcome is a refusal by name (`OutlineTooSmall` `:1708-1713`; `OutlineUnreachable` or `CapacitanceOverBound` `:1727-1740`) or every gate passes and `extract` is either `pass` with no finding or `fail` exit 1 with only `capacitance_over_declared_bound` (`:1865-1876`). The outcome is not pinned: any of them is accepted. | pass, refusal | `bounded_gates` (`:1706`) x3, one per size (`:1856`): `validate` x1, `generate` x1, `emit` x1 (`:1723`), `check erc` x1, `check drc --spec` x2 (no parity, parity), `check support` x1, `extract` x1 each, if no refusal comes first. `emit` of a renamed sentinel-03 x3. Locked (`:1847`). |
| `the_sentinel_03_fixture_passes_every_gate_inside_its_bound` | 155.2 | - | `sentinel-03.json` at 4 layers, inside its 100 x 100 mm bound, passes `validate`, `generate`, `emit`, ERC, DRC with and without parity, support, and `extract` with no finding (`:1824-1826`, plus the asserts inside `bounded_gates` `:1741-1791`). | pass | `bounded_gates` x1 (`:1823`): `emit` x1 at `:1723` of the unrenamed `tests/fixtures/sentinel-03.json`; `generate` x1, `validate` x1, ERC x1, DRC x2, support x1, `extract` x1. Locked (`:1818`). |
| `the_power_fixture_passes_erc_drc_its_loop_rules_and_extract` | 89.1 | - | **MULTI:** (a) the power tree passes ERC, DRC with parity (17 loop rules pass, classes POWER 500 / SUPPLY_MCU 200) and `extract` (`:1159-1206`); (b) moving C11 25 mm off fails DRC with exactly one `loop_rule` finding starting `C11 ` (`:1225-1237`). | pass, fail | `emit` (CLI) x1 at `:1157` of `power_tree()`; library `schematic::emit_moved` x1 at `:1217` (re-places and re-routes; same spec); `check erc` x1; `check drc --spec --schematic` x2 (`:1182` pass, `:1224` moved); `extract` x1. Locked (`:1148`). |
| `every_corpus_file_the_reader_accepts_gets_its_stated_kicad_verdict` | 80.0 | - | Each corpus edit the strict reader accepts gets exactly its stated exit, verdict and id from KiCad on `check drc`, `drc-parity` and `check erc` (`:661-671`; `ran > 0` at `:674`). | fail, refusal (a table) | `emit` x1 of `examples/sentinel-01-minimal.json` (`common/corpus.rs:189`, from `:652`); then 40 gate subprocesses: counted from the 55 corpus files in `tests/fixtures/strict/`, the expectations with exit not 3 are 19 `drc` + 19 `drc-parity` + 2 `erc` (`:658` filter; counted with grep on the `# <gate>: <code>` lines). Not locked. No shared value. |
| `the_own_parts_fixture_s_transfers_prove_within_its_scripts` | 64.7 | - | Every `*_transfer_is_bounded_and_ordered` harness of the generated Sentinel-02-own crate passes (`:239-240`); the IMU's own harness is among them (`:208`). | pass | Library `solve` + `generate_with` of `tests/fixtures/sentinel-02-own.json` (`common/verify.rs:generated`) x1; then P(generated crate, one transfer harness) x N, N = 3 (spi1, spi2, i2c1; from an older log `ANSWER 4220-PB10`, so inferred for today). Each took 8-14 s in that log. Locked per harness (`:213`). |
| `the_imu_fixture_s_transfers_prove_within_its_scripts` | 43.7 | - | Both transfer harnesses (`spi1_`, `i2c1_transfer_is_bounded_and_ordered`) of the 1.8 V IMU/baro crate pass (`:183-184`); the generated proofs carry the scripted bounds (`:145-152`). | pass | Library `solve` + `generate_with` of `tests/fixtures/imu-baro-1v8.json` x1 (`:127-140`); P(generated crate, small harness) x2 (`:154-185`). Locked per harness (`:157`). |
| `prove_with_spec_tells_a_hand_written_harness_from_the_generated_one` | 20.8 | - | With `--spec`, a hand-written vacuous harness that passes Kani is `harness_provenance: supplied`, `cover_count 0` (`:427-430`); the committed crate's `i2c1_transfer_is_bounded_and_ordered` is `generated`, `cover_count 7` (`:436-438`). | forgery | `prove --spec examples/sentinel-01-minimal.json` x2: a copy with a 5-line vacuous `proofs.rs` (`:403-424`), then `boards/sentinel-01`, harness `i2c1_transfer_is_bounded_and_ordered` (`:432-434`). Locked (`:403`). |
| `the_full_power_fixture_routes_and_passes_every_gate_it_reaches` | 20.4 | - | The committed `sentinel-02-power.json` routes and passes ERC, DRC (loops non-empty and all pass), support (`:1283-1304`), and `extract` refuses it as `TranslatorPinUnbounded` (`:1308-1309`). | pass, refusal | `emit` (CLI) x1 at `:1261` of the full committed fixture; `check erc` x1; `check drc --spec --schematic` x1; `check support` x1; `extract` x1 (refused). Locked (`:1253`). |
| `the_bench_board_passes_every_gate_at_two_and_four_layers` | 14.3 | - | `sentinel-01-bench.json` passes `validate`, `generate`, `emit` (12 test points and SW1 on the board, `:1441-1447`), ERC, DRC with and without parity, support (BOOT0 strap named, `:1480-1483`) and `extract` with no finding and no test pad counted as a pin, at 2 and at 4 layers (`:1448-1505`). | pass | 2 passes (`:1417`, layers 2 and 4) of: `validate` x1, `generate` x1, `emit` x1 (`:1433`), ERC x1, DRC x2, support x1, `extract` x1. `emit` of the bench spec x2 (the 4-layer copy has the board of `four-layer.json`, `:1415-1416`). Locked (`:1408`). |

### `verify_2k2_tier2` (2 tests)

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `verify_fab_out_writes_the_package_its_verdict_names_at_any_path` | 1080.7 | - | **MULTI:** `--fab-out` at two paths gives one verdict and one set of bytes (`verify_2k2_tier2.rs:280-286`); the package is exactly 17 named files (`:348`), each named by sha256 in the verdict (`:311-323`); the ELF, BOM (emit's, `:368`) and JLCPCB files (`:376-384`) are the ones their sources give; the manifest names the verdict without `fab`, the spec, the topology and the scenarios (`:393-414`); all four preconditions are met and both `attribute` gates pass (`:426-447`); B.Cu carries the GND pour and no Gerber has a clock line (`:457-471`); `FAB-NOTES.md` states the stackup, the NRND line, the ledger rows and lists every other file (`:488-558`). | pass, determinism | V(ref, SEEDED, `--fab-out`) x2 (`:78`, `:79`, args built at `:62-76`, paths `a` and `a-much/longer/path/b`), each with 7 prove, 1 campaign over 5 scenarios, 2 `attribute`, kicad-cli export; `emit` x1 of the reference at `:363`. Locked (`:268`). |
| `verify_binds_the_reference_and_two_runs_print_the_same_bytes` | 1077.9 | - | **MULTI:** `verify --against` the committed tree prints identical bytes twice (`:102`), `against` is `identical` (`:106`), every file's hash is named (`:145`), the 14 gates run in order and pass (`:176`), each gate's uid boundary is its grants' (`:178-215`), the ELF is the one `build` gives (`:223`), the run id and state hash are the ones `campaign` gives (`:235-251`). | pass, determinism | `emit` x1 (`:55`); V(ref, `pullup-2k2.json`, `--against tree`) x2 (`:78`, `:79`, identical args); `build` x1 (`:219`); `campaign` x1 of pullup-2k2 (`:239`). Each verify: 7 prove, build, simulate, 1 campaign. Locked (`:97`). |

### `verify_47k_tier2` (2 tests, one shared run)

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `verify_fab_out_writes_nothing_when_a_gate_fails` | 529.8 | - | A failing `verify` writes nothing into `--fab-out`: `fab.status` `not_written`, `fab.files` `{}`, the directory empty (`verify_47k_tier2.rs:122-129`). | fail | The shared `RUN` (`:24`): V(ref, 5 scenarios: pullup-47k, droop-bor3, spi-absent-u4-one-kind, pullup-2k2, serial-rx-open-console, `--fab-out`) x1 (`:36-52`). Shared with the other test below. |
| `verify_fails_a_run_that_fails_otherwise_than_declared_and_detects_47k` | 529.8 | - | `verify` exits 1, the campaign gate fails on exactly `spi_identity_mismatch` and `spi_transaction_unwitnessed` of `spi-absent-u4-one-kind`, 47k is `attribute`d `pass`, and every earlier gate passes (`:71-108`). | fail | The same `RUN` (`:24`, made once; both tests read it). The other test's time is the same value because both wait on the one `OnceLock` (`common/verify.rs:183`). |

### `against_fab_tier2` (2 tests, one shared package)

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `verify_against_fab_names_every_changed_and_extra_file` | 1193.0 | - | One flipped byte in each of the 17 package files plus one extra file gives exit 1, 17 `against_differs` and 1 `against_extra`, each named (`against_fab_tier2.rs:135-150`; count 18 at `:148-150`). | forgery | Shared `PACKAGE` (`:28`): V(ref, SEEDED, `--fab-out`) x1 (`:42`), made by whichever test runs first; then V(ref, SEEDED, `--against-fab` of a changed copy) x1 (`:110`). Locked around the second (`:109`). |
| `verify_against_fab_finds_the_package_it_wrote_identical` | 1163.5 | - | `--against-fab` on an untouched package is `identical`, `fab.status` `derived`, same `fab.files`, no `differences` (`:72-92`). | pass | The same shared `PACKAGE` (`:28`); then V(ref, SEEDED, `--against-fab` of a copy) x1 (`:70`). Locked around it (`:69`). |

Total for this binary: 3 `V(ref, SEEDED)` runs (1 `--fab-out`, 2 `--against-fab`).

### `attribute_seeded` (5 of its 11 tests are in the input; 4 are tier 2)

The manifest is `scenarios/sentinel-01/seeded/cases.json`: **30 cases, 13 mutants** today (1909 said 22 runs; stale). Firmwares: the unmutated one plus 13 mutants = 14.

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `tier2::p3_gate_clause_2_propose_answers_every_seeded_case_as_its_rule_says` | 1392.8 | - | **MULTI:** `propose` answers each of the 30 cases as its rule says (exit and files per case kind: proposal `:971`, report `:1001`, model slot `:1041`, below floor `:1057`, INV-05, indeterminate `:1095`); `propose` twice writes the same bytes (`:955-966`); each of the 7 proposals re-gates through every gate with its rise within 2 % of the prediction (`regate` `:999`, `:1184`; `within_2_percent` `:1250`, assert `:1277`). | fail, pass, determinism | `ran()` (`RAN`, `:564`): 13 `generate` + 14 `build` + 14 `campaign` over 30 scenarios (30 Renode runs, inferred) once; `propose` x60 (`:946` and `:954`, 2 per case); `every_gate` (`:1289`) x(distinct proposed specs, inferred 2: 2700 ohm for six cases, 1200 ohm for `cap-220p-2k2`, from `want()` `:865-895`), each: `validate`, `solve`, `generate`, ERC, DRC with parity, P(generated crate, `every_valid_sequence`, `--spec`) x1 (`:1333`), `build`, 1 `campaign`, 1 `attribute`; the cache `REGATED` (`:1178`) makes each distinct spec run once. |
| `tier2::attribute_twice_is_identical_and_a_one_byte_change_is_refused_as_replay_refuses_it` | 393.5 | - | **MULTI:** `attribute` is byte-identical twice on all 30 runs (`:801-807`); a one-byte trace change is refused as `HashMismatch`, exit 3, exactly as `replay` refuses it (`:820-833`). | determinism, forgery | `ran()` (`RAN`); `attribute` x61 (`:802-803` two per run x 30, plus `:820`); `replay` x1 (`:819`). |
| `tier2::p3_gate_clause_1_attribute_names_every_case_with_evidence_its_rule_allows` | 384.0 | - | `attribute` on each of the 30 runs gives the manifest's domain, rule, exact evidence events and exact masked events, each a record of the kind it says (`:702-735`); exit 1 only for `indeterminate` (`:696-701`). | fail | `ran()` (`RAN`); `attribute` x30 (`:690`); library `replay_dir` x30 (`:709`). |
| `tier2::every_case_fails_the_way_its_manifest_says` | 374.8 | - | Each of the 30 runs reports exactly its manifest's finding list (`:643-647`); each unmutated case matches its own `expect` (`:649-654`). | fail | `ran()` (`RAN`); library `replay_dir` x30 (`:657`, prints rises only). |
| `every_mutant_applies_exactly_once_to_the_generated_board_as_committed` (tier 1, also in `tools`) | 36.5 | 34.3 | Each of the 13 mutants' `find` text occurs exactly once, in code, in the generated board, and changes it (`:273-274`). | other | `generate` subprocess x13 (`:269-273` via `board_of` `:56-71`): 12 of `examples/sentinel-01-minimal.json`, 1 of `tests/fixtures/motors-pwm.json`. The 12 are the same input. No shared value; each writes its own directory. |

### `with_tools` (4 of its 32 tests are in the input)

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `d3_and_d1_pass_erc_and_drc_with_parity_at_four_layers` | 19.6 | - | `d3-24-i2c-12-spi.json` and `d1-24-i2c.json` route at 4 layers and pass ERC and DRC with parity with no finding of any kind (`with_tools.rs:1108-1112`). | pass | `emit` x2 (`:1100`), ERC x2 (`:1103`), `check drc --spec --schematic` x2 (`:1109`, via `drc_parity` `:740`). Inputs used only here in tier 2 (`grep`: also read by tier-1 tests). |
| `a_turned_keepout_is_where_kicad_draws_it` | 18.9 | - | An IMU turned 90, 180 or 270 degrees has its two keep-outs where KiCad draws them: DRC with parity passes with no finding (`:1650-1651`). | pass | 3 passes (`:1621`): library `solve` + `schematic::emit` of `tests/fixtures/sentinel-02-own.json` with U5's orientation changed (`:1637`); `check drc --spec --schematic` x3 (`:1649`). No CLI `emit`. |
| `p1_gate_clause_1_the_reference_board_passes_erc_and_drc_with_no_finding` | 15.7 | - | **MULTI:** (a) the reference and the S5 names board each pass ERC and DRC with parity with no finding, bound to the emitted hashes (`:834-853`); (b) routing the read-back pads again lays the same segments and vias (`:866-867`). | pass, determinism | 2 passes (reference, then S5-renamed copy): `emit` x2 (`:824`), ERC x2, DRC parity x2, library `route::read` + `route::route` x2 (`:857-865`, a second routing, debug build). |
| `prove_of_an_unresolvable_dependency_names_the_package` | 11.5 | - | A crate with an unresolvable dependency is `ToolError`, exit 5, naming the package and "the tool said: error:" (`:232-235`). | refusal | `prove` x1 (`:224`) on a throw-away crate. Where 11.5 s goes is not in the test (section 5). |

### `adversarial_1301_tools` (2 of its 6 tests are in the input)

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `attack_wide_references_leave_the_emitted_board_clean` | 34.1 | - | 24 parts with 64-byte references emit a board whose DRC with parity shows nothing beyond `unconnected_items` (`adversarial_1301_tools.rs:337`), after the printed-reference partition holds (`:347-387`). | forgery | 2 passes (`short`, `wide`, `:304`): `emit` x2 (`:319`) of a 24-device spec built from `sentinel-01-minimal.json`; library `solve` + `anchored` x2 (`:347`); `check drc --spec --schematic` x2 (`:327`). |
| `parity_fails_every_other_mismatch_with_kicads_finding` | 17.6 | - | Each of 19 board/schematic mismatches fails DRC parity with exit 1 and exactly the finding kinds KiCad gave when measured (`:674-675`); the routed swap adds `lib_footprint_mismatch` (`:688-689`); two unconnected pins joined on one name is refused `SharedUnconnectedNet`, exit 3 (`:702-703`). | fail (a table) | `emit` x1 of the reference (`:479` via `reference` `:112`); `check drc --spec --schematic` x21 (`:669` x19, routed `:679`, joined `:699`; the last is refused before KiCad runs, inferred from the exit 3). |

### Other 1909-era rows that moved (not in the input; for the reader's check)

`a_missing_clock_enable_fails` now uses `SPI_SETUP`, the cheapest SPI1 harness (`board_tier2.rs:34`, `:260`), so it no longer costs minutes. `oq_03_the_reference_runs_bytes_with_kind_15` no longer exists in `attribute_seeded.rs`. `build_gives_one_sha256_at_two_paths`, `as_root_...`, `a_cargo_configuration_...` are not in the input (under 10 s).

---


## Census rows, group D (AET-WO-1910)

Repository read at `/home/forge/aethelgard-1910` (main 565b32c5). Nothing was run. All `file:line` below are in that tree. Test files are in `crates/aethelgard-cli/tests/` unless a path is given.

### Abbreviations (the ladder count)

- **LAD** = one `schematic::emit` (library, or the CLI `emit`/`verify` subprocess) on a specification that declares `board.stackup.fab_tolerances` and whose lines `extract::lines` can judge. It routes the board 8 times (`RUNGS`, `route.rs:138`; `laddered`, `schematic.rs:926-1030`; chosen at `emit_moved`, `schematic.rs:813`). The rungs run on 8 threads (`schematic.rs:972-990`). FEEDBACK measured it at about 25 s in release against 2.85 s for one routing (`docs/work-orders/AET-WO-1314.FEEDBACK.md:90`).
- **UNJ** = fab_tolerances declared, but `extract::lines` refuses (`TranslatorPinUnbounded`). `laddered` then routes once at `Rung::TODAY` and records `unjudged` (`schematic.rs:932-949`). No ladder.
- **PLAIN** = no `fab_tolerances`. `ruled` is false and the board is routed once at `Rung::TODAY` (`schematic.rs:819-825`; `RingRule::of` returns `None`, `stack.rs:65-67`).
- `placement` = `schematic::placement` (`schematic.rs:904`): place only, no routing.

Which fixtures declare `fab_tolerances` (grep over the whole tree, `*.json`): only `tests/fixtures/sentinel-03.json:868` (it also has `max_outline_um`, `:897`) and `crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json:254`. `examples/sentinel-01-minimal.json`, `d1-24-i2c.json`, `d3-24-i2c-12-spi.json`, `four-layer.json`, `pair-standin.json`, `imu-baro-1v8.json`, `connectors.json` declare none. Both fixtures' blocks declare all five fields, so `RingRule::of` is `Some`.

Whether `lines` judges: sentinel-03 and `power_tree()` (crossing and IMU removed, `common/power.rs:10-33`) are judged, because `extract` on their boards ends in a verdict, not a `TranslatorPinUnbounded` refusal (`via_ring_tier1.rs:126-128`, `power_board_tier1.rs:216-217`, `supply_tier1.rs:213,340`). The full `sentinel-02-power.json` (crossing kept) is not judged: `via_ring_tier1.rs:261`, `power_board_tier1.rs:344-348`. That the ladder runs on `power_tree()` is inferred from these.

`power_tree()` is `sentinel-02-power.json` with `power.crossings = []`, the ICM20948 acknowledgement dropped, and U5/IMU entries removed from `spi_devices`, `spi_buses`, `net_constraints` (`common/power.rs:10-33`). It is a different input from the full fixture.

CLI `verify` runs one library `emit` inside `generate::tree` (`generate.rs:94`) before it reads the scenarios, checks `--fab-out` (`verify.rs:1090-1092`, `1164-1172`) or the coverage and ledger (`verify.rs:1217-1254`). So every `verify` call that gets past `solve_spec` is one PLAIN emit of its `--spec` (the reference, no fab_tolerances). A clap `conflicts_with`/`requires` refusal (`main.rs:125-126`) comes before `verify()`: no emit.

Seconds are the CI figures from `in-D.tsv`. They are wall time per test inside a threaded `cargo test` process, so they include waiting on a `OnceLock` and CPU contention (inferred; see section 5).

---

### 1. Tables

### `power_board_tier1` (10 tests; fixture = `power_tree()` unless noted; no OnceLock, each test builds its own)

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `a_loop_part_moved_off_its_pin_fails_its_rule` | 334.3 | 797.5 | Moving C11, R10, L1, C30 or C50 25 mm off its pin fails that part's loop rule and only that one (`power_board_tier1.rs:91`). | fail | LAD ×6: 1 `emit` (`:76`) plus 5 `emit_moved` (`:81`, one per part) on `power_tree()` (no `max_outline_um`); each followed by `read` and `loops::judge` (`:85`). 0 `placement`. 48 routings. |
| `every_loop_rule_holds_on_the_fixtures_own_board` | 123.3 | 386.1 | **MULTI:** all 17 loop records pass (`:49-52`); C50 sits on the solved sense pin (`:58`); that MCU pad carries `/VBAT_SENSE` (`:66`); two emissions are equal (`:68`). | pass, determinism | LAD ×2: `emit` at `:44` and again at `:68`, same input. 0 `placement`. 16 routings. |
| `the_crossing_is_drawn_and_its_limits_are_refused_by_name` | 79.5 | 234.9 | **MULTI:** U5's nine routed pads escape from their own centres (`:315-318`); the keepout is centred and 1.75 x 1.59 mm (`:334-338`); two emissions are equal (`:339-343`); `extract::lines` refuses `TranslatorPinUnbounded` naming U13 (`:346-347`). | pass, residual | UNJ ×2 on the full `sentinel-02-power.json` (`full()`, `:279-286`): `emit` at `:300` and `:340`. One routing each, no ladder. 0 `placement`. Plus `route::check`, `connectivity`, one `extract::lines`. |
| `a_rule_whose_part_is_missing_is_refused` | 100.9 | 218.8 | `loops::judge` refuses a rule whose part C13 the board lacks, naming C13 (`:127`). | refusal | LAD ×1 (`:122`), `power_tree()`. 0 `placement`. 8 routings. |
| `a_starved_thermal_array_is_refused` | 100.7 | 217.6 | `route::check` passes the array as emitted (`:259`) and refuses one short of its rule or one via off its site (`:263`, `:266`). | fail | LAD ×1 (`:256`), `power_tree()`. 0 `placement`. 8 routings. |
| `a_source_or_return_pad_off_its_net_fails_k5` | 81.9 | 217.0 | `loops::source_pads` refuses a divider pad on the wrong net, naming the part and `K5` (`:114`). | fail | LAD ×1 (`:100`), `power_tree()`. 0 `placement`. 8 routings. |
| `extract_bounds_the_power_board` | 69.1 | 216.3 | CLI `extract --board --spec` on the emitted board exits 0 with verdict `pass` (`:216-217`). | pass | LAD ×1 (`:197`) plus one `extract` subprocess (`:208`) on `power_tree()`. 0 `placement`. 8 routings. |
| `only_a_rail_no_power_out_pin_drives_carries_a_flag` | 101.0 | 215.8 | The schematic has exactly 4 `PWR_FLAG`s, not one on the LDO-driven VDD_1V8 (`:168`). | pass | LAD ×1 (`:165`), `power_tree()`. 0 `placement`. 8 routings. |
| `the_nets_validate_reserves_are_the_emitters_own` | 111.7 | 179.7 | Every net `validate` reserves is drawn on the board, and every net drawn that the spec does not spell is reserved (`:567`, `:573`). | pass | 2 emits: `examples/sentinel-01-minimal.json` PLAIN (1 routing) and `power_tree()` LAD (8 routings), in the loop at `:546-549`; 2 `solve`. 0 `placement`. |
| `the_reader_admits_a_footprints_keepout_clearance_and_nothing_more` | 110.4 | 111.5 | The strict reader admits a keepout's own `(connect_pads (clearance 0))`, and refuses an extra field or a non-keepout zone as `LocalClearance` (`:457`, `:459-462`). | refusal | LAD ×1 (`:439`), `power_tree()`; then 3 `board::read` + `prepare_read` on that board text plus a footprint (`:455-462`). 0 `placement`. |

Not in the input but in the same binary: `a_crossing_on_a_bus_of_two_devices_is_not_drawable`, `a_net_spelled_vbat_beside_a_power_tree_is_refused`, `a_power_reference_the_emitter_refuses_is_refused_at_validate` all refuse at `validate` before routing.

### `outline_tier1` (6 tests; fixture = `sentinel-03.json`, which declares fab_tolerances and a 100 x 100 mm bound; no OnceLock)

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `a_turned_imu_is_compacted_whole_with_its_keepouts_inside_the_bound` | 244.4 | 535.9 | **MULTI:** under the bound, U5 is placed at the turned rotation (`outline_tier1.rs:461-465`), the outline stays within 100 mm (`:467`), every U5 keepout point is inside it (`:479`), plus the gap and loop checks (`:487-488`). | pass | LAD ×4: 4 `emit` (`:455`), one per U5 orientation (right, forward, left, aft), each of the fixture with `board.forward = "top"` and U5's `orientation` set (`:446-451`). Each followed by `assert_gaps` (>50 footprints, `library::footprint` each, pairwise gaps, `:226-292`) and `assert_loops`. 0 `placement`. 32 routings. |
| `the_fixture_fits_its_bound_with_connectors_on_the_side_edges` | 151.8 | 403.7 | **MULTI:** with the bound, the board is within 100 x 100 and every connector is at 90 or 270 (`:93-97`); with the bound removed, the board is wider than 100 mm and every connector at 0 (`:106-107`). | pass | LAD ×2: `emit` at `:91` (the fixture as committed), `emit` at `:104` (same file with `board.max_outline_um` removed, `:102`; it still declares fab_tolerances, so it is a ladder too, on an unbounded placement). 0 `placement`. 16 routings. |
| `order_only_renamings_place_the_fixture_the_same_way` | 130.0 | 365.1 | **MULTI:** `emit`'s board places exactly as `placement` does (`:359-363`, and the board is laddered, `:358`); each of 20 order-only renamings places identically (`:373`). | determinism | LAD ×1 (`:357`, the fixture as committed) plus 21 `placement` calls: base (`:355`) and seeds 1..=20 (`:373`), the 20 on up to 8 threads (`:365-377`). Placement runs the bounded compaction. 22 `solved` (validate + solve). 8 routings. |
| `each_size_sample_places_with_its_gaps_or_is_refused_by_name` | 130.5 | 312.6 | For each of 3 size samples, `emit` places inside 100 mm with every gap and loop kept, or is refused `OutlineUnreachable` or `OutlineTooSmall` (`:416-426`). | pass, refusal | Up to LAD ×3: `emit` (`:413`) per `rename::sized(fixture, len)` for len 2, 6, 8 (`:410-411`). Each renamed spec is a different input. A refusal at placement does no routing. FEEDBACK says len 8 is `OutlineUnreachable` and 2 and 6 emit (`docs/work-orders/AET-WO-1313c.FEEDBACK.md:10`, measured before the ladder); `AET-WO-1314.FEEDBACK.md:254` says it emits "three times". Inferred: 2 or 3 ladders (16 or 24 routings); not re-measured. Then `assert_gaps` and `assert_loops` per emitted board. 0 `placement`. |
| `the_fixture_places_at_the_spikes_outline_with_every_gap_kept` | 129.3 | 222.6 | The fixture's outline is 85.25 x 80.95 mm (`:330`) and its name-free placement hashes to the pinned digest (`:335`); every gap holds (`:336`) and every loop rule passes (`:337`). | pass | LAD ×1 (`:328`, the fixture as committed), then SHA-256, `assert_gaps`, `assert_loops`. 0 `placement`. 8 routings. |
| `validate_refuses_a_bound_that_cannot_hold_the_parts` | 10.0 | 23.5 | `validate` exits 2 `OutlineTooSmall` for a 40 x 40 bound (id null) and a 20 x 200 bound (id U1) (`:148-149`), and exits 0 for the committed 100 x 100 (`:154`). | refusal | 3 `validate` subprocesses (`:146`, `:153`) on the fixture with 3 bounds. No emit, no `placement`. |

Not in the input: `a_bound_the_placer_cannot_reach_is_refused_by_name_never_a_partial_board`, `a_version_14_file_is_refused_by_its_version_before_its_shape`, `placement_writes_no_copper_and_refuses_as_emit_does`.

### `supply_tier1` (8 tests; CLI subprocesses; two OnceLocks)

`emitted(label, spec)` is one CLI `emit` subprocess (`supply_tier1.rs:70-85`). `s03_board()` is the OnceLock emit of `sentinel-03.json`; `s024l_board()` is the OnceLock emit of `s024l()` (`power_tree()` with Sentinel-03's `copper_layers`, `layers`, `stackup`, so its fab_tolerances too, `:101-108`; spec not gated).

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `a_drop_over_its_limit_fails_with_the_pitch_note` | 393.7 | 486.9 | A 60 mV limit fails with the "pitch" note and exactly one finding, `supply_drop_over_limit` (`:319-329`). | fail | 1 `extract` subprocess on `s024l_board()` with `gated(s024l(), 60)` (`:317`). `s024l_board()` is the OnceLock LAD (first caller pays). 0 `placement`. |
| `sentinel_03_on_rule_sized_vias_bounds_every_return` | 384.1 | 466.0 | On rule-sized vias every supply and return pin is bounded and the supply verdict is `pass` (`:276`, `:278-281`). | pass | 1 `extract` on `s03_board()` with `gated(s03(), 165)` (`:273`). `s03_board()` is the OnceLock LAD of `sentinel-03.json`. 0 `placement`. |
| `the_four_layer_board_bounds_every_supply_pin_within_its_limit` | 397.5 | 392.0 | **MULTI:** verdict `pass` and source L2/return U11 (`:214-218`); 716 mA (`:221`); 16 supply and 11 return pins, each bounded in 10,000..80,000 uOhm (`:223-229`); drop equals worst pair times load and is at most 165 mV (`:237-238`); no note (`:239`); conditions name each priced tolerance (`:248-264`). | pass | 1 `extract` on `s024l_board()` with `gated(s024l(), 165)` (`:212`). `s024l_board()` is the OnceLock LAD. 0 `placement`. |
| `a_via_pad_the_worst_hole_does_not_fit_refuses_its_pins_with_the_remedy` | 381.2 | 382.1 | At `via_ring_um = 0`, every return pin is refused `SupplyViaPadTooSmall` naming "via pad >= 0.7 mm", exit 1 (`:297`, `:302-307`). | fail | LAD ×1 (`:295`, CLI `emit`) of `gated(s03, 165)` with `fab_tolerances.via_ring_um.value = 0` (`:292-293`), not OnceLock, so a different input from `s03_board()`; plus 1 `extract` (`:296`). 0 `placement`. |
| `a_limit_on_a_domain_the_load_does_not_use_is_refused` | 101.4 | 175.3 | A limit on VDD_5V is refused `SupplyTerminalMissing`, exit 2 (`:203-204`). | refusal | 1 `extract` on `s03_board()` (`:202`, OnceLock LAD) with a spec whose drop limit is VDD_5V. 0 `placement`. |
| `a_specification_without_a_drop_limit_has_no_supplies` | 101.4 | 175.3 | A specification with no drop limit exits 0 and its verdict has no `supplies` key (`:135-136`). | pass | 1 `extract` on `s03_board()` (`:134`, OnceLock LAD) with the committed spec. 0 `placement`. |
| `the_two_layer_power_fixture_refuses_its_ground_return_by_name` | 74.0 | 110.4 | On the two-layer power fixture, 16 supply pins are bounded and every return is refused `GroundReturnUnread`, with one `supply_pin_unbounded` finding per return and no drop, exit 1 (`:340-359`). | fail | LAD ×1 (`:338`, CLI `emit`) of `gated(power_tree(), 165)` (extra stackup keys and a drop limit; same base fixture as `power_board_tier1`, different file bytes); 1 `extract` (`:339`). 0 `placement`. |
| `each_input_the_gate_reads_is_refused_by_name_when_absent` | 85.4 | 2.4 | **MULTI:** each of 4 stackup inputs and 2 fab tolerances missing gives `SupplyInputUndeclared` naming it (`:152-157`, `:169-177`); a missing U4 load gives `SupplySinkUncounted` (`:188-191`). | refusal | 7 `extract` subprocesses (`:151`, `:168`, `:186`) on `s03_board()` (OnceLock LAD) with 7 mutated specs. No other emit. 0 `placement`. |

Not in the input: `a_declared_pair_gets_each_conductors_bound_and_its_mismatch_named` (CLI emit of `pair-standin.json`, no fab_tolerances).

### `escape_tier1` (5 tests; no OnceLock; no fab_tolerances in any spec used, so no ladder anywhere)

`one_geometry(label, spec, seeds)` (`:159-169`) does `emitted` once (a CLI `emit` subprocess, PLAIN, `:53-61`, `:69-76`), then for each seed renames the read-back board and calls `route::route` once (`:166`). `route::route` is the router on a built `Board`, not `emit`.

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `renamings_10_to_19_route_d3_to_its_own_copper` | 155.6 | 200.3 | Each of 10 renamings of D3's board routes to exactly its own copper once names are put back (`:167`). | determinism | 1 PLAIN CLI `emit` of `d3-24-i2c-12-spi.json` (`:194`) plus 10 `route::route` (seeds 10..20). 0 `placement`. |
| `renamings_0_to_9_route_d3_to_its_own_copper` | 172.5 | 195.2 | Same for seeds 0..10 (`:167`). | determinism | 1 PLAIN CLI `emit` of the same D3 file (`:189`) plus 10 `route::route`. Same emit as the test above. 0 `placement`. |
| `twenty_renamings_route_d1_to_its_own_copper` | 30.6 | 59.6 | Each of 20 renamings of D1's board routes to its own copper (`:167`). | determinism | 1 PLAIN CLI `emit` of `d1-24-i2c.json` (`:178`) plus 20 `route::route`. 0 `placement`. |
| `twenty_renamings_route_the_reference_to_its_own_copper` | 31.4 | 46.4 | Each of 20 renamings of the reference's board routes to its own copper (`:167`). | determinism | 1 PLAIN CLI `emit` of `examples/sentinel-01-minimal.json` (`:173`) plus 20 `route::route`. 0 `placement`. |
| `the_reproducers_of_1102_a6_and_2204_a5_route` | 19.0 | 31.5 | Twelve renamed or re-referenced reference specs each emit with exit 0 (`:278`). | pass | 12 PLAIN CLI `emit` (`:277`), each a different mutation of the reference: `FLASH_CS` renamed 7 ways (`:250-253`), the six bus nets renamed `N0..N5` (`:254-264`), 4 references renamed on the first I2C and SPI device (`:265-271`). 0 `placement`. |

Not in the input: `shifted_placements_route_without_reaching_the_exact_judge` (1 PLAIN emit of `four-layer.json` + 5 routes), `a_zone_that_is_not_the_board_outline_is_refused_by_name` (2 PLAIN emits), `a_via_in_a_courtyard_is_refused_but_in_the_dense_parts_own_ring` (2 PLAIN emits of the reference).

### `via_ring_tier1` (4 tests; CLI subprocesses; one OnceLock)

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `a_board_no_rung_fits_is_refused_with_every_net_over_its_bound` | 100.3 | 172.4 | When no rung holds two nets' bounds, `emit` exits 2 `CapacitanceOverBound`, names IMU_SCK and IMU_MOSI "fF over 12000 fF", and writes no board (`:216-223`). | fail | LAD ×1, CLI `emit` (`:215`) of `sentinel-03.json` with `IMU_SCK` and `IMU_MOSI` `max_capacitance_femtofarads = 12000` (`:210-213`). All 8 rungs route, then refuse. Not the OnceLock. 0 `placement`. |
| `sentinel_03_on_rule_sized_vias_passes_every_bound_judged_by_extract_alone` | 110.2 | 172.4 | **MULTI:** vias are only 0.7/0.254 and 0.75/0.3 (`:96-103`); the thermal-via note (`:110-115`); the ladder's golden: rung 7, cost 320, bounded first, IMU_SCK 24,945 fF (`:120-124`); `extract` alone passes 8 lines (`:127-133`); its smallest margin is the ladder's (`:154-158`). | pass | LAD ×1 via `s03_emitted()` (OnceLock `ONCE`, `:64-67`, CLI `emit` of the committed `sentinel-03.json`), plus 1 `extract` subprocess (`:126`). 0 `placement`. |
| `a_board_extract_cannot_judge_is_routed_as_today_and_named` | 13.5 | 110.5 | `emit` of the full power fixture exits 0 with ladder rung 0 and `unjudged.kind = TranslatorPinUnbounded` (`:260-262`); vias are still the rule's (`:266`); `extract` then refuses it by the same kind, exit 2 (`:276-277`). | residual | UNJ ×1, CLI `emit` (`:257`) of `crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json` as committed (crossing kept); 1 `extract` (`:269`). One routing. 0 `placement`. |
| `a_board_without_tolerances_is_routed_as_before` | 6.5 | 50.9 | Without `fab_tolerances`, `emit` exits 0 with no `capacitance_ladder` and no `notes`, and the vias are the class's 0.6/0.3 and the engine's 0.554/0.254 (`:237-247`). | pass | PLAIN ×1, CLI `emit` (`:235`) of `sentinel-03.json` with `board.stackup.fab_tolerances` removed (`:231-234`); a different input from the committed file (still has `max_outline_um`, so the bounded placement runs). One routing. 0 `placement`. |

Not in the input: `a_class_via_under_the_ring_is_refused_by_name`, `each_input_the_ring_rule_needs_is_refused_by_name` (both refuse at `place`, before routing, so no routing).

### `fab_tier1` (6 tests; `verify` subprocesses with `PATH=""`; no OnceLock)

Every spec here is `examples/sentinel-01-minimal.json` or a mutation of it (no fab_tolerances): every `verify` that reaches `generated` does one PLAIN emit (`verify.rs:1090`, `generate.rs:94`) and stops at the refusal named. No ladder, no `placement`.

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `a_campaign_that_misses_a_bus_or_rail_is_refused_by_name` | 36.3 | 52.8 | For 7 scenario sets and each fab flag, `verify` exits 3 `CampaignCoverageShort` naming exactly the uncovered buses and rails, and creates nothing (`:360-368`). | refusal | 14 `verify` subprocesses (`:359`: 7 cases x `--fab-out`/`--against-fab`), the reference, 5 scenario files read each. 14 PLAIN emits, all the same input. Stops before the gates (`verify.rs:1223-1233`). |
| `an_unusable_fab_out_is_refused_before_any_gate_and_left_untouched` | 10.3 | 12.1 | `--fab-out` as a used directory, a file, or a symlink to an empty directory exits 3 `UnwritableOutput`, and nothing in or behind it changes (`:84-91`). | refusal | 3 `verify` (`:71`), reference spec; 3 PLAIN emits. |
| `a_fab_out_another_uid_could_write_is_refused` | 11.2 | 10.3 | A `--fab-out` that is group- or other-writable (0775, 0757), or (as root) owned by uid 65534, exits 3 `UnwritableOutput` (`:181-182`, `:200-201`). | refusal | 2 `verify` (`:168`), plus a third (`:187`) only when the euid is root (`:185`); 2 or 3 PLAIN emits. |
| `a_run_that_cannot_pass_creates_nothing_and_the_flags_exclude_each_other` | 6.1 | 10.3 | **MULTI:** with an empty `PATH` an absent `--fab-out` is not created, exit 4 (`:118-122`); `--against-fab` on an absent dir exits 3 (`:137`); both flags exit 64 (`:154`). | refusal | 3 `verify` calls (`:105`, `:124`, `:139`). Calls 1 and 2 each do a PLAIN emit (call 1 then starts the first gate and fails on the missing tool, exit 4: stop point inferred from the exit code); call 3 is a clap conflict (`main.rs:125-126`), no emit. 2 PLAIN emits. |
| `a_tree_that_differs_writes_no_package_and_says_so` | 6.0 | 10.1 | `--against` an empty tree exits 1 with `against: differs`; with `--fab-out` the verdict is `fab: not_written` and nothing is written (`:228-231`); with `--against-fab` it is `not_derived`/`not_compared` (`:248-249`). | refusal | 2 `verify` (`:213`, `:232`), reference spec; 2 PLAIN emits, then `tree::diff` against an empty dir. |
| `an_unacknowledged_unwitnessed_row_is_refused_by_each_fab_flag_naming_every_one` | 4.4 | 10.0 | Each fab flag exits 2 `UnwitnessedFieldNotAcknowledged` naming all 17 unwitnessed rows, the first (`GPIOB.OTYPER.OT0`) as the id, and writes nothing (`:519-537`). | refusal | 2 `verify` via `rebased_verify` (`:517`, `:476`), on the reference with `GPIOB.OTYPER` and `MODER0` acknowledgements dropped (`:506-515`) and 5 scenarios rebased onto it. 2 PLAIN emits. |

### `unittests src/lib.rs [aethelgard_eda]` (11 tests; no emit, no `placement`, no fab_tolerances in any)

All 11 are in-crate tests on hand-built boards or networks (`supply.rs:1776-`, `flow.rs:1969-`, `pcb.rs:2345-`). None calls `schematic::emit`. `supply::copper` prices one junction table per distinct via geometry per call (`flow::junction_table`, `supply.rs:590-622`).

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `supply::tests::a_relieved_pad_joins_no_plane` | 58.9 | 47.0 | A through-hole pad with `Join::Relief` joins no plane and the note is `ReliefUnpriced`; `Join::Solid` joins In1.Cu (`supply.rs:2341`, `:2346`). | pass | `copper` ×2 (loop over Relief, Solid, `:2314-2350`) on a 20 x 6 mm 4-layer `board4`, one through-hole pad of 0.8 mm drill. No `bound`. Inferred: junction table plus the 4-layer plane cells in an unoptimised test build. |
| `supply::tests::a_through_hole_pad_carries_the_current_as_a_barrel` | 40.4 | 31.2 | A through-hole pad's barrel carries the full `flow::QUANTA` between F.Cu and B.Cu where no via does (`:2012`). | pass | `copper` ×1 and `bound` ×1 (`bound(.., 400)`) on a 2-layer 20 x 6 mm board. |
| `supply::tests::a_path_through_two_vias_is_bounded_above_its_floor` | 29.1 | 39.3 | **MULTI:** bound at least the floor and at most 1.6x it (`:1947-1948`); both vias carry `QUANTA` (`:1949`); a smaller hole raises it (`:1959-1960`); at the largest hole the 0.6 mm pad is left out and 0.7 mm is named (`:1969-1970`). | pass | `copper` ×3 and `bound` ×2 on one board and one routed path, with 3 models (nominal, `hole_minus 80 um`, `hole_plus 130 um`). The 130 um model leaves the via out, so builds no table. |
| `supply::tests::a_declared_dielectric_maximum_bounds_the_barrel_row` | 30.9 | 39.2 | A declared dielectric maximum gives a smaller bound than none (`:2059`). | pass | `copper` + `bound` ×2 (closure `r`, `:2052-2055`; open and tight boards), 2 vias each. |
| `supply::tests::a_hole_inside_every_ports_hull_joins_no_face` | 8.6 | 19.9 | A same-net hole 0.21 mm from a via joins no face on F.Cu (`:2501`). | pass | `copper` ×1, 2-layer board, 2 vias. |
| `supply::tests::a_knockout_beside_a_via_costs_the_plane_only_its_own_side` | 17.2 | 19.5 | A foreign via's knockout removes only the faces on its own side: ports exist, are fewer than the faces beside copper, and none faces the knockout (`:2465-2468`). | pass | `copper` ×1 on `board4` (plane copper), 2 vias. |
| `supply::tests::a_plane_island_is_left_out` | 14.3 | 19.3 | A pocket between four foreign knockouts is dropped from the plane, with an "islands" note, and the rest of the plane stays (`:2402`, `:2409`, `:2415`). | pass | `copper` ×1 on `board4`, 5 vias. |
| `supply::tests::a_port_whose_rays_cross_a_hole_of_the_net_is_not_joined` | 14.7 | 19.3 | Every joined port's hull clears the neighbouring pad's hole at its largest and at every position the tolerance allows (`:2567`). | pass | `copper` ×1, 2-layer, 1 via and 1 through-hole pad; `convex_hull` per port. |
| `supply::tests::a_port_whose_rays_leave_copper_is_not_joined` | 16.5 | 19.3 | A pad 12.5 um beyond the via's pad joins no port, though faces border it (`:2619-2620`). | pass | `copper` ×1, 2-layer, 1 via. |
| `flow::tests::a_via_with_its_hole_anywhere_in_tolerance_is_bounded_above_each_eccentric_floor` | 11.5 | 13.1 | A junction priced for every hole position within +-75 um is at least the exact eccentric-annulus floor at five offsets (`flow.rs:2319`) and at most 1.20x the concentric floor (`:2329`), at 62.5 and 31.25 um pitch. | pass | `via_in_disc(0.075, h)` ×2 (`flow.rs:2306`; h = 62.5 um and 31.25 um; builds a sheet of about 4.6k and 18.5k cells, a 16-box `junction_table`, and `net.bound(.., 2000, 1e-12)`, `:2204-2282`). |
| `pcb::tests::under_a_bound_the_board_does_not_depend_on_the_parts_order` | 7.4 | 10.7 | Under a 100 x 100 bound, every rotation of the 16-part list and its reverse places every part identically (`pcb.rs:2756`, `:2758`), and the reversed list writes the same bytes (`:3416-3419`). | determinism | 33 `layout_of` (`same_in_every_order`: 1 + 2 x 16 parts, `pcb.rs:2751-2760`) with the bound (compaction), plus 2 `write_moved` (`:3417-3418`, one PLAIN routing each at `Rung::TODAY`, `pcb.rs:2088-2103`). Comment at `pcb.rs:3392-3394`: writing the bounded board takes over a second in a test build. |

---


## AET-WO-1910 census, group E (rows-E.md)

Repository `/home/forge/aethelgard-1910` at main 565b32c5, read only. Paths are relative to `crates/aethelgard-cli/` unless they begin with `crates/`. Seconds are the CI figures in `in-E.tsv`. "inferred" marks a reading not proved by a line of code.

**Ladder note (AET-WO-1314).** `emit` runs the eight-routing ladder (`laddered`, `crates/aethelgard-eda/src/schematic.rs:926`; `RUNGS` is 8 long, `crates/aethelgard-eda/src/route.rs:138`) only when `RingRule::of` is `Some` (`crates/aethelgard-eda/src/stack.rs:64-66`, `schematic.rs:892`, `:813`), that is only when the spec declares `board.stackup.fab_tolerances`. Specs with that block in this tree (grep of every `*.json`): `tests/fixtures/sentinel-03.json:868` and `crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json:254`. Of my 15 tests, exactly one emits such a spec: `a_power_tree_solves_into_the_topology_and_emits_its_board` (power tree = `sentinel-02-power.json`, `tests/common/power.rs:8-12`). Every other emitting test in group E uses `examples/sentinel-01-minimal.json` (0 hits for `fab_tolerances`), `sentinel-02-own.json` (0), `d1-24-i2c.json` (0), `four-layer.json` (0), `d3-24-i2c-12-spi.json` / `serial-four.json` (not in the grep hits), or a spec built from `sentinel-01-minimal`, so no ladder runs there.

**A recurring cost (inferred as a major share of time).** `pin_table` rebuilds from bytes on every call (`crates/aethelgard-svd-hal/src/lib.rs:67-71`) and `cmsis::register_facts` hashes and parses the 2.1 MB, 26 972-line `stm32h753xx.h` on every call (`crates/aethelgard-svd-hal/src/cmsis.rs:105-107`, `:26`; header size by `wc`). Neither is cached. `register_fields` (`crates/aethelgard-svd-hal/src/board/ledger.rs:108`), `ledger::build` (`src/ledger.rs:1606`) and `twin::boot::expected` (`crates/aethelgard-twin/src/boot.rs:845`) each call it.

---

### 1. Tables, one per binary

### Binary `board_ui` (`crates/aethelgard-svd-hal/tests/board_ui.rs`, not in the cli crate)

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `misuse_does_not_compile_and_the_driver_does` | 75.1 | 70.7 | MULTI: the 3 `tests/ui/pass/*.rs` drivers compile (`board_ui.rs:12`); each of the 34 `tests/ui/fail/*.rs` misuses fails to compile with its committed `.stderr` snapshot (`board_ui.rs:13`) | fail | One trybuild run (`board_ui.rs:11`): 37 one-file compile units (3 pass, 34 fail by `ls`). 22 include the sentinel-01 board crate (`use sentinel_01_board`), 7 include `tests/ui/motors-pwm/board.rs` (894 lines) by `#[path]`, 8 include `tests/ui/serial-four/board.rs` (1255 lines) (grep of `#[path]`). Dev-deps built first: `aethelgard-cli`, `aethelgard-twin`, `sentinel-01-board`, `trybuild` (`crates/aethelgard-svd-hal/Cargo.toml` dev-dependencies). No `OnceLock`. Once per job. |

### Binary `solve_cli`

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `a_power_tree_solves_into_the_topology_and_emits_its_board` | 41.9 | 54.0 | MULTI: the power tree solves with the stated VDD_3V3 window, source/return pads, input currents (`solve_cli.rs:593-609`); the sense net takes the lowest-position free ADC pin (`:612-635`); `emit` exits 0 (`:645`) and the board has the 11 named references (`:650`), 24 `(drill 0.254)` vias (`:657`), J1's 4 `thru_hole` pads and 4 `(zone_connect 2)` (`:663-664`) | pass | `sentinel-02-power.json` via `power_tree()` (`tests/common/power.rs:8`). `solve` subprocess x1 (`solve_cli.rs:590` via `solved`, `:170-176`), `pin_table("STM32H753ZITx")` x1 (`:610`), then `emit --file ... --out` subprocess x1 (`:638-644`). **This emit declares `fab_tolerances` (`sentinel-02-power.json:254`), so it runs the 8-rung ladder**: 8 routings on threads, each read and bounded by `extract::read` / `extract::bound` (`schematic.rs:951-990`). The emit re-solves inside the CLI. No shared value. |

### Binary `generate_tier1`

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `verify_against_fails_on_one_changed_byte_in_any_file_naming_it` | 38.7 | 53.4 | MULTI: a one-bit change in any of the 14 files makes `verify --against` exit 1, `against`="differs", no gate, one finding `against_differs` whose message starts with the path (`generate_tier1.rs:197-218`); a missing and an extra file are named as `against_missing` / `against_extra` (`:226-244`, assert `:238`) | fail | `reference("against")` (`:195` -> `:77-90`): copy of `boards/sentinel-01`, plus `emit --file examples/sentinel-01-minimal.json` subprocess x1 (`:84`). Then `verify --spec SPEC --against <dir>` subprocess x14 (loop `:198-220`) +x1 (`:225`) = 15, all with `PATH=""` (`:86`, run `no_tools=true`). Each `verify` regenerates the whole tree in memory first (`src/verify.rs:1251-1263`, `src/generate.rs:94,101`: `schematic::emit`, board crate, `twin::platform::emit`) and diffs it. No ladder (sentinel-01-minimal has no `fab_tolerances`). No shared value. |
| `two_generations_are_identical_and_a_used_directory_is_refused_untouched` | 10.5 | 17.7 | MULTI: two `generate` runs give equal reports and trees (`generate_tier1.rs:137-138`); a used directory is refused, exit 3 `UnwritableOutput`, tree unchanged (`:143-145`); a symlink to an empty dir is refused, exit 3, nothing written (`:153-154`); a file target is refused, exit 3, file unchanged (`:160-161`) | determinism | `generate --spec SPEC` subprocess x5 (`:135,136,142,152,159`), `PATH=""`. The first two do the full generate composition of `sentinel-01-minimal`; the last three are refusals (inferred: they stop before generating; not read in `generate.rs`). No ladder. |
| `verify_against_refuses_another_specifications_tree_and_a_hand_written_harness` | 11.0 | 13.3 | MULTI: a tree generated from a spec with the TMP117 removed fails `verify --against` exit 1 naming `kicad/sentinel-01.kicad_sch` and `.bom.csv` (`generate_tier1.rs:268,275-276`); a hand-written vacuous `proofs.rs` is caught as `against_differs` on `board/src/proofs.rs` (`:286-288`) | forgery | `generate --spec other.json` subprocess x1 (`:262`), `verify --against` x1 (`:267`); then `reference("vacuous")` (`:279`: `emit` subprocess x1) and `verify --against` x1 (`:285`). Specs: `sentinel-01-minimal` with TMP117 removed, and `sentinel-01-minimal`. `PATH=""`. No ladder. |

### Binary `orientation_tier1`

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `the_placed_imu_agrees_with_the_declared_orientation` | 40.3 | 39.3 | The emitted board's U5 footprint carries the declared axes and turn for 7 (forward, x) cases: axes `orientation_tier1.rs:209`, turn `:214`, the DRC reader accepts it (`:217`), keep-out size (`:260`) and centring (`:268`) | pass | Library, in process, x7 (loop over `cases`, `:195-203`): `fixture()` read, `spec.validate`, `pin_table`, `aethelgard_ir::solve` (`:30-36`), `schematic::emit` (`:207`), `strict::board` parse x2 (`:218`, `:221`), `route::declared` (`:217`). Spec `crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json` with `board.forward` and U5 orientation edited per case (`:39-51`); 0 hits for `fab_tolerances`, so no ladder. No shared value. |

### Binary `adversarial_1301`

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `attack_a_wide_reference_stays_clear_of_every_other_part` | 24.8 | 30.7 | With 24 I2C devices on 64-character references, no printed reference leaves the outline or meets another part's courtyard or reference, and a short-reference control meets none (`adversarial_1301.rs:200`, `:202`); inner checks that every F.Fab/F.SilkS reference is the one the design says (`:239-259`) | pass | `reference_collisions` (`:227`) x2: `emit` subprocess x1 per call (`:228`), plus `on_fab` in process (`:210-215`: `pin_table`, `solve`, `schematic::anchored`) x1 per call, plus board parse and geometry. Spec: `examples/sentinel-01-minimal.json` with 4 buses x 6 devices (`:156-187`); the control renames references to `S{i}` (`:190-198`) = a different input from the wide one. No ladder. No shared value. |

### Binary `ledger_tier1`

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `verify_s_verdict_carries_the_ledger_and_the_silicon_findings` | 20.4 | 27.4 | MULTI: `verify` of the reference fails against an empty dir with `against`="differs" (`ledger_tier1.rs:582`); ledger total 289 / 67 independent-rule / 164 acknowledged (`:583-598`); silicon holds with the exact CONSOLE USART row (`:587-595`); `declared_peers`, empty `twin_limits` (`:613-617`); key order `findings < declared_peers < silicon < ledger < twin_limits` (`:629-631`); no `fab` keys (`:632`); the 24-I2C/12-SPI verdict under 512 KiB (`:634`); `serial-four` carries 3 declared peers, 4 USARTs and all rows marked unacknowledged (`:639-653`) | pass | `verify --spec X --against <empty dir>` subprocess x3 (`run` closure `:563-579`; calls `:580` `sentinel-01-minimal.json`, `:633` `fixtures/d3-24-i2c-12-spi.json`, `:637` `fixtures/serial-four.json`). No `--fab-out` flag, so it stops at `--against` differs (`src/verify.rs:1254-1268`) before any gate, but after generating the tree and building the ledger. No ladder. No shared value in the test. |
| `every_specification_that_solves_has_a_ledger` | 16.7 | 21.8 | Every `*.json` under `examples`, `scenarios`, `boards`, `crates` (skipping `target`) that parses and solves and whose fields the generator accepts yields a ledger with `total > 0` (`ledger_tier1.rs:259`), and at least 15 do (`:263`) | pass | In process, per JSON file: `solve` (`pin_table` + `aethelgard_ir::solve`, `:33-41`), `register_fields` (`:254`), `ledger::build` (`:257`, which calls `register_fields` again, `src/ledger.rs:1606`). So two CMSIS header parses and one pin-table build per file that gets that far. Files visited: 3 (`examples`) + 28 (`scenarios`) + 0 (`boards`) + 39 (`crates`) = 70 JSON files by `find` (before the `target` skip). No `emit`, so no ladder even on `sentinel-03.json` / `sentinel-02-power.json`, which this test reaches by solve only. Shared: library `LazyLock` `ledger::TABLE` (`src/ledger.rs:415`). |

### Binary `aethelgard` (unittests `src/main.rs`; module `renode`, `src/renode.rs`)

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `renode::tests::a_hook_line_out_of_shape_is_refused_and_one_lagging_is_cpu_halted` | 10.4 | 16.8 | MULTI: a hook line in no shape is refused (`renode.rs:4575`); a lagging count gives one `cpu_halted` and falls back to its stamp (`:4584-4590`); a count ahead of its stamp (wrong start) is refused (`:4597`); scheduled-action lines one grain early are refused, one grain late are `cpu_halted` (`:4608`, `:4619`); exact action lines of any shape are read (`:4629`) | refusal | No Renode, no subprocess. `judge_anchors` x13 (`:4565-4640` by count: 1+1+1+1+1 single calls, 4 in the first loop, 3 in the second, 1 final), each over a built `dshot_log()` of 9 438 lines (`:4237-4268`: 4716 x 2 edge lines + 6) with the log rebuilt per call (14 builds). Each `judge_anchors` calls `dshot_parts()` -> `image()` (`:4211`, `:3660`) -> `twin::boot::expected` (parses the CMSIS header, see top note), then `hooks_of`, `observer_edges`, `own_lines`. Shared: `INSTANCES` `LazyLock` (`:3703`) only. |
| `renode::tests::each_anchor_bound_fails_by_name` | 5.8 | 13.6 | MULTI: ten named findings, each from one edited DShot log: `motor_edge_after_window` (`renode.rs:4357`), `motor_edge_before_anchor` (`:4366`), `motor_anchor_repeated` (`:4377`), `motor_anchor_out_of_order` x3 (`:4386,4390,4394`), `judge_stop_missing` (`:4405,4416`), `judge_stop_mismatch` (`:4406,4428`), and `motor_anchor_unobserved` with status `unanchored` (`:4438`) | fail | Same pipeline as the row above. `judge_anchors` x12 (11 `kinds(...)` calls and the final direct call, `:4344-4440`), on one base `dshot_log()` (built once, `:4349`) cloned per case. Each call re-runs `image()` (CMSIS parse). No shared value except `INSTANCES` (`:3703`). |

### Binary `support_tier1`

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `a_capacitor_moved_off_its_pin_fails_and_an_unbounded_one_does_not` | 9.9 | 16.0 | A bounded capacitor (`vdd_bypass`, `vcap`, `vdd_bulk`) moved 25 mm off the board gives exactly `["support_capacitor_missing"]` (`support_tier1.rs:230-235`); the unbounded NRST one (`reset`) moved the same way still passes with no findings (`:237`) | fail | `reference()` x2 (`:215`, and again inside `passing()` `:216`; each is `validate` + `pin_table` + `solve`, `:21-27`). `passing()` also `schematic::emit` x1 (`:99`) and a support judge. Then per case x4 (`:220-239`): `schematic::emit_moved` (`:227`), `pads_of`, `judge` (`:228`). So 1 emit + 4 emit_moved, in process, spec `sentinel-01-minimal.json`, no ladder. No shared value. |

### Binary `stackups_tier1`

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `d1_routes_at_four_layers_and_at_two` | 9.1 | 12.9 | MULTI: the 24-I2C D1 fixture emits at 4 layers (exit 0 `stackups_tier1.rs:235`, `In1.Cu` present `:236`); `extract` of that board passes, 10 lines each `pass` and within the declared bound (`:241-246`); with the reference's 2-layer board it emits and has no `In1.Cu` (`:254-255`) | pass | `emit` subprocess x2 (`:234`, `:253`) and `extract` subprocess x1 (`:240`). Spec `fixtures/d1-24-i2c.json` (`D1`, `:14`), the second with `board` replaced by the reference's (`:248-250`) = a different input. Neither has `fab_tolerances` (0 hits), no ladder. No shared value. |

### Binary `strict_tier1`

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `every_corpus_file_is_refused_or_reaches_its_gate_as_stated` | 9.4 | 12.0 | For all 55 corpus edit files and every gate each names, a strict refusal is exit 3 with the stated kind and id, or `extract`'s own verdict; an accepted file reaches the tool lookup as exit 4 `ToolMissing` (`strict_tier1.rs:73-76` equal triple, `:81` ToolMissing) | refusal | `corpus::reference` (`:51` -> `tests/common/corpus.rs:190`): `emit --file examples/sentinel-01-minimal.json` subprocess x1. Then `corpus::gate` subprocess x153 with `PATH=""` (`:68-69`): 43 board files x 3 gates (`drc`, `drc-parity`, `extract`) + 12 schematic files x 2 (`erc`, `drc-parity`) (`corpus.rs:167-171`; 55 `.edit` files by `find`; gate counts from the `# drc/drc-parity/extract/erc` header lines: 43, 55, 43, 12). Each is a process start plus a strict read. No ladder. No shared value. |

### Binary `subgrid_tier1`

| test | tools s | gates s | the distinct failure it guards | class | expensive work |
|---|---|---|---|---|---|
| `a_part_moved_inside_the_board_keeps_the_outline` | 9.9 | 10.1 | A part (R1) moved within the board at three heights leaves the emitted outline equal to the placer's own (`subgrid_tier1.rs:820`) | pass | In process: `pin_table` + `solve` x1 (`:801-802`), `schematic::emit` x1 (`:812-815`), `schematic::emit_moved` x3 (`:819`, loop `:817`), each board re-read with `strict::board` + `route::read` (`:803-811`). Spec `examples/sentinel-01-minimal.json`, no ladder. No shared value. |

---

