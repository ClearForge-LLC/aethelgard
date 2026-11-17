# AET-WO-1801 FEEDBACK — The 0.2.0 release

**Builder:** Claude (Opus 5.5), worktree `~/aethelgard-4403`, branch `wo/aet-1801-release-0.2.0` from `main` `36e428b`.
**Cadence:** prepare now; the bump lands in a quiet window the architect opens, after `2302` merges. The work order is read from `origin/claude/argon-repo-docs-synthesis-rjrb0d` (docs PR #187); it is not committed here.

Labels: **[read]** read at source; **[measured]** run on Furnace or GitHub's runner; **[inferred]** neither.

## 1. Process note and predictions

The §1.1 inventory (metadata, `publish` settings, packaged sizes, the licence of every file in `aethelgard-renode-models`) ran before this section was written. It is reported in §2 as measured, not as predicted. The predictions below cover only the bump, which has not been tried.

All **[inferred]**, from reading the sources at `36e428b`:
- **What the bump touches.** `[workspace.package] version` and the nine path entries in `[workspace.dependencies]` go to 0.2.0, and so does `Cargo.lock`'s ten workspace entries. Nothing else in a manifest moves.
- **What the bump moves at run time.** `env!("CARGO_PKG_VERSION")` reaches output in seven places: the KiCad schematic's and PCB's `generator_version`, the fab notes, `verify`, `extract`, `speeds`, and the MCP server info. No committed golden stores those bytes; tests compare them at run time.
- **Which tests fail at 0.2.0 until their literals move** (they write the engine version as "0.1.0"):
  - `aethelgard-cli/tests/validate_cli.rs:222` (`--version`);
  - `aethelgard-cli/tests/with_tools.rs:1515` (the PCB header, tier 2);
  - `aethelgard-eda/src/fab.rs` (three test fixtures);
  - `aethelgard-cli/src/fab.rs:994`.
- **Which tests do not fail.**
  - The chaos tests' `("aethelgard", "0.1.0")` trace headers are test data, not the engine's version.
  - The `notes-before-4300d2-*.md` fixtures are notes as an older engine wrote them.
  - The bridge's `"version":"0.1.0"` is the bridge's own, a crate outside the workspace that stays 0.1.0.
- **Packaging at 0.2.0.** `cargo package --locked` with verification passes for all ten: no 0.2.0 is on crates.io, so each verify build resolves its siblings to the local packages. That retires 3200 F18's `--no-verify`.
- **CI's packaging step at 0.2.0** fails as written. It names `$c-0.1.0.crate` in three places, and `.github/workflows/ci.yml` is outside this work order's fence.
