# AET-WO-1917 — FEEDBACK: CI that cannot skip what it should run

**Branch:** `wo/aet-1917-ci-coverage` from `main` `77e829f1`. `main` `1c32c8ea` (docs only) is merged in.
**Builder:** Claude (Opus 5.5), acting as project manager.
- Two Sonnet subagents built (4) and (6), each in its own worktree. I reviewed every line of both and re-ran one mutant of each.
- One subagent traced the workflow's inputs for §1.1, and I checked each claim at source.
- The workflow, the judge and the Rust tests stayed single-threaded.

Written 2026-11-23.

**Status: READY.**
- Stopped at §1.1 on PR #370 (`b768d70f`); D1-D6 were ruled 16:24Z on the leans.
- Predictions: `spike/1917/predictions.md` (`11082af5`, committed before any measurement). The measurement: `spike/1917/measurement.md`.

**The answer:**
- **(1)** `ardupilot-image.yml`'s two `paths:` blocks name every input its jobs read.
  - They are `container/**` (replacing 24 entries), `bridge/**`, `boards/sentinel-01/**`, and the six non-container entries as before.
  - `image_workflow_paths.rs` derives the inputs; there is no hand list. It has four readers, each proven by a planted miss, and every glob must match some file.
- **(2)** `ardupilot-pinout.sh` judges the boards the workflow's own matrices name. A matrix board with no artifact, an artifact no matrix names, a hand board with no expected findings, and an empty matrix are each refused by name before any board is judged.
- **(3)** `examples/sentinel-01.json` is in the generated matrix as `sentinel-01-full`.
  - Its fixture pair was written by the generator. Both fixture tests hold it byte for byte.
  - Locally, under the pinned ArduPilot image, it builds and passes `check pinout` with no finding.
- **(4)** `route.sh inventory` sees a docs path written through a placeholder or with a trailing slash, under the anchored reading.
  - All 14 measured forms are now seen. The rule adds 0 rows on today's tree.
  - The unanchored residual is a proposed §7.1 row (§5).
- **(5)** `TIER2_RE` is checked both ways against every test file with a `with-tools` cfg (`tier2_re.rs`).
- **(6)** The shard timer reads a result line split by a tool's output on either side.
  - `45198295391`'s shard-5 log is frozen as a fixture, and the timer times all 11 of `attribute_seeded.rs`'s tests and every test in the log.
  - A name left pending is refused by name. Trybuild's lone `ok` (log l.1668) stays uncounted.

## 1. The measurement (§1.1)

See `spike/1917/measurement.md`. Against the predictions:
- **P1: more than predicted.** Seven groups of inputs were unlisted (U1-U7), not 2-4. `boards/sentinel-01/**` (a workspace path dependency), the sibling `*.sources` glob in `check-pins.sh`, and the tools Dockerfiles' inputs were not foreseen.
- **P4: wrong.** `sentinel-01.json` generates hwdef with no refusal, so the stop for `4410` did not fire.
- **P2, P3, P5, P6, P7: held.**

## 2. What was built

### 2.1 The paths and their test (D2)

**Readers in `image_workflow_paths.rs`:**

| Reader | Derives | Planted miss |
|---|---|---|
| COPY | every `COPY` source of every `container/Dockerfile` and `*.Dockerfile`, and the Dockerfiles | `container/copied.txt` |
| named paths | every existing repository path a job names after `jobs:`: a `$(cat F)`, an argument, a `-v "$PWD/D:…"` mount (a bare segment counts only after `$PWD/`), a matrix `spec:`. Every `.sh` or `.py` so derived is read the same way, recursively. A directory stands for every entry under it. | `container/IMAGE`, `mounted/m.json`, `spec/x.json`, `tools/s.sh` and the `data/by-script.txt` it names |
| manifest | a `…/Cargo.toml` so named stands for its package directory (`--manifest-path bridge/Cargo.toml`) | `pkg/Cargo.toml`, `pkg/src/main.rs` |
| path dependencies | every `path =` of the root `Cargo.toml` | `dep/Cargo.toml`, `dep/src/lib.rs` |

