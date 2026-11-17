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

## 2. Measured (§1.1), and against the predictions

Furnace, 2026-11-17. Cargo 1.99.1. Evidence in `~/.cache/aet-1801/`. Every cargo run was under a capped user scope.

**Every member can publish [measured].**
- All ten have `publish` unset (allowed) and a description, licence, repository, readme, keywords and categories (`cargo metadata --no-deps`).
- The three never-published crates already carry complete metadata, so §1.2 needed no edit:
  - `aethelgard-evolve`: keywords attribution, traces, hardware, embedded;
  - `aethelgard-pincheck`: stm32, gpio, elf, pinout, embedded;
  - `aethelgard-renode-models`: renode, models, sensors, data.
- crates.io answered 404 for all three and 200 for `aethelgard-ir` (2026-11-17).

**Packaged sizes** (`cargo package --locked --no-verify`, as CI runs it; `.crate` bytes and files). All are far under crates.io's 10 MB limit.

| Crate | `.crate` bytes | Files |
|---|---|---|
| `aethelgard-ir` | 325 955 | 43 |
| `aethelgard-kicad-lib` | 33 735 | 48 |
| `aethelgard-renode-models` | 16 045 | 10 |
| `aethelgard-svd-hal` | 437 698 | 104 |
| `aethelgard-eda` | 205 419 | 28 |
| `aethelgard-twin` | 103 566 | 25 |
| `aethelgard-chaos` | 142 176 | 27 |
| `aethelgard-evolve` | 72 223 | 14 |
| `aethelgard-pincheck` | 32 205 | 20 |
| `aethelgard-cli` | 680 452 | 180 |

**`aethelgard-renode-models`' files and their licences [read].**
- `BMP581.cs` and `ICM42688P.cs`, whose headers read "AET-WO-4220. MIT OR Apache-2.0. Written for Aethelgard; no Antmicro source copied".
- `src/lib.rs`, which `include_str!`s the two.
- `README.md`, `LICENSE-MIT`, `LICENSE-APACHE`, and the generated manifest files.
- No third-party file.

**Third-party files elsewhere, checked for the adversarial pass [read]:**
- `aethelgard-kicad-lib`: KiCad's libraries under CC-BY-SA-4.0, with KiCad's `LICENSE.md`. CI requires it.
- `aethelgard-svd-hal`: ST's data, with its `LICENSE` and `SOURCE.md` files. CI requires them.
- `aethelgard-twin`: three Renode 1.17.0 `.repl` files, copied unchanged and MIT, with Renode's `LICENSE` and a `NOTICE` naming tag `v1.17.0` at `ab721d88`.
- Everything else is the project's own.

**No stop.** No crate fails to publish, and none is over 10 MB.

