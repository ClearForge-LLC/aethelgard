# AET-WO-4400e3 — The twin corrects Renode 1.17.0's `ICSR.RETTOBASE` to the architecture, by a named monitor read hook, until the pinned Renode carries the fix

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4400e3-rettobase`.
**Author:** Claude (architect) · **Builder:** strong tier (the `4400e2` builder, reused), with a targeted adversarial pass.
**Phase:** P4. Written 2026-11-17 from `AET-WO-4400e2`'s stop (PR #215, ruling comment on that PR, fix (1)). **Cadence: build, no stop unless §7 fires.**

**Grounds (`4400e2` FEEDBACK, measured):**
- **Renode 1.17.0's bug:** `NVIC.cs` L1079 computes `ICSR.RETTOBASE` as "active **system** exceptions ≤ 1". A nested external interrupt therefore reads 1, which Armv7-M forbids.
- **What it does to ArduPilot:** ChibiOS's epilogue (`chcore.c` L315) builds a thread-mode switch frame on that bit. With TIM5 nested over a DMA stream's interrupt, the idle thread later returns into a stray frame and switches to `ch0`. That is the `INVSTATE` hard fault of `4400e` B1, and of `4400c2` D3: the same fault, measured.
- **The fix, measured on both repros:** a one-line monitor read hook on `sysbus.nvic` that gives `ICSR` reads the architectural `RETTOBASE` (`spike/4400e2/stubs-mon-fix.txt`).
  - No fault on either repro.
  - Unmodified ArduPilot reaches `Init Gyro` and polls `FIFO_COUNT` 975 times.
  - The cost is 9.4 s of wall time and 500 MiB for 4 s of virtual time.
  - It touches only bit 11; `VECTACTIVE` is unchanged.
- **Upstream** fixed it in `renode-infrastructure` `e7a8584c`. No release carries it yet.

> **What this is:** the emulation made architectural for one register bit, in the monitor script the CLI already owns. It is a named, cited correction of a pinned simulator's bug, removed when the pinned Renode carries the upstream fix. It is not a stub and not a weakening.
>
> It is NOT:
> - a Renode image change (backporting `e7a8584c` or pinning a later Renode is the backlog's, at the next image rebuild);
> - the foreign judge (`4400e`);
> - the ICM's FIFO (`4220c`).

## 1. Build

1. **The hook,** in the monitor script the CLI emits (`renode.rs`, or wherever `4400e2` placed it in the measured script), applied to every boot.
   - A comment cites the Armv7-M Architecture Reference Manual's definition of `RETTOBASE` (section and page), `NVIC.cs` L1079 at the pinned commit, and upstream `e7a8584c`.
   - It reads only what the architecture defines `RETTOBASE` from: the active exceptions, system and external.
   - Every other bit of `ICSR` is passed through unchanged.
2. **It is tied to the pin.**
   - The hook is emitted only while the pinned tools image's Renode is the one with the bug (its version and commit, read from the image's pin as the CLI already reads it).
   - A test fails if the pin moves and the hook is still emitted, or is dropped without a measured check that the new Renode is fixed.
   - So the correction cannot outlive the bug, or vanish while the bug remains.
3. **A regression witness that needs no ArduPilot.** A minimal firmware, built in the tools image, that nests two external interrupts at the priorities `4400e2` measured and reads `ICSR` inside the inner handler:
   - it must read `RETTOBASE` 0 with the hook;
   - it reads 1 without the hook, in a test that names the bug.
4. **Every generated-firmware verdict is byte-identical:** the reference, the bench board and every scenario. The firmware arms no interrupt (`4400e2`: inferred). Measure it.

## 2. Acceptance

- **Both repros are free of the fault,** under the CLI's own emitted script:
  - `4400e`'s ELF reaches `Init Gyro`;
  - `4400c2`'s repro passes 1.6016 s.

  Record each as measured, against the commit.
- **The regression witness passes with the hook, and fails by name without it.**
- **No generated-firmware verdict moves,** byte for byte.
- **The pin test,** as §1.2.
- **The CLAUDE.md gates pass.** CI is green, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-03`, `INV-06`, `INV-07`; `RUL-05`, `RUL-13`, `RUL-16`, `RUL-21`.
- **Fence:**
  - the CLI's monitor-script emission (`aethelgard-cli`, `renode.rs` and its tests);
  - the regression firmware and its tier-2 test;
  - `spike/4400e2/` (deleted once cited);
  - FEEDBACK.
- **Running beside you:** `4220c` (the ICM model's FIFO, `models/renode/`). You share no file with it. One container of yours at a time, under `capped-run`.
- One PR, left unmerged. Write `~/.cache/aet-4400e3/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A hook that changes another bit of `ICSR`,** or `VECTACTIVE`.
- **A hook that reads 0 when the architecture says 1:** one active exception, returning to thread mode.
- **A hook that silently survives a Renode upgrade,** or is silently lost.

## 7. Flag-and-stop conditions

- **A generated-firmware verdict moves:** stop, with the bytes.
- **The hook cannot be tied to the pin** without a change to `container/`: stop and say what.
