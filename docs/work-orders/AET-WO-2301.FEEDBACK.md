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

---

## Results — after the predictions

Everything below was written after the runs. The predictions above are unchanged since `62c2271`. Each output under `spike/2301/out/` begins with its `# cmd:` line; raw run files stay on Furnace under `~/.cache/aet-2301/`.

### What the spike added (a build deletes all of it)

- `spike/2301/img.sh`: the runner (pinned image, `--memory 12g --memory-swap 12g --pids-limit -1`, swap zeroed and the caps read back into `out/caps.log` before the command may start), and `rerun.sh` (F13's re-runs).
- `spike/2301/renode/`: `drive.py` (P1 and P3: Renode's monitor on a piped stdin, the CLI's own boot prefix, reads between whole-line sentinels), `all.sh`, `summary.py`.
- `spike/2301/ngspice/`: `AET-WO-2101`'s `ng-probe` crate, copied from its spike branch less its two unused modules (the hand-declared libngspice FFI; its only `unsafe`).
- `spike/2301/lockstep/`: the P2 and P4 owner (`lockstep`: modes `rail`, `lockstep`, `schedule`, `replay`; no `unsafe`; depends on `ng-probe` and, by path, on the real `aethelgard-chaos` writer and reader), and its run scripts.
- `spike/2301/p5/`: the flood firmware (`flood/`, no dependencies), `sbx.sh` (the CLI's sandbox from a shell, rebuilt against today's `run.rs`), `stream.py`, `cli.sh`, `summarize.sh`. Measured by a Sonnet subagent; the lead re-read its outputs (below, "re-checked").
- Every probe crate declares its own `[workspace]`. Nothing outside `spike/2301/` and this file differs from the base.

### Read at source: DS12117, and what could not be read

- **ST's site could not be reached from Furnace** (st.com, and two distributors, hung). The datasheet was read from a third-party mirror of **DS12117 Rev 7, April 2019** (sha256 `61fe6bb8…`, 356 pages, kept outside the repository). The current revision (Rev 10, January 2026, per search results) was not opened. The lead checked the rows below against the extracted text.
- **Table 26, "Reset and power control block characteristics"** (§6.3.4, pp. 109–110; repeated as Table 125, §7.3.4):

  | Symbol | Condition | Min | Typ | Max |
  |---|---|---|---|---|
  | `VBOR0` | rising / falling | 1.62 / 1.58 | 1.67 / 1.62 | 1.71 / 1.68 V |
  | `VBOR1` | rising / falling | 2.04 / 1.95 | 2.10 / 2.00 | 2.15 / 2.06 V |
  | `VBOR2` | rising / falling | 2.34 / 2.25 | 2.41 / 2.31 | 2.47 / 2.37 V |
  | `VBOR3` | rising / falling | 2.63 / 2.54 | 2.70 / 2.61 | 2.78 / 2.68 V |
  | `Vhyst_BOR_PVD` | Run mode | | 100 mV | |
  | `tRSTTEMPO` | "Reset temporization after BOR0 released" | | 377 µs | |

  `VPVD0` to `VPVD6` run from 1.86 to 2.76 V falling (typ). Rev 7 has **no separate POR/PDR row**: the lowest reset is `VBOR0`, and footnote 2 says BOR0 "is enabled in all modes".
- **`VDD`** (Table 23, p. 107): standard operating voltage 1.62 to 3.6 V, footnote "When RESET is released functionality is guaranteed down to VBOR0 min". §3.5.2: "Three BOR thresholds (from 2.1 to 2.7 V) can be configured through option bytes."
- **NOT FOUND: RM0433.** The `BOR_LEV` option-byte encoding, the level the part is delivered at, and the PWR chapter's text were not obtained from any source, and nothing was filled from memory. **Which BOR level the part runs at is therefore unknown to this project** (decision D2).
- With these values the provisional floors stand: the `bor3` case's rail floor (2.35 V) is below `VBOR3` falling at its maximum (2.68 V) and above `VBOR0` rising at its maximum (1.71 V); the `pdr` case's floor (1.15 V) is below `VBOR0` falling at its minimum (1.58 V). No floor was changed.

### P1 — measured (`out/p1-p3-summary.txt`; commands of one run in `out/p1-machine-reset-vtor.cmds`)

The probe boots the generated firmware with the CLI's exact boot prefix (captured from `aethelgard campaign` on `pullup-2k2`: `out/p1-cli-cmdline.txt`), runs 10 ms, re-arms the boot word and the record, applies the means, then reads every 10 µs for 1 ms and once at 20 ms. **Every case gave one hash over 10 runs.**

| Means | What the firmware observes | Boots again | I2C read resumes | `ExecutedInstructions` at 20 ms |
|---|---|---|---|---|
| control (re-arm only) | nothing: boot word stays 0 | — | — | 2,000,000 (= MIPS × 20 ms) |
| `machine Reset` | **locks up**: Renode re-guesses `VTOR` = `0x00000000` (the ITCM, not the ELF), PC `0xEFFFFFFE` | **no** | no | 0 |
| `machine Reset` + `VTOR` and cold writes re-applied | boot word and OK `0x60` again | yes, by +20 µs | yes, OK `0x60` | 1,000,000: **the counter restarts at the reset** |
| `cpu Reset` | the same | yes, +20 µs | yes | 1,000,000 (restarts too) |
| held: `machine Reset` + `VTOR`/cold + `cpu IsHalted true`, 1 ms, `cpu IsHalted false` | 0 instructions while held; after release PC = reset vector `0x08000040`, but **the first `RunFor` after the release executes nothing** | yes, +30 µs | yes | 899,000: 9 ms less the first 10 µs segment |
| held, with a 10 ns `RunFor` right after the release (`held-halted-nudge`) | only the 10 ns segment is lost | yes, +20 µs | yes | 900,000 |
| held by `CPUWAIT` (`cpu OnGPIO 4096`) | release leaves `IsHalted` True until a sync point; two segments lost | yes, +40 µs | yes | 898,000 |
| PVD: `sysbus.exti OnGPIO 16 true` | **nothing**: the hash equals the control's, `ISPR0` stays 0 | — (never stopped) | — | 2,000,000 |

- **`RSR` reads `0x00FE0000` throughout,** and `PWR` `CSR1` reads `0x6000` throughout: Renode can render neither which reset happened nor a PVD.
- **Each reboot re-ran the I2C transfer to completion.** Recovery is 20 µs of virtual time from the moment the core runs (10 µs resolution); with the release artefact it is that plus the segment the release consumed.
- **A finding the probe tripped over:** Renode's log thread splits echoed commands on stdout when the monitor is driven on stdin. Before the log went to a file (`logFile` plus `logLevel 3 console`), fragments such as `-2"` and a bare `0x20000000` landed between the sentinels, and the "hashes" differed run to run (F8). The 10 of 10 above are after that change.

### P2 — measured (`out/p2-summary.txt`, raw lines `out/p2-matrix.jsonl`)

Deck as proposed: 3.3 V source, 0.5 Ω, 400 nF, 100 mA; the sag starts at 1 ms (20 µs fall, 200 µs at the floor, 20 µs rise); the dips are 2 µs fall, 1 µs floor, 2 µs rise. libngspice in-process, `ioc` route, a breakpoint at every boundary. "Exact" is the interpolation of ngspice's accepted points; ngspice's own `meas … WHEN` agreed with it to under 1 ns at every crossing.

| Case | Rail minimum | Below `VBOR3` falling (2.61 V), ngspice | Seen at a 10 µs boundary | Seen at a 1 µs boundary |
|---|---|---|---|---|
| tolerance (3.20 V) | **3.1500 V**, above 3.135 | never | — | — |
| bor3 (2.40 V) | 2.3500 V | 1,014.422 µs → 1,225.978 µs | 1,020 → 1,230 µs (5.6 µs, 4.0 µs late) | 1,015 → 1,226 µs (0.58, 0.02 µs late) |
| pdr (1.20 V) | 1.1500 V | `VBOR0` (1.62 V): 1,015.724 → 1,224.676 µs | 1,020 → 1,230 µs | 1,016 → 1,225 µs |
| dip, between boundaries | 2.3506 V | **1,004.622 → 1,006.774 µs (2.15 µs)** | **never: missed** | 1,005 → 1,007 µs |
| dip, straddling 1,010 µs | 2.3506 V | 1,009.622 → 1,011.774 µs | 1,010 → 1,020 µs (caught, 8.2 µs late on the way up) | 1,010 → 1,012 µs |

- **The rail lags the source by 200 ns, exactly R·C**: the bor3 crossing is at 1,014.422 µs, against 1,014.222 µs for the source alone.
- **Cost of 100 ms of rail, ngspice alone** (bor3): at 10 µs quanta, 0.16 s with a 1 µs step cap and 8.3 s with ngspice's default (it caps the step at the 10 ns print step: 10,030,005 points). At 1 µs quanta, 9.9 s and 18.2 s: the 100,000 breakpoints themselves cost. The 1 µs cap gives the same crossings as the 10 ns default to 0.001 ns, so every later run uses it.
- **2101's "about 10 µs" answered:** the declared rail has no feature on a 10 µs scale. Its only time constant is R·C (200 ns), and the rest is whatever ramps the scenario declares. A quantum of q delays every crossing by up to q and drops any excursion shorter than q that falls between two boundaries. No quantum is "enough" for an arbitrary declared sag; ngspice's own crossing times are exact.

### P2.6 — end to end, with the generated firmware (`out/p26-matrix.jsonl`)

Lock-step (`ioc`): Renode is the owner's child; at each boundary Renode runs the quantum and one word (the boot word) is read; the bridge then decides from the boundary sample whether the next quantum is held (P1's held means; release at the rising threshold, plus `tRSTTEMPO` for BOR0). Schedule: the rail is solved first; the owner runs Renode segment by segment to each reset and release time, rounded up to 10 ns. Both have Renode's history at `/dev/null` and its log in a file. Each row ran twice.

