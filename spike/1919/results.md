# AET-WO-1919 results: the measurement, before the stop

Measured 2026-11-26, base `main` `649e81a8`, branch heads `f6ba23c0` (predictions, committed first) and `4254a2a0` (the probe timing every gate).
- **CI:** the six runs since `1918`'s split that ran `bench-package-s03`: 45620143376 (3377a9a7), 45663730941 (1cc2d01c), 45664966472 (589030ca), 45678279272 (a8d84779), 45709090931 (1ec33945, cancelled by its hang), 45733269794 (b5bac764). Also this PR's first run, 45743010585 (`4254a2a0`), whose probe times every gate. Read with `tools/fetch.sh` (jobs API and job logs, read-only). Raw: `~/.cache/aet-1919/ci/`.
- **Furnace:** Intel Core Ultra 7 265K, 20 cores, shared with about eight builders. The pinned image under `capped-run` (swap 0 read back), `nice -n 10`, one container at a time. `ci/probe.py` (as built at `4254a2a0`) runs inside the container, so it sees only this run's processes (`tools/ctl.sh`). Each run's load average and `free -g` at its start are in its `.meta`.
- Predictions: `predictions.md`, checked in §5.

## 1. `bench-package-s03`

### 1.1 The job's steps on CI (`out/ci-steps.txt`, jobs API)

| Run | Head | CPU (4 vCPUs each) | Job min | Pull | CLI + bridge build | `verify --against-fab` |
|---|---|---|---|---|---|---|
| 45620143376 | 3377a9a7 | Intel Xeon Platinum 8573C | 70.4 | 0.7 | 1.6 | 68.0 |
| 45663730941 | 1cc2d01c | AMD EPYC 9V74 | 60.7 | 0.6 | 1.5 | 58.5 |
| 45664966472 | 589030ca | AMD EPYC 7763 | 69.7 | 0.5 | 1.6 | 67.4 |
| 45678279272 | a8d84779 | AMD EPYC 7763 | 68.5 | 0.6 | 1.6 | 66.3 |
| 45709090931 | 1ec33945 | AMD EPYC 9V74 | 60.0 | 0.7 | 1.6 | 57.7 |
| 45733269794 | b5bac764 | AMD EPYC 9V45 | 47.8 | 0.6 | 1.3 | 45.8 |
| **45743010585** | **4254a2a0** (this PR, the probe timing every gate) | **AMD EPYC 9V74** | **74.3** | 0.7 | 1.6 | **71.9** |

- **`verify` is 96–97 % of the job.** The image pull and the build take about 2.3 min together. The checkout and the probe steps take under 0.1 min each.
- **The slowest run measured is 74.3 min** (45743010585, a 9V74). The two earlier 9V74 runs took 60.0 and 60.7, so one CPU model varies by 24 % between runners.
- **The probe does not explain the 74.3.**
  - **The shards run the same probe** over many more `aethelgard`, Renode and Kani processes. Their test step averaged 9.7 min in 45743010585, against 9.8–10.4 min in the earlier runs (14.9 in 45709090931, which hung).
  - **The runners were slow before the probe started.** This run's `bench-package` (Sentinel-01, a 7763) took 11.8 min against 8.4–8.6 on the same CPU, and its build step, which runs before the probe starts, took 2.1 min against 1.6.
  - **On Furnace,** the old probe and this one each used 0.8 s of CPU per minute; this one added 0.08 s. That test saw only 70 processes and no Kani or `aethelgard` process, so it is the weaker evidence.
**The three runs of `4254a2a0`** (run 45743010585, attempts 1–3; `out/ci-45743010585*-gates.txt`), in seconds:

| Attempt | CPU | Job min | `verify` | generation | `extract` | 26 prove gates | – CBMC | – the CLI's own | campaign |
|---|---|---|---|---|---|---|---|---|---|
| 1 | AMD EPYC 9V74 | 74.3 | 4,312 | 73 | 265 | 3,794 | 1,896 | 1,809 | 162 |
| 2 | **Intel Xeon Platinum 8573C** | **76.2** | 4,400 | 72 | 531 | 3,632 | 1,746 | 1,803 | 148 |
| 3 | AMD EPYC 9V74 | 71.6 | 4,158 | 71 | 258 | 3,651 | 1,781 | 1,785 | 158 |

- **The slowest run measured is attempt 2, 76.2 min, on a Xeon 8573C.**
- The CLI's own share of the prove gates was 1,785–1,809 s in every run: (G)'s target.
- Attempt 2's `tools` job failed on the probe's self-test. Its cause and fix are in FEEDBACK §4. Every other job ran.
- **Projected for the built head on attempt 2's runner:** 76.2 − 29.2 (G) − 11.5 ((X): extract 531 s, the campaign 148 s and `simulate` 10 s, beside the prove lane) ≈ **35.5 min**, plus contention on 4 vCPUs. The pushed head's own runs are in §6.

### 1.2 Inside `verify`, gate by gate (`tools/gates.py` on the probe's lines)

The probe times each `aethelgard` subcommand that `verify` runs as its own process, and inside each prove gate kani-driver, `kani-compiler` and `cbmc` (`ci/probe.py` at `4254a2a0`).

