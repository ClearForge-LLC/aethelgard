# AET-WO-1209b — A power load's part must have a pin on the load's rail

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1209b-load-on-rail`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P1 (validation). Written 2026-11-20. **Cadence:** build.

**Grounds** (`1209` F4, #253; `4202`'s renumbering, #257; roadmap §7.1):
- **`1209` made `validate` refuse a load that names no part the board has.** The IR refuses `LoadNamesNoPart`; the CLI refuses `LoadNamesUndrawnPart` against the emitter's drawn set.
- **It cannot see a load that names a real part, but the wrong one.** When U2 left Sentinel-03, every later strap renumbered:
  - `R8` came to name BOOT0's pull-down, which goes to GND, not VDD_3V3;
  - `R7` came to name the HOLD pull-up, which is still on VDD_3V3.
- **`1209`'s adversarial pass proposed the partial catch:** a load's part must have a pin on the load's rail. That refuses `R8` and any load on the wrong rail. It cannot refuse `R7`, which stays a §7.1 row (a load named by what it is, not by its emitted number).

> **What this is:** the CLI's `validate`, on the solve it already runs (`quiet_solve`, shared with `outline_holds`), refuses a `power.loads` entry whose named part, as drawn, has no pin on that entry's rail. The refusal is `LoadPartOffRail`, naming the reference, the rail, and the nets the part does touch.
>
> It is NOT:
> - the stable-reference specification change;
> - anything in the IR's own `validate`, which cannot see drawn nets;
> - a change to any committed specification.

## 1. Build

1. **Read** how `schematic::drawn` (`1209`) exposes the drawn parts. Find, or add as one read-only accessor in `aethelgard-eda`, each drawn part's net per pin.
2. **The refusal:**
   - for each `power.loads` entry, the named part's pins' nets must include the entry's rail;
   - a part with no drawn pins (a declared off-board load, if any form allows it) is out of scope;
   - say which forms of load reference this covers.
3. **Tests:**
   - Sentinel-03 as committed passes. Its R6 and R7 are on VDD_3V3.
   - A copy whose R7 row is renamed to R8 (BOOT0's pull-down) is refused `LoadPartOffRail`, naming GND.
   - **Mutants:**
     - checking only that the part exists, and the second test fails;
     - checking any pin's net against any rail, and the test fails.
4. **Corpus:** every committed specification (`1209`'s 48-file list, now about 52) validates exactly as on `main`. Any newly refused file is a stop.

## 2. Acceptance

- The refusal, the tests and the mutants are shown.
- Nothing committed is newly refused.
- No emitted byte moves, and the bench package does not move.
- **The CLAUDE.md gates pass, and CI is green.** `TIER2_MIN` does not move.

## 3. Invariants and fences

- **Invariants:** `INV-06`, `INV-01`.
- **Fence:**
  - `crates/aethelgard-cli/src` (`validate` only);
  - one read-only accessor in `crates/aethelgard-eda`, if needed;
  - tests;
  - FEEDBACK.
  - **Not:** the IR, the emitter's behaviour, specifications, `boards/`, `fab/`, workflows or `ci/`.
- **Running beside you:**
  - `1103` (#262), `4240v` (#259), `2305` and `2306`. None touches `validate`.
  - Merge `main` before READY.
- **One container of yours at a time,** under `capped-run`, at `nice -n 10`.
- One PR, left unmerged. Write `~/.cache/aet-1209b/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A part with a pin on the rail that is not its supply pin,** such as a pull-up's far end. Is it a correct load? Say why.
- **A load on a rail the part reaches only through another part.**

## 7. Flag-and-stop conditions

- **A committed specification is newly refused.** Stop with the list.
- **Drawn pin nets cannot be read** without a solve the CLI's `validate` does not already run. Stop and say what it costs.
