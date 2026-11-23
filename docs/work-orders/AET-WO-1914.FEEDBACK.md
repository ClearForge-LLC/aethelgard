# AET-WO-1914 — FEEDBACK: the shard planner's P3, and refusal fixtures that check their reason

**Status: CI pending; READY once it is green and P3 is read back from this PR's own shard logs (§5).** One PR, unmerged. ci/ and spike/ only; no crate, specification, `fab/` or workflow change.

**Base:** `main` `d09a5187` (verified live after the build: `origin/main` has not moved). **Branch:** `wo/aet-1914-planner-p3`.
**Predictions:** `spike/1914/predictions.md`, committed `f164d606` before any `main` CI log was read for this work order.
**The measurement:** `spike/1914/measurement.md`.

## Answer first

1. **P3 is built.**
   - `run` stamps every line of a shard's log with `$EPOCHREALTIME`.
   - `result` reconstructs each test's seconds from the stamps and refuses, by name, a test of 10 s or more that has no row and is
     in no colocated binary.
   - It fails closed: a line with no stamp, a stamp that goes back, or a binary whose results it cannot all time is refused, never
     skipped.
2. **The reconstruction is checked against libtest's own clock, not assumed.**
   - libtest prints "running for over 60 seconds" 60 s after a test starts. Over the three held `main` runs, 145 such lines agree
     with P3's start to within 23 ms. On the first three runs measured, 50 of 153 were queued tests, the branch that depends on
     name order and the thread count.
3. **One test on `main` needed a row, and it is tier 1:** `pin_device_refusals::a_power_sense_input_is_not_generated`, 20.971 s.
   - No tier-2 test is in the 10–60 s band.
   - Two more rows are added as margin (D2): an unweighted test's in-shard time varies up to 2.52× run to run.
   - With the three rows, P3 is silent on all 36 held shard logs. Without them it refuses `main`'s own shard 4 by that one name
     (a fixture from CI's log).
4. **Every refusal fixture asserts its reason** (`refuses NAME REASON CMD…`), and fails on any awk warning.
   - `RE` uses `[.]`, so the runner's gawk is silent.
   - Two of the old fixtures did not refuse for the reason they named, and are fixed:
     - "aggregate: a test in two shards" died on a count before it reached the guard;
     - P3's own first draft passed the old `result` fixtures on "no time stamp".
5. **The awk** is gawk on the runner (`ubuntu-26.04`) and on Furnace, which print the warning, and mawk in the image, which does
   not. The warning printed in exactly the fixtures predicted.
6. **Decisions for the architect:** D1 (P3's scope) and D2 (the margin rows). Both are built on my lean, posted on the bus at
   02:38Z. No ruling yet.

## 1. What was built

- **`ci/shard.sh`:**
  - `stamp_lines`: `seconds.micro<TAB>line`, stamped as read, keeping a last line with no newline.
  - `run` pipes each cargo test through it; `PIPESTATUS[0]` is still cargo's.
  - `times LOG THREADS`: each test's seconds. libtest starts a binary's tests in name order on THREADS threads, and each
    completion starts the next. So the j-th by name, for j > THREADS, starts at the (j − THREADS)-th completion.
  - `unweighted WEIGHTS COLOCATE`: the one rule for a weighed test, now shared by P2's `heavy_unweighted` and by P3.
  - `result`:
    - takes the stamped logs;
    - times every suite's log (`RUST_TEST_THREADS`, else `nproc`, as libtest does);
    - strips the stamps for `tally` and P2;
    - refuses `P3_SECONDS=10` or more with no row.
    - There is no upper bound, so a test past 60 s whose libtest warning a tool's output split is refused too.
  - `compiled_in` reads past a stamp.
- **`ci/test-shard.sh`:**
  - `refuses` takes the reason as a required argument. It fails a refusal that does not say it, or that prints an awk warning.
    Two self-tests check `refuses` itself.
  - The nine separate "says so" greps that the argument replaces are gone.
  - The run-log fixtures are stamped, with real `running N tests` and result lines.
  - 17 P3 checks: 14 synthetic, 3 on `main` d09a5187's own shard 4 (`ci/fixtures/p3-d09a5187-shard-4.log`, cargo's and
    libtest's lines of that job's log), judged against that commit's weights and blocks frozen beside it (`p3-d09a5187-weights.tsv`,
    `p3-d09a5187-colocate.tsv`), as P2's fixtures are, so a later rename or refit cannot break it.
  - The suite's first line names the awk it ran under.
  - 137 checks pass under gawk 5.3.2 and mawk 1.3.4 (a PATH shim), each run. `times` gives byte-identical output under both awks
    on all 84 shard logs of the seven runs measured.
