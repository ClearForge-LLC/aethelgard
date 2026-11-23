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

## 2. Measured (Furnace, 2026-11-23 00:20–01:10 UTC)

**Verdict: STOPPED at §7, "no snapshot date reproduces the committed tables."**
The two tables were resolved at different times and disagree on `linux`. Each
one reproduces exactly at a date of its own; no single date serves both.

### 2.1 The reference (P2 held) [measured]

The two published images' `/SOURCES-debian.tsv`, minus their base images'
dpkg lists, are what apt resolved. No base package was upgraded in either.

| Image | Pin | apt resolved | apt layer created | `linux-libc-dev` |
|---|---|---|---|---|
| tools | `e5fc7ddb` | 35 packages on `kicad/kicad@3224d1bf` | 2026-11-08T00:07:39Z | `6.12.115-2` |
| ArduPilot | `160f9955` | 93 packages on `debian:trixie-slim@a72b4433` | 2026-11-16T10:16:08Z | `6.12.117-1` |

`TOOLS-DEBIAN-GPL2.tsv` names `linux 6.12.115-2` and `ARDUPILOT-DEBIAN-GPL2.tsv` names
`linux 6.12.117-1`, as the images do. Every other package the two remainders
share has one version in both. `ca-certificates` and `libbrotli1` differ, but in
the tools image they come from the KiCad base, so apt does not resolve them (P3's
sub-bullet held).

### 2.2 A fresh build today fails (P1, wrong: worse than predicted) [measured]

Using Debian's moving index (the base images' own sources) on 2026-11-23, both
Dockerfiles' apt steps refuse:

    E: Version '2.41-12+deb13u4' for 'libc6-dev' was not found

The trixie point release between 2026-11-21T06:00Z and 12:00Z moved `libc6-dev` to
`deb13u5`. Both Dockerfiles pin `deb13u4`. So today either image builds only from
the CI cache: any change to an apt `RUN` line breaks it, and so does a gha cache
eviction before main's next publish. That includes this work order's own change.

### 2.3 The windows (P3, wrong) [measured]

`window.py` reads each timestamp's `trixie`, `trixie-updates` and `trixie-security`
amd64 `Packages` from snapshot.debian.org. It takes apt's candidate as the highest
version across the three, and lists every remainder package whose candidate is not
its published version. The edges come from bisection and from snapshot's
`first_seen` times.

| Table | Reproduces from | Until | What ends it |
|---|---|---|---|
| tools | `20261102T161514Z` (`linux 6.12.115-2` enters trixie-security) | before `20261108T202647Z` | `linux 6.12.116-1` enters trixie-security |
| ArduPilot | `20261113T212035Z` (`linux 6.12.117-1` enters trixie-security) | between `20261121T060000Z` and `T120000Z` | the point release: `libc6-dev`, `libc-dev-bin` `deb13u5`, `openssl 3.5.9-1~deb13u1`, `libnghttp2-14 …+deb13u2`, `rsync …+deb13u2` |

The windows do not overlap. Inside the ArduPilot window, `linux 6.12.115-2` is in none
of the three suites, so an explicit `linux-libc-dev=6.12.115-2` pin would not resolve
either.

Confirmed by real apt, `resolve.sh` (`apt-get install -s` of each Dockerfile's
explicit pins, in a capped container, with the snapshot source in place of the base's):

| Run | Result against the published remainder |
|---|---|
| tools on KiCad base at `20261105T120000Z` | **identical**, 35 of 35 |
| ArduPilot on trixie-slim at `20261117T120000Z` | **identical**, 93 of 93 (re-run from the committed script: identical) |
| tools at `20261117T120000Z` | one line: `linux-libc-dev 6.12.117-1` for `6.12.115-2` |
| ArduPilot at `20261105T120000Z` | one line: `linux-libc-dev 6.12.115-2` for `6.12.117-1` |

**The diff, at any one date:** exactly the `linux` source package. That is three rows
of one table (`.dsc`, `.debian.tar.xz`, `.orig.tar.xz`) and one binary
(`linux-libc-dev`, headers that come in through `libc6-dev`). No tool version is
involved.

### 2.4 The other predictions [measured]

- **P4 held.** The shape in `trixie-*.sources` works on `trixie-slim`, which has no CA
  bundle: deb822, `http://snapshot.debian.org/archive/debian[-security]/<T>`,
  `Signed-By` the base's keyring, `Check-Valid-Until: no`. `apt-get update` passes
  the signature check on the `InRelease` files.
- **P5: measured after the ruling (§4).**
- **P6 held.** On the SDK base (bookworm, apt 2.6.1), a `bookworm` snapshot at
  either date resolves `patch 2.7.6-7` alone (`Debian:12.15/oldstable`).
- **P7 held.** Today `check-pins.sh` passes all of these:
  - `# syntax=docker/dockerfile:1`, `#syntax=…` and `# SYNTAX = …`;
  - a UTF-8 BOM before `# syntax=`;
  - a shebang line before it.

  A BOM before `# escape=` also passes, so today's escape refusal is blind to it.
- **P8 held.**
  - `GtkSharp/GtkSharp` has `964cb9c31506adf7c1c68fe8a6ac8c8076237f3a`, committed
    2023-05-08T16:55:59Z, a merge with parents `45175543` and `82dc24c1`.
  - It has no `.gitmodules` (HTTP 404) and no gitlink in its tree.
  - GitHub's archive of it, downloaded twice, is 2 200 493 bytes, sha256
    `7e1e6bc4ae1ad356f884512cc3d58f7e1c642ee5dfafc7ded547473d287f3cef` both times.
- **P9 held.** A pin file `x\n::warning::injected\n` makes `check-ardupilot-pin.sh`
  print `::warning::injected` alone, at the start of a stderr line.

## 3. Reproduce

    python3 -I spike/4407/window.py IDXDIR REMAINDER.tsv 20261105T120000Z ...
    bash spike/4407/resolve.sh NAME BASE spike/4407/trixie-<T>.sources PKG=VER ...

`REMAINDER.tsv` is `package<TAB>version`: the published image's `/SOURCES-debian.tsv`
minus its base image's dpkg list.

## 4. After the ruling (D1 (b), architect 2026-11-23T01:15Z)

One timestamp, `20261117T120000Z`, in every apt stage. Measured on Furnace on
2026-11-23 with `podman build --isolation=chroot --no-cache` in a capped scope.

- **Before (`main` `28fba951`):** both builds fail by name at their apt step.
  ArduPilot fails after 2 s and tools after 51 s, with
  `E: Version '2.41-12+deb13u4' for 'libc6-dev' was not found`.
- **After, ArduPilot ×2** (77 s each):
  - the two `/SOURCES-debian.tsv` are byte-identical (171 packages) and equal
    the published `160f9955`'s;
  - `check-sources.sh inside ardupilot` passes in fail mode (29 flagged sources,
    all in the table);
  - the layer digests differ between the two, except the base and the plain
    `COPY` layers (P5's second half held).
- **After, tools ×2** (94 s and 114 s):
  - the two `/SOURCES-debian.tsv` are byte-identical (669 packages); against the
    published `e5fc7ddb`, only `linux-libc-dev` moves, `6.12.115-2` → `6.12.117-1`,
    as ruled;
  - `check-sources.sh inside tools` passes in fail mode with the new table (84
    flagged sources);
  - `smoke.sh` passes, and both Renode DLLs match their pinned sha256.
