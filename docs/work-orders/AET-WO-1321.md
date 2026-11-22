# AET-WO-1321 — eda's small backlog: `DECOUPLERS` checked both ways, the bulk's row push folded into `two_pin`, the IPROPI guard's text, and two stale names

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1321-eda-backlog`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4. Written 2026-11-22. **Cadence:** a small build, measured against an unchanged byte. **After `1320`** (#323).

**Grounds** (roadmap §7.1; read `1320`, `1211f` and `1217` FEEDBACK first):
1. **`1320` F2:** `DECOUPLERS`, the arm table's per-part decoupler count, has no reverse check. A part it names that has no row is not refused.
2. **`1211f` F3:** `declared_bulk` repeats `schematic_power::two_pin`'s row push, because that helper's `note` is fixed to `None`.
3. **`1217`'s ruling (Q1):** a DRV8876 may now tie IPROPI to GND. eda draws the tie generically, but `PinDeviceNotDrawable`'s guard text at `schematic_arms.rs:1358` still says an undeclared IPROPI is the only case, which is now stale.
4. **`1320` F1:** `spike/1319/harness/mutants.sh` still names the renamed `schematic_pin_device.rs`.
5. **`1320` F3:** `1319` FEEDBACK's hazard table names a message that has moved. FEEDBACK files are records and are not edited; instead, this work order's FEEDBACK states the moved message, and the harness is fixed.

> **What this is:**
> - **(1)** a `check()` line refusing a `DECOUPLERS` entry with no row, with its bad-table test.
> - **(2)** `two_pin` takes the note, and `declared_bulk` calls it. Byte-identical output.
> - **(3)** the guard text corrected to name both undeclared IPROPI and what the GND tie admits. **Only if `1217` has merged;** otherwise, leave it and say so.
> - **(4)** the harness's file name corrected, and the harness re-run to show it still kills its mutants.
> - **(5)** the moved message stated in this FEEDBACK.
>
> It is NOT:
> - the `device_nets` restatement (`1319` F1, the schema-epoch decision);
> - any change to a row, a drawing or an emitted byte.

## 1. Build

1. **Measure first:** the emit baseline and `1319`'s harnesses, with predictions committed in `spike/1321/`.
2. **(1)–(5),** each with a test or a re-run that can fail.
3. **Re-run the baseline:** identical.

## 2. Acceptance

- **Every committed specification's emitted bytes, exit codes and stdout are identical,** and so are `1319`'s harnesses.
- **The `DECOUPLERS` reverse check** is refused at compile time by a bad-table test.
- **The CLAUDE.md gates pass, and CI is green.** `TIER2_MIN` is unchanged.

## 3. Fences

- **Fence:** `crates/aethelgard-eda` and its tests; `spike/1319/harness/mutants.sh` (the file name only); `spike/1321/` and FEEDBACK.
- **Not:** the IR, the CLI, the generator, the twin; workflows or `ci/`.
- **Running beside you:** `1217` (IR, and one CLI test file) and `4300h` (a spike, stopped). Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-1321/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A fold that changes a drawn byte**, a note, or a reference order.
- **A reverse check that refuses today's table.**

## 7. Flag-and-stop conditions

- **Any emitted byte moves.** Stop.
