# AET-WO-1914: the measurement (§1.1), against `predictions.md`

Measured 2026-11-23, 02:20–02:50 UTC, after `predictions.md` was committed (f164d606).

## Answer first

- **One test on `main` has no row and takes 10 s or more, and it is tier 1, not tier 2:**
  `tests/pin_device_refusals.rs a_power_sense_input_is_not_generated`, 20.971 s worst, 10 s or more on all three held runs.
  **No tier-2 test is in the 10–60 s band.**
- **Two more unweighted tests could reach 10 s on a slow runner**, both tier 1, at 5.758 s and 5.334 s worst. The same test's
  in-shard time varies up to 2.52× from run to run.
- **No §7 stop.** Every test on `main` ran and was timed in a held run: 3,023 of 3,023 on 45098769482 and on 45099268629.
- **The awk:** the runner's (`ubuntu-26.04`) prints the `\.` warning, in exactly the predicted fixtures. The image's (mawk 1.3.4)
  does not. The warning comes from the one `\.` in `ci/test-shard.sh`'s `RE`.

## Method

- **Held runs:** the three newest completed `main` runs of `ci.yml`, all twelve `tools-shard` jobs of each:
  - 45099268629 (`d09a5187`, the base);
  - 45098769482 (`b22600d3`, `3307`'s merge; same tests as the base);
  - 45088311091 (`fa9d7edd`).
  Four more runs (45087684683, 45086603833, 45086155277, 45066478830) are used for the spread only.
- **Per-test seconds:** P3's own `ci/shard.sh times`, run over each job's `shard.sh run` step. `held.sh stamp` turns GitHub's
  per-line UTC stamp into the stamp `run` now writes. One implementation is both the measurement and the build.
- **The reconstruction checked against libtest's own clock** (`witness.sh`, rows in `witness-held.tsv`).
  - libtest prints `test X has been running for over 60 seconds` 60 s after X starts, so that line's stamp less 60 is X's true
    start.
  - Over the three held runs, 145 such lines, P3's start (its end less its seconds) agrees with the true start to within 23 ms.
  - Over the first three runs measured, 50 of 153 witnesses were queued tests, timed from another test's completion: the branch
    that depends on name order and the thread count. Every one agreed.
  - The single-test block of the one test in the band reads 20.971 s; libtest's own `finished in` for it is 20.97 s.
- **Completeness:** `times` refuses a binary whose results it cannot all time. All 36 held shard logs passed: 3,022 or 3,023
  results each run, equal to the shards' own `passed`.
- **The band:** `band.sh`, against the base's `ci/test-weights.tsv` and `ci/colocate.tsv`, as P2 excludes. Output for the held
  three, every test of 3 s or more: `band-held.tsv`.

## The band (`band-held.tsv`)

| test (all tier 1) | worst, held three | runs at 10 s or more | row added |
|---|---|---|---|
| `tests/pin_device_refusals.rs a_power_sense_input_is_not_generated` | 20.971 | 3 of 3 (7 of 7) | 20.971 (required) |
| `tests/emit_tier1.rs a_pin_device_beside_a_test_point_validates_solves_and_emits_with_stable_references` | 5.758 | 0 (worst of 7: 6.530) | 5.758 (margin; D2) |
| `tests/generate_tier1.rs a_declared_hse_validates_and_solves_and_h1_is_not_refused_as_hse` | 5.334 | 0 (worst of 7: 5.487) | 5.334 (margin; D2) |

No other unweighted, non-colocated test reached 3 s on any of the seven runs.

**The spread** (seven runs, the 131 tests of 2 s or more outside the lock-serialised and colocated binaries): the worst run
divided by the best is 1.50 at the median, 1.90 at p90 and 2.52 at most (`aethelgard_eda`'s `supply::tests`). So a test whose
worst is 4 s or more (10 / 2.52) can reach 10 s. Inside the lock-serialised binaries the spread reaches 42× (queue time), but every
one of their tests carries a row (P2's rule), so P3 never judges them.

## The awk (§1.1, second item)

- **Runner:** run 45088311091's `tools` job (`Image: ubuntu-26.04`), step "the guard's own test", prints
  ``awk: warning: escape sequence `\.' treated as plain `.'`` 13 times:
  - as the reason of "result: one test fewer ran than assigned", "result: a shard that passes while running nothing" and
    "result: a failed test", in each of the 4, 5 and 7-shard suites (9);
  - as the first line of "P2: result refuses an unweighted test past 60 s", whose next check names its reason (1);
  - to the console from the `tally` doc-test check (3).
  The message's form is gawk's. This PR's CI prints the runner's awk by name (`== awk:`, the suite's first line).
- **Furnace:** `awk` is gawk 5.3.2 (`update-alternatives`; `nawk` is gawk too): the warning prints. The triage's "not
  reproduced" (`docs/reviews/2026-11-23-backlog-triage.md` row 843) does not hold here: `bash ci/test-shard.sh` on `d09a5187`
  prints it in the same 13 places. Why the triage saw none is not known; inside the image it would not print.
- **The image** (`container/IMAGE`, by `capped-run`): `awk` is `/usr/bin/mawk`, mawk 1.3.4 20250131: no warning. `TIER2_RE`
  already uses `[.]`, so no production call carried a `\.`.

## Against the predictions

| prediction | measured | held? |
|---|---|---|
| Tier 2 in the band: 0 to 2 | 0 | yes |
| Tier 1 and units in the band: 2 to 6 | 1 | no: fewer |
| None in the 60 s band | none | yes |
| Straddlers in 5–10 s: 3 to 10 | 2 | no: fewer |
| Spread up to about 1.5× | up to 2.52× (median 1.50) | no: wider |
| No §7 stop | none | yes |
| Runner prints the warning; image does not | as predicted | yes |
| The three `result` refusals per suite, P2's, the `tally` console line | exactly those | yes |

## Files

- `predictions.md`: written first.
- `held.sh`: a GitHub job log's `run` step as `run` stamps it; every shard log of a run through `shard.sh times`.
- `witness.sh`: P3's start against libtest's 60 s line. `witness-held.tsv`: run, shard, binary, test, true start, P3's start, difference.
- `band.sh`: unweighted tests at or over a floor across runs. `band-held.tsv`: its output for the held three at 3 s.
- The job logs themselves were in `~/.cache/aet-1914/logs/` (not committed; `gh api repos/…/actions/jobs/ID/logs`).
