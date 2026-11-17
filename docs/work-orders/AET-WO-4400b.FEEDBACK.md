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

## 7. The build, after the rulings (PR #189, 01:45 UTC)

Every recommendation of §5 was ruled accepted (D1(a), D2(a), D3(a), D4, D5, D6, D7, D8, D10(a); D9 coordinated). D6 carried a condition: stop if ArduPilot's SPI kernel cannot be cited at the tag. It can.

### 7.1 D6: ArduPilot's SPI kernel, cited [read at source]

With `OSCILLATOR_HZ 0`, `stm32h7_mcuconf.h` (sha256 `67f48e5f…a426`) selects HSI as the PLL source (lines 113-121) and sets SPI123SEL to PLL1_Q and SPI45SEL and SPI6SEL to PLL2_Q (lines 332, 333, 345).

ChibiOS `hal_lld_type1.h` at `d103983f` (sha256 `d6dd1160…eee3`; the H753 takes type 1, `hal_lld.h:147-158`) defines HSI as 64 MHz (lines 244, 1518) and the PLL formulas (1546, 1561, 1592, 1801, 1822, 1917, 1934). That gives:
- **SPI1–3:** PLL1_Q = 64 MHz / 4 × 50 / 10 = **80 MHz** (`mcuconf` 117, 170, 172; `hal_lld_type1.h` 2593);
- **SPI4–6:** PLL2_Q = 64 MHz / 8 × 45 / 5 = **72 MHz** (`mcuconf` 118, 175, 177; `hal_lld_type1.h` 2636, 2661).

`SPIDevice.cpp:259-276` starts from the kernel / 2 and halves at most seven times, so the slowest rate is the kernel / 256: 312.5 kHz on SPI1–3 and 281.25 kHz on SPI4–6. These are rows of `power-parts.json`'s MCU row, each cited by document sha256.

Our solver reaches 250 kHz on HSI (64 MHz / 256), so a bus declared between 253.7 kHz and 312.5 kHz solves, and the generator refuses it (`SpiBelowArduPilotDivider`). This is measured in `hwdef_tier1.rs`: a bus at 300 kHz is solved to 250 kHz and refused by name.

### 7.2 What was built, by ruling

