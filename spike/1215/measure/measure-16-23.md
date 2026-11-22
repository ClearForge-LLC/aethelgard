# Specification schema history, versions 16 to 23 (read-only measurement)

Method: `git show <bump>` diffs of `crates/aethelgard-ir/src` and fixtures, the HEAD tree (ddfa64d6), README lines 5-7 and the doc comment above `SPECIFICATION_SCHEMA_VERSION` (lib.rs:40-121 at HEAD). Nothing was built or run: every claim is from reading code, and claims that depend on serde semantics are marked (read).
"HEAD" means commit ddfa64d6 (read with `git show`), NOT the working tree, which had uncommitted edits to lib.rs and main.rs while I worked; all HEAD line numbers were re-verified against the commit.
Citation form: `commit:file:line`. `lib.rs` = `crates/aethelgard-ir/src/lib.rs` unless a path is given.

## 0. Findings that cut across versions (read these first)

**F-A. The version-16 key set changed twice with no bump.** At `SPECIFICATION_SCHEMA_VERSION = 16` (acae53f7:lib.rs:81 and 565b32c5:lib.rs:81) two work orders added OPTIONAL keys to the specification, so "a version-16 file" is not one shape:
- `acae53f7` (AET-WO-1303b, 2026-11-18, 16 -> still 16): `board.stackup.outer_copper_min_um`, `plane_copper_min_um`, `plating_min_um`, `copper_temperature_max_c`, `fab_tolerances.{hole_position_um, hole_size_plus_um, hole_size_minus_um, track_width_permille}`, `board.stackup.dielectrics[].max_thickness_um` (acae53f7:board.rs diff), `power.drop_limits` and `power.loads[].reference` (acae53f7:power.rs diff). All `#[serde(default, deserialize_with = "present")]`, so absent is legal and an explicit `null` is refused.
- `565b32c5` (AET-WO-1314, 2026-11-19, still 16): `board.stackup.fab_tolerances.via_ring_um`.
- HEAD locations: board.rs:156,162,168,174,184,196,227,255; power.rs:128,321.
- The README version paragraph names none of them and no version "adds" them (see D1).

**F-B. Validate rules landed without a bump, on content the previous version already admitted.**
- `2fda6e13` (AET-WO-1209, between 18 and 19, const still 18 at 2fda6e13:lib.rs:89): `LoadNamesNoPart`, names.rs:338, called from lib.rs:2752 (`names::loads`). Refuses a `power.loads[].reference` naming no part. `reference` is the optional key of F-A, so a v16/v17/v18 file could hold one.
- `fff49a49` (AET-WO-1209c, between 19 and 20, const 19): `LoadNamesCountedPart`, names.rs:353. Refuses a load whose `reference` is the MCU or a declared regulator.
- `adb8f61a` (AET-WO-1213, between 22 and 23, const 22): `ConsoleBaudNotRoundTrip`, serial.rs:730. Refuses a console serial port whose `framing.baud` ArduPilot's `SERIAL0_BAUD` cannot round-trip. A v22 file can hold any console baud. Its commit subject says "no committed specification refused".
- None of these appears in lines 5-7 or in the doc comment; they are in the README step table only.

**F-C. HEAD parser, `mcu.hse` / `mcu.hse_hz` (lib.rs:1007-1080 at HEAD; `McuWire` 1064-1080).** The match is `match (w.hse, w.hse_hz)` at lib.rs:1029.

| input | result | source |
|---|---|---|
| `hse` present (null, `{"crystal":..}` or `{"external":..}`), no `hse_hz` | accepted, value as written | lib.rs:1030 |
| `hse_hz: 0`, no `hse` | accepted, read as `hse: null` | lib.rs:1031 |
| `hse_hz: N`, N != 0, no `hse` | refused, serde custom error "mcu.hse_hz is N: a crystal frequency was always refused there; declare `mcu.hse`"; CLI shows it as `SchemaViolation`, exit 2 | lib.rs:1032-1036; CLI main.rs:2813-2820 |
| both keys, any values (including `hse_hz: 0` beside `hse: null`) | refused, "duplicate field `hse` (`hse_hz` is the version-19 spelling of `hse`)" | lib.rs:1038-1041 |
| neither | refused, `missing field hse` | lib.rs:1037 |

