# AET-WO-2300 — The fault campaign: independent scenarios, archived runs, replay from the archive (closes P2)

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-2300-campaign`.
**Author:** Claude (architect) · **Builder:** a strong builder, with a **full** adversarial pass (`docs/roadmap.md` §6).
**Phase:** P2. It closes the exit gate's three remaining clauses:
- **Clause 2's campaign form:** "With the specification's I2C pull-up changed to 47 kΩ, the campaign reports a bus failure whose trace shows the rise time exceeding the solved ceiling; with 2.2 kΩ, the same campaign passes."
- **Clause 3:** "Every failing campaign's trace replays from the archive to the same state sequence hash, twice (`INV-04`)."
- **Clause 4:** "Trace bytes per campaign are recorded… and `OQ-03` is answered or escalated."

**Grounds:**
- **`AET-WO-2200` and `AET-WO-2202`, merged.** `simulate --firmware` already produces three separate record kinds: `analog` (the measured rise), `faults` (the bridge's decision) and `boot` (the firmware's observation, including the I2C witness).
- **`AET-WO-0400`'s archive (`aethelgard-chaos`)** is deterministic CBOR, append-only, with a strict replay. Its profile **refuses floats**, so every physical value is an integer in a stated unit: rise times in picoseconds, resistance in ohms, capacitance in femtofarads.
- **`INV-07`:** scenarios are authored independently of the generated artifacts.
- **`RUL-17` L7 (PR #66)** names literal `INV-04`, replay from the archive without the solvers, as this work order's to show.

> **What this is:**
> - a scenario file format, with two checked-in reference scenarios;
> - `aethelgard campaign`, which runs each scenario through the existing `simulate --firmware` path and archives every record;
> - `aethelgard replay`, which recomputes a run's state-sequence hash from the archive alone.
>
> It is NOT:
> - dynamic faults: rail droop, interrupt storm, in-run lock-step (`AET-WO-2301`, later);
> - attribution (P3);
> - a change to the bridge's measurement.

**Cadence:** build-only. Stop only on §7. One PR, left unmerged.

## 1. Scope

1. **Scenarios.**
   - **The format.** A strict JSON file: `{schema_version, name, base: <path to a specification>, overrides: [...]}`. An override is closed-world:
     - `{"bus": <id>, "pull_up_ohms": <u32>}`;
     - `{"net": <name>, "max_capacitance_femtofarads": <u64>}`;
     - `{"bus": <id>, "line_held_low": "sda"|"scl"}`.

     Unknown keys, unknown ids, and an override that leaves the specification invalid are refused (`INV-06`). The runner applies the overrides to the specification and solves it. It never edits generated files.
   - **`line_held_low`** is a static fault the bridge decides on without measuring: the bus is faulted, and the record names the scenario as its source.
   - **Two reference scenarios** are checked in under `scenarios/sentinel-01/`: `pullup-2k2.json` and `pullup-47k.json`.
2. **`aethelgard campaign --scenarios <file>... --firmware <elf> --archive <dir>`.**
   - For each scenario, in the order given, it runs exactly the `simulate --firmware` path, under the same sandbox and `RUL-13` bounds.
   - **Each run is one archive run directory.** It appends, as integer-only events:
     - the scenario's sha256 and its overrides;
     - the solved topology's sha256;
     - every `analog.rise_time_measured`, with `rise_30_70_ps` and `complete`;
     - every `bridge.fault_state`;
     - the boot's `firmware.observed` records, the I2C checks included;
     - the verdict.

     Then it commits the run.
   - **Output:** one JSON line per scenario (name, verdict, run id, state-sequence hash, `trace_bytes`), then a campaign summary. The exit code is 0 only if every scenario's verdict matches its declared `expect` field (`pass` or `fail`). A campaign therefore asserts what should fail.
3. **`aethelgard replay --archive <run dir>`.**
   - It reads the run through `aethelgard-chaos`'s strict reader, with no solver, no Renode and no bridge.
   - It recomputes the state-sequence hash, defined in this work order as sha256 over the canonical CBOR of the ordered event list, excluding wall-clock fields.
   - **It refuses** a run whose recomputed hash differs from the one committed in its manifest.
   - Two `replay` calls on one run give byte-identical output.
4. **`OQ-03`.** FEEDBACK records `trace_bytes` per scenario and per campaign, and bytes per simulated second, measured on the two reference scenarios. The architect amends `architecture.md` §2 from those numbers.

## 2. Invariants and fences

- `INV-04`, `INV-06`, `INV-07`, `RUL-06`, `RUL-13`, `RUL-17`. No `unsafe`.
- **Fence:**
  - `aethelgard-chaos`: the scenario model, the campaign runner and the event kinds;
  - the CLI: `campaign` and `replay`, their MCP exposure (generated from the tree, as always) and the README;
  - `scenarios/` (new);
  - tests;
  - this work order's FEEDBACK.
- `simulate --firmware`'s own behaviour does not change. The campaign calls it, or the same internals, unchanged.
- **The W2 follow-up (`AET-WO-2203`) may run in parallel.** It moves the I2C devices into `SolvedTopology`. If it lands first, merge `main`.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance: P2 gate clauses 2–4, as tier-2 tests

1. **The campaign** over `pullup-2k2.json` (`expect: pass`) and `pullup-47k.json` (`expect: fail`) exits 0. The 47 kΩ run's archived events show `rise_30_70_ps` = 3,982,300 ± 500 against a ceiling of 300,000, the bus fault, and U2's `i2c_timeout` at TXIS, as three separate events.
2. **`replay`** on the 47 kΩ run, twice: identical output, and the hash equals the committed one. A one-byte change to the archived trace is refused.
3. **The same campaign run twice** gives identical state-sequence hashes per scenario.
4. **FEEDBACK** holds the trace-byte numbers for `OQ-03`.

§1.4 passes in `gates` and `tools`.

## 5. Adversarial pass — full

1. **Make a failing scenario archive as passing,** or a run replay to a hash it did not produce: reorder events, drop the analog event, or carry a float.
2. **Make a scenario reach outside the specification:** an override on a generated file, an unknown key, or a path escape in `base`.
3. **Make `replay` touch a solver, the network or anything outside the run directory.**
4. **Make `expect` hide a regression:** a scenario that "expects fail" but fails for the wrong reason. Each scenario names the finding kind it expects, and a mismatch is a campaign failure.

## 7. Flag-and-stop conditions

- The archive's profile cannot carry a record without a float or a map. Report it; do not widen the profile.
- A change to `simulate --firmware`'s behaviour seems needed.
- A test that can only go green by being weakened.
