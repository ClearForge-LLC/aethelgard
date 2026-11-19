# AET-WO-2304b — FEEDBACK (build)

Branch `wo/aet-2304b-battery-sag`, from `main` at `ee835bde` (`4400g` merged: specification 17, solved 11, trace 14, power-parts 4, scenario `[1, 2, 3]`). The work order was read from the docs branch (copy at `~/.cache/aet-2304b-wo.md` on Furnace); it is not committed here.

Every figure is labelled **measured**, **read at source** or **inferred**. Measured results come after the predictions below, which were committed before any measured run of this build.

## Predictions — written before any measured run of the build

### Already done before this commit (orientation probes, not runs of the build)

- **The ledger, measured on `ee835bde` by `ledger::build` and `acknowledge` (no tool):** Sentinel-03 has **280 of 698 rows unacknowledged**, across 35 registers. They are GPIO A/B/C/E (MODER, AFR, OSPEEDR, PUPDR, OTYPER), I2C2 (CR1, CR2, ICR), RCC (AHB4ENR, APB1LENR), SPI2 (CR2, IFCR, SR), TIM1 (BDTR, CCER, CCMR1, CCMR2, CR1, EGR) and USART2/3 (CR1, CR2).
  - The 2304 spike's verify verdict (main `cd43b207`) shows the same 280.
  - `verify --fab-out` refuses them as `UnwitnessedFieldNotAcknowledged`, before any gate (`verify.rs` L1237). §7 makes that a stop outside this fence. It was reported on the bus.
- **Two open RX lines in one run, measured with `main`'s binary** (serial-four; `serial_rx_open` CRSF and MAVLINK in one scenario, plus a baseline):
  - the run fails with `serial_echo_missing` ×2, matched;
  - `attribute` names `electrical`, rule `E3`, with subjects `port:CRSF` and `port:MAVLINK`, nothing unexplained or masked.
  - So `rail_out_of_band` on `VDD_5V` can be detected with no change to `aethelgard-evolve` (outside the fence), provided kind 5 carries both `serial_rx_open` overrides (§ Proposed names).

### Proposed names and shapes (WO §1.2, §1.6, §1.7)

- `power.battery.series_milliohms`: `{"value", "source"}` or `null`. The key is required.
- `motor_outputs[].esc`: `{"none": {"source": "…"}}` or `null`. The key is required.
- **The `battery_sag` override (scenario 4):** `{"battery_sag": {"series_milliohms": N, "start_ns": T}}`, wire `[6, series_milliohms, start_ns]`.
- **The `rail_out_of_band` override (scenario 4):** `{"rail_out_of_band": "VDD_5V"}`, wire `[7, domain]`.
  - **Deviation for the architect:** the scenario also lists the compositions the rail renders (`serial_rx_open` per port, `spi_device_absent` per device).
  - The CLI refuses unless that list is exactly the set derived from the specification (`RailOutOfBandRendering`).
  - The reason: evolve's E3/E2 evidence, and chaos's `open_ports` and absent set, read only the literal overrides in kind 5. Without them the rail can never be detected.
- **Refusals:**
  - one kind, `BatterySagRefused`, with a message per clause (no tree, not a fault, no operating point, below shutdown, below regulation, past the span, combined);
  - `SeriesResistanceUndeclared`, `EscUndeclared` and `BatterySagSolvedBelowFloor` by their own names, as the WO names them;
  - one kind `RailOutOfBandRefused`, a message per clause.
- **Record kind 31:** `BatterySagDeclared`.

### Predicted

