# AET-WO-1606 — measured, against `PREDICTIONS.md` (committed `a266ef1d` before any run)

All runs are in the tools image `container/IMAGE`, under `~/bin/capped-run` (8 GiB, swap 0, read back), `nice -n 10`, one container at a time. The scripts are in `spike/1606/sh/`, and their outputs are in `spike/1606/out/`.

- **Baseline:** `main` `122378c7`, exported with `git archive` and built in the image.
- **Branch:** this branch's tree, built the same way.
- **Shared inputs:** both trees share scenarios, specifications and fixtures, since this work order changes none of them.

## §7 stop 1: did not fire

The banner is read from the `SendChar` log that `ngSpice_Init` (vendored `sharedspice.h` L371) already fills. The bridge makes no new libngspice call.

## P1 — committed carriers: as predicted

- **P1.1 met.** `git grep 5bdc40e4` finds `crates/aethelgard-cli/src/analog.rs` and a FEEDBACK note (`docs/work-orders/AET-WO-2304b.FEEDBACK.md`, a prefix in prose).
  - `tests/common/mod.rs` recomputes the digest. `doctor_tier1` has no copy of its own; the work order's "doctor_tier1's copy" is `common`'s computed one.
  - No golden, `fab/` package, example or fixture carries it. `fab/*/package/` holds no archive.
- **P1.2 met.** The answer lines in `boot_tier1`, `campaign_tier1`, `workspace_shape`, `container/engine-smoke.sh` and the analog tests are unchanged, and every one still holds.
- **P1.3 met.** `verify`'s `build` gate copies `build`'s `tool.version` alone (`verify.rs:603`), and `tool.version` stays cargo's line.
  - The package gate (below) shows no `fab/` byte moves.

## P2 — the bridge, by mode: as predicted

From `out/bridge-main/` and `out/bridge-br/`; `<D>` stands for each tree's digest.

| mode | main | branch |
|---|---|---|
| no request | identity, usage, exit 2 | identity, usage (now naming `--version`), exit 2 |
| `--version` | read as a request path: `request_refused`, exit 2 | `{"bridge":…,"ngspice":"44.2","ngspice_banner":"** ngspice-44.2 shared library","source_sha256":<D>,"version":"0.1.0"}`, alone, exit 0 |
| request refused | identity, `request_refused`, exit 2 | the same bytes but the digest |
| `--renode /bin/true` | identity, exit 0 | the same bytes but the digest; no ngspice initialised |

- **P2.2 met.** The pinned library prints exactly one banner line, `** ngspice-44.2 shared library`.
- **P2.3 met.** See P3.

## P3 — the differential: met, with derived hashes named

`out/compare.txt` covers `examples/sentinel-01-minimal.json` (four scenarios: solve, `--rail`, `--renode`) and `tests/fixtures/sentinel-03.json` (droop, and `--chain`'s battery sag). Every campaign exits 0 on both trees.

- **36 files are byte-identical:**
  - every generated crate file;
  - both ELFs (`fw.sha256`: `74a731a8…` and `e4906886…` on both);
  - `generate.json`;
  - `campaign.err`.
- **6 files** (every `trace.cbor`) **are equal once the bridge digest is substituted.** The digest appears once, in the header's `tools`, as hex.
- **8 files differ only in values derived from a trace:**
  - each `manifest.cbor`, which holds `sha256(trace.cbor)` and the replayer's state hash (`aethelgard-chaos` `event.rs:441`);
  - each `campaign.jsonl`, whose `state_sequence_hash` is that state hash.
  - The script checks each identity and substitutes each side's own value. A prediction that said "only the digest" missed these. They move because the trace's digest bytes moved, and by nothing else.
- **2 files** (`build.json`) **are equal once the run path is substituted and the new `tool.rustc` is removed.**

So every archived analog byte equals the baseline, except the bridge digest and the hashes over it.

## P4 — the rustc witness: (b), cargo's `.rustc_info.json`

From `out/p4-main.txt` (with `main`'s `build`) and `out/p4-br.txt` (with the branch's).

| case | `cargo --version` | (a) `rustc -vV`, same env and cwd | (b) `.rustc_info.json` | ELF |
|---|---|---|---|---|
| default (`/opt/rustup`) | 1.99.1 | 1.99.1 `58cf8f9c` | 1.99.1 `58cf8f9c` | `74a731a8` |
| `RUSTUP_TOOLCHAIN=nightly-2026-08-21` | 1.100.0-nightly | nightly `8925ea35` | nightly `8925ea35` | none (exit 101: no thumbv7em target) |
| private `RUSTUP_HOME`, `RUSTUP_TOOLCHAIN=alt` | **1.99.1** | 1.99.1-alt `a1a1…` | **1.99.1-alt `a1a1…`** | **`74a731a8`** |
| private `RUSTUP_HOME`, `build.rustc` in an ancestor `.cargo/config.toml` | 1.99.1 | **1.99.1 `58cf8f9c`** | **1.99.1-alt `a1a1…`** | `74a731a8` |

- **P4.1 met.**
  - A fresh target has no record. `cargo --version` alone writes none, and creates no target directory (`build` asks it first, with the same `CARGO_TARGET_DIR`).
  - After the build, each record holds exactly one `-vV` entry: 3 entries on 1.99.1, 4 on nightly.
- **P4.2 met.** Nightly fails to build and is still named.
- **P4.3 met.**
  - `cargo --version` and the ELF are identical, so `tool.version` cannot tell `alt` from 1.99.1, which is `3301` L1.
  - (b), and so the branch's `tool.rustc`, names `alt`: `{"release":"1.99.1-alt","commit_hash":"a1a1…"}` (`out/build-alt-br.json`). `main`'s verdict says only `cargo 1.99.1` (`out/build-alt-main.json`).
- **P4.4 met for plain cargo, and closed in `build`:**
  - With plain cargo, cargo ran the ancestor's compiler: (b) names it and (a) does not. So (a) is not a witness.
  - `build` already refuses that configuration before cargo runs (`CargoConfigRefused`, `AET-WO-3301` F13), so inside `build` no case was found where (a) and (b) differ.
  - (b) is chosen anyway, because it is the record of the run, not a second question.
- **P4.5 met, with a caveat:**
  - The stripped ELF keeps `.comment`: `rustc version 1.99.1 (58cf8f9c3 2026-10-13)`.
  - Under `alt`, whose `rustc` is 1.99.1's binary behind a wrapper that lies only to `-vV`, `.comment` still says 1.99.1 while (b) says alt.
  - So (b) records what the compiler cargo invoked says of itself, and `.comment` what the code generator says. Both are self-reports. `.comment` is also read from the artifact under judgment. Not used; FEEDBACK names this residual.

## P5 — `doctor`: met

Both runs are in the pinned image, with `PATH` holding `/opt/cargo/bin` (`out/doctor-*.json`).

- Both exit 0 with nothing missing.
- The branch's bridge line is `{"tool":"aethelgard-bridge","version":"0.1.0","ngspice_loaded":true,"ngspice":"44.2"}`. Every other tool line is identical.

## P6 — tests

Recorded in FEEDBACK, with the mutants.
