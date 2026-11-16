# AET-WO-4401 — The ArduPilot image: its pin, and the static check's blind spots

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4401-image-pins`.
**Author:** Claude (architect) · **Builder:** strong tier, with a targeted adversarial pass.
**Phase:** backlog (roadmap §7.1), written 2026-11-16 at Scotty's direction: clear the backlog before new roadmap items. **Cadence: build only.**

**Grounds (§7.1 rows, each with its source):**
- **`4400a` D-4** (`AET-WO-4400a.FEEDBACK.md`): `container/ARDUPILOT-IMAGE` does not exist until `main` publishes. A follow-up PR sets it from the publish summary, as `RUL-15` says.
  - `main` has since published. Run 43805854193, job `publish`, on `8030fa5`, pushed `ghcr.io/clearforge-llc/aethelgard-ardupilot@sha256:a6d8a4e5cb57eb2a306e8e303ff835c66733b006b7e29caf588c37bddcd1383a` (read by the architect from the job log, 2026-11-16).
- **`4400a` A7** (FEEDBACK §A7, declared in `check-pins.sh`'s header):
  - `git submodule update --remote`, `git fetch`/`pull` and `pipx` pass the static check;
  - the venv's pip comes from Debian's `python3-pip-whl`, an apt dependency whose version is not pinned.

> **What this is:** the ArduPilot image is pinned the way `container/IMAGE` pins the tools image. The static check refuses the four forms it lets through today, and pip is pinned by version and hash.
>
> It is NOT:
> - `4400a` D-2, mirroring Arm's GCC source (Scotty ruled it to `AET-WO-4400b`);
> - a change to ArduPilot's tag, the build, or any board.

## 1. Build

1. **The pin.**
   - Write `container/ARDUPILOT-IMAGE` with the digest above, in `container/IMAGE`'s format.
   - Make every consumer that should run the published image read it: the CI jobs that pull rather than build, and the README if it names the image. List each consumer, and leave the ones that build from the Dockerfile on a PR building.
   - Add the same drift refusal `container/IMAGE` has, if `check-pins.sh` has one for it.
2. **The check's blind spots.** `check-pins.sh` refuses, each by name:
   - `git submodule update --remote`;
   - `git fetch` or `git pull` after the pinned checkout;
   - `pipx`;
   - a `pip install` without `--require-hashes`.

   Each refusal gets a fixture under `container/fixtures/` that fails before and passes after. Today's Dockerfile must still pass; if it uses one of the forms, change it to a pinned form, or stop.
3. **pip itself.** Pin the venv's pip by version and sha256 (a hashed `pip` entry in the requirements the image already hashes, or the apt package by version). The image's smoke and the step-G build must still pass under `--network none` in CI.

## 2. Acceptance

- `ardupilot-image` is green on the PR, smoke and step G included.
- The four refusals each have a fixture.
- `check-pins.sh`'s header no longer lists A7 as a blind spot, or says what is left of it.
- **The CLAUDE.md gates pass.** CI is green.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-07`; `RUL-15`; roadmap §6.
- **Fence:** `container/` (the ArduPilot Dockerfile, `check-pins.sh`, fixtures, `ARDUPILOT-IMAGE`), `.github/workflows/ardupilot-image.yml`, FEEDBACK. No crate changes.
  - `AET-WO-4230` changes `.github/workflows/ci.yml` (its timeout); do not touch `ci.yml`.
- No version number of any kind.
- One PR, left unmerged. Write `~/.cache/aet-4401/pr-title-body.md`.

## 5. Adversarial pass, targeted

Try to get each refused form past the check:
- through a line continuation;
- through an `ARG`-built command;
- through a `RUN` heredoc;
- through a script the Dockerfile copies in and runs.

## 7. Flag-and-stop conditions

- **The published digest does not pull or does not smoke** on Furnace or CI.
- **Pinning pip needs a network fetch at build time** that the image's `--network none` build cannot make.
