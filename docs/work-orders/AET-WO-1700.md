# AET-WO-1700 — The README quickstart, run verbatim in CI

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1700-quickstart`.
**Author:** Claude (architect) · **Builder:** a cheap builder with a targeted adversarial pass (`docs/roadmap.md` §6). A fresh session on Furnace.
**Phase:** P1. It closes the first half of the launch-readiness clause (`roadmap.md` §5, P1 exit gate).

**Grounds:**
- **The clause:** "On a clean machine with only Docker installed, the README's quickstart reproduces this gate's first two clauses with commands copied verbatim: the reference probe passes ERC and DRC, and the doubly-assigned pin is refused, naming both claimants. The quickstart runs against the published image, which then carries the `aethelgard` binary."
- **What exists:**
  - P1 gate clause 1 holds as a `tools` test (`AET-WO-1302`, PR #60).
  - Clause 2's refusal is `solve`'s `PinClaimedTwice`, exit 2, which carries `error.claimants` naming both (`crates/aethelgard-cli/README.md`).
  - The engine image `ghcr.io/clearforge-llc/aethelgard:<sha>` publishes from every `main` push that touches the engine (`engine-image.yml`). It is public.
- **A quickstart nobody runs drifts.** So the README's block is executed, byte for byte, by CI, on a GitHub runner with Docker and nothing of this repository's toolchain.

> **What this is:** one fenced block in `README.md`, between two marker comments, and a workflow that extracts and runs exactly that block.
>
> It is NOT:
> - the README's status line or any other section (the architect writes those);
> - the crates.io half of the clause, which needs Scotty's token (`roadmap.md` §3);
> - a change to the engine image, to its smoke test, or to any crate.

**Cadence:** build-only. Stop only on §7. One PR, left unmerged.

## 1. Scope

1. **The block.** `README.md` gains a `## Quickstart` section with one ```` ```bash ```` block between `<!-- quickstart:begin -->` and `<!-- quickstart:end -->`, and at most three sentences around it. The block, run by `bash -euo pipefail` in an empty directory:
   1. **Fetches the reference specification** from `raw.githubusercontent.com`, pinned to a commit, and checks its sha256 with `sha256sum -c`.
   2. **Pulls the engine image by digest:** `ghcr.io/clearforge-llc/aethelgard@sha256:…`. Never a tag. The digest must be one `engine-image` published from `main`.
   3. **Runs `aethelgard emit`, then `check erc`, then `check drc --spec --schematic`** with `docker run --rm -v "$PWD:/work" -w /work <image> aethelgard …`. It shows that each exits 0 with `"verdict":"pass"`.
   4. **Makes the doubly-assigned specification from the downloaded one,** with one command readable in the README, such as a `jq` edit run inside the image, never a second download. It shows that `solve` exits 2 naming both claimants.

   Each step's check is visible in the block itself, for example `grep -q '"verdict":"pass"'`, so a reader sees what success looks like. The block needs nothing on the host but Docker, `bash`, `curl` and `sha256sum`.
2. **The workflow,** `.github/workflows/quickstart.yml`:
   - **Triggers:** `pull_request` and `push` to `main` when `README.md` or the workflow changes; a weekly `schedule`; and `workflow_dispatch`.
   - **One job** on `ubuntu-latest`, `permissions: contents: read`, with actions pinned by commit sha as the other workflows pin them.
   - It extracts the text between the markers with `sed` or `awk` and fails if:
     - either marker is missing or repeated;
     - the block is empty;
     - the extracted text is not exactly one fenced `bash` block.
   - It writes the block to a file and runs it with `bash -euo pipefail` in a fresh `mktemp -d`. There is no checkout of the source in that directory, and no Rust.
   - It also asserts two things:
     - the pinned image digest's `org.opencontainers.image.revision` label is a commit reachable from `origin/main`;
     - the specification's pinned commit is reachable from `origin/main`.
3. **The negative control.** In FEEDBACK, record three local runs of the workflow's extraction-and-run step, under `act` or by running the same script directly:
   - a block with one check changed to expect `"verdict":"fail"`, which must fail;
   - a block with the markers removed, which must fail;
   - the real block, which must pass.
4. **`crates/aethelgard-cli/README.md`** gains one line pointing to the quickstart. Nothing else changes there.

## 2. Invariants and fences

- `INV-03`, `INV-06`. No secret, token or credential anywhere, and no `docker login`. The image is public.
- **Fence:**
  - the `## Quickstart` section of `README.md`, between and around its markers, and nothing else in `README.md`;
  - `.github/workflows/quickstart.yml` (new);
  - one line of `crates/aethelgard-cli/README.md`;
  - this work order's FEEDBACK.
- Every other protected surface diffs to empty. `README.md` outside that section diffs to empty too: the architect grants exactly this section.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance

- The `quickstart` job is green on the PR.
- FEEDBACK holds the block's full output from that run.
- FEEDBACK holds the image digest and the commit it was published from.
- The three negative-control results are in FEEDBACK.

## 5. Adversarial pass — targeted

1. Make the block pass while ERC or DRC fails. Every exit code and verdict must be checked inside the block itself.
2. Make the workflow run something other than the README's bytes, for example a second block, a marker inside a code span, or CRLF.
3. Make the quickstart depend on the runner having anything but Docker, `bash`, `curl` and `sha256sum`.

## 5a. Reduction pass — diff-sized

One line in FEEDBACK's gates.

## 7. Flag-and-stop conditions

- A change outside the fence.
- The published image cannot run `emit` and the checks under a plain `docker run`, for example because of the sandbox inside Docker's default seccomp profile. Measure it, report it and stop. That is a ruling, not a workaround.
- A test that can only go green by being weakened.
