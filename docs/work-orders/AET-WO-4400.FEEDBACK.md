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
