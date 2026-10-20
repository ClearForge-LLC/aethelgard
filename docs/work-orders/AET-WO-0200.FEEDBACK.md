# AET-WO-0200 — FEEDBACK

**Branch:** `wo/aet-0200-tools-image` · **Base:** `9c6b4d7` (the merge of PR #6) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace, tmux `aet` · **Half:** spike, stopped for the architect's ruling. The build half has not started.

Entry schema: `finding · where (file:line) · type · recommendation · decision-needed`.

## Spike

Furnace still has no container runtime and no Nix. On 2026-10-20, `command -v` for `docker`, `podman`, `buildah`, `nix`, `apptainer`, `skopeo` and `crane` found none of them. Every measurement below therefore comes from GitHub Actions.

The spike lives entirely under `spike/` and `.github/workflows/spike-tools-image.yml`, so nothing sits at the build half's paths (`container/`). The workflow builds two candidate images, one per job, from one `spike/Dockerfile`:

- **a** `FROM kicad/kicad:10.0.7`
- **b** `FROM ubuntu:26.04` plus the archive's `kicad`

Both add ngspice, Renode, Rust and Kani from checksummed sources.

Each candidate's cold build exports its layers to the Actions cache, and a second job rebuilds it on a fresh runner from that cache. `spike/probe.sh` runs inside each image twice, normally and under `--network none`, and prints `ANSWER` lines.

**The measuring run** is spike run [39355216653](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39355216653), on head `5b2086c`. All six jobs passed on the first push, and the existing `gates` job passed in run [39355216632](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39355216632). Every number below comes from that run's `ANSWER` lines, its logs, or the Actions cache API read straight after it. Job links:

| Job | Link |
|---|---|
| a, cold | [`cold (a-kicad-image)`](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39355216653/job/118408317018) |
| b, cold | [`cold (b-ubuntu-apt)`](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39355216653/job/118408317255) |
| a, warm | [`warm (a-kicad-image)`](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39355216653/job/118410316790) |
| b, warm | [`warm (b-ubuntu-apt)`](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39355216653/job/118410316846) |
| nixpkgs | [`nix`](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39355216653/job/118408317340) |
| GHCR | [`ghcr`](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39355216653/job/118408317336) |

**Recommendation in one line:**
- Use candidate **a**, a Dockerfile `FROM kicad/kicad:10.0.7@sha256:3224d1bf…` with the four checksummed additions.
- Pin the Debian archive packages by version.
- S5 needs a ruling before the build half can publish.

### S1 — Packaging: Dockerfile, confirmed

The measured reason is the `nix` job, which evaluated the four tools in nixpkgs at pinned revision `09063bd1f2b36fd9fd793c36dfade7bd40a93dfc` (nixos-unstable, 2026-10-17):

| Package | Result |
|---|---|
| `kicad` | `10.0.7` |
| `renode` | `1.17.0` |
| `ngspice`, `libngspice` | `45` |
| `cbmc` | `6.11.0` |
| `rustc` | `1.99.0` |
| `kani`, `kani-verifier` | no such attribute |

Nix would supply three of the four tools at the right versions, but **not Kani**, and **not the pinned Rust** (it carries 1.99.0, and `rust-toolchain.toml` says 1.99.1). Both would still need installing outside Nix. A Nix image would therefore be Nix plus a second install mechanism, and a Dockerfile is one mechanism.

The Dockerfile also measured well: the runner's Docker 28.0.4 built both candidates with no setup beyond `docker/setup-buildx-action`.

### S2 — Source and pin for each tool

Every row was verified in the image by the probe, both with and without the network.

**KiCad.** Use the official image `kicad/kicad:10.0.7`:
- Index digest: `sha256:3224d1bf143185ecc1bd3a1082a36f37f0f4e59d707983a16515a845e3737fc6`.
- The only platform in that index is amd64, with manifest `sha256:c8f05703…`, 812 MB compressed.
- Debian GNU/Linux 13 (trixie) underneath.
- `kicad-cli version` prints `10.0.7`.

The alternative fails on version. Ubuntu 26.04's archive package is **`kicad 9.0.8+dfsg-1`, not 10** (candidate b), so KiCad 10 comes only from the official image.

KiCad 10's `kicad-cli pcb drc --help` lists both `--exit-code-violations` and `--format` (the probe grep matched both, in both images). The flag works as `RUL-03` requires, measured on the fixtures under §S4. The flag-and-stop condition did not occur.

**Renode 1.17.0.**
- Official artifact: the GitHub release asset `https://github.com/renode/renode/releases/download/v1.17.0/renode-1.17.0.linux-portable.tar.gz`, 63,459,129 bytes.
- sha256: `4ba7c68b59e2447f188ef4b4b112fcccd2802582c460beedceb63b84e5605a5f`, GitHub's computed digest of the asset, checked by `sha256sum -c` in the build.
- There is **no `-dotnet` variant in 1.17.0**, which is why the `builds.renode.io` URL in the work order 404s. The release also ships `renode_1.17.0_amd64.deb` (`sha256:c46af81b…`) as an alternative.

`renode --version` prints `Renode v1.17.0`. Headless, `renode --disable-gui --console --plain -e "mach create; machine LoadPlatformDescription @platforms/boards/nucleo_h753zi.repl; quit"` exits 0 in 1–2 s, both with and without the network. Its log shows `System bus created`, the platform's PRNG peripheral seeded, no error, and `machine-0: Disposed`.

`--disable-xwt` is still accepted as well. The image sets `DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1` and needed no extra system packages for the portable build.

**ngspice and `libngspice0`.** Both come from the base distribution's archive, and the shared library installs alongside:

| | Candidate a (Debian trixie) | Candidate b (Ubuntu 26.04) |
|---|---|---|
| Package version | `ngspice 44.2+ds-1`, `libngspice0 44.2+ds-1` | `45.2+ds-1` |
| Shared library | `/usr/lib/x86_64-linux-gnu/libngspice.so.0.0.12`, registered with `ldconfig` as `libngspice.so.0` | |

`ngspice -b` on the RC step fixture measured a 10–90 % rise of `2.197224e-03` s against 2.2·R·C = `2.2e-03` s, which is 0.13 % off and inside the build half's 2 % bound.

**Kani 0.68.0.**
- The driver comes from crates.io: `cargo install --locked kani-verifier --version 0.68.0`, with cargo checking the registry checksum.
- The backend comes from the release bundle `kani-0.68.0-x86_64-unknown-linux-gnu.tar.gz`, `sha256:32e2b484d73ede0bbf64a2cf0879c4259422497d8aec4ee67d448de9ae7843d3`, verified and then installed with `cargo kani setup --use-local-bundle` at build time.
- `KANI_HOME=/opt/kani` is honoured: the install lands at `/opt/kani/kani-0.68.0`, and `~/.kani` does not exist.
- `cargo kani --version` prints `Kani Rust Verifier 0.68.0 (cargo plugin)` and reports CBMC 6.11.0.

Under **`--network none`**, the two-harness probe crate gave the same result in both images:
- `add_in_range` → `VERIFICATION:- SUCCESSFUL`, exit 0.
- `add_overflows` → `VERIFICATION:- FAILED`, `Failed Checks: attempt to add with overflow`, exit 1.

So nothing is fetched at run time, and the backend really runs.

**Rust.** `1.99.1`, through `rustup-init` 1.29.1 from the versioned archive URL `https://static.rust-lang.org/rustup/archive/1.29.1/x86_64-unknown-linux-gnu/rustup-init`, `sha256:dda7234360b7f578ca8b0ddcb80145646fa61a67c1720a5abc7051b35c9fcb71`. `rustc --version` in the image prints `rustc 1.99.1 (58cf8f9c3 2026-10-13)`.

### S3 — Cost

| | a (`kicad/kicad:10.0.7`) | b (`ubuntu:26.04` + apt) |
|---|---|---|
| Image size (`docker image inspect .Size`) | **4,802,171,602 B (4.8 GB)** | 3,062,476,838 B (3.1 GB) |
| Cold build step, empty cache, including cache export and loading into Docker | 250 s | 208 s |
| Cold job total, with both probe runs | 400 s | 418 s |
| Warm build step on a fresh runner, from the Actions cache | 95 s | 60 s |
| Warm job total | 166 s | 151 s |

Both are far inside the 10 GB and 45-minute stops.

**Where the cold time goes,** for candidate a:

| Stage | Time |
|---|---|
| Base pull | 17 s |
| apt | 13 s |
| Renode | 3 s |
| Rust | 10 s |
| Kani | 16 s |
| Export and load into Docker | 75 s |
| Writing layers to the Actions cache | the remainder |

The installs are cheap; moving 4.8 GB around is the cost.

**The cache holds.**
- In both warm builds, every RUN layer reported `CACHED`.
- After the run, the cache API reported 25 entries totalling **2,585,832,496 B (2.6 GB) for both candidates together**, which is 26 % of the 10 GB repository limit.
- The entries are scoped to `refs/pull/7/merge`. GitHub lets a pull request read its own caches and `main`'s, so a `main` build's cache would serve every later PR.

**Runner facts:**
- `ubuntu-latest`: x86_64, 4 CPUs, 15 GiB, Docker 28.0.4.
- 86 GB of disk free before cleanup, and 94 GB still free after building candidate a. The `rm -rf` of preinstalled SDKs was not needed (F6).

### S4 — `kicad-cli pcb drc` without `--exit-code-violations`: exit 0

On the violation fixture without the flag, the exit code is **0**, in KiCad 10.0.7 and in 9.0.8 alike, with and without the network. The fixture is two tracks of different nets 0.05 mm apart against the default 0.2 mm, and its JSON report lists 1 `clearance` and 2 `track_dangling`.

Exit codes in all four runs (two images, each with and without the network):

| Board | `--exit-code-violations` | Exit | Report |
|---|---|---|---|
| Violation fixture | absent | **0** | 3 violations |
| Violation fixture | present | **5** | 3 violations |
| Clean fixture | absent | 0 | nothing |
| Clean fixture | present | 0 | nothing |

KiCad's own demo boards in the official image (18 found) agree. With the flag, `CM5_MINIMA_3` and `ecc83-pp` exit 5 and `complex_hierarchy` exits 0. Without the flag, all three exit 0.

This settles what `architecture.md` §2 marked unverified: **a DRC gate written as "exit 0" passes with violations present.** `RUL-03`'s mandatory flag is correct, and §5.3 of the adversarial pass has its expected value: removing the flag turns a 5 into a 0.

### S5 — Where the image lives: the prior holds as far as a PR can measure; the publish itself needs a ruling

What a pull-request job holding only `contents: read` measured, in the `ghcr` job:
- GitHub's job header lists the token's permissions as `Contents: read`, `Metadata: read`.
- `docker login ghcr.io` with that `GITHUB_TOKEN` succeeds (exit 0). That proves the registry is reachable and the token is accepted for login, **not** that it can push.
- An anonymous `GET https://ghcr.io/v2/clearforge-llc/aethelgard-tools/tags/list` returns 401. That is GHCR's answer to an anonymous request for both a missing and a private package, so it does not tell us which.
- `GET /orgs/ClearForge-LLC/packages/container/aethelgard-tools` with the same token returns 404. The package does not exist yet, or a token without `packages: read` cannot see it; the two are indistinguishable here.

**What was not measured, and why.** Confirming that `GITHUB_TOKEN` can *publish* needs a job holding `packages: write`. §2 allows that permission only on the `main` publish job, and a PR job or a branch `workflow_dispatch` would break it. So the first publish job, on the first `main` push of the build half, is the confirming measurement (F1).

Two org-side preconditions decide whether that first publish succeeds:
- the organization's package-creation policy must allow GitHub Actions to create a package;
- the package must be linked to this repository. The `org.opencontainers.image.source` label on the image does that when the first push creates it.

**Proposed digest update, with CI never writing to the repository:**
1. The `main` publish job pushes `ghcr.io/clearforge-llc/aethelgard-tools`, tagged with the commit SHA.
2. It writes the pushed digest to the job summary and prints it as an `ANSWER`-style line.
3. Consumers pin `ghcr.io/clearforge-llc/aethelgard-tools@sha256:…` from one committed file, for example `container/IMAGE`.
4. A builder updates that file by PR from the digest in the publish run's summary, and the PR's own CI pulls and smoke-tests that digest before merge.

Until a digest exists, PRs build the image from the Dockerfile, as this spike did.

### What did not work, or nearly misled

- **My own `ANSWER` filter dropped a line.** While extracting answers I excluded `drc.*violations=` to hide the long per-type counts, and that also removed `drc.help.exit_code_violations=2`. I recovered it from the log before recording S2. Lesson for the build half: grep answers by exact key, not by exclusion.
- **`kani.version` printed CBMC's line.** The probe took the last line of `cargo kani --version`, which is `CBMC 6.11.0`. The Kani version (`Kani Rust Verifier 0.68.0 (cargo plugin)`) is in the log above it and is what S2 records.
- **Renode's exit 0 is not yet proof by itself.** The log shows the platform loading without error, but this run did not measure whether a *failed* platform load exits nonzero. If it does not, an exit code alone cannot gate Renode, and the build half's smoke check must also match the log (F3).
- **Nothing in the image failed to build or run on the first push.** Every tool's pin, checksum and headless invocation worked as first written.

### Spike findings

| # | finding | where | type | recommendation | decision-needed |
|---|---|---|---|---|---|
| F1 | §0 S5 asks to confirm that `GITHUB_TOKEN` can publish, but §2 confines `packages: write` to the `main` publish job, so a PR-only spike cannot measure it. Login succeeds with `contents: read`; the push is unmeasured | `.github/workflows/spike-tools-image.yml` `ghcr` job | scope-question | Accept the first `main` publish of the build half as the confirming measurement, after checking that the org allows Actions to create packages, or rule another way | **yes** |
| F2 | KiCad 10 exists only in the official image. Ubuntu 26.04's archive carries 9.0.8. The image is amd64-only, so the tools image is amd64-only | S2 | note | Use candidate a; record amd64-only in `architecture.md` §5 | **yes** (ratify candidate a) |
| F3 | Renode exits 0 after loading `nucleo_h753zi.repl`, but a failed load's exit code was not measured | `spike/probe.sh` Renode section | risk | Build half: a negative control (a nonexistent `.repl`), and assert on the log line as well as the exit code if the codes do not differ | no |
| F4 | Debian packages (`ngspice`, `libngspice0`, `gcc`, `libc6-dev`, `curl`, `ca-certificates`) are installed unpinned in the spike, signed by Debian's archive keys but free to move | `spike/Dockerfile` apt step | risk | Build half: pin exact versions (`ngspice=44.2+ds-1`, `libngspice0=44.2+ds-1`) against `snapshot.debian.org` at a fixed timestamp, so a rebuild is the same image | no |
| F5 | The spike image runs everything as `root`, and the official KiCad image defines its own user | `spike/Dockerfile` `USER root` | scope-question | Build half: decide the runtime user (probably a non-root user owning `KANI_HOME`'s use and a writable `HOME` for KiCad's config) | no |
| F6 | The runner had 86 GB free before any cleanup, and the 4.8 GB build used about 10 GB of it. The preinstalled-SDK `rm -rf` is unnecessary today | workflow `cold` job | note | Build half: drop the cleanup and keep only the `df` measurement, so a shrinking runner disk is noticed rather than silently worked around | no |
| F7 | Candidate b is 1.7 GB smaller and 35 s faster warm, but loses KiCad 10 | S3 | note | None; KiCad 10 decides it | no |

**AET-WO-0100 FEEDBACK, promote-or-needs-none:** its findings were promoted at its close. F3 and F9 became `RUL-13`, F4 and F5 became `RUL-14`, and the measurements became `architecture.md` §2.3, through PR #5. F14 (Rust on Furnace's `PATH`) is still open on the furnace777 side, and every `cargo` call this session still exported `PATH`. Nothing else needs promoting.

**Gates for the spike half:** the existing `gates` job passed in run [39355216632](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39355216632). `git diff --stat origin/main -- crates docs/northstar.md docs/architecture.md docs/roadmap.md README.md LICENSE-MIT LICENSE-APACHE docs/inputs` prints nothing. The adversarial pass (§5) and the reduction pass (§5a) belong to the build half's close and have not run.

**STOP.** The build half (`container/Dockerfile`, `container/smoke.sh`, the publish job) has not started and waits for the architect's ruling on F1 and F2.
