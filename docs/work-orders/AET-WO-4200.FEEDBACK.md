# AET-WO-4200 — FEEDBACK (spike)

**Branch:** `wo/aet-4200-spi-devices` · **Base:** `a5be795` (`origin/main`) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Cadence:** spike (§1), then **STOP** for the architect's ruling. No build code before it.

**The work order** is read from `origin/claude/argon-repo-docs-synthesis-rjrb0d:docs/work-orders/AET-WO-4200.md` (it lands through docs PR #116). It is not committed here.

**Where everything runs.** The pinned tools image (`container/IMAGE`, `ghcr.io/clearforge-llc/aethelgard-tools@sha256:c69ff9bc…`) under rootless podman, through `spike/4200/img.sh`:
- `--memory X --memory-swap X --pids-limit -1`;
- after start, `0` is written into the container's own `memory.swap.max`, and `memory.max`, `memory.swap.max` and `pids.max` are read back into `spike/4200/out/caps.log` before the command inside may start;
- `TMPDIR` is unset inside the container, as in CI.

Renode runs in the CLI's composition (`AET-WO-2204` D4):
- `--config` with every key `renode::config` states, synchronous logging on;
- no console `logLevel`, and `logLevel -1` on each device source;
- `BEGIN`/`END` markers.

Nothing runs inside the CLI's sandbox (the build's tier-2 tests will).

Entry schema for findings: `finding · where · severity · type · recommendation · decision-needed`.

## Predictions — written before any run

This section is the branch's first commit, made before the first container started. Nothing in it is edited after a run. Each prediction has an id; the table at the end sets each result beside it.

**What was staged before it.** The probe scripts in `spike/4200/` were written before this section, and none had run:
- `img.sh`: 2204's runner;
- `fw.py`: the probe firmwares;
- `plat.py` and `platforms/`: the platform variants, plain text written from the committed `.repl`;
- `drive.py`: one boot in the CLI's composition;
- `s2.py`: the cases.

`s1-census.md` is a read-only census of Renode's SPI models by a subagent.

**Provenance labels.**
- **[read]**: I read it at source.
- **[read-sub]**: only the census subagent read it; I have not checked it myself.
- **[inferred]**: reasoned, not read.
- **[recall]**: from memory of a document not held on Furnace.

### What the predictions rest on

- **Renode source:** `renode` at `ab721d88` (tag `v1.17.0`), `src/Infrastructure` at `066a7f13` (`~/.cache/aet-2301/src/{renode,infra}`).
  - The image's binary is `1.17.0+20260907gitf1dd1b4af`, two commits earlier; `AET-WO-2204` F-notes say only `MappedMemory.cs` and one unit test differ.
  - Every file cited below is outside that difference.
- **`STM32H7_SPI.cs`** (the model of `spi4`, and of the `spi1` the twin adds) **[read]**:
  - A `NullRegistrationPointPeripheralContainer<ISPIPeripheral>`: **one** attached device, no chip-select addressing (L19).
  - **The transfer** (`TryTransmitData`, L341–389):
    - it runs only with `SPE`, `CSTART` and a non-empty TX FIFO;
    - each byte goes to `RegisteredPeripheral?.Transmit(b) ?? 0`, so with nothing attached the byte read is `0`, silently (L361);
    - each received word is queued, so `RXP` rises either way.
  - **`FinishTransmission` is called only from `EndTransfer`** (L391–396):
    - at `EOT`, when `TSIZE` packets have gone (L383–386);
    - or on a `CSUSP` write before `EOT` (L116–127).
    - Not on `SPE` clear, not on `EOTC`.
  - **What it logs:**
    - it has **no per-byte log**;
    - `UpdateInterrupts` logs `Debug` `Setting IRQ to {0}` (L404);
    - an empty `RXDR` read logs `Error` `Receive data FIFO is empty. Returning 0` (L288);
    - a `CFG1`/`CFG2` write while `SPE` is set logs `Error` and is refused (L333).
  - **Tags, not fields:** `CPHA`, `CPOL`, `SSM`, `SSI`, `SSOE`, `SSIOP` and `MBR`. `MASTER` always reads 1 (L188).
