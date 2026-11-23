# AET-WO-4240e: mutants of the guards this build added outside the rule

The rule's own are `mutants-rule.md`. Each mutant was applied alone, its named test run, the file restored and compared byte for byte (`~/.cache/aet-4240e/kit/mutants.py`, `mutants-rerun.py`, `mutants-review.py`).

## The build (G)

| id | file | change | result |
|---|---|---|---|
| G01 | `crates/aethelgard-svd-hal/src/board.rs` | PLL1ON never set | killed by the_switch_locks_pll1_before_the_select_and_stops_it_after |
| G02 | `crates/aethelgard-svd-hal/src/board.rs` | no fault on a lock that does not come | killed by the_switch_locks_pll1_before_the_select_and_stops_it_after |
| G03 | `crates/aethelgard-svd-hal/src/board.rs` | PLL1 left running | killed by the_switch_locks_pll1_before_the_select_and_stops_it_after |
| G04 | `crates/aethelgard-svd-hal/src/board.rs` | one transition read | killed by the_switch_locks_pll1_before_the_select_and_stops_it_after |
| G05 | `crates/aethelgard-svd-hal/src/board.rs` | transition wait too short | killed by the_switch_locks_pll1_before_the_select_and_stops_it_after |
| G06 | `crates/aethelgard-svd-hal/src/board.rs` | any CKMODE taken | killed by the_adc_converts_on_the_clock_the_topology_states |
| G08 | `crates/aethelgard-svd-hal/src/board/firmware.rs` | PLL1RDY bound one read | killed by a_lock_that_does_not_come_writes_its_boot_word_before_any_enable |
| G09 | `crates/aethelgard-svd-hal/src/board/firmware.rs` | main's fault arm on the wrong boards | killed by a_select_between_live_inputs_starts_no_pll, a_lock_that_does_not_come_writes_its_boot_word_before_any_enable |
| G11 | `crates/aethelgard-ir/src/solve.rs` | switch without a per_ck select | killed by a_select_between_live_inputs_starts_no_pll |
| G12 | `crates/aethelgard-ir/src/solve.rs` | no ADC clock fact | killed by the_adc_converts_on_the_clock_the_topology_states |
| G13 | `crates/aethelgard-ir/src/solve.rs` | VCO floor | killed by solve::differential::the_switch_words_hold_across_the_hsi_band |
| G14 | `crates/aethelgard-ir/src/solve.rs` | Q ceiling | killed by solve::differential::the_switch_words_hold_across_the_hsi_band |
| G15 | `crates/aethelgard-ir/src/lib.rs` | validate refuses the switch | killed by the_switch_locks_pll1_before_the_select_and_stops_it_after, a_switch_slower_than_per_ck_is_refused, a_lock_that_does_not_come_writes_its_boot_word_before_any_enable, the_adc_converts_on_the_clock_the_topology_states |
| G16 | `crates/aethelgard-cli/src/ledger.rs` | verdict holds over the rule | killed by the_verdict_holds_the_switch_and_refuses_the_scar_by_name |
| G18 | `crates/aethelgard-cli/src/ledger.rs` | reset select rows back to ignored-by-model | killed by ledger::tests::a_field_the_table_does_not_know_is_refused_by_name |
| G20 | `crates/aethelgard-twin/src/serial.rs` | twin: select outside the window passes | killed by serial::tests::the_rcc_takes_the_kernel_switch_in_order_around_the_select |
| G21 | `crates/aethelgard-twin/src/serial.rs` | twin: a half-done switch passes | killed by serial::tests::the_rcc_takes_the_kernel_switch_in_order_around_the_select |
| G22 | `crates/aethelgard-twin/src/serial.rs` | twin: PLL1ON never cleared passes | killed by serial::tests::the_rcc_takes_the_kernel_switch_in_order_around_the_select |
| G23 | `crates/aethelgard-ir/src/lib.rs` | derive_controllers forgets the switch | killed by board::tests::each_enable_reads_its_register_back |
| G07 | `crates/aethelgard-svd-hal/src/board.rs` | generation ignores the rule | killed by every_scar_board_is_clocked_as_solved_and_refused_by_name_as_it_was |
| G17 | `crates/aethelgard-cli/src/ledger.rs` | verdict names nothing | killed by the_verdict_holds_the_switch_and_refuses_the_scar_by_name |
| G19 | `crates/aethelgard-svd-hal/src/board/ledger.rs` | PLL1RDY's polled row dropped | killed by the_switch_s_ledger_rows_are_pll1on_alone_and_pll1rdy_read_zero |

- **G10** (the HSE test in `solve::kernel_switch`) survived: equivalent, since an HSE board's SPI1-3 are never on `per_ck`. The guard was removed (the reduction pass).
- **G18** (the reset select rows' class) was run before Q10 and killed; Q10 reverted those rows to main's, so it no longer applies.
- G07, G17 and G19 first failed to apply (rustfmt had reflowed their lines) and were re-run.

## The adversarial pass's fixes (R)

| id | file | change | result |
|---|---|---|---|
| R01 | `crates/aethelgard-ir/src/kernel_clock.rs` | DIVN1 0x002 passes | killed by codes_the_manual_does_not_allow_are_refused |
| R02 | `crates/aethelgard-ir/src/kernel_clock.rs` | DIVP1 /5 passes | killed by codes_the_manual_does_not_allow_are_refused |
| R03 | `crates/aethelgard-ir/src/kernel_clock.rs` | DIVR1 0 passes | killed by codes_the_manual_does_not_allow_are_refused |
| R04 | `crates/aethelgard-ir/src/kernel_clock.rs` | FRACEN passes | killed by codes_the_manual_does_not_allow_are_refused |
| R05 | `crates/aethelgard-ir/src/kernel_clock.rs` | shared select split passes | killed by controllers_sharing_a_select_on_different_kernels_are_refused, refusals_are_one_per_peripheral_in_name_order |
| R06 | `crates/aethelgard-svd-hal/src/board.rs` | slow Q emitted | killed by a_switch_slower_than_per_ck_is_refused |

## The twin judge's RCC_CR comparison (T), after the simulate run

The first form compared each `RCC_CR` write with the last read in the access log. Renode's log carries no reads, so the real boot failed (`serial_tag_mismatch`), and G22 tested that form. The judge now masks `RCC_CR`'s read-only flags and compares the write against the word from reset. Simulating both boards in the tools image then passes.

| id | file | change | result |
|---|---|---|---|
| T01 | `crates/aethelgard-twin/src/serial.rs` | twin: CR's read-only flags not masked | killed by serial::tests::the_rcc_takes_the_kernel_switch_in_order_around_the_select |
| T02 | `crates/aethelgard-twin/src/serial.rs` | twin: any switch word passes | killed by serial::tests::the_rcc_takes_the_kernel_switch_in_order_around_the_select |
