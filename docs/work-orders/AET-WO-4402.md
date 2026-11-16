# AET-WO-4402 — Housekeeping: the ArduPilot pin from its own publish, the pin's regex tested, and the merged spikes deleted

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4402-housekeeping`.
**Author:** Claude (architect) · **Builder:** cheap tier, with a targeted adversarial pass on the pin check only.
**Phase:** backlog (roadmap §7.1), written 2026-11-16 at Scotty's direction: clear the backlog before new roadmap items. **Cadence: build only.** Every item is fully ruled here.

**Grounds (§7.1 rows, each with its source):**
- **`4401` Q1** (`AET-WO-4401.FEEDBACK.md`): `container/ARDUPILOT-IMAGE` names the image `main` published from `8030fa5`. `4401` then changed the Dockerfile (pip pinned through `python3-pip-whl`), so the pinned image is not the one the Dockerfile builds.
  - `main` published the new image from `4419c59`: run 43847208884, job `publish` (133234319639), pushed `ghcr.io/clearforge-llc/aethelgard-ardupilot@sha256:a4818a0472b8c1ed675b7a39e35a558fb4001fe2511b8abb6e02501214749d18`. The architect read this from the job log on 2026-11-16.
  - `container/` is unchanged between `4419c59` and `80f5dac` (`git diff --stat` is empty).
- **`4401` F5:** no committed test exercises the `pinned` job's pin regex. It lives inline in `.github/workflows/ardupilot-image.yml`.
- **Grok review 2026-11-16, row 16; roadmap §6 "a spike merged as the record is deleted by its build":**
  - `spike/4210`, `spike/4220`, `spike/4300d` and `spike/4300f` outlived their merged builds;
  - `crates/aethelgard-twin/src/tests.rs` (two comments) names `spike/2100`, which no longer exists;
  - four `crates/aethelgard-pincheck/fixtures/*.hwdef.dat` headers name `spike/4400`, which no longer exists.

> **What this is:** three small, separate fixes in one PR. Each has a test or a measurement that shows it.
>
> It is NOT:
> - `spike/1303` (its build is deferred; it stays);
> - any change to the ArduPilot Dockerfile, its tag, `check-pins.sh`'s refusals, or any crate's behaviour;
> - `4401` Q2 and Q3 (the `# syntax=` directive; unpinned apt dependencies), which wait for the next container work order.

## 1. Build

1. **The pin.**
   - Write the digest above into `container/ARDUPILOT-IMAGE`, in the file's existing one-line format.
   - Before writing it, confirm on Furnace (through `~/bin/capped-run`, or with `podman pull` and `podman image inspect`) that the digest pulls, and that its `org.opencontainers.image.revision` label is `4419c590e63e46e106113d5ce9e82c231d176e8e`. Record both outputs in FEEDBACK.
   - Confirm `git diff 4419c59 <base> -- container/` is empty at your base. If it is not, **STOP**: a later Dockerfile change needs a later publish.
2. **The pin's regex, tested (F5).**
   - Move the `read the pin` step's check into a script, `container/check-ardupilot-pin.sh`. The script takes the file path as `$1`, prints the pin on success, and prints today's message to stderr and exits 1 on failure. The workflow step calls it, and the regex lives only there.
   - Add `container/test-check-ardupilot-pin.sh`. Model it on `test-check-pins.sh`, with fixtures under `container/fixtures/ardupilot-pin/`.
     - **Accepted:** the committed pin; the committed pin with one trailing newline. Today's `$(cat …)` strips it, so keep that behaviour and say so.
     - **Refused:** each fixture is refused by name:
       - an uppercase hex digit;
       - 63 or 65 hex digits;
       - a tag (`:latest`) instead of a digest;
       - a digest and a tag together;
       - another registry or repository;
       - `sha512:`;
       - two lines;
       - a leading or trailing space;
       - an empty file;
       - a missing file;
       - a CRLF line ending.
   - Run the new test in the `image` job, beside `check-pins against its fixtures`.
3. **The spikes.**
   - Delete `spike/4210`, `spike/4220`, `spike/4300d` and `spike/4300f`.
   - Before deleting, show by `grep -rn` across the tree (excluding `docs/work-orders/` and `docs/reviews/`, which cite them as history) that no code, test, workflow or script reads them. Record the command and its output.
   - `.dockerignore`'s `spike/` line stays, because `spike/1303` remains.
4. **The stale comments.**
   - **`twin/src/tests.rs`:** each comment that names `spike/2100/out/...` instead cites where the measurement is recorded now. That is `AET-WO-2100.FEEDBACK.md` and its section, or the PR (#56) and commit `459005f`. Find which, and cite it exactly.
   - **The four `pincheck` fixture headers:** name the specification by its sha256 alone, or by the file that holds it on `main` now, if one does. Drop the dead path.
   - First find whether any test, digest or golden reads those header bytes (the fixtures' sha256, a byte comparison, a line count). If one does, change the header only if the dependent value moves with it in the same commit, and list each in FEEDBACK. If a pinned value would move for another reason, **STOP**.

## 2. Acceptance

- `ardupilot-image` is green on the PR, `pinned` included. Its `ANSWER pinned revision=` line names `4419c59…`.
- `test-check-ardupilot-pin.sh` passes, and each refused fixture fails by name. A mutation of the regex shows it: drop the `$` anchor, or widen `[0-9a-f]` to `[0-9a-fA-F]`. Each mutation must turn at least one fixture's verdict, and is reverted.
- The four spike directories are gone, and nothing outside `docs/work-orders/` and `docs/reviews/` names them or `spike/2100` or `spike/4400`.
- **The CLAUDE.md gates pass.** CI is green on the balanced plan, and `TIER2_MIN` is unchanged.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-06` (the pin check refuses by name), `INV-07`; `RUL-15`; roadmap §6 (spike deletion).
- **Fence:**
  - `container/ARDUPILOT-IMAGE`, the two new `container/` scripts and their fixtures;
  - `.github/workflows/ardupilot-image.yml`: the `read the pin` step, and one new step in the `image` job;
  - `spike/4210`, `spike/4220`, `spike/4300d`, `spike/4300f` (deleted);
  - comment lines only in `crates/aethelgard-twin/src/tests.rs` and the four `pincheck` fixtures;
  - FEEDBACK.
- **Running builders:**
  - Do not touch `ci.yml` (`AET-WO-4230`).
  - Leave `aethelgard-eda` alone (`AET-WO-1311`, PR #168).
  - If `4230` merges first and conflicts in `twin/src/tests.rs`, merge `main` with a merge commit and keep both sides.
- No version number of any kind.
- One PR, left unmerged. Write `~/.cache/aet-4402/pr-title-body.md`.

## 5. Adversarial pass, targeted

On the pin check only. Try to get a pin that does not name exactly one `ghcr.io/clearforge-llc/aethelgard-ardupilot` digest past `check-ardupilot-pin.sh`:
- Unicode look-alike characters;
- an embedded NUL;
- a very long line;
- a symlink in place of the file;
- a pin that is valid on its first line, followed by a second.

Add a fixture for each one that reaches the regex.

## 7. Flag-and-stop conditions

- **The digest does not pull, or its revision label is not `4419c59`.**
- **`container/` changed after `4419c59` at your base.**
- **Something reads a spike directory**, or a pinned value would move for a reason other than the comment change.
