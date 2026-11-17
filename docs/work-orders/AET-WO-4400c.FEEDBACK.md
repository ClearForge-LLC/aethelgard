# AET-WO-4400c — FEEDBACK (spike; stops at §2)

Branch `wo/aet-4400c-ardupilot-twin`, from `main` at `ac5f8c82`. The work order is read from `~/.cache/aet-4400c-wo.md`, the architect's copy; it lands on `main` by a docs PR and is not committed here. Prototypes live under `spike/4400c/`; no crate, model, workflow or `container/` file changes.

Every answer is labelled **measured**, **read at source** or **inferred**.

**The ELF (not in git).** `arducopter.elf`, sha256 `c8a2c9fa915abcdb1cca10eb4eeee3786ea57f3f3eac075a132bb890a1f4559e`, 2,491,248 B.
- **Source:** CI's own, the artifact `generated-sentinel-02` of `ardupilot-image` run 44051581563 on `main` at `ac5f8c82`, the base of this branch. Its `hwdef.h` sha256 is `6906ac97…` and its `hwdef.dat` sha256 `e69033da…`.
- **Rebuilds with:** `container/ardupilot-build.sh boards sentinel-02`, in `container/ARDUPILOT-IMAGE`, on the `hwdef.dat` that `aethelgard generate hwdef` emits from `crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json` (`ardupilot-image.yml`, job `generated`).

## §1.1 Predictions — written before any run

### What the predictions rest on (read at source, nothing run)

**The board, from `sentinel-02-own.json` and the CI `hwdef.dat` and `hwdef.h`:**
- STM32H753, `OSCILLATOR_HZ 0`, so the HSI drives the PLLs;
- one serial port, USART1 on PA9/PA10 at 115200 (SERIAL0, the console; `HAL_HAVE_SERIAL1 0`);
- I2C1, carrying `BARO BMP280 I2C:0:0x76` (the BME280) and `BARO BMP581 I2C:0:0x46`;
- SPI1 with `SPIDEV dataflash` (W25Q64JV at 16 MHz), and SPI2 with `SPIDEV icm42688` (2 MHz);
- **no `IMU` line**, **no motor output** (`motor_outputs: []`, `HAL_USE_PWM FALSE`), no RC port, no USB, no SD card;
- DMA, from `dma_resolver.py`:
  - USART1 TX on DMA1 stream 4 and RX on DMA1 stream 0;
  - SPI1 TX on DMA1 stream 1 and RX on DMA2 stream 1;
  - SPI2 on DMA1 streams 2 and 3;
  - I2C1 without DMA;
- the system timer is TIM5 (`STM32_ST_USE_TIMER 5`);
- storage is flash pages 14 and 15 (bank 2, `0x081C0000`).

**ChibiOS's start-up on the H753** (`stm32h7_mcuconf.h`, `hal_lld.c`):
- `PWR_CR3 = LDOEN | USB33DEN`, VOS scale 1, then waits on `PWR_CSR1.ACTVOSRDY` and `PWR_D3CR.VOSRDY`;
- HSI at 64 MHz, `HSIDIV` 1. PLL1 is `DIVM 4 × DIVN 50 / DIVP 2`, giving 400 MHz; PLL2 and PLL3 are also set up, and HSI48 is enabled;
- each `RDY` bit is awaited in `RCC_CR`; then the flash wait states, and the switch to PLL1 with a wait on `RCC_CFGR.SWS`;
- after that, the MPU regions for the non-cacheable SRAM, then DMAMUX.

