# AET-WO-1215 FEEDBACK — STOPPED at §1.1 (§7: the README prose and the code disagree about a version)

**Builder:** Claude (Opus 5.5), 2026-11-22. **Base:** main `ddfa64d6` (specification 23, solved 15). **Branch:** `wo/aet-1215-version-changes`.
**State:** the measurement is complete and committed (`spike/1215/`). §7's stop condition fired on three contradictions (C1–C3 below). The build is written and compiles, but it is parked on a local branch, not pushed, because forks D1–D8 decide its `rules` column and its prose test. One ruling on D1–D8 unblocks all of it.
**Gates:** this push is spike-only. It touches nothing under `crates/` and no workflow. The CLAUDE.md gates are not run on it. They run on the build after the ruling.

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

## 5. The build, parked

The build is on the local branch `wip/aet-1215-build` (`85dbd358`), not pushed. It holds:
- `aethelgard_ir::versions` with `CHANGES` (rows 2–15 drafted to D2 (a)), `lacked_by(v)` and `unsupported(v)`, the only constructor at the two specification sites;
- `UnsupportedSchemaVersion { version, expected, changes }`, with `changes: &[]` for a `SolvedTopology`;
- the CLI's `RefusalBody.entries`;
- the seven IR tests that match the variant, with `changes: _` put first so the line `expected: 23` keeps the renumber kit's `expected: N$` form;
- `tests/versions.rs`: the last row against the literal, rows contiguous, 20 → 21/22/23, 1 → 2–23, and 24, 0 and `u32::MAX` refused with no rows. Each path is checked against serde on every committed version-23 specification: a required path removed gives `missing field`, an optional one probed gives no `unknown field`, and a replaced one gives `unknown field`, except D5. Each rule must be a kind the crate returns, and each row is held to its README paragraph (D3).

After the ruling: rows 16–23, then gates, the differential against `out/differential-before.txt`, mutants (drop a row, swap two rows), the adversarial pass, the reduction pass, and READY.
