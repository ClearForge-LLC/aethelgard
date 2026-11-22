# Specification schema history, versions 2 to 8 (read-only measurement)

HEAD measured: ddfa64d6. Bump commits: 2 d00c9653, 3 50d7a6f2, 4 eb19c640, 5 a20d3323, 6 7ca65e6e, 7 27e504ac, 8 10806528.
Citation form `commit:file:line`. `lib.rs`, `solve.rs` = `crates/aethelgard-ir/src/`; `README` = `crates/aethelgard-ir/README.md`.
Each bump commit is one squash-merge of a single work order. "Rules" below are read from the bump commit's own diff, NOT from witnesses.txt's "(window)" kind lists, which also include kinds from other merges between bumps (for example the 4200 SPI kinds `SpiDeviceIndex`, `SpiDevicePeripheral`, `SpiDeviceSelect` and `ReferenceCollision` listed under 3, and `ThermalVia*` under 6).
Prose sources, identical at the bump commit and HEAD for versions 2-8 (verified: HEAD lib.rs:47-61 and README:7 carry the same sentences):
- README:7 (the "Version 2 ... Version 8" paragraph; heading at README:5).
- The doc comment above `SPECIFICATION_SCHEMA_VERSION`. At the bump commits: d00c9653:lib.rs:27-30, 50d7a6f2:lib.rs:30-33, eb19c640:lib.rs:35, a20d3323:lib.rs:36-39, 7ca65e6e:lib.rs:41-43, 27e504ac:lib.rs:44-47, 10806528:lib.rs:45-51. At HEAD: lib.rs:47-61.

Key mechanics, all verified in code: a key is optional only with `#[serde(default)]` or `deserialize_with = "present"`. Nothing in 2..8 has either (the first optional spec keys are `test_points` at v14 and later). So every key added in 2..8 is required.

---

## Version 2 (d00c9653, AET-WO-4100)

1. requires: `board` (whole subtree, required, not nullable): `board.copper_layers`, `board.layers`, `board.stackup`, `board.rules`. d00c9653:lib.rs:52 `pub board: board::Board`. Subtree: `d00c9653:board.rs:59-129` has no default anywhere. `layers[].net` is `Option` with `deserialize_with = "Option::deserialize"` (board.rs:73): the key is required, `null` allowed. Unconditional.
2. optional: none.
3. replaced / removed: none. Witness: `keys -` is empty.
4. rules (content a v1 file could already hold): none. All 11 new kinds (`UnsupportedLayerCount`, `LayerTable`, `LayerRole`, `PlaneNet`, `PlaneLayout`, `DielectricTable`, `FinishedThickness`, `StackupValue`, `InvalidSource`, `RuleBelowFloor`, `UnsupportedViaRule`) judge only `board` (called at d00c9653:lib.rs:1145).
5. DISAGREEMENTS: none on version-2 content. Wording only: README and lib.rs say "added `board`", which means required. Later versions use "adds ... optional" for keys that may be absent (14-17, 21, 22). The word "added" is therefore not a reliable signal of optionality (see also 4).

## Version 3 (50d7a6f2, AET-WO-4300c)

