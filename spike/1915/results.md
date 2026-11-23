# AET-WO-1915 results: where `verify` and `prove`'s memory peak comes from

Measured 2026-11-23 on Furnace. Base is `main` `57b62195`, from a scratch tree exported from this branch. The image is `aethelgard-tools@sha256:e5fc7ddb…1b58` (`container/IMAGE`) and the CLI is built in debug inside it. Every run used `capped-run --mem 16G`, unless it says 8G, with swap 0 read back, `nice -n 10`, `--network none`, and one container at a time (a flock).

**Every peak below is the scope's `memory.peak`, read inside the container as its last step** (`tools/ctl.sh`). The 1 s samples (`tools/sample.py`) are used only to attribute that peak to a phase and a process; §5 says why. GiB = 2^30 bytes. The per-run summaries are in `out/runs/*.json`, with one row each in `out/runs.tsv` and the 1 s timelines in `out/timelines/`. The raw samples stay in `~/.cache/aet-1915/runs/`.

## 1. The answer

- **On today's `main`, neither committed board comes near 8 GiB.**
  - `verify` peaks at 3.50–3.54 GiB on both boards, over five runs at 16G.
  - It passes at `--mem 8G` on both, with the same peak and wall time.
- **The peak is one process: Kani's driver (`kani-driver`), at the end of the `every_valid_sequence_spi1` harness.**
  - It is not CBMC or the solver. kani-driver grows only as CBMC finishes, and peaks at about 3× CBMC's own peak.
  - It is serial: no cargo job is alive beside CBMC in any sample of any run, and the gates run one after another.
- **The 9–13 GB ground came from the unsplit schedule harness, which `AET-WO-3303b` (`52a2d62e`, 2026-11-20) split per bus.**
  - Both grounds, `3304` F3 and `4201` F4, were measured on 2026-11-19, before the split.
  - Measured here from a scratch copy, the bench crate's whole harness takes **11.30 GiB and 1,046 s**. Split, its largest part takes **2.63 GiB** and the three parts **81 s** in all (§4).
- **The lean:**
  - The change that brings the peak under 8 GiB has already landed (3303b).
  - Local callers can go back to `capped-run --mem 8G` for `verify` and `prove` on the committed boards (measured, §4.2).
  - Two boards in `examples/` have no `boards/` directory and are reported separately:
    - `sentinel-02-bench`: its `every_valid_sequence_spi1` peaks at 6.53 GiB, and passes at 8G (§4.1) with 1.47 GiB to spare.
    - `sentinel-02-bench-hse8` (`1607` F-H, routed here mid-spike): its `every_valid_sequence_spi1` is OOM-killed at 16G (§4.1a). At the kill, kani-driver alone holds 15.1–15.3 GiB after CBMC (3.3 GiB) has exited. The lever there is kani-driver, not the formula.

## 2. `verify`, by phase (measured)

| Run | Cap | Exit | Wall s | `memory.peak` GiB | Where the peak is |
|---|---|---|---|---|---|
| `verify-ref-r1` (`sentinel-01-minimal`, `SEEDED`) | 16G | 0 | 135.1 | 3.503 | kani-driver, `prove every_valid_sequence_spi1` |
| `verify-ref-r2` | 16G | 0 | 151.5 | 3.510 | same |
| `confirm8g-verify-ref` | **8G** | 0 | 144.4 | 3.503 | same |
| `verify-bench-r1` (`sentinel-01`, the five bench scenarios) | 16G | 0 | 182.0 | 3.523 | same |
| `verify-bench-r2` | 16G | 0 | 201.7 | 3.516 | same |
| `confirm8g-verify-bench` | **8G** | 0 | 192.6 | 3.536 | same |
| `verify-bench-r0-nobridge` (stopped at `simulate`: the bridge was not built) | 16G | 4 | 175.8 | 3.546 | same |

- **Spread:** 0.2 % on each board at 16G (3.503/3.510 and 3.523/3.516). The 8G runs are within 0.6 % of the 16G runs.
- **Wall time** varies 12 % between repeats (135 to 152 s). Furnace is shared by about eight builders; the peak does not move with it.

**The timeline** (`out/timelines/verify-bench-r1.txt`; the reference board has the same shape):

| Phase | When (s) | Its processes' largest sampled VmHWM |
|---|---|---|
| `check erc`, `check drc`, `check support`, `extract` | 0–4 | kicad-cli 0.08–0.28 GiB |
| `prove` × 11, one after another | 4–162 | kani-driver 3.20–3.43, CBMC 1.13–1.14, goto-cc/instrument ≤ 0.13, kani-compiler ≤ 0.25, rustc ≤ 0.02 |
| `build` (the firmware's cargo build) | ~162 | **45 MiB, 0.2 s alone** (`gate-build-*`, four runs; the ELF hash equals verify's) |
| `simulate` (Renode) | 163–165 | Renode 0.36 GiB |
| `campaign` (Renode, ngspice through the bridge), then `attribute` | 166–180 | Renode 0.38 GiB; ngspice 6 MiB sampled. The campaign's own `memory.peak` alone is in §2.1 |

