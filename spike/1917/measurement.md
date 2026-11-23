# AET-WO-1917 §1.1 — the measurement (2026-11-23)

Base `77e829f1`; predictions committed first (`predictions.md`, `11082af5`). Scratch and raw logs in `~/.cache/aet-1917` (not committed: the logs are GitHub's).

## (1) Files `ardupilot-image.yml`'s jobs read, against `paths:`

The two `paths:` blocks are identical, item for item (P2 held). One independent subagent traced every job (scripts, files they source or read, COPY sources, mounts, cargo builds). I checked every unmatched claim at the source.

**Unmatched today** (each read on a pull request whose only change is that file, the workflow does not run):

| # | Input | Job | Read at |
|---|---|---|---|
| U1 | `container/IMAGE` | `foreign` | `$(cat container/IMAGE)`, workflow l.371-375. Also `image`, through `check-pins.sh:50,243-246` on `engine.Dockerfile`'s `FROM ${TOOLS_IMAGE}` |
| U2 | `bridge/Cargo.toml`, `bridge/Cargo.lock`, `bridge/build.rs`, `bridge/src/**` | `foreign` | `cargo build --locked --manifest-path bridge/Cargo.toml`, l.373; `bridge/build.rs:10-30` digests those files. `Cargo.*` is root-only |
| U3 | `boards/sentinel-01/**` (its `Cargo.toml`, `src/`) | `cli`, `foreign`, `pin-table` | root `Cargo.toml:56` path dependency, a dev-dependency of `aethelgard-svd-hal` (`Cargo.toml:25`); `Cargo.lock` names it, so every `cargo build --locked` in the workspace loads it, and `pin-table`'s `cargo test -p aethelgard-svd-hal` compiles it |
| U4 | `container/debian-bookworm.sources` | `image`, `ardupilot`, `generated`, `publish` | `check-pins.sh:70` parses every `*.sources` beside the Dockerfile and refuses a bad line |
| U5 | `container/Dockerfile`, `container/engine.Dockerfile` | `image` | `test-check-pins.sh:46-48` requires each to pass `check-pins.sh` |
| U6 | the tools Dockerfile's COPY sources: `KICAD-LICENSE.md`, `TOOLS-SOURCES.md`, `TOOLS-DEBIAN-GPL2.tsv`, `renode-nvic/**` | `image` | `check-pins.sh:198-208` refuses a COPY source that is missing or a script (reached by U5) |
| U7 | `container/TOOLS-DEBIAN-GPL2.tsv` | `image` | `test-debian-src.sh:77` checks it |

**How each was verified:**
- U3: `aethelgard-svd-hal/Cargo.toml:25` `sentinel-01-board.workspace = true`, and `Cargo.lock:105,785`. `cargo metadata --locked --offline` loads `boards/sentinel-01/Cargo.toml`.
- U1, U4-U7: each read at the line cited.
- **Repository `.cargo/` configs:** there is none at the root. `bridge/.cargo/config.toml` is not read by `foreign`'s build: cargo finds config from the working directory, `/repo`. It is under `bridge/**` anyway.

**Matched and read** (each confirmed): `ci/foreign-judge.sh` (listed), `models/renode/*.cs` (`models/**`), every `container/ardupilot*`, the pin and GCC/Debian source files, `crates/**`, `examples/**`, `Cargo.*` and `rust-toolchain.toml`.
- I read `ci/foreign-judge.sh` myself for every repository path it reads or runs. It runs `target/debug/aethelgard`, which U2 and U3 build. It reads `crates/aethelgard-cli/tests/fixtures/{sentinel-02-own,four-port}.json` (l.20, 160), and generates everything else into its own scratch.

**Unresolved:** what the `aethelgard` binary reads at run time in `solve`, `generate`, `check pinout` and `simulate`, beyond `models/renode` and the files named on its command line.
- The subagent grepped non-test sources for path literals and `include_*` and found nothing more. Nobody ran it under a file-access trace.

**Against P1:** predicted `container/IMAGE`, `bridge/**` and a tools Dockerfile (2-4 misses). Found 7 groups. U3 (`boards/sentinel-01`), U4, U6 and U7 were not predicted. Nothing under `ci/` beyond `foreign-judge.sh` is read.

**Today's paths test** (`image_workflow_paths.rs`) reads only `ardupilot.Dockerfile`'s COPY sources. U1-U7 are invisible to it.

## (2) The generated matrix against the pinout judge

- The matrix has 5 rows: 4 upload `generated-*` (sentinel-01, sentinel-02, serial-dshot-9600, four-port) and 1 uploads `mutant-*` (sentinel-02-noimu).
- `ardupilot-pinout.sh:73` hand-lists the same 4. No miss today (P3 held).
- **The gap:** a new `generated` row uploads an artifact that `pinout` downloads (`pattern: generated-*`) and never judges. The judge exits 0 with the board unread.

## (3) `examples/sentinel-01.json` — it generates

**P4 was wrong.** The host CLI gives, on `77e829f1`, the results below.
- §1.1 says "in the tools image". I ran the host build because the workflow's `cli` job builds on the runner and `generated` runs that binary there. The tools image is not in this path.

- `validate` exit 0 (project `sentinel-01-bench`), `solve` exit 0;
- `generate hwdef` exit 0, `not_emitted: ["U3 (TMP117) …"]`, which is the same informational line the minimal spec gives. There is no refusal and no `SenseNotEmitted`.
- **Against the minimal spec,** the generated file adds I2C2 (`EXT_SCL` PB10, `EXT_SDA` PB11, open-drain), `I2C_ORDER I2C1 I2C2`, and `HAL_I2C_MAX_CLOCK 100000` (was 400000).
- **Built with unmodified ArduPilot** in the pinned image (`container/ARDUPILOT-IMAGE`, `e1809b90`) under `capped-run --mem 8G --network none`, as `ardupilot-build.sh boards sentinel-01-full`: it built (19.6 s copter).
- **`check pinout`** on it against its own topology, with no allow-list: exit 0, `findings: []`, `decodes: elf and hwdef.h agree`, `identity: judged`.
  - This ran on the published image, not this branch's Dockerfile. No image input changed since the pin's revision `ab9d4663` (`git log ab9d4663..HEAD` on the Dockerfile, its build script, requirements and sources: empty).

So the stop condition "hwdef refuses sentinel-01.json" did not fire. **How it lands:** see the bus note's D1.
- The matrix row needs a committed fixture for its `cmp` (workflow l.296), in `crates/aethelgard-hwdef/tests/fixtures/`.
- The row is then held byte for byte by both fixture tests, `aethelgard-hwdef/tests/generate.rs` (`BOARDS`) and `aethelgard-cli/tests/hwdef_tier1.rs` (`BOARDS`). Both are outside the fence as written.

## (4) `route.sh inventory`'s missed literal forms (`route-forms.sh`)

Each form was planted alone in a scratch tree with `docs/work-orders/W.md`, `docs/reviews/R.md`, `fab/b/ORDER.md` and `README.md`.

- **Seen:** a plain path, a directory, and `{root}/docs/a.md`.
- **Missed, 14 forms:**
  - `format!("docs/work-orders/{}.md")` and `{x}.md`;
  - `docs/{}/W.md` and `docs/{}`;
  - `{}/docs/{}.md` and `../../docs/reviews/{}`;
  - the trailing slashes `docs/reviews/`, `docs/` and `../../docs/work-orders/`;
  - `fab/{}/ORDER.md` and `fab/b/`;
  - `{}.md`, `README{}` and `{}/{}.md`.
- `concat!("docs/reviews/", "R.md")` was seen only through the bare name `R.md`.

**Over today's tree** (`route-glob-count.sh`, 67,498 literals), reading each placeholder as "any run" and dropping a trailing slash:

| Reading | Literals newly reported |
|---|---|
| **anchored**: the literal holds a docs anchor outside its placeholders (`docs` segment, `fab/`, `README`, `LICENSE`, `.md`) | **0** |
| **any**: every literal whose wildcard reading matches some docs path or name (`{}`, `{v}`, `{}/{}` …) | **610 distinct literals in 234 files** |

**The zero is controlled.**
- Seven literals were planted in a copy of the list:
  - `docs/work-orders/{}.md`, `docs/reviews/`, `{}.md`, `README{}`, `fab/{}/ORDER.md`, `../../docs/{x}`;
  - `{}/{}`.
- The first six came out `anchored`, and `{}/{}` came out `any`.

So the anchored rule lands with no new `docs-tests.tsv` row. A fully unresolvable literal (`format!("{}/{}", a, b)`) stays a residual of a literal scanner, as `1911` F4 records for `concat!` and `format!` arguments.

## (5) `with-tools` files against `TIER2_RE` (P6 held)

- 11 test files carry `#![cfg(feature = "with-tools")]`. One, `attribute_seeded.rs`, gates a module instead (`#[cfg(feature = "with-tools")]` at l.474).
- All 12 are in `crates/aethelgard-cli/tests/`. `TIER2_RE` lists exactly these 12, and each name exists.
- No other crate or the bridge uses the feature: it is `aethelgard-cli`'s alone. `chaos/tests/archive.rs`'s `header_with_tools` is a function name.
- **Latent:** `TIER2_RE` matches cargo's `tests/NAME.rs` with no crate, so a same-named test file in another crate would be counted as tier 2.

## (6) The shard timer's split forms (`split-forms.sh`)

**Run `45198295391`, both attempts, 24 shard logs:**
- **One split line**, shard 5 attempt 1, job `137778234534`, log l.444-445:
  ```
  1795445949.129358	test a_patch_that_matches_zero_or_two_times_is_refused_naming_the_mutant ... ANSWER 3200: proposals/ absent: nothing to check
  1795445949.129384	ok
  ```
  - libtest writes `test NAME ... ` and its result as two writes, and another test's line landed between them.
  - `times()` (`shard.sh:283`) needs the result at the end of the line holding the name, so it timed 10 of 11 and refused.
- **The form `times()` already reads** (a tool line before the name) occurs in this run only on libtest's 60 s warning: `test tests/ui/fail/foreign_speed.rs [should fail to compile] ... test misuse_… has been running for over 60 seconds`, l.1667. `heavy_unweighted` matches its end.
- trybuild's own `test tests/ui/… ... ok` lines name no Rust path, and the timer correctly does not time them.

**A wider sample: 12 more CI runs, 144 shard logs** (`45198980248` … `45173993181`):
- 37,399 plain result lines;
- **zero** split lines of any form.

**Forms to read** (P7 held):
- (a) `TOOL test NAME ... ok`, read today;
- (b) `test NAME ... TOOL`, then later `ok` alone on a line;
- (c) both sides, `TOOL test NAME ... TOOL`, then `ok`. **Not observed in 168 logs.** It follows from (a) and (b) together, and is tested as reasoned.

**A lone result that is not libtest's:** the same shard-5 log, l.1667-1668:
- l.1667 is trybuild's `test tests/ui/fail/foreign_speed.rs [should fail to compile] ... `, with libtest's 60 s warning after it.
- l.1668 is `ok` alone. That is trybuild's result, not a pending libtest test's.
- So a lone result with nothing pending must not be counted. One with a name pending could be another tool's, which would make the time early, never a pass: the count's `n == want` and the tally's `test result:` line are unchanged.

Fail closed:
- a second name opened while one is pending, or a `test result:` line with a name pending, is refused;
- a lone result with nothing pending is not a test;
- `TOOLok`, where a partial tool line runs into the result, stays unread and is refused as today (`timed n of m`);
- the tally's pass count still comes from libtest's own `test result:` line, never from per-test lines.

## Decisions for the architect (the §1.1 stop), each with my lean

- **D1 — (3) lands as a new row; the fixture and two test arrays sit outside the fence.**
  - The row needs a board name other than `sentinel-01`, which the minimal row owns for its artifact and its fixture. Lean: `sentinel-01-full`.
  - It needs a committed `crates/aethelgard-hwdef/tests/fixtures/sentinel-01-full.{hwdef.dat,topology.json}` for the `cmp`.
  - It needs a row in each of `aethelgard-hwdef/tests/generate.rs` `BOARDS` and `aethelgard-cli/tests/hwdef_tier1.rs` `BOARDS`, which hold every CI board's fixture byte for byte.
  - Dropping the `cmp` for this row would make a gate require less, so it is not offered.
  - Lean: widen the fence to those two files and the fixture pair, with no generator change.
- **D2 — (1) the scope of `paths:` and where the test learns the inputs.**
  - Lean: add `container/**` (it replaces the 24 `container/...` entries and covers U1 and U4-U7), `bridge/**` and `boards/sentinel-01/**`.
  - `container/**` costs almost nothing: `crates/**` already runs this workflow on nearly every PR.
  - The test should derive its inputs, not hold a second hand list:
    - (a) the COPY sources of every Dockerfile under `container/`, and the Dockerfiles themselves;
    - (b) every `$(cat F)`, `head -n 1 F`, `bash F`, `python3 -I F` and `--manifest-path D/Cargo.toml` file in the workflow's `run:` blocks;
    - (c) every workspace `path =` dependency in the root `Cargo.toml`, as `DIR/**`.
  - Each must match both lists, and each reader is proven by a planted miss. U4 and U7, a sibling glob and a loop in a test script, are what such a reader cannot see. `container/**` covers them; a narrower list would need them named.
- **D3 — (4) the mechanism.** The work order says an unresolvable literal "routes the change to the full run". Today's mechanism is to report it and refuse it until it is classified in `docs-tests.tsv`, where a `reads` row runs on docs-only PRs.
  - Lean: the anchored reading. A placeholder or trailing-slash literal with a docs anchor outside its placeholders is read as a wildcard and reported as a row, refused until classified. Today that adds 0 rows.
  - The fully unresolvable `{}`/`{}/{}` forms (610 literals in 234 files) stay a recorded residual, beside `1911` F4.
  - Routing every such literal to the full run would end docs-only routing. It is not offered.
- **D4 — (5) what a "with-tools file" is.**
  - Lean: any `tests/*.rs` in any crate or the bridge whose source carries a `feature = "with-tools"` cfg, at file or item level, so `attribute_seeded.rs` counts.
  - The check is both ways: such a file missing from `TIER2_RE` fails by name, and a `TIER2_RE` name with no such file fails by name. `TIER2_RE` itself is unchanged.
  - Lean: a Rust test beside `image_workflow_paths.rs`, with a planted file.
- **D5 — (2) how the judge knows the set.** `pinout` merges `ardupilot-*` and `generated-*` into one directory, so the script cannot see artifact names. A generated board's directory is the one holding `topology.json`.
  - Lean: `ardupilot-pinout.sh` takes the workflow file and reads the `generated` matrix's rows that have no `drop`. It requires the generated directories to equal that set, judging each against its own topology.
  - A matrix board with no directory, a directory not in the set, and an empty set are each refused by name. The four hand boards stay as they are.
  - The planted-row test needs either a stub `BIN` run of the script or a new `container/test-ardupilot-pinout.sh`; the second is outside the fence. Lean: the stub run, from a Rust test beside the paths test.
- **D6 — (6) no decision.** `times()` keeps one pending name after `test NAME ... TOOL`, and the next line that is exactly `ok`/`FAILED`/`ignored` (stamp stripped) closes it.
  - Refused: a second name opened while one is pending, and a `test result:` line or the log's end with one pending.
  - A lone result with nothing pending is not counted (l.1668's trybuild `ok`).
  - The tests are built from l.444-445 of the real log, with form (c) as reasoned, and a mutant for each guard.
