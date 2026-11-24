# AET-WO-4410 research A (read-only; ArduPilot Copter-4.7.2, fb773567, ChibiOS d103983f)

Paths are relative to the pinned tree (fb773567, ChibiOS d103983f). H = libraries/AP_HAL_ChibiOS/hwdef.
HEX = modules/ChibiOS/os/hal/ports/STM32/STM32H7xx. Every quoted line was read from the tree.
Anything marked INFERRED is my reading, not a quoted line.

======================================================================
## 1. Battery monitor, voltage only

### 1.1 Pin-line grammar and what the script does with it
- Pin line is `P<port><n> <LABEL> <TYPE> [extra...]`: H/scripts/chibios_hwdef.py:2873-2886 `process_line`:
  - `a = shlex.split(line, posix=False)` ; `port = a[0][1]; pin = int(a[0][2:]); label = a[1]; type = a[2]; extra = a[3:]`.
  - `valid_type` (chibios_hwdef.py:2788-2793) patterns: `r'INPUT', r'OUTPUT', r'TIM\d+', r'USART\d+', r'UART\d+', r'ADC\d+', ...`
    so ADC1, ADC2, ADC3 all pass the grammar. The type is NOT restricted to ADC1.
- The type then picks the ADC (chibios_hwdef.py:2192-2202, `write_ADC_config`):
  - `if p.type.startswith('ADC1'): index = 0; chan = self.get_ADC1_chan(self.mcu_type, p.portpin)`
  - likewise `ADC2` (index 1, 2197) and `ADC3` (index 2, 2200); anything else: `self.error("Unknown ADC type %s" % p.type)` (2202).
- The channel comes from per-MCU maps. Missing pin: `self.error("Unable to find ADC1 channel for pin %s" % pin)` (chibios_hwdef.py:198; ADC2 211; ADC3 224).
- H753 maps (H/scripts/STM32H753xx.py):
  - `ADC1_map = {` at 1281. `"PC0" : 10` at 1291. In this table: PC1=11, PC2=12, PC3=13, PA2=14, PA3=15, PA0=16, PA1=17, PA4=18, PA5=19, PC4=4, PC5=8, PB0=9, PB1=5, PA6=3, PA7=7, PF11=2, PF12=6.
  - `ADC2_map = {` at 1303 (PC0=10 at 1312). `ADC3_map = {` at 1324.
  - ADC3_map entries: PF9=2, PF7=3, PF5=4, PF3=5, PF10=6, PF8=7, PF6=8, PF4=9, PH2=13, PH3=14, PH4=15, PH5=16. There is no PC0..PC3 in the H753 ADC3 map.
- The pin goes to analog mode: MODER `elif self.type.startswith('ADC'): v = "ANALOG"` (chibios_hwdef.py:335-336). `get_CR`/ODR code at 521-522 and 551-552 does the same for the other register paths.
- The generated header comment gets `ADC1_IN<chan>` (chibios_hwdef.py:617-618).

