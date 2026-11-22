# Specification schema versions 9..15: measured from git history (read-only)

Citation form is `commit:file:line` (file under `crates/aethelgard-ir/`; `lib.rs` = `src/lib.rs`). "HEAD" = ddfa64d6 (spec 23).
Scope of "rules": kinds added in the bump commit's own diff. The `witnesses.txt` "kinds + (window)" lists also hold kinds from other commits of the window (e.g. ConnectorBudgetTooLarge, NoAdcPin in the v9 window); those are not bump changes and are not listed. Kinds from `SolveError` (solve.rs) are marked `[solve]`.

Common: each version was bumped exactly once, by one commit, on HEAD's history (list from `git log -G'SPECIFICATION_SCHEMA_VERSION: u32 = '`: 25640c55 -> 8030fa5c -> 4e821edb -> 1178ca85 -> 70d1621d -> 9a37ae6e -> f845b6fd, in that order, no number repeated). No key was removed or replaced in 9..15. HEAD still declares every key below with the same required/optional status (checked: `lib.rs` HEAD:178 serial_ports, :185 acknowledge_unwitnessed, :191 test_points `serde(default)`, :351 supply_source `Option::deserialize`, :355 reset_switch `serde(default)`, :457/:470 pin_capacitance `Option::deserialize`; `power.rs` HEAD:147 dc_bias_derating, :178 hot_plug, :247 junction, :249 and :361 controls, all required; `serial.rs` HEAD:146/:149; `board.rs` HEAD:80 max_outline_um `default`+`present`).

---------------------------------------------------------------------

## Version 9 (25640c55, AET-WO-4300b)

1. requires: `serial_ports` (top-level array; `[]` declares none) (25640c55:lib.rs:99, no serde attribute). Element subtree (only when an element exists): `serial_ports[]` = label, peripheral (nullable), role, framing{baud,data_bits,parity,stop_bits,inverted}, rx{net,pin}, tx{net,pin}, negotiated (nullable) (from witnesses.txt:59; not individually re-derived).
2. optional: none.
3. replaced: none. removed: none.
4. rules (kinds in the diff): DuplicateSerialLabel, SerialPeripheralLpuart, UnknownSerialPeripheral, SerialPeripheralTwice, SerialFramingShape, SerialFramingContradictsRole, SerialNegotiatedOffCrsf, SerialNegotiatedAtCitedBaud, UnjudgedSerialConstraint (lib.rs:1948), SerialPortMismatch (solved topology), ConnectorSerialRoleContradictsStandard (ds009.rs:257), SerialPortNotAtConnector (connectors.rs:410). All judge only the new `serial_ports` content or a connector pin that names a serial net. None refuses content a v8 file could already hold: a v8 file has no serial port, so none of them can fire on it. The DS-009 TX/RX roles went from `Undeclared` (refused every net) to `SerialTx/SerialRx` (ds009.rs diff), a loosening. `NetShadowsDomain` is only extended to serial nets (lib.rs diff), a new key.
5. renumbering: doc comment says "renumbered from 7 by the same rule" (25640c55:lib.rs:52). `serial_ports` first appears in HEAD's history at 25640c55 under 9 (`git log -S'pub serial_ports: Vec<SerialPort>'`). Branch-only commit 5833b9c7 carried it under spec 7; it is not an ancestor of HEAD. Main never carried it under another number.
6. disagreements: none about the keys. Small prose gap: the doc comment carries "renumbered from 7", the README paragraph (HEAD README.md:7) carries no renumber note for 9 (nor for 10; 11 and 12 have one). Not a content disagreement.

---------------------------------------------------------------------

## Version 10 (8030fa5c, AET-WO-4300d3)

1. requires (all inside a non-null `power`; with `power: null` nothing is required):
   - `power.conditions.dc_bias_derating` (array, `[]` allowed) (8030fa5c:power.rs:102)
   - `power.input.hot_plug` (array, `[]` allowed) (power.rs:133)
   - `power.regulators[].junction` (key required, value `null` or object `{copper_square_centimetres,loss_milliwatts}`, each `{value,source}`) (power.rs:195, `Option::deserialize`)
   - `power.regulators[].controls` (array, `[]` allowed) (power.rs:197)
   - `power.crossings[].controls` (array, `[]` allowed) (power.rs:299)
   Conditional: the regulator and crossing keys only when such an element exists. An element of `controls` is `{pin, strap, wire}`, strap and wire each required but nullable, exactly one null (shape rule, "takes exactly one of `strap` and `wire`", in the `controls` closure of power.rs `shape`).
