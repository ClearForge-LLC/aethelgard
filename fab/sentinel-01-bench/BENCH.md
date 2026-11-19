# Sentinel-01 (`sentinel-01-bench`): the bench plan

The rows to run when the boards arrive (AET-WO-3401 §2.4, corrected by AET-WO-3403 after the post-order review, `docs/reviews/2026-11-19-sentinel-01-as-ordered.md`). Each row says which prediction it tests, the instrument, the pass line and what a disagreement adds to the record. A pass line is a number from a cited prediction, and a reading outside it fails the row. A failed row is a finding, never a retry.

The board is `examples/sentinel-01.json`, and its package is `package/` beside this file. The ELF is `package/sentinel-01-bench.elf` (sha256 `008a2a59…`, the full hash in `package/MANIFEST.json`). Every row here runs that ELF. The BMP581 on J4 runs a different ELF, from a bench specification, and has its own sheet: `BENCH-EXT-BMP581.md`. That specification describes a board that was never made (the breakout as an on-board part), so its `verify` verdict says nothing about J4's copper with a breakout plugged in.

**What each prediction rests on** (`RUL-20`'s terms, as `AET-WO-3304.bench.md` labels them). Every pass line names one:
- **twin:** the twin's own run of this ELF (Renode's models and the boot judge, or ngspice through the campaign).
- **rule:** an independent rule in `aethelgard-ir` that shares no code with the generator, or a datasheet limit cited at source.
- **generator:** the generated code's own word. This is consistency, not judgement, and only the bench judges it.
- **repeat:** `3302` already measured this on the Nucleo with the same generated code. The row checks for drift; it is not a fresh prediction.
- **not predicted:** said plainly, with the reason. The row records it.

