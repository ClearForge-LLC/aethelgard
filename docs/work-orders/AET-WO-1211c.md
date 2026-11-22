# AET-WO-1211c — Pin devices simulated, faulted and verified: the twin's ADC and GPIO witnesses, `pin_held` and `analog_value`, coverage targets and per-item gates

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1211c-pin-devices-twin`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4, before the baseline. Written 2026-11-22. **Cadence:** a measurement, then a build with stops. **After `1211b`**, which merged as #307.

**Grounds:**
- `AET-WO-1211` FEEDBACK:
  - §5 item 3 (this work order's scope);
  - §2.7 (what each projection does: the twin, ledger and verify rows of (a)–(e));
  - §4 (the twin's reach, measured: G0–G5 and the timer rows T1 onward);
  - §8 (Q1–Q10, as ruled).
- `1211b` FEEDBACK:
  - `simulate` refuses a pin device (`PinDeviceNotSimulated`) and an idle high (`IdleHighNotSimulated`) until this work order;
  - a pin device's tree has no `.repl` (S2);
  - every ADC field is ledger `silicon-only`.
- Read those first. This work order orders the build; it does not restate them.

> **What this is:**
> - **The twin:**
>   - `twin::kind` gains ADC1 and ADC3, with every declared channel set explicitly;
>   - a static-level motor witness (an idle low or high, as zero edges at its level, T1–T2);
>   - the GPIO witnesses §4 allows: the BSRR write and its order before `MODER`, and never a pad level (G1). A pull is never witnessed (G2).
>   - `PinDeviceNotSimulated` and `IdleHighNotSimulated` are lifted for what is witnessed, and kept by name for what is not.
> - **Chaos:**
>   - `pin_held {net, level}` drives an input (G3);
>   - holding a declared output is refused (`PinHeldIsOutput`, G4);
>   - `analog_value` sets an ADC input;
>   - the next scenario version and the next trace version.
> - **verify:**
>   - coverage targets for inputs (`Target::Gpio`), ADC inputs and bridges;
>   - per-item gates in `2306`'s shape, one per signal and never a whole input counted once;
>   - the fidelity notes of `1211` §4, recorded as ledger reasons.
> - **The motor rail's coverage** (`1211` §2.7 (a)). A part's supply (`VMOTOR`) becomes a `Rail` target that no power tree droops, so `--fab-out` is blocked (`campaign_faults_every_bus_and_rail`).
>   - **Measure first.** Propose, as a stop, either a droop on a sourceless domain or the domain's exclusion, with its reason.
>   - Do not decide it yourself.
>
> It is NOT:
> - eda, or `bulk` (`1211e`, and the eda follow-up);
> - a Renode model of either driver part (the device stays `device_unmodelled`);
> - any change to an existing specification's verdict, campaign or emitted byte.

## 1. Build

1. **Measure first,** with predictions committed before any run, in `spike/1211c/`:
   - re-run §4's G and T rows on `main`'s Renode pin;
   - D1 and D2 under `simulate`, today (both refused by name);
   - the motor rail's coverage on D1 and D2.
   - **Stop on the motor rail with the measurement and a lean.**
2. **The twin's ADC and GPIO witnesses.** Name every field they cannot witness, by name.
3. **Chaos:** the two fault kinds, the refusal, and the version bumps. Each new record round-trips through the archive, and an old trace version is refused by name.
4. **verify:** the targets and the per-item gates. A run with no pin device moves no verdict byte.
5. **Tests and mutants:** one per witness, fault kind, gate and refusal.

## 2. Acceptance

- **D1 and D2 simulate,** with every unwitnessable field named.
- **A `pin_held` on nFAULT is detected** by a gate. Holding an output is refused by name.
- **Every committed specification's verdict, campaign and emitted bytes are unchanged,** except the version lines the bumps move. Show this by a null sweep.
- **The CLAUDE.md gates pass, and CI is green.** If the trace version moves the bench package's `verdict_sha256`, queue for a dispatch: the package queue is serial, behind `1211e`.

## 3. Fences

- **Fence:**
  - `crates/aethelgard-twin`, `crates/aethelgard-chaos`, and `crates/aethelgard-cli`'s `verify`, `simulate` and coverage, with their tests;
  - the ledger's reasons;
  - the version literals the bumps move, and the goldens they reach, by their generators;
  - `spike/1211c/` and FEEDBACK.
- **Not:**
  - the IR schema or the specification version;
  - the generator, except a refusal it must lift;
  - eda;
  - workflows, or `ci/` (except weight rows).
- **Running beside you:**
  - `1211e` (IR, specification 24, next in the package queue);
  - `2307` (CLI `verify`'s acknowledgement refusal: the same file as your coverage work, so merge `main` early);
  - `1215` (IR);
  - `1319` (eda);
  - `4240d` (a bench sheet).
- Merge `main` before READY. One PR, left unmerged. Write `~/.cache/aet-1211c/pr-title-body.md`.

## 5. Adversarial pass, full

- **A witness that claims a pad level or a pull the twin cannot see** (G1, G2).
- **A `pin_held` that reaches an output by another name.**
- **A per-item gate that counts a whole input once** (`2306`'s defect).
- **A verdict of an existing board that moves.**

## 7. Flag-and-stop conditions

- **The motor rail's coverage:** stop with the measurement and a lean (§1.1).
- **Any existing verdict, campaign or emitted byte moves beyond the version lines.** Stop and report it.
- **A witness needs a Renode model change.** Stop: that is Tviburi's side of the seam, not ours.
