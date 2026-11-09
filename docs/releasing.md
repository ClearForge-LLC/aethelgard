# Releasing: the first crates.io publish

`AET-WO-1800` made every publishable crate package and made CI prove it (the `package every publishable crate` step in `.github/workflows/ci.yml`). The publish itself is the operator's, from his own machine. This file holds no token and asks for none in a file or an argument.

## Which machine

The operator's own workstation, never Furnace (Furnace has no credentials by design). Log in once with `cargo login`, which reads the token from the terminal prompt, not from an argument. Use a token scoped to `publish-new` and `publish-update`. Revoke it on crates.io when the last crate is up.

## Before each publish

1. `git switch main && git pull`, then `git status` is clean and `git log -1` is the commit CI passed on.
2. CI's packaging step is green on that commit.
3. `cargo publish -p <crate> --dry-run --locked` succeeds.
4. The crate's dependencies from this workspace are already live: `cargo search <dep>` or the crates.io page shows `0.1.0`. crates.io's index can lag a few seconds behind a publish; wait for it before the next crate.

## The order

Dependencies first. Each command is `cargo publish -p <crate> --locked`:

1. `aethelgard-ir`
2. `aethelgard-kicad-lib`
3. `aethelgard-svd-hal`
4. `aethelgard-eda`
5. `aethelgard-twin`
6. `aethelgard-chaos`
7. `aethelgard-evolve`
8. `aethelgard-cli`

The board crates are `publish = false`; do not publish them. No crate is published under the bare name `aethelgard`. After step 8, `cargo install aethelgard-cli` installs the `aethelgard` binary.

`aethelgard-bridge` (`bridge/`) is never published: it is outside the workspace, `publish = false`, and links the tools image's `libngspice.so.0`, so it ships only in the engine image beside `aethelgard` (`AET-WO-2200`).

## If something goes wrong

A published version cannot be deleted or overwritten. `cargo yank --version 0.1.0 <crate>` stops new projects from resolving it; existing lockfiles keep working. `cargo yank --undo --version 0.1.0 <crate>` reverses it. Fix the cause, bump the version in `[workspace.package]` and the `version` on the path entries in `[workspace.dependencies]`, and publish again from step 1 for every crate that changed.

If a publish stops halfway, the crates already up stay up; resume at the failed crate once the cause is fixed. The packaging step exists so that this does not happen.

## Afterward: trusted publishing

Once the crates exist, a release workflow can publish without a stored token:

1. On crates.io, for each crate: Settings, Trusted Publishing, add a GitHub repository `ClearForge-LLC/aethelgard`, workflow filename (for example `release.yml`), and optionally an environment name.
2. In that workflow, grant `id-token: write` and exchange the OIDC token with the crates.io trusted-publishing action; no secret is stored in the repository.

The workflow itself is a later work order.
