# AET-WO-1203 — Static I2C electrical rules in `solve`: the pull-up floor and the bus-capacitance limit (UM10204)

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1203-electrical-rules`.
**Author:** Claude (architect) · **Builder:** a cheap builder with a targeted adversarial pass (`docs/roadmap.md` §6).
**Phase:** P1 refinement, from the Mythos 5.2 review of 2026-11-12 (A2, B2; `docs/reviews/`).

**Grounds:**
- **The review's A2, reproduced:** 24 Ω, 50 Ω and 100 Ω pull-ups solve and pass the campaign. `aethelgard-ir` refuses only a pull-up of 0 (`crates/aethelgard-ir/src/lib.rs`, `ZeroPullUp`). At 3.3 V, 24 Ω asks the driver to sink 137 mA against UM10204's 3 mA.
- **`AET-WO-1202`** already maps each bus's declared rise time onto a UM10204 mode and refuses with named errors. These rules sit beside it.
- **`AET-WO-3200`** (the proposal loop) must take its floor from the gate, so the floor and the gate are one fact, not two (`RUL-18`, `INV-07`).

> **What this is:** two static rules, checked in `solve`, each a named refusal, each exposed as one public function.
>
> It is NOT:
> - extraction from the board;
> - a change to the analog netlist, the bridge, or any emitted byte for a specification that passes both rules.

**Cadence:** build-only. Stop only on §7. One PR, left unmerged.

## 1. Scope

1. **A new module, `aethelgard_ir::electrical`,** citing UM10204 rev. 7 for each constant (§7.1 and table 10; the builder confirms the numbers against the document). It provides:
   - `pub fn i2c_mode(speed_hz: u32) -> I2cMode`: `Standard` up to 100 kHz, `Fast` up to 400 kHz, `FastPlus` up to 1 MHz. Higher speeds are already refused (`UnsupportedI2cSpeed`).
   - `pub fn i2c_pull_up_floor_ohms(speed_hz: u32, domain_millivolts: u32) -> u32`:
     - the formula is `ceil((VDD − VOL) / IOL)`;
     - `VOL` is 400 mV, or `0.2 × VDD` when VDD is at most 2 V;
     - `IOL` is 3 mA for Standard and Fast, and 20 mA for Fast-mode Plus;
     - integer arithmetic only.
   - `pub fn i2c_bus_capacitance_limit_femtofarads(speed_hz: u32) -> u64`: 400 pF for Standard and Fast, 550 pF for Fast-mode Plus.
2. **`solve` refuses, after its existing I2C checks:**
   - `PullUpBelowSinkFloor { bus, pull_up_ohms, floor_ohms }` when a bus's pull-up is below its floor, at the bus's rail (`domain_millivolts`, as `analog_lines` carries it);
   - `BusCapacitanceOverLimit { net, capacitance_femtofarads, limit_femtofarads }` when an I2C line's `max_capacitance_femtofarads` exceeds its bus's limit.

   Both are `INV-06` refusals: a JSON error and a nonzero exit, with no default substituted.
3. **The seeded set stays at its count.**
   - `scenarios/sentinel-01/seeded/cap-470p-sda-2k2.json` exceeds 400 pF and would now be refused. Replace it with a 390 pF case, which still fails its ceiling (about 727 ns predicted).
   - Update `cases.json` and every test that names the old case.
   - Record the old and new measured rises in FEEDBACK.
   - Every other seeded case must still pass `solve`; check each.
4. **`AET-WO-2203` F3:** `DeviceNotPlaced` is unreachable. Remove it if no input reaches it and the tests agree. Otherwise say why it stays.

## 2. Invariants and fences

- `INV-06`, `INV-07`, `RUL-16`. No `unsafe`.
- **Fence:**
  - `aethelgard-ir`: `electrical` and `solve`;
  - the CLI's mapping of the two new errors;
  - `scenarios/sentinel-01/seeded/`;
  - tests;
  - this work order's FEEDBACK.
- **Every committed generated file stays byte-identical.** The regeneration guard must pass with no change. Any difference is a §7 stop.
- **`AET-WO-3200` may run in parallel** and will call these two functions by the names above. Do not rename them. If it lands first, merge `main`.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance

- **The floor:** at 3.3 V and 400 kHz, 966 Ω is refused and 967 Ω solves. The reference specification (2.2 kΩ, 100 pF) solves unchanged.
- **The review's inputs:** a 24 Ω, a 50 Ω and a 100 Ω pull-up are each refused with `PullUpBelowSinkFloor`, through `solve` and through a campaign scenario's override.
- **The capacitance limit:** 400 pF solves at 400 kHz and 401 pF is refused; 550 pF solves at 1 MHz.
- **A unit table of the three modes,** at 1.8 V and 3.3 V.
- **The seeded set:** all 20 failing cases still fail as their manifest says, and the replaced case is attributed `electrical E1`.
- §1.4 passes in `gates` and `tools`.

## 5. Adversarial pass — targeted

1. **Find a specification that passes `solve` with a pull-up below the floor,** through rounding, a rail of 0, a rail at the 2 V boundary, or an overflow.
2. **Find a capacitance over the limit that solves.**
3. **Find a change to any emitted byte for a specification that passes.**

## 7. Flag-and-stop conditions

- UM10204's numbers differ from §1.1. Report the document's values; do not guess.
- Any emitted byte changes.
- A seeded case cannot keep its domain without weakening a test.
