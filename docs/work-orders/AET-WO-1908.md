# AET-WO-1908 — CI's shards back under 15 minutes with headroom: weights refitted from CI's own times

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1908-ci-rebalance`.
**Author:** Claude (architect) · **Builder:** a cheap builder with a targeted adversarial pass (`docs/roadmap.md` §6).
**Phase:** infrastructure, carried from `AET-WO-1907` and `AET-WO-1102` §8.

**Grounds:**
- **Eight shards (`ci/SHARDS`), planned from `ci/test-weights.tsv`:** Furnace times scaled by one `CI_SCALE` of 1.8, a 0.2 s default for anything unmeasured, and `attribute_seeded` kept whole.
- **The slowest shard on the last three PRs:**
  - 15:04 on #106 (run 43518552803);
  - about 16:30 on #106's head (run 43523317353; shard 4, 22:56:48 to about 23:13);
  - 14:44 on #107 (run 43526602891).
- **One scale cannot fit every test.** `1102`'s tier-2 test was weighed at 248 s and runs in 130 to 190 s on CI; `2301`'s three tests were weighed from Furnace alone. The planned balance and the real one have drifted apart.
- **The goal (Scotty, 2026-11-13)** is a whole run under 15 minutes. **This work order adds headroom:** the slowest shard under 13 minutes on three consecutive runs.

> **What this is:** the weights refitted from CI's own measured times, and the shard count set from them, so the plan matches what CI does.
>
> It is NOT:
> - a test change;
> - nextest;
> - a change to the guard, the canary or the aggregate's meaning.

## 1. Scope

1. **Measure per test, on CI.**
   - Run the shard plan once with libtest's per-test timing (`-Z unstable-options --report-time` needs nightly, which the image does not have). Alternatively, time each test of a shard by its own `--exact` invocation inside the shard step, in a measurement-only commit.
   - Record each test's CI seconds in FEEDBACK.
   - Say which method you used, and why.
2. **Weights in CI seconds.**
   - `ci/test-weights.tsv` holds CI seconds directly, and `CI_SCALE` goes, or is 1.0.
   - A test with no measured weight keeps a stated default.
   - Drop the colocation of `attribute_seeded` only if the measurements show that splitting it costs less than it saves.
3. **The shard count.** Set `ci/SHARDS` to the smallest count whose planned maximum is at most 11 minutes of test time, so that setup (about 2.5 minutes) leaves headroom.
4. **A refit rule in `ci/` documentation**, a comment block in `shard.sh`, saying how to refresh the weights when a shard passes 13 minutes. **Not a new tool.**

## 2. Fence

- `ci/`, `.github/workflows/ci.yml` (only if `SHARDS` handling needs it), and this work order's FEEDBACK.
- **No test, crate or image change.**
- Bash and awk only (`RUL-12`).
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance

- **Three consecutive CI runs on the PR,** each with every shard under 13 minutes, job start to end. Each run's shard times recorded in FEEDBACK is the commit that starts the next run; no empty commit. Record every shard's time for all three.
- **The guard and the canary pass on each run;** `TIER2_MIN` is unchanged.
- **The measurement table is in FEEDBACK,** with the old weight beside each new one.

## 5. Adversarial pass — targeted

1. **Make the plan put two heavy tests in one shard** with the new weights: ties, rounding, the default weight.
2. **Make a test disappear** from the plan when its weight line is removed or malformed.

## 7. Flag-and-stop conditions

- A single test exceeds about 10 minutes on CI.
- Under 13 minutes needs more than 12 shards.

## 8. Amendment, 2026-11-14 (Scotty's ruling, after the builder's measured runs)

The measured runs showed that the `attribute_seeded` block runs alone on its shard in 13:27 to 13:39 and gets slower when split (its heaviest test takes 632 s alone, against about 382 s inside the block). So no shard count meets §3's 13-minute target. Scotty ruled:

- **`ci/SHARDS` is 8,** balanced as evenly as the plan allows. The block stays whole unless the measurements say otherwise.
- **Acceptance (replaces §3's first bullet):** three consecutive CI runs on the PR, each with every shard under **20 minutes**, job start to end. Record each run's shard times and the spread (slowest minus fastest) in FEEDBACK. The runs already made at other shard counts stay in FEEDBACK as measurements.
- **§1.3's 11-minute planned maximum and §7's "more than 12 shards" condition are withdrawn.**
- **The fence is unchanged: no test change.** Whether every CI test earns its time is `AET-WO-1909`'s reduction pass, a separate work order.
