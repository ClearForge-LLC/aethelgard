# AET-WO-1214 — `validate` refuses a serial baud ArduPilot would read as kbaud, and the generator's console guard is kept and tested

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1214-serial-kbaud`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4. Written 2026-11-20. **Cadence:** a measurement, then a small build.

**Grounds** (`1213` F3 and F4, #297; roadmap §7.1):
- **F4.** For SERIAL1 to 9, ArduPilot stores the define as it stands, and `map_baudrate(b)` differs from `b` only when `b ≤ 2000`. Such a `SERIALn_BAUD` is read as kbaud.
  - Today a non-console, non-RC port at 2000 baud or less is refused only by the generator (`SerialBaud`, `hwdef/src/lib.rs`, about L640). `validate` passes it.
- **F3.** `hwdef`'s own `ConsoleBaud` refusal can no longer be reached from the CLI: `validate` (13c, `ConsoleBaudNotRoundTrip`) and `SolvedTopology::validate` refuse first.
- Read `1213`'s FEEDBACK §7 and its `aethelgard_ir::serial::map_baudrate`, which is already cited to `Copter-4.7.2`.

> **What this is:**
> - **F4:** a 13c sibling rule in `validate`, also re-checked in `SolvedTopology::validate`, as every 13c rule is. It refuses by name a port whose role ArduPilot opens from `SERIALn_BAUD` for n ≥ 1 when `map_baudrate(b) != b`.
>   - Name in FEEDBACK which roles it covers and why. RC is excluded if ArduPilot sets its rate by protocol: cite where.
>   - Use the IR's existing `map_baudrate`; no second copy.
> - **F3:** the generator's `ConsoleBaud` guard **stays**, as defence in depth for a topology that reaches the generator by another path (`RUL-20`'s pair). Give it a direct library-level test, so it is neither dead nor untested.
>
> It is NOT a change to any committed specification's bytes, to `pincheck`, or to the generator's behaviour.

## 1. Build

1. **Measure first,** with predictions committed before the runs, in `spike/1214/`:
   - every committed specification and solved topology: each non-console serial port's role and baud, and whether `map_baudrate(b) == b`.
   - **Prediction:** none is refused.
2. **The rule**, its kind named (for example `SerialBaudReadAsKbaud`), and the IR README row.
3. **Tests:**
   - 2000 and 1200 refused on a `mavlink` port;
   - 2001 and 9600 pass;
   - a solved topology moved to 1200 is a mismatch;
   - the generator's `ConsoleBaud` refusal fires when called on such a topology directly.
   - **Mutants:** drop the rule; flip `!=` to `==`. A test kills each.

## 2. Acceptance

- **The measurement** is in FEEDBACK. No committed specification is refused.
- **The CLAUDE.md gates pass, and CI is green.** `TIER2_MIN` is unchanged.

## 3. Fences

- **Fence:**
  - `crates/aethelgard-ir` (13c and `SolvedTopology::validate`), its tests and README;
  - a test in `crates/aethelgard-hwdef` for the guard;
  - `spike/1214/` and FEEDBACK.
- **Not:**
  - the generator's code;
  - `pincheck` or specifications;
  - workflows, or `ci/` (except a weight row).
- **Running beside you:**
  - `4240b` (IR clock, specification 23, in the package queue);
  - `1211b` (generator);
  - `1211d` (eda);
  - `1211e` (IR pin-device `validate`, specification 24: you share `validate`, so merge `main` early);
  - `2307` (CLI).
- Merge `main` before READY. One PR, left unmerged. Write `~/.cache/aet-1214/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A baud source the rule misses:** a role default, or a protocol that sets its own rate.
- **A port the generator writes** with a define the IR never sees.

## 7. Flag-and-stop conditions

- **A committed specification would be refused.** Stop and report it.
