# AET-WO-3302 — bench sheet

Every prediction here was written by the architect **before** the bench, from `main` at `e3e3cd2`, in the pinned image on Furnace (2026-11-13). Fill the blank columns at the bench. Never edit a prediction: a disagreement is the result.

**Firmware:** `sentinel-01.elf`, sha256 `2d9ee2641f6e0a1b6e48f1637cefe4fa01cab3f97849a1dd074474c879fe0d2f`.
- It is the ELF the passing `verify --scenarios scenarios/sentinel-01/pullup-2k2.json --fab-out` named, and the one `simulate --firmware` booted.
- On Furnace it is at `~/.cache/aet-3302-prep/sentinel-01.elf`. Copy it to the bench machine with `scp furnace:.cache/aet-3302-prep/sentinel-01.elf .`, then check `sha256sum sentinel-01.elf` against the hash above.

**The instrument, added 2026-11-15 before the bench (the scope arrived: FNIRSI 2C53T, 2 channels, 50 MHz, 250 MSa/s, 10× probes):**
- **Its own resolution is part of every rise-time row.** At 250 MSa/s one sample is 4 ns, so a 30–70 % edge is placed within about ±4 ns at each threshold. At 2.2 kΩ and the bench's likely 25–35 pF, the 30–70 % rise is about 45–65 ns, so a reading there carries roughly ±15 %. The 47 kΩ rise, around a microsecond, is resolved easily.
- **Check the manual for the sample rate with both channels on.** If two channels halve it, measure the B3 and B5 rises **one line at a time** (one channel on), and use the second channel only for the SCL period.
- **The probe is part of the bus.** A 10× probe adds its own input capacitance (read it off the probe or the manual), so C_bench in B3 includes it. Record the figure, and keep the same probe on the same line throughout.

Sample rate, both channels on (manual): ________ · Probe input capacitance at 10×: ________ pF

**Chip for `probe-rs`:** `STM32H753ZITx`. Run `probe-rs --version` and record it: ________

## B0. Pin conflicts (UM2407)

| Pin | Our use | On-board use (UM2407) | Solder bridge opened? |
|---|---|---|---|
| PB6 | I2C1 SCL, AF4 | | |
| PB7 | I2C1 SDA, AF4 | (expected: user LED LD2, to confirm) | |
| PA0 | flash CS, GPIO out | | |
| PA5 | SPI1 SCK, AF5 | | |
| PA6 | SPI1 MISO, AF5 | | |
| PA7 | SPI1 MOSI, AF5 | (expected: Ethernet PHY, to confirm) | |

## B1. Flash

```
probe-rs download --chip STM32H753ZITx sentinel-01.elf
probe-rs reset --chip STM32H753ZITx
```

`probe-rs` command syntax varies by version; check `probe-rs read --help` if a command is refused. The chip `probe-rs` reports: ________

## B2. Register image, no sensors wired

Let the firmware run for at least 1 s after reset, then read each address, for example `probe-rs read --chip STM32H753ZITx b32 0x58020000 1`. Compare **only the masked bits**: `observed & mask` against `expected`. The other bits are not ours. On real silicon, for example, PA13 and PA14 stay SWD.

| Register | Address | Mask | Expected (Renode) | Bench (raw) | Bench & mask | Agree? |
|---|---|---|---|---|---|---|
| boot word | `0x20000000` | `0xffffffff` | `0x2100b007` | | | |
| GPIOA.MODER | `0x58020000` | `0x0000fc03` | `0x0000a801` | | | |
| GPIOA.AFRL | `0x58020020` | `0xfff00000` | `0x55500000` | | | |
| GPIOB.MODER | `0x58020400` | `0x0000f000` | `0x0000a000` | | | |
| GPIOB.AFRL | `0x58020420` | `0xff000000` | `0x44000000` | | | |
| RCC.AHB4ENR | `0x580244e0` | `0xffffffff` | `0x00000003` | | | |
| RCC.APB1LENR | `0x580244e8` | `0xffffffff` | `0x00200000` | | | |
| RCC.APB2ENR | `0x580244f0` | `0xffffffff` | `0x00001000` | | | |

**Prediction, no sensor wired:** the BME280 identity record's status at `0x20000004` reads a timeout code, not `0x22020000`:
- `0x22020001` means TXIS;
- `0x22020002` means TC;
- `0x22020003` means RXNE;
- `0x22020004` means STOPF.

Status: ________ · value at `0x20000008`: ________

Not comparable on silicon: `GPIOB.OTYPER` and `I2C1.TIMINGR` (Renode stores neither; the silicon does, so read them for the record: ________ / ________).

## B3–B5. The bus

