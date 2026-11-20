# AET-WO-1314b — FEEDBACK

Labels:
- **[measured]**: by a run named beside it.
- **[read at source]**: from a file and line named beside it.
- **[inferred]**: from reasoning, neither read nor run.

## 0. Status: READY

CI is green on `90067e88` (main `cb489f2a`, AET-WO-1911's new CI, merged in): run 44658192021, every job passed, all 12 tools shards, tier-2 191 against `TIER2_MIN` 191 (unchanged). This commit changes this file only.

- **The measurement (§1.1) [measured, `spike/1314b/`]:** with Sentinel-03's `fab_tolerances` block copied into `logger-spi-only-cap` and nothing else changed, **the ladder meets every bound.** It chose **rung 3** (a bounded net's via at 320, bounded nets *not* first). `LOG_SCK` is 26,886 fF against 30,000 fF (2 vias, 44.6 mm; margin 10.4 %), down from 37,508 fF (8 vias, 48.9 mm) routed once. `extract`, run on its own, agrees with the ladder's figure. No §7 condition fired.
- **The cause (§1.2–1.3):** a `capacitance_over_declared_bound` finding carries a new optional field, `cause`, exactly when the specification declares no `fab_tolerances`. It names `board.stackup.fab_tolerances`. The message, kind, severity, exit code and every other finding are unchanged. `verify` copies the finding into its extract gate's line as it is.
- **Tests and mutants (§1.4):** two tier-1 tests and one unit test. Each of the two mutants is killed by the test meant for it (§4.2).
- **No committed artifact moved (§1.4) [measured]:** every committed specification was emitted and extracted by the base binary and by this branch's binary. Every board is byte-identical, and so is every passing `extract` line. The five that fail a bound today gain only `cause` (§2.3).
- **Gates and CI:** §5.

## 1. What is built

### 1.1 `crates/aethelgard-eda/src`
- `schematic.rs` L862 `ladder_runs(spec)`: whether `emit` routes the specification by the ladder. It is `RingRule::of(spec)` being `Some`, the expression `place` sets `ruled` from (L881, L901). It refuses a partial block with the field it lacks, as `place` does. Routing, the ladder and `stack.rs` are untouched.
- `extract.rs` L277 `LADDER_NOT_RUN`: the sentence.

### 1.2 `crates/aethelgard-cli/src`
- `extract.rs`:
  - L101: `Finding` gains `cause: Option<&'static str>`, skipped when absent.
  - L197–208: `extract` reads `Stack::of` and `ladder_runs` from the specification. Either one refuses a partial tolerance block with `FabToleranceUndeclared`, through the one refusal arm that was already there.
  - L335: each `capacitance_over_declared_bound` finding sets `cause` when the ladder did not run. The supply findings set none.
- `verify.rs`: no logic change. The extract gate's line already carries the gate's `findings` verbatim (L547, `gate_report`), so the fact reaches the verdict with them. A unit test shows it (L2358).
  - **On a real verdict:** `4600`'s `verify` of this board (`spike/4600/out/logger-spi-only-cap/verify.json`) has an extract gate (`fail`, exit 1) whose one finding is `extract`'s object exactly. That board's sha256 there, `cc1269fa…`, is the board this branch's as-is `emit` wrote (§2.1). So the gate line will carry `cause` the same way, on the same bytes.

**A premise of the work order's fence, corrected:**
- **What the fence assumed:** `extract`'s error text lives in `aethelgard-eda/src`, and `aethelgard-cli/src` is touched only for the verdict line.
- **What I found:** the `capacitance_over_declared_bound` finding is built in `crates/aethelgard-cli/src/extract.rs` (L290–336 at base), and eda only supplies the figures.
- **What I did:**
  - The finding's field is added there, in the CLI.
  - eda holds the sentence (`LADDER_NOT_RUN`) and the predicate (`ladder_runs`).
  - `verify.rs` gained only a unit test.
- Nothing outside the listed directories changed.

**Where `extract` learns whether the ladder ran:** from the specification alone (`ladder_runs`, `schematic.rs` L862, the same `RingRule::of` as `place` L881/L901), never from the board file. No routing output or board byte changed to carry it, so §7's second condition did not fire.

