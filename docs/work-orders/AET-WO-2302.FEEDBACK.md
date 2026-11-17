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
- **What ran:** `main`'s CLI at `1178ca8` with a measurement-only patch (`spike/2302/measure.patch`, applied in a scratch worktree, never to `crates/` on this branch).
  - With no `AET2302_*` variable set, it is `main`.
  - The variables:
    - set the window (the CLI's and replay's) and the `--for` cap;
    - dump Renode's raw stdout with wall time, kept bytes and dropped lines (on the bridge's path too);
    - add monitor commands before the run, and at one instant mid-run;
    - silence the motor observers;
    - move the serial injection, or turn it off.
- **Exact phase times.** Watchpoint hooks log `cpu.ExecutedInstructions` (`spike/2302/hooks.py`):
  - at each write of the boot word and each record word;
  - at each write of each timer's `CR1` and each port's `TDR`;
  - at each read of `RDR`.

  An instruction is 10 ns at 100 MIPS, so every phase time below is exact to the instruction, never the 100 µs quantum a CPU-caused log line carries.
  - The hooks do not change the run: the reference's verdict is identical with and without them.
  - They cost wall time: `imu-baro` took 3.65 s with them against 2.0–2.1 s without. No wall time below comes from a hooked run unless it says so.
- **Generating the boards.** `imu-baro-1v8` and the Sentinel-02-shaped board cannot go through `generate` (`UnknownPart`: no vendored KiCad model for the ICP10111 or the BMP581). They were generated as `twin_tier2.rs` does (`generate_with`, `platform::emit`; `spike/2302/spike2302.rs.txt`), then built with `aethelgard build`.
- **Wall, log and memory.**
  - Wall is Renode's own run (`wall_ms`). The CLI's whole call adds 1.7–3.0 s for the sandbox and the analog pre-pass.
  - Peak RSS is the summed RSS of the Renode processes, sampled every 100 ms (`spike/2302/sh/rss.sh`). It is given in MiB and as a share of the 1 GiB bound.
  - Rows are three runs unless they say one. Every row is in `spike/2302/out/runs.tsv`, in KiB.
- **The Sentinel-02-shaped fixture** (`spike/2302/fixtures/sentinel-02-shaped.json`):
  - `sentinel-02-own`: BME280, TMP117 and BMP581 on I2C; W25Q64JV and ICM-42688-P on SPI;
  - `motors-dshot600`'s four DShot600 outputs and connector;
  - `serial-four`'s MAVLINK (57600) and CRSF (420000) ports with their DS-009 connectors.

  It validates and solves on `1178ca8`. aet4211's inbox was told it places a W25Q64JV.

## §2 Each phase's virtual time (measured, exact to the instruction)

Times are in µs from reset. "Pattern" runs from the first to the last `TDR` write of the port's pattern. "Echo" runs from the first `RDR` read to the last echoed `TDR` write. Every value is in `spike/2302/out/phases-*.json`.

