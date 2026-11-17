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

## 2. §1.1, the field table against today's topology (measured and read)

Key:
- `[T]`: the `SolvedTopology`;
- `[P]`: a parts-table row;
- `[C]`: a fixed generator constant, named in the emitted header;
- `[N]`: nowhere.

"After" marks what this work order adds: the topology's `mcu`, and the parts columns. Every line below was built in the pinned image (§3) in the form shown, except where marked.

### 2.1 The lines a build needs

| line | form emitted (Sentinel-01) | source | read by ArduPilot at |
|---|---|---|---|
| `MCU` | `MCU STM32H7xx STM32H753xx` | `[T]` after: `solve` copies `mcu.part` (`STM32H753ZITx`), solved schema 9. `[P]` after: the ArduPilot series and table name, a column on the MCU's row | CH:2894-2897 |
| `OSCILLATOR_HZ` | `OSCILLATOR_HZ 0` | `[T]` `clock.hse_hz` | CH:885, as `STM32_HSECLK 0U` (measured) |
| `FLASH_SIZE_KB` | `FLASH_SIZE_KB 2048` | `[P]` after: DS12117 Rev 10 Table 2 (p. 20, "2 x 1 Mbyte") and §3.3.1 (p. 25) | CH:1022, as `BOARD_FLASH_SIZE 2048` (measured) |
| `APJ_BOARD_ID` | `APJ_BOARD_ID 24400` | `[C]`, unassigned and outside every reserved range: 1000–19999 is ArduPilot's (`board_types.txt`:72-77), and the file allocates nothing above 19,999 ("do not allocate board IDs above 19,999 in this file", :619) beyond ModalAI's three `TARGET_HW` rows (:38-40) | HB:132-139; `hwdef.h` `APJ_BOARD_ID 24400` (measured) |
| no bootloader | `define AP_BOOTLOADER_FLASHING_ENABLED 0`; `FLASH_RESERVE_START_KB 0` | `[C]` (`4400` D3, ruled) | CH:1004-1010; `FLASH_LOAD_ADDRESS 0x08000000` (measured) |
| storage | `define HAL_STORAGE_SIZE 16384`; `STORAGE_FLASH_PAGE 14` | `[C]` size; `[C]` rule "the last two sectors", from `[P]` `FLASH_SIZE_KB` and the H7's 128 KB sectors (CH:799-801) | CH:1056-1059, 849-865, 1287-1292 |
| system timer | `STM32_ST_USE_TIMER 5` | `[C]` TIM5 when no motor uses it, else TIM2, else refused by name | CH:1004-1006 (an `STM32_*` line is passed through as a define) |
| serial | `SERIAL_ORDER USART1`; `PA9 USART1_TX USART1 …`; `PA10 …` | `[T]` `serial_ports` and `assignments`, the console first | CH:1682-1790 |
| **serial, Sentinel-02-shaped** | — | **`[N]`: the fixture declares no serial port, and ArduPilot builds only with a USART (§3, `s02n`)** | — |
| I2C | `I2C_ORDER I2C1`; pin lines | `[T]` `assignments`, `i2c_devices` (bus order by first device index) | CH:1931-1942 |
| I2C clock | `define HAL_I2C_MAX_CLOCK 400000` | `[T]` the minimum of `i2c_speeds` (ruled) | `I2CDevice.cpp:53, 209` |
| SPI | pin lines; CS `PA0 FLASH_CS CS` | `[T]` `assignments`, `gpio`, `spi_devices` | CH:1451-1485 |
| `SPIDEV` | `SPIDEV dataflash SPI1 DEVID1 FLASH_CS MODE0 16*MHZ 16*MHZ` | name `[P]` after (an ArduPilot column); bus, CS `[T]`; `DEVIDn` `[C]` (n = place on its bus, from 1); `MODE0` `[T]`/`[P]` (`DRIVEN_SPI_MODE`, `parts.rs:136`); speeds `[T]` `spi_controllers[..].achieved_hz` | CH:1451-1485 |
| logger | `define HAL_LOGGING_DATAFLASH_ENABLED 1` | `[P]` after: the part's role (`dataflash`) | `AP_Logger_config.h:15-17` |
| `BARO` | `BARO BMP280 I2C:0:0x76`; S02 also `BARO BMP581 I2C:0:0x46` | driver `[P]` after; index, address `[T]` | HB:557-589; `HAL_BARO_PROBE1/2` (measured) |
| ADC | `define HAL_USE_ADC FALSE` | `[T]`: no `power.sense` | step C (`4400`) |
| `IMU` (S02) | **omitted, named** | driver `[P]` after; rotation `[N]`, a mechanical fact nothing declares | HB:382-426 |
| TMP117 | **no line, named** | no ArduPilot driver for 0x0117 | `AP_TemperatureSensor_TMP119.cpp:31, 62` |
| motors | `PA8 TIM1_CH1 TIM1 PWM(1) …` (`serial-dshot-9600` only) | `[T]` `motor_outputs`, `assignments`; `PWM(n)` from `index`+1 | CH:1993-2082; `HAL_PWM_COUNT 4` (measured) |
| `BIDIR` | never emitted | nothing declares bidirectional DShot (`MotorOutput`, `motor.rs:82-90`) | — |
| ADC sense with `SCALE` | not reached by any board | pins `[T]` `power.sense`; **`SCALE` `[N]`**: `SolvedSense` holds no divider ratio. No committed fixture solves with sense, so the generator refuses such a topology by name until a source exists (D7) | — |
| `IMU`, sense, `BIDIR` | — | not needed to build | — |

