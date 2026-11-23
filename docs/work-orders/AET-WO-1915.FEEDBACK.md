# AET-WO-1915 — FEEDBACK: where `verify` and `prove`'s memory peak comes from

**Status: STOPPED after §1.4, as the WO orders,** with the attribution and the lean. One spike-only PR, unmerged. It touches only `spike/1915/` and this file: no crate, specification, `fab/`, workflow or `ci/` file.

- **Base:** `main` `57b62195`. **Branch:** `wo/aet-1915-memory-peak`.
- **Predictions:** `spike/1915/predictions.md`, committed alone at `7363c014` (04:32Z), before any measured run.
- **The record:** `spike/1915/results.md`. The per-run summaries are in `spike/1915/out/`: 72 runs, each with its JSON summary and its `meta`, which holds the exact command, image, cap and exit.

## Answer first

1. **On today's `main`, the committed boards peak at 3.50–3.54 GiB in `verify`, and `verify` passes at `--mem 8G` on both** (measured).
   - **Runs:** five per board at 16G, plus one at 8G each.
   - **Stability:** the peak repeats within 0.2 % at 16G, and the 8G runs are within 0.6 % of it.
   - **Wall time:** 135–202 s.
2. **The peak is one process, serially. It is Kani's driver (`kani-driver`), not CBMC or the solver.**
   - It comes at the end of `prove every_valid_sequence_spi1`, the largest schedule part.
   - kani-driver grows only as CBMC finishes, and reaches about 3× CBMC's peak: 3.4 GiB against 1.1 GiB.
   - At the peak, kani-driver holds 95–99 % of the scope's anonymous memory.
   - No cargo job is alive beside CBMC in any sample of any run.
   - What it is processing is inferred, not measured: CBMC's results, including the witness traces of the satisfied `kani::cover!` properties (F4).
3. **The rest of `verify` is small** (measured):
   - the firmware's cargo `build`: 45 MiB in 0.2 s;
   - kicad-cli: ≤ 0.28 GiB;
   - Renode: 0.38 GiB;
   - the whole campaign alone, Renode and ngspice in the bridge included: 0.35 GiB.
4. **The 9–13 GB ground came from the unsplit schedule harness. `AET-WO-3303b` (`52a2d62e`, 2026-11-20) split it per bus.**
   - Both grounds, `3304` F3 and `4201` F4, were measured on 2026-11-19, the day before.
   - **The candidate, measured from a scratch copy** (one variable: the bench crate at `62e1bafb` against `52a2d62e`, where only `src/proofs.rs` differs):
     - whole: **11.30 GiB, 1,046 s**;
     - split: largest part **2.63 GiB**, all three parts **81 s**.
     - So the split cuts the peak by 77 % and the wall time by 92 %.
   - **The old peak was concurrent inside one Kani run:** CBMC at 4.25 GiB was still resident while kani-driver grew to 7.00 GiB.
5. **The lean:**
   - **The change that brings the peak under 8 GiB exists, and is on `main`.**
     - Close `roadmap.md` §7.1's row at L1036 on this record (D1).
     - Local `capped-run` callers can go back to `--mem 8G` for `verify` and `prove` on the committed boards (D2).
   - **Nothing in a crate, workflow or CI file is asked for.** Then stop.
6. **The declared extra:** `examples/sentinel-02-bench.json`, `3304`'s ground, which has no `boards/` directory.
   - On today's `main` its `every_valid_sequence_spi1` peaks at **6.53 GiB**: twice at 16G, and once at 8G, where it passes.
   - That leaves 1.47 GiB of headroom, the least of the boards that fit (F3).
