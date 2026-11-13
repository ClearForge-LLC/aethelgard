# AET-WO-1907 — CI's `tools` job in four shards, under 15 minutes, with every test run exactly once

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1907-ci-shards`.
**Author:** Claude (architect) · **Builder:** a cheap builder with a targeted adversarial pass (`docs/roadmap.md` §6).
**Phase:** infrastructure. Scotty's direction of 2026-11-13: move the split forward, four shards, under 15 minutes. It replaces the backlog row "Split the `tools` job's tier 2 into two jobs" (`3200` F17).

**Grounds (measured by the architect from PR #99's `tools` run, head 377dccc):**
- **The job took about 47 minutes.**
  - Pulling the image: 60 s.
  - Building the tests: 62 s.
  - The bridge, fmt and clippy: about 25 s.
  - The test step: **2,662 s**.
- **The test step by binary:**
  - `board_tier2.rs`: 1,551 s;
  - `attribute_seeded.rs`: 560 s;
  - `adversarial_1301_tools.rs`: 82 s;
  - `adversarial_1301.rs`: 76 s;
  - `twin_tier2.rs`: 63 s;
  - `boot_tier1.rs`: 62 s;
  - every other binary: under 45 s each, about 330 s together.
- **So a split by file cannot go under 26 minutes.** The unit of a shard must be the individual test.
- **The floor is the slowest single test.** `board_tier2`'s `verify` tests run `verify` once or twice, about 140 s each on Furnace and slower on CI's runners. Measure them (§1.1).
- **The tier-2 canary** (`TIER2_MIN`, `AET-WO-1900`) counts passing tests in named tier-2 files from one log. Sharded, it must count across every shard.

> **What this is:** the same tests, the same image, the same features, in four parallel jobs balanced by measured weight, with two guards. Every test is assigned to exactly one shard, and the canary floor holds across the shards.
>
> It is NOT:
> - a test change;
> - nextest;
> - a new image;
> - fewer tests.

**Cadence:** measure (§1.1), then build. Stop on §7. One PR, left unmerged.

## 1. Scope

### 1.1 Measure each test's weight

Measure each tier-2 test's wall time, and each test above 20 s in the other binaries. Run it alone (`cargo test … --test <bin> -- --exact <name>`) in the pinned image on Furnace, with the CI job's own flags. Record the table in FEEDBACK.

Commit the weights as data, for example `ci/test-weights.tsv` with one `binary<TAB>test<TAB>seconds` per line. Say in FEEDBACK how Furnace's times relate to CI's: the architect's figures above are CI's, so compare.

### 1.2 Assign at run time, from the live list

A small script (`ci/shard.sh`, shell plus the image's existing tools only):
- lists every test the `tools` job runs today (`cargo test --workspace --all-targets --locked --features aethelgard-cli/with-tools -- --list`, together with the bridge's tests);
- assigns each test to one of four shards, heaviest first, onto the lightest shard (LPT);
- gives a test missing from the weights file a stated default weight, so a new test is still assigned;
- writes each shard's exact list.

Because the list is computed in CI from the code, a new test can never be left unassigned. **A guard step fails the run** unless the union of the four lists equals the full list and no test appears twice.

### 1.3 Four shard jobs and one aggregate

- **The `tools` job becomes a four-way matrix.** Each shard:
  - pulls the same pinned image;
  - builds with the same command (cache the `target` directory with `actions/cache` keyed on `Cargo.lock` and the image digest, if it measurably helps);
  - runs only its list, with `--exact`.
- **The non-test steps run once, in shard 0:** the bridge build and tests, fmt and clippy.
- **A `tools-tier2` aggregate job** needs all four shards. It sums each shard's tier-2 passes (each shard uploads its count as an artifact or a job output), checks the sum against `TIER2_MIN`, and checks the guard of §1.2.
- **The required status checks in branch protection may name `tools`.** Keep a job called `tools`, the aggregate for example, that is green only when all four shards and the guard are green. Say in FEEDBACK what Scotty must change in branch protection, if anything. This work order cannot change branch protection.
- **Each shard keeps today's caps and environment.** That includes the `TMPDIR` rule (`AET-WO-1601`) and the closing of leaked descriptors.

### 1.4 Prove it on CI

- **CI on the PR is the measurement.** The slowest shard's wall time, from job start to end, must be under 15 minutes.
- If it is not:
  - report the per-shard times;
  - try once to rebalance with CI's own measured times;
  - if four shards cannot reach 15 minutes because a single test is longer, report that test, which is a §7 stop.
- `gates` (about 10 minutes) is out of scope.

## 2. Invariants and fences

- `INV-06` (a shard that cannot list its tests fails, never runs nothing), and the canary's purpose (`AET-WO-1900`): fewer tests running than the floor fails CI.
- **Fence:**
  - `.github/workflows/ci.yml`;
  - the new `ci/` directory;
  - this work order's FEEDBACK.
- **No test, no crate, no image changes.**
- **`AET-WO-3300` and `AET-WO-1306` may land first, and both raise `TIER2_MIN`.** Merge `origin/main` with a merge commit, and keep the floor exact.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance

- **On the PR's CI:**
  - four shards run;
  - the guard passes;
  - the aggregate's tier-2 sum equals `TIER2_MIN`;
  - the slowest shard finishes in under 15 minutes. Record all four times.
- **The total number of tests run** across the shards equals today's single-job total on the same commit. Show both counts.
- **The guard catches each failure it exists for.**
  - In a scratch commit, or by unit test of the script: an unassigned test, a doubled test, a shard that lists nothing, and a tier-2 file whose tests silently stop running each fail the run.
  - Show each.

## 5. Adversarial pass — targeted

1. **Make a test run in no shard, or in two,** without the guard failing: a test name that is a prefix of another (`--exact` matters), a test in a new binary, a test behind a feature, or the bridge's tests.
2. **Make the canary pass with fewer tier-2 tests run than the floor.**
3. **Make a shard pass while running nothing.**

## 7. Flag-and-stop conditions

- A single test exceeds about 12 minutes on CI, so no number of shards gets the job under 15.
- Sharding changes a test's result, so a test that passes alone fails beside others, or the reverse. Report it; do not fix the test here.
- The required-check name cannot be kept without a branch-protection change the builder cannot make. Report the exact change.