> **EXT is 3.3 V on pin 1, not DS-009 5 V. Plug in only 3.3 V-tolerant I2C devices.** (J4, the EXT port: DS-009 I2C's pinout, 1 VDD_3V3, 2 SCL, 3 SDA, 4 GND, with `standard: null`; ruled on PR #190, Q3.)

> **EXT's load budget.**
> - `extract`'s bound for EXT assumes at most **20 pF per line off-board**: J4's declared allowance, for the mated contact, a JST lead of up to 100 mm and one remote pin.
> - Anything plugged into J4 that adds more is outside the bound the gates judged.
> - The copper itself leaves about 71 pF per line before the declared 100 pF: a declared 70 pF off-board passes `extract`, and 85 pF fails it [measured on this board, 2026-11-17].

> **U2 is not fitted on these boards.** The BME280 is out of stock everywhere, and its replacement, the BMP581, is an LGA-10 of 2.0 × 2.0 mm that does not land on U2's LGA-8 2.5 × 2.5 mm pads (the review's U2; Scotty chose the BMP581 on #237). What the ordered ELF does with U2 empty, row by row, is in "U2 empty" below. A BMP581 breakout can stand in on J4 instead (`BENCH-EXT-BMP581.md`).

## Before power

Every step here runs with nothing powered. Each catches a fault that a later row could not tell apart from a pass.

1. **Cables** (review F9–F11).
   - **J1, J3 and J4 are identical JST GH 4-pin headers** in one row along an edge, with J2 (the 6-pin SH) between J1 and J3: left to right, J1, J2, J3, J4. Label each cable for its header before the first plug-in.
   - **J1 and J3 are never swapped.** J3 pin 1 is `VDD_3V3`, wired straight to every VDD pin with no regulator, no reverse-polarity element and no clamp. A supply moved to J1 drives PA9 (CONSOLE_TX) of an unpowered MCU and leaves the board unpowered (the review's F9). Power into J4 is benign: it shares J3's pin 1 and pin 4.
   - **J2 has the project's own pinout:** 1 VDD_3V3, 2 BOOT0, 3 NRST, 4 SWDIO, 5 SWCLK, 6 GND. It is **not** the Dronecode/Pixhawk JST-SH 6 debug pinout, which carries UART on pins 2 and 3: such a cable puts its UART TX on NRST, so every byte it sends resets the board, and its UART RX on BOOT0, where an adapter's pull-up can fight R8. Wire the SWD cable to J2's own pinout, pin by pin.
2. **SW1 (PTS810), before fitting it** (review F11). With the DMM on continuity: pads 1–1 continuous, pads 2–2 continuous, 1–2 open until pressed, closed while pressed. KiCad's pairing was not read at the maker's source. If it were wrong, NRST would be tied to GND and the board would never leave reset. SW1 is optional: reset also works through J2 pin 3 and by power cycling. Source: **not predicted** (C&K's sheet was unreadable; the review's F11).
3. **Resistances, read with the DMM** (review F4, F5).
   - Read each pair in both lead polarities and record both; the pass line is on the **higher** reading. A parallel junction in an unpowered part can only lower a reading.
   - Expect a slow settle on TP4: it charges the board's decoupling.

   | Pads | Part | Pass | Fault it catches | Source |
   |---|---|---|---|---|
   | TP14 (I2C_SDA) to TP4 | R1/R2, 2.2 kΩ 1 % | 2.13–2.27 kΩ | a 10 kΩ fitted on SENSORS (B6 would also catch it) | rule: `ORDER.md` §2's part, ±1 % plus a DMM's ±2 % |
   | TP13 (I2C_SCL) to TP4 | R1/R2, 2.2 kΩ | 2.13–2.27 kΩ | as above | as above |
   | TP12 (EXT_SDA) to TP4 | R3/R4, 10 kΩ 1 % | 9.7–10.3 kΩ | **a 2.2 kΩ fitted on EXT**, which passes B6 and removes the weak-bus point B7 needs | as above |
   | TP11 (EXT_SCL) to TP4 | R3/R4, 10 kΩ | 9.7–10.3 kΩ | as above | as above |
   | TP2 (BOOT0) to TP1 (GND) | R8, 10 kΩ 1 % | 9.7–10.3 kΩ | **R8 missing or open**, which B8.1's voltage line cannot see (a DMM's 10 MΩ pulls a floating BOOT0 to 0 V) | as above; R8 is `1312`'s judged pull-down |
   | TP4 to TP1 | — | not a short (above 100 Ω after the settle) | a solder bridge across the rail | not predicted: a short is the fault, not a figure |

   Write down the measured R of each line: B6 uses it in place of the nominal.
4. **NRST, the copper half** (review F3). Continuity under 1 Ω from the SWD cable's NRST wire, at the probe end, to TP3. That holds the cable, J2 pin 3 and the copper together. B1 holds the powered half. Source: **rule** (the netlist: J2 pin 3 and TP3 are both `NRST`).
5. **Probe capacitance, `C_p`.** Read `C_p` from your probe's own data (a ×10 passive probe prints its input capacitance, typically about 10–15 pF) and write it down before the first rise-time row. A probe clipped to a test pad is an off-board load, outside `extract`'s bound (ruled on #191, Q2). So every rise-time pass line below adds `C_p` to the line.
6. **Supply.** J3 is the power header: pin 1 is `VDD_3V3`, the MCU's rail, which the board takes from J3 (`mcu.supply_source`); pin 4 is GND. Use a current-limited bench supply, **set and checked at 3.30 V before it is connected, never above 3.6 V** (DS12117 Rev 10, VDD absolute maximum 4.0 V), limited to 100 mA.

**Test points** (bare 1 mm pads):

| Pad | Net | Pad | Net |
|---|---|---|---|
| TP1 | GND | TP8 | SPI_MISO |
| TP2 | BOOT0 | TP9 | SPI_MOSI |
| TP3 | NRST | TP10 | SPI_SCK |
| TP4 | VDD_3V3 | TP11 | EXT_SCL |
| TP5 | CONSOLE_RX | TP12 | EXT_SDA |
| TP6 | CONSOLE_TX | TP13 | I2C_SCL |
| TP7 | FLASH_CS | TP14 | I2C_SDA |

**One GND pad, TP1, as ruled** (`AET-WO-3402` Q2 (c), PR #203). The specification now admits several GND test points (`AET-WO-3402` §1.1), but the placer puts a second one where the specification cannot steer it, 25.8–31.5 mm from the I2C pads [measured]. So the other ground points are the GND pins of J1–J4 (pin 4 of J1, J3 and J4; pin 6 of J2), and B6 names the nearest.

## What the ordered ELF puts where (review F1)

The ELF reports in two places, and the console carries neither record.

- **The console** (USART1, J1 pin 1 = CONSOLE_TX, 115200 8N1) sends exactly **12 bytes** once per boot, `43 4F 4E 53 4F 4C 45 00 FE 7F 80 FF` (`CONSOLE`, then the tail `00 FE 7F 80 FF`; `boards/sentinel-01-bench/firmware/src/main.rs` L172–177 and L56). It sends them **before** any I2C or SPI read. It then echoes at most 8 received bytes, each within a bounded wait (L204–211), and falls silent. Source: **twin** (the boot judge's witness of this pattern, `simulate`'s `serial[0].twin.pattern`).
  - Open the terminal, in hex, **before** releasing reset: the bytes come within milliseconds of it.
  - The echo is not a pass line. Its window closes within milliseconds of the boot word (at most 20,000 polls of `ISR` per byte, L60).
- **The records** are the RAM array `AETHELGARD_BOOT` at `0x20000000` (`link.x`: the first word of DTCM; `main.rs` L30), six words, read over SWD:

  | Word | Address | What | Expected, U2 empty | Source |
  |---|---|---|---|---|
  | 0 | `0x20000000` | boot word, written last (L203) | `0x2100B007` | twin |
  | 1 | `0x20000004` | `FLASH_OPTSR_CUR` as read at boot (L148–150) | bits 3:2 = `11` after B3's procedure; `00` before it | B3 |
  | 2 | `0x20000008` | record 0 status: I2C1, address `0x76`, register `0xD0` (U2's identity; L179–190) | `0x22020001` (TXIS) | generator and RM0433, as "U2 empty" traces; **repeat in kind** (`3302` read TXIS with no device, same driver) |
  | 3 | `0x2000000C` | record 0 value | `0x00000000` | as above |
  | 4 | `0x20000010` | record 1 status: SPI1 `0x9F` to U4 (L191–201) | `0x22020000` (OK) | twin |
  | 5 | `0x20000014` | record 1 value | `0x00EF4017` (`EF 40 17`) | twin; Winbond W25Q64JV rev J §8.1, read at source by `3401` |

  With U2 fitted (a BME280), record 0 would be `0x22020000` / `0x00000060`, the twin's value. No board of this order has it.

**The commands.** `probe-rs` 0.32.0 is the bench's (`AET-WO-3304.bench.md` §0). Its exact syntax was not run on Furnace (no `probe-rs` here): check `probe-rs read --help` and `probe-rs write --help` before the first row, as `3302` and `4240` said.

```
P=~/tools/probe-rs/bin/probe-rs
$P read --chip STM32H753ZITx b32 0x20000000 6
```

**Clear the records before the boot being judged.** The reset handler zeroes `.bss` only, and `.aethelgard_boot` is `NOLOAD` (`link.x`), so a reset leaves the last boot's words in RAM (`3302` saw residue there). A boot that dies early would then still show `0x2100B007` and the old records. So either power-cycle the board first (J3's supply off and on), or write zeros and read them back as zeros, then reset:

```
$P write --chip STM32H753ZITx b32 0x20000000 0 0 0 0 0 0
$P read  --chip STM32H753ZITx b32 0x20000000 6
$P reset --chip STM32H753ZITx
```

Let the firmware run for at least 1 s after reset before reading. A read attaches over the AHB access port; whether this `probe-rs` halts the core to read is not known here. The core is parked in its final loop by then (L213), so a halt changes no record.

## U2 empty: what the ordered ELF does

**The code path** (the ordered firmware, `boards/sentinel-01-bench/`):
1. The console's 12 bytes go out first (`firmware/src/main.rs` L172–177).
2. `i2c1.transfer(0x76, [0xD0], 1 byte)` (`main.rs` L179). The write phase writes `CR2` with `START`, one byte, `AUTOEND` 0 (`src/board.rs` L587), then waits for `TXIS` at most `I2C_WAIT_SPINS` = 20,000 reads of `ISR` (L204, L589–590).
3. No device answers `0x76`, so the address is NACKed. RM0433 Rev 9 §47.4.9 (p. 1975, read at source): "When a NACK is received, the TXIS flag is not set and a STOP condition is automatically sent. The NACKF flag of the I2C_ISR register is set."
4. The wait expires: `I2cError::Txis` (L590). No `TC`, read or `STOPF` wait follows (L595–611). `ICR` is written with `STOPCF | NACKCF` (L613), so I2C1 is left idle and clear.
5. `main.rs` L184 records `(0x22020001, 0)` as record 0 (L190). The SPI read, the boot word and the echo follow unchanged (L191–211).
6. Nothing else in the firmware addresses `0x76` or I2C1.

`3302` measured this path on silicon (rev V Nucleo, `AET-WO-3302.bench.md` L242–246): `0xEC`, NACK, STOP, status `0x22020001`, no lockup. That was a sibling ELF of the same generated driver, not this ELF.

**What the twin cannot say.** The twin runs this ELF with U2 answering `0x60`. It renders an absent I2C device only as the whole bus failing (`AET-WO-3304.FEEDBACK.md` §1.1), never one device absent beside another. So no twin run covers "U2 absent, U3 present", and every U2-empty line below is the code trace above, labelled **generator**, with `3302`'s **repeat** where it applies.

| Row | With U2 empty |
|---|---|
| B1 | Unchanged: the console bytes go out before the I2C read |
| B2 | Unchanged: no row depends on the transfer (`I2C1.CR2`'s mask leaves out the fields a transfer sets) |
| B3, B5, B8 | Unchanged: nothing in them touches I2C1 |
| B4 | Unchanged: record 1 follows record 0 whatever it holds |
| B6 | SENSORS' bounds still hold. They include U2's pads and pin capacitance, so with U2 off they are looser by up to U2's pins |
| B7 | Unchanged: EXT, not SENSORS |
| B9 | Record 0 is TXIS, and the scope shows `0xEC`, NACK, STOP. U3 (the TMP117) is the positive witness of I2C1 |

**What is lost:** only the silicon witness of a full I2C read on SENSORS by the firmware (record 0 = `0x60`). B9's TMP117 probe witnesses PB6/PB7 at AF4 and the SENSORS copper without it.

## B1. Programming by SWD, with connect-under-reset

- **Tests:** `3301`'s programming path on our copper, and NRST on J2 pin 3 (`3400` probe `p-nrst`, now built).
- **Instrument:** an SWD probe on J2, wired to J2's own pinout (Before power, step 1); probe-rs or OpenOCD in connect-under-reset mode; a scope on TP3 (NRST); a 3.3 V USB-UART on J1: its RX to J1 pin 1 (the board's TX), its TX to J1 pin 2 (the board's RX; it idles high, so PA10, which has no pull, does not float into the echo), its GND to J1 pin 4.
- **Pass:**
  1. **NRST reaches TP3** (review F3). Hold reset from the probe's own reset line and keep it held: with OpenOCD, `adapter assert srst`, then `adapter deassert srst` after the reading (syntax not run on Furnace: check the installed OpenOCD's `help adapter`), or the probe maker's tool. While it is held, TP3 reads **at most 0.3 V** on the DMM (the review's figure, well under the 0.3·VDD = 0.99 V input-low level of Table 146), and the console stays silent across a power cycle of J3. Source: **rule** (DS12117 Table 151: NRST has a permanent 30–50 kΩ pull-up, so an open NRST on J2 leaves TP3 at VDD, and fails this line).
     - A scope on a connect-under-reset is not enough: the MCU pulls NRST low itself on every internal reset (`AET-WO-3403.FEEDBACK.md` §2.2).
     - With Before power step 4, this is the row's only witness of J2 pin 3: the probe attaches through SWD whether or not NRST arrives, so items 2 and 3 pass with NRST open.
  2. `package/sentinel-01-bench.elf` flashes and verifies (`download --verify` if this `probe-rs` offers it). Its sha256 is checked against `MANIFEST.json` before the download. Source: **not predicted** (no model of the probe).
  3. After reset, the console sends exactly the 12 bytes `43 4F 4E 53 4F 4C 45 00 FE 7F 80 FF` at 115200 8N1. Source: **twin**. Count from the first `43`: any other byte among the twelve, a missing byte or a different count fails. A byte or framing error before the first `43` is recorded, not judged: PA9 is switched to AF7 (`main.rs` L163) before USART1 is enabled (L171), and the twin does not model the pin's level in between. Send nothing from the terminal during the reading: the firmware echoes what it receives.
- **On a disagreement:** a row in `architecture.md` §2.1 (corrections) naming the step that failed. Item 1 failing implicates J2 pin 3's net, which parity judged.

## B2. The register image, read back

- **Tests:** first silicon's rows (`AET-WO-3302.bench.md` B2), now on our board, and the supply state the ELF runs in (`4240`'s F-VOS).
- **Instrument:** the SWD probe, reading at least 1 s after boot.
- **Pass:** each row of the table below reads back, under its mask, with the expected word. Compare **only the masked bits**.

  **Read B2 right after a reset, before B6, B7 and B9:** B6 rewrites `GPIOB.MODER`, and B7's and B9's probes write `I2C1.CR2` and `I2C2.CR2`.

  Where the rows come from: the twin's recorded registers (`simulate` of this ELF, with their masks), then the fields the twin does not store, each with the written value. Every source is the class the package's own `verify` ledger gives the field (`RUL-20`; `FAB-NOTES.md`, "Acknowledged ledger rows"):
  - **twin:** the model reacts to the field (`observed`);
  - **twin (Renode's reset value):** the firmware never writes the register, so the expected word is the model's reset value. A difference on silicon is with Renode, not with the firmware;
  - **generator:** `same-source`: the twin reads back only what the firmware wrote. A read-back is the only judge;
  - **rule:** `independent-rule`, `silicon.rs`;
  - **not predicted:** `ignored-by-model` or `silicon-only`: only the bench judges it.
  - `TIMINGR`'s exact word is the generator's (the rule holds its fields only by inequalities), so those rows can fail only on a read-back difference (`AET-WO-3304.FEEDBACK.md` A5).
  - `I2C1.CR2`'s mask leaves out `AUTOEND`, `START`, the address and the count: they hold the last transfer's, which differs with U2 fitted or empty. No row below depends on U2.

  | Register | Address | Mask | Expected | Source |
  |---|---|---|---|---|
  | USART1.CR1 | `0x40011000` | `0x040087ed` | `0x0000000d` | twin; generator |
  | USART1.CR2 | `0x40011004` | `0x00003000` | `0x00000000` | generator |
  | USART1.CR3 | `0x40011008` | `0x000000c0` | `0x00000000` | twin (Renode's reset value: the firmware never writes it) |
  | USART1.BRR | `0x4001100c` | `0x0000ffff` | `0x0000022c` | rule |
  | GPIOA.MODER | `0x58020000` | `0x003cfc03` | `0x0028a801` | generator |
  | GPIOA.AFRL | `0x58020020` | `0xfff00000` | `0x55500000` | generator |
  | GPIOA.AFRH | `0x58020024` | `0x00000ff0` | `0x00000770` | generator |
  | GPIOB.MODER | `0x58020400` | `0x00f0f000` | `0x00a0a000` | generator |
  | GPIOB.AFRL | `0x58020420` | `0xff000000` | `0x44000000` | generator |
  | GPIOB.AFRH | `0x58020424` | `0x0000ff00` | `0x00004400` | generator |
  | RCC.AHB3ENR | `0x580244d4` | `0xffffffff` | `0x00000000` | twin (Renode's reset value: the firmware never writes it) |
  | RCC.AHB1ENR | `0x580244d8` | `0xffffffff` | `0x00000000` | twin (Renode's reset value: the firmware never writes it) |
  | RCC.AHB2ENR | `0x580244dc` | `0xffffffff` | `0x00000000` | twin (Renode's reset value: the firmware never writes it) |
  | RCC.AHB4ENR | `0x580244e0` | `0xffffffff` | `0x00000003` | twin |
  | RCC.APB3ENR | `0x580244e4` | `0xffffffff` | `0x00000000` | twin (Renode's reset value: the firmware never writes it) |
  | RCC.APB1LENR | `0x580244e8` | `0xffffffff` | `0x00600000` | twin; generator |
  | RCC.APB1HENR | `0x580244ec` | `0xffffffff` | `0x00000000` | twin (Renode's reset value: the firmware never writes it) |
  | RCC.APB2ENR | `0x580244f0` | `0xffffffff` | `0x00001010` | twin |
  | RCC.APB4ENR | `0x580244f4` | `0xffffffff` | `0x00010000` | twin (Renode's reset value: the firmware never writes it) |
  | SysTick.CSR | `0xe000e010` | `0x00010003` | `0x00000000` | twin (Renode's reset value: the firmware never writes it) |
  | NVIC.ISER0 | `0xe000e100` | `0xffffffff` | `0x00000000` | twin (Renode's reset value: the firmware never writes it) |
  | NVIC.ISER1 | `0xe000e104` | `0xffffffff` | `0x00000000` | twin (Renode's reset value: the firmware never writes it) |
  | NVIC.ISER2 | `0xe000e108` | `0xffffffff` | `0x00000000` | twin (Renode's reset value: the firmware never writes it) |
  | NVIC.ISER3 | `0xe000e10c` | `0xffffffff` | `0x00000000` | twin (Renode's reset value: the firmware never writes it) |
  | NVIC.ISER4 | `0xe000e110` | `0xffffffff` | `0x00000000` | twin (Renode's reset value: the firmware never writes it) |
  | SCB.CPACR | `0xe000ed88` | `0x00f00000` | `0x00f00000` | generator |
  | RCC.D2CCIP1R | `0x58024450` | `0xb13771c7` | `0x00004000` | rule |
  | GPIOA.OTYPER | `0x58020004` | `0x0000ffff` | `0x00000000` | not predicted (no model reacts) |
  | GPIOA.PUPDR | `0x5802000c` | `0xffffffff` | `0x64000000` | generator |
  | GPIOB.OTYPER | `0x58020404` | `0x0000ffff` | `0x00000cc0` | not predicted (no model reacts) |
  | GPIOB.PUPDR | `0x5802040c` | `0xffffffff` | `0x00000100` | generator |
  | GPIOA.OSPEEDR | `0x58020008` | `0x003cfc03` | `0x00004400` | generator |
  | GPIOB.OSPEEDR | `0x58020408` | `0x00f0f000` | `0x00000000` | generator |
  | I2C1.CR1 | `0x40005400` | `0x00ffdfff` | `0x00000001` | generator; rule |
  | I2C1.TIMINGR | `0x40005410` | `0xf0ffffff` | `0x10c7123c` | generator, read back (the rule holds the fields by inequalities, not the word) |
  | I2C1.CR2 | `0x40005404` | `0x0500d800` | `0x00000000` | generator |
  | I2C2.CR1 | `0x40005800` | `0x00ffdfff` | `0x00000001` | generator; rule |
  | I2C2.TIMINGR | `0x40005810` | `0xf0ffffff` | `0x50d32a40` | generator, read back (the rule holds the fields by inequalities, not the word) |
  | I2C2.CR2 | `0x40005804` | `0x0500d800` | `0x00000000` | generator |
  | SPI1.CR1 | `0x40013000` | `0x0001ff01` | `0x00001000` | rule |
  | SPI1.CFG1 | `0x40013008` | `0x705fdfff` | `0x10070007` | rule |
  | SPI1.CFG2 | `0x4001300c` | `0xf7fe80ff` | `0x84400000` | rule |
  | RCC.CR | `0x58024400` | `0x00008018` | `0x00008000` | twin; not predicted (no model reacts) |
  | RCC.CFGR | `0x58024410` | `0x00008038` | `0x00000000` | twin |
  | RCC.D1CFGR | `0x58024418` | `0x00000f7f` | `0x00000000` | twin |
  | RCC.D2CFGR | `0x5802441c` | `0x00000770` | `0x00000000` | twin |
  | RCC.D1CCIPR | `0x5802444c` | `0x30000000` | `0x00000000` | not predicted (no model reacts) |
  | RCC.D2CCIP2R | `0x58024454` | `0x0000303f` | `0x00000000` | not predicted (no model reacts) |

- **The supply state, `4240`'s bench row B-2** (review F14). The ordered ELF runs at VOS3 with every APB clock at 64 MHz, above DS12117's 50 MHz for VOS3, and never writes a PWR register. `PWR_CR3` is reset only by a power-on reset, so **power-cycle the board first** (remove J3's supply), then attach under reset and read, before and again after the boot:

  ```
  $P read --chip STM32H753ZITx b32 0x5802480C 1   # PWR_CR3: SCUEN bit 2, LDOEN bit 1, BYPASS bit 0
  $P read --chip STM32H753ZITx b32 0x58024804 1   # PWR_CSR1: ACTVOSRDY bit 13
  $P read --chip STM32H753ZITx b32 0x58024818 1   # PWR_D3CR: VOS bits 15:14, VOSRDY bit 13
  ```

  Source: **not predicted.** The twin's PWR is a stub (it reads `CR3` = `0x04000000` and `VOS` = 00, `4240` FEEDBACK F-VOS). `4240` read RM0433's reset values (`CR3` `0x46` on rev V, `0x06` on rev Y; `D3CR` VOS = Scale 3), and these reads say whether `ACTVOSRDY` is 1 without the firmware writing `CR3`. They are recorded beside B2, not judged: the B2 and B4 readings come from an ELF outside DS12117's operating conditions, and are read again with `4240v`'s ELF once it lands.
- **On a disagreement:** a corrections row per register field, as `3302` B2 did.

## B3. `BOR_LEV`

- **Tests:** `bor_level: 3` and its programming by `package/FAB-NOTES.md`'s procedure (`4230`).
- **Instrument:** SWD: `$P read --chip STM32H753ZITx b32 0x5200201C 1` (`FLASH.OPTSR_CUR`, as `3302` read it), and record word 1 (`0x20000004`).
- **Pass:** bits 3:2 (`BOR_LEV`) read `11` after the procedure, in both reads. They read `00` before it, the factory default.
- **Source:** **not predicted** by the twin, which presents level 3 itself (`acknowledge_unwitnessed`, `FLASH.OPTSR_CUR`, silicon-only). The `00` before is a **repeat** (`3302` B7). Fails if the procedure does not take.
- **On a disagreement:** a corrections row against `4230`'s factory step.

## B4. SPI1 with the `MODF` fix, and the flash's identity

- **Tests:** northstar §2's first scar, fixed by `4230` (`SSM`/`SSI` before `SPE`), and SPI1 on PA5–PA7 at AF5 with PA0 as chip select, on our copper.
- **Instrument:** SWD; a scope on TP10 (SPI_SCK) and TP7 (FLASH_CS), single-shot on TP7's falling edge, across a reset. TP9 (SPI_MOSI) and TP8 (SPI_MISO) are there too.
- **Pass:**
  - **Record 1** (above): `0x20000010` = `0x22020000` and `0x20000014` = `0x00EF4017`. Source: **twin**; the value is Winbond's (W25Q64JV rev J §8.1, Read JEDEC ID 9Fh: `EFh`, then `4017h` for the IQ ordering codes; `parts.json`'s row, sha256 `452d906c…`). The value can come only from the flash over PA5–PA7 and PA0: with a wrong AF, an open line or a dead U4, the controller still completes the transfer (status OK) but the value is whatever floating MISO reads, never `EF4017` [inferred: `3304`'s floating-MISO note].
  - **Word 0** = `0x2100B007` (written after record 1; L203). Source: **twin**.
  - `SPI1.SR.MODF` (`0x40013014`, bit 9) = 0. Source: **not predicted** by the twin (Renode models no `MODF`): the scar's row.
  - `SPI1.CFG2.MASTER` (`0x4001300C`, bit 22) = 1. Source: **rule** (`silicon::spi_words`, held by `check_spi`).
  - **SCK on TP10:** 32 clocks while TP7 is low (the command and three bytes), at **15.59–16.24 MHz** (16.000 MHz at HSI's 64 MHz, `MBR` 1, /4). Source: **rule** (the solved `MBR`, judged by `silicon::check_spi`: `achieved_max_hz` 16,239,744; the low end is HSI's −2.6 %). This replaces "`SPE` is set during the transfer", which cannot be read after boot.
- **On a disagreement:** `MODF` = 1 again would reopen the first scar. It is a northstar §2 row and a `4230` defect, not a board fault.

## B5. The BOR sweep

- **Tests:**
  - `bor_level: 3`, `2301`'s droop model;
  - `1208`'s MCU supply range, at DS12117 Rev 10 **Table 115** (p. 212, rev V; the rev Y half's Table 16 prints the same values): VBOR3 **falling 2.54 / 2.61 / 2.68 V** (min/typ/max), **rising 2.63 / 2.70 / 2.78 V**.
- **Instrument:** the bench supply on J3 pin 1, a DMM on TP4 (VDD_3V3) and a scope on TP3 (NRST).
- **Method:** with `BOR_LEV` = 3 (B3), lower the supply in 10 mV steps, held for 1 s each, from 3.30 V until NRST asserts. Record that VDD. Raise it again until NRST releases. Repeat three times. Never raise it above 3.6 V.
- **Pass:** every falling trip is in **2.54–2.68 V** and every rising release in **2.63–2.78 V**, measured at TP4.
- **Source:** **rule** (the cited Table 115 rows, the values `SupplyFloorBelowBorRising` judges). A board whose `BOR_LEV` did not take trips near the power-down reset instead, far below 2.54 V, and fails.
- **On a disagreement:** a corrections row against `power-parts.json`'s cited Table 115 rows. Those are the values `SupplyFloorBelowBorRising` judges, so a disagreement reaches every board.

## B6. Rise time at both operating points, against `extract`'s bound

- **Tests:**
  - `RUL-19` (1): the campaign's rise time is computed from declared values, and `extract` bounds the routed copper;
  - roadmap §3: Sentinel-01 is ordered to falsify the extractor;
  - the probe is added as an off-board load (ruled on #191, Q2).
- **Instrument:** a scope with the probe of known `C_p` on each pad. Its ground goes to the nearest ground point (ruled on PR #203, Q2 (c)), distances measured pad centre to pad centre on the board's emitted layout:
  - **SENSORS** (TP13 I2C_SCL, TP14 I2C_SDA): **TP1**, at 20.5 mm and 21.2 mm.
  - **EXT** (TP11 EXT_SCL, TP12 EXT_SDA): **J4 pin 4**, the EXT port's own GND, at 14.0 mm and 8.5 mm. J3 pin 4 is nearer EXT_SCL, at 8.7 mm.
  - So the probe's ground is a **lead of at least that length**, typically the probe's 5–15 cm ground clip, not a spring. Record its length beside `C_p`.
  - **Why a few centimetres of lead is enough here** [inferred]: the lead's inductance is about 1 nH/mm, so 50–150 nH, which with `C_p` of about 10 pF rings at roughly 130–230 MHz. The rises B6 times last 100 ns to 1 µs (table below), so their spectrum lies far below that resonance. The ring rides on the fast falling edge (`3302` B3 saw a 40–50 ns ring on it), not on the 30–70 % rise this row reads.
- **Method:** as `3302` B3–B5, the debugger drives the line low and releases it, so each edge is the pull-up's RC with no firmware. **J4 empty.** The pins are in alternate-function mode after boot, where `BSRR` does nothing, so first make the pin a GPIO output; `OTYPER` already makes PB6, PB7, PB10 and PB11 open-drain (B2), so writing its `BSRR` set bit releases the line rather than driving it high.
  1. Read `GPIOB.MODER` (`0x58020400`). Write it back with the pin's two bits set to `01`: bits 13:12 for PB6 (I2C_SCL), 15:14 for PB7 (I2C_SDA), 21:20 for PB10 (EXT_SCL), 23:22 for PB11 (EXT_SDA); leave every other bit as read (PB3/PB4 and the debug pins).
  2. `GPIOB.BSRR` (`0x58020418`): write `1 << (n + 16)` to drive pin n low, then `1 << n` to release it. Capture the rise on release.
  3. **Reset the board after B6.** `MODER` keeps the change, and B7 and B9 need the pins back at AF4.
- **Prediction:** `t(30–70 %) = 0.8473 · R · C`.
  - **R is the line's pull-up as measured in Before power, step 3.** That step is what fails a wrong part (review F4): this row's bound is an upper bound only, so a 2.2 kΩ on EXT would pass it.
  - C is `extract`'s bound for the line plus `C_p`.
  - The bounds are from this package's board at two layers, `extract` on `examples/sentinel-01.json`, measured 2026-11-17. EXT's bounds include J4's declared 20 pF off-board allowance, so with J4 empty the line is lower.
- **Pass:** the measured 30–70 % rise is at or below the upper bound, and that figure ignores ringing (`3302` B3 saw a release delay and ringing at 2K). Source: **rule** (`extract`'s bound, an independent reading of the copper).

  | Line (pad) | R (nominal) | `extract` bound | Upper bound on the rise (R measured, C_p in pF) | At 2.2 / 10 kΩ and C_p = 10 pF |
  |---|---|---|---|---|
  | I2C_SDA (TP14) | 2.2 kΩ | 58.6 pF | 0.8473 × R × (58.6 + C_p) | 128 ns |
  | I2C_SCL (TP13) | 2.2 kΩ | 53.4 pF | 0.8473 × R × (53.4 + C_p) | 118 ns |
  | EXT_SDA (TP12) | 10 kΩ | 48.6 pF | 0.8473 × R × (48.6 + C_p) | 497 ns |
  | EXT_SCL (TP11) | 10 kΩ | 47.5 pF | 0.8473 × R × (47.5 + C_p) | 487 ns |

  The campaign's own prediction, from the declared 100 pF, is 186 ns at 2.2 kΩ and 847 ns at 10 kΩ, under the 300 ns and 1000 ns ceilings. A board that passes this row is faster than the campaign assumed.
- **Record on every line, pass or fail** (review F4): the implied C, `t / (0.8473 · R) − C_p`, beside `extract`'s bound. That measures the extractor's margin instead of only bounding it. The implied C is the bench restated, not a prediction.
- **On a disagreement:** a rise above the upper bound means the copper plus probe exceeds `extract`'s bound. **That falsifies the extractor** and goes in `architecture.md` §2.1 with the line, `C_p`, R and the implied C.

## B7. The H7's I2C clock-high threshold, on the weak bus

- **Tests:** northstar §2's second divergence. The H7 counts `SCLH` from its own pin's input threshold, not from 0.7·VDD = 2.31 V. (`3302` B5 saw SCL top out near 1.7–1.8 V at 100 kΩ on the Nucleo; that figure is a sawtooth peak, not a threshold measurement.)
- **Why the old method could not do it** (review F2). RM0433 Rev 9 §47.4.9 (p. 1971, read at source): the controller "detects its own SCL high level after a tSYNC2 delay depending on the SCL rising edge, SCL input noise filters (analog and digital), and SCL synchronization to the I2CxCLK clock", and counts `SCLH` from that detection. So one edge times threshold, delay and the kernel clock's error together. At this edge's slope (about 3–4 mV/ns near 1.7 V) a 100 ns delay reads as 0.3–0.4 V of threshold, and HSI's −2.6/+1.5 % on 4.031 µs as ±0.2–0.3 V. Both exceed the old ±0.2 V band.
- **The method: two edge rates, solved together.** A threshold is fixed in volts; the delay and the count are fixed in time. Two edges of different slope separate them.
  - **Instrument:** the scope on TP11 (EXT_SCL, R3 10 kΩ), J4 empty, at ≥ 500 MSa/s, probe ground to J4 pin 4; the DMM on TP4. One 47 pF C0G capacitor (any value 33–100 pF works; C0G so its value does not move with voltage), to be fitted from TP11 to J3 pin 4 or J4 pin 4 for the second edge and removed after.
  - **The bus traffic, over SWD** (the ordered ELF starts no transfer on EXT; I2C2 is enabled and idle, `TIMINGR` `0x50D32A40`, B2):
    ```
    $P write --chip STM32H753ZITx b32 0x4000581C 0x00000030   # I2C2.ICR: clear STOPF, NACKF
    $P write --chip STM32H753ZITx b32 0x40005804 0x020020AA   # I2C2.CR2: address 0x55, write, NBYTES 0, AUTOEND, START
    ```
    No device answers `0x55`: the controller clocks nine SCL pulses, gets a NACK and sends a STOP by itself (RM0433 p. 1975). Nothing stretches SCL, so no wait depends on the debugger. Repeat the pair at least 10 times per edge rate.
  - **Clock words** (`TIMINGR` `0x50D32A40`: `PRESC` 5, `SCLH` 42, `SCLL` 64): one prescaled tick is 6 kernel cycles; the high count is 43 ticks = 258 cycles (4.031 µs at 64 MHz) and the low count 65 ticks = 390 cycles (6.094 µs).
  - **Steps.**
    1. **Vertical reference.** Read VDD at TP4 on the DMM. On the scope, take SCL's idle-high level (before the START) as VDD and the probe tip on TP1 as 0 V, and express every voltage below as a fraction of that span times the DMM's VDD. The scope's gain error cancels.
    2. **Edge A, no capacitor.** Capture pulses 2–8 of each address phase. For each pulse, from its falling edge (the 50 % crossing, which is sharp: the controller drives it) read: the period `P` to the next falling edge, and the time `a(V)` to the rising edge's crossing of V, for V from 1.0 to 2.4 V in 50 mV steps. Average over every pulse.
    3. **Edge B,** with the capacitor fitted. The same readings.
    4. **Edge A again,** capacitor removed. If its `P` differs from step 2's by more than 5 ns, the kernel clock drifted between captures: record it and add it to δ below.
    5. **The threshold.** In each period, `P = a(V_th) + t_SYNC2 + SCLH`, and the delay and the count are the same at both edge rates. So **`V_th` is the V at which `a_B(V) − a_A(V) = P_B − P_A`**. Interpolate between the 50 mV steps.
       - **The assumption** [inferred]: the delay after the threshold crossing does not depend on the slope. RM0433 lists "the SCL rising slope" among `tSYNC2`'s terms; this method takes that term to be the rise to the threshold, which it measures as `a(V)`. So the `t_SYNC2` of step 6 is RM0433's `tSYNC2` less that rise. If the analog filter's delay itself changed with the slope, `V_th` would move by about 3.5 mV per ns of change; that is not bounded here.
    6. **The delay and the clock.** The SCL low phase is `L = SCLL + t_SYNC1` (its start is the controller's own fast edge), so `T_k ≈ L / 390` calibrates the kernel clock, high by `t_SYNC1 / 390`. Cross-check it without any synchronisation delay on the console: the `00` byte keeps TP6 low for exactly 9 bits of `BRR` = 556 cycles, so **`T_k = t_low(00) / 5004`** (78.19 µs at 64 MHz; USART1 and I2C2 both run from HSI through dividers of 1, the solved topology). Then `t_SYNC1 = L − 390 · T_k` and `t_SYNC2 = P_A − a_A(V_th) − 258 · T_k`.
  - **Uncertainty.** Let `s_A`, `s_B` be the slopes at `V_th` on the two edges (V/ns), and δ the timing uncertainty of `(P_B − P_A) − (a_B − a_A)` (the scope's time base, the averaging, step 4's drift). Then `u = δ / (1/s_B − 1/s_A)`, plus the vertical reference's offset. At the expected edges (τ about 0.4 µs, then 0.87 µs with 47 pF), `1/s_B − 1/s_A` is about 0.29 ns/mV, so δ = 5 ns gives about ±17 mV and δ = 10 ns about ±35 mV [computed]. Write `u` down from your own captures.
- **Pass:** `V_th` lies within DS12117 Rev 10's by-design input thresholds at the measured VDD, widened by `u`: **`0.4·VDD − 0.1 − u ≤ V_th ≤ 0.47·VDD + 0.25 + u`**, which is **1.22–1.80 V** at 3.30 V (Table 146, p. 241 for rev V; Table 49, p. 136 for rev Y; "guaranteed by design", note 2). A rising threshold must sit above the input's guaranteed low level and at or below its guaranteed high level. Source: **rule** (DS12117, read at source).
  - **It fails for the named reason:** a controller counting from 0.7·VDD would put `V_th` near 2.31 V, 0.5 V above the band's top.
  - **If `u` exceeds 0.25 V,** the band no longer separates the two cases: the row is then a measurement with no pass line, and says so.
- **Record, no pass line:** `V_th`, `u`, `t_SYNC1`, `t_SYNC2`, `T_k` by both routes, and VDD. RM0433 names the delays' parts (the analog filter, which is on here: `ANFOFF` 0; two to three kernel cycles, 31–47 ns) but gives no figure for the filter's delay. They are **not predicted**.
- **On a disagreement:** the northstar §2 row is revised with our copper's figure, and the twin's controller-timing note (roadmap §7.1) with it.

## B8. USART boot through BOOT0, and the console

- **Tests:**
  - BOOT0 on J2 pin 2, through the judged 10 kΩ pull-down R8 (`1312` P1);
  - the console on USART1, PA9/PA10, J1;
  - `3301`'s user path with no SWD probe.
- **Instrument:** a 3.3 V USB-UART on J1, wired as B1 says (its RX to J1 pin 1, the board's TX; its TX to J1 pin 2, the board's RX; GND to pin 4), a DMM on TP2 (BOOT0), and the reset switch SW1.
- **The input thresholds (DS12117 Rev 10, Table 49, read at source by `1312`):** BOOT0 VIH min = 0.17·VDD + 0.6 V = **1.161 V** at VDD 3.3 V; VIL max = 0.19·VDD + 0.1 V = **0.727 V**; leakage at most 15 nA.
- **Method and pass:**
  1. **Not driven.** R8 is proved by Before power, step 3 (TP2 to TP1 = 10 kΩ). With J2 pin 2 open, BOOT0 reads at most 0.727 V (prediction 15 nA × 10 kΩ = 0.15 mV). Source: **rule** (`1312`'s judgement). This line alone cannot see a missing R8. Press SW1: the console sends the 12 bytes of B1 item 3. Source: **twin**.
  2. **R8 under load** (review F5's second test; optional once step 3 has passed). Drive J2 pin 2 from 3.3 V through a 100 kΩ resistor: TP2 reads **0.25–0.35 V** (3.3 × 10 k / 110 k = 0.300 V; ±1 % R8, ±5 % 100 kΩ). Without R8 it reads near 3.3 V and fails. Source: **rule** (the divider).
  3. **Driven.** Drive J2 pin 2 to 3.3 V directly (the driver sources 3.3 V / 10 kΩ = 0.33 mA). BOOT0 reads at least 1.161 V, with **margin 3.3 − 1.161 = 2.14 V**; at a driver of VDD − 0.4 V the margin is 1.74 V (`1312`'s judgement). Source: **rule**. This line fails only on a short to GND: it is a check, not a judgement of R8. Press SW1.
     - The system bootloader should answer `0x7F` with ACK `0x79` on USART1 at 8E1. **That is [inferred]: AN2606 is unread** (its download timed out three times, `3400` FEEDBACK §2.1 T8), so the row's protocol line is a gap until it is read. Source: **not predicted**.
- **On a disagreement:**
  - a BOOT0 voltage out of its line is a corrections row against `1312`'s judgement (`bench::judge_boot0_pull_down`);
  - a bootloader that does not answer, with BOOT0 high, is a finding against the AN2606 gap first.

## B9. I2C1 on silicon: PB6/PB7 at AF4, and the TMP117 (review F6, U1)

- **Tests:** PB6/PB7 at AF4 for I2C1 and the SENSORS copper, which no other row witnesses. B2's `AFRL` row reads back what the generator wrote, so a wrong AF in both the generator and its table would agree there (review F6, F7).
- **Instrument:** the FX2 analyzer (`sigrok-cli`, `D0` on TP13 I2C_SCL, `D1` on TP14 I2C_SDA, ground to TP1), or a scope on the same pads; the SWD probe.
- **1. The firmware's own transaction, across reset.** Start an untriggered 4 s capture, then reset within about a second (`AET-WO-3304.bench.md` B5 says why: a triggered capture loses the first START). Decode:
  ```
  sigrok-cli -d fx2lafw --config samplerate=24m --channels D0,D1 --time 4s -o s01-i2c1.sr
  sigrok-cli -i s01-i2c1.sr -P i2c:scl=D0:sda=D1 -A i2c=addr-data
  ```
  **Pass:**
  - exactly these lines, once:
    ```
    i2c-1: Start
    i2c-1: Write
    i2c-1: Address write: 76
    i2c-1: NACK
    i2c-1: Stop
    ```
    On a scope, the address byte is `1110 1100` (`0xEC`, the 7-bit `0x76` and the write bit). Source: **generator** (the address, `main.rs` L179) and **RM0433** (the STOP after a NACK, p. 1975); **repeat** (`3302`: `0xEC`, NACK, STOP).
  - **SCL's period inside the address byte: at least 2.5247 µs** (at most 396.1 kHz). Expected **2.65–2.90 µs**, recorded beside it, not a pass line: the band is from `3302`'s Nucleo rig (about 59 pF), not this board. The floor is **rule** (`silicon.rs`: 160 programmed kernel cycles of `TIMINGR` `0x10C7123C` plus `t_SYNC1 + t_SYNC2` ≥ 4 cycles, at HSI's fastest 64.958976 MHz). The expected band is **repeat-derived** (`3302`'s ≈ 0.26 µs excess at 2K; `AET-WO-3304.bench.md` §B6). Read the period on a scope: the FX2's 41.7 ns sample is too coarse for the margin. "400 kHz" exactly is not a pass line: a correct board runs slower, by its edges and its synchronisation.
  - **Record 0** (`0x20000008`, `0x2000000C`) = `0x22020001` / `0x00000000` (U2 empty, above).
- **2. The TMP117 (U3, `0x48`) answers on I2C1, over SWD, no firmware change.** After the boot, start a second, longer untriggered capture (`--time 60s`, file `s01-i2c1-probe.sr`), then run the six commands within it:
  ```
  $P write --chip STM32H753ZITx b32 0x4000541C 0x00000030   # I2C1.ICR: clear STOPF, NACKF
  $P write --chip STM32H753ZITx b32 0x40005404 0x02002090   # I2C1.CR2: address 0x48, write, NBYTES 0, AUTOEND, START
  $P read  --chip STM32H753ZITx b32 0x40005418 1            # I2C1.ISR
  $P write --chip STM32H753ZITx b32 0x4000541C 0x00000030
  $P write --chip STM32H753ZITx b32 0x40005404 0x020020EC   # the same at 0x76, U2's empty address
  $P read  --chip STM32H753ZITx b32 0x40005418 1
  ```
  **Pass:**
  - **`0x48`: `ISR & 0x00008030` = `0x00000020`** (`STOPF` 1, `NACKF` 0, `BUSY` 0), and the decoder shows `Start`, `Write`, `Address write: 48`, **`ACK`**, `Stop`.
  - **`0x76`: `ISR & 0x00008030` = `0x00000030`** (`NACKF` 1), and `Address write: 76`, **`NACK`**, `Stop`. This is the control: it shows the probe's write reached the controller, and that the ACK above was the TMP117's, not a stuck-low SDA.
  - Source: the address is the specification's (`U3`, 72 = `0x48`, its ADD0 on GND); the flags are **RM0433**'s (with `AUTOEND` 1 and `NBYTES` 0, a STOP follows the address; on a NACK, `NACKF` and a STOP, p. 1975). The twin has no TMP117 model (`simulate`: `device_unmodelled`), so this is **not predicted by the twin**.
  - **Not a read of register `0x0F`:** a debugger-paced read holds SCL low past the TMP117's 20–40 ms bus timeout and would fail for a reason this row does not name (`AET-WO-3403.FEEDBACK.md` §2.1). The address probe never stretches SCL.
- **On a disagreement:** a NACK at `0x48` with the `0x76` control behaving implicates PB6/PB7's AF, the SENSORS copper or U3's fitting: record which line the scope shows not moving. A corrections row against the generator's AF for I2C1 if the lines are dead at the pins.

## What this plan cannot measure

- **A GND pad beside the I2C pads.** The placer cannot be steered to put one there (`AET-WO-3402` F1); a placer rule is on the architect's backlog (1313b). The probe grounds by lead (B6).
- **The buck, the flight sensors and the witness:** they are Sentinel-03's (ruled on PR #184).
- **SPI rise times** against `extract` are not a row: SPI's declared bound (30 pF) chooses the pin speed and is judged by `check speeds`. The SPI pads (TP8–TP10) are for logic timing.
- **U2's identity read on silicon** (record 0 = `0x60`): U2 is not fitted (above). B9 witnesses I2C1 through U3 instead.
- **The ordered ELF inside DS12117's conditions.** It runs at VOS3 with 64 MHz APB clocks (B2, `4240` F-VOS). Every reading here is of that ELF as ordered; `4240v`'s ELF re-runs B2 and B4.
