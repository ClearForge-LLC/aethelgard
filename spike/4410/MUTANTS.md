# AET-WO-4410 — mutants (one per guard this work order adds or changes)

Run by `mutants.py` (each edit reverted in a `finally`), in a capped user scope (`MemoryMax=32G`, `MemorySwapMax=0`), against the named test only. **18 of 18 killed.**

| id | guard | test | result |
|---|---|---|---|
| h-swd-lines | hwdef emits both SWD lines | `every_board_keeps_swd_on` | killed (first run: the mutant did not compile; rewritten) |
| h-swd-solved | `SwdPinSolved` | `a_net_solved_on_an_swd_pin_is_refused` | killed |
| h-swd-solved-gpio | the `gpio` pins in that walk | same | killed |
| h-adc1 | `SenseNotOnAdc1` | `a_sense_solved_on_adc3_alone_is_refused_by_name` | killed |
| h-f2-af-prefix | `LabelTakesAfPrefix` (`4400b` F2) | `a_chip_select_named_like_an_alternate_function_is_refused` | killed |
| h-f1-serial0 | the SERIAL0 pull sentence (`2311` F1) | `each_board_generates_its_committed_file` | killed |
| p-swd-check | `SwdNotKept` on any mode | `an_image_that_loses_swd_is_named` | killed |
| p-swd-af | `SwdNotKept` on the AF, not the mode alone | same | killed |
| p-swd-named | `SwdPinNamed` | `an_swd_pin_named_by_the_topology_or_the_allow_list_is_refused` | killed |
| p-swd-allow | `AllowNamesSwdPin` | same | killed (first run: not applied, `cargo fmt` had rewrapped the line) |
| p-odr | `WrongOutputLevel` | `a_chip_select_low_from_its_first_write_is_named` | killed |
| p-odr-bit | the `ODR` bit read is the pin's own | same | **survived the first run** (the review's `0xFFFC` holds PA0 and PA1 both low); the test gained `0xFFFE` and `0x0001`; killed |
| p-pmcr | `pmcr::WRITERS` masks | `no_pmcr_writer_at_the_tag_touches_pc2so_or_pc3so` | killed |
| r-fn-value | function-like refused in `Preprocessed::value` | `a_function_like_define_of_a_name_the_judge_reads_is_refused` | killed |
| r-fn-definition | function-like refused in the 33-macro reader | `a_function_like_register_macro_is_refused_by_name` | killed |
| r-fn-candidates | function-like refused in `candidates` | `a_function_like_define_of_a_name_the_judge_reads_is_refused` | killed |
| r-fn-blank | `NAME (x)` stays object-like | same | killed |
| c-order | `check pinout` admits the topology before reading the ELF | `a_topology_refusal_is_reported_beside_an_unreadable_elf` | killed |

Not mutable to a kill: the `ODR` level rule's `low` branch. Every `gpio` entry the check admits is a chip select at `high` (`GpioNotJudged`), so a mutant that expects `1` always is equivalent on every admissible topology (README, "What it cannot see").

## The IR half (after `AET-WO-4240e` merged): 23 of 23 killed

| id | guard | test | result |
|---|---|---|---|
| ir-adc1-first | the sense takes an ADC1 pin first (S2) | `sense_pin::the_sense_takes_an_adc1_pin_before_a_lower_adc3_only_pin` | killed |
| ir-sense-top | `SolvedSense.top_ohms` is the declared top | same | killed |
| pp-osc-dup | an oscillator cited twice | `power::a_malformed_mcu_ardupilot_column_is_refused` | killed |
| pp-osc-empty | no oscillator cited | same | killed |
| pp-osc-lookup | `McuArduPilot::oscillator` by exact Hz | `power::the_mcu_row_holds_its_flash_and_ardupilot_s_names_and_kernels` | killed |
| h-osc-select | hwdef picks the topology's oscillator | `hwdef_tier1::a_16_mhz_board_generates_and_an_uncited_oscillator_is_refused` | killed (first run, against `generate`'s 16 MHz test, **survived**: Sentinel-01's SPI1 runs at 80 MHz on both; the refusal of an uncited 8 MHz board kills it) |
| h-batt-scale | the quotient's numerator is top + bottom | `a_solved_sense_writes_the_battery_monitor` | killed |
| h-batt-type | `HAL_BATT_MONITOR_DEFAULT 3` | same | killed |
| h-batt-adcoff | no `HAL_USE_ADC FALSE` beside a sense | same | killed |
| h-divider-exact | the float bound is exclusive of 2^24 | `a_divider_with_no_exact_float_quotient_is_refused` | killed |
| h-divider-zero | a zero bottom | same | killed |
| h-sense-twice | `SenseTwice` | `two_battery_senses_are_refused` | killed |
| p-batt-pin | `HAL_BATT_VOLT_PIN` is the channel | `the_battery_monitor_is_judged_against_the_solved_sense` | killed |
| p-batt-scale | the scale is equal, not merely at least | same | killed |
| p-batt-type | the type is 3 | same | killed |
| p-batt-adcoff | `AdcOff` | same | killed |
| p-batt-row-scale | the analog row has no multiplier | same | killed |
| p-batt-row-analog | the analog row's pin number | same | killed |
| p-batt-unexpected | `UnexpectedBattMonitor` | `a_battery_monitor_with_no_adc1_sense_is_named` | killed |
| p-batt-adc1 | the sense's ADC is ADC1 | same | killed |
| p-fraction-frac | a decimal's denominator | `the_battery_monitor_is_judged_against_the_solved_sense` | killed |
| p-notanalog | `NotAnalog` on any non-analog mode | `check::a_battery_sense_pin_that_is_not_analog_is_named` | killed |
| p-sense-allow | the allow-list may not name a sense pin | same | killed |
