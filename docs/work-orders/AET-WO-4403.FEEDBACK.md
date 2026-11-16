# AET-WO-4403 FEEDBACK — Corresponding source for the public images

**Builder:** Claude (strong tier; Opus 5.5), worktree `~/aethelgard-4403`, branch `wo/aet-4403-gpl-source` from `main` `1178ca8`.
**Cadence:** build, with one measured stop (§1.1), and a targeted adversarial pass. The work order is read from `origin/claude/argon-repo-docs-synthesis-rjrb0d` (docs PR #174).

Labels:
- **[read]**: read at source (this repository, an upstream page or file, a run log).
- **[measured]**: run on Furnace or on GitHub's runner.
- **[inferred]**: neither; reasoning from the two.

## 1. Predictions (committed before any pull, download or run)

All **[inferred]**.

**The ArduPilot image's GPL components.**
- ArduPilot `Copter-4.7.2` (GPL-3.0-or-later), source in the image.
- Arm's `gcc-arm-none-eabi-10-2020-q4-major`: GCC (GPL-3.0-or-later with the runtime exception), binutils and GDB (GPL-3.0-or-later), newlib (mostly BSD-style). Its source is one tarball on Arm's site.
- About 150 Debian packages, of which about half carry a GPL or LGPL licence in `/usr/share/doc/*/copyright` (bash, coreutils, git, make, gcc-14, rsync, glibc's LGPL).
- Two Python packages under the LGPL: `pymavlink` (LGPL-3.0) and `empy` (LGPL-2.1). The rest of the pip set is MIT, BSD, ISC or Apache.

**The tools image's GPL components.**
- KiCad 10.0.7 (GPL-3.0-or-later). I expect the `kicad/kicad` image to carry KiCad built from source by KiCad's own CI, outside dpkg, so its source is KiCad's `10.0.7` tag (GitLab, with GitHub mirrors and a source tarball on KiCad's download host). Not the only copy, so no mirror.
- Several hundred Debian packages (wxWidgets, OCCT, Python, ngspice and their libraries), with GPL ones among them.
- Renode's portable release carries some GPL or LGPL code (its `tlib` derives from QEMU). Rust, rustup and Kani (with CBMC) carry none.

**Arm's source tarball.** `https://developer.arm.com/-/media/Files/downloads/gnu-rm/10-2020q4/gcc-arm-none-eabi-10-2020-q4-major-src.tar.bz2`, about 170 MB, fetchable anonymously. The binary that ArduPilot's firmware server serves is byte-identical to Arm's binary release (Arm published an MD5 for each package). The toolchain's `share/doc/gcc-arm-none-eabi/readme.txt` names the source package.

**Where the mirror lives.**
- (a) ghcr: `packages: write` covers a push. A **new** package (`aethelgard-ardupilot-source`) is private when Actions create it, so an anonymous pull fails until an owner sets it public, as `RUL-15`'s record says of the tools image. The same blob pushed under a tag in the **existing**, already public `aethelgard-ardupilot` package pulls anonymously at once. If the artifact's one layer is the tarball itself, its blob digest is the tarball's sha256, and its manifest can be built deterministically, so the pin can be computed before the publish.
- (b) a release asset: needs `contents: write`, which no job holds and `RUL-15` keeps off CI. That is a credential the workflow does not hold (§7).
- So (a), in the existing package. No stop at §1.1.
