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

## 2. Measured against the predictions

Furnace, 2026-11-16 16:41–17:40 UTC. Raw evidence in `~/.cache/aet-4403/` (inventories `inv-*.txt`, `out-tools.txt`, `bundled-tools.txt`, `snap-check.tsv`, `size-*.tsv`). Every container ran under `~/bin/capped-run` (memory 8 GiB, swap 0, read back), one at a time, `--network none` except the one rehearsal that needed Arm's host.

| Prediction | Result |
|---|---|
| Arm's tarball is about 170 MB at the predicted URL, anonymous | **Wrong on size, right on the rest [measured].** 200 406 637 bytes; sha256 `2721e414ef279118ac6c6f4f57114d8dad09a875f98025b123e9dc1c59177156`; the URL redirects (302) to `armkeil.blob.core.windows.net`, Last-Modified 2020-12-08. Arm's download page answered 403 to a script, so Arm's published MD5s were not read there. |
| ArduPilot's copy of the binary is Arm's | **Held [measured].** Arm's `gcc-arm-none-eabi-10-2020-q4-major-x86_64-linux.tar.bz2` has sha256 `21134caa…cd618`, the value `ardupilot.Dockerfile` pins for ArduPilot's firmware server. |
| The toolchain's docs name the source package | **Held [read].** `share/doc/gcc-arm-none-eabi/release.txt`: "The release also contains source code package (together with build scripts and instructions to setup the build environment)", and it names the gcc, binutils, newlib and gdb commits. `readme.txt` refers to "the source package". |
| (not predicted) Is Arm's tarball complete? | **Yes, for this Linux binary [measured].** It holds `gcc`, `binutils`, `newlib`, `gdb`, `gmp-6.1.0`, `mpfr-3.1.4`, `mpc-1.0.3`, `isl-0.18`, `libiconv-1.15`, `expat-2.1.1`, `zlib-1.2.8`, `libelf-0.8.13`, the build scripts and the build manual. The release notes' only other prerequisite, `python-2.7.7.msi`, is for the Windows host. |
| Repository public; ArduPilot package anonymous; a new package private | **Held in part.** The repository is public (anonymous API 200) and both image packages serve an anonymous token and `tags/list` [measured]. The anonymous token for `aethelgard-ardupilot-source` was refused, but that package does not exist, so "a new package is created private" is **[read]** from architecture §2.4 (`RUL-15`'s record), not measured. `aethelgard-engine` is refused anonymously: token endpoint HTTP 403 [measured]. |
| About 150 Debian packages in the ArduPilot image, half GPL | **Wrong in count [measured].** 171 binary packages from 97 source packages; 154 mention a GPL-family licence in their `copyright` file. |
| Several hundred in the tools image | **Held [measured].** 669 binary packages from 431 source packages. |
| Every Debian source version is retrievable | **Held [measured].** All 442 distinct (source, version) pairs answer 200 at `snapshot.debian.org/mr/package/<src>/<ver>/srcfiles`. Their sources total about 594 MB (ArduPilot image) and 2.96 GB (tools image, one package unsized). |
| `pymavlink` (LGPL-3.0) and `empy` (LGPL) are the pip set's GNU-licensed packages | **Held [measured/read].** `empy`'s metadata says `%LICENSE`, so this came from its `em.py` (`__license__ = 'LGPL'`). `pymavlink` is installed as a binary wheel; its sdist (sha256 `8d65a8b0…10062`) is on PyPI. `empy` is installed from its sdist as pure Python, so the image holds its source. |
| KiCad is built from source outside dpkg, at the tag | **Held [measured/read].** No dpkg package owns `/usr/bin/kicad-cli` or `libkicommon.so.10.0.7`; `kicad-cli version --format about` says `10.0.7, release build`, built 2026-10-18. KiCad's recipe, `Dockerfile.10.0-stable` in `kicad/packaging/kicad-docker` (last changed `131ab51f`, 2026-09-22), clones `-b $KICAD_VERSION` with the release tag and applies no patch [read]. Tag `10.0.7` is commit `69cafb90…` on GitLab and on KiCad's GitHub mirror alike [measured, `git ls-remote`]: two hosts, so no mirror (§1.3). |
| Renode carries GPL/LGPL code | **Held [measured/read].** 10 of the 346 licence files in `/opt/renode/licenses` carry a GNU header: `tlib` (LGPL-2.1, QEMU-derived), `Packet.Net` and `NetMQ` (LGPL-3.0), and seven GTK# assemblies (LGPL-2.0). Versions are from `Renode.deps.json`. The tlib and Packet.Net commits are Renode `v1.17.0`'s gitlinks (`src/Infrastructure` `066a7f13`, its `tlib` `167decf9`; `lib/Packet.Net` `20e0f2fe`). NetMQ `4.0.1.12` is commit `cdf17750`, named in its own nuspec. |
| Rust, rustup and Kani carry no GPL code | **Held [measured].** The GPL texts under `share/doc/rust/licenses` cover `src/gcc` and GCC test-suite files in Rust's source tree. The installed components are rustc, cargo, rust-std, clippy and rustfmt, with no libgccjit; `gcc-ld` is rust-lld's wrapper. Kani's `license-notes.txt`: CBMC BSD-4-Clause, kissat MIT. |
| The ~1 000 Python files no dpkg package owns | **Byte-compiled caches [measured].** All but two are `.pyc`; the two are KiCad's `pcbnew.py` and `_pcbnew.so`. |
| (b) a release asset | **Not measured; [read].** The repository has no release, and creating one needs `contents: write`, which no job holds and the builder cannot test without that credential (§7). A public repository's release assets download anonymously [read, GitHub docs]. |
| The manifest digest is computable before the push | **Held [measured].** oras 1.3.6 pushed the committed bytes to distribution 3.1.2 inside a `--network none` container and reported `sha256:00f5fb5b…31c8`, the pin. `oras pull` and `gcc-source.sh fetch` both returned Arm's exact bytes. |

**GTK# `3.24.24.95` (the one source not tagged upstream).** Its package and assembly name no commit (no SourceLink, no `.snupkg`, nuspec `repository` without `commit`). The image's `GtkSharp.dll` is byte-identical to the nupkg's `lib/net6.0/GtkSharp.dll` [measured]. The commit `964cb9c` is **[inferred], from premises each [read]:**
1. GtkSharp's workflow at `964cb9c` triggers only on `push` and `pull_request`.
2. `build.cake` at `964cb9c` names a build `3.24.24.$GITHUB_RUN_NUMBER` and appends `-develop` unless `GITHUB_REF` is `refs/heads/master`.
3. A re-run keeps its run number, so run 95 is a fresh run of a push to `master`.
4. GitHub Packages shows `3.24.24.95` created 2023-05-08 16:59:03Z, the CI's own push step.
5. `master`'s head at 16:55:59Z became `964cb9c` (the merge of PR #402) and is still `964cb9c`.
6. The DLL's embedded PDB path is `/home/runner/work/GtkSharp/GtkSharp/…/net6.0/GtkSharp.pdb`, a GitHub-hosted runner.

A byte-for-byte rebuild would make this measured. I did not attempt it: SDK 6.0.x/7.0.x as of that day, the runner path, and network restore make it an hour with no certain end (§7, D4).

## 3. GPLv3 §6, the text relied on

From the licence Arm ships in `share/doc/gcc-arm-none-eabi/license.txt`:

> d) Convey the object code by offering access from a designated place (gratis or for a charge), and offer equivalent access to the Corresponding Source in the same way through the same place at no further charge. You need not require recipients to copy the Corresponding Source along with the object code. If the place to copy the object code is a network server, the Corresponding Source may be on a different server (operated by you or a third party) that supports equivalent copying facilities, provided you maintain clear directions next to the object code saying where to find the Corresponding Source. Regardless of what server hosts the Corresponding Source, you remain obligated to ensure that it is available for as long as needed to satisfy these requirements.

