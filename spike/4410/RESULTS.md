# AET-WO-4410 — §1.1 measurement

Measured 2026-11-23 on `465bd860` + `f6106fae` (the predictions). Binary: `target/debug/aethelgard` built from this branch's clean tree. Every result below was re-taken on a rebuild after the scratch builds: `generate.sh`'s table, and every topology and `hwdef.dat` it wrote, are byte-identical, and so are the `check pinout` results. ArduPilot: the pinned image `aethelgard-ardupilot@sha256:e1809b90…` (`container/ARDUPILOT-IMAGE`), its `/opt/ardupilot` copied out with `podman cp` (no run) to the kit, `fb773567` with ChibiOS `d103983f`. Every ArduPilot citation below is a path in that tree at that tag, read there.

## M1 · `generate hwdef` on every committed specification

`generate.sh` (this directory) solves each of the 35 (`universe.sh`), then generates. Table: `generate-main.tsv`.

| result | boards |
|---|---|
| **ok** (9) | `driver-judges-nothing`, `four-port`, `logger-4600`, `sentinel-01-bench`, `sentinel-02-own`, `serial-dshot-9600`, `sentinel-01-ext-bmp581`, `sentinel-01-minimal`, `sentinel-01` |
| `NoSerialPort` (12) | `connectors`, `d1-24-i2c`, `d3-24-i2c-12-spi`, `four-layer-power`, `four-layer`, `imu-baro-1v8`, `motors-dshot600`, `motors-pwm`, `pair-standin`, `spi-only`, `two-bme`, `sentinel-02-bench` |
| `NoConsole` (2) | `sentinel-02-shaped`, `serial-four` |
| `OscillatorNotCited` (3) | `hse-crystal-16` (16 MHz), `hse-external-8`, `sentinel-02-bench-hse8` (8 MHz) |
| `SenseNotEmitted` (2) | `sentinel-03`, `sentinel-02-power` (`VBAT_SENSE` each) |
| `LabelTakesAfPrefix` (1) | `solve-stress` (`SPI_BUS_0_CS0_NET`, prefix `SPI`): `4400b` F2's refusal, live on a committed board |
| `GpioNotEmitted` (1) | `d1-drv8876` (`U2`) |
| solve refuses (5) | `solve-infeasible-shared`, `solve-pigeonhole-spi`, `solve-thrash-pins` (`Infeasible`); `d2-drv8833` (`PartLimitUndeclared`); `invalid-zero-domain` (`ZeroNominalVoltage`) |

Total: 9 + 12 + 2 + 3 + 2 + 1 + 1 + 5 = 35.

### The next refusal, in a scratch build (reverted; not committed)

A scratch build gated `SenseNotEmitted` and `OscillatorNotCited` behind environment variables (`~/.cache/aet-4410/aethelgard-bypass`); `lib.rs` was restored byte for byte (`git status` clean).

- **Sentinel-03:** `ImuOrientationUndeclared` (`U5`, ICM42688P). Its specification declares no `orientation` for U5 and no `board.forward`.
  - With both added in a scratch copy (`orientation {x: right, z: up}`, as `sentinel-02-own` declares; `board.forward: top`), it **generates**, and every topology key but `specification_sha256` and `spi_devices` is identical to the committed specification's: no pin moves.
  - Its file then writes `define HAL_USE_ADC FALSE  # [T] no power.sense pin` beside a solved sense pin: the defect (1) removes.
  - **Its sense pin is `PF3`, whose only channel is `ADC3_INP5`** (`power.sense.VBAT_SENSE`: `{pin: PF3, position: 13, channels: [ADC3_INP5]}`). This is the adversarial case §5 names.
- **`sentinel-02-power`:** `NoSerialPort`. It cannot generate for a reason outside this work order.
- **`hse-crystal-16`:** `NoConsole`. **It cannot generate after (2) either**: it declares no serial port with the console role. The same holds for `hse-external-8` (`NoConsole`) and `sentinel-02-bench-hse8` (`NoSerialPort`).

## M2 · The ArduPilot line texts

### (3) SWD — built and checked

`PA13 JTMS-SWDIO SWD` and `PA14 JTCK-SWCLK SWD`, appended to `sentinel-01-minimal`'s generated file (`boards/s01swd.hwdef.dat` in the kit), built in the pinned image under `capped-run` (`container/ardupilot-build.sh boards`, exit 0).

