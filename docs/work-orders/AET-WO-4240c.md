# AET-WO-4240c — The crystal drawn: the HSE support group, the vendored symbol and footprint, its load capacitors picked against a bounded `Cs`, and oscillator routing judged

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4240c-hse-eda`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4 (Sentinel-02's clock). Written 2026-11-20. **Cadence:** a measurement, then a build with stops. **Independent of `4240b`** (generator and twin): this is eda only.

**Grounds** (`AET-WO-4240` FEEDBACK §1 (sources), §8 (`aethelgard-eda` row), §9, D10 item 3; `4240a` FEEDBACK D-3, D-4, A3; roadmap §5 and §7.1):
- **`4240a` declared the crystal and its load capacitors by value** (`mcu.hse.crystal`, `load_caps_femtofarads: [CL1, CL2]`, D-3 option (a)).
  - It judges them as far as it can without the stray capacitance `Cs`: `CL1` = `CL2`, and the series value at most the row's CL (`HseLoadCapsUnfit`).
  - The part for the capacitors is this work order's.
  - So is `Cs`, which `extract` bounds: the copper of the two oscillator nets plus the OSC pins' capacitance.
- **Nothing in eda draws a crystal.**
  - `support.rs` groups only VCAP, NRST and BOOT0, from `power-parts.json`'s `Support`.
  - The MCU symbol has `RCC_OSC_IN`/`RCC_OSC_OUT` (`STM32H753ZITx.kicad_sym` L617–650).
  - **Not vendored:** KiCad 10.0.7's `Device:Crystal_GND24` (blob `97e45475…`) and `Crystal:Crystal_SMD_3225-4Pin_3.2x2.5mm` (blob `11874e16…`). `4240a` D-4 names both, and they exist upstream at tag `10.0.7`.
- **The crystal's row** is `ABM8AIG-16.000MHZ-12-2Z-T3` in `power-parts.json` (schema 5, kind `crystal`). It has CL 12 pF, C0 ≤ 2.0 pF and ESR ≤ 70 Ω, every figure cited.
- **`1210`'s rule** (once it merges): `validate` refuses a part with no emitter arm on a specification that draws a board. Until this work order, a crystal specification can only be one that draws no board.

> **What this is:**
> - **The vendored symbol and footprint,** by the vendoring path `1315` used (`PINS`, `FILES`, the README row), byte for byte from KiCad 10.0.7, by blob sha.
> - **An HSE support group in the emitter.** For a crystal: the crystal on PH0/PH1 (`RCC_OSC_IN`/`OUT`), its two GND pins grounded, and two C0G capacitors to GND. For an external clock: a net to PH0 from the declared source, and nothing on PH1.
> - **A C0G 0402 capacitor row** (or a family the emitter picks by value), every figure cited to a held maker's document. It is picked against the declared `load_caps_femtofarads`.
> - **`Cs` bounded by `extract`:** the two oscillator nets' copper capacitance plus the OSC pins' capacitance (DS12117, cited). The load the crystal sees is then judged, `series(CL1, CL2) + Cs` against the row's CL, by a rule this work order measures and proposes (§1.4).
> - **Oscillator routing:** the crystal placed beside PH0/PH1, no via on either oscillator net, and both nets inside a bound `extract` judges. Every number is cited or declared, never invented.
>
> It is NOT:
> - the firmware or the twin (`4240b`);
> - a change to a specification that declares no HSE: its KiCad bytes must not move;
> - Sentinel-02's drawn board (`4500`);
> - the bypass bench.

## 1. Build

1. **Measure first,** in `spike/4240c/`, with predictions committed before the runs.
   - `emit` every committed specification on `main`, and record every KiCad file's sha256. **Prediction:** after this work order, no `hse: null` file moves. **Stop and report** if any does.
   - Emit `4240a`'s 16 MHz crystal fixture and 8 MHz external fixture as drawn boards (each declaring that it draws one, under `1210`'s rule if it has merged).
2. **Vendor** the two KiCad files. Prove each byte against its upstream blob sha at `10.0.7`.
3. **The support group and the emitter arm** for `crystal`, and the external-clock net.
   - Pin numbering, the GND pins and the capacitors' placement follow the footprint and the row. AN2867 is uncited (only Rev 10 is held, from a crystal maker's copy), so any guidance taken from it is labelled uncited and judges nothing.
4. **`Cs` and the load judgement.**
   - **Measure:** `extract`'s bound on the two oscillator nets' copper capacitance for the fixture as routed, plus DS12117's OSC pin capacitance, cited. Report `Cs` minimum and maximum.
   - **Propose the judgement** and say which: (a) `series(CL1, CL2) + Cs` ≤ CL + a declared tolerance, with the tolerance declared and cited; or (b) report only, if no held source bounds the frequency effect of a load mismatch (the row gives no pullability).
   - **Stop and ask** before building a judgement that needs a figure no held document states.
5. **Pick the C0G part.** `4240` §9's family is C0G/NP0 0402 at ±0.25 pF or ±1 %. From `Cs`, state the value that `2(CL − Cs)` gives, and say whether the fixture's declared value fits. **The declaration is not changed silently:** a mismatch is a finding, reported with the value that would fit.
6. **Routing rules:** no via on either oscillator net, and the crystal placed within a declared distance of PH0/PH1. Each is a named refusal (`OscillatorNetVia`, `CrystalTooFar`), and each bound is declared or cited.
7. **Tests:**
   - each fixture emits, routes and passes `extract`, or is refused by name;
   - every `null` board's KiCad bytes are unchanged.
   - **Mutants:** a via on `RCC_OSC_IN`, the crystal moved away, `Cs` dropped from the load judgement, and a capacitor of the wrong dielectric. A test fails for each.

## 2. Acceptance

- **The measurement** is in FEEDBACK. No `null` board's KiCad byte moves.
- **The crystal fixture is drawn and routed,** with `Cs` reported and the load judged (or reported, per §1.4's ruling).
- **The CLAUDE.md gates pass, and CI is green.** `TIER2_MIN` rises by your own tier-2 count, if any.

## 3. Invariants and fences

- **Invariants:** `INV-01`, `INV-06`, `INV-07`, `RUL-14` (declared, not invented), `RUL-19`.
- **Fence:**
  - `crates/aethelgard-eda` (support group, emitter arm, routing rules, `extract`'s `Cs`);
  - `crates/aethelgard-kicad-lib` (the two vendored files, their `PINS` and `FILES` entries, the README row);
  - `crates/aethelgard-ir/data/power-parts.json` (the C0G row);
  - `aethelgard-ir`'s parts reader, only if the C0G row needs a field it lacks;
  - tests and fixtures;
  - `spike/4240c/`;
  - FEEDBACK.
  - **Not:** the IR schema, the solver, the generator, the twin, `boards/`, `fab/`, workflows, or `ci/` (except a weight row the P1/P2 guard asks for).
- **Running beside you:**
  - `4240b` (generator and twin: no eda);
  - `1211a` (IR and named refusals: its `PinDeviceNotDrawn` lands in eda; merge it in as it comes);
  - `1210` (#280: the draws-a-board rule and the emitter-arm check);
  - `1912` (CI weights).

  Merge `main` before READY.
- **One container of yours at a time,** under `capped-run`, at `nice -n 10`. KiCad runs in the engine image only.
- One PR, left unmerged. Write `~/.cache/aet-4240c/pr-title-body.md`.

## 5. Adversarial pass, full

- **A crystal board that passes every eda gate but cannot start:** `Cs` underestimated, a capacitor on the wrong net, or the crystal's GND pins left floating.
- **A routed oscillator net that passes the via rule** but runs under a noisy net, or beside one.
- **A `null` board whose KiCad bytes moved.**
- **A vendored file that differs from upstream** by a byte.

## 7. Flag-and-stop conditions

- **A `null` board's KiCad byte moves.** Stop with the diff.
- **The load judgement needs a figure no held source states.** Stop and ask (§1.4).
- **The fixture cannot be routed** inside the declared crystal distance with no via. Stop with the placement.
