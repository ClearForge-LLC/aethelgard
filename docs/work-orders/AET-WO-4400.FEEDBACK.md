# AET-WO-4400 FEEDBACK — ArduPilot's board definition: spike

**Builder:** Claude (strong tier), worktree `~/aethelgard-4400`, branch `wo/aet-4400-hwdef` from `main` `c621461`.
**Cadence:** spike (§1), then STOP for the architect's ruling.

Predictions are committed in two rounds, each before the runs it predicts, and are never edited afterwards; corrections go in §3.
- **Round 1** (§1.1) was committed before any ArduPilot clone, download or network fetch.
- **Round 2** (§1.2) was committed after the S2 source reads and before the S3 and S4 runs. Its predictions are informed by those reads, and say so.

Labels:
- **[read]**: read at source, either in this repository at `c621461`, in ArduPilot at the pinned tag, or in Renode 1.17.0.
- **[measured]**: run on Furnace. The cap, the `nice` level and `-j` are recorded with each run.
- **[inferred]**: neither of those; reasoning from the two.

## 0. Source reads that shape round 1

All **[read]**, this repository at `c621461`:
- **`SolvedTopology`** (`crates/aethelgard-ir/src/lib.rs`, schema 3) holds:
  - `assignments`: net to pin, peripheral, signal and AF;
  - `gpio`: net to pin;
  - `i2c_ceilings` and `i2c_speeds`;
  - `analog_lines`;
  - `i2c_devices`: part, bus, peripheral and address;
  - `spi_devices`: part, bus, peripheral and the chip select's net.
- **It holds no MCU part, no SPI bus speed and no supply.** These stay in the `Specification` (`mcu`, `spi_buses[].speed_hz`, `voltage_domains`). Under `RUL-16` a consumer may not read them there.
- **Sentinel-01's solved pins** come from the committed board crate (`boards/sentinel-01/src/board.rs`):
  - `I2C1_SCL` on PB6 AF4, and `I2C1_SDA` on PB7 AF4;
  - `SPI1_SCK` on PA5 AF5, `SPI1_MISO` on PA6 AF5, and `SPI1_MOSI` on PA7 AF5;
  - the flash's chip select on PA0, as a GPIO.
- **The MCU** is `STM32H753ZITx` (LQFP144) in `examples/sentinel-01-minimal.json`.

## 1. Predictions

### 1.1 Round 1 (before any clone or fetch)

All **[inferred]** unless marked otherwise.

**S1, building unmodified ArduPilot.**
- **The tag:** the newest stable `Copter-4.6.x`, or a `4.7.x` if one has shipped. It is a lightweight or annotated tag on a `Copter-4.x` branch.
- **The toolchain** ArduPilot documents for ChibiOS: `gcc-arm-none-eabi-10-2020-q4-major`, an x86_64 Linux tarball of about 150 MB, from `firmware.ardupilot.org`. A newer tag may have moved to 13.x; if so, I expect the tag's own prerequisite script to say so.
- **Python:** the host's 3.14 fails somewhere: `empy` 3.3.4, `future`, or waf's own import of a removed module. A uv-managed 3.12 works. That adds the host that serves uv's Python builds.
- **Host tools:** waf needs no host C compiler for a ChibiOS board, and nothing missing forces a §7 stop. `git`, `python3` and `uv` are present.
- **Disk:**
  - the clone with submodules: 1.5–2.5 GB;
  - the toolchain unpacked: about 0.5 GB;
  - the venv: under 0.2 GB;
  - one board's build directory: about 0.3 GB.
  - Total: about 3 GB.
- **Time, from clean, `nice 10`, `-j6`:** `./waf configure` takes about 30 s. `./waf copter` takes 4–8 min wall and 20–35 CPU-minutes (user + sys).
- **Peak memory** of the build scope: 2–4 GB.
- **Network hosts**, all in the fetch phase:
  - `github.com`, for the clone and every submodule;
  - `firmware.ardupilot.org`, for the toolchain;
  - `pypi.org` and `files.pythonhosted.org`, for pip;
  - GitHub's release-asset host, for uv's Python.
