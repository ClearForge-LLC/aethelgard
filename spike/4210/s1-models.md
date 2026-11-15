# AET-WO-4210 S1: Renode sensor models, read from source

Source: renode-infrastructure at `066a7f13c` (verified with `git rev-parse --short HEAD`), tree `/home/forge/.cache/aet-2301/src/infra`. Nothing was run; every behaviour below is read from C#. Anything not read directly is labelled INFERRED.

Path abbreviations used in citations:
- `S/` = `src/Emulator/Peripherals/Peripherals/Sensors/`
- `REG/` = `src/Emulator/Main/Core/Structure/Registers/`
- `LOG/` = `src/Emulator/Main/Logging/Logger.cs`

Level notation: [N] Noisy, [D] Debug, [I] Info, [W] Warning, [E] Error. A log run at Noisy level prints every level, so "lines Renode would print at Noisy" means all N+D+W+E lines. The logger's own line prefix (timestamp, `[LEVEL]`, peripheral name) was not read and is not reproduced.

## 0. Shared base-class behaviour (applies to all five)

**0.1 Log helpers.** `NoisyLog/DebugLog/InfoLog/WarningLog/ErrorLog(this IEmulationElement, fmt, args)` and `Log(this IEmulationElement, LogLevel, fmt, args)` are plain wrappers over `LogAs` at the stated level (`LOG:181-326`). The wrappers themselves do not dedupe. **The logger does**: `LogAggregator` collapses consecutive entries that are equal in level, thread, source and message (`LogEntry.EqualsWithoutIdTimeAndCount`, `src/Emulator/Main/Logging/LogEntry.cs:31-38`; loop `LOG:735-770`) into the first line plus a later flush carrying a repeat `Count` (`LOG:800-812`, up to `MaxRepeatedLogs` = 10000 `LOG:920`). It is on by default (`general` / `collapse-repeated-log-entries`, default `true`, `LOG:825`). So two byte-for-byte identical adjacent lines appear once in the log (with a count), not twice. In the transcripts below no two adjacent lines are identical, because the per-byte lines alternate with differently-worded or differently-addressed lines, except where noted. How a backend prints the `Count` was not read.

**0.2 Hex byte lists.** `ToLazyHexString()` and `Misc.PrettyPrintCollectionHex(x)` print `[0xA, 0xB]`, with each element as `0x{0:X}` (no zero padding), and `[]` for an empty collection (`src/Emulator/Main/Utilities/Misc.cs:210,220-230`, `Misc.cs:2001-2014`). `{0:X}` on a byte 0 prints `0`; `{0:X2}` prints `00`.

**0.3 Unhandled register access (register collections).**
- A read of an offset with no register defined: `BaseRegisterCollection.Read` calls `parent.LogUnhandledRead(offset)` (`REG/RegisterCollection.cs:175-183`) and returns 0.
- A write to an offset with no register: `parent.LogUnhandledWrite(offset, value)` (`REG/RegisterCollection.cs:228-233`).
- Format strings (`LOG:452-468`):
  - [W] `"Unhandled read from offset 0x{0:X}{1}."`
  - [W] `"Unhandled write to offset 0x{0:X}{1}, value 0x{2:X}."`
  - `{1}` is ` (<name>)` only if the peripheral implements `IHasMappedRegisters`. None of the five does (ICM20948 `ICM20948.cs:21`, LSM6DSO `LSM6DSO_IMU.cs:24` via `BasicBytePeripheral` which does not implement it `BasicBytePeripheral.cs:16`, BMP180 `BMP180.cs:18`, LPS25HB `LPS25HB.cs:19`). So `{1}` is empty. Example: `Unhandled read from offset 0x1E.`
- ICP-101xx has no register collection.

**0.3a Writes to bits that are tags or reserved (the important one).**
- `WriteInner` (`REG/PeripheralRegister.cs:590-692`) computes `difference = UnderlyingValue ^ value` and `unhandledWrites = difference & ~definedFieldsMask` (`:682`), where `definedFieldsMask` is the union of real fields only (`:838-846`). Tags (`WithTag`, `WithTaggedFlag`, `WithReservedBits`) are not fields.
- If `unhandledWrites != 0`, `LogUnhandledWrites` (`:726-738`) emits one [W] line, using the non-silent tags, with this exact format (`:755`):
  `"Unhandled write to offset 0x{2:X}. Unhandled bits: [{1}] when writing value 0x{3:X}.{0}"`
  where `{0}` = `" Tags: <NAME> (0x<val>), ..."`, `{1}` = pretty set-bit list (`BitHelper.GetSetBitsPretty`: `"0"`, `"4"`, `"0-2"`, `"(none)"`), `{2}` = register offset, `{3}` = full byte or word written.
- A silent tag would be [N] (`:733-737`). No tag in these five models passes `silent: true`.
- **The warning fires only for bits that CHANGED relative to the stored value.** Writing 0 over a 0 reset value is silent. Writing a 1 into a tag-only bit is a Warning every time, because the stored value never changes: tags are not stored (`WriteInner` only updates `UnderlyingValue` for fields).
- Consequence (read from the code path, `:590-640`): a **tag-only bit reads back as its reset value (0)** after being written. A driver that writes a config register and reads it back to verify will see tag bits as 0.
- Reserved bits are tags named `RESERVED` with no `allowedValue` (`PeripheralRegister.cs:302-322` via `WithReservedBits` `PeripheralRegisterExtensions.cs:445-449`). So a 1 written to a reserved bit logs `... Tags: RESERVED (0x1).`
- Bits that are neither field nor tag are silently ignored (no log). Only LPS25HB `RES_CONF` and a few others have such bits.
- `Error` at `:690` fires only for tags with `allowedValue`. None of these models set one, so it never fires here.

