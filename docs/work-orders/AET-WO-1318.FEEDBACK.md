# AET-WO-1318 FEEDBACK — one way to read the drawing, true weights, a true README

## Status: READY — built; CI green on 48519439 (run 44728879123); `main` a75fffcc (2305) merged in after it

Builder: Claude (Opus 5.5), a fresh `1318` session, 2026-11-20, from ~10:55 UTC. Worktree `~/aethelgard-1318`, branch `wo/aet-1318-eda-housekeeping` from `origin/main` `fff49a49`, with `main` `7ebca78c` (docs only) and then `a75fffcc` (`2305`, which adds `twin_tier2` rows and raises `TIER2_MIN` to 198) merged in. Neither merge touched a fenced line. The work order built to is `docs/work-orders/AET-WO-1318.md` on `main`; it is identical to the kickoff copy (`diff`, measured).

Labels: **measured** (run here), **read** (read at source), **inferred** (reasoned, not run). Evidence is under `~/.cache/aet-1318/`.

## 1. The readers (§1.1)

- **Callers found** [read: `grep -rn "schematic::anchored\|schematic::bound_holds"` over `crates/`, `ci/`]: seven call sites in four test files. There was no caller in library or CLI code.
  - `anchored`: `hot_plug_tier1.rs` ×4 (L253, L378, L389, L434), `adversarial_1301.rs` L213, `adversarial_1301_tools.rs` L351 (`with-tools` only).
  - `bound_holds`: `outline_tier1.rs` L130.
- **Moved:** each reads `schematic::Drawing::of(..)` and then `.anchored()` or `.bound_holds()`.
  - In the bulk-capacitor test, the second `anchored` and the `drawn` read the same rebound specification. They now share one `Drawing`, which is one `design()` fewer.
  - The refusal check at L434 binds `let Err(e) = Drawing::of(..) else { panic!(..) }`, because `Drawing` is not `Debug`.
  - `hot_plug_tier1.rs`'s module doc now names `Drawing::of`.
- **Removed:** `schematic::anchored` and `schematic::bound_holds`, −22 lines in `schematic.rs`. Every import there is still used [measured: clippy clean with and without `with-tools`].
- **Semantics kept:** the free `bound_holds` returned `Ok` without drawing when no bound is declared. Its one caller (`a_bound_the_placer_cannot_reach…`, `with_bound(60, 60)`) declares a bound. `Drawing::bound_holds` passes on no bound itself [read: `schematic.rs` `Drawing::bound_holds`].
- **`validate` still draws once:** `drawing_holds` in `main.rs` makes its single `Drawing::of`, read by `outline_holds` and `loads_drawn`. This branch does not touch `main.rs` [read].
- **API note:** `aethelgard-eda` 0.2.0's public `schematic::anchored` and `schematic::bound_holds` are removed. No crate outside the workspace is known to call them. The next release's notes should say so, as `1316`'s did for `drawn` and `drawn_nets`.

## 2. The weights (§1.2)

### 2.1 Rule and measurement

- **Rule:** `ci/shard.sh`'s REFIT RULE, rule 5 (L67–75), with rule 3 for a colocated block (L56–57) [read]:
  - `board_tier2` is lock-serialised (`LOCKED`). A test there is weighed at its worst in-shard time: its `ok` timestamp less the previous lock-holder's, or less the binary's `Running` line when it completed first (an upper bound).
  - `outline_tier1` is a block in `ci/colocate.tsv`. It is weighed at its worst libtest `finished in`, rounded up.
- **Runs:** the five green `main` push runs after `1317` merged [measured: `gh run view --job ID --log`, parsed by `kit/measure.py`; logs in `logs/`]:

| Run (head) | `each_size_sample_…` s | `outline_tier1` block `finished in` s |
|---|---|---|
| 44706411946 (`bb1bcb8c`) | 113.46 (first in binary: `ok` − `Running`) | 157.13 |
| 44711555077 (`943f7112`) | 116.67 | 203.26 |
| 44712465606 (`d70a99e5`) | **156.94** (first in binary: `ok` − `Running`) | 195.92 |
| 44722695274 (`d0c4cfe0`) | 98.98 | 163.88 |
| 44724070629 (`fff49a49`) | 119.30 | **237.61** |