### 2.1a Solved facts the file does not carry (read; added after the advisor's review, before the push)

None of these is needed to build or for `check pinout`. Each is a fact the topology solves that the flight software must also hold. Where ArduPilot can state it, that is named; where the generator cannot, it is a decision (D10). Not emitting a fact is not a guess, but a flight would then run on ArduPilot's defaults.

| solved fact | how ArduPilot can hold it | source at the tag |
|---|---|---|
| each port's role | `define DEFAULT_SERIALn_PROTOCOL SerialProtocol_…`: the console is SERIAL0, the MAVLink protocol by default; `gps` is `SerialProtocol_GPS` (5); `rc_crsf` and `rc_sbus` are `SerialProtocol_RCIN` (23); `mavlink` is MAVLink2 | `AP_SerialManager.cpp:37-90`; `AP_SerialManager.h:39-62` |
| baud | `define DEFAULT_SERIALn_BAUD <baud>`. SERIAL0's default is 115200 (`AP_SerialManager_config.h:78-81`), so `serial-dshot-9600`'s 9600-baud console would run at 115200 unless the define is emitted | `AP_SerialManager.cpp:40-41, 172, 419` |
| data bits, parity, stop bits | no define. ArduPilot derives them from the protocol (8N1, or SBUS's 8E2). The generator can refuse framing other than what the role implies | — |
| inversion | `define DEFAULT_SERIALn_OPTIONS` for n ≥ 1 only. SERIAL0 has none, so an inverted console would be refused | `AP_SerialManager.cpp:53, 281` |
| `negotiated`, `peer_deviation_ppm`, `brr` | ArduPilot computes its own BRR on its own clock; the rest are judge inputs, not configuration | — |
| motor protocol (`pwm`, `dshot300`, `dshot600`) | the `MOT_PWM_TYPE` parameter (DShot600 = 6), with no `hwdef.dat` define. The way to state it is a `defaults.parm` file beside `hwdef.dat` in the same untracked board directory, which configure embeds (CH:2727-2745, 3083) | `AP_MotorsMulticopter.cpp:92-98`; `AP_Motors_Class.h:360-367` |
| PWM rate and pulse range | `MOT_PWM_MIN`/`MAX` and the output rate, parameters only (`AP_MotorsMulticopter.cpp:100-107`; `AP_Motors_Class.h:48`); `defaults.parm`, as above | — |
| `psc`, `arr`, `ccr`, and the clock tree | ArduPilot runs its own PLL clock from HSI (`OSCILLATOR_HZ 0`) and computes its own timer values. The topology's 64 MHz reset clock is our board crate's, not ArduPilot's | — |

### 2.2 The per-pin tokens (what the grown check judges)

The tokens on every emitted pin line, in ArduPilot's own forms (CH:349-431). All were measured in the ELF (§3):
- **Speed:** `SPEED_VERYLOW` for a solved `00`, `SPEED_LOW` for `01`. Every pin left unnamed stays `SPEED_MEDIUM` (`10`), which is ArduPilot's default.
- **Output type:** `OPENDRAIN` on each I2C line, `PUSHPULL` on every other line.
- **Pull:** `FLOATING`, except `PULLDOWN` on a motor output.

### 2.3 The check's expected values: two have a source only by decision

| register | expected value | source |
|---|---|---|
| `MODER`, AF | as today | `[T]` `assignments`, `gpio` |
| `OTYPER` | open-drain exactly on the I2C lines | `[T]`: the keys of `analog_lines`, which holds every solved I2C line (`lib.rs:381-386`) |
| `OSPEEDR` | each pin's solved speed | **decision D2.** The topology holds `spi_lines`, not a speed. If the generator and the check both call `ir::speeds::choose`, the check is consistency, not judgement (`RUL-20`) |
| `PUPDR` | "what the topology implies" | **decision D3.** Only the motor pull-down is a rule in `ir` (`silicon::motor_output_pupd`). The topology declares an external pull-up on each I2C line (`analog_lines[..].pull_up_ohms`) and no pull on any other line. ArduPilot's defaults pull up a CS and each USART line (CH:387-427; `4400a` §2.4 measured them on step G). ArduPilot does this on purpose (CH:393-399, "generate pullups for UARTs"): with `FLOATING`, an RX with nothing plugged in floats |

## 3. Results (measured, Furnace, the pinned image `160f9955…`)

**The setup:**
- each run under `~/bin/capped-run --mem 8G`, `--network none`, read back as `memory.max` 8589934592, `memory.swap.max` 0 and `pids.max` 8192;
- `container/ardupilot-build.sh boards` (a clean build directory, `GITHUB_ACTIONS` unset, `git status --porcelain --untracked-files=no` empty after each build);
- `nproc` 20;
- the boards in `~/.cache/aet-4400b/boards/`;
- the bytes decoded from `arm-none-eabi-objdump -s` of `gpio_default_config` (`~/.cache/aet-4400b/decode.py`).

| board | build | wall | text B | `check pinout` today |
|---|---|---|---|---|
| `s01` | ✔ | 29.8 s | 1,283,664 | exit 0, decodes agree |
| `s02n` (no serial) | ✘ `hal_serial_lld.h:438: #error "SERIAL driver activated but no USART/UART peripheral assigned"` | 2 s | — | — |
| `s02c` (a hand console, USART3 PD8/PD9) | ✔ | 27.6 s | 1,284,124 | exit 1: `reverse UnnamedPin PD8`, `PD9` |
| `s02d` (the console the solver placed, USART1 PA9/PA10, when the fixture declares one) | ✔ | 29 s | — | exit 0 against that topology, 12 pins configured |
| `dshot` | ✔ | 28.9 s | 1,299,548 | exit 0 |

The scope peaked at 3,982,434,304 B (four boards in one container) and 2,774,175,744 B (`s02d` alone).

**The decoded bytes** (ports C and E–K at reset defaults on every board: `MODER` 0, `OSPEEDR 0xaaaaaaaa`, `PUPDR` 0):
- **`s01`:**
  - A: `MODER 0x0028a801`, `OTYPER 0`, `OSPEEDR 0xaa8246a8` (PA5 and PA7 `01`; PA0, PA6, PA9 and PA10 `00`), `PUPDR 0`, `AFRL 0x55500000`, `AFRH 0x00000770`;
  - B: `MODER 0x0000a000`, `OTYPER 0xc0`, `OSPEEDR 0xaaaa0aaa`, `PUPDR 0`.
- **`dshot`:** PA8–PA11 are AF1 with `PUPDR 10` (A `PUPDR 0x00aa0000`) and `OSPEEDR 00`. PB14/PB15 are USART1 AF4, push-pull, no pull.
- **`s02c` / `s02d`:** PA1 is output; PA12, PB14 and PB15 are AF5 at `00`; SPI1 SCK and MOSI are at `01`.

**`hwdef.h` holds** [measured]:
- `STM32_HSECLK 0U`, `HAL_I2C_MAX_CLOCK 400000`, `STM32_ST_USE_TIMER 5`;
- `BOARD_FLASH_SIZE 2048`, `STORAGE_FLASH_PAGE 14`, `HAL_STORAGE_SIZE 16384`;
- `FLASH_LOAD_ADDRESS 0x08000000`, `HAL_ENABLE_SAVE_PERSISTENT_PARAMS 0`, `APJ_BOARD_ID 24400`;
- `HAL_SPI_DEVICE0 SPIDesc("dataflash", 0, 1, PAL_LINE(GPIOA,0U), SPIDEV_MODE0, 16*MHZ, 16*MHZ)`, and on S02 `HAL_SPI_DEVICE1 … "icm42688" … 2*MHZ`;
- `HAL_BARO_PROBE1 … AP_Baro_BMP280::probe, 0, 118` and, on S02, `HAL_BARO_PROBE2 … AP_Baro_BMP581::probe, 0, 70`;
- `HAL_PWM_COUNT 4` on `dshot`.

**Results beside predictions:**

| # | prediction | result |
|---|---|---|
| 1 | `s01` builds; flash within 2 % of step G; the five defines | ✔ 1,283,664 B text (−0.1 %); every define as predicted |
| 2 | `s01`'s A `MODER 0x0028a801`; speeds `01`/`00` as emitted, `10` elsewhere; B `OTYPER 0xc0`; `PUPDR` 0 on every port | ✔ exactly |
| 3 | `s02n` fails with the serial `#error` | ✔ the same line, `hal_serial_lld.h:438` |
| 4 | `s02c` builds, both `BARO`s and both `SPIDEV`s accepted | ✔ |
| 5 | `dshot` builds, pull-down and `00` on PA8–11, no DMA line needed | ✔ |
| 6 | 50–90 s and under 2 GB per board | ✘ 28–30 s at `-j20`; up to 3.98 GB (four boards, one scope) and 2.77 GB (one) |
| 7 | `check pinout`: `s01` and `dshot` pass; `s02c` names PD8 and PD9 | ✔ |
| 8 | one required `[N]`, the S02 serial port; speed and pull by decision | ✔ and found also: `SCALE` for sense pins is `[N]` (not needed by either board) |

**Measured after the first draft of this record, before any push:**
- **`solve` and `check speeds` over all 23 specifications** (`examples/*.json` and `crates/aethelgard-cli/tests/fixtures/*.json`):
  - 15 solve and pass `check speeds`;
  - 8 are refused at `solve` (`invalid-zero-domain`, and 7 files at an older specification version) and refused alike by `check speeds`;
  - none solves and is then refused by `check speeds`.
- **D1(a), the console in `sentinel-02-own.json`.** I added `examples/sentinel-01-minimal.json`'s `CONSOLE` port and its `J1` connector (as `J3`) to the fixture, ran the tier-1 tests that read it or sweep every fixture, then restored it, with `git diff` empty after. All passed:
  - `own_models_tier1` 12/12;
  - `ledger_tier1` 3/3;
  - `fab_tier1` 66/66;
  - `solve_cli` 11/11;
  - `silicon_rules` 1/1;
  - `campaign_tier1` 18/18;
  - svd-hal `silicon_words` 6/6.

  The tier-2 sweeps (`board_tier2`, `with_tools`, `tools_tier1`, `verify_47k_tier2`) were not run. The fixture stays `UnknownPart` at `generate` either way [inferred: the console adds no part].
- **`SPIDevice.cpp:259-276` [read].** ArduPilot starts from the bus's kernel clock ÷ 2 and halves it until it is at or below the speed given, **at most 7 times**. So a requested speed below the kernel ÷ 256 runs faster than requested. On a 200 MHz kernel that is anything under 390 kHz [inferred: the kernel value was not read], far below both boards (16 MHz and 2 MHz).

**Corrections to §0** (it was committed with the predictions and is not edited):
- `Role::policy` is at `board.rs:826-834`, not 823-831.
- `board_types.txt` also says "do not allocate board IDs above 19,999 in this file" (line 619). §0's reading came from a listing cut off at line 576. 24400 holds.

## 4. STOP (§1.1): the Sentinel-02-shaped fixture has no serial port

`sentinel-02-own.json` declares no serial port. Every line that would put a USART in its `hwdef.dat` therefore has no source, and ArduPilot does not build without one:
- **[measured]** at this tag: `s02n`;
- the spike's steps B to E, at the same tag.

The work order's §1.1 says to stop here. Nothing has been built past the measurement: no schema bump, no parts column, no generator, no check. Those wait for the ruling. The proposals below are what I will build once it is given.

## 5. Decisions for the architect

- **D1, the Sentinel-02-shaped board's serial port.** Measured: with a console declared, plus the connector its pins must end at (`AET-WO-4300b` ruling 1: S01's `J1`, `SM04B-GHS-TB`), the solver places it on USART1 PA9/PA10, and that board builds and passes the check (`s02d`). Options:
  - **(a)** declare the console and its connector in `sentinel-02-own.json`. Measured (§3): every tier-1 test that reads the fixture or sweeps all fixtures still passes.
  - **(b)** a new fixture, `sentinel-02-hwdef.json`: `sentinel-02-own.json` plus the console and its connector, used only by the generated builds.
  - **(c)** the generator emits a fixed console. This is what `4400` D2 declined (`INV-06`).

  **Recommended: (a).** It keeps one Sentinel-02-shaped file, a flight controller needs a console in any case, and no tier-1 test moves. The fixture is in `crates/aethelgard-cli/tests/fixtures/`, not in `3401`'s `examples/`. Whichever is ruled, the generator refuses by name a topology with no serial port, and one with ports but no `console` role (`serial-four.json`).
