# Releasing

`AET-WO-1800` made every publishable crate package and made CI prove it, in the `package every publishable crate` step of `.github/workflows/ci.yml`. `AET-WO-1801` extended this file from the first publish (0.1.0) to every later release, and `AET-WO-1802` added the engine's identity: the version between releases and the commit (below).

The publish itself is the operator's, from his own machine. This file holds no token, and asks for none in a file or an argument. Each release has a one-page checklist at the repository root, `RELEASE-<version>.md`, with the exact commands in order. This file is the procedure behind it.

A published release is a pinned snapshot with no promise of updates (`RUL-21`).

## Which machine, which token

- **The machine:** the operator's own workstation, never Furnace (Furnace has no credentials by design).
- **Logging in:** run `cargo login` once. It reads the token from the terminal prompt, not from an argument.
- **The token's scopes:**
  - `publish-update`, which every release needs;
  - `publish-new`, only when the release brings a crate that crates.io has never seen. 0.3.0 does: `aethelgard-hwdef` answered 404 on 2026-11-26 (0.2.0 brought `aethelgard-evolve`, `aethelgard-pincheck` and `aethelgard-renode-models`).
- **Crate scopes:** limit the token to the eleven crate names below, or to the pattern `aethelgard-*`.
- **Expiry:** give it a short expiry. Revoke it on crates.io when the last crate is up.

## Before the first publish of a release

1. `git switch main && git pull`. `git status` is clean, and `git log -1` is the commit CI passed on: the release's bump commit, or a later one.
2. CI's packaging step is green on that commit.
3. `[workspace.package] version` in `Cargo.toml` is the release's version, with no `-dev`, and so is the `version` on every path entry in `[workspace.dependencies]`. A `-dev` version is `main` between releases: stop.
4. **`cargo publish --workspace --dry-run --locked` succeeds, run in a fresh cargo home and target directory:** `d=$(mktemp -d); CARGO_HOME="$d/home" CARGO_TARGET_DIR="$d/target" cargo publish --workspace --dry-run --locked; rm -rf "$d"`. Cargo treats a name and version as immutable. It reuses an earlier dry run's unpacked copy and compiled crates of the same version even when the code has changed since, which can make the dry run fail, or pass, for the wrong code (`AET-WO-1801` §9). It needs no token. It packages every crate, verifies each against the others' local packages (a version not yet on crates.io resolves to them), and stops before uploading. It prints `Uploading <crate> v<version>` and then `warning: aborting upload due to dry run` for each of the eleven.

## The order

Dependencies first. The order is the dependency graph's: no crate comes before one of its dependencies, and CI's packaging step lists the same order. Each command is `cargo publish -p <crate> --locked`:

1. `aethelgard-ir`
2. `aethelgard-kicad-lib`
3. `aethelgard-renode-models`
4. `aethelgard-svd-hal` (needs 1)
5. `aethelgard-eda` (needs 1, 2)
6. `aethelgard-twin` (needs 1, 3, 4)
7. `aethelgard-chaos`
8. `aethelgard-evolve` (needs 4, 7)
9. `aethelgard-pincheck` (needs 1)
10. `aethelgard-hwdef` (needs 1; `AET-WO-4400b`, first published at 0.3.0)
11. `aethelgard-cli` (needs 1, 4, 5, 6, 7, 8, 9, 10)

`aethelgard-svd-hal`'s dev-dependencies on `aethelgard-cli` and `aethelgard-twin` are path-only, so `cargo package` drops them from the published manifest, and they do not bind the order.

The board crates are `publish = false`; do not publish them. No crate is published under the bare name `aethelgard`. After step 11, `cargo install aethelgard-cli` installs the `aethelgard` binary.

`aethelgard-bridge` (`bridge/`) is never published. It is outside the workspace, `publish = false`, and versioned on its own. It links the tools image's `libngspice.so.0`, so it ships only in the engine image beside `aethelgard` (`AET-WO-2200`).

## Before each crate

**Every dependency it names is live at the release's version.** For each one, check `https://crates.io/api/v1/crates/<dependency>/<version>`, which needs no login. The step numbers above say which crates to check.
- A live version answers HTTP 200.
- One crates.io does not have answers 404.

From a shell: `curl -s -o /dev/null -w '%{http_code}\n' https://crates.io/api/v1/crates/aethelgard-ir/0.2.0`.

## If crates.io's index lags

