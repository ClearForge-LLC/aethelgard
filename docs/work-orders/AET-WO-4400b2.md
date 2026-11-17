# AET-WO-4400b2 — The identity judge: `check pinout` holds ArduPilot's device tables against the solved topology

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4400b2-identity`.
**Author:** Claude (architect) · **Builder:** strong tier (the `4400b` builder, reused), with a full adversarial pass.
**Phase:** P4 (exit clause 3: "builds unmodified ArduPilot, and the pinout check passes"). Written 2026-11-17 from `AET-WO-4400b`'s second stop. **Cadence: build, with one measured stop (§1.1).**

**Grounds (on `main`):**
- **`AET-WO-4400b` FEEDBACK §14 (F1) and §11 (A1–A3)**, measured. Each of these builds unmodified ArduPilot and passes `check pinout` with exit 0:
  - **A1:** two motor outputs swapped (`PWM(1)` and `PWM(2)` exchanged on PA8 and PA9). `hwdef.h`'s `HAL_PWM_GROUP1` channel map becomes `{1, 0, 2, 3}`.
  - **A2 / M1c:** two chip selects swapped.
  - **A3:** one-line edits to the four-console board:
    - `SERIAL_ORDER USART1 USART3 USART2 UART4`;
    - `HAL_I2C_MAX_CLOCK 400000`;
    - `BARO BMP280 I2C:0:0x77`;
    - `SPIDEV … MODE3 40*MHZ 40*MHZ`;
    - `DEFAULT_SERIAL0_BAUD 921600`.

  ArduPilot writes every `define` and `STM32_` line through verbatim (`chibios_hwdef.py:1004-1010`). A judge that compares the generated file with ArduPilot's copy of it judges nothing.
- **The ruling on #189 (comment 6938902594), F1 (a):** a `hwdef.h` judge in `aethelgard-pincheck`, as this follow-up. #189 merged with §2's third bullet narrowed to GPIO and alternate-function swaps.
- **F4, ruled yes:** `validate` re-derives `pin_speeds`, with svd-hal's `a_line_no_selectable_speed_serves_is_refused` (`crates/aethelgard-svd-hal/src/board.rs:6886`) moved to the topology's refusal.

> **What this is:** the second half of `INV-07`'s independent check for ArduPilot boards. It covers the facts that live in ArduPilot's device tables rather than in the GPIO image:
> - which chip select reaches which device, and in which SPI mode and at what speed;
> - which timer channel drives which motor;
> - the I2C clock;
> - the serial order;
> - the barometer probe.
>
> Each is judged from ArduPilot's own computation against the solved topology, and refused by name.
>
> It is NOT:
> - a change to the generator's output (it is judged, not re-made);
> - a twin observation (`4400c`);
> - a change to ArduPilot's tree.

## 1. Build

1. **Measure first (the stop).** For each fact above, say where ArduPilot's own computation of it can be read, and which input is truer to the firmware that runs:
   - **(i)** the `hwdef.h` ArduPilot writes during the build (`HAL_SPI_DEVICEn … PAL_LINE(GPIOx,n)`, `HAL_PWM_GROUPn` channel maps, `HAL_I2C_MAX_CLOCK`, `HAL_SERIAL_DEVICE_LIST` or its equivalent, `HAL_BARO_PROBEn`);
   - **(ii)** the ELF's copies of those tables, where they exist as bytes.

   Prefer the ELF where a table is bytes in it, as `check pinout` reads `gpio_default_config`. Use `hwdef.h` where the information only exists as preprocessor text. Name which for each fact, and why.

   **STOP** if any of the five can be read only from the generated `hwdef.dat` itself: that would judge the generator against itself (`INV-07`).
2. **The judge, in `aethelgard-pincheck`.** It shares no code with `aethelgard-hwdef`. Its rules come from `ir` and the topology, never from the generator's module. It judges:
   - **each SPI device:** its chip-select pin, its bus, its mode (the driven mode, `DRIVEN_SPI_MODE`) and its low and high speeds against the solved controller;
   - **each motor output:** its timer and channel, and its `PWM(n)` index;
   - **`HAL_I2C_MAX_CLOCK`:** not above the minimum declared bus speed;
   - **the serial order:** the console first, then the topology's order;
   - **the barometer probe:** the part, the bus and the address, where the topology declares one.

   Each mismatch is refused by name, naming the fact, the expected value and the observed value.
3. **F4.** `validate` re-derives `pin_speeds` and refuses a stored speed that differs, at every reader, as it re-derives ceilings and controllers. Move svd-hal's `a_line_no_selectable_speed_serves_is_refused` to the topology's refusal, and say what moved.
4. **CI.** The `pinout` step runs the new judge on each `generated` board and each hand seed, beside today's GPIO judgement.

## 2. Acceptance

- **Each of A1, A2 (M1c) and A3's five edits,** applied to a generated board, builds, and is refused by `check pinout` by name. Record each mutation and its verdict.
- **Every generated board passes,** in CI.
- **F4's refusal has a tier-1 test.**
- **The CLAUDE.md gates pass.** CI is green, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-06`, `INV-07`; `RUL-16`, `RUL-20`; `4400a` D-1, D4; `4400b`'s rulings.
- **Fence:**
  - `aethelgard-pincheck`;
  - `aethelgard-ir` (F4: validation only);
  - `aethelgard-svd-hal` (only the one test F4 moves);
  - `.github/workflows/ardupilot-image.yml` (the `pinout` step);
  - tests;
  - FEEDBACK.
- **Running beside you:**
  - `2302` (CLI, chaos);
  - `3401` (examples, Sentinel-01's board);
  - `1313` (ir's `board` type, `pcb.rs`; specification 15).

  No specification version moves here. Coordinate through the bus inbox.
- One PR, left unmerged. Write `~/.cache/aet-4400b2/pr-title-body.md`.

## 5. Adversarial pass, full

- **A judge that passes because it reads the generator's own file** (the A3 trap): prove every input is ArduPilot's computation, or bytes in the ELF.
- **Two devices whose chip selects are swapped *and* whose modes differ:** both facts are refused, not the first only.
- **A motor map that is a rotation, not a swap,** of the channels.
- **A topology with no barometer, or one bus:** the judge refuses nothing it should not.

## 7. Flag-and-stop conditions

- **§1.1:** a fact readable only from the generated `hwdef.dat`.
- **ArduPilot's build exposes no stable form of a table at the pinned tag.**
