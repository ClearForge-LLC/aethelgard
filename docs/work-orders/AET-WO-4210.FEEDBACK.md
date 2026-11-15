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

### Third session (2026-11-15): S2 and S3 measured under the replacement rule 3

The architect's ruling on PR #131 (comment `6888603447`) replaces rule 3 with `~/.cache/aet-bus/RULE3.md`. Each container runs inside a capped user scope that it shares: `systemd-run --user --scope -p MemoryMax=8G -p MemorySwapMax=0 -p TasksMax=8192 podman run --rm --cgroups=disabled --network none …`, with no `--memory`, `--memory-swap` or `--pids-limit` flag.
- **The runner** (`img.sh`, `1a1265c` and `5695882`):
  - It reads `memory.max`, `memory.swap.max` and `pids.max` inside the scope, at `/sys/fs/cgroup` plus the path in `/proc/self/cgroup`.
  - It logs them to `spike/4210/out/caps.log`, and starts no container unless swap is `0`.
  - It writes no cgroup file.
- **Every read-back of this session** (measured): `memory.max=8589934592 memory.swap.max=0 pids.max=8192`, each with its own `run-p…scope` path.
  - The container's PID showed the scope's own cgroup: `aet4210-728128-17128`, PID 728187, `/proc/728187/cgroup` = `0::/…/app.slice/run-p728128-i143337130.scope`, the path its read-back logged.
  - `TMPDIR` printed `unset` inside the container.
- **`systemd-run` expands `$VAR` in the command it runs (measured, F13).** My first check printed `TMPDIR=`, because systemd-run had already substituted the variable before the container started. The runner passes `--expand-environment=no`.
- **The sweep:** `bash spike/4210/img.sh -- python3 spike/4210/s2.py`, all 11 cases and 17 Renode boots, in 36 s.
  - Every boot exited 0 with the boot word `0x2100B007` and no `[ERROR]` line.
  - The firmware sha256s are in `out/fw.txt`, and every row in `out/s2.jsonl`.
  - The table below is `out/s2-table.md`, which `s2_read.py` generates from the rows.
- **One harness fix before the sweep:** `IRQ` is read as `IRQ IsSet`, because the bare GPIO does not print a value (`d7c31c0`). No prediction was changed: the Predictions block is byte-identical to `1c193b7`'s.
- **A trial run came first.** One `imu-script` container ran at 03:35:09 (its read-back is in `caps.log`) to shake out the harness. Its three rows were deleted before the full sweep, which reran that case.

### Second session (2026-11-15): the corrected rule stops at the swap read-back (measured)

The architect's corrected kickoff rule 3 drops the cgroup write, which caused the first denial, and states its premise: "with `--memory-swap` equal to `--memory`, podman sets `memory.swap.max` to 0 itself under cgroup v2. … stop if `memory.swap.max` is not 0."
- **The runner was changed to match** (`76ff075`):
  - `img.sh` writes no cgroup file;
  - it reads `memory.max`, `memory.swap.max` and `pids.max` back;
  - it releases the command inside only if `memory.swap.max` is exactly `0`.
- **Two containers started**, each `bash spike/4210/img.sh -- renode --version`, a plain call. The classifier allowed both. From `spike/4210/out/caps.log`:
  - `aet4210-265274-14998` read once, at the instant the file appeared: `memory.swap.max=8589934592`.
  - **`aet4210-318423-22996`** (runner `c145906`) sampled every 0.25 s for 40 s. It logged one value, unchanged throughout: `memory.max=8589934592 memory.swap.max=8589934592 pids.max=max`.
- **The premise is false on Furnace (measured).** Podman 5.7.0 (systemd cgroup manager, crun) with `--memory 8g --memory-swap 8g` sets `memory.swap.max` to 8 GiB and leaves it there. So the container could use 8 GiB of RAM plus 8 GiB of swap. That is what the global contract already records (Aethelgard `2101` F11). Each command inside exited 99 having run nothing, as built.
- **1307's caps log reads 0 with the same flags; its zeros most likely came from a write (inferred).** I read `~/.cache/aet-1307/` read-only:
  - its live container `aet1307ci-240016` has the same `CreateCommand` flags and `HostConfig.MemorySwap` 8589934592, and reads `memory.swap.max=0`;
  - that file's mtime is `02:10:18.478`, 0.5 s after `memory.max` (`02:10:17.985`): the poll interval of 1307's runner;
  - 1307's `ci-img.sh` and `img.sh` were rewritten without the write at `02:16:05`, after the last line of its caps log (`02:10:18`).

  So every zero in 1307's log predates the correction. No read under the corrected rule shows 0.
- **This was the kickoff's stop.** I did not try other flags, a parent slice, or Renode on the host: each changes the cap mechanism the rule fixes. The architect ruled (D8), and the third session measured S2 and S3.

### Datasheets for Scotty (not a stop)

The hosts below refuse Furnace or no longer serve the part. Each was retried once on 2026-11-15 with a plain `curl` (no User-Agent change, no mirror).

| URL | Result on 2026-11-15 |
|---|---|
| `https://www.st.com/resource/en/datasheet/lsm6dso.pdf` | curl rc 92, no HTTP status (HTTP/2 `INTERNAL_ERROR`) |
| `https://www.st.com/resource/en/datasheet/lps25hb.pdf` | rc 92 |
| `https://www.st.com/en/mems-and-sensors/lsm6dso.html` | rc 92 |
| `https://www.st.com/en/mems-and-sensors/lps25hb.html` | rc 92 |
| `https://www.bosch-sensortec.com/media/boschsensortec/downloads/datasheets/bst-bmp180-ds000.pdf` (BMP180 datasheet) | HTTP 404 (a Bosch HTML page, 90,599 bytes) |
| `https://www.bosch-sensortec.com/en/products/environmental-sensors/pressure-sensors/bmp180` | HTTP 404 |

**If Scotty fetches them:** put them in `~/.cache/aet-4210/ds/`. They are needed only if D2 (b), the BMP180, is chosen; the ST parts are out on other grounds (S1).

### First session: where it did not run

