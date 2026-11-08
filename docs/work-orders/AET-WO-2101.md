# AET-WO-2101 — The lock-step bridge: how the analog solver and the twin exchange state, and how a fault reaches the firmware (spike)

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-2101-lockstep-spike`.
**Author:** Claude (architect) · **Builder:** a strong builder, with a full adversarial pass at the build (`docs/roadmap.md` §6). This work order is the spike only.
**Phase:** P2. It decides `OQ-02`, which **Scotty ratifies** (`architecture.md` §8). It blocks `AET-WO-2200` and `AET-WO-2300`.

**Cadence: SPIKE, then STOP for the architect's ruling and Scotty's ratification.** Probes live under `spike/2101/`. Nothing under `crates/`, `container/`, `.github/` or the protected documents changes in this PR.

## 0. What the desk spike already read at source (2026-11-08)

The architect's desk spike read these at pinned sources: Renode `v1.17.0` (`ab721d88`, infrastructure `066a7f13`) and ngspice `ngspice-44.2` (`80d22fc8`). The full cited report is `docs/work-orders/AET-WO-2101.desk.md`, and you start from it. Do not re-derive it.

1. **ngspice can be stepped on the caller's thread.** `sharedspice.h` declares `ngSpice_Init_Sync(GetVSRCData*, GetISRCData*, GetSyncData*, …)`.
   - A `V… external` source takes its value from `GetVSRCData` at every load.
   - `GetSyncData` fires on each accepted or rejected step. It can rewrite the next delta, or redo the step by returning 1.
   - `ngSpice_Command("run")`, without `bg_`, blocks the caller's thread, and the callbacks run on that thread.
   - Debian's `libngspice0` 44.2 is built `--with-ngshared`.
   - The manual's `GetSyncData` signature, with 5 parameters, is stale. The header has 7; follow the header.
2. **The FFI needs `unsafe`, and no workspace crate may contain it.** An in-process route means a crate outside the workspace, the way `boards/` is excluded, with its own audit. The alternative is ngspice as a sandboxed child.
3. **Renode exposes no I2C transaction hook to an external process.**
   - External Control (TCP, a C client) offers ADC, GPIO, SPI, CAN and the system bus, but no I2C.
   - Co-simulation (TCP, or an in-process native library, with C++ or SystemVerilog adapters) is bus-mapped only.
   - The sandbox refuses every socket in any case.
4. **What exists inside Renode for a project-written I2C device:**
   - **`Mocks.DummyI2CSlave`** is an I2C device whose `ReadRequested(count)`, `DataReceived(bytes)` and `TransmissionFinished` events take Python handlers. A `Read` short of its count is padded with zeros. Upstream uses this pattern in `tests/platforms/Renesas_DA14592.robot` with `echo-i2c-peripheral.py`.
   - **C# loaded at run time** (`include @file.cs`, compiled by Roslyn) works on 1.17.0's .NET build.
   - **The stock models:** `BME280` (I2C) exists; `TMP117` and `TMP102` do not; `TMP108` is a stub of `TMP103`. `GenericSpiFlash` takes a JEDEC ID.
5. **The STM32 I2C controller (`STM32F7_I2C`) signals no NACK.**
   - An absent address logs a warning and returns. TC is never set.
   - A short read sets TC early.
   - An empty RX queue reads 0 with a "Receive buffer underflow" warning.

   So a fault can reach the firmware only as: no completion (a timeout in the firmware), a short transfer, or wrong data.

## 1. Questions the spike answers, each with its command and output in FEEDBACK

**S1. Argon's measurement.** Measure the wall-clock cost of 1,000 synchronous ngspice transients, each one I2C byte window long. That is 9 bit times at 400 kHz, about 22.5 µs, on an RC network that models one line:
- the declared pull-up;
- the declared capacitance;
- an open-drain driver as a switched source.

Run it three ways, and report the per-transient time, its spread, and whether two runs give byte-identical results:
- **a.** one `ngspice -b` process per transient;
- **b.** one long-lived `ngspice` process in interactive or pipe mode, re-running or `alter`ing the circuit;
- **c.** `libngspice` with `ngSpice_Init_Sync`, from a probe host binary outside the workspace. Record its `unsafe` surface in lines.

**S2. The measured rise time against the ceiling.** Use the reference's 2.2 kΩ and 100 pF, and then the P2 gate's 47 kΩ. Report:
- the 30 %–70 % rise time ngspice measures in each route, set against `0.8473 × R × C`;
- whether the 47 kΩ case exceeds the reference bus's solved ceiling at 300 ns.

**S3. How a fault reaches the firmware.** Build probe firmware under `spike/2101/` that completes one I2C read of the reference sensor's ID register, and that times out on a bus that never completes. Measure three routes. For each, report what the firmware observes, whether the outcome is deterministic across 10 runs, and what it adds under `RUL-12` (a language layer):
- **i.** `DummyI2CSlave` plus a Python handler, emitted beside the `.repl`. The handler answers as the sensor, and on a fault it withholds or corrupts the bytes.
- **ii.** a C# sensor model loaded at run time.
- **iii.** stock models only. The fault is the device's absence from the platform: removed or never declared.

**S4. Lock-step.** Drive both simulators from one owner, which is the CLI's process. In each quantum it:
1. tells Renode, through the monitor on stdin (already this project's channel), `emulation RunFor <q>`;
2. advances ngspice to the same time;
3. sets the fault state for the next quantum in the sensor model, by a monitor command.

Report the cost of one quantum and the total cost for the P2 demo's span. Report whether two runs give the same state-sequence hash (`INV-04`). Say which quantum is enough for a static rise-time fault and which for a rail droop.

**S5. The sandbox.** Run each candidate route under the CLI's existing sandbox (`run.rs`), with its socket refusal, its inherited-descriptor check and Landlock. Report what breaks. A named FIFO in the run's temporary directory is the sanctioned channel if one is needed (`architecture.md` §8).

**S6. Trace honesty.** Show where the bridge records two separate events:
- the **measured** rise time, from ngspice, as the evidence;
- the **effect the firmware saw**, from Renode or the probe firmware.

Neither may stand in for the other (`architecture.md` §8).

**S7. A proposal.** In 300 words or fewer, propose the route for each half:
- the transport (a, b or c);
- the fault path (i, ii or iii);
- the quantum;
- where any `unsafe` or any non-Rust file lives, and why that is or is not a `RUL-12` amendment.

Then sketch the end-to-end shape of the P2 gate's 47 kΩ / 2.2 kΩ campaign under it.

## 2. Invariants and fences

- `INV-04` (replayable traces), `INV-06`, `INV-07`, `RUL-05`, `RUL-12`, `RUL-13`. No `unsafe` in any workspace crate. A probe host outside the workspace may have `unsafe`; report its lines.
- **Fence:**
  - `spike/2101/**`;
  - this work order's FEEDBACK.

  Everything else diffs to empty.
- The tools image already carries `ngspice` and `libngspice0` 44.2 and Renode 1.17.0. Measure with them, in the image under podman. Anything the image lacks is a finding, not an install.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance

- Every S1–S7 answer is in FEEDBACK, with its command and the output file it came from under `spike/2101/out/`.
- The PR says **STOP: spike for the architect's ruling and Scotty's ratification of `OQ-02`.**

## 7. Flag-and-stop conditions

- None of S1's routes completes a transient.
- No route in S3 gets one I2C read to the firmware.
- A route needs a socket, or an inherited descriptor above 2. Measure it, and record it as a finding.
- The spike needs a change outside `spike/2101/`.
