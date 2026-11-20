# AET-WO-1210 FEEDBACK — a specification can declare that it draws no board

Builder: Claude (Opus 5.5), a fresh `1210` session, 2026-11-20, from ~09:40 UTC. Worktree `~/aethelgard-1210`, branch `wo/aet-1210-draws-no-board` from `origin/main` `943f7112`. The work order is read from `~/.cache/aet-1210-wo.md`, the architect's copy of the docs branch's `docs/work-orders/AET-WO-1210.md`; this branch does not commit it.

Labels: **measured** (run here), **read** (read at source, file:line), **inferred** (reasoned, not run).

## 0. The proposal (§1.1 and §1.2), written before any build

### 0.1 The field: `board.drawn`

- **Name and place:** `board.drawn`, a boolean beside `board.max_outline_um` and `board.forward`. `false` declares that no board is drawn from this specification. Absent, or `true`, is a drawn board, so every committed specification keeps its meaning.
- **Why this one, in one sentence:** it names the fact that rules 1 and 3 and the four refusals turn on (is this board drawn?), not a purpose, because the never-drawn files are the solver's search fixtures as well as the twin's stand-in (§0.2), and absent is the strict case, as `RUL-14`'s amendment on PR #191 requires of an optional key.
- **Shape:** by `mcu.reset_switch`'s precedent, mirrored.
  - The key is optional; absent reads as `true`, and `true` is not written back.
  - An explicit `null` is refused as the wrong type, as `test_points` and `mcu.reset_switch` refuse it.
