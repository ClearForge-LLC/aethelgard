# AET-WO-4300i phase B2 mutants

One mutant per guard phase B2 (eda draws the motor pull) adds, sized to the diff. `spike/4300i/mutants_b2.py` applies each edit, runs the named filter of `cargo test -p aethelgard-cli --test <binary>`, records the result and restores the file with `git checkout`. It ran on the committed tree. Raw results: `out/mutants-b2.tsv`.

**Result: 16/16 killed, 0 survived.** B16 survived its first run (no test put a non-resistor across a motor net and GND); `a_second_resistor_beside_a_motor_pull_fails_by_name` gained the capacitor case, and the run above is after it.

| id | guard | file | mutant (exact edit) | killing test | result |
|---|---|---|---|---|---|
| B1 | eda draws each motor pull (`motor_pulls` in `design`) | `src/schematic.rs` | `motor_pulls(&mut b)?;` -> `` | `emit_tier1::every_motor_net_carries_its_declared_pull_down` | killed |
| B2 | the motor pull's reference is claimed with the declared ones | `src/schematic.rs` | `.map(\|m\| &m.pull.resistor.reference),` -> `.map(\|m\| &m.pull.resistor.reference).filter(\|_\| false),` | `emit_tier1::a_motor_pull_declared_below_the_emitters_numbers_keeps_its_reference` | killed |
| B3 | the netlist's `footprint` is read | `src/support.rs` | `Some(_) => Some(text(&comp, "footprint")?.to_owned()),` -> `Some(_) => None,` | `support_tier1::the_motor_fixtures_pulls_pass_the_support_gate` | killed |
| B4 | `motor_pull_missing`: the row's symbol | `src/support.rs` | `.filter(\|c\| c.symbol.as_deref() == Some(row.symbol.as_str()) && !c.unpopulated)` -> `.filter(\|c\| !c.unpopulated)` | `support_tier1::a_motor_pull_absent_unpopulated_or_not_a_resistor_is_missing` | killed |
| B5 | `motor_pull_missing`: populated | `src/support.rs` | `.filter(\|c\| c.symbol.as_deref() == Some(row.symbol.as_str()) && !c.unpopulated)` -> `.filter(\|c\| c.symbol.as_deref() == Some(row.symbol.as_str()))` | `support_tier1::a_motor_pull_absent_unpopulated_or_not_a_resistor_is_missing` | killed |
| B6 | `motor_pull_off_net` is reported | `src/support.rs` | `if !joins(reference) {` -> `if false && !joins(reference) {` | `support_tier1::a_motor_pull_off_its_nets_fails_by_name` | killed |
| B7 | `motor_pull_off_net`: pins exactly 1 and 2 | `src/support.rs` | `pins.iter().map(\|(p, _)\| *p).eq(["1", "2"]) && {` -> `pins.len() == 2 && {` | `support_tier1::a_motor_pull_off_its_nets_fails_by_name` | killed |
| B8 | `motor_pull_off_net`: the two nets are the label's and `to`'s | `src/support.rs` | `have == want` -> `have.len() == want.len()` | `support_tier1::a_motor_pull_off_its_nets_fails_by_name` | killed |
| B9 | `motor_pull_value`: the resistance | `src/support.rs` | `ohms(token) == Some(u64::from(r.ohms)) && part == r.part` -> `part == r.part` | `support_tier1::a_motor_pull_of_another_value_or_part_fails_by_name` | killed |
| B10 | `motor_pull_value`: the part | `src/support.rs` | `ohms(token) == Some(u64::from(r.ohms)) && part == r.part` -> `ohms(token) == Some(u64::from(r.ohms))` | `support_tier1::a_motor_pull_of_another_value_or_part_fails_by_name` | killed |
| B11 | `motor_pull_footprint`: the row's footprint | `src/support.rs` | `if c.footprint.as_deref() != Some(row.footprint.as_str()) {` -> `if c.footprint.is_none() {` | `support_tier1::a_motor_pull_on_another_footprint_or_none_fails_by_name` | killed |
| B12 | `motor_pull_footprint`: none stated is a finding | `src/support.rs` | `if c.footprint.as_deref() != Some(row.footprint.as_str()) {` -> `if c.footprint.is_some() && c.footprint.as_deref() != Some(row.footprint.as_str()) {` | `support_tier1::a_motor_pull_on_another_footprint_or_none_fails_by_name` | killed |
| B13 | `motor_pull_count` is reported | `src/support.rs` | `if !parallel.is_empty() {` -> `if false && !parallel.is_empty() {` | `support_tier1::a_second_resistor_beside_a_motor_pull_fails_by_name` | killed |
| B14 | `motor_pull_count`: the declared pull is not its own parallel | `src/support.rs` | `q.as_str() != reference                     &&` -> `` | `support_tier1::the_motor_fixtures_pulls_pass_the_support_gate` | killed |
| B16 | `motor_pull_count`: only a resistor counts | `src/support.rs` | `&& x.symbol.as_deref() == Some(row.symbol.as_str())` -> `` | `support_tier1::a_second_resistor_beside_a_motor_pull_fails_by_name` | killed |
| B15 | step 8 runs in `judge` | `src/support.rs` | `motor_pulls(spec, netlist, &mut fail)?;` -> `` | `support_tier1::a_motor_pull_` | killed |
