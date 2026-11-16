# AET-WO-4403 — Corresponding source for the public images: Arm's GCC mirrored, and a written offer for every GPL component

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4403-gpl-source`.
**Author:** Claude (architect) · **Builder:** strong tier, with a targeted adversarial pass.
**Phase:** compliance. It is carved out of `AET-WO-4400b` (Scotty, 2026-11-16), because `4400b` is held until the first scar and this obligation is live now. **Cadence: build, with one measured stop (§1.1).**

**Grounds:**
- **Both images are public.** On 2026-11-16 the architect fetched an anonymous pull token from `ghcr.io`, and `tags/list` returned HTTP 200 for `clearforge-llc/aethelgard-ardupilot` and `clearforge-llc/aethelgard-tools`. `aethelgard-engine` returned 403. Every push to `main` publishes the first two.
- **GPLv3 §6** applies when object code is conveyed. If we offer it by network access, §6(d) requires us to offer the Corresponding Source "through the same place", or on a third party's server with clear directions next to it. We must also keep that source available for as long as we offer the object code.
- **`4400a` D-2, ruled by Scotty on 2026-11-15:** mirror the exact Arm GCC corresponding-source tarball by sha256. Do not rely on Arm's host (roadmap §7.1).
- **`container/ARDUPILOT-LICENSE.md`** already covers ArduPilot itself: the image carries ArduPilot's complete source at `Copter-4.7.2`, with `.git`. It names Arm's release as the toolchain's source but mirrors nothing.
- **`container/KICAD-LICENSE.md`** covers only the KiCad libraries (CC-BY-SA 4.0). The tools image also carries KiCad 10.0.7 itself (GPL-3.0-or-later) and Debian packages under the GPL, and says nothing of their source.

> **What this is:** for each public image, a written source offer next to the image, and a mirror for every GPL component whose source we do not already ship in the image.
>
> It is NOT:
> - legal advice, or a licence audit of every Debian package. Debian's source archive by exact version is the stated route for those, with `snapshot.debian.org` as the fallback;
> - a change to any crate, any emitted byte, or what the images contain, beyond a notice file;
> - the rest of `4400b`.

## 1. Build

1. **Measure first (the stop).**
   - List every GPL component in each public image: the package name, its exact version, and where its source lives today.
   - Find the Arm source tarball that corresponds to `gcc-arm-none-eabi-10-2020-q4-major`. Give its URL on Arm's site, its size and its sha256, and confirm that the toolchain's own docs name it.
   - Choose where the mirror lives, and say why. Two candidates:
     - **(a)** an OCI artifact on ghcr, next to the image (`ghcr.io/clearforge-llc/aethelgard-ardupilot-source@sha256:…`), pushed by the workflow that publishes the image;
     - **(b)** a GitHub release asset on this repository.

     Measure what each costs: size, whether the workflow's `packages: write` permission covers it, and whether an anonymous pull or download works.
   - **STOP** if no candidate can be reached anonymously and kept for as long as the image is offered.
2. **The mirror.**
   - A workflow step downloads Arm's tarball, refuses it unless its sha256 matches the value pinned in `container/`, and publishes it to the chosen place.
   - The mirror's digest is pinned in a file (`container/GCC-SOURCE`, in `ARDUPILOT-IMAGE`'s format), checked the way `check-ardupilot-pin.sh` checks a pin.
3. **The written offer.** Each public image carries a `SOURCES.md` at a fixed path, and the repository keeps the same files under `container/`. Each names:
   - every GPL component, its version, and exactly where its Corresponding Source is;
   - ArduPilot (in the image), Arm's GCC (the mirror), KiCad 10.0.7 (KiCad's tagged source, mirrored if KiCad's host is the only copy), and Debian (by package and version);
   - how long the source stays available.

   Label each image to point at it (`org.opencontainers.image.licenses` and a `documentation` label).
4. **The licence files.**
   - `ARDUPILOT-LICENSE.md` names the mirror.
   - `KICAD-LICENSE.md` gains KiCad itself, beside the libraries.

## 2. Acceptance

- An anonymous client can fetch the mirrored GCC source by its pinned digest. CI proves this with a job that does exactly that, with no credentials.
- Each public image contains `SOURCES.md` at the stated path, and its labels point at it. A CI step reads both from the built image.
- The pin file is refused by name when it is malformed, with fixtures in the style of `check-ardupilot-pin.sh`.
- **The CLAUDE.md gates pass.** CI is green.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-07`; `RUL-15`; `4400a` D-2.
- **Fence:**
  - `container/` (the licence files, `SOURCES.md`, the pin file and its check);
  - `.github/workflows/ardupilot-image.yml` and `tools-image.yml` (the mirror step and the label and read checks);
  - FEEDBACK.

  No crate. Do not touch `ci.yml`.
- No version number of any kind; the specification is frozen at 12 (roadmap §6).
- One PR, left unmerged. Write `~/.cache/aet-4403/pr-title-body.md`.

## 5. Adversarial pass, targeted

- A tarball whose sha256 does not match the pin.
- A mirror that pulls only with credentials.
- An image that ships without `SOURCES.md`.
- A `SOURCES.md` naming a version other than the one installed in the image (check at least ArduPilot's commit, GCC's release and KiCad's version against the image).

## 7. Flag-and-stop conditions

- **§1.1:** no anonymous, durable place for the mirror.
- **A GPL component whose source we cannot name** at its exact version.
- **Mirroring needs a credential the workflow does not already hold.**