- **D2, the per-pin speed the check judges** (`RUL-20`). Options:
  - **(a)** `solve` writes each emitted pin's `OSPEEDR` into the topology (`pin_speeds`, keyed by net, beside `mcu`, solved schema 9), through `ir::speeds::choose` for SCK and MOSI and `00` for every other role. Then the generator and the check only read it, and `check speeds` judges the stored value.
  - **(b)** the generator takes svd-hal's `pin_speeds` and the check calls `ir::speeds::choose` itself. Both reach the same function, so the check is consistency.

  **Recommended: (a).** Measured cost: no committed specification that solves today is refused by `check speeds` (all 23 files, §3), so moving the choice into `solve` refuses no committed fixture. In-code tests that build a refusing line would move from `check speeds`' refusal to `solve`'s. `svd-hal`'s own board crate keeps its computation, and a test can hold the two equal. svd-hal is outside this fence: I would add the test, not change svd-hal.
- **D3, `PUPDR`, what the topology implies.** Options:
  - **(a)** no internal pull on every topology pin, except a motor output pulled down by `ir::silicon::motor_output_pupd`. This is our own board crate's policy (`Role::policy`), so both firmwares from one topology agree. The generator emits `FLOATING` over ArduPilot's CS and USART pull-ups. The check's rule is a new small function in `ir` (no internal pull unless the topology states one) that the generator does not call. Measured: ArduPilot accepts the overrides, and `PUPDR` is 0 except PA8–11.
  - **(b)** keep ArduPilot's defaults and judge only the I2C and motor lines.

  **Recommended: (a).** Its cost is that ArduPilot's pull-ups are deliberate (CH:393-399): with `FLOATING`, an RX or a CS with nothing driving it floats. This is the same as our own board crate today. A pull the board needs belongs in the specification as a declared fact, not in either generator's default.
