# AET-WO-4408 — The ArduPilot matrix grows: a four-port generated board, ST's pin table checked against ArduPilot's, the image re-pins, and `4407`'s follow-ups

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4408-matrix`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4, backlog wave 2. Written 2026-11-23. **Cadence:** a measurement, then a build. **After `4407`** (#339) and **`4300i`** (#338, specification 28).

**Grounds** (roadmap §7.1; triage W2-MATRIX):
1. **`4404` (draft).** No generated board in CI has more than a console, so nothing generated exercises SERIAL1–9. Add one four-port generated board to `ardupilot-image.yml`'s matrix, as a fixture at specification 28.
2. **The Sentinel-01 review F7 (2026-11-19).** `verify`'s gates take each AF number and pin position from one source, ST's pin table. Add a test that checks it against ArduPilot's `STM32H753xx.py`, and a hand-entered DS12117 pad table. Put that data in a test fixture, not in `svd-hal/src`.
3. **`4407` Q1, Q3, Q4, Q5:**
   - `ardupilot-image.yml`'s `paths:` miss `container/debian-trixie.sources`;
   - re-pin `container/IMAGE` and `ARDUPILOT-IMAGE` to the images `main` published after #339;
   - seed the GtkSharp rehearsal from the published mirror;
   - `test-check-pins.sh` and `fixtures/README.md` were edited beside `4407`'s fence: confirm them and record it.

> **What this is:** (1)–(3), each with a check that fails without it. **The re-pin moves every builder's tools image:** state the digest change, and run the bench package's verify against it. If a `fab/` byte moves, stop.
>
> It is NOT:
> - a tool version change beyond what `4407`'s snapshot already fixed;
> - any crate `src/` change.

## 1. Build

1. **Measure first,** with predictions in `spike/4408/`:
   - the four-port board's generated hwdef;
   - the pin-table diff;
   - the published digests after #339.
2. **(1)–(3).**
3. **The bench package's verify** under the re-pinned image.

## 2. Acceptance

- **The matrix's four-port board** builds ArduPilot and passes `check pinout`.
- **The pin-table test fails on a planted mismatch.**
- **The image pins name `main`'s published digests,** and the bench package verifies identically under them.
- **The CLAUDE.md gates pass, and CI is green.**

## 3. Fences

- **Fence:**
  - `.github/workflows/ardupilot-image.yml` (the matrix and `paths:`);
  - `container/` (`IMAGE`, `ARDUPILOT-IMAGE`, the GtkSharp rehearsal);
  - one new fixture specification;
  - a new test file with its data;
  - `spike/4408/` and FEEDBACK.
- **Not:** any crate `src/`, specification other than the new fixture, or `ci/` except weight rows.
- **Running beside you:** `2310`, `1916`, `1322`.
- **The S25 window:** if it is closed, build and commit locally, and post PUSH-READY on the bus. Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-4408/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A pin-table row** that both sources agree on, but that DS12117 contradicts.
- **A re-pin** that a cached layer hides.

## 7. Flag-and-stop conditions

- **A `fab/` byte moves under the re-pinned image.** Stop.
- **`main`'s publish has not produced a digest yet.** Stop and say so.
