# AET-WO-1312 FEEDBACK — bench access: test points, BOOT0 on a connector, a reset switch

Builder: Claude (Opus 5.5), worktree `~/aethelgard-1312`, branch `wo/aet-1312-bench-access`, cut from `main` 36e428b. The work order is read from docs PR #187 (`origin/claude/argon-repo-docs-synthesis-rjrb0d`, 6a5a181). Labels: **[measured]** run here, **[read]** read at source, **[inferred]** reasoned, not run.

## 0. Before building: what was read, and two premise findings

- **P1, BOOT0's strap is a wire, not a resistor [read].** `power-parts.json`'s MCU row straps `BOOT0` to the `VSS` group (RM0433 p. 138, Table 9), and `schematic_support::draw` puts pin 138 on `GND` directly. The work order's "the strap the emitter draws today stays; the pin pulls BOOT0 to its default through the strap" assumes a resistor. Through a wire, a connector cannot drive BOOT0 high (it would short the driver to ground), so §1.3's judgement fails for today's strap.
  - **What I build:** when a connector pin or a test point names `BOOT0`, the emitter draws pin 138 on its own `BOOT0` net with a pull-down resistor to `GND` whose value is judged at source (DS12117). When nothing names `BOOT0`, the wire stays, so no existing board moves. This is a deviation from "the strap stays", taken because it is the only design that satisfies both §1.3 and §2's byte identity. It is not a §7 stop: the strap can be judged at source (below).
  - **DS12117 Rev 10, Table 49, pp. 136–137 [read: the rendered page, `~/.cache/aet-ds/aet-1312/ds12117-rev10-p137.png`, rendered in the tools image with `gs`; the text dump lost the unit]:** BOOT0 `VIL` max `0.19·VDD + 0.1` V; `VIH` min `0.17·VDD + 0.6` V; the `VPP (BOOT0 alternate function)` leakage, `0 < VIN ≤ VDD`, is at most **15 nA** (the `Ilkg` rows share one unit cell, `nA`). Both revision sections (rev Y p. 136–137, rev V) print the same BOOT0 rows [read].
- **P2, the version bump moves the golden board's provenance lines [read].** `boards/sentinel-01/**` (8 files) and `crates/aethelgard-svd-hal/tests/ui/serial-four/board.rs` carry the sha256 of `examples/sentinel-01-minimal.json`'s bytes. Writing `"schema_version": 13` into that file changes that hash, whatever the code does. So §1.1 is measured in two steps: (1) the new code with the version still 12, against `main`'s binary, on the whole corpus; (2) the bump alone, showing that only the version and the hash lines move.
- **P3, absent fields [read].** Every earlier bump made its new keys required (`INV-06`: "nothing substitutes a permissive default"). This work order says "empty or absent is today's board" and "default: false". I follow the work order's words: `test_points` may be absent and `mcu.reset_switch` may be absent, and neither is serialized when empty or false. **Question for the architect (Q1):** should they be required keys instead, as every earlier bump was? Either is a small change; the required form would add two keys to every specification in the corpus.

## 1. Predictions (committed before any measured run)

1. §1.1 step 1 (new code, version 12, no new field set): **zero bytes move** on every example, fixture, scenario and spike specification, through `validate`, `solve`, `generate` and `emit` [inferred: every new draw is gated on a field or a `BOOT0` mention no corpus file has].
2. §1.1 step 2 (the bump): only `schema_version` lines and the specification-hash provenance lines move [inferred from P2].
3. No corpus specification spells a net or a domain `BOOT0` in any case, so adding it to the reserved names refuses nothing that passes today [inferred; to measure].
4. The router on Sentinel-01 with 13 test points (both I2C lines, the four SPI lines, flash CS, console TX and RX, `VDD_3V3`, `GND`, NRST, BOOT0): **routes at 2 and 4 layers**; the outline grows by under 10 % in area; vias rise by fewer than 20; rip-ups rise [inferred: each pad is one more pin on a net that already routes; the 1.0 mm pads sit beside their anchors].
5. `extract` bounds a test pad's copper as it bounds every pad (the pad's box in its shell, Chew §7.1.2), with no pin capacitance: a bare pad has no part. I predict the I2C lines' bounds rise by under 1 pF each [inferred].
6. The BOOT0 pull-down: **10 kΩ** passes both judgements over the MCU's whole supply range (1.62–3.6 V): unplugged, `15 nA × 10 kΩ = 0.15 mV ≤ VIL max 0.408 V` at 1.62 V; driven to VDD, `VDD ≥ VIH min` at every VDD ≥ 0.73 V, at a strap current of at most 0.36 mA [inferred arithmetic on read values].
7. The tier-2 board passes `validate`, `generate`, `emit`, `check erc` and `check drc --spec` with no finding at 2 and 4 layers [inferred]. The likeliest failure: a test pad placed by the MCU's pins blocks an escape, which would show as an unrouted net at 2 layers (a §7 stop).