- **D4, where `FLASH_SIZE_KB` and the ArduPilot MCU names live.** The MCU's row is in `power-parts.json` (schema 3), not `parts.json`. I propose two columns on that row, `power-parts.json` schema 4:
  - `flash_kilobytes` (2048, DS12117 Table 2 p. 20);
  - `ardupilot_mcu` (`["STM32H7xx", "STM32H753xx"]`, cited to CH:2894-2897 and the table file `STM32H753xx.py`).

  `parts.json` (schema 7) takes, per bus part, an optional `ardupilot` column: `{"kind": "baro"|"imu"|"spidev", "driver", "spidev", "id", "at"}`. Its `id` is the chip ID the driver accepts, cited at the tag, and a test refuses a row whose `id` differs from the part's own `identity.value`, which closes the TMP117 case by data. The work order names only a `parts.json` bump; this is a second table.
- **D5, `STORAGE_FLASH_PAGE`.** The work order says "the last sector". ArduPilot's flash storage uses pages `p` and `p+1` (`Storage.cpp:331-336, 427`), so `p` = 15 runs past the flash. I propose `p` = sectors − 2 (14 on the H753, the last two sectors), as MatekH743 does (`hwdef.dat:178-180`), and measured (§3).
- **D6, the `SPIDEV` speeds.** Low = high = `spi_controllers[..].achieved_hz`, rounded down to a whole kHz (ArduPilot accepts only `*MHZ` or `*KHZ`, CH:1473-1477). It is not `declared_hz`: the pin speed was judged at `achieved_max_hz`, and ArduPilot divides down from the value given (`SPIDevice.cpp:259-276` [read], §3), so a value at `declared_hz` (20 MHz on SPI1) could clock past the speed the check judged. The generator refuses a speed below its bus kernel ÷ 256, where ArduPilot's divider runs out; the kernel is ArduPilot's, so that bound needs a cited value first.
- **D7, paths neither CI board reaches:**
  - **Motors:** I propose `serial-dshot-9600.json` as a third generated CI board, measured here (PWM, pull-down, the system timer).
  - **ADC sense:** `SCALE` has no source, so the generator refuses a topology with `power.sense` by name until one exists (a later work order).
  - **`BIDIR`:** nothing declares it, so it is never emitted.