**A field, not a sentence in the message, and why:** the work order allows either. Appending to `message` would have rewritten every pinned message string (`extract_tier1.rs` L212, L241–242) and any consumer that matches one. An optional field moves no existing string or array length. It is the smaller change for a JSON consumer: its only new failure is a strict "no unknown keys" check, and no such check exists in the repository (§4.1 A3).

## 2. Measured

### 2.1 The two variants [measured, `spike/1314b/run.sh`; outputs in `spike/1314b/out/`]

**How:**
- The binary was `target/debug/aethelgard`, built on Furnace from the base `03d89554` with no change of mine. It is kept as `~/.cache/aet-1314b/aethelgard-baseline-03d89554` (sha256 `f20f1627…`).
- Debug profile, as `4600`'s runs used. It ran on the host, not in a container: `emit` and `extract` run no tool.
- Each step ran under its own scope (`MemoryMax=8G`, no swap) at `nice -n 10`, timed by `/usr/bin/time -v`.
- Furnace has 20 CPUs. Load averaged 6–7 from other builders.

**Inputs** (predictions committed first, `a6327a9d`, `spike/1314b/PREDICTIONS.md`):
- `logger-spi-only-cap.json` is `spike/4600/logger-spi-only-cap.json` with `schema_version` 16 → 17 and **nothing else changed**. `diff` shows the one line.
- `logger-spi-only-cap-tol.json` is that file plus Sentinel-03's `fab_tolerances` block and nothing else. Its 2-layer stackup, `board.rules` and bounds are the logger's.

| # | step | predicted | measured | |
|---|---|---|---|---|
| P1 | `validate`, both | exit 0 | exit 0, both | hit |
| P2 | `emit` as is | exit 0, no ladder | exit 0, no `capacitance_ladder` | hit |
| P3 | `extract` as is | exit 1, `LOG_SCK` 37,508 fF (trace 15,793, vias 12,152, pads 1,563, pins 8,000), the rest pass | **exactly that**: exit 1, one finding, `LOG_SCK` 37,508 fF with 8 vias over 48.875 mm; VIB_SCK 23,854, VIB_MOSI 24,306, LOG_MOSI 29,176 pass | hit |
| P4 | `emit` with tolerances | fits, rung 4–7 (bounded first), `LOG_SCK` smallest at 25–29.5 pF with ≤ 3 vias | **fits at rung 3** (`bounded_via_cost` 320, `bounded_first` false). Smallest margin `LOG_SCK` 26,886 fF, 2 vias | fit, net, figure and vias hit; **rung missed** |
| P5 | `extract` with tolerances | exit 0, agrees with the ladder | exit 0, no finding; `LOG_SCK` 26,886 fF, equal to the ladder's smallest | hit |
| P6 | wall / peak | as is: emit < 20 s, < 1 GB; with tolerances 3–10× wall, < 3 GB; extract < 5 s, < 500 MB | below | hit |

**Every bounded net, routed once and through the ladder (fF, against 30,000):**

| net | routed once (no tolerances) | ladder, rung 3 | margin at rung 3 |
|---|---|---|---|
| `VIB_SCK` | 23,854 · 4 vias · 19.50 mm | 26,636 · 4 vias · 19.75 mm | 11.2 % |
| `VIB_MOSI` | 24,306 · 4 vias · 22.00 mm | 23,376 · 2 vias · 22.25 mm | 22.1 % |
| `LOG_SCK` | **37,508** · 8 vias · 48.875 mm | **26,886** · 2 vias · 44.625 mm | **10.4 %** (smallest) |
| `LOG_MOSI` | 29,176 · 6 vias · 32.625 mm | 26,379 · 2 vias · 37.50 mm | 12.1 % |

At rung 3, `LOG_SCK`'s terms are: trace 13,611, vias 3,712 (2 × 1,856 at the rule's 0.75/0.3 mm via), pads 1,563, pins 8,000. VIB's lines read U40's declared 10 pF as `declared_pins` in both runs; VIB_SCK rises 2.8 pF: its 4 vias at the rule's pad add 1,348 fF (1,519 → 1,856 each), and its trace term 1,434 fF (6,948 → 8,382, at 19.50 → 19.75 mm; more than the length explains, so the route's geometry moved, which I did not trace).

**Wall time and peak RSS:**

