# AET-WO-3304 — bench sheet: the BMP581 and the ICM-42688-P on the Nucleo

Every prediction here was written by the 3304 builder **before** the bench, from `main` at `c3f8ddbb`, in the pinned tools image (`aethelgard-tools@sha256:e5fc7ddb…1b58`) on Furnace, 2026-11-19. Fill the blank columns at the bench. **Never edit a prediction: a disagreement is the result** (`INV-07`; `northstar.md` §2).

**What each prediction rests on.** Each row names its source, in `RUL-20`'s terms:
- **twin:** the twin's own run (Renode's models and the boot judge, or ngspice through the campaign).
- **rule:** an independent rule in `aethelgard-ir` that shares no code with the generator (`silicon.rs`).
- **generator:** the generated code's own word. This is consistency, not judgement, and only the bench judges it.
- **repeat:** `3302` already measured this on this board. The row checks for drift, and it is not a fresh prediction.
- **not predicted:** said plainly, with the reason.

## 0. Setup and safety

- Everything is 3.3 V. Power comes from the ST-LINK USB only. The breakouts take 3V3 from the Nucleo (CN8 pin 7), and **nothing touches 5V**.
- Change wiring only with the USB unplugged.
- **No solder bridge or jumper changes** (§B0): every pin this firmware uses is free on the board as delivered.
- **The instruments:**
  - The FX2 analyzer (24 MSa/s, so one sample is 41.7 ns; 1.6 V threshold), with `sigrok-cli` 0.7.2.
  - The FNIRSI 2C53T with a ×10 probe, optional, for the rise rows, as in `3302`.
  - `probe-rs` 0.32.0 at `~/tools/probe-rs/bin/probe-rs` on t9o777.
- **Chip for `probe-rs`:** `STM32H753ZITx`.

## 1. The firmware

**ELF:** `sentinel-02-bench.elf`, sha256 `375fd31a28f4442b796110f0df82d31a9e133ce4a12f2a982fbb923cfe369c02`.
- It is generated from `examples/sentinel-02-bench.json` (sha256 `25597b3d3f10a3df9609632d486bf301d0c64a99d8d12de5a94b9c29c6cbf4c3`). That is Sentinel-02's own BMP581 and ICM-42688-P rows, pinned to Nucleo header pins, with nothing else on the board (`AET-WO-3304.FEEDBACK.md` §1 says why).
- It was built by `main`'s CLI in the pinned image (`generate`, then `build`).
- It was built twice, at two different paths, with one sha. The board crate's input hash is `7127d94a…f55b`; the image hash `simulate` reports is `8f3cbbc3…d306`.
- **On Furnace:** `~/.cache/aet-3304/bench/sentinel-02-bench.elf`, beside `SHA256SUMS`, the `simulate` verdict and the campaign lines. Copy it as `3302` did, then check:

```
scp furnace:.cache/aet-3304/bench/sentinel-02-bench.elf .
echo "375fd31a28f4442b796110f0df82d31a9e133ce4a12f2a982fbb923cfe369c02  sentinel-02-bench.elf" | sha256sum -c -
```

**The source runs** (all on Furnace, under `~/.cache/aet-3304/`):
- **S1:** `simulate --firmware … --for 100ms` on the emitted platform (`.repl` sha256 `051883e2…0b9a`). Verdict `pass`, saved as `bench/simulate-verdict.json`, sha256 `9b8a3728…52e7`.
- **S2:** `campaign`, seven scenarios on the same ELF (`bench/scen/`, `bench/campaign.jsonl` sha256 `2f845626…a8c`), whose run ids are named in each row.
  - Five matched their expectations in full.
  - `sda-held-low` and `spi-absent-u5` matched in verdict, and each raised one finding more than written before the run (FEEDBACK §2). Their records, used in B2, are as reported.
- **S2b:** `campaign`, three more scenarios at the declared 100 pF (3.3, 4.7 and 10 kΩ; `bench/campaign100.jsonl`, `bench/scen/pullup-*-100pf.json`). All three matched their expectations, written before the run from 0.8473·R·C.
- **S3:** `verify` on the specification, with `baseline-2k2`, `pullup-2200-60pf` and `unwired-47k`. **`pass`, every gate** (ERC, DRC, support, extract, six Kani proofs, build, simulate, campaign, attribute), on this ELF's sha. So `INV-03` holds for it. Its ledger classes the sources below (`AET-WO-3304.FEEDBACK.md` §3).

## B0. Pins and the board (UM2407 Rev 6, sha256 `b493e14f…a4cd`)

Defaults were read from the manual's bold rows (Table 10, Table 14 pp. 31–33). An independent reader checked them and agreed on every pin.

