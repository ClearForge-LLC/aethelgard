# AET-WO-1917 — CI that cannot skip what it should run: the image workflow's paths, every generated board judged, docs-only routing through a placeholder, and `TIER2_RE` and the shard timer checked

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1917-ci-coverage`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4, from the round-3 review. Written 2026-11-23. **Cadence:** a measurement, then a build with one stop. **After `4408`** (#355), merged.

**Grounds:**
- **Review 3** (`docs/reviews/2026-11-23-mythos-5-2.md`) A15's `paths:` item and A16; owners in `….response.md` (rows A15 `paths:`, A16 ×2).
- **`4408` FEEDBACK** §3 A3 and §5 Q4 (the hand-written pinout list), §5 Q5 (F7's third item: `examples/sentinel-01.json` is not in the ArduPilot matrix). Roadmap §7.1 rows `4408` Q4 and Q5.
- **`4300j`'s bus note, 2026-11-23T15:17Z** (PR #358, run `45198295391`, shard 5): `ci/shard.sh`'s timer failed "timed 10 of its 11 tests" because another test's stdout landed *after* a test's name on the same line (`test a_patch_… ... ANSWER 3200: …`). A tool line *before* a name is handled; one after it is not. That is a false red, and a false red trains people to re-run.

> **What this is:**
> - **(1) `ardupilot-image.yml`'s `paths:`** (both blocks) name every input its jobs read: at least `container/IMAGE` and `bridge/**` (A15), and whatever the measurement finds (`ci/foreign-judge.sh`, from `2311`, is one to check). `image_workflow_paths.rs` (`4408`) proves each one, with a planted test that a missing input is caught.
> - **(2) `container/ardupilot-pinout.sh` judges every `generated-*` artifact it is given,** not a fixed list (`4408` Q4). A matrix board missing from the judge is refused by name, never skipped.
> - **(3) `examples/sentinel-01.json` joins the generated matrix** (`4408` Q5), beside `sentinel-01-minimal`. If hwdef refuses it (for example `SenseNotEmitted`, which `4410` owns), **stop** with the refusal: the row then waits for `4410`.
> - **(4) `ci/route.sh inventory` sees a docs path named through a placeholder or a trailing slash** (A16), such as `format!("docs/work-orders/{}.md", …)` or `"docs/reviews/"`. Fail closed: a literal the reader cannot resolve routes the change to the full run. A test in `ci/test-route.sh` for each form.
> - **(5) `TIER2_RE` is checked, not trusted** (A16): a test that every test file built only under `with-tools` matches it, and that every name it lists exists. A new `with-tools` file outside it fails by name.
> - **(6) The shard timer reads a result line split by a tool's output on either side** of the test's name (`4300j`'s note). A test in `ci/test-shard.sh` from the run's real log line. The tally still refuses a missing result: the fix must not let a test with no result pass.
> - **Each refusal comes with a test, and each test with a mutant it kills.**
>
> It is NOT:
> - any crate's code beyond the tests that check CI's own files;
> - the foreign judge's late panic (`AET-WO-1609`);
> - a change to what any gate requires, or to shard weights except by the REFIT RULE.

## 1. Build

1. **Measure first.** Commit predictions in `spike/1917/`, then record:
   - every file each job of `ardupilot-image.yml` reads, against both `paths:` blocks;
   - the generated matrix against the pinout judge's list;
   - whether `examples/sentinel-01.json` solves and generates hwdef in the tools image;
   - each docs-path literal form `route.sh inventory` misses, with a planted case;
   - the `with-tools` files against `TIER2_RE`;
   - the split-line forms the timer misses, from `45198295391`'s raw log.
2. **Stop with the measurement,** and how (3) lands.
3. **Build to the ruling.** Run `ci/test-route.sh` and `ci/test-shard.sh`, and dispatch nothing: the image workflow runs on the PR by its own paths.

## 2. Acceptance

- **A change to `container/IMAGE` or `bridge/**` alone** triggers the image workflow (shown by the paths test).
- **A generated board absent from the judge** is refused by name (a planted matrix row).
- **A placeholder docs literal** routes to the full run (a planted case).
- **A planted `with-tools` file outside `TIER2_RE`** fails by name.
- **`45198295391`'s shard-5 log** times all 11 tests.
- **The CLAUDE.md gates pass, and CI is green.**

## 3. Fences

- **Fence:** `.github/workflows/ardupilot-image.yml` (`paths:` and the generated matrix only); `container/ardupilot-pinout.sh`; `ci/route.sh`, `ci/shard.sh`, `ci/test-route.sh`, `ci/test-shard.sh`, and weight rows by the REFIT RULE; the tests that read CI's files (`image_workflow_paths.rs` and new ones beside it); `spike/1917/` and FEEDBACK.
- **Not:** any generator, the IR, eda, the twin, `fab/`, other workflows.
- **Running beside you:** `4300j`, `1608`, then `4240e`; none touches `ci/` beyond weight rows. Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-1917/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A check that passes by matching nothing:** an empty list, a glob that matches no file, a regex that matches every line.
- **A timer fix that lets a missing result pass,** or a tally that counts a tool line as a test.
- **A path the measurement missed:** a script a job sources, a fixture a container mounts.

## 7. Flag-and-stop conditions

- **After §1.1,** with the measurement and (3)'s outcome.
- **A fix would need a gate to require less.** Stop.
- **Sentinel-01's full specification is refused by hwdef.** Stop with the refusal; it waits for `4410`.
