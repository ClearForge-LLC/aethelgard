# AET-WO-4404 — The hwdef and pincheck follow-up: each serial port's protocol and options judged from ArduPilot's own reading, an alternate-function-prefixed label refused by the generator by name, and the `pincheck` fixture headers made true

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff (`cd43b207` when written; verify live HEAD).
**Branch:** `wo/aet-4404-hwdef-serial`.
**Author:** Claude (architect) · **Builder:** strong tier, with a targeted adversarial pass.
**Phase:** P4, exit clause 3 ("the generated `hwdef.dat` builds unmodified ArduPilot, and the pinout check passes"). Written 2026-11-19. **Cadence: build, with one measured stop (§1.1). Any need for a specification schema change is a §7 stop.**

**Grounds:**
- **The rows** (roadmap §7.1, this docs branch; on `main` `cd43b207` each sits six lines higher: L795, L754, L773):
  - **L801:** "Judge each port's `DEFAULT_SERIALn_PROTOCOL` (from its role) and `_OPTIONS` (from `inverted`). Measure the options rule first · `4400b2` D2 (A6), #196 · measured in `AET-WO-4400c`, then a pincheck build."
  - **L760:** "A chip-select name with an alternate-function prefix is refused by ArduPilot, not by the generator by name · `4400b` F2, A9 (#189) · the next `aethelgard-hwdef` work order."
  - **L779:** "The `pincheck` `hwdef.dat` fixture headers say they were written from specification `a277c933…`, but since `4300b` they also carry `USART1` and `SERIAL_ORDER` … · `4402` Q5 · the next `pincheck` work order (reword, or name the specification they match)."
  - **Not bundled:** L759 (the RC tolerance at 416666, `4240`'s); L800 and L802 (deferred, triggers unmet); L750 and L782 (the next CLI work order); L783 (the next reader work order); L785 (`solve`). No other §7.1 row names the next hwdef or pincheck work order.
- **What `4400c` measured of L801, and what it did not** (`AET-WO-4400c.FEEDBACK.md` §1.4, L180) [read]:
  - SERIAL0 only: MAVLink 2 frames, `BRR` `0x271`. The board (`sentinel-02-own.json`) has one port.
  - SERIAL0 has no `_OPTIONS` parameter, and `DEFAULT_SERIAL0_PROTOCOL` is never read (`4400b` A4). So **nothing of the options rule was measured.** The row is still open in full.
- **What the judge is asked to catch** (`4400b2` FEEDBACK §4, A6): `adv-proto-swap`, the four-port board with `DEFAULT_SERIAL1_PROTOCOL SerialProtocol_RCIN` and `DEFAULT_SERIAL2_PROTOCOL SerialProtocol_GPS`, builds and passes `check pinout` with exit 0.
- **The generator today** (`crates/aethelgard-hwdef/src/lib.rs`) [read]:
  - `:268` `protocol()`: console and `mavlink` → `SerialProtocol_MAVLink2`, `gps` → `SerialProtocol_GPS`, `rc_crsf` and `rc_sbus` → `SerialProtocol_RCIN`.
  - `:283` `implied()` and `:575-597` (`FramingNotImplied`): only `rc_sbus` may be inverted; every other role must declare `inverted: false`.
  - `:646-656`: `DEFAULT_SERIALn_PROTOCOL` for n ≥ 1 only (`4400b` A4).
  - `:660-671`: an RC port takes no `_BAUD` or `_OPTIONS` (`4400b` F3, ruled on #189).
  - `:673-678`: `DEFAULT_SERIALn_OPTIONS` for n ≥ 1, `3` if inverted, else `0`.
  - **So the `3` branch is unreachable.** Inverted means `rc_sbus`, which is RC and `continue`s at `:671`. Every reachable generated file states `_OPTIONS 0` or none. [read; §1.1 measures it]
- **ArduPilot `Copter-4.7.2` (`fb773567`)**, read by the architect while drafting, from GitHub at that commit, **not** in the pinned image. The builder re-reads each in the image (`container/ARDUPILOT-IMAGE`, `sha256:160f9955…`):
  - `AP_SerialManager.cpp:47-161`: `#ifndef` defaults. SERIAL1–2 `MAVLink2`, SERIAL3–4 `GPS`, SERIAL5–9 `None`; every `_OPTIONS` 0.
  - `:180`: SERIAL0's protocol is a literal; `:442-445` force it back to MAVLink if changed.
  - `:191-395`: the `AP_GROUPINFO` table. **`7_OPTIONS` (`:357`) and `8_OPTIONS` (`:376`) default to a literal `0`**: `DEFAULT_SERIAL7_OPTIONS` and `_8_OPTIONS` are defined (`:131-145`) and never read. `4400b` A4 checked n = 0–3 and 9 only, and `hwdef/tests/generate.rs:95-105` accepts `_OPTIONS` for every n in 1–9.
  - `:451-457`: `set_options(i)` runs for every SERIAL1–9 before its protocol is started; `:536-541`: a second RCIN port is dropped.
  - `AP_HAL/UARTDriver.h:90-102`: the option bits. 0 RXINV, 1 TXINV, 2 HDPLEX, 3 SWAP, 4–7 pulls, 8–9 no DMA, 11 no FIFO.
  - `AP_HAL_ChibiOS/UARTDriver.cpp:1562-1672` (`set_options`, H7 branch): RXINV, TXINV and SWAP go to `USART_CR2`. **With RXINV or TXINV clear it also sets that line's pull-up (`:1611`, `:1624`).** So at run time every SERIAL1–9 TX and RX is pulled up, whatever `FLOATING` put in `gpio_default_config` (`4400b` D3 (a)).
  - `AP_RCProtocol.cpp:326-336` (`apply_to_uart`): RC input sets or clears RXINV only, never TXINV. The specification's `inverted` means both lines (`ir/src/serial.rs:82`).
  - `chibios_hwdef.py:1004-1010`: every `define` line reaches `hwdef.h` verbatim. `:33` (`af_labels`) and `:173-178`: any pin label that **starts with** `USART`, `UART`, `SPI`, `I2C`, `SDIO`, `SDMMC`, `OTG`, `JT`, `TIM`, `CAN`, `QUADSPI`, `OCTOSPI`, `ETH` or `MCO` is looked up in the AF table; absent, configure fails.
- **Where these facts can be read** (`4400b2` FEEDBACK §0) [read]: each port's baud has no ELF table; the defaults sit in `AP_SerialManager::var_info` as floats, not decoded. The judge reads the bauds from `hwdef.h`'s verbatim defines and holds them against the **topology**. Protocol and options have the same standing. This is §7.1 L800's residual (D1 (a)): judged from the same build directory, not bound to the ELF.
- **The generator's labels** (`lib.rs:405-409`): a `gpio` net (a chip select) is emitted as its own pin label (`:920`). `LabelCollision` refuses a label equal to an emitted signal. Nothing refuses an AF prefix. `4400b` A9 measured `SPI_FLASH_CS` and `TIM2_CH1`: each generates, and ArduPilot refuses it at configure.
- **The fixture headers** [read, measured by sha256 over `git log`]:
  - `a277c933…` is `examples/sentinel-01-minimal.json` as of `d00c9653` (`AET-WO-4100`). The file's sha256 is now `21e1b9d5…` (`0829fe83`, `4400d`), and `4400g` moves it again at specification 17.
  - The serial lines came in with `4300b` (`25640c55`; branch commit `33f4fd4c`).
  - CI judges the four hand boards against the topology it solves fresh from `examples/sentinel-01-minimal.json` (`ardupilot-image.yml:410`).
  - Nothing reads the header bytes (`4402` FEEDBACK L90: ArduPilot drops comment lines; no digest, size or line count of a `.dat` is pinned).
- **No CI board exercises SERIAL1–9.** `sentinel-01`, `sentinel-02` and `serial-dshot-9600` each have a console only (`ardupilot-image.yml:256-268`). `4400b2` judged its four-port board locally (`fc-base`), in tier 1 and in the pinned image. This work order does the same.

> **What this is:**
> - `check pinout` judges each serial port's protocol and options, as ArduPilot compiles them from `hwdef.h`, against the topology's role and framing. The rules are written from ArduPilot's source and `aethelgard-ir`, never from the generator;
> - the options rule measured first, as the row asks;
> - the generator refuses, by name, a pin label ArduPilot would read as an alternate function;
> - the generator stops emitting `DEFAULT_SERIAL7_OPTIONS` and `_8_OPTIONS`, which ArduPilot never reads, and its keyword test names exactly what the tag reads;
> - the four `pincheck` fixture headers say what they are.
>
> It is NOT:
> - a specification or parts-table schema change. `4400g` is taking specification 17 now. **Any need for one is a §7 stop;**
> - the RC tolerance at 416666 (§7.1 L759, `4240`'s);
> - binding the `hwdef.h` facts to the ELF (§7.1 L800, deferred);
> - a twin read-back of `USART_CR2`. The twin and the foreign path are `4400g`'s;
> - a fix for the run-time pull-ups `set_options` adds. It is named (§1.4), not changed: no `hwdef.dat` line can remove them;
> - a new CI board. A four-port generated board in `ardupilot-image.yml` is a workflow change (§3); propose it;
> - a change to ArduPilot's tree.

**Ruled at writing (the architect, 2026-11-19):**
- **Where protocol and options are read:** from `hwdef.h`, with the one time-boxed attempt at `var_info` in the ELF (§1.1 (d)). They then share §7.1 L800's residual with the bauds.
- **MAVLink is an exact match.** `protocol_match`'s equivalences do not apply: MAVLink 1 and 2 are different wire formats.
- **`container/ardupilot-pinout.sh` is outside the fence.** If a hand board's expected findings would move, stop and say which. Do not edit the script.
- **The run-time pull-ups:**
  - name them in the READMEs and the emitted header, as drafted;
  - the architect opens a §7.1 row against `4400b`'s D3 (a);
  - change nothing else.
- **A four-port generated CI board:** the architect opens a §7.1 row, sequenced after `1911` (workflows). It is not built here.
- **`USART_CR2`'s run-time effect is read at source, not measured.** That closes L801's judge. A twin read-back after `4400g` is a residual row the architect opens.
- **The unreachable `3` branch:** delete it in the reduction pass, unless a test reaches it.

## 1. Build

0. **Predict, then measure.** Commit a prediction table to FEEDBACK before the first build. Label each answer measured, read or inferred. At least:
   - the compiled protocol and options of each port of the four-port board;
   - `check pinout`'s exit on each §1.1 mutation, on `main`'s binary and after this branch;
   - whether any committed golden moves (predicted: none).
1. **Measure first (the stop).**
   - **(a) At source, in the pinned image.** Re-read every ArduPilot cite in Grounds. Correct any line number that differs and say so. Add what `begin()` does with `_last_options`, and where the pull options (`UARTDriver.cpp:1546-1556`) run.
   - **(b) Over every committed specification** under `examples/` and `crates/**/tests/fixtures/`, and the serial-four variants `hwdef_tier1.rs` builds: generate, and list each emitted `_PROTOCOL` and `_OPTIONS`. Predicted: no nonzero `_OPTIONS`; `inverted` only on `rc_sbus`.
   - **(c) Built in the pinned image** (`capped-run`, `--network none`, `container/ardupilot-build.sh boards`). Base: `4400b2`'s `fc-base` (`serial-four.json` with its MAVLink port made the console, its SBUS port a GPS port at 8N1 115200, the bus at 100 kHz). For the base and each mutation, record: it builds; `hwdef.h` carries the define verbatim; `main`'s `check pinout` exit (predicted 0, the gap):
     - `adv-proto-swap` (`4400b2` A6);
     - `DEFAULT_SERIAL1_OPTIONS 8` (SWAP: TX and RX exchanged in the USART, invisible to the GPIO judge);
     - `DEFAULT_SERIAL1_OPTIONS 3` on the GPS port;
     - a `_PROTOCOL` define deleted from a non-default slot (a MAVLink port at SERIAL3 runs GPS by `:73-74`).
   - **(d) The ELF, once, time-boxed.** Can `AP_SerialManager::var_info`'s defaults be read as bytes, as `_serial_tab` is? If not in a working session, use `hwdef.h` and say why.
   - **(e) The CR2 effect is read, not measured.** Label it so.

   **STOP** with the table if a fact can be read only from the generated `hwdef.dat` (the A3 trap), or if the measured rule needs something the specification cannot state. Otherwise build.
2. **The judge, in `aethelgard-pincheck`** (`src/identity.rs`, beside `serial_ports` at `:577`). It shares no code with `aethelgard-hwdef`.
   - **What compiles.** Each table below is its own, cited to ArduPilot at the tag in the module documentation:
     - **SERIAL0:** no protocol is judged. The literal (`:180`) and the forced reset (`:442-445`) make it MAVLink, whatever `hwdef.h` says. Say so in the documentation; read no `DEFAULT_SERIAL0_PROTOCOL`.
     - **SERIAL1–9 protocol:** the define if `hwdef.h` defines it, else ArduPilot's `#ifndef` default for that n (`:47-152`).
     - **SERIAL1–6 and 9 options:** the define, else 0. **SERIAL7 and 8:** 0, whatever the define says.
     - A value is a C integer constant or a name in the judge's own `SerialProtocol` table (`AP_SerialManager.h:37-62`). Anything else is refused by name, as `4400b2` refuses an unknown table form (`INV-06`).
   - **What is expected, from the topology and `ir` only:**
     - **Protocol by role:** `mavlink` → `SerialProtocol_MAVLink2` (2); `gps` → `SerialProtocol_GPS` (5); `rc_crsf` and `rc_sbus` → `SerialProtocol_RCIN` (23). An exact value: `protocol_match`'s equivalences (`:773-793`) do not apply, because MAVLink 1 and 2 are different wires.
     - **Options, a non-RC port:** RXINV and TXINV set exactly when the port declares `inverted` (both lines, `ir/src/serial.rs:82`); every other bit clear.
     - **Options, an RC port:** every bit but RXINV clear. RXINV is `AP_RCProtocol`'s (`apply_to_uart`, `:326-336`), so it is not judged.
     - **One RCIN port at most** (`:536-541`): a second compiled RCIN is a finding, even where the topology asks for it.
     - **A declared inverted console** is a finding: SERIAL0 has no options parameter, so it runs not inverted.
   - **Findings** carry `check: "identity"`, the fact, the expected and the observed: for example `WrongSerialProtocol` and `WrongSerialOptions`. Name them in FEEDBACK. Ports are judged only where the topology has one; a port the topology lacks is already `UnexpectedSerialPort`.
   - **`RUL-20`:** `aethelgard-hwdef` keeps its own `protocol()`. Label the judge's table `belt-and-suspenders:`, as `4400b2` labelled its `map_baudrate` (`identity.rs:183`).
3. **The generator refuses an AF-prefixed label** (L760).
   - Beside `LabelCollision` (`lib.rs:405-409`): any `gpio` net whose label starts with one of `chibios_hwdef.py:33`'s prefixes is refused by name (`LabelTakesAfPrefix`, or a name you propose), citing that line.
   - The prefix list is ArduPilot's data, copied and cited once. The rule is case-sensitive as ArduPilot's is: Python's `startswith`, and the parser does not upper-case a label (`chibios_hwdef.py:2840-2851`, read by the architect). Confirm it with one build: refuse a lower-case `spi_cs` only if ArduPilot refuses it.
   - Add the name to `crates/aethelgard-cli/README.md:134`'s list and to `crates/aethelgard-hwdef/README.md`'s refusals.
4. **The generator's serial lines.**
   - `DEFAULT_SERIAL7_OPTIONS` and `_8_OPTIONS` are not emitted: ArduPilot never reads them (`:357`, `:376`). This is `4400b` A4's precedent. The header names the omission.
   - `generate.rs:95-105` holds exact (n, key) pairs, each with its `AP_SerialManager.cpp` line.
   - **The unreachable `3`** (`lib.rs:673`): delete the branch, or label it `belt-and-suspenders:` with the refusal that makes it unreachable. Decide in the reduction pass.
   - **The run-time pull-ups** (Grounds): add one sentence to `hwdef/README.md` and the emitted header. `FLOATING` holds at reset; ArduPilot's `set_options` then pulls up TX and RX on SERIAL1–9. Changing nothing else.
5. **The fixture headers** (L779), comment lines only, in the four `crates/aethelgard-pincheck/fixtures/*.hwdef.dat`. Each header says:
   - it was hand-written in `AET-WO-4400` from the topology of specification `a277c933…`, `examples/sentinel-01-minimal.json` as of `d00c9653`;
   - the console's `USART1` lines and `SERIAL_ORDER` were added in `AET-WO-4300b` (`25640c55`);
   - CI judges it against the topology solved from `examples/sentinel-01-minimal.json` at the commit under test (`ardupilot-image.yml`, `pinout`). **By path, never by a hash:** a hash goes stale at every specification bump, `4400g`'s included;
   - the per-line tags are the spike's, and are not kept current.

   First re-check that nothing reads those bytes (`4402` FEEDBACK L90's method). If something does, stop and say what.
6. **The READMEs.**
   - `pincheck/README.md:39` ("protocol and options are not judged") becomes what is judged, from `hwdef.h`.
   - `cli/README.md:520`'s list of `hwdef.h` facts bound only by the build directory gains protocol and options.
7. **Nothing else moves.** Every committed `hwdef.dat` golden, every topology and every CI board is byte-identical to `main`. Report it from a run.

## 2. Acceptance

- **§1.1's table is in FEEDBACK before any judge code,** with each answer labelled.
- **Each §1.1 (c) mutation builds unmodified ArduPilot and is refused by `check pinout`, by name.** `adv-proto-swap` gives two findings, SERIAL1 and SERIAL2. Record each verdict, as `4400b2` §3 did. The unmutated four-port board exits 0.
- **Tier 1, in `pincheck/tests/identity.rs`:**
  - every finding kind;
  - each `SerialProtocol` form: a name, a C integer, an absent define per n with its default;
  - SERIAL7 and 8's ignored define;
  - an RC port with RXINV set, which passes; and with SWAP set, which is refused;
  - two RCIN ports;
  - an inverted console.
- **The AF-prefix refusal:** a tier-1 test per prefix, at the generator and through `generate hwdef`. Show once, measured, that ArduPilot refuses what the generator now refuses first (`4400b` A9's `SPI_FLASH_CS`).
- **Each new guard has a mutant a test kills.** List them.
- **CI:** the three generated boards pass, with `identity: judged`. The four hand boards keep exactly the findings `container/ardupilot-pinout.sh` expects today; that file does not change (§3).
- **Every committed `hwdef.dat` golden is byte-identical** (CI's `cmp`, `ardupilot-image.yml:290`, and `generate.rs`).
- **The fixture diff is comment lines only** (`git diff -U0` shows only `#` lines in the four `.dat` files).
- **`the_generator_calls_none_of_the_check_s_rules` and `pincheck/tests/separation.rs` pass unchanged.**
- **The CLAUDE.md gates pass. CI is green. `TIER2_MIN` is exact:** no tier-2 test is expected.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-06`, `INV-07`; `RUL-16`, `RUL-20`, `RUL-21`; `4400b` F3 (#189); `4400b2` D1 (a) and D2 (#196); `4400d`'s identity judge as built.
- **Fence:**
  - `aethelgard-pincheck`: `src/identity.rs`; `src/header.rs` only if a reader must grow; tests; `README.md`; the four fixtures' comment lines;
  - `aethelgard-hwdef`: `src/lib.rs` (the label refusal, the serial lines); `tests/generate.rs`; `README.md`;
  - `aethelgard-cli`: `README.md` (L134, L520) and `tests/hwdef_tier1.rs`. No CLI source change is expected. If one is needed, it is call-site only; say why;
  - FEEDBACK.
  - **Not:**
    - `aethelgard-ir`: read its types, change nothing. `4400g` owns it;
    - `.github/workflows/` and `ci/`: `1910` and `1911` own them;
    - `container/`, `ardupilot-pinout.sh` included. If the hand boards' findings must move, that is a stop (§7);
    - any committed specification: `examples/**` and `crates/aethelgard-cli/tests/fixtures/*.json`. Derive the four-port board and every mutation inside the test, from committed files, as `hwdef_tier1.rs:222-260` does. A committed specification at 16 breaks when `4400g` takes 17;
    - `aethelgard-twin`, `aethelgard-chaos`, `aethelgard-eda`, the CLI's `simulate`, `foreign` and `campaign`;
    - the steering documents and the root `README.md`.
- **Running beside you:**
  - **`AET-WO-1910` and `1911`** own the workflows, `ci/`, the weights and `TIER2_MIN`. **If you add a tier-2 test, say so on the bus.** The weights row and `TIER2_MIN` are theirs to change, never here.
  - **`AET-WO-4400g`** owns `aethelgard-ir` (specification 16 → 17), the twin, chaos, and the CLI's `simulate`, `foreign` and `campaign`. It edits `crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json` (adds `mcu.foreign_clock`), which `hwdef_tier1.rs` and CI's `sentinel-02` board read. **If you need that fixture changed, or its golden moves under you, merge `main` after `4400g` merges and re-gate.** It may also edit `crates/aethelgard-cli/README.md`: merge, never overwrite.
  - **`AET-WO-1315`** owns `aethelgard-eda` and a carve-out in `ir/src/power.rs`. No overlap.
  - **`AET-WO-2304` and `4240`** are spikes; they change nothing here.
  - You run the ArduPilot image: one container of yours at a time, under `capped-run`.
- One PR, left unmerged. Write `~/.cache/aet-4404/pr-title-body.md`.

## 5. Adversarial pass, targeted

Hand it to a fresh subagent, read-only, with the kit. Probe:
- **The A3 trap:** an expected value read from `hwdef.h`, `hwdef.dat`, or the generator's `protocol()`. Prove each comes from the topology, `ir`, or a cited ArduPilot table.
- **The compiler's reading:** a protocol as `5`, `05`, `0x5`, `AP_SerialManager::SerialProtocol_GPS`, a name the table lacks; a define with odd blanks, under `#if 0`, `#undef`'d, or `#pragma push_macro`'d. Each is read as GCC reads it, or refused (`4400b2` A1, A3, A10; `4400d`'s digraph test).
- **The defaults:** a MAVLink port at SERIAL3 with no define (runs GPS); a GPS port at SERIAL3 with none (passes); a port at SERIAL5 with none (runs nothing).
- **SERIAL7 and 8:** a nonzero `_OPTIONS` define there does nothing, and the judge must not flag it. A nonzero one at SERIAL9 must be flagged.
- **Option bits that move pins:** SWAP, HDPLEX, each pull bit, on a non-RC port and on an RC port.
- **RC input:** two RCIN ports by define; an SBUS port with TXINV set.
- **The label prefix:** each of the fourteen; the case rule; a name that is also a real AF on its pin (a chip select named `SPI1_NSS` on PA4). With the guard disabled, does ArduPilot build it as alternate, and does the GPIO judge catch it?
- **The fixtures:** any test, digest or workflow step that reads the header bytes.
- **Goldens:** any byte of a committed `hwdef.dat` or topology moving.

## 7. Flag-and-stop conditions

- **§1.1:** a protocol or options fact readable only from the generated `hwdef.dat`; or a measured rule that needs something the specification cannot state (a role ArduPilot reads differently, an RX-only inversion, a non-RC inverted port).
- **Any need for a specification or parts-table schema change:** a field, a role, `SPECIFICATION_SCHEMA_VERSION` (16; `4400g` takes 17). Stop with what is needed and why. A second bump would collide.
- **Any change to `aethelgard-ir`.**
- **Any change to `.github/workflows/`, `ci/` or `container/`,** including the hand boards' expected findings in `ardupilot-pinout.sh`. Stop with the findings and why they move.
- **A committed golden, topology or CI board's `hwdef.dat` moves.** Stop with the bytes.
- **The judge needs code from `aethelgard-hwdef`, or the generator from `aethelgard-pincheck`** (`separation.rs`). Stop.
- **ArduPilot's tag reads the options or the protocol somewhere this work order did not cite, in a way that changes a committed board's run.** Stop and report it first.
