# AET-WO-1607 — FEEDBACK: `check support` pinned, test scratch off `$HOME`, `verify` on a board that draws none, the lifecycle banner on every verdict

**Branch:** `wo/aet-1607-cli-backlog` from `main` `28fba951`, with `main` `fa9d7edd` merged in. **Builder:** Claude (Opus 5.5 orchestrating), with one fresh subagent (Opus, chosen for judgement) for the targeted adversarial pass. Its findings were checked before being adopted. Written 2026-11-23.

**Status: READY** (CI in §7). D1 was ruled (b) at 01:15Z and built as ruled. The three edits outside §3's file list were accepted in the same ruling. **The answer:**
- **(1)** `check support` refuses any KiCad but 10.0.7 through `kicad::pinned`, as `check drc` and `check erc` do.
- **(2)** The tier-1 tests that take `build` or `prove` past the pre-flight get a `TMPDIR` outside `$HOME` (`common::private_tmp`: `/run/user/<euid>`; root keeps `/var/lib/aethelgard`).
  - **Base:** with a `~/.cargo/config.toml` and the target outside the repository, 36 tests fail `CargoConfigRefused`.
  - **Now:** 0 fail.
- **(3)** `fab_tier1`'s scratch is under the repository (`target/aet-tests/`). Its one `BaseOutsideRoot` failure is gone.
- **(4)** `readme_status.rs` checks every phase of roadmap §5:
  - a passed phase must be called done in every sentence that names it;
  - an open phase must be named, and never called done;
  - no phase §5 lacks may be named.
  **Today's README passes.**
- **(5)** Plain `verify` on a `board.drawn: false` specification runs every gate it can. `erc`, `drc`, `support` and `extract` are `not_applicable`, with a reason.
  - **Never `pass` (D2, ruled 02:15Z):** with every other gate passed, the verdict is `not_verified`, `reason` `DrawsNoBoard`, **exit 7**. A failed gate is still `fail`, exit 1.
  - Measured in the image on `d1-drv8876`: ten `prove` runs, `build` and `simulate` pass. The tier-2 test asserts the whole outcome (§7).
  - Both fab flags still refuse with `DrawsNoBoard`.
- **(6)** The banner is on the `emit`, `check drc --spec`, `check support`, `prove --spec` and `check pinout` verdicts, equal to `validate`'s, `eol` as `nrnd`. It is on `solve`'s stderr (D1 (b)), and `solve`'s stdout stays the topology alone.
- **No emitted file and no `fab/` byte moved.** Over the null sweep of 34 specifications, the only lines that move are the ones (6) names (§2). `TIER2_MIN` is unchanged, and no test was weakened.

## 0. How it was made

- **Predictions first:** `spike/1607/PREDICTIONS.md`, committed at `44742a42` before the baseline was read; results beside them in `spike/1607/RESULTS.md`. Two things were measured before it and are recorded there as measured: D1, and the start of the base test run.
- **Bus:** D1 at 00:23Z and the out-of-list edits at 00:43Z (`~/.cache/aet-bus/architect.md`, "from 1607"); ruled 01:15Z (`~/.cache/aet-bus/1607.md`).
- **Kit (`spike/1607/`):**
  - `sweep.sh`: every committed specification through `validate`, `solve`, `emit`, `check erc`, `check drc` (with and without `--spec`), `check support` and `generate`, plus `prove` with and without `--spec` on a chosen subset.
  - `same.sh`: compares two sweeps byte for byte.
  - `mutants.py`: 22 mutants, run under the acceptance condition of §2.
  - Each tree is a `git archive` of one commit. Each binary runs in the pinned tools image (`e5fc7ddb…`) under `~/bin/capped-run`: 8 GiB (16 GiB for `verify`), swap 0, caps read back, `--network none`.
- **Evidence:** `~/.cache/aet-1607/` (`sw/`, `m/`, `mut/`, `vnb/`, `d1/`, `adv/`).

## 1. What was built

- **(1)** `main.rs` `check_support`: `kicad::pinned(version(…))`.
  - `doctor.rs`: `KICAD_PINNED` is gone. An off-pin KiCad now blocks every subcommand in `KICAD`, `check support` included (out-of-list edit (a), accepted). Its `doctor_tier1` assertion and one CLI README sentence follow.
- **(2)** `tests/common/mod.rs` `private_tmp`:
  - as root, `/var/lib/aethelgard`;
  - otherwise, `/run/user/<euid>/aethelgard-tests`, derived from the uid and never from an environment variable;
  - it panics, naming why, if the base is missing or resolves under the canonical `$HOME`.
  - `tools_tier1.rs`'s and `generate_tier1.rs`'s runners pass it as `TMPDIR`. They left `TMPDIR` unset, and that is where 36 of the 37 base failures were (out-of-list edit (b), accepted).
  - `generate_tier1`'s config-above test now builds its directories in `private_tmp`, not under `$HOME`.
