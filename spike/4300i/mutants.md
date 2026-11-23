# AET-WO-4300i mutants

One mutant per guard this work order adds, sized to the diff (CLAUDE.md). `spike/4300i/mutants.py` applies each edit, runs the named filter of `cargo test -p aethelgard-ir --test offboard`, records the result, and restores the file with `git checkout`. It ran on the committed tree, with no other change in any file it touches. The raw results are in `out/mutants.tsv`.

**Result: 37/37 killed, 0 survived.**

| id | guard | file | mutant (exact edit) | killing test | result |
|---|---|---|---|---|---|
| M1 | MotorPullNotDown | `src/offboard.rs` | `if r.to != crate::GND {` -> `if false && r.to != crate::GND {` | `offboard::a_pull_that_is_not_to_gnd` | killed |
| M2 | PowerPartUnknown: a motor pull's part is a resistor row | `src/offboard.rs` | `Some(power_parts::Kind::Resistor(_))` -> `Some(_)` | `offboard::a_pull_whose_part_is_not_a_resistor_row` | killed |
| M3 | MotorPullOutsideRpd, above max | `src/offboard.rs` | `if ohms < rpd.min \|\| ohms > rpd.max {` -> `if ohms < rpd.min {` | `offboard::a_pull_outside_rpd` | killed |
| M4 | MotorPullOutsideRpd, min is admitted (30000) | `src/offboard.rs` | `if ohms < rpd.min \|\| ohms > rpd.max {` -> `if ohms <= rpd.min \|\| ohms > rpd.max {` | `offboard::a_pull_outside_rpd` | killed |
| M5 | MotorPullOutsideRpd, max is admitted (50000) | `src/offboard.rs` | `if ohms < rpd.min \|\| ohms > rpd.max {` -> `if ohms < rpd.min \|\| ohms >= rpd.max {` | `offboard::a_pull_outside_rpd` | killed |
| M6 | ReservedPinTwice | `src/offboard.rs` | `if !seen.insert(r.pin.as_str()) {` -> `if false && !seen.insert(r.pin.as_str()) {` | `offboard::a_pin_reserved_twice` | killed |
| M7 | ReservedPinEngineReserved | `src/offboard.rs` | `if engine.contains(&r.pin.as_str()) {` -> `if false && engine.contains(&r.pin.as_str()) {` | `offboard::a_pin_the_engine_reserves` | killed |
| M8 | PowerPartUnknown at mcu.part, only where the row is needed | `src/offboard.rs` | `if spec.motor_outputs.is_empty() && hands.is_empty() {` -> `if spec.motor_outputs.is_empty() \|\| hands.is_empty() {` | `offboard::an_mcu_with_no_row` | killed |
| M9 | ConnectorPinNotFt | `src/offboard.rs` | `if !is_ft(structure) {` -> `if false && !is_ft(structure) {` | `offboard::a_hand_tt_pin_at_a_connector` | killed |
| M10 | McuPinStructureUnheld at a connector | `src/offboard.rs` | `let Some((structure, at)) = io.pins.get(pin) else {` -> `let Some((structure, at)) = io.pins.get(pin).or_else(\|\| io.pins.get("PA0")) else {` | `offboard::a_hand_pin_at_a_connector_with_no_held_structure` | killed |
| M11 | step 26 judges each connector-facing hand pin | `src/offboard.rs` | `.filter(\|(_, net, _)\| facing.contains(net))` -> `.filter(\|(_, net, _)\| facing.contains(net) && false)` | `offboard::a_hand_tt_pin_at_a_connector` | killed |
| M12 | the FT family is the structure before its first `_` | `src/offboard.rs` | `structure.split('_').next() == Some("FT")` -> `structure.starts_with("FT_")` | `offboard::a_hand_ft_pin_of_bare_structure` | killed |
| M13 | ft_first: FT first, then the rest | `src/offboard.rs` | `let (mut first, rest): (Vec<T>, Vec<T>) = items.into_iter().partition(\|t\| ft.contains(pin(t)));` -> `let (mut first, rest): (Vec<T>, Vec<T>) = items.into_iter().partition(\|t\| !ft.contains(pin(t)));` | `offboard::a_connector_facing_` | killed |
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
| M26 | FT first: a motor output's channels | `src/solve.rs` | `.ft_first(self.model.motors[m].label, options, \|c\| c.2);` -> `.ft_first("", options, \|c\| c.2);` | `offboard::a_connector_facing_motor` | killed |
| M27 | FT first: a serial TX | `src/solve.rs` | `self.model.ft_first(port.tx.net, pins(p, &port.tx), \|c\| c.0);` -> `self.model.ft_first("", pins(p, &port.tx), \|c\| c.0);` | `offboard::a_connector_facing_port` | killed |
| M28 | FT first: a serial RX | `src/solve.rs` | `self.model.ft_first(port.rx.net, pins(p, &port.rx), \|c\| c.0);` -> `self.model.ft_first("", pins(p, &port.rx), \|c\| c.0);` | `offboard::a_connector_facing_port` | killed |
| M29 | FT first: a bus line | `src/solve.rs` | `model.ft_first(sig.net, model.pins_for` -> `model.ft_first("", model.pins_for` | `offboard::a_connector_facing_bus_line` | killed |
| M30 | FT first: a chip select | `src/solve.rs` | `ft.order(cs.net, keys, \|p\| p)` -> `ft.order("", keys, \|p\| p)` | `offboard::a_connector_facing_chip_select` | killed |
| M31 | FT first: a pin device's ADC input | `src/pin_devices.rs` | `ft.order(&a.net, free, \|(n, _)\| n);` -> `ft.order("", free, \|(n, _)\| n);` | `offboard::a_connector_facing_adc_input` | killed |
| M32 | FT first: a pin device's signal | `src/pin_devices.rs` | `.order(                         &s.net,` -> `.order(                         "",` | `offboard::a_connector_facing_device_signal` | killed |
| M33 | implied_pupd is 0 on a motor output | `src/silicon.rs` | `(Some(_), _) => 0,` -> `(Some(_), _) => PUPDR_PULL_DOWN,` | `offboard::a_motor_net_has_no_internal_pull` | killed |
| M34 | step 2: a reserved pin is an identifier | `src/lib.rs` | `plain(&r.pin, \\|\\| format!("mcu.reserved_pins[{i}].pin"))?;` -> `` | `offboard::a_blank_reserved_pin` | killed |
| M35 | step 2: the pull's reference is a reference | `src/lib.rs` | `identifier(IdentifierKind::Reference, &r.reference, \\|\\| {                 format!("{at}.resistor.reference")             })?;` -> `` | `offboard::a_blank_source_or_reference` | killed |
| M36 | step 2: the pull's part is an identifier | `src/lib.rs` | `plain(&r.part, \\|\\| format!("{at}.resistor.part"))?;` -> `` | `offboard::a_blank_source_or_reference` | killed |
| M37 | step 2: the pull's `to` is an identifier | `src/lib.rs` | `plain(&r.to, \\|\\| format!("{at}.resistor.to"))?;` -> `` | `offboard::a_blank_source_or_reference` | killed |

Not mutated:

- `pin_devices::hand_free`'s call to `reserved_refusal`. `Model::new` refuses every pin device's hand pin first, through the same `reserved_refusal`, so this call is not reachable from a specification. That is the pre-existing double check, and the M25 edit to `reserved_refusal` covers both callers.
- The battery sense order. A `power.sense` net is never a connector's net (`ConnectorNetUnknown`), so no partition was added there.
