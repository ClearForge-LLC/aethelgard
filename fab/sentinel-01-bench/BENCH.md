# Sentinel-01 (`sentinel-01-bench`): the bench plan

The rows to run when the boards arrive (AET-WO-3401 §2.4). Each row says which prediction it tests, the instrument, the pass line and what a disagreement adds to the record. A pass line is a number from a cited prediction, and a reading outside it fails the row. A failed row is a finding, never a retry.

The board is `examples/sentinel-01.json`, and its package is `package/` beside this file. The ELF is `package/sentinel-01-bench.elf`, and its hash is in `package/MANIFEST.json`.

> **EXT is 3.3 V on pin 1, not DS-009 5 V. Plug in only 3.3 V-tolerant I2C devices.** (J4, the EXT port: DS-009 I2C's pinout, 1 VDD_3V3, 2 SCL, 3 SDA, 4 GND, with `standard: null`; ruled on PR #190, Q3.)

> **EXT's load budget.**
> - `extract`'s bound for EXT assumes at most **20 pF per line off-board**: J4's declared allowance, for the mated contact, a JST lead of up to 100 mm and one remote pin.
> - Anything plugged into J4 that adds more is outside the bound the gates judged.
> - The copper itself leaves about 71 pF per line before the declared 100 pF: a declared 70 pF off-board passes `extract`, and 85 pF fails it [measured on this board, 2026-11-17].

## Before any row

- **Probe capacitance, `C_p`.** Read `C_p` from your probe's own data (a ×10 passive probe prints its input capacitance, typically about 10–15 pF) and write it down before the first rise-time row. A probe clipped to a test pad is an off-board load, outside `extract`'s bound (ruled on #191, Q2). So every rise-time pass line below adds `C_p` to the line.
- **Supply.** J3 is the power header: pin 1 is `VDD_3V3`, the MCU's rail, which the board takes from J3 (`mcu.supply_source`); pin 4 is GND. Use a current-limited bench supply at 3.30 V, limited to 100 mA.
- **Test points** (bare 1 mm pads):

  | Pad | Net | Pad | Net |
  |---|---|---|---|
  | TP1 | GND | TP8 | SPI_MISO |
  | TP2 | BOOT0 | TP9 | SPI_MOSI |
  | TP3 | NRST | TP10 | SPI_SCK |
  | TP4 | VDD_3V3 | TP11 | EXT_SCL |
  | TP5 | CONSOLE_RX | TP12 | EXT_SDA |
  | TP6 | CONSOLE_TX | TP13 | I2C_SCL |
  | TP7 | FLASH_CS | TP14 | I2C_SDA |

  **Only one GND pad.** The specification refuses a second test point on one net (`TestPointNetRepeated`). The other ground points are the GND pins of J1–J4 (pin 4 of J1, J3 and J4; pin 6 of J2).

## B1. Programming by SWD, with connect-under-reset

- **Tests:** `3301`'s programming path on our copper, and NRST on J2 pin 3 (`3400` probe `p-nrst`, now built).
- **Instrument:** an SWD probe on J2 (JST SH 6: 1 VDD_3V3, 2 BOOT0, 3 NRST, 4 SWDIO, 5 SWCLK, 6 GND), with probe-rs or OpenOCD in connect-under-reset mode.
- **Pass:**
  1. The probe attaches while NRST is held low.
  2. `package/sentinel-01-bench.elf` flashes and verifies.
  3. The console (J1, 115200 8N1) prints the firmware's boot record.
- **On a disagreement:** a row in `architecture.md` §2.1 (corrections) naming the step that failed. A failure to attach under reset implicates J2 pin 3's net, which parity judged.

## B2. The register image, read back

- **Tests:** first silicon's rows (`AET-WO-3302.bench.md` B2), now on our board.
- **Instrument:** the SWD probe, reading after boot.
- **Pass:** each register the twin's run of this ELF records (the boot record of `verify`'s `simulate`, regenerated from this package) reads back with the same word. That includes:
  - GPIOA/GPIOB `MODER`, `AFRL`/`AFRH`, `OTYPER` and `PUPDR`;
  - `I2C1.CR1`/`TIMINGR` and `I2C2.CR1`/`TIMINGR` (`0x50D32A40` for both at 100 kHz; I2C1 runs at 400 kHz);
  - `SPI1.CFG1`/`CFG2`.

  The rows the specification acknowledges as unwitnessed (`acknowledge_unwitnessed`) are read here, each against the reason its entry gives.
- **On a disagreement:** a corrections row per register field, as `3302` B2 did.

## B3. `BOR_LEV`

- **Tests:** `bor_level: 3` and its programming by `package/FAB-NOTES.md`'s procedure (`4230`).
- **Instrument:** SWD: read `FLASH.OPTSR_CUR`.
- **Pass:** bits 3:2 (`BOR_LEV`) read `11` after the procedure. They read `00` before it, the factory default, as `3302` B7 measured.
- **On a disagreement:** a corrections row against `4230`'s factory step.

## B4. SPI1 with the `MODF` fix, and the flash's identity

- **Tests:** northstar §2's first scar, fixed by `4230` (`SSM`/`SSI` before `SPE`), on our copper.
- **Instrument:**
  - SWD for `SPI1.SR`/`CFG2`/`CR1`;
  - the console's boot record for the flash's JEDEC ID;
  - optionally a scope on TP10 (SPI_SCK). TP9 (SPI_MOSI) and TP8 (SPI_MISO) are there too.
- **Pass:**
  - `SPI1.SR.MODF` = 0;
  - `SPI1.CFG2.MASTER` = 1;
  - `SPE` is set during the transfer;
  - the identity reads **`EF 40 17`** (Winbond W25Q64JV, rev J §8.1, Read JEDEC ID 9Fh: `EFh`, then `4017h` for the IQ ordering codes; `parts.json`'s row, sha256 `452d906c…`).
- **On a disagreement:** `MODF` = 1 again would reopen the first scar. It is a northstar §2 row and a `4230` defect, not a board fault.

## B5. The BOR sweep

- **Tests:**
  - `bor_level: 3`, `2301`'s droop model;
  - `1208`'s MCU supply range, at DS12117 Rev 10 **Table 115** (p. 212, rev V; the rev Y half's Table 16 prints the same values): VBOR3 **falling 2.54 / 2.61 / 2.68 V** (min/typ/max), **rising 2.63 / 2.70 / 2.78 V**.
- **Instrument:** the bench supply on J3 pin 1, a DMM on TP4 (VDD_3V3) and a scope on TP3 (NRST).
- **Method:** with `BOR_LEV` = 3 (B3), lower the supply in 10 mV steps, held for 1 s each, from 3.30 V until NRST asserts. Record that VDD. Raise it again until NRST releases. Repeat three times.
- **Pass:** every falling trip is in **2.54–2.68 V** and every rising release in **2.63–2.78 V**, measured at TP4.
- **On a disagreement:** a corrections row against `power-parts.json`'s cited Table 115 rows. Those are the values `SupplyFloorBelowBorRising` judges, so a disagreement reaches every board.

## B6. Rise time at both operating points, against `extract`'s bound

- **Tests:**
  - `RUL-19` (1): the campaign's rise time is computed from declared values, and `extract` bounds the routed copper;
  - roadmap §3: Sentinel-01 is ordered to falsify the extractor;
  - the probe is added as an off-board load (ruled on #191, Q2).
- **Instrument:** a scope with the probe of known `C_p` on each pad, ground spring to TP1. Method as `3302` B3–B5: the debugger holds the line low through `GPIOx_BSRR`, then releases it, so each edge is the pull-up's RC with no firmware.
- **Prediction:** `t(30–70 %) = 0.8473 · R · C`.
  - R is the fitted pull-up at its +1 % limit, if the 1 % part in `ORDER.md` is fitted; use the fitted part's own tolerance otherwise.
  - C is `extract`'s bound for the line plus `C_p`.
  - The bounds are from this package's board at two layers, `extract` on `examples/sentinel-01.json`, measured 2026-11-17. EXT's bounds include J4's declared 20 pF off-board allowance, so with J4 empty the line is lower.
- **Pass:** the measured 30–70 % rise is at or below the upper bound, and that figure ignores ringing (`3302` B3 saw a release delay and ringing at 2K).

  | Line (pad) | R | `extract` bound | Upper bound on the rise (C_p in pF) | At C_p = 10 pF |
  |---|---|---|---|---|
  | I2C_SDA (TP14) | 2.2 kΩ | 58.6 pF | 1.883 ns/pF × (58.6 + C_p) | 129 ns |
  | I2C_SCL (TP13) | 2.2 kΩ | 53.4 pF | 1.883 ns/pF × (53.4 + C_p) | 119 ns |
  | EXT_SDA (TP12) | 10 kΩ | 48.6 pF | 8.558 ns/pF × (48.6 + C_p) | 501 ns |
  | EXT_SCL (TP11) | 10 kΩ | 47.5 pF | 8.558 ns/pF × (47.5 + C_p) | 492 ns |

  The campaign's own prediction, from the declared 100 pF, is 186 ns at 2.2 kΩ and 847 ns at 10 kΩ, under the 300 ns and 1000 ns ceilings. A board that passes this row is faster than the campaign assumed.
- **On a disagreement:** a rise above the upper bound means the copper plus probe exceeds `extract`'s bound. **That falsifies the extractor** and goes in `architecture.md` §2.1 with the line, `C_p` and the implied C (`t / (0.8473 · R)`).

## B7. The H7's I2C clock-high threshold, on the weak bus

- **Tests:** northstar §2's second divergence. The H7 counts `SCLH` from its own pin threshold, about 1.7 V (`3302` B5, measured on the Nucleo), not from 0.7·VDD = 2.31 V.
- **Instrument:** the scope on TP11 (EXT_SCL, 10 kΩ), J4 empty.
- **Method:**
  1. Over SWD, start an address phase on I2C2 (`CR2`: a 7-bit address with no device, `START`). The H7 clocks nine SCL pulses and gets a NACK.
  2. `TIMINGR` is `0x50D32A40` at a 64 MHz HSI kernel clock (the solved topology): PRESC 5 (93.75 ns), so SCLH is 43 × 93.75 ns = **4.031 µs**.
  3. Find the voltage `V_th` on SCL's rising edge from which the high phase lasts 4.031 µs, with HSI's stated −2.6 %/+1.5 % taken into account.
- **Pass:** `V_th` is within **1.7 ± 0.2 V**, and SCL's slow edge (about 0.5 µs at 10 kΩ) makes the high phase measurably shorter than 4.031 µs when timed from 0.7·VDD.
- **On a disagreement:** the northstar §2 row is revised with our copper's figure, and the twin's controller-timing note (roadmap §7.1) with it.

## B8. USART boot through BOOT0, and the console

- **Tests:**
  - BOOT0 on J2 pin 2, through the judged 10 kΩ pull-down R8 (`1312` P1);
  - the console on USART1, PA9/PA10, J1;
  - `3301`'s user path with no SWD probe.
- **Instrument:** a 3.3 V USB-UART on J1 (1 TX, 2 RX, 4 GND), a DMM on TP2 (BOOT0), and the reset switch SW1.
- **The input thresholds (DS12117 Rev 10, Table 49, read at source by `1312`):** BOOT0 VIH min = 0.17·VDD + 0.6 V = **1.161 V** at VDD 3.3 V; VIL max = 0.19·VDD + 0.1 V = **0.727 V**; leakage at most 15 nA.
- **Method:**
  1. **Not driven:** with J2 pin 2 open, BOOT0 must read at most 0.727 V. The prediction is 15 nA × 10 kΩ = 0.15 mV. Press SW1: the board boots from flash and the console prints the boot record.
  2. **Driven:** drive J2 pin 2 to 3.3 V through nothing but the pull-down (the driver sources 3.3 V / 10 kΩ = 0.33 mA). BOOT0 must read at least 1.161 V, with **margin 3.3 − 1.161 = 2.14 V**; at a driver of VDD − 0.4 V the margin is 1.74 V (1312's judgement). Press SW1.
     - The system bootloader should answer `0x7F` with ACK `0x79` on USART1 at 8E1. **That is [inferred]: AN2606 is unread** (its download timed out three times, `3400` FEEDBACK §2.1 T8), so the row's protocol line is a gap until it is read.
     - The voltage lines above are read at source.
- **On a disagreement:**
  - a BOOT0 voltage out of its line is a corrections row against `1312`'s judgement (`bench::judge_boot0_pull_down`);
  - a bootloader that does not answer, with BOOT0 high, is a finding against the AN2606 gap first.

## What this plan cannot measure

- **A second GND test point** (`TestPointNetRepeated`; J1–J4's GND pins stand in).
- **The buck, the flight sensors and the witness:** they are Sentinel-03's (ruled on PR #184).
- **SPI rise times** against `extract` are not a row: SPI's declared bound (30 pF) chooses the pin speed and is judged by `check speeds`. The SPI pads (TP8–TP10) are for logic timing.
