# AET-WO-1209 — `validate` refuses three things only a later step refuses today

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1209-validate-late-refusals`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P1 (`1207` and `1208`'s line: `validate` refuses what a later step refuses statically). Written 2026-11-19. **Cadence:** build.

**Grounds:** `INV-06` says malformed input is refused, and early. Three specifications pass `validate`, then fail later or not at all:

- **A device on a driven line with no pin capacitance** (`4600` F17).
  - `logger-spi-only` validated, solved, emitted, generated and built.
  - Only then did `extract` and `verify` refuse `UnboundedPinCapacitance` for an ICM-42688-P (`eda/src/extract.rs` about L1070). That part's row cites no maximum, and the specification declared none (`SpiDevice.pin_capacitance`).
  - `validate` holds everything that decision needs: the part's row, the bus, and the empty declaration.
- **A `power.loads` reference that names no declared part** (`4202`'s finding).
  - `validate` does not check it (`ir/src/power.rs` about L821–838).
  - The `4201` spike's "U2 removed" Sentinel-03 kept a load row for the absent BME280, and it passed every gate. The load then counts against a rail for a part that is not on the board.
- **A part with a row and no emitter arm** (`4600` F15).
  - ICP10111 has a parts row, so `validate` passes it, and `emit` then refuses it.
  - Whether a part can be drawn is a static fact the parts table and the emitter both know.

> **What this is:**
> - three refusals in `validate`, each by name, each with a mutant;
> - a test that each was refused by the later step on `main` before this change (the late refusal is the evidence);
> - every committed specification and example still validating, or a stop.
>
> It is NOT:
> - a new specification field or a schema bump;
> - a change to `extract`, `emit` or the parts rows. The later refusals stay, as a second line;
> - the parts-as-data work (`4600` D4, a §7.1 row).

## 1. Build

1. **Pin capacitance.** `validate` refuses a device on an I2C or SPI line that `extract` bounds when **both** of these hold:
   - its part row cites no maximum pin capacitance;
   - the specification declares no `pin_capacitance` for it.

   Name the device, the bus and the line, in the same terms `extract` uses.
   - **Read `extract`'s condition at source** and match it exactly: which lines it bounds, and which devices it counts.
   - **The coverage test:** for every row in the parts table, `validate`'s answer equals `extract`'s precondition. If they cannot be made to agree without changing `extract`, stop (§7).
2. **Load references.** `validate` refuses a `power.loads` entry whose reference names no declared part, and names the reference and the rail.
   - If a load may legitimately name something other than a declared part (a connector's off-board load, a test fixture's dummy), find it in the committed specifications and say what it is.
   - Any form you allow must be explicit in the IR. If none exists, every such reference is refused.
3. **Drawable parts.** `validate` refuses a device whose part the emitter has no arm for. Name the part, and say "row present, no emitter arm".
   - **The emitter is the source of truth.** Read the set of drawable parts from the same table or match the emitter dispatches on, never from a second hand-kept list.
   - If that set cannot be read without moving code out of `aethelgard-eda` into `aethelgard-ir`, stop and say which lines (`INV-01`: shared types live only in `aethelgard-ir`).
4. **Tests,** in `crates/aethelgard-ir/tests` or the CLI's validate tests, following the existing pattern for `1207` and `1208`'s refusals:
   - for each refusal, a specification refused by name, and the same specification shown reaching its later refusal on `main`'s behaviour (record the `main` output in FEEDBACK; the test asserts the new early refusal);
   - **mutants:** each refusal's condition removed, and the test fails;
   - every file under `examples/`, `boards/*` source specifications and test fixtures still validates. The four `examples/` walkers pass.

## 2. Acceptance

- **The three refusals** fire by name, each with a mutant that its test kills.
- **Nothing committed is newly refused.** If something is, it is a stop (§7), not a fixture edit.
- **The pin-capacitance rule matches `extract`'s precondition** on every parts row, shown by the coverage test.
- **The CLAUDE.md gates pass, and CI is green.**

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-06`; the `1207` and `1208` refusal pattern.
- **Fence:**
  - `crates/aethelgard-ir/src` (validation only);
  - the CLI's `validate` error rendering, if a new error needs a JSON shape;
  - tests;
  - FEEDBACK.
  - **Not:** `extract`, the emitter's behaviour, parts rows, `examples/`, `boards/`, the twin, workflows, `ci/` or `container/`. Reading the emitter's set of drawable parts may need a read-only accessor in `aethelgard-eda`; that is in the fence if it is one function and changes no output.
- **Running beside you:**
  - **`2304b`** edits `ir/src/power.rs` (the battery-sag chain) and bumps the specification version.
  - **`4240v`** edits `ir/src/lib.rs` (`mcu.core_supply`) and also bumps the version.
  - **You bump nothing.** Merge `main` after either lands, and keep your additions in validation functions, away from their fields.
  - **`4202`** moves the U2 rows on Sentinel-02 and -03. If it lands first, its specifications are part of "nothing committed is newly refused".
- **One container of yours at a time,** under `capped-run`, `--mem 16G` for anything that runs `verify`, at `nice -n 10`.
- One PR, left unmerged. Write `~/.cache/aet-1209/pr-title-body.md`.

## 5. Adversarial pass, full

- **A refusal broader than the later one:** it rejects a specification `extract` or `emit` would have accepted. That is a new false refusal, and `INV-06` cuts both ways.
- **A refusal narrower than the later one:** something still reaches `extract` or `emit` and is refused there.
- **A second list of drawable parts** that can drift from the emitter's.
- **A load reference form** the committed specifications use that your rule silently allows or silently refuses.

## 7. Flag-and-stop conditions

- **A committed specification, example or fixture is newly refused.** Stop with the list. The architect rules whether the file or the rule is wrong.
- **`validate` and `extract` cannot agree** on pin capacitance without an `extract` change. Stop and say which.
- **Reading the drawable set needs code moved across crates.** Stop and say which lines.
