# AET-WO-4412 adversarial pass (in progress: the CI ELF item waits on #395)

## A reboot that isn't one (WO §5; ruling D5: "list what Renode's `machine Reset` clears that silicon keeps after a software reset, and the reverse, each one tested or named")

Each row was measured on Sentinel-03's ArduPilot under the reset macro (`py/resetmacro.py`, replays `reboot-m`, `reboot-v`, `reboot-h`, `reboot-st` and `reboot-hook`), or read at source.

| state | silicon, system reset by `SYSRESETREQ` (RM0433 Rev 9) | the twin | how established |
|---|---|---|---|
| Flash, banks 1 and 2 (the ELF; `AP_FlashStorage` pages 14 and 15) | kept | **kept** | probe: `0x081C0000`, `0x081E0000` and `0x08000000` unchanged across the reset. Parameters read back after it (`FRAME_CLASS` 1, `COMPASS_USE` 0) |
| SRAM and the TCMs | content kept (a system reset does not clear RAM) | **kept** (DTCM `0x20000000` nonzero after) | probe. AXI SRAM's first word read 0, and it was not read before the reset: named, not shown |
| Backup domain (RTC registers, `RTC_BKPxR`) | kept (§8.4.6: only `BDRST` or a `VSW` loss resets it) | **kept** | probe: `RTC_BKP0R` and `RTC_BKP2R` nonzero across the reset |
| Peripherals (RCC, GPIO, timers, USARTs, DMA, SPI, I2C) | reset to their reset values | reset to each Renode model's reset value | measured: RCC back on the HSI before the firmware runs again. Where a model's reset value differs from silicon's, that is the same gap as at power-on, not new |
| `RCC_RSR` | `SFTRSTF`, `PINRSTF`, `CPURSTF` set (Table 56 row 4, p. 333), with earlier flags kept until `RMVF` | **row 4's flags alone**, read through the macro's hooks until `RMVF` clears them | measured: ArduPilot reads `0x01420000` at 6.709 s, then writes `RMVF`, and the self-test reads 0 after it. **Named:** flags from before the reset are not carried (the twin models no `RMVF` before a reset), which equals silicon only for a firmware that clears at boot, as ArduPilot does (`board.c` L291–293) |
| The core | from the vector table at the boot address | the same, by the macro's `cpu VectorTableOffset` | measured: PC `0x8000D8D`, SP `0x30000600`. Without the line the core restarts at 0 and faults (today's `fail`) |
| The external SPI flash (U4) and the sensors | not reset by the MCU's reset; a transaction in flight is ended by its chip select rising | Renode resets every model; the flash model logs its idle warning at the reset instant | measured: two warnings at 6.708900 s. Exempt within a quantum of a `SYSRESETREQ` line only (boot_tier1, and a mutant) |
| The sensors' physical inputs (gravity, pressure) | unchanged: physics does not reset | **unchanged:** our models' stimuli are auto-properties that `Reset()` never touches (`ICM42688P.cs` L70–85, `BMP581.cs` L61–69) | read at source |
| The judge's own hooks (panic, fault, peripheral write hooks) | — | **survive the reset** | measured: a CPU execution hook on the reset handler fires at 0 s and again at 6.708900 s (`reboot-hook`); peripheral hooks are `AET-WO-2302` P1–P5's |
| The input's timeline (injected bytes, stimuli) | — | goes on across the reset, in virtual time | by construction: the timeline is the monitor's, not the machine's |

## A firmware that passes because the twin stopped it (WO §5)

