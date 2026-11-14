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
- 18 containers of the pinned image ran, each under `spike/2204/img.sh`: 16 at `--memory 16g` and 2 at `12g`.
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
- `s1.py` (S1, S2), `s3.py`, `s4.py`, `s4_marker.py` and `s5.py` run the questions.
- `two-bme.json` is the reference specification with a second BME280, `U5` at `0x77`, on `SENSORS`.

**The outputs** are in `spike/2204/out/`, one JSON line per run. Raw stdout stayed in the cache.

**One correction during the runs.** My first parser trimmed `\r` only at line ends; the CLI trims both ends (`renode.rs` `clean_lines`). Its file-mode rows were misread, so it was fixed and S4 was re-run. `out/s4.jsonl` is the re-run.

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
- **Renode folds a repeated identical line** into one with a ` (N)` count. A counting witness must sum the counts.

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

### S4 — measured (`out/s4.jsonl`, `s4b`, `s4c`, `s4d`, `s4-marker.txt`; and `s3.jsonl` for the judge's own reads)

**The three compositions, ten runs each.**
- **cli:** today's tail, `RunFor; logLevel 3`.
- **onelevel:** no level change.
- **file:** `logFile` first and `logLevel 3 console`, both before anything runs.

"Lines" counts each warning kind with the folds summed.

