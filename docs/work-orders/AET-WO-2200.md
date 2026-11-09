# AET-WO-2200 — The analog bridge, and the static rise-time fault (47 kΩ fails, 2.2 kΩ passes)

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-2200-analog-bridge`.
**Author:** Claude (architect) · **Builder:** a strong builder, with a **full** adversarial pass (`docs/roadmap.md` §6). `OQ-02` is where a simulation runs but measures nothing.
**Phase:** P2. It builds the transport and fault path that `OQ-02` ruled, and that Scotty ratified on 2026-11-09. It delivers the analog half of the gate's second clause. Archive and replay are `AET-WO-2300`'s.

**Grounds:** `AET-WO-2101`'s spike and the architect's ruling on PR #66, L1–L7. Read both in full; this work order does not restate them. The short form:
- **L1:** `libngspice` runs in-process and is stepped synchronously, with `SetBkpt` at the edges.
- **L2:** the `unsafe` lives in one crate outside the workspace, `bridge/`, in its `ffi` module only. The CLI runs the bridge binary as a sandboxed tool.
- **L3:** the FFI is hand-declared against a vendored, hash-pinned `sharedspice.h`, and a test checks the signatures. There is no image change for headers.
- **L4:** the fault is the sensor's absence, and the trace labels it as the model's rendering of a NACK.
- **L5:** for a static fault, the analog pass runs before the boot.
- **L6:** Renode's monitor history goes to `/dev/null`.
- **The research pass of 2026-11-09:** no open tool couples Renode to a SPICE solver. Adopt one suggestion: the bridge's internal solver trait has an FMI 3.0 Co-Simulation shape (instantiate, set inputs, `do_step(t, h)` with early return, read outputs), without the dependency.

> **What this is:**
> - a bridge binary that measures each I2C line's rise time from the solved topology;
> - an analog pre-pass in `simulate --firmware`, whose decision removes a failing bus's devices from the emitted platform;
> - a verdict that carries the measured evidence and the firmware's effect as separate records.
>
> It is NOT:
> - in-run lock-step, or a dynamic fault such as droop (a later work order);
> - archive or replay (`AET-WO-2300`);
> - extracted parasitics (`AET-WO-1303`);
> - the firmware's I2C transaction itself (`AET-WO-2202`, in parallel).

**Cadence:** build-only, with one measurement stop (§7). One PR, left unmerged.

## 1. Scope

1. **The analog facts reach the twin through `SolvedTopology` (`RUL-16`).**
   - It gains `analog_lines: BTreeMap<net, AnalogLine { bus, pull_up_ohms, capacitance_femtofarads, ceiling_ns }>` for every I2C line. The values are taken from the specification's bus and net constraints, and the ceiling is the declared `max_rise_time_nanoseconds` (`AET-WO-1202`).
   - `validate` refuses a zero or missing value, and a key that is not a solved I2C net.
   - Golden digests move. Record the old and new values in FEEDBACK.
2. **`aethelgard-twin::analog`** emits one deterministic netlist per line from `analog_lines`:
   - the S2 deck, with R to the domain voltage, C to ground, and the open-drain switch;
   - a byte pattern with one release edge inside a 22.5 µs window;
   - `.options` exactly as in S1.

   Two emissions are byte-identical.
3. **`bridge/`**, a crate excluded from the workspace (as `boards/` is), builds `aethelgard-bridge`:
   - **Interface:** it reads one request file of netlists and nets, and writes one JSON line per net to stdout: `{net, rise_30_70_ns, method:"interpolated", ngspice:"44.2"}`.
   - **Measurement:** it runs `ngSpice_Init_Sync`, with `SetBkpt` at the stimulus edges. The rise time comes from its own interpolation of accepted points, cross-checked against ngspice's `meas`; disagreement beyond 0.01 ns is an error.
   - **`unsafe`:** the crate is `#![deny(unsafe_code)]`, with `#[allow(unsafe_code)]` on `ffi` only.
   - **The header:** `sharedspice.h` is vendored at the `ngspice-44.2` tag, with ngspice's licence and a `SOURCE.md` pinning its sha256. A test parses it and asserts that every declared signature matches.
   - **The solver trait** has the FMI-3 shape above.
   - It links `libngspice.so.0` from the image. It has no dependency except `serde_json`, and no network.
