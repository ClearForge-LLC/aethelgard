# AET-WO-4404 FEEDBACK — serial protocol and options judged, the AF-prefix refusal, the fixture headers

**Builder:** Claude (strong tier), worktree `~/aethelgard-4404`, branch `wo/aet-4404-hwdef-serial` from `main` `cd43b207`.
**Cadence:** build, with one measured stop (§1.1). The work order is read from the docs branch (`~/.cache/aet-4404-wo.md`) and is not committed here.

Labels:
- **[read]**: at source, this repository or ArduPilot `Copter-4.7.2` (`fb773567`), copied read-only out of the pinned image `160f9955…`;
- **[measured]**: run on Furnace in that image under `capped-run --mem 8G` (`memory.swap.max` 0), `--network none`;
- **[inferred]**: neither.

## 1. Predictions (before any judged build)

Written before any build result was read, and never edited afterwards. **One exception, stated plainly:** the configure-only label probes (§1.1 (c), 14 prefixes and the lower-case label) ran a few minutes before this table was committed, and their results had been read. The rows for the label rule below are therefore labelled **post hoc**. They match Grounds A9 (`4400b`), which predicted them first.

| # | question | prediction | basis |
|---|---|---|---|
| P1 | fc-base, the compiled protocol of SERIAL0..3 | SERIAL0 MAVLink2 (the literal at `:180`); SERIAL1 GPS (5), SERIAL2 RCIN (23), SERIAL3 GPS (5), each from its define | [read] |
| P2 | fc-base, the compiled options | SERIAL1 0, SERIAL2 0 (its define is not emitted, so the `#ifndef` default is 0; RC input then sets RXINV itself), SERIAL3 0; SERIAL0 has no options parameter | [read] |
| P3 | fc-base builds; `main`'s `check pinout` exits | builds; exit 0 | [inferred], from `4400b2` |
| P4 | each mutation builds unmodified ArduPilot, and `hwdef.h` carries its define verbatim (or lacks the deleted one) | yes for all five | [read], `chibios_hwdef.py:1004-1010` |
| P5 | `main`'s `check pinout` on each of `adv-proto-swap`, `adv-opt-swap` (S1 OPTIONS 8), `adv-opt-inv` (S1 OPTIONS 3), `adv-proto-del1` (GPS port at SERIAL1 with no define: runs MAVLink2), `adv-proto-del3` (MAVLink port at SERIAL3 with no define: runs GPS) | exit 0 on every one (the gap) | [inferred]: `main` reads no `_PROTOCOL` or `_OPTIONS` |
| P6 | this branch's `check pinout` on the same | fc-base 0; each mutation exit 1 with `identity` findings: `adv-proto-swap` two `WrongSerialProtocol` (SERIAL1, SERIAL2); `adv-opt-swap` and `adv-opt-inv` one `WrongSerialOptions` (SERIAL1); `adv-proto-del1` and `adv-proto-del3` one `WrongSerialProtocol` each (SERIAL1, SERIAL3) | [inferred] |
| P7 | any committed `hwdef.dat` golden or topology moves | none: no committed board has a SERIAL7 or 8, and none has a SERIAL1–9 at all (§1.1 (b)) | [measured] (b) |
| P8 | **post hoc:** the 14 AF prefixes as a chip-select label | each refused at configure (`Unknown pin function`) | [measured] before this table |
| P9 | **post hoc:** a lower-case `spi_cs` | configures, so it is not refused by the generator | [measured] configure before this table; the full build is predicted to pass |

## 2. §1.1: the options rule measured first

**The stop does not fire.** Every protocol and options fact is in ArduPilot's `hwdef.h` as the compiler reads it, and in the ELF's parameter table (§2.4). The expected values need only the topology's `role` and `framing.inverted`. §2.5 explains why the RC inversion is not "an RX-only inversion the specification cannot state".

### 2.1 (a) The cites, re-read in the pinned image [read]

