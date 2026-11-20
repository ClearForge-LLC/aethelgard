# AET-WO-1210 — A specification can declare that it draws no board, and `validate` refuses early what every drawn board would refuse late

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1210-draws-no-board`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P1 (validation). Written 2026-11-20. **Cadence:** build. **Takes specification version 20** (`4240v` took 19).

**Grounds** (`1209` D1 (ii) and its FEEDBACK §B4, #253; `4600` F15, F17; `4210` D1; roadmap §7.1):
- **`1209` built rule 2 only.** Rules 1 and 3 (`AET-WO-1209.md` §1) were deferred by ruling:
  - **rule 1:** a device on a line `extract` bounds, with no maximum pin capacitance in its part row or the specification;
  - **rule 3:** a device whose part has a row but no emitter arm.
- **Why they were deferred:** each would refuse a twin-only stand-in, `crates/aethelgard-cli/tests/fixtures/imu-baro-1v8.json` (`4210` D1), which is never drawn. A specification cannot say it draws no board, so `validate` cannot tell a stand-in from a board.
- **So today** both faults reach `extract` and `emit`, after the work, not before it.
- **`1316` (#273)** now holds the parts table and the emitter's arms together by test and names the arms (`schematic::KNOWN_PARTS`). Rule 3 reads that set.

> **What this is:**
> - one optional, additive specification field that declares a twin-only specification, which draws no board. Specification 20;
> - `validate` applying rules 1 and 3, as `1209` §1 wrote them, to every specification that does not declare it;
> - `emit`, `extract`, `fab` and `verify --fab-out` refusing, by name, a specification that declares it.
>
> It is NOT:
> - a change to what `extract` or `emit` judge on a drawn board;
> - `4600` F2 or F6, the other specification rows, which take later versions.

## 1. Build

1. **The field.** Propose its name and place in FEEDBACK before building, with one sentence on why. For example, `board.drawn: false` beside the outline bound, or a top-level `twin_only: true`.
   - It is optional. Absent means drawn, so every committed specification keeps its meaning.
   - It is version 20, by the repository's renumbering script, and a version-19 file is refused as the version policy says.
2. **Who declares it.** Inventory every committed specification and fixture, and list each one that is never drawn: never `emit`ted in a test or in CI.
   - Each of those gains the declaration, and nothing else changes.
   - Any file whose drawn status is unclear is a stop.
3. **Rules 1 and 3,** as `1209` §1 states them, read at source:
   - rule 1 from `extract`'s own condition;
   - rule 3 from `schematic::KNOWN_PARTS`. If the IR cannot read that set without moving eda code into `aethelgard-ir`, the CLI's `validate` applies it, as `1209b`'s rule does.
   - **The coverage test** (`1209` §1.1): for every parts row, `validate`'s answer equals `extract`'s precondition.
4. **The refusals** for a declared specification at `emit`, `extract`, `fab` and `verify --fab-out`, by one named kind.
5. **Tests:**
   - each rule refuses a drawn specification by name, and its early refusal is shown beside `main`'s late one;
   - the stand-in, declared, passes `validate` and is refused at `emit`;
   - **mutants:** each rule's condition removed fails a test, and ignoring the declaration fails the stand-in test.
6. **Corpus:** every committed specification validates exactly as on `main`, apart from the version line and the declarations added in step 2. A newly refused file is a stop.

## 2. Acceptance

- The field's name and place, the inventory, the rules, the tests and the mutants are shown.
- Nothing committed is newly refused, and no emitted byte moves. The bench package moves only if the version line is in it; say which.
- **The CLAUDE.md gates pass, and CI is green.** `TIER2_MIN` does not move.

## 3. Invariants and fences

- **Invariants:** `INV-01`, `INV-06`.
- **Fence:**
  - `crates/aethelgard-ir/src` (the field, the version, the rules that live in the IR);
  - `crates/aethelgard-cli/src` (`validate`, and the refusals at `emit`, `extract`, `fab`, `verify --fab-out`);
  - `crates/aethelgard-eda/src`, only to expose a read-only set that already exists;
  - committed specifications and fixtures: the version line, and the declaration where step 2 lists it;
  - the IR README's list of kinds and fields;
  - tests;
  - FEEDBACK;
  - `fab/sentinel-01-bench/package/`, byte-for-byte commit only, if the version line moves it.
  - **Not:** what `extract` or `emit` judge, `boards/`, workflows or `ci/`.
- **Running beside you:**
  - `1209c`: IR `validate`, a new refusal kind in the same enum. Whoever merges second merges `main`.
  - `2306` (#269) and `2305` (#271): the bench-package queue. If the version line moves the package, you join it after them.
  - `4405` (pincheck) and `4241` (firmware and judge): no shared file.

  Merge `main` before READY.
- **One container of yours at a time,** under `capped-run`, at `nice -n 10`.
- One PR, left unmerged. Write `~/.cache/aet-1210/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A drawn board that declares itself twin-only** to escape the early rules. What stops it from reaching a fab package?
- **A stand-in that is drawn somewhere** after all (a test, a CI job).

## 7. Flag-and-stop conditions

- **A committed specification is newly refused,** or its drawn status is unclear. Stop with the list.
- **`validate`'s rule 1 cannot match `extract`'s precondition** without changing `extract`. Stop, as `1209` §7 said.
