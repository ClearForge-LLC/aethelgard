# AET-WO-1911 — FEEDBACK

**Status: STOPPED at §1.3 with the table (`ci/SHARDS` unchanged at 8 on this branch).** §1.1 (docs-only routing, inventory guard) and §1.2 (one shared build, sameness proven) are built on PR #242. §1.4 waits on the ruling.

**Branch:** `wo/aet-1911-ci-wait` · **Base:** `main` `47ec18f6`, `main` `56639a89` merged in · **Builder:** Claude (Opus 5.5).
- A Sonnet subagent mapped which tests read documents; I checked its two readers against the source.
- The routing, the guards and the shared build are mine (gate logic stays single-threaded).

## Answer first

- **Recommended: `ci/SHARDS` 12.** More shards stop buying wall time at 12.
  - Measured lone: `tools` green at 31.4 min at 12, 31.6 at 16, and 40.7 at 8 (36.1–38.7 before this build).
  - From 12 up, the slowest shard is one indivisible block, `four_layer_fab_tier2` (planned 1,371 s, measured 1,368–1,474 s). It shares one `--fab-out` package (`1910` D3), so it cannot be split across shards.
  - 16 adds four jobs a run toward the account's ceiling and buys nothing.
- **The concurrent-job limit:**
  - GitHub does not report it to my token (`orgs/ClearForge-LLC` → `plan: null`).
  - **Measured, it is consistent with 60 jobs:** in each of four windows, the org's public repos held at 59–60 jobs running for 15–20 minutes while 1–25 more queued. Re-counted after every job had completed, the exact maximum is 60 in every window.
    - Two first counts, a maximum of 70 and a plateau at 31–37, were artifacts of counting while jobs were still in flight.
    - Private repositories' hosted jobs, if any, are not visible to my token.
  - **Job counts:**
    - A scratch run takes N + 5 jobs: 13 at 8 shards, 17 at 12, 21 at 16. At most about N + 2 run at once (`route` and `tools-build` finish before the shards start, and `tools` starts after).
    - Three runs at 16 peak near 54 jobs, under the ceiling by themselves; the queueing came from the other jobs running.
    - A real code PR touching `crates/**` also starts the path-filtered image workflows, which these scratch PRs did not (they changed only `ci/SHARDS`). So they understate a real PR's share of the ceiling.
- **Queueing under three PRs at once is dominated by the other builders' load, not by my shard count.**
  - Three PRs at 8 queued up to 824 s and went green at 42.8–52.4 min.
  - At 12: up to 464 s, green at 35.4–44.5.
  - At 16: up to 547 s, green at 28.0–35.0.
  - The windows had 20–57 other jobs running. The measured queueing is ambiguous as a function of N (§7), so the recommendation rests on the lone times, the ceiling arithmetic and the plan.
