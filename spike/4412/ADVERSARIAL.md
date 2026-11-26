# AET-WO-4412 adversarial pass (in progress)

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