Every cite in Grounds holds at the line given, except those listed here.
- **`AP_SerialManager.cpp:442-445`** (SERIAL0 forced back to MAVLink): the block is `:436-446` and sits under **`#ifdef HAL_OTG1_CONFIG`** (`:436`), a USB console. No board here defines it (fc-base's `hwdef.h` has no `OTG` line [measured]). On a UART console, SERIAL0's MAVLink2 rests on the literal at `:180` alone. The init loop starts no protocol for SERIAL0 (`:451`, `i=1`). The judge's conclusion is unchanged: SERIAL0's protocol is not judged.
- **Each `AP_GROUPINFO` row is compiled only under `#if HAL_HAVE_SERIALn_PARAMS`** (`:183`, `:202` … `:379`; defaults in `AP_SerialManager_config.h:169-197`). `chibios_hwdef.py:1700-1709` writes `HAL_HAVE_SERIALn 1` and `HAL_HAVE_SERIALn_PARAMS 1` for each `SERIAL_ORDER` entry. A `define HAL_HAVE_SERIAL1_PARAMS 0` in `hwdef.dat` would be a second definition, and the judge's reader already refuses a macro defined twice.
- **`chibios_hwdef.py:1868-1877`** writes `DEFAULT_SERIALn_PROTOCOL HAL_OTG2_PROTOCOL` itself, for a second USB CDC, inside `#if defined(HAL_NUM_CAN_IFACES) && …`. The reader refuses a macro named in a conditional it does not evaluate (`Conditional`), so such a board is refused, never misread.
- **`UARTDriver.cpp:1562-1672`:** `set_options` is `:1562-1717`; the H7 branch is `:1589-1635`; `:1611` and `:1624` are as cited.
- **`AP_SerialManager.cpp:47-161`:** the SERIAL1–9 `#ifndef` blocks are `:47-162`. Protocol defaults: 1–2 MAVLink2 (`:48`, `:61`), 3–4 GPS (`:74`, `:87`), 5–9 None (`:100` … `:152`).
- **Added, as asked:**
  - **`begin()` and `_last_options`:** `begin` reads only RX and TX no-DMA (`UARTDriver.cpp:318-339`) and no-FIFO (`:355`) from `_last_options`. It starts `sercfg.cr1/cr2/cr3` from `_cr1/2/3_options` (`:435-438`), which `set_options` wrote. RXINV, TXINV and SWAP thus reach `USART_CR2` at every later `begin`, including AP_RCProtocol's (`AP_RCProtocol.cpp:335`).
  - **The pull options** (`set_pushpull`, `:1543-1559`) run at the end of `set_options` (`:1697`), after the pull-up or pull-down that RXINV and TXINV choose (`:1601-1625`). An explicit pull bit therefore wins.
  - **`pre_arm_checks`** (`AP_SerialManager.cpp:843-861`): bits 10 and 12 of any SERIALn_OPTIONS fail pre-arm. The judge's "every other bit clear" covers them.
- **The label rule:** `af_labels` at `chibios_hwdef.py:33`, the lookup at `:173-178`. `process_line` takes `a[1]` as the label with no case change (`:2840`). `get_alt_function` returns before the lookup for a label ending `_TXINV` or `_RXINV` (`:154-156`), and for one starting `USART` or `UART` and ending `_RTS` or `_CTS_GPIO` (`:165-168`). Those labels configure (§2.3).

### 2.2 (b) Every committed specification, generated [measured]

`main`'s binary (`cd43b207`); `solve`, then `generate hwdef`, over `examples/*.json` and `crates/*/tests/fixtures/*.json`:
- **Generated:** `sentinel-01`, `sentinel-01-minimal`, `sentinel-01-bench`, `sentinel-02-own` and `serial-dshot-9600`. Each has a console only, so it emits **no `_PROTOCOL` and no `_OPTIONS` line**.
- **Refused:** every other specification that solves. Each fails with `NoSerialPort`, `NoConsole` (`serial-four`, `sentinel-02-shaped`) or `SenseNotEmitted`.
- **`inverted: true`** appears only on `serial-four`'s `SBUS` (`rc_sbus`).
- **The serial-four variants** (`hwdef_tier1.rs`):
  - `four-console` is refused `RcInputTwice`.
  - `one-rc`, and fc-base (`one-rc` with the bus at 100 kHz), emit `SERIAL1_PROTOCOL SerialProtocol_GPS`, `_BAUD 115200`, `_OPTIONS 0`; `SERIAL2_PROTOCOL SerialProtocol_RCIN` (no baud or options, F3); `SERIAL3_PROTOCOL SerialProtocol_GPS`, `_BAUD 115200`, `_OPTIONS 0`.
- **So no reachable generated file holds a nonzero `_OPTIONS`.** The `3` branch is unreachable, as predicted.

### 2.3 (c) Built in the pinned image [measured]

Built with `container/ardupilot-build.sh boards …`, 23–33 s each with a warm ccache. Each mutation is fc-base's generated file with one change, except `adv-proto-del3`, whose base is `fc-mav3`: fc-base with the `SBUS` port given role `mavlink`, solved and generated, and built as the control. "var_info" is the ELF's compiled default (§2.4).

| board | change | builds | `hwdef.h` | var_info (PROTOCOL / OPTIONS) | `main`'s `check pinout` |
|---|---|---|---|---|---|
| fc-base | — | yes | S1 `GPS`, S2 `RCIN`, S3 `GPS`; S1, S3 `OPTIONS 0` | 0:2 1:5 2:23 3:5 / 1:0 2:0 3:0 | exit 0 |
| adv-proto-swap | S1 `RCIN`, S2 `GPS` | yes | verbatim | 1:23 2:5 | **exit 0** (the gap) |
| adv-opt-swap | S1 `OPTIONS 8` | yes | verbatim | 1_OPTIONS 8 | **exit 0** |
| adv-opt-inv | S1 `OPTIONS 3` | yes | verbatim | 1_OPTIONS 3 | **exit 0** |
| adv-proto-del1 | S1 `_PROTOCOL` deleted (GPS port) | yes | no S1 `_PROTOCOL` | 1:**2** (MAVLink2, `:48`) | **exit 0** |
| fc-mav3 | control | yes | S3 `MAVLink2` | 3:2 | exit 0 |
| adv-proto-del3 | fc-mav3, S3 `_PROTOCOL` deleted (MAVLink port) | yes | no S3 `_PROTOCOL` | 3:**5** (GPS, `:74`) | **exit 0** |

P1–P5 hold. The label rule, configure only, fc-base with `FLASH_CS` relabelled on both its pin line and its `SPIDEV` line:
- **Each of the 14 prefixes + `_FLASH_CS`** (`SPI_FLASH_CS` is `4400b` A9's case): configure fails with `Error: Unknown pin function PA11:<label> for MCU STM32H753xx`.
- **`spi_cs`:** configures, builds (`lab-lower`), and passes `check pinout` with exit 0. **The rule is case-sensitive, so the generator refuses no lower-case label.**
- **`SPI_FLASH_TXINV`, `TIM_FLASH_RXINV`, `USART_FLASH_RTS`, `UART_FLASH_CTS_GPIO`:** each configures, because of the carve-outs in §2.1. The generator still refuses all four. ArduPilot gives each a meaning other than a chip select (an inversion-control pin, software RTS/CTS), so a chip select must not carry one. **The generator therefore refuses a strict superset of what ArduPilot refuses at configure, and this report claims nothing more.**

### 2.4 (d) The ELF: `AP_SerialManager::var_info` is readable [measured, in under the time box]

- **Where it is:** `_ZN16AP_SerialManager8var_infoE` is a `T` symbol in `.text` (fc-base: `0x0811c498`, 0xf0 bytes).
- **Its layout:** an array of `AP_Param::GroupInfo` (`AP_Param.h:206-218`), 16 bytes on ARM32:
  - `const char *name`;
  - `ptrdiff_t offset`;
  - a union whose `float def_value` is the default (`AP_GROUPINFO`, `:140`, `:152`);
  - `uint16_t flags`, `uint8_t idx`, `uint8_t type`;
  - terminated by `AP_GROUPEND` (`idx` 255, `type` 0).
- **What it shows:** decoded with `nm`, `readelf` and a 30-line reader (kit `~/.cache/aet-4404/varinfo.py`), it lists every compiled SERIALn row with its default. Only the rows `HAL_HAVE_SERIALn_PARAMS` compiles are present. The column in §2.3 is that decode, so the compiled values are **measured**, not inferred.
- **Not built, as ruled:** the judge reads `hwdef.h` (ruled at writing), and binding to the ELF is §7.1 L800's (not this work order's). `elf.rs` is also outside the fence. **Decision D1 (§6):** one follow-up could read each port's protocol, options *and* baud from `var_info` and close L800 for all three.
- **The baud column shows a trap.** `1_BAUD`'s default is `115200`, the define verbatim. `0_BAUD`'s is `57`, because its `AP_GROUPINFO` divides by 1000 (`:172`). A reader must apply ArduPilot's `map_baudrate` rule, not take the float as a baud.

### 2.5 (e) The CR2 effect [read, not measured]

- **What each bit does:** RXINV, TXINV and SWAP set `USART_CR2` bits (`UARTDriver.cpp:1601-1635`), held in `_cr2_options` and applied at every `begin` (`:437`).
- **SWAP** exchanges the TX and RX functions inside the USART. The GPIO alternate functions, which `check pinout` reads, are unchanged, so the GPIO judge cannot see it. That is why `adv-opt-swap` must be caught here.
- **Read at source only, as ruled.** A twin read-back after `4400g` is the architect's residual row.
- **Why §7's "RX-only inversion" does not fire:**
  - An `rc_sbus` port declares `inverted` (both lines, `ir/src/serial.rs:82`).
  - ArduPilot's RC input inverts RX only (`AP_RCProtocol.cpp:330-334`), and the ruled RC rule leaves RXINV to it and requires TXINV clear.
  - The generator already names that divergence in the header (F3, #189).
  - So the rule needs nothing the specification cannot state. It does keep ArduPilot's choice for TX. That is recorded as decision D2 (§6), not a stop.
