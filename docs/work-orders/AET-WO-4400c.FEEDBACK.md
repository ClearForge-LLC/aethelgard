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

## The (h), reported first (measured and read at source)

**ArduPilot never probes the ICM-42688-P, because the generated `hwdef.dat` has no `IMU` line.** At 5.2055 s of virtual time it prints `INS: unable to initialise driver` and enters `config_error`. Measured by execution hooks in run `r5`: `AP_InertialSensor::init` at 5.204391580 s, `detect_backends` at 5.205016050 s, `AP_BoardConfig::config_error` at 5.205148750 s and `throw_error` at 5.205148820 s. This is the work order's §7 stop. Nothing was worked past it.

- **The line.**
  - The `hwdef.dat` from CI (sha256 `e69033da…`) holds `SPIDEV icm42688 SPI2 DEVID1 IMU_CS MODE0 2*MHZ 2*MHZ` and no `IMU Invensensev3 SPI:icm42688 ROTATION_…`.
  - Its header says why: `Not emitted: U5 (ICM42688P): no IMU line; its rotation is a mechanical fact no specification declares (its SPIDEV icm42688 is emitted)`.
- **The generator's rule:** `crates/aethelgard-hwdef/src/lib.rs` L760 and L866 (4400b FEEDBACK L47: `IMU` takes `<driver> SPI:<spidev> <rotation>`, and no `rotation` field exists in `aethelgard-ir`).
- **ArduPilot's reading.**
  - `AP_InertialSensor.cpp` L1220–1318: only `HAL_INS_PROBE_LIST` probes IMUs on a board like this one, because `AP_FEATURE_BOARD_DETECT` is 0 off FMUv3 (`AP_BoardConfig_config.h` L6–11). With no `IMU` line it is undefined, so `_backend_count == 0`, and the boot ends in `AP_BoardConfig::config_error("INS: unable to initialise driver")`.
  - **In the artifact itself:** the ELF holds no `Invensensev3` symbol at all (`readelf -s`: 0 matches), so the driver is not even compiled in. It does hold the string `INS: unable to initialise driver`.
