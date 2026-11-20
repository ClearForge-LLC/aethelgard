# AET-WO-1209c — A power load never names a part whose current its own row already counts

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1209c-load-counted-once`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P1 (validation). Written 2026-11-20. **Cadence:** build.

**Grounds** (`1209b` F1 and §4 A1, #270; roadmap §7.1):
- **The rule the power tree already states.** `ir/src/power.rs`'s module doc says the MCU's load is its table row's, never typed by hand, and a regulator's load includes each child regulator's input current. `Load::reference`'s doc says the same: each part that can draw current, "other than the MCU and the child regulators, which their part rows count".
- **Nothing enforces it.** `1209b` measured that a load naming the LDO `U12` passes on its input rail (`VDD_3V3`) and on the rail it makes (`VDD_1V8`):
  - on the input rail, the regulator's current is counted twice;
  - on the output rail, the claim is wrong: the regulator is that rail's source.
- **Nothing committed names one** (`1209b`, measured for regulators). Confirm it for the MCU.

> **What this is:**
> - the IR's `validate` refuses a `power.loads` entry whose `reference` names the MCU or a declared regulator (`power.regulators[].reference`), by one new kind;
> - the refusal names the reference, the domain, and which row already counts it.
>
> It is NOT:
> - a change to how the supply gate counts current;
> - `R7`'s stable-reference row;
> - any change to a committed specification.

## 1. Build

1. **The refusal** in `aethelgard-ir`'s `validate`, beside `LoadNamesNoPart` (`1209`):
   - name the kind, `LoadNamesCountedPart`, unless an existing family suggests better (say why);
   - it covers the MCU's reference, in every form `1209` accepts, and every regulator's reference;
   - it gives `error.path`, the domain, and the counting row: "the MCU's table row" or "regulator `U<n>`'s input on `<domain>`".
2. **Order:** `LoadNamesNoPart` is judged first. The new kind applies only to a reference that names a real part.
3. **Tests:**
   - `U12` named on `VDD_3V3` and on `VDD_1V8` (from `1209b`'s `adv/s02-U12-*` copies): each is refused by name;
   - the MCU named: refused;
   - every committed specification validates exactly as on `main`.
   - **Mutants:**
     - regulators only: the MCU test fails;
     - the MCU only: a regulator test fails.
4. **Corpus:** every committed specification (about 52 files) validates byte-identically. A newly refused file is a stop.

## 2. Acceptance

- The refusal, the tests and the mutants are shown.
- Nothing committed is newly refused, and no emitted byte moves.
- **The CLAUDE.md gates pass, and CI is green.** `TIER2_MIN` does not move.

## 3. Invariants and fences

- **Invariants:** `INV-06`, `INV-01`.
- **Fence:**
  - `crates/aethelgard-ir/src` (`validate`, the new kind, its message and path);
  - the IR README's list of kinds, one line;
  - tests;
  - FEEDBACK.
  - **Not:** the CLI, eda, the supply gate, specifications, `boards/`, `fab/`, workflows or `ci/`.
- **Running beside you:**
  - `4240v` (#259) takes spec 19. You add no field, so no version moves.
  - `2306` (#269), `2305` (#271) and `4405` (pincheck) touch nothing of yours.

  Merge `main` before READY.
- **One container of yours at a time,** under `capped-run`, at `nice -n 10`.
- One PR, left unmerged. Write `~/.cache/aet-1209c/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A part that is both a load and counted elsewhere:** a regulator's enable pull-up, or a part `1209b` §4 A2 named in a series chain. Is it refused? It must not be.
- **The MCU named by a form `1209` accepts** that this rule misses.

## 7. Flag-and-stop conditions

- **A committed specification is newly refused.** Stop with the list.