1. requires: NO new JSON key (witness `keys +` is empty). The new requirement is on existing key `net_constraints`: one entry (`net_name`) for every `spi_buses[].sck.net` and every `spi_buses[].mosi.net`. Conditional: only when `spi_buses` is non-empty. Refused by `solve` as `MissingSpiConstraint` (50d7a6f2:solve.rs:437; `spi_lines()` at solve.rs:~420-450). `validate` did NOT refuse it at this commit. At HEAD it is `validate`'s own check (HEAD lib.rs:3181, comment at solve.rs:50, `AET-WO-1207`).
2. optional: none.
3. replaced / removed: none. Value rename: the flash part name `W25Q64JV` became `W25Q64JVxIQ` (50d7a6f2:parts.rs test, `names == [..., "W25Q64JVxIQ"]`; `BARE_W25Q64JV` constant added). A v2 file naming `W25Q64JV` in any case is refused `AmbiguousPartName` (lib.rs:1531, in `part_entry`, lib.rs:1525). A v2 file naming a table part in the wrong case (for example `w25q64jvxiq`, `bme280`) is refused `PartNameCase` (lib.rs:1545).
4. rules (new kinds that refuse content valid at v2; all via keys that already existed):
   - `AmbiguousPartName` (lib.rs:1531): any v2 device naming bare `W25Q64JV`.
   - `PartNameCase` (lib.rs:1545): any device whose part equals a table part only case-insensitively.
   - `SupplyOutsidePart` (lib.rs:1187, `validate_parts` at lib.rs:1154): a device whose supply domain (nominal +/- tolerance) leaves the part's range. Step 14 is called at lib.rs:1146.
   - `PartOnWrongBus` (`wrong_bus` lib.rs:1560, from `check_i2c_address` lib.rs:1573 and `check_spi_clock` lib.rs:1598): an I2C part in `spi_devices` or the reverse, for example a BME280 on an SPI bus.
   - `I2cAddressNotOffered` (lib.rs:1573): an I2C device's `address` not in the part's strap-offered set.
   - `SpiClockAbovePart` (lib.rs:1598): `spi_buses[].speed_hz` above the part's `max_clock_hz`.
   - `UnjudgedSpiConstraint` (lib.rs:1163): a `net_constraints` entry on an SPI bus's `miso.net` or a device's `cs.net`. A v2 file could carry one, and v3 refuses it.
   - `MissingSpiConstraint` (solve.rs:437) and `SupplyOutsideSpeedTable` (solve.rs:429): an SPI bus on an MCU supply whose min or max leaves 1620 to 3600 mV. See item 1.
   - `OutputSpeedTableInvalid` (lib.rs `OutputSpeedTable`, speeds.rs:255) and `SpiLineMismatch` (lib.rs `validate_spi_lines`) concern embedded data and the solved topology, not specification content.
5. DISAGREEMENTS:
   a. README:7 / lib.rs:30-32 say v3 "names parts as the version-2 parts table does (`W25Q64JVxIQ`)". The code also adds SIX per-part limit checks (supply range, bus kind, I2C address, SPI clock, unjudged MISO/CS constraint, plus the case rule). No prose mentions them. Prose omission of refusing rules, reported as a disagreement.
   b. "version-2 parts table" is `PARTS_SCHEMA_VERSION` 2 (50d7a6f2:parts.rs `PARTS_SCHEMA_VERSION: u32 = 2`), not specification version 2. The README conflates the two counters.
   c. "requires a net constraint on every SPI bus's `sck` and `mosi`" is true at the bump commit only at `solve`, not `validate` (see item 1). It is true at `validate` at HEAD (HEAD lib.rs:3181).
   d. The README at this commit is otherwise consistent (heading v3 README:5, example `schema_version` 3 README:11).

## Version 4 (eb19c640, AET-WO-4300f)

1. requires: `connectors` (top-level array; an empty array is valid, so "required" means the key must be present). eb19c640:lib.rs:61 `connectors: Vec<connectors::Connector>`, no default. Per element (connectors.rs:116-146): `reference`, `part`, `standard` (nullable, key required), `pins[]` (`pin`, `net` nullable-but-required, `offboard` nullable-but-required: `capacitance_femtofarads`, `source`). Unconditional at top level.
2. optional: none.
3. replaced / removed: none.
4. rules: none on old content. All 11 new connector kinds (`UnknownConnectorPart`, `ConnectorPin*`, `ConnectorNetUnknown`, `ConnectorBudget*`, `UnknownConnectorStandard`, `Connector*Contradicts*`) fire from `connectors::validate` (lib.rs:1244, called at step 15), which loops over `connectors` only. Connector references join the reference-uniqueness check (lib.rs:1447), which cannot fire for a v3 file with no connectors. The `GND` in witness `kinds +` is a regex artifact (the `GND` constant), not a kind.
5. DISAGREEMENTS: wording only. README:7 and lib.rs:35 say v4 "adds `connectors`" / "added". The code makes it required, as for v2's `board`; v5-v8 prose uses "requires". The README's later convention is "adds ... optional" (v14-17, 21, 22), so "adds" is ambiguous here. The README's trailing sentence "nothing supplies ... a connector list for it" implies required.

