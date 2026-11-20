# AET-WO-1213 — `validate` refuses a console baud that ArduPilot cannot open at boot as declared

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1213-console-baud`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4. Written 2026-11-20. **Cadence:** a measurement, then a small build.

**Grounds** (`4405` D1, #277; roadmap §7.1):
- ArduPilot opens the console at boot at `DEFAULT_SERIAL0_BAUD` (`AP_SerialManager.cpp` `init_console`, about L419).
- `var_info` holds that rate only as the define divided by 1000 (`:172`).
- The judge reads `var_info` through `map_baudrate` (`:737-770`), as `4405` built it in `pincheck`.
- **The residual:** a console baud `b` with `map_baudrate(b / 1000) != b` opens at boot at a rate the judge never sees. Every committed board divides exactly (115200, 57600, 9600).

> **What this is:** `validate` refuses, by name (`ConsoleBaudNotRoundTrip`), a specification whose console baud `b` gives `map_baudrate(b / 1000) != b`.
> - The rule lives in `aethelgard-ir`, with its own cited copy of `map_baudrate`. It is the specification's rule, not `pincheck`'s, and the two copies are the `RUL-20` pair.
> - Measure first which specifications and roles it reaches.
>
> It is NOT a change to `pincheck`, the generator or any committed specification's bytes.

## 1. Build

1. **Measure first,** with predictions committed before the runs:
   - every committed specification's console baud (and any serial role ArduPilot's `init_console` opens);
   - whether `map_baudrate(b / 1000) == b` for each.
   - **Prediction:** all pass.
   - **Stop and report** if a committed specification would be refused.
2. **The rule** in `validate`, its own `map_baudrate` cited line by line to the pinned ArduPilot tag.
   - Name which serial roles it covers, and why. A role ArduPilot does not open at boot from `DEFAULT_SERIALn_BAUD` is not covered.
3. **Tests:**
   - 115200 passes;
   - a baud such as 100000 or 250000, chosen so it fails the round trip on the cited table, is refused by name;
   - a cross-check against `pincheck`'s copy over every baud in the table plus a sweep;
   - **mutant:** drop the rule, and a test fails.

## 2. Acceptance

- **The measurement** is in FEEDBACK. No committed specification is refused.
- **The CLAUDE.md gates pass, and CI is green.** `TIER2_MIN` is unchanged.

## 3. Fences

- **Fence:** `crates/aethelgard-ir` (the rule and its `map_baudrate`), tests, `spike/1213/`, FEEDBACK.
- **Not:** `pincheck`, the generator, specifications, workflows, or `ci/` (except a weight row).
- **Running beside you:** `1211a` (IR, specification 22), `4240b` (IR clock, specification 23), `4240c` (eda), `1913` (CI weights). Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-1213/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A console role or baud source the rule misses,** such as a declared baud that the generator rewrites.
- **The IR's `map_baudrate` and `pincheck`'s** disagreeing on any input.

## 7. Flag-and-stop conditions

- **A committed specification would be refused.** Stop and report it.
