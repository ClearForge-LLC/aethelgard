# AET-WO-1919 — FEEDBACK: CI headroom (2)

**Branch:** `wo/aet-1919-ci-headroom-2`. **PR:** #405. **Base:** `main` `649e81a8`.
**Builder:** Claude (Opus 5.5) as project manager. It used one subagent: the targeted adversarial pass (Opus, read-only, because the work is judgement). Its findings were checked at source before use (§5).

## 0. How it was made

- **Measure, stop, build.**
  - Predictions were committed first (`f6ba23c0`).
  - Then the probe that times every gate was pushed (`4254a2a0`), so that CI's own logs would hold the profile.
  - The stop was posted on the bus on 2026-11-26 at 04:08Z, with an addendum at 04:42Z. It was ruled at 04:51Z: (1) (G)+(X), (2a)–(2e), (2c) granted, (3a)–(3c).
- **The record:** `spike/1919/results.md`. The architect re-ran `4254a2a0` twice, so the table has three CI runs of one head.
- **Pushed three times after the stop:** `4752028a` (the build, after the third run, as ruled), `d507f8fb` (the 06:49Z and 07:01Z findings, one push as ruled), `2efdee9c` (the 08:49Z finding, the tests' observation). This FEEDBACK's final text is a local commit after `2efdee9c`, not pushed (ruled 09:38Z: no further push; the architect carries it in the docs PR).

## 1. (1) `bench-package-s03`

- **Measured.**
  - `verify` is 96–97 % of the job.
  - Inside it, CI's 26 prove gates took 63.2 min, and **30.2 min of that was the CLI's own work, outside Kani:** 69 s per gate.
  - `prove --spec` ran `generate::tree` (the schematic, placement and the 8-rung routing ladder, one thread per rung) and kept only `board/`. On a 4-vCPU runner that costs 73 s, against 17.8 s on Furnace's 20 cores.
  - So a `verify` generated the tree 27 times.
  - CBMC is 31.6 min. Compiling is under 1.2 s per harness. A release CLI saves 8 %: the time is in `aethelgard-eda`, already at opt-level 2.
- **Built, (G):** `generate::board_tree`, which is the `board/` files alone. It shares one `board_files` with `tree`, so its bytes are `tree`'s by construction (`board_tree_is_tree_s_board`). `prove --spec` uses it.
  - **The ruled behaviour change, named in the CLI README:** a specification whose crate generates but whose board does not route is refused by `generate` and `verify`. `prove --spec` judges its crate `generated` (`prove_spec_judges_the_crate_of_a_specification_whose_board_does_not_route`).
  - **One further change, from the adversarial pass:** a specification whose emission and crate both fail is now refused with the board error's kind, not the emission error's.
- **Built, (X):** `verify` runs two lanes ahead of their turn (`verify.rs` `Prefetch`, `lane`, `start_lanes`):
  - the prove gates;
  - beside them, `extract` (on a drawn board), then `build`, then `simulate` when `build` wrote an ELF, then the campaign when there are scenarios.
  - **Each lane's processes are captured whole and judged by today's code, in today's order,** through `gate()`. So the report, every binding and the first refusal are today's whatever finishes first. stdout carries one line, and each gate's stderr is written whole in its turn.
  - A lane stops at a gate that refuses. `finish` plans a lane's next gate under the same lock, so `gate()` never misses one. A gate no lane planned runs in its turn.
  - Planted:
    - `of_two_refusals_in_two_lanes_the_earlier_in_today_s_order_is_reported`: `build` refuses at once, the first prove gate after 3 s, and the prove refusal is the one reported;
    - `the_prove_lane_and_build_s_lane_run_beside_each_other`: measured against the host's own prove-lane time.
- **No byte moved.** The built head's whole Sentinel-03 verify on Furnace, pinned to 4 cores, gave a verdict line byte-identical to the baseline's (`out/furnace-built-s03.meta`, sha256 `5ba5f66b…`, all 26 prove gates). CI's `--against-fab` on both bench packages: §8.
- **What a stop does to the lanes (the architect's 06:49Z finding).**
  - **Measured first.** A planted early refusal (the first prove gate refuses while `build`'s lane runs a 30 s stand-in). On `4752028a` the lane's gate, its reaper and the stand-in outlived `verify` by 1.1–1.2 s, three runs. The parent-death signal each gate arms (`PARENT_PID_VAR`) killed them when the lane's thread ended with the process; mutating it away left them running.
  - **Fixed, `stop_lanes`, on every path out of the gates:**
    - no lane starts another gate;
    - every descendant of `verify` is killed, the tools under each gate's reaper included. The tools are in the reaper's own process group, so the gate's group would not reach them;
    - each killed gate's scratch guardian is waited for, never killed, so its directory is gone;
    - `verify`'s own guardian is spared;
    - the lane threads are joined.
  - `verify` is a child subreaper from the lanes' start, so nothing a killed gate leaves goes to init.
  - **After the fix:** nothing is left one `/proc` scan (about 4 ms) after `verify` exits, and the call's scratch is empty and stays so (`a_verify_that_stops_early_leaves_no_lane_gate_and_no_late_file`).
  - **A `verify` killed outright** (SIGKILL, so `stop_lanes` never runs) still takes its lanes with it in 1.15 s, through the same parent-death signal (`a_verify_killed_outright_leaves_no_lane_gate`).
- **A file changed after a gate ran ahead (the architect's 07:01Z finding; CI run 45770483030 shard 13).**
  - `board_tier2::verify_refuses_a_board_swapped_before_extract` passed a swapped board on `4752028a`. `extract` had read the board ahead of its turn, so its own binding matched, and nothing read the board again.
  - **Fixed, two checks against the generated hashes** (`EXPECTED`: every generated file, the specification's copy, the ELF once `build` wrote it, each scenario file and each resolved base):
    - **at its turn,** a gate that ran ahead has the files on its argv hashed again (`recheck`). A change is `BindingMismatch` naming that gate, as when it read the changed file in its turn. So the kept test still gets `extract`, unchanged, timing included.
    - **after the last gate,** before anything is written to `--fab-out`, every file is hashed again (`recheck_all`). A change is `BindingMismatch` naming the last gate in today's order that judged the file: the board's is `extract`, the crate's `build`, the ELF's the campaign (or `simulate`), a scenario's or base's the campaign.
  - **Why that `id`:** the gate named is the one whose judgement the change voids, and the board's is `extract` whichever check sees it first.
  - **Before (X), a change after a gate's turn was never seen.** The end check now refuses it too.
  - **Planted:**
    - `board_tier2::verify_refuses_a_board_swapped_after_extract_ran_ahead`: the swap lands 0.5 s after the support gate's process has ended, so after `extract`'s turn. Five passes in the image, 173–186 s each.
    - `verify::tests::a_gate_run_ahead_is_refused_at_its_turn_if_its_file_changed`, through the real `gate()`;
    - `a_file_changed_after_every_turn_is_refused_at_the_end`.
  - **What else (X) moved:** every lane gate's binding: `extract` (the board and the specification's copy), each prove gate and `build` (the crate), `simulate` (the platform, the ELF, the specification's copy), and the campaign (the ELF, the scenarios, their base). All are in `EXPECTED`, so each is re-checked at its turn when it is on the gate's argv, and at the end. The base is on no argv, so only the end check sees it, naming the campaign; a refusal between the campaign's turn and the end (an `attribute` gate's) would then come first. The scenario files and the specification are the caller's own, read once before any gate, as before.
- **The combined peak (ruled condition):**
  - Furnace (the probe inside the container): 8.53 GiB of watched-process RSS, at a `tim1` part after the campaign had ended. The container's whole `memory.peak` was 9.68 GiB.
  - CI: `PROBE combined` and `PROBE host` in the bench job's log (§8).
  - Both are under `1918`'s 12 GiB, so the campaign does not wait for the prove lane.
- **Projected and measured job time:** §8 and `results.md` §1.

## 2. (2) Every Renode the CLI starts, under the hang guard

- **Measured:**
  - the slowest CLI load in seven CI runs is 5.1 s, and the slowest whole `simulate --platform` call 4.7 s;
  - on a loaded Furnace, the slowest is 7.35 s (Sentinel-03, its two models compiled);
  - **no load is within 5× of 120 s.** The two loads that hung in run 45709090931 took 3,111 and 2,852 s.
- **Built:**
  - `renode::LOAD_BOUNDS` = { the boot's 120 s, `rss_kib: u64::MAX` }: the guard alone, with no memory cap (2a);
  - a load runs under it, and so do `simulate`'s `renode --version` (2b) and `doctor`'s (2c, granted);
  - a kill is `renode::hung`, a `ToolError` (exit 5) naming the load or the `--version`, never a finding or a verdict (2d);
  - the hidden `--wall-bound-ms` is admitted on a load, absent from the MCP tools, and recorded as `wall_bound_ms` (2e).
- **Planted:**
  - `a_load_waiting_at_its_prompt_is_refused_at_the_hang_guard`;
  - `a_renode_version_that_never_returns_is_refused_at_the_hang_guard`;
  - `a_load_s_test_guard_is_on_its_verdict`;
  - `a_load_takes_the_boot_s_hang_guard_and_no_memory_cap`;
  - `doctor_tier1::a_renode_that_never_says_its_version_is_killed_at_the_hang_guard`. `doctor` takes no test guard, so this one waits the 120 s out (weighted 125 s).
  - Each stand-in gives up by itself, so a CLI with no guard fails the test's time bound rather than hanging it.
- **No load test sets a timeout of its own,** and neither does the MCP server. Before this work order, the shard job's 60 minutes ended a hung load.
- **The draft `RUL-13` amendment (the architect lands it):**

  > **Amended 2026-11-26 (`AET-WO-1919` (2), ruled on PR #405):** every Renode the CLI starts is bounded. A platform load (`simulate --platform`, and the MCP `simulate` tool, which runs it) and every `renode --version` (`simulate`'s, before each load and boot, and `doctor`'s) take the boot's 120 s wall-clock hang guard, with no memory cap. The slowest load measured was 5.1 s over seven CI runs and 7.35 s on a loaded Furnace. A kill there is a `ToolError`, never a finding or a verdict. `simulate --wall-bound-ms` shortens it for tests and is recorded in the verdict. `prove` and `check` stay unbounded, and the caller's timeout ends them.

## 3. (3) An empty input

- **Searched by parsing:** every committed JSON, every `input_file` call, every `check_input` call, and archives. **Nothing relies on an empty input.** The one literal (`boot_tier1.rs:3841`) is refused for `input_version: 2` first, and it still is.
- **Built:** `foreign::frames_present`, one check that the reader (`frames_of`, after the version check, 3a) and replay (3b) both call. The refusal is `InputMalformed`, exit 2, `id: "frames"`.
- **Planted:** `an_input_with_no_frame_is_refused_by_its_field_and_in_replay`, at the reader and on a crafted archive.
- **(3c), ruled:** a heartbeats-only input stays accepted. A frame with nothing to send cannot parse (`parse_exact` needs a header and a CRC).

## 4. The probe's self-test flake (05:05Z note; CI run 45743010585, attempt 2)

- **Cause: the planted driver became `sleep` the moment its growth ended.** The planted `kani-driver` ran `x=$(…60 MB…); sleep 1`. bash execs the last command of a `-c` string, so the process's `comm` turned to `sleep` when the growth finished. The probe then kept only what it had sampled mid-growth.
- **Shown on Furnace, 10 runs each:**
  - the old line read 44.8–62.7 MB (one under the test's 50,000 KiB);
  - with a trailing `:`, every run read the true peak, about 121 MB.
- The second, small line is the `$(…)` subshell, a fork of the same driver.
- **The race predates `4254a2a0`** (the planted line is `1918`'s). Its timing on a runner decides whether it bites.
- **Fixed by construction:** the trailing `:` keeps the driver alive as itself. The assertion is on the highest line.
- **The adversarial pass found the same class in my own gate-time assertion (F4)**, which required `seconds=1.x` for a 1 s process. The planted gate now lives 1.5 s, and the assertion allows one poll period either side.

## 5. Adversarial pass (targeted; Opus, read-only)

- **Lens 1, a proof that stops running:** no defect in (X)'s matching, waiting, stopping or refusal order, and (G) is byte-safe.
  - **F1, fixed:** the README's "A `--spec` that `solve` or `generate` refuses is refused as they refuse it" contradicted (G). Reworded.
  - **F2, fixed:** the combined sum also counts `kicad-cli`, `cargo`, `rustc` and the bridge. CI's figure to read is `PROBE host`.
  - **F3, fixed:** a lane whose thread cannot start now withdraws its plan (`Prefetch::unplan`), so `gate()` runs the gate in its turn instead of waiting forever or panicking.
  - **Kept, diagnostics only:** a gate's stderr is now held until its turn, so a hung prove shows none while it runs.
- **Lens 2, a bound that judges the host:** no defect.
  - `u64::MAX` round-trips through the reaper's `number()` and never trips.
  - A load's verdict gains no field except `wall_bound_ms` under a test guard.
  - Nothing that runs Renode is left unbounded.
  - Noted: campaign boots now run beside CBMC. They were measured serially, but a slow one is still a `ToolError`, not a verdict.
- **Lens 3, an empty input by another name:** nothing beyond what was ruled.
- **What the pass missed:** it cleared the lanes' paths on a normal run. It missed what a stop leaves running (06:49Z) and a swap after a gate ran ahead (07:01Z); the architect found both (§1).

## 6. Mutants

`spike/1919/mutants/run.py`, one per guard this work order adds, each against its test, counted killed only when the test ran and failed: **21/21 killed** (`out.txt`). That counts six for the two findings: the lanes not stopped, a gate's guardian killed, the parent-death signal, the turn check, the end check blinded, and the end check removed (run in the image against the tier-2 test). The probe's two mutants, run by hand, were killed too.

## 7. Gates

- `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace` at `3cb73690`: 3,223 passed, 0 failed (3,219 at `72b51e0e`). After it, only the two lane-process tests changed, and they were run on the host and in the image as root (F10).
- Mutants 21/21 at the final head (`spike/1919/mutants/out.txt`).
- `ci/test-ceiling.sh` and `ci/test-shard.sh`: pass.
- **Reduction pass (diff), before the two findings:** −7 net · 1 duplicate removed (the `wall_bound_ms` line for a load and a boot) · 1 wrapper inlined (`simulate_load`) · 0 protected kept · 1 finding below. The lane stop and the re-checks (about 350 lines with their tests) had no reduction pass of their own.
- **New tests' shard weights** (predicted, to be replaced at the next refit): `doctor_tier1`'s guard test 125 s; the two load-guard tests 3 s each; the `board_tree` unit test 3 s; `cli_backlog_tier1`'s block 32 → 62 s.

## 8. CI

**`2efdee9c`, the final head:** ci 45788933494 (26/26 jobs), ardupilot-image 45788933242, engine-image 45788933264, all success.
- **`bench-package-s03`: 28.0 min** on an Intel Xeon 6975P-C: pull 0.5, build 1.3, `verify` 26.0. Inside `verify`, the 26 prove gates took 1,502 s (CBMC 1,434) and the CLI's own time 6 s.
- **Peaks:** `PROBE combined peak_rss_kib=8535320` (8.14 GiB) and `PROBE host peak_used_kib=9438552` (9.00 GiB), under `1918`'s 12 GiB.
- **`--against-fab`:** `identical` on Sentinel-03 and on Sentinel-01's bench board (6.2 min).
- **Every gate and harness, by name:** the verify verdict line in both bench jobs is identical to `4254a2a0`'s (main's `verify`, run 45743010585): Sentinel-03 40 gates (26 prove), Sentinel-01 21 gates (12 prove).
- **Tier 2:** 209 passed against `TIER2_MIN` 205; 3,466 tests passed across the shards; the shards took 9.3–19.3 min.
- **Boots beside CBMC:** the longest Sentinel-03 boot was 7.5 s, and the longest whole CLI load call 3.5 s.

**Measured on the branch's three built heads,** against 60–76 min before:

| head | CPU | `bench-package-s03` |
|---|---|---|
| `4752028a` | EPYC 7763 | 33.6 min |
| `d507f8fb` | EPYC 9V74 | 32.2 min |
| `2efdee9c` | Xeon 6975P-C | 28.0 min |

`crates/*/src` is unchanged from `d507f8fb` to `2efdee9c`. No built head ran on a Xeon 8573C, the slowest runner measured (76.2 min). For it, about 35.5 min is a projection, not a measurement.

## 9. Rows for the architect

| # | Finding | Lean |
|---|---|---|
| F1 | **A Furnace `--against-fab` of either bench package exits 1 by construction.** Rootless podman is not the initial user namespace, so every gate says `uid_boundary: false` and `verdict_sha256` moves. Only CI can re-derive a committed package | a row: say so where a builder would try it (the CLI README's `--against-fab`), or measure under a root container |
| F2 | **The shards' weights are now high for every tier-2 test that runs `verify` or `prove --spec`:** about 68 s saved per prove gate in CI | the next `times.tsv` refit, from this head's runs |
| F3 | **`extract` takes 265–531 s in CI and 428–505 s on Furnace,** in-process eda at opt-level 2. CBMC ran faster on Furnace than in CI in the same verify, so extract's Furnace time is unexplained | a later eda row, if `extract` becomes the long lane |
| F4 | **The `--version` probe and the boot share one `--wall-bound-ms`.** `twin_tier2.rs:593`'s 30 s test guard also bounds its `--version`, which takes at most 1.0 s in CI | none: noted |
| F5 | **Reduction finding, not cut:** `renode::bounded`'s wall message and `renode::hung` are two texts for one kind of kill (the firmware's run against a load or `--version`) | merge them at a later pass if the boot's message may move |
| F6 | **(M), two matrix prove jobs, is held in reserve.** After (G)+(X), CBMC is the long lane, about 32 min on the slowest runner | when `4411`'s `uart4` harness or another board takes the room |
| F7 | **Process:** I edited my own line in `architect.md` once, after appending it, to put in a commit SHA. Next time I will append a correction instead | none |
| F8 | **Process:** three runs of the new swap test failed in the image on a stale binary. A mutant run had built `verify` with no end check, then restored and touched the file. My next `rsync -a` copied the branch's older mtime back over it, so cargo kept the mutant's binary. Every run since touches the synced sources: 5 of 5 pass. The lesson matches `Mutant restore needs touch` | none |
| F9 | **A gate's stderr is held until its turn,** so a hung prove shows none while it runs | none: diagnostics only |
| F10 | **The two lane-process tests failed in CI (the 08:49Z note; run 45784116105, shard 13); `verify` was right, the tests' observation was wrong.** They found a lane's processes by their environment. In CI the tests run as root, so the CLI runs each tool under uid 65534, which leaves it non-dumpable. A container's root has no `CAP_SYS_PTRACE` to read such a process's `environ`; its `cmdline` stays readable. Shown: (1) CI's own probe logged both tests' lane gates at 39.3 s, and the early-stop test's `build` gate lived 2.2 s, killed when the prove gate refused at 2 s, though its stand-in sleeps 30 s: `verify` killed its lane in CI; (2) in the pinned image as root, a non-dumpable process's `environ` is "Permission denied" (`CapEff` `0x800405fb`) and its `cmdline` readable; (3) `d507f8fb`'s tests fail there exactly as in CI (`:618`, `:694`), and this head's pass twice (`out/ci-repro-image-root.txt`, `tools/ci-repro.sh`). **Fixed in the tests:** a process is matched by its command line, which names the toolbox, the call's scratch or a stand-in's marker unique to the run (a fixed one also matched my own shell's command line). The call's own guardian, which outlives `verify` by design, is left out. The same two properties are tested, and their three mutants are killed again | none |
| F11 | **`TIER2_MIN` is 205 while 209 tier-2 tests pass.** Main's code had 208, and this WO adds one | the architect: raise it to 209 in the docs PR |
| F12 | **New tests' weights, predicted against measured (45788933494):** `verify_refuses_a_board_swapped_after_extract_ran_ahead` predicted 300 s, measured about 194 s; `doctor_tier1`'s guard test predicted 125 s, measured about 121 s | replace at the next refit, with F2 |
| F13 | **`stop_lanes`, low:** it signals pids from a `/proc` snapshot, so a pid a lane thread reaped in between could be reused on a busy host (a pidfd per gate would close it). A gate's guardian that ends after its gate stays `verify`'s zombie until `verify` exits | a row |
| F14 | **The end check is a new user-visible refusal:** `BindingMismatch` (exit 5) also covers the caller's own scenario files and their base changing during `verify`, named for the campaign. The CLI README's exit-5 row doesn't name it | a docs line |
