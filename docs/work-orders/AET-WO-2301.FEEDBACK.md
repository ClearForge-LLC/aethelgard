# AET-WO-2301 — FEEDBACK (spike)

**Branch:** `wo/aet-2301-dynamic-faults` · **Base:** `e3e3cd2` (`origin/main`) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Cadence:** spike (§1), then **STOP** for the architect's ruling. No build code before it.

**Where everything runs.** The pinned tools image `ghcr.io/clearforge-llc/aethelgard-tools@sha256:c69ff9bc…` (`container/IMAGE`) under rootless podman, with `--memory X --memory-swap X --pids-limit -1`, `0` written into the container's `memory.swap.max` after start, and `memory.max`, `memory.swap.max` and `pids.max` read back into `spike/2301/out/caps.log`. `TMPDIR` is unset inside the container, as in CI.

Entry schema for findings: `finding · where · severity · type · recommendation · decision-needed`.

## Predictions — written before any run

This section was committed before the first probe ran; its commit is the first on this branch. Nothing below it was edited after a run. Each prediction has an id; the table at the end puts each result beside it.

### What the predictions rest on (read at source, nothing run)

- **Renode 1.17.0's STM32H7 has no power model.** `platforms/cpus/stm32h7.repl` (sha256 `b85cb4af…`, the file `boards/sentinel-01/sentinel-01.repl` names) L728–746 declares `pwr` as a `Python.PythonPeripheral` whose reads are constants: `CR1` `0xF000C100`, `CSR1` `0x6000` (`ACTVOSRDY` only, so `PVDO` is never set), `CR3` `0x4000000`, `D3CR` `0x2000`; every other offset logs "Unhandled access". No PVD, no BOR, no PDR.
- **Its reset-status register is a constant.** `STM32H7_RCC.cs` (renode-infrastructure `066a7f13`) L672–702: `RSR` is defined with reset value `0x00FE0000` (POR, PIN, BOR, D1, D2, C1, C2 flags all set), every flag read-only, cleared only by `RMVF`. Any machine reset restores it. Renode cannot say which reset happened.
- **EXTI line 16 (PVD/AVD) is wired to NVIC IRQ 1** (`stm32h7.repl` L17, `[16-19] -> nvic@[1, 41, 2, 3]`). Nothing in Renode drives it.
- **A machine reset keeps memory.** `MappedMemory.Reset()` (`MappedMemory.cs` L184–191) does nothing: RAM and flash survive. `Machine.Reset()` (`Machine.cs` L804–826) resets every registered peripheral, then raises `MachineReset`, which the monitor answers with the `reset` macro if one is defined, else a warning (`Monitor.cs` L1077–1099).
- **A Cortex-M reset forgets `VTOR`.** `CortexM.InnerReset` (L930–935) clears `vtorInitialized`; on leaving reset, `InitPCAndSP` (L1063–1110) re-guesses `VTOR` from the first non-empty section unless `InitVectorTableOffset` is set, then loads SP and PC from it. The CLI sets `cpu VectorTableOffset` explicitly before the first boot because the guess can be wrong (`AET-WO-2100` adversarial A3).
- **A CPU can be held.** `CortexM.CpuWaitSignal` (signal `0x1000`, L34–37, L937–967) and `IsHalted`: a held core executes nothing while virtual time advances; release resumes it, from the reset vector when it is still in reset.
- **The monitor's history path is configurable.** `ShellProvider.cs` L33: `history-path` under `[general]` in Renode's config, default `$HOME/.config/renode/history`.
- **`logFile <path>` adds a file backend** (`LoggerFileCommand.cs`); it does not remove the console backend.
- **The generated firmware enables no interrupt.** `boards/sentinel-01/firmware/src/main.rs` has a vector table of exactly 15 entries (`Reset` and 14 core exceptions, every one `fault`, which parks), no NVIC write, and no interrupt-enable bit: the board crate's only `I2C1` `CR1` write is `PE` (`board.rs` L327–329), and `SPI1`'s only `CFG2` bit is 22 (`SSOE`, L425). The generator (`crates/aethelgard-svd-hal`) names no NVIC, `SysTick` or `cpsie`.
- **The rail the specification declares.** `examples/sentinel-01-minimal.json` declares `VDD_3V3` at 3,300 mV ± 50 ‰ (3,135 to 3,465 mV) and no source impedance, load or regulator. The schematic emitter places one 100 nF decoupling capacitor per IC on its supply (`schematic.rs` L512, L610, L649): the MCU, U2, U3 and U4, so 400 nF on `VDD_3V3`. The 2.2 µF capacitors are on the core's `VCAP` pins, not on `VDD`.
- **The fold.** `replay.rs`: a `Record` event (kinds 5 to 15) is pushed into `State.records` and counts against `MAX_RECORDS` = 65,536; an `AnalogSample` event (kind 3) overwrites one map entry per net and is not counted.
- **The sandbox's file cap.** `run.rs` L93: `RLIMIT_FSIZE` = 64 MiB for every tool, and a file left at the cap makes the call a `ToolError` (L533–542), exit 5.
- DS12117's thresholds are being read at source as these predictions are written; they appear below by symbol (`V_PDR`, `V_BOR`), and P1's table gives the cited values.

