# AET-WO-4310a FEEDBACK: gates' `cargo test` time

Builder: the 4310/4413 builder, 2026-11-28. Branch `wo/aet-4310a-gates-time` from `main` `ef8c46f5`. The work order is the architect's bus ruling of 2026-11-28T04:07Z (no work-order file).

- **Measured first** (`spike/4310a/MEASURE.md`, `ci-binaries.tsv`):
  - The growth is 4411's: Sentinel-03's `emit`, run once per binary on gates' one runner. 4310 and 4413 add nothing: 4310's head ran 2,801 s of binaries against `main`'s 2,923 s, and 4413's head is that time at a 2.07× slower runner.
  - `808766ea`'s 104 min is the runner: untouched CPU-bound binaries slowed 2×, and wall-clock-bound ones did not.
  - It was no hang.
- **The fix is routing:** four binaries join `ci/tools-only.tsv`, under 1910 D5 (a)'s condition. Each runs only `emit` and the library; the libm census finds 1910's five sites, all pinned in gates. Three stay in both, by the same condition.
- **Before and after on Furnace,** as CI runs gates: binaries 2,173 → 1,386 s (like for like); wall 2,313 → 1,521 s (one target directory); all green.
- **Not done, a row:** cheaper Sentinel-03 emits. Each costs about 100 s on Furnace (about 3 min on CI), and that is what keeps gates above its pre-4411 time. It is engine work: the pour ladder's eight-rung routing.
- **Gates:** `ci/test-shard.sh` and the route guard pass. Nothing in a crate changed, so fmt, clippy and the crates' tests are unaffected; CI runs them.
- **Reduction pass (diff):** nothing to remove. The change adds four rows and their comment.
