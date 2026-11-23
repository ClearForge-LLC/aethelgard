# AET-WO-4407 — FEEDBACK: image hygiene (a measurement, then a build)

**Branch:** `wo/aet-4407-image-hygiene` from `main` `28fba951`.
**Builder:** Claude (Opus 5.5), alone. I kept it single-threaded: the work is gate logic, a pin and its check, and ingress (the global contract's no-fan-out list).
Written 2026-11-23.

**Status: READY.**
- The measurement STOPPED at §7 (§1). It was ruled D1 (b) by the architect at 2026-11-23T01:15Z, and the build follows that ruling (§2).
- No crate, specification, `fab/` or `ci/` byte moves. No tool version moves.

**The answer, in six lines:**
- **Both images build fresh again.**
  - Before: from Debian's moving index, both fail by name, `E: Version '2.41-12+deb13u4' for 'libc6-dev' was not found`.
  - After: from `snapshot.debian.org` at `20261117T120000Z`, both build. Two uncached builds of each resolve byte-identical dpkg lists, and `check-sources inside` passes in fail mode (§2.1).
- **One date, ruled:** `20261117T120000Z`. The tools table's three `linux` rows move to `6.12.117-1`, regenerated from snapshot and identical to the ArduPilot table's rows. The tools image's `linux-libc-dev` moves `6.12.115-2` → `6.12.117-1`.
- **`check-pins.sh` enforces the snapshot.** It refuses:
  - an `apt-get update` in a stage with no snapshot `COPY`;
  - a sources line that is not snapshot;
  - two timestamps;
  - any other write to `/etc/apt`, and `APT_CONFIG`;
  - every `# syntax=` spelling BuildKit reads, and a byte-order mark.
- **GtkSharp's source at `964cb9c` is mirrored** by sha256 beside the tools image. A pull-request rehearsal runs the push and the fetch first.
- **The revision label** is set in tools `publish` and checked before the push. **ArduPilot's `publish`** reads its digest through `check-ardupilot-pin.sh`. **The pin-check echo** is cut to 200 bytes and escaped with `%q`.
- **Adversarial, found and fixed:** under `en_US.UTF-8`, `check-pins.sh` accepted a fullwidth digit in a `FROM` digest. It now runs under `LC_ALL=C`. Mutation: 10/10 killed.

## 1. The measurement and the stop

Measured on Furnace, 2026-11-23 00:20–01:10 UTC; full record in `spike/4407/README.md` §1–§3. Predictions committed first (`7cf11f82`).

- **The two committed tables disagreed on `linux`:** tools `6.12.115-2`, ArduPilot `6.12.117-1`. Each matched its published image.
- **Each reproduced exactly at a snapshot date of its own.** The test was real apt against the published image's full dpkg remainder:
  - tools 35/35 at `20261105T120000Z`, window `20261102T161514Z` to before `20261108T202647Z`;
  - ArduPilot 93/93 at `20261117T120000Z`, window `20261113T212035Z` to the point release between `20261121T06` and `T12Z`.

  The windows do not overlap. At either date the diff was the `linux` source package alone.
- **Stopped (§7).** Ruled D1 (b) at 01:15Z: one date, `20261117T120000Z`, with the tools table's `linux` rows moved, "not a tool version, so not the §7 stop". The fence was confirmed for new `container/` files and for a revision step in tools `publish`.

## 2. What was built

| WO item | Change | Test that can fail |
|---|---|---|
| (1) revision label | `tools-image.yml` `publish`: `org.opencontainers.image.revision=${{ github.sha }}`, and a step `the revision label is the commit published` before any push | that step, on `main` (it cannot run on a pull request: nothing there sets the label) |
| (2) `# syntax=` | `check-pins.sh` refuses `(#\|//)\s*syntax\s*=` in any case, on any line, and a UTF-8 BOM anywhere, which also uncovers a BOM-hidden `# escape=` | fixtures `syntax-directive`, `syntax-upper`, `syntax-slashes`, `syntax-shebang`, `bom-syntax`, `bom-escape` |
| (3) apt from one snapshot | `container/debian-trixie.sources` and `debian-bookworm.sources` (deb822, `http://snapshot.debian.org/archive/debian[-security]/20261117T120000Z`, `Check-Valid-Until: no`), `COPY`ed over the base's `debian.sources` in all three apt stages (tools, its `renode-nvic` stage, ArduPilot); `check-pins.sh` enforces the shape (above); `TOOLS-DEBIAN-GPL2.tsv` `linux` rows → `6.12.117-1` | fixtures `apt-moving`, `apt-stage-skips`, `apt-second-source`, `apt-config-env`, `apt-sources-missing`, `apt-moving-sources/`, `apt-two-dates/`; the fresh builds (§2.1); CI's `check-sources inside` |
| (4) GtkSharp source | `container/gtksharp-source.manifest.json` (one layer: GitHub's archive of `964cb9c`, sha256 `7e1e6bc4…`, 2 200 493 bytes) and `container/gtksharp-source.sh` (`fetch`, `publish`, on `oci-mirror.sh`); push in tools `publish`, anonymous fetch in `source`, a new `source-rehearsal` job on pull requests; `TOOLS-SOURCES.md`'s GTK# row names the pin `ghcr.io/clearforge-llc/aethelgard-tools@sha256:03cf1db8…` | the rehearsal job (publish twice, the second pushes nothing; `TOOLS-SOURCES.md` must name the pin); the `source` job on `main` |
| (5) one pin regex | ArduPilot `publish` writes the pushed digest to a file and reads it through `check-ardupilot-pin.sh` | `test-check-ardupilot-pin.sh` (the script's fixtures); the step itself on `main` |
| (6) bounded, escaped echo | `check-ardupilot-pin.sh` quotes at most 200 bytes with `printf %q`, and says how many bytes there were; the tools revision step does the same | fixtures `workflow-command`, `carriage-return`, `long-line`; every `form` fixture must now give one stderr line of at most 1 024 bytes |

**Not done, by the WO:** `container/IMAGE` is not re-pinned. That follows main's publish of this PR, as a follow-up (ruled).

### 2.1 The fresh builds [measured]

`podman build --isolation=chroot --no-cache --pull=never` in a capped user scope (16 GiB, no swap), on Furnace, 2026-11-23.

| Build | Result |
|---|---|
| ArduPilot, `main` `28fba951` | **fails** in 2 s: `E: Version '2.41-12+deb13u4' for 'libc6-dev' was not found` |
| tools, `main` `28fba951` | **fails** in 51 s, the same line |
| ArduPilot, this branch, A and B | both build, in 77 s each. `/SOURCES-debian.tsv` A = B byte for byte (171 packages) = published `160f9955`. `check-sources.sh inside ardupilot` passes in fail mode: 29 flagged sources, all in the table |
| tools, this branch, A and B | both build, in 94 s and 114 s. `/SOURCES-debian.tsv` A = B byte for byte (669 packages). Against published `e5fc7ddb` one line differs, as ruled: `linux-libc-dev 6.12.115-2` → `6.12.117-1`. `check-sources.sh inside tools` passes in fail mode: 84 flagged sources, all in the new table. `smoke.sh` passes, so the tool versions it checks (KiCad, Renode, ngspice, Kani) are unchanged. Both Renode DLLs match their pinned sha256 |

The layer digests differ between A and B, except the base's and the plain `COPY`s. That was predicted (P5): apt's logs and caches carry times. What is reproduced is the package set, which is what the tables are judged on.

## 3. Adversarial pass, targeted (§5)

- **An apt fetch that still reaches the moving index:**
  - A stage with no snapshot `COPY` is refused (`apt-moving`), including a later stage after an earlier one that had it (`apt-stage-skips`; a stage `FROM` an earlier one is refused too, conservatively).
  - A second `apt-get update` in a snapshotted stage still reads the snapshot, because nothing else may touch `/etc/apt`. These are refused, read on whole instructions with quotes spliced:
    - an `echo > /etc/ap''t/…` (`apt-second-source`);
    - `ENV APT_CONFIG` (`apt-config-env`);
    - `COPY --from`, `--mount` and `ADD` onto `/etc/apt`.
  - A sources file with any non-snapshot line, or a second timestamp, is refused (`apt-moving-sources/`, `apt-two-dates/`).
  - **Residual:** a base image's own `/etc/apt/sources.list` or `apt.conf.d` would still be read. The three bases are pinned by digest and were read: each has only `sources.list.d/debian.sources`, which the `COPY` replaces.
- **A `# syntax=` spelling the check misses.** The spellings come from how BuildKit reads it: it skips a BOM, skips a shebang line, and reads `#` or `//`, key case-insensitive, `\s*` around `=`.
  - Case, spaces, `//`, a shebang before the directive and a BOM are each refused.
  - Every fixture runs under `C` and `en_US.UTF-8`.
  - A whole-file JSON form has no `FROM` line, so it is already refused.
  - **Residual:** `BUILDKIT_SYNTAX` as a build argument. No workflow sets one.
- **Found here and fixed:** under `en_US.UTF-8`, `check-pins.sh` accepted `FROM debian@sha256:<63 a>４`, with a fullwidth digit **[measured]**. It now sets `LC_ALL=C`, as `check-ardupilot-pin.sh` has since 4402. Fixture: `from-lookalike`. This is a one-line hardening outside items (1)–(6), inside the fenced file; flagged here.
- **An echo that still reaches the log unbounded:**
  - `check-ardupilot-pin.sh`'s refusal is one line of at most 1 024 bytes for every fixture, including the 65 600-byte `long-line` (343 bytes).
  - `::warning::` after a newline or a CR is escaped into the same line.
  - `check-pins.sh` still echoes a refused Dockerfile line whole, but always after a `check-pins: line N:` prefix, so no line it prints starts with `::`. A Dockerfile line holds no newline. Its length is the committed file's.
  - The eda reader's echo is the later work order's, per the WO.

**Mutation (one mutant per guard this WO adds, against its own suite): 10/10 killed.**

| Mutant | Killed by |
|---|---|
| m1, `apt-get update` with no snapshot allowed | `apt-moving` |
| m2, `/etc/apt` and `APT_CONFIG` not checked | `apt-config-env` |
| m3, no syntax refusal | `syntax-directive` |
| m4, no BOM refusal | `bom-escape` |
| m5, any sources line accepted | `apt-moving-sources` |
| m6, no one-date check | `apt-two-dates` |
| m7, no `LC_ALL=C` | `from-lookalike`, under `en_US.UTF-8` |
| m8, the `COPY`'d file not checked | `apt-sources-missing` |
| m9, the echo not escaped | `two-lines` and 10 more |
| m10, the echo not bounded | `two-lines`, `long-line` |

## 4. Reduction pass (diff)

- **Cut:** a test block in `test-check-pins.sh` that re-checked "one timestamp for every Dockerfile". `check-pins.sh` already refuses that over every `*.sources` beside the Dockerfile.
- **Cut:** a URL/title regex in `gtksharp-source.sh`. The layer's sha256 and size pin the bytes, and `oci-mirror.sh` checks the title and refuses any other file before a push.
- **Kept:** `gtksharp-source.sh` instead of generalising `gcc-source.sh`. `gcc-source.sh` is ArduPilot-bound and outside the fence; its seeding path has no GtkSharp counterpart. Merging the two is a candidate for the next container work order.
- No `belt-and-suspenders:` code was touched.

## 5. Questions for the architect

- **Q1, a trigger gap (outside the fence).** `ardupilot-image.yml`'s `paths:` lists do not include `container/debian-trixie.sources`. A later PR that only moves the snapshot date would rebuild the tools image (`container/**`) but not the ArduPilot image, so ArduPilot's table would not be judged at the new date. The fix is one line in each of its two `paths:` lists.
- **Q2, moving the date.** Moving it is now one edit to both `*.sources` files (check-pins refuses a mismatch), plus regenerating whichever table rows move. Regenerate with `debian-src.py table` from a snapshot build: any machine now reproduces CI's layer, so 1605's "only from CI" lesson no longer binds.
- **Q3, the re-pin.** `container/IMAGE` still pins `e5fc7ddb`, whose `linux-libc-dev` is `6.12.115-2`. After main publishes this PR, the re-pin picks up `6.12.117-1`, the GtkSharp mirror and the revision label. Main's `publish` also mirrors `linux 6.12.117-1` beside `aethelgard-tools` (about 154 MB).

## 6. Gates

All three pass on Furnace at `83bed98e` (2026-11-23):
- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- `cargo test --workspace`: 160 suites, 2 784 passed, 0 failed.

Beside them:
- `bash container/test-check-pins.sh` and `bash container/test-check-ardupilot-pin.sh` pass;
- `python3 -I container/debian-src.py check container/TOOLS-DEBIAN-GPL2.tsv` passes (84 packages, 274 files);
- `git diff --stat origin/main -- crates fab ci` is empty, and the one `ardupilot-image.yml` hunk is inside `publish`.

## 7. CI

CI_RESULT

**Signal:** READY.
