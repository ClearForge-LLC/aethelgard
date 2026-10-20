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
| Cold build, empty cache: `ANSWER cold_build_seconds`, the workflow's own clock from before buildx setup to after the build, including cache export and loading into Docker | **257 s** | **216 s** |
| The same, as the `build (cold)` step alone, from the jobs API's step timestamps | 250 s | 208 s |
| Cold job total, with both probe runs | 400 s | 418 s |
| Warm build on a fresh runner, from the Actions cache: `ANSWER warm_build_seconds`, same clock | **100 s** | **65 s** |
| The same, as the `build (warm)` step alone, from the jobs API | 95 s | 60 s |
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

The installs are cheap; moving 4.8 GB around is the cost. *Corrected at the architect's review: the headline seconds are now the `ANSWER` values, and the jobs API's step durations are named as the second source.*

**The cache holds.**
- In both warm builds, every RUN layer reported `CACHED`.
- After the run, the cache API reported 25 entries totalling **2,585,832,496 B (2.6 GB) for both candidates together**, which is 26 % of the 10 GB repository limit.
- The entries are scoped to `refs/pull/7/merge`. GitHub lets a pull request read its own caches and `main`'s, so a `main` build's cache would serve every later PR.

**Runner facts:**
- `ubuntu-latest`: x86_64, 4 CPUs, 15 GiB, Docker 28.0.4.
- 86 GB of disk free before cleanup (`disk_free_before`). The cleanup raised that to 104 GB, and 94 GB remained after building candidate a, so the build used about 10 GB, well inside the 86 GB the runner started with. The `rm -rf` of preinstalled SDKs was not needed (F6). *Corrected at the architect's review: the 94 GB was measured after the cleanup, not from the 86 GB.*

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

### S5 — Where the image lives: measured to the limit a PR can reach; publish confirmed by the first `main` run

*Ruled on PR #7 (F1, option 1): §2 stands, the build half writes the `main`-only publish job, and the architect reads the first publish run and records its digest after merge.*

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
| F7 | Candidate b is 1.7 GB smaller and 35 s faster warm (`ANSWER` 100 s against 65 s), but loses KiCad 10 | S3 | note | None; KiCad 10 decides it | no |

**AET-WO-0100 FEEDBACK, promote-or-needs-none:** its findings were promoted at its close. F3 and F9 became `RUL-13`, F4 and F5 became `RUL-14`, and the measurements became `architecture.md` §2.3, through PR #5. F14 (Rust on Furnace's `PATH`) is still open on the furnace777 side, and every `cargo` call this session still exported `PATH`. Nothing else needs promoting.

**Gates for the spike half:** the existing `gates` job passed in run [39355216632](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39355216632). `git diff --stat origin/main -- crates docs/northstar.md docs/architecture.md docs/roadmap.md README.md LICENSE-MIT LICENSE-APACHE docs/inputs` prints nothing. The adversarial pass (§5) and the reduction pass (§5a) belong to the build half's close and have not run.

**STOP (spike half, 2026-10-20).** Ruled on PR #7 the same day; the build half follows below. At the time: the build half (`container/Dockerfile`, `container/smoke.sh`, the publish job) has not started and waits for the architect's ruling on F1 and F2.

## Build half

Built to the architect's rulings on PR #7 (2026-10-20). The deliverables:

| Path | What it is |
|---|---|
| `container/Dockerfile` | the pinned tools image |
| `container/smoke.sh` | the §1.2 checks |
| `container/check-pins.sh` | the §5.5 refusal of an unpinned base, plus every other drift shape |
| `container/fixtures/` | the spike's fixtures and Kani crate, graduated per the ruling, with the §1.5 README |
| `.github/workflows/tools-image.yml` | the `image` job, and the `publish` job that only `main` runs |

