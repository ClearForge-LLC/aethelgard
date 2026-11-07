# AET-WO-1501 — The engine image: `aethelgard` on top of the pinned tools, published from `main`

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` after `AET-WO-0300c` has merged (named in the kickoff; verify live HEAD before branching).
**Branch:** `wo/aet-1501-engine-image`.
**Author:** Claude (architect) · **Builder:** a cheap builder (`docs/roadmap.md` §6), in a fresh session on Furnace.
**Phase:** P1 · **Gate clause served:** the launch-readiness clause. Its quickstart "runs against the published image, which then carries the `aethelgard` binary". This work order makes that image exist. The quickstart itself is later work.

**Grounds:**
- `docs/architecture.md` `RUL-15`: how the tools image is built, published and consumed. This work order follows it for a second image and changes nothing about the first.
- `docs/architecture.md` §6, with `AET-WO-0300c` merged: the sandbox lifecycle is now safe to ship.
- `docs/northstar.md` §4: `INV-06`.
- The seams:
  - `container/Dockerfile` and `container/IMAGE`, the tools image and its digest;
  - `.github/workflows/tools-image.yml`, its build, smoke and publish jobs, to copy the shape of;
  - `crates/aethelgard-cli`.

> **What this is:** a second image, `ghcr.io/clearforge-llc/aethelgard`. It is the tools image, pinned by digest, plus the `aethelgard` binary built from this repository's source inside that same image. It is smoke-tested, and published only from `main`.
>
> It is NOT:
> - any change to the tools image, its Dockerfile, its pins, or its workflow;
> - a crates.io publish;
> - the quickstart README;
> - an `ENTRYPOINT` that changes how the tools run.

**Cadence:** build-only. Stop only on §7. One PR, left unmerged.

## 1. Scope

1. **`container/engine.Dockerfile`:**
   - `FROM` the exact digest in `container/IMAGE`. The workflow passes it as a build argument read from that file, and is never handed a tag;
   - `COPY` the workspace source, with a `.dockerignore` that excludes `target/`, `.git/`, `docs/` and `spike/`;
   - `cargo build --release --locked -p aethelgard-cli`, offline after the dependency fetch, with the image's own pinned Rust;
   - install the binary at `/usr/local/bin/aethelgard`, then drop the source and the build directory in the same layer;
   - no `ENTRYPOINT` change, and the default user stays root, as `RUL-15` rules for the tools image.
2. **`container/engine-smoke.sh`,** run under `--network none`:
   - `aethelgard --version`;
   - `aethelgard validate --file` on both checked-in examples, exit 0 and exit 2;
   - `aethelgard check drc` on `drc-clean.kicad_pcb` and `drc-violation.kicad_pcb`, exit 0 and exit 1;
   - `aethelgard mcp` answering `initialize` and `tools/list` over a pipe, listing the generated tools.
3. **`.github/workflows/engine-image.yml`,** in the shape of `tools-image.yml`:
   - **A pull request** touching `crates/**`, `Cargo.*`, `container/engine*` or this workflow builds the image and runs the smoke. It never publishes.
   - **A push to `main`** does the same, and then a `publish` job, the only one holding `packages: write`, pushes `ghcr.io/clearforge-llc/aethelgard:<sha>`, the very image it has just smoke-tested.
   - **The digest** goes into the run summary. CI never writes to the repository.
   - **Pinning:** every action is pinned by commit sha, as `tools-image.yml` pins them. `check-pins.sh` is extended to check `engine.Dockerfile` as well. That is its one permitted change: it gains the second file, and every existing rule stays as it is.
4. **`container/fixtures/README.md` or the CLI README:** one paragraph on how to pull and run the engine image, by digest.

## 2. Invariants and fences

- **`INV-06`:** a missing or unreadable `container/IMAGE` fails the build by name.
- **`RUL-15` holds:** only the publish job on `main` can push, and only the image it smoke-tested.
- **Protected surfaces diff to empty:**
  - the steering documents, `README.md`, `LICENSE-*`, `docs/inputs/**`;
  - `container/Dockerfile`, `container/IMAGE`, `container/smoke.sh`, and `container/check-pins.sh` except as §1.3 allows;
  - `.github/workflows/tools-image.yml` and `ci.yml`;
  - every crate's source.
- **No secret.** `GITHUB_TOKEN` with `packages: write` in the publish job is the only credential, as in `RUL-15`.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route:** a `grep -o` redirect from your transcript into `gh-write prep`'s file, never the Write tool.

## 3. Acceptance

1. On the PR, the new workflow builds the engine image and passes the smoke under `--network none`. This is proven in CI, since Furnace has no container runtime (`docs/roadmap.md` §6).
2. `ci.yml` and `tools-image.yml` are untouched and stay green.
3. FEEDBACK records:
   - the image's size;
   - the build time, cold and warm;
   - the smoke's output lines.

## 4. Scope fence

Publishing to crates.io; changing the package's visibility, which is the operator's step in the GitHub UI; a `latest` tag; multi-arch builds (`RUL-15`: amd64 only); the quickstart.

## 5. Adversarial pass — targeted

1. Make the published image differ from the smoke-tested one.
2. Make the build depend on the network after the fetch, or on anything outside the pinned digest and the locked dependencies.
3. Leak the repository's `.git/` or any credential into an image layer. Inspect every layer with `docker history` and a filesystem listing.

## 5a. Reduction pass — diff-sized

Run the `reduction-pass` skill on the finished diff. One line in FEEDBACK's gates.

## 6. Upward-feedback directive

`docs/work-orders/AET-WO-1501.FEEDBACK.md`, same schema. It carries:
- the numbers in §3.3;
- the gates line;
- **one operator step, listed for the operator.** The first publish creates the `aethelgard` package as private. It is made public in the GitHub UI, as the tools image was.

## 7. Flag-and-stop conditions

Park the branch and say so, in FEEDBACK and in the session, if any of these appears:
- A design that needs a change to the tools image, its workflow, or `ci.yml`.
- A publish path reachable from a pull request.
- Any credential beyond `GITHUB_TOKEN`, or any repository or organization settings change.
- A test that can only go green by being skipped, ignored, or weakened.

## 8. Kickoff prompt

**Step 1, in its own turn:** send `/goal` alone, read what is in force, and dismiss it.

**Step 2, in its own turn:**

```
/goal AET-WO-1501: §1 is built and the engine image builds and passes its smoke in CI on the PR, or a §7 stop is recorded in FEEDBACK; the adversarial and reduction passes and FEEDBACK are written; one open unmerged PR from wo/aet-1501-engine-image.
```

Confirm that the "Goal set:" echo matches.

**Step 3, the kickoff message:** carried by the architect at kickoff, with the live base.