| Part | CI 45743010585 (9V74), s | Furnace `s03-1`, s |
|---|---|---|
| `verify`, whole | **4,312** (71.9 min) | 2,805 (46.7 min) |
| in-process, before the first gate (generation: solve, the schematic, the routed board) | 73 | 27 |
| ERC, DRC, support | 4 | 3 |
| **`extract`** (in-process: the capacitance bound and the supply-drop network; no tool) | **265** | 505 |
| **26 prove gates** | **3,794** (63.2 min) | 2,164 (36.1 min) |
| – of which CBMC | 1,896 | 1,548 |
| – of which kani-driver outside CBMC (its parse) | 62 | 72 |
| – of which `kani-compiler` | 26 | 20 |
| – **of which the CLI's own, outside kani-driver** | **1,809** (30.2 min) | 524 (8.7 min) |
| build, `simulate --firmware`, attribute ×2 | 12 | 7 |
| campaign (11 scenarios; Renode's 15 boots summed to 88–168 s across the seven CI runs) | 162 | 97 |
| in-process, after the last gate (the package derived and compared) | 3 | 2 |

**Every harness, CI 45743010585** (`out/ci-45743010585-gates.txt`; Furnace: `out/furnace-s03-1-gates.txt`):

| Harness | Gate s | CBMC s | CLI s | kani-driver peak GiB |
|---|---|---|---|---|
| `every_valid_sequence_tim1_1` | 352.1 | 274.6 | 73.2 | 7.997 |
| `every_valid_sequence_tim1_0` | 345.1 | 269.4 | 72.1 | 6.955 |
| `every_valid_sequence_spi2_1` | 268.6 | 196.6 | 68.6 | 6.192 |
| `every_valid_sequence_spi1_1` | 251.7 | 179.3 | 69.1 | 6.070 |
| `every_valid_sequence_spi2_0` | 242.0 | 170.1 | 68.9 | 5.297 |
| `every_valid_sequence_spi1_0` | 240.1 | 168.0 | 68.9 | 5.178 |
| `every_valid_sequence_i2c1_1` | 180.7 | 108.4 | 69.3 | 4.447 |
| `every_valid_sequence_i2c2_1` | 169.8 | 98.1 | 68.7 | 4.461 |
| `every_valid_sequence_i2c1_0` | 167.9 | 95.4 | 69.7 | 3.880 |
| `every_valid_sequence_i2c2_0` | 160.9 | 88.7 | 68.9 | 3.794 |
| `spi2_transfer_is_bounded_and_ordered` | 115.3 | 41.1 | 69.3 | 1.598 |
| `spi1_transfer_is_bounded_and_ordered` | 106.3 | 32.2 | 69.4 | 1.409 |
| `i2c1_transfer_is_bounded_and_ordered` | 99.2 | 25.0 | 69.7 | 0.799 |
| `tim1_start_writes_the_solved_values` | 94.7 | 20.7 | 69.9 | 0.939 |
| `usart2_start_writes_the_solved_values` | 94.6 | 19.9 | 69.8 | 1.048 |
| `i2c2_transfer_is_bounded_and_ordered` | 93.0 | 19.4 | 68.9 | 0.792 |
| `usart1_start_writes_the_solved_values` | 92.5 | 18.0 | 70.4 | 1.048 |
| `usart3_start_writes_the_solved_values` | 91.2 | 17.4 | 69.7 | 1.046 |
| `spi2_transfer_refuses_too_many_frames` | 87.2 | 12.7 | 69.6 | 0.558 |
| `every_valid_sequence_gpio_1` | 86.5 | 14.9 | 69.5 | 1.241 |
| `spi1_transfer_refuses_too_many_frames` | 86.3 | 12.4 | 68.9 | 0.554 |
| `every_valid_sequence_gpio_0` | 83.1 | 11.8 | 69.0 | 1.114 |
| `hse_start_is_in_order_bounded_and_named` | 72.2 | 1.2 | 69.3 | 0.025 |
| `reset_clock_is_ok_exactly_at_reset` | 71.4 | 0.6 | 69.1 | 0.023 |
| `pwr_scale2_is_once_in_order_and_bounded` | 71.1 | 0.2 | 69.9 | 0.006 |
| `flash_option_status_is_one_read` | 70.2 | 0.2 | 69.2 | 0.006 |

Every peak equals `1918`'s split figures (largest 7.997 GiB, under the 12 GiB ceiling).

### 1.3 Where the CLI's 69 s per prove gate goes

- **`prove --spec` regenerates the whole tree and keeps only `board/`.** `generated_board` (`main.rs`) calls `generate::tree`, which draws the schematic and routes the board, then takes `tree::subtree(.., "board/")` for the provenance check. The routed board is thrown away. `verify` passes `--spec` to each of its 26 prove gates, so a verify generates the tree 27 times.
- **The routing ladder is what costs, and it costs by core count.** `schematic::emit` routes all 8 rungs of `route::RUNGS` on their own threads (`schematic.rs:1068`). Furnace's 20 cores route them at once (17.8 s for `generate`). A 4-vCPU runner routes them in turns: 73 s in CI, and 27.5 s on Furnace pinned to 4 cores.
- **The board crate needs none of it.** `tree`'s `board/` files are `svd_hal::board::generate_with(topology, table, answers)` and `twin::platform::emit(topology)`. Neither reads the emission.
- **Measured on Furnace** (`out/furnace-micro-*.txt`; the lightest harness, `flash_option_status_is_one_read`, so the gate is nearly all CLI):

| CLI build | `prove --spec` s | `prove` without `--spec` s | `generate` s | `extract` s |
|---|---|---|---|---|
| debug, 20 cores | 18.49 / 18.57 | 0.76 / 0.76 | 17.84 | 466.55 |
| release with debug assertions and overflow checks kept, 20 cores | 16.88 / 16.92 | 0.78 / 0.76 | 16.41 | 428.49 |
| debug, pinned to 4 cores | 27.52 / 27.79 | 0.70 / 0.70 | 27.51 | — |
| **candidate (G): `board/` generated alone, pinned to 4 cores** | **0.83 / 0.83** | 0.70 / 0.70 | 25.26 | — |

  - Under (G), the prove verdict line is byte for byte the debug build's, apart from the `--crate` path each run was given. It is still `pass`, with provenance `generated`.
  - **(G) over the whole verify** (`cand-s03`: the scratch copy with `out/candidate-G.diff`, pinned to 4 cores, `out/furnace-cand-s03-gates.txt`):
    - its verdict line is **byte-identical** to the baseline Furnace run `s03-1` (sha256 `5ba5f66b…`), all 26 prove gates included;
    - against CI's verdict line, it differs in the uid fields only, as `s03-1` does (§1.4);
    - the CLI's own time across the 26 prove gates fell from 524 s (`s03-1`) to **6 s**;
    - `verify` took 32.6 min on 4 pinned cores, against `s03-1`'s 46.7 on 20 unpinned cores under a heavier load. The two are not comparable as times.
  - **A release build is not the lever.** It takes 8–9 % off, because the time is in `aethelgard-eda`, which `[profile.dev.package.aethelgard-eda]` already builds at opt-level 2.
- **`extract` is the in-process copper work,** at eda's opt-level 2: 265 s in CI, and 428–505 s on Furnace. The CLI build does not change it.
  - **Why Furnace is slower here is not explained.** CBMC ran faster on Furnace in the same verify, so this is not the box's load. The projections use CI's figure.

### 1.4 Furnace's `--against-fab` exits 1, from the sandbox alone

Furnace's `s03-1` passed every gate and all 26 prove gates. `--against-fab` then said `MANIFEST.json` differs, so the verdict is `fail`. Diffing its verdict line against CI's (45733269794):
- **the only differences are `uid_boundary: false` and `uid_boundary_unmet: "user_namespace"`,** on each gate and on `fab`;
- every judged value is identical: gates, harnesses, hashes, the campaign and the ELF `6e4a81f2…`.

Rootless podman is not the initial user namespace. So the verdict line, and with it `verdict_sha256`, differs on Furnace by construction. **The committed package can be re-derived only where the uid boundary holds, which today means CI.** `1918` §6.1's Furnace run also differed in `MANIFEST.json` alone, which that work order read as its split's expected move. Under the same rootless podman it would have differed in any case [inferred: its uid fields were not compared]. This is recorded so that a Furnace `--against-fab` is not read as a moved package.

### 1.5 The options: under 45 min on the slowest runner measured (74.3 min at the stop; 76.2 since), every proof and gate kept

Projected from 45743010585's own parts. (G)'s per-gate CLI time in CI is taken as 2 s, against 0.83 s measured on 4 Furnace cores, where the whole-tree generation took 27.5 s against CI's 73.

| Option | What changes | Projected job, min | Bytes that move | Ruling needed for |
|---|---|---|---|---|
| **(G) `prove --spec` generates `board/` alone** | `generated_board` calls a `generate::board_tree` that skips the schematic and the routing, which it discarded anyway. Every gate and harness runs as today | **74.3 − 29.3 ≈ 45.0** | **none expected:** each prove line is byte-identical (measured on one harness). Shown on CI by `--against-fab` unchanged | `main.rs`'s `prove` and `generate.rs` are outside the fence. One behaviour moves: a specification whose board cannot be drawn or routed, but whose crate generates, is no longer refused by a standalone `prove --spec`, which would say `generated`. `verify` refuses such a specification first, so `verify` does not move |
| **(G) + (X) `extract`, then build, `simulate` and the campaign, run beside the prove gates** | `verify` runs two lanes: the 26 prove gates in order, and beside them extract → build → simulate → campaign → attribute. Each lane's results go into the report in today's order. A refusal is the first in today's order, and the other lane is waited for and killed with `verify` (`PARENT_PID_VAR`) | **≈ 37.7–40** (45.0 − 7.3, less contention on 4 vCPUs: CBMC takes one core, extract one, the campaign's Renode up to two) | none: the verdict line is assembled in today's order | `verify.rs` orchestration ("only as ruled"). Memory: extract and the campaign are under 1 GiB beside a ≤8 GiB harness on a 16 GiB runner |
| (G) + (M) the prove gates in 2 matrix jobs, then one job composing | Each prove job proves a measured-balanced half, under the 12 GiB ceiling, and uploads its prove lines. `verify --proved DIR` binds each line (harness, `input_sha256`, provenance, versions) instead of running it | ≈ 31.5 (each prove job ≈ 20.5 = 2.3 setup + 1.2 generate + 17; the compose job ≈ 11) | none, if the composed lines are byte-identical. But `verify` no longer runs what it reports | **a change to what `verify` derives** (the work order's stop): it trusts a file for a proof. A local `verify --proved` could be handed a forged line, so the verdict would have to say its prove gates were imported, and then the verdict line and `MANIFEST.json` move |
| (C) a cache of prove results keyed by input hashes | A prove line reused when its key matches | 45.0 on a hit (and (G) is still needed on a miss) | none on a hit | **not recommended.** The key must cover the crate, the harness, Kani, CBMC, the image and the CLI's judge. Keyed on the CLI binary, nearly every code PR misses. Keyed on less, a proof can be reused past a change to its judge (the adversarial pass's "a cached result whose key misses an input") |
| (R) a release CLI | `cargo build --release` with the checks kept | ≈ 70 (−8 %) | none expected | not recommended: it is not where the time is |