## Version 5 (a20d3323, AET-WO-1308)

1. requires: `net_classes` and `diff_pairs` (top-level arrays; empty valid). a20d3323:lib.rs:69 and :71, no default. Per element (classes.rs:54-78): `net_classes[].name`, `track_width_um`, `via` (nullable, key required; `diameter_um`, `drill_um`), `nets`; `diff_pairs[].name`, `nets`, `gap_um`, `width_um`. Unconditional.
2. optional: none.
3. replaced / removed: none.
4. rules: none on old content. All 10 kinds (`ClassBelowMinimum`, `ClassAboveMaximum`, `ViaRingUnderRules`, `DuplicateClassName`, `ClassNoNets`, `ClassUnknownNet`, `ClassNetNotLiteral`, `NetInTwoClasses`, `PairNets`, `PairOnRail`) are raised only inside `classes::validate` (called at step 13a) over the two new arrays.
5. DISAGREEMENTS: README is internally stale at this commit. a20d3323:README:5 heading says "version 5", but the JSON example at a20d3323:README:11 has `"schema_version": 4` (the example does contain `net_classes` and `diff_pairs`). The same staleness persists at 7ca65e6e (see 6). The prose itself ("requires `net_classes` and `diff_pairs`; an empty array declares none", README:7) matches the code.

## Version 6 (7ca65e6e, AET-WO-4300a)

1. requires: `mcu.hse_hz` (u32, 7ca65e6e:lib.rs:89, no default) and `motor_outputs` (top-level array, empty valid; lib.rs:77). Per element (motor.rs:82-90): `label`, `pin` (nullable, key required), `protocol` (`pwm`|`dshot300`|`dshot600`), `pwm` (nullable, key required; object `{rate_hz, pulse_min_us, pulse_max_us}`, motor.rs:71-75). Unconditional at top level. `pwm` must be an object exactly when `protocol` is `pwm`, else `MotorTimingShape` (lib.rs:1864).
2. optional: none.
3. replaced / removed: none in 2..8. NOTE: `mcu.hse_hz` itself is later replaced by `mcu.hse` at v20. See the HEAD check at the end.
4. rules (old-content hits):
   - `InvalidIdentifier` (existing kind) with `OBSERVER_RULE` (7ca65e6e:lib.rs:1389-1390; rule text lib.rs:1396; `is_observer_name` lib.rs:1400): a device `reference` shaped `p` + port letter a-k + pin 0-15 + `obs` (for example `pa8obs`, any case) is now refused. A v5 file could carry such a reference. The prose does not mention it.
   - New-key shape only: `CrystalNotSupported` (lib.rs:1567: `hse_hz != 0`), `UnjudgedMotorConstraint` (lib.rs:1497, a `net_constraints` entry on a motor label), `NetShadowsDomain` for a motor label `GND` (lib.rs:1761), `ZeroMotorRate`, `PulseRange`, `PulseExceedsPeriod`, `MotorTimingShape` (`motor_shape` lib.rs:1864). `MotorOutputMismatch`, `TimerShared` and `UnsupportedClock` concern the solved topology.
5. DISAGREEMENTS:
   a. At 7ca65e6e the README is stale. 7ca65e6e:README:5 is "version 5", the example at README:11 has `"schema_version": 4`, and the example has neither `mcu.hse_hz` nor `motor_outputs`, both required at 6 (the constant is 6). The prose at README:7 and lib.rs:41-43 is correct. The README example does not parse at 6.
   b. README:7 says `mcu.hse_hz` is "the only value admitted until a crystal part exists". Code: any non-zero value is refused `CrystalNotSupported` (lib.rs:1567). Consistent.
   c. The prose omits the observer-name reservation (item 4), which refuses v5 content.

