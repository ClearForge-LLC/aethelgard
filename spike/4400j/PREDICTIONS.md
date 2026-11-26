# AET-WO-4400j predictions

Committed before any Renode run of the ELF below, smoke runs included. Graded in `RESULTS.md`.

## The board and the firmware: the choice, named

- **Sentinel-03 is main's** (`examples/sentinel-03.json` at `589030ca`, sha256 `f6d409a3…`, schema 30). `py/derive.py` adds one declaration, `mcu.foreign_clock`. Its values are read from the pinned ArduPilot image's `stm32h7_mcuconf.h` for `OSCILLATOR_HZ 16000000`: timers 200 MHz, VOS scale 1, every USART 100 MHz.
  - `4400h`/`4400i` copied `sentinel-02-own`'s HSI values (72 MHz USARTs). On this HSE board those would be wrong.
  - The derived spec is sha256 `0c5bb563…`, kept in `~/.cache/aet-4400j/boards/s03/`; `examples/` is `4411`'s.
- **ELF:** unmodified Copter-4.7.2, built for it by CI's own `container/ardupilot-build.sh` in the pinned image `e1809b90…` (`sh/board.sh`). It is sha256 `97afa171…`, from hwdef `7f2b62aa…`. CI builds no Sentinel-03 ELF.
- **Anchor:** one run each on `4400h`'s ELF `dbf598d8…` with its spec `135c96fa…`, tying back to `4400i`'s 56 MB and 2.13–2.45 GB.
- **Input:** `4400i`'s `arm12.json` (sha256 `7f231564…`), unchanged.
- **CLI:** main `589030ca` plus `patches/measure.patch`, which is `4400i`'s `window.patch` rebased past `1918`. It reads the wall and RSS bounds and the log budget from the environment, and raises `MAX_CAPTURE` to 256 MiB and the window ceiling to 30 s. Unset, these are main's bounds.

## P1. Boot

| # | prediction |
|---|---|
| P1.1 | Main's Sentinel-03 boots under `--foreign 5000ms` and **passes**. No `foreign_clock_contradicted`: the RCC decodes 400 MHz from the HSE |
| P1.2 | The USARTs at a declared 100 MHz carry ArduPilot's 115200 baud console cleanly. The input's frames are answered |
| P1.3 | The ICM-42688-P's first FIFO read is between 4.3 and 4.5 s (4.394 s on `sentinel-02-own`) |

## P2. The log, by source and virtual second (CLI path, budget lifted)

| # | prediction |
|---|---|
| P2.1 | Before the ICM's FIFO starts (below 4.4 s), the kept log grows under 0.4 MB per virtual second |
| P2.2 | From 4.4 s with no input, the kept log grows 2.0–2.5 MB per virtual second, from the four `4400h` sources in about `4400h`'s shares: the mux and ICM NOISY lines about 55 %, `AETHELGARD-CFG2` about 22 %, spi2's slave-mode ERROR about 20 %. Kept at 12 s: 16–21 MB |
| P2.3 | With the input, the SPI flash model (U4, W25Q64JV) is the second-largest source by bytes. It comes in bursts when ArduPilot stores parameters (`4400i` F6), not as a steady rate. Kept at 12 s: 45–65 MB (`4400i`: 56 MB) |
| P2.4 | Every repeating per-transaction source is one template whose varying fields are the data bytes, a register value and the timestamp. The cycle `CFG2 (MASTER 0)` → `ERROR` → `CFG2 (MASTER 1)` repeats with at most 4 distinct value tuples per controller |
| P2.5 | Renode's raw console (replay, nothing summarised) carries 10–14 MB per virtual second from 4.4 s, most of it `Unhandled write` |

## P3. RSS by virtual second

| # | prediction |
|---|---|
| P3.1 | The tree's RSS is nearly flat at 650–800 MiB until the ICM's FIFO starts, then grows **90–130 MiB per virtual second**, with no input |
| P3.2 | With the input, the slope is within 15 % of the no-input slope: the IMU's traffic, not the injection, carries the growth |
| P3.3 | The CLI's peak passes 1.5 GiB (1,572,864 KiB) between 9.5 and 11 s with no input. At 12 s it is 1.75–2.1 GiB with no input, 2.0–2.5 GiB with it |
| P3.4 | Monitor-read managed figures (`GC.GetTotalMemory(true)`, GC info) carry **at least 70 %** of the RSS slope: the growth is mostly live managed memory, and a full GC frees under 10 % of it |
| P3.5 | Core count moves the level: 4 pinned cores read 50–200 MiB higher than unpinned at the same virtual time (`4400h` saw +115–170 MB at 2 cores) |