How the build reads it **[inferred; not legal advice]**:
- **"Through the same place".** The GCC mirror is in the very package the image is pulled from, so it is available by the same protocol and the same anonymous token.
- **"Clear directions next to the object code".** That is `/SOURCES.md` in the image, plus the `documentation` label.
- **"For as long as needed".** That is the "How long" section, and it makes the next point a commitment the operator should read.

## 4. What was built

- **The choice: (a), in the existing package** (`ghcr.io/clearforge-llc/aethelgard-ardupilot`, tag `gcc-arm-none-eabi-10-2020-q4-major-src`). Why not the alternatives:
  - **A new package** would be private until an owner flips it, so the anonymous fetch could not hold on main's first run.
  - **(b) needs `contents: write`.** It lands in the repository's releases rather than next to the image, and `RUL-15` keeps write permissions off everything but `publish`.
  - **Costs:** 200 MB in ghcr (public storage is free) and one push of `packages: write`, which `publish` already holds. On later publishes, the mirror is verified and Arm is not asked.
- **`container/gcc-source.manifest.json`** is the manifest, committed byte for byte, and **`container/GCC-SOURCE`** pins its sha256 in `ARDUPILOT-IMAGE`'s format.
  - The tarball's sha256 and size are the manifest's one layer: one source for each fact.
  - `check-gcc-source.sh` checks the pin through `check-ardupilot-pin.sh`, the manifest's sha256 against the pin, and the manifest's shape. It has 29 fixtures in `test-check-gcc-source.sh`, each drawing its one refusal by name.
