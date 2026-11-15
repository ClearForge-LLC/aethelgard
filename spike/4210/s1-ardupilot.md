# AET-WO-4210 S1: what ArduPilot does to talk to five parts

Source: ArduPilot tag `Copter-4.7.2`, commit `fb773567896014da007bca58358704d4a0d8516f`, read-only sparse clone at `/home/forge/.cache/aet-4210/ardupilot`.
ArduPilot is GPLv3: this file cites `file:line` and describes behaviour; it contains no copied source blocks.

Path shorthand (all under `libraries/`):
- `INS/` = `AP_InertialSensor/`
- `BARO/` = `AP_Baro/`
- `HWDEF/` = `AP_HAL_ChibiOS/hwdef/`
- `HAL/` = `AP_HAL/`

Label rule: anything not read directly from the code is marked INFERRED.

## 0. Answers to the two direct questions

**LSM6DSO (WHO_AM_I 0x0F = 0x6C): NOT accepted by any driver in this tree.**
- The only ST LSM6-family INS driver is LSM6DSV. Its identity switch has exactly one accepted value, `LSM6DSV_ID_LSM6DSV16X` = 0x70 (`INS/AP_InertialSensor_LSM6DSV.cpp:65`, switch at `:375-377`). Any other value makes `check_whoami()` return false (`:379`).
- ASM330 accepts only 0x6B (`INS/AP_InertialSensor_ASM330_registers.h:5`, compare at `INS/AP_InertialSensor_ASM330.cpp:80-81`).
- No file under `libraries/AP_InertialSensor` contains an accepted 0x6C. The only grep hits for 0x6C are `MPUREG_PWR_MGMT_2` (`INS/AP_InertialSensor_Invensense_registers.h:143`, a register address) and `ASM330_REG_6C_RESERVED` (`INS/AP_InertialSensor_ASM330_registers.h:291`, a reserved register address). Neither is an identity value.
- Full accepted-ID table is in section 2.

**BMP180 (chip id register 0xD0 = 0x55): YES, handled by `AP_Baro_BMP085`.**
- The probe reads register 0xD0 and rejects anything but 0x55 (`BARO/AP_Baro_BMP085.cpp:78-84`). The in-code comment on the rejection branch says "not BMP180" (`:83`), so 0x55 is treated as the BMP180/BMP085 family.
- It is compiled in by default (section 6.3) but is probed only if a hwdef `BARO BMP085` line names it, or if the user sets bit 0 of `BARO_PROBE_EXT` (section 7).

## 1. ICM-20948 via `AP_InertialSensor_Invensensev2`

Files: `INS/AP_InertialSensor_Invensensev2.cpp` (call it `INV2.cpp`), `INS/AP_InertialSensor_Invensensev2_registers.h` (`INV2reg.h`).
The same driver handles ICM-20648 (0xE0), ICM-20948 (0xEA) and ICM-20649 (0xE1) (`INV2reg.h:217-219`).

### A. Probe (`probe()` -> `_init()` -> `_hardware_init()`)

Entry. There are two `probe()` overloads: I2C (`INV2.cpp:89`) and SPI (`INV2.cpp:108`).
- The SPI overload first calls `set_read_flag(0x80)` (`:117`), so every register read ORs 0x80 into the address byte (`HAL/Device.cpp:63-66`).
- The I2C overload sets no read flag.
- If `INVENSENSEV2_DRDY_PIN` is defined, `_init()` makes that GPIO an input (`:131-134`). Otherwise nothing.

`_hardware_init()` runs under the bus semaphore (`:685`). In order:
1. Allocates checked-register storage: 7 slots, with a check frequency of 20 on SPI or 200 on I2C (`:688`). The source comment on `:687` reads "disabled setup of checked registers ...", but `:688` still makes the call. The readback behaviour is in the B section.
2. Registers a bank-select callback (`:689`). Every register access first goes through `_select_bank()` (`HAL/Device.cpp:194-203`). `_current_bank` starts at 0xFF (`INS/AP_InertialSensor_Invensensev2.h:150`), so the very first access always writes `REG_BANK_SEL` (0x7F, `INV2reg.h:214`) with `bank << 4` (`INV2.cpp:591`).
3. Sets bus speed LOW (`:692`).
4. Identity check: reads WHO_AM_I (bank 0, 0x00, 1 byte; `INV2reg.h:18`, `INV2.cpp:667`). It accepts 0xE0, 0xEA or 0xE1 and records the type (`INV2.cpp:669-678`). Anything else returns false, with no retry (`:694-696`).
5. Reset loop, up to 5 tries (`:700`). Each try:
   - reads USER_CTRL (bank 0, 0x03; `:701`);
   - if the I2C-master-enable bit is set, clears it and writes USER_CTRL back, then delays 10 ms (`:706-710`);
   - writes PWR_MGMT_1 (0x06) with the DEVICE_RESET bit 0x80 (`:713`), then delays 100 ms (`:714`);
   - on SPI only, ORs `BIT_USER_CTRL_I2C_IF_DIS` (0x10) into the cached USER_CTRL value and writes USER_CTRL (`:717-722`);
   - writes PWR_MGMT_1 = 0x01 (clock auto; `BIT_PWR_MGMT_1_CLK_AUTO`, `INV2reg.h:30`), then delays 5 ms (`:727-728`);
   - reads PWR_MGMT_1 back and succeeds if it equals 0x01 (`:731`). This is a readback verify of the wake-up;
   - if not, delays 10 ms and checks `_data_ready()` (reads INT_STATUS_1, 0x1A, nonzero means ready; `:735-737`, `:308-315`). Success if ready; otherwise it loops.