### P1. The generated firmware under a reset, by which means

The probe boots the generated firmware (`boards/sentinel-01/firmware`, built in the image) on the emitted `sentinel-01.repl` with the CLI's own command prefix (seed, cold writes, cleared boot word and record, `VectorTableOffset`). It runs 10 ms, re-arms the boot word (`0`) and the record (`RECORD_UNSET`) exactly as the CLI does before the first boot, so that only the firmware's code can set them again, applies the means, and runs 10 ms more, reading between quanta.

| id | Means | Predicted |
|---|---|---|
| P1.1 | `machine Reset`, nothing re-applied | Renode re-guesses `VTOR` as `0x08000000`, the ELF's first section; the firmware boots again, re-writes the boot word, and its I2C read gives OK `0x60`. Medium confidence on the guess. |
| P1.2 | `machine Reset`, then `cpu VectorTableOffset` and the cold writes re-applied | The same as P1.1, and every read equal to the first boot's. **The means to recommend.** |
| P1.3 | `cpu Reset` only | Boots again, read OK. Peripherals keep their state (I2C1 clocked and enabled); the firmware re-initialises them and the result is the same. |
| P1.4 | Held reset: `machine Reset`, `cpu IsHalted true`, run 1 ms, `cpu IsHalted false` | While held, `ExecutedInstructions` does not move and virtual time does. After release it boots, read OK. |
| P1.5 | The CPUWAIT signal (`cpu OnGPIO 4096 true`, then `false`) around a `machine Reset` | The same as P1.4. Lower confidence: release depends on the core still being "in reset". |
| P1.6 | A PVD signal: `exti OnGPIO 16 true` | **Nothing observable.** EXTI's mask for line 16 is clear at reset and IRQ 1 is not enabled, and the firmware has no vector for it. No reboot; the read is not repeated. |
| P1.7 | Recovery, every means that reboots | Boot word re-written and read OK within 100 µs of virtual time after the release. |
| P1.8 | Determinism | Every row identical over 10 runs: one hash over the reads, `ExecutedInstructions` and `PC`. |
| P1.9 | `ExecutedInstructions` against virtual time | Equal to MIPS × time across P1.1 to P1.3 (the core never stops). **Short of it by exactly the held time** across P1.4 and P1.5, so the judge's rule "instructions = MIPS × time" (`renode.rs`) cannot hold for a held reset as written. |

The re-arm (boot word to `0`, record to `RECORD_UNSET`) is the bridge's action, not the firmware's: Renode keeps RAM across a reset, as a warm reset on the part does, so without it the old boot word would read as a new boot. A build records it as the model's rendering, beside the reset itself.

### P2. The rail, and what a droop scenario declares

**Proposed deck (the honest minimum), every value declared:**
- a source at the domain's `nominal_millivolts`, whose sag is the fault: a PWL the scenario declares (start, fall, hold, rise, floor);
- through a source resistance the scenario declares, into the decoupling the emitted BOM puts on that domain (400 nF for `VDD_3V3`);
- with a constant load current the scenario declares.

The probe runs it with libngspice in-process, by `AET-WO-2101`'s `ioc` route (a breakpoint at every quantum boundary, the bridge's hook at each), at 10 µs and 1 µs quanta, with 0.5 Ω and 100 mA, for four cases: a sag to 3.20 V (inside tolerance), to 2.40 V (below `V_BOR` at its highest level, above `V_PDR`), to 1.20 V (below `V_PDR`), and a short dip that stays below a threshold for 3 µs.

**The 2.40 V and 1.20 V floors are provisional** until DS12117's table is cited. If the cited `V_BOR` levels or `V_PDR` put either floor on the wrong side, the floors are fixed in a separate commit, before the first P2 run, and that commit says so. Which BOR level the part runs at is an option-byte setting the specification does not declare; it is a decision below, not a prediction.

