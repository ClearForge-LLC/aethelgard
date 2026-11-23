# AET-WO-1916 — FEEDBACK: CI's fourth refit, built to the ruling of 2026-11-23 06:35Z (after the §7 STOP)

**Status: READY FOR REVIEW at this PR's last push.** One PR, left unmerged.
- The measurement STOPPED the work order at §7 (`bc0f63e9`; its record is kept below).
- The architect ruled D1–D6 on the bus at 06:35Z, and this PR builds that ruling.
- One deviation, D2 (ii), was flagged on the bus before it was built (below). No ruling on it had reached the bus at READY. If it is overruled, drop commit `4dc6fa7b` (it touches `with_tools.rs` only), and LP1 becomes an accepted trade.

**Base:** `main` `aaad1555`. `main` was merged three times: `057e7d8b` (1915), `7fbe5d11` (4300i, with docs) and `f42a0c88` (docs).
**Branch:** `wo/aet-1916-ci-refit`.
**Predictions:** `spike/1916/predictions.md` (`33405d92`). **The measurement:** `spike/1916/measurement.md`.
**Fence:**
- `ci/` (`shard.sh`'s REFIT RULE and `LOCKED`, `test-weights.tsv`, `colocate.tsv`);
- `verify_2k2_tier2.rs`, `cli_backlog_tier1.rs` and `with_tools.rs` (D2 (ii), flagged);
- `spike/1916/`;
- this file.

Not touched: any `src/`, any workflow, `ci/SHARDS`, `fab/`, and every `2310` file (`campaign_tier1/2`, `twin_tier2`,
`board_tier2`, `fab_tier1`, `verify.rs`'s tests), whose rows alone moved, as ruled.

## Answer first

1. **D1, the REFIT RULE's spread clause, is rewritten** in `ci/shard.sh`, the one place that states it.
   - The new clause: over three runs of one plan, a shard's mean of ln(step / the run's median step) passes ln 1.25.
   - Its model figures sit beside it. On noise alone it fires with P 0.06–0.09. It catches a shard 20 % heavy with P 0.30–0.33,
     30 % heavy with P 0.61, and 40 % heavy with P 0.84–0.85.
   - The 20-minute clause is kept.
   - `spike/1916/measure.py clause` applies it to held runs. On `main`'s last three runs of one plan it reads ×1.19 at most
     (shard 8), so it does not fire.
   - `docs/roadmap.md:857` paraphrases the old clause. That file is protected, so the row is here (F6).
2. **D2 (i), `1909` B, is built.**
   - `verify_2k2_tier2`'s two tests read one shared pair of runs, made once: `verify` of the reference with the seeded set,
     `--against` the committed tree and `--fab-out` at two paths. That is 4 verifies → 2.
   - `verify_binds…` asserts the seeded run whole, at equal or greater strength:
     - every gate, in order (now with the two `attribute` gates);
     - which faults are attributed, in order;
     - each `attribute` gate's absent uid boundary;
     - all five campaign runs, in `SEEDED`'s order, each with its verdict, `expect` and `matched`;
     - every run's id and state hash against `campaign` run directly on the same ELF. The original checked one run.
   - `verify_fab_out…` keeps every assertion. Its manifest is checked against its own run's line, which carries `against`
     (B as ruled). Its BOM is compared with the `--against` tree's, which `emit` wrote, instead of a second `emit`.
   - On Furnace the binary went from 571 s (1909's two unshared tests) to 280 s. On CI the block is 566.42 and 623.41 s on one shard,
     against 537 + 458 s on two.
3. **D2 (ii), the LP1 guard: built otherwise than ruled, flagged first.**
   - The ruled path, a stub `kicad-cli` on a tool-less tier-1 run, cannot see LP1. `verify` prints the report that carries
     `against` only from `conclude`, after every gate has run. Every halt after `--against` is a `Refusal` with no report:
     `gate`'s 2–5 arm (`verify.rs:556`), `mismatch` (`:481`) and `stopped`.
   - Built instead: `with_tools`' `a_board_not_drawn_runs_its_gates_and_is_never_verified` now also runs `--against` the tree
     `generate` writes, with no fab flag, and asserts `against: "identical"` and no `fab` key.
     - It reaches `conclude` (exit 7). Its other assertions stand, because `--against` changes no gate input (`1909`'s
       independence argument).
     - The cost is one `generate`.
   - The LP1 mutant ML1 is killed there.
4. **D2, `1607` F-L, is built.**
   - `cli_backlog_tier1`'s `check support`, `emit` and `drc`/support tests share one `emit` per specification (`Shared`,
     three cells): 7 emits → 3.
   - The binary is one colocated block of 52.75 and 63.46 s on CI (two runs).
5. **Every share and rewritten assertion has a mutant, and every mutant is killed at its own assertion** (below): 11 of 11 (MB1–MB3, MB4b, MB5, MB6, ML1, MF1–MF3), each failing its named test with that assertion's message; MB4 was caught by `verify` itself first and was replaced by MB4b.
6. **D6: the stale Furnace-estimate rows are re-weighed from CI's own timings.**
   - The six still-estimated rows come from a throwaway alone pass (REFIT RULE 1, run 45126254602, reverted at `5d257694`),
     taking the worse of two runners. `with_tools`' CPU-heavy test takes its worst in-shard time (rule 5).
   - `campaign_tier2`'s block (it carried `1211c`'s estimate) takes its worst `finished in` over nine runs.
   - The two new blocks take the worse `finished in` of their two CI runs.
