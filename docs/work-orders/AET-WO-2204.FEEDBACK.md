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