**0.4 Tag/field read.** Reads do not log. A register with only tags reads as its reset value (Define's `resetValue`, default 0).

**0.5 Not used.** `SensorSamplesFifo<T>` (`src/Emulator/Main/Peripherals/Sensor/SensorSamplesFifo.cs`) is not referenced by any of the five models (grep). All five use either RESD streams with `StartSampleFeedThread` (`src/Emulator/Main/Utilities/RESD/RESDStream.cs:91-115`) or plain properties. RESD-only logs are [D] `"RESD: ..."` lines (`RESDStream.cs:50,75,337-493`) and appear only if a RESD file is attached.

**0.6 Who calls `FinishTransmission`.** Controller dependent, not part of these models. Examples that do call it: `MAX32650_SPI.cs:143`, `Quark_SPI.cs:171`, `HiFive_SPI.cs:212-216`, `LiteX_SPI_Flash.cs:51` under `src/Emulator/Peripherals/Peripherals/SPI/`. INFERRED: the target controller must drive it at CS deassert (SPI) or STOP (I2C). Whether an I2C controller calls it between the address write and the read (repeated START) matters for LSM6DSO, LPS25HB and ICM20948; see each section.

---

## 1. ICM-20948 (`S/ICM20948/*.cs`)

### 1A. Bus and interface
- Class: `public partial class ICM20948 : II2CPeripheral, ISPIPeripheral, IGPIOReceiver, IProvidesRegisterCollection<ByteRegisterCollection>, IPeripheralContainer<II2CPeripheral, NumberRegistrationPoint<int>>` (`ICM20948.cs:21`). Also `ITemperatureSensor` (`ICM20948_Thermometer.cs:14`) and `IUnderstandRESD` (`ICM20948_Accelerometer.cs:16`, `ICM20948_Gyroscope.cs:16`).
- Supports **both SPI and I2C** on the same state machine (`state`, `selectedRegister`).
- It is also an I2C **container** for downstream slaves (the AK09916 magnetometer path, `ICM20948.cs:49-53`, Bank 3 `ICM20948_Bank3Registers.cs:18-56`). Not relevant unless a magnetometer is registered.
- CS input: `IGPIOReceiver.OnGPIO` (`ICM20948.cs:166-181`): only pin 0; on CS deassert (`previousChipSelected && !chipSelected`) calls `ISPIPeripheral.FinishTransmission()`.
- Interrupt output: `public GPIO IRQ { get; }` (`ICM20948.cs:216`, created `:26`).

**SPI decode** (`Transmit`, `ICM20948.cs:119-158`):
- State `Idle` first byte: `selectedRegister = BitHelper.GetValue(data, 0, 7)` (low 7 bits); `isRead = BitHelper.IsBitSet(data, 7)` (bit 7 = 1 is a read). So the address byte is `0x80|reg` for a read, `reg` for a write (`:126-127`).
- Read: state `Reading`; each later byte returns `RegistersCollection.Read(selectedRegister)` then `selectedRegister++` (`:140-144`). The `byte` increment wraps at 0xFF but register address space is 7 bits; INFERRED no masking, irrelevant here.
- Write: state `Writing`; each later byte does `RegistersCollection.Write(selectedRegister++, data)` (`:146-149`).
- **Auto-increment is unconditional** in both directions. There is no `IF_INC` bit. A burst read from 0x2D walks 0x2D, 0x2E, ... until CS.
- `ISPIPeripheral.FinishTransmission` (`:160-164`): [N] log, `state = Idle`.
- If a SPI `Transmit` arrives while `state` is `ReceivedFirstByte`, `WaitingForAddress` or `WritingWaitingForValue`: [E] line 152 (`default:` branch). Those states are only reachable via I2C `Write` (see below), so mixing I2C and SPI without a finish would trigger it.

**I2C decode** (`Write`/`Read`, `ICM20948.cs:55-109`):
- `Write(data)` in `Idle`: `selectedRegister = data[0]` (no R/W bit; full byte is the register address), state `ReceivedFirstByte`; any further bytes are written to successive registers (`:62-72`, loop `:78-81`), then state `WritingWaitingForValue`.
- `Write(data)` in `ReceivedFirstByte`/`WritingWaitingForValue`: all bytes are data (`:74-85`).
- `Write` in `Reading`: [W] line 89, ignored. `Write` in `Writing` or `WaitingForAddress`: [E] line 93 (`default:` branch).
- `Read(count)`: sets state `Reading` unconditionally (`:100`), reads `count` registers starting at `selectedRegister`, auto-incrementing (`:105`).
- `II2CPeripheral.FinishTransmission` (`:111-117`): **no log**. If `state != ReceivedFirstByte` then `state = Idle`. When the last transfer was an address-only write (state `ReceivedFirstByte`), a STOP leaves the state in place so a following read still works. This is deliberate (comment `:113`). So this model tolerates both STOP-then-read and repeated-START.

**Bank select:**
- `REG_BANK_SEL` (0x7F) is defined in every bank's collection by `DefineBankSelectRegister` (`:263-271`): `WithValueField(4, 2, writeCallback: (_, value) => userBankSelected = value, name: "USER_BANK")`, reserved (6,2) and (0,4).
- `RegistersCollection => userBankRegisters.GetOrDefault(userBankSelected, userBankRegisters[0])` (`:214`). Bank switches take effect on the very next register access.
- The `userBankSelected` variable is **not** reset by `SoftwareReset` (`:202-212`); only the register collections are. INFERRED consequence: after `DEVICE_RESET` while in bank N, the model stays in bank N while a driver assumes bank 0 (the bank-select register's stored value is reset but its write callback does not fire).
- INFERRED: each bank has its own physical 0x7F register object, so the value read back from 0x7F reflects only writes made while that bank was active.

### 1B. Registers
All registers reset to 0 except `WHO_AM_I` (`Define(collection, 0xEA)`). No other `Define` passes a reset value. Real-silicon reset values (e.g. `PWR_MGMT_1` = 0x41) are NOT modelled.

Register names in logs come from C# enums cast from the byte (`RegisterIndexToString` `ICM20948.cs:246-261`); an address not in the enum prints as a decimal number, e.g. `0x7F` in banks 1-3 prints `(127)` (enum has `RegisterBankSelection = 0x7F` only in bank 0, `Bank0:338`).

**Bank 0** (`ICM20948_Bank0Registers.cs`; enum `:272-339`):

| Addr | Name (enum) | Access | Behaviour / what is real |
|---|---|---|---|
| 0x00 | WhoAmI | R | `Define(..., 0xEA)` + tag only (`:15-17`). Reads 0xEA. |
| 0x03 | UserControl (USER_CTRL) | RW | **All tags** (`:19-28`): DMP_EN, FIFO_EN, I2C_MST_EN, I2C_IF_DIS (bit4), DMP_RST, SRAM_RST, I2C_MST_RST; bit 0 reserved. Write of any 1 bit = [W] tag warning. Not stored. |
| 0x05 | LPConfig | RW | Tags (`:30-36`). Not stored. |
| 0x06 | PowerManagement1 (PWR_MGMT_1) | RW | `DEVICE_RESET` bit7 = write-only flag, write-1 calls `SoftwareReset()` (`:39`). **SLEEP (bit6), LP_EN, TEMP_DIS, CLKSEL(0-2) are tags** (`:40-44`). Reads 0 after any write. Sleep is never set, so the part is always "awake". |
| 0x07 | PowerManagement2 | RW | Tags + reserved (`:47-51`). |
| 0x0F | InterruptPinConfig (INT_PIN_CFG) | RW | **All tags** (`:53-62`): INT1_ACTL, INT1_OPEN, INT1_LATCH_INT_EN, INT_ANYRD_2CLEAR, ACTL_FSYNC, FSYNC_INT_MODE_EN, BYPASS_EN. Polarity/latch/open-drain are ignored. |
| 0x10 | InterruptEnable (INT_ENABLE) | RW | Real flags: WOM_INT_EN(3), PLL_RDY_EN(2), DMP_INT1_EN(1), I2C_MST_INT_EN(0), REG_WOF_EN(7); change callback `UpdateInterrupts` (`:64-72`). |
| 0x11 | InterruptEnable1 (INT_ENABLE_1) | RW | `RAW_DATA_0_RDY_EN` bit 0, change callback (`:74-78`). |
| 0x12 | InterruptEnable2 | RW | `FIFO_OVERFLOW_EN` bit 0 (`:80-84`). |
| 0x13 | InterruptEnable3 | RW | `FIFO_WM_EN` bit 0 (`:86-90`). |
| 0x17 | I2CMstStatus | RW | All tags (`:92-101`). |
| 0x19 | InterruptStatus (INT_STATUS) | RW | Real **writable** flags WOM_INT, PLL_RDY_INT, DMP_INT1, I2C_MST_INT (`:103-110`). Default `WithFlag` mode is read/write; the model never sets them itself. |
| 0x1A | InterruptStatus1 (INT_STATUS_1) | RW | `RAW_DATA_0_RDY_INT` bit 0 (`:112-116`). **Never set by the model.** Reads 0 unless software writes 1. |
| 0x1B | InterruptStatus2 | RW | `FIFO_OVERFLOW_INT` (`:118-122`). |
| 0x1C | InterruptStatus3 | RW | `FIFO_WM_INT` (`:124-128`). |
| 0x28, 0x29 | DelayTimeHigh/Low | RW | Tags (`:130-136`). |
| 0x2D-0x32 | ACCEL_XOUT_H/L, YOUT_H/L, ZOUT_H/L | R | `valueProviderCallback: _ => (byte)(RawAccelerationX >> 8)` etc. (`:138-160`). Big-endian pairs, H first. |
| 0x33-0x38 | GYRO_XOUT_H/L, YOUT, ZOUT | R | Same pattern with `RawAngularRateX/Y/Z` (`:162-184`). |
| 0x39, 0x3A | TEMP_OUT_H/L | R | `RawTemperature` (`:186-192`). |
| 0x3B-0x52 | EXT_SLV_SENS_DATA_00..23 | R | `externalSlaveSensorData[index]` (`:194-199`). Only filled by the I2C-slave0 thread (Bank 3). Reads 0 otherwise. |
| 0x66, 0x67, 0x68, 0x69 | FIFO_EN_1, FIFO_EN_2, FIFO_RST, FIFO_MODE | RW | All tags (`:201-226`). FIFO is **not implemented**. |
| 0x70, 0x71, 0x72 | FIFO_COUNTH/L, FIFO_R_W | RW | All tags (`:228-239`). Always read 0. |
| 0x74 | DataReadyStatus (DATA_RDY_STATUS) | RW | **All tags** (`:241-245`): WOF_STATUS, RAW_DATA_RDY(0-3). **Always reads 0.** A driver polling `RAW_DATA_RDY` never sees data ready. |
| 0x76 | FifoConfig | RW | Tag (`:247-250`). |
| 0x7F | RegisterBankSelection | RW | `USER_BANK` at bits 4-5 (`ICM20948.cs:263-271`); selects the bank. |

Anything else in bank 0 (e.g. 0x01, 0x02, 0x04): [W] `Unhandled read from offset 0x{0:X}.` / `Unhandled write to offset 0x{0:X}, value 0x{2:X}.` (0.3).

**Bank 1** (`ICM20948_Bank1Registers.cs:13-71`): self-test, accel/gyro offset registers, `TBC_PLL`, all **tags**; plus 0x7F. Reads 0.

**Bank 2** (`ICM20948_Bank2Registers.cs`; enum `:143-...`): real, stored fields:
- 0x00 `GYRO_SMPLRT_DIV` 8-bit value (`:15-17`).
- 0x01 `GYRO_CONFIG_1`: FCHOICE bit0, `GYRO_FS_SEL` bits 1-2 (enum, 250/500/1000/2000 dps), `GYRO_DLPFCFG` bits 3-5 **tag** (`:19-24`).
- 0x02 `GYRO_CONFIG_2`: self-test enables bits 3-5, `GYRO_AVGCFG` bits 0-2 (`:26-32`).
- 0x03-0x08 gyro offset cancel registers (value fields).
- 0x09 `ODR_ALIGN_EN` tag (`:58-61`).
- 0x10, 0x11 `ACCEL_SMPLRT_DIV` hi (4 bits)/lo (`:63-70`).
- 0x12 `ACCEL_INTEL_CTRL`: `ACCEL_INTEL_EN` bit1, mode bit0 (`:72-76`). 0x13 `ACCEL_WOM_THR` (`:78-80`).
- 0x14 `ACCEL_CONFIG`: FCHOICE bit0, `ACCEL_FS_SEL` bits 1-2 (enum 2/4/8/16 g), `ACCEL_DLPFCFG` bits 3-5 **tag** (`:82-87`).
- 0x15 `ACCEL_CONFIG_2`: self-test enables, `DEC3_CFG` (`:89-95`).
- 0x52 `FSYNC_CONFIG`, 0x53 `TEMP_CONFIG`, 0x54 `MOD_CTRL_USR`: tags (`:97-115`).
- Full-scale selection drives the output scaling (below). The 2g/250dps default is the reset value of the zero fields.

**Bank 3** (`ICM20948_Bank3Registers.cs:62-216`): I2C master/slave registers. Only slave 0 has behaviour:
- `I2C_SLV0_ADDR` (0x03): `I2C_SLV0_RNW` bit7, `I2C_ID_0` bits 0-6; `I2C_SLV0_REG` (0x04); `I2C_SLV0_CTRL` (0x05): bit 7 `I2C_SLV0_EN` write callback starts/stops a managed thread at `InternalSampleRateHz` = 1125 Hz (`ICM20948_Bank3Registers.cs:93-117`) that calls `ReadSensorData` (`:50-56`); `I2C_SLV0_LENG` bits 0-3.
- `I2C_SLV0_DO` (0x06): write triggers an I2C write to the registered slave (`:119-129`).
- Other slaves and `I2C_SLV4_DI` are tags.
- Skip unless an AK09916 is registered under it.

### 1C. Every log call the model makes

Own logs (`ICM20948*.cs`):

| Line | Lvl | Format string (verbatim) | Fires when |
|---|---|---|---|
| `ICM20948.cs:58` | N | `"Write {0}"` | every I2C `Write`; arg is hex list of the whole buffer |
| `:64` | D | `"Selected bank #{0} register 0x{1:X} ({2})"` | I2C `Write` in `Idle` (first byte) |
| `:83` | D | `"Write at bank #{0} from 0x{1:X} to 0x{2:X}: data {3}"` | I2C `Write` that carries data bytes. **Bug in source: five args are passed but the format has only `{0}`-`{3}`; `{3}` renders `SelectedRegisterName` (the register after the last write), not the data. The data hex is arg `{4}` and is never printed.** |
| `:89` | W | `"Trying to write without specifying address, ignoring write"` | I2C `Write` in state `Reading` |
| `:93` | E | `"Attempted write in an unexpected state: {0}"` | I2C `Write` in state `Writing` or `WaitingForAddress` |
| `:103` | D | `"Staring read of {0} bytes at bank #{1} from 0x{2:X} ({3})"` | each I2C `Read` (typo "Staring" is verbatim) |
| `:107` | N | `"Read at bank #{0} from 0x{1:X} to 0x{2:X}, returned {3}"` | each I2C `Read`; `{2}` is the next address, `{3}` is the hex list |
| `:131` | N | `"Preparing for read at bank #{0} from 0x{1:X} ({2})"` | SPI address byte with bit 7 set |
| `:136` | N | `"Preparing for write at bank #{0} to 0x{1:X} ({2})"` | SPI address byte with bit 7 clear |
| `:142` | N | `"Read at bank #{0} from 0x{1:X} ({2}), returned 0x{3:X}"` | each SPI data byte while `Reading` |
| `:147` | D | `"Write at bank #{0} to 0x{1:X} ({2}): value 0x{3:X}"` | each SPI data byte while `Writing` (logged **before** the register write, so any tag [W] follows it) |
| `:152` | E | `"Attempted transmission in an unexpected state: {0}"` | SPI `Transmit` in any other state |
| `:156` | N | `"Received byte 0x{0:X}, returning 0x{1:X}"` | **every** SPI `Transmit` call, including the address byte |
| `:162` | N | `"Finishing transmission, going idle"` | SPI `FinishTransmission` (CS deassert and/or controller call) |
| `:170` | W | `"This model supports only CS on pin 0, but got signal on pin {0}"` | `OnGPIO` with number != 0 |
| `:240` | D | `"IRQ {0}"` (`"set"`/`"unset"`) | every `UpdateInterrupts` |
| `:241-242` | N | `"Interrupts: DMP {0}, Wake on Motion {1}, PLL RDY {2}, I2C Master {3}, Raw Data Ready {4}, FIFO Overflow {5}, FIFO Watermark {6}"` | every `UpdateInterrupts` (bool args print `True`/`False`) |
| `Bank3:22` | W | `"Selecting unconnected IIC slave address: 0x{0:X}"` | slave-0 transaction with no registered slave |
| `Bank3:25` | D | `"Selected slave address 0x{0:X}"` | slave-0 transaction with a registered slave |
| `Bank3:39` | W | `"Requested read length is greater than the number of available external sensor data registers. Clamping to the maximum available length."` | `I2C_SLV0_LENG` > 24 |
| `Bank3:110` | D | `"Starting reading magnetometer samples at frequency {0}Hz"` | `I2C_SLV0_EN` set |

Base-class logs: 0.3 and 0.3a. `UpdateInterrupts` runs only from the change callbacks on `INT_ENABLE`, `INT_ENABLE_1/2/3`, `INT_STATUS`, `INT_STATUS_1/2/3` (Bank0 `:71,77,83,89,109,115,121,127`), so it logs only when one of those registers changes value.

### 1C'. Exact Noisy-level transcripts

Register-name text comes from the enum: `PowerManagement1`, `RegisterBankSelection`, `WhoAmI`, `AccelerationXOutHigh`.

**SPI, write PWR_MGMT_1 (0x06) = 0x01** (bytes `06 01`):
```
[N] Preparing for write at bank #0 to 0x6 (PowerManagement1)
[N] Received byte 0x6, returning 0x0
[D] Write at bank #0 to 0x6 (PowerManagement1): value 0x1
[W] Unhandled write to offset 0x6. Unhandled bits: [0] when writing value 0x1. Tags: CLKSEL (0x1).
[N] Received byte 0x1, returning 0x0
[N] Finishing transmission, going idle
```
= 6 lines, 1 Warning (CLKSEL is a tag, bit 0 changed). Writing **0x00** gives the same minus the [W] (5 lines). Writing 0x80 (DEVICE_RESET) has no tag warning but calls `SoftwareReset` (resets all banks' registers, `selectedRegister`=0, `state`=Idle; `:202-212`) from inside the write callback; INFERRED: `Finishing transmission` still logs after.

**SPI, write REG_BANK_SEL (0x7F) = 0x20** (select bank 2), from bank 0:
```
[N] Preparing for write at bank #0 to 0x7F (RegisterBankSelection)
[N] Received byte 0x7F, returning 0x0
[D] Write at bank #0 to 0x7F (RegisterBankSelection): value 0x20
[N] Received byte 0x20, returning 0x0
[N] Finishing transmission, going idle
```
= 5 lines, no Warning (USER_BANK is a real field). Writing 0x00 back from bank 2 prints `Preparing for write at bank #2 to 0x7F (127)` and `Write at bank #2 to 0x7F (127): value 0x0`, because the bank-2 enum has no 0x7F name.

**SPI, identity read WHO_AM_I** (bytes `80 00`):
```
[N] Preparing for read at bank #0 from 0x0 (WhoAmI)
[N] Received byte 0x80, returning 0x0
[N] Read at bank #0 from 0x0 (WhoAmI), returned 0xEA
[N] Received byte 0x0, returning 0xEA
[N] Finishing transmission, going idle
```
= 5 lines.

**SPI, data read ACCEL_XOUT_H (0x2D) burst of N data bytes** (bytes `AD` + N dummies): **2N + 3 lines** (2 header, 2 per data byte, 1 finish). For N = 6 this is 15 lines; for N = 14 (0x2D..0x3A, accel+gyro+temp) this is 31. A burst logs **one `Read at...` and one `Received byte...` line per byte**. Per data byte:
```
[N] Read at bank #0 from 0x2D (AccelerationXOutHigh), returned 0x0
[N] Received byte 0x0, returning 0x0
```
with the address and name advancing (0x2E `AccelerationXOutLow`, ...). Values for no-feeder defaults are in 1F. No Warning, because 0x2D-0x3A are all defined.

**I2C equivalents** (the controller delivers a `Write` buffer then a `Read(count)`):
- Write PWR_MGMT_1 = 0x01 as `Write([0x06, 0x01])`:
  ```
  [N] Write [0x6, 0x1]
  [D] Selected bank #0 register 0x6 (PowerManagement1)
  [W] Unhandled write to offset 0x6. Unhandled bits: [0] when writing value 0x1. Tags: CLKSEL (0x1).
  [D] Write at bank #0 from 0x6 to 0x7: data PowerManagement2
  ```
  (the last line shows the source bug; 4 lines, FinishTransmission is silent).
- Identity: `Write([0x00])` then `Read(1)`:
  ```
  [N] Write [0x0]
  [D] Selected bank #0 register 0x0 (WhoAmI)
  [D] Staring read of 1 bytes at bank #0 from 0x0 (WhoAmI)
  [N] Read at bank #0 from 0x0 to 0x1, returned [0xEA]
  ```
- Burst of N: same 4 lines; **I2C logs 4 lines regardless of N** (one summary line holds all bytes).

### 1D. Warnings and errors a correct driver can hit
- **PWR_MGMT_1 = 0x01 (CLKSEL auto-select) always warns** (tag CLKSEL). 0x00 does not. 0x80 does not.
- **USER_CTRL (0x03) writes with any of I2C_IF_DIS (0x10), I2C_MST_EN (0x20), FIFO_EN (0x40), DMP_EN (0x80)** warn. A SPI driver that disables the I2C interface (0x10) warns once.
- `INT_PIN_CFG` (0x0F) writes with any bit set warn, e.g. latch/open-drain/polarity.
- `GYRO_CONFIG_1`/`ACCEL_CONFIG` writes with DLPF bits (3-5) set warn.
- Any write to the FIFO enable/reset registers, I2C master registers, offsets, `SMPLRT`-adjacent tags warns when a 1 is written into a tag bit.
- Reads of `DATA_RDY_STATUS`, `FIFO_COUNT` return 0 without a warning (tags read as reset value).
- Unhandled-register warnings for any undefined address (0.3).
- [E] 152/93, [W] 89 only on mixed or mis-sequenced traffic (e.g. SPI after an I2C write that was never finished; INFERRED).
- Nothing warns on "write to read-only register": data registers are plain `WithValueField` with default (read/write) mode, so a write is accepted silently and discarded (`valueProviderCallback` overrides on next read).
- No reset warning: `DEVICE_RESET` is silent.

### 1E. Interrupts and data-ready
- `public GPIO IRQ` (`ICM20948.cs:216`). A single line. `IRQ.Set(irq)` in `UpdateInterrupts` (`:243`), where `irq` is the OR of `enabled && status` for DMP, WOM, PLL_RDY, I2C master, RAW_DATA_RDY, FIFO overflow, FIFO watermark (`:223-238`).
- Enable registers: `INT_ENABLE` (0x10) bits {WOM 3, PLL_RDY 2, DMP 1, I2C_MST 0}, `INT_ENABLE_1` (0x11) bit 0 = RAW_DATA_0_RDY_EN, `INT_ENABLE_2/3`.
- **The model never sets any status flag.** No code assigns `rawDataReadyInterruptStatus` etc. other than the register field being written by the bus (grep of `ICM20948/*.cs` for `Status` writes finds only the `WithFlag(out ...)` definitions). So `IRQ` only goes high if the driver itself writes a 1 into `INT_STATUS_1` (0x1A) bit 0 after enabling `INT_ENABLE_1`. **No data-ready interrupt is ever produced by the model**, and no timer or feeder sample drives it.
- `INT_PIN_CFG` (polarity, latch, open-drain) is all tags, so polarity is fixed at active-high-level whenever set at all (INFERRED: `GPIO.Set(true)`).
- Clearing: `INT_STATUS*` registers are not clear-on-read, so a read does not clear. The driver must write 0 into them (writing 0 triggers the change callback and clears).

### 1F. Where the data comes from
- No randomness, no seed.
- Properties settable from the monitor/`.repl` (all decimal, g / dps / degC):
  - `DefaultAccelerationX/Y/Z` (`ICM20948_Accelerometer.cs:29-69`), range-checked against the **current** `ACCEL_FS_SEL` full scale, else `RecoverableException` `"Value out of currently set range. Maximum value is {..}[g]"` (`:37,51,65`). Default 0.
  - `DefaultAngularRateX/Y/Z` (`ICM20948_Gyroscope.cs:29-69`), range-checked vs `GYRO_FS_SEL` (message says `[g]`, copy-paste in source, `:37`). Default 0.
  - `DefaultTemperature` (`ICM20948_Thermometer.cs:23`). Default 0. **`Temperature` setter throws** `RecoverableException("Explicitly setting temperature is not supported by this model. Temperature should be provided from a RESD file or set via the 'DefaultTemperature' property")` (`:50-51`).
  - `AccelerationX/Y/Z`, `AngularRateX/Y/Z` are get-only: `AccelerationX => accelerationX ?? DefaultAccelerationX` (`Accel:71-75`).
- RESD feeders: `FeedAccelerationSamplesFromRESD`, `FeedAngularRateSamplesFromRESD`, `FeedTemperatureSamplesFromRESD`. Accel/gyro start a feed thread at `(uint)AccelerometerOutputDataRateHz` computed **at the time of the call** from the divider and `FCHOICE` (`Accel:18-27,112-122`; `Gyro:18-27,108-118`). Set the Bank 2 ODR registers first, otherwise the rate is the max (4500 Hz accel `Accel:204`, 9000 Hz gyro `Gyro:179`).
- Scaling (read-only `RawX` properties):
  - Accel: `value * 16384/8192/4096/2048` by FS (`Accel:144-162,188-192`).
  - Gyro: `value * 131/65.5/32.8/16.4` by FS (`Gyro:140-160,162-166`).
  - Temp: `((T - 21.0) * 333.87) + 0.0`, rounded, clamped to short (`Thermo:54-60`).
  - `ConvertMeasurement` rounds and clamps to int16 (`ICM20948.cs:273-279`).
- **Deterministic with no feeder: YES.** All-zero accel and gyro. Temp raw = `round(-21 * 333.87)` = -7011 = `0xE49D` (INFERRED arithmetic: `-7011.27` rounds to -7011; `65536-7011 = 58525 = 0xE49D`), so `TEMP_OUT_H = 0xE4`, `TEMP_OUT_L = 0x9D`.
- Accel data reads do not change over time without a feeder. There is no noise model and no time dependence.

### 1G. Identity
- `WHO_AM_I` (Bank 0, 0x00) = **0xEA** (`Bank0:15`).
- Other probe registers: none are modelled beyond zero-reset registers. A probe that reads `PWR_MGMT_1` gets 0x00 (real part resets to 0x41; INFERRED not modelled).

### 1H. Mode / power
- **No enforcement.** `SLEEP`, `CLKSEL`, `PWR_MGMT_2` (accel/gyro disable) are tags; data registers return values regardless of power state.
- ODR is irrelevant for plain register reads. It only matters for RESD feed thread timing (1F).
- Full-scale (Bank 2) affects both the scaling and the allowed range of the `Default*` properties.
- **No data-ready gating**: `DATA_RDY_STATUS` reads 0, `INT_STATUS_1` reads 0 (1B). A driver that blocks until a ready bit is set will time out.

---

## 2. LSM6DSO (`S/LSM6DSO_IMU.cs`)

### 2A. Bus and interface
- `public class LSM6DSO_IMU : BasicBytePeripheral, ISPIPeripheral, II2CPeripheral, IProvidesRegisterCollection<ByteRegisterCollection>, ITemperatureSensor, IUnderstandRESD` (`LSM6DSO_IMU.cs:24`).
- No `IGPIOReceiver`, so SPI CS must come via the controller calling `FinishTransmission`.
- Interrupt output: `public GPIO Interrupt1 { get; }` (`:169`), created `:28`. No INT2.

**SPI** (`Transmit`, `:104-128`):
- `CommandTypes.None` first byte: `address = bits 0-6`, `commandInProgress = (CommandTypes)bit7` (0 = `Write`, 1 = `Read`; enum comment `:1188`, values `:1189-1195`). [N] line 116.
- `Read` state: `PerformRead(address)` per byte; `Write` state: `PerformWrite(address, data)`.
- Auto-increment is controlled by `CTRL3_C.IF_INC` (`addressAutoIncrement`, bit 2, **reset value 1**: `Control3.Define(this, resetValue: 0b00000100)` at `:244`). `TryIncrementAddress` (`:708-723`): `address = (address + 1) % 0x80` when `IF_INC` is set; when `address == FifoOutputZHigh (0x7E)` it wraps to `FifoOutputTag (0x78)` regardless of `IF_INC` (`:710-714`).
- `FinishTransmission` (`:80-84`): [N] log, `commandInProgress = None`. Shared between SPI and I2C.

**I2C** (`Write`/`Read`, `:130-155`):
- `Write(data)` in `None`: `address = data[0]` (full byte, 7-bit address assumed, no R/W bit masking), state `ReceivedAddress`; remaining bytes `PerformWrite`. No log for the address byte.
- `Read(count)`: `PerformRead(address)` ×count. `PerformRead` promotes `ReceivedAddress` -> `Read` (`:790-793`).
- If a previous `Write` carried data bytes (state `Write`), a following `Read` in the same transaction is rejected: [W] `Tried to read in invalid state: Write` (`:796`). A subsequent `Read` after `FinishTransmission` (STOP in between) is also rejected: [W] `Tried to read in invalid state: None`. So the controller must do write-address / repeated-START / read with no `FinishTransmission` between (INFERRED about controllers).

**Register banks:** `FUNC_CFG_ACCESS` (0x01) bits 6-7 select the bank (`:211-214`). Any bank other than `Default` makes every address except 0x01 fail: [W] 801 / 824 (the model supports only the default bank).

### 2B. Registers (defined set, `DefineRegisters` `:209-418`; enum `:1255-1337`)

| Addr | Name | Access | Reset | Behaviour |
|---|---|---|---|---|
| 0x01 | FUNC_CFG_ACCESS | RW | 0 | bits 6-7 `SHUB_REG_FUNC_CFG_ACCESS` enum; bits 0-5 reserved (`:211-214`) |
| 0x02 | PIN_CTRL | R(bits0-5)/tags | 0 | bits 0-5 read as 0x3F always (`valueProviderCallback: _ => 0x3F`), bits 6,7 tags (`:216-221`) |
| 0x07 | FIFO_CTRL1 | RW | 0 | WTM[0-7] (`:344-346`) |
| 0x08 | FIFO_CTRL2 | RW | 0 | WTM8 bit0; UNCOPTR_RATE, ODRCHG_EN, FIFO_COMPR_RT_EN, STOP_ON_WTM tags (`:348-356`) |
| 0x09 | FIFO_CTRL3 | RW | 0 | `BDR_XL` bits 0-3, `BDR_GY` bits 4-7; **change callbacks create the default sample feeders** (`:358-379`, see 2F) |
| 0x0A | FIFO_CTRL4 | RW | 0 | `FIFO_MODE` bits 0-2 (`commonFifo.Mode = newValue` on **every write**, `Bypass` clears the FIFO and latest samples, `:381-386`, `LSM6DSO_FIFO.Mode` `:973`, `Reset` `:935-944`); ODR_T_BATCH, DEC_TS_BATCH tags |
| 0x0D | INT1_CTRL | RW | 0 | **only `INT1_FIFO_OVR` (bit 4) is real**; INT1_DRDY_XL(0), INT1_DRDY_G(1), INT1_BOOT, INT1_TH, INT1_FULL, INT1_CNT_BDR, DEN_DRDY are tags (`:399-409`) |
| 0x0F | WHO_AM_I | R | 0x6C | `valueProviderCallback: _ => 0x6C` (`:223-225`) |
| 0x10 | CTRL1_XL | RW | 0 | bit0 reserved, bit1 `LPF2_XL_EN` **tag**, `FS_XL` bits 2-3 (enum, change cb updates scaling), `ODR_XL` bits 4-7 (enum DataRates) (`:227-233`) |
| 0x11 | CTRL2_G | RW | 0 | bit1 `FS_125`, `FS_G` bits 2-3, `ODR_G` bits 4-7 (`:235-242`) |
| 0x12 | CTRL3_C | RW | 0x04 | `SW_RESET` bit0 (`FieldMode.WriteOneToClear` with a **change** callback that calls `SoftwareReset`; **never fires**, see below), bit1 reserved, `IF_INC` bit2, **`SIM`, `PP_OD`, `H_LACTIVE`, `BDU`(6), `BOOT`(7) tags** (`:244-253`) |
| 0x14 | CTRL5_C | RW | 0 | all tags (`:255-264`) |
| 0x15 | CTRL6_C | RW | 0 | all tags (`:266-275`) |
| 0x16 | CTRL7_G | RW | 0 | all tags (`:277-286`) |
| 0x17 | CTRL8_XL | RW | 0 | `XL_FS_MODE` bit1 real, rest tags (`:288-296`) |
| 0x18 | CTRL9_XL | RW | 0xE0 | all tags (`:298-307`) |
| 0x20, 0x21 | OUT_TEMP_L/H | R | | `GetScaledTemperatureValue` (`:653-660`): `(short)((Temperature - 25) * 256)` (`:874-875`) |
| 0x22-0x27 | OUTX_L_G ... OUTZ_H_G | R | | from `commonFifo.AngularRateSample` (`:397`) |
| 0x28-0x2D | OUTX_L_A ... OUTZ_H_A | R | | from `commonFifo.AccelerationSample` (`:395`) |
| 0x3A, 0x3B | FIFO_STATUS1/2 | R | | DIFF_FIFO count, FIFO_OVR_LATCHED/FULL/OVR/WTM (`:321-342`) |
| 0x62 | I3C_BUS_AVB | RW | 0 | tags/reserved (`:411-417`) |
| 0x78 | FIFO_DATA_OUT_TAG | R | | **a read dequeues** a sample from the FIFO (`:389-393`) |
| 0x79-0x7E | FIFO_DATA_OUT_X_L..Z_H | R | | from `commonFifo.Sample` (`:396`) |

**NOT defined (read and write both go to the unhandled path, 0.3):** 0x0B/0x0C `COUNTER_BDR_REG1/2`, 0x0E `INT2_CTRL`, 0x13 `CTRL4_C`, 0x19 `CTRL10_C`, 0x1A-0x1D interrupt source regs, **0x1E `STATUS_REG`**, 0x35-0x39, 0x40-0x43 timestamp, 0x56-0x5F interrupt/tap/wake-up config, 0x63, 0x6F-0x77. The enum lists them (`:1255-1337`) but no `Define` exists (grep of `Registers\.\w+\.Define` and `DefineOutputRegistersGroup` shows exactly the rows above).

**`CTRL3_C.SW_RESET` does not reset the model (INFERRED by tracing `WriteInner`).** The field is `WithFlag(0, FieldMode.WriteOneToClear, ..., changeCallback: ...)` (`:245`). In `WriteInner` the `WriteOneToClear` branch only acts when `~difference & value` has the bit set (`REG/PeripheralRegister.cs:622-628`), i.e. when the stored bit is already 1. The stored bit is 0 and can only be set by the model, never by a write, so the field never joins `changedRegisters` and the **change** callback never runs. A driver's soft reset (`CTRL3_C |= 0x01`) therefore leaves every register, the FIFO and the samples untouched; the bit reads back 0. The same write does clear `IF_INC` (bit 2) if the written byte has bit 2 = 0, so a bare `0x01` write stops auto-increment (burst reads then re-read one address) until the driver rewrites `0x04`/`0x44`. Contrast ICM-20948 `DEVICE_RESET`, which uses a **write** callback gated on the written bit and does fire (`ICM20948_Bank0Registers.cs:39`; write callbacks run on every write to the register: `REG/RegisterField.cs:140-148`, `PeripheralRegister.cs:630-640`).

**STATUS_REG (0x1E) is not implemented.** A driver polling `XLDA`/`GDA` gets `[W] Unhandled read from offset 0x1E.` and 0x00, forever. Reads of 0x0E, 0x13, 0x19, 0x1A-0x1D behave the same.

### 2C. Every log call the model makes

| Line | Lvl | Format string (verbatim) | Fires when |
|---|---|---|---|
| `:82` | N | `"Finishing transmission, going to the Idle state"` | `FinishTransmission` |
| `:116` | N | `"Received 0x{0:X2}; setting commandInProgress to {1} and address to 0x{2:X2}"` | SPI first byte only (the I2C path has no equivalent) |
| `:526` | D | `"Invalid value for the current full scale selection ({0}): {1}"` | lower-byte read of a sample value that clamps to int16 |
| `:748` | D | `"New INT1 state: {0}"` (`set`/`reset`) | `Interrupt1` level changes |
| `:796` | W | `"Tried to read in invalid state: {0}"` | `PerformRead` in a state other than Read/ReceivedAddress |
| `:801` | W | `"Unhandled read from 0x{0:X2}: unsupported {1} bank"` | read while FUNC_CFG_ACCESS selects a non-default bank (address != 0x01) |
| `:806` | N | `"Read from 0x{0:X2} ({1}): returning 0x{2:X2}"` | every byte read (`{1}` = `Registers` enum name; undefined values print the number) |
| `:819` | W | `"Tried to write in invalid state: {0}"` | `PerformWrite` in a state other than Write/ReceivedAddress |
| `:824` | W | `"Unhandled write to offset 0x{0:X2}. value 0x{1:X2}: unsupported {2} bank"` | write in a non-default bank |
| `:829` | N | `"Write to 0x{0:X2} ({1}): 0x{2:X2}"` | every byte written (after `WriteByte`, so any tag [W] precedes it) |
| `:922` | D | `$"{name}: Overrun"` (interpolated; name = `"fifo"`) | first FIFO overrun in Continuous mode |
| `:927` | N | `$"{name}: Fifo filled up. Dumping the oldest sample."` | FIFO full in Continuous mode on each new sample |
| `:937` | D | `"Resetting FIFO"` | `LSM6DSO_FIFO.Reset`: at construction / machine `Reset()` (via `SoftwareReset`, `:672`), and on **every** write to FIFO_CTRL4 whose `FIFO_MODE` is Bypass (value 0), because the field's write callback assigns `commonFifo.Mode` unconditionally (`:382`) |
| `:952` | N | `"New sample dequeued: {0}"` | FIFO_DATA_OUT_TAG read dequeues |
| `:957` | N | `"Dequeueing new sample failed."` | FIFO_DATA_OUT_TAG read, queue empty or FIFO bypass |
| `:1000` | D | `"Sample unavailable -- FIFO disabled."` | tag read in Bypass |
| `:1023` | W | `"{0}: No sample found with {1} tag"` | **every** read of an accel or gyro output register while no sample has been produced (renders `fifo: No sample found with AccelerometerNC tag` / `GyroscopeNC tag`) |

Base logs: 0.3, 0.3a.

### 2C'. Exact Noisy-level transcripts (SPI; I2C equivalents below)

**Write CTRL1_XL (0x10) = 0x40** (104 Hz, 2 g; bytes `10 40`):
```
[N] Received 0x10; setting commandInProgress to Write and address to 0x10
[N] Write to 0x10 (Control1_Accelerometer): 0x40
[N] Finishing transmission, going to the Idle state
```
= 3 lines, no Warning (`ODR_XL`, `FS_XL` are real fields). A value with bit 1 set (`LPF2_XL_EN`) adds `[W] Unhandled write to offset 0x10. Unhandled bits: [1] when writing value 0x42. Tags: LPF2_XL_EN: Accelerometer high-resolution selection (0x1).` (the tag name in the source is the long string, `:229`).

**Write CTRL3_C (0x12) = 0x44** (BDU|IF_INC, the usual init):
```
[N] Received 0x12; setting commandInProgress to Write and address to 0x12
[W] Unhandled write to offset 0x12. Unhandled bits: [6] when writing value 0x44. Tags: BDU (0x1).
[N] Write to 0x12 (Control3): 0x44
[N] Finishing transmission, going to the Idle state
```
(the Warning is emitted inside `WriteByte`, before the [N] write line at `:829`). A common `0x44` init therefore costs one Warning.

**Identity read WHO_AM_I (0x0F)** (bytes `8F 00`):
```
[N] Received 0x8F; setting commandInProgress to Read and address to 0x0F
[N] Read from 0x0F (WhoAmI): returning 0x6C
[N] Finishing transmission, going to the Idle state
```
= 3 lines.

**Data read OUTX_L_A (0x28) burst of N bytes** (bytes `A8` + N dummies): **N + 2 lines** (1 header, 1 per byte, 1 finish). For N = 6: 8 lines. One line per byte:
```
[N] Read from 0x28 (AccelerometerXLow): returning 0x00
[N] Read from 0x29 (AccelerometerXHigh): returning 0x00
[N] Read from 0x2A (AccelerometerYLow): returning 0x00
...
```
- Needs `IF_INC` (set by default) to advance (`:722`). If a driver clears it, every byte reads 0x28 again.
- **If no accel sample exists, each of the 6 byte-reads also emits `[W] fifo: No sample found with AccelerometerNC tag`, 6 warnings per 6-byte burst, and returns 0.** See 2F.

**I2C equivalents:** the address-only `Write([0x28])` logs nothing; `Read(N)` logs N `Read from ...` lines; `FinishTransmission` logs 1. So a 6-byte burst is 7 lines; a config write `Write([0x10, 0x40])` is `Write to 0x10 (Control1_Accelerometer): 0x40` plus the finish line (2 lines); WHO_AM_I is 2 lines.

### 2D. Warnings and errors a correct driver can hit
- **`CTRL3_C` = 0x44 (BDU)**: one [W] (BDU tag).
- **`CTRL1_XL` with LPF2_XL_EN**, **`CTRL6_C`** (`FTYPE`, `USR_OFF_W`, `XL_HM_MODE`, ...), **`CTRL7_G`** (`HPM_G`, `HP_EN_G`, `G_HM_MODE`), **`CTRL5_C`**, **`CTRL8_XL`** (all but bit 1), **`CTRL9_XL`**, `PIN_CTRL`: each tag bit written as 1 gives a [W].
- **`INT1_CTRL` = 0x01 / 0x02 (DRDY_XL/DRDY_G)**: [W] `Unhandled write to offset 0xD. Unhandled bits: [0] when writing value 0x1. Tags: INT1_DRDY_XL (0x1).` Only bit 4 is accepted.
- **Any read of `STATUS_REG` (0x1E), `CTRL4_C` (0x13), `CTRL10_C`, `INT2_CTRL`, `ALL_INT_SRC`, `WAKE_UP_SRC`, `TAP_SRC`, `D6D_SRC`**: [W] `Unhandled read from offset 0x{0:X}.`; writes likewise [W] `Unhandled write to offset 0x{0:X}, value 0x{2:X}.`
- **Output reads with no sample**: [W] 1023, once per byte.
- **I2C write-then-read with a STOP between**, or write-with-data then read: [W] `Tried to read in invalid state: None` / `Write`. Returns 0.
- Read/write while `FUNC_CFG_ACCESS` selects a non-default bank: [W] 801/824.
- `FIFO_CTRL4` write of 0 (Bypass) drops the latest accel and gyro samples (`LSM6DSO_FIFO.Reset` `:935-944`): a later data read warns with 1023 until a new sample arrives. Order matters: write `FIFO_CTRL4` before the feeder/feed.
- INFERRED source defect: `UpdateAngularRateSampleFrequency` calls `accelerometerFeederThread.Stop()` in its else branch (`:489`), a null dereference if the gyro BDR is disabled while the accel feeder is null. Only on an odd order of writes.
- No [E] lines in the model proper. `Transmit` throws `ArgumentException("Invalid commandInProgress: ...")` (`:125`) if SPI follows an I2C address-only write without a finish (INFERRED).

### 2E. Interrupts and data-ready
- `Interrupt1` only. `UpdateInterrupts` (`:738-752`): `newInt1Status = interrupt1EnableFifoOverrun.Value && FifoOverrunStatus`. Set when the FIFO overruns in Continuous mode with `INT1_CTRL.INT1_FIFO_OVR` (bit 4) enabled; callers are `commonFifo.OnOverrun` (`:31`), the `INT1_CTRL` write callback (`:408`), and `SoftwareReset` (`:672`, reachable only from construction/machine `Reset()`, not from `SW_RESET`). Cleared only by `FIFO` Reset (clears `OverrunOccurred`) via a reset or a write setting Bypass.
- **No data-ready interrupt exists.** `INT1_DRDY_XL`/`INT1_DRDY_G` are tags. No timer or sample drives `Interrupt1` except via the overrun path.
- `Interrupt2` does not exist.
- Level is plain high/low; polarity `H_LACTIVE` and `PP_OD` are tags (ignored).

### 2F. Where the data comes from
- **There is no sample until something feeds one.** Output registers read the FIFO's `accelerationSample` / `angularRateSample` (`:395,397,961,963`), both null at reset (`Reset` `:935-944`). A null sample returns 0 and, for the accel/gyro groups, logs [W] 1023 (`:1006-1026`). The FIFO output group reads `commonFifo.Sample`, which returns null without a warning (`:988`: `latestSample?.Tag != FifoTag.UNKNOWN ? latestSample : null`).
- The ways a sample appears:
  1. **`FIFO_CTRL3` (0x09) write with `BDR_XL != 0` (accel) or `BDR_GY != 0` (gyro)** whose change callback creates the default feeder (`:358-379`, `CreateAccelerationDefaultSampleFeeder` `:420-431`, `CreateDefaultSampleFeeder` `:446-453`). It feeds `DefaultAccelerationX/Y/Z` immediately and then at `DataRateToFrequency(AccelerometerODR)` on a managed thread. `AccelerometerODR` is `BDR_XL` if batching enabled, else `ODR_XL` (`:833`).
  2. **Monitor command `FeedAccelerationSample x y z [repeat]` / `FeedAngularRateSample`** (`:37-43,56-62`).
  3. **RESD**: `FeedAccelerationSamplesFromRESD`, `FeedAngularRateSamplesFromRESD`, `FeedTemperatureSamplesFromRESD` (`:45-78`). The RESD thread rate is `DataRateToFrequency(AccelerometerODR)` **at call time**, so set `ODR_XL`/`BDR_XL` first. When the RESD stream ends, `HandleAccelerationSampleEnded` creates the default feeder (`:560`).
- **`CTRL1_XL.ODR_XL` alone does NOT create a feeder** (confirmed: the only callers of `CreateAccelerationDefaultSampleFeeder` are `:365` (FIFO_CTRL3 BDR) and `:560` (RESD end)). A driver that configures only `CTRL1_XL`/`CTRL2_G` and reads `OUTX_L_A` gets zeros and a Warning per byte unless the harness feeds samples via monitor or sets BDR.
- In `Bypass` (reset default) FIFO mode, `FeedSample` keeps only the latest sample (`:907-916`); output registers return it. In other modes samples queue and `OUT*` registers show only the most recently **dequeued** one (dequeue happens on a `FIFO_DATA_OUT_TAG` read).
- Values:
  - `DefaultAccelerationX/Y/Z`, `DefaultAngularRateX/Y/Z` plain decimal properties, no range check (`:171-205`). Default 0.
  - `Temperature` plain decimal property, default 0 (`:207`). Raw = `(short)((T - 25) * 256)`: default 0 gives `-6400 = 0xE700` (INFERRED arithmetic), i.e. `OUT_TEMP_L = 0x00`, `OUT_TEMP_H = 0xE7`. `OUT_TEMP_L` read refreshes `Temperature` from RESD if attached (`:310-314`, `TryUpdateCurrentTemperatureSample` `:687`).
  - Scaling: accel `sensitivity = 32768 / full_scale_g` (`UpdateAccelerationSensitivity` `:726-729`, full scale `:592-609`); gyro `1000 / mdps_per_lsb` with 4.375/8.75/17.5/35/70 mdps/LSB (`UpdateAngularRateSensitivity` `:731-735`, table `:611-629`); values clamped to int16 (`GetScaledValueByte`, `:1088-...`), with [D] 526 when clamped.
- **Deterministic with no feeder: yes but zero**: 0 for every accel and gyro byte, plus the [W]s above. Temperature is 0xE700. After a BDR default feeder starts: the `Default*` values (0 unless set).

### 2G. Identity
- `WHO_AM_I` (0x0F) = **0x6C** (`:223-225`).
- Other probe registers: `CTRL3_C` readable (reset 0x04); `STATUS_REG` is not there (2B).

### 2H. Mode / power
- **Not enforced for data reads**: `OUT*` registers return the stored sample whatever `ODR_XL`/`ODR_G` is. `IsAccelerometerPoweredOn` / `IsGyroscopePoweredOn` (`:163,167`) only affect `FIFO_FULL_IA` in `FIFO_STATUS2` (`:334-338`).
- ODR affects (a) the default feeder rate and (b) RESD rate. `Disabled` ODR makes `CreateAccelerationDefaultSampleFeeder` return null (`:422-425`).
- `H_LACTIVE`, `BDU`, filters, high-performance mode are tags and ignored.

---

## 3. BMP180 (`S/BMP180.cs`)

### 3A. Bus and interface
- `public class BMP180 : II2CPeripheral, IProvidesRegisterCollection<ByteRegisterCollection>, ITemperatureSensor` (`BMP180.cs:18`). **I2C only. No SPI, no GPIO.**
- `Write(byte[] data)` (`:33-57`):
  - `data.Length == 0`: [W] `"Unexpected write with no data"` (`:37`), return.
  - Otherwise `registerAddress = (Registers)data[0]` (the first byte is always the register address); bytes `1..` are written to `RegistersCollection.Write((byte)registerAddress, b)`, with the **same** address for every byte (no auto-increment on writes, `:47-51`).
  - A one-byte write only selects the register: [N] `"Preparing to read register {0} (0x{0:X})"` (`:55`).
- `Read(int count)` (`:59-70`): reads `count` bytes starting at `registerAddress`, calling `RegistersAutoIncrement()` after each byte.
- Auto-increment (`:166-175`): only when `registerAddress` is in `[0xAA, 0xBF)` or `[0xF6, 0xF8)`: it increments, and logs [N]. It does **not** increment elsewhere, so reading N bytes from 0xD0 repeats the chip ID N times; `0xF8` and `0xBF` do not advance.
- `FinishTransmission` (`:72-74`): empty. State persists across transactions: `registerAddress` stays after a STOP, so write-then-STOP-then-read works.

### 3B. Registers (`DefineRegisters` `:97-164`; enum `:249-279`)

| Addr | Name | Access | Reset | Behaviour |
|---|---|---|---|---|
| 0xAA-0xB1 | CoefficientCalibrationAA..B1 | R | 1B,CB,FB,CB,C6,91,7B,A8 | plain `Define(this, value)` with no fields (`:99-106`) |
| 0xB2-0xB5 | ...B2..B5 | R | 7F,75,5A,71 | `WithValueField(..., FieldMode.Read)`; used by temp conversion (`:108-118`) |
| 0xB6-0xBB | ...B6..BB | R | 15,7A,00,38,80,00 | (`:120-125`) |
| 0xBC-0xBF | ...BC..BF | R | `CalibMB>>8`=DD,`F9`,0B,34 | MC = -8711 (`:241`), MD = 0x0B34 (`:127-137`) |
| 0xD0 | ChipID | R | 0x55 | (`:139`) |
| 0xE0 | SoftReset | W | 0 | write of 0xB6 calls `Reset()` (`:141-148`); any other value is a silent no-op |
| 0xF4 | CtrlMeasurement | RW | 0 | `CTRL_MEAS` bits 0-4, `SCO` bit 5, `OSS` bits 6-7; **write callback `HandleMeasurement`** runs the conversion immediately (`:150-154,196-218`) |
| 0xF6 | OutMSB | R | 0x80 | (`:156-157`) |
| 0xF7 | OutLSB | R | 0 | (`:159-160`) |
| 0xF8 | OutXLSB | R | 0 | (`:162-163`) |

- Decoded calibration (INFERRED from the byte values, standard BMP180 big-endian layout): AC1 = 7115, AC2 = -1077, AC3 = -14703, AC4 = 31656, AC5 = 32629, AC6 = 23153, B1 = 5498, B2 = 56, MB = -32768, MC = -8711, MD = 2868. Every calibration register is answered. A driver's probe/calibration read of 0xAA-0xBF works as a 22-byte burst.
- Any address not in the enum, e.g. 0xF5 (reserved) or 0xE1: [W] 0.3 text.
- The registers have no tags, so writes to partially-defined registers are silent.

### 3C. Every log call

| Line | Lvl | Format string (verbatim) | Fires when |
|---|---|---|---|
| `:30` | N | `"Reset registers"` | `Reset()` (machine reset and 0xE0=0xB6) |
| `:37` | W | `"Unexpected write with no data"` | `Write` with empty buffer |
| `:41` | N | `"Write with {0} bytes of data: {1}"` | every `Write` (count, hex list) |
| `:49` | N | `"Writing 0x{0:X} to register {1} (0x{1:X})"` | each data byte after the address (`{1}` = enum name, then its hex; see the padding note below) |
| `:55` | N | `"Preparing to read register {0} (0x{0:X})"` | one-byte write |
| `:61` | N | `"Reading {0} bytes from register {1} (0x{1:X})"` | each `Read` (count, name, hex) |
| `:66` | N | `"Read value {0} from register {1} (0x{1:X})"` | each byte read; **`{0}` is the decimal value** |
| `:83` | W | `"Temperature is out of range. Supported range: {0} - {1}"` | `Temperature` setter outside -40..85 (`:238-239`) |
| `:88` | N | `"Sensor temperature set to {0}"` | `Temperature` set OK |
| `:173` | N | `"Auto-incrementing to the next register 0x{0:X} - {0}"` | each auto-increment |
| `:198` | N | `"HandleMeasurement set {0}"` | every `CtrlMeasurement` write (`{0}` = `Temperature` for 0x0E, `Pressure` for 0x14, else the decimal 5-bit value) |
| `:217` | N | `"Conversion is complete"` | after `HandleMeasurement` |

Base logs: 0.3 for undefined addresses.

**Hex padding of enum arguments (INFERRED, not run).** `Registers` in `BMP180.cs:249` is a plain `enum` (underlying `int`). .NET formats an enum with the `X` specifier as the underlying value in hex **padded to the underlying type's width**, so `(0x{1:X})` and `0x{0:X}` (lines 49, 55, 61, 66, 173) should render 8 digits, e.g. `(0x000000F4)`, not `(0xF4)`. The transcripts below use that padded form. If a live run shows `(0xF4)`, the padding assumption is wrong and only these hex suffixes change. The same rule makes ICP-101xx `Command : ushort` render 4 digits, which is how it is already written (`609C`, `EFC8`), and does not affect the LSM6DSO/ICM/LPS25HB lines, whose `X` arguments are bytes/ints/longs and not enums.

### 3C'. Exact Noisy-level transcripts (I2C)

**Write CTRL_MEAS (0xF4) = 0x2E (start temperature)**, buffer `[F4 2E]`:
```
[N] Write with 2 bytes of data: [0xF4, 0x2E]
[N] Writing 0x2E to register CtrlMeasurement (0x000000F4)
[N] HandleMeasurement set Temperature
[N] Conversion is complete
```
= 4 lines. 0x34 (pressure, OSS 0) prints `HandleMeasurement set Pressure`; the 5-bit field for any other value prints the number. No Warning.

**Identity read**, `Write([D0])` then `Read(1)`:
```
[N] Write with 1 bytes of data: [0xD0]
[N] Preparing to read register ChipID (0x000000D0)
[N] Reading 1 bytes from register ChipID (0x000000D0)
[N] Read value 85 from register ChipID (0x000000D0)
```
= 4 lines (0x55 = 85 decimal). No auto-increment line.

**Data read 0xF6**, `Write([F6])` then `Read(2)`:
```
[N] Write with 1 bytes of data: [0xF6]
[N] Preparing to read register OutMSB (0x000000F6)
[N] Reading 2 bytes from register OutMSB (0x000000F6)
[N] Read value 128 from register OutMSB (0x000000F6)
[N] Auto-incrementing to the next register 0x000000F7 - OutLSB
[N] Read value 0 from register OutLSB (0x000000F7)
[N] Auto-incrementing to the next register 0x000000F8 - OutXLSB
```
= 7 lines (2 for the address `Write` + 5 for the `Read`). The increment runs after **every** byte including the last, and 0xF7 is still below `OutXLSB` (`:168-170`), so a 2-byte read leaves the pointer on 0xF8 and logs a second increment line. **Per N-byte read: 1 + N + (number of auto-increments)**. `Read(2)` from 0xF6 = 5 lines; `Read(3)` from 0xF6 (MSB, LSB, XLSB, as needed for pressure) = 1 + 3 + 2 = 6 lines (increments after F6 and F7 only; after F8 there is none). Calibration `Read(22)` from 0xAA = 1 + 22 + 21 = 44 lines. Add 2 lines for the address `Write`. So the BMP180 **logs one line per byte, plus extras**, not a single summary.

### 3D. Warnings and errors a correct driver can hit
- None in the standard sequence (write 0xF4, wait, read 0xF6..0xF8, read 0xAA..0xBF, read 0xD0).
- [W] 37 only for an empty write. [W] 83 only from monitor/`.repl` setting `Temperature` outside -40..85.
- Unhandled-address warnings for reserved/undefined registers (0.3) only.
- No [E] lines.

### 3E. Interrupts and data-ready
- None. No GPIO, no EOC pin, no IRQ. `SCO` (bit 5 of 0xF4) is cleared **synchronously** in the write callback (`:216`), so a driver polling `SCO` sees "done" on its first read. No timer.

### 3F. Where the data comes from
- Properties: `Temperature` (decimal degC, validated -40..85, `:76-91`) and `UncompensatedPressure` (**int, raw uncompensated counts, not Pa**, `:93`). Both default 0. No RESD, no feeder, no randomness.
- `OutMSB/LSB/XLSB` are refreshed **only when `0xF4` is written** with command 0x0E or 0x14 (`HandleMeasurement` `:196-218`); the values come from the properties at that moment. Changing `Temperature` later does not change the registers until the next command.
- Temperature path (`GetUncompensatedTemperature` `:177-194`) back-computes `UT` from `Temperature` using the model's own calibration (`AC5`, `AC6`, `MC`, `MD` registers) so that the standard datasheet compensation returns that temperature. `b5 = (int)(((uint)(temperature * 10) << 4) - 8)`. INFERRED recomputation (my arithmetic, not run): `Temperature = 0` -> UT 17231 (`0x434F`), `25` -> UT 18254 (`0x474E`). At 0 the `uint` subtraction wraps; correct in an unchecked context (default C#).
- Pressure: `UncompensatedPressure << (8 - OSS)` split into MSB/LSB/XLSB (`:206-211`). Default 0 gives 0x000000.
- **Deterministic with no feeder and no command: YES.** `OutMSB = 0x80`, `OutLSB = 0x00`, `OutXLSB = 0x00` (reset values `:156-163`, the datasheet's "0x8000" power-on). After a `0x2E` command with `Temperature = 0`: `OutMSB/LSB = 0x43, 0x4F` (INFERRED arithmetic above), `OutXLSB = 0`.

### 3G. Identity
- `ChipID` (0xD0) = **0x55** (`:139`).
- Calibration EEPROM 0xAA-0xBF answered with the table in 3B (22 bytes, fixed). `SoftReset` 0xE0 accepted (0xB6).

### 3H. Mode / power
- None. No power state; the model answers any read at any time. `OSS` (bits 6-7 of 0xF4) only shifts the pressure raw value (`:207`). No conversion time is modelled.

---

## 4. LPS25HB (`S/LPS25HB.cs`)

### 4A. Bus and interface
- `public class LPS25HB : II2CPeripheral, IProvidesRegisterCollection<WordRegisterCollection>, ITemperatureSensor, IPressureSensor, ISensor, IUnderstandRESD` (`LPS25HB.cs:19`). **I2C only. No SPI. No GPIO or IRQ of any kind** (no `GPIO`, no `IGPIOSender` in the file).
- Registers are a **16-bit `WordRegisterCollection`** (byte offsets as keys, one `WordRegister` each); reads return `(byte)(Read(addr) & 0xFF)` (`:45`). Unhandled-read semantics as 0.3.
- `Write(byte[] data)` (`:65-87`):
  - If `disableI2C.Value` (CTRL_REG2 bit 3): [W] `"Tried to write data to sensor via disabled I2C"`, return.
  - If `currentAddress != -1` (an address is already latched, no `FinishTransmission` since): `data[0]` is treated as a **data byte** (`WriteToRegister(data[0])`).
  - Else (`currentAddress == -1`): `autoIncrement = (data[0] >> 7) == 1`, `currentAddress = data[0] & 0x7F`. So the **auto-increment flag is bit 7 of the sub-address byte**, as in the datasheet (`0x80 | reg`).
  - Bytes `1..` -> `WriteToRegister` (`:83-86`): [D] log, `RegistersCollection.Write(currentAddress, data)`, then `currentAddress++` only if `autoIncrement`.
- `Read(count)` (`:28-53`): if `disableI2C`: [W] and returns **empty array**; if `currentAddress == -1`: [W] `"Tried to read without addressing"` and returns an **empty array**; else reads `count` bytes, incrementing only if `autoIncrement`.
- `FinishTransmission` (`:89-93`): `autoIncrement = false; currentAddress = -1`. **So a controller that issues STOP between the address write and the read breaks the read** (INFERRED about which controllers do that). The expected sequence is write-address, repeated START, read, STOP.
- No log on the address-byte-only write and no log on any read.

### 4B. Registers (`DefineRegisters` `:244-345`; enum `:464-489`)

| Addr | Name | Access | Reset | Behaviour |
|---|---|---|---|---|
| 0x08 | REF_P_XL | RW | 0 | value field (`:246-247`) |
| 0x09 | REF_P_L | RW | 0 | (`:248-249`) |
| 0x0A | REF_P_H | RW | 0 | (`:250-251`; the enum's own comment says `REF_P_H`, field name `REF_P_HL`) |
| 0x0F | WHO_AM_I | R | | `valueProviderCallback: _ => 0b10111101` = **0xBD** (`:253-254`) |
| 0x10 | RES_CONF | RW | **0x1B** (from `Reset()`) | `AVGP` bits 0-1, `AVGT` bits 3-4; bits 2, 5-7 untagged (silent). `Reset()` forces both to 3 (`:58-59`), so the register reads `0x1B` (INFERRED arithmetic `3 | 3<<3`) |
| 0x20 | CTRL_REG1 | RW | 0 | SIM bit0, `RESET_AZ` bit1 (W1C, clears REF_P), BDU bit2, `DIFF_EN` bit3 **tag**, `ODR` bits 4-6, `PD` bit7 (`:260-266`) |
| 0x21 | CTRL_REG2 | RW | 0 | ONE_SHOT, AUTOZERO, `SWRESET` (W1C -> `Reset()`), `I2C_DIS` bit3, FIFO_MEAN_DEC/STOP_ON_FTH/FIFO_EN **tags**, `BOOT` (W1C -> `Reset()`) (`:268-276`) |
| 0x22 | CTRL_REG3 | RW | 0 | `INT_S` bits 0-1, bits 2-5 reserved, `PP_OD`, `INT_H_L` (`:278-282`); **no pin is driven** |
| 0x23 | CTRL_REG4 | RW | 0 | `DRDY`, `F_OVR`, `F_FTH`, `F_EMPTY` **tags**; bits 4-7 reserved (`:284-289`) |
| 0x24 | INTERRUPT_CFG | RW | 0 | tags (`:291-294`) |
| 0x25 | INT_SOURCE | RW | 0 | tags (`:296-299`) |
| 0x27 | STATUS_REG | R | | `T_DA` = 1, `P_DA` = 1, `T_OR` = 0, `P_OR` = 0 (**constants**, `:301-307`) -> reads **0x03** |
| 0x28 | PRESS_OUT_XL | R | 0 | `pressureOutputLow` (`:309-310`) |
| 0x29 | PRESS_OUT_L | R | 0 | (`:312-313`) |
| 0x2A | PRESS_OUT_H | R | 0 | (`:315-316`) |
| 0x2B, 0x2C | TEMP_OUT_L/H | R | 0 | (`:318-322`) |
| 0x2E | FIFO_CTRL | RW | 0 | WTM_POINT, F_MODE (`:324-326`) |
| 0x2F | FIFO_STATUS | R | | FSS = 0 (const), `EMPTY_FIFO`/`OVR`/`FTH_FIFO` tags (`:328-332`) |
| 0x30, 0x31 | THS_P_L/H | RW | 0 | (`:334-338`) |
| 0x39, 0x3A | RPDS_L/H | RW | 0 | (`:340-344`) |

**Write callbacks that ignore the value (read from the code, important).** `RESET_AZ` (CTRL_REG1 bit 1, `:262`), `SWRESET` (CTRL_REG2 bit 2, `:271`) and `BOOT` (CTRL_REG2 bit 7, `:276`) are `WriteOneToClear` flags with `writeCallback: (_, _) => ...`. Write callbacks run on **every** write to the register regardless of the written bit (`REG/RegisterField.cs:140-148`, `REG/PeripheralRegister.cs:630-640`). So:
- **every write to CTRL_REG1 (0x20) runs `ResetAutoZero()`**, zeroing `REF_P_XL/L/H` (`:347-352`);
- **every write to CTRL_REG2 (0x21) runs `Reset()` twice** (SWRESET and BOOT callbacks), even for 0x00 or 0x40.
- `Reset()` sets `currentAddress = -1` in the middle of `WriteToRegister`. After a CTRL_REG2 write, the trailing `if(autoIncrement) currentAddress++` (`:181-184`) leaves `currentAddress` at **0** for an auto-increment burst, or -1 otherwise. So an auto-increment write burst through 0x21 (e.g. `[A0 90 00 00]` = CTRL_REG1, 2, 3) lands the next byte on offset 0x00 (`[D] Writing 0x00 to register 0x00`, then `[W] Unhandled write to offset 0x0, value 0x0.`) instead of CTRL_REG3; a non-incrementing buffer with extra bytes hits [W] `:177`.
- `Reset()` also re-forces `RES_CONF` fields AVGP/AVGT to 3 (reads 0x1B) on every CTRL_REG2 write.
- Single-register writes (`[21 xx]`, `[20 xx]`) are unaffected on the bus: the register's stored fields (ONE_SHOT, AUTOZERO, I2C_DIS, ODR, PD...) update normally.

`Reset()` (`:55-63`) sets `currentAddress = -1`, forces `AVGP`/`AVGT`, sets `temperature` and `pressure` cached fields. **It does NOT reset the register collection, nor the OUT registers.** A `SWRESET`/`BOOT` write therefore leaves CTRL_REG1 etc. unchanged (INFERRED consequence; the method body is what it is).

Undefined (e.g. 0x0B-0x0E, 0x11-0x1F, 0x26, 0x32-0x38): [W] 0.3.

### 4C. Every log call

| Line | Lvl | Format string (verbatim) | Fires when |
|---|---|---|---|
| `:32` | W | `"Tried to read data from sensor via disabled I2C"` | `Read` with `I2C_DIS` set |
| `:38` | W | `"Tried to read without addressing"` | `Read` with no latched address |
| `:69` | W | `"Tried to write data to sensor via disabled I2C"` | `Write` with `I2C_DIS` set |
| `:106` | N | `"RESD stream set to {0}"` | pressure RESD feed attached |
| `:119` | N | `"RESD stream set to {0}"` | temperature RESD feed attached |
| `:174` | D | `$"Writing 0x{data.ToHex()} to register 0x{currentAddress:X02}"` (interpolated; `ToHex` is 2-digit upper hex `Misc.cs:1648,1977-1985`) | every data byte written |
| `:177` | W | `"Tried to write without specyfing register"` | `WriteToRegister` with `currentAddress == -1`. **Reachable**: a write callback calls `Reset()` mid-buffer (see 4B), so any further data byte in the same buffer hits this (typo verbatim). The preceding [D] line then prints `register 0xFFFFFFFF` (INFERRED: `X02` on -1) |
| `:191` | W | `$"Value {pressureValue} is not in valid pressure range, ignoring."` | `Pressure` setter or RESD sample outside 260..1260 hPa |
| `:206` | W | `$"Value {temperatureValue} is not in valid temperature range, ignoring."` | `Temperature` setter or RESD sample outside -30..105 degC |

Base logs: 0.3, 0.3a.

### 4C'. Exact Noisy-level transcripts (I2C)

**Write CTRL_REG1 (0x20) = 0x90**, buffer `[20 90]`:
```
[D] Writing 0x90 to register 0x20
```
= 1 line, no Warning (`PD`, `ODR` are real fields). With the auto-increment bit, buffer `[A0 90]` prints the same line (the address is masked to 0x20).

**Identity read**, `Write([0F])` + `Read(1)`: **0 lines**. Returns `BD`. If a STOP/`FinishTransmission` separates the two, `Read` returns `[]` and logs `[W] Tried to read without addressing`.

**Data read PRESS_OUT_XL (0x28) with auto-increment**, `Write([A8])` + `Read(3)`: **0 lines** (no read logging at all). `Read(5)` (0x28..0x2C: pressure + temperature): 0 lines. Without bit 7 in the sub-address the same register is returned N times (no auto-increment).

### 4D. Warnings and errors a correct driver can hit
- **`CTRL_REG4` (0x23)** writes with `DRDY` (bit 0), `F_OVR`, `F_FTH`, `F_EMPTY`: [W] tag warnings, e.g. `Unhandled write to offset 0x23. Unhandled bits: [0] when writing value 0x1. Tags: DRDY (0x1).`
- **`CTRL_REG1`** with `DIFF_EN` (bit 3) set: [W]. **`CTRL_REG2`** with `FIFO_EN`/`STOP_ON_FTH`/`FIFO_MEAN_DEC`: [W]. **`INTERRUPT_CFG`** (`PH_E`, `PL_E`, `LIR`): [W]. **`FIFO_CTRL`** is real, silent.
- `CTRL_REG3` bits 2-5 are reserved; a 1 there warns.
- A read after a STOP-separated address write: [W] 38, returns empty (a bus-level failure).
- Undefined-register read/write (0.3).
- Setting `Pressure` outside 260..1260 or `Temperature` outside -30..105 from the monitor: [W] 191/206 and the value is ignored (not clamped).
- A standard init using separate single-register writes (`CTRL_REG1 = 0x90` or `0xB0`, `RES_CONF`) yields **0 Warnings** unless it touches the tags above. `CTRL_REG2 = 0x40` would warn on tag FIFO_EN (bit 6). A multi-register write burst crossing 0x21 produces the offset-0x0 Warning described in 4B. A driver that writes `REF_P` and then `CTRL_REG1` loses the reference (4B).
- No [E] lines.

### 4E. Interrupts and data-ready
- **None.** No output pin. `CTRL_REG3`/`CTRL_REG4` bits are stored (`INT_S`, `PP_OD`, `INT_H_L`) or tags (`DRDY`, etc.) but nothing is driven.
- `STATUS_REG` always reads `0x03` (T_DA | P_DA), so a **polling** driver proceeds immediately.

### 4F. Where the data comes from
- No randomness, no seed.
- Properties: `Pressure` (hPa, **`decimal`**, setter validates 260..1260 and writes the 24-bit `PRESS_OUT` registers as `(int)(p*4096)`, `:141-164,187-200`), `Temperature` (degC, validates -30..105, writes `TEMP_OUT` as `(int)((T-42.5)*480)`, `:124-139,202-214`). The raw getters read the output registers back: `Temperature = 42.5 + raw/480`, `Pressure = (count - ...)/4096`. The `Pressure` getter subtracts `REF_P` unconditionally (`:160`) in addition to the autozero branch (`:154-158`), so with non-zero `REF_P` the getter differs from the set value (INFERRED, harmless for raw reads).
- RESD: `FeedPressureSamplesFromRESD`, `FeedTemperatureSamplesFromRESD` (`:95-120`). Both call `OutputFrequency` (`:354-362`), which **throws `RecoverableException("Sensor is in one-shot mode")` if `ODR` = 0** or `"Illegal OutputDataRate"` for 5-7. Set `CTRL_REG1.ODR` first. RESD pressure sample is divided by 100 (`:219`), temperature by 100 (`:225`) to hPa/degC.
- **Deterministic with no feeder: YES.** All output registers are 0 after construction (`PRESS_OUT` = `00 00 00`, `TEMP_OUT` = `00 00`; INFERRED: field default 0, `Reset()` does not touch them). `Temperature` getter then returns 42.5, `Pressure` 0 hPa. `STATUS_REG` = 0x03.
- The platform must set `Pressure`/`Temperature` (or RESD) to get a realistic value.

### 4G. Identity
- `WHO_AM_I` (0x0F) = **0xBD** (`0b10111101`, `:254`).
- `REF_P_XL/L/H` (0x08-0x0A): readable, 0 (or whatever the driver wrote). Answers with 0. `RES_CONF` reads 0x1B. `STATUS_REG` 0x03.

### 4H. Mode / power
- **Not enforced.** `PD` (bit 7) and `ODR` do not gate data. Outputs hold their last value whatever the power state.
- `ODR` matters only for RESD feeder rate (4F), and one-shot (ODR = 0) is only rejected by `OutputFrequency`.
- `ONE_SHOT` bit is a plain flag, no behaviour.
- No averaging, no FIFO behaviour.

---

## 5. ICP-101xx (`S/ICP_101xx.cs`)

### 5A. Bus and interface
- `public class ICP_101xx : II2CPeripheral, ISensor, ITemperatureSensor, IUnderstandRESD` (`:21`). **I2C only. No register collection, no GPIO.** The protocol is **16-bit commands**, not registers.
- `Write(byte[] data)` (`:49-64`):
  - length 0: [E] `"Unexpected write with no data."`
  - length 1: [W] `"Malformed command."`
  - else [D] `"Received command: {0}"` (hex list of the whole buffer), `command = (Command)BitHelper.ToUInt16(data, 0, reverse: false)` (**MSB first**: `0x48A3` is sent as `48 A3`; INFERRED for `ToUInt16` by its twin `GetBytesFromValue(reverse:false)` which was read at `BitHelper.cs:460-474` and is big-endian; `ToUInt32` not read), then `HandleWrite(data, 2)`.
- `Read(int count)` (`:66-77`): if no command: [E] `"Attempted read, but Command not specified."` returns empty. Else it builds `InsertCrc(HandleRead(count)).Concat({0,0}).Take(count)`: the reply is the handler's bytes with a **CRC byte after each 2-byte word**, padded with zeros if `count` is larger, truncated to `count`. [D] log.
- CRC: `new CRCEngine(0x31, 8, false, false, 0xFF, 0x00)` (`:25`), the Sensirion-style CRC-8 (poly 0x31, init 0xFF). INFERRED from the constructor arguments.
- **State persists:** `FinishTransmission` is empty (`:93-96`, comment explains). The last command stays active until `SoftReset` (0x805D) or `Reset()`. So a driver may re-read the same measurement without re-sending the command, and a read needs no STOP/START discipline.

### 5B. "Registers" (command table, `DefineCommandHandlers` `:196-294`; enum `:492-506`)

| Command (hex, sent MSB first) | Name | Read returns (before CRC insertion) | Notes |
|---|---|---|---|
| 609C | LowPowerTemperaturePressure | T(2) + P(4) | measurement, T then P |
| 401A | LowPowerPressureTemperature | P(4) + T(2) | |
| 6825 | NormalTemperaturePressure | T(2) + P(4) | |
| 48A3 | NormalPressureTemperature | P(4) + T(2) | |
| 70DF | LowNoiseTemperaturePressure | T + P | |
| 5059 | LowNoisePressureTemperature | P + T | |
| 7866 | UltraLowNoiseTemperaturePressure | T + P | |
| 58E0 | UltraLowNoisePressureTemperature | P + T | |
| 805D | SoftReset | (none) | write handler `SoftwareReset()` -> `command = null` (`:254-259`) |
| EFC8 | Id | `00 08` (`BitHelper.GetBytesFromValue(0b001000, 2, reverse: false)`, `:261-263`) | |
| C595 | CalibrationParametersPointer | (none) | write needs `[C5 95 00 66 9C]`, address 0x00669C, resets the calibration index to 0 (`:265-284`) |
| C7F7 | CalibrationParameters | next calibration word, 2 bytes (`:286-293`) | index rotates through 4 words per **Read call** |

- All four power modes compute **identical** values (`GetTemperature(OperationMode _)` ignores the mode, `:372-375`; `GetPressure` takes `mode` but the coefficients do not depend on it, `:399-418`). No timing/conversion delay, so reads right after a command are valid.
- Any unknown command code: [W] 444/449 `Unhandled write, command: {0}.` and `Unhandled read, command: {0}.`
- Calibration words (`:113`): `{620, 17540, 9475, 2945}` = `0x026C, 0x4484, 0x2503, 0x0B81`; settable with `SetCalibrationValue(index, value)` (`:98-105`).
- Reply layout of a measurement read of 9 bytes: word1 (2 bytes) + CRC + word2 + CRC + word3 + CRC. T-first commands: T, P_hi word, P_lo word. P-first: P_hi, P_lo, T. The 4-byte pressure value is `(uint)(raw << 8)` big-endian (`:393-397`), so the P_lo word is `{raw_bits_7..0 << 8 ..}` with a trailing 0x00 (INFERRED layout).
- INFERRED CRC values (my computation, same polynomial): `Id` reply `00 08 38`; calibration reads `02 6C 9E`, `44 84 35`, `25 03 79`, `0B 81 D0`.

### 5C. Every log call

| Line | Lvl | Format string (verbatim) | Fires when |
|---|---|---|---|
| `:53` | E | `"Unexpected write with no data."` | empty `Write` |
| `:58` | W | `"Malformed command."` | 1-byte `Write` |
| `:61` | D | `"Received command: {0}"` | every `Write` with >= 2 bytes |
| `:70` | E | `"Attempted read, but Command not specified."` | `Read` before any command or after `SoftReset` |
| `:75` | D | `"Reading data with CRC: {0}, current command: {1}, requested: {2} bytes"` | every `Read` (`{1}` = `0x{command:X} ({command})`, e.g. `0xEFC8 (Id)`, `:440`) |
| `:182` | W | `"Pressure value: {0} is out of range and it will be clamped. Supported range: {1} - {2}"` | `Pressure`/`DefaultPressure` outside 30000..110000 Pa |
| `:191` | W | `"Temperature value: {0} is out of range and it will be clamped. Supported range: {1} - {2}"` | outside -45..85 degC |
| `:270` | E | `"Read from OTP received not enough bytes, got: {0}, total of: {0} are expected"` | calibration-pointer write shorter than 5 bytes (the source passes `{0}` twice, so the second number repeats the first) |
| `:276` | E | `"Read from OTP from offset 0x{0:X}, different than 0x{1:X} is unsupported!"` | pointer address != 0x00669C |
| `:303` | N | `"Registered command 0x{0:X}."` | at construction, 12 times (`{0:X}` of the enum prints hex such as `609C`) |
| `:309` | W | `"Attempted write while handling read-only command '{0}'."` | extra bytes after a read-only command's 2-byte code |
| `:316` | W | `"Attempted read while handling write-only command '{0}'."` | `Read(count != 0)` while `command` is `CalibrationParametersPointer` (**`HandleRead` is called with the requested `count`, which is non-zero**, so reading after the pointer write without first writing `C7F7` warns; the read still returns zero-padded bytes). After `SoftReset` the command is null, so a read is [E] 70 instead |
| `:385` | W | `"Raw pressure value is invalid: {0}. The sensor might be miscalibrated."` | raw P < 0 or > 0xFFFFFF |
| `:389` | N | `"p_raw={0}, p_long={1}"` | every pressure computation |
| `:417` | N | `"A={0} B={1} C={2}, p_LUT0={3}, p_LUT1={4}, p_LUT2={5}"` | every pressure computation |
| `:444` | W | `"Unhandled write, command: {0}."` | unknown command (default handler) |
| `:449` | W | `"Unhandled read, command: {0}."` | unknown command (default handler) |

### 5C'. Exact Noisy-level transcripts (I2C)

**Measurement command** (normal, P then T = `48 A3`), buffer `[48 A3]`:
```
[D] Received command: [0x48, 0xA3]
```
= 1 line, no Warning (a 2-byte write to a read-only command: `offset == data.Length`, `:307`).

**Data read, `Read(9)`:** per call 3 lines, one per computation + the read summary (the computation runs eagerly inside the `Concat` regardless of `count`, so every `Read` logs both [N] lines even for `Read(1)`):
```
[N] A=132784 B=-346442540656 C=267157, p_LUT0=3679377, p_LUT1=6296196, p_LUT2=12201686
[N] p_raw=3103431, p_long=3103431
[D] Reading data with CRC: [0x..., ...], current command: 0x48A3 (NormalPressureTemperature), requested: 9 bytes
```
(the numeric values are my own recomputation, INFERRED, for the model defaults `T = 0 degC`, `P = 30000 Pa`, calibration `{620,17540,9475,2945}`; they are examples of the line shape only). Total **3 lines per `Read` call regardless of N**. No Warning with defaults (the raw value 3103431 is inside 0..0xFFFFFF). INFERRED sanity check for typical values: `T = 25, P = 101325` gives `p_raw = 10520286`, valid.

**Identity**, `Write([EF C8])` + `Read(3)`:
```
[D] Received command: [0xEF, 0xC8]
[D] Reading data with CRC: [0x0, 0x8, 0x38], current command: 0xEFC8 (Id), requested: 3 bytes
```
= 2 lines. (CRC byte INFERRED.)

**OTP calibration read sequence** (the standard driver probe):
1. `Write([C5 95 00 66 9C])`: `[D] Received command: [0xC5, 0x95, 0x0, 0x66, 0x9C]` (1 line; the handler sets the index to 0).
2. Four times: `Write([C7 F7])` then `Read(3)`: each is `[D] Received command: [0xC7, 0xF7]` and `[D] Reading data with CRC: [0x2, 0x6C, 0x9E], current command: 0xC7F7 (CalibrationParameters), requested: 3 bytes` (INFERRED bytes, then `0x44 0x84 0x35`, ...). The index advances **once per `Read` call**; re-sending `C7F7` is not required for the next word (`command` persists) but is harmless.
3. 2 lines per word; 9 lines for the whole probe. No Warning.

**Soft reset:** `Write([80 5D])` -> `[D] Received command: [0x80, 0x5D]`, then the command is cleared. A subsequent `Read` is [E] 70.

### 5D. Warnings and errors a correct driver can hit
- **None in a correct sequence** (ID, OTP pointer + 4 reads, measurement command + read).
- [W] 316 (`Attempted read while handling write-only command`) if the driver `Read`s while the active command is the OTP **pointer** (`C595`) rather than `C7F7`.
- [E] 70 for a read after `SoftReset` with no new command.
- [W] 385 if the user sets `DefaultPressure`/`DefaultTemperature` and the model's calibration yields an out-of-range raw value (the source comment `:107-112` says the default calibration vector was chosen to avoid this).
- [W] 182/191 for out-of-range properties; the value is clamped, not rejected.
- [E] 270/276 for a malformed OTP pointer write.
- [W] 58 for a single-byte write; [E] 53 for an empty write.
- 12 [N] "Registered command" lines at construction (5C).

### 5E. Interrupts and data-ready
- **None.** No GPIO. No status. A read right after a command always succeeds (no conversion delay).

### 5F. Where the data comes from
- `DefaultPressure` (long, **Pa**, constructor sets it to `MinPressure` = 30000, `:28`) and `DefaultTemperature` (decimal degC, default 0, `:146-154`). Both clamp with [W].
- **`Pressure` and `Temperature` setters throw `RecoverableException`** (`:140-141,167-168`): `"Explicitly setting pressure is not supported by this model. Pressure should be provided from a RESD file or set via the 'DefaultPressure' property"` / the temperature equivalent. Generic `sensor Temperature` setters will fail; use `DefaultPressure`/`DefaultTemperature`.
- RESD: `FeedTemperatureSamplesFromRESD`, `FeedPressureSamplesFromRESD` (`:79-91`). **No feeder thread**: samples are fetched at read time using current virtual time (`TryGetSampleFromRESDStream` `:346-365`); RESD pressure in Pa after `/1000` (`:338`), temperature `/1e3` (`:326`).
- Temperature word: `(ushort)((T + 45) * 65536 / 175)` (`:372-375`). Default T = 0 gives 16852 = `0x41D4` (INFERRED arithmetic).
- Pressure: 24-bit `p_raw` via the datasheet LUT formula (`:377-418`) from `T`, `P` and the calibration words.
- **Deterministic with no feeder: YES.** `T = 0 degC`, `P = 30000 Pa`: temperature word `41 D4`, raw pressure 3103431 (INFERRED computation, 0x2F5AC7).
- No randomness, no seed.

### 5G. Identity
- Command `EFC8` -> `00 08` + CRC (`:261-263`): ID value 0x08 in bits 5:0 (the datasheet ICP-10101/10125 ID). INFERRED CRC `0x38`.
- Calibration: command `C595` + address `00 66 9C`, then `C7F7` x 4, each returns 2 bytes + CRC: `0x026C, 0x4484, 0x2503, 0x0B81` by default (`:113`).

### 5H. Mode / power
- None. No sleep/wake. The model enforces only the command order: a `Read` needs a prior command (otherwise [E] 70). Power mode (`LowPower`/`Normal`/`LowNoise`/`UltraLowNoise`) selects the command code only; the output is identical.

---

## 6. Cross-model summary

| Model | Bus | Identity | Logs a config write? | Logs a data read? | Lines per N-byte burst | Warning lines in a plain correct sequence | Drives a data-ready GPIO? | Deterministic data without a feeder? |
|---|---|---|---|---|---|---|---|---|
| ICM-20948 | SPI and I2C (+ I2C container) | `WHO_AM_I` 0x00 = **0xEA** | SPI: 5 lines (+1 [W] if a tag bit changes); I2C: 3 lines (+1 [W] if a tag bit changes) | SPI: yes, 2 lines per byte; I2C: one summary line | SPI **2N + 3**; I2C **4** (independent of N) | `PWR_MGMT_1 = 0x01`: 1; SPI `USER_CTRL` I2C_IF_DIS: +1; each other tag bit written 1: +1; data reads 0 | **No.** `IRQ` exists but the model never sets any status flag | Yes: accel and gyro 0; temp `0xE49D` |
| LSM6DSO | SPI and I2C | `WHO_AM_I` 0x0F = **0x6C** | SPI 3 lines; I2C 2 lines (+[W] if a tag bit) | Yes: 1 line per byte | SPI **N + 2**; I2C **N + 1** | `CTRL3_C = 0x44`: 1; each `STATUS_REG` poll: 1; with no sample source, **6 per 6-byte accel/gyro burst** | **No** (INT1 = FIFO overrun only; DRDY bits are tags) | Zeros + warning, **until BDR set in `FIFO_CTRL3`, a monitor `Feed...Sample`, or RESD**; ODR alone creates no sample; temp `0xE700` |
| BMP180 | I2C only | `ChipID` 0xD0 = **0x55** | Yes: 4 lines for `0xF4`; 2 for a plain data write | Yes: 1 + N + auto-increment lines | `1 + N + increments` (+2 for the address write); `Read(3)` at 0xF6 = 6; `Read(22)` at 0xAA = 44 | 0 | No (no GPIO; SCO clears instantly) | Yes: `OUT` = `80 00 00` until the first `0xF4` command; then from `Temperature`/`UncompensatedPressure` (default 0) |
| LPS25HB | I2C only | `WHO_AM_I` 0x0F = **0xBD** | Yes: 1 [D] line | **No**: reads log nothing | **0** | 0 (unless `CTRL_REG4`/`DIFF_EN`/FIFO tags are written; a STOP between address write and read gives a [W] and an empty reply) | **No** (no pin at all) | Yes: `PRESS_OUT` `00 00 00`, `TEMP_OUT` `00 00`; `STATUS_REG` 0x03 |
| ICP-101xx | I2C only (16-bit commands) | command `EFC8` -> `00 08` (+CRC) | Yes: 1 [D] line (`Received command: ...`) | Yes: 1 [D] line per `Read`, plus 2 [N] lines per measurement read | **3** per measurement `Read`, **1** per ID/OTP `Read`, independent of N | 0 | No | Yes: T = 0 degC, P = 30000 Pa -> `41 D4`, raw P 3103431 |

---

## 7. The five most decision-relevant facts

1. **Logging volume is not uniform.**
   - Per-byte Noisy lines: ICM-20948 over SPI (`2N + 3`: 15 lines for a 6-byte burst, 31 for 14), LSM6DSO (`N + 2`), BMP180 (`1 + N + increments`).
   - Zero lines on reads: LPS25HB.
   - A fixed line count: ICM-20948 over I2C (4), ICP-101xx (3 per measurement read, independent of N).
   - A harness that counts or matches Noisy lines per transaction must be model-specific.
2. **Two models never report data ready and one cannot produce data from ODR alone.**
   - The ICM-20948 `DATA_RDY_STATUS` and `INT_STATUS_1` always read 0 and its `IRQ` is never raised by the model.
   - The LSM6DSO `STATUS_REG` is unimplemented (a Warning per poll) and `INT1_DRDY_*` are tags.
   - Neither drives a DRDY GPIO. Only polling-free or fixed-delay drivers work.
   - The LSM6DSO also yields zeros plus six Warnings per accel burst unless `FIFO_CTRL3.BDR_XL`/`BDR_GY` is set (default feeder), a sample is fed from the monitor, or RESD is used. `CTRL1_XL.ODR_XL` alone does nothing for data.
3. **A plain correct init already produces Warnings from tag bits.**
   - ICM-20948: `PWR_MGMT_1 = 0x01` (CLKSEL), and a SPI driver's `USER_CTRL = 0x10`.
   - LSM6DSO: `CTRL3_C = 0x44` (BDU).
   - These are real [W] lines. Written tag bits also read back as 0 (never stored).
   - Reset-type bits do not behave like silicon: LSM6DSO `CTRL3_C.SW_RESET` never triggers a reset (it clears nothing but `IF_INC` if the written byte lacks bit 2); LPS25HB's `CTRL_REG1` write always zeroes `REF_P` and its `CTRL_REG2` write always calls `Reset()` and drops the latched address mid-buffer (a multi-register burst through 0x21 spills a byte onto 0x00 with a Warning).
   - BMP180 and ICP-101xx produce no Warnings in a standard sequence; LPS25HB produces none for single-register writes.
4. **Bus-framing assumptions.**
   - LPS25HB: needs bit 7 set in the sub-address for auto-increment, and breaks (a [W] and an empty read) if `FinishTransmission` is called between the address write and the read.
   - LSM6DSO over I2C: rejects a read after STOP or after a write that carried data bytes ([W] `Tried to read in invalid state`).
   - ICM-20948: tolerates both.
   - ICP-101xx and BMP180: state persists across STOP.
   - All ICM-20948 and LSM6DSO SPI transactions rely on the controller (or, for the ICM, a CS GPIO on pin 0) to call `FinishTransmission`.
5. **Defaults are deterministic and mostly zero, but set differently per model.**
   - ICM-20948 and LSM6DSO: platform properties `Default{Acceleration,AngularRate}{X,Y,Z}` (ICM range-checked against full scale, LSM unrestricted), plus RESD. ICM `Temperature` and ICP `Pressure`/`Temperature` setters throw: use `DefaultTemperature`, `DefaultPressure` (Pa).
   - LPS25HB: `Pressure` (hPa, 260..1260) and `Temperature` (-30..105) properties write the output registers.
   - BMP180: `UncompensatedPressure` is a raw count, not Pa; its `OUT_*` registers update only on an `0xF4` command.
   - Identity values: ICM 0xEA, LSM 0x6C, BMP 0x55, LPS 0xBD, ICP command `EFC8` -> `00 08` + CRC.
   - No model has randomness or seed. All RESD feed threads take their rate at the time of the feed call.

## 8. Items not verified (explicit)
- `BitHelper.ToUInt32(byte[], int, int, bool)` was not read; ICP command byte order (MSB first) is INFERRED from `GetBytesFromValue`.
- The CRC engine (`CRCEngine`) implementation was not read; the CRC values in 5B/5C' are my recomputation of CRC-8 poly 0x31 init 0xFF.
- Numeric ICP/BMP180/ICM/LSM examples were recomputed outside Renode (Python, truncating-division semantics) and are labelled INFERRED where used.
- The Renode logger line prefix and `.repl` wiring (IRQ pin names, SPI CS) were not read; no `.repl` in the pinned tree references these five models.
- Which controller calls `FinishTransmission` between a write and a read was not examined for the target SoC.
- How a log backend renders a collapsed-repeat `Count` was not read (0.1). Whether the target run keeps `collapse-repeated-log-entries` at its default (`true`) is not known.
- The BMP180 enum hex padding (`(0x000000F4)` versus `(0xF4)`, 3C) rests on .NET enum `X` formatting rules, not on a run.
- The LSM6DSO `SW_RESET` no-op (2B) is derived by tracing `WriteInner`, not by a run.