7. **D3: no `four_layer` split. D4: no shard-count change** (`ci/SHARDS` stays 12). **D5:** setup's share is the architect's
   backlog row.
8. **Gates and CI:** green.
   - On the host, at the merged tree: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
     and `cargo test --workspace` (2,850 passed, 0 failed).
   - In the pinned image: `clippy` with `with-tools`; `verify_2k2_tier2` 2/2; `with_tools` 33/33.
   - On CI: 45134177463 (`12699d7f`) green, 17/17 jobs, tier 2 205/205, slowest job 18.4 min. The READY head's run is in the
     bus note.

## The build

| ruling | what changed | where |
|---|---|---|
| D1 | the REFIT RULE's spread clause becomes the persistent-shard clause, with its model figures; the 20-minute clause kept | `ci/shard.sh:47-53` |
| D2 (i) | one shared pair of seeded `verify --against --fab-out` runs; `verify_binds…` asserts the seeded run whole; `ONE_AT_A_TIME` and the `Flag` enum removed; the binary a colocated block, out of `LOCKED` (colocated binaries are exempt) and out of REFIT RULE 5's lock list | `verify_2k2_tier2.rs`, `ci/colocate.tsv`, `ci/shard.sh` |
| D2 (ii) | LP1 guard: `--against generate's tree` on the board-not-drawn `verify` | `with_tools.rs` |
| D2, F-L | one `emit` per specification, shared; the binary a colocated block | `cli_backlog_tier1.rs`, `ci/colocate.tsv` |
| D6 | rows re-weighed from CI, below | `ci/test-weights.tsv`, `ci/colocate.tsv` |

**D6's rows**, all from run 45126254602 (head `9c7c6cb8`, the throwaway, reverted) unless named:

