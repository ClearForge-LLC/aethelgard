# AET-WO-4407 spike: apt from a snapshot.debian.org date

Predictions first (committed before any measurement), then what was measured.

## 1. Predictions (written 2026-11-23, before measuring)

All **[inferred]**, from `1605` FEEDBACK (main run 44299148057), `4401` Q3 and `4403` D5.

- **P1, a fresh uncached build today drifts.** With Debian's moving index, the
  tools image resolves `linux-libc-dev` at a version other than the table's
  `6.12.115-2` (1605 measured `6.12.117-1`), so `check-sources inside tools`
  (fail mode) refuses it. The ArduPilot image drifts on the same package if its
  table names `linux` at all; otherwise it reproduces its table today.
- **P2, the reference is the published images' full dpkg lists.** The pinned
  tools image (`container/IMAGE`, `e5fc7ddb`) and ArduPilot image
  (`ARDUPILOT-IMAGE`, `160f9955`) each pass their table in fail mode, because
  main's publish refused otherwise. Their `/SOURCES-debian.tsv` minus their base
  image's dpkg list is what apt resolved; that set is the target, and the GPL-2
  tables follow from it.
- **P3, one date exists.** A single timestamp T, used for `trixie`,
  `trixie-updates` and `trixie-security` (and `bookworm*` for the `renode-nvic`
  stage), resolves both remainders exactly. T lies in the window where
  `linux-libc-dev 6.12.115-2` was trixie-security's current version, near the
  creation time of the published tools image's apt layer.
  - The explicit pins disagree on `ca-certificates` (`20250419` in tools,
    `20250419+deb13u1` in ArduPilot). One date still serves both because the
    KiCad base already holds `20250419`, so apt keeps it installed.
- **P4, the source shape.** `debian:trixie-slim` has no CA bundle before apt
  installs one, so the snapshot source is `http://` (apt checks the signed
  `InRelease`). An old snapshot's `Valid-Until` has passed, so the source needs
  `check-valid-until=no`.
- **P5, two fresh builds at T** give byte-identical sorted dpkg lists (and
  `/SOURCES-debian.tsv`). The apt layers' digests still differ (logs, caches,
  mtimes).
- **P6, `renode-nvic`.** Its one package `patch=2.7.6-7` resolves at T from
  `bookworm` with no other package installed; its two DLL sha256 checks are
  unchanged.
- **P7, `# syntax=`.** Today `check-pins.sh` passes a Dockerfile whose first
  line is `# syntax=docker/dockerfile:1`, in any spelling (case, spaces, a
  UTF-8 BOM before it). A BOM also hides `# escape=` from today's check.
- **P8, GtkSharp.** `964cb9c31506adf7c1c68fe8a6ac8c8076237f3a` exists in
  `GtkSharp/GtkSharp`, has no submodules, and GitHub's archive of it has a
  stable sha256 across two downloads. The commit stays **[inferred]** as the
  one `3.24.24.95` was built from; mirroring makes the source available, not
  the inference measured.
- **P9, the pin-check echo.** Today a `container/ARDUPILOT-IMAGE` whose second
  line is `::warning::x` prints that line alone on stderr, at the start of a
  log line.

## 2. Measured

(Filled in below as measured.)
