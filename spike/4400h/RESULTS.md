# AET-WO-4400h results

**STOP: §7, "the cap fails before 10 s"** (D1 rules whether this is that stop). What was measured at 10 s:
- all six logged runs passed `4400g` §1.4's line (a run over 900‰ of the cap stops it); the one no-log run read 851‰;
- two of the logged runs passed the cap, both with the cap raised to read the share;
- the three runs at the real cap, at load 3, were not killed by it (904–952‰);
- every 10 s run lost every witness to the CLI's 8 MiB log budget, which is exhausted at 9 s whatever the load.

**Proposed bound: keep 5 s.** Parts (2)–(4) are predicted (`PREDICTIONS.md`) and harnessed (`sh/`, `py/`; the plans are written by `py/plans.py`), but not run, because the stop came first. §4 gives what the twin lacks for them, from source.

Each number below is from a run whose command and record are in this directory.

## Inputs

| | |
|---|---|
| ELF | `5eace4ca…` (`out/elf.sha256`): CI's `generated-sentinel-02` from `ardupilot-image` run 45410032458 on main `29b3a97d`, Copter-4.7.2 `fb773567`. Same bytes as `AET-WO-2314`'s copy |
| Board | `sentinel-02-own` (`crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json` at `29b3a97d`, spec sha256 `e2d6526c…`). `motor_outputs` is `[]` and its hwdef has no PWM line |
| CLI | main `29b3a97d` plus `patches/window.patch` (window ceiling 30 s; `AET4400H_WALL_S`, `AET4400H_RSS_KIB` and `AET4400H_NO_USART_LOG`; unset, these are main's bounds). Built in the tools image `1e0ab105…` |
| Runs | `sh/window.sh` (one run), `sh/series.sh`, `sh/fine.sh`, `sh/ten.sh` (lifted, cap); `sh/capture.sh` and `sh/replay.sh` for the replays. Every container ran under `~/bin/capped-run` (8 GiB, swap 0, read back), one at a time, `nice -n 10` |
| Records | `out/win/<run>/` holds `row.json`, `env.txt`, `load.txt` (the host's load average each second) and `verdict.json`. `out/window.tsv` is the table (`py/table.py`); `out/replay/` holds the replays |

Shares are in ‰ of 1,572,864 KiB. "Lifted" means the CLI's wall bound was raised to 600 s, with or without the RSS cap raised to 6 GiB to read past it. `out/window.tsv` says which per run.

## 1. The window's cost

### 1.1 The curve

| window | log | bounds | runs (wall s, peak KiB, ‰, load) | verdict |
|---|---|---|---|---|
| 5 s | on | main's | `w05-a` 30.88, 1,386,768, **881**, load 22–28 | **killed: `wall_bound_exceeded`** |
| 5 s | on | main's | `f1-M-w5a` 9.91, 828,252, 526, load 7; `f1-M-w5b` 9.78, 809,484, 514, load 7 | pass |
| 5 s | on | lifted | `s1-L-w5` 19.10, 1,142,068, 726, load 26–28 | pass |
| 5 s | on | lifted, **2 cores** | `f1-T2-w5a` 10.66, 943,740, 600; `f1-T2-w5b` 11.19, 982,156, 624, load 7 | pass. Wall moves about 1 s but the peak rises 115–170 MB: core count moves RSS by more than wall time does (perhaps the .NET GC's heap count), a second, host-dependent part |
| 5 s | off | lifted | `s1-N-w5` 12.36, 844,256, 536, load 25–26 | fails `serial_options_unwritten` (the log is that judge's input) |
| 6 s | on / off | wall lifted | 12.2, 924,964, 588 / 12.22, 937,056, 595, load 10 | fails `register_mismatch` (§1.4) |
| 7 s | on / off | wall lifted | 14.49, 1,042,204, 662 / 14.06, 1,020,844, 649, load 9–10 | pass / `serial_options_unwritten` |
| 8 s | on / off | wall lifted | 16.51, 1,141,480, 725 / 16.49, 1,141,796, 725, load 9 | pass, log 8,115,924 B kept (97 % of the budget) |
| 9 s | on / off | wall lifted | 18.49, 1,233,296, 784 / 18.89, 1,270,428, 807, load 8 | **`foreign_not_booted`**: log over budget, 23,772 lines dropped |
| 10 s | on | lifted | `s1-L-w10` 30.66, 1,630,336, **1,036**, load 22–25; `t1-L-w10a` 28.61, 1,736,200, **1,103**, load 6–9; `t1-L-w10b` 24.78, 1,452,436, **923**, load 9–10 | `foreign_not_booted` (log) |
| 10 s | on | wall lifted, real cap | `t2-C-w10a` 22.73, 1,498,732, **952**; `t2-C-w10b` 21.32, 1,423,256, **904**; `t2-C-w10c` 21.92, 1,472,588, **936**, load 3 | `foreign_not_booted` (log); not killed by the cap |
| 10 s | off | lifted | `s1-N-w10` 20.93, 1,339,856, 851, load 18–22 | `foreign_not_booted` |
| 20 s | on / off | lifted | 59.75, 2,915,188, 1,853 / 44.23, 2,363,904, 1,502, load 14–18 | `foreign_not_booted` |
| 30 s | on / off | lifted | 89.38, 4,590,036, 2,918 / 63.75, 3,522,032, 2,239, load 11–18 | `foreign_not_booted` |

### 1.2 Three limits, in the order they bind

1. **The CLI's 8 MiB log budget, at 9 s, at any load.**
   - The kept log is 1.60 MB at 5 s, 3.77 MB at 6 s, 5.94 MB at 7 s and 8.12 MB at 8 s, so it grows about 2.2 MB per virtual second from the IMU's start.
   - Past the budget no witness is read whole, and the verdict is `foreign_not_booted`.
   - `2311`'s access log is almost none of it: 1.54 MB against 1.60 MB at 5 s.
2. **The memory cap and 900‰, at 10 s.**
   - Peak RSS grows about 100–120 MiB per virtual second at load 3–10, more under load.
   - 10 s is over 900‰ in 6 of 6 runs, at load 3 to 25.
3. **The 30 s wall bound, under load, already at 5 s.**
   - At load 22–28, main's own 5 s window took 30.88 s and was killed at 881‰. Another 5 s run at load 26–28 took 19.1 s (726‰) and would have passed: one kill in two runs above load 20 (n = 2).
   - At load 7 it takes 9.8 s.

### 1.3 The cause, by replay against replay

These are replays of the CLI's own Renode invocation (`sh/capture.sh`, `sh/replay.sh`; argv in `out/replay/capture-c16-argv.txt`), 16 s virtual, at load 2–3. The RSS measure is the tree's summed VmRSS at 100 ms, Tviburi (v)'s measure; it is compared only replay to replay.

| replay | change | wall s | peak KiB |
|---|---|---|---|
| `pure16`, `pure16b` | none | 36.45, 36.99 | 2,297,792, 2,353,932 |
| `quiet16` | `logLevel 3` before the run. Checked (`flood.txt`): it dropped the WARNING lines (`Unhandled write`, the CFG2 hook), but each device's own `logLevel -1` survived, so 1.70 MB per virtual second still printed | 34.67 | 2,282,452 |
| `quiet16b` | the same, plus `logLevel 3` on each device and mux. Only spi2's slave-mode ERROR remains: 0.44 MB/s against 12.3 | 31.65 | 2,109,272 |
| `nohook16` | the CLI's two SPI `CFG2` write hooks removed | 36.41 | 2,290,836 |
| `gc768` | `DOTNET_GCHeapHardLimit` = 768 MiB | 12.74 | 1,022,272, then **`System.OutOfMemoryException` at 6.362 s virtual** (`out/replay/gc768/oom.txt`) |

- **What fills the log** (`out/replay/pure16/flood.txt` and, without the summarised warnings, `flood-kept.txt`; `py/flood.py`):
  - From 4.4 s, when ArduPilot starts reading the ICM-42688-P's FIFO, the console carries 12.3 MB per virtual second.
  - About 10 MB/s of that is Renode's `Unhandled write` warnings (dma1 `LIFCR`/`HIFCR`/`FCR`/`CR`, spi2 `CR1`/`CFG1`/`CFG2`, rcc `SPI2LPEN`), which the CLI summarises by kind (`4400e` D7 (a)).
  - **The 2.17 MB/s the budget keeps is per-transaction lines of the IMU's SPI traffic, about 3,895 transactions per second:**
    - the CLI's own `AETHELGARD-CFG2` hook (0.47 MB/s);
    - Renode's `spi2: Attempted to set peripheral into SPI slave mode` ERROR, the MODF scar's model line (0.44 MB/s);
    - the spi2 mux's and the ICM model's NOISY lines, which the judge turns on to witness identity (1.25 MB/s).

  The judge needs each of these once.
- **What grows memory: not the log, not the hooks; at least partly live managed memory.**
  - Silencing all but 0.44 MB/s of the console lowers the 16 s peak by about 8 % (2,109,272 against 2,282,452–2,353,932 KiB). Removing the SPI hooks changes nothing.
  - A 768 MiB managed-heap limit ends in out-of-memory at 6.36 s, 2 s after the IMU starts. So the live managed heap passes 768 MiB there: at least that much of the growth is retained, not garbage awaiting collection. How much of the later RSS is managed and how much native was not split.
  - **It starts with the IMU traffic.** On main's bounds at load 4 (`e1-M-w2`, `-w3`, `-w4`), the peaks are 671,676, 685,488 and 749,512 KiB at 2, 3 and 4 s, then 809,484–828,252 at 5 s: nearly flat until the ICM is probed, then about 60–120 MiB per virtual second. Our ICM-42688-P model's buffers are bounded (`models/renode/ICM42688P.cs`: `txBytes` cleared at L96/L152; `fifo` drained at L665).
  - **What holds it is not found.** The candidates are Renode's SPI, DMA or unhandled-access paths. A heap dump is the next step (§5, D2).

### 1.4 A transient at the window's end

At exactly 6.000 s, with and without the log, the end-of-window read finds GPIOB `MODER` with PB13 (SPI2 SCK) as an output and `AFRH` AF0, not the solved AF5 (`register_mismatch`). 7 s and 8 s read clean. A window's end can land on a firmware transient: a longer bound needs its end placed, or the read repeated.

### 1.5 The proposed bound

**Keep `FOREIGN_MAX_VIRTUAL_NS` at 5 s and do not raise it in this state.**
- 8 s passed only off-peak (load 9), one run per cell, with the log at 97 % of its budget, so it fails the quiet-host test.
- 10 s fails 900‰ at any load.

A longer window needs, in order:
1. the per-transaction witness lines summarised after their first match;
2. the retained growth found and removed;
3. the wall bound raised with the window, since 5 s already reaches 30 s of wall at load 22–28.

Then measure again.

## 2. Arming with no defaults file: not run (the stop came first)

What is known, from source at the pins (spot-checked; `PREDICTIONS.md` P2.1–P2.7 has the line numbers):

- **The MAVLink port is SERIAL0 = USART1,** the board's only serial (`SERIAL_ORDER USART1`, MAVLink2 at 115200). It receives by DMA1 stream 0 (request 41), into a 64-byte bounce buffer (`UARTDriver.h`:25, `UARTDriver.cpp`:549–566).
- **How the twin injects into a foreign firmware's USART today: it cannot.** A monitor `WriteChar` queues the byte, but ArduPilot never reads it (TVI-WO-1302 §2.6, measured there). What the twin lacks:
  1. **The request line.** Renode's `STM32F7_USART` has a `ReceiveDmaRequest` output (`STM32F7_USART.cs`:87, raised at :109), which the emitted platform never connects. It needs `ReceiveDmaRequest -> dmamux1@41` in `usart1`'s block, as the twin already writes `DMARecieve -> dmamux1@N` for SPI (`platform.rs`:789–790).
  2. **IDLE.** `IDLE` and `IDLEIE` are tags (`STM32F7_USART.cs`:154, :231). ArduPilot hands a short frame to its driver on the USART IDLE interrupt (`UARTDriver.cpp`:576–590). Without it, only a transfer-complete at 64 bytes moves data (:606–622), and the harness pads each frame to 64 bytes (`py/frames.py`).
- **No parameter routes around it:** SERIAL0 has no `OPTIONS` parameter in 4.7.2 (`AP_SerialManager.cpp`:172–180), so `NoDMA_RX` cannot be set.
- **Neither is a defaults file or a rebuilt ELF;** both are twin changes. §7's third stop is not reached.
- **The arming path, as harnessed** (`py/plans.py` arm, P2.5):
  - `FRAME_CLASS` 1, `FS_THR_ENABLE` 0, `DISARM_DELAY` 0 and `INS_ACCOFFS_X` 0.001;
  - `MAV_CMD_PREFLIGHT_CALIBRATION` param5 = 76;
  - `MAV_CMD_COMPONENT_ARM_DISARM` with no force.

  4.7.2 has `ARMING_SKIPCHK`, not `ARMING_CHECK` (`AP_Arming.cpp`:201). A forced arm still runs the mandatory checks (:1885). The injected bytes are written by `py/frames.py` to `injected.tsv` on each run.
- **Persistence:**
  - A `PARAM_SET` is saved through `AP_FlashStorage` to page 14 or 15, which is bank 2 sectors 6–7 (`0x081C0000` and `0x081E0000`), in 32-byte flash words under `CR2.PG`, read back with memcmp.
  - The boot already erases and writes both sector headers on blank flash.
  - Renode's `STM32H7_FlashController` models bank 2, the keys, PG, 32-byte words and sector erase.
  - Renode's `SYSRESETREQ` resets the machine without clearing memory (`NVIC.cs`:1194; `MappedMemory.cs`:183–190), so the `dshot` plan reboots over MAVLink to test persistence inside one run.

  This is source only, not measured. It goes to `TVI-WO-1304` as such.

## 3. What arming reaches: not run

The queue builder (`sh/queue.sh`) runs Tviburi `TVI-WO-1150` (v)'s own readers, unchanged, from Tviburi main `d22dad8` (hashes in `out/tviburi-readers.sha256`). There are three controls:
- no injection;
- the `pad` plan: the same bytes, all zero frames;
- the `params` plan: the same parameters, no arm.

Monitor-issued accesses carry no PC and fall out as `judge` rows (`view2-writes.awk`).

**On this board, no TIM1 or TIM2 row can be reached:** it has no motor output (bus question, 2026-11-24, open).

## 4. A GUIDED command: not run

From source:
- GUIDED refuses once armed without a position estimate (`mode.cpp`:391–396). The twin has no GPS model, so `NAV_TAKEOFF` has no GUIDED to run in.
- GUIDED_NOGPS takes `SET_ATTITUDE_TARGET` thrust (the `arm` plan carries both).
- The outputs go to `Empty::RCOutput`: no register moves on this board.

**What the twin cannot say: anything about flight.** There is no vehicle dynamics, so the IMU and barometer do not respond to thrust.

**The bridge to ArduPilot's SITL, as a row:** it would need SITL's physics model driving the twin's IMU and barometer models through a sample feed with a declared clock relation, and the twin's PWM or DShot outputs fed back as SITL's servo inputs. That is a lock-step co-simulation across two time bases. Nothing is built.

## 5. The stop: decisions for the architect

- **D1. §7's cap stop fired,** on 4400g's 900‰ line at 10 s (6 of 6). The cap itself killed no run at load 3, but was passed in 2 of 3 runs at load 6–25 with the cap raised to read the share. **Lean:** the stop stands, and 5 s stays.
- **D2. The build work order I would write: "the foreign window's cost", a build with one measured stop.**
  - (a) The foreign log filter keeps the first line of each per-transaction witness kind and counts the rest, as it already does for `Unhandled write`:
    - the device models' NOISY transaction lines and the mux lines;
    - the `AETHELGARD-CFG2` hook lines, after the first per controller and value;
    - Renode's SPI slave-mode ERROR, already exempted and counted (`spi_errors_exempted`).
  - (b) Find what holds the growth (about 100–120 MiB per virtual second, at least part of it live managed memory), with a heap dump of Renode in the tools image. Remove it if it is ours (a model, a hook); name it upstream if it is Renode's.
  - (c) Raise the wall bound with the window.
  - (d) Measure again under load (two cores and three repeats per cell). Propose the new `FOREIGN_MAX_VIRTUAL_NS`; the architect rules it.
  - **Fence:**
    - `crates/aethelgard-cli/src/renode/foreign.rs` (the filter and the window; after `AET-WO-2314` merges);
    - `crates/aethelgard-cli/src/renode.rs` (the log filter, `BOOT_BOUNDS.wall` only if ruled);
    - `models/renode/` (if a model holds the growth);
    - their tests and `ci/foreign-judge.sh`'s window;
    - not the IR, not the memory cap's value.
  - **Stops:**
    - the growth is Renode's and cannot be removed;
    - a witness the judge reads is lost by the summary.
- **D3. The twin's USART receive, a second build: "a foreign firmware hears MAVLink".**
  - `ReceiveDmaRequest -> dmamux1@<request>` for every declared USART, from the topology (`aethelgard-twin` `platform.rs`).
  - IDLE in an `STM32F7_USART` overlay model of our own, as for the sensors, so no padding is needed.
  - A `--foreign` input of timed frames, recorded in the trace (`INV-04`), so a run replays with its injections.
  - **Fence:**
    - `aethelgard-twin/src/platform.rs` and its goldens;
    - `models/renode/` (the overlay);
    - `crates/aethelgard-cli` (the input surface);
    - tests;
    - not the IR.
  - It waits on D2, since the arm needs about 12 s.
- **D4. Run (2)–(4) now, in replay, outside the CLI's bounds** (the harness is committed), or after D2? **Lean: now.** The arming path and persistence are independent of the bound, are an input to D3's design, and the persistence result is `TVI-WO-1304`'s.
- **D5. The motored board (bus, open):** (3)'s TIM rows and DShot need a board with motors and an IMU, and no CI-built ELF has both.

## 6. Predictions, graded

| | prediction | measured |
|---|---|---|
| P1.1 | the wall bound binds first | **partly:** at load 22–28 the wall bound killed main's 5 s run. At load 3–10 the log budget binds first (9 s) |
| P1.2 | 2.4–3.2 s wall per virtual second | **wrong both ways:** 3.1–6.2 s per virtual second at load 22–28 (30.7 s and 19.1 s for 10 s and 5 s; 30.9 s for 5 s), about 2 s at load 3–10 |
| P1.3 | the RSS table | **wrong past 5 s.** 5 s is in range only off-peak (514–526‰ against 520–650), and 726–881‰ under load. 10 s read 851–1,103‰ against 620–780; 20 s 1,853‰ against 780–980; 30 s 2,918‰ |
| P1.4 | the log costs under 10 % | **wrong past 5 s:** 26 % at 5 s (load-confounded), 0 % at 8 s, 23 % at 20 s, 30 % at 30 s |
| P1.5 | no-log fails the serial judge and keeps `memory` | **right** (`serial_options_unwritten`) |
| P1.6 | the log stays under budget at 30 s | **wrong:** the budget is exhausted at 9 s |
| P1.7 | the cap stop does not fire | **wrong:** it fired at 10 s on 900‰ |
| P1.8 | propose 10 s | **wrong:** 5 s |
| P1.9 | two cores raise the share, because RSS follows wall time | **the share rose (600–624‰ against 514–526‰), but not by wall time:** wall moved about 1 s. The reason is wrong |
| P2–P4 | | not run |