- **`ci/test-weights.tsv`:** three rows, inserted in place with nothing else moved, and one comment line naming their source.

| row | seconds | why |
|---|---|---|
| `tests/pin_device_refusals.rs a_power_sense_input_is_not_generated` | 20.971 | 10 s or more on all 3 held runs (7 of 7 measured) |
| `tests/emit_tier1.rs a_pin_device_beside_a_test_point_validates_solves_and_emits_with_stable_references` | 5.758 | margin (D2): 6.53 s worst of 7 runs |
| `tests/generate_tier1.rs a_declared_hse_validates_and_solves_and_h1_is_not_refused_as_hse` | 5.334 | margin (D2): 5.49 s worst of 7 runs |

Each is its worst in-shard seconds over the held runs 45099268629, 45098769482 and 45088311091: an upper bound, like rule 5's.
The plan on `main`'s own list (run 45099268629's `entries.tsv`) is 847 s per shard before and 849 after. At the base weights it
reproduces CI's own shard 0 byte for byte.

## 2. Decisions

- **D1, P3's scope. Built: every test a shard runs, as P2.**
  - The WO's §1.1 says "tier-2 tests". The one test P3 must weigh is tier 1, and so were all 17 of `1910`'s.
  - A P3 that judged only `TIER2_RE` would refuse nothing on `main` today.
  - Narrowing it is one `awk` condition on the binary. The fixtures name binaries outside `TIER2_RE`, so they would change too.
- **D2, the margin rows. Built: two rows.**
  - Across seven `main` runs, the in-shard time of one test (outside the lock-serialised and colocated binaries) varied by up to
    2.52× (median 1.50, p90 1.90).
  - So an unweighted test whose worst is 4 s or more can reach 10 s on a slow runner and fail someone else's CI. Exactly two do.
  - If §3's "the rows the measurement requires" means the 10 s row alone, strike the two: P3 still passes the held runs without
    them.

## 3. Adversarial pass (targeted, §5)

