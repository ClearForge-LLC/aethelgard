# AET-WO-1918 — FEEDBACK: CI headroom (the prove peak, the wall bound, the refit)

**Branch:** `wo/aet-1918-ci-headroom`. **PR:** #401. **Base:** `fd12b2f8`.
**Builder:** Claude (Opus 5.5) as project manager. It used three subagents:
- the wall-bound survey (Sonnet 5.5, read-only);
- the adversarial pass (Opus, read-only: judgement);
- the reduction pass (§6).

Each subagent's report was spot-checked at source before use.

## 0. How it was made

- **Measure, stop, build.**
  - Predictions were committed first (`0ebdc43f`), then the measurement in `spike/1918/results.md`.
  - The stop was posted 2026-11-25 at about 03:45Z, and ruled at 04:01Z: (1) (S) at 80 statements, (2) (b).
  - (3) was stopped once more at the build, on its floors, and ruled at 06:37Z: (i) and (ii), with (iii) measured and not adopted, and (iv) refused.
- **The diagnostic CI head `9c06e112`** carried the probe and `lscpu` in every job. Its run 45528571905 was re-run by the architect for the five runs (2) asked for.
- **Corrections the advisor made before the stop:**
  - a wall-time claim for (S) that compared two windows (withdrawn: back to back it is +8 %);
  - the miscount of the unreached sites (16, all traced, §4);
  - the §7 condition (S) meets;
  - and that `--rebind` does not apply to Sentinel-03.

## 1. (1) The prove peak

- **Measured.** Sentinel-03's `every_valid_sequence_tim1` peaked at 15.09 GiB (`memory.peak`, Furnace) and at 14.78 GiB of kani-driver VmHWM in CI.
  - It was the only harness over 12 GiB. `spi1` and `spi2` were at 11.3–11.5 GiB, and Sentinel-01's heaviest at 3.53 GiB.
- **The cause is Kani's default assertion reach checks.** CBMC handed kani-driver 4.12 GiB of JSON for `tim1`, and 97 % of it was 449 traces, one per reachable assertion of `check_final_tim1`, each a whole 90-step schedule (`results.md` §1.2).
- **Built, (S):** `aethelgard-svd-hal` `board.rs` `split_final`, with `FINAL_STATEMENTS_MAX` = 80.
  - A schedule part whose final check holds more than 80 top-level statements is proved in parts.
  - Each part is the whole harness, the same schedule, unwind, covers and reach checks, calling a contiguous run of the check.
  - A binding is glued to the check that uses it. The parts' statements together are the check's, each once.
  - A whole-schedule board (no parts) is never split.
- **The four crates that move** (ruled: named here), with every JSON under `examples/`, `templates/` and the fixtures generated (`spike/1918/tools/count-finals.sh`):
  - `examples/sentinel-03.json`: 20 harnesses become 26.
    - Its package's `verdict_sha256` moves, and nothing else in it does. The ELF is `6e4a81f2…`, unchanged.
    - The architect's `write` dispatch, run 45561825388, rewrote it.
  - `examples/sentinel-02-bench-hse8.json`: its split `spi1` parts prove at 7.53 and 7.26 GiB, and `i2c1` at 3.86 and 4.42, each at 16G. Unsplit, `1915` §4.1a had it OOM-killed at 16G. Recorded, not chased (ruled).
  - The fixtures `crates/aethelgard-cli/tests/fixtures/hse-crystal-16.json` and `hse-external-8.json`. No test names a harness on these four boards.
  - `boards/sentinel-01*` do not move (41–52 statements).
- **Measured as built.** Every split is exact, and the rest of `proofs.rs` is byte for byte the old.
  - Sentinel-03's whole `verify --fab-out` in the image passes all 26 prove gates and peaks at 9.24 GiB for the whole scope.
  - CI's own peaks: §5.
- **The CI ceiling:**
  - `ci/kani-ceiling` = 12 GiB.
  - `ci/probe.py` reads kani-driver and cbmc VmHWM at 0.2 s.
  - `ci/ceiling.sh` fails a job above the ceiling, or with no log. The bench jobs require a peak.
  - Run in `tools-shard`, `bench-package` and `bench-package-s03`.
  - `ci/test-ceiling.sh` plants a heavy harness, tests the probe on planted processes, and pins the three jobs.
