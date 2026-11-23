# AET-WO-1916 — CI's fourth refit: the slow `verify` tier-2 tests split, the `1909` B shared runs, and the spread back under the REFIT RULE

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1916-ci-refit`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4, backlog wave 2. Written 2026-11-23. **Cadence:** a measurement, then a build. **After `1607`** (#342) and **`1914`** (#344).

**Grounds** (roadmap §7.1, `docs/work-orders/AET-WO-1912.md` and `1914` first):
1. **The REFIT RULE fired again** (`1607` F-K): the spread was 6.2 min, then 7.3 min, and the slowest shard was 19.9 min on `ae6627cf`. `1912`'s row also stands: `twin_tier2`'s rows sit high, and `four_layer_fab_tier2` and `verify_47k_tier2` sit low.
2. **`4300b` (c), `4230` §12.** Split the long `verify` tier-2 tests, which hold the slowest shards.
3. **`1909` B.** `verify_binds…` and `verify_fab_out_writes…` can share one pair of `verify --against --fab-out` runs, about 500 CI s per run. Assert the derived `verdict_sha256` field against its own run's line. The architect judged that difference acceptable at `1909`.
4. **`1607` F-L.** Three emit-heavy tier-1 tests, weighed at 185 s, could share one emit per specification.

> **What this is:** the spread back under the REFIT RULE, by measured weights and by the splits and shares (2)–(4). Every test keeps what it asserts, and every split or share has a mutant showing that it still catches the same defect.
>
> It is NOT:
> - dropping or weakening an assertion;
> - a shard-count change, unless the measurement shows that splits and weights cannot meet the rule (then stop);
> - any crate change outside the tests.

## 1. Build

1. **Measure first,** with predictions in `spike/1916/`. Use `1914`'s P3 timings from the last three `main` runs to get each test's in-shard seconds, the spread, and the plan.
2. **Apply (2)–(4),** each with its mutant.
3. **Weights** from the measurement. Re-plan, and show the spread on this PR's own CI.

## 2. Acceptance

- **The spread is under the REFIT RULE** on this PR's CI, and on a second run.
- **Every assertion stands,** shown by the mutants.
- **The CLAUDE.md gates pass, and CI is green.** `TIER2_MIN` is unchanged, or raised by a split.

## 3. Fences

- **Fence:** `ci/` (`test-weights.tsv`, `SHARDS` only if §1 shows the stop); the CLI's tier-2 and tier-1 test files that (2)–(4) name; `spike/1916/` and FEEDBACK.
- **Not:** any crate's `src/`; workflows (except `TIER2_MIN`, one line, if a split raises the count).
- **Running beside you:**
  - `2310` edits `campaign_tier1/2`, `twin_tier2`, `board_tier2`, `fab_tier1` and `verify.rs`'s tests: do not touch those files;
  - `4300i`;
  - `1915`.

  If a split you need is in a file `2310` is editing, take the weights only and name the split as a row. Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-1916/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A split that loses a setup step** the original asserted through.
- **A shared run that hides one test's failure** behind the other's.

## 7. Flag-and-stop conditions

- **The rule cannot be met without a shard-count change.** Stop with the plan.
- **A `fab/` byte moves.** Stop.
