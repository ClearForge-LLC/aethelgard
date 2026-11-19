# aethelgard-hwdef

ArduPilot's board definition, `hwdef.dat`, generated from a `SolvedTopology` and the parts tables alone (`AET-WO-4400b`, `RUL-16`). Unmodified ArduPilot `Copter-4.7.2` (`fb773567`) builds it as an untracked board directory. `aethelgard check pinout` (`aethelgard-pincheck`) then judges the ELF ArduPilot links, and shares no code with this crate (`INV-07`, `RUL-20`). The CLI runs it as `aethelgard generate hwdef --topology <file> --out <file>` (`crates/aethelgard-cli/README.md`).

**What it reads.**
- **The topology,** validated first; an invalid one is refused.
- **`parts.json`'s `ardupilot` column:** a part's `BARO` or `IMU` driver, its `SPIDEV` name, and the chip ID that driver accepts, which the parser holds equal to the part's identity.
- **`power-parts.json`'s MCU row:** `flash_kilobytes`, ArduPilot's `MCU` names, and the SPI kernel clocks ArduPilot runs at.

It never reads a `Specification`.

**What it writes,** in this order, each line with a comment naming its source: `[T]` the topology, `[P]` a parts row, `[C]` a constant named in the header.

| line | source |
|---|---|
| `MCU <series> <mcu>` | `[T]` `mcu`; `[P]` the MCU row's `ardupilot` names |
| `OSCILLATOR_HZ` | `[T]` `clock.hse_hz`. The MCU row cites ArduPilot's clocks only for `0`, so any other value is refused (`OscillatorNotCited`) |
| `FLASH_SIZE_KB` | `[P]` `flash_kilobytes` |
| `APJ_BOARD_ID 24400` | `[C]` |
| `define AP_BOOTLOADER_FLASHING_ENABLED 0`, `FLASH_RESERVE_START_KB 0` | `[C]` no bootloader (`4400` D3) |
| `define HAL_STORAGE_SIZE 16384`, `STORAGE_FLASH_PAGE` | `[C]` the size; `[P]` + `[C]` the page, the last two 128 KB sectors (`flash_kilobytes / 128 − 2`), because storage uses page and page+1 (`Storage.cpp:331-336`) |
| `STM32_ST_USE_TIMER` | `[C]` TIM5, else TIM2: a timer no motor output takes. Both taken is refused (`NoSystemTimer`) |
| `SERIAL_ORDER`, the USART/UART pins, `define DEFAULT_SERIALn_PROTOCOL / _BAUD / _OPTIONS` | `[T]` `serial_ports`: the console first, as SERIAL0, then the specification's order. SERIAL0 takes `_BAUD` alone: its protocol default is a literal and it has no options, so those defines would be read by nothing (`AP_SerialManager.cpp:37-44, 172, 180`). `_OPTIONS` is `3` (RXINV, TXINV) on an inverted port. An RC-input port (CRSF, SBUS) takes its `_PROTOCOL` alone: ArduPilot's RC input sets its UART itself (`AP_RCProtocol.cpp:326-352`), so the header names the declared baud and the one ArduPilot runs (CRSF 416666, SBUS 100000 8E2 inverted) instead (ruling F3, PR #189). SERIAL7 and SERIAL8 take no `_OPTIONS`: ArduPilot reads a literal 0 for them (`AP_SerialManager.cpp:357, 376`), and the header names the omission (`AET-WO-4404`) |
| `I2C_ORDER`, `define HAL_I2C_MAX_CLOCK`, the I2C pins | `[T]`: buses by their first device in index order, then device-less buses by peripheral. The clock is the minimum of `i2c_speeds`, so no bus runs faster than declared |
| `BARO <driver> I2C:<order index>:<address>` | `[T]` the device; `[P]` its driver |
| the SPI pins, each chip select as `CS`, `SPIDEV <name> <SPIn> DEVID<k> <cs> MODE0 <hz> <hz>`, `define HAL_LOGGING_DATAFLASH_ENABLED 1` | `[T]` + `[P]`. The speed is the controller's `achieved_hz`, rounded down to a whole kHz. `DEVID` is the device's place on its bus, from 1. The mode is `parts::DRIVEN_SPI_MODE` |
| `<pin> TIMn_CHk TIMn PWM(index+1)` | `[T]` `motor_outputs` |
| `IMU <driver> SPI:<name> ROTATION_<x>` | `[T]` the device's declared `orientation`; `[P]` its driver, `SPIDEV` name and `ardupilot.frame` (`chip`); `[C]` the board frame (`AET-WO-4400d`). The rotation is the one whose matrix, as ArduPilot's `Vector3::rotate` computes it at the tag, takes the chip's X, Y = Z × X and Z to the board frame: x forward, y right, z down through F.Cu, the board mounted F.Cu up (ruled D3, named in the header). Of two names for one matrix, the lower-numbered (ruled D4) |
| `define HAL_USE_ADC FALSE` | `[T]` no `power.sense` |

**Every pin line** carries ArduPilot's own tokens (`chibios_hwdef.py:349-431`):
- the stored speed, `SPEED_VERYLOW` for `0` and `SPEED_LOW` for `1`;
- `OPENDRAIN` on an I2C line and `PUSHPULL` on every other;
- `FLOATING`, which overrides ArduPilot's pull-up on a CS and a USART, except `PULLDOWN` on a motor output (`4400b` D3). On the USART of SERIAL1 to SERIAL9 it holds from reset only: ArduPilot's `set_options` then pulls TX and RX up, or down on a line it inverts (`UARTDriver.cpp:1601-1625`). No `hwdef.dat` line can remove that, and the header says so on any board with such a port (`AET-WO-4404`).

`DEFAULTGPIO` is never emitted (`4400a` D-1). The comments (`#`) are discarded by ArduPilot's reader (`hwdef.py:209`).

**What it refuses, by name:**
- an invalid topology;
- no serial port; ports but no console; two consoles; two RC-input ports (`AP_SerialManager.cpp:536-541` drops the second); more than ten ports;
- an inverted console;
- framing other than the role's: 8N1 not inverted for console, MAVLink, GPS and CRSF; 100,000 8E2 inverted for SBUS (`AP_RCProtocol.cpp:339-352`);
- a console baud SERIAL0's `baud / 1000` parameter cannot map back to, or a SERIALn baud of 2000 or less;
- a signal outside I2C, SPI, USART/UART and TIM;
- a chip select named like an emitted pin label, or whose name starts with one of ArduPilot's alternate-function prefixes, case-sensitive as ArduPilot's own test is (`LabelTakesAfPrefix`; `chibios_hwdef.py:33`, `173-178`). ArduPilot would look the label up as an alternate function of its pin and fail at configure (`AET-WO-4404`, measured for all fourteen);
- two devices with one `SPIDEV` name;
- an SPI rate below ArduPilot's slowest on the cited kernel (kernel ÷ 256, `SPIDevice.cpp:259-276`), or an SPI peripheral with no cited kernel;
- a topology with battery sense pins, because their `SCALE` has no source (`4400b` D7);
- a solved pin no emitted line configures;
- an IMU with no declared orientation (`ImuOrientationUndeclared`, `INV-06`), or whose parts row cites no driver frame (`ImuFrameNotCited`).

**What it names and does not emit** (the header lists each, and the CLI's report carries them):
- a device whose part has no `ardupilot` column. Today that is the TMP117, which ArduPilot's TMP119 driver refuses: 0x2117 against 0x0117;
- an RC-input port's baud and options, with what ArduPilot runs;
- a motor output's protocol and PWM range, which are ArduPilot parameters (`MOT_PWM_TYPE`, `MOT_PWM_MIN/MAX`) for the `defaults.parm` work order;
- `BIDIR`, which nothing declares.

**What judges it.**
- `check pinout`, on the ELF: mode, AF, `OTYPER`, `OSPEEDR` and `PUPDR` of every topology pin, the reverse direction and the package; with `hwdef.h`, the identity judge (`aethelgard-pincheck`), which reads each device table as the compiler expands it, the `IMU` probes and their rotations among them (`AET-WO-4400d`), and each serial port's protocol and options (`AET-WO-4404`).
- ArduPilot's own configure and compile, which refuse a pin with no such function in its AF table, a serial port with no pins, and a system timer a motor takes.
- This crate's tests, which hold every emitted keyword and define to the exact names ArduPilot reads at the tag, each with its line.

**What nothing independent judges yet** (`AET-WO-4400b` FEEDBACK §11; ruled to `AET-WO-4400b2`, a `hwdef.h` judge):
- which chip select belongs to which `SPIDEV`;
- which motor is which `PWM(n)`;
- `SERIAL_ORDER`, `HAL_I2C_MAX_CLOCK`, `BARO`, `SPIDEV` modes and speeds, and `DEFAULT_SERIALn_BAUD`.

These are in `hwdef.h`'s device tables, not in `gpio_default_config`. CI's `cmp` against the committed file checks consistency, not judgement.
