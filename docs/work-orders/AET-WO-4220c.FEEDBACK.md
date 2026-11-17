# AET-WO-4220c — FEEDBACK

Labels: **[read at source]** from a file and line named beside it; **[rendered]** a datasheet table confirmed on the rendered page image, not only its extracted text; **[measured]** by a run named beside it; **[inferred]** from reasoning, never from a source.

**Sources.**
- **DS-000347 Revision 1.9:** `icm42688p_ds.pdf`, sha256 `9c6b57b0a2e45cf0251dd2423878573bd4da46e0cc2faa93dc558c28af51300a`, the file `AET-WO-4220` read. Page numbers are the PDF's, which equal its printed "Page N of 110".
- **ArduPilot `Copter-4.7.2`, commit `fb773567`:** the image's pin (`container/ARDUPILOT-SOURCES.md`).
  - `IV3` = `libraries/AP_InertialSensor/AP_InertialSensor_Invensensev3.cpp`;
  - `INS` = `libraries/AP_InertialSensor/AP_InertialSensor.cpp`.
- **The model:** `models/renode/ICM42688P.cs` at `main` `7d52d29`.

## 0. Status: STOPPED at §1.1, as the work order requires

- **Measured and read; nothing built.** No line of the model or of any test is changed on this branch.
- **The proposal (§4):** packets 1–3 (16-bit), `STOP-on-FULL`, the count in records or bytes in either byte order, `FIFO_FLUSH`, the ODR timestamp at 1 µs resolution, and `FIFO_FULL_INT` as a status bit. The 20-bit packet, `Stream-to-FIFO`, FSYNC, the watermark, the FIFO interrupts on a pin and `FIFO_LOST_PKT` are left out. A firmware that selects one of them gets a warning from the model, never a guessed value.
- **ArduPilot needs four behaviours the datasheet does not state (§3).** That is §7's second condition, so each is cited for the ruling with a recommended reading:
  - **B1:** whether `SENSOR_DATA_ENDIAN` applies to FIFO packets;
  - **B2:** which byte `0x2E` holds when the count is little-endian;
  - **B3:** whether a burst read at `FIFO_DATA` (`0x30`) holds its address;
  - **B4:** what a chunked read does with `FIFO_RESUME_PARTIAL_RD = 0`.
- **The packet ArduPilot gets for the twin's stimulus is worked byte by byte (§5).**
- **Moved goldens (§6):** none of the generated firmware's verdicts move [read at source; measured in the build]. Two moves outside the fence are forced: the model's sha256 pin (`aethelgard-twin` `model.rs` L71) and three line citations in `aethelgard-cli` `ledger.rs` (L1337, L1342, L1347).

## 1. The datasheet's FIFO [read at source; tables rendered]

