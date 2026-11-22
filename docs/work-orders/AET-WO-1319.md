# AET-WO-1319 — The device arms as rows: one table and one interpreter in eda, every emitted byte unchanged

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1319-arms-as-rows`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4. Written 2026-11-22. **Cadence:** a build, measured against an unchanged byte.

**Grounds** (`4600` F15 and D4; `1211d` FEEDBACK §4, "measured, proposed"; roadmap §7.1, the `1211d` §4 row):
- **Every part costs code.** Each device arm is hand-written in `schematic.rs`, and a new part costs edits in four files.
- **The prototype exists.** `1211d` wrote the six bus-device arms (BME280, BMP581, TMP117, W25Q64JVxIQ, ICM20948, ICM42688P) as six table rows and one interpreter (`spike/1211d/d4/table-interpreter.patch`).
  - It gave identical bytes on all 31 committed specifications (259 files).
  - A second reviewer ran 274 further inputs with no divergence (926 files). Two planted mutants made 451 files differ, so the harness can fail.
- **Read `1211d` FEEDBACK §4 first:**
  - the eight interpreter features a row cannot say as plain data;
  - the four row-authoring hazards;
  - the costs a new part still carries beyond its row.

> **What this is:**
> - **One table of arm rows in `aethelgard-eda`,** beside the symbol library. Each row carries:
>   - part, bus, symbol and footprint;
>   - the pin → role map;
>   - the required passives (value, rating, footprint, page);
>   - an optional stand-in.
> - **One interpreter** draws every bus-device arm and both pin-device arms (`1211d`'s `Arm` is already that shape). It replaces the hand-written arms, which are deleted.
> - **Tests:**
>   - rows against `KNOWN_PARTS`, replacing the source-scan test;
>   - each row's own-net capacitor names its own `Own` pin;
>   - no part has two rows;
>   - an address role on an SPI row is refused when the table is built, not when a board is drawn.
>
> It is NOT:
> - passives as power-parts rows (`1211d` R9/A4: a later work order);
> - deriving `KNOWN_PARTS` or `aethelgard_ir::names::device_nets` from the rows. That moves message bytes and is a schema-epoch decision. FEEDBACK says what it would take.
> - any change to an emitted byte, a refusal's kind or message, or a part's drawing.

## 1. Build

1. **Re-take the baseline** at the `main` you start from, with predictions committed first, in `spike/1319/`:
   - every committed specification's `emit` output, exit code and stdout;
   - `1211d`'s 274-input harness (`~/.cache/aet-1211d/d4harness/` on Furnace, or its copy in `spike/1211d/d4/`).
2. **Port the patch onto current `main`.** `main` gained the two pin-device arms after the patch was written; fold them into the table as well.
3. **Delete the hand-written arms.** Report the net line count: the patch measured +108, so say what the pin-device arms change about that.
4. **Order:** a row's order is the draw order. Keep today's order exactly. Any reorder is a byte change, and a stop.

## 2. Acceptance

- **Every committed specification's emitted bytes, exit codes and stdout are identical** to the baseline, and so is the 274-input harness. Run the harness's two planted mutants again, to show it can still fail.
- **`parts_arms_tier1` passes,** with every part drawn.
- **The CLAUDE.md gates pass, and CI is green.** `TIER2_MIN` is unchanged.

## 3. Fences

- **Fence:**
  - `crates/aethelgard-eda` (the table, the interpreter, the deleted arms) and its tests;
  - `crates/aethelgard-kicad-lib` only as far as a test names a symbol;
  - `spike/1319/` and FEEDBACK.
- **Not:**
  - the IR (`KNOWN_PARTS`, `device_nets`, `parts.json`);
  - the generator or the twin;
  - specifications;
  - workflows, or `ci/` (except a weight row).
- **Running beside you:**
  - `1211b` (generator, solved 16, in the package queue);
  - `1211e` (IR, specification 24, next in the package queue);
  - `2307` (CLI);
  - `1215` (IR version table).
  None of them touches eda. Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-1319/pr-title-body.md`.

## 5. Adversarial pass, full

- **An input the 274 do not cover,** where the interpreter and the deleted arms would differ. Build the old arms from the base commit for this comparison.
- **The four hazards in `1211d` §4:**
  - the own-net capacitor's `format!` name;
  - the address role on an SPI row;
  - a duplicate row;
  - an I2C decoupler left out.
  Each must be refused by a test, or reported.

## 7. Flag-and-stop conditions

- **Any emitted byte, exit code or refusal message moves.** Stop and report it with the diff.
- **A row cannot express a part without a new interpreter feature beyond §4's eight.** Stop and report it.
