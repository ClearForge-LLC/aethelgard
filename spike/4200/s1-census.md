<!-- AET-WO-4200 spike: S1 census by a Sonnet subagent, read-only, at infra 066a7f13 / renode ab721d88. Every claim here is [read-sub] unless FEEDBACK marks it [read]. -->
# Renode 1.17.0 SPI-peripheral census (read-only)

Paths: INFRA = /home/forge/.cache/aet-2301/src/infra/src/Emulator ; P = INFRA/Peripherals/Peripherals ; RN = /home/forge/.cache/aet-2301/src/renode.
Infra HEAD reads 066a7f13c. Interface: `ISPIPeripheral : IPeripheral { byte Transmit(byte data); void FinishTransmission(); }` (INFRA/Main/Peripherals/SPI/ISPIPeripheral.cs:10-15). No chip-select method exists in the interface: CS is either (a) implied by the controller calling FinishTransmission() or (b) a GPIO input on the peripheral (IGPIOReceiver.OnGPIO(0, level)).
Log-visibility [inferred from Renode knowledge, not read here]: Info/Warning/Error show at the default level; Debug and Noisy do not.

## ANSWER SUMMARY (modelled or not)
| Part | Modelled? |
|---|---|
| ICM-42688-P, ICM-20602, MPU-6000/6050/9250/9150 | NO. Case-insensitive search of infra+renode: zero hits. |
| BMI088 (accel/gyro) (also BMI270/160/085) | NO. Zero hits. |
| BMP388 / BMP3xx | NO. Zero hits. (BME280 I2C-only, id 0x60, P/I2C/BME280.cs:171-172; BMP180 I2C chip id 0x55, P/Sensors/BMP180.cs:139; also LPS25HB, ICP_101xx are I2C-only.) |
| DPS310/DPS368, MS5611/MS5607 | NO. Zero hits. |
| Winbond / W25Q* / W25N / W25X | NO model and no literal "winbond"/"w25" string anywhere in infra or renode. Closest: GenericSpiFlash (settable ID: manufacturerId 0xEF memoryType 0x40 capacityCode 0x17 would be EF 40 17 = W25Q64JV [ID value from outside knowledge, inferred]); NANDFlash returns hard-coded ID bytes 00 EF BC (manufacturer byte 0xEF) but is a NAND, not NOR. |
| ICM20948 | YES, P/Sensors/ICM20948/*.cs: SPI and I2C both (class implements II2CPeripheral, ISPIPeripheral, IGPIOReceiver, ICM20948.cs:21). WHO_AM_I bank0 reg 0x00 = 0xEA (ICM20948_Bank0Registers.cs:15; enum :274). |
| GenericSpiFlash, Micron_MT25Q, Macronix_MX25R, ISSI_IS25WP, Cypress_S25H | YES, SPI only (GenericSpiFlash is the base; others derive). GigaDevice_GD25LQ, Puya_P25Q are separate classes, SPI only. |

## A. Storage / flash models (ISPIPeripheral)

### A1. GenericSpiFlash — P/SPI/GenericSpiFlash.cs:22 `class GenericSpiFlash : ISPIPeripheral, IGPIOReceiver, ISFDPPeripheral`
- Part: generic JEDEC NOR flash (also used by QSPI controllers STM32H7_QuadSPI, ZynqMP_GQSPI as container type).
- Constructor verbatim (:24-28):
```
public GenericSpiFlash(MappedMemory underlyingMemory, byte manufacturerId, byte memoryType, byte? capacityCode = null,
    bool writeStatusCanSetWriteEnable = true, byte extendedDeviceId = DefaultExtendedDeviceID,
    byte deviceConfiguration = DefaultDeviceConfiguration, byte remainingIdBytes = DefaultRemainingIDBytes,
    // "Sector" here is the largest erasable memory unit. ...
    int sectorSizeKB = DefaultSectorSizeKB, bool useStatusRegisterStubs = true)
```
  So YES: JEDEC bytes are settable from .repl by name. manufacturerId and memoryType are REQUIRED (no default). Underlying memory size must be a power of two (:30-33, throws ConstructionException).
- Defaults: remainingIdBytes 0x10 (:760), extendedDeviceId = 1<<6 = 0x40 (:761, DeviceGeneration :759), deviceConfiguration 0x00 (:762), sectorSizeKB 64 (:763). capacityCode default (:70, GetCapacityCode :232-251): log2(size) for size <= 32MB (8MB -> 0x17, 16MB -> 0x18); size > 32MB: log2-26+0x20 (64MB -> 0x20).
- Identity: command 0x9F (Commands.ReadID :772; also 0xAF MultipleIoReadID :773) -> RecognizeOperation :287-290 sets Operation=ReadID; 0 dummy bytes (GetDummyBytes :260-274 lists only fast-read/SFDP commands -> 1 dummy). Response bytes, one per Transmit after the command byte, from deviceData built in GetDeviceData :569-580: [0]=manufacturerId [1]=memoryType [2]=capacityCode [3]=remainingIdBytes [4]=extendedDeviceId [5]=deviceConfiguration, rest 0 up to 20 bytes. Read at :599-609.
  => default read of 9F gives `mfr, type, cap, 0x10, 0x40, 0x00`.
- Transfer: `public byte Transmit(byte data)` :127; `public void FinishTransmission()` :88; also `public void OnGPIO(int number, bool value)` :79 — CS: if number==0 && value (high = deasserted) calls FinishTransmission(). So it works BOTH ways: controller calling FinishTransmission() or GPIO pin 0 going high. FinishTransmission resets the decoder state (:106-107) and clears WEL for program/erase/write-reg (:99-105).
- Logging in the transfer path (all `this.Log`):
  - :83 Noisy "Chip Select is deasserted."
  - :95 Warning "Transmission finished in the unexpected state: {0}" (finish while still in RecognizeOperation / accumulating address bytes — i.e. a CS toggle with zero or too few bytes)
  - :129 Noisy "Transmitting data 0x{0:X}, current state: {1}"   <- every byte
  - :347 Noisy "Setting write enable latch"; :351 Noisy "Unsetting write enable latch"; :380 Noisy "Performing bulk/chip erase"; :396 Noisy "Entering 4-byte address mode"; :400 Noisy "Exiting 4-byte address mode"
  - :459 Warning "Unhandled parameterless command {0}"; :462 Error "Command decoding failed on byte: 0x{0:X} ({1})."  (unknown opcode)
  - :465 Noisy "Decoded operation: {0}, write enabled {1}"
  - :472 Error "Trying to write a register, but write enable latch is not set"; :487 Error "Trying to write to register {0} with more than expected 2 bytes."; :508 Warning "Trying to write 0x{0} to unsupported register \"{1}\""; :544 Warning "Trying to read from unsupported register \"{0}\""
  - :564 Noisy "Address accumulated: 0x{0:X}"; :588 Noisy "Handling dummy byte in {0} operation, {1} remaining after this one"
  - :606 Error "Trying to read beyond the length of the device ID table."; :621 Error "Memory write operations are disabled."; :631 Warning "Unhandled operation encountered while processing command bytes: {0}"
  - :635 Noisy "Handled command: {0}, returning 0x{1:X}"   <- every data byte
  - :643 Warning "Tried to read SFDP signature byte out of range at position {0}"
  - :200 Error "Cannot read from address 0x{0:X} because it is bigger than configured memory size."; :221 Error "Cannot write to address ... configured memory size."; :226 Error "Cannot write to address 0x{0:X} because it is in the locked range"
  - :663 Error erase beyond size; :681 Warning "Unsupported erase type: {0}"; :687 Error "Erase operations are disabled."; :691 Warning "Encountered unexpected command: {0}"; :701 Error "Chip erase can only be performed when there is no locked range"; :731 Noisy "Full segment to erase: {0}"; :734 Noisy "Erasing subrange {0}"
  => Per-byte logging is Noisy only (invisible by default). Nothing at Info/Debug. A 0x9F with correct CS produces no default-visible log line.
- Externally observable: `public MappedMemory UnderlyingMemory` (:156), `public byte[] SFDPSignature {get;set;}` (:158). NO transfer counters. Memory contents change only on real program/erase.

### A2. Derived from GenericSpiFlash (all SPI-only, all use A1's Transmit/OnGPIO/FinishTransmission)
- Micron_MT25Q — P/SPI/Micron_MT25Q.cs:13. Ctor (:15) `public Micron_MT25Q(MappedMemory underlyingMemory, byte extendedDeviceId = DefaultExtendedDeviceId)`; base(manufacturerId: 0x20 (:27), memoryType: 0xBB (:28), extendedDeviceId default 0x00 (:29)); size must be 64KB-256MB (:20-23). capacityCode auto from size. JEDEC = 20 BB <log2size> 10 <extId> 00. ONLY extendedDeviceId settable from .repl. Used e.g. renode/platforms/boards/sltb004a.repl:6, tests/platforms/STM32L072.robot:7.
- Macronix_MX25R — Macronix_MX25R.cs:16; ctor `public Macronix_MX25R(MappedMemory underlyingMemory)` (:18); mfr 0xC2 (:70), type 0x28 (:71); capacity auto; writeStatusCanSetWriteEnable:false, useStatusRegisterStubs:false. Not settable.
- ISSI_IS25WP — ISSI_IS25WP.cs:21; ctor `public ISSI_IS25WP(MappedMemory underlyingMemory, BlockSize blockSize = BlockSize._256KB)` (:23) base(..., ManufacturerId 0x9D (:421), MemoryType 0x70 (:422), capacityCode: (byte)Misc.Logarithm2(size), sectorSizeKB: 4 (:424)). (a private const CapacityCode=0x1A at :423 is NOT used by the ctor.) Own warnings at :116,134,152,164,170,176,185,234,244,261,300,321 (protection/registers, not per-byte). Not settable.
- Cypress_S25H — Cypress_S25H.cs:15; ctor `public Cypress_S25H(MappedMemory underlyingMemory, S25HxFamily memoryFamily = S25HxFamily.HS_T)` (:17); mfr 0x34 (:160), memoryType = (byte)memoryFamily (HL_T 0x2A, HS_T 0x2B, :168-171), remainingIdBytes 0x0F (:162), ExtendedDeviceID 0x03 (:163), DeviceConfiguration 0x90 (:164), sector 256KB; overrides GetCapacityCode (:31); size 32-128MB (:22). Only memoryFamily settable.

### A3. GigaDevice_GD25LQ — P/SPI/GigaDevice_GD25LQ.cs:17 `: ISPIPeripheral, IGPIOReceiver` (NOT derived from GenericSpiFlash)
- Part: GD25LQ (ID table suggests GD25LQ128). Ctor `public GigaDevice_GD25LQ(MappedMemory underlyingMemory)` (:19) — no ID args.
- Identity: 0x9F (:253 ReadID) returns `deviceID = { 0xC8, 0x60, 0x18 }` (:102, static readonly), read at :161-171.
- Transfer: Transmit(byte) ~:85; FinishTransmission ~:58; OnGPIO(0,true)=CS deassert (:49-54) -> FinishTransmission.
- Logs: :52 Noisy "Chip Select is deasserted."; :65 Warning "Transmission finished in the unexpected state: {0}"; :68 Noisy "Transmission finished in state: {0}"; :87 Noisy "Transmitting data 0x{0:X}, current state: {1}"; :135 Error "Command decoding failed on byte: 0x{0:X} ({1})."; :149 Error "Encountered invalid immediate command: {0}"; :153 Noisy "Decoded operation: {0}"; :168 Error "Trying to read beyond the length of the device ID table."; :189 Warning "Unhandled operation encountered while processing command bytes: {0}"; :193 Noisy "Handled command: {0}, returning 0x{1:X}"; :206 Warning "Trying to write 0x{0:X} to {1} register while operation is not enabled."; :209 Noisy "Writing value: 0x{0:X} to {1}". Counters: none public.

### A4. Puya_P25Q — P/SPI/Puya_P25Q.cs:20 `: ISPIPeripheral, IGPIOReceiver`
- Ctor `public Puya_P25Q(MappedMemory underlyingMemory)` (:22). `public GPIO WriteProtect {get;} = new GPIO()` (:164).
- Identity: NONE answered. `ReadJEDECID = 0x9F` is only declared in the enum (:483); Transmit's switch (:99-135) has no case for it -> falls to default: Warning "Unsupported command: {0}" (:130), returns 0. A JEDEC read of this model returns 00 00 00 plus a Warning per byte after the command.
- Transfer: `public byte Transmit(byte data)` :65; first byte latches command (no response), later bytes handled; `FinishTransmission()` :141-150 (currentCommand=null); `OnGPIO` :152-160: index 0 only else Error ":158 'This peripheral supports gpio input only on index 0, but {0} was called.'" (body of the index-0 branch not shown here; [inferred] it calls FinishTransmission when deasserted).
- Logs (NoisyLog/WarningLog/ErrorLog): :69 Noisy "Selecting new command"; :90 Warning "Trying to reset while Reset Enable is not active. Operation will be ignored."; :97 Noisy "Current command: {0}" (every byte after the first); :130 Warning "Unsupported command: {0}"; :134 Error "Invalid command: {0}"; :143 Noisy "Transmission finished."; :158 Error (above); :219 Error "Invalid block protect configuration."; :229,234,245,250 Warning (status/config write protected); :260 Error page program while write-disabled; :266 Noisy "Accumulating address bytes."; :271 Noisy "Calculated address for memory write operation: 0x {0:X}"; :279 Error program on protected block; :283 Noisy "Writing 0x{1:X} to address 0x{0:X}"; :304 Error; :310 Noisy; :319 Noisy; :324 Error; :341 Error; :361 Error.
- Example repl: renode/platforms/cpus/egis_et171.repl:57 `p25q16: SPI.Puya_P25Q @ spi0` (CS not shown wired).

### A5. MXIC_MX66UM1G45G — P/SPI/MXIC_MX66UM1G45G.cs:19 `: ISPIPeripheral, IDisposable`; ctor `public MXIC_MX66UM1G45G(MappedMemory underlyingMemory)` (:21), size 64KB-256MB pow2.
- ID (GetDeviceData :234-242) = C2 80 3B 00 (ManufacturerID 0xC2 :716, MemoryType 0x80 :718, MemoryDensity 0x3B :719, [3]=0). 0x9F at :731, handled :369-376. Has OPI mode (:100-115).
- Transmit :100; FinishTransmission :59 (no GPIO input in what I read; [inferred] relies on controller FinishTransmission; FinishTransmission also called from Reset :97).
- Logs: :61 Noisy "Transmission finished"; :67 Warning "Transmission finished in the unexpected state: {0}"; :115 Error OPI inverse-byte check; :174,183,194 Debug backing file; :198 Warning backing file size; :348 Error "Command decoding failed on byte: 0x{0:X} ({1})."; :362 Noisy "Handling dummy byte in ReadFast operation"; :376 Error "Trying to read beyond the length of the device ID table."; :389 Error "Memory write operations are disabled."; :405 Warning unhandled op; others :417-619 (register/erase), :454 Noisy "OPI mode {0}". No per-byte log in the data path seen.

### A6. NANDFlash — P/SPI/NANDFlash.cs:18 `: ISPIPeripheral, IProvidesRegisterCollection<ByteRegisterCollection>`
- Ctor (:20-23) `NANDFlash(MappedMemory dataMemory, MappedMemory spareMemory, uint pageSize = 2048, uint spareSize = 64, uint pagesPerBlock = 64, uint blocksPerLun = 2048, uint lunsPerChip = 1)`.
- Identity: ReadId = 0x9f (:250) -> `ReadIdBytes = { 0x0, 0xef, 0xbc }` (:241; dummy 0x00 then 0xEF 0xBC). Hard-coded.
- Transmit :58; FinishTransmission :129; no GPIO CS.
- Logs: :60 Noisy "Received byte: 0x{0:X} in state {1}"  <- every byte; :74 Noisy "Reading register {0} = {1}"; :84 Noisy "Column set to {0}"; :100 Error "Trailing bytes on SetFeature command"; :106 Noisy "Setting register {0} = {1}"; :113 Warning "Nothing left in send queue"; :121 Error "Received byte 0x{0:X} in an unexpected state: {1}. Ignoring it..."; :125 Noisy "Returning byte: 0x{0:X}"  <- every byte; :131 Noisy "Transmission finished"; :137 Error "Incorrect byte count for PageRead {0}"; :142 Noisy "Loading page {0} from cache"; :213 Error "Unsupported command 0x{0:X}".

### A7. NPCX_Flash — P/SPI/NPCX_Flash.cs `: ISPIPeripheral ...` ctor `NPCX_Flash(MappedMemory memory)` (:22). 0x9F JEDECID only in enum (:314); not handled in Transmit (:40-172) -> "Unsupported command" Warning (:163) / returns 0. OnGPIO (:186) is WRITE-PROTECT (WriteProtect.Set(value)), NOT chip select; error log :190 "This peripheral supports gpio input only on index 0, but {0} was called." Other logs :82,90,112,119,153 Error; :142 Warning "Trying to write status register while SRP is enabled and WP signal is low; ignoring"; :163 Warning "Unsupported command: {0}"; :167 Error "Invalid command: {0}". FinishTransmission :174 (clears command).

### A8. DummySPIFlash (P/MTD/DummySPIFlash.cs:14) implements ISPIFlash (MTD/ISPIFlash.cs), NOT ISPIPeripheral; logs ":46 Warning 'Reading ID'". Not attachable to an SPI controller as ISPIPeripheral. NORFlash/ dir holds only DecodedOperation.cs (shared enum/struct), no model.

## B. Sensors (ISPIPeripheral)

### B1. GenericSPISensor — P/Sensors/GenericSPISensor.cs:13 `: ISPIPeripheral`
- Part: none (scalar-byte feeder). No identity. `public GenericSPISensor()`.
- `public byte Transmit(byte b)` :27: dequeues next sample from a SensorSamplesFifo<ScalarSample>, returns (byte)sample, ignores input. `FinishTransmission()` :21 does NOTHING. No CS handling at all.
- LOG (only one, in every transfer): **:32 `this.Log(LogLevel.Info, "Received byte 0x{0:X}, returning 0x{1:X}", b, result);`  Info = visible by default, once per byte.** This is the only Info-per-byte logger in the census.
- Observable: FeedSample(byte), FeedSamplesFromFile(path) :36-44. No counters.
- Test usage: tests/platforms/Renesas_RA8M1.robot:120 `sensor: Sensors.GenericSPISensor @ sci2`.

### B2. ICM20948 — P/Sensors/ICM20948/ICM20948.cs:21 `: II2CPeripheral, ISPIPeripheral, IGPIOReceiver, IProvidesRegisterCollection<ByteRegisterCollection>, IPeripheralContainer<II2CPeripheral,...>`; ctor `ICM20948(IMachine machine)` :23. SPI AND I2C.
- Identity: WHO_AM_I = bank0 reg 0x00, value 0xEA (ICM20948_Bank0Registers.cs:15 `Define(..., 0xEA)`, enum :274). Bank select via REG_BANK_SEL (bank0..3).
- SPI protocol: first byte bit7=1 read, bits[6:0]=register (ICM20948.cs:Transmit ~:145-160), then subsequent bytes read/write with auto-increment.
- Transfer: `public byte Transmit(byte data)` (~:141); `void ISPIPeripheral.FinishTransmission()` :160 (explicit interface; state=Idle); `OnGPIO(int number, bool value)` ~:166: pin 0 only (else Warning "This model supports only CS on pin 0, but got signal on pin {0}"), chipSelected = !value (active-low), on deassert calls FinishTransmission. Does not refuse Transmit when CS unasserted.
- Logs in SPI path: Noisy "Preparing for read at bank #{0} from 0x{1:X} ({2})"; Noisy "Preparing for write at bank #{0} to 0x{1:X} ({2})"; Noisy "Read at bank #{0} from 0x{1:X} ({2}), returned 0x{3:X}"; **Debug** "Write at bank #{0} to 0x{1:X} ({2}): value 0x{3:X}"; Error "Attempted transmission in an unexpected state: {0}"; Noisy "Received byte 0x{0:X}, returning 0x{1:X}" (every byte); Noisy "Finishing transmission, going idle"; plus UpdateInterrupts Debug "IRQ {0}" / Noisy "Interrupts: ...". (line numbers ~ in the 140-260 range of ICM20948.cs; exact line numbers beyond those quoted for 'Read' (:100,:103) not individually pinned.)
- Observable: `ByteRegisterCollection RegistersCollection`, `GPIO IRQ`. Data feeders via RESD threads.
- Repl: only I2C use found (platforms/boards/renesas-ck_ra6m5.repl:29 `icm: Sensors.ICM20948 @ iic0 0x68`).

### B3. LSM6DSO_IMU — P/Sensors/LSM6DSO_IMU.cs:24 `: BasicBytePeripheral, ISPIPeripheral, II2CPeripheral, ...`. SPI and I2C.
- Identity: WHO_AM_I reg 0x0F (enum :1268) returns 0x6C (:224, `valueProviderCallback: _ => 0x6C`).
- Transfer: `Transmit(byte)` :104; `FinishTransmission()` :80 (commandInProgress=None); NO GPIO CS (relies on controller FinishTransmission). First byte: bits0-6 address, bit7 read.
- Logs: :82 Noisy "Finishing transmission, going to the Idle state"; :116 Noisy "Received 0x{0:X2}; setting commandInProgress to {1} and address to 0x{2:X2}"; :796 Warning "Tried to read in invalid state: {0}"; :801 Warning "Unhandled read from 0x{0:X2}: unsupported {1} bank"; :806 Noisy "Read from 0x{0:X2} ({1}): returning 0x{2:X2}"; :819 Warning "Tried to write in invalid state: {0}"; :824 Warning unsupported bank write; :829 Noisy "Write to 0x{0:X2} ({1}): 0x{2:X2}"; :526 Debug invalid scale; :748 Debug "New INT1 state: {0}"; FIFO logs :922-1023 (Debug/Noisy/Warning). Per-byte: Noisy only.

### B4. ADXL372 — P/Sensors/ADXL372.cs:20 `: ISPIPeripheral, II2CPeripheral, IProvidesRegisterCollection<ByteRegisterCollection>, IGPIOReceiver, ISensor`. SPI and I2C.
- Identity: DEVID_AD reg 0x00 = 0xAD (:159, enum :242), DEVID_PRODUCT (PartID) reg 0x02 = 0xFA (:162, enum :243). (Real part: ADXL372.)
- Transfer: `Transmit(byte b)` :94; `FinishTransmission()` :130; `OnGPIO` :28-42: pin 0 else Warning :32 "This model supports only CS on pin 0, but got signal on pin {0}"; value=negated CS: if chipSelected && value -> FinishTransmission; chipSelected = !value.
- **Requires CS asserted via GPIO: `if(!chipSelected){ Log(Warning, "Received transmission, but CS pin is not selected"); return 0; }` (:96-100).** i.e. an ADXL372 with no CS GPIO wired answers 0 to everything. reset sets chipSelected=false (:141).
- Logs: :78 Noisy "Unexpected reading in Idle state"; :98 Warning (above); :110 Noisy "Reading register {0} (0x{0:X})"; :116 Noisy "Writing 0x{0:X} to register {1} (0x{1:X})"; :122 Error "Received byte in an unexpected state!"; :126 Noisy "Transmitting - received 0x{0:X}, sending 0x{1:X} back"; :132 Noisy "Finishing transmission, going to the Idle state".
- Observable: AccelerationX/Y/Z props (set from monitor), RegistersCollection. Repl w/ CS: renode/tests/platforms/NRF52840.robot:32-35 (see section F).

### B5. MAX86171 — P/Sensors/MAX86171.cs:21 `: ISPIPeripheral, IProvidesRegisterCollection<ByteRegisterCollection>` (optical AFE). Identity: PartID reg 0xFF (enum :1373) = 0x2C (:664-667). Protocol: byte0=register, byte1=transaction type (States.Write/Read), then data (Transmit :156-186). FinishTransmission :120 (chosenRegister=null,state=null); no GPIO CS. Logs in transfer path: none in Transmit itself; register-level Warnings :485 "Unhandled read from {0}; returning 0x{1:X02}", :490 "Unhandled write to {0}; written 0x{1:X02}"; Info :106 "Started feeding samples from RESD file at {0}Hz", :790 "Starting the default sample feeding at {0}Hz"; Warnings :399,:469,:933,:940,:1077,:1095.

### B6. ArduCAMMini2MPPlus — P/Sensors/ArduCAMMini2MPPlus.cs:23 `: ISPIPeripheral, II2CPeripheral, IGPIOReceiver, ...`. SPI+I2C camera. Chip-version register ArduChipVersion = 0x40 (enum :295); no value literal seen in grep [inferred: unset/default]. Transmit :55 ; ISPIPeripheral.FinishTransmission :93; OnGPIO :99-112 (pin 0 else Warning :103 "This model supports only CS on pin 0, but got signal on pin {0}"; chipSelected=!value, finish on deassert). Logs per byte: :66 Noisy "Decoded register {0} (0x{0:X}) and isWrite bit as {1}"; :75 Noisy "Reading register {0} (0x{0:X})"; :80 Noisy "Writing 0x{0:X} to register {1} (0x{1:X})"; :85 Error "Received byte in an unexpected state: {0}"; :89 Noisy "Received byte 0x{0:X}, returning 0x{1:X}"; :95 Noisy "Finishing transmission, going idle".

### B7. MAX6682MUA — P/Sensors/MAX6682MUA.cs:13 `: ISPIPeripheral`: 2-byte temperature read (first Transmit returns (uint)Temperature*8 >>8, second returns low byte :27-39); FinishTransmission :25 -> Reset (isFirstByte). No ID, NO Log calls at all. `decimal Temperature` property settable (range check :52-57).
### B8. TI_LM74 — P/Sensors/TI_LM74.cs:14 `: ISPIPeripheral, ITemperatureSensor`: same 2-byte pattern, value = (int)(T*10000/625)<<3 | 0x7 (:37-38); no ID register; no Log calls. `Temperature` settable. FinishTransmission :(~21) -> Reset.

## C. Other ISPIPeripheral implementers (non-sensor/flash)
(Transmit/FinishTransmission signatures are the interface's in all cases. Not fully log-enumerated unless shown.)
- SDCard P/SD/SDCard.cs:31 (SPI-mode SD; Transmit :64, FinishTransmission :52; logs :56,:68 Error if SPI mode disabled, :60 Noisy, :72 Noisy "SPI: Received byte 0x{0:X} in state {1}", :80,:119,:136,:163,:194 Noisy, :89 Warning, :141 Warning).
- ENC28J60 P/Network/ENC28J60.cs:21 (Transmit :237, Finish :270); KS8851 P/Network/KS8851.cs:20 (Transmit :50, Finish :77; chip id 0x8870 at :120).
- CC1200 P/Wireless/CC1200.cs:24 (OnGPIO :40, Finish :145, Transmit :151; PartNumber ext reg 0x8F :716/:1243); CC2520 :21 (OnGPIO :66, Transmit :100, Finish :120); AT86RF233 :17 (OnGPIO :55, Transmit :70, Finish :107).
- AD4011_ADC P/Analog/AD4011_ADC.cs:24 (Debug logs :141,:179; Warning :113); AD56x1_DAC P/Analog/AD56x1_DAC.cs:17 (Warning :50,:62; Debug :75,:82,:87).
- STP08ShiftRegister P/SPI/STP08ShiftRegister.cs:18 (Debug :44 "Recived data 0x{0}", :47 "Setting #{0} to {1}"); MAX3421E P/SPI/MAX3421E.cs:23 (USB host; Noisy :71,:102,:113,:118,:127,:162).
- SPIMultiplexer P/SPI/SPIMultiplexer.cs:18 `: SimpleContainer<ISPIPeripheral>, IGPIOReceiver, ISPIPeripheral`; ctor `SPIMultiplexer(IMachine machine, bool suppressExplicitFinishTransmission = true)` (:20). Children registered by number; OnGPIO(number,value) (:30) stores inputState and updates chipSelects (Noisy :32 "GPIO #{0} set to {1}"); SetActiveLow/High(number). Transmit (:50-82) forwards to the single selected child; Warnings :54 "Tried to transmit byte 0x{0:X}, but no device is currently selected - ignoring transfer and returning a dummy byte" (returns 0xFF), :60 multiple selected, :67 not connected. FinishTransmission (:84-103) is a no-op when suppressExplicitFinishTransmission (default true: CS-edge drives children's finish instead); Warnings when none/multiple selected.
- Mocks/test: DummySPISlave P/Mocks/DummySPISlave.cs:14 (Debug :39 "Data received: 0x{0:X} (idx: {1})", :43 "No data left in buffer, returning 0."); SPILoopback INFRA/Main/Peripherals/SPI/SPILoopback.cs:10 (returns data, no logs; used tests/platforms/nucleo_wba52cg.robot:15); ExternalControlSPIPeripheral (Noisy :26,:37; Error :23,:34); MockSpiHost P/Mocks/SpiHost.cs (host, not a peripheral).
- Dual-role / controllers that also implement ISPIPeripheral: OpenTitan_SpiDevice (SPI target device, :21), UARTToSpiConverter (container), SCI/USART containers (RenesasRA6M5_SCI, RenesasRA8M1_SCI, EFR32 USARTs) are CONTROLLERS holding an ISPIPeripheral.

## D. Controllers (STM32) — chip select / FinishTransmission
### D1. STM32SPI — P/SPI/STM32SPI.cs:20 `: NullRegistrationPointPeripheralContainer<ISPIPeripheral>` (series F0/F1/F4/F7/G0/L0/L1/L5; ctor `STM32SPI(IMachine machine, STM32Series series, int bufferCapacity = DefaultBufferCapacity)` :22). Used by stm32f0/f4/f412/f746/f777/l071/l151/l552/g0 .repl.
- **FinishTransmission() is NEVER called** (no occurrence in the file). NSS is not modelled: SSI (:208-? `WithFlag(8,"SSI")`) and SSM (:209 `WithFlag(9,"SSM")`) are stored flags only; SSOE tagged only (:Control2). SPE (Control1 bit6, :218-232) only unsets IRQ / sets DMASend. => CS MUST be done with a GPIO output wired to the peripheral's OnGPIO (peripheral must implement IGPIOReceiver) — e.g. `gpioPortB: 12 -> spiFlash@0`. A peripheral that relies purely on FinishTransmission (LSM6DSO, MAX86171, MAX6682MUA, TI_LM74, GenericSPISensor, NANDFlash, MXIC) will never be reset between frames on STM32SPI.
- One peripheral only (NullRegistrationPoint container; `RegisteredPeripheral`, :144). Each DR write -> `peripheral.Transmit((byte)value)` :151 (byte mode only; 'Data size > 8 bits not supported').
- Log calls: :147 Warning "SPI transmission while no SPI peripheral is connected." (pushes 0x00 to RX); :154 Debug "Receiving response while RXFIFO is full, dropping it"; :168 `NoisyLog` "Transmitted 0x{0:X}, received 0x{1:X}."  (per byte); :211 Error "Setting slave mode which is not supported."; :225 Error "Enabled SPI in slave mode, which is not supported."; :265 Warning "Data size > 8 bits not supported"; :325 Warning "Trying to enable not supported I2S mode."
### D2. STM32H7_SPI — P/SPI/STM32H7_SPI.cs:19 `: NullRegistrationPointPeripheralContainer<ISPIPeripheral>, IKnownSize, IDoubleWordPeripheral, IWordPeripheral, IBytePeripheral`; ctor `STM32H7_SPI(IMachine machine)` :21. STM32WBA_SPI (P/SPI/STM32WBA_SPI.cs) just derives with IsWba=true. Used in stm32h7.repl (`spi4: SPI.STM32H7_SPI @ { sysbus 0x40013400; rcc RegistrationPeripheralAlias.SPI4 }` :193-197) and stm32wba52.repl.
- Transfer: TryTransmitData() :340-383 — requires SPE && CSTART && TX FIFO non-empty; for each FIFO word, per byte `RegisteredPeripheral?.Transmit(bytes[i]) ?? 0` :361 (silently 0 when nothing attached).
- FinishTransmission is called in exactly ONE place: EndTransfer() :391-395 `RegisteredPeripheral?.FinishTransmission(); endOfTransfer.Value = true; startTransmission.Value = false;`. EndTransfer is called (a) from TryTransmitData when TSIZE != 0 and transmittedPackets == TSIZE (:383-386, i.e. at EOT), and (b) from CSUSP write when !EOT (:110-119 write-callback). 
- NOT calling FinishTransmission: SPE clear (:92-105) only ResetTransmissionState + clears FIFOs + TSIZE=0; EOTC write (:251-258) ResetTransmissionState only; TSIZE==0 (endless transaction) never finishes (comment :379-382: "CSTART flag is not reset automatically and we don't call FinishTransmission(). To reset CSTART and de-assert CS pin, CSUSP ... should be set."). CSTART (CR1 bit 9, :108-115) write 1 clears EOT and calls TryTransmitData. SSI (:129) tagged flag only, SSM (:198), SSIOP (:200), SSOE (:201) tagged flags only; no NSS pin/GPIO output is modelled. CS GPIO therefore must come from a GPIO port, or relies on FinishTransmission at EOT.
- Log calls: :137 Warning "Attempted to set IOLOCK while peripheral is enabled"; :192 Error "Attempted to set peripheral into SPI slave mode. Only master mode is supported"; :288 Error "Receive data FIFO is empty. Returning 0"; :302 Error "Attempted to enable I2S. This mode is not supported"; :333 Error "Attempted to write 0x{0:X} to {0} register while peripheral is enabled" (note: format uses {0} twice; value/reg swapped [as read]); :404 Debug "Setting IRQ to {0}" (every UpdateInterrupts). No per-byte log.

## E. Flash IDs: can the .repl set them?
- GenericSpiFlash: yes — verbatim constructor in A1; repl example renode/platforms/boards/zynqmp-zcu104.repl:16-22:
```
// mt25qu512a
spiFlash: SPI.GenericSpiFlash @ qspi
    underlyingMemory: quadSpiFlash
    manufacturerId: 0x20
    memoryType: 0xbb
    extendedDeviceId: 0x44
    capacityCode: 0x20
```
  and inline form (tests/platforms/frdm-imx8mp_linux.robot:27): `spiNor: SPI.GenericSpiFlash @ ecspi2 1 { underlyingMemory: spiNorBackingMemory; manufacturerId: 0x20; memoryType: 0xbb; extendedDeviceId: 0x44; capacityCode: 0x20 }`.
  => a W25Q64JV-style ID would be `manufacturerId: 0xEF; memoryType: 0x40; capacityCode: 0x17` (JEDEC EF 40 17 [inferred from outside knowledge]); note default remainingIdBytes (0x10), extendedDeviceId (0x40) are also returned after the first 3 bytes unless overridden (a real W25Q returns only 3 ID bytes; bytes 4+ of real part are 0xFF/undefined [inferred]).
- Micron_MT25Q: only extendedDeviceId. Macronix_MX25R / ISSI_IS25WP / Cypress_S25H (memoryFamily only) / GigaDevice_GD25LQ (none) / Puya_P25Q (none; no ID answered) / MXIC_MX66UM1G45G (none) / NANDFlash (none): not settable.

## F. Example .repl attaching to an STM32 SPI controller (with CS)
1. tests/platforms/STM32L072.robot:2-12 (STM32SPI `spi2` = platforms/cpus/stm32l071.repl:118-121 `spi2: SPI.STM32SPI @ sysbus 0x40003800 / series: STM32Series.L0 / bufferCapacity: 1 / IRQ ->nvic@26`). CS = GPIO output into OnGPIO(0):
```
${SPI_FLASH}=    SEPARATOR=
...  """                                         ${\n}
...  spiFlashMemory: Memory.MappedMemory         ${\n}
...  ${SPACE*4}size: 0x100000                    ${\n}
...                                              ${\n}
...  spiFlash: SPI.Micron_MT25Q @ spi2           ${\n}
...  ${SPACE*4}underlyingMemory: spiFlashMemory  ${\n}
...                                              ${\n}
...  gpioPortB:                                  ${\n}
...  ${SPACE*4}12 -> spiFlash@0                  ${\n}
...  """
```
   (STM32 GPIO port pin 12 -> spiFlash GPIO input 0; level high = deasserted -> FinishTransmission.)
2. Same pattern, non-STM32 (nRF52840): tests/platforms/NRF52840.robot:29-36:
```
...  using "platforms/cpus/nrf52840.repl"    ${\n}
...  adxl372: Sensors.ADXL372 @ spi2         ${\n}
...  gpio0:                                  ${\n}
...  ${SPACE*4}22 -> adxl372@0 // CS         ${\n}
```
3. Controller-driven finish, no CS wire: tests/platforms/nucleo_wba52cg.robot:15 `loopback: SPI.SPILoopback @ spi3` (STM32WBA_SPI) ; renode/platforms/boards/ramn/BODY.repl:1 `ledController: SPI.STP08ShiftRegister@ spi2`.
4. No in-tree .repl/.robot attaches any sensor (ICM/LSM6DSO/etc.) over SPI to an STM32 SPI controller; none attaches a flash to STM32H7_SPI. H7 QSPI flashes: tests/platforms/nucleo_h753zi.robot:45 `externalQspiFlash: SPI.Macronix_MX25R @ qspi {underlyingMemory: qspiMappedFlashMemory}` and :67 `SPI.ISSI_IS25WP @ qspi { underlyingMemory: qspiMappedFlashMemory; }` (STM32H7_QuadSPI, not SPI controller).

## G. Caveats / not verified
- Exact line numbers for a few ICM20948 SPI log lines were not individually pinned (quoted as "~"); every format string is verbatim.
- OnGPIO body of Puya_P25Q (:152-162) read only up to the error branch.
- ArduCAM ArduChipVersion default value not read.
- Winbond/BMI/ICM-426xx/MPU/BMP3xx/DPS/MS5611 absence is from case-insensitive rg over infra and renode trees (excluding png/elf).
