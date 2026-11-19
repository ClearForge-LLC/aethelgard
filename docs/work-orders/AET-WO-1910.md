# AET-WO-1910 — CI that earns its minutes, second pass: eight shards, every guarded failure kept, the cost of honesty accepted

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` after `AET-WO-1314` merges, named in the kickoff.
**Branch:** `wo/aet-1910-ci-reduction`.
**Author:** Claude (architect) · **Builder:** strong tier (the `1314` builder, reused), with a targeted adversarial pass.
**Phase:** infrastructure. Written 2026-11-19. **Cadence: census (§1), then STOP for the ruling, then build (§2).** `AET-WO-1909` is the precedent and the method: read it first.

**Grounds:**
- **Scotty, 2026-11-18 20:45 UTC:**
  - `ci/SHARDS` stays 8;
  - "make sure there isn't any redundancy or inefficiency. We will accept the outcome";
  - Aethelgard is to be "brutally honest and accurate", not fast. A slow, honest emit is accepted. CI's time is a cost to cut only where it buys nothing.
- **The suite has grown past its plan since `1909`:**
  - main `16404fc7`: tools shards 30.5–40.5 min, the plan's mean 1,699 s;
  - #229 (main's code): 21.5–39.4 min;
  - `1314`'s head `739b4a24`: 32.7–49.3 min, the plan's mean 2,141 s.
  - Against these, `ci/shard.sh`'s REFIT RULE targets 20 minutes per shard.
- **Variance is about 25 % run to run on one plan.** `1314`'s shard 4 took 36.0 min on `2a79bbe6` and was cut at 45.3 min on `579b848b`, which adds FEEDBACK only. `tools-shard`'s hang guard is 60 minutes since `1314` (ruling 6991936758).
- **The `gates` job is unsharded and has no guard of its own.** Its `cargo test --workspace` took 34.9 min on main and 51.7 min on `1314`: the via ladder runs there too.
- **Weights went missing silently.** `supply_tier1` (since `1303b`) and `via_ring_tier1` had no rows in `ci/test-weights.tsv`. The planner took the 0.05 s default for about 1,500 s of tests and stacked them on one shard. Nothing refused it.

> **What this is:** where CI pays twice for one fact, or pays for a fact no test asserts, paid once, without losing any failure a test exists to catch. And a planner that refuses a heavy test with no weight, instead of guessing.
>
> It is NOT:
> - lowering a gate, weakening an assertion, or skipping, ignoring or quarantining a test (`CLAUDE.md`);
> - production code made faster at any cost to accuracy. The via ladder's eight routings stay, and so does every honest pipeline run;
> - a ninth shard, nextest, a new CI provider, or caching across runs.

## 1. Census

Write it into FEEDBACK. **No test is changed in this phase.**

1. **Every test of 10 CI seconds or more, in `tools` and in `gates`.** Give each its CI seconds, the distinct failure it guards (one sentence, naming the kind, assertion or property), and the expensive work it does: which pipeline, on which input, how many times.
2. **Overlap.** Group the tests that run the same expensive work on the same input, especially the Sentinel-03 and power-fixture emits that `1314` made eight routings each. For each group, give the measured saving of one shared run against the sum (`OnceLock`, or one test asserting several named properties).
3. **Redundancy.** List any test whose guarded failure another already catches, with the evidence: a mutant both kill, or the same assertion on the same output. "Looks similar" is not redundancy. A test that pins a forgery or an adversarial residual is never redundant with one that pins the pass.
4. **`gates` against `tools`.** For each test that runs in both jobs, say what running it twice buys. If nothing, propose where it runs once, and show that no failure is lost (a host-only property, for instance).
5. **The planner's silence.** Propose how `ci/shard.sh plan` refuses, by name, a test that has no weight row and that measures heavy in the run's own entries. The 0.05 s default stays for genuinely light tests.
6. **Predictions.** Predict the suite's CI seconds after the proposals, the eight-shard plan's maximum, and `gates`' wall time.

**STOP** with the table, the groups, the redundancy claims with their evidence, a proposal per group, and the decisions needed. Write `AET-WO-1910 STOPPED: census done, ruling needed on PR #<n>`.

## 2. Build, after the ruling

Expected, to be confirmed by the ruling:
1. **Shared expensive runs.** One run per distinct input inside a binary, every property still asserted by name. A shared run that fails fails every test that depends on it, naming the property, and never passes one silently.
2. **Redundant tests removed only where the ruling accepts the evidence,** each with the mutant the surviving test kills.
3. **The planner's refusal** (§1.5), with a test.
4. **`TIER2_MIN` set to the new exact count,** each removed or merged test named in `ci.yml`'s comment. `ci/test-weights.tsv` refitted by the REFIT RULE. `ci/SHARDS` stays 8.

## 3. Fence and invariants

- **Fence:**
  - test files and fixtures in every crate;
  - `ci/`;
  - `.github/workflows/ci.yml`, for `TIER2_MIN` and its comment, and for moving a test between `gates` and `tools` as ruled;
  - this work order's FEEDBACK.
- **No production code.** A production change that would make a test cheaper is a §7 stop.
- **`INV-04`:** a shared run must not make a test's outcome depend on order. **`INV-06`:** no assertion becomes optional.
- **Running beside you:** `AET-WO-4400g`, which touches the twin, chaos and motor tests.
  - Your census changes nothing, so it cannot collide.
  - Before building, rebase your plan on main. If `4400g` has merged, its tests are in your census. If it has not, leave its files alone and say so.
- One PR, left unmerged. Write `~/.cache/aet-1910/pr-title-body.md`.

## 4. Acceptance

- **The census, ruled.**
- **A coverage map in FEEDBACK:** every removed or merged test, its guarded failure, and the test that now guards it, with the mutant that shows it.
- **CI, measured:**
  - three consecutive runs, each shard's time and the spread recorded;
  - the suite's CI seconds and `gates`' wall time, before and after, measured and not planned.
- **The CLAUDE.md gates pass.**

## 5. Adversarial pass, targeted

As `1909` §5:
1. **Lose a failure:** for each merged group, make one property fail and show CI red, naming it.
2. **Order:** a shared-run binary gives identical outcomes in reverse and permuted order.
3. **Poison:** a shared run that panics once fails every dependent test. None passes and none hangs.

## 7. Flag-and-stop conditions

- A saving needs a production change.
- A test's guarded failure cannot be stated in one sentence. That is a finding, not a deletion.
- Removing anything would lower `TIER2_MIN` below the count of distinct failures the census lists.
- After the build, a shard plan still above 2,400 s, or a measured shard over 50 minutes. The guard and the plan are then put to Scotty, not to this branch.
