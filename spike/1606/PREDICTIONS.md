# AET-WO-1606 — predictions, written before any measured run

Written 2026-11-22 on `wo/aet-1606-tool-identities` from `main` `122378c7`.
Before writing this file I had only read source and run three commands:
- `rustc -vV` on Furnace's host;
- `rustup toolchain list` in the tools image;
- `cargo --version` and `rustc -vV` in the tools image.

Nothing below was measured before it was written.

What those three commands showed:
- The image has two toolchains: `1.99.1`, with `thumbv7em-none-eabihf`, and `nightly-2026-08-21`, which is Kani's and has no thumbv7em target.
- The host has `1.99.1` alone, also without the thumbv7em target. So `build` runs only in the image.

## §7 stop 1 does not fire

The banner already reaches the bridge through `SendChar`, the `printfcn` of `ngSpice_Init`, which is vendored `sharedspice.h` L371. `Ngspice::start` reads it from `host::take_log()` today (`solver.rs:57–66`). Reading it costs no new libngspice call.

## P1 — committed carriers (§1.1, first bullet)

- **P1.1 The bridge digest** (`5bdc40e4…`) is committed in one place: `crates/aethelgard-cli/src/analog.rs` (`BRIDGE_SOURCE_SHA256`).
  - `analog.rs`'s pin test recomputes it from `bridge/` on disk, and so does `tests/common/mod.rs::bridge_identity`.
  - `doctor_tier1` holds no copy of its own; it uses `common`'s.
  - No golden, `fab/` package, `examples/` file or fixture carries it. A campaign archive carries it in its header, but no archive is committed.
- **P1.2 `NGSPICE` / `"ngspice":"44.2"`** is in:
  - `bridge/src/main.rs` (answers, and its own tests) and `solver.rs` (the pin);
  - `analog.rs` (the CLI's pin, two of its own writes at the chain `tools` and the I2C `AnalogRecord`, and its tests);
  - `tests/boot_tier1.rs`, `tests/campaign_tier1.rs` and `tests/workspace_shape.rs`;
  - `container/engine-smoke.sh` (an answer line).
  - Under this build every one of those answer lines keeps its bytes.
- **P1.3 `build`'s version string** (`cargo 1.99.1 (5f94df478 2026-08-27)`) is in:
  - `verify`'s `build` gate entry (`tool_version`, read from `build`'s `tool.version`, `verify.rs:603`). Through that entry it is inside the hash that every `fab/*/package/MANIFEST.json` `verdict_sha256` pins.
  - Committed spike outputs (`spike/3303b`, `spike/4600`), and `doctor_tier1`'s stub `cargo 1.95.0 (stub)`.
  - **So `tool.version` stays cargo's line,** and rustc goes in a new field. Prediction: no `fab/` byte moves.

## P2 — the bridge, by mode (§1.2)

The identity names ngspice only in a mode that loads the library and accepts it. Every exit code and refusal kind stays as on `main`. Today the identity is `{bridge, source_sha256, version}`.

| mode | library initialised | first line | then | exit |
|---|---|---|---|---|
| `<request>`, `--rail`, `--chain`, accepted | yes, after the request parses | identity + `ngspice` (parsed) + `ngspice_banner` (verbatim) | answers, `ngspice` from the banner | as today |
| any of those, request refused | no | identity, as today | `request_refused` | 2 |
| any of those, banner absent, repeated, unparseable or not 44.2 | yes | identity, as today | `ngspice_unusable`, naming what it read | 3 |
| `--version` (new) | yes | identity + `ngspice` + `ngspice_banner` | nothing | 0 |
| `--version`, library unusable | yes | identity, as today | `ngspice_unusable` | 3 |
| no request or bad usage | no | identity, as today | `usage` | 2 |
| `--renode` | no | identity, as today | Renode's output | Renode's |

- **P2.1** `--renode` does not initialise ngspice: in that mode no ngspice runs, so naming one would report a version that did not run (§5). This is a fork, and I record it in FEEDBACK.
- **P2.2** On the pinned library the banner is exactly one line, `** ngspice-44.2 shared library`, and it parses to `44.2`.
- **P2.3** Every answer line's bytes equal `main`'s. Only the identity line changes, and the CLI does not archive that line, only its digest.

## P3 — baseline and differential (§1.1, second bullet; §2, first bullet)

- **Runs:** in the image, under `capped-run`, with `main`'s CLI and bridge, then again with this branch's.
  - `examples/sentinel-01-minimal.json` with `scenarios/sentinel-01/{pullup-2k2,pullup-47k,droop-tolerated,droop-bor3}` (solve, `--rail`, `--renode`).
  - `tests/fixtures/sentinel-03.json` with `scenarios/sentinel-03/{droop-tolerated,battery-sag-tolerated}` (`--chain`).
- **P3.1** The campaign stdout, every `simulate` verdict and every archive file are byte-identical between the two, after the bridge digest is replaced by a placeholder. The digest occurs only in the archive header's `tools`.
- **P3.2** The ELF that `build` writes is byte-identical between `main` and the branch, for each board.

## P4 — the rustc witness (§1.1, third bullet)

Two candidates:
- **(a)** `rustc -vV`, run by the CLI under `build`'s variables and grants;
- **(b)** `$CARGO_TARGET_DIR/.rustc_info.json`, which cargo writes when it asks its own compiler `-vV`.

Predictions:
- **P4.1** A fresh target directory has no `.rustc_info.json` before cargo runs. After `cargo build` there is one, and exactly one of its `outputs` entries has a stdout starting with `rustc ` and a `release:` line. That entry equals the compiler's `-vV`.
- **P4.2** Under `RUSTUP_TOOLCHAIN=nightly-2026-08-21`, the image's second toolchain:
  - (b) names nightly's release and commit;
  - the build fails, because nightly has no thumbv7em target;
  - `cargo --version` also differs.
  - So this pair does not show L1 by itself.
- **P4.3** A private `RUSTUP_HOME`, under `/cache`, holding a copy of 1.99.1 and a second toolchain `alt`:
  - `alt` is the same files, except that `bin/rustc` is a wrapper that reports another release and commit for `-vV` and runs the real rustc for everything else.
  - It gives one `cargo --version`, an identical ELF and two different (b).
  - So `tool.version` alone cannot tell the two compilers apart, and (b) can.
- **P4.4** A compiler chosen outside the cleared environment: a `.cargo/config.toml` with `build.rustc`, in an ancestor of the scratch (`TMPDIR`'s parent).
  - Cargo uses that compiler, and (b) names it.
  - (a), run separately, names the toolchain's rustc instead.
  - So (a) is not a witness. **(b) is the witness.**
- **P4.5** The stripped ELF keeps a `.comment` section naming `rustc version 1.99.1 (58cf8f9c3 2026-10-13)`. This is a cross-check only: `strip = "symbols"` keeps `.comment`.

## P5 — `doctor` (§1.3)

- **P5.1** In the pinned image, `doctor`'s bridge line gains `"ngspice":"44.2"` beside `"ngspice_loaded":true`, from `--version`.
- **P5.2** Every other byte of `doctor`'s report is unchanged, except paths and sandbox fields that already vary between runs.

## P6 — tests

- **P6.1** `TIER2_MIN` rises only by the tier-2 tests this work order adds: one, `--version` against the real library. I will state the number.
- **P6.2** A mutant that writes the constant into an answer's `ngspice` is killed by a unit test that feeds a non-44.2 banner through the parse and record path.
- **P6.3** A mutant that drops the pin check is killed by a test that asserts a `45.0` banner is refused.