- **Version:** specification 20, version lines only, by a renumbering script like `4240v`'s (`renumber19.py`); a version-19 file is refused by name (`UnsupportedSchemaVersion`). `4240a` also takes 20; the second to merge renumbers to 21.
- **Not chosen:**
  - top-level `twin_only: true` (the work order's other example), because `solve/stress.json` and `solve/thrash-pins.json` are never drawn and are not the twin's;
  - a key whose absence relaxes a check: absent here applies rules 1 and 3.

### 0.2 Who declares it (§1.2): the inventory

**The corpus:** the 27 committed specifications at version 19 (a JSON object with `mcu` and `voltage_domains`, outside `spike/`, `docs/` and `target/`): 21 CLI fixtures, `ir/tests/fixtures/sentinel-02-power.json` and 5 under `examples/`. The `spike/` files (versions 16 and 17) and the `1200` repros (version 1) are already refused by version and are not renumbered, as `4240v` left them.

**How "drawn" was found:**

- **Measured:** a local instrument, never committed, in `schematic::design`, the one function every drawing goes through. It logged the sha256 of each specification it drew, and the result. `cargo test --workspace` ran on Furnace with it (tier 1; every tier-2 test self-skips here): 2,393 passed, and 1 failed, `workspace_shape` refusing the instrument's own dependency. Each committed file's hash was computed by parsing and re-serialising it, as the instrument did.
- **Read:** every reference to each file that drew 0 times, in every test (tier 2 included), `.github/workflows/*`, `ci/`, `container/`, `scenarios/` and `fab/`.
  - The log is a lower bound. A CLI run whose environment is cleared does not log: `boot_tier1` `a_console_too_slow_for_the_boot_is_refused_before_it` runs `generate` on `serial-dshot-9600.json` itself, and that file logged 0.

| File | Drawn? | Evidence |
|---|---|---|
| `examples/sentinel-01-minimal.json`, `examples/sentinel-01.json`; CLI `connectors`, `d1-24-i2c`, `d3-24-i2c-12-spi`, `four-layer`, `four-layer-power`, `motors-dshot600`, `motors-pwm`, `pair-standin`, `sentinel-01-bench`, `sentinel-02-own`, `sentinel-03`, `serial-four`; ir `sentinel-02-power` | **drawn** | tier 1 drew each 1 to 288 times [measured]. `pair-standin` is a stand-in pair of nets, not of parts: `supply_tier1` and `with_tools` emit it [read] |
| CLI `serial-dshot-9600` | **drawn** | `boot_tier1.rs` L2268 `generate`s it, which writes `kicad/` [read] |
| CLI `spi-only` | **drawn** | `campaign_tier2.rs` L1088 `generate`s it [read] |
| CLI `two-bme` | **drawn** | `twin_tier2.rs` L1091 and `boot_tier1.rs` L1713 `generate` it [read] |
| `examples/sentinel-01-ext-bmp581.json` | **drawn**, at the bench | `3403` ran `verify` on it, recorded in `fab/sentinel-01-bench/BENCH-EXT-BMP581.md` L34–42 [read] |
| `examples/sentinel-02-bench.json` | **drawn**, at the bench | `3304`'s bench specification, `verify` passed on it [read: `AET-WO-3304` record] |
| `examples/invalid-zero-domain.json` | **never drawn**: `validate` refuses it | it stays refused; no declaration, which would change nothing (§0.3) |
| CLI `imu-baro-1v8` | **never drawn** | the twin's 1.8 V stand-in (`4210` D1). Its six users never draw it: `board_tier2` L144 and `twin_tier2` L1673 use the board generator and the twin's emitter, as "KiCad cannot draw the stand-ins"; `own_models_tier1` and `svd-hal` `board.rs` the same; `speeds_tier1` asserts "no board can be drawn (UnknownPart:"; `fab_tier1` L273 is refused by `validate` first [read]. Tier 1 drew it once, refused `UnknownPart` [measured]. **Gains the declaration.** |
| CLI `sentinel-02-shaped` | **unclear (§7)** | no test or CI job draws it: `twin_tier2`'s `motor_board` only [read, measured 0]. But `1314` and `1314b` emitted and extracted it in their differentials, and roadmap §7.1's router row names it as a board that fails a declared capacitance bound |
| CLI `solve/stress` | **unclear (§7)** | `solve_cli` only solves it [read, measured 0]. `1314b`'s differential drew every specification that solves ("22 of 22"), and the same §7.1 row names it |
| CLI `solve/infeasible-shared`, `solve/pigeonhole-spi`, `solve/thrash-pins` | **never drawn**: `solve` refuses each (`Infeasible`, measured: `emit` on `main` and on this branch), so no draw can start | as `invalid-zero-domain`: refused before any board; unclear whether to declare (§0.3) |

**Predicted** from the files' JSON, before any build: rules 1 and 3 refuse exactly one committed file, `imu-baro-1v8` (rule 1: U5 and U7, ICM-20948s on uncrossed bus `IMU`, no row maximum and none declared; rule 3: U6, ICP-10111). Every unclear file passes both rules undeclared. So the unclear list decides only whether `emit` refuses those files, never whether `validate` newly refuses one.

### 0.3 The §7 stop for four files, and what is built meanwhile

- **Built now:** `imu-baro-1v8` declares `board.drawn: false`. No other file is declared.
- **Q1, for the architect:** `sentinel-02-shaped`, `solve/stress`, and the three `solve` fixtures `solve` refuses: declare or leave?
  - **Recommendation: leave all five undeclared.**
    - The first two are drawable and have been drawn (`1314b`), and §7.1 keeps both as boards for a later router work order; a declaration would make `emit` refuse them.
    - The last three cannot be drawn either way. A declaration there tests nothing and adds a field no reader needs.
    - Undeclared is the strict case: rules 1 and 3 hold for each, and each passes them.
  - The other reading, the work order's letter ("never emitted in a test or in CI"), declares all five. It is a byte change to five fixtures and moves nothing else; I can apply it in one commit on the ruling.

## 1. What is built

**Commits:** `dddbdc1a` (this proposal, before any code), `8de76cde` (specification 20, version lines only), `d6cc9edc` (the build).

- **The field** (`ir/src/board.rs`): `board.drawn: bool`, `#[serde(default = true, skip_serializing_if = true)]`, as §0.1 proposed. `ir/README.md` and the version's doc comment name it.
- **Specification 20** (`8de76cde`), by `~/.cache/aet-1210/kit/renumber.py`, `4240v`'s `renumber19.py` with the versions as arguments:
  - 27 specifications' `"schema_version"` and 8 test files' version pins;
  - every line that moved only because a specification's bytes moved, each re-taken and, for each digest, shown equal to `main`'s with the old sha256 put back [measured]:
    - `boards/sentinel-01/` and `boards/sentinel-01-bench/`: the "Solved topology of specification sha256" line of 8 files each, nothing else (the architect's widening of 10:05Z);
    - `hwdef` fixtures (3 `.hwdef.dat`, 3 `.topology.json`) and `svd-hal` goldens (`ui/motors-pwm`, `ui/serial-four`): the sha line;
    - `solve_cli`'s two digests: with `5945da18…0722` and `0b511dc9…a9a3` put back, `fa009ce7…9a16` and `a62263f7…ba9f`, `main`'s;
    - `order_2203`'s three moved digests: with `ffd83362…3aa5` put back, `main`'s three exactly; the boot image did not move;
    - the CLI README's example `specification_sha256` prefix (`solve_cli` pins it): `5945da18…` to `9d6b2309…`;
    - `fab/sentinel-01-bench/BENCH-EXT-BMP581.md` L9: the ext-bmp581 specification's sha256 at 20 (`e12b4a22…541f`), which had stayed at 18's since `4240v`.
  - The IR's version list in `refuses_unsupported_schema_version` now refuses 19 and 21.
- **Rule 1, step 23 of `Specification::validate`** (`ir/src/drawn.rs` `pin_capacitance`), on a drawn board only. It is `extract`'s precondition, read at source at this tree (`eda/src/extract.rs` `lines` L1119–1283, `device_pin` L1062–1082), which is 1209's reading unchanged:
  - only SPI lines refuse; an I2C pin counts at UM10204's `Ci` or its declaration;
  - each SPI bus in document order, its devices in document order, named on the bus's SCK as `extract` names the first;
  - a row with a cited maximum passes; else a declaration; else `UnboundedPinCapacitance`, `extract`'s kind, with `id` the device and `path` `spi_devices[i].pin_capacitance`;
  - a bus a `power.crossings` entry carries is skipped: its device is drawn on the translator's B-side nets (`schematic_power::behind`), never on the bus's own, and `extract` refuses the translator there (`TranslatorPinUnbounded`).
  - Because `solve` runs `validate`, `extract`'s own check is now the second line for every drawn board, as 1209 §1 intended.
- **Rule 3, the CLI's `validate`** (`cli/src/main.rs` `drawable`, before the acknowledgement and drawing checks), through `Specification::drawable_parts(arms)` (`ir/src/drawn.rs`).
  - The CLI cannot read a specification's device lists (`RUL-16`'s compiled fence), and the IR cannot read `aethelgard_eda::schematic::KNOWN_PARTS` (eda depends on ir). So the IR walks the devices, in the emitter's draw order (I2C, then SPI, each in document order), against the set the CLI hands it: `KNOWN_PARTS`, which `1316` holds to the arms by test.
  - `PartNotDrawable`, `id` the device, `path` `…_devices[i].part`, message "row present, no emitter arm; the emitter draws: …".
  - No eda change was needed: `KNOWN_PARTS` is already public.
- **The refusal of a declared specification** (`cli/src/main.rs` `draws_a_board`): one kind, `DrawsNoBoard`, `id` the project, `path` `board.drawn`, exit 2, before anything is solved, drawn, read or written:
  - `emit` (after `validate`, before `solve`);
  - `extract` (after `validate`, before the board is read);
  - `verify` with `--fab-out` or `--against-fab` (after `validate`, before the scenarios are read). **"fab" read as the package path:** there is no `fab` subcommand; `fab::derive` is called only by `verify`'s two fab flags, so both refuse.
- **The declaration:** `crates/aethelgard-cli/tests/fixtures/imu-baro-1v8.json` gains `"drawn": false` as its board's first key, and nothing else. Q1 (§0.3) is open for five more.

## 2. Each rule's early refusal beside `main`'s late one

`main` is `943f7112`, its binary built from that tree; inputs and outputs are in `~/.cache/aet-1210/evidence/` [measured].

- **Rule 1.** The board is `examples/sentinel-01-minimal.json` with its flash `U4` replaced by an ICM-42688-P declaring no pin capacitance (`r1-main.json`; the IR test `imu_undeclared()` is the same board at 20).
  - `main`: `validate` → `{"ok":true,…}`; `emit` → `{"ok":true,…}`; then `extract` on the emitted board → `{"ok":false,"error":{"kind":"UnboundedPinCapacitance","id":null,"message":"the design puts U4 (ICM42688P) on `SPI_SCK`, a line the MCU drives, and the parts table cites no maximum for its pin capacitance"}}`, exit 2.
  - This branch: `validate` → `UnboundedPinCapacitance`, `id` `U4`, `path` `spi_devices[0].pin_capacitance`, "the design puts U4 (ICM42688P) on `SPI_SCK` of SPI bus `FLASH`, a line the MCU drives, and the parts table cites no maximum for its pin capacitance, nor does `spi_devices[0].pin_capacitance` declare one", exit 2 (`ir/tests/board.rs` `a_drawn_boards_spi_device_with_no_pin_bound_is_refused_by_name`). `emit`, `generate`, `simulate` and `solve` refuse it there too, since each runs `validate`.
- **Rule 3.** The board is `imu-baro-1v8.json` undeclared, with its two ICM-20948s declaring 10 pF each, so rule 1 holds (`r3-main.json`).
  - `main`: `validate` → `{"ok":true,…}`; `emit` → `{"kind":"UnknownPart","id":"U6","message":"`U6` is a `ICP10111`, which has no vendored model; known: BME280, TMP117, BMP581, W25Q64JVxIQ, ICM20948, ICM42688P"}`, exit 2.
  - This branch: the CLI's `validate` → `PartNotDrawable`, `id` `U6`, `path` `i2c_devices[1].part`, "`U6` is a `ICP10111`: row present, no emitter arm; the emitter draws: BME280, TMP117, BMP581, W25Q64JVxIQ, ICM20948, ICM42688P", exit 2; and `emit`'s own `UnknownPart` stands beside it as the second line (`emit_tier1.rs` `the_stand_in_undeclared_is_refused_by_validate_before_any_board`, which also shows rule 1 first on the same file with nothing declared: `UnboundedPinCapacitance` U5).
- **The stand-in, declared:** `main` validated it and `emit` refused it `UnknownPart` U6. This branch validates it (exit 0), and `emit`, `extract`, `verify --fab-out` and `verify --against-fab` refuse it `DrawsNoBoard` (`emit_tier1.rs` `a_declared_stand_in_validates_and_every_drawing_step_refuses_it_by_name`; the board and scenario paths given do not exist, so each refusal is before any read).

## 3. Acceptance (§2)

| Item | Result |
|---|---|
| **The field's name and place** | `board.drawn` (§0.1); `ir/tests/board.rs` `drawn_is_optional_and_only_false_is_written_back`: absent and `true` read as drawn and are not written back, `false` is, and `null`, `0` and `"false"` are shape errors |
| **The inventory** | §0.2; one declaration, five files on Q1 |
| **The rules** | §1 and §2 |
| **The coverage test** (`1209` §1.1) | `parts_arms_tier1.rs` `validate_refuses_a_drawn_board_exactly_where_extract_and_emit_would`: every one of the table's 7 rows, on its kind of bus with nothing declared, solved from the same board declared a stand-in. For each row, rule 1 refuses exactly when `extract::lines` refuses `UnboundedPinCapacitance`, naming the same device on the same line, and rule 3 exactly when `Drawing::of` refuses `UnknownPart`, naming the same device. Neither side is vacuous: rule 1 refuses ICM-20948 and ICM-42688-P, rule 3 refuses ICP-10111. The ICP-10111's row never reaches `device_pin`: `design` refuses it first, and it is an I2C part, so rule 1 passes it as `extract` would have, had it drawn. `1316`'s `minimal` now declares the row's typical pin capacitance on an SPI row with no maximum, so that test still draws every row through a `validate` that passes |
| **Nothing committed newly refused** | The 52 specification-shaped files `validate` identically on `main`'s binary (its tree) and this branch's (its tree): 26 pass on both, and the other 26 are refused by the same kind, id and message, except that the 16 old-version files say "not 20" for "not 19" [measured: `corpus-main.txt`, `corpus-branch.txt`] |
| **No emitted byte moves** | `emit` on all 27 committed specifications, each binary on its own tree: the 22 boards' `.kicad_sch`, `.bom.csv` and `.kicad_pcb` are byte-identical, 4 refusals are identical (`Infeasible` ×3, `ZeroNominalVoltage`), and `imu-baro-1v8` is refused `DrawsNoBoard` for `main`'s `UnknownPart` [measured: `emits.txt`] |
| **The bench package** | It moves: its `FAB-NOTES.md` and `MANIFEST.json` name `examples/sentinel-01.json`'s sha256, which the version line moves (`803882c7…4f9a` → `92abdb0d…dbbe`). `bench_board_tier1` `the_committed_package_is_bound_to_its_specification_scenarios_and_emit` fails until the architect's write=true dispatch; I join the queue (2306, 2305, 4241, then 1210 and 4240a in merge order) |
| **Gates** | below, §6 |
| **`TIER2_MIN`** | unchanged: every new test is in a tier-1 binary (`ir` `board`, CLI `emit_tier1`, `parts_arms_tier1`) |

## 4. The adversarial pass, targeted (§5)

[measured unless labelled; inputs in `~/.cache/aet-1210/adv/`]

- **A1, a drawn board that declares itself a stand-in to escape the early rules.** `adv/bench-declared.json` is `examples/sentinel-01.json`, the ordered board, with `"drawn": false`.
  - `validate` passes it. `emit`, `extract` and `verify --fab-out` refuse it `DrawsNoBoard`, before reading the board or the scenarios. `verify --against-fab` the same (`emit_tier1`).
  - **So no fabrication package can be made or compared from it:** `fab::derive` has one caller, `verify`'s package step, behind both flags [read: `verify.rs` L1532].
  - **Plain `verify`** draws the board in its own process (`generated()`), runs ERC, DRC and support on it, and then its `extract` gate exits 2 with `DrawsNoBoard`, which `verify` returns as its own refusal, "gate `extract`: …", exit 2 [read: `verify.rs` `gate`, L505–515; not run, it needs the tools image]. A named refusal, after work wasted.
  - **`generate` is not refused,** and writes `kicad/` (schematic, BOM, placed board) for it [measured: `adv/g/kicad/`]. It is the twin's entry: the board crate and platform the twin boots come from it, so refusing it would refuse what a stand-in is for. Those KiCad files are a drawing nothing in the engine packages, and nothing stops a person from taking them to a fabricator by hand. **F1 below.**
  - `check speeds` passes it, judging the declared capacitance against the board it draws internally [measured]. It is not a fab path.
  - The library calls `aethelgard_eda::schematic::emit` and `aethelgard_cli::generate::tree` do not read the declaration; only the CLI's subcommands refuse. That is the work order's line (§1.4 names subcommands, and eda's fence is a read-only set).
- **A2, a stand-in drawn somewhere after all.**
  - **Tier 1, measured:** with `imu-baro-1v8` declared, the workspace passes but for the bench package (§6). So no tier-1 test emits, extracts or packages it.
  - **Tier 2, read:** `board_tier2` L144 and `twin_tier2` L1673 build its crate with `svd_hal::board::generate_with` and its platform with `twin::platform::emit`, neither of which reads the declaration. `fab_tier1` L273 is refused by `validate`'s `LifecycleNotAccepted` before the new check, as before. No workflow, `ci/` script, scenario or `fab/` file names it [read].
  - `check speeds` draws it internally and still labels it "UNVERIFIED: no board can be drawn (UnknownPart:" (`speeds_tier1` passes).
  - The in-code stand-ins in `ir/tests/validate.rs` (`stand_ins()` and the 1.8 V case of `a_device_whose_io_range_excludes_its_bus_domain_is_refused`) are only validated; each now declares `board.drawn = false`.
- **A3, is rule 1 narrower or broader than `extract`?** The coverage test settles it per row on one bus. Across buses and devices, rule 1 walks the same order as `extract::lines`' SPI half, and the crossing skip matches `behind`. A crossed bus's device stays unjudged by both: `extract` refuses its translator first. `sentinel-02-power` (an ICM-20948 behind U13, declaring nothing) validates, as on `main`, and M3 below shows the skip is load-bearing.

## 5. Mutants (§1.5), one per guard this work order adds

Each mutant edits one guard and runs the suites named; the harness is `~/.cache/aet-1210/mut/mut.py`, the results `results.txt` [measured]. **10 of 10 killed.** M1, M2, M4 and M5 were run again after the reduction pass: killed.

| Mutant | Killed by |
|---|---|
| M1 rule 1's condition removed | `a_drawn_boards_spi_device_with_no_pin_bound_is_refused_by_name`, the coverage test, `the_stand_in_undeclared_is_refused_by_validate_before_any_board` |
| M2 rule 1 ignores the declaration | `a_declared_stand_in_validates_and_every_drawing_step_refuses_it_by_name` (the stand-in test), the coverage test (its stand-in no longer solves), the IR rule-1 test |
| M3 rule 1's crossing skip removed | `ir --test power`: `the_committed_fixture_with_its_crossing_validates_on_the_sensor_tables_own_delay`, `the_fixture_passes_every_new_check_and_names_each_condition` (`sentinel-02-power`'s U5, behind U13) |
| M4 rule 3's condition removed | `the_stand_in_undeclared_is_refused_by_validate_before_any_board`, the coverage test |
| M5 rule 3 ignores the declaration | the stand-in test |
| M6 the CLI's `validate` does not call rule 3 | `the_stand_in_undeclared_is_refused_by_validate_before_any_board` |
| M7 `DrawsNoBoard` ignores the declaration | the stand-in test |
| M8, M9, M10 `emit`, `extract`, `verify`'s fab flags each do not refuse | the stand-in test, each by its own command's row |

A first run printed every mutant as surviving with no test named. That was the harness: under `cargo test -q`, libtest prints no `test … FAILED` lines, so nothing was counted. It was re-run without `-q`; the table is that run.

## 6. Gates, the reduction pass, and the version

- **Gates on `d6cc9edc`** (Furnace, capped scope, `~/.cache/aet-1210/gates-1.log`): `cargo fmt --all -- --check` exit 0; `cargo clippy --workspace --all-targets -- -D warnings` exit 0; `cargo test --workspace`: 144 suites, 2,397 passed, 1 failed, `bench_board_tier1`'s package binding, which waits on the dispatch (§3). Every tier-2 test self-skips here; CI is the run of record.
- **Reduction pass (diff): −2 net · 0 duplicates removed · 0 dead paths removed · 0 protected kept · 3 findings below.** It ran through a fresh subagent, read-only, on `git diff 943f7112 d6cc9edc`.
  - **Removed:** the IR rule-1 test's last `verdict(&reference())`, which other tests already assert; the doc sentence now points to the coverage test for the cited-maximum case.
  - **Declined, with reasons:**
    - R1, dropping `emit`'s `UnknownPart` beside rule 3's refusal: that assertion is §1.5's "early refusal shown beside `main`'s late one".
    - R2, passing `KNOWN_PARTS` as one string instead of a slice: that is taste, and it would tie the IR's interface to a display separator.
    - R3, folding a local in `drawable_parts`: after rustfmt it is +2 lines, so it was reverted.
  - **Findings, not removed:** `board.drawn`'s rule is stated in its field doc, the version-20 doc, the IR README and the test doc, as versions 14 to 17 state theirs; `UnboundedPinCapacitance`'s variant doc restates `drawn.rs`'s; `KNOWN_PARTS.split(", ")` now appears in four places (`schematic.rs`, `parts_arms_tier1` twice, `main.rs`), and only an accessor would join them, which would be an addition.
- **The version:** this branch takes 20. If `4240a` merges first, the version commit is redone to 21 by the same script, in its own commit.

## 7. Findings and questions for the architect

- **Q1** (§0.3): declare `sentinel-02-shaped`, `solve/stress` and the three `solve` fixtures `solve` refuses, or leave them? Recommended: leave all five.
- **F1, `generate` draws a declared board** (A1). A specification declaring `board.drawn: false` whose parts the emitter can draw gets a `kicad/` tree from `generate`. Nothing in the engine packages it, but the drawing exists. Options:
  - (a) leave it;
  - (b) `generate` omits `kicad/` for a declared specification, so the twin still gets `board/`. That changes `generate`'s output tree, a later work order;
  - (c) `generate` refuses one, which breaks the twin's entry for any drawable stand-in.
  - **Recommendation: (b).** Today's one declared file cannot be drawn at all, so (a) costs nothing until a drawable stand-in is declared.
- **F2, plain `verify`** of a declared board draws it and runs ERC, DRC and support before its `extract` gate refuses `DrawsNoBoard` (A1, read). It is a named refusal, after wasted work. Refusing every `verify` up front is one line, but it goes beyond §1.4's "`verify --fab-out`", so it is not built.
- **F3, the CLI README** lists no kind this work order adds. Its example hash moved with the version (a test pins it). The exit-2 row's kinds wait on the fence question on the bus.
- **F4, `BENCH-EXT-BMP581.md` L9** had stayed at specification 18's sha256 through `4240v`. It is now at 20's, by the 10:05Z widening.

## 8. The rulings of 11:10Z and 11:20Z, and what was built for them

`~/.cache/aet-bus/1210.md`.

- **Q1: leave all five undeclared.** Done; no file changed. `imu-baro-1v8` stays the only declaration.
- **F1, built** (`e1d6c1d5`): `generate` on a declared board writes no `kicad/` and says why, in one field. `generate::tree` skips the emitter when `board.drawn` is `false`, so the board crate, its firmware and the platform are still written. The report's `kicad` field reads "not written: `board.drawn: false` declares a twin stand-in no board is drawn from", and is absent for a drawn board.
  - One test, `generate_tier1.rs` `generate_writes_no_kicad_for_a_twin_stand_in_and_says_why`, on `imu-baro-1v8` itself, which `generate` refused `UnknownPart` before: it now writes `board/` only.
  - **One consequence, built with it, flagged here:** `verify` builds its tree through the same `generate::tree` and indexes the KiCad files by key (`verify.rs` `hashes[&sch]`). On a declared board it would now panic, where it used to refuse at its `extract` gate. So `verify` refuses a declared board in every mode, by the same `DrawsNoBoard`, before any gate. It is the same kind it reached before, without the wasted ERC and DRC. The emit test gains a plain `verify` row.
  - F2's "board gates not applicable" report is still the §7.1 row, as ruled, and would replace this refusal.
- **F2:** a §7.1 row, the architect's. Not built.
- **F3, built** (`e1d6c1d5`): the CLI README's exit-2 row names `UnboundedPinCapacitance`, `PartNotDrawable` and `DrawsNoBoard` (and `generate`'s `kicad` field), and, by the 11:20Z note, 1209c's `LoadNamesCountedPart`. Sentences only.
- **Mutants for the new guards:** M11, `generate` draws a declared board, and M12, no `kicad` field, are each killed by the `generate` test; M13, plain `verify` not refused, is killed by the emit test's `verify` row (it panics, exit 101). **13 of 13 killed** in all.
  - The first run of M11 to M13 was void. The harness restores each file with `git checkout`, and the F1 edits were not yet committed, so it put the files back without them and every kill was false. The edits were re-applied, committed, and the mutants re-run; the kills above are that run.
