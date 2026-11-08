# AET-WO-1304 — KiCad's vendored libraries move into their own CC-BY-SA crate

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` at or after `AET-WO-1302`'s merge, named in the kickoff.
**Branch:** `wo/aet-1304-kicad-lib-crate`.
**Author:** Claude (architect) · **Builder:** a cheap builder with a targeted adversarial pass (`docs/roadmap.md` §6). A fresh session on Furnace.
**Phase:** P1, written 2026-11-08 at Scotty's licensing direction. It is housekeeping. It serves the launch clause, because crates.io shows a crate's `license` field to everyone who considers depending on it.

**Grounds:**
- **Scotty, 2026-11-08:** "I want to build this and place it on the wild to help fellow devs and builders. No matter who they are or who they work for." The code stays `MIT OR Apache-2.0`, and nothing a user depends on for the engine's own logic should carry a share-alike term.
- **`RUL-11`, as amended 2026-11-08.** Today `aethelgard-eda`'s `license` is `(MIT OR Apache-2.0) AND CC-BY-SA-4.0`, only because 17 KiCad 10.0.7 files are vendored inside it (ruled D1 on PR #40). Moving them into a crate of their own makes `aethelgard-eda` pure `MIT OR Apache-2.0`. The share-alike term then sits only on the crate that holds KiCad's data.
- **The integrity rule from D1.** Each file's sha256 is checked before any is used. **The pins stay in the consumer**, `aethelgard-eda`. A data crate that pinned its own files would be checking itself.

> **What this is:** a move of `data/kicad-10.0.7/` and the `include_bytes!` table, with no change to a single emitted byte.
>
> It is NOT:
> - a change to which files are vendored, their bytes or their hashes;
> - a change to `emit`'s output, the gate, or any rule file;
> - the image's missing KiCad licence text (`AET-WO-1300` D10, still carried forward);
> - a crates.io publish.

**Cadence:** build-only. Stop only on §7. One PR, left unmerged.

## 1. Scope

1. **A new workspace crate, `crates/aethelgard-kicad-lib`.**
   - It holds `data/kicad-10.0.7/**`, moved with `git mv` so history follows, byte for byte. That includes `LICENSE.md` and `README.md`.
   - Its `src/lib.rs` exposes one item:
     ```rust
     pub const FILES: [(&str, &[u8]); 17]
     ```
     Each entry is a path under `data/kicad-10.0.7/` and its `include_bytes!`. There are no hashes, no parsing and no other code. It is `#![no_std]` and `#![forbid(unsafe_code)]`.
   - Its `Cargo.toml` has `license = "CC-BY-SA-4.0"`, no dependencies, and the workspace's version, edition and repository.
   - It has a README that says three things:
     - what the crate is;
     - that its licence is CC-BY-SA 4.0 with KiCad's design exception;
     - that designs emitted from it carry no obligation (`RUL-11`).
   - It is added to `members` and to `[workspace.dependencies]`.
2. **`aethelgard-eda` keeps its pins and loses the bytes.**
   - `library.rs` keeps its `(path, sha256)` table, in the same order.
   - It reads the bytes from `aethelgard_kicad_lib::FILES` by path.
   - Three cases are refused, each with its own `LibraryError` variant:
     - a path that is pinned but absent from the data crate;
     - a file present in the data crate but not pinned;
     - a hash that does not match.

     Two of those variants are new, and their names are the builder's choice, in the existing style.
   - `Cargo.toml`: `license = "MIT OR Apache-2.0"`. The comment about the vendored files is removed. It gains the `aethelgard-kicad-lib` dependency.
   - Its README's licence line and `library` row now point at the new crate.
3. **The tests.**
   - `crates/aethelgard-cli/tests/workspace_shape.rs` gets two changes:
     - the `eda` file list loses `data/`;
     - a new test pins `aethelgard-kicad-lib`'s file list and its manifest. It allows no dependencies, `license = "CC-BY-SA-4.0"`, and no `.rs` file but `src/lib.rs`.
   - One new test asserts that no `.kicad_mod` or `.kicad_sym` file exists anywhere under `crates/` outside `crates/aethelgard-kicad-lib/data/`.
   - `library.rs` gains three tests, one for each refusal in §1.2. They drive the check through an injected table, as the existing hash-mismatch test does.
   - **Emitted output is unchanged.** Every golden digest and committed emitted file in the repository stays byte-identical. If one moves, that is a §7 stop.

## 2. Invariants and fences

- `INV-01`, `INV-06`, `INV-07`, `RUL-03`, `RUL-11`. No `unsafe`. No `HashMap`.
- **Fence:**
  - `crates/aethelgard-kicad-lib/**` (new);
  - `crates/aethelgard-eda/{Cargo.toml,README.md,src/library.rs}`;
  - the moved `data/` tree;
  - the root `Cargo.toml` and `Cargo.lock`;
  - `crates/aethelgard-cli/tests/workspace_shape.rs`;
  - this work order's FEEDBACK.
- Every other file in `aethelgard-eda`, and `rules.rs`, `board.rs`, `parity.rs` and `pcb.rs` in particular, diffs to empty. Protected surfaces diff to empty, as in every work order.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance

`emit` on `examples/sentinel-01-minimal.json` produces byte-identical files before and after. FEEDBACK records the sha256 of each. `cargo metadata` shows:
- `aethelgard-eda`'s licence as `MIT OR Apache-2.0`;
- `aethelgard-kicad-lib`'s as `CC-BY-SA-4.0`.

§1.4 passes in `gates` and `tools`. CI is green on the first push.

## 5. Adversarial pass — targeted

1. Remove one file from the data crate, then add one unpinned file. Both must be refused before any symbol or footprint is served.
2. Change one byte in a data-crate file. It must be refused by `aethelgard-eda`'s pin, not by anything in the data crate.
3. Find any path by which `emit` could read a KiCad file that is not the pinned bytes, for example a filesystem read or an environment variable. There must be none.

## 5a. Reduction pass — diff-sized

One line in FEEDBACK's gates.

## 7. Flag-and-stop conditions

- A change outside the fence.
- Any emitted byte changes.
- A test that can only go green by being weakened.
