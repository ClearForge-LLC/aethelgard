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

Everything below was measured after commit `f61da75`, which holds the predictions above unchanged.

### Where it ran

**The containers.** Two containers of the pinned image ran, each under `spike/4200/img.sh` at `--memory 12g`. Each ran `s2.py`, which builds the eight probe firmwares and boots every case.
- Both read back `memory.max=12884901888`, `memory.swap.max=0` and `pids.max=max` before their command started (`out/caps.log`). Neither logged `CAPS NOT READ`.
- `TMPDIR` was unset inside each.

**The composition.** Every boot used the CLI's composition:
- `--config` with the 19 keys of `renode::config`, synchronous logging on;
- `logLevel -1` only on the device sources the platform declares;
- `BEGIN`/`END` around the reads (`raw/*.cmds` in `~/.cache/aet-4200`).

**Outputs.**
- `out/s2.jsonl` is pass 2, the results below: 18 runs, 14 cases, two of them three times.
- `out/s2-pass1.jsonl` is pass 1 (see "Corrections"), built by `fw.py` as committed at `f61da75`; `out/fw-pass1.txt` holds its ELFs' sha256.
- `out/fw.txt` holds the probe ELFs' sha256.
- Raw stdout stayed in the cache.

**Wall time.** Each boot took 1.9 to 2.1 s, the reference's.

### Corrections during the runs

**Pass 1's mux cases measured a probe defect, kept as evidence.** Pass 1 left `PA1` (the IMU's chip select) undriven.
- **Cause:** Renode's `GPIO.Connect` calls the receiver's `OnGPIO(n, state)` at connection (`GPIO.cs` L80, read after the run). So the mux's active-low input 1 started at `false`, selected, and stayed so.
- **Effect:** every byte of every mux case met two selected devices: `[WARNING] spi1.spi1mux: Tried to transmit byte 0x9F, but multiple devices are currently selected ([0x0, 0x1]) - ignoring transfer and returning a dummy byte`. Each read gave `0x00FFFFFF`.
- **In `mux-cs-never`,** the IMU alone was selected and answered `0x9F` as a read of register `0x1F`: `Unhandled read from offset 0x1F.`, value `0x000000`.
- **Pass 2's fix:** every probe drives `PA0` and `PA1` high (`BSRR` = 3) before any SPI code, as `into_output` does for `PA0`.
- **What it means for the build:** an undriven chip select is a selected part. The generated firmware must idle every chip select high before the first transfer on its bus (F3).

### S1 — read (no run)

- **The controller.** `STM32H7_SPI` holds one device, logs no byte and has no chip select (P1.1, read).
- **The census** (`s1-census.md`, [read-sub]) lists the models; I read `GenericSpiFlash`, `SPIMultiplexer` and `ICM20948` myself.
- **Spot-checked:**
  - the absence of every named flight-controller part, by my own search of both trees: no hit for ICM-42688, ICM-20602, MPU-6000/6050/9250, BMI088, BMP3xx, DPS310, MS5611, W25Q or Winbond;
  - `ICM20948`'s `WhoAmI` = `0xEA` (`ICM20948_Bank0Registers.cs` L15).

### S2 — measured (`out/s2.jsonl`)

- **All five platforms load:** `peripherals` lists `spi1 (STM32H7_SPI)` → `spi1mux (SPIMultiplexer)` → `u4 (GenericSpiFlash)`, `imu (ICM20948)`. No `[ERROR]`; `init: SetActiveLow` accepted.
- **`direct-jedec`:**
  - **record 1:** `0x22020000`, `0x00EF4017`;
  - **the flash's lines:** exactly P2.2's eight, path `spi1.u4`;
  - **nothing else from `u4`:** no warning at load (`Reset()` logged nothing visible), none at `EOT`;
  - **record 0:** `0x22020000`, `0x60`, as the reference.
- **No register witness (P2.3):** every case that ends with the probe's sequence leaves the same `SPI1` registers:

  | Register | Value |
  |---|---|
  | `CR1` | `0` |
  | `CR2` | `0` |
  | `CFG1` | `7` |
  | `CFG2` | `0x00400000` |
  | `SR` | `0x00001002` |

  That includes `jedec` on every platform, `forge-regs`, `wrong-cmd`, `cs-never` and `mode3`. `GPIOA.ODR` is `0x3` after pass 2's idle.
- **`direct-cs-jedec`:**
  - **the read:** as `direct`, with the same eight lines;
  - **each rising `PA0` edge** (`into_output`'s first set-high, then the one after the transfer; the probe's idle re-drives an already-high `PA0` and signals nothing, `GPIO.cs` L47) adds `[NOISY] spi1.u4: Chip Select is deasserted.` and `[WARNING] spi1.u4: Transmission finished in the unexpected state: RecognizeOperation`;
  - **so with the wire, every good read ends with a device warning.**
- **`none-jedec`:** record 1 = `0x22020000`, `0x00000000`; no line from any `spi1` source; no `[ERROR]`.
- **`mux-jedec`, 3/3 identical reads** (`reads_sha256` `c388a871…`, the same as `direct-jedec`'s):
  - **the read:** `EF4017`, with the eight lines under `spi1.spi1mux.u4`.
  - **Around them, the mux's lines:** `GPIO #n set to …` and `Finisihing transmission on device 0x…` (Renode's spelling), and the IMU's `Finishing transmission, going idle`.
  - **Every edge on any input finishes every inactive input's device:**
    - `PA1`'s idle-high finished `u4` again, with a second `Transmission finished in the unexpected state` warning;
    - after the transfer, `PA0`'s rise finished `u4` in `HandleCommand`, with no warning (P2.6).
- **`mux-two`:**
  - **the records:** record 1 = `EF4017`; record 2 = `0x22020000`, `0x000000EA`;
  - **the IMU's lines:** exactly P2.7's four, path `spi1.spi1mux.imu`;
  - **two devices on one controller,** each read witnessed by its own model's lines, in order.

### S4 — measured (the forgeries; `out/s2.jsonl`)

