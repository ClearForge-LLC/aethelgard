# AET-WO-2303 — FEEDBACK (spike; stops at §2)

Branch `wo/aet-2303-dynamic-volume`, from `main` at `ecc6be95`. The work order was read from docs PR #198 and is on `main` since #198 merged (`19741d21`), unchanged; this branch merges it. Prototypes live under `spike/2303/`; no crate changes.

Every answer is labelled **measured**, **read at source** or **inferred**.

## §1.1 The question first, at source: the faults `4500`'s campaign needs

**Where the list comes from (read at source):**
- roadmap §5, `AET-WO-4500`: "a fault campaign that includes an SPI fault". The P4 exit gate's second clause asks for "a **static** fault campaign whose electrical and SPI faults are attributed correctly (`INV-05`)".
- `AET-WO-3303`'s coverage rule, `crates/aethelgard-cli/src/verify.rs` L872–946: `--fab-out` refuses a campaign that leaves a declared I2C bus, SPI bus, serial port or rail uncovered. Rails other than the MCU's supply read `no fault kind droops it`, and so does `battery` once a power tree is declared.
- The override kinds in `crates/aethelgard-chaos/src/campaign.rs` L565–577: `pull_up_ohms`, `max_capacitance`, `line_held_low` (SDA, SCL), `droop`, `spi_device_absent`, `serial_rx_open`.
- `2301`'s fault kinds (`docs/work-orders/AET-WO-2301.FEEDBACK.md`): the declared droop (built), and the interrupt storm (P3, deferred to the first firmware that takes an interrupt).
- The fixture `crates/aethelgard-cli/tests/fixtures/sentinel-02-shaped.json`:
  - parts: BME280, TMP117 and BMP581 on I2C `SENSORS`; W25Q64JV on SPI `FLASH`; ICM-42688-P on SPI `IMU`;
  - four `dshot600` motor outputs on J3;
  - serial ports `MAVLINK` and `CRSF`;
  - domains `VDD_3V3` (the MCU's supply, fed from J2 pin 1), `VDD_5V` and `VDD_1V8`;
  - `power: null`, so it declares no battery.
- The roadmap rows owned by `4500` or the battery-sag work order: L559, L627, L646–648, L658, L673 and L674.

**Three facts that bound the table (read at source):**
- **F-a.** The fixture declares no power tree. A campaign over one that does is refused: `PowerNotSimulated`, `crates/aethelgard-cli/src/campaign.rs` L379–391. `4500`'s Sentinel-02 flies a 3S–4S pack (roadmap §5), so its specification will declare one.
- **F-b.** The P4 exit gate's own word is "static".
- **F-c.** The generated firmware's DShot output is "a continuous zero-throttle pattern, with no DMA":
  - after `CR1 = ARPE | CEN` nothing reads or writes the timer (`crates/aethelgard-svd-hal/README.md` L64; `AET-WO-4300a.md` L54);
  - the timer's `CEN` is set once per boot, and a rendered reset (`machine Reset`) clears it.

  So what the firmware commands during a run is: whether frames go out (`CEN`), and the frame's throttle (`CCR`, constant zero).

**The classification:**
- **static:** the analog side is computed once, before the boot, from declared values.
- **declared:** the analog side moves in time, but its waveform is a function of declared inputs only (`2301`'s droop).
- **firmware-dependent:** the electrical effect depends on what the firmware does during the run.

| # | Fault | Where it comes from | Fault kind today | Class | Why |
|---|---|---|---|---|---|
| 1 | Pull-up outside its band on `SENSORS` | 3303 coverage (I2C bus) | `pull_up_ohms` | **static** | The rise time is computed before the boot (`RUL-17`, §2.17) |
| 2 | A line's capacitance over its bound | 3303 coverage (I2C bus) | `max_capacitance` | **static** | As row 1 |
| 3 | SDA or SCL held low | 3303 coverage (I2C bus) | `line_held_low` | **static** | Rendered as the device's absence before the boot (`RUL-17`) |
| 4 | An SPI device absent (`U4` or `U5`) | `4500`'s "SPI fault"; 3303 coverage (SPI buses `FLASH`, `IMU`) | `spi_device_absent` | **static** | The device is left out of the platform |
| 5 | A serial port's RX line open | 3303 coverage (`MAVLINK`, `CRSF`); roadmap L647 | `serial_rx_open` | **static** | The line's state holds for the whole run. The peer's frames are a digital stimulus timed from the boot word; nothing analog moves |
| 6 | `VDD_3V3` (the MCU's supply) droops | 3303 coverage (MCU rail); `2301` | `droop` | **declared** | `2301` D1 and D8, ratified: solved once, applied at its crossings, nothing per quantum (`OQ-03`, §8) |
| 7 | `VDD_5V` or `VDD_1V8` droops | 3303 coverage: `no fault kind droops it` | none | **declared** (inferred) | Each rail's loads are the specification's declared loads and part-table rows (`aethelgard-ir/src/power.rs` L37–48); the firmware draws nothing from them that it chooses at run time |
| 8 | The battery sags under the motors' current, and through the regulator `VDD_3V3` with it | 3303 coverage (`battery`); roadmap L559, L648, L658, L673; `4500`'s 3S–4S pack | none (`PowerNotSimulated`) | **firmware-dependent** | See below |
| 9 | An interrupt storm | `2301` P3 | none (deferred) | not analog: **outside `OQ-03`** | The generated firmware enables no interrupt (`2301` P3, measured). A storm is digital stimulus; no solver runs per quantum |
| 10 | A motor pin's off-board load (ESC input plus cable) | roadmap L627 (4300a Q3, owned by `4500`) | none | **static** | A capacitance on a timer output: a rise time and Fmax check before the boot |
| 11 | J2's current rating against the battery sag | roadmap L673 (4300g K7) | none | **static** | A rating judged against a declared or computed current, with nothing in time |
| 12 | A covering fault moved past its declared tolerance | roadmap L646 (3303 F3) | rule over rows 1–2 | **static** | A rule on the static kinds |
| 13 | An off-board device absent (GPS compass on DS-009) | roadmap L635 (4300f D9) | `spi_device_absent`-like; none on I2C yet | **static** | As row 4 |

**Row 8: why the battery sag is firmware-dependent (read at source, with one inference marked).**
- On a quadcopter, the battery's current is the four ESCs' current. An ESC's current follows the throttle it is commanded on DShot, and it goes to zero when the commands stop.
  - That this is the dominant load is **inferred**: no ESC or motor part is declared anywhere in the repository.
- Who commands it is the firmware, during the run:
  - today, by setting the timer's `CEN` once per boot (F-c);
  - under ArduPilot, every control loop.

  Two firmwares on the same board draw different battery waveforms.
- **The loop closes.** A sag deep enough to cross the regulator's dropout drops `VDD_3V3` below BOR. The rendered reset clears `CEN`, the commands stop, the current falls and the battery recovers. The firmware boots again, sets `CEN`, and the load returns.
  - The waveform therefore depends on when the firmware boots and starts its timer: a reboot loop.
  - It cannot be solved before the boot, as `2301`'s droop is, without leaving the coupling out.
- **It can be declared instead, at a cost.** Roadmap L559 words the battery-sag chain as "fed into `--rail` from declared parts": declare a current profile as a stimulus, as `2301` declares its load.
  - That is honest as a stimulus.
  - It cannot show the reboot loop, because a reset that stops the motors would not relieve a declared load.

  Which reading `4500` takes is decision D1 below.
- **Today's firmware makes the dependence weak.** The command is a constant zero throttle, so a real ESC would draw its idle current. What the firmware controls today is only *when* frames are present.
  - **Nothing in the repository declares an ESC's supply current per throttle or per frame presence**: no part row, and 4300a Q3's motor load is the ESC's *input*.
  - The prototype therefore uses a **spike-declared** load map, labelled as such, to close the loop. Its magnitudes are not sourced.

**Result of §1.1: one row (8) is firmware-dependent, so §1.2 runs.** `OQ-03` is not left answered by default.

## Predictions — written before any measured run

### The prototype, as designed (`spike/2303/`)

- **Starting point.** `2301`'s lock-step probe at `a451cd15` (`spike/2301/lockstep`, `spike/2301/ngspice`), restored under `spike/2303/` with its FFI (outside the workspace, as `2301`'s was).
  - It runs the `ioc` route: libngspice in-process, a breakpoint at every 10 µs boundary, and Renode as the probe's child, run one quantum at a time from `GetSyncData`.
  - Each quantum is sent as one repeated `;`-joined monitor line with a constant sentinel (`2301` F15).
- **The board and the firmware.**
  - The Sentinel-02-shaped fixture, generated and built by the unmodified CLI (`aethelgard generate`, `aethelgard build`).
  - Its Renode command line is captured from the unmodified CLI's own `simulate` run, then driven by the probe one quantum at a time instead of in one `RunFor`.
  - The judge's DShot stop at the anchor plus 2 ms (`2302`) is left out, because the load must follow the firmware's timer for the whole run. The DShot observers' logging is silenced, as `2302` measured. Both are recorded as deviations.
- **The deck (spike-declared, not sourced):**
  - the battery: a 3S pack at 11.1 V with a series resistance R_pack (the fault: a failing pack or connector) into a bulk capacitor;
  - the motors' current: a current source driven by an external control voltage the probe sets at each boundary, through a first-order lag (τ, the ESC and motor's current rise);
  - the regulator: behavioural, `VDD_3V3 = min(3.3 V, V_bat − dropout)` through a small output resistance into the emitted BOM's decoupling (400 nF), with the MCU's 100 mA load as in `2301`;
  - BOR3, DS12117 Rev 7 Table 26 typical (2.61 V falling, 2.70 V rising), as `2301`.
- **Each boundary:**
  1. Renode runs the quantum.
  2. The motor timer's `CR1` and one `CCRk` are read.
  3. The load is set from them: I_run when `CEN` is set, 0 otherwise. The `CCR` is recorded, and with the zero-throttle pattern it does not scale the load.
  4. ngspice's `V_bat` and `VDD_3V3` are sampled.
  5. The bridge applies `2301`'s held-reset rendering on a BOR crossing: `machine Reset`, the cold writes and `VTOR`, halted, then released with the 10 ns nudge.
- **The archive.** It is written with the real `aethelgard-chaos` `TraceWriter` in several shapes, and replayed with `replay_dir` and with the CLI's `replay` (below).

### Cited, not re-measured (read at source, `2301` FEEDBACK P4 and F11)

- One kind 3 (`AnalogSample`) per quantum: 20.0 B, **2.0 MB per simulated second** at 10 µs.
- One labelled record per quantum (kind 8 as the stand-in): 31.0 B, **3.1 MB/s**.
- The fold: about 428 B per record. The writer refuses the 65,537th record (`MAX_RECORDS`), so one record per quantum stops at **0.66 s**.
- Lock-step cost: 1.4 ms per quantum with one read per quantum, so 14–15 s per 100 ms.

### Predicted

| id | Quantity | Prediction |
|---|---|---|
| Q1 | Per-quantum quantities | Three: `V_bat`, `VDD_3V3`, and the commanded load (the timer state). The new one against `2301` is the third: what the firmware commanded |
| Q2 | Bytes per simulated second, as kind 3 samples (three per quantum) | About 60 B per quantum: **6.0 MB/s** ±15 %. Two per quantum: 4.0 MB/s |
| Q3 | Bytes per simulated second, as stand-in records (three per quantum) | About 93 B per quantum: **9.3 MB/s** ±15 % |
| Q4 | Bytes per simulated second, one combined record per quantum | 35–45 B: **3.5–4.5 MB/s** |
| Q5 | Event records (resets, releases, `CEN` changes, crossings) | Under 1 KB per reboot cycle, and tens of cycles per second at most: under 100 KB/s, negligible beside Q2–Q4 |
| Q6 | Records per run | Records shape, three per quantum: 300,000 per simulated second. Combined: 100,000 |
| Q7 | Where `MAX_RECORDS` (65,536) stops a run | Three records per quantum: **0.22 s**. Combined: 0.66 s. Kind 3 samples: never (kind 3 folds to one map entry per net) |
| Q8 | Where 256 MiB (`MAX_TRACE_BYTES`) stops a run | Kind 3, three per quantum: about **45 s** simulated. Combined record: about 65 s, but `MAX_RECORDS` stops it first |
| Q9 | Wall time per simulated second | 1.4 ms per quantum for lock-step, plus Renode's DShot cost (0.10 s per ms of DShot running, `2302`) for the fraction of time `CEN` is set: **150–250 s per simulated second** |
| Q10 | Renode memory | 4.7 MiB of RSS per ms of DShot (`2302`): **the CLI's 1 GiB bound is passed before about 0.25 s of DShot running.** Inferred to be the first limit any run meets, before `MAX_RECORDS` for the kind 3 shape |
| Q11 | Determinism | Two runs of the same case give byte-identical traces (as `2301`'s lock-step did) |
| Q12 | `replay_dir` | Reproduces the state-sequence hash. A byte flipped in a mid-run sample is refused (`HashMismatch`), because every event's delta enters the hash chain (`replay.rs` L26–63, read at source) |
| Q13 | The CLI's `aethelgard replay` | **Refuses** a campaign run directory holding a kind 3 sample: `NotACampaignRun`, "a campaign run holds an event that is not a campaign record" (`campaign.rs` `check_archived`, read at source). The campaign archive has no place for per-quantum state today. Only a new record kind and its run rules would give it one, and that is a crate change |
| Q14 | The verdict from the records | The bridge's actions (each reset and release) can be re-derived from the recorded per-quantum `VDD_3V3` samples alone, with no solver. A spike re-judge reproduces them exactly |
| Q15 | The loop | With R_pack large enough that the motors' current pulls `V_bat` under the regulator's dropout, the run shows a reboot loop: reset, `CEN` cleared, recovery, boot, `CEN` set, sag, reset. A period of a few ms to tens of ms, set by the firmware's time from release to `CEN` |

## Results — measured after the predictions (`975b6f9c`)

Runs added after the predictions were committed had no prediction of their own: `Q_NS`, `FULL_SETUP`, `synth`, the re-sealed flip, and both droop-run splices. They are reported as measured, with nothing to hold them against.

**The evidence** is committed in `spike/2303/out/`:
- per run: `run.json`, `actions.txt` (the probe's side log), `replay.jsonl`, `setup.cmds`;
- the batch logs `m*.log`, and `caps.log`;
- the captured CLI argv `renode-argv.txt`;
- the campaign and splice outputs.

The traces themselves (up to 367 MB) are not kept: every one is reproducible from `spike/2303/sh/` and their sha256 is quoted here.

**How it ran (measured).**
- Every container ran through `spike/2303/sh/img.sh`: `~/bin/capped-run --mem 8G`, `--network none`, TMPDIR unset, one container at a time.
- Each container's caps were read back inside the scope before it started: `memory.max=8589934592 memory.swap.max=0 pids.max=8192` (`spike/2303/out/caps.log`).
- Image: `container/IMAGE`, the pinned tools image (Renode 1.17.0, ngspice 44.2).
- The CLI, the bridge and the probe were built in it from this branch: the CLI and the bridge unmodified, the probe from `spike/2303/lockstep` and `spike/2303/ngspice`.

**The board, the firmware and the setup (measured).**
- `aethelgard generate` and `aethelgard build` on `sentinel-02-shaped.json`, then `aethelgard simulate --for 100ms`: `pass`, 7.9 s wall.
- While it ran, its Renode argv (the whole monitor script) was copied from `/proc` (`spike/2303/sh/gen.sh`).
- The probe replays that script's setup:
  - every command before the first watchpoint hook;
  - the CLI's scratch paths mapped to copies of the same files: the emitted `.repl`, the built ELF, and the two C# models.
- It then runs the quanta itself. Left out, as deviations:
  - the hooks, which are the judge's witness machinery: the TIM1 anchor with its +2 ms stop, the boot word's serial injection, and the TDR echo;
  - `logLevel -1` on devices and observers, `LogPeripheralAccess`, and the capture backends.
- **What the firmware does is unchanged.** The boot word `0x2100B007` and TIM1 `CEN` both appear at 39.08 ms, against `2302`'s 39.072 ms for the anchor (`spike/2302/out/phases-sentinel-02-shaped.json` at `83f20420`). `CR1` reads `0x81` (`ARPE | CEN`) and `CCR1` reads `0x28` (40, the zero bit's 3/8, `4300a`).
- The per-quantum line is `RunFor 10 µs; ReadDoubleWord boot word; ReadDoubleWord TIM1_CR1; ReadDoubleWord TIM1_CCR1; echo`, the same line every quantum.

**The two cases (spike-declared deck, §Predictions).**
- `healthy`: R_pack 0.05 Ω.
- `weak`: R_pack 1.0 Ω, the fault.
- The motors' current is 10 A while `CEN` is set, through a 1 ms lag.

### §1.2 The prototype's run: one board, one fault, 10 µs quanta (measured)

**The loop closes (measured, `weak`, `spike/2303/out/runs/weak-100ms-1/actions.txt`).**

| Time | Who | What |
|---|---|---|
| 39.08 ms | Renode | the boot word written; TIM1 `CR1` = `0x81`; the motors' current starts to rise |
| 40.74 ms | bridge | `VDD_3V3` 2.5902 V < BOR3 (`V_bat` 3.0952 V): reset asserted, as the CLI renders it; `CR1` reads 0, so the load falls |
| 40.80 ms | bridge | `VDD_3V3` 2.7294 V > 2.70 V: released with the nudge |
| 79.88 ms | Renode | booted again; `CEN` set again |
| 81.54 ms | bridge | reset asserted again: a reboot loop, period 40.80 ms |

- Over 1 s, 24 resets, each 1.66 ms after its `CEN`.
- The `healthy` case sags `V_bat` by 0.5 V and never resets.
- Two firmwares on this board would draw different waveforms; this one draws a reboot loop. That is row 8's dependence, measured, with a spike-declared load.

**The archive, per simulated second (measured, the real `TraceWriter`; `spike/2303/out/runs/*/run.json`).**
- Each run writes three traces from the same boundaries:
  - **samples:** `V_bat` and `VDD_3V3` as kind 3, and the firmware's `CR1` as kind 0, at three events a quantum. Kind 0 is a stand-in: it is a register *write* event, used here for a per-quantum *read*;
  - **records:** the same three as stand-in records (no per-quantum record kind exists), each rail as kind 17 (`RailCrossing`, level `quantum`) and `CR1` as kind 9 (`RegisterObserved`), at three a quantum;
  - **events:** only the changes: `CR1` changes and the bridge's resets and releases.

| Run | Simulated | Samples: bytes (B/s) | Records: bytes, written | Events: bytes (count) | Renode RSS, start → end | Wall |
|---|---|---|---|---|---|---|
| `healthy` ×2 | 100 ms | 506,139 (5.06 MB/s) | 992,707, 30,000 | 133 (2) | 426 → 441 MiB | 20.6, 22.7 s |
| `weak` ×2 | 100 ms | 500,380 (5.00 MB/s) | 986,948, 30,000 | 506 (11) | 427 → 442 MiB | 12.7, 13.3 s |
| `healthy` | 1 s | 5,096,139 (5.10 MB/s) | 2,177,242, **65,536 then refused at 218.46 ms** | 133 (2) | 430 → 462 MiB | **235.1 s** |
| `weak` | 1 s | 5,004,054 (5.00 MB/s) | 2,160,139, **65,536 then refused at 218.46 ms** | 4,906 (121) | 426 → 450 MiB | **87.1 s** |
| The CLI's own static campaign run, for scale (`s02-baseline`) | 100 ms | — | 6,662 B, 91 events | — | — | — |

- **Per quantum:** 50–51 B as samples (16.7–17.0 B an event) and 99 B as records (33 B each).
- **Past 4.295 s the samples cost 6.20 MB/s, not 5.0.** A timestamp past 2³² ns takes CBOR's 9-byte integer instead of the 5-byte one.
  - Measured with the real writer on the measured run's events, repeated with their timestamps moved on (`synth`): 5,003,386 B at 1 s, 25,862,794 at 5 s, then +6,203,340 B every second.
- **Two runs of each 100 ms case are byte-identical** in all three traces (sha256: `healthy` samples `25ff88e4…`, `weak` samples `01deef20…`).
- **The wall time is Renode's.**
  - Renode busy 220.1 s and ngspice 14.4 s of `healthy`'s 235.1 s; Renode 73.1 s and ngspice 13.4 s of `weak`'s 87.1 s.
  - The difference is DShot running: 960.9 ms against 40.1 ms of `CEN` set, about **0.16 s of wall per ms of DShot**, inferred from the two (`2302` measured 0.10).
  - Without DShot, lock-step costs about 0.73 ms a quantum (`weak`: Renode 73.1 s over 100,000 quanta, 4 % of it DShot), under `2301`'s 1.4 ms.
- **Renode's memory does not grow with DShot under per-quantum lock-step:** +32 MiB over 961 ms of DShot, about 0.03 MiB per ms, not `2302`'s 4.7 MiB per ms.
  - With the CLI's witness logging kept (`FULL_SETUP`: `LogPeripheralAccess` on USART2, USART3, GPIOA, GPIOB and RCC, the capture backends, devices at `logLevel -1`, observers silenced as `2302` silenced them), healthy at 100 ms: 434 → 448 MiB, flat from 40 to 100 ms while DShot runs. The samples trace is byte-identical to the run without it (`25ff88e4…`).
  - The cause of `2302`'s growth is not established here. Inferred: it may belong to one long `RunFor` (the CLI's) rather than to DShot. That was not measured.
- **Renode's own log** carries two INFO lines a quantum ("Machine resumed", "Machine paused"): 10.3–10.5 MB per simulated second (measured, the file `renode.log`).
  - Inferred: as a file under the sandbox's 64 MiB `RLIMIT_FSIZE` (`2301` P5) that is about 6 s of simulated time. Through the build's stream filter it is counted, not kept.
- **The probe's own memory grows with simulated time, not with quanta** (measured, `VmHWM`). That points at libngspice.
  - Same 10,000 quanta and 30,000 records: 27 MB for 100 ms at 10 µs, against 98 MB for 1 s at 100 µs.
  - Same 100,000 quanta and 65,536 records: 68 MB for 100 ms at 1 µs, against 131 MB for 1 s at 10 µs.
  - So the analog side costs **about 70–79 MB per simulated second** at the deck's 1 µs step cap. The records writer's fold is the rest.
  - Inferred: ngspice keeps every accepted point of the `tran`, so a bridge bounded at 1 GiB passes it at about 13 s. Inferred, not measured: `.save` of only the sampled nodes would shrink it.

**Replay (measured).**

| What | Result |
|---|---|
| `replay_dir`, every trace of every run | `ok`; the state-sequence hash reproduces; the same hash on both runs of a case |
| A consistency check, not a verdict: the bridge's reset rule applied again to the recorded `VDD_3V3` samples alone, with no solver, against the `ResetRendered` records archived in the events trace | equal in every run: 4 of 4 at 100 ms, 48 of 48 at 1 s (`weak`, at 1, 10 and 100 µs); 0 of 0 (`healthy`). **The probe's traces carry no verdict record**, so "does `replay` reproduce the verdict" is answered only by the CLI's `replay` rows below |
| A byte of a mid-run sample flipped (event 15,000 of 30,000: `VBAT` at 50.01 ms) | `HashMismatch` (the trace's sha256) |
| The same, with the manifest re-sealed over the flipped trace | **`ReplayDiverged`**: the state-sequence hash chain covers every per-quantum sample |
| The fold, at `MAX_RECORDS` | records: peak 44.5 MB replaying 65,536 records against 10.6 MB for the 300,000 samples, so about 500 B held a record (inferred from the difference); samples fold to one map entry a net |
| A samples trace of 267,793,054 B (44 s, `synth`) | `ok`, 2.1 s, peak RSS 269 MiB |
| A samples trace of 367,046,494 B (60 s, `synth`) | the **writer wrote it**; `replay_dir` refuses it, `TraceTooLarge` |
| **The CLI's `aethelgard replay`** on a real Sentinel-02-shaped campaign run with a declared droop (`spike/2303/scenarios/s02-droop.json`, 100 events) with one kind 3 sample spliced in and re-sealed | **`NotACampaignRun`**: "a campaign run holds an event that is not a campaign record", exit 3. The same on the static `s02-baseline` run (91 events) |
| The same droop run with 1,000 per-quantum stand-in records (kind 17, `VDD_3V3`, level `quantum`) spliced in **after its own crossings, resets and observations, in the campaign's time order** | **`NotACampaignRun`**: "a crossing names a level no active threshold has", exit 3. **The rule:** a kind 17 record must be a crossing of one of the declared rail's active thresholds (`check_joins`). No record kind takes a per-quantum value. (An earlier splice that broke the time order was refused for order; that was the splice's fault, not the reader's) |
| The same runs, unspliced | `ok`, and each reproduces its own verdict. `s02-baseline`: `pass`, matched. `s02-droop`: **`fail` (`brownout_unrecovered`) against an expected `pass`, unmatched**. Its expectations were copied unchanged from `scenarios/motors/dshot600-droop.json`; the run serves the splice, and why this board does not recover inside that bound was not investigated (outside this question; noted as F11) |

**Fit: where a run stops (measured unless marked).**

| Limit | Samples shape (kinds 3 and 0) | Records shape (three a quantum) |
|---|---|---|
| `MAX_RECORDS` (65,536) | never: samples fold to one entry a net | **0.2185 s** (measured, both cases). One combined record a quantum: 0.655 s (inferred) |
| 256 MiB (`MAX_TRACE_BYTES`) | **44.10 s** (measured with the real writer, on events repeated from the measured run) | never reached first |
| The campaign reader (`check_archived`, `check_joins`) | **0 s**: refused outright (not a campaign record) | **0 s**: refused outright (a crossing must name an active threshold) |
| CI's 10 minutes a tier-2 test | about 2.5 s with DShot running throughout (235 s/s), about 6.9 s for the reboot loop (87 s/s) (inferred from the measured rates) | same |
| The analog side's memory at a 1 GiB bound | about 13 s (inferred from the measured 70–79 MB/s) | same |
| Renode's memory (the CLI's 1 GiB bound) | not met: +32 MiB in 1 s, flat under DShot with or without the witness logging (measured) | same |

## §1.3 It does not fit: the options, without choosing

**What does not fit (measured above):**
- Today's campaign archive holds no per-quantum data at all: `aethelgard replay` refuses both shapes.
- Per-quantum records pass `MAX_RECORDS` at 0.22 s.
- Per-quantum samples, if admitted, pass 256 MiB at 44.1 s, and the writer would not stop them.
- At 10 µs the lock-step costs 87–235 s of wall per simulated second, so a CI tier-2 test holds 2.5–7 s of simulated time.

So a firmware-dependent fault cannot be archived by today's campaign at any length. Every option below changes `aethelgard-chaos`, the CLI's campaign rules, or both, and none is built here.

| Option | What it is | What it costs (measured or inferred) | Sampling under `INV-04`? |
|---|---|---|---|
| **(a1) Larger bounds: admit per-quantum samples** | `check_archived` admits kinds 3 and 0 (or a new per-quantum kind) in a dynamic run, with the trace version bumped | 5.0 MB/s, then 6.2 MB/s past 4.29 s; no fold growth; 256 MiB at 44.1 s. The writer must refuse at the reader's bound, which it does not today (finding F3) | **No.** Every quantum's exchanged values are recorded |
| **(a2) Larger bounds: per-quantum records, `MAX_RECORDS` raised** | Three records a quantum, or one combined | About 500 B held a record. One second at 10 µs is 300,000 records (about 150 MB of fold), or 100,000 combined (about 50 MB). A 10 s run is 0.5–1.5 GB (inferred from the measured fold) | **No** |
| **(b) A coarser quantum, its error stated** | Lock-step at 100 µs, the twin's own `QUANTUM_NS` | See the measurement below: bytes and wall about 10× smaller, a `CEN` change seen up to one quantum late, a crossing up to one quantum late, an excursion shorter than a quantum missed (`2301` F7) | **No** by itself: nothing computed is dropped. It is a coarser model, so `RUL-17`'s "about 10 µs" is the architect's |
| **(c) Record only around events** | Lock-step at 10 µs, keeping only the quanta around a `CEN` change, a crossing or a reset (the `events` trace: 506 B per 100 ms, 4.9 KB per second here) | Under 5 KB/s | **Yes.** Per-quantum state the run computed would be dropped and could not be replayed without the solvers. That is Scotty's (`2301` P4, as written there). The re-judge above shows that the *decisions* replay from the samples. Whether they would from the events alone is the question this option puts to Scotty |
| **(d) Not in the work order's list, named only:** re-plan the schedule at each firmware command | `2301`'s schedule, solved again from each `CR1` write (a watchpoint hook gives its exact time), so no per-quantum state exists | Inferred: event-sized archives, as `2301` D8. But a crossing that falls inside a Renode segment the firmware has already run past cannot be undone: `2302` measured that no Renode call ends `RunFor` early, and Renode has no cheap rollback. Not measured | No per-quantum state, as D8 ruled for the schedule; the architect's |

**Option (b), measured: the `weak` case at 1, 10 and 100 µs quanta** (`spike/2303/out/runs/weak-*`). 1 µs is the reference; ngspice's own crossing is finer still.

| Quantum | First reset asserted | Released | `VDD_3V3` when asserted | Loop period | Resets in 1 s | Samples | Wall per simulated second |
|---|---|---|---|---|---|---|---|
| 1 µs | 40.724 ms | 40.775 ms | 2.6082 V | 40.775 ms | — (100 ms run: 2) | 50.0 MB/s (5,002,996 B per 100 ms) | 918 s (91.8 s per 100 ms) |
| 10 µs | 40.740 ms (+16 µs) | 40.800 ms (+25 µs) | 2.5902 V | 40.800 ms | 24 | 5.00 MB/s | 87.1 s |
| 100 µs | 40.800 ms (+76 µs) | 40.900 ms (+125 µs) | 2.5120 V (98 mV deeper past BOR3 than at 1 µs) | 40.900 ms | 24 | 0.50 MB/s | 20.2 s |

- The error is one quantum at most at each step:
  - the motors see the firmware's `CEN` up to a quantum late;
  - the bridge sees the crossing up to a quantum late;
  - the rail has gone deeper by then.
- The loop's period moves by 0.3 % at 100 µs, and the count of resets in 1 s does not change.
- At 100 µs, the records shape is 30,000 a second, so `MAX_RECORDS` comes at 2.18 s (inferred from the rate).
- The re-judge reproduces every action at every quantum (4 of 4, 48 of 48).

## The spike's table: each prediction beside its result

| id | Predicted | Result | |
|---|---|---|---|
| Q1 | Three per-quantum quantities | Three: `V_bat`, `VDD_3V3`, `CR1` | held |
| Q2 | Samples: 6.0 MB/s ±15 % | **5.00–5.10 MB/s** up to 4.29 s; 6.20 MB/s after it | missed by 1–2 points under the band; the 9-byte timestamp past 2³² ns was not predicted |
| Q3 | Records: 9.3 MB/s ±15 % | 9.87–9.93 MB/s | held |
| Q4 | One combined record: 3.5–4.5 MB/s | not measured: no record kind holds three values, and the stand-ins would measure a made-up layout | open |
| Q5 | Events under 100 KB/s | 4.9 KB/s with 24 reboot cycles a second; 133 B per run without | held |
| Q6 | 300,000 records a simulated second | 300,000 (three a quantum) | held |
| Q7 | `MAX_RECORDS` at 0.22 s | **0.2185 s** (record 65,537 refused), both cases | held |
| Q8 | 256 MiB at about 45 s | **44.10 s** | held |
| Q9 | 150–250 s of wall per simulated second | 235 s with DShot throughout; **87 s** for the reboot loop | held for DShot; missed for the loop (lock-step costs 0.73 ms a quantum here, not 1.4) |
| Q10 | Renode passes 1 GiB before 0.25 s of DShot (4.7 MiB/ms) | **+32 MiB in 961 ms of DShot**; flat with the CLI's witness logging too | missed: `2302`'s figure does not reproduce under lock-step |
| Q11 | Two runs byte-identical | yes, all three traces, both cases | held |
| Q12 | A flipped sample is refused | `HashMismatch`; re-sealed, `ReplayDiverged` | held |
| Q13 | `aethelgard replay` refuses kind 3 in a campaign run | `NotACampaignRun`; per-quantum stand-in records are refused too (order) | held, and wider than predicted |
| Q14 | The actions re-derive from the samples alone | 4/4, 48/48, at every quantum | held |
| Q15 | A reboot loop of a few to tens of ms | 40.80 ms, set by the firmware's 39.08 ms from release to `CEN` | held |

## Findings

- **F1 · The campaign archive has no place for per-quantum data · `aethelgard-chaos` `check_archived`, `check_joins` · high for `4500` · measured.** `aethelgard replay` refuses a real Sentinel-02 droop run in two cases:
  - with one kind 3 sample in it: `NotACampaignRun`, not a campaign record;
  - with per-quantum kind 17 records in the campaign's own time order: `NotACampaignRun`, because a crossing must name an active threshold. A firmware-dependent fault cannot be archived today at any length. Any build needs a format and rules change, which is the trace version.
- **F2 · Per-quantum records pass `MAX_RECORDS` at 0.2185 s · `replay.rs` L74 · high · measured.** Samples (kinds 3 and 0) fold to one entry a net and never meet it.
- **F3 · `TraceWriter` writes past the reader's 256 MiB bound · `archive.rs` `append` (no `MAX_TRACE_BYTES` check; the reader's is L329) · medium for `INV-04` · measured.** It wrote 367,046,494 B, which `replay_dir` then refused (`TraceTooLarge`). A long dynamic run would leave an archive that cannot replay. Static campaigns, at about 1 KB a run, never come near it.
- **F4 · Timestamps past 2³² ns cost 4 B more an event · CBOR's integer widths · low · measured.** 5.0 MB/s becomes 6.2 MB/s past 4.29 s simulated.
- **F5 · `2302`'s DShot window and a firmware-dependent load contradict each other · `twin/src/boot.rs` L46–54, `cli/src/anchor.rs` · medium for a build · read at source.** The judge clears TIM1 `CEN` at the anchor plus 2 ms, so a load that follows the firmware's timer would see the judge's stop and not the firmware's behaviour. The probe left the stop out. A build would have to choose.
- **F6 · Renode logs two INFO lines a quantum ("Machine resumed", "Machine paused") · Renode 1.17.0 · medium · measured: 10.3–10.5 MB of log per simulated second at 10 µs.** Inferred: as a file under the sandbox's 64 MiB `RLIMIT_FSIZE` (`2301` P5) that is about 6 s. Through the build's stream filter it is counted, not kept.
- **F7 · The analog side's memory grows with simulated time · libngspice `tran` vectors · medium · measured about 70–79 MB per simulated second, separated from the fold by runs of equal quanta; the cause is inferred.** ngspice keeps every accepted point. Inferred: about 13 s against a 1 GiB bound, unless only the sampled nodes are saved (not measured).
- **F8 · Lock-step at 10 µs costs 0.73 ms a quantum without DShot, and DShot adds about 0.16 s of wall per ms · measured, inferred from two runs.** 87–235 s per simulated second: a CI tier-2 test holds 2.5–7 s of simulated time.
- **F9 · Nothing in the repository declares an ESC's supply current · `aethelgard-ir`, parts tables · high for row 8 · read at source.** The prototype's 10 A while `CEN` is set is spike-declared. Under `INV-06` a build halts without a declared source, and with today's zero-throttle firmware a real ESC draws only its idle current.
- **F10 · The CLI's DShot observers at `logLevel -1` split the probe's console lines · 2301 F8 again · low · measured: the probe hung at quantum 4,299, 4 ms into DShot.** A per-quantum monitor protocol must not share the console with noisy sources.