| Case | Mode | Wall | Reset rendered | Boot word re-seen | Two runs identical |
|---|---|---|---|---|---|
| tolerance | lock-step 10 µs, 100 ms | 14.0, 14.5 s | none | — | yes |
| bor3 | lock-step 10 µs, 100 ms | 15.1, 15.1 s | 1,020 → 1,230 µs | 1,260 µs | yes |
| pdr | lock-step 10 µs, 100 ms | 15.1, 15.3 s | 1,020 → 1,610 µs (`tRSTTEMPO` 377 µs after 1,225 µs) | 1,640 µs | yes |
| dip, between | lock-step 10 µs, 100 ms | 15.2, 14.9 s | **none: the firmware never saw the dip** | — | yes; its event trace equals the tolerance case's |
| bor3 | lock-step 1 µs, 10 ms | 15.2 s | 1,015 → 1,228 µs | 1,241 µs | — |
| dip, between | lock-step 1 µs, 10 ms | 16.1 s | 1,005 → 1,007 µs | 1,020 µs | — |
| bor3 | lock-step 1 µs, 100 ms | **802 s** (separate lines; F15) · **54.4 s** with one repeated line per quantum | 1,015 → 1,228 µs | 1,241 µs | — |
| tolerance, bor3, pdr, dip | **schedule, 100 ms** | **1.98 to 2.06 s** (ngspice 0.10 s, Renode start 1.7 s) | at ngspice's crossings, to 10 ns: bor3 1,014.43 → 1,227.98 µs; dip 1,004.63 → 1,006.98 µs | after the release segment | yes, all four |

