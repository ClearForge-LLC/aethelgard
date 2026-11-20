# AET-WO-1912 — CI's third refit: `supply_tier1` and every row the REFIT RULE now finds lagging, measured on `main`

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1912-refit`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** CI hygiene, any phase. Written 2026-11-20. **Cadence:** a measurement, then a weights-only build.

**Grounds** (`1318` F1, F2, F6, ruled 2026-11-20; `3306`'s READY note; `ci/shard.sh`'s REFIT RULE):
- **The REFIT RULE has fired on `main`.**
  - Run 44706411946 (`bb1bcb8c`), shard 8: 20.1 min.
  - Run 44735701946, shard 8: 23.0 min.
  - `3306`'s run 44741362475: shard 8 took 25.7 min and shard 10 took 20.4 min. All of them were green.
- **`supply_tier1`'s colocate row is 302 s** (`1911`'s second refit). Its block measured 309–739 s on eight runs (`1318` F6), and it drove the overrun in each. `1910` had it at 795.25 s.
- **`board_tier2::a_missing_clock_enable_fails`** ran 62.42 s once against a 6.3 s row, and 6.0–7.0 s on four other runs (`1318` F2). That could be a lock wait or a real outlier.
- **`1318`'s rows** (`each_size_sample` 156.94 s, `outline_tier1` 251 s) and **`3306`'s new tier-2 row** (3.9 s, a Furnace estimate) are recent.

> **What this is:** the REFIT RULE (`ci/shard.sh`, steps 1–3 and 5), run once on `main`'s current head, for every row a measured run shows lagging. `supply_tier1` is first. Weights and colocate rows only.
>
> It is NOT:
> - a change to the shard count (`ci/SHARDS` stays 12, `1911` D1);
> - a change to the planner, the guard or any test;
> - a speed-up of any test.
>
> If a block cannot be weighed so every shard plans under 20 minutes, that is a finding for a split, not a fix here.

## 1. Build

1. **Measure first.** Take the five most recent green `main` runs (`gh run list --workflow ci.yml --branch main`). For each shard, record:
   - its wall time;
   - the planned seconds against the measured `finished in` of every colocated block;
   - every test of 30 s or more against its row.

   Commit the table in FEEDBACK §1 before changing a row. Name every row that sits more than 25 % under its worst measured time.
2. **Alone times,** by step 1 of the REFIT RULE: in a throwaway commit, for every lagging test that is not colocated. Revert the commit before READY. A test that takes a lock is weighed only from an alone run (step 2's warning).
3. **Colocated blocks** (`supply_tier1` and any other lagging one) at their worst in-process wall time over the five runs and the throwaway run (step 3).
4. **`a_missing_clock_enable_fails`:** decide from its alone time and the five runs whether 62.42 s was a lock wait. Raise its row only if an alone run shows it. Say which.
5. **The plan.** `ci/shard.sh plan` on the new rows: every shard's planned seconds, the spread, and the guard. Then your PR's CI: every shard under 20 minutes. If one is not, stop with the table.

## 2. Acceptance

- **The table** (§1) before and after, with each row's source run and job.
- **Every shard of the PR's final run is under 20 minutes,** and the spread is under 6 minutes, or the stop below.
- **The CLAUDE.md gates pass.** `TIER2_MIN` is unchanged.

## 3. Invariants and fences

- **Fence:**
  - `ci/test-weights.tsv` and `ci/colocate.tsv` (rows and their provenance comments);
  - the throwaway commit (reverted);
  - `spike/1912/` for scripts;
  - FEEDBACK.
  - **Not:** `ci/shard.sh`'s logic, `ci/SHARDS`, workflows, tests or crates.
- **Running beside you:**
  - `4240a` (#281), `4241` (#279) and `1210` (#280), each adding tests;
  - `4406` and `1211`.

  A row they add is theirs. Merge `main` before READY, and re-plan on the merged tree.
- One PR, left unmerged. Write `~/.cache/aet-1912/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A row raised from one outlier** that will now over-plan a shard every run.
- **A colocated block weighed from a shared-process run** while it waited on a lock.

## 7. Flag-and-stop conditions

- **No weighting puts every shard under 20 minutes at 12 shards.** Stop with the table. The architect rules a split or a shard count.