| Net | MCU pin | Function | Header pin | On the Nucleo by default | Change needed? | Bench: confirmed? |
|---|---|---|---|---|---|---|
| I2C_SCL | PB6 | I2C1, AF4 | CN10 pin 14 (Zio D1) | not on the VCP: SB9 and SB18 OFF (Table 10). The VCP is USART3 on PD8/PD9 | none | |
| I2C_SDA | PB7 | I2C1, AF4 | CN10 pin 16 (Zio D0) | not on the VCP: SB34 and SB66 OFF (Table 10) | none | |
| IMU_SCK | PA5 | SPI1, AF5 | CN7 pin 10 (D13) | LD1 is on PB0 (SB39 ON, SB47 OFF, bold) | none | |
| IMU_MISO | PA6 | SPI1, AF5 | CN7 pin 12 (D12) | nothing | none | |
| IMU_MOSI | PB5 | SPI1, AF5 | CN7 pin 14 (D11); also CN7 pin 13 (D22) | D11 is PB5 by default (SB33 OFF, SB35 ON, bold); PA7 is not on D11 | none | |
| IMU_CS | PD14 | GPIO output | CN7 pin 16 (D10) | nothing | none | |

**One thing the manual does not settle.** The bridges that route PB6 and PB7 to CN10 pins 14 and 16 (SB8 and SB68, Table 11) have no default state in the manual. The evidence that they are closed is `3302`'s bench: on these two header pins, the scope decoded the generated firmware's I2C traffic (`0xEC`, NACK, STOP).
- **Confirm with the meter, unpowered:**
  - continuity from CN10 pin 14 to CN12 pin 17 (PB6), and from CN10 pin 16 to CN11 pin 21 (PB7);
  - **no low resistance** from CN10 pin 14 or pin 16 to GND, or to 3V3.