- **A docs-only PR is green in 93 s** (scratch #246, run 44578347985, at `0a98b9c0`, before §4's fixes changed `decide` and the list's format): every document-reading test ran. A README edit that breaks `readme_status` turned `gates` red in 91 s (run 44578760473).
- **The shared build produces the same tests with the same outcomes:** 8 of 8 shards byte-identical, 2,470 tests (§2).
- **The shared build, measured, as predicted on the bus before it was built:**
  - It cuts each shard's setup from 211–331 s to 80–375 s (typically 85–240), but puts a 247–322 s build job in series in front of every shard.
  - For a lone PR at 8 shards, that is 2–4.6 min *slower* (40.7 against 36.1–38.7).
  - It saves about 119 s of runner time per shard, net of the download and unpack: 258 s against 139 s before the test step, mean over the 8 shards of the same-commit proof run. Less the build job, that is about 11 runner-minutes a run at 8 shards and about 19 at 12. Runner-minutes per run stay 232–268 at 8, 12 and 16 shards, against 244–260 at 8 before.
  - Whether to keep it is decision D2.
- **Past 12, the levers are not the shard count:**
  - `four_layer_fab_tier2`'s block (about 23–25 min);
  - the serial build (about 4.5–5 min);
  - `bench-package` (28.5–37.9 min in these runs), which now finishes after `tools` at 12 and 16 shards.

## 1. What was built

### §1.1 Docs-only routing (`ci/route.sh`, `ci/docs-tests.tsv`, `ci/test-route.sh`)

- **The `route` job** replaces the `shards` job. It runs, on every event:
  - the routing's own test;
  - the inventory guard;
  - `decide`, which classifies the pull request's merge commit against its first parent and prints every path, its class and which jobs run;
  - the shard matrix from `ci/SHARDS`.
- **The positive list** (`is_docs`): `docs/**`; `*.md`, `README*` and `LICENSE*` at the repository root; `*.md` under `fab/` outside any `fab/**/package/**`.
- **Code** is anything else, and so is:
  - a mode other than 100644 on either side (a symlink, an executable bit, a submodule);
  - an empty diff;
  - a HEAD that is not a two-parent merge;
  - any event but `pull_request`.
- **Renames:** the diff is taken with `--no-renames`, so a rename shows both its old and new paths. A source moved into `docs/` is code by its old path.
- **The required checks are exactly `gates` and `tools`** (the repository ruleset). GitHub counts a skipped job as passing a required check, including one skipped because a job it needs failed. So both run `always()`, and their first step fails unless `route` succeeded and its class is literally `docs` or `code`.
- **On docs-only:**
  - `gates` runs `cargo fmt` and every target `ci/docs-tests.tsv` marks `reads`, on the host. Each target must run at least one test and all must pass.
  - `tools` passes only when `tools-build` and every shard were skipped.
  - `bench-package` is skipped.
  - `main`'s push always runs everything.
- **The inventory and its guard:**
  - `route.sh inventory` reads every string literal, comments skipped, in every Rust source in the tree, with escapes and continuations decoded. It lists those that name a docs-only file: its path, its directory, a docs directory, a docs-only file's bare name, or the stems `README` and `LICENSE`.
  - Each literal is charged, with its number of uses, to the test target that compiles it. A `tests/<dir>/` helper is charged to each binary that declares `mod <dir>;`, and to each binary in any crate that includes it by `#[path]`.
  - `check` refuses, by target, literal and count, a row missing from `ci/docs-tests.tsv`, and a row the inventory no longer finds.
  - `check` also refuses, by file, any of these that names a docs-only path: production `src/`, a build script, an example, the bridge, a source outside `crates/`, a source no test target includes, or an `include_str!`/`include_bytes!` that resolves into a docs-only path.
  - Each row says `reads` (run on a docs-only PR) or `cites` (a crate-level file of the same name), with its reason in a comment. Adversarial fixes, §4.
- **`decide` routes as code unless the merge commit's second parent is the pull request's head** (`PR_HEAD_SHA`).

### §1.2 Build once, test many (`ci/shard.sh build`, `pack`, `unpack`, `compiled_in`)

- **`tools-build`** runs, in the pinned image, the commands each shard ran before: `cargo test --workspace --features aethelgard-cli/with-tools --no-run`, then the bridge's build and test build.
- **`pack`** tars `target/` (cargo-home included, `incremental/` left out) as zstd. Its stamp holds the checkout's tree hash, `container/IMAGE` and the archive's sha256, and refuses a checkout that is not clean. The archive is uploaded once per run.
- **Each shard:**
  - downloads the archive;
  - **`unpack`** refuses it unless the stamp names this checkout's tree, this image and this archive;
  - sets every checked-out file *and directory* to one old mtime (2000-01-01), so cargo reads the build as fresh;
  - extracts it.
- **`plan`** re-runs the build's commands offline, then lists the tests. `plan` and `run` refuse any `Compiling` line cargo prints before a test binary runs.
- **The runner is unchanged:** `cargo test ... -- --exact NAMES`, as before. Nothing about what decides a pass moved (no nextest).

## 2. The sameness proof (§1.2)

- **The run:** 44568116756, head `049a4bf7`. That was a throwaway commit, reverted in `0a98b9c0`, which added an `old-shard` matrix: the per-shard build exactly as on `main` `47ec18f6` (`ci/shard-old.sh` was `main`'s `shard.sh`), beside the new `tools-build` and shards, on one commit, with both sides' test logs kept.
- **What was compared:** every shard's sorted list of (binary, test, outcome), parsed from the libtest result lines.
  - Binaries are keyed by their deps name without the hash.
  - `- should panic` lines are handled.
  - Each list's length equals libtest's own `test result` totals for that shard.
- **The result:** all 8 shards are identical. Together they are 2,470 tests, every one `ok`, with tier 2 at 190 (canary floor 190), and the assignment (`shard.tsv`) is identical. No test changed its outcome (§7: no stop).

| shard | tests | sha256 of the (binary, test, outcome) list, both builds |
|---|---|---|
| 0 | 301 | `4a80263c42e0e23bad096eac91aa0ef067a5502e31f389983d3e8e7786c63a93` |
| 1 | 305 | `5fdd1995a11d7a2541f17774a1930cd8fa9d0588f2b9cc5454201d0063fd56e0` |
| 2 | 301 | `ea3b9497888833eecccd9303f690fe1e8728b209c09843d1590102702600ee34` |
| 3 | 301 | `9769dd07c561baa3f739c65b073968e0d3ebd1a212fff6c78c24f20bc7f6170b` |
| 4 | 315 | `240734b99d101f6dbef702d8f67296446f64968920b9446537b8ea18d810da40` |
| 5 | 311 | `20342ef4a487070d36eed4360f01da4ccefc2e6db0146ac83ba99770e743ef0c` |
| 6 | 313 | `ed072a7b6ee9b480e7ab0238f7da5dcd20d63985f395cc8c0ff7e99ceb705041` |
| 7 | 323 | `cba7b32a1a9ee1d9d4e7ff3be8dabb6edcf90f37f34ef8ed2b70d9a1016ae7a0` |
| all | 2,470 | `b463f7b3c4e5a66c73f7f1b74a0fb4a4f85375e1fe322dabd1f37322f05091a0` (both) |

- **Fresh, measured:** before the build went to CI, a second clone restored the archive and ran the shards' four cargo commands offline. None compiled anything. On CI, every shard's `plan` checks the same thing and `run` refuses a `Compiling` line; every shard of every run since has passed both.
- **Two defects found on the way:**
  - The old shard build's `--all-targets --no-run` does not build the cli examples that the shards' default `cargo test` selection builds, so the old plan step compiled them (0.5 s, unnoticed). The build now builds exactly the selection the shards run.
  - Cargo reads a build script's `rerun-if-changed` directory by the directory's own mtime, so the directories are dated too.
- **What the proof cannot see** (adversarial D1): `tests/common/mod.rs:246` `expected_boundary` recomputes its expectation from the live state. A flip there would be invisible to a comparison of names and outcomes. No such flip was found by reasoning: the bridge grant and `toolchain_clean` see a runner-owned `/repo` in both layouts.

## 3. The shard count (§1.3)

**Method.**
- Every run is the full suite on scratch pull requests based on this branch: A #248, B #249, C #250, all closed after. Only `ci/SHARDS` changed, and each plan is `ci/shard.sh`'s from today's weights.
- "Lone" is one PR pushed with nothing else of mine running. "Three at once" is three pushes within 4 s.
- Times are from the first job's creation (the push, within seconds).
- Setup is a shard's job start to its test step. For shard 0 it includes the in-image fmt and clippy, about 40 s.
- Queue is a job's created-to-started.
- Runner-minutes are `tools-build` plus every shard.
- The org-wide counts are from `~/.cache/aet-1911/tools/concurrency.sh` (every job of every run in the org's public repositories).

| run | id | shards | `tools` green, min | all done, min | build job, s | shard setup, s | shard test, s | max queue, s | runner-min |
|---|---|---|---|---|---|---|---|---|---|
| before 1 (`1910`, per-shard build) | 44477143464 | 8 | 38.7 | 38.7 | — | 211–329 | 1421–1967 | 32 | 260 |
| before 2 | 44542565050 | 8 | 38.3 | 38.3 | — | 211–308 | 1370–1992 | 2 | 246 |
| before 3 | 44549868831 | 8 | 36.1 | 36.1 | — | 230–331 | 1242–1813 | 2 | 244 |
| **lone 8** | 44578349187 | 8 | **40.7** | 40.7 | 295 | 105–211 | 1264–1959 | 2 | 242 |
| **lone 12** | 44585686986 | 12 | **31.4** | 34.8 | 272 | 83–375 | 797–1368 | 3 | 257 |
| **lone 16** | 44592098469 | 16 | **31.6** | 35.0 | 253 | 87–260 | 424–1462 | 3 | 263 |
| three at once, 8 | 44613055563 | 8 | 47.5 | 47.5 | 322 | 100–233 | 1517–1909 | 824 | 248 |
| | 44613055732 | 8 | 52.4 | 52.4 | 282 | 85–228 | 1099–1918 | 822 | 232 |
| | 44613061129 | 8 | 42.8 | 42.8 | 259 | 107–183 | 1330–1899 | 382 | 240 |
| three at once, 12 | 44605214137 | 12 | 35.4 | 35.4 | 318 | 85–237 | 730–1344 | 300 | 250 |
| | 44605212605 | 12 | 44.5 | 44.5 | 274 | 87–243 | 783–1374 | 464 | 242 |
| | 44605218028 | 12 | 42.1 | 42.1 | 247 | 84–303 | 718–1497 | 292 | 257 |
| three at once, 16 | 44598412740 | 16 | 35.0 | 35.0 | 312 | 84–153 | 495–1474 | 547 | 262 |
| | 44598416100 | 16 | 28.0 | 35.2 | 255 | 85–244 | 527–1196 | 3 | 268 |
| | 44598421962 | 16 | 31.9 | 37.9 | 288 | 80–225 | 487–1196 | 259 | 263 |

**Concurrency in the three-at-once windows** (org public repos; running / queued, every 2 minutes):
- **16** (19:24–20:06): 60 running for 19:34–19:52 with 1–20 queued; 36–44 before my push.
- **12** (20:04–20:52): 59–60 running for 20:06–20:34 with up to 19 queued; 57 running when I pushed.
- **8** (20:50–21:40): 60 running at 11 sample points; 20–22 before my push.
- **A first window, 16:36–16:52** (my first two runs, 35 jobs): 59–60 running with 14–25 queued.
- Every window was re-counted after all its jobs completed. The exact maximum is 60 in each, and no sample exceeds it.

**The plan, from today's weights** (`ci/shard.sh assign`, planned seconds per shard):

| shards | slowest planned | mean | slowest / mean |
|---|---|---|---|
| 8 | 1,831 | 1,831 | 1.00 |
| 10 | 1,465 | 1,465 | 1.00 |
| 11 | 1,371 | 1,332 | 1.03 |
| 12 | 1,371 | 1,221 | 1.12 |
| 16 | 1,371 | 916 | 1.50 |
| 20 | 1,371 | 733 | 1.87 |

**Why 12, not 11 or 16.**
- From 11 shards up, the slowest shard is the `four_layer_fab_tier2` block, so the planned wall time stops falling.
- Measured: 31.4 min at 12 and 31.6 at 16.
- At 11 every shard plans within 3 % of that block. With the run-to-run variance roadmap §6 records (about 25 % on one plan, `1314`'s CI), the slowest of eleven near-equal shards is likely to land above the block. At 12, only the block's shard sits at the floor.
- 16 costs four more jobs a run toward the 60-job ceiling for no measured gain.
- **One conflict with roadmap §6.** Its balance gate says no shard is planned more than 10 % above the mean, and 12 plans 12 % above it. No plan at 12 can meet that rule, because the block alone is above the mean. 11 is the largest count that meets it, at an expected cost in wall time. The rule needs a ruling for an indivisible block (D1).

## 4. Adversarial pass (§5, targeted; a fresh Opus subagent)

- **The reviewer:** a fresh Opus subagent working on a clone, with the work order, the diff and the attack list. It was not given my conclusions.
- **Its verdict:**
  - **holds:** the shared build (stale binary, a shard that compiles, shared vs per-shard outcome) and the fail-closed required checks;
  - **broke:** the inventory guard.
- **Kit:** `~/.cache/aet-1911/adv/kit/`. Every fix below has a fixture in `ci/test-route.sh` or `ci/test-shard.sh`.

| # | attack | result | done |
|---|---|---|---|
| A1–A3, A5–A10, A12 | rename, symlink, `include_str!` of a docs path, `Docs/`, `docs` as a file, gitlink, mode change, deletion, non-UTF-8 and newline paths, a newline path injecting `class=docs`, force-push, octopus merge, other events | PASS | — |
| A4, B3 | production `src/` compiling a module from `docs/**` (`#[path]`, `include!`, `include_str!(concat!(…))`), or a docs-path constant a test reads, was charged to `unit` and accepted with a row | FAIL | `src/` naming a docs-only path is now a refusal (`src!`), like `build!` and `example!`; there were no `unit` rows |
| A11 | a merge commit whose parents are swapped, or a PR head that merged `main`, classified as docs | INFO (not reachable with GitHub's merge ref) | `decide` routes code unless `HEAD^2` is the pull request's head (`PR_HEAD_SHA`) |
| B1 | a `cites` row hid any later real read of the same literal in the same target | FAIL | rows pin an occurrence count; a new use is refused until classified |
| B2 | helpers included by `#[path]` (`common/stub.rs` into five binaries; pincheck's `common/mod.rs` into the cli's `pinout_tier1`) charged to the wrong binaries | FAIL | `#[path]` in every crate's `tests/*.rs` is resolved, and the helper is charged to each binary that includes it |
| B4 | `models/renode`, `boards/` and other `.rs` files outside `crates/` and `bridge/` were not scanned | FAIL (latent) | every `.rs` file is scanned; outside `crates/*/` it is `unmapped!` |
| B5 | a walker of `fab/sentinel-01-bench` (`BENCH.md`, `ORDER.md`) | FAIL | each docs-only file's directory is a token. A walk of the repository root is out of reach for a literal scanner: **residual** |
| B6 | literal evasion: `\x2f`/`\u{64}` escapes, line continuation, `cr"…"` desynchronising the scanner, bare `README`/`LICENSE` stems | FAIL | decoded or matched. **Residual:** a path split across expressions (`concat!("do", "cs/…")`, `format!` arguments, `["d","o"].concat()`) |
| B7, B8 | a doctest in a doc comment, a docs path inside a `.json` fixture | FAIL (latent) | **residual:** no Rust doctest exists today; a fixture is data |
| B9, B10 | every row's reads/cites classification; `ledger_tier1` | PASS | `ledger_tier1` reads no document |
| C1–C7 | stale tree or image, corrupt archive, unpack over `target/`, compile anyway, a re-run of one shard, untracked build inputs | PASS | — |
| C3 | a coloured `Compiling` line passes `compiled_in` (not reachable: the colour variable is not passed into docker) | INFO | colour codes stripped |
| C4 | pack from a dirty tree, or unpack over an edited checkout, accepted (no CI step does either) | INFO | the stamp refuses a dirty or untracked checkout |
| D1–D4 | ownership, mtimes, missing `incremental/`, hard links, dropped `--all-targets` | PASS (reasoned) | — |
| E | `gates` or `tools` green on a code PR without tier 2 or the gates' tests | PASS | — |

### Mutants of the guards this work order adds (`~/.cache/aet-1911/mut/run.sh`)

- **24 mutants, one per guard:**
  - the positive list's two exclusions;
  - the mode check and the empty diff;
  - `decide`'s event, merge and PR-head checks;
  - the inventory's `src!` refusal, `#[path]` map, `include!` refusal, every-file scan, directory token and escape decoding;
  - the check's counts, missing, stale and refusal branches;
  - `compiled_in` and its colour stripping;
  - `unpack`'s tree, image, sha256 and existing-`target/` checks;
  - the clean-tree stamp and the mtime normalisation.
- **All 24 killed by their own suite.**
- Two survived the first round:
  - a non-merge HEAD, which the new PR-head check also catches; an octopus merge now separates the two;
  - the refusal branch, whose fixtures matched a message the "missing row" branch also prints; they now match the refusal's own wording.

## 5. Findings for the architect

- **F1: the work order's example `ledger_tier1` reads no document.** It pins the bench rows by hash, building them in memory. It writes `bench-rows-reference.md` only when `AET_LEDGER_DUMP` is set.
  - Found by a subagent, checked in the source (`ledger_tier1.rs:502-526`).
  - The inventory found two readers: `readme_status` (`docs/roadmap.md`, `README.md`) and `package_licences` (the root `LICENSE-*`).
- **F2: the positive list is read with `README*` and `LICENSE*` at the root only** (a fork I took; unknown is code).
  - `aethelgard-kicad-lib` `include_bytes!` its crate-level `data/aethelgard/README.md` and `data/kicad-10.0.7/LICENSE.md` into the library. Read at any depth, an edit to them would skip the build that embeds them.
  - Crate-level `README.md` and `LICENSE-*` are package inputs, checked by the `gates` package step.
- **F3: `quickstart.yml` already routes itself.** It runs on `README.md` (its paths filter). The image workflows' filters cover no docs-only path, so `ci.yml` was the only workflow to route.
- **F4: the inventory guard is a literal scanner.** Its residuals, after the adversarial fixes:
  - a path split across expressions (`concat!("do", "cs/…")`, `format!` arguments);
  - a walk of the repository root;
  - a doctest in a doc comment (none exist today);
  - a docs path inside a data fixture;
  - a cited name reached through a constant already counted (`LICENCE_FILES[1]` in `workspace_shape`).
  The two tests the list marks `reads` are the only ones reading documents today. Each new literal, file, `#[path]` helper or use count is refused until classified.
- **F5: the old per-shard build compiled in its plan step.** Its `--all-targets --no-run` did not build the cli examples that `cargo test`'s default selection builds, so every shard's plan step compiled them (0.5 s), unnoticed. The shared build builds exactly what the shards run.
- **F6: `bench-package` is now the longest job at 12 and 16 shards** (28.5–37.9 min). It is not a required check, but it sets "all done".
- **F7: `four_layer_fab_tier2`'s block is the floor for any shard count.** Splitting it would undo `1910` D3's shared `--fab-out` package: each shard holding one of its tests would pay the shared verify itself, as `ci/colocate.tsv` records for `against_fab_tier2` (about 575 s more machine time). That is a separate work order if wanted.

## 6. Decisions needed

- **D1: the shard count.** Recommended **12**: measured at 31.4 min lone, the floor reached.
  - (b) 11 meets §6's 10 % balance rule but is unmeasured, and expected slower.
  - (c) 16 buys nothing and costs four jobs a run.
  - With 12, §6's balance clause needs an exception for a block that alone exceeds the mean. Suggested wording: "no shard planned more than 10 % above the mean, or above the largest indivisible block".
- **D2: keep the shared build?**
  - **(a) Keep (recommended):** it is what §1.2 ordered and what this branch has.
    - It is proven identical in outcome.
    - It frees about 119 s of runner time per shard (measured on the same-commit proof run), about 19 runner-minutes a run at 12 net of the build job. Under the 60-job ceiling that is less slot time per run.
    - It removes the build's 145–240 s spread from the shards.
    - A lone PR pays about 4.5–5 min of serial build: at 8 shards, 2–4.6 min slower than before.
  - **(b) Revert to per-shard builds:** a lone PR at 12 is predicted to be about 2–3 min faster (the 272 s build job leaves the path, and each shard's setup grows by about 119 s), at about 19 more runner-minutes a run. The docs routing stands either way.
- **After the ruling (§1.4):** set `ci/SHARDS`, refit by the REFIT RULE, rewrite REFIT RULE 4 citing this order and the ruling, record before-and-after wall times over three runs, then the reduction pass and READY.

## Not done

- §1.4 waits on D1.
- The reduction pass runs at close-out, after §1.4.
- `TIER2_MIN` is unchanged at 190: no tier-2 test was added or removed, and every run above counted 190.

## Where the evidence is

- **Runs:** every run id is in the table.
  - The sameness proof's logs are under `~/.cache/aet-1911/proof/`, with the per-shard hashes in `proof-hashes.txt`.
  - Run ids are listed in `~/.cache/aet-1911/runs.txt`, and the concurrency windows in `conc-*.txt`.
- **Tools:** `~/.cache/aet-1911/tools/` (`outcomes.sh`, `runstats.sh`, `row.sh`, `concurrency.sh`).
- **Mutants:** `~/.cache/aet-1911/mut/run.sh`.
- **Adversarial kit:** `~/.cache/aet-1911/adv/kit/`.
- **The local build experiment:** `~/.cache/aet-1911/a` and `b` (13 GB of `target/`, 1.86 GB as tar.zst), removed after the stop.

