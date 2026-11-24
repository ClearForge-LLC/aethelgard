# AET-WO-4400h results

**STOP: §7, "the cap fails before 10 s"** (ruled fired, 2026-11-24T15:50Z). What was measured at 10 s:
- all six logged runs passed `4400g` §1.4's line (a run over 900‰ of the cap stops it); the one no-log run read 851‰;
- two of the logged runs passed the cap, both with the cap raised to read the share;
- the three runs at the real cap, at load 3, were not killed by it (904–952‰);
- every 10 s run lost every witness to the CLI's 8 MiB log budget, which is exhausted at 9 s whatever the load.

**Proposed bound: keep 5 s.**

Parts (2)–(4) were then run as ruled, in a replay outside the CLI's bounds (§2–§4):
- **ArduPilot arms over MAVLink with no defaults file, no skipped check and no forced arm.** It needs one twin change (the USART1 receive request line), five parameters and two stimuli.
- **On Sentinel-03, unarmed, every TIM1 row is reached;** arming changes only the compare values.
- **GUIDED is refused for lack of a position estimate.**
- **Rebooting a foreign firmware is not possible in the twin today.**
- **DShot set over MAVLink takes effect at run time, and puts TIM1 into timer-DMA mode, but no DShot bit leaves the twin, which runs out of memory 1.65 s later.**

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

## 2. Arming with no defaults file (replay, outside the CLI's bounds)

Ruled 2026-11-24T15:50Z: (2)–(4) run in a Renode replay of the CLI's own invocation, each container under `capped-run --mem 4G`. Every run's argv, injected bytes (`injected.tsv`), ArduPilot's decoded replies (`mav.tsv`) and end are in `out/inj/<run>/`.

### 2.1 The port, and how the twin injects into a foreign firmware's USART

- **The port: SERIAL0 = USART1,** the console, MAVLink2 at 115200. On `sentinel-02-own` it is the only serial port (`SERIAL_ORDER USART1`). Sentinel-03's console is also USART1.
- **Today the twin cannot deliver a byte.** `arm-noreq` ran the same plan with no request line: ArduPilot sent 20 heartbeats and no `COMMAND_ACK`, because no injected byte was read.
- **What it lacks, and the change used:**
  1. **The request line.** `ReceiveDmaRequest -> dmamux1@41` in `usart1`'s block (`patches/usart1-rx-request.diff`). It is one line from `platform.rs`, as SPI's `DMARecieve` already is (`STM32F7_USART.cs`:87, :109).
  2. **IDLE.** The USART model has none (a tag, `STM32F7_USART.cs`:154, :231). ArduPilot reads a short frame on IDLE (`UARTDriver.cpp`:576–590), and otherwise only when its 64-byte bounce buffer fills (:606–622). So each frame is padded with zero bytes to 64 (`py/frames.py`), injected from the monitor with `WriteChar`.

  With both, every command was answered (`arm1` on).
- **No parameter routes around it:** SERIAL0 has no `OPTIONS` (`AP_SerialManager.cpp`:172–180). Neither change is a defaults file or a rebuilt ELF.

### 2.2 The failed checks, each by its own message and classed

Found in order, each run fixing the last finding.

| run | message (ArduPilot's own) | class | resolution (in every later run) |
|---|---|---|---|
| `arm1` | `PreArm: Motors: Check frame class and type` / `Arm: Motors: …` | **a parameter:** `FRAME_CLASS` defaults to 0 | `PARAM_SET FRAME_CLASS 1`: **passes** |
| `arm1` | `PreArm: Compass 1 not healthy` | **a parameter:** the board has no compass, and `COMPASS_USE` defaults to 1, so `use_for_yaw(0)` is true and `healthy()` false (`AP_Arming.cpp`:625–631) | `PARAM_SET COMPASS_USE 0`: **passes** |
| `arm1` | `PreArm: Baro: not healthy` | **a twin model gap: a sensor reads constant.** ArduPilot calls a barometer unhealthy whose reading has not changed for its data-change timeout (`AP_Baro_Backend.cpp`:36–41). The BMP581 model holds the `Pressure` it is given | a stimulus, the model's own `Pressure` property, alternating by 0.5 Pa every 0.5 s: **passes** |
| `arm2`, `arm3` | `Arm: Leaning` | **a twin model gap: the IMU's stimulus.** The CLI sets the ICM-42688-P's witness stimulus from the parts table (`parts.json`:528: 1 g on the sensor's y; z is the model's default, 0). For ArduPilot that is a board on its edge | a stimulus at t = 0: X 0, Y 0, Z +1 g, a board at rest with the sensor's z up (U5's declared orientation): **passes** |

