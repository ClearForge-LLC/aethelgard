# AET-WO-1801 — The 0.2.0 release: every workspace crate packages at 0.2.0, the three never-published crates ready, and Scotty's publish checklist

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1801-release-0.2.0`.
**Author:** Claude (architect) · **Builder:** cheap tier, with a targeted adversarial pass on the packaging.
**Phase:** release, written 2026-11-17 at Scotty's go. **Cadence: prepare now; the bump lands in a quiet window, after `AET-WO-2302` merges (§1.4).**

**Grounds:**
- **README:** "The crates.io release, version 0.1.0, predates P2 and cannot read today's specification. 0.2.0 will replace it."
- **Roadmap §7.1, the 0.2.0 row:**
  - three crates have never been published: `aethelgard-evolve`, `aethelgard-pincheck` and `aethelgard-renode-models` (crates.io answered 404 for each, checked 2026-11-16 and 2026-11-17);
  - the engine version is written into every emitted board, so the bump moves golden bytes;
  - the archive header's version (`2205` F4) and the `--no-verify` collision (`3200` F18) close with the bump.
- **`docs/releasing.md`** (`AET-WO-1800`) holds the publish order and the operator's procedure. CI's `package every publishable crate` step proves packaging.
- **`RUL-21`:** a release is a pinned snapshot with no update promise.

> **What this is:** everything up to the publish. That covers the metadata, the version bump with every golden byte it moves, packaging proven at 0.2.0, and an exact checklist Scotty runs from his own workstation with his own token.
>
> It is NOT:
> - the publish itself (Scotty's; no credential reaches Furnace);
> - the README's install line (an architect docs PR, after the publish);
> - any behaviour change.

## 1. Build

1. **Measure first.**
   - List every workspace member, with its `publish` setting, its packaged size (`cargo package --list`, plus the `.crate` size) and its metadata (description, licence, repository, readme, keywords, categories).
   - crates.io refuses a crate over 10 MB: give `aethelgard-kicad-lib`'s and `aethelgard-renode-models`' sizes.
   - Confirm the licence of every file `aethelgard-renode-models` packages (C# models and any vendored model code), with its source.
   - **STOP** if any crate cannot be published as it stands, with the reason.
2. **The three new crates:** complete their metadata to match the others. Each must package and build from its package alone, as `1800` proved for the rest.
3. **`docs/releasing.md`:** update it for an update release. That means:
   - the order, which already lists all ten;
   - a check that each dependency is live at 0.2.0 before its dependant;
   - what to do if crates.io's index lags;
   - the token scope.

   Then `RELEASE-0.2.0.md`, a one-page checklist for Scotty: the exact commands in order, what each should print, and how to verify the result (`cargo install aethelgard-cli --version 0.2.0`, then `aethelgard validate < examples/sentinel-01-minimal.json`).
4. **The bump, in a quiet window.** Prepare it on the branch, but push the bump commit only when the architect says the window is open: after `2302` merges, with no builder mid-regeneration.
   - `[workspace.package] version` and every path dependency's `version` go to 0.2.0.
   - Regenerate every golden byte the version moves, in the same commit. List each file in FEEDBACK.
   - `2205` F4 and `3200` F18 close here. Say how.
5. **Release notes:** `CHANGELOG.md`, or a section of `RELEASE-0.2.0.md`. It names what 0.2.0 can do that 0.1.0 could not, with each item cited to the work order that built it. Specification 12 is read; 0.1.0 read version 1.

## 2. Acceptance

- CI's packaging step is green at 0.2.0 for all ten crates.
- A clean `cargo install --path crates/aethelgard-cli` runs `validate` on today's Sentinel-01 with exit 0.
- Every golden byte the bump moved is listed, and nothing else changed.
- `RELEASE-0.2.0.md` is complete. Each command in it has been dry-run (`cargo publish --dry-run --locked`), where a dry run is possible before its dependencies are live, and the rest are named.
- **The CLAUDE.md gates pass.** CI is green, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-07`; `RUL-11`, `RUL-12`, `RUL-21`.
- **Fence:**
  - every `Cargo.toml`;
  - `Cargo.lock`;
  - `docs/releasing.md` and `RELEASE-0.2.0.md`;
  - the golden files the version moves;
  - FEEDBACK.

  No behaviour.
- **Running beside you:**
  - `2302` (CLI, chaos), `1312` (ir, eda) and `3401` (examples, a board).
  - Your bump touches every crate's manifest and their goldens: push it only in the window the architect opens.
- One PR, left unmerged. Write `~/.cache/aet-1801/pr-title-body.md`.

## 5. Adversarial pass, targeted

On the packaging only:
- **A crate that builds in the workspace but not from its package** (a path, an `include_str!` outside the crate, a missing file).
- **A dependency published after its dependant in the order.**
- **A crate that packages a file it may not license.**

## 7. Flag-and-stop conditions

- **§1.1 finds a crate that cannot be published.**
- **A crate is over 10 MB.**
- **The bump would move a byte that is not the version.**