- **D8, the generator's crate.** I propose a new crate `crates/aethelgard-hwdef`, depending on `aethelgard-ir` alone, with `aethelgard generate hwdef --spec <spec> --out <file>`; the CLI owns the file write. `pincheck`'s separation test already refuses a generator dependency, and I add the reverse. A tenth member edits `workspace_shape` and architecture §3's table. The table is protected, so that row would be the architect's.

  The alternative is a module in `aethelgard-svd-hal` beside the board crate's generator, which D2(b) would need.
- **D10, the solved facts `hwdef.dat` does not carry** (§2.1a). Options:
  - **(a)** emit what `hwdef.dat` can state now: `define DEFAULT_SERIALn_BAUD` and `DEFAULT_SERIALn_PROTOCOL` for each port, and `DEFAULT_SERIALn_OPTIONS` for an inverted port at n ≥ 1. Refuse an inverted console, and framing other than what the role implies. Leave the motor protocol and PWM range to the work order that writes `defaults.parm`, named in the header as not emitted.
  - **(b)** also write `defaults.parm` now (`MOT_PWM_TYPE`, `MOT_PWM_MIN`/`MAX`). That is a second emitted file, and its judge would be ArduPilot at run time (`4400c`).
  - **(c)** emit none of it, and name every fact in the header.

  **Recommended: (a).** It stays one file, and with no `DEFAULT_SERIAL0_BAUD` line the 9600-baud console of `serial-dshot-9600` would run at 115200. These are not `gpio_default_config` bytes, so `check pinout` cannot judge them. A judge would read `hwdef.h`'s defines, which the check already reads as a cross-check, against the topology.
