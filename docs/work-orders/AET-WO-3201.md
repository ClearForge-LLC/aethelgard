# AET-WO-3201 — Evolve, replay and the judge: closing the attribution backlog

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-3201-evolve`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass (attribution is where a bug becomes a confident wrong answer, roadmap §3).
**Phase:** backlog (roadmap §7.1), written 2026-11-16 at Scotty's direction: clear the backlog before new roadmap items. **Cadence: build, with one measured stop (§1.1).**

**Grounds (§7.1 rows, each with its source):**
- **`2205` adversarial B1–B3** (`AET-WO-2205.FEEDBACK.md`):
  - B1: `check_joins` does not require a kind 7 or 8 for each kind-15 line, so dropping every rise replays as `pass`, `N1`;
  - B2: a boot that read nothing, with no `boot_word_missing`, replays as `pass`, and `attribute` says `I1`;
  - B3: a finding's severity is open text, so `zzz` counts as failing.
- **`2205` pass 2 (M1):** an I3 seeded trap on a second shape (`cap-390p-sda-2k2` × `leave-firmware`), which would separate evidence mutants the one-bus set cannot.
- **`2205` §1.6, `3100` F15:** a missing boot word suppresses the I2C findings, so an identity read's timeout record goes unnamed (`no-boot-word` on 47 kΩ, `I1`).
- **`4300a` Q7:** `identity_read_outside_window` has no evolve rule, so `attribute` answers `I1`. Proposed: an indeterminate rule, decided by the unset record.
- **`4210` F-B5:** evolve's S1/S2 report text truncates a 6-byte value to 32 bits. Selection is unaffected.
- **`4210` F-B10:** a data-read mismatch is reported as `{bus}_identity_mismatch`. A kind of its own needs evolve's rule table.
- **`3301` R2:** `propose::write_out` is a third copy of `tree::write`'s write discipline.
- **`1205` review:** `propose` reads the declared speed from the specification, where `i2c_speeds` now carries it in the topology (`RUL-16`).

> **What this is:** replay refuses what it cannot bind, the judge names what it saw, and evolve has a rule for every finding kind the judge emits. Every seeded case keeps its manifest's answer.
>
> It is NOT:
> - a new trace version. If one is needed, that is the stop;
> - new fault kinds (the battery-sag / chaos work order owns them);
> - the committed-proposals question (`1909` F2, the architect's).

## 1. Build

1. **Find where each lives and what it costs (the stop).**
   - **Severity (B3):** is it archived in the CBOR (a trace format matter) or only in the verdict JSON?
   - **B1/B2:** does requiring the joins change any committed replay hash or seeded answer?
   - **F-B10:** does a new finding kind need a new trace kind, or only a new name in the judge and evolve?

   Commit predictions first. **STOP** with the answers if any item needs a trace-version bump. `AET-WO-4230` takes trace 12, and two builders must not both bump.
2. **Replay (B1, B2).**
   - Each kind-15 line requires its kind 7 or 8.
   - A boot with no read and no `boot_word_missing` is refused by name, not passed.
   - Committed hashes stay valid; any that change are named with the reason.
3. **Severity (B3):** a closed set, refused when unknown. In the archive, enforce it at read with no byte change.
4. **The judge (F15):** with no boot word, still name each identity read's timeout record. The `no-boot-word` finding stays.
5. **Evolve's rules.**
   - `identity_read_outside_window` gets a rule: indeterminate, decided by the unset record. Add a seeded case.
   - F-B10's data mismatch gets a kind of its own, with its rule.
   - F-B5: the report text prints the full 64-bit value.
6. **The I3 trap (M1).** Add the second shape to the seeded set with its manifest answer, and show which evidence mutant only it kills.
7. **`propose` (R2, `1205` review).**
   - `write_out` uses `tree::write`.
   - The declared speed comes from the topology's `i2c_speeds`.
   - Proposals stay byte-identical on the 47 kΩ case.

## 2. Acceptance

- Every seeded case keeps its manifest's answer.
- The new cases attribute as their manifests say.
- Each B item has a forged run that failed before and is refused or named after.
- The 47 kΩ proposal is byte-identical.
- **The CLAUDE.md gates pass.** CI is green on the balanced plan, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants:** `INV-05`, `INV-06`; `RUL-07`, `RUL-18`, `RUL-19`.
- **Fence:**
  - `aethelgard-evolve`;
  - `aethelgard-chaos`: `replay.rs` and the seeded set;
  - `aethelgard-cli`: `propose.rs`, `tree.rs`, and the judge's findings in `renode.rs` (only the no-boot-word path);
  - tests; FEEDBACK.

  `AET-WO-4230` (PR #161) is changing `renode.rs` and chaos's serial replay. Read its branch first and keep your hunks separate.
- One PR, left unmerged. Write `~/.cache/aet-3201/pr-title-body.md`.

## 5. Adversarial pass, full

A fresh, read-only subagent forges runs against the finished replay and rules:
- a kind-15 line with a kind 7 from another bus;
- a severity in another case or with whitespace;
- a boot word written after the window;
- a data mismatch dressed as an identity mismatch.

Every finding is fixed or named.

## 7. Flag-and-stop conditions

- **§1.1: any item needs a trace-version bump.**
- **A seeded case's answer would change.**
- **A committed replay hash changes for a reason other than a refusal this work order adds.**

## 8. Amendments

**2026-11-16, from the build (PR #167, `AET-WO-3201.FEEDBACK.md`), accepted at merge.** §1.2 as written would have refused honest runs, so the builder built these two rules instead and named them before building:
- **B1, a held line.** A held line has no kind 7 or 8 by design: the analog pass leaves it out of the decks, and its fault is a kind 9 `from: [held_low]` naming the net. Each kind-15 line has exactly one of a kind 7, a kind 8, or a held-low kind 9 naming that net.
- **B2, an unread boot.** A boot killed at a bound is honestly unread, and its verdict names the bound (`cpu_aborted`, `wall_bound_exceeded` or `memory_cap_exceeded`), never `boot_word_missing`.
  - A boot that read nothing must name the bound it was killed at.
  - A boot that was read names `boot_word_missing` exactly once when its word is not the expected one, and never otherwise.
- **Left open:** a booted run whose registers and records are all set to unread still replays as `N1`. The all-read-or-none rule across kinds 10–13, 21 and 22 is a roadmap §7.1 row.
