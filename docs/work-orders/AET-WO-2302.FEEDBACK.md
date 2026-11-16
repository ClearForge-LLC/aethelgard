# AET-WO-2302 FEEDBACK — the twin's windows

Branch `wo/aet-2302-windows`, cut from `main` at `1178ca8` (specification 12, trace 12, `TIER2_MIN` 160). Spike first, then stop for the ruling. Every answer is labelled **measured**, **read at source** or **inferred**.

## §0 Predictions (committed before any measured run, never edited)

Read at source before predicting:
- The generated bring-up (`svd-hal/src/board/firmware.rs`) runs: reset clock check, `OPTSR` read, clock enables, GPIO, I2C/SPI init, each serial port configured, **each port's pattern sent**, every identity and configure step, **one settle** (the largest `settle_us`, spun at the fastest HSI: 64.96 spins per µs), every data step, **`start_outputs`** per timer, **the boot word**, then **each port's echo**, then the self-loop.
- `settle_us` in `parts.json`: ICM20948 40 000, ICM42688P 20 000, BMP581 10 000, ICP10111 340; BME280, TMP117 and W25Q64JV none.
- Renode runs the settle spin about 1.92 times silicon's lower bound (`4220` F2); `4230` measured `imu-baro-1v8`'s boot word at 78.1 ms.
- The serial frame is injected at 100 µs, paced one quantum-rounded character apart; the wire budget (`wire_ns`) is silicon time: pattern (label + 5 + 1 chars) plus 8 paced frame bytes. Renode's USART transmits in zero virtual time.
- `DSHOT_WINDOW_NS` was set by log volume: four DShot600 channels for 100 ms are 139.6 MB of log and 10.8 s of timer events (`4300a` P5.2); `LOG_FILTER` is 8 MiB.

Predictions:

