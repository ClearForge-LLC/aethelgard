# AET-WO-1610 — The CLI's backlog: a gate that cannot run is not "off", three checks that skip in silence, and `init`'s refusals by kind

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1610-cli-backlog`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4, backlog. Written 2026-11-24. **Cadence:** a measurement, then a build with one stop. **After `4410`** (both touch `validate`'s refusals and the topology's `spi_devices`).

**Grounds** (roadmap §7.1, from `main`):
- **`1609` R6** (FEEDBACK §5 item 6, §6): `prove` with no harness reads `off` in the "ran" table, though the run fails `no_harness`. `off` means a key or a flag turned the gate off, by the table's own definition (`1609` (1)). A missing harness is no one's choice, so the table and the run disagree. Review 6 makes the same point under `INV-03` (cite it; do not restate it). **Ruled now: the label becomes `not_reached: no_harness`.** The ruling at `1609` kept the change inside that work order's scope; it was not a judgement that `off` is right.
- **`1609` R5:** `judged_nothing` can be derived from `ran`, and only three tests read it.
- **`1609` F3:** `spi_devices[].orientation`, `board.max_outline_um` and `power.loads[].reference` each skip a `validate` or emit-time check in silence, outside any verdict.
- **`1608` F1:** a well-formed file is refused as `InvalidJson` (exit 3) when an externally tagged enum object is left `{}`. Eight corpus mutants show it.
- **`1608` F2:** `UnwitnessedAcknowledgementStale` could end with the ledger's own fields for that register and class (`cli/src/ledger.rs`).

> **What this is:**
> - **(1) R6:** `prove` with no harness reads `not_reached: no_harness`. Every reader of the table (MCP, CI scripts, the goldens) moves with it, found by search. The verdict format bumps only if the repo's versioning says a label value needs it; say which.
> - **(2) R5:** remove `judged_nothing`. Each of its three tests reads `ran` instead, and each keeps a mutant it kills. If a reader outside the tests needs the field, stop.
> - **(3) F3, one at a time.** For each of the three fields, measure what the check would judge and why it skips. Then either:
>   - make the check run and refuse by name when it fails; or
>   - list the field in the "ran" table as `not_reached: <reason>`, so the skip is no longer silent.
>
>   The first is preferred where the data to judge exists. Say which and why for each.
> - **(4) F1:** an enum object left `{}` is refused by its own kind and path, as a missing variant, not `InvalidJson`. The eight mutants become tests, each naming the kind.
> - **(5) F2:** `UnwitnessedAcknowledgementStale`'s text ends with the ledger's fields for that register and class. The text is built from the ledger's parsed values, never from text a specification supplies, so a planted string cannot reach the message.
> - **Each refusal comes with a test, and each test with a mutant it kills.**
>
> It is NOT:
> - `1608` F3–F5 (`init`'s partial file on `ENOSPC`, `LifecycleStatementInvalid`'s `path`, a second template): rows, kept until a second template is wanted;
> - the foreign judge (`1609` R1–R4 belong to the foreign-judge work order);
> - a change to what any gate judges beyond (3).

## 1. Build

1. **Measure first.** Commit predictions in `spike/1610/`, then record:
   - every reader of `off`, `not_reached` and `judged_nothing`;
   - for each F3 field, the check it skips, where it skips, and which committed specifications carry the field;
   - the eight F1 mutants with today's refusal.
2. **Stop** with the measurement and, for each F3 field, the choice between a check and a `not_reached` row.
3. **Build (1)–(5)** to the ruling. Merge `main` before READY.

## 2. Acceptance

- **`prove` with no harness** reads `not_reached: no_harness`, and the run still fails `no_harness`.
- **`judged_nothing` is gone;** its three tests read `ran`.
- **Each F3 field** is checked and refuses by name, or appears in the table as `not_reached`.
- **The eight F1 mutants** refuse by kind and path.
- **`UnwitnessedAcknowledgementStale`** quotes the ledger's fields.
- **The CLAUDE.md gates pass, and CI is green.**

## 3. Fences

- **Fence:**
  - `crates/aethelgard-cli` (the verdict, `validate`, `ledger.rs`, the MCP surface), and its tests and goldens;
  - `crates/aethelgard-ir` only for (4)'s refusal kind;
  - `ci/` only where a script reads the changed labels;
  - `spike/1610/`, `docs/work-orders/AET-WO-1610.*`.
- **Not:** `crates/aethelgard-eda`, `-hwdef`, `-pincheck`, `-twin`; `fab/`; the steering documents.
- One PR, left unmerged. Write `~/.cache/aet-1610/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A gate that still reads `off`** when nothing turned it off.
- **A planted specification string** that reaches F2's message.
- **An F3 field** that a specification can still set to skip its check in silence.

## 7. Flag-and-stop conditions

- **After §1.1,** with the measurement.
- **A reader outside the tests** depends on `judged_nothing` or on `off` for a missing harness.
- **An F3 check would refuse a committed specification.** Stop, with the specification and the check.
