# AET-WO-4210 — FEEDBACK (spike)

Builder: Claude Code (Opus 5.5) in `~/aethelgard-4210`, branch `wo/aet-4210-imu-baro`, cut from `main` at `c621461`. The work order is read from `origin/claude/argon-repo-docs-synthesis-rjrb0d` at `733bbca` (docs PR #127); it is not committed here.

Labels: **measured** (a run), **read** (at source, in a datasheet, or on a vendor page, cited), **inferred** (reasoned from what was read, not run).

## Predictions — written before any run

Committed alone, before any container started. Each rests on what was read, cited; none on a run of this work order.

### What the predictions rest on

**Renode, read at `renode-infrastructure` `066a7f13`** (`~/.cache/aet-2301/src/infra`, the pinned image's commit), under `src/Emulator/Peripherals/Peripherals/`:

- **The ICM-20948's SPI path, `Sensors/ICM20948/ICM20948.cs` `Transmit` (L119–158).** The first byte selects the register (bits 0–6) and the direction (bit 7).
  - A read logs `Preparing for read at bank #{0} from 0x{1:X} ({2})` (L131, Noisy).
  - A write logs `Preparing for write at bank #{0} to 0x{1:X} ({2})` (L136, Noisy).
  - Each later byte of a read logs `Read at bank #{0} from 0x{1:X} ({2}), returned 0x{3:X}` (L142, Noisy) and increments the register.
  - Each later byte of a write logs `Write at bank #{0} to 0x{1:X} ({2}): value 0x{3:X}` at **Debug** (L147) and writes the register.
  - Every byte also logs `Received byte 0x{0:X}, returning 0x{1:X}` (L156, Noisy).
  - A chip-select release logs `Finishing transmission, going idle` (L162, Noisy; already skipped by the judge, `model.rs`).

  `{2}` is the bank's register enum name (`ICM20948_Bank0Registers.cs` L274 on): `WhoAmI` = `0x0`, `PowerManagement1` = `0x6`, `InterruptEnable1` = `0x11`, `InterruptStatus1` = `0x1A`, `AccelerationXOutHigh` = `0x2D`, and `RegisterBankSelection` = `0x7F`.
- **Tags warn on every write that sets them.** The register framework computes `difference = UnderlyingValue ^ value` (`Main/Core/Structure/Registers/PeripheralRegister.cs` L593).
  - Bits outside every field are `unhandledWrites = difference & ~definedFieldsMask` (L682). `definedFieldsMask` holds fields only, not tags (L838–846).
  - It logs `Unhandled write to offset 0x{2:X}. Unhandled bits: [{1}] when writing value 0x{3:X}.{0}` with ` Tags: NAME (0xV).` at **Warning** for a non-silent tag (L726–760).
  - A tag's bits are never stored, so the warning repeats on every such write.
- **`PWR_MGMT_1` in the model** (`ICM20948_Bank0Registers.cs` L38–45):
  - `DEVICE_RESET` is a write-only field that calls `SoftwareReset()`.
  - `SLEEP`, `LP_EN`, `TEMP_DIS` and `CLKSEL` are tags.
  - The register has no reset value, so it reads 0.

  **Silicon resets it to `0x41`**, with `SLEEP` set (DS-000189 1.6 §8.4, "Reset Value: 0x41"). So the model needs no wake write before data, and the part does. A firmware that never clears `SLEEP` passes in the twin and reads zeros on the bench (inferred).
- **Data, `ICM20948_Accelerometer.cs`.**
  - `AccelerationX/Y/Z` is the feeder's sample, or else `DefaultAccelerationX/Y/Z` (L29–75), a settable property that is 0 unless set.
  - The raw value is `round(g × sensitivity)` clamped to `i16` (L188–192, `ICM20948.cs` L273–279).
  - The sensitivity is 16384 LSB/g at ±2 g, 8192 at ±4 g, 4096 at ±8 g and 2048 at ±16 g (L144–158), the datasheet's own scale (inferred: the table is the datasheet's, not yet cited by page).
  - The data registers do not depend on `PWR_MGMT_1` or on any sample rate. Nothing is random.
- **Data-ready, `ICM20948.cs` L216–244 and `ICM20948_Bank0Registers.cs` L74–116.**
  - `IRQ` is a `GPIO`, set to the OR of seven enable-and-status pairs, recomputed only when an enable or status register changes.
  - `RAW_DATA_0_RDY_INT` (`INT_STATUS_1` bit 0) is a plain flag. No line of the model sets it (grep: L114, L227, L267 only), so no sample, feeder or timer raises it.
  - Only a write of `INT_STATUS_1` itself sets it: a firmware store over SPI.
- **The multiplexer, `SPI/SPIMultiplexer.cs`.**
  - Every chip-select edge logs `GPIO #{0} set to {1}` (L30, Noisy), with the raw level: `False` is asserted, because the inputs are active-low.
  - A deassert logs `Finisihing transmission on device 0x{0:X}` (L107, Noisy, sic).
  - A byte to an empty address logs `Tried to transmit byte 0x{0:X} to device 0x{1:X}, but it's not connected - …` (L69, Warning).

**`main`, read at `c621461`.**
- `crates/aethelgard-twin/src/model.rs` `witness` knows exactly one transaction per model, the identity read. Anything else is `None` (`i2c_witness_unavailable`/`spi_witness_unavailable`).
- `crates/aethelgard-cli/src/renode.rs` collects each device's `[NOISY] <device>: ` lines over the whole run (`device_line`, L617). It collects the multiplexer's `[WARNING]` lines per controller (`unrouted_line`, L529).
- The board crate's `transfer` already performs a write-only step (an empty read buffer) and a write-then-read step on either bus (`boards/sentinel-01/src/board.rs` L391 and L508).

### P1. The ICM-20948's script, through the generated board crate (`fw.py imu-script`)

The script: the identity read `[0x80]` → 1 byte; the configuration write `[0x06, 0x01]` (`PWR_MGMT_1`: `CLKSEL` 1, `SLEEP` 0, ArduPilot's wake value: P5); the data read `[0xAD]` → 6 bytes (`ACCEL_XOUT_H` to `ACCEL_ZOUT_L`).

| # | Prediction | Rests on |
|---|---|---|
| P1.1 | The identity read logs the 4 lines `model.rs` already expects, unchanged by the steps after it. | `Transmit`; 4200 measured |
| P1.2 | The configuration write logs, for `spi1.spi1mux.u5`, exactly 3 Noisy lines: `Preparing for write at bank #0 to 0x6 (PowerManagement1)`, `Received byte 0x6, returning 0x0`, `Received byte 0x1, returning 0x0`. **The value written appears only in a `[DEBUG]` line** (`Write at bank #0 to 0x6 (PowerManagement1): value 0x1`). The Noisy pair still carries it, as the second `Received byte`. | L136, L147, L156 |
| P1.3 | The same write logs one `[WARNING]` from `u5`: `Unhandled write to offset 0x6. Unhandled bits: [0] when writing value 0x1. Tags: CLKSEL (0x1).` This is a warning in a correct run. | `PeripheralRegister.cs` L682, L755 |
| P1.4 | The data read logs 2 + 2 × 6 = 14 Noisy lines: `Preparing for read … from 0x2D (AccelerationXOutHigh)` and `Received byte 0xAD, returning 0x0`, then, per byte, `Read at bank #0 from 0x{2D..32} ({name}), returned 0x0` and `Received byte 0x0, returning 0x0`. With no default set, every byte is `0x0`. | L131, L142, L156; `DefaultAcceleration*` 0 |
| P1.5 | With `DefaultAccelerationZ` set to 1 (g) by the platform before the run, `ACCEL_ZOUT_H`/`_L` return `0x40`/`0x0`. That is 16384 at ±2 g; the other four bytes are `0x0`. The value is the stimulus's, not the firmware's. | L188–192, L144–158 |
| P1.6 | Between steps, `u5` logs one `Finishing transmission, going idle` per chip-select release, and nothing else. A correct script on U5 logs no `[ERROR]`, and no `[WARNING]` beyond P1.3. | L162 |
| P1.7 | Three runs give byte-identical device lines. | `SetSeed`, no randomness in the model |

**Line counts, which size kinds 13 and 22 (F18):** the identity read is 4 lines, the configuration write 3 and the 6-byte read 14, so 21 for this script, each line under 100 bytes. A step reading `n` bytes logs `2 + 2n` lines. The budget belongs to the script, not 16 × 256 per read (inferred).

### P2. Forgeries, one per new step type

| # | Prediction |
|---|---|
| P2.1 | `imu-forge-config`: PA1 driven low then high by plain stores, and a record claiming the write. Logs the multiplexer's `GPIO #1 set to False`/`True` and `u5`'s `Finishing transmission, going idle`, and no `Preparing for write`. So the step is unwitnessed. |
| P2.2 | `imu-forge-data`: the write performed, the data step claimed with no transfer. No `Preparing for read … 0x2D`, so the step is unwitnessed. The configuration step's 3 lines are present, so a step-wise judge names the data step, not the device. |

### P3. Per-read windows on SPI (F16; `fw.py two-windows`, and the same with U4 absent)

| # | Prediction |
|---|---|
| P3.1 | Every transfer line of a step lies between its own select's `GPIO #k set to False` and the next `GPIO #k set to True` from `spi1.spi1mux`, in log order. With the CLI's synchronous logging (`2204` §2.20), no line of another step falls inside. |
| P3.2 | With U4 absent, each `… to device 0x0, but it's not connected …` line lies inside a `GPIO #0` window. None lies inside a `GPIO #1` window. |
| P3.3 | The window lines are forgeable (P2.1), but a forged edge only adds an empty window or splits none: it cannot move a transfer line or a `not connected` line from one select's window into another's. So windows attribute; they never witness (inferred). |

### P4. Data-ready (S3; `fw.py imu-drdy`, `imu-drdy-store`)

| # | Prediction |
|---|---|
| P4.1 | `imu-drdy` (`INT_ENABLE_1` = 1, then the data read, then `INT_STATUS_1`) leaves `u5`'s `IRQ` unset for the whole run, and the status read returns `0x0`. **The model never drives data-ready on its own.** |
| P4.2 | `imu-drdy-store` (`INT_ENABLE_1` = 1, then `INT_STATUS_1` = 1 written over SPI) sets `IRQ`. The only data-ready the model can raise is one the firmware writes, so an interrupt the judge learned from it would witness a store (inferred). |

### P5. ArduPilot's bring-up against the models

These are read, not predicted. They are recorded here because the probes' script follows them (`spike/4210/s1-ardupilot.md`, Copter-4.7.2 `fb77356`, spot-checked by me at the lines cited):
- **ICM-20948.** `AP_InertialSensor_Invensensev2` accepts `WHO_AM_I` `0xEA` (`_registers.h` L218, `.cpp` L672).
  - It resets with `PWR_MGMT_1` = `0x80`, wakes with `0x01`, and disables I2C with `USER_CTRL` `I2C_IF_DIS` on SPI.
  - It configures bank 2 through `REG_BANK_SEL`, and enables `INT_ENABLE_1`.
  - It then reads the **FIFO** (`FIFO_COUNTH` `0x70`, `FIFO_R_W` `0x72`) on a timer, polled.
  - A DRDY pin is used only under `#ifdef INVENSENSEV2_DRDY_PIN` (L131), in the probe's retry loop.
- **The model's FIFO is not implemented.** `FIFO_CNT` and `FIFO_R_W` are tags that read 0 (`ICM20948_Bank0Registers.cs` L228–239, read by me). So ArduPilot's steady-state read would get no sample in the twin (inferred).
  - The probe's data step is therefore an `ACCEL_XOUT_H` burst, which the model implements.
  - ArduPilot's own read path is a fidelity gap, and it does not block this work order, because ArduPilot is not in the twin.

### P6. The barometers, on the reference I2C bus (`I2C1`, beside U2)

Read at source (`spike/4210/s1-models.md`, spot-checked by me at `LPS25HB.cs`, `BMP180.cs`, `ICP_101xx.cs`):
- **The LPS25HB** logs nothing on `Read` except warnings (L28–38). It logs one **Debug** line per `Write` (L174).
- **The ICP-101xx** logs every command and read at **Debug** (L61, L75). Its one Noisy line, `Registered command 0x{0:X}.` (L303), is written at construction, not per transaction.
- **The BMP180** logs every step at **Noisy** (L41–66, L173, L198, L217).

| # | Prediction |
|---|---|
| P6.1 | **LPS25HB at `0x5D`:** identity `0x0F` → `0xBD`, `CTRL_REG1` = `0x90`, `PRESS_OUT_XL` `0xA8` → 3 bytes. Not one `[NOISY]` line from the device in the whole script; one `[DEBUG] Writing 0x… to register 0x…` per write; values `0xBD`, then 3 zero bytes. **It cannot be witnessed under today's rule, not even its identity.** |
| P6.2 | **ICP-101xx at `0x63`:** identity command `EF C8` → 3 bytes `00 08 <crc>`; measurement command `48 A3` then a 9-byte read. Per transaction, only `[DEBUG] Received command: [0xEF, 0xC8]` and `[DEBUG] Reading data with CRC: [0x0, 0x8, 0x38], current command: 0xEFC8 (Id), requested: 3 bytes`. Witnessable only if the judge reads this model's Debug lines. The data read's line carries the bytes. |
| P6.3 | **BMP180 at `0x77`:** identity `0xD0` → `0x55`; configuration `[0xF4, 0x2E]` (a temperature conversion); data `0xF6` → 2 bytes. Noisy lines per step: identity 4 (`Write with 1 bytes of data: [0xD0]`, `Preparing to read register ChipID (0x…D0)`, `Reading 1 bytes …`, `Read value 85 …`); configuration 4 (`Write with 2 bytes of data: [0xF4, 0x2E]`, `Writing 0x2E to register CtrlMeasurement (0x…F4)`, `HandleMeasurement set Temperature`, `Conversion is complete`); data 2 + 2 × 2 − 1 = 5 (auto-increment lines between bytes). Values print in **decimal** (`Read value 85`); enum hex is padded to 8 digits (the census's inference, unconfirmed). Zero warnings. |
| P6.4 | None of the three barometer models has a GPIO or IRQ (census §E), so S3 is the IMU's alone. |

**The I2C judge reads `[NOISY] <device>: ` lines only** (`renode.rs` `witness_shaped`), so P6.1 and P6.2 predict `i2c_witness_unavailable` for the LPS25HB and the ICP-101xx under today's rule, whatever the firmware does.