2. optional: none (the arrays may be empty, the keys may not be missing).
3. replaced: none. removed: none.
4. rules (all `power::Refusal`; a v9 file with a power tree can hit them):
   - SupplyFloorBelowBorRising: 8030fa5c:power.rs:2654 (`fn bor_rising` at :2642). Reads only `mcu.bor_level` and the minimum of `mcu.supply`'s domain, both v9 keys. A v9 file with, e.g., BOR level 3 and a domain minimum not above the row's rising maximum (power-parts.json `bor_rising_max_millivolts`) is refused with nothing new added.
   - EffectiveCapacitanceBelowMinimum: :1885, :1924, :2618 (`fn effective` at :2555). Judges every existing ceramic `external[]` capacitor at its working voltage; a v9 file can hold a cap with no maker curve and no declared derating covering it.
   - DcBiasBeyondCurve: :2565. A cap working past the last point of the held Murata curve; existing capacitors and domain voltages.
   - BatteryInputUndamped: :2342, :2355, :2377, :2389, :2433. Fires when a regulator's `input` is the battery net (existing topology) and no bulk capacitor or clamp is declared; a v9 file migrated with `hot_plug: []` is refused.
   - ControlPinUnwired: :2726 (`fn controls` at :2674). Each control pin of the part's parts-table row must be strapped or wired; a v9 regulator or translator with `controls: []` is refused if its row lists a pin.
   - BuckJunctionOverLimit: :2475 (buck with `junction: null`), :2484, :2501 (screen over limit). Depends on the regulator part (existing) and its load.
   The last three are partly "the new key must be filled in"; the first three judge existing content.
5. renumbering: doc comment "renumbered from 9 by the same rule" (8030fa5c:lib.rs:54). `hot_plug` first appears in HEAD's history at 8030fa5c under 10. Branch-only 24d6bc85 carried it under spec 9; not an ancestor of HEAD.
6. disagreements:
   - README (HEAD README.md:7): "Version 10 (`AET-WO-4300d3`) adds to `power`: `conditions.dc_bias_derating` ..., `input.hot_plug` ..., and on each regulator `junction` ... and `controls`, as on each crossing". Doc comment (8030fa5c:lib.rs:53-55): "Version 10 adds `power`'s judgement declarations ... `hot_plug`, `junction`, `controls` and `dc_bias_derating`." Code (8030fa5c:power.rs:102,133,195,197,299): every one is a required key, none has `default`. The 10 paragraph says "adds" while versions 9, 11, 12, 13 say "requires"; the closing sentence of the paragraph ("Every key is required, but `test_points` ... may be absent") makes it right by implication, since none of the v10 keys is in that exception list. The 10 wording alone would let a reader take them as optional.
   - Prose does not say that v10 adds refusals on existing content (see 4): nothing in README or doc comment says a v9 file can become invalid at 10 by SupplyFloorBelowBorRising alone.

---------------------------------------------------------------------

## Version 11 (4e821edb, AET-WO-4300g)

1. requires: `mcu.supply_source` (key required; `null` or `{connector, pin}`) (4e821edb:lib.rs:189, `Option::deserialize`).
2. optional: none.
3. replaced: none. removed: none.
4. rules:
   - NoProgrammingPath: 4e821edb:connectors.rs:455 (`programming_path`, called at lib.rs:1960). Refuses any file with no declared connector carrying `SWDIO`, `SWCLK`, `GND` and the MCU supply together. A v10 file with `connectors: []` (valid at 10) is refused at 11, and adding `supply_source` cannot cure it.
   - SupplyHasNoSource: connectors.rs:486/:497 (null `supply_source` and no regulator of `power` outputs `mcu.supply`). A v10 file with `power: null` hits this with `supply_source: null`.
   - SupplyHasTwoSources: connectors.rs:500. Only with a non-null `supply_source`, so it judges the new key.
   - Not a new kind, but new refusal of v10 content: `DEBUG_NETS = [SWDIO, SWCLK, NRST]` (lib.rs:70) seeds `folded_nets` (lib.rs:2381) and a domain check: a v10 net or domain named `swdio`/`SWCLK`/`nrst` in any case now gives `DuplicateNetName` / `NetShadowsDomain`.
5. renumbering: doc comment "renumbered from 9 by the same rule" (lib.rs:56-57). `supply_source` first appears in HEAD's history at 4e821edb under 11. Branch-only 4d9dd300 carried it under spec 9.
6. disagreements:
   - README (HEAD README.md:7): "Version 11 (`AET-WO-4300g`, renumbered from 9) requires `mcu.supply_source`: `{connector, pin}`, the declared connector pin the supply enters by, or `null` when a regulator of `power` outputs the supply." Code: the requirement that a connector carry SWDIO/SWCLK/GND/supply (NoProgrammingPath, connectors.rs:455) and the reserved debug names are not in that paragraph or in the lib.rs doc comment (lib.rs:56-58); they appear only in rule-table rows 11 and 17/18 (4e821edb:README.md:137,146; HEAD README.md:168,179). A reader of the version paragraph would think adding one key makes a v10 file valid.