### 1.2 Exact pin-line text on H7 boards (real examples)
- Dominant text in libraries/AP_HAL_ChibiOS/hwdef/*/hwdef.dat (counts over all boards, `grep -rh BATT_VOLTAGE_SENS`):
  - 52x `PC0 BATT_VOLTAGE_SENS ADC1 SCALE(1)` ; 25x `PC3 ... ADC1 SCALE(1)` ; 18x `PA2 ...` ; 15x `PC1 ...`.
  - Variants: 5x `PC0  BATT_VOLTAGE_SENS ADC1` (no SCALE); `PC0 BATT_VOLTAGE_SENS ADC1 SCALE(10)` (Morakot hwdef.dat:78);
    `PF3 BATT_VOLTAGE_SENS ADC3 SCALE(1)` (NWBLUE_PROH757/hwdef.dat:105); `PC3 BATT_VOLTAGE_SENS ADC2 SCALE(1)` (SPRacingH7RF/hwdef.dat:78);
    `PF11 BATT_VOLTAGE_SENS ADC1 SCALE(1) ANALOG(14)`.
- H753 boards in the tree:
  - GPILOT_P1/hwdef.dat:104 `PA2 	BATT_VOLTAGE_SENS 	ADC1 		SCALE(1)` with
    :181 `define HAL_BATT_VOLT_PIN                    14` and :183 `define HAL_BATT_VOLT_SCALE                  12.02`
    (PA2 is ADC1 chan 14 in STM32H753xx.py:1295, so pin number = ADC1 channel).
  - Lectron-Pi5-H7/hwdef.dat:12 `MCU STM32H7xx STM32H753xx`, :13 `OSCILLATOR_HZ 16000000`, :288 `define HAL_BATT_MONITOR_DEFAULT 21`. It has no BATT_VOLTAGE_SENS pin line.
  - NucleoH753ZI/hwdef.dat:212 `PC4  BAT_VOLTAGE ADC1_INP14 ADC1 SCALE(11)`: label and type are mixed up in the file; not a model to copy.
- A typical H743 reference: CubeOrange/hwdef.dat:72 `PC0 BATT_VOLTAGE_SENS ADC1 SCALE(1)`, :77 `define HAL_BATT_MONITOR_DEFAULT 4`, :78 `define HAL_BATT_VOLT_PIN 10`, :82 `define HAL_BATT_VOLT_SCALE 11.0`.
  AEDROXH7, CORVON743V1 and BlitzH743Pro also pair `PC0` with `HAL_BATT_VOLT_PIN 10`.
- The label BATT_VOLTAGE_SENS has NO special handling in the generator. NOT FOUND in
  H/scripts/chibios_hwdef.py: the only labels that write defines are `VDD_5V_SENS` (2208-2210) and `FMU_SERVORAIL_VCC_SENS` (2211-2213).
  The label is just a convention. HAL_BATT_VOLT_PIN, HAL_BATT_VOLT_SCALE and HAL_BATT_MONITOR_DEFAULT are never emitted automatically;
  they must be written as explicit `define` lines. The only script that writes them is convert_betaflight_unified.py:352-366
  (`... BATT_VOLTAGE_SENS %s SCALE(1)`, `define HAL_BATT_VOLT_PIN %s`, `define HAL_BATT_VOLT_SCALE 11.0`, `define HAL_BATT_MONITOR_DEFAULT 4`).
- `define X v` in hwdef.dat is copied verbatim as `#define X v` into hwdef.h: chibios_hwdef.py:1007-1011 `if d.startswith('define '): ... f.write('#define %s\n' % d[7:])`.
  So the value is a raw C token. `12.02` works as a float literal.

### 1.3 SCALE(n) and the analog pin number
- Parser: `extra_value` chibios_hwdef.py:303-316. It takes the text between `NAME(` and `)` as a STRING (no `type=` given at 2203). `scale = p.extra_value('SCALE', default=None)`.
- Consumed only in `write_ADC_config` (chibios_hwdef.py:2244-2250):
  - `vdd = self.get_config('STM32_VDD', default='330U')` (2241); trailing `U` stripped (2242-2243); `vdd = float(vdd) * 0.01` (2244).
  - `scale_str = '%.2f/4096' % vdd` (2247) gives `3.30/4096`. `if scale is not None and scale != '1': scale_str = scale + '*' + scale_str` (2248-2249).
  - It is written as the `scaling` field of a `HAL_ANALOG_PINS` table row (2246-2251): `{ chan, analog, scale_str }, /* portpin label */`. The row struct is `pin_info {uint8_t channel; uint8_t analog_pin; float scaling;}` (libraries/AP_HAL_ChibiOS/AnalogIn.h:113-117).
  - Example output for SCALE(2): `{ 12, 12, 2*3.30/4096 }`. `SCALE(1)` and a missing SCALE both give `3.30/4096`.
  - SCALE is a per-pin multiplier of the ADC count-to-volt factor. It is not the battery divider.
- `STM32_VDD` is a hwdef.dat key (e.g. `STM32_VDD 330U` in MambaF405v2/hwdef.dat:17). There is a `define STM32_VDD 330U` in common/stm32f47_mcuconf.h:248 but none in stm32h7_mcuconf.h (grep found none).
  The generator's default is 3.30 V. `%.2f` means two decimals, so a VDD like 3.3 V is exact.
- Analog pin number (what `HAL_BATT_VOLT_PIN` must name), chibios_hwdef.py:2204-2207:
  - `analog = p.extra_value('ANALOG', type=int, default=chan) # default to ADC channel if not specified`.
  - The number is the ADC channel of that ADC (the ADC1, ADC2 or ADC3 named in the line). There is NO offset for ADC3.
  - Proof: NWBLUE_PROH757/hwdef.dat:105 `PF3 ... ADC3` with :179 `define HAL_BATT_VOLT_PIN  5`. H757 ADC3_map has PF3 = 5 (STM32H757xx.py:1313).
  - `ANALOG(n)` overrides the number.
  - The set is shared across ADC1/2/3. `analogset = {252, 253, 254}` (2186, "reserved values for VSENSE, VREF and VBAT in H7") and 2207-2208 `if analog in analogset: self.error("Duplicate analog pin %u" % analog)`.
    So an ADC3 pin whose channel equals a used ADC1 channel (e.g. PF3 = 5 vs PB1 = 5) errors unless `ANALOG(n)` is given.
  - The table is `#define HAL_ANALOG_PINS` (chibios_hwdef.py:2245). ADC2 gives `HAL_ANALOG2_PINS` (2257) plus `STM32_ADC_DUAL_MODE TRUE` (2256). ADC3 gives `HAL_ANALOG3_PINS` plus `#define STM32_ADC_USE_ADC3 TRUE` (2266-2267).
- Pin-number range: parameter `@Range: -1 127` (AP_BattMonitor_Analog.cpp:62 for `VOLT_PIN`, :71 for `CURR_PIN`). The row struct stores `uint8_t`.