7. **A ground routed here mid-spike: `sentinel-02-bench-hse8` (`1607` F-H, roadmap row added at `aaad1555`). Measured to this WO's 16G cap.**
   - Its `every_valid_sequence_spi1` was OOM-killed at 16G twice, at 315 and 313 s.
   - At the kill, **kani-driver alone held 15.1–15.3 GiB, still growing at about 1.25 GiB/s. CBMC had peaked at 3.3 GiB and already exited.**
   - So `1607`'s 16.5 GiB is the same process and shape as every board here, only larger.
   - No formula change (unwind, solver, `--jobs`, a further split) acts on that process. The next candidate is kani-driver's handling of CBMC's results (D4). This WO has measured its one candidate, so it stops.

## 1. What was measured, and how

- **The ground for each run:**
  - the pinned tools image, under `capped-run --mem 16G` (8G where named);
  - swap 0, read back;
  - `nice -n 10` and `--network none`;
  - one container at a time, held by a flock in `tools/img.sh`;
  - the CLI and the bridge built in earlier, separate containers, so neither counts as `verify`'s build.
- **The peak:** the scope's `memory.peak`, read inside the container as its last step (`tools/ctl.sh`). The container's cgroup root is the scope, and the self-test shows the in-container read and the host agree.
- **The attribution:**
  - `tools/sample.py` reads, from the host every second:
    - `memory.current`, `memory.peak`, `memory.stat` and `memory.events`;
    - each process in `cgroup.procs`, with `/proc/<pid>/status` (RssAnon, RssShmem, VmHWM);
    - the host's MemAvailable.
  - `tools/attribute.py` assigns each process to the nearest `aethelgard <subcommand>` ancestor, and `prove` to its harness. The raw samples are in `~/.cache/aet-1915/runs/`.
- **`prove` on its own:** every harness of both committed boards, each in its own container, so each has its own `memory.peak`. Two runs each.
- **`verify`:**
  - `sentinel-01-minimal.json` with `common/verify.rs`'s `SEEDED`;
  - `sentinel-01.json` with `bench_board_tier1.rs`'s five scenarios;
  - each with `--fab-out` into an empty directory, as CI's `bench-package` dispatch does. Two runs each.
- **Short gates alone,** because each falls between two samples: `build`, ×2 per board, and `campaign`, ×2 per board.

## 2. Decisions for the architect

- **D1. Close the §7.1 row at L1036 ("needs more than 8 GiB … until the peak is measured to its source") on this record.**
  - The source is measured: the unsplit schedule harness, removed by 3303b.
  - The row at L1050 (`4600` F11, "a board whose schedule harness is unsplit (no timer)… ruled meanwhile: `prove` at 16G on any board without a timer") also describes the state before 3303b. Today `board.rs` L5330 splits whenever `parts` yields two or more, timers or not.
  - **Lean:** close both, citing `spike/1915/results.md` §4. Both rows are yours: `docs/` is outside this fence.
- **D2. The 16G advice for local callers.** This WO's kickoff says "verify can need more than 8 GiB: use --mem 16G".
  - **Measured:** `verify` passes at 8G on both committed boards, and `sentinel-02-bench`'s heaviest harness passes at 8G.
  - **Lean:**
    - 8G again for `verify` and `prove` on the committed boards and on `sentinel-02-bench`.
    - 32G for `sentinel-02-bench-hse8`, which needs more than 16 GiB (D4).
    - 16G for a new board, until its heaviest `every_valid_sequence_*` has been proved once.
  - The kickoff template and CLAUDE.md's `capped-run` line are yours.
- **D4. `sentinel-02-bench-hse8`: the next candidate is kani-driver, not the formula.**
  - **Measured:** kani-driver alone, after CBMC exits, holds the whole peak (§Answer 7, `results.md` §4.1a).
  - **Proposed, for a follow-up spike:** measure kani-driver's peak against what it reads.
    - The size of CBMC's output for that harness, through Kani's `--keep-temps` and the CBMC command it prints under `--verbose`.
    - Then one Kani option measured from a scratch copy that changes what CBMC hands the driver, if 0.68 has one. Which options exist is not checked here [not measured].
    - It needs a cap above 16G for the passing side, which is outside this WO's fence and Furnace's one-16G rule.
  - **Lean:**
    - A follow-up spike at `--mem 32G` when Furnace has the headroom, one container, with the 12 GiB floor kept: it needs 44 GiB available to start.
    - Until then, hse8 stays a 32G board for any caller, as `1607` F-H says.
    - CI runs no `verify` of it, so no gate is affected.