6. After the loop it sets bus speed HIGH (`:741`). If all 5 tries fail it prints "Failed to boot Invensense 5 times" and returns false (`:743-746`).
7. For an ICM-20649 only, it sets a 29.5 g clip limit (`:748-750`).

The probe does not read any ICM-20948 identity register other than WHO_AM_I. It does not touch the AK09916 magnetometer.

### B. Init/configure (`start()`, called later by the INS front end; `INV2.cpp:167`)

Order:
1. Speed LOW (`:177`).
2. PWR_MGMT_2 (bank 0, 0x07) = 0x00 (`:180`), delay 1 ms (`:181`).
3. `_fifo_reset()` (`:184`, body `:141-160`):
   - LOW speed;
   - FIFO_EN_2 (0x67) = 0;
   - USER_CTRL with FIFO_EN (0x40) cleared;
   - FIFO_RST (0x68) = 0x0F, then 0x00;
   - USER_CTRL with FIFO_EN (0x40) set;
   - FIFO_EN_2 = accel + gyro X/Y/Z + temp enable bits (0x10|0x08|0x04|0x02|0x01 = 0x1F; `INV2reg.h:96-101`), written as a checked register (`:152-153`);
   - 1 us delay, then HIGH speed.
4. Registers the gyro and accel instances at 1125 Hz (`:210-211`).
5. `_set_filter_and_scaling()` (`:217`, body `:602-660`):
   - Gyro full scale +-2000 dps (`BITS_GYRO_FS_2000DPS` 0x06, or 0x04 for the 20649). Accel full scale +-16 g (`BITS_ACCEL_FS_16G` 0x06, or the 30 g constant for the 20649) (`:604-605`).
   - Not fast-sampling (the default; the I2C path never fast-samples, `:612`): sets the gyro DLPF-enable bit with 188 Hz config and the accel DLPF-enable bit with 265 Hz config (`:654-655`).
   - Fast-sampling, SPI only: DLPF off, 9 kHz gyro / 4.5 kHz accel (`:649-651`), and I2C_SLV4_CTRL (bank 3, 0x15) = 0x1F (`:644`).
   - Writes GYRO_CONFIG_1 (bank 2, 0x01), ACCEL_CONFIG (bank 2, 0x14), FIFO_MODE (bank 0, 0x69) = 0x0F. All three are written as checked (`:657-659`).
6. GYRO_SMPLRT_DIV (bank 2, 0x00) = 0, checked (`:230`), delay 1 ms (`:231`). The comment says this gives 1.125 kHz. ACCEL_SMPLRT_DIV is never written (INFERRED: reset default, so the accel rate follows the same 1.125 kHz).
7. INT_ENABLE_1 (bank 0, 0x11) = 0x01 (`:234`), delay 1 ms. This enables the raw-data-ready interrupt bit in the status register. INT_PIN_CFG (0x0F) is never written (grep of `INV2.cpp`: not referenced), so no pin configuration is done here (INFERRED: no interrupt pin is used).
8. Speed HIGH (`:238`). Sets orientation (`:241-242`). Allocates an 8-sample x 14-byte DMA-safe FIFO buffer (`:249`; `INV2_SAMPLE_SIZE` 14 and `INV2_FIFO_BUFFER_LEN` 8 at `:63-64`).
9. Registers the periodic callback (`:255`).

Readback verification:
- Explicit in code: only the PWR_MGMT_1 wake readback in probe (`:731`).
- Deferred, "checked registers": writes flagged `checked=true` are remembered (`HAL/Device.cpp:69-76`, `:86-107`) and one is re-read every 20th (SPI) / 200th (I2C) FIFO poll (`INV2.cpp:688`, `:539-548`; `HAL/Device.cpp:112-157`). A mismatch is re-written. The checked set is GYRO_CONFIG_1, ACCEL_CONFIG, FIFO_MODE, GYRO_SMPLRT_DIV, FIFO_EN_2 and, per `INV2.cpp:591`, BANK_SEL 0x7F (a checked write via the 2-argument path, `HAL/Device.cpp:169-174`, `:81-84`).

### C. Steady-state read

FIFO, not direct data registers. `_poll_data()` calls `_read_fifo()` (`INV2.cpp:320-323`). Per poll cycle:
1. Reads FIFO_COUNTH (bank 0, 0x70), 2 bytes (`:462`).
2. `n_samples` = count / 14 (`:466-467`). If zero, goes straight to the register-check tail (`:469-472`).
3. Caps the read: more than 32 samples on SPI means read 24 then reset the FIFO. More than 4 samples on I2C means read 4 then reset (`:484-494`).
4. Reads FIFO_R_W (0x72) in chunks of at most 8 samples (112 bytes) (`:495-517`).
   - Where the HAL supports `set_chip_select`, it uses a manual-CS path: transfer_bank of the register byte, then a full-duplex clock-out of n*14 bytes (`:501-516`). Otherwise `_block_read` (`:498`).
   - Each 14-byte sample is accel X/Y/Z (6 B), gyro X/Y/Z (6 B), temp (2 B) (`:336-353`, `:341`).
5. Sanity: a temperature jump of 800 or more versus the cached value triggers a 2-byte read of TEMP_OUT_H (0x39) and, if still bad, a FIFO reset (`:554-568`, `:342-347`).
6. Tail: drops to LOW speed, runs `check_next_register()` (one checked-register readback every 20th/200th call), and returns to HIGH speed (`:539-548`).

Poll period is `1265625 / _gyro_backend_rate_hz` us (`:255`). At 1125 Hz that is 1125 us. Fast sampling multiplies the rate (`:625-627`).

It does not use ACCEL_XOUT_H bursts. The data register constants exist (`INV2reg.h:53-66`) but are used only for TEMP_OUT_H and EXT_SLV_SENS_DATA (the aux-bus path). INV2.cpp has no reference to ACCEL_XOUT_H in the sampling path (INFERRED from reading `_read_fifo`/`_accumulate`).

