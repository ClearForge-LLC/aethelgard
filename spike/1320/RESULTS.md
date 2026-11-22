# AET-WO-1320 — results beside the predictions

Predictions: `spike/1320/PREDICTIONS.md`, committed at `bb3b2661` before any run.

Kit:
- **The null sweep** is `1319`'s harness, unchanged: `spike/1319/harness/run.sh` and `compare.sh`. Base is a `git archive` of `f46c3c42`; the branch is an rsync of the worktree. Each has its own target dir, and each log shows `Compiling aethelgard-eda (<that tree>)`.
- **The mutants:** `spike/1320/mutants.sh`.
- **Evidence:** `~/.cache/aet-1320/`: `out-base`, `out-branch`, `out-final`, `compare*.txt`, `mutants.txt`, `run-*.log`, `gates.log`.

| # | predicted | measured | held? |
|---|---|---|---|
| B1 | the sweep selects 34 specifications | 34 specifications, 280 files | yes |
| B2 | `run.sh` runs unchanged at `f46c3c42`; d4diff 277 cases, d1319 84 | exit 0; d4diff 277 cases (926 files), d1319 84 (328 files) | yes |
| B3 | `spike/1319/harness/mutants.sh` names the renamed `schematic_pin_device.rs` | it does (line 10); not run | yes (F1) |
| C1 | all five checks can be written in the `const fn` on stable | all five written; §7's first stop did not fire | yes |
| C2 | today's `ARMS` passes all five; no row touched | the crate compiles; no row's text changed | yes |
| C3 | every existing bad-table test passes unmodified | yes, at the build commit `a10b8e72`. The reduction pass later changed one expected message (FEEDBACK §6) | yes |
| C4 | each check is refused at compile time by its own test, and each deletion mutant is killed by it | K1–K5 killed, each by its own test only (at `a10b8e72`, and again after the adversarial and reduction passes) | yes |
| C5 | the decoupler count is stated per part inside `check()`; no `Arm` field | `DECOUPLERS`, six rows; an unnamed bus part is refused | yes |
| R1 | sweep, d4diff and d1319 identical, branch against base | identical at `a10b8e72` and at the final head (below) | yes |
| R2 | `cargo test --workspace` passes; `TIER2_MIN` unchanged | passes (in-repo target; FEEDBACK §6); unchanged | yes |
| R3 | net change in `crates/` +80 to +200 | **+282**: code +76, tests +206 | **no**: each bad table is written out row by row, one entry per line |

**The null sweep:**
- At the build commit `a10b8e72` (`compare.txt`): `identical: 280 files`; `d4diff identical: 277 cases, 926 files`; `d1319 identical: 84 cases, 328 files`.
- At the final head `f3cc1b2f` (`compare-final.txt`): the same three lines, identical.

**Mutants** (`mutants.txt`, final head; each planted in a copy, with eda's recompile from that copy checked):
- **K1–K5** each delete one check, and each is killed by that check's own test.
- **W1–W5** are the reviewer's weakening mutants. All five survived the first tests and are killed now:
  - W1 and W2: `share_a_pin` without one of its second-end terms;
  - W3: the pairwise loop compares adjacent pins only;
  - W4: `own_name` without the address strap's name;
  - W5: no `b.value.is_none()`.
