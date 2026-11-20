# AET-WO-1211 — FEEDBACK: a driver IC as a specification (design spike, STOPPED at §2 for the schema ruling)

**Branch:** `wo/aet-1211-driver-ic-spike` from `main` `472d2d84`. **Builder:** Claude (Opus 5.5 orchestrating; four Sonnet readers, each report spot-checked at source before use). Written 2026-11-20.
**Status: STOPPED for the schema ruling (§7); RULED 2026-11-20T14:42Z, every lean taken (§8).** Nothing is built: no crate, specification, `boards/`, `fab/`, workflow or `ci/` file changed. Everything is in `spike/1211/` and this file.

**The answer, in five lines:**
- The proposal fits today's engine. With `idle` = `pulse_min_us` and `resolution_ps` = 1,000,000 stated on every PWM output, no solved word moves on any committed file (§3). A byte change to the specification moves only its hash lines; the ELF and every emitted board file stay byte-identical (measured).
- **Rule (e) as worded is wrong for both parts.** Both inputs high is **brake** on the DRV8833 and on the DRV8876 in IN/IN mode. It is the slow-decay state their PWM tables use (SLVSAR1E Table 1/2 p. 9; SLVSDS7B Table 4 p. 11). In PH/EN mode, both high is **forward**. Neither part has a forbidden input state, since both make their own dead time. I propose (Q3) that the hazard 4600 F2 found becomes "an idle must be a non-driving row of the part's truth table", with the truth table as row data.
- **The twin reaches GPIO and ADC, with holes.**
  - A script can drive an input low (nFAULT) and set an ADC channel's voltage.
  - Renode does not model `PUPDR`, `OTYPER`, `MOE`, `CCxE` gating in forced modes, the timer output before the first update event, ADC2, or the ADC's power-up.
  - The twin's motor judge calls a static idle `unwitnessed`.
- The parts row must carry pins, levels, pulls and truth tables, because `validate` cannot read eda's per-part arms (Q6).
- Four build work orders follow (§5).

## 0. How it was made

- **Held documents** (`~/.cache/aet-1211/ds/`, `SHA256SUMS`, `PROVENANCE.md`):
  - TI DRV8876 **SLVSDS7B** (Aug 2019, rev Nov 2019, 48 pp), sha256 `f96a70b1…6f3e`.
  - TI DRV8833 **SLVSAR1E** (Jan 2011, rev Jul 2015, 30 pp), `3e400e33…0de4`. Both were fetched from ti.com by plain curl. A first fetch set a browser User-Agent; it was discarded, and the plain fetch is byte-identical.
  - ST **RM0433 Rev 9** `672a9433…b6a0` and **DS12117 Rev 10** `607b9ead…c032`: Scotty's st.com copies (roadmap, 2301 D10), hardlinked.
  - Every table value cited below was read on a rendered page (PyMuPDF, `~/.cache/aet-1211/pages/`), not from extracted text. Three tables were first read from text and were rendered and checked before the push: SLVSDS7B Table 6 (p. 13), RM0433 Table 337 (p. 1608) and DS12117 Table 50 (p. 139). Prose is cited from text. Printed page numbers equal PDF page indices in all four documents.
- **Renode source:** `renode-infrastructure` `cd4b002a`, the tarball `container/Dockerfile` fetches, sha256 `83e6af97…6fa8`. The image's rebuilt `Infrastructure.dll` adds three NVIC commits, which touch only `src/Emulator/Cores/Arm-M/NVIC.cs` (`container/renode-nvic/*.patch`). No line quoted below is affected.
- **Runs:** one container at a time, in the pinned tools image `e5fc7ddb…` under `~/bin/capped-run` (8G, swap 0, read back), `--network none`, `nice -n 10`.
  - `spike/1211/twin/run.sh`: six hand-written Renode scripts on the stock platform. Output: `twin/twin.log`, Renode `1.17.0+20260907gitf1dd1b4af.aet1605`.
  - `spike/1211/measure/bytes.sh`: generate, build, solve and emit, in the image.
  - `measure/measure.py`: the §1.4 words, run on the host CLI built at `472d2d84`.
- **Predictions:** `spike/1211/PREDICTIONS.md`, one commit (`84a19cb4`), written before any fan-out. They are judged in §6.

## 1. The two fixtures (§1.2)

`spike/1211/fixtures/make.py` writes both fixtures from the committed 4600 driver (`crates/aethelgard-cli/tests/fixtures/driver-judges-nothing.json`). It keeps that driver's MCU, board, console port and SWD connector unchanged and drops its acknowledgements, since a board's acknowledgements are learned from `verify`'s ledger and no ledger exists for an unbuilt schema. Every number carries its page in the script. `fixtures/parts-rows.json` holds the two parts rows they bind to (§2.1). `schema_version` is the string `"22-or-23"` on purpose. As ruled, `1211a` takes 22, or 23 if `4240b` adds `startup_max_us` first (§5). **As ruled (§7), D2 is refused `PartLimitUndeclared` as written:** SLVSAR1E states no maximum input PWM frequency, so D2 has none to declare.

**D1, `fixtures/d1-drv8876.json` (DRV8876PWPR, PH/EN mode).**
- **Inputs:** `DRV_EN` and `DRV_PH` are channels of one timer, at 20 kHz, `resolution_ps` 15,625 (one 64 MHz kernel tick), idle `low`. EN = 0 is brake for any PH (Table 3 p. 11, row 2).
- **Straps:** `PMODE` tied to GND gives PH/EN mode, latched on wake (Table 2 p. 10; p. 17). `IMODE` tied to GND gives fixed off-time chopping and automatic retry, and nFAULT then reports overcurrent only (Table 6 p. 13). `VREF` tied to `VDD_3V3` (0–3.6 V allowed, p. 4) gives ITRIP = 3.3 / (1500 × 1e-3) = 2.2 A (Eq. 3 p. 13).
- **`nSLEEP`:** an output, initial `low`. It has a 100 kΩ internal pull-down (pp. 3, 5), so the part sleeps from MCU reset onward. Sleep is PH/EN's only Hi-Z state (Table 3, row 1).
- **`nFAULT`:** an input, active `low`, open-drain (p. 3), IOD ≤ 5 mA (p. 4). It has a 10 kΩ pull-up R3 to `VDD_3V3`. The **value is Figure 18's example** (p. 19, "not part of the TI component specification"); no recommended value exists.
- **`IPROPI`:** an ADC input through R2 = 1,500 Ω. A_IPROPI is 1000 µA/A, **typical only** (p. 6). At the declared full scale of 2 A, 2 × 1e-3 × 1500 = 3.0 V ≤ VREF+ 3.3 V.
- **Bridge `M1`:** mode `ph_en`, OUT1 and OUT2 on nets `M1_OUT1` and `M1_OUT2`, which reach J5.
- **`SERVO1`:** 50 Hz, idle `pulse_us` 1500, the centre, not the end stop of 4600 F2.

**D2, `fixtures/d2-drv8833.json` (DRV8833PWPR, two IN/IN bridges).**
- **Inputs:** `A_IN1`/`A_IN2` on PA8/PA9 (TIM1 CH1/CH2) and `B_IN1`/`B_IN2` on PC6/PC7 (TIM3 CH1/CH2), all idle `low` (00 is coast, Table 1 p. 9). Pinned by hand because the timer key would otherwise co-locate all four. Today's `solve` accepts exactly these pins (`fixtures/pins-check.json`, a today-schema variant of the 4600 driver).
- **Straps:** `AISEN` and `BISEN` tied to GND, since no current control is used (pp. 4, 10). No mode strap: the part is IN/IN only.
- **`nSLEEP`:** an output, initial `low`, with a 500 kΩ internal pull-down (p. 6). Its VIH min of 2.5 V (p. 6) is met: the MCU's VOH ≥ VDD − 0.4 V at 8 mA (DS12117 Table 50 p. 139 rev Y = Table 147 p. 243 rev V) gives 2.735 V at the domain's 5 % low edge.
- **`nFAULT`:** an input, active `low`, pulled up by the **MCU's own pull-up** (RPU 30/40/50 kΩ, DS12117 Table 49 p. 137 = Table 146 p. 241). SLVSAR1E gives no pull-up value (NOT FOUND).
- **No ADC input:** the DRV8833 has no analog current output. xISEN is a sense-resistor return (pp. 4, 10, 13).
- **`SERVO1`** as in D1.

