# AET-WO-1916: the measurement (§1.1), against `predictions.md`

Measured 2026-11-23, finishing at 06:05 UTC (by `date -u`). `predictions.md` was committed at `33405d92` before the two newest
`main` runs were read.

## Answer first

- **The REFIT RULE fired again on `main`, on the two runs this work order waited for.** 45117816997 (`c7a62975`) had a
  spread of 7.1 min and 45118544871 (`aaad1555`) 6.7 min. Their slowest jobs were 18.3 and 18.5 min, under the 20-minute line.
- **What drives the spread is the runner, not the plan.**
  - The plan is balanced to the second: every shard is planned at 895–896 s.
  - The twelve shards' steps do not correlate from run to run: the mean pairwise r over four runs of one plan is +0.02.
  - Between two runs, 72–89 % of a unit's change in log time is its shard's common factor, which is its runner's speed.
    What is left per unit is an sd of 0.08–0.14, against the shards' 0.15–0.16.
- **So no row, split or share can hold the spread under 6 min on two runs in a row with any reliability.**
  - The model is fitted to all seven held runs (s 0.154–0.163). "Today" is the mean step of the four runs since `1607`'s
    tests: 744 s.
  - Today, P(one run under 6 min) is 0.24–0.30, and P(two in a row), the acceptance, is 0.06–0.09.
  - After the shares, at about 703 s, P(two in a row) is 0.09–0.14.
  - Observed: 5 of the 7 held runs were over 6 min, and the two newest were over in a row. The two under were 3.8 and 3.7.
- **This is the §7 stop.** The plan and the decisions it needs are in `docs/work-orders/AET-WO-1916.FEEDBACK.md`.

## The runs

The logs were fetched with `spike/1912/fetch.sh`, stamped by `stamp.sh` (which calls `spike/1914/held.sh stamp`), and timed by
`ci/shard.sh times`. That is P3's own reconstruction: 3,029 or 3,041 results per run, equal to each shard's passes. The tables
come from `measure.py shards` and `measure.py blocks`.