| Firmware (what it logs) | cli | onelevel | file |
|---|---|---|---|
| `ref` | reads hash `b5b77d13…`, 10/10 | same hash, 10/10 | same hash, 10/10 |
| `burst` (500 identical warnings, folded into one line, then 1 locked) | 500 + 1, 10/10 | 500 + 1, 10/10 | 500 + 1, 10/10 |
| `flood` (paced, 167 + 167) | complete 10/10 | complete 10/10 | complete 10/10 |
| `flood-fast` (15,607 + 15,607; 2.9 MB) | complete 10/10 | complete 10/10 | complete 10/10 |
| `flood-max` (332,937 + 332,936; 62 MB) | **3,446 to 3,627 lost per kind**, varying; reads parse 10/10 | all lines 10/10; **`BEGIN` unparsable 7/10** | all lines 10/10; reads 10/10; file 62.9 MB |
| `flood-locked` (199,761 + 199,761, one locked read about 600 lines before the end; 37.7 MB) | **locked line lost 7/10**; 0 to 3,221 lost | locked line 10/10 | locked line 10/10 |
| ref at 1 µs (the **judge's own** reads hit gated `I2C1`/GPIO) | 0 lines shown; reads clean 10/10 | 2 locked lines **in the read block**; **a value line split 7/10** | 2 lines in the file; reads clean 10/10 |

- **The marker vanishes at character level.** `s4-marker.txt` is ten more `flood-max` runs at one level. In 8 of 10, `AETHELGARD-BOOT-BEGIN` was scattered one or a few characters at a time across consecutive log lines: `A[…]`, `E[…]`, `TH[…]`, `EL[…]`, `GA[…]`, `RD-B[…]`, `OO[…]`, `T-BEGIN[…]`. The monitor's output and Renode's log thread interleave below line level. A value line splits the same way: `0x000000[01:21:37.0167] [WARNING] sysbus: Tried to read a locked peripheral…`.
- **Today's `simulate` on `flood-locked`, ten runs:** `fail` with `locked_peripheral_access` and `renode_log_flood`, **10/10**. The loss that the raw reader shows did not happen through the CLI's own reader. B10's loss is real in Renode, but whether it happens depends on how fast the reader drains the pipe.
- **An in-stream marker separates the run's lines from the judge's.** `log "AETHELGARD-RUN-END"`, sent right after `RunFor` in the file composition, goes through the same queue as the firmware's lines.
  - ref at 1 µs: the judge's own 2 locked lines fell **after** it, 10/10.
  - `flood-locked`: the firmware's locked line fell **before** it, 10/10.

  (`LogCommand.cs` L33–35: `log "<message>"` logs at `Info`.)

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
| P2.3 | 2 device lines per read; the access log 8 to 14 lines | 2 device lines; access log 14 CPU lines for one read (19 with the judge's reads) | device yes; access slightly over |
| P2.4 | about 150 B (device lines) and 1.5 KiB (access log) per read; under 50 ms | about 84 B and about 1 KiB; wall time within noise | yes, lower |
| P2.5 | the judge's own reads in the access log, no `[cpu:]` prefix | so | yes |
| P2.6 | device lines depend on the model; the access log does not | by construction (the source); not run on a second model | read, not run |
| P3.1 | fixed point = `yield; b.n` at the end of `main`; the `take` `else` loop separate (60 %) | `[0x08000594, 0x08000598)`; `else` loop at `0x0800055A` | yes |
| P3.2 | ref `PC` alternates in the loop; `systick-before` `0x08000096` | so | yes |
| P3.3 | `VECTACTIVE` 0 / 15; `CPSR` `IPSR` = `VECTACTIVE` | 0 / 15; `0x21000000` / `0x2100000F` | yes |
| P3.4 | known from the ELF alone, no generator change; §7's second condition does not hold | `main`'s last 4 bytes, on two ELFs | yes |
| P4.1 | before `logLevel 3`, the console prints warnings | so | yes |
| P4.2 | today's tail loses lines on a 501-line burst, sometimes the locked one | **no**: no loss on the burst, the paced flood or a 2.9 MB flood. Loss only past about 37 MB (`flood-locked`, `flood-max`), and not through today's `simulate` in 10/10 | **no** |
| P4.3 | one level: all lines, byte-exact reads, listing parses | all lines, yes; **but** the `BEGIN` marker scattered 7/10 and 8/10 under `flood-max`, and the judge's own locked reads split a value 7/10 | **no** |
| P4.4 | between markers only bare values or whole log lines | **no**: fragments (above) | **no** |
| P5.1 | about 100 names, static and pinned | 97 instance names, 2 builtins, 13 keywords; pinned by hash | yes |
| P5.2 | per-reference outcomes as tabled | every row as predicted, with the measured messages | yes |
| P5.3 | a prefix changes the committed `.repl` | not run; by the fence | read |

## Recommendations, one per question

- **S1. The witness is the device model's own log of the transaction, not a register.**
  - **What it reads:** each modelled identity read must show, in the log of the run (S4's file, before the marker), `[NOISY] i2c1.<ref>: Write <register>` and then `[NOISY] i2c1.<ref>: Read <expected bytes>`, for that device's emitted name.
  - **How it is set up:** the judge sets `logLevel -1 sysbus.<bus>.<ref>` for each modelled device before the run.
  - **Why it holds:** only the controller's `StartTransfer` to that device's address calls the model's `Read`, so no store can produce the line. Measured: the reference has it, and every forgery lacks it.
  - **The pinning test:** the A11.1 test is inverted on this, with a finding named for the device (for example `i2c_transaction_unwitnessed`, reworded to name the absent model line).
  - **The `CR2`/`TXDR` witness** adds nothing against a forgery (P1.2). It still catches a wrong address, but so does the device line. Recommend dropping it at the build (D7).
- **S2. The same witness, one per identity read.**
  - Each check is judged by its own device's line pair, in record order, so every transaction is witnessed, not the last on a bus.
  - `two-bme.json`'s shape is the test fixture for the second device. It is a test fixture, not a change to the reference specification.
  - Where a future model (`P4`'s IMU) logs nothing, the controller's access log is the model-independent fallback: a `[cpu: …]` `CR2` write with the device's `SADD` and `START`, then `ReceiveData … returned <byte>`. It costs about 12 times the bytes (D2).
- **S3. The judge requires the core at the fixed point by two readings.**
  1. **`ICSR.VECTACTIVE` is 0.** This is Renode's NVIC state, not the firmware's claim. A new read, `sysbus ReadDoubleWord 0xE000ED04`, under mask `0x1FF`. `systick-before` fails here.
  2. **`PC` is inside the last 4 bytes of the ELF's global `main`.** The ELF checks (R2) refuse a `main` whose last 4 bytes are not `yield; b.n` back to the `yield`.
  - This needs no generator change, so §7's second condition does not hold.
  - The location comes from the firmware's symbol table, as the executable ranges do today (D3).
- **S4. Not one console level: a log file, plus an in-stream marker.**
  - **The composition:**
    - `logFile @<scratch>/renode.log` and `logLevel 3 console` first, before `mach create`;
    - then the boot;
    - `log "AETHELGARD-RUN-END"` right after `RunFor`;
    - then the reads between the console markers, as today.
  - **What the judge reads where:** every log-derived finding (locked access, error lines, the S1 witness) comes from the file, up to the marker. The console keeps only the monitor's own output.
  - **Measured:**
    - every line reached the file in every fixture, 10/10;
    - the reads were byte-identical to today's on the reference, 30/30;
    - the judge's own lines fell after the marker, 10/10;
    - one console level failed twice: character-level interleaving under a flood, and the judge's own reads injecting into the read block.
  - **Open:** the file's size under a flood against the sandbox's 64 MiB `RLIMIT_FSIZE`. `flood-max` wrote 62.9 MB in 10 ms, so a 100 ms flood would cross it (D4).
  - **Unchanged:** `main`'s static verdicts change only where a firmware floods (none of the reference's tests do).
- **S5. Refuse the reference before any output.**
  - **The rule:** a reference whose emitted (lowercased) name equals an all-lowercase stock instance name, a builtin (`machine`, `sysbus`) or a keyword is refused.
  - **Where:** the set is static and pinned by hash, so by the work order's rule the refusal belongs in `validate`, as a constant list in `aethelgard-ir` with a tier-1 twin test that parses the pinned chain's entries (a parser, not a line regex: `else:`) and asserts the list equal (D5).
  - **The existing check:** `split_devices`' check stays; it already refuses before output (`i2c1`).

## §7 checked

- **§7.1 holds, measured: no `I2C1` register in Renode 1.17.0 is both unforgeable and present after a real transfer.**
  - By the work order this is a stop, and the spike stops here anyway.
  - The remedy the spike measured is a Renode-recorded event: the device model's log line, read from a file (S1, S4).
  - Whether a log-derived witness is acceptable in place of a register is **D1**.
- **§7.2 does not hold.** The fixed point is known from the ELF alone (S3).
- **§7.3 is not reached by the spike.** Its reads were byte-identical across compositions. The verdict-level identity is the build's test.

## Findings

- **F1 · §7.1 holds: the reference's transfer leaves no register a store cannot set · `STM32F7_I2C.cs`, `board.rs` L402 · high · measured (S1).** The transfer consumes `RXNE`/`RXDR` and clears `STOPF`, so the final state is the reset state plus `CR2`/`TXDR`, all writable. Recommendation: S1. Decision needed: D1.
- **F2 · Renode's console interleaves the monitor's output and its log thread below line level · Renode 1.17.0 · high for any composition that prints log lines while reading · measured (S4).** `BEGIN` was scattered 8/10, and values split 7/10. A whole-line skip cannot recover it; it can only refuse (exit 5). This affects the work order's proposed §2.4 (one console level). Recommendation: S4.
- **F3 · The judge's own monitor reads log too · `renode.rs` reads · medium · measured.**
  - Reads of a gated peripheral log `Tried to read a locked peripheral` (ref at 1 µs, 2 lines), and with the access log on every read logs.
  - Today's `logLevel 3` hides them. Any lower console level would attribute them to the firmware as `locked_peripheral_access`.
  - Recommendation: the in-stream marker (S4).
- **F4 · B10's loss depends on how fast the reader drains the pipe · today's static tail · medium · measured.** A raw pipe lost the late locked line 7/10, and today's `simulate` caught it 10/10. The loss needs a backlog of about 37 MB or more; a 2.9 MB flood lost nothing. B10 stands, but its occurrence through the CLI was not reproduced in 10 runs.
- **F5 · Renode folds identical consecutive log lines into one, ` (N)` · Renode 1.17.0 · low · measured** (`returned 0x1 (10000)`; 500 underflows in one line). A count-based witness must sum the folds.
- **F6 · The droop probes (2301 A2) keep one console level and skip whole log lines · `renode.rs` `probes_in` · medium · REASONED from F2, not run on the droop path.** Under a flood, a probe's marker or value can scatter. That gives exit 5, never a false value. It is the same exposure as F2, on the bridge's path. Recommendation: the S4 composition there too, at the build or later.
- **F7 · `split_devices`' taken-set compares the lowercased name with mixed-case stock names · `platform.rs` L128 · low · measured.** `gpioPortA` emits `gpioporta`, which Renode treats as distinct, and the platform loads. Harmless today; S5's list subsumes it.
- **F8 · Three identical `yield; b.n` loops in the generated firmware · `main.rs` (`fault`, `Board::take`'s `else`, the fixed point) · low · measured.** "`PC` inside `main`" alone would accept the `else` loop; S3 uses `main`'s last 4 bytes.

## Decisions needed

- **D1 (§7.1): accept a log-derived witness in place of a register.** It is the model's `Noisy` line, read from a file. Recommend yes: it is what Renode records, not what the firmware writes (`INV-07`).
- **D2 (S2): which channel carries the witness.**
  - **(a)** the device-model lines only. Recommended: about 84 B per read, names the device.
  - **(b)** the controller access log only: model-independent, about 1 KiB per read.
  - **(c)** both, (b) only for a model that does not log.
- **D3 (S3): where the fixed point comes from.**
  - **(a)** `main`'s last 4 bytes from the ELF, with `VECTACTIVE` = 0. Recommended: no generator change, the same footing as the executable ranges.
  - **(b)** a generator-emitted linker symbol: a generator change, so a §7 stop of its own.
- **D4 (S4): the file composition, and how a flood is bounded once the log is a file.** At about 64 MiB, the sandbox's `RLIMIT_FSIZE` turns today's flood verdict (`renode_log_flood`) into a `ToolError`. Options:
  - **(a)** the judge treats a file at the cap as a flood (count, keep the first of each kind) and still reaches a verdict;
  - **(b)** a higher cap for this one file;
  - **(c)** keep the console path for floods.

  Recommend (a). Also: whether 2301's droop probes move to the same composition (F6) in this work order or later.
- **D5 (S5): where the refusal lives.**
  - **(a)** `validate`, with the list in `aethelgard-ir` and a twin test holding it equal to the pinned files. Recommended, by the work order's rule.
  - **(b)** the twin's `split_devices`, which keeps Renode facts out of the IR and is also before any output.
- **D6: the trace version.** The witness's record (kind 13) changes from `CR2`/`TXDR` words to per-read log evidence, so the trace version rises once. `AET-WO-2205` owns the record renames, and **one bump per work order** means the two must not both bump for the same change. Which work order bumps first?
- **D7: drop the `CR2`/`TXDR` witness at the build,** since the device line subsumes it (S1). Recommended: yes, in the reduction pass.

## Gates (spike)

- The diff is this file and `spike/2204/`: Python, shell, JSON and outputs. No Rust changed.
- `cargo fmt`, `clippy` and the tests are unaffected; CI runs them on the PR.
- `docs/work-orders/AET-WO-2204.md` is the architect's, on `claude/argon-repo-docs-synthesis-rjrb0d`. It is not committed here.
