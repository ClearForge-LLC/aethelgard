# AET-WO-4220 S2/S3 — what ArduPilot's drivers do at probe, init and steady state

- Commit: `fb773567896014da007bca58358704d4a0d8516f` (tag Copter-4.7.2), confirmed with `git rev-parse HEAD` on `/home/forge/.cache/aet-4400/ardupilot`.
- Date: 2026-11-15
- **Read at source; nothing run.** No build, no checkout, no container. Datasheet register meanings are not asserted here; names and meanings are the driver's own (its `#define`s and comments).

Path legend (all under `/home/forge/.cache/aet-4400/ardupilot/libraries/`):

| short | file |
|---|---|
| IV3 | `AP_InertialSensor/AP_InertialSensor_Invensensev3.cpp` (`.h` = IV3h) |
| B088 | `AP_InertialSensor/AP_InertialSensor_BMI088.cpp` |
| B270 | `AP_InertialSensor/AP_InertialSensor_BMI270.cpp` |
| BMP388 | `AP_Baro/AP_Baro_BMP388.cpp` (serves BMP388 and BMP390) |
| BMP581 | `AP_Baro/AP_Baro_BMP581.cpp` |
| DPS | `AP_Baro/AP_Baro_DPS280.cpp` (serves DPS280 and DPS310) |
| ICP | `AP_Baro/AP_Baro_ICP201XX.cpp` (serves ICP-20100) |
| MS | `AP_Baro/AP_Baro_MS5611.cpp` |
| DEV | `AP_HAL/Device.cpp` |
| HWDEF | `AP_HAL_ChibiOS/hwdef/<board>/hwdef.dat` |

## 0. Common mechanics every section relies on

