# AET-WO-1909 — CI that earns its minutes: a reduction pass over the test suite, every guarded failure kept

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1909-ci-reduction`.
**Author:** Claude (architect) · **Builder:** strong tier, with a targeted adversarial pass (`docs/roadmap.md` §6).
**Phase:** infrastructure, after `AET-WO-2204`. **Cadence: census (§1), then STOP for the architect's ruling, then build (§2).**

**Grounds, each measured before this work order:**
- **Scotty, 2026-11-14:** eight balanced shards under about 20 minutes is acceptable, and "take at least one reduction pass at the CI build out to make sure we aren't wasting any efforts. If it all looks good and is genuinely valuable to have incorporated as solid and stable CI tests then leave it."
- **The suite's cost (`AET-WO-1908` FEEDBACK, CI seconds, each test alone):** about 5,774 s over 248 weighted tests, so about 12 minutes of tests per shard plus about 2.3 minutes of setup.
  - **`board_tier2`, 17 tests, about 3,600 s.** Six of its tests each run the whole `verify` pipeline on the reference: `verify_fab_out_writes_the_package_its_verdict_names_at_any_path` 516 s, `verify_against_fab_names_every_changed_and_extra_file` 453 s, `verify_against_fab_finds_the_package_it_wrote_identical` 416 s, `verify_binds_the_reference_and_two_runs_print_the_same_bytes` 382 s, `verify_fails_the_47k_pull_up_with_the_campaigns_findings` 218 s and `verify_fab_out_writes_nothing_when_a_gate_fails` 200 s. They serialise on `ONE_AT_A_TIME`, which is also the shards' main source of run-to-run variance.
  - **`attribute_seeded`, 12 tests, kept whole on one shard (720 s planned).** It takes 1,377 s alone and 475 to 721 s together. Its heaviest test, `p3_gate_clause_2_propose_answers_every_seeded_case_as_its_rule_says`, takes 632 s alone.
- **`TIER2_MIN` is 91** (`AET-WO-2204`), with each test's reason in `ci.yml`'s comments.

> **What this is:** finding where CI pays twice for one fact (the same expensive pipeline run per test, the same fixture rebuilt), and paying once, without losing any failure a test exists to catch.
>
> It is NOT:
> - lowering any gate, weakening any assertion, or skipping, ignoring or quarantining a test (`CLAUDE.md`);
> - a change to production code. Test harness, fixtures and `ci/` only, unless §7 is ruled;
> - nextest, a new CI provider, or caching across CI runs.

## 1. Census

Write it into FEEDBACK. **No test is changed in this phase.**

1. **Every tier-2 test and every test of 10 CI seconds or more.** For each, give:
   - its CI seconds (`ci/test-weights.tsv`);
   - **the distinct failure it guards**, one sentence, naming the finding kind, assertion or property;
   - the expensive work it does: which pipeline (`verify`, `prove`, a campaign, a boot, KiCad, Kani), on which input, and how many times.
2. **Overlap.** Group the tests that run the same expensive work on the same input. For each group:
   - say whether the tests could share one run inside one test binary (a `OnceLock` result, or one test asserting several named properties);
   - give the measured saving: one shared run against the sum.
3. **Redundancy.** List any test whose guarded failure another test already catches. Name the other test and the evidence: a mutant that both kill, or the same assertion on the same output.
   - "Looks similar" is not redundancy. A test that pins a forgery or an adversarial residual is never redundant with a test that pins the pass.
4. **`attribute_seeded`.** Say why its tests overlap in one process (shared fixtures, Renode boots, campaigns), and what a shared fixture would save. Measure `p3_gate_clause_2` inside the block and alone.
5. **Predictions.** Predict the suite's CI seconds after the proposed changes, and the eight-shard plan's maximum.

**Stop** with the table, the groups, the redundancy claims with their evidence, a proposal per group, and the decisions needed. **Write `AET-WO-1909 STOPPED: census done, ruling needed on PR #<n>`.**

## 2. Build, after the ruling

Expected, to be confirmed by the ruling:
1. **Shared expensive runs.** One run per distinct input inside a binary, with every property still asserted by name.
   - A shared run that fails must fail every test that depends on it, naming the property. It must never pass one silently.
2. **Redundant tests removed only where the ruling accepts the evidence.** For each, give the mutant that the surviving test kills.
3. **`TIER2_MIN` set to the new exact count,** with every removed or merged test listed in `ci.yml`'s comment beside the reason. The guard and the canary are otherwise unchanged.
4. **`ci/test-weights.tsv` refitted** by `ci/shard.sh`'s refit rule. `ci/SHARDS` stays 8.

## 3. Fence and invariants

- **Fence:**
  - test files and fixtures in every crate;
  - `ci/`, and `.github/workflows/ci.yml` (`TIER2_MIN` and its comment only);
  - this work order's FEEDBACK.
- **No production code.** A production change that would make a test cheaper (a cache in `verify`, say) is a §7 stop for a ruling.
- `INV-04`: a shared run must not make a test's outcome depend on test order. `INV-06`: no assertion may become optional.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 4. Acceptance

- **The census, ruled.**
- **A coverage map in FEEDBACK:** every removed or merged test, its guarded failure, and the test that now guards it, with the mutant that shows it.
- **Mutants.** For each shared run, a mutant breaking one of its properties is caught by the named test, and only by that test's assertion.
- **CI.** Three consecutive runs, every shard under 20 minutes, the spread recorded. The suite's CI seconds are stated before and after, measured, not planned.
- **The CLAUDE.md gates pass.**

## 5. Adversarial pass, targeted

1. **Lose a failure.** For each merged group, make one property fail and show that CI goes red naming it.
2. **Order.** Run a shared-run binary with `--test-threads=1` in reverse order and in random order (`-Z shuffle-seed` is unstable, so permute `--exact` names by hand). The outcomes must be identical.
3. **Poison.** Make the shared run panic once. Every dependent test must fail, none pass, and none hang.

## 7. Flag-and-stop conditions

- A saving needs a production change.
- A test's guarded failure cannot be stated in one sentence. That is a finding for the architect, not a deletion.
- Removing anything would lower `TIER2_MIN` below the count of distinct failures the census lists.