**Renode 1.17.0's platform**, which the twin's `.repl` builds on (`platforms/cpus/stm32h753.repl` → `stm32h743.repl` → `stm32h7.repl`, read in the tools image):
- `pwr` is a Python stub. It answers `CSR1 = 0x6000` (`ACTVOSRDY`), `CR3` with `USB33RDY` set, and `D3CR = 0x2000` (`VOSRDY`);
- `rcc` is `STM32H7_RCC`;
- `dma1`, `dma2` and `dmamux1` are `STM32DMA` and `STM32_DMAMUX`; each DMA stream goes straight to the NVIC;
- **USART1's IRQ goes to `exti@26`, and I2C1's event IRQ to `exti@22`.** Both reach the NVIC only through EXTI, whose CPU mask ChibiOS never writes for these direct lines (it relies on the reset value; grep of ChibiOS's STM32 HAL and `AP_HAL_ChibiOS`);
- `timer5` and the `dwt` have a fixed `frequency: 250000000`, whatever RCC holds;
- `nvic`'s SysTick frequency is 480 MHz;
- there is no IWDG peripheral, and USB is a stub.

**The twin's emitted platform** (`aethelgard generate`, `sentinel-02-own.repl`, 73 lines) adds:
- `spi1` and `spi2` as `STM32H7_SPI`, each with an `SPIMultiplexer` on PA0 and PA1;
- `u2` `I2C.BME280` at 0x76, and `u7` `Sensors.BMP581` (ours) at 0x46;
- `u4` `GenericSpiFlash` (JEDEC `EF 40 17`), and `u5` `Sensors.ICM42688P` (ours);
- the `FLASH_OPTSR` brown-out presentation;
- U3, the TMP117, is unmodelled.

**ArduPilot's init order** (Copter-4.7.2, `AP_Vehicle::setup`, `Copter::init_ardupilot`):
1. `serial_manager.init_console`;
2. the banner `"\n\nInit ArduCopter V4.7.2 …"` and the free RAM;
3. `load_parameters`, which opens storage on flash pages 14 and 15;
4. the scheduler and GCS;
5. `serial_manager.init`;
6. `BoardConfig.init`;
7. **`logger.init`**, where the dataflash backend probes JEDEC `0x9F` on SPI1;
8. `init_ardupilot`, which runs **`barometer.init()`** (L35: the BMP280 driver at 0x76, then BMP581 at 0x46), then `gcs().setup_uarts`, `barometer.calibrate()`, and later `startup_INS_ground()` → **`ins.init()`** (L214);
9. inside `ins.init()`, `detect_backends()`.

**`detect_backends()` with no `IMU` line** (`AP_InertialSensor.cpp` L1220–1318): `HAL_INS_PROBE_LIST` is undefined, so nothing is probed. `_backend_count == 0`, so ArduPilot prints `INS: unable to initialise driver` and calls `AP_BoardConfig::config_error`, which loops reporting it.

**The judge** (`crates/aethelgard-cli/src/elf.rs` L149–163): every `PT_LOAD` with a `memsz` must sit inside flash or DTCM RAM at both its physical and its virtual address. ArduPilot's `.data` segment is virtual `0x30002200` (D2 SRAM1), so `simulate --firmware` refuses this ELF as `FirmwareOutsideMemory` **before any boot**. The spike therefore drives Renode itself:
- the emitted platform and the CLI's own setup, as `2303`'s probe drove it;
- the boot RunFor in steps, with the PC, the instruction count, the console (USART1 to a file) and the models' own lines read after each step.

### Predicted

| id | What | Prediction |
|---|---|---|
| P1 | `simulate --firmware` on CI's ELF, unmodified | refused before any boot, `FirmwareOutsideMemory`, naming the virtual address `0x30002200` (read at source; confirmed by one run) |
| P2 | ChibiOS's start-up: PWR, RCC, the PLLs, flash latency, `SWS`, the MPU, DMAMUX | passes; every wait it makes is on a bit Renode answers (`4400a` D8 reached the console's first byte on Renode's own `stm32h753.repl`) |
| P3 | The first output | one byte on USART1, `\n`, as `4400a`'s D8 measured on USART3 |
| P4 | **The first stall** | **(m)**, right after that byte: the console's first DMA transmit never completes, so the UART thread waits on `EVT_TRANSMIT_DMA_COMPLETE`. That wait has a timeout (`UARTDriver.cpp` L924), so the core also idles in `__idle_thread`, as `4400a` measured. Three named candidates: **(a)** DMA1 stream 4's transfer-complete interrupt; **(b)** TIM5's compare interrupt, ChibiOS's tick-less system time, so no timeout ever fires; **(c)** EXTI line 26's mask for USART1, which matters only if the driver waits on the USART's own interrupt. I rank them (b), (a), (c) |
| P5 | Past P4, with a stub | the banner prints. Then the next stall is **(m)** again: storage on bank 2's pages 14 and 15. Renode's `flashBank2` starts at 0x00, not 0xFF, so `AP_FlashStorage` erases a sector through `STM32H7_FlashController`, whose bank-2 erase and program I expect to be partial |
| P6 | Virtual time | ArduPilot's clock runs **1.25× fast** against the virtual time (inferred): ChibiOS sets TIM5's prescaler from its computed 200 MHz APB1 timer clock, and Renode's TIM5 counts at 250 MHz |
| P7 | The dataflash probe (`logger.init`) | reached before the barometer: JEDEC `0x9F` on SPI1, answered `EF 40 17` by `GenericSpiFlash`, if SPI1's DMA completes. If SPI DMA has the same interrupt gap as P4, it stalls here (m) |
| P8 | **The barometer probe** | if I2C1's event interrupt reaches the NVIC through EXTI 22: BMP280 first (`0xD0` → `0x60`, accepted as the BME280's id; then the calibration block at `0x88`; then config writes), then BMP581 (`0x01` → `0x50`; a soft reset `0x7E ← 0xB6`; the status and `INT_STATUS` reads; the OSR and ODR writes at `0x36`/`0x37`). If EXTI line 22 is masked at reset in Renode, **every I2C transfer times out**, both probes fail, and ArduPilot reports no baro (`config_error`): (m) |
| P9 | The probe against the `4210` scripts | BME280: identity satisfied. BMP581: identity satisfied; the configure step (`0x36 ← 0x40 0x81`) not matched as written, because ArduPilot's sequence writes other registers and values first; the data read (`0x1D`, 6 bytes) matched only once ArduPilot reads in normal mode |
| P10 | **The IMU** | **never probed.** No `IMU` line, so `HAL_INS_PROBE_LIST` is undefined, and ArduPilot reports `INS: unable to initialise driver` in `config_error`. That is an **(h)** stall: the generated `hwdef.dat` lacks the line ArduPilot needs to use the board's IMU. **The work order's §7 stop: I stop there** and do not work past it. The ICM-42688-P's model sees no SPI2 traffic, and the `SPI_MAX_FRAMES` row is not reached |
| P11 | The owned rows | `SPI_MAX_FRAMES`: not reached (P10). DShot and its stop: not reachable, because the board declares no motor output. RC 416666: not reachable, no RC port. Port protocol and options: SERIAL0 only, MAVLink2 at 115200, read from USART1's `BRR` |
| P12 | The console as a second witness | the banner, then `INS: unable to initialise driver` / `Config Error: …` repeated, if the console works. ArduPilot prints no line per barometer found at boot, so the console cannot witness the baro probe |
| P13 | Cost | 1–4 s of wall per simulated second at the twin's 100 MIPS (inferred: ArduPilot idles most of the time); Renode about 400–500 MiB |
| P14 | The judge, rule by rule (predicted) | boot word: **fail** (never written); 100 MIPS × time: **fail** (`WFI` in the idle thread, `cpu_halted`); self-loop: **fail**; solved registers: **mixed** (pin modes and AFs as solved, since ArduPilot's `gpio_default_config` comes from the same topology; RCC enables differ: ArduPilot enables more than the solved set, so `cold` registers fail); `--for` ≤ 100 ms: **cannot be judged**, nothing ArduPilot does reaches its probes within 100 ms; the `4210` witness: **fail** within 100 ms |