| Fact | Where |
|---|---|
| The FIFO holds 2048 bytes; a counter keeps the bytes of valid data | p.37 §6 |
| Four packets: 1 = header + accel (6) + temperature (1), 8 bytes; 2 = the same with gyro; 3 = header + accel + gyro + temperature (1) + timestamp (2), 16 bytes; 4 = the 20-bit form, 20 bytes | p.37 Figure 10; p.38–39 tables [rendered] |
| Every 16-bit field in the packet tables is high byte first (`[15:8]` at the lower offset) | p.38 [rendered] |
| Header: bit 7 `HEADER_MSG` (1: FIFO is empty); 6 `HEADER_ACCEL`; 5 `HEADER_GYRO`; 4 `HEADER_20`; 3:2 `HEADER_TIMESTAMP_FSYNC` (10: ODR timestamp; 11: FSYNC time, only if `FIFO_TMST_FSYNC_EN`); 1 `HEADER_ODR_ACCEL` and 0 `HEADER_ODR_GYRO` (1: the ODR differs from the previous packet with that sensor) | p.39 §6.2 [rendered] |
| Enable bits to header and size: `HIRES 0, ACCEL 1, GYRO 1, TMST_FSYNC 0` → `0110_10xx`, 16 bytes; `ACCEL 1, GYRO 0` → `0100_00xx`, 8 bytes; `ACCEL 0, GYRO 1` → `0010_00xx`, 8 bytes; neither → "No FIFO writes"; `HIRES 1` → 20 bytes | p.41 [rendered] |
| Storage: 2048 bytes, less one packet reserved against reading a packet being written; a 2-packet read cache holds packets only during serial traffic, so capacity is "non-deterministic", up to 2048 bytes + 1 packet | p.40 §6.3, Figure 11 |
| The `FIFO_CONFIG1` bits "may change while the FIFO is being filled without corruption of the FIFO" | p.40 §6.4 |
| `FIFO_CONFIG` (`0x16`, reset `0x00`) bits 7:6 `FIFO_MODE`: 00 Bypass (default), 01 Stream-to-FIFO, 10 and 11 STOP-on-FULL. No further text defines the modes | p.65 §14.4 |
| `FIFO_COUNTH` (`0x2E`) `FIFO_COUNT[15:8]`; `FIFO_COUNTL` (`0x2F`) `[7:0]`. The count is in records or bytes per `FIFO_COUNT_REC`. "Reading this byte latches the data for both"; "Must read FIFO_COUNTH to latch new data" | p.69 §14.22, p.70 §14.23 |
| `FIFO_DATA` (`0x30`), reset `0xFF`, "FIFO data port" | p.70 §14.24 |
| `INTF_CONFIG0` (`0x4C`, reset `0x30`): bit 7 `FIFO_HOLD_LAST_DATA_EN`; bit 6 `FIFO_COUNT_REC` (0 bytes, 1 records); bit 5 `FIFO_COUNT_ENDIAN` (0 little, 1 big, the default); bit 4 `SENSOR_DATA_ENDIAN` ("Sensor data is reported in Little Endian format" / Big, the default) | p.74–75 §14.34 [rendered] |
| Invalid samples: (a) from reset to a sensor's first sample; (b) a sensor disabled; (c) different ODRs: "the sensor with lower ODR will generate invalid samples when it has no new data" | p.75 [rendered] |
| A 16-bit FIFO packet's values: with `HOLD_LAST` 0, valid samples in {−32766 … +32767} and an invalid sample is −32768; with `HOLD_LAST` 1, valid samples in {−32768 … +32767} and an invalid sample is "Copy last valid sample" | p.58 §12.8 table [rendered]; p.74 |
| Timestamp interval in the FIFO, no RTC: at `TMST_RES` 0 (1 µs) scale by 32/30; "when ODR = 1kHz … the value in FIFO toggles between 937 and 938". At `TMST_RES` 1 (16 µs), 58 and 59 | p.58 §12.7 [rendered] |
| `TMST_CONFIG` (`0x54`, reset `0x23`): `TMST_EN` 1, `TMST_FSYNC_EN` 1, `TMST_DELTA_EN` 0, `TMST_RES` 0 | p.81 §14.42 |
| `SIGNAL_PATH_RESET` (`0x4B`, W/C) bit 1 `FIFO_FLUSH`: "When set to 1, FIFO will get flushed" | p.73 §14.33 |
| `FIFO_CONFIG1` (`0x5F`): bit 6 `FIFO_RESUME_PARTIAL_RD` ("0: Partial FIFO read disabled, requires re-reading of the entire FIFO / 1: FIFO read can be partial, and resume from last read point"); 5 `FIFO_WM_GT_TH`; 4 `FIFO_HIRES_EN`; 3 `FIFO_TMST_FSYNC_EN`; 2 `FIFO_TEMP_EN`; 1 `FIFO_GYRO_EN`; 0 `FIFO_ACCEL_EN` | p.83 §14.45 |
| `FIFO_CONFIG2`/`3`: `FIFO_WM[11:0]`, "Interrupt only fires once", "Do not set FIFO_WM to value 0" | p.83 §14.46–47 |
| `INT_STATUS` (`0x2D`): bit 2 `FIFO_THS_INT` (FIFO reaches its threshold), bit 1 `FIFO_FULL_INT` ("when the FIFO buffer is full"), each cleared by reading the register | p.69 §14.21 |
| `INT_CONFIG0` bits 3:0: the FIFO interrupts' clear options, two of which clear on a FIFO data read | p.84 |
| FIFO temperature: "an 8-bit quantity, FIFO_TEMP_DATA", °C = `FIFO_TEMP_DATA / 2.07 + 25` | p.25 §4.13 |
| `FIFO_LOST_PKT0`/`1` (`0x6C`/`0x6D`): **the datasheet contradicts itself.** The register map gives `0x6C` `[15:8]` and `0x6D` `[7:0]`; the register detail gives `0x6C` "Low byte" `[7:0]` and `0x6D` "High byte" `[15:8]` | p.61 vs p.87 [both rendered] |
| SPI: "Supports Single or Burst Reads and Single Writes"; nothing on the address during a burst. The I2C section says the address auto-increments on a burst write; nothing names `FIFO_DATA` | p.52 §9.6 item 7; p.50 |
| `ACCEL_ODR`/`GYRO_ODR` code `0101` = 2 kHz, `0110` = 1 kHz (default) | p.78, p.77 |
| Register changes during operation: only ODR, FSR and mode; anything else with the sensors off | p.59 §12.9 |

