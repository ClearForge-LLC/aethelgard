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