## Version 7 (27e504ac, AET-WO-4300d slice A)

1. requires: `mcu.bor_level` (u8, 27e504ac:lib.rs:102, no default) and `power` (top-level, key required, value `null` or an object; lib.rs:85 `deserialize_with = "Option::deserialize"`). Under a non-null `power`, ALL of these are required (power.rs:60-201): `power.conditions` (`board_temperature_celsius{min,max,source}`), `power.input{reference,part}`, `power.battery{chemistry,cells{min,max},cell_min_millivolts{value,source},cell_max_millivolts{...}}`, `power.regulators[]{reference,part,input,output,feedback(nullable-required),external[]}`, `power.loads[]{domain,milliamps,source}`, `power.sense[]{net,top,bottom,reservoir(nullable-required),adc_resolution_bits}`, `power.crossings[]{reference,part,bus,a_domain,b_domain,cs_pull_up,external[]}`. Conditional: the subtree only when `power` is non-null. A version bump with `power: null` adds nothing else.
2. optional: none. `power.regulators[].feedback` and `power.sense[].reservoir` may be `null` (key still required).
3. replaced / removed: none.
4. rules: none on old content. With `power: null`, `power::check` returns immediately (27e504ac:power.rs:1966-1968), so none of the ~25 power kinds fire. `BorLevelOutOfRange` (lib.rs:1723, `bor_level > 3`) judges only the new key. One relaxation, not a refusal: step 9 now lets a device sit on a declared crossing's other domain (lib.rs diff near `crossed`).
5. DISAGREEMENTS: README:7 and lib.rs:44-47 match the code ("requires `mcu.bor_level` (0 to 3) and `power`, whose value `null` declares no power tree"). Stale README only: README:7 closing sentence "nothing supplies a board, a declaration, a connector list, a class list or a motor output list for it" does not list the power tree or bor level. At 27e504ac the README example (27e504ac:README:5-53) carries `hse_hz`, `bor_level` and `"power": null` but NO `motor_outputs` (required since 6), so it does not parse at 7.

## Version 8 (10806528, AET-WO-4210)

