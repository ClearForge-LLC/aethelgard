# aethelgard-pincheck

The pinout check (`AET-WO-4400a`, ruling D7 on PR #130). It judges what a linked STM32H753 firmware writes to its GPIO registers at reset against the `SolvedTopology` the firmware was meant to realise. The CLI runs it as `aethelgard check pinout` (`crates/aethelgard-cli/README.md`).

**What it reads.**
- **The ELF.** `gpio_default_config`, found in `.symtab` by name, type and size: exactly one `STT_OBJECT` of 308 bytes, which is 11 ports × `MODER OTYPER OSPEEDR PUPDR ODR AFRL AFRH`. ChibiOS's `gpio_init` writes these bytes at reset. The bytes are located through the symbol's section and through the `PT_LOAD` that holds them, and the two must agree. The reader is a few hundred lines of strict ELF32 little-endian parsing with no dependency; no toolchain runs.
- **`hwdef.h`, optional.** ArduPilot's `VAL_GPIOx_MODER/AFRL/AFRH`. Each must equal the ELF's decode, or the check refuses (exit 5). A disagreement is never a pass.
- **The topology, the package and the allow-list.** These give the expected values. The package is the solver's pin table, passed in by the caller. The allow-list is a declared file of pins outside the topology, each with its mode, its AF and why. The check holds no list of its own and never reads a `hwdef.dat` (`INV-07`).

**What it judges.**
- **forward:** every assignment's pin is alternate at the solved AF, and every `gpio` pin is an output.
- **reverse:** every *configured* pin is named by the topology or the allow-list. An allowed pin is exactly as declared, and the allow-list may not name a topology pin.
- **package:** every configured pin exists in the package.

A pin is **configured** when it is output or alternate and not at its reset state. Reset is RM0433 Rev 9 §11.4.1: `MODER` is analog except PA13–PA15 and PB3–PB4, which are alternate. §11.4.9–10 put both AF registers at 0. Input and analog drive nothing, and ArduPilot writes input to every pin it is not told about.

**What it cannot see:**
- a pin reconfigured at run time;
- `OTYPER`, `OSPEEDR`, `PUPDR` and `ODR`, which `AET-WO-4400b` judges against `4300c`'s solved speeds;
- a pull resistor on an input;
- whether a driver uses the bus its line names.

**Separation (D7).** This crate depends on `aethelgard-ir` alone. `tests/separation.rs` reads Cargo's resolve and fails if the check reaches any other workspace crate, or if anything but the CLI links it.

**Fixtures.** `fixtures/` holds the four `hwdef.dat` files of the `AET-WO-4400` spike: step G and the seeds `seed-pin`, `seed-af` and `seed-pkg`. They are this project's own. `step-g.allow.json` declares the USART3 pins step G needs. CI builds the four in the ArduPilot image and judges them (`.github/workflows/ardupilot-image.yml`). No ArduPilot binary is committed.

Licensed `MIT OR Apache-2.0`.
