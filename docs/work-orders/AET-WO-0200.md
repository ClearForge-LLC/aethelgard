# AET-WO-0200 — The pinned tools image: KiCad, Renode, ngspice and Kani headless, smoke-tested in CI

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` (named in the kickoff; verify live HEAD before branching).
**Branch:** `wo/aet-0200-tools-image`.
**Author:** Claude (architect) · **Builder:** Claude Code session, tmux `aet` on Furnace, user `forge`, directory `~/aethelgard`.
**Phase:** P0 (Foundation) · **Phase exit gate:** `docs/roadmap.md` §5 P0. This work order owns gate clauses 1 (the workspace gates *inside the container*), 2 (each tool's `--version` inside the container) and 3 (`kicad-cli pcb drc --exit-code-violations` exits 5 on a deliberate violation and 0 on a clean board).
**Grounds:** `docs/northstar.md` §4 `INV-03`, `INV-06`; `docs/architecture.md` §2 (the `kicad-cli` default exit code and Kani's first-run backend download, both unverified), §2.2 (Renode 1.17.0's `stm32h7.repl` and `nucleo_h753zi.repl`; the in-process `libngspice` prior for `OQ-02`), §2.3, §5 (one pinned image used identically by a local builder and by CI; Docker or Nix decided here, with reasons), §4 `RUL-03`, `RUL-05`, `RUL-08`, `RUL-12` (shell is for container build and CI glue only); `docs/roadmap.md` §5 P0.

> **What this is:** one container image that holds every external tool the engine will call, each at a pinned version from a checksummed source, built and exercised in GitHub Actions, with smoke checks that prove each tool *works* rather than that it *exists*. It is NOT the `aethelgard` subcommands that call those tools (`AET-WO-0300`), NOT the trace archive (`AET-WO-0400`), NOT firmware running in Renode (`AET-WO-2100`), and NOT a container runtime on Furnace (human-track, furnace777's decision). Why now: every later work order runs one of these four tools, and none can be measured until they run somewhere reproducible.

**Cadence:** spike first, then STOP for the architect's ruling, then build. One PR for both halves, left unmerged.

**The constraint that shapes everything:** Furnace has no container runtime of any kind and no Nix (measured 2026-10-20: `docker`, `podman`, `buildah`, `nix`, `apptainer` all absent; `forge` has subuid and subgid ranges allocated). Installing one needs root, which no agent has (furnace777 N3). So **every image build and every run of it happens in GitHub Actions**, and every push is a tap on Scotty's phone. Design the spike so one or two pushes answer all of §0. If the furnace777 co-architect lands a rootless runtime for `forge` while you work, use it, and say so in FEEDBACK.

## 0. Spike first — the questions the spike must answer, measured in CI

Push one spike commit carrying a workflow that builds a candidate image and prints the answers. Record every answer, with its CI run URL, in `docs/work-orders/AET-WO-0200.FEEDBACK.md` under "Spike". Then STOP.

- **S1 Packaging.** Dockerfile or Nix. **Prior: a Dockerfile**, base image pinned by digest. Reasons for the prior, held loosely: Furnace has neither runtime, so neither has a local advantage today; GitHub runners ship Docker; KiCad publishes official Docker images; nixpkgs carries no Kani. Confirm or refute with one measured reason.
- **S2 Source and pin for each tool**, each with a checksum or digest:
  - **KiCad.** The official image `kicad/kicad:10.0.7` exists (0.81 GB, published 2026-10-18), and so does Ubuntu 26.04's archive package. Pick one, record the exact version, and confirm `kicad-cli pcb drc` has `--exit-code-violations` and `--format json` in KiCad 10. `architecture.md` §2 measured those flags from the 9.0 documentation only.
  - **Renode 1.17.0.** Find the official artifact and its checksum. It is **not** at `https://builds.renode.io/renode-1.17.0.linux-portable-dotnet.tar.gz` (404, measured 2026-10-20). The candidates are the GitHub release assets, the `antmicro/renode` image at a 1.17.0 tag, and the builds.renode.io directory listing.
  - **ngspice.** The archive version, plus whether the shared library (`libngspice0`) installs alongside, because `OQ-02`'s prior is the in-process library route.
  - **Kani.** `kani-verifier` `0.68.0` (crates.io, 2026-10-20). Run `cargo kani setup` at image build time, because `architecture.md` §5 records that Kani otherwise downloads its backend on first run.
  - **Rust.** `1.99.1`, from `rust-toolchain.toml`.
