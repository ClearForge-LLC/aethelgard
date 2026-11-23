# AET-WO-4300i mutants

One mutant per guard this work order adds, sized to the diff (CLAUDE.md). `spike/4300i/mutants.py` applies each edit, runs the named filter of `cargo test -p aethelgard-ir --test offboard`, records the result, and restores the file with `git checkout`. It ran on the committed tree (`1989af79`, the adversarial-pass fixes), with no other change in any file it touches. The raw results are in `out/mutants.tsv`.

**Result: 43/43 killed, 0 survived.** The whole IR set was re-run after the fixes: 29 of the first 37 (M8 re-anchored on `at`, the claimants at a connector), and A1 to A14, one per guard the fixes add or change.

**Retired (8):** M13 (`ft_first`'s partition) and M26 to M32 (its call sites: motor channels, serial TX, serial RX, bus line, chip select, ADC input, device signal). The adversarial pass's F1 fix offers a connector-facing net FT pins only, so the partition had nothing left to order and was removed with its call sites; A1 to A8 mutate the filter that replaced it in each of those branches.

| id | guard | file | mutant (exact edit) | killing test | result |
|---|---|---|---|---|---|
| M1 | MotorPullNotDown | `src/offboard.rs` | `if r.to != crate::GND {` -> `if false && r.to != crate::GND {` | `offboard::a_pull_that_is_not_to_gnd` | killed |
| M2 | PowerPartUnknown: a motor pull's part is a resistor row | `src/offboard.rs` | `Some(power_parts::Kind::Resistor(_))` -> `Some(_)` | `offboard::a_pull_whose_part_is_not_a_resistor_row` | killed |
| M3 | MotorPullOutsideRpd, above max | `src/offboard.rs` | `if ohms < rpd.min \|\| ohms > rpd.max {` -> `if ohms < rpd.min {` | `offboard::a_pull_outside_rpd` | killed |
| M4 | MotorPullOutsideRpd, min is admitted (30000) | `src/offboard.rs` | `if ohms < rpd.min \|\| ohms > rpd.max {` -> `if ohms <= rpd.min \|\| ohms > rpd.max {` | `offboard::a_pull_outside_rpd` | killed |
| M5 | MotorPullOutsideRpd, max is admitted (50000) | `src/offboard.rs` | `if ohms < rpd.min \|\| ohms > rpd.max {` -> `if ohms < rpd.min \|\| ohms >= rpd.max {` | `offboard::a_pull_outside_rpd` | killed |
| M6 | ReservedPinTwice | `src/offboard.rs` | `if !seen.insert(r.pin.as_str()) {` -> `if false && !seen.insert(r.pin.as_str()) {` | `offboard::a_pin_reserved_twice` | killed |
| M7 | ReservedPinEngineReserved | `src/offboard.rs` | `if engine.contains(&r.pin.as_str()) {` -> `if false && engine.contains(&r.pin.as_str()) {` | `offboard::a_pin_the_engine_reserves` | killed |
| M8 | PowerPartUnknown at mcu.part, only where the row is needed | `src/offboard.rs` | `if spec.motor_outputs.is_empty() && at.is_empty() {` -> `if spec.motor_outputs.is_empty() \|\| at.is_empty() {` | `offboard::an_mcu_with_no_row` | killed |
| M9 | ConnectorPinNotFt | `src/offboard.rs` | `if !is_ft(structure) {` -> `if false && !is_ft(structure) {` | `offboard::a_hand_tt_pin_at_a_connector` | killed |
| M10 | McuPinStructureUnheld at a connector | `src/offboard.rs` | `let Some((structure, at)) = io.pins.get(pin) else {` -> `let Some((structure, at)) = io.pins.get(pin).or_else(\|\| io.pins.get("PA0")) else {` | `offboard::a_hand_pin_at_a_connector_with_no_held_structure` | killed |
| M11 | step 26 judges each connector-facing hand pin | `src/offboard.rs` | `.filter(\|(_, net, _)\| facing.contains(net))` -> `.filter(\|(_, net, _)\| facing.contains(net) && false)` | `offboard::a_hand_tt_pin_at_a_connector` | killed |
| M12 | the FT family is the structure before its first `_` | `src/offboard.rs` | `structure.split('_').next() == Some("FT")` -> `structure.starts_with("FT_")` | `offboard::a_hand_ft_pin_of_bare_structure` | killed |
| M14 | connector_facing: the nets connector pins name | `src/offboard.rs` | `.filter_map(\|p\| p.net.as_deref())` -> `.filter_map(\|_\| None::<&str>)` | `offboard::a_connector_facing_` | killed |
| M15 | motor pull source: InvalidSource | `src/lib.rs` | `board::source(&m.pull.source, &format!("{at}.source"))?;` -> `` | `offboard::a_blank_source_or_reference` | killed |
| M16 | reserved reason: InvalidSource | `src/lib.rs` | `board::source(&r.reason, &format!("mcu.reserved_pins[{i}].reason"))?;` -> `` | `offboard::a_blank_reason` | killed |
| M17 | the pull's reference joins names::references | `src/names.rs` | `.map(\|m\| m.pull.resistor.reference.as_str()),` -> `.map(\|m\| m.pull.resistor.reference.as_str()).filter(\|_\| false),` | `offboard::a_pull_reference_another_part_holds` | killed |
| M18 | weak_pull_down_ohms min above zero | `src/power_parts.rs` | `if raw.min <= 0 \|\| raw.min > raw.typical \|\| raw.typical > raw.max {` -> `if raw.min > raw.typical \|\| raw.typical > raw.max {` | `offboard::the_mcu_row_holds_rpd` | killed |
| M19 | weak_pull_down_ohms min <= typical | `src/power_parts.rs` | `if raw.min <= 0 \|\| raw.min > raw.typical \|\| raw.typical > raw.max {` -> `if raw.min <= 0 \|\| raw.typical > raw.max {` | `offboard::the_mcu_row_holds_rpd` | killed |
| M20 | weak_pull_down_ohms typical <= max | `src/power_parts.rs` | `if raw.min <= 0 \|\| raw.min > raw.typical \|\| raw.typical > raw.max {` -> `if raw.min <= 0 \|\| raw.min > raw.typical {` | `offboard::the_mcu_row_holds_rpd` | killed |
| M21 | all_reserved: the declared pins join the engine's | `src/solve.rs` | `pins.extend(spec.mcu.reserved_pins.iter().map(\|r\| r.pin.as_str()));` -> `` | `offboard::never_` | killed |
| M22 | solve_inner's reservation (cs, sense, pin devices) is all_reserved | `src/solve.rs` | `spec.validate().map_err(SolveError::InvalidSpecification)?;     let reserved = all_reserved(spec);` -> `spec.validate().map_err(SolveError::InvalidSpecification)?;     let reserved = reserved_pins(spec.mcu.hse.as_ref());` | `offboard::never_` | killed |
| M23 | Model::new's reservation (channels, bus and serial candidates, hand pins) is all_reserved | `src/solve.rs` | `let reserved = all_reserved(spec);         let pads` -> `let reserved = reserved_pins(spec.mcu.hse.as_ref());         let pads` | `offboard::never_` | killed |
| M24 | a declared reservation is a pin of the table (UnknownPin) | `src/solve.rs` | `if !table.pins.contains_key(&r.pin) {` -> `if false && !table.pins.contains_key(&r.pin) {` | `offboard::a_reservation_of_an_unknown_pin` | killed |
| M25 | ReservedPin names the declared reason | `src/solve.rs` | `format!("`mcu.reserved_pins`: {}", r.reason)` -> `ENGINE_RESERVED.to_owned()` | `offboard::a_hand_pin_on_a_declared_reservation` | killed |
| M33 | implied_pupd is 0 on a motor output | `src/silicon.rs` | `(Some(_), _) => 0,` -> `(Some(_), _) => PUPDR_PULL_DOWN,` | `offboard::a_motor_net_has_no_internal_pull` | killed |
| M34 | step 2: a reserved pin is an identifier | `src/lib.rs` | `plain(&r.pin, \\|\\| format!("mcu.reserved_pins[{i}].pin"))?;` -> `` | `offboard::a_blank_reserved_pin` | killed |
| M35 | step 2: the pull's reference is a reference | `src/lib.rs` | `identifier(IdentifierKind::Reference, &r.reference, \\|\\| {                 format!("{at}.resistor.reference")             })?;` -> `` | `offboard::a_blank_source_or_reference` | killed |
| M36 | step 2: the pull's part is an identifier | `src/lib.rs` | `plain(&r.part, \\|\\| format!("{at}.resistor.part"))?;` -> `` | `offboard::a_blank_source_or_reference` | killed |
| M37 | step 2: the pull's `to` is an identifier | `src/lib.rs` | `plain(&r.to, \\|\\| format!("{at}.resistor.to"))?;` -> `` | `offboard::a_blank_source_or_reference` | killed |

| A1 | F1 withhold: a bus line's placement, every instance | `src/solve.rs` | `let order: Vec<(&'t str, u8)> = model.offered(inst, sig).collect();` -> `let order: Vec<(&'t str, u8)> = model.pins_for(inst, sig.name).to_vec();` | `offboard::a_connector_facing_bus` | killed |
| A2 | F1 withhold: a bus line's pruning (`Search::pins`: `has_pin`, `placeable`, `consistent`, `check_each_bus`) | `src/solve.rs` | `self.model             .offered(inst, sig)             .map(\|(pin, _)\| pin)` -> `self.model             .pins_for(inst, sig.name)             .iter()             .map(\|(pin, _)\| *pin)` | `offboard::a_connector_facing_bus` | killed |
| A3 | F1 withhold: a serial port's TX and RX | `src/solve.rs` | `self.model                 .offered(p, sig)                 .filter(\|(pin, _)\|` -> `self.model                 .pins_for(p, sig.name)                 .iter()                 .copied()                 .filter(\|(pin, _)\|` | `offboard::a_connector_facing_port` | killed |
| A4 | F1 withhold: a serial refusal's reason reads the offered pins | `src/solve.rs` | `let pins: Vec<(&str, u8)> = self.model.offered(p, sig).collect();` -> `let pins: Vec<(&str, u8)> = self.model.pins_for(p, sig.name).to_vec();` | `offboard::a_connector_facing_port_whose_only_pin_is_tt` | killed |
| A5 | F1 withhold: a motor output's channels | `src/solve.rs` | `&& self.model.at.offers(motor.label, pin)` -> `` | `offboard::a_connector_facing_motor` | killed |
| A6 | F1 withhold: a chip select | `src/solve.rs` | `free().find(\|p\| model.at.offers(cs.net, p))` -> `free().find(\|_\| true)` | `offboard::a_connector_facing_chip_select` | killed |
| A7 | F1 withhold: a pin device's ADC input | `src/pin_devices.rs` | `free.retain(\|(n, _)\| at.offers(&a.net, n));` -> `` | `offboard::a_connector_facing_adc_input` | killed |
| A8 | F1 withhold: a pin device's signal | `src/pin_devices.rs` | `free().find(\|p\| at.offers(&s.net, p))` -> `free().find(\|_\| true)` | `offboard::a_connector_facing_device_signal` | killed |
| A9 | F1: the withheld note names the non-FT pins | `src/offboard.rs` | `pins.into_iter().filter(\|p\| !self.offers(net, p)).collect();` -> `pins.into_iter().filter(\|_\| false).collect();` | `offboard::a_connector_facing_` | killed |
| A10 | F1: a pin with no held structure counts as non-FT | `src/offboard.rs` | `self.structure(pin).is_some_and(is_ft)` -> `self.structure(pin).is_none_or(is_ft)` | `offboard::a_pin_with_no_held_structure` | killed |
| A11 | F2 offer: no motor channel on a reset-pull pin | `src/solve.rs` | `&& !reset_pull(pin)` -> `` | `offboard::the_solver_never_offers_a_motor_a_reset_pull` | killed |
| A12 | F2 hand: MotorPinResetPull | `src/offboard.rs` | `if let Some(pin) = m.pin.as_deref().filter(\|p\| {` -> `if let Some(pin) = m.pin.as_deref().filter(\|p\| false && {` | `offboard::a_motor_pinned_on_a_reset_pull` | killed |
| A13 | F3: MotorIdleHighPulledDown | `src/offboard.rs` | `if m.pwm.as_ref().map(\|p\| p.idle)` -> `if false && m.pwm.as_ref().map(\|p\| p.idle)` | `offboard::a_motor_idling_high` | killed |
| A14 | F8: a solved claimant at a connector needs the MCU row | `src/offboard.rs` | `.filter(\|(_, net, _)\| facing.contains(net))` -> `.filter(\|(_, net, p)\| facing.contains(net) && p.is_some())` | `offboard::an_mcu_with_no_row` | killed |

Not mutated:

- `pin_devices::hand_free`'s call to `reserved_refusal`. `Model::new` refuses every pin device's hand pin first, through the same `reserved_refusal`, so this call is not reachable from a specification. That is the pre-existing double check, and the M25 edit to `reserved_refusal` covers both callers.
- The battery sense. A `power.sense` net is never a connector's net (`ConnectorNetUnknown`), so no filter was added there.
- `AtConnector::new`'s refusal at `mcu.part`. Step 26 refuses the same case first (A14 and M8 cover it), so the solver's copy is unreachable from a specification; it replaces the silent `_ => BTreeSet::new()` arm (F8).
- `reset_held`'s motor-output arm, removed: F2's refusal at validate and the solver's filter (A12, A11) leave it nothing to refuse.