| row | was | now | from |
|---|---|---|---|
| `board_tier2` `d1_s_pin_device_generates_builds_and_proves` | 269.332 (estimate) | 148.30 | alone, 148.30 / 127.08 |
| `board_tier2` `build_names_the_rustc_cargo_ran` | 1.12 (estimate) | 0.37 | alone, 0.37 / 0.36 (lock-serialised binary, so it keeps a row) |
| `fab_tier1` `a_drawn_twin_stand_in_is_refused_by_each_fab_flag_by_name` | 2.6 (estimate) | 2.71 | alone, 2.71 / 2.13 |
| `main.rs` `verify::tests::the_exit_code_is_the_verdicts_whatever_the_fab_mode` | 1.4 (estimate) | 1.40 | alone, 1.40 / 1.08 |
| `bounds.rs` `the_writer_refuses_an_event_that_would_take_the_trace_past_256_mib` | 16.2 (estimate) | 7.95 | alone, 7.95 / 7.93 |
| `with_tools` `a_board_not_drawn_runs_its_gates_and_is_never_verified` | 345.324 (estimate) | 244.010 | worst in-shard (rule 5) over six runs, 173.7–244.0; alone 198.14 / 215.83 |
| block `campaign_tier2` | 116 (83 + `1211c`'s 32.228 estimate) | 98 | worst `finished in` over nine runs, 56.3–97.0 |
| block `verify_2k2_tier2` | (new; predicted 572) | 624 | the worse of its two runs: 566.42 (45126254602), 623.41 (45134177463) |
| block `cli_backlog_tier1` | (new; predicted 70) | 64 | the worse of its two runs: 52.75, 63.46 |
| `cli_backlog_tier1`'s 7 rows, `verify_2k2_tier2`'s 2 | removed | — | their binaries are blocks now |

- The throwaway's runners carried five CPU models (AMD EPYC 7763, 9V45 and 9V74; Intel Xeon 6975P-C and Platinum 8573C). That
  corroborates D1's finding that the noise is the runner's.

## Mutants (`spike/1916/mutants.py`)

- Each mutant is one exact edit to an archive copy of `b02332d9`, run against the test it targets. Tier 2 runs in the pinned
  image (16 GiB, swap 0, read back), and tier 1 on the host.
- A mutant counts as killed only when its named test fails with that assertion's message. The driver first judged by exit
  status, but `t2.sh` exits 0 whatever its tests do. That rule is fixed, and `--judge` re-reads the logs.
- Every binary passes unmutated (the six PRISTINE runs).

| id | the defect | the guard it tests | result |
|---|---|---|---|
| MB1 | `verify.rs` `against` "identical" → "same" (`1909`'s Y) | `verify_binds…` "--against finds the committed tree identical" | killed |
| MB2 | `eda/fab.rs` FAB-NOTES "plated via, drilled" → "plated via drilled" (`1909`'s X) | `verify_fab_out…` "FAB-NOTES.md states plated via, drilled 0.3 mm" | killed |
| MB3 | the campaign's runs reported in reverse | `verify_binds…` "the campaign ran every seeded scenario, in order, each as it declares" | killed |
| MB4 | each run's state hash recorded as its run id | (aimed at the direct-`campaign` check) | **caught for another reason**: `verify`'s own `attribute` binding refuses the run (BindingMismatch) before the test compares; replaced by MB4b |
| MB4b | `pullup-3k3`'s reported state hash moved (no `attribute` gate re-reads it) | `verify_binds…` "verify's state hash is the one campaign gives the ELF directly" | killed (`verify_2k2_tier2.rs:361`) |
| MB5 | `attribute` gate reports `uid_boundary: false` | `verify_binds…` "attribute reports no uid boundary" | killed |
| MB6 | an `attribute` gate pushed first | `verify_binds…` "every gate runs, in order, and passes" | killed |
| ML1 | LP1: `against` "same" when no fab flag is given | `with_tools` "--against with no fab flag finds generate's tree identical" | killed |
| MF1 | `emit`'s verdict loses the banner | `the_emit_verdict_carries_the_banner…` | killed |
| MF2 | `check support`'s verdict loses the banner | `the_drc_and_support_verdicts_carry_the_banner…` | killed |
| MF3 | `check support` unpinned (`1607`'s M1) | `check_support_runs_only_the_pinned_kicad` | killed (`cli_backlog_tier1.rs:214`; re-anchored: its first text matched both pins in `main.rs`) |

## Adversarial pass (targeted, §5)

- **A split that loses a setup step.** No split was built (D3).
  - The share's restructure was checked step by step. Every step the original runs made is still made: the committed board
    copied, `emit` into the tree, `build`, and `campaign`.
  - One step changed shape: `verify_fab_out…`'s second `emit`, used for the BOM, is replaced by the tree's `emit`. Same
    command, same specification; `--against` found that tree identical.
  - The plain `--fab-out` pass path (no `--against`) is still exercised by `against_fab_tier2`'s and `four_layer_fab_tier2`'s
    shared packages.
- **A shared run that hides one test's failure behind the other's.**
  - Each test checks the shared runs' exit codes itself; the maker records them but asserts none.
  - Poison (A2): the NRND `emit` was made to fail in `cli_backlog_tier1`. Its three readers all failed at once, each naming
    its own property and "the shared run `emit of the NRND reference` failed once". No hang, and the other six tests passed
    (`~/.cache/aet-1916/adv/a2.log`). `Shared` is `1909`'s, whose poison was measured for the verify cells.
  - Read-only (A1): after a full run of the binary, each of the three shared `emit` directories was byte-identical to a fresh
    `emit` of its specification, so no reader writes into what the others read.
  - Mutants: every reader still fails on its own defect (MB1–MB6, MF1–MF3), and only that reader is named.
- **Found and fixed:** the mutant driver's verdict rule (above), and MB4's wrong reason (above).

## Reduction pass

Removed:
- `verify_2k2_tier2`'s `ONE_AT_A_TIME`, the `Flag` enum, the per-test `two_runs`, one `emit` per run, and two verifies;
- four `emit` calls in `cli_backlog_tier1`;
- `verify_2k2_tier2` from `LOCKED`;
- nine rows.

The REFIT RULE's text grew by five lines, because the ruling asks for the model figures beside the clause. Nothing labelled
`belt-and-suspenders:` was touched. `spike/1916/` is the work order's record.

## Gates and CI

- **Host** (merged tree `bf02cbaa`): `cargo fmt --all -- --check` passes; `cargo clippy --workspace --all-targets --
  -D warnings` is clean; `cargo test --workspace` passes 2,850, fails 0 (1,109 s). `bash ci/test-shard.sh`: 137 PASS.
- **Image** (`capped-run`, 16 GiB, swap 0, read back): `cargo clippy --workspace --all-targets --features
  aethelgard-cli/with-tools -- -D warnings` is clean. On the merged tree, `verify_2k2_tier2` passes 2 of 2 (387 s) and
  `with_tools` 33 of 33.
- **CI:**

  | run | head | result | jobs | tier 2 | slowest job | spread | note |
  |---|---|---|---|---|---|---|---|
  | 45126254602 | `9c7c6cb8` (throwaway) | green | all | — | 19.4 min | 6.5 min | the alone pass on six shards; not a spread sample |
  | 45134177463 | `12699d7f` (the build) | green | 17/17 | 205/205 | 18.4 min | 8.5 min | blocks measured: `verify_2k2_tier2` 623.41, `cli_backlog_tier1` 63.46 |

  - On `12699d7f` the shards are planned at 830 s, down from 850–896 before the shares. Shard 2 ran 365 s against 830:
    `supply_tier1`'s fast mode on a fast runner. That is the noise D1 now ignores.
  - The 20-minute clause held on both runs.
  - The new clause reads three runs of one plan. This PR's last rows (the two blocks, at the worse of their two runs) change the plan once more, so its count
    starts on `main` after the merge.
- `TIER2_MIN` is unchanged (205): no test was added or removed.

## Findings for the architect

- **F1** (D2 (ii)): the ruled LP1 path cannot exist, as measured from source (above). The guard is built in `with_tools.rs`,
  outside the named fence, and was flagged first. No ruling on it had reached the bus at READY. If it is overruled, drop commit `4dc6fa7b` (it touches `with_tools.rs` only), and LP1 becomes an accepted trade.
- **F2:** each share's directory outlives its process: statics are never dropped (`1909`'s note, now also for three `emit`s
  of about 1 MB each, in `TMPDIR`).
- **F3** (MB4): a state hash that disagrees with the archive is refused by `verify` itself, before any test can see it. That
  is a defence, not a gap, and it is why MB4b targets a run no `attribute` gate re-reads.
- **F4:** shard 8 reads ×1.19 over `main`'s last three runs of the old plan, near the new clause's ×1.25. The old plan's
  shard 8 held `against_fab_tier2` and `escape_tier1`. This PR's rows change the plan, so the count restarts. Watch it.
- **F5:** two CI runs are thin ground for a new block (`verify_2k2_tier2` 566.42 and 623.41 s; `cli_backlog_tier1` 52.75 and
  63.46 s). The next refit should re-weigh both, by the new clause or by rule 3.
- **F6** (protected file): `docs/roadmap.md:857` still states the old "spread past 6 minutes twice" clause.

## Record: the measurement and the STOP (`bc0f63e9`)

### Answer first

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

### The plan: decisions for the architect

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

### What was done

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

### Findings for the architect

- **F1** (D1): the REFIT RULE's spread clause fires on runner noise about every other pair of runs, and no refit can answer it.
- **F2** (D2 (i)): `1909` B's premise, one scenario for both tests, was overtaken by `3303` D1's seeded set.
- **F3** (D2 (ii)): LP1 has no guard; only `verify_binds…`, running `--against` without a fab flag, catches it. LP2 is now
  guarded, by `2308`'s unit test.
- **F4** (D2): `1607` F-L's "one emit per specification" does not reach the `prove` test, which runs `generate`.
- **F5:** every shard's step runs at 0.35–1.08 of its plan, because the rows are worst-case (REFIT RULE 5). A planned 896 s is
  not a forecast of the job. Read a shard against its run's median, not against its plan.
