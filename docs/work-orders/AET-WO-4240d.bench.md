# AET-WO-4240d — bench sheet: the HSE bring-up in bypass on the Nucleo

Every prediction here was written by the 4240d builder **before** the bench, on Furnace on 2026-11-22. The base is `main` `7fc81640` (specification 23, solved 16), with the pinned tools image (`aethelgard-tools@sha256:e5fc7ddb…1b58`). Fill the blank columns at the bench. **Never edit a prediction: a disagreement is the result** (`INV-07`; `northstar.md` §2).

**What each prediction rests on.** Each row names its source, in `RUL-20`'s terms. Where a row names a ledger class, it is the class `aethelgard-cli`'s ledger gives that field (`crates/aethelgard-cli/src/ledger.rs`). A row's label is never chosen by hand.
- **twin:** the twin's own run (Renode's models and the boot judge), S1 below.
- **rule:** an independent rule in `aethelgard-ir` that shares no code with the generator. The ledger's `independent-rule`: `clock::judge_tree` for the PLL words, `vos::flash_at_reset` for `FLASH_ACR`, `silicon::check_spi` for SPI.
- **generator:** the generated code's own word. This is consistency, not judgement, and only the bench judges it.
  - **Where the ledger says otherwise:** the ledger classes every field of the bring-up's whole `RCC_CR` and `RCC_CFGR` words as `independent-rule` through the twin's HSE clock rule (`aethelgard_twin::hse::judge_clock`, ledger.rs L744, L857–905). That rule is the twin's, not an `aethelgard-ir` rule, so this sheet labels the fields it alone holds (`HSION`, the PLL2/PLL3 bits, `CFGR`'s other fields) **generator**. FEEDBACK F5 says so.
- **manual:** RM0433's stated reset value, read at source: the silicon's documented state, with no project code behind it.
- **twin (Renode reset):** the firmware never writes the register. The expected value is Renode's reset value, as in `3304` B2, so a silicon difference there disagrees with the model's reset value, not with the firmware.
- **repeat:** an earlier bench on this board measured it. The row checks for drift, and it is not a fresh prediction.
- **not predicted:** said plainly, with the reason.

Register citations are to RM0433 Rev 9 (sha256 `672a9433…b6a0`) and DS12117 Rev 10 (sha256 `607b9ead…c032`). The board is revision V (`DBGMCU_IDC` `0x20036450`, `3302`), so DS12117's rev V tables apply.

## 0. Setup and safety

- **What this bench is:** `4240b`'s bring-up from the HSE on silicon for the first time, in **bypass**.
  - The ST-LINK-V3E's `MCO` drives the H753's `PH0` through SB45. That is the board's default (UM2407 Rev 6 §7.5.1, p. 26).
  - `mcu.hse` is `external`, and the firmware is a variant of `3304`'s `sentinel-02-bench` (`4240` D7 (c)).
- **It witnesses:**
  - the bring-up's register words on silicon;
  - `SWS` = PLL1;
  - a clock out at the predicted ratio to the source;
  - SPI's SCK at the predicted rate.
- **It cannot witness** (§B7):
  - a crystal's ppm, because the source is the ST-LINK MCU's own RC oscillator;
  - the `HSERDY` timeout path;
  - CSS;
  - the time `HSERDY` and `PLL1RDY` take.
- Everything is 3.3 V. Power comes from the ST-LINK USB only, and **nothing touches 5V**.
- **No solder bridge, no jumper, and no ST-LINK firmware change.** The ST-LINK's `MCO` setting is read (B-0, B-1), never changed.
- Change wiring only with the USB unplugged.
- **The instruments:** the FX2 analyzer (24 MSa/s; one sample is 41.7 ns) with `sigrok-cli` 0.7.2, and `probe-rs` 0.32.0 at `~/tools/probe-rs/bin/probe-rs` on t9o777.
- **Chip for `probe-rs`:** `STM32H753ZITx`.

```
P=~/tools/probe-rs/bin/probe-rs
C="--chip STM32H753ZITx"
```

**The `probe-rs` commands, read at source** (`probe-rs-tools` v0.32.0, `src/bin/probe-rs/cmd/`; copies and sums in `~/.cache/aet-4240d/probe-rs/`):
- `read` and `write` take the width, then the address: `$P read $C b32 0x58024400 1`, `$P write $C b32 0x58024410 0x00900000`. Neither asks for a halt (`cli.rs` `attach_probe`, L59–113). Whether the probe server halts the core on attach is not held. Nothing below depends on it: every write is to a register the running firmware no longer touches.
  - `write`'s help text says "Only supports RAM addresses". It writes through the same `write_memory_32` as any address, and flash is not programmed by it.
  - **After every write in this sheet, read the register back.** A write that did not take is a stop for that step: record it.
- `reset` is `core.reset()`. For a Cortex-M that is `AIRCR.SYSRESETREQ` (`probe-rs` `architecture/arm/sequences.rs`, `cortex_m_reset_system`).
  - That is a system reset, which "resets all registers to their reset values unless otherwise specified in the register description" (RM0433 §8.4.2, p. 330).
  - A system reset leaves `PWR_CR3` (POR only, `4240v`) and the `DBGMCU` registers ("not reset by a system reset, only by a power on reset", p. 3195; `probe-rs` writes `DBGMCU_CR` on every attach). The bench never writes `PWR_CR3`.
- **Word 0 survives a warm reset.** `link.x` places the boot record in `.aethelgard_boot (NOLOAD)`, which the reset handler never initialises, and no held source says a system reset clears it. A parked boot writes nothing, so the previous run's `0x2100b007` would remain. **So before every reset whose word 0 is then read, write word 0 to 0 and read it back as 0** (`$P write $C b32 0x20000000 0`).
- `list` prints each probe's name, `VID:PID` and serial (`cmd/list.rs`). Whether that text carries the firmware version is not stated there.

