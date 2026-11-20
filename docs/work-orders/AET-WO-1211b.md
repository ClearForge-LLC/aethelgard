# AET-WO-1211b — Pin devices generated: idle words, GPIO and ADC bring-up, bridge enables after `CEN`, Kani's oracles and the ledger's classes

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1211b-pin-devices-gen`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4, before the baseline. Written 2026-11-20. **Cadence:** a measurement, then a build with stops. **After `1211a` merges** (it lands the IR and the named refusals this work order lifts).

**Grounds:**
- `AET-WO-1211` FEEDBACK §5, item 2, and §2.4, §2.6 and §2.7, as ruled (Q1–Q10, 2026-11-20).
- `1211a`'s FEEDBACK: its §3 F3 says `generate` and `hwdef` refuse idle high until this work order.
- Read those first. This work order orders the build; it does not restate them.

> **What this is:**
> - **The generator** (`aethelgard-svd-hal`), for a specification with pin devices or a declared PWM idle:
>   - the idle CCR word: 0 for `low`, ARR + 1 for `high`, the pulse otherwise;
>   - `PSC` from `resolution_ps`;
>   - GPIO outputs written to their declared initial level before `MODER` = output, and inputs configured with their pull;
>   - ADC1/ADC3 enabled for the solved channels;
>   - a bridge's enable input written after its bridge's `CEN`, waiting `t_wake` when it wakes the part.
> - **`motor_output_pupd`** follows the idle (Q9), in the generator and in the oracle (`implied_pupd`).
> - **Kani's oracles** are derived from the specification: the idle words, the GPIO initial levels, and a bridge's static idle being a non-driving row.
> - **The ledger's classes** (`1211` §2.7) for every new register field written. An unclassified field stays a hard error.
> - `PinDeviceNotGenerated` and the idle-high refusal are lifted at `generate`. `hwdef` keeps `GpioNotEmitted` (ArduPilot is out of scope).
>
> It is NOT:
> - the twin's ADC or GPIO witnesses, or chaos (`1211c`);
> - eda (`1211d`);
> - any change to an existing specification's emitted bytes or ELF. With `idle` = `pulse_min_us` and `resolution_ps` = 1,000,000, nothing moves (`1211a` measured the words).

## 1. Build

1. **Measure first:** every committed specification's generated tree and ELF on `main`.
   - **Prediction:** unchanged.
   - **Stop and report** on any byte that moves, except a line `1211a` already moved.
2. **The generator, the oracle and the ledger,** as above. Each new word comes from the solved topology, never recomputed.
3. **Kani:**
   - one harness per new oracle, or the existing harness widened. Report each one's time;
   - **stop** if any exceeds 600 s.
4. **Fixtures:** `1211a`'s D1 (DRV8876, PH/EN, nSLEEP, nFAULT, IPROPI) generates, builds and proves. D2 stays refused, as `1211a` recorded.
5. **Mutants:**
   - write the idle after `CEN`;
   - drop a GPIO initial level;
   - enable a bridge before its `CEN`;
   - an unclassified ledger field.

   A test or a proof fails for each.

## 2. Acceptance

- **No existing specification's byte or ELF moves.**
- **D1 generates, builds and proves.**
- **The CLAUDE.md gates pass, and CI is green.** `TIER2_MIN` rises by your own tier-2 count.

## 3. Fences

- **Fence:**
  - `crates/aethelgard-svd-hal`;
  - `crates/aethelgard-cli/src` (lifting the refusals at `generate`; the ledger);
  - `crates/aethelgard-ir/src`, read-only except the oracle helpers the generator needs, which you must name;
  - tests and fixtures;
  - `spike/1211b/`;
  - FEEDBACK.
  - **Not:** the twin, chaos, eda, the IR schema, workflows, or `ci/` (except weight rows).
- **Running beside you:** `4240b` (generator: the bring-up and `Scale2`, so expect conflicts in `board.rs`), `4240c` (eda), `1211d` (eda), `1213` (IR `validate`), `1913` (CI). Merge `main` often; before READY, without fail.
- **One container of yours at a time,** under `capped-run`. Kani at most one harness at a time.
- One PR, left unmerged. Write `~/.cache/aet-1211b/pr-title-body.md`.

## 5. Adversarial pass, full

- **A driver that generates and drives its motor at boot:** an idle written late, an enable before `CEN`, or a pull fighting the idle.
- **A bridge whose oracle passes** a tuple the row's truth table marks as driving.

## 7. Flag-and-stop conditions

- **An existing specification's byte or ELF moves.** Stop with the diff.
- **A Kani harness exceeds 600 s.** Stop with the time.