Also (read, serde semantics): `hse_hz` is `Option<u32>` with `#[serde(default)]` (lib.rs:1070-1071), so an explicit `hse_hz: null` is the same as absent. That makes `hse` + `hse_hz: null` accepted and `hse_hz: null` alone "missing field hse". A negative, fractional or > u32 `hse_hz` fails as a serde type error before the match. `hse` uses `present` (lib.rs:1068), so `"hse": null` is a present value (Some(None)), which is why it conflicts with `hse_hz`.
- The parser is not version-aware. The only version gate before it is the CLI's pre-parse in `parse_spec` (cli main.rs:2801-2812): a file stamped other than 23 is refused `UnsupportedSchemaVersion` before shape is judged. So the `hse_hz: 0` leniency is reachable only by a file stamped 23. A library caller of `Specification::deserialize` reaches it at any stamp and gets `UnsupportedSchemaVersion` from `validate` step 1 (lib.rs:3570 at HEAD).
- Serialization never writes `hse_hz` (`Serialize for Mcu`, lib.rs:1005-1009); write-back asserts `hse: null` (cli tests campaign_tier1.rs:1839-1842, ir tests bench.rs:57-61, validate.rs:864-867). Tests for 0, N and both: ir tests/motor.rs:178-195.
- Count at HEAD (git ls-files, `schema_version` integer 23, `mcu` object, outside `spike/`): **33 specification files**, of which **31 carry `mcu.hse_hz` (all 0)** and 2 carry `mcu.hse` (cli fixtures hse-crystal-16.json, hse-external-8.json). None carries both. I get 31, not 36: the other `hse_hz` hits are 5 topology files (solved `clock.hse_hz`, a different thing), 7 `spike/4600/out/*/solve.json`, and spike specs stamped 16, 17, 19 (not 23). The 2 `spike/1211/fixtures` files are stamped the string "22-or-23" and are not valid specifications.

**F-D. Renumbering** (item 6 per version below). All eight bumps are `N-1 -> N` in the bump commit's own diff, and every key was added under the final number on main. The work-order files in `docs/work-orders/` still carry the pre-renumber numbers (4240v: "18", 1210: "20", 1211a/4240b: "22 or 23"); this is stale prose, not a code defect.

---

## Version 16 (0829fe83, AET-WO-4400d)

1. requires: none. (`parts.json` rows gained required `axes` and `ardupilot.frame`, but that is the parts table, parts schema 9, 0829fe83:parts.rs diff, not the specification.)
2. optional: `board.forward` (`top|right|bottom|left`), 0829fe83:board.rs:89; `spi_devices[].orientation` `{x, z}`, each `forward|aft|right|left|up|down`, 0829fe83:lib.rs:395 (SPI devices only, not I2C). Both `default` + `board::present`, so an explicit `null` is refused (0829fe83:board.rs diff, lib.rs diff).
3. replaced / removed: none.
4. rules: all new kinds are keyed to the new key, so a v15 file cannot hit them.
   - `OrientationOnNonImu` (lib.rs:2829) and `ImuAxesNotCited` (lib.rs:2842) in `validate_parts`, only for a device that declares `orientation`.
   - `ImuAxesNotPerpendicular` (lib.rs:3357) and `ForwardUndeclared` (lib.rs:3364), validate step 13, only per device with `orientation`. `ForwardUndeclared` is a cross-key requirement: `orientation` needs `board.forward`.
   - `ImuAxesNotPerpendicular` / `OrientationOnNonImu` are repeated in `SolvedTopology::validate` (lib.rs:4183).
5. solved: `SOLVED_SCHEMA_VERSION` 9 -> 10, `SolvedSpiDevice.orientation` copied by `solve` (0829fe83:lib.rs:760). Parts schema 8 -> 9.
6. renumbering: const 15 -> 16 at 0829fe83:lib.rs:81; WO says "after 1313 merges (specification 15)". Confirmed.
7. disagreements: see D2.

## Version 17 (ee835bde, AET-WO-4400g)

