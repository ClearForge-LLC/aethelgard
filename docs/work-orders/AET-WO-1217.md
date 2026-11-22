# AET-WO-1217 — Pin devices' IR backlog: a current output tied to GND, the analog-switch pads, the reservoir against VDDA's maximum, step 24's order, and an unknown MCU refused once

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1217-pin-device-ir-backlog`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4, before the P4 baseline. Written 2026-11-22. **Cadence:** a measurement, then a build with one stop. **After `1216b`** (#320), which held the IR.

**Grounds** (roadmap §7.1, the `1211e` rows; read `1211e` FEEDBACK §5 first):
1. **`1211e` F2, a GND tie for a `current_output` pin.** SLVSDS7B p. 13's honest way to turn the DRV8876's current regulation off is IPROPI on GND with VREF above GND. It cannot be declared: a strap on IPROPI is `SignalKindMismatch`, and `TripSenseUnbound` refuses an IPROPI no analog input terminates. Wanted: a strap kind for it, and A2 skipping the trip when it is declared.
2. **`1211e` F4, PC2_C and PC3_C.** They are analog-switch pads on the LQFP144: `I/O` in the open pin data, `ANA` in DS12117 Table 8, and limited to 1 mA sunk by Table 11 (p. 104). The solver offers them as GPIO.
3. **`1211e` F8, pre-existing from `1211a`.** The ADC reservoir's rating is compared with VDDA's **minimum** where its maximum belongs (`pin_devices.rs`, `analog()`). It is harmless with the 50 V parts held.
4. **`1211e` F9.** Step 24 runs `analog()` before `trip()`, so an ADC refusal hides A2's kind on the same device.
5. **`1211e` F10.** An unknown `mcu.part` is refused at `pin_devices[0].mcu`, a path that names no field. It should be refused once, at `mcu.part`, before step 24.

> **What this is:**
> - **(1)** a strap kind that ties a `current_output` pin to `GND`, admitted only with its reference above `GND`, and A2 skipping the trip when it is declared. **Measure first, and stop with the schema shape** (§7): it is a specification key, so it takes the next specification version.
> - **(2)** the analog-switch pads marked in the MCU row's `io` data from Table 8, and the solver refusing to place a digital signal on one. Measure first which committed boards it would move; any move is a stop.
> - **(3)–(5)** as their rows say, each with a test that fails without it.
>
> It is NOT:
> - a capacitance floor for `bulk` (`1211e` F6, standing until a held document states one);
> - a control wire's net judged against its other drivers (`1211e` F7, a power-tree work order);
> - any change to an emitted byte of a committed board, beyond the version lines the bump moves.

## 1. Build

1. **Measure first,** with predictions committed in `spike/1217/`:
   - every committed specification's `validate`, `solve` and `emit` output on `main`, as the baseline;
   - for (2), which committed boards place a signal on PC2_C or PC3_C;
   - for (4) and (5), each refusal kind today on a board that triggers it.
2. **Stop with (1)'s schema shape** (the strap kind, where it lives, and its version) and (2)'s measurement. Then build to the ruling.
3. **(3)–(5),** each with its test and a mutant it kills.

## 2. Acceptance

- **(1):** a DRV8876 with IPROPI on GND and VREF above GND validates; IPROPI on GND with VREF on GND is still refused.
- **(2):** no digital signal is placed on PC2_C or PC3_C, and the committed boards move only as measured and ruled.
- **(3)–(5)** closed, each by a test that fails without it.
- **Every other committed specification is byte-identical** except the version lines. Show it by a null sweep.
- **The CLAUDE.md gates pass, and CI is green.** If a `fab/` byte moves, queue for a dispatch.

## 3. Fences

- **Fence:**
  - `crates/aethelgard-ir` (pin devices, the MCU row's `io`, the solver's pin offer, validate's order, versions, README, tests);
  - the version literals and goldens a bump moves, by their generators;
  - `spike/1217/` and FEEDBACK.
- **Not:** the generator, the twin, chaos, eda, `verify`; workflows.
- **Running beside you:** `2308` (`verify`; in the package queue) and `4300h` (a spike, stopped for Scotty's ruling on Sentinel-03's motor pins). Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-1217/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A GND tie admitted where the trip still matters:** VREF on GND, or a second current output on the same device.
- **A digital signal reaching an analog-switch pad by another path:** a hand pin, a connector pin, a test point.
- **A refusal kind that changes for a board that already refuses.**

## 7. Flag-and-stop conditions

- **After §1.1:** stop with (1)'s schema shape and (2)'s measurement.
- **(2) would move a committed board's pinout.** Stop and report which, and what moves.
- **Any other committed byte moves beyond the version lines.** Stop.
