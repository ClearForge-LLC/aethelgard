# AET-WO-1914 — The shard planner's P3: time-stamped lines and a 10 s threshold, and the refusal fixtures that check their reason

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1914-planner-p3`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4, backlog wave 1, last in the wave. Written 2026-11-23. **Cadence:** a measurement, then a build.

**Grounds** (roadmap §7.1; read `1910` and `1907` FEEDBACK first):
1. **`1910` D6 (#233), the planner's P3.** Time-stamped log lines, and a 10 s threshold that refuses an unweighted tier-2 test in the 10–60 s band that P2 (the 60 s line) does not catch.
2. **`1907` review.** Three `ci/test-shard.sh` refusal fixtures print the runner awk's `\.` warning instead of their reason, and they check only the exit code. Make each fixture assert its reason. The triage could not reproduce the warning on Furnace's awk, so measure which awk prints it first.

> **What this is:** (1) and (2), each with a fixture that fails without it.
>
> It is NOT:
> - a re-weighting of `ci/test-weights.tsv` beyond the rows the measurement requires;
> - a shard-count change;
> - any crate change.

## 1. Build

1. **Measure first,** with predictions in `spike/1914/`:
   - which tier-2 tests on `main` fall in the 10–60 s band without a weight, from the last three `main` runs' logs;
   - the awk warning: which awk build prints it, and on which fixtures.
2. **(1) and (2).**
3. **Avoid failing the other builders' CI.** If the measurement finds unweighted tests in the band, add their measured weights in this same PR, so P3 lands green. Name each added weight.

## 2. Acceptance

- **P3** refuses an unweighted 10–60 s test, and passes on `main` once this PR's weights are in.
- **Every refusal fixture** asserts its reason, not only its exit code.
- **The CLAUDE.md gates pass, and CI is green.**

## 3. Fences

- **Fence:** `ci/shard.sh`, `ci/test-shard.sh`, `ci/fixtures/`, `ci/test-weights.tsv` (the measured rows only); `spike/1914/` and FEEDBACK.
- **Not:** any crate, specification, `fab/` file, or workflow.
- **Running beside you:**
  - `4300i`;
  - `4407`;
  - `1607`;
  - `2309`.

  Each may add rows to `ci/test-weights.tsv`. Merge `main` before READY, and re-run the planner on the merged weights.
- One PR, left unmerged. Write `~/.cache/aet-1914/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A test whose time straddles 10 s or 60 s** across runners.
- **A log line without a time stamp** that P3 silently skips.

## 7. Flag-and-stop conditions

- **P3 would refuse `main` for a test whose weight cannot be measured** from a held run. Stop and name it.