| variant | `validate` | `emit` | `extract` |
|---|---|---|---|
| as is | 0.00 s · 20 MB | 0.53 s · 41 MB | 0.04 s · 28 MB |
| with tolerances | 0.00 s · 21 MB | **1.96 s · 164 MB** (eight routings on threads) | 0.04 s · 29 MB |

**Board hashes:** as is `cc1269fa…` and with tolerances `04800c1d…`. The boards are not committed; the JSON lines are, under `spike/1314b/out/`.

**What it means [inferred]:** `4600` F18's failure is the missing ladder, not the placement. With the fabricator's tolerances declared, the engine's own routing meets the bound with 10 % to spare. The placer row that §7's first condition would have fed gets no new evidence from this board.

### 2.2 The cause, on the measured board [measured, this branch's binary]

`extract` on the as-is board now prints the same finding with `"cause": "the router's capacitance ladder runs only when \`board.stackup.fab_tolerances\` is declared, and this specification declares none, so its lines were routed once with no regard to their bounds"`. On the tolerances variant it prints no finding (exit 0).

### 2.3 Bytes: every committed specification, base vs branch [measured, `spike/1314b/differential.sh`, rows in `spike/1314b/out/differential.tsv`]

**What was run:**
- **The specifications:** every committed one outside `spike/`, 29 in all: `examples/`, every test fixture with `net_constraints`, and the docs repros.
- **The two binaries:** the base binary above, and this branch's.
- **The steps:** each specification was emitted by both binaries, and the two boards compared byte for byte. Each board was then extracted by both binaries against its own specification, and the two lines compared byte for byte.

**What it showed:**
- **Every board is byte-identical (22 of 22).** The other 7 specifications are refused by `emit` with byte-identical refusals.
- **Every passing `extract` line is byte-identical (16).** So is `sentinel-02-power`'s refusal (exit 2, both).
- **Five lines differ, and only by `cause`.** Once `cause` is removed from each finding, the two lines are equal. All five fail a bound today and declare no `fab_tolerances`:
  - `d3-24-i2c-12-spi`: 16 findings;
  - `serial-four`: 2;
  - `sentinel-02-shaped`: 2;
  - `solve/stress`: 8;
  - `examples/sentinel-01-ext-bmp581.json`: 1 (`SPI_MOSI` 47,396 fF).
- The first four are the fixtures roadmap §7.1 L886 (L877 before `70ce5403`) names. The fifth is `3403`'s bench specification. Its failure is recorded in `fab/sentinel-01-bench/BENCH-EXT-BMP581.md` L36–38 in prose, and the message it quotes is unchanged.
- **No committed file pins a failing `extract` line except as a run record:** `spike/4600/out/*`, `spike/3303b/out/*`. They record what was printed at their own SHAs and were not touched.

The first run used `b1565adf`, before the reduction. It was run again with the binary of `a3d8001e` (the code head; sha256 `c85a3b8b…`, kept as `~/.cache/aet-1314b/aethelgard-head-a3d8001e`), and every row came out the same. `out/differential.tsv` is that second run.

## 3. Rulings applied

None asked. The work order's choices, as taken:
- the field (§1.2);
- the fact read from the specification (§1.2);
- `verify` with no logic change (§1.2).

## 4. Adversarial pass (targeted, WO §5), mutation, reduction

### 4.1 Adversarial pass [measured, this branch's binary; probes in `~/.cache/aet-1314b/adv/`]
- **A1. The cause printed where the ladder did run.**
  - **Setup:** `a_bound_missed_where_the_ladder_ran_states_no_cause` takes the reference with Sentinel-03's tolerances. `emit` routes it through the ladder (rung 7). The test then extracts that board under the same specification with `SPI_SCK`'s bound 1 fF below its own figure.
  - **Result:** it fails, exit 1, with one finding and no `cause`.
  - **Mutant:** the always-print mutant fails exactly this assertion (§4.2).