| | reference | `motors-dshot600` | `serial-dshot-9600` | `imu-baro-1v8` | Sentinel-02-shaped |
|---|---|---|---|---|---|
| `OPTSR` read | 1.59 | 1.58 | 1.59 | 1.59 | 1.58 |
| pattern | CONSOLE 12.62–18.49 | none | CONSOLE 16.07–21.94 | none | MAVLINK 22.49–28.36; CRSF 28.98–33.23 |
| last identity or configure step | 25.70 (U4) | 20.16 (U4) | 29.15 (U4) | 42.86 (U7 configure) | 79.10 (U5 configure 3) |
| the one settle (largest `settle_us`) | none | none | none | 40 000 cited → 77 953.5 | 20 000 cited → 38 979.4 |
| data steps | none | none | none | 77 996.36–78 016.27 | 39 058.49–39 070.62 |
| `start_outputs` (TIM1 `CEN`) | none | 21.67 | 30.67 | none | 39 072.33 |
| boot word | 25.75 | 21.75 | 30.75 | 78 016.48 | 39 072.41 |
| echo (today's injection at 100 µs) | 100.14–800.54 | none | 100.17–7 800.59 | none | MAVLINK 39 072.71–39 078.49; CRSF 39 078.89–39 084.67 |

- **Without a settle, the bring-up takes 22–31 µs.** On every board with outputs, the boot word follows `start_outputs` by 80 ns.
- **The settle runs 1.949 times its cited time** on both boards: 77 953.5 / 40 000 and 38 979.4 / 20 000.
  - That equals 3 instructions × 10 ns × 64.958976 spins per µs. Inferred from the arithmetic: the loop executes 3 instructions per spin.
- **`imu-baro-1v8`'s boot word is at 78.016 ms**, inside 100 ms with 22 ms to spare. `4230` Q-E is closed for this fixture, measured.
- **Where the echo lands** (F1, §5):
  - On the reference and `serial-dshot-9600`, the frame goes in after the boot word, and each byte is echoed as it arrives.
  - On the Sentinel-02-shaped board, the frames go in at 100–1 500 µs, before the 39 ms settle. All 16 bytes then wait about 38 ms in Renode's queue and are echoed back to back in 12 µs.

## §3 What DShot costs (measured on `motors-dshot600`, one run each)

Wall is Renode's. "Log" is the stdout the CLI kept (budget 8 MiB, 8 388 608 B).

| DShot running | observers logged: wall, log, peak RSS, verdict | observers silenced: wall, log, peak RSS |
|---|---|---|
| 2 ms | 2.69 s, 2.78 MB, 418 MiB, pass | 2.54 s, 0.02 MB, 408 MiB |
| 5 ms | 3.54 s, 6.97 MB, 452 MiB, pass | 3.18 s, 0.02 MB, 443 MiB |
| 10 ms | 4.07 s, 8.40 MB, 75 009 lines dropped, 476 MiB: `renode_log_flood`, every witness fails closed | 3.62 s, 0.02 MB, 466 MiB |
| 20 ms | 5.13 s, flood, 540 MiB | 4.55 s, 508 MiB |
| 40 ms | 7.66 s, flood, 653 MiB | 6.32 s, 588 MiB |
| about 130–150 ms (the phase runs) | | killed at the 1 GiB bound (`memory_cap_exceeded`) after 12.8–14.8 s |

- **The log budget holds about 6 ms of four DShot600 channels:** 1.40 MB per ms (2 ms to 5 ms added 4.19 MB).
- **Silencing the observers does not remove the cost.** The timer's events still cost about 0.10 s of wall and **4.7 MiB of RSS per ms of DShot** (2 ms to 40 ms: +3.8 s, +180 MiB).
- So memory, not log volume, is what limits a long DShot run.

## §4 The candidates

Measured on the Sentinel-02-shaped board unless a row says otherwise. Every row's frames go in one quantum after the boot word (39.1 ms, static), except "today" and (c)'s device boot. RSS is peak, as a share of the 1 GiB bound.

| | boot | wall (Renode) | log kept | peak RSS | trace bytes per run (`OQ-03`) | witnessed |
|---|---|---|---|---|---|---|
| today (unpatched) | refused, exit 3 `SerialWireTimePastWindow`: MAVLINK's wire time ends at 3.96 ms, past `DSHOT_WINDOW_NS` (measured). The data steps at 39 ms would miss it too. | — | — | — | — | nothing |
| no motors (`sentinel-02-own`, today's 100 ms boot), for scale | 100 ms | 3.20–3.24 s | 0.03 MB | 607–615 MiB (59–60 %) | — | — |
| **(a) static**: boot to a fixed 44.1 ms, observers logged throughout | 44.1 ms | 4.87–5.31 s (campaign 8.5–9.0 s) | 7.07 MB (84 % of budget) | 615 MiB (60 %, one run) | **6 386 B** | everything; **today's judge passes it** (3 of 3 campaign runs) |
| (a) static, observers silenced at `start_outputs` + 2 ms | 44.1 ms | 4.70–5.02 s (campaign 7.96–8.10 s) | 2.88 MB | 670–686 MiB (65–67 %) | **6 408 B** (a built judge adds window fields on 8 records, about 0.1 KB, inferred) | DShot inside its 2 ms: today's judge says only "the output stopped" (§4 note) |
| **(a-i) anchored, 100 ms boot**: at `start_outputs` + 2 ms the judge's own action reads `TIM1.CR1` back (0x81), silences the observers and clears `CEN` | 100 ms | **3.98–4.74 s** (campaign 7.35–7.68 s) | 2.84 MB | **645–661 MiB (63–65 %)** | **6 434 B** | as above; plus `register_mismatch` on `CR1`, the judge's own clear |
| (b) one 100 ms window, DShot sub-window by silencing only | 100 ms | 9.84–10.23 s (campaign 13.1–13.6 s) | 2.88 MB | **926–935 MiB (90–91 %)** | **6 408 B** | as (a) silenced |
| (c) two boots: the device boot (100 ms, observers silenced, frames at 39.1 ms) | 100 ms | 9.51–10.12 s (campaign 12.6–13.3 s) | 0.05 MB | 830–838 MiB (81–82 %) | 6 360 B | devices and serial |
| (c) its output boot (41.1 ms, observers logged, today's injection) | 41.1 ms | 4.01–4.06 s (campaign 7.22–7.30 s) | about 2.9 MB | — | 6 386 B | DShot (passes) |
| **(c) both boots** (measured as two campaign runs) | 100 + 41.1 ms | **13.5–14.2 s** (campaign 19.8–20.6 s) | — | 838 MiB | **12 746 B** (two archives; every boot writes a whole one) | as (a-i) |

**Trace bytes, measured** (`spike/2302/out/candidates-campaign.txt`): one campaign per candidate, three runs each, identical across runs; only (a) and (c)'s output boot pass with today's judge, the others fail on the sub-window as below, and every archive is written whole.

**Every `motor_timing_mismatch` above is the "output stopped" form**, read in the `stop1`, `b1` and `as1` verdicts. For example: "4716 rises follow the model's period 424 ns and high time 160 ns …, but the last is at 41071980 ns, more than a period before the window's end". Inside the 2 ms the witness is today's, so a judge that ends DShot's window at its anchor + 2 ms would witness it whole (inferred, not built).

**The other DShot boards under (a-i)** cost what today's 2 ms boot costs:

| | today | (a-i), 100 ms boot |
|---|---|---|
| `motors-dshot600` | 2 ms: 2.62–2.71 s, 417–418 MiB (41 %) | 2.76–2.79 s, 419–427 MiB (41–42 %) |
| `serial-dshot-9600` | refused (`SerialWireTimePastWindow`: 22.3 ms of wire) | 3.22–3.34 s, 435–448 MiB (43–44 %); the 9600 echo runs from 100 µs to 7.8 ms, paced |

Today's other boots, for scale (three runs each):

| | wall | peak RSS |
|---|---|---|
| reference | 2.21–2.23 s | 398 MiB |
| `imu-baro-1v8` | 2.01–2.11 s | 390–398 MiB |
| `motors-pwm` (100 ms) | 2.05–2.06 s | 389–391 MiB |

- **Trace bytes (`OQ-03`) do not depend on the window.** Every witness record is read once, at the boot's end.
  - Measured per run: reference 3 428 B, `motors-dshot600` 4 183 B, `imu-baro-1v8` 9 723 B, Sentinel-02-shaped 6 386 B. The per-kind split is in `spike/2302/out/trace-kinds-*.txt`.
  - Per-witness windows add an anchor and an end to kinds 26, 28 and 29: 8 records on Sentinel-02.
  - Every candidate is far inside `MAX_RECORDS` (65 536) and the reader's 256 MiB, so §7's archive stop does not fire.
  - §2.17's "about 1 KB per scenario" is now 3.4 KB on the reference, from the records added since (§12).
- **CI** (inferred): `ci/test-weights.tsv` times one reference boot at about 5 s on GitHub's runner, against 4.4 s of CLI wall here. Scale Furnace's CLI times by about 1.15–1.2.
- **No mechanism ends a boot early.** A watchpoint hook's `ScheduleAction` fires exactly: at `CEN` + 200 000 instructions on every run. But none of these stopped `RunFor`:
  - `PauseAndRequestEmulationPause()`: 10 M instructions still ran;
  - `PauseAll()`, `Pause()` and the monitor's `pause`: each hung the run until the reaper killed it at 1 GiB (`spike/2302/sh/probe-pause.sh`).

  A boot's length must be fixed before it starts.

## §5 Findings that shape the windows

- **F1 · A wait before the echo defeats the paced frame.** Measured; this was not in my predictions.
  - The frame goes in at reset + 100 µs. On a board with a settle, it waits in Renode's queue for the whole settle (about 38 ms on Sentinel-02), and the firmware then echoes 16 bytes in 12 µs. That is `4300b`'s ADV4-2 attack (a firmware that hardly waits after the first byte) reopened.
  - **Injected one quantum after the boot word** (a hook on the boot word schedules each `WriteChar`; measured, `spike/2302/sh/probe2.sh`): **the first port's bytes are read paced**, MAVLINK 200 µs apart from 39.1726 ms to 40.5726 ms.
  - **But the second port's frame, injected at the same instant, queues while the firmware echoes the first.** CRSF's 8 bytes were echoed back to back in 5.4 µs at 40.573 ms. The generated echo serves ports in turn, so a simultaneous injection defeats pacing on every port after the first.
  - Two ways out (a decision, D4):
    - inject port *k* when port *k*−1's last echoed byte is written (a `TDR` hook): sequential, every port paced;
    - or accept that later ports are not pacing-tested.
  - On silicon, the generated `CR1` sets bit 29 (`FIFOEN`; Renode logs it as `RESERVED`). So an 8-byte frame would wait in the RX FIFO, not overrun (inferred from RM0433's FIFO, not re-read). This is a lost test, not a false pass.
- **F2 · A late anchor silently shrinks the DShot evidence in any window longer than 2 ms.** Measured.
  - The mutant delays `start_outputs` by a 2 000 000-spin settle (`spike/2302/sh/late.sh`). `CEN` is at 99.072 ms, and the boot word 80 ns later.
  - **Today's judge, given a 100 ms window, passes it:** the motor witness needs two rises and coverage to the end, and 0.93 ms of DShot gives both.
  - So a longer window needs the anchor bound (§6.4). The anchor is observable exactly: by the hook's instruction count, and by the timer access line's exact stamp (`4300a` M4).
- **F3 · Memory is the binding cost, not the log or the archive.**
  - Sentinel-02's own models alone take 59–60 % of the 1 GiB bound (`sentinel-02-own`, no motors).
  - DShot adds 4.7 MiB per ms. (b) reaches 90–91 %, and (c)'s device boot 81–82 %.
  - The bound was set at 2× the reference's 404 MB (README); Sentinel-02 is the first board near it.
- **F4 · The bus's time is not modelled.** The twin's I2C and SPI transfers take a few µs of CPU each. On the reference, U2's identity record is written 1.66 µs after the pattern's last byte, and U4's 5.55 µs after U2's. On silicon one 400 kHz byte is about 22 µs. Every window above has milliseconds of slack, so nothing here depends on it (§9).
- **F5 · A rendered reset restarts the anchors, and a pending action outlives it.** Measured on `scenarios/motors/dshot600-droop.json` with a 100 ms window.
  - The `CEN` hook fired at 21.62 µs, and again at 353.32 µs after the release. The instruction count restarted: both read 2 167.
  - The 2 ms action scheduled in the first boot **fired inside the second**, at 2.02167 ms, where the count read 168 997 instead of the 202 167 it was scheduled for. The second boot's own action fired at 2.35337 ms (count 202 167).
  - So the anchors, the judge's actions and their bounds are per boot. An action can tell that it is stale exactly, because the count it sees is not its anchor's + 200 000.

## §6 Recommendation: (a-i)

**Each witness gets the window its evidence needs, anchored where that evidence starts, inside one 100 ms boot on every board.**

1. **Every board boots `MAX_VIRTUAL_NS` (100 ms).**
   - `DSHOT_WINDOW_NS` stops being a board's boot length and becomes the length of DShot's evidence window.
   - `BootPastWindow` and the per-board boot window go: `simulate --for` admits up to 100 ms on every board.
2. **Anchors are observed by the CLI's own watchpoint hooks**, composed in the monitor:
   - each DShot timer's `CR1` write that sets `CEN`;
   - the boot word.

   Each hook logs its instruction count. The judge reads the count and checks it against the timer access line's exact stamp.
3. **At each DShot anchor + `DSHOT_WINDOW_NS`, the judge's scheduled action runs.** It is the judge's own act, recorded as such:
   - it reads the timer's registers back (the read-back `4300a` takes at the end today);
   - it silences that timer's observers;
   - it clears `CEN`.

   Edges after the window are never logged and never evidence. An action whose count is not its anchor's + `DSHOT_WINDOW_NS` × MIPS is stale (a reset came between, F5) and does nothing.

   Measured cost:
   - on `motors-dshot600`, today's cost;
   - on Sentinel-02, its no-motor cost plus 0.7–1.5 s and 30–55 MiB.
4. **The anchors are bounded by the judge, per boot**, each failure by name:
   - the boot's first `CEN` must come after that boot's last data-step witness and before its boot word;
   - `CEN` + `DSHOT_WINDOW_NS` must lie within the run;
   - an observer edge before the anchor in its boot, or a second `CEN` rise in one boot, is refused.

   The evidence that counts is the last boot's, as with today's probes after a release. F2's mutant then fails by name. The bound uses only the run's own witnessed records and the constant, never the generator (`INV-07`).
5. **Serial frames are injected one quantum after the boot word**, by the boot-word hook (measured working, F1).
   - Each later port either waits for the previous port's echo (sequential), or is not pacing-tested (D4).
   - The static wire budget is judged from that instant, and refused by name if it passes 100 ms.
   - The pattern is judged from reset, as today.
   - `serial-dshot-9600` (22.3 ms of wire) and Sentinel-02 (4.9 ms) both fit.
6. **Devices are judged from reset, as today.** The data steps must precede the DShot anchor.
7. **The trace moves to version 13:**
   - kind 26 carries its anchor and its window's end;
   - kinds 28 and 29 carry the injection instant;
   - a record states the judge's stop;
   - replay derives each window from the records, not from the protocols (`chaos/src/campaign.rs`'s `short_window`).

**Why not the others:**
- **(a) static** (a boot length fixed from a bound on the anchors, observers silenced after 2 ms):
  - It costs about the same (4.7–5.0 s, 65–67 %) and needs no judge write.
  - But its bound needs the twin's instructions per spin, 3, a property of the generated loop that `INV-07` keeps the judge from taking.
  - And its injection must land within the echo's poll budget after the real boot word (`ECHO_POLLS` 20 000, about 1–2 ms, inferred), which a static bound cannot promise.
- **(b)** costs 2.4× (a-i)'s wall and 90–91 % of the memory bound, for no extra evidence.
- **(c)** costs about 3× (a-i)'s wall (13.5–14.2 s against 4.0–4.7 s) and twice its trace bytes (12 746 B against 6 434 B), and needs a determinism check between its boots, for no extra evidence.

## §7 Q6 under (a-i): a mixed board

- **Read at source:** a timer's outputs share one protocol and rate (`ir/src/lib.rs`: "the outputs of one timer share its counter"). So stopping a DShot timer never stops a PWM output.
- **What the witness needs:** two rises, so two model periods, with the first edge one period late (`4300a`).
  - The model's timer runs at a fixed 250 MHz. Measured: `motors-pwm`'s 400 Hz output (2.5 ms on silicon) has a model period of 639 744 ns, 3.908 times shorter.
  - So a 50 Hz servo's model period is 5.12 ms, and two are 10.24 ms.
- **It fits.** Under (a-i), a PWM output's window runs from its own timer's `CEN` to two model periods later, inside the same 100 ms boot.
  - On the Sentinel-02-shaped board (anchor 39.07 ms), a 50 Hz servo ends at 49.3 ms. On an `imu-baro`-shaped board (78.0 ms) it ends at 88.3 ms.
  - The slowest PWM that fits after an anchor `t` has a silicon period of (100 ms − `t`) × 3.908 / 2: about 119 ms (8.4 Hz) at `t` = 39 ms.
- **Cost: none beyond (a-i).**
  - The boot is 100 ms anyway, and a PWM timer costs nothing measurable: `motors-pwm`'s 100 ms boot takes 2.05 s, against the reference's 2.21 s.
  - What is refused today (two periods past 2 ms) is witnessed.

## §8 Q8: `Target::Serial`

- **Read at source:**
  - `Target::Serial` already exists in `cli/src/verify.rs`, and each port is listed `no fault kind yet: serial` (never blocking, ruled on #155).
  - **`stress()` lives in `aethelgard-cli/src/verify.rs`**, not in `aethelgard-chaos` as the work order's §3 fence says. This is a fence flag.
  - `detection()` requires `attribute` to name the **electrical** half by the mechanism's rule. Evolve gives a serial failure rule `I6` (indeterminate) when no software cause explains it, and `S5` (software) when one does.
  - `serial-rx-dead` (PA10 on AF8) is a patch of the generated board in the seeded set. `verify`'s campaign runs the ELF it regenerates, so it can never carry that mutant.
- **(i) The work order's literal reading: the seeded mutant as the mechanism.**
  - A port counts as covered when the campaign holds a baseline whose run witnessed that port's echo, and the seeded set (`attribute_seeded.rs`) shows `serial_echo_missing` detected by `S5`.
  - No version or fence change: `verify.rs` only.
  - But it is evidence about the judge on the reference, not about this board under a fault. `3303`'s rule asks for a fault seeded in the board's own campaign, so I would not call it coverage.
- **(ii) An override kind, `serial_rx_open { port }`: the board's RX line open** (`3303`'s "an RX line held, or open").
  - The composition renders it by injecting no frame into that port. It is a board fault, never a firmware mutant.
  - Mechanism: `serial_echo_missing` on that port alone.
  - **Cost:**
    - an `Override` variant `[5, label]` and its record (trace 13, taken with §6);
    - scenario schema 3, or adding it to the list schema 2 already requires;
    - the injection skips the port;
    - `stress()`, `classify()` and `coverage()` move serial ports out of `no_fault_kind` into `unmet` and `covered`.
  - **And one of:**
    - evolve gains `E3` (serial: an open RX line, with no software cause). `aethelgard-evolve` is outside this work order's fence;
    - or `detection()` admits `I6` for a serial target, with that port as its only subject.
  - **Recommended: (ii) with `E3`,** if the architect extends the fence. With neither, ports stay `no_fault_kind`.
- **The seeded firmware mutant stays what it is:** the seeded set's proof that `S5` names a firmware cause. Together with (ii), it separates a dead RX by cause.

## §9 The 3302 synchronisation note

**No verdict depends on the controller's SCL period** (read at source):
- **The analog deck** (`twin/src/analog.rs`) releases each line once, 2.5 µs into a fixed 22.5 µs window at the 400 kHz deck rate. It judges the 30–70 % rise against the bus's ceiling, using the peer's thresholds, whenever the controller sees SCL high. A rise unfinished at the window's end is past every ceiling (≤ 1 000 ns).
- **`ir/src/silicon.rs`'s `TIMINGR` rule** judges the shortest period: `tSYNC` at its 2-cycle minimum, slopes not credited. A controller that waits for its own 1.7 V threshold only lengthens `tHIGH`, so every bound the rule checks holds a fortiori. `data_valid`, which a longer period could touch, is reported but does not gate.
- **The twin's I2C transfer takes no bus time** (F4), so no window edge depends on it either.

**Recorded as a model divergence for architecture §2** (§12): the H7 times `SCLH` from its own pin's threshold (about 1.7 V) by clock synchronisation, which Renode does not model. The campaign's rise verdict uses the peer's thresholds and is unaffected. On silicon a slow pull-up lengthens the clock: 8 µs against 2.6 µs at 100K (3302).

## §10 Predictions, scored (§0 is unedited)

| # | Verdict |
|---|---|
| P1 | **Missed** by about 10×: the boot word is at 25.75 µs, not 0.2–1.0 ms. Held: the pattern precedes the steps, and the last echo is written 0.54 µs after the last byte goes in. |
| P2 | **Missed** on the scale (21.67 µs); **held** on the gap (80 ns). |
| P3 | **Held:** refused today by the wire budget (22.3 ms); its phases match P2 plus the pattern. |
| P4 | **Held:** settle 77.95 ms; data steps and boot word at 78.0 ms. |
| P5 | **Held:** settle 38.98 ms, `start_outputs` 39.07 ms, wire budget 3.86 + 1.04 ms. |
| P6 | **Held, but incomplete:** about 0.10 s per ms of DShot with the observers silenced. I missed the memory, 4.7 MiB per ms, which is what limits DShot. |
| P7 | **Held in shape:** (a) took 4.7–5.0 s and 2.9–7.1 MB. The boot ends at 44.1 ms, not 41 ms, because the wire budget counts from the injection. |
| P8 | **Held:** silenced, 9.8 s, 5 s more than (a); unsilenced, it floods. I did not predict 90–91 % of the memory bound. |
| P9 | **Held on wall** (13.5–14.2 s, measured). **Missed on trace:** 12 746 B, not 2×: each boot writes a whole archive. |
| P10 | **Held, refined:** (a), as (a-i). The judge can bound the anchor, and it must (F2); per boot (F5). |
| P11 | **Missed:** the cost is about 0, because under (a-i) the boot is 100 ms anyway and DShot is stopped. I also took the period as silicon's: 50 Hz is 5.12 ms in the model. |
| P12 | **Held in substance.** I did not foresee the fence or evolve's `I6`/`E3`. |
| P13 | **Held.** |

## §11 Decisions for the architect

- **D1 · The candidate.** (a-i), recommended (§6); or (a) static, (b) or (c), at the costs in §4.
- **D2 · The judge's stop.** Clearing `CEN` on each DShot timer at the end of its window is the judge acting on the machine.
  - It is a new twin-detection oracle after the window, of `4300b` Q9's class: a firmware that reads its timer later sees it stopped.
  - Generated firmware never touches a timer after `start_outputs` (read at source, `firmware.rs`). ArduPilot (`4400c`) would.
  - The alternative is silencing alone, at (b)'s memory.
  - Accept the stop, with that residual named in every verdict?
- **D3 · The first monitor Python the CLI composes.** (a-i) needs, in Renode's IronPython:
  - two watchpoint hooks;
  - one `ScheduleAction` per DShot timer;
  - the injection's actions.

  `RUL-06`, as corrected, says no Python hook exists, and nothing forbids one. They run inside the existing sandbox and touch only the machine.

  Build rules I would add:
  - an observer edge with no anchor observed fails by name, because a `CR1` write narrower than a word may not fire a `DoubleWord` hook (not measured);
  - every action is per boot (F5).

  Accept, or require (a) static, which needs no Python but takes the generator's instructions per spin?
- **D4 · The serial injection after the boot word** (F1): sequential per port (every port paced) or simultaneous (only the first port pacing-tested)?
- **D5 · Q8:** (ii) with `E3` (the fence extended to `aethelgard-evolve`), (i), or `no_fault_kind` stays.
- **D6 · The memory bound.** Sentinel-02 takes 63–65 % of 1 GiB under (a-i), and 59–60 % with no motors at all. I propose no change, but the next model of our own may cross it. Raise the bound, or record each board's margin in the verdict?
- **D7 · Fences.**
  - (a-i) changes `aethelgard-twin/src/boot.rs` (`window()`, and what `DSHOT_WINDOW_NS` means), which is outside §3's fence.
  - `stress()` is in `cli/src/verify.rs`.
  - Q8 (ii) needs `aethelgard-evolve`.
- **D8 · Versions.** Trace 13 for (a-i), and Q8's override with it. No specification change.

## §12 For the docs (protected, so recorded here)

- **Architecture §2.17:** "about 1 KB per scenario" is now 3.4 KB on the reference, still independent of virtual time.
- **Architecture §2:** add the clock-synchronisation divergence (§9) and the bus-time divergence (F4).
- **The CLI README** documents only version-1 scenarios. The build brings it up to date (WO §2).

## §13 The stop

- **Status:** STOPPED at the spike (WO §1). Nothing is built, and nothing in `crates/`, `ci/` or `scenarios/` changes. The recommendation is (a-i) (§6), and §11 lists the decisions.
- **Base:** `1178ca8`; `main` had not moved at the stop.
- **Gates on `6c7c9c3`:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` all pass (1 934 passed, 0 failed), on the host under a capped user scope.
- **The spike fixture** `spike/2302/fixtures/sentinel-02-shaped.json` places the W25Q64JV and carries no `accept_lifecycle` entry. Once `4211` merges (the row becomes `nrnd`), it needs one before it can move under `crates/`.
- **Re-signalled.** After the first STOPPED push (`7eff16f`), every candidate's `OQ-03` trace bytes and wall time were measured as campaign archives, three runs each (§4; `spike/2302/sh/camp2.sh`), in place of the inferred figures: (a) 6 386 B, (a) silenced 6 408 B, (a-i) 6 434 B, (b) 6 408 B, (c) 6 360 + 6 386 = 12 746 B. (c)'s inferred 8.1 KB was wrong, because each boot writes a whole archive. The recommendation is unchanged.

# The build (ruled on PR #175, comment 6923968111)

## §14 What the ruling asked, and where it is built

| Ruling | Built | Where |
|---|---|---|
| D1 (a-i): every board boots 100 ms; per-witness windows anchored where their evidence starts | `boot::window()` removed; `DSHOT_WINDOW_NS` is DShot's evidence window from its anchor; `BootPastWindow` gone; `--for` up to 100 ms on every board; campaign and verify boot 100 ms | `twin/src/boot.rs`, `cli/src/{main,renode,campaign,verify}.rs` |
| D2: the judge clears `CEN` at anchor + `DSHOT_WINDOW_NS`; the residual named | the stop reads the timer's solved registers back, then clears `CEN` (it never silences the observers: adversarial H1); `boot.residuals` names the residual on every board with a DShot output | `cli/src/anchor.rs`, `renode.rs` |
| D3: composed monitor Python; an edge with no observed anchor fails by name; every action per boot, stale does nothing | `anchor.rs` composes the hooks (CR1 at DoubleWord, Word and Byte width; the boot word; each port's `TDR`) and the scheduled actions; one strict line parser; every action checks the instruction count it was due at and logs `STALE` otherwise | `cli/src/anchor.rs` |
| D4: sequential serial injection; the summed wire budget from the boot word past 100 ms refused by name | port 0 one quantum after the boot word; port *k* after port *k*−1's last echoed frame byte (counted only when it is the frame's next byte); `serial_wire_past_run`, `serial_injection_out_of_order` | `anchor.rs`, `renode.rs` |
| D5: `serial_rx_open { port }` with `E3`; scenario schema 3 | `Override::SerialRxOpen` `[5, port]`, schema 3 only; rendered by injecting nothing into that port (the chain skips it); `stress()`/`classify()`/`coverage()` map it to `Target::Serial` with mechanism `serial_echo_missing`, rule `E3`, subject that port; serial ports leave `no_fault_kind`; evolve rule `E3`; the reference's seeded set gains `serial-rx-open-console` | `chaos/src/{record,campaign}.rs`, `cli/src/verify.rs`, `evolve/src/lib.rs`, `scenarios/` |
| D6: each board's peak memory share in the verdict | the reaper reports the tree's peak RSS; the verdict's `memory` carries `peak_rss_kib`, `bound_kib`, `share_permille` (outside the archive: it varies run to run) | `cli/src/run.rs`, `verdict.rs` |
| D7: fences | as ruled; outside them only evolve's test fixtures (record fields) and `twin/src/serial.rs` (one dead function) | |
| D8: trace 13 | kind 26 gains `anchor_ns`, `edges_before`, `edges_after`, `cen_rises`; kind 29 gains `injected_ns`; new kind 30 (the judge's stop); replay derives each window from the records; version-12 archives refused by name | `chaos/src/{record,event,campaign}.rs` |
| spike deleted | `spike/2302` deleted | |

**How it was built.** I ran it as project manager, delegating to workers one at a time in their own worktrees, since the judge is gate logic and two agents must never edit one file:
- S1, the core (Opus);
- S2, Q8 (Opus);
- two fix rounds (Opus);
- two fresh adversarial passes (Opus), each given the code, never my conclusions;
- a fresh reduction and mutation pass.

I spot-checked every worker's claims at source before merging. `main` was merged three times with merge commits: docs #174; then 4211, 1207, 4403 and the 4230 refit; then 4220b, specification 13. On the third merge the Sentinel-02-shaped fixture moved to specification 13, carrying `sentinel-02-own`'s declared pin capacitances.

## §15 Decisions I took inside the ruling (for the architect to confirm)

- **The seeded case `identity-late-dshot` is retired**, with its mutant.
  - Its premise, a device window shorter than the start-up, is what this work order removes. Under 100 ms boots its firmware passes (attribute `N1`).
  - The alternative, C1, was built and measured, then reverted. It names `identity_read_outside_window` on every board for a read still unset at the boot's end, and lengthens the mutant past 100 ms.
  - That kept `I7`, but it changed two reference tests' exact findings: the unbooted fill record, and a 1 µs `--for`. It also needed an evolve change outside `E3`.
  - **Left unreachable, outside the fence:** evolve's `I7`, chaos's `short_window` and `OUTSIDE_WINDOW_FINDING`. Remove them, or keep them for a future short window?
- **Replay keeps no tie between kind 10 and the 100 ms instructions.** A droop's boot counts from its last release (existing rule).
- **A scheduled action's line must be stamped exactly at its instant** (measured exact in every probe). A hook's line keeps a one-quantum tolerance; hook lines were measured up to 60 ns early.
- **The open-port chain.** A port held open (`serial_rx_open`) gets no frame and no `TDR` hook. The next port goes in after the last earlier port that is not open. Residual: that next port's frame then waits in Renode's queue while the firmware polls the open port, so it is not pacing-tested in such a run (F1's lost test); its echo is still judged.
- **`E3` yields to software.** A port that is held open and also has a software cause attributes `S5`, with nothing masked. That is the inverse of `E1`/`E2`, where the electrical half wins and software is masked. It cannot cause false coverage, because `detection()` then fails.
- **The stop never silences** (adversarial H1, below), and every `CEN` rise schedules its own stop. A restart after the window is a second act on the machine, and it is already named (`motor_anchor_repeated`, `motor_edge_after_window`).

## §16 The adversarial passes

**Pass 1** (fresh Opus, on `fbb4e88`). WO §5's four attacks, F2 and F5 were measured, and every reproduction was kept. Three false passes, all fixed:

| id | sev | the attack, measured | disposition |
|---|---|---|---|
| H1a/b/c | high | After the window: the motor pins bit-banged through GPIO; `CEN` restarted by a 16-bit store with bad `CCR`s; `CEN` restarted by DMA, then each restored. **All passed**, because the stop silenced the observers and `motor_edge_after_window` was unreachable: truncation, which the work order forbids. | **Fixed:** the stop never silences, and every edge after a window is named `motor_edge_after_window` (three tier-2 tests). |
| M1 | med | A core that sleeps (WFI) before `start_outputs`: the hook's count-against-stamp check was a ToolError, so a firmware could abort a campaign. Without a boot word, the pre-2302 `cpu_halted` + `boot_word_missing` was lost. | **Fixed:** a lagging count is `cpu_halted`, its line falls back to its stamp, and an action due while the core lags goes stale and fails by name. Still exit 5 when the boot word is written: that is `2100`'s ruled A10 path, not new (§18 Q1). |
| M2 | med | `CEN` set by a 16-bit or byte store, or by DMA: no anchor, no stop; it failed closed only under the flood's names, at 930–991 ‰ of the memory bound. | **Fixed for the CPU:** `CR1` hooks at Word and Byte width too, measured to fire once per store. A DMA start stays fail-closed (`renode_log_flood` + `motor_output_unwitnessed`) at 98.6 % of the bound, named as a residual in the README. |
| L1/L2 | low | The `TDR` hook counted any write after the boot word, so pattern or junk bytes triggered the next port early, and the order was not judged. | **Fixed:** only the frame's next byte counts, and `serial_injection_out_of_order` is named. |
| L3 | low | Replay accepted archives that added or dropped six judge findings. | **Fixed:** kind 26 carries `edges_before`, `edges_after` and `cen_rises`. Replay binds `judge_stop_mismatch`, `motor_anchor_repeated` and both `motor_edge_*`; what stays unbound is listed in the README. |

**Measured regression on pass 1's tree:**
- **§5.1, a delayed `start_outputs`:** the window follows the anchor; F2 below.
- **§5.2, emitting before the anchor:** GPIO toggles give `motor_edge_before_anchor`, and a boot word before the anchor gives `motor_anchor_out_of_order`.
- **§5.3, an echo straddling the run's end:** `serial_wire_past_run` + `serial_echo_missing`.
- **§5.4, a device read at the window's edge:** before the anchor it is `motor_anchor_out_of_order`; pushed to the end it is `boot_word_missing`.
- **F2:** an anchor at exactly 98 000 000 ns passes, with the stop at 100 ms; 98 000 030 ns gives `motor_window_past_run`.
- **F5:** the droop's first-boot stop goes stale and does nothing. `CEN` set, cleared and set again, or set again after the stop, gives `motor_anchor_repeated`.

**Pass 2** (a second fresh Opus, on `75e2207`): **no false pass, and no firmware-chosen window.** Four findings, all fixed except as noted:

| id | sev | finding | disposition |
|---|---|---|---|
| A2-1 | low | Kind 30's read-back carried its own expected value, mask and address, bound to nothing. | **Fixed:** bound to kind 11, register by register, covering exactly the timer's solved set. |
| A2-3 | low | A twin false-fail. `serial_wire_past_run` counted each port's pattern after the boot word, though the firmware sends it before. Band: a boot word at about 77.6–90 ms on a 9600 console. | **Fixed:** from the boot word only frames and echoes count. Tier 2: boot words at 79.53 and 87.03 ms pass; at 94.53 ms it is still named. |
| A2-4 | low | `TIM1.BDTR.MOE` toggled after the window moves no observer: a Renode tag, already in `unobservable`. CCMR forced modes are caught. | README, beside the post-window text. |
| A2-5 | low | `serial_wire_past_run` and an anchor after the boot word are decidable from the records. | **Fixed:** bound in replay. |
| A2-2 | low, older than 2302 | A booted run may drop a kind 26 with its finding: `check_motor_joins` asks for "at most one". | Backlog (§18). |
| A2-6 | info | `strd`/`stm` anchor; a 64-bit `vstr` is dropped by Renode (`register_mismatch` + unwitnessed); unaligned stores are `cpu_aborted`. | Nothing sets `CEN` unseen. |

Pass 2 re-ran §5, F2 (98 000 000 / 98 000 020 ns) and F5 on its tree: every case fails by name or passes legitimately.

## §17 Measured at the end

- **The Sentinel-02-shaped board under (a-i)**, witnessed whole:
  - peak 704 096 KiB, 67.1 % of the 1 GiB bound;
  - 2.85 MB of log, nothing dropped;
  - `CEN` at 39 072 330 ns, the boot word 80 ns later;
  - MAVLINK injected at 39 172 410 ns, CRSF after MAVLINK's echo.
- **Each verdict now carries `memory.share_permille`** (D6): `motors-dshot600` 406 ‰, `serial-dshot-9600` 413 ‰, the DMA-start mutant 986 ‰.
- **Tier 2: 160 on `1178ca8`; `TIER2_MIN` is 175 after merging `main`** (161 with 4220b).
  - Added: 15 in `twin_tier2`: the 100 ms windows, H1a/b/c, M1 ×2, M2 ×2, L1 ×2, A2-3, the open RX line, the Sentinel-02-shaped board, `serial-dshot-9600` and F2.
  - Removed: 1, the 2 ms DShot boot.
  - Interim weights for the new tests are in `ci/test-weights.tsv`, for refit from CI.

## §18 Gates, reduction and open questions

- **Reduction pass (diff): −84 net · 8 duplicates removed · 2 dead paths removed · 6 protected kept · 5 findings.** A fresh worker ran it on `969dd168`, five commits. The findings:
  - (1) chaos's `short_window`/`OUTSIDE_WINDOW_FINDING` and evolve's `I7` are unreachable, and outside the fence (§15).
  - (2) The verdict's and campaign lines' `window` field now always says `MAX_VIRTUAL_NS`. Removing it is a schema change, which needs a ruling.
  - (3) The DShot window's end is computed in three places in `renode.rs`.
  - (4) `E3` has guards no test can tell from their absence (below).
  - (5) Some older chaos tests used `.is_err()` where several guards refuse.
- **Mutation sweep (one mutant per guard this work order added, its own suites, all tier 1):** 88 mutants, counted from the worker's tables. The harness and logs are in `~/.cache/aet-2302/red-mut/`.
  - **Killed:** 70 on the first run. 12 more (R08, R17, R20, R22; C02, C03, C05, C06, C08, C10, C21, C33) were killed after three tests were added: e3f77a3d, a236fc2d and 684b0020.
  - **Survived:** 6, `E3`'s E02–E07 (`booted`, `holds`, `t.holds()`, tags `Some`, and the two injection-`None` guards). They are equivalent, because other rules or replay decide those cases first.
    - 606c1244 pins the observable behaviour (I6 and S5) but does not kill them.
    - They are reported, not removed: unlabelled redundancy is never cut, so the architect may keep or drop them.
  - **Fix round 2's own mutants**, all killed, are separate from these 88: A2-1 3, A2-3 2, A2-5 7.
  - Fix round 1 recorded no mutation run of its own; its guards are among the 88.
- **Open questions for the architect:**
  - **Q1:** a core that sleeps and then writes its boot word still exits 5, by `2100`'s ruled A10 ("boot word set but the core executed fewer"), so a firmware can abort a campaign that way. Unchanged by this work order; pinned by `m1_a_core_that_slept_and_wrote_its_boot_word_is_the_ruled_disagreement`.
  - **Q2:** `I7` and `short_window`: remove, or keep for a future short window? (§15)
  - **Q3:** the new motor and serial finding kinds have no `attribute` rule, so `attribute` reports them unexplained (`I1`). Add rules in evolve, outside this fence?
  - **Q4 (closed by measurement, after the first CI run):** the DMA-start test was red on shard 0 of run 44007663813 (job 133786836234).
    - **Root cause, measured: the reaper's 30 s wall bound, not memory and not the fixture.** The verdict carried exactly one finding, `wall_bound_exceeded`, at 34 s, with peak memory at 851 ‰. Renode had exit -1 and every observed field was null, because a killed boot is never read.
    - The fixture started DShot by DMA at about 22 µs. With no anchor there is no stop, so four DShot600 channels flooded for about 98 ms. That is the cost `4300a` measured (10.8 s of timer events per 100 ms on Furnace); Furnace finished in 16.8 s, while a loaded CI shard passed the bound.
    - **The test asserted a race against the machine's speed**, which roadmap §6 forbids.
    - **Fixed** by forcing the outcome: the DMA start now waits 3 230 000 spins (about 96.9 ms, inferred from the spin cost), so about 3 ms of unanchored edges stay inside the log budget, and every output is `motor_anchor_unobserved`. Measured 3 of 3 in the pinned image: 6.6–8.3 s, 41–43 % of the bound.
    - The early start stays a named residual in the README. It fails closed only by a bound, and which bound depends on the machine: the flood's names on Furnace, `wall_bound_exceeded` on CI.
    - No bound is raised and no re-run was asked for.
  - **Q4b (closed by measurement, after the second CI run): what kills Renode at about 40 s.** On run 44012579895 (job 133802886211), shard 0 failed `f2_a_late_dshot_anchor_fails_by_name` with the same symptom: Renode exit -1, every observed field null, 40.68 s of call wall, peak memory at 602 ‰.
    - **The signal is the CLI's own reaper.** The verdict's only finding is `wall_bound_exceeded`, which only the reaper names when a boot's tool passes `BOOT_BOUNDS`' 30 s wall (read at source, `run.rs` and `renode.rs`). Nothing else is involved: no runner limit, no sandbox and no watchdog.
    - **Why the boot is slow: test concurrency, measured.**
      - libtest runs a binary's tests on every CPU of GitHub's 4-vCPU runner. In that job's `twin_tier2` section, ten tests ran four at a time.
      - Single boots that take about 4.4–6 s alone on Furnace took 18.6–24.1 s each on CI.
      - The late-anchor boot is heavier in itself: the Sentinel-02-shaped board has our own C# models, two serial ports with access logs, and DShot. Its boot word at 99 ms means the frames never go in, so the firmware polls each USART's `ISR` `ECHO_POLLS` times under access logging.
      - It takes 7.9–8.3 s alone on Furnace. Reproduced with the same ten tests pinned to 4 CPUs (`taskset -c 0-3`) in the pinned image: siblings 5.9–8.3 s, late anchor 14.7 s, the same 1.8–2.5× ratio as CI. Furnace's faster cores keep it under the bound; CI's did not.
    - **What 6fde5a62 fixed was the same cause, by another driver.** m2's early DMA start made its own boot long (about 98 ms of flooding DShot: 16.8 s alone on Furnace), and moving the start late removed that. Under contention m2 still took 24.1 s of the 30 s on CI. This work order's heavier boots exposed contention that was already near the line: before 2302, a DShot board booted 2 ms.
    - **Fixed without raising a bound:** `twin_tier2` runs one Renode at a time (`RENODE_TURN`), every call that runs Renode taking the turn. That is the condition the bound was measured for, and roadmap §6 asks a test to force the condition it asserts.
      - Measured with the lock, same pinned reproduction: every `simulate` held the turn 4.2–7.8 s (the late anchor 7.79 s, its alone time). The ten tests' binary wall went from 24 to 53 s; the shard plan already weighs these tests at their alone times.
      - Each call's time holding the turn is printed (`ANSWER 2100: turn: …`), so a CI log shows the margin.
      - Residual: other tier-2 binaries (`campaign_tier2`, `with_tools`) still boot in parallel. They are green on both CI runs and their boots are reference-sized; no lock was added there without a failure.
  - **Q5 (backlog, older than 2302, A2-2):** replay should require exactly one kind 26 per kind 24 in a booted run.
  - **Q6:** `E3` yields to `S5`, with nothing masked (§15): confirm.

## §19 CI and the close

- **CI run 44016616340 on `831139c1`: green.** `gates` 24.5 min; the eight tools shards 23.3–33.6 min (0: 28.7, 1: 29.3, 2: 23.3, 3: 24.1, 4: 27.0, 5: 33.6, 6: 27.0, 7: 31.1). `engine-image` and `ardupilot-image` are green too.
- **Refit rule:** no refit is triggered.
  - The plan is balanced at 1 665 s per shard.
  - The slowest shard was 3 on run 1, 4 on run 2 and 5 on run 3, never the same shard on two consecutive runs. On run 3, shard 5 led shard 7 by 2.5 min, under the rule's 3.
  - The interim weights stand until a refit is due.
- **The bound's margin on CI, with one Renode at a time** (115 calls printed across the shards):
  - `simulate` held its turn for a median of 6.57 s;
  - this work order's heaviest boot, the Sentinel-02-shaped board witnessed whole, held it 18.61 s, 62 % of the 30 s bound;
  - the longest was 26.13 s: the pre-existing `an_aborted_machine_returns_inside_the_bound_as_cpu_aborted` (`2100` A6), which is built to run until its machine aborts and returned inside the bound;
  - `prove` (Kani, unbounded) held the turn for up to 74.93 s.
- **Earlier runs:**
  - run 44007663813 on `9261c79d`: shard 0 red, Q4;
  - run 44012579895 on `40f381e7`: shard 0 red, Q4b.

  Both were measured to their cause before any change; neither was re-run.
- **Housekeeping:** every worker worktree and target is removed, and `~/.cache/aet-2302` keeps only logs and evidence (146 MB).

## §20 The READY head's red, and its cause (the architect's diagnosis, confirmed)

- **What failed.** The READY head `e3728ee5` was red on shard 2 (job 133829516434). `an_aborted_machine_returns_inside_the_bound_as_cpu_aborted` (`2100` A6) took 62.45 s against its 30 + 30 s assertion.
  - Its verdict was right: exit 1, only `cpu_aborted`, the machine aborted at 0.0008 s.
- **The cause was my measurement, not the twin.** The test started its clock before `boot_elf`, which takes `RENODE_TURN` (53c036db) inside. So `took` included the queue behind other tests' Renode runs.
- **The fix.** Every `aethelgard` call records its own time, from the moment it holds the turn to its exit (`last_call()`, per test thread), and every timing reads that, never a clock started before the turn.
  - The A6 test asserts on it.
  - `boot_on`'s and the 2200 runs' `wall_s`, and the two Kani loops' times, now print it.
  - `run_campaign` starts its clock after the turn.
- **The audit.** `RENODE_TURN` is used only in `twin_tier2.rs`. Of its seven clocks, only the A6 test asserted on a time; the rest only printed. The only clock left outside the turn times a `cargo build`, which takes no turn.
- **Measured.** The whole `twin_tier2` binary in the pinned image, pinned to 4 CPUs (`taskset -c 0-3`, GitHub's runner size): 82 of 82 pass.
  - A6's own call: 16.97 s.
  - 87 `simulate` calls: median 4.49 s, longest 16.97 s (A6).
- **No bound raised, no re-run.** The host gates pass: `fmt`, `clippy -D warnings` with and without `with-tools`, and 1 995 tier-1 tests.