- **Tags** (`PeripheralRegister.cs` L682–760) **[read]**:
  - a write that sets a tag's bits logs `Warning` `Unhandled write to offset 0x{off:X}. Unhandled bits: [{bits}] when writing value 0x{v:X}. Tags: {name} (0x{val:X}), …`;
  - the write loop updates only defined fields, so a tag's bits are never stored and read back 0.
- **`GenericSpiFlash.cs`** **[read]**:
  - **Construction:** `GenericSpiFlash(MappedMemory underlyingMemory, byte manufacturerId, byte memoryType, byte? capacityCode = null, …)`. The size must be a power of two. The `0x9F` reply is `deviceData` = `[manufacturerId, memoryType, capacityCode, remainingIdBytes 0x10, extendedDeviceId 0x40, deviceConfiguration 0x00, …]`. The default capacity code is the size's top bit index (`0x17` for 8 MiB).
  - **`Transmit` never checks chip select.** It logs `Noisy` `Transmitting data 0x{0:X}, current state: {1}` for every byte (L129).
  - **The first byte** is `RecognizeOperation`:
    - `0x9F` gives `Noisy` `Decoded operation: Operation: ReadID, write enabled False` (L298; `DecodedOperation.ToString`);
    - an unknown byte gives **`Error`** `Command decoding failed on byte: 0x{0:X} ({1}).` (L295).
  - **Each later byte** is `HandleCommand`, logging `Noisy` `Handled command: Operation: ReadID, returning 0x{0:X}` (L635).
  - **`OnGPIO(0, true)`** (chip select deasserted) logs `Noisy` `Chip Select is deasserted.` and calls `FinishTransmission`. Asserting it does nothing.
  - **`FinishTransmission` in the idle state** (`RecognizeOperation`, the enum's 0 and the default) logs **`Warning`** `Transmission finished in the unexpected state: RecognizeOperation` (L90–95). `Reset()` calls it too.
- **`SPIMultiplexer.cs`** **[read]**:
  - It is a `SimpleContainer<ISPIPeripheral>`, `IGPIOReceiver` and `ISPIPeripheral`: devices at numbered addresses, each selected by its GPIO input; `SetActiveLow n` inverts input `n`.
  - **`Transmit`** with:
    - **no input active:** `Warning` `Tried to transmit byte 0x{0:X}, but no device is currently selected - ignoring transfer and returning a dummy byte`, and returns `0xFF`;
    - **more than one active:** a `multiple devices are currently selected` warning, and `0xFF`;
    - **an active input with no device at its address:** `Warning` `Tried to transmit byte 0x{0:X} to device 0x{1:X}, but it's not connected - ignoring transfer and returning a dummy byte`, and `0xFF`.
  - **`FinishTransmission`** from the controller is ignored by default (`suppressExplicitFinishTransmission = true`).
  - **An input change** logs `Noisy` `GPIO #{0} set to {1}` and calls `FinishTransmission` on every inactive input's device.
- **`ICM20948.cs` `Transmit`** **[read]**:
  - **The first byte:** bit 7 set means a read; it logs `Noisy` `Preparing for read at bank #0 from 0x0 (WhoAmI)`.
  - **Each later byte** logs `Noisy` `Read at bank #0 from 0x0 (WhoAmI), returned 0xEA` (the identity: `WhoAmI` bank 0 register 0, `0xEA`, `ICM20948_Bank0Registers.cs` L15).
  - **Every byte** also logs `Noisy` `Received byte 0x{0:X}, returning 0x{1:X}`.
- **`STM32_GPIOPort.cs`** **[read]**: a `BSRR` write sets each pin's `Connections[n]` whatever `MODER` says. `GPIO.Set` signals only on a change (`GPIO.cs` L47).
- **The reference board** (`boards/sentinel-01`) **[read]**:
  - **`spi1`:** `Spi1::init` clears `SPE`, sets `CFG2.MASTER`, then sets `SPE`, and **makes no transfer**. The firmware binds `_spi1`.
  - **`PA0`:** `PA0::into_output` writes `BSRR` = 1 (`FLASH_CS` high) before `MODER`.
  - **The SPI pins** `PA5`/`PA6`/`PA7` are set to `OSPEEDR` = `10` (high speed).
  - **The compensation cell:** nothing in the board crate, the firmware or the generator writes `SYSCFG_CCCSR` (grep).
- **The judge** (`renode.rs`) **[read]**: a run log line with `[ERROR]` is `renode_error_log`; warnings are not findings except `locked_peripheral_access`.
- **DS12117 Rev 10** (`~/.cache/aet-2301/ds/txt/ds10.txt`) **[read]**:
  - **Silicon revision:** the reference is rev V silicon. The tables cited are §7, "Electrical characteristics (rev V)"; rev Y's are §6.
  - **Table 149, output timing (HSLV OFF)**, at 2.7–3.6 V:
    - speed `00`: Fmax 12/12/16 MHz at 50/30/10 pF;
    - speed `10`: 85/110/166 MHz, tr/tf 3.8/2.8/1.8 ns, with note 5: "Compensation system enabled";
    - Fmax is defined by (tr+tf) ≤ 2/3 T, 45–55 % duty.
  - **Table 194, SPI dynamic:** master `fSCK` at most 100 MHz for SPI1–3 at 2.7–3.6 V (80 MHz from 1.62 V).
- **W25Q64JV:** JEDEC ID `EF 40 17` (the work order). Its maximum clock (133 MHz) and modes 0 and 3 are **[recall]**; the Winbond datasheet is not on Furnace.

### S1. What Renode 1.17.0 models on SPI

- **P1.1 [read] SPI controllers relevant to the H753:**
  - **`STM32H7_SPI`:** as above. One device, no chip select of its own, no log of a transfer.
  - **`STM32SPI`** (older STM32 families) **[read-sub]**: never calls `FinishTransmission`, and logs each byte at `Noisy`.
- **P1.2 Device models that attach to an SPI controller** **[read-sub]** except where marked:

  | Kind | Models |
  |---|---|
  | NOR flash | `GenericSpiFlash` **[read]** and its subclasses `Micron_MT25Q`, `Macronix_MX25R`, `ISSI_IS25WP`, `Cypress_S25H`; `GigaDevice_GD25LQ`; `Puya_P25Q` (no `0x9F`); `MXIC_MX66UM1G45G` |
  | NAND flash | `NANDFlash` |
  | IMUs | `ICM20948` **[read]**; `LSM6DSO_IMU` (`WHO_AM_I` `0x0F` = `0x6C`) |
  | Accelerometer | `ADXL372` (refuses a transfer without its own chip select) |
  | Other sensors and front ends | `MAX86171`, `MAX6682MUA`, `TI_LM74`, `ArduCAMMini2MPPlus` |
  | Generic and plumbing | `GenericSPISensor` (logs every byte at `Info`); the `SPIMultiplexer` **[read]** |

  Every flash and IMU listed logs its per-byte transaction at `Noisy` only.
- **P1.3 [read] The flight-controller parts, by name: none is modelled.** A case-insensitive search of both trees for ICM-42688-P, ICM-20602, MPU-6000 (and 6050, 9250), BMI088, BMP388/BMP3xx, DPS310, MS5611 and W25Q/Winbond finds nothing.
  - **Stand-ins:**
    - **the W25Q64JV:** `GenericSpiFlash` with `manufacturerId 0xEF`, `memoryType 0x40`, `capacityCode 0x17` (exact `EF 40 17`);
    - **an IMU on SPI:** `ICM20948` (`0xEA`) or `LSM6DSO` (`0x6C`);
    - **a barometer on SPI:** none. Only I2C barometers exist (`BMP180`, `LPS25HB`, `ICP_101xx`) **[read-sub]**.
  - §7.2 does not hold: a stock model stands in for the flash.

### S2. A transfer through `spi1`

Five platforms, each the committed reference description plus what is behind `spi1`:
- `none` (nothing);
- `direct` (`u4: SPI.GenericSpiFlash @ spi1`);
- `direct-cs` (`direct` plus `gpioPortA: 0 -> u4@0`);
- `mux` (`spi1mux: SPI.SPIMultiplexer @ spi1`, inputs 0 and 1 active-low, `PA0 -> 0`, `PA1 -> 1`, `u4` at 0, `imu: Sensors.ICM20948` at 1);
- `mux-absent` (`mux` without `u4`).

Every probe firmware is the reference plus a second record (record 1) after the I2C read.

The `jedec` probe:
- sets `CR1` = 0, `CFG1.DSIZE` = 7, `CFG2` = `MASTER` and `CR2.TSIZE` = 4;
- drives `PA0` low, sets `SPE`, then `CSTART`;
- writes `9F 00 00 00` to `TXDR`, reading `RXDR` after each byte;
- clears `EOT`, clears `SPE` and drives `PA0` high;
- records the three bytes after the command.

- **P2.1 [inferred, 70 %]** All five platforms load: a CPU in the listing and no `[ERROR]` line. `mux`'s `init: SetActiveLow 0/1` is accepted.
- **P2.2 [read] `direct-jedec`:** record 1 = `0x22020000`, `0x00EF4017`. Among the run's log lines, exactly these eight from the flash, in order:
  ```
  [NOISY] spi1.u4: Transmitting data 0x9F, current state: RecognizeOperation
  [NOISY] spi1.u4: Decoded operation: Operation: ReadID, write enabled False
  [NOISY] spi1.u4: Transmitting data 0x0, current state: HandleCommand
  [NOISY] spi1.u4: Handled command: Operation: ReadID, returning 0xEF
  [NOISY] spi1.u4: Transmitting data 0x0, current state: HandleCommand
  [NOISY] spi1.u4: Handled command: Operation: ReadID, returning 0x40
  [NOISY] spi1.u4: Transmitting data 0x0, current state: HandleCommand
  [NOISY] spi1.u4: Handled command: Operation: ReadID, returning 0x17
  ```
  - The device path is `spi1.u4` (80 %, by 2204's `i2c1.u2`).
  - At `EOT` the controller calls `FinishTransmission` in `HandleCommand`: no warning.
  - At most one `Transmission finished in the unexpected state: RecognizeOperation` warning precedes the run, from `Reset()` at load (50 %).
  - The I2C read and record 0 are unchanged (`0x22020000`, `0x60`).
- **P2.3 [read] No `SPI1` register witnesses the transfer.** `jedec`'s final reads match `forge-regs`'s exactly:

  | Register | Value |
  |---|---|
  | `CR1` | `0x00000000` |
  | `CR2` | `0x00000000` (`SPE` clear zeroes `TSIZE`) |
  | `CFG1` | `0x00000007` |
  | `CFG2` | `0x00400000` |
  | `SR` | `0x00001002` (`TXP`, and `TXC`, which reads `TX FIFO empty` when `TSIZE` = 0) |
  | `GPIOA.ODR` | bit 0 set |

  This is the SPI analogue of `AET-WO-2204` §7.1.
- **P2.4 [read] `direct-cs-jedec`:** the same read and the same eight lines.
  - Each rising edge of `PA0` adds two lines: `[NOISY] spi1.u4: Chip Select is deasserted.`, then `[WARNING] spi1.u4: Transmission finished in the unexpected state: RecognizeOperation`. One edge is `into_output`'s first set-high, and one is after the transfer, when `EOT` has already finished it.
  - So with the wire, every good read logs a device warning.
- **P2.5 [read] `none-jedec`:** record 1 = `0x22020000`, `0x00000000`. No line from any `spi1` source, and no `[ERROR]`.
- **P2.6 [read; path 60 %] `mux-jedec`, 3 runs:**
  - record 1 = `0x22020000`, `0x00EF4017`, and the same eight lines, with the device path `spi1.spi1mux.u4`;
  - **the mux's lines:** `GPIO #0 set to False`, then `GPIO #0 set to True`;
  - **no finish warning after the transfer:** the controller's finish at `EOT` is suppressed, and the deassert finishes the flash while it is still in `HandleCommand`;
  - **one finish warning at `into_output`:** `PA0`'s first high finishes the idle flash;
  - all three runs give byte-identical reads.
- **P2.7 [read] `mux-two`** (the `jedec` read on `PA0`, then `WHO_AM_I` on `PA1`): record 1 = `EF4017`, record 2 = `0x22020000`, `0x000000EA`. The IMU's lines:
  ```
  [NOISY] spi1.spi1mux.imu: Preparing for read at bank #0 from 0x0 (WhoAmI)
  [NOISY] spi1.spi1mux.imu: Received byte 0x80, returning 0x0
  [NOISY] spi1.spi1mux.imu: Read at bank #0 from 0x0 (WhoAmI), returned 0xEA
  [NOISY] spi1.spi1mux.imu: Received byte 0x0, returning 0xEA
  ```
  Two devices share one controller only through the mux.

### S4. The SPI witness, and what a store can produce

- **P4.1 [read] `mux-forge-claim`** (no SPI access; record 1 claims `EF4017`): no `Transmitting`, `Decoded` or `Handled` line from `u4`.
- **P4.2 [read] `mux-forge-regs`:**
  - **what it does:** the probe's exact register and chip-select sequence with no `TXDR` write and no `RXDR` read, then the claim;
  - **registers:** identical to `mux-jedec`'s (P2.3);
  - **the flash:** no `Transmit`-path line.
- **P4.3 [read] `forge-csusp`** (as `forge-regs`, plus `CSUSP` while `SPE` is set):
  - **`direct`:** the store alone makes the flash log `[WARNING] spi1.u4: Transmission finished in the unexpected state: RecognizeOperation`. A device line produced by no transfer.
  - **`mux`:** the controller's finish is suppressed, so nothing from `u4`.
  - **So the witness must be the `Transmit`-path lines only,** never a finish, chip-select or reset line.
- **P4.4 [read] `mux-wrong-cmd`** (a real transfer of `90 00 00 00`, then the claim):
  - `[ERROR] spi1.spi1mux.u4: Command decoding failed on byte: 0x90 (144).`, which today's judge already fails as `renode_error_log`;
  - then three `Warning` `Unhandled operation encountered while processing command bytes: None` lines, each followed by `Noisy` `Handled command: Operation: None, returning 0x0`.
- **P4.5 [read] The witness (S4's answer):**
  - **what it is:** for a JEDEC read, the eight whole `Noisy` lines of P2.2 under the device's path;
  - **its source:** emitted only by `GenericSpiFlash.Transmit`, which only `STM32H7_SPI.TryTransmitData` calls, on a `TXDR` write with `SPE` and `CSTART` set;
  - **the level it needs:** `logLevel -1` on the device source;
  - **forgery:** no store produces them. A firmware can produce them only by making the read.
  - §7.1 does not hold.

### S5. A static SPI fault

- **P5.1 [read; fold 70 %] `mux-absent-jedec`** (the device absent behind its chip select), 3 runs:
  - **record 1:** `0x22020000`, `0x00FFFFFF` (`RXP` rises; each byte is the mux's `0xFF`);
  - **the mux's lines:** `[WARNING] spi1.spi1mux: Tried to transmit byte 0x9F to device 0x0, but it's not connected - ignoring transfer and returning a dummy byte`, then the same for `0x0`, folded to one line ending ` (3)`;
  - **no `[ERROR]`;**
  - **verdict:** today's judge is unaware of SPI, so the SPI value goes unjudged; the I2C verdict is unchanged;
  - the three runs give identical reads.
- **P5.2 [read] `mux-cs-never`** (chip select never asserted):
  - **record 1:** `0x00FFFFFF`;
  - **the mux's lines:** `[WARNING] spi1.spi1mux: Tried to transmit byte 0x9F, but no device is currently selected - ignoring transfer and returning a dummy byte`, then the `0x0` line folded `(3)`;
  - **no line from `u4`;**
  - **for attribution:** the two warnings differ, `not connected` against `no device is currently selected`.
- **P5.3 [read] `direct-cs-never`:** reads `EF4017` with the eight lines, as `jedec`. Without the mux, chip select cannot be faulted.
- **P5.4 [read; bits format 60 %] `mux-mode3`** (`CPHA` and `CPOL` set):
  - **the read:** `EF4017`, the same eight lines;
  - **its only record:** `[WARNING] spi1: Unhandled write to offset 0xC. Unhandled bits: [24-25] when writing value 0x3400000. Tags: CPHA (0x1), CPOL (0x1).`;
  - **`CFG2`:** reads `0x00400000` (the tags are not stored);
  - **the wrong clock phase is not renderable as a data fault** with stock models.
- **P5.5 [read, not run] SCK above the part's maximum is not renderable in the twin.**
  - `MBR` is a tag, and no model has a clock.
  - It is a static check: the bus's `speed_hz` against the part's maximum.
- **P5.6 [read] MISO stuck at a level:**
  - **stuck at 0** renders as `none-jedec` (P2.5): no device, every byte `0`, no log line;
  - **stuck at 1** renders as the mux's `0xFF`, in the `mux-absent` shape (P5.1);
  - **no stock model sits on the line itself.**

### S3 and S6 (read and reasoned; nothing to run)

- **P3.1 [read] Where a part is code today, outside tests:**
  - `aethelgard-ir/src/parts.rs` (identity: register, width, value, source);
  - `aethelgard-twin/src/model.rs` (`MODELS`: part to Renode model);
  - `aethelgard-twin/src/boot.rs` `witness_lines` (the model's log format);
  - `aethelgard-eda/src/schematic.rs`:
    - `KNOWN_PARTS`;
    - per part: symbol, pin map, address straps (`BME280` `SDO`, `TMP117` `ADD0`), the `W25Q32JVSS` symbol standing in for `W25Q64JV`, and 100 nF decoupling;
  - `aethelgard-eda/src/library.rs` and `aethelgard-kicad-lib/src/lib.rs` (symbol and footprint files by part);
  - `aethelgard-eda/src/extract.rs` (`Ci` 10 pF: UM10204's bound for every I2C pin, a bus rule, not a part's);
  - `aethelgard-ir/src/electrical.rs` (UM10204 sink limits, also a bus rule).

  No part carries a supply range, an SPI mode or a maximum clock anywhere.
- **P6.1 [read] S6:**
  - **The margin:** at `OSPEEDR` `10`, Table 149 allows 85 MHz into 50 pF at 2.7–3.6 V, more than four times the reference's 20 MHz, at any capacitance the stackup can give. A gate on SCK edge rate would never fire on the reference.
  - **Where a check would bite:** only at `OSPEEDR` `00` (16 MHz even at 10 pF). The generator never emits `00` for an SPI pin.
  - **A caveat:** the `10`/`11` rows assume the I/O compensation cell enabled (note 5), and nothing enables it.
  - **Prediction for the recommendation:** no analog gate in this work order; a table check belongs with the stackup (`4100`).

## Results — after the predictions

*(To be written after the runs.)*
