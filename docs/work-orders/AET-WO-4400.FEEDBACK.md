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

### 1.2 Round 2 (after the S2 source reads, before any S3 or S4 run)

These are informed by §4's reads, which were done by a subagent and spot-checked at source:
- `get_alt_function` (chibios_hwdef.py:151-179);
- the `STM32H753xx.py` diff and its provenance comment;
- `VAL_GPIOB_AFRL` in Matek's `hwdef.h`;
- `gpio_default_config` (board.c:108), present in the ELF as a 308-byte symbol (`nm -S`).

**S3** [inferred]:
- **The hand-written board** has only what the topology has, plus the four keys the script requires: `MCU`, `OSCILLATOR_HZ`, `FLASH_SIZE_KB` and `APJ_BOARD_ID`. With those, `./waf configure` passes.
- **`./waf copter` then fails to compile.** With no `SERIAL_ORDER`, `write_UART_config` writes no serial configuration, and the HAL's serial driver macros are missing.
- **With `SERIAL_ORDER` added** (`EMPTY`, or `OTG1` without its pins), it builds. A missing `STORAGE_FLASH_PAGE` does not stop the build.
- **The `APJ_BOARD_ID`:** a name absent from `board_types.txt` is refused at configure. A numeric ID is accepted off CI.
- **The `BARO` line** is `BARO BMP280 I2C:0:0x76`. `AP_Baro_BMP280` accepts the BME280's 0x60 (AP_Baro_BMP280.cpp:39, :76, read).
- **The TMP117 gets no line.** ArduPilot has no TMP117 driver: its TMP119 driver checks the ID against 0x2117 (AP_TemperatureSensor_TMP119.cpp:31, :62, read), and the TMP117 reads 0x0117 [inferred from the part].
- **The W25Q64JV** is a `SPIDEV dataflash` that the JEDEC logger knows (`JEDEC_ID_WINBOND_W25Q64 0xEF4017`, AP_Logger_Flash_JEDEC.cpp:54, read). Its speed is not in the topology.
- **The split:** of the lines needed to build, the topology alone gives at most 9 of about 16: two I2C pin lines, three SPI pin lines, the chip select, `I2C_ORDER`, `BARO` and part of `SPIDEV`.

**S4** [inferred]:
- **A `hwdef.dat` carries no AF** (read: no AF token; the AF comes from `AltFunction_map` by `PIN:LABEL`). A wrong AF can be seeded only by a label whose table AF differs on the same pin:
  - `PB6 I2C4_SCL I2C4` and `PB7 I2C4_SDA I2C4`, with `I2C_ORDER I2C4`, give AF6 where the topology solved AF4;
  - a wrong pin is `PB8 I2C1_SCL I2C1`.
- **The check** decodes `MODER`, `AFRL` and `AFRH` per port from the ELF's `gpio_default_config` bytes, which are what the board's start-up writes into the GPIO registers. It also decodes them from `hwdef.h`'s `VAL_GPIOx_*` text. Driven from the topology, it names:
  - **on the clean board:** all 5 AF lines and the 1 GPIO match, and the two decodes agree;
  - **for the wrong-pin seed:** `I2C_SCL` at PB6 is not in alternate mode, and the reverse direction finds PB8 on AF4 with no topology net;
  - **for the wrong-AF seed:** `I2C_SCL` at PB6 is on AF6 where AF4 was solved, and the same for `I2C_SDA` at PB7.
- **ArduPilot's table against `STM32_open_pin_data`** (`STM32H753ZITx.xml` plus the H747 GPIO modes file the repository vendors), over the signals the LQFP144 pins carry:
  - after the names are normalised (`CAN`/`FDCAN`, `EVENT-OUT`, `_CTS_NSS`), more than 98 % of shared (pin, signal) pairs carry the same AF;
  - a handful disagree or appear on only one side, among them `PB7:CAN2_TX` AF9, a hand-added line with different indentation (STM32H753xx.py:352);
  - the table was extracted by tabula from a datasheet PDF (`af_parse.py`; the header says so), so it is independent of the solver's ST XML in its extraction, but not in its ultimate source, ST.
- **Package blindness:** ArduPilot configures a board that names a pin the LQFP144 package lacks (PI0, say) without an error, because the table has no package data (read). A third seed measures this.

## 2. S1 — building unmodified ArduPilot on Furnace (measured)

Everything lives under `~/.cache/aet-4400/`, nothing in the repository. Each build step ran through `run.sh` (in `spike/4400/`) as follows:
- inside `systemd-run --user --scope -p MemoryMax=16G -p MemorySwapMax=0`, under `nice -n 10`;
- timed with `/usr/bin/time -v`;
- with the scope's `memory.peak` read before it exited.

The scope read back `memory.max` = 17179869184 and `memory.swap.max` = 0. No container was used for S1.

**What was fetched** [measured]:

| item | pin | from | wall | size |
|---|---|---|---|---|
| ArduPilot | `Copter-4.7.2`, the newest stable Copter tag of 58 `Copter-4*` tags (no 4.8). A lightweight tag on commit `fb773567896014da007bca58358704d4a0d8516f` | `git clone --depth 1 -b Copter-4.7.2 --recurse-submodules --shallow-submodules https://github.com/ArduPilot/ardupilot.git` | 29.1 s | 1.4 GB (`modules/ChibiOS` 375 MB, `gsoap` 100 MB) |
| 15 submodules | the commits the tag records, among them ChibiOS `d103983f`, mavlink `288b907c`, waf `35eadbb6` | `github.com` only (every `.gitmodules` URL) | (in the clone) | |
| toolchain | `gcc-arm-none-eabi-10-2020-q4-major` (GCC 10.2.1 20201103), the one the tag's `Tools/environment_install/install-prereqs-ubuntu.sh:249` pins | `https://firmware.ardupilot.org/Tools/STM32-tools/gcc-arm-none-eabi-10-2020-q4-major-x86_64-linux.tar.bz2` | 236 s (about 0.66 MB/s from Furnace) | 156,882,554 B tarball, 693 MB unpacked. sha256 `21134caa478bbf5352e239fbc6e2da3038f8d2207e089efc96c3b55f1edcd618`, md5 `8312c4c91799885f222f663fc81f9a31` |
| Python | the host's `/usr/bin/python3` 3.14.4 in a venv. **It worked**, so there was no uv Python and no extra host | | | 132 MB venv |
| pip | `dronecan==1.0.27 empy==3.3.4 fastcrc==0.5.0 future==1.0.0 intelhex==2.3.0 lxml==6.1.3 pexpect==4.9.0 ptyprocess==0.7.0 pymavlink==2.4.50` (freeze). This is a superset taken from the tag's `PYTHON_PKGS` (line 197); the minimal set was not bisected | `pypi.org`, `files.pythonhosted.org` | 10.7 s | |

- **The md5** matches the figure ARM published for this tarball, as I recall it **[inferred: recalled, not fetched]**. The sha256 above is the pin to use.
- **The toolchain's binaries:** `arm-none-eabi-gcc` resolves every shared library. `arm-none-eabi-gdb` needs `libncurses.so.5` and `libtinfo.so.5`, which Furnace lacks. The build never runs gdb.

**Network hosts** [measured]. The fetch steps ran under `strace -f -e trace=connect,sendto,sendmsg,sendmmsg`, and DNS query names and TLS SNI were decoded from the trace (`hosts.py`). The hosts:
- `github.com` (140.82.112.4, .113.4, .114.3);
- `firmware.ardupilot.org` (159.65.186.182);
- `pypi.org` and `files.pythonhosted.org` (Fastly 151.101.x.223, 2a04:4e42::223 and others).

That is all, and each one answered. **§7's second condition does not hold.**

**The build phase reaches no host** [measured]:
- A clean `./waf configure --board MatekH743` followed by `./waf copter -j6`, traced the same way with `--seccomp-bpf`, made **0** `connect`, `sendto`, `sendmsg` or `sendmmsg` calls (4,457 trace lines, all signal lines).
- A positive control with the same flags on `git ls-remote` caught 4 `connect` calls and `github.com`.
- `unshare -n` could not be used, because Furnace sets `kernel.apparmor_restrict_unprivileged_userns = 1`. The trace is the proof instead.
- waf's "Update submodules: yes" found the submodules at their recorded commits and fetched nothing.