| what could stop early | what holds it | how established |
|---|---|---|
| The timer, skipping updates (the lazy schedule) | The model is eager whenever the firmware could observe an update: UDE, any CCxDE, UIE or any CCxIE set (ruling D-c). Otherwise it is lazy, and every skipped event that turns out to have an effect is logged as the model's own `lazy: a skipped event … had an effect` line. **New:** the judge fails that line as `foreign_timer_lazy_differed` and keeps it past the log budget | planted (`boot_tier1` `a_lazy_timer_that_differed_fails_the_run_by_name`) and mutants on the mark and the keep entry, both killed. Measured: 0 such lines in the frozen arm12 and dshot12 runs and in the armed Sentinel-03 run |
| A DShot stream ending before the firmware's | A paced stream ends where silicon's does: when NDTR reaches 0 (RM0433 Rev 9 §15.3.10, p. 647). A circular paced stream, which would never end, is refused and not started, and fails the run as `foreign_dma_stream_unbounded` (ruling (a)) | planted and mutated (`a_circular_paced_stream_is_refused_by_name`). Measured: 8,011 transfers on TIM1, the last ending at 11.9996 s of the 12 s window |
| The window | There is no end-of-window stop on the DShot path. TIM1's PSC and ARR are read at the run's end, and `boot.dshot.read` says so | measured: the frames run to the window's end |
| The judge's own log | Past the 8 MiB budget, the DMA model's transfer lines are dropped, so `boot.dshot` would undercount and a malformed frame could be among the dropped. **New:** a run that dropped lines and saw DShot transfers fails as `foreign_dshot_unwitnessed` | unit test and mutant, killed. Measured: the armed 12 s run keeps 2.9 MB |
| A circular stream that is not paced | Our DMA model serves it as stock Renode's `STM32DMA` does: at EN, whole. The armed run writes CIRC once, on DMA1 stream 3 (offset 0x58, at 3.29 ms, a peripheral's own stream, not a timer's) | **named, not changed:** an unpaced stream is the same gap stock Renode has for every generated and foreign board, and no DShot frame rides on it |

## A DMA request assumed (WO §5)

- **Which request each timer raises.** It is read from RM0433 Rev 9 Table 124 (p. 697), not assumed: `motor::UP_REQUESTS`, by page.
- **A timer with no update request.**
  - TIM12, TIM13 and TIM14 raise none, so a DShot output on one is refused at the solver as `NoDshotDmaRequest` (planted, `dshot_dma_tier1`).
  - The twin's emitter also refuses a retyped timer with no row (`TwinError::UnsupportedPeripheral`).
- **Which stream serves which request.** The firmware chooses it, and the DMA model reads that choice from DMAMUX1_CxCR.DMAREQ_ID as written; the platform does not wire it.
  - A stream paces only on a request in the platform's `pacedRequests`. That list is built from the topology's DShot timers alone.
  - Measured: ArduPilot routes request 15 (TIM1_UP) to DMA2 stream 1, and the transfer line carries both numbers.
- **Not covered:**
  - A firmware that paces a stream by a request it never enables (UDE clear) waits forever. The run then shows no transfer for that timer, and the motor protocol check reports PWM-rate updates as `foreign_motor_protocol_unmet`, a named info finding.
  - The case is named here and not planted.

## An ELF in CI that isn't the bench's (WO §5) — **open until Sentinel-03's CI row lands (waits on #395)**

- **hwdef.** CI generates Sentinel-03's hwdef.dat with `generate hwdef` from the committed specification, and holds it to the committed fixture (the generated job's `cmp`).
- **defaults.parm.** It is held to its fixture in the same step, or held absent where there is none.
- **Build.** The same `container/ardupilot-build.sh boards` builds it in the pinned ArduPilot image.
- **Clock.**
  - The specification's `foreign_clock` is held to ArduPilot's tree at run time (`ardupilot_clock::hold`, `data/ardupilot-h7-clock.tsv`, from mcuconf.h sha256 67f48e5f).
  - An ELF built for another tree is refused by name.
- **Local measurement:**
  - The ELF built here, with defaults.parm, has sha256 1cfc5e02.
  - Its hwdef lines are identical to the earlier build's, comments aside.
  - MAVLink reads back MOT_PWM_TYPE = 6.
- **To close:**
  - CI's sentinel-03 ELF sha256 against the write dispatch's package ELF (the one the bench flashes).
  - The bench-package-s03 target held (`1919`).
