# AET-WO-1918 predictions, committed before any measurement

Written 2026-11-25, on `main` `fd12b2f8`. Each prediction names its falsifier. They are checked in `results.md`.

**Method** (`AET-WO-1915`'s, its tools copied into `tools/` with their paths renamed):
- One container per harness, under `capped-run --mem 16G` with swap 0.
- The peak is the scope's `memory.peak`, read inside the container as the last step.
- A 1 s host sampler attributes it to a process. 1915 §5 showed a 1 s sampler alone under-reads a Kani peak by up to 34 %.

## (1) The prove peak

- **P1. The heaviest Sentinel-03 harness is a schedule part, `every_valid_sequence_spi1` or `_spi2`.**
  - Its `memory.peak` is 14.5–16 GiB, the cap.
  - kani-driver holds at least 90 % of the anonymous memory at the peak, after CBMC has exited, as on `hse8` (1915 §4.1a).
  - CI's sampled 14.73 GiB (run 45501452199) and Furnace's sampled 14.97 GiB are both under the true peak.
  - **Falsifier:** the heaviest harness is not a schedule part, or CBMC holds the peak.
- **P2. Every Sentinel-03 harness that is not a schedule part peaks under 4 GiB, and at most two harnesses pass 12 GiB.**
  - **Falsifier:** a third harness over 12 GiB, or a non-schedule harness over 4.
- **P3. Sentinel-01's and Sentinel-01-bench's heaviest is `every_valid_sequence_spi1`, at 3.4–4.5 GiB.** That is 1915's 3.48, plus whatever the boards gained since.
  - **Falsifier:** a peak outside that range.
- **P4. The kani-driver peak scales with what CBMC hands it, not with CBMC's own memory.**
  - For the heaviest harness, CBMC's JSON result stream is at least 0.5 GB.
  - Most of it is the traces of satisfied properties: the reach checks Kani adds per assertion, and the `kani::cover!`s.
  - **Falsifier:** a stream under 0.5 GB, or one made mostly of something other than traces.
- **P5. A split that keeps every property brings the heaviest part under 12 GiB.**
  - The candidate: split the heavy schedule part's assertions into harnesses that each check a subset of the final state over the same schedule.
  - Each split part keeps the full schedule, the same unwind and the same covers, and the union of their asserts is the original's.
  - **Falsifier:** a part over 12 GiB with the fewest assertions, which would mean the peak is the schedule's, not the assertions'.

## (2) The 30 s wall bound

- **P6. The boot that CI kills at 30 s takes 15–20 s on Furnace undisturbed, and 25–40 s on a loaded GitHub runner.** That is a 1.5–2.5× runner-to-Furnace ratio.
  - **Falsifier:** a Furnace boot over 25 s, or a ratio outside 1.3–3×.
- **P7. A virtual-time bound already exists**, the boot's emulated nanoseconds (`virtual_ns`), and it is what the verdict's timing facts rest on. The wall bound is only a guard.
  - **Falsifier:** a verdict fact computed from wall time.

## (3) The refit

- **P8. `verify_2k2_tier2`'s block is the longest indivisible unit, at 1,300–1,550 s, so a single shard cannot meet 20 min with it whole.**
  - Meeting the clause needs that block split into its two tests, or its verifies made cheaper.
  - (1) shrinks its prove phase, so part of the cost goes with (1).
  - **Falsifier:** the block under 1,000 s on the slowest runner after (1).
- **P9. `bench-package-s03` drops from 54 min toward 35–45 min after (1).** Kani's time falls with its memory, as the 3303b split showed.
  - **Falsifier:** no drop of 5 min or more.