- **A2. The cause missing where the ladder did not run: a partial block.**
  - **Probe:** the failing as-is logger board under its tolerances specification, with each of `via_ring_um`, `hole_size_plus_um` and `hole_position_um` removed in turn, and with the block emptied.
  - **Result:** each is refused, exit 2, `FabToleranceUndeclared`, naming the missing field. Each is refused before any line is bounded, so no capacitance finding exists to carry a cause or lack one.
  - **Test:** the existing test `every_reader_of_a_board_refuses_a_partial_tolerance_block_by_name` (`via_ring_tier1.rs`) pins `extract`'s refusal, and it passes on this branch.
  - **The third state** has tolerances declared and the ladder unjudged. The board is routed at today's rung, and `extract` refuses it with the reader's own kind, exit 2, before any finding (`a_board_extract_cannot_judge_is_routed_as_today_and_named`, passing).
- **A3. A verdict whose JSON consumers break.**
  - **Survey [read at source]:** every reader of extract's findings, in `crates/`:
    - `verify.rs` L547 clones them as `Value`s;
    - `preconditions` (L171–185) reads only gate verdicts and `kind`s;
    - `support_judged` reads `kind`;
    - `board_tier2.rs` L1781 asserts `kind`;
    - `extract_tier1.rs` asserts `message` strings, which are unchanged.
  - **No strict reader:** nothing deserializes a finding into a struct with `deny_unknown_fields` (no match in `crates/aethelgard-cli/src`), and no test compares a failing finding's whole object or its key count.
  - **MCP:** it returns `extract`'s line as printed.
  - **The fabrication notes:** they are built from `conditions`, not findings, so a package's FAB-NOTES cannot move.