| run | head | plan (s) | job max (min) | job min (min) | spread (min) | mean step (s) |
|---|---|---|---|---|---|---|
| 45105438499 | `15798f91` | 848 | 18.9 | 9.2 | **9.8** | 698 |
| 45111449629 | `29a34afa` | 851 | 16.4 | 12.6 | 3.8 | 680 |
| 45111922226 | `57b62195` | 851 | 18.1 | 11.9 | **6.2** | 701 |
| 45112159401 | `ae6627cf` (PR #342) | 896 | 19.9 | 12.9 | **7.0** | 791 |
| 45114924858 | `664ca6d2` (PR #342) | 896 | 17.2 | 13.5 | 3.7 | 717 |
| 45117816997 | `c7a62975` | 896 | 18.3 | 11.2 | **7.1** | 722 |
| 45118544871 | `aaad1555` | 896 | 18.5 | 11.8 | **6.7** | 746 |

- **Every shard's step is under its plan.** Steps ran 365–951 s against plans of 848–896 s, at 0.35–1.08 of the plan. The rows
  are worst-case in-shard times, by REFIT RULE 5.
- **Setup** (checkout, image pull, artifact download, unpack): over 84 jobs, 107–409 s, with a mean of 183 s and an sd of 57 s.
  The rule reads job time, so setup counts.
- **Setup and step are mildly anti-correlated within a run** (r −0.11 to −0.52). The model draws them independently, so it
  leans pessimistic. For the spread it still fits: 2 of 7 runs were under 6 min (0.29), against its 0.24–0.30 at 744 s. For the
  20-minute line it reads high: 0 of the 4 runs since `1607` passed 20 min (slowest 19.9), against its 0.33–0.38 for one run.

## The predictions, tested

| | prediction | measured | |
|---|---|---|---|
| P1 | both new runs plan exactly as 45114924858 | equal on every shard, both runs | **holds** |
| P2 | a shard's units within 0.15 log of each other in ≥ 8 of 10 shards; shards differing by ≥ 0.3 | **7 of 11** shards within 0.15, on each run; shard factors −0.25 to +0.45 | **misses as worded** |
| P3 | mean pairwise r of shard steps over the four runs of one plan < 0.4 | +0.02 (−0.22 to +0.29) | **holds** |
| P4 | at least one new run over 6 min (P 0.84–0.92) | both over: 7.1 and 6.7 | **holds** |

**P2, the miss.**
- Of the four shards outside 0.15, three lie within 0.16–0.24. The fourth is shard 2 on 45118544871, where `supply_tier1` moved
  −0.35 while `twin_tier2` moved +0.41. `supply_tier1` is the one unit whose modes (`1912` D1) are not only its runner's.
- **The conclusion rests on a claim measured directly.** Over five pairs of runs (29–33 units in 11 shards each), take each
  shard's mean log ratio as its factor.
  - The factor explains 72–89 % of the variance of the units' log ratios. The residual is taken on units less shards degrees of
    freedom.
  - The total sd is 0.22–0.27, of which 0.08–0.14 is left per unit.
- A split acts only on that residual.

## The weights leg, bounded

- Take each shard's step over its run's median, as the mean over three runs of the one plan:
  - 45114924858, 45117816997, 45118544871: 0.95 1.06 1.03 0.89 0.92 1.04 1.10 1.07 **1.12** 1.00 1.01 **0.86**
  - 45112159401, 45114924858, 45117816997: 0.94 0.91 **1.09** 0.93 1.02 0.98 1.06 1.00 1.06 0.88 0.96 **0.82**
- That is what noise alone gives. A three-run mean has an sd of about 0.155 / √3 = 0.09 in log, so twelve of them should range
  over about 0.29. They range over 0.26 and 0.29.
- The shard that is high is not the same in the two trios (8, then 2), and P3's r is +0.02.
- The within-run sd (median 0.154) does not exceed the pair noise (0.163), so the persistent part estimates at zero. **A perfect
  re-weighting buys nothing measurable.**
- **The Furnace-estimate rows** marked "replace at the next refit", against their in-shard samples on the four runs since `1607`
  (P3 seconds):

  | row | weight | in-shard samples |
  |---|---|---|
  | `board_tier2` `the_bench_board_passes_every_gate_at_two_and_four_layers` | 11.83 | 9.6 12.2 11.1 12.4 |
  | `with_tools` `a_turned_keepout_is_where_kicad_draws_it` | 17.1 | 13.2 16.4 14.0 14.6 |
  | `board_tier2` `verify_names_what_judged_nothing_on_the_motor_driver` | 177.49 | 210.9 384.8 261.7 337.7 (lock queue time, `1913`'s lesson: not a row to raise from these) |
  | `fab_tier1` `a_drawn_twin_stand_in_is_refused_by_each_fab_flag_by_name` | 2.6 | 3.6 2.8 2.7 3.6 |
  | `main.rs` `verify::tests::the_exit_code_is_the_verdicts_whatever_the_fab_mode` | 1.4 | 1.4 1.8 1.4 1.8 |
  | `with_tools` `a_board_not_drawn_runs_its_gates_and_is_never_verified` | 345.324 | 173.7 244.0 198.9 205.1 |
  | `cli_backlog_tier1` `the_prove_verdict_carries_the_banner_of_its_spec` | 92.008 | 60.6 44.6 52.4 64.7 |
  | `cli_backlog_tier1` `the_drc_and_support_verdicts_carry_the_banner_of_their_spec` | 47.432 | 34.0 26.4 30.8 20.5 |
  | `cli_backlog_tier1` `the_emit_verdict_carries_the_banner_and_the_files_do_not` | 45.136 | 25.7 29.9 25.9 25.4 |
  | `cli_backlog_tier1`, its four small rows | 0.6–3.2 | each under its row |

  The `board_tier2` and `fab_tier1` rows are in `2310`'s files' binaries, but a row is not a test file. All of these are ready for
  a weights-only ruling. By the bound above, none of them moves the spread.

## The model (`odds.py`, refitted on the seven runs)

- Each job's time is its setup, plus the plan's mean step × exp(N(0, s)).
- Setups are drawn from the 84 held setups.
- s is 0.163 from the five same-plan pairs (60 shard pairs), and 0.154 as the median within-run sd.

| shards | mean step (s) | P(≤ 6 min), one run | P(two in a row ≤ 6) | P(slowest > 20 min) |
|---|---|---|---|---|
| 12 | 790 | 0.19–0.25 | 0.04–0.06 | 0.56–0.61 |
| 12 | **744 (today)** | 0.24–0.30 | 0.06–0.09 | 0.33–0.38 |
| 12 | **703 (after the shares)** | 0.30–0.37 | 0.09–0.14 | 0.17–0.21 |
| 12 | 500 | 0.69–0.74 | 0.48–0.55 | 0.00 |
| 14 | 638 | 0.33–0.39 | 0.11–0.15 | 0.04–0.06 |
| 16 | 558 | 0.43–0.50 | 0.19–0.25 | 0.00–0.01 |

- **The 14- and 16-shard rows are optimistic.** They scale the mean step and ignore the blocks that would sit above it:
  `four_layer_fab_tier2` (741) and `supply_tier1` (721), and at 16 shards `against_fab_tier2`'s and `verify_2k2_tier2`'s tests
  (446–585).
- **How often the rule fires on noise alone** (two runs in a row over X), at 12 shards and 744 s:

| X (min) | 6 | 7 | 8 | 9 | 10 |
|---|---|---|---|---|---|
| P(fires) | 0.49–0.57 | 0.22–0.30 | 0.07–0.11 | 0.02–0.03 | 0.00–0.01 |

## The candidate builds, in CI seconds

Each is read from the `ANSWER 3301` walls in the held logs. One `verify` took 154–296 s.

- **(3) `1909` B:**
  - The four `verify` runs become two, saving about 460 s per CI run.
  - The two tests become one colocated block of about 2 × V(seeded) + 10 s, which is 510–600 s.
- **(4) `1607` F-L:**
  - The `emit` and `drc`/support tests ran 20.5–34.0 s each in shard, and could share one `emit` per specification, saving
    about 25 s.
  - The `prove` test (44.6–64.7 s) runs `generate`, not `emit`, so a shared `emit` does not reach it.
- **(2), splitting `four_layer_fab_tier2`** the way `against_fab_tier2` was split costs one more `verify`, about 230 s per run.
  It buys only the per-unit residual (sd 0.08–0.14).
- **Together, (3) and (4) cut about 490 s per run, 41 s per shard: from 744 s to about 703 s.**
  - P(two in a row ≤ 6 min) moves from 0.06–0.09 to 0.09–0.14.
  - P(slowest > 20 min) moves from 0.33–0.38 to 0.17–0.21 in the model. The model reads that tail high: 0 of 4 runs were over.
  - So the shares halve the 20-minute risk in the model, and barely move the spread.