**The bump, tried on a local branch that is never pushed** (`local/1801-bump-trial`, one commit on top of this branch's first commit):

| Prediction | Result |
|---|---|
| Ten manifest lines and ten `Cargo.lock` entries move | **Held [measured].** `[workspace.package] version` and the nine path entries in `[workspace.dependencies]`; `cargo update --workspace --offline` moves exactly the ten workspace entries in `Cargo.lock`. |
| Four test sites fail until their literal moves | **Wrong: one does [measured].** Tier 1 at 0.2.0 fails only `aethelgard-cli/tests/validate_cli.rs:222`, `version_prints_name_and_version` (`b"aethelgard 0.1.0\n"`). |
| | **The other three are test data, not the engine's version [read]:** `aethelgard-eda/src/fab.rs`'s three fixtures and `aethelgard-cli/src/fab.rs:994` pass a version in as data. `with_tools.rs:1515` is a hand-written input board. |
| | **Tier 2** compares the emitted `generator_version` and `engine_version` with `env!("CARGO_PKG_VERSION")` at run time. No committed golden stores them. CI on the bump commit is its proof. |
| The notes fixtures, the chaos traces and the bridge's version do not move | **Held [measured]:** no failure. |
| `cargo package --locked` with verification passes at 0.2.0 | **Held [measured]:** exit 0, ten `Verifying` lines. |
| CI's packaging step fails as written at 0.2.0 | **[read]:** it names `$c-0.1.0.crate` and `$c-0.1.0/` at lines 75, 99, 112, 113 and 115 of `ci.yml`. Not run. |

**Golden bytes the bump moves, the complete list:**
- `Cargo.toml`: 10 lines.
- `Cargo.lock`: 10 entries.
- `crates/aethelgard-cli/tests/validate_cli.rs:222`: `0.1.0` → `0.2.0`.

Nothing else changes; no byte other than the version moves (§7 does not stop).

**The acceptance clauses that can run before the window [measured, on the trial bump]:**
- **`cargo publish --workspace --dry-run --locked`: exit 0.** It packaged, verified and stopped before uploading all ten, at 0.2.0.
- **`cargo publish -p <crate> --dry-run --locked`: exit 0** for `aethelgard-ir`, `aethelgard-kicad-lib` and `aethelgard-renode-models`, the three with no workspace dependency. The other seven cannot be dry-run one at a time until their dependencies are live: `aethelgard-evolve` alone fails with "failed to select a version for the requirement `aethelgard-chaos = "^0.2.0"`", before any upload. `RELEASE-0.2.0.md` names them.
- **A clean `cargo install --path crates/aethelgard-cli --locked`:** `aethelgard --version` prints `aethelgard 0.2.0`, and `validate < examples/sentinel-01-minimal.json` exits 0 with `{"ok":true,"project_name":"sentinel-01",…`.

**The two findings the bump was to close:**
- **`3200` F18 (`--no-verify`): the bump retires it for this release, not for good.**
  - At 0.2.0 no copy is on crates.io, so verification resolves siblings to the local packages and passes (above).
  - Once 0.2.0 is published, any later change still says 0.2.0 until the next bump, so the collision returns.
  - CI should therefore keep `--no-verify` and its offline build from the unpacked packages; `docs/releasing.md` now says so.
  - The release-time check is `cargo publish --workspace --dry-run`, which verifies.
- **`2205` F4 (the engine's version is the package version, not a commit): the bump does not close it.** After the bump, two builds of different commits both say 0.2.0, as before. Closing it needs the commit in the version string (a build script, or `git describe` in CI), and that is a behaviour change §1 excludes. **Recommendation:** keep the backlog row and give it to the next work order that owns the archive header.

## 3. Adversarial pass (targeted, on the packaging)

All [measured].
- **A crate that builds in the workspace but not from its package.**
  - A mutant made `aethelgard-renode-models` `include_str!("../../../README.md")`. The workspace built; `cargo package -p aethelgard-renode-models --locked` refused: "couldn't read `src/../../../README.md`", "failed to verify package tarball". The mutant was reverted.
  - The release-time dry run verifies, and CI's include check would also refuse it.
  - The verified packaging of all ten at 0.2.0 shows that no such crate exists today.
- **A dependency published after its dependant.**
  - Against `cargo metadata`'s normal and build dependencies, `docs/releasing.md`'s order has no crate before one of its dependencies.
  - `aethelgard-svd-hal`'s dev-dependencies on `aethelgard-cli` and `aethelgard-twin` are path-only and absent from its packaged manifest, so they bind nothing.
  - A dependant published first is refused before upload (`aethelgard-evolve`, above).
- **A crate that packages a file it may not license.** Every non-code file in the ten packages is listed in §2. The third-party files carry their licences and notices; the rest are the project's own.

## 4. What was built (non-bump commits; the bump is prepared, not pushed)

- **`docs/releasing.md`,** for every release:
  - the token's scopes, with `publish-new` only for new names (0.2.0 has three) and a crate scope;
  - the workspace dry run before the first publish, and a live-dependency check before each crate (crates.io's API, 200 or 404, no login);
  - the order annotated with each crate's dependencies;
  - index lag, and the new-name rate limit that 0.1.0 met once;
  - why CI keeps `--no-verify`.
- **`RELEASE-0.2.0.md`** (repository root): Scotty's checklist, with the exact commands in order, what each should print, the dry runs done and the ones not possible, the verification by `cargo install aethelgard-cli --version 0.2.0` and `validate`, and the release notes.
- **The release notes** are 21 capabilities, each cited to its work order. A subagent drafted them from `docs/roadmap.md`, `git log` and the READMEs; I checked its anchors:
  - specification 12 at `aethelgard-ir/src/lib.rs:66`;
  - 1800 merged as `e9b5b34` on 2026-11-08, with the publish on 2026-11-09 (`3a2a4b5`);
  - `aethelgard-evolve` was `publish = false` at 1800;
  - every cited work order is on `main`.
- **The bump is ready on `local/1801-bump-trial`** and is the three files listed in §2. It is pushed only when the architect opens the window, after `2302` merges.

## 5. Questions for the architect

- **Q1, `ci.yml` at the bump: required.** CI's packaging step names `0.1.0` at five places (lines 75, 99, 112, 113, 115), so the bump commit fails CI unless they change, and `ci.yml` is outside this fence.
  - **Proposal:** read the version once in the step (`v=$(cargo metadata --no-deps --format-version 1 --locked | jq -r '.packages[] | select(.name=="aethelgard-cli") | .version')`) and use `$c-$v`. That makes the step version-agnostic, so later bumps need no `ci.yml` edit.
  - **Your call:** whether the bump commit may carry it, or you land it first.
- **Q2, `crates/aethelgard-cli/README.md`.** Its example outputs show `"aethelgard":"0.1.0"` and `"engine_version":"0.1.0"` (lines 462, 656, 720). They are examples, not goldens, and outside the fence. Update them at the bump, or leave them?
- **Q3, the bridge.** `bridge/Cargo.toml` stays 0.1.0: it is never published, has its own version, and its version appears in the bridge's own output (`analog.rs`, tests). Confirm it does not follow the workspace.

- **Q4, a peer note (4220b, 03:30 UTC).** 4220b will change `aethelgard-kicad-lib`'s `license` to `CC-BY-SA-4.0 AND (MIT OR Apache-2.0)`, because the crate gains the project's own symbols and footprints.
  - CI's packaging step currently requires KiCad's `LICENSE.md` in that crate and forbids `LICENSE-MIT` and `LICENSE-APACHE`; the crate would then need both licences' texts.
  - If 4220b merges before the bump, that step and the packaged file list change with it. I measured the crate as it is on `36e428b`.
  - Nothing in this PR depends on it, but the release should go out with whichever licence is on `main`.

## 6. Gates

- **Local [measured]**, on this branch (no Rust changed), Furnace, capped scope:
  - `cargo fmt --all -- --check` clean;
  - `cargo clippy --workspace --all-targets -- -D warnings` clean;
  - `cargo test --workspace` exit 0, no failure.
- **On the local trial bump:** tier 1 passes once `validate_cli.rs:222` moves.
- **TIER2_MIN** is untouched.
- **Reduction pass (diff): nothing to remove.** The diff is two documents and this FEEDBACK. `docs/releasing.md` gained sections and lost none, and `RELEASE-0.2.0.md` states the order once, as a table that cites `docs/releasing.md` rather than repeating its reasons.
