# AET-WO-4400j results: the foreign window's cost

**STOP: §1, with the measurement.** Measured 2026-11-25 on Furnace, main `589030ca`. Nothing in a crate is changed on this branch.

## The answer first

1. **The retained growth is ours to fix, and it is not the IMU's.**
   - The CLI starts Renode with `--console` and stdin from `/dev/null` (`run.rs:561`, inherited through the reaper and confine, and by the bridge's Renode).
   - Renode's console reader (`ConsoleIOSource`, `isInputRedirected` true) then reads EOF forever. It queues each `-1` into a `BlockingCollection<int>` that nothing drains.
   - That queue is **100.0 % of the managed heap's growth** between two heap dumps (6 s and 10 s). Every item in it is `-1`.
   - It grows about **48 MiB per wall second**, whatever the firmware does: with or without an input, before or after the IMU starts.
   - **With stdin held open** (a pipe never written), Sentinel-03's peak is **547,604–584,568 KiB (348–372‰ of the cap) at every window from 5 to 12 s, with or without the input**. Before, it was 622–1,861‰. Every verdict is otherwise identical: all eight cells compared.
2. **The log is now the only bound a 12 s armed run passes:** 56.07 MB kept against 8 MiB.
   - It has five per-transaction sources, each with very few distinct texts.
   - With them summarised (exact text, plus U5's FIFO data elided once its data step is witnessed), the 12 s armed run would keep **about 0.97 MB (12 %)**. With no input it would keep 0.20 MB.
3. **The arm is at 8.752 s on main's Sentinel-03** with `4400i`'s `arm12.json` (the armed heartbeat's first byte), not about 9.8 s.
4. **The window's end lands on firmware state, three kinds of it.** The repeated read cures the first two; the third needs a rule:
   - SPI2/SPI1 stopped between transactions (pin and clock enable): at most 2 quanta in a row, measured per quantum;
   - the core inside an interrupt handler: at most 2 quanta in a row;
   - **USART2.CR2 at two stop bits for 2 s of every 6 s:** ArduPilot's RC-protocol search on a CRSF port with no receiver.
5. **Proposed window: 12 s.**
   - The arm at 8.75 s leaves a 3.25 s margin.
   - The end lands in a clean phase of the RC search, `[10.222, 14.2)`.
   - Margins with the fix: memory 63 %; log about 88 % (after summarising); wall 44–46 s on Furnace, about 55 s at CI's measured ratio, against the 120 s guard.

## Inputs

| | |
|---|---|
| Board | **Main's Sentinel-03** (`examples/sentinel-03.json` at `589030ca`, sha256 `f6d409a3…`), plus `mcu.foreign_clock` only (`py/derive.py`). The clock is read from the pinned ArduPilot image's `stm32h7_mcuconf.h` for `OSCILLATOR_HZ 16000000`: timers 200 MHz, USARTs **100 MHz** (PLL2_Q = 16/2 × 75/6), VOS scale 1. Derived spec `0c5bb563…`, in the cache, never in `examples/` |
| ELF | Copter-4.7.2 built unmodified by CI's `container/ardupilot-build.sh` in the pinned image `e1809b90…` (`sh/board.sh`): **`97afa171…`**, hwdef `7f2b62aa…`. CI builds no Sentinel-03 ELF |
| Anchor | `4400h`'s spec (schema 29) is refused by main's CLI (`UnsupportedSchemaVersion`, as `INV-06` requires). The tie-back is the log: main's armed 12 s run keeps 56,072,034–56,072,420 B against `4400i`'s 56,071,523 |
| Input | `4400i`'s `arm12.json` (`7f231564…`); for 5, 8 and 10 s, its prefix (`py/cut.py`), since a longer input is refused (`InputPastWindow`) |
| CLI | main + `patches/measure.patch`: the bounds and the log budget from the environment, the kept log dumped, `MAX_CAPTURE` 256 MiB, the window ceiling 30 s, and `AET4400J_HOLDIN` (stdin a held-open pipe). Unset, these are main's |
| Runs | **CLI path:** `sh/cli.sh`, `sh/series.sh`, one container per run (`sh/img.sh`: `capped-run` 8 GiB, swap 0, read back), **pinned to 4 cores** (`taskset`, read back in `env.txt`), `nice -n 10`, load 6–16. These runs carry every margin. **Replays:** of the CLI's own capture (`sh/capture.sh`, `sh/replay.sh`), stepped in whole virtual seconds with a probe (`py/stepify.py`). These carry the per-second figures, the dumps and the end-read timeline |
| Tables | `sh/tables.sh`, one command, regenerates `out/table-*.md`, `out/cli-*.tsv`, `out/analysis/*` (every fact below) and `out/verdicts/` (every CLI run's verdict, gzipped) from the records in `~/.cache/aet-4400j` |
| Dumps | `dotnet-dump` 9.0.661903 (NuGet, sha512 matches the catalog: `out/dump/dotnet-dump-nuget-sha512.txt`) on the .NET 8.0.12 runtime (sha512 matches Microsoft's release metadata). Its `libcoreclr.so` equals Renode's (`8347a634…`, `out/rep/dump/dac-match.txt`). `createdump` and the DAC were copied beside Renode in the replay's container only. The dumps (1.29 and 1.98 GB) stay in the cache; their sha256 is in `out/dump/dumps.sha256` |

## 1. The log, by source and virtual second

### 1.1 What the 8 MiB budget keeps (CLI path, budget lifted)

No input, 12 s (`L-s03-12000-1`, `py/flood.py`), bytes per virtual second:

| source | 0–3 s | 4 s | 5–11 s, each | what it is |
|---|---|---|---|---|
| `NOISY spi2.spi2mux` | 0 | 302,080 | 809,744–810,512 | the mux's `GPIO #0 set to True/False` and `Finisihing transmission` lines, three per transaction |
| `WARNING spi2` | 0 | 175,756 | 471,053–471,416 | the CLI's own `AETHELGARD-CFG2` hook, two per transaction |
| `ERROR spi2` | 0 | 165,528 | 443,802–444,144 | Renode's slave-mode ERROR, one per transaction |
| `NOISY spi2.spi2mux.u5` | 0 | 167,749 | 436,323–436,767 | the ICM-42688-P's register and FIFO reads |
| `WARNING adc3` | 91,540–92,000 | 92,000 | 92,000 | `Issued a start event before the last sequence finished`: not in `4400h`'s list; new on this board |
| everything else | ≤ 70,000 | — | ≤ 13,000 | U7, U4 at boot, the USARTs' access logs |
| **total** | 104,474–183,447 | 915,063 | **2,264,865–2,266,781** | kept at 12 s: **17,374,097 B (207 %)** |

**With the input,** 12 s (`L-s03-12000-in-1`): **56,072,163 B (668 %)**.
- The SPI flash model (U4, W25Q64JV) is **37.3 MB of it**:
  - a 15.7 MB burst at 5 s;
  - then **3.2–3.4 MB per virtual second, steady**: ArduPilot's dataflash logging after its `Chip erase` (`Transmitting data` and `Handled command: Operation: Program`, one each per byte).
- The IMU's four sources are as above.
- `INFO usart1` (the console's `ReceiveData` reads) adds 26–85 KB/s.
- Renode's raw console, nothing summarised (replay `stats`), carries **12.40–12.42 MB per virtual second** from 5 s. Most of it is `Unhandled write`, which is already summarised.

### 1.2 The shape of each source: what a summary may elide

**The D4 cycle** (`py/cycle.py`): on each SPI controller, every transaction is one sequence.
- The sequence is `CFG2 MASTER 1` → device lines → `CFG2 MASTER 0 (CR1.SPE 0)` → slave-mode `ERROR` (`MDS0E`).
  - spi2: 28,673 times.
  - spi1 (with input): 1,977 times.
  - Plus one `D` at each controller's start.
- CFG2 writes take **two value tuples per controller**, `(0x20400000, 0x0)` and `(0x0, 0x0)`. Every ERROR is D4-exempt.

**Distinct exact texts** (timestamp dropped, every value kept; `py/distinct.py`, input run after 5 s):

| template | lines | distinct texts |
|---|---|---|
| U4 `Transmitting data #, current state: HandleCommand` | 161,141 | 256 |
| U4 `Handled command: Operation: Program, …, returning #` | 160,512 | 256 |
| spi2 `AETHELGARD-CFG2 # #` | 54,510 | 2 |
| spi2 slave-mode ERROR; mux `GPIO #0 set to True` / `False`, `Finisihing transmission` | 27,255 each | 1 each |
| U5 `bank 0 read # -> # #` | 13,295 | 2 |
| **U5 `bank 0 read 0x30 -> <16 bytes>` (FIFO)** | 12,590 | **10,792** |
| adc3 warning | 7,000 | 1 |

**What the judge reads in each:**
- The device steps match exact line text, found from the start of the device's own lines. Some steps are several consecutive lines: U4's identity is four.
- D4 reads every CFG2 (bits `MASTER` and `SPE`), every ERROR, and "a device line on the controller", in order.
- The device-warnings rule reads a declared device's `[WARNING]`s.
- Nothing reads FIFO bytes past `reads[k]`'s first match ("at least 16 bytes").

**The kept log under each scheme** (`py/residual.py`, applied to the real logs):

| scheme | 12 s, input | 12 s, no input |
|---|---|---|
| today | 56,051,844 B (668 %) | 17,353,920 B (207 %) |
| exact text per listed source (first kept, later counted) | 6,628,687 B (79 %); the 1,024-key cap reached | 2,214,440 B (26 %) |
| **the same, and U5's FIFO reads by template after its data step is witnessed** | **966,815 B (12 %)**, about 43–75 KB per second from 6 s | **203,234 B (2 %)**, about 12 KB per second |

The last row is reached at the 1,024-key cap, but the cap costs little.
- From 6 s, lines kept past the cap are 31,807 B (U4).
- The rest is lines the scheme does not list: USART1's `ReadByte` access lines (213,233 B), U7 (50,280 B) and USART2 (23,353 B) (`out/analysis/residual-12-in.txt`).
- `py/residual.py` models a simpler scheme than D2 as worded below: no cycle unit, and no wait for multi-line steps. It is a bound on what D2 keeps, not D2 itself.

## 2. RSS, and what holds it

### 2.1 The CLI path as main runs it (stdin `/dev/null`, bounds lifted to read past them)

| input | window | n | verdicts | wall s | peak KiB (‰ of cap) | kept log B (% of 8 MiB) | armed |
|---|---|---|---|---|---|---|---|
| no | 5 s | 4 | pass ×4 | 12.3–13.6 | 977,664–1,038,072 (622–660) | 1,514,994 (18 %) | |
| no | 8 s | 3 | pass ×3 | 25.3–25.4 | 1,507,280–1,626,940 (958–1,034) | 8,312,197 (99 %) | |
| no | 10 s | 3 | fail: `register_mismatch` ×3 (§4) | 31.7–33.5 | 1,807,780–1,983,232 (1,149–1,261) | 12,844,205 (153 %) | |
| no | 12 s | 3 | pass ×3 | 38.8–41.9 | 2,141,296–2,388,564 (1,361–1,519) | 17,374,097 (207 %) | |
| yes | 5 s | 3 | pass ×3 | 15.7–16.4 | 1,042,568–1,069,312 (663–680) | 3,800,974–3,801,277 (45 %) | no |
| yes | 8 s | 3 | pass ×3 | 33.7–43.2 | 1,846,872–2,186,736 (1,174–1,390) | 33,646,820–33,646,862 (401 %) | no |
| yes | 10 s | 3 | fail: `register_mismatch` ×3 (§4) | 43.1–43.7 | 2,282,580–2,424,052 (1,451–1,541) | 44,979,796–44,980,012 (536 %) | **yes** |
| yes | 12 s | 3 | pass ×3 | 53.0–54.5 | 2,762,440–2,927,700 (1,756–1,861) | 56,072,121–56,072,250 (668 %) | **yes** |

**Under main's own bounds at 12 s** (no lift): `memory_cap_exceeded` both ways, killed at 1,573,344 and 1,576,240 KiB after 24.6 and 25.6 s of wall.

### 2.2 By virtual second (replays; MiB)

`stats`, `stats-in`: probes only. `fullgc`: each probe also forces a full collection. `stats-holdin`: stdin held open.

| s | RSS | RSS anon | GC heap | live after full GC | RSS, input | GC heap, input | RSS, held stdin | GC heap, held stdin |
|---|---|---|---|---|---|---|---|---|
| 0 | 557 | 470 | 312 | 304 | 597 | 319 | 476 | 199 |
| 2 | 716 | 628 | 466 | 441 | 805 | 508 | 497 | 201 |
| 4 | 806 | 717 | 554 | 529 | 925 | 623 | 498 | 201 |
| 5 | 895 | 807 | 642 | 609 | 1,027 | 726 | 500 | 202 |
| 6 | 1,054 | 966 | 794 | 761 | 1,270 | 966 | 502 | 202 |
| 8 | 1,361 | 1,272 | 1,097 | 1,065 | 1,658 | 1,350 | 506 | 202 |
| 10 | 1,663 | 1,575 | 1,393 | 1,369 | 2,029 | 1,718 | 513 | 202 |
| 12 | 1,970 | 1,881 | 1,697 | 1,681 | 2,391 | 2,078 | 515 | 202 |

Every second is in `out/rep/*/stats.txt`.
- **It is all live managed memory:** from 6 to 12 s, RSS +916 MiB, the GC heap +903, and live after a forced full GC +920.
- File-backed RSS is flat at 88 MiB.
- Gen-2 collections: 5 → 6 in the whole run (18 when forced).

**It is paced by wall time, not virtual time** (`py/wallrate.py` on the wall-stamped `rss.txt`):
- With `/dev/null`, after Renode's start-up, RSS grows **42.3–50.9 MiB per wall second in every 5 s slice**: with and without the input, before and after the IMU starts.
- With stdin held open: 0.5–2.0 MiB per wall second.
- The "per virtual second" slopes (152 MiB without input, 187 with) are only wall seconds per virtual second: the input run is slower.

### 2.3 The dump: what holds it

Two dumps of the `dump` replay, at 6 s and 10 s. At each point the machine was paused and two compacting full collections were forced first.
- `dumpheap -stat` diff (`out/dump/diff-t6-t10.txt`): the heap went from 840,031,942 to 1,519,217,739 B (+679,185,797).
  - **`ConcurrentQueueSegment<Int32>+Slot[]` +679,479,192 B (+81 arrays): 100.0 % of the growth.**
  - The next type grew by 2,348 B.
- One `ConcurrentQueue<Int32>` holds every segment (`out/dump/t10-queues.txt`).
- **Root** (`out/dump/t10-gcroot.txt`), a strong handle:
  - `… Antmicro.Renode.Core.Machine → … PythonPeripheral → … IronPython PythonContext → … Antmicro.Renode.UserInterface.Monitor → AntShell.Shell → AntShell.Terminal.NavigableTerminalEmulator → Antmicro.Renode.UI.ConsoleIOSource → BlockingCollection<Int32> → ConcurrentQueue<Int32>`.
  - `gcroot` printed this chain and the start of a second; then `dotnet-dump` exited 139. The first chain is whole.
- **The items** (`out/dump/t10-arr.txt`): every slot read (0–5 and 500,000–500,002 of a 1,048,576-slot segment) holds `Item -1` with consecutive sequence numbers.
  - `ConsoleIOSource.isInputRedirected` is `true`; the collection is unbounded (`_boundedCapacity -1`).
  - At 8 B a slot, 679 MB is about 85 million EOFs in 4 virtual seconds (about 13 s of wall): **about 6 million per second, one core spinning on `read()` at EOF.**

**The fix, measured.**
- **Replay** (`HOLDIN=1`): stdin a pipe held open and never written. RSS is flat at 476–515 MiB to 12 s, and the GC heap at 199–202 MiB (§2.2).
- **CLI path** (`AET4400J_HOLDIN` in `measure.patch`): the CLI holds the pipe's write end, unwritten, until the reaper is waited for. n = 3 per cell:

| input | window | verdicts | wall s | peak KiB (‰ of cap) | kept log B | armed |
|---|---|---|---|---|---|---|
| no | 5 s | pass ×3 | 12.1–12.5 | 547,604–553,808 (348–352) | 1,514,994 | |
| no | 8 s | pass ×3 | 21.4–22.0 | 555,860–568,688 (353–362) | 8,312,197 | |
| no | 10 s | fail: `register_mismatch` ×3 | 27.9–29.2 | 558,900–576,052 (355–366) | 12,844,205 | |
| no | 12 s | pass ×3 | 34.6–35.3 | 558,612–569,368 (355–362) | 17,374,097 | |
| yes | 5 s | pass ×3 | 14.3–14.9 | 562,816–570,096 (358–362) | 3,800,974 | no |
| yes | 8 s | pass ×3 | 28.0–30.0 | 574,844–582,604 (365–370) | 33,646,733–33,646,820 | no |
| yes | 10 s | fail: `register_mismatch` ×3 | 36.3–37.0 | 575,744–584,568 (366–372) | 44,979,796 | yes |
| yes | 12 s | pass ×3 | 44.2–45.7 | 573,752–583,412 (365–371) | 56,072,034 | yes |

Under main's other bounds (memory cap and log budget not lifted), held stdin at 12 s is **no longer killed by the cap** (366 and 372‰). It fails `renode_log_flood` / `foreign_not_booted` instead: the log is what binds (§1).

**What the fix changes besides memory: nothing** (`py/same.py`; WO §5, "a growth fix that changes behaviour").
- Verdict against verdict, `/dev/null` against held, in all eight cells (`out/analysis/same.txt`).
- With no input, only `memory.*` differs.
- With the input, `memory.*` and `log_kept_bytes` differ (by 129–303 B).
- The kept logs, with the scratch path masked and Renode's host-timing line `Thread has been woken up 2 times, verify your condition` dropped, are **byte-identical** (0 differing lines).
- Those bytes are that host line (no source, so no rule reads it) and the scratch directory's name. Between `/dev/null` runs the same cell already varies (10 s with the input: 44,979,796–44,980,012 B). F1 below.
- **Read, not measured:**
  - That the bridge's Renode inherits the held stdin (`bridge/src/main.rs:429`, `Command::status()` inherits).
  - No generated or droop boot was run with the fix.
  - One mechanism was tried: a held pipe. Others, such as Renode run without `--console`, were not.
- Wall is 2–19 % shorter with the fix (one core no longer spins): 12 s with the input, 53.0–54.5 → 44.2–45.7 s.

**What else this explains** (each consistent with it; none re-measured here):
- `4400f` F1 ("RSS follows wall time");
- `4400h` F1 (5 s killed at load 22–28) and its core-count effect;
- Tviburi `TVI-WO-1301`'s 768,856–1,049,556 KiB spread across CI runs, which `RUL-13`'s 1.5 GiB amendment cites as "a cause not yet known";
- `architecture.md:300`'s hang "while Renode grew at about 55 MB/s";
- possibly `4400h` F10 and F15 (the reboot's and DShot's out-of-memory under a 4 GiB cap).

The memory share in every Renode verdict has depended on the host's speed. With the fix it follows virtual time.

## 3. Wall time

- **On Furnace, 4 cores pinned, load 6–8, held stdin:** 5 s takes 12.1–14.9 s; 12 s takes 34.6–35.3 s with no input and **44.2–45.7 s with it**. From 5 s each virtual second takes about 3.2 s (no input) and 4.3 s (input).
- **CI against Furnace on this path, measured.**
  - CI's 5 s runs of `sentinel-02-own` (`no-imu-line`, `late-panic`, `late-hardfault` in `ardupilot-image` runs 45663731874, 45620143404 and 45531323358): **13.4–15.1 s** of wall between consecutive ANSWER lines; `input-arm` 17.8–19.0 s. The first case, `positive`, has no earlier ANSWER, so its own wall is not in the log.
  - The `positive` case on Furnace (`R0`, `/dev/null`, 4 cores): **12.5–12.9 s**. **CI ≈ 1.04–1.2× Furnace on the foreign path: an estimate.** The firmware differs (CI's planted and no-IMU ELFs against Furnace's `positive`), the Furnace side is n = 2, and CI's runner model is not recorded (`out/analysis/ci-wall.txt`).
  - Furnace's ELF is `4400h`'s copy of CI's `5eace4ca`; today's CI ELF may differ in bytes.
- **Estimate for 12 s armed on the slowest runner:** about 55 s at the measured 1.2×. It stays under the 120 s guard up to 2.6× Furnace. `1918`'s 1.5–3.5× for generated boots under parallel tests would put its top at 160 s; the foreign job runs alone, one call at a time.

## 4. The window's end (F4)

**The CLI's own end reads every 10 ms from 4 to 12 s** (replays `reads`, `reads-in`, `py/endreads.py`; each register judged by the verdict's own rule), 801 steps:

| register (rule) | no input | input | what it is |
|---|---|---|---|
| USART2.CR2 = 0x2000 (STOP bits 2) | 222 steps: 4.00–4.20 s, **8.22–10.22 s** | the same | ArduPilot's RC-protocol search on the CRSF port, no receiver |
| RCC.APB1LENR, SPI2EN clear | 62 (to 4.58 s; then 4.65, 10.08, 10.51 s) | 61 | SPI2's clock gated between transactions |
| GPIOB MODER/AFRH, PB13 output, AF0 | 6 single steps (4.65, 6.11, 9.09, 10.08, 10.51, 11.27 s) | 4 | SPI2 stopped: SCK as GPIO (`4400h` F4) |
| GPIOA MODER/AFRL, PA5 output, AF0 | 0 | 10 single steps | SPI1 (the flash) stopped |
| RCC.APB2ENR, SPI1EN clear | 0 | 3 single steps | SPI1's clock gated |
| ICSR `VECTACTIVE` ≠ 0 (`core_in_exception`) | 11 steps (vectors 66, 11) | 3 steps (66, 75) | the core in a handler at the end |

- **The RC search** (`AP_RCProtocol.cpp` at the pinned image, `serial_configs` L336–352, `check_added_uart` L359–396): six configurations, the next every 1 s while no protocol is detected. The two SBUS rows are 8E2, so STOP = 2 for **2 s of every 6 s**.
  - Read from the kept log's CR2 writes: 2.206–4.210 s and 8.218–10.222 s; next about 14.2–16.2 s.
  - Deterministic: the same phases in every run.
  - It is a firmware state of seconds, not a transient: **no repeated read cures it.**
- **PB13's episodes** (replay `gpiob`, GPIOB's access log): 28,708 in 12 s, one per SPI2 transaction. Each lasts **at most 101.4 µs** (median under the log's resolution), 36.7 ms in all.
- **Per quantum**, 11–12 s with the input (replay `quantum`: ICSR, GPIOB `MODER` and `RCC.APB1LENR` read at every 100 µs, 10,001 steps; `out/analysis/quantum-11-12s-in.txt`):
  - a handler active at 204 steps (201 episodes, the longest **2 quanta**), including 11.9999 s, as the CLI run found;
  - PB13 as output at 290 steps (285 episodes, the longest 2);
  - SPI2EN clear at 106 steps (each 1 quantum).
  - So a rule "fails only if it fails at every one of k reads one quantum apart" needs k ≥ 3; k = 4 leaves one quantum of margin.
- **USART2.CR1 in the SBUS phase:** ArduPilot writes `0x2000153D` (`PCE`, `M0` set: 8E2), but **the twin reads back `0x10D`**. Renode's USART model drops `PCE` (`out/analysis/cr1-sbus.txt`; the kept log's CR1 writes).
  - So the frame rule (CR1 mask `0x609`) never failed here, but on silicon it would, in the same 2 s of every 6.
  - The model's dropped `PCE` is a gap of its own: F2 below.
- **At the four lengths:** 5, 8 and 12 s are clean. **10 s fails USART2.CR2** in all 12 CLI runs (`register_mismatch`, with and without input, either stdin).
- **One quantum either side of 12 s** (held stdin):
  - no input: 11.9999 s pass, 12.0001 s pass;
  - input: **11.9999 s fails `core_in_exception`**, 12.0001 s passes, 12 s passes ×3.

  So today's single end read does not meet "the same verdict at the ruled length and one quantum either side".

## 5. The window proposed, and its margins

**`FOREIGN_MAX_VIRTUAL_NS` = 12 s, with the growth fixed (D1), the flood summarised (D2) and the end read repeated (D4).**

| bound | at 12 s, armed input, measured | margin |
|---|---|---|
| the arm | armed heartbeat at **8.752 s** (`stats-in`'s TDR stamps, offset 7,383, the verdict's witness byte) | 3.25 s |
| 1.5 GiB cap (`RUL-13`) | 573,752–583,412 KiB, n = 3 (held stdin) | **63 %** (371‰) |
| 8 MiB log budget | 56,072,034 B today → about **966,815 B** under D2's scheme (estimated on the real log, not built) | **88 %** |
| 120 s hang guard | 44.2–45.7 s on Furnace; about 55 s at CI's measured 1.2× | 54 % at 1.2×; holds to 2.6× |
| the end | 12 s lies in the RC search's 8N1 phase `[10.222, 14.2)`; ±1 quantum gives one `core_in_exception` without D4 | needs D4 |

- **Without D1, no length that holds the arm fits:** at 8 s the cap is already at 958–1,390‰. That would be §7's stop, but the holder is ours.
- **Without D2,** 8 s is the longest window inside the budget with no input (99 %), and 5 s with it (45 %).

## 6. Decisions for the ruling

**D1. The growth's fix, and its scope.** Renode's stdin becomes a pipe the CLI holds open, unwritten, until the tool's reaper is waited for. The tool then reads no EOF. Measured above.
- **(a) Renode runs only** (`simulate --firmware`, `--foreign`, and the bridge's Renode, which inherits it), by a per-run choice in `run.rs`. **Lean (a).** Another tool that reads stdin would block on a held pipe until the 120 s guard, where today it reads EOF.
- **(b) Every tool's stdin.** Simpler, but it changes KiCad, ngspice and cargo, which this did not measure.
- **(c) Foreign boots only.** It leaves the generated boots' host-dependent share (Tviburi's spread) in place.
- Either way, `run.rs` is outside this WO's literal fence ("the CLI's Renode configuration" is inside it). I ask for the grant.
- **The upstream row:** Renode 1.17.0's `ConsoleIOSource` queues EOF without bound when stdin is redirected and at its end (the dump, `out/dump/`).
- `RUL-13`'s 1.5 GiB stays. Whether its "cause not yet known" note is amended is the architect's.

**D2. The flood's summary.** Today's `Unhandled write` summary has only a key, the first line and a count. Lean, for the foreign filter:
- **(i) The D4 cycle,** per controller.
  - A held `CFG2 MASTER 0 / SPE 0` plus its ERROR is counted as one exempt cycle when the controller's next event is `CFG2 MASTER 1`. That is the judge's own rule, run online.
  - Anything else is released verbatim, so a non-exempt ERROR is never summarised.
  - The first and last cycle are kept verbatim; the count is added to `spi_errors_exempted`.
- **(ii) Each declared device's lines and its mux's:** by exact text (first and last kept, counted), beginning only once that device's multi-line steps are found. U4's four-line identity must stay contiguous.
- **(iii) U5's FIFO reads (`bank 0 read 0x30`):** by template, values elided, once its `reads[k]` step is witnessed.
- **(iv) Renode's own model warnings of undeclared peripherals** (adc3): by exact text, as `Unhandled write` is.
- A device's `[WARNING]` is never summarised (the device-warnings rule).
- Each source gets a planted test whose 1,000th line differs from its first in a judged field (WO §5).
- **Alternative:** (ii) by template from the start. It is cheaper, but it hides a field the judge reads. Refused by the adversarial case.

**D3. The window.** 12 s (§5). The next clean RC-search phase on this firmware is `[10.222, 14.2)`. An end of 13 s holds 1.2 s more margin past the arm at about +7 s of wall. **Lean 12 s.**

**D4. The end.**
- **(i) The end read repeated four times, one quantum apart, ending at the ruled end** (end − 3q, − 2q, − 1q, end), never past it.
  - The archive's transmission ends at `end_ns` = the window. Reading past it would capture bytes past the window.
  - The arm and the log are judged over the window as today.
  - A register, or `core_in_exception`, fails only if it fails at all four.
  - Measured episodes are at most 2 quanta.
- **(ii) A declared RC port's frame not judged at the end** while ArduPilot searches: `CR2`'s STOP bits and `CR1`'s frame bits (`PCE`, `PS`, `M0`), which silicon would also show in the SBUS phase. Name it from `AP_RCProtocol.cpp`.
  - Its SWAP and inversion options stay judged from the access log (`serial_options`).
  - Otherwise every window must avoid two seconds in six: 10 s never passes on this board.
- **Lean (i) + (ii).**

**D5. Where the acceptance run lives.** CI builds no Sentinel-03 ELF.
- **(a) In the spike** on Furnace, from this ELF, as `4400i` did. CI's foreign job keeps `sentinel-02-own`.
- **(b) Add `sentinel-03` to `ardupilot-image.yml`'s build.** Outside my fence; it also needs the derived `foreign_clock` in a committed spec.
- **Lean (a)** for this WO, with (b) as a row.

## 7. What would move

- **No committed package or golden:**
  - The fab packages record no memory (`grep` of `fab/`).
  - `ci/foreign-judge.sh` asserts `peak_rss_kib < bound_kib` once (`late-hardfault`), which still holds.
  - The tier-1 memory tests plant `yes | dd` stubs that never read stdin.
  - The tier-2 tests assert only `peak > 0` and the cap's absence.
  - A campaign records a peak only when the cap fired.
- **Every Renode verdict's `memory`** (peak and share) drops: Sentinel-03 at 5–12 s from 977,664–2,927,700 to 547,604–584,568 KiB; `sentinel-02-own` at 5 s from 1,000,668–1,010,996 to 659,408 KiB (n = 2 and 1).
- **D2 and D4:**
  - D2 moves every foreign verdict's `log_kept_bytes` and `warnings`, and adds summary fields to the verdict schema.
  - D4 adds the repeated reads and the RC exemption to the rule table text (`RULES`), which `ci/foreign-judge.sh`'s cases read only by `kinds`.
  - Each is named again at READY against the 14 generated boards' verdicts, which never take the foreign filter.
- **`FOREIGN_MAX_VIRTUAL_NS`, 5 → 12 s.** Its readers:
  - `boot_tier1.rs:3151` refuses `5001ms`; it becomes the ceiling + 1.
  - `boot_tier1.rs:2867` checks the window's name, unchanged.
  - `renode.rs:7370`'s rendering test strides 9,973 ns to the ceiling: about 2.4× its iterations.
  - The CLI README names the constant, not 5 s.
  - `ci/foreign-judge.sh` hard-codes `window=5000ms` (L76) and passes `--foreign` per call. **Its cases stay at 5 s** unless the ruling moves them. Each moved to 12 s costs about 2.7× its wall on CI: 13–15 s → about 40 s.
  - Only the acceptance's armed run needs 12 s (D5).
- **The trace and the archive: no record changes.**
  - D2 adds verdict fields only.
  - D4 reads before `end_ns`.
  - The input archive (kinds 38–40) holds the input, the stimuli and the transmission, not the log or the end reads. No trace version bump, so no bench-package verdict moves through a trace (`4400i` F16's path).
  - A raised ceiling refuses no archived run.

## 8. Predictions, graded

| # | prediction | graded |
|---|---|---|
| P1.1 | boots at 5 s and passes; no clock contradiction | **right** |
| P1.2 | 100 MHz USARTs carry the input | **right:** armed |
| P1.3 | ICM's first FIFO read 4.3–4.5 s | **wrong:** 4.636 s (identity 4.585 s) |
| P2.1 | under 0.4 MB/s before 4.4 s | **right without input** (0.10–0.18); with input a 2.1 MB flash read at 3 s |
| P2.2 | 2.0–2.5 MB/s from 4.4 s, the shares, 16–21 MB at 12 s | **right:** 2.27 MB/s; mux + ICM 55 %, CFG2 21 %, ERROR 20 %; 17.37 MB. adc3 (92 KB/s) not foreseen |
| P2.3 | flash second-largest, in bursts; 45–65 MB | **wrong in shape:** largest (37.3 of 56 MB), steady 3.3 MB/s after one burst; total right |
| P2.4 | one template per source; ≤ 4 CFG2 tuples per controller | **right:** 2 tuples, one sequence |
| P2.5 | raw 10–14 MB/s | **right:** 12.4 |
| P3.1 | flat until the FIFO, then 90–130 MiB/vs | **wrong:** it grows from the start, paced by wall time |
| P3.2 | input slope within 15 % | **wrong:** +23 % (a slower run, more wall per vs) |
| P3.3 | the cap passed at 9.5–11 s; 12 s 1.75–2.1 / 2.0–2.5 GiB | **wrong:** passed by 8 s; 2.04–2.28 / 2.63–2.79 GiB |
| P3.4 | ≥ 70 % live managed; full GC frees < 10 % | **right:** about 100 %; under 1 % |
| P3.5 | core count moves the level | **not measured** (every run on 4 cores) |
| P4.1 | 1–3 types carry > 60 % | **right:** one type, 100.0 % |
| P4.2 | ranked holders (a)–(f) | **wrong:** none of (a)–(e). The monitor's console input queue, my "something else" at 0.05 |
| P4.3 | rooted at an IMU-bus peripheral | **wrong:** rooted at the monitor's terminal |
| P4.4 | Renode's own, not configurable | **wrong:** Renode's defect, triggered by our stdin; configurable away |
| P5.1 | 5 s 9–13 s; 2–3 s per vs; 12 s 25–35 s | **5 s right** (12.1–13.6); **per vs wrong** (3.2–4.3); 12 s 34.6–35.3 held, 38.8–41.9 `/dev/null`: **wrong** |
| P5.2 | 12 s on CI 40–120 s; little margin at the top | **right in range** (about 55 s at the measured 1.2×); the margin is wider than predicted |
| P6.1 | PB13 an SPI2-release transient under 5 % | **right:** 0.3 % of time, ≤ 101.4 µs |
| P6.2 | 5, 8 clean; 10, 12 clean at p 0.9 | **10 s wrong:** USART2's RC search, not PB13 |
| P6.3 | none of the ±1-quantum ends fails | **wrong:** 11.9999 s input fails `core_in_exception` |
| P7.1 | no window fits the cap with the growth as found | **right** (8 s is over); but the holder is configurable, so not §7's stop |
| P7.2 | with the growth removed, 12 s fits with over 40 % margin; the log only if the IMU and flash sources are summarised | **right:** 63 %; 12 % of the budget after summary |
| P7.3 | arm at 9.4–10.2 s | **wrong:** 8.752 s |
| P7.4 | no committed package moves | **right as far as read** (§7); confirmed at READY |

## 9. Findings for FEEDBACK

| # | finding | proposed owner |
|---|---|---|
| F1 | **`log_kept_bytes` is host-dependent:** the scratch directory's name is in Renode's echo of its own command, and Renode logs `Thread has been woken up N times` on host timing. Both are kept lines (0–303 B between runs of one cell). No rule reads either, but the field is in every foreign verdict | the architect: a row (mask or drop both from the kept log) |
| F2 | **Renode 1.17.0's USART model drops `PCE` and `M0` on read-back** (`0x2000153D` written, `0x10D` read), so no parity or word-length change is visible to the twin's frame rule | a row beside `4400i` F8's model gaps |
| F3 | **Renode 1.17.0's `ConsoleIOSource` queues EOF without bound** when stdin is redirected and at its end (§2.3) | an upstream row, with the dump |
| F4 | **The memory share in every Renode verdict has depended on host speed** (§2.2), with Tviburi's spread and `RUL-13`'s 1.5 GiB note in its history | the architect: `RUL-13`'s note, if D1 lands |

## 10. Process notes

- **Two runs voided by my own edits:**
  - `series.sh` was edited while `L-s03-12000-in-1` ran; the run itself was complete, only the script's tail broke. It is kept, and matches `L2` and `L3`.
  - `cli.sh` was edited while `Q-s03-12000100us-in` ran. That run is discarded and was rerun.
  - The same lesson as `4400i`'s memory note: copy, then edit.
- **The first dump attempt failed:** `createdump` needs the DAC beside it. Fixed in `sh/replay.sh`.
- **The first held-stdin replay hung** in the script's final `wait` on the held `sleep`. Its stats are complete (`out/rep/stats-holdin`); the container was killed by its id.
- The no-input 10 ms end-read replay passed Renode's 128 KiB argument limit; its stepped commands now go in a file.
- Downloads, both checked against their publisher's hash: .NET 8.0.12 runtime and `dotnet-dump`. They are used only inside capped containers with no network.

## 11. The build, to the ruling (2026-11-25T18:49Z; the acceptance question (a) at 19:33Z)

Built on the branch, one commit per decision:

| commit | what |
|---|---|
| D1 `8ce1ed6c` | Renode's and the bridge's stdin is a held pipe |
| D4 `70830466` | four end reads; the RC fields |
| D3 `cf84b402` | 12 s with an input |
| D2 `b4fde539` | the summary |
| `a03534e8` | the adversarial pass's fixes |
| `5c6f9cc1` | the reduction pass |
| `cf7b45c8` | the test weights |

Evidence is in `out/build/`.

### 11.1 The acceptance runs

Main's Sentinel-03, ELF `97afa171…` (Copter-4.7.2, built by CI's `container/ardupilot-build.sh` in the pinned image `e1809b90…`, `sh/board.sh`; `out/build/elf.sha256`), with `4400i`'s `arm12.json`. Four cores, held stdin; nothing lifted unless the run says spike.

| run | which | verdict | armed | peak (‰ of 1.5 GiB) | log kept (of 8 MiB) | trace sha256 |
|---|---|---|---|---|---|---|
| 11.9999 s | **CLI**, the branch's `simulate --foreign 11999900us --input` | pass | yes | 347‰ | 895,449 B (11 %) | `4bcedee0…` |
| **12 s** | **CLI**, the branch's `simulate --foreign 12000ms --input --archive` | **pass** | **yes** (`base_mode` 209, offset 7,383) | 424‰ | 895,449 B (11 %), 0 dropped | `ee330dfa…` |
| 12.0001 s | **spike**: the branch plus `patches/plus-one-quantum.patch` (the input ceiling one quantum higher), since the CLI refuses past its ruled 12 s | pass | yes | 364‰ | 895,449 B | `231e2b19…` |
| 5 s, no input | CLI | pass | — | 348‰ | 108,786 B | — |

- **The window's end gives one verdict:** pass at 11.9999 s, 12 s and 12.0001 s. Before D4, 11.9999 s failed `core_in_exception` (§4).
- **Replay:** `replay --archive` of the 12 s run re-judges its input from the trace alone (`input_verdict` pass, `out/build/replay.json`).
- **Byte for byte:** three separate 12 s runs, by three builds of this branch (`accept-12-in`, `accept4096-12-in`, `F-12000-in`), wrote a byte-identical trace (`ee330dfa…`) and manifest (`3dc5286e…`; `out/build/archives.sha256`).
- **The peak spread:** 347–424‰ across these runs (352–371‰ in §2.3's), the same bimodal ~100 MB `sentinel-02-own` showed at 5 s (564,024–659,604 KiB). The margin to the cap at worst is 58 %.

### 11.2 Against the unsummarised log, on real runs

- **12 s with the input:** the summarised run (`F-12000-in`) against the unsummarised, lifted-budget run of the same cell (`H1-s03-12000-in-1`).
- **5 s with none:** `F-s03-5000-1` against `H1-s03-5000-1`.
- **Identical:** every finding, every step's `observed_ns`, `spi_errors_exempted` (30,650 and 1,468), the serial options, the input's witness, the `Unhandled write` warnings and the locked accesses.
- **Added only:** D4's `rc_protocol_search` info.

### 11.3 The adversarial pass and its fixes

- A separate reviewer, given the code and no conclusions, found four ways the summarised log could read differently from the whole. All came from one design choice: a held pair let other lines pass ahead of it, so the kept log was reordered.
- **Fixed in `a03534e8`:** the summary never reorders; the kept log is the whole log less the lines dropped (FEEDBACK §Adversarial pass).
- **The oracle:** the judge's own reading (first unexempted error, exempt count, each device's warnings and step times; around the read blocks, nothing after the last) of the summarised log against the whole. It covers a Sentinel-03-shaped log and sixteen planted cases, five of them the reviewer's counterexamples.
- **Mutants:** one per guard, each killed:

| decision | killed |
|---|---|
| D1 | 3/3 |
| D4 | 6/6 |
| D3 | 4/4 |
| D2 | 15 of 16 (`out/build/mutants-d2.txt`) |

- **D2's surviving mutant is equivalent:** the first read's instant as `<` or `<=` in the timeline's split. An event at that instant runs before the same block either way.

### 11.4 The reduction pass

- `-61` net lines.
- Duplicates removed: the summaries' kept counter, always 1; the filter's decision passed to the summary, dead; two copies of the summary-to-row mapping; two copies of the run-to-each-event loop; `parts` built twice; the RC rule stated twice.
- Proposed, not taken: three `run.rs` plumbing merges, for a few lines each in the capture path; and folding the PC, clock and pull reads to the last block, which would stop validating the earlier blocks' values. FEEDBACK lists them.
