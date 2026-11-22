# AET-WO-1320 — The arm table's `check()` holds what it now leaves to run time: one unstated passive per pin, an I2C row's address, the decoupler count, a name used twice, a pin listed twice

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1320-arms-check`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4. Written 2026-11-22. **Cadence:** a small build, measured against an unchanged byte. **After `1319`** (#310) and `1211f` (#316).

**Grounds** (roadmap §7.1):
- **`1211f` F2:** `check()` allows two `value: None` passives on one supply pin to GND. One `bulk` would answer both, and the sheet writer's duplicate-reference check would refuse it later, under a misleading kind.
- **`1319` F3:** what `check()` does not hold, each unreachable by today's rows:
  - an I2C row with no `Addr` (`validate` still refuses an unsupported address);
  - the count of a second, third or fourth decoupler (only the byte sweep holds those);
  - a strap and an own net of one name in one row (refused at run time as `ReservedName`, which blames the specification);
  - a pin listed twice in a row (a unit test, not the `const` check).
- Read `1319` FEEDBACK §4–§5 and `1211f` FEEDBACK first. `check()` is the `const fn` in `crates/aethelgard-eda/src/schematic_arms.rs`, evaluated over `ARMS` when the crate compiles.

> **What this is:**
> - **Five lines of `check()`**, one per item above. Each refuses a bad table when the crate compiles, and each has a test over a bad table, as `1319`'s four hazards do.
> - **The decoupler count** is held against the row's own passives list, so a row that drops or adds a decoupler fails to compile, not only the byte sweep.
>
> It is NOT:
> - any change to a row, a drawing or an emitted byte;
> - the `device_nets` restatement (`1319` F1, the schema-epoch decision);
> - the power tree's `two_pin` fold (`1211f` F3).

## 1. Build

1. **Measure first:** the emit baseline on every committed specification, and `1319`'s harnesses (`spike/1319/`), with predictions committed.
2. **The five checks,** each with its bad-table test and a mutant the test kills.
3. **Re-run the baseline:** identical.

## 2. Acceptance

- **Every committed specification's emitted bytes, exit codes and stdout are identical,** and so are `1319`'s harnesses.
- **Each of the five is refused at compile time** by a test over a bad table.
- **The CLAUDE.md gates pass, and CI is green.** `TIER2_MIN` is unchanged.

## 3. Fences

- **Fence:** `crates/aethelgard-eda` (`check()` and its tests); `spike/1320/` and FEEDBACK.
- **Not:** the IR, the CLI, the generator, the twin; workflows or `ci/`.
- **Running beside you:** `1216b` (IR), `2308` (`verify`, and eda's `fab.rs` note column only), `4300h` (a spike). Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-1320/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A check that refuses a row in today's table.**
- **A check whose test passes with the check deleted.**

## 7. Flag-and-stop conditions

- **A check cannot be written in a `const fn` on stable Rust.** Stop and report which, and where it would run instead.
- **Any emitted byte moves.** Stop.
