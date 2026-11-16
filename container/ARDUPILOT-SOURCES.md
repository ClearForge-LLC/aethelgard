# Corresponding source: ghcr.io/clearforge-llc/aethelgard-ardupilot

ClearForge LLC publishes this image's object code from `ghcr.io`. This file is the written offer of its source, under GPLv3 §6(d) and the matching terms of the other licences below.

- **In the image:** at `/SOURCES.md`.
- **In the repository:** as `container/ARDUPILOT-SOURCES.md`, byte for byte.
- **The labels:** `org.opencontainers.image.documentation` links to this file.

Each component under the GNU GPL or LGPL is listed with its version in this image and the place its complete Corresponding Source is.

| Component | Version in this image | Licence | Corresponding Source |
|---|---|---|---|
| ArduPilot | tag `Copter-4.7.2`, commit `fb773567896014da007bca58358704d4a0d8516f`, each submodule at the commit the tag records | GPL-3.0-or-later (`COPYING.txt`) | **In this image**, complete with `.git`, at `/opt/ardupilot`. Also at <https://github.com/ArduPilot/ardupilot>, tag `Copter-4.7.2`. |
| GNU Arm Embedded Toolchain | `gcc-arm-none-eabi-10-2020-q4-major` (GCC 10.2.1 20201103, binutils 2.35, newlib 3.3.0, GDB 10), Arm's x86_64 Linux binary, sha256 `21134caa478bbf5352e239fbc6e2da3038f8d2207e089efc96c3b55f1edcd618`, at `/opt/gcc-arm-none-eabi-10-2020-q4-major` | GPL-3.0-or-later (GCC, with the GCC Runtime Library Exception; binutils; GDB), LGPL-3.0-or-later (GMP, MPFR, MPC, linked into the tools), and the others in `share/doc/gcc-arm-none-eabi/license.txt` | **Mirrored next to this image**, at `ghcr.io/clearforge-llc/aethelgard-ardupilot@sha256:00f5fb5b2402161226493495be4c5cc189e6b9594251010f1da3ba99220e31c8` (tag `gcc-arm-none-eabi-10-2020-q4-major-src`). It is an OCI artifact whose one file is Arm's `gcc-arm-none-eabi-10-2020-q4-major-src.tar.bz2`, byte for byte: 200 406 637 bytes, sha256 `2721e414ef279118ac6c6f4f57114d8dad09a875f98025b123e9dc1c59177156`. It holds the source of GCC, binutils, newlib and GDB, the libraries the build links (GMP, MPFR, MPC, ISL, libiconv, expat, zlib, libelf), and Arm's build scripts. Arm first published it at <https://developer.arm.com/-/media/Files/downloads/gnu-rm/10-2020q4/gcc-arm-none-eabi-10-2020-q4-major-src.tar.bz2>. |
| pymavlink | `2.4.50`, the PyPI wheel for CPython 3.13 on x86_64 (it contains compiled code), in `/opt/venv` | LGPL-3.0 | The sdist `pymavlink-2.4.50.tar.gz` on PyPI, sha256 `8d65a8b05b2971babeef4b6ca74a247a72276f62f7559466c64e96ece7f10062`: <https://pypi.org/project/pymavlink/2.4.50/#files>. |
| empy | `3.3.4`, in `/opt/venv` | LGPL (its `em.py`) | **In this image**: it is pure Python, installed from its sdist, so the installed `em.py` is its source. The sdist is `empy-3.3.4.tar.gz` on PyPI, sha256 `73ac49785b601479df4ea18a7c79bc1304a8a7c34c02b9472cf1206ae88f01b3`. |
| Debian 13 packages | every installed package, listed at `/SOURCES-debian.tsv` | each under its own licence, in `/usr/share/doc/<package>/copyright`; many are under the GPL or LGPL | By source package and exact version, in the archive of every version Debian has published. Each line of `/SOURCES-debian.tsv` gives the binary package, its version, its source package and source version, and that source's page on `snapshot.debian.org`. While a version is still current, `apt-get source <source>=<version>` fetches it from Debian's mirrors. **Every source package with code under GPL-2.0-only is also mirrored next to this image,** byte for byte, because GPLv2 has no third-party-server clause. `/SOURCES-debian-gpl2.tsv` lists each one's files with their sha256. Each package is an OCI artifact at `ghcr.io/clearforge-llc/aethelgard-ardupilot@sha256:<that file's last column>`, tag `debian-src.<source>__<version>` (`+`, `~` and `:` written `_p`, `_t` and `_e`). |

The other Python packages in `/opt/venv` are under MIT, BSD or ISC licences. They are pinned by hash in `container/ardupilot-requirements.txt` and `container/ardupilot-build-requirements.txt`, and their sources are on PyPI at those versions.

## Fetching the mirrored toolchain source

No account or token is needed. With [oras](https://oras.land):

```
oras pull ghcr.io/clearforge-llc/aethelgard-ardupilot@sha256:00f5fb5b2402161226493495be4c5cc189e6b9594251010f1da3ba99220e31c8
sha256sum gcc-arm-none-eabi-10-2020-q4-major-src.tar.bz2
```

Or with `curl` only, from a checkout of the repository:

```
bash container/gcc-source.sh fetch ghcr.io/clearforge-llc/aethelgard-ardupilot@sha256:00f5fb5b2402161226493495be4c5cc189e6b9594251010f1da3ba99220e31c8 DIR
```

The mirrored Debian sources, every package of `container/ARDUPILOT-DEBIAN-GPL2.tsv`, with `curl` only, from a checkout of the repository:

```
python3 container/debian-src.py fetch ghcr.io/clearforge-llc/aethelgard-ardupilot container/ARDUPILOT-DEBIAN-GPL2.tsv DIR
```

## How long

- **ClearForge LLC keeps the mirrors and this offer available** for as long as it offers any image that names them from `ghcr.io/clearforge-llc`. None of them is deleted while such an image remains.
- **Some sources are on a third party's server:** every source above that is neither in this image nor in the mirror. If one of those servers stops serving the exact version named here, ClearForge LLC mirrors that version next to the image.
- **To ask for any of it,** open an issue at <https://github.com/ClearForge-LLC/aethelgard/issues>.
