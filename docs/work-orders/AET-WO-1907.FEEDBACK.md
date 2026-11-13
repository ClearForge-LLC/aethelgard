# AET-WO-1907 — FEEDBACK

**Status: built to every §3 item except the 15-minute line.** Four shards run, the guard and canary pass, the totals match; the slowest shard is 16 m 30 s. See Result and the decision there.

**Branch:** `wo/aet-1907-ci-shards` · **Base:** `72af687` (`origin/main`, carrying `AET-WO-1306` and the docs PR), merged by a merge commit · **Builder:** Claude Code (Sonnet 5.5) as `forge` on Furnace, as project manager · **Cadence:** measure (§1.1), build, targeted adversarial pass by a fresh subagent.

**Fence kept:** `.github/workflows/ci.yml`, the new `ci/` directory, and this file. No test, crate or image change.

## Result

- **Furnace, all four shards locally, before the merge of `AET-WO-1306`, the pinned image, one after another:** 419 s, 458 s, 500 s and 566 s. Together they passed **1,004 tests**, and the single-job baseline on the same commit passed **1,004** (bridge 1 + workspace 1,003 names with one name in two binaries). The aggregate printed `tier2_tests_passed=74 floor=74` on that tree.
- **CI on PR #104 (job start to job end), two runs.** The single job took about 47 minutes on PR #99. Every run was green: guard ok, 1,019 passes for 1,019 listed occurrences (1,018 names), tier-2 sum 75 = `TIER2_MIN`.

| shard | run 1 (`80c21b1`, LPT on Furnace weights) | run 2 (`91679a6`, CI-scaled weights, `attribute_seeded` kept whole) |
|---|---|---|
| 0 | 15 m 14 s | 16 m 27 s |
| 1 | 12 m 28 s | 15 m 48 s |
| 2 | 18 m 48 s | 14 m 40 s |
| 3 | 15 m 46 s | 16 m 30 s |

  Test step alone, run 2: 13 m 45 s, 13 m 31 s, 12 m 04 s, 14 m 10 s. Before it, every shard spends about 2 m 25 s on setup: image pull 46 to 72 s, crate fetch and build 70 to 78 s, bridge 4 s; shard 0 adds fmt and clippy, 25 s.