- **D9, versions.** Solved schema 9 (`mcu`, and `pin_speeds` under D2(a)) overlaps `1312`'s `ir` work (specification 13). `power-parts.json` 4 and `parts.json` 7 may overlap `4220b`. I have put notes in both inboxes.

## 6. Process

- No permission was denied. One container ran at a time.
- **The ArduPilot sources** were copied read-only out of the pinned image into a scratch tree, and are reproduced from the digest by `tar` of the paths cited.
- **Disk (kickoff item 8).** The scratch tree is removed before this commit is pushed: the ArduPilot extract, the five boards, their ELFs and logs, the solved topologies, the build runner and the decoder. Only `~/.cache/aet-4400b/pr-title-body.md` stays, for the architect. The evidence is in this file:
  - **The build** was `container/ardupilot-build.sh boards <name>`, run in the pinned image under `capped-run` with `/boards` and `/out` mounted.
  - **The decode** found `gpio_default_config` with `arm-none-eabi-nm -S` and dumped its 308 bytes with `arm-none-eabi-objdump -s --start-address --stop-address`. A short Python reader then took 11 ports × 7 little-endian words in `gpio_default_config`'s order (`MODER OTYPER OSPEEDR PUPDR ODR AFRL AFRH`), as `aethelgard-pincheck`'s `elf.rs` does.
