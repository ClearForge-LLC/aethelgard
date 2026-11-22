| function | at | ST-vocabulary literals | samples |
|---|---|---|---|
| `judge_boot0_pull_down` | `bench.rs:76` | 3 | `unplugged, BOOT0 sits at up to {floating`, `a driver at {high} mV is below BOOT0's V`, `driving BOOT0 high at {vdd_max} mV sourc` |
| `budget` | `clock.rs:196` | 10 | `the HSE is {hz_num}/{hz_den} Hz, outside`, ``{part}`'s critical gain is {gm_crit_nan`, `the HSE's budget is +{fast_ppm}/-{slow_p` |
| `refuse` | `clock.rs:326` | 2 | `DS12117 Rev 10 Table 33 rev Y p. 125 = T`, `DS12117 Rev 10 Table 31 rev Y p. 123 = T` |
| `judge_tree` | `clock.rs:607` | 12 | `RCC_PLLCKSELR.PLLSRC is not HSE (10)`, `RCC_PLLCFGR.PLL1FRACEN is 1`, `RCC_PLLCKSELR` |
| `foreign_clock_values` | `lib.rs:1181` | 8 | `USART1`, `USART6`, `USART2` |
| `ccr_shaped` | `lib.rs:1449` | 13 | ``acknowledge_unwitnessed` names register`, `serial port `{id}` names `{peripheral}`:`, `serial port `{id}` names peripheral `{pe` |
| `validate_motor_outputs` | `lib.rs:4629` | 1 | `the PLL words` |
| `reset` | `motor.rs:274` | 1 | `HSI` |
| `from_rcc` | `motor.rs:361` | 7 | `HSI`, `CSI`, `HSE` |
| `pll1_p` | `motor.rs:445` | 11 | `RCC_PLLCKSELR.PLLSRC`, `RCC_PLLCKSELR.DIVM1`, `RCC_PLL1DIVR.DIVN1` |
| `from_values` | `motor.rs:545` | 5 | `{field} is {value}: RM0433 Rev 9 calls i`, `HSE clocks the system or PLL1, and the s`, `RCC_CFGR.SWS names {clock} as the system` |
| `paths` | `off_domain.rs:218` | 1 | `PH0` |
| `new` | `off_domain.rs:512` | 2 | `PA13`, `PA14` |
| `<item level>` | `pin_devices.rs:0` | 5 | `DS12117 Rev 10 Table 50 p. 139 (rev Y) =`, `PA13`, `PA15` |
| `drive_levels` | `pin_devices.rs:1231` | 1 | ``{pin}` has no internal pull and no stra` |
| `pull_tolerated` | `pin_devices.rs:1489` | 3 | `the pull to `{}` holds `{}` at up to {v}`, `the pull holds `{pin}` at up to {v} mV; `, ``{pin}` has no held I/O structure (DS121` |
| `validate_solved` | `pin_devices.rs:2044` | 2 | `ADC1`, `ADC3` |
| `choose_channel` | `pin_devices.rs:2128` | 1 | `ADC2` |
| `mcu_ardupilot` | `power_parts.rs:1370` | 1 | ``kind.mcu.ardupilot.spi_kernel_hz` is ke` |
| `name` | `serial.rs:167` | 9 | `USART1`, `USART2`, `USART3` |
| `kernel_hz` | `serial.rs:201` | 2 | `USART1`, `USART6` |
| `solve_brr` | `serial.rs:220` | 1 | `RM0433 Rev 9 §48.5.8, Tables 406 and 407` |
| `shape_error` | `serial.rs:798` | 2 | `the data bits and a parity bit make a wo`, `1 or 2 stop bits (RM0433 Rev 9 §48.8.2, ` |
| `of` | `silicon.rs:90` | 6 | `SPI1`, `SPI2`, `SPI3` |
| `i2c_kernel_hz` | `silicon.rs:351` | 4 | `I2C1`, `I2C2`, `I2C3` |
| `usart_kernel_hz` | `silicon.rs:741` | 8 | `USART1`, `USART6`, `USART2` |
| `<item level>` | `solve.rs:0` | 4 | `PA13`, `PA14`, `PC14` |
| `reserved_pins` | `solve.rs:27` | 3 | `PH0`, `PH1` |
| `analog_pads` | `solve.rs:42` | 7 | ``{claimant}` names pin `{pin}`, whose pa`, `ADC net `{net}`: the free ADC pins {} ca`, ``{net}` drives a pin device's input from` |
| `hse_clock` | `solve.rs:840` | 1 | `PLL1` |
| `spi_kernel` | `solve.rs:962` | 12 | `SPI1`, `SPI2`, `SPI3` |
| `i2c_kernel_hz` | `solve.rs:1033` | 4 | `I2C1`, `I2C2`, `I2C3` |
| `fmt` | `speeds.rs:106` | 4 | `its supply's minimum of {vdd_min_millivo`, `its declared {capacitance_femtofarads} f`, `its rate of {rate_hz} Hz is above {fmax_` |
| `fmt` | `vos.rs:44` | 2 | `DS12117 Rev 10 Table 111 (continued), p.`, `RM0433 Rev 9 Table 60, pp. 355-356` |
| `usart_family` | `vos.rs:291` | 9 | `USART1`, `USART6`, `USART2` |
| `generated_clocks` | `vos.rs:305` | 6 | `SPI1`, `SPI2`, `SPI3` |
| `judge_foreign` | `vos.rs:574` | 2 | `HSI`, `PLL1` |

37 functions, 175 literals