**Counting edges.** Use `edges.py`, committed at `spike/4240d/edges.py` and copied beside the ELF. It reads `sigrok-cli … -O binary`, which is the logic payload as is: one byte per sample in fx2lafw's 8-bit mode, channel Dn at bit n. That is libsigrok 0.5.2 `src/output/binary.c` and `src/hardware/fx2lafw/protocol.c`, read at source, copies in `~/.cache/aet-4240d/sigrok/`.
- It prints the rising edges and the shortest high and low runs.
- **A shortest run of 1 sample means the count is not trusted.** Repeat the capture.
- If `sigrok-cli` reports dropped samples, repeat at the same rate. Do not lower the rate for B4's 4.3 MHz.

## 1. The firmware

**ELF:** `sentinel-02-bench-hse8.elf`, sha256 `4823bd30503416da7dc84e2178433a1d6d9b7dd8f2960b3ae69b4640196d6530`.
- **Source:** generated from `examples/sentinel-02-bench-hse8.json` (sha256 `fefb053a6e3fea16f770b197c2fa8954d5190870031f164e6601c6586d7e53c5`).
- **The specification:** `sentinel-02-bench` (`3304`'s bench) with `mcu.hse = external`:
  - 8,000,000/1 Hz at ±80,000 ppm;
  - start-up bound 20,000 µs, as ruled 2026-11-20;
  - `board.drawn: false`, since the Nucleo is the board (`4240b` D-9).
- **The ppm source** (ruled 2026-11-22T04:00Z):
  - RN0093 Rev 40 §4.16 p. 13: the `MCO` default is "HSI / 2 = 8 MHz";
  - DS11853 Rev 8, Table 42 p. 130: the STM32F723's ACCHSI is −8/+4.5 % over −40…105 °C. The −8 % side is declared.
  - Held from a distributor's mirror (st.com refuses Furnace). Rev 7 agrees. Rev 8's currency is unconfirmed.
- **The build:** `main`'s CLI in the pinned image built it (`generate`, then `build`), twice, at two different paths, with one sha.
  - The board crate's input hash is `c669cefc0800e60813ec3add829132ac26ebe003b58c366d241867e81286e53a`.
  - The image hash `simulate` reports is `f71e355d81410b092d31f3a83f01fa2e92b645a7abd745c88acc05491f886d95`.
- **S1:** `simulate --firmware … --for 100ms` on the emitted platform (`.repl` sha256 `1c493e8d…f553`). Verdict **`pass`**, no findings.
  - The boot word is `0x2100b007` at 41,539,110 ns virtual. That is 2,499,230 ns after `main`'s `sentinel-02-bench` (39,039,880 ns), from the 20 ms wait counted at the source's fastest (1,382,400 spins against 1,299,180).
  - Every record, witness line and register row equals `main`'s `sentinel-02-bench` run. Peak RSS is 666,332 KiB.
  - It is saved as `bench/simulate-verdict.json`. Its residuals name the HSE and PWR limits (§B7).
- **No `verify`:** a `board.drawn: false` specification has no board for ERC, DRC or extract. So, unlike `3304`'s, this ELF carries no `INV-03` claim. It is a bench image, not a fabrication pair.

**The image for B-1:** `sentinel-02-bench.elf`, sha256 `3f7b0507bf933c68a5bb42954e23c56968757b238377ec2abea12b2083b38dc5`.
- That is `main`'s `examples/sentinel-02-bench.json`, built here in the same run. It is the HSI-only image `3304`'s bench runs since `4240v` (no longer `375fd31a`).
- It never writes `RCC_CR` or `RCC_CFGR` (its `board.rs`, read).

**On Furnace:** `~/.cache/aet-4240d/bench/`, beside `SHA256SUMS`, both verdicts, the topology, the platform and `edges.py`.

```
scp furnace:.cache/aet-4240d/bench/{sentinel-02-bench-hse8.elf,sentinel-02-bench.elf,edges.py,SHA256SUMS} .
sha256sum -c --ignore-missing SHA256SUMS
```

## B-0. The ST-LINK firmware version (read only)

RN0093 Rev 40 ties the `MCO`'s source to the ST-LINK firmware:
- **§4.16, V3J7M2 (pp. 12–13):** "Possibility added to choose the MCO clock output from ST-LINK to target among 3 values. The default value (HSI / 2 = 8 MHz) can be changed … to HSE / 3 = 8.33 MHz (for better clock accuracy) or cut off."
- **§4.8, V3J14M5 (p. 9):** "Rework of MCO clock selection, with new possible choices (HSE/4 and HSE/5)."
- The same §4.16 adds "Display of STMicroelectronics firmware version string into file details.txt".

Run each, and paste what it prints. None changes anything.

```
$P --version
$P list
lsusb -v -d 0483: 2>/dev/null | grep -E 'idProduct|bcdDevice|iProduct|iSerial'
# The ST-LINK's mass-storage volume (NOD_H753ZI or similar), if the desktop mounted it.
# RN0093 writes the name "details.txt"; its case depends on the mount, so list first:
ls /media/$USER/NOD_*/
cat /media/$USER/NOD_*/[Dd][Ee][Tt][Aa][Ii][Ll][Ss].[Tt][Xx][Tt]
```

| Read | Predicted | Source | Bench |
|---|---|---|---|
| `probe-rs list` | one STLINK-V3 probe; **no** firmware version in the line | `cmd/list.rs` prints the probe's `Display` (name, `VID:PID`, serial), read at source; that `Display` was not read | |
| `DETAILS.TXT` | a version string `V3J…M…`; **not predicted** which | RN0093 §4.16 p. 13 | |
| `bcdDevice` | **not predicted** (the mapping from `bcdDevice` to a version is not held) | — | |

**What B-0 decides:** nothing alone. It corroborates B-1.
- A version at or after V3J7 defaults to HSI/2 [inferred: §4.16 states the default at V3J7, and §4.8's V3J14 rework does not restate it], unless someone changed it with STLinkUpgrade, which this bench does not do. §4.11 (p. 10) records "Fixed the MCO change to a nondefault value ignored on some STLINK-V3E boards", so a changed setting may not have taken: B-1 measures what is there.
- A version before V3J7 says UM2407's "fixed at 8 MHz" (§7.5.1 p. 26), with no source stated.

## B-1. The `MCO` frequency at the H753: the gate

**Read this before anything else is flashed. If B-1 says stop, do not flash `sentinel-02-bench-hse8.elf`.**

The source is an RC oscillator, the F723's HSI. So "8.000 MHz within the analyzer's resolution" cannot hold: DS11853 gives ±1 % even at 25 °C, factory calibrated and unsoldered.
- HSE/3 (8.333 MHz, +4.17 %) lies inside both the declared ±80,000 ppm and the −8/+4.5 % row.
- So **HSE/3 is told apart from HSI/2 by its exact ratio, not by the band**: the ST-LINK's 25 MHz oscillator is ±50 ppm (MB1364 BOM row 100), and with the analyzer's error it is far inside 0.1 %.
- The gate below is the architect's ruling (Q2, 2026-11-22T04:00Z).

**Look first: is CN12 fitted?** UM2407 Rev 6 §9.2 (p. 43) says the morpho headers CN11 and CN12 are "both soldered by default". MB1364's BOM (H753ZI, C-01 and E-01, row 106) says "CN11, CN12 … Not Fitted". The two disagree, so look at the board.
- **If CN12 has pins:** the analyzer's GND goes to CN12 pin 20, and D0 to CN12 pin 23, which is `PA8` (UM2407 Table 22, p. 43).
- **If it does not:** D0 goes by a hook clip on test point TP4 (`PA8`, UM2407 Table 12 p. 30), and GND on CN7 pin 8.
- **If neither can be reached without solder, stop** (WO §7). `PA8` is on no other pin.

**Probe.** Unplug the USB, then connect as above.
- Nothing else is on `PA8`: UM2407 Table 12 (p. 30) gives it no bridge, only test point TP4.
- MB1364's `PA8` net runs to TP4 and the connector sheet.

**1. A known image.** Flash `sentinel-02-bench.elf` (`3f7b0507…`), as in `3304`'s B1, even if the board seems to run it already. Word 0 alone cannot tell the two images apart: both write `0x2100b007`.

```
$P download $C sentinel-02-bench.elf
$P write $C b32 0x20000000 0
$P read  $C b32 0x20000000 1            # expect 0x00000000
$P reset $C
```

Wait at least 2 s.

**2. Read before writing.** Each write below is the whole word that a read-modify-write of the predicted read gives. **If any read differs from its prediction, do not write. Record it and stop B-1.** A stop after any write in step 3 still runs step 6's reset.

| Read | Address | Mask | Predicted | Source | Bench |
|---|---|---|---|---|---|
| word 0 | `0x20000000` | `0xffffffff` | `0x2100b007` | twin (`bench/sentinel-02-bench.simulate-verdict.json`) | |
| `RCC_CR`: every writable bit | `0x58024400` | `0x150d129b` | `0x00000001`: `HSION` alone, so HSE, CSS and every PLL off | manual: reset `0x0000_0001` (§8.7.2 p. 383); this image never writes `CR` (its `board.rs`) | |
| `RCC_CFGR` | `0x58024410` | `0xffffffff` | `0x00000000` (`SW`/`SWS` HSI; `MCO1PRE` 0000, "prescaler disabled", so no output [inferred]) | manual: reset value (§8.7.7 pp. 391–392); never written by this image | |
| `RCC_AHB4ENR` | `0x580244e0` | `0xffffffff` | `0x0000000b` (GPIOA, B, D) | twin (observed: GPIOA, GPIOB); generator (GPIOD), as `3304` B2 | |
| `GPIOA_MODER` | `0x58020000` | `0xffffffff` | `0xabffebff`: PA5 and PA6 AF, every other pin at reset, PA8 `11` | manual: reset `0xABFF_FFFF` for port A (§11.4.1 p. 541); generator: PA5/PA6 | |
| `GPIOA_AFRH` | `0x58020024` | `0xffffffff` | `0x00000000`, so PA8 AF0 | manual: reset value (§11.4.10 p. 546); no pin 8–15 configured | |

**3. Route `MCO1` = `hse_ck` ÷ 4 to `PA8`.** These are `4240` FEEDBACK §2's writes, in its order. `MCO1` is set "only after reset, before enabling the external oscillators and the PLLs", as §8.7.7 p. 391 recommends.

```
$P write $C b32 0x58024410 0x00900000   # RCC_CFGR: MCO1[24:22] = 010 (hse_ck), MCO1PRE[21:18] = 0100 (/4)
$P read  $C b32 0x58024410 1            # expect 0x00900000
$P write $C b32 0x58020000 0xabfeebff   # GPIOA_MODER: PA8 [17:16] = 10 (alternate function)
$P read  $C b32 0x58020000 1            # expect 0xabfeebff
$P write $C b32 0x58024400 0x00040001   # RCC_CR: HSEBYP (bit 18), written while HSEON = 0 (p. 384)
$P read  $C b32 0x58024400 1            # expect & 0x000d0000 = 0x00040000
$P write $C b32 0x58024400 0x00050001   # RCC_CR: HSEON (bit 16)
$P read  $C b32 0x58024400 1            # expect & 0x000f0000 = 0x00070000 (HSEBYP, HSERDY, HSEON)
```

Each field is cited:
- `MCO1` and `MCO1PRE`: §8.7.7 pp. 391–392;
- PA8 AF0 = `MCO1`: DS12117 Table 9 p. 89;
- the MCO pin in alternate function: §8.5.4 p. 345;
- `GPIOAEN` is already 1, so there is no `AHB4ENR` write;
- AF0 is the reset value, so there is no `AFRH` write;
- 2 MHz is under the pin's 12 MHz at `OSPEEDR` 00 (DS12117 Table 149 p. 245, rev V, C = 50 pF, 2.7–3.6 V), so there is no speed write.

If `HSERDY` reads 0, no clock is reaching `PH0`: record it, run step 6, and stop. That is the bench's answer to whether SB45 and the `MCO` are as UM2407 says.

**4. Capture 1 s and count:**

```
sigrok-cli -d fx2lafw --config samplerate=24m --samples 24000000 -C D0 -O binary -o b1.bin
python3 -I edges.py count b1.bin
```

**5. The gate.** Let E be the rising edges, normalised to 24,000,000 samples (E = edges × 24,000,000 / the samples `edges.py` prints). The 1-s count gives the `MCO` frequency as 4·E Hz.

| E | Meaning | Action |
|---|---|---|
| 0 | `MCO` cut off, or not reaching `PH0` | **stop: do not flash** |
| within ±0.1 % of 2,083,333 (2,081,250 to 2,085,416) | HSE/3 = 8.333 MHz, the ST-LINK's 25 MHz source ÷ 3 (MB1364 BOM row 100: X1, "Oscillator 25Mhz, +-50ppm", NDK NZ2520SH) | **stop: do not flash** (the specification declares 8,000,000/1 Hz) |
| within ±0.1 % of 1,562,500 (1,560,938 to 1,564,063) | HSE/4 = 6.25 MHz | **stop: do not flash** |
| within ±0.1 % of 1,250,000 (1,248,750 to 1,251,250) | HSE/5 = 5 MHz | **stop: do not flash** |
| outside 2,000,000 ± 4 % (1,920,000 to 2,080,000) | not the HSI/2 the specification declares, at an indoor temperature (DS11853 Table 42's −10…85 °C row, ±4 %) | **stop: do not flash** |
| otherwise | HSI/2 | **proceed.** Record E and its deviation, (E − 2,000,000)/2,000,000 |

The three HSE windows lie outside the ±4 % band. Each is named so a stop says which source it found.

**Predicted:** E inside 1,980,000 to 2,020,000, the ±1 % of DS11853's 25 °C row.
- Source: DS11853 Table 42 p. 130. That row is "factory calibrated, parts not soldered", so it is a prediction for the record, not the gate.
- The analyzer's own timebase adds its crystal's error. That figure is uncited, and is tens of ppm at most for a typical part [inferred].

E: ________ · shortest high / low run: ________ · deviation: ________ ppm · gate: proceed / stop (which row)

**6. Restore:** `$P reset $C`, wait 2 s, then read:

| Read | Address | Mask | Predicted | Source | Bench |
|---|---|---|---|---|---|
| `RCC_CR` | `0x58024400` | `0x000d0000` | `0x00000000` (`HSEON`, `HSEBYP` and `HSECSSON` cleared) | manual: §8.4.2 p. 330; `HSECSSON` "disabled by a system reset" (p. 384) | |
| `RCC_CFGR` | `0x58024410` | `0xffffffff` | `0x00000000` | manual: §8.4.2 and reset value | |
| `GPIOA_MODER` | `0x58020000` | `0xffffffff` | `0xabffebff` (PA8 back to analog) | manual: reset value, then the image's PA5/PA6 (generator) | |

## B2. Flash and boot (only if B-1 says proceed)

Leave the analyzer on PA8 or remove it; nothing in B2 depends on it.

```
sha256sum sentinel-02-bench-hse8.elf   # 4823bd30…6530
$P download $C sentinel-02-bench-hse8.elf
$P write $C b32 0x20000000 0
$P read  $C b32 0x20000000 1            # expect 0x00000000
$P reset $C
```

Wait at least 2 s, then read the record:

```
$P read $C b32 0x20000000 20
```

**The bring-up, in order** (generated `board.rs` `Hse::start`, read; `4240b` P3, confirmed there for `hse-external-8`; the same words here). The firmware writes each whole word once:
1. `PWR_CR3`, then `VOS` Scale 2 with its two bounded waits (`4240v`);
2. `RCC_CR` `0x0004_0001` (`HSEBYP`), then `0x0005_0001` (`HSEON`);
3. at most 1,299,180 reads of `HSERDY`;
4. `0x000D_0001` (`HSECSSON`);
5. `PLLCKSELR` `0x0000_0052`, `PLLCFGR` `0x0003_0002`, `PLL1DIVR` `0x0103_069F`;
6. `0x010D_0001` (`PLL1ON`), then at most 9,744 reads of `PLL1RDY`;
7. `FLASH_ACR` `0x0000_0011`, read back;
8. `RCC_CFGR` `0x0000_0003` (`SW` = PLL1), then at most 9,744 reads of `SWS`;
9. then the 3304 bring-up of I2C1 and SPI1, on the HSE's clock.

**Word 0 says how far it got.** These are the generated `main.rs` constants, which are the twin's `boot` words:

| Word 0 | Meaning | What the bench would show by it |
|---|---|---|
| `0x2100b007` | **predicted**: the whole bring-up, then every device step (twin S1; generator) | — |
| `0x2100b0f1` | `SUPPLY_NOT_DECLARED` | not reachable here: the specification declares `ldo` |
| `0x2100b0f2` / `0x2100b0f3` | `ACTVOSRDY` / `VOSRDY` timeout (`4240v`) | these words may not land on silicon (`4240v` FEEDBACK); read `PWR_CR3`/`CSR1` as in B3 |
| `0x2100b0f4` | `HSE_NOT_READY`: `HSERDY` not set within 1,299,180 reads | no clock on `PH0`. It halts after **at least** 20 ms (`4240b` A11: one read is at least one cycle at the HSI's fastest). B-1 makes it unlikely |
| `0x2100b0f5` | `PLL1_NOT_LOCKED`: `PLL1RDY` not set within 9,744 reads | a PLL that does not lock on these words: a disagreement with `4240` §4 |
| `0x2100b0f8` | `FLASH_ACR_NOT_TAKEN`: `FLASH_ACR` did not read back `0x11` under `0x3F` | the flash interface refusing the latency word |
| `0x2100b0f6` | `SW_NOT_TAKEN`: `SWS` not PLL1 within 9,744 reads | the switch refused |
| `0x2100b0f7` | `HSE_CSS_FAULT`: the CSS NMI (`css_hold`) | the HSE failed after `HSECSSON`. The hardware has already switched `sys_ck` to the HSI and disabled the PLLs (RM0433 §8.5.3 p. 344). The bench has no motor, so `css_hold` drives nothing that matters |
| `0x00000000` | parked before any word: `Board::take` or `reset_clock` refused (they write nothing; word 0 keeps the 0 written before the reset) | **not predicted** |
| anything else | not a word this firmware writes | **not predicted** |

Word 0: ________ · words 2–19 (the records): ________

**The records, nothing wired:** as `3304` B2's "nothing wired" table, row for row. The device steps are the same image's, unchanged by the clock (S1's records equal `main`'s).
- U7 records `0x22020001` (TXIS) three times.
- U5 records `0x22020000` (OK), with a value the floating PA6 gives.

If the sensors are wired as `3304` B4, the records are `3304`'s "records, wired" table instead.

## B3. The register read-back after boot

One read each, for example `$P read $C b32 0x58024400 1`. Compare **only the masked bits**. **Read all of B3 before B4 writes anything.**

| Register | Address | Mask | Expected | Source | Bench raw | & mask | Agree? |
|---|---|---|---|---|---|---|---|
| word 0 | `0x20000000` | `0xffffffff` | `0x2100b007` | twin (S1) | | | |
| `RCC_CR` | `0x58024400` | `0x3f0f0001` | `0x030f0001` | per bit, below | | | |
| `RCC_CFGR` | `0x58024410` | `0xffffffff` | `0x0000001b`: `SWS` 011 (PLL1), `SW` 011, `MCO1PRE` 0000, every other field 0 | `SW`: twin (observed). `SWS`: the twin's clock rule reads it, and Renode's `SWS` follows `SW` at once, so **silicon's `SWS` = PLL1 is first witnessed here** (§8.7.7 p. 393). The rest: generator (`CFGR` written whole, `0x3`) | | | |
| `RCC_PLLCKSELR` | `0x58024428` | `0x03f3f3f3` | `0x00000052`: `PLLSRC` 10 (HSE), `DIVM1` 5, `DIVM2` and `DIVM3` 0 | rule (`clock::judge_tree`; ledger `independent-rule`), also observed. Fields §8.7.11 p. 398, reset `0x0202_0200` | | | |
| `RCC_PLLCFGR` | `0x5802442c` | `0x01ff0fff` | `0x00030002`: `DIVP1EN` and `DIVQ1EN` 1, `DIVR1EN` 0, PLL2/PLL3 outputs off, `PLL1RGE` 00 (1–2 MHz), `PLL1VCOSEL` 1 (medium), `PLL1FRACEN` 0 | rule (`judge_tree`). **The twin is blind here:** Renode reads `0x01FF_0000` whatever is written (ledger: a tag; `4240b` §2.2). Fields §8.7.12 pp. 400–402, reset `0x01FF_0000` | | | |
| `RCC_PLL1DIVR` | `0x58024430` | `0x7f7fffff` | `0x0103069f`: `DIVN1` 159 (×160), `DIVP1` 3 (÷4), `DIVQ1` 3 (÷4), `DIVR1` 1 (reset) | rule (`judge_tree`), also observed. Fields §8.7.13 pp. 403–404, reset `0x0101_0280` | | | |
| `RCC_D2CCIP1R` | `0x58024450` | `0x00007000` | `0x00000000`: `SPI123SEL` 000, `pll1_q_ck` | ignored-by-model (a reset-row tag; the twin reads 0 whatever silicon holds). The kernel `check_spi` assumes is `pll1_q` (topology `spi_controllers.SPI1.kernel`). §8.7.20 pp. 413–414 | | | |
| `FLASH_ACR` | `0x52002000` | `0x0000003f` | `0x00000011`: `LATENCY` 1, `WRHIGHFREQ` 01 | rule (`vos::flash_at_reset`, Table 17's VOS2 ]55;110] MHz row). Reset `0x37` (§4.9.1 p. 201) | | | |
| `PWR_CR3` | `0x5802480c` | `0x00000007` | `0x00000002`: `LDOEN` 1, `BYPASS` 0, `SCUEN` 0 | rule (ledger `independent-rule` for `PWR.CR3`); `4240v` FEEDBACK §0.4 predicts the whole word `0x42`, inferred for bit 6. §6.8.4 p. 307 | | | |
| `PWR_CSR1`: `ACTVOS` | `0x58024804` | `0x0000c000` | `0x00008000`: `ACTVOS` 10 | manual: `ACTVOS` reflects the `VOS` applied (§6.8.2 p. 305); no ledger row. `4240v` §0.4 predicts the same | | | |
| `PWR_CSR1`: `ACTVOSRDY` | `0x58024804` | `0x00002000` | `0x00002000` | silicon-only (ledger: `PWR.CSR1.ACTVOSRDY`, polled) | | | |
| `PWR_D3CR`: `VOS` | `0x58024818` | `0x0000c000` | `0x00008000`: Scale 2 | rule (ledger `independent-rule`, `PWR.D3CR.VOS`). §6.8.6 p. 310 | | | |
| `PWR_D3CR`: `VOSRDY` | `0x58024818` | `0x00002000` | `0x00002000` | silicon-only (ledger: `PWR.D3CR.VOSRDY`, polled) | | | |
| `SPI1.CFG1` | `0x40013008` | `0x7000001f` | `0x40000007`: `MBR` 4 (/32), `DSIZE` 7 | rule (`silicon::check_spi`), as `3304` B2. The same `MBR` as at the HSI: the kernel is 64 MHz in both | | | |
| `SPI1.SR` (`MODF`, bit 9) | `0x40013014` | `0x00000200` | `0x00000000` | ignored-by-model: the scar's row, as `3304` | | | |
| `I2C1.TIMINGR` | `0x40005410` | `0xffffffff` | `0x10d81340` | rule (ledger `independent-rule`: `aethelgard_ir::silicon::check_timingr` holds its fields by inequalities); the exact word is the generator's, so this row fails only on a read-back difference. Not `3304`'s `0x10C7123C`: the solve takes the ±80,000 ppm budget's fastest kernel | | | |
| `GPIOA_MODER`, whole | `0x58020000` | `0xffffffff` | `0xabffebff` (B4's write needs exactly this) | manual: reset `0xABFF_FFFF` (p. 541); generator: PA5/PA6 | | | |
| `GPIOA_AFRH`, whole | `0x58020024` | `0xffffffff` | `0x00000000` (PA8 AF0; B4 writes none) | manual: reset value (p. 546) | | | |
| `DBGMCU_IDC` | `0x5c001000` | `0xffffffff` | `0x20036450` (rev V) | repeat (`3302`) | | | |

**`RCC_CR`, bit by bit** (§8.7.2 pp. 383–386). Reset `0x0000_0001`. Each class is the ledger's:

| Bit | Field | Expected | Source |
|---|---|---|---|
| 0 | `HSION` | 1 | generator: written 1 in every `CR` word (the ledger's class is the twin clock rule's, see the legend). The HSI stays on, unused after the switch |
| 16 | `HSEON` | 1 | twin (observed) |
| 17 | `HSERDY` | 1 | silicon-only: the bench's first witness that the bypass clock was taken |
| 18 | `HSEBYP` | 1 | ignored-by-model (a tag in Renode, so crystal and bypass run the same there) |
| 19 | `HSECSSON` | 1 | silicon-only (the model stores the flag with no detection) |
| 24 | `PLL1ON` | 1 | twin (observed) |
| 25 | `PLL1RDY` | 1 | silicon-only |
| 26–29 | `PLL2ON`, `PLL2RDY`, `PLL3ON`, `PLL3RDY` | 0 | generator (written 0; as `HSION`) |

**Not predicted, for the record:** bits 2 (`HSIRDY`), 5 (`HSIDIVF`), 8 (`CSIRDY`), 13 (`HSI48RDY`), 14 (`D1CKRDY`) and 15 (`D2CKRDY`; the ledger takes silicon to have set it, `3302`).

**The boot judge's register image:** S1's 20 rows equal `main`'s `sentinel-02-bench` run, row for row (measured), and therefore `3304` B2's table. Read them as there:

| Register | Address | Mask | Expected | Source | Bench & mask |
|---|---|---|---|---|---|
| GPIOA.MODER | `0x58020000` | `0x00003c00` | `0x00002800` | generator | |
| GPIOA.AFRL | `0x58020020` | `0x0ff00000` | `0x05500000` | generator | |
| GPIOB.MODER | `0x58020400` | `0x0000fc00` | `0x0000a800` | generator | |
| GPIOB.AFRL | `0x58020420` | `0xfff00000` | `0x44500000` | generator | |
| GPIOD.MODER | `0x58020c00` | `0x30000000` | `0x10000000` | generator | |
| RCC.AHB3ENR | `0x580244d4` | `0xffffffff` | `0x00000000` | twin (Renode reset) | |
| RCC.AHB1ENR | `0x580244d8` | `0xffffffff` | `0x00000000` | twin (Renode reset) | |
| RCC.AHB2ENR | `0x580244dc` | `0xffffffff` | `0x00000000` | twin (Renode reset) | |
| RCC.AHB4ENR | `0x580244e0` | `0xffffffff` | `0x0000000b` | twin (observed: GPIOA, GPIOB); generator (GPIOD) | |
| RCC.APB3ENR | `0x580244e4` | `0xffffffff` | `0x00000000` | twin (Renode reset) | |
| RCC.APB1LENR | `0x580244e8` | `0xffffffff` | `0x00200000` | twin (observed) | |
| RCC.APB1HENR | `0x580244ec` | `0xffffffff` | `0x00000000` | twin (Renode reset) | |
| RCC.APB2ENR | `0x580244f0` | `0xffffffff` | `0x00001000` | twin (observed) | |
| RCC.APB4ENR | `0x580244f4` | `0xffffffff` | `0x00010000` | twin (Renode reset) | |
| SysTick.CSR | `0xe000e010` | `0x00010003` | `0x00000000` | twin (Renode reset) | |
| NVIC.ISER0–4 | `0xe000e100`–`0xe000e110` | `0xffffffff` | `0x00000000` each | twin (Renode reset) | |

## B4. A clock out: `MCO1` = `pll1_q` ÷ 15 on `PA8`

`pll1_q` is `sys_ck` under H1: `DIVQ1` = `DIVP1` = 3, so both are the VCO ÷ 4. `MCO1` cannot select `sys_ck`; `MCO2` on `PC9` can (§8.7.7 p. 391, `4240` S4).
- The PLL is integer: p. 347's integer mode is `SH_REG` = 0, which is latched from `FRACN1` only on a 0 → 1 edge of `PLL1FRACEN`. The firmware writes `PLL1FRACEN` 0 and never sets it, so FVCO = F_ref × (`DIVN1` + 1) and F_q = FVCO / (`DIVQ1` + 1) (§8.5.5 p. 347).
- So `pll1_q` = F_HSE ÷ 5 × 160 ÷ 4 = **8 × F_HSE exactly**, whatever the F723's HSI is doing.
- The sharp prediction is therefore a **ratio** to F_HSE, measured on the same pin within a minute.

**Why N = 15.** At the declared source's fastest (8 MHz × 1.08), `pll1_q` is 69.12 MHz.
- **The pin's speed** (§8.5.4 p. 345): ≤ 12 MHz at `OSPEEDR` 00 (DS12117 Table 149 p. 245), so N ≥ 6 with no speed write.
- **The duty cycle:** RM0433 states none for the MCO prescaler (§8.5.4 and `MCO1PRE`, p. 392). So assume an odd N splits as ⌊N/2⌋ and ⌈N/2⌉ cycles.
- **Two samples per phase at 24 MSa/s** need ⌊N/2⌋ / 69.12 MHz ≥ 83.3 ns, so ⌊N/2⌋ ≥ 6 and N ≥ 12.
- **The largest prescaler is 15** (p. 392), which leaves the most room: 7 cycles is 101 ns, or 2.4 samples, at the fastest, and the edge is ≤ 16.6 ns (Table 149).
- **Measured on a synthetic capture** (`edges.py synth`, 2,400,000 samples):
  - at 64 MHz ÷ 15, with 7/15 or 8/15 duty, the counts are within 1 of ideal, with a shortest run of 2 samples;
  - at 69.12 MHz ÷ 15, also within 1, shortest run 2.
- At 64 MHz ÷ 15 = **4.2667 MHz**, a 1-s capture holds about 4,266,667 edges.

**The glitch.** RM0433 says `MCO1`/`MCO1PRE` changes "may generate glitches on MCO1" while the PLLs run (p. 392). The firmware writes `CFGR` whole at the switch, so the output cannot be set before it.
- The glitch is on `PA8` alone, before each capture starts. It does not touch `sys_ck`: `SW` stays 011 in every word written below.

**The bracket: three captures, back to back.** B3's `RCC_CFGR` read must be `0x0000001b`, and its whole `GPIOA_MODER` read `0xabffebff`, first. If either is not, do not write; record it.

```
$P write $C b32 0x58024410 0x00900003   # MCO1 = 010 hse_ck, MCO1PRE = 0100 (/4), SW = 011 kept
$P read  $C b32 0x58024410 1            # expect & 0x01fc003f = 0x0090001b
$P write $C b32 0x58020000 0xabfeebff   # PA8 alternate function (B3's GPIOA.MODER read must be 0xabffebff)
$P read  $C b32 0x58020000 1            # expect 0xabfeebff
sigrok-cli -d fx2lafw --config samplerate=24m --samples 24000000 -C D0 -O binary -o c1.bin
$P write $C b32 0x58024410 0x00fc0003   # MCO1 = 011 pll1_q, MCO1PRE = 1111 (/15), SW = 011 kept
$P read  $C b32 0x58024410 1            # expect & 0x01fc003f = 0x00fc001b
sigrok-cli -d fx2lafw --config samplerate=24m --samples 24000000 -C D0 -O binary -o c2.bin
$P write $C b32 0x58024410 0x00900003   # back to hse_ck / 4
$P read  $C b32 0x58024410 1            # expect & 0x01fc003f = 0x0090001b
sigrok-cli -d fx2lafw --config samplerate=24m --samples 24000000 -C D0 -O binary -o c3.bin
for f in c1 c2 c3; do python3 -I edges.py count $f.bin; done
$P read  $C b32 0x20000000 1            # word 0 still 0x2100b007 (no CSS fault meanwhile)
```

Bit positions: `MCO1[24:22]` 011 is `0x00c0_0000`, and `MCO1PRE[21:18]` 1111 is `0x003c_0000`. `GPIOAEN` (`AHB4ENR` = `0x0b`, B3) and AF0 (`AFRH` at reset) need no write.

| Quantity | Predicted | Source | Bench |
|---|---|---|---|
| C1, C3 (HSE ÷ 4) | each equal to B-1's E within the HSI's drift over the minutes between. **Not predicted** beyond that: DS11853 gives no short-term stability | — | C1 ____ · C3 ____ |
| C2 (`pll1_q` ÷ 15) | **C2 = (32/15) × (C1 + C3)/2**, within ± ((16/15) × \|C1 − C3\| + 4) | **rule:** RM0433's integer PLL formula (p. 347) over the words B3 reads back, which `clock::judge_tree` judged (64 MHz from 8 MHz). The 4 covers edge quantization: ±1 in C2 plus (32/15) × ±1 in the mean of C1 and C3, at most 3.13. The \|C1 − C3\| term brackets the source's drift, **assuming it drifts monotonically over the three captures** [inferred; DS11853 states no short-term stability]. The analyzer's timebase cancels in the ratio | C2 ____ |
| C2, absolute | 4,266,667 at exactly 8.000 MHz; 4,096,000 to 4,437,334 inside B-1's gate (±4 %) | rule, as above, times B-1's band | |
| shortest run in C2 | ≥ 2 samples (7 cycles of `pll1_q` is 2.4 to 2.7 samples) | the synthetic capture above | |

A C2 outside the ratio's band is a disagreement with `4240` §4's decode of the PLL words, or with the words themselves. B3's reads say which.

## B5. SPI SCK at the predicted rate

`SPI1` takes `pll1_q` (`SPI123SEL` 000, B3) at `MBR` 4 (/32): SCK = 8 × F_HSE / 32 = **F_HSE / 4, the same frequency as C1**.

**Wiring:** analyzer D2 on CN7 pin 10 (`PA5`, SCK) and D5 on CN7 pin 16 (`PD14`, CS), GND as before, exactly as `3304` B4's channel map. The ICM-42688-P is optional. With nothing wired, the master clocks the same frames (`3304` B2), so the SCK is the same.

**Capture through a reset,** untriggered, as `3304` B5 (its reasons hold). Zero word 0 first, so B5's boot is read as its own:

```
$P write $C b32 0x20000000 0
sigrok-cli -d fx2lafw --config samplerate=24m --channels D0,D1,D2,D3,D4,D5 --time 4s -o hse8-spi.sr
# second terminal, within ~1 s:
$P reset $C
# after the capture: word 0 is 0x2100b007 again
$P read  $C b32 0x20000000 1
sigrok-cli -i hse8-spi.sr -O binary -o hse8-spi.bin
python3 -I edges.py frames hse8-spi.bin --bit 2 --gate 5
sigrok-cli -i hse8-spi.sr -P spi:clk=D2:mosi=D3:miso=D4:cs=D5:cpol=0:cpha=0 -A spi=mosi-transfer
```

| Quantity | Predicted | Source | Resolution | Bench |
|---|---|---|---|---|
| CS-low frames and their SCK rising edges | 5 frames: 16, 16, 16, 16, 56 (120 in all) | generator: the frames `F5 00`, `4C C0`, `50 66`, `4E 03`, `9D` + 6 fill bytes, 8 clocks a byte. Twin: S1's MOSI witness | exact | |
| MOSI, if D3 is on CN7 pin 14 (`PB5`) | `F5 00` / `4C C0` / `50 66` / `4E 03` / `9D 00 00 00 00 00 00` | twin (S1 witness, equal to `main`'s), as `3304` B5 | exact | |
| SCK frequency within bytes, each frame (`edges.py frames`: 7 periods per byte, first to eighth rising edge, summed over the frame's bytes) | **equal to C1 (Hz) within 1.5 %** | rule: `silicon::check_spi` (`MBR` 4 on `pll1_q`) and `judge_tree` (`pll1_q` = 8 × F_HSE). The ratio of two exact dividers of one source | ±1 sample per byte span of 84 samples at 2 MHz: worst ±1.2 % on any frame, and C1's drift since B4 | |
| SCK frequency, absolute | **2.000 MHz** at exactly 8.000 MHz; 1.920 to 2.080 MHz inside B-1's gate; at most 2,160,000 Hz at the declaration's fastest | rule (`check_spi`: `achieved_hz` 2,000,000, `achieved_max_hz` 2,160,000 ≤ 4 MHz declared; topology `spi_controllers`) | as above | |
| SCK stalls between bytes (`max_gap_samples`) | **not predicted**: the driver writes one byte to `TXDR` and waits for `RXP` before the next (`board.rs` `spi_frame`), so the TxFIFO empties at every byte and SCK stops there (RM0433 p. 2176: "the master … stops, waiting for additional data"). The gap is core and bus time, which no source here times. This is why the rate above is taken within bytes only | generator (the loop) | — | |
| beside `3304`'s rate at the HSI clock | `3304` B6 predicts 2.000 MHz from the H7's own HSI (1.948 to 2.030 MHz). **Its bench has not run** (no `3304` result is held on `main` or the bus at this writing), so there is no measured rate to repeat. If both run in one session, record both; their ratio is the two RC oscillators' and is **not predicted** | repeat, when held; until then rule (`3304` B6) beside generator (`MBR` 4, unchanged) | — | 3304 SCK ____ |
| SPI mode | `CPOL` 0, `CPHA` 0: SCK is low whenever CS is high | rule, as `3304` B6 | — | |

A frame before the first five can only be `PD14` floating during the reset, as in `3304`. `PA5` floats too then, so that frame may count noise rather than 0 edges. It is not a transfer; note it if seen, and exclude it.

## B6. Restore

`$P write $C b32 0x20000000 0` (read it back as 0), then `$P reset $C`, wait 2 s, and read. The firmware boots again from a system reset (§8.4.2 p. 330), which returns every register written in B4 to its reset value before the firmware runs.

| Read | Address | Mask | Predicted | Source | Bench |
|---|---|---|---|---|---|
| word 0 | `0x20000000` | `0xffffffff` | `0x2100b007` | twin (S1) | |
| `RCC_CFGR` | `0x58024410` | `0xffffffff` | `0x0000001b`: `MCO1` and `MCO1PRE` 0 again | manual (§8.4.2; reset `0x0`, p. 391), then the generator's whole `0x3` | |
| `GPIOA_MODER` | `0x58020000` | `0x00030000` | `0x00030000`: PA8 analog again | manual: reset `0xABFF_FFFF` (p. 541); the firmware never configures PA8 | |
| `RCC_CR` | `0x58024400` | `0x3f0f0001` | `0x030f0001` | as B3: the bring-up ran again | |

The bench writes nothing else: `AHB4ENR`, `AFRH`, `OSPEEDR` and every PWR register are only read. `sentinel-02-bench-hse8.elf` stays flashed. Flash `3304`'s image again before `3304`'s own sheet.

## B7. What this bench cannot witness

- **A crystal's ppm.** The source is the ST-LINK F723's HSI, an RC oscillator; B-1 measures its deviation on the day, not a part's tolerance. A crystal is first witnessed on Sentinel-02 hardware (`4240` D7 (d), `4500`).
- **The `HSERDY` timeout path (`HSE_NOT_READY`).** Showing it needs `PH0` without a clock: SB45 opened, or the ST-LINK's `MCO` cut off by a firmware setting. Both are fenced here (no solder, no ST-LINK change).
- **CSS (`HSE_CSS_FAULT`, `css_hold`).** It needs the clock stopped while running, which needs the same change. The bench shows only that CSS raised no NMI (word 0 stays `B007`).
- **How long `HSERDY` and `PLL1RDY` take.** Nothing toggles a pin before the switch, and `probe-rs` reads after the boot. B2 shows only that each came within its bound: at least 1,299,180 and 9,744 reads, `4240b` A11. It does not test the ruled 20,000 µs against a measured time (`4240b` F3 stays open).
- **Anything at the twin's 100 MIPS.** The boot word's 41.539110 ms is a model convention (`RUL-05`), as in `3304` B6.
- **The analyzer's timebase** is uncited. Every sharp prediction above (C2/C1, SCK/C1) is a ratio in which it cancels.

## B8. Results

Filled at the bench. Send the sheet, the three `c*.bin`, `b1.bin`, `hse8-spi.sr`, the `edges.py` lines and the `probe-rs` output to the architect. **Every disagreement is written beside its prediction, with its size.**

## Appendix: S1's twin records and witness lines, verbatim

From `bench/simulate-verdict.json` (S1), unedited:

| Bus | Device | Step | Command or register | Width | Expected | Observed | Status |
|---|---|---|---|---|---|---|---|
| I2C1 | U7 `0x46` | identity | `0x01` | 1 | `0x50` | `0x50` | ok |
| I2C1 | U7 `0x46` | configure | `0x36 0x40 0x81` | 0 | `0x0` | `0x0` | ok |
| I2C1 | U7 `0x46` | data | `0x1d` | 6 | `0x1940f362` | `0x1940f362` | ok |
| SPI1 (CS PD14) | U5 | identity | `0xf5` | 1 | `0x47` | `0x47` | ok |
| SPI1 (CS PD14) | U5 | configure 1 | `0x4c 0xc0` | 0 | `0x0` | `0x0` | ok |
| SPI1 (CS PD14) | U5 | configure 2 | `0x50 0x66` | 0 | `0x0` | `0x0` | ok |
| SPI1 (CS PD14) | U5 | configure 3 | `0x4e 0x03` | 0 | `0x0` | `0x0` | ok |
| SPI1 (CS PD14) | U5 | data | `0x9d` | 6 | `0x120200000040` | `0x120200000040` | ok |

```
[NOISY] i2c1.u7: read 0x01 -> 0x50
[NOISY] i2c1.u7: write 0x36 <- 0x40 0x81
[NOISY] i2c1.u7: read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62
[NOISY] spi1.spi1mux.u5: bank 0 read 0x75 -> 0x47
[NOISY] spi1.spi1mux.u5: bank 0 write 0x4C <- 0xC0
[NOISY] spi1.spi1mux.u5: bank 0 write 0x50 <- 0x66
[NOISY] spi1.spi1mux.u5: bank 0 write 0x4E <- 0x03
[NOISY] spi1.spi1mux.u5: bank 0 read 0x1D -> 0x12 0x02 0x00 0x00 0x00 0x40
```

Boot word observed `0x2100b007` at 41,539,110 ns virtual; brown-out word observed `0x0406aaf0`; every masked register row equal; no finding; peak RSS 666,332 KiB.