- **`main`** has not moved since `36e428b`, so there was nothing to merge. No test or workflow reads `docs/work-orders/` [measured, grep of `crates/*/tests`, `scripts` and `.github`].

**`s01.hwdef.dat`, verbatim** (the first line was a comment naming it a measurement):

```
MCU STM32H7xx STM32H753xx
OSCILLATOR_HZ 0
FLASH_SIZE_KB 2048
APJ_BOARD_ID 24400
define AP_BOOTLOADER_FLASHING_ENABLED 0
FLASH_RESERVE_START_KB 0
define HAL_STORAGE_SIZE 16384
STORAGE_FLASH_PAGE 14
STM32_ST_USE_TIMER 5
SERIAL_ORDER USART1
PA9 USART1_TX USART1 SPEED_VERYLOW PUSHPULL FLOATING
PA10 USART1_RX USART1 SPEED_VERYLOW PUSHPULL FLOATING
I2C_ORDER I2C1
define HAL_I2C_MAX_CLOCK 400000
PB6 I2C1_SCL I2C1 SPEED_VERYLOW OPENDRAIN FLOATING
PB7 I2C1_SDA I2C1 SPEED_VERYLOW OPENDRAIN FLOATING
PA5 SPI1_SCK SPI1 SPEED_LOW PUSHPULL FLOATING
PA6 SPI1_MISO SPI1 SPEED_VERYLOW PUSHPULL FLOATING
PA7 SPI1_MOSI SPI1 SPEED_LOW PUSHPULL FLOATING
PA0 FLASH_CS CS SPEED_VERYLOW PUSHPULL FLOATING
SPIDEV dataflash SPI1 DEVID1 FLASH_CS MODE0 16*MHZ 16*MHZ
define HAL_LOGGING_DATAFLASH_ENABLED 1
BARO BMP280 I2C:0:0x76
define HAL_USE_ADC FALSE
```

**The other boards, by their differences from `s01`:**
- **`s02n`** drops `SERIAL_ORDER` and the two USART1 lines, and adds:
  ```
  PA12 SPI2_SCK SPI2 SPEED_VERYLOW PUSHPULL FLOATING
  PB14 SPI2_MISO SPI2 SPEED_VERYLOW PUSHPULL FLOATING
  PB15 SPI2_MOSI SPI2 SPEED_VERYLOW PUSHPULL FLOATING
  PA1 IMU_CS CS SPEED_VERYLOW PUSHPULL FLOATING
  SPIDEV icm42688 SPI2 DEVID1 IMU_CS MODE0 2*MHZ 2*MHZ
  BARO BMP581 I2C:0:0x46
  ```
- **`s02c`** is `s02n` plus:
  ```
  SERIAL_ORDER USART3
  PD8 USART3_TX USART3 SPEED_VERYLOW PUSHPULL FLOATING
  PD9 USART3_RX USART3 SPEED_VERYLOW PUSHPULL FLOATING
  ```
- **`s02d`** is `s02n` plus `s01`'s three USART1 lines.
- **`dshot`** is `s01` with USART1 on PB14/PB15 in place of PA9/PA10, plus:
  ```
  PA8 TIM1_CH1 TIM1 PWM(1) SPEED_VERYLOW PUSHPULL PULLDOWN
  PA9 TIM1_CH2 TIM1 PWM(2) SPEED_VERYLOW PUSHPULL PULLDOWN
  PA10 TIM1_CH3 TIM1 PWM(3) SPEED_VERYLOW PUSHPULL PULLDOWN
  PA11 TIM1_CH4 TIM1 PWM(4) SPEED_VERYLOW PUSHPULL PULLDOWN
  ```
- **The solved topology for `s02d`** came from `sentinel-02-own.json` with `examples/sentinel-01-minimal.json`'s `serial_ports` and its `J1` connector (renamed `J3`) added.