- **`main` merged again** (`4f0f6d1c`, after 2306 #269 and 1209c #276).
  - One conflict, `IrValidationError::path`'s arms: all three kept.
  - 2306's new specification `crates/aethelgard-cli/tests/fixtures/driver-judges-nothing.json` came at 19 and is renumbered to 20 by the script. It is drawn (`extract_tier1` `emit`s it), so it is not declared.
- **The version:** `4240a` takes 20 and merges first, so this branch renumbers to 21 after it lands: the script, the `boards/` sha lines and `BENCH-EXT-BMP581.md` L9 again, in one commit. Then the package dispatch comes after `4241`.
- **Gates on `4f0f6d1c`** (the merged tree, `gates-2.log`): fmt and clippy exit 0. `cargo test --workspace`: 144 suites, 2,406 passed, 1 failed, the package binding (§3).
- **CI on the first push** (run 44722197295, head `d807fee4`): every job but the package's passed. `gates` and tools shard 2 failed only `bench_board_tier1`'s binding, and `bench-package` derived the package with every gate passing, the board `401b9e1e…` unchanged, and only `FAB-NOTES.md` and `MANIFEST.json` differing from the committed package [measured from the job logs].

## 9. Specification 21, after `4240a` (the 14:18Z note)

- **`main` at `14bb1e5b` merged** (`ac24eac1`): `4240a` (specification 20, `mcu.hse`, solved 13), `2305`, `4405`, `3306` and `1318`.
  - Conflicts were in the version prose (`ir/src/lib.rs`, the IR and CLI READMEs) and in hash goldens (`solve_cli`, `order_2203`, four topology fixtures). Each golden was taken from `main`, then moved by the renumber alone.
  - The prose keeps `4240a`'s version 20 and adds this branch's as version 21, "renumbered from 20 by the same rule, merging after `4240a`".
- **The renumber** (`16be2d38`), by `main`'s own `spike/4240a/kit/renumber.py 20 21 13 13` over the merged tree:
  - 29 specifications' version lines, `2305`'s `logger-4600.json` and `2306`'s `driver-judges-nothing.json` included, and 8 test files' pins;
  - the constant by hand; the IR README's heading, example and step 1; the IR's unsupported-version list, which now refuses 20 and 22.
- **Every sha line and digest re-taken,** each moved by the specification's sha256 alone [measured]:
  - the `boards/sentinel-01*/` sha lines (16 lines, nothing else);
  - the `hwdef` and `svd-hal` goldens (9 files);
  - `BENCH-EXT-BMP581.md` L9 (`ecd44d36…6ee4`, specification 21);
  - the CLI README prefix (`23c31aa0…`);
  - `solve_cli`: with `9d6b2309…3eba` and `6b82460e…c011` put back, `main`'s `a431ad59…0292` and `db746468…69c9`;
  - `order_2203`: with `ccc80f11…e8b2` put back, `main`'s three digests; the boot image did not move.
- **The inventory, for the two new specifications:** `logger-4600.json` is `generate`d in `twin_tier2`'s recovery tests, and `driver-judges-nothing.json` is `emit`ted in `extract_tier1`. Both are drawn, so neither is declared.
- **Measured at 21:**
  - **Corpus:** 55 files on `main`'s binary (`14bb1e5b`, its tree) and this branch's (its tree): 28 pass on both, and the other 27 are refused alike; the only change is that 15 old-version files say "not 21" for "not 20" (`corpus-main21.txt`, `corpus-branch21.txt`).
  - **`emit` of all 29 specifications:** 24 boards byte-identical; 4 refusals identical; `imu-baro-1v8` `DrawsNoBoard` for `main`'s `UnknownPart` (`emits21.txt`).
- **The package:** not committed, as ruled. It waits for `4241`'s run and the architect's dispatch.
- **Gates on `16be2d38`** (`gates-3.log`): fmt and clippy exit 0. `cargo test --workspace`: 145 suites, 2,441 passed, 1 failed, `bench_board_tier1`'s package binding. `hwdef_tier1` and `extract_tier1` pass.

## 10. The merge after `4241` (the 15:35Z note)

- **`main` at `8fdc9422` merged** (`55f27fd7`: `4241`, `4406`, `1211` as the record, docs).
  - The package came from `main` with no conflict; this branch never touched it.
  - Two conflicts:
    - `generate::tree`: this branch's drawn-board guard is kept, with `4241`'s new comment.
    - `order_2203`: `main`'s 4241 move is taken, then `main.rs` re-taken at 21. With `ccc80f11…e8b2` put back it is `4241`'s `a95a1bb6…f918` exactly, and the platforms and the boot image did not move [measured].
  - No specification was left at 20 (the script moved 0), and no spec-sha line at 20 remained.
- **Gates on `55f27fd7`** (`gates-4.log`): fmt and clippy exit 0. `cargo test --workspace`: 146 suites, 2,457 passed, 1 failed, the package binding. `4241`'s tests pass at 21.
