# AET-WO-1913 — CI's fourth refit: lower the rows that sit high, so the spread comes inside the REFIT RULE's 6 minutes

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1913-refit-lower`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** CI hygiene. Written 2026-11-20. **Cadence:** a measurement, then a weights-only build.

**Grounds** (`1912` D1 and D2, ruled at #286's merge; roadmap §7.1):
- After `1912`, every shard is under 20 min, but the spread was 7.05–7.2 min, over the REFIT RULE's 6.
- **The causes, as `1912` measured them:**
  - `twin_tier2`'s rows sit high: 1,807 s of rows against 1,201–1,390 s run, and 51 s against 277 s of rows on shard 0;
  - `supply_tier1` has two modes, 292–444 s and 621–720 s;
  - `four_layer_fab_tier2` (625 against 740.63) and `verify_47k_tier2` (220 against 277.29) sit under their rows, within 25 %.
- `1912`'s fence allowed raising rows, not lowering them. **This work order may lower them.**

> **What this is:** every `twin_tier2` and `board_tier2` row re-measured alone (REFIT RULE step 1), lowered or raised to its alone time, and the two under-rows raised to their worst. Weights only.
>
> It is NOT a change to the planner, the shard count or any test.

## 1. Build

1. **Measure first:** the last five green `main` runs, per shard (wall time, the planned and run times of each colocated block, every test of 30 s or more), committed as FEEDBACK §1 before any row changes.
2. **Alone times** in a throwaway commit (REFIT RULE step 1) for every `twin_tier2` and `board_tier2` test, and for `four_layer_fab_tier2` and `verify_47k_tier2`. Revert the commit before READY.
3. **Set each row to its alone time.**
   - A lock-taking test is weighed only from an alone run.
   - `supply_tier1` stays at its worst mode (721 s). If its two modes are the remaining spread, say so; do not split it here.
4. **The plan and the PR's CI:** every shard under 20 min. Report the spread; under 6 min is the target.

## 2. Acceptance

- **The table,** before and after, with each row's run and job.
- **Every shard under 20 min** on the final run, and the spread reported.
- **The CLAUDE.md gates pass.** `TIER2_MIN` is unchanged.

## 3. Fences

- **Fence:** `ci/test-weights.tsv`, `ci/colocate.tsv`, the throwaway (reverted), `spike/1913/`, FEEDBACK.
- **Not:** `ci/shard.sh`'s logic, `ci/SHARDS`, workflows, tests or crates.
- **Running beside you:** `1211a`, `4240b`, `4240c`, `1213`, each adding tests. A row they add is theirs. Merge `main` before READY, and re-plan on the merged tree.
- One PR, left unmerged. Write `~/.cache/aet-1913/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A row lowered from an alone run** that, colocated, waits on a lock and over-runs its shard.

## 7. Flag-and-stop conditions

- **Lowering puts a shard over 20 min.** Stop with the table.