- **(R), `--no-assertion-reach-checks`:** 1.81 GiB and 21 s for `tim1`, all 20 harnesses passing. Recorded, not built (ruled).

## 2. (2) The wall bound

- **Built, (b):** `renode::BOOT_BOUNDS.wall` = 120 s, a hang guard.
  - The worst boot measured over eight CI runs was 23.6 s (the survey, `~/.cache/aet-1918/wall-survey.md`), and 20.8 s on this branch's diagnostic runs, an EPYC 7763: about 5x headroom.
  - **A plain kill at it is `renode::hung`, a `ToolError`.** That holds at the boot, at the droop's undisturbed boot (`undisturbed_boot_word_ns`), and at a foreign run whose hook caught a fault.
  - `cpu_aborted` and the memory cap stay findings: each is the firmware's, logged in virtual time.
  - Replay refuses `wall_bound_exceeded` on a read or an unread boot (`aethelgard_chaos::campaign::HANG_GUARD_FINDING`).
- **The judged bound stays virtual** (`MAX_VIRTUAL_NS`, 100 ms).
  - **This supersedes `RUL-13`'s 2026-11-08 "30 s wall bound"**, to be amended in the docs pass after the merge (ruled).
  - The crate READMEs are updated here.
- **The test's guard:** `simulate --wall-bound-ms`, hidden, 1 to 120,000.
  - **It cannot reach a judged verdict:**
    - a wall kill writes no verdict, so a shorter guard can only end a run sooner;
    - the verdict records it as `wall_bound_ms`;
    - `campaign` sets `BOOT_BOUNDS` in `Boot.bounds`, and `verify` spawns `simulate` without it;
    - the MCP tools skip hidden arguments.
  - `boot_tier1` and `twin_tier2`'s A6 test use it (2 s and 30 s).
- **Planted:** `campaign_tier1::a_renode_that_never_returns_is_refused_and_archives_nothing` (122 s, the guard itself), and `renode`'s `an_undisturbed_boot_killed_at_the_hang_guard_is_refused`.
- **Tests whose assertions changed by the ruling:**
  - `chaos/tests/campaign.rs`: `wall_bound_exceeded` is refused, not accepted;
  - `chaos/tests/droop.rs`: a killed droop boot is planted at the memory cap;
  - `boot_tier1`: the wall test is now a refusal;
  - `twin_tier2`: A6 under the test's guard, and `m2`'s list.

## 3. (3) The refit

- **Measured before (the stop):** 3 runs of the 12-shard plan, every shard planned at 994 s.
  - The test steps ran 648–2,072 s.
  - Setup (image pull, artifact, unpack) took 127–411 s.
  - The floors: `verify_2k2_tier2`'s block, up to 1,683 s, and `against_fab_tier2`'s tests, up to 918 s each, every one recomputing the shared `--fab-out` in its own shard.
- **Built, (ii)** (the fence widened to two test files):
  - **`verify_2k2_tier2`:** each pair's two verifies run at once, one pair at a time (`PAIR`), so at most two verifies run together, under the ceiling.
    - It is decolocated: its two tests have read separate runs since `4300k` (T).
    - Measured in CI (run 45572067977): the package test 529 s, the reference test 376 s, against up to 1,683 s.
  - **`against_fab_tier2`:** the `--fab-out` package is written once per binary.
    - The binary is colocated, held by `fab_tier1::against_fab_tier2_writes_its_one_package_once_per_binary`.
    - Its four verifies run in two lanes (`LANES`), at most two at once. That is my addition, so the shard is not five verifies in a row.
    - **The block is 1,715 s (run 45572067977, EPYC 9V74): a floor, over 20 minutes.** Its four tests, each in its own shard, had taken up to 918 s. Writing the package once costs one shard's wall time.
    - The only lever left is more lanes, which the 12 GiB ceiling forbids (about 3.5 GiB per verify).
- **Built, (i):** `spike/1918/tools/refit.py`.
  - **439 tests** at their worst in-shard sample (REFIT RULE 5) over seven runs:
    - 45528571905, attempts 1–4;
    - 45539991165;
    - 45561825388, the write dispatch;
    - the throwaway, 45572067977.
  - **The 122 tests of the locked binaries** (`board_tier2`, `twin_tier2`) at their alone time, from the throwaway's alone pass (REFIT RULE 1–2).
  - **10 blocks** at their worst `finished in` (REFIT RULE 3), for example:
    - `via_ring_tier1` 61 → 367 s;
    - `campaign_tier2` 98 → 266 s;
    - `outline_tier1` 251 → 400 s;
    - `verify_47k_tier2` 278 → 431 s;
    - `four_layer_fab_tier2` 741 → 931 s.
  - A test or block this WO changed is weighed from the throwaway run only.
  - Every decision is in `spike/1918/out/refit.tsv`.