1. requires: none (the whole subtree is optional). Inside a declared `mcu.foreign_clock`: `apb1_timers_hz{value,source}`, `apb2_timers_hz{value,source}`, `usarts` (a map with exactly the 8 keys `USART1 USART2 USART3 UART4 UART5 UART7 UART8 USART6`, each `{value, source}`), ee835bde:motor.rs `ForeignClock` (deny_unknown_fields; no defaults). Conditional on `foreign_clock` being declared.
2. optional: `mcu.foreign_clock` (`default` + `board::present`, ee835bde:lib.rs:314; null refused).
3. replaced / removed: none.
4. rules (all gated on `foreign_clock` present, lib.rs:3225, so not on N-1 content):
   - `ForeignClockZero` (lib.rs:970), `ForeignClockUsarts` (lib.rs:979: missing or unknown USART), `ForeignClockMuxSplit` (lib.rs:995: USART1 must equal USART6, and USART2/3/UART4/5/7/8 must be equal).
   - The mux-split and no-zero requirements are not in the README prose (see D3).
5. solved: 10 -> 11, `SolvedTopology.foreign_clock` (`null` or a clock, required in the current version), ee835bde:lib.rs:459, 550.
6. renumbering: 16 -> 17 at ee835bde:lib.rs:84. Confirmed (4400g WO line 49 says "from 16 to 17").
7. disagreements: D3 only (prose omits two constraints).

## Version 18 (36a494fd, AET-WO-2304b)

1. requires: `power.battery.series_milliohms` (required key, `null` or `{value, source}`; `value` 0 refused as `InvalidSource`-class `invalid(...)`, power.rs:738), 36a494fd:power.rs:214. Conditional on a non-null `power`. `motor_outputs[].esc` (required key on EVERY motor output, PWM and DShot alike; `null` or `{"none":{"source"}}`), 36a494fd:motor.rs:92. For the `esc.none.source` string, `board::source` check at lib.rs:3639.
2. optional: none.
3. replaced / removed: none.
4. rules:
   - New kind `BatterySideUnmodelled` (power.rs:395, raised at power.rs:3169 in `battery_side_against`). It is NOT called by `validate`: its only caller is the CLI's campaign (cli campaign.rs:762), i.e. `simulate`. It can refuse content a v17 file could hold (a battery-fed regulator that is not a buck, has no `junction`, or whose row states no regulation input), but only at simulate time and only for a file already moved to 18.
   - `SeriesResistanceUndeclared` / `EscUndeclared` are CLI campaign halts (campaign.rs:773, 780) on the new `null`s, not IR kinds.
   - Rule that landed in this window without a bump: `LoadNamesNoPart` (F-B).
5. solved: none (`SOLVED_SCHEMA_VERSION` stays 11, 36a494fd:lib.rs:464).
6. renumbering: 17 -> 18 in the diff. Confirmed (2304b WO line 126: "After 4400g merges, main is at specification 17").
7. disagreements: none material; README says "with a non-zero value" and the code enforces it (power.rs:738).

## Version 19 (943f7112, AET-WO-4240v, renumbered from 18)

1. requires: `mcu.core_supply` `{mode: ldo|bypass, source}` (unconditional required key, 943f7112:lib.rs:309; `CoreSupply` deny_unknown_fields, no defaults). `mcu.foreign_clock.vos` `{value: scale0..scale3, source}` (required, conditional on a declared `foreign_clock`), 943f7112:motor.rs:496.
2. optional: none.
3. replaced / removed: none.
4. rules:
   - `VosLimitExceeded` (lib.rs:4252 area) and `FlashWaitStatesBelowTable` are in `SolvedTopology::validate` (the `impl SolvedTopology` at 943f7112:lib.rs:3938), not in `Specification::validate` (`impl Specification` at 2761). They judge the solved clock. `solve` can only produce the reset clock at Scale 2, which passes, so a specification alone cannot trigger them; only a forged or edited topology can.
   - Spec-side: `InvalidSource` on `mcu.core_supply.source` (lib.rs:3309) and `mcu.foreign_clock.vos.source` (lib.rs:3318): new keys only.
   - A foreign clock over its VOS maximum is "reported, never refused" (motor.rs doc, 943f7112:motor.rs:489-495).
   - Rule that landed in this window without a bump: `LoadNamesNoPart` (F-B), `2fda6e13`, merged before this bump.
