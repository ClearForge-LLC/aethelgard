# AET-WO-4400 — ArduPilot's board definition from the solved topology: spike

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4400-hwdef`.
**Author:** Claude (architect) · **Builder:** strong tier.
**Phase:** P4. **Cadence: spike (§1), then STOP for the architect's ruling,** which scopes the build. It may stage the build behind the `4300` pieces each `hwdef.dat` field needs.

**Grounds:**
- **P4's exit gate, clause 3** (roadmap §5): the generated `hwdef.dat` builds unmodified ArduPilot, and the pinout check passes.
- **`RUL-16`:** consumers read the `SolvedTopology`, never the specification. **`INV-07`:** the pinout is checked by something the generator does not compute.
- **What the topology solves today** (`main` at `c621461`): the MCU, the voltage domains, I2C buses and devices, SPI buses and devices with chip selects, and pin assignments with alternate functions. **What it does not solve yet:**
  - timer outputs (`4300a`);
  - UARTs (`4300b`);
  - power and sense parts (`4300d`);
  - USB (`4300e1`);
  - the oscillator and clock tree, as a declared fact.
- **ArduPilot is GPLv3 and is never vendored here.** The build happens outside the repository, from a pinned upstream tag.

## 1. Spike

Probes and clones live under `~/.cache/aet-4400/` on Furnace, with a written record in `spike/4400/`. **Predictions go into FEEDBACK before any run.**

**S1. Can Furnace build unmodified ArduPilot?** Measure, without `sudo` and without system packages:
- clone ArduPilot at a pinned release tag (choose the latest stable Copter tag, and record it and its commit);
- the toolchain ArduPilot pins for ChibiOS boards (its documented `gcc-arm-none-eabi` version), installed in user space or in a container;
- the Python dependencies in a venv;
- `./waf configure --board <an existing H743/H753 board>` and `./waf copter`.

Record wall time, disk, peak memory, and every network host reached. Then propose where the build belongs:
- the pinned tools image (a new layer, built and pinned by CI as the image is today);
- a second image;
- or Furnace only.

Say what CI time it would cost.

**S2. The `hwdef.dat` format.** Read ArduPilot's `libraries/AP_HAL_ChibiOS/hwdef/scripts/chibios_hwdef.py` and the `STM32H743xx.py` pin and AF table at the pinned tag. Read two existing H743/H753 board definitions. Then:
- **The field table:** list every field a flight controller needs (MCU, oscillator, flash, UARTs and their order, SPI devices and chip selects, I2C buses, PWM outputs and their timers, DMA, ADC sense pins, USB, LEDs, buzzer, SD card, `IMU` and `BARO` lines).
- **For each field:** the `SolvedTopology` value it comes from today, or the `4300` piece or other work order it waits on.

**S3. A minimal `hwdef.dat` from today's reference.** Write by hand, as a spike file, the `hwdef.dat` the generator would emit for Sentinel-01's topology: the MCU, I2C1 with the BME280 as `BARO`, SPI1 with the flash, plus the fields ArduPilot requires. Then:
- **Measure** whether `./waf configure --board` and `./waf copter` build with it, or what is missing.
- **Mark** each line as one the generator could emit from the topology today, or not.

**S4. The pinout check (`INV-07`).** Propose a check the generator does not compute:
- **The candidate:** parse the `hwdef.h` that ArduPilot's own `chibios_hwdef.py` writes during the build, with pins and AF numbers from ArduPilot's own table, and compare it pin by pin with the `SolvedTopology`.
- **Measure** on S3's board that the comparison works.
- **Seed** a wrong pin and a wrong AF into the `hwdef.dat`, and show the check names each.

**S5. Read only: does ArduPilot boot in the twin?** Read whether Renode 1.17.0's H753 platform can run a ChibiOS ArduPilot image far enough to print its boot banner on a UART. Do not chase it if it does not.

**Stop** with:
- the field table;
- the build measurements;
- the pinout check's proposal;
- the proposed staging of the build behind the `4300` pieces;
- the decisions needed.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-03`, `INV-06`, `INV-07`; `RUL-12`, `RUL-16`.
- **The spike fence:**
  - `spike/4400/` and this work order's FEEDBACK;
  - ArduPilot's clone and toolchain only under `~/.cache/aet-4400/`, never in the repository;
  - no crate change.
- **Other builders on Furnace:** `1307` (the router) and `4300c` (pin speeds). Never touch their worktrees. Keep each container at 8 GiB or less, and run one container of yours at a time. ArduPilot's build is CPU-heavy, so run it at `nice 10`.
- One PR, left unmerged. Write `~/.cache/aet-4400/pr-title-body.md`. **Push and revoke by the FUR-WO-0301 F7 route.**

## 7. Flag-and-stop conditions

- Furnace cannot build unmodified ArduPilot without `sudo`, a system package or a credential. Stop, with exactly what is missing.
- A network host the build needs is unreachable from Furnace. Stop, naming it.