- Not inputs, each pinned by the planted test: a path named only in a comment, an absolute or in-container path, `target/`.
- **Dead globs:** every glob of both lists must match some file (`a_glob_that_matches_no_file_is_found`).

### 2.2 The pinout judge (D5)

- `ardupilot-pinout.sh` takes a fifth argument, the workflow, and has three read-only modes: `hand`, `generated` and `boards`. They need no `aethelgard` and no `jq`, which the tools image lacks.
- The hand boards' expected findings are one table, `OWN`, keyed by board (§4).
- `ardupilot_pinout_set.rs` covers it:
  - the real matrices: 4 hand boards and 5 generated, with `sentinel-02-noimu` excluded by its `drop:`;
  - planted refusals: a row with no board, a stray directory, a new hand board, and an empty `include:`;
  - each of those also through the main mode, which refuses before it runs `BIN`.

### 2.3 Sentinel-01's full specification (D1)

- `crates/aethelgard-hwdef/tests/fixtures/sentinel-01-full.{topology.json,hwdef.dat}` were written by `aethelgard solve` and `generate hwdef` from `examples/sentinel-01.json`. The `.dat` equals the §1.1 measurement's byte for byte.
- One `BOARDS` row was added to each of `hwdef/tests/generate.rs` and `cli/tests/hwdef_tier1.rs`, appended so that no indexed use moves.
- No generator change.

### 2.4 `route.sh` (D3), built by a subagent

- `wild(L)` in `inventory` applies when a literal has a `{…}` placeholder or ends in `/`. `{{` and `}}` are literal braces.
  1. The literal must hold an anchor once its placeholders are stripped: a `docs` segment, `fab/`, `README`, `LICENSE` or `.md`.
  2. It is normalised like `norm`. A leading `{…}/` is stripped like `../`, the reading `{root}/` already had; this is the one point beyond the ruled text.
  3. Trailing `/`s are dropped.
  4. Each placeholder is read as `.*` and every other metacharacter escaped, and the result is matched whole against the docs-only paths, directories and names.
- `ci/test-route.sh` gains 19 refused forms and 13 controls that stay `ok`. The controls include `{}/{}`, `{}`, `FAB-NOTES.md states {want}`, the escaped-brace cases, `docs/.{x}`, `rs/W.md{}` and `docs/a.m/`.
- `spike/1917/route-forms.sh`: 18/18 forms are seen.

### 2.5 The shard timer (D6), built by a subagent

- In `times()`, a libtest name followed by other text is held pending. The next line that is exactly `ok`, `FAILED` or `ignored` closes it, at that line's stamp.
- Refused by name:
  - a second name opened, or a `test result:` line, while one is pending;
  - the log ending while one is pending ("the log ends while X is awaiting its result").