**1. The container was denied (the kickoff's stop).** The cause, given by the architect afterwards, was the old rule 3's write into the container's `memory.swap.max`; Furnace hard-denies writes under `/sys/fs/cgroup`.
- My first container was a no-op whose only job was to prove the caps read back, at 8g. **The permission classifier denied that call, and gave no reason.** The call was one compound command:
  - `chmod +x spike/4210/img.sh`;
  - then `spike/4210/img.sh -- bash -c 'echo inside; env | grep -c TMPDIR; renode --version | head -2; ls /cache'`;
  - then `cat spike/4210/out/caps.log`.

  Which part tripped the classifier was not known then. The architect later named the cause: the cgroup write.
- The kickoff says a denial is not worked around, so:
  - I did not retry it, split it or reword it;
  - I did not hand it to a subagent;
  - I did not run Renode on the host.
- **No container started** for this work order. `spike/4210/out/` was never created.
- **So in this session every S2 and S3 row was *read* or *inferred*** (the third session measured them). The harness that would measure them is committed, **unrun and untested**:
  - its exec bit was never set;
  - the firmware variants were never compiled, because the host has no `thumbv7em` target and only the image does;
  - so its first run may need fixes.

  Its pieces:
  - `aet-4210-gen` (built and run on the host under the capped scope; it writes `~/.cache/aet-4210/boards/spi-imu`);
  - `img.sh`, `fw.py`, `s2.py`, and `drive.py` (4200's, with only names and bounds changed).

  One command runs it all: `bash spike/4210/img.sh -- python3 spike/4210/s2.py`. Each case names the prediction it tests.

**2. st.com refuses Furnace.** In the first session this was read as §7's second condition and a stop. The goal for the second session rules it is not a stop. The URLs and the retry are in "Datasheets for Scotty" above.
- A browser-like User-Agent was tried once by the first session's subagent. The classifier denied it, and nobody tried again.
- **Effect on the recommendation: none (inferred).** Neither ST part can be chosen on other grounds: no ArduPilot driver accepts the LSM6DSO, and the LPS25HB model writes no Noisy line at all (below).

**Bosch's BMP180 page and datasheet are 404s** (listed above for Scotty). The BMP180 is absent from Bosch's current pressure list, which names the BMP384, BMP390, BMP580, BMP581 and BMP585. No BMP180 datasheet is held, so no BMP180 value can be cited as the parts table requires (`4200` §2.22).

### S1 — the parts, by evidence (read; `s1-models.md`, `s1-ardupilot.md`, `~/.cache/aet-4210/ds/REPORT.md`)

Each subagent's claims that the recommendation rests on were spot-checked by me at the cited lines, and the hashes recomputed.
- **Renode** at `066a7f13`.
- **ArduPilot** at `Copter-4.7.2` (`fb773567`), which `ArduCopter-stable` also points at. That is `4400`'s clone's commit too (F9).

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
- Its whole script can be witnessed at Noisy, and the value written is in the line (measured: P1.1–P1.5).
- **Its costs:**
  - TDK lists it **EOL**;
  - its `VDDIO` tops out at 1.95 V;
  - the model's FIFO is not implemented, so ArduPilot's own read path delivers nothing in the twin. The bring-up script uses the data registers instead.

**Barometer: the ICP-10111, if the architect accepts Debug-level witness lines for this model (D2). Otherwise none qualifies.**
- **What it has:**
  - the datasheet is held;
  - ArduPilot drives it;
  - one fixed address;
  - its whole bring-up logs one line per step, with the bytes, at Debug. Its measurement read also logs 2 Noisy lines (measured: P6.2).
- **Its costs:**
  - **EOL**;
  - **1.8 V only**, the IMU's domain;
  - a 16-bit identity command, so `identity.write` becomes a byte list;
  - an identity value that no held document states. ArduPilot says `0x08`, and Renode answers `00 08`.
- **The alternatives:**
  - **The BMP180** witnesses every step at Noisy, with no device warning (measured: P6.3), and runs at 3.3 V, but no vendor datasheet can be held, so its values cannot be cited.
  - **The LPS25HB** cannot be witnessed at all: no Noisy line in a whole script (measured: P6.1).

**The 1.8 V domain.**
- `4300d` as split (`4300` FEEDBACK §S7) adds buck, LDO and sense rows to the parts table. It can therefore declare a 1.8 V LDO rail. A `voltage_domains` entry of 1800 mV is already expressible today (inferred).
- **What neither `4300d` nor the specification can say:**
  - that a device's I/O domain differs from its supply (the ICM-20948's `VDD` against its `VDDIO`);
  - that a bus crosses from the MCU's 3.3 V pins to 1.8 V parts through a level translator.

  The parts table's `supply_millivolts` still has no consumer (`4200` F19). So **`4300d` can declare the domain, but not the crossing**, and this is a decision (D7).

**What all this means for `4500` (the larger decision, D1).** Every part that the twin models and ArduPilot drives, and that this spike could hold a datasheet for, is EOL. Sentinel-02 is the first board that is bought. §7's first condition is not met as worded: the ICM-20948 qualifies, and so do the ICP-10111 (with D2) and the BMP180 (without a citation). But a purchase of EOL parts is close to it. A model of our own, for example an ICM-42688-P and a BMP390, is the ruling `4500` was to make. **It may be worth making now**, before the build puts EOL parts into `parts.json`.

### S2 — the transaction script (proposal; measured on the stand-ins)

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
- **The data read's value (D3).** The twin sets a stimulus, for example `DefaultAccelerationZ 1`. The spike measured it set by a monitor command before `RunFor` (`s2.py` `STIM`). Setting it in the emitted platform instead is unmeasured. The judge expects `round(g × sensitivity)` with the datasheet's sensitivity, cited (P1.5). That is the judge's stimulus and the datasheet's scale, never the firmware's output (`INV-07`). The weaker option is to match the line's shape with any value. That proves a read happened, not what it returned.
- **Order (2204 A4).** Each device's steps are judged in order, by device, not globally across devices. So `W U2, W U5, R U2, R U5` passes. Extra traffic to a device beyond its script still fails, as today.
- **Per-read windows on SPI (F16).**
  - A window is the multiplexer's own `GPIO #k set to False` … `GPIO #k set to True` (`SPIMultiplexer.cs` L30, Noisy), `k` being the device's address. Each step's transfer lines must lie inside one window of its own select.
  - A `not connected` line is attributed to the window it falls in, so `E2` no longer depends on the address alone.
  - The edges can be forged, but a forged edge only adds an empty window (P2.1 measured it; P3.3). Windows attribute lines; they never witness.
  - **Measured (P3.1):** in 3 runs of interleaved U5/U4/U5/U4 steps, all 46 transfer lines lay inside their own select's window, and the two selects' windows never overlapped.
  - **Measured (P3.2):** with U4 absent, all 12 `Tried to transmit … not connected` lines fell inside `GPIO #0` windows.
    - The multiplexer also warns `Tried to finish transmission to device 0x0, but it's not connected` on every edge, 3 of them inside U5's windows.
    - `unrouted_line` already ignores those, and the window rule must keep doing so (F11).
- **The event bound (F18), measured.** Kinds 13 and 22 become one event per step, not per read. The correct ICM run's step sizes (log bodies, all from U5):
  - identity: 4 lines, 261 bytes;
  - configuration write: 3 Noisy lines, 195 bytes. With its Debug and Warning lines it is 5 lines, 394 bytes;
  - 6-byte data read: 14 lines, 1,006 bytes, a mean of 71.9 bytes and a longest line of 87.

  So a step reading `n` bytes logs `2 + 2n` lines (measured for n = 1 and 6). At the longest measured line, `(2 + 2n) × 87 ≤ 1,800` gives **`n ≤ 9`**. That cap is inferred, not measured: it assumes kind 22's 1,800 bytes and an event that stores each whole log body. The build sets it from the event format. A 14-byte ArduPilot-sized burst does not fit. Kind 13 takes kind 22's per-step budget, which closes F18.
- **Device warnings (F22, D4), measured.** A correct configuration write warns: `CLKSEL`, exactly as predicted (P1.3). `I2C_IF_DIS` was not scripted here (read only). So "any device warning fails" would fail a correct script.
  - **A second source of device warnings (measured, P1.6; F11).** `4200` recorded the flash's idle-finish warning on `PA0` and `PA1` edges (`AET-WO-4200.FEEDBACK.md` L72, L319–326). This spike measures how far it reaches:
    - on every chip-select edge, the multiplexer finishes every deselected device (`SPIMultiplexer.cs` `UpdateChipSelectState`);
    - the flash model warns `Transmission finished in the unexpected state: RecognizeOperation` whenever it is finished idle (`GenericSpiFlash.cs` L95);
    - so in a correct run, **U4 warns 8 times on edges that are not its own transfers**.

    A device-warning rule that judges U4 would fail U4 for U5's script. Sentinel-01's firmware reads U4 (`boards/sentinel-01/firmware/src/main.rs` L153).
  - **The correct ICM run's warnings, every one:** U5's `CLKSEL` (1); U4's idle finish (8); the multiplexer's `Tried to finish transmission … not connected`, logged with no source name during platform load, before `Machine started` and before the devices were attached (3; raw log L6–8); and the controllers' own tag warnings, `i2c1` (2) and `gpioPortB` (2). No barometer model warned in any case.
  - **Proposal:**
    - each step declares its expected warnings exactly, pinned to source;
    - the flash's idle-finish warning is pinned as an edge artefact: allowed only directly after a multiplexer `Finisihing transmission on device 0x0` line that an edge produced;
    - any other `[WARNING]` from a scripted device fails the step.
  - **The alternative:** script only writes that touch no tag. `PWR_MGMT_1` = `0x00` wakes the silicon (`0x41` → `0x00`, internal oscillator) and warns nothing in the model, but it is not ArduPilot's value.

### S3 — interrupts (measured)

- **The ICM-20948 model never raises data-ready by itself** (P4.1, measured):
  - with `INT_ENABLE_1` = 1 and a data read, `IRQ IsSet` read `False` after the run, and `INT_STATUS_1` read `0x0`;
  - `RAW_DATA_0_RDY_INT` is set by nothing but a firmware write of `INT_STATUS_1`;
  - `DATA_RDY_STATUS` is a tag that reads 0.
- **An interrupt the judge learned from this model would witness a firmware store, not the sensor** (P4.2, measured): the firmware wrote `INT_STATUS_1` = 1 over SPI, the model logged `IRQ set` (Debug), and `IsSet` read `True`.
- Each write to an interrupt register also logs a Noisy `Interrupts: DMP False, … Raw Data Ready …` line. A script step writing `INT_ENABLE_1` is therefore 4 Noisy lines, not 3.
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

Every row is **measured** except P3.3 and P6.4 (as marked) and P5 (read). Generated by `spike/4210/s2_read.py` from `spike/4210/out/s2.jsonl`; the predictions are unchanged above.

| Prediction | Result | Measured |
|---|---|---|
| P1.1 | held | identity window: 4 U5 lines, equal to the 4 `model.rs` expects |
| P1.2 | held | 3 Noisy lines as predicted; the value only in `Write at bank #0 to 0x6 (PowerManagement1): value 0x1` (Debug) and the second `Received byte`. Order: Preparing, Received 0x6, Debug, Warning, Received 0x1 |
| P1.3 | held | U5 warns 1 time(s) in the run, inside the configuration window: `Unhandled write to offset 0x6. Unhandled bits: [0] when writing value 0x1. Tags: CLKSEL (0x1).` |
| P1.4 | held | 14 Noisy lines (predicted 14); bytes ['0x0', '0x0', '0x0', '0x0', '0x0', '0x0']; registers 0x2D–0x32 named |
| P1.5 | held | with `DefaultAccelerationZ 1`: AccelerationXOutHigh 0x0, AccelerationXOutLow 0x0, AccelerationYOutHigh 0x0, AccelerationYOutLow 0x0, AccelerationZOutHigh 0x40, AccelerationZOutLow 0x0 |
| P1.6 | failed in part | U5's lines outside its steps: {'Finishing transmission, going idle': 6}, by the edge that preceded each: {'GPIO #1 True': 4, 'GPIO #0 False': 1, 'GPIO #0 True': 1}. On every edge of any select, the multiplexer finishes every deselected device (`SPIMultiplexer.cs` `UpdateChipSelectState`), so U5 also goes idle on U4's edges. U5 warns only P1.3's line, but **U4 warns 8 times** in a correct run (`Transmission finished in the unexpected state: RecognizeOperation`, `GenericSpiFlash.cs` L95): it is finished while idle on edges of either select |
| P1.7 | held | 3 runs: kept lines and reads byte-identical: True |
| P2.1 | held | 2 U5 windows: the forged one holds 0 U5 lines; `GPIO #1` edges and U5's idle line are logged; no `Preparing for write`. Records still claim `0x601`: 0x00000601 |
| P2.2 | held | 2 U5 windows; the configuration step's 3 lines present; no read of 0x2D; record 4 claims 0x22020000/0x00000000 |
| P3.1 | held | 3 runs, 46 transfer lines each: 0 outside their own select's window or inside another's. The two selects' windows never overlap |
| P3.2 | held for the transmit lines; a second kind found | `Tried to transmit … to device 0x0, but it's not connected`: {'#0': 12} (inside `GPIO #0` windows only). But the multiplexer also logs `Tried to finish transmission to device 0x0, but it's not connected` (Warning) on every edge: {'none': 8, '#1': 3} by the window open when logged. The judge's `unrouted_line` takes only `Tried to transmit byte` (`renode.rs` L529), so these do not reach `E2` today |
| P3.3 | consistent (inferred) | P2.1's forged window is empty: forged edges add an empty window and move no transfer line. Windows attribute; they do not witness |
| P4.1 | held | `IRQ IsSet` after the run: False; `INT_STATUS_1` read: 0x00000000. The `INT_ENABLE_1` write logs `IRQ unset` (Debug) and an `Interrupts: … Raw Data Ready False …` Noisy line |
| P4.2 | held | after the firmware writes `INT_STATUS_1` = 1: `IRQ set` (Debug), `IsSet` True, status read 0x00000001 |
| P6.1 | held | identity 0x000000BD, data 0x00000000; device lines: ['[DEBUG] i2c1.baro: Writing 0x90 to register 0x20']; Noisy: 0; device warnings 0 |
| P6.2 | held for identity; failed in part for data | identity 0x00000838 (`00 08` + CRC `0x38`). Debug lines: 4, one per command and one per read, bytes in the line. **The measurement read also logs 2 Noisy lines** (`A=… B=… C=…`, `p_raw=…`), so a data step is visible at Noisy, but not its identity. Device warnings 0 |
| P6.3 | held but for the data step's count | Noisy lines per step: [4, 4, 7] (predicted 4, 4, 5: the 2-byte read logs `Write`, `Preparing`, `Reading`, then a `Read value` and an `Auto-incrementing` line per byte, 7). Values decimal (`Read value 85`, `67`, `79`); register hex 8 digits. Device warnings 0. 3 runs identical: True |
| P6.4 | held (read) | none of the three barometer models has a GPIO; no barometer case logged an IRQ line |
| P5 | read, not run (ArduPilot is not in the twin) | as read. One correction: the model's FIFO is tags, not merely unknown |

Step sizes in the correct ICM run (log bodies without the host timestamp; the judge's events are built from these):

| Step | U5 lines (all levels) | Noisy | bytes, all | bytes, Noisy |
|---|---|---|---|---|
| identity | 4 | 4 | 261 | 261 |
| configuration write | 5 | 3 | 394 | 195 |
| data read, 6 bytes | 14 | 14 | 1006 | 1006 |

Longest Noisy line of the data read: 87 bytes; mean 71.9.

Every [WARNING] of the correct ICM run (`imu-script`, run 0), by source:

- 1 × `(no source): Tried to finish transmission to device 0x0, but it's not connected`
- 2 × `(no source): Tried to finish transmission to device 0x1, but it's not connected`
- 1 × `gpioPortB: Unhandled write to offset 0x4. Unhandled bits: [6] when writing value 0x40. Tags: OT6 (0x1).`
- 1 × `gpioPortB: Unhandled write to offset 0x4. Unhandled bits: [7] when writing value 0x80. Tags: OT7 (0x1).`
- 1 × `i2c1: Unhandled write to offset 0x10. Unhandled bits: [0-1, 4-6, 8, 13, 21-22] when writing value 0x602173. Tags: SCLL (0x73), SCLH (0x21), SCLDEL (0x6).`
- 1 × `i2c1: Unhandled write to offset 0x1C. Unhandled bits: [4] when writing value 0x30. Tags: NACKCF (0x1).`
- 8 × `spi1.spi1mux.u4: Transmission finished in the unexpected state: RecognizeOperation`
- 1 × `spi1.spi1mux.u5: Unhandled write to offset 0x6. Unhandled bits: [0] when writing value 0x1. Tags: CLKSEL (0x1).`

## §7 checked

- **No modelled IMU or barometer both drivable and witnessable beyond identity:** not met as worded (measured).
  - The IMU qualifies: the ICM-20948.
  - A barometer qualifies only with a ruling: the ICP-10111 at Debug, or the BMP180 without a citation.
  - Every candidate is EOL or out of catalogue (D1).
- **A vendor host refuses Furnace:** met by st.com, and Bosch serves 404s. Ruled not a stop for this spike; the URLs are listed for Scotty.
- **A tool call denied by the permission classifier** (the kickoff's stop): met in the first session (the cgroup write), and not met in the second.
- **`memory.swap.max` not 0:** met in the second session (F10), under the rule since replaced. Under `RULE3.md` every read-back was 0.

## Held, by full sha256 (`~/.cache/aet-4210/ds/`)

| File | sha256 |
|---|---|
| ICM-20948 DS-000189 1.6 (`icm20948_new.pdf`, `icm20948_held.pdf`) | `0588625f4f1d6d27382a4322f48073dc0ce569e537c63786b13aab3eeb76b718` |
| ICP-10101 DS-000408 1.2 | `5f1dddc3f7c59453374d8aafb727cd10326472565f0d6001626eaa5493198f49` |
| ICP-10111 DS-000177 1.3 | `249f1431a27dc70d39be1dfd39ff36429d1f66bdea115178ed05e3e3275a2d58` |
| ICP-10125 DS-000329 1.1 | `27947486a648cb734da8cb982e4331981aff997440f8584ad3a8157c6b69657e` |
| TDK ICM-20948 product page (EOL) | `694cc3e2bc2871c94f65735a7057c8280ba74cb02ea820f9e588aa810a1eb284` |
| TDK ICP-10101 page (EOL) | `be6c588ef20c83ac746d9f22e14ed314082ba4646d1e23e7b3e0d316cf0ad65a` |
| TDK ICP-10111 page (EOL) | `fdc2178341ce7da92d79d10901d8f5163a541cf585b9133f9097d20673fe7a57` |
| TDK ICP-10125 page (EOL) | `29ff151405afcb876bc2f1ed3923b9ab534fd24348b27601d3a9b7cda65c4b67` |
| TDK pressure family page ("EOL and not recommended for new designs") | `0b307a68fe91c773412c177db37d25662b711ded852e7bfc4af2ed8a4fc51865` |

## Findings

- **F1 · The first container was denied by the permission classifier.** The cause, per the architect, was the old rule 3's write into `memory.swap.max`. It was removed from the runner (`76ff075`), and the second session's container was allowed.
- **F2 · st.com refuses Furnace at the transport** (HTTP/2 `INTERNAL_ERROR`, rc 92; retried 2026-11-15). Nothing ST-made can be held from this box.
- **F3 · The ICM-20948 and every ICP-101xx are EOL**, on TDK's own pages (held in `~/.cache/aet-4210/ds/pages`, sha256 in REPORT.md). The BMP180 is absent from Bosch's catalogue.
- **F4 · The ICM-20948 model has no FIFO** (`FIFO_CNT` and `FIFO_R_W` are tags). ArduPilot's read path gets no samples in the twin. That matters when ArduPilot is ever in the twin, which is not this work order.
- **F5 · The silicon resets `PWR_MGMT_1` to `0x41`** (asleep), and the model resets it to 0. A firmware that never wakes the part passes in the twin and reads zeros on the bench. The script's configuration write should be the wake, so the bench and the twin agree on what was done.
- **F6 · The ICP-10111's identity value is in no held document.** Its upper 10 bits are unspecified from sensor to sensor, so an identity check must mask to bits 5:0.
- **F7 · The LPS25HB model writes no Noisy line,** and its `CTRL_REG2` write calls `Reset()` mid-buffer (census §3). It is not a candidate under any witness rule today.
- **F8 · ArduPilot.**
  - A hwdef `BARO` line forces that driver's enable (`hwdef.py` L650).
  - `BARO_PROBE_EXT` defaults to 0.
  - The ICP-101xx has no autodetect, so `4400`'s generated hwdef must name the barometer.
- **F9 · `4400`'s clone is at the same commit.** I read `Copter-4.7.2` (`fb773567`). Its clone appeared during this spike, and `~/.cache/aet-4400/ardupilot` `HEAD` is `fb773567` (read-only `rev-parse`).
- **F10 · `--memory-swap` equal to `--memory` does not zero swap on Furnace (measured). Ruled: `RULE3.md` replaces the rule.**
  - Podman 5.7.0 set the container's `memory.swap.max` to `8589934592`, the value given, and it held for 40 s of sampling (`spike/4210/out/caps.log`).
  - 1307's zeros with the same flags predate the correction, by file times (inferred; above). A note went to 1307's inbox and to the other builders whose runners take the same rule.
  - The corrected rule 3 rests on the opposite premise, so under it no Renode run can start on this box.
  - Two other routes zeroed swap before, and both are now closed:
    - the global contract's "write `0` into `memory.swap.max`" step is hard-denied by Furnace;
    - the runner's old version did exactly that, and was denied (F1).
  - The global contract (furnace777, `host/home/forge/.claude/CLAUDE.md`) still prescribes that write. It needs the same correction, by PR there.
- **F11 · The multiplexer finishes every deselected device on every chip-select edge.** It extends `4200`'s observation, L319–326 (measured P1.6, P3.2; `SPIMultiplexer.cs` `UpdateChipSelectState`). Three consequences:
  - the flash model warns on edges that are not its own transfers;
  - U5 logs its idle line on U4's edges;
  - with a select's device absent, a `Tried to finish transmission … not connected` Warning lands inside other devices' windows.

  The judge must attribute by window and pin these lines. It must not judge "all warnings of a device".
- **F12 · Two step counts differed from the predictions** (measured):
  - the BMP180's 2-byte read logs 7 Noisy lines, not 5;
  - the ICP-101xx's measurement read logs 2 Noisy computation lines.

  The scripts' line counts are set from measurement, as these were.
- **F13 · `systemd-run` expands `$VAR` in its command line** unless `--expand-environment=no` is given (systemd 259; measured). `RULE3.md`'s command form would otherwise rewrite any `$` in a container's command.
- **F14 · From 4300c's bus note (peer information, not checked here):** under `parts.json` schema 2 (PR #129, unmerged), the ICM-20948's `pin_capacitance` is null. DS-000189 1.6 §3.4 gives only a typical CI "< 10". 4300c's `extract` then refuses a null part on a bounded SPI SCK or MOSI (`UnboundedPinCapacitance`). **The build of this recommendation needs a cited maximum, or a ruling, once 4300c lands.**

## Decisions needed

- **D1 · The parts are EOL.** Accept the ICM-20948 (and D2's barometer) for the twin's path now, or make `4500`'s "model of our own" ruling now, before `parts.json` gains EOL parts.
- **D2 · The barometer.**
  - **(a)** The ICP-10111, with the judge reading that model's Debug lines, a 16-bit identity command, and the identity masked to bits 5:0 and cited to Renode's source and ArduPilot (no datasheet value). **Recommended if D1 accepts EOL parts.**
  - **(b)** The BMP180, uncited.
  - **(c)** None, until a model of our own.
- **D3 · The data read's value:** exact, from a judge-set stimulus and the datasheet's scale (recommended), or the line's shape only.
- **D4 · Device warnings:** declared exactly per step and pinned to source, with the flash's idle-finish warning pinned as an edge artefact (F11; recommended); or only tag-free writes, which does not remove F11's warnings.
- **D5 · Per-read windows:** the multiplexer's chip-select lines, attribution only (recommended; P3.1 and P3.2 measured it). `Tried to finish … not connected` stays out of `E2` (F11).
- **D6 · Interrupts:** stay polled, and adopt `2204` A2's `SysTick`/`ISER` check now (recommended), or arm one EXTI. The model gives no sensor-driven data-ready to learn from.
- **D7 · 1.8 V.** Where a device's I/O domain and a bus's level crossing are declared (`4300d`, or this build), and whether a translator is a part or a refusal.
- **D8 · How a container is held to zero swap on Furnace: ruled** (PR #131 comment `6888603447`, `RULE3.md`). S2 and S3 were measured under it.

## Stop

**AET-WO-4210 STOPPED: spike done, ruling needed on PR #131.** S1 was read; S2 and S3 were measured under `RULE3.md`. D1–D7 are for the architect.

## Gates (spike)

The fence: `git diff --name-only origin/main...HEAD` lists this file and `spike/4210/` only (`main` `5f47a5d` merged in at `0871026`).

On the host, under `systemd-run --user --scope -p MemoryMax=32G -p MemorySwapMax=0`, at `0871026`:
- `cargo fmt --all -- --check`: ok;
- `cargo clippy --workspace --all-targets -- -D warnings`: ok;
- `cargo test --workspace`: exit 0, 77 `test result: ok`, no failure.

This branch changes no crate. Tier 2 is CI's (`roadmap.md` §6). The spike's Renode runs are the third session's, under `RULE3.md`.

---

# Build (WO §2, as ruled on PR #131, comment `6888973755`)

Builder: Claude Code (Opus 5.5) in `~/aethelgard-4210`, branch `wo/aet-4210-build`, cut from `main` at `50d7a6f`. The work order was read from `origin/claude/argon-repo-docs-synthesis-rjrb0d` (docs PR #136); it is not committed here. Labels as above: **measured**, **read**, **inferred**.

## Build predictions — written before any container run

Committed alone, before any Renode, Kani or tier-2 container of this build ran. What they rest on:
- the spike's measured lines (`spike/4210/out/s2.jsonl`);
- Renode at `066a7f13`, read at the lines cited in the code;
- the code as committed at this commit.

| # | Prediction | Rests on |
|---|---|---|
| PB1 | The 1.8 V fixture (`imu-baro-1v8.json`: U2 BME280 and U6 ICP-10111 on I2C1; U5 and U7 ICM-20948 on SPI1) boots and passes with the generated firmware. 10 records in order: U2 identity; U6 identity, configure, data; U5 the same; U7 the same. U6's data reads `0x6666` (25 °C). U5's and U7's data read `0x4000` (0 g, 0 g, 1 g at 16,384 LSB/g). | S2 measured; `ICP_101xx.cs` L372-375 (truncating, 26214.4 → 26214) |
| PB2 | Each IMU's steps lie in its own chip-select windows, step k in window k. Lines per step: identity 4, configure 4 (3 Noisy plus the `CLKSEL` warning, between `Received byte 0x6` and `Received byte 0x1`), data 14. Each IMU's idle line, logged on its own edges and on the other's (F11), is skipped. No `unrouted` line. | P1.1-P1.6, P3.1 measured |
| PB3 | At `Debug`, the ICP logs exactly 2, 1 and 2 lines for its steps. The data line is `Reading data with CRC: [0x66, 0x66], current command: 0x7866 (UltraLowNoiseTemperaturePressure), requested: 2 bytes`. No ICP `[NOISY]` line prints. | `ICP_101xx.cs` L61, L75; `logLevel 0` |
| PB4 | After the run, `SysTick.CSR` reads `0x00000004` (`CLKSOURCE` alone, `NVIC.cs` L1040) and `NVIC.ISER0`-`ISER4` read `0x00000000`. So the judged bits (`0x00010003`) are 0. | `NVIC.cs` L1023-1046, L719-722 |
| PB5 | The stimulus can be set in the emitted platform: `DefaultAccelerationZ: 1` under the ICM's `.repl` entry, and `DefaultTemperature: 25` under the ICP's, both load. The monitor then reads `1` and `25` back. | Renode's `.repl` sets a writable property by name (inferred) |
| PB6 | Seeded IMU fault (U5's wake transfer deleted, its record forged OK): exit 1. The findings are exactly `spi_transaction_unwitnessed`, twice, both for U5: its configure step, which holds the data read's lines (window 1), and its data step, which holds none. U7 passes. | the window rule, `renode.rs` `attribute` |
| PB7 | Seeded barometer fault (U6's data command `0x48A3`, pressure first, not `0x7866`): exit 1 with `i2c_transaction_unwitnessed` for U6's data step (command and name differ). The 2 bytes read are pressure, not `0x6666`, so also `i2c_identity_mismatch` naming the data step. | `ICP_101xx.cs` command table |
| PB8 | Sentinel-01's tier-2 verdicts stay pass. The flash's idle warnings are each directly after `Finisihing transmission on device 0x0`, so they are skipped. Every expected witness is unchanged. | P1.6, `SPIMultiplexer.cs` L107 |
| PB9 | The existing SysTick mutant (`a_firmware_parked_in_its_fault_handler…`) now also gives `register_mismatch` on `SysTick.CSR` (`ENABLE` and `TICKINT` set). A firmware that only sets `ENABLE` (no tick interrupt) after the boot word, or enables one NVIC line, passed before this build and now fails with `register_mismatch` naming the register. | D6; `NVIC.cs` |
| PB10 | The fixture's Kani harness proves; its SPI transfer proof runs at unwind 8 (wl ≤ 2, rl ≤ 6) and takes under 400 s in the image. Each boot of the fixture takes under 10 s. | the reference's 183 s (`ci/test-weights.tsv`) |

### Predictions for Scotty's ruling on D1 (`accept_lifecycle`), written before its first container run

The architect relayed the ruling at 05:15 UTC into this builder's inbox (`~/.cache/aet-bus/4210.md`). It is the authority for this added scope: "A specification that uses any part whose lifecycle is `eol` or `nrnd` must acknowledge each one explicitly … `validate` refuses by name any unacknowledged eol/nrnd part … Every verdict, every generated file's header and the fabrication package then carry a loud banner … A fab-intended specification is no longer refused outright: it is refused unless acknowledged." The same author's next note (1307 merged at `d52c051`) matches `origin/main`.

| # | Prediction | Rests on |
|---|---|---|
| PL1 | With both stand-ins acknowledged, the fixture's `simulate --spec` verdict carries `lifecycle` with two lines, ICP10111 (U6) first, then ICM20948 (U5, U7), each naming `EOL` and TDK's product page. The emitted platform's header and every board-crate file with a header (`lib.rs`, `board.rs`, `proofs.rs`, `Cargo.toml`, `firmware/src/main.rs`, `firmware/Cargo.toml`, `firmware/.cargo/config.toml`, `firmware/link.x`) carry the same two lines. | `lifecycle_banner`, devices I2C then SPI |
| PL2 | Every Sentinel-01 verdict, board-crate file and platform is unchanged but for the specification hash. It has no `lifecycle` key, because its parts are `active`. | `skip_serializing_if`, an empty banner |
| PL3 | The fixture's earlier tier-2 results (PB1-PB10) are unchanged: only the verdict's new key and the headers differ. | — |

## Build results — after the predictions

Every container of this build ran under `~/bin/capped-run`, through `~/.cache/aet-4210/t2.sh`: a capped user scope that the container shares (`--cgroups=disabled`, `--network none`, `TMPDIR` unset), one at a time under a lock. **Every read-back:** `memory.max=8589934592 memory.swap.max=0 pids.max=8192`, each logged in `~/.cache/aet-4210/t2.log` with its own `run-p…scope` path. Crates were fetched on the host into `~/.cache/aet-4210/cargo-home`, then built offline in the image.

| Prediction | Result | Measured |
|---|---|---|
| PB1 | held | 10 records in the predicted order. U6 data `0x6666`; U5 and U7 data `0x4000`; identities `0x60`, `0x8` (mask `0x3f`), `0xea`. Exit 0 (`the_imu_and_barometer_fixture_runs_every_script_step_witnessed`) |
| PB2 | held | SPI witnesses 4, 4, 14, 4, 4, 14 lines, each observed equal to expected. No unrouted line |
| PB3 | held | ICP at `Debug`: 2, 1 and 2 lines. Data line `Reading data with CRC: [0x66, 0x66], current command: 0x7866 (UltraLowNoiseTemperaturePressure), requested: 2 bytes` |
| PB4 | held | `SysTick.CSR` observed `0x00000004`; `NVIC.ISER0`-`4` `0x00000000` |
| PB5 | held | A `.repl` with `DefaultAccelerationZ: 1` and `DefaultTemperature: 25` loaded. The monitor read back `1`, `0` (X, unset) and `25` (`~/.cache/aet-4210/stim/stim.repl`) |
| PB6 | held | exactly 2 × `spi_transaction_unwitnessed` for U5: its configure step holds the data read's 14 lines, its data step none. U7 untouched. **And F5 measured:** U5's data read `0x4000` though it was never woken |
| PB7 | held | `i2c_identity_mismatch` (U6's data step read `0x303d`, not `0x6666`) and `i2c_transaction_unwitnessed` (U6's data step logged `0x48A3 (NormalPressureTemperature)`) |
| PB8 | held | Sentinel-01's tier-2 boots pass (`twin_tier2`, 28 tests, run 2). The flash's idle warnings are skipped as edge artefacts |
| PB9 | held | The fault-parked SysTick mutant: `core_in_exception` and `register_mismatch` on `SysTick.CSR`. `systick-counting` (CSR = 1) and `nvic-line` (ISER0 = 1): `register_mismatch` alone, each naming its register |
| PB10 | held | The fixture's `spi1_transfer_is_bounded_and_ordered` (wl ≤ 2, rl ≤ 6, unwind 8) proved in 10.67 s; `i2c1_transfer_is_bounded_and_ordered` in 5.22 s. Fixture boots: 2.9-3.1 s each |

## Adversarial pass (a fresh subagent, Opus: judgement over gate logic)

The adversary was not shown the builder's reasoning. It attacked `fa01450`, then the `accept_lifecycle` delta (`fa01450..908fce4`). Its reproduction tests are on its own worktree's detached HEAD `53b8051` (`~/aethelgard/.claude/worktrees/agent-a438bc9390ce88630`), not in this branch. It ran host-only; no container. Every claim below that the builder adopted was checked by the builder at the code; the fixes are `e9ee98c`.

| id | severity | finding | disposition |
|---|---|---|---|
| A1 | medium | A scripted device's `[ERROR]` and `[INFO]` lines are not witness lines (D4 names `[WARNING]`), so the step's witness does not see them. | **Held at run level (read by the builder):** `judge_boot` fails any run whose log holds an `[ERROR]` line before `BEGIN` (`renode_error_log`, `renode.rs`), so the ICP's `Unexpected write with no data.` (L53) and `Attempted read, but Command not specified.` (L70) fail the run, though the finding does not name the step. No model in the table logs `[INFO]` per transaction. **Decision D-A1** below. |
| A2 | low | `EolPartForFabrication` could never fire (the KiCad emitter refuses the stand-ins first). | Superseded: Scotty's ruling withdrew it, and the builder moved the rule into `validate`. |
| A3 | low | The banner and `fab.lifecycle_unheld` are in the verdict that names the package, not in `FAB-NOTES.md` or `MANIFEST.json`; a comment said otherwise. | Comment corrected. **Finding F-B3:** no board with an `eol`/`nrnd` part can be drawn today (D-B2), so the package side is unreachable. `lifecycle_unheld` (the W25Q64JV) reaches only the verdict's `fab` key, which `verdict_sha256` excludes. |
| A4 | low | `cargo test --workspace` was red at `fa01450` (a stale digest). | Fixed at `908fce4`. |
| A5 | low | A channel's conversion overflowed 64 bits and panicked in debug builds. | **Fixed:** 128-bit arithmetic, one function for both uses. Test `an_overflowing_channel_or_an_active_stand_in_is_refused`. |
| A6 | low | A stand-in marked `active` parsed, so it needed no acknowledgement and carried no banner. | **Fixed:** a stand-in must be `eol` or `nrnd`. Same test. |
| A7 | low, fails safe | The parser accepts a script on a model with no script format, and a data step over `SPI_MAX_FRAMES`. Either ends `witness_unavailable` or an invalid record. | Finding F-B4: the parts parser cannot know the twin's formats or the board crate's frame limit without a dependency it must not have (`INV-01`). Fails closed. |
| A8 | low, latent | D3 says `round`; Renode's ICP model truncates. At 25 °C both give 26214; at 30 °C they differ by one. | **Decision D-A8** below. The fixture's stimulus agrees with both. |
| A9 | low, latent | The I2C transfer proof stayed at 2 bytes each way. | **Fixed:** derived from the bus's scripts, as SPI's is. Sentinel-01 and the fixture are both (2, 2), so their text is unchanged. |
| A10 | low | evolve's report fields are 32 bits, so a 6-byte read shows truncated in S1/S2 text. Selection uses the full 64 bits. | Finding F-B5 (report text only). |
| A11 | low | A2 is judged at the end of the run: SysTick armed then disarmed, or ISER5-15 (Renode implements them; the part has 150 lines), passes. | Residual of D6's "at the end", recorded. |
| D-B1 | low | The banner is not in either `Cargo.lock`. | Recorded: those are Cargo's own format, rewritten by Cargo; every other generated file carries the banner (measured, `the_imu_and_barometer_fixture_runs_every_script_step_witnessed`). |
| D-B2 | low | `generate` and `verify` refuse the acknowledged fixture with `UnknownPart` before writing, so the banner in `generate`'s verdict, `verify`'s verdict and a package is unreachable from the CLI. | Recorded: KiCad has no symbol for the stand-ins (vendoring is `4300f`'s). Measured surfaces: `validate`, `simulate --spec`, `check speeds`, the board crate's and platform's headers. |
| D-B3 | low | `campaign`'s lines carried no banner. | **Fixed:** each scenario line carries its boot's banner. |

**Attacks that held**, by the adversary's measurement unless marked read:
- SPI windows: these each fail, never pass:
  - all three steps in one window;
  - an empty first window;
  - another address's edges;
  - configure and data swapped;
  - lines before any window.
- A device warning inside a step fails it, on I2C and SPI.
- `fit()` never cuts a failing observation to the expected lines. Reference lengths 1-51 were swept; the largest event was 4,102 bytes before fitting.
- Every spec-reading subcommand (`validate`, `solve`, `emit`, `generate`, `check speeds`, `extract`, `simulate --spec`, `campaign`, `verify`) refuses the unacknowledged fixture with `LifecycleNotAccepted` naming U6.
- Sentinel-01 is unchanged but its specification-hash lines.
- Read:
  - a value with bits above its width, or a forged high word, fails;
  - evolve's `read_matches` agrees with the judge;
  - masks are refused when too wide, zero, or leaving value bits unfixed;
  - a part name in the wrong case is refused first;
  - `Tried to finish … not connected` stays out of the evidence;
  - the flash's idle warning is skipped only after its own address's finish line;
  - replay joins on (reference, step);
  - the IMU bus's Kani bounds are (2, 6);
  - `irq_max` is 149;
  - the changed tests are faithful updates, not weakened.

The adversary reported five tool calls the permission classifier refused in its own worktree (a heredoc append, a chained checkout, a `git log` in this worktree, a `HOME` override, a foreground sleep). Each was done another way or not at all; none touched this worktree.

## Build decisions for the architect (forks taken; each reversible)

- **D-B1 · SysTick's `CLKSOURCE`** (read, `NVIC.cs` L1040; measured PB4). The ruling says "`SysTick CSR` = 0". Renode reads `CLKSOURCE` as 1 whatever the firmware writes, so the whole word is `0x00000004` on every boot, and the literal check would fail every board. The judge checks `ENABLE`, `TICKINT` and `COUNTFLAG` (mask `0x00010003`) = 0. That keeps the ruling's intent: no SysTick armed, and none that counted to 0. Say if the whole word is meant; then the check needs Renode's value, not 0.
- **D-B2 · The stimulus** (D3; measured PB5). The emitted platform *can* set it (`DefaultAccelerationZ: 1`, `DefaultTemperature: 25` load and read back). The build keeps the judge's monitor command before `RunFor` anyway. `simulate --platform` takes a caller's platform, and the judge's own act holds whatever platform is passed: the expected value and the stimulus both come from the parts table, never from the platform. Moving it into the emitter is two lines in `platform.rs`, if preferred.
- **D-B3 · The fixture is honest at 1.8 V** (D7). The whole board, MCU included, runs on a 1.8 V rail (STM32H753 `VDD` 1.62-3.6 V), so no bus crosses a level and no translator is needed. The specification cannot declare a crossing, which stays `4300d`'s. The flash (2.7-3.6 V) cannot sit on it. So the F11/F16 cross-device windows are exercised by two ICM-20948s on one bus, a common redundant-IMU layout, and the flash's edge artefact by Sentinel-01.
- **D-B4 · The data reads.**
  - The ICM-20948 reads `ACCEL_XOUT_H`-`ZOUT_L` (6 bytes) after the wake. The stimulus is 0, 0, 1 g, so 16,384 LSB/g gives `0x4000`. The datasheet does not state two's complement, so the stimuli stay non-negative, where signed and unsigned agree.
  - The ICP-10111 soft-resets (`0x805D`), then measures with ArduPilot's command, ULN temperature first (`0x7866`, `AP_Baro_ICP101XX.cpp` L44, L93). It reads 2 bytes, which the datasheet allows (§5.5: NACK after any byte). Its expected value is the datasheet's conversion (§5.9, sample code §5.11) of 25 °C, `0x6666`.
  - The ICP's pressure words come from Renode's own calibration constants, not a datasheet scale, so the build does not judge pressure.
- **D-A1 · Device `[ERROR]` lines.** Should a scripted device's `[ERROR]` fail its step by name as well as the run (`renode_error_log` today)? Either is a few lines.
- **D-A8 · Rounding.** D3 says `round(g × sensitivity)`; the parts table rounds half away from zero. Renode's ICP truncates, so a stimulus whose conversion has a fraction of ½ or more expects one more than the model answers (30 °C: 28087 against 28086). Rule: keep `round` and choose stimuli the model agrees on (today's), or match each model's rounding in the twin.
- **D-B5 · Specification version 5.**
  - The ruling's `accept_lifecycle` is a required key, so the specification moved up a version, as `board` (v2) and 4300c (v3) did.
  - `4300f` merged version 4 (`connectors`) first, so by the numbering rule this build took **5** in its merge of `main`. `4300a` takes the next free one, and its inbox has the note.
  - The bump moved only Sentinel-01's board-crate specification-hash lines, the pinned solve and order digests, and the twin's reference hash.
- **D-B6 · Refusals the ruling did not name:** an acknowledgement given twice, one for a part the board does not use as `eol`/`nrnd`, and a statement that is empty, whitespace, non-printable or over 512 bytes. The stand-in rule (A6) makes a stand-in always need an acknowledgement.

## Findings (build)

- **F-B1 · Renode accepts the stimulus in the platform** (PB5, measured). See D-B2.
- **F-B2 · F5 measured at the twin:** U5's data read answered `0x4000` though it was never woken (PB6). Only the witness catches a missing wake; the value cannot.
- **F-B3 · The fabrication package carries no lifecycle line** (A3). The banner and `lifecycle_unheld` are in the verdict, outside `verdict_sha256`'s bytes (the `fab` key is excluded). For a package with an `eol` part, the line belongs in `FAB-NOTES.md`, which is `aethelgard-eda`'s. Unreachable until a stand-in or an EOL part can be drawn.
- **F-B4 · The parts parser cannot refuse a script no model can witness, or one over the board crate's frame limit** (A7). Either fails closed at the judge.
- **F-B5 · evolve's S1/S2 report text truncates a 6-byte value to 32 bits** (A10). Selection is unaffected.
- **F-B6 · The W25Q64JV's lifecycle is not held.** Winbond's catalog page (`https://www.winbond.com/hq/new-online-purchasing-guide/?__locale=en&pLine=/product/code-storage-flash/qspi-nor/&pNo=W25Q64JV`) loads the status by script; one plain POST to its endpoint returned 4 bytes. It is cited `pending`, and `fab.lifecycle_unheld` names it. **For Scotty:** the page as a browser shows it, saved into `~/.cache/aet-4210/ds/pages/`.
- **F-B7 · The BME280's `active` rests on the Bosch page listing it with order links and no discontinuation notice.** No status word is printed (held `974f1fe8…`).
- **F-B8 · `main`'s `engine-image` workflow is red at `50d7a6f`** (its quickstart exits 2; run 43676073419). That is not `ci`. The cause is not investigated here: it is outside the fence.
- **F-B9 · The droop probe compares a masked first identity as a whole word.** It can fail a probe, never pass one. Today's droops are Sentinel-01's, which is unmasked.
- **F-B10 · A data read mismatch is reported as `{bus}_identity_mismatch`, naming the step.** A kind of its own would need evolve's rule table: a naming question for the architect.

## Closed by this build

- `2204` A2 (D6): `SysTick.CSR` and `NVIC.ISER0`-`4` are judged.
- `2204` A4: per-device step order; the cross-device check is removed.
- `4200` F16: per-read windows.
- `4200` F18: kinds 13 and 22 fitted to the event bound, one per step.
- `4200` F22: device warnings judged per step, declared or failing.
- `4200` F19: `IoOutsidePart` consumes the I/O range.

## Gates (build)

Run at the merge `4b3c73a` (main `091003f`, with `4300f`, `4220`, `4300d` and `1307` merged in), each on the host under `systemd-run --user --scope -p MemoryMax=24G -p MemorySwapMax=0`:
- `cargo fmt --all -- --check`: ok.
- `cargo clippy --workspace --all-targets --features aethelgard-cli/with-tools -- -D warnings`: ok.
- `cargo test --workspace`: 82 `test result: ok`, no failure. `AET-WO-1307`'s two `renamings_` routing tests (140 s each) were skipped locally; CI runs them.

Tier 2, in the pinned image under `capped-run` (read-backs in `~/.cache/aet-4210/t2.log`):
- The full suite ran at `908fce4`. Its five failures were each fixed and the five tests rerun green at `8c54b9f`:
  - the two `attribute_seeded` gates (the seeded set's evidence indices moved +6 past the A2 registers, same rule and domain);
  - the SPI-first droop (the probe now skips the flash's edge artefact through the judge's own filter);
  - the reference boot's JSON (the `step` key);
  - the fab package's specification-hash prefix.
- `twin_tier2` ran green in full (28 tests).
- CI is the gate for the merged tree.

`TIER2_MIN`: 98 on `main` plus 4 (in `twin_tier2.rs`, minus 1 and plus 4; in `board_tier2.rs`, plus 1), so **102**.

Reduction pass (diff): −11 net · 2 duplicates removed · 0 dead paths removed · 2 protected kept · 3 findings below. This was a fresh subagent's pass, `5e1f83e`:
- **Removed:**
  - the read-width mask, now one copy (`aethelgard_ir::parts::width_mask`);
  - the I2C-then-SPI device chain, now `SolvedTopology::parts_in_order`.
- **Kept, protected:** the judge's record layout and status codes (`boot.rs`), and the generator's copy of the value-word count (`firmware.rs`), now labelled `belt-and-suspenders:` (INV-07).
- **Its two uncaught mutants now each fail a test** (`438d423`):
  - drop the SPI devices from `parts_in_order`: `speeds_tier1`'s fixture test;
  - set width 0's mask to all ones: twin's `a_two_byte_identity…`.
- **Findings:**
  - evolve keeps a third copy of the width mask, because it depends on neither ir nor twin;
  - the parser's channel rounding was already one function;
  - the generator's copy of the value-word count is deliberate.

## CI and the final state

- **PR #143, head `45c3e2e`.**
  - `ci` run 43687552089: success.
  - `engine-image` run 43687552065: success.
- **Tools shards, in minutes,** each under the 20-minute limit: 12.2 (0), 17.5 (1), 11.6 (2), 11.4 (3), 14.5 (4), 15.5 (5), 13.1 (6), 12.2 (7).
- **The canary:** `tier2_tests_passed=102 floor=102`, so `TIER2_MIN` is exact. `tools_total_passed=1487`.
- `main` (`091003f`) is merged in, and nothing landed after it.

**AET-WO-4210 READY FOR REVIEW: PR #143.** The build follows the work order §2 as ruled on PR #131, and Scotty's ruling on D1 (`accept_lifecycle`). It had a full adversarial pass by a fresh subagent and a reduction pass, and its gates and CI are green. The decisions for the architect are D-A1, D-A8 and D-B1 to D-B6 above.