## P4. What holds it (the dump)

Two dumps, at about 6 s and 9 s virtual, each after a full GC with the machine paused, diffed by type.

| # | prediction |
|---|---|
| P4.1 | The growth between the dumps is dominated, more than 60 % of the bytes, by **one to three types**. It is not spread across the heap |
| P4.2 | **Ranked holder hypotheses,** with the type each would show growing: |
| | (a) **a per-transfer collection in Renode's DMA or DMAMUX path** (`STM32DMA`, the request wiring): `Byte[]`, a `List<…>` or `Queue<…>` of transfer records, or delegates, rooted at the DMA peripheral. **Most likely** (0.35) |
| | (b) **the SPI or SPI-multiplexer model's buffers** (`STM32H7_SPI`, `SPIMultiplexer`): a `Queue<Byte>` or `Byte[]` that a disabled peripheral or the slave-mode scar never drains, rooted at `spi2` (0.25) |
| | (c) **the time framework:** actions or synchronisation hooks scheduled per transaction and kept (`ClockEntry`, `Action`, `TimeHandle` delegates), rooted at the machine's time source (0.15) |
| | (d) **the logging path:** formatted entries or a per-source repeat cache kept across the run (`String`, `LogEntry`), rooted at the logger (0.10) |
| | (e) **our ICM-42688-P model** (`models/renode/ICM42688P.cs`), despite `4400h`'s reading (0.10) |
| | (f) something else, or native memory not visible in a managed dump (0.05) |
| P4.3 | Whatever the type, its root chain runs through a **Renode peripheral of the IMU's bus** (spi2, its mux or a DMA stream), not through the CLI's monitor commands or hooks |
| P4.4 | The holder is **Renode's own code**, not configurable from the platform or the CLI's monitor commands. The likely outcome is §7's stop "Renode's own", with the dump as evidence |

## P5. Wall time

| # | prediction |
|---|---|
| P5.1 | On Furnace pinned to 4 cores at load under 10: 5 s takes 9–13 s of wall. Each further virtual second takes 2.0–3.0 s; 12 s takes 25–35 s |
| P5.2 | CI's slowest runner (EPYC 7763; `1918` measured boot medians 1.5–3.5× Furnace's): 12 s takes about 40–120 s. **The 120 s guard holds 12 s on the slowest runner with little or no margin at the high end.** |

## P6. The window's end (F4)

| # | prediction |
|---|---|
| P6.1 | PB13 (SPI2 SCK) is output/AF0 for short intervals around SPI2's bus release. It is an ArduPilot driver transient that repeats while the IMU runs, under 5 % of virtual time |
| P6.2 | At the four lengths: 5 s and 8 s clean (as `4400h`); 10 s and 12 s each clean with probability about 0.9 |
| P6.3 | At ±1 quantum (100 µs) around each length, at least one of the 12 ends reads the transient, or none does. Which one is a coin; I predict **none** at these 12 |

## P7. The window, and what would move

| # | prediction |
|---|---|
| P7.1 | **No window that holds the arm fits** the 1.5 GiB cap with the growth as found: the arm (8.6–9.8 s) plus a witness needs about 10–10.5 s, where the peak is 1.45–1.65 GiB. The stop says so unless the holder is configurable |
| P7.2 | **If the growth is removed,** 12 s fits the cap with over 40 % margin, and the log fits 8 MiB only with the four IMU sources and the flash model's lines summarised |
| P7.3 | Arm witnessed between 9.4 and 10.2 s on main's Sentinel-03 with `arm12.json` (parameters after `ArduPilot Ready`) |
| P7.4 | No committed package moves: the bench packages are generated-firmware `verify` verdicts, which never take the foreign filter. Summarising moves only foreign verdicts' log fields and `ci/foreign-judge.sh`'s expectations, if any of them reads them |
