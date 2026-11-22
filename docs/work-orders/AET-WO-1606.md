# AET-WO-1606 — Tool identities from what ran: the bridge reports the ngspice banner it read, `doctor` reports ngspice's version, and `build` names the rustc cargo ran

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1606-tool-identities`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4, tooling backlog. Written 2026-11-22. **Cadence:** a measurement, then a small build.

**Grounds** (roadmap §7.1, three rows):
- **`2205` F1, D3:** the archive's `ngspice` is the bridge's constant `NGSPICE` (`bridge/src/solver.rs:42`). The bridge writes it after refusing any library whose banner is not `** ngspice-44.2 shared library` (`solver.rs:57–66`). The value is the library's, but the text archived is the bridge's. Ruled: the bridge's identity line carries the banner it read, in the next work order that touches the bridge.
- **`1604` Q1 and D4:** `doctor` reports `ngspice_loaded` and no version, because the bridge has no version mode. Ruled as backlog: a bridge `--version` that asks libngspice itself.
- **`3301` L1:** `build`'s verdict names `cargo --version` (`crates/aethelgard-cli/src/firmware.rs`, `build`). The compiler is rustc, so a caller's `RUSTUP_TOOLCHAIN` could give two ELFs under one version string. Recommendation: name `rustc -vV`'s release.
- Read those three FEEDBACK sections first.

> **What this is:**
> - **The bridge:**
>   - its identity line carries the banner the library printed, verbatim;
>   - every answer's `ngspice` is read from that banner, not from the constant. The pin check stays: the bridge still refuses any banner but 44.2's, so on the pinned library the archived bytes stay identical;
>   - a `--version` mode loads libngspice, reads its banner and prints one JSON line with the identity and the version. It runs no circuit.
> - **`doctor`** reports ngspice's version from that mode, beside `ngspice_loaded`.
> - **`build`** names the rustc that compiled the ELF, release and commit hash, as well as cargo's version.
>   - **Measure first which witness is the compiler cargo ran.** One candidate is `rustc -vV` under the same environment and grants; another is cargo's own record of it in the target directory (`.rustc_info.json`). A version string that is not shown to be the one that ran is not a witness (`INV-07`).
> - **`BRIDGE_SOURCE_SHA256` moves,** with its pin test, and `doctor_tier1`'s copy.
>
> It is NOT:
> - the archive header's engine version (`2205` F4). It moves every trace header while `1211c` holds the trace version: a later row.
> - the `verify` campaign gate's tool version (`3301` L3). `verify` is `1211c`'s file now.
> - a new ngspice, or any change to what the bridge computes.

## 1. Build

1. **Measure first,** with predictions committed before any run, in `spike/1606/`:
   - every committed file that carries the bridge's digest, `NGSPICE`, or `build`'s version string: goldens, `fab/` packages, `examples/`, test fixtures;
   - one analog measurement's archived bytes and one `build` verdict on `main`, as the baseline;
   - the rustc witness (§ above): run `build` under two `RUSTUP_TOOLCHAIN` values if two are installed, or show why only one can be.
2. **The bridge:** the banner in the identity line, `ngspice` parsed from it, and `--version`. A banner the bridge cannot parse is refused by name. It never falls back to the constant (`INV-06`).
3. **`doctor`:** the version field, from `--version`. A bridge that cannot answer is reported, as `ngspice_loaded` is today, with exit `1` (`1604` D2).
4. **`build`:** the rustc release and commit in the verdict.
5. **Tests and mutants:**
   - the archived `ngspice` comes from the banner (a mutant that restores the constant is killed);
   - a banner with another version is still refused;
   - `--version` against the real library in tier 2, and against a stub in tier 1;
   - `build`'s rustc field against the witness §1.1 chose.

## 2. Acceptance

- **Every archived analog byte is identical to the baseline,** except the bridge identity digest where a record carries it. Show it by a differential on the committed analog fixtures.
- **`doctor` in the pinned image names ngspice 44.2.**
- **`build`'s verdict names the rustc that compiled the ELF,** and the ELF's bytes are unchanged.
- **The CLAUDE.md gates pass, and CI is green.** `TIER2_MIN` moves only by the tests this work order adds, stated.
- **If a `fab/` package's bytes move,** stop before READY: the package queue is serial, behind `1211e` and `1211c`.

## 3. Fences

- **Fence:**
  - `bridge/` (identity, answers, `--version`);
  - `crates/aethelgard-cli`: `analog.rs` (the digest and its readers), `doctor`, and `firmware.rs`'s `build`, with their tests;
  - `spike/1606/` and FEEDBACK.
- **Not:**
  - `verify`, `simulate` or coverage (`1211c`);
  - the IR, the twin, chaos or the trace format;
  - eda;
  - workflows, or `ci/` (except a weight row).
- **Running beside you:**
  - `1211c` (twin, chaos, CLI `verify` and `simulate`);
  - `1211e` (IR, specification 24, in the package queue);
  - `1215` (IR version table);
  - `1319` (eda);
  - `4240d` (a bench sheet).
  None of them touches `bridge/`, `doctor` or `build`. Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-1606/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A version reported that did not run:**
  - a bridge whose identity names a banner another library printed;
  - a `build` whose rustc field can differ from the compiler cargo invoked (a `RUSTC` or `RUSTUP_TOOLCHAIN` in the caller's environment).
- **A fallback:** any path that writes the constant `NGSPICE` or a default where the banner or rustc could not be read.
- **An archived byte that moves** beyond the digest.

## 7. Flag-and-stop conditions

- **Reading the banner needs a libngspice call outside the vendored `sharedspice.h`.** Stop and report it.
- **No witness shows which rustc cargo ran.** Stop with the measurement.
- **Any archived analog byte, ELF byte or `fab/` byte moves beyond the digest.** Stop and report it with the diff.
