# AET-WO-4400b FEEDBACK — ArduPilot's board definition generated from the solved topology

**Builder:** Claude (strong tier), worktree `~/aethelgard-4400b`, branch `wo/aet-4400b-hwdef` from `main` `36e428b`.
**Cadence:** build, with one measured stop (§1.1). The work order is read from `origin/claude/argon-repo-docs-synthesis-rjrb0d` (docs PR #187) and is not committed here.

Labels:
- **[read]**: read at source, this repository at `36e428b` or ArduPilot `Copter-4.7.2` (`fb773567`), the tree inside the pinned image `container/ARDUPILOT-IMAGE` (`160f9955…`), copied out read-only. `CH` is `libraries/AP_HAL_ChibiOS/hwdef/scripts/chibios_hwdef.py`; `HB` is `libraries/AP_HAL/hwdef/scripts/hwdef.py`.
- **[measured]**: run on Furnace. Each container runs under `~/bin/capped-run` with the cap read back.
- **[inferred]**: neither.

Predictions (§1) were committed before any ArduPilot build of this work order, and are never edited afterwards; results go in §3.

## 0. Source reads behind the predictions

**Today's topology** [measured, `aethelgard solve` at `36e428b`, solved schema 8]:

| | Sentinel-01 (`examples/sentinel-01-minimal.json`) | Sentinel-02-shaped (`crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json`) |
|---|---|---|
| serial ports | `CONSOLE`, USART1, PA9/PA10, role `console` | **none** |
| I2C | `SENSORS` on I2C1 (PB6/PB7), 400 kHz; BME280 at 0x76, TMP117 at 0x48 | the same, plus BMP581 at 0x46 |
| SPI | `FLASH` on SPI1 (PA5/6/7), W25Q64JVxIQ, CS `FLASH_CS` PA0; controller declared 20 MHz, achieved 16 MHz, fastest 16,239,744 Hz | the same, plus `IMU` on SPI2 (PA12/PB14/PB15), ICM42688P, CS `IMU_CS` PA1; declared 4 MHz, achieved 2 MHz |
| motors | none | none |
| power sense | none (`power` is `null`) | none |
| clock | HSI, `hse_hz` 0 | HSI, `hse_hz` 0 |
| solved pin speeds (`check speeds`) | SCK, MOSI `01`; every other pin `00` | SPI1 SCK/MOSI `01`; SPI2 SCK/MOSI and every other pin `00` |

- **No committed fixture solves with power sense pins** [measured, every `crates/aethelgard-cli/tests/fixtures/*.json`]. `serial-dshot-9600.json` is the only one with both a console and motors (four DShot600 on TIM1, PA8–PA11; console USART1 on PB14/PB15).
- **`SolvedTopology` has no `mcu` field** and no per-pin output speed [read, `crates/aethelgard-ir/src/lib.rs:389-452`]. The speed is chosen at generation time by `aethelgard-svd-hal`'s `pin_speeds` (`board.rs:897`), from `spi_lines` through `aethelgard_ir::speeds::OutputSpeeds::choose`, and `00` for every role out of scope (`Role::scope`, `board.rs:840`).
- **Pin policy** (output type and pull) for our own board crate is `Role::policy` in svd-hal (`board.rs:823-831`): I2C open-drain, no internal pull; SPI, USART and a GPIO output push-pull, no pull; a timer channel push-pull, pulled down. Only the motor pull-down is a rule in `aethelgard-ir` (`silicon::motor_output_pupd`, `silicon.rs:763-770`).
- **The MCU's part row** is in `power-parts.json` (schema 3, `kind.mcu`), not `parts.json` (schema 6), which holds the bus devices only.
- **`DRIVEN_SPI_MODE` is 0** (`parts.rs:136`).

**ArduPilot at the tag** [read]:
- **Per-pin tokens.** A pin line's extra tokens set `OSPEEDR` (`SPEED_VERYLOW`, `SPEED_LOW`, `SPEED_MEDIUM`, `SPEED_HIGH` = `00`…`11`, default `SPEED_MEDIUM`, CH:365-385), `OTYPER` (`PUSHPULL`, `OPENDRAIN`; default open-drain for an `I2C*` type, CH:349-363) and `PUPDR` (`FLOATING`, `PULLUP`, `PULLDOWN`; default pull-up on a CS pin and on a USART/UART TX, RX or CTS, CH:387-431). A token overrides the default.
- **`SPIDEV`** takes exactly seven tokens; speeds must end in `*MHZ` or `*KHZ` (CH:1451-1485). ArduPilot then picks its own divider on its own kernel clock, at or below the speed given [inferred from `SPIDevice.cpp`; not chased].
- **`STM32_*` lines** are written through as `#define` (CH:1004-1006); `define X` lines likewise. **Any other unknown keyword is stored and never read** (CH:2885).
- **`HAL_I2C_MAX_CLOCK`**: default 100 kHz; every bus starts at it and a driver can only lower its bus (`I2CDevice.cpp:53-54, 209`).
- **Storage.** `STORAGE_FLASH_PAGE p` uses pages `p` and `p+1` (`Storage.cpp:331-336, 427`). The linker then ends the image at page `p`'s offset (CH:1287-1292). `HAL_STORAGE_SIZE` must be less than one page (CH:849-865). The H753's flash is two banks of eight 128 KB sectors (DS12117 Rev 10 §3.3.1, p. 25; Table 2, p. 20: "2 x 1 Mbyte"). So the last sector usable as `p` is **14**, not 15. `FLASH_RESERVE_START_KB 0` also writes `HAL_ENABLE_SAVE_PERSISTENT_PARAMS 0` (CH:1030-1032).
- **`APJ_BOARD_ID`.** `board_types.txt` reserves 1000–19999 for allocation by pull request to ArduPilot (lines 72-77). Its largest ID is 41777 (ModalAI, lines 38-40), and no comment reserves anything at 20000 or above. **24400** is unassigned and outside every reserved range. The numeric form is accepted with `GITHUB_ACTIONS` unset (HB:129-139, `4400a` D4).
- **Drivers against our parts' identities:**
  - BME280 (0x60) is `BMP280` (`AP_Baro_BMP280.cpp:39, 76`);
  - BMP581 (0x50) is `BMP581` (`AP_Baro_BMP581.cpp:23, 99`);
  - ICM42688P (0x47) is `Invensensev3` (`AP_InertialSensor_Invensensev3.cpp:139, 1011`);
  - W25Q64JVxIQ (0xEF4017) is the JEDEC logger on `SPIDEV dataflash` (`AP_Logger_Flash_JEDEC.cpp:54, 65`);
  - TMP117 (0x0117) has none: `TMP119` requires 0x2117 (`AP_TemperatureSensor_TMP119.cpp:31, 62`).
- **`HAL_LOGGING_DATAFLASH_ENABLED`** defaults off on ChibiOS (`AP_Logger_config.h:15-17`), so a dataflash device needs the define.
- **`BARO`** accepts `<driver> I2C:<order index>:<address>` (HB:557-589). `IMU` accepts `<driver> SPI:<spidev> <rotation>` (HB:382-426). Rotation is a mechanical fact that no specification field declares today [read: no `rotation` or `orientation` in `aethelgard-ir`].

## 1. Predictions (committed before any build of this work order)

All **[inferred]** from §0 unless marked otherwise.

**The measurement.** Four boards are hand-assembled in `~/.cache/aet-4400b/boards/` exactly as the proposed generator would emit them. They are built in the pinned image with `container/ardupilot-build.sh boards`, and each ELF's `gpio_default_config` is decoded:
- `s01`: Sentinel-01;
- `s02n`: the Sentinel-02-shaped fixture, with no serial port, because its topology has none;
- `s02c`: `s02n` plus a console no declared fact chose (USART3, PD8/PD9). Measurement only; it would never be emitted;
- `dshot`: `serial-dshot-9600.json`.

1. **`s01` configures and builds,** with `OSCILLATOR_HZ 0`, `STORAGE_FLASH_PAGE 14`, `HAL_STORAGE_SIZE 16384` and `APJ_BOARD_ID 24400`. Flash used is within 2 % of step G's 1,285,344 B, and `FLASH_LOAD_ADDRESS` is `0x08000000`. `hwdef.h` holds:
   - `STM32_HSECLK 0U`;
   - `HAL_I2C_MAX_CLOCK 400000`;
   - `STORAGE_FLASH_PAGE 14`;
   - `STM32_ST_USE_TIMER 5`;
   - `BOARD_FLASH_SIZE 2048`.
2. **`s01`'s `gpio_default_config`:**
   - **Port A:** `MODER` is `0x0028a801` (PA0 output; PA5–7, PA9 and PA10 alternate). `OSPEEDR` is `10` on every pin the file does not name, `01` on PA5 and PA7, and `00` on PA0, PA6, PA9 and PA10. `OTYPER` is 0 and `PUPDR` is 0.
   - **Port B:** `OTYPER` is `0xc0`, and `OSPEEDR` is `00` on PB6 and PB7.
   - **`PUPDR` is 0 on every port.** The `FLOATING` tokens remove ArduPilot's pull-ups on PA0, PA9 and PA10, which `4400a` measured on step G.
3. **`s02n` configures and fails to compile** with `#error "SERIAL driver activated but no USART/UART peripheral assigned"`, the error the spike's steps B and E measured.
4. **`s02c` builds.** Both `BARO` lines (BMP280 at 0x76, BMP581 at 0x46) and both `SPIDEV`s are accepted. With no `IMU` line, the `icm42688` device is in the SPI table and nothing probes it.
5. **`dshot` builds**:
   - PA8–PA11 are alternate AF1 with `PUPDR` `10` (pull-down) and `OSPEEDR` `00`;
   - ArduPilot's DMA resolver places TIM1's DShot DMA without a `DMA_NOSHARE` line;
   - `STM32_ST_USE_TIMER 5` conflicts with nothing.
6. **Cost:** 50–90 s wall per board at `-j$(nproc)`, and a scope peak under 2 GB against the 8 GB cap.
7. **Today's `check pinout`** (mode and AF only), against each board's own topology with no allow-list:
   - `s01` and `dshot` pass;
   - `s02c` fails with exactly `reverse UnnamedPin PD8` and `PD9`.
8. **The field table** (§2) finds exactly one required line with no source: the **serial port of the Sentinel-02-shaped fixture**. Every other line a build or `check pinout` needs is `[T]`, `[P]` after this work order's columns, or `[C]`. Two expected values of the grown check have a source only by a decision (§2.3): the per-pin speed and the pull.