### D. Bus and hwdef

- Both I2C and SPI (`INV2.cpp:89` and `:108`). Aux-bus (a downstream magnetometer on the IMU's own I2C master) is offered only when the bus is not I2C (`:162-165`).
- hwdef IMU line, parsed in `HAL/hwdef/scripts/hwdef.py`:
  - `process_line` stores everything after `IMU` (`hwdef.py:249-250`).
  - `write_IMU_config` (`:382-425`): first word = driver name; later words that start `SPI:` become `hal.spi->get_device("<name>")` (`:402-403`, `:325-328`); words starting `I2C:` go through `parse_i2c_device` (`:404-405`, `:336-373`).
  - The emitted macro is `HAL_INS_PROBE<n>` = `ADD_BACKEND(AP_InertialSensor_<driver>::probe(*this,<args>))` (`:419-421`).
  - Optional trailing `INSTANCE:<n>`, `AUX:<id>` or `BOARD_MATCH(...)` (`:395-400`, `:408-417`).
  - Accepted I2C forms are `I2C:<busnum>:<addr>` or `I2C:{ALL_INTERNAL|ALL_EXTERNAL|ALL}:<addr>` (`:345-348`).
- Examples, CubeOrange (STM32H743, `FLASH_SIZE_KB 2048` at `HWDEF/CubeOrange/hwdef.inc:18`):
  - `SPIDEV icm20948 SPI1 DEVID4 MPU_CS MODE3 4*MHZ 8*MHZ` (`hwdef.inc:216`)
  - `SPIDEV icm20948_ext SPI4 DEVID1 MPU_EXT_CS MODE3 4*MHZ 8*MHZ` (`hwdef.inc:217`)
  - `IMU Invensensev2 SPI:icm20948_ext ROTATION_PITCH_180` (`hwdef.inc:228`)
  - `IMU Invensensev2 SPI:icm20948 ROTATION_YAW_270` (`hwdef.inc:229`)
  - CubeBlack has the same lines at `HWDEF/CubeBlack/hwdef.dat:25,64-65`.
- SPIDEV line format, as parsed in `HWDEF/scripts/chibios_hwdef.py:1453-1479`:
  `SPIDEV <name> <SPIbus> DEVID<n> <CS-pin-label> MODE0|1|2|3 <low>*MHZ|KHZ <high>*MHZ|KHZ`.
  - Exactly 7 fields after the keyword (`len(dev) != 7` check, `:1456`).
  - The mode must be MODE0..MODE3 (`:1471-1472`).
  - Speeds must end in `*MHZ` or `*KHZ` (`:1473-1478`).
  - The CS label must be a pin declared with the `CS` function (`:1469-1470`).
  - Emitted as `SPIDesc(name, busidx, devid, PAL_LINE, mode, lowspeed, highspeed)` (`:1481-1484`).
  - The driver's `SPEED_LOW`/`SPEED_HIGH` map to the low/high fields (INFERRED from the field names and the driver's use of `set_speed`).
- Default SPI speeds from the example: low 4 MHz, high 8 MHz, MODE3.

### E. Data-ready

No interrupt/DRDY GPIO in the default build. The sensor is polled by a bus-thread periodic callback (`INV2.cpp:255`). The DRDY pin is used only if the board defines `INVENSENSEV2_DRDY_PIN` (`:131-134`), and even then only inside `_data_ready()` (`:310-312`), which the code calls solely from the probe-time wake-up retry loop (`:736`). Not used in `_read_fifo`. (INV2 has no other references to `_drdy_pin`; checked with grep.) `INVENSENSEV2_DRDY_PIN` is referenced only at `INV2.cpp:131-132` across the whole sparse clone, and no hwdef I read defines it.

### F. Default build inclusion

- `INS/AP_InertialSensor_config.h` has no per-driver enable define for Invensensev2, and none that depends on `BOARD_FLASH_SIZE` or `HAL_MINIMIZE_FEATURES`. The only INS-config symbols that touch flash are `HAL_PROGRAM_SIZE_LIMIT_KB > 1024` (temperature-cal at `:46-48`, harmonic-notch count at `:66-70`). There is an `AP_INERTIALSENSOR_BACKEND_DEFAULT_ENABLED` (0) at `:87-89`, but a grep of the INS directory finds no user of it (INFERRED: unused in this tag).
- `AP_InertialSensor_Invensensev2.cpp/.h` have no `#if` feature guard around the driver (checked file heads and `grep ^#if`). So the driver source is compiled on any ChibiOS build, including H7 with more than 1 MB flash. Whether it is linked in depends on the code reaching it (INFERRED: link-time garbage collection drops unreferenced code).
- It is only reachable on ChibiOS through the hwdef-generated `HAL_INS_PROBE_LIST` (`INS/AP_InertialSensor.cpp:1220-1222`). The legacy board-detect table (`:1252-1256`) lists "icm20948" probes only under `AP_FEATURE_BOARD_DETECT` when no `HAL_INS_PROBE_LIST` exists, so it does not apply to a hwdef board.
- What a board's hwdef must set: an `SPIDEV` (or I2C bus) entry plus an `IMU Invensensev2 ...` line (section D). No `AP_..._ENABLED` define is needed.
- Out-of-tree build-time stripping (Tools/scripts/build_options.py, the custom firmware builder) is not in this sparse clone: not examined.

### G. Autodetect

Probed only when a hwdef `IMU Invensensev2` line exists. `AP_InertialSensor.cpp` has no default I2C scan for it. Each probe goes through `ADD_BACKEND`, which is gated by the `INS_ENABLE_MASK` bit for that probe slot (`INS/AP_InertialSensor.cpp:1163-1169`). Wrong or absent WHO_AM_I just fails the probe.