- **S3 Cost.** Image size; CI build time cold and warm; whether the GitHub Actions cache (10 GB per repository) holds the layers between runs.
- **S4 The unverified exit code.** `kicad-cli pcb drc` on a board with a violation and **without** `--exit-code-violations`: its exit code. This is the number `architecture.md` §2 marked unverified, which is why `RUL-03` makes the flag mandatory.
- **S5 Where the image lives and how CI consumes it.** **Prior:** GHCR at `ghcr.io/clearforge-llc/aethelgard-tools`, published only on pushes to `main` by one job with `packages: write`. Pull requests build the image on the runner and never publish. Consumers pin the image by digest. Confirm `GITHUB_TOKEN` can publish to the organization's registry with no new secret. Propose how the pinned digest gets updated without CI writing to the repository.

**STOP** when §0 is answered: FEEDBACK's Spike section written, the PR open, every number cited with its run. The architect rules by a comment on the PR and a new goal; then the build half starts.

## 1. Scope — the build half, after the ruling

1. `container/Dockerfile` (or the ratified alternative). Every base is pinned by digest, and every download is verified against a checksum committed in the repository. No `latest`, no unpinned tag, no `curl | sh`.
2. `container/smoke.sh`, shell per `RUL-12`. Every check is a command with an expected exit code, and the script exits nonzero on the first one that fails, naming it:
   1. `kicad-cli version`, `renode --version`, `ngspice --version`, and `cargo kani --version` each exit 0.
   2. `kicad-cli pcb drc --exit-code-violations --format json` exits 0 on `container/fixtures/drc-clean.kicad_pcb` and exits 5 on `container/fixtures/drc-violation.kicad_pcb`. The violation board holds one deliberate clearance violation between two nets.
   3. The violation board without the flag: print the exit code (S4's measurement). Assert nothing about it.
   4. Renode, headless and with no monitor, loads `platforms/boards/nucleo_h753zi.repl` from its own install and quits with exit 0. This proves `RUL-08`'s platform loads. No firmware runs.
   5. `ngspice -b` on `container/fixtures/rc-step.cir` (a resistor into a capacitor, driven by a step) prints a 10–90 % rise time within 2 % of 2.2·R·C.
   6. `cargo kani` proves a harness in `container/fixtures/kani-smoke/`, a crate outside the workspace, and **fails** a second harness that carries a seeded arithmetic overflow. A version string proves the binary exists. A failing proof proves the backend works.
   7. The `libngspice` shared object exists at a recorded path.
3. The workspace gates run inside the built image: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`. This is P0 gate clause 1.
4. CI, per the S5 ruling. A job builds the image on any pull request that touches `container/` or the workflow, then runs `smoke.sh` and the workspace gates inside it. Publishing happens only on `main`, from one job, with least permissions. The existing `gates` job stays as it is.
5. `container/fixtures/README.md` says these files are tool-calibration inputs, not board artifacts. `INV-01` governs artifacts generated from a specification, and no specification produces these. Write that down so a later builder does not flag-and-stop on it.
6. Everything measured goes in FEEDBACK as amendment candidates for `architecture.md` §2: versions, digests, sizes, times, the S4 exit code. Do not edit the architecture yourself.

## 2. Invariants (`docs/northstar.md` §4), the slice this work order touches

- **`INV-06` Absence halts.** A missing tool, a failed download or a checksum mismatch fails the build or the smoke run loudly, naming what is missing. No `|| true`, no `continue-on-error`, no fallback to an unpinned source.
- **`INV-03` "Verified" means the gate ran.** Every smoke check executes the tool's real function on a real input. `--version` alone never counts as proof, which is why §1.2 needs a failing Kani harness and a DRC that exits 5.
- **`INV-01`.** The fixtures are not specification artifacts (§1.5).
- **Protected surfaces diff to empty:** the three steering documents, `README.md`, `LICENSE-*`, `docs/inputs/**`, and every crate under `crates/`. This work order changes no Rust.
- **Secrets by name only, and no new secret.** `GITHUB_TOKEN` with least permissions is the only credential. Permissions are `contents: read` everywhere, and `packages: write` appears only on the `main` publish job.
- **One PR, left unmerged.** Push with `gh-write`, and batch the pushes.

## 3. Tests and acceptance, proven not asserted

1. Every §0 answer is recorded in FEEDBACK with the CI run that measured it.
2. `smoke.sh` passes inside the image in CI, every check from §1.2 included, on the PR's head.
3. The workspace gates pass inside the image in CI.
4. The existing `gates` job stays green.
5. `smoke.sh` runs inside the image under `docker run --network none`, and every check still passes. That proves nothing is fetched at run time, which matters most for Kani.
6. CI on the PR is green on the first push after the ruling. A red first run is a finding.

## 4. Scope fence — what is NOT in this work order

- Any `aethelgard` subcommand that runs a tool (`AET-WO-0300`).
- Cap'n Proto, the trace archive, R2 (`AET-WO-0400`).
- Firmware in Renode; a Renode platform description generated from a specification; the lock-step bridge (`AET-WO-2100`, `AET-WO-2200`).
- `rmcp` and the `mcp` subcommand (`AET-WO-1500`).
- Any change under `crates/`.
- Installing a container runtime on Furnace. That is furnace777's, and the operator's.
- Reading the rest of Renode's platform list. The roadmap had it here; it was done in `architecture.md` §2.2 on 2026-10-19.
- Python, or any language but shell and the Dockerfile, outside a tool's own install (`RUL-12`).

## 5. Adversarial pass — break it before calling it done

Hand this to a fresh subagent if you can. Every finding, fixed or not, goes in FEEDBACK with a severity.

1. Remove each tool from the image in turn, or shadow it on `PATH`. `smoke.sh` must fail, naming that tool.
2. Corrupt one committed checksum. The build must fail.
3. Drop `--exit-code-violations` from the DRC check. The check must catch it: the violation board would then exit with the S4 code instead of 5.
4. Run the Kani harnesses against an image built without `cargo kani setup`, then under `--network none`. They must fail, not fetch.
5. Replace a digest-pinned `FROM` with a bare tag. A check in `smoke.sh` or in CI must refuse the unpinned base.
6. Look for assumptions that hold today only by luck: the runner's architecture, its disk space, its preinstalled tools, the cache being warm.

## 5a. Reduction pass — diff-sized

Run the `reduction-pass` skill on the finished diff. One line in FEEDBACK's gates.

## 6. Upward-feedback directive

`docs/work-orders/AET-WO-0200.FEEDBACK.md`, same schema as AET-WO-0100's: `finding · where · type · recommendation · decision-needed`. It must carry what did not work and why, the gates line with CI run URLs, what was deliberately not built, the measured facts for `architecture.md` §2, and a promote-or-needs-none pass over AET-WO-0100's FEEDBACK. Those findings were promoted into `architecture.md` §2.3, `RUL-13` and `RUL-14` at its close; say so.

## 7. Flag-and-stop conditions

Park the branch and say so, in FEEDBACK and in the session, if any of these appears:

- Anything that needs a new secret, a personal access token, or a change to repository or organization settings.
- A tool with no pinnable, checksummed official source.
- `kicad-cli` in the chosen KiCad lacking `--exit-code-violations`.
- Any step that needs `sudo` or a system package on Furnace.
- An image over 10 GB or a cold CI build over 45 minutes. Report it; do not hack around it.
- A smoke check that can only pass by weakening what it checks.

## 8. Kickoff prompt

**Step 1, in its own turn:** send `/goal` alone, read what is in force (AET-WO-0100's achieved goal), and dismiss it.

**Step 2, in its own turn, set the goal for the spike half.** It ends at the STOP, which is the builder's own authority boundary:

```
/goal AET-WO-0200 spike: every §0 question has a measured answer recorded in FEEDBACK with its CI run, on an open unmerged PR from wo/aet-0200-tools-image, and the build half has not started.
```

**Step 3, the kickoff message:** carried by the architect at kickoff, with the live base and anything that changed since this file was written.

The architect sets a second goal for the build half after ruling on the spike.
