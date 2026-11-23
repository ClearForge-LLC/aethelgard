# AET-WO-1916 — FEEDBACK: CI's fourth refit, STOPPED at §7 (the rule cannot be met by rows, splits or shares)

**Status: STOPPED at §7's first condition, before the build.**
- Nothing outside `spike/1916/` and this file changes. No row, test, `ci/` file, crate or workflow is touched, so no `fab/` byte
  can move.
- One PR, as the record, left unmerged.

**Base:** `main` `aaad1555`. **Branch:** `wo/aet-1916-ci-refit`.
**Predictions:** `spike/1916/predictions.md`, committed at `33405d92` before the two newest `main` runs were read.
**The measurement:** `spike/1916/measurement.md`.

## Answer first

1. **The REFIT RULE fired again on `main`, on the two runs this work order waited for.**
   - The spreads were 7.1 min (45117816997, `c7a62975`) and 6.7 min (45118544871, `aaad1555`).
   - The slowest jobs were 18.3 and 18.5 min.
2. **The spread is runner noise, not imbalance, so a refit cannot remove it.**
   - The plan is balanced to the second, at 895–896 s per shard, and every shard runs under it.
   - Over four runs of one plan, the shards' steps do not correlate: the mean pairwise r is +0.02.
   - Between two runs, 72–89 % of a unit's change in log time is its shard's common factor. That factor is its runner's speed,
     with an sd of 0.15–0.16. The residual per unit is an sd of 0.08–0.14.
   - Setup (checkout, image pull, artifact download, unpack) adds 107–409 s per job, outside the tests.
   - A perfect re-weighting buys nothing measurable: the persistent part of the shards' differences estimates at zero
     (`measurement.md`, the weights leg).
3. **The odds, from a model fitted to seven held runs** (`spike/1916/odds.py`). At 12 shards and today's mean step of 744 s,
   the mean of the four runs since `1607`:
   - P(one run under 6 min) is 0.24–0.30;
   - P(two in a row), this work order's acceptance, is 0.06–0.09;
   - after the shares (3) and (4), at about 703 s, P(two in a row) is 0.09–0.14;
   - at 16 shards it is 0.19–0.25, and that is optimistic, since the blocks above the mean step are ignored.
   - **Observed:** 5 of the 7 held runs were over 6 min (2 under: 3.8 and 3.7), and the two newest were over in a row.
4. **Of the four predictions, three hold and one misses as worded.**
   - P2 said a shard's units move within 0.15 of each other in at least 8 of 10 shards; it was 7 of 11 on each run.
   - The claim the stop rests on is measured directly instead: the 72–89 % share in (2).
5. **Two of the build's own steps need rulings anyway** (D2): `1909` B's run no longer has one scenario, and LP1 has no guard.

## The plan: decisions for the architect

**D1, the spread clause (lean: (a)).** The rule's 6 minutes is an absolute line. The noise it reads scales with the mean step,
so on noise alone the rule fires (two in a row over 6 min) with P 0.49–0.57.
- **(a) Fire on what a refit changes: a shard persistently over the others.**
  - Candidate: over three runs of one plan, a shard's mean of log(step / that run's median step) passes log 1.25.
  - On noise alone it fires with P 0.06–0.09. It catches a shard 30 % heavy with P 0.61, and one 40 % heavy with P 0.84–0.85
    (`spike/1916/persist.py`).
  - On `main`'s last three runs, the shards read ×0.86 to ×1.12, so it does not fire. That fits a plan that is balanced.
  - Keep the 20-minute clause.
- **(b) Keep the clause, raise the line.** At 8 min it fires on noise with P 0.07–0.11, and at 9 min with P 0.02–0.03. It then
  misses any real imbalance smaller than the line.
- **(c) Keep it as written.** It fires about every other pair of runs, and each refit buys nothing.

The REFIT RULE's text is in `ci/shard.sh`, outside this fence's listed files. Rewriting it is its own small work order.

**D2, build the shares anyway, for the 20-minute clause (lean: yes; (3) with (i) and (ii), and (4)).**
- Together they cut about 490 CI s per run, 41 s per shard, from 744 s to about 703 s.
- In the model, P(slowest job > 20 min) falls from 0.33–0.38 to 0.17–0.21.
  - The model reads that tail high: 0 of the 4 runs since `1607` passed 20 min.
  - Even so, 19.9 min was seen on `ae6627cf`.
  - It is a modest gain; the spread odds barely move.