- **The fewest parameters, measured** (`s03-min`). Only the stimuli, `COMPASS_USE` and `FRAME_CLASS` were set. The failures were:

  | message | class | resolution |
  |---|---|---|
  | `PreArm: RC not found` | **a parameter:** an agent-flown vehicle has no RC, and `FS_THR_ENABLE` defaults to 1 | `FS_THR_ENABLE` 0 |
  | `PreArm: 3D Accel calibration needed` | **a parameter:** a new board has no saved calibration, as on silicon | `INS_ACCOFFS_X` 0.001, then `MAV_CMD_PREFLIGHT_CALIBRATION` param5 = 76; `INS_ACC_ID` reads 3408146 |
  | `PreArm: EKF attitude is bad` | **timing:** at 8.5 s, before EKF3's tilt alignment, and gone at the 9 s arm | none |

  So the fewest are four parameters: `COMPASS_USE` 0, `FRAME_CLASS` 1, `FS_THR_ENABLE` 0 and `INS_ACCOFFS_X` 0.001 with the accept, plus the two stimuli. `DISARM_DELAY` 0 is a choice, to stay armed past 10 s, not a check. Copter returns as soon as the motor check fails (`AP_Arming_Copter.cpp`:51–69), which is why `arm1` showed one failure, then two.