- **Rows set:**
  - `tests/board_tier2.rs each_size_sample_passes_every_gate_or_is_refused_by_name`: 1.500 → **156.940**.
  - `tests/outline_tier1.rs` (colocate): 152 → **238**.
  - Each carries a provenance comment naming the runs. No other row and no shard count moved [measured: `git diff origin/main -- ci`, 2 rows].
  - The `test-weights.tsv` comment sits under `1313c`'s estimate line, not at the header's end, because `4240a` and `4405` delete that file's last header line. Appending there would conflict.

### 2.2 Guards

- **The plan is reproduced before any change** [measured]:
  - Entries are the host's `cargo test --workspace --features aethelgard-cli/with-tools -- --list`, through `shard.sh parse`.
  - The bridge's 36 entries come from the CI logs, because the bridge does not link on the host (no `libngspice.so.0`).
  - `shard.sh assign 12 … 37 colocate "$LOCKED"` with `main`'s rows prints **exactly** CI's plan for `fff49a49` (`diff` empty against run 44724070629's `planned` lines).
- **With the new rows** (`plan/local-after.txt`): every shard is planned at **697 s**, under the clause's 1,006 s.
  - The guard printed `guard ok: 2620 test names, 2627 listed occurrences, 12 shards`.
  - **P1** (a row naming no listed test) is inside `assign` and passed.
- **P2** (`heavy_unweighted`, a test with no row past 60 s): empty with the new rows on all 60 shard logs of the five runs [measured].
  - The check is live: with the `each_size_sample` row deleted, it names that test. With the `outline_tier1` block deleted, it names its three slow tests.
- **`ci/test-shard.sh`:** passes (exit 0) [measured].

### 2.3 Plan table, before and after

"Measured" is the run step: `shard.sh run`'s group start to its `shard=K expected=` line. After: run 44728879123 on this PR's head `48519439` (job minutes in brackets).

| Shard | Before: planned (`main` rows, fff49a49 plan) | Before: measured, run 44724070629 | After: planned (this branch) | After: measured (this PR's CI) |
|---|---|---|---|---|
| 0 | 677 | 533 | 697 | 609 (14.1 min) |
| 1 | 677 | 546 | 697 | 771 (15.8 min) |
| 2 | 677 | **772** (holds `each_size`) | 697 | 625 (12.8 min) |
| 3 | 677 | 548 | 697 | 749 (14.4 min) |
| 4 | 677 | 615 | 697 | 528 (11.3 min) |
| 5 | 677 | 748 | 697 | 561 (11.6 min) |
| 6 | 677 | 589 | 697 | 848 (15.7 min) |
| 7 | 677 | 678 | 697 (holds `each_size`) | 430 (9.3 min) |
| 8 | 677 | **847** (holds `outline_tier1`) | 697 | 846 (15.9 min) |
| 9 | 677 | 409 | 697 | 618 (12.8 min) |
| 10 | 677 | 796 | 697 (holds `outline_tier1`) | 600 (12.7 min) |
| 11 | 677 | 387 | 697 | 715 (14.3 min) |

- **Under `main`'s rows**, the shard holding `each_size_sample` ran 984, 831, 1,066, 668 and 772 s against plans of 660–677 [measured, `plan/shards.tsv`].
- **After, on this PR's run 44728879123:**
  - CI printed the same plan as the local one (`diff` of the `planned` lines is empty).
  - `each_size_sample` took 105.74 s in shard 7, which ran 430 s. The `outline_tier1` block finished in 202.47 s in shard 10, which ran 600 s.
  - The slowest shard was 6, at 848 s, holding neither. Every job finished under 16.0 min. The spread (slowest less fastest run step) was 418 s, against 460 s before on 44724070629.
  - The tools aggregate: `tools_total_passed=2627 names=2620`, `tier2_tests_passed=195 floor=195`.
  - `main`'s `2305` (merged after this run) adds rows of its own, so the final head's plan differs by those rows only.
- **On run 44706411946, shard 8's job took 20.1 min** (GitHub's `startedAt` to `completedAt`). That is past the REFIT RULE's 20-minute trigger, on `main` (F1).

## 3. The README (§1.3)

`crates/aethelgard-eda/README.md`, "Under a declared bound" and the refusal sentence. The proposed text from `1317` F7 was taken, and each claim was checked [read, `pcb.rs` at this head]:

| Claim | Where |
|---|---|
| Under a bound only, the canonical rank replaces reference order | `place` L580 (`bounded.then(\|\| canonical_rank(..))`); `placing_order` L866 |
| The rank: footprints, ownership, draw sequence, net graph (colour refinement); the reference breaks only a remaining tie | `canonical_rank` L923, doc L909–916 |
| A part not anchored to an IC targets the centroid of the pads of the part it goes beside, on every shared net | L697 (an anchored part goes to `place_anchored` first, bound or not), L702 `centroid_target`, L1080 |
| Each IC's group is compacted toward the MCU, 0.25 mm steps, sidestepping up to 10 mm, keeping gaps, approach strips, x-range | `compact` L1145, doc L1127–1144; `STEP` L61; `SIDESTEP_STEPS` 40 L1125; called at L725 |
| Connectors on the left and right edges, cable out, in the rank's order | `BOUNDED_PLAN = Sides` L756; `EdgePlan::Sides` doc L747–750 |
| A placement that misses the bound is placed again, seeds 1 to 8 in turn, the first that fits kept | `layout_of` L1978, `RESEEDS = 8` L1963, loop L1996, `done => return done` L1994/L2003 |
| A placement that fits the first time is the one returned | L1992–1995 (seed 0 first); seed 0 is `FNV_BASIS`, `1313c`'s rank (L936) |
| `OutlineUnreachable` names the least-over of nine attempts: larger overrun, then the sum, then the earlier seed | `over` L1985–1989 `(max, sum)`; strict `<` at L1999 keeps the earlier seed on a tie |

- **One correction beyond `1317`'s text.** The old bullet said "a part's target is the centroid". An anchored part never takes that target: `place_anchored` places it by the pads it serves, before the bounded branch. So the bullet now says "a part that is not anchored".

## 4. Equivalence (§1.4)

- **`emit`:** byte-identical to `main` on every committed specification [measured, `eq/`; `kit/emit-all.sh`].
  - Scope: every git-tracked JSON with `voltage_domains`, 53 files. 23 are emitted and 30 refused, with identical exit codes.
  - The 123 files compared (every written file plus each stdout) have identical sha256s. Binaries: `bin/main` (`7ebca78c`) and `bin/branch`.
- **`validate`:** byte-identical on the same 53 files (`corpus-main.txt` = `corpus-branch.txt`, `cmp`).
- **The bench package does not move.** CI's `bench-package` job runs `verify --against-fab fab/sentinel-01-bench/package`, which derives the committed package again and compares it (`.github/workflows/ci.yml`, the job's step "verify --against-fab, the committed package derived again"). It is green on `48519439` [measured: run 44728879123].
- **`cargo test --workspace`:** 2,401 passed, 0 failed, 0 ignored on this branch [measured, `test-workspace.log`]. The pass set is identical to `main`'s, name for name (§6).