- **F11 · A declared droop on the Sentinel-02-shaped board (`dshot600-droop`'s values: VDD_3V3 to 2.4 V at 100 µs) ends `brownout_unrecovered` · the campaign · low for this work order · measured, not investigated.** The same droop passes on `motors-dshot600`. Inferred: this board's boot takes 39 ms behind its sensors' start-up waits, against the droop scenario's recovery expectation. Recorded for whoever writes `4500`'s droop scenario.

## Decisions needed

- **D1 (architect, then Scotty for `4500`).** Row 8, the battery under the motors:
  - (i) firmware-dependent, by per-quantum lock-step, measured here;
  - (ii) declared, as `2301`'s droop: a current profile as a stimulus. It cannot show the reboot loop measured here, because the load would not stop at the reset.

  The P4 exit gate says "a static fault campaign", so (ii), or leaving row 8 to the battery-sag work order, keeps `4500` within its gate.
- **D2 (architect, Scotty for any that sample).** Which option of §1.3, if (i): (a1), (a2), (b), (c) or (d). Only (c) is sampling under `INV-04`.
- **D3 (architect).** F3: the writer refuses at the reader's bound. It belongs to whichever work order first writes a dynamic trace; static campaigns never reach it.
- **D4 (architect).** F9: where an ESC's supply current comes from (a parts row, a specification field, a scenario field). `INV-06`: absent means halt.
- **D5 (architect).** F5: under lock-step, whether the judge's DShot stop moves to the run's end.

## §7 checked

- **"The prototype needs a bridge change that cannot be made under `spike/2303/`":** it did not.
  - The probe owns Renode and libngspice itself, as `2301`'s did, and reads the real `aethelgard-chaos` as a path dependency.
  - Nothing outside `spike/2303/` and this file changed: `git diff --stat main -- crates bridge` is empty.
  - **What a build would change is not the bridge but the archive** (F1, F3): the campaign reader's rules and the trace version, in `aethelgard-chaos` and the CLI's campaign.
- **§2 is the stop:** this is it.

## Gates (spike)

- **No crate changed:** `git diff --stat ecc6be95 -- crates bridge Cargo.toml Cargo.lock` is empty.
- **The probe** builds in the pinned image with no warnings (`cargo build --release --manifest-path spike/2303/lockstep/Cargo.toml`). `spike/2303/lockstep` is `cargo fmt`-clean. `spike/2303/ngspice` is `2301`'s FFI copied unchanged from `a451cd15` and is not `fmt`-clean, as it was not there.
- **Evidence:** in `spike/2303/out/`; the scripts that made it are in `spike/2303/sh/`.

**STOPPED at §2**, 2026-11-17, for the architect's ruling on D1–D5. The PR is a record only, not to be merged as code. Its title and body are in `~/.cache/aet-2303/pr-title-body.md` on Furnace. Scratch trees are removed, and the evidence is in `spike/2303/out/`.
