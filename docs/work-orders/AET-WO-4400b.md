# AET-WO-4400b — ArduPilot's board definition generated from the solved topology: unmodified ArduPilot builds it, and `check pinout` passes on the ELF

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4400b-hwdef`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4 (exit clause 3: "builds unmodified ArduPilot, and the pinout check passes"). Written 2026-11-17, when the hold was released (roadmap §5). It is the first of three steps toward flying a simulated Sentinel-02: `4400b`, then `4400c` (ArduPilot booting in the twin), then a flight-loop work order. **Cadence: build, with one measured stop (§1.1).**

**Grounds, on `main`:**
- **`AET-WO-4400`'s spike record** (`AET-WO-4400.FEEDBACK.md`):
  - §3's field table (`hwdef.dat` against the solved topology);
  - §4's step G, a 21-line board that builds unmodified `Copter-4.7.2` (9 lines from the topology, 1 from the specification, 11 from nowhere);
  - §8's staging;
  - §9's decisions.
- **The ruling (comment 6887864517)**, which split the build into `4400a` and `4400b`. `4400b` is:
  - the generator, with no bootloader;
  - `HAL_I2C_MAX_CLOCK` as the minimum I2C clock;
  - `OSCILLATOR_HZ` from `mcu.hse_hz`;
  - the solved pin speeds judged from the same ELF bytes.

  Each later piece emits its own lines.
- **`AET-WO-4400a`** (merged, PR #135):
  - the pinned ArduPilot image, with step G and three seeds built in CI;
  - `aethelgard check pinout` in `aethelgard-pincheck`, reading `gpio_default_config` from the linked ELF;
  - **D-1: the generator never emits `DEFAULTGPIO`**;
  - **D4:** the build step runs with `GITHUB_ACTIONS` unset, so a numeric `APJ_BOARD_ID` is accepted. It is checked in CI by the `D4 control` step;
  - A11: `OTYPER` is not compared yet. That is this work order's.
- **What has merged since the spike and closes its `[N]` lines:**
  - `4300b`: `serial_ports` (`SERIAL_ORDER` and the UART pins);
  - `4300a`: `motor_outputs` (the `PWM(n)` lines, DShot, the motor timers) and `mcu.hse_hz`;
  - `4300c`: each pin's solved output speed;
  - `4230`: the SPI and I2C kernel clocks;
  - `4300d`: the power tree's sense pins, where declared;
  - `1207`: `DRIVEN_SPI_MODE`.
- **The images are pinned** (`container/ARDUPILOT-IMAGE`, re-pinned by `4403` Q3, #186), and their GPL sources are mirrored.

> **What this is:** a generator that writes `hwdef.dat` from the `SolvedTopology` alone (`RUL-16`) and the parts tables. CI builds unmodified ArduPilot from it, for Sentinel-01 and the Sentinel-02-shaped fixture. The independent pinout check (`INV-07`) then passes on the ELF ArduPilot linked.
>
> It is NOT:
> - ArduPilot in the twin (`4400c`);
> - a bootloader;
> - USB (`4300e1`);
> - a change to ArduPilot's tree (the board directory is untracked, as step G's was);
> - a guess in any emitted line (`INV-06`).

## 1. Build

1. **Measure first (the stop).** Redo the spike's field table against today's `SolvedTopology` and parts tables. For every line step G and its successors need, mark where it comes from:
   - `[T]`: the topology;
   - `[P]`: a parts-table row;
   - `[C]`: a fixed generator constant, named in the emitted file's header and in FEEDBACK;
   - `[N]`: nowhere.

   **STOP** with the table if any line a build or `check pinout` needs is `[N]`. The ones to settle:
   - `MCU`: the topology has no MCU field today, so `solve` copies it, a solved-schema bump;
   - `FLASH_SIZE_KB`: a parts row;
   - the `SPIDEV` role and the ArduPilot driver names: parts-table columns, citing ArduPilot's source at the pinned tag;
   - `IMU` rotation: a mechanical fact. If nothing declares it, the `IMU` line is omitted and named, not guessed.
2. **The generator.** Its module and crate are your proposal at the stop. It must not live in `aethelgard-pincheck`, and must share no code with it (`INV-07`). It reads only the `SolvedTopology` and the parts tables, never the `Specification` (`RUL-16`). It emits, in a fixed order, each line with a comment naming its source:
   - **`MCU`, `OSCILLATOR_HZ`** (`0` when `hse_hz` is 0: HSI drives the PLLs, `stm32h7_mcuconf.h:113-119`), **`FLASH_SIZE_KB`**;
   - **no bootloader:** `define AP_BOOTLOADER_FLASHING_ENABLED 0` and `FLASH_RESERVE_START_KB 0`, as step G measured. `HAL_STORAGE_SIZE` and `STORAGE_FLASH_PAGE` (the last sector) are named constants;
   - **`APJ_BOARD_ID`:** a number, as `4400a` D4 measured. Choose one that collides with no registered ID in `board_types.txt` at the pinned tag, and cite the file;
   - **every serial port,** with the console first in `SERIAL_ORDER`;
   - **every SPI bus and chip select:**
     - `SPIDEV` with the solved controller's speeds (low and high, from `spi_controllers`);
     - the driven mode (`DRIVEN_SPI_MODE`);
     - the role from the parts column;
   - **every I2C bus:** `I2C_ORDER` in the topology's order, and `HAL_I2C_MAX_CLOCK` as the **minimum** declared bus speed (the ruling);
   - **`BARO` and `IMU`** lines from the parts columns, where a device's driver exists at the tag. A device with no driver gets no line, and is named;
   - **every motor output** as `PWM(n)`, with DShot's `BIDIR` where declared, and `STM32_ST_USE_TIMER` on a timer no motor uses;
   - **ADC sense pins** with their `SCALE` where the topology has them; otherwise `define HAL_USE_ADC FALSE`;
   - **each pin's solved speed,** in ArduPilot's own per-pin speed token (cite its parser at the tag). Never `DEFAULTGPIO` (D-1).
3. **The check grows** (`aethelgard-pincheck`, judged from the ELF bytes only):
   - each pin's `OSPEEDR` against the solved speed;
   - `OTYPER` (A11) against the topology's open-drain lines;
   - `PUPDR` against what the topology implies.

   Each mismatch is refused by name. The check computes nothing the generator computes.
4. **CI** (`ardupilot-image.yml`, beside step G and the seeds):
   - Generate `hwdef.dat` for `examples/sentinel-01-minimal.json` and for the Sentinel-02-shaped fixture (`sentinel-02-own.json`).
   - Build both with unmodified ArduPilot, with `git status --porcelain --untracked-files=no` empty.
   - Run `check pinout` on each ELF.
   - Run the step-G and seed jobs as today.
5. **Versions:** the solved topology schema takes its next number for `mcu`. `parts.json` takes its next number for its new columns; coordinate with `4220b`, which may touch the same rows. No specification version, unless §1.1 proves one is needed; then stop and say so.

## 2. Acceptance

- **P4 exit clause 3, on both boards:** unmodified ArduPilot builds from the generated file, and `check pinout` passes, including speeds, `OTYPER` and `PUPDR`.
- **Every emitted line names its source;** no line is a guess.
- **A mutation of the generator** (a swapped pin, a wrong speed, a dropped serial port) is caught by `check pinout` on the rebuilt ELF. Record each mutation and its verdict.
- **The CLAUDE.md gates pass.** CI is green, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-06`, `INV-07`; `RUL-12`, `RUL-15`, `RUL-16`, `RUL-20`, `RUL-21`; `4400` D1–D9 as ruled; `4400a` D-1, D4.
- **Fence:**
  - the generator's module;
  - `aethelgard-ir` (`solve` copying `mcu`; the topology schema; the parts columns);
  - `aethelgard-pincheck` (the three new comparisons);
  - `.github/workflows/ardupilot-image.yml` (the two generated builds);
  - the CLI subcommand that writes the file;
  - tests;
  - FEEDBACK.