### 1.4 Consumption in AP_BattMonitor (file:line)
libraries/AP_BattMonitor/AP_BattMonitor_Analog.cpp:
- :30 `#if defined(HAL_BATT_VOLT_PIN)`
- :32-34 `#ifndef HAL_BATT_VOLT_SCALE` / `#error "You must specify HAL_BATT_VOLT_SCALE when you specify HAL_BATT_VOLT_PIN"`. Defining the pin without the scale is a compile error.
- :35 `# define AP_BATT_VOLT_PIN                  HAL_BATT_VOLT_PIN`
- :36 `# define AP_BATT_VOLTDIVIDER_DEFAULT       HAL_BATT_VOLT_SCALE`
- :37-39 default when undefined on ChibiOS: `# define AP_BATT_VOLT_PIN 4` / `# define AP_BATT_VOLTDIVIDER_DEFAULT 10.1f`.
- :65 `AP_GROUPINFO("VOLT_PIN", 1, AP_BattMonitor_Analog, _volt_pin, AP_BATT_VOLT_PIN)`; :80 `AP_GROUPINFO("VOLT_MULT", 3, ..., _volt_multiplier, AP_BATT_VOLTDIVIDER_DEFAULT)`. These are the parameter defaults (BATT_VOLT_PIN, BATT_VOLT_MULT).
- Type is a float parameter. The macro is pasted as a C token, so `12.02` is a double literal narrowed to float and `11` is an int.
- No-current case: current pin default on ChibiOS is `AP_BATT_CURR_PIN 3` and `AP_BATT_CURR_AMP_PERVOLT_DEFAULT 17.0f` (:23-24). With voltage-only type 3, current is never read.
- AP_BattMonitor.cpp:568-569: `#ifdef HAL_BATT_MONITOR_DEFAULT` / `_params[0]._type.set_default(int8_t(HAL_BATT_MONITOR_DEFAULT));`. Without the define the default is NONE: AP_BattMonitor_Params.cpp:58 `AP_GROUPINFO_FLAGS("MONITOR", 1, ..., _type, int8_t(AP_BattMonitor::Type::NONE), AP_PARAM_FLAG_ENABLE)`.
- Type values, libraries/AP_BattMonitor/AP_BattMonitor_Params.h:17-18: `ANALOG_VOLTAGE_ONLY = 3`, `ANALOG_VOLTAGE_AND_CURRENT = 4`.
  IMPORTANT for the WO: `HAL_BATT_MONITOR_DEFAULT 4` (the value on ~111 boards and what convert_betaflight writes) is voltage AND current. For voltage-only the type is 3. Most boards using 4 also define a current pin.
  With 4 and no current pin defined, the constructor builds a current channel on the default pin 3 (`hal.analogin->channel(_curr_pin)` at :143). INFERRED: use 3 for voltage only.
- Constructor requirement: AP_BattMonitor_Analog.cpp:134-139 `if (_params._type != ...ANALOG_CURRENT_ONLY) { _volt_pin_analog_source = hal.analogin->channel(_volt_pin); if (... == nullptr) AP_BoardConfig::config_error("No analog voltage channel for battery %d", ...)`.
- Sibling defines for battery 2: `HAL_BATT2_VOLT_PIN`, `HAL_BATT2_VOLT_SCALE`, `HAL_BATT2_MONITOR_DEFAULT` (AP_BattMonitor_Analog.cpp:119-120 etc.; AP_BattMonitor.cpp:571-572).

### 1.5 How voltage is computed
- AP_BattMonitor_Analog.cpp, `read()` (:153-160): `_state.healthy = _volt_pin_analog_source->set_pin(_volt_pin);` then
  `_state.voltage = (_volt_pin_analog_source->voltage_average() - _volt_offset) * _volt_multiplier;`
  The parameter is described at :78 as `(pin_voltage * VOLT_MULT)`. So HAL_BATT_VOLT_SCALE = (R_top + R_bottom) / R_bottom exactly, when the pin line has SCALE(1) or no SCALE. The offset `VLT_OFFSET` defaults to 0 (:101).
- `voltage_average()` is `return _pin_scaler() * read_average();` (libraries/AP_HAL_ChibiOS/AnalogIn.cpp:200-203).
  `_pin_scaler()` (AnalogIn.cpp:169-198) finds the `scaling` of the matching `pin_config`, `pin_config_2` or `pin_config_3` row and returns it. The fallback is `VOLTAGE_SCALING` (3.3f/4095, AnalogIn.cpp:44).
- Count input: H7 uses 16-bit transfers and divides by 16 to match the 12-bit scale factors:
  - AnalogIn.cpp:99-105 `#if defined(ADC_CFGR_RES_16BITS)` / "on H7 we use 16 bit ADC transfers, giving us more resolution. We need to scale by 1/16 to match the 12 bit scale factors in hwdef.dat" / `#define ADC_BOARD_SCALING (1.0/16)`.
  - AnalogIn.cpp:561-563 `adcgrpcfg[index].cfgr = ADC_CFGR_CONT | ADC_CFGR_RES_16BITS;`
  - AnalogIn.cpp:723-726 `c->_add_value(buf_adc[i] * ADC_BOARD_SCALING, _board_voltage);`
  - So pin_V = raw16/16 * SCALE * VDD/4096, with VDD = 3.30 V from the generator's `%.2f/4096` (full scale 4096, not 4095).
  - Sample time on H7: AnalogIn.cpp:583 `smpr[chan/10] |= ADC_SMPR_SMP_384P5 ...`. ADC kernel clock is `STM32_ADCSEL_PLL3_R_CK` (common/stm32h7_mcuconf.h:348).
