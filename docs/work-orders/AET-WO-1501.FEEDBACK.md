# AET-WO-1501 — FEEDBACK

## What was built
- `container/engine.Dockerfile`: `ARG TOOLS_IMAGE` / `FROM ${TOOLS_IMAGE}`, one `RUN` (the context is bind-mounted with `--mount=type=bind,target=/src`, not COPYed) that does `cargo fetch --locked`, then `cargo build --release --locked --offline -p aethelgard-cli` against a throwaway `CARGO_HOME`/target under `/tmp`, installs `/usr/local/bin/aethelgard`, and removes the build directory and fetched registry; the source never enters a layer, since a bind mount is not one. No `ENTRYPOINT`, user stays root.
- `.dockerignore` (repo root): `**/target/`, `.git/`, `docs/`, `spike/`.
- `container/engine-smoke.sh`: version, `validate` (0 and 2), `check drc` (0 and 1), `mcp` `initialize` + `tools/list` over a pipe naming all five tools.
- `.github/workflows/engine-image.yml`: PR and push-to-main `image` job (builds, layer and leftover checks, smoke under `--network none`); `publish` job on `main` only, the sole holder of `packages: write`, smokes the image it then pushes. All actions pinned by the same shas as `tools-image.yml`.
- `container/check-pins.sh`: gains the second file's one shape: `FROM ${TOOLS_IMAGE}` is accepted only after a bare `ARG TOOLS_IMAGE` (no default), and only if `container/IMAGE` beside the file exists and is `image@sha256:<64 hex>`. Every other rule is unchanged.
- One paragraph in `container/fixtures/README.md` on pulling and running by digest.

## §3.3 numbers (PR #36, CI run 42424634787)
- Image size: **4,808,173,457 bytes** (4.81 GB; the tools base is most of it). The first, failed run measured 4,808,173,328.
- Build time: the `image` job took 2m56s with the tools base layers from the GHA cache; the engine layer itself (`cargo fetch` + build) is about 40 s (`Finished release ... in 36.97s`). **Cold** (empty cache) was not measured separately: the first push's job ran 4m01s including a failure and had no engine cache. Warm = 2m56s.
- Smoke output (`--network none`):
```
ok      version
ok      validate.valid
ok      validate.invalid
ok      drc.clean
ok      drc.violation
ok      mcp
engine-smoke: all checks passed
```
- `ci.yml` and `tools-image.yml` untouched and green. `tools-image` also runs on this PR, since `container/**` changed; it ran unmodified.

## Findings
1. **First CI round failed on my own check:** the layer credential grep scanned the whole `docker history`, including the tools base's apt line. Fixed by scoping it to the engine's own two layers (the change in `f779159`). Not a weakened test: the check covers the layers this WO adds.
2. **A literal `FROM ${TOOLS_IMAGE}` cannot satisfy the existing `check-pins.sh` rule** that every `FROM` names `image@sha256:`. §1.3 allows only "gains the second file"; the extension above was the smallest reading that keeps the digest enforced (via `container/IMAGE`). Flagged for the architect to rule on.
3. **Sandbox inside Docker:** the CLI's Landlock/seccomp sandbox was not exercised by the smoke (`validate`, `drc`, `mcp` listing). `check drc` ran `kicad-cli` through it and passed in CI's default Docker seccomp profile, so no `--security-opt` was needed.

## Adversarial pass (§5, targeted)
1. *Published differs from smoke-tested:* `publish` builds `:<sha>`, smokes that exact tag with `--network none`, then `docker push`es the same tag; no rebuild between. Holds by construction; not exercisable off `main`.
2. *Network after fetch / outside the pin:* build is `--offline` after `cargo fetch --locked`; base is the digest from `container/IMAGE`; `check-pins.sh` negative controls run locally: `ARG` with a default, a non-digest `IMAGE`, and a missing `IMAGE` each exit 1 by name. The `--network none` smoke proves the binary needs none at run time.
3. *Leaked `.git` / credentials:* `.dockerignore` excludes `.git/`; checkout uses `persist-credentials: false`; CI asserts `/src`, `/.git`, `/tmp/cargo-build`, `/tmp/target-build` are absent and greps the engine's layers' history for credential words. Full-layer filesystem listing was not done beyond those paths.

## Reduction pass (diff-sized)
Removed the duplicated base-read/pin steps? No: the `image` and `publish` jobs repeat them deliberately, as `tools-image.yml` does, so `publish` depends on nothing from a PR job. Nothing else cut; no `belt-and-suspenders` code touched.

## Gates
`cargo` gates not run: no crate source changed. `bash -n` on both scripts, `check-pins.sh` on both Dockerfiles, reduction pass done (one line above). CI: `gates`, `tools`, both `image` jobs green.

## Operator step
The first publish from `main` creates `ghcr.io/clearforge-llc/aethelgard` as **private**. Make it public in the GitHub UI, as the tools image was.

## Not built (§4 held)
No crates.io publish, no `latest` tag, no multi-arch, no quickstart; no §7 condition arose.

## Rulings and review (architect)
- **R1, caught by the architect's review of PR #36 (defect):** the first build used `COPY . /src` then `rm -rf /src`. A COPY is a layer of its own, so the source shipped in the published image although it was hidden from the final filesystem; the sentence "removes the source ... in the same layer" above was false for that build. Fixed by `RUN --mount=type=bind,target=/src` (no `# syntax=` line).
- **R2, adversarial §5.3 listing:** a CI step now runs `docker save`, asserts exactly one engine layer, and prints that layer's file list (below), failing on anything beyond the binary, its parent directories and an empty `tmp/`. It adds to the history grep and the absent-path check.
- Finding 2 (`FROM ${TOOLS_IMAGE}` in `check-pins.sh`): accepted. Cold build time: accepted as unmeasured, upper bound 4m01s.
- **Handed to AET-WO-0300d (PR #38):** the `gates` failure at 5770bc9, `mcp_tier1::an_immediate_cancel_storm_leaves_no_process_and_no_directory` left one `aethelgard-*` directory. That is 0300c's guardian residual, not 1501's; PR #32 hit it too. No crate source or test was touched here.
