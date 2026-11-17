# AET-WO-3402 — FEEDBACK

Builder: Claude (strong tier, the `3400`/`3401` builder), worktree `~/aethelgard-3402`, branch `wo/aet-3402-preorder`, cut from `origin/main` `ac5f8c8`. The work order is read from `origin/claude/argon-repo-docs-synthesis-rjrb0d` (docs PR #201) and is not committed here.

Labels: **[measured]**, **[read at source]**, **[inferred]**.

## 0. Status: STOPPED at §1.3, as the work order's cadence asks

**Superseded by the ruling (PR #203, comment 6944745635): the current state is §8–§9.** §1.3 is built as ruled, one GND pad, and the package is awaiting the architect's `bench-package` write. Sections 0–7 below are the STOP record as signalled.

- **§1.1** (GND exempt from `TestPointNetRepeated`) and **§1.2** (the ledger rows in `FAB-NOTES.md`) are built and gated (§2, §3).
- **§1.3** (each I2C peripheral at its own bus's speed) was prototyped, measured and **not committed**. The prototype is kept as a patch for after the ruling, at `~/.cache/aet-3402/s13-prototype.patch`.
- **§4 holds the three answers the work order asks for:**
  1. **No solved-schema field is needed.**
  2. **Only Sentinel-01 as ordered moves:** three files of its board crate and its package. The reference, every other golden and every other fixture are unchanged.
  3. **I2C1 goes from `0x50D32A40` (100 kHz) to `0x10C7123C` (400 kHz); I2C2 stays `0x50D32A40`.**
- **§1.4 is measured, not built** (§5). A second GND pad routes and passes every gate at two layers, **but the specification cannot place it near TP11–TP14:** the placer puts it in the same spot whatever its list position or reference.

## 1. Predictions

**Out of order, stated:** I measured §1.1–§1.4 before writing predictions, which the kickoff's cadence asks for first. Nothing below is scored as a prediction. The expectations the work order itself states are scored instead: §1.3 "expected: only boards with two I2C buses at different declared speeds" held (§4).

## 2. §1.1: `TestPointNetRepeated` exempts `GND` [built]

- `ir/src/bench.rs`: the repeated-net check skips `crate::GND`, so any number of test points may name `GND`, each with its own reference. Every other net still carries one.
- **Tier 1** (`ir/tests/bench.rs`):
  - `the_same_net_twice_is_refused_by_name` now repeats `I2C_SDA` (refused, naming `TP1` and `TP12`);
  - new: `gnd_carries_any_number_of_test_points` (a second `GND` pad validates; a `GND` pad reusing `TP10`'s reference is still refused, `TestPointReferenceTaken`).
- The error's text, the `bench.rs` doc and the ir README's step 15a say "any net but `GND`".
- No specification version: this accepts what was refused and changes no shape.
- **Every consumer of test points reads them by reference, none by net** (`eda/src/support.rs` 5a, `schematic.rs`, `names.rs`) [read at source]. So two GND pads need nothing downstream, and the second-GND variant passes every gate (§5).
- **1313** is told, in its inbox, of the one line in `ir/src/lib.rs` (the error text).

## 3. §1.2: the ledger rows in `FAB-NOTES.md` [built]

- `eda/src/fab.rs`: `Notes` gains `acknowledged`, and `acknowledged_section` writes `## Acknowledged ledger rows` between the lifecycle section and `## Files`.
  - One line per entry of the specification's `acknowledge_unwitnessed`, in its order: `` - `REGISTER` (class): `F1`, `F2`. reason ``, each part exactly as written.
  - A board with none writes "- None: the specification acknowledges no unwitnessed ledger row."
- `cli/src/fab.rs` passes `&spec.acknowledge_unwitnessed`. That is one line in 1604's fence, told to 1604 in its inbox.
- **Tests:**
  - an `eda` unit test pins the line format and the "None" line;
  - `verify_2k2_tier2.rs` (tier 2, the reference's real `--fab-out` package) now checks that the section's lines equal, in order, the lines the reference specification's entries render to. That is §5's "a ledger row that differs from the specification's entry".
  - Its file-list check is scoped to the `## Files` section, because the ledger bullets also start with a backtick.
- **Moved goldens:** the three `crates/aethelgard-cli/tests/fixtures/fab/notes-before-4300d2-*.md` gain the section their own specification renders to:
  - the reference: 27 rows;
  - `four-layer`: 22;
  - `four-layer-power`: none, so its "None" line.

  `a_via_only_boards_notes_are_byte_identical_to_before_4300d2` passes against them. No other golden moves.
- **Every package's `FAB-NOTES.md` and its `MANIFEST.json` hash move.** The committed `fab/sentinel-01-bench/package/` is therefore stale against today's `verify` until it is rewritten (`bench-package`, after the §1.3 ruling, together with §1.4). The per-PR binding test still passes, because it binds the package to its own manifest and inputs. `main`'s `bench-package --against-fab` would name `FAB-NOTES.md` and `MANIFEST.json`, so this PR cannot merge before the package is rewritten. That rewrite is planned (§6).

## 4. §1.3 measured: each I2C peripheral at its own bus's declared speed (STOP)

**The prototype** (`~/.cache/aet-3402/s13-prototype.patch`, 101 lines, `svd-hal/src/board.rs` only):
- `bus_speed(solved, peripheral)` joins each I2C peripheral to its bus from the solved topology alone (`RUL-16`):
  - `analog_lines` names each I2C line's bus;
  - `assignments` names the peripheral of each line's net.
- It refuses, by a new `BoardError::I2cBusUnresolved` (`INV-06`):
  - a peripheral with no bus;
  - a peripheral carrying lines of two buses;
  - a bus with a line on another peripheral.
- The lowest-speed fallback is removed.

**(1) Does the derivation need a solved-schema field? No** [read at source; measured].
- `SolvedTopology.analog_lines` (net → bus) is filled by `solve` for every line of every I2C bus that has a net constraint and a domain (`ir/src/solve.rs` `analog_lines`). `validate` refuses a bus without its constraints (`MissingBusConstraint`), so every solved bus is there, a deviceless one included. On Sentinel-01: `EXT_SCL`/`EXT_SDA` → EXT and `I2C_SCL`/`I2C_SDA` → SENSORS.
- `assignments` puts those nets on `I2C2_*` and `I2C1_*`.
- A hand-written topology lacking an analog line is refused by name, never given a speed.
- **The alternative,** a field such as `i2c_peripherals: {bus: peripheral}` (solved schema 10), would state the join once for the generator and the judge alike. It is not needed for the derivation. It is the architect's call under `RUL-20` (point (4) below).

**(2) Every committed board and golden whose bytes move** [measured: the workspace's tests with the prototype, then `generate` diffed against the committed tree]:

| What | Moves? | How |
|---|---|---|
| `boards/sentinel-01-bench/src/board.rs` | **yes** | `i2c1_speed`'s doc reads 400000 Hz; `impl AtMostCeiling for Fast400k { const TIMINGR: u32 = 0x10c7123c; }` added |
| `boards/sentinel-01-bench/firmware/src/main.rs` | **yes** | `i2c1.init::<i2c1_speed::Standard100k, _>` → `Fast400k` |
| `boards/sentinel-01-bench/src/proofs.rs` | **yes** | the oracle's `i2c1_timingr = 0x10c7123c`, a `Fast400k` start and its Kani covers, replacing a `kani::assume(false)` arm |
| `fab/sentinel-01-bench/package/` | **yes** | the ELF, `FAB-NOTES.md` (§1.2) and `MANIFEST.json`; the Gerbers, drill and placement do not move, because the board's copper does not |
| `boards/sentinel-01` (reference), `svd-hal/tests/ui/{motors-pwm,serial-four}/board.rs` | no | one I2C bus each |
| every other example, fixture and scenario | no | the only specification with two I2C buses at different declared speeds is `examples/sentinel-01.json`; the four-bus fixtures (`d1-24-i2c`, `d3-24-i2c-12-spi`, `solve/{infeasible-shared,stress,thrash-pins}`) are all at 400 kHz |
| tests that pin the old rule | **change** | `svd-hal` `board::tests::ceilings_must_number_the_i2c_peripherals_and_the_lowest_applies` (by name); `svd-hal/tests/silicon_words.rs`'s `every_fixture_s_controllers_are_written_the_rule_s_words` and `every_fixture_s_schedule_starts_each_i2c_bus_at_the_rule_s_mode`, which state the expected mode as `t.i2c_speeds.values().min()` |

With the prototype, the workspace ran 2055 passed and 5 failed. The five are the three tests above, the bench board's comparison and §1.2's notes golden, since fixed. Nothing else failed.

**(3) Each `TIMINGR`, before and after** (64 MHz HSI kernel, the solved `i2c_timings`; Sentinel-01 as ordered):

| Peripheral (bus, declared) | Admitted modes, before → after | Written at boot, before → after |
|---|---|---|
| I2C1 (SENSORS, 400 kHz) | `Standard100k` → `Standard100k`, `Fast400k` | **`0x50D32A40` (100 kHz) → `0x10C7123C` (400 kHz)** |
| I2C2 (EXT, 100 kHz) | `Standard100k` → `Standard100k` | `0x50D32A40` → `0x50D32A40` |

No bus runs faster than its declared speed (§5 adversarial). `silicon::check_timingr` holds both words, as `silicon_words.rs` asserts for every solved mode.

**(4) What the ruling also decides, `RUL-20`.**
- `silicon_words.rs` is the independent judge of the words and start modes. It now expects the lowest speed, so it must expect each peripheral's own bus's speed.
- Its join of bus to peripheral must not call the generator's `bus_speed`, or the judge becomes consistency.
- **Proposed:** the test recomputes the join itself from `analog_lines` and `assignments`, as a dozen lines of its own. With a solved-schema field, both would read one stated fact.

## 5. §1.4 measured: a second GND test pad (not built, waits on §1.3)

**A second `GND` pad (TP15) routes and passes every gate at two layers** [measured]:
- `validate`, `generate`, `emit`, ERC, DRC by the specification's rules, parity, support and `extract` all exit 0, with no finding.
- Outline 49.10 × 54.85 mm.
- `extract`: I2C_SDA 62.8, I2C_SCL 57.9, EXT_SDA 47.2, EXT_SCL 48.9 pF (against 100); SPI_SCK 15.0, SPI_MOSI 20.8 pF (against 30).

§7's second stop does not fire.

**Finding F1: the specification cannot place the second GND pad.**
- Six variants placed it identically: TP15 appended, inserted first, or inserted before the I2C group; and the new pad numbered TP11, TP12 or TP14 among the I2C and EXT pads.
- In every one it lands **25.8–31.5 mm** from TP11–TP14's nets, farther than the existing GND pad, TP1, at 20.5–31.0 mm. The outline and every bound were identical too.
- So §1.4's "near TP11–TP14" cannot be met from the specification. The placer puts `GND` test pads by its own rule.
- `BENCH.md` B6's ground spring needs a pad within a few millimetres of the probed pad, so as placed the second pad adds nothing for B6.
- **Options (Q2):**
  - (a) a placer rule that puts each `GND` test pad beside the test point listed before it (`aethelgard-eda`'s placer, outside this fence);
  - (b) a field naming a test point's neighbour (a specification change);
  - (c) order with one GND pad, and B6 grounds its probe at the nearest ground [measured on the emitted board]:
    - for EXT, J4 pin 4, 8.8 mm from EXT_SDA's pad and 15.2 mm from EXT_SCL's;
    - for SENSORS, TP1, 20.5–21.2 mm from I2C_SCL/I2C_SDA's pads (J1, J3 and J4's GND pins are 35.8–52.6 mm away).
    - None of these is within a short ground spring's reach [inferred: a spring is a few millimetres], so B6 would use a ground lead and state its length.

## 6. Gates [measured, on the committed tree: §1.1, §1.2 and the goldens; the prototype not applied]

- `cargo fmt --all -- --check` 0;
- `cargo clippy --workspace --all-targets -- -D warnings` 0;
- `cargo test --workspace` 0, with 2060 passed and 0 failed.
- In the pinned image: `cargo clippy --workspace --all-targets --features aethelgard-cli/with-tools -- -D warnings` 0. `verify_2k2_tier2` passes 2 of 2 in 1284 s: the reference's real `--fab-out` package carries the ledger rows, equal line for line to its specification's entries, and its file list matches.
- `TIER2_MIN` is unchanged: no tier-2 test is added, and `verify_2k2_tier2.rs` gains assertions, not tests.

**After the ruling:**
1. apply §1.3 as ruled, with the judge's change;
2. build §1.4 (the second GND pad, placed as ruled);
3. re-bless `boards/sentinel-01-bench`;
4. rewrite the package through `bench-package` (the temporary-commit round trip);
5. update `ORDER.md` (the test-point count) and `BENCH.md` (B2's per-peripheral `TIMINGR`, B6's ground);
6. run the adversarial cases of §5.

## 7. Questions for the ruling

- **Q1 (§1.3).** Build the prototype as measured, with no solved-schema field and the judge in `silicon_words.rs` given its own join? Or add a solved field (schema 10) that both read?
- **Q2 (§1.4, F1).** The second GND pad cannot be placed near TP11–TP14 from the specification. (a) a placer rule, (b) a specification field, or (c) one GND pad and J4 pin 4 for B6's ground?
- **Q3.** `cli/src/fab.rs`'s one line (§3) is in 1604's fence. Accept it here, or should 1604 carry it?

## 8. After the ruling (PR #203, comment 6944745635): predictions, committed before any run

**The ruling:**
- **Q1:** build §1.3 as prototyped, with no schema field; `silicon_words.rs` gets its own join (never `bus_speed`, `RUL-20`); restate the lowest-speed unit test; run §5's adversarial cases.
- **Q2 → (c):** one GND pad. `BENCH.md` B6 grounds at J4 pin 4 (EXT) and TP1 (SENSORS), with distances, the lead and one [inferred] line on edge rates. The second pad is dropped from §1.4; §1.1's exemption stays. The placer rule goes to the backlog (1313b).
- **Q3:** the `cli/src/fab.rs` line stays here.

**Predictions:**
- **R1. The merge of `26e4c62`** (4400b2) conflicts at most in `svd-hal/src/board.rs`'s tests, where one test moved to `ir`. My patch touches `generate_with`, the error enum and a new `bus_speed`, not that test (70 %, no conflict).
- **R2. After §1.3 and the re-bless,** exactly three files of `boards/sentinel-01-bench/` move (`src/board.rs`, `src/proofs.rs`, `firmware/src/main.rs`), as §4 measured. Nothing else committed moves: the reference board and the `svd-hal` ui goldens are byte-identical.
- **R3. The tests:**
  - with the judge's own join, `silicon_words.rs`'s two tests pass on every fixture, Sentinel-01 as ordered included;
  - the restated unit test passes, with I2C1 at `Fast400k`/`FastPlus1M` admitted for a 1 MHz bus and I2C2 at `Standard100k` only;
  - every other workspace test passes, except the package binding test (`bench_board_tier1`). Its committed package's specification hash still matches, since the example is unchanged, so **it passes too** (60 %). The per-PR test binds specification, topology, scenarios, its own manifest and `emit`'s BOM, none of which §1.3 moves.
- **R4. The adversarial cases** (§5), each a tier-1 unit test in `svd-hal`:
  - a peripheral carrying two buses' lines → `I2cBusUnresolved`;
  - a bus with one line on another peripheral → `I2cBusUnresolved`;
  - an I2C peripheral with no analog line → `I2cBusUnresolved`, or `InvalidTopology` from `validate` first, whichever comes first;
  - no admitted mode is ever faster than its own bus's declared speed, checked over every fixture by `silicon_words.rs`'s judge.
- **R5. `TIER2_MIN` stays 177:** no tier-2 test is added.

## 9. The ruled build [measured]

**Built:**
- **§1.3:** the prototype as measured (`svd-hal/src/board.rs`: `bus_speed`, `BoardError::I2cBusUnresolved`; the lowest-speed fallback removed).
- **The unit test is restated** as `ceilings_must_number_the_i2c_peripherals_and_each_takes_its_own_bus`: a 1 MHz bus A on I2C1 admits all three modes and boots `FastPlus1M`; a 100 kHz bus B on I2C2 admits `Standard100k` only.
- **The judge** (`tests/silicon_words.rs`) gets its own `own_bus_speed`, a join from `analog_lines` and `assignments` written in the test. It never calls `bus_speed` (`RUL-20`).
- **`boards/sentinel-01-bench` is re-blessed.** Exactly the three files of §4 move, with the `TIMINGR` of §4.
- **Docs:**
  - the svd-hal README's I2C paragraph states the new rule and lists `I2cBusUnresolved`;
  - `BENCH.md` B2 states each peripheral's word, and B6 grounds at TP1 and J4 pin 4 with the measured distances, a lead and the [inferred] edge-rate line;
  - the "one GND pad" note and the open-items row explain why there is one.
- **`ORDER.md` is unchanged:** 14 test points, as ruled (one GND pad).

**Predictions scored:**
- **R1 held:** the merge of `26e4c62` had no conflict, and the patch applied cleanly.
- **R2 held:** exactly the three bench-board files moved; the reference board and the ui goldens did not.
- **R3 missed once.** The judge's first run failed on `sentinel-01.json I2C2 Fast400k`.
  - The cause was the judge, not the generator. It searched the whole board file for each mode's `const TIMINGR` line, and I2C1 now admits `Fast400k` with the same word as I2C2's, so the search found I2C2's "admitted" mode in I2C1's module.
  - The judge now reads each peripheral's own `{m}_speed` module, and checks that each mode is admitted with its solved word, or not admitted at all.
  - The restated unit test and `bench_board_tier1` passed as predicted (the package binding test, 2 of 2: nothing it binds moved).
- **R4:**
  - A topology whose peripherals each carry one line of each bus is refused by `SolvedTopology::validate` first (`InvalidTopology`, "does not share one bus"), so `bus_speed` is a second line behind it. The test asserts both: `generate`'s refusal, then `bus_speed`'s own.
  - Covered by name (`I2cBusUnresolved`): two buses on one peripheral, a bus with a line on another peripheral, a peripheral with no I2C line, and a bus with no declared speed.
  - The judge checks "no admitted mode above its own bus's declared speed" on every fixture.
- **R5 held:** `TIER2_MIN` stays 177.

**§5 adversarial, as the work order lists it:**
- A second test point on a non-GND net is still refused: `the_same_net_twice_is_refused_by_name`.
- Two buses sharing a peripheral, or no derivable peripheral: refused by name, as in R4.
- No `TIMINGR` runs a bus faster than its declared speed: `silicon_words.rs` on every fixture, per peripheral.
- A FAB-NOTES row differing from the specification's entry: `verify_2k2_tier2` (§3).

**Mutation** (one mutant per guard this WO adds, against its own suite; all killed):

| Mutant | Killed by |
|---|---|
| M1: the GND exemption removed | `ir/tests/bench.rs` |
| M2: no line → 100 kHz | `an_i2c_peripheral_with_no_one_bus_is_refused_by_name` |
| M3: two buses → the first | the same |
| M4: a foreign line ignored | the same |
| M5: no declared speed → 1 MHz | the same |
| M6: the generator back on the lowest speed | `silicon_words.rs` (the independent judge) |
| M7: the ledger row's reason dropped | `eda` `fab::` tests |

**Gates:**
- after merging `26e4c62`: `cargo fmt --all -- --check` 0; `cargo clippy --workspace --all-targets -- -D warnings` 0; `cargo test --workspace` 2073 passed, 0 failed.
- **In the pinned image** (`capped-run`, the same commands as CI's `bench-package` job):
  - `cargo clippy --workspace --all-targets --features aethelgard-cli/with-tools -- -D warnings` 0; the CLI and the bridge build.
  - `verify --fab-out` on `examples/sentinel-01.json` with the job's five scenarios **passes** (`fab.status` written, every precondition met, the harnesses proved with the bench board's new `Fast400k` covers).
  - `verify --against-fab fab/sentinel-01-bench/package` **fails, as it must until the artifact is committed.** Exactly three files differ: `sentinel-01-bench.elf` (I2C1 boots at 400 kHz), `FAB-NOTES.md` (§1.2) and `MANIFEST.json`. The Gerbers, drill, placement and BOM are byte-identical, since the copper does not move.
- The package committed is CI's, written in write mode after the architect's dispatch, not this local one: the MANIFEST verdict includes the sandbox line.

**Reduction pass** (diff-sized): nothing removed. Each addition is a guard with a killed mutant, its test, or the prose that states it. The prototype's lowest-speed doc lines and the README's old rule were replaced, not kept beside the new ones.

**Distances.** The ruling quoted 8.8/15.2 mm for J4 pin 4, which came from the second-GND variant in §5. `BENCH.md` B6 states **8.5/14.0 mm**, measured on the board as committed (one GND pad, outline 46.35 × 54.85 mm). TP1's 20.5/21.2 mm is the same on both.

**B7 is unchanged:** it measures I2C2 (EXT), whose word stays `0x50D32A40`. Its step 2 now says so.

**Other readers of `i2c_speeds`** [read at source]:
- `cli/src/analog.rs` and `propose.rs` already read each bus's own speed.
- `hwdef`'s minimum is ArduPilot's one global clock (`I2CDevice.cpp`), a different rule, and correct as it is.
- Nothing else picked a speed for a peripheral.

**Next, as ruled:**
1. This push is `AWAITING-PACKAGE`.
2. The architect dispatches `bench-package` in write mode on this branch.
3. I commit its artifact unchanged under `fab/sentinel-01-bench/package/` and re-check the binding.
4. READY.

**Backlog** (the architect's): placer rule (a), putting a GND test pad beside a named test point → 1313b.

## 10. Merges in the queue (1313, then 1604) [measured]

**`f845b6f` (1313, specification 15):**
- The merge had no textual conflict.
- `AETHELGARD_BLESS=1` on `svd-hal --test board` rewrote nothing, so the auto-merged bench board is the generator's output at specification 15 with the per-peripheral speeds.
- Workspace 2109 passed, 0 failed.

**`2642ece` (1604):**
- One conflict, `cli/tests/verify_2k2_tier2.rs`, resolved by keeping both sides: 3402's ledger-row check, and the `## Files` reading both WOs share, its comment naming both sections. `cli/src/fab.rs` merged cleanly, with `acknowledged:` kept beside 1604's `jlcpcb_notes`.
- The first bless run reported 1 failure in 7. A rerun passed 7 of 7, and neither run changed a file. The failure did not recur, so I record it here rather than call it understood.
- Gates: fmt 0, clippy 0, workspace **2138 passed, 0 failed**. `TIER2_MIN` is main's 178; 3402 adds no tier-2 test.

**The package on the branch is now main's,** CI's write for 1604 (run 44079386736). It lacks 3402's ELF and FAB-NOTES rows, so `bench-package --against-fab` differs until the architect's dispatch writes it again.

## 11. The package, as CI wrote it [measured]

- **Source:** the architect dispatched `bench-package` in write mode on `9b2a4c27`, **run 44091740015** (concluded success). Its **artifact 14335020719** ("bench-package", 17 files) was downloaded with the read-only token and copied into `fab/sentinel-01-bench/package/` **unchanged**: `cmp` holds for every file.
- **What moved against main's package** (1604's write, run 44079386736): exactly three files, as predicted in §4 and measured locally in §9:
  - `sentinel-01-bench.elf`, I2C1 boots `Fast400k`;
  - `FAB-NOTES.md`, which gains `## Acknowledged ledger rows`;
  - `MANIFEST.json`.
  The file set is the same 17, and the Gerbers, drill, placement, BOMs and JLCPCB files are byte-identical.
- **Binding:** `bench_board_tier1` passes, 2 of 2, on the committed package. No document quotes the old package hashes.
- **Known follow-up, for whichever merges second:** 4400c2 (#204, the bench `.repl`) and 4400d (specification 16) each move this package again.
