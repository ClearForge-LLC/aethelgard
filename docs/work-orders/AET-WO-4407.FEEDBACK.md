# AET-WO-4407 — FEEDBACK: image hygiene (a measurement, STOPPED at §7)

**Branch:** `wo/aet-4407-image-hygiene` from `main` `28fba951`.
**Builder:** Claude (Opus 5.5), alone. The work so far is measurement, sequenced by its result; no worker was needed.
Written 2026-11-23.

**Status: STOPPED at §7, "no snapshot date reproduces the committed tables. Stop with the diff."**
- Changed: only `spike/4407/` and this file.
- Unchanged: every crate, specification, `fab/`, `ci/`, `container/` and workflow file.
- The predictions were committed (`7cf11f82`) before any measurement.

**The answer, in five lines:**
- **The two committed tables were resolved on different days and disagree on `linux`:** tools `6.12.115-2`, ArduPilot `6.12.117-1`. Each matches its published image.
- **Each table reproduces exactly at a snapshot date of its own:**
  - tools from `20261102T161514Z` until before `20261108T202647Z`;
  - ArduPilot from `20261113T212035Z` until the point release between `20261121T06` and `T12Z`.

  The windows do not overlap, so no single date serves both (`spike/4407/README.md` §2.3).
- **The diff at any one date is the `linux` source package alone:** three rows of one table and the `linux-libc-dev` headers. No tool version is involved.
- **Worse than predicted: neither image builds fresh today.** Both Dockerfiles pin `libc6-dev=2.41-12+deb13u4`, which the trixie point release of about 2026-11-21 replaced. Both images build only from the CI cache. Changing any apt `RUN` line breaks them, and so does a gha cache eviction before a main publish. A snapshot pin, at either date, ends that.
- **Everything else predicted held:**
  - the snapshot source shape works on `trixie-slim` without a CA bundle;
  - the `renode-nvic` stage resolves `patch 2.7.6-7` at either date;
  - every `# syntax=` spelling, and a BOM before `# escape=`, passes today's check;
  - GtkSharp `964cb9c` has no submodules, and its archive's sha256 is stable;
  - the pin-check echo puts `::warning::` at a line start.

## 1. Measured

Measured on Furnace, 2026-11-23 00:20–01:10 UTC. The predictions, the tables of results and the scripts (`window.py`, `resolve.sh`, the two snapshot sources files) are in `spike/4407/README.md`.

**Method.**
- **Reference:** each published image's `/SOURCES-debian.tsv` minus its base image's dpkg list. That is 35 packages for tools and 93 for ArduPilot. It is stricter than the GPL-2 tables, which are a subset of it.
- **Windows:** bisected on snapshot's `Packages` indices with `window.py`.
- **Confirmation:** a real `apt-get install -s` of each Dockerfile's explicit pins in a capped container (`resolve.sh`), with the snapshot source replacing the base's `debian.sources`.

**Results.**
- **At its own date, each resolves its published remainder exactly:** tools 35/35 at `20261105T120000Z`, ArduPilot 93/93 at `20261117T120000Z`.
- **At the other image's date, each differs in exactly one line,** `linux-libc-dev`.
- **The published apt layers were built at 2026-11-08T00:07Z (tools) and 2026-11-16T10:16Z (ArduPilot).** Each lies inside its window.

## 2. Decision needed

**D1, the date (§7 stop).** Three ways to proceed. In all three, the build (§1.2) is the same apart from the date and the table rows.

| | Dates | Table bytes | Image change from today's pins |
|---|---|---|---|
| **(a)** | one per Dockerfile: tools (with `renode-nvic`'s bookworm) `20261105T120000Z`, ArduPilot `20261117T120000Z` | none move | none in apt; both reproduce what is published |
| **(b)** | one, `20261117T120000Z`, for every Dockerfile | `TOOLS-DEBIAN-GPL2.tsv`: the three `linux` rows → `6.12.117-1`, regenerated with `debian-src.py table` from a snapshot build, which now equals CI's | tools `linux-libc-dev` `6.12.115-2` → `6.12.117-1`; main's next tools publish mirrors `linux 6.12.117-1` beside `aethelgard-tools` |
| **(c)** | one, in the tools window | `ARDUPILOT-DEBIAN-GPL2.tsv`: `linux` back to `6.12.115-2` | ArduPilot moves back one kernel-headers release; not recommended |

**Lean: (b).**
- It is the WO's literal "one timestamp", and the newer of the two.
- The table moves only for headers that reach the image through `libc6-dev`.
- The regeneration is the honest kind 1605 asked for: from a build that any machine reproduces.
- **(a) is the zero-byte route** if you would rather no table move in this work order. `check-pins.sh` would then enforce one date per Dockerfile, across its stages, in place of one date for the repository.

**D2, for scheduling, not a ruling.** Until this lands, both images are unbuildable from Debian's moving index (§1). A concurrent work order that changes an apt line in either Dockerfile, or a cache eviction, fails the build by name at `libc6-dev`.

## 3. Fence notes for the build (applies under any D1)

- **(1) The revision label.** Its check cannot go in `check-sources.sh` (outside the fence). I will assert it in `tools-image.yml`'s `publish`: one `docker image inspect` step against `$GITHUB_SHA`, before the push.
- **(4) The GtkSharp mirror** needs new `container/` files on the GCC-source pattern:
  - a committed manifest (`gtksharp-source.manifest.json`, one layer: the archive above) and its pin;
  - the push in `tools-image.yml`'s `publish`, and the anonymous fetch in its `source` job;
  - a localhost-registry rehearsal job in `tools-image.yml`, so the mirror code runs on the pull request before main's publish (1605's failure mode).

  `gcc-source.sh` is ArduPilot-bound and outside the fence. I will drive `oci-mirror.sh` directly, or add a small sibling script, whichever is shorter. Please confirm that new `container/` files are within the fence.
- **(3) The snapshot shape I will build:**
  - one committed deb822 sources file per suite family, `COPY`ed over the base's `/etc/apt/sources.list.d/debian.sources` in each apt stage;
  - `check-pins.sh` refuses:
    - an `apt-get update` in a stage without that `COPY` above it;
    - a sources file that names anything but `http://snapshot.debian.org/archive/debian[-security]/<T>` with `Check-Valid-Until: no`;
    - a date that differs from the ruled one(s);
    - any other write to `/etc/apt` and any `APT_CONFIG`.

  Each refusal gets a fixture.
- **(2) The syntax directive.** Refuse `# syntax=` in any case and spacing, and a UTF-8 BOM, which also closes the BOM gap in today's `# escape=` refusal. Fixtures for each spelling, run under C and UTF-8 locales.

## 4. Gates

- **No Rust byte moved** (`git diff --stat origin/main -- crates fab ci container .github` is empty).
- `cargo fmt --all -- --check` passes. `cargo clippy` and `cargo test` were not run: nothing they read changed.
- `bash container/test-check-pins.sh` passes unchanged.

## 5. Adversarial and reduction passes

Not run: they belong to the build, after the ruling.

**Signal:** STOPPED. Waiting on D1 (and the fence confirmation in §3).