`spike/` and `spike-tools-image.yml` are deleted, so the spike's numbers above, kept inline, are the durable record. The existing `gates` job is untouched. No `container/IMAGE` was created, per the F1 ruling: no digest exists yet, and the first digest PR creates the file.

### Rulings applied

- **F1 / S5.**
  - `publish` runs only when `github.event_name == 'push' && github.ref == 'refs/heads/main'`. It is the only job holding `packages: write`, and its checkout keeps no credentials.
  - It builds the image from the `tools-image` cache and runs `smoke.sh` on that exact image under `--network none`. Only then does it log in, `docker push` it, and write `Published ghcr.io/clearforge-llc/aethelgard-tools@sha256:…` to the job summary. The pushed image is the tested image, not a rebuild of it (B2).
  - The `org.opencontainers.image.source` label is in the Dockerfile.
  - `publish` showed as **skipped** on every PR run and has never run. Its first run is the architect's to read after merge.
- **F2.** `FROM kicad/kicad:10.0.7@sha256:3224d1bf143185ecc1bd3a1082a36f37f0f4e59d707983a16515a845e3737fc6`.
- **F3, measured.** The exit codes differ only by accident, and the ruling's suggested log strings do not discriminate. I ran the pinned Renode 1.17.0 portable binary on Furnace as `forge`, unpacked from its checksummed tarball: no container runtime was involved, and that is the only tool run locally. CI confirmed it in adversarial run [39362225617](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39362225617):
  - A nonexistent `.repl` prints `There was an error executing command … Could not find file '…'.` with **no `[ERROR]` tag**.
  - `System bus created` is printed by `mach create` before the platform is read, so it appears for the failed load too.
  - With stdin at EOF, which is the case in Docker with or without `-i`, the failed load **exits 0** in about 2 s. With an interactive stdin it waits in the monitor until killed.

  So `smoke.sh` asserts the positive case as exit 0 plus `cpu (CortexM)` in the `peripherals` listing plus no `There was an error executing command`. The negative control requires `Could not find file 'platforms/boards/does-not-exist.repl'` and no CPU line. Every PR run records it as `measure renode.missing_platform exit=0`.
