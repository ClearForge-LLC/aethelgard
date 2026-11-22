# AET-WO-1218 — Test points beside pin devices, the analog-pad check on the MCU actually solved, and the power tree's reservoir against VDDA's maximum

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1218-ir-defects`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4, before the P4 baseline. Written 2026-11-22. **Cadence:** a measurement, then a build. **After `1217`** (#327).

**Grounds** (roadmap §7.1; read `1217` FEEDBACK §7 and `1216b` FEEDBACK first):
1. **`1216b` F1 = `1217` R6, a defect since `1211a`.** No test point validates on any board with a pin device: `names::references` chains pin devices after test points, and the bench's step 15a assumes test points come last (`TestPointReferenceTaken`, `bench.rs:154`, `names.rs:125`). A motor-driver board cannot carry a test point today.
2. **`1217` R2.** `check pinout` ignores `topology.mcu`, so a board whose MCU is renamed escapes `1217`'s analog-pad check.
3. **`1217` R3.** `power.rs`'s sense reservoir is still judged against VDDA's **minimum**. `1217` F8 fixed the same defect for pin devices; this is the power tree's copy.
4. **`1217` R4 and R5.** `NoInstance`'s wording; an unneeded reservoir is never looked up.

> **What this is:**
> - **(1)** a test point and a pin device on one board validate, with references assigned in one order that every reader shares. **Measure first** what moves: a reference change on any committed board is a stop (§7).
> - **(2)** `check pinout` reads the MCU from the topology it judges, and refuses a pad claim on a renamed MCU.
> - **(3)** the power tree's reservoir rated for VDDA's maximum, by `1217` F8's helper.
> - **(4)** as the rows say.
> - Each with a test that fails without it.
>
> It is NOT:
> - any change to a committed board's references, pinout or emitted bytes beyond the version lines;
> - the analog-pad data itself (`1217`'s).

## 1. Build

1. **Measure first,** with predictions committed in `spike/1218/`: the validate/solve/emit baseline, and for (1) a scratch board with D2's pin device plus a test point, today and after the change.
2. **(1)–(4),** each with its test and a mutant it kills.
3. **The null sweep.**

## 2. Acceptance

- **A board with a pin device and a test point validates, solves and emits,** and its references are stable across two runs.
- **(2)–(4)** each closed by a test that fails without it.
- **Every committed specification is byte-identical** except the version lines a bump moves (a bump is due if (1) admits content the version before refused, `1215` D4).
- **The CLAUDE.md gates pass, and CI is green.** If a `fab/` byte moves, queue for a dispatch.

## 3. Fences

- **Fence:** `crates/aethelgard-ir` (names, bench step 15a, power, versions, README, tests); `crates/aethelgard-pincheck` for (2); the version literals and goldens a bump moves, by their generators; `spike/1218/` and FEEDBACK.
- **Not:** the generator, the twin, chaos, eda, `verify`; workflows.
- **Running beside you:** `4300h` (a spike, stopped for Scotty's ruling on Sentinel-03's motor pins). Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-1218/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A reference that differs between two readers** (eda, the bench, `check pinout`, the generator).
- **A renamed MCU that still escapes a check.**

## 7. Flag-and-stop conditions

- **(1) moves a committed board's references.** Stop with the diff.
- **Any other committed byte moves beyond the version lines.** Stop.
