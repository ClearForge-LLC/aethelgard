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

[the throwaway run and the final plan: filled when measured]

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

## 5. Gates, CI, the adversarial pass

[filled at READY]

## 6. Reduction pass

[filled at READY]