- **Lock-step costs 1.4 ms per quantum here, not 2101's 9 ms**, with one read per quantum, the history at `/dev/null`, and the log off stdout, over 10,000 quanta. Past that the cost per quantum grows with the commands sent, even with the history at `/dev/null` (F15); one repeated line per quantum removes the growth.
- **The release artefact shows in both modes.** In lock-step the quantum after a release executes nothing (10 µs, or 1 µs). In the schedule the next segment does; in the run above it was the 100 µs probe segment, so the final count is 100 µs short. A 10 ns `RunFor` after each release bounds it (P1, `held-halted-nudge`).

### P3 — measured (`out/p1-p3-summary.txt`)

- **The generated firmware enables no interrupt** (read at source, above). Its vector table holds the 15 core exceptions only, so any IRQ it did enable would vector through words of `.text`.
- **`SysTick` armed after the fixed point:** `PC` moves into `fault` (`0x08000096`) by +20 µs and stays there; `ExecutedInstructions` = MIPS × time continues; `CSR` shows `COUNTFLAG`. One interrupt is terminal.
- **`SysTick` armed before the boot:** the firmware **completed its boot first**: boot word written, OK `0x60`, then parked in `fault`. Every `simulate` reading the judge takes (boot word, record, instructions, `PC` inside the executable ranges) is the same as a healthy boot's except `PC`, which is still inside `.text`. **REASONED from `renode.rs`, not run: today's boot judge would pass a firmware parked in its fault handler** (F9).
- **IRQ 1 set pending (`ISPR0` = `0x2`) or EXTI line 16 raised:** pending, never taken; nothing else changes.

