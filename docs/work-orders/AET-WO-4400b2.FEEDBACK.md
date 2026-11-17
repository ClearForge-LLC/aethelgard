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
- **Two facts are verbatim copies** of a generated `define`: the I2C clock and the bauds. Their judge's expected value is the topology's, so a wrong generated line is refused, which is what A3 measures.
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
