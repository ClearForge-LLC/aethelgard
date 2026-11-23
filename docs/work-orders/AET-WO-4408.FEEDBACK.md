# AET-WO-4408 — FEEDBACK: the ArduPilot matrix grows (a measurement, then a build)

**Branch:** `wo/aet-4408-matrix` from `main` `7fbe5d11`.
**Builder:** Claude (Opus 5.5) as project manager. Subagents (Sonnet) did volume work only: transcribing the DS12117 tables and checking them by eye. Gate logic, pins and workflows stayed single-threaded.
Written 2026-11-23.

**Status: READY** (§8). Predictions: `spike/4408/PREDICTIONS.md` (committed `ebd66a16` before any measurement). Results: `spike/4408/RESULTS.md`.

**The answer:**
- **WO §7's second stop fired, and was ruled and cleared.**
  - `main`'s tools-image publish after #339 failed (run 45105034798) on a smoke flake: `ldconfig -p | grep -qF` under `pipefail`, 2 runs in 1000.
  - Reported 07:11Z. Ruled 08:00Z: (a) the architect re-ran it; (b) this PR carries the one-line fix (§2.5b).
  - Attempt 2 published `aethelgard-tools@sha256:1e0ab105…9c63` (08:25Z), and **`IMAGE` names it**.
  - Its dpkg list equals a `--no-cache` build of `main`'s Dockerfile.
  - The bench package's verify under the pulled digest derives the same 17 files as under `e5fc7ddb`, and 16 equal `fab/` (§2.4).
- **`ARDUPILOT-IMAGE` re-pinned** to `main`'s newest publish, `e1809b90…` from `ab9d4663`.
  - Its dpkg list equals `160f9955`'s (171 packages).
  - Sentinel-01's and four-port's ELF and `hwdef.h` are byte-identical under both images.
