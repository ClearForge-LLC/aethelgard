# AET-WO-4400h predictions

Written before the run each part predicts. Part 1 is committed before the first `simulate` run. Parts 2–4 are committed before the first injection run.

Each line carries a figure or a yes/no and its source:
- **[S]** is source read at a pin, given as file:line;
- **[R]** is a prior record;
- **[M]** is memory or judgement.

## Inputs, fixed for every run

- **ELF:** `5eace4ca7de47819e8728c403ff9506e72a9f79e2d16ca5a38e819b522c9dfe1`. It is CI's `generated-sentinel-02` artifact of `ardupilot-image` run 45410032458 on main `29b3a97d`, Copter-4.7.2 `fb773567`. Byte-identical with `AET-WO-2314`'s copy.
- **Specification:** `crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json` at `29b3a97d`.
- **CLI:** main `29b3a97d` plus `patches/window.patch`, built in the pinned tools image (`container/IMAGE`, `1e0ab105…`). The patch changes three things:
  - `FOREIGN_MAX_VIRTUAL_NS` goes from 5 s to 30 s;
  - the foreign run's wall and RSS bounds come from `AET4400H_WALL_S` and `AET4400H_RSS_KIB`, defaulting to main's `BOOT_BOUNDS` (30 s, 1,572,864 KiB);
  - `AET4400H_NO_USART_LOG` drops `2311`'s `LogPeripheralAccess` lines.

  Unset, the patched CLI is main's, except for the window's ceiling.
- **The board has no motor output.** `motor_outputs` is `[]`, and the hwdef has no PWM line. This is asked on the bus (from 4400h, 2026-11-24).

## Part 1: the window's cost

- **P1.1 The bound that binds first, with main's bounds unlifted, is the 30 s wall bound.** It is not the RSS cap.
  - At 5 s virtual: wall 11–16 s.
  - At 10 s: wall 24–34 s, so under today's load (12–16 on 20 cores) a 10 s run is killed by the wall bound in at least one of two runs.
  - At 20 s and 30 s: killed by the wall bound every time.
  - [R] `4400e`: 10.6–12.0 s of wall for 4 s virtual. `4400f` F1: 29.7 s for 4 s at load 26.
- **P1.2 Wall time grows linearly with the window:** 2.4–3.2 s of wall per virtual second at load 12–16 [R, M].
- **P1.3 Peak tree RSS grows with wall time, not with virtual time alone** [R: `4400f` F1]. With the wall bound lifted (`AET4400H_WALL_S=600`) and the log on:

  | window | peak (MiB) | share of 1,572,864 KiB |
  |---|---|---|
  | 5 s | 800–1,000 | 520–650‰ |
  | 10 s | 950–1,200 | 620–780‰ |
  | 20 s | 1,200–1,500 | 780–980‰ |
  | 30 s | 1,400–1,800 | 900‰ or more, possibly killed at the cap |

- **P1.4 Without `2311`'s access log, the peak is lower by under 10 %** at every window [M]. In a normal run, USART1's logged accesses are the console's DMA-fed TX and the driver's register traffic, not a polling loop. `1609`'s 2.59 against 1.10 GB was CrashCatcher polling USART1, which does not happen here.
- **P1.5 The no-log run's verdict still carries `memory`, but it fails the serial-options judge** (`serial_options_unwritten` or `unjudgeable`): the log is that judge's input [S: `foreign.rs` `judge_serial`].
- **P1.6 Log volume.** The kept log grows with the window and stays under the 8 MiB budget at 30 s, so no line is dropped [M; held loosely]. `4400e` kept 1.21 MB at 4 s, before `2311`'s log.
- **P1.7 The §7 stop "the cap fails before 10 s" does not fire:** no 5 s or 10 s run passes 1,572,864 KiB.
- **P1.8 The proposal.** The longest window whose worst peak stays at or under 900‰ is **10 s** (20 s is marginal), and it needs main's 30 s wall bound raised with it. Keeping the 30 s wall bound means keeping 5 s.
- **P1.9 A CPU-restricted run passes a higher share of the cap than an unrestricted one at the same window,** because RSS follows wall time. Restricting to 2 cores with `taskset` stands in for a loaded host.

## Parts 2–4: arming, what it reaches, a commanded step

