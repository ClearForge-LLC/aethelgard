# AET-WO-1900 — Evidence gaps: prove the checks ran, and close the review's small holes

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1900-evidence`.
**Author:** Claude (architect) · **Builder:** a cheap builder with a targeted adversarial pass (`docs/roadmap.md` §6).
**Phase:** P1 refinement, from the Mythos 5.2 review of 2026-11-12. It covers the review's A5, A6, A7, A10.1, A11.1, A11.2, A11.3, A12.2, B5 and B8, and the `AET-WO-1900` rows of `roadmap.md` §7.1.

**Grounds:**
- The project's best property is knowing whether a claim was checked (`docs/reviews/2026-11-12-mythos-5-2.md`, Part C 3). The review found places where a green CI says less than it seems to.
- Every item below is small. Each is either reproduced by the review or marked by it as unverified. The unverified ones are checked first, and fixed only if real.

> **What this is:** a set of small, independent fixes, one commit each, every one with a test that fails before and passes after.
>
> It is NOT:
> - a change to what any gate passes or fails on a valid input, except where this work order names it;
> - the board-binding work of `AET-WO-3301`.

**Cadence:** build-only. Stop only on §7. One PR, left unmerged.

## 1. Scope

1. **A tier-2 canary (A7.3).** The CI `tools` job fails when fewer tier-2 tests ran than a pinned count. Pin the count from the measured run, and have the test print it, so a dropped `with-tools` flag or a skipped suite turns the job red.
2. **The missing-key test removes every key of every object (A7.2).** Reproduce the review's surviving mutant (`#[serde(default)]` on `spi_devices`) first, show it now fails the test, then revert the mutant.
3. **The quickstart against `main`'s own image (A6, 1700 F1).**
   - On every push to `main`, after `engine-image` publishes, run the README's quickstart block against that run's image digest.
   - The engine image carries `org.opencontainers.image.revision`, and the run asserts it equals the commit.
   - The README's pinned digest stays the user-facing one. `roadmap.md` §6 already says to bump it at each phase close.
4. **A README status test (B8).** It checks that the README's status line names, as complete, exactly the phases `roadmap.md`'s §5 marks with "Exit gate: passed".
5. **The DRC rules label (A5, 1300 B1).** `rules: "specification"` becomes `rules: "engine-floor"`, everywhere it is printed and documented, until a specification rules field exists (`AET-WO-4100`).
6. **Case-colliding labels refused at `validate` (A10.1).** Two nets, or a net and a power name, that differ only in case are refused, as reference names already are. Use the review's two inputs: `gnd`, and `i2c_scl` beside `I2C_SCL`.
7. **The archive manifest read is capped (A11.2):** `take(1 MiB)` before `read_to_end`, with a test that a 2 MiB manifest is refused by name.
8. **State is bounded at fold time (A11.3), if small.** Refuse a second scenario, topology, boot or verdict record while folding. If it is not small, record why in FEEDBACK and leave it for `2301`.
9. **Relative `PATH` entries are refused by `Tool::find` (A12.2).** Absolute entries keep working.
10. **The I2C witness, measured (A11.1).**
    - In the tools image, write the reference's final `CR2` and `TXDR` values directly from a hand-written firmware that calls no transfer, and see whether the judge passes it.
    - **If it does:** record the measurement, and add one observable a plain write cannot set, provided it fits this work order's size (for example `ISR`'s state after a real transfer). Otherwise state in the CLI README that the witness guards a mistaken generator, not a hostile one, and leave the observable to `2301`.
    - **If it does not:** record why, and keep the test.
11. **The review's unverified test notes (A7.4–A7.7, the A10 notes).** Check each at source, fix what is real and small, and record the rest in FEEDBACK with a reason.
12. **The small carried items in §7.1 owned by this work order:**
    - 1300 D10 (KiCad's licence text in the image);
    - 1302 B9 (`opt-level` for `aethelgard-eda` in the dev profile);
    - 2200 F17 (the duplicated `decimal`);
    - 2300 F13 (the held-low `.repl` comment says "measured");
    - 2300 F14 (a `FirmwareChanged` test, if it can be reached from outside the sandbox; otherwise say why).

    Triage 1300 A6 and A8, 1301 A5, 2100 F23–F25, and 2200 F11 and F14: fix each if small, or name its owner in FEEDBACK so the architect can update §7.1.

## 2. Invariants and fences

- `INV-03`, `INV-06`, `RUL-12` (no Python in CI), `RUL-13`. No `unsafe`.
- **Fence:**
  - `.github/workflows/`;
  - `container/` (the label and the licence text only);
  - the CLI's `check drc` label and `Tool::find`;
  - `aethelgard-ir`'s case rule and its tests;
  - `aethelgard-chaos`'s manifest read and fold;
  - the twin's held-low comment;
  - `Cargo.toml` profiles;
  - tests;
  - the READMEs' sentences these change;
  - this work order's FEEDBACK.
- **Every committed generated file stays byte-identical.** The one exception is the `.repl` comment of 2300 F13, if the twin emits it; regenerate it with the tools and record the old and new bytes.
- **`AET-WO-3200` may run in parallel** in `aethelgard-evolve` and the CLI's `propose`. Do not touch those files. If it lands first, merge `main`.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance

- Each item in §1 has a test that fails before its fix and passes after. FEEDBACK lists the pairs.
- CI is green, with the canary's count printed in the `tools` job log.
- §1.10's measurement is recorded with the exact firmware and output.
- §1.4 passes in `gates` and `tools`.

## 5. Adversarial pass — targeted

1. Make the `tools` job pass with tier 2 skipped.
2. Make the quickstart run test an image other than this commit's.
3. Find a case collision that `validate` still accepts.
4. Make a run directory allocate more than the cap.

## 7. Flag-and-stop conditions

- A fix changes what a gate passes on a valid input, beyond what §1 names.
- The witness measurement shows a forgery passes and no observable fits this work order's size. Record it and leave the observable to `2301`; this is not a stop, but say so in the PR.
- A test that can only go green by being weakened.