| Case | Record 1 | `u4` `Transmit`-path lines | Other `u4` lines | Today's judge |
|---|---|---|---|---|
| `mux-forge-claim` | OK `EF4017` | none | the idle-edge finish warnings only | SPI unjudged |
| `mux-forge-regs` | OK `EF4017` | none | as above, plus one more at the `PA0` rise | SPI unjudged |
| `mux-forge-csusp` | OK `EF4017` | none (the mux suppresses the controller's finish) | as `forge-regs`; `SR` `0x1802` (`SUSP`, not cleared) | SPI unjudged |
| `direct-forge-csusp` | OK `EF4017` | none | **`[WARNING] spi1.u4: Transmission finished in the unexpected state: RecognizeOperation` from the `CSUSP` store alone** | SPI unjudged |
| `mux-wrong-cmd` | OK `EF4017` (claimed) | `Transmitting data 0x90 …`, then `None` lines | `[ERROR] spi1.spi1mux.u4: Command decoding failed on byte: 0x90 (144).` and 3 × `Unhandled operation … None` | `renode_error_log` |

**What a store can and cannot produce:**
- **Can:** a device line from a store, the finish warning, through `CSUSP` on the direct wiring and through any chip-select edge on the mux.
- **Cannot:** a `Transmitting`/`Decoded`/`Handled` line. None of the five forgeries produced one.

### S5 — measured (`out/s2.jsonl`)

- **`mux-absent-jedec`, 3/3 identical:**
  - **record 1:** `0x22020000`, `0x00FFFFFF`;
  - **the mux's lines:** `[WARNING] spi1.spi1mux: Tried to transmit byte 0x9F to device 0x0, but it's not connected - ignoring transfer and returning a dummy byte`, then three identical lines for `0x0`, **not folded** (P5.1's fold did not happen);
  - **each `PA0` edge** logs `[WARNING] spi1.spi1mux: Tried to finish transmission to device 0x0, but it's not connected`;
  - **no `[ERROR]`;** record 0 and the I2C side unchanged.
- **`mux-cs-never`:** record 1 = `0x00FFFFFF`. Four unfolded `[WARNING] spi1.spi1mux: Tried to transmit byte 0x…, but no device is currently selected - ignoring transfer and returning a dummy byte`. No `Transmit`-path line from `u4`.
- **The other chip-select failure** (pass 1, the defect above): another part's chip select held gives `multiple devices are currently selected ([0x0, 0x1])` and `0xFF`.
- **`direct-cs-never`:** reads `EF4017` with the eight lines, identical to `jedec`. With the direct wiring, chip select cannot be faulted.
- **`mux-mode3`:** reads `EF4017` with the eight lines. Its one extra line is `[WARNING] spi1: Unhandled write to offset 0xC. Unhandled bits: [24-25] when writing value 0x3400000. Tags: CPHA (0x1), CPOL (0x1).`, and `CFG2` reads `0x00400000`.
- **Not measured, read:**
  - **SCK above the maximum:** `MBR` is a tag; no model has a clock.
  - **MISO stuck:** stuck at 0 is `none-jedec`'s rendering; stuck at 1 is the mux's `0xFF`.
  - **An open chip-select trace:** removing `0 -> spi1mux@0` leaves input 0 never connected, so never selected. That logs `no device is currently selected`, the same line as `cs-never`'s software fault.

## The spike's table: each prediction beside its result

| Id | Prediction | Result | Held? |
|---|---|---|---|
| P1.1 | `STM32H7_SPI`: one device, no byte log, finish only at `EOT`/`CSUSP`; tags not stored | read; `CSUSP` finish measured (`direct-forge-csusp`); tags not stored measured (`mode3` `CFG2`) | yes |
| P1.2 | the models table | census [read-sub]; three read | read |
| P1.3 | no named FC part modelled; stand-ins `GenericSpiFlash` (EF 40 17), `ICM20948`/`LSM6DSO`; no SPI barometer | as predicted (my own search); `EF4017` and `0xEA` measured | yes |
| P2.1 | all five platforms load (70 %) | all load, no `[ERROR]` | yes |
| P2.2 | `direct`: `EF4017`, the eight lines, path `spi1.u4`; ≤ 1 load warning (50 %) | exactly; no load warning | yes |
| P2.3 | `jedec` and `forge-regs` leave identical `SPI1` registers | identical (`CR1` 0, `CR2` 0, `CFG1` 7, `CFG2` `0x400000`, `SR` `0x1002`) | yes |
| P2.4 | `direct-cs`: the eight lines plus a deassert and a warning per `PA0` rise | exactly, twice | yes |
| P2.5 | `none`: value 0, no line, no error | exactly | yes |
| P2.6 | `mux`: `EF4017`, path `spi1.spi1mux.u4` (60 %), no warning after the transfer, 3/3 | **pass 1 no** (`PA1` undriven: two selected, `0xFFFFFF`); **pass 2 yes**, and every edge on any input also finishes every inactive device (an extra warning at `PA1`'s idle) | yes after the probe fix; the extra finish not predicted |
| P2.7 | `mux-two`: `0xEA`, the four IMU lines | exactly (pass 2) | yes |
| P4.1 | `forge-claim`: no `Transmit`-path line | none | yes |
| P4.2 | `forge-regs`: registers identical, no `Transmit`-path line | so | yes |
| P4.3 | `forge-csusp`: direct, a device warning from a store; mux, nothing from `u4` | direct, exactly; mux, nothing from the `CSUSP` (the chip-select edges still finish it) | yes |
| P4.4 | `wrong-cmd`: `[ERROR] … 0x90 (144).`, 3 × `Unhandled operation … None` | exactly | yes |
| P4.5 | the witness is the `Transmit`-path lines; no store makes them; §7.1 does not hold | measured on 5 forgeries | yes |
| P5.1 | absent: `0xFFFFFF`, `not connected` lines folded ` (3)` (70 %), 3/3 | `0xFFFFFF`, `not connected`, 3/3; **not folded** | value yes; fold **no** |
| P5.2 | `cs-never`: `0xFFFFFF`, `no device is currently selected` | exactly (unfolded) | yes |
| P5.3 | `direct-cs-never` reads as `jedec` | identical | yes |
| P5.4 | `mode3`: same read, a tag warning `[24-25]` (60 %), `CFG2` `0x400000` | exactly | yes |
| P5.5 | SCK over maximum not renderable | read, not run | read |
| P5.6 | MISO stuck 0 = `none`; stuck 1 = mux `0xFF` | measured as `none-jedec` and `mux-absent-jedec` | yes |
| P3.1 | the inventory | as listed (S3 below) | read |
| P6.1 | no gate in this work order; `4100` | reasoned (S6 below) | reasoned |

**Why no fold (P5.1):** `AET-WO-2204` F5 saw Renode fold repeated lines with `collapse-repeated-log-entries = True`. Here, with the same key and synchronous logging, three consecutive identical warnings were not folded on the console. Why was not established; a hidden line at a lower level between them (for example the controller's `Debug` `Setting IRQ to …`) is an untested cause. The build's witness must neither assume folding nor be broken by it, so it should refuse a ` (N)` suffix on a witness line, as 2204's does.

## Recommendations, one per question

- **S1. Stock models carry the work order's first device; the flight-controller parts are not modelled.**
  - **The flash:** `GenericSpiFlash` stands in for the W25Q64JV, ID set from the platform (`manufacturerId 0xEF`, `memoryType 0x40`, `capacityCode 0x17`), the exact `EF 40 17`.
  - **The IMU:** `ICM20948` (`0xEA`, measured) is the one SPI IMU measured end to end; `LSM6DSO` (`0x6C`) is read only.
  - **Not modelled:** none of ICM-42688-P, ICM-20602, MPU-6000, BMI088, BMP388, DPS310 or MS5611 in 1.17.0.
  - **The barometer gap:** no barometer has an SPI model, so Sentinel-02's barometer is either an I2C part Renode models (`BMP180`, `LPS25HB`, `ICP-101xx`), or a model of our own (D5).
- **S2. Wire every SPI bus through `SPI.SPIMultiplexer`.**
  - **The wiring:**
    - one `<spi>mux` per bus;
    - each device at its own address, with its chip-select pin wired `<port>: <pin> -> <spi>mux@<k>` and `SetActiveLow <k>`;
    - `logLevel -1` on each device.
  - **Why the mux, measured:**
    - it is the only stock way two devices share `STM32H7_SPI`;
    - it is the only one that renders a chip-select fault (`cs-never`: `0xFF` and a warning; direct: identical to a good read);
    - its `0xFF` is what an undriven MISO reads.
  - **Two consequences for the generator:**
    - every chip select idles high before the bus's first transfer (F3);
    - the twin emits two new names per bus or device (`spi1mux`, `u4_memory`), which must stay clear of references and of 2204's reserved list (D7).
- **S3. A pinned data file of part values in `aethelgard-ir`, with each Renode model's behaviour kept as code (D3).**
  - **Home and form:** `crates/aethelgard-ir/data/parts.json`, embedded with `include_str!` and sha256-pinned by a tier-1 test. The precedent is `svd-hal`'s pinned CMSIS header (`cmsis.rs` L26).
  - **Parsing:** serde with `deny_unknown_fields`; `serde_json` moves from a dev-dependency to a dependency of `aethelgard-ir`. `workspace_shape.rs` (`ir_depends_on_no_other_member`) forbids only `aethelgard-*` dependencies there, so this passes it (read; not run).
  - **Validation:** once, at first use, refusing by name (`PartsTableInvalid { part, field, reason }`):
    - a duplicate part;
    - a field out of range, such as an I2C address above `0x7F`, an identity wider than 4 bytes, an SPI mode outside 0 to 3, or a supply minimum above its maximum;
    - a bus field that does not match the bus kind;
    - a Renode model outside the closed set.

    A tier-1 test feeds each such entry.
  - **Fields, one entry per part:**
    - `part`;
    - `bus`: `{"i2c": {"addresses": [..]}}` or `{"spi": {"modes": [0, 3], "max_clock_hz": …}}`;
    - `identity`: `{"write": [0x9F], "read": 3, "value": "EF4017"}` (I2C: `write` is the register);
    - `supply_millivolts: [min, max]`;
    - `pins: {"capacitance_femtofarads": …, "sink_microamps": …}`, optional, present only where a gate reads it;
    - `renode`: one of a closed set of model names, or `null` for unmodelled;
    - `datasheet: {"document", "revision", "section"}`, one per value group.
  - **The witness format** stays a `match` on the model enum in `aethelgard-twin`. It is a fact about Renode's source (`GenericSpiFlash.cs` L129/L298/L635), pinned to Renode 1.17.0, not a fact about the part. That keeps 2204 D2's guarantee: a model added cannot reach the judge without a witness decision.
  - **What migrates:**
    - `parts.rs` identities (BME280, TMP117), plus the W25Q64JV;
    - `model.rs` `MODELS`;
    - `schematic.rs`'s address straps (`allowed` 0x76/0x77 and 0x48–0x4B) and `KNOWN_PARTS`.
  - **What stays in `aethelgard-eda`:** the KiCad symbol, pin and footprint choices, which are layout facts keyed by part, named in FEEDBACK.
  - **What does not migrate:** UM10204's 10 pF `Ci` (`extract.rs`) and sink limits (`electrical.rs`), which are bus rules, not part values.
  - **The alternative:** a typed Rust `const` table. Simpler, with no parser and no runtime refusal, but a malformed entry is then a test failure rather than a refusal by name (§5.3), and every part added is a code change.
- **S4. The SPI witness is the device model's `Transmit`-path lines, as whole lines, in order; never a finish, chip-select, reset or mux line.**
  - **For `GenericSpiFlash` and a JEDEC read:** the eight lines of P2.2.
  - **For `ICM20948` and `WHO_AM_I`:** P2.7's four.
  - **The level:** all are `Noisy`, set by `logLevel -1` on the device.
  - **Measured:** no store produced one in five forgeries. A store does produce the finish warning (`direct-forge-csusp`), and every mux chip-select edge finishes every inactive device, so those lines are evidence for nothing.
  - **The `0x9F` reply beyond 3 bytes** is Renode's (`10 40 00 …`, `GetDeviceData`), not Winbond's, so the read is exactly 3 bytes (F7).
  - §7.1 does not hold.
- **S5. The first SPI fault: the device absent behind its chip select (`mux-absent`).** Measured 3/3: record `0xFFFFFF`, the mux's `… to device 0x0, but it's not connected …` per byte, no error.
  - **Trace:** a kind-13 analogue for SPI, carrying the device's witness lines and the mux's `not connected` lines for that address, as 2204's `absent`.
  - **Attribution rule:**
    - **electrical:** `not connected` at the faulted device's own address, with no line from the model;
    - **software:** `no device is currently selected` (the firmware never asserted chip select) or `multiple devices are currently selected` (another part's chip select held), or the model's lines with another command (`wrong-cmd`'s `[ERROR]`).
  - **A seeded case for each domain is possible:** absent (electrical), and `cs-never` (software).
  - **The other candidates fail:**
    - **wrong clock phase:** unrenderable; the read is unchanged, and only a tag warning records it;
    - **SCK over maximum:** unrenderable; a static check;
    - **MISO stuck at 0:** indistinguishable from an empty `spi1`, with no line at all;
    - **an open chip select:** collides with `cs-never` (the same line).
- **S6. No analog gate in this work order.**
  - **Silicon revision (a correction to the predictions' grounds):** the predictions called the reference rev V silicon; nothing read established that. Rev Y's Table 52 (`ds10.txt` L9371 on) gives the same speed-`10` limits (85/110/166 MHz at 50/30/10 pF, 2.7–3.6 V, note 5), so the margin below holds on both revisions.
  - **The margin:** at `OSPEEDR` `10`, DS12117 Rev 10 Table 149 (rev V) allows 85 MHz into 50 pF at 2.7–3.6 V, more than four times the reference's 20 MHz at any capacitance the stackup can give.
  - **Where a check would fire:** only at `OSPEEDR` `00` (Fmax 16 MHz even at 10 pF), which the generator never emits for an SPI pin.
  - **The check, when it comes:** a static table check, `Fmax(OSPEEDR, VDD, C)` ≥ the bus clock, with C from the routed geometry. It belongs with the stackup and extraction (`4100`).
  - **One caveat for `4100`** (F6): the `10`/`11` rows assume the I/O compensation cell enabled (note 5), and the generated firmware never enables it.

**Not run, and the build's first measurement:** anything across `machine Reset`. `SPIMultiplexer.Reset()` clears its input state (L47–50), so chip-select state after a droop's reset may differ from the state at load. The reference's droop scenarios will cross it.

## §7 checked

- **§7.1 does not hold, measured.** `GenericSpiFlash` and `ICM20948` log each transaction at `Noisy`, and no store produces those lines (S4). `STM32H7_SPI` itself logs nothing per transfer, and no `SPI1` register separates a real transfer from a forgery (P2.3), so the witness is the model's log, as for I2C.
- **§7.2 does not hold.** The W25Q64JV is unmodelled, but `GenericSpiFlash` with its JEDEC bytes is a stock stand-in, measured to read `EF 40 17`.
- **§7.3 not reached.** Every probe left record 0 (the I2C read) at `0x22020000`, `0x60`. The reference's verdicts are the build's test.

## Findings

- **F1 · `STM32H7_SPI` has no chip select and one device; chip select is modelled only by `SPIMultiplexer` · Renode 1.17.0 · high for the twin's wiring · measured.**
  - Direct wiring cannot render a chip-select fault, and cannot hold two devices.
  - Recommendation: S2. Decision needed: D1.
- **F2 · A store makes a device log: the finish warning · `STM32H7_SPI` `CSUSP`, `SPIMultiplexer` input edges · high for the witness · measured (`direct-forge-csusp`, every mux case).**
  - The witness must exclude every finish, chip-select and mux line.
  - Recommendation: S4.
- **F3 · An undriven chip select is a selected part · `GPIO.Connect` L80, `SetActiveLow` · high for the generator · measured (pass 1).**
  - At load every mux input reads low, so active. Until the firmware drives a chip select high, its part is selected, and two such parts turn every transfer into `0xFF`.
  - Recommendation: the generated firmware idles every chip select high before the bus's first transfer. Two seeded possibilities follow (another part's chip select held).
- **F4 · Renode did not fold three identical consecutive warnings · `collapse-repeated-log-entries = True`, synchronous logging · low · measured.**
  - `AET-WO-2204` F5's folding (asynchronous) is not reproduced here; the cause is untested.
  - Recommendation: the witness refuses a ` (N)` suffix and never relies on folding.
- **F5 · No barometer, and none of the named IMUs, is modelled on SPI · Renode 1.17.0 · high for `4500` · read** (after the census: `BMP180`, `LPS25HB` and `ICP_101xx` implement `II2CPeripheral` only, and no class under `Sensors/` implementing `ISPIPeripheral` is a pressure sensor).
  - Recommendation: D5.
- **F6 · DS12117's speed-`10`/`11` limits assume the I/O compensation cell enabled; nothing enables it · `board.rs` (`OSPEEDR` `10`), DS12117 Rev 10 Table 149 note 5 · medium, for `4100` · read.**
  - Recommendation: `4100` either enables it in the generated firmware or reads the margin without it. Not this work order.
- **F7 · `GenericSpiFlash` answers `0x9F` with six or more bytes · `GetDeviceData` · low · read.**
  - The bytes are `EF 40 17 10 40 00 …`, Renode's defaults past the third byte, not Winbond's.
  - Recommendation: the identity read is exactly 3 bytes.
- **F8 · Renode spells its mux line `Finisihing transmission on device 0x…` · `SPIMultiplexer.cs` L107 · low · measured.** Any exact-line rule over it must use Renode's spelling. S4 excludes these lines anyway.
- **F9 · The W25Q64JV's datasheet is not on Furnace · B7's "data licensing per part" · medium · —.**
  - Its JEDEC ID is the work order's. Its modes, maximum clock and supply range are [recall].
  - Recommendation: the build cites a held revision, fetched once and pinned like DS12117. Decision needed: D8.

## Decisions needed

- **D1 (S2): the SPI wiring.**
  - **(a)** a `SPIMultiplexer` per bus, chip selects wired active-low, devices at their index. Recommended, measured: two devices, chip-select faults, `0xFF` MISO.
  - **(b)** the device directly on `spi1`, one device per bus, with no chip-select fault.
- **D2 (S4): accept the model's `Transmit`-path lines as the SPI witness** (2204 D1's rule, for SPI), exact whole lines in order, with no ` (N)` suffix (F4), and every finish, chip-select and mux line excluded (F2). Recommended.
  - **2204's rule cannot be reused as written.** Its judge fails any extra line with a witness's shape (`[NOISY] <device>: `), and these models log `Noisy` lines that are not transfers on every chip-select edge: the flash's `Chip Select is deasserted.`, the IMU's `Finishing transmission, going idle`.
  - **What SPI needs instead:** a per-model, exact list of such lines, which the judge skips. Any other line from the device still fails.
- **D3 (S3): the parts table's form.**
  - **(a)** the pinned `parts.json` in `aethelgard-ir`, `serde_json` promoted to a dependency, refused by name at first use, the model's witness format kept as code. Recommended.
  - **(b)** a typed Rust `const` table.
- **D4 (S5): the first SPI fault.** The device absent behind its chip select, electrical by the mux's `not connected` line, with `cs-never` as its software twin. Recommended.
- **D5 (§2.6): the flight-controller IMU.**
  - **(a)** `ICM20948` declared in a test-fixture specification and read end to end in this build (measured, `0xEA`). Recommended: it is the only measured SPI IMU.
  - **(b)** defer to `4500`.
  - Separately for `4500`: the named IMUs and every SPI barometer are unmodelled (F5). The choice is between modelled parts and a model of our own, which is a ruling.
- **D6 (S6): no analog SPI gate here; the `Fmax` table check and F6 go to `4100`.** Recommended.
- **D7: the twin's new emitted names** (`<spi>mux`, `<ref>_memory`).
  - The recommendation is that `validate` refuses a reference whose lowercased name, or name plus `_memory`, collides with one, as 2204 D5 does for stock names.
  - The alternative is names a reference cannot take (`aet_` prefixed, with references refused from that prefix).
- **D8 (F9): fetch and pin the W25Q64JV datasheet** (Winbond, current revision) for the parts table's citations.

## Gates (spike)

- **The diff against `main`** is this file and `spike/4200/`:
  - the scripts, which are Python and shell;
  - the five platform `.repl` files;
  - the census;
  - the outputs.

  No Rust changed.
- **`cargo fmt`, `clippy` and the tests** are unaffected; CI runs them on the PR.
- **`docs/work-orders/AET-WO-4200.md`** reached `main` through PR #116 (`9035203`), which is merged into this branch (`d6c40dd`). It is byte-identical to the copy this spike worked from (`claude/argon-repo-docs-synthesis-rjrb0d`).

---
# The build (§2), to the architect's ruling on PR #118

**Ruling** (`~/.cache/aet-4200-ruling.md`, 2026-11-14), each decision as built:
- **D1 (a):** a `SPI.SPIMultiplexer` per SPI controller. Each device sits at its index on that controller, and its chip select is wired `<port>: <pin> -> <spi>mux@<k>` with `SetActiveLow <k>`. Each device's source gets `logLevel -1`.
- **D2:** the SPI witness is the model's `Transmit`-path lines, matched exactly. The model's `Noisy` lines that are not transfers form an exact skip list, a `match` on the model enum. Each skipped line is pinned to its Renode source line by a test. Any other device line fails.
- **D3 (a):** `crates/aethelgard-ir/data/parts.json`, read by one parser, `parse_parts`, which refuses a malformed entry by name.
- **D4:**
  - the device absent behind its chip select (electrical, `E2`) and `cs-never` (software, `S3`);
  - trace version 7, with kinds 21 and 22.
- **D5 (a):** the ICM-20948 in a test-fixture specification.
- **D6:** no analog gate here.
- **D7:** the multiplexer names are reserved, and a reference whose name plus `_memory` collides is refused.
- **D8:** all four parts' datasheets fetched and held.

## Before any code: measured

- **Across a reset** (the ruling's first measurement). Run by `spike/4200/s7.py` in the pinned image, 2 runs at each of two reset times (`out/s7.jsonl`, sha256 `1c3ab3f5…`). Composed as `renode.rs` `droop_schedule` composes a droop, but driven directly, not through the bridge:
  - **During the halt:** `machine Reset` resets the GPIO port, which drives both chip selects low, so both mux inputs are selected.
  - **After the release:** the firmware idles them high and reads again. The flash's eight `Transmit` lines appear again between `RELEASE-0` and `PROBE-BEGIN-0`.
  - **At release + 100 µs** the probe reads the boot word set and both records OK (`0x60`, `0xEF4017`).
  - The device's `Noisy` level survives the reset, so an SPI read is re-witnessed after recovery. No §7 stop.
- **The §7.3 baseline:** `main`'s own binaries at `a71bef5` (`~/.cache/aet-4200/baseline/main-bin`):
  - CLI `984910b2…`, bridge `980d1ed0…`, reference firmware `deee3a94…`;
  - every scenario (2 static, 2 droop, every seeded scenario file), every replay, and the static `simulate` at 100 ms;
  - campaign exit 0, `simulate` exit 0.
- **The datasheets** (D8), each fetched from its vendor's own domain into `~/.cache/aet-4200/ds/`, with sha256 checked against the files:

  | Part | Document | Revision | sha256 | Where it came from |
  |---|---|---|---|---|
  | W25Q64JV | Winbond W25Q64JV | J, 2018-03-27 | `452d906c3be0da077fd751a2b8aa062629caf6081f8dd1eda04880854ce7c8f9` | `winbond.com/resource-files/w25q64jv revj 03272018 plus.pdf`; whether J is current is not proven (the product page lists no PDF; a guessed Rev K returned 404) |
  | ICM-20948 | TDK InvenSense DS-000189 | 1.6, 2024-03-12 | `0588625f4f1d6d27382a4322f48073dc0ce569e537c63786b13aab3eeb76b718` | TDK's CDN, linked from its product page |
  | BME280 | Bosch Sensortec BST-BME280-DS001-24 | 1.24, 2024-02-20 | `a2ccdb449fec94380742fe8eec851a11d9bd4142252d332b34682b4deecd7d89` | `bosch-sensortec.com`; the URL says `ds002`, the document says `DS001-24`; its page footers still say `DS001-23` |
  | TMP117 | Texas Instruments SNOSD82D | D, 2022-09 | `78212e91e478035a91165397a8de8a90bfc776759dc445243c033537f0c4b789` | `ti.com/lit/ds/symlink/tmp117.pdf` |

  No citation is pending, so nothing is needed from Scotty. `tests/parts.rs` `no_citation_is_pending` holds the empty list.

## What was built

**The parts table (D3).** `crates/aethelgard-ir/data/parts.json` holds BME280, ICM20948, TMP117 and W25Q64JV.
- **Embedding:** `include_str!`, with the file's sha256 pinned by `tests/parts.rs`.
- **Parsing:** one parser, `parts::parse_parts`, reads it. It never panics.
- **What it refuses** (`PartsTableInvalid`, naming the part and the field), each a tier-1 test in `tests/parts.rs`, 28 in all:
  - an unknown key at any level;
  - a bus holding neither or both of `i2c` and `spi`;
  - an address, mode, clock, width, value or supply out of range;
  - malformed hex;
  - an unknown Renode model, or one on the wrong kind of bus;
  - a citation neither held (sha256) nor pending (an https URL), or both;
  - a duplicate part;
  - a part name that is not an identifier;
  - another schema version.
- **How a malformed table surfaces:** `parts::table()` returns the parse once, as a `Result`. `Specification::validate` and `SolvedTopology::validate` end by reading it, so a malformed embedded table is `validate`'s JSON refusal, never a panic.
- **Model behaviour stays code:** the witness format and the skip list live in `aethelgard-twin` `model.rs`, an exhaustive `match`.
- **What migrated:**
  - `parts.rs`'s identities (`write`, the byte written first, replaces `register`; the free-text `source` is gone);
  - `model.rs`'s `MODELS`, now read from the table's `renode`.
- **What stays in `aethelgard-eda`, as the ruling says:**
  - the KiCad symbols, pin maps and footprints;
  - `W25Q32JVSS` standing in for `W25Q64JV`'s symbol;
  - `KNOWN_PARTS`;
  - the strap pins (`SDO`, `ADD0`).

  `schematic.rs`'s allowed-address lists were not migrated, because each is the strap's own mapping (which pin to which rail gives which address), a layout fact beside the pin map (F12).
- **What stays in `aethelgard-ir` as bus rules:** UM10204's `Ci` and its sink limits.

**`SolvedTopology` version 3.**
- **`spi_devices`:** reference to `{index, part, bus, peripheral, cs}`, filled by `solve` from the bus's `sck` assignment, read through `spi_devices_in_order` (`RUL-16`: the twin and the generators learn devices only from the topology).
- **Validation:** an SPI device must be on a solved `SPIn` peripheral, its `cs` net must be a solved GPIO, and its indices must be exactly `0..n`.
- **Older files:** a version-2 file is refused as `UnsupportedSchemaVersion`, never as a missing key.
- **Digest moves:**

  | Solve | Before | After |
  |---|---|---|
  | Reference | `c655bf2e…` | `c28a36bf…` |
  | Stress fixture | `6eda552c…` | `6b9107c3…` |

**D7.** `RESERVED_REFERENCES` gains `spi1mux` to `spi6mux` (87 names; `architecture.md` §2.20 says 81). `twin/tests/reserved_names.rs` holds it equal to the pinned chain's names plus one mux per SPI controller the pinned header has (six, checked). `references_distinct`, in `Specification::validate` and now in `SolvedTopology::validate` too, refuses:
- two references equal in any case (`DuplicateReference`);
- a reference whose lowercased name plus `_memory` is another reference, or a reserved name (`ReferenceCollision`).

The collision matrix, one assertion per cell, is in `ir/tests/validate.rs`. No reserved name ends in `_memory`, so that cell is empty by construction, and a test holds it so.

**The twin (D1, D2, D4).** `platform.rs` emits, per SPI controller with a device:
- `<spi>mux: SPI.SPIMultiplexer @ <spi>`, with `SetActiveLow k` for each device;
- one block per GPIO port, `<pin> -> <spi>mux@k`;
- then each modelled device present:
  - a flash: `<ref>_memory: Memory.MappedMemory` of `2^capacity` bytes, then `<ref>: SPI.GenericSpiFlash @ <spi>mux k` with `manufacturerId`, `memoryType` and `capacityCode`, all from the table's identity bytes;
  - the IMU: `Sensors.ICM20948 @ <spi>mux k`.

An unmodelled SPI device keeps its wiring, is commented, and is listed in the header. `emit_faulted(topology, faulted, absent)` leaves each absent device out behind its chip select, keeps its wiring, and names it in the provenance. The output matches the platform the spike measured, line for line.

`boot.rs`: `I2cCheck`/`I2cPlan` became `IdentityCheck`/`IdentityPlan`.
- **`Wire`:** each check carries one, `I2c { address }` or `Spi { address, select }`.
- **Order:** the I2C checks come first, then the SPI ones, in the specification's order; records are consecutive.
- **Statuses:** `judge` counts a status only on its own bus. An I2C record holding `STATUS_RXP` (`0x22020005`, new) is invalid, a defect a tier-1 test caught during the build.

**The judge (D2).** `renode.rs` collects each check's witness-shaped lines. Its skip lines are those exactly in the twin's list; any other line from the device fails.
- **Finding kinds** carry the bus:
  - `spi_identity_mismatch`;
  - `spi_timeout` (stage `RXP`);
  - `spi_record_invalid`;
  - `spi_transaction_unwitnessed`;
  - `spi_witness_unavailable`.
- **Order** is judged per bus: `i2c_witness_out_of_order` keeps its name, and `spi_witness_out_of_order` is new. A cross-bus order would have renamed an I2C verdict on any board with SPI (§7.3).
- **Status names** come from each check's own judgement.
- **Evidence:** each SPI check's `unrouted` holds every whole `[WARNING] <spi>.<spi>mux: Tried to transmit byte 0x…` line: attribution's evidence, never a witness.
- **The verdict** gains `boot.spi` and `boot.spi_witness`. Every I2C message is byte-identical.

**The board crate and firmware.** Built by an Opus subagent in its own worktree from `9b47c1d`, and cherry-picked (`6ca2be8`, `5c0875c`, `3c55ade`); its report is summarised here and spot-checked against the generated code.
- **Byte access:** `Registers` gains `read8`/`write8`. `Mmio` uses `u8` volatile access, because a 32-bit `TXDR`/`RXDR` access packs four 8-bit frames on silicon (RM0433 §50.4.13) and Renode does not pack.
- **The chip-select typestate:**
  - `Spi1Selects { pa0: PA0<Output> }` and `enum Spi1Device { U4 }`, the reference upper-cased;
  - `Spi1::init` consumes the selects, so no transfer exists before every chip select of the bus went through `into_output`. That is F3 at the type level, and `init` also drives each high itself;
  - `init` sets `DSIZE` = 7 and `MASTER`, and leaves `SPE` off.
- **The transfer:** `transfer(device, write, read)`:
  1. `TSIZE`;
  2. select;
  3. `SPE`;
  4. `CSTART`;
  5. per frame, one `TXDR` byte (`0x00` dummies while reading), a wait on `RXP` (at most `SPI_WAIT_SPINS`), and one `RXDR` byte only after `RXP`;
  6. `EOTC`, `SPE` off, deselect, also after a timeout.

  It returns `SpiError::Length` for 0 or more than 8 frames, touching nothing, and `SpiError::Rxp` on a timeout. SPI's 32-bit `write_byte` is gone.
- **The firmware:** after the I2C reads, `spi1.transfer(&mut r, Spi1Device::U4, &[0x9f], &mut buf1)`, exactly 3 bytes (F7), recorded as record 1.
- **Its tests:**
  - **Compile-fail cases (trybuild):** `init` without the selects (E0061), a select not in `Output` (E0308), a transfer naming no device of the bus (E0308).
  - **Text-order checks:** F3 on the reference and on a two-device board, with a negative control.
  - **A second device:** it gets its own select, and is read after the flash.
  - **`SharedChipSelect`:** one pin as two devices' chip select is refused.
- **Kani** (Kani 0.68.0, the pinned image), every harness passing:

  | Board | Harness | Covers | Time |
  |---|---|---|---|
  | Reference | `every_valid_sequence` | 8/8 | 115 s |
  | Reference | `i2c1_transfer_is_bounded_and_ordered` | 7/7 | 5 s |
  | Reference | `spi1_transfer_is_bounded_and_ordered` (new) | 4/4 | 7 s |
  | Reference | `spi1_transfer_refuses_too_many_frames` (new) | 1/1 | 3 s |
  | Four-device | `every_valid_sequence` | 14/14 | 304 s |

  The new SPI harness checks four things: the transfer touches only SPI1 and the chosen device's `BSRR`; it selects before the first byte and deselects on every path; it never reads `RXDR` without `RXP`; and it ends with `SPE` off.
- **Mutation:** 13 mutants of the generated SPI code, one each, all killed.
- **A cap not met (F13):** the subagent's Kani container ran with `--memory 24g` but was denied writing `0` into `memory.swap.max`, which read back `25769803776`.

**Chaos (D4).**
- **Trace version 7** (6 is `AET-WO-2205`'s, on PR #119, not yet on `main`):
  - **kind 21, `SpiCheck`:** reference, peripheral, select, mux address, command, expected, observed, status;
  - **kind 22, `SpiWitness`:** peripheral, reference, device, expected, observed, `unrouted`, each line bounded and printable as kind 13's.
- **Override 4, `[4, reference]`:** `{"spi_device_absent": "U4"}`, admitted only at scenario `schema_version` 2, as 2301 admitted the droop. The CLI refuses a reference the solved topology has no SPI device for (`UnknownDevice`).
- **Run rules:**
  - kinds 21 and 22 rank after 12 and 13;
  - each SPI read appears once and is witnessed once, under `<spi>.<spi>mux.<ref>`;
  - no device is both I2C and SPI.
- **Refusals:** versions 1 to 6 are refused by name. The golden trace changed only in its version byte: set back to 5, it is version 5's golden trace again.

**Evolve.**
- **`E2` (electrical, evidence kind 22):** the multiplexer routed a byte to the device's own address and found no device there, and the model logged nothing.
- **`S3` (software, kinds 21 and 22):** the multiplexer found no chip select asserted, or several; or the model logged lines other than the read's; or the read claims completion with nothing logged or routed.
- **Proposals:** `E2` proposes nothing (no rule, as for a held line). `S3` gives a generator report, through a `Fields` view of kind 22: `select`, `command` and `read`, each with its solved value.

  A test subagent found that `S3` first yielded no report, because kind 22 had no view. That is fixed and tested (`tests/spi.rs`, 11 tests).
- **The seeded set** gains two cases, 24 in all:
  - **`spi-absent-u4`:** a scenario, electrical, `E2`;
  - **`cs-never`:** a mutant of `board.rs` whose select drives `PA0` high, software, `S3`.

**The trace's other records are unchanged.** Kind 19's probe still reads the first check, U2's.

## Measured, at the build

**Tier 1 (host):** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` (also with `--features aethelgard-cli/with-tools`) and `cargo test --workspace` are all clean.

**Tier 2 (the pinned image, under `spike/4200/img.sh` then `~/.cache/aet-4200`'s copy; every container read back `memory.swap.max=0`, `pids.max=max`):**

| Binary | Result | Note |
|---|---|---|
| `twin_tier2` | 25/25 | 3 new |
| `campaign_tier2` | 8/8 | Both reference droops hold with the SPI read in place: `droop-bor3` resets once and recovers, with U4 re-witnessed after the release |
| `attribute_seeded` | 12/12 | All 24 cases attribute as their manifest says: `spi-absent-u4` → `E2` (evidence kind 22), `cs-never` → `S3` |
| `board_tier2` | 18/18 | `verify` now runs four `prove` harnesses |
| `with_tools` | 23/23 | |
| `adversarial_1301_tools` | 6/6 | |
| `adversarial_1302_tools` | 2/2 | |

**Each new tier-2 test alone, in the image** (`ci/test-weights.tsv`, local seconds, not CI's): 3.07, 6.24, 3.40 s. `TIER2_MIN` is 91 + 3 = 94.

**§7.3 against `main`** (`~/.cache/aet-4200/baseline`, each side's own CLI, bridge and reference firmware; all 13 scenarios, every replay keyed by scenario, the static `simulate` at 100 ms):
- **Every campaign line and every replay** is identical but for:
  - `event_count` (+2: kinds 21 and 22);
  - the state hash;
  - the firmware and topology artifacts.

  Every verdict and every finding is unchanged.
- **The new seeded scenario `spi-absent-u4`** fails as declared: `spi_identity_mismatch`, `spi_transaction_unwitnessed`.
- **The static `simulate`:**
  - the verdict, every I2C field, the registers and `icsr` are identical;
  - it gains `boot.spi` and `boot.spi_witness`;
  - `boot.pc` moved from `0x08000594` to `0x080006e8`, the longer firmware's loop;
  - `machine.peripherals` lists `spi1mux` and `u4` beside `spi1`;
  - the platform's and firmware's hashes moved.

**The spike's outputs**, deleted with `spike/4200/` at this build (sha256 as committed, first 12):

| File | sha256 |
|---|---|
| `out/caps.log` | `e51b7448d3d9` |
| `out/fw-pass1.txt` | `f6723ea4ce19` |
| `out/fw.txt` | `e4355a4d1c0f` |
| `out/s2.jsonl` | `1eff191b6fc9` |
| `out/s2-pass1.jsonl` | `339cc1ce671b` |
| `out/s7.jsonl` | `1c3ab3f5c245` |
| `platforms/direct-cs.repl` | `09cfd39a09c1` |
| `platforms/direct.repl` | `4b369f244374` |
| `platforms/mux-absent.repl` | `df3041c104a2` |
| `platforms/mux.repl` | `b38cce5063b9` |
| `platforms/none.repl` | `9d12fe3256ee` |
| `s1-census.md` | `596fa5f0ee7e` |

A copy is in `~/.cache/aet-4200/spike-copy`. Every measurement the build relies on is now a Rust test (`twin_tier2`, `campaign_tier2`, `attribute_seeded`).

## Deviations and decisions, each named

1. **`SolvedTopology::validate` now folds references** (`references_distinct`). `AET-WO-2204` measured it not doing so. Two `twin` tests that expected the emitter's `InvalidDeviceName` for `U2`/`u2` now get `InvalidTopology`, earlier and stricter.
2. **The multiplexer names (`spi1mux`) are reserved for every reference, I2C ones too**, as is the `_memory` rule: a name the platform declares is reserved whatever bus the reference is on.
3. **`spi_device_absent` needs scenario `schema_version` 2,** admitted as 2301 admitted the droop; no version 3.
4. **The order of witnesses is judged per bus.** A cross-bus order would have renamed `i2c_witness_out_of_order` on any board with SPI.
5. **A status counts only on its own bus,** and status names come from each check's judgement.
6. **The board crate** (the svd-hal subagent's decisions, kept):
   - `init` leaves `SPE` off, and each transfer enables and disables it (RM0433 §50.11.2: `TSIZE` is written with `SPE` clear);
   - `DSIZE` = 7 is set once in `init`;
   - `CR1`, `CFG1` and `CFG2` are written read-modify-write;
   - `SPI_MAX_FRAMES` = 8;
   - no wait for `EOT` before `EOTC`;
   - the device enum's variants are the references upper-cased;
   - `SharedChipSelect` refuses one pin as two devices' chip select.

   The read-modify-write and `SPE`-off choices were unmeasured when made. Every tier-2 boot above measures them on Renode.
7. **`edit` in `twin_tier2` now requires its find to occur exactly once.** `a_value_one_bit_off_fails_as_an_identity_mismatch`'s find (`fold(0u32,`) matched the SPI read's fold too and mutated both, so it now names `buf0`.
8. **The ICM-20948 fixture is generated through the library,** not `aethelgard generate`: the vendored KiCad libraries have no ICM-20948 symbol, and eda refuses the part.
9. **Kind 22's generator report names the board crate when the chip select is the wrong field,** and the firmware otherwise. The board crate's transfer drives the selects; the firmware passes the command and keeps the record.

## Findings for the architect (build)

- **F10 · The ICM-20948's `VDDIO` is 1.71 to 1.95 V · DS-000189 §3.4 · high for `4500` · read.** On a 3.3 V SPI bus it needs a level shifter, or a 1.8 V I/O domain. The fixture is the twin's only; the table's `supply_millivolts` holds `VDD` (1.71 to 3.6 V). Recommendation: `4500`'s IMU choice weighs it.
- **F11 · The W25Q64JV's JEDEC device ID depends on the ordering code · W25Q64JV Rev J §8.1 · medium · read.**
  - The IQ and JQ parts answer `EF 40 17`; the IM and JM parts answer `EF 70 17`.
  - The table holds the IQ/JQ value, which the work order and the reference assume.
  - Recommendation: a specification names its ordering code, or the table's part name does (`W25Q64JVxIQ`).
- **F12 · The I2C strap address lists stay in `aethelgard-eda` `schematic.rs`,** beside each strap's own mapping (which pin to which rail gives which address). The table's `bus.i2c.addresses` lists them too, cited, so two sources hold one fact. Recommendation: `schematic.rs` checks its strap targets against the table (a reduction for the next eda work order). Not done here: it is outside the fence.
- **F13 · One container ran without the swap cap.** The svd-hal subagent's Kani runs used `podman run --memory=24g`. Writing `0` into the container's `memory.swap.max` was denied to it, and it read back `25769803776`. The host's swap was full, so swap use was unlikely, but the cap rule was not met for those runs. Every container of mine read back `memory.swap.max=0`.
- **F14 · `GenericSpiFlash`'s `0x9F` reply beyond the third byte is Renode's, not Winbond's** (`10 40 00`). The read is exactly the table's 3 bytes (F7), so nothing judges those bytes.
- **F15 · TMP117 `Device_ID`'s low nibble is a revision field** (`Rev[3:0]`, `R-x`, SNOSD82D §7.6.11). The table's `0x0117` is the value at reset of the current revision. The TMP117 is unmodelled, so nothing judges it today.
- **F16 · The multiplexer's lines are collected per controller, not per read.** A device's `unrouted` evidence holds every such line on its controller during the run, another device's reads included. `E2` requires `not connected` at the device's own address, so another device's absence cannot make it electrical.
  - A `not connected` line for another address is therefore no evidence (`I4`, tested). The I2C path treats a wrong address as software; SPI does not, because the line cannot be tied to this device's read.
  - Recommendation: per-read windows (markers between reads) if `P4` puts two read devices on one bus.
- **F17 · PR #118's title still describes the spike.** `gh-write` has no command to edit a PR, so the builder cannot retitle it. Proposed title: **"AET-WO-4200: SPI devices end to end — parts as data, a witnessed SPI read behind a chip-select multiplexer, a static SPI fault"**.

## Adversarial pass (full, a fresh Opus subagent, on a frozen copy at `1fe460d`)

Run in its own worktree. Four containers, each read back `memory.max=17179869184 memory.swap.max=0 pids.max=max`. Harnesses are in that worktree's `adversarial/` (23 tier-2 tests, tier-1 archive and table harnesses); none reached the branch.

**Held, each measured.**
- **Every SPI forgery failed** `spi_transaction_unwitnessed`:
  - a claim with no transfer;
  - the `SPI1` registers by hand with `CSUSP` (observed `[]`);
  - `CSTART` with `RXDR` reads and no `TXDR` (plus `renode_error_log`);
  - receive-only mode;
  - a 2-byte read, and a 4-byte read with 3 recorded;
  - another flash's read recorded as `U40`'s;
  - a select of another part held low through the read: the multiplexer's `multiple devices`, `S3`.
- **Every correct read passed:**
  - `CPOL`/`CPHA` = 1, and `MBR` = /256;
  - the chip select asserted after `SPE` and `CSTART`;
  - `PA0` low before `init`;
  - `--for` from 500 µs to 5 ms;
  - a path that is a prefix of another (`u4` and `u40`).

  The reference droops held.
- **The parts table:** no `unwrap` or `expect` on `table()` or `identity()` outside tests.
- **Override 4:** refused by name for an I2C reference, a case variant, an unknown device, `schema_version` 1, the same device twice, and a malformed value.
- **D7:** `SPI1MUX` and `U2_MEMORY` beside `U2` are refused.

**Broken, then fixed in this build (`16f6bbe`), each with a test that fails on `1fe460d` and a mutant per new guard:**
- **A1 (high) · A droop on a board whose first identity read is SPI stopped the campaign** (exit 70: "an observation's readings are not the boot's own").
  - **Cause:** chaos took the first read from kinds 12 and 13 only, while the CLI's probe takes the first check of either bus. The probe also collected the device's lines without the skip list.
  - **Fix:** `first_witness` and the probe's check take I2C first, then SPI. The probe uses `device_line`, so an ICM-20948's idle lines no longer fail its recovery.
  - **Test:** `campaign_tier2::a_droop_on_a_board_whose_first_read_is_on_spi_recovers_and_replays` (fixture `spi-only.json`, the reference without U2; `droop-bor3` passes and replays, and the probe carries U4's eight lines).
- **A2 (medium) · `parse_parts` collapsed a key given twice in an entry, and read a missing `renode` as unmodelled.** With the W25Q64JV's `renode` removed, `generate` listed U4 unmodelled, and its read would never have been judged: a false-pass path.
  - **Fix:** each entry is held as raw JSON (`serde_json`'s `raw_value` feature, workspace-wide) and parsed strictly. `renode` is required (`null` is unmodelled). A part twice in any case, and an empty table, are refused.
  - **Test:** `ir/tests/parts.rs` `a_key_twice_a_missing_renode_a_case_twin_or_no_parts_is_refused`.
- **A3 (medium) · Replay joined no SPI record to anything,** while attribution decided `E2` on kind 21's address.
  - **What was accepted:** a forged address with `not connected` at it (attributed `E2`); a `not connected` line from another source (attributed `E2`); an absent override naming no device; an "absent" device whose model answered; a malformed select.
  - **Fix:** kind 23, `SpiBusSolved` (peripheral, then each device's reference, multiplexer address and chip select), from the solved topology through `platform::spi_wiring`, archived beside kind 15. Replay requires:
    - each kind 21 at its kind-23 address and select;
    - every multiplexer line in a kind 22 to be its own multiplexer's whole line;
    - every absent device to be a solved one, whose model logged nothing.

    Evolve's `E2` also requires the multiplexer's own source.
  - **Test:** `evolve/tests/spi.rs` `replay_refuses_spi_records_that_do_not_join_the_solved_controller`. Four mutants, all killed.
  - **The trace stays at version 7.** No archive of version 7 exists outside this branch.
- **A4 (medium-low) · A firmware could make a kind 22 over the reader's 4,096-byte event bound,** and the campaign stopped with `ArchiveWriteFailed`.
  - **Fix:** kind 22's observed and multiplexer lines are each cut to a byte budget (1,800 and 1,200), keeping the longest prefix. The judge compared them whole first, so a cut observation still differs.
  - **Test:** `a_kind_22_stays_inside_the_event_bound_however_long_its_lines`.
  - **Not fixed, outside the fence:** kind 13 has the same bound (16 × 256 bytes), inherited from `AET-WO-2204` (F18).

**Findings, not fixed (decisions or residuals):**
- **F18 · Kind 13 can reach the 4,096-byte event bound** at its line bound (16 × 256), inherited from `AET-WO-2204`. Recommendation: the same budget, in the work order that next touches kind 13.
- **F19 · The table's `modes`, `max_clock_hz` and `supply_millivolts` have no consumer** (adversary A5, measured).
  - An SPI bus at 200 MHz with the W25Q64JV (maximum 104 MHz) passes `validate`, `solve` and `generate`.
  - A part on the wrong kind of bus passes `validate` and `solve`, and `generate` refuses it with eda's message ("no vendored model; known: BME280, …"), which contradicts itself.
  - Spike P5.5 called SCK-over-maximum "a static check", but no work order holds it.
  - Decision needed: a `validate` gate (the bus's declared speed against each part's `max_clock_hz`, the part's bus kind), in which work order.
- **F20 · A malformed flash identity reaches `generate`.** A `SPI.GenericSpiFlash` entry whose identity is not `0x9F`, 3 bytes, capacity `0x10..=0x19`:
  - passes `validate`;
  - `generate` refuses it as `FlashIdentity`, with no `id`;
  - a command other than `0x9F` is emitted, and the judge names it `spi_witness_unavailable`.

  It is caught, but late. Recommendation: the parser checks a model's identity shape. That is a model fact in `aethelgard-ir`, so it is the architect's call.
- **F21 · The controllers' own names (`spi1` and the other `SPIn`) are not reserved** (pre-existing). A reference `SPI1` passes `validate` and is refused by `generate` (`InvalidDeviceName`, no `id`). Recommendation: add them to `RESERVED_REFERENCES` with the multiplexer names.
- **F22 · D2's "any other line from the device fails" holds for the device's `Noisy` lines.** The judge reads only `[NOISY] <device>: ` lines, as `AET-WO-2204` built it. A device's `[WARNING]` (`Transmission finished in the unexpected state`) is not read. Every `Transmit` line is `Noisy`, so no witness rests on a `Warning`.
- **Residuals the adversary confirmed, each named before:**
  - two devices on one controller, another one absent (F16);
  - `0xFF` dummy bytes fail the flash's witness, which expects `0x00` (the generated firmware clocks `0x00`; the Kani spy requires it);
  - a second read after the identity read fails (the one-transaction rule, `2204` A4);
  - an open chip-select trace renders as `cs-never` (spike S5);
  - `cs-never` with U4 absent is `S3`: only the firmware's fault is observable.
- **Not tried by the adversary:** U5 absent on the ICM-20948 fixture (`campaign` refuses the IMU: no KiCad symbol), a reset during a transfer, A4 within a multi-scenario campaign, and Kani.

## Reduction pass (diff)

Run after the adversarial build and before its fixes; re-checked after them.

Reduction pass (diff): −4 net · 2 duplicates removed (S3's software conditions, stated once in `spi_fields`; the skip filter, once in `device_line`, now also the probe's) · 0 dead paths removed · 1 protected kept (chaos's restatement of the multiplexer path, now labelled `belt-and-suspenders:`) · 1 finding below (F12: eda's strap address lists beside the table's).

Each surviving guard's mutant fails a test:
- the select field: 5 tests;
- the claimed-read event: 1 test;
- the skip filter: killed only after a tier-1 test was added for it (`renode::tests::a_device_s_line_is_kept_unless_its_model_lists_it_as_skipped`);
- the four replay joins.

## Merges of `main`

- **`2967d16` (`AET-WO-1909`, CI reduction), at `1fe460d`:**
  - `board_tier2.rs`'s `verify` tests moved to `verify_2k2_tier2.rs`, and the four `prove` gates went with them;
  - `TIER2_MIN` was 1909's 90 plus 3.
- **`d00c965` (`AET-WO-4100`, the board section), at `748a733`:**
  - **The fixtures:** `spi-imu.json` and `spi-only.json` are rebuilt from the migrated reference (specification version 2, its board section copied), each differing from it only as before.
  - **`TIER2_MIN` 96:** 4100's 92 plus this build's 4.
  - **Digests re-taken on the merged code:** the solve digests (reference `3c39199f…`, stress `3169ae88…`) and `order_2203`'s firmware and both platforms (`c468b3cb…`, `9fd77dad…`, `c80833a9…`), each naming 4100's own value on `main`.
  - **One expectation moved:** `campaign_tier1::a_solved_topology_is_not_a_base` now expects `UnsupportedSchemaVersion`, not `SchemaViolation`. The topology is at version 3 and a specification at version 2, so the reader refuses it at its version, before its shape; it is still refused before any run.
- **`d1385f3` (`AET-WO-2205`, trace version 6), at `4ac9f3c`:**
  - **The trace version is 7,** after 2205's 6:
    - `archive.rs` refuses versions 1 to 6 and 8;
    - the golden trace (`dd0dd2db…`, state hash `abf1ccd2…`) differs from 2205's golden in its version byte alone: set back to 6, it is `1f9442fe…`, and 2205's own chain back to 5 and 4 runs on that.
  - **2205's renames carried into this build's tests:** `RiseComputed` and its window, kind 10's `executable`, and the header's `tools`.
  - **Evolve:** `Rule::ALL` holds E2 and S3, so `Rule::from_id` finds them, and the eleven-rule test counts eleven.
  - **The manifest:** the `cs-never` mutant is restored in the schema-2 manifest; the merge had kept only 2205's side of the mutants hunk.
  - **`TIER2_MIN` 96:** `main` is still 92 after 2205, and the merged tree lists exactly 96 tests in `ci/shard.sh`'s `TIER2_RE` binaries.

  | Binary | Tests |
  |---|---|
  | `with_tools` | 23 |
  | `twin_tier2` | 25 |
  | `board_tier2` | 14 |
  | `verify_2k2_tier2` | 2 |
  | `against_fab_tier2` | 2 |
  | `verify_47k_tier2` | 2 |
  | `campaign_tier2` | 9 |
  | `attribute_seeded` | 11 |
  | `adversarial_1301_tools` | 6 |
  | `adversarial_1302_tools` | 2 |
  | **Total** | **96** |

### The seeded table's indices, re-derived (`attribute_seeded.rs`'s rule)

**The record added, and where.** This build adds kind 23 (`SpiBusSolved`, SPI1 with U4 at address 0, chip select `PA0`). Every reference run archives it at index 3:
- it ranks with kind 15 (both are the solved buses, before the boot);
- the campaign writes it straight after kind 15, which is index 2 in every seeded run (index 0 the scenario, 1 the topology; no seeded scenario declares `expect_counts`).

Every record from index 3 on therefore moves up by one: the analog records (kinds 7, 9), the boot (10), the registers (11), the I2C read and witness (12, 13). Kinds 21 and 22 also join after kind 13, which moves the verdict too, but no case names the verdict.

**Measured:** `attribute_seeded` (11/11, the pinned image, `p3_gate_clause_1` asserting each case's exact evidence and masked events) passes on `4ac9f3c` with the table below. Each changed row is 2205's row with every index of 3 or more plus one, and no other change:

| Case | 2205's evidence → this build's | Masked |
|---|---|---|
| `pullup-3k9`, `pullup-4k7`, `pullup-10k`, `pullup-47k`, `pullup-100k`, `cap-220p-2k2`, `trap-indeterminate-leave-47k` | `[3,7] [4,7] [5,9]` → `[4,7] [5,7] [6,9]` | — |
| `cap-390p-sda-2k2` | `[4,7] [5,9]` → `[5,7] [6,9]` | — |
| `held-low-sda`, `held-low-scl` | `[4,9]` → `[5,9]` | — |
| `trap-wrong-address-47k` | `[3,7] [4,7] [5,9]` → `[4,7] [5,7] [6,9]` | `[15,13]` → `[16,13]` |
| `wrong-address-2k2`, `trap-wrong-address-3k3`, `wrong-register`, `wrong-nbytes` | `[3,7] [4,7] [14,13]` → `[4,7] [5,7] [15,13]` | — |
| `i2c-clock-skipped` | `[3,7] [4,7] [11,11]` → `[4,7] [5,7] [12,11]` | — |
| `scl-af-wrong`, `sda-af-wrong` | `[3,7] [4,7] [9,11]` → `[4,7] [5,7] [10,11]` | — |
| `scl-mode-output` | `[3,7] [4,7] [8,11]` → `[4,7] [5,7] [9,11]` | — |
| `gpiob-clock-skipped` | `[3,7] [4,7] [8,11] [9,11] [10,11]` → `[4,7] [5,7] [9,11] [10,11] [11,11]` | — |
| `no-boot-word`, `leave-firmware` | `[3,7] [4,7] [5,10]` → `[4,7] [5,7] [6,10]` | — |

`trap-unmutated-3k3` and `unmutated-2k2` (evidence empty) do not change.

**This build's two cases, derived from `attribute`'s output:**
- **`spi-absent-u4`:** `E2`, evidence `[17,22]`, masked none, symptoms kinds 21 and 22 at 16 and 17.
- **`cs-never`:** `S3`, evidence `[17,22]`, masked none.

Kind 22 is at 17. A seeded run's records, on this build:

| Index | Record |
|---|---|
| 0 | the scenario |
| 1 | the topology |
| 2 | kind 15 |
| 3 | kind 23 |
| 4, 5 | the two rises (kind 7) |
| 6 | the boot (kind 10) |
| 7 to 13 | the seven registers (kind 11) |
| 14, 15 | U2's read and witness (kinds 12, 13) |
| 16, 17 | U4's read and witness (kinds 21, 22) |
| 18 | the verdict |

2205's `[14,13]` is the same witness one index lower.

## CI

| Head | Result | Shards | Slowest |
|---|---|---|---|
| `1fe460d` | green | 8 | shard 1, 17m11s |
| `c1d151d` | one test red, then fixed | — | — |
| `6040fc1` | green | 8, from 9.0 to 17.2 min | shard 1, 17.2 min |

The `c1d151d` failure: `a_droop_on_a_board_whose_first_read_is_on_spi_recovers_and_replays` generated its board under `CARGO_TARGET_TMPDIR`. CI keeps that inside the repository, so cargo refused the crate as a workspace member. It now generates under the system temp directory; reproduced and fixed with a target directory inside the repository.

**The merged head `748a733`** (run 43595196849): green, 8 shards from 11.6 to 15.1 min, the slowest shard 0, and `gates` 10.3 min. `main` then gained `4fc60e9` (docs only), merged at `597b3bb`: green (run 43596501423).

**After the merge of `AET-WO-2205`, at `3dccd3a`** (run 43597954549): green, 8 shards from 12.6 to 18.4 min, the slowest shard 0, and `gates` 9.1 min. `main` then gained `99544b5` (2205's reconciliation, docs only), merged at the head that carries this line; CI runs on it.