| # | attack | result |
|---|---|---|
| A1 | a test whose time straddles 10 s across runners | **Real; handled by D2.** The spread is up to 2.52×, so the margin rows cover every unweighted test of 4 s or more. Residual: adding rows re-plans, so a test meets new neighbours, and the spread was measured on one plan. This PR's own CI run is the first measurement on the new plan. |
| A2 | a test that straddles 60 s | **Moot for an unweighted test:** P3 has no upper bound, so it refuses one at 10 s before 60 s matters. A weighted test is not judged. The lock-serialised binaries' tests all carry rows (P2's rule). |
| A3 | a log line without a time stamp that P3 silently skips | **Held: refused, by line number** (fixture). So are a stamp that goes back, a result line P3 cannot read (the binary's timed count falls short of its `running N tests`), and a log that ends inside a binary. All are mutated. |
| A4 | a tool line that looks like a result (`trybuild` prints `test tests/ui/x.rs ... ok`) | **Held:** a result names a Rust path (`[A-Za-z0-9_:]+`). Anything else is not timed, and if it was a real result the count check refuses it. In the 36 held logs, no result-like line names anything but a Rust path (grep). |
| A5 | a tool's unfinished line before libtest's result | **Held:** the result is matched at the line's end (fixture). The held logs have no such line (grep), so the fixture is the only evidence. |
| A6 | a wrong thread count | **Mostly fail-safe.** Too many threads starts tests early, which over-reads them (a false refusal). Too few under-reads them, which is silent. `nproc` honours `OMP_NUM_THREADS`, which libtest ignores, so that variable set below the cores would under-read. CI sets it nowhere. It is a finding (F3), not built. |
| A7 | the stamper lagging a burst of tool output | **Negligible:** 81,000 lines/s on Furnace, read a byte at a time from a pipe. A whole shard log is about 1,300 lines. |
| A8 | the clock stepping back during a run | **A false refusal** ("the time stamp goes back"), never a silent pass. Accepted. |
| A9 | the doc-tests | Every `Doc-tests` block runs 0 tests. A doc-test name has spaces, so `parse` already refuses it at plan time, before P3 would. |

## 4. Mutations (one per guard this WO adds or changes; `~/.cache/aet-1914/mutate.sh`)

17 of 17 killed, each by a named fixture:
- the threshold at 60;
- an unstamped line skipped;
- the per-binary count removed;
- the start taken as the binary's;
- an upper bound of 60;
- the back-stamp check removed;
- the thread check removed;
- `compiled_in`'s stamp strip removed;
- the end-inside check removed;
- the weights ignored;
- the colocation ignored;
- P3 not called;
- `RE` back to `\.` (killed under gawk by the warning check);
- the result match anchored at the line's start;
- result names of any characters;
- `refuses`' reason check removed;
- `refuses`' warning check removed.

Two survived on the first sweep, and both were fixed:
- the colocation fixture's log had no test over 10 s, so it could not tell;
- `result`'s "timed equals passed" check could not fire once `times` checks each binary against `running N` and `tally` refuses
  a failed or ignored test. It was removed (§5).

## 5. Gates

- `cargo fmt --all -- --check`: clean. `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: GATES_TEST_RESULT.
- `bash ci/test-shard.sh`: 137 passed, 0 failed, under gawk and mawk.
- CI: CI_RESULT.
- **Reduction pass (diff):** −11 lines net (`ci/shard.sh` −2 against the first draft, the label included; `ci/test-shard.sh` −9 greps) · 2 duplicates removed · 1 dead path removed ·
  1 protected kept · 2 findings below.
  - Duplicates:
    - the weights-and-colocation rule, once in P2 and once in P3, is now one `unweighted`;
    - nine reason greps are replaced by `refuses`' argument.
  - Dead: `result`'s "timed equals passed" check (above).
  - Protected (labelled `belt-and-suspenders:` at the site): P2 stays beside P3. P3 covers every case P2 does, but P2 reads
    libtest's own clock and P3 reconstructs from the thread count, so each covers the other's failure. Removing a ruled guard is
    yours (F5).

## 6. Findings

- **F1. In the image, `main`'s `ci/test-shard.sh` passes refusals for the wrong reason.** The image has no `jq` or `zstd`, so
  the gates refusals and five pack/unpack refusals "refuse" on `command not found`.
  - With reasons asserted they now fail there, which is correct.
  - The suite runs on the runner, where both tools exist, so CI is unaffected.
  - Informational: it is this WO's defect class, seen live.
- **F2. The triage row 843's "not reproduced" does not hold on Furnace.** `bash ci/test-shard.sh` on `d09a5187` prints the
  warning in the same 13 places as the runner. Why the triage saw none is not known.
- **F3. `nproc` honours `OMP_NUM_THREADS`; libtest does not.** If the variable were set in a shard's environment below the core
  count, P3 would under-read queued tests. Not built: CI sets it nowhere, and the cure is either one line in `result` or reading
  libtest's count another way. Backlog if wanted.
- **F4. Four rows understate their test's in-shard worst by more than 2×** over the held runs:
  - `boot_tier1`'s `a_capture_that_is_not_the_pattern_then_the_frame_fails_by_name` (row 6.1, 13.9 s);
  - three of `models_tier2`'s FIFO tests (rows 6.0–7.16, 15.4–15.6 s).
  P3 trusts any row, by design, and rows are alone times by rule 2, so part of the gap is contention. A refit's business, outside
  this fence.
- **F5. P2 is now inside P3.** Kept and labelled (§5). Strike it if one witness is enough.
- **F6. `ci/test-weights.tsv` is not strictly sorted:** one `fab_tier1` row and three `twin_tier2` rows are out of order. The
  three new rows were inserted without moving them. Reordering is a re-weighting PR's churn, not this one's.
- **F7. For the builders beside this one (4300i, 4407, 1607, 2309), once P3 is on `main`:** any test of theirs that takes 10 s
  or more in a shard needs a row, or their CI fails after they merge `main`. The refusal names the test and its seconds. Posted on
  the bus. `1607`'s new `with_tools` plain verify is the likely one.
- **F8. `result`'s temporary directory leaks when it dies** (`trap … RETURN` does not run on `exit`), as `aggregate`'s already
  does. Harmless on a runner.
