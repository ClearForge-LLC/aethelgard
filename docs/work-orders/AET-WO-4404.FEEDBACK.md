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

## 3. What was built

- **The judge** (`aethelgard-pincheck/src/identity.rs`, `serial_settings`, beside `serial_ports`). It reads `hwdef.h` and holds each port against the topology's `role` and `framing.inverted`. Its tables are its own, labelled `belt-and-suspenders:` (`RUL-20`), and it shares no code with the generator.
  - **`SERIAL_PROTOCOLS`:** all 51 `SerialProtocol` names, `AP_SerialManager.h:37-91`.
  - **`DEFAULT_PROTOCOLS`:** the per-n `#ifndef` defaults, `AP_SerialManager.cpp:47-152`.
  - **The option bits:** `UARTDriver.h:90-102`.
- **What it reads, exactly as §1.2 asks:**
  - SERIAL0's define is never read. Whatever stands there runs MAVLink 2 (`:180`), so a role other than console or `mavlink` there is a finding. That rule was added after the adversarial pass (§5, A-D1); §1.2's words assume the console stands there. A port at SERIAL0 declared inverted is a finding, because SERIAL0 has no options parameter.
  - SERIAL1–9's protocol is the define, else the default for its n, and must equal the role's exactly.
  - Options are the define, else 0, on 1–6 and 9; SERIAL7 and 8 are 0 whatever the define says, and the define is not read.
  - A non-RC port must have RXINV and TXINV exactly when it is inverted, and no other bit. An RC port may have RXINV and no other bit.
  - At most one port may compile as RCIN.
- **Findings** (`check: "identity"`):
  - `WrongSerialProtocol`;
  - `WrongSerialOptions`;
  - `RcInputTwice`;
  - `MissingSerialParameters`: a topology port past SERIAL9, which no ArduPilot parameter covers. It cannot occur on an H753, which has nine U(S)ARTs; `INV-06` names it rather than skip it.
- **Refusals** (`MalformedHeader`, by macro name, `INV-06`):
  - a protocol that is neither a C integer constant (read as GCC does: hex, octal, `U`/`L`) nor one of the table's names;
  - a name defined as a macro;
  - a `HAL_HAVE_SERIALn_PARAMS` other than `1`, or one defined twice.
  - The reader's existing refusals cover the rest: a conditional, `#undef`, `#pragma`, a second definition.
- **The generator** (`aethelgard-hwdef/src/lib.rs`):
  - **`LabelTakesAfPrefix`**, beside `LabelCollision`. ArduPilot's 14 `af_labels` are copied once and cited (`chibios_hwdef.py:33`), and the test is case-sensitive.
  - **SERIAL7 and SERIAL8 emit no `_OPTIONS`.** `not_emitted` names the omission, so the header and the CLI's report carry it.
  - **The "Serial pulls" header sentence** appears only on a board with a `DEFAULT_SERIALn_PROTOCOL`, which is SERIAL1 or later, so no committed golden moves.
- **`generate.rs`:** the keyword test holds 26 exact `(define, AP_SerialManager.cpp line)` pairs in place of a pattern over n. Only `DEFAULT_SERIAL0_BAUD` is reached there, because the three committed boards have a console only. The multi-port lines, SERIAL7 included, are checked in `hwdef_tier1.rs` on solved four- and eight-port specifications.
- **The four fixture headers:** rewritten as §1.5 asks, comment lines only (§4).
- **READMEs:**
  - pincheck: what is judged; the residual (read from `hwdef.h`, not the ELF); CR2 read at source;
  - cli: L134 gains `LabelTakesAfPrefix`; L520 gains protocol and options;
  - hwdef: the SERIAL7/8 omission, the run-time pull-ups, the refusal, and what judges it.
- **No CLI source changed.**

## 4. §2 acceptance [measured]

**Each §1.1 (c) mutation is refused by name, and the bases pass.** This branch's binary runs on the boards of §2.3. It was first measured at `66b10879` and re-measured at `c3d4f4f0`, after the SERIAL0 role fix and the reduction: every verdict is identical, and the no-console topology of A-D1 now gives `WrongSerialProtocol SERIAL0`:

| board | `main` | this branch |
|---|---|---|
| fc-base | exit 0 | **exit 0** |
| adv-proto-swap | exit 0 | exit 1: `WrongSerialProtocol SERIAL1` (expected GPS (5), runs RCIN (23)), `WrongSerialProtocol SERIAL2` (expected RCIN (23), runs GPS (5)) |
| adv-opt-swap | exit 0 | exit 1: `WrongSerialOptions SERIAL1` (none, 0x8) |
| adv-opt-inv | exit 0 | exit 1: `WrongSerialOptions SERIAL1` (none, 0x3) |
| adv-proto-del1 | exit 0 | exit 1: `WrongSerialProtocol SERIAL1` (GPS, runs MAVLink2) |
| fc-mav3 | exit 0 | **exit 0** |
| adv-proto-del3 | exit 0 | exit 1: `WrongSerialProtocol SERIAL3` (MAVLink2, runs GPS) |
| lab-lower (`spi_cs`) | exit 0 | exit 0 |
| eight (8 ports, SERIAL7 with no `_OPTIONS`) | exit 0 | **exit 0**; `hwdef.h` and var_info: `7_PROTOCOL=2`, `7_OPTIONS=0` |
| eight-opt7 (`DEFAULT_SERIAL7_OPTIONS 8` added) | exit 0 | **exit 0**; var_info `7_OPTIONS=0`, so ArduPilot ignores the define |
| eight-opt6 (`DEFAULT_SERIAL6_OPTIONS 0` → 8) | exit 0 | exit 1: `WrongSerialOptions SERIAL6` (none, 0x8); var_info `6_OPTIONS=8` |

P6 holds exactly.

**CI's pinout step, reproduced locally.** The pinned image built the four hand boards (fixtures with the new headers) and the three generated boards (from this branch's `generate hwdef`). Then `container/ardupilot-pinout.sh` ran unchanged, with `main`'s binary and with this branch's (`66b10879`, re-run at `c3d4f4f0`). The hand-board ELFs were built before the A-D3 header correction, which changes comment lines only, and ArduPilot drops those (`hwdef.py:209`).
- the script exits **0** with both;
- the four hand boards keep exactly the findings the script expects;
- the three generated boards exit 0 with `identity: judged`.

The hand boards' topology has a console only, so the judge reads nothing new there.

**Goldens are byte-identical.**
- `solve` and `generate hwdef` from this branch, `cmp` against `crates/aethelgard-hwdef/tests/fixtures/`, are byte-identical for `sentinel-01`, `sentinel-02` and `serial-dshot-9600`, in both `hwdef.dat` and `topology.json`.
- `generate.rs`'s `each_board_generates_its_committed_file` and `hwdef_tier1.rs`'s `each_ci_board_generates_its_committed_file` pass.
- No topology fixture changed (`git diff --stat cd43b207` lists none).

**The label rule, measured once against ArduPilot.** Each of the 14 prefixes, on `SPI_FLASH_CS` and its siblings, is refused at configure (§2.3). The generator now refuses each first:
- in `generate.rs`, per prefix, at the generator;
- in `hwdef_tier1.rs`, per prefix, through `generate hwdef`: exit 2, `LabelTakesAfPrefix`, the net as `id`, no file.

Lower-case `spi_cs` builds in ArduPilot and generates.

**The fixture diff is comment lines only.** `git diff -U0 cd43b207 -- crates/aethelgard-pincheck/fixtures/` gives no `+` or `-` line that does not start with `#`. Nothing reads the header bytes:
- `git grep` finds only the CI jobs that copy the files into ArduPilot's build, `ardupilot-build.sh d4`, and `step-g.allow.json`'s readers;
- no digest, size or line count is pinned;
- ArduPilot drops comment lines (`hwdef.py:209`).

**Unchanged:** `the_generator_calls_none_of_the_check_s_rules` and `pincheck/tests/separation.rs` pass, and neither file was edited.

## 5. Adversarial pass

A fresh subagent ran it read-only, on Opus because it is judgement work. It worked at `66b10879` with the kit in `~/.cache/aet-4404`, ran in its own clone, and made two builds of its own in the pinned image. I spot-checked the defect before acting (A-D1, A-D3 below).

