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