- **Two witnesses agree.**
  - The console prints `INS: unable to initialise driver` (byte 477 of USART1's capture; its first byte written at 5.205549320 s).
  - The ICM-42688-P model's own log holds **no transaction in 15 s** (`spike/4400c/out/r2/models.log`: 0 `spi2.spi2mux.u5` lines after reset).
- **What a build would do** (for the architect; nothing chosen): give the specification a declared IMU orientation, so the generator can emit the `IMU` line; or emit `ROTATION_NONE` and state it. The second is a fact no specification declares, so it is the architect's question, not the builder's.

## Results — measured after the predictions (`f89a7847`)

**How it ran (measured).**
- Every container went through `spike/4400c/sh/img.sh`: `~/bin/capped-run --mem 8G`, `--network none`, TMPDIR unset, one at a time.
- The caps were read back before each container started (`spike/4400c/out/caps.log`): `memory.max=8589934592 memory.swap.max=0 pids.max=8192`.
- **The CLI's setup, not a reconstruction.** The unmodified CLI booted this board's own generated Rust firmware (`aethelgard generate` and `build` on `sentinel-02-own.json`; `simulate --for 100ms`: `pass`, finding `device_unmodelled` only). Its Renode argv was copied from `/proc` (`spike/4400c/out/renode-argv.txt`, `spike/4400c/sh/gen.sh`).
- `spike/4400c/drive.py` replays that script under the CLI's own `renode.config` (synchronous logging, virtual timestamps), with **only the ELF swapped**.
- Left out, as deviations:
  - the boot-word watchpoint hook and its serial injection;
  - the single `RunFor 0.1` and the judge's reads, replaced by steps of 100–500 ms, each followed by the driver's reads.

  Kept: the cold RCC writes, the boot-word fills, the device `logLevel -1` lines, the access logging, the capture backends (USART1's capture is the console), and the sensor stimuli.
- Runs:
  - `r1`: 2 s in 100 ms steps;
  - `r2`: 15 s in 500 ms steps;
  - `r3`: a 3 s diagnostic with one platform overlay;
  - `r4`: 100 ms with the judge's register reads;
  - `r5`: 6 s with execution hooks only (`spike/4400c/stubs-hooks.txt`).

  Evidence for each, except the 10 MB Renode logs, is in `spike/4400c/out/`.

### §1.2 The stall table

| # | Virtual time | Where | Class | What | Worked past? |
|---|---|---|---|---|---|
| — | 0 → 9.37 ms | ChibiOS start-up: PWR, RCC, the PLLs, flash, `SWS`, MPU, DMAMUX | — | **no stall.** `Storage: Using flash pages 14 and 15`, `Initialised Storage type=2`, then the banner `Init ArduCopter V4.7.2 (fb773567)` (its first byte at 9.37 ms), and `Firmware change: erasing EEPROM... done` | — |
| 1 | 0.1635 → 1.797 s | `logger.init` → `AP_Logger_Flash_JEDEC::probe`, SPI1 | **(p)** | **The model answers, the firmware does not receive it.** `u4`'s own lines: JEDEC `0x9F` → `EF`, `40`, `17`, `10` at 0.179610600 s. The console: `Unknown SPI Flash 0x00F0BB03`, and the logger disables itself; the boot continues. **Cause, measured:** the twin's emitted `spi1` and `spi2` declare no `DMARecieve -> dmamux1@…` line, which Renode 1.17.0's own `spi4` does (`stm32h7.repl` L197). ArduPilot receives SPI1 on DMA2 stream 1, DMAMUX request 37 (`stm32_dmamux.h` L72). With the overlay `spike/4400c/overlay-spi-dma.repl` (`spi1: DMARecieve -> dmamux1@37`, `spi2: … @39`), run `r3` prints `SPI Flash 0x00EF4017 found pages=32768 erase=64k` and the logger comes up | only in diagnostic `r3`, which stopped at 3 s, before the (h). A build would emit these lines in `aethelgard-twin`. The same gap would garble the IMU's SPI2 reads on any board whose `IMU` line exists |
| 2 | 3.774 s | `compass.init` | **(x)**, not a stall | `No Compass backends available`. The board declares no compass; roadmap §5 makes it optional and disabled indoors | not a stall |
| 3 | **5.2055 s** | `startup_INS_ground` → `ins.init` → `detect_backends` | **(h)** | `INS: unable to initialise driver`, then `config_error` → `throw_error`, entered at 5.205148750 s (`r5`'s execution hooks, `spike/4400c/out/r5/hooks.log`). `AP_INERTIALSENSOR_ALLOW_NO_SENSORS` is 0 (`AP_InertialSensor_config.h` L54–55, read in the image). `throw_error` should print `Config Error: …` and send a STATUSTEXT every 5 s (`AP_BoardConfig.cpp` L513–533), but in 15 s (`r2`) SERIAL0 carries neither: only heartbeats and one `msgid 111`. Why is **not established**. The core is in `__idle_thread` (PC `0x08100858`) at every later step boundary | **no: the §7 stop** |

- **Counts are exact.** The config collapses repeated log entries, but no repeat marker appears in any run's log (measured), so 1,626, 813 and 1,038 are whole counts.
- **What did not stall, against P4 and P5:**
  - `4400a`'s D8 stall (one byte on USART3 on Renode's stock platform) does not happen here. Every console byte is a CPU store to `TDR` from `UARTDriver::write_pending_bytes_DMA`+0x13e (1,038 of 1,038 logged writes). Renode's `usart1` declares no DMA request line (`stm32h7.repl` L298–304), so no transmit DMA is involved. Why 4400a's run stopped was not chased.
  - The storage erase on bank 2 completed.
- **EXTI:** `C1IMR1` reads `0xFFC00000` straight after load (lines 22 and 26 unmasked), so P4(c) and P8's condition are both out (measured).

### §1.3 The probe, witnessed

**Virtual times (measured, the models' own lines):**
- SPI1 dataflash: status `0x05` at 0.163538800 s, JEDEC `0x9F` at 0.179610600 s;
- **BME280** identity at **1.973600630 s**;
- **BMP581** identity at **1.973786350 s**;
- the first BMP581 data read at 2.013765940 s;
- **no IMU traffic in 15 s.**

ArduPilot's own clock runs 1.25× the virtual time (P6, measured two ways). So ArduPilot believes the barometer probe happened at about 2.47 s.

**These times are the platform as emitted, where the dataflash's SPI reads fail (stall 1).** With the (p) overlay (`r3`), the BME280 identity comes at **0.344289850 s** and the BMP581's at 0.344485340 s: the dataflash failure's timeouts cost about 1.6 s. Runs `r3`, `r4` and `r5` were made after `r2` had reached the (h).

**Against each part's `4210` script** (`crates/aethelgard-ir/data/parts.json`), with each expected line in the judge's own format (`crates/aethelgard-twin/src/model.rs` L208–218, L798–830; never from ArduPilot's source):

| Part | Step | Expected line (the judge's) | ArduPilot's traffic (`r2`, 15 s) | |
|---|---|---|---|---|
| BME280 (U2) | identity | `[NOISY] i2c1.u2: Write D0`, `[NOISY] i2c1.u2: Read 60` | exactly these, at 1.973600630 s, once | **satisfied** |
| BME280 | (no further step: `script: null`) | — | `Write 88` / `Read` 24 calibration bytes; `Write F4 57`; `Write F5 8`; 813 data reads `Write F7` + `Read`; 20 further `F4` and `F5` accesses | not named by any script |
| BMP581 (U7) | identity | `[NOISY] i2c1.u7: read 0x01 -> 0x50` | exactly this, at 1.973786350 s | **satisfied** |
| BMP581 | configure | `[NOISY] i2c1.u7: write 0x36 <- 0x40 0x81` (a two-byte burst), then 10 ms settle | **never.** ArduPilot writes `0x37 <- 0x00`, `0x36 <- 0x72`, `0x37 <- 0x3D`, three single-byte writes with other values | **violated** (as `i2c_configure_mismatch` would name it) |
| BMP581 | data | `[NOISY] i2c1.u7: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62` (25 °C, 101,325 Pa) | exactly this line, 1,626 times, from 2.013765940 s | **satisfied**, though only after a configure the script does not name |
| BMP581 | (not named) | — | `read 0x28 -> 0x02` (STATUS), `read 0x27 -> 0x10` (INT_STATUS) before configuring; 40 reads of `0x36` and 41 of `0x37` later | not named by any script |
| ICM-42688-P (U5) | identity, configure, data | `[NOISY] spi2.spi2mux.u5: bank 0 read 0x75 -> 0x47`, … | **nothing**: the (h) | not reached |
| W25Q64JV (U4) | identity (`0x9F` → `0xEF4017`, `script: null`) | served by `GenericSpiFlash` | the model served it; the firmware read `0x00F0BB03` (stall 1). With the (p) overlay it read `0x00EF4017` | served, not received |

- **SPI transfer lengths and `SPI_MAX_FRAMES`:** not reached for the IMU, which ArduPilot's FIFO reads would exercise. The dataflash's JEDEC transfer is 1 + 4 bytes (the model's lines). No IMU frame exists to measure.
- **The console as a second witness (`INV-07`).** ArduPilot prints no line per barometer found, so the console cannot witness the barometer probe (P12, measured: the console holds no `BMP` or `Baro` text). It witnesses only:
  - the dataflash outcome, which agrees with the model only once the (p) overlay is in;
  - `No Compass backends available`;
  - `INS: unable to initialise driver`, which agrees with the ICM model's silence.

  None of these lines witnesses anything on its own.

### §1.4 The owned rows

| Row | Reached? | Measured or why not |
|---|---|---|
| `SPI_MAX_FRAMES` 8 against FIFO reads up to 129 B (`4220` D-B4, F-B5) | **no** | the (h): no IMU driver runs. Stall 1 shows that SPI receive by DMA does not work on the emitted platform at all, so a build meets that first |
| The judge's DShot stop as a twin artifact (`2302` D2) | **no, unreachable on this board** | `sentinel-02-own.json` declares no motor output (`motor_outputs: []`; `HAL_USE_PWM FALSE`). `serial-dshot-9600`, which CI also builds, has DShot; not run here, as scoped |
| RC at 420000 against ArduPilot's 416666 (#189 F3) | **no, unreachable** | no RC port on this board |
| Each port's protocol and options (`4400b2` D2) | **SERIAL0 only** | SERIAL0 runs MAVLink 2: 36 frames with magic `0xFD`, `msgid 0` (heartbeat) and one `msgid 111`, in 15 s. USART1 `BRR` is written four times (3.2, 9.1, 95.7 and 99.87 ms), each `0x271` (625), so SERIAL0 keeps one baud here (`spike/4400c/out/r2/usart1.log`). ArduPilot intends `DEFAULT_SERIAL0_BAUD 115200`, so its USART1 kernel clock after the prescaler is 72 MHz (inferred: 625 × 115200). Renode's `usart1` model is fixed at 200 MHz (`stm32h7.repl` L303), so a baud derived from the model's frequency would read 320,000 (inferred) |

### §1.5 The judge, rule by rule

From the unmodified CLI (P1), and otherwise from `r4`: the judge's own register list read after 100 ms, its window (`spike/4400c/out/r4/judge-reads.json`, measured). **These are the spike's reads of each rule's observable, not the judge's verdict**, which refuses before any boot.

| Rule (CLI README, *Booting firmware*) | This boot | What a build would need (options, none chosen; `INV-07` weakenings named) |
|---|---|---|
| A `PT_LOAD` only in flash or DTCM, at both addresses (`elf.rs` L149–163) | **fails**: `FirmwareOutsideMemory`, `.data` at virtual `0x30002200` (exit 3, measured) | admit D2 SRAM1–3 and AXI SRAM from ST's header for a foreign firmware. No witness is weakened, since the rule guards loading, not booting |
| The boot word `0x2100B007` at `0x20000000` | **fails**: `0x0` at 100 ms; ArduPilot writes none | (a) an ArduPilot hook, which would no longer be unmodified ArduPilot; (b) a different witness of "booted", for example a model's identity line (firmware-independent, `INV-07`-sound) or the console banner (**weakens:** firmware-chosen text witnesses nothing alone); (c) a separate judge for foreign firmware |
| 100 MIPS × the virtual time | **fails**: 9,661,939 of 10,000,000 at 100 ms; 157,154,916 of 1,500,000,000 (10.5 %) at 15 s; the core sleeps in `__idle_thread` (`cpu_halted`) | count only while the core is not halted, or drop the rule for a foreign firmware. **Weakens:** the rule is today's witness that the core kept executing inside the firmware |
| The core in thread mode after the boot word (`ICSR.VECTACTIVE` 0) | **observable passes** at 100 ms (`ICSR` `0x800`), but there is no boot word to anchor it | keeps its meaning only with a new "booted" anchor |
| `PC` on a `yield; b.n` self-loop | **fails**: `0x0805F482` (`AP_Param::find_var_info_group`) at 100 ms; `__idle_thread` later | none fits an RTOS. Drop it for a foreign firmware (**weakens** the "parked" witness, D3) or replace it by "`PC` in `__idle_thread`", read from the ELF's symbols |
| Each solved pin's `MODER`, `AFRL`, `AFRH` | **passes**, 6 of 6 (`GPIOA` and `GPIOB` equal to the solved values) | unchanged. `gpio_default_config` comes from the same topology (`4400b`) |
| USART1 `CR1`, `CR2`, `CR3`, `BRR` | **fails** 3 of 4: `CR1` `0x10D` against `0x0D` (`PEIE`); `CR3` `0xC0` against `0x00` (DMA enables); `BRR` `0x271` against `0x22C`. `CR2` passes | the expectation is the generated firmware's (HSI, its own `BRR`). A foreign firmware needs an expectation from the solved baud and *its own* clock tree, or a `BRR`-free baud check on the wire |
| Each cold RCC enable register, exactly | **fails** 6 of 9: `AHB1ENR` `0x1`, `AHB2ENR` `0x40`, `AHB4ENR` `0x7FF`, `APB1LENR` `0x8`, `APB2ENR` `0x10`, `APB4ENR` `0x10002`, against `0`, `0`, `0x3`, `0x204000`, `0x1010`, `0x10000`. At 100 ms ArduPilot has not yet enabled I2C1, SPI1 or SPI2, and it enables more besides | "at least the solved bits", read once the drivers start, or not judged for a foreign firmware. **Weakens:** the exact set witnesses that nothing unsolved was clocked |
| `SysTick.CSR` | **fails**: `0x5`, against expected `0` | a foreign RTOS's tick is its own; not judged |
| `NVIC.ISER0`–`4` all 0 | **fails** 3 of 5: `ISER0` `0x80808FC0`, `ISER1` `0x40139`, `ISER3` `0x4000000` | not judged for a foreign firmware, or judged against an allowlist from the topology |
| `--for` at most 100 ms | **cannot be judged**: nothing ArduPilot does that a witness reads happens in 100 ms (barometer at 1.97 s, INS at 5.21 s) | a longer window for a foreign firmware: at least about 2.1 s to see both barometers' data steps. Measured cost: 1.09 s of wall per simulated second (`r2`, 15 s in 16.33 s of steps) and a peak Renode RSS of 460 MiB (45 % of the 1 GiB bound), so 2.5 s fits the 30 s wall bound with room |
| The `4210` witness, in order, from the models' own lines | **fails**: within 100 ms nothing; at 15 s BME280 identity satisfied, BMP581 identity and data satisfied, BMP581 configure violated, ICM-42688-P absent (the (h)) | a script per firmware, or an identity-and-data-only witness for a foreign firmware. **Weakens:** the configure step is the witness that the driver set the part as the script requires |
| `device_unmodelled` (TMP117) | the same: info | unchanged |
| The serial witness (`2302`: the generated firmware's pattern, then the echo of injected frames) | **fails**: SERIAL0 carries MAVLink 2, not the pattern | a witness of the declared framing (baud, bits) independent of the payload |

### Measurements

| Quantity | Value | |
|---|---|---|
| Wall per simulated second | 1.09 s (`r2`, 15 s; 0.07–2.0 s per 500 ms step); 1.85 s (`r1`, 2 s, start-up included) | measured |
| Peak Renode RSS | 460 MiB (`r2`); 447 MiB (`r1`) | measured |
| Instructions | 10.5 % of 100 MIPS × time at 15 s; the core idles | measured |
| ArduPilot's clock against virtual time | 1.25× (TIM5 at 1.25 MHz: `PSC` 199 against the model's fixed 250 MHz) | measured |
| Virtual time to the banner, the dataflash probe, the barometers, INS | 9.37 ms; 0.1635 s; 1.9736 s; 5.2055 s | measured |

## Each prediction beside its result

| id | Predicted | Result | |
|---|---|---|---|
| P1 | `FirmwareOutsideMemory` at virtual `0x30002200`, before any boot | exactly that, exit 3 | held |
| P2 | ChibiOS's start-up passes | passes; banner at 9.37 ms | held |
| P3 | one `\n`, then nothing | the whole storage report and banner, then 1,038 bytes in 15 s | missed |
| P4 | the first stall at the console's first DMA transmit, (m) | none: every console byte is a CPU store to `TDR`, and USART1 has no DMA line in Renode's platform. EXTI line 26 is unmasked (`C1IMR1` `0xFFC00000`), and TIM5's compare interrupt runs ArduPilot's clock | missed |
| P5 | storage on bank 2 stalls, (m) | `Initialised Storage type=2`; the erase completed | missed |
| P6 | ArduPilot's clock 1.25× the virtual time | 1.25×, measured two ways: TIM5 counts 125,000 per 100 ms virtual (`PSC` 199 at the model's 250 MHz), and ArduPilot's own `Dataflash: erase done (1793 ms)` spans 1.434948 s of virtual time between its two lines (`r3`), 1.2495× | held |
| P7 | the dataflash probe before the barometer, stalling if SPI DMA has a gap | probed at 0.16 s, and SPI receive DMA has a gap; not a stall but a garbled ID, a **(p)**: the emitted platform lacks the DMA request lines | held in substance; the class is (p), not (m) |
| P8 | the barometer probes, if EXTI line 22 is unmasked | unmasked; both probed at 1.9736 s | held |
| P9 | BME280 identity satisfied; BMP581 identity satisfied, configure violated, data matched once ArduPilot reads in normal mode | exactly that | held |
| P10 | the IMU never probed: the (h), and the stop | `INS: unable to initialise driver` at 5.2055 s; no ICM traffic; stopped | held |
| P11 | `SPI_MAX_FRAMES`, DShot and RC rows unreachable; SERIAL0 is MAVLink 2 | as predicted. `BRR` is `0x271` | held |
| P12 | the console cannot witness the barometer; the banner, then `INS: …` and `Config Error: …` repeated | no barometer text; the banner and `INS: unable to initialise driver`, but **no `Config Error` text and no STATUSTEXT in 15 s**, though `throw_error` was entered (`r5`) | held for the barometer, missed for `Config Error` |
| P13 | 1–4 s of wall per simulated second; 400–500 MiB | 1.09–1.85 s; 447–460 MiB | held |
| P14 | the judge's rules, mostly failing | as the table in §1.5; pins pass, `ICSR` observable passes | held |

## Decisions needed

- **D1 (architect; the (h)).** How the generated `hwdef.dat` gets an `IMU` line:
  - (a) a declared IMU orientation in the specification (`aethelgard-ir`), from which the generator emits `IMU Invensensev3 SPI:icm42688 ROTATION_…`;
  - (b) emit `ROTATION_NONE` and state it as the generator's constant, which is a fact no specification declares.

  Until then, this build of unmodified ArduPilot cannot pass `ins.init`, on the twin or on copper: no IMU driver is compiled into its ELF.
- **D2 (architect; stall 1, (p)).** `aethelgard-twin` emits SPI receive DMA lines (`DMARecieve -> dmamux1@37` and `@39` for SPI1 and SPI2), as Renode's own `spi4` does. Measured to fix the dataflash ID. Whether transmit DMA also needs a line was not measured: the receive line alone sufficed for a JEDEC read.
- **D3 (architect; §1.5).** Which option per rule, if foreign firmware is to be judged. The ones that weaken a witness are named in the table: the console banner as "booted", dropping the instruction count, dropping the self-loop, "at least" RCC bits, and identity-and-data-only scripts.
- **D4 (architect).** The window for a foreign firmware. Measured: barometer data at 2.01 s virtual, at 1.09 s of wall per simulated second and 460 MiB.
- **D5 (architect).** The RC and DShot rows need a board that has them. `serial-dshot-9600`, which CI builds, has DShot and serial ports. It was not run, as scoped.

## §7 checked

- **An (h) stall:** reached at 5.2055 s and stopped there. It is reported first, with the line, the generator's rule and ArduPilot's reading. Nothing was worked past it:
  - run `r3`'s overlay addresses stall 1 and stops at 3 s;
  - run `r5` adds only execution hooks, which change no state, and stops at 6 s, 0.8 s after the (h), to show that `config_error` was entered.
- **Booting needs a change to a crate, model or image:** no. The boot reached the (h) unmodified; the one overlay is diagnostic and lives in `spike/4400c/`.
- **§2 is the stop:** this.

## Gates (spike)

- **No crate, model, workflow or `container/` file changed:** `git diff --stat ac5f8c82 -- crates models .github container` is empty.
- **The ELF is not in git.** It is named by its sha256 above, with the command and the CI run that build it.
- **Evidence:** `spike/4400c/out/`. Each run's step reads, setup, model lines and USART1 capture are there. For `r2`, `r3` and `r5`, so are the timestamped `usart1:` lines, from which every console time here is read, and the hook lines. Also there: the captured CLI argv, both `simulate` outputs and `caps.log`. The scripts are `spike/4400c/drive.py` and `spike/4400c/sh/`.

**STOPPED at §2**, 2026-11-17, for the architect's ruling on D1–D5. The PR is a record only. Its title and body are in `~/.cache/aet-4400c/pr-title-body.md`.