- **(3)** `fab_tier1.rs` `scratch`: `<repo>/target/aet-tests/…`, which git already ignores.
- **(4)** `readme_status.rs`, rewritten:
  - every sentence of the status line is read, matched by whole words;
  - a done word ("complete", "passed", "done", "finished", "met") counts only with no word that takes it back ("not", "incomplete", "waits", "under", …).
  - The edited-document test catches 13 wrong lines, five of them the adversary's (§5 F1).
- **(5)** `verify.rs`:
  - `DrawsNoBoard` is refused only with a fab flag;
  - `run_gates` runs `board_gates` on a drawn board, or pushes `BOARD_GATES` as `not_applicable` with `reason`;
  - `passes(…, drawn)` excuses `not_applicable` only for those four gates and only when not drawn;
  - `outcome(passed, drawn)` decides the verdict, `reason` and code in one place: `pass`/0 only on a drawn board, `not_verified`/`DrawsNoBoard`/7 on a board not drawn whose gates passed, `fail`/1 otherwise. `Report` gains `reason`, skipped when absent, and `main.rs` gains `EXIT_NOT_VERIFIED = 7`;
  - a board file's hash is read with `get`, falling back to `""`, which never matches a hash (`BindingMismatch`), and the BOM is read only on the fab path.
  - `Gate` gains `reason`, skipped when absent, so a drawn board's verdict is byte-identical.
  - `emit_tier1`'s stand-in test drops its plain-`verify` row (out-of-list edit (c), accepted).
- **(6)** The banner sites:
  - `emit`: `EmitReport.lifecycle`;
  - `check drc --spec`: `v.lifecycle` from `--spec`;
  - `check support`: likewise;
  - `prove --spec`: `generated_board` returns the banner beside the crate;
  - `check pinout`: `pinout::Verdict.lifecycle`, from the topology (the banner line only);
  - `solve`: `eprintln!`, one line each.
  - Each is skipped when empty.
  - The CLI README's lifecycle and `verify` sentences name them.