- A lone result with nothing pending is not counted. The tally (passes from libtest's `test result:` line) and `n == want` are unchanged.
- `ci/fixtures/p3-45198295391-shard-5.log`: the run's stamped lines, with GitHub's prefix removed (1,645 lines, the same format as `p3-d09a5187-shard-4.log`).
- **Tests:**
  - the real log: 11 `attribute_seeded.rs` rows, and 269 rows in all, which equals the log's `running N` sum;
  - forms (b), (c), and (b) with a tool line between the name and its result;
  - `FAILED` closing a pending name;
  - the real trybuild lines l.1667-1668;
  - five refusals.

### 2.6 `TIER2_RE` (D4)

- `tier2_re.rs` reads the one `TIER2_RE=` line. Its form `^tests/(a|b|…)[.]rs$` is pinned: any other form panics.
- It scans every crate's `tests/*.rs` and the bridge's for a `feature = "with-tools"` cfg outside `//` comments, spaces squeezed.
- Both directions fail by name, each planted.

### 2.7 Edits beside the named fence (flagged)

- **The D1 widening:** the fixture pair and the two `BOARDS` rows.
- **The `pinout` job's one step in `ardupilot-image.yml`.** The fence names "`paths:` and the generated matrix only". D5 needs the judge to read the workflow, so the step passes `.github/workflows/ardupilot-image.yml` as a fifth argument, with a three-line comment.
  - The alternative was to default the path inside the script and leave the step alone. I chose the explicit argument: the step then shows which file the judge reads.
  - If you would rather not have it, it reverts with a one-line default.
- **`ci/fixtures/p3-45198295391-shard-5.log`,** a new fixture beside `ci/test-shard.sh`. The acceptance's "`45198295391`'s shard-5 log times all 11 tests" needs the real log, and `p3-d09a5187-shard-4.log` is the precedent.

## 3. Adversarial pass, targeted (§5)

| # | Attack | Result |
|---|---|---|
| A1 | A check that passes by matching nothing | **Covered:**<br>- D2: the inputs are pinned not-vacuous (14 named inputs); empty `paths:` lists panic; a dead glob was **found unguarded and fixed** (§2.1).<br>- D4: twelve names and both kinds of cfg are required.<br>- D5: an empty matrix is refused.<br>- D3: a regex that matches every line is excluded, because the anchor is required (the `{}` and `{}/{}` controls). |
| A2 | A timer fix that lets a missing result pass | Held. `n == want` per binary and the tally's `test result:` counts are untouched. A pending name at `test result:` or at the end of the log is refused, and both mutants are killed. |
| A3 | A tally that counts a tool line as a test | Held (reasoned):<br>- `tally()` counts only `test result:` lines.<br>- A tool's fake `test result:` line with no `running` before it is refused by `times()` ("a result with no `running` line").<br>- One that lands inside an open binary is refused as "timed n of m".<br>- A tool's lone `ok` with a name pending could close it early: the time is early, never a pass (`n` and the tally are unchanged). This residual is named in `measurement.md`. |
| A4 | A path the measurement missed: a script a job sources, a fixture a container mounts | The readers now derive these. Building the test found two faults in my own reader, both fixed before commit:<br>- `-v "$PWD/container:…"` has no `/` after the prefix, so it was missed;<br>- `"$boards/$name"` leaves a bare `boards/`, which pulled in all of `boards/**`.<br>Each now has a planted case or a real-tree assertion, and its mutant is killed. |
| A5 | A path the derivation cannot see | Residual, named in the test's header: a path a script builds from a variable or a glob (`check-pins.sh`'s `*.sources`). Every such case today is under `container/`, which is listed whole. |
| A6 | Not run locally: the judge's main mode past the set check | No test runs the two judging loops, which need `aethelgard`, `jq` and the ArduPilot ELFs. A mutant that deletes either loop survives every local suite.<br>- CI's `pinout` job is the proof. Its log must show nine `NAME: exit N` lines: four hand boards at exit 1 and five generated at exit 0, `sentinel-01-full` among them.<br>- That run is also the first with `jq` of the `OWN` line `"$defaults${OWN[$name]:+$'\n'${OWN[$name]}}"`. A stubbed run on Furnace's bash proved it gives byte-identical expectations.<br>- I read that log before READY (the bus note). |
| A7 | The awk the checks run under | CI runs `times()` and the Rust tests (which run the judge's modes) in the tools image, whose awk is **mawk** 1.3.4; Furnace's default is gawk 5.3.2.<br>- In the image: `times` on the new fixture gives 269 rows (11 for `attribute_seeded.rs`), and the `hand`/`generated` modes read 4 and 5 boards.<br>- On the host under mawk through a PATH shim: `test-route.sh` 0 FAIL, `test-shard.sh` 0 FAIL (it prints `== awk: mawk 1.3.4`), `route.sh check` unchanged. |
| A8 | `D5` against what `pinout` really receives | `download-artifact` with `merge-multiple` puts `NAME/` directories in one place, and a top-level file would be refused as an extra board. The `mutant-*` pattern is not downloaded. |

**Mutants: one per guard, each killed by its own suite.**
- 21 of mine, re-run on the head after rustfmt and the reduction (D2 ×10 with the dead-glob check, D4 ×5, D5 ×6 with the `OWN` membership check). All 21 were killed, and the tree was clean after each.
- (4): 21 in all, two of them argued equivalent (the subagent's table, with the arguments below).
- (6): 6.
- **(4)'s two equivalent survivors:**
  - Removing the trigger leaves only exact paths, which the existing exact checks already report.
  - Removing the `{{` handling can only add a literal brace, and no docs path contains one.
- I re-ran one of each subagent's mutants myself (route's directory match; the shard's lone-result guard). Both were killed by the tests they named.

## 4. Reduction pass (diff)

Reduction pass (diff): −4 net (in the pass itself) · 1 duplicate removed · 0 dead paths removed · 1 protected kept · 1 finding below
- **Removed:** the hand boards' names lived twice in `ardupilot-pinout.sh`, as `HAND` and as four literal `expect` calls.
  - A board added to `HAND` and to the matrix with no `expect` line would have passed the set check and never been judged.
  - They are now one table, `OWN`. Its expectations were proven byte-identical to the old ones by a stubbed run.
- **Kept:** `shard.sh`'s labelled `belt-and-suspenders:` (P2 and P3 in `result()`) was not touched.
- The build itself folds 24 `container/…` entries in each list into one `container/**`.
- **Finding (not removed, outside the diff):** `spike/1917/route-glob-count.sh` and `route.sh`'s `wild()` state one rule twice. The spike script is the measurement's record and is not run by CI, so it is left as it was measured.

## 5. Findings and questions for the architect

- **F1, a proposed §7.1 row (D3's residual).**
  - Text: "`route.sh inventory` cannot see a docs path written through placeholders with no docs anchor outside them (`format!("{}/{}", a, b)`, `{}`, `{root}/{path}`): 610 distinct literals in 234 files match some docs path by wildcard alone (`spike/1917/route-glob-count.sh`, 2026-11-23)."
  - From `1917` D3; it joins `1911` F4's literal-scanner residuals. Owner: a reader of expressions, not literals, if ever (not a CI work order).
- **F2: `TIER2_RE` matches `tests/NAME.rs` with no crate.** A same-named test file in another crate would count toward the floor.
  - Not reachable today: the feature is `aethelgard-cli`'s alone. `tier2_re.rs` scans every crate, so such a file with the cfg would be named.
  - A row only if a second crate gains a `with-tools`-like feature.
- **F3: unresolved.** What the `aethelgard` binary reads at run time in the workflow's jobs, beyond `models/renode` and the files named on its command line. The subagent grepped and found nothing; nobody ran a file-access trace (`measurement.md`).
- **F4: one message changed.** A `test c ... TOOLok` line is now refused as "c is awaiting its result", not "timed n of m". It is still refused.
- **F5: a host lesson, not this WO's code.** My first workspace test run failed two `aethelgard-chaos` tests with `NotFound`.
  - `target/host` held binaries built in `~/aethelgard-2311`, now removed. Cargo judged them fresh.
  - After `rm -rf target/host`, all passed. The same lesson as `1323`'s.
- **F6: CI cost.** `container/**` makes a tools-image-only PR run this workflow. That is nil in practice, because `crates/**` already triggers it on nearly every PR.

## 6. Gates

- `cargo fmt --all -- --check`: pass.
- `cargo clippy --workspace --all-targets --locked --features aethelgard-cli/with-tools -- -D warnings`: pass on the head. Run on the host, capped scope.
- `cargo test --workspace --locked --no-fail-fast`: **2,934 passed, 0 failed**. Run on the host, from a clean target dir, before the dead-glob check and the reduction; the targets those changed were re-run after.
- `bash ci/test-route.sh`: 77 passed. `bash ci/test-shard.sh`: 150 passed. `bash ci/route.sh check ci/docs-tests.tsv`: 9 rows, unchanged.
- No weight rows: every new test runs in under 0.1 s.

## 7. CI

Nothing was dispatched (§1.3). The image workflow runs on this PR by its own paths. CI's run IDs and results are reported on the bus with READY.