## 2. §1.1, measured (the stop did not fire)

**Step 1, the new code at version 12 against `main`'s binary [measured].** Both binaries built from 36e428b and from this branch; every specification file in the corpus (39: `examples/` 2, `crates/*/tests/**` 19, `spike/3400/variants` 17 and its one probe) run through `validate`, `solve` and `generate` (whose `kicad/` is `emit`'s), and every written file's sha256 compared (`~/.cache/aet-1312/v-base`, `v-new`):
- **38 of 39 are byte-identical**: every exit code, every stdout line, every emitted file.
- The 39th is the intended change: `spike/3400/probes/t8-boot0-on-swd.json` (BOOT0 on J2 pin 2) was refused `ConnectorNetUnknown` and now passes `validate`, `solve` and `generate`.
- Prediction 1 held. Prediction 3 held: no corpus file spells a net or domain `BOOT0` (only that probe names it, as a connector pin).
- The whole workspace's tests, the golden board's byte-for-byte test included, are in §5.
- **Specifications held inside tests:** the differential covers spec files only. Specifications written inline in Rust tests, and the scenarios (which name their base specification), are covered by the workspace suite (tier 1, green at every commit) and by CI's tier 2, not by the differential.

**Step 2, the bump alone [measured]** (its own commit, 5dc49df, then to 14 at the merge below). The 22 specification files of `examples/` and `crates/*/tests/**` were moved from `"schema_version": 12` to `13`; nothing else in them changed.
- `main`'s binary ran on `main`'s (version-12) files and this branch's binary on the version-13 files: `validate`, `solve` and `generate` on all 21 corpus files outside `spike/`.
- With each file's old sha256 replaced by its new one, **all 210 generated files and every stdout line are equal; 0 moved** (`~/.cache/aet-1312/tools/step2.sh`). Prediction 2 held.
- **The committed goldens moved by the hash alone [measured]:**
  - `boards/sentinel-01/**`: 8 lines in 8 files, each the provenance line `Solved topology of specification sha256`, `3f89077c…aee4` to `eebb4b90…a638`;
  - `crates/aethelgard-svd-hal/tests/ui/serial-four/board.rs` and `ui/motors-pwm/{board.rs,topology.json}`: the same line, and the topology's `specification_sha256`. Re-blessed with `AETHELGARD_BLESS=1`; the topology is equal with the hash substituted.
- **Pinned digests moved, with the old hash put back reproducing the old digests exactly:**
  - `solve_cli`'s two (the reference `d751e1f4…0dcf`, the stress fixture `00be83e1…eef6`);
  - `order_2203`'s three (`b738cc97…912f`, `b27e162f…31cd`, `8e390d00…c21e`); the boot image did not move.
  - Each is recorded beside its constant, as earlier bumps did.
  - `crates/aethelgard-cli/README.md`'s example hash prefix moved too.
  - `verify_2k2_tier2.rs` pins the reference's hash prefix in `FAB-NOTES.md` (`3f89077c` to `eebb4b90`). It runs only in the image, so my local suite missed it and CI's first run on this branch (shard 1) caught it [measured].
