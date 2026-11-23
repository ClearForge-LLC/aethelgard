# AET-WO-1607 — The CLI's small backlog: `check support` pinned, test scratch off `$HOME`, `verify` on a board that draws none, the lifecycle banner on every verdict

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1607-cli-backlog`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4, backlog wave 1. Written 2026-11-23. **Cadence:** a small build, measured against unchanged bytes.

**Grounds** (roadmap §7.1):
1. **`1311` F4.** `check support` runs `kicad-cli` unpinned. Use `kicad::pinned`, as the other KiCad paths do.
2. **`1603` F2.** Tier-1 tests that take `build`/`prove` past the pre-flight fail on a machine with `~/.cargo/config.toml`, because the default scratch sits under `$HOME`. Give them a scratch directory outside the home (`tests/common/mod.rs`).
3. **`1211e` F11.** `fab_tier1::a_rail_or_bus_no_fault_kind_reaches_is_named` fails with `BaseOutsideRoot` when `CARGO_TARGET_DIR` is outside the repository. Put the test's scratch under the repository.
4. **The Grok review of 2026-11-16, the `readme_status.rs` half.** The test checks only phases marked complete. Make it check the status line the README states for each phase. The README is protected: change the test, not the README, and say in FEEDBACK whether the README's current line passes.
5. **`1210` F2.** Plain `verify` on a specification that declares it draws no board is refused up front with `DrawsNoBoard`. It should instead run, and report the board gates as not applicable.
6. **`4211` F2, adversarial A-B1, the verdict half.** The lifecycle banner is missing from the `emit`, `solve`, `check drc --spec`, `check support`, `prove` and `check pinout` verdicts, for `eol` as for `nrnd`. The KiCad-file half moves emitted bytes and is a later work order.

> **What this is:** (1)–(6), each with a test that fails without it.
>
> It is NOT:
> - a banner inside any emitted KiCad file;
> - a new `validate` refusal or specification key;
> - any change to `renode.rs`, `campaign.rs`, the IR, or `tests/fixtures/*.json`.

## 1. Build

1. **Measure first,** with predictions in `spike/1607/`:
   - the emitted bytes, verdict lines and exit codes of every committed specification;
   - which verdict lines (6) changes.
2. **(1)–(6),** each with a test and a mutant it kills.
3. **The null sweep:** only the verdict lines (6) names move.

## 2. Acceptance

- **(1)–(6) closed.** The tests from (2) and (3) pass with `CARGO_TARGET_DIR` outside the repository and with a `~/.cargo/config.toml` present.
- **No emitted file and no `fab/` byte moves.**
- **The CLAUDE.md gates pass, and CI is green.**

## 3. Fences

- **Fence:**
  - `crates/aethelgard-cli`: `src/main.rs`, `kicad.rs`, `verify.rs` (the `DrawsNoBoard` path), `pinout.rs` (the banner line only);
  - `tests/common/mod.rs`, `fab_tier1.rs`, `readme_status.rs`, and new tests;
  - `spike/1607/` and FEEDBACK.
- **Not:** `renode.rs`, `campaign.rs`, `tests/fixtures/*.json`, the IR, eda, the generators; workflows, `ci/` (except a new tier-2 test's row in `ci/test-weights.tsv`).
- **Running beside you:**
  - `4300i` (IR; it re-versions the CLI's fixtures);
  - `4407` (images);
  - `3307` (evolve);
  - `2309` (chaos).

  Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-1607/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A verdict path that still omits the banner** for an `eol` part.
- **A `verify` of a no-board specification** that reports a board gate as passed, not as not applicable.
- **A test scratch that still resolves under `$HOME`** through an environment variable.

## 7. Flag-and-stop conditions

- **A `fab/` byte or an emitted file moves.** Stop.
