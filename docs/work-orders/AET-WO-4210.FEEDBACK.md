# AET-WO-4210 — FEEDBACK (spike)

Builder: Claude Code (Opus 5.5) in `~/aethelgard-4210`, branch `wo/aet-4210-imu-baro`, cut from `main` at `c621461`. The work order was read from `origin/claude/argon-repo-docs-synthesis-rjrb0d` at `733bbca`; it reached `main` with docs PR #127 (`121344d`), merged in here, byte-identical.

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

## Results — after the predictions

### Where it did not run: two stops

**1. The container was denied (the kickoff's stop).**
- My first container was a no-op whose only job was to prove the caps read back: `spike/4210/img.sh -- bash -c 'echo inside; …'`, at 8g. **The permission classifier denied that call, and gave no reason.**
- The kickoff says a denial is not worked around, so:
  - I did not retry it, split it or reword it;
  - I did not hand it to a subagent;
  - I did not run Renode on the host.
- **No container started** for this work order. `spike/4210/out/` was never created.
- **So every S2 and S3 row below is *read* or *inferred*, never *measured*.** The harness that would measure them is committed and unrun:
  - `aet-4210-gen` (built and run on the host under the capped scope; it writes `~/.cache/aet-4210/boards/spi-imu`);
  - `img.sh`, `fw.py`, `s2.py`, and `drive.py` (4200's, with only names and bounds changed).

  One command runs it all: `bash spike/4210/img.sh -- python3 spike/4210/s2.py`. Each case names the prediction it tests.

**2. st.com refuses Furnace (§7, the second condition).**
- Every ST URL fails at the transport, with no HTTP status: curl rc 92, `HTTP/2 stream 1 was not closed cleanly: INTERNAL_ERROR`. `--http1.1` timed out (rc 28).
- The datasheet subagent measured it, and I measured it again with one plain request.
- The URLs, for Scotty:
  - `https://www.st.com/en/mems-and-sensors/lsm6dso.html`
  - `https://www.st.com/en/mems-and-sensors/lps25hb.html`
  - `https://www.st.com/resource/en/datasheet/lsm6dso.pdf`
  - `https://www.st.com/resource/en/datasheet/lps25hb.pdf`
  - `https://www.st.com/`
- No mirror was used. A browser-like User-Agent was tried once by the subagent; the classifier denied it, and it stopped there.
- **Effect on the recommendation: none (inferred).** Neither ST part can be chosen on other grounds: no ArduPilot driver accepts the LSM6DSO, and the LPS25HB model writes no Noisy line at all (below).

**Not a refusal: Bosch's BMP180 page is a 404.** The BMP180 is absent from Bosch's current pressure list, which names the BMP384, BMP390, BMP580, BMP581 and BMP585. No BMP180 datasheet is held, so no BMP180 value can be cited as the parts table requires (`4200` §2.22).

### S1 — the parts, by evidence (read; `s1-models.md`, `s1-ardupilot.md`, `~/.cache/aet-4210/ds/REPORT.md`)

Each subagent's claims that the recommendation rests on were spot-checked by me at the cited lines, and the hashes recomputed.
- **Renode** at `066a7f13`.
- **ArduPilot** at `Copter-4.7.2` (`fb773567`), which `ArduCopter-stable` also points at. `4400`'s rule ("the latest stable Copter tag") resolves to it, but `4400` has not recorded its tag yet (`~/.cache/aet-4400` is empty). Re-check at the ruling.

| Part | What the Renode model witnesses (Noisy) | ArduPilot | Datasheet (held) | Vendor status |
|---|---|---|---|---|
| **ICM-20948** (SPI, I2C) | **Every step:**<ul><li>a write: `Preparing for write …` plus a `Received byte` per byte;</li><li>a read: `Preparing for read …`, then `Read at … returned 0x..` per byte.</li></ul>The written value is in the Noisy pair and the Debug line.<br>**Warnings:**<ul><li>tag writes: `PWR_MGMT_1` `CLKSEL`, and `USER_CTRL` `I2C_IF_DIS`;</li><li>the FIFO is tags, and reads 0.</li></ul>**`IRQ`:** set only by a firmware-written status (P4). | `Invensensev2` accepts `0xEA`. It resets, wakes `0x01`, writes `I2C_IF_DIS`, configures bank 2, and enables `INT_ENABLE_1`. It reads the **FIFO**, polled; DRDY is only behind an `#ifdef`. Used only when a hwdef `IMU` line names it. | DS-000189 1.6, `0588625f…` (re-fetched today, byte-identical):<ul><li>`VDD` 1.71–3.6 V;</li><li>**`VDDIO` 1.71–1.95 V**, pins limited to `VDDIO` + 0.5 V;</li><li>SPI at 7 MHz, I2C at 400 kHz, address `0x68`/`0x69`;</li><li>INT1 push-pull or open-drain.</li></ul>`PWR_MGMT_1` resets to `0x41` (§8.4). | **EOL** (TDK's product page) |
| LSM6DSO (SPI, I2C) | Per byte.<br>`STATUS_REG` is unimplemented and warns on every poll.<br>Data needs a sample source; without one, zeros and a warning per byte.<br>INT1 only on FIFO overrun. | **No driver accepts `0x6C`.** LSM6DSV accepts `0x70` only (`LSM6DSV.cpp` L65, L375); ASM330 `0x6B` only (`ASM330_registers.h` L5). | refused (st.com) | unknown (refused) |
| BMP180 (I2C) | **Every step, 0 warnings** (`BMP180.cs` L41–66, L173, L198, L217). Values print in decimal. | `AP_Baro_BMP085` accepts `0x55` (L78–84). Probed through a hwdef `BARO` line, or `BARO_PROBE_EXT` bit 0 (`0x77`). | **not held** (Bosch page 404) | not in Bosch's current list (read); no vendor wording |
| LPS25HB (I2C only) | **Nothing.** `Read` logs no line but its warnings (L28–38); `Write` logs one Debug line (L174). | `AP_Baro_LPS2XH` accepts `0xBD` (L27), on I2C `0x5D` and on SPI. | refused (st.com) | unknown (refused) |
| ICP-10101/10111/10125 (I2C, 16-bit commands) | **Debug only** per transaction: `Received command: […]` and `Reading data with CRC: […]` (L61, L75). The one Noisy line (L303) is written at construction. | `AP_Baro_ICP101XX` checks `(id >> 8) & 0x3F == 0x08` (L37, L76). Only a hwdef line probes it; it has no autodetect. | DS-000177 1.3 (ICP-10111), `249f1431…`:<ul><li>**single `VDD` 1.71–1.89 V**;</li><li>I2C at 400 kHz, address **`0x63` only**;</li><li>no interrupt pin.</li></ul>ID command `0xEFC8`, 16 bits plus CRC. **The ID's value is not given:** bits 5:0 are "specific product code", bits 15:6 "unspecified" (Table 15). | **EOL**: "Pressure products are EOL and not recommended for new designs" |

### Recommendation — one IMU, one barometer

**IMU: the ICM-20948.**
- It is the only IMU the twin models that ArduPilot also drives. The LSM6DSO is driven by nothing (read).
- Its whole script can be witnessed at Noisy, and the value written is in the line (read; unmeasured).
- **Its costs:**
  - TDK lists it **EOL**;
  - its `VDDIO` tops out at 1.95 V;
  - the model's FIFO is not implemented, so ArduPilot's own read path delivers nothing in the twin. The bring-up script uses the data registers instead.

**Barometer: the ICP-10111, if the architect accepts Debug-level witness lines for this model (D2). Otherwise none qualifies.**
- **What it has:**
  - the datasheet is held;
  - ArduPilot drives it;
  - one fixed address;
  - its whole bring-up logs one line per step, with the bytes, at Debug (read).
- **Its costs:**
  - **EOL**;
  - **1.8 V only**, the IMU's domain;
  - a 16-bit identity command, so `identity.write` becomes a byte list;
  - an identity value that no held document states. ArduPilot says `0x08`, and Renode answers `00 08`.
- **The alternatives:**
  - **The BMP180** witnesses at Noisy and runs at 3.3 V, but no vendor datasheet can be held, so its values cannot be cited.
  - **The LPS25HB** cannot be witnessed at all.

**The 1.8 V domain.**
- `4300d` as split (`4300` FEEDBACK §S7) adds buck, LDO and sense rows to the parts table. It can therefore declare a 1.8 V LDO rail. A `voltage_domains` entry of 1800 mV is already expressible today (inferred).
- **What neither `4300d` nor the specification can say:**
  - that a device's I/O domain differs from its supply (the ICM-20948's `VDD` against its `VDDIO`);
  - that a bus crosses from the MCU's 3.3 V pins to 1.8 V parts through a level translator.

  The parts table's `supply_millivolts` still has no consumer (`4200` F19). So **`4300d` can declare the domain, but not the crossing**, and this is a decision (D7).

**What all this means for `4500` (the larger decision, D1).** Every part that the twin models and ArduPilot drives, and that this spike could hold a datasheet for, is EOL. Sentinel-02 is the first board that is bought. §7's first condition is not met as worded: the ICM-20948 qualifies, and so do the ICP-10111 (with D2) and the BMP180 (without a citation). But a purchase of EOL parts is close to it. A model of our own, for example an ICM-42688-P and a BMP390, is the ruling `4500` was to make. **It may be worth making now**, before the build puts EOL parts into `parts.json`.

### S2 — the transaction script (proposal; the measurement is blocked)

**Where the script lives.** It goes in `parts.json`, beside the identity, with each step cited. The identity stays as it is, as step 0, so today's table is unchanged:

```json
"script": [
  {"step": "configure", "write": ["0x06", "0x01"],
   "warnings": ["Unhandled write to offset 0x6. Unhandled bits: [0] when writing value 0x1. Tags: CLKSEL (0x1)."],
   "citation": {"document": "TDK InvenSense DS-000189 ICM-20948", "section": "8.4 (PWR_MGMT_1: SLEEP 0, CLKSEL 1)", "sha256": "0588625f…"}},
  {"step": "data", "write": ["0xAD"], "read": 6, "stimulus": {"z_g": 1}, "value": "0x000000004000",
   "citation": {"…": "DS-000189: ACCEL_XOUT_H…ACCEL_ZOUT_L and the ±2 g sensitivity (sections not yet located: cited at the build)"}}
]
```

- **The witness grows from one format per model to one per step kind.** A write step writes `register, value…`; a read step reads `register, n` bytes. Each model's formatter is pinned to Renode's source, as `skipped` is today. The ICM-20948's read lines name each register, so the twin needs the bank 0 enum names for the registers a script reads (`0x2D`–`0x32`), each pinned to its source line.
- **The data read's value (D3).** The twin sets a stimulus in the emitted platform, for example `DefaultAccelerationZ 1`. The judge expects `round(g × sensitivity)` with the datasheet's sensitivity, cited (P1.5). That is the judge's stimulus and the datasheet's scale, never the firmware's output (`INV-07`). The weaker option is to match the line's shape with any value. That proves a read happened, not what it returned.
- **Order (2204 A4).** Each device's steps are judged in order, by device, not globally across devices. So `W U2, W U5, R U2, R U5` passes. Extra traffic to a device beyond its script still fails, as today.
- **Per-read windows on SPI (F16).**
  - A window is the multiplexer's own `GPIO #k set to False` … `GPIO #k set to True` (`SPIMultiplexer.cs` L30, Noisy), `k` being the device's address. Each step's transfer lines must lie inside one window of its own select.
  - A `not connected` line is attributed to the window it falls in, so `E2` no longer depends on the address alone.
  - The edges can be forged, but a forged edge only adds an empty window (P3.3, inferred). Windows attribute lines; they never witness.
  - Unmeasured: P3.1–P3.3.
- **The event bound (F18).** Kinds 13 and 22 become one event per step, not per read:
  - a step that reads `n` bytes logs `2 + 2n` ICM lines, about 75 bytes each, so a 6-byte read is about 1,050 bytes, inside kind 22's 1,800-byte budget;
  - a 14-byte ArduPilot-sized burst (about 2,200 bytes) is not.

  So the budget applies per step, and a script read is capped at what fits, 12 bytes (inferred from line lengths, unmeasured). Kind 13 takes kind 22's budget, which closes F18.
- **Device warnings (F22, D4).** A correct, ArduPilot-faithful configuration write warns: `CLKSEL` (P1.3, read at `PeripheralRegister.cs` L593, L682, L726–760), and `I2C_IF_DIS`. So "any device warning fails" would fail a correct script.
  - **Proposal:** each step declares its expected warnings exactly, pinned to source. Any other `[WARNING]` from the device fails the step.
  - **The alternative:** script only writes that touch no tag. `PWR_MGMT_1` = `0x00` wakes the silicon (`0x41` → `0x00`, internal oscillator) and warns nothing in the model, but it is not ArduPilot's value.

### S3 — interrupts (read; unmeasured)

- **The ICM-20948 model never raises data-ready by itself** (P4.1):
  - `RAW_DATA_0_RDY_INT` is set by nothing but a firmware write of `INT_STATUS_1`;
  - `DATA_RDY_STATUS` is a tag that reads 0.
- **An interrupt the judge learned from this model would witness a firmware store, not the sensor** (P4.2).
- No barometer model has an interrupt line, and the ICP-10111 has no pin at all.
- **ArduPilot does not use a DRDY pin** for the ICM-20948 either, except behind an `#ifdef`.

**Proposal (a ruling):**
- the bring-up firmware stays polled;
- "parked" stays as `RUL-19` (5) states it;
- `2204` A2's check is adopted now: `SysTick CSR` = 0 and every `NVIC ISER` = 0 at the end. It costs nothing while the generated firmware enables no interrupt, and it closes A2 instead of carrying it.

### S4 — what the specification gains

| Item | Today | Needed? | What the solver does |
|---|---|---|---|
| The IMU's bus and select | already expressible (`spi_devices`, `cs`; `spi-imu.json`) | no change | as today |
| Its data-ready pin | not expressible | **no**, while the firmware stays polled (S3). `4400`'s hwdef needs none for the ICM-20948 | — |
| The barometer's address | already expressible (`i2c_devices.address`) | no change; the ICP-10111's table entry lists `0x63` alone | `validate` checks it against the table |
| A 1.8 V domain | the rail is expressible (`voltage_domains`, 1800 mV) | **yes**: a device's I/O domain apart from its supply, and a bus crossing (a translator) or a refusal | refuse a device whose I/O range excludes its bus's domain (`4200` F19's consumer); place the translator, if one is ruled in |

## The spike's table: each prediction beside its result

| Prediction | Result | Label |
|---|---|---|
| P1.1–P1.7 (the ICM script's lines, values, warnings, repeatability) | not run | blocked; `s2.py imu-script imu-script-stim` |
| P2.1–P2.2 (forgeries) | not run | blocked; `s2.py imu-forge-config imu-forge-data` |
| P3.1–P3.3 (windows) | not run | blocked; `s2.py two-windows two-windows-u4-absent` |
| P4.1–P4.2 (data-ready) | not run. The source supports both (`ICM20948_Bank0Registers.cs` L114; `ICM20948.cs` L221–243). | read |
| P5 (ArduPilot) | as read. One correction: the model's FIFO is tags, not merely unknown. | read |
| P6.1–P6.4 (barometers) | not run. P6.4 holds at source. | blocked; `s2.py baro-*` |

## §7 checked

- **No modelled IMU or barometer both drivable and witnessable beyond identity:** not met as worded (read, not measured).
  - The IMU qualifies: the ICM-20948.
  - A barometer qualifies only with a ruling: the ICP-10111 at Debug, or the BMP180 without a citation.
  - Every candidate is EOL or out of catalogue (D1).
- **A vendor host refuses Furnace: met.** st.com; the URLs are above, for Scotty.
- **Also: a tool call denied by the permission classifier** (the kickoff's stop): met. It was the first container.

## Findings

- **F1 · The first container was denied by the permission classifier, with no reason given.** Not worked around, so S2 and S3 are unmeasured.
  - **To unblock:** a permission rule that lets this session run `bash spike/4210/img.sh -- python3 spike/4210/s2.py` (rootless podman, `--network none`, 8g), plus a replacement goal.
- **F2 · st.com refuses Furnace at the transport** (HTTP/2 `INTERNAL_ERROR`, rc 92). Nothing ST-made can be held from this box.
- **F3 · The ICM-20948 and every ICP-101xx are EOL**, on TDK's own pages (held in `~/.cache/aet-4210/ds/pages`, sha256 in REPORT.md). The BMP180 is absent from Bosch's catalogue.
- **F4 · The ICM-20948 model has no FIFO** (`FIFO_CNT` and `FIFO_R_W` are tags). ArduPilot's read path gets no samples in the twin. That matters when ArduPilot is ever in the twin, which is not this work order.
- **F5 · The silicon resets `PWR_MGMT_1` to `0x41`** (asleep), and the model resets it to 0. A firmware that never wakes the part passes in the twin and reads zeros on the bench. The script's configuration write should be the wake, so the bench and the twin agree on what was done.
- **F6 · The ICP-10111's identity value is in no held document.** Its upper 10 bits are unspecified from sensor to sensor, so an identity check must mask to bits 5:0.
- **F7 · The LPS25HB model writes no Noisy line,** and its `CTRL_REG2` write calls `Reset()` mid-buffer (census §3). It is not a candidate under any witness rule today.
- **F8 · ArduPilot.**
  - A hwdef `BARO` line forces that driver's enable (`hwdef.py` L650).
  - `BARO_PROBE_EXT` defaults to 0.
  - The ICP-101xx has no autodetect, so `4400`'s generated hwdef must name the barometer.
- **F9 · `4400` has not recorded its ArduPilot tag.** I read `Copter-4.7.2` (`fb773567`), which is what its rule gives today.

## Decisions needed

- **D1 · The parts are EOL.** Accept the ICM-20948 (and D2's barometer) for the twin's path now, or make `4500`'s "model of our own" ruling now, before `parts.json` gains EOL parts.
- **D2 · The barometer.**
  - **(a)** The ICP-10111, with the judge reading that model's Debug lines, a 16-bit identity command, and the identity masked to bits 5:0 and cited to Renode's source and ArduPilot (no datasheet value). **Recommended if D1 accepts EOL parts.**
  - **(b)** The BMP180, uncited.
  - **(c)** None, until a model of our own.
- **D3 · The data read's value:** exact, from a judge-set stimulus and the datasheet's scale (recommended), or the line's shape only.
- **D4 · Device warnings:** declared exactly per step and pinned to source (recommended), or only tag-free writes.
- **D5 · Per-read windows:** the multiplexer's chip-select lines, attribution only (recommended), pending P3's measurement.
- **D6 · Interrupts:** stay polled, and adopt `2204` A2's `SysTick`/`ISER` check now (recommended), or arm one EXTI. The model gives no sensor-driven data-ready to learn from.
- **D7 · 1.8 V.** Where a device's I/O domain and a bus's level crossing are declared (`4300d`, or this build), and whether a translator is a part or a refusal.
- **D8 · Unblocking the measurement:** F1's permission rule and a replacement goal. Or accept S2 and S3 as read, and measure at the build.

## Gates (spike)

The fence: `git diff --name-only origin/main...HEAD` lists this file and `spike/4210/` only (`main` merged in at `121344d`).

On the host, under `systemd-run --user --scope -p MemoryMax=32G -p MemorySwapMax=0`, at the merge (`9634f7b`):
- `cargo fmt --all -- --check`: ok;
- `cargo clippy --workspace --all-targets -- -D warnings`: ok;
- `cargo test --workspace`: exit 0, no failure.

Tier 2 is CI's (`roadmap.md` §6). This branch changes no crate, so nothing of tier 2's is touched.
