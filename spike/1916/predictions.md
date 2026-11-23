# AET-WO-1916: predictions (§1.1), written before the two pending `main` runs are read

Written 2026-11-23, about 05:45 UTC, at base `aaad1555`. **Held so far:** five completed runs, fetched with `spike/1912/fetch.sh`,
stamped with `spike/1914/held.sh stamp`, and timed by `ci/shard.sh times` (P3, from `1914`):
- three on `main`: 45105438499 (`15798f91`), 45111449629 (`29a34afa`) and 45111922226 (`57b62195`);
- two on PR #342, the only runs yet of `1607`'s new tests: 45112159401 (`ae6627cf`) and 45114924858 (`664ca6d2`).

**Not yet read:** the two `main` runs in progress when this was written, 45117816997 (`c7a62975`, `1607`'s merge) and
45118544871 (`aaad1555`, docs only). The predictions below are tested on them in `measurement.md`.

## What the held five already show (`measure.py`, `odds.py`)

- **The plan is balanced to the second, and every shard runs under it.** Each shard is planned at 848–896 s, and each test
  step ran 365–925 s.
- **The spread is run-to-run noise, not imbalance.** Two pairs of runs share a plan: 45111449629 with 45111922226, and
  45112159401 with 45114924858.
  - The same shard's step moves by up to 305 s between the two runs of a pair. The per-shard difference has an sd of 156–179 s.
  - Within one run, the shards' steps have an sd of 82–116 s, which is no more than that noise. So no persistent imbalance is
    left for rows to remove.
- **The noise belongs to the runner, not to the test.** In a shard, every unit of 40 s or more speeds up or slows down
  together between the two runs of a pair. For example, shard 8's `against_fab_tier2` and `escape_tier1` move −0.38 and −0.41
  in log; shard 10's three units move +0.43, +0.43 and +0.29. Across shards, the factor runs from −0.45 to +0.57.
  - So splitting a test into smaller units does not average the noise out: every unit on a slow runner is slow.
  - `supply_tier1`'s two modes (`1912` D1) are this effect too: 300 s on a fast runner, 700 s on a slow one.
- **Setup is a second, separate noise.** Checkout, image pull, artifact download and unpack take 107–342 s over 60 jobs
  (mean 182 s, sd 49 s). The REFIT RULE counts it, because it reads job times.

## The model (`odds.py`)

- Each job is its setup, drawn from the 60 held setups, plus its step: the plan's mean step times exp(N(0, s)).
- s, the sd of the log speed, is bracketed two ways:
  - 0.171 from the same-plan pairs;
  - 0.154, the median within-run sd. This one is an upper bound on a run's noise, and it still reads lower.
- At 12 shards and the held mean step of about 700 s, the model gives:

  | | s 0.171 | s 0.154 |
  |---|---|---|
  | P(spread ≤ 6 min), one run | 0.28 | 0.40 |
  | P(two runs in a row ≤ 6 min), the acceptance | 0.08 | 0.16 |
  | P(the rule fires: two in a row > 6 min) | 0.53 | 0.36 |
  | P(slowest job > 20 min), one run | 0.22 | 0.13 |

- **Observed:** 2 of the 5 held runs were at or under 6 min (3.8 and 3.7). The others were 6.2, 7.0 and 9.7. That is
  consistent with the model.

## Predictions for the two pending `main` runs

- **P1, the plan.** Both print each shard's planned seconds exactly as 45114924858 did (895 or 896). The tests and rows are
  `664ca6d2`'s.
- **P2, the runner.** Take each new run against 45114924858, shard by shard, over the units of 40 s or more in a shard holding
  two or more. In at least 8 of every 10 such shards, the units' log ratios lie within 0.15 of each other. Across shards they
  differ by 0.3 or more.
- **P3, no persistent imbalance.** Over the four runs of this plan (45112159401, 45114924858 and the two new ones), the mean
  pairwise correlation of the twelve shards' steps is under 0.4.
- **P4, the spread.** Each new run's spread is a draw with P(≤ 6 min) of 0.28–0.40, at a mean step of 700–800 s. So at least
  one of the two is over 6 min, with P 0.84–0.92.

## Predictions for the build the work order names

These are in CI seconds per run, from the `ANSWER 3301` walls in the held logs. One `verify` takes 154–296 s.

- **(3), `1909` B:**
  - It saves two `verify` runs, about 460 s per CI run.
  - Its two tests become one colocated block of about 2 × V(seeded) + 10 s, which is 510–600 s.
  - The issues found, which need a ruling before it is built:
    - **The scenario set.** `--fab-out` needs the seeded set (`3303` D1), which came after `1909`. `verify_binds…` asserts a
      one-scenario run: the gate list with no `attribute` gate, `runs.len() == 1`, and `pullup-2k2`. A shared run means
      rewriting those assertions for the seeded run.
    - **LP1** (`1909` FEEDBACK). After the share, no test runs `--against` without a fab flag, and that mutant survives.
- **(4), `1607` F-L:**
  - One `emit` per specification, shared by the `emit` test and the `drc`/support test, saves about 25 s.
  - The `prove` test runs `generate`, not `emit`, so it is not shared.
  - Its three tests, weighed at 184.6 s, ran 25.7–60.6 s each in shard.
- **(2), the split.** Taking `four_layer_fab_tier2` out of its block, as `against_fab_tier2` was, costs one more `verify`
  (about 230 s). It buys no spread, because the noise is per runner (above).
- **Net:** about −490 s per run, so the mean step falls about 40 s, to about 660 s. P(≤ 6 min) per run becomes about
  0.34–0.48, and P(two in a row) about 0.12–0.23.
- **So the acceptance (under the rule on two runs) is predicted to fail more often than it passes, whatever the rows,
  splits and shares.** A shard-count change does not cure it either: the model gives 0.21–0.37 two in a row at 16 shards (0.12–0.24 at 14). The
  levers that move the odds are less work per shard and a rule whose threshold matches the runner's noise. **This is the
  §7 stop.** `measurement.md` tests P1–P4 first.