- **Why the second check.** Table 11 also names SB12 (ON by default, the ST-LINK's TX to PD9) in the D0 route. The manual does not exclude a driver there. `3302` saw SDA float low with no hard driver.

**Ground and power:**
- GND is on CN8 pins 11 and 13, CN7 pin 8 and CN10 pin 17.
- 3V3 is on CN8 pin 7.

## B1. Flash

```
P=~/tools/probe-rs/bin/probe-rs
$P --version
$P download --chip STM32H753ZITx sentinel-02-bench.elf
$P reset --chip STM32H753ZITx
```

**Verify the flash.** If this `probe-rs` takes `download --verify` (check `$P download --help`, as `3302` did), download again with `--verify` and record its result.

**Either way, two checks are the verification of record:**
- the ELF's sha256 against §1, before the download;
- the boot word `0x2100b007` in B2, which only this image writes.

`--verify` result (or "flag not offered"): ________

`probe-rs` version: ________ · chip reported: ________ · `DBGMCU_IDC` (`$P read --chip STM32H753ZITx b32 0x5C001000 1`): ________ (`3302`: `0x20036450`, rev V)

## B2. Register image and records, nothing wired

Flash with no breakout and no pull-up connected (the analyzer may stay connected). Reset, wait at least 2 s, then read:

```
$P read --chip STM32H753ZITx b32 0x20000000 20
```

The other rows are one read each, for example `$P read --chip STM32H753ZITx b32 0x58020000 1`. Compare **only the masked bits**: `observed & mask` against the expected value.

### The register image (S1's boot judge: every row `observed` = `expected` in the twin)

**What each row's source means here.** The class of every written field is the one S3's `verify` ledger gives it:
- **generator:** the field is same-source. The twin only reads back what the firmware wrote.
- **twin (observed):** the model reacts to the field.
- **twin (Renode reset):** the firmware never writes the register. The expected value is Renode's reset value, so a silicon difference there is a disagreement with the model's reset value, not with the firmware.

| Register | Address | Mask | Expected | Source | Bench raw | & mask | Agree? |
|---|---|---|---|---|---|---|---|
| boot word | `0x20000000` | `0xffffffff` | `0x2100b007` | twin | | | |
| brown-out word (`FLASH_OPTSR_CUR` as read) | `0x20000004` | `0x0000000c` | `0x00000000` (`BOR_LEV` 00) | twin; **repeat** (`3302` B7 read `0x1BC6AAF0`; the twin's raw word is `0x0406aaf0`) | | | |
| GPIOA.MODER | `0x58020000` | `0x00003c00` | `0x00002800` | generator | | | |
| GPIOA.AFRL | `0x58020020` | `0x0ff00000` | `0x05500000` | generator | | | |
| GPIOB.MODER | `0x58020400` | `0x0000fc00` | `0x0000a800` | generator | | | |
| GPIOB.AFRL | `0x58020420` | `0xfff00000` | `0x44500000` | generator | | | |
| GPIOD.MODER | `0x58020c00` | `0x30000000` | `0x10000000` | generator | | | |
| RCC.AHB3ENR | `0x580244d4` | `0xffffffff` | `0x00000000` | twin (Renode reset) | | | |
| RCC.AHB1ENR | `0x580244d8` | `0xffffffff` | `0x00000000` | twin (Renode reset) | | | |
| RCC.AHB2ENR | `0x580244dc` | `0xffffffff` | `0x00000000` | twin (Renode reset) | | | |
| RCC.AHB4ENR | `0x580244e0` | `0xffffffff` | `0x0000000b` (GPIOA, B, D) | twin (observed: GPIOA, GPIOB); generator (GPIOD) | | | |
| RCC.APB3ENR | `0x580244e4` | `0xffffffff` | `0x00000000` | twin (Renode reset) | | | |
| RCC.APB1LENR | `0x580244e8` | `0xffffffff` | `0x00200000` (I2C1) | twin (observed) | | | |
| RCC.APB1HENR | `0x580244ec` | `0xffffffff` | `0x00000000` | twin (Renode reset) | | | |
| RCC.APB2ENR | `0x580244f0` | `0xffffffff` | `0x00001000` (SPI1) | twin (observed) | | | |
| RCC.APB4ENR | `0x580244f4` | `0xffffffff` | `0x00010000` | twin (Renode reset) | | | |
| SysTick.CSR | `0xe000e010` | `0x00010003` | `0x00000000` | twin (Renode reset) | | | |
| NVIC.ISER0 | `0xe000e100` | `0xffffffff` | `0x00000000` | twin (Renode reset) | | | |
| NVIC.ISER1 | `0xe000e104` | `0xffffffff` | `0x00000000` | twin (Renode reset) | | | |
| NVIC.ISER2 | `0xe000e108` | `0xffffffff` | `0x00000000` | twin (Renode reset) | | | |
| NVIC.ISER3 | `0xe000e10c` | `0xffffffff` | `0x00000000` | twin (Renode reset) | | | |
| NVIC.ISER4 | `0xe000e110` | `0xffffffff` | `0x00000000` | twin (Renode reset) | | | |

**The SPI controller: `3302` B8's rows after `4230`'s fix.**
- The twin's boot judge reads none of these registers.
- Each row's source is its class in S3's `verify` ledger (`RUL-20`):
  - **rule** means `silicon::spi_words`, held by `check_spi`;
  - **ignored-by-model** means no model reacts to it, so only the bench judges it.
- Renode models no `MODF`. The last row is the scar's.

| Register | Address | Mask | Expected | Source | Bench raw | & mask | Agree? |
|---|---|---|---|---|---|---|---|
| RCC.D1CCIPR (`CKPERSEL` = HSI, left at reset) | `0x5802444c` | `0x30000000` | `0x00000000` | ignored-by-model | | | |
| RCC.D2CCIP1R (`SPI123SEL` = per_ck) | `0x58024450` | `0x00007000` | `0x00004000` | rule | | | |
| SPI1.CR1 (idle after the last transfer: `SSI`, `SPE` 0) | `0x40013000` | `0x00001201` | `0x00001000` | rule | | | |
| SPI1.CFG1 (`MBR` 4, i.e. /32; `DSIZE` 7, i.e. 8 bits) | `0x40013008` | `0x7000001f` | `0x40000007` | rule | | | |
| SPI1.CFG2 (`AFCNTR`, `SSM`, `MASTER`; `CPOL` 0, `CPHA` 0) | `0x4001300c` | `0x87400000` | `0x84400000` | rule | | | |
| SPI1.SR (`MODF`, bit 9) | `0x40013014` | `0x00000200` | `0x00000000` | ignored-by-model: the scar's row | | | |

**Not stored by the twin** (its `unobservable` list). Read these for the record:
- `GPIOB.OTYPER` (`0x58020404`): PB6 and PB7 open-drain, `& 0x000000e0` = `0x000000c0`.
  - Ignored-by-model (Renode stores no `OTYPER`).
  - **Repeat** of `3302`'s `0x000000c0` for bits 6–7; PB5 push-pull.
- `I2C1.TIMINGR` (`0x40005410`): `0x10C7123C`, the solved Fast400k word.
  - **Generator, read back.** `silicon.rs` holds the fields only by inequalities (the ledger classes them independent-rule), and the exact word is the generator's own table.
  - So this row can fail only on a read-back difference.

| | GPIOB.OTYPER | I2C1.TIMINGR |
|---|---|---|
| Bench | | |

### The records, nothing wired

**The twin's bus, nothing wired.** The twin renders an unwired I2C bus as its devices' absence. It does so from either cause:
- the rise: S2 `unwired-47k`, 3,982.301 ns at 100 pF, over the 300 ns ceiling;
- SDA held low: S2 `sda-held-low`.

It claims no waveform for that bus. With no device, the wire form expected is `Start`, `Write`, `Address write: 46`, `NACK`, `Stop`, once per U7 step, from RM0433, not the twin. With no pull-ups at all, `3302` saw SDA float low and `ARLO` set, so the analyzer may show no START at all.

Status `0x22020000` is OK, `0x22020001` is TXIS, `0x22020005` is RXP and `0x22020006` is EOT. A record's value is in the words after its status, most significant first.

| Step | Status at | Value at | Predicted, nothing wired | Source | Bench status | Bench value |
|---|---|---|---|---|---|---|
| U7 BMP581 identity | `0x20000008` | `0x2000000c` | `0x22020001` (TXIS) / `0` | twin: S2 `unwired-47k` (`3a5de556…`) and `sda-held-low` (`a0396f68…`), the model's rendering of an absent bus (`AET-WO-2101` S6). **Repeat in kind:** `3302` read TXIS with no device on these pins and this driver | | |
| U7 configure | `0x20000010` | `0x20000014` | `0x22020001` / `0` | twin, as above | | |
| U5 ICM identity | `0x20000018` | `0x2000001c` | `0x22020000` (OK) / twin `0x000000ff`; silicon value not predicted | twin: S2 `spi-absent-u5` (`5d5029ba…`) gives OK / `0xff`. The `0xff` is Renode's mux dummy byte, a model convention. On silicon a master clocks with no slave, so the status is OK, and the value is whatever the floating PA6 (no pull) reads | | |
| U5 configure 1, 2, 3 | `0x20000020`, `0x20000028`, `0x20000030` | `+4` each | `0x22020000` / `0` each | twin, as above; silicon, as above | | |
| U7 data | `0x20000038` | `0x2000003c`, `0x20000040` | `0x22020001` / `0`, `0` | twin, as U7 identity | | |
| U5 data | `0x20000044` | `0x20000048`, `0x2000004c` | `0x22020000` / twin `0x0000ffff`, `0xffffffff`; silicon value not predicted | twin `0x0000ffff`, `0xffffffff` (the mux dummy); silicon floats | | |

## B3. The pull-ups

The breakouts carry their own pull-ups; the bench adds none. Read the value off the BMP581 breakout's resistors or its documentation, or measure it unpowered with the meter, from SCL to the breakout's VDD pin and from SDA to VDD.

| Breakout | SCL to VDD | SDA to VDD | Level shifter on the board? | Board name and revision |
|---|---|---|---|---|
| BMP581 | | | | |
| ICM-42688-P (on SPI lines; not a bus pull-up) | | | | |

**What the campaign predicts at the bench's capacitance.** C is `3302`'s measured 55 pF plus the analyzer's 5 pF, so 60 pF per line. Source: **twin**, S2, ngspice through the campaign, at 400 kHz against the 300 ns ceiling.

| Pull-up | Rise, 30–70 % | Campaign outcome | U7 records the twin then gives | Run id |
|---|---|---|---|---|
| 2.2 kΩ | 111.833 ns | pass | identity OK `0x50`; configure OK; data OK | `660a533e…` |
| 3.3 kΩ | 167.771 ns | pass | as 2.2 kΩ | `55953887…` |
| 4.7 kΩ | 238.939 ns | pass | as 2.2 kΩ | `ea2cf3c4…` |
| 10 kΩ | 508.376 ns | **fail**: `analog_rise_exceeds_ceiling`, `i2c_timeout` | **all three TXIS** (the model renders the bus absent) | `2125da01…` |
| any other R | 0.8473 × R × 60 pF (each value above follows it to 0.01 %) | fail above 300 ns | as the row on its side of 300 ns | — |

**The same, at the specification's declared 100 pF** (twin, S2b: `bench/campaign100.jsonl`, all three expectations matched). It applies only if a C0G capacitor per line brings each line to about 100 pF, as `3302`'s optional B6 did. Without one, these are the reference the campaign judges at, not the bench's prediction.

| Pull-up | Rise, 30–70 % | Campaign outcome | U7 records the twin then gives | Run id |
|---|---|---|---|---|
| 2.2 kΩ | 186.402 ns | pass | OK `0x50`; OK; OK | S1 |
| 3.3 kΩ | 279.601 ns | pass | as 2.2 kΩ | `d17d821f…` |
| 4.7 kΩ | 398.230 ns | **fail**: `analog_rise_exceeds_ceiling`, `i2c_timeout` | all three TXIS | `62bd35c3…` |
| 10 kΩ | 847.298 ns | **fail**: as 4.7 kΩ | all three TXIS | `a60e68da…` |

**What in these rows can fail:**
- **The side of 300 ns.** At 60 pF, the crossover is R = 300 ns ÷ (0.8473 × 60 pF) = **5.90 kΩ**:
  - 4.7 kΩ passes with 20 % margin;
  - at the declared 100 pF, S1's own C, 4.7 kΩ would fail at 398 ns.
- **The rise itself,** within **±15 %**. That is `3302`'s scatter of C (52–64 pF), plus the scope's ±4 ns at each threshold. "C implied" is the bench restated, not a prediction.

**The 10 kΩ row's prediction is the twin's: TXIS.** `3302`'s 100 kΩ run is why silicon might complete the read: the H7 times SCL-high from its own pin threshold, about 1.7 V. If silicon reads `0x50`, it is a disagreement with the twin, recorded with its size.

**What the 60 pF leaves out:**
- the BMP581's own pins: up to 10 pF declared, by UM10204 Ci, because Bosch prints none;
- the breakout's traces;
- any level shifter.

A level-shifted bus is not an RC line, and the rise rows do not apply to one: record it, and the row then says "not comparable".

Optional, with the scope as in `3302` (×10 probe, 30–70 % = 0.99 to 2.31 V), on the wired bus:

| Line | Rise, bench | C implied = t ÷ (0.8473 × R) | Prediction at the measured R (row above) |
|---|---|---|---|
| SCL | | | |
| SDA | | | |

## B4. Wiring

Unplug the USB first. **Ground first**, then 3V3, then signals.

| # | From (breakout pin) | To (Nucleo) | Analyzer |
|---|---|---|---|
| 1 | — | CN10 pin 17 (GND) | **GND** |
| 2 | BMP581 GND | CN8 pin 11 (GND) | — |
| 3 | ICM GND | CN7 pin 8 (GND) | — |
| 4 | BMP581 VDD (and VDDIO, if separate) | CN8 pin 7 (3V3) | — |
| 5 | ICM VDD (and VDDIO, if separate) | CN8 pin 7 (3V3) | — |
| 6 | BMP581 SCL (SCK) | CN10 pin 14 (PB6) | **CH1** (sigrok `D0`) |
| 7 | BMP581 SDA (SDI) | CN10 pin 16 (PB7) | **CH2** (`D1`) |
| 8 | BMP581 SDO | GND: address `0x46` (Bosch DS 5.6: "SDO must not be floating") | — |
| 9 | BMP581 CSB | 3V3 (I2C mode; DS Fig. 26), if the breakout does not already tie it | — |
| 10 | ICM SCLK (often labelled SCL) | CN7 pin 10 (PA5) | **CH3** (`D2`) |
| 11 | ICM SDI (often SDA), MOSI | CN7 pin 14 (PB5) | **CH4** (`D3`) |
| 12 | ICM SDO (often AD0), MISO | CN7 pin 12 (PA6) | **CH5** (`D4`) |
| 13 | ICM CS | CN7 pin 16 (PD14) | **CH6** (`D5`) |
| 14 | ICM FSYNC, if exposed and not tied on the board | GND (DS-000347 pin table: "Connect to GND if FSYNC not used") | — |
| 15 | ICM INT1/INT2; BMP581 INT | unconnected | — |
| — | analyzer CH7, CH8 | unconnected | — |
| — | analyzer **PWR** and **CLK** pins | **unconnected** | — |

**If the BMP581 breakout fixes SDO high** (address `0x47`) and it cannot be moved, do not rewire the specification at the bench. Run as is, and record that.
- The firmware addresses `0x46`, so the decoder will show `Address write: 46` then **`NACK`**, then `Stop`.
- The record will be `0x22020001` (TXIS), as in B2.

Breakout's SDO strap: ________

## B5. Capture and decode

**Check the decoders first.** The lines below are what libsigrokdecode 0.5.3's `i2c` and `spi` decoders print through `sigrok-cli` 0.7.2.
- Read from source:
  - `decoders/i2c/pd.py`, sha256 `bd28a545d42d61b2c82688d8a3c056d09af3c968890312159fbc01dc807ea6ba`;
  - `decoders/spi/pd.py`, sha256 `57d988e806614821d8aff561eca2f723f8a789e3b334755e8e97471e47bb212d`;
  - `sigrok-cli` 0.7.2's `decode.c`: the line is `<instance>: <longest text>`, and the first instance of a decoder is `i2c-1`, `spi-1` (libsigrokdecode `instance.c`).
- If the installed decoders differ, the lines may differ; record that, and compare by meaning.

```
sigrok-cli --version
sha256sum /usr/share/libsigrokdecode/decoders/i2c/pd.py /usr/share/libsigrokdecode/decoders/spi/pd.py
sigrok-cli --scan
```

Versions: ________ · i2c `pd.py` sha256 matches? ____ · spi `pd.py` sha256 matches? ____

**Capture.** Plug in the USB. Flash as in B1 if not already flashed. Start the capture, then **reset within about a second**, from a second terminal (or press B2, black). The capture runs 4 s.

```
sigrok-cli -d fx2lafw --config samplerate=24m --channels D0,D1,D2,D3,D4,D5 \
  --time 4s -o s02bench-wired.sr
# second terminal, within ~1 s:
~/tools/probe-rs/bin/probe-rs reset --chip STM32H753ZITx
```

**Why no trigger.**
- With `--wait-trigger`, `sigrok-cli` stores no sample before the trigger (`session.c`).
- The decoder takes sample 0 as the previous pin state (libsigrokdecode `instance.c`). A capture triggered on SDA's first fall therefore cannot see the first START. The identity read's `Start repeat` would then print as `Start`, a false disagreement.
- Untriggered, the bus idles high at sample 0: the firmware's last run left I2C idle and CS high.

**This is a warm reset.** The sensors stay powered, so the BMP581 and the ICM are still configured from the power-on run, while the twin modelled a cold boot.
- No predicted byte depends on that: the identity reads and the configuration writes are the same.
- The data reads come after the 20 ms wait either way.
- Only a cold-boot capture (unplug, start the capture, plug in) would differ, and only in the sensors' start-up state. Record which one you ran: ________

If `sigrok-cli` reports dropped samples at 24 MSa/s, repeat at `samplerate=12m`: that still gives 3 samples per SCK half-period at 2 MHz. Record which rate was used: ________

**Decode.** Run three passes over the one file:

```
sigrok-cli -i s02bench-wired.sr -P i2c:scl=D0:sda=D1 -A i2c=addr-data
sigrok-cli -i s02bench-wired.sr -P spi:clk=D2:mosi=D3:miso=D4:cs=D5:cpol=0:cpha=0 -A spi=mosi-transfer
sigrok-cli -i s02bench-wired.sr -P spi:clk=D2:mosi=D3:miso=D4:cs=D5:cpol=0:cpha=0 -A spi=miso-transfer
```

### The I2C transactions, U7 at `0x46`

**Every U7 block below, and every U7 row of "The records, wired", holds for the measured pull-up below B3's crossover (5.90 kΩ at 60 pF).**
- **Above it, the twin's prediction is B3's:** all three U7 steps record TXIS (`0x22020001`). The twin claims no waveform for that case.
- **The wire form then expected** is `Start`, `Write`, `Address write: 46`, `NACK`, `Stop`, once per step (three times), with no data bytes. Its source is RM0433's STOP after a NACK, not the twin.

Pick the one case by the R recorded in B3, before reading the capture.

The order follows the firmware: identity, configure, then the data read after the one wait.

**The line format.**
- `Write` or `Read` is the R/W bit's own annotation, printed **before** the `Address …` line.
- **`Start repeat` is not `Stop` then `Start`.** The decoder prints `Start repeat` only for a START with no STOP since the last one. A read that shows `Stop` then `Start` before `Read` is a **disagreement**, not agreement.

**Where the repeated start comes from.**
- The `Start repeat` lines come from the **generator**, not the twin: Renode's I2C model is transaction-level, and its witness (`read 0x01 -> 0x50`) cannot tell Sr from P+S.
- The generated `transfer` writes `CR2` with `START` after `TC`, with `AUTOEND` 0 in the write phase. By RM0433 that is a repeated start.
- The bench is the first judge of it. Bosch accepts either form (DS 5.6.2).

**Predicted output** (`#` comments are the sources, not decoder output):

```
# U7 identity: twin read 0x01 -> 0x50 (S1, kind 13)
i2c-1: Start
i2c-1: Write
i2c-1: Address write: 46
i2c-1: ACK
i2c-1: Data write: 01
i2c-1: ACK
i2c-1: Start repeat
i2c-1: Read
i2c-1: Address read: 46
i2c-1: ACK
i2c-1: Data read: 50
i2c-1: NACK
i2c-1: Stop
# U7 configure: twin write 0x36 <- 0x40 0x81 (one burst: OSR_CONFIG 0x40, ODR_CONFIG 0x81)
i2c-1: Start
i2c-1: Write
i2c-1: Address write: 46
i2c-1: ACK
i2c-1: Data write: 36
i2c-1: ACK
i2c-1: Data write: 40
i2c-1: ACK
i2c-1: Data write: 81
i2c-1: ACK
i2c-1: Stop
# U7 data, at least 20.0 ms later (B6): six bytes from 0x1D
i2c-1: Start
i2c-1: Write
i2c-1: Address write: 46
i2c-1: ACK
i2c-1: Data write: 1D
i2c-1: ACK
i2c-1: Start repeat
i2c-1: Read
i2c-1: Address read: 46
i2c-1: ACK
i2c-1: Data read: t0
i2c-1: ACK
i2c-1: Data read: t1
i2c-1: ACK
i2c-1: Data read: t2
i2c-1: ACK
i2c-1: Data read: p0
i2c-1: ACK
i2c-1: Data read: p1
i2c-1: ACK
i2c-1: Data read: p2
i2c-1: NACK
i2c-1: Stop
```

| Transaction | Predicted | Bench (paste) | Agree? |
|---|---|---|---|
| U7 identity | the 13 lines above | | |
| U7 configure | the 11 lines above | | |
| U7 data, framing (Sr, 6 reads, 5 ACK, last NACK, Stop) | as above | | |
| U7 data, values `t0..p2`: **the twin's** | `Data read: 00`, `00`, `19`, `40`, `F3`, `62` (S1, kind 13: `read 0x1D -> 0x00 0x00 0x19 0x40 0xF3 0x62`; its stimulus, 25 °C and 101,325 Pa) | | — |
| U7 data, values `t0..p2`: **silicon** | **not predicted:** the bench reads the room, not the stimulus | | — |

To read the data bytes: T = signed(`t2 t1 t0`) / 65,536 °C, and P = (`p2 p1 p0`) / 64 Pa (DS registers `0x1D`–`0x22`, the part table's channels). Bench T ____ °C · P ____ Pa (for the record; no prediction).

### The SPI transactions, U5, mode 0, CS on PD14

`mosi-transfer` and `miso-transfer` print one line per CS-low frame, emitted when CS rises.
- CS is driven high before PD14 becomes an output, so the first frame is the identity read.
- An empty `spi-1:` line before it can only come from PD14 floating during reset. It is not a transfer; note it if seen.

**MOSI:** source **twin** (the model's witness: `bank 0 read 0x75`, `write 0x4C <- 0xC0`, `0x50 <- 0x66`, `0x4E <- 0x03`, `read 0x1D`). The `00` fill bytes are the **generator's** read fill.

```
spi-1: F5 00
spi-1: 4C C0
spi-1: 50 66
spi-1: 4E 03
spi-1: 9D 00 00 00 00 00 00
```

**MISO, as the twin's model gives it.** This is the twin's full answer:
- The model returns `0x00` on every command byte and every write data byte (`models/renode/ICM42688P.cs` L97 and L99/L116, read at source; S1's witness records only the read bytes). Each read byte is its register.
- The data bytes are its stimulus: 29 °C, x 0 g, y +1 g.

```
spi-1: 00 47
spi-1: 00 00
spi-1: 00 00
spi-1: 00 00
spi-1: 00 12 02 00 00 00 40
```

**MISO, as predicted for silicon.** `??` marks a byte the twin's value does not carry over to silicon; the reasons are below the table.

```
spi-1: ?? 47
spi-1: ?? ??
spi-1: ?? ??
spi-1: ?? ??
spi-1: ?? d0 d1 d2 d3 d4 d5
```

| Frame | Predicted MOSI | Twin's MISO (model) | Predicted MISO, silicon | Bench MOSI | Bench MISO | Agree? |
|---|---|---|---|---|---|---|
| identity | `F5 00` | `00 47` | `?? 47` (twin S1: `0x47`) | | | |
| configure 1 (`INTF_CONFIG0` = `0xC0`) | `4C C0` | `00 00` | `?? ??` | | | |
| configure 2 (`ACCEL_CONFIG0` = `0x66`) | `50 66` | `00 00` | `?? ??` | | | |
| configure 3 (`PWR_MGMT0` = `0x03`) | `4E 03` | `00 00` | `?? ??` | | | |
| data | `9D 00 00 00 00 00 00` | `00 12 02 00 00 00 40` | `?? d0..d5` | | | |

**Not predicted, and why:**
- **The MISO byte during a command byte, and during a write's data byte.** DS-000347 says SDO is high-Z while CS is high, and nothing about it during the command byte. The twin's witness records only the bytes the model returns for a read.
- **`d0..d5`.** The twin's `12 02 00 00 00 40` is its stimulus: 29 °C, x 0 g, y +1 g. The bench reads the room and the breakout's attitude.

**For the record (no prediction).** Decode by `INTF_CONFIG0` = `0xC0`, `SENSOR_DATA_ENDIAN` 0, little-endian, so the lower address holds the low byte (DS-000347 p.74–75; the model reads it the same way):
- T = signed(`d1 d0`) / 132.48 + 25 °C;
- ax = signed(`d3 d2`) / 16,384 g;
- ay = signed(`d5 d4`) / 16,384 g.

The read does **not** touch the FIFO, so `4220c`'s B1–B4 (the FIFO's undocumented behaviour) do not enter any row here.

A sample of `00 80` (−32,768, the reset value) in x or y would mean no valid accelerometer sample yet. The generator's single wait is 20 ms against DS-000347's **10 ms typical**; no maximum is printed.

Bench T ____ °C · ax ____ g · ay ____ g · breakout attitude ________

### The records, wired

Read `$P read --chip STM32H753ZITx b32 0x20000000 20` after the capture.

| Step | Status at | Value at | Predicted | Source | Bench status | Bench value |
|---|---|---|---|---|---|---|
| U7 identity | `0x20000008` | `0x2000000c` | `0x22020000` / `0x00000050` | twin S1 (below B3's 5.90 kΩ crossover; above it, B3's row: TXIS for all three U7 steps) | | |
| U7 configure | `0x20000010` | `0x20000014` | `0x22020000` / `0x00000000` | twin S1 | | |
| U5 identity | `0x20000018` | `0x2000001c` | `0x22020000` / `0x00000047` | twin S1 | | |
| U5 configure 1–3 | `0x20000020`, `28`, `30` | `+4` each | `0x22020000` / `0` each | twin S1 | | |
| U7 data | `0x20000038` | `0x2000003c`, `0x20000040` | `0x22020000` / `0x0000t0t1`, `0xt2p0p1p2`: framing only (the twin: `0x00000000`, `0x1940f362`, its stimulus) | twin S1 (status and the twin's value); silicon value not predicted | | |
| U5 data | `0x20000044` | `0x20000048`, `0x2000004c` | `0x22020000` / `0x0000d0d1`, `0xd2d3d4d5`: framing only (the twin: `0x00001202`, `0x00000040`) | twin S1 (status and the twin's value); silicon value not predicted | | |

The record words and the decoded bytes must agree with each other, byte for byte. That is a check of the bench itself, not of the twin.

**After the capture, the controller's last writes.** The last transfer is U7's data read. Masked as `3302`. The sources are S3's ledger:
- `SADD`, `RD_WRN`, `NBYTES` and `TXDATA`: **observed** (the twin's model reacts to them);
- `AUTOEND`: **same-source**.

Either way the expected words are what the firmware wrote. These rows test only whether silicon reads them back after the STOP: **generator, read back**, as `3302`'s were.

| Register | Address | Mask | Expected | Bench raw | Agree? |
|---|---|---|---|---|---|
| I2C1.CR2 (`AUTOEND`, `NBYTES` 6, `RD_WRN`, `SADD` `0x8C`) | `0x40005404` | `0x02ff07ff` | `0x0206048c` | | |
| I2C1.TXDR (the last byte written, `0x1D`) | `0x40005428` | `0x000000ff` | `0x0000001d` | | |

## B6. Timing the analyzer can check

Read on the analyzer (in PulseView, from the same `.sr` file) or with the scope. **The twin claims none of these on silicon.**
- The twin runs instruction-level at 100 MIPS (`RUL-05`). Its only time for this ELF is the boot word at 39.039070 ms virtual (S1, `boot_word_ns`), which is a **model convention**, not a prediction.
- This firmware uses **no timer and no USART**. So nothing here rests on `4300a`'s timer conventions or `4400f`'s whole-ns realization, and **no row moves when `AET-WO-4400g` merges**. 4400g may change the emitted `.repl`'s hash (S1's input), but not this ELF or any row.

| Quantity | Predicted | Source | Resolution | Bench |
|---|---|---|---|---|
| SCL period, inside a byte | **≥ 2.5247 µs** (at most 396.1 kHz): 164 kernel cycles at HSI's fastest 64.958976 MHz | **rule:** `aethelgard-ir` `silicon.rs` (RM0433 §47.4.9 with `tSYNC1 + tSYNC2` ≥ 4 cycles; DS12117 Table 36). The excess over the bound is slopes, the analog filter (50–260 ns each, DS12117) and the H7's clock synchronisation (`3302`), none of which the rule credits | analyzer ±42 ns; scope ±4 ns | |
| SCL period, expected | **2.65 to 2.90 µs** at R near 2 kΩ: 160 programmed cycles (2.500 µs at 64 MHz) plus the ≈ 0.26 µs excess `3302` measured (2.6 µs for 150 programmed cycles, 2.344 µs, at 2K). The excess grows with the rise, so at a larger R expect more | **repeat-derived**, not the twin's | as above | |
| SPI SCK frequency | **2.000 MHz** at HSI's 64 MHz (/32); **1.948 to 2.030 MHz** over HSI's tolerance (−25,894 / +14,984 ppm) | **rule:** the solved `MBR` 4, judged by `silicon::check_spi` (`achieved_max_hz` 2,029,968 ≤ 4 MHz declared) | analyzer: 6 samples per half-period, so average over a byte's 8 clocks; scope for the figure | |
| SPI mode | `CPOL` 0, `CPHA` 0: **SCK is low whenever CS is high** | **rule** (S3's ledger: `CFG2.CPOL` and `CPHA` independent-rule; the part's mode by `validate`'s `SpiModeNotDriven`) | — | |
| Wait between U5 configure 3 (CS rises) and U7 data (Start) | **≥ 20.0 ms** | **generator:** 1,299,180 spins of at least one core cycle at ≤ 64.958976 MHz. S3's ledger classes it `silicon-only` (`WAIT.U5`, `WAIT.U7`). Not judged: silicon's cycles per spin (flash wait states, no I-cache at reset) can make it several times longer. Only the lower bound is claimed | ±42 ns | |
| Every other gap (between the first six transactions; between the two data reads) | twin: none recorded; silicon: **not predicted** | S1 records one time only, the boot word at 39.039070 ms virtual (a 100 MIPS convention); every trace event carries the run's end time | — | |

**The decoder cannot see the mode.** It samples on the rising edge in both mode 0 and mode 3, so a bus running mode 3 decodes to the same bytes, and B5's SPI lines would call it agreement. Only this row's idle level and B2's `CFG2` row can show the mode.

## B7. Results

Filled at the bench. Send the `.sr` file, the three decoder outputs, the `probe-rs` output and the sheet to the architect. **Every disagreement is written beside its prediction, with its size.**