- Version tests: `UnsupportedSchemaVersion` now expects 13, a version-12 file is refused by its version (library and CLI, with and without the new fields), and 12 joined the refused list.
- **Renumbered to 14 [measured].** `4220b` merged first (#192, 70d1621) and took specification 13 (`pin_capacitance` on every device). The architect ruled the renumber (inbox, 02:17 UTC). Main was merged with a merge commit, and the version moved to 14 (the 22 files, `"schema_version": 13` to `14`).
  - The bench fixture was rebuilt from the merged example, so it carries `pin_capacitance`.
  - Measured again against main's binary on main's version-13 files, all 21 corpus files outside `spike/`: **224 generated files and every stdout line equal with the hash substituted; 0 moved.**
  - Golden board: 8 provenance lines again. The `svd-hal` UI goldens: the hash line, and the topology equal with the hash substituted.
  - Pinned digests re-taken, with main's put back reproducing main's exactly: `solve_cli` (`2e19ebee…31d3`, `a59e85d7…7af7`) and `order_2203` (`b4c0b9a0…8ec2`, `f0b5a670…d67f`, `ba58ff9e…cdad`).
  - Hash prefixes moved in the CLI README and `verify_2k2_tier2`. A version-13 file is now refused by its version.
  - Tier-2 count 162 in the image, so `TIER2_MIN` is 162 (4220b's test, then this one). The workspace suite is green.
- `spike/3400/**` was not bumped (§3).

**The router on Sentinel-01 with a test point on every net §1.1 lists [measured]** (fixture `crates/aethelgard-cli/tests/fixtures/sentinel-01-bench.json`: V0 plus 12 test points, BOOT0 on J2 pin 2, NRST on J2 pin 3, the reset switch). "The four SPI lines" are read as SCK, MOSI, MISO and the flash's CS, so the list is 12 distinct nets, not the 13 prediction 4 counted. Figures are read from the emitted board by a scratch reader (`~/.cache/aet-1312/stats`), which also re-routes the read board and gets the same copper; V0 was emitted by `main`'s binary.

| Board | Outline (mm) | Area (mm²) | Parts | Track (mm) | Segments | Vias | Rip-ups |
|---|---|---|---|---|---|---|---|
| V0, 2 layers | 44.10 × 48.50 | 2138.8 | 34 | 597.4 | 549 | 101 | 18 |
| Bench, 2 layers | 46.35 × 52.45 | 2431.1 (+13.7 %) | 48 | 761.9 | 679 | 130 (+29) | 10 |
| V0, 4 layers | 44.10 × 48.50 | 2138.8 | 34 | 413.8 | 393 | 104 | 1 |
| Bench, 4 layers | 46.35 × 52.45 | 2431.1 (+13.7 %) | 48 | 567.5 | 496 | 124 (+20) | 1 |

- **It routes at 2 and at 4 layers.** All eight steps pass on both: `validate`, `generate`, `emit`, `check erc`, `check drc --spec`, `check drc --spec --schematic` (parity), `check support` and `extract`. None has a finding.
- Prediction 4 was wrong in three of four figures. The area grew by 13.7 %, not under 10 %. Vias rose by 29 at 2 layers, not under 20. Rip-ups **fell** at 2 layers (18 to 10) instead of rising. I have no measured explanation for the fall; the placement differs, so the search differs.
- **`extract` on the bench board [measured]:** each test pad adds 897 fF (2L, I2C) to the line's `pads` term, so prediction 5 held for the pad. The line's whole bound rises by up to 17.9 pF (I2C_SDA at 2L: 43.3 to 61.3 pF), from the track and vias routed to the pad. Every line stays under its declared bound (I2C 100 pF, SPI 30 pF) at 2 and 4 layers.

## 3. What was built, and the decisions taken inside the work order

**Specification (`aethelgard-ir`).**
- `test_points: [{net, reference}]` (`bench::TestPoint`) and `mcu.reset_switch` (bool), each optional and not written back when empty or false (P3, Q1).
- Step 15a (`bench::validate`), after the connectors' step 15, refuses by name:
  - `TestPointNetUnknown`: the net is not one a connector pin may name. The test point and the connector read one list, `connectors::nameable`, so the two cannot disagree. That list is a declared signal, a domain, `GND`, `SWDIO`, `SWCLK`, `NRST` or `BOOT0`.
  - `TestPointReferenceTaken`, for either of two collisions:
    - another reference of the specification has the same KiCad key;
    - the reference is one the emitter numbers: `U`, `R`, `C` or `SW` and a number, as `ConnectorReferenceReserved` holds for a connector, with `SW` added for the switch.
  - `TestPointNetRepeated`: one net carries two test points.
- Identifiers are checked at step 2, as `test_points[i].net` and `.reference`.
- **`BOOT0`** joins `DEBUG_NETS`, so a connector pin or test point may name it, and step 11 refuses a declared net or domain spelled like it.
  - It joins `names::own_nets` only where it is drawn, that is when a connector pin or test point carries it.
  - The existing guard `the_nets_validate_reserves_are_the_emitters_own` (every listed net is drawn) caught my first version, which listed it always.
- **The switch** is numbered `SW<n>` as the MCU's `U<n>` is: the first `n` whose `SW<n>` the specification does not spell (`bench::switch_reference`). Step 20 refuses another spelling of it (`sw1`, `SW01`) as `EmitterReference`.
- **The BOOT0 judgement** (`bench::judge_boot0_pull_down`), from DS12117 Rev 10, Tables 49 and 50, read at source:
  - **unplugged:** leakage times R must not exceed VIL max at VDD min;
  - **driven:** a driver at VDD − 0.4 V must reach VIH min at VDD min;
  - **drive current:** VDD max / R must not exceed the 4 mA at which Table 50 states VOH.
  - It is judged over the MCU row's 1.62–3.6 V. The pull-down is 10 kΩ. It passes between 900 Ω and 27.1 MΩ.

**Emitter (`aethelgard-eda`).**
- Bench parts are drawn after every other part (`schematic::bench`), so no other reference moves:
  - BOOT0's 10 kΩ pull-down, anchored by pin 138;
  - the switch, `Switch:SW_Push` on `Button_Switch_SMD:SW_SPST_PTS810`, valued `PTS810 SJM 250 SMTR LFS`. C&K's part is listed Active by digikey.sg, as `3400` FEEDBACK T8 read it.
  - each test point, `Connector:TestPoint` on `TestPoint:TestPoint_Pad_D1.0mm`, drawn on its net as a connector pin is.
- The four files are vendored byte for byte from KiCad's GitLab at tag 10.0.7, and each file's git blob SHA matches the API's [measured]. They are pinned in `library.rs`, and the README rows carry the licence (`RUL-11`).
- **Placement:** test points and the switch are not ICs, so they are never an anchor, and they are placed after every other part except the connectors.
- **Two readers taught one shape each:**
  - the placer reads an `fp_circle` courtyard or silkscreen as its box;
  - `route::read` takes a circular courtyard as the square that holds it. A courtyard only keeps vias out, so the rule is kept wider, never weakened. Arcs are still refused.
- **BOM and parity.** KiCad's `TestPoint` symbol is `in_bom yes`, and its pad footprint is `exclude_from_bom`. Parity found 12 `footprint_symbol_mismatch` warnings on the first tier-2 run [measured]. Each test point's symbol instance is now written `in_bom no`, and test points are left out of `emit`'s BOM, because a bare pad is bought and placed by no one.
- **`extract`:** a test point is `Load::TestPad`. Its pad's copper is bounded as every pad's is (Chew §7.1.2, the existing trial), and it carries no `Ci`, because no part is mounted on it. A probe clipped to it is outside the bound, as any off-board load is. **Answer to §1.2:** the bound is declared, and the net is not refused. `extract`'s footprint reader admits `fp_circle` off copper, as it admits `fp_arc`.
- **Support gate:**
  - When BOOT0 is carried, its strap is judged as BOOT0 on a net of its own, shared with no other MCU pin, with exactly one populated resistor to the strap's ground net, whose value passes the DS12117 judgement. The record line names the resistor and its value.
  - A connector pin on `BOOT0` is judged against the boot pin's net, as NRST is.
  - New findings: `test_point_off_net` (each test point's pad on its declared net) and `reset_switch_wrong` (a switch from the MCU's NRST to `GND` exactly when declared).

**`RUL-16`.** `test_points` is private, read through `Specification::declared_test_points()`, as `connectors` is. The three runners' `clippy.toml` disallow it, and `rul16_fence.rs` pins the four accessors. Nothing solves a test point, and no runner reads one.

**Files outside §3's fence, and why.**
- `eda/src/support.rs`: the support gate must judge BOOT0's new strap and the switch, or the bench board fails `check support`.
- `.github/workflows/ci.yml` and `ci/test-weights.tsv`: `TIER2_MIN` is exact and the new tier-2 test needs a weight.
- `crates/aethelgard-cli/README.md` and the CLI's `clippy.toml`: the README's example hash moved with the bump, and the `clippy.toml` carries the `RUL-16` fence.
- `crates/aethelgard-svd-hal/` and `crates/aethelgard-twin/` (`clippy.toml`, goldens, the `order_2203` digests): the bump's hash-only moves, and the fence.
- `boards/sentinel-01/**`: the bump's provenance lines.

**Not measured.** `verify` and `verify --fab-out` were not run on the bench board, so its fabrication package is untested. `3401`'s ordered board will take that path first.

**Not changed, and why.**
- `spike/3400/**` stays at version 12. It is the 3400 spike's record, nothing builds or tests it, and builds delete `spike/` (roadmap). The bump makes its specifications refused by version.
- `aethelgard-cli`'s `extract` conditions are not changed (the CLI is `2302`'s). See Q2.

## 4. Adversarial pass (§5), by a fresh agent at 5dc49df

**No escape was reachable from a specification [agent, measured by it].** Every finding needed a hand-edited netlist, schematic value or board. I fixed each one first and then wrote its tier-1 test. I did not re-run the agent's reproducers against the unfixed code. The proof that each test bites is the mutant of each fix (mutants 20–25 in §5), which every one of them kills.

- **F1, medium, new.** The BOOT0 strap check never looked at anything else on BOOT0's net. All of these passed the support gate with no finding:
  - a 1 kΩ, 100 Ω or 0 Ω resistor from `/BOOT0` to the supply beside the 10 k pull-down;
  - a jumper on BOOT0;
  - a device pin (U4's WP) moved onto BOOT0.

  Unplugged, such a board boots from system memory. **Fixed:** BOOT0's net may hold only the boot pin, its one pull-down, and the connector pins and test points the specification puts on BOOT0. Anything else is `strap_wrong`, naming the part and pin. Test: `anything_else_on_boot0_s_net_fails`.
- **F3, medium, new.** `judge_boot0_pull_down` multiplied leakage by resistance unchecked.
  - In a release build the product wrapped: a pull-down valued `1229782938247303442` (about 1.2 EΩ, an open) passed.
  - In a debug build it panicked, a gate panicking instead of refusing.

  **Fixed:** checked multiply, and a product past `u64` is refused on VIL. Tests: `bench.rs` unit and `an_unpopulated_or_overflowing_pull_down_fails`.
- **F4, low.** A test point was judged by pin 1's net alone. All of these passed:
  - declared TP9 exported as a 0 Ω `Device:R` with pin 2 on GND (a supply short);
  - an undeclared test point;
  - a switch with a third pin on a net.

  **Fixed:** a declared test point must be a one-pin `Connector:TestPoint`, every `Connector:TestPoint` in the netlist must be declared, and the switch may have nets on pins 1 and 2 only. Test: `a_test_point_or_switch_of_another_shape_fails`.
- **F5, low, robustness.** `route::read` subtracted circle coordinates in `i64`, so a hand-edited courtyard circle with far coordinates panicked in debug. **Fixed:** a circle beyond a metre is refused before any arithmetic, and the subtraction is widened in the placer too.
- **Information, acted on.** Test point references were checked as `Plain` identifiers, a connector's as `Reference`. `TP?`, `~` and `TP-1` reached the emitted files. They are now `Reference` identifiers, and those are refused (test).
- **Information, recorded (agent's measurements, not re-run here).**
  - A board that carries BOOT0 places differently: the anchored pull-down moved 7 parts on `four-layer-power` and 10 on `two-bme`. Test points alone and the switch alone moved none.
  - Library paths that skip `validate` (`extract::lines`, `anchored`) accept a test point on an emitter net. `emit` refuses it (`TestPointNetUnknown`), and connectors behave the same way, so nothing is emitted.
- **What held, as the agent measured it (not re-run here).**
  - Every spelling of a reserved or emitter net is refused by `validate` and by `emit`.
  - Reference folding (`tp12`, `j02`, `SW01`) and the switch numbering hold.
  - `[]` and `false` emit the same bytes as absent, schematic, board and BOM.
  - No `pintype` and no `board_only` on the test pad.
  - An `fp_circle` on copper is refused.
  - The `ohms` parser holds.
- Reproducers: `~/.cache/aet-1312/adv-scratch/adv_1312.rs` (9 tests).

## 5. Reduction pass, mutation, gates

- **Reduction pass (diff): +7 net (measured: the shared helper and its doc outweigh the copies removed) · 2 duplicates removed · 0 dead paths removed · 0 protected kept · 1 finding below.**
  - The declared-net-to-connection rule written twice in `schematic.rs` is one `Builder::declared`, used by connectors and test points.
  - The debug-net map written twice in `support.rs` is one: test points use step 5's `expected`.
  - **Finding:** `pcb::graphic_box`'s refusal arm is now unreachable, because `geometry` passes it only the five shapes it measures. I kept it as a refusal rather than an `unreachable!`, and it is not labelled `belt-and-suspenders`. The owner decides.
- **Mutation (diff-sized), 26 mutants, one per guard this work order adds, run against `ir` `bench` and lib, `cli` `bench_tier1` and `eda` lib: 26 killed.** All 26 were re-run on the final code, after the adversarial fixes and the `RUL-16` change. Two patterns were updated to the rewritten checks, mutants 13 and 14 (`~/.cache/aet-1312/mut/summary.txt`, `mutants.py`).
  - The first run left two survivors. One was the BOOT0 wired-to-GND check, which the shared-pin check subsumes by kind; the test now asserts its message. The other was the unpopulated pull-down, which had no test.
- **Main merged again at 7452bf4** (`AET-WO-1801`, the 0.2.0 bump) **and then at 2b43672** (`4220b`'s floor, parts schema 7), as the architect asked. Both merges were clean, with no `Cargo.lock` conflict. No file this work order adds or re-blessed embeds the engine version (grep for `0.1.0`: the one hit is DS-009's own revision in the IR README, already on main). The three gates were re-run after each merge: fmt, clippy (with `with-tools` too), and the workspace suite, 121 test binaries green.
- **Gates:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` (with `--features with-tools` too) and `cargo test --workspace` pass. The tier-2 bench test passes in the pinned image (8.5 s on Furnace). `TIER2_MIN` is 161, counted in the image with `shard.sh parse`. The new test's weight is a Furnace estimate (× 2.8, `AET-WO-1310`'s factor) until a refit.

## 6. Questions for the architect

- **Q1.** `test_points` and `mcu.reset_switch` are optional, as the work order words them. Every earlier bump made its keys required (`INV-06`). Keep them optional, or make them required? Required would add two keys to every specification in the corpus.
- **Q2.** `extract`'s verdict states its conditions. A probe clipped to a test pad is outside the bound, like any off-board load. A condition line saying so when a line carries a test pad belongs in `aethelgard-cli/src/extract.rs`, which is `2302`'s fence, so I did not add it. Who adds it?
- **Q3.** `spike/3400/**` stays at version 12. Its specifications are now refused by version. Delete it here, leave it for `3401`, or bump it?
- **Q4 (P1).** The strap stays a wire on boards that do not carry BOOT0. Should every board get the judged pull-down instead, so BOOT0 is one design everywhere? That would move every existing board's bytes, which the work order's §1.1 forbids here.
- **Q5 (`RUL-03`).** Two readers now accept a circle:
  - `route::read` used to refuse a circular courtyard; it now takes the circle as the square that holds it, a via keep-out kept wider and never weaker;
  - `extract`'s footprint reader admits `fp_circle` off copper, as `4300d2` admitted `fp_arc`.

  Neither is in `RUL-03`'s text, which rules the strict reader's refusals in general. Is the square acceptable, or should the test pad's courtyard be refused and a pad with a rectangular courtyard chosen instead?
- **Order with `4220b`:** done. It merged first, and this work order is specification 14 (§2).

## 7. Status

**READY.** CI is green on e34f0f5, the head before this commit, which is main at 2b43672 merged in: 22 of 22 jobs pass, and `tier2_tests_passed=162 floor=162`.
- This commit adds only this section. Nothing is pushed after it unless the architect asks.
- The open items for the architect are §6: Q1 (optional or required fields), Q2 (the `extract` condition, in the CLI's fence), Q3 (`spike/3400` at version 12), Q4 (the BOOT0 pull-down on every board) and Q5 (`RUL-03` and the circular courtyard), plus P1's deviation.
- The PR title and body are in `~/.cache/aet-1312/pr-title-body.md`.
- Scratch trees are removed. Kept, being small and cited above: `~/.cache/aet-1312/{tools,mut,adv-scratch,stats,v-base,v-new}` and `~/.cache/aet-ds/aet-1312/`.
