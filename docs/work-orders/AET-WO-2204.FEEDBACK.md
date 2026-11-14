# AET-WO-2204 — FEEDBACK (spike)

**Branch:** `wo/aet-2204-boot-judge` · **Base:** `a9d94ec` (`origin/main`) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Cadence:** spike (§1), then **STOP** for the architect's ruling. No build code before it.

**Where everything runs.** The pinned tools image (`container/IMAGE`, `ghcr.io/clearforge-llc/aethelgard-tools@sha256:c69ff9bc…`) under rootless podman with `--memory X --memory-swap X --pids-limit -1`. After start, `0` is written into the container's `memory.swap.max`, and `memory.max`, `memory.swap.max` and `pids.max` are read back into `spike/2204/out/caps.log` before the command inside starts. `TMPDIR` is unset inside the container, as in CI.

Entry schema for findings: `finding · where · severity · type · recommendation · decision-needed`.

## Predictions — written before any run

This section is the branch's first commit, made before the first container started. Nothing in it is edited after a run. Each prediction has an id; the table at the end sets each result beside it.

**Provenance labels.** **[read]**: I read it at source. **[read-sub]**: only a subagent read it; I have not checked it myself. **[inferred]**: reasoned, not read.

### What the predictions rest on

- **Renode source.** `renode` at `ab721d88` (tag `v1.17.0`), with its `src/Infrastructure` submodule at `066a7f13` (the commit the tag pins). The image's tarball is `renode-1.17.0.linux-portable.tar.gz` (sha256 `4ba7c68b…`, `container/Dockerfile` L37–38). The build string the image's binary prints is recorded at the first run.
- **The reference transaction.** `boards/sentinel-01/src/board.rs` `transfer_within` (L357–407) **[read]**:
  1. `CR2` = `SADD | NBYTES 1 | START`, with no `AUTOEND` because a read follows;
  2. wait `TXIS`, then `TXDR` = `0xD0`; wait `TC`;
  3. `CR2` = `SADD | RD_WRN | NBYTES 1 | START | AUTOEND`;
  4. wait `RXNE`, then read `RXDR`; wait `STOPF`;
  5. **always** write `ICR` = `STOPCF | NACKCF` (`0x30`, L402).
- **`STM32F7_I2C.cs`** **[read]**:
  - `ISR.TXE` always reads 1; a write of 1 flushes `txData` (L228–236).
  - `TXIS` is writable only with `NOSTRETCH` set (L237–246).
  - `RXNE` is `rxData.Count > 0` (L248). `ADDR`, `STOPF`, `TC` and `TCR` are read-only (L249–253). `NACKF` and `BUSY` are tags that always read 0.
  - `ICR.STOPCF` clears `STOPF` (L278–284).
  - `RXDR` is read-only, and a read dequeues (L297–299, `ReceiveDataRead`). An empty queue logs a warning, `Receive buffer underflow!`, and reads 0.
  - A `CR2` write with `STOP` and no `START` runs `StopTransfer`, which sets `STOPF` (L173–176, `StopTransfer`). So a firmware sets `STOPF` without a transfer.
  - `StartTransfer` logs a warning, `Unknown slave at address {0}.` (decimal), for an address with no device, and returns before any flag is set.
  - `SetTransferCompleteFlags` sets `TC` without `AUTOEND`. With `AUTOEND` it calls the device's `FinishTransmission`, sets `STOPF` and leaves master mode.
- **`BME280.cs`** **[read]**:
  - `Write` logs `Write {bytes}` at `Noisy` (L38). `Read` logs `Read {bytes}` at `Noisy` (L75); it reads from the selected register and increments it.
  - `Reset` sets the selected register to `0x00`.
  - `selectedRegister` and `state` are private (L288–289).
  - It counts nothing.
- **The access log** **[read-sub]**: `sysbus LogPeripheralAccess <peripheral> true` exists (`SystemBus.cs` L688). It logs at `Info` every bus access to the peripheral, as `[cpu: 0x<PC>] Read… from 0x24 (ReceiveData), returned 0x60` (`ReadLoggingWrapper.cs` L27, `WriteLoggingWrapper.cs` L25).
- **The NVIC** **[read]**: `ICSR` (`0xE000ED04`) `VECTACTIVE` [8:0] is `activeIRQs.Peek()`, 0 when none is active (`NVIC.cs` L1077). The CPU's registers include an alias `CPSR` (index 25) on Cortex-M (`CortexMRegisters.cs` L371).
- **The reference ELF** **[read]**, from `nm`/`readelf` on 2301's build of the unchanged reference firmware:
  - `Reset` at `0x08000040` (size `0x50`); local `fault` at `0x08000090` (size 8); global `main` at `0x08000098` (size `0x500`, so to `0x08000598`).
  - One `R E` segment, `0x08000040`–`0x080005EE`.
- **`systick-before`** (`AET-WO-2301`, measured there) is the reference firmware with `SysTick` armed from the monitor before the run (`CSR` = 7, reload 999). It wrote its boot word and record, then parked in `fault`, at a final `PC` of `0x08000096`.
- **The emitter** **[read]**, `twin/platform.rs` `split_devices` (L123–143):
  - The name is the reference, lowercased, matching `[a-z][a-z0-9_]*`.
  - It is refused only if a `STOCK` Renode name or a solved peripheral already has it. `STOCK` lists 16 of the stock platform's instances.
  - `aethelgard-ir` lets a reference be any `[A-Za-z][A-Za-z0-9_]*`.
- **Renode's `.repl` grammar** **[read]**:
  - Keywords (`Grammar.cs` L106–129): `using`, `new`, `as`, `preinit`, `init`, `reset`, `local`, `true`, `True`, `false`, `False`, `none`, `empty`. `Identifier` excludes them (L131).
  - A typed entry whose name is already declared is `HandleDoubleDeclarationError` (`CreationDriver.cs` L118–127).
  - **[read-sub]**: `machine` and every registered peripheral, `sysbus` included, are pre-declared builtins (`CreationDriver.cs` L355–366). Variables are case-sensitive (`VariableStore.cs` L21).
- **The judge's log budget.** `LOG_FILTER.budget` is 8 MiB (`renode.rs` L137) **[read]**.

### S1. An observable a plain store cannot set

- **P1.1 [read]** The reference boot leaves `I2C1` thus:

  | Register | Value | Note |
  |---|---|---|
  | `CR1` | `0x00000001` | `PE` |
  | `CR2` | `0x020104EC` | |
  | `TXDR` | `0x000000D0` | |
  | `ISR` | `0x00000001` | `TXE` only, the reset value: the board's `ICR` write cleared `STOPF` |
  | `RxNotEmpty` | `False` | |

  One monitor read of `RXDR` gives `0x00000000` and logs `Receive buffer underflow!`.
- **P1.2 [read]** The forgery stores every candidate's reference value and calls no `transfer`:
  - `CR2` = `STOP` alone, which sets `STOPF`;
  - then `CR2` = `0x020104EC`, `TXDR` = `0xD0` and `ICR` = `0x30`;
  - an `ISR` write of `0x01`;
  - an attempted write of `0x60` to `RXDR`;
  - and the OK record with `0x60`.

  Every register above reads identically to P1.1.
  - Today's `simulate` judges it **`pass`**, exit 0, as `AET-WO-1900` §1.10 measured.
- **P1.3 [read]** A firmware that transfers to the wrong address (`0x77`, absent from the platform), then writes the OK record:
  - Renode logs `Unknown slave at address 119.`
  - The firmware's `TXIS` wait expires, so it writes no `TXDR`.
  - The controller shows `CR2` = `0x000100EE` (`START` self-clears), `TXDR` = `0x00000000` and `ISR` = `0x00000001`.
  - Today's judge fails it, `i2c_transaction_unwitnessed`, with all five fields named.
  - The same firmware that then also hand-writes `CR2` and `TXDR` reads identically to P1.1, and passes today.
- **P1.4 [read] — §7's first condition is predicted to hold.** No `I2C1` register in Renode 1.17.0 is both unforgeable and different from its reset or forged value after a real transfer:
  - the transaction consumes `RXNE` and `RXDR` itself;
  - the board clears `STOPF`, and a `STOP`-only write sets it anyway;
  - `TC` is cleared by the read phase's `StartTransfer`;
  - `NACKF` and `BUSY` are never modelled;
  - the model keeps no counter.

  The spike reports this under "§7 checked" as a decision, and still answers S2 to S5.
- **P1.5 [read]** Two observables outside the register file do show a real transfer to the right device:
  - **the device's `Noisy` lines**, `Write D0` then `Read 60`, emitted only when the controller calls the model's `Write`/`Read`. They carry the device's name, `i2c1.u2` **[inferred]**.
  - **the controller's access log** **[read-sub]**, whose `RXDR` line reads `returned 0x60`. `RXDR` is filled only from the device.

  None of the three probe firmwares other than the reference produces either.

### S2. One witness per transaction

- **P2.1 [inferred]** A specification with a second BME280, `U5` at `0x77` on `SENSORS`:
  - it validates, solves and generates;
  - its platform has two `I2C.BME280` entries;
  - its generated firmware reads both, `U2` then `U5`;
  - today's judge passes it and witnesses only `U5`, the last on the bus.
- **P2.2 [read]** Two forgeries, both passed by today's judge:
  - **forged-first:** `U2`'s read replaced by an OK record with no transfer, and `U5` real. `U2` is witnessed by nothing.
  - **forged-last:** `U2` real, and `U5` forged with `CR2` and `TXDR` hand-written for `0x77`.
- **P2.3 [inferred]** Each candidate witness, per real identity read:
  - **the device's `Noisy` lines:** exactly two (`Write D0`, `Read 60`), naming the device; a forged device has none.
  - **the controller's access log:** about 8 to 14 lines (two `CR2` writes, a `TXDR` write, an `RXDR` read, an `ICR` write, and the `ISR` polls). It names the controller, not the device; the device is known only through the logged `CR2` value.
  - **a monitor hook** writing per-transaction state: not measured unless the two above fail.
- **P2.4 [inferred]** Cost per transaction:
  - the `Noisy` lines add about 150 bytes, and the access log about 1.5 KiB, against the 8 MiB budget;
  - wall time is under 50 ms on a boot of about 2 s.