**Wiring:** BME280 on SDO low (0x76) and TMP117 on ADD0 to GND (0x48), 3.3 V, with their own pull-ups removed, on PB6 and PB7. Then the pull-up under test to 3.3 V, one per line. Reset after each change.

**Scope:** 10× probe. Rise time is 30 % to 70 %, 0.99 V to 2.31 V. Save one screenshot per line per pull-up.

**The firmware's record:**
- status at `0x20000004`, where OK is `0x22020000`;
- value at `0x20000008`, where the BME280 ID `0x60` is expected on OK.

**The controller witness:**
- `I2C1.CR2` at `0x40005404`, masked with `0x02ff07ff`, expected `0x020104ec`;
- `I2C1.TXDR` at `0x40005428`, masked with `0x000000ff`, expected `0x000000d0`.

### B3. 2.2 kΩ

| Quantity | Campaign (declared 100 pF) | Bench |
|---|---|---|
| SCL rise, 30–70 % | 186.4 ns (ngspice, `rise_30_70_ns`) | |
| SDA rise, 30–70 % | 186.4 ns | |
| Bench C per line = t_rise ÷ (0.8473 × 2200) | 100 pF declared | SCL ____ pF · SDA ____ pF |
| SCL period (400 kHz = 2.5 µs) | 2.5 µs | |
| Record status / value | `0x22020000` / `0x60` | |
| CR2 & mask / TXDR & mask | `0x020104ec` / `0xd0` | |

### B4. 47 kΩ