### P4 — measured (`out/p26-matrix.jsonl`, `out/p4-replay.jsonl`)

From the bor3 lock-step at 10 µs over 100 ms, three traces written by the real `TraceWriter`:

| Trace | Events | `trace.cbor` | Per event | Per simulated second at 10 µs |
|---|---|---|---|---|
| one kind 3 (`AnalogSample`) per quantum | 10,000 | 200,069 B | 20.0 B | **2.0 MB/s** |
| one labelled record per quantum (kind 8 as the stand-in: two texts, an i64, a u64) | 10,000 | 309,791 B | 31.0 B | **3.1 MB/s** |
| crossings, bridge actions and firmware changes only | 8 | 309 B | | 309 B per run, whatever the length |
| the same, schedule | 5 | 237 B | | |

**Fold memory, `replay_dir`, peak RSS of a fresh process, three runs each:** an empty trace 5.6 MB; 10,000 kind 3 events 5.8 to 5.9 MB (the fold is one map entry; the rest is the trace held while read); **10,000 records 9.8 to 10.0 MB, so about 428 B per record** (wire included). At `MAX_RECORDS` that is about 28 MB. One simulated second at 10 µs (100,000 records) is past `MAX_RECORDS`, and the writer refuses the 65,537th, because it folds as it writes. Measured at 1 µs × 100 ms: `quantum 65537: event 65536 is over the 65536-record bound on a run` (`out/p26-bor3-1us-100ms.json`); the kind 3 trace beside it took all 100,000 samples (1,999,951 B).

### P5 — measured by the subagent, re-checked by the lead (`out/p5-*.txt`)

| | Measured |
|---|---|
| F23 today | The CLI on the flood firmware: exit 5, `ToolError renode "output exceeded 16777216 bytes"`, at both `--for 10ms` and `--for 100ms` (`out/p5-cli-10ms.txt`, `out/p5-cli-100ms.txt`). |
| `logFile` alone | stdout 24,411,338 B at 10 ms, against 24,463,614 B without it: the console keeps printing. |
| `logFile` + `logLevel 3 console` | stdout 14,851 B, marker block intact; the file 24,654,509 B (191,220 lines) at 10 ms. |
| The flood's rate | 19,100 to 19,600 lines and 2.35 to 2.58 MB per virtual ms, 129 B a line. Renode does not collapse them. |
| At 100 ms, under the rebuilt sandbox | **the file stops at exactly 67,108,864 B; Renode dies of SIGXFSZ** (reaper status `ended signal=25`); stdout has **no marker block**. The cap is reached at about 27 virtual ms. |
| Stream reader, no file, console at its default level | exit 0 at 100 ms: 255,042,464 B and 1,993,870 lines through the reader, marker block intact, reader peak RSS 11.3 MB, 6.3 s. At 300 ms: 766 MB through, 11.2 MB RSS, 14 s. |
| At 1,000 ms | Renode's own RSS passes the 1 GiB bound and the reaper kills it (`ended bound=memory`), which the CLI already reports as `memory_cap_exceeded`, a verdict. Renode's RSS grows with the flood: 430 MB at 10 ms, 600 MB at 100 ms, 877 MB at 300 ms. |
| `locked_peripheral` | Counted from the stream: 1,993,441 at 100 ms. **The count differs run to run under a flood** (191.0k to 193.7k at 10 ms) while `ExecutedInstructions` is identical (F10). |

## The spike's table: each prediction beside its result