## 5. Adversarial pass, targeted (§5)

- **A1. A caller that relied on `anchored` re-running `design()` on a changed input** [measured, `kit/adv.py`, `adv/a1.txt`].
  - Only one converted site reads a changed input: the bulk-capacitor test, which rebinds `spec`. It draws again for the new input.
  - **A1a, a stale `Drawing`:** the first drawing is reused after the rebind. The test **fails**: `hot_plug_tier1.rs:394`, `assertion left == right` (C1 is still anchored to U10). So a missed redraw at that site is caught.
  - **A1b, an in-place edit while a `Drawing` lives:** `spec.board.max_outline_um = None` while `d` borrows `spec`. This is **refused at compile time**: `error[E0506]: cannot assign to spec.board.max_outline_um because it is borrowed`.
  - The other six call sites each build their input once and draw once. Their loops (L253, L434) draw fresh each iteration.
- **A2. A row set so low that the planner packs a shard past its limit on a slow runner** [measured, `plan/sc-*.txt`, `plan/shards.tsv`].
  - Each row is at its worst over five runs, so neither unit is under-planned.
  - Projected at each test's worst measured time, the shard holding it comes to:
    - `main`'s rows (1.5 / 152): 832 s for the `each_size` shard, 763 s for the `outline_tier1` shard;
    - rows set at the lowest of the five runs (98.98 / 164): 744 s and 760 s;
    - this branch's rows: 697 s and 697 s.
  - **The residual is whole-runner variance, not these rows.**
    - Across the 50 shard runs that held neither unit, the worst run step was 1.32 × its plan (44722695274 shard 10, 892 s of 677).
    - Job overhead (log span less the run step) had a median of 139 s, p90 205 s and max 296 s over the 60.
    - A 697 s plan at 1.32 × is 918 s: 17.6 min with the median overhead, 18.7 min with p90, and **20.2 min with both worsts together**.
  - That last case is the REFIT RULE's "one re-run of the same head decides". It is not a fault of the two rows, and it is outside this fence.

