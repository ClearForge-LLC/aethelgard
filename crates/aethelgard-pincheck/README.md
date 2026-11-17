# aethelgard-pincheck

The pinout check (`AET-WO-4400a`, ruling D7 on PR #130). It judges what a linked STM32H753 firmware writes to its GPIO registers at reset against the `SolvedTopology` the firmware was meant to realise. The CLI runs it as `aethelgard check pinout` (`crates/aethelgard-cli/README.md`).

**What it reads.**
- **The ELF.** `gpio_default_config`, found in `.symtab` by name, type and size: exactly one `STT_OBJECT` of 308 bytes, which is 11 ports × `MODER OTYPER OSPEEDR PUPDR ODR AFRL AFRH`. ChibiOS's `gpio_init` writes these bytes at reset.
  - The bytes are located through the symbol's section and through the `PT_LOAD` that holds them, and the two must agree.
  - The segment must run where it is flashed (`p_vaddr == p_paddr`), no other segment may flash over the bytes, and the section may not be writable.
  - The reader is a few hundred lines of strict ELF32 little-endian parsing with no dependency; no toolchain runs.
- **`hwdef.h`, optional.** ArduPilot's `VAL_GPIOx_MODER/AFRL/AFRH`. Each must equal the ELF's decode, or the check refuses (exit 5). A disagreement is never a pass.
- **The topology, the package and the allow-list.** These give the expected values. The package is the solver's pin table, passed in by the caller. The allow-list is a declared file of pins outside the topology, each with its mode, its AF and why. The check holds no list of its own and never reads a `hwdef.dat` (`INV-07`).

**What it judges.**
- **forward:** every assignment's pin is alternate at the solved AF, and every `gpio` pin is an output. Since `AET-WO-4400b` (§1.3), every topology pin's drive is also judged from the same bytes:
  - **`OSPEEDR`** against the speed `solve` stored in `pin_speeds` (`WrongSpeed`);
  - **`OTYPER`** open-drain exactly on the I2C lines, `aethelgard_ir::silicon::implied_open_drain` (`WrongOutputType`);
  - **`PUPDR`** against `aethelgard_ir::silicon::implied_pupd`: pull-down on a motor output, no internal pull elsewhere (`WrongPull`).

  The check reads the stored speed and calls these two rules. The generator calls neither, and stores nothing (`RUL-20`); a test in `aethelgard-hwdef` holds that. A topology with no stored speed for a pin is refused (`NoPinSpeed`), never judged as `00`.
- **reverse:** every *configured* pin is named by the topology or the allow-list. An allowed pin is exactly as declared, and the allow-list may not name a topology pin.
- **package:** every configured pin exists in the package.

A pin is **configured** when it is output or alternate and not at its reset state. Reset is RM0433 Rev 9 §11.4.1: `MODER` is analog except PA13–PA15 and PB3–PB4, which are alternate. §11.4.9–10 put both AF registers at 0. Input and analog drive nothing, and ArduPilot writes input to every pin it is not told about.

The exception is a `DEFAULTGPIO` line before the `MCU` line, which sets every pin of ports A–K. With `OUTPUT`, the package check then names every pin the LQFP144 does not bond out, and no allow-list can clear them (`AET-WO-4400a` FEEDBACK, decision for `4400b`).

**The identity judge** (`AET-WO-4400b2`, `src/identity.rs`). When `hwdef.h` is given, it also judges the facts ArduPilot keeps in its device tables rather than in `gpio_default_config`. Each is read from ArduPilot's own computation and held against the topology; the generated `hwdef.dat` is never read. Its findings are `check: "identity"`:
- **Each SPI device the parts table names for ArduPilot,** from `HAL_SPI_DEVICE_LIST` and `HAL_SPI_BUS_LIST` in `hwdef.h`. The table is built by a constructor at start-up, so the ELF holds no bytes of it. The judge holds one `SPIDesc` of the part's `spidev` name on its solved controller, its chip select on its solved pin, the driven mode, and speeds within the kHz at or below the controller's `achieved_hz`. Findings: `MissingSpiDevice`, `SpiDeviceTwice`, `UnknownSpiDevice`, `WrongSpiBus`, `WrongChipSelect`, `WrongSpiMode`, `WrongSpiSpeed`.
- **Each motor output,** from each `HAL_PWM_GROUPS` entry's channel map and `&PWMDt`, also constructor-built. The motor's `PWM(n)` index must sit on its solved timer channel, and every other channel must be disabled. Findings: `WrongMotorChannel`, `MissingPwmGroup`, `UnexpectedPwmOutput`.
- **The I2C clock,** `HAL_I2C_MAX_CLOCK`, or ArduPilot's 100 kHz default. It must not be above the slowest declared bus (`I2cClockAboveDeclared`), and ArduPilot's H7 timings must not be redefined (`I2cTimingOverridden`). `hwdef.h` holds the generated `define` verbatim, so the expected value is the topology's, never the file's.
- **The serial order and pins,** from the ELF's `_serial_tab`: `const` bytes in flash, 44-byte `SerialDef`s. SERIAL0 must be the console, then the specification's order, each on its solved U(S)ART and pins, and every later entry empty. Each non-RC port's baud comes from `DEFAULT_SERIALn_BAUD`, mapped as ArduPilot maps it, with ArduPilot's 115200 for SERIAL0 when absent. Findings: `MissingSerialPort`, `WrongSerialPort`, `UnexpectedSerialPort`, `WrongSerialBaud`.
- **The barometer probes,** from `HAL_BARO_PROBE_LIST` in order. Each must be the part's ArduPilot driver, on its solved controller (through `HAL_I2C_DEVICE_LIST`), at its address. Findings: `WrongBaroProbe`, `MissingBaroProbe`, `UnexpectedBaroProbe`.
- **The IMU probes** (`AET-WO-4400d`), from `HAL_INS_PROBE_LIST` in order (`AP_InertialSensor.cpp:1220-1222`). Each must be `ADD_BACKEND(AP_InertialSensor_<driver>::probe(*this,hal.spi->get_device("<name>"),ROTATION_<x>))` for an SPI device whose part's ArduPilot row is an IMU, in index order: the row's driver and `SPIDEV` name, and a rotation whose matrix is the declared orientation's. The judge holds its own table of ArduPilot's 26 right-angle `Rotation` names and their matrices (`vector3.cpp` at the tag), so either name of one rotation passes (ruled D4); it computes the expected matrix from the solved `orientation` with the board frame x forward, y right, z down through F.Cu (ruled D3), and shares no code with the generator. An IMU with no declared orientation, or whose driver frame is uncited, has no right rotation. The ELF must define the driver's `probe(AP_InertialSensor&, OwnPtr<Device>, Rotation)` under its mangled name (measured in the built ELF). A rotation spelled any other way, or redefined as a macro, is refused. A defined `HAL_SITL_INVENSENSEV3`, which compiles one more probe after the list (`AP_InertialSensor.cpp:1223-1225`), is named. Findings: `WrongImuProbe`, `WrongImuRotation`, `MissingImuProbe`, `UnexpectedImuProbe`, `ImuDriverNotLinked`.

Each table is read through the list macro the compiler expands, with any macro it names expanded in turn. A directive is matched with any blanks GCC allows and with `#` spelled `#` or `%:`; a name the judge reads that a compiled `#undef` or `#pragma` (`push_macro`, `pop_macro`) also names is refused, because it could stand another definition in (`AET-WO-4400d` adversarial); and an integer is read as C reads it (`0x` hex, a leading `0` octal). SERIAL0's baud is judged both at its define and at the define / 1000 mapped, which the GCS reopens it at. A serial entry's driver pointer must be its `SD<instance>`, and its padding zero. A table in a form the reader does not know is refused by macro name (`MalformedHeader`), never read loosely.

**What the identity judge cannot see** (`AET-WO-4400b2` FEEDBACK §5):
- **Only the serial table and the IMU driver's probe symbol are read from the ELF.** The other facts are read from `hwdef.h` alone, so `check pinout` judges an ELF and the `hwdef.h` of the same build, as CI gives them (D1).
- **Each port's protocol and options are not judged** (D2).
- **No SPI speed floor:** the generator refuses a speed below ArduPilot's slowest divider (D3). `HAL_I2Cn_CONFIG`'s two DMA branches must agree on the bus number, which is all the judge reads of them. With no `hwdef.h`, the verdict's `identity` says it was not judged.

**What it cannot see:**
- a symbol table that lies: a `st_value` pointing at a decoy copy of good bytes passes on the ELF alone. Only the `hwdef.h` cross-check catches it, and CI always passes `hwdef.h`;
- which MCU the firmware was built for: an H743 or H757 image decodes like an H753;
- a pin reconfigured at run time, including ArduPilot's `ALT(n)` sets (`HAL_PIN_ALT_CONFIG`);
- a declared allow-list pin carrying a second copy of a topology signal;
- `ODR`, and a pin's drive outside the topology: an allow-listed pin is judged on mode and AF only;
- whether `hwdef.h`'s `OTYPER`, `OSPEEDR` and `PUPDR` agree with the ELF: the cross-check covers `MODER`, `AFRL` and `AFRH`;
- a pull resistor on an input the topology does not name;
- whether a driver uses the bus its line names.

**Separation (D7, `4400b` D8).** This crate depends on `aethelgard-ir` alone. `tests/separation.rs` reads Cargo's resolve and fails if the check reaches any other workspace crate, if anything but the CLI links it, or if the `hwdef.dat` generator (`aethelgard-hwdef`) reaches anything but `aethelgard-ir`.

**Fixtures.** `fixtures/` holds the four `hwdef.dat` files of the `AET-WO-4400` spike: step G and the seeds `seed-pin`, `seed-af` and `seed-pkg`. They are this project's own. `step-g.allow.json` declares the USART3 pins step G needs. CI builds the four in the ArduPilot image and judges them (`.github/workflows/ardupilot-image.yml`). They carry no speed or pull tokens, so since `4400b` each fails on ArduPilot's default speeds (`10`) and pull-ups beside its seed's own findings. The boards that pass are the three `aethelgard-hwdef` generates. No ArduPilot binary is committed.

Licensed `MIT OR Apache-2.0`.