- **`container/gcc-source.sh`:**
  - `fetch` is anonymous by construction. `curl -q` reads no `.curlrc`, no netrc and no docker config, and sends only the token a registry's Bearer challenge hands out. It verifies the manifest's and the blob's sha256.
  - `publish` pushes only when the pin answers 404, and pushes nothing over a mirror that is there but wrong.
  - oras 1.3.6 is downloaded and checked by sha256.
- **`ardupilot-image.yml`:**
  - `publish` mirrors before pushing the image.
  - `source` (`needs: publish`, no login) fetches the pin anonymously.
  - `source-rehearsal` runs on pull requests. It does the same publish and fetch end to end against distribution 3.1.2 on the runner, fetches a public image's manifest and blob from ghcr anonymously, and requires `aethelgard-engine` to be refused by name.
  - The `paths` gain the new files, and `ARDUPILOT-LICENSE.md`, which `container/ardupilot*` never matched (case).
- **`/SOURCES.md` and `/SOURCES-debian.tsv` in both images,** `container/ARDUPILOT-SOURCES.md` and `container/TOOLS-SOURCES.md` in the repository.
  - The Debian list is written by dpkg in the Dockerfile's last step, so it is exact by construction even though transitive packages float.
  - `check-sources.sh` reads both and the labels from every built image and from the image `publish` is about to push. The expected values come from the image, never from the document.
- **Labels:**
  - **`documentation`** is the offer's URL at `main`. `publish` sets it to the published commit.
  - **The tools image's `licenses`** changes from `MIT OR Apache-2.0` (wrong for KiCad's image) to `GPL-3.0-or-later AND CC-BY-SA-4.0`, its principal components; the rest is in the offer.
- **`ARDUPILOT-LICENSE.md`** names the mirror and the offer.

## 5. Adversarial pass (targeted, §5) and mutations

All [measured] on Furnace. Harnesses are in `~/.cache/aet-4403/adv/`. They predate the reduction pass: they call the `push` and `arm` subcommands since removed, and the tail images and tarball they mounted were deleted at clean-up. After the reduction, the full `publish` path (Arm, push, fetch, a second publish, `oras pull`) and the push-digest refusal through `publish` were re-run and held.

- **A tarball whose sha256 is not the pin's.** These are refused by name:
  - a manifest whose layer sha256 differs from Arm's real file (`arm`: "…is not sha256:…; refused", and the file is deleted);
  - a tampered local file at push;
  - a registry serving a wrong blob, a short blob, or a manifest that is not the digest's;
  - a lying mirror under `publish`, which pushes nothing.
- **A mirror that pulls only with credentials.**
  - ghcr's private `aethelgard-engine`: refused, "the token endpoint answered HTTP 403".
  - A registry answering 401 with Basic: refused, "401 without a Bearer challenge".
  - A Bearer realm over HTTP: refused by name.
  - With a hostile `HOME` (a `.curlrc` adding `Authorization`, a `.netrc`, a docker `config.json`), the fake registry logged no `Authorization` header. Without `-q`, it logged two (mutation `no-q`).
- **An image without `SOURCES.md`.** Refused: "missing from the image". So are a `/SOURCES.md` that differs from the repository's, a missing, tampered or truncated Debian list, a duplicated row, and a row removed for a GPL pip package or for KiCad.
- **A `SOURCES.md` naming another version.** Refused, each naming the row and the value the image holds:
  - ArduPilot's commit changed, or moved out of the version cell;
  - the GCC release, the mirror's pin and the tarball sha256;
  - pymavlink's and empy's versions;
  - KiCad `10.0.6`, Renode, GTK# and NetMQ;
  - `tlib` and `Packet.Net` unnamed.

  These were checked against the pinned published images, given the new labels and `/SOURCES.md` by a RUN-free build. Their dpkg list was written exactly as the Dockerfile writes it.
