# AET-WO-4220 — FEEDBACK (spike)

Builder: Claude Code (Opus 5.5) in `~/aethelgard-4220`, branch `wo/aet-4220-own-models`, cut from `main` at `08d6206`. The work order was read from `origin/claude/argon-repo-docs-synthesis-rjrb0d` (docs PR #139); it is not committed here.

Labels: **measured** (a run of this work order), **read** (at source, in a held datasheet, or on a held vendor page, cited), **inferred** (reasoned from what was read, not run), **recall** (from memory, not yet checked: predictions only).

## Predictions — written before any run

Committed alone, before any container started and before any vendor fetch.

### What the S1 predictions rest on (read)

Renode 1.17.0, the pinned image's: `renode-infrastructure` `066a7f13`, `renode` `ab721d88` (`~/.cache/aet-2301/src`).
- **`include @x.cs` is a monitor command.** `IncludeFileCommand.cs` L57–89 dispatches by extension; `.cs` goes to `Monitor.TryCompilePlugin` (`Monitor.cs` L395–460). Several `.cs` files may be included in one command; nothing else may be.
- **`TryCompilePlugin` hashes the file's bytes with SHA-256** (L405–420) only to skip a second compile of the same bytes (`Code from file … has already been compiled. Ignoring...`). It refuses nothing: any bytes at that path are compiled.
- **`AdHocCompiler.Compile`** (`Main/Utilities/AdHocCompiler.cs`) is Roslyn in-process: C# 12, a release-mode DLL, references `typeof(object).Assembly.Location` plus `AssemblyHelper.GetAssembliesLocations()`, which handles a bundled build (`AssemblyHelper.cs` L47–60). On failure it throws `Could not compile assembly from: <paths>` with each error diagnostic; the monitor writes `Errors during compilation or loading:` before it (L455).
- **Where the DLL goes:** `TemporaryFilesManager.GetTemporaryFile()` with `.dll`, under `Path.GetTempPath()` + `renode-<pid>` (`TemporaryFilesManager.cs` L22, L32–40, L222), cleaned at exit unless cleanup is disabled. The compiled-files cache is off: the CLI's config states `compiler-cache-enabled = False` (`renode.rs` L40), and `EmulationManager.cs` L345 reads that key.
- **Under the CLI's sandbox,** `TMPDIR` and `HOME` are the call's own `workdir`, read-write for the tool (`crates/aethelgard-cli/src/run.rs` L517–518). So the DLL would land in the call's scratch (read, not measured).
- **The `.repl` cannot pull in C#.** Its `using` entries are platform descriptions (`renode/src/Renode/PlatformDescription/UsingsGraph.cs` L49–55). So the include must be a monitor command before `machine LoadPlatformDescription`.
- **Type names in a `.repl`** are tried as written, then with `Antmicro.Renode.Peripherals.` prefixed (`CreationDriver.cs` L1914–1919, L1980).
- **The interfaces a model implements:** `ISPIPeripheral` (`Transmit(byte)`, `FinishTransmission()`) and `II2CPeripheral` (`Write(byte[])`, `Read(int)`, `FinishTransmission()`), both `IPeripheral` (`Reset()`).

### P1. S1 — the pinned Renode loads our C#

The probe: `spike/4220/s1/AetProbe.cs`, two minimal classes in `Antmicro.Renode.Peripherals.Sensors`: an SPI one answering register `0x00` with `0xA5`, and an I2C one answering register `0xD0` with `0x5A`, each logging one Noisy line per byte. They replace `4210`'s `u5` (ICM-20948 at `spi1mux 1`) and `u2` (BME280 at `i2c1 0x76`) in a copy of `4210`'s emitted `spi-imu` platform. The firmware is `4210`'s `imu-script` ELF (sha256 `4896a531…`, `spike/4210/out/fw.txt`), unchanged: record 0 reads U2's `0xD0`, record 2 reads U5's `0x00`. Run in the CLI's composition (`drive.py`'s config and environment), under `~/bin/capped-run`, `--network none`.

| # | Prediction |
|---|---|
| P1.1 | **The pinned Renode compiles and loads the file offline.** `include @…/AetProbe.cs` before `LoadPlatformDescription` succeeds with no network; Roslyn is in the portable package. §7's first stop does not fire. |
| P1.2 | **Load time:** the include adds 1 to 6 s of wall time to a boot (a cold Roslyn in-process), against the same boot with no include. |
| P1.3 | **Both attach, and the unchanged firmware reads them:** record 0 holds `0x5A` (I2C, `0x76`, register `0xD0`), record 2 holds `0xA5` (SPI, `spi1mux` address 1, register `0x00`). The probe's Noisy lines appear in the log, in the order of the firmware's bytes. |
| P1.4 | **Type names:** `Sensors.AetProbeSpi` resolves (default namespace prefixed). A class in a namespace of our own (`Aethelgard.Models`) resolves only when the `.repl` writes its full name. |
| P1.5 | **A deliberate fault** (a missing `;` and an undefined type) prints `Errors during compilation or loading:` then `Could not compile assembly from: <path>` and one `<path>(line,col): error CSnnnn: …` line per error, with the file's path and position. |
| P1.6 | **After a failed include the rest of the `-e` line still runs:** `LoadPlatformDescription` then fails on the unknown type (`There was an error executing command 'machine LoadPlatformDescription …'`), and Renode exits 0 at `quit`. So a refusal (`INV-06`) must come from the CLI reading these lines, as it already does for a load error (`renode.rs` L672), not from Renode's exit code. |
| P1.7 | **The DLL** is written to `/tmp/renode-<pid>/<name>.dll` in the container (`TMPDIR` unset), one per compiled include, a few KiB to tens of KiB, and is gone after Renode exits. Nothing is written under `HOME` (cache off). |
| P1.8 | **Renode's own hash is no pin:** including a changed file at the same path compiles the changed bytes with no complaint (a different SHA-256 is just a new compile). So the pin must be the CLI's check of its own copy. |
| P1.9 | Three runs give byte-identical probe lines and records (no randomness in the probe; `SetSeed`). |

### P2. S2 — the parts (recall, to be read)

From recall, before any vendor page or datasheet is fetched. Each row will be replaced by what is read.

| Part | Lifecycle | I/O voltage | Identity (register → value) | ArduPilot driver at `fb773567` | Datasheet fetch from Furnace | KiCad 10.0.7 symbol / footprint | Notes |
|---|---|---|---|---|---|---|---|
| ICM-42688-P (TDK) | active | `VDDIO` 1.71–3.6 V | `0x75` → `0x47` | `Invensensev3` | served (tdk.com served 4210) | symbol absent; LGA-14 footprint present | FIFO-read; init touches banks 0, 1, 2 |
| ICM-45686 (TDK) | active | `VDDIO` 1.08 (or 1.71)–3.6 V | `0x72` → `0xE9` | `Invensensev3` | served | both absent | newer register map (indirect register access) |
| BMI088 (Bosch) | active | `VDDIO` 1.2–3.6 V | accel `0x00` → `0x1E`, gyro `0x00` → `0x0F` | `BMI088` | uncertain (Bosch served 404s to 4210 for a discontinued part) | symbol present (uncertain) | **two chip selects**; accel SPI reads return a dummy byte first |
| BMI270 (Bosch) | active | `VDDIO` 1.2–3.6 V | `0x00` → `0x24` | `BMI270` | uncertain | absent | **an ~8 KiB configuration blob** uploaded at init |
| BMP390 (Bosch) | active | `VDDIO` 1.2–3.6 V | `0x00` → `0x60` | `BMP388` (accepts `0x50`, `0x60`) | uncertain | absent | SPI or I2C |
| BMP581 (Bosch) | active | `VDDIO` 1.08–3.6 V | `0x01` → `0x50` | `BMP581` | uncertain | absent | added by me beyond the WO's examples |
| DPS310 (Infineon) | active (or NRND) | `VDDIO` 1.2–3.6 V | `0x0D` → `0x10` | `DPS280` (accepts the DPS310) | likely refused (Infineon's CDN) | symbol present (uncertain) | coefficients in registers |
| ICP-20100 (TDK) | **EOL** (TDK's pressure family page said "Pressure products are EOL", `4210` F3) | 1.8 V or 3.3 V | `0xA3` → `0x63` | `ICP201XX` | served | absent | OTP boot sequence at init |
| MS5611 (TE) | active | single `VDD` 1.8–3.6 V | none (PROM CRC) | `MS5611` | uncertain | `MS5611-01BA` symbol and footprint present | command-based, no registers |

- **P2.1** Every current candidate accepts 3.3 V I/O, so the recommended pair needs no 1.8 V domain from `4300d` (recall).
- **P2.2** The predicted recommendation: **ICM-42688-P** (one chip select, no blob, a register map ArduPilot drives through `Invensensev3`) and **BMP390** (registers, one identity byte, ArduPilot's `BMP388` driver), both active.
- **P2.3** Neither recommended part has a KiCad 10.0.7 symbol; both have a stock footprint. The build would vendor a symbol or ask for a ruling (recall; checked in the image).
- **P2.4** ArduPilot reads the ICM-42688-P only through its FIFO (`FIFO_COUNTH`, `FIFO_DATA`), never the data registers, and its init writes fewer than 20 registers across banks 0, 1 and 2 (recall).

### P4. S4 — a model tested without firmware (added before its run, after S1)

Committed before `s4.py` ran. The method: Renode's monitor calls a peripheral's public methods by name.

| # | Prediction |
|---|---|
| P4.1 | `sysbus.spi1.spi1mux.u5 Transmit 0x80` then `… Transmit 0x00` returns `0xA5` from the probe, printed by the monitor as a value, and `… FinishTransmission` returns nothing. No firmware and no `RunFor` are needed. |
| P4.2 | `sysbus.i2c1.u2 Write [0xD0]` (an array literal) then `sysbus.i2c1.u2 Read 1` prints a one-byte array holding `0x5A`. |
| P4.3 | The probe logs the same Noisy lines for these calls as for the firmware's bytes, so the same witness format serves both levels. |

## Results — after the predictions

The predictions above were committed alone at `7ce365d`, before any container started and before any vendor fetch. They are unchanged below this line.

### Containers (measured)

Every container ran through `spike/4220/run.sh`, which calls `~/bin/capped-run --mem 8G --tasks 8192` with `--network none`, the repository at `/repo` and `~/.cache/aet-4220` at `/cache`; it starts nothing while another `aet4220-*` container runs. Every read-back in `spike/4220/out/caps.log` is `memory.max=8589934592 memory.swap.max=0 pids.max=8192`, each with its own `run-p…scope` path. `TMPDIR` printed `unset` inside (`s1.py`'s first line). One container at a time.

### S1 — the pinned Renode loads our C# (measured)

`spike/4220/s1/s1.py`, 8 cases, 14 boots, in one container (`out/s1.jsonl`, 12 rows; a 13th boot in a second container listed the temporary directory after exit, `out/s1-ls.jsonl`). The firmware is `4210`'s `imu-script` ELF, checked by sha256 before every case; the platform is `4210`'s emitted `spi-imu` platform (sha256 `c91018d4…`) with only `u2` and `u5` swapped.

| # | Prediction | Result | Measured |
|---|---|---|---|
| P1.1 | compiles and loads offline | **held** | `include @/repo/spike/4220/s1/AetProbe.cs` logs `[INFO] Including script(s): …` and nothing else; no error; the boot ends with the boot word `0x2100B007`. `/opt/renode` carries `Microsoft.CodeAnalysis.CSharp.dll`. |
| P1.2 | the include adds 1–6 s | **failed (faster)** | 3 boots each: 2.002 / 2.055 / 2.031 s without, 2.653 / 2.619 / 2.681 s with the include: **+0.62 s** for a 2-class file. |
| P1.3 | both attach; records `0x5A` and `0xA5` | **held** | record 0 (I2C `0x76`, `0xD0`) = `0x5A`, record 2 (SPI `spi1mux` 1, `0x00`) = `0xA5`. The probe's 20 Noisy lines follow the firmware's bytes in order: `i2c1.u2: AetProbeI2C write 1 byte(s), pointer 0xD0`, `… read 1 byte(s), first 0x5A`, then `spi1.spi1mux.u5: AetProbeSpi byte 0x80 -> 0x00`, `… byte 0x00 -> 0xA5`, the configuration write's 2 bytes and the data read's 7, each transfer closed by `AetProbeSpi finish`. The multiplexer finishes the probe on other devices' edges too (`4210` F11): 3 of its 6 `finish` lines close no transfer of its own. |
| P1.4 | short name resolves in Renode's namespace; our own namespace only in full | **held for the first, failed for the second** | `Sensors.AetProbeSpi` resolves. `Aethelgard.Models.AetProbeOwnNs` does **not**, even written in full: `Error E04: Could not resolve type: 'Aethelgard.Models.AetProbeOwnNs'.` Read after: `TypeManager` indexes only types whose namespace starts with `Antmicro.Renode` or `NetMQ` (`TypeManager.cs` L262–266, L354–356). **A model must live in `Antmicro.Renode.Peripherals.*`.** |
| P1.5 | the fault's text | **held** | `Errors during compilation or loading:` / `Could not compile assembly from: /repo/spike/4220/s1/AetProbeFault.cs` / `…AetProbeFault.cs(13,27): error CS1002: ; expected` / `…AetProbeFault.cs(14,13): error CS0246: The type or namespace name 'NoSuchType' could not be found (are you missing a using directive or an assembly reference?)`. Untimestamped monitor text, on stdout. |
| P1.6 | the rest of the `-e` line runs; exit 0 | **failed in part** | **The line stops at the failed include:** no `LoadPlatformDescription`, no `RunFor`, no `BEGIN` marker; `machine-0: Disposed.` follows the diagnostics, and **Renode exits 0**. So the refusal must be the CLI's: the diagnostics' first line, or the missing `BEGIN`, which the CLI already treats as a failed boot. Never the exit code. |
| P1.7 | the DLL in `/tmp/renode-<pid>/`, gone at exit | **held for the place, failed for the cleanup** | `/tmp/renode-<pid>/<guid>.dll`, **4,096 bytes**, beside an empty `<guid>.tmp` and Renode's own 1,575,616-byte native library (`<guid>.tmp`, executable). **The directory and the DLL are left after exit** (listed after `quit`, `out/s1-ls.jsonl`); the next Renode deletes the directories of dead PIDs at start (`TemporaryFilesManager.Cleanup`, L119–137, read). Nothing is written under `HOME` but the history and `renode.config.lock`, as without an include. |
| P1.8 | a changed file compiles silently | **held** | the probe with `0xA6`/`0x5B` compiles with no complaint and the records read `0x5B` and `0xA6`. |
| — | (not predicted) two files defining the same types, in turn | **refused by Renode** | the second include fails with `Tried to load assembly 'Antmicro.Renode.Peripherals.Sensors.AetProbeSpi' that has been already loaded. Aborting operation.` (`TypeManager.cs` L570), and the line stops as in P1.6, in either order. A second model can never silently shadow the first. |
| P1.9 | three runs identical | **held** | the probe's lines and all 12 read values are byte-identical across the 3 boots. |

**Where the compiled assembly goes under the CLI.** The CLI gives Renode `TMPDIR` = `HOME` = the call's own `workdir`, read-write for the tool (`crates/aethelgard-cli/src/run.rs` L517–518). So the DLL would land in the call's scratch, and go with it (read, not measured: no run of this work order used the CLI's sandbox).

**§7's first stop does not fire:** the pinned Renode compiles a C# file offline, in the image, in the CLI's composition.

### S1 — the load mechanism and its pin (proposal)

What Renode gives: a monitor `include @file.cs` before `LoadPlatformDescription`, which compiles whatever bytes are at that path (P1.8), hashes them only to skip a recompile (`Monitor.cs` L405–428), stops the line on a compile error with exit 0 (P1.6), and refuses a second definition of a type (above). The `.repl` cannot include C#. So the pin is ours, by the pattern this repository already uses twice (`aethelgard-kicad-lib`'s data crate with `aethelgard-eda`'s pins, `AET-WO-1304`; the bridge's `BRIDGE_SOURCE_SHA256`, `analog.rs` L66–74):

1. **`models/renode/` is a data-only crate** (`aethelgard-renode-models`, `MIT OR Apache-2.0`, `#![no_std]`, `#![forbid(unsafe_code)]`): one `<Part>.cs` per part beside `src/lib.rs`, which exports `FILES: [(&str, &str); N]` by `include_str!` from inside its own package, so a published CLI carries them. No hashes there, no logic.
2. **The pins live with the emitter** (`aethelgard-twin`): a table of `(part, file, sha256)` constants, and a test that recomputes each sha256 from `FILES`. A changed model fails the build's tests until its pin moves with it, in the same commit, by review.
3. **The emitted platform names each model** in its `@generated` header, as it already names `stm32h7.repl`'s sha256: `// - Model: models/renode/ICM42688P.cs sha256 <64 hex>` for each part whose model is ours, and the device line uses the class's name in `Antmicro.Renode.Peripherals.Sensors` (P1.4). The regeneration test covers the header.
4. **At boot the CLI**:
   - refuses a platform whose header names a model or sha256 not in its own table (`ModelUnpinned`, by name);
   - writes `FILES`' bytes into the call's sealed scratch (`AET-WO-1601`: the root is the CLI's, read-only to the tool uid), re-reads the copy and refuses unless its sha256 equals the pin (`ModelPinMismatch`);
   - puts one `include @<copy>` per model, all models in one command, before `machine LoadPlatformDescription`;
   - refuses the boot if the output holds `Errors during compilation or loading:` (`ModelLoadFailed`, carrying the diagnostics), before it looks for `BEGIN`.
5. **`replay` and `verify`** re-check the header's pins against the table, as they re-check the platform's other hashes.

**What it claims.** The bytes Renode compiled are the pinned bytes, up to the same-uid residual `RUL-19` (2) already names for every gate (a process of the tool's uid could change the copy between the CLI's hash and Renode's read, unless the `1601` boundary held). It does not claim the compiled DLL: Roslyn's output is not pinned (its sha256 was `a4dd59c2…` in one run; whether it is stable across runs was not measured).

### S4 — a model driven from the monitor (measured)

`spike/4220/s1/s4.py`, one boot, no firmware and no `RunFor` (`out/s4.jsonl`).

| # | Result | Measured |
|---|---|---|
| P4.1 | **held** | `Transmit 0x80` printed `0x00`, `Transmit 0x00` printed `0xA5`; `FinishTransmission` printed nothing. |
| P4.2 | **held** | `Write [0xD0]` accepted the array literal; `Read 1` printed a one-byte array, `0x5A` (on three lines: `[`, `0x5A, `, `]`). |
| P4.3 | **held** | each call logged the same Noisy line the firmware's byte logged in S1 (`AetProbeSpi byte 0x00 -> 0xA5`, `AetProbeI2C write 1 byte(s), pointer 0xD0`, …). |

So a model's register map can be checked in the image against the datasheet's reset table with no firmware at all, and the line the check reads is the line the twin's witness reads.

### S2 — the parts, by evidence (read)

Three subagents gathered the evidence: IMU pages and datasheets (`~/.cache/aet-4220/ds/imu/REPORT.md`), barometer pages and datasheets (`ds/baro/REPORT.md`), and ArduPilot at `Copter-4.7.2` (`fb773567…`, confirmed by `rev-parse`; `spike/4220/s2-ardupilot.md`, 665 lines, file:line per step). I spot-checked every fact the recommendation rests on, at the cited line or page, and recomputed every sha256 (`sha256sum -c` clean in both directories):
- ICM-42688-P: `WHO_AM_I` §14.58, reset `0x47`; VDDIO in Table 3; the page's `Production` badge; and the datasheet's URL, which appears in the held product page.
- ArduPilot `Invensensev3` L139–140, L247–252, L351, L456–459, L886–902.
- `AP_Baro_BMP581` L23–25, L40–41, L106–133, L170–180.
- `AP_Baro_BMP388` L28–29, L92–102.
- ICP-20100's `field_status` `EOL`.

KiCad 10.0.7's libraries were listed in the pinned image (measured, two containers; `spike/4220/out/kicad-libs.txt`; `Sensor_Motion.kicad_sym` sha256 `ff5ca176…`, `Sensor_Pressure.kicad_sym` `63740b4b…`).

| Part | Lifecycle (held page) | Datasheet (held) | I/O voltage | Identity | ArduPilot at `fb773567` (probe + init) | Read path | KiCad 10.0.7 |
|---|---|---|---|---|---|---|---|
| **ICM-42688-P** | **"Production"**: the status badge beside the part's own title, `field_status` `Production` (`icm42688.html` `764fd014…`). The page also carries `Announced` and `Active`, which belong to related products. | DS-000347 Rev 1.9 (11/12/2024), `9c6b57b0…`, from the URL the held product page lists (TDK's CDN `d17t6iyxenbwp1.cloudfront.net`) | **VDDIO 1.71–3.6 V**; VDD 1.71–3.6 V (Table 3) | `0x75` → `0x47` (§14.58) | `Invensensev3`: probe 1 read, 0 writes; `start()` 32 write transactions, 16 distinct registers (banks 0, 1, 2 via `REG_BANK_SEL` `0x76`), 3 reads, 1 checked register (`INTF_CONFIG1` `0x4D`); no polled status | **FIFO only** (IV3 L351): 16-byte packets, count `0x2E`, data `0x30`, 1 kHz (2 kHz on H7 instance 0). It sets `INTF_CONFIG0` = `0xC0` (little-endian, counts in records), against the reset `0x30` (big-endian, bytes) | symbol **absent**; `Package_LGA:LGA-14_3x2.5mm_P0.5mm_LayoutBorder3x4y` present (its land pattern against the datasheet's: not checked) |
| ICM-45686 | "Production" (`field_status`, `icm45686.html` `4b8f6f95…`). The page's "EOL" belongs to a related microphone (T5818). | DS-000577 Rev 1.0, `1fc7e54b…` (TDK's CDN, listed in the held page) | VDDIO 1.08–3.6 V | `0x72` → `0xE9` | `Invensensev3`: 0x75 read first, then 0x72; IREG indirect writes (4 addresses through `0x7C`–`0x7E`); **a hard gate**: `0x19` bit 7 after soft reset | FIFO only, little-endian by default | both absent |
| BMI088 | **no lifecycle word** (`bmi088.html` `f8efce16…`; a distributor "Out of Stock" widget only) | BST-BMI088-DS000-19 Rev 1.9, `53ab734d…` (read through a removed copy restriction: F3) | VDDIO 1.2–3.6 V, **VDD 2.4–3.6 V** | accel `0x00` → `0x1E`, gyro `0x00` → `0x0F` | `BMI088`: **two SPI devices**, accel reads return a dummy byte, accel writes verified by read-back (8 tries) | FIFO only | **symbol present** (`Sensor_Motion:BMI088`, footprint `Bosch_LGA-16_4.5x3mm…`) |
| BMI270 | no lifecycle word (`bmi270.html` `9c7c942a…`) | BST-BMI270-DS000-08 Rev 1.6, `0dab4065…` (as BMI088: F3) | VDDIO 1.2–3.6 V | `0x00` → `0x24` | `BMI270`: **a configuration blob** to `INIT_DATA` `0x5E`. It is 8 kB in the datasheet (§4.4, p25), and 328 bytes in ArduPilot's `maximum_fifo_config_file` (B270 L105–124). A **hard gate**: `0x21` bit 0 within 20 ms. Every write is verified by read-back, `CMD` included | FIFO only | both absent (`Bosch_LGA-14_3x2.5mm` footprint present) |
| BMP390 | **no lifecycle word** (`bmp390.html` `a0163b10…`). Listed in Bosch's current pressure catalogue (`bosch-pressure-overview.html` `b955a973…`: BMP384, BMP390, BMP580, BMP581, BMP585) | held, `8dcf0f74…`, **not read** (F3) | VDDIO 1.2–3.6 V (product page) | `0x00` → `0x60` (ArduPilot `BMP388` L29; datasheet unread) | `BMP388`: 2 distinct writes (`CMD` `0xB6`, `PWR_CTRL` `0x33`). Calibration is 21 bytes at `0x31`/`0x36` | registers, 7 bytes at `0x03` every 20 ms. **It needs the `STATUS` bits `0x20`/`0x40`**, a sensor-driven data-ready. Host-side polynomial compensation | both absent |
| **BMP581** | **no lifecycle word** (`bmp581.html` `67e49600…`). Listed in the same catalogue page | held, `ddcf4202…`, **not read** (F3) | VDDIO 1.08–3.6 V (product page) | `0x01` → `0x50` (ArduPilot `BMP581` L23–25; datasheet unread) | `BMP581`: 3 writes, 2 distinct (`ODR_CONFIG` `0x37`, `OSR_CONFIG` `0x36`), both checked; **two gates**: `STATUS` `0x28` bit 1 set, bit 2 clear; `INT_STATUS` `0x27` bit 4 set | registers, two equal 6-byte reads at `0x1D` every 20 ms. **On-chip compensation**: Pa = raw / 64, °C = raw / 65536 (L174, L179) | both absent |
| DPS310 | **refused**: Infineon answers HTTP 202 with 0 bytes, 3 product and 3 datasheet URLs, retried once | — | — | `0x0D` → `0x10` (ArduPilot `DPS280`) | `DPS280` | registers, a status-bit gate | both absent |
| ICP-20100 | **EOL** (`field_status` `EOL`, `icp20100.html` `6c5343a1…`; the family's "Pressure products are EOL") | DS-000416 Rev 1.4, `86c21185…` ("preliminary") | VDDIO three bands, 1.08–3.63 V | `0x0C` → `0x63` | `ICP201XX`: **three unbounded waits** (ICP L196–204, L223–231, L436–440) | FIFO | both absent |
| MS5611-01BA03 | **refused**: te.com 403, twice | — | — | none (a PROM with CRC-4) | `MS56XX` | commands | **symbol present** (`Sensor_Pressure:MS5611-01BA`, footprint `LGA-8_3x5mm_P1.25mm`) |

**No driver uses a data-ready pin**, not even behind an `#ifdef` (all nine, read). **None needs a 1.8 V domain:** every current candidate's I/O range includes 3.3 V.

### The predictions beside the results (S2)

| # | Prediction (recall) | Result |
|---|---|---|
| P2 rows | lifecycle, I/O, identity, driver, fetch, KiCad, as tabled | **ICM-42688-P:** every column held. **ICM-45686:** held (VDDIO 1.08 V). **BMI088:** IDs, two selects, dummy byte and symbol held; Bosch **served**, not refused. **BMI270:** ID and blob held; the blob is 8 kB (datasheet) or 328 B (ArduPilot), not "~8 KiB" alone. **BMP390:** ID held at ArduPilot; served, but encrypted (F3). **BMP581:** held at ArduPilot. **DPS310:** refused, as predicted. **ICP-20100:** EOL held; the identity register is `0x0C`, not `0xA3` (**failed**). **MS5611:** refused (predicted uncertain); symbol held. No part's lifecycle reads "active" in those words: TDK says "Production", Bosch says nothing |
| P2.1 | no 1.8 V domain needed | **held** |
| P2.2 | ICM-42688-P and BMP390 | **held for the IMU, failed for the barometer:** the BMP581 is recommended, for its closed-form scale (below) |
| P2.3 | neither has a symbol; both have a footprint | **held for the IMU.** For the barometer, no BMP5xx footprint was found by name in `Package_LGA` (unchecked against its land pattern) |
| P2.4 | FIFO only; under 20 registers across banks 0–2 | **held:** 16 distinct registers, banks 0, 1, 2 (no bank 4) |

### Recommendation

**IMU: the ICM-42688-P.**
- **Its evidence is complete:** "Production" on TDK's own page, a held and read datasheet, VDDIO 1.71–3.6 V, and ArduPilot's `Invensensev3`.
- **It is the smallest model ArduPilot accepts:** one chip select, no blob, no dummy byte, no polled status at probe or init, and 16 registers at init.
- **Its costs:**
  - KiCad has no symbol (a decision: D5);
  - the input capacitance is "< 10 pF" with no column resolved (Table 4), so `4300c`'s `extract` needs a ruling or a cited maximum, as `4210` F14 did;
  - ArduPilot reads only its FIFO.
- **The alternates:**
  - the ICM-45686 is also "Production", but needs IREG indirect access and a hard reset gate;
  - the Bosch IMUs need two selects and a dummy byte (BMI088), or a blob, write-verify and a hard gate (BMI270).

  Each of these grounds stands on ArduPilot's source alone, without the Bosch datasheets.

**Barometer: the BMP581, with the BMP390 as the alternate.**
- **The deciding fact is `4210` D3, the exact value.** The BMP581 compensates on-chip, so a judge-set pressure becomes register bytes by one closed-form scale: raw = round(Pa × 64), and °C × 65536 for temperature. That is read at ArduPilot's source (L174, L179), and the datasheet must confirm it.
- The BMP390 reports raw ADC counts that the host corrects through 11 NVM coefficients and a polynomial (BMP388 L203–237). A model must then invent coefficients and invert the polynomial numerically. That is sensor physics beyond a judge-set stimulus, which the work order rules out.
- **Its other strengths:**
  - two registers at init;
  - two status gates a model sets at reset;
  - I2C (`0x46`/`0x47`) and SPI;
  - VDDIO 1.08–3.6 V.
- **Its costs:**
  - **its datasheet is held but unread** (F3, D2), so every register value the build needs waits on that decision;
  - Bosch's page has no lifecycle word, only a place in the current catalogue (D3);
  - no KiCad symbol.

**The §7 stops.**
- **The first does not fire** (S1).
- **The second does not fire as worded.** The ICM-42688-P has a fetched datasheet and a driver, and so does each Bosch barometer, whose datasheets were fetched and hashed. What the barometer lacks is a read datasheet and a lifecycle word: decisions D2 and D3, not a stop.

### S3 — the model contract (proposal; the timer, GPIO and FIFO parts are inferred, not measured)

**What every model of ours logs (to `4210`'s witness).** `4210`'s build judges each step by the exact whole lines a model writes (`wo/aet-4210-build`, `crates/aethelgard-twin/src/model.rs` `Model::witness`, `skipped`, `warnings`, `stimulus`; read with `git show`, its worktree untouched). A model of our own can be shaped for that judge from the start:
- **One `Noisy` line per transaction,** written when the transaction ends: at chip-select release on SPI, or at `FinishTransmission` on I2C, where S1 measured one finish after a write-then-read. Every byte of the transaction is in the line:
  - `ICM42688P read 0x75 -> 0x47`;
  - `ICM42688P write 0x4E 0x0F`;
  - `ICM42688P bank1 write 0x0C 0x06`;
  - `ICM42688P read 0x1F -> 0x00 0x00 0x00 0x00 0x40 0x00`;
  - `BMP581 read 0x01 -> 0x50`.

  The device name prefix is Renode's (S1 measured `spi1.spi1mux.u5: …`).
- **Nothing on an empty finish.** S1 measured the multiplexer finishing the probe 3 times on other devices' edges (`4210` F11). Our model logs only when it saw bytes, so `skipped()` is empty for it and F11 cannot reach it.
- **No `Warning` in a correct script.** The model does not use Renode's tag framework for registers the script touches. A write to a read-only or unknown register logs one declared `Warning` line, which a step may list (`4210` D4).
- **Size per step:** a line is under 40 + 5n bytes for n bytes moved. ArduPilot's largest single read is 8 FIFO packets, 129 bytes, so under 700 bytes, inside kind 22's 1,800-byte step budget (`4210` S2). The data registers' 6- or 12-byte burst is under 100 bytes.
- **Step kinds** are `4210`'s `Identity`, `Configure` and `Data`. The barometer needs a `Quantity::Pressure` (in Pa) beside `4210`'s acceleration and temperature (`parts.rs` L155–169 on that branch).

**The minimum register map.**
- **ICM-42688-P:**
  - **identity:** `WHO_AM_I` `0x75` = `0x47`;
  - **ArduPilot's init set** (16 registers): bank 0 `0x16`, `0x4B`, `0x4C`, `0x4D`, `0x4E`, `0x4F`, `0x50`, `0x5F`, `0x76`; bank 1 `0x0B`–`0x0E`; bank 2 `0x03`–`0x05`. Each register keeps its datasheet reset value (`ds/imu/REPORT.md` §1.6, pp. 64–88) and reads back what was written. `0x4D` is checked by ArduPilot every 20th callback;
  - **data:** `TEMP_DATA` `0x1D`–`0x1E`, `ACCEL_DATA` `0x1F`–`0x24`, `GYRO_DATA` `0x25`–`0x2A`, with `INTF_CONFIG0`'s `SENSOR_DATA_ENDIAN` honoured. The reset is big-endian and ArduPilot switches to little-endian, so the bring-up script and ArduPilot see different layouts, and the model must serve both;
  - **status:** `INT_STATUS` `0x2D` (reset `0x10`, `RESET_DONE`).
- **BMP581:** `CHIP_ID` `0x01` = `0x50`; `INT_STATUS` `0x27` and `STATUS` `0x28` at the values ArduPilot gates on; `OSR_CONFIG` `0x36` and `ODR_CONFIG` `0x37`, read-back-stable; `CMD` `0x7E`; data `0x1D`–`0x22`. Every reset value waits on the datasheet (D2).

**The stimulus to register values, through the datasheet's scale.** The model exposes properties in physical units, set by the judge's monitor command before `RunFor` (`4210` D3's route, measured there):
- `AccelerationX/Y/Z` in g and `AngularRateX/Y/Z` in dps, for the IMU;
- `Pressure` in Pa and `Temperature` in °C, for the barometer.

At each sample it writes round(value × sensitivity) into the data registers, clamped to the field:
- **ICM-42688-P accel:** 2,048 / 4,096 / 8,192 / 16,384 LSB/g at ±16 / 8 / 4 / 2 g (Table 2, p12), from `ACCEL_CONFIG0`'s `FS_SEL`. The reset is ±16 g, so 1 g reads `0x0800`, not `4210`'s `0x4000`;
- **ICM-42688-P gyro:** 16.4 to 2,097.2 LSB/dps (Table 1, p11);
- **BMP581:** × 64 for Pa and × 65,536 for °C (ArduPilot L174, L179; the datasheet citation is pending, D2).

**Until the firmware enables the sensor** (`PWR_MGMT0` `0x4E` reset `0x00`: accel and gyro off), the data registers hold their reset value, `0x8000` (p65–67), and no sample is taken. So a firmware that never enables the part reads `0x8000` in the twin, as on silicon. That is `4210` F5's lesson built in (read; the silicon behaviour is the datasheet's reset table, not a bench run).

**Data-ready from the sensor's own sampling.**
- **How:** the model owns a `LimitTimer` on `machine.ClockSource` at the configured output data rate, the idiom `Sensors/AS6221.cs` L31–32 and L157–161 use. That takes an `IMachine` constructor argument, which the probe did not have, and is unmeasured.
- **On each tick, if enabled,** the model:
  - latches the stimulus into the data registers;
  - sets `INT_STATUS.DATA_RDY_INT` (bit 3, cleared on read; p69);
  - if `INT_SOURCE0.UI_DRDY_INT1_EN` (bit 3, p85) is set, drives an `INT1` `GPIO` with `INT_CONFIG`'s polarity, pulsed or latched mode, and pulse width (100 µs or 8 µs; p65, p85).
- **`INT_STATUS` is read-only in our model:** a firmware store to it logs a declared `Warning` and changes nothing. The ICM-20948's model let a store raise the interrupt (`4210` P4.2).
- **The BMP581** gets the same timer, at `ODR_CONFIG`'s rate, with its data-ready status bit (bit position from the datasheet, D2).

**What that would let `RUL-19` (5) judge (inferred).** Today "parked" means no interrupt enabled (SysTick `CSR` = 0, every `ISER` = 0, `4210` D6). With a sensor-driven line, a firmware that arms one EXTI could be judged on four things:
- the only enabled interrupt is that EXTI line;
- every entry to its handler follows a model `INT1` edge in the log;
- the edges' count over the run is ⌊t × ODR⌋ ± 1 after the enable;
- the step that reads the data follows an edge.

That replaces "no interrupt runs after the boot word" (`2204` A2) with "the one interrupt that runs is the sensor's". It does not claim the edge's timing beyond Renode's virtual time (`RUL-05`). **ArduPilot uses no data-ready pin for any candidate**, so this serves the bring-up firmware and `RUL-19`, not ArduPilot.

**The FIFO.** ArduPilot reads these IMUs only through their FIFO (IV3 L351), with 16-byte packets of header, accel, gyro, temperature and timestamp (IV3 L153–159, L173). **The datasheet's packet figures are images** (`ds/imu/REPORT.md` §1.9). The layout is therefore citable today to ArduPilot's struct, not to TDK. Proposal:
- **the build implements the data registers now,** and the bring-up script reads `ACCEL_DATA` (`0x1F`, 6 bytes) with the judge's exact value;
- **the FIFO follows,** with ArduPilot in the twin. The timer above is the FIFO's producer, so adding it is a push on each tick plus `FIFO_COUNTH/L` (`0x2E`/`0x2F`, latched on the high byte's read, p69–70) and `FIFO_DATA` (`0x30`). The datasheet's packet figure then has to be read from the PDF's images (D7).
- ArduPilot-in-the-twin would need:
  - the FIFO;
  - `INTF_CONFIG0`'s endianness and count-in-records;
  - the header `(h & 0xFC) == 0x68` (IV3 L458; a wrong header resets the FIFO silently in a loop);
  - the checked-register read-back;
  - a hwdef that leaves `HAL_INS_HIGHRES_SAMPLE` undefined (16-byte packets).

### S4 — tests (proposal; one level measured)

1. **The C# against the datasheet's register defaults, with no firmware (measured feasible: P4.1–P4.3).** A tier-2 test loads the model, calls `Transmit` and `FinishTransmission`, or `Write` and `Read`, from the monitor, and compares each register's read with its cited reset value. Each row carries its datasheet page. The test also writes each init register, reads it back, and checks the model's one line per transaction. A second table-driven case sets a stimulus and checks the data registers against round(value × sensitivity) at every `FS_SEL`.
2. **Through the twin, with `4210`'s witness (tier 2).** The Sentinel-02-shaped fixture's firmware runs the bring-up script (identity, configure, data). The judge witnesses each step's exact line and value as `4210` builds it, with `skipped()` empty and no declared warning. A seeded fault per part:
   - a wrong identity in the table;
   - a firmware that skips the enable, so reads `0x8000`;
   - a stimulus the record does not match.
3. **Against ArduPilot's probe sequence, read at source.** A table in the test lists ArduPilot's probe and init transactions, each cited to `fb773567` file:line (`s2-ardupilot.md` §1.2–1.3, §6.1). A monitor-driven case replays them against the model and checks every read ArduPilot gates on: `0x47`; `0x50`; `0x28` bit 1 set and bit 2 clear; `0x27` bit 4. It also checks that every checked register reads back. That is ArduPilot's probe as ArduPilot wrote it, without running ArduPilot.
4. **The pin.** A tier-1 test recomputes each model file's sha256 against the twin's constant. Others refuse a platform header naming an unknown model or sha256, and a scratch copy whose bytes changed. A tier-2 case boots with a deliberately non-compiling model and expects `ModelLoadFailed` (S1 measured the line Renode prints).

**What the adversarial pass mutates:**
- in the C#: the identity byte, one reset value, one sensitivity entry, the endianness switch, the bank select, `INT_STATUS` read-only, the empty-finish silence, and the timer's enable condition;
- in the Rust: the pin compare, the header parse, the include's placement before `LoadPlatformDescription`, the `Errors during compilation or loading:` check, and the stimulus scale.

Each must be killed by a test above, one mutant each (sized to the diff).

### S5 — upstream (read)

- **Renode is MIT** (`renode/LICENSE`; each model file carries Antmicro's MIT header, e.g. `ICM20948.cs` L1–6). **The models live in `renode-infrastructure`** (`src/Emulator/Peripherals/Peripherals/Sensors/`, 43 entries at `066a7f13`, base classes included).
- **The contribution path** (`renode/CONTRIBUTING.md`):
  - open an issue first;
  - fork, branch `NUMBER-short_name`, small commits `[#1234] tag: description`;
  - preferably a failing and a passing test from their issue-reproduction template;
  - **and sign Antmicro's Contributor License Agreement in the pull request** (L5, L52–54).
- **Licence fit:** our models are `MIT OR Apache-2.0` (`RUL-11`), so they can be offered under MIT as Renode's files are. If a model of ours copies code from an Antmicro model, Antmicro's notice must stay with it. The proposal is to write ours fresh and cite Renode's idioms only.
- **Proposal: offer them after the build's tests pass and the models have run in the twin for one phase**, so what is offered is tested and stable. Offer each part as its own pull request, with its monitor-driven register test as the Robot test Renode asks for.
- **The CLA and the public pull request are Scotty's call** (D9). Until then, nothing is sent.

## Findings

- **F1 · The pinned Renode compiles our C# offline in +0.62 s** (measured, P1.1–P1.2). Roslyn ships in the portable package.
- **F2 · Renode's failures exit 0.**
  - A failed include stops the `-e` line, and Renode exits 0 (measured, P1.6). It prints `Errors during compilation or loading:`, not the `There was an error executing command` the CLI matches as `LOAD_ERROR` (`renode.rs` L15, L659–681).
  - So today such a boot would fall to `renode.rs` L682–688, "renode printed 0 begin and 0 end markers" (read, not run through the CLI). That fails closed, but unnamed. The build names it (`ModelLoadFailed`).
- **F3 · The subagents split on the Bosch PDFs: a finding, not merged.**
  - All four Bosch datasheets are AES-encrypted with an **empty user password** and the **copy/extract permission cleared**.
  - The barometer agent did not decrypt them (BMP390, BMP581) and called it Scotty's call.
  - The IMU agent decrypted the BMI088 and BMI270 in memory, with a patched extractor (`~/.cache/aet-4220/ds-tools/pdftext_dec.py`; held files unmodified), and extracted their text.
  - I did not extend the decrypt to the barometers. Every BMI088/BMI270 fact above is labelled; no recommendation rests on one (the grounds against both stand on ArduPilot's source). The BMP581's register values all wait on this (D2).
  - The host also has no `pdftoppm`, so the Read tool cannot render the pages either.
- **F4 · TDK serves its datasheets from a CDN host, `d17t6iyxenbwp1.cloudfront.net`.** The URL is listed in TDK's own product page, held and hashed (`icm42688.html` `764fd014…` holds the DS-000347 URL; `icm45686.html` `4b8f6f95…` holds DS-000577's). `invensense.tdk.com`'s download link returns an HTML wrapper (`icm42688p_dl_wrapper.html`). Accepted here as the vendor's own link (this spike's barometer agent took the ICP-20100's datasheet the same way). Discard the TDK facts if the architect rules otherwise (D4).
- **F5 · Infineon and TE refuse Furnace:** Infineon with HTTP 202 and an empty body, TE with HTTP 403 (listed for Scotty below).
- **F6 · A model must live in `Antmicro.Renode.Peripherals.*`.** `TypeManager` indexes no other namespace (measured, P1.4; `TypeManager.cs` L262–266).
- **F7 · Two models defining one type cannot shadow each other.** Renode refuses the second and stops the line (measured).
- **F8 · Renode leaves its temporary directory, the compiled DLL included, after exit** (measured). Under the CLI that is the call's scratch (read), removed with it.
- **F9 · No ArduPilot driver among the nine uses a data-ready pin** (read). Every IMU is read through its FIFO only.
- **F10 · The ICM-42688-P's reset layout differs from ArduPilot's.** Reset is big-endian with byte counts (`INTF_CONFIG0` `0x30`); ArduPilot writes `0xC0`, little-endian with record counts. A bring-up script read at reset and ArduPilot's read disagree on byte order, and the model must implement the switch.
- **F11 · The ICM-42688-P's reset full scale is ±16 g** (`ACCEL_CONFIG0` `0x06`). The judge's exact value for 1 g is `0x0800` unless the script configures ±2 g.
- **F12 · ArduPilot's ICP-20100 driver can hang the boot task.** It has three unbounded waits (ICP L196–204, L223–231, L436–440), so a model that does not answer them would hang ArduPilot. Moot while the part is EOL.
- **F13 · Pin capacitance.** The ICM-42688-P's input capacitance is "< 10 pF" with no min/typ/max column resolved (Table 4, p14). `4300c`'s `extract` refuses a part with no cited maximum on a bounded SCK or MOSI line, so the build needs a ruling, as `4210` F14 did. The BMP581's waits on D2.
- **F14 · The 4300d note's I/O ranges.** None of the recommended parts needs 1.8 V I/O. That removes the ICM-20948 stand-in's 1.8 V domain from the purchase path, though not from the stand-in fixture.

## Datasheets and pages for Scotty

| What | URL | Result on 2026-11-15 |
|---|---|---|
| DPS310 datasheet and product page (3 URLs each) | `https://www.infineon.com/…dps310…` (full list in `ds/baro/REPORT.md`) | HTTP 202, 0 bytes, retried once |
| MS5611-01BA03 datasheet | `https://www.te.com/commerce/DocumentDelivery/DDEController?Action=srchrtrv&DocNm=MS5611-01BA03&DocType=Data+Sheet&DocLang=English` | HTTP 403 Access Denied, twice (body held: `ms5611_pdf_refused_403.html`) |
| MS5611 product page | `https://www.te.com/…ms5611…` (in REPORT.md) | HTTP 403, twice |
| ICP-20100 on `product.tdk.com` | (in REPORT.md) | HTTP 403; the same datasheet was taken from TDK's CDN link instead |

**Held but unread, needing D2:**
- **BMP581** `ddcf4202…`. The build needs from it:
  - `CHIP_ID` and its reset;
  - the `STATUS` and `INT_STATUS` reset values and bit meanings;
  - the `OSR_CONFIG`/`ODR_CONFIG` reset values and rate table;
  - the data register layout and its scale (× 64 Pa, × 65,536 °C);
  - the data-ready bit;
  - VDDIO;
  - the SPI/I2C maxima;
  - a maximum input capacitance;
  - the land pattern.
- **BMP390** `8dcf0f74…`, the same list, plus the NVM layout and the compensation formula.

## Decisions needed

- **D1 · The parts:** the ICM-42688-P and the BMP581 (recommended), or the alternates: the ICM-45686, and the BMP390 with a host-side compensation model.
- **D2 · The Bosch PDFs (F3).**
  - **(a)** Accept extraction through the empty user password for reading and citing, and apply it to the BMP581 (and BMP390) at the build;
  - **(b)** Scotty fetches or reads them himself and the values come back through him;
  - **(c)** discard the BMI facts and choose parts without Bosch.

  Recommended: (a) or (b), since the BMP581 is the recommendation.
- **D3 · Lifecycle wording.** `parts.json`'s `lifecycle` field takes `active`, `nrnd` or `eol`. TDK writes "Production". Bosch writes nothing, but lists the part in its current catalogue page. Rule whether "Production", and a current catalogue listing, each cite as `active`.
- **D4 · TDK's CDN host (F4):** accept the datasheet from the URL the held vendor page lists (recommended), or not.
- **D5 · KiCad symbols.** Neither recommended part has a KiCad 10.0.7 symbol. Three options:
  - **(a)** author our own symbol for each, with its pin table cited to the datasheet, in `aethelgard-kicad-lib` (`4300f`'s vendoring fence);
  - **(b)** wait for KiCad's library;
  - **(c)** generate the symbol from the parts table.

  The licence of a symbol of our own is a further question (`RUL-11`).
- **D6 · The pin mechanism:**
  - a data-only `models/renode/` crate;
  - pins in `aethelgard-twin`;
  - the platform header naming each model's sha256;
  - the CLI's sealed copy, re-hashed, included before the load;
  - `ModelUnpinned`, `ModelPinMismatch` and `ModelLoadFailed`.

  As proposed in S1, or another.
- **D7 · FIFO:** data registers now and the FIFO with ArduPilot-in-the-twin (recommended), or the FIFO in this build. If now, the packet layout cites ArduPilot's struct until the datasheet's figures are read.
- **D8 · Data-ready:** build the sensor-driven timer and `INT1` now and keep the firmware polled, so "parked" stands (recommended). Or also let the bring-up firmware arm one EXTI and amend `RUL-19` (5) as S3 sketches.
- **D9 · Upstream (Scotty):** whether and when to offer the models to `renode-infrastructure`, which needs Antmicro's CLA. Recommended: after the build, one part per pull request.
- **D10 · Pin capacitance (F13):** a cited maximum for the ICM-42688-P's "< 10 pF", or a ruling for `4300c`'s extract.

## §7 checked

- **The pinned Renode cannot load a C# file offline:** not met (measured, P1.1).
- **No current IMU or barometer has both a fetchable datasheet and an ArduPilot driver:** not met as worded. The ICM-42688-P has both, read. The BMP581 and BMP390 have both, but their datasheets are unread (D2) and they have no lifecycle word (D3).
- **A tool call denied by the permission classifier:** none in this work order.
- **`memory.swap.max` not 0:** never; every read-back was 0.

## Held, by full sha256 (`~/.cache/aet-4220/ds/`)

| File | sha256 |
|---|---|
| ICM-42688-P DS-000347 Rev 1.9 (`imu/icm42688p_ds.pdf`) | `9c6b57b0a2e45cf0251dd2423878573bd4da46e0cc2faa93dc558c28af51300a` |
| ICM-45686 DS-000577 Rev 1.0 (`imu/icm45686_ds.pdf`) | `1fc7e54bcb03126e2094745a29440b97cd1ff8aff679f8a31e4ac121c415f274` |
| BMI088 BST-BMI088-DS000-19 Rev 1.9 (`imu/bmi088_ds.pdf`) | `53ab734dba49ac202fa6182fc2b545df6c6d7459c9fec6e851a1209e6bac417e` |
| BMI270 BST-BMI270-DS000-08 Rev 1.6 (`imu/bmi270_ds.pdf`) | `0dab40659d3a8598af76a0b2da1fd8a216aeb87254541632c89145f974694af7` |
| BMP390 datasheet (`baro/bmp390.pdf`, unread) | `8dcf0f74449cb1627e865c7f24d328b76c204d8b9621091ede8c6869fb393e48` |
| BMP581 datasheet (`baro/bmp581.pdf`, unread) | `ddcf4202c4fdf2ab6ad86f22b67ba4d2471d76b9ce510397cacfe398400ab439` |
| ICP-20100 DS-000416 Rev 1.4 (`baro/icp20100.pdf`) | `86c2118516cdf943445abb915c03839c4f034bbcfc7d0dbb3599d214f3827f60` |
| TDK ICM-42688-P page ("Production") | `764fd0148dedea2cbe74e4a7dc34d64cba13508f3d54c72f86f6a83cc0568add` |
| TDK ICM-45686 page ("Production") | `4b8f6f95cdfa43e414f00ac85d6d2fcb2ea62943b05cfbea7a12a53db055dce6` |
| TDK ICM-42688-P download wrapper | `3f289250f910a13f7febd58f523f3b454a8cbce645030e0607461953f4986ca7` |
| Bosch BMI088 page | `f8efce167771c6db5d4e7d48cb1e09d6a89ee448be8718892f7bd17beea28da9` |
| Bosch BMI270 page | `9c7c942ad9541a6dff781b91d564bed9c7e73d5f256abbd79a467062dbc6b30b` |
| Bosch BMP390 page | `a0163b10f61000ea5cd9d6d9b34d14f70069008ab847669b10dddafba1e34f58` |
| Bosch BMP581 page | `67e496006ab5b3013dec7408cc4a392d0b04c7e666b0859b44f52403fa1b1d3f` |
| Bosch pressure-sensor catalogue (BMP384, BMP390, BMP580, BMP581, BMP585) | `b955a973fb97ef85e416250d007d2a4b13a772fb63b4cbe995aa353a2de2c390` |
| TDK ICP-20100 page ("EOL") | `6c5343a1ea72ea2be41cf8616c769dd4f387c9296ce92efedd5aedb1e76bf91c` |
| TDK ICP-20100 download resource | `ca291e77865f5b442ac3506a3fbf04222fd16ecb6e341a182243a955d2934b03` |
| TDK ICP-20100 product.tdk.com 403 body | `f4e7c81b114289865c76efbf35d01decaedf48e68e1645183c382653e9613b89` |
| TDK ICP-20100 product.tdk.com page | `7dc0209e35730f39545fdae83379e5bbf6b826227b2a954c9fc0375581174494` |
| TE MS5611 page (403 body) | `6bb58f6c965a9529b86c497dfbfa0bd5a226f92895edf3b2cd3ea276b9d58e7e` |
| TE MS5611 datasheet (403 body) | `5a8db0129245eed3af0b5e2b0e1bbd5c0796d71f862ba821248dc7ac22806e3a` |

## Notes sent (mailbox, appended with a quoted heredoc)

- **`4210.md`**: what our models will log. One Noisy line per transaction with every byte, nothing on an empty finish, no warning in a correct step. It also carries the size bound, the need for `Quantity::Pressure`, the namespace rule, and that a failed include is not `LOAD_ERROR`.
- **`4300d.md`**: each candidate's I/O voltage, and that the recommended pair needs no 1.8 V domain.
- **`4300f.md`**: no KiCad 10.0.7 symbol exists for either recommended part (D5 would land in its vendoring fence).
- **Inbox `4220.md` read** at start and before the push. It holds the architect's note that `1307` merged (`d52c051`). `origin/main` at `cd0bd31` (docs PR #139, this work order) is merged in at `777d216`. This work order holds no literal from the routed board.

## Gates (spike)

**The fence:** `git diff --name-only origin/main...HEAD` lists this file and `spike/4220/` only. The work order on `main` is byte-identical to the docs branch's copy that was read (sha256 `1a798148…`).

**Reduction pass (diff):** −1 line net, 0 duplicates removed, 2 dead items removed (an unused import in `s1.py`, and a `__pycache__` committed by the S4 run), 0 protected kept, 0 findings. The spike's harness reuses `s1.py`'s configuration in `s4.py` rather than copying it. The configuration itself is a copy of `renode.rs`'s, via `4210`'s `drive.py`, kept because the spike runs Renode without the CLI.

On the host, under `systemd-run --user --scope -p MemoryMax=32G -p MemorySwapMax=0`, at `777d216`:
- `cargo fmt --all -- --check`: ok.
- `cargo clippy --workspace --all-targets -- -D warnings`: ok.
- `cargo test --workspace`:
  - **The first run failed one test:** `tools_tier1::a_stub_cannot_create_files_beside_the_original_input`, with `cannot start kicad-cli: Text file busy (os error 26)`. That is an exec of a freshly written stub while a parallel test thread still held a write descriptor. This branch changes no crate.
  - **The rerun passed:** that binary alone, 121 of 121; then the whole workspace, exit 0, 80 `test result: ok` and no failure.
  - The flake is F15 below. No test was changed.

This branch changes no crate, so tier 2 is CI's (`roadmap.md` §6). The spike's Renode runs are listed under "Containers".

- **F15 · An `ETXTBSY` flake in `tools_tier1`** (measured once in two full runs on Furnace). `a_stub_cannot_create_files_beside_the_original_input` execs a stub `kicad-cli` that the test has just written. If another test thread forks while holding that file's write descriptor, `execve` fails with `ETXTBSY`. That is a test-harness race, outside this fence, reported for the architect.