- **The plan:** `ci/SHARDS` 12 → 20.
  - **The cap:** 20 minutes less the worst setup measured (411 s) and the after-steps leaves 783 s for the test step.
    - At 18 shards the largest unit is 740 s; at 20 it is 703 s, which is 18.7 min with the worst setup.
  - **The plan at 20** (`ci/shard.sh plan`, guard ok, 3,429 test names):
    - shard 0, `against_fab_tier2`, 1,715 s: **a floor**;
    - shard 1, `four_layer_fab_tier2`, 931 s: **a floor**. It is one block (`3303b`'s split already applied), so it can only get cheaper, never split;
    - shard 2, 703 s;
    - 17 shards at 655 s.
  - **The account's concurrent-job ceiling** (about 60, `ci/shard.sh` rule 4): one pull request's run is 20 shards plus about 6 other jobs. Two at once queue.
- **The final head's measured per-shard table:** in the READY post, on the bus.
- **(iii), `opt-level=1`, measured, not adopted (ruled).** Throwaway run 45572067977 had `tools-build-o1`, `tools-shard-o1` and `bench-package-o1`.
  - Shard by shard, the normal pass ran 0.71–1.50× the opt-level-0 time, on runners that differed.
  - The one pair on the same CPU (shard 8, EPYC 7763 both) is 1.01×. Summed, it is −3 %, inside runner noise (`spike/1918/out/o1-compare.tsv`).
  - The heavy work is Kani, Renode and KiCad, and `aethelgard-eda`, `svd-hal` and `sha2` are already built at `opt-level=2`.
  - **No verdict byte moved at opt-level 1:** `bench-package-o1`'s `--against-fab` of Sentinel-01's committed package passed, and every test in the 12 `-o1` shards passed.
- **(iv), the clause less setup: refused (ruled).** The floors above are named instead.
- **The throwaway** (`8d9fca24`) is reverted in `cbaf4fb1`.

## 4. Rows for the roadmap

- **Reach checks as a judge (ruled a row).** Board-wide, each generated assertion reached by some harness, guards excepted. The generator must name its guards.
  - **Measured on Sentinel-03** (the 25-harness pass): 1,492 assertion sites of ours.
    - 1,439 are reached by some harness.
    - 37 are `kani::assert(false, …)` guards.
    - **16 are reached by no harness. All 16 were traced, and none is vacuous:**
      - 8 SPE ordering checks: those in `Image::write` are dead, because transfers write SPE through `Spi1Spy` and `Spi2Spy`, whose copies are reached and SUCCESS; those in the part images are dead, because `init` leaves SPE clear;
      - 5 ODR checks, after a BSRR guard in the same branch;
      - `Tim1 accessed before RCC_APB2ENR bit 0 is set` (`Image::clocked`), proven in `ImageTim1::clocked`;
      - `a PWR register other than CR3 and D3CR was written` (`Image::write`), proven in every part image;
      - `a GpioC MODER write moved a pin the topology does not use` (`ImageGpio::write`), proven at `Image::write` in `usart3_start_writes_the_solved_values`.
  - Per harness, 21 of the 25 have unreachable assertions of ours by design, so a per-harness judge would be wrong.
- **`BRIDGE_BOUNDS`' 30 s.** The ngspice bridge alone keeps 30 s. A kill there was already a `ToolError`, so it opens no `INV-04` hole. Furnace's bridge runs took median 3–4 s and max 10 s, and CI's are unprobed.
- **`--wall-bound-ms` on the verdict.** A run under it records `wall_bound_ms`, but a campaign archive has no field for it, by design: a campaign cannot set it.
- **The `opt-level=1` test profile:** §3, measured, not adopted (ruled).
- **The `4411` module board:** not measured. Its branch holds no specification yet, and its schedule parts will split by the same rule.
- **The two floor shards (§3):**
  - `against_fab_tier2`, 1,715 s: its four verifies after the one `--fab-out`, in two lanes;
  - `four_layer_fab_tier2`, 931 s.
  - Each would need its verifies made cheaper, or fewer. That is a test-shape question for a later CI work order.
- **`RENODE_TURN` (`twin_tier2`) and `FLOOD_ALONE` (`campaign_tier2`)** were made only because parallel boots hit the 30 s bound, now a 120 s hang guard.
  - Removing either changes those binaries' timing and weights, so it needs CI timing first.
  - Found by the reduction pass; not removed.
- **Twenty shards against the account's concurrent-job ceiling** (about 60): two pull requests' runs at once queue. Watch it if queueing shows.

## 5. Gates, CI, the adversarial pass

- **The adversarial pass (Opus, read-only, `~/.cache/aet-1918/adversarial.md`): 2 blocking and 8 should-fix findings, all built.**
  - **B1:** the hidden flag was offered over MCP. Hidden arguments are now skipped, held by `simulate_keeps_its_name…`.
  - **B2:** the removed wall test's weight row would have failed every shard's plan; dropped, and the 120 s test weighed.
  - **S1:** a foreign fault beside a wall kill is refused.
  - **S2:** the split's statements are tested on every shape the generator writes.
  - **S3, S4:** the probe watches cbmc, writes what is alive at its stop, and is tested on planted processes.
  - **S5:** crate docs.
  - **S6:** 26 harnesses are recorded.
  - **S7:** replay refuses `wall_bound_exceeded` on a read boot too.
  - **S8:** the undisturbed boot's guard is unit-tested.
- **Found nothing:**
  - no assertion dropped, duplicated or misassigned by the split;
  - the covers and unwind kept in every copy;
  - the renames safe;
  - the flag unable to reach `verify` or `campaign`;
  - `if: always()` unable to pass a failed job;
  - the ceiling failing closed.
- **Mutations: 13 of 13 killed** (`spike/1918/tools/mutants.py`, `spike/1918/out/mutants.txt`). One mutant per guard this WO adds or changes, against its suite:
  - the split's bound and its binding glue;
  - the wall kill as a refusal, at the boot and at the undisturbed boot;
  - replay's refusal;
  - the MCP skip;
  - the flag on the verdict, and the flag honoured;
  - the ceiling's comparison, both ways;
  - the probe's flush at its stop;
  - a bench job's required peak;
  - `against_fab_tier2`'s colocation.
  - One first-sweep mutant hit only a printed label (equivalent) and was moved to the decision.
- **Gates on the final head (`main` `4e5b0d3b` merged):**
  - `cargo fmt --all -- --check` passes;
  - `cargo clippy --workspace --all-targets -- -D warnings` is clean, and so is `-p aethelgard-cli --features with-tools`;
  - `cargo test --workspace`: 3,194 passed, 0 failed;
  - `ci/test-ceiling.sh`, `test-shard.sh`, `test-route.sh` and the inventory guard pass.
- **CI:**
  - the final head's run is in the READY post;
  - the throwaway run 45572067977 was green on every job, including both bench jobs against the committed packages.

## 6. Reduction pass

Reduction pass (diff): −22 net in `crates`, `ci` and `.github` · 4 duplicates removed · 2 dead paths removed · 3 protected kept · 3 findings in §4.
- **The duplicates removed:**
  - `renode::bounded` and `hung` are one `Result`, the wall-kill rule stated once;
  - three copies of the probe and ceiling steps became `ci/probe-step.sh`;
  - the flag's 120,000 is derived from `BOOT_BOUNDS`;
  - the test helpers take extra arguments, where two had rebuilt `simulate`'s arguments.
- **The dead paths removed:** `Ready.bounds`, and the probe's unused `--boot-comm` and `Name`.
- **Kept as deliberate, unlabelled, for the owner:**
  - `test-ceiling.sh`'s pin of the ceiling's number;
  - `split_final`'s three shape asserts;
  - `check_run`'s refusal of `HANG_GUARD_FINDING` beside the unread count.
- **The findings** (`RENODE_TURN`, `FLOOD_ALONE`; the protected `RUL-13` and the roadmap's 30 s lines) are in §4, and in §2 for `RUL-13`.
- **The pass** was a fresh read-only subagent (`~/.cache/aet-1918/reduction.md`), and every proposal was spot-checked before it was applied.