## 2. The LSM6DSO question: every accepted identity value

WHO_AM_I register addresses are 0x0F for the ST parts, 0x75 or 0x72 for InvenSense, and various for the others.

| Driver | Register | Accepted value(s) | Citation |
|---|---|---|---|
| `AP_InertialSensor_LSM6DSV` | 0x0F | 0x70 only (LSM6DSV16X) | `INS/AP_InertialSensor_LSM6DSV.cpp:64-65`, `:369-380` |
| `AP_InertialSensor_ASM330` | 0x0F | 0x6B | `INS/AP_InertialSensor_ASM330_registers.h:5`, `INS/AP_InertialSensor_ASM330.cpp:80-82` |
| `AP_InertialSensor_LSM9DS0` | 0x0F (gyro) and 0x0F (xm), two chips | gyro 0xD4 (L3GD20) or 0xD7 (L3GD20H); xm 0x49 (LSM303D) | `INS/AP_InertialSensor_LSM9DS0.cpp:27-29`, `:461-467` |
| `AP_InertialSensor_LSM9DS1` | 0x0F | 0x68 (mag 0x3D defined at `:15`) | `INS/AP_InertialSensor_LSM9DS1.cpp:14`, `:256-257` |
| `AP_InertialSensor_L3G4200D` | 0x0F | 0xD3 | `INS/AP_InertialSensor_L3G4200D.cpp:93`, `:205-206` |
| `AP_InertialSensor_Invensensev2` | bank 0, 0x00 | 0xE0 (20648), 0xEA (20948), 0xE1 (20649) | `INS/AP_InertialSensor_Invensensev2_registers.h:217-219` |
| `AP_InertialSensor_Invensense` (v1) | 0x75 | 0x68, 0xAE, 0xAF, 0x12, 0xAC, 0x70, 0x71, 0x73, 0x03, 0x02, 0x98 | `INS/AP_InertialSensor_Invensense_registers.h:185-195`, switch `INS/AP_InertialSensor_Invensense.cpp:962-991` |
| `AP_InertialSensor_Invensensev3` | 0x75, then 0x72 | 0x33, 0x3B, 0x42, 0x47, 0xDB, 0x6F, 0x56, 0x67 at 0x75; 0xE9 at 0x72 | `INS/AP_InertialSensor_Invensensev3.cpp:136-144`, `:1003-1039` |
| `AP_InertialSensor_BMI055` | accel 0x00, gyro 0x00 | accel 0xFA, gyro 0x0F | `INS/AP_InertialSensor_BMI055.cpp:128`, `:182` |
| `AP_InertialSensor_BMI088` | accel 0x00, gyro 0x00 | accel 0x1E (BMI088) or 0x1F (BMI085), gyro 0x0F | `INS/AP_InertialSensor_BMI088.cpp:205-222`, `:243` |
| `AP_InertialSensor_BMI160` | 0x00 | 0xD1 | `INS/AP_InertialSensor_BMI160.cpp:36-37` |
| `AP_InertialSensor_BMI270` | 0x00 | 0x24 | `INS/AP_InertialSensor_BMI270.cpp:129` |
| `AP_InertialSensor_ADIS1647x` | 16-bit PROD_ID 0x72 | 0x4056, 0x405D, 0x407B | `INS/AP_InertialSensor_ADIS1647x.cpp:26-29`, `:134-145` |
| `AP_InertialSensor_ADIS16607` | 16-bit DEV_ID 0x00 | 0x6000 | `INS/AP_InertialSensor_ADIS16607.cpp:26-27` |

0x6C appears in none of these. **LSM6DSO: not accepted.** An LSM6DSO on a hwdef `IMU LSM6DSV ...` line would fail at `check_whoami()` (0x6C is not 0x70).

Whether the LSM6DSO's register map is close enough to the LSM6DSV driver to be a trivial patch is INFERRED/unknown and was not assessed here.

## 3. BMP180 via `AP_Baro_BMP085`

Files: `BARO/AP_Baro_BMP085.cpp` (`BMP085.cpp`), `BARO/AP_Baro_BMP085.h`.

Is the BMP180 handled? **Yes.** One class covers both, keyed on id 0x55 (`BMP085.cpp:78-84`).

### A. Probe (`probe()` -> `_init()`, `BMP085.cpp:43-52`, `:54-144`)

Under the bus semaphore (`:68`). In order:
1. If `BMP085_EOC` >= 0 (not defined by default; default -1, `:31-33`), set up an end-of-conversion GPIO (`:70-73`).
2. Read register 0xD0, 1 byte. Must equal 0x55, else fail (`:78-84`). A failed read also fails.
3. Read register 0xD1, 1 byte, into `_vers`. Return value ignored (`:87`).
4. Read calibration: register 0xAA, 22 bytes in one burst (`:93`). If that read fails, fall back to 11 separate 2-byte reads of 0xAA + 2*i (`_read_prom`, `:99`, `:146-171`). This is for I2C hosts that cannot burst. It rejects an all-zero PROM (`:166-168`).
5. Parse eleven big-endian 16-bit words AC1..AC6, B1, B2, MB, MC, MD (`:108-118`).
6. Reject if any of AC1..AC3 is 0 or -1, or AC4..AC6 is 0 or 0xFFFF (`:120-127`).
7. Write register 0xF4 = 0x2E (start temperature conversion) (`_cmd_read_temp`, `:133`, `:246-250`).
8. Register the sensor and a 20 ms periodic callback (`:137-142`).

No explicit retries or delays in the driver (INFERRED: any retries are in the HAL I2C layer; not read). No soft reset (the BMP085/180 reset register 0xE0 is never written).

### B. Init/configure