Written after part 1's first runs, before any injection run. The source lines were read by subagents and spot-checked:
- Renode: `STM32F7_USART.cs`:87, :109, :154, :231 and `STM32DMA.cs`:237–245, in renode-infrastructure `cd4b002a`, the tools image's pin.
- ArduPilot: `UARTDriver.h`:25, `UARTDriver.cpp`:549–566 and :606–622, `AP_SerialManager.cpp`:172–180, `AP_Arming.cpp`:201 and :1885, `AP_InertialSensor.cpp`:1639–1660, `GCS_Common.cpp`:4966, and `ArduCopter/Parameters.cpp`:26–30, all at `fb773567`.

### How the twin injects into a foreign firmware's USART today

- **P2.1 It does not.** Bytes written into `usart1` from the monitor are queued and raise RXNE, but ArduPilot never reads them [R: TVI-WO-1302 §2.6].
  - ArduPilot clears `RXNEIE` and receives by DMA1 stream 0 (`CR3.DMAR`).
  - The emitted platform does not connect Renode's USART `ReceiveDmaRequest` to `dmamux1`. Request 41 never rises, and NDTR never moves [S].
  - **The twin lacks one platform line:** `ReceiveDmaRequest -> dmamux1@41` in `usart1`'s block, which the twin already writes for SPI (`platform.rs`:789–790).
- **P2.2 With that line, a byte moves into the bounce buffer, but ArduPilot reads it only when 64 bytes have arrived.** NDTR reaching 0 raises transfer-complete, since TCIE is set [S].
  - On silicon, a short frame is handed over by the USART's IDLE interrupt. The model has no IDLE (a tag) [S].
  - So each injected frame is padded with zero bytes to exactly 64. The second twin gap, IDLE, is named and not modelled.
  - **No parameter can route around this.** SERIAL0 has no `OPTIONS` parameter (no `NoDMA_RX`) [S]. Changing the ELF would be a §7 stop.
- **P2.3 This is neither a defaults file nor a rebuilt ELF,** so §7's third stop does not fire. Parameters arrive at run time, over MAVLink.

### Arming

- **P2.4 Before any parameter is set, `MAV_CMD_RUN_PREARM_CHECKS` and an arm attempt report at least these, classed** [S, M]:

  | message | class |
  |---|---|
  | `Motors: Check frame class and type` | a parameter: `FRAME_CLASS` 0 |
  | `3D Accel calibration needed` | a parameter: no saved calibration on a new board; a real board needs it too |
  | `RC not found` or `Throttle below failsafe` | a parameter, not a twin gap: an agent-flown vehicle has no RC, and `FS_THR_ENABLE` must say so |

  Held loosely:

  | message | class |
  |---|---|
  | `Main loop slow` | a twin cost |
  | `Logging failed` | a twin model gap, if the W25Q model misanswers |
  | `Leaning` | a twin model gap, if the IMU model's gravity is off-axis after `ROLL_180_YAW_90` |
  | `AHRS: …` / `EKF3 not started` | a twin model gap |

  No real fault is predicted.
- **P2.5 The smallest set that arms without skipping or forcing:**
  - `FRAME_CLASS` = 1;
  - `FS_THR_ENABLE` = 0;
  - `DISARM_DELAY` = 0, so it stays armed (a choice, not a check);
  - `INS_ACCOFFS_X` = 0.001, then `MAV_CMD_PREFLIGHT_CALIBRATION` with param5 = 76, which accepts the accel calibration as valid.

  Then arm in STABILIZE, the boot mode. I predict this arms with no `ARMING_SKIPCHK` bit and no force, held moderately. A skip bit is used only if a twin-gap check remains, and it is named. The force code (21196) is used only if a check no bit skips remains. A forced arm still runs the mandatory checks.