Each fixture's proposed timer words (the §3 derivation):

| output | rate | `resolution_ps` | idle | PSC | ARR | CCR at idle | CCR at `pulse_min`/`pulse_max` |
|---|---|---|---|---|---|---|---|
| D1 `DRV_EN`, `DRV_PH`; D2 `A_IN1`…`B_IN2` | 20 kHz | 15,625 | low | 0 | 3199 | 0 | 64 / 3136 |
| `SERVO1` (both) | 50 Hz | 1,000,000 | 1500 µs | 63 | 19999 | 1500 | 1000 / 2000 |

## 2. The proposal (§1.3)

One rule holds throughout: **every new field is required, and no default substitutes (`INV-06`)**. A pin is `null` only to mean "the solver chooses", as everywhere today. The shared types live in `aethelgard-ir` (`INV-01`).

### 2.1 (a) A device with no bus, bound to a parts row

**Shape A (lean): a top-level `pin_devices` list.** It is private like the other device lists and read through `Specification::declared_pin_devices` (`RUL-16`). Each entry has these keys:

| key | meaning | refusals |
|---|---|---|
| `reference` | in the board's one reference namespace (`lib.rs:3466`, `power.rs:935`, `names.rs:98`) | `DuplicateReference` |
| `part` | a parts row whose `bus` is `none` | `PartWithoutRow`; `PartHasBus` (a bus part declared here); `PartWithoutBus` (a `none` part declared on a bus) |
| `supplies` | each supply pin of the row → a declared domain, e.g. `{"VM": "VMOTOR"}` | `SupplyPinUndeclared`, `UnknownDomain`, `SupplyOutOfRange` (the domain's band outside the row's `range_mv`) |
| `straps` | each strap pin of the row → `{"tie": GND | <domain>}`, or a resistor `{reference, part, ohms, to}` for a quad-level input | `StrapUndeclared`, `StrapLevelInvalid` (e.g. a tri-level strap given a resistor its row lacks) |
| `signals` | (b) | |
| `analog` | (c) | |
| `bridges` | (e) | |

**The parts row** (`fixtures/parts-rows.json`) gains `bus: "none"`, `identity: null`, and per pin a `kind` (`logic_input`, `enable_input`, `open_drain_output`, `current_output`, `strap_*`, `bridge_output`, `supply`, `ground`, `passive`, `sense_return`), its internal pull with value, its levels, and the part's timing (`t_wake`, `t_sleep`, input PWM maximum) and truth tables per strapped mode.
- **Why it must be row data, not an eda arm:** `validate` judges a signal against the device pin it binds to. Those facts live today only in `aethelgard-eda/src/schematic.rs` arms (L1405–1559) and `names.rs`, which `validate` cannot read (`INV-01`). This settles half of 4600 D4: **pins, kinds, levels, pulls, timing and truth tables are row data.** Symbols, footprints and the passive placement stay the eda work order's question (§5, 1211d).
- **What it costs:** today a row cannot lack a bus (`ir/src/parts.rs:151`, `enum Bus { I2c, Spi }`; `bus` and `identity` are required in `Part`). The parts schema bumps 10 → 11.

**Shape B: the device as a connector-like part with no signals of its own**, and GPIO and ADC declared on the MCU (§2.2, shape B). It is rejected as the lean because a device's truth table, wake time and pin levels would then have no home that the signals bound to it can be checked against.

**Typical-only values:** A_IPROPI and every input pull resistance are printed in the TYP column only. The row names them `*_typical`, and the engine uses them as typical, never as bounds (`RUL-19`).

### 2.2 (b) GPIO signals

**Choice: field of the device, or of the MCU?**
- **Shape A (lean): of the device.** `pin_devices[].signals[]` = `{device_pin, net, pin, direction, …}`. `pin` is the MCU pin (`null` to solve), as `Signal` has it.
  - **Why:** it follows the repo's own precedent. The SPI `cs` is declared on its device and solved into `topology.gpio` (`solve.rs:417–440`). The device's row gives the semantics: the active level, open drain, the internal pull, and what holds the pin before the first write.
  - **What it cannot express:** a GPIO with no device (an LED, a button). Each of those is a device with a row anyway, so it stays one shape.
- **Shape B: a top-level `gpio` list owned by the MCU** (`{net, pin, direction, …, device_pin: {reference, pin}}`). It is closer to the peripheral the solver hands out. Every device-side check then joins across two lists, and a removed device leaves dangling signals.

Per signal:

| field | values | meaning, refusals |
|---|---|---|
| `device_pin` | a row pin name | `UnknownDevicePin`. `SignalKindMismatch`: an output bound to a pin whose `kind` is not `logic_input` or `enable_input`, or an input bound to one that is not `*_output` |
| `net` | net name | the existing net guards (GND, debug nets, the reference namespace) |
| `pin` | MCU pin or `null` | the solver's `UnknownPin`, `ReservedPin`, `PinClaimedTwice` (`solve.rs:1139–1166`) |
| `direction` | `output` / `input` | — |
| `initial` (output) | `low` / `high` | the level the firmware writes **before** `MODER` = output. `LogicLevelIncompatible`: the MCU's VOH/VOL (DS12117 Table 50/147) at the domain's band edge against the row's VIH/VIL. `UnheldDuringReset`: the row gives the pin no internal pull and no `straps` entry holds it (§2.6) |
| `active` (input) | `low` / `high` | which level is asserted |
| `pull` (input) | `{"mcu": "up" / "down" / "none", source}` or `{"resistor": {reference, part, ohms, to}, source}` | `OpenDrainUnpulled`: an `open_drain_output` with `mcu: none` and no resistor. `PullOverDrive`: a resistor whose current at the domain exceeds the row's IOD |

Solved: `topology.gpio` becomes `{net: {pin, direction, initial | pull}}`, not `{net: pin}` (`lib.rs:516`). That is a **solved-version bump**. Without the direction, pincheck's `NotOutput` (pincheck `lib.rs:345–520`) and `implied_pupd` (`silicon.rs:779`) would fire on every input.

### 2.3 (c) ADC inputs

`pin_devices[].analog[]` = `{device_pin, net, pin, channel, transfer, full_scale, adc_resolution_bits}`:
- `channel`: `null` for the solver to choose, or `{peripheral, input}` by hand (`{"peripheral": "ADC3", "input": "INP5"}`), as `pin` is. Refused when not on the pin: `AdcChannelNotOnPin`.
- `transfer` is one of:
  - `{"current_output": {"resistor": {reference, part, ohms}}}`. The row's `ua_per_a_typical` gives V = I_load × A × R.
  - `{"divider": {top, bottom, reservoir}}`. This is `power.sense`'s shape (`power.rs:324`), so the battery sense becomes the same kind of input.
