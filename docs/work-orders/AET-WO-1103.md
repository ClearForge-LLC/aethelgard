# AET-WO-1103 — No board's name in engine code, held by a test that can fail

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1103-no-board-names`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P1 (one source). Written 2026-11-20. **Cadence:** build.

**Grounds** (`4600` D5 and F13, ruled on #245):
- **The rule:** roadmap §6 says capability enters through the specification. Engine code must not know a board by name. A Sentinel-shaped special case is exactly what P4's second-board clause is there to catch.
- **The gap:** nothing holds that rule.
  - `4600`'s board-name check found the obvious pathspec vacuous: `crates/*/src` matches no file.
  - With `/**`, the check finds 62 hits. One is a string outside tests: `crates/aethelgard-cli/src/ledger.rs:388`.
  - The other hits are comments citing where a value was measured, plus two manifests (the workspace member and a trybuild dev-dependency). Both kinds are kept, by ruling.
- **Roadmap §6's measured sentence** may rest on the vacuous pathspec. The architect re-measures it from this order's test and rewords it in a docs PR.

> **What this is:**
> - a tier-1 test that walks `crates/*/src/**`, `bridge/src/**` and `models/**`, skips each file's `#[cfg(test)]` regions, and fails on a board name in non-test code;
> - its file selection pinned by a planted hit, so the test cannot go vacuous;
> - `ledger.rs:388`'s board path moved into a doc comment or a test, with its evidence kept.
>
> It is NOT:
> - a rename, or a cleanup of comments that cite measurements;
> - any change to `boards/`, `examples/` or the specification.

## 1. Build

1. **Define "a board name"** from the repository, not a hand list.
   - The names are the directory names under `boards/` and the file stems under `examples/` that name a board (`sentinel-01`, `sentinel-02`, …).
   - Say exactly how the list is derived, and which stems are excluded and why (a fixture, a variant).
   - Match case-insensitively and across `-` and `_`.
2. **The test**, beside the existing workspace-shape test (find it; `4600` names `workspace_shape.rs`):
   - **Selection:** every `.rs` file under `crates/*/src/**`, `bridge/src/**` and `models/**`.
   - **Skipped regions:** `#[cfg(test)]` modules and items, parsed well enough to skip a whole module. Say how, and what the parser does not handle.
   - **Comments:**
     - Line and block comments are allowed, because they carry citations.
     - Doc comments (`///`, `//!`) are allowed too, and FEEDBACK says why.
     - A string literal, an identifier or an attribute value is a hit.
   - **The failure** names the file, the line and the matched name.
3. **Pin the selection.**
   - A test plants a hit in a temporary copy of one selected file's layout (or a fixture tree laid out the same way) and expects a failure.
   - A second test asserts that the selection count is above a floor measured from `main` and stated.
   - The aim: a pathspec that stops matching fails loudly.
4. **Fix the one real hit,** `cli/src/ledger.rs:388`. Move the board path into a doc comment, or into the test that needs it.
   - The evidence the line carries (what the platform emitter was measured on) is kept and still cited.
   - Show that `ledger`'s behaviour is unchanged: its tests pass and nothing else moves.
5. **Report the count** in FEEDBACK, so the architect can re-measure roadmap §6:
   - files selected, lines scanned, and hits before and after the fix;
   - each allowed comment hit grouped by crate.

## 2. Acceptance

- The test passes on the branch and fails on `main`, on `ledger.rs:388`. Show both runs.
- **Mutants:**
  - break the selection (for example, back to `crates/*/src`), and the floor test fails;
  - remove the skip of `#[cfg(test)]`, and the test fails on a known test-only hit;
  - plant a string literal hit, and the test fails.
- The CLAUDE.md gates pass, and CI is green. If you add a tier-1 test only, `TIER2_MIN` does not move.

## 3. Invariants and fences

- **Invariants:** `INV-01`, `INV-06`.
- **Fence:**
  - the new test file and its fixture tree;
  - `crates/aethelgard-cli/src/ledger.rs`, the one line and its doc comment;
  - FEEDBACK.
  - **Not:** any other source file, `boards/`, `examples/`, workflows, `ci/` or steering documents. A weight row in `ci/test-weights.tsv` for your test, only if CI's P2 guard asks for one, is in the fence.
- **Running beside you:** `1911` (CI), `4240v`, `2304b`, `4202`, `1209`, `3305`. You share no file with them, except possibly `ledger.rs` with 2304b: merge `main` before READY.
- **One container of yours at a time,** under `capped-run`, at `nice -n 10`.
- One PR, left unmerged. Write `~/.cache/aet-1103/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A board name that reaches engine code by another spelling:**
  - a constant built by concatenation;
  - an `include_str!` of a board path;
  - a name in a `build.rs`.

  Say which the test catches and which it does not.
- **A `#[cfg(test)]` skip that swallows non-test code,** such as a `cfg(any(test, feature = …))`.

## 7. Flag-and-stop conditions

- **More than the one known string hit in non-test code.** Stop with the list. The architect rules which are real.
- **The `#[cfg(test)]` skip cannot be made reliable without a full Rust parser dependency.** Stop and say what a minimal approach misses.