5. solved: 11 -> 12: `SolvedTopology.core_supply`, `SolvedClock.vos`, `SolvedForeignClock.vos` (943f7112:lib.rs:496, 569). A solved file lacking `clock` or `core_supply` is now a parse error for any version (lib.rs:1092-1100 comment), not only the current one.
6. renumbering: 18 -> 19 in the diff. Confirmed. WO 4240v line 116 says "goes to **18**" (written before 2304b took 18); stale WO text.
7. disagreements: D4.

## Version 20 (14bb1e5b, AET-WO-4240a, renumbered from 19)

1. requires: `mcu.hse` is a required key (`null` | `{"crystal":{part, load_caps_femtofarads:[CL1,CL2], pull_ppm, temperature_celsius:{min,max}, life_hours}}` | `{"external":{hz_num, hz_den, ppm, source}}`), 14bb1e5b:lib.rs:313, clock.rs `Hse`/`HseCrystal`/`HseExternal` (deny_unknown_fields, no defaults). The crystal/external keys are conditional on `hse` being non-null. **But the requirement is soft**: `hse_hz: 0` stands in for `hse: null` (replaced section, F-C). So a v19 file needs only its version line changed.
2. optional: none (`hse_hz` stays a parse-time alternative spelling, not an optional key).
3. replaced: `mcu.hse_hz` (u32) -> `mcu.hse`. Removed: nothing else. `hse_hz: 0` is still read; any other value refused (F-C table, 14bb1e5b:lib.rs:972-989).
4. rules (14bb1e5b:clock.rs): `HsePartNotCrystal`, `HseLoadCapsUnfit`, `HseTemperatureOutsidePart`, `HseLifeBeyondAging`, `HseDeclarationZero` (clock.rs:396), `HseFrequencyOutOfRange`, `HseGmCritExceeded`, `HseBudgetUnbounded`, `HseTreeUnreachable`, `HseClockMismatch`, `Pll*`. All act on a declared crystal/external. `clock::judge_tree` returns `Ok(None)` for `hse: null` (clock.rs:556-559), and `declared_budget` is not run for null, so **no new kind can refuse a v19 file** (a v19 file is by construction hse null). `CrystalNotSupported` was removed (replaced by `Hse(..)`). `NoI2cTiming` is a `SolveError`, replacing an `.expect` panic (14bb1e5b:solve.rs:152, 944), unreachable on the HSI clock a v19 file solves to.
   - Rule that landed without a bump in this window: `LoadNamesCountedPart` (F-B), `fff49a49`, merged before this bump.
5. solved: 12 -> 13: `SolvedClock.hse` (absent for `mcu.hse: null`, so such a topology differs only in its version), 14bb1e5b:lib.rs:507.
6. renumbering: 19 -> 20 in the diff. Confirmed (4240a WO line 90: "Whoever merges first takes 20").
7. disagreements: none material (README says exactly what the parser does: README line 7 "`hse_hz: 0`, the only value version 19 admitted, reads as `null`... any other `hse_hz`, or `hse_hz` beside `hse`, is refused at parsing" matches lib.rs:1029-1041). One thing the prose does not say: the same leniency still applies at 23 and 31 committed files use it (F-C).

## Version 21 (63334d69, AET-WO-1210, renumbered from 20)