- **The slowest shard is not under 15 minutes. It is not one test.** The longest single test takes about 6 minutes on CI, far under the §7 12-minute line. The sum of every shard's `finished in` is 3,203 s (822, 809, 723, 849 in run 2), so a perfect split gives 801 s, 13.35 minutes, plus 2.4 minutes of setup: **about 15.9 minutes at best.** The §1.4 rebalance was tried once, and it moved the spread (14 m 40 s to 16 m 30 s) without reaching the line. I did not try a second one, since the arithmetic says it cannot.
- **Where the time is.** `board_tier2` is 1,630 CI-seconds, serial: its heavy tests take a `Mutex` (`ONE_AT_A_TIME`, `board_tier2.rs:25`), so its tests do not overlap even with 20 cores (859 s in parallel against 872 s alone on Furnace). `attribute_seeded` overlaps (12 tests in 641 s together, 861 s when split four ways). The rest is about 700 s of small binaries, run one after another.
- **Decision needed (the architect's).** Under the 15-minute line with four shards would need one of these. None is in this work order's fence:
  1. **A fifth shard**, or a runner with more than 4 vCPU, against the 15-minute line (about 13 minutes with five).
  2. **Running `board_tier2` beside the other binaries in one shard**, which would overlap its serial work. I did not build this: several tier-1 tests read `/proc` for surviving processes (`mcp_tier1.rs:409`, `tools_tier1.rs:1664`) and `board_tier2.rs:618` lists `/var/lib/aethelgard`, so tests in separate binaries that never ran at once would run at once. That is the §7 condition "sharding changes a test's result" waiting to happen.
  3. **Accept about 16 minutes**, 3 times faster than the 47-minute job, with `timeout-minutes: 30` on each shard.
  4. **`actions/cache` on `target`** could save part of the 70 s build. I did not try it: even a free restore leaves 14.7 to 15.2 minutes at perfect balance, and the run 1 and 2 spreads show about 1.5 minutes of imbalance that a cache does not fix.
- **The failure cases of §3 are shown by `ci/test-shard.sh`** (29 fixture checks, run in the `tools` aggregate on every PR). Each refuses: a test in no shard, a test in two, a shard that lists nothing, a name run once though listed in two binaries, an empty or malformed list, a benchmark, one test fewer than assigned, a shard that passes while running nothing, a failed test, a missing shard, a tier-2 sum under the floor, and tier-2 tests that silently stop.

## §1.1 Measurement

**Setup.** The pinned tools image `…@sha256:c69ff9bc…` (`container/IMAGE`), rootless podman 5.7 on Furnace, in the CI layout (`/repo`, `CARGO_HOME` and `CARGO_TARGET_DIR` under `/repo/target`), `--network none` for the runs, `--memory 24g --memory-swap 24g --pids-limit -1`, `0` written into `memory.swap.max` after start. Read back for every container: `memory.max=25769803776 swap=0 pids=max`.

**`TMPDIR` fork (F1).** The kickoff said to set `TMPDIR` to a 0700 directory. I did that first. `AET-WO-1601`'s `as_root_a_third_uid_cannot_change_what_build_reads` fails when `TMPDIR` is set (it looks only under `/var/lib/aethelgard`), and CI's `tools` job sets none. The numbers here are with `TMPDIR` unset, the CI shape. The first baseline with it set failed that test.

**Disk fork (F2).** The first baseline ended `No space left on device` (the host disk was 100% full; another session freed it). I discarded it and re-ran the whole baseline. I deleted nothing.

**The baseline:** 23 min 48 s on Furnace, 1,004 passed, 0 failed, 0 ignored. By binary (parallel inside a binary, as the job runs it): `board_tier2` 859 s (12 tests), `attribute_seeded` 309 s (12), `adversarial_1301_tools` 51 s, `adversarial_1301` 49 s, `boot_tier1` 32 s, `generate_tier1` 23 s, `twin_tier2` 16 s, `board_ui` 12 s, `with_tools` 11 s.

**Weights, each test alone** (`cargo test --workspace … -- --exact <name>`, the job's flags), for every test in a binary of 10 s or more, the tier-2 binaries, and the two `adversarial_130x` ones: 127 tests, in `ci/test-weights.tsv`, plus one added after the merge (`every_corpus_file_the_reader_accepts_gets_its_stated_kicad_verdict`, 37.18 s). Each run printed `1 passed`. The ones above 20 s:

| s | binary | test |
|---|---|---|
| 307.8 | `attribute_seeded` | `tier2::p3_gate_clause_2_propose_answers_every_seeded_case_as_its_rule_says` |
| 271.6 | `board_tier2` | `verify_binds_the_reference_and_two_runs_print_the_same_bytes` |
| 131.7 | `board_tier2` | `verify_fails_the_47k_pull_up_with_the_campaigns_findings` |
| 113.0 | `board_tier2` | `the_generated_board_proves` |
| 110.8 | `board_tier2` | `a_missing_clock_enable_fails` |
| 110.1 | `board_tier2` | `a_write_past_the_block_fails` |
| 76.3 | `board_tier2` | `a_wrong_alternate_function_fails` |
| 74.0, 71.9, 70.1, 69.6, 69.6 | `attribute_seeded` | five more |
| 50.6, 47.8 | `adversarial_1301_tools`, `adversarial_1301` | `attack_wide_references_…`, `attack_a_wide_reference_…` |
| 46.7 | `board_tier2` | `a_vacuous_harness_fails` |
| 32.1 | `boot_tier1` | `a_boot_past_the_wall_bound_fails_and_names_it` |
| 23.2 | `generate_tier1` | `verify_against_fails_on_one_changed_byte_in_any_file_naming_it` |

The sum of the measured tests alone is 1,965 s. A test missing from the file gets 1 s (`DEFAULT_WEIGHT`, `ci/shard.py`); the unmeasured remainder is about 880 fast tests.

**Furnace against CI.** The architect's CI figures for the same binaries, `board_tier2` 1,551 s and `attribute_seeded` 560 s, are 1.8 times Furnace's 859 s and 309 s. Applied to the slowest single test, 308 s on Furnace, that is about 560 s on CI, under the 12-minute §7 line (720 s). The weights are Furnace's alone-times. Tests in one binary of one shard run in parallel, so the plan's sum is a balance, not a prediction of wall time. Running all four shards, the sum of weights per shard is 731 s; the measured Furnace wall times were 419 to 566 s, a ratio of 0.57 to 0.77.

## What was built

- `ci/shard.sh` — `plan` lists every test of the job from the code (`cargo test --workspace … -- --list` and the bridge's), assigns, and runs the guard; `run K` runs shard K.
- `ci/shard.py` — the pure functions: `parse`, `assign` (LPT, heaviest first onto the lightest shard, ties by lowest index; sort key a total order, so every shard computes the same plan: checked with three `PYTHONHASHSEED` values), `guard`, `result`, `aggregate`. Python 3 is in the image; `jq` is not.
- `ci/test-weights.tsv`, `ci/test-shard.sh`.
- `ci.yml`: the `tools` job is now `tools-shard` (matrix 0 to 3, `fail-fast: false`, each shard pulls the image, builds, builds the bridge, and plans) and an aggregate **`tools`**. Shard 0 alone runs fmt and clippy. The bridge is built in every shard, because the tier-2 tests look for it beside `aethelgard`; its tests are in the list and sharded like any other.
- **The unit is the test name, per suite.** `--exact` matches by name in every binary of the suite, so a name in two binaries (one exists: `an_invalid_specification_is_refused_as_validate_refuses_it`) runs in the same shard, and the shard must see as many passes as the list has occurrences. `-p <pkg>` was ruled out: it recompiled serde and syn (a different feature build), and `--test <bin>` is ambiguous (`adversarial_1300` and `adversarial_1301` are in both `aethelgard-cli` and `aethelgard-eda`). Running the executables directly was ruled out: `firmware.rs:317` reads `RUSTUP_TOOLCHAIN`, which cargo passes down.
- **Each shard checks its own run.** `result` requires passes equal to the assignment, per suite, and fails on any failed or ignored test, and on an empty assignment. The `--exact` list is never empty (an empty one would run everything). The aggregate checks all four shards left a result, saw the same list (byte-identical), partition it (the guard re-run over their own `shard.tsv`), and passed exactly what was listed; then it sums the tier-2 passes against `TIER2_MIN`.
- **The canary** keeps its regexp (`TIER2_RE` in `shard.sh`, the old awk's alternation) and its floor, now 75 with `AET-WO-1306`. The comment history of `TIER2_MIN` is intact.

## Branch protection (§1.3, §7)

Nothing to change if a required check is named `tools`: the aggregate keeps the job id and name `tools`. It runs `if: always()` and its first step fails unless `needs.tools-shard.result` is `success`, so a failed, skipped or cancelled shard is red. **Scotty may add `tools-shard (0)` to `(3)` as required checks; that is optional.** I could not test a required check here.

## Facts the work order assumed that were not so

- **The `TMPDIR` rule and the closing of leaked descriptors are not in today's `tools` job.** The descriptor closing is in the `gates` job's test step only, and no `TMPDIR` is set anywhere in `ci.yml`. "Each shard keeps today's caps and environment" was kept literally: CI's `tools` job has no memory cap and sets no `TMPDIR`. I added none.
- **`--all-targets` is not what the job runs.** The job's test step is `cargo test --workspace --locked --offline --features aethelgard-cli/with-tools`; the build step uses `--all-targets --no-run`. The list uses the test step's selection. The two lists have the same 991 tests at base, but `--all-targets` adds no doc-tests and the job does run them (there are none: every `Doc-tests` block says 0).

## Adversarial pass (targeted, a fresh subagent)

All three attacks held. A claim I checked: the subagent reported the assignment identical across `PYTHONHASHSEED` 1, 2 and 3; I re-ran `plan` after the merge and each shard's list was disjoint and complete (`guard ok: 1018 names, 1019 occurrences`).

| # | attack | result |
|---|---|---|
| 1 | a test in no shard or two: a name that prefixes another, a new binary, a feature-gated test, the bridge's tests, a name in two binaries or both suites, odd characters, doc-tests | held. Doc-test names contain spaces and are refused by `parse` (fail closed). |
| 2 | the canary passing under the floor | held. One latent flaw, found and fixed: a doc-test result right after a tier-2 binary was credited to it (the old awk had the same flaw; it cannot fire now). `tally` resets on a `Doc-tests` line, with a fixture. |
| 3 | a shard passing while running nothing: empty list, 0 tests, stale logs, exit codes, `if: always()`, skipped shard | held. |

Other findings, taken: `upload-artifact` gets `overwrite: true` (a re-run of failed jobs would hit "artifact name already exists"); this file now says that **a test marked `#[ignore]` fails the shard** (the old job tolerated it; there are none now).

## Reduction pass

Removed: `plan.sha256` (written, uploaded and compared, but the byte-identical `entries.tsv` and the re-run guard already prove it); the old `plan.sha256.run` copy; the old single-job canary step and `awk` (replaced by `shard.py`); the 90-minute timeout and its "splitting is backlog" comment; the separate bridge-test step (the bridge's tests are in the list). Kept: the guard run in both the shard (`plan`) and the aggregate, because the shard proves its own list and the aggregate proves the four agree.

## Open and backlog

- **`TIER2_RE` is a copy** of the old awk's alternation, in `shard.sh`. It must stay in step with the tier-2 files, as the old awk did.
- **A list line the parser does not know fails `plan` in all four shards.** Cargo replays warnings from a cached build into stderr, so a future warning would do it; clippy's `-D warnings` keeps it clean now. Fail closed, by design (INV-06).
- **No `actions/cache`.** The target directory is 3 GB against a build of about a minute.
- **A rebalance** was done once (run 2): weights are scaled by `CI_SCALE` 1.8 (every tier-2 test measured on CI took 1.7 to 1.9 times Furnace's alone-time), and `ci/colocate.tsv` keeps `attribute_seeded` whole. A further one needs per-test CI times, which a libtest `--report-time` run would give.