| item | behaviour | file:line |
|---|---|---|
| SPI read flag | `read_registers(reg)` ORs `_read_flag` into the first byte, then `transfer(&reg,1,recv,len)`. Every IMU driver here and BMP388/BMP581/DPS calls `set_read_flag(0x80)` on SPI. | DEV:183-192, DEV:211-214 |
| `write_register(reg,val,checked)` | one 2-byte transfer `{reg,val}`; if `checked` it records `(reg,val)` in the checked-register table. No read-back. | DEV:169-180 |
| checked-register table | `setup_checked_registers(n, freq)` allocates n slots, `freq` default 10. `set_checked_register` re-uses the slot if `(bank,reg)` matches, so writing one register twice costs one slot. A checked write made before `setup_checked_registers` is silently dropped (`regs == nullptr`). | DEV:40-56, DEV:86-107, Device.h:221 |
| `check_next_register()` | called from the periodic callback; every `freq`-th call it reads back one checked register (twice if the first read differs); on mismatch it rewrites the register and returns false (the driver only bumps an error counter). It never fails probe. A model must therefore return the last written value for checked registers. | DEV:112-154 |
| SPI default speed | a new `SPIDevice` starts at `SPEED_LOW` (the SPIDEV line's first frequency); `SPEED_HIGH` is the second. | AP_HAL_ChibiOS/SPIDevice.cpp:121-124, :148-156 |
| hwdef IMU line | `IMU <Driver> SPI:<devname> [SPI:<devname2>] <ROTATION> [INSTANCE:n]` becomes `HAL_INS_PROBEn = ADD_BACKEND(AP_InertialSensor_<Driver>::probe(*this, <spi dev(s)>, rotation))`. Probes run in hwdef order, no autodetect. | AP_HAL/hwdef/scripts/hwdef.py:382-425; AP_InertialSensor.cpp:1220-1222 |
| hwdef BARO line | `BARO <Driver> SPI:<devname>` or `BARO <Driver> I2C:<bus>:<addr>` becomes `probe_spi_dev(AP_Baro_<Driver>::probe, "<devname>")` or `probe_i2c_dev(..., bus, addr)`. For `DPS280` the script substitutes `probe_280`. It also forces `AP_BARO_<Driver>_ENABLED 1`. | hwdef.py:557-650 (esp. :603, :649); AP_Baro.cpp:527-558, :664-666 |
| DRDY / interrupts | None of the eight drivers references a DRDY pin, GPIO or interrupt (grep for `drdy|interrupt|gpio|attach_interrupt` over all eight files: only ICP's `REG_INTERRUPT_*` register names and a licence comment match). Some boards declare `..._DRDY INPUT` pins in hwdef (e.g. `Pixhawk5X/hwdef.dat:76`) but these drivers never read them. All reads are polled from `register_periodic_callback`. | grep over the 8 files |
| `INS_FAST_SAMPLE`, `INS_GYRO_RATE` | mask default 1 (instance 0 only); `INS_GYRO_RATE` default 1 on STM32H7/F7 (so `get_fast_sampling_rate()`=2), else 0 (=1). Affects only the IV3 callback rate/ODR. | AP_InertialSensor.cpp:55-56, :553, :86-91, :600; Backend.h:334-346 |
| `HAL_INS_HIGHRES_SAMPLE` | default 0; boards define a bitmask (e.g. `Aeromind6X/hwdef.dat:305` = 7). Off in the base case. | Backend.h:32-33, :339-341 |

---

## 1. ICM-42688-P (Invensensev3 driver)

Driver: IV3, class `AP_InertialSensor_Invensensev3` (IV3h:17), enum `Invensensev3_Type` (IV3h:34-43).

### 1.1 Identity (all whoami values the driver accepts)

`check_whoami()` reads register **0x75** first (IV3:1005, `INV3REG_WHOAMI` IV3:46) and switches (IV3:1007-1030):

| whoami @0x75 | maps to enum | defined | accepted at |
|---|---|---|---|
| 0x3B | ICM40609 | IV3:137 | IV3:1008-1010 |
| 0x47 (`INV3_ID_ICM42688_P`) | ICM42688 | IV3:139 | IV3:1011-1014 |
| 0xDB (`INV3_ID_ICM42688_V`) | ICM42688 | IV3:140 | IV3:1011-1014 |
| 0x42 | ICM42605 | IV3:138 | IV3:1015-1017 |
| 0x33 | ICM40605 | IV3:136 | IV3:1018-1020 |
| 0x6F | IIM42652 | IV3:141 | IV3:1021-1023 |
| 0x56 | IIM42653 | IV3:142 | IV3:1024-1026 |
| 0x67 | ICM42670 | IV3:143 | IV3:1027-1029 |

If 0x75 matches none, it reads **0x72** (`INV3REG_456_WHOAMI`, IV3:101, :1032) and accepts only 0xE9 (ICM45686, IV3:144, :1033-1037); otherwise probe returns false (IV3:1039).

### 1.2 Probe sequence (ICM-42688-P)

| step | op | register | value | file:line |
|---|---|---|---|---|
| 1 | — | `set_read_flag(0x80)` (SPI only) | | IV3:222-224 |
| 2 | — | `setup_checked_registers(8, 20)` on SPI (200 on I2C) | 8 slots | IV3:1088 |
| 3 | — | `set_speed(SPEED_LOW)` | | IV3:1091 |
| 4 | R | 0x75 WHOAMI | must be 0x47 (or 0xDB) | IV3:1005, :1011 |
| 5 | — | `set_speed(SPEED_HIGH)` | | IV3:1097 |
| 6 | — | sets `_clip_limit` only (no bus traffic) | | IV3:1099-1112 |
| 7 | — | the 42688 case has **no further probe-time bus traffic** (the ICM42670 and ICM45686 branches only) | | IV3:1114, :1148 |

Probe-time register traffic: **1 read (0x75), 0 writes**. No delay, no retry, no polling. Default (SPI) bus: MODE3, SPEED_LOW 2 MHz, HIGH 4 MHz in `PixPilot-V3/hwdef.dat:177`. Note: if the 42688 returns a whoami not in the table, the second read at 0x72 is also issued (IV3:1032).

### 1.3 Init write set (`start()`, run later by the frontend; not part of `probe`)

Order in `start()`: `set_speed(LOW)` (IV3:268) → `fifo_reset()` (IV3:352) → `set_filter_and_scaling()` (IV3:360) → `register_gyro/accel` → `set_speed(HIGH)` (IV3:376) → allocate buffer → `register_periodic_callback` (IV3:395). `fifo_config1 = 0x07` (IV3:271) for ICM42688.

Base case below: `HAL_INS_HIGHRES_SAMPLE=0`, `INS_GYRO_RATE=0`, 1 kHz, `odr_config=0x06`, AAF delt 6/deltsqr 36/bitshift 10, accel AAF 5/25/10 (IV3:752-759).

| step | op | register | value | meaning (driver's name) | file:line |
|---|---|---|---|---|---|
| 1 | W | 0x16 FIFO_CONFIG | 0x80 | FIFO_MODE stop-on-full | IV3:247 |
| 2 | W | 0x5F FIFO_CONFIG1 | 0x07 (0x17 if highres) | accel+gyro+temp into FIFO | IV3:249, :271, :334 |
| 3 | W | 0x4C INTF_CONFIG0 | 0xC0 | little-endian, fifo count in records, last-data hold | IV3:251 |
| 4 | W | 0x4B SIGNAL_PATH_RESET | 0x02 | (FIFO flush) | IV3:252 |
| 5 | W | 0x4E PWR_MGMT0 | 0x00 | gyro+accel off | IV3:844 |
| 6 | W | 0x4F GYRO_CONFIG0 | 0x06 (0x05 at 2 kHz) | gyro ODR | IV3:847 |
| 7 | W | 0x50 ACCEL_CONFIG0 | 0x06 (0x05 at 2 kHz) | accel ODR | IV3:849 |
| 8 | W | 0x76 BANK_SEL | 1 | (register_read_bank) | IV3:676 |
| 9 | R | 0x0B GYRO_CONFIG_STATIC2 (bank 1) | value `v` | | IV3:853, :677 |
| 10 | W | 0x76 BANK_SEL | 0 | | IV3:678 |
| 11 | W | 0x76 / 0x0B / 0x76 | 1 / `v & ~0x03` / 0 | AAF+notch enable | IV3:854, :697-699 |
| 12 | W | bank1 0x0C GYRO_CONFIG_STATIC3 | 6 (12 @2 kHz) | GYRO_AAF_DELT | IV3:855 |
| 13 | W | bank1 0x0D GYRO_CONFIG_STATIC4 | 36 = 0x24 (144=0x90 @2 kHz) | GYRO_AAF_DELTSQR low | IV3:856 |
| 14 | W | bank1 0x0E GYRO_CONFIG_STATIC5 | 0xA0 (0x80 @2 kHz) | BITSHIFT<<4 \| DELTSQR hi | IV3:857 |
| 15 | W | bank2 0x03 ACCEL_CONFIG_STATIC2 | 10 = 0x0A | ACCEL_AAF_DELT<<1 | IV3:860 |
| 16 | W | bank2 0x04 ACCEL_CONFIG_STATIC3 | 25 = 0x19 | ACCEL_AAF_DELTSQR | IV3:861 |
| 17 | W | bank2 0x05 ACCEL_CONFIG_STATIC4 | 0xA0 | BITSHIFT/DELTSQR hi | IV3:862 |
| 18 | R | 0x4D INTF_CONFIG1 | value `v` | | IV3:877 |
| 19 | W (**checked**) | 0x4D INTF_CONFIG1 | `(v & 0x3F) \| 0x40` | AFSR "stuck gyro" fix | IV3:890 (or :888 under `INVENSENSEV3_CLKIN_BITMASK`, IIM42652 only for RTC bits) |
| 20 | W | 0x4E PWR_MGMT0 | 0x0F | gyro+accel low-noise | IV3:901 |
| 21 | — | `delay_microseconds(300)` | | | IV3:902 |

Each "bank write" is a 3-transfer sequence BANK_SEL=n, reg=val, BANK_SEL=0 (`register_write_bank`, IV3:686-701). Banks used: 1 and 2 only (no bank 4 in this driver for the 42688; bank 4 is not referenced anywhere in IV3).

Counts for ICM-42688 (base case): **write transactions at init = 32** (4 fifo_reset + 3 + 2 bank-read selects + 7 bank writes x3 + 1 INTF_CONFIG1 + 1 PWR_MGMT0); **distinct addresses written = 16** (0x16, 0x5F, 0x4C, 0x4B, 0x4E, 0x4F, 0x50, 0x76, 0x4D, plus bank1 0x0B/0x0C/0x0D/0x0E and bank2 0x03/0x04/0x05); **reads at probe+init = 3** (0x75, bank1 0x0B, 0x4D). **Checked registers: 1** (0x4D) of the 8 allocated. No polled status bit at probe or init.

### 1.4 Steady-state read path

| item | value | file:line |
|---|---|---|
| mode | FIFO only ("always use FIFO") | IV3:351 |
| callback period | `backend_period_us = 1e6 / backend_rate_hz`; `backend_rate_hz` = 1000 (base), 2000 on H7 instance 0 default (fast sampling, SPI only), up to 8000 | IV3:364, :395, :752, :778-782, :813-837 |
| count read | 2 bytes from 0x2E (FIFO_COUNTH) as host-order `uint16`: byte 0 = low (the driver set INTF_CONFIG0 little-endian), in packets | IV3:573, :583 |
| data read | one full-duplex SPI transfer: first byte `0x30 \| 0x80`, then `n*16` bytes; at most 8 packets per transfer (24 if `AP_INERTIALSENSOR_FAST_SAMPLE_WINDOW_ENABLED`) | IV3:574, :599, :605-611; IV3:185-189 |
| packet | 16 bytes `FIFOData`: header(1), accel 3x int16 LE, gyro 3x int16 LE, temp int8, timestamp uint16; `static_assert` == 16 | IV3:153-159, :173 |
| header check | `(header & 0xFC) == 0x68` else the whole FIFO is reset (`fifo_reset`) | IV3:458, :628-630 |
| highres packet | 20 bytes, header `(h & 0xFC) == 0x78`; only if `HAL_INS_HIGHRES_SAMPLE` bit set and SPI | IV3:161-168, :517 |
| DRDY | not used (no `#ifdef`, no pin) | grep |
| checked-reg check | every callback, at SPEED_LOW then back to HIGH; actual register read every 20th call (SPI) | IV3:632-641, :1088 |

### 1.5 hwdef lines

```
SPIDEV ICM42688         SPI1 DEVID6  42688_CS       MODE3   2*MHZ   4*MHZ    # PixPilot-V3/hwdef.dat:177
IMU Invensensev3 SPI:ICM42688         ROTATION_NONE                          # PixPilot-V3/hwdef.dat:183
```

No autodetect; `BARO_PROBE_EXT` is baro-only and does not apply. I2C is also accepted by the driver (`dev->bus_type()` checks, IV3:1088) but fast sampling and highres are SPI-only (IV3:779, :322).

### 1.6 Registers a minimal model must answer beyond identity

- 0x75 = 0x47.
- Writable, read-back-stable: 0x4D (checked), plus 0x76 BANK_SEL banking for 0x0B (read at IV3:853; any value tolerated, the write is `v & ~3`).
- FIFO: 0x2E/0x2F count (LE, packets), 0x30 data port (16-byte packets with header byte `(h&0xFC)==0x68`).
- Nothing is polled at init. If the model never produces FIFO data, `read_fifo` just sees `n_samples==0` (IV3:587).

---

## 2. ICM-45686 (same driver, `Invensensev3_Type::ICM45686`)

### 2.1 Identity

0x75 is read first (IV3:1005); with ICM45686 hardware the value must not equal any of 0x3B/0x47/0xDB/0x42/0x33/0x6F/0x56/0x67 (a 0xE9 at 0x75 is not in the first switch and falls through, harmless). Then **0x72 must read 0xE9** (`INV3_ID_ICM45686`, IV3:144; check IV3:1032-1037). So two reads: 0x75, 0x72.

### 2.2 Probe sequence

| step | op | register | value | file:line |
|---|---|---|---|---|
| 1 | — | `set_read_flag(0x80)`; `setup_checked_registers(8,20)`; `set_speed(LOW)` | | IV3:222-224, :1088, :1091 |
| 2 | R | 0x75 | any non-matching value | IV3:1005 |
| 3 | R | 0x72 | 0xE9 | IV3:1032-1034 |
| 4 | — | `set_speed(HIGH)` | | IV3:1097 |
| 5 | W | 0x7F REG_MISC2 | 0x02 (soft reset) | IV3:1151 |
| 6 | — | `delay_microseconds(1000)` | | IV3:1152 |
| 7 | R | 0x19 INT1_STATUS0 | **bit 0x80 must be set (reset done) or probe returns false** — single read, no retry | IV3:1154-1157 |
| 8 | W | 0x54 PWR_MGMT_AUX1 | 0x03 (turn off aux1) | IV3:1159 |
| 9 | W | 0x10 PWR_MGMT0 | 0x0F (gyro+accel low-noise) | IV3:1162 |
| 10 | IREG-R | indirect 0xA200+0x68 = **0xA268** (I3C_STC_MODE) | see 2.3 | IV3:1177 |
| 11 | IREG-W | 0xA268 | `v & ~0x04` | IV3:1178 |

(`#ifdef ICM45686_CLKIN`, off unless the board defines it, e.g. `CubeOrangePlus/hwdef.dat:70`, adds checked writes of 0x31, 0x30 and a write of 0x26: IV3:1164-1174. Verified by grep: `MicoAir743-Lite/hwdef.dat` has no CLKIN define.)

### 2.3 IREG indirect access (the "bank" mechanism for the 456)

Constants: `IREG_ADDRH 0x7C`, `IREG_ADDRL 0x7D`, `IREG_DATA 0x7E`, `REG_MISC2 0x7F` (IV3:119-122); bank bases `IMEM_SRAM 0x0000`, `IPREG_BAR 0xA000`, `IPREG_TOP1 0xA200`, `IPREG_SYS1 0xA400`, `IPREG_SYS2 0xA500` (IV3:125-129). `addr = bank_addr + reg`.

- Read (`register_read_bank_icm456xy`, IV3:1042-1063): one 3-byte write `[0x7C, addr>>8, addr&0xFF]` (burst; the model must auto-increment 0x7C→0x7D); then up to 10 times: read 0x7F and stop when `bit0` is set, `delay 10 us` otherwise (**not a hard fail if never set**); then read 0x7E.
- Write (`register_write_bank_icm456xy`, IV3:1065-1081): one 4-byte write `[0x7C, addr>>8, addr&0xFF, val]` (burst; the write to 0x7E is what latches the indirect write); then one read of 0x7F; if bit0 set return, else `delay 10 us`. Never fails.

### 2.4 Init write set (`start()`)

`fifo_reset()` first (456 branch, IV3:241-244), then `set_filter_and_scaling_icm456xy()` (IV3:934-998).

Rate: `backend_rate_hz = 800` (IV3:937); with fast sampling on SPI it is `calculate_fast_sampling_backend_rate(800, 6400)` (IV3:940-943), which on H7 instance 0 default is 1600. `backend_rate_hz <= 3200` → `odr_config = 0x04`, `sampling_rate_hz = 3200` (IV3:948-951); else 0x03 / 6400.

| step | op | register | value | meaning | file:line |
|---|---|---|---|---|---|
| 1 | W | 0x20 FIFO_CONFIG2 | 0x80 | FIFO_FLUSH | IV3:243 |
| 2 | W (**checked**) | 0x20 FIFO_CONFIG2 | 0x00 | | IV3:244 |
| 3 | W | 0x21 FIFO_CONFIG3 | 0x00 | disable FIFO first | IV3:957 |
| 4 | W | 0x1D FIFO_CONFIG0 | 0x00 | | IV3:958 |
| 5 | W | 0x1C GYRO_CONFIG0 | 0x04 (`(0<<4)\|odr`) | 4000 dps, ODR | IV3:961 |
| 6 | W | 0x1B ACCEL_CONFIG0 | 0x04 | 32 g, ODR | IV3:964 |
| 7 | IREG-R | 0xA258 (SMC_CONTROL_0) | `reg` | | IV3:968 |
| 8 | IREG-W | 0xA258 | `reg \| 0x01` (`\|0x10` first if `ICM45686_CLKIN`) | timestamps in FIFO | IV3:972, :969-971 |
| 9 | W (**checked**) | 0x21 FIFO_CONFIG3 | 0x06 (`FIFO_ACCEL_EN\|FIFO_GYRO_EN`; 0x0E with highres) | | IV3:974, :982 |
| 10 | W (**checked**) | 0x1D FIFO_CONFIG0 | 0x87 (`(2<<6)\|0x07`) | stop-on-full, bypass off, 2K FIFO off | IV3:985 |
| 11 | IREG-R | 0xA400+0xA6 = **0xA4A6** (GYRO_SRC_CTRL) | `reg` | | IV3:988 |
| 12 | IREG-W | 0xA4A6 | `(reg & ~0x60) \| 0x40` | gyro interpolator+AAF | IV3:989 |
| 13 | IREG-R | 0xA500+0x7B = **0xA57B** (ACCEL_SRC_CTRL) | `reg` | | IV3:992 |
| 14 | IREG-W | 0xA57B | `(reg & ~0x03) \| 0x02` | accel interpolator+AAF | IV3:993 |
| 15 | W (**checked**) | 0x21 FIFO_CONFIG3 | 0x07 (`\|FIFO_IF_EN`) | enable FIFO | IV3:996-997 |

Counts for ICM-45686:
- **Direct-register writes**: probe 3 (0x7F, 0x54, 0x10) + start 9 (0x20 x2, 0x21 x3, 0x1D x2, 0x1C, 0x1B) = **12 write transactions; 8 distinct addresses** (0x7F, 0x54, 0x10, 0x20, 0x21, 0x1D, 0x1C, 0x1B).
- **IREG writes**: 4 (0xA268, 0xA258, 0xA4A6, 0xA57B) = 4 distinct indirect addresses; each is preceded by an IREG read of the same address, so 4 IREG reads too. These 8 IREG operations go through the 0x7C-0x7E window (3 more distinct physical addresses).
- **Reads at probe+init**: direct 0x75, 0x72, 0x19 = 3; plus 0x7F once per IREG write and 1-10 times per IREG read, plus 0x7E x4.
- **Checked registers: 3** (0x20→0x00, 0x21→0x07, 0x1D→0x87) of the 8 allocated.
- **Polled status bits**: 0x19 bit 0x80 at probe (hard requirement, one read after 1 ms); 0x7F bit 0 around IREG (soft, bounded at 10 tries).

### 2.5 Steady state

| item | value | file:line |
|---|---|---|
| FIFO count | 2 bytes from 0x12 (INV3REG_456_FIFO_COUNTH) read as host `uint16` LE: byte at 0x12 is the low byte, 0x13 the high (`FIFO_COUNTL` is named at IV3:112 but unused). The driver never writes INTF_CONFIG0 (0x2C) for the 456, so it relies on power-on defaults being little-endian and counting packets (the same read/loop as 1.4 is used with count = packets). | IV3:111-112, :565, :583 |
| FIFO data | read at 0x14 (|0x80 = 0x94), full-duplex, n*16+1 bytes, same 16-byte `FIFOData` packet (20 B with highres); header `(h&0xFC)==0x68` (comment at IV3:456 says "ICM45686 - TMST_FIELD_EN bit 3") | IV3:113, :566, :605, :456-458 |
| callback | `backend_period_us` = 1250 us at 800 Hz (non-fast); 625 us at 1600 Hz (H7 inst 0) | IV3:937, :395 |
| DRDY | not used | grep |

### 2.6 hwdef lines

```
SPIDEV  icm45686    SPI3  DEVID1  ICM45686_CS    MODE3   2*MHZ 8*MHZ     # MicoAir743-Lite/hwdef.dat:130
IMU Invensensev3 SPI:icm45686 ROTATION_ROLL_180_YAW_90                    # MicoAir743-Lite/hwdef.dat:133
```
(CubeOrangePlus uses MODE0, `CubeOrangePlus/hwdef.dat:51`, and `define ICM45686_CLKIN 1`, :70.) No autodetect.

### 2.7 Model must answer beyond identity

0x72=0xE9; 0x19 with bit 0x80 set after the 0x7F=0x02 write; the 0x7C-0x7E IREG window with burst auto-increment and bit0 of 0x7F set (otherwise only a 10x10 us delay per read, values still returned from 0x7E); read-back-stable 0x20, 0x21, 0x1D (checked); FIFO count at 0x12/0x13, data at 0x14.

---

## 3. BMI088 (`AP_InertialSensor_BMI088`)

Separate accel and gyro SPI devices (two chip selects). The hwdef line carries both.

### 3.1 Identity

| device | register | value | file:line |
|---|---|---|---|
| accel | 0x00 REGA_CHIPID, read **twice** (first read is a throw-away "dummy read ... to init accel") | 0x1E = BMI088 (range 24 g); 0x1F = BMI085 (range 16 g); else probe fails | B088:204-224, B088:26 |
| gyro | 0x00 REGG_CHIPID | 0x0F | B088:243, B088:45 |

### 3.2 Bus setup

`init()` sets `set_read_flag(0x80)` on **both** devices (B088:288-289). The driver never calls `set_speed`, so both run at SPEED_LOW (first frequency of each SPIDEV line, 10 MHz in `BROTHERHOBBYH743`). Accel SPI reads use a special function that sends `[reg|0x80, 0, ...]` full-duplex over `len+2` bytes and **discards the first returned byte** (B088:122-138). Gyro reads are normal (`dev_gyro->read_registers`). On I2C the accel read would be normal (B088:125-127); the hwdef script accepts `I2C:` devices on an `IMU` line (hwdef.py:400-405) but every existing example uses SPI.

### 3.3 Probe + init sequence (probe = `init()`: accel_init then gyro_init)

| step | dev | op | register | value | file:line |
|---|---|---|---|---|---|
| 1 | accel | R | 0x00 | dummy | B088:205 |
| 2 | accel | R | 0x00 | 0x1E or 0x1F | B088:207-224 |
| 3 | accel | R | 0x40 REGA_CONF | compared with 0x9C | B088:161, :180-183 |
| 3w | accel | W+R-verify | 0x40 | 0x9C (only if read differs) | B088:186 |
| 4 | accel | R / W | 0x41 REGA_RANGE | 0x03 | B088:163 |
| 5 | accel | R / W | 0x7C REGA_PWR_CONF | 0x00 | B088:165 |
| 6 | accel | R / W | 0x7D REGA_PWR_CTRL | 0x04 | B088:166 |
| 7 | accel | R / W | 0x48 REGA_FIFO_CONFIG0 | 0x02 | B088:168 |
| 8 | accel | R / W | 0x49 REGA_FIFO_CONFIG1 | 0x50 | B088:169 |
| 9 | gyro | R | 0x00 | 0x0F | B088:243 |
| 10 | gyro | W (result ignored) | 0x14 REGG_BGW_SOFTRESET | 0xB6 | B088:251 |
| 11 | — | `delay(30)` ms | | | B088:252 |
| 12 | gyro | — | `setup_checked_registers(5, 20)` | | B088:254 |
| 13 | gyro | W (checked, result tested) | 0x0F REGG_RANGE | 0x00 (2000 dps) | B088:257 |
| 14 | gyro | W (checked) | 0x10 REGG_BW | 0x80 (532 Hz, no decimation) | B088:262 |
| 15 | gyro | W (checked) | 0x11 REGG_LPM1 | 0x00 | B088:267 |
| 16 | gyro | W (checked) | 0x13 REGG_RATE_HBW | 0x00 (filtered) | B088:272 |
| 17 | gyro | W (checked) | 0x3E REGG_FIFO_CONFIG_1 | 0x40 (stream XYZ, stop-at-full) | B088:277 |

Accel config detail (`setup_accel_config`, B088:172-193): for each of the 6 table entries (B088:159-170) it first **reads** the register and writes only when the value differs; each write is `write_accel_register` = write then read-back, up to 8 attempts, failing if never equal (B088:144-154). If `setup_accel_config()` fails at probe it is *not* fatal (B088:226-228): `read_fifo_accel` re-runs it every callback until it succeeds (B088:299-301). Probe therefore passes with any accel-register behaviour as long as the accel ID is 0x1E/0x1F, but the accel FIFO never starts unless the 6 registers read back as written.

Counts: **distinct registers written: accel 6** (0x40, 0x41, 0x7C, 0x7D, 0x48, 0x49), **gyro 6** (0x14 + 5 checked), total **12** (the accel writes happen only when the read-back differs from the target). **Reads at probe+init: accel 2 (ID) + 6 (config) = 8 minimum (+ one per write attempt); gyro 1.** **Checked registers: 5** (gyro: 0x0F, 0x10, 0x11, 0x13, 0x3E); accel none. **Polled status bit: none**, but accel config relies on read-back equality (retried up to 8x at probe, then every callback).

### 3.4 Steady state

| item | accel | gyro | file:line |
|---|---|---|---|
| callback | 1600 Hz (625 us) | 2000 Hz (500 us) | B088:59-63, :113-116 |
| length/status | 2 bytes from 0x24/0x25 (REGA_FIFO_LEN0/1), LE; `fifo_length & 0x8000` treated as empty | 0x0E REGG_FIFO_STATUS: bit 7 = overrun (rewrite 0x3E=0x40, checked), bits 0-6 = frame count | B088:42-43, :303-311; :50, :397-412 |
| data | burst from 0x26 REGA_FIFO_DATA, up to 56 bytes (8x7), via the dummy-byte SPI read | burst from 0x3F REGG_FIFO_DATA, up to 8 frames x 6 bytes | B088:41, :314-326; :57, :402, :424 |
| framing | variable frames, type = `p[0] & 0xFC`: 0x84 accel frame (7 bytes = hdr + 3 x int16 LE), 0x40 skip (2), 0x44 sensortime (4), 0x48 fifo-config (2), 0x50 sample-drop (2); loop runs while `fifo_length >= 7` | no header; 6 bytes = 3 x int16 LE, frame of all INT16_MIN skipped | B088:334-375; :403, :430-433 |
| temperature | every 101st accel callback: read 2 bytes at 0x22 (REGA_TEMP_MSB) | — | B088:377-388, :31 |
| checked regs | — | `check_next_register` every callback (read every 20th) | B088:441-446 |
| DRDY | not used | not used | grep |

### 3.5 hwdef lines

```
SPIDEV bmi088_g    SPI1 DEVID1  IMU1_CS           MODE3  10*MHZ  10*MHZ   # BROTHERHOBBYH743/hwdef.dat:179
SPIDEV bmi088_a    SPI1 DEVID2  BMI088_ACCEL_CS   MODE3  10*MHZ  10*MHZ   # BROTHERHOBBYH743/hwdef.dat:180
IMU BMI088 SPI:bmi088_a SPI:bmi088_g ROTATION_PITCH_180                    # BROTHERHOBBYH743/hwdef.dat:197
```
Order is accel then gyro (`probe(imu, dev_accel, dev_gyro, rotation)`, B088:78-83). No autodetect on hwdef boards (a BMI055/BMI088 board-match path exists in `AP_InertialSensor.cpp:1263-1268`, not used with hwdef `IMU` lines).

### 3.6 Model must answer beyond identity

Accel: 0x00 → 0x1E (answers twice), 0x40/0x41/0x7C/0x7D/0x48/0x49 as writable, read-back-stable registers (otherwise the driver retries 8x then defers), FIFO length 0x24/0x25 (bit 7 of 0x25 clear when non-empty), FIFO data 0x26 with 0x84 frames, the SPI dummy first byte. Gyro: 0x00 → 0x0F; 0x14 write accepted; 0x0F/0x10/0x11/0x13/0x3E read-back-stable (checked); 0x0E frame count, 0x3F data.

---

## 4. BMI270 (`AP_InertialSensor_BMI270`)

### 4.1 Identity

`BMI270_CHIP_ID 0x24` (B270:129) at 0x00 (`BMI270_REG_CHIP_ID`, B270:28). Compared at B270:537-540 against the **third** read (see below).

### 4.2 Bus setup

`init()` → `set_read_flag(0x80)` (B270:582); `hardware_init()` runs `set_speed(LOW)` (B270:518) ... `set_speed(HIGH)` (B270:571). Every driver register read discards the first returned byte (B270:238-250: send `[reg|0x80,0,..]`, `len+2` bytes full-duplex). The class's own `write_register` is **write then read-back-verify, up to 8 attempts** (B270:256-266); the return value is ignored by `hardware_init` and `configure_*`.

### 4.3 Probe sequence (`hardware_init`, up to 5 tries: `BMI270_HARDWARE_INIT_MAX_TRIES`, B270:144, :520)

| step | op | register | value | file:line |
|---|---|---|---|---|
| 0 | — | `delay(5)` ms before taking the semaphore | | B270:514 |
| 1 | R | 0x00 CHIP_ID | value discarded ("switch to SPI mode") | B270:527 |
| 2 | — | `delay(1)` ms | | B270:528 |
| 3 | W+verify | 0x7E CMD | 0xB6 (soft reset). Up to 8 writes if 0x7E does not read back 0xB6 | B270:530, :130 |
| 4 | — | `delay(5)` ms | | B270:531 |
| 5 | R | 0x00 | discarded | B270:534 |
| 6 | — | `delay(1)` ms | | B270:535 |
| 7 | R | 0x00 | **must be 0x24**, else `continue` to next try | B270:537-540 |
| 8 | W+verify | 0x7C PWR_CONF | 0x00 | B270:546 |
| 9 | — | `delay(1)` ms | | B270:547 |
| 10 | W+verify | 0x59 INIT_CTRL | 0x00 | B270:550 |
| 11 | W | **config blob** streamed as one SPI transfer, first byte = 0x5E (`BMI270_REG_INIT_DATA`) then 328 data bytes (329-byte transfer; `maximum_fifo_config_file`) | | B270:553, :105-124, :63 |
| 12 | W+verify | 0x59 INIT_CTRL | 0x01 | B270:556 |
| 13 | — | `delay(20)` ms | | B270:559 |
| 14 | R | 0x21 INTERNAL_STATUS | **bit 0 must be 1**, else retry whole sequence (max 5) | B270:561-568, :39 |

Config blob: defined in B270:105-124 as `const uint8_t AP_InertialSensor_BMI270::maximum_fifo_config_file[] = { BMI270_REG_INIT_DATA, ...328 bytes... }` (declared B270h:118). Counted with a script over the source text: 329 tokens including the leading register byte, so **328 payload bytes**. No checksum or readback of the blob; the driver only tests INTERNAL_STATUS bit 0 afterwards. Bosch BSD-3 licence block at B270:71-104, :125-127.

Write-verify consequence: 0x7E (CMD) normally reads back 0 on real silicon, so `write_register(CMD, 0xB6)` may issue 8 identical writes (B270:258-264); same for CMD=0xB0 in `fifo_reset` (B270:365). A model must tolerate repeated soft-reset writes; if it returns the written value for 0x7E, one write suffices.

### 4.4 Start (`start()`) write set

Order: `configure_accel()` → `configure_gyro()` → `configure_fifo()` (B270:207-211). Every write is `write_register` (verify, 8x).

| step | op | register | value | meaning | file:line |
|---|---|---|---|---|---|
| 1 | W | 0x40 ACC_CONF | 0x8C (`1<<7 \| 0x0C`) | high-perf, OSR4, 1600 Hz | B270:324 |
| 2 | W | 0x41 ACC_RANGE | 0x03 | 16 g | B270:326 |
| 3 | W | 0x42 GYRO_CONF | 0xED (`1<<7\|1<<6\|2<<4\|0x0D`) | | B270:335 |
| 4 | W | 0x43 GYRO_RANGE | 0x08 | 2000 dps (ois_range bit) | B270:339 |
| 5 | W | 0x48 FIFO_CONFIG_0 | 0x01 | stop when full, no sensortime | B270:347 |
| 6 | W | 0x49 FIFO_CONFIG_1 | 0xD0 (`1<<7\|1<<6\|1<<4`) | acc+gyr+header | B270:349 |
| 7 | W | 0x45 FIFO_DOWNS | 0x89 (`1<<7\|1<<3\|0x01`) | filtered, /2 → 1600 Hz | B270:351 |
| 8 | W | 0x7C PWR_CONF | 0x02 | | B270:353 |
| 9 | W | 0x7D PWR_CTRL | 0x0E | gyro+accel+temp on, aux off | B270:355 |
| 10 | W | 0x7E CMD | 0xB0 | FIFO flush | B270:365 (via `fifo_reset`, B270:357) |

Counts for BMI270: **distinct registers written, probe+start = 12** (0x7E, 0x7C, 0x59, 0x5E[blob], 0x40, 0x41, 0x42, 0x43, 0x45, 0x48, 0x49, 0x7D). **Reads at probe: 0x00 x3 per try, 0x21 x1 (plus a verify read for each write)**. **Checked registers: none** (no `setup_checked_registers`). **Polled status bit: yes — 0x21 INTERNAL_STATUS bit 0 after INIT_CTRL=1 (after a 20 ms delay), required for probe to pass.** `BMI270_DEBUG` off (B270:24) so `check_err_reg` is a no-op.

### 4.5 Steady state

| item | value | file:line |
|---|---|---|
| callback | 1600 Hz = 625 us, FIFO only | B270:134, :146, :225 |
| error check | read 0x02 ERR_REG each callback; bit 6 → `fifo_reset` | B270:378-382 |
| length | 2 bytes from 0x24/0x25 LE; `& 0x8000` = empty; cap 8x13 = 104 bytes | B270:385-402 |
| data | burst from 0x26 (with dummy first byte); frames by `p[0]&0xFC`: 0x84 accel(7), 0x88 gyro(7), **0x8C accel+gyro (13 bytes: header, gyro at p+1, accel at p+7)**, 0x40 skip(2), 0x44 sensortime(4), 0x48 config(5), 0x50 drop(2), 0x80 invalid → fifo_reset; loop while `fifo_length >= 12` | B270:404-458 |
| temperature | every 101st callback, 2 bytes from 0x22/0x23; 0x8000 invalid | B270:461-475, :40-41 |
| DRDY | not used | grep |

### 4.6 hwdef lines

```
SPIDEV bmi270        SPI2 DEVID3  BMI270_CS   MODE3   1*MHZ 10*MHZ     # CORVON743V1/hwdef.dat:156
IMU BMI270 SPI:bmi270 ROTATION_ROLL_180                                # CORVON743V1/hwdef.dat:161
```
(Simpler pair: `BeastH7v2/hwdef.dat:52` SPIDEV and `:58` IMU.) `probe()` is overloaded for `SPIDevice` and `I2CDevice` (B270:159-201), but `read_registers` always uses the SPI dummy-byte form (B270:238-250), so only SPI is coherent. No autodetect.

### 4.7 Model must answer beyond identity

0x00 → 0x24 (read 3x per try; only the third is checked); read-back for 0x7C, 0x59, 0x7E (or accept 8 retries) and every start-register; absorb the 329-byte transfer to 0x5E; **0x21 bit 0 = 1 once INIT_CTRL=1 was written** (the status read happens 20 ms later); 0x02 = 0 (bit 6 clear); FIFO 0x24/0x25 (bit 7 of 0x25 clear), 0x26, frames 0x8C.

---

## 5. BMP390 (BMP388 driver)

### 5.1 Identity

`BMP388_REG_ID 0x00`: 0x50 = BMP388 (`DEVTYPE_BARO_BMP388`), **0x60 = BMP390** (`DEVTYPE_BARO_BMP390`); other → false (BMP388:28-29, :31, :86-100). The BMP390 is explicitly described as using this driver in `ARKV6S/hwdef.dat:275`.

### 5.2 Probe + init sequence (`init()`)

| step | op | register | value | file:line |
|---|---|---|---|---|
| 1 | — | `set_speed(SPEED_HIGH)` | | BMP388:80 |
| 2 | — | SPI: `set_read_flag(0x80)` | | BMP388:83-85 |
| 3 | R (1 B) | 0x00 ID | 0x60 (BMP390) / 0x50 | BMP388:87-100 |
| 4 | W | 0x7E CMD | 0xB6 soft reset (**failure of the transfer fails probe**) | BMP388:103-105, :55 |
| 5 | — | `delay(10)` ms | | BMP388:106 |
| 6 | W (checked, but dropped: table not yet allocated) | 0x1B PWR_CTRL | 0x33 (normal mode, temp+press) | BMP388:109 |
| 7 | R (16 B) | 0x36 (`BMP388_REG_CAL_P`) | calib_p, 16 bytes | BMP388:112, :52, header BMP388.h:42-54 |
| 8 | R (5 B) | 0x31 (`BMP388_REG_CAL_T`) | calib_t, 5 bytes | BMP388:113, :53, header :56-60 |
| 9 | — | `scale_calibration_data()` | | BMP388:115, :180-198 |
| 10 | — | `setup_checked_registers(4)` (freq default 10) | | BMP388:117 |
| 11 | W (checked) | 0x1B PWR_CTRL | 0x33 | BMP388:120 |
| 12 | — | register 50 Hz callback | 20 ms | BMP388:127 |

On SPI every driver read has the dummy-byte form (`[reg|0x80, 0, ...]`, discard first byte): BMP388:242-258; on I2C it is a plain read (BMP388:245-247).

Calibration layout: `calib_p` packed `int16 p1 @0x36, int16 p2, int8 p3, int8 p4, int16 p5, int16 p6, int8 p7, int8 p8, int16 p9, int8 p10, int8 p11` (BMP388.h:42-54); `calib_t` `uint16 t1 @0x31, uint16 t2, int8 t3` (BMP388.h:56-60). Scaling in BMP388:183-197.

Counts: **registers written, distinct: 2** (0x7E, 0x1B; 0x1B twice). **Reads at probe+init: 3 transactions** (0x00 x1 B, 0x36 x16 B, 0x31 x5 B). **Checked registers: 1** (0x1B=0x33). **No polled status at probe or init** (the first status check is the first `timer()`).

### 5.3 Steady state

| item | value | file:line |
|---|---|---|
| callback | 50 Hz (20 ms) | BMP388:127 |
| read | one 7-byte read from 0x03: buf[0]=STATUS(0x03), buf[1..3]=PRESS 24-bit LE (0x04-0x06), buf[4..6]=TEMP 24-bit LE (0x07-0x09) | BMP388:137, :145-146, :35-36 |
| data-ready | `status & 0x20` → pressure sample; `status & 0x40` → temperature sample (**the model must set these bits or no sample is ever consumed**) | BMP388:147-154 |
| compensation | float polynomial: temperature `partial1 = data - par_t1; temp = partial1*par_t2 + partial1^2*par_t3`; pressure uses par_p1..p11 and the current temperature | BMP388:203-237 |
| check | `check_next_register()` each timer (read every 10th) | BMP388:156 |
| OSR/ODR/CONFIG | **not written** (power-on defaults used) | — |
| DRDY pin | not used | grep |

### 5.4 hwdef / probe-ext

```
BARO BMP388 I2C:0:0x76        # ARKV6S/hwdef.dat:276 (line 275: "barometers - BMP390 (uses the BMP388 driver)")
```
`BARO_PROBE_EXT` bit **10** (`PROBE_BMP388`, AP_Baro.h:266) probes I2C 0x76 and 0x77 on external buses (AP_Baro.cpp:804-807, :774-857; bit list at AP_Baro.cpp:185). Defaults `HAL_BARO_BMP388_I2C_ADDR 0x76`, `..._ADDR2 0x77` (BMP388.h:10-15). The driver also does SPI (dummy-byte read, BMP388:83-85, :242-258); real SPI lines: `BARO BMP388 SPI:bmp390` (`Airvolute-DCS2/hwdef.dat:166`), `BARO BMP388 SPI:BMP388` (`PixPilot-V6PRO/hwdef.dat:212`).

### 5.5 Model must answer beyond identity

0x00 = 0x60; 0x7E write accepted; 0x1B read-back-stable (checked); 0x31..0x35 and 0x36..0x45 as calibration (sane values or compensation yields NaN/garbage); 0x03 status with bits 0x20|0x40, and 0x04-0x09 data.

---

## 6. BMP581 (`AP_Baro_BMP581`)

### 6.1 Identity and status gates

| step | op | register | value | file:line |
|---|---|---|---|---|
| 1 | — | `set_speed(HIGH)` | | BMP581:81 |
| 2 | — | SPI only: `set_read_flag(0x80)` then **one throw-away read** of 0x01 | | BMP581:86-92 |
| 3 | R | 0x01 CHIP_ID | **0x50** (`BMP581_ID`) | BMP581:94-104, :23, :25 |
| 4 | R | 0x28 STATUS | **bit 1 (0b10) must be set and bit 2 (0b100) must be clear**, else probe fails | BMP581:106-113, :41 |
| 5 | R | 0x27 INT_STATUS | **bit 0x10 must be set**, else probe fails | BMP581:115-122, :40 |
| 6 | — | `setup_checked_registers(4)` (freq default 10) | | BMP581:124 |
| 7 | W (checked) | 0x37 ODR_CONFIG | 0x00 (standby) | BMP581:127, :53 |
| 8 | W (checked) | 0x36 OSR_CONFIG | 0x72 (`0b01110010`: press enable, osr_p 64x, osr_t 4x per driver comment) | BMP581:130, :52 |
| 9 | W (checked) | 0x37 ODR_CONFIG | 0x3D (`0b0111101`: ODR 50 Hz, normal mode per driver comment) | BMP581:133 |
| 10 | — | register 50 Hz callback | 20 ms | BMP581:140 |

No calibration read: the driver converts raw counts directly (factory-compensated part): `temperature = (int32)(b2<<24|b1<<16|b0<<8)>>8 / 65536`, `pressure = (b5<<16|b4<<8|b3) / 64` (BMP581:174, :179).

Counts: **writes: 3 transactions, 2 distinct addresses** (0x37, 0x36). **Reads at probe+init: 3 distinct** (0x01 [x2 on SPI], 0x28, 0x27). **Checked: 2** (0x37→0x3D, 0x36→0x72). **Polled: none (no loop), but two status gates (0x28, 0x27) that must read as given or probe fails.**

### 6.2 Steady state

| item | value | file:line |
|---|---|---|
| callback | 50 Hz (20 ms) | BMP581:140 |
| read | **two** consecutive 6-byte reads from 0x1D (TEMP XLSB/LSB/MSB 0x1D-0x1F, PRESS XLSB/LSB/MSB 0x20-0x22); the two must be equal, else a third read must equal the second, else the sample is dropped | BMP581:146-168, :34-39 |
| invalid marker | all three bytes 0x7F means no data (temp: buf[0..2]; pressure: buf[3..5]) | BMP581:172, :177 |
| DRDY | not used; no data-ready flag consulted | BMP581:146-184 |
| check | `check_next_register()` each timer | BMP581:183 |

### 6.3 hwdef / probe-ext

```
SPIDEV bmp581     SPI4 DEVID1 BMP581_CS     MODE3 7.5*MHZ 12*MHZ      # CUAV-X25-MEGA/hwdef.dat:156
BARO BMP581 SPI:bmp581                                                # CUAV-X25-MEGA/hwdef.dat:309
BARO BMP581 I2C:0:0x46                                                # CORVON743V2/hwdef.dat:205
```
`BARO_PROBE_EXT` bit **13** (`PROBE_BMP581`, AP_Baro.h:269); I2C addresses 0x46 and 0x47 (BMP581.h:10-15; AP_Baro.cpp:808-811).

### 6.4 Model must answer beyond identity

0x01 = 0x50; **0x28 with bit 1 set, bit 2 clear**; **0x27 with bit 0x10 set**; 0x37 and 0x36 read-back-stable; 0x1D-0x22 returning a stable 6-byte block (not 0x7F x3) between consecutive reads.

---

## 7. DPS310 / DPS280 (`AP_Baro_DPS280`)

### 7.1 Identity

`DPS280_REG_PID 0x0D` must read **0x10** (`DPS280_WHOAMI`) for both DPS280 and DPS310 (DPS:37, :41, :171-176).

### 7.2 Probe + init

| step | op | register | value | file:line |
|---|---|---|---|---|
| 1 | — | SPI: `set_read_flag(0x80)`; `set_speed(HIGH)` | | DPS:159-163 |
| 2 | — | `set_chip_select(true); set_chip_select(false)` (CS toggle for a DPS310 boot state; the base `Device::set_chip_select` returns false and does nothing on I2C) | | DPS:168-169; Device.h:320 |
| 3 | R | 0x0D PID | 0x10 | DPS:171-176 |
| 4 | R (18 B) | 0x10 COEF | calibration c0,c1,c00,c10,c01,c11,c20,c21,c30 | DPS:99, :103-121 |
| 5 | R | 0x28 CSRC | `&0x80` = temp_source | DPS:124-127 |
| 6 | — | `setup_checked_registers(4, 20)` | | DPS:183 |
| 7 | W (checked) | 0x09 CREG | 0x0C | DPS:134 |
| 8 | W (checked) | 0x06 PCONF | 0x54 | DPS:135 |
| 9 | W (checked) | 0x07 TCONF | `0x54 \| temp_source` | DPS:136 |
| 10 | W | 0x08 MCONF | 0x07 (continuous temp+pressure) | DPS:137 |
| 11 | W (**DPS310 only**, `is_dps310`) | 0x0E, 0x0F, 0x62, 0x0E, 0x0F | 0xA5, 0x96, 0x02, 0x00, 0x00 (undocumented temperature fix) | DPS:139-148 |
| 12 | — | register 64 Hz callback | (1000/64)=15 ms → 15 000 us | DPS:198 |

Coefficient decode: `C0=(b0<<4)+(b1>>4)`, `C1=b2+((b1&0xF)<<8)`, `C00=(b4<<4)+(b3<<12)+(b5>>4)`, `C10=((b5&0xF)<<16)+b7+(b6<<8)`, 16-bit pairs for C01..C30, then sign extension (DPS:103-121).

Counts: **writes: 4 distinct (0x09, 0x06, 0x07, 0x08); DPS310 adds 3 more distinct (0x0E, 0x0F, 0x62) = 7**. **Reads at probe+init: 3 transactions** (0x0D, 0x10 x18 B, 0x28). **Checked registers: 3** (0x09, 0x06, 0x07) of 4. **No polled status at probe/init.** Hwdef form selects the variant: `BARO DPS310 I2C:...` calls `AP_Baro_DPS310::probe` (is_dps310 = true, workaround writes) (DPS:66-70); `BARO DPS280 ...` is rewritten to `probe_280` (is_dps310 = false) (hwdef.py:603; DPS280.h:24-26).

### 7.3 Steady state

| item | value | file:line |
|---|---|---|
| callback | 15 ms (66.7 Hz) | DPS:198 |
| ready | read 0x08 MCONF; **bit 4 (0x10) must be set**; if not, `err_count++`, and after more than 16 consecutive misses the driver writes 0x0C=0x09 (reset) and re-runs `set_config_registers` | DPS:257-265, :229-241 |
| data | 3 bytes from 0x00 (pressure, 24-bit big-endian two's complement), 3 bytes from 0x03 (temperature) | DPS:259-270 |
| compensation | `scaling_16 = 1/253952`; `T = C0*0.5 + C1*Tsc`; `P = C00 + Psc*(C10 + Psc*(C20 + Psc*C30)) + Tsc*C01 + Tsc*Psc*(C11 + Psc*C21)` | DPS:206-224 |
| plausibility | `pressure_ok()` jump filter (inactive by default, `BARO_FLTR_RNG` default 0, AP_Baro.cpp:65-66, AP_Baro_Backend.cpp:94-97); `|T| > 120` counts as error and, after more than 16, triggers the reset path | DPS:278, :233-242 |
| check | `check_next_register()` per `check_health`, read every 20th call | DPS:231 |
| DRDY | not used | grep |

### 7.4 hwdef / probe-ext

```
BARO DPS310 I2C:0:0x76               # FlywooF745/hwdef.dat:152
SPIDEV dps310      SPI2 DEVID3  BARO_CS           MODE3  5*MHZ  5*MHZ     # luminousbee5/hwdef.dat:179
BARO DPS280      SPI:dps310                                                # luminousbee5/hwdef.dat:237
```
`BARO_PROBE_EXT` bit **6** (`PROBE_DPS280`, AP_Baro.h:262) probes I2C 0x76 and 0x77, with `probe_280` (**no DPS310 workaround**) (AP_Baro.cpp:826-829; DPS280.h:10-15).

### 7.5 Model must answer beyond identity

0x0D = 0x10; 0x10..0x21 coefficient block (valid or compensation is garbage but probe passes); 0x28 bit 7 (temp source); 0x09/0x06/0x07 read-back-stable; 0x08 with bit 4 set; 0x00-0x05 data.

---

## 8. ICP-20100 (`AP_Baro_ICP201XX`)

I2C only (the driver never sets a read flag or speed; every access is a `dev->transfer(&reg,1,buf,len)`). **Every read and write is followed by a dummy read of register 0x00** (ICP:146-175).

### 8.1 Identity

`REG_DEVICE_ID 0x0C` read twice (ICP:102-103); must be **0x63** (`ICP201XX_ID`, ICP:35, :106). `REG_VERSION 0xD3` must be **0x00 or 0xB2**, else failure (ICP:104, :110).

### 8.2 Probe + init sequence

| step | op | register | value | file:line |
|---|---|---|---|---|
| 1 | R x2 | 0x0C DEVICE_ID | 0x63 | ICP:102-103, :106 |
| 2 | R | 0xD3 VERSION | 0x00 or 0xB2 | ICP:104, :110 |
| 3 | — | `delay(10)` ms | | ICP:114 |
| 4 | — | `soft_reset()` | | ICP:116 |
| 4a | R-poll | 0xCD DEVICE_STATUS | **bit 0 must become 1 — unbounded loop with 1 ms sleeps** (`mode_select`) | ICP:192-204, :68 |
| 4b | W | 0xC0 MODE_SELECT | 0x00 | ICP:206 (via :180) |
| 4c | — | `delay(2)` ms | | ICP:182 |
| 4d | R, W | 0xC4 FIFO_FILL: read `v`, write `v \| 0x80` (flush) | | ICP:185, :443-458 |
| 4e | W | 0xC3 FIFO_CONFIG | 0x00 | ICP:188 |
| 4f | W | 0xC2 INTERRUPT_MASK | 0xFF | ICP:189 |
| 5 | — | `boot_sequence()` → R 0xD3; **if 0xB2 return true immediately** | | ICP:286-302 |
| 5a | (only if VERSION==0x00) | R 0xBF OTP_STATUS2; if bit 0 set, return true | | ICP:305-312 |
| 5b | (only if VERSION==0x00 and 0xBF bit0 clear) | long OTP path: `mode_select(0x04)`, W 0xBE=0x1F, RMW 0xAC \|=0x03, RMW 0xBC toggle bit 7, W 0xAF=0x04, 0xB0=0x04, 0xB1=0x21, 0xB2=0x20, 0xAD=0x10, 0xAE=0x80, three OTP reads (W 0xB5=addr {0xF8,0xF9,0xFA}, W 0xB6=0x10, **poll 0xB9 until 0, unbounded**, R 0xB8), RMW 0x05/0x07/0x06, R 0xBF (ICP:375; the following `if (!ret)` at ICP:376 is inverted, so the boot-status write at ICP:378 runs only when that read fails), RMW 0xAC &= ~3, W 0xBE=0x00, `mode_select(0x00)` | | ICP:314-393 |
| 6 | — | `configure()`: value `0x48` = op_mode 2 <<5 (0x40) \| continuous 1<<3 (0x08); then `mode_select(0x48)` = poll 0xCD bit 0, W 0xC0=0x48 | | ICP:396-416; header ICP.h:50-78 |
| 7 | — | `wait_read()` | | ICP:126, :418-441 |
| 7a | loop | `delay(10)`; R 0xC4 `& 0x1F`; repeat while **>= 14** | | ICP:427-431 |
| 7b | W,R | flush (R 0xC4, W 0xC4 \| 0x80) | | ICP:433 |
| 7c | loop | `delay(10)`; R 0xC4 `& 0x1F`; **repeat while == 0 (unbounded)** | | ICP:436-440 |
| 8 | — | `set_retries(0)`; register; 12.5 ms callback | `CONVERSION_INTERVAL/2` = 25000/2 us | ICP:128, :137, :37 |

Counts (model that reports VERSION=0xB2, the short path): **distinct registers written: 4** (0xC0 twice with 0x00 and 0x48, 0xC4 flush x2, 0xC3, 0xC2). **Distinct registers read: 0x0C, 0xD3, 0xCD, 0xC4, plus the 0x00 dummy = 5**. **Checked registers: none.** **Polled status: YES, three unbounded waits** — 0xCD bit 0 (twice, in `mode_select`), 0xC4 count < 14 then >= 1 (`wait_read`); with VERSION=0x00 and 0xBF bit 0 clear, also 0xB9 == 0.

### 8.3 Steady state

| item | value | file:line |
|---|---|---|
| callback | 12.5 ms | ICP:137 |
| FIFO fill | R 0xC4 `& 0x1F`; `> 16` → flush and drop; `1..16` → read | ICP:250-256 |
| data | burst from 0xFA (`REG_FIFO_BASE`), `fifo_packets * 6` bytes: per packet 3 bytes pressure then 3 bytes temperature, each 20-bit (`byte2 & 0x0F`) LE, sign-extend at 0x80000 | ICP:256-275, :71 |
| compensation | `P_kPa = raw/2^17 * 40 + 70`; `T = raw/2^18 * 65 + 25`; averaged over the packets; `pressure *= 1000` | ICP:265, :273, :277-278 |
| stall recovery | if no data for `3 * 25000` us, flush | ICP:473-476 |
| DRDY | not used (interrupts masked at init, ICP:189) | grep |

### 8.4 hwdef / probe-ext

```
BARO ICP201XX I2C:0:0x63        # Aeromind6X/hwdef.dat:284
BARO ICP201XX I2C:0:0x64        # CUAV_GPS/hwdef.dat:88
```
**Not in the `BARO_PROBE_EXT` list**: `baroprobespec[]` has no ICP201XX entry (AP_Baro.cpp:791-857); it is hwdef-only. The I2C address is whatever the hwdef line gives (0x63 and 0x64 both appear).

### 8.5 Model must answer beyond identity

0x0C=0x63; 0xD3=0xB2 (skips the OTP boot) or 0x00 with 0xBF bit 0 = 1; **0xCD bit 0 = 1** (else hang); **0xC4 fill count (low 5 bits) must be < 14 before the flush and >= 1 within the wait after it** (else hang in `init()`); 0xC4 bit 7 write = flush; I2C dummy reads of 0x00 must not disturb state; FIFO data at 0xFA in 6-byte pairs.

---

## 9. MS5611 (`AP_Baro_MS5611` over `AP_Baro_MS56XX`)

No registers; command bytes. `set_retries(10)` during init, `set_retries(3)` afterwards (MS:131, :165). The driver never sets a read flag or speed on SPI (the device runs at SPEED_LOW; both are 20 MHz in `Durandal/hwdef.dat:232`).

### 9.1 Identity

There is no ID register. The identity check is the **PROM**: 8 words, all zero rejected, CRC-4 matched (MS:194-222).

### 9.2 Probe + init

| step | op | command / register | value | file:line |
|---|---|---|---|---|
| 1 | W | 0x1E RESET | | MS:135, :27 |
| 2 | — | `delay(4)` ms | | MS:136 |
| 3 | R x8 | PROM word w = `transfer(&(0xA0 + 2*w),1,val,2)`, i.e. commands 0xA0, 0xA2, ..., 0xAE, each followed by 2 bytes, big-endian | | MS:175-183, :205, :31 |
| 4 | — | reject if all 8 words are 0; `crc_read = prom[7] & 0xF`; `prom[7] &= 0xFF00`; require `crc_crc4(prom) == crc_read` | | MS:211-221; `AP_Math/crc.cpp:30-53` |
| 5 | — | `_cal_reg.c1..c6 = prom[1..6]` (`prom[0]` and the rest are not used) | | MS:146-151 |
| 6 | W | 0x54 (`ADDR_CMD_CONVERT_D2_OSR1024`, temperature convert) | | MS:154, :54, :43 |
| 7 | — | register 100 Hz callback | 10 ms | MS:170 |

Counts: **2 command writes** (0x1E, 0x54); **8 PROM reads**; no registers, no checked regs, **no polled status**. The model must supply a PROM whose CRC-4 is valid (algorithm `crc_crc4`, AP_Math/crc.cpp:30-53: 16-byte loop, polynomial step `0x3000`, returns `(n_rem>>12)&0xF`).

### 9.3 Steady state

| item | value | file:line |
|---|---|---|
| callback | 100 Hz (10 ms) | MS:170 |
| state machine | state 0 → temperature (D2), states 1-4 → pressure (D1); each callback: read ADC (`transfer(&0x00,1,val,3)`, 24-bit BE), then issue next convert command (0x54 for D2, 0x44 `ADDR_CMD_CONVERT_D1_OSR1024` for D1) | MS:185-192, :265-313, :53-54 |
| failure | ADC 0 or 0xFFFFFF discards the next sample too | MS:275-300 |
| compensation | `dT = D2 - (C5<<8)`; `TEMP = dT*C6/2^23 + 2000`; `OFF = C2*65536 + C4*dT/128`; `SENS = C1*32768 + C3*dT/256`; below 20 degC second-order terms; `P = (D1*SENS/2^21 - OFF)/2^15` | MS:357-394 |
| plausibility | `pressure_ok(adc_val)` jump filter applied to the raw D1 count; **inactive by default** (`pressure_ok` returns true when `BARO_FLTR_RNG <= 0`, and `HAL_BARO_FILTER_DEFAULT` is 0) | MS:307; AP_Baro_Backend.cpp:87-97; AP_Baro.cpp:65-66, :179 |
| conversion time | no delay: the driver just reads whatever the ADC holds at the next 10 ms tick | MS:265-313 |
| DRDY | not applicable | — |

### 9.4 hwdef / probe-ext

```
SPIDEV ms5611         SPI4 DEVID1  MS5611_CS    MODE3 20*MHZ 20*MHZ     # Durandal/hwdef.dat:232
BARO MS5611 SPI:ms5611                                                   # Durandal/hwdef.dat:305
BARO MS5611    I2C:0:0x77                                                # FlyingMoonF427/hwdef.dat:101
```
`BARO_PROBE_EXT` bit **2** (`PROBE_MS5611`, AP_Baro.h:258) probes I2C 0x77 and 0x76 (AP_Baro.cpp:812-815; MS5611.h:11-16).

### 9.5 Model must answer beyond identity

Command decoder: 0x1E reset, 0xA0-0xAE PROM (with valid CRC-4 in `prom[7]` low nibble), 0x44/0x54 convert, 0x00 ADC read returning 3 bytes (not 0, not 0xFFFFFF, and, only if `BARO_FLTR_RNG` > 0 (default 0), slowly varying to survive the jump filter).

---

## 10. Summary table

| part | driver | identity check | init writes (distinct addrs) | FIFO-only? | DRDY used? | config blob | polled status at probe/init |
|---|---|---|---|---|---|---|---|
| ICM-42688-P | Invensensev3 (IV3:214, :259) | R 0x75 == 0x47 (or 0xDB) | 16 distinct (32 write transactions; 0 at probe, all in `start()`), bank 1/2 via 0x76; 1 checked reg (0x4D); 3 reads | FIFO only, 16-B packets, count @0x2E, data @0x30, 1 kHz (2 kHz on H7 inst 0) | no | none | none |
| ICM-45686 | Invensensev3, `ICM45686` | R 0x75 (any non-match), R 0x72 == 0xE9 | 8 direct distinct (12 transactions: 3 at probe, 9 at start) + 4 IREG-indirect (0xA268, 0xA258, 0xA4A6, 0xA57B) via 0x7C-0x7E; 3 checked regs; 3+ reads | FIFO only, 16-B packets, count @0x12, data @0x14, 800 Hz (1600 H7 inst 0) | no | none | **0x19 bit 0x80 after soft reset (hard)**; 0x7F bit 0 around IREG (bounded) |
| BMI088 | BMI088 (B088:78, :286) | accel 0x00 == 0x1E/0x1F (read twice); gyro 0x00 == 0x0F | accel 6 (written only if read differs, verify-retry 8x), gyro 6 (1 reset + 5 checked) = 12; 5 checked regs | FIFO only; accel 1600 Hz (0x24/0x26), gyro 2000 Hz (0x0E/0x3F) | no | none | none (accel config verified by read-back; non-fatal if it fails) |
| BMI270 | BMI270 (B270:159, :509) | 3rd read of 0x00 == 0x24 (up to 5 tries) | 12 distinct (4 at probe incl. blob stream to 0x5E; 8 at start); verify-readback on every write; 0 checked regs | FIFO only, 1600 Hz, `0x8C` combined frames | no | **328-byte** `maximum_fifo_config_file` at B270:105-124, streamed to 0x5E (329-B transfer) | **0x21 bit 0 == 1** after INIT_CTRL=1 + 20 ms (hard) |
| BMP390 | BMP388 (BMP388:63, :73) | R 0x00 == 0x60 (BMP390) / 0x50 (BMP388) | 2 distinct (0x7E=0xB6, 0x1B=0x33); 1 checked; reads 0x00, 0x36 (16 B), 0x31 (5 B) | regs: 7 B @0x03 each 20 ms | no | none (cal read from NVM regs) | none; **runtime needs status bits 0x20/0x40 at 0x03** |
| BMP581 | BMP581 (BMP581:63, :73) | R 0x01 == 0x50 (SPI: read twice) | 2 distinct (0x37 x2, 0x36), 3 writes; 2 checked; no cal | regs: 2 x 6 B @0x1D each 20 ms | no | none | **0x28 bit1=1/bit2=0 and 0x27 bit 0x10=1 (hard gates)** |
| DPS310 | DPS280 (`AP_Baro_DPS310`, DPS:66, :151) | R 0x0D == 0x10 | 4 distinct (+3 for DPS310 = 7); 3 checked; reads 0x0D, 0x10 (18 B), 0x28 | regs: MCONF bit4 gate, 3+3 B each 15 ms | no | none (cal read from regs) | none at init; runtime needs 0x08 bit 4 |
| ICP-20100 | ICP201XX (ICP:82, :92) | R 0x0C == 0x63 (x2) and R 0xD3 in {0x00,0xB2} | 4 distinct (0xC0, 0xC4, 0xC3, 0xC2) on the 0xB2 path; ~20 more on the 0x00/OTP path; 0 checked | FIFO @0xFA, 6 B/packet, 12.5 ms | no | none (OTP trim only on VERSION=0x00 path) | **0xCD bit 0 (unbounded); 0xC4 count <14 then >=1 (unbounded); 0xB9==0 and 0xBF on OTP path** |
| MS5611 | MS56XX / MS5611 (MS:122, :77) | PROM CRC-4 (no ID register), not all-zero | commands only: 0x1E reset, 0x54 convert; 8 PROM reads (0xA0-0xAE) | command/ADC state machine, 10 ms | n/a | PROM (16 B) is the calibration; must carry valid CRC-4 | none |

---

## 11. Things that would force a model to do more than identity + init registers + data/FIFO

0. **Brief vs source.** The ICM-42688 driver uses banks 1 and 2 only (`register_write_bank`/`register_read_bank` calls at IV3:853-862, :884-885 are banks 1 and 2; bank arguments 0 at :1131/:1141 are ICM42670-only `INV3REG_BANK_MREG1`), not bank 4; the ICM-45686 uses IREG indirect, not banks. The BMI270 blob is 328 bytes (`maximum_fifo_config_file`), not the 8 KB Bosch file.

1. **BMI270 config blob and INTERNAL_STATUS.** The model must accept a 329-byte burst to 0x5E and then present `INTERNAL_STATUS(0x21) & 1` when read 20 ms after `INIT_CTRL=1`. Without it probe returns false after 5 tries (B270:561-577).
2. **BMI270 write-verify on a command register.** `write_register(CMD=0xB6/0xB0)` reads 0x7E back and retries up to 8 times (B270:256-266); on real silicon it reads 0, so 8 identical resets happen. The model should either echo CMD or tolerate 8 repeated soft resets.
3. **ICM-45686 reset-done and the IREG window.** After the `0x7F=0x02` write, 0x19 bit 0x80 must be set (single read, hard fail, IV3:1154). IREG access needs burst writes across 0x7C-0x7E with a latch on 0x7E; reads of 0x7F bit 0 are soft (10 tries) but the indirect values returned drive read-modify-write.
4. **ICP-20100 has three unbounded `do/while` waits and no timeout** (ICP:196-204 `mode_select` on 0xCD bit 0, ICP:436-440 `wait_read` until FIFO fill != 0, ICP:223-231 OTP status). A model that does not set 0xCD bit 0 or never produces FIFO packets after the flush hangs the boot task. It also needs a time- or flush-aware FIFO fill (must be < 14 before the flush and >= 1 after) and tolerates a dummy 0x00 read after every transaction.
5. **BMP581 POR status gates** (0x28 bit1 set / bit2 clear, 0x27 bit 0x10 set) are hard fails (BMP581:111-122).
6. **MS5611 requires a valid PROM CRC-4** computed by `crc_crc4` (MS:216-221; AP_Math/crc.cpp:30-53), and an ADC value that is neither 0 nor 0xFFFFFF, and (only when `BARO_FLTR_RNG` > 0, default 0: AP_Baro.cpp:65-66, AP_Baro_Backend.cpp:94-97) does not jump.
7. **BMI088 has two SPI devices with different bus quirks.** Accel SPI reads return a throw-away first byte; the accel ID is read twice; accel registers need read-back stability (write verify, 8 tries) (B088:122-154). Gyro writes are tested for transfer success only (B088:257-277).
8. **Checked-register readback in the steady-state callback** (IV3:632-641; B088:441-446; baro `timer()` calls). A model that returns a different value than last written for any checked register causes silent rewrites plus error counters, not failure. The tables: ICM42688 {0x4D}; ICM45686 {0x20, 0x21, 0x1D}; BMI088 gyro {0x0F, 0x10, 0x11, 0x13, 0x3E}; BMP388 {0x1B}; BMP581 {0x37, 0x36}; DPS {0x09, 0x06, 0x07}.
9. **Frame/packet validity**: IV3 resets the FIFO on a bad header byte (`(h & 0xFC) != 0x68`, IV3:458-461); BMI270 resets on header 0x80 or ERR_REG bit 6 (B270:379-382, :451-454). A model that streams the wrong header looks like an endless reset loop, not a probe failure.
10. **Rate/feature dependence on parameters and hwdef**: ICM42688 and ICM45686 callback rate, ODR value and AAF constants change with `INS_FAST_SAMPLE`/`INS_GYRO_RATE` (H7 default 1) and `HAL_INS_HIGHRES_SAMPLE` (changes packet to 20 bytes, FIFO_CONFIG1 to 0x17 / FIFO_CONFIG3 to 0x0E-0x0F). A model that supports only the base case (1 kHz/800 Hz, 16-byte packets) must be paired with a hwdef that does not define `HAL_INS_HIGHRES_SAMPLE` for that instance.
11. **ICM-45686 FIFO count endianness is assumed, not set.** The driver never writes INTF_CONFIG0 (0x2C) for the 456 yet reads 0x12/0x13 as a host-order `uint16` of packets (IV3:565, :583). The model should return the packet count little-endian (0x12 = low byte).
12. **BARO_PROBE_EXT does not cover ICP201XX at all**, and for DPS only the `probe_280` (no DPS310 temperature workaround) form is in the ext table (AP_Baro.cpp:826-829). The 4220 board should use an explicit `BARO` hwdef line for each part.
