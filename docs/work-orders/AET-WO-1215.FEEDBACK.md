# AET-WO-1215 FEEDBACK — an older specification is refused with every change it lacks

**Builder:** Claude (Opus 5.5), 2026-11-22. **Base:** main `ddfa64d6`, then `7c1f369f` and `c1e444dd` merged (specification 23, solved 15). **Branch:** `wo/aet-1215-version-changes`, PR #303.
**State:** BUILT to the ruling. It first STOPPED at §1.1 (§7: the README prose and the code disagreed about versions 14, 16 and 20; C1–C3 in §3). The architect ruled every lean of D1–D8 (bus, 2026-11-22T03:05Z), with one question still open on D6 (§6).
**Gates:** §8.

## 1. What was measured, and how (`spike/1215/`)

- **`measure.py` → `out/witnesses.txt`:** three mechanical witnesses per bump, read from git. A bump is one of the 22 commits that set `SPECIFICATION_SCHEMA_VERSION` to N, 2 to 23, with exactly one commit per N.
  - The JSON key paths gained and lost by the committed specifications at N, against the parent at N−1.
  - The `Deserialize` struct fields of `aethelgard-ir` added and removed by the bump. The `?` optional marker is a regex that also fires on "default" in doc comments. Optionality was read from the code instead.
  - The refusal kinds (`=> "Kind"`) added in the window since the previous bump, and among them the ones a commit other than the bump landed.
- **Three readers, one per range** (2–8, 9–15, 16–23; `measure/measure-*.md`). Each read the README prose and each bump commit's own diff. They are a reader's claims, with `commit:file:line` cites. What I verified myself is marked **(verified)** below.
- **`entrypoints.sh`:** today's bytes for versions 0, 1, 20 and 24 (`in/`) through every specification-reading entry point: `validate --file`, `validate` on stdin, `solve`, `emit`, `generate`, `extract`, `check drc --spec`, `check support`, `check speeds` and `verify`.
  - Output: `out/entrypoints-before.txt`. All ten print identical bytes, for example `{"ok":false,"error":{"kind":"UnsupportedSchemaVersion","id":null,"message":"schema_version is 20, not 23"}}`, exit 2. **(verified)**
  - Version 1: `"message":"schema_version is 1, not 23"`. Version 24: `"schema_version is 24, not 23"`. Version 0: `"schema_version is 0, not 23"`.
- **`differential.sh`:** every committed JSON file with an `mcu` object outside `spike/` (35 files, any version) through `validate` and `solve` at the base binary. It records the exit code and stdout's sha256 (`out/differential-before.txt`, 70 lines). The build must reproduce it byte for byte.

## 2. The version table as measured

Every bump's work order is the one README §`Specification` names, in order. **(verified: all 22)** Paths are `.`-separated keys, with `[]` for each element of an array. A required path is required wherever its parent exists.