- **Tests:** `tests/cli_backlog_tier1.rs` (9 tests, stub `kicad-cli` and Kani, `validate`'s banner as the oracle) and one unit test in `verify.rs`.

## 2. Measurement and acceptance

- **Null sweep, base against the final head `8ff08248`** (34 specifications, the tools image; `RESULTS.md`): **94 files differ, every one as (6) names.**
  - 69 stdout lines, each differing from base only by a `lifecycle` key equal to that specification's `validate` banner: `emit`, `check drc --spec` and `check support` of the 23 drawn, bannered specifications.
  - 25 `solve` stderr files, each now exactly the banner's lines. `solve`'s stdout is byte-identical on all 29 that solve.
  - Every emitted file (`emit/`'s three, `generate`'s whole tree) and every other line (`validate`, `check erc`, plain `check drc`, `generate`) is byte-identical.
  - No `fab/` byte moves; this branch touches no file in `fab/`.
  - A repeat base sweep is identical over 1,178 files.
- **`prove` with Kani** (image; `sentinel-01-minimal`, `sentinel-02-power`, `sentinel-02-bench`; base and branch):
  - every `prove` exits 0 and passes;
  - `prove --spec` differs only by `lifecycle`, equal to `validate`'s: NRND; NRND and EOL; none on the active board;
  - plain `prove` is unchanged.
- **(5) in the image:** FILLED BELOW.
- **(2)/(3) acceptance:** CLI tests on base, with `HOME` a private directory holding `.cargo/config.toml` (`CARGO_HOME`, `RUSTUP_HOME` the real ones) and `CARGO_TARGET_DIR` outside the repository: **37 failures** (`m/base-failures.txt`). On the branch, the touched suites (207 tests) pass under the same condition. The adversary ran the whole CLI suite so: 968 passed, 0 failed. Every mutant run (§7) is made under it too.

## 3. Decisions

- **D1, ruled (b):** `solve` prints the banner on stderr. Measured: a `lifecycle` key on `solve`'s stdout is refused by `generate hwdef --topology` and `check pinout --topology` (`SchemaViolation`, exit 2), which CI pipes it into (`ardupilot-image.yml` 288–289, 410). **(a) remains a later row**, as ruled: a strict topology reader that accepts a `lifecycle` key and refuses it unless it equals the banner recomputed from the topology. The architect adds it to the roadmap.
- **D2, ruled 02:15Z, not as I leaned.**
  - What I raised: the first build let a no-board `verify` say `pass`. Measured: `d1-drv8876` did. I leaned to keep it.
  - The ruling: `INV-03`'s "verified" needs ERC, which a board not drawn never has, so it is never verified.
  - Built as ruled: the run reports every gate it could run, and with all of them passed its verdict is `not_verified` with `reason` `DrawsNoBoard`, `ok: false`, exit **7**.
  - **Why 7:** it is the next free code in the CLI's scheme.
    - `0`–`5` are pass, fail, invalid specification, unreadable input, tool unusable and tool error.
    - `6` is `propose`'s generator report. Reusing it would give one code two meanings.
    - `64` and `70` are usage and internal errors.
    - `7` is distinct from `1`, so a caller can tell "not verifiable" from "a gate failed".
  - A run with a failed gate stays `fail`, exit 1: a failure is still a failure, and it says more.
  - The CLI README's exit-code table has the row.
  - **Tests:** `verify::tests::a_board_not_drawn_never_exits_0` (all four combinations; mutant M19), and the tier-2 `with_tools::a_board_not_drawn_runs_its_gates_and_is_never_verified` (exit 7, `not_verified`, the four board gates `not_applicable`, every other gate `pass`).
- **D3, `reason`:** a new optional field on a gate entry, present only on a `not_applicable` gate. Built without asking: the WO says "report the board gates as not applicable", and a bare verdict word would not say why.

## 4. Predictions against results

All in `spike/1607/RESULTS.md`. The misses:
- **P2:** narrower than predicted. The failures were in `tools_tier1` (35) and `generate_tier1` (1) only, not `lifecycle_tier1`, `boot_tier1` or `mcp_tier1`.
- **P5, partly missed:** only the named `fab_tier1` test failed. Lines 350 and 602 are refused before `read_base`.
- **P6, missed:** no other file failed with the target outside.
- **G2, missed:** +807 net lines in `crates/`, against +250 to +600. Most of it is `cli_backlog_tier1.rs` (478 lines).

## 5. Adversarial pass (§5), targeted

A fresh read-only subagent (Opus), given the work order and the diff, not these conclusions. Its report is in this session; it could not write `adv/REPORT.md`. It found no high or medium defect in (1), (5) or (6). Each attack:
- **An `eol` verdict path without the banner: none.**
  - It checked every return of the five sites. Failing DRC, support and pinout verdicts carry the banner; refusals do not, as with `validate`.
  - It checked that `validate`'s banner equals the topology's on all 24 bannered committed specifications, `imu-baro-1v8`'s two EOL parts included.
  - It checked that MCP passes verdicts through unchanged.
- **A no-board `verify` reporting a board gate passed: none.**
  - `passes` and `run_gates` read the same `board.drawn`.
  - The `""` hash and the empty BOM are unreachable for a drawn board. Every board gate bound to `""` would fail closed.
  - `--against` is now reachable for a no-board specification, and is sound: an identical tree runs on; a stray `kicad/` file fails with `against_extra`.
  - Plain `verify` on all five no-board specifications reaches `prove` with no panic.
- **A test scratch under `$HOME` through an environment variable: none, CONFIRMED.**
  - The adversary planted `HOME` with `.cargo/config.toml` holding the repository, target and `CARGO_TARGET_TMPDIR`, and ran 17 tier-1 binaries: all pass.

Findings and what was done:

| # | Severity | Finding | Done |
|---|---|---|---|
| F1 | low, real, CONFIRMED | `readme_status` passed five wrong lines: "incomplete" (substring), "done"/"met"/"finished", a contradicting sentence, and a claim after the first sentence naming no phase | **Fixed** (`be71206e`): whole words, every sentence, every sentence about a phase must agree. All five are test cases; mutants M5c, M5d added |
| F2 | low/medium, PLAUSIBLE | As non-root, `private_tmp` needs `/run/user/<euid>`; with no login session it panics (fails closed). CI's host tier-1 step runs as the runner user | **Measured by CI** (§7) |
| F3 | low, CONFIRMED | `the_private_scratch_is_outside_home` panicked with `HOME=/` or unset, which the helper tolerates | **Fixed:** it reads `HOME` as the helper does |
| F4 | low, residual, CONFIRMED | If `<repo>/target` is a symlink out of the repository, `fab_tier1`'s named test fails `BaseOutsideRoot` again | **Residual (§6).** A scratch elsewhere under the repository needs a `.gitignore` row, outside the fence |
| F5 | low, outside (6)'s list | `generate hwdef --topology`'s report has no `lifecycle` key; it reads the same topologies | **Row (§6)** |

The adversary also noted, for D1: a second stdout line would break `solve > t.json` and, over MCP, make the call an error (`mcp.rs` accepts one line except for `campaign`). MCP drops stderr on success, so **`solve`'s banner is invisible over MCP** (§6).

## 6. Findings, not fixed here

- **F-A (D1 (a)):** the strict topology reader that accepts and checks `lifecycle`, so the banner can be in `solve`'s verdict line and seen over MCP. The architect's row.
- **F-B (F5):** `generate hwdef`'s report carries no banner. One line in `hwdef.rs` (outside this fence) would add it, from the topology, as `check pinout` does.
- **F-C:** the KiCad files' banner (the WO's later work order). Measured here: the emitted files carry no `LIFECYCLE` line on any specification, and `cli_backlog_tier1` asserts it, so that work order will move that assertion.
- **F-D (F4):** `fab_tier1` under a symlinked `target/`.
- **F-E:** `doctor` still says an off-pin KiCad blocks `verify`, which a no-board `verify` no longer needs. True for every drawn board; very low.
- **F-F:** each banner site takes `lifecycle_banner().unwrap_or_default()`, as the existing sites do. It can only swallow a failure to parse the embedded parts table, which `validate` refuses first. Read only; kept for consistency.
- **F-G:** `cli_backlog_tier1`'s first-harness reader copies `verify::harnesses` (the bin-private parser). A test cannot reach it; reported, not removed.

