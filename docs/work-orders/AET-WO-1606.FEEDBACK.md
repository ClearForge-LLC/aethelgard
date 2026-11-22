# AET-WO-1606 FEEDBACK — tool identities from what ran

Builder: Claude (Opus 5.5), a fresh `1606` session, 2026-11-22.
- Worktree `~/aethelgard-1606`, branch `wo/aet-1606-tool-identities` from `origin/main` `122378c7`.
- The work order is read from `~/.cache/aet-1606-wo.md`, the architect's copy (it is not on `main` yet).

Labels: **measured** (run here), **read** (read at source, file:line), **inferred** (reasoned, not run).

## 1. The measurement (§1.1), predictions first

- **Predictions:** `spike/1606/PREDICTIONS.md`, committed `a266ef1d` before any run.
- **Outcomes:** `spike/1606/MEASURED.md`, prediction by prediction.
- **Scripts:** `spike/1606/sh/`; outputs in `spike/1606/out/`.
- **Where:** all in the pinned tools image under `capped-run`.

Findings, in brief [measured]:
- **Carriers:**
  - The digest is committed only in `analog.rs`, plus a prose prefix in `AET-WO-2304b.FEEDBACK.md`.
  - `tests/common/mod.rs` recomputes it. The work order's "`doctor_tier1`'s copy" is that computed one; `doctor_tier1` holds no literal.
  - No golden, example or fixture carries it.
  - It does reach `fab/sentinel-01-bench/package/MANIFEST.json`, though, through the campaign state hashes in `verify`'s verdict (§6). I predicted otherwise; the bench measurement found it.
  - `build`'s version string reaches `fab/` only through `verify`'s `build` gate (`tool_version`, read from `build`'s `tool.version`, `verify.rs:603`) inside each `MANIFEST.json`'s `verdict_sha256`.
  - So `tool.version` stays cargo's line, and the compiler is a new field.
  - No tier-2 test pins a trace-derived hash as a literal: each compares two runs, or a recomputation (`grep` of `state_sequence_hash`, `verdict_sha256` and 64-hex literals over the tier-2 files).
- **Baseline differential** (`out/compare.txt`):
  - **Setup:** `main` and this branch, built in the image, on `sentinel-01-minimal` (I2C solve, `--rail`, `--renode`) and `sentinel-03` (`--chain`).
  - **Byte-identical:** 36 of 52 files.
  - **Equal once the digest is substituted:** the 6 traces.
  - **Derived values:** 8 files, each checked to be derived. These are each manifest's `sha256(trace)` and state hash, and each campaign line's `state_sequence_hash`, which is that state hash.
  - **`build.json`:** the 2 differ by the new `tool.rustc` and the run path.
  - Both ELFs are identical.
- **The rustc witness:** (b), cargo's `.rustc_info.json` in the fresh target (§3 F3).

## 2. What is built

- **Bridge** (`bridge/src/solver.rs`, `main.rs`):
  - `solver::banner` reads the one log line starting `** ngspice-`, which must be `** ngspice-<digits and dots> shared library`. None, two or more, or one that does not read so is refused by what was read.
  - `pinned` then refuses a release other than `44.2`. `NGSPICE` is now private to `solver.rs` and appears only in that comparison.
  - `Ngspice` holds the `Banner`. `record`, `solve_rail` and `solve_chain` write `spice.banner().release`.
  - **The identity:**
    - In a mode that starts the library and accepts it (a request, `--rail`, `--chain`, `--version`), the identity is printed after the library has started, with `ngspice` (the release) and `ngspice_banner` (the line, verbatim).
    - A refused request (exit 2), a refused library (exit 3), a usage error (exit 2) and `--renode` print the bare identity, as today. Every exit code and refusal kind is `main`'s (§3 F1).
  - **`--version`:** starts the library, prints the identity alone, exit 0. Any further argument is usage, exit 2.
- **CLI** (`crates/aethelgard-cli/src/analog.rs`):
  - `identified` returns `Identity { source_sha256, ngspice: Option<String> }`.
  - It accepts exactly the 3 fields, or 5 with both `ngspice` (which must be `NGSPICE`) and `ngspice_banner` (which must read `** ngspice-<that release> shared library`). Anything else is `BridgeMismatch`, exit 4.
  - Each of the three measuring paths requires the identity's `ngspice` once the bridge exited 0 (`Identity::ran`, else `ToolError`). It holds every answer to that release, not to the constant, and archives that value: for the chain, where it used to write `NGSPICE`; and in `AnalogRecord.ngspice`, now a `String` (`verdict.rs`), where `judge` used to write `NGSPICE`.
  - `boot` (`--renode`) takes either shape, because nothing it reports comes from the identity but the digest, and that it drops.
- **`doctor`** (`doctor.rs`):
  - It probes the bridge with `--version`, which must exit 0 with the identity alone, naming ngspice. It reports `"ngspice":"44.2"` beside `ngspice_loaded: true`.
  - Anything else is `ToolUnusable` in `missing`, exit 1 (`1604` D2), with what the bridge said.
- **`build`** (`firmware.rs`, `verdict.rs`):
  - `tool.rustc`, `{"release","commit_hash"}`, comes from the one successful `-vV` answer in `<target>/.rustc_info.json`, read with `read_beneath`. It is read for a pass and for `build_failed` alike.
  - No record, or one without exactly one `-vV` answer, or without exactly one non-empty `release` and `commit-hash`, is `ToolError`. Nothing is asked twice.
- **`BRIDGE_SOURCE_SHA256`:** `5bdc40e4…` → `2e95092a…`. Its pin test passed; `common`'s recomputation follows the files.
- **`crates/aethelgard-cli/README.md`:** the bridge's identity, the header's `ngspice`, `doctor`'s bridge line, and `build`'s `tool.rustc`.
- **Tests:** §5.

## 3. Findings and forks taken

- **F1 · The identity names ngspice only in a mode that started the library and accepted it · `bridge/src/main.rs` · fork.**
  - **The choice:** the work order says "its identity line carries the banner". Today the identity is printed before anything, in every mode. Carrying the banner means starting the library first.
  - **What I did:** in the library modes, the request is parsed, then the library is started, then the identity is printed. A refused request or library prints the bare identity, then the refusal, exactly as today: every exit code, kind and order is `main`'s.
  - **Why not start the library everywhere:**
    - `--renode` runs no ngspice. Starting it there only to name it would report a version that did not run (§5 of the work order).
    - It would also change the Renode boot's process (ngspice's init, its code models, under Renode's grants) for nothing it reports.
    - The usage path likewise stays bare.
  - **The alternative:** start the library in every mode and name it everywhere. That needs a droop-boot differential, and makes `--renode` fail on a library fault that does not concern it.
  - **Cost:**
    - The CLI accepts two identity shapes. It requires the five-field one exactly where it reports ngspice.
    - The identity is no longer printed before the request is parsed. A bridge that crashed while parsing would now print no first line, and the CLI would report `BridgeMismatch` ("its first line is not its identity") rather than `ToolError`: exit 4 where it was 5. No answer is read either way.
- **F2 · An identity that names a banner it did not read · adversarial · measured, §4.** Covered by "exactly one banner line".
- **F3 · The rustc witness is cargo's own record · `firmware.rs` · measured (`spike/1606` P4).**
  - **The `alt` toolchain:** a private `RUSTUP_HOME` with a toolchain `alt` (1.99.1's files; its `rustc` a wrapper that answers `-vV` with another release and commit).
    - It gives one `cargo --version` and one ELF. Today's verdict, `cargo 1.99.1 (5f94df478 2026-08-27)`, cannot tell it from 1.99.1, which is `3301` L1.
    - `.rustc_info.json` names `1.99.1-alt`, and so does the branch's `tool.rustc`.
  - **A `.cargo/config.toml` above the copy naming `build.rustc`:** plain cargo runs that compiler, and its record names it. A separate `rustc -vV` under the same environment and working directory names 1.99.1. So a second probe is not a witness.
  - **`build` already refuses that configuration** (`CargoConfigRefused`, `3301` F13), so inside `build` I found no case where the two differ. (b) is chosen because it is this run's record, not a second question.
  - **`cargo --version`,** which `build` asks first with the same `CARGO_TARGET_DIR`, writes no record and creates no target (measured). The record is the build's.
- **F4 · Both witnesses are self-reports · residual.**
  - `.rustc_info.json` is what the compiler cargo invoked says to `-vV`. A compiler, or a `rustc-wrapper`, that lies to `-vV` is reported as it lies: under `alt`, the ELF's `.comment` still says `rustc version 1.99.1`.
  - Likewise the banner is what the library prints of itself. A library built to print `** ngspice-44.2 shared library` passes.
  - Neither is a content hash. Pinning libngspice's or the toolchain's bytes would be a later row.
  - `.comment` is not used: it is read from the artifact under judgment, and it is the code generator's self-report too.
- **F5 · Fence extensions, named.**
  - `verdict.rs`: `ToolReport.rustc` and `RustcReport`, skipped when absent, so no other verdict's bytes move. `AnalogRecord.ngspice` becomes a `String`, with the same bytes.
  - `tests/common/mod.rs`: `bridge_identity()` is now the five-field identity a library run prints, and `bare_bridge_identity()` the three-field one.
  - `board_tier2.rs`: one new test. `crates/aethelgard-cli/README.md`.
  - `bridge/tests/rail.rs` and `edges.rs`: the identity's two shapes, and `--version`.
- **F6 · The usage line names `--version` · `bridge/src/main.rs` · low.** No archive carries the usage line; the bridge prints it only on a usage error.

## 4. Adversarial pass (§5, targeted)

**A1 · A bridge whose identity names a banner another library printed.**
- **The source:** the banner is read from the `SendChar` log of this process's own `ngSpice_Init` [read, `solver.rs`]. The only libngspice in the process is the one the loader resolved for `-l:libngspice.so.0`, and the CLI runs the bridge with an empty environment, so no `LD_PRELOAD` or `LD_LIBRARY_PATH` reaches it [read, `analog.rs`, `env: Vec::new()`].
- **What else writes that log:** ngspice runs a `.spiceinit` from its working directory before it prints its banner, and that file's `echo` lands in the same log [measured, `spike/1606/sh/adv.sh`, `out/adv.txt`]:
  - `echo ** ngspice-44.2 shared library` there → **refused**, exit 3: "the library printed 2 banner lines, not one". The log shows the echoed line first, then the library's own.
  - `echo ** ngspice-45.0 shared library` → refused, the same way.
  - A `.spiceinit` in `HOME` alone, with a clean working directory → not read (one banner, exit 0).
  - The CLI's invocation is a fresh directory of its own and an empty environment → one banner, exit 0.
  - So "exactly one line starting `** ngspice-`" is what stops an added line. No line can remove the library's own.
- **Residual, as F4:** a different library that prints `** ngspice-44.2 shared library` and nothing else of the kind is accepted. So is a `.spiceinit` echoing it beside a library that prints no banner, but the CLI never gives the bridge such a file.

**A2 · A `build` whose rustc can differ from the compiler cargo invoked.**
- **`RUSTUP_TOOLCHAIN`:** passed through and honoured by both cargo and the record: nightly is named on a failed build [measured, P4.2; tier-2 `build_names_the_rustc_cargo_ran`].
- **`RUSTC`, `RUSTC_WRAPPER`, `CARGO_BUILD_RUSTC`:** never reach cargo, because the environment is emptied but for four variables [read, `firmware::toolchain`].
- **A `build.rustc` in a configuration file:**
  - one in the crate is refused unless it is the generated one;
  - one above the copy is refused, `CargoConfigRefused` [measured, P4.4];
  - one in cargo's home is outside the sandbox's read grant [read, `toolchain`'s `read_only`: cargo's directories and `RUSTUP_HOME`].
- **Whichever configuration cargo did read, the record names the compiler cargo ran** [measured: the ancestor case with plain cargo]. That is why the record, not a probe, is the witness.
- **Residual:** F4.

**A3 · A fallback.**
- The bridge holds no constant outside `solver::pinned` [read: `grep NGSPICE bridge/src` gives the definition and `pinned` only].
- The CLI uses `NGSPICE` only in `identified`'s comparison. Every archived `ngspice`, on the chain path and in `AnalogRecord` included, is the identity's [read].
- `doctor` writes no version it was not given. `build` refuses a missing record rather than asking `rustc -vV` [read; mutants `cli-ran-default`, `doctor-accepts-bare`, §5].

**A4 · An archived byte that moves beyond the digest** [measured, §1; `out/compare.txt`, `out/bench-package.txt`]:
- None in the analog fixtures' archives.
- In `fab/sentinel-01-bench/package`, `MANIFEST.json`'s `verdict_sha256` moves, because `verify`'s verdict carries the campaign's state hashes, which are hashes over the digest-carrying traces. That is §2's stop (§6).

## 5. Tests and mutants

**Tests added or changed:**
- **Bridge, tier 1** (`solver.rs`, `main.rs`):
  - `the_release_is_read_from_the_banner_and_the_pin_refuses_another`: a `45.0.1` banner parses, and the pin refuses it.
  - `a_banner_absent_repeated_or_unreadable_is_refused`: seven cases.
  - `each_outcome_is_its_documented_line` now writes a `45.0` release through `record`.
  - `the_identity_line_is_the_documented_one`: both shapes.
- **Bridge, the `tools` job** (real libngspice; the bridge suite, not counted in `TIER2_RE`):
  - `edges.rs::version_names_the_banner_the_library_printed`: `--version` alone, five fields, `44.2` and its banner; `--version x` is usage with the bare identity.
  - `edges.rs`'s helper holds every answer's `ngspice` to the identity's.
  - `rail.rs::identity_ok` takes the two shapes: the solved rail's is five fields, and the refusals' and `--renode`'s three.
- **CLI, tier 1:**
  - `analog.rs`: the identity's shapes, six new refusals, and `Identity::ran`. `answers` and `rail_answer` hold every line to the identity's release (`44.3` refused), and `judge` records the release it is given (`45.0`).
  - `firmware.rs::the_rustc_is_the_one_vv_answer_cargo_recorded`: one pass and eight refusals.
  - `doctor_tier1`:
    - the stub answers `--version`, and `ngspice` is `44.2`;
    - `a_bridge_whose_version_names_no_ngspice_is_unusable`, three stubs, replaces `a_bridge_that_runs_without_a_request_is_not_the_bridge`, whose premise (a usage probe) is gone.
  - `tests/common`: the stubs print the five-field identity.
- **CLI, tier 2:** `board_tier2.rs::build_names_the_rustc_cargo_ran`.
  - The pinned toolchain's release, held to `rust-toolchain.toml`'s channel, and its commit, against `rustc -vV`.
  - Under `RUSTUP_TOOLCHAIN` set to the image's one nightly: `build_failed`, exit 1, naming nightly's release and commit.

**Mutants** (`spike/1606/sh/mut.py`): one per guard this work order adds or changes, each against its own suite in the image. Each file's sha256 was checked restored after each one. **16 of 16 killed:**
- `record-constant`, `pin-dropped`, `banner-first-of-many`, `banner-release-unchecked`, `version-names-no-banner` (bridge);
- `cli-identity-pin-dropped`, `cli-identity-banner-unchecked`, `cli-ran-default`, `cli-answers-constant`, `cli-rail-constant`, `cli-judge-constant` (CLI);
- `doctor-accepts-bare`, `doctor-accepts-more`;
- `rustc-first-of-many`, `rustc-unsuccessful-read`, `rustc-field-repeated`.

After the reduction pass, the five whose code it touched were run again, and all were still killed.

**Equivalent mutants, named, not run:**
- `solve_rail`, `solve_chain` and the solve loop take `spice.banner().release`. A mutant that writes the literal `"44.2"` there instead behaves identically, because `pinned` has refused any other release before. The `record` unit test covers the write itself.
- Likewise `chain_answer`'s comparison, behind `identified`'s pin.

## 6. Gates, the package, and CI

- **Gates, in the pinned image in CI's layout** (the worktree read-write at `/repo`, `CARGO_TARGET_DIR=/repo/target/img`), on `4266f887` + `faaed30d`, before the merge [measured, `~/.cache/aet-1606/gates-final.log`]:
  - `cargo fmt --all -- --check`, and the bridge's: clean.
  - `cargo clippy --workspace --all-targets -- -D warnings`, the same with `--features aethelgard-cli/with-tools`, and the bridge's: clean.
  - `cargo test --workspace` and the bridge's suite (its `tools` tests against the real libngspice): **2668 passed, 0 failed, 0 ignored.**
  - Tier 2, the tests this work order touches: `board_tier2` `build_*` (3, the new one included, 0.76 s together) and `campaign_tier2` (10): all pass.
  - **After merging `main` `bf497733`** (`00c8a9dd`; `1215`, `4240d`, `1319`, no file of this work order's), in the same layout: fmt and the three clippies clean; workspace and bridge suites **2689 passed, 0 failed, 0 ignored** (`~/.cache/aet-1606/gates-merged.log`).
  - The differential, `doctor` and P4 were run again with those binaries: same results (`spike/1606/out/compare.txt`, `head.txt`).
  - **After merging `main` `c9d4096b`** (`1211c`, `1211f`, `1216`; the package queue's call), in the same layout: fmt and workspace clippy clean; the CLI unit tests, `doctor_tier1`, `campaign_tier1` and `boot_tier1` pass (202, 14, 61, 70). The differential against `c9d4096b` itself is the same as before: 36 identical, 6 traces by the digest alone, 8 derived, 2 by path; ELFs `74a731a8` and `e4906886` on both; `doctor` names 44.2 (`spike/1606/out/compare-c9d4096b.txt`, `runs-c9d4096b.txt`). `TIER2_MIN` 202 (`main`'s, after `1211c`) → **203**.
- **A layout effect, not this work order.** The first test run, with the target outside `/repo`, failed one test: `fab_tier1::a_rail_or_bus_no_fault_kind_reaches_is_named`, `BaseOutsideRoot`, because its scenarios' base lay under `CARGO_TARGET_TMPDIR`. It passes in CI's layout (11/11).
- **`TIER2_MIN` +1** (201 → 202 on `122378c7`; 202 → 203 after `main` `c9d4096b` raised it for `1211c`).
  - The one tier-2 test added is `board_tier2::build_names_the_rustc_cargo_ran`.
  - `edges.rs::version_names_the_banner_the_library_printed` is in the bridge suite, which `TIER2_RE` does not count.
  - The floor lives in `.github/workflows/ci.yml`, outside §3's fence: named, one line and its comment.
  - **A weight row,** `ci/test-weights.tsv` (the one `ci/` edit §3 allows): `board_tier2` is lock-serialised, so every one of its tests needs a row. CI's plan refused the first push's shards for the missing row, which I had wrongly taken to be needed only past 60 s. The row is a Furnace estimate: 0.40 s alone in the pinned image, × 2.8 (`1310`'s factor) = 1.12 s. `ci/shard.sh plan`, run locally in the CI layout, then passes ("guard ok").
- **The package (§2): a `fab/` byte moves, so stop before READY.**
  - **What I measured:** `verify --fab-out` for sentinel-01-bench with `main`'s CLI and with this branch's, in the pinned image on Furnace (`spike/1606/out/bench-package.txt`):
    - only `MANIFEST.json` differs, and only its `verdict_sha256`;
    - in `verify`'s verdict, only the five campaign runs' `state_sequence_hash` differ (and the manifest's own hash with them);
    - Gerbers, drill file, BOM, CPL, FAB-NOTES and the ELF are byte-identical.
  - **The cause:** each state hash is the replayer's hash over a trace whose header names the bridge digest, which this work order must move.
  - **The work order predicted no package would move.** The measurement wins (architect, bus 2026-11-22 05:56Z).
  - **Plan ruled:** the PR is AWAITING-PACKAGE. The queue is `1211e`, `1211c`, then `1606`; the `write=true` dispatch comes when the architect calls it, and its package is committed byte for byte before READY.
  - `main`'s own Furnace-derived manifest also differs from the committed one, in `verdict_sha256`: the committed package is CI's (as `2305` saw).
  - **The package, committed:** `ci.yml` was dispatched with `write=true` at `78b24572` (run `45004732000`, 08:51Z). Its `bench-package` artifact differs from the committed package in `MANIFEST.json`'s `verdict_sha256` alone (`860b1211…` → `6a10a189…`); every other file is byte-identical. That file is committed byte for byte, as downloaded.
- **CI:**
  - On `44a2e613`, the plan step refused every tier-2 shard for the missing weight row above.
  - On `10c78731` (run `44994141844`): every check is green but `bench-package` (`MANIFEST.json` differs from the derived file, and nothing else, as measured). Tier 2: **203 passed, floor 202.**

## 7. Reduction pass

Reduction pass (diff): −4 net (doctor −9, `Identity.version` +5) · 1 duplicate removed · 1 dead path removed · 1 protected kept · 0 findings below.
- **Removed:** `doctor`'s second parse of the identity's `version` (`identified` had already parsed it and required it), together with its unreachable "its identity names no version" branch and an unused import. `Identity` now carries `version`.
- **Kept and labelled (`belt-and-suspenders:`):** `identified`'s check that `ngspice_banner` reads `** ngspice-<release> shared library`.
  - The bridge's parser already holds it, but the CLI cannot link that parser and judges what its tool reports.
- **Looked at and kept:**
  - The two stub identities in `tests/common`: one each for the two shapes.
  - The empty `Pass` built in `measure` for no decks: `judge` would need an ngspice it was never given.

## 8. Decisions needed

- **D1 · F1: the identity names ngspice only in a mode that started the library and accepted it.** `--renode` and every refusal keep the bare identity. Recommendation: accept. Starting ngspice under Renode only to name it would report a version that did not run.
- **D2 · The bench package's `MANIFEST.json` moves** (`verdict_sha256`, through the digest-carrying traces).
  - Every other package byte is unchanged.
  - Recommendation: queue the `write=true` dispatch after `1211e` and `1211c`, then commit its package byte for byte.
- **D3 · F4's residual (self-reported versions).** Recommendation: a backlog row, if a content hash of libngspice and of the toolchain should be archived beside the names.
