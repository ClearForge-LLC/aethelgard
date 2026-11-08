# AET-WO-1800 — FEEDBACK

**Branch:** `wo/aet-1800-publish-ready` · **Base:** `ecdb86c` · **Builder:** Claude Code (Sonnet 5.5) as `forge`.

**§7, held:** no `.rs` under `src/` changed; no crate near 10 MiB; no test weakened (two existing tests extended, see F1 and F2); no token, publish or `cargo login`.

## Acceptance

Packages, from the gates step run locally and in CI (`.crate` size, file count). Each but kicad-lib holds `LICENSE-MIT`, `LICENSE-APACHE` and `README.md`; kicad-lib (CC-BY-SA-4.0) holds `README.md` and KiCad's `data/kicad-10.0.7/LICENSE.md` and neither MIT nor Apache file, asserted by the step and by `package_licences.rs`; svd-hal holds ST's `LICENSE.md`, `LICENSE` and both `SOURCE.md`.

| crate | bytes | files |
|---|---|---|
| aethelgard-ir | 38152 | 11 |
| aethelgard-kicad-lib | 19172 | 24 |
| aethelgard-svd-hal | 309806 | 44 |
| aethelgard-eda | 73950 | 19 |
| aethelgard-twin | 15341 | 11 |
| aethelgard-chaos | 35478 | 20 |
| aethelgard-cli | 174045 | 44 |

`cargo metadata`: `aethelgard-evolve` has `publish = []`; the board crates (`boards/sentinel-01`, its `firmware`) carry `publish = false` and are outside the workspace. The CLI built and ran (`validate` on the minimal example) from the unpacked packages with `--offline` and `patch.crates-io` pointing at them.

## Adversarial pass

The packaging step is bash and `jq` only (no Python). Re-run on it:

1. Excluding `data/st-cmsis-device-h7/stm32h753xx.h` from svd-hal: fails, naming the file (`couldn't read src/../data/.../stm32h753xx.h`, from cargo's verification build). Excluding ST's `data/st-open-pin-data/SOURCE.md` fails in the step: `aethelgard-svd-hal: licence or notice file data/st-open-pin-data/SOURCE.md is missing from the package`.
2. Dropping `version` from the `aethelgard-ir` path entry: fails (`failed to verify manifest` of aethelgard-eda).
3. Reads outside a package directory: none at build or run time. `board/firmware.rs:171` and `board.rs:644` mention `../../../target` and `boards/` only inside emitted text. svd-hal's tests reach `boards/` through a path dev-dependency, stripped when packaged; packaging never runs tests.
4. Licence mutants: `LICENSE-MIT` removed from twin fails (`aethelgard-twin: licence or notice file LICENSE-MIT is missing from the package`); an MIT copy added to kicad-lib fails (`aethelgard-kicad-lib: LICENSE-MIT must not be in the package`); one appended byte in `twin/LICENSE-MIT` fails `package_licences.rs`.

The first adversarial run after the rewrite showed the offline `cargo install` quoting was wrong in the committed step (an uncommitted fix had been lost to a `git reset`); fixed and the full step re-run green, ending in `aethelgard validate` on the packaged binary.

## Findings

Entry schema: `finding · where · severity · type · recommendation · decision-needed`.

- **F1** `workspace_shape.rs` pinned each crate's exact file set and svd-hal's dev-dependencies to `workspace = true` · `crates/aethelgard-cli/tests/workspace_shape.rs` · low · test extended, not weakened · it now allows `README.md` and the two licence files, and exempts one dev-dependency (F2) while asserting it is path-only · no.
- **F2** svd-hal's dev-dependency on `aethelgard-twin` must be path-only (`../aethelgard-twin`), not `workspace = true`. A versioned entry survives packaging, and twin depends on svd-hal, so the first publish would be a cycle · `crates/aethelgard-svd-hal/Cargo.toml` · medium · design · none; the same applies to any future dev-dependency cycle · no.
- **F3** Two READMEs are new (`aethelgard-twin`, `aethelgard-evolve`), outside the fence's letter, because the WO asks every member for `readme` and neither had one · low · scope · none · no.
- **F4** `cargo package --workspace` packages `publish = false` members too in 1.99.1 (it packaged evolve), so the step names the publishable set with `-p` and a metadata assertion fails if that list drifts from the workspace · low · note.
- **F5** `aethelgard-kicad-lib` is CC-BY-SA-4.0 and carries KiCad's `LICENSE.md` only, no MIT or Apache file (architect ruling on PR #65) · note.
- **F6** The step reaches the crates.io index to resolve third-party crates; packaging itself is token-free. The CLI install is `--offline`.

## Reduction pass (gates)

One line: the step is inline in `ci.yml` with no helper script; nothing added that a cargo flag already does.