| id | Predicted | Result | Held? |
|---|---|---|---|
| P1.1 | `machine Reset` alone: `VTOR` re-guessed `0x08000000`, reboots | `VTOR` re-guessed **`0x00000000`** (ITCM); PC `0xEFFFFFFE`; never reboots | **no** |
| P1.2 | + `VTOR` and cold writes: reboots, reads equal the first boot's | reboots by +20 µs; boot word, record and registers equal; `ExecutedInstructions` restarts | yes |
| P1.3 | `cpu Reset`: reboots, read OK | reboots by +20 µs, OK `0x60` | yes |
| P1.4 | held by `IsHalted`: 0 instructions while held, reboots after | yes, but the first `RunFor` after the release executes nothing | partly |
| P1.5 | held by `CPUWAIT`: as P1.4 | reboots, two segments lost after the release | partly |
| P1.6 | PVD by EXTI 16: nothing observable | hash identical to the control | yes |
| P1.7 | recovery within 100 µs | 20 µs of running time (30 to 40 µs with the release artefact) | yes |
| P1.8 | 10 of 10 identical | 10 of 10 in every case, once the log left stdout | yes, after F8 |
| P1.9 | instructions = MIPS × time without a hold; short by the hold with one | **the counter restarts at every reset**; held runs are also short by the lost segment | **no** |
| P2.1 | follows the source less 50 mV, lag ≈ 200 ns; tolerance minimum 3.150 V | 200 ns lag measured; minimum 3.1500 V | yes |
| P2.2 | each crossing up to one quantum late | 0.02 to 8.2 µs late at 10 µs; 0.02 to 0.85 µs at 1 µs | yes |
| P2.3 | the between-boundaries dip missed at 10 µs, caught at 1 µs and by ngspice | exactly so | yes |
| P2.4 | 100 ms of rail: < 1 s at 10 µs, < 5 s at 1 µs | 0.16 s and **9.9 s** with a 1 µs step cap; 8.3 s and 18.2 s with ngspice's default | **no** at 1 µs |
| P2.5 | no 10 µs feature in this rail; the schedule needs no quantum | measured: only R·C (200 ns) and the declared ramps; the schedule renders every case exactly | yes |
| P2.6 | lock-step ≈ 90 s (10 µs) and ≈ 700 s (1 µs) per 100 ms; schedule < 5 s | **14 to 15 s** at 10 µs; at 1 µs **802 s**, from 1.3 ms per quantum growing to 17 ms (F15); schedule **2.0 s** | **no**: 10 µs far faster; 1 µs slower than predicted |
| P3.1 | no interrupt enabled | none | yes |
| P3.2 | `SysTick` after: parks in `fault`, instructions continue | exactly so | yes |
| P3.3 | `SysTick` before: parks before the boot word | **the boot completed first**, then it parked | **no** |
| P3.4 | IRQ 1 pending or EXTI 16: no effect | `ISPR0` = 2, never taken | yes |
| P4.1 | kind 3 ≈ 20 B, ≈ 2 MB/s | 20.0 B, 2.0 MB/s | yes |
| P4.2 | a record 35 to 45 B, 3.5 to 4.5 MB/s | 31.0 B, **3.1 MB/s** (with the kind 8 stand-in) | **no**, lower |
| P4.3 | kind 3 O(1); a record ≈ 350 B held | kind 3 O(1); a record **≈ 428 B** | partly |
| P4.4 | crossings only < 1 KB per droop | 309 B (lock-step), 237 B (schedule) | yes |
| P4.5 | real reader within 20 % of P4.3 | 22 % above for records | **no**, narrowly |
| P5.1 | `logFile` alone leaves the flood on stdout; `logLevel 3 console` removes it | exactly so | yes |
| P5.2 | ≈ 24k lines, 3 MB per ms; the file hits 64 MiB at 100 ms; exit 5 | 19.1 to 19.6k lines, 2.35 to 2.58 MB per ms; the cap at ≈ 27 ms; **SIGXFSZ, no marker block** | yes in outcome, numbers ≈ 20 % lower |
| P5.3 | a stream reader returns whatever the flood | to 300 ms with 11 MB of reader RSS; at 1 s Renode's own 1 GiB bound, a verdict | yes |

## Recommendations, one per question

**P1 → render a brownout as a held reset, by `machine Reset`, then the cold writes and `cpu VectorTableOffset` re-applied, then `cpu IsHalted true`; release by `cpu IsHalted false` and a 10 ns `RunFor`.** Measured deterministic, 20 µs from running to a re-written boot word and an OK read.
- `machine Reset` alone is not usable (it locks up), and `cpu Reset` leaves the peripherals live, which a brownout does not.
- The re-arm of the boot word and record, the reset and the release are the bridge's actions and are recorded as the model's rendering (`RUL-17`). Renode has no PVD, no BOR and no reset cause, and the trace must not imply otherwise.
- The judge's instruction rule changes for a dynamic scenario: the counter restarts at each reset, so the expectation is MIPS × (time since the last release, less the nudge), stated in the verdict.
- The recovery bound for §2.3's verdict: measured 20 µs; propose 100 µs.