1. requires: `accept_lifecycle` (top-level array, empty valid; 10806528:lib.rs:94, no default). Per element (`LifecycleAcceptance`): `part`, `status`, `statement`. Unconditional at top level. A non-empty list is needed whenever a device names an `eol` or `nrnd` part, which at this commit are `ICM20948` and `ICP10111` (10806528:data/parts.json:80-81 and :213-214). A file valid at 7 that uses `ICM20948` therefore cannot migrate by the version line plus `[]`.
2. optional: none.
3. replaced / removed: none.
4. rules (old-content hits, via existing keys):
   - `IoOutsidePart` (10806528:lib.rs:1855): a device whose bus domain (or the crossing's B-side domain for an SPI device) leaves the part's I/O range (`Part::io_range`; parts.json `io_millivolts`: BME280 1200-3600 mV, ICM20948 1710-1950 mV). A v7 file could hit this with no new key. NEW and not tied to `accept_lifecycle`.
   - `LifecycleNotAccepted` (lib.rs:1924) and `LifecycleStatusMismatch` (lib.rs:1932): a v7 file using an eol/nrnd part, which has no `accept_lifecycle` entry to carry.
   - `ICP10111` is a new parts-table row (parts.json:213; `RENODE_MODELS` gains `Sensors.ICP_101xx`). A v7 file naming it was unjudged before, and is now judged by every per-part rule from 3 (and needs lifecycle acceptance).
   - New-key shape only: `LifecycleAcceptedTwice` (lib.rs:1896), `LifecycleStatementInvalid` (lib.rs:1903: empty, over 512 bytes, or not printable ASCII), `LifecycleAcceptanceUnused` (lib.rs:1948: an entry for a part no device uses as eol/nrnd). `validate_lifecycle` is lib.rs:1891, called at lib.rs:1883.
5. DISAGREEMENTS:
   a. README:7 names only `LifecycleNotAccepted` and `LifecycleStatusMismatch`. The code at 10806528 also refuses `LifecycleAcceptedTwice`, `LifecycleStatementInvalid` (statement bounds 1-512 bytes printable ASCII, README says only "non-empty") and `LifecycleAcceptanceUnused` (an entry for an unused or `active` part is refused). The prose "an empty array acknowledges none" is true, but it omits that a non-empty entry for a part not used is refused.
   b. Prose omits `IoOutsidePart` (item 4), which refuses v7 content independent of the new key. Prose omission of a refusing rule.
   c. README stale at 10806528: heading README:5 "version 8", but the example at 10806528:README:11 has `"schema_version": 7` and no `accept_lifecycle`, so it does not parse at 8.
   d. lib.rs:45-51 says "an empty list acknowledges none" and "Nothing supplies any of them for an older file". Consistent.

---

## HEAD check: keys removed or replaced in 2..8

- No version in 2..8 removed or replaced a key (witnesses `keys -` and `fields -` are empty for all seven).
- HEAD still requires every key added in 2..8, with no default: `board`, `connectors`, `net_classes`, `diff_pairs`, `motor_outputs`, `power`, `accept_lifecycle` (HEAD lib.rs:136-190 struct; `power` and the arrays are plain required), and `mcu.bor_level`.
- Related, outside the range: `mcu.hse_hz` (added at 6) IS replaced later, at v20, by `mcu.hse`. HEAD lib.rs:1014-1040 still accepts `hse_hz`, but only as `0` (read as `hse: null`) and not beside `hse`; any other value, or neither key, is refused (`missing_field("hse")`). So HEAD still parses the v6 key in one value.
- HEAD CLI reads the version first (`aethelgard-cli/src/main.rs:2795-2807`, `parse_spec`) and refuses any other version as `UnsupportedSchemaVersion` before deserializing. A version 2..8 file is never judged by the keys above at HEAD. Calling `Specification` deserialization directly would refuse it for missing fields.
- HEAD comment drift (outside the range, noticed in passing): the `Specification` doc comment at HEAD lib.rs:138 still says "version 15" while the constant is 23. HEAD lib.rs:1014 and :1017 describe `hse_hz` as "the version-19 key", although v6 introduced it (v6..v19 all carried it).

## Summary of disagreements (most important first)

1. v3: README/lib.rs prose says only "names parts as the version-2 parts table does"; the code (50d7a6f2:lib.rs:1154-1215, 1525-1610) adds `SupplyOutsidePart`, `PartOnWrongBus`, `I2cAddressNotOffered`, `SpiClockAbovePart`, `UnjudgedSpiConstraint`, and the case rule `PartNameCase`, none mentioned. "version-2 parts table" is the parts-data schema, not specification version 2.
2. v3: "requires a net constraint on every SPI bus's `sck`/`mosi`" was enforced only at `solve` at 50d7a6f2 (solve.rs:437), not `validate`; at HEAD it is `validate`'s (lib.rs:3181).
3. v8: prose omits `IoOutsidePart` (a v7 file can hit it with no new key) and three of the five new lifecycle kinds, and README:7 understates statement validation (1-512 bytes printable ASCII) and unused-entry refusal. `ICM20948` and `ICP10111` are `eol`, so a v7 file using either cannot migrate by version line alone.
4. v6: prose omits the new device-reference reservation `pa<n>obs` (`OBSERVER_RULE`, 7ca65e6e:lib.rs:1389-1396), refused as `InvalidIdentifier`.
5. README examples at bump commits did not match the bump: a20d3323 (version 5, example says 4), 7ca65e6e (version heading 5, example 4, no `hse_hz`/`motor_outputs`), 27e504ac (no `motor_outputs`), 10806528 (example says 7, no `accept_lifecycle`).
6. v2 and v4: "added/adds" means required, while later "adds ... optional" is the usage for 14-17, 21 and 22. Wording ambiguity only.