| id | probe | severity | verdict, evidence |
|---|---|---|---|
| A-D1 | SERIAL0 skipped by position, not by role | **medium** | **Fixed.** A topology with no console (fc-base with port 0 made `gps`) puts the GPS port at SERIAL0, where it runs MAVLink2 (`0_PROTOCOL=2` in var_info). `check pinout` exited 0 on `main` and on `66b10879`; I reproduced it [measured]. Now `WrongSerialProtocol SERIAL0` (expected GPS (5), runs MAVLink2: SERIAL0's literal, `:180`), and the real fc-base build exits 1 on that topology [measured]. Tier 1: no console with GPS first; `mavlink` first passes; `rc_sbus` first gives two findings. Mutant G11 is killed. Without this, only the generator's `NoConsole` stood in the way, which is the cross-dependence `RUL-20` forbids. |
| A-D2 | GPS receivers capped at two (`AP_GPS_config.h:16` `GPS_MAX_RECEIVERS 2`; `AP_GPS.cpp:345-349` binds the first two GPS ports) | medium-low | **Not built: outside this work order.** It is the same class as the RCIN rule; the eight-port board's five GPS ports pass [measured by the adversary]. Proposed as a §7.1 row (§6, D3). |
| A-D3 | Fixture header history | low | **Fixed.** `SERIAL_ORDER` with USART3 and the PD8/PD9 lines came in `4400a` (`47a883b8`). `4300b` (`25640c55`) added the USART1 lines and put USART1 first. Checked per file with `git log -S` [measured]. The headers now say both. Grounds L779 carries the same imprecision. |
| A-D4 | `HAL_HAVE_SERIALn_PARAMS (1)` refused with "not compiled" | low | **Wording fixed.** The refusal stands (`INV-06`). The message now says the value is not the `1` ArduPilot's script writes. |
| A-D5 | Option bits 16 and up | low | **No change.** The HAL takes `uint16_t` (`UARTDriver.h:86`), so ArduPilot runs `0x10000` as 0 while the judge names it. That is a false positive, never a pass of something wrong. It stays strict. |
| A-R1 | The unreachable `3` branch | — | Deleted in the reduction pass (§7). |
| 1–10 | The A3 trap; the compiler's reading (eleven protocol forms, odd blanks, `%:`, `#if 0`, `#undef`, `push_macro`, a macro value; and a real build, `adv-numforms`, with `05`, `027`, `0x5UL`, `00` and `0x0u`, whose var_info matches the judge); the defaults; SERIAL7, 8 and 9; every option bit on RC and non-RC ports; two RCIN ports; all 14 prefixes and the case rule; the fixture bytes; the goldens; other readers (`set_protocol_and_baud`'s only caller is IOMCU, `defaults.parm`, OTG2) | — | **All hold** [measured or read, per the adversary's table, kept at `~/.cache/aet-4404/adv`]. The probe of a real AF used as a chip select (`SPI2_NSS` on PA11, AF5, guard bypassed in the `.dat`) builds as alternate. `check pinout` catches it on both binaries (`forward NotOutput PA11`). The adversary also re-ran §2.3 and §4 and confirmed every verdict and compiled value. |

## 6. Decisions for the architect

- **D1. Bind protocol, options and baud to the ELF.**
  - `AP_SerialManager::var_info` is readable as bytes (§2.4). It covers exactly the compiled ports and folds in every `#ifndef`, the SERIAL7/8 literal and `HAL_HAVE_SERIALn_PARAMS`.
  - Reading it would close §7.1 L800 for all three facts at once and retire the `hwdef.h` forms the judge must refuse.
  - **Recommended:** a follow-up work order, with `elf.rs` in its fence. Not built here, as ruled.
- **D2. The RC port's TX inversion.**
  - `rc_sbus` declares `inverted` (both lines). ArduPilot's RC input inverts RX only, and the ruled rule requires TXINV clear.
  - The judge holds ArduPilot's behaviour, and the generator's header already names it (F3).
  - **Recommended:** no change. Confirm that the judge should not hold ArduPilot to the specification's TX inversion.
- **D3. A §7.1 row for the GPS receiver cap (A-D2).**
  - A third GPS-protocol port compiles and is never bound.
  - **Recommended:** the next `pincheck` work order names a third compiled GPS port, as `RcInputTwice` names a second RCIN, cited `AP_GPS_config.h:16` and `AP_GPS.cpp:345-349`.
- **D4. SERIAL0's role is judged** (adversarial A-D1).
  - §1.2 says "SERIAL0: no protocol is judged". That holds only while the console stands there. With no console, the first port of any role sits at SERIAL0 and runs MAVLink 2 (`:180`), so a role other than console or `mavlink` there is now a finding. The define is still never read.
  - Built, because without it the only guard is the generator's `NoConsole`.
  - **Recommended:** accept it as the judge's reading of §1.2.
- **The rows the work order says the architect opens** (no action here): the run-time pull-ups against `4400b` D3 (a); the four-port generated CI board after `1911`; the twin read-back of `USART_CR2` after `4400g`.

## 7. Reduction pass, mutants and gates

- **Reduction pass (diff): −1 dead path, −1 duplicate; net +1 line (one comment). 2 protected kept; 0 findings.**
  - **Dead path:** the generator's unreachable `3` branch is gone. The line is now `_OPTIONS 0`, with the refusal that makes it so named at the site and in the README. No test reached the branch (§2.2).
  - **Duplicate:** the judge's `rc` flag restated `want == RCIN`.
  - **Kept, protected:** the judge's `SERIAL_PROTOCOLS` table and its role match, each labelled `belt-and-suspenders:` at the site (`RUL-20`). Their twin is the generator's `protocol()`, which carries no label: it is the generator's own choice, not a copy.
  - **Kept, not dead:**
    - `MissingSerialParameters`: a hand-written topology can carry an 11th port, and skipping it would breach `INV-06`.
    - The `HAL_HAVE_SERIALn_PARAMS != 1` refusal: ArduPilot itself writes `0` for a hidden IOMCU UART (`chibios_hwdef.py:1700-1701`).
  - The two tests' 14-prefix lists are the tests' own oracles, not copies to derive.
- **Mutants:** one per guard this work order adds, each against the suite it lives in (`~/.cache/aet-4404/mut/mutants.py`). Results: §9.
- **Gates** (results: §9):
  - `cargo fmt --all -- --check`: clean;
  - `cargo clippy --workspace --all-targets -- -D warnings`: clean;
  - `cargo test --workspace`: 2,269 passed, 0 failed, at `66b10879` (21 min 56 s, capped scope); re-run after the fixes, §9.
- **CI planner (`1910`):**
  - no test was added to a lock-serialised binary;
  - no test was renamed or removed;
  - every new test runs in well under a second;
  - so no weight row is needed (P1, P2) and `TIER2_MIN` is unchanged: no tier-2 test.

## 8. Outside the fence

Nothing. No change was made to `aethelgard-ir`, `.github/workflows/`, `ci/`, `container/`, any specification or any topology fixture. `main` `47ec18f6` (`1910`) is merged in; it touches none of this work order's files.

## 9. Status

- **Mutants** at `6edb8ab9`: **16 of 16 killed, each by a failing test.**
  - The judge (`G1`–`G11`): SERIAL0's inversion and role; the default table; past SERIAL9; `HAL_HAVE_SERIALn_PARAMS`; a macro-named protocol; an unknown name; the protocol comparison; RCIN twice; the SERIAL7/8 options; the RC mask; the inverted non-RC rule.
  - The generator (`H1`, `H1b`, `H2`, `H3`): a dropped prefix; a case-folded prefix; the SERIAL7/8 options; the pull-up header condition.
  - One early mutant (G7) first died only because it did not compile. It was rewritten until a test killed it.
- **Gates** at `7566320a` (main `47ec18f6` merged):
  - `cargo fmt --all -- --check`: clean;
  - `cargo clippy --workspace --all-targets -- -D warnings`: clean;
  - `cargo test --workspace`: **2,269 passed, 0 failed** (17 min 13 s, capped scope);
  - `cargo package --no-verify` of `aethelgard-ir`, `-pincheck` and `-hwdef`: clean, and no `src/` file includes a file outside its crate.
- One PR, left unmerged. The title and body are in `~/.cache/aet-4404/pr-title-body.md`.