- Rounding/format: no decimal-digit limit on HAL_BATT_VOLT_SCALE. It is a C float token. The only generated rounding is VDD `%.2f` (chibios_hwdef.py:2247). SCALE(n) text passes verbatim as a C expression (`9.33`, `2`, `10`).

### 1.6 HAL_USE_ADC
- chibios_hwdef.py never emits `HAL_USE_ADC TRUE`. The default comes from halconf.h:
  H/common/halconf.h:60-62 `#if !defined(HAL_USE_ADC) || defined(__DOXYGEN__)` / `#define HAL_USE_ADC                 TRUE`.
- The only generator mention is for the bootloader build: chibios_hwdef.py:1192 `#define HAL_USE_ADC FALSE` (inside the `is_bootloader_fw()` block, 1188-1195).
- On H7, mcuconf enables the drivers unconditionally: common/stm32h7_mcuconf.h:420 `#define STM32_ADC_USE_ADC12                 TRUE`; :422-424 `#ifndef STM32_ADC_USE_ADC3` / `#define STM32_ADC_USE_ADC3                  TRUE` (except H750). chibios_hwdef.py:2514-2516 also does `self.periph_list.append('ADC3')` on H7 "to get MCU temperature and reference voltage".
- If hwdef.dat says `define HAL_USE_ADC FALSE` (which becomes `#define HAL_USE_ADC FALSE` in hwdef.h, before halconf.h's guard): the real driver is compiled out.
  - AnalogIn.cpp:22 `#if HAL_USE_ADC == TRUE && !defined(HAL_DISABLE_ADC_DRIVER)` and HAL_ChibiOS_Class.cpp:88-92 `static ChibiOS::AnalogIn analogIn; #else static Empty::AnalogIn analogIn;`.
  - The Empty driver returns a dummy source: AP_HAL_Empty/AnalogIn.cpp:35-36 `AnalogIn::channel(int16_t n) { return NEW_NOTHROW AnalogSource(1.11); }`, with `voltage_average()` = `5.0f * _v / 1024.0f` (:13-14).
  - So the battery monitor would silently read a constant, ~5.42 V times the multiplier, never a real sensor. A `define HAL_USE_ADC FALSE` conflicts with having an ADC pin and must not be emitted when a sense pin exists.
  - Need not set TRUE: nothing to add.

### 1.7 ADC3-only pins and "_C" pins
- ADC3-only pin (e.g. PF3 on H753, PF3=5 at STM32H753xx.py:1324+): works with type `ADC3`. It goes to `HAL_ANALOG3_PINS` (chibios_hwdef.py:2265-2273); AnalogIn.cpp:80-94 `#if defined(HAL_ANALOG3_PINS) || HAL_WITH_MCU_MONITORING` builds `pin_config_3[] = { HAL_ANALOG3_PINS HAL_MCU_MONITORING_PINS}`. The MCU-monitor rows use pins 252-254 (AnalogIn.cpp:86 `{ADC3_VBAT4_CHAN, 252, 3.30/4096}, ...`).
  - The pin number is the ADC3 channel unless `ANALOG(n)` is given (see 1.3). `set_pin` searches all three tables (AnalogIn.cpp:241-257).
  - Example in tree: NWBLUE_PROH757/hwdef.dat:105 `PF3 BATT_VOLTAGE_SENS         ADC3 SCALE(1)   # VSYS divider`, :179 `define HAL_BATT_VOLT_PIN  5`.
- ADC2: H753 has an ADC2_map (STM32H753xx.py:1303) and `ADC2` is accepted, but it turns on dual mode (`STM32_ADC_DUAL_MODE TRUE`, `ADC12_CCR_DUAL`, chibios_hwdef.py:2254-2258) and inserts dummy rows (2220-2237). Avoid it for one pin; use ADC1.
- "_C" pins (PC2_C / PC3_C): NOT FOUND in any H/scripts/*.py (grep `_C\b` in chibios_hwdef.py and STM32H753xx.py returned nothing) and not in any hwdef.dat pin line. The only mention is a comment, NWBLUE_PROH757/hwdef.dat:102-104:
  `# Current and 3V3 sense arrive on the module's PC2_C / PC3_C direct-analog pads.` / `# PC3 has an ADC3 mapping, PC2 does not, so the current sense uses ADC1 ch12 -` / `# the PC2_C to PC2 analog switch is closed at reset, so the pad still reaches it.`
  ArduPilot therefore uses the ordinary pin name PC2/PC3 (ADC1 chan 12/13 in the H753 map; analog switch default closed per that comment); no `_C` token exists in the generator.

======================================================================
## 2. SWD lines

### 2.1 Text in the tree
- Dominant exact text (all boards, uncommented): 196x `PA13 JTMS-SWDIO SWD` and 196x `PA14 JTCK-SWCLK SWD` (plus 11x each with a trailing space). Count of lines after whitespace normalisation, from `grep -hE "^[[:space:]]*#?[[:space:]]*PA13\b" libraries/AP_HAL_ChibiOS/hwdef/*/hwdef.dat | sort | uniq -c`.
- Commented variants: 7x `#PA13 JTMS-SWDIO SWD`, 1x `# PA13 JTMS-SWDIO SWD` (the same for PA14).
- Other uses of PA13/PA14 (non-SWD boards): 8x PA13 as `LED_GREEN`/`LED1`/`OSD_CS` etc., 8x PA14 as `LED_BLUE`/`FLASH_CS`/`CAM_C` etc.
- Restricted to the 118 hwdef.dat files whose own `MCU` line starts `STM32H7`: 103x `PA13 JTMS-SWDIO SWD`, 103x `PA14 JTCK-SWCLK SWD`, 1x commented. Boards without an own uncommented line (e.g. CubeOrange, NucleoH753ZI) include another file or leave it out; NucleoH753ZI/hwdef.dat:124-125 has them commented out.
- Pull tokens on SWD lines: NONE. `grep -rhE "SWD.*(PULL|FLOAT)" */hwdef.dat` returned nothing. Pulls come from the generator (2.3).
- Lectron-Pi5-H7 (an H753 board, 16 MHz): hwdef.dat:36-38 `# pins for SWD debugging` / `PA13 JTMS-SWDIO SWD` / `PA14 JTCK-SWCLK SWD`.

### 2.2 How the label becomes an AF
- `valid_type` accepts `SWD` (chibios_hwdef.py:2792 `r'SWD'`).
- `af_labels` (chibios_hwdef.py:33): `['USART', 'UART', 'SPI', 'I2C', 'SDIO', 'SDMMC', 'OTG', 'JT', 'TIM', 'CAN', 'QUADSPI', 'OCTOSPI', 'ETH', 'MCO']`. The label `JTMS-SWDIO` starts with `JT`.
- `get_alt_function` (chibios_hwdef.py:151-179): for the first `label in af_labels` with `function.startswith(label)`, it builds `s = pin + ":" + function` and returns `alt_map[s]` (173-178); a missing key is `self.error("Unknown pin function %s for MCU %s" ...)` (177).
- H/scripts/STM32H753xx.py:247 `"PA13:JTMS-SWDIO"   	:	0,` and :249 `"PA14:JTCK-SWCLK"   	:	0,` (EVENT-OUT is AF15 at 246 and 248). So AF0 for both. process_line applies it: `af = self.get_alt_function(self.mcu_type, a[0], label); if af is not None: p.af = af` (chibios_hwdef.py:2851-2853).
- The type string is not used to find the AF, only the label. The label MUST be exactly `JTMS-SWDIO` / `JTCK-SWCLK`; `SWDIO` alone would fail the `JT` prefix test (no AF, so MODER INPUT).

### 2.3 Registers the line produces (H7)
- MODER: `get_MODER_value` (chibios_hwdef.py:329-333): `if self.af is not None: v = "ALTERNATE"`, so `PIN_MODE_ALTERNATE(13U)`/`(14U)`.
- AFIO: `get_AFIO_value` (466-472) returns `self.af` = 0 so `PIN_AFIO_AF(13U, 0U)`; written to AFRH since pin >= 8.
- PUPDR: `get_PUPDR_value` (387-424): default `v = 'FLOATING'`; then
  - 407-409 `if (self.type.startswith('SWD') and 'SWDIO' in self.label): v = "PULLUP"`
  - 411-413 `if (self.type.startswith('SWD') and 'SWCLK' in self.label): v = "PULLDOWN"`
  - any extra token among FLOATING/PULLUP/PULLDOWN overrides (417-419, `for e in self.extra: if e in values: v = e`).
  So PA13 = pull-up, PA14 = pull-down (the STM32 reset default as well; INFERRED, RM not checked here).
- OTYPER: `get_OTYPER_value` (349-357) PUSHPULL unless an extra token says otherwise. OSPEEDR: `get_OSPEEDR_value` (365-370) SPEED_MEDIUM default. ODR: `get_ODR_value` (451-461) HIGH default. These come from the same loop that writes every pin (chibios_hwdef.py:2607-2622, `modefunc = getattr(p, "get_" + vtype)`).

### 2.4 Unlisted pins
- Default for every pin not in hwdef.dat: `self.default_gpio = ['INPUT', 'FLOATING']` (chibios_hwdef.py:47). Pins are pre-created as `generic_pin(port, pin, None, self.default_gpio[0], self.default_gpio[1:], ...)` (:145, inside `setup_mcu_type_defaults` at 131-146).
  Result: MODER = INPUT (the `else: v = "INPUT"` branch of get_MODER_value), PUPDR = FLOATING, AF 0, label None.
- `DEFAULTGPIO` overrides it: chibios_hwdef.py:2877-2879 `if a[0] == 'DEFAULTGPIO': self.default_gpio = a[1:]` (e.g. Aeromind6X/hwdef.dat:28 `DEFAULTGPIO OUTPUT LOW PULLDOWN`).
  INFERRED ordering caveat: the pre-creation at 145 runs when the `MCU` line is processed (2894-2897), so `DEFAULTGPIO` only takes effect if it precedes the `MCU` line.
- The `UNDEF` path resets a pin to the same `'INPUT', []` (chibios_hwdef.py:2943).

### 2.5 A define to disable SWD/debug pins
- NOT FOUND. Searched: `grep -rn "SWD\|JTMS\|SWCLK\|DBGMCU"` over libraries/AP_HAL_ChibiOS/*.cpp *.h and hwdef/common/*.h *.c. The only DBGMCU hit is hwdef/common/stm32_util.c:574 (`const uint16_t revid = (*(uint32_t *)DBGMCU_BASE) >> 16;`, a revision read). No HAL_ define turns SWD off. Leaving the PA13/PA14 lines out only leaves them as floating inputs (2.4), which on the STM32 does NOT switch off the debug port; the pins reset into AF0 in hardware, and ArduPilot's MODER write puts them in INPUT. INFERRED, not a quoted line.

======================================================================
## 3. 16 MHz clock tree (STM32H753)

### 3.1 Which mcuconf applies to H753
- The generator writes `#define STM32H753xx` via `f.write('#define %s\n\n' % mcu_subtype)` (chibios_hwdef.py:883) and `STM32_HSECLK <OSCILLATOR_HZ>U` (:885 `f.write('#define STM32_HSECLK %sU\n\n' % self.get_config('OSCILLATOR_HZ'))`). The MCU line `MCU STM32H7xx STM32H753xx` gives the subtype from `get_config('MCU', 1)` (:879-882 also writes `STM32H753_MCUCONF` style names).
- H/common/mcuconf.h:56-61:
  - `#elif defined(STM32H730xx) || defined(STM32H723xx)` -> `#include "stm32h7_type2_mcuconf.h"`
  - `#elif defined(STM32H7A3xx)` -> `#include "stm32h7_A3_mcuconf.h"`
  - `#elif defined(STM32H7)` -> `#include "stm32h7_mcuconf.h"`
  H753 matches the third: stm32h7_mcuconf.h. (That `STM32H7` is defined for H753 comes from ChibiOS's registry; I did not locate the exact define line, INFERRED from the H753 boards compiling this way.)
- ChibiOS side: HEX/hal_lld.h:146-157: `#if defined(STM32H723xx) || ... STM32H730xx` -> `hal_lld_type2.h`; `#elif defined(STM32H7A3xx) ...` -> `hal_lld_type3.h`; `#else` -> `#include "hal_lld_type1.h"`. H753 is TYPE1. So the file the existing table cites as "CHIBIOS_HAL_LLD_TYPE1" is HEX/hal_lld_type1.h.
- stm32h7_mcuconf.h:16-22 `#if !defined(HAL_CUSTOM_MCU_CLOCKRATE) || HAL_CUSTOM_MCU_CLOCKRATE <= 400000000` / `#define STM32_ENFORCE_H7_REV_XY`. This selects the rev-XY limits in hal_lld_type1.h:209-237: `STM32_PLLVCO_MAX 836000000` (228), `STM32_SPI123_MAX 133000000` (233), `STM32_SPI456_MAX 100000000` (234), `STM32_ADCCLK_MAX 36000000` (235).
  `STM32_SPI123_MAX`/`STM32_SPI456_MAX` are defined only; no `#if`/`#error` in hal_lld_type1.h uses them (grep over the file shows only lines 197, 202, 233, 234).

### 3.2 stm32h7_mcuconf.h branches (line = file line)
Common to every HSE value:
- 104 `#define STM32_HSIDIV                        STM32_HSIDIV_DIV1` (HSI kernel/PLL input 64 MHz, `STM32_HSI_OSC 64000000` at hal_lld_type1.h:244).
- 109 `#define STM32_VOS                           STM32_VOS_SCALE1`.
- 265-280 PLL enables (PLL1 265-270, PLL2 271-276 incl. R at 274, PLL3 277-281): `STM32_PLL3_ENABLED TRUE`, `STM32_PLL3_P_ENABLED FALSE` (278), PLL3 Q and R enabled.
- 287 `#define STM32_SW                            STM32_SW_PLL1_P_CK`.
- 332 `#define STM32_SPI45SEL                      STM32_SPI45SEL_PLL2_Q_CK`
- 333 `#define STM32_SPI123SEL                     STM32_SPI123SEL_PLL1_Q_CK`
- 345 `#define STM32_SPI6SEL                       STM32_SPI6SEL_PLL2_Q_CK`
- 343 `STM32_USART16SEL ... PLL2_Q_CK`, 344 `STM32_USART234578SEL ... PLL2_Q_CK`, 348 `STM32_ADCSEL STM32_ADCSEL_PLL3_R_CK`, 340 `STM32_USBSEL ...PLL3_Q_CK` (340).
- 259-261: `#ifndef STM32_PLLSRC` / `#define STM32_PLLSRC                        STM32_PLLSRC_HSE_CK`.

DIVM branches (lines 113-166):
| HSECLK | lines | PLL1 M / PLL2 M / PLL3 M | other |
|---|---|---|---|
| 0U | 113-122 | 4 / 8 / 4 (lines 117,118,119) | `STM32_HSE_ENABLED FALSE` (115), `STM32_HSI_ENABLED TRUE` (116), `STM32_PLLSRC STM32_PLLSRC_HSI_CK` (120), MCO1SEL HSI (121), `STM32_CKPERSEL STM32_CKPERSEL_HSI_CK` (122) |
| 8000000U | 124-131 | 1 / 1 / 2 | |
| 16000000U | 132-138 | 2 / 2 / 4 (136,137,138) | `STM32_HSE_ENABLED TRUE` (134), `STM32_HSI_ENABLED FALSE` (135) |
| 24000000U | 140-146 | 3 / 2 / 6 | |
| 25000000U | 148-154 | 2 / 5 / 5 | |
| 32000000U | 156-162 | 4 / 4 / 8 | |
| else | 164-165 | `#error "Unsupported HSE clock"` | |

N/P/Q/R branches (lines 168-253):
- HSECLK == 0U (168-182): PLL1 N50 (170) P2 (171) Q10 (172) R2 (173); PLL2 N45 (175) P2 (176) Q5 (177) R8 (178); PLL3 N15 (180) Q5 (181) R8 (182). PLL3 has no P (disabled).
- 8, 16, 32 MHz (184-207), shared: PLL1 N100 (194, or N120/Q12 if `HAL_CUSTOM_MCU_CLOCKRATE == 480000000`, 187-189) Q10 (195) P2 (197) R2 (198); PLL2 N75 (200) P3 (201) Q6 (202) R3 (203); PLL3 N72 (205) Q6 (206) R9 (207).
- 24 MHz (209-234): PLL1 N100/P2/Q10/R2 (221-225); PLL2 N50 (227) P3 Q6 R3; PLL3 N72 Q6 R9.
- 25 MHz (236-253): PLL1 N64 (240) P2 Q10 R2; PLL2 N120 (245) P3 Q6 R3; PLL3 N48 (250) Q5 R8.

### 3.3 ChibiOS formulas (HEX/hal_lld_type1.h)
- :1334 `#define STM32_HSICLK            STM32_HSI_OSC` (inside `#if STM32_HSI_ENABLED` at 1333); :1518 `#define STM32_HSI_CK                (STM32_HSICLK / 1U)` (HSIDIV_DIV1 branch, 1517); :1410 `#define STM32_HSE_CK            STM32_HSECLK`.
- :1543 `#define STM32_PLLCLKIN              STM32_HSE_CK` (HSE source); :1546 `#define STM32_PLLCLKIN              STM32_HSI_CK` (HSI source).
- :1561 `#define STM32_PLL1_REF_CK           (STM32_PLLCLKIN / STM32_PLL1_DIVM_VALUE)`; :1592 PLL2_REF_CK likewise; :1623 PLL3_REF_CK likewise.
- Input range check: PLLIN 1..16 MHz (:221 `STM32_PLLIN_MIN 1000000`, :225 `STM32_PLLIN_MAX 16000000`; checks :1569, :1600, :1631). VCO 150..836 MHz (:226, :228; checks :1806, :1827, :1848).
- :1801 `#define STM32_PLL1_VCO_CK           (STM32_PLL1_REF_CK * STM32_PLL1_DIVN_VALUE)`; :1822 PLL2_VCO_CK likewise.
- :1917 `#define STM32_PLL1_Q_CK             (STM32_PLL1_VCO_CK / STM32_PLL1_DIVQ_VALUE)`; :1934 PLL2_Q_CK likewise; :1951 PLL3_Q_CK likewise.
- SPI clocks:
  - :2589 `#if (STM32_SPI123SEL == STM32_SPI123SEL_PLL1_Q_CK) || defined(__DOXYGEN__)`; :2593 `#define STM32_SPI1CLK               STM32_PLL1_Q_CK`; :2598 `STM32_SPI2CLK STM32_PLL1_Q_CK`, :2603 `STM32_SPI3CLK STM32_PLL1_Q_CK`.
  - SPI4/SPI5: :2636 `#define STM32_SPI4CLK               STM32_PLL2_Q_CK` (:2637 `STM32_SPI5CLK STM32_PLL2_Q_CK`) under `STM32_SPI45SEL == STM32_SPI45SEL_PLL2_Q_CK` (:2635).
  - :2661 `#define STM32_SPI6CLK               STM32_PLL2_Q_CK`, under `#elif STM32_SPI6SEL == STM32_SPI6SEL_PLL2_Q_CK` (:2660).

### 3.4 Computed values
OSCILLATOR_HZ 0 (HSI 64 MHz, DIV1), cross-checks the table's current values:
- PLL1: 64 / 4 = 16 MHz ref; x50 = 800 MHz VCO; Q: /10 = 80 MHz. So SPI1 = SPI2 = SPI3 = 80,000,000 Hz (matches the table).
- PLL2: 64 / 8 = 8 MHz ref; x45 = 360 MHz VCO; Q: /5 = **72 MHz**. SPI4 = SPI5 = SPI6 = 72,000,000 Hz (SPI4 matches the table; SPI5 and SPI6 are the same: 72,000,000).
- Also: PLL1 P = 800/2 = 400 MHz system clock; PLL3: 16 MHz ref x15 = 240 MHz VCO, R /8 = 30 MHz (ADC kernel).

OSCILLATOR_HZ 16000000 (lines: M 136-138, N/P/Q/R 194-207):
- PLL1: 16 / 2 = 8 MHz ref; x100 = 800 MHz VCO (<= 836); Q /10 = 80 MHz. SPI1 = SPI2 = SPI3 = **80,000,000 Hz**.
- PLL2: 16 / 2 = 8 MHz ref; x75 = 600 MHz VCO; Q /6 = 100 MHz. SPI4 = SPI5 = SPI6 = **100,000,000 Hz**.
- PLL1 P = 800/2 = 400 MHz (SYSCLK); PLL3: 16 / 4 = 4 MHz ref; x72 = 288 MHz VCO; Q /6 = 48 MHz (USB); R /9 = 32 MHz (ADC kernel, under STM32_ADCCLK_MAX 36 MHz).
- All reference/VCO values are inside the ChibiOS checks. SPI4/5/6 at 100,000,000 Hz sits exactly at `STM32_SPI456_MAX 100000000` (hal_lld_type1.h:234) but no check uses that macro. The SPI4/5/6 kernel clock changes from 72 MHz to 100 MHz with the crystal, while SPI1-3 stay at 80 MHz.
For comparison, 8 MHz: the same 80 / 100 MHz; 24 MHz: PLL1 24/3 x100 / 10 = 80, PLL2 24/2 x50 / 6 = 100; 25 MHz: PLL1 25/2 x64 / 10 = 80, PLL2 25/5 x120 / 6 = 100; 32 MHz: 80 / 100.

### 3.5 Status of the table's citations
The table's "CHIBIOS_HAL_LLD_TYPE1" is HEX/hal_lld_type1.h (selected by HEX/hal_lld.h:146-157). Re-read at the tag:
- 244: `#define STM32_HSI_OSC                   64000000` : as claimed.
- 1518: `#define STM32_HSI_CK                (STM32_HSICLK / 1U)` : as claimed (HSIDIV DIV1).
- 1546: `#define STM32_PLLCLKIN              STM32_HSI_CK` : as claimed.
- 1561: `#define STM32_PLL1_REF_CK           (STM32_PLLCLKIN / STM32_PLL1_DIVM_VALUE)` : as claimed. (PLL2_REF_CK is 1592 and is not in the cited list.)
- 1801: `#define STM32_PLL1_VCO_CK           (STM32_PLL1_REF_CK * STM32_PLL1_DIVN_VALUE)` : as claimed. (PLL2_VCO_CK is 1822.)
- 1917: `#define STM32_PLL1_Q_CK             (STM32_PLL1_VCO_CK / STM32_PLL1_DIVQ_VALUE)` : as claimed. (PLL2_Q_CK is 1934.)
- 2593: `#define STM32_SPI1CLK               STM32_PLL1_Q_CK` : as claimed.
- The SPI4 value of 72 MHz needs PLL2 lines 1592, 1822, 1934, 2636 (and 2661 for SPI6), which the cited list lacks.
mcuconf citations: 333 `STM32_SPI123SEL ... PLL1_Q_CK` : correct. 113-121 HSE==0 branch is correct (the branch runs to 122, where CKPERSEL sits). 168-178 is the N/P/Q/R for PLL1 and PLL2 in the HSE==0 branch (correct; PLL3 values are 180-182). SPI45SEL is 332 and SPI6SEL is 345 (not cited in the table).

### 3.6 SPIDevice.cpp use of the kernel clock
- libraries/AP_HAL_ChibiOS/SPIDevice.cpp:40-45 (H7/G4 branch): `#define SPI1_CLOCK  STM32_SPI1CLK` ... `#define SPI6_CLOCK  STM32_SPI6CLK`. The F4/F7 branch (:53-58) uses `STM32_PCLK1/2` instead.
- :62-64 `static const uint32_t bus_clocks[6] = { SPI1_CLOCK, ..., SPI6_CLOCK };`
- :259-263 `derive_freq_flag_bus`: `uint32_t spi_clock_freq = SPI1_CLOCK; if (busid > 0 && uint8_t(busid-1) < ARRAY_SIZE(bus_clocks)) { spi_clock_freq = bus_clocks[busid-1] / 2; }` then halves until <= the requested frequency (loop at :266-269, limit 7 steps).
  So the prescaler is derived from the ChibiOS macros (not from a separate SPIn_CLOCK_FREQUENCY define). It uses the same STM32_SPInCLK expressions computed in 3.3, i.e. 80 MHz for buses 1-3 and 72 or 100 MHz for 4-6 depending on OSCILLATOR_HZ.
- SPIDevice.cpp:512 `#ifdef HAL_SPI_CHECK_CLOCK_FREQ` and SPIDevice.h:150 are an optional check only.