### 2.1 The campaign alone (measured)

`campaign` on the five scenarios, run alone in its own container on the ELF `build` wrote (`gate-campaign-*`), twice per board. It covers Renode and ngspice (in the bridge) together. All four exited 0 with the gate's `pass`.

| Board | r1 GiB / s | r2 GiB / s |
|---|---|---|
| `sentinel-01` | 0.347 / 17.1 | 0.349 / 16.8 |
| `sentinel-01-bench` | 0.350 / 17.5 | 0.350 / 16.7 |

So the whole campaign phase, Renode included, stays under 0.35 GiB: one tenth of the Kani peak.

## 3. `prove`, per harness (measured, two runs each)

`memory.peak` in GiB and wall time, then the process holding the most anonymous memory at the sampled peak. Every run exited 0 with `"verdict":"pass"`.

| Harness | `sentinel-01` r1 / r2 | `sentinel-01-bench` r1 / r2 | Top process |
|---|---|---|---|
| `every_valid_sequence_spi1` (unwind 77) | **3.476 / 3.476** (60 / 52 s) | **3.492 / 3.480** (68 / 62 s) | kani-driver |
| `every_valid_sequence_i2c1` (77) | 1.976 / 1.969 (29 / 22 s) | 2.227 / 2.229 (27 / 25 s) | kani-driver |
| `every_valid_sequence_i2c2` (77) | — | 2.137 / 2.137 (27 / 23 s) | kani-driver |
| `spi1_transfer_is_bounded_and_ordered` (5) | 1.052 / 1.052 | 1.098 / 1.098 | kani-driver |
| `usart1_start_writes_the_solved_values` | 0.818 / 0.819 | 0.844 / 0.843 | kani-driver |
| `i2c1_transfer_is_bounded_and_ordered` (6) | 0.659 / 0.646 | 0.792 / 0.679 | CBMC / kani-driver |
| `i2c2_transfer_is_bounded_and_ordered` (6) | — | 0.751 / 0.669 | kani-driver / CBMC |
| `spi1_transfer_refuses_too_many_frames` (2) | 0.441 / 0.449 | 0.523 / 0.484 | CBMC / kani-driver |
| `reset_clock_is_ok_exactly_at_reset` | 0.153 / 0.154 | 0.157 / 0.157 | — |
| `flash_option_status_is_one_read` | 0.249 / 0.087 | 0.090 / 0.091 | — |
| `pwr_scale2_is_once_in_order_and_bounded` (3) | 0.095 / 0.095 | 0.099 / 0.099 | — |

- **Every harness of 1 GiB or more repeats within 0.6 %.**
- **Harnesses under 1 GiB vary by up to 0.16 GiB.** That is page cache and short-lived compiler processes counted in `memory.peak`; read those in absolute terms.
- **`verify`'s peak is the worst `prove`'s:** 3.50–3.54 GiB against 3.48–3.49 GiB, within 1.5 %.

**Inside one Kani run** (`every_valid_sequence_spi1`, `sentinel-01` r1, 1 s samples):
- **0–47 s:** CBMC grows to 1.03 GiB, and kani-driver holds nothing.
- **48–57 s:** kani-driver grows at about 0.1 GiB/s while CBMC is still resident. This is the only overlap of two large processes.
- **58–59 s:** CBMC exits, and kani-driver alone reaches 3.39 GiB.
- **What it is processing (inferred, not measured):** CBMC's result stream, including the witness traces of the satisfied `kani::cover!` properties (six on this harness) at unwind 77.

## 4. The candidate: the schedule harness whole and split (measured)

**One variable.** `boards/sentinel-01-bench` at `62e1bafb` (main before 3303b, with the whole `every_valid_sequence`) against `52a2d62e` (3303b, split per bus):
- Only `src/proofs.rs` differs between the two crates (`git diff --stat`).
- Both copies were extracted into the scratch tree, outside the branch.
- Both were proved with today's CLI, with no `--spec`.
- One run of the whole side (1,046 s at about 11 GiB); its result matches 3303b's own figure.