- **A4 (found while probing): the cause describes the specification, not the board.**
  - **Why:** `extract` takes the board and the specification separately. The fact comes from the specification, as §1.2 orders, so it is exact only when the board was emitted from that specification. It always is inside `verify`, which emits the board it judges.
  - **Mismatch one way:** a ladder-routed board judged under a specification without tolerances is refused, `BoardUnreadable` exit 3 (its 0.75 mm via is not the engine's 0.6). No false cause is possible there.
  - **Mismatch the other way:**
    - **Probe:** tolerances whose rule pad is exactly 0.6 mm (plus 0, position 0, ring 150 µm, `adv/pad600.json`) make a board routed once readable under them.
    - **Result:** its `LOG_SCK` failure carries no cause, though no ladder routed that board.
    - **Assessment:** this is a judged board that is not that specification's emission, which only a hand-supplied pair can produce. **Finding F1** (§6).

### 4.2 Mutation [measured, `~/.cache/aet-1314b/mut/`, one mutant per guard, against `extract_tier1`]

| mutant (at `extract.rs` L335) | killed by | where |
|---|---|---|
| always print: `cause: Some(LADDER_NOT_RUN)` | `a_bound_missed_where_the_ladder_ran_states_no_cause` | `extract_tier1.rs` L1008 (`cause` absent on the ladder's failing board) |
| never print: `cause: None` | `a_bound_missed_with_no_ladder_names_the_field_that_runs_it` | `extract_tier1.rs` L947 (`cause` equals the sentence) |

The file was restored from a copy after each mutant, and `git diff` was clean after the run. Both mutants were run again at the code head `a3d8001e`, in a separate target directory, with the same result: each was killed by the same test at the same line.

### 4.3 Reduction pass (diff)
- **Removed:** `extract`'s second refusal arm. It was a copy of the `Stack::of` arm; one `and_then` now feeds both values into the one existing arm.
- **Kept, each with a reason:**
  - `ladder_runs`, rather than the CLI testing `fab_tolerances.is_none()` itself. It is one predicate with `place`, and a separate test would drift.
  - The `cause: None` on each supply finding. The struct has no default, and the supply findings must state none.
- **Not added:**
  - a `ci/test-weights.tsv` row. The P2 guard asks only for a test past 60 s, and these run about 1.5 s and 5.3 s;
  - a tier-2 test. `TIER2_MIN` is unchanged.

## 5. Gates

Local, at the code head `a3d8001e`. Since then only this FEEDBACK, `spike/1314b/out/differential.tsv` and the docs-only merge of `main` have changed, and nothing in a crate.

**Commands:**
- `cargo fmt --all -- --check`: pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass. It also passes with `--features aethelgard-cli/with-tools`.
- `cargo test --workspace`: **137 test binaries, 2,308 passed, 0 failed, 0 ignored**. It ran under a 32G no-swap scope with `-j 6 --test-threads=6`, from 01:18 to 01:33 UTC, and the log is `~/.cache/aet-1314b/test-workspace.log`. Tier-2 tests are behind `with-tools` and did not run here, as on every host build.
- `cargo test --workspace --features aethelgard-cli/with-tools --no-run`: compiles.

**CI and `TIER2_MIN`:**
- `main` moved to `70ce5403`, a docs-only change (this work order, byte-identical to the copy I built from, and the roadmap). It is merged as `724da8e1`. No crate changed, so the gates above stand.
- `main` moved again, to `cb489f2a` (AET-WO-1911: a shared `tools-build`, the docs-only route, the `ci/docs-tests.tsv` inventory guard). It is merged as `90067e88`, and it changed only `ci/`, the workflow and docs.
  - Before pushing, I ran locally `bash ci/route.sh check ci/docs-tests.tsv` (inventory ok, 9 rows) and `bash ci/test-route.sh` (exit 0).
- **CI on `90067e88`** (run 44658192021, 02:04–02:30 UTC): `ci`, `engine-image` and `ardupilot-image` all succeeded.
  - `gates`, `route`, `tools-build`, `tools-shard` 0–11, `tools`, `bench-package` and the rest passed. `publish` ×2 and `quickstart` were skipped by their own conditions.
  - The `tools` aggregate: `ANSWER tier2_tests_passed=191 floor=191`.
  - The earlier run on `86dfa921` predates `cb489f2a`, so it is not counted.
- `TIER2_MIN` is unchanged, because no tier-2 test was added.

**Resources:**
- Each `emit` and `extract` step ran under an 8G scope. No container was used: no step runs a tool.
- Disk: my `target/` (25 GB) was removed after the gates, when Furnace's free space reached 4.1 GB. It is back to 29 GB.

## 6. Findings

| # | finding | evidence | class | proposed | §7.1 row? |
|---|---|---|---|---|---|
| F1 | `extract`'s `cause` reads the specification, so it is exact only for a board emitted from that specification. Three ways it can overstate or miss: (a) a hand-supplied pair can lack it (a once-routed board under tolerances whose rule pad is the engine's 0.6 mm); (b) its last clause, "so its lines were routed once with no regard to their bounds", describes what `emit` does with that specification, not what was done to the board judged; (c) it is set on every missed bound with no ladder, including one no routing could meet (`extract_tier1`'s 20 pF case fails on its pins alone, 30 pF), where the ladder would only have turned the failure into `emit`'s `CapacitanceOverBound` | §4.1 A4, `adv/pad600.json`; [inferred] for (b) and (c) | risk, low: `verify` always judges its own emission, and in (c) the ladder would still have been the next step | none now; the architect may trim the clause in (b). If `extract` ever judges boards from elsewhere, the board could carry its rung as a comment, which changes the board bytes the work order excluded | no |
| F2 | `crates/aethelgard-cli/README.md` L857 describes the finding's fields and does not name `cause` | read at source | doc | add one clause: "and `cause` where the specification declares no `fab_tolerances`, so no ladder ran". The crate README is outside this fence | yes, a doc row |
| F3 | Five committed specifications fail a declared bound on today's routing, including `examples/sentinel-01-ext-bmp581.json`, which roadmap L886's row does not list | §2.3 | record | add `sentinel-01-ext-bmp581` to L886's row (already recorded by `3403`) | yes, an edit to L877 |
| F4 | `4600` F18's board meets its bound when its fabricator's tolerances are declared (rung 3, `LOG_SCK` 26.9 pF) | §2.1 | answer | F18's "labelled variant measures it first" is done. What remains is the row's other half, `validate`, which the work order excluded | edit L927 |

## 7. Record

- **Branch:** `wo/aet-1314b-ladder-unruled` from `03d89554`.
- **Commits:**
  - `a6327a9d`: predictions and inputs, before any run;
  - `b1565adf`: the build and the measurement's outputs;
  - `a3d8001e`: the verify test and the reduction;
  - then this FEEDBACK.
- **Evidence:**
  - `spike/1314b/`: inputs, `PREDICTIONS.md`, `run.sh`, `differential.sh`, `out/`;
  - `~/.cache/aet-1314b/`: the runs, `diff/`, `adv/`, `mut/`, and the base binary.