**Host tools waf looked for** [measured, configure's log]: `python` (the venv), the cross `gcc`/`g++`/`nm`/`objcopy`/`size`, `make`, `git` and `rsync`. All are present on Furnace. Nothing needed `sudo`, a system package or a credential. **§7's first condition does not hold.**

**Build measurements**, board `MatekH743` (STM32H743), `-j6`, `nice 10` [measured]:

| run | configure wall | `copter` wall | `copter` CPU (user + sys) | scope `memory.peak` | build dir |
|---|---|---|---|---|---|
| first build | 0.58 s | **59.96 s** | 285.2 + 46.1 = **331 s** | **1.23 GB** (1,225,867,264 B) | 105 MB |
| clean rebuild, traced | 0.57 s | 54.85 s | 267.9 + 46.3 = 314 s | 1.22 GB | 105 MB |

- **The output:** `bin/arducopter` text 1,545,804 B, 1,550,440 B of flash used and 153,484 B free. waf wrote `.bin`, `.apj`, `.abin` and `_with_bl.hex`.
- **No tracked file changed:** `git status --porcelain` in the clone printed nothing.
- **Disk:** about 2.5 GB in all. That is 1.4 GB of clone, 0.1 GB of one board's build, 0.69 GB of toolchain, 0.15 GB of tarball and 0.13 GB of venv.

### Where the build belongs (proposal)

**Recommended: a second image, `aethelgard-ardupilot`,** built and pinned by CI exactly as the tools image is today, and holding:
- the toolchain tarball, checked against the sha256 above, unpacked;
- a venv with the frozen pip set;
- the shallow clone at `fb773567` with its submodules.

Its build job then runs with `--network none` [inferred from the trace above].

Reasons:
- **Licence.** ArduPilot is GPLv3. An image of its own keeps the GPL tree out of the image the crates and the engine are built `FROM`, and out of `container/IMAGE`'s digest.
- **Lifecycle.** A tag bump touches only this image.
- **Size.** About 2.3 GB of content, against the tools image's roughly 5 GB.
- **Against the other two options.** A layer in the tools image is the second choice: it is simpler to pin, but every tools-image rebuild would carry ArduPilot, and an ArduPilot bump would move the digest every tier-2 test runs in. Furnace-only fails "CI is the witness" (roadmap §6) for a P4 exit clause.

**CI time** [inferred, scaled from the measured CPU time]:
- **Per PR that runs the build:** about 331 CPU-s ÷ 4 vCPU, so 1.5–2 min on a 4-vCPU GitHub runner, plus the image pull (about 2.5 GB, roughly 1 min). That is **3–4 min**.
- **The image build** happens only when its pins change: the clone, the toolchain and pip, about 2–5 min on GitHub's network.
- **Without an image** (fetching every time), add the clone and the toolchain. That is fast from GitHub's network, and 4.5 min from Furnace's.

## 3. S2 — the `hwdef.dat` format and the field table

Read at `Copter-4.7.2` by a subagent; every row used here was spot-checked at source (`chibios_hwdef.py`, called `CH` below; its base class `libraries/AP_HAL/hwdef/scripts/hwdef.py`, called `HB`). The subagent tried once to run `chibios_hwdef.py` into a scratch directory, outside its read-only brief. The permission classifier denied it and it did not retry. I record it here. No call of mine was denied.

**How the file is read** [read]:
- A line `Pxn LABEL TYPE [extras]` is a pin.
- Any other line is `config[keyword] = rest`. Unknown keywords are stored and ignored, never refused (CH:2884).
- Required keys fail with `missing required value X in hwdef.dat` (CH:632).
- **The AF is never written.** `get_alt_function` (CH:151-179) looks up `"PIN:LABEL"` in the MCU module's `AltFunction_map`. A label with no entry is `Unknown pin function` and exits 1. There is no AF token, and `ALT(n)` selects an alternate configuration set, not an AF.
- **The MCU line.** `MCU STM32H7xx STM32H753xx` imports `STM32H753xx.py`. That file is `STM32H743xx.py` byte for byte except the shebang and the docstring (diff, read). The table has no package data: ports A–K, 16 pins each.
- **What reaches silicon.** The AF reaches the binary only through `VAL_GPIOx_{MODER,AFRL,AFRH}` in `hwdef.h` (CH:2605-2623). `board.c:108` compiles these into `gpio_default_config`, which `gpio_init` writes at start-up. A few `PAL_LINE`s go into bus configurations. The per-pin comment block is not compiled.
- **Out of tree.** There is none. `--board` resolves only to `libraries/AP_HAL_ChibiOS/hwdef/<name>/` (boards.py:677-697). `--extra-hwdef` appends to a board's file and cannot replace it (wscript:423, chibios.py:631).

### The field table

Key to "today": **T** = emitted from `SolvedTopology` today; **S** = the value exists only in the `Specification` (`RUL-16` forbids reading it there); **N** = in neither.

| field | `hwdef.dat` form | ArduPilot needs it? | today | waits on |
|---|---|---|---|---|
| MCU | `MCU STM32H7xx STM32H753xx` | required (CH:3102) | **S**: `SolvedTopology` has no MCU | a topology `mcu` field, copied by `solve` (small; with `4400`'s build) |
| oscillator | `OSCILLATOR_HZ 8000000`; no HSE pin lines | required (CH:885). `0` is legal on H7: HSI drives the PLLs, giving 400 MHz (`stm32h7_mcuconf.h:113`, read) | **N**: the solver reserves PH0/PH1; there is no declared frequency | **no work order**: the oscillator and clock tree as a declared fact (D1) |
| flash size | `FLASH_SIZE_KB 2048` | required | **N**: a part fact | a parts-table row for the MCU (with the `mcu` field) |
| flash layout | `FLASH_RESERVE_START_KB` (default 16) | optional, but must be 0 without a bootloader (§4, measured) | **N** | decision D3 |
| bootloader | `define AP_BOOTLOADER_FLASHING_ENABLED 0`, or a `hwdef-bl.dat` and a built bootloader | one or the other is required (measured, step A) | **N** | decision D3 |
| storage | `define HAL_STORAGE_SIZE n`; `STORAGE_FLASH_PAGE p` | the size is required to compile (measured, step C); the page is optional to build | **N** | D3: a generator constant |
| board id | `APJ_BOARD_ID <name or number>` | required. A name must be in `Tools/AP_Bootloader/board_types.txt`; a number is refused in CI (`GITHUB_ACTIONS`) | **N** | decision D4 |
| system timer | `STM32_ST_USE_TIMER n` | optional (default TIM5, `stm32h7_mcuconf.h:605`) | n/a | `4300a`: must avoid the timers motors take |
| UARTs, `SERIAL_ORDER` | `PD8 USART3_TX USART3` and RX; `SERIAL_ORDER USART3 …` | **at least one USART/UART is required** (measured: `EMPTY` and `OTG1` alone both fail) | **N** | **`4300b`** |
| SPI bus pins | `PA5 SPI1_SCK SPI1` … | when an `SPIDEV` exists | **T** (`assignments`) | — |
| chip select | `PA0 FLASH_CS CS` | for each `SPIDEV` | **T** (`gpio`, label = net) | — |
| `SPIDEV` | `SPIDEV name SPIn DEVIDk CS MODEm low high` (7 tokens, CH:1451-1475) | per SPI device | bus and CS **T**; speed **S** (`spi_buses[].speed_hz`); mode and role name **N** | the bus speed into the topology; the mode and ArduPilot role as parts-table columns |
| I2C pins, `I2C_ORDER` | `PB6 I2C1_SCL I2C1`; `I2C_ORDER I2C1` | `I2C_ORDER` is required with I2C pins (CH:1932) | **T** (`assignments`, `i2c_devices[].peripheral`) | — |
| I2C speed | **no per-bus field.** One global `define HAL_I2C_MAX_CLOCK` (default 100 kHz, I2CDevice.cpp:53) | optional | **T** (`i2c_speeds`), but ArduPilot cannot say "per bus" | decision D6 |
| `BARO` | `BARO BMP280 I2C:0:0x76`; the index is the position in `I2C_ORDER` | optional | **T** (part, address, order). The driver name is a mapping (`BME280` → `BMP280`, AP_Baro_BMP280.cpp:39) | a parts-table column "ArduPilot driver" |
| other I2C parts | TMP117 | — | **no line possible**: ArduPilot's `TMP119` driver refuses ID 0x0117 (`…TMP119.cpp:31,62`); temperature sensors are set by parameters, not `hwdef` | — |
| flash logger | `define HAL_LOGGING_DATAFLASH_ENABLED 1` with an `SPIDEV dataflash` | optional | **N**: the part's role | parts-table column (the W25Q64 is `0xEF4017`, AP_Logger_Flash_JEDEC.cpp:54) |
| PWM outputs | `PE9 TIM1_CH1 TIM1 PWM(1) GPIO(50)`, plus `BIDIR` | optional (`HAL_USE_PWM FALSE`) | **N** | **`4300a`** |
| DMA | computed by `dma_resolver.py` (DMAMUX on H7); `DMA_PRIORITY` and `DMA_NOSHARE` optional | — | n/a | — (see S5: TX DMA in the twin) |
| ADC sense | `PC0 BATT_VOLTAGE_SENS ADC1 SCALE(1)`, plus `define HAL_BATT_VOLT_PIN/SCALE …` | **with no ADC pin, `define HAL_USE_ADC FALSE` is required** (measured, step B: `STM32_ADC_ADC12_DMA_STREAM not defined`) | **N** | **`4300d`** (pins and scales) |
| USB | `PA11 OTG_FS_DM OTG1`, `PA12 OTG_FS_DP OTG1`, `OTG1` in `SERIAL_ORDER` | optional | **N** | **`4300e1`** |
| LEDs | `PE3 LED0 OUTPUT LOW GPIO(90)` plus `define HAL_GPIO_A_LED_PIN 90` | optional | **N** | **no work order**: declared generic GPIO outputs |
| buzzer | a timer channel with `ALARM` | optional | **N** | `4300a` (timers) plus no work order for the part |
| SD card | `PC8 SDMMC1_D0 SDMMC1` … | optional | **N** | **no work order** |
| `IMU` | `IMU Invensensev3 SPI:icm42688 ROTATION_YAW_180` | optional to build; needed to fly | **N** | P4's IMU work order: `SPIDEV`, driver and rotation (a mechanical fact) |
| `COMPASS` | `COMPASS <drv> I2C:b:addr …` | optional | **N** | no device declared |

**Required to configure and compile, by measurement:** `MCU`, `OSCILLATOR_HZ`, `FLASH_SIZE_KB` and `APJ_BOARD_ID`. Then a bootloader or its absence, at least one USART, `HAL_USE_ADC FALSE` (or an ADC pin) and `HAL_STORAGE_SIZE`. None of these is in the topology.

## 4. S3 — a minimal `hwdef.dat` for Sentinel-01 (measured)

**The input** is `spike/4400/sentinel-01.topology.json`, from `aethelgard solve --file examples/sentinel-01-minimal.json` at `c621461`. Its specification sha256 is `a277c933…1196`.

**The method.** `spike/4400/s3.sh` copies `spike/4400/hwdef/<step>.hwdef.dat` into an untracked board directory, `libraries/AP_HAL_ChibiOS/hwdef/Sentinel01/`, in the clone. It then runs `./waf configure --board Sentinel01` and `./waf copter -j6` under `nice 10` in the capped scope. After every step, `git status --porcelain --untracked-files=no` printed nothing: the clone's tracked files are unmodified. Each step adds only what the previous step's error named.

| step | added | configure | `copter` | the error that named the next line |
|---|---|---|---|---|
| A | the topology's 9 lines plus the 4 required keys | **fails** | — | `Bootloader (./Tools/bootloaders/Sentinel01_bl.bin) does not exist and AP_BOOTLOADER_FLASHING_ENABLED` |
| B | `define AP_BOOTLOADER_FLASHING_ENABLED 0` | ok | **fails** | `#error "STM32_ADC_ADC12_DMA_STREAM not defined"` and `#error "SERIAL driver activated but no USART/UART peripheral assigned"` |
| C | `SERIAL_ORDER EMPTY`, `define HAL_USE_ADC FALSE` | ok | **fails** | `StorageManager.h:30: "HAL_STORAGE_SIZE" is not defined` |
| D | `define HAL_STORAGE_SIZE 16384` | ok | **fails** | `UARTDriver.cpp:514: 'SerialDriver' was not declared` (`EMPTY` alone leaves no serial driver) |
| E | D, with `SERIAL_ORDER OTG1` and PA11/PA12 in place of `EMPTY` | ok | **fails** | the same `#error "SERIAL driver activated but no USART/UART peripheral assigned"` |
| F | D, with `SERIAL_ORDER USART3`, PD8 and PD9 in place of `EMPTY` | ok | **builds**: 48.1 s wall, 280 CPU-s, peak 1.04 GB; 1,286,028 B of flash | — but `FLASH_LOAD_ADDRESS 0x08004000`: the default 16 KB reserve with no bootloader leaves the reset vector empty, so it would not boot |
| **G** | `FLASH_RESERVE_START_KB 0` | ok | **builds**: 48.4 s, 280.5 CPU-s, peak 1.06 GB; 1,285,344 B of flash, 811,804 B free; `FLASH_LOAD_ADDRESS 0x08000000` | — |

**Step G is `spike/4400/sentinel-01.hwdef.dat`.** It has 21 lines, each tagged:
- **9 `[T]`**, emitted from the topology today: the five pin lines, the chip select, `I2C_ORDER`, and `BARO`, whose driver name is a mapping. Also the bus and CS fields of `SPIDEV`, whose speed is `[S]` and whose mode and role are `[N]`.
- **1 `[S]`**: `MCU`.
- **11 `[N]`**: `OSCILLATOR_HZ`, `FLASH_SIZE_KB`, `APJ_BOARD_ID`, the bootloader define, the logger role, `SERIAL_ORDER` and the two USART pins, `HAL_USE_ADC FALSE`, `HAL_STORAGE_SIZE` and `FLASH_RESERVE_START_KB`.

Notes on the board:
- **The USART3 pins (PD8/PD9) are my choice,** the Nucleo's ST-LINK console. No declared fact chose them.
- **The TMP117 has no line** (§3).
- **The oscillator value 8 MHz is a guess,** the Nucleo's.

## 5. S4 — the pinout check (`INV-07`)

**What it is.** `spike/4400/pincheck.py` takes the topology, ArduPilot's build directory and the toolchain:
- **Expected values** come only from `SolvedTopology` (`assignments` and `gpio`).
- **Observed values** come from ArduPilot's own computation, which it makes from its own table, in two decodes:
  - (1) the **308 bytes of `gpio_default_config` in the linked ELF**: the 11 ports × `MODER OTYPER OSPEEDR PUPDR ODR AFRL AFRH` that `gpio_init` writes at reset, found by `nm -S` and read by `objdump -s`;
  - (2) the `VAL_GPIOx_MODER/AFRL/AFRH` initialisers in `hwdef.h`.
- **It never reads the `hwdef.dat` the generator wrote.** For each net it checks that the solved pin is in alternate mode at the solved AF, or is an output for a `gpio` entry.
- **In reverse,** it lists every pin in alternate mode that the topology does not name.
- **It requires the two decodes to agree.**

**Measured on step G and three seeds.** All four images built. ArduPilot raised no error for any of them, because every seed is legal `hwdef.dat`.

| board | check | what it named |
|---|---|---|
| step G (clean) | **exit 0** | 5 AF lines and 1 GPIO match in both decodes. Outside the topology: `PD8 AF7`, `PD9 AF7` (the USART the build needed). The decodes agree. Hand decode of the ELF: GPIOA `MODER` = `0x0000A801`, so PA0 is an output and PA5–7 are alternate |
| `seed-pin`: `PB8 I2C1_SCL I2C1` in place of PB6 (PB8 carries `I2C1_SCL` on AF4 too) | **exit 1** | `I2C_SCL (I2C1_SCL): solved pin PB6 is input, not alternate AF4` in both decodes; outside the topology: `PB8 AF4` |
| `seed-af`: PB6/PB7 as `I2C4_SCL/SDA`, `I2C_ORDER I2C4` (AF6 in the table) | **exit 1** | `I2C_SCL (I2C1_SCL): PB6 is on AF6, the topology solved AF4`, and the same for `I2C_SDA` at PB7, in both decodes |
| `seed-pkg`: `PI0 SEED_GPIO OUTPUT LOW GPIO(1)`; PI0 does not exist on the LQFP144 package | **exit 0** | nothing. **ArduPilot builds a board on a pin the package lacks**, and the forward check cannot see it |

**`hwdef.dat` cannot carry an AF number.** "Seed a wrong AF into the `hwdef.dat`" as the work order words it is therefore done by naming a function whose table AF differs on the same pin, as `seed-af` does.

**How independent the two tables are** [measured, `spike/4400/tablediff.py`]. It compares ArduPilot's `STM32H753xx.py` `AltFunction_map`, read as a literal and never imported, with the `STM32_open_pin_data` files the solver vendors (`STM32H753ZITx.xml` and `GPIO-STM32H747_gpio_v1_0_Modes.xml`), over the 112 GPIO pins of the LQFP144 package:
- **597 (pin, signal) pairs share a name, with 0 AF disagreements.**
- ArduPilot has 924 entries on those pins and ST has 830 with an AF.
- In the families a flight controller uses (I2C, SPI, USART, UART, TIM, OTG, SDMMC, CAN, QUADSPI, LPUART), 22 of ArduPilot's 372 entries differ from ST after normalising `CANn` to `FDCANn` and `OTG_FS` to `USB_OTG_FS`:
  - 12 are name merges: `USARTn_CTS_NSS`, `TIMx_BKIN_COMP12`;
  - **10 are CAN entries on pins where ST's data has no FDCAN signal at all:** PA9, PA10, PB7, PD4, PD5, PD6, PD9, PD10, PE0 and PE1, all AF9. They are hand-added lines with different indentation (e.g. `STM32H753xx.py:352` `"PB7:CAN2_TX"`, `:674`, `:724`).
- **Every I2C, SPI, USART/UART data line, OTG and SDMMC entry equals ST's.**
- ArduPilot's header says the table was extracted from the datasheet by tabula (`af_parse.py`). That is an independent extraction from a different ST product (the datasheet PDF against ST's XML), but not an independent source: both are ST.

### The proposal

1. **The check.** After `./waf copter`, read `gpio_default_config` from the ELF. Decode (1) is the primary one, because it is what the hardware receives. Decode (2) is a cross-check that refuses on disagreement.
   - Judge every topology net forward: the mode, the AF, and output for `gpio`.
   - Judge in reverse: every pin whose `MODER` is not its reset or `DEFAULTGPIO` state must be named by the topology, or by a declared non-topology line (USART, USB and so on, as the `4300` pieces arrive).
   - Judge the package: every pin the image configures must exist in `STM32H753ZITx.xml`, a file the solver holds and ArduPilot does not. This closes `seed-pkg`.
2. **Where it runs.** In Rust (`RUL-12`), as a gate in `aethelgard-eda` or a new consumer crate, reading the ELF with a small ELF symbol reader. It needs only `nm`/`objdump`-equivalent logic, not the toolchain.
3. **What it cannot see:**
   - pins ArduPilot reconfigures at run time (the I2C bus-clear GPIO toggle, the `ALT(n)` sets in `HAL_PIN_ALT_CONFIG`);
   - `OSPEEDR`, `PUPDR` and `OTYPER` against `4300c`'s speeds, which is possible later from the same bytes;
   - whether the I2C driver uses the bus the `BARO` line names;
   - and AF agreement between two ST-derived tables is not proof against ST's own error.
4. **`INV-07` holds.** Expected values come from the topology. Observed values come from ArduPilot's own computation, never from the emitted `hwdef.dat`.

## 6. S5 — ArduPilot in the twin (read only, not chased)

Read by a subagent in Renode 1.17.0's source (`~/.cache/aet-2301/src`) and ChibiOS/ArduPilot at the tag. Key points were spot-checked by me.

**The verdict: as generated, the banner would not reach a UART** [read + inferred]. The clock and power start-up is not the blocker; output is.
- **Every busy-wait on the path is satisfied by the model** [read]:
  - `PWR_CSR1.ACTVOSRDY` and `PWR_D3CR.VOSRDY`: the platform's PWR is a Python stub returning `0x6000` and `0x2000` (checked: `stm32h7.repl:728`);
  - `HSIRDY`, `HSERDY`, `HSI48RDY` and `PLL1-3RDY`, which the RCC model mirrors from the enables;
  - `SWS`;
  - the read-back of `FLASH_ACR.LATENCY`.
- **The compensation cell is never touched.** No `CCCSR` access exists in ChibiOS or ArduPilot outside ST's headers (checked by grep). `4300` S5's hanging `READY` wait does not arise. My round-1 prediction was wrong.
- **The banner.** It is `DEV_PRINTF("\n\nInit %s…")` (AP_Vehicle.cpp:318) to `hal.console`, which is SERIAL0. On a board whose SERIAL0 is `OTG1` it waits for USB enumeration. On S3's board SERIAL0 is USART3, and `hwdef.h` gives it a TX DMA stream (`STM32_UART_USART3_TX_DMA_STREAM STM32_DMA_STREAM_ID(1, 2)`, measured). Renode's `STM32F7_USART` has only a receive DMA request: `DMAT` is an inert flag (`STM32F7_USART.cs:187`, checked). So TX by DMA would stall [inferred].
- **What would get past it (not run):** `NODMA` on the console UART, or the USB route through `USB_UART` that `4300` measured with Zephyr.
- **Also unverified:**
  - the OTG model's reset and flush waits;
  - the TIM-based tickless timer at Renode's fixed 250 MHz, which is self-consistent but skewed against TIMCLK;
  - loading an ELF at `0x08000000`.
- Neither Renode's tree nor its tests mention ArduPilot or ChibiOS.

## 7. Results beside predictions

| prediction (round) | result |
|---|---|
| tag `Copter-4.6.x` or `4.7.x` (1) | `Copter-4.7.2` ✔ |
| toolchain 10-2020-q4-major from firmware.ardupilot.org, about 150 MB (1) | ✔ 156.9 MB, 693 MB unpacked |
| host Python 3.14 fails (1) | ✘ it worked. No uv Python, and one fewer host |
| no host compiler needed (1) | ✔ `make` and `rsync` are looked for (present) |
| disk about 3 GB (1) | about 2.5 GB |
| configure about 30 s; copter 4–8 min wall, 20–35 CPU-min, `-j6` (1) | ✘ 0.6 s; **60 s wall, 5.5 CPU-min**. I overestimated about 5× |
| peak memory 2–4 GB (1) | ✘ 1.23 GB |
| hosts: github, firmware.ardupilot.org, pypi, uv's host (1) | ✔ the first three; there was no uv host |
| build phase offline (1) | ✔ 0 network syscalls (traced) |
| `--board` has no out-of-tree option; untracked dir leaves tracked files clean (1) | ✔ (`--extra-hwdef` only appends) |
| configure fails on a missing required field (1) | ✔ partly. It failed on the bootloader embed, which I did not name |
| configure passes with the 4 keys; copter fails without `SERIAL_ORDER` (2) | ✘ configure first needs the bootloader define; ✔ copter fails on serial, and also on ADC DMA (not predicted) |
| with `SERIAL_ORDER` (`EMPTY` or `OTG1`) it builds (2) | ✘ both fail; **one real USART is required** |
| storage not needed (2) | ✘ `HAL_STORAGE_SIZE` is required |
| at most 9 of about 16 lines from the topology (2) | 9 of 21 |
| the check names the seeds as predicted (2) | ✔ both, with the exact wording predicted |
| more than 98 % AF agreement; a handful of disagreements, `PB7:CAN2_TX` among them (2) | ✔ 0 AF disagreements in 597 shared pairs; 10 unsupported CAN lines, `PB7:CAN2_TX` among them |
| ArduPilot accepts a pin the package lacks (2) | ✔ PI0 builds |
| S5: no banner, blocked by `CCCSR`, VOS or PLL waits (1) | ✘ for the reason: all of those waits are modelled and `CCCSR` is never touched. ✔ for "no banner" (TX DMA), by reading, not measured |

## 8. Proposed staging of the build behind the `4300` pieces

**Today, with no `4300` piece,** the generator could emit S3's 9 `[T]` lines. A board that builds needs 11 more, none of which comes from the topology. The pieces add as follows:
1. **`4400a`, now (no `4300` dependency):**
   - Small topology additions copied by `solve`: `mcu`, plus the SPI bus speed. The MCU and the SPI speed are already declared; `solve` copies them, as `2203` did for devices.
   - Parts-table columns: ArduPilot driver (`BMP280`), `SPIDEV` mode and role (`dataflash`), and the MCU's flash size.
   - The generator, the pinout check (§5, in Rust), and the second image with CI's ArduPilot build.
   - The `[N]` lines with no source are fixed generator constants, named in the emitted file's header: no bootloader, reserve 0, storage size, and `HAL_USE_ADC FALSE` while there is no ADC.
   - **What it cannot satisfy is the required USART**, so `4400a` either waits for `4300b`, or emits a fixed console USART as a declared constant (decision D2).
2. **After `4300b`** (serial links): `SERIAL_ORDER` and the UART pins come from the topology. The constant console goes.
3. **After `4300a`** (motor outputs): `PWM(n)` lines, `BIDIR`, and `STM32_ST_USE_TIMER` chosen off the motor timers.
4. **After `4300d`** (power parts): ADC sense pins, `SCALE`, `HAL_BATT_*`. `HAL_USE_ADC FALSE` goes.
5. **After `4300e1`** (USB static): `OTG1` pins, and `OTG1` first in `SERIAL_ORDER`.
6. **After P4's IMU work order:** `SPIDEV` plus `IMU` with rotation.
7. **Not covered by any work order yet:** the oscillator as a declared fact (`OSCILLATOR_HZ`), LEDs and buzzer as declared GPIO, the SD card, the `APJ_BOARD_ID` registry. Until the oscillator lands, `OSCILLATOR_HZ` is a guess, which `INV-06` does not allow in an emitted file. That is D1.

**P4 exit clause 3** ("builds unmodified ArduPilot, and the pinout check passes") is reachable at step 1 with D1 and D2 ruled, and grows with each piece.

## 9. Decisions needed

- **D1 — the oscillator.** `OSCILLATOR_HZ` is required and nothing declares it.
  - ArduPilot's H7 configuration does support no crystal: `OSCILLATOR_HZ 0` selects HSI as the PLL source, giving 400 MHz (`stm32h7_mcuconf.h:113-119`, read; I corrected an earlier inference here). So the choice is a declared fact either way:
    - **(a)** the specification gains an HSE frequency (`mcu.hse_hz`, `0` for none), copied into the topology;
    - **(b)** `4400a` emits `0` as a fixed constant and the solver stops reserving PH0/PH1.
  - Neither may be a guess (`INV-06`). S3's 8 MHz is the Nucleo's and is a placeholder.
  - **Recommended: (a)**, a small schema bump, with `4400a`. A flight controller wants a crystal for USB and UART baud accuracy [inferred].
- **D2 — the required USART before `4300b`.** ArduPilot cannot build without one (measured). Options:
  - **(a)** `4400a` waits for `4300b`;
  - **(b)** `4400a` emits a fixed console USART whose pins the solver must reserve, a declared constant;
  - **(c)** reorder so that `4300b` lands before `4400a`.
  - **Recommended: (c)**, because `4300b` is already ruled before `4400` (`4300` S7).
- **D3 — the bootloader and storage.** Either no bootloader (`AP_BOOTLOADER_FLASHING_ENABLED 0`, `FLASH_RESERVE_START_KB 0`, as built here), or build ArduPilot's bootloader from a generated `hwdef-bl.dat` (the twin and the factory flash both get simpler with none). Also: the storage size, and whether `STORAGE_FLASH_PAGE` is emitted. **Recommended: no bootloader for P4,** with the storage page set to the last sector.
- **D4 — `APJ_BOARD_ID`.** A name must be in ArduPilot's `board_types.txt`, which would be a change to ArduPilot, not "unmodified". A number is refused under `GITHUB_ACTIONS`, which our CI sets. Options:
  - **(a)** unset `GITHUB_ACTIONS` for the ArduPilot build step (it gates only ArduPilot's own CI checks) [inferred, not run];
  - **(b)** use an existing registered name;
  - **(c)** apply for an ID upstream.
  - **Recommended: (a), measured first in `4400a`.**
- **D5 — where the build lives.** The second image (§2), the tools-image layer, or Furnace only. **Recommended: the second image.**
- **D6 — I2C speed.** ArduPilot has one global I2C clock (`HAL_I2C_MAX_CLOCK`), not one per bus. Emit the minimum of `i2c_speeds`, or refuse a topology whose buses differ?
- **D7 — the pinout check's scope.** Forward, reverse and package, from the ELF bytes with `hwdef.h` as a cross-check (§5). Should the package check use the solver's own `STM32H753ZITx.xml`, as proposed? It is independent of ArduPilot, but it is the solver's input.
- **D8 — S5.** The twin booting ArduPilot is reachable only with a console UART without TX DMA, or USB. Pursue in a later work order, or drop? **Recommended: a later work order,** after `4300e1` (USB through `USB_UART`), not now.
- **D9 — the 10 unsupported CAN lines** in ArduPilot's H743/H753 tables. Report them upstream (a public issue, Scotty's call), or only make our check refuse CAN on those pins when CAN arrives?

## 10. What a build deletes

`spike/4400/` (`roadmap.md` §6). The tables above are the record. Measurements a build relies on return as Rust tests (`RUL-12`): the pinout check and its three seeds, and the table comparison if D7 keeps it.