- **No real fault. No check disabled. No forced arm.** `ARMING_SKIPCHK` stayed 0, and the force code was never sent.
- **The arm** (`arm4`, `arm-s1`, `s02-arm-s`, `s03-arm1`, `s03-arm-s`): ACK 0, HEARTBEAT `base_mode` 209 (`MAV_MODE_FLAG_SAFETY_ARMED`).
- **An arm with outputs at minimum (§5's case):**
  - In STABILIZE, with no RC and throttle zero, the outputs stay at 1000 µs. `SERVO_OUTPUT_RAW` reads 1000, and on Sentinel-03 TIM1 `CCR1`–`CCR4` hold 0x3E8.
  - On `sentinel-02-own` they reach no pin at all: the board has no output, and ArduPilot writes to `Empty::RCOutput`.
  - **The arm is real; its outputs move only under a command** (§4).

### 2.3 Persistence, and what the stock flash controller does (bank 2, for `TVI-WO-1304`)

From `s03-dshot2`'s flash-controller access log (`out/inj/s03-dshot2/flash-writes.tsv`):
- **Within a run, a `PARAM_SET` persists.** It is echoed as `PARAM_VALUE`, and read back unchanged later in the run (`FRAME_CLASS` 1.0 at 15 s, `arm1`).
- **Bank 2 is programmed while the board runs.**
  - At boot, ArduPilot unlocks bank 2 (`KEYR2`) and erases sectors 6 and 7 (`CR2` 0x6B4 and 0x7B4: `SER`, `START`, `SNB` 6/7). These are `AP_FlashStorage`'s pages 14 and 15 at `0x081C0000` and `0x081E0000`. It then runs 50 program cycles.
  - The six `PARAM_SET`s (7.45–7.69 s) add **17 program cycles**. Each is `KEYR1`/`KEYR2` unlock, `CR2` 0x734, then 0x736 (with `PG`), then 0x735 (`LOCK`), then two `CCR2` clears.
    - The words are given as written. `SER` and `SNB` 7 stay set from the boot's last erase (0x7B4), so `SNB` does not say which sector is programmed.
    - The programmed addresses are memory writes, which the peripheral access log does not carry. Which sector holds a parameter was not measured.
  - No `Storage: failed` appears in ArduPilot's console text (`out/inj/arm4/tx-strings.txt`). That text travels on the MAVLink port and is what the decoder counts as `BAD_DATA`: `Init ArduCopter V4.7.2`, `Initialised Storage type=2`, `Storage: Using flash pages 14 and 15`, `No Compass backends available`. So ArduPilot's read-back compare passed.
  - The stock `STM32H7_FlashController` takes all of it: keys, `PG`, the 32-byte flash word, sector erase.
- **Across a reboot: not measurable in the twin today.** Two twin gaps:
  - **The platform registers no `reset` macro.** `MAV_CMD_PREFLIGHT_REBOOT_SHUTDOWN` reached `SYSRESETREQ`, and Renode logged `No action for reset - macro machine_0.reset is not registered`. ArduPilot then waited forever (`s03-dshot`).
  - **With a reset macro of the CLI's own boot lines, Renode throws a managed `System.OutOfMemoryException` 2.1 s after the reboot**, at 10.11, 10.18 and 10.11 s virtual. This was under the ruled 4 GiB cap, at a sampled tree peak of 3.45 GB; the cause was not found. This happened with and without `LoadELF` in the macro, and with a narrow or wide access log (`s03-dshot2`, `-3`, `-4`). So whether a value survives a reboot, and what ArduPilot does with `MOT_PWM_TYPE` after one, is not known.

## 3. What arming reaches (replay, outside the CLI's bounds)

**The queue files are in `out/queue/`, with `README.md` naming each file's ELF, board and window.** Four logged runs per board, 13.000 s each (`sh/logged.sh`): no injection, zero frames, the parameters without the arm, and the arm. The deltas are register-level, from `py/delta.py`.

- **`sentinel-02-own` (no outputs):**
  - No queue row changes class between the four runs.
  - Tier 3 is 29 rows class (a) and **120 rows (c): the TIM1/TIM2 rows stay unreached when armed,** as in Tviburi (v).
  - What arming changes, by value, never by a new register: USART1's transmissions and DMA1 stream 4's lengths for them, TIM5's compare (the scheduler), and the RTC backup registers.
  - The RTC backup registers also change between zero frames and none (`s02-injection.tsv`), so that change is not shown to be the arm's.
- **Sentinel-03 (TIM1 CH1–4 motors):**
  - **Unarmed, every TIM1 row is reached:** 6 class (a) and 61 class (b). **TIM2's 54 rows stay (c);** no output uses it. So (v)'s 120 "not reached in 5 s" was the board having no motor, not the vehicle being disarmed.
  - **Setting the parameters adds one row,** `RCC.APB2LPENR.TIM1LPEN`: motor init writes TIM1's sleep-clock enable, a tag the stock RCC drops. It also moves TIM1 from the boot's 50 Hz (`ARR` 0x4E1F) to 400 Hz (`ARR` 0x9C3, `PSC` 0xC7) and sets `CCR` to 1000 µs.
  - **Arming adds no register.** Under GUIDED_NOGPS thrust, TIM1 `CCR1`–`CCR4` carry 622 distinct values. USART1/USART3's transmissions and the RTC backup registers also change value; the RTC change is not shown to be the arm's, as above.
- **Timer DMA, by default: none.** Sentinel-03's spec declares `dshot600`, but its hwdef emits no `MOT_PWM_TYPE` default, so **unmodified ArduPilot runs 400 Hz PWM on it.** `DIER` stays 0 on every arm run, and DMAMUX1 never routes TIM1's update request (15).
- **`MOT_PWM_TYPE` 6 (DShot600) over MAVLink, no reboot** (`s03-dshot-pre`; `out/inj/s03-dshot-pre/tim1-dmamux-writes.tsv`):
  - The setting is read at motor init (`AP_Motors_Class.cpp`:119–154, called from `AP_MotorsMatrix.cpp`:126), which a `FRAME_CLASS` change re-runs. So it is sent before `FRAME_CLASS`.
  - **It moves TIM1 to timer DMA:** `DIER` 0x100 (`UDE`) at 8.18 s, `PSC` 0x29 and `ARR` 0x7 (DShot600's bit timing), and `DCR` 0x30D (a 4-word burst from `CCR1`) rewritten 1,986 times.
  - DMAMUX1 channel 9 is set to request 15 (TIM1_UP) 1,987 times: one re-arm per frame.
  - ArduPilot armed at 9.0 s.
  - **No DShot bit leaves the twin.** The stock timer raises no DMA request (TVI-WO-1303 P2), and the CCR words never move by DMA. Inferred: the stream's registers were not logged in this run.
  - **The twin then throws a managed `OutOfMemoryException` at 9.83 s, 1.65 s into DShot,** under the 4 GiB cap. A DShot board cannot run long in the twin today.
- **The injection's own traffic** (`s0N-injection.tsv`, zero frames against none):
  - Each 64-byte frame adds DMA1 stream 0's re-arm: 3 `S0CR`, 1 `S0NDTR`, 1 `S0M0AR`, 1 `LIFCR`, and 2 D-cache invalidates of the bounce buffer.
  - All of these registers are also written at boot, so the injection adds no queue row, only counts.
  - The monitor's `WriteChar` and stimulus commands are not bus writes: 0 writes without a CPU PC in every run.

## 4. A commanded step

| command | result |
|---|---|
| `MAV_CMD_DO_SET_MODE` GUIDED, armed | refused: ACK 4, STATUSTEXT `Mode change to Guided failed: requires position` |
| `MAV_CMD_NAV_TAKEOFF` 2 m | refused: ACK 4 |
| GUIDED_NOGPS, then `SET_ATTITUDE_TARGET` thrust 0.6, an extra row | **the motor outputs change** |

The GUIDED refusals come from the twin's missing position estimate: there is no GPS model.

The GUIDED_NOGPS outputs, in order:
- `sentinel-02-own`: `SERVO_OUTPUT_RAW` 1073, 1144, 1339, 1797, 1940 µs over 2.5 s, with `system_status` ACTIVE.
- Sentinel-03: TIM1 `CCR1`–`CCR4` from 0x3E8 through 622 values (`s03-arm1`: 1100 to 1935 µs).

**What the twin cannot say: anything about flight.**
- There is no vehicle dynamics, so the IMU and barometer do not respond to thrust.
- The climb controller sees no climb and winds the outputs toward their maximum. The ramp is the controller acting on a vehicle that does not move, not a takeoff.

**The bridge to ArduPilot's SITL, as a row:**
- SITL's physics would drive the twin's IMU and barometer models (their `Acceleration*`, `AngularRate*` and `Pressure` stimuli) at a declared clock relation.
- The twin's TIM1 compare values (or DShot frames) would feed back as SITL's servo inputs.
- That is a lock-step co-simulation across two time bases. Nothing is built.

## 5. The stop: decisions for the architect

- **D1. §7's cap stop: fired (ruled 15:50Z).** 5 s stays.
- **D2. The build work order I would write first: "the foreign window's cost"** (a build with one measured stop).
  - (a) The foreign log filter keeps the first and last line of each per-transaction witness kind and a count, as it does for `Unhandled write`. The kinds are the device and mux NOISY lines, the `AETHELGARD-CFG2` hook, and spi2's slave-mode ERROR. First and last, because a liveness judge (`foreign_device_silent`, not built) would read late lines.
  - (b) Find what holds the growth: about 100–120 MiB per virtual second from the IMU's start, at least part of it live managed memory. Use a heap dump of Renode.
  - (c) Raise the wall bound with the window.
  - (d) Measure again under load. The architect rules the bound.
  - **Fence:** `renode/foreign.rs` (after `2314`), `renode.rs`'s filter, `models/renode/` (if a model holds the growth), tests, and `ci/foreign-judge.sh`'s window. Not the IR, not the cap.
- **D3. The second build: "a foreign firmware hears MAVLink."**
  - Each declared USART's `ReceiveDmaRequest` line, from the topology (`aethelgard-twin` `platform.rs`).
  - IDLE in a `STM32F7_USART` overlay model of our own, so frames need no padding.
  - A `--foreign` input of timed frames and stimuli, recorded in the trace (`INV-04`), so an armed run replays.
  - The rest stimuli an armed run needs, declared, not defaulted (`INV-06`): gravity on the IMU, and a barometer that moves.
  - **Fence:** `platform.rs` and goldens, `models/renode/`, the CLI's input surface, tests. Not the IR, except a stimulus declaration if ruled.
  - It waits on D2: an armed run needs at least 11 s.
- **D4. Rows from (2)–(4):**
  - (a) The twin's reboot: no `reset` macro, and with one Renode runs out of memory 2.1 s into the second boot. This blocks persistence across a reboot.
  - (a2) **DShot drives the twin out of memory** 1.65 s after it starts (`s03-dshot-pre`). Any DShot board, and any long armed run on one, waits on it. It may share a cause with (a) and with §1's growth.
  - (b) **Sentinel-03's hwdef emits no `MOT_PWM_TYPE` for its `dshot600` outputs,** so ArduPilot runs PWM on a board whose spec says DShot. This is a hwdef or pincheck row, and the same as TVI-WO-1302 §7.
  - (c) The parts-table IMU witness stimulus (1 g on y) reads as "Leaning" to an armed ArduPilot. That is fine for the witness, but an armed run needs a declared rest stimulus (D3).
  - (d) No GPS model, so no GUIDED. The SITL bridge row (§4).
  - (e) **The end-of-window read samples a firmware transient** (ruled a row, 15:50Z). At exactly 6.000 s, GPIOB PB13 reads as an output with AF0, so `register_mismatch` fires with and without the log. 5, 7 and 8 s read clean. The verdict depends on where the window lands (§1.4).
- **D5. The motored board: Sentinel-03 used** (ruled), derived as `out/queue/README.md` says. Its fixture on main lacks U5's orientation and `board.forward` (`ImuOrientationUndeclared`, then `ForwardUndeclared`). `4300k`'s unmerged `examples/sentinel-03.json` has both.

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
| P2.1 | the twin cannot inject today; one platform line is missing | **right** (`arm-noreq`: no reply; `arm1`: every command answered with the line) |
| P2.2 | with the line, a frame is read only at 64 bytes; IDLE is missing | **right as built:** padded frames were read. Unpadded frames were not tried |
| P2.3 | neither a defaults file nor a rebuilt ELF | **right** |
| P2.4 | Motors, Accel calibration and RC fail first | **right for those three,** all measured (`arm1`, `s03-min`). Not predicted: Compass (the reader's "passes by default" was wrong). Baro (a constant reading) and Leaning (the stimulus) were twin gaps, as held loosely. EKF attitude was a timing condition |
| P2.5 | four parameters and the accept arm it with no skip or force | **right in kind, one parameter off:** `COMPASS_USE` 0 was needed and `DISARM_DELAY` was not (`s03-min`), plus two stimuli. No skip, no force |
| P2.6 | saved; bank 2 programmed; boot erases; no `Storage: failed`; nothing survives a twin restart | **right within a run.** The reboot test was not possible (§2.3) |
| P2.7 | armed, outputs about 1100 µs, no register on Sentinel-02 | **partly:** armed (`base_mode` 209), no register; but outputs stay at 1000 µs in STABILIZE, not 1100 |
| P3.1–P3.2 | Sentinel-02: no TIM row, no new peripheral; SPI1 rises | **right:** no class change. SPI1 rises about 1 % (a count, not a row) |
| P3.3 | DShot moves no timer DMA | **half right:** DShot puts TIM1 in timer-DMA mode (`UDE`, `DCR`, DMAMUX request 15, 1,986 re-arms), and no request moves data. Not predicted: the twin runs out of memory 1.65 s into DShot |
| P3.4 | the injection's traffic is DMA1 stream 0's re-arm | **right,** plus 2 D-cache invalidates per frame; no row added |
| P4.1 | GUIDED refused for position; TAKEOFF refused | **right** (exact message measured) |
| P4.2 | GUIDED_NOGPS thrust moves the outputs; no dynamics | **right:** 1073 to 1940 µs |
| P5.1 | Sentinel-03's hwdef emits | **wrong for main's fixture** (`ImuOrientationUndeclared`, then `ForwardUndeclared`); right with two declarations copied |
| P5.2 | unarmed, TIM1 is reached; TIM2 is not | **right:** 67 TIM1 rows reached, 54 TIM2 rows (c) |
| P5.3 | no timer DMA | **right** (`DIER` 0; no request 15) |
| P5.4 | arming adds no register; DShot is already the board's default | **first half right.** Second half **wrong:** no `MOT_PWM_TYPE` default is emitted, so ArduPilot runs PWM. DShot set over MAVLink takes effect at the next motor init, without a reboot |
| P5.5 | a battery check fails | **wrong:** Sentinel-03 armed with the same parameters |