- **The build phase reaches no host:** configure and build pass with the network removed.
- **Where the build belongs:** a **second image**, `FROM` nothing of ours, holding the pinned toolchain, the venv and a pinned shallow clone. The tools image is about 5 GB, the toolchain is about 0.5 GB unpacked, and ArduPilot's GPL tree is not ours to put in an image we publish beside our own crates.
- **CI cost** for an image-cached build: about 10 min per PR on a 4-core runner. That is CPU-time ÷ 4 plus configure, and it runs only when the generator or its input changes.

**S2, the field table** (what I expect before reading `chibios_hwdef.py`):
- **From the topology today:**
  - I2C buses and their order, from `assignments` and `i2c_devices`;
  - `BARO BME280 I2C:<n>:0x76`;
  - SPI buses and pins;
  - the flash's chip select as a GPIO output.
- **Not from the topology today, though the specification has them:**
  - the MCU part, which is the `MCU` line and `FLASH_SIZE_KB`;
  - the SPI speed for `SPIDEV`;
  - the I2C bus speed. It is in `i2c_speeds`, but whether ArduPilot reads a per-bus speed at all is unknown.
- **Waiting on a `4300` piece:** PWM outputs (`4300a`), UARTs and `SERIAL_ORDER` (`4300b`), ADC sense pins and scales (`4300d`), and USB, `OTG1`, PA11 and PA12 (`4300e1`).
- **Waiting on a work order not yet written:**
  - the oscillator: `OSCILLATOR_HZ`, PH0 and PH1;
  - LEDs and the buzzer, which need a generic declared GPIO;
  - the SD card (SDMMC);
  - `IMU`, which waits on P4's IMU work order;
  - `APJ_BOARD_ID`, a registry number with no source in the topology;
  - the flash layout, the bootloader and the storage.
- **DMA** is computed by ArduPilot's own resolver from the peripherals, so no topology field is needed.

**S3, a minimal `hwdef.dat`:**
- **The form:** `--board` takes only a directory name under `libraries/AP_HAL_ChibiOS/hwdef/`, with no out-of-tree option. An untracked directory added to the clone keeps every tracked file unmodified, shown by `git status --porcelain`.
- **The lines:** with the MCU, the clock, flash, I2C1 with the `BARO` line, SPI1 and its `SPIDEV`, and no UART, PWM, IMU or USB, `./waf configure` fails on a missing required field. My guess is `SERIAL_ORDER`, the storage, or `APJ_BOARD_ID`.
- **The fix:** with those added, `./waf copter` builds.
- **The split** of the final file: fewer than half its lines can be emitted from today's topology.

**S4, the pinout check:**
- **What `hwdef.dat` can say:** it names a pin's function, not its AF number. The AF comes from ArduPilot's table by function label, so "seed a wrong AF into the `hwdef.dat`" can only be done by naming a function with a different AF on the same pin.
- **What `hwdef.h` holds:** per-port initialisers for `MODER` and `AFRL`/`AFRH` (ChibiOS `VAL_GPIOx_*`), which compile into the binary. A parser of those, driven from the topology, names:
  - **a wrong pin** (`I2C1_SCL` moved from PB6 to PB8) as "the net's solved pin PB6 is not on AF4";
  - **a wrong AF** (PB6 given a function on another AF) as "PB6 is on AF *n*, the topology solved AF4".
- **Independence:** ArduPilot's `STM32H743xx.py` was generated from an ST source other than `STM32_open_pin_data`, probably a datasheet table or CubeMX XML. It is therefore partly independent of the solver's table.

**S5, ArduPilot in the twin (read only):** Renode 1.17.0's H753 platform does **not** reach ArduPilot's boot banner. ChibiOS's H7 start-up waits on a ready flag that Renode does not model. The candidates are `SYSCFG_CCCSR.READY` (`4300` S5 measured that this wait hangs), a PWR `VOSRDY`, or a PLL ready bit. It never reaches the UART.

**§7:** no stop.
