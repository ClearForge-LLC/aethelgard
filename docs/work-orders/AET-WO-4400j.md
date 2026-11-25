# AET-WO-4400j — The foreign window's cost: the IMU's log flood summarised, Renode's retained growth found, and a window long enough for an armed input run, inside the CLI's bounds

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4400j-foreign-window-cost`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4, the foreign judge. Written 2026-11-25. **Cadence:** a measurement, a stop, then the build.

**Grounds:**
- **`AET-WO-4400h`** (#393, the record), D2 and FEEDBACK F1–F4:
  - **F2:** the foreign log budget is spent by the IMU's per-transaction SPI lines, about 2.17 MB per virtual second from 4.4 s. These are the device and mux NOISY lines the judge enables, the CLI's own `AETHELGARD-CFG2` hook, and Renode's SPI slave-mode ERROR. Each is needed once. The window cannot pass 8 s until they are summarised, as `Unhandled write` already is.
  - **F3:** Renode's RSS grows about 100–120 MiB per virtual second with the IMU's traffic, and at least part of it is live managed memory. It is not the log, the hooks, a lagging GC or our ICM model's buffers. What holds it is not found.
  - **F4:** a window's end can land on a firmware transient. At exactly 6.000 s, GPIOB PB13 reads output/AF0.
- **`AET-WO-4400i`** (#396), F6 and F7:
  - an input run kept 5.0 MB of the 8 MiB log budget at 5 s;
  - the armed 12 s run kept 56 MB of log and peaked at 2.13–2.45 GB against the 1.5 GiB cap (`RUL-13`), possible only with the spike's lifted bounds;
  - parameters must arrive after `ArduPilot Ready` (5.2 s). The arm is witnessed at about 9.8 s.
- **`AET-WO-1918`** (#401): the wall bound is a 120 s hang guard whose kill is a `ToolError`, so wall time no longer judges the firmware.
- **Roadmap §7.1:** the rows the window's end, the reboot and DShot's timer DMA name as "after `4400j`".
- `INV-04`, `INV-06`, `RUL-13`.

> **What this is:**
> - **(1) Measure first, with predictions committed before the runs.** On Sentinel-03's ArduPilot ELF, with and without an input, at 5, 8, 10 and 12 s:
>   - the log by source, per virtual second;
>   - the RSS by virtual second, and what holds it: a managed heap dump (or the equivalent the pinned image allows) at two points of the window, compared by type and root;
>   - the wall time on Furnace pinned to 4 cores, and the CI estimate at the slowest runner;
>   - the end-of-window reads at each length, against F4.
> - **(2) Build, to the ruling:**
>   - **the flood summarised:** each repeating per-transaction line is kept as its first and last occurrence with a count, as `Unhandled write` is. The judge's reads of those lines are shown to be unchanged, with a planted test per source;
>   - **the retained growth:** fixed where the holder is ours, or in the platform or the CLI's configuration of Renode. Where it is Renode's own, it is named with its evidence as an upstream row, and the window's bound is set from the measured growth;
>   - **the window:** a length that holds an armed input run on Sentinel-03 (the arm at about 9.8 s) inside the 1.5 GiB cap and the 8 MiB log budget, with its margin stated. If no length fits, the stop says what does;
>   - **the window's end:** placed off a firmware transient, or the end read repeated, so the verdict does not depend on where the window lands (F4).
> - **Each change comes with a planted test, and each test with a mutant it kills.**
>
> It is NOT:
> - the reboot (a reset macro), or DShot's timer DMA (their rows follow this one);
> - the WiFi module (`AET-WO-4411`, running beside you);
> - the bounds of `RUL-13` themselves: the 1.5 GiB cap and the 8 MiB budget stay. Raising either is Scotty's, and is a stop;
> - Tviburi.

## 1. Build

1. **Measure first** (1). **Stop** with:
   - the log and RSS tables by virtual second, with and without an input;
   - the holder of the retained growth, as far as the dump shows it;
   - the window length you propose, and its margin against the cap and the budget;
   - the F4 reads at each length;
   - every committed verdict or package this would move, by name.
2. **Build (2)** to the ruling. Merge main before READY.

## 2. Acceptance

- **An input run on Sentinel-03's ELF arms inside the CLI's own bounds** at the ruled window: no lifted bound, the arm witnessed from the firmware's heartbeat, and the run replayed byte for byte from its archive.
- **The log of a 12 s run** keeps the first and last line of each repeating source with its count, and the judge's findings are unchanged against the unsummarised log (a planted firmware per source).
- **The retained growth** is fixed, or named with its dump as evidence.
- **The window's end** gives the same verdict at the ruled length and one quantum either side.
- **The 14 generated boards' committed verdicts are unchanged,** or each change is named.
- **CI is green.** Every new test is in a shard, at its measured weight.

## 3. Fences

- **Fence:**
  - `crates/aethelgard-cli/src/foreign.rs` and the foreign path of `renode.rs`;
  - the twin's log filters and the CLI's Renode configuration;
  - `tests/` for the foreign judge;
  - `ci/test-weights.tsv`: your own rows;
  - `spike/4400j/`, `docs/work-orders/AET-WO-4400j.*`.
- **Not:**
  - `RUL-13`'s bounds (a stop);
  - the EDA crates, the specifications and the fab packages (`4411`'s);
  - the steering documents;
  - Tviburi.
- **Running beside you:** `AET-WO-4411` (Sentinel-03's WiFi module). It changes Sentinel-03's board and specification, and may move its ELF. Measure on main's Sentinel-03. Whichever of you lands second merges main and re-measures what moved.
- **If a committed verdict or package moves,** stop and name it. The package path is the architect's `write` dispatch.
- One PR, left unmerged. Write `~/.cache/aet-4400j/pr-title-body.md`.

## 5. Adversarial pass, full

- **A summary that hides a finding:** a source whose 1,000th line differs from its first in a field the judge reads.
- **A growth fix that changes behaviour:** a twin that frees what the firmware still reads.
- **A window that passes by where it ends:** an arm or a fault that lands one quantum past the end.

## 7. Flag-and-stop conditions

- **After §1,** with the measurement.
- **No window length fits the cap and the budget** with the growth as found. Stop with the numbers; raising a bound is Scotty's.
- **The retained growth is Renode's own** and cannot be configured away. Stop with the dump.
- **A committed verdict moves.**
