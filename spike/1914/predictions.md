# AET-WO-1914: predictions, written before the measurement

Written 2026-11-23 02:25 UTC, before any `main` CI log was read for this work order. Base `d09a5187`.

## P1. Unweighted tests in the 10–60 s band on `main`

The held runs: the three newest completed `main` runs of `ci.yml`, all twelve `tools-shard` jobs each.
Per-test seconds: the reconstruction `1910` proposed for P3 (each test's `ok` time less its reconstructed start;
libtest starts tests in name order on `available_parallelism` threads, 4 on a public-repository runner, and each
completion starts the next queued test).
Excluded, as P2 excludes them: a test with a row in `ci/test-weights.tsv`, a test of a binary in `ci/colocate.tsv`.

- **Tier 2** (`TIER2_RE`): **0 to 2** unweighted tests reach 10 s. `1910`'s refit wrote a row for every test it
  measured at 0.5 s or more, and every lock-serialised binary's test carries one by P2's rule, so only a tier-2
  test added since by a builder who did not weigh it can be in the band. Most likely candidates: `with_tools`,
  `models_tier2`, `campaign_tier2` (not lock-serialised, so not forced to carry a row).
- **Tier 1 and units** (the same job runs them): **2 to 6**. `1910` counted 17 at or over 10 s before its refit;
  the refit weighed them; new `aethelgard_eda` units and `boot_tier1` tests since then are the likely ones.
- **None of the 60 s band** (P2 is silent on `main`; its fixtures say so).
- **Straddlers:** 3 to 10 unweighted tests whose worst time over the three runs is in 5–10 s. P3 at 10 s would fire on
  one of them on a slow runner. Run-to-run spread of one test's in-shard time: up to about 1.5×.
- **A test P3 would refuse that no held run measures** (the §7 stop): **none expected**. A test added by `3307`
  (`b22600d3`, `aethelgard-evolve`'s `attribute.rs`) is in a held run once that run finishes.

## P2. The awk warning

`awk: warning: escape sequence '\.' treated as plain '.'` comes from `-v re='...\.rs$'`: gawk (and the One True Awk)
process escapes in a `-v` assignment and warn on an unknown one; mawk passes `\.` through.

Already seen on Furnace before this was written (awk is gawk 5.3.2 by `update-alternatives`), so this part is a
prediction for the runner and the image only:
- **The GitHub runner** (`ubuntu-latest`, where `bash ci/test-shard.sh` runs, outside the image): its `awk` is gawk.
  The warning prints there.
- **The pinned image** (Debian, where `shard.sh run` and `result` run): its `awk` is mawk. No warning; and `TIER2_RE` in
  `ci/shard.sh` already uses `[.]`, so no production call carries a `\.`.
- **The fixtures:** the `\.` is only `ci/test-shard.sh`'s `RE`, passed to `tally` by `result`. The three `result`
  refusals that reach `tally` print the warning as their first line, in each of the 4, 5 and 7-shard suites:
  - "one test fewer ran than assigned";
  - "a shard that passes while running nothing";
  - "a failed test".
  A fourth, P2's "result refuses an unweighted test past 60 s", prints it first too but checks its reason on the next
  line. "a shard file that lists nothing" dies before `tally`: no warning. The `tally` doc-test check prints it to the
  console, uncaptured.
- Under gawk `\.` is `.`, so `RE` also matches `tests/board_tier2Xrs`: the fixtures' regexp is looser than
  `TIER2_RE`, harmlessly.
