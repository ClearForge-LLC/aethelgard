# AET-WO-4300j Phase A: mutants of the guards this build adds or changes in `aethelgard-ir`

Run by `spike/4300j/mutants_a.py`: each mutant applied as exact text, its killing test run alone, every file restored byte for byte.

| id | guard | mutation | killing test | killed |
|---|---|---|---|---|
| A1 | bound floor side (`MotorPullOutsideBound`) | `<` to `<=` at the floor | `offboard::a_pull_outside_the_bound_is_refused_at_both_edges` | yes |
| A2 | bound ceiling side (`MotorPullOutsideBound`) | `>` to `>=` at the ceiling | `offboard::a_pull_outside_the_bound_is_refused_at_both_edges` | yes |
| A3 | floor rounded up | floor rounded down | `offboard::a_pull_outside_the_bound_is_refused_at_both_edges` | yes |
| A4 | floor rounded up, exactly | floor rounded up one step too far on a whole-ohm floor | `motor::the_bound_moves_with_the_rail` | yes |
| A5 | ceiling rounded down | ceiling rounded up | `offboard::a_pull_outside_the_bound_is_refused_at_both_edges` | yes |
| A6 | cross-rail: the peer's VIH at VDD max (D1 (a)) | same-rail: VIH at VDD min | `offboard::a_pull_outside_the_bound_is_refused_at_both_edges` | yes |
| A7 | the pull-up at VDD min | the pull-up at nominal | `offboard::a_pull_outside_the_bound_is_refused_at_both_edges` | yes |
| A8 | I = leakage + peer input current | the peer's current dropped | `offboard::a_pull_outside_the_bound_is_refused_at_both_edges` | yes |
| A9 | margin at the floor | margin dropped at the floor | `offboard::a_pull_outside_the_bound_is_refused_at_both_edges` | yes |
| A10 | margin at the ceiling | margin dropped at the ceiling | `offboard::a_pull_outside_the_bound_is_refused_at_both_edges` | yes |
| A11 | `MotorPullFloorUnreachable` (denominator <= 0) | check removed | `motor::an_empty_or_unreachable_bound_admits_nothing` | yes |
| A12 | `MotorPullBoundEmpty` (floor > ceiling) | check removed | `motor::an_empty_or_unreachable_bound_admits_nothing` | yes |
| A13 | link part a jumper row (`PowerPartUnknown`) | a resistor row admitted as the link | `offboard::a_link_whose_part_is_not_a_jumper_row_is_refused` | yes |
| A14 | link reference in `names::references` | link references left out | `offboard::a_pull_reference_another_part_holds_is_refused` | yes |
| A15 | `<label>_PD` in `names::own_nets` | pull nets left out | `motor::a_net_named_motor1_pd_is_refused_in_any_case` | yes |
| A16 | link reference an identifier (step 2) | check removed | `offboard::a_blank_source_or_reference_is_refused_by_the_house_rules` | yes |
| A17 | `fitment` required, no default (`INV-06`) | `#[serde(default)]` on `fitment` | `offboard::a_missing_null_or_unknown_fitment_or_link_is_refused_at_parse` | yes |
| A18 | `implied_pupd` pull-down when `open` | `open` gives 0 | `offboard::a_motor_net_is_pulled_internally_only_when_open` | yes |
| A19 | `implied_pupd` none when `fitted` | `fitted` gives pull-down | `offboard::a_motor_net_is_pulled_internally_only_when_open` | yes |
| A20 | solve copies the fitment | every output solved `fitted` | `offboard::solve_copies_the_pull_with_its_bound` | yes |
| A21 | solve copies the window | floor one ohm low | `offboard::solve_copies_the_pull_with_its_bound` | yes |
| A22 | reader: margin < input low | check dropped | `offboard::the_mcu_row_holds_the_bound_inputs_and_refuses_them_malformed` | yes (second run; survived the first, see below) |
| A23 | reader: input high + margin < 1000 | check dropped | `offboard::the_mcu_row_holds_the_bound_inputs_and_refuses_them_malformed` | yes |
| A24 | reader: input low < input high | check dropped | `offboard::the_mcu_row_holds_the_bound_inputs_and_refuses_them_malformed` | yes |
| A25 | reader: a ruled value above zero | check dropped | `offboard::the_mcu_row_holds_the_bound_inputs_and_refuses_them_malformed` | yes |
| A26 | reader: jumper figures above zero | `positive` to `limit` | `offboard::the_jumper_row_is_held_and_refuses_it_malformed` | yes |
| A27 | reader: a ruled value's basis is text | basis check dropped | `offboard::the_mcu_row_holds_the_bound_inputs_and_refuses_them_malformed` | yes (second run) |
| A28 | reader: jumper current above zero | `positive` to `limit` | `offboard::the_jumper_row_is_held_and_refuses_it_malformed` | yes (second run) |
| A29 | link part an identifier (step 2) | check removed | `offboard::a_blank_source_or_reference_is_refused_by_the_house_rules` | yes (second run) |

**A22, the one survivor of the first run.** The test's planted case was a margin of 300 permille against an input low of 300, but with an input high of 700 that also makes `high + margin` 1000, so A23's guard refused it and the dropped check went unseen. The case now sets input low 100 against the margin 100 (`high + margin` stays 800), refused, with 101 admitted. A22, A23 and A24 were re-run against the changed test (`python3 -I spike/4300j/mutants_a.py . A22 A23 A24`), and all three were killed. After the full run, the working tree's diff was byte-identical to before it (sha256 of `git diff`, before and after).

A27 to A29 were added after the first run, to cover three guards that had a test but no mutant, and were run on their own (`... mutants_a.py . A27 A28 A29`). All 29 were killed.
