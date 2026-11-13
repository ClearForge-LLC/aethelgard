# AET-WO-3302 — bench sheet

Every prediction here was written by the architect **before** the bench, from `main` at `e3e3cd2`, in the pinned image on Furnace (2026-11-13). Fill the blank columns at the bench. Never edit a prediction: a disagreement is the result.

**Firmware:** `sentinel-01.elf`, sha256 `2d9ee2641f6e0a1b6e48f1637cefe4fa01cab3f97849a1dd074474c879fe0d2f`.
- It is the ELF the passing `verify --scenarios scenarios/sentinel-01/pullup-2k2.json --fab-out` named, and the one `simulate --firmware` booted.
- On Furnace it is at `~/.cache/aet-3302-prep/sentinel-01.elf`. Copy it to the bench machine with `scp furnace:.cache/aet-3302-prep/sentinel-01.elf .`, then check `sha256sum sentinel-01.elf` against the hash above.

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

This is evidence for the specification field (D2 (b)), not a default the engine assumes.