## 2. What ArduPilot sets and reads, in order [read at source; measured where named]

At `Copter-4.7.2` the ICM-42688-P takes the `Invensensev3` default path. Fast sampling is on by default on an H7: `INS_FAST_SAMPLE` defaults to 1 (`INS` L55–56, L553), and `INS_GYRO_RATE` to 1 on `STM32H7` (`INS` L87–91, L600), so the rate is 2 kHz (`IV3` L779, L787–788). Hi-res is off: `HAL_INS_HIGHRES_SAMPLE` defaults to 0 (`AP_InertialSensor_Backend.h` L32–33). `4400e`'s twin run measured the writes below (`4400e` FEEDBACK at `ed936af7`, L154).

| # | Transaction | Value | What it selects | Source |
|---|---|---|---|---|
| 1 | read `0x75` | `0x47` | identity | `IV3` L1005; measured (`4400e` P5) |
| 2 | write `FIFO_CONFIG` `0x16` | `0x80` | STOP-on-FULL | `IV3` L247 |
| 3 | write `FIFO_CONFIG1` `0x5F` | `0x07` | accel, gyro, temperature; partial reads off; no hi-res; no FSYNC | `IV3` L249, L271 |
| 4 | write `INTF_CONFIG0` `0x4C` | `0xC0` | hold-last on; count in records; count and sensor data little-endian | `IV3` L251 |
| 5 | write `SIGNAL_PATH_RESET` `0x4B` | `0x02` | `FIFO_FLUSH` | `IV3` L252 |
| 6 | write `PWR_MGMT0` `0x4E` | `0x00` | both sensors off | `IV3` L844 |
| 7–8 | write `0x4F`, `0x50` | `0x05` each | 2 kHz, ±2000 dps, ±16 g | `IV3` L847, L849; measured `0x05` (`4400e` L154) |
| 9 | bank 1 `0x0B`–`0x0E`, bank 2 `0x03`–`0x05` via `0x76` | AAF values | filters (modelled as storage) | `IV3` L853–862 |
| 10 | read `0x4D`, write `(v & 0x3F) \| 0x40` | `0x91` → `0x51`, checked | AFSR off | `IV3` L877, L890 |
| 11 | write `PWR_MGMT0` | `0x0F` | gyro and accel low-noise | `IV3` L901 |
| 12 | every 500 µs: read `0x2E`, 2 bytes | the count, little-endian `uint16` | records waiting | `IV3` L395, L583 |
| 13 | if the count is 0, stop | — | — | `IV3` L587–590 |
| 14 | else one burst: `0x30 \| 0x80`, then `n × 16` bytes, `n = min(count, 24)`, repeated until the count is read | packets | — | `IV3` L599–611; `INV3_FIFO_BUFFER_LEN` 24 with the fast-sample window, which an H7 Copter build has (`IV3` L185–189; `chibios.h` L150–158) |
| 15 | each packet: `(header & 0xFC) == 0x68`, else `fifo_reset()` (#2–5) | — | packet 3 with an ODR timestamp; bits 1:0 ignored | `IV3` L458, L616–629 |
| 16 | decode: `FIFOData` = `uint8 header; int16 accel[3]; int16 gyro[3]; int8 temperature; uint16 timestamp`, packed, little-endian | — | the timestamp is never read; temperature × 1/2.07 + 25 | `IV3` L150–159, L276, L473 |

- **No interrupt and no status register:** the driver polls on a periodic callback. `INV3REG_INT_STATUS` is defined (`IV3` L63) and never read.
- **`4220`'s replay of this sequence** (`models_tier2.rs` `ardupilot_s_probe_and_init_pass_against_our_models`) uses its read's "base case", ODR `0x06` and the 1 kHz filter values (`s2-ardupilot.md` L78). Its own table says the H7 default is 2 kHz (L113), and `4400e` measured `0x05`. Proposed: the build moves the replay to the measured writes (§4.4).

## 3. What ArduPilot needs that the datasheet does not state (§7)

Each item is cited, given a recommended reading, and named with the source that would settle it. **None is decided here.**

- **B1. Byte order inside a FIFO packet.**
  - **The text:** the packet tables put `[15:8]` first (p.38). `SENSOR_DATA_ENDIAN` = 0 is "Sensor data is reported in Little Endian format" (p.75) and does not say whether that covers the FIFO.
  - **Why it matters:** ArduPilot sets it to 0 and decodes the packet as little-endian `int16`s (`IV3` L150–159: "really nice that this sensor has an option to request little-endian data"). Read big-endian, 1 g at ±16 g would decode as 8/2048 g.
  - **Recommended reading:** the bit swaps every 16-bit field of a packet: accel, gyro and timestamp (p.39–41 give no other 16-bit field in packets 1–3).
  - **Settles it:** TDK's own driver for the part (its FIFO decode under each `SENSOR_DATA_ENDIAN`) or a silicon read.
- **B2. Which byte `0x2E` holds when the count is little-endian.**
  - **The text:** `0x2E` is named `FIFO_COUNT[15:8]` (p.69); `FIFO_COUNT_ENDIAN` = 0 reports the count "in Little Endian format" (p.75).
  - **Why it matters:** ArduPilot reads `0x2E`–`0x2F` into a little-endian `uint16` (`IV3` L583).
  - **Recommended reading:** the bit swaps the pair, so `0x2E` holds `[7:0]` and `0x2F` holds `[15:8]`. That is the same swap the model already makes for the data registers under `SENSOR_DATA_ENDIAN` (`ICM42688P.cs` L186–194, ruled in `4220`). The latch is on reading `0x2E`, whichever byte it holds (p.69).
  - **Settles it:** as B1.
- **B3. A burst read at `FIFO_DATA`.**
  - **The text:** "Supports Single or Burst Reads" (p.52); `FIFO_DATA` is a "FIFO data port" (p.70); the part "allow[s] the system processor to burst read sensor data" from the FIFO (p.9). Nothing states the address during an SPI burst.
  - **Why it matters:** ArduPilot reads up to 384 bytes from `0x30` in one transaction (`IV3` L605–608). The model today increments the address on every byte (`ICM42688P.cs` L104), so byte 2 of such a burst would come from `0x31`.
  - **Recommended reading:** a burst that reaches `0x30` stays there, and every byte after it is the next FIFO byte. A burst from below `0x30` increments up to it (ArduPilot's count read stops at `0x2F`).
  - **Settles it:** TDK's driver, which reads the FIFO in one burst, or silicon.
- **B4. Reading the FIFO in chunks with `FIFO_RESUME_PARTIAL_RD` = 0.**
  - **The text:** "Partial FIFO read disabled, requires re-reading of the entire FIFO" (p.83). It does not define "partial": fewer records than the count, or a packet cut mid-way.
  - **Why it matters:** ArduPilot leaves the bit at 0 (`0x07`, `IV3` L249) and reads at most 24 records per transaction (`IV3` L599). After a stall longer than 12 ms it reads in chunks. If "re-reading of the entire FIFO" meant chunks re-presented old records, ArduPilot on silicon would see duplicates [inferred: it runs on silicon at every boot of such a board].
  - **Recommended reading:** a packet read in full is consumed, whatever the bit. A transaction that ends inside a packet with the bit at 0 logs a warning ("a partial packet read with FIFO_RESUME_PARTIAL_RD = 0 is not modelled"), and the next read resumes at the next byte, which is what bit 6 = 1 states. ArduPilot never ends inside a packet (`n × 16`), so its path logs nothing.
  - **Settles it:** TDK's driver or an application note on partial reads.

The behaviours below are also left unstated, but ArduPilot's path does not depend on them. Each gets a recommended reading, for the ruling:
- **B5. A gyro slot before the gyro's first sample.**
  - **The text:** with `HOLD_LAST` 1, an invalid sample is "Copy last valid sample" (p.58), and there is none yet.
  - **Recommended reading:** 0, as p.59 gives the sense registers from reset to the first sample with `HOLD_LAST` 1. With `HOLD_LAST` 0, −32768 (p.58, stated).
  - ArduPilot reads these packets for the 20 ms between the accelerometer's start-up (10 ms) and the gyro's (30 ms), and uses their gyro value.
- **B6. The timestamp's epoch.** p.58 fixes the step. Nothing fixes its zero. Recommended reading: the model's reset. ArduPilot never reads the field.
- **B7. `HEADER_ODR_*` on the first packet after a flush.** Recommended reading: 0, since there is no previous packet to differ from. ArduPilot masks bits 1:0.
- **B8. Reading an empty FIFO.** `HEADER_MSG` = 1 means "FIFO is empty" (p.39), and `FIFO_DATA` resets to `0xFF` (p.70). Recommended reading: every byte read with nothing waiting is `0xFF`. The first such byte is a header with `HEADER_MSG` = 1, and the rest are not stated.
- **B9. Capacity.** p.40 makes it non-deterministic. Recommended reading: the lower bound, 2048 bytes less one packet (Figure 11, "no serial interface operation"), so 127 packets of 16 bytes or 255 of 8. It is deterministic, and a twin must be.
- **B10. A temperature outside the 8-bit range.** −40 °C gives −134.6 counts (p.25, p.14's range). Recommended reading: saturate at −128 and +127. Neither ArduPilot nor the stimulus reaches it.

## 4. The proposal

### 4.1 Modelled, each cited

| Behaviour | Rule | Source |
|---|---|---|
| Packets 1, 2, 3 | layout by p.38; header by p.41's rows for `HIRES` 0, `TMST_FSYNC` 0; size 8 or 16 | p.37–41 |
| Packet clock | a packet is written at each instant a FIFO-enabled sensor samples; accel and gyro at the same instant share one packet. A slot whose sensor has no new sample holds its invalid value: `HOLD_LAST` 1 → the last valid sample (0 before any, B5); `HOLD_LAST` 0 → −32768 | p.75 (c); p.58 table |
| Values | the 16-bit words the data registers already hold (the same `Raw16`, the same −32766 floor with `HOLD_LAST` 0, as p.74 says a 16-bit packet is "same as Sense Registers"); temperature `round((T − 25) × 2.07)`, half away from zero as the model rounds (`4210` D-A8), saturating (B10) | p.74; p.25 |
| Byte order | `SENSOR_DATA_ENDIAN` swaps each 16-bit field (B1) | p.75 |
| Timestamp (packet 3) | header `10`; value `floor(t_µs × 30/32) mod 2^16` from the model's reset (B6). p.58's own figures are the oracle: steps of 937 and 938 at 1 kHz, 468 and 469 at 2 kHz | p.58 §12.7; p.81 reset `0x23` |
| `HEADER_ODR_*` | 1 when that sensor's ODR code differs from the one in its previous packet since the last flush or reset (B7) | p.39 |
| `FIFO_MODE` | Bypass: no writes, count 0 [inferred from "Bypass", p.65]. STOP-on-FULL (`10`, `11`): writes until a packet would pass capacity, then drops packets | p.65; p.40 |
| Capacity | 2048 bytes less one packet (B9) | p.40 Figure 11 |
| `FIFO_COUNT` | records (whole packets not yet fully read) or bytes (unread bytes), per `FIFO_COUNT_REC`; byte order per `FIFO_COUNT_ENDIAN` (B2); latched by a read of `0x2E`, so `0x2F` reads the latched value | p.69–70; p.75 |
| `FIFO_DATA` | each byte read is the next FIFO byte; a burst holds at `0x30` (B3); past the count, `0xFF` (B8); B4's rule for a packet cut short | p.70; p.52 |
| `FIFO_FLUSH` | empties the FIFO and the ODR-change memory; the register stays write-to-clear | p.73 |
| Soft reset and `Reset()` | empty the FIFO with everything else | p.64 |
| `FIFO_FULL_INT` | `INT_STATUS` bit 1 set when the FIFO becomes full, cleared by the status read the model already makes. Status only: no pin | p.69 |
| Stimulus and time | unchanged: the sampling timer, the ODR tables and the start-up times that feed the data registers also clock the FIFO, in virtual nanoseconds (`4220` PB2, held) | `ICM42688P.cs` L49, L304–311, L391–478 |

### 4.2 Left out, and how the model says so

Each is outside the ruled set. A write that selects it logs a `Warning` naming it, as unmodelled registers do today (`ICM42688P.cs` L174, L242). Packets are not written in that configuration and no value is guessed.

| Left out | Selected by | Why left out |
|---|---|---|
| Packet 4, 20-bit (`FIFO_HIRES_EN`) | `0x5F` bit 4 | ArduPilot's hi-res is off by default (§2); the 3-nibble extension adds width and is unused |
| FSYNC in the FIFO | `0x5F` bit 3 | FSYNC is not modelled (`ICM42688P.cs` L8) |
| `FIFO_TEMP_EN` = 0 with a sensor enabled | `0x5F` bit 2 | p.41's sizes do not depend on it, yet p.83 calls it "Enable temperature sensor packets": unstated what the packet then holds |
| Stream-to-FIFO | `0x16` = `01` | the mode is named, never defined (p.65) |
| `FIFO_THS_INT`, the watermark, `FIFO_WM_GT_TH` | `0x5F` bit 5; a non-zero `FIFO_WM` | "fires once" without a re-arm rule (p.83); unused by ArduPilot |
| FIFO interrupts on `INT1`/`INT2` | `INT_SOURCE0`/`3` bits 2:1 | the WO excludes interrupts that ArduPilot does not need (§2) |
| FIFO interrupt clear options `10`, `11` | `INT_CONFIG0` bits 3:0 | they belong to the interrupts above |
| `TMST_DELTA_EN`, `TMST_RES` = 1, `TMST_EN` = 0 | `0x54` | kept small: only the reset configuration's timestamp is proposed (p.58 gives `TMST_RES` 1's step too, if wanted) |
| `FIFO_LOST_PKT` | read of `0x6C`/`0x6D` | the datasheet contradicts itself (p.61 vs p.87); they stay unmodelled registers, as today |
| The FIFO in banks 1–4 | — | none exists |

### 4.3 Unchanged

- The data registers, `INT_STATUS`'s other bits and `INT1`'s data-ready behaviour.
- Every reset value, and the log contract: one `Noisy` line per transaction, so a 384-byte FIFO burst is one line.
- `SIGNAL_PATH_RESET`'s other bits stay ignored, as today (`ICM42688P.cs` L267).

### 4.4 Tests, after the ruling (`models_tier2.rs`, monitor-driven, as `4220` set the method)

1. **The count grows at the ODR in virtual time.** Configure as ArduPilot does (§2 #2–11). After `RunFor` T, read the count. The accelerometer starts at 10 ms (`ICM42688P.cs` L570), so the expected count is `floor((T − 10 ms) / 500 µs) + 1`, at several T, and also at 1 kHz.
2. **Packets byte for byte:** §5's two packets, for the twin's stimulus; packets 1 and 2; and the timestamp's step against p.58's 937/938 at 1 kHz.
3. **Reading past the count:** `0xFF`, and the count stays 0.
4. **Flush, STOP-on-FULL at 127 packets, and `FIFO_FULL_INT`** set and cleared.
5. **The count's byte order and latch** under each `FIFO_COUNT_ENDIAN`.
6. **The data registers do not move:** the same reads of `0x1D` with the FIFO on and off.
7. **Each left-out selection warns, and ArduPilot's path does not.**
8. **The replay** (`ardupilot_s_probe_and_init_pass_against_our_models`) moves to the measured writes (`0x05`, the 2 kHz filter values) and gains the count read and one FIFO read.

## 5. The packet ArduPilot gets, worked byte by byte

**The twin's stimulus** (`4400e` at `ed936af7`, `spike/4400e/out/runs/j1/setup.cmds` L63–65, the part table's data step): 29 °C, `AccelerationX` 0 g, `AccelerationY` 1 g; `AccelerationZ` and every angular rate at the model's 0 (`ICM42688P.cs` L51–57).

**The configuration** (§2): accelerometer at ±16 g, 2048 LSB/g (p.78, `ICM42688P.cs` L551); gyro at ±2000 dps, 16.4 LSB/dps; `INTF_CONFIG0` = `0xC0`.

**The values:**
- accel Y = 1 × 2048 = 2048 = `0x0800`;
- every other axis 0;
- temperature `round((29 − 25) × 2.07)` = round(8.28) = 8 = `0x08`.

**The clock:** `PWR_MGMT0` ← `0x0F` at t₀. The accelerometer is valid from t₀ + 10 ms, the gyro from t₀ + 30 ms, then every 500 µs. Before t₀ + 30 ms the gyro slots hold 0 (B5), which equals this stimulus's gyro.

**Two packets, for illustration with t₀ = 30.000 ms after the model's reset** (a twin run's t₀ is its own; only the timestamp depends on it):

| Offset | Field | Packet at 40.000 ms | Packet at 40.500 ms | Rule |
|---|---|---|---|---|
| 0 | header | `68` | `68` | `0110_10` + `00` (B7; then the same ODR) |
| 1–2 | accel X | `00 00` | `00 00` | LE (B1) |
| 3–4 | accel Y | `00 08` | `00 08` | 2048, LE |
| 5–6 | accel Z | `00 00` | `00 00` | |
| 7–12 | gyro X, Y, Z | `00 00 00 00 00 00` | `00 00 00 00 00 00` | |
| 13 | temperature | `08` | `08` | |
| 14–15 | timestamp | `7C 92` | `50 94` | ⌊40000 × 30/32⌋ = 37500 = `0x927C`; ⌊40500 × 30/32⌋ = 37968 = `0x9450`; step 468 (p.58 scaled to 2 kHz: 468.75) |

- **The count read** after both packets: `bank 0 read 0x2E -> 0x02 0x00` (records, LE, B2).
- **The FIFO read:** `bank 0 read 0x30 -> 0x68 0x00 0x00 0x00 0x08 …`, 32 bytes on one log line (B3).
- **ArduPilot's decode:**
  - header `0x68 & 0xFC` = `0x68`: accepted;
  - accel (0, 2048, 0) × `ACCEL_SCALE` = (0, 1 g, 0) before its rotation;
  - gyro 0;
  - temperature 8 / 2.07 + 25 = 28.86 °C.

## 6. Goldens and verdicts that would move

- **The generated firmware's verdicts: none [read at source; measured in the build].**
  - The firmware's script writes only `0x4C` ← `0xC0`, `0x50` ← `0x66` and `0x4E` ← `0x03` (`parts.json`, the `ICM42688P` script). It never writes `FIFO_CONFIG` or `FIFO_CONFIG1`, so the FIFO stays in Bypass with no sensor enabled: no packet and no new log line.
  - Its one data read, `0x1D` for 6 bytes, ends at `0x22`, so B3's hold at `0x30` cannot reach it.
  - `models_tier2.rs`'s reset-value reads are single bytes that touch none of `0x2E`–`0x30`.
  - In the build: `twin_tier2`, `own_models_tier1`, `own_board_tier1` and the bench board, unchanged.
- **Forced, outside the fence; asked by name:**
  - **`crates/aethelgard-twin/src/model.rs` L71**, the model's sha256 pin (`OWN`, `AET-WO-4220` D6). Any change to the file moves it, and `every_model_of_our_own_is_pinned_by_its_sha256` fails until it moves. No other file carries the hash (grep).
  - **`crates/aethelgard-cli/src/ledger.rs` L1337, L1342, L1347**, which cite `ICM42688P.cs L174, L242` and `L172–175`. The header's two lines will be rewritten in place, but new code before L242 moves it. Proposed: update the three citations to the new lines (or cite the method names). No test or committed output carries them (grep).
- **Inside the fence:** the model's header comment (L5–10, "Not modelled: the FIFO …"), its README entry and `models_tier2.rs`. `4220`'s replay moves, as in §4.4 item 8.
- **Not moved:** the BMP581 and its pin; the bench package (no ICM-42688-P); `parts.json` (D6 (i) adds the FIFO data step later, in `4400e`'s fence, not here).

## 7. Record

- **Base:** `main` `7d52d29`. Branch `wo/aet-4220c-icm-fifo`: this FEEDBACK only.
- **How the datasheet was read:** text by `pymupdf`. Every table this proposal rests on was confirmed on the rendered page: pp.38, 39, 41, 58, 61, 75 and 87.
- **A label to correct, outside this fence:** `4400e` FEEDBACK L154 calls `0x50` ← `0x05` "±16 g, 8 kHz". Code `0101` is 2 kHz (p.78), which matches ArduPilot's 2 kHz path (`IV3` L787–788).
- **Running beside this:** `4400e2`. Nothing here touches its files.
