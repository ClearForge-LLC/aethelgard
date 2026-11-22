# AET-WO-4300h — Spike: what a connector or test point can put on an MCU pin. FT or TT pins for off-board signals, and a stated off-board drive bound

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4300h-offboard-pins`.
**Author:** Claude (architect) · **Builder:** strong tier, with a targeted adversarial pass.
**Phase:** P4, before any Sentinel-03 fabrication. Written 2026-11-22. **Cadence:** a design spike that stops for the ruling. No crate code.

**Grounds** (`1216` F1, F5 and D2, roadmap §7.1):
- **Sentinel-03's MOTOR1 is on PE9, a TT_ha pin, at J5.** It is the one connector-facing TT pin on any committed board. With VDD up, DS12117 Rev 10 Table 13 holds a TT pin to VDD + 0.3 V, so a 5 V-logic peer on J5 would exceed it even when the MCU is powered.
- **Test points land on PA5 (TT_ha) and PA7 (TT_a)** on three Sentinel-01 boards.
- **No off-board end has a bound.** DS-009 prints a nominal +3.3V for its TELEM, GPS and I2C signal pins, with no maximum, and says nothing on driving a line while the autopilot is unpowered. `1216` ruled an unbounded path named, never passed.
- Read `1216` FEEDBACK (§1 walk 1 and walk 2, F1, F5), `4300f` (connectors as declared parts, DS-009 pinouts) and `1211e` (Table 8's pin structures in `power-parts.json` `io`) first.

> **What this is:**
> - **A measurement** on every committed board: every MCU pin that reaches a connector pin or a test point, with its Table 8 structure (FT, FT_a, TT, TT_a …), its signal, and what is known about the far end.
> - **For each TT pin found:** whether the solver could have put that signal on an FT pin instead (the alternate-function rows the solver already holds), and what moving it would change. For Sentinel-03's MOTOR1: which other timer channel on an FT pin serves it, and what that moves in the solved topology.
> - **Options, each with its cost and its measured effect on the committed boards.** At least:
>   - (a) the solver prefers an FT pin for any connector- or test-point-facing signal, and refuses a TT pin there unless the specification states why;
>   - (b) a declared off-board drive bound per connector pin (`connectors[].pins[].offboard.drive_max_millivolts` with a source), judged by A1 with VDD up and by `1216b`'s rule at VDD = 0;
>   - (c) both.
> - **A lean, with its reason, and a stop.**
>
> It is NOT:
> - any crate, specification, `fab/` or workflow change;
> - a re-pinning of Sentinel-03. That is the build the ruling orders.

## 1. Measure

1. **Predictions first,** in `spike/1216`'s style, committed in `spike/4300h/` before any run.
2. **The pins,** by a scratch reader of `solve` and `emit` output (as `1216` D6 did, no `Cargo.toml` under `spike/`), cross-checked on the drawn netlist.
3. **The alternatives** for each TT pin found, from the solver's own alternate-function data.
4. **Each option's refusals** on the committed boards, run, not estimated.
5. **The options and the lean** in FEEDBACK.

## 2. Acceptance

- **Every connector- and test-point-facing MCU pin on every committed board is listed** with its structure and source.
- **Each option names what it refuses or moves,** measured.
- **No crate file changes.** The gates are not run on a spike-only branch; say so.

## 3. Fences

- **Fence:** `spike/4300h/` and FEEDBACK.
- **Not:** any crate, specification, `fab/`, workflow or `ci/` file.
- **Running beside you:** `1216b` (IR), `2308` (`verify`), `1320` (eda). None conflicts with a spike-only branch.
- One PR, spike-only, left unmerged. Write `~/.cache/aet-4300h/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A connector- or test-point-facing pin the reader misses,** including through a series resistor or a level translator.
- **A pin structure read from anything but Table 8.**
- **An alternative pin the solver could not actually take** (an AF conflict, a reserved pin, a pin another signal holds).

## 7. Flag-and-stop conditions

- **Stop after §1.5 with the options and the lean.**
- **A committed board drives a pin beyond its stated rating with the MCU powered, by a path no option judges.** Stop at once and report it.