1. requires: none.
2. optional: `board.drawn` (bool; absent = `true`, `true` is not written back; explicit `null` refused because it is a plain `bool` with `serde(default = "drawn")`), 63334d69:board.rs:98.
3. replaced / removed: none.
4. rules (the important one, answering the brief):
   - **Yes, 1210 refused content a version-20 file could already hold**, because "absent" now means "drawn", so every pre-existing file is held to the new steps unless it declares `drawn: false`. Two kinds:
     - `UnboundedPinCapacitance`: validate step 23 (`drawn::pin_capacitance`, drawn.rs:21, raise at drawn.rs:37, called at 63334d69:lib.rs:2959). Refuses an SPI device on an SPI bus (no `power.crossings` entry carrying that bus) whose part row has `pin_capacitance_femtofarads: null` and whose `spi_devices[].pin_capacitance` is `null`. A v20 file can hold exactly that (`pin_capacitance` has been a required-but-nullable key since v13). I2C devices never refuse. Before 1210 the same fact was refused only later by `aethelgard_eda::extract` (63334d69^:extract.rs:1076), not by `validate`.
     - `PartNotDrawable`: NOT in `Specification::validate`. It is `Specification::drawable_parts(arms)` (drawn.rs:56, raise at drawn.rs:71), called by the CLI's `validate` only (cli main.rs:2585), with the emitter's arm list. Refuses any I2C or SPI device whose part has a parts row but no emitter arm. Before 1210 `emit` refused it as `UnknownPart` (63334d69^:schematic.rs:1273).
   - Measured by the WO itself: of 27 committed specs, rules 1 and 3 refuse exactly `imu-baro-1v8.json` (U5/U7 ICM-20948 on uncrossed bus `IMU`, no pin capacitance anywhere; U6 ICP-10111 with no arm), which was given `"drawn": false` (63334d69:docs/work-orders/AET-WO-1210.FEEDBACK.md lines 56, 60). All other committed files moved by version line only.
   - A `drawn: false` file skips both rules (drawn.rs:22, 57).
   - Window note: this WO's "rules 1 and 3" are 1209's; 1209 itself landed only rule 2 (`LoadNamesNoPart`, F-B).
5. solved: none (`SOLVED_SCHEMA_VERSION` stays 13, 63334d69:lib.rs:512).
6. renumbering: 20 -> 21 in the diff. Confirmed. 1210 WO line 6 says "Takes specification version 20 (4240v took 19)" (stale; 4240a took 20 first).
7. disagreements: none. README says "absent, or `true`, is a board drawn from the specification, which `validate` holds to step 23 and the CLI's drawable-parts rule" (README line 7), which is what the code does; it does not say outright that this newly refuses v20 content, but it implies it. (D6: README line 3 "Nothing defaults" vs `board.drawn` default true.)

## Version 22 (25203c58, AET-WO-1211a, renumbered from 21)

1. requires: `motor_outputs[].pwm.idle` (`{"level":"low|high"}` xor `{"pulse_us":N}`, exactly one key) and `motor_outputs[].pwm.resolution_ps` (u64), 25203c58:motor.rs:79,84, no serde default. **Conditional on a PWM output** (`pwm` non-null); DShot outputs carry `pwm: null`.
2. optional: `pin_devices` (array; absent or empty = none; not written back; `null` refused since the type is `Vec`), 25203c58:lib.rs:193. Everything inside a declared pin device is new and governed by `pin_devices.rs` (analog, bridges, signals, straps, supplies, part_limits, temperature_celsius...); a pin-device key is required or optional by that file, not enumerated here.
3. replaced / removed: none (the old implicit idle, `pulse_min_us`, becomes explicit `idle: {"pulse_us": pulse_min_us}`, and the implicit 1 us tick becomes `resolution_ps: 1000000`).
4. rules:
   - PWM kinds `IdleSpelledTwice` (motor.rs:891; lib.rs:4015 for `pulse_us: 0`), `ResolutionNotDerivable`, `PulseNotOnTick`, `IdleNotOnTick`, `IdleExceedsPeriod` (motor.rs `PwmRefusal` 746): all read the new `idle`/`resolution_ps`. With the restating values (`{"pulse_us": pulse_min_us}`, 1,000,000) none can fire on a v21 file: every whole microsecond is on a 1 us tick, a pulse equal to the period or 0 was already refused as `RateOutOfRange` (`ccr == 0 || ccr + 1 >= ticks` before; `min == 0 || min + 1 >= ticks` after, motor.rs diff), and `ResolutionNotDerivable` at 1,000,000 ps is the old `NoMicrosecondTick` condition.
   - Pin-device kinds (`PartHasBus`, `PartWithoutBus`, `BridgeUndeclared`, `Adc*`, `Supply*`, `Strap*`, ... ) all run from `pin_devices::validate` (lib.rs:3072) over declared devices, or from `part_without_bus` (a bus device naming a busless row), which a v21 file cannot hold since the busless rows are new in `parts.json`. `SenseAdcMismatch` / `PinDeviceUndeclared` / `SignalShape` are `SolvedTopology::validate` (`validate_solved`, lib.rs:4328) rules.
   - `TimerResolutionConflict` is a `SolveError` (solve.rs:140), on `resolution_ps`.
   - Net/reference uniqueness now also chains `pin_devices::nets/references` (lib.rs diff, names.rs:122): empty for v21 files.
   - So **no new rule refuses content a restated v21 file holds**.
