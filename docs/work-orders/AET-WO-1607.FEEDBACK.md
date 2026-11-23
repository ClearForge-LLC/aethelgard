# AET-WO-1607 — FEEDBACK: `check support` pinned, test scratch off `$HOME`, `verify` on a board that draws none, the lifecycle banner on every verdict

**Branch:** `wo/aet-1607-cli-backlog` from `main` `28fba951`, with `main` `fa9d7edd` merged in. **Builder:** Claude (Opus 5.5 orchestrating), with one fresh subagent (Opus, chosen for judgement) for the targeted adversarial pass. Its findings were checked before being adopted. Written 2026-11-23.

**Status: READY** (CI green on `ff210f1d`, §7). D1 was ruled (b) at 01:15Z, and the three edits outside §3's file list were accepted with it. D2 was ruled at 02:15Z, MCP's exit 7 and `TIER2_MIN` at 03:30Z. Each is built as ruled. **The answer:**
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
- **No emitted file and no `fab/` byte moved.** Over the null sweep of 34 specifications, the only lines that move are the ones (6) names (§2). `TIER2_MIN` is raised to 205, as ruled. No test was weakened.

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
  - `outcome(passed, drawn)` decides the verdict and `reason`: `pass` only on a drawn board, `not_verified`/`DrawsNoBoard` on a board not drawn whose gates passed, `fail` otherwise. `exit_code(ok, verdict)` reads the verdict as finally stated, after the fab step (which `--against-fab` can turn to `fail`): 0, 7 or 1. `Report` gains `reason`, skipped when absent, and `main.rs` gains `EXIT_NOT_VERIFIED = 7`;
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

- **Null sweep, base against `8ff08248`, and again against the shipped code `33348085`** (34 specifications, the tools image; `RESULTS.md`): **94 files differ, every one as (6) names, at both** (`same-head.txt`, `same-final.txt`). Nothing under `fab/`, `boards/`, `examples/`, `scenarios/` or the CLI's test fixtures differs from `main` (`git diff --stat origin/main...HEAD`: empty).
  - 69 stdout lines, each differing from base only by a `lifecycle` key equal to that specification's `validate` banner: `emit`, `check drc --spec` and `check support` of the 23 drawn, bannered specifications.
  - 25 `solve` stderr files, each now exactly the banner's lines. `solve`'s stdout is byte-identical on all 29 that solve.
  - Every emitted file (`emit/`'s three, `generate`'s whole tree) and every other line (`validate`, `check erc`, plain `check drc`, `generate`) is byte-identical.
  - No `fab/` byte moves; this branch touches no file in `fab/`.
  - A repeat base sweep is identical over 1,178 files.
- **`prove` with Kani** (image; `sentinel-01-minimal`, `sentinel-02-power`, `sentinel-02-bench`; base and branch):
  - every `prove` exits 0 and passes;
  - `prove --spec` differs only by `lifecycle`, equal to `validate`'s: NRND; NRND and EOL; none on the active board;
  - plain `prove` is unchanged.
- **(5) in the image** (plain `verify`, 16 GiB, the bridge beside the binary, `vnb/`):
  - **Before D2:** `d1-drv8876`, `hse-external-8` and `imu-baro-1v8` run every gate. The four board gates are `not_applicable`; every `prove`, `build` and `simulate` passes. `imu-baro-1v8`'s line carries both EOL banner lines.
  - **After D2:** `d1-drv8876` is `not_verified`, exit 7, by the tier-2 test (§7).
  - **`sentinel-02-bench-hse8`:** OOM-killed three times at 16 GiB. In the scope's journal, "the kernel OOM killer killed some processes in this unit", then podman's `container kill`, which is the SIGTERM and the empty output. Proved harness by harness, `every_valid_sequence_i2c1` passes in 103 s and `every_valid_sequence_spi1` dies at 355 s. At 32 GiB that harness passes in 357 s, with a 16.5 GiB peak by the scope's own accounting (F-H).