| id | Quantity | Prediction | Basis |
|---|---|---|---|
| Q1 | Sentinel-03 battery side | V_open 9,000,000 µV; U10 input 11,645 mW (5,237,403 µV × 2,000 mA, rounded up, + 1,170 mW loss); U11 3,107 mW (3,417,204 µV × 748 mA, rounded up, + 550 mW); **P 14,752 mW** | inferred from the fixture, IR's ranges (2304 §1.4) and the WO's formula |
| Q2 | The refusal edges at P 14,752 mW | **regulation (U10's 7,000 mV) at 0.949 Ω**: 948 mΩ admitted, 949 mΩ refused (R = (9 − 7)·7 / 14.752). U11's floor is max(3,800, 3,800) = 3,800 mV, at R 1.188 Ω, past the collapse. **The collapse is at 1.372 Ω.** Shutdown (3,800 mV) is dominated by regulation on every real row | inferred |
| Q3 | V_bat at the healthy 1 mΩ | 8,998,360 µV, 1.64 mV below `battery_range`'s 9,000 mV minimum. That is K6's figure: IR counts no path drop | inferred |
| Q4 | V_bat at the tolerated sag, 500 mΩ | about 8,088,000 µV (2304 §1.4: 8.09 V). Static below 9,000 mV, so it covers `battery` | inferred |
| Q5 | The deck's steady state against the static point | within 1 mV at three R points (1 mΩ, 500 mΩ, 900 mΩ). The minimum is the settled point: a step in R under constant-power sinks decays monotonically | inferred |
| Q6 | Decoupling, before and after F3 (IR's `effective`: no maker curve is held for the four Cout parts, so the fixture's 500 ‰ declared derating) | **Sentinel-03:** `VDD_3V3` 10,810,000 → 54,810,000 pF; `VDD_5V` 0 → 44,000,000 pF. **Power fixture:** `VDD_3V3` 8,300,000 → 52,300,000; `VDD_1V8` 200,000 → 700,000; `VDD_5V` 0 → 44,000,000. Under a tolerance-minimum reading (22 µF K, −10 %) the Cout figures would be 79.2 µF a rail; that is not used (one derivation, `RUL-20`) | read at source (rows, derating), inferred (sums) |
| Q7 | Boards without a tree | every byte of `emit`, `solve` and `generate` identical, except `schema_version` and `"esc": null`. Every campaign archive moves only its trace version, its specification's hash (`schema_version`), the bridge digest where the bridge ran, and the hashes that bind them | inferred |
| Q8 | Committed droops under L697 (the loaded floor) | no scenario moves: `sentinel-01/droop-tolerated` 3,150 mV > 3,135; both `droop-bor3` stay below. `motors/dshot600-droop` keeps its refusal or pass as today: it has no tree, so F3 adds nothing | inferred |
| Q9 | Sentinel-03's survived droop (floor 3,000 mV, 500 mΩ, 100 mA, hold 600 µs) | loaded floor 2,950,000 µV, below the 3,135 mV band and above BOR3's region (2.54–2.78 V). 10 τ at 54.81 µF is 274.05 µs, and at the nominal 98.81 µF 494.05 µs; 600 µs clears both. `brownout_reset` 0, `rail_below_tolerance` 1 (info) | inferred |
| Q10 | Sentinel-03's campaign | every scenario matches its expectation. `rail_out_of_band` on `VDD_5V` fails `serial_echo_missing` ×2, E3 | inferred |
| Q11 | Sentinel-03 `verify --fab-out` | **refused `UnwitnessedFieldNotAcknowledged` (280 rows), unless ruled.** If ruled through: about 1,075 s plus about 9 s per scenario boot (11 scenarios), peak 3.4–4 GiB under 16 G | measured (the refusal), inferred (the rest) |
| Q12 | Versions | specification 18, scenario 4 (`[1, 2, 3, 4]`), trace 15; solved 11 and power-parts 4 unchanged | read at source |
| Q13 | Tier-2 count | +3: Sentinel-03's campaign with its replay and determinism; the power fixture's campaign; Sentinel-03's `verify --fab-out` (only if acknowledged) | inferred |
| Q14 | A kind-31 run's archive beyond a baseline's | one record, under 1 KB | inferred |

## What was built (read at source on this branch)

- **Specification 18** (`ir/src/lib.rs`): `power.battery.series_milliohms` (`{value, source}` or `null`; a 0 is refused, an empty source is `InvalidSource`) and `motor_outputs[].esc` (`{"none": {"source"}}` or `null`; hand-written deserializer so an empty object is refused as the missing `none`, a second or unknown key refused). Both keys required. `validate` judges nothing else with either (K6). Solved stays 11; power-parts stays 4 (K3: no field added; the ruled 3.8 V edge is `vin_millivolts.min` and D-B9's floors `regulation_vin_min`, as the WO read).
- **IR's one derivation of the battery side** (`ir/src/power.rs` `battery_side`): V_open = `cells.min × cell_min_millivolts`; the healthy R as declared; per battery-fed buck its input power ⌈VOUT max × load⌉ + `loss_milliwatts` and its two floors (`vin_millivolts.min`; `regulation_vin_min` for its output, through one lookup `regulation_floor` now shared with D-B9's `battery_inputs`); the hot-plug bulk parts (rated C, ESR max) and the battery-fed regulators' input ceramics at IR's credited figure. It runs only after every `check_against` step passes. A battery-fed LDO, a buck with no loss, or no regulation row is `BatterySideUnmodelled` (all three are refused earlier by `validate` on every committed tree; kept as named refusals).
- **F3** (`eda/src/schematic.rs` `decoupling_picofarads`): each regulator output capacitor the emitter draws counts on its rail at `aethelgard_ir::power::output_capacitance`, which calls IR's own `effective` (maker curve where held; the declared derating otherwise; refused where neither covers it). One derivation shared across eda → ir (eda already depends on ir). A declared Cout not drawn on its rail is an internal error.
- **The consumers of a rail** (`ir/src/power.rs` `rail_consumers`): in IR, because the CLI may not read the specification's connectors or device lists (`RUL-16`'s compiled fence, `cli/clippy.toml`).
- **chaos:** `battery.rs` (the integer operating point, rounded down toward refusal; the binding floor; the incremental settling time 10·R·C·V²/(V(2V − V_open)); `check_battery_sag`, one function for the runner and replay); overrides `BatterySag` `[6, mΩ, start_ns]` and `RailOutOfBand` `[7, domain]`; scenario 4; kind 31 `BatterySagDeclared` (rank 6, beside kind 16, so a sag and a droop cannot both be archived); trace 15; `tools_of` names the bridge and ngspice for kind 31; replay re-judges kind 31 (`check_battery_sag_records`) and refuses a rail out of band not rendered by consumers' absences alone (`check_rail_out_of_band`, judged before the joins).
- **bridge `--chain`** (built by a subagent from a written brief, spot-checked: the deck takes integers only, names never reach it): V_open → R(t) (a 1 ns PWL step at `start_ns`, breakpoints set) → `vbat`; bulk C + ESR; the ceramics as one ideal C, labelled; one B-source per sink, I = P / max(V, floor); `.nodeset` at the analytic high root, checked against the first point (50 µV) so the deck starts at the right one of constant power's two solutions; no external source. `--rail` untouched. New `BRIDGE_SOURCE_SHA256` `5bdc40e4…636e`.
- **CLI:** `PowerNotSimulated` deleted; `effective` judges a rail out of band's rendering and a sag's static clauses (so `verify`'s pre-gate pass refuses them too); `prepare` solves the deck once (`BatterySagSolvedBelowFloor` on any floor crossed; a minimum off the static point by more than 1 mV is exit 70); `run_one` archives kind 31 before the boot's records and merges the chain's tools into the header; each campaign line carries `battery_sag` (kind 31's chain) and `conditions`; `verify` carries both per run and FAB-NOTES gains a section stating every condition verbatim (`cli/src/fab.rs`, see deviations).
- **Coverage** (`verify.rs`): `Target::Battery` (its own variant); a sag covers it when its static point lies below `battery_range`'s minimum; `rail_out_of_band` is `Detect(Rail, E3 or E2)` with its consumers' subjects; a non-MCU rail is met when covered and otherwise unmet with why; the blanket lines are gone; the droop arm judges the loaded floor (L697).