5. solved: 13 -> 14 (25203c58:lib.rs:532): `gpio` entries become `{pin, direction, initial|pull}` (a bare pin string is refused by version only), new required `adc_inputs`, `motor_outputs[].pwm.{idle,resolution_ps}`, `pin_devices` (absent when none). Parts schema -> 11.
6. renumbering: 21 -> 22 in the diff. Confirmed (1211a WO line 7: "Whoever merges first takes 22; the second ... renumbers to 23").
7. disagreements: none (README line 7 and lib.rs:108-114 agree with the code; I checked the "8 PWM outputs" claim: at 25203c58 the 31 specs hold 16 PWM outputs, 8 restating, 8 new in the two driver fixtures at 22).

## Version 23 (ddfa64d6, AET-WO-4240b, renumbered from 22)

1. requires: `mcu.hse.crystal.startup_max_us` (u32) and `mcu.hse.crystal.startup_source` (string); `mcu.hse.external.startup_max_us` and `mcu.hse.external.startup_source`. ddfa64d6:clock.rs:112,133. Conditional on a non-null `hse`; a `null` HSE carries neither. A v22 file with `hse_hz: 0`/`hse: null` moved by version line only (31 files did).
2. optional: none.
3. replaced / removed: none.
4. rules:
   - `HseDeclarationZero` (field `startup_max_us`, clock.rs:424) and `InvalidSource` for `startup_source` (clock.rs:429, path text `mcu.hse.startup_source`, which omits the `crystal`/`external` step), both inside `declared_budget`, only for a declared HSE. New keys only.
   - **`HseCopyNotSolved` judges a topology, not a specification.** It is raised at ddfa64d6:lib.rs:4590 inside `impl SolvedTopology` (starts lib.rs:4227; `impl Specification` is 3021-4226), clock.rs:300. It re-solves the topology's own copy of its HSE declaration (`solve::hse_clock(&h.declared)`) and refuses a topology whose `clock` differs. A specification cannot reach it; only an edited topology file can. README line 7 puts it inside the "version 23" paragraph, worded "A topology whose clock is not the one its own HSE declaration solves to".
   - Not a new kind but a behavior change on v22 content: for an HSE board, SPI controller kernels are now `pll1_q` (SPI1-3) and `pclk` (SPI4-6) at the HSE budget, no longer `per_ck`/`hsi_ker_ck` at the HSI budget (solve.rs `spi_kernel`/`kernel_budget`). A v22 HSE file with SPI devices can therefore solve to a different MBR or refuse (`SpiSpeedBelowKernelRange`, a `SolveError`) where it did not. v22 HSE files could not be generated, so no firmware is affected.
   - Rule that landed in this window without a bump: `ConsoleBaudNotRoundTrip` (F-B).
5. solved: 14 -> 15 (ddfa64d6:lib.rs:541): `SolvedHse.flash_acr`, the declaration's `startup_*` copied, SPI kernel kinds `pll1_q`/`pclk` (lib.rs doc). `FlashWaitStatesBelowTable` message widened (not a new kind).
6. renumbering: 22 -> 23 in the diff. Confirmed (4240b WO: "it is 22 if 1210 has merged"; 1211a took 22).
7. disagreements: D5.

---

## DISAGREEMENTS (README prose / doc comment vs code)