**The room is thin, so weigh it:**
- one CPU model varied by 24 % between runners (§1.1);
- `4411`'s `uart4` harness is still to come, and it adds to `prove`;
- after (G), about 32 of `verify`'s 43 minutes are CBMC, which only (M) reaches.

So (G) alone does not pass, and (G) + (X) has about 5–7 min in hand on the slowest run measured.

**(X)'s independence, read in `proof_and_boot_gates`:** build, `simulate` and the campaign read only the ELF and the board, never a prove gate's result. Today a refusal at `extract` stops `verify` before any prove gate runs. Under (X), the first refusal in today's order (extract before prove) must still be the one printed, and the other lane is ended with `verify`.

**Lean: (G) + (X).** It removes a repeated build (the work order's third example) and runs the gates that share no input beside each other. Every proof and every gate still runs, by name, in `verify` itself, and no byte should move. (M) stays in reserve: it is the only option that halves CBMC's 32 min, and it is a verdict-derivation change.

**Side effect of (G) on the shards:** every tier-2 test that runs `verify` or `prove --spec` saves about 68 s per prove gate in CI. Their weights in `ci/test-weights.tsv` would then be high, so the shard plan should be re-weighed from the built head's runs, as at `1918` (3).

## 2. Platform loads

### 2.1 Every Renode invocation the CLI makes (the census)

| Where | What | Bound today |
|---|---|---|
| `main.rs` `simulate_with`, `(None, _) => run.run()` | **a load:** `simulate --platform` with no `--firmware`; also the MCP `simulate` tool, which runs the same subcommand as a child | **none** |
| `main.rs` `simulate_with`, before either | **`renode --config … --version`**, the pinned-build check, on every `simulate` (load or boot), so on every campaign scenario and every `verify` | **none** |
| `main.rs` `simulate_with`, `run_bounded_filtered` / `run_bounded_summarised` | a boot, `--foreign` included | `BOOT_BOUNDS`: 120 s hang guard, 1.5 GiB |
| `main.rs` → `analog::boot` | a droop's boot and its undisturbed boot, Renode as the bridge's child | `BOOT_BOUNDS` |
| `campaign.rs` | each scenario through `crate::simulate` | as `simulate` above |
| `doctor.rs` `renode()` | `renode --config … --version` | **none** (outside this work order's fence) |

The bridge alone (`analog.rs`, ngspice) keeps `BRIDGE_BOUNDS`.

### 2.2 Each load's wall time

Renode's own lifetime, first seen to gone (`ci/probe.py`, 0.2 s polls). In CI, the CLI's runs are told apart by their configuration in its scratch (`--config /var/lib/aethelgard/…`), and kinds by the command line the CLI builds (`tools/renode_ci.py`). A load's `-e` starts `include @` or `mach create`; a boot's starts `emulation SetSeed`.

**CI, the CLI's loads** (real Renode only; tier-1 shell stand-ins named `renode` set aside), seven runs:

| CPU | Loads | Median s | Max s | Max in |
|---|---|---|---|---|
| AMD EPYC 7763 | 10 | 3.0 | 4.5 | 45733269794 shard 16 |
| AMD EPYC 9V45 | 13 | 1.6 | 3.6 | 45709090931 shard 0 |
| AMD EPYC 9V74 | 11 | 2.6 | **5.1** | 45678279272 shard 17 |
| Intel Xeon Platinum 8573C | 3 | 3.0 | 3.2 | 45664966472 shard 4 |
| Intel Xeon 6973P-C | 3 | 1.8 | 4.0 | 45663730941 shard 17 |
| Intel Xeon 6975P-C | 5 | 3.2 | 4.0 | 45733269794 shard 5 |
| Intel Xeon Platinum 8370C | 1 | 3.8 | 3.8 | 45664966472 shard 0 |
| **hung (run 45709090931, the failed load at its prompt, `4400j` F14)** | 2 | — | **3,111 and 2,852** | shards 15 (9V45) and 7 (9V74) |

- **The slowest load measured in CI is 5.1 s,** 23.5× under the 120 s guard. **No load comes within 5× of it** (24 s).
- About seven CLI loads run in each full CI run.
- **Each whole `simulate --platform` call in 45743010585** (the probe's gate lines, the CLI's own start to exit): 12 calls, at most 4.7 s (9V74).
- **No load test sets a timeout of its own.** Each calls `Command::output()` and waits. So today what ends a hung load is the shard job's 60 minutes. The MCP server sets none either (`mcp.rs`: "`RUL-13` gives the call no timeout").
- **The CLI's `--version` runs:** at the probe's 0.2 s floor in almost every case, with a maximum of 1.0 s (9V74), over 641 runs.
- **Loads a test makes itself** (`models_tier2`, Renode with its own configuration under `/tmp/aet-4220-models-*`, no CLI): median 4.0–9.0 s by CPU, max 9.1 s (8370C) and 9.0 s (7763). The CLI does not run these, so no CLI guard reaches them. They are listed because the work order asks for every tier-2 test that loads.

**Furnace** (`out/furnace-loads1.txt`; load average 21 on 20 cores at the start): each platform loaded three times by `simulate --platform`, then the six tier-2 tests that load, one at a time.

| Platform | Wall s (the CLI's call, three runs) |
|---|---|
| `boards/sentinel-01`, `boards/sentinel-01-bench`, and the generated `sentinel-01`, `sentinel-01-minimal` | 2.14–2.28 |
| generated `sentinel-01-ext-bmp581` (a model of our own) | 3.29–4.79 |
| generated `sentinel-02-bench`, `sentinel-02-bench-hse8` | 3.51–3.77 |
| **generated `sentinel-03`** (two models of our own, compiled first) | **6.14–7.35** |
| `@platforms/boards/nucleo_h753zi.repl` | 2.08–2.17 |
| `@platforms/cpus/stm32h753.repl` (fails to load, exit 1) | 1.42–1.59 |

| Tier-2 test that loads | Wall s |
|---|---|
| `with_tools::simulate_loads_the_reference_board` | 2.17 |
| `with_tools::simulate_fails_a_platform_that_does_not_load` | 1.46 |
| **`with_tools::mcp_simulate_loads_the_nucleo_board`** (the MCP `simulate` tool) | 2.13 |
| `twin_tier2::the_emitted_platform_loads_and_lists_every_solved_peripheral` | 2.32 |
| `twin_tier2::a_device_named_for_a_renode_instance_or_keyword_breaks_the_load` | 2.91 |
| `twin_tier2::the_fixture_s_platform_lists_its_models_and_an_unpinned_one_is_refused` | 3.88 |

- Renode's own lifetime in these loads: at most 7.3 s (`out/furnace-loads1-probe.txt`). The whole run's scope peaked at 659 MiB.
- **The slowest load measured anywhere is 7.35 s,** Sentinel-03 on a loaded Furnace: 16× under 120 s.

### 2.3 For the ruling

- **(2a) The load's bound is a wall guard only.** A bounded load gains no verdict field: `memory` and `wall_bound_ms` are set only on a boot's verdict (`main.rs`, inside the `(ready, plan)` branch). So a load's verdict moves only under a test's `--wall-bound-ms`, if (2e) records it there. `run::Bounds` carries the wall bound and the 1.5 GiB resident cap together, and the reaper enforces both. The work order grants the hang guard and leaves the memory cap out (`4400j` F4). **Lean:** a load runs under `LOAD_BOUNDS = { wall: BOOT_BOUNDS.wall, rss_kib: u64::MAX }`. The reaper still samples, but a load is never killed for memory. The alternative is the boot's 1.5 GiB cap, which is a new bound on a load.
- **(2b) `simulate`'s `renode --version`** is an invocation the CLI makes before every load and boot, and it is unbounded. It sits in `main.rs`'s `simulate` dispatch, inside the fence. **Lean:** the same guard, also a `ToolError` when it fires.
- **(2c) `doctor`'s `renode --version`** (`doctor.rs`) is outside the fence. **Lean:** the same guard, by a grant to touch `doctor.rs`; failing that, a row.
- **(2d) The kill's text** is the boot's, naming a load: "Renode did not return from its load within the 120 s wall-clock hang guard and was killed; a hang judges nothing about the platform, so no verdict is written (AET-WO-1919)", with exit 5 and kind `ToolError`. There are no findings and no verdict, as for the boot (`renode::bounded`).
- **(2e) The planted test** uses the hidden `simulate --wall-bound-ms`, which today `requires` `--firmware`. **Lean:** admit it on a load too, still hidden, still absent from the MCP tools, and recorded in the verdict as today. Then a tier-1 stand-in that answers `--version` and then reads its stdin to an end that never comes is killed at a 2 s guard, as `a_renode_waiting_on_its_stdin_is_refused_at_the_hang_guard` is for a boot.

## 3. An empty input

**Searched by parsing, not by one spelling** (`git ls-files`):
- every committed JSON, walked for any object with `input_version` or with both `frames` and `port`;
- every `.rs` for an empty `frames` array or an `input_file(…, &[])` call;
- every `check_input(` call site.

**Found:**
- **15 committed inputs:** four fixtures (`tests/fixtures/4400i/*.json`, 41–42 frames), and `spike/4400i` and `spike/4400j` inputs and the inputs held in their verdicts (41–118 frames). **None is empty.**
- **One literal `"frames": []`:** `boot_tier1.rs:3841`. It is refused as `InputMalformed` for `input_version: 2`, before its frames are read. With the empty check placed after the version check, it keeps that kind and that reason.
- **No test helper call passes an empty frame list.** Every `input_file` call in `boot_tier1.rs` passes one frame or more. No twin test calls `check_input` with none.
- **No committed archive holds an input.** The chaos archive tests hold none, and no trace file is committed outside `spike/`.

**So nothing committed relies on an empty input. Stop condition (3) does not fire.**

**For the ruling:**
- **(3a) Where:** `frames_of`, in the CLI's reader (`renode/foreign.rs`, fenced), after the version check and before the port. Refused `InputMalformed`, exit 2 (`EXIT_INVALID_SPEC`), with the error's `id` set to `frames` and a message naming it.
- **(3b) Replay:** `replay` re-reads an archived input with its own loop (`foreign.rs` ~3626), so that "the file's frames are refused exactly as they were". **Lean:** one shared check, so replay refuses an empty input too. No such archive can exist once the reader refuses one, and none is committed.
- **(3c) "By another name":**
  - **Frames each with nothing to send** cannot reach a run. Each frame's `mavlink` must parse as one whole listed MAVLink 2 frame (`parse_exact`), so every frame sends at least a header and a CRC.
  - **An input that arms nothing** (heartbeats only, say) is accepted, and it sends what the firmware reads. **Lean:** not empty, so not refused. A rule that an input must arm is a judgement about the experiment, not about absence.

## 4. `4411`

`git ls-remote` at 02:21Z: `wo/aet-4411-radio-module` is `de132429`, its spike record. It holds no Sentinel-03 with the module, so there is nothing of `4411`'s to measure. Re-checked at the stop: `de132429`, unchanged.

## 5. Predictions against the measurements

| # | Verdict | Measured |
|---|---|---|
| P1 | **Wrong** | pull 0.5–0.7 min and build 1.3–1.6 min, both under their ranges. `verify` 45.8–71.9 min, a wider range than predicted |
| P2 | **Wrong, low** | `prove` is 88 % of `verify` in CI (63.2 of 71.9 min) and 77 % on Furnace |
| P3 | **Wrong on the cause** | `kani-compiler` takes 0.6–1.2 s per harness. The repeated work is the CLI's own generation of the whole tree, 69 s per prove gate in CI, 30.2 min in all (§1.3) |
| P4 | Holds | `tim1_1` 352.1 s and `tim1_0` 345.1 s in CI. The six split parts of `tim1`, `spi1` and `spi2` take 45 % of `prove` |
| P5 | **Wrong, high** | beside `prove`: extract 4.4 min, campaign 2.7, generation 1.2, build 0.4 s, ERC/DRC/support 4 s; 8.6 min in all |
| P6 | **Wrong on the vendor; holds on the spread** | an Intel Xeon 8573C ran the job too (70.4 min). The slowest measured is a 9V74 (74.3 min). One model varies 24 % |
| P7 | Holds | 46.7 min on a loaded Furnace; `prove` is the largest share (77 %) |
| P8 | **Wrong** | the build takes 1.3–1.6 min |
| P9 | Holds, barely | (G) alone projects 45.0 min on the slowest run, not under. Two matrix jobs with (G) project 31.5 min, and they change what `verify` derives |
| P10 | Holds | CLI loads in CI: at most 5.1 s, median 1.6–3.8 by CPU. Whole `simulate --platform` calls (the probe's gate lines, 45743010585): at most 4.7 s |
| P11 | **Wrong** | Furnace up to 7.35 s (Sentinel-03, two models compiled, load average 21) |
| P12 | **Wrong, high** | about 7–8 CLI loads per full CI run |
| P13 | **Wrong** | `with_tools::mcp_simulate_loads_the_nucleo_board` loads through the MCP tool in tier 2 (2.13 s on Furnace) |
| P14 | Holds | `--version` at most 1.0 s in CI. `doctor`'s is unbounded too |
| P15 | Holds | §3 |
| P16 | Holds | §3 (3c) |
| P17 | Holds on the branch | `de132429` is the spike record. The harness half is not measurable yet |

## 6. The built head (after the 04:51Z ruling: (1) (G)+(X), (2a)–(2e), (3a)–(3c))

**On Furnace** (`out/furnace-built-s03-gates.txt`, `out/furnace-built-s03.meta`): the built head `72b51e0e`, Sentinel-03's whole `verify --against-fab`, pinned to 4 cores.
- **Its verdict line is byte-identical to the baseline `s03-1`** (sha256 `5ba5f66b…`). Every gate, all 26 prove gates included, and every binding are unchanged. Against CI, the only differences are the uid fields (§1.4).
- **The lanes, as the probe saw them:**
  - `extract` started at 29 s beside the first prove gate, and ran 478.6 s;
  - build, `simulate` and the campaign were done by 630 s;
  - the prove lane ran to 1,693 s.
  - The CLI's own time across the 26 prove gates is 7 s.
- **28.2 min** on 4 pinned cores. With (G) alone it was 32.6, and the baseline 46.7 on 20 unpinned cores.
- **Memory:** the combined resident peak of the watched processes was 8.53 GiB, at 1,431 s (a `tim1` part, after the campaign had ended). The container's `memory.peak` was 9.68 GiB. Both are under `1918`'s 12 GiB.

**In CI:** (from the pushed head's runs)