| Quantity | Campaign | Bench |
|---|---|---|
| Rise, 30–70 % | 3.98 µs at 100 pF; at the bench's C: 0.8473 × 47,000 × C_bench = ____ | |
| Outcome | **fail:** `analog_rise_exceeds_ceiling` and `i2c_timeout` (the model renders the bus as the sensor's absence) | |
| SCL period | the campaign does not model clock synchronisation | |
| Record status / value | a timeout code | |

**Architect's note, written before the bench:** a real STM32 I2C controller waits for SCL to reach high before timing the next phase (clock synchronisation). So the real bus may simply run slower and read `0x60`, or fail on SDA timing. Either is a result.

### B5. 2.7 kΩ (`propose`'s answer)

| Quantity | Campaign | Bench |
|---|---|---|
| Rise, 30–70 % | 228.8 ns at 100 pF; at the bench's C: 0.8473 × 2,700 × C_bench = ____ | |
| Outcome | pass: re-gated by `propose` | |
| Record status / value | `0x22020000` / `0x60` | |

### B6. Optional: at the declared 100 pF

C0G capacitor per line = 100 pF − C_bench, rounded to a stock value: ____ pF.

| Pull-up | Campaign rise | Bench rise | Outcome |
|---|---|---|---|
| 2.2 kΩ | 186.4 ns | | |
| 2.7 kΩ | 228.8 ns | | |
| 47 kΩ | 3.98 µs | | |

## B7. Optional, for `AET-WO-2301` D2: the delivered brownout level

`AET-WO-2301`'s brownout model needs the part's BOR level, which the option bytes set. No document we hold states the delivered default. Read it once:
- **Register:** `FLASH_OPTSR_CUR` at `0x5200201C`. This is RM0433's FLASH base `0x52002000` plus `0x1C`; check the address in RM0433 before trusting the bits.
- **Command:** `probe-rs read --chip STM32H753ZITx b32 0x5200201C 1`.
- **Field:** `BOR_LEV` is bits [3:2]:
  - `00` is BOR off, so only POR/PDR, about 1.62 V;
  - `01` is level 1;
  - `10` is level 2;
  - `11` is level 3.

Raw: ________ · BOR_LEV: ________

**Prediction, added 2026-11-14 before the bench, from ST's documents:**
- RM0433 Rev 9 §4.9.8 confirms the register: `FLASH_OPTSR_CUR` at offset `0x01C` from FLASH base `0x52002000`, so `0x5200201C`; `BOR_LEV` is bits [3:2], where `00` selects VBOR0.
- DS12117 Rev 10 Table 16 names VBOR0 as the POR/PDR threshold, so `00` is "no BOR above POR/PDR".
- RM0433 Table 21 gives `FLASH_OPTSR[15:0]`'s default factory value as `1010101011110000`. Its `RDP` byte decodes to `0xAA` (level 0), which checks the bit order.
- **Predicted:** `raw & 0x0000000C` = `0x0` (`BOR_LEV` `00`, VBOR0 only), unless the board's option bytes were changed after the factory.

This is evidence for the specification field (D2 (b)), not a default the engine assumes.

## B8. The SPI controller's clock, rate and slave-select (second Mythos review, A1)

**Prediction, added 2026-11-15 before the bench, from RM0433 Rev 9 (as quoted in the review) and ST's CMSIS header held in `crates/aethelgard-svd-hal/data/st-cmsis-device-h7/stm32h753xx.h` (addresses and bit positions read there):** the firmware above enables SPI1's bus clock but selects no kernel clock. `SPI123SEL` resets to `000`, `pll1_q_ck`, and PLL1 is off at reset. **So SPI1 produces no SCK at all.** The flash read the twin witnessed cannot happen on this silicon. `AET-WO-4230` fixes it; this row records the silicon before the fix.

Read after the same 1 s as B2. Compare only the masked bits.

| Register | Address | Mask | Predicted | Bench (raw) | Bench & mask | Agree? |
|---|---|---|---|---|---|---|
| RCC.CR (`PLL1ON`) | `0x58024400` | `0x01000000` | `0x00000000` | | | |
| RCC.D1CCIPR (`CKPERSEL`) | `0x5802444c` | `0x30000000` | `0x00000000` | | | |
| RCC.D2CCIP1R (`SPI123SEL`) | `0x58024450` | `0x00007000` | `0x00000000` (`pll1_q_ck`) | | | |
| SPI1.CR1 (`SSI`) | `0x40013000` | `0x00001000` | `0x00000000` | | | |
| SPI1.CFG1 (`MBR`) | `0x40013008` | `0x70000000` | `0x00000000` (never written) | | | |
| SPI1.CFG2 (`SSM`, `MASTER`) | `0x4001300c` | `0x04400000` | `0x00400000` (master, `SSM` clear) | | | |

**Scope, channel 1 on PA5 (SPI1 SCK, the Nucleo's D13), trigger on any edge, from reset for 1 s:**
- **Predicted:** no edge at all.
- **If edges appear,** record their frequency. The prediction is then wrong, and the result is a scar either way.

Edges seen: ________ · frequency if any: ________

## Results: first silicon, 2026-11-16

Filled by the architect from the session's own record. Scotty was at the bench. `probe-rs` ran over the tailnet: first on PRECISION777 (Ubuntu 24.04) for B1, B2, B7 and B8, then on t9o777 (Omarchy, the bench machine) for the scope work. **The predictions above are unedited; where they disagree, the result is below.**

**Setup:**
- NUCLEO-H753ZI with an ST-LINK V3, `0483:374e`. `probe-rs` 0.32.0 (`48f5e4d`), a user install.
- `DBGMCU_IDC` `0x5C001000` = `0x20036450`: `DEV_ID` `0x450`, `REV_ID` `0x2003` (rev V).
- The ELF's sha256 matched the sheet. Before the flash, the boot word read `0xa219dd69`, which is RAM residue.
- Scope: FNIRSI 2C53T with ×10 probes on both ends. Probe compensation was **not verified**, and an under-compensated probe slows displayed edges.
- The kit holds no 2.2K, 2.7K or 47K resistors, so B3–B5 used 2K, 5K1, 10K and 100K. **No sensors were wired;** the BME280 and TMP117 are still in shipping.

### B0, corrected from UM2407 rev 6 (held)

| Pin | Header | On this board |
|---|---|---|
| PB6 | CN10 pin 14 (Zio D1) | LPUART1 TX, routable to the ST-LINK virtual COM port by SB34/SB66 *(architect's correction, 2026-11-19, from `3304` §5: in UM2407 Table 11, PB6's VCP route is SB9 and SB18; SB34 and SB66 are PB7's)* |
| PB7 | CN10 pin 16 (Zio D0) | LPUART1 RX, likewise |

- **The sheet's "PB7: LD2" expectation was wrong.** It came from an older Nucleo-144. On this board, LD1 is PB0/PA5, LD2 is PE1 and LD3 is PB14.
- Ground is on CN10 pin 17 and CN8 pins 11 and 13; 3V3 is on CN8 pin 7.
- PA7's Ethernet use is still to be confirmed.

### B1 and B2: **agree**

- The download finished in 1.94 s, and the reset was clean.
- The boot word and all seven masked rows agreed: every `& mask` equals the expected column.
- Raw values:

| Register | Raw |
|---|---|
| GPIOA.MODER | `abffabfd` |
| GPIOA.AFRL | `55500000` |
| GPIOB.MODER | `ffffaebf` |
| GPIOB.AFRL | `44000000` |
| AHB4ENR | `00000003` |
| APB1LENR | `00200000` |
| APB2ENR | `00001000` |

- **Status `0x22020001` (TXIS) and value 0, as predicted.**
- For the record:
  - `GPIOB.OTYPER` = `0x000000c0` (open-drain on PB6 and PB7);
  - `I2C1.TIMINGR` = `0x00602173`;
  - `I2C1.CR1` = `0x00000001`.
- `I2C1.ISR` read `0x00000201`, which is `ARLO`. With no pull-ups, SDA floats low; `GPIOB.IDR` read `0x50`.

### B7: **agrees**

- `FLASH_OPTSR_CUR` = `0x1BC6AAF0`, so `BOR_LEV` is `00`.
- The low 16 bits are exactly Table 21's factory value (`RDP` `0xAA`).
- The whole word is neither Table 21's `0x0006_AAF0` nor Renode's `0x0406_AAF0`. A firmware or a factory step must therefore read-modify-write it (`AET-WO-4230` Q-J).

### B8: **one row disagrees, and the disagreement is a scar**

| Register | Raw | & mask | Predicted | Agree |
|---|---|---|---|---|
| RCC.CR | `0004c025` | `0` | `0` | yes: PLL1 is off |
| RCC.D1CCIPR | `0` | `0` | `0` | yes |
| RCC.D2CCIP1R | `0` | `0` | `0` | yes: no kernel clock |
| SPI1.CR1 | `0` | `0` | `0` | yes, and `SPE` is 0 |
| SPI1.CFG1 | `00070007` | `0` | `0` | yes: the reset value |
| SPI1.CFG2 | `0` | `0` | `0x00400000` | **no: `MASTER` is clear** |

- `SPI1.SR` = `0x00001202`: **`MODF` is set.** With `SSM` = 0 and the internal NSS low, the first `SPE` raised a mode fault, and the H7 cleared `SPE` and `MASTER`.
- Renode models no `MODF`, so the twin passed what silicon refused.
- The SCK scope check was not run; with `MASTER` cleared, no SCK is possible.
- `4230` §1.3 (`SSM` = 1, `SSI` = 1, written before `SPE`) is the fix. This is the first silicon proof of review A1.

### B3–B5 with substitute pull-ups, and no sensors

**Method:**
- The debugger held PB6 low through `BSRR` and then released it, so each edge is the pull-up's RC with no firmware involved.
- Predictions were written before each reading, from `t(30–70 %) = 0.8473·R·C`.
- The first band assumed 20–40 pF; every later band was predicted from the C measured at 2K.

| Pull-up | Predicted | Measured (30–70 %) | C implied | Result |
|---|---|---|---|---|
| 2K | 34–68 ns (prior) | ≈ 100 ns ± 15 | ≈ 59 pF | **missed:** the line is about 55 pF, not 20–40 |
| 5K1 | 205–305 ns | ≈ 220–235 ns | ≈ 52 pF | hit |
| 10K | 400–600 ns | ≈ 470 ns ± 30 | ≈ 55 pF | hit |
| 100K | 4.0–6.0 µs | ≈ 5.4 µs ± 0.3 | ≈ 64 pF | hit |

- **The bench line is 52–64 pF** (mean ≈ 57 pF), including the ×10 probe, the dupont wires, the breadboard, the Zio trace and the pin.
- **The rise scales linearly with R, so the RC model holds.** The 2K edge is inflated by a fixed release delay and by ringing.
- **The falling edge** (open-drain) takes about 10 ns, rings at a 40–50 ns period, undershoots by −0.5 V, and puts about 0.7 V of crosstalk onto SDA.

**The firmware's transfer with a 100K SCL pull-up (SDA on 2K):**
- The transfer completed: SDA decodes `1110 1100` (`0xEC`), then NACK, then STOP. Status `0x22020001`; no lockup.
- **The SCL period was about 8 µs, against 2.6 µs at 2K.** The prediction was 9–10 µs: the completion hit, and the period missed.
- SCL is a sawtooth peaking at about 1.7–1.8 V, and **it never reaches 0.7·VDD = 2.31 V.**

**Why the period missed:** the H7's clock synchronisation counts `SCLH` from when its own pin threshold sees SCL high. That threshold is about 1.7 V, the FT I/O's V_IH, not 0.7·VDD.

**The finding:**
- The controller is self-consistent at 100K, but a compliant peer (the BME280's V_IH is 0.7·VDDIO) would see no valid clock.
- So the campaign's `analog_rise_exceeds_ceiling` is **right for the bus**: the H7 survives, and its peers do not.
- **The twin's note:** the controller's timing uses the pin's threshold, and the bus verdict uses the peer's V_IH (roadmap §7.1).

### Still to run (clause 4 stays open)

- B3–B5 with the BME280 and TMP117, at 2.2 kΩ, 47 kΩ and 2.7 kΩ when those values are in hand. Or rule the substitutes in, scaled by the measured C.
- B6.
- From `4230`'s ruling:
  - `D2CKRDY` after reset;
  - programming `BOR_LEV` by `FAB-NOTES`' procedure;
  - the 120 ledger bench rows `4230` generates.
- The scars, written into `northstar.md` §2 and ratified by Scotty once the sensor rows have run: the SPI `MODF` and the H7's synchronisation threshold.
