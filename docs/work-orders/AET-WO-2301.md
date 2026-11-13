# AET-WO-2301 — Dynamic faults: a rail droop in lock-step, and what an interrupt storm means for the generated firmware

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-2301-dynamic-faults`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass at the build (`docs/roadmap.md` §6).
**Phase:** P2's remainder, the first of P4's prerequisites. The order is `2301`, then `4200`, then `4100` (Scotty, 2026-11-12).
**Cadence: SPIKE (§1), then STOP for the architect's ruling, then BUILD (§2) in this same work order.** If the spike proposes sampling, `OQ-03`'s answer goes to Scotty (`INV-04`).

**Grounds:**
- **`AET-WO-2101` measured lock-step already.** Start from its FEEDBACK's S4 to S7; do not re-derive them.
  - Renode takes monitor commands on piped stdin, line by line.
  - The `ioc` route (ngspice's `tran` with a breakpoint at every quantum, and Renode's quantum run from inside `GetSyncData`) runs under the unchanged sandbox with no extra grant, deterministically. It took about 9 s per 10 ms of simulated time at 10 µs quanta.
  - Renode's monitor costs about 2 ms per quantum, and the history fix keeps it flat (2101 F16).
  - A static fault needs no in-run lock-step.
  - A rail droop needs about 10 µs quanta. That was **reasoned, not measured** (2101 S4).
- **`RUL-17`:**
  - the bridge may own Renode as its child;
  - the bridge's solver trait has the FMI 3.0 Co-Simulation shape;
  - the trace records the measured analog quantity and the firmware's effect as separate records;
  - an effect the model renders is labelled as the model's rendering.
- **There is no power model.** The northstar's "the brownout the regulator permits is one the firmware recovers from" has no work order before `4300` (the Mythos review, adopted).
  - Renode 1.17.0's STM32H7 has no brownout model known to this project.
  - The generated firmware enables no interrupt known to this project.
- **`OQ-03` is still open for dynamic campaigns.** The estimate is 3 to 6 MB per simulated second at 10 µs quanta. The reader's `MAX_RECORDS` is 65,536 (`AET-WO-1900`), and records fold at about 35 times their wire size.

> **What this is:** a rail droop as a scenario fault, run in lock-step. ngspice computes the rail, the bridge decides each quantum what the firmware sees, and the archive records both, separately and labelled. Interrupt storms are built only if the spike shows they mean something for the generated firmware.
>
> It is NOT:
> - a regulator design or a predicted regulator behaviour (`4300`);
> - a change to static campaigns' records or verdicts;
> - the I2C witness hardening (`AET-WO-2204`).

## 1. Spike: questions answered with commands and outputs in FEEDBACK

Probes live under `spike/2301/`. **Predictions go into FEEDBACK before any run.**

**P1. What does the generated firmware do under a droop, in Renode 1.17.0, by which means?**
- **Read and cite at source:**
  - the STM32H753 datasheet's (DS12117) POR, PDR and BOR thresholds and hysteresis;
  - VDD's operating range;
  - what Renode 1.17.0's STM32H7 platform models of `PWR` (PVD, BOR) and of reset.
- **Measure, by monitor command, between quanta:**
  - a reset (`machine Reset` or the CPU's reset);
  - holding the CPU in reset for a stated time, then releasing it;
  - whatever PVD or BOR signal Renode has.
- **Report for each means:**
  - what the generated firmware observes;
  - whether it boots again;
  - whether its I2C read resumes;
  - whether the outcome is deterministic over 10 runs.

**P2. How is the rail computed, and what does a droop scenario declare?**
- **The honest minimum** is the declared rail (the specification's voltage domain, nominal and tolerance) driven through a stated source impedance into the decoupling the BOM declares, with a declared load step or source sag as the fault. Propose it, or something better and still declared.
- **Measure ngspice's rail waveform** for 3 cases at 10 µs and at 1 µs quanta:
  - a droop inside the domain's tolerance;
  - one that crosses BOR;
  - one below PDR.
- **Answer 2101's reasoned 10 µs claim** with a measurement.

**P3. What does an interrupt storm mean for the generated firmware today?**
- List every interrupt the generated firmware enables, from `boards/sentinel-01/` and the board crate generator.
- If none is enabled, say what a storm could be:
  - an EXTI line with no handler;
  - SysTick;
  - a peripheral's error interrupt;
  - …
- Measure the one cheapest candidate, if any is meaningful. Otherwise **recommend deferring storms to the first board whose firmware takes an interrupt** (P4's IMU data-ready).

**P4. `OQ-03`, dynamic.**
- Measure bytes per simulated second of the per-quantum records the build would write, at 10 µs quanta, over 100 ms, with a droop.
- Measure the fold memory per record.
- **Propose:**
  - full fidelity with `MAX_RECORDS` raised (state the new bound and its memory); or
  - streaming replay; or
  - a sampling rule. Sampling goes to Scotty, because it touches `INV-04`.

**P5. The log flood (`AET-WO-2100` F23).** Show that Renode writes its log to a file in the sandbox's scratch (for example `logFile @<path>`), and that only the marker block is read from stdout. A firmware flooding the log must then reach a verdict, not exit 5.

**Stop here.** FEEDBACK ends with:
- the spike's table, with each prediction beside its result;
- a recommendation for each of P1 to P5;
- the decisions you need.

Then write the stop line.

## 2. Build, after the ruling

As ruled. Expected, and to be confirmed by the ruling:

### 2.1 In-run lock-step in the bridge
- **The bridge owns Renode as its child,** stepped by the `ioc` route at the ruled quantum. Static campaigns keep their pre-pass: their records and verdicts are byte-identical.
- **A dynamic scenario takes a new override kind** for the droop (P2), and storms if ruled (P3). `SCENARIO_VERSION` rises only if the static scenarios still parse unchanged.

### 2.2 Records, labelled per `RUL-17` and `RUL-19`
- **Separate records, each naming its source:**
  - the rail's computed waveform, or its crossings, from ngspice;
  - the reset or PVD event the bridge applied, labelled as the model's rendering;
  - the firmware's observed effect, from Renode.
- **The trace version rises** by exactly what this adds.
- **The analog deck's fixed 400 kHz window is labelled** in its record (review A10.4).

### 2.3 Run rules
- **Expectations by count as well as kind** (`AET-WO-2300` F11): an optional `expect_counts`, because one dynamic scenario can fault many times.
- **The verdict on a droop scenario:**
  - a droop inside the domain's tolerance must pass with no reset;
  - one past BOR must show a reset **and** the firmware's recovery (boot word re-observed, I2C read resumed) within a bound you measure and state.

### 2.4 Bounds and hygiene
- **`MAX_RECORDS` and replay** are as ruled from P4. `OQ-03` is answered in FEEDBACK with the measurement.
- **Renode's log goes to a file;** stdout carries only the marker block (P5). A log-flood test that exited 5 now reaches a verdict.
- **Renode's monitor history goes to `/dev/null`** (`AET-WO-2200` L6), with a test that 100,000 commands do not grow memory.
- **The bridge identity handshake** (`AET-WO-2200` F15): the CLI refuses a bridge whose version or build hash it does not expect.

### 2.5 The reference campaign
- **Add two scenarios** to `scenarios/sentinel-01/`, with predictions written first:
  - a tolerated droop, expected to pass;
  - a BOR-crossing droop, expected to show a reset and a recovery.
- **The archive replays,** and two runs are byte-identical.
- **Record the wall time per scenario** in FEEDBACK. Each tier-2 test you add must weigh under 10 minutes on CI, so that sharding holds (`ci/test-weights.tsv`; `TIER2_MIN` rises by exactly the tests added).

## 3. Invariants and fences

- `INV-02`, `INV-04`, `INV-05`, `INV-06`, `INV-07`; `RUL-05`, `RUL-12`, `RUL-13`, `RUL-17`, `RUL-19`. `unsafe` only in `bridge/src/ffi.rs`, as `RUL-17` allows.
- **Fence:**
  - `bridge/`;
  - `crates/aethelgard-chaos`, `aethelgard-twin` and the CLI's `analog`, `campaign`, `renode`, `run` and `verdict` modules;
  - `scenarios/sentinel-01/`;
  - tests;
  - `ci.yml` (`TIER2_MIN`) and `ci/test-weights.tsv`;
  - this work order's FEEDBACK.
- **Not in the fence:** the I2C witness, kind 13, attribution rules and the generated firmware. Those are `AET-WO-2204` and later.
- **Every static scenario's records, verdict and replay are byte-identical to `main`'s,** except where the trace version itself is recorded.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 4. Acceptance

- **The spike's table and recommendations** are in FEEDBACK, ruled before any build commit.
- **Both new scenarios behave as predicted,** in the pinned image, twice, byte-identical. The archive replays.
- **`OQ-03`'s dynamic case is measured,** and answered or brought to Scotty.
- **A log flood reaches a verdict.**
- **A stale or impostor bridge is refused.**
- **The gates** (`CLAUDE.md`) pass. CI is green with every shard under 15 minutes.

## 5. Adversarial pass — full

1. **Make a droop scenario pass** while the firmware never recovered, or while the rail never crossed what the scenario declares.
2. **Make a static scenario's verdict or records change.**
3. **Make the archive's dynamic records disagree** with what ngspice computed or what Renode observed: a record whose source is not the one named, or an effect recorded as measured.
4. **Exhaust memory or time** through a dynamic scenario that passes validation.

## 7. Flag-and-stop conditions

- Renode 1.17.0 offers no means to render a reset or a held reset from the monitor.
- The ruled quantum makes one reference droop scenario take more than about 10 minutes of CI.
- A static scenario's records or verdict must change.
- `INV-04` would need sampling. That is Scotty's ratification.