## 6. Gates, the reduction pass, and CI

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean; also clean with `--features aethelgard-cli/with-tools` |
| `cargo test --workspace` (host) | **2,401 passed, 0 failed, 0 ignored** |
| Pass set against `main` `7ebca78c` | **identical**: 2,401 names each, `diff` empty (`passset.diff`); `main` built in its own worktree with its target inside the tree |
| CI | green on `48519439` (run 44728879123: gates, bench-package, 12 shards, tools); the final head's run follows this commit |
| Gates on the merged tree (`a75fffcc` in) | fmt clean; clippy clean with and without `with-tools`; `cargo test --workspace` **2,407 passed, 0 failed, 0 ignored** (`2305` adds 6) |

- **Reduction pass (diff):** −10 lines net in code (crates/ less the README: +24 / −34; the README +5 / −4) · 2 duplicate readers removed · 0 dead paths left · 1 protected kept (`pcb.rs`'s `belt-and-suspenders:` outline check, untouched) · 0 findings below.
  - A fresh subagent asked the four questions over `git diff origin/main...HEAD -- crates ci` and found nothing to remove. It checked for stale mentions, unused imports, `Drawing` methods with no caller, and two draws of one input.
  - Spot-checked here: the `grep` for stale mentions is empty, and both clippy configurations are clean.

## 7. Findings

| # | Finding | Proposed |
|---|---|---|
| F1 | **The REFIT RULE's 20-minute trigger fired on `main`.** Shard 8 of run 44706411946 (`bb1bcb8c`) took 20.1 min, start to end. It held `each_size_sample` at its 1.5 s row. That is the lag this WO fixes; the run was green. | The architect's call whether this PR's run counts as the refit's answer, or a wider refit is due. |
| F2 | **`board_tier2::a_missing_clock_enable_fails` ran 62.42 s** (ok less the previous completion) in shard 8 of 44712465606, against a 6.3 s row. On the four other runs it was 6.0–7.0 s: one outlier, perhaps a lock wait. | Outside this fence; for the next refit to judge. |
| F3 | **The `1316` F5 row on `docs/roadmap.md` §7.1** ("`schematic::anchored` and `bound_holds` survive…", L982) is done by this PR. `docs/roadmap.md` is protected. | The architect strikes it. |
| F4 | **The bulk-capacitor test now does one `design()` fewer.** Its 63 s row (`1911`) may sit slightly high [inferred; not measured]. | Harmless; the next refit. |
| F5 | **The bridge does not link on Furnace's host** (`rust-lld: unable to find library -l:libngspice.so.0`). Reproducing the plan locally therefore needs the bridge's list from the image or from CI logs, as done here. | Note only. |

## 8. Neighbours

- **`2305`** merged into `main` (`a75fffcc`) after this PR's first CI run. It came in through a clean merge: it edits other rows of `ci/test-weights.tsv` and adds no caller. The merged tree passes every gate (§6).
- **The four still open** (`1210` `f33e3360`, `4240a` `7dd8799d`, `4241` `6fa7ce35`, `4405` `c7ec348d`) hold `main`'s old call lines and add no new caller [measured: `git diff origin/main -- crates`, `+` lines matching `schematic::(anchored|bound_holds)(`, 0 in each].
  - Whichever lands after this PR takes its edits through a clean merge, with no compile break [inferred].
  - `1210` and `4240a` edit an unrelated line of `outline_tier1.rs`.
  - `4241`, `4240a` and `4405` edit other rows of `ci/test-weights.tsv`.