- **P2.6 Persistence.**
  - A `PARAM_SET` is saved: it is echoed as `PARAM_VALUE`, and within the same run its value is written to flash storage page 14 (`0x081C0000`, bank 2 sector 6).
  - The write is 32-byte flash words under `CR2.PG`, read back with memcmp.
  - Renode's `STM32H7_FlashController` models bank 2, keys, PG, 32-byte words and sector erase. So the write succeeds and no `Storage: failed` is printed.
  - The boot itself already erases page 14 and page 15 and writes their headers before any `PARAM_SET` (the twin's flash starts blank). `2314`'s erase hook names those erases in the verdict.
  - Across a twin restart nothing persists, because each run starts from the ELF alone. Persistence on silicon is bank 2 sectors 6–7: an input to `TVI-WO-1304`.
- **P2.7 The arm is an arm.** HEARTBEAT `base_mode` gets `MAV_MODE_FLAG_SAFETY_ARMED`, and `SERVO_OUTPUT_RAW` servo1–4 move from 1000 to about 1100 (`MOT_SPIN_ARM`). But no register changes: the board has no PWM output, and ArduPilot writes to `Empty::RCOutput`. This is the §5 "armed while its outputs stay disabled" case, by construction of the board, not of the twin.

### Part 3: what arming reaches (the access log, armed against unarmed)

- **P3.1 Armed against the injection control, no TIM1 or TIM2 row is reached,** and no DMA stream newly starts, because the board has no motor output. The control is the same frames as zero padding.
- **P3.2 Arming starts no peripheral.** If logging starts at arm (`LOG_DISARMED` = 0), SPI1 (the W25Q dataflash) traffic rises after arming: more writes to `spi1` and DMA2's SPI1 streams. That is the one change, held loosely.
- **P3.3 With `MOT_PWM_TYPE` = 6 (DShot600), no timer DMA moves.** There is no output, `Empty::RCOutput` ignores the mode, and the stock timer has no DMA request [R: TVI-WO-1303 P2].
- **P3.4 The injection's own traffic is DMA1 stream 0's re-arm.** Each 64-byte frame is followed by `S0CR` disable/enable, `S0NDTR`, `S0M0AR` and a `LIFCR` clear, plus ArduPilot's USART1 TX of the replies. The zero-padding control produces the DMA rows but not the reply TX, so the replies are the arming run's own rows, which the queue file marks.

### Part 4: a GUIDED command

- **P4.1 Armed in STABILIZE, `MAV_CMD_DO_SET_MODE` GUIDED is refused** with STATUSTEXT `Mode change failed: GUIDED requires position` (or the ACK `FAILED`): there is no position estimate, since no GPS model exists.
  - `MAV_CMD_NAV_TAKEOFF` is then refused, because the mode is not GUIDED.
  - No output changes; `SERVO_OUTPUT_RAW` stays at ground idle.
  - Class: a twin model gap (no GPS). A real flight would have one.
- **P4.2 In GUIDED_NOGPS** (an extra row, not the WO's command), `SET_ATTITUDE_TARGET` with thrust 0.6 changes `SERVO_OUTPUT_RAW` above ground idle. The baro and IMU do not respond (no dynamics), so the outputs stay where the controller puts them, and the EKF sees no climb.

## Part 5: the motored board (ruled 2026-11-24T15:50Z), before any Sentinel-03 run

The board:
- `crates/aethelgard-cli/tests/fixtures/sentinel-03.json` at main `ef3b43d9`, with `mcu.foreign_clock` copied from `sentinel-02-own`, since `--foreign` needs a declared clock (`py/derive.py`).
- Four DShot600 motors on TIM1 CH1–CH4: PA8, PE11, PE13, PE14.
- An ICM-42688-P and a BMP581; the console on USART1.
- Unmodified Copter-4.7.2, built by CI's own `container/ardupilot-build.sh boards` in the pinned ArduPilot image.

- **P5.1 The hwdef emits and the ELF builds.** `AET-WO-4410` removed `SenseNotEmitted`.
- **P5.2 Unarmed, TIM1 is reached in the window.**
  - ArduPilot sets DShot up at init and sends zero-throttle frames while disarmed, so `PSC`, `ARR`, `CCMRx`, `CCER`, `DIER.UDE`, `DCR` and `BDTR.MOE` are written in the first seconds.
  - So most of Tviburi's 120 "not reached" TIM rows are reached by a motored board without arming. "Not reached in 5 s" was the board, not the arm. Held moderately.
  - TIM2 stays unreached (no output on it).
- **P5.3 No timer DMA moves, armed or not.**
  - The twin's timer raises no DMA request [R: TVI-WO-1303 P2]. The DShot stream (TIM1_UP through DMAMUX1) is configured and enabled, but NDTR never drops, and no DShot bit is ever clocked.
  - ArduPilot carries on with its DShot DMA timeout, held loosely. A STATUSTEXT may name it.
- **P5.4 Arming adds no register.**
  - Armed against the same frames without the arm, no TIM1 or DMA register is newly written. DShot's register traffic is the same armed or disarmed; only the frame values in memory change.
  - `SERVO_OUTPUT_RAW` rises to ground idle.
  - **MOT_PWM_TYPE is already DShot on this board**, from its hwdef's default, so the "set DShot" question is answered by the board: it moves no timer DMA.
- **P5.5 Arming fails more checks than on Sentinel-02.**
  - The same four parameters, plus the battery monitor: with `VBAT_SENSE` the hwdef enables `BATT_MONITOR`, the ADC model reads 0 V, and a battery check fails.
  - `BATT_MONITOR` = 0 passes it as a parameter, a disabled check. Class: twin model gap (no analog front end).
  - Held loosely.