- **Two holes, closed before the run.** A version moved to another cell, and a second row with a wrong version, would each have passed a whole-row match. `need` now reads one column of exactly one row.
- **Mutations of the guards this WO adds** (one each):
  - `check-gcc-source.sh`: 17 of 17 killed by its fixtures, after adding `layer-annotation-extra` for the one that survived.
  - `gcc-source.sh`: 9 of 9 change the outcome.
  - `check-sources.sh`: 9 of 9 change the outcome.

  Four of those 18 are defence in depth: the next guard still refuses, with a vaguer message. They are the realm check (`--proto` also refuses), the push's file check (the digest check also refuses), and the `/SOURCES.md` and Debian-list missing checks (the `cmp` also refuses). They are kept because §5 asks for refusals by name.

## 6. Gates and CI

**Local gates [measured], Furnace, capped scope (32 GiB, swap 0):** `cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo test --workspace` exit 0, no failure. No crate changed; the merge of `main` (`311398d`) brought docs only. `check-pins.sh` passes on all three Dockerfiles, and `test-check-pins.sh`, `test-check-ardupilot-pin.sh` and `test-check-gcc-source.sh` pass. TIER2_MIN is untouched (main's 160).

**CI on `794a1cc` (PR #176), all green [measured]:**
- **`ardupilot-image` [run 43917454424](https://github.com/ClearForge-LLC/aethelgard/actions/runs/43917454424):**
  - `image` builds fresh and reads back the labels and `/SOURCES.md`: 171 dpkg packages, every version matched.
  - `pinned`, the four boards and `pinout` pass.
  - `source-rehearsal`:
    - **The full publish path:** 404, then Arm's tarball (sha256 `2721e414…`, 200 406 637 bytes), oras pushes it at `sha256:00f5fb5b…`, and it is fetched anonymously.
    - **A second `publish`:** "already mirrored".
    - **`oras pull`:** the same bytes.
    - **ghcr's anonymity, both ways:** `aethelgard-ardupilot@a4818a04…`'s manifest and config are fetched anonymously, and `aethelgard-engine` is refused: "the token endpoint answered HTTP 403".
  - `publish` and `source` are skipped, as on any PR (D1).
- **`tools-image` [run 43917454461](https://github.com/ClearForge-LLC/aethelgard/actions/runs/43917454461):**
  - `image` builds fresh and reads back the offer: 669 dpkg packages; KiCad `10.0.7`, Renode `1.17.0`, GTK# `3.24.24.95` and NetMQ `4.0.1.12` matched; Renode's 10 GNU-licensed parts named.
  - The workspace gates inside the image pass.
- **`ci` [run 43917454907](https://github.com/ClearForge-LLC/aethelgard/actions/runs/43917454907):** `gates` and `tools` pass. Shard times, a measurement under the 04:35 rule:
  - shard 0: 24:30
  - shard 1: 19:57
  - shard 2: 28:37
  - shard 3: 30:09
  - shard 4: 28:50
  - shard 5: 23:12
  - shard 6: 22:59
  - shard 7: 27:31

  This work order changes no test and no `ci/` file, so the plan is main's.

Reduction pass (diff): −4 net in the last commit · 1 duplicate removed (the empty config's digest, a second copy of the manifest's) · 3 dead paths removed (the `arm` and `push` subcommands, which nothing calls, and a workflow step that repeated what `gcc-source.sh` runs first) · 2 protected kept (the realm check, labelled `belt-and-suspenders`; the dpkg format string written in both Dockerfiles and in the check, because the check must regenerate independently) · 1 finding below (F3). An earlier cut, `fetch`'s `cmp` against the committed manifest, was load-bearing: a REF with another digest would have passed. It became "the REF's digest must be the pin".

## 7. Decisions and questions for the architect

- **D1, sequencing (the main one).** `RUL-15` lets only `main` push, so this PR cannot show ghcr serving the mirror anonymously.
  - **What this PR proves.** `source-rehearsal` proves the scripts on the runner, and ghcr's anonymity both ways.
  - **What gates the image.** The first ghcr proof comes on main's first publish after merge. Inside `publish`, the mirror is pushed and then fetched anonymously before the image push, and that re-fetch is what stops the image if the mirror fails. `source` runs after the image is pushed: the separate, credential-free proof.
  - **Three things only `main` exercises, all [inferred] today:**
    - oras using `docker/login-action`'s credentials against ghcr;
    - ghcr accepting this artifact manifest (empty config, `application/octet-stream` layer, `artifactType`) in a container package;
    - build-push-action's `labels:` overriding the Dockerfile's `documentation` LABEL, which the `labels … "$GITHUB_SHA"` check in both `publish` jobs requires.

    Each fails closed, but a failure stops image publishing on every `main` push until it is fixed, so that first run is worth watching.
  - **Arm's host on every PR.** Until then, `source-rehearsal` starts from an empty registry and downloads from Arm on every pull request that triggers `ardupilot-image`, which is any `crates/**` change by any builder. An Arm outage would turn those PRs red, which cuts against D-2's intent.
  - **The follow-up.** The PR that re-pins `ARDUPILOT-IMAGE` and `IMAGE` should come straight after the merge. It should run `source` on pull requests against the then-real pin, and seed the rehearsal from the mirror instead of Arm.
- **D2, `KICAD-LICENSE.md` not edited (deviation from §1.4).** It must stay byte-identical to `crates/aethelgard-kicad-lib/data/kicad-10.0.7/LICENSE.md` (`workspace_shape.rs:878`, `smoke.sh:121`), and that copy is KiCad's own text, outside the fence. So KiCad itself (GPL-3.0-or-later, tag, commit, recipe) is named in `TOOLS-SOURCES.md` instead. If you want it in a licence file, a new `container/KICAD-PROGRAM-LICENSE.md` is the route.
- **D3, the commitment in "How long".** Both offers say ClearForge LLC keeps the mirror and offer as long as it serves an image that names them, and mirrors any third-party source that disappears. The second is what §6(d)'s last sentence obliges, and Scotty should confirm the public wording. Third-party-only sources today:
  - KiCad and its libraries (two hosts);
  - Renode with tlib and Packet.Net, GTK#, NetMQ (GitHub only);
  - pymavlink's sdist (PyPI);
  - Debian (snapshot.debian.org).
- **D4, GTK# `3.24.24.95` (§7 "cannot name").** It is named, at commit `964cb9c`, but **[inferred]** from read premises (§2). The options are to accept that, to fund a byte-for-byte rebuild to make it measured, or to mirror GtkSharp's source at that commit next to the tools image.
- **D5, GPLv2-only Debian packages.** GPLv2 §3 has no third-party-server clause like GPLv3 §6(d). It counts source "from the same place" only, so pointing at snapshot.debian.org is weaker for them **[inferred, not legal advice]**. Mirroring the Debian sources costs about 0.6 GB beside the ArduPilot image and 3.0 GB beside the tools image [measured]. The work order names Debian's archive as the route, so I did not.
- **D6, the tools image's inherited labels.** `vendor=KiCad`, `url=https://kicad.org` and `authors=<KiCad's devlist>` come from the base and describe ClearForge's image as KiCad's. Left alone (outside "point at it"); a one-line override if you want it.
- **F3, the engine image** is `FROM` the tools image. At the next re-pin it will carry `/SOURCES.md` and the tools image's `documentation` label, though not its `licenses` label, which it overrides. It is private today; if it is ever published it needs its own offer for the CLI it adds.

## 8. The signal corrected: READY → STOPPED

**The first acceptance clause is not met on this PR.** It reads: "An anonymous client can fetch the mirrored GCC source by its pinned digest. CI proves this with a job that does exactly that, with no credentials".

- On `c3c9525`, the job that does exactly that, `source`, was skipped. `source-rehearsal` fetched from a registry on the runner, and from ghcr only an existing image, never the mirror itself.
- The mirror cannot exist on ghcr before a `main` publish, because `RUL-15` gives `packages: write` to `main`'s `publish` job alone.
- Pushing it any other way would need a credential the workflow does not hold, which is a §7 stop.
- So `c3c9525`'s READY claimed more than CI showed. This commit replaces it with STOPPED: the push after a READY that kickoff item 7 allows only on request, made because the earlier signal overclaimed.

**Ruling needed:**
- **(a) Merge, then prove (recommended).** `main`'s `publish` pushes the mirror and re-fetches it anonymously before the image. `source` then fetches it with no credentials. The re-pin follow-up runs `source` on pull requests. The clause is met on `main`'s first run, not on this PR.
- **(b) Amend `RUL-15` for one publish of the mirror from this PR's run.** Its content is fixed by the committed manifest and the pinned sha256. Then `source` runs here.
- **(c) Another route you name.**

Nothing else changes. Everything in §§4–6 is built, and CI is green at `c3c9525`.

## 9. Rulings on #176 (architect, 2026-11-16 18:50 UTC), and D6 built

- **D1: (a), merge then prove.** `RUL-15` is unchanged. The first acceptance clause is met by `main`'s first run after merge, with the anonymous re-fetch inside `publish` and then `source`. The re-pin follow-up runs `source` on pull requests.
- **D2: accepted.** KiCad itself is named in `TOOLS-SOURCES.md`; `KICAD-LICENSE.md` stays byte-identical to the crate's copy.
- **D4: accepted for now.** A backlog row is to mirror GtkSharp's source at `964cb9c`.
- **D6: built in this PR.**
  - **Vendor and url:** the tools image now says `vendor=ClearForge LLC` and `url=https://github.com/ClearForge-LLC/aethelgard`.
  - **Authors:** `authors` is set empty. A Dockerfile has no instruction that deletes an inherited label, so "drop" is as close as an empty value comes, and the key stays in `docker inspect`.
  - **The check:** `check-sources.sh labels tools` now requires all three, on every build and on the image `publish` pushes.
  - **Measured on Furnace:** a RUN-free build of the pinned tools image plus the new `LABEL` passed. The same build with `vendor=KiCad` was refused: "vendor label is [KiCad], not [ClearForge LLC]".
- **D3 (the "How long" wording) and D5 (the GPLv2 Debian mirror) wait for Scotty.** Untouched until the architect's next note.
- **Signal:** STOPPED again after D6, `main` `424f22d` merged in. Waiting only on Scotty for D3 and D5.
- **Signal:** `main` `55ed4c4` (4211, #173) merged in, with no conflict and no `container/` or workflow change. CI was green at `e403af9` on all three workflows. Still waiting only on Scotty for D3 and D5.

## 10. Scotty's rulings on D3 and D5 (architect's note, 2026-11-16 19:30 UTC)

- **D3: the "How long" wording is approved as written** ("Honesty first always"). Both offers keep it unchanged.
- **D5: the GPLv2-only Debian sources are to be mirrored in a follow-up PR** after this one merges, on a new branch `wo/aet-4403-debian-src` from `main`.
  - It is pushed by the `main` publish job, as the GCC mirror is.
  - `SOURCES.md` points to it.
  - It gets the same anonymous-fetch proof.
  - This PR's scope is unchanged.
- **Acceptance stays as D1 ruled:** the first clause is met on `main`'s first publish after merge.
- **Signal:** READY. `main` `2add53d` (docs #179) is merged in, and every ruling (D1–D6) is applied or recorded. CI was last green on all three workflows at `e403af9`; since then, only `main` and FEEDBACK have changed.

# D5 follow-up: the GPLv2-only Debian sources mirrored (branch `wo/aet-4403-debian-src`)

**The first ghcr proof of these mirrors comes on `main`'s first publish after merge, as D1(a) ruled for the GCC mirror.** This branch was cut from `main` `3dcd242`, after #176 merged as `938b7f4`.

## D5.0 After #176 merged (proven on main)

`ardupilot-image` push run 43944349355 is green.
- **`publish`** found the pin absent (404), took Arm's tarball by sha256, and pushed the mirror at exactly `sha256:00f5fb5b…`. It re-fetched the mirror anonymously, then pushed the image `aethelgard-ardupilot@sha256:7b7ebed3…`.
- **`source`**, with no credentials, fetched the manifest and the 200 406 637-byte tarball by the pinned digest.
- **Furnace** fetched it anonymously at 20:06:37Z.

The three main-only items D1 called [inferred] are therefore measured. The record is `~/.cache/aet-4403/post-merge.md`.

## D5.1 Process note and predictions

**Process.** The first measurements ran before any prediction was committed: the GPL-2-only classifier on the published ArduPilot image (`7b7ebed3`) and on the tools image pinned in `container/IMAGE`, and the free-text blind-spot scan. They are reported as measured in D5.2. The predictions below cover only what is still unmeasured.

All **[inferred]**:
- **The tools image published from `938b7f4`** has the same 669 dpkg packages as its pin (the fresh CI build of #176 had 669). The classifier gives the same 47 flagged source packages, about 389 MB, and the same 22 blind-spot packages, about 188 MB.
- **Every flagged (source, version)** has a `.dsc` on snapshot.debian.org whose `Checksums-Sha256` matches the files snapshot serves.
- **ghcr accepts one artifact per (source, version)** in the image's own package, in the GCC mirror's manifest shape, with one layer per source file.
- **From a GitHub runner, snapshot.debian.org** serves a ~10 MB subset in under a minute. The full ~370 MB for the ArduPilot image fits `publish`'s 20 minutes only if snapshot is fast. That is unmeasured, and it decides the timeout.
