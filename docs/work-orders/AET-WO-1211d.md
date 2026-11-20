# AET-WO-1211d — Pin devices drawn: the DRV8876 and DRV8833 arms, their symbols and footprints, straps and passives from the rows

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1211d-pin-devices-eda`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4, before the baseline. Written 2026-11-20. **Cadence:** a measurement, then a build with stops. **After `1211a` merges.** Beside `1211b`.

**Grounds:**
- `AET-WO-1211` FEEDBACK §5, item 4, as ruled. Q6: pins, kinds, levels, pulls and truth tables are row data. Symbols, footprints and passives are this work order's, with `4600` D4's question.
- `1210`'s rule refuses a part with no emitter arm on a specification that draws a board. Until this work order, a pin device can only be declared on one that draws none.

> **What this is:**
> - **An emitter arm per part,** DRV8876PWPR and DRV8833PWPR:
>   - the symbol and footprint (vendored from KiCad 10.0.7 if it has them, by blob sha; otherwise authored under `aethelgard-kicad-lib` with every pad cited to the datasheet's package drawing);
>   - the straps and passives from the row (IPROPI's resistor, nFAULT's pull-up, the VM bulk and bypass capacitors the datasheet requires, each cited);
>   - the bridge outputs to a declared connector.
> - `PinDeviceNotDrawn` is lifted for these two parts.
> - **`4600` D4, answered by measurement:** say whether the per-part arm can be row data (a pin map, straps and passives in the row) without moving any existing part's bytes. **Propose it; do not build it** unless it is free.
>
> It is NOT the generator (`1211b`) or the twin (`1211c`).

## 1. Build

1. **Measure first:** `emit` every committed specification. **Prediction:** no byte moves. **Stop and report** if one does.
2. **Symbols and footprints:**
   - vendor them where KiCad 10.0.7 has them, proved byte for byte against the upstream blob;
   - otherwise author them, each pad cited.
3. **The arms, the straps and the passives.** Every value comes from the row or is cited. A passive the datasheet requires but whose value it does not state is refused by name, never invented.
4. **D1 and D2 drawn:** emit, ERC, DRC, `extract` and `check support` for each, as a drawn board (`board.drawn` absent). D2 stays refused by `PartLimitUndeclared` at `validate` until its row is complete; draw it at library level.
5. **Mutants:**
   - a strap left floating;
   - nFAULT's pull-up missing;
   - a bridge output to no connector.

   A test fails for each.

## 2. Acceptance

- **No existing specification's KiCad byte moves.**
- **D1 is drawn** and passes ERC, DRC, `extract` and `check support`.
- **The CLAUDE.md gates pass, and CI is green.**

## 3. Fences

- **Fence:**
  - `crates/aethelgard-eda`;
  - `crates/aethelgard-kicad-lib` (vendored or authored files, `PINS` and `FILES`, the README row);
  - the two parts rows' symbol and footprint fields;
  - tests and fixtures;
  - `spike/1211d/`;
  - FEEDBACK.
  - **Not:** the IR schema, the solver, the generator, the twin, workflows, or `ci/` (except weight rows).
- **Running beside you:** `4240c` (eda: the crystal; expect conflicts in the emitter and `kicad-lib`), `1211b` (generator), `4240b`, `1213`, `1913`. Merge `main` often; before READY, without fail.
- **KiCad runs in the engine image only,** one container of yours at a time.
- One PR, left unmerged. Write `~/.cache/aet-1211d/pr-title-body.md`.

## 5. Adversarial pass, full

- **A drawn driver that passes every gate but would not run:** nSLEEP floating at power-up, a missing VM capacitor, or IPROPI's resistor on the wrong net.
- **An authored footprint** whose pad differs from the datasheet's drawing.

## 7. Flag-and-stop conditions

- **An existing specification's KiCad byte moves.** Stop with the diff.
- **A footprint or passive needs a figure no held datasheet states.** Stop and name it.