- **V6 (a drawn board's `verify` unchanged):** plain `verify` on the reference, base binary against head, in the image, gives byte-identical lines (202,708 bytes). The same holds again with the D2 binary, so `reason` is absent on a drawn board.
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

Every prediction is scored in `spike/1607/RESULTS.md` (its last table). The misses:
- **P2:** narrower than predicted. The failures were in `tools_tier1` (35) and `generate_tier1` (1) only, not `lifecycle_tier1`, `boot_tier1` or `mcp_tier1`.
- **P5, partly missed:** only the named `fab_tier1` test failed. Lines 350 and 602 are refused before `read_base`.
- **P6, missed:** no other file failed with the target outside.
- **P8, missed (overruled):** "its `ok` follows the gates that ran" was built, then ruled against (D2). A no-board run is never `pass`.
- **G1, missed:** a tier-2 test was added (D2), with a `ci/test-weights.tsv` row, and `TIER2_MIN` moved to 205.
- **G2, missed:** +912 net lines in `crates/` against `main` (+1042 −130), against +250 to +600. Most of it is `cli_backlog_tier1.rs` (478 lines).

## 5. Adversarial pass (§5), targeted

A fresh read-only subagent (Opus), given the work order and the diff at `c1eb5153`, not these conclusions. **It did not see what came after:** D1 (b)'s stderr banner, D2's `not_verified` and exit 7, `exit_code`, and MCP's one line. CI's tier-2 tests caught the one defect in that code (§5b); the rest is covered by its tests and mutants M18–M21, not by a fresh read. Its report is in this session; it could not write `adv/REPORT.md`. It found no high or medium defect in (1), (5) or (6). Each attack:
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

## 5b. A defect I introduced, caught by CI

- **What:** D2's first build (`1a8a4364`, pushed at `ea46983a`) computed the exit code in `outcome` *before* the fab step. `--against-fab` on a differing package sets `ok: false`, `verdict: fail` inside `package`, but the code stayed the earlier 0.
- **Caught by:** CI run 45103263052, tools shards 1 and 8. `against_fab_tier2::verify_against_fab_names_every_changed_and_extra_file` and `four_layer_fab_tier2::against_fab_names_a_missing_inner_layer_and_a_stackup_that_lies` both read `left: Some(0), right: Some(1)`.
- **Why nothing earlier caught it:** only those two tier-2 tests reach a pass that the package then fails. The host suite, the image runs above and the 23 mutants never do. `the_exit_code_is_the_verdicts_whatever_the_fab_mode` starts from a failing gate.
- **Fix (`5a0a1b33`):** `outcome` returns the verdict and reason only, and `exit_code(report.ok, report.verdict)` is read after the package, as `main`'s code read `report.ok`. The unit test asserts `exit_code(false, "fail") == 1`, and M21 covers `exit_code`'s `not_verified` arm. **On `ff210f1d` both tier-2 tests pass** (CI run 45107410136), as does D2's tier-2 test.

## 6. Findings, not fixed here

- **F-A (D1 (a)):** the strict topology reader that accepts and checks `lifecycle`, so the banner can be in `solve`'s verdict line and seen over MCP. The architect's row.
- **F-B (F5):** `generate hwdef`'s report carries no banner. One line in `hwdef.rs` (outside this fence) would add it, from the topology, as `check pinout` does.
- **F-C:** the KiCad files' banner (the WO's later work order). Measured here: the emitted files carry no `LIFECYCLE` line on any specification, and `cli_backlog_tier1` asserts it, so that work order will move that assertion.
- **F-D (F4):** `fab_tier1` under a symlinked `target/`.
- **F-E:** `doctor` still says an off-pin KiCad blocks `verify`, which a no-board `verify` no longer needs. True for every drawn board; very low.
- **F-F:** each banner site takes `lifecycle_banner().unwrap_or_default()`, as the existing sites do. It can only swallow a failure to parse the embedded parts table, which `validate` refuses first. Read only; kept for consistency.
- **F-G:** `cli_backlog_tier1`'s first-harness reader copies `verify::harnesses` (the bin-private parser). A test cannot reach it; reported, not removed.
- **F-H: Kani needs more than 16 GiB for `sentinel-02-bench-hse8`'s `every_valid_sequence_spi1`**, which plain `verify` on that stand-in now reaches (base refused it with `DrawsNoBoard`). Measured above. CI runs no `verify` of it, so no gate is affected; A caller who runs one needs more than 16 GiB, measured at a 16.5 GiB peak. The kickoff's 16 GiB rule for `verify` does not cover it.
- **MCP and exit 7, ruled 03:30Z:** a successful call, like 1. `mcp.rs` counts 7 with 0, 1 and 6 (one line), and its unit test is now `exit_0_1_6_and_7_are_successful_calls_and_others_are_errors`; the snapshot `tests/mcp-tools.json` does not move (`mcp_snapshot_tier1` passes). Mutant M20.
- **`TIER2_MIN`, ruled 03:30Z:** raised in `ci.yml` from 203 to **205**: `main`'s count, 204 (run 45099268629 at `d09a5187`; 2309 and 4407 add no `TIER2_RE` test), plus D2's. As ruled, one line: the value only. The comment history above it, which ends at 203, is not extended.