---------------------------------------------------------------------

## Version 12 (1178ca85, AET-WO-4230)

1. requires: `acknowledge_unwitnessed` (top-level array, `[]` acknowledges none; element `{register, class, fields[], reason}`) (1178ca85:lib.rs:126); `serial_ports[].peer_deviation_ppm` (u32) (serial.rs:146) and `serial_ports[].peer_deviation_source` (`"cited"` | `"declared"`) (serial.rs:149). Conditional: the two serial keys only when a serial port exists.
2. optional: none.
3. replaced: none. removed: none.
4. rules:
   - Existing kind, changed meaning (`[solve]` SerialBaudTolerance, solve.rs:579, check at serial.rs:406 `check_silicon`): tolerance is now judged on quantization at HSI's worse extreme (HSI_FAST_PPM/HSI_SLOW_PPM, serial.rs:25,428) plus the declared peer deviation, instead of quantization alone. A v11 port that passed can fail at 12 for its baud and kernel clock alone.
   - `[solve]` SpiSpeedBelowKernelRange (solve.rs:657, `spi_controller` at :644) and SpiKernelClockUnavailable (:651): a v11 SPI bus whose `speed_hz` is below kernel-at-HSI-fastest / 256 (or whose peripheral has no kernel clock) is refused at solve. Content: `spi_buses[].speed_hz` and the assigned peripheral, no new key.
   - I2cTimingMismatch, SpiControllerMismatch: solved-topology consistency only (not spec content).
   - PeerDeviationBelowCited (serial.rs:653), PeerDeviationNotCited (:661), Unwitnessed* (lib.rs:2813 `validate_unwitnessed`): judge the new keys. (PeerDeviationBelowCited reads an existing `role == rc_crsf`, but the figure it compares is the new key.)
   - Behaviour change without a new kind: `electrical.rs` `sink_microamps` for Fast-mode Plus changed from 20 mA to 8 mA (4 mA below 2.7 V) and the floor is taken at the domain maximum, so existing I2C pull-up floors moved (1,022 vs 967 ohm at 100 kHz on a 3.465 V maximum).
   - Not in this commit (window only): UnknownPart, InvalidDeviceName, ReservedName (introduced 5a8892d6 / 315ce1b2, earlier).
5. renumbering: doc comment "renumbered from 10, then from 11, by the same rule" (1178ca85:lib.rs:60-65). `acknowledge_unwitnessed` and `peer_deviation_ppm` first appear in HEAD's history at 1178ca85 under 12. Branch-only 7601a9de carried them under spec 11 and f1a8f09a under spec 10; neither is an ancestor of HEAD.
6. disagreements: none about keys (README HEAD :7 "requires `peer_deviation_ppm` and `peer_deviation_source` on every serial port, and `acknowledge_unwitnessed`" matches code). Omission: neither prose names the tightened SerialBaudTolerance or the new SPI kernel/MBR refusals as changes of this version.

---------------------------------------------------------------------

## Version 13 (70d1621d, AET-WO-4220b)

