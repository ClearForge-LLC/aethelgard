# AET-WO-4300i — Off-board pins, the build: an external motor pull-down, FT at connectors, Sentinel-03's MOTOR1 on PA8, and declared pin reservations

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4300i-offboard-build`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4, before any Sentinel-03 fabrication. Written 2026-11-23. **Cadence:** a measurement, then a build with one stop.

**Grounds:** `AET-WO-4300h` (#322) and its ruling (Scotty, 2026-11-23). Read `4300h` FEEDBACK §3–§4 and §7 first, then PR #161's Q-G.

**The ruling:**
- **D0 `external`.** Every motor line carries a declared external pull-down and no internal pull. This reverses Q-G for motor outputs only.
- **D1 (a) at connectors.** The solver prefers an FT pin for a signal that reaches a connector pin. A hand TT pin there is refused. **D5:** no escape field.
- **D2.** Test points are named, not moved.
- **D3.** MOTOR1 moves PE9 → PA8. `3400`'s E5 clock-out takes MCO2 on PC9.
- **D4.** (b) is deferred.
- **F7.** A declared reservation (`mcu.reserved_pins`, each with a reason) that the solver honours.

> **What this is:**
> - **(1) The motor pull as a declared part.**
>   - A field on `motor_outputs` declaring the pull-down: its part row, value and source.
>   - A1 judges it as a declared pull.
>   - Both generators stop writing the motor `PUPDR` (`svd-hal`) and `PULLDOWN` (`hwdef`).
>   - `implied_pupd` and `check pinout` follow.
>   - eda draws the resistor on every motor net. `4300h` counted 35 motor nets on 9 boards.
>   - The value is not chosen by hand: cite a held source for it (an ESC input or AN4899's pull guidance, if held). If none is held, **stop** and name what is needed.
> - **(2) FT at connectors.** The solver prefers an FT pin for any signal with a connector-facing end. A hand TT pin at a connector is refused by name.
> - **(3) Sentinel-03.** MOTOR1 is re-pinned to PA8, and `3400`'s E5 note moves to MCO2/PC9.
> - **(4) `mcu.reserved_pins`** with a reason per pin. The solver never offers a reserved pin. Sentinel-03 reserves PA11/PA12 (USB) and PC9 (MCO2). A hand pin on a reserved pin is refused.
> - **These are specification keys**, so they take the next specification version. Each refusal comes with its test, and each test with a mutant it kills.
>
> It is NOT:
> - (b), the off-board drive bound;
> - moving a test point;
> - TIM4;
> - any board other than Sentinel-03 re-pinned. Any other pin move is a stop (§7).

## 1. Build

1. **Measure first.** Commit predictions in `spike/4300i/` covering:
   - the validate/solve/emit baseline on every committed specification;
   - for (1), which boards and nets gain the resistor;
   - for (2) and (4), which pins move.
2. **Stop with the measurement and (1)'s schema shape:** the field, where it lives, its version, and the cited value. Then build to the ruling.
3. **Build (1)–(4),** with goldens moved by their generators.
4. **Run the null sweep.**

## 2. Acceptance

- **Every motor net** carries a declared, drawn, judged pull-down, and no generated firmware writes an internal motor pull.
- **Sentinel-03:** MOTOR1 is on PA8. PA11, PA12 and PC9 are reserved, and the solver honours the reservation.
- **A hand TT pin at a connector** is refused, with a test.
- **No committed board's pins move** beyond Sentinel-03's MOTOR1. Every other byte moves only as measured and ruled.
- **The CLAUDE.md gates pass, and CI is green.** If a `fab/` byte moves, queue for a package dispatch.

## 3. Fences

- **Fence:**
  - `crates/aethelgard-ir` (motor, solve, validate, versions, README, tests);
  - `crates/aethelgard-svd-hal`'s generator (motor `PUPDR`);
  - `crates/aethelgard-hwdef` (`PULLDOWN`);
  - `crates/aethelgard-pincheck` (`implied_pupd`);
  - `crates/aethelgard-eda` (the drawn pull);
  - `examples/sentinel-03*.json` and the version literals and goldens a bump moves, by their generators;
  - `spike/4300i/` and FEEDBACK.
- **Not:** the twin, chaos, `verify`; workflows or `ci/`.
- **Running beside you:** the backlog wave in roadmap §5, whose fences avoid yours. Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-4300i/pr-title-body.md`.

## 5. Adversarial pass, full

- **A motor line still pulled internally on any path:** ArduPilot hwdef, the bootloader, or `start_outputs`.
- **A connector-facing signal that still lands on TT** through a path the solver does not see: a hand pin, an `acknowledge_unwitnessed` entry (`4300h` F6), or a translator.
- **A reserved pin offered by any solver branch.**
- **A drawn resistor whose net, value or footprint disagrees with the declaration.**

## 7. Flag-and-stop conditions

- **After §1.2:** stop with the measurement and the schema shape.
- **No held source for the pull value.** Stop.
- **A committed board's pin moves other than Sentinel-03's MOTOR1.** Stop with the diff.