- **The four-port board** (`four-port.json`, specification 28, 4404's measured `fc-base`) is in the generated matrix.
  - It is held in tier 1.
  - It builds with unmodified ArduPilot and passes `check pinout` with no finding [measured locally].
- **ST's pin table** is held against DS12117 Rev 10 (tier 1) and ArduPilot's `STM32H753xx.py` (`ardupilot-image.yml`, ruled D1 (a)).
  - All three give the same 167 AF rows in the solver's families.
  - ArduPilot shares 603 rows, with 0 disagreements.
  - 142/144 pads equal. Pads 28 and 29 are `PC2_C`/`PC3_C`, cited and allowed exactly (§3 A1).
- **`4407` Q1, Q4, Q5:**
  - `paths:` take `debian-trixie.sources`, held by a tier-1 test over the Dockerfile's `COPY`s;
  - the GtkSharp rehearsal and publish seed from the published mirror;
  - Q5 is confirmed (§2.5).

## 1. The measurement (§1.1)

Predictions were committed before any measurement. Their outcomes:
- **D:** I read the digests before predicting and recorded them as facts.
- **A1, A2, H1–H10, P1–P3, P5–P7, G1:** held.
- **P4:** held for 142 of 144 pads. The two exceptions are the finding in §3 A1.
- **B1:** §2.4.

## 2. What was built

### 2.1 The four-port generated board

- **The fixture** is `crates/aethelgard-cli/tests/fixtures/four-port.json`, `serial-four.json` changed in 13 lines:
  - MAVLINK becomes the console;
  - SBUS becomes `GPS2`, 8N1 115200, not inverted, with its nets renamed;
  - the I2C bus runs at 100 kHz.

  This is 4404's `fc-base`, which pincheck's `FC_BASE` tests already model.
- **Why not `serial-four` as it is:** it has no console and two RC ports, so the generator refuses it (`NoConsole`, then `RcInputTwice`).
- **Its generated files** are `crates/aethelgard-hwdef/tests/fixtures/four-port.{topology.json,hwdef.dat}`. They are held byte for byte by both tier-1 holders of the CI boards: `hwdef_tier1.rs` (solve and generate through the CLI) and `generate.rs`.
- **In CI:** a `generated` matrix entry in `ardupilot-image.yml`, and `four-port` in `ardupilot-pinout.sh`'s list of generated boards. That list is hard-coded, so without that line the board would build and never be judged.

### 2.2 The pin-table test (`crates/aethelgard-svd-hal/tests/pin_table_sources.rs`)

- **Three sources, compared pair by pair, with no vote.** Every difference fails, naming the row and both values.
  - **ST–DS12117 pads**, both ways: 144 positions.
  - **ST–DS12117 AFs** in the solver's families (I2C SCL/SDA, SPI SCK/MISO/MOSI, U(S)ART TX/RX, TIM CH1–4, written from their names in the test, not imported).
  - **ST–ArduPilot:** every shared (pin, signal) on the package across all families (603), and the solver's families both ways (167).
- **The DS12117 data** is `tests/fixtures/ds12117/{pads,af}.tsv`. Each row carries its PDF page, and the header carries the PDF's sha256 `607b9ead…`. How it was transcribed is in `RESULTS.md` §P.
- **The ArduPilot leg (ruling D1 (a), 07:37Z):**
  - **Not `#[ignore]`d, unlike the ruling's literal words** (§3 A4): `ci/shard.sh` refuses any ignored test in the workspace. It is the module `ardupilot_leg`, compiled only under `RUSTFLAGS="--cfg aethelgard_ardupilot"`, so every normal `cargo test` lacks it entirely.
  - `ardupilot-image.yml`'s new `pin-table` job copies the file out of the pinned image and runs the leg with that cfg and `AETHELGARD_ARDUPILOT_AF_TABLE`. It requires the test's `ANSWER ardupilot shared=` line, so a build without the cfg (0 tests) fails rather than passes [measured both ways].
  - The file's sha256 `58fd32c3…` is pinned, and any other file is refused by name, as is an unset variable.
  - No ArduPilot byte is committed. The roadmap row's "tier-1" holds for the planted half only: the planted tests use lines I wrote in ArduPilot's format. The ArduPilot table itself is GPL-3.0-or-later, and `container/ARDUPILOT-LICENSE.md` keeps every GPL byte out of the crates, whose `tests/` ship to crates.io (98 files in svd-hal's package, measured). (b) was not taken.

### 2.3 The re-pins

- **`ARDUPILOT-IMAGE`:** `160f9955` → `e1809b909b02…c2b6`, `main`'s publish from `ab9d4663` at 07:13Z.
  - The image carries the revision label.
  - Its dpkg list is identical to the old image's.
  - ArduPilot builds byte-identically in it (A1, A2).
- **`IMAGE`:** `e5fc7ddb` → `1e0ab105ec42…9c63`, `main`'s publish from `9958f594`, run 45105034798 attempt 2 (`publish` job 137569487351, 08:25:19Z).
  - The image carries revision label `9958f594`.
  - Its 669 dpkg lines equal the `--no-cache` Furnace build's. Against `e5fc7ddb` only `linux-libc-dev` moves, `6.12.115-2` → `6.12.117-1`, `4407`'s ruled move.
  - The same publish pushed the GtkSharp mirror (`…:GtkSharp-964cb9c…` at `03cf1db8…`).

### 2.4 The bench package under the re-pinned tools image

- **The rehearsal (`RESULTS.md` §B):** CI's exact `verify --against-fab`, run three times: under the published `1e0ab105` pulled by digest, under `main`'s tools image built `--no-cache` on Furnace, and, as a control, under the old `e5fc7ddb`.
  - All three build the same CLI and bridge, and derive the same 17 files. 16 equal the committed package byte for byte.
  - `MANIFEST.json` (`d47031ce…` in all three) differs from the committed one only by Furnace's sandbox record (`uid_boundary: false`), which the control shares.
- **So no `fab/` byte moves under the re-pinned content, and WO §7's first stop does not fire.**
- The acceptance is CI's `bench-package` job on this PR, which runs under `container/IMAGE` (§7).

### 2.5 `4407`'s follow-ups

- **Q1 (`paths:`).** `container/debian-trixie.sources` is in both lists.
  - `crates/aethelgard-cli/tests/image_workflow_paths.rs` (tier 1) reads every `COPY` source of `ardupilot.Dockerfile`, plus the Dockerfile itself, and requires a glob in both lists to match each one.
  - It refuses, rather than misreads, the forms it does not take: JSON form, continuation, wildcard, `..`.
  - Before the fix it failed naming exactly `debian-trixie.sources` under both events.
  - It is tier 1 because the workflow cannot guard its own `paths:`.
- **Q4 (the GtkSharp seed).** `gtksharp-source.sh` publish now takes `gcc-source.sh`'s `CANON` pattern:
  - the published mirror first;
  - GitHub only when ghcr answers 404;
  - fail-closed ("GitHub not asked") on any other refusal.

  The rehearsal asserts which source it took. While ghcr has no mirror, that must be GitHub; once `main` publishes it, it must be `seeded from ghcr.io/…`. All three branches were measured against a loopback registry (`RESULTS.md` §G).
- **Q5 (the fence).**
  - `container/test-check-pins.sh` and `container/fixtures/README.md` are as #339 merged them. They are unchanged here, and `test-check-pins.sh` passes.
  - The edit sits beside check-pins' own fixtures and is confined to the tests. Confirmed and recorded.

### 2.5b The smoke flake (ruling (b), 08:00Z)

- **The failure it prevents** (`main`'s tools-image run 45105034798, job `image`, 2026-11-23T03:25:04Z):
  `FAIL    libngspice: ldconfig does not list libngspice.so.0`
  The library was listed. `ldconfig -p | grep -qF` runs under `set -o pipefail`; `grep -q` exits at its first match (byte 18,612 of 62,785), and ldconfig, still writing, dies of SIGPIPE (PIPESTATUS `141 0`).
- **The fix**, `container/smoke.sh`, one line: `grep -F … > /dev/null`. grep reads all of ldconfig's output, so no early close.
- **Measured in `main`'s tools image:**
  - the new form fails 0/1000; the old form fails 2/1000 in the same container;
  - an absent library name is still refused;
  - the whole smoke passes (17 `ok`).
- **Other `| grep -q` under `pipefail`** in `container/` and `ci/` read from `printf` of a few lines or from `head`. None has ldconfig's 62 KB producer, so none was changed.

### 2.6 Edits beside the named fence (flagged)

Each one is needed for something inside the fence to be checked at all:
- **`crates/aethelgard-hwdef/tests/generate.rs` and `crates/aethelgard-cli/tests/hwdef_tier1.rs`:** four-port added to each `BOARDS` array (3 → 4). Without it, a generator change that moves four-port's file would be caught only by `ardupilot-image.yml`.
- **`container/ardupilot-pinout.sh`:** four-port added to its list of generated boards. Without it, the matrix board is built and never judged.
- **`crates/aethelgard-cli/tests/workspace_shape.rs`:** svd-hal's file allowlist takes the two DS12117 fixtures, on 3303's precedent for `tests/ui/motors-pwm/topology.json`.
- **`ardupilot-image.yml`:** a new `pin-table` job, beyond "the matrix and `paths:`". It is ruling D1 (a)'s CI half.
- **No crate `src/`, specification other than `four-port.json`, `ci/`, or `fab/` byte changes** (`git diff --stat origin/main -- 'crates/*/src' ci fab examples` is empty).

## 3. Adversarial pass, targeted (§5)

**A1. A row both sources agree on that DS12117 contradicts.**
- **Planted:**
  - `a_row_only_ds12117_contradicts_fails` takes `PB6 I2C1_SCL`: AF4 in ST and in ArduPilot-format input, and AF6 in DS12117. It fails with exactly `PB6 I2C1_SCL: st AF4, ds12117 AF6`.
  - Mutants on the real fixture (an AF changed, a row dropped, two pads swapped) are each killed.
- **Found in the real data, at the pad level:**
  - ST's XML and ArduPilot (which has no package data) both give PC2 and PC3 their digital AFs. ST's table puts them at pads 28 and 29.
  - DS12117 prints those LQFP144 pads `PC2_C`/`PC3_C`. Note 4 says Pxy's AFs reach Pxy_C only through a closed analog switch.
  - RM0433 Rev 9 p. 549: on a package with only `PXn_C`, `PXnSO` resets to 0, so the switch is closed.
  - ST's model therefore matches silicon at reset. The test carries the two pads as a cited, exact allowance (`SWITCHED`: pad, printed name and pin must all match). It is not a normalisation of `_C`.
  - **Residual:** nothing checks that firmware leaves `PC2SO`/`PC3SO` at 0 (§5 Q3).

**A2. A re-pin that a cached layer hides.**
- **ArduPilot:**
  - The published `e1809b90` (from `ab9d4663`) has `Created` 03:18:27Z, the time of the `9958f594` build. Its layers came from the GHA cache, and only the label layer is new.
  - That is harmless here, and measured so, not assumed: it was pulled by digest, its dpkg list equals `160f9955`'s, and the ELFs are identical.
- **Tools:**
  - The `--no-cache` build of `main`'s Dockerfile holds 669 packages, the same count as `e5fc7ddb`. The only difference is `linux-libc-dev` `6.12.115-2` → `6.12.117-1` [measured], which is `4407`'s ruled move.
  - `main`'s failed build also built from the GHA cache.
  - The `--no-cache` build on Furnace is the uncached truth, and it reproduces the same smoke flake at the same rate. The flake is in the check, not in a stale layer.
- **General:**
  - A pin is checked by `check-ardupilot-pin.sh` for shape only. What the image holds is read back in the image: `pinned` smokes it, and this WO reads its dpkg list and label.
  - A cached layer cannot hide a changed `COPY` input, because BuildKit keys on content. It could only hide a change in what an unchanged instruction fetches, and the snapshot date fixes apt's answer.

**A3 (found while building).**
- The ArduPilot `pinout` list of generated boards is hard-coded, so a matrix board missing from it builds and is never judged. I added four-port there and did not generalise. §5 Q4 has the one-line generalisation.

**A4. `ci/shard.sh` refuses the ruled `#[ignore]` (found by CI, twice).**
- **On `0b102fd4`, the timer.** `#[ignore = "…"]` makes libtest print `test x ... ignored, <reason>`. The P3 timer (`times`) matches only a result line that ends in `ok|FAILED|ignored`, so it counted 1 of 2 and failed shard 11 ("tests/pin_table_sources.rs: timed 1 of its 2 tests").
- **On `81f66368`, the tally.** With a bare `#[ignore]` the timer passed, but `run` refuses any ignored test (`ci/shard.sh:310`, `die "$suite: $f failed, $ig ignored"`): "shard: ws: 0 failed, 1 ignored".
- **So no ignored test can live in the workspace.** The ArduPilot leg became a cfg-gated module (§2.2), and every ruled property holds:
  - no ArduPilot byte is committed;
  - it is absent from `cargo test`;
  - CI runs it with the variable;
  - an unset variable or a wrong hash panics by name.

  Only the mechanism changed (the cfg, not `#[ignore]`), and the job's `ANSWER` guard covers what the cfg adds: a run of nothing.
- **`#![allow(unexpected_cfgs)]` is file-level** in that test file. The workspace lints refuse an undeclared cfg name, an item-level allow cannot reach a cfg that removes its item (measured), and svd-hal's `Cargo.toml` takes its lints from the workspace.

## 4. Reduction pass (diff)

- **Cut:**
  - 36 DS12117 AF rows for pins the LQFP144 does not bond (the test only filtered them out). The filter became a refusal of any off-package pin, which is stricter.
  - A roundabout assertion in the planted test.
- **Kept:**
  - the two refusal tests of the ArduPilot reader (an unread line, a key given twice): each guards a silent misread;
  - `image_workflow_paths.rs`'s planted test: it is the only thing proving the reader is not vacuous on its glob forms.
- **Not touched:** `serial-four.json`. `four-port.json` duplicates 475 of its 488 lines, but `serial-four` is the twin's four-port fixture with SBUS. Deriving one from the other at test time is what `4404` did, and it is exactly what this WO replaces with a committed specification for CI.

## 5. Questions for the architect

1. **(ruled 08:00Z, done) The tools re-pin.** (a) Re-run: attempt 2 published. (b) The `smoke.sh` fix is in this PR. Nothing is open.
2. **The roadmap row (`4404` draft) for the run-time serial pull-ups and `USART_CR2`**, assigned to 4408 at roadmap L1054 and L1056, is not in this WO's text. It is not built here. Four-port's generated header carries 4404's "Serial pulls" line.
3. **PC2SO/PC3SO.** A board on PC2 or PC3 depends on `SYSCFG_PMCR`'s package-dependent reset. A rule that no generated firmware writes `PC2SO`/`PC3SO`, or a twin row for `PMCR`, would close it. Today no committed board uses PC2 or PC3 [not measured beyond the CI boards'].
4. **`ardupilot-pinout.sh`** could judge every `generated-*` artifact it is given instead of a fixed list. It is one loop, deferred as outside this WO.
6. **The cfg instead of `#[ignore]`** (§3 A4) is a departure from D1 (a)'s literal words, forced by `ci/shard.sh:310`. If you would rather keep the `#[ignore]`, that needs a `ci/` change: an allowance for named ignored tests in `run`, and the timer reading `ignored, <reason>`. Both are outside this fence.
5. **F7's third item**, `examples/sentinel-01.json` in the ArduPilot matrix (roadmap L1057), is not in the WO text, and is not built.

## 6. Gates

All three pass on Furnace at `fa847441` (`main` `3b59fa8d` merged; 2026-11-23):
- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets -- -D warnings`;
- `cargo test --workspace`: 165 suites, 2,892 passed, 0 failed, 0 ignored.

An earlier run at `7d8b7190` passed too (164 suites, 2,860, 1 ignored, then the ArduPilot leg). A run before that caught one failure of mine (`workspace_shape.rs`, §2.6), which is fixed.

Beside them:
- `bash container/test-check-pins.sh` and `test-check-ardupilot-pin.sh` pass;
- the ArduPilot leg passes under its cfg with the pinned file (603 shared rows, 167 family rows), and fails by name without the variable.

## 7. CI

**All four workflows are green on `fa847441`** (`main` `3b59fa8d` merged) **[measured]**: `ci` 45151496656, `ardupilot-image` 45151496655, `tools-image` 45151496660, `engine-image` 45151496649.

The acceptance lines, read from the job logs rather than from the green status:
- **The matrix's four-port board builds ArduPilot and passes `check pinout`:**
  - `generated (four-port)`: `ANSWER board=four-port jobs=4 wall_ms=260510`;
  - `pinout`: `four-port: exit 0` with `"findings":[]`.
- **The pin-table test:** `pin-table` gives `ANSWER ardupilot shared=603 families=167`. The DS12117 legs and the planted mismatches run in the shards. The planted mismatch fails by name (§3 A1, mutants in `RESULTS.md`).
- **The image pins name `main`'s published digests, and the bench package verifies identically under them:**
  - `ci`'s `bench-package` passes under `IMAGE` = `1e0ab105…`: `verify --against-fab fab/sentinel-01-bench/package` exits 0, so no `fab/` byte moves;
  - `pinned` smokes `ARDUPILOT-IMAGE` = `e1809b90…`.
- **The smoke fix:** tools `image`, `ok      libngspice` in both smokes.
- **The GtkSharp seed, live:** tools `source-rehearsal`, `GtkSharp-964cb9c….tar.gz seeded from ghcr.io/clearforge-llc/aethelgard-tools`. GitHub was not asked.
- **ci:** `tier2_tests_passed=206 floor=205`.

Earlier heads, for the record:
- `b81902a6`: `ardupilot-image` green, the first run of four-port and `pin-table`.
- `0b102fd4` and `81f66368`: `ci` red in shard 11, the two `ci/shard.sh` refusals of §3 A4. Every other job was green, `bench-package` included.

## 8. Signal

**READY.**
- **Built:** every item of WO §1 and §2.
- **The stop:** the §7 stop fired and was ruled and cleared (`IMAGE` re-pinned after the architect's re-run, with the `smoke.sh` fix carried).
- **Departure:** one departure from D1 (a)'s literal `#[ignore]`, forced by `ci/shard.sh` (§3 A4, Q6).
- **Untouched:** no crate `src/`, `ci/`, `fab/` or `examples/` byte changes.

The architect merges. This READY commit is the last push.
