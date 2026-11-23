# Corresponding source: ghcr.io/clearforge-llc/aethelgard-tools

ClearForge LLC publishes this image's object code from `ghcr.io`. This file is the written offer of its source, under GPLv3 §6(d) and the matching terms of the other licences below.

- **In the image:** at `/SOURCES.md`.
- **In the repository:** as `container/TOOLS-SOURCES.md`, byte for byte.
- **The labels:** `org.opencontainers.image.documentation` links to this file.

Each component under the GNU GPL or LGPL is listed with its version in this image and the place its complete Corresponding Source is.

| Component | Version in this image | Licence | Corresponding Source |
|---|---|---|---|
| KiCad | `10.0.7`, release build: binaries in `/usr/bin`, libraries in `/usr/lib/x86_64-linux-gnu`, `pcbnew` in `/usr/lib/python3/dist-packages`. It comes from the base image `kicad/kicad:10.0.7` | GPL-3.0-or-later | KiCad's tag `10.0.7`, commit `69cafb907ebc09e7722a2a464b1d466fc2e3608d`, unmodified, at <https://gitlab.com/kicad/code/kicad> and at KiCad's mirror <https://github.com/KiCad/kicad-source-mirror>. The base image was built by `Dockerfile.10.0-stable` at commit `131ab51f246671f73ce4be97d13a71fcb54d6795` of <https://gitlab.com/kicad/packaging/kicad-docker>, which clones that tag and applies no patch. |
| KiCad libraries | `10.0.7`: symbols, footprints and templates in `/usr/share/kicad` | CC-BY-SA-4.0 with KiCad's design exception (`/usr/share/kicad/LICENSE.md`) | Tag `10.0.7` of <https://gitlab.com/kicad/libraries/kicad-symbols> (commit `e1bb0a65cb243525367c5dc7294665b9fd57fc4d`), <https://gitlab.com/kicad/libraries/kicad-footprints> (commit `51f8a59ed7d5f3bab9bb296c7b767bb033f3ba4c`) and <https://gitlab.com/kicad/libraries/kicad-templates> (commit `3ed4538b0f965d821df63a5fffc4441e723cfe7f`). |
| Renode | `1.17.0`, Antmicro's portable release, sha256 `4ba7c68b59e2447f188ef4b4b112fcccd2802582c460beedceb63b84e5605a5f`, at `/opt/renode`, with two of its assemblies rebuilt from the same source (`AET-WO-1605`): `Infrastructure.dll` with three upstream NVIC commits, and `Renode.dll` with the build string `1.17.0+20260907gitf1dd1b4af.aet1605` | MIT, with the LGPL parts below; every bundled licence is in `/opt/renode/licenses` | <https://github.com/renode/renode> at commit `f1dd1b4af7838b45a925c17603cdfed0a583844a` ("Add Changelog for v1.17.0", two commits before the tag `v1.17.0`), the commit the asset's build string names and was built from, with its submodules at the commits it records: `git clone https://github.com/renode/renode`, `git -C renode checkout f1dd1b4af7838b45a925c17603cdfed0a583844a`, `git -C renode submodule update --init --recursive`. Its `src/Infrastructure` is <https://github.com/renode/renode-infrastructure> at `cd4b002aac2398994c4a61a34a8b31b5700facf2` and its `lib/InpliTftpServer` <https://github.com/antmicro/InpliTftpServer> at `2a4877dce99fd7a30b85b5bf217c631538cd7086`. The rebuilt `Infrastructure.dll` adds `renode-infrastructure` commits `e7a8584cac829e060e98fc2850dcc9df61ddd594`, `c8b2075c042d1da6ae07745a63c8bd3edea31c4f` and `7b68754055b87ab3cc64c929eaac4be2b8060fef`, each in this repository as `container/renode-nvic/<commit>.patch`; `container/Dockerfile`'s `renode-nvic` stage is the build. |
| tlib | in Renode's `translate-*.so`, as recorded by Renode `f1dd1b4af7838b45a925c17603cdfed0a583844a`: `src/Infrastructure` at `cd4b002aac2398994c4a61a34a8b31b5700facf2`, whose `src/Emulator/Cores/tlib` is at `167decf9129758829762e582939128c0694e90d6` | LGPL-2.1 (derived from QEMU) | <https://github.com/antmicro/tlib> at `167decf9129758829762e582939128c0694e90d6`; included by the recursive clone above. |
| Packet.Net | Renode's `PacketDotNet.dll`, from Renode `f1dd1b4af7838b45a925c17603cdfed0a583844a`'s `lib/Packet.Net` at `20e0f2fe40743fed4e6a81bb8fe9523c257f168b` | LGPL-3.0 | <https://github.com/antmicro/Packet.Net> at that commit; included by the recursive clone above. |
| GTK# | NuGet `3.24.24.95` of GtkSharp, AtkSharp, CairoSharp, GdkSharp, GioSharp, GLibSharp and PangoSharp (the `net6.0` assemblies), in `/opt/renode` | LGPL-2.0 (Library GPL version 2) | <https://github.com/GtkSharp/GtkSharp>, branch `master` at commit `964cb9c31506adf7c1c68fe8a6ac8c8076237f3a`. GtkSharp tagged no release after `3.24.24.34`, and its package names no commit. Its CI numbers a build of a push to `master` `3.24.24.<run>`. It published `3.24.24.95` to GitHub Packages at 2023-05-08 16:59 UTC, three minutes after this commit became `master`'s head, which it has stayed since. GitHub's archive of that commit (sha256 `7e1e6bc4ae1ad356f884512cc3d58f7e1c642ee5dfafc7ded547473d287f3cef`) is mirrored next to this image at `ghcr.io/clearforge-llc/aethelgard-tools@sha256:03cf1db8b2a73d43442fc1543dc6a1c70cd603ae67bf718179c4eaa6ac2629fe`. |
| NetMQ | NuGet `4.0.1.12`, in `/opt/renode` | LGPL-3.0 | <https://github.com/zeromq/netmq>, tag `4.0.1.12`, commit `cdf17750c947b562fe3f11c025bd16b7003acff3` (the commit its NuGet package names). |
| Debian 13 packages | every installed package, listed at `/SOURCES-debian.tsv` | each under its own licence, in `/usr/share/doc/<package>/copyright`; many are under the GPL or LGPL | By source package and exact version, in the archive of every version Debian has published. Each line of `/SOURCES-debian.tsv` gives the binary package, its version, its source package and source version, and that source's page on `snapshot.debian.org`. While a version is still current, `apt-get source <source>=<version>` fetches it from Debian's mirrors. **Every source package with code under GPL-2.0-only, LGPL-2.0-only or LGPL-2.1-only is also mirrored next to this image,** byte for byte, because GPLv2 §3 and LGPL-2.1 §4 count source only from the same place. Code under a version 2 "or later" licence can be taken under GPLv3 or LGPLv3, whose §6(d) allows the third-party route above. `/SOURCES-debian-gpl2.tsv` lists each one's files with their sha256. Each package is an OCI artifact at `ghcr.io/clearforge-llc/aethelgard-tools@sha256:<that file's last column>`, tag `debian-src.<source>__<version>` (`+`, `~` and `:` written `_p`, `_t` and `_e`). |

No other part of the image is under the GPL or LGPL:
- **Rust 1.99.1** and Kani's `nightly-2026-08-21`, through rustup: MIT OR Apache-2.0. Their GPL licence texts cover parts of Rust's source tree that these components do not install.
- **Kani 0.68.0:** MIT OR Apache-2.0. It bundles CBMC (BSD-4-Clause) and kissat (MIT); see `/opt/kani/kani-0.68.0/license-notes.txt`.

## Fetching the mirrored Debian sources

No account or token is needed. Every package of `container/TOOLS-DEBIAN-GPL2.tsv`, with `curl` only, from a checkout of the repository:

```
python3 container/debian-src.py fetch ghcr.io/clearforge-llc/aethelgard-tools container/TOOLS-DEBIAN-GPL2.tsv DIR
```

Or one package with oras: `oras pull ghcr.io/clearforge-llc/aethelgard-tools@sha256:<its manifest_sha256>`.

## How long

- **ClearForge LLC keeps the mirrored Debian sources and this offer available** for as long as it offers any image that names them from `ghcr.io/clearforge-llc`. None of them is deleted while such an image remains.
- **The other sources above are on third parties' servers.** If one of those servers stops serving the exact version named here, ClearForge LLC mirrors that version next to the image.
- **To ask for any of it,** open an issue at <https://github.com/ClearForge-LLC/aethelgard/issues>.