| # | Prediction |
|---|---|
| P1 | Reference (BME280, TMP117, W25Q64JV, CONSOLE 115200): the boot word is written between 0.2 and 1.0 ms of virtual time; every device step falls before 1 ms; the pattern is out before the first device step; the echo ends within 0.1 ms after the frame's last byte (800 µs). |
| P2 | `motors-dshot600`: `start_outputs` and the boot word fall between 0.2 and 1.0 ms, within 20 µs of each other. |
| P3 | `serial-dshot-9600`: refused before the boot today (`SerialWireTimePastWindow`; wire budget about 22.3 ms against 2 ms). Its twin phases match P2 plus the console's pattern. |
| P4 | `imu-baro-1v8`: identity and configure steps before 1 ms; the settle takes 76–78 ms; data steps and the boot word at 77–79 ms. |
| P5 | Sentinel-02-shaped (written here: `sentinel-02-own` + four DShot600 + MAVLINK 57600 + CRSF 420000): settle ≈ 38–39 ms; `start_outputs` and the boot word at 38–41 ms; wire budget ≈ 3.9 ms (MAVLINK) + 1.0 ms (CRSF) from the 100 µs injection, so the serial evidence ends by about 5 ms. |
| P6 | DShot's wall cost is the timer events, not the log: with the observers' logging off, four channels still cost ≈ 0.1 s of wall per ms of virtual time. Logging on adds ≈ 1.4 MB of log per ms. |
| P7 | (a) anchored windows on Sentinel-02-shaped: the boot ends at `start_outputs` + 2 ms ≈ 41 ms; wall ≈ the `imu-baro` boot's share for 41 ms plus 2.6 s of DShot; log ≈ 3 MB; the witnesses hold. Trace ≈ today's records plus one window record per witness: 1.5–3 KB per run. |
| P8 | (b) one 100 ms window with a DShot sub-window: DShot runs ≈ 60 ms after its sub-window; unless the observers are silenced at the sub-window's end, the log passes 8 MiB and the witness fails closed; silenced, the wall is ≈ 6–7 s more than (a). Trace bytes as (a). |
| P9 | (c) two boots: wall ≈ (b)'s device boot plus (a)'s output boot (the firmware is one ELF, so DShot runs in the device boot too); trace ≈ 2× one boot's records. |
| P10 | Recommendation expected: (a), with the DShot anchor bounded by the judge (after the last data step's witness, before the boot word, and early enough that anchor + window ≤ 100 ms). |
| P11 | Q6 under (a): a mixed board needs the output window to hold two of its slowest PWM periods (40 ms at 50 Hz); with DShot observers silenced after 2 ms, the cost is ≈ 4 s of wall per boot from DShot's timer events. |
| P12 | Q8: no override kind reaches a serial port; the seeded `serial_rx_dead` is a firmware mutant, which `verify`'s campaign (the generated ELF) cannot carry. `Target::Serial` needs an override kind the composition renders on the board side (the RX line open: no frame injected), detected by `serial_echo_missing`. |
| P13 | 3302 synchronisation: no verdict depends on the controller's SCL period; the twin's I2C transfer is instantaneous in virtual time, and the windows are ms-scale. Recorded as a model divergence. |

## §1 How it was measured

- **Where:** Furnace, the pinned tools image (`container/IMAGE`, Renode `1.17.0+20260907gitf1dd1b4af`), every container through `~/bin/capped-run` (read back each time: `memory.max` 8 GiB, `memory.swap.max` 0, `pids.max` 8192), `--network none`, one container at a time.
- **What ran:** `main`'s CLI at `1178ca8` with a measurement-only patch (`spike/2302/measure.patch`, in a scratch worktree, never in `crates/` here). With no `AET2302_*` variable set, it is `main`. The variables:
  - set the board's window and the `--for` cap;
  - dump Renode's raw stdout with wall time, kept bytes and dropped lines;
  - add monitor commands before the run, and at one instant mid-run;
  - silence the motor observers;
  - move the serial injection;
  - pass the same window to replay.
- **Exact phase times.** Watchpoint hooks log `cpu.ExecutedInstructions` at each write of the boot word and every record word, each timer's `CR1` and each port's `TDR`, and at each `RDR` read (`spike/2302/hooks.py`). An instruction is 10 ns at 100 MIPS, so every phase time below is exact to the instruction, never the 100 µs quantum a CPU-caused log line carries.
  - The hooks do not change the run: the reference's verdict is identical with and without them.
  - They cost wall time: `imu-baro` took 3.65 s with them against 2.0–2.1 s without, so no wall time below comes from a hooked run except where it says so.
- **Generating the boards.** `imu-baro-1v8` and the Sentinel-02-shaped board cannot go through `generate` (`UnknownPart`: no vendored KiCad model for the ICP10111 or the BMP581). They were generated as `twin_tier2.rs` does (`generate_with` and `platform::emit`; `spike/2302/spike2302.rs.txt`), then built with `aethelgard build`.
- **Wall, log and memory.**
  - Wall is Renode's own run (`wall_ms`); the CLI's whole call adds 1.7–3.0 s for the sandbox and the analog pre-pass.
  - Peak RSS is the summed RSS of the Renode processes, sampled every 100 ms (`spike/2302/sh/rss.sh`).
  - Every row is three runs unless it says one. All rows are in `spike/2302/out/runs.tsv`.
- **The Sentinel-02-shaped fixture** (`spike/2302/fixtures/sentinel-02-shaped.json`): `sentinel-02-own` (BME280, TMP117, BMP581 on I2C; W25Q64JV and ICM-42688-P on SPI) plus `motors-dshot600`'s four DShot600 outputs and connector, and `serial-four`'s MAVLINK (57600) and CRSF (420000) ports with their DS-009 connectors. It validates and solves. aet4211's inbox was told it places a W25Q64JV.

## §2 Each phase's virtual time (measured, exact to the instruction)

Times in µs from reset. "Pattern" is the first to the last `TDR` write of the port's pattern; "echo" is the first `RDR` read to the last echoed `TDR` write. `phases-*.json` in `spike/2302/out/` holds every value.

| | reference | `motors-dshot600` | `serial-dshot-9600` | `imu-baro-1v8` | Sentinel-02-shaped |
|---|---|---|---|---|---|
| `OPTSR` read | 1.59 | 1.58 | 1.59 | 1.59 | 1.58 |
| pattern | CONSOLE 12.62–18.49 | none | CONSOLE 16.07–21.94 | none | MAVLINK 22.49–28.36; CRSF 28.98–33.23 |
| last identity or configure step | 25.70 (U4) | 20.16 (U4) | 29.15 (U4) | 42.86 (U7 configure) | 79.10 (U5 configure 3) |
| the one settle (largest `settle_us`) | none | none | none | 40 000 cited → 77 953.5 | 20 000 cited → 38 979.4 |
| data steps | none | none | none | 77 996.36–78 016.27 | 39 058.49–39 070.62 |
| `start_outputs` (TIM1 `CEN`) | none | 21.67 | 30.67 | none | 39 072.33 |
| boot word | 25.75 | 21.75 | 30.75 | 78 016.48 | 39 072.41 |
| echo | 100.14–800.54 | none | 100.17–7 800.59 | none | MAVLINK 39 072.71–39 078.49; CRSF 39 078.89–39 084.67 |

- **The bring-up without a settle takes 22–31 µs.** The boot word follows `start_outputs` by 80 ns on every board with outputs.
- **The settle runs 1.949 times its cited time** on both boards: 77 953.5 / 40 000 and 38 979.4 / 20 000. That is exactly 3 instructions × 10 ns × 64.958976 spins per µs (inferred from the arithmetic: the loop executes 3 instructions per spin).
- **`imu-baro-1v8`'s boot word is at 78.016 ms**, inside 100 ms with 22 ms to spare. This is `4230` Q-E: closed for this fixture, measured.
- **Where the echo lands (finding F1, below).** On the reference and `serial-dshot-9600` the frame goes in at 100 µs, after the boot word, and each byte is echoed as it arrives, paced. On the Sentinel-02-shaped board the frame goes in at 100–1 500 µs, before the 39 ms settle. All 16 bytes then wait about 38 ms in Renode's queue, and the firmware echoes them back to back in 12 µs.

## §3 What DShot costs (measured on `motors-dshot600`, one run each)

Wall is Renode's; "log" is the stdout the CLI kept (budget 8 MiB).

| DShot running | observers logged: wall, log, RSS, verdict | observers silenced: wall, log, RSS |
|---|---|---|
| 2 ms | 2.69 s, 2.78 MB, 0.43 GB, pass | 2.54 s, 0.02 MB, 0.42 GB |
| 5 ms | 3.54 s, 6.97 MB, 0.46 GB, pass | 3.18 s, 0.02 MB, 0.45 GB |
| 10 ms | 4.07 s, 8.40 MB + 75 009 lines dropped, 0.49 GB: `renode_log_flood`, every witness fails closed | 3.62 s, 0.02 MB, 0.48 GB |
| 20 ms | 5.13 s, flood | 4.55 s, 0.52 GB |
| 40 ms | 7.66 s, flood | 6.32 s, 0.60 GB |
| about 130–150 ms (the phase runs) | — | killed at 1 GiB (`memory_cap_exceeded`) after 12.8–14.8 s |

- **The log budget holds about 6 ms of four DShot600 channels** (1.40 MB per ms: 2 to 5 ms added 4.19 MB).
- **Silencing the observers does not stop the cost.** The timer's events still cost about 0.10 s of wall and 4.8 MiB of RSS per ms of DShot (2 to 40 ms: +3.8 s, +179 MiB).
- So DShot's real limit is memory, not log volume. A DShot board cannot run its timer to 100 ms beside Sentinel-02's own models (0.62 GB before any motor, §4) without nearing the 1 GiB bound.

## §4 The three candidates (measured on the Sentinel-02-shaped board unless stated)

Common to every row: frames injected one quantum after the boot word (39.1 ms), except (c)'s device boot.

| | boot | wall (Renode) | log kept | peak RSS | trace bytes per run (`OQ-03`) | witnessed |
|---|---|---|---|---|---|---|
| today | refused (read at source: the 2 ms window holds neither the data steps at 39 ms nor the 4.9 ms wire budget) | — | — | — | — | nothing whole |
| no motors (`sentinel-02-own`, today's 100 ms boot), for reference | 100 ms | 3.20–3.24 s | 0.03 MB | 0.62–0.63 GB | — | — |
| **(a) static**: boot to a fixed 44.1 ms, observers logged throughout | 44.1 ms | 4.86 s (one run) | 7.07 MB (84 % of budget) | 0.63 GB | **6 386 B (measured, campaign, 90 records)** | everything; **today's judge passes it** |
| (a) static, observers silenced at `start_outputs` + 2 ms (41.1 ms) | 44.1 ms | 4.72–5.02 s | 2.88 MB | 0.69–0.70 GB | as above, +≈0.2 KB of window fields (inferred) | everything once the judge knows the sub-window (today: `motor_timing_mismatch`) |
| **(a-i) anchored, 100 ms boot**: at `start_outputs` + 2 ms the judge's own action reads `TIM1.CR1` back (0x81), silences the observers and clears `CEN` | 100 ms | **3.98–4.74 s** | 2.84 MB | **0.66–0.68 GB** | as (a) (inferred) | everything once the judge knows the sub-window and its own `CEN` write (today: `motor_timing_mismatch`, `register_mismatch`) |
| (b) one 100 ms window, DShot sub-window by silencing only | 100 ms | 9.84–9.86 s | 2.88 MB | **0.95–0.96 GB (90–91 % of the 1 GiB cap)** | as (a) (inferred) | as (a-i) |
| (c) two boots: the device boot alone (100 ms, observers silenced) | 100 ms | 9.54–9.72 s | 0.05 MB | 0.85–0.86 GB | | |
| (c) both boots (inferred: the device boot plus (a)'s output boot) | 100 + 41–44 ms | ≈ 14.3–14.7 s | ≈ 2.9 MB | 0.86 GB | ≈ 8.1 KB (inferred: kinds 10, 11, 12 and 14 twice, 1 732 B more) | as (a-i) |

On the other DShot boards, (a-i) costs what today's 2 ms boot costs:

| | today | (a-i), 100 ms boot |
|---|---|---|
| `motors-dshot600` | 2 ms: 2.62–2.71 s, 0.43 GB | 2.76–2.79 s, 0.43–0.44 GB |
| `serial-dshot-9600` | refused (`SerialWireTimePastWindow`: 22.3 ms of wire) | 3.22–3.34 s, 0.45–0.46 GB; the 9600 echo runs 100 µs to 7.8 ms, paced |

Today's other boots, for comparison (three runs each): reference 2.21–2.23 s, 0.41 GB; `imu-baro-1v8` 2.01–2.11 s, 0.40 GB; `motors-pwm` (100 ms) 2.05–2.06 s, 0.40 GB.

- **Trace bytes (`OQ-03`) do not depend on the window.** Every witness record is read once at the boot's end. Measured per run: reference 3 428 B, `motors-dshot600` 4 183 B, `imu-baro-1v8` 9 723 B, Sentinel-02-shaped 6 386 B (per kind in `spike/2302/out/trace-kinds-*.txt`).
  - Per-witness windows add an anchor and an end to kinds 26, 28 and 29: about 18 records on Sentinel-02, roughly 0.2 KB (inferred).
  - Every candidate is far inside `MAX_RECORDS` (65 536) and the reader's 256 MiB.
  - §7's archive stop does not fire.
  - Note: §2.17's "about 1 KB per scenario" is now 3.4 KB on the reference, because of the records added since; this is a docs row, below.
- **CI** (inferred): the weights file times one reference boot at about 5 s on GitHub's runner against 4.4 s of CLI wall here, so scale Furnace's CLI times by about 1.15–1.2.
- **No candidate can end a boot early.** A watchpoint hook's `ScheduleAction` fires exactly (at `CEN` + 200 000 instructions, every run). But `PauseAndRequestEmulationPause()` from it did not stop `RunFor` (10 M instructions still ran), and `PauseAll()`, `Pause()` and the monitor's `pause` each hung the run until the reaper killed it at 1 GiB (`spike/2302/sh/probe-pause.sh`). A boot's length must be fixed before it starts, whatever the anchors are.

## §5 Findings that shape the windows

- **F1 · A settle before the echo defeats the paced frame** (measured; this was not in my predictions).
  - The frame goes in at reset + 100 µs, so on a board with a settle it waits in Renode's queue for the whole settle: about 38 ms on Sentinel-02.
  - The firmware then echoes all 16 bytes of both ports in 12 µs. This is 4300b's ADV4-2 attack (a firmware that hardly waits after the first byte), reopened on every board with a sensor start-up.
  - On silicon the generated `CR1` sets bit 29 (`FIFOEN`; Renode logs it as `RESERVED`), so an 8-byte frame would wait in the RX FIFO, not overrun. That is inferred from RM0433's FIFO, not re-read. So this is a lost test, not a false pass.
  - **Fix:** inject one quantum after the boot word, as (a) and (a-i) do. Measured: both ports' frames then echo paced and pass.
- **F2 · A late anchor shrinks the DShot evidence silently in any longer window** (measured).
  - The mutant delays `start_outputs` by a 2 000 000-spin settle (`spike/2302/sh/late.sh`). Its `CEN` is at 99.072 ms, and the boot word 80 ns later.
  - Today's judge, given a 100 ms window, **passes it**: the motor witness needs two rises and coverage to the end, and 0.93 ms of DShot gives both.
  - So any window longer than 2 ms needs the anchor bound: `CEN` + `DSHOT_WINDOW_NS` within the boot, `CEN` after the last data step's witness, and before the boot word.
  - The anchor is observable exactly: by the hook's instruction count, and by the timer access line, which carries an exact time (`4300a` M4).
- **F3 · Memory is the binding cost, not the log or the archive.**
  - Sentinel-02's own models alone take 0.62–0.63 GB of the 1 GiB bound (`sentinel-02-own`, no motors).
  - DShot adds about 4.8 MiB per ms. (b) reaches 90–91 % of the cap, and (c)'s device boot 81–82 %.
  - The bound was set at 2× the reference's 404 MB (README); Sentinel-02 is the first board near it.
- **F4 · The bus's time is not modelled.** The twin's I2C and SPI transfers take a few µs of CPU each: on the reference, U2's identity record is written 1.66 µs after the pattern's last byte, and U4's 5.55 µs after U2's. On silicon a 400 kHz byte is about 22 µs. Every window above has milliseconds of slack, so nothing here depends on it (§9).

## §6 Recommendation: (a-i)

**Each witness gets the window its evidence needs, anchored where it starts, inside one 100 ms boot on every board:**

1. **Every board boots `MAX_VIRTUAL_NS`** (100 ms). `DSHOT_WINDOW_NS` stops being a board's boot length and becomes the length of DShot's evidence window. `BootPastWindow` and the per-board boot window go: `simulate --for` admits up to 100 ms on every board.
2. **Anchors are observed by the CLI's own watchpoint hooks**, composed in the monitor:
   - each DShot timer's `CR1` write that sets `CEN`;
   - the boot word.

   Each hook logs its instruction count; the judge reads it and checks it against the timer access line's exact stamp.
3. **At each DShot anchor + `DSHOT_WINDOW_NS`, the judge's scheduled action** (its own act, recorded as such):
   - reads the timer's registers back (the read-back `4300a` takes at the end today);
   - silences that timer's observers;
   - clears `CEN`.

   Edges after the window are never logged and never evidence. Measured cost: today's on `motors-dshot600`, and on Sentinel-02 its no-motor cost plus 0.7–1.5 s and 30–55 MB.
4. **The anchors are bounded by the judge**, each failure by name:
   - the first `CEN` after the last data step's witness and before the boot word;
   - `CEN` + `DSHOT_WINDOW_NS` ≤ 100 ms;
   - an observer edge before the anchor, or a second `CEN` rise, is refused.

   F2's mutant then fails by name. The bound uses only the run's own witnessed records and the constant, never the generator (`INV-07`).
5. **Serial frames are injected one quantum after the boot word** (the boot-word hook schedules the paced `WriteChar`s), so the echo is witnessed live (F1).
   - The static wire budget is judged from that instant: refused by name if it passes 100 ms.
   - The pattern is judged from reset, as today.
   - Measured: `serial-dshot-9600` (22.3 ms of wire) and Sentinel-02 (4.9 ms) both fit.
6. **Devices are judged from reset as today.** The data steps must precede the DShot anchor.
7. **The trace moves to version 13:**
   - kind 26 carries its anchor and its window's end;
   - kinds 28/29 carry the injection instant;
   - a record states the judge's stop;
   - replay derives each window from the records, not from the protocols (`campaign.rs`'s `short_window`).

**Why not the others:**
- **(a) static** (a boot length fixed from a bound on the anchors, observers silenced after 2 ms):
  - costs the same (4.7–5.0 s, 0.69–0.70 GB) and needs no judge write;
  - but its bound needs the twin's instructions per spin (3, a property of the generated loop: `INV-07` asks the judge not to take it from the generator);
  - and its injection must land within the echo's poll budget after the real boot word (`ECHO_POLLS` 20 000, about 1–2 ms, inferred), which a static bound cannot promise.
- **(b)** costs 2.4× (a-i)'s wall and 90–91 % of the memory cap for no extra evidence.
- **(c)** costs about 3.5× (a-i)'s wall and adds a determinism check between its boots, for no extra evidence.

## §7 Q6 under (a-i): a mixed board

- **Rule (read at source).** A timer's outputs share one protocol and rate (`ir/src/lib.rs`: "the outputs of one timer share its counter"), so stopping a DShot timer never stops a PWM output.
- **Window length.** The PWM witness needs two rises: two model periods, the first edge one period late (`4300a`).
  - The model's timer runs at its fixed 250 MHz, not the solved 64 MHz. So a 50 Hz output's model period is about 5.12 ms, and two are 10.24 ms. This is inferred, and agrees with `4300a`'s "about 260 Hz" at 2 ms.
- **Does it fit?**
  - Under (a-i), a PWM output's window runs from its own timer's `CEN` to two model periods later, inside the same 100 ms boot.
  - On the Sentinel-02-shaped board (anchor 39.07 ms) a 50 Hz servo ends at 49.3 ms. On an `imu-baro`-shaped board (anchor 78.0 ms) it ends at 88.3 ms.
  - The slowest PWM that fits after an anchor `t` has a silicon period of at most (100 ms − `t`) × 250/64 / 2: about 119 ms (8.4 Hz) at `t` = 39 ms.
- **Cost: none beyond (a-i).**
  - The boot is 100 ms anyway, and a PWM timer costs nothing measurable: `motors-pwm`'s 100 ms boot is 2.05 s, against the reference's 2.21 s.
  - What was refused (two periods beyond 2 ms) is witnessed.

## §8 Q8: `Target::Serial`

- **Read at source:**
  - `Target::Serial` already exists in `cli/src/verify.rs`, and each port is listed `no fault kind yet: serial` (never blocking, ruled on #155).
  - `stress()` is in `aethelgard-cli/src/verify.rs`, not in `aethelgard-chaos` as §3's fence says. This is a fence flag.
  - `detection()` requires `attribute` to name the **electrical** half by the mechanism's rule. Evolve gives a serial failure with no software cause rule `I6` (indeterminate), and with one, `S5` (software).
  - `serial-rx-dead` (PA10 on AF8) is a patch of the generated board in the seeded set. `verify`'s campaign runs the ELF it regenerates, so it can never carry that mutant.
- **(i) The work order's literal reading: the seeded mutant as the mechanism.**
  - A port counts as covered when the campaign holds a baseline whose run witnessed that port's echo, and the seeded set (`attribute_seeded.rs`) proves `serial_echo_missing` by `S5` is detected.
  - It needs no version and no fence change: `verify.rs` only.
  - But it is evidence about the judge on the reference, not about this board under a fault. `3303`'s rule wants a fault seeded on the board's own campaign, so I would not call it coverage.
- **(ii) An override kind, `serial_rx_open { port }`: the board's RX line open** (the `2302` reading of `3303`'s "an RX line held, or open").
  - The composition renders it by injecting no frame into that port. It is a board fault, never a firmware mutant.
  - The mechanism is `serial_echo_missing`, on that port alone.
  - **Cost:** an `Override` variant `[5, label]` and its record (trace 13, taken with §6); scenario schema 3 (or folded into 2's `v2_only` list); the campaign's injection skips the port; `stress()`, `classify()` and `coverage()` move serial ports from `no_fault_kind` into `unmet` and `covered`.
  - **Then one of:**
    - evolve gains `E3` (serial: an open RX line when no software cause explains it). Evolve is outside this work order's fence;
    - or `detection()` admits `I6` for a serial target with that port as its only subject.
  - Recommended, with `E3`, if the architect extends the fence. With neither, it stays `no_fault_kind`.
- **The seeded firmware mutant stays what it is:** the proof, in the seeded set, that `S5` names a firmware cause. (ii) gives the board-side fault, so the two together separate a dead RX by cause.

## §9 The 3302 synchronisation note

- **Read at source:** no verdict depends on the controller's SCL period.
  - The analog deck (`twin/src/analog.rs`) releases each line once, 2.5 µs into a fixed 22.5 µs window at the 400 kHz deck rate, and judges the 30–70 % rise against the bus's ceiling. That threshold is the peer's, and it is independent of when the controller sees SCL high.
  - A rise unfinished at the window's end is past every ceiling (≤ 1 000 ns), whatever the controller does.
  - `ir/src/silicon.rs`'s `TIMINGR` rule judges the shortest period: `tSYNC` at its 2-cycle minimum, slopes not credited. A controller that waits for its own 1.7 V threshold only lengthens `tHIGH`, so every bound it checks holds a fortiori. `data_valid`, which a longer period could touch, is reported and not gating.
  - The twin's I2C transfer takes no bus time (F4), so no window edge depends on it either.
- **Recorded as a model divergence for architecture §2:** the H7 controller times `SCLH` from its own pin's threshold (about 1.7 V) by clock synchronisation, which Renode does not model. The campaign's rise verdict uses the peer's thresholds and is unaffected. On silicon a slow pull-up lengthens the clock (3302: 8 µs against 2.6 µs at 100K).

## §10 Predictions, scored (§0 is unedited)

| # | Verdict |
|---|---|
| P1 | **Missed** by about 10×: the boot word is at 25.75 µs, not 0.2–1.0 ms. The pattern does precede the steps, and the last echo is written 0.54 µs after the last byte goes in. |
| P2 | **Missed** on the scale (21.67 µs), held on the gap (80 ns). |
| P3 | **Held:** refused today by the wire budget (read at source; 22.3 ms); its twin phases match P2 plus the pattern. |
| P4 | **Held:** settle 77.95 ms, data and boot word at 78.0 ms. |
| P5 | **Held:** settle 38.98 ms, `start_outputs` 39.07 ms; wire budget 3.86 + 1.04 ms. |
| P6 | **Held, and incomplete:** about 0.10 s per ms of DShot with the observers silenced. But I missed the memory: about 4.8 MiB per ms, which is what limits DShot. |
| P7 | **Held in shape:** (a) at 44.1 ms took 4.7–5.0 s and 2.9–7.1 MB. The boot ends at 44.1 ms, not 41 ms, because the serial wire budget counts from the injection. |
| P8 | **Held:** silenced, 9.8 s (+5 s over (a)); unsilenced it floods. I did not predict 0.95 GB. |
| P9 | **Held:** about 14.5 s; trace about 8.1 KB (inferred). |
| P10 | **Held, refined:** (a), as (a-i) (100 ms boot, judge stops DShot at its window's end). The judge can bound the anchor, and it must (F2). |
| P11 | **Missed:** the cost is about 0, because under (a-i) the boot is 100 ms anyway and DShot is stopped. I also had the model period wrong: 50 Hz is about 5.1 ms in the model, not 20 ms. |
| P12 | **Held in substance;** the fence and evolve's `I6`/`E3` were not in it. |
| P13 | **Held.** |

## §11 Decisions for the architect

- **D1 · The candidate.** (a-i) recommended (§6). Or (a) static, (b), or (c) at the costs in §4.
- **D2 · The judge's stop.** Clearing `CEN` on each DShot timer at its window's end is the judge acting on the machine.
  - It is a new twin-detection oracle after the window, of `4300b` Q9's class: a firmware that reads its timer later sees it stopped.
  - Generated firmware never touches a timer after `start_outputs` (read at source, `firmware.rs`); ArduPilot (`4400c`) would.
  - The alternative is silencing alone, at (b)'s memory.
  - Accept the stop, with the residual named in every verdict?
- **D3 · The first monitor Python the CLI composes.** (a-i) needs two watchpoint hooks and one `ScheduleAction` per DShot timer, in Renode's IronPython.
  - `RUL-06`, as corrected, says no Python hook exists, and nothing forbids one.
  - They run inside the existing sandbox and read and write only the machine.
  - Accept, or require (a) static, which needs no Python but takes the generator's instructions per spin?
- **D4 · Serial injection after the boot word** (F1): accept as part of the windows?
- **D5 · Q8:** (ii) with `E3`, which needs the fence extended to `aethelgard-evolve`; or (i); or `no_fault_kind` stays.
- **D6 · The memory bound.**
  - Sentinel-02 takes 0.66–0.68 GB of 1 GiB under (a-i), and 0.62 GB with no motors at all.
  - No change is proposed, but the next model of our own may cross it.
  - Raise the bound (it was 2× the reference's 404 MB), or record the margin per board in the verdict?
- **D7 · Fences.**
  - (a-i) changes `aethelgard-twin/src/boot.rs` (`window()`, `DSHOT_WINDOW_NS`'s meaning), outside §3's fence.
  - `stress()` is in `cli/src/verify.rs`.
  - Q8 (ii) needs `aethelgard-evolve`.
- **D8 · Versions.** Trace 13 for (a-i) (and Q8's override). No specification change.

## §12 For the docs (protected, so here)

- **Architecture §2.17:** "about 1 KB per scenario" is 3.4 KB on the reference today; still independent of virtual time.
- **Architecture §2:** the clock-synchronisation divergence (§9) and the bus-time divergence (F4).
- **The CLI README** still documents only version-1 scenarios. The build brings it up to date (WO §2).