| Crate | Harness | Exit | Wall s | `memory.peak` GiB |
|---|---|---|---|---|
| whole (`62e1bafb`) | `every_valid_sequence` | 0, pass | **1,046.1** | **11.300** |
| split (`52a2d62e`) | `every_valid_sequence_i2c1` | 0, pass | 19.5 | 1.499 |
| split | `every_valid_sequence_i2c2` | 0, pass | 17.4 | 1.440 |
| split | `every_valid_sequence_spi1` | 0, pass | 44.3 | **2.632** |

- **Peak:** −77 % (11.30 → 2.63 GiB). **Wall:** −92 % (1,046 → 81 s for the three parts). 3303b's own figures were 12.16 GB / 1,224 s → 2.87 GB / 87 s; 11.30 GiB is 12.13 GB.
- **The old peak was concurrent, inside one Kani run:**
  - CBMC held 4.25–4.43 GiB from 200 s to its exit at 1,045 s.
  - kani-driver grew to 7.00 GiB in the last 25 s while CBMC was still resident: 11.26 GiB anonymous at 1,044 s.
  - So the split removes the peak by shrinking both processes, not by reordering them.
- **Growth since the split:** the bench's `every_valid_sequence_spi1` was 2.63 GiB at 3303b and is 3.48 GiB on today's `main` (+32 %). The board content generated since (pin devices, `1211b` onward) is the likely cause [inferred].

### 4.1 The declared extra: `sentinel-02-bench` on today's `main`

- **Why it is declared:** `3304`'s ground came from `examples/sentinel-02-bench.json`. It has no `boards/` directory, so it is not a committed board, and it is reported separately for that reason.
- **Method:** generated with today's CLI (`target/s02`, 8 harnesses, all split), then each harness proved once, with the heaviest proved twice.
- **Results:**
  - `every_valid_sequence_spi1` reaches **6.525 / 6.525 GiB** in 158 / 150 s, with the same shape: CBMC 1.75 GiB, then kani-driver 6.40 GiB after CBMC exits.
  - `every_valid_sequence_i2c1` reaches 2.156 GiB and `spi1_transfer_is_bounded_and_ordered` 1.570 GiB.
  - Every other harness stays under 1 GiB.
- **At 8G** (`confirm8g-prove-s02-every_valid_sequence_spi1`), it passes at 6.525 GiB in 155 s. That leaves 1.47 GiB of headroom, the least of any board measured.

### 4.1a A ground that came in mid-spike: `sentinel-02-bench-hse8` (measured to the cap)

- **The row:** `main`'s roadmap gained it after this WO was written (`aaad1555`): "Kani peaks at 16.5 GiB on `sentinel-02-bench-hse8`'s spi1 harness (`1607` F-H) → `AET-WO-1915`". `1607` measured that harness OOM-killed at 16 GiB, and passing at 32 GiB with a 16.5 GiB peak.
- **What was run here:** `examples/sentinel-02-bench-hse8.json`, generated with this WO's CLI (`target/hse8`, 9 harnesses). Its `every_valid_sequence_spi1` was proved twice at `--mem 16G`. A larger cap is outside this WO's fence (§3) and Furnace's one-16G rule.

| Run | Exit | Wall s | `memory.peak` | `oom_kill` | At the kill |
|---|---|---|---|---|---|
| `prove-hse8-every_valid_sequence_spi1-r1` | 143 (killed) | 315.0 | 16.00 GiB (the cap) | 1 | kani-driver 15.08 GiB, CBMC gone |
| `prove-hse8-every_valid_sequence_spi1-r2` | 143 (killed) | 312.7 | 16.00 GiB (the cap) | 1 | kani-driver 15.32 GiB, CBMC gone |

- **The shape is the same as every board here, only larger:**
  - CBMC peaks at 3.29–3.32 GiB and exits at 305–308 s.
  - kani-driver grows at about 0.1 GiB/s for 45 s beside it, then at about 1.25 GiB/s alone, and is killed while still growing.
- **So `1607`'s 16.5 GiB is kani-driver, serial, after the solver has exited.** CBMC needs a fifth of it.
- **What follows:** a change to the formula (the unwind bound, the solver, `--jobs`, a further split) does not act on the process that holds the peak. What kani-driver does with CBMC's results does. That is the next candidate (FEEDBACK D4); this WO has measured its one.

### 4.2 The confirmation at 8G

`verify` on both committed boards under `capped-run --mem 8G` passes. It takes the same wall time as at 16G (144 s and 193 s against 135–152 s and 182–202 s) and reaches the same peak (3.503 and 3.536 GiB). There were no OOM events.

## 5. Adversarial pass, targeted (WO §5)

**A peak missed between samples. Found, and it matters.**
- The largest 1 s sample of `memory.current` is below the in-container `memory.peak`:
  - by up to 0.72 GiB (34 %) on harnesses of 1.5 GiB or more;
  - by 0.03–0.29 GiB (1–8 %) on `verify`.