There is no separate init. All of the above is it: no ODR or range setting exists on this part. Oversampling is fixed at compile time. `OVERSAMPLING` is ultra-high-res (3) unless `BMP085_EOC` is defined (`:31-36`). No register is read back to verify a write.

### C. Steady state (`_timer`, `:176-195`, every 20 ms)

State machine, driven by elapsed-time checks (`_data_ready`, `:315-346`):
- State 0: wait more than 5 ms since the temp command, then read register 0xF6, 2 bytes (raw temperature, `:253-266`).
- Other states: wait more than 26 ms (ultra-high-res) since the pressure command, then read register 0xF6, 3 bytes (fallback: 0xF6 2 bytes plus 0xF8 1 byte) (`:222-243`).
- After each read, write register 0xF4 = 0x34 + (OVERSAMPLING << 6) to start the next pressure conversion (`:215-219`). Every 25th state it instead rewrites 0xF4 = 0x2E to refresh temperature (`:188-194`).
- Direct data registers, no FIFO.
- Approximate pressure rate is one sample per two 20 ms ticks, about 25 Hz (INFERRED from the 26 ms wait versus the 20 ms tick).

### D. Bus

- The driver takes a generic `AP_HAL::Device` and uses `read_registers`/`write_register`/`transfer` (`:78`, `:217`, `:150`). The code has no bus-type check.
- BMP085/180 is an I2C part; the default address is 0x77 (`BARO/AP_Baro_BMP085.h:11-12`). SPI use is INFERRED not meaningful.
- hwdef example, speedybeef4 (STM32F405, `FLASH_SIZE_KB 1024`): `BARO BMP085 I2C:0:0x77` (`HWDEF/speedybeef4/hwdef.dat:126`, `:17`). Next to it, `define HAL_BARO_ALLOW_INIT_NO_BARO` (`:127`).
- BARO line parsing: `HAL/hwdef/scripts/hwdef.py:557-589` (first word `DRIVER[:probe_fn]`, then `SPI:<name>`, `I2C:<bus>:<addr>`, or a bare `0xNN`). Generated macro is `HAL_BARO_PROBE<n>` = `probe_i2c_dev(AP_Baro_BMP085::probe, <bus>, <addr>)` or `probe_spi_dev(..., "<name>")` (`:591-647`; `AP_Baro.cpp:529-540`). `probe_dev` deletes the Device if the driver returns null (`:543-558`).
- Speeds: no SPI speeds apply (I2C).

### E. Data-ready

No DRDY used by default. The optional EOC pin (`BMP085_EOC`) is off by default (`:31-33`). Timing is by elapsed millis (`:321-345`). The timer callback is `register_periodic_callback(20000, ...)` (`:142`).

### F. Default build inclusion

- `AP_BARO_BMP085_ENABLED` = `AP_BARO_BACKEND_DEFAULT_ENABLED` (`BARO/AP_Baro_config.h:21-23`), and that defaults to 1 (`:17-19`). Neither depends on `BOARD_FLASH_SIZE` or `HAL_MINIMIZE_FEATURES`. A grep of `AP_Baro`, `AP_HAL` and `AP_HAL_ChibiOS` found no override of `AP_BARO_BACKEND_DEFAULT_ENABLED` and no `HAL_MINIMIZE_FEATURES` use (the only hit in ChibiOS is `AP_BARO_ENABLED` in `defaults_periph.h:387`, for the AP_Periph build).
- So it is ON by default on an H7 build with more than 1 MB flash.
- A hwdef `BARO` line forces it on regardless: `write_BARO_config` emits `#undef AP_BARO_<driver>_ENABLED` then `#define ... 1` (`hwdef.py:650`).

### G. Autodetect

Two routes:
1. The hwdef `BARO BMP085 I2C:<bus>:<addr>` line (`HAL_BARO_PROBE_LIST`, `AP_Baro.cpp:664-666`).
2. The external-I2C scan `_probe_i2c_barometers()` (`AP_Baro.cpp:767-857`). Its table includes BMP085 at `HAL_BARO_BMP085_I2C_ADDR` = 0x77 (`:793-795`), but each entry is gated by a bit of the `BARO_PROBE_EXT` parameter (`:847-851`). BMP085 is bit 0 (`AP_Baro.h:256`). The default is `HAL_BARO_PROBE_EXT_DEFAULT` = 0 (`AP_Baro.cpp:69-71`, `:187`), so nothing is auto-probed unless the user sets the parameter. It scans the external I2C buses (`:776`), or only `BARO_EXT_BUS` if that is >= 0 (`:783-786`; default -1, `:73-75`).

The macro the task named (`HAL_PROBE_EXTERNAL_I2C_BAROS`) does not exist in this tag. The equivalents are `AP_BARO_PROBE_EXTERNAL_I2C_BUSES` (default 1, `AP_Baro_config.h:114-116`) and `AP_BARO_PROBE_EXT_PARAMETER_ENABLED` (`:118-120`). The scan code is compiled in by default, but the param default of 0 makes it a no-op.

## 4. LPS25HB via `AP_Baro_LPS2XH`

Files: `BARO/AP_Baro_LPS2XH.cpp` (`LPS.cpp`), `BARO/AP_Baro_LPS2XH.h`.
The driver accepts the LPS22HB (0xB1) and the LPS25HB (0xBD) (`LPS.cpp:26-27`, `:193-200`). It treats the part as one of two types, `BARO_LPS22H` or `BARO_LPS25H` (`LPS2XH.h:21-24`). An LPS25H or LPS25HB reading 0xBD is accepted; there is no separate handling of LPS25H vs LPS25HB.

### A. Probe (`probe()` -> `_init()`, `LPS.cpp:64-73`, `:129-182`)