- **D3. Watch the heaviest schedule part (lean: backlog only, by the scope test).**
  - The bench's `every_valid_sequence_spi1` grew from 2.63 GiB at 3303b to 3.48 GiB now (+32 %). The generated board content since, `1211b` onward, is the likely cause [inferred].
  - `sentinel-02-bench`'s is 6.53 GiB.
  - CI's 16 GB runners hold all of it, and nothing fails today.
  - A cheap guard, if wanted later: `prove` reports its scope's `memory.peak` beside its wall time, or a tier-2 test bounds the largest part.

## 3. Adversarial pass, targeted (§5)

- **A peak missed between samples: found, and it matters.**
  - The largest 1 s sample is below `memory.peak`:
    - by up to 0.72 GiB (34 %) on heavy harnesses;
    - by 0.03–0.29 GiB on `verify`.
  - The cause is kani-driver's last 1–2 s, which grow at about 1 GiB/s.
  - **Example:** the same harness sampled 3.447 and then 2.796 GiB, against a `memory.peak` of 3.476 both times.
  - So every peak in the record is `memory.peak`, and the samples only say whose it is. The same shape fooled P10 (F5).
- **A process in the wrong phase: not found.**
  - No sample in 72 runs has a `cargo`, `rustc` or `kani-compiler` process alive beside CBMC.
  - No `verify` sample has two gates' processes alive at once.
  - Two processes in all the runs have no gate. Both are the CLI's scratch guardian (`--aethelgard-internal-guard`, a few MiB), which outlives its parent by design.
  - kani-driver's RssAnon is 95–99 % of the cgroup's `anon` at every peak on today's `main`, so per-process sums cannot double count.
  - Page cache is not the peak: `memory.stat` `file` stays ≤ 0.10 GiB.
- **Against my own claims:**
  - **"The ground is stale"** started as an inference from a commit title. It is now measured: the whole harness takes 11.30 GiB, which is 12.13 GB against 3303b's 12.16.
  - **4201's own case,** U2 a BMP581 on a scratch tree, was not re-run on today's `main`. "It predates the split" is inferred for it.
  - **The 0.2 s `build`:** I suspected a cached target. It is not cached: the target is fresh in its scratch directory (`firmware.rs` L708), and the ELF hash equals `verify`'s.
- **My own slips, each caught and recorded:**
  - **The bridge:** the first bench `verify` ran without the bridge built, and stopped at `simulate` with exit 4. It is kept as `verify-bench-r0-nobridge` for its gates up to `build`.
  - **Two exit-status bugs in my helpers:**
    - `prove-all.sh` ended on a failing test, so a chained `&&` skipped the bench batch once;
    - a `run` helper printed `$?` after a `$(date)`.
    - Every exit in the record is read from each run's `meta` and its `CTL1915` line instead.
  - **`--exact`:** one candidate run passed `--exact` to `aethelgard prove`, which adds it itself, and was refused. It was re-run without it; the refused one is in `~/.cache/aet-1915/runs-refused-cand-whole-exact`.
- **The spike's own guards, tested live:**
  - **The 12 GiB stop:** with a raised floor, the sampler's first version called `systemctl --user stop`. That waited 30 s on a container whose PID 1 ignores SIGTERM, so it now SIGKILLs its own scope, within 1 s (measured).
  - **The leftover record:** a killed podman client leaves its record behind, so `img.sh` removes its own by name.
  - **The start gate:** it refuses to start without 12 GiB plus the cap available.

## 4. Gates

