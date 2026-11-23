# AET-WO-4240e: mutants of the kernel-clock rule

Each mutant was applied alone to `crates/aethelgard-ir/src/kernel_clock.rs`, then
`cargo test -p aethelgard-ir --test kernel_clock` was run; the file was restored and
compared byte for byte after the run. 54 mutants, 54 killed, none a compile error.
Line numbers are the current file's (doc comments were edited after the run; the code
differs only in the `NO_MUX_PAGES` text). Tests are in `crates/aethelgard-ir/tests/kernel_clock.rs`.

| id | file:line | change | killed by |
|---|---|---|---|
| K01 | `kernel_clock.rs:399` | PLLSRC guard: != 00 -> == 11 | `pllsrc_divm1_and_divq1en_are_read` |
| K02 | `kernel_clock.rs:403` | DIVM1 guard: == 0 -> == 64 (never) | `pllsrc_divm1_and_divq1en_are_read (by a divide-by-zero panic in `hz``, `not by the assertion)` |
| K03 | `kernel_clock.rs:410` | inside(): lower bound dropped | `the_reference_is_judged_against_fpll_in_for_its_vco_range`, `the_reference_is_judged_at_both_band_edges_against_pll1rge`, `the_vco_is_judged_across_the_band_on_the_stricter_revision` |
| K04 | `kernel_clock.rs:410` | inside(): upper bound dropped | `the_reference_is_judged_at_both_band_edges_against_pll1rge`, `the_vco_is_judged_across_the_band_on_the_stricter_revision`, `the_reference_is_judged_against_fpll_in_for_its_vco_range` |
| K05 | `kernel_clock.rs:413` | PLL1RGE check dropped | `the_reference_is_judged_at_both_band_edges_against_pll1rge` |
| K06 | `kernel_clock.rs:423` | PLL1VCOSEL decode swapped | `pll1_q_on_an_hsi_board_is_not_running`, `pll1_q_is_judged_at_the_fastest_against_the_scale_maximum`, `pllsrc_divm1_and_divq1en_are_read`, `the_reference_is_judged_against_fpll_in_for_its_vco_range`, `the_ruled_kernel_switch_passes`, `the_vco_is_judged_across_the_band_on_the_stricter_revision`, `the_reference_is_judged_at_both_band_edges_against_pll1rge` |
| K07 | `kernel_clock.rs:428` | fPLL_IN check dropped | `the_reference_is_judged_against_fpll_in_for_its_vco_range` |
| K08 | `kernel_clock.rs:441` | VCO check dropped | `the_vco_is_judged_across_the_band_on_the_stricter_revision` |
| K09 | `kernel_clock.rs:452` | VOS0 given VOS1's Q maximum | `pll1_q_is_judged_at_the_fastest_against_the_scale_maximum` |
| K10 | `kernel_clock.rs:454` | VOS2 row read as VOS1's | `pll1_q_is_judged_at_the_fastest_against_the_scale_maximum` |
| K11 | `kernel_clock.rs:458` | Q judged at the band's bottom, not its top | `pll1_q_is_judged_at_the_fastest_against_the_scale_maximum` |
| K12 | `kernel_clock.rs:466` | DIVQ1EN check dropped | `pllsrc_divm1_and_divq1en_are_read` |
| K13 | `kernel_clock.rs:263` | wide VCO max 836 -> 960 (rev V / RM) | `the_vco_is_judged_across_the_band_on_the_stricter_revision` |
| K14 | `kernel_clock.rs:264` | wide Q max at VOS3 200 -> 210 | `pll1_q_is_judged_at_the_fastest_against_the_scale_maximum` |
| K15 | `kernel_clock.rs:274` | medium Q max at VOS2 210 -> 300 | `pll1_q_is_judged_at_the_fastest_against_the_scale_maximum` |
| K16 | `kernel_clock.rs:262` | wide fPLL_IN min 2 -> 1 MHz | `the_reference_is_judged_against_fpll_in_for_its_vco_range` |
| K17 | `kernel_clock.rs:273` | medium VCO max 420 -> 430 | `the_vco_is_judged_across_the_band_on_the_stricter_revision` |
| K18 | `kernel_clock.rs:241` | PLL1RGE 10 max 8 -> 8.2 MHz | `the_reference_is_judged_at_both_band_edges_against_pll1rge` |
| K19 | `kernel_clock.rs:241` | PLL1RGE 11 min 8 -> 7.7 MHz | `the_reference_is_judged_at_both_band_edges_against_pll1rge` |
| K20 | `kernel_clock.rs:532` | per_ck dropped from the reset sources | `an_hse_board_runs_pll1_q_when_divq1en_is_set`, `pll1_q_is_judged_at_the_fastest_against_the_scale_maximum`, `pllsrc_divm1_and_divq1en_are_read`, `the_ruled_kernel_switch_passes`, `the_reference_is_judged_against_fpll_in_for_its_vco_range`, `the_reference_is_judged_at_both_band_edges_against_pll1rge`, `the_vco_is_judged_across_the_band_on_the_stricter_revision`, `the_scar_spi1_per_ck_with_no_switch_is_a_dead_switch` |
| K21 | `kernel_clock.rs:536` | rcc_pclk2 dropped from the reset sources | `a_kernel_switch_beside_an_hse_tree_is_refused`, `i2c_and_usart_pass_on_their_reset_inputs`, `spi4_to_6_pass_on_both_their_kernels` |
| K22 | `kernel_clock.rs:554` | HSE tree: DIVP1EN read for pll1_q_ck | `an_hse_board_runs_pll1_q_when_divq1en_is_set` |
| K23 | `kernel_clock.rs:551` | HSE tree's PLL1 ignored | `an_hse_board_runs_pll1_q_when_divq1en_is_set` |
| K24 | `kernel_clock.rs:558` | no kernel_switch: PLL1 treated as running | `pll1_q_on_an_hsi_board_is_not_running`, `refusals_are_one_per_peripheral_in_name_order`, `the_scar_spi1_per_ck_with_no_switch_is_a_dead_switch` |
| K25 | `kernel_clock.rs:560` | kernel_switch words not judged | `the_reference_is_judged_against_fpll_in_for_its_vco_range`, `the_vco_is_judged_across_the_band_on_the_stricter_revision`, `pllsrc_divm1_and_divq1en_are_read`, `pll1_q_is_judged_at_the_fastest_against_the_scale_maximum`, `the_reference_is_judged_at_both_band_edges_against_pll1rge` |
| K26 | `kernel_clock.rs:563` | PLL1 not stopped after the window | `pll1_q_on_an_hsi_board_is_not_running` |
| K27 | `kernel_clock.rs:565` | inputs off at reset treated as running | `an_adc_runs_on_ckmode_and_not_on_pll2_p`, `refusals_are_one_per_peripheral_in_name_order` |
| K28 | `kernel_clock.rs:709` | power-sense filter dropped | `a_power_sense_input_is_not_an_enabled_adc` |
| K29 | `kernel_clock.rs:735` | kernel_switch beside hse not refused | `a_kernel_switch_beside_an_hse_tree_is_refused` |
| K30 | `kernel_clock.rs:743` | timers not skipped | `a_peripheral_the_table_cannot_judge_is_unknown` |
| K31 | `kernel_clock.rs:196` | TIM17 dropped from the no-mux row | `a_peripheral_the_table_cannot_judge_is_unknown` |
| K32 | `kernel_clock.rs:752` | absent clock.adc passed | `an_adc_runs_on_ckmode_and_not_on_pll2_p`, `a_power_sense_input_is_not_an_enabled_adc` |
| K33 | `kernel_clock.rs:758` | CKMODE 11 not read as synchronous | `an_adc_runs_on_ckmode_and_not_on_pll2_p` |
| K34 | `kernel_clock.rs:759` | CKMODE 00 passed without the mux | `an_adc_runs_on_ckmode_and_not_on_pll2_p`, `refusals_are_one_per_peripheral_in_name_order` |
| K35 | `kernel_clock.rs:760` | CKMODE > 3 passed | `an_adc_runs_on_ckmode_and_not_on_pll2_p` |
| K36 | `kernel_clock.rs:767` | SPI with no controller passed | `a_peripheral_the_table_cannot_judge_is_unknown`, `every_source_of_an_enabled_peripheral_is_read` |
| K37 | `kernel_clock.rs:223` | per_ck mapped to SPI123SEL's reset code | `an_hse_board_runs_pll1_q_when_divq1en_is_set`, `pll1_q_is_judged_at_the_fastest_against_the_scale_maximum`, `the_scar_spi1_per_ck_with_no_switch_is_a_dead_switch`, `pllsrc_divm1_and_divq1en_are_read`, `refusals_are_one_per_peripheral_in_name_order`, `the_reference_is_judged_at_both_band_edges_against_pll1rge`, `the_reference_is_judged_against_fpll_in_for_its_vco_range`, `the_vco_is_judged_across_the_band_on_the_stricter_revision`, `the_ruled_kernel_switch_passes` |
| K38 | `kernel_clock.rs:225` | HsiKerCk arm deleted (no code) | `spi4_to_6_pass_on_both_their_kernels` |
| K39 | `kernel_clock.rs:222` | Pll1Q accepted on SPI6SEL | `a_peripheral_the_table_cannot_judge_is_unknown` |
| K40 | `kernel_clock.rs:765` | SPI kernels not read | `an_hse_board_runs_pll1_q_when_divq1en_is_set`, `a_peripheral_the_table_cannot_judge_is_unknown`, `the_vco_is_judged_across_the_band_on_the_stricter_revision`, `every_source_of_an_enabled_peripheral_is_read`, `pllsrc_divm1_and_divq1en_are_read`, `pll1_q_is_judged_at_the_fastest_against_the_scale_maximum`, `the_scar_spi1_per_ck_with_no_switch_is_a_dead_switch`, `the_reference_is_judged_against_fpll_in_for_its_vco_range`, `the_ruled_kernel_switch_passes`, `the_reference_is_judged_at_both_band_edges_against_pll1rge`, `refusals_are_one_per_peripheral_in_name_order` |
| K41 | `kernel_clock.rs:786` | the switch never judged | `an_hse_board_runs_pll1_q_when_divq1en_is_set`, `pll1_q_is_judged_at_the_fastest_against_the_scale_maximum`, `the_vco_is_judged_across_the_band_on_the_stricter_revision`, `the_scar_spi1_per_ck_with_no_switch_is_a_dead_switch`, `pllsrc_divm1_and_divq1en_are_read`, `the_reference_is_judged_at_both_band_edges_against_pll1rge`, `the_reference_is_judged_against_fpll_in_for_its_vco_range`, `refusals_are_one_per_peripheral_in_name_order` |
| K42 | `kernel_clock.rs:787` | the from input not judged in the window | `the_vco_is_judged_across_the_band_on_the_stricter_revision`, `the_reference_is_judged_against_fpll_in_for_its_vco_range`, `the_scar_spi1_per_ck_with_no_switch_is_a_dead_switch`, `the_reference_is_judged_at_both_band_edges_against_pll1rge`, `pllsrc_divm1_and_divq1en_are_read`, `pll1_q_is_judged_at_the_fastest_against_the_scale_maximum`, `refusals_are_one_per_peripheral_in_name_order`, `an_hse_board_runs_pll1_q_when_divq1en_is_set` |
| K43 | `kernel_clock.rs:805` | the start judged at the window | `pll1_q_on_an_hsi_board_is_not_running` |
| K44 | `kernel_clock.rs:205` | SPI4/5 on SPI6SEL's row | `a_peripheral_the_table_cannot_judge_is_unknown` |
| K45 | `kernel_clock.rs:186` | ADCSEL reset input per_ck | `refusals_are_one_per_peripheral_in_name_order`, `an_adc_runs_on_ckmode_and_not_on_pll2_p` |
| K46 | `kernel_clock.rs:715` | serial ports not enabled | `every_source_of_an_enabled_peripheral_is_read` |
| K47 | `kernel_clock.rs:713` | I2C controllers not enabled | `every_source_of_an_enabled_peripheral_is_read` |
| K48 | `kernel_clock.rs:717` | pins' peripherals not enabled | `a_peripheral_the_table_cannot_judge_is_unknown` |
| K49 | `kernel_clock.rs:718` | SPI devices' peripherals not enabled | `every_source_of_an_enabled_peripheral_is_read` |
| K50 | `kernel_clock.rs:719` | I2C devices' peripherals not enabled | `every_source_of_an_enabled_peripheral_is_read` |
| K51 | `kernel_clock.rs:716` | ADC inputs not enabled | `an_adc_runs_on_ckmode_and_not_on_pll2_p`, `a_power_sense_input_is_not_an_enabled_adc`, `refusals_are_one_per_peripheral_in_name_order` |
| K52 | `kernel_clock.rs:674` | kind() name swapped | `refusals_are_one_per_peripheral_in_name_order` |
| K53 | `kernel_clock.rs:234` | HSI nominal 64 -> 64.5 MHz | `the_reference_is_judged_at_both_band_edges_against_pll1rge`, `the_vco_is_judged_across_the_band_on_the_stricter_revision` |
| K54 | `kernel_clock.rs:209` | USART6 row dropped | `i2c_and_usart_pass_on_their_reset_inputs` |

Not mutated, and why:

- **Whether the bounds include their limits** (`>=` → `>` and `<=` → `<` in `inside`; `>` → `>=` in the Q check).
  Equality cannot occur. Neither edge of the HSI band (62,342,784 and 64,958,976 Hz) is divisible
  by 5. Every limit is a multiple of 10⁶ = 2⁶·5⁶. The only other factor, `DIVN1` + 1, is at most
  512, which is less than 5⁶ = 15,625.
- **`spi_code`'s `HsiKerCk` changed to `0b000` on SPI45SEL or SPI6SEL.** Both `hsi_ker_ck` and the
  APB clock run from reset, so the verdict cannot change. K38 deletes the arm instead, and is killed.
- **The `to` half of the window check** (`[from, to]` → `[from]`). Every non-reset input a
  topology selects today (`per_ck`, `hsi_ker_ck`) runs from reset, so no mutant of it is killable.
- **The `continue` after a `KernelClockDeadSwitch`.** A dead switch today always has a dead
  `from` and a running `to`, so the start check after it never fires.
- **`DIVN1` codes 0–2** ("wrong configurations", RM0433 p. 404) have no guard of their own. N ≤ 3
  puts the VCO under 48 MHz, and the VCO check refuses that.