**D1. Optional keys the prose does not know (the most consequential).**
- README line 7: "Every key is required, but `test_points` and `mcu.reset_switch` (version 14), `pin_devices` (version 22), `board.max_outline_um` (version 15), `board.forward` and an SPI device's `orientation` (version 16), and `mcu.foreign_clock` (version 17), and `board.drawn` (version 21)."
- Code: eight more keys are optional at HEAD, none in that sentence and none assigned to any version: `board.stackup.outer_copper_min_um` (board.rs:156), `plane_copper_min_um` (162), `plating_min_um` (168), `copper_temperature_max_c` (174), `fab_tolerances` (184) with `hole_position_um` (196), `hole_size_plus_um`, `hole_size_minus_um`, `track_width_permille`, `via_ring_um` (227), `board.stackup.dielectrics[].max_thickness_um` (255), `power.drop_limits` (power.rs:128), `power.loads[].reference` (power.rs:321). They exist in the README step-13/16 table text, not the version paragraph. They were added at version 16 (`acae53f7`, `565b32c5`) and 17 never mentions them. The lib.rs doc comment (40-121) omits them too.
- Consequence: "version 16" is ambiguous, and the sentence is false as written.

**D2. Version 16: cross-key requirement missing from the prose.**
- README line 7: "Both keys are optional by the same precedent and an explicit `null` is refused."
- Code: `spi_devices[].orientation` is refused without `board.forward` (`ForwardUndeclared`, 0829fe83:lib.rs:3364). The prose says "refused by `generate hwdef` (`ImuOrientationUndeclared`)" for the opposite case only. The kinds `ForwardUndeclared`, `ImuAxesNotPerpendicular`, `OrientationOnNonImu`, `ImuAxesNotCited` do appear in the step table.

**D3. Version 17: two constraints the prose omits.**
- README line 7: "`usarts`, one entry per USART the platform carries, keyed `USART1`, `USART2`, `USART3`, `UART4`, `UART5`, `USART6`, `UART7` and `UART8`."
- Code: also no field may be 0 (`ForeignClockZero`, ee835bde:lib.rs:970) and USART1 must equal USART6 and the six others must be equal (`ForeignClockMuxSplit`, 995). Both kinds are in the step table (README mentions each twice) but not in the paragraph.

**D4. Versions 19 and 20: "cited" is not enforced.**
- README line 7 (v19): "`source` where the mode was read (1 to 200 bytes of printable ASCII, cited with a document number and its revision)"; lib.rs:CoreSupply doc: "A source is cited only with a document number and its revision (F-CITE strict ...)".
- Code: `board::source` (board.rs:592-603) checks only length 1..=200, bytes 0x20..=0x7E, no leading or trailing space. clock.rs says so outright for external.source ("`validate` checks the text's form only ... whether it names a document is the reviewer's to judge"). The README parenthetical reads as a requirement the engine does not enforce.

**D5. Version 23: a topology rule inside a specification paragraph.**
- README line 7: "A topology whose clock is not the one its own HSE declaration solves to is refused (`HseCopyNotSolved`, `4240a` F3)."
- Code: it is a `SolvedTopology::validate` rule (ddfa64d6:lib.rs:4590) and no specification, however edited, can trigger it. The paragraph heading is `Specification`. Not wrong, but it is not a requirement on a specification author, and the same sentence does not say that `startup_source`'s error path reads `mcu.hse.startup_source` (clock.rs:429) rather than `mcu.hse.crystal.startup_source`.

**D6. Version 21 default vs README line 3.**
- README line 3: "Every type refuses unknown fields and accepts only a JSON object. Nothing defaults (`INV-06`)."
- Code: `board.drawn` absent is `true` (board.rs:97 `serde(default = "drawn")`), `mcu.reset_switch` absent is `false` (lib.rs:354), and `hse_hz: 0` is read as `hse: null`. Each is documented in line 7, so this is an overstatement in line 3, not a hidden behavior.

**D7. Version 20's "required" key.**
- README line 7: "replaces `mcu.hse_hz` with `mcu.hse`, a required key".
- Code: `mcu.hse` is required only in the sense that one of `hse` or `hse_hz: 0` must be present; the same paragraph says `hse_hz: 0` "reads as `null`". The two sentences agree; but "`hse` is required" is not enforced and 31 of 33 version-23 specification files lack it.

No other disagreement found for 16-19, 21 and 22: each prose claim I checked matches the code at the bump commit and at HEAD.
