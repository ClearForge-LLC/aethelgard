# AET-WO-1918 — CI headroom on GitHub's runners: Sentinel-03's prove peak under a stated margin, the wall bound measured, and the shard refit

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1918-ci-headroom`.
**Author:** Claude (architect) · **Builder:** strong tier (the `4300k` builder, who measured all three), with a targeted adversarial pass.
**Phase:** P4. Written 2026-11-25. **Cadence:** a measurement, a stop, then the build.

**Grounds:**
- **Roadmap §7.1:** the row "Sentinel-03's `prove` peaks at 14.73 GiB of a runner with about 15 GiB" (#399), and the row "the tier-2 campaign tests' 30 s wall bound fails on loaded CI runners" (#398).
- **`4300k`'s measurements** (#390; bus, 2026-11-24 22:02Z and 23:26Z):
  - `kani-driver` peaked at 14.73 GiB in CI (run 45501452199) and 14.97 GiB locally under a 16 GiB cap. Verify runs one harness at a time, so this is one harness's own peak.
  - An earlier run died as `cargo kani exited 101`, now read as a memory kill.
  - `bench-package-s03` took 54 min of its 90-min limit.
  - `verify_2k2_tier2` took 1303.20 s and 1545.60 s, against its `colocate.tsv` row of 624.
  - The green run's shard jobs ran 14–34 min, past the REFIT RULE's 20-minute clause.
- **`4411`'s heads-up** (2026-11-24 23:55Z): Sentinel-03 with the WiFi module may push `bench-package-s03` past 90 min, and adds a `uart4` harness.
- **The wall bound is a product bound, not only a test's:** `renode.rs` and `analog.rs` set `wall: 30 s`, and `simulate` judges `wall_bound_exceeded` by it. A verdict that depends on the host's load is a determinism question (`INV-04`), so its change is ruled, not built blind.
- `RUL-09`, `RUL-13`, `RUL-15`.

> **What this is:**
> - **(1) The prove peak, under a stated margin.**
>   - **Measure first:** every harness's peak RSS on Sentinel-03 and Sentinel-01 (with `kani-driver`'s argv in each sample, so the peak names its harness), and the same with `4411`'s module if its branch is pushed.
>   - **Then shrink the heaviest harnesses**, without weakening what is proven. Every property proven before is proven after: the harness list may split one into several, never drop one. Candidates:
>     - a narrower register image;
>     - a split by peripheral;
>     - a smaller unwind, only with a proof that the bound covers the loop.
>   - **Target:** every harness's peak at or under **12 GiB** in CI, which leaves 3 GiB on a runner with about 15. If that cannot be met without weakening a proof, stop with the numbers; a larger runner is a cost and Scotty's call.
> - **(2) The wall bound: measure, then stop.**
>   - Measure the undisturbed boot's wall time for every campaign scenario and tier-2 test that the 30 s bound judges, across at least five CI runs and on Furnace, with each runner's CPU model.
>   - Say what each would need to pass on the slowest runner with a stated margin, and whether a virtual-time bound already exists that could be the judged bound, with the wall bound kept only as a hang guard.
>   - **Stop with the options.** Changing a product bound changes verdicts and is ruled first.
> - **(3) The refit.**
>   - Re-weigh `ci/colocate.tsv` (`verify_2k2_tier2` and the two re-bind estimates) and `ci/test-weights.tsv` by the REFIT RULE, from CI's own runs.
>   - Re-plan the shards so the longest job meets the REFIT RULE's 20-minute clause at the slowest runner measured. Add shards if needed; GitHub minutes cost nothing on a public repository.
>   - Report `bench-package-s03`'s time against its 90-min limit, before and after (1).
>
> It is NOT:
> - dropping, skipping or weakening any test, proof or gate;
> - a larger runner, a self-hosted runner or Quench (Aethelgard is public);
> - changing the 30 s product bound before (2) is ruled.

## 1. Build

1. **Measure (1) and (2),** with predictions committed first in `spike/1918/`. **Stop** with:
   - each harness's peak;
   - the shrink you propose for each one over 12 GiB, and how it keeps every property;
   - the wall-bound options with their numbers.
2. **Build** (1) and (3) to the ruling, and (2) as ruled. Merge `main` before READY.

## 2. Acceptance

- **Every harness's peak at or under 12 GiB in CI,** read from `bench-package-s03`'s and `bench-package`'s own logs. Each property proven before is proven after, shown by the harness list and each property's name.
- **The planted heavy harness:** a test fails if any harness's measured peak in the CI log exceeds the stated ceiling. The ceiling is a number in the repo, read by the job.
- **(3):** every shard job meets the 20-minute clause at the slowest runner measured, `colocate.tsv` and `test-weights.tsv` are re-weighed from CI runs, and the aggregate still proves every test ran once.
- **(2)** as ruled.
- CI is green.

## 3. Fences

- **Fence:**
  - `ci/` and `.github/workflows/ci.yml`;
  - the prove harness generator and its harness shape (`aethelgard-svd-hal`'s board proofs, and `aethelgard-cli`'s prove gate in `verify.rs`);
  - `spike/1918/` and `docs/work-orders/AET-WO-1918.*`.
- **Not:**
  - any committed board or package;
  - the 30 s bound before (2) is ruled;
  - a test's assertions;
  - the steering documents.
- **Running beside you:**
  - `4400i` (#396), whose `write` dispatch regenerates both bench packages;
  - `4411`, whose dispatch follows #396 and may wait on (1).
  - Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-1918/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A proof weakened to fit:** an unwind cut below the loop's real bound, or a property dropped in a split.
- **A peak measured where it is not judged:** Furnace's number used for CI's ceiling.
- **A shard plan that meets 20 minutes only on a fast runner.**

## 7. Flag-and-stop conditions

- **After the measurement** (the one planned stop).
- **12 GiB cannot be met without weakening a proof.** Stop with the numbers.
- **A harness's peak moves a committed verdict or package.** Stop.
