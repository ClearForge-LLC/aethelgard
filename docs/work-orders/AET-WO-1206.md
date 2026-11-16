# AET-WO-1206 — `validate` refuses what `generate` refuses: names, buses and one error set

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1206-validate`.
**Author:** Claude (architect) · **Builder:** strong tier, with a targeted adversarial pass.
**Phase:** backlog (roadmap §7.1), written 2026-11-16 at Scotty's direction: clear the backlog before new roadmap items. **Cadence: build, with one measured stop (§1.1).**

**Grounds (§7.1 rows, each with its source):**
- **`4100` F1:** `validate` accepts a specification that `solve` and `emit` refuse for a bus with no net constraint (`MissingBusConstraint`).
- **`1102` A2, A4 and A5** (`AET-WO-1102.FEEDBACK.md`): `validate` passes three kinds of name that `generate` refuses before any output:
  - a reference equal, case folded, to a Renode stock instance or a solved peripheral (`i2c1`, `spi1`, `gpioa`), refused later as `InvalidDeviceName`;
  - a reference differing only in case from one the emitter chose (`u1` against `U1`), refused later as `DuplicateReference`;
  - a net or domain named `GND`, one starting `unconnected-`, or an emitter-named net (`U1_VCAP1`, `U4_WP`), refused later as `ReservedName`.
- **`1102` A7:** a project name that is a Windows reserved device name (`con`, `prn`, `aux`, `nul`, `com1`–`com9`, `lpt1`–`lpt9`) writes files Windows cannot open.
- **`1205` F1:** `IrValidationError` and `SolveError` both carry `BusTooFast`, `PullUpBelowSinkFloor` and `BusCapacitanceOverLimit`; keep one set.
- **`4300c` reduction:** three refusals are now pre-empted by `validate`: eda `UnsupportedAddress`, twin `ModelOffItsBus`, and emitter `InvalidDeviceName` for `spi1`–`spi6`. Delete each, or keep it as a guard.

> **What this is:** `INV-06` at the earliest stage. Every refusal above moves to `validate`, by the same name and JSON shape the later stage uses today, so a caller learns of it before any file is written.
>
> It is NOT:
> - the closed vocabularies (`UnknownPin`, `UnknownPart`), which stay with the tables (`1102` A9);
> - a change to any emitted byte;
> - a specification version bump. Refusing more is not a schema change.

## 1. Build

1. **Measure first (the stop).** List every reference, net, domain and project name in every example, fixture and scenario in the repository, and run each candidate rule over them. **STOP** with the list if any rule would refuse an existing specification that `generate` accepts today. A false refusal is the risk this work order must not take.
2. **`MissingBusConstraint` at `validate`.** Use the same check as `solve`, or an equivalent one, by the same name. If it shares code with `solve`, say so; that is acceptable here, because both refuse.
3. **The names (A2, A4, A5).**
   - **A2:** refuse a reference that case-folds to any peripheral instance in the MCU's closed peripheral table or to Renode's stock instance names. This is static, so it needs no solve; name the table it reads.
   - **A4:** refuse a reference that case-folds to a reference the emitter reserves. Read the emitter's naming rule from source and state it; if the reserved set depends on the solve, refuse the whole pattern only if §1.1 shows that no existing specification uses it.
   - **A5:** refuse the reserved nets by name, with the emitter's reserved list moved to one place both stages read.
4. **Windows names (A7).** Refuse a project name whose stem, before the first `.`, case-folds to a reserved device name. Apply the same rule to any other identifier that becomes a file name; list them.
5. **One error set (`1205` F1).** `SolveError` wraps or reuses the validation variants. The JSON each refusal prints must stay byte-identical; pin that with a test per variant.
6. **The three pre-empted refusals.** For each, find whether any entry point can still reach it without `validate`: a hand-written `SolvedTopology` read by `generate`, or a library caller. Keep it, with a test that reaches it, if reachable; delete it if not. Record which, and why.

## 2. Acceptance

- Every new refusal has a tier-1 test, and each shows the old behaviour first: `validate` passes, `generate` refuses.
- No existing example, fixture or scenario changes its verdict.
- Every emitted file is byte-identical.
- **The CLAUDE.md gates pass.** CI is green, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants:** `INV-01`, `INV-06`; `RUL-16`.
- **Fence:**
  - `aethelgard-ir`: `lib.rs`'s validation, `solve.rs`'s error set;
  - the emitter's reserved-name list (moved, not changed);
  - the three guards in eda and twin;
  - tests; FEEDBACK.

  `AET-WO-4300g` (PR #159) and `AET-WO-4230` (PR #161) both add specification fields in `aethelgard-ir`. Merge `main` as each lands, keep your hunks to the validation functions, and take no version number.
- One PR, left unmerged. Write `~/.cache/aet-1206/pr-title-body.md`.

## 5. Adversarial pass, targeted

- Names that defeat the folding: Unicode case (`İ`, `ſ`), trailing dots, `CON.` and `con .txt`, and mixed-case reserved nets.
- A hand-written topology that reaches a deleted refusal's code path.

## 7. Flag-and-stop conditions

- **§1.1 finds an existing specification a new rule would refuse.**
- **The emitter's reserved references depend on the solve** in a way `validate` cannot know without solving.
- **Keeping the JSON byte-identical would need two error types to stay apart.**