| v | Commit | Work order | Requires | Optional | Replaced | Rules on content the version before could hold (reader's, spot-checked where marked) |
|---|---|---|---|---|---|---|
| 2 | d00c9653 | 4100 | `board` | | | none |
| 3 | 50d7a6f2 | 4300c | none. The constraint rule is on existing `net_constraints`. | | | `MissingSpiConstraint` (`solve` at the bump, `validate` since 1207), `SupplyOutsideSpeedTable`, `AmbiguousPartName` (bare `W25Q64JV`) **(verified)**, `PartNameCase`, `SupplyOutsidePart`, `PartOnWrongBus`, `I2cAddressNotOffered`, `SpiClockAbovePart`, `UnjudgedSpiConstraint` |
| 4 | eb19c640 | 4300f | `connectors` | | | none |
| 5 | a20d3323 | 1308 | `net_classes`, `diff_pairs` | | | none |
| 6 | 7ca65e6e | 4300a | `mcu.hse_hz`, `motor_outputs` | | | `InvalidIdentifier` (an existing kind: a reference shaped `p<port><pin>obs` is reserved) |
| 7 | 27e504ac | 4300d | `mcu.bor_level`, `power` | | | none (the power checks run only on a non-null `power`) |
| 8 | 10806528 | 4210 | `accept_lifecycle` | | | `IoOutsidePart` **(verified)**, `LifecycleNotAccepted`, `LifecycleStatusMismatch` |
| 9 | 25640c55 | 4300b | `serial_ports` | | | none |
| 10 | 8030fa5c | 4300d3 | `power.conditions.dc_bias_derating`, `power.input.hot_plug`, `power.regulators[].junction`, `power.regulators[].controls`, `power.crossings[].controls` | | | `SupplyFloorBelowBorRising`, `EffectiveCapacitanceBelowMinimum`, `DcBiasBeyondCurve`, `BatteryInputUndamped`. Partly judged by the new keys: `ControlPinUnwired`, `BuckJunctionOverLimit` |
| 11 | 4e821edb | 4300g | `mcu.supply_source` | | | `NoProgrammingPath`, `SupplyHasNoSource`; `DuplicateNetName` and `NetShadowsDomain` (debug nets reserved) |
| 12 | 1178ca85 | 4230 | `serial_ports[].peer_deviation_ppm`, `serial_ports[].peer_deviation_source`, `acknowledge_unwitnessed` | | | `SerialBaudTolerance` (tightened), `SpiSpeedBelowKernelRange`, `SpiKernelClockUnavailable` (all `solve`), and `PullUpBelowSinkFloor` (tightened: `sink_microamps` at the domain maximum) **(verified)** |
| 13 | 70d1621d | 4220b | `i2c_devices[].pin_capacitance`, `spi_devices[].pin_capacitance` | | | none |
| 14 | 9a37ae6e | 1312 | | `test_points`, `mcu.reset_switch` | | `DuplicateNetName`, `NetShadowsDomain` (`BOOT0` reserved) |
| 15 | f845b6fd | 1313 | | `board.max_outline_um` | | none |
| 16 | 0829fe83 | 4400d | | `board.forward`, `spi_devices[].orientation` | | none |
| 17 | ee835bde | 4400g | | `mcu.foreign_clock` | | none |
| 18 | 36a494fd | 2304b | `power.battery.series_milliohms`, `motor_outputs[].esc` | | | none in `validate`. `BatterySideUnmodelled` is `simulate`'s only. |
| 19 | 943f7112 | 4240v | `mcu.core_supply`, `mcu.foreign_clock.vos` | | | none (`VosLimitExceeded` and `FlashWaitStatesBelowTable` judge a topology) |
| 20 | 14bb1e5b | 4240a | `mcu.hse` | | `mcu.hse_hz` → `mcu.hse` | none (`hse: null` skips the HSE judge) |
| 21 | 63334d69 | 1210 | | `board.drawn` | | `UnboundedPinCapacitance` (`validate` step 23; absent `drawn` now means drawn) **(verified: absent from IR at the parent)**, and `PartNotDrawable` (the CLI's `validate` only) |
| 22 | 25203c58 | 1211a | `motor_outputs[].pwm.idle`, `motor_outputs[].pwm.resolution_ps` | `pin_devices` | | none |
| 23 | ddfa64d6 | 4240b | `mcu.hse.crystal.startup_max_us`, `mcu.hse.crystal.startup_source`, `mcu.hse.external.startup_max_us`, `mcu.hse.external.startup_source` | | | none (`HseCopyNotSolved` judges a topology). The SPI kernel on an HSE board moved (`solve`). |

- **Removed:** no version removed a key.
- **Renumbering:** every "renumbered from" note checks out (all three readers). Each key first appears on main at its final number, in its bump commit. The provisional numbers exist only on branch commits that are not ancestors of main (for example `5833b9c7` at 7, `24d6bc85` at 9, `cb17efdb` at 12).
- **Solved versions in the same bumps** (a specification author cannot see them; they are not rows): 16→10, 17→11, 19→12, 20→13, 22→14, 23→15.

## 3. Findings, triaged

### Contradictions (§7 fires on these)

- **C1. The version-16 shape changed twice with no bump, and the README says otherwise.** **(verified)**
  - README `## Specification`: "Every key is required, but `test_points` and `mcu.reset_switch` (version 14), `pin_devices` (version 22), `board.max_outline_um` (version 15), `board.forward` and an SPI device's `orientation` (version 16), and `mcu.foreign_clock` (version 17), and `board.drawn` (version 21)."
  - Code: eight more optional keys (`default` plus `present`). They were added at literal 16 by `acae53f7` (1303b) and `565b32c5` (1314; the literal is 16 at both):
    - `board.stackup.outer_copper_min_um`, `.plane_copper_min_um`, `.plating_min_um`, `.copper_temperature_max_c` and `.fab_tolerances` (board.rs:156–184);
    - `board.stackup.dielectrics[].max_thickness_um` (board.rs:255);
    - `power.drop_limits` (power.rs:128);
    - `power.loads[].reference` (power.rs:321).
  - README rows 13 and 16 mention some of them; the version paragraph names none.
- **C2. README rule 11** (README.md:168): "The MCU's debug nets `SWDIO`, `SWCLK` and `NRST` are the engine's". Code: `DEBUG_NETS: [&str; 4] = [SWDIO, SWCLK, NRST, BOOT0]` (lib.rs:136, since version 14, `AET-WO-1312`). **(verified)**
- **C3. "`mcu.hse`, a required key"** (README version 20). Code: `hse_hz: 0` alone validates as `hse: null`. **(verified on the base binary, `~/.cache/aet-1215/hse/`:** `hse: null` passes, `hse_hz: 0` passes, `hse_hz: 8000000` is refused `SchemaViolation`, both keys are refused as a duplicate, and neither key is refused `missing field hse`.)
  - 31 of the 33 committed version-23 specifications carry `hse_hz: 0` and no `hse`; two carry `hse`, and none carries both. **(verified;** my first count, 36, included solved topologies and spike files.)
  - The same paragraph states the reading ("`hse_hz: 0` … reads as `null`"), so this is the weakest of the three. But README's own example writes `"hse": null`, while `examples/sentinel-01-minimal.json`, which it describes, writes `"hse_hz": 0`.

### Omissions: rules the version paragraph never names

- Versions 3, 6, 8, 10, 11, 12, 14 and 21 each added or tightened a refusal of content the version before admitted (the `Rules` column). The README's version paragraph names only `LifecycleNotAccepted` and `LifecycleStatusMismatch` (8) among them.
- Smaller gaps:
  - v8's three other lifecycle kinds;
  - v16: `orientation` requires `board.forward` (`ForwardUndeclared`);
  - v19: "cited with a document number and its revision", which `board::source` does not check;
  - v23: `HseCopyNotSolved` sits in the specification paragraph, though it judges a topology.
- README line 3, "Nothing defaults", is contradicted only in wording by `board.drawn`'s documented default. I took that as prose, not a contradiction.

### Changes no row can hold (landed under an unchanged version)

- **Refusals landed without a bump.** A file valid at that version stopped validating, still at that version. Mechanical list, from `kinds + (window, by a commit other than the bump)`, with the literal each landed under **(verified for the starred ones)**:
  - at 2: `SpiDeviceIndex`, `SpiDevicePeripheral`, `SpiDeviceSelect`, `ReferenceCollision`, `PartsTableInvalid`;
  - at 5: `ThermalVia*` (5 kinds);
  - at 8: `ConnectorBudgetTooLarge`, `ConnectorReferenceReserved`, `NoAdcPin`;
  - at 11: `InvalidDeviceName`, `ReservedName`, `UnknownPart`;
  - at 12: `SpiModeNotDriven`;
  - at 13: `PinCapacitanceBelowTypical` (`2b436729`*);
  - at 14: `LoadAboveSpeedTable`, `PinSpeedMismatch`, `PinSpeedNotSolved`, `RateAbovePinSpeed`, `RiseTimeBelowPinSpeed`;
  - at 18: `LoadNamesNoPart` (`2fda6e13`*);
  - at 19: `LoadNamesCountedPart` (`fff49a49`*);
  - at 22: `ConsoleBaudNotRoundTrip` (`adb8f61a`*).
  - Also: `MissingSpiConstraint` moved from `solve` to `validate` under `AET-WO-1207`.
  - Whether each refuses older content is judged only for the starred ones. The list is complete for kinds written `=> "Kind"` in `aethelgard-ir/src`, and a lower bound beyond that (CLI-only kinds such as `PartNotDrawable`).
- **Optional keys landed without a bump:** C1's eight.

## 4. Forks for the ruling, each with my lean

- **D1. C1–C3.** **Lean:** correct the crate README in this PR, inside the fence.
  - C1: name the eight keys in the "Every key is required, but" list, as optional at 16 by `1303b` and `1314`, with no bump.
  - C2: add `BOOT0` to rule 11.
  - C3: "required, or `hse_hz: 0` alone, the version-19 spelling".
  - The table carries none of them, because none is a version change.
- **D2. What `rules` holds.** **Lean (a):** every kind a bump added or tightened whose check reads a key the version before already had. That includes existing kinds (6's `InvalidIdentifier`, 11's and 14's `DuplicateNetName`/`NetShadowsDomain`, 12's `SerialBaudTolerance` and `PullUpBelowSinkFloor`), `solve`'s kinds, and the CLI `validate`'s (`PartNotDrawable`). It does not include topology-only kinds.
  - (b): only new kinds in `Specification::validate`.
  - The table above is (a).
- **D3. Where a listed rule is cited.** **Lean (a):** the README's version paragraph gains each listed kind by name, so the table is cited to the prose row by row, and a test holds every row's paths and kinds to its paragraph.
  - (b): the row cites the bump commit, and the prose test checks paths only.
- **D4. The changes of §3's last part.** **Lean:** out of the table. A version line cannot tell an author about them, because the file's version is current. Make it a backlog row: "a validate refusal or a key added without a bump is a bump (or the work order says why not)".
- **D5. The `mcu.hse_hz` exception.** The WO's test "replaced paths the schema must not have" collides with C3.
  - **Lean:** row 20 states `replaced: mcu.hse_hz by mcu.hse`.
  - The test carves `mcu.hse_hz` out by name and pins exactly today's behaviour on every committed specification: `0` alone validates, any other value is refused, and either value beside `hse` is refused.
  - Every other replaced or removed path must draw serde's `unknown field`.
- **D6. Version 0** never existed, and version 1 was a bare literal. **Lean:** refused as today, with no rows, like a newer file.
- **D7. The structured field.** **Lean:** the key is `error.entries`, as `2307`'s, with one element per row: `{version, work_order, requires, optional, replaced: [{path, by}], removed, rules}`. One optional key with a kind-specific element; absent for a newer file, so its bytes do not move.
- **D8. Where `entries` is absent.** `check drc --spec` and `check support` flatten the refusal into `run::Stop` (`kind`, `id`, `message`), so `entries` would be dropped there. `Stop` has 62 struct-literal constructions. The message's tail comes from the IR's `Display`, so it also reaches library callers.
  - **Lean (b):** the message lists the rows compactly and does not mention `error.entries`. `cli/README.md` names the two commands that carry the message only, as claimants already are.
  - (a): `Stop` carries the field.

## 5. The build, as ruled

- **`aethelgard_ir::versions`** (`src/versions.rs`) holds the table and the functions around it.
  - `CHANGES` has one row per version from 2 to 23, as in §2, with `rules` per D2 (a). Row 23 also names `SerialBaudReadAsKbaud`, as ruled: `1214` added it at version 23, a rule on an existing key. It also names `SpiSpeedBelowKernelRange`, tightened for an HSE board.
  - `lacked_by(v)` returns every row after `v`, and nothing for 0. A current or newer version lacks none.
  - `unsupported(v)` is the one constructor, used at both specification sites (`validate`'s step 1 and the CLI's pre-parse).
  - `UnsupportedSchemaVersion` gains `changes`. It is `&[]` for a `SolvedTopology`, so a solved file never lists specification rows.
- **The message** appends the rows compactly for an older file, for example `schema_version is 20, not 23; it lacks 3 version change(s), each named by path, and nothing is filled: 21 (AET-WO-1210) optional [board.drawn], refuses by [UnboundedPinCapacitance, PartNotDrawable]; 22 (AET-WO-1211a) requires [...], optional [pin_devices]; 23 (AET-WO-4240b) requires [...], refuses by [...]`. A newer file and version 0 keep today's bytes exactly.
- **The CLI** adds `error.entries` (D7): one `{version, work_order, requires, optional, replaced: [{path, by}], removed, rules}` per row, absent otherwise.
  - Measured on every entry point (`out/entrypoints-after.txt`): `validate` (file and stdin), `solve`, `emit`, `generate`, `extract`, `check speeds` and `verify` print it.
  - `check drc --spec` and `check support` print the message only (D8 (b), `cli/README.md`).
  - To stay within clippy's `result_large_err`, `entries` holds the version (`Lacking(u32)`, serialized as its rows) and `path` became `Option<Box<str>>`. Neither changes a byte of the output.
- **The README** carries D1 and D3.
  - C1: the eight optional keys of version 16 join the "Every key is required, but" list.
  - C2: `BOOT0` joins rule 11.
  - C3: `mcu.hse` is "a required key, for which `hse_hz: 0` alone still stands".
  - Every listed rule is named in its version's sentence.
  - The closing sentence states the listing, the path notation, and "a row names paths, never a value".
- **Tests.** `crates/aethelgard-ir/tests/versions.rs` (11):
  - the last row is the literal, and the rows run 2..=literal with none skipped;
  - version 20 lists 21, 22 and 23 with their work orders;
  - version 1 lists 2 to 23;
  - the next version, 0 and `u32::MAX` name the current version only, with today's message;
  - every older version lacks exactly the rows after it;
  - the message suggests no value.
  - The schema is tested by serde on all 33 committed version-23 specifications. Each required path, removed wherever it is present, gives `missing field`. Each optional path parses when absent and, given a wrong value, gives no `unknown field`. Each replaced path gives `unknown field`, except `mcu.hse_hz`, which is pinned by name (D5).
  - Every rule is a kind the crate returns (`=> "Kind"`), and every row's paths and kinds are named in its README sentence.
  - `stackups_tier1.rs` gains `an_older_specification_lists_every_change_it_lacks`: `validate`, `solve`, `emit` and `verify` at versions 20, 1, 24 and 0.
  - Two existing CLI tests pinned an older version's exact message (`bench_tier1.rs` v13, `outline_tier1.rs` v14). They now compare it to `versions::unsupported(N)`. Seven IR tests put `changes: _` (or `lacked_by(v)`) first in the variant, so `expected: 23` stays the line the renumber kit rewrites.
- **The next bump** (`1211e`, 24) writes row 24, and the last-row test fails until it does. It also edits by hand the literal lists the WO asks for (`[21, 22, 23]`, `2..=23`) and the `not 23; ` prefixes, which the kit does not rewrite. The kit gains nothing.

## 6. Open: D6's wording

- The ruling reads "Version 0 and version 1 are refused as today, with no rows". My lean was version 0 alone, and WO §1.3 requires "a version-1 file lists 2 to 23".
- Built to the WO: version 1 lists 2 to 23, and version 0 lists none. I asked on the bus (2026-11-22).
- If version 1 is meant to have no rows too, it is one guard in `lacked_by` and two tests.

## 7. Adversarial pass (targeted, §5)

- **A renumbered work order.** Every row's work order is its final number on main (§2), and a test finds `Version N (`WO`` in the README for each row.
- **A key made optional.** The serde test removes every required path from every committed specification that holds it, and must see `missing field`. A key later relaxed, or a row classed wrongly (mutant M8), fails it.
- **A rule added without a key.** Version 3 is such a row, and so are the rules of 6, 8, 11, 12, 14 and 21. A rule landed under an unchanged version is no row (D4, the §7.1 row below).
- **A solved-version change.** Not a row (§2); `changes` is `&[]` for a `SolvedTopology`.
- **A message that suggests a value.** It names paths, kinds, versions and work orders only. A test refuses `null`, `true`, `false`, `pulse_us":`, `1,000,000`, `= ` and `e.g.` in version 1's message.
- **Not a number.** A `schema_version` that is not a `u32` is refused as before, by the CLI's pre-parse falling through to `SchemaViolation`. No change.
- **Bytes.** `out/differential-compare.txt`: 70 (file, command) lines over the 35 committed specifications. The 4 that differ are the two version-1 files in `docs/work-orders/AET-WO-1200.repro/`, through `validate` and `solve`, which now list rows 2–23. Every current-version file's bytes are unchanged, and so are versions 0 and 24 on every entry point. **No file that validates today stops validating.**

## 8. Gates, mutants, reduction

- `cargo fmt --all -- --check`: clean. `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace --no-fail-fast`: run in a 32 G capped scope on the head before the reduction, it gave 156 binaries, 2,624 passed and 0 failed. The final run is on the READY head (§9). `TIER2_MIN` is unchanged, and no `ci/` row is needed: no new test binary, and nothing reads a docs-only path.
- **Mutants** (`spike/1215/mutants.sh` → `out/mutants.txt`): 12 of 12 killed.
  - M1: drop row 22. M2 and M2b: swap rows 21/22 and 2/3.
  - M3: version 0 lists every row. M4: an older file lists its own row.
  - M5: the message lists no row. M6: a newer file carries `entries: []`. M7: the CLI omits `entries`.
  - M8: a required path made optional. M9: row 20 loses its replacement.
  - M10: a rule its prose does not name. M11: a path the schema lacks.
  - Each is killed by a failing test (M1 checked by hand: 4 tests fail, and it compiles).
- **Reduction pass (diff):** −6 net, 0 duplicates removed, 2 dead paths removed (the message's unreachable "version line only" branch, and `lacked_by`'s `>= current` guard, which `partition_point` already covers; M3 still dies on the remaining `== 0` guard), 1 protected kept, 1 finding below.
  - Protected: the table beside the README prose, which is one fact in two forms held equal by a test.
  - Finding (outside the fence's intent): the doc comment above `SPECIFICATION_SCHEMA_VERSION` (`lib.rs`) is a third prose copy of every version. It is already stale (`lib.rs:138` "version 15"; `hse_hz` "the version-19 key"). One home, the README, citing it would remove ~70 lines.

## 9. For the architect

- **The §7.1 row (D4), for the roadmap, which I do not edit:** "A `validate` refusal, or a key, added without a bump is a bump, or the work order says why not." Evidence:
  - C1: eight optional keys at version 16 (`1303b` `acae53f7`, `1314` `565b32c5`);
  - §3's refusals under unchanged versions (`2b436729`, `2fda6e13`, `fff49a49`, `adb8f61a`, and `1214`'s `SerialBaudReadAsKbaud` at 23).
- **D6**, §6.
- **The `lib.rs` doc comment**, §8's finding.