Under the bus semaphore (`:134`). In order:
1. Set speed HIGH (`:136`).
2. On SPI only: `set_read_flag(0x80)` (`:139-141`). No auto-increment bit (0x40) is set.
3. Read WHO_AM_I: register 0x0F, 1 byte (`:188`). Accept 0xB1 (LPS22H) or 0xBD (LPS25H); otherwise fail with no retry (`:193-202`).
4. Type-specific configuration (next section).

Alternate entry, `probe_InvensenseIMU` (`:75-93`, `:99-127`): used when the baro hangs off an InvenSense IMU's pass-through bus. It temporarily changes the I2C address to the IMU's, reads 0x75 (WHOAMI), writes FIFO_EN = 0x00, PWR_MGMT_1 = `BIT_PWR_MGMT_1_CLK_XGYRO`, delays 10 ms, writes INT_PIN_CFG = `BIT_BYPASS_EN`, restores the address; sets 4 retries (`:108`). The crazyflie2 board uses this: `BARO LPS2XH:probe_InvensenseIMU I2C:0:0x5D 0x69` (`HWDEF/crazyflie2/hwdef.dat:111`). Not needed for a stand-alone LPS25HB.

### B. Init/configure

LPS25H (0xBD) (`:149-157`):
1. CTRL_REG1 (0x20) = 0x00 (power-down for config).
2. CTRL_REG2 (0x21) = 0x00 (FIFO disabled).
3. FIFO_CTRL (0x2E) = 0x01.
4. CTRL_REG1 (0x20) = 0xC0. (INFERRED from the datasheet bit layout: PD plus 25 Hz ODR; the code comment says "25Hz update (maximum refresh rate)".)
5. Poll period = 40 ms (`CallTime`, `:156`).

LPS22H (0xB1) (`:159-170`):
1. CTRL_REG1 (0x10) = 0x00.
2. CTRL_REG1 = 75 Hz ODR | BDU | EN_LPFP | LPFP_CFG.
3. CTRL_REG2 (0x11) = 0x18 on SPI (disables I2C), or 0x10 on I2C.
4. Poll period = 1/75 s.

Then it registers the sensor (`:172-175`) and a periodic callback (`:179`).

- No soft reset, no boot wait, and no register is read back to verify any write.
- Nothing sets BDU on the LPS25H path (INFERRED from the code).

### C. Steady-state read (`_timer`, `:206-221`)

Every 40 ms (LPS25H):
1. Read STATUS (0x27), 1 byte (`:210`).
2. If bit 1 (0x02): read TEMP_OUT (0x2B), 2 bytes, then convert (`:214-215`, `:237-254`).
3. If bit 0 (0x01): read PRESS_OUT_XL (0x28), 3 bytes (`:218-219`, `:257-270`).

- Direct data registers, no FIFO.
- Observation, flagged INFERRED: the header comment at `LPS.cpp:56` says the MSB of the address turns on auto-increment, but no code sets it. The multi-byte reads at 0x2B and 0x28 are issued with the plain address (I2C) or with 0x80 only (SPI). For the real LPS25HB this would need the auto-increment bit on I2C; I have not verified this against the datasheet, nor against how the ChibiOS I2C layer treats the register address. Worth checking in S2/S3 rather than assuming.

### D. Bus