- ArduPilot's own `hwdef.h` lists them as `PA13 JTMS-SWDIO SWD AF0` and `PA14 JTCK-SWCLK SWD AF0`, and defines `HAL_GPIO_PIN_JTMS_SWDIO PAL_LINE(GPIOA,13U)`.
- Fields written: PA13 `ALTERNATE`, AF0, `PUSHPULL`, `OSPEED_MEDIUM`, **`PULLUP`**, ODR high; PA14 the same with **`PULLDOWN`**.
- `check pinout` on that ELF with its `hwdef.h`, against `sentinel-01-minimal`'s topology: **ok, no finding**. PA13 and PA14 are at their reset decode (alternate AF0), so they are not in `configured`.

- **The text is ArduPilot's own:** `grep -h '^PA13 JTMS-SWDIO SWD *$' */hwdef.dat` counts 197 (186, and 11 with a trailing space), and PA14's line the same. 98 boards whose own `MCU` line is H7 carry it. **No SWD line in the tree carries a pull or `FLOATING` token** (0 of 394). (`source-A.md` §2 counted 196 and 103 by another method.)
- **The label alone picks the AF:** `JT` is in `af_labels` (`chibios_hwdef.py:33`); `get_alt_function` (`:151-179`) finds `PA13:JTMS-SWDIO` and `PA14:JTCK-SWCLK` at AF0 (`scripts/STM32H753xx.py:247`, `:249`). `SWD` is a type `valid_type` accepts (`:2792`). Its pulls are the script's defaults for those labels (`:407-413`), and they equal RM0433's reset pulls (Rev 9 §11.4.4 p. 542: `GPIOA_PUPDR` resets to `0x6400 0000`, PA13 up and PA14 down; held text line 21320, as `ir/pin_devices.rs:59` cites).
- **An unlisted pin is `INPUT FLOATING`** (`chibios_hwdef.py:47`, `:145`): today's boards turn SWD off at boot, as A11 says.
- **No define turns SWD off** (searched `SWD`, `JTMS`, `SWCLK`, `DBGMCU` in the HAL and `hwdef/common`).
- **Every CI board, with the two lines:** the four hand boards (`pincheck/fixtures/{step-g,seed-pin,seed-af,seed-pkg}`) and the five generated ones (`sentinel-01`, `sentinel-02`, `serial-dshot-9600`, `four-port`, `sentinel-02-noimu`, from this branch's goldens, each byte-identical to `generate`'s output) each had the two lines appended. All nine were built in one capped container (`sh/ap.sh`, 22–46 s each), and every build passed. CI's own `container/ardupilot-pinout.sh` over the eight it judges, against `sentinel-01-minimal`'s solved topology and `step-g.allow.json`: **exit 0, no `::error::`**. Each hand board fails with exactly its expected findings, and each generated board passes, as on `main`. **The SWD §7 condition does not fire.**
- **Also built:** `PA13 JTMS-SWDIO SWD PULLUP` and `PA14 JTCK-SWCLK SWD PULLDOWN` (the pulls written out, the house style of every other line) build and judge the same (`s03adc1`, below).

### (1) The battery monitor — read, then built

Source (`source-A.md` §1; spot-checked lines marked ✓):
- **Pin line:** `<pin> BATT_VOLTAGE_SENS ADC<n> SCALE(1)`. `ADC1`, `ADC2` and `ADC3` are all parsed (`chibios_hwdef.py:2191-2202` ✓). 56 boards write `PC0 BATT_VOLTAGE_SENS ADC1 SCALE(1)` (my grep; CubeOrange `hwdef.dat:72`).
- **The analog pin number is the named ADC's channel,** with no offset for ADC3 (`chibios_hwdef.py:2204` ✓ `analog = p.extra_value('ANALOG', type=int, default=chan)`; NWBLUE_PROH757 `hwdef.dat:105`, `:179` ✓: `PF3 … ADC3`, `HAL_BATT_VOLT_PIN 5`).
- **`SCALE(n)` is the pin's own multiplier** in `HAL_ANALOG_PINS` (`{chan, analog, n*3.30/4096}`), not the divider. With `SCALE(1)` the row is `3.30/4096`.
- **The defines are written by hand as `define` lines** (the script writes none). `HAL_BATT_VOLT_PIN` without `HAL_BATT_VOLT_SCALE` is a compile `#error` (`AP_BattMonitor_Analog.cpp:30-36` ✓). `HAL_BATT_VOLT_SCALE` becomes `AP_BATT_VOLTDIVIDER_DEFAULT`, the `VOLT_MULT` default; ArduPilot computes `(pin volts − offset) × VOLT_MULT` (`:160`). So with `SCALE(1)`, **`HAL_BATT_VOLT_SCALE` is `(top + bottom) / bottom`** exactly.
- **`HAL_BATT_MONITOR_DEFAULT` is `3`, not `4`:** `ANALOG_VOLTAGE_ONLY = 3`, `ANALOG_VOLTAGE_AND_CURRENT = 4` (`AP_BattMonitor_Params.h:17-18` ✓), consumed at `AP_BattMonitor.cpp:568-569` ✓. The work order's "4" (from memory) would select current sensing, which is not declared.
- **`define HAL_USE_ADC FALSE` must go** when a sense is emitted: it swaps in `Empty::AnalogIn` (`HAL_ChibiOS_Class.cpp:88-92`). The default is `TRUE` (`hwdef/common/halconf.h:60-62`).