| ruling | built | where |
|---|---|---|
| D1(a) | `sentinel-02-own.json` declares the console. **Built on J2's spare pins 2 and 3, not on a new J3:** after `main`'s 4220b merge (#192), a new J3 moved the placed board so that `IMU_MOSI`'s extract bound became 31,846 fF, over its declared 30,000 fF (`own_board_tier1`, measured). On J2 every test passes. The solver puts the console on USART1 PA9/PA10 either way | the fixture |
| D2(a) | solved schema 9: `mcu`, and `pin_speeds` keyed by net, `0` or `1`, chosen in `solve` (`solve::pin_speed`, the same `choose` call svd-hal makes). `validate` refuses a stray or missing net, or a value above 1 (`PinSpeedMismatch`). `check speeds` judges the stored value. `derive_controllers()` also re-derives `pin_speeds` for hand-built topologies, leaving out a pin no speed serves. The equality test is `hwdef_tier1.rs::the_board_crate_s_speeds_equal_the_stored_ones`, over every committed specification that solves (at least 12) | `ir` lib.rs, solve.rs; CLI speeds.rs |
| D3(a) | `ir::silicon::implied_pupd` (the motor pull-down, else none) and `implied_open_drain` (the keys of `analog_lines`). The check calls them; the generator emits `FLOATING`/`PULLDOWN` and `OPENDRAIN` by its own rule from the signal name. A source test holds that the generator names neither function, nor `motor_output_pupd`, `speeds::` or `.choose(` | `ir` silicon.rs; hwdef tests |
| D4 | `parts.json` 8 (7 when written; renumbered behind #193): a required-but-nullable `ardupilot` column (`kind` baro, imu or dataflash; `driver`; `spidev`; `id`; cited by the ArduPilot source file's sha256). A row whose `id` is not the part's `identity.value` is refused ("ArduPilot's driver would refuse the chip"). `power-parts.json` 4: the MCU row's `flash_kilobytes` and `ardupilot` (series, mcu, oscillator_hz, spi_kernel_hz, each cited) | `ir` data, parts.rs, power_parts.rs |
| D5 | `STORAGE_FLASH_PAGE` = flash / 128 − 2 = 14 | generator |
| D6 | SPIDEV low = high = `achieved_hz` rounded down to a whole kHz; refused below the cited kernel / 256 | generator |
| D7 | `serial-dshot-9600` is the third CI board; `power.sense` is refused (`SenseNotEmitted`); `BIDIR` is never emitted | generator; CI |
| D8 | the crate `aethelgard-hwdef` (11th member, `aethelgard-ir` alone), `aethelgard generate hwdef --topology --out`. The separation test checks both directions | crate; CLI hwdef.rs |
| D10(a) | `DEFAULT_SERIALn_PROTOCOL` and `_BAUD`, and `_OPTIONS` for n ≥ 1 (`3` on an inverted port, `UARTDriver.h:90-91`). Refused: an inverted console, framing other than the role's (8N1 for console, MAVLink, GPS and CRSF; 100,000 8E2 inverted for SBUS, `AP_RCProtocol.cpp:339-352`), a console baud SERIAL0's parameter cannot hold (`map_baudrate`, `AP_SerialManager.cpp:737-770`), and a SERIALn baud ≤ 2000. The motor protocol and PWM range are named in the header as not emitted | generator |

### 7.3 CI (`ardupilot-image.yml`)

- **A new `generated` matrix, one job per board:** `sentinel-01`, `sentinel-02` and `serial-dshot-9600`. Each job:
  - solves the specification;
  - runs `generate hwdef`, and `cmp`s the file against the committed golden;
  - builds it with unmodified ArduPilot (`ardupilot-build.sh boards`: `GITHUB_ACTIONS` unset, the tree clean, `--network none`);
  - uploads the ELF, `hwdef.h`, the topology and the file.
- **`pinout` now needs `generated`.**
  - **The hand boards:** step G and each seed keep their own findings. Each now also fails on ArduPilot's default speeds (`10`) and on the pull-ups on the CS and the USART. seed-pin adds `WrongOutputType PB6`, an I2C pin left as a push-pull input.
  - **Each generated board** passes against its own topology with no allow-list.
  - **A define witness was built, then removed.** It required every `define` and `STM32_` line to appear verbatim in `hwdef.h`. The adversarial pass showed it cannot fail: ArduPilot writes those lines through verbatim (`chibios_hwdef.py:1004-1010`), so it compared the artifact with itself (`INV-07`; §11, A3).
- **Forced by the new member:** `ci.yml`'s package list.

## 8. Measured: the generated boards in unmodified ArduPilot

Each run used Furnace, the pinned image `160f9955…`, `capped-run --mem 8G` (read back `memory.max` 8589934592, `memory.swap.max` 0, `pids.max` 8192), `--network none`, `ardupilot-build.sh boards` with a clean tree after each build, and `-j20`.

| board (generated) | build | wall | `check pinout` (grown) |
|---|---|---|---|
| sentinel-01 | ✔ | 26 s | exit 0; 8 pins configured; decodes agree |
| sentinel-02 (before main's 4220b merge: IMU_SCK on PA12) | ✔ | 23 s | exit 0; 12 pins |
| sentinel-02 (after the merge: IMU_SCK on PB13, the console on J2) | ✔ | 22 s | exit 0 |
| serial-dshot-9600 | ✔ | 23 s | exit 0; 12 pins; PA8–PA11 pulled down |
| serial-four with its MAVLink port made the console (four ports, SBUS inverted) | ✔ | 21 s | exit 0; `DEFAULT_SERIAL3_OPTIONS 3` in `hwdef.h` |

- **What reached `hwdef.h`** [measured]: `DEFAULT_SERIAL0_PROTOCOL SerialProtocol_MAVLink2`, `DEFAULT_SERIAL0_BAUD 115200` (9600 on dshot), `HAL_SPI_DEVICE0 SPIDesc("dataflash", 0, 1, PAL_LINE(GPIOA,0U), SPIDEV_MODE0, 16*MHZ, 16*MHZ)`, `HAL_SPI_DEVICE1 … "icm42688" … 2*MHZ`, and both `HAL_BARO_PROBE`s. The scope peaked at 3.9 GB for three boards in one container.
- **Step G and the seeds, rebuilt** [measured]: each fails with exactly the findings `container/ardupilot-pinout.sh` now expects, including `WrongSpeed` on all eight topology pins and `WrongPull` on PA0, PA9 and PA10.

## 9. §2: mutations of the generator, each judged on the rebuilt ELF [measured]

Each mutant edited `crates/aethelgard-hwdef/src/lib.rs`. The mutant then rebuilt the CLI, regenerated the file and built it in the pinned image. `check pinout` judged the ELF against the unmutated topology, and the source was restored, leaving `git status` clean. The runner is `~/.cache/aet-4400b/mut/run.py`, removed at the close; its mutants are listed here.

| mutant | what it emits | caught by | verdict |
|---|---|---|---|
| M1, a swapped pin: USART1 TX and RX exchange pins | `PA10 USART1_TX USART1 …`, `PA9 USART1_RX …` | **ArduPilot's AF table**, at configure | `Error: Unknown pin function PA10:USART1_TX for MCU STM32H753xx`; no ELF |
| M1b, a moved pin: FLASH_CS on PA2 | `PA2 FLASH_CS CS …` | **the generator**, before ArduPilot (`PinNotEmitted FLASH_CS`); with that guard disabled, **`check pinout`** | exit 1: `NotOutput PA0`, `UnnamedPin PA2`, `WrongSpeed PA0` |
| M1c, swapped chip selects (S02: FLASH_CS on PA1, IMU_CS on PA0) | two `CS` lines exchanged | **nothing** | builds; `check pinout` exit 0. **A blind spot:** both pins are push-pull outputs at the same speed and pull, so `gpio_default_config` cannot tell which select is which. The device's CS lives only in ArduPilot's SPI table (`HAL_SPI_DEVICEn … PAL_LINE(GPIOA,1U)`), which the check does not read. See §11, F1 |
| M2, a wrong speed: the token map inverted | `SPEED_LOW` for `00`, `SPEED_VERYLOW` for `01` | **`check pinout`** | exit 1: `WrongSpeed` on all eight pins |
| M3, a dropped serial port on a one-port board (Sentinel-01) | no USART1 pin lines | **the generator** (`PinNotEmitted CONSOLE_RX`); with the guard disabled, **ArduPilot's build** (`#error "SERIAL driver activated but no USART/UART peripheral assigned"`) | no ELF |
| M3b, the same on four ports, pins only (`SERIAL_ORDER` kept) | three ports in the order with no pins | **ArduPilot's compile** | `hwdef.h:232: 'SD2' was not declared` |
| M3c, a whole port dropped (four ports, SBUS gone from the order, its pins and its defines) | three ports | **`check pinout`** | exit 1: `NotAlternate PA0 SBUS_TX`, `NotAlternate PA1 SBUS_RX`, `WrongSpeed` on both |
| M4, a motor timer as the system timer (hand edit, which the generator never emits: `STM32_ST_USE_TIMER 1` on dshot, whose motors are on TIM1) | — | **ChibiOS**, at compile | `hal_st_lld.h:155: #error "ST requires TIM1 but the timer is already used"` |
| M5, I2C emitted push-pull | `PB6 I2C1_SCL I2C1 … PUSHPULL FLOATING` | **`check pinout`** | exit 1: `WrongOutputType PB6`, `WrongOutputType PB7` |
| M6, the serial pull tokens dropped | no `FLOATING` on PA9 or PA10 | **`check pinout`** | exit 1: `WrongPull PA9`, `WrongPull PA10` (ArduPilot's own pull-ups) |

## 10. Edits outside the literal fence, each forced

- **`crates/aethelgard-twin/src/tests.rs`, `tests/spi_platform.rs`; `crates/aethelgard-cli/src/analog.rs`, `src/renode.rs` (tests):** each holds a schema-8 topology literal. Each was migrated to schema 9 by a script that refused any SPI line it could not classify. `pre_serial` drops the console's stored speeds with its pins.
- **`crates/aethelgard-svd-hal`** (`board.rs` tests, `tests/motor_outputs.rs`, `tests/serial_ports.rs`, `tests/ui/motors-pwm/topology.json`): the test builders add a pin without a stored speed, which `validate` now refuses. Each now inserts `0` or calls `derive_controllers`. No svd-hal code changed.
- **`crates/aethelgard-cli/src/mcp.rs`:** `leaves()`. With `generate` gaining a subcommand, the MCP list lost its `generate` tool (`the_tools_come_from_the_tree`, measured). A command with its own arguments is now a tool even with subcommands, so the list has 19 tools, with `generate_hwdef`.
- **`crates/aethelgard-cli/tests/strict_fence.rs`, `workspace_shape.rs`, `package_licences.rs`, `.github/workflows/ci.yml` (the package list), `Cargo.toml`:** each was forced by the new member or the new module.
- **`crates/aethelgard-cli/tests/solve_cli.rs`:** the two pinned digests. Each move is proved in its comment: with the version put back and the two keys removed, the output hashes to the old digest, exactly.

## 11. Adversarial pass (a fresh agent, Opus: judgement, not volume)

The agent worked read-only against this worktree, with a pinned copy of the CLI that regenerated all three goldens byte for byte, and built in the pinned image, one container at a time. I re-ran or read each high finding before adopting it: A1 (the motor-swap verdict and its `HAL_PWM_GROUP1` map), A2 (my M1c), and A4 and A5 at `AP_SerialManager.cpp`. Each repro below is one line, because the scratch files are deleted at the close.

| # | finding | evidence | caught by | disposition |
|---|---|---|---|---|
| A1 | **Swapped motor outputs pass.** `serial-dshot-9600.hwdef.dat` with `PWM(1)` and `PWM(2)` exchanged on PA8 and PA9 builds; `hwdef.h`'s `HAL_PWM_GROUP1` channel map becomes `{1, 0, 2, 3}`; `check pinout` exits 0 | measured | nothing | **F1**, the stop (§14) |
| A2 | **Swapped chip selects pass** (my M1c, re-run by the agent) | measured | nothing | **F1** |
| A3 | **Every non-GPIO line has no independent judge, and the CI define witness could not fail.** Each of these one-line edits to the four-console board at 100 kHz builds and gets exit 0 from `check pinout`: `SERIAL_ORDER USART1 USART3 USART2 UART4`; `HAL_I2C_MAX_CLOCK 400000`; `BARO BMP280 I2C:0:0x77`; `SPIDEV … MODE3 40*MHZ 40*MHZ`; `DEFAULT_SERIAL0_BAUD 921600`. ArduPilot writes every `define` and `STM32_` line through verbatim (`chibios_hwdef.py:1004-1010`), so the witness compared the file with itself | measured; read | nothing (the golden `cmp` is consistency) | the witness is **removed** (`INV-07`); the judge is **F1** |
| A4 | **`DEFAULT_SERIAL0_PROTOCOL` is never read**: only its `#ifndef` and the `#error` guard (`AP_SerialManager.cpp:37-44`); the parameter default is a literal (line 180). An undefined symbol there builds | measured | nothing | **fixed**: not emitted (a deviation from D10(a)'s letter, for §5's fifth case). The keyword test now holds exact names: SERIAL0 `_BAUD` only; SERIAL1–9 `_PROTOCOL`, `_BAUD` and `_OPTIONS`, each read in an `AP_GROUPINFO` (lines 191-395, checked for 0–3 and 9). Mutant: putting it back fails the goldens |
| A5 | **Two RC-input ports**: CRSF and SBUS both became `SerialProtocol_RCIN`, and ArduPilot drops the second ("duplicate RCIN not permitted", `AP_SerialManager.cpp:536-541`) | read; generate measured | nothing | **fixed**: `RcInputTwice`, tested on a solved specification (`hwdef_tier1.rs`); mutant: removing the guard fails it |
| A6 | **On an RC port the emitted `BAUD` and `OPTIONS` are not what runs.** The RCIN branch never calls `begin(baud)`. `AP_RCProtocol` cycles its own configurations and sets or clears RXINV itself (`AP_RCProtocol.cpp:326-352`), so a CRSF port declared at 420000 runs at 416666 | read | — | **F3** (§14) |
| A7 | **Stored speeds are never re-derived.** A topology with `SPI_SCK` and `SPI_MOSI` stored as `0` validates, generates `SPEED_VERYLOW` on a 16 MHz SCK, and passes `check pinout`. The same ELF against the solved topology fails with `WrongSpeed` | measured | nothing, unless the topology is solved fresh (CI solves it fresh) | **F4** (§14) |
| A8 | **A wrong driver can sit in the parts table.** The `id` check ties a row to itself, not to the driver: `{kind: baro, driver: DPS280, id: "0x60"}` on the BME280 would parse. Downstream, `BARO DPS280 I2C:0:0x76` builds and passes `check pinout` | read; downstream measured | the citation only | **noted**: the `parts.rs` comment no longer claims more. §5's D4 said this check "closes the TMP117 case by data"; it closes only a row that writes the driver's true ID |
| A9 | **A CS net named with an ArduPilot AF prefix** (`SPI_FLASH_CS`, `TIM2_CH1`) generates, and ArduPilot then refuses it at configure (`chibios_hwdef.py:33, 173-178`) | measured | ArduPilot | **backlog**: refused, but not by name before the build |
| A10 | **Two fallbacks in the generator:** `unwrap_or(100_000)` (I2C clock) and `map_or(0, …)` (SPI kernel) | read | — | **fixed**: the I2C clock is an `expect` on what `validate` guarantees; the SPI kernel is `SpiKernelNotCited` |

**What held:**
- **Case 2:** two buses at 400 and 100 kHz give `HAL_I2C_MAX_CLOCK 100000`, measured. On ArduPilot's own clock no bus runs faster than declared (`I2CDevice.cpp:209-225`; PLL3_R = 30 MHz) [read, inferred].
- **Case 3:** a motor timer as the system tick is refused by ChibiOS (M4). The generator moves the tick to TIM2 when a motor takes TIM5, and that builds and passes [measured].
- **Moved pins and AF-swapped pins** (M1, M1b).
- **Arithmetic and constants:** the SPIDEV speed against ArduPilot's divider, the console baud map, `APJ_BOARD_ID` 24400.
- **Input and refusals:** the strict topology reader (duplicate keys and unknown fields), and the named refusals.
- **`RUL-20`:** the check and the generator call none of each other's rules. Their one shared input is the stored `pin_speeds` (A7).

## 12. Reduction pass (diff)

**Reduction pass (diff): −19 net (hwdef.rs −13, lib.rs −12, pinout.rs +6; counted at the edits) · 1 duplicate removed · 1 dead path removed · 1 protected kept · 2 findings below.**
- **Removed duplicate:** `generate hwdef` repeated `check pinout`'s topology parse and its exit mapping. It now calls `pinout::parse_topology`.
- **Removed dead path:** `SpiSpeedTooLow` (below 1 kHz) could not fire, because every such rate is below the cited kernel ÷ 256 (at least 281.25 kHz), which `SpiBelowArduPilotDivider` refuses first.
- **Protected:** `PinNotEmitted`, labelled `belt-and-suspenders`. Mutants M1b and M3 show it firing before ArduPilot.
- **Findings:**
  - `solve::pin_speeds` and `derive_controllers` both walk `solved_pins` through `pin_speed`. One refuses and the other leaves a pin out, as `derive_controllers` already does for controllers. That is two error policies, not one fact twice, so both are kept.
  - The generator's I2C/SPI/serial family test and svd-hal's `Role` both read a signal's name; they are separate crates by `RUL-20`.

**Proof:** the full suite passes after each removal. The mutants of the guards touched (RCIN, SERIAL0's protocol) each still fail their test.

## 13. Gates and CI

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`: all pass on Furnace (123 test binaries) at the commit before the signal.
- `main` (`7452bf4`, 1801's 0.2.0) is merged in: the crate is 0.2.0 in `workspace.dependencies` and step 10 of `docs/releasing.md`'s order, as ruled. `TIER2_MIN` follows main (161, then 162 with 1312): this work order adds no tier-2 test.
- **CI on `e26d7a47` is green in all four workflows:**
  - `ci`: `gates`, `shards` and all eight `tools-shard`s. Tier 2 includes the Sentinel-02 board with the console on J2, and the aggregate holds `TIER2_MIN` 161.
  - `ardupilot-image`: step G and the seeds; `generated (sentinel-01)`, `(sentinel-02)` and `(serial-dshot-9600)`, each built under its own name; `pinout`; `source`; `source-rehearsal`.
  - `tools-image`.
  - `engine-image`.
- **Disk:** every scratch tree of mine is removed, except `~/.cache/aet-4400b/pr-title-body.md`.

### 13.1 CI on `54bb000d`, and two fixes

The architect read runs 133801515696 (`generated (sentinel-02)`) and 133801515678 (`generated (sentinel-01)`).
- **Each failed after ArduPilot built** (1,288,364 B and 1,287,908 B), on `cp: cannot create regular file …/boards/<board>/topology.json: Permission denied`. The container, running as root, had made the output directory. **Fixed:** the step makes the directory and copies the topology and the file as the runner's user before the container writes the ELF and `hwdef.h` into it. No chmod.
- **Both boards built in `build/Sentinel01`.** The name came from neither `project_name` nor the generator: `container/ardupilot-build.sh` (from `4400a`) installed every board as the fixed untracked board `Sentinel01`. Builds in one image ran one after another, each from a clean build directory, so none overwrote another's output, but they shared one name. **Fixed:** `boards NAME` installs and builds each board as `NAME` (letters, digits, `-` and `_`, checked). Only the D4 control keeps `Sentinel01`, for step G.
- **Measured on Furnace:** `sentinel-01`, `sentinel-02` and `step-g` built in one container (`Setting board to : sentinel-02`; 28, 21 and 21 s). Both generated boards pass `check pinout`.

### 13.2 `main` again: 4220b's floor (#193)

#193 took `parts.json` schema 7 (`pin_capacitance.typical_femtofarads`). My column is renumbered to **8**; every row carries 4220b's field, and the table's sha256 pin is recomputed. The full suite passes (123 binaries).

## 14. STOPPED: §2's third bullet fails for swaps of one kind, and closing it is outside §1.3

**Why it stopped.** §2 requires a generator mutation of "a swapped pin" to be caught by `check pinout` on the rebuilt ELF. Measured:
- **Where the swap crosses an AF,** ArduPilot's own AF table refuses it before an ELF exists (M1).
- **Where it does not cross an AF,** nothing catches it: two chip selects (M1c, A2) and two motor outputs (A1).

Every non-GPIO line has no independent judge either (A3). That information is not in `gpio_default_config`. It is in `hwdef.h`'s device tables (`HAL_SPI_DEVICEn … PAL_LINE(GPIOx,n)`, `HAL_PWM_GROUPn` channel maps, `HAL_BARO_PROBEn`, `HAL_I2C_MAX_CLOCK`, `HAL_SERIALn_DRIVER`) and in the ELF's copies of them. §1.3 names three comparisons, all from GPIO bytes. Reading those tables is a scope change, so it is the architect's.

Everything else in the work order is built, measured and green on Furnace.

**F1, the options:**
- **(a) A `hwdef.h` judge in `aethelgard-pincheck`.** It would hold each topology SPI device's CS pin, bus and mode, and each motor's timer channel and PWM index, against `HAL_SPI_DEVICEn` and `HAL_PWM_GROUPn`. It would hold `HAL_I2C_MAX_CLOCK` against the minimum of `i2c_speeds`. `BARO`, serial order and bauds could be added. Its inputs are `hwdef.h` (ArduPilot's computation, never the generated file) and `ir` (the parts table's `spidev` names). It is a follow-up work order of about the check's own size; I recommend it, ahead of `4400c`.
- **(b) Count ArduPilot's own refusals** as the catch for cross-AF swaps (M1), and name chip-select, motor and non-GPIO identity as a residual for `4400c`, whose twin could observe which device answers on which select.
- **(c) Accept §2 as met for the GPIO configuration alone,** and record the residual in the roadmap.

**F3, RC ports (A6).** ArduPilot runs CRSF at its own 416666 baud and sets SBUS inversion itself, whatever the defines say. So a declared 420000 CRSF port cannot be stated in `hwdef.dat` (§7, second condition). Options:
- (a) emit no `BAUD` or `OPTIONS` for an RC port, and name that in the header;
- (b) refuse an RC port whose declared baud is not one of `AP_RCProtocol`'s;
- (c) leave it as built.

I recommend (a).

**F4, stored speeds (A7).** Should `validate` re-derive `pin_speeds`, as it re-derives ceilings and controllers? That would refuse a hand-edited speed at every reader. It would also move svd-hal's `a_line_no_selectable_speed_serves_is_refused` from svd-hal's refusal to the topology's, which is outside my fence. I recommend yes, with that test moved, in the follow-up.

**F2, AF-prefixed CS names (A9):** backlog. ArduPilot refuses them; the generator could do so by name.

## 15. Process

- **One built-in denial.** A regenerate command containing `rm -f $D/$1.hwdef.dat`, a variable path, was refused by Claude Code's removal check. Nothing in it ran, and the work was redone by writing to a fresh directory and copying. It was not retried.
- **One container at a time:** mine and the adversary's ran sequentially.
- **Merges:** `main` was merged twice, at 4220b and at 1801. 4220b's merge turned the S02 console's J3 into an extract failure, measured; the console moved to J2.
- **Disk:** every scratch tree of mine under `~/.cache` is removed before the signal except `~/.cache/aet-4400b/pr-title-body.md`. The datasheets stay in `~/.cache/aet-ds/`. No agent worktree was created: the adversary worked read-only here.

## 16. The ruling on the second stop (PR #189, 04:59 UTC), and what this PR closes with

- **F1: (a), as a follow-up.** `AET-WO-4400b2`, mine, on a new branch from `main` after this merges: a `hwdef.h` judge in `aethelgard-pincheck`, sharing no code with the generator. It will hold `HAL_SPI_DEVICEn`, `HAL_PWM_GROUPn`, `HAL_I2C_MAX_CLOCK`, the serial order and `BARO` against the topology. Its acceptance is M1c, A1 and A2, each refused by name.
- **§2's third bullet, as narrowed by the ruling, for this PR:**
  - **a swap across an alternate function** is caught by ArduPilot's own AF table, before an ELF exists (M1, measured);
  - **a swap within the GPIO configuration** is caught by `check pinout` (M1b, M3c, M5, M6, measured);
  - **chip-select, motor-channel and non-GPIO identity** belong to `4400b2`.
- **F3: (a), built here.**
  - An RC-input port (CRSF, SBUS) takes `DEFAULT_SERIALn_PROTOCOL SerialProtocol_RCIN` and no `_BAUD` or `_OPTIONS`.
  - The header names the port, its SERIAL number, the declared baud and what ArduPilot runs, cited at the tag: CRSF at 416666 baud 8N1 (`AP_RCProtocol.cpp:351`), SBUS at 100000 8E2 with RX inverted (line 344). Both come from `apply_to_uart` (lines 326-336), which sets parity, stop bits and RXINV itself.
  - **Test (`hwdef_tier1.rs`):** serial-four with its MAVLink port as the console and SBUS made a plain GPS port, solved and generated. CRSF's SERIAL has `_PROTOCOL` and neither `_BAUD` nor `_OPTIONS`, the header names 416666 against the declared 420000, and every other port keeps its baud.
  - **Mutant:** with the RC branch disabled, that test fails [measured].
  - The architect owns the backlog row for judging an RC port's tolerance at the baud the firmware runs.
- **F4** (`validate` re-derives `pin_speeds`, with svd-hal's test moved) goes with `4400b2`.
- **F2** is a backlog row (A9).
- **D8's docs** are the architect's: the architecture §3 row and the releasing order are his, after this merges. The order line I added (step 10) is in `docs/releasing.md`, which the ruling leaves to him; he may restate it.

## 17. READY

- **CI on `554cbe5b`** (F3 built, `main` at `2b43672`): green in all four workflows: `ci` with gates and all eight tier-2 shards, `ardupilot-image` with the three `generated` boards and `pinout`, `tools-image` and `engine-image`.
- **`main` then moved** (1312 #191, specification 14; docs #194) and is merged in at `f425df94`. Only specification hashes moved:
  - the two solve digests, proved again in their comment;
  - the hwdef fixtures' topologies and the goldens' header line. No emitted line changed.
- **The local re-gate passes:** fmt, clippy `-D warnings`, `cargo test --workspace` (125 binaries).
- **CI on the READY head** is in the PR's checks. The architect merges on green.
- **Disk:** every scratch tree of mine is removed, except `~/.cache/aet-4400b/pr-title-body.md`.
- **Next:** `AET-WO-4400b2` (F1 and F4), on a new branch from `main` after this merges.