- Generic `AP_HAL::Device`; I2C and SPI both appear in the code (`:139`, `:162`). The LPS25HB supports both on the part itself (INFERRED from the datasheet, not from the code).
- Default I2C address 0x5D (`LPS2XH.h:13-14`).
- hwdef: `BARO LPS2XH I2C:0:0x5d` form (the task's example) goes through `probe_i2c_dev(AP_Baro_LPS2XH::probe, 0, 0x5d)` per `hwdef.py:620-621`. The SPI example in this tag is mini-pix: `SPIDEV lps22h SPI2 DEVID3 LPS22HB_CS MODE3 1*MHZ 10*MHZ` (`HWDEF/mini-pix/hwdef.dat:116`), `BARO LPS2XH SPI:lps22h` (`:120`).
- Default SPI speeds from that example: low 1 MHz, high 10 MHz, MODE3 (it is an LPS22HB, not LPS25HB).
- I did not find an LPS25HB SPI hwdef example in the boards I read. I did not find a stand-alone I2C hwdef line using this driver in the boards I read either (the I2C board I found is crazyflie2, via the IMU pass-through).

### E. Data-ready

No DRDY pin. Timer callback `register_periodic_callback(CallTime, ...)` (`:179`); data-ready is read from the STATUS register each tick (`:210-219`).

### F. Default build inclusion

`AP_BARO_LPS2XH_ENABLED` = `AP_BARO_BACKEND_DEFAULT_ENABLED` (`AP_Baro_config.h:69-71`) = 1 by default, no flash-size or minimize-features condition. ON by default on H7. A hwdef `BARO LPS2XH ...` line also forces it to 1 (`hwdef.py:650`).

### G. Autodetect

Yes, but only when the user opts in. LPS25H is in the external-I2C scan table at `HAL_BARO_LPS25H_I2C_ADDR` = 0x5D (`AP_Baro.cpp:830-832`), gated by `PROBE_LPS25H` = bit 7 of `BARO_PROBE_EXT` (`AP_Baro.h:263`; default 0). Note the table has a single LPS25H entry, 0x5D. A second address 0x5C is not probed.

## 5. ICP-10100 / ICP-10111 via `AP_Baro_ICP101XX`

Files: `BARO/AP_Baro_ICP101XX.cpp` (`ICP.cpp`), `BARO/AP_Baro_ICP101XX.h`.
Both parts share one product ID, so there is no way to tell 10100 and 10111 apart in this driver (INFERRED from the single `ICP101XX_ID` constant). All of it is I2C-style 16-bit-command traffic.

### A. Probe (`init()`, `ICP.cpp:66-113`)

Under the bus semaphore (`:72`). In order:
1. Send command 0xEFC8 (`CMD_READ_ID`), then read 2 bytes (`read_response`, `:75`, `:120-126`). The transfer's return value is ignored.
2. Identity check: `(id >> 8) & 0x3F` must equal 0x08 (`ICP101XX_ID`, `:37`, `:76-79`). On a little-endian host this is the second byte received, bits 5:0 (INFERRED from the host byte order).
3. Send command 0x805D (`CMD_SOFT_RESET`) (`:81`), then delay 10 ms (`:86`).
4. `read_calibration_data()` (`:88`, `:167-187`):
   - send command 0xC595 (`CMD_SET_ADDR`) with 3 data bytes 0x00, 0x66, 0x9C (`:170-171`);
   - then 4 times: send command 0xC7F7 (`CMD_READ_OTP`) and read 3 bytes, each a 16-bit word plus an 8-bit CRC (`:174-185`). The CRC is seeded 0xFF with polynomial 0x31 over the two data bytes (`:145-165`, `:176-183`). A mismatch fails the probe;
   - yields four signed 16-bit constants (`:37`, `:184`).
5. Start a measurement: send command 0x7866 (`CMD_MEAS_ULN`, ultra-low-noise) (`:93`, `:211-213`, `:221`).
6. `set_retries(0)` (`:97`). Register the instance and a periodic callback at `measure_interval` = 95000 us (`:99-106`, `:212`).

The ID read result and the OTP read return values are not checked individually (`:75`, `:177`). Only the ID mask, the CRC, and the sends are checked.

### B. Init/configure

There are no ODR or range registers: the part is command-driven. The "configuration" is the choice of measurement command: ULN (0x7866) is hard-coded at `:93`, `:287`, `:292`. The other commands exist as constants: LP 0x609C, N 0x6825, LN 0x70DF (`:41-44`). The calibration is the four OTP words (A.4). The measurement-time table is `:191-219`. No read-back verify of anything except the OTP CRC.

### C. Steady state (`timer`, `:278-296`, every 95 ms)

1. Raw I2C read of 9 bytes with no command, `transfer(nullptr, 0, buf, 9)` (`:115-118`, `:281`).
2. If it succeeds: temperature raw = bytes 0-1 (`:283`), pressure raw from bytes 3, 4 and 6 (`:284`). The CRC bytes are ignored ("ignore CRC bytes for now", `:282`). Convert, then immediately send the next ULN command 0x7866 (`:286-288`).
3. If it fails (the conversion is not done, so the part NACKs, INFERRED): if more than 3x the interval has passed since the last measure, resend 0x7866 (`:290-293`).

Direct reads, no FIFO. About 10.5 Hz (95 ms period). The accumulated samples are averaged in `update()` (`:298-307`).

### D. Bus

- Generic `AP_HAL::Device`, using `transfer()` only (`:117`, `:125`, `:133`, `:142`). The code has no bus-type check. The part is I2C (INFERRED from the datasheet/pin set; the driver itself would work on either).
- I2C address: the header defines none. The hwdef example CUAV_GPS lists two: `BARO ICP101XX I2C:0:0x63` and `BARO ICP101XX I2C:0:0x64` (`HWDEF/CUAV_GPS/hwdef.dat:86-87`; STM32F412, `FLASH_SIZE_KB 512` at `:28`; `env AP_PERIPH 1`, so a peripheral firmware, `:17-18`). So the parsing is the same `BARO` path: `probe_i2c_dev(AP_Baro_ICP101XX::probe, 0, 0x63)`.
- Speeds: not applicable (I2C).

### E. Data-ready

No DRDY or interrupt pin. A timer callback (`register_periodic_callback(measure_interval, ...)`, `:106`) polls; "ready" is discovered by whether the 9-byte read succeeds (`:281-295`).

### F. Default build inclusion

`AP_BARO_ICP101XX_ENABLED` = `AP_BARO_BACKEND_DEFAULT_ENABLED` (`AP_Baro_config.h:57-59`) = 1 by default, with no flash-size or minimize-features condition. ON by default on H7. A hwdef `BARO ICP101XX ...` line also forces it to 1 (`hwdef.py:650`).

### G. Autodetect

None. ICP101XX is not in the external-I2C scan table (`AP_Baro.cpp:793-844`) and has no `PROBE_` bit (`AP_Baro.h:256-270`). It is reachable only through a hwdef `BARO ICP101XX I2C:<bus>:<addr>` line. (`AP_Baro.cpp:49` includes its header; there is no other reference.)

## 6. Cross-cutting findings

### 6.1 How default-enable works for baros

- `AP_BARO_BACKEND_DEFAULT_ENABLED` is 1 (`AP_Baro_config.h:17-19`). All of BMP085, LPS2XH and ICP101XX default to it, so they are compiled in by default on any non-periph ChibiOS build (INFERRED for H7 > 1 MB from the absence of a size condition; no board I read overrides it).
- Compiled in is not probed. The probe happens only through:
  - `HAL_BARO_PROBE_LIST` (hwdef `BARO` lines), or
  - the `BARO_PROBE_EXT` bitmask (default 0) on external I2C buses, for BMP085 and LPS25H only.
- ICP101XX can only be reached through a hwdef `BARO` line.
- There is also a no-baro tolerance: unless `HAL_BARO_ALLOW_INIT_NO_BARO` is defined, no working baro makes `AP_BoardConfig::config_error("Baro: unable to initialise driver")` fire (`AP_Baro.cpp:747-756`).

### 6.2 `BARO` hwdef line gotchas

- `BARO <driver>[:<probe_fn>] <dev> [<dev2>]`, where `<dev>` is `SPI:<spidev-name>` or `I2C:<bus>:<addr>`.
- Only `SINGLE` bus numbers are accepted for baros (`hwdef.py:634-637`): `ALL`, `ALL_INTERNAL` and `ALL_EXTERNAL` raise an error for BARO lines.
- Duplicate lines are an error (`hwdef.py:562-563`).

### 6.3 INS drivers have no compile-time enable switch in this tag

Unlike the baros, the INS drivers listed above have no `AP_INERTIALSENSOR_<X>_ENABLED` (grep returned only ASM330 debug, fast-sample-window and harmonic-notch symbols). Selection is purely by hwdef `IMU` line.

## 7. Summary table

| Part | Driver | Buses | Probe identity check | Registers touched at probe + init | Steady-state read | DRDY used? | Default-enabled on H7 (>1 MB)? |
|---|---|---|---|---|---|---|---|
| ICM-20948 | `AP_InertialSensor_Invensensev2` | SPI and I2C | WHO_AM_I (bank 0, 0x00) in {0xE0, 0xEA, 0xE1}; 0xEA is the 20948 | BANK_SEL 0x7F, WHO_AM_I 0x00, USER_CTRL 0x03, PWR_MGMT_1 0x06 (reset 0x80, wake 0x01, readback), PWR_MGMT_2 0x07, FIFO_EN_2 0x67, FIFO_RST 0x68, FIFO_MODE 0x69, INT_ENABLE_1 0x11, INT_STATUS_1 0x1A (probe retry only), GYRO_SMPLRT_DIV b2/0x00, GYRO_CONFIG_1 b2/0x01, ACCEL_CONFIG b2/0x14, (I2C_SLV4_CTRL b3/0x15 if fast-sampling) | FIFO: FIFO_COUNTH 0x70 (2 B) then FIFO_R_W 0x72, 14 B per sample (accel 6, gyro 6, temp 2), up to 8 samples per burst; TEMP_OUT_H 0x39 (2 B) on a sanity miss; poll about every 1125 us | No. Optional `INVENSENSEV2_DRDY_PIN` is used only in probe retry; INT_PIN_CFG never written. Polled by `register_periodic_callback` | Driver code has no feature guard (compiled). Used only if a hwdef `IMU Invensensev2` line names it |
| LSM6DSO | none (`AP_InertialSensor_LSM6DSV` accepts 0x70, ASM330 0x6B; the DSO's 0x6C matches nothing) | n/a | not accepted | n/a | n/a | n/a | n/a |
| BMP180 | `AP_Baro_BMP085` | I2C (generic Device; no bus check in code) | reg 0xD0 == 0x55 | 0xD0 (1 B), 0xD1 (1 B), 0xAA (22 B, or 11 x 2 B fallback), 0xF4 write 0x2E | 0xF6 2 B (temp) or 3 B (pressure; fallback 0xF6 2 B + 0xF8 1 B); 0xF4 command write after each read; 20 ms tick, about 25 Hz | No (optional `BMP085_EOC`, default off); elapsed-time check | Yes: `AP_BARO_BMP085_ENABLED` = 1 via `AP_BARO_BACKEND_DEFAULT_ENABLED`; no flash-size condition |
| LPS25HB | `AP_Baro_LPS2XH` | I2C (default 0x5D) and SPI in code | reg 0x0F in {0xB1 (LPS22H), 0xBD (LPS25H)} | 0x0F read; LPS25H: CTRL_REG1 0x20 (0x00 then 0xC0), CTRL_REG2 0x21 (0x00), FIFO_CTRL 0x2E (0x01) | STATUS 0x27 (1 B), then TEMP_OUT 0x2B (2 B) and PRESS_OUT_XL 0x28 (3 B) when flagged; every 40 ms (25 Hz) | No. Polled by `register_periodic_callback`, STATUS register | Yes: `AP_BARO_LPS2XH_ENABLED` = 1; no flash-size condition |
| ICP-10100 / 10111 | `AP_Baro_ICP101XX` | I2C-style `transfer()` (0x63; CUAV_GPS lists 0x63 and 0x64) | cmd 0xEFC8 read 2 B; `(id>>8)&0x3F` == 0x08 | cmd 0xEFC8, 0x805D (reset), 0xC595 + {0x00, 0x66, 0x9C}, 4 x (0xC7F7 read 3 B, CRC-8 0x31 seed 0xFF), 0x7866 (start ULN) | raw 9-byte read with no command every 95 ms (about 10.5 Hz); resend 0x7866 after each good read or after 3x interval | No. Polled by `register_periodic_callback` | Yes: `AP_BARO_ICP101XX_ENABLED` = 1; no flash-size condition; hwdef `BARO` line only (no autodetect bit) |

## 8. What I did not do or verify

- I did not read the ChibiOS I2C/SPI HAL implementation (`AP_HAL_ChibiOS/I2CDevice.cpp`, `SPIDevice.cpp`) for default retry counts, the SPI low/high speed semantic, or how a 1-byte register address is formed on I2C.
- Default I2C retries for BMP085/LPS2XH are therefore unstated, not zero.
- I did not examine `Tools/scripts/build_options.py` or the custom-build server (not in the sparse clone), which can disable `AP_BARO_*_ENABLED`/features for size.
- No datasheets were consulted. Every claim about what a register or bit means on the silicon is INFERRED from names and comments in the source.
- hwdef examples are limited to the boards I added: CubeOrange, CubeBlack, Durandal (an H743 board, but its only baro is an MS5611, so no matching lines), mini-pix, crazyflie2, CUAV_GPS, speedybeef4. I found no H7 board that uses LPS2XH, BMP085 or ICP101XX. Those baro examples are F4 boards, and the CUAV_GPS one is an F412 periph board.
