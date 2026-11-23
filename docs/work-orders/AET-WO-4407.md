# AET-WO-4407 — Image hygiene: the revision label, `# syntax=` refused, apt from a snapshot date, GtkSharp's source mirrored, one pin regex, a bounded echo

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4407-image-hygiene`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4, backlog wave 1. Written 2026-11-23. **Cadence:** a measurement, then a build.

**Grounds** (roadmap §7.1; read `4401`, `4402`, `4403` and `1605` FEEDBACK first):
1. **`4403` Q3 (#186).** `tools-image.yml`'s `publish` sets no `org.opencontainers.image.revision` label. Add it in one line.
2. **`4401` Q2, Q3.** A `# syntax=` directive is not refused by `check-pins.sh`. The apt-dependency half of this row is (3).
3. **`4403` D5 Q1 and `1605` (main run 44299148057).** Pin apt in every Dockerfile to one `snapshot.debian.org` timestamp, so a fresh build resolves the same packages as the committed GPL-2/LGPL-2 source tables.
4. **`4403` D4.** GtkSharp `3.24.24.95` is named at commit `964cb9c` by inference. Mirror its source at that commit by sha256, on the `oci-mirror.sh` and GCC-source pattern, or state why the commit cannot be confirmed.
5. **`4402` Q3.** `ardupilot-image.yml`'s `publish` has its own copy of the pin regex. Route it through `container/check-ardupilot-pin.sh`.
6. **`4402` Q4, the pin-check half of `1311` adversarial J, F.** The pin check echoes the refused file, so a `::warning::` line reaches the log. Bound the echo and escape it. The eda reader's half is a later work order.

> **What this is:** (1)–(6), each with a test or a CI check that can fail.
>
> It is NOT:
> - re-pinning `container/IMAGE` or `ARDUPILOT-IMAGE`. That follows `main`'s publish, as in `1605` and `4402`;
> - a tool version change;
> - any crate change.

## 1. Build

1. **Measure first,** with predictions in `spike/4407/`:
   - a fresh, uncached build's resolved apt set against the committed tables;
   - the snapshot date that reproduces them.
2. **(1)–(6),** each with its test.
3. **Two fresh builds** at the snapshot resolve the same package set.

## 2. Acceptance

- **(1)–(6) closed.**
- **A fresh build with no cache** resolves apt identically twice, and matches the committed tables.
- **The CLAUDE.md gates pass, and CI is green.**

## 3. Fences

- **Fence:**
  - `container/`: the Dockerfiles, `check-pins.sh` and its fixtures, `check-ardupilot-pin.sh` and its tests, `oci-mirror.sh`, `*-SOURCES.md`, `*-DEBIAN-GPL2.tsv`;
  - `.github/workflows/tools-image.yml`;
  - **only** the `publish` job of `ardupilot-image.yml`;
  - `spike/4407/` and FEEDBACK.
- **Not:** any crate, specification or `fab/` file; `ci/`.
- **Running beside you:**
  - `4300i` (IR, generators, eda);
  - `1607` (CLI);
  - `3307` (evolve);
  - `2309` (chaos).

  Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-4407/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **An apt fetch that still reaches the moving index:** a second `apt-get update`, or a stage that skips the snapshot source.
- **A `# syntax=` spelling the check misses:** case, spaces, or a BOM.
- **An echo that still reaches the log unbounded.**

## 7. Flag-and-stop conditions

- **A tool version in the image moves.** Stop.
- **No snapshot date reproduces the committed tables.** Stop with the diff.
- **A crate or `fab/` byte moves.** Stop.