- **(3) `1909` B** saves about 460 s. Its two tests must then be colocated, as one block of 510–600 s. It needs two rulings the
  work order's text does not cover:
  - **(i) The scenario set.**
    - `--fab-out` needs the seeded set (`3303` D1, which came after `1909`). But `verify_binds…` asserts a one-scenario run:
      the exact gate list with no `attribute` gate, `runs.len() == 1`, and `pullup-2k2`.
    - A shared run means rewriting those assertions for the seeded run:
      - the full seeded gate list, exact and in order;
      - the five campaign runs, in `SEEDED`'s order;
      - `pullup-2k2`'s run id and state hash, against `campaign` run directly.
    - Lean: rewrite them at that strength. It changes what the test pins, so it is the architect's call.
  - **(ii) LP1** (`1909` FEEDBACK): `--against` alone reports `same` when no fab flag is given.
    - After the share, no test runs `--against` without a fab flag, so the mutant survives, as `1909`'s adversarial pass found.
    - **No guard exists today** (checked):
      - `verify.rs`'s other `identical` site (`:2204`) is `against_fab`'s.
      - `2308`'s unit test `the_exit_code_is_the_verdicts_whatever_the_fab_mode` builds its report with `against: None`. It
        guards LP2, which closes `1909`'s LP2 trade.
      - `generate_tier1`'s `--against` tests assert only `differs`.
    - **A tool-less tier-1 guard cannot see it.**
      - A missing tool makes `gate` return a `Refusal` (`verify.rs:466`), which carries no `against` key.
      - A guard needs a run that gets past `--against` and then fails a gate with a verdict: for example, a stub `kicad-cli`
        whose `erc` reports a finding (`cli_backlog_tier1`'s stub pattern). **Not yet checked:** that this path prints
        `against`.
    - Lean: such a guard, in a tier-1 file the fence names. Otherwise accept LP1 as a trade.
- **(4) `1607` F-L** saves about 25 s.
  - Only the `emit` and `drc`/support tests can share one `emit` per specification.
  - The `prove` test (44.6–64.7 s in shard) runs `generate`, not `emit`, so F-L's "a third" does not hold for it.

**D3, the split (2) (lean: do not build).**
- Taking `four_layer_fab_tier2` out of its block, as `against_fab_tier2` was, costs one more `verify`: about 230 CI s per run.
- It buys only the per-unit residual noise (sd 0.08–0.14).
- `verify_2k2_tier2` has no split that keeps its two-run comparisons, since each needs both of its runs in one process.
- The `board_tier2` splits are in `2310`'s file.

**D4, the shard count (lean: no change).**
- 16 shards gives P 0.19–0.25 of two in a row under 6 min, for 4 more jobs a run.
- That figure is optimistic, because the 721 and 741 s blocks sit above a 558 s mean.
- Under D1 (a), a count change would answer a measured imbalance, not noise.

**D5, backlog: setup's share.**
- Setup runs 107–409 s per job (sd 57 s), and its largest step is the artifact download.
- This is workflow work, outside this fence.

**D6, the stale rows (lean: a weights-only pass after D1).**
- `measurement.md` lists the Furnace-estimate rows marked "replace at the next refit" against their in-shard samples.
  - `with_tools`' 345.324 row runs 174–244 s.
  - `cli_backlog_tier1`'s 92 / 47 / 45 rows run 45–65 / 20–34 / 25–30 s.
- They are ready for a weights-only ruling. By the bound in (2), none of them moves the spread.

## What was done

- **Held runs:**
  - five `main` runs: 45105438499, 45111449629, 45111922226, 45117816997 and 45118544871;
  - PR #342's two runs, 45112159401 and 45114924858, the only earlier runs of `1607`'s tests.
  - Fetched read-only with `spike/1912/fetch.sh`, stamped with `spike/1914/held.sh`, and timed with `ci/shard.sh times` (P3).
  - Every result of every run was timed: 3,029 or 3,041 per run, equal to its passes. The kit is in `~/.cache/aet-1916`.
- **Scripts, in `spike/1916/`:**
  - `stamp.sh`, the glue from fetch to P3;
  - `measure.py`, with `shards`, `blocks`, `tests`, and `check` (which tests the predictions);
  - `odds.py`, the Monte Carlo of the spread;
  - `persist.py`, D1 (a)'s clause.
- **Gates, as they apply to a branch of spike scripts and documents:**
  - `cargo fmt --all -- --check` passes.
  - `clippy` and `cargo test` were not run locally: no Rust file changed. `ci/route.sh classify` reads this diff as `code`
    (`spike/` is not on the docs-only list), so this PR's CI runs every gate and every shard. Those are two more measured runs.
  - No `ci/docs-tests.tsv` test reads `spike/` or `docs/work-orders/`.
  - `spike/1916/` holds no `__pycache__`.
- **Adversarial pass, targeted.** Its two items, a split that loses a setup step and a share that hides a failure, concern a
  split and a share, and neither was built. D2 (i) and (ii) are what the pass found on paper.
  - Against the measurement itself:
    - The model draws setup and step independently, but they are anti-correlated (r −0.11 to −0.52). It still fits the spread
      (2 of 7 under 6 min, against its 0.24–0.30), but reads the 20-minute tail high (0 of 4 over, against 0.33–0.38).
    - The residual share is corrected for its degrees of freedom: 72–89 %, not the 81–92 % first computed.
    - The shares' effect is taken from today's 744 s mean step, not from 700 s.
    - P2's miss is reported as a miss, not reworded.
- **Reduction pass.** Nothing was built, so there is nothing to remove from a build. In the spike, `measure.py`'s `tests` view
  is kept. It is the evidence that `board_tier2`'s and `twin_tier2`'s per-test P3 times are lock queue time (`1913`'s lesson),
  so they are not rows to raise.

## Findings for the architect

- **F1** (D1): the REFIT RULE's spread clause fires on runner noise about every other pair of runs, and no refit can answer it.
- **F2** (D2 (i)): `1909` B's premise, one scenario for both tests, was overtaken by `3303` D1's seeded set.
- **F3** (D2 (ii)): LP1 has no guard; only `verify_binds…`, running `--against` without a fab flag, catches it. LP2 is now
  guarded, by `2308`'s unit test.
- **F4** (D2): `1607` F-L's "one emit per specification" does not reach the `prove` test, which runs `generate`.
- **F5:** every shard's step runs at 0.35–1.08 of its plan, because the rows are worst-case (REFIT RULE 5). A planned 896 s is
  not a forecast of the job. Read a shard against its run's median, not against its plan.