- **The cause:** kani-driver's final growth runs at about 1 GiB/s for 1–2 s at the very end of a harness.
- **Example:** `sentinel-01` `every_valid_sequence_spi1` sampled 3.447 then 2.796 GiB, while its `memory.peak` was 3.476 both times.
- **So a 1 s sampler alone is not a peak measurement here.** Every peak in this record is `memory.peak`, and the samples only say whose it was.
- **The read is trustworthy:** the in-container read equals the host's within one sample on the self-test (`selftest-2g`: 2.02 GiB for a 2 GiB allocation, sampled 2.003 GiB anonymous).

**A process in the wrong phase. Not found.**
- **The two WO examples:**
  - No sample in 72 runs has a `cargo`, `rustc` or `kani-compiler` process alive beside CBMC (`cbmc_beside_build_samples`, empty in every run).
  - No sample in any `verify` has two gates' processes alive at once.
- **How a phase is assigned:** by ancestry, to the nearest `aethelgard <subcommand>` ancestor; `prove` takes its harness from `--harness`. Through the CLI's reaper and sandbox, every tool process found its gate.
- **The only processes with no gate:** two, both the CLI's scratch guardian (`--aethelgard-internal-guard`), a few MiB, which outlives its parent for a moment by design (`main.rs` `Scratch::new`).
- **Double counting:** summing per-process `RssAnon` cannot double count the peak. At the sampled peak of every heavy run on today's `main`, kani-driver's `RssAnon` is 95–99 % of the cgroup's `anon` (for example 3.389 of 3.452 GiB). The exception is the whole-harness candidate, at 62 %, where CBMC holds the rest (§4).
- **Page cache counted as peak:** a minor effect. `memory.stat` `file` stays at 0.01–0.10 GiB in every run, because the scratch directories are on the container's overlay and are small.

**Two of my own claims were tested and held:**
- **4201's OOM** was on a scratch variant (U2 a BMP581), not on the committed board, and before the split. It has not been run on today's `main`, so "predates the split" is inferred for it.
- **The roadmap row at L1050** ("a board whose schedule harness is unsplit (no timer) needs more than 8 GiB") describes the state before 3303b. Today `board.rs` (L5330) splits whenever `parts` yields two or more, timers or not. A one-part board's "whole" harness is that one part.

## 6. Predictions against the measurements

| # | Verdict | Measured |
|---|---|---|
| P1 | **Wrong, on the process.** The harness family is right: `every_valid_sequence_spi1` | the peak is kani-driver after CBMC, not CBMC. CBMC peaks at 1.1 GiB, kani-driver at 3.4 |
| P2 | Holds | serial; kani-driver is 95–98 % of the anonymous memory at the peak; no cargo job is alive beside CBMC |
| P3 | **Wrong** on the `every_valid_sequence_*` range (2.0–3.5 GiB, not 6–10). Holds on the rest: the short-bound harnesses are ≤ 1.10 and the others ≤ 0.85 | §3 |
| P4 | Holds | the firmware `build` takes 45 MiB; kani-compiler ≤ 0.25 GiB and rustc ≤ 0.02 inside `prove` |
| P5 | Holds, at the low end | Renode 0.36–0.38 GiB, one instance at a time |
| P6 | Holds | ngspice, in the bridge, 6 MiB sampled; the whole campaign phase alone peaks at 0.35 GiB (§2.1) |
| P7 | Holds | bench 3.49 against reference 3.48 GiB, 0.5 % apart |
| P8 | Holds | `verify` 3.50–3.54 against the worst `prove` at 3.48–3.49 GiB |
| P9 | Holds for every run of 1 GiB or more (≤ 0.6 %) | runs under 1 GiB vary by up to 0.16 GiB in absolute terms |
| P10 | **Wrong in direction.** `memory.peak` sits above the largest sample by 0–0.72 GiB, but because of growth between samples, not page cache (file ≤ 0.10 GiB) | §5 |
| P11 | Moot | the change that brings the peak under 8 GiB already exists (3303b's split). No flag was needed on the committed boards |

## 7. Furnace's memory

- **The stop rule, as read:** MemAvailable under 12 GiB, not the `free` column.
  - Before my first run the `free` column read 8–10 GB with MemAvailable at about 50 GB: it is page cache, not headroom.
  - This reading was posted on the bus at the start.
- **The guard:**
  - A run started only with 12 GiB plus its cap available.
  - The sampler SIGKILLs its own scope on any sample under 12 GiB. That was tested with a raised floor: the scope was killed within 1 s.
- **Result:** the lowest MemAvailable seen in any sample of any run was 32.8 GiB, during the two hse8 runs at 16G. The guard never fired outside its test.