- `full_scale`: the largest declared quantity (`milliamps`, or `millivolts` at the divider's top).
- `adc_resolution_bits`: as `power.sense` declares it today.

Refusals:
- `AdcFullScaleExceeded` (the analogue of `SenseFullScaleExceeded`, `power.rs:2121`): the transfer at full scale against VREF+ = the MCU domain, **at the worst edge of the row's error band**, not at the typical. SLVSDS7B gives A_IPROPI as typical only but bounds its error by range (AERR, p. 6): ±4 % for 0.5–2 A (PWP, TJ < 125 °C), ±5 % at 125–150 °C. The row carries these bands (`parts-rows.json`). D1 at 2 A: 3.0 V × 1.05 = 3.15 V ≤ 3.3 V.
- `AdcSourceImpedance`: R against RAIN at the chosen sampling time.
- `AdcOutputCurrentExceeded`: I_load × A above the row's `output_max_ma` (DRV8876 IIPROPI 3 mA, p. 4).
- `NoAdcPin`, as today.

**Solved:** a map `adc_inputs: {net: {pin, peripheral, channel}}`. Today's `SolvedSense` stores the candidate `channels` and **chooses no ADC peripheral** (`solve.rs:449–480`); the solver must now choose one. Since Renode has no ADC2 (§4, A6), the lean is that **the solver prefers an ADC1 or ADC3 channel and refuses an ADC2-only pin as `AdcUnmodelled`** unless the specification pins it by hand. On LQFP144 that is PF13/PF14 (ADC2_INP2, INN2/INP6, DS12117 pp. 65–72).

Also: today's rule takes the lowest package position. **Measured:** `solve` on `crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json` puts `VBAT_SENSE` on **PF3, `ADC3_INP5` only**. A battery sense on an H753 board lands on ADC3 today, and nothing chooses that consciously.

**`power.sense` (choice):** keep it where it is and solve it into the same map (lean), or move it into `pin_devices` as a divider device. The lean keeps the battery's place in the power tree, which `power.rs`'s checks read.

### 2.4 (d) `idle` and `resolution` on a PWM output

Both go **inside `pwm`**, so a DShot output (`pwm: null`) takes neither. DShot's idle is its protocol's zero-throttle frame, out of scope here.

| field | values | meaning | refusals |
|---|---|---|---|
| `pwm.idle` | `{"level": "low"}`, `{"level": "high"}`, `{"pulse_us": n}` (0 allowed, as the work order states) | what the output carries from `CEN` until the first command. It is separate from `[pulse_min_us, pulse_max_us]`, which bounds commands; the range rules are unchanged | `IdleExceedsPeriod` (`pulse_us` × 1e6 ps > (ARR+1) × tick), `IdleNotOnTick` (`pulse_us` × 1e6 not a whole number of ticks) |
| `pwm.resolution_ps` | ps per timer tick | the tick. **Whole ns cannot state it:** the 64 MHz kernel tick is 15,625 ps | `ResolutionNotDerivable` (kernel_hz × ps / 1e12 not a whole number in 1…65,536), `RateOutOfRange` (ARR + 1 outside 3…65,536 ticks), `PulseNotOnTick` (`pulse_min_us` or `pulse_max_us` × 1e6 not a whole number of ticks) |

**Derivation** (`measure/measure.py`, written independently of `solve_timing`):
- PSC = kernel × ps / 1e12 − 1.
- ARR = round_half_up(1e12 / (rate × ps)) − 1.
- CCR at idle: `pulse_us` × 1e6 / ps; `low` → 0; `high` → ARR + 1.

**`high` breaks one clause of today's `solve_timing`:** `ccr + 1 >= ticks` refuses any CCR ≥ ARR (`motor.rs:661`). The clause stays for commanded pulses and is lifted for the idle word alone.

**Low and high are realised in PWM mode 1, not in forced modes.**
- RM0433 Rev 9 p. 1558: "If the compare value in TIMx_CCRx is greater than the auto-reload value … OCxREF is held at '1'. If the compare value is 0 then OCxRef is held at '0'."
- Leaving idle is then one `CCR` write; no `OCxM` change is needed.
- Kani judges one word, and the twin measured both cases (§4).

**Choices.**
- **Is `idle` required on every PWM output?** Yes (`INV-06`). Every committed specification must state it: **8 PWM outputs in 2 files**, and the 4 solved outputs of `svd-hal/tests/ui/motors-pwm/topology.json` (§3).
- **`pulse_us: 0` and `level: low` both give CCR 0.** *Ruled (Q4): `pulse_us: 0` is refused as `IdleSpelledTwice`.*
  - Option 1, accept both: the work order's wording.
  - Option 2 (lean): refuse `pulse_us` 0 as `IdleSpelledTwice`, naming `level: low`. One state then has one spelling in every verdict.
- **Does `resolution_ps` replace whole-µs ticks for every PWM output?** Yes: it is required on every one, and 1,000,000 restates today's tick. **No solved word moves** (§3).
  - It **joins the timer-share key** (`solve.rs:1224`), which becomes `(protocol, rate_hz, resolution_ps)`.
  - A new refusal applies when two outputs pinned to one timer differ (`TimerResolutionConflict`, beside `TimerRateConflict`).
- **Its exactness depends on the kernel clock.** That is the reset HSI at 64 MHz until `4240b` moves the clock. A resolution valid at 64 MHz may be refused at another clock, so `4240b` must re-judge every declared one.

**The pad before `CEN` (`silicon::motor_output_pupd`, `silicon.rs:768`).** Today every motor pin is pulled down from configuration until `start_outputs` (`svd-hal board.rs:860–866`, ruling Q-G on 4230). With `idle: high` that pull fights the idle.
- **The rule becomes the pull of the idle:** `low` and `pulse_us` → down (unchanged), `high` → up.
- The oracle (`implied_pupd`) and the ledger's `motor_pupd` (`ledger.rs:1727`) take the same rule.
- Also from RM0433 p. 1558: in the generator's order (`board.rs:3272–3290`: …, CCER, BDTR, EGR.UG, then CR1.CEN), an output whose CCR > 0 drives `OCxREF` high from `UG`, with `CNT` = 0 < CCR, until `CEN`. For `idle: low` (CCR 0) no such window exists.

### 2.5 (e) The bridge pair

**Choice: a property of two outputs, or a kind the device owns?**
- **Lean: the device owns it.** `pin_devices[].bridges[]` = `{id, mode, inputs: {device_pin: motor_output_label}, outputs: {device_pin: net}}`.
  - **Why:** D1's only Hi-Z state is `nSLEEP` low (SLVSDS7B Table 3). Its idle therefore spans a GPIO and two outputs, and only the device sees all three.
  - `mode` must match the strap (`BridgeModeStrapMismatch`: PMODE logic low is `ph_en`, logic high (VTIH ≥ 1.5 V, SLVSDS7B p. 5) is `in_in`, Hi-Z is independent half-bridges, which no bridge declares). It must also be a mode the row has.
- **The other shape:** `pwm.bridge: {"pair": "<label>"}` on each of the two outputs. It duplicates the pair across two entries and cannot reach nSLEEP.

**Choice: the rule, measured against the datasheets.** *Ruled (Q3): option 2. **The work order's "never driven high together" is withdrawn as wrong for both parts**, on the table below.*

| inputs | DRV8833 (SLVSAR1E Table 1 p. 9) | DRV8876 IN/IN (SLVSDS7B Table 4 p. 11) | DRV8876 PH/EN (Table 3 p. 11) |
|---|---|---|---|
| 0 0 | coast (Z, Z) | coast (Z, Z) | brake (L, L) |
| 1 1 | **brake / slow decay (L, L)** | **brake (L, L)** | **forward (H, L)** |
| PWM on one, other held 1 | forward/reverse PWM, slow decay (Table 2) | — | — |

Both parts make their own dead time (SLVSDS7B p. 11; SLVSAR1E p. 6 fn. 1 and p. 12: "prevents any risk of cross-conduction"). **No input state of either part is forbidden.**

- **Option 1, the work order's rule: "never driven high together."** The firmware, Kani and the twin each judge that no instant has both inputs high.
  - It forbids slow-decay PWM on the DRV8833, a mode its datasheet tabulates.
  - It forbids brake on both IN/IN parts.
  - In PH/EN it forbids forward.
  - It fits a **discrete half-bridge leg** (high-side and low-side gate inputs of one leg), where both high is shoot-through. Neither part here is one.
- **Option 2 (lean): the truth table is row data.**
  - Each strapped mode lists its states, with `drives` and a `forbidden` set (`parts-rows.json`). Both parts' `forbidden` sets are empty; a gate-driver row would fill its own.
  - **`BridgeIdlePulsed`:** a bridge input's idle must be a `level`. A `pulse_us` idle makes the pair visit two rows every period: an IN/IN pair at 1 µs passes brake (1 1) and then coast (0 0), which are both non-driving. So a row test alone would pass the very hazard 4600 F2 found, where an IN/IN pair chops brake from boot and a PH/EN pair at 2 % drives. The 4600 driver as committed (`HB_IN1`/`HB_IN2`, idle = `pulse_min_us` 1 once migrated) is refused here, once declared as a bridge.
  - **`BridgeIdleDrives`:** the static idle tuple (`nSLEEP`'s `initial`, each input's `level`) must be a row whose `drives` is false. **Both fixtures pass.** Both declare `nSLEEP` `initial: low`, so each idles in sleep. With `initial: high`, D1 would idle in brake (EN 0) and D2 in coast (00), which are both non-driving.
  - **`BridgeForbiddenState`:** any reachable state in the forbidden set.
  - **The rule judges idle only.** The generated board crate has no run-time drive: `Tim{n}::start_outputs` returns `Running`, which has no method (`svd-hal board.rs:2636`, `3309–3330`). **It judges nothing at run time** until a drive API exists. Designing one is out of scope here.
- **D1's PH on a timer (choice).** As the work order writes D1, `DRV_PH` is a timer channel that only ever holds 0 % or 100 %. The alternative is a GPIO output (`signals[]`, `initial: low`), which frees a channel and makes PH's static nature explicit. Lean: allow both. The bridge's `inputs` may name either a motor output or a signal; the truth-table rule reads levels either way.

### 2.6 The window from reset to the first write

RM0433 Rev 9 p. 541: `MODER` resets to analog on every port except A (`0xABFF FFFF`) and B (`0xFFFF FEBF`). Analog mode disconnects the output buffer and the pulls (p. 539). From reset until the firmware's first write, a driver's inputs are held only by the part and the board, not by the specification's initial levels.
- **Rule:** a `logic_input` pin bound to a signal or a motor output must have a row pull (DRV8876 100 kΩ, DRV8833 150 kΩ / 500 kΩ) or a `straps` entry, else `UnheldDuringReset`.
- **The pulled level must be a non-driving state.** Both parts pull every input down, giving sleep. **Both fixtures pass.**
- **The enable pin.** `nSLEEP` is a row pin of kind `enable_input` (active high, with `t_wake`). Its `initial` is the level the bring-up leaves it at. No separate wake field exists: `initial: high` is the declaration that the bring-up wakes the part.
- **RM0433 states no "write ODR before MODER" guarantee** (searched; only "the value written to ODR is output on the pin when configured as output", p. 534). The generator already orders BSRR before MODER (`board.rs:2788–2799`), and Kani holds it (`board.rs:4991–5005`). The rule cites the general statement, not a guarantee that does not exist.

### 2.7 What each projection does

**(a) device**
- **validate:** §2.1's refusals.
- **solver:** places nothing for the device itself. Its signals are (b)/(c).
- **generator:** an `enable_input` signal is written after its bridge's inputs are at idle (`CEN`). Only when its `initial` is the enabling level does a `t_wake` wait from the row follow (DRV8876 and DRV8833 tWAKE 1 ms max, SLVSDS7B p. 5, SLVSAR1E p. 7). Both fixtures declare `low`, so neither wakes at bring-up. The bring-up's one wait today is `settle(spins)` (`firmware.rs:373`, `601–613`); a bounded wait for the row's `t_wake` is new. Motors start at `firmware.rs:993`, so the step goes after it and before `BOOT_MAGIC` (`firmware.rs:1000`).
- **Kani:** the schedule harness gains the order "inputs at idle and `CEN` before an enable input's write". Today's harness takes pins-before-timer from the typestate (`board.rs:5755–5765`).
- **twin:** no Renode model of either part exists. The twin witnesses the MCU side only and names the device `device_unmodelled`, as for an I2C part with no model.
- **ledger:** no device registers.
- **eda:** an arm or row data per part (§5, 1211d). No `Driver_Motor` symbol is vendored: `eda/src/library.rs`'s pinned list (`PINS`, L19) names none, and `kicad-lib` holds no DRV88xx.
- **verify:** an `erc`/`drc` run on the drawn device. Coverage gains a target only through (b), (c) and (e), **and through the motor rail.** A part's supply (`VMOTOR`) is a voltage domain, and coverage makes every domain a `Rail` target (`verify.rs:1130–1157`). No power tree renders a droop on it, so it stays unmet, which blocks `--fab-out` (`campaign_faults_every_bus_and_rail`, `verify.rs:204`). Measured: today's `validate` accepts a second, sourceless domain. That is `1211c`'s, or 4600 F7's power-input kind. The motor connector's current rating is not a field and is unjudged (4600 §2).
- **validate, connectors:** a connector may name only a declared signal, a domain or `GND` (`ConnectorNetUnknown`, measured on a today-schema variant). A bridge's `outputs` nets must join that set.
- **hwdef:** refuses by name for a board with no ArduPilot (4600 D1).
- **pinout:** judges nothing on the device.
- **speeds:** judges nothing.

**(b) GPIO**
- **validate:** §2.2.
- **solver:** places each signal's pin like `cs` (`solve.rs:417–440`), into a directional `topology.gpio` (a solved bump).
- **generator:**
  - Outputs: BSRR `set` (high) or `reset` (bit 16+n, low) before `MODER` = 01 (`board.rs:2788–2799`). Today it always sets high.
  - Inputs: `MODER` 00, and `PUPDR` per `pull.mcu`. Today the oracle gives every non-motor pin `00` (`oracle_pupdr`, `board.rs:3723`).
- **Kani:** `into_output`'s ready clause takes the declared level's BSRR word (`board.rs:4991–5005`), not a constant high. A new input clause holds `MODER` 00 and `PUPDR` against the declared pull. Both oracle values come from the specification (`INV-07`).
- **twin (§4):**
  - Witnesses the initial level only as the BSRR write and its order before `MODER`. The pad itself: G1 reads the level on IDR in **any** mode, so the pad is not witnessed.
  - A scenario can drive an input (G3). That calls for a new fault kind, `pin_held {net, level}` (`chaos/src/record.rs:197–225` has eight kinds, none on a GPIO).
  - Holding a declared **output** is refused (`PinHeldIsOutput`), since G4 shows the drive overwrites `ODR` with no contention.
  - The pull is never witnessed (G2), so it is rule-only.
- **ledger:**
  - `PUPDR` of a declared input becomes `independent-rule` (the declared pull), like `motor_pupd`.
  - BSRR's `observed` justification today cites chip selects only. It must name a declared output's initial level.
  - `OTYPER` stays `ignored-by-model` (G5).
- **eda:** a pull-up resistor from `pull.resistor`.
- **verify:** a per-signal gate, counting one per signal (2306: "a gate whose line lists none judges its one input whole, and counts 1" is the shape to avoid). It covers levels, pull and reset hold. Coverage gains `Target::Gpio(net)` for inputs, reached by `pin_held` (`verify.rs:710`).
- **hwdef:** no `GPIO(n)` line exists (hwdef `lib.rs:522–540` `PinNotEmitted`). It refuses by name (`GpioNotEmitted`) until a line exists.
- **pinout:** forward checks need the direction. Its reverse scan sees only output and AF pins (pincheck `lib.rs:345–520`), so **it judges nothing on an input or an analog pin**. That must be said in its verdict.
- **speeds:** a static output at `OSPEEDR` 00 is out of scope, as motor outputs are (`MOTOR_OUTPUT`'s reason, `board.rs:953`); it judges nothing, and says so.

**(c) ADC**
- **validate:** §2.3.
- **solver:** chooses the pin, peripheral and channel into `adc_inputs`, preferring ADC1 or ADC3.
- **generator (new: no generated firmware touches an ADC today):**
  - The enable sequence of RM0433 Rev 9 pp. 926–933: `DEEPPWD` 0, `ADVREGEN` 1, wait TADCVREG_STUP, calibration, `ADEN`, `ADRDY`.
  - `PCSEL` holding every sequenced channel (p. 935–936: the result is wrong otherwise).
  - `SMPR` from the RAIN rule.
  - One conversion read at bring-up.
- **Kani:** a new harness for the enable order, PCSEL ⊇ SQR channels, and SMPR at least the rule's minimum. The oracle comes from the specification.
- **twin (§4):**
  - A script sets each declared channel's voltage (A3, A5), and `DR` returns it.
  - **The twin must set every declared channel explicitly.** An unset channel reads 0 V (A2), which is a substituted value (`INV-06`).
  - Never witnessed: the transfer, source impedance against sampling time, the regulator and calibration timing (A0, A1), and ADC2 (A6). All of these are rule-only.
  - A new fault kind, `analog_value {net, millivolts}`, drives a declared value.
  - Today `twin::kind()` refuses any ADC peripheral as `UnsupportedPeripheral` (`twin/src/lib.rs:157–187`), although the stock include instantiates both models.
- **ledger:** ADC registers have no rows today, so the first ADC write is refused `LedgerFieldUnclassified` (`ledger.rs:297` `classify`). Proposed classes:
  - `ADC_CR.DEEPPWD/ADVREGEN/ADCAL`: `silicon-only`. The model starts in `0x0FC00000`, not RM0433's `0x2000 0000` (A0), and gives `ADRDY` at once (A1).
  - `PCSEL`: `observed`, through the model's "not preselected" warning (`STM32_ADC_Common.cs` L196–202).
  - `SQR1`: `observed` (`DR` from the channel).
  - `SMPR`: `independent-rule` and `ignored-by-model`.
- **eda:** the transfer's resistor (R2) and the ADC net.
- **verify:** an ADC gate counting one per input (full scale, RAIN). Coverage gains `Target::Analog(net)`, reached by `analog_value`.
- **hwdef:** refuses as today (`SenseNotEmitted`, hwdef `lib.rs:407`; `HAL_USE_ADC FALSE`, L517).
- **pinout:** judges nothing on analog pins.
- **speeds:** judges nothing.

**(d) idle and resolution**
- **validate:** §2.4.
- **solver:** the timer key gains `resolution_ps`. `solve_timing` (`motor.rs:637–665`) derives PSC from the resolution, and CCR from `idle` with 0 allowed (`ccr == 0` refused at `motor.rs:661` today, and `PulseRange` at `lib.rs:3770`).
- **generator:** no change in order. It writes the solved CCR.
- **Kani:** `timer_oracle` (`board.rs:3506`) restates the emitter by design. The independent judge is `check_silicon` (`motor.rs:722`), whose `high_ok` (`motor.rs:765`) must judge the declared idle, not `pulse_min_us`.
- **twin:** see §4. A static idle has **zero edges, which `witness_motor` reports as `unwitnessed`** (`renode.rs:1720`, fewer than two rises). A static-level witness is new: zero edges, plus the observer's level at the window's end. Idle high in Renode reads low until the first update event (H2), so the witness must start after one period.
- **ledger:** `PSC`, `ARR` and `CCR` stay rule rows (`TIM_RULE`), with the rule reading `idle` and `resolution_ps`.
- **eda:** none.
- **verify:** `check_silicon`'s `silicon_findings` (`verify.rs:1620`) judges the declared idle.
- **hwdef:** `MOT_PWM_*` stays not emitted (4600 F14). Its motor line hard-codes `PULLDOWN` (hwdef `lib.rs:495–507`), so it must follow the idle as the pull rule does (§2.4).
- **pinout:** `implied_pupd` follows the idle (§2.4).
- **speeds:** unchanged.

**(e) bridge**
- **validate:** `BridgeModeStrapMismatch`, `BridgeIdleDrives`, `BridgeForbiddenState`, and `BridgeSplitAcrossTimers`: both inputs of an IN/IN pair on one timer, so they share one counter. PH as a GPIO is exempt.
- **solver:** keeps a pair's two motor outputs on one timer.
- **generator:** nothing beyond (d). No drive API exists (§2.5).
- **Kani:** at idle, the pair's two CCR words and `nSLEEP`'s initial level form a non-driving row. **It judges nothing at run time.**
- **twin:** both pins' static levels at idle, from (d)'s new witness. The device's response is never witnessed: no model exists.
- **ledger:** none.
- **eda:** the bridge outputs on their declared nets.
- **verify:** a bridge gate counting one per bridge. Coverage gains `Target::Bridge(id)`, `unmet (no fault kind reaches it)` until an output fault exists (4600 F3).
- **hwdef, pinout, speeds:** judge nothing.

## 3. What moves (§1.4): measured, with the prediction beside it

`measure/measure.py` was run on every tracked `.json` outside `spike/`. A file counts as a specification when it has `schema_version`, `mcu`, `motor_outputs` and `project_name`, and as a solved topology when it has `specification_sha256` and non-empty `motor_outputs`. Output: `measure/measure.json`.
- **Today's words** come from the **real** `aethelgard solve` built at `472d2d84` for each specification, and from the committed words for each solved topology.
- **Excluded:** `spike/4600/*` (never committed as specifications, its §9), schema-1 scenario and repro files (not specifications), and the parts tables.

| set | files | outputs | gains | words compared | moved |
|---|---|---|---|---|---|
| committed specifications (schema 19) | 29 | 24: 8 PWM in 2 files (`driver-judges-nothing`, `motors-pwm`), 16 DShot in 4 | `schema_version` on all 29; `pwm.idle` + `pwm.resolution_ps` on the 8 PWM | 8 PWM against `solve`'s output | **0** |
| solved topologies (schema 12) | 2 (`hwdef/tests/fixtures/serial-dshot-9600.topology.json`, `svd-hal/tests/ui/motors-pwm/topology.json`) | 8: 4 PWM, 4 DShot | the solved version on both; the copied `pwm` fields on the 4 PWM | 4 PWM against the committed words | **0** |
| specifications built in Rust | `ir/tests/motor.rs` (`pwm()` L58, `PwmTiming` L423), `ir/tests/validate.rs`, `ir/src/motor.rs` tests (L795), `cli/src/renode.rs` L4060: 7 `pulse_min_us` sites outside `motor.rs`'s own definition, solver and judge (15 in 4 files counting those) | — | the same two fields | not run | — |

- **DShot outputs are untouched by construction.** No field reaches them (`pwm` is `null`), so they are not "compared"; `today` would equal `today`.
- **Prediction P1a ("no solved word moves"): holds** on all 12 PWM outputs. PSC 63 (64 MHz / 1 MHz − 1) and ARR are unchanged. CCR = `pulse_min_us` = `idle.pulse_us`.

**Emitted bytes** (`measure/bytes.sh`, `measure/bytes-result.md`).
- **The stand-in:** no engine reads the new fields, so the measurement asks the question that can be asked today. Does a byte change in the specification that no field reads move anything but the hash?
- **The run:** for each specification with PWM outputs, generate, build, solve and emit from the committed file and from a copy with one newline appended (a different sha256).
- **Results:**
  - **Firmware ELF:** byte-identical. `motors-pwm` `8f94854f…`, `driver-judges-nothing` `5b31d753…`.
  - **Emitted schematic, board and BOM:** byte-identical on both.
  - **Generated tree:** 8 files differ, each only in its header line(s) naming the specification's sha256:
    - `firmware/.cargo/config.toml`, `firmware/Cargo.toml`, `firmware/link.x`, `firmware/src/main.rs`;
    - `<project>.repl`;
    - `src/board.rs`, `src/lib.rs`, `src/proofs.rs`.
  - **`solve`'s output:** only `specification_sha256`.
  - The generated tree carries no schema version and no topology hash (grepped).
- **Prediction P1b ("no byte moves by value; bytes move by version or hash"): holds**, with this list as the "by hash" set.
- **The committed files that carry a specification's hash**, which the version bump moves:
  - `boards/sentinel-01/*` and `boards/sentinel-01-bench/*` (from `examples/sentinel-01-minimal.json` and `sentinel-01.json`);
  - `fab/sentinel-01-bench/package/{FAB-NOTES.md, MANIFEST.json}` (eda `fab.rs:521`, cli `fab.rs:712`);
  - `hwdef/tests/fixtures/{sentinel-01,sentinel-02,serial-dshot-9600}.*`;
  - `svd-hal/tests/ui/{motors-pwm,serial-four}/*`.
- **Moved by the solved-version bump, not by a specification hash:** the inline v12 solved-topology literals in `twin`, `cli/src/renode.rs` and `cli/src/analog.rs`.
- **Not mapped:** the sha256 literals in `chaos/tests/golden.rs` and `svd-hal/tests/{serial_ports,order_2203}.rs`. The reader did not trace which input each hashes; `1211a` must.

**One output, migrated** (`crates/aethelgard-cli/tests/fixtures/motors-pwm.json`, `MOTOR1`):

```json
"pwm": {"rate_hz": 400, "pulse_min_us": 1000, "pulse_max_us": 2000}
"pwm": {"rate_hz": 400, "pulse_min_us": 1000, "pulse_max_us": 2000, "idle": {"pulse_us": 1000}, "resolution_ps": 1000000}
```

Both give PSC 63, ARR 2499 and CCR 1000 (`measure.json`).
- **Version literals to edit:** five `assert_eq!(…, 19)` (`ir/tests/{motor,bench,serial,board,orientation}.rs`) and `ir/tests/readme_versions.rs`'s README pin.
- **Not measured:** the version bump itself, and a topology carrying the new fields, since no engine reads them. The design's claim, that the generator reads only the solved words, which do not move, rests on `timer_writes` (`board.rs:3272–3290`) reading `TimerPlan`'s PSC, ARR and CCR alone.

## 4. The twin's reach (§1.5): read at source, then measured

The stock platform is `platforms/cpus/stm32h753.repl` → `stm32h7.repl`, pinned in `twin/src/platform.rs:37–53`. The twin's emitted platform `using`s it and adds the solved peripherals (`platform.rs:777`). Runs: `twin/run.sh`, log `twin/twin.log`. A `b .` firmware in flash lets virtual time run; every access is from the monitor.

**GPIO (`STM32_GPIOPort.cs`, `cd4b002a`)**
- `IDR` and `ODR` both read one `State` array (L195–202). `ODR` and `BSRR` write it in **any** mode (L199–210, `WriteState` L130–139).
- `PUPDR` is stored and changes nothing (L190–194).
- `OTYPER` is `WithTaggedFlag` (L166–184).
- `OnGPIO` sets `State` from outside (L108–112).
- `MODER`'s reset value is a constructor argument (L27, L92), and `stm32h7.repl` passes none.

| # | measured | silicon (RM0433 Rev 9) | so the twin… |
|---|---|---|---|
| G0 | `GPIOA_MODER`, `GPIOC_MODER` read `0x00000000` at reset | `0xABFFFFFF`, `0xFFFFFFFF` (analog, p. 541) | models no reset-to-first-write window (pins read as inputs) |
| G1 | `BSRR` set PC0 with `MODER` at reset: `IDR` and `ODR` read 1 | the level drives only once `MODER` = output (p. 534) | witnesses an initial level **only as the write and its order**, never as a pad level |
| G2 | PC1 input, `PUPDR` = 01, undriven: `IDR` bit 1 reads 0 | the pull-up gives 1 | cannot witness a pull: **rule-only** |
| G3 | `gpioPortC OnGPIO 2 true/false`: `IDR` 0x5 → 0x1 | — | **can** drive nFAULT from a scenario |
| G4 | PC3 output, `ODR` bit 3 set, then `OnGPIO 3 false`: `ODR` and `IDR` both 0 | contention; `ODR` keeps 1 | must never hold a declared output (refuse) |
| G5 | `OTYPER` write 0x4 → "Unhandled write … Tags: OT2", reads 0 | open drain | open drain unmodelled (already `ignored-by-model`) |

**Timer (`STM32_Timer.cs`).**
- `MOE` is `WithTaggedFlag` (L385), and `OCxPE` likewise (log).
- Forced modes set the connection at once (`WriteOutputCompareMode` L723–737).
- A CCR write of 0 disables the channel's compare timer (L396–403).
- PWM 1 sets the output only at the update event (L66–84).

| # | measured (TIM1 CH1 on PA8; the platform's 250 MHz, PSC 249, ARR 49) | silicon | so |
|---|---|---|---|
| T1–T2 | CCR 0: PA8 0 before `CEN` and at 8 instants after | held 0 (p. 1558) | **idle low witnessed**, as zero edges at level 0 |
| T3 | CCR 25 running: 1,1,0,0,0,1,1,1 at 7 µs steps | 50 % | edges as today |
| O1–O2 | the generator's order with CCR 25: PA8 0 after `CCER`, `BDTR` and `UG`, and 0 at 0, 7 and 37 µs after `CEN` | `OCxREF` high from `UG` (CNT 0 < CCR), MOE set | **the pre-`CEN` window and the first period are not modelled** |
| H1–H2 | fresh machine, idle high (CCR = ARR + 1 = 50): PA8 0 before `CEN` and at 0, 7, 37 µs; 1 at 57 and 107 µs | held 1 from `UG` | **idle high witnessed after one period only** |
| T4–T6 | forced active with `CEN` 0, `MOE` 0: PA8 1; forced inactive: 0; forced active with `CC1E` 0: 1 | `MOE` 0 or `CC1E` 0: output disabled, Hi-Z (Table 337, p. 1608) | `MOE`/`CCxE` gating unmodelled (`BDTR` already `ignored-by-model`); the lean avoids forced modes |

**ADC (`STM32H7_ADC.cs` L13–42 → `STM32_ADC_Common.cs`).**
- 19 channels per instance.
- Samples come from `ADCContainer` (L433–444), and default children read 0 µV (`IADC.cs` L95–105).
- `SetVoltage(µV, channel)` replaces a channel's sample (`IADC.cs` L21–58).
- A sample converts as `round(mV / (Vref·1000) · (2^bits − 1))` (L451–460).
- `stm32h7.repl` instantiates `adcM1S2` at `0x40022000` (ADC1, with ADC2 as its unmodelled slave, L231–239) and `adc3` at `0x58026000` (L84–91), both with `referenceVoltage: 3.3`.
- **Prediction P4d ("not instantiated") missed.**

| # | measured | so |
|---|---|---|
| A0 | `ADC1_CR` reads `0x0FC00000` at reset | RM0433: `DEEPPWD` 1. Power-up unmodelled |
| A1 | `ADRDY` 1 right after `ADEN` | regulator and calibration waits unmodelled |
| A2 | channel 5 never set: `DR` 0 | an unset channel is 0 V. The twin must set each declared one (`INV-06`) |
| A3 | `adcM1S2 SetVoltage 1650000 5`: `DR` `0x8000` | 1.65 / 3.3 × 65,535 = 32,767.5 → 32,768. **A script sets a value** |
| A5 | `adc3 SetVoltage 825000 5` (PF3 = ADC3_INP5, DS12117 p. 65): `DR` `0x4000` | ADC3 works the same (16,383.75 → 16,384) |
| A6 | `ADC2_CR` write at `0x40022108`: reads 0; `ADC2_ISR`: "Unhandled read from offset 0x100" | **ADC2 is not modelled** |

**The rule-only judgement for what the twin cannot witness:**
- **ADC:** the expected code at a declared quantity is computed from the specification by one rule: round(V/VREF+ × (2^bits − 1)), with V from the declared transfer at the row's typical A. The twin witnesses only that the firmware's sequence returns the injected value on the declared channel, never the analog chain.
- **GPIO input pull:** `PUPDR` against the declared pull, by rule.

## 5. The split (§1.6)

Specification **20** is `4240a`'s and **21** is `1210`'s: the architect's kickoff for this work order says "`1210` holds 21, and `4240b` may take the next". **Ruled:** `1211a` takes the next free version after `1210`'s 21. That is **22**, or **23** if `4240b` adds `startup_max_us` first. The architect writes `1211a` from this section. The schema work order below therefore takes **the next free specification version after `4240b`'s**, and the solved version after `4240a`'s 13. This spike picks no number.

1. **`1211a`, IR and schema** (strong tier).
   - **Scope:** `pin_devices` with `signals`, `analog` and `bridges`; the parts rows (parts schema 11) for DRV8876PWPR and DRV8833PWPR; `pwm.idle` and `pwm.resolution_ps`; every refusal in §2; the solver (directional `gpio`, `adc_inputs` with a peripheral, the timer key); `check_silicon` judging idle. It also needs `hwdef` and `pincheck` to refuse the new nets by name (`GpioNotEmitted`; the direction read) rather than fail.
   - **Migration:**
     - `schema_version` on all 29 specifications;
     - `idle` and `resolution_ps` on the 8 PWM outputs;
     - the two solved topologies and the 4 Rust builders;
     - the version literals.
   - **Note (Q5, ruled):** `resolution_ps` is exact only at the clock it is judged at. `4240b` re-judges every declared resolution when it moves the clock.
   - **Fence:** `crates/aethelgard-ir`, the fixtures and examples, and the **`boards/sentinel-01*` hash lines by the generator plus a re-dispatched package**, as `4240a`'s fence ruling had it.
   - **It also reaches every reader of `topology.gpio`'s new shape:** `svd-hal`, `twin`, `cli`, `eda`, `hwdef` and `pincheck`. Each gets a named refusal for a pin device until its work order lands (`PinDeviceNotGenerated`, `PinDeviceNotSimulated`, `PinDeviceNotDrawn`, `GpioNotEmitted`), on `HseNotGenerated`'s precedent (`4240a`).
   - **Against `1210`** (specification 21): `validate` refuses a part with no emitter arm on a specification that draws a board. Until `1211d` lands, a pin device is declared only on a specification that draws none. `1211a`'s fixtures must declare that, or `1211d` lands first.
2. **`1211b`, generator, Kani and ledger** (after `1211a`).
   - **Generator:** idle CCR (0, ARR + 1, pulse); PSC from the resolution; GPIO outputs at their declared level and inputs with their pull; the ADC enable sequence; enable inputs written after their bridge's `CEN`, with `t_wake` when they wake the part.
   - **Kani:** the oracles from the specification.
   - **`motor_output_pupd`:** follows the idle.
   - **Ledger:** the classes in §2.7. Without them, the first ADC write is refused before any gate runs.
3. **`1211c`, twin, chaos and verify** (after `1211b`).
   - `twin::kind` gains ADC1 and ADC3; every declared channel is set explicitly.
   - A static-level motor witness.
   - The `pin_held` and `analog_value` fault kinds.
   - Coverage targets for inputs, ADC inputs and bridges.
   - Per-item gates (2306).
   - The fidelity notes of §4 recorded as ledger reasons.
   - The motor rail's coverage (§2.7 (a)).
   - It takes the next scenario version (today `SCENARIO_VERSIONS` = 1–4, `chaos/src/campaign.rs:32`) and the next trace version (today 17, `chaos/src/event.rs:52`).
4. **`1211d`, eda: an arm per part** (after `1211a`, beside `1211b`). Two parts, one work order: the symbols and footprints authored under `crates/aethelgard-kicad-lib` (none vendored), the passives and straps from the rows, and the bridge outputs to connectors. Whether arms become row data (4600 D4) is the architect's call before it is written.

`1211b`, `1211c` and `1211d` take no specification version.

## 6. Predictions beside results

| # | prediction | result |
|---|---|---|
| P1a | no PSC, ARR or CCR moves; DShot untouched | **holds** (12 PWM outputs, `solve` and committed words). DShot is untouched by construction |
| P1b | no byte moves by value; version and hash lines move | **holds** for a byte change no field reads: the ELF and board files are identical, 8 generated files move by hash lines only. The bump itself is not measurable today |
| P1c | every PWM output gains `idle` and `resolution`; nothing else on a driverless specification | **holds**, plus the version line on all 29 specifications and both solved topologies |
| P2a | IN/IN both high is brake (slow decay), used by slow-decay PWM | **holds** (SLVSAR1E Table 1/2 p. 9; SLVSDS7B Table 4 p. 11) |
| P2b | PH/EN both high is forward; each mode needs its own rule | **holds**, plus: PH/EN has **no coast**. EN = 0 is brake, and Hi-Z only by sleep (Table 3) |
| P2c | nSLEEP internal pull-down; nFAULT's pull-up value given | **half.** Pull-downs hold (100 kΩ; 500 kΩ). **No pull-up value is specified in either datasheet**; DRV8876 gives only an example (p. 19) |
| P2d | IPROPI 1000 µA/A with a tolerance band; range bounded by VREF | **half.** A_IPROPI is **typical only**, with no min or max; an error band applies by current range (p. 6). The clamp relative to VREF holds (p. 12) |
| P3a | most pins reset to analog; inputs float until the first write | **holds** (p. 541) |
| P3b | ODR before MODER gives a glitch-free initial level | **not stated by RM0433** (searched); only the general ODR statement (p. 534) |
| P3c | ADC 0–VREF+, SMPR per channel, the pin fixed by the package; ADC3 on ports F, C and H | **half.** **No port-H ADC pin on LQFP144** (DS12117 pp. 65–72); ADC3 is on PF3–PF10 and PC2_C/PC3_C |
| P4a | the twin drives an input; IDR follows | **holds** (G3), plus G4's overwrite |
| P4b | PUPDR has no effect | **holds** (G2) |
| P4c | the initial level is witnessed as a write, not as a pad level | **holds** (G1). The timer's pre-`CEN` level is not modelled either (O1–O2, H1–H2) |
| P4d | the stock platform has no H7 ADC instance | **missed.** ADC1 (with ADC2 unmodelled) and ADC3 are instantiated, and a script sets a value |
| P5a | resolution in ps or ticks, not whole ns; 20 kHz at PSC 0 gives ARR 3199 | **holds** |
| P5b | outputs on one timer agree in rate and resolution | **holds**. Today's key is `(protocol, rate)` (`solve.rs:1224`) |

## 7. Stops, and what could not be cited

**STOP:** the work order always stops at §2 for the schema ruling. Decisions Q1–Q10 are below (Q, so they do not collide with fixtures D1 and D2). **Ruled 2026-11-20T14:42Z** (§8).

**NOT FOUND in a held maker's document.** Each is named, and none is filled from a distributor's copy.

**The ruling** (§8): a row field with no held maker's source is not entered. Where a check needs such a value, the specification declares it with a citation (`RUL-14`), or `validate` refuses by name. Each value is listed below with the check that would need it.

| value not found | the check that would need it | as ruled |
|---|---|---|
| DRV8833 maximum input PWM frequency | `InputRateAbovePart`: each bridge input's `rate_hz` against the part's maximum | not entered in the row. A specification declares it with a citation, or `validate` refuses `PartLimitUndeclared {part, input_pwm_max_hz}`. **D2 as written has no citable value, so it is refused by name.** That is the honest result for a part whose maker states no limit. D1's DRV8876 has one (100 kHz, p. 4) |
| A recommended nFAULT pull-up value (both parts; DRV8876 gives only Figure 18's example, p. 19) | none. The specification declares the pull itself (Q1): D1 a 10 kΩ resistor, D2 the MCU's RPU. `PullOverDrive` needs only the row's IOD | not entered. No check needs it |
| DRV8833 tSLEEP and minimum nSLEEP-low pulse; DRV8876 minimum nSLEEP reset pulse | a check on a sleep-then-wake sequence (re-latching straps, clearing a latched fault). No generator step sleeps after a wake today (§2.7 (a)) | not entered. Declared or refused when a step that needs it lands |
| DRV8876 minimum input pulse width | the commanded range's `pulse_min_us` against the part's minimum. No run-time drive exists today (§2.5) | not entered. Declared or refused when a drive API lands |
| A_IPROPI min/max (typical only) | `AdcFullScaleExceeded`. It is judged at the AERR band's worst edge, which **is** found (p. 6, in `parts-rows.json`), so no min/max is needed | not entered |
| An RM0433 "ODR before MODER" guarantee | Kani's BSRR-before-MODER clause (`board.rs:4991–5005`), which cites the general statement (p. 534) | not needed |
| Which silicon revision (Y or V) the H753 carries, so DS12117's two ADC tables (Table 76 p. 166 rev Y; Tables 173/174 pp. 272, 276 rev V) both stand | `AdcSourceImpedance` (RAIN against sampling time) | the rule takes the narrower table, or the specification declares the revision with a citation |

**Finding, not fixed here (Q8):** today's sense rule lands `power.sense` on PF3, an ADC3-only pin (measured, §2.3). `1211a` solves `power.sense` into the ADC map with the ADC1/ADC3 choice.
- **An internal contradiction:** SLVSAR1E Table 3 (p. 11) reports no nFAULT on UVLO, while §7.3.5.3 says nFAULT is driven low. Treat it as unspecified.

## 8. Decisions, as ruled

The architect ruled every lean on 2026-11-20 at 14:42Z (`~/.cache/aet-bus/1211.md`), with the notes under each Q. The split in §5 is accepted as written.

- **Q1 · GPIO and ADC on the device or the MCU?** **Lean: the device** (§2.2 shape A), following `cs` and the row semantics. Shape B is a top-level MCU list.
  - **RULED** (architect, bus 2026-11-20T14:42Z): as the lean. GPIO and ADC signals sit on the device (shape A).
- **Q2 · Bridge: owned by the device or a property of two outputs?** **Lean: the device** (§2.5), because D1's idle spans nSLEEP.
  - **RULED** (architect, bus 2026-11-20T14:42Z): as the lean. The bridge is owned by the device.
- **Q3 · The bridge rule.** The work order's "never driven high together" forbids brake on both parts, slow-decay drive on the DRV8833, and forward in PH/EN (§2.5 table). **Lean: the truth table as row data. A bridge input idles at a `level`, never a pulse (`BridgeIdlePulsed`), and the static idle tuple must be a non-driving row (`BridgeIdleDrives`); forbidden states are row data (empty for both parts); it judges idle only and nothing at run time.** Keep option 1 for a future discrete half-bridge-leg part.
  - **RULED** (architect, bus 2026-11-20T14:42Z): as the lean. The truth table is row data. A bridge input idles at a `level`, never a pulse (`BridgeIdlePulsed`). The static idle tuple must be a non-driving row (`BridgeIdleDrives`). Forbidden states are row data, empty for both parts. The rule judges the idle state only. **The work order's "never driven high together" is withdrawn as wrong for both parts** (§2.5's table).
- **Q4 · `idle`.** Inside `pwm`, required on every PWM output (8 committed). **`pulse_us: 0`:** accept (the work order) or refuse as `IdleSpelledTwice` (**lean**, one spelling per state).
  - **RULED** (architect, bus 2026-11-20T14:42Z): `idle` sits inside `pwm` and is required on every PWM output, with no default (`INV-06`). **`pulse_us: 0` is refused as `IdleSpelledTwice`:** one spelling per state, so `{"level": "low"}` is the only way to say CCR 0.
- **Q5 · `resolution`.** **`resolution_ps`**, required on every PWM output. 1,000,000 restates today, and no word moves. It joins the timer key. `4240b` must re-judge it at its clock.
  - **RULED** (architect, bus 2026-11-20T14:42Z): as the lean. `resolution_ps` is required on every PWM output, and 1,000,000 restates today. It joins the timer key. `4240b` re-judges it at its clock, carried as a §5 note for `1211a`.
- **Q6 · Row data.** **Lean: pins, kinds, levels, pulls, timing and truth tables in the parts row** (parts schema 11), because `validate` must read them. Symbols, footprints and passives go to 4600 D4's question for `1211d`.
  - **RULED** (architect, bus 2026-11-20T14:42Z): as the lean. Pins, kinds, levels, pulls, timing and truth tables go in the parts row (parts schema 11). Symbols, footprints and passives go to `1211d` with 4600 D4's question.
- **Q7 · D1's PH on a timer or a GPIO.** **Lean: allow both** inputs in a bridge.
  - **RULED** (architect, bus 2026-11-20T14:42Z): a bridge input may be a timer channel or a GPIO.
- **Q8 · ADC peripheral choice.** **Lean: the solver chooses ADC1 or ADC3 and refuses an ADC2-only pin as `AdcUnmodelled`** unless it is pinned. `power.sense` is solved into the same map and stays in the power tree. Note that today's rule lands a sense on PF3 (ADC3-only) unchosen.
  - **RULED** (architect, bus 2026-11-20T14:42Z): the solver chooses ADC1 or ADC3. An ADC2-only pin is refused as `AdcUnmodelled` unless the specification pins it, and a pinned one is judged `silicon-only` (Q10's reason). `power.sense` is solved into the same map. **Today's PF3 landing (ADC3-only) is a finding, not a fix here** (§7).
- **Q9 · The pad before `start_outputs`.** **Lean: `motor_output_pupd` follows the idle** (down for low and pulse, up for high).
  - **RULED** (architect, bus 2026-11-20T14:42Z): as the lean. `motor_output_pupd` follows the idle: down for low and pulse, up for high.
- **Q10 · Twin fidelity.** Renode holds idle high low for one period (H2), shows no pre-`CEN` level (O1), and ignores `MOE`/`CCxE` in forced modes (T4–T6). **Lean: judge static idle after one period, and record the window as an acknowledged `silicon-only` reason; do not patch Renode.**
  - **RULED** (architect, bus 2026-11-20T14:42Z): as the lean. Static idle is judged after one period, and the window is recorded as an acknowledged `silicon-only` reason. Renode is not patched.

## 9. Gates, fence, reduction pass

- **Fence:** `git diff --stat 472d2d84 -- crates bridge models examples scenarios boards fab ci .github container Cargo.toml Cargo.lock` is empty. The diff is `spike/1211/` and this file only.
- **`cargo fmt --all -- --check`:** passes.
- **Clippy and `cargo test --workspace`:** not run. No Rust file changed, so both are `main`'s own (CLAUDE.md: "for a spike-only branch, run what applies and say so"). No test reads `spike/` (4600 §9a's reading of `workspace_shape.rs` still holds).
- **Adversarial pass:** none, by the work order (nothing is built).
- **Reduction pass (diff):** −13 net (in the two scripts) · 1 duplicate removed · 1 dead path removed · 0 protected kept · 1 finding below.
  - **Duplicate:** `twin/timer.resc`'s T7 measured idle high a second time, on a pin T6 had left high. `high.resc` (H1–H2) measures it cleanly, so T7 is gone from the script, the log and §4.
  - **Dead path:** `measure.py` compared each DShot output's words with themselves (`today` = `today`). It now records them as `untouched` and compares nothing.
  - **Proof:** both reran in the image after the cut. Six Renode scripts exit 0. Every value that remains is the same as before the cut: G0–G5, T1–T6, A0–A6, O1–O2 and H1–H2 are compared value by value, and Renode's log interleaving is checked by hand at T3/T4. `measure.json` still shows 0 moved.
  - **Finding:** `twin/run.sh` and `measure/bytes.sh` repeat the container recipe (capped-run, flock, mounts) in about 8 lines each. Folding them into one runner saves about 4 lines but means rerunning both evidence scripts for no change in evidence, so it is left. It would be one deletion if these scripts outlive the ruling.
- **Scratch left in `~/.cache/aet-1211`:**
  - held documents and pages, kept for the ruling;
  - `target/` and `target-img/`, the CLI builds (removed at close);
  - `cargo-home/` (removed at close);
  - `src/`, the Renode source (removed at close);
  - `bytes/`, the runs (kept until the ruling).
