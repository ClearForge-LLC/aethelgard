# AET-WO-4400b2 FEEDBACK — the identity judge

**Builder:** Claude (strong tier, the `4400b` builder), worktree `~/aethelgard-4400b2`, branch `wo/aet-4400b2-identity` from `main` `56318a5`.
**Cadence:** build, with one measured stop (§1.1). The work order is read from `origin/claude/argon-repo-docs-synthesis-rjrb0d` (docs PR #195) and is not committed here.

Labels:
- **[read]**: at source, this repository or ArduPilot `Copter-4.7.2` (`fb773567`), copied read-only out of the pinned image `160f9955…`.
- **[measured]**: run on Furnace in that image, under `capped-run --mem 8G` (`memory.swap.max` 0), `--network none`.
- **[inferred]**: neither.

Predictions (§1) were committed before any judged build and are never edited afterwards.

## 0. §1.1: where ArduPilot's own computation of each fact can be read

Two generated boards (`sentinel-02`, `serial-dshot-9600`) were built from the committed goldens, and their ELFs read with `arm-none-eabi-nm -S` and `objdump -s` [measured].

| fact | in the ELF as bytes? | in `hwdef.h`? | input chosen, and why |
|---|---|---|---|
| each SPI device's CS, bus, mode, low and high speed | **no.** `ChibiOS::SPIDeviceManager::device_table` is in `.bss` (`B`, 0x50 and 0x28 bytes) and is filled at start-up by the constructor `_GLOBAL__sub_I__ZN7ChibiOS16SPIDeviceManager12device_tableE`: `SPIDesc` has a non-`constexpr` constructor (`SPIDevice.h:66-75`, `SPIDevice.cpp:75`). The values are immediates in that function's code, not a table | **yes.** `#define HAL_SPI_DEVICEn SPIDesc("name", bus, devid, PAL_LINE(GPIOx,nU), SPIDEV_MODEm, low, high)` and `HAL_SPI_BUS_LIST` of `HAL_SPIn_CONFIG { &SPIDn, n, … }`. `chibios_hwdef.py:1451-1485` computes them: the bus *index*, and the CS label resolved to its pin's `PAL_LINE` | **`hwdef.h`**: the information exists only as preprocessor text |
| each motor's timer, channel and `PWM(n)` | **no.** `ChibiOS::RCOutput::pwm_group_list` is in `.bss` (0x110 bytes), filled by `_GLOBAL__sub_I__ZN7ChibiOS8RCOutput14pwm_group_listE` | **yes.** `HAL_PWM_GROUPn { adv, {c0, c1, c2, c3}, …, &PWMDt, t, …, { PAL_LINE(…) × 4 }}` (`RCOutput.h:339-…`), which ArduPilot computes from the `PWM(n)` pin lines and their timer | **`hwdef.h`** |
| `HAL_I2C_MAX_CLOCK` | **no.** It is an immediate in `I2CDeviceManager`'s initialiser (`I2CDevice.cpp:209`); `businfo` is `.bss` | **yes, as a verbatim copy.** `define` lines are written through unchanged (`chibios_hwdef.py:1004-1010`). With no line, ArduPilot's default is 100,000 (`I2CDevice.cpp:53-54`) | **`hwdef.h`**: it is the macro the compiler reads. Note: for this fact `hwdef.h` holds the generated `define` verbatim. That is why the judge holds it against the **topology** (the minimum declared bus speed), never against the generated file, which is the A3 trap the work order names |
| the serial order (and each port's pins) | **yes.** `ChibiOS::UARTDriver::_serial_tab` is `const` in flash (`T`, 0x84 = 132 bytes = 3 × 44, `UARTDriver.cpp:53`). Decoded on `serial-dshot-9600`, entry 0 is: `0x300163d8` = `SD1`, `instance` 1, `is_usb` 0, DMA fields, `tx_line` `0x5802040e` = GPIOB + 14, `rx_line` `0x5802040f` = PB15; entries 1 and 2 are all zero (`{}`) | yes, `HAL_SERIAL_DEVICE_LIST` | **the ELF**: the table is bytes in it, as `gpio_default_config` is. The layout is `SerialDef` (`UARTDriver.h:58-80`) with `HAL_UART_NODMA` undefined: 44 bytes; `instance` at 4, `is_usb` at 5, `tx_line` at 20, `rx_line` at 24 |
| each port's baud (A3's `DEFAULT_SERIAL0_BAUD`) | **no table.** The defaults sit in `AP_SerialManager`'s parameter table as floats; they were not decoded [inferred: not practical] | **yes, as a verbatim copy** (`define`) | **`hwdef.h`**, against the topology's declared baud, as for the I2C clock |
| the barometer probe | **no.** `HAL_BARO_PROBE_LIST` expands to code (`AP_Baro.cpp:664-666`) | **yes.** `HAL_BARO_PROBEn probe_i2c_dev(AP_Baro_<driver>::probe, <bus index>, <address>)`. `hwdef.py:591-…` computes the bus index (from `I2C_ORDER`) and the decimal address | **`hwdef.h`**; the bus index resolves through `HAL_I2C_DEVICE_LIST`'s `HAL_I2Cn_CONFIG { &I2CDn, n, … }` order |

**No fact is readable only from the generated `hwdef.dat`.** Every one is in ArduPilot's `hwdef.h` or in the ELF, so §1.1's stop does not fire.
- **Corrected after the adversarial pass (§4, A2):** more of `hwdef.h` echoes `hwdef.dat` than this table first said.
  - **Verbatim echoes** [read at source, `chibios_hwdef.py:1004-1010`, `1462-1484`]:
    - the I2C clock;
    - the bauds;
    - each `SPIDesc`'s mode token and both speeds (only their form is checked);
    - the barometer's driver name.
  - **What ArduPilot computes:**
    - the SPI bus index and the CS's `PAL_LINE`;
    - the PWM channel map and timer;
    - the barometer's bus index and decimal address;
    - the serial table.
  - Every echo's expected value is the topology's, so a wrong generated line is refused, which is what A3 measures.
  - **The I2C clock is witnessable in the ELF after all:** exactly one H7 `TIMINGR` literal, `0x00707CBB` at 100 kHz or `0x00300F38` at 400 kHz [measured by the adversary]. It is not read; see §5 D1.
- **Each table is read through the list macro the compiler expands** (`HAL_SPI_DEVICE_LIST`, `SPIDevice.cpp:75`; `HAL_PWM_GROUPS`, `RCOutput.cpp:75`; `HAL_BARO_PROBE_LIST`, `AP_Baro.cpp:666`), not through the numbered macros. The first build read the numbered macros, and a list written with no numbered macro passed (§4, A4).
- **The serial order is judged from ELF bytes,** and needs no `hwdef.h`.

**ArduPilot's tables at the pinned tag** are stable enough to judge [read]:
- `SPIDesc`'s seven-argument form (`chibios_hwdef.py:1482-1484`);
- `HAL_PWM_GROUPn`'s channel map and `&PWMDt` (`chibios_hwdef.py:1993-2082`);
- `probe_i2c_dev`'s form;
- `SerialDef`'s layout.

Anything else is refused by name, never read loosely (`INV-06`).

## 1. Predictions (before any judged build)

All **[inferred]**.
1. **The judge** lives in `aethelgard-pincheck` beside the GPIO judgement, shares no code with `aethelgard-hwdef`, and reports `check: "identity"` findings. It reads `hwdef.h` with the same strict preprocessor reader (`header.rs`), and the serial table from the ELF with the same strict reader (`elf.rs`).
2. **Every generated CI board passes:** `sentinel-01`, `sentinel-02`, `serial-dshot-9600`.
3. **A1** (`PWM(1)` and `PWM(2)` exchanged on `serial-dshot-9600`) is refused twice: TIM1 CH1 and CH2 each carry the wrong motor.
4. **A2 / M1c** (Sentinel-02's two chip selects exchanged) is refused twice: `dataflash`'s CS is PA1 where PA0 was solved, and `icm42688`'s is PA0 where PA1 was solved.
5. **A3's five edits** on the four-console board at 100 kHz are each refused by name:
   - `SERIAL_ORDER USART1 USART3 USART2 UART4`: SERIAL1 and SERIAL2 are each the wrong peripheral;
   - `HAL_I2C_MAX_CLOCK 400000`: above the declared 100 kHz;
   - `BARO BMP280 I2C:0:0x77`: the probe's address is 119 where 118 was declared;
   - `SPIDEV … MODE3 40*MHZ 40*MHZ`: the mode is 3, not 0, and both speeds are above the controller's achieved rate;
   - `DEFAULT_SERIAL0_BAUD 921600`: the console's baud is not the declared 57,600.
6. **The hand boards** (step G and seeds) gain identity findings beside their GPIO ones:
   - `SPIDEV dataflash … MODE3 20*MHZ` is the wrong mode and above the achieved 16 MHz;
   - SERIAL1 is USART3, a port the topology does not have.
7. **F4:** `validate` refuses a stored speed different from `solve`'s. svd-hal's `a_line_no_selectable_speed_serves_is_refused` then sees the topology's refusal (`InvalidTopology`, the speed's own kind inside) for a line no speed serves, and moves to `ir`'s tests. A3's hand-stored `0` on Sentinel-01's SCK (`4400b` A7) is refused at `validate`.

## 2. What was built

- **`aethelgard-pincheck/src/identity.rs`, the judge.** It is called by `check pinout` whenever `--hwdef-h` is given. The verdict gains `identity` (`judged`, or `not judged: no hwdef.h`), and its findings carry `check: "identity"`. It reads:
  - `hwdef.h` through the cross-check's own strict preprocessor reader (`header.rs`). Two accessors are new: `value`, a definition that may be named elsewhere; and `candidates`, every definition the preprocessor may compile, since `HAL_I2Cn_CONFIG` has one per DMA branch and the judge reads only the bus number, which every branch must give alike;
  - the ELF's serial table through the GPIO reader's own checks (`elf.rs`'s `object`, generalised from `read`): one `STT_OBJECT` in read-only allocated `PROGBITS`, located by its section and its `PT_LOAD`, which must agree;
  - the parts table (`aethelgard-ir`), for each part's ArduPilot name.

  It never reads the generated `hwdef.dat`, and the crate's separation test still holds it to `aethelgard-ir` alone.
- **The rules, each from the topology or `ir`:**
  - **SPI:** each topology device whose part has an `ardupilot.spidev` has one `SPIDesc` of that name, on the solved controller, with its CS on the solved `gpio` pin, `MODE` = `DRIVEN_SPI_MODE`, and each speed in `(achieved_hz − 1 kHz, achieved_hz]`. Any other `SPIDesc` is unknown. `MHZ` and `KHZ` are read from `hwdef.h`, where ArduPilot writes them (`chibios_hwdef.py:2459-2460`), and refused in any other form.
  - **Motors:** each motor's `PWM(index+1)` on its `TIMt_CHk`; every other channel of every group disabled (255).
  - **I2C:** `HAL_I2C_MAX_CLOCK`, or ArduPilot's default of 100 kHz, at most the slowest `i2c_speeds`.
  - **Serial:**
    - the order: SERIAL0 the console, then the other ports in the specification's order, each on its solved peripheral number and TX/RX pins; every later entry `{}`;
    - the bauds, for every port but RC input (whose UART `AP_RCProtocol` sets, `4400b` F3): SERIAL0's baud is its define, or 115200 when absent; SERIALn's is its define mapped by the judge's own copy of `map_baudrate`, since it shares no code with the generator, and must be stated.
  - **Barometers:** the topology's barometers (I2C devices whose row is `baro`), in index order, against the probes in order: the row's driver, the solved controller (through `HAL_I2C_DEVICE_LIST`) and the address.
- **F4 (`aethelgard-ir`):**
  - `validate_pin_speeds` re-derives each stored speed with `solve::pin_speed`, the function `solve` itself uses. A different value is `PinSpeedNotSolved`. A line no selectable speed serves is `PinSpeedUnserved`, whose `kind()` is the speed table's own (`RateAbovePinSpeed`, `LoadAboveSpeedTable`, `RiseTimeBelowPinSpeed`, `SupplyOutsideSpeedTable`).
  - It runs last in `validate`, after the part limits, so a bus past its part is still named `SpiClockAbovePart`.
  - `derive_controllers` already re-derives `pin_speeds` for hand-built topologies (`4400b`).
- **What moved for F4:**
  - svd-hal's `a_line_no_selectable_speed_serves_is_refused` (`board.rs:6886`) is removed. Its four cases (rate, load, rise, supply) are now `ir`'s `a_line_no_selectable_speed_serves_is_refused_by_the_topology` (`tests/validate.rs`), refused by the topology itself under the same kinds.
  - Three tests outside the fence changed one line each, forced by F4:
    - `ir`'s `solved()` builder stored `0` for a 20 MHz SCK, where `solve` gives `01`;
    - twin's `what_cannot_be_lowered_is_refused` cleared `spi_lines` and keeps `pin_speeds` at `0` with them;
    - hwdef's `each_pin_takes_the_stored_speed` now asserts the topology's refusal of a hand-edited speed.
  - `solved_holds_the_flash_to_its_clock_after_a_bus_is_sped_up`'s 104 MHz case, which passed `validate`, is now refused `RateAbovePinSpeed`: no selectable speed serves 104 MHz, so `solve` never produced it.
- **CI (`container/ardupilot-pinout.sh`):** every board must report `identity: judged`.
  - Each generated board passes.
  - Each hand board expects its GPIO findings plus, measured, `WrongSpiMode` (MODE3), `WrongSpiSpeed` (low and high, 20 MHz against the achieved 16 MHz) and `UnexpectedSerialPort SERIAL1` (USART3); seed-af also expects `WrongBaroProbe` (I2C4).
- **Tests:**
  - `pincheck/tests/identity.rs`, nine tests: every finding kind, §5's three cases, refusals of unknown table forms;
  - the synthetic `hwdef.h` (`tests/common`) carries the reference board's tables, and the synthetic ELF can carry a serial table;
  - `ir`'s F4 tests.

## 3. §2: the acceptance mutations, each built in unmodified ArduPilot and judged [measured]

Each is a one-line edit to a generated board, the board built in the pinned image (`capped-run`, `--network none`, clean tree) and judged against the board's own topology. The GPIO judge alone finds nothing in any of them; the identity judge refuses each by name.

| mutation | base board | identity findings (expected, then decoded) |
|---|---|---|
| A1: `PWM(1)` and `PWM(2)` exchanged on PA8 and PA9 | serial-dshot-9600 | `WrongMotorChannel TIM1_CH1 [PWM(1) \| PWM(2)]`; `WrongMotorChannel TIM1_CH2 [PWM(2) \| PWM(1)]` |
| A2 / M1c: the two chip selects exchanged | sentinel-02 | `WrongChipSelect SPIDEV dataflash [PA0 (FLASH_CS) \| PA1]`; `WrongChipSelect SPIDEV icm42688 [PA1 (IMU_CS) \| PA0]` |
| A3 `SERIAL_ORDER USART1 USART3 USART2 UART4` | the four-port board at 100 kHz | `WrongSerialPort SERIAL1 [U(S)ART2 TX PA2 RX PA3 \| U(S)ART3 TX PB10 RX PB11]`; the same on SERIAL2, reversed |
| A3 `HAL_I2C_MAX_CLOCK 400000` | same | `I2cClockAboveDeclared [at most 100000 Hz \| 400000 Hz]` |
| A3 `BARO BMP280 I2C:0:0x77` | same | `WrongBaroProbe HAL_BARO_PROBE1 [BMP280 on I2C1 at 0x76 \| … at 0x77]` |
| A3 `SPIDEV … MODE3 40*MHZ 40*MHZ` | same | `WrongSpiMode [MODE0 \| MODE3]`; `WrongSpiSpeed` low and high `[≤ 16000000 Hz \| 40000000 Hz]` |
| A3 `DEFAULT_SERIAL0_BAUD 921600` | same | `WrongSerialBaud SERIAL0 [57600 baud \| 921600 baud]` |
| (base, unmutated) | the four-port board | exit 0 |

The four-port board is `serial-four.json` with its MAVLink port made the console, its SBUS port made a plain GPS port (`4400b` refuses two RC ports), and its bus at 100 kHz. It is solved and generated as committed, and built in 23 s. The generated `sentinel-01`, `sentinel-02` and `serial-dshot-9600` each pass, with `identity: judged` [measured].

## 4. Adversarial pass

A fresh Opus agent worked read-only, with the kit and 13 new boards built in unmodified ArduPilot. WO §5's four cases held [measured]:
- **Swapped chip selects with different modes:** every fact is named (`adv-cs-mode`: `WrongChipSelect` ×2 and `WrongSpiMode`).
- **A motor-map rotation:** four `WrongMotorChannel` (`adv-motor-rot`); 3-cycles and partial maps are each named.
- **No barometer, no I2C, two buses:** each passes (`adv-nobaro`, `adv-noi2c`, `adv-baro2`, `adv-baro2b`).
- **The A3 trap:** every expected value comes from the topology or `ir`, never from `hwdef.h` or `hwdef.dat` [read].

It broke the first build in the ways below. Each fix was then re-judged with the fixed binary [measured]. The reproductions are the adversary's: they were rebuilt from `hwdef.dat`, or are `hwdef.h`/ELF edits.

| | finding | severity | fixed by | after the fix |
|---|---|---|---|---|
| A1 | `#define` with any spacing but one blank was invisible; a `define  HAL_I2C_MAX_CLOCK 400000` (two blanks) builds and ran at 400 kHz | high | one matcher for every define (`header.rs` `defines`), with the blanks GCC allows | `adv-i2c-dbl`: `I2cClockAboveDeclared`; `adv-s01-baud-dbl`: `WrongSerialBaud`; tab and `  #  define` edits caught |
| A2 | five `hwdef.h` facts are not bound to the ELF; a mutated ELF with a clean `hwdef.h` passes | medium | **not fixed:** decision D1 (§5) | |
| A3 | a leading `0` was read as decimal; C reads octal | medium-low | `number` reads C integer constants (hex, octal, suffixes) | `adv-s01-baud-oct`: `WrongSerialBaud [115200 \| 39552, then 39000]`; `adv-spi-oct`: `WrongSpiSpeed` 7 168 000 Hz |
| A4 | `HAL_BARO_PROBE_LIST` (what `AP_Baro` compiles) was never read, only `HAL_BARO_PROBEn`; also SPI and PWM | medium | every table is read through its list macro, names expanded as the preprocessor does | `adv-nobaro-list`: `UnexpectedBaroProbe`; an unlisted numbered macro is not judged (the compiler ignores it) |
| A5 | `HAL_I2C_H7_100_TIMINGR` overridden: the declared 100 kHz bus ran at 400 kHz | medium | `I2cTimingOverridden` for either H7 timing defined in `hwdef.h` (`#ifndef`-guarded at `I2CDevice.cpp:65-69`, read) | `adv-timingr`: `I2cTimingOverridden` |
| A6 | `DEFAULT_SERIALn_PROTOCOL` swapped (RC on the GPS UART) passes | medium | **not fixed:** outside §1.2's five facts; decision D2 | `adv-proto-swap` still exit 0 |
| A7 | SERIAL0 runs at the define, then at `map_baudrate(define/1000)` once the GCS starts; 14400, 76800 and 4 000 000 baud passed | medium-low | both bauds judged (`GCS_Common.cpp:221` and `AP_SerialManager.h:151-153`, read in the pinned image) | `WrongSerialBaud [14400 \| 14400, then 14000]` (tier 1) |
| A8 | no SPI floor: 250 kHz runs at 312.5 kHz (ArduPilot halves at most 7 times) | low-medium | **not fixed:** decision D3 | |
| A9 | a speed past `u64` panicked (exit 101, no JSON) | low (`INV-06`) | `checked_mul`; the window compare cannot overflow | refused by macro name, or a finding |
| A10 | a table entry defined inside an unevaluated conditional was skipped | low | reading through the list: a listed name under a conditional is refused (`HwdefConditionalUnsupported`) | refused |
| A11 | the serial entry's driver pointer was unread (`instance` names only the thread); its padding was unchecked | low | `driver_is_instance` (the pointer is the `SD<instance>` symbol) and zero padding (bytes 14–15, 42–43 of `SerialDef`, `UARTDriver.h:58-80`) | `fc-ptrswap.elf`: `WrongSerialPort` ×2; `fc-pad.elf`: `MalformedElf` |
| A12 | SPI realised rate: ArduPilot's divider on its 80 MHz kernel runs 16 MHz at 10 MHz | info | ruled residual (`4400b` D6: request ≤ achieved); the window is the generator's rule restated | |
| A13 | no CI ArduPilot build exercises a motor or CS mismatch | info | covered in tier 1 only; the mutations of §3 were measured locally | |

**Prediction check (§1).** Predictions 1 to 7 held [measured], with two qualifications:
- Prediction 1's "same strict reader" missed A1, A3, A4 and A10.
- Prediction 7: the moved test's topology refusal carries the speed table's own kinds, not `InvalidTopology`. `InvalidTopology` is what `generate hwdef` reports around it.

**Mutations, one per guard this WO adds or changes** [measured]. Each was run against its crate's suite, and all 11 are killed:
- the define blanks;
- octal;
- list expansion;
- `checked_mul`;
- the speed window's 1 kHz edge (it survived at first; the edges at 1000 Hz and 999 Hz below were then added to `each_spi_fact_is_judged`);
- the timing override;
- SERIAL0's reopen baud;
- the driver in the port match;
- the padding;
- the driver compare;
- F4's `stored != solved`.

## 5. Decisions for the architect

- **D1 (A2): bind the `hwdef.h` facts to the ELF?**
  - **What holds today:** only `MODER`/`AFRL`/`AFRH` tie `hwdef.h` to the firmware. The SPI devices, PWM groups, I2C clock, barometer probes and bauds are judged from `hwdef.h` alone. A mutated ELF paired with a clean `hwdef.h` passes six of §3's seven mutations [measured by the adversary].
  - **Why that is not wrong today:** CI takes both files from one build directory, so CI's verdict is sound. The gap is a hand-paired ELF.
  - **Options:**
    - (a) record it as a residual: `check pinout` judges a build directory, and the README says so;
    - (b) a follow-up that witnesses each fact in the ELF where it can: the I2C clock by its single `TIMINGR` literal, the SPI and PWM tables by decoding their constructors (not attempted);
    - (c) take the ELF and `hwdef.h` as one input, the build directory.
  - **Recommended:** (a) now, and (b) for the I2C clock only if wanted.
  - The READY text does not claim the `hwdef.h` facts are bound to the firmware.
- **D2 (A6): judge each port's protocol and options?** A sixth fact, outside §1.2.
  - **The rule is simple:** `DEFAULT_SERIALn_PROTOCOL` from the role; `_OPTIONS` from `inverted`. But the options rule is unmeasured.
  - **Recommended:** build it in this PR on a ruling. Otherwise it is a follow-up.
- **D3 (A8): an SPI floor in the judge?**
  - The generator refuses a speed below ArduPilot's slowest divider (`SpiBelowArduPilotDivider`), so the gap is reachable only by editing `hwdef.h`.
  - A floor needs the MCU's cited kernel in the judge's signature, and a second copy of a generator rule.
  - **Recommended:** backlog.

## 6. Reduction pass (diff)

- **Removed:**
  - `Preprocessed::numbered`, now dead with the lists read;
  - two `strip_prefix("#define ")` matchers, now one `defines`;
  - two number-or-refuse blocks, now `defined_number`.
- **Protected:** the judge's own `map_baudrate`, labelled `belt-and-suspenders:` because the generator keeps its own (`RUL-20`).
- **Finding:**
  - `header.rs` has two value readers, the GPIO cross-check's free `value` (which also holds a macro to being named once) and `Preprocessed::value`. They differ in that rule and were left apart.
  - Its `number` reads only `nU` pins and AFs, a different domain from the judge's C constants.

Reduction pass (diff): −13 lines (`numbered`), against +127 net for the adversarial fixes in pincheck's sources [measured, `git diff --numstat`] · 2 duplicates removed · 1 dead path removed · 1 protected kept · 1 finding above.

## 7. Gates

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` (126 test binaries) all pass [measured].
- No tier-2 test was added, so `TIER2_MIN` stays at 162.
- No specification version moves.