4. **The pre-pass in `simulate --firmware --spec`.**
   1. Solve and emit the netlists.
   2. Run the bridge as a tool, under the sandbox, with `RUL-13`'s bounds and a `bridge_unavailable` refusal if the binary is missing (`INV-06`). The CLI finds it beside its own executable.
   3. For each line, record `analog.rise_time_measured`.
   4. If a line's rise exceeds its ceiling, record `bridge.fault_state` and emit the `.repl` without that bus's devices.
   5. Boot, and record the firmware's observation as today.

   The platform-identity check (`PlatformNotEmitted`) compares against the emission for the specification plus the recorded fault decision. The verdict gains `analog: [...]` and `faults: [...]`, separate from `boot`. The TXIS timeout is labelled as the model's rendering, never as a measured waveform (S6).
5. **The images and CI.**
   - `engine.Dockerfile` builds and installs `aethelgard-bridge` beside `aethelgard`, and the engine's one layer holds exactly those two binaries. Its layer test is updated.
   - The `tools` job builds the bridge before tier 2.
   - The bridge is not published to crates.io. `docs/releasing.md` says so in one line.

## 2. Invariants and fences

- `INV-04` (two runs identical; archive replay is `2300`'s), `INV-06`, `INV-07`, `RUL-05`, `RUL-12`, `RUL-13`, `RUL-16`.
- **No `unsafe` in any workspace crate.** In `bridge/`, `unsafe` appears in `ffi` only, and FEEDBACK counts its lines.
- **Fence:**
  - `aethelgard-ir`: `SolvedTopology`, `validate` and `solve`'s copy of the analog facts;
  - `aethelgard-twin`: the new `analog` module, and `platform.rs`'s device omission;
  - the CLI: `simulate`'s pre-pass and verdict fields, and its README;
  - `bridge/` (new);
  - `container/engine.Dockerfile` and its smoke test;
  - the CI and engine-image workflow steps for the bridge;
  - one line of `docs/releasing.md`;
  - tests;
  - this work order's FEEDBACK.
- **`AET-WO-2202` runs in parallel.** It touches `aethelgard-ir::parts`, `twin::boot`, the svd-hal firmware generator and the CLI's boot judging. Do not touch those. Expect to merge `main` if it lands first.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance: P2 gate clause 2, analog half, as one tier-2 test

1. **At 2.2 kΩ,** the reference specification gives `analog.rise_time_measured` = 186.40 ns ± 0.05 against a ceiling of 300 for both lines, no fault, and a boot that passes.
2. **At 47 kΩ** (the same specification with `pull_up_ohms: 47000`):
   - the measured rise is 3,982.30 ns ± 0.5, exceeding 300;
   - `bridge.fault_state` names the bus;
   - the BME280 is absent from the platform;
   - the verdict is `fail`. It names the bus, the measured rise and the ceiling, and, as a separate record, the firmware's observation.

   Before `AET-WO-2202` lands, that observation is the boot witnesses. After it, it is the TXIS timeout.
3. **Each case run twice gives byte-identical `analog` and `faults` arrays.**

§1.4 passes in `gates` and `tools`.

## 5. Adversarial pass — full

1. Make a 47 kΩ bus pass:
   - stale netlists;
   - a fault decision that doesn't reach the platform;
   - a platform-check bypass;
   - a measurement taken from the wrong edge.
2. Make the bridge disagree with itself across runs, or with ngspice's `meas`, silently.
3. Reach outside the sandbox from the bridge, or crash it into a pass.
4. Feed hostile `analog_lines` values: zero, huge or NaN-making.
5. Check the `ffi` module line by line, against the vendored header.

## 7. Flag-and-stop conditions

- The measured 2.2 kΩ or 47 kΩ rise differs from S2 by more than its tolerance: stop and report.
- The bridge needs any sandbox grant beyond what the spike's S5 measured.
- A change outside the fence.
- A test that can only go green by being weakened.