1. requires: `i2c_devices[].pin_capacitance` (70d1621d:lib.rs:345) and `spi_devices[].pin_capacitance` (lib.rs:358); key required, value `null` or `{declared:{femtofarads, condition, source}}` (`Option::deserialize`). Conditional: only when a device element exists.
2. optional: none.
3. replaced: none. removed: none.
4. rules in the bump: PinCapacitanceOutOfRange (lib.rs:2763), PinCapacitanceBelowI2cCi, PinCapacitanceAlreadyCited. All judge a declaration, i.e. the new key; none fires on a file whose devices carry `null`. (UnboundedPinCapacitance is `extract`'s and predates v13: 50d7a6f2.)
   Later tightening at the SAME version, no bump: PinCapacitanceBelowTypical added by 2b436729 (lib.rs:1740, check at :2801-2804; README line changed in the same commit), on the v13 key. A v13 file valid at 70d1621d can be refused at 2b436729 and HEAD if its declared figure is below the row's cited typical. 2b436729 is on HEAD's history, after 70d1621d, before 9a37ae6e.
5. renumbering: no "renumbered" note, and none needed on main: `pin_capacitance` first appears at 70d1621d under 13. Branch-only 2cea05dc also carried it under 13.
6. disagreements: none about keys. README HEAD :7 "requires `pin_capacitance` on every I2C and SPI device (below)" matches. Gap: the PinCapacitanceBelowTypical tightening at an unchanged version is described in README (2b436729 edit) but not in the lib.rs version comment (HEAD lib.rs: "Version 13 requires `pin_capacitance` ... `null` or a declaration").

---------------------------------------------------------------------

## Version 14 (9a37ae6e, AET-WO-1312)

1. requires: none.
2. optional: `test_points` (top-level array; `serde(default, skip_serializing_if = Vec::is_empty)`, 9a37ae6e:lib.rs:144; element `{net, reference}`, both required); `mcu.reset_switch` (bool; `serde(default, skip_serializing_if = Not::not)`, lib.rs:295). An explicit `null` for either is refused (README HEAD :7; neither type is an `Option`, so serde rejects null).
3. replaced: none. removed: none.
4. rules:
   - New kinds TestPointNetUnknown, TestPointReferenceTaken, TestPointNetRepeated (bench.rs:149): judge the new key only. PinCapacitanceBelowTypical is NOT in this commit (it is 2b436729, see v13).
   - Not a new kind, but new refusal of v13 content: `DEBUG_NETS` grows to `[SWDIO, SWCLK, NRST, BOOT0]` (lib.rs:89; used at lib.rs:3136 `folded_nets`), so a v13 signal net or voltage domain named `BOOT0` (any case) is now `DuplicateNetName` / `NetShadowsDomain`. Also `names.rs:259`: a reference spelled `sw1`/`SW01` collides (EmitterReference), but only when `reset_switch` is true (new key).
5. renumbering: doc comment "renumbered from 13 by the same rule, merging after `4220b`" (lib.rs:69-70); `test_points` and `reset_switch` first appear on HEAD's history at 9a37ae6e under 14. Branch-only cb17efdb carried them under spec 12 (not an ancestor of HEAD). Version 13 on main belongs to 4220b (70d1621d) and carried neither key.
6. disagreements:
   - README rule 11 (HEAD README.md:168): "The MCU's debug nets `SWDIO`, `SWCLK` and `NRST` are the engine's (`AET-WO-4300g`): a signal net spelling one in any case is a duplicate, and a domain spelling one shadows it". Code (9a37ae6e:lib.rs:89, HEAD lib.rs:136): `pub const DEBUG_NETS: [&str; 4] = [SWDIO, SWCLK, NRST, BOOT0];`, seeded at HEAD lib.rs:3773. The README omits `BOOT0`; the v14 paragraph and the lib.rs v14 comment do not mention it either. README rule 15a (HEAD :177) does name BOOT0 as a nameable net, which is a different statement.

---------------------------------------------------------------------

## Version 15 (f845b6fd, AET-WO-1313)

1. requires: none.
2. optional: `board.max_outline_um` (`{width, height}` u32, 1 to 1,000,000) (f845b6fd:board.rs:80, `serde(default, deserialize_with = "present", skip_serializing_if = Option::is_none)`); explicit `null` refused (`present` is `T::deserialize`, board.rs diff).
3. replaced: none. removed: none.
4. rules: only OutlineBound (board.rs:351), on the new key. OutlineTooSmall / OutlineUnreachable are not in the IR crate. None on N-1 content (witnesses.txt:107 lists LoadAboveSpeedTable, PinSpeedMismatch, etc. in the window; they are not in this diff).
5. renumbering: none claimed; `max_outline_um` first appears at f845b6fd under 15.
6. disagreements: none found. README HEAD :7 (optional by the PR #191 Q1 precedent, not written back, `null` refused) matches board.rs.

---------------------------------------------------------------------

## HEAD parser vs removed/replaced keys in 9..15
Nothing was removed or replaced in 9..15, so HEAD cannot still accept such a key. (Outside the range: v20 replaced `mcu.hse_hz` by `mcu.hse`; HEAD README.md:7 says `hse_hz: 0` reads as `null` and any other `hse_hz`, or `hse_hz` beside `hse`, is refused. Not verified in code here.)

## Summary of disagreements to act on
1. v10: "adds" vs required keys (README :7, lib.rs v10 comment) vs power.rs:102/133/195/197/299.
2. v11: version paragraph silent on NoProgrammingPath (connectors.rs:455) and reserved debug names.
3. v14: README :168 lists SWDIO/SWCLK/NRST; code `DEBUG_NETS` also holds BOOT0 (lib.rs:136).
4. v13: PinCapacitanceBelowTypical added at the same version (2b436729), not in the lib.rs comment.
5. v10/v12: content-affecting refusals (SerialBaudTolerance change, SpiSpeedBelowKernelRange, SupplyFloorBelowBorRising and others) reach N-1 files with no prose.