- **P2.5 [inferred]** On whether the judge's own reads show up in the access log:
  - The judge's own `sysbus ReadDoubleWord` reads of `I2C1` also appear in the access log, with no `[cpu: …]` prefix or a different one, so they can be told apart.
  - They print after `RunFor`, so they fall inside the read block.
- **P2.6 [read]** The two candidates differ in what they depend on:
  - **`Noisy` lines:** exist only because Renode's BME280 model logs. `P4`'s IMU model may not.
  - **access log:** independent of the device model.

### S3. The fixed point

- **P3.1 [inferred]** Where the loops sit:
  - The fixed point is the `loop { spin_loop() }` at the end of `main`, one `yield` and one backward `b` (4 bytes), at or near `[0x08000594, 0x08000598)`.
  - `main`'s other loop, `Board::take`'s `else`, is a separate loop elsewhere in `main`; the two are not merged (60 %).
- **P3.2 [read]** At the end of the run:
  - the reference boot's final `PC` lies in that loop at every `--for` sampled (10 ms, 10.00001 ms, 20 ms, 100 ms), alternating between its two instructions;
  - `systick-before` ends at `0x08000096`, inside `fault`.
- **P3.3 [inferred]** `ICSR.VECTACTIVE` and the `CPSR` alias:
  - **`ICSR.VECTACTIVE`:** 0 for the reference, 15 (`SysTick`) for `systick-before`.
  - **`CPSR`:** the alias reads xPSR, with `IPSR` [8:0] equal to `VECTACTIVE`.
- **P3.4 [inferred] — known from the ELF alone, no generator change needed.** The entry `Reset` ends in `bl main`, `main` is a global symbol with a size, and its tail is the self-loop.
  - Against a hostile firmware, every ELF-derived location is the firmware's own claim.
  - The observable Renode owns is `VECTACTIVE` = 0: the core is in thread mode, not in a handler.
  - §7's second condition is predicted **not** to hold.

### S4. One console level through the reads

The probe firmware is the reference with, just before its boot word:
- 500 reads of an empty `RXDR`, each logging one `Receive buffer underflow!` warning;
- then one read of `GPIOC.MODER`, whose clock is off, which logs `Tried to read a locked peripheral`.

That makes 501 known lines, the last emitted within a few instructions of the fixed point. It is run in the CLI's shape: one `-e` line, stdout through a pipe, the CLI's environment, no `logFile`.

- **P4.1 [inferred]** Before the static tail's `logLevel 3`, the console runs at Renode's default level (`Info` or below), so every one of the 501 lines is printable.
- **P4.2 [inferred]** Today's tail (`RunFor; logLevel 3`), ten runs:
  - lines are lost, a number that varies run to run;
  - in some runs the locked-access line is among them, and `simulate` then passes a firmware it should fail with `locked_peripheral_access`.
- **P4.3 [inferred]** The console kept at its level, with whole log lines skipped between the markers:
  - all 501 lines reach stdout in all ten runs;
  - the reads are byte-identical to today's tail's on the reference firmware;
  - the `peripherals` listing after `END` still parses.
  - Confidence on "none lost at `quit`" is 50 %.
- **P4.4 [inferred]** Between the markers, every line is either a bare value or a whole log line starting with `[`, ten runs in a row.

### S5. Renode's reserved names

- **P5.1 [read]** The name set:
  - the platform chain is `stm32h753.repl` → `stm32h743.repl` → `stm32h7.repl`, all pinned by hash (`RENODE_FILES`);
  - its declared instances, about 100, plus the 13 keywords and the builtins `machine` and `sysbus`, form a static set, pinned with the files.
- **P5.2** Outcome for each reference given to `U2` (one specification each):

  | Reference | Emitted as | Predicted outcome |
  |---|---|---|
  | `nvic`, `cpu`, `rcc`, `exti`, `timer2`, `usart1`, `dwt`, `crypto` | itself | `generate` exit 0; `simulate` loads → `platform_load_error`, "Variable '…' was already declared" **[read]** |
  | `sysbus`, `machine` | itself | same error kind, "already registered peripheral or builtin" **[read-sub]** |
  | `using`, `init`, `as`, `none`, `true` | itself | `generate` exit 0; `simulate` → `platform_load_error`, a syntax error ("expected variable name") **[read]** |
  | `NVIC` | `nvic` | as `nvic` **[read]** |
  | `gpioPortA` | `gpioporta` | `generate` exit 0; it loads, because Renode's names are case-sensitive **[read-sub]** |
  | `i2c1` | — | `generate` refuses, `InvalidDeviceName` (`STOCK`) **[read]** |

- **P5.3 [read]** A prefix on every reference would change the committed `.repl`, which §3's fence forbids. A refusal is the only remedy inside the fence.

## Results — after the predictions

Everything below was measured after commit `b88fd27`, which holds the predictions above unchanged.

### Where it ran, and what the spike added

**The runs.**
- 30 containers of the pinned image ran, each under `spike/2204/img.sh`: 27 at `--memory 16g` and 3 at `12g`. Besides the committed scripts' runs, some were setup (the version, the CLI and the firmware builds), and some were inline diagnostics of the `BEGIN` marker and of the stripped ELF. Each diagnostic was then committed as a script (`s4_marker.py`, `s3_stripped.sh`) and re-run.
- Every one read back `memory.swap.max=0` and `pids.max=max` before its command started (`out/caps.log`). None logged `CAPS NOT READ`.
- `TMPDIR` was unset inside each container.

**Renode's binary against the source read.**
- The image's binary prints `1.17.0+20260907gitf1dd1b4af` (Release, .NET 8.0.12).
- `f1dd1b4af` is two commits before the tag `ab721d88`, which is the source read above. Its `src/Infrastructure` is `cd4b002a`, against `066a7f13`.
- Between them only `MappedMemory.cs` and one unit test differ. Every file cited for S1 to S5 is the same in the binary as in the source read.

**The scripts in `spike/2204/`.** Python and shell only; no crate. A build deletes the directory.
- `img.sh` is the capped runner.
- `fw.py` builds each probe firmware from a copy of a generated firmware crate, offline and locked, as `twin_tier2.rs` `mutant()` does.
- `drive.py` runs one boot in `simulate`'s exact shape: one `-e` line, stdout through a pipe, the CLI's environment and command prefix (`ref-cmdline.txt` of 2301), with a selectable tail.
- `s1.py` (S1, S2), `s3.py`, `s3_stripped.sh`, `s4.py`, `s4_marker.py`, `s4_sync.py` and `s5.py` run the questions.
- `two-bme.json` is the reference specification with a second BME280, `U5` at `0x77`, on `SENSORS`.

**The outputs** are in `spike/2204/out/`, one JSON line per run. Raw stdout stayed in the cache.

**Corrections during the runs.**
- My first parser trimmed `\r` only at line ends; the CLI trims both ends (`renode.rs` `clean_lines`). Its file-mode rows were misread, so it was fixed and S4 was re-run. `out/s4.jsonl` is the re-run.
- `out/s1.jsonl` predates `s1.py`'s switch to a log file for the witness runs. Its `*-witness` rows read the witnesses from the console; that is how the interleaving (F2) was first seen. The committed `s1.py` reproduces `out/s2.jsonl`, not those rows.

**Probe firmwares** (sha256, first 12 hex digits):

| Firmware | sha256 |
|---|---|
| `ref` | `d9140a4294c9` (identical to 2301's build of `main`'s reference) |
| `forge-regs` | `6b31e34df014` |
| `forge-all` | `4cf93c90b63f` |
| `wrong-addr` | `61bf1f326c6b` |
| `wrong-addr-forge` | `4170421d61ba` |
| `burst` | `a807ce59dbf3` |
| `flood` | `cf6652c53466` |
| `flood-fast` | `3fa1d30548fa` |
| `flood-max` | `fc867423307e` |
| `flood-locked` (`LOCKED_AT=199461`) | `4a5279386723` |
| `two-ref` | `87a3ccfc8385` |
| `two-forged-first` | `ed13bef72d83` |
| `two-forged-last` | `5e7971400fd5` |

### S1 — measured (`out/s1.jsonl`, `out/s2.jsonl`)

`I2C1` after a 10 ms boot, in the CLI's shape. `RXDR` was read last, by the monitor:

| Firmware | `CR1` | `CR2` | `TXDR` | `ISR` | `RxNotEmpty` | `RXDR` | Today's `simulate` |
|---|---|---|---|---|---|---|---|
| `ref` | `0x1` | `0x020104EC` | `0xD0` | `0x1` | False | `0x0` (underflow logged) | pass |
| `forge-regs` (1900's) | `0x1` | `0x020104EC` | `0xD0` | `0x1` | False | `0x0` | **pass** |
| `forge-all` | `0x1` | `0x020104EC` | `0xD0` | `0x1` | False | `0x0` | **pass** |
| `wrong-addr` (to `0x77`) | — | `0x000100EE` | `0x0` | `0x1` | — | — | fail, `i2c_transaction_unwitnessed` |
| `wrong-addr-forge` | `0x1` | `0x020104EC` | `0xD0` | `0x1` | False | `0x0` | **pass** |

- **Register reads.** Every forgery reads exactly as the reference in every `I2C1` register. The reference's own transfer leaves `ISR` at its reset value, `0x00000001`: it consumes `RXNE` and `RXDR`, and the board's `ICR` write clears `STOPF`.
  - `forge-all`'s `RXDR` write is ignored, because `RXDR` is read-only.
  - `wrong-addr`'s raw read lost its `END` marker to a warning on one-level console runs (see S4). Its `CR2` and `TXDR` are the CLI verdict's.
- **The two observables outside the register file.** They are read with `logLevel -1 sysbus.i2c1.u2` and `sysbus LogPeripheralAccess sysbus.i2c1 true`, set before the run.
  - **The reference:**
    - `[NOISY] i2c1.u2: Write D0` and `[NOISY] i2c1.u2: Read 60`, once each;
    - in the access log, `[INFO] i2c1: [cpu: 0x80005D4] ReadUInt32 from 0x24 (ReceiveData), returned 0x60`, preceded by `WriteUInt32 to 0x4 (Control2), value 0x20124EC` (`SADD` 0x76, `RD_WRN`, `START`, `AUTOEND`).
  - **Every forgery:** none of either. The access log shows instead what the forgery wrote (`WriteUInt32 to 0x24 (ReceiveData), value 0x60` for `forge-all`).
  - **`wrong-addr`:** `Unknown slave at address 119.`, then the `ISR` polls folded as `returned 0x1 (10000)` and `(9999)`, and no device line.

**Each candidate, judged:**

| Candidate | After the reference | Can a store set it, or is it otherwise useless? | How known |
|---|---|---|---|
| `ISR.TXE` | 1 | always reads 1 (`STM32F7_I2C.cs` L230) | read; measured (1 in every row) |
| `ISR.TXIS` | 0 | writable with `NOSTRETCH` and `TXIE` (L242–246); 0 after the reference | read |
| `ISR.RXNE` | 0 | not writable (L248), but **consumed** by the firmware's own `RXDR` read | read; measured (False in every row) |
| `ISR.TC` | 0 | read-only, cleared by the read phase's `START` (`StartTransfer`) | read; measured (`ISR` 1) |
| `ISR.STOPF` | 0 | read-only, but set by a `STOP`-only `CR2` write (`StopTransfer`) and cleared by `ICR.STOPCF`, which the board writes (L402) | read; the STOP-only set is **not observed** (`forge-all` clears it at once) |
| `ISR.NACKF`, `ISR.BUSY` | 0 | tags: never modelled (L250, L261) | read |
| `RXDR` | empty | read-only (L297) and filled only by the device, but **consumed** by the firmware | read; measured (`0x0` and underflow in every row) |
| transfer counter | — | none in the controller or in `BME280.cs` | read |
| `CR2`, `TXDR` | `0x020104EC`, `0xD0` | read-write | measured (`forge-regs`, `forge-all`) |
| the model's own `Write`/`Read` log lines | present | **no store produces them** | measured |
| the access log's `ReceiveData … returned 0x60` | present | **no store produces it**; `RXDR` is filled only from the device | measured |

### S2 — measured (`out/s2.jsonl`; witnesses read from a log file, S4)

- **`two-bme.json`:** validates, solves and generates. The platform has `u2` and `u5`, both `I2C.BME280`. The generated firmware reads `U2` then `U5`.
- **Today's judge passes all three two-device firmwares,** and witnesses only `U5` (`CR2` `0x020104EE`):
  - `two-ref`;
  - `two-forged-first`, where `U2`'s read is a claim;
  - `two-forged-last`, where `U5`'s read is a claim and its `CR2`/`TXDR` are hand-written.
- **The device lines, three runs each:**

  | Firmware | `u2` Write D0 / Read 60 | `u5` Write D0 / Read 60 | access-log `returned 0x60` |
  |---|---|---|---|
  | `two-ref` | 1 / 1 | 1 / 1 | 2 |
  | `two-forged-first` | **0 / 0** | 1 / 1 | 1 |
  | `two-forged-last` | 1 / 1 | **0 / 0** | 1 |

- **Cost per identity read, measured from the file:** 1,102 bytes for both channels, of which the two device lines are about 84 (two-device 4,317 bytes against one-device 3,215). Wall time 1.83 s with the witnesses against 1.82 s without, which is inside run-to-run noise.
- **The judge's own monitor reads appear in the access log,** with no `[cpu: …]` prefix (P2.5).
- **Renode folds a repeated identical line** into one with a ` (N)` count. A counting witness must sum the counts. The fold is Renode's `[general] collapse-repeated-log-entries`, on by default (`Logger.cs` L824); turning it off was not measured.
- **The same witnesses on the console, with synchronous logging** (S4's recommended composition; `out/s4e.jsonl`, three runs each):
  - every real read gives one `Write D0`…`Read 60` pair per device: `two-ref` 1 + 1, `ref` 1;
  - each forged device has none (`two-forged-first`: `u5` only; `two-forged-last`: `u2` only; `forge-all`: none);
  - the judge's own access-logged reads land after `BEGIN` as whole lines;
  - `ref`'s reads hash `b5b77d13…`, as today's.

**The work order's three candidates, disposed:**
1. **The BME280 model's own state through the monitor.** It is not readable: `selectedRegister` and `state` are private (`BME280.cs` L288–289), and the model counts nothing. Its public `Read` would itself mutate the pointer. **Read**, not run. Its `Noisy` log lines carry the same evidence, and they are what was measured.
2. **The controller's access log.** **Measured**, above.
3. **A per-transaction record judged against controller state captured by a monitor hook.** **Not measured.** The model's lines already witness each transaction, and a hook adds a Python path inside Renode.

### S3 — measured (`out/s3.jsonl`; ELF bytes from `objdump -s`)

**The reference ELF:**
- `main` is `[0x08000098, 0x08000598)`. It ends with `str r0,[r1]` (the boot word, `BOOT_MAGIC` built just before) at `0x08000592`, then `yield` (`0xBF10`) at `0x08000594` and `b.n 0x08000594` (`0xE7FD`) at `0x08000596`: **the fixed point is the last 4 bytes of `main`.**
- `Board::take`'s `else` is a second, separate `yield; b.n` at `0x0800055A`, reached by a `beq.w` at `0x080000A4`.
- `fault` is a third, at `0x08000094`.

**The two-device ELF:** `main` is `[0x0800019C, 0x08000616)`, and its last 4 bytes are again the loop, at `0x08000612`.

| Run | Final `PC` | `ICSR` | `VECTACTIVE` | `CPSR` (xPSR) |
|---|---|---|---|---|
| ref 10 ms | `0x08000594` | `0x00000800` | 0 | `0x21000000` |
| ref 10.00001 ms | `0x08000596` | `0x00000800` | 0 | `0x21000000` |
| ref 10.00002 ms / 10.00003 ms | `0x08000594` / `0x08000596` | `0x00000800` | 0 | `0x21000000` |
| ref 20 ms, 100 ms | `0x08000594` | `0x00000800` | 0 | `0x21000000` |
| two-ref 10 ms | `0x08000614` | — | — | — |
| `systick-before` 10 ms, 100 ms | `0x08000096` (`fault`) | `0x0400080F` | **15** | `0x2100000F` (`IPSR` 15) |

`systick-before` writes its boot word and record, and executes MIPS × time instructions, exactly as the reference does.

**The ELF the pipeline judges has no symbols.**
- `aethelgard build` adds `--config profile.release.strip="symbols"` (`crates/aethelgard-cli/src/firmware.rs` L37, from `AET-WO-3301`, for path independence).
- Measured on the reference, built by `aethelgard build --crate boards/sentinel-01` in the image (`s3_stripped.sh`, `out/s3-stripped.txt`; ELF sha256 `2d9ee2641f6e…`, the reference ELF `AET-WO-3301` recorded):
  - `nm` reports `no symbols`;
  - the entry point is the same, `0x08000041`;
  - `.text` is **byte-identical** to the cargo build's (1,454 bytes at `0x08000040`);
  - the boot ends at the same `PC`, `0x08000594`, with `VECTACTIVE` 0.
- So "the last 4 bytes of `main`" is available only on an unstripped ELF: the tier-2 tests' cargo builds, not what `build` hands `verify`.
- Without symbols, the ELF still holds:
  - the entry point;
  - the bytes at the final `PC`;
  - one occurrence in `.text` of the fixed point's tail: `movw r0,#0xB007; movt r0,#0x2100; str r0,[r1]; yield; b.n` at `0x08000588` to `0x08000598`.

  That last sequence is the generator's codegen, not a symbol.

### S4 — measured (`out/s4.jsonl`, `s4b`, `s4c`, `s4d`, `s4e`, `s4-marker.txt`; and `s3.jsonl` for the judge's own reads)

**Four compositions, ten runs each.**
- **cli:** today's tail, `RunFor; logLevel 3`.
- **onelevel:** no level change.
- **file:** `logFile` first and `logLevel 3 console`, both before anything runs.
- **sync:** onelevel, with Renode's `[general] use-synchronous-logging = true` in the run's config file.

"Lines" counts each warning kind with the folds summed.

**What `use-synchronous-logging` does** (`Logger.cs` L822–834): every entry is written by the thread that logs it, with no queue and no log thread. The CLI runs Renode with `HOME` set to its scratch directory and no config file, so today's logging is asynchronous. `renode --config <path>` takes a config file.

| Firmware (what it logs) | cli | onelevel | file | sync |
|---|---|---|---|---|
| `ref` | reads hash `b5b77d13…`, 10/10 | same hash, 10/10 | same hash, 10/10 | same hash, 10/10; 1.80 s |
| `burst` (500 identical warnings, folded into one line, then 1 locked) | 500 + 1, 10/10 | 500 + 1, 10/10 | 500 + 1, 10/10 | — |
| `flood` (paced, 167 + 167) | complete 10/10 | complete 10/10 | complete 10/10 | — |
| `flood-fast` (15,607 + 15,607; 2.9 MB) | complete 10/10 | complete 10/10 | complete 10/10 | — |
| `flood-max` (332,937 + 332,936; 62 MB) | **3,446 to 3,627 lost per kind**, varying; reads parse 10/10 | all lines 10/10; **`BEGIN` unparsable 7/10** | all lines 10/10; reads 10/10; file 62.9 MB | **all lines, markers and values whole, 10/10**; 4.2 s against 3.8 s |
| `flood-locked` (199,761 + 199,761, one locked read about 600 lines before the end; 37.7 MB) | **locked line lost 7/10**; 0 to 3,221 lost | locked line 10/10 | locked line 10/10 | **locked line 10/10**, all lines |
| ref at 1 µs (the **judge's own** reads hit gated `I2C1`/GPIO) | 0 lines shown; reads clean 10/10 | 2 locked lines **in the read block**; **a value line split 7/10** | 2 lines in the file; reads clean 10/10 | **2 whole lines after `BEGIN`** (lines 44 and 47; `BEGIN` is 16); reads clean 10/10 |

- **Why the loss is bounded.** Renode's asynchronous queue holds 10,000 entries (`Logger.cs` L832, `BlockingCollection<LogEntry>(10000)`). The largest loss measured was about 7,250 lines (two kinds), under that. That fits B10's lines being whatever was still queued at the level change. **Inferred**, not shown run by run.
- **The threshold is not located.** No loss at 2.9 MB (31k lines). Loss at 37.7 MB (`flood-locked`) and 62 MB (`flood-max`). Nothing between was run.

- **The marker vanishes at character level.** `s4-marker.txt` is ten more `flood-max` runs at one level. In 8 of 10, `AETHELGARD-BOOT-BEGIN` was scattered one or a few characters at a time across consecutive log lines: `A[…]`, `E[…]`, `TH[…]`, `EL[…]`, `GA[…]`, `RD-B[…]`, `OO[…]`, `T-BEGIN[…]`. The monitor's output and Renode's log thread interleave below line level. A value line splits the same way: `0x000000[01:21:37.0167] [WARNING] sysbus: Tried to read a locked peripheral…`.
- **Today's `simulate` on `flood-locked`, ten runs:** `fail` with `locked_peripheral_access` and `renode_log_flood`, **10/10**. The loss that the raw reader shows did not happen through the CLI's own reader. The difference is measured; its cause (for example the reader's pace, or the sandbox) is **not established**.
- **An in-stream marker separates the run's lines from the judge's.** `log "AETHELGARD-RUN-END"`, sent right after `RunFor` in the file composition, goes through the same queue as the firmware's lines.
  - ref at 1 µs: the judge's own 2 locked lines fell **after** it, 10/10.
  - `flood-locked`: the firmware's locked line fell **before** it, 10/10.

  (`LogCommand.cs` L33–35: `log "<message>"` logs at `Info`.)

**Not measured for any composition:**
- inside the CLI's sandbox. All the S4 runs used `drive.py`'s own environment. The `RLIMIT_FSIZE` concern below is read from `run.rs` (`AET-WO-2301`), not run.
- a boot killed at a bound.
- an aborted CPU, where `RunFor` never returns and no marker follows. `This machine has aborted…` is an `ErrorLog` (`Machine.cs` L1235), so it reaches a console at `logLevel 3` in every composition.

### S5 — measured (`out/s5.jsonl`)

**The name set.** Read from the image's own files, whose hashes equal `RENODE_FILES`:
- `stm32h753.repl` (`13c5571e…`) declares `crypto` and `using "./stm32h743.repl"`;
- `stm32h743.repl` (`86b2c2c5…`) has only an untyped update of `nvic`, and `using "platforms/cpus/stm32h7.repl"`;
- `stm32h7.repl` (`b85cb4af…`) declares 96 typed instances, and an untyped `sysbus:` update.

That is **97 instance names**, plus the builtins `machine` and `sysbus`, plus the 13 keywords. A line-regex also matches `else:` inside the `.repl`'s embedded Python peripheral script (L744, L762), which is not an entry. Several names are mixed-case (`nvicInput23`, `gpioPortA`, `canMessageRAM`). The emitter lowercases every reference, and Renode compares case-sensitively, so only the all-lowercase names can collide.

**Each reference given to `U2`:**

| Reference | `generate` | Emitted line | `simulate --platform` (load) |
|---|---|---|---|
| `nvic` | exit 0, output written | `nvic: I2C.BME280 @ i2c1 0x76` | `platform_load_error`: `Error E02: Variable 'nvic' was already declared, previous declaration was at :6:1.` (the boot too) |
| `cpu`, `rcc`, `exti`, `timer2`, `usart1`, `dwt`, `crypto` | exit 0 | itself | `platform_load_error`, E02 "already declared", at `:1:1`, `:513:1`, `:13:1`, `:454:1`, `:298:1`, `:10:1`, `:3:1` |
| `sysbus`, `machine` | exit 0 | itself | `platform_load_error`, E02 "…already declared as an already registered peripheral or buil[tin]" |
| `using`, `init`, `as`, `none`, `true` | exit 0 | itself | `platform_load_error`: `Error E00: Syntax error, Unexpected using.; expected entry` (and the others alike; the boot of `using` too) |
| `NVIC` | exit 0 | `nvic` | as `nvic` |
| `gpioPortA` | exit 0 | `gpioporta` | **pass**: it loads |
| `i2c1` | **exit 2**, `InvalidDeviceName`, no output directory | — | — |
| `u2` (control) | exit 0 | `u2` | pass |

`generate` builds every file in memory and writes them only at the end (`tree::write`); `i2c1`'s refusal left no directory. A refusal raised where `i2c1`'s is raised is therefore already "before any output".

**The monitor side** (**[read-sub]**: the source-reading subagent, not checked by me).
- The CLI's monitor commands name no device today. It uses `sysbus ReadDoubleWord`/`WriteDoubleWord` and `cpu …` only.
- S1's recommendation would be the first time a reference reaches a monitor command (`logLevel -1 sysbus.<bus>.<ref>`), as a path component after `sysbus.`. That position is not a command name. With S5's refusal, no keyword or instance name can get there.

## The spike's table: each prediction beside its result

| Id | Prediction | Result | Held? |
|---|---|---|---|
| P1.1 | ref leaves `CR1` 1, `CR2` `0x020104EC`, `TXDR` `0xD0`, `ISR` 1, RXNE False; `RXDR` read 0 with underflow | exactly so | yes |
| P1.2 | `forge-all` reads identically; today passes | identically; pass (also `forge-regs`) | yes |
| P1.3 | wrong address: `Unknown slave at address 119.`; `CR2` `0x000100EE`, `TXDR` 0; fails today; with the registers hand-written, identical and passes | exactly so | yes |
| P1.4 | §7's first condition holds: no register witness | holds | yes |
| P1.5 | device lines `Write D0`/`Read 60` named `i2c1.u2`, and the access log's `returned 0x60`, only on a real transfer | exactly so; none for any forgery | yes |
| P2.1 | two-device spec generates; today passes, witnesses only `U5` | so | yes |
| P2.2 | forged-first and forged-last both pass today | both pass | yes |
| P2.3 | 2 device lines per read; the access log 8 to 14 lines | 2 device lines; access log 14 CPU lines for one read (19 with the judge's reads) | device yes; access at the top of the range |
| P2.4 | about 150 B (device lines) and 1.5 KiB (access log) per read; under 50 ms | about 84 B and about 1 KiB; wall time within noise | yes, lower |
| P2.5 | the judge's own reads in the access log, no `[cpu:]` prefix | so | yes |
| P2.6 | device lines depend on the model; the access log does not | by construction (the source); not run on a second model | read, not run |
| P3.1 | fixed point = `yield; b.n` at the end of `main`; the `take` `else` loop separate (60 %) | `[0x08000594, 0x08000598)`; `else` loop at `0x0800055A` | yes |
| P3.2 | ref `PC` alternates in the loop; `systick-before` `0x08000096` | so | yes |
| P3.3 | `VECTACTIVE` 0 / 15; `CPSR` `IPSR` = `VECTACTIVE` | 0 / 15; `0x21000000` / `0x2100000F` | yes |
| P3.4 | known from the ELF alone through `main`'s symbol, no generator change | true of the cargo build; **`aethelgard build`'s ELF is stripped**: no `main` symbol | **partly** |
| P4.1 | before `logLevel 3`, the console prints warnings | so | yes |
| P4.2 | today's tail loses lines on a 501-line burst, sometimes the locked one | **no**: no loss on the burst, the paced flood or a 2.9 MB flood. Loss at 37.7 MB and 62 MB with a raw reader; none through today's `simulate` in 10/10 | **no** |
| P4.3 | one level: all lines, byte-exact reads, listing parses | all lines, yes; **but** the `BEGIN` marker scattered 7/10 and 8/10 under `flood-max`, and the judge's own locked reads split a value 7/10 | **no** (holds only with synchronous logging) |
| P4.4 | between markers only bare values or whole log lines | **no** with asynchronous logging (fragments); yes with synchronous, 10/10 | **no** as predicted |
| P5.1 | about 100 names, static and pinned | 97 instance names, 2 builtins, 13 keywords; pinned by hash | yes |
| P5.2 | per-reference outcomes as tabled | every row as predicted, with the measured messages | yes |
| P5.3 | a prefix changes the committed `.repl` | not run; by the fence | read |

## Recommendations, one per question

- **S1. The witness is the device model's own log of the transaction, not a register.**
  - **What it reads:** each modelled identity read must show, among the log lines before `BEGIN` (S4), `[NOISY] <bus>.<ref>: Write <register>` and then `[NOISY] <bus>.<ref>: Read <expected bytes>`, for that device's emitted name.
  - **How it is set up:** the judge sets `logLevel -1 sysbus.<bus>.<ref>` for each modelled device before the run.
  - **Why it holds:** only the controller's `StartTransfer` to that device's address calls the model's `Read`, so no store can produce the line. Measured: the reference has it, and every forgery lacks it, on a file and on a synchronous console.
  - **The pinning test:** the A11.1 test is inverted on this, with a finding that names the device and the absent line.
  - **The `CR2`/`TXDR` witness** adds nothing against a forgery (P1.2). A wrong address is also caught by the device line, and by "Unknown slave". Recommend dropping it at the build (D7).
- **S2. The same witness, one per identity read.**
  - Each check is judged by its own device's line pair, in record order, so every transaction is witnessed, not only the last on a bus.
  - `two-bme.json`'s shape is the test fixture for the second device. It is a test fixture, not a change to the reference specification.
  - Where a future model (`P4`'s IMU) logs nothing, the controller's access log is the model-independent fallback: a `[cpu: …]` `CR2` write with the device's `SADD` and `START`, then `ReceiveData … returned <byte>`. It costs about 12 times the bytes (D2).
- **S3. The judge requires the core parked in thread mode at the fixed point. Two readings, both valid on a stripped ELF:**
  1. **`ICSR.VECTACTIVE` is 0.** This is Renode's NVIC state, not the firmware's claim: `sysbus ReadDoubleWord 0xE000ED04`, under mask `0x1FF`. `systick-before` fails here (15).
  2. **The final `PC` is on a two-instruction self-loop, `yield` then `b.n` back to it** (`0xBF10`, `0xE7FD`), read from the ELF's own loaded bytes at `PC` and `PC` − 2. Measured on both builds: the reference ends at `0x08000594` or `0x08000596`, both on the loop.
  - With the boot word required (as today), this excludes:
    - `fault`, which `VECTACTIVE` rejects;
    - the `Board::take` `else` loop, where the boot word is unwritten.
  - The residual (the build's adversarial §5.3): a firmware that writes the boot word and then parks in a different thread-mode self-loop.
  - Pinning it to *the* fixed point on a stripped ELF needs either:
    - the codegen pattern `movw/movt BOOT_MAGIC; str; yield; b.n` (once in `.text`; brittle across compilers);
    - or a marker the generator emits that survives `strip`, which is a generator change and so §7.2 (D3).
- **S4. Synchronous logging, one console level, `BEGIN` as the boundary.**
  - **The composition:**
    - the run's Renode config (`--config <scratch>/renode.config`, holding `[general]` `use-synchronous-logging = true` and the history path);
    - no `logLevel` command at all.
  - **What the judge reads where:**
    - log lines before `BEGIN` are the firmware's run: locked access, errors, the S1 witness;
    - whole log lines between the markers are the judge's own reads, and are skipped;
    - anything else between the markers is refused, as the droop probes do now.
  - **Measured, 10/10 each:**
    - every line arrived on `flood-max`, 665,873 lines, with markers and values whole;
    - the late locked line arrived on `flood-locked`;
    - the judge's own locked reads were whole lines after `BEGIN`;
    - the reference's reads were byte-identical to today's.
  - **Cost:** wall time +9 % on the 62 MB flood; none measurable on the reference.
  - **The flood path stays the console's,** so `LOG_FILTER`'s budget and first-of-kind rule still apply, and there is no file to meet the sandbox's 64 MiB `RLIMIT_FSIZE`.
  - **The alternative:** the file composition with a `log` marker, also 10/10 on every fixture, but bounded by that file cap (`flood-max` wrote 62.9 MB in 10 ms) (D4).
  - **Not run:** inside the CLI's sandbox, at a bound kill, or on an aborted CPU. The build's tier-2 tests must cover those.
- **S5. Refuse the reference before any output.**
  - **The rule:** a reference whose emitted (lowercased) name equals an all-lowercase stock instance name, a builtin (`machine`, `sysbus`) or a keyword is refused.
  - **Where:** the set is static and pinned by hash, so by the work order's rule the refusal belongs in `validate`, as a constant list in `aethelgard-ir` with a tier-1 twin test that parses the pinned chain's entries (a parser, not a line regex: `else:`) and asserts the list equal (D5).
  - **The existing check:** `split_devices`' check stays; it already refuses before output (`i2c1`).

## §7 checked

- **§7.1 holds, measured: no `I2C1` register in Renode 1.17.0 is both unforgeable and present after a real transfer.**
  - The spike stops here anyway.
  - The remedy measured is a Renode-recorded event: the device model's log line (S1).
  - Whether a log-derived witness is acceptable in place of a register is **D1**.
- **§7.2 holds in part.**
  - On an unstripped ELF, the fixed point is `main`'s last 4 bytes.
  - On the stripped ELF `aethelgard build` produces, which is what `verify` judges, the ELF gives only "a `yield; b.n` self-loop at `PC`" (S3). *The* fixed point would need the codegen pattern or a generator change.
  - Which of these the build uses is **D3**.
- **§7.3 is not reached by the spike.** Reads were byte-identical across compositions on the reference. The verdict-level identity is the build's test.

## Findings

- **F1 · §7.1 holds: the reference's transfer leaves no register a store cannot set · `STM32F7_I2C.cs`, `board.rs` L402 · high · measured (S1).** The transfer consumes `RXNE`/`RXDR` and clears `STOPF`, so the final state is the reset state plus `CR2`/`TXDR`, all writable. Recommendation: S1. Decision needed: D1.
- **F2 · With asynchronous logging, Renode's console interleaves the monitor's output and its log thread below line level · Renode 1.17.0 · high for any composition that prints log lines while reading · measured (S4).**
  - `BEGIN` was scattered 8/10, and values split 7/10.
  - A whole-line skip cannot recover this; it can only refuse (exit 5).
  - This affects the work order's proposed §2.4 (one console level) as written.
  - Synchronous logging removes it (10/10). Recommendation: S4.
- **F3 · The judge's own monitor reads log too · `renode.rs` reads · medium · measured.**
  - Reads of a gated peripheral log `Tried to read a locked peripheral` (ref at 1 µs, 2 lines), and with the access log on every read logs.
  - Today's `logLevel 3` hides them. A lower console level must treat lines after `BEGIN` as the judge's, or they are attributed to the firmware as `locked_peripheral_access`.
  - Recommendation: S4's boundary.
- **F4 · B10 reproduced with a raw reader, not through `simulate` · today's static tail · medium · measured.**
  - A raw pipe lost 3,446 to 3,627 lines per kind (`flood-max`), and the late locked line 7/10 (`flood-locked`).
  - Today's `simulate` caught that line 10/10. Why the CLI's path did not lose it is not established.
  - No loss at 2.9 MB. Between 2.9 and 37.7 MB nothing was run.
  - The loss stayed under Renode's 10,000-entry queue (inferred bound).
  - B10 stands as a Renode behaviour.
- **F5 · Renode folds identical consecutive log lines into one, ` (N)` · Renode 1.17.0, `collapse-repeated-log-entries` · low · measured** (`returned 0x1 (10000)`; 500 underflows in one line). A count-based witness must sum the folds, or the config can turn folding off (not measured).
- **F6 · The droop probes (2301 A2) keep one console level and skip whole log lines · `renode.rs` `probes_in` · medium · REASONED from F2, not run on the droop path.**
  - Under a flood with asynchronous logging, a probe's marker or value can scatter. That gives exit 5, never a false value.
  - The same synchronous-logging config removes it, if the bridge's Renode gets it too.
- **F7 · `split_devices`' taken-set compares the lowercased name with mixed-case stock names · `platform.rs` L128 · low · measured.** `gpioPortA` emits `gpioporta`, which Renode treats as distinct, and the platform loads. Harmless today; S5's list subsumes it.
- **F8 · Three identical `yield; b.n` loops in the generated firmware · `main.rs` (`fault`, `Board::take`'s `else`, the fixed point) · low · measured.** "`PC` on a self-loop" needs `VECTACTIVE` and the boot word to tell them apart (S3).
- **F9 · `aethelgard build` strips symbols, so the judged ELF carries none · `firmware.rs` L37 · medium for S3 · measured.** `.text` is byte-identical to the cargo build's. Anything the judge takes from the symbol table (S3's first idea) is unavailable on `verify`'s path.

## Decisions needed

- **D1 (§7.1): accept a log-derived witness in place of a register.** It is the model's `Noisy` line. Recommend yes: it is what Renode records, not what the firmware writes (`INV-07`).
- **D2 (S2): which channel carries the witness.**
  - **(a)** the device-model lines only. Recommended: about 84 B per read, names the device.
  - **(b)** the controller access log only: model-independent, about 1 KiB per read.
  - **(c)** both, (b) only for a model that does not log.
- **D3 (S3, §7.2 in part): what "at the fixed point" means for a stripped ELF.**
  - **(a)** `VECTACTIVE` = 0, plus `PC` on a `yield; b.n` self-loop read from the ELF's bytes. Recommended: no generator change, valid on both builds. Its residual is any other thread-mode self-loop after the boot word.
  - **(b)** (a) plus the codegen pattern `movw/movt BOOT_MAGIC; str; yield; b.n` located once in `.text`: exact today, brittle across toolchains.
  - **(c)** a generator-emitted marker that survives `strip` (for example a dedicated output section in `link.x`): exact, but a generator change, so the work order's §7 stop.
- **D4 (S4): the composition.**
  - **(a)** synchronous logging (`--config`), one console level, `BEGIN` as the boundary. Recommended: measured 10/10 on every fixture, keeps the console's flood handling, no file cap.
  - **(b)** a log file with a `log` marker: 10/10, but needs a flood bound against the 64 MiB `RLIMIT_FSIZE`.
  - **(c)** the work order's one level, asynchronous: measured unsafe (F2).

  Also: whether 2301's droop probes get the same config (F6) in this work order or later.
- **D5 (S5): where the refusal lives.**
  - **(a)** `validate`, with the list in `aethelgard-ir` and a twin test holding it equal to the pinned files. Recommended, by the work order's rule.
  - **(b)** the twin's `split_devices`, which keeps Renode facts out of the IR and is also before any output.
- **D6: the trace version.** The witness's record (kind 13) changes from `CR2`/`TXDR` words to per-read log evidence, so the trace version rises once. `AET-WO-2205` owns the record renames, and **one bump per work order** means the two must not both bump for the same change. Which work order bumps first?
- **D7: drop the `CR2`/`TXDR` witness at the build,** since the device line subsumes it (S1). Recommended: yes, in the reduction pass.
- **D8 (§4): the acceptance test "a line the old tail dropped now reaches the judge".** Through `simulate`, the old tail dropped nothing in 10/10 (F4). Candidates:
  - **(a)** the judge's own locked reads at a short run, which today's tail hides and which must now land after `BEGIN`, not among the firmware's lines (measured 10/10);
  - **(b)** the raw-reader `flood-locked` case at the Renode level;
  - **(c)** drop the clause and keep F2's character-level case as the test (`flood-max`: markers and values whole).

## Gates (spike)

- The diff against `main` is this file and `spike/2204/`: Python, shell, JSON and outputs. No Rust changed.
- `cargo fmt`, `clippy` and the tests are unaffected; CI runs them on the PR.
- `docs/work-orders/AET-WO-2204.md` reached `main` through PR #110 (`39d7221`), which is merged into this branch. It is identical to the copy this spike worked from (`claude/argon-repo-docs-synthesis-rjrb0d`).

---

# The build (§2), to the architect's ruling on PR #111 (comment 6870064439)

**Ruling:** D1 yes (exact whole-line match; matched values), D2 (a) (device lines only; a model with no format is a named failure; the `LOG_FILTER` rule stated and tested), D3 (a), D4 (a) for every Renode the CLI starts, D5 (a), D6 (2204 bumps 4 → 5), D7 yes, D8 (a) plus one `flood-locked` test. Binding: no Python to `main`, the second device a test fixture only, the reference's static verdicts byte-identical but for the named kind-13 fields, the full adversarial pass with item 4's additions, `TIER2_MIN` and weights, the gates, then FEEDBACK and the reduction pass.

**One operator ruling during the build (2026-11-14), answered in the session:** D7 drops the `CR2`/`TXDR` witness, but 2301's droop probe (kind 19) read `CR2`/`TXDR` too, and replay pinned kind 19's readings to kind 13's (`aethelgard-chaos` `campaign.rs`, `check_observed_against_the_boot`). The fence allowed chaos changes "only for kind 13 and the trace version". The operator chose **device lines for the probe too**, extending the fence to kind 19 (one trace bump covers both).

## Before any code: measured

- **The baseline** (`~/.cache/aet-2204/baseline/main`, main's CLI and bridge at `49f6b34`, sha256 `221c3eed…`/`44abce24…`): all 13 scenarios (2 reference static, 2 reference droop, 9 seeded) matched, every archive replayed, and the static `simulate` at 100 ms passed. Compared with the build's below.
- **Synchronous logging across a reset and several `RunFor` segments** (design probe, then pinned by the droop tests): a device's `Noisy` level survives `machine Reset` (a second `Write D0`/`Read 60` after the release); every log line falls between the markers of the segment that caused it.
- **What `--config` changes** (pinned by `twin_tier2::renode_s_configuration_differs_from_its_default_in_synchronous_logging_alone`): Renode's own default file, after a whole boot, holds 19 keys; the CLI's differs in `use-synchronous-logging` alone. The history path (`$HOME/.config/renode/history`) and `collapse-repeated-log-entries = True` are as today. Renode leaves `renode.config.lock` beside it, in the call's scratch directory (the tool may add entries there). A first probe's read of the default was cut at 500 characters and gave a wrong `min-tb-size` and no `max-tb-size`; the measurement above is the whole file.
- **The spike's outputs**, deleted with `spike/` at this build (sha256 as committed at `49f6b34`, first 12): `caps.log` `34a8de48ca3d`, `s1.jsonl` `b7791eca2dd3`, `s2.jsonl` `dbec57b899e1`, `s3.jsonl` `91db6fd22ef4`, `s3-stripped.txt` `3fa94096f599`, `s4.jsonl` `c207838158da`, `s4b.jsonl` `40a84a7715d9`, `s4c.jsonl` `b7c41a0f912b`, `s4d.jsonl` `b737d515fdf0`, `s4e.jsonl` `18f8f765950e`, `s4-marker.txt` `affb3884d323`, `s5.jsonl` `63c3b21f11c5`.

## What was built

**The composition (D4).** Every Renode the CLI starts (the version query, a platform load, a static boot, and a droop's boot through `bridge --renode`, which passes the CLI's own arguments) gets `--config <scratch>/renode.config` (`renode::config`). The file states every key Renode 1.17.0 reads, each at the value Renode writes for itself when given no file, except `use-synchronous-logging = true`; the history stays under the call's home, as today.
- **Why every key (a deviation from "a file with one key", measured):** Renode writes a key it finds missing back into the file it was given (`ConfigurationManager.Get` → `Set` → `Save`). In the image the CLI runs as root and the tool as uid 65534, and `AET-WO-1601`'s handover seals every file the CLI wrote read-only. Given a one-key file, every boot through the CLI aborted (`renode was killed by signal 6`; Renode: `Access to the path …/renode.config is denied`, from `TranslationCPU.Init` reading `[translation] max-tb-size`). The spike never ran inside the sandbox (it said so), which is why it did not see this. With every key stated, Renode only reads the file, and the seal holds. `twin_tier2::renode_s_configuration_differs_from_its_default_in_synchronous_logging_alone` pins Renode's own default (19 keys, measured after a whole boot including a reset) and that the stated file is left byte-identical after a boot; a renode.rs unit test pins the CLI's text to it. A key Renode reads that the file lacks fails loudly (exit 5), never silently. `RENODE_CI_MODE=YES`, which also stops the write (`Emulator.cs` L147), was rejected: it changes Renode's behaviour beyond the configuration. No `logLevel` command changes the console. The one `logLevel` kept is `logLevel -1 sysbus.<bus>.<device>`, set once before the run for each modelled device the booted platform declares: a source's level, which D1's witness needs, not the console's (a faulted bus leaves its devices off the platform, and a level on no element is a failed command). Measured on Renode: no line is dropped and none is split by it.

**The boundary (D4, F3).** Every log line before `BEGIN` is the run's; between `BEGIN` and `END` only bare values and whole log lines (a host timestamp, then a level) are allowed, the log lines being the judge's own reads' and never attributed; anything else is exit 5. A droop prints `AETHELGARD-RELEASE-k` at each release; the probes' own blocks are the judge's reads too. `locked_peripheral_access` and `renode_error_log` read only the run's log.

**The witness (D1, D2, D7).** `aethelgard-twin` computes, per identity read, the device's Renode path (`i2c1.u2`: the controller's stock name, then the emitted name) and the model's whole log lines, by an exhaustive `match` on `model::Model` (`witness_lines`; BME280: `[NOISY] i2c1.u2: Write D0`, `[NOISY] i2c1.u2: Read 60`, bytes upper-case hexadecimal without padding, as `BME280.cs` L38/L75 log them). The judge (`renode::witness_findings`):
- matches each line as the entire text after the host timestamp (`log_body`: `[hh:mm:ss.ffff] ` exactly, nothing trimmed), never `contains` or a prefix; a line with a witness's shape (`[NOISY] <device>: `) is collected only so that an extra, folded (` (N)`) or reordered line fails;
- requires, for each record whose status is OK, that the device's lines since the last release are exactly the expected ones; the `Read` line carries the table's value, and `i2c_identity_mismatch` already requires the record's value to be that, so a real transfer under a forged record fails;
- requires the devices' lines in the records' order (`i2c_witness_out_of_order`, its own kind: it belongs to no single read);
- names a model with no witness format `i2c_witness_unavailable` (never skipped).
- **`LOG_FILTER` (D2):** past the 8 MiB budget a log line may have been dropped, so every OK record is `i2c_transaction_unwitnessed` ("the log passed the filter's budget"), never a pass. Tested in tier 1 (`a_log_past_the_filter_s_budget_leaves_every_read_unwitnessed`) and on Renode (`a_locked_access_late_in_a_flood_reaches_the_judge_whole`).
- The `CR2`/`TXDR` witness is gone: the reads, the twin's `I2cWitness`, `fields`, `differing`, the header widths it used, and in evolve `Joined.cr2_at`/`txdr_at`, `SADD_BITS`, `NBYTES_BITS`, `TXDR_DATA`, `WROTE_TXDR`, `READ_PHASE`, `Run.sadd`/`nbytes` and `Bus.peripheral`.

**The droop probe (kind 19, the operator's ruling).** The probe reads the boot word and the first record, and its witness is the first device's lines from `RELEASE-k` to `PROBE-BEGIN-k`. Recovered means the boot word, an OK record with the table's value, and exactly the model's lines since the release. 2301's A1 (`fw-late`) holds: a firmware that claims OK before it talks to the bus has no lines in that window.

**The core parked (D3).** The reads add `ICSR` (`0xE000ED04`). After the boot word, `VECTACTIVE` (mask `0x1FF`) must be 0 (`core_in_exception`, naming the exception) and the final `PC` must be on a `yield; b.n` self-loop read from the ELF's executable `PT_LOAD` bytes (`elf::Image::self_loops`; `pc_not_on_self_loop`). `pc_outside_firmware` takes precedence. **Named residual, in the finding's documentation (`boot::YIELD`) and here:** a pass claims "parked in thread mode on a self-loop after the boot word", not "at the generator's fixed point"; a firmware that writes the boot word and then parks on another thread-mode self-loop passes (`RUL-19`). Valid on the stripped ELF `aethelgard build` gives (`board_tier2`'s `verify` path boots it).

**Reserved names (D5).** One list, `aethelgard_ir::RESERVED_REFERENCES` (81: the 68 all-lowercase instance names of the pinned chain, `machine`, `sysbus`, and the 11 keywords lowercased), refused by the Reference identifier rule on the lowercased reference, for I2C and SPI references and `SolvedTopology::validate`. The tier-1 test `aethelgard-twin/tests/reserved_names.rs` parses the vendored, hash-checked chain (`tests/renode-1.17.0/`, with Renode's licence) with a parser that tracks `'''` blocks (the `else:` lines inside the embedded Python peripheral scripts are not entries) and asserts the list equal. `split_devices` no longer seeds its taken set from `STOCK`; a test pins every all-lowercase `STOCK` name into the list (one list, F7). **Measured:** `U2`/`u2` are refused by `Specification::validate` (`DuplicateReference`, exit 2, no output directory), and `split_devices` alone also catches them (`InvalidDeviceName`), since `SolvedTopology::validate` does not fold references.

**Records and the trace (D6).** Trace version 4 → 5. Kind 13 is `{peripheral, reference, device, expected, observed, absent}` (one per identity read; replay requires each identity read witnessed and each witness of a read, and the device path to end in the lowercased reference; the per-controller uniqueness rule is gone, since a bus with two devices has two). Kind 19's `cr2`/`txdr` became `witness`. Observed lines are archived as at most 16, each printable ASCII of at most 256 bytes (escaped and cut otherwise; a changed line still differs, so this cannot make an observation match). Version-4 archives are refused by name (`Fault::Version`), the precedent 2301 set for version 3 (`event.rs` `envelope`); `chaos/tests/archive.rs` asserts 1 to 4 refused, and the golden test that setting the byte back to 4 gives version 4's golden trace (`e9b23970…`).

**Attribution (evolve, fence: where it reads the witness).** A witness yields three fields: `{device}.address` (the addresses the device's controller found no device at, `absent`), `{device}.Write` and `{device}.Read` (the first the model logged). Software evidence: the model logged another register or value; the record says OK and the model logged nothing; or the controller found no device at an address other than the device's. **Why `absent` exists (a deviation, named):** the seeded set's `wrong-address-2k2` and `trap-wrong-address-3k3` are `S1` by the old `CR2.SADD` evidence. A transfer to a wrong address leaves the model's log empty, as a faulted bus does, so device lines alone would have lost them. Renode's controller logs `[WARNING] i2c1: Unknown slave at address 119.` (`STM32F7_I2C.cs` L349) for it, which a firmware writes only by addressing an absent device; on a faulted bus the correct firmware's own address appears there instead, which is not counted, so the 47 kΩ trap stays electrical. It is attribution's evidence only: the verdict's witness is D2's device lines alone.

## Mutants (the guards this build adds, one each, tier 1)

Run by a subagent in its own worktree at `247df35`, every edit verified applied (`git diff`) before its run and reverted after; M4 and M21 re-run by the lead after their tests were added (`2be67ad`).

| | Guard | Edit | Killed by |
|---|---|---|---|
| M1 | `log_body` exact | accept a trimmed line | `boot_tier1::any_witness_but_the_model_s_exact_lines_fails` |
| M2 | `witness_shaped` device boundary | any suffix after the device | `renode::tests::a_log_body_is_exactly_the_text_after_a_whole_host_timestamp` |
| M3 | over the budget is unwitnessed | always read the witness | `boot_tier1::a_log_past_the_filter_s_budget_leaves_every_read_unwitnessed` |
| M4 | the records' order | order never checked | **survived**; then `boot_tier1::two_devices_witness_their_own_reads_in_the_records_order` (added) |
| M5 | `VECTACTIVE` 0 | only above 15 fails | `boot_tier1::a_core_handling_an_exception_fails_naming_vectactive` |
| M6 | both halfwords of the loop | `PC` on the `yield` only | `boot_tier1::a_core_not_on_a_self_loop_fails_and_both_of_its_halfwords_pass` |
| M7 | the run's log ends at `BEGIN` | the whole output | `boot_tier1::the_judge_s_own_log_after_begin_is_never_the_run_s` |
| M8 | refuse a non-line between the markers | never refuse | the same |
| M9 | `use-synchronous-logging = true` | `false` | `boot_tier1` (the stub checks the file) |
| M10 | a level only for a declared device | every check | `renode::tests::a_droop_boot_runs_the_measured_recipe_at_the_schedules_times` |
| M11 | a probe's witness unread past the budget | always read | `renode::tests::a_probe_is_read_only_between_its_own_whole_line_markers` |
| M12 | a probe's window starts at its release | from the start | the same |
| M13 | recovered needs the lines | needs a format only | `campaign_tier1::a_record_that_claims_ok_without_the_device_logging_a_transfer_is_unrecovered` |
| M14 | replay's recovered needs the lines | dropped | `chaos campaign::an_observations_value_and_witness_are_pinned_to_the_check_and_the_witness` |
| M15 | each read witnessed, each witness of a read | dropped | `chaos campaign::each_identity_read_has_a_witness_and_each_witness_a_read` |
| M16 | a witness's path is its device's | dropped | `chaos campaign::a_witness_device_must_end_with_its_references_lowercased` |
| M17 | an absent address other than the device's | any absent address | `evolve attribute::a_transfer_the_controller_found_no_device_for_is_evidence_unless_it_is_the_devices_own` |
| M18 | OK with nothing logged is evidence | never | `evolve attribute::every_witness_line_is_evidence_within_its_width` |
| M19 | the reserved list in `check_identifier` | never reserved | `ir validate::a_solved_topology_refuses_a_reserved_reference` |
| M20 | bytes unpadded, as `BME280.cs` logs them | `{:02X}` | `twin tests::the_witness_lines_are_computed_from_the_part_table_and_the_width` |
| M21 | 16 archived lines | unbounded | **survived**; then `campaign::tests::witness_lines_are_archived_bounded_and_printable` (added) |

## Measured, at the build

**Every measurement the build relies on is a Rust test** (binding item 1; no Python reached `main`):

| Measurement (spike) | Test |
|---|---|
| A11.1 forgery (`CR2`/`TXDR` by hand) fails | `twin_tier2::a_firmware_that_writes_the_controller_registers_by_hand_fails_unwitnessed` (inverted, renamed from `…passes_today_a_named_residual`) |
| a forged record with no transfer fails | `twin_tier2::a_forged_record_without_a_transaction_fails_unwitnessed` (renamed) |
| `forge-all`, `wrong-addr-forge` | `twin_tier2::a_forgery_through_every_register_or_a_wrong_address_fails_unwitnessed` |
| `two-forged-first`, `two-forged-last` (and the real two-device read) | `twin_tier2::on_a_bus_with_two_devices_each_read_is_witnessed` (fixture `tests/fixtures/two-bme.json`) |
| `systick-before` | `twin_tier2::a_firmware_parked_in_its_fault_handler_fails_naming_vectactive` (SysTick armed by the firmware after its boot-word store) |
| the short-run judge reads (D8 a) | `twin_tier2::the_judge_s_own_reads_of_a_gated_peripheral_are_never_the_firmware_s`; deterministic in tier 1: `boot_tier1::the_judge_s_own_log_after_begin_is_never_the_run_s` |
| `flood-locked` (D8) | `twin_tier2::a_locked_access_late_in_a_flood_reaches_the_judge_whole` |
| S5's colliding references | `twin_tier2::a_device_named_for_a_renode_instance_or_keyword_breaks_the_load` (Renode refuses the load), `aethelgard-ir validate` and `aethelgard-cli reserved_references_tier1` (refused before any output), `aethelgard-twin reserved_names` (the list) |
| the configuration's keys | `twin_tier2::renode_s_configuration_differs_from_its_default_in_synchronous_logging_alone` |
| the stripped ELF passes D3 | `board_tier2`'s `verify` path (`aethelgard build` strips; `simulate` boots it) |
| the reference's witness lines, `ICSR`, `PC` | `twin_tier2::the_reference_firmware_boots_and_two_boots_agree` |
| the droop probe's window, a device level surviving a reset | `campaign_tier2::the_two_reference_droops_behave_as_predicted_twice_byte_identical_and_replay`, with `droop-bor3`'s probe witnessed |

**D8's `flood-locked` test does not discriminate the composition** (named, as the advisor asked before the build): the spike measured today's CLI catching that late locked line 10/10 with asynchronous logging, so removing the configuration would not fail it. What does fail when the configuration is removed or a console level is set is deterministic and tier 1: `boot_tier1`'s and `campaign_tier1`'s stubs refuse a call without `--config`, without the synchronous key, or with a `logLevel` other than a device's (mutant M9 killed), and the renode.rs unit test pins the file. The spike's F2 measurement (the marker scattered 8/10 under `flood-max`) was not made a test: the ruling left `flood-max` out for CI time.

**Tier 1 (host, `systemd-run` scope, 32 GiB, no swap):** `cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean, and with `--features aethelgard-cli/with-tools`; `cargo test --workspace`: 1,084 passed, 0 failed.

**Tier 2 (the pinned image, before the merge of `main`'s `32a2be0`, docs only):** `twin_tier2` 21/21 (one assertion of mine corrected after its first run: at 1 µs the registers are also at reset, `register_mismatch`; the property, no locked access attributed, held), `campaign_tier2` 8/8, `board_tier2` 18/18, `with_tools` 23/23, `attribute_seeded` 12/12 (the seeded set's 22 cases, its rules and both traps unchanged). Each new tier-2 test alone, in the image (`ci/test-weights.tsv`): 5.98, 11.78, 2.95, 2.83, 4.65, 2.14, 3.62 s; the two renamed 2.97 and 2.98 s.

**Static verdicts against `main` (binding item 3; `~/.cache/aet-2204/baseline/`, main's binaries and the build's, the same 13 scenarios, every replay, the reference `simulate` at 100 ms):**
- every campaign line, every replay line: identical but for `state_sequence_hash` (each trace carries kind 13's new shape and version 5);
- the reference `simulate`: identical but for `boot.i2c_witness[0]` (`cr2`, `txdr` gone; `device`, `expected`, `observed`, `absent` added: D6, D7) and the new `boot.icsr` (D3);
- every verdict, finding and match: identical.

**Caps.** Every container ran under `~/.cache/aet-2204/img.sh` (`--memory`/`--memory-swap` equal, `--pids-limit -1`, `0` written to the container's own `memory.swap.max`, the three files read back before the command may start). **A record lost, mine:** the build's read-back lines for containers before 03:59 were in the spike's `caps.log`, and my own `git checkout` of that file before several commits discarded them. The 46 start-gate flags each container needed (written only after the read-back) remain in `~/.cache/aet-2204/capflags`; the values are gone. Every container since logs to `~/.cache/aet-2204/caps.log` (outside the repository): `memory.swap.max=0` and `pids.max=max` on each.

## Deviations and fence extensions, each named

- **`crates/aethelgard-chaos` kind 19** (the operator's ruling above) and kind 13's `absent` field (the seeded set's attribution, above).
- **`crates/aethelgard-evolve`**: `witness_fields`, `witness_wrong`, the `i2c_transaction_unwitnessed` selector, and the now-unused `CR2`/`SADD`/`NBYTES` state removed.
- **`crates/aethelgard-cli/src/elf.rs`** (`self_loops`, the ELF's own bytes), **`analog.rs`** (the probe's recovered rule and report), **`campaign.rs`** (kinds 13/19 emitted, `archived_lines`), outside the fence's "`renode.rs`, `verdict.rs` and `main.rs`'s `simulate`": each is where the record or the boot is built.
- **`crates/aethelgard-twin/src/model.rs`**: `Model` became an enum for D2's exhaustive `match`.
- **`crates/aethelgard-cli/README.md`**: the composition, the witness, D3's residual and the attribution rules.
- **`crates/aethelgard-svd-hal/tests/order_2203.rs`**: its "boot image" digest re-taken (the `Image` it prints carries the witness now); the firmware and both platform digests unchanged, so the generated files and the emitter are byte-identical.
- **`crates/aethelgard-cli/tests/workspace_shape.rs`**: allows the twin's vendored Renode files and its `sha2` dev-dependency.
- **The bridge** is unchanged: the droop's Renode gets the CLI's own arguments through `bridge --renode`, `--config` included, so the fence extension D4 allowed was not needed.

## CI

- **First run of the build** (`c017373`, run `43558095160`): every job green. Shards (8): 14m07s, 10m38s, 12m41s, 7m59s, 11m39s, 13m15s, 11m36s, **15m13s**. Shard 7, over the 15-minute bar, holds none of this build's tier-2 tests (they went to shards 0, 2, 3, 4, 6); its heaviest are `board_tier2`'s Kani and `verify` tests. For comparison, `main`'s latest run (`32a2be0`) peaked at 13m41s and `AET-WO-1908`'s branch at 15m33s: run-to-run variance is about a minute and a half.


## Adversarial pass (full, a fresh Opus subagent, on a frozen copy at `4641663`)

Harness and every verdict in `~/.cache/aet-2204-adv/` on Furnace (44 firmware edits, the two-device fixture, droop campaigns with replay). 30 containers, each reading back `memory.max=17179869184`, `memory.swap.max=0`, `pids.max=max`. Under rootless podman every verdict shows `uid_boundary:false` (`user_namespace`), as in the builder's runs: the sealed configuration under uid 65534 was exercised by the builder's own failure and fix (above), not by the attacker.

**No false pass against the witness.** Every forgery of §5.1 and the ruling's (b) and (c) failed: a wrong register (`Write D1`/`Read 0`), a value under a forged record (`i2c_identity_mismatch`), U5 read twice and claimed as U2 (U2 unwitnessed, `[]`), both reads from U2, the two devices swapped (`i2c_witness_out_of_order`), the UART, semihosting and echo variants.

**(a) The witness through another channel.** Each detector did the real transfer and then pushed a second `AETHELGARD-BOOT-BEGIN` line and a timestamped witness line through the channel:
- USART3 and USART1, no analyzer (the CLI attaches none): nothing reached stdout; the witness unchanged.
- With an analyzer (diagnostic only): every fragment, `\r` and `\n` included, is prefixed `[INFO] usart3: [host: …]`, never a whole line.
- Semihosting `bkpt 0xAB`: Renode logs only `Semihosting handler is not registered; R0=…`.
- Values Renode echoes in its own lines (`Unhandled write … value 0x…`, `Unknown slave at address 96.`): hexadecimal or decimal only. No run printed an ELF symbol or section name.

**(d) Abnormal endings under the new composition.** `abort-before`/`abort-after`: `cpu_aborted`, killed at the memory bound (about 15 s); `mem-touch`, `flood-after` at 100 ms: `memory_cap_exceeded`. On `droop-bor3`, each with replay agreeing: the reference passes (replay hash `2f17d035…`); `abort-after` and an abort only after the release give `brownout_unrecovered` + `cpu_aborted`; a flood gives `brownout_unrecovered` + `memory_cap_exceeded`; RAM- and flash-forged records give `brownout_unrecovered` + `i2c_transaction_unwitnessed`; a late claim gives `brownout_unrecovered`. The wall bound was not reached (floods and aborts hit the memory cap first).

**§5.2, a correct firmware:** the reference ×3 from 12 µs to 100 ms, the stripped `aethelgard build` ELF at 10 and 100 ms, folded warnings around the transfer, a flood under the budget: all pass. At the boot word's edge, in 10 ns steps: 11,870 ns fails `boot_word_missing` (the core on the `str`); from 11,880 ns it passes. **§5.4:** a locked access folded at the run's end is found; identical consecutive witness lines are printed apart, never folded; 60k flood pairs (past the budget) give exactly `i2c_transaction_unwitnessed` + info `renode_log_flood`. **Reserved names:** 16 references in mixed case (`NVIC`, `Usart1`, `SysBus`, `Machine`, `True`, `Using`, `Spi4`, `I2C1`, `Sram1`, `System_flash_ro`, `Empty`, `Preinit`, `Local`, `Reset`, `Dtcm`, `Watchdog`), I2C and SPI: all refused by `validate`, exit 2, no output; `gpioPortA`, `flashBank1`, `Echo` generate, load and pass.

| | Finding | Severity | Disposition |
|---|---|---|---|
| A1 | D3's self-loop and executable ranges came from the ELF's virtual addresses and file bytes, while Renode loads at the physical one and runs what is in memory. A decoy executable `PT_LOAD` (the loop's bytes, virtual at the core's spin, physical in free flash) passed a non-parked firmware; one `image_sha256` got opposite verdicts (the hash leaves out the virtual address); reachable through `aethelgard build` with a crafted `link.x`. Without a crafted ELF: the firmware rewrote its own loop in Renode's flash and spun there (`smc-flash2`), passing | medium, measured | **Fixed** (`77d9071`): the ELF check refuses an executable segment whose virtual address is not its physical one (Cortex-M has no MMU), takes `executable` and the loops at the physical address, and refuses more than 64 loops; the judge reads each loop's two halfwords back from the machine after the run (`sysbus ReadWord`), and a loop the machine no longer holds is `pc_not_on_self_loop`. Re-run on the attacker's ELFs: the three decoys (`store-loop-decoy`, `pwr-spin-decoy`, `decoy-built`) are refused `MalformedFirmware`; `smc-flash2` and `store-loop-twin` fail `pc_not_on_self_loop`. Pinned: `elf::tests::self_loops_are_found_where_renode_loads_them`, `boot_tier1::a_core_not_on_a_self_loop_fails_and_both_of_its_halfwords_pass` (the rewritten case), `twin_tier2::a_firmware_that_rewrites_its_loop_in_flash_is_not_parked` (tier 2, `TIER2_MIN` 91). Mutants N1 (the virtual-address refusal), N2 (the cap), N3 (the read-back): each killed. |
| A2 | "Parked" is not enforced: after the boot word a SysTick ISR toggled PA0 every 1.56 ms while the main code sat on the real fixed point; it passes | low, measured | **Named residual**, added to D3's: a pass also does not claim no interrupt runs after the boot word. **Decision needed (low):** require `SysTick CSR` = 0 and every `NVIC ISER` = 0 at the end (the generated firmware enables none), or keep it named until P4's firmware enables interrupts. |
| A3 | The static judge has no window between the record and the witness: a firmware that writes the OK record and the boot word first and transfers afterwards, or that writes a constant 0x60 instead of the byte it read, passes. The droop probe has the window (`droop-late-claim` fails) | low, measured | **Named residual:** a static pass means "the right device returned the table's value during the run"; it does not mean the record was derived from that read. |
| A4 | "Exactly the model's lines" fails a firmware that legitimately does more on the bus: a configuration write after the read, a second read, an extra write, or the two devices' phases interleaved (`W U2, W U5, R U2, R U5` gives `i2c_witness_out_of_order`). Extra traffic to an absent address passes (attribution's evidence only) | low, design | **Decision needed before P4** (the IMU bus will do more than one read): the witness's semantics for more than the identity read. Today's generated firmware makes exactly one read per device, and passes. |
| A5 (out of fence) | References `Peripherals` and `Sysbus_` give `Unroutable` ("cannot reach Peripherals pad 8"): the router depends on reference names | low, measured | Reported, not changed (`aethelgard-eda`, outside the fence). |

## Merges of `main`

- `32a2be0` (#112, docs: the RUL-17 amendment ratified, `AET-WO-1908` amended): merge commit `a0c8e03`, no conflict.
- `23a7c79` (#109, `AET-WO-1908`: CI weights refitted to CI seconds; `CI_SCALE` gone): merge commit `d101084`. `ci/test-weights.tsv` conflicted; main's file is kept whole, the two renamed tests keep main's CI times under their new names, and the eight new tier-2 tests are added at CI scale (alone in the image × 2.2, `twin_tier2`'s own ratio between main's CI times and my image times: 12.390/5.52, 6.400/2.96). `TIER2_MIN` 91 = main's 83 + 8. `ci/test-shard.sh` passes.

## Findings for the architect (build)

- **B1 · Renode rewrites a configuration file that lacks a key, and the CLI's files are sealed for the tool uid · `renode.rs` `config` · high, measured, fixed.** Above ("Why every key"). The spike missed it because its runs were outside the sandbox; it said so.
- **B2 · The seeded set's wrong-address cases needed the controller's `Unknown slave` lines · evolve, kind 13 `absent` · medium, measured, built.** Above. Decision-needed (low): keep `absent` as attribution evidence (recommended; `attribute_seeded` 12/12) or rule the seeded expectations changed.
- **B3 · D8's flood test does not discriminate the composition · medium, by design of the ruling.** Above; the deterministic tier-1 guard is the stub's check of the arguments and the file.
- **B4 · The droop's `ObservedReport.witness` is a whole `WitnessReport`, of which kind 19 archives only `observed`; `absent` is always empty there** · low, reduction found it. Slimming it changes the verdict's JSON: decision-needed (low).
- **Adversarial A2, A3, A4 and A5** above: two named residuals and two decisions (A2's interrupt check; A4's witness semantics before P4).

## Gates (build)

- `cargo fmt --all -- --check`: clean. `cargo clippy --workspace --all-targets -- -D warnings`: clean, and with `--features aethelgard-cli/with-tools`. `cargo test --workspace`: 1,085 passed, 0 failed (host, after the reduction).
- Tier 2 in the image: the suites above, then after A1's fix the new test and its neighbours (`a_firmware_that_rewrites_its_loop_in_flash_is_not_parked` 3.76 s alone).
- **Reduction pass (diff): +3 net (code −16, doc +19) · 2 duplicates removed (the host-timestamp parser in `log_text`, now `log_body`'s; the `WitnessReport` built in two places, now `WitnessReport::of`) · 0 dead paths left (the `CR2`/`TXDR` state was removed with D7, above) · 1 protected kept (`analog.rs`'s `belt-and-suspenders` on `check_droop`) · 4 findings: stale prose fixed in the chaos README, `record.rs` and evolve's `Field` doc; the witness line format lives in the twin (written), evolve (parsed: evolve cannot depend on the twin) and the CLI (matched), forced by crate layering; `split_devices` and `device_path` both lower-case the reference (both needed: the boot path does not go through `emit`); B4.** Run by a fresh subagent, read-only at `01b3858`; the lead applied and proved it (the suite green; the guards' mutants above unchanged).

**Not shown here:** live acceptance (merge, deploy, the uid-boundary path under real root in CI rather than rootless podman) is the operator's. A green CI is not acceptance.