Built in the pinned image (scratch Sentinel-03: its generated file with U5's orientation and `board.forward` added, `HAL_USE_ADC FALSE` removed, the SWD lines with pulls written out):

| board | sense line | defines | build | `check pinout` |
|---|---|---|---|---|
| `s03batt` | `PF3 BATT_VOLTAGE_SENS ADC3 SCALE(1)` (the solved pin and channel) | `HAL_BATT_VOLT_PIN 5`, `HAL_BATT_VOLT_SCALE 11.0`, `HAL_BATT_MONITOR_DEFAULT 3` | **fails** | — |
| `s03adc3b` | as `s03batt`, plus `define STM32_ADC_USE_ADC12 FALSE` | as above | **fails** | — |
| `s03adc1` | `PC0 BATT_VOLTAGE_SENS ADC1 SCALE(1)` (an ADC1 pin, scratch) | `HAL_BATT_VOLT_PIN 10`, `HAL_BATT_VOLT_SCALE 11.0`, `HAL_BATT_MONITOR_DEFAULT 3` | ok | **ok, no finding** |
| `s03quot` | as `s03adc1` | `HAL_BATT_VOLT_SCALE (109090.0f/9090.0f)` | ok | — |

- **An ADC3-only board did not build at the tag, in either of two probes.** `s03batt` stops at `modules/ChibiOS/os/hal/ports/STM32/LLD/ADCv4/hal_adc_lld.h:339` `#error "STM32_ADC_ADC12_DMA_STREAM not defined"`.
  - ArduPilot's H7 mcuconf enables ADC12 unconditionally (`stm32h7_mcuconf.h:420` `#define STM32_ADC_USE_ADC12 TRUE`) and takes its DMA stream from an ADC1 or ADC2 pin (`:433-438`).
  - `s03adc3b`'s workaround is refused too: `stm32h7_mcuconf.h:420: error: "STM32_ADC_USE_ADC12" redefined [-Werror]`, and the stream is still missing.
  - **No board in the tree is ADC3-only.** Every `hwdef.dat` with an `ADC3` pin also has an `ADC1` pin (NWBLUE_PROH757: `PC2 … ADC1`, `hwdef.dat:106`).
- **The work order's §5 case is live on Sentinel-03:** its sense is solved to `PF3`, `ADC3_INP5` only (`adc_inputs.VBAT_SENSE`: `{PF3, ADC3, INP5}`), because the solver takes ADC1 or ADC3 (`ir/src/pin_devices.rs:2119` `MODELLED_ADCS`, `1211` Q8).
- **With an ADC1-only rule** (scratch: `MODELLED_ADCS = ["ADC1"]`, reverted), Sentinel-03's sense is solved to **`PC0`** (position 26, `ADC1_INP10`). Only `power.sense` and `adc_inputs` differ; no other pin moves. That is still a pin move.
- `s03adc1`'s `hwdef.h`: `HAL_ANALOG_PINS { 10, 10, 3.30/4096 }, /* PC0 BATT_VOLTAGE_SENS */`.
- **`s03adc1`'s clean verdict is not a confirmation of the sense.** It was judged against the topology whose sense is on PF3, while the board put it on PC0, and no finding named it: **today's judge does not see a sense pin at all** (an analog pin decodes as reset, and the topology's `power.sense` is never read). The verdict confirms the SWD lines, the rest of the board and the build only.
- **The `reserved_pins` route,** scratch: reserving Sentinel-03's ADC3-only pins one at a time moves the sense PF3 → PF4 → … → PF10 → **PC0** after eight reservations (PF3–PF10). Only `power.sense` and `adc_inputs` differ. `power.sense` has no pin key (`ir/src/power.rs:329-336`; "a sense is never pinned", `pin_devices.rs`).

### (2) The 16 MHz clock tree — read

Which files: an H753 takes `stm32h7_mcuconf.h` (`hwdef/common/mcuconf.h:56-61`) and ChibiOS `STM32H7xx/hal_lld_type1.h` (`hal_lld.h:146-157`). Spot-checked ✓ at the tag:

| | `OSCILLATOR_HZ 0` (HSI 64 MHz) | `OSCILLATOR_HZ 16000000` |
|---|---|---|
| branch | `stm32h7_mcuconf.h:113-122` ✓ (PLLSRC HSI `:120`); N/Q `:168-182` ✓ | M `:132-138` ✓; N/Q `:184-207` ✓ (`8 MHz, 16 MHz or 32 MHz`, "common clock tree"); PLLSRC HSE by default `:259-260` ✓ |
| PLL1 M, N, Q | 4, 50, 10 | 2, 100, 10 |
| PLL2 M, N, Q | 8, 45, 5 | 2, 75, 6 |
| SPI1–3 (`SPI123SEL` PLL1_Q, `:333` ✓) | 64 / 4 × 50 / 10 = **80 MHz** | 16 / 2 × 100 / 10 = **80 MHz** |
| SPI4–5 (`SPI45SEL` PLL2_Q, `:332` ✓), SPI6 (`SPI6SEL` PLL2_Q, `:345` ✓) | 64 / 8 × 45 / 5 = **72 MHz** | 16 / 2 × 75 / 6 = **100 MHz** |

ChibiOS formula lines (`hal_lld_type1.h` ✓): `PLLCLKIN` `:1541-1546`; `PLL2_REF_CK` `:1592`; `PLL2_VCO_CK` `:1822`; `PLL1_Q_CK` `:1917`; `PLL2_Q_CK` `:1934`; `SPI4CLK`/`SPI5CLK` from PLL2_Q `:2635-2637`; `SPI6CLK` `:2660-2661`. `SPIDevice.cpp:40-45` takes `STM32_SPInCLK` directly. The row's existing citations (lines 244, 1518, 1546, 1561, 1801, 1917, 2593; mcuconf 117, 170, 172, 333) still say what they claim; its SPI4 row lacks the PLL2 lines above. SPI4–6 at 16 MHz is exactly `STM32_SPI456_MAX` (100 MHz, `:234`).

**The 16 MHz tree can be cited at the tag.** No §7 stop on (2).

## M3 · (7) PMCR: every writer at the tag

**There is no witness.** One writer exists in the whole tree, outside the vendor headers:

- `modules/ChibiOS/os/hal/ports/STM32/LLD/MACv2/hal_mac_lld.c:305-311`, in `mac_lld_init`, under `#if defined(STM32H7XX)`:
  - `:306` `SYSCFG->PMCR |= SYSCFG_PMCR_PA1SO;`
  - `:308` `SYSCFG->PMCR = (SYSCFG->PMCR & ~SYSCFG_PMCR_EPIS_SEL_Msk) | SYSCFG_PMCR_EPIS_SEL_2;` (under `BOARD_PHY_RMII`)
  - `:310` `SYSCFG->PMCR &= ~SYSCFG_PMCR_EPIS_SEL_Msk;`
- Each is a masked read-modify-write of bit 25 (PA1SO) and bits 23:21 (EPIS). None touches PC2SO (bit 26) or PC3SO (bit 27).
- No other `PMCR`, `PC2SO` or `PC3SO` appears in `libraries/`, in ChibiOS outside the ST headers, or in either hwdef generator script (`grep -rn`, the reports in the kit).
- What `check pinout` reads: from the ELF, `gpio_default_config` and the identity tables (`_serial_tab`, `var_info` rows, parameter defaults); from `hwdef.h`, the 33 `VAL_GPIOx_{MODER,AFRL,AFRH}` macros and, through `Preprocessed::value`, the identity judge's device lists. **None holds a PMCR value**: the writer's operands are instruction immediates, and no macro in `hwdef.h` carries one.
- Also searched, nothing found: raw-address stores (`0x58000400`–`0x5800040f`) in C, C++, assembly, Python and Lua; any `PMCR` or `SYSCFG` in `.S`/`.s` files; `PMCR` in Lua scripts. **Not covered:** a write at run time through a generic memory interface, which no source text names.
- The nearest readable fact is `HAL_USE_MAC` (written by `chibios_hwdef.py:952-954` only when ETH1 is present): it says whether the one writer is linked, not what PC2SO holds.

## M4 · (6) F4, today's order

`check pinout --topology d1-drv8876.topology.json --elf <11-byte file>` prints `Truncated` (exit 3), and with a missing ELF `UnreadableInput` (exit 3). The topology's own refusal, `GpioNotJudged` (`U2`), is never reported.

## M5 · R9, pincheck's stale test topology

`tests/common/topology.json` is schema 18 but holds `specification_sha256 03a82a04…`, no serial port and `bor_level 0`. Regenerated by its stated generator (`aethelgard solve --file examples/sentinel-01-minimal.json`), it gains the console (`CONSOLE_TX` PA9, `CONSOLE_RX` PA10, USART1 AF7, their pin speeds) and `bor_level 3`. Then **9 of 24 `check.rs` tests fail**, each by two added findings, `NotAlternate PA9 CONSOLE_TX input` and `NotAlternate PA10 CONSOLE_RX input` (one, the cross-check test, by `!j.ok`). The test ELFs are built in `tests/common` with the console's pins at reset. The file was restored.

## M6 · 2311 F1, SERIAL0's pull

`AP_SerialManager::init` calls `set_options(i)` for SERIAL1 up (`AP_SerialManager.cpp:451-457`), but **SERIAL0 reaches `UARTDriver::set_options` too**: `GCS::setup_console` asks for MAVLink instance 0 (`GCS_Common.cpp:2808-2810`). `find_protocol_instance` searches from SERIAL0 (`AP_SerialManager.cpp:605`), and MAVLink2, SERIAL0's protocol, matches MAVLink (`:781-783`). `find_serial` then sets the port's options "before any user does begin()" (`:632-656`, `port->set_options(_state->options)` at `:656`). With RXINV and TXINV clear, `set_options` pulls both lines up (`UARTDriver.cpp:1611`, `:1624`).

## M7 · Which committed bytes would move (read; nothing built yet)

- **`fab/`:** only `sentinel-01-bench/package/MANIFEST.json`, two lines, by reading `fab.rs:680-760`:
  - `topology_sha256` (line 4);
  - `verdict_sha256`, the sha256 of the `verify` verdict line, which carries the topology's hash (`verify.rs:77`).
  - **The scenarios' `sha256` do not move:** each is the `--scenarios` input file's hash (`fab.rs:690-691`, `:725-729`), and no scenario file changes. The `files` map moves only if a file does. `FAB-NOTES.md` holds no topology hash (`grep 857ca021 -r fab` finds MANIFEST only). The ELF's provenance names the specification's hash (`svd-hal/src/board/firmware.rs:191`), which does not move. `SOLVED_SCHEMA_VERSION` appears in svd-hal only under `#[cfg(test)]` (`board.rs:8593`, after `:8572`). **No Gerber, drill, BOM, CPL or ELF byte moves.** Confirmed by the dispatch, after `4240e`.
- **hwdef goldens:** there are **five** `hwdef/tests/fixtures/*.hwdef.dat`, not four (`four-port`, `sentinel-01`, `sentinel-02`, `serial-dshot-9600`, `serial-dshot-9600-open`). Each moves by the two SWD lines, plus 2311 F1's sentence where a `DEFAULT_SERIALn_PROTOCOL` is emitted. None moves by (1), since none has a sense.
- **The four hand-written CI boards** (`pincheck/fixtures/{step-g,seed-af,seed-pin,seed-pkg}.hwdef.dat`) must gain the SWD lines too. Otherwise their ELFs leave PA13 and PA14 as inputs, and the new SWD finding names every one in CI's `ardupilot (…)` jobs. Q5 rewords their first lines in the same edit. The prediction (P13) missed them.
- **Topologies:** the eight committed `SolvedTopology` files move by `schema_version` (P12), and `pincheck/tests/common/topology.json` is regenerated (R9, M5).

## Predictions against results

| | predicted | measured |
|---|---|---|
| P1, P2 | 35; two sense boards | yes |
| P3 | four solve refusals | **five**: also `solve-thrash-pins` (`Infeasible`) |
| P4 | the named refusals | yes; also `LabelTakesAfPrefix` on `solve-stress` (not forecast) |
| P5 | the rest ok or serial | yes: 9 ok, 12 `NoSerialPort`, 2 `NoConsole` |
| P6 | Sentinel-03 generates once the sense is bypassed | **no:** `ImuOrientationUndeclared` (U5) first |
| P7 | SWD text; PA13 `PULLUP`, PA14 `PULLDOWN` "on some" | the text, yes. **No line writes a pull**: the pulls are the script's defaults for the SWD labels |
| P8 | battery text; `HAL_BATT_MONITOR_DEFAULT 4` | the text, yes; **`4` is wrong, voltage only is `3`** |
| P9 | SPI1–3 80 MHz at 16 MHz; SPI4/5 may differ | yes; SPI4–6 **100 MHz** |
| P10 | no witness for PC2SO/PC3SO | yes: one writer, which never touches them |
| P11–P15 | the moves | as M7; P13 missed the four CI hand boards; P14's "each scenario's sha256" is wrong, `verdict_sha256` moves instead |