| id | Predicted |
|---|---|
| P2.1 | The rail follows the source's PWL less the 50 mV IR drop, lagging by about R·C = 200 ns. It has no feature shorter than the declared ramps, because the deck has no regulator. The tolerance case's minimum is 3.150 V, above 3.135 V: no crossing. |
| P2.2 | Read at quantum boundaries, each crossing is seen up to one quantum late: between 0 and 10 µs at 10 µs quanta, between 0 and 1 µs at 1 µs. |
| P2.3 | The 3 µs dip is missed at 10 µs quanta when it falls between two boundaries, and caught at 1 µs. Solving the rail first and taking ngspice's own crossing times catches it at any quantum. |
| P2.4 | ngspice's wall time for 100 ms of rail: under 1 s at 10 µs (10,000 breakpoints), under 5 s at 1 µs (100,000). |
| P2.5 | **2101's "about 10 µs" is not a property of this rail.** With no regulator, the time scale is the scenario's own ramps. And because a declared sag does not depend on what the firmware does, the rail can be solved before the boot and its crossings applied at their exact times: no per-quantum lock-step is needed for this fault. This is a recommendation to measure against, not an exemption: the lock-step runs below are still made. |
| P2.6 | **End to end, lock-step, 100 ms with a BOR-crossing droop and the generated firmware**, `ioc` route, one read per quantum, Renode's history at `/dev/null`: about **90 s at 10 µs** (10,000 quanta at about 9 ms each, `AET-WO-2101` S4) and about **700 s at 1 µs** (100,000 at about 7 ms). The second is past §7's "about 10 minutes of CI" for one scenario. The schedule of P2.5 (Renode run segment by segment to each crossing): under 5 s. |

### P3. What an interrupt storm means for the generated firmware today

| id | Predicted |
|---|---|
| P3.1 | It enables **no** interrupt (above). |
| P3.2 | `SysTick`, armed from the monitor after the fixed point (`CSR` = 7, a short reload): the first tick enters `fault` and parks there for good. The boot word stays written; `PC` moves into `fault`; instructions still equal MIPS × time. One interrupt is terminal, so a storm is indistinguishable from one interrupt. |
| P3.3 | `SysTick` armed before the boot: the firmware parks before it writes the boot word, and the judge sees no boot. |
| P3.4 | EXTI line 16 raised, or IRQ 1 set pending in the NVIC: no effect. Pending, never taken. |
| P3.5 | Recommendation: defer storms to the first board whose firmware takes an interrupt (P4's IMU data-ready). |

### P4. `OQ-03`, dynamic

| id | Predicted |
|---|---|
| P4.1 | One rail sample per quantum as the existing kind 3 (`AnalogSample`: net, µV): about 20 B on the wire, 100,000 per simulated second at 10 µs, so **about 2 MB/s**; 100 ms is about 200 KB. |
| P4.2 | One per quantum as a labelled record (a new kind that names its source): 35 to 45 B, **3.5 to 4.5 MB/s**, inside §2.17's estimate of 3 to 6 MB/s. |
| P4.3 | Fold memory: kind 3 costs one map entry per net, independent of count. A record costs about 350 B held (the 35× of `AET-WO-1900`): 100 ms at 10 µs is 10,000 records, about 3.5 MB; one simulated second is 100,000, past `MAX_RECORDS`. |
| P4.4 | Crossings only (P2.5's schedule): under 1 KB per droop, independent of virtual time. |
| P4.5 | Measured with the real `TraceWriter` and `replay_dir` (a path dependency on `aethelgard-chaos`), peak RSS of the replay: within 20 % of P4.3's figures. |

Whether crossings-only recording is "not sampling" depends on the ruling. If the build keeps per-quantum lock-step and records only the crossings, the dropped samples are a sampling rule, and that is Scotty's (`INV-04`). If the build runs the schedule of P2.5, there is no per-quantum state to drop. It is listed as a decision, not settled here.

### P5. The log flood (`AET-WO-2100` F23)

| id | Predicted |
|---|---|
| P5.1 | `logFile @<scratch>/renode.log` alone does **not** take the flood off stdout: the console backend keeps printing. Raising the console's level (`logLevel 3 console`) does. |
| P5.2 | With the console quiet, a flood firmware writing varying warnings at about 24,000 lines per virtual ms fills the file at about 3 MB per virtual ms. A 100 ms boot reaches `RLIMIT_FSIZE`'s 64 MiB, and the call is exit 5 again. **A plain file does not by itself make a flood reach a verdict.** |
| P5.3 | A bridge that owns Renode reads its output as a stream, keeps the marker block and a bounded count of log lines by kind, and returns a verdict whatever the flood's size. REASONED; the probe measures it. |

P5 is measured under the CLI's current sandbox (the trampoline sets `RLIMIT_FSIZE`; a bare container has no 64 MiB cap), through a runner rebuilt against today's `run.rs`, with a flood firmware under `spike/2301/` in `AET-WO-2100` A9's shape (varying locked-peripheral writes). The judge reads its `locked_peripheral` finding from the same stdout today (`renode.rs`, `LOCKED`), so a quiet console needs another stated source for that finding; the probe reports where it comes from.

**Orientation, before these predictions.** Three short containers ran capped at `--memory 4g --memory-swap 4g`, without swap zeroed or the caps read back: copying the platform files out of the image, `renode --version`, and a `find`. Nothing was measured in them. Every run from here goes through the spike's runner.
