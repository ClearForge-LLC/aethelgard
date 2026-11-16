# AET-WO-1207 — `validate` refuses the rest of what `solve` refuses statically, and an SPI device whose modes the controller cannot drive

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1207-validate-rest`.
**Author:** Claude (architect) · **Builder:** strong tier, with a targeted adversarial pass.
**Phase:** backlog (roadmap §7.1), written 2026-11-16. **Cadence: build, with one measured stop (§1.1).**

**Grounds (§7.1 rows, each with its source):**
- **`1206` F3:** `MissingSpiConstraint`, `RiseTimeTooSlow` and `SupplyOutsideSpeedTable` are still refused only by `solve` (`ir/src/solve.rs`), although each is decidable from the specification alone.
- **`1206` F7:** `COM0` and `LPT0` are missing from the Windows reserved names (`ir/src/names.rs`).
- **`1206` F1:** the emitter's own name check (`schematic.rs`'s `spelled`) omits serial and motor nets. `validate` covers the CLI, but a library caller of `schematic::emit` can still short `U1_VCAP1`.
- **Grok review 2026-11-16, row 5:**
  - `parts.json` SPI `modes` are parsed and checked for shape (`parts.rs`), then never read.
  - The generated firmware drives mode 0.
  - A device whose modes exclude the mode the controller drives is not refused. Today all three SPI rows list `[0, 3]`.

> **What this is:** each refusal moves to the earliest stage that can decide it, by the same name and JSON shape the later stage uses (as in `AET-WO-1206`), and the SPI mode becomes a judged fact.
>
> It is NOT:
> - a specification field or version (the specification is frozen at 12, roadmap §6);
> - a change to any emitted byte;
> - the chip-select backtrack (its own §7.1 row).

## 1. Build

1. **Measure first (the stop).** Run each candidate rule over every example, fixture and scenario in the repository, and list what it would refuse. **STOP** with the list if any rule would refuse a specification that `generate` accepts today.
2. **The three refusals at `validate` (F3).** Use the same check as `solve`, shared or equivalent, under the same name and JSON. Keep `solve`'s refusal as a guard, with a test that reaches it, only if a library caller can reach it without `validate`. Record which applies.
3. **Windows names (F7).** Add `COM0` and `LPT0`, with the folding rule `1206` built.
4. **The emitter's names (F1).** `spelled` covers every net the emitter names, serial and motor included. Add a library-level test that calls `schematic::emit` directly with `U1_VCAP1` as a serial net.
5. **SPI modes (Grok 5).**
   - Read from the generator the one SPI mode the firmware drives, and state where it is set.
   - `solve` (and `validate`, if the mode is static) refuses a device whose `modes` exclude it, by a new named refusal that cites the part's row.
   - If the generator could drive a per-device mode, say so in FEEDBACK as a question, and do not build it.

## 2. Acceptance

- Every new refusal has a tier-1 test showing the old behaviour first: `validate` passes, and `solve` or `generate` refuses.
- No existing example, fixture or scenario changes its verdict.
- Every emitted file is byte-identical.
- **The CLAUDE.md gates pass.** CI is green, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants:** `INV-01`, `INV-06`; `RUL-16`.
- **Fence:**
  - `aethelgard-ir` (`lib.rs`'s validation, `solve.rs`'s refusal set, `names.rs`, `parts.rs`'s read of `modes`);
  - `aethelgard-eda` `schematic.rs` (`spelled` only);
  - tests;
  - FEEDBACK.
- **Running beside you:**
  - `AET-WO-4211` changes `parts.json`'s W25Q64JV row and may fold the two `Lifecycle` enums (`parts.rs`, `power_parts.rs`). Merge `main` as it lands, and keep your hunks out of the lifecycle code.
  - `AET-WO-2302` changes the CLI's twin judge; you do not touch the CLI.
- No version number of any kind.
- One PR, left unmerged. Write `~/.cache/aet-1207/pr-title-body.md`.

## 5. Adversarial pass, targeted

- A specification that `validate` now passes but `solve` refuses (search for one).
- A `modes` list that is empty, holds the driven mode twice, or uses another case or form.
- A serial or motor net named like an emitter net, reaching `emit` by the library path.

## 7. Flag-and-stop conditions

- **§1.1 finds an existing specification a new rule would refuse.**
- **The driven SPI mode is not one fixed value** (it depends on the solve or the part): stop with where it is decided.
- **Keeping the JSON byte-identical would need two error types to stay apart.**