- **The CLAUDE.md gates were not run.** `cargo fmt --check`, `cargo clippy -D warnings` and `cargo test --workspace` do not apply: the WO grants the exception, and this branch changes no crate, workflow or `ci/` file (`git diff --stat main` lists only `spike/1915/` and this file).
- **Mutations:** none. The WO adds or changes no guard in the repository. The spike's own guards were tested live (§3).
- **Reduction pass (diff): nothing to remove** · 0 duplicates · 0 dead paths removed · 0 protected · 3 findings (F9).

## 5. Findings

- **F1. Furnace's "free memory under 12 GB" read as MemAvailable.**
  - Before my first run, the `free` column read 8–10 GB with MemAvailable about 50 GB: it is page cache.
  - I posted this reading on the bus at the start. The lowest MemAvailable in any sample of any run was 32.8 GiB, during the hse8 runs, and the guard never fired outside its test.
  - If another reading was meant, every run here was inside it anyway.
- **F2. `4201` F4's wording.** It says "Sentinel-01 as ordered", but its run (`4201` FEEDBACK §4.2) was a scratch variant with U2 a BMP581. A protected document; noted, not edited.
- **F3. Of the boards that fit, `sentinel-02-bench` is the closest to 8 GiB:** 6.53 GiB, and 1.47 GiB to spare at 8G. `sentinel-02-bench-hse8`, a twin stand-in, does not fit 16G (D4). The two are the boards to watch if D3's watch is wanted.
- **F4. kani-driver, not CBMC, is the largest process in every heavy harness.** On today's `main` its sampled peak is 2.7–4.6× CBMC's; on the whole harness before the split it is 1.3×. Both are sampled VmHWM, so lower bounds. Its growth starts when CBMC begins to finish.
  - The cause is inferred: parsing CBMC's results, including the satisfied covers' traces at unwind 77.
  - On hse8 it does matter: it is the whole of a peak above 16 GiB (D4).
- **F5. A method lesson for any later memory measurement here.**
  - A 1 s sampler under-reads the peak of a Kani run by up to a third; read `memory.peak`.
  - The method that works: one container per short gate, `memory.peak` read inside it as the last step, and the samples used for attribution only.
- **F6. A per-fd reset of `memory.peak` was not tried.**
  - The idea was to write to the scope's own `memory.peak`, for a true peak per 1 s window. The auto-mode classifier denied it as a cgroup write. I did not work around it.
  - The per-gate containers above answered the same question.
- **F7. `capped-run`'s scope and SIGTERM:**
  - `systemctl --user stop <scope>` blocks for as long as the container's PID 1 ignores SIGTERM, which a bare `sleep` as PID 1 does;
  - `systemctl --user kill -s KILL <scope>` returns at once;
  - after a SIGKILL, the podman client's `--rm` never runs, so the record stays until `podman rm -f <name>`.
  - Measured. Worth a line in furnace777's `capped-run` notes; outside this fence.
- **F8. The WO's "9–13 GB" combines two boards:**
  - 9.1 GiB from `3304` (`sentinel-02-bench`, before the split);
  - 13 GB from `4600` F11 (an I2C+SPI logger, roadmap L1050).
  - The bench board's own whole harness measures 11.30 GiB here.
- **F9. Reduction findings, not edits.** The spike's tools have no suite, and they produced the committed data, so a removal is a finding:
  - `sample.py` records four fields nothing reads: `VmRSS`, `RssFile`, `file_dirty` and `file_writeback`;
  - `ctl.sh` computes one subtraction by `bc`, falling back to `awk`;
  - `attribute.py`'s `--table` and JSON modes compute the same summary twice.
- **F10. Scratch on Furnace.** `~/.cache/aet-1915` holds 2.4 GB: the scratch tree with its CLI build, 2.3 GB; cargo-home, 106 MB; the raw samples, 18 MB. It is kept until the ruling for any re-run you ask for, then removed.