- **F4.**
  - The pins are `ca-certificates=20250419`, `curl=8.14.1-2+deb13u7`, `gcc=4:14.2.0-1`, `libc6-dev=2.41-12+deb13u4`, `ngspice=44.2+ds-1` and `libngspice0=44.2+ds-1`, from the normal trixie archive.
  - The base already carries `ngspice`, `libngspice0` and `ca-certificates` at exactly those versions (the spike's apt log reports "already the newest version"), so apt installs only `curl`, `gcc` and `libc6-dev`. The pins on the other three stay, per the ruling: they fail loudly if a future base differs.
- **F5.** The default user is root, and tool state is under `/opt` (`RUSTUP_HOME=/opt/rustup`, `CARGO_HOME=/opt/cargo`, `KANI_HOME=/opt/kani`), with `chmod -R a+rX`.
- **F6.** The SDK cleanup is gone. `ANSWER runner disk_free_before` stays, and read **86G** in run [39362225677](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39362225677).
- **Exact keys.** Every smoke result is an exact exit code plus an exact line or delimited string: `is_line` with `grep -qxF` for `10.0.7`, `Renode v1.17.0`, and `Kani Rust Verifier 0.68.0 (cargo plugin)`. Nothing is asserted by exclusion alone. The two `lacks` checks only ever add to a positive check on the same output.

### §3 acceptance, in CI

All of these are at head `d9eaf49`, the first push after the ruling.

| §3 | Check | Result | Run |
|---|---|---|---|
| 1 | Every §0 answer recorded with its run | yes, Spike section above | 39355216653 |
| 2 | `smoke.sh` passes inside the image, every §1.2 check | **pass**: 12 `ok` lines (every §1.2 check, including the Renode negative control) and `smoke: all checks passed` | [`tools-image` 39362225677](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39362225677), job [`image`](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39362225677/job/118430786671) |
| 3 | The workspace gates inside the image | **pass**: fmt, clippy `--locked -D warnings`, and test `--locked` with 10 + 8 + 18 tests | same job |
| 4 | The existing `gates` job | **pass** | [`ci` 39362225338](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39362225338) |
| 5 | `smoke.sh` under `docker run --network none`, every check | **pass**: the identical 12 `ok` lines | same job, step "smoke with --network none" |
| 6 | Green on the first push after the ruling | **yes**: `tools-image` and `ci` both green on `d9eaf49` | above |

The head carrying this FEEDBACK reruns `tools-image` and `ci`. Their result is on the PR.

**Measured in the build** (amendment candidates for `architecture.md` §2 and §5):

| Fact | Value |
|---|---|
| Image | 4,802,171,503 B (`ANSWER image_bytes`) |
| `build` step, cold for the new `tools-image` cache scope | 212 s |
| `smoke` | 8 s, and 7 s under `--network none` |
| Workspace gates inside the image | fmt 0 s, clippy 7 s, test 10 s |
| `kicad-cli pcb drc` without the flag, KiCad 10.0.7 | exits **0** on the violation board (`measure kicad-cli.drc.violation_without_flag exit=0`) |
| ngspice rise | `2.197224e-03` s |
| Renode, nonexistent platform, stdin at EOF | exits 0 |
| `cargo install --locked kani-verifier` | finishes in 2.6 s, because the crate is a thin driver |

- **Cross-run cache, from spike run 2** ([39356460448](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39356460448), triggered by the FEEDBACK-only push): the cold jobs of a *second* run took **68 s and 46 s** against run 1's 250 s and 208 s, with all four RUN layers `CACHED` in both. The cache stayed at 2,585,841,953 B. So the Actions cache holds the layers between runs, not only between jobs.
- **Without `cargo kani setup`** (adversarial run [39362762915](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39362762915), job `nosetup`): Kani's first run tries to download `kani-0.68.0-x86_64-unknown-linux-gnu.tar.gz` from GitHub. Under `--network none` it fails (`curl: (6) Could not resolve host: github.com`, exit 1). With the network it succeeds. Build-time setup is what keeps the image offline, which `architecture.md` §5 predicted and this measures.

### Adversarial pass (§5)

A fresh Sonnet subagent ran it in its own worktree.
- **Half 1:** local attacks on `check-pins.sh`, `smoke.sh` logic, and the workflow.
- **Half 2:** wrote `.github/workflows/adversarial-0200.yml` for the attacks that need a built image. I reviewed it, fixed three stub version strings so they matched the tightened smoke script, pushed it, and **deleted it on this head**.

Every claim below was read from the runs, not the report.

- **§5.1, tools removed or shadowed.** All four tools were each removed from the built image at run time, as were `libngspice.so.0` and `/opt/kani`. Each failed with a line naming it, for example `FAIL    cargo-kani: not on PATH` and `FAIL    libngspice: missing /usr/lib/x86_64-linux-gnu/libngspice.so.0`. Each tool was also shadowed by a stub printing the exact version, and each stub failed at the real-function check:
  - `kicad-cli.drc.violation: exit 0, expected 5`
  - `ngspice.rc_step: no 'rise =' measurement`
  - `renode.nucleo_h753zi: output lacks 'cpu (CortexM)'`
  - `kani.add_in_range: output lacks 'VERIFICATION:- SUCCESSFUL'`

  A "smart" Renode stub that prints the CPU line for any platform was caught only by the negative control. Run 39362225617, job `runtime`, all `caught`.
- **§5.2, corrupted checksums.** Flipping one hex digit of the Renode, rustup-init and Kani sha256 values each failed the build with `<path>: FAILED`. Jobs `checksum (…)`, all `caught`.
- **§5.3, dropping `--exit-code-violations`** from a copy of the smoke script failed it with `kicad-cli.drc.violation: exit 0, expected 5`. `caught`.
- **§5.4, an image without `cargo kani setup`.**
  - The first try failed for a harness reason: the variant's own `chmod … /opt/kani` hit a directory that only setup creates, run 39362225617.
  - With the harness fixed (`2cc89b9`), run [39362762915](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39362762915) shows the harnesses fail under `--network none` rather than fetch, and the full smoke fails at `kani.`. Both `caught`.
- **§5.5, a bare-tag `FROM`.** Bare tag, no tag, `:latest`, a short digest and an appended unpinned `FROM` were all refused by `check-pins.sh`. Job `pins`, all `caught`.
- **§5.6, luck.**
  - The image is amd64-only (ruled), and `smoke.sh` and the download URLs say `x86_64` outright (B9).
  - The runner had 86 GB free, and the build needs about 10.
  - Nothing relies on a runner tool beyond Docker and buildx.
  - A cold cache is the 212 s case above, and a warm one the 68 s case.

### Build-half findings

| # | finding | where | type | recommendation | decision-needed |
|---|---|---|---|---|---|
| B1 | The first `check-pins.sh` was a line grep that knew only the shapes the Dockerfile already used. It was bypassed 26 ways locally: lowercase or indented `FROM`, `COPY --from=img:tag`, `curl -O`, `> file`, `wget`, `ADD <url>`, `\|\| true` after a checksum, `apt-get -y install`, `pkg=*`, and `cargo install` without `--locked`. **High. Fixed:** it now accepts only known-safe shapes and names the line it refuses, and all 26 are refused. Shape-strict by design: a future Dockerfile edit must keep these shapes or extend the checker | `container/check-pins.sh` | bug | Done | no |
| B2 | `publish` rebuilt the image instead of pushing the one smoke-tested, so the published digest was not provably the tested one. **Medium. Fixed:** `publish` smoke-tests the exact image it then pushes | `.github/workflows/tools-image.yml` `publish` | bug | Done | no |
| B3 | Renode's exit code does not reflect a failed load, and the negative control was by exclusion only. **Low. Fixed:** it now requires `Could not find file '…'` (F3 above) | `container/smoke.sh` §4 | bug | Done | no |
| B4 | Version checks were substring matches (`10.0.7` would match `10.0.70`). **Low. Fixed:** exact lines | `container/smoke.sh` §1 | bug | Done | no |
| B5 | `publish`'s checkout persisted the `packages: write` token in `.git/config`. **Low. Fixed:** `persist-credentials: false` | `.github/workflows/tools-image.yml` | risk | Done | no |
| B6 | Running the workspace gates in the image as root leaves a root-owned `target/` in the runner's checkout | `tools-image.yml` gate steps | note | None while runners are ephemeral; revisit if a self-hosted runner is ever used | no |
| B7 | A same-repository PR that edits `tools-image.yml` could grant itself `packages: write` and drop the `if:`, because same-repo PRs run the workflow as edited. Only review and branch protection stop that, not file content | repository | risk | Branch protection requiring review on `main`; it is the operator's setting, and nothing in this WO needs it changed | no |
| B8 | The Rust 1.99.1 components and Kani's pinned nightly are fetched by rustup and verified by its channel manifest's sha256 over TLS, not by a checksum committed here. `cargo install --locked` is verified by the crates.io index checksum | `container/Dockerfile` | risk | Accept. The toolchain versions are pinned, and the published digest freezes the result | no |
| B9 | `x86_64` appears directly in the rustup-init and Kani URLs and in `smoke.sh`'s `libngspice` path | `container/Dockerfile`, `container/smoke.sh` | note | Consistent with the amd64-only ruling. An arm64 image would revisit all three | no |
| B10 | The clearance check counts the violation's type, not which two nets it names | `container/smoke.sh` §2 | note | Enough for calibration | no |
| B11 | A stub written to special-case the fixture names could still pass `smoke.sh`. Shell cannot defend against a tool built to lie about these exact inputs | `container/smoke.sh` | note | None | no |
| B12 | The `adversarial-0200` harness had one defect, the `nosetup` variant's `chmod`, found and fixed in one push | `.github/workflows/adversarial-0200.yml` (deleted) | note | None | no |

### Reduction pass (§5a)

A fresh read-only subagent proposed 8 cuts over `9c6b4d7..1c9ef8b`, and I checked each against the rulings and the runs.

**Taken:**
- the Dockerfile `SHELL` line, since every RUN is an `&&` chain and the only pipe ends in `sha256sum`;
- the after-build disk echo;
- the duplicate digest echo;
- the kani-smoke `[lints]` block and its doc comment, which `container/fixtures/README.md` already states.

**Rejected:**
- **P1, dropping the `ngspice`, `libngspice0` and `ca-certificates` pins:** the F4 ruling names them.
- **P3, the `description` and `licenses` labels:** GHCR displays both.
- **P6, the `command -v` loop:** it is what names `cargo-kani` exactly when it is missing, as the §5.1 run shows.
- **P7, the empty-`rise` guard:** it tells "no measurement" apart from "out of tolerance".
- **P8a, trimming the fixtures' layer tables:** an untested load risk on calibration inputs.
- **P8d, a smoke comment.**

### Gates line

- `ci` [39362225338](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39362225338): green.
- `tools-image` [39362225677](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39362225677): green, with `publish` skipped.
- `adversarial-0200`:
  - [39362225617](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39362225617): 23 caught; the `nosetup` job never reached its attacks because of its harness defect.
  - [39362762915](https://github.com/ClearForge-LLC/aethelgard/actions/runs/39362762915): green, all caught.
- `git diff --stat origin/main -- crates docs/northstar.md docs/architecture.md docs/roadmap.md README.md LICENSE-MIT LICENSE-APACHE docs/inputs` prints nothing.
- **Reduction pass (diff):** −10 lines net, 2 duplicates removed (the digest echo and the kani-smoke doc comment), 3 dead paths removed (the `SHELL` line, the after-build disk echo, and the kani-smoke `[lints]` block), 0 protected kept, 1 finding (P8a).

### What did not work, and why

- **`check-pins.sh` version 1** (B1). I wrote a checker that confirmed the Dockerfile in front of me instead of refusing the Dockerfiles that could follow it. The adversarial pass found 26 bypasses in minutes. The allowed-shapes rewrite is the fix, and the lesson is to write a gate against the attacker, not against the current input.
- **The ruling's suggested Renode strings** (F3) would have passed a failed load. Measuring the real binary first, rather than trusting either the suggestion or my first guess (`System bus created`), is what found that.
- **The `nosetup` attack's first run** failed on its own harness (B12), which cost one push.
- **My wait loop** for push 1's runs exited early on a bad condition. I caught it from its output, and nothing was misread.

### What was deliberately not built (§4 held)

Confirmed against the diff:
- no `aethelgard` subcommand runs a tool;
- no Cap'n Proto, trace archive or R2;
- no firmware in Renode, no generated platform description, and no lock-step bridge;
- no `rmcp`;
- nothing changed under `crates/`;
- no container runtime on Furnace;
- no Python or any language but shell, YAML and the Dockerfile.

`container/IMAGE` was not created (F1 ruling).

**Promote-or-needs-none:**
- AET-WO-0100's findings were promoted at its close into `architecture.md` §2.3, `RUL-13` and `RUL-14` (PR #5). Its F14 (Rust on Furnace's `PATH`) is still open on the furnace777 side.
- For the architect to promote from this WO:
  - into §2: the S1–S4 facts and the measured build facts above;
  - into §5: candidate a and amd64-only, the publish flow, and that a failed Renode load exits 0 with stdin at EOF;
  - the S5 confirmation, read from the first `main` publish run.