- **Running beside you:**
  - `1312` (ir, eda, kicad-lib; specification 13);
  - `4220b` (kicad-lib, eda; maybe `parts.json`);
  - `3401` (examples, a board directory);
  - `1801` (the release; it bumps every manifest in a window the architect opens);
  - `2302` (CLI, chaos).

  Coordinate through the bus inbox on `parts.json`, the topology schema and the CLI.
- One PR, left unmerged. Write `~/.cache/aet-4400b/pr-title-body.md`.

## 5. Adversarial pass, full

- **A generated file that builds but whose ELF puts a signal on a pin the topology did not solve,** and does `check pinout` catch it?
- **Two buses at different speeds:** `HAL_I2C_MAX_CLOCK` must not run either faster than declared.
- **A motor timer chosen as `STM32_ST_USE_TIMER`.**
- **A part whose ArduPilot driver name is wrong for its chip ID** (the spike's TMP117 case).
- **An emitted line that ArduPilot's parser ignores silently** (`CH:2884` stores unknown keys). Every emitted keyword must be one ArduPilot reads at the tag.

## 7. Flag-and-stop conditions

- **§1.1 finds a required line with no source.**
- **ArduPilot cannot express a solved fact** (for example a per-bus I2C speed) without a guess.
- **The build needs a change to ArduPilot's tree.**