**P2 → declare the droop in the scenario, compute the rail once before the boot, and apply its crossings as events (the schedule), not per-quantum lock-step.**
- The scenario declares the sag (domain, floor in mV, start, fall, hold and rise in ns), the source resistance (mΩ) and the load (mA). The specification declares none of these. The decoupling is the emitter's rule (one 100 nF per IC on that supply).
- The schedule is exact (ngspice's crossings to 10 ns), 2.0 s per 100 ms scenario, byte-identical on repeat, and cannot miss a dip. Lock-step at 10 µs takes 15 s and **missed a 2.15 µs crossing of `VBOR3` outright**.
- In-run lock-step is needed only when the analog side depends on what the firmware does (a firmware-driven load). No declared input carries that today, so it is deferred to the first scenario that declares one (`AET-WO-2101` F9).
- The bridge still owns Renode as its child (the schedule drives it), so `RUL-17` holds as written. Its "inversion of control at about 10 µs for a dynamic one" does not; that is decision D1.

**P3 → defer storms to the first board whose firmware takes an interrupt (P4's IMU data-ready).** Today an interrupt either parks the firmware in `fault` (any core exception) or is never taken (any IRQ). Neither is a storm, and neither exercises anything the firmware does.

**P4 (`OQ-03`, dynamic) → with the schedule, record crossings, bridge actions and firmware changes as records: about 300 B per run, whatever the virtual time. `MAX_RECORDS` stays at 65,536.**
- **Nothing is sampled, because no per-quantum state exists.** That is the reading for Scotty to confirm (D8), not one this spike settles.
- If D1 rules lock-step instead, the per-quantum rail goes as kind 3 (`AnalogSample`): 2.0 MB per simulated second at 10 µs, with a fold that does not grow. The 256 MiB trace cap then holds about 134 s per run.
- Per-quantum records cost 3.1 MB/s on the wire and about 428 B each in the fold, and pass `MAX_RECORDS` at 0.66 s; that is not recommended.

**P5 → the bridge, which owns Renode, reads Renode's stdout as a stream.** It keeps the marker block and a per-kind count of log lines in bounded memory, with no log file. It reports the counts and the first line of each kind.
- A log file is **not** the answer as worded. The sandbox's 64 MiB `RLIMIT_FSIZE` kills Renode at about 27 virtual ms of flood, and the marker block is lost, exit 5 as before.
- With the stream, a flood reaches a verdict up to the point where Renode's own memory passes 1 GiB, about 0.3 to 1 s of flood, and past it the bound's verdict, `memory_cap_exceeded`.
- The monitor history goes to `/dev/null` through Renode's own `history-path` setting (`ShellProvider.cs` L33), measured in every run here; no symlink is needed. That alone does not stop the monitor slowing (F15): a bridge that sends Renode many commands sends repeated lines.

## §7 checked

- Renode 1.17.0 **can** render a reset and a held reset from the monitor (P1). Not a stop.
- Neither reference droop takes near 10 minutes of CI: 2.0 s by the schedule, 15 s by lock-step at 10 µs. At 1 µs, lock-step as the probe first drove it took **802 s, past 10 minutes**. That would be §7's stop if 1 µs were the ruled quantum. With one repeated line per quantum (F15) it took 54.4 s (`out/p26-oneline-1us-100ms.json`), with the same actions and final reads. The recommendation rules no quantum. Not a stop as recommended; flagged for D1.
- No static scenario's records or verdict change: nothing outside `spike/2301/` and this file changed. Not a stop.
- `INV-04` needs no sampling under the recommendation. Under lock-step with crossings-only records it would be sampling, and that goes to Scotty (D8).

## Findings

- **F1 · DS12117 was read from a third-party mirror of Rev 7 (2019), and RM0433 not at all · source · medium · provenance · decision-needed (D10).** st.com and two distributors hung from Furnace; other mirrors return bot-check pages, which were not worked around. The values above are Rev 7's; the current revision was not opened.
- **F2 · The BOR level is unknown · the part's option bytes · high · gap · decision-needed (D2).** §3.5.2 of DS12117 says the three BOR thresholds are configured through option bytes. The specification does not declare one, the generated firmware writes none, Renode models none, and the delivered default (RM0433) was not found.
- **F3 · `machine Reset` re-guesses `VTOR` as `0x00000000` and the core locks up · Renode 1.17.0 `CortexM.InitPCAndSP` · high for the build · measured.** The guess takes the first non-empty section on the bus, which on this platform is the ITCM at 0, not the ELF. The CLI's own `cpu VectorTableOffset` (`AET-WO-2100` A3) must be re-applied after every reset, with the cold writes.
- **F4 · `ExecutedInstructions` restarts at every reset · Renode 1.17.0 · medium · measured · decision-needed (D5).** The judge's "instructions = MIPS × time" cannot hold across a reset as written.
- **F5 · A release takes effect at the end of the next `RunFor`, whatever its length · Renode 1.17.0 · medium · measured.** By `IsHalted`, one segment is lost; by `CPUWAIT`, two. A 10 ns `RunFor` right after the release bounds the loss to 10 ns (`held-halted-nudge`).
- **F6 · Renode renders neither a PVD, a BOR nor a reset's cause · `stm32h7.repl` L728–746, `STM32H7_RCC.cs` L672–702 · info · read and measured.** `PWR` is a Python stub with constant reads, and `RSR` reads `0x00FE0000` after every reset. Raising EXTI line 16 changes nothing.
- **F7 · Per-quantum lock-step drops excursions shorter than the quantum · the `ioc` route · high · measured.** A 2.15 µs crossing of `VBOR3` between two 10 µs boundaries never reached the firmware, and its trace equals the tolerance case's. Every crossing it does see is up to one quantum late.
- **F8 · Renode's log thread splits echoed commands on stdout when the monitor is driven on stdin · Renode 1.17.0 · medium · measured.** Fragments such as `-2"` and a bare `0x20000000` landed between sentinels, so a whole-line parse can read an echoed address as a value. With the log in a file and `logLevel 3 console` the runs were 10 of 10 identical. A build that keeps the console at its default level, so as to count log lines on stdout (P5), must parse against this. Sending the log to a FIFO the bridge reads (no `RLIMIT_FSIZE` on a pipe) would separate the channels. That is REASONED, not measured (D6).
- **F9 · A firmware parked in its fault handler would pass today's boot judge · `renode.rs` `judge_boot` · medium · REASONED from the code; the readings are measured (`systick-before`).** The boot word and record are written, the instruction count equals MIPS × time, and `PC` is inside the executable ranges. Recommend the judge require `PC` at the fixed point, in `AET-WO-2204` or later.
- **F10 · Under a log flood, the count of locked-peripheral warnings differs run to run · Renode 1.17.0 · medium · `INV-04` for the finding's message · measured by the subagent.** 191.0k to 193.7k at 10 ms, with `ExecutedInstructions` identical. The `locked_peripheral_access` finding prints the count. Cause not found.
- **F11 · Lock-step costs 1.4 ms per quantum, not 9 ms, over the first 10,000 quanta · supersedes `AET-WO-2101` S4 · info · measured.** One read per quantum, the history at `/dev/null`, the log off stdout. 10 µs × 100 ms = 14 to 15 s. It grows after that (F15).
- **F12 · ngspice caps its step at the print step when no `TMAX` is given · the deck · low · measured.** `.tran 10n 100m` accepts 10 million points (8.3 s); a 1 µs cap gives the same crossings to 0.001 ns in 0.16 s.
- **F13 · The spike's first runner read the caps after the container started; six short containers ended before it read them · `spike/2301/img.sh` · low · process · fixed.** By timestamp they are quick builds, a smoke run and one start that failed at once (`/usr/bin/time` is not in the image). The runner logged no command, so this cannot be proved. The runner now holds the command until swap is zeroed and the caps are read back, and logs the command beside them. P1, P2, P2.6, P4 and the nudge case were re-run under it, each container with its caps and command logged: every P1 hash, every P2 and P2.6 line, and every P4 event count equal the first runs, apart from wall times (`out/*-rerun.*`, `out/p1-held-halted-nudge.txt`). The P5 subagent's containers went through the first runner; their caps lines are in `out/caps.log` without commands.
- **F14 · P1 and P2.6 boot `boards/sentinel-01/sentinel-01.repl`, not the CLI's emitted copy · probe · low.** It is the same generated file, which the regeneration test pins. The P5 subagent did the same.
- **F15 · Renode's monitor still slows with every distinct command when its history file is `/dev/null` · AntShell at `ca49bd32` (Renode 1.17.0's pin) · high for any per-quantum lock-step · measured, cause read at source.**
  - `Shell.HandleCommand` adds each command to an in-memory list and then rewrites the whole list to `history-path` (`AntShell/Shell.cs` L213–217; `CommandHistory.cs` L33–40, L180–188). `/dev/null` makes the write cheap but not the enumeration, so the cost per command grows with the commands so far.
  - Measured at 1 µs × 100 ms (separate lines per quantum): 1.31 ms per quantum at quantum 1,000; 1.82 at 10,000; 5.09 at 40,000; 11.8 at 71,000 (from the `Machine resumed` timestamps of its log).
  - `CommandHistory.Add` skips a command equal to the previous one. Sending each quantum as one `;`-joined line with a constant sentinel (safe, since each quantum waits for its own) stops the growth: 1 µs × 10 ms took 8.6 s against 15.0 s, with byte-identical traces, and 1 µs × 30 ms took 20.0 s, linear (`out/p26-oneline.txt`).
  - `AET-WO-2200` L6's test ("100,000 commands do not grow memory") does not catch this: it is time, not memory. A build that drives the monitor per quantum needs the repeated line, or a test on time per command.

## Decisions needed

- **D1 (architect; `RUL-17` wording).** A declared droop rendered by the **schedule** (rail solved before the boot, crossings applied as events, the bridge still owning Renode), not per-quantum `ioc` lock-step at 10 µs. Lock-step deferred to the first firmware-dependent analog load. Recommended (P2, F7).
- **D2 (architect, perhaps Scotty).** Where the BOR level comes from: (a) a scenario field, declared and refused when absent; (b) a specification field (`aethelgard-ir`, outside this fence); (c) the delivered default, once RM0433 is read. Recommend (a) for this work order and (b) later. No default is substituted (`INV-06`).
- **D3 (architect).** Which value of each threshold the bridge uses. Recommend typical for the rendering, and validation refusing a scenario whose floor or recovery lies inside a threshold's min–max band, since the part's own behaviour there is not stated.
- **D4 (architect).** The release: at the rising threshold, plus `tRSTTEMPO` (377 µs) only after BOR0 as Table 26 states, then a 10 ns `RunFor` (F5).
- **D5 (architect).** The instruction rule for a dynamic scenario: MIPS × (time since the last release, less the nudge), stated in the verdict (F4).
- **D6 (architect; §2.4's "Renode's log goes to a file").** A plain file fails under `RLIMIT_FSIZE`. Recommend the stream on stdout, as measured, with F8's parse. The FIFO alternative is to be measured at the build if preferred.
- **D7 (architect).** The decoupling on the rail comes from the emitter's rule (one 100 nF per IC on the supply), and the scenario declares the source resistance and the load. Recommended.
- **D8 (Scotty, `INV-04`).** Under D1's schedule, recording crossings, actions and firmware changes leaves out no state, because no per-quantum state exists. The recommendation is that this is **not sampling**, and Scotty's confirmation is asked. If D1 rules lock-step, the per-quantum rail goes as kind 3 at 2.0 MB/s (no sampling), unless Scotty rules a sampling rule.
- **D9 (architect).** F9's judge gap: which work order.
- **D10 (Scotty).** The ST PDFs: DS12117's current revision and RM0433, placed in `~/.cache/aet-2301/ds/` from a host that reaches st.com, so F1 and F2 can be closed.

## Gates (spike)

- **Workspace gates,** run in the pinned image on this branch: `cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo test --workspace`: 65 suites, 954 passed, 0 failed. The workspace is `e3e3cd2`'s; nothing in it changed.
- **The probe crates** build with no warnings (`cargo build --release` in the image), and `spike/2301/lockstep` is `cargo fmt`-clean.
- **Adversarial pass:** at the build, by a fresh subagent, as the kickoff sets it. This PR is the spike only.
- **Reduction pass (diff):** −162 lines net (two files of 158 lines, three lines in `levels()`, two `mod` lines less one comment) · 0 duplicates removed · 3 dead paths removed (`ng-probe`'s `pattern.rs` and `sha256.rs`, unused by this owner, and a no-op filter in `levels()`) · 0 protected kept · 1 finding: the CLI's boot prefix (the cold registers, `VTOR`, the re-arm) is restated in `renode/drive.py`, `lockstep/src/main.rs` and the P5 scripts, in two languages. It is kept, because the build deletes `spike/` whole; a build that drives Renode itself takes it from `renode.rs` `BootPlan` alone.
- **`git diff --stat e3e3cd2 -- . ':!spike/2301' ':!docs/work-orders/AET-WO-2301.FEEDBACK.md'`** is empty before the merge of `origin/main`.