- **F-K: the shard refit rule fires** (`ci/shard.sh` REFIT RULE: a spread above 6 minutes on two runs in a row).
  - Tools-shard spread on this PR: 6.2 min on `f6372ddd` (run 45095787493), then 7.3 min on `ff210f1d` (45107410136). The slowest shard is 17m52s, under the 20-minute line.
  - `main`'s `b22600d3` (3307) was already 7.2 min, and `d09a5187` 3.1 min.
  - D2's 345 s row is one of the inputs. The refit is `ci/` work, outside this fence, and yours to schedule.

## 7. Gates, fence, reduction pass, mutants

- **Gates at `6ea27f87`** (D2's first build; the merged final code is under "`main` merged" below):
  - `cargo fmt --all -- --check`: clean;
  - `cargo clippy --workspace --all-targets -- -D warnings`: clean, and again for `aethelgard-cli` with `--features with-tools`;
  - `cargo test --workspace` (target in the repository, `TMPDIR` unset, as CI's host job): **2795 passed, 0 failed, 0 ignored**, 161 `test result` lines (`m/ws-d2.log`). At `8ff08248`, before D2, it was 2794.
  - The D2 tier-2 test passed alone in the image (`t2.log`). `ci/test-shard.sh` passes with its row (126 PASS).
- **CI on `ff210f1d` (PR #342, the final code): green.**
  - Runs: `ci` 45107410136, `engine-image` 45107410154 and `ardupilot-image` 45107410239. 33 checks pass; 3 are skipped by the workflow.
  - The tier-2 guard reports `tier2_tests_passed=205 floor=205`.
  - `with_tools::a_board_not_drawn_runs_its_gates_and_is_never_verified`, `against_fab_tier2::verify_against_fab_names_every_changed_and_extra_file` and `four_layer_fab_tier2::against_fab_names_a_missing_inner_layer_and_a_stackup_that_lies` pass.
- **CI on `ea46983a` (D2's first build): red**, three checks. Tools shards 1 and 8 failed, and `tools` with them: the defect in §5b.
- **CI on `f6372ddd` (PR #342, before D2): green.**
  - Runs: `ci` 45095787493, `engine-image` 45095787593 and `ardupilot-image` 45095787549. 33 checks pass; 3 are skipped by the workflow.
  - The tier-2 guard reports `tier2_tests_passed=204 floor=203`. At that commit no tier-2 test had been added; D2 later added one, and `TIER2_MIN` became 205.
  - **F2, settled:** CI's host `gates` job runs as the runner user, not root. There `cli_backlog_tier1::the_private_scratch_is_outside_home` and every `private_tmp` caller pass, so the runner has `/run/user/<uid>`.
- **Mutants** (`spike/1607/mutants.py`; 25, one per guard this work order adds or changes). Each ran with a fake `HOME` holding `.cargo/config.toml` and the target outside the tree.
  - **22 of 22 killed at `d2a5737a`, and at `8ff08248` after the reduction. 23 of 23 at `1a8a4364`. 25 of 25 at `ff210f1d`, the final code**, with M19–M21. Each was killed by its named test, and every selection passes on the pristine tree (`mut-final/result.txt`).
  - The groups: M1 (pin); M2 (scratch under `$HOME`, with M2b, its assertion off); M3 (`fab_tier1` scratch); M4, M5, M5b, M5c, M5d (`readme_status`); M6, M6b, M7, M8, M9 (`verify`); M10–M14 and M18 (each banner site); M15 (`doctor`); M16, M17 (the two runners' `TMPDIR`); M19 (a no-board run that passed mapped to `pass`); M20 (MCP's exit 7 an error); M21 (`exit_code`'s `not_verified` arm to 1).
- **Fence:** §3's files, plus the three out-of-list edits accepted at 01:15Z:
  - (a) `doctor.rs` `KICAD_PINNED`, its `doctor_tier1` assertion, and the CLI README;
  - (b) `tools_tier1.rs` and `generate_tier1.rs`, on `private_tmp`;
  - (c) `emit_tier1.rs`'s plain-`verify` row.
  - `mcp.rs`, one line, with its unit test, and `.github/workflows/ci.yml`'s `TIER2_MIN`, one line: both ruled 03:30Z.
  - D2's tier-2 test in `with_tools.rs`, with its `ci/test-weights.tsv` row, which §3 allows. It is a Furnace estimate: 123.33 s alone in the image, times 2.8, gives 345.324.
  - The CLI README's lifecycle and `verify` sentences, which name the new verdicts and `not_applicable`, are (a)'s file.
  - Not touched: `renode.rs`, `campaign.rs`, `ledger.rs` and `ledger_tier1.rs` (4300i's), `tests/fixtures/*.json`, the IR, eda, the generators, workflows and `ci/`.
- **`main` merged twice:**
  - `fa9d7edd` (docs only: `docs/roadmap.md` and a review), at `d2a5737a`.
  - `d09a5187` (3307's evolve change, its spike, and `docs/`), at `28662844`.
  - `15798f91` (2309's chaos change, 4407's images, and `docs/`), at `ff708b3a`. One conflict: `ci/test-weights.tsv`, where each side added a header line, and both are kept. On the merged tree: fmt and clippy clean, `cargo test --workspace` **2802 passed, 0 failed** (`m/ws-merged.log`), and `ci/test-shard.sh` 126 PASS. It touches nothing this branch changes, and merged clean. On the merged tree, `readme_status` (which reads the moved roadmap) and `aethelgard-evolve`'s 164 tests pass, and fmt and clippy are clean. The measurement stands: evolve is not in the sweep's commands, and its spike JSON is under `spike/`, which the sweep skips.
- **Reduction pass (diff): +912 net in `crates/` against `main` (+1042 −130; +807 at `8ff08248`, when the pass ran) · 3 duplicates removed (and one module loaded twice) · 0 dead paths removed · 0 protected kept · 1 finding (F-G).**
  - **Removed:** `doctor`'s `KICAD_PINNED`, equal to `KICAD` once (1) landed.
  - **Removed:** the banner-to-lines conversion, written twice in `cli_backlog_tier1` (now `banner_lines`).
  - **Removed:** `generate_tier1`'s re-derivation of `private_tmp`'s root and non-root mode, and its probe scratch.
  - **Also:** `tools_tier1`'s second load of `common/stub.rs` (clippy's `duplicate_mod`).
  - **Proof:** the suites stay green and all 22 mutants are still killed at `8ff08248`.
  - Run inline over the diff: the adversary had already read all of it.