## 7. Gates, fence, reduction pass, mutants

- **Gates at `8ff08248`** (the code head; later commits are `docs/` and `spike/` only):
  - `cargo fmt --all -- --check`: clean;
  - `cargo clippy --workspace --all-targets -- -D warnings`: clean;
  - `cargo test --workspace` (target in the repository, `TMPDIR` unset, as CI's host job): **2794 passed, 0 failed, 0 ignored**, 161 `test result` lines (`m/ws-final.log`).
- **CI on `f6372ddd` (PR #342): green.**
  - Runs: `ci` 45095787493, `engine-image` 45095787593 and `ardupilot-image` 45095787549. 33 checks pass; 3 are skipped by the workflow.
  - The tier-2 guard reports `tier2_tests_passed=204 floor=203`. `TIER2_MIN` and `ci/test-weights.tsv` are unchanged; no tier-2 test was added.
  - **F2, settled:** CI's host `gates` job runs as the runner user, not root. There `cli_backlog_tier1::the_private_scratch_is_outside_home` and every `private_tmp` caller pass, so the runner has `/run/user/<uid>`.
- **Mutants** (`spike/1607/mutants.py`; 22, one per guard this work order adds or changes). Each ran with a fake `HOME` holding `.cargo/config.toml` and the target outside the tree.
  - **22 of 22 killed at `d2a5737a`, and again at `8ff08248` after the reduction**, each by its named test. Every selection passes on the pristine tree (`mut/result-final.txt`).
  - The groups: M1 (pin); M2 (scratch under `$HOME`, with M2b, its assertion off); M3 (`fab_tier1` scratch); M4, M5, M5b, M5c, M5d (`readme_status`); M6, M6b, M7, M8, M9 (`verify`); M10–M14 and M18 (each banner site); M15 (`doctor`); M16, M17 (the two runners' `TMPDIR`).
- **Fence:** §3's files, plus the three out-of-list edits accepted at 01:15Z:
  - (a) `doctor.rs` `KICAD_PINNED`, its `doctor_tier1` assertion, and the CLI README;
  - (b) `tools_tier1.rs` and `generate_tier1.rs`, on `private_tmp`;
  - (c) `emit_tier1.rs`'s plain-`verify` row.
  - The CLI README's lifecycle and `verify` sentences, which name the new verdicts and `not_applicable`, are (a)'s file.
  - Not touched: `renode.rs`, `campaign.rs`, `ledger.rs` and `ledger_tier1.rs` (4300i's), `tests/fixtures/*.json`, the IR, eda, the generators, workflows and `ci/`.
- **`main` merged:** `fa9d7edd` (docs only: `docs/roadmap.md` and a review), at `d2a5737a`. `readme_status` passes on the merged tree.
- **Reduction pass (diff): +807 net in `crates/` (+927 −120) · 3 duplicates removed (and one module loaded twice) · 0 dead paths removed · 0 protected kept · 1 finding (F-G).**
  - **Removed:** `doctor`'s `KICAD_PINNED`, equal to `KICAD` once (1) landed.
  - **Removed:** the banner-to-lines conversion, written twice in `cli_backlog_tier1` (now `banner_lines`).
  - **Removed:** `generate_tier1`'s re-derivation of `private_tmp`'s root and non-root mode, and its probe scratch.
  - **Also:** `tools_tier1`'s second load of `common/stub.rs` (clippy's `duplicate_mod`).
  - **Proof:** the suites stay green and all 22 mutants are still killed at `8ff08248`.
  - Run inline over the diff: the adversary had already read all of it.