`cargo publish` waits after uploading until the new version appears in the index, printing `Waiting for <crate> v<version> to be available at registry crates-io` and then `Published <crate> v<version> at registry crates-io`. If it gives up first, it prints a timeout warning. The upload has still succeeded: do not publish that crate again.

Before the next crate, wait until the check above answers 200; a minute or two is usual. If the next crate's publish says `failed to select a version for the requirement <dependency> = "^<version>"`, the index has not caught up: wait, then run the same command again. Nothing was uploaded.

## If crates.io rate-limits a new name

crates.io limits how fast new crate names can be created, and the 0.1.0 publish ran into the limit once (2026-11-09). If a crate's first publish is refused with a message that names a rate limit and a time to wait, nothing was uploaded. Wait that long, then run the same command again.

## If something goes wrong

A published version cannot be deleted or overwritten.
- **Yanking:** `cargo yank --version <version> <crate>` stops new projects from resolving it, while existing lockfiles keep working. `cargo yank --undo --version <version> <crate>` reverses it.
- **To publish a fix:**
  - fix the cause;
  - bump the version in `[workspace.package]` and the `version` on the path entries in `[workspace.dependencies]`;
  - regenerate what the bump moves (the release's work order lists it);
  - publish again from step 1 for every crate that changed.

If a publish stops halfway, the crates already up stay up. Resume at the failed crate once the cause is fixed. The packaging step and the workspace dry run exist so that this does not happen.

## After the release

- Run the checklist's verification: install from crates.io in a clean root, then `validate` the reference specification.
- Revoke the token.
- The README's install line is the architect's to update.
- **The commit right after the release sets `main` to the next version with `-dev`** (below), by its own pull request.

## The engine's identity: the version and the commit

`AET-WO-1802`. Two facts name a build, and each is written only where it cannot make a committed byte stale.

- **The version** is `[workspace.package] version`. It is written into committed and hashed bytes: `FAB-NOTES.md`'s `Generated by aethelgard <version>` line, the KiCad files' `generator_version`, the hwdef header, and the verdict's `engine_version`, which `MANIFEST.json` binds through `verdict_sha256`. So it moves only at a release and in the commit right after it, and each time it moves, the committed packages and the hwdef fixtures move with it (the release's work order lists them).
- **Between releases, `main` carries the next version with a `-dev` suffix** (after 0.3.0, `0.4.0-dev`). A package built from `main` then says `aethelgard 0.4.0-dev`, never a published version it is not. The release's bump removes the suffix. **A `-dev` version is never published:** the checklist's first step refuses one. CI's packaging step admits no suffix but `-dev`, from the pull request that first sets one (`AET-WO-1802` (4)).
- **The commit** is written only at run time, never into a committed or hashed byte: `aethelgard --version` (`aethelgard 0.3.0 (commit <sha>, clean)`), `doctor`'s `engine_commit` and `engine_tree`, the MCP server's info, and a campaign run's archive header. `crates/aethelgard-cli/build.rs` records it:
  - from git, when the crate is built in its own checkout: the commit, and `dirty` if `git status` names any change or untracked file;
  - from `.cargo_vcs_info.json`, which `cargo package` writes, for a build from a published crate: the commit it was packaged from;
  - otherwise `commit unknown`, never a default. The engine image is built with neither, so inside it `--version` says `commit unknown`. The image's commit is its `org.opencontainers.image.revision` label (`engine-image.yml`).
- So a release is published from a clean checkout of the bump commit (`git status` clean, step 1 above), and `cargo install aethelgard-cli --version <v>` prints that commit, clean.

## Why CI's packaging step keeps `--no-verify`

`cargo package` verifies each crate by building it against its siblings. A sibling's version that is already on crates.io resolves to the published copy, not the local one (`AET-WO-3200` F18). Between a release's bump and its publish, no copy at the new version exists, so verification passes; `AET-WO-1801` measured this at 0.2.0. After the publish, any later change carries the same version until the next bump, so the collision returns.

CI therefore keeps `--no-verify`. Its check is the next part of the step: it builds the CLI offline from the unpacked packages alone, with every sibling patched to them. The workspace dry run above is the release-time check.

## Afterward: trusted publishing

Once the crates exist, a release workflow can publish without a stored token:

1. On crates.io, for each crate, open Settings, then Trusted Publishing. Add the GitHub repository `ClearForge-LLC/aethelgard`, the workflow's filename (for example `release.yml`), and optionally an environment name.
2. In that workflow, grant `id-token: write` and exchange the OIDC token with the crates.io trusted-publishing action. No secret is stored in the repository.

The workflow itself is a later work order.
