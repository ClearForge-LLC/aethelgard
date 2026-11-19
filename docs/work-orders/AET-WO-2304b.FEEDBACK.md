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

## Results — measured after the predictions

### The predictions, against their results

| id | Predicted | Result | |
|---|---|---|---|
| Q1 | Sentinel-03's P 14,752 mW (U10 11,645, U11 3,107) | **measured** by IR's `battery_side` through the campaign's kind 31: U10 11,645 mW, U11 3,107 mW, 14,752 mW; the power fixture 14,642 mW (its U11 2,997 mW: its own `VDD_3V3` load, U12 behind it) | held |
| Q2 | D-B9's edge: 948 admitted, 949 refused; collapse at 1.372 Ω | **measured** (`chaos/tests/battery.rs`, `campaign_tier1`): **949 mΩ admitted at 7,000,070 µV, 950 refused** (`below regulation`, naming U10, 7,000,000 and 3,800,000 µV); collapse between 1,372 (4,601,271 µV) and 1,373 mΩ | **missed by 1 mΩ** (the edge is 0.94902 Ω; the prediction rounded the wrong way) |
| Q3 | V_bat at 1 mΩ: 8,998,360 µV | measured 8,998,360 µV (static); the deck starts at 8,998,361 µV | held |
| Q4 | about 8,088,000 µV at 500 mΩ, below 9,000 mV, covers `battery` | measured static 8,088,035 µV; the deck's solved minimum within 1 µV of it in the tier-2 campaign; `battery` covered | held |
| Q5 | deck = static within 1 mV at three R | measured by the bridge's tests: 2 mΩ 8,996,721 against 8,996,720.58; 500 mΩ 8,088,036 against 8,088,035.67; 900 mΩ 7,140,682 against 7,140,681.73 (µV). Well inside 1 mV | held |
| Q6 | decoupling, F3 | measured exactly as predicted on both boards: Sentinel-03 `VDD_3V3` 10,810,000 → 54,810,000 pF, `VDD_5V` 0 → 44,000,000; power fixture `VDD_3V3` 8,300,000 → 52,300,000, `VDD_1V8` 200,000 → 700,000, `VDD_5V` 0 → 44,000,000; no-tree boards unchanged (8,300,000) | held |
| Q7 | no-tree boards identical but version lines and binding hashes | measured (a subagent's differential of `main` `ee835bde` against this branch at `70adb15b`, before the merge of `main` and the adversarial fixes, none of which touches emit, solve or generate; spot-checked against its reports): 21 specifications solve/emit/generate identical after substituting each specification's sha256 (KiCad and BOM files identical raw), 5 refusals identical; the two tree boards also identical after the same substitution; 27 committed campaigns: every line field equal but the state hash; every `trace.cbor` the same size, and each reproduces the branch's bytes from main's with only the version byte (14 → 15), the specification/base/topology digests, the run id and the bridge digest substituted; the five ELFs byte-identical | held |
| Q8 | no committed droop moves under L697 | measured in the differential: every committed droop scenario's verdict, findings and counts identical; `verify`'s unit tests of the seeded set unchanged; a new unit test pins the loaded-floor rule (3,140 mV at 100 mA through 500 mΩ covers; 3,200 does not) | held |
| Q9 | the droop survives, `rail_below_tolerance` 1 | measured: `droop-tolerated` passes with its counts (brownout_reset 0, unrecovered 0, below_tolerance 1) | held |
| Q10 | every Sentinel-03 scenario matches | measured: 9 of 10 matched on the first run; **`spi-absent-u5` also failed `spi_data_mismatch`** (finding F-IMU); with that expectation changed after the run, 10 of 10 | missed on one |
| Q11 | `--fab-out` refused at the acknowledgements | measured: refused `UnwitnessedFieldNotAcknowledged`, 280 rows, after coverage passed on every row (below) | held |
| Q12 | versions | specification 18, scenario `[1, 2, 3, 4]`, trace 15; solved 11 and power-parts 4 unchanged (read at source; the solve digests move only by the specification's sha256, measured) | held |
| Q13 | +3 tier-2 tests | +2 (the two campaigns); the `--fab-out` run is tier 1, because it is refused before any gate | missed in count |
| Q14 | kind 31 under 1 KB | measured (the tier-2 archive, `replay`): Sentinel-03's baseline 105 events, 7,273 B; the sag run 106 events, 7,362 B: kind 31 and the override's tag add 89 B | held |

### The two tree boards' campaigns (tier 2, in the pinned image)

- **Sentinel-03, 10 scenarios** (`scenarios/sentinel-03/`): every expectation met. The baseline, the tolerated sag (500 mΩ at 10 ms) and the droop pass. `vdd5v-out-of-band` fails `serial_echo_missing` ×2 (CRSF and MAVLINK, one E3); each serial port, SENSORS held low, and FLASH and IMU absent fail by their mechanisms.
- **The power fixture, 3 scenarios**: the baseline and the sag pass; `vdd1v8-out-of-band` (U5 absent) fails by the absence's kinds.
- Every run replays to its own state hash, and a second campaign writes every trace byte for byte again (`INV-04`). Both campaigns together: 187 s wall in the image, scope peak 1.94 GB of an 8 GiB cap (237 ‰).
- **The battery sag's line** (kind 31): static 8,088,035 µV, solved minimum within 1 mV, U10 binding at 7,000,000 µV, with §1.11's sentences, the healthy R's source and each output's `esc` source. The rail's line carries §1.7's sentence verbatim.
- **Attribution of the two IMU absences**, measured: `electrical`, rule E2, subject `device:U5`, nothing unexplained or masked.

### Sentinel-03's `verify` (§2; merged tree `38bdca30`, after `1315` and `4404`)

- **Run once in the pinned image under `capped-run --mem 16G`** (caps read back: `memory.max` 17179869184, `memory.swap.max` 0), with every committed scenario and no `--fab-out` (`~/.cache/aet-2304b/sh/verify-s03.sh`).
  - **Wall 460 s; scope peak 5,718,163,456 B (5.33 GiB, 333 ‰ of 16 G).** §1.10 measured 1,075 s and 3.40 GiB with no scenarios on `main` `cd43b207`; this run adds the campaign (10 scenarios, each with its attribution) and is on the post-`4400g`/`1315` tree. The wall is shorter (not investigated: the box's load differs run to run; nothing here claims a cause).
  - **Every gate passes but `campaign`:** ERC, DRC, support, extract, 18 Kani harnesses, build, simulate and 7 attributions. Every run matches its expectation.
  - **`campaign` fails one detection:** `held-low-sda`, "attribute masks [{index 77, kind 13}] on the faulted bus: something else fails too". Finding F-MASK (below): a rule in `aethelgard-evolve`, outside this fence. Reported on the bus.
- **Re-run at the final head** (`ba8fb43b`, after the adversarial fixes; same script, 16G): **wall 447 s, scope peak 6,497,370,112 B (6.05 GiB, 378 ‰)**; the same verdict: every gate passes but `campaign`, whose one refused detection is still `held-low-sda` (F-MASK). Every run matched; the sag's run carries `battery_sag` (the chain) and 8 conditions, the rail's run 1, every other none. The changed tier-1 targets (`campaign_tier1`, `battery_sag_tier1`, `boot_tier1`, `fab_tier1`) pass in the image as root too (the uid path the host does not take).
- **F-MASK, ruled 21:40Z:** the architect records it as a §7.1 row (any I2C bus with two or more modelled devices meets it). `4202` removes U2 from Sentinel-03, leaving SENSORS one modelled device (U7): after `4202` lands, `main` is merged and this verify re-run at 16G, and the before and after go here, a pass to be read as "one modelled device on the bus", not as the rule fixed. Pending at this writing.
- **`--fab-out`, the same scenarios:** refused `UnwitnessedFieldNotAcknowledged`, **280 rows**, first `RCC.AHB4ENR.GPIOCEN`; the 35 registers as measured before this build. Unchanged by this work (ruled 2026-11-19 20:40Z: the acknowledgements are a person's, a §7.1 row; the package stops here). Coverage, judged first, passes on every row: `VDD_3V3`, `VDD_5V`, `battery`, CONSOLE, CRSF, MAVLINK, SENSORS (declared), FLASH and IMU; `EXT` no device (tier-1 test `sentinel_03_s_scenarios_cover_every_row_and_the_package_waits_on_its_ledger`).
- **The power fixture** (tier-1 test): `CampaignCoverageShort` names `rail \`VDD_5V\` (\`VDD_5V\` has no consumer a judge observes)`; `battery` and `VDD_1V8` are met by its set as declared, before any gate (its campaign's `VDD_1V8` run attributes E2, `device:U5`, which detection accepts: a unit test holds that measured attribution); no package is claimed.

### Mutants (one per guard this build adds; a subagent ran them at `35c94bd3` in its own worktree, every run reported, spot-checked)

30 mutants, **30 killed**, none survived:
- chaos C1–C12: no operating point, below shutdown, below regulation, not a fault, past the span, combined, the scenario-4 gate, replay's static point, solved minimum, binding floor, rail rendering and sag/declaration pairing;
- CLI L1–L13: no tree, `SeriesResistanceUndeclared`, `EscUndeclared`, `BatterySagSolvedBelowFloor`, the deck/static disagreement, the rail on the MCU's supply, unknown domain, no consumer, the rendering mismatch (the architect's "one composition missing, one extra": both are tier-1 cases of the one equality, which L9 removes), `battery` coverage, `rail_out_of_band` coverage, L697, F3;
- IR I1–I5: a 0 mΩ path, an unsourced path, an unsourced `esc`, the input power's loss, the regulation-floor lookup.
Notes from the run: L13 (F3) is killed by the CLI's tier-1 pin, not by any test inside `aethelgard-eda`; L2 and L3 are killed because the mutated run proceeds to the deck (`BridgeUnavailable`, exit 4, against the expected named refusal, exit 2).

## Findings (house schema: finding · where · type · recommendation · decision-needed)

- **F-MASK (ruled 21:40Z: a §7.1 row; 23:10Z: `3305`, #256, owns the fix, by a trace bump after this one) · evolve reads another device's absent address as a witness's wrong address · `crates/aethelgard-evolve/src/lib.rs` `witness_fields` (`elsewhere = absent.iter().copied().find(|a| a != address)`) · defect, measured · with a bus faulted, every device is left out, so the BME280's witness on Sentinel-03 carries the BMP581's 0x46 among its absent addresses; that marks U2's address field wrong, so its witness is software evidence and is masked, and `verify`'s detection refuses the run. No I2C fault on Sentinel-03's SENSORS can be detected until the rule takes only this witness's own transfers (for example, absent addresses between this check's start and the next check's). Sentinel-01 never met it: its TMP117 is unmodelled, so one device answers · outside this fence (evolve); a work order · decision: the architect's.**
- **F-IMU · an absent IMU also fails its data read · `verify.rs` `ABSENT` · expectation changed after a run, with its reason · the bring-up reads an IMU's data as well as its identity (`4220c`), so an absent ICM42688P or ICM20948 fails `spi_data_mismatch` too; `attribute` names E2, subject `device:U5`, nothing unexplained (measured on both). `ABSENT` gained `spi_data_mismatch` and the two IMU expectations gained it after their first run. No coverage is granted without observation: detection still needs E2 with only the seeded subjects · decision-needed: is widening the closed mechanism table the architect's to rule? (default here: widened, flagged).**
- **F-Q2 · D-B9's edge is 949/950 mΩ, not 948/949 · `chaos/src/battery.rs` · prediction miss (rounding) · none.**
- **F-K6 · IR's static headroom counts no input-path drop · `ir/src/power.rs` `headroom` · as K6 ruled: with the healthy 1 mΩ declared, the board at rest sits at 8,998,360 µV, 1.64 mV below `battery_range`'s 9,000 mV minimum · not fixed here; the architect's §7.1 row (K6).**
- **F-LOSS · the declared losses are typical at a 36 V input, used at 9 V · fixture `junction.loss_milliwatts` sources · condition carried from the spike (2304 §1.4) · the chain uses them as declared, as the specification states them; kind 31 carries each sink's figure, not its source text.**
- **F-BULK · the bulk capacitor enters the deck at its rated value, with no tolerance held · `power_parts` `BulkCapacitor` has no tolerance field · the deck's steady state does not depend on C (a step in R under constant power settles monotonically), and the span clause's settle time scales with it: at 500 mΩ the chain settles in well under 1 ms of a 90 ms span · none, named.**
- **F-SCHED · there is no `VDD_3V3` crossing to schedule · the deck solves `VBAT` only (K4), so no `VDD_3V3` node exists; the WO's "a non-empty list is an internal error" is met by construction, and replay refuses any kind 17 in a run without a droop · none.**
- **F-SHARD · `ci/shard.sh` counts tier 2 by binary name (`TIER2_RE`) · a new tier-2 file would not be counted, and `ci/shard.sh` is outside the carve-out · the two tier-2 tests live in `twin_tier2.rs` as module `battery_sag` · none (said on the bus to `1911`).**

## Decisions — each with a recommendation

- **D1 · F-IMU's widening of `ABSENT`.** Recommend keep: the mechanism is the device's absence, measured E2 on both IMUs; the alternative leaves every IMU row unmeetable.
- **D2 · F-MASK.** Recommend an evolve work order (one rule, the absent addresses a witness may cite); until it lands, Sentinel-03's SENSORS row is met only as declared, not after the gates.
- **D3 · power-parts stays 4 (K3).** No field was needed: the floors are `vin_millivolts.min` and `regulation_vin_min`.
- **D4 · `RUL-16` and the rail's consumers (adversarial 6).** `rail_consumers` reads the specification's connectors and device lists inside `aethelgard-ir`, and the CLI decides from its answer. The fence is met in form (IR may read its own specification; the CLI's `clippy.toml` holds), but the runner does decide from declared connectors, which `RUL-16`'s amendment (`4300f`: "a connector is drawn on the board only; nothing solves it, and no runner reads it") meant to keep out. §1.7 asks for exactly that rendering. Recommend: rule it as `RUL-16`'s second exception (as `RUL-16` scoped the schematic emitter), or move the consumers into the solved topology (a solved bump this WO did not take).

## Adversarial pass (a fresh subagent on Opus, at `35c94bd3`, every finding reproduced; spot-checked and each fixed with a test and a mutant)

1. **High: only the first `rail_out_of_band` was judged**, so a second one escaped every refusal (the MCU's supply, an unknown domain, no consumer) and the first still covered its rail. Fixed: one rail per scenario, refused when the scenario is read and in replay (`check_rail_out_of_band`, shared).
2. **Medium-high: a rail feeding a regulator was not its consumer**, so a rail out of band could be rendered as two RX lines while it fed the MCU's own regulator (latent: no committed board). Fixed: refused, naming the regulator (`rail_consumers.regulators`).
3. **Medium: a rail beside another fault passed every pre-run check** and failed only after the boot, as exit 70. Fixed: refused when read (same shared rule).
4. **Medium: replay did not re-judge kind 31's shape or "combined"** (start 0, R past 10 kΩ, 500 V, 0 pF, 0 floors, two sags in kind 5). Fixed: `check_battery_sag` judges the shape the parser and the deck hold; replay judges "combined".
5. **Medium: one observed consumer could cover the whole rail.** Fixed: `Mechanism.required`, a rail is detected only with each rendered consumer's `port:` or `device:` subject attributed (the controller admissible, not required: the power fixture's measured E2 attribution cites `device:U5` alone; an advisor caught a first fix that demanded the controller too).
6. **Medium: `RUL-16` in form only.** For the architect: D4.
7. **Low: the battery's coverage test does no work beyond "not a fault"** (K6). The comment now says so.
8. **Low: `unwrap_or(0)` where a row's figure must be positive.** Fixed: a named refusal (`BatterySideUnmodelled`, `PowerValueInvalid`).
9. **Low:** (a) a TX-only peer on a rail's connector is now named in the condition; (b) a zero discriminant is now "no operating point", not "past the span".
- **Held** (the subagent's list, abbreviated): the static edges (949/950, 1,372/1,373) and overflow to refusal; the deck against the static point at the edges, near the collapse and at the latest admitted start, all within 1 mV (off by at most 211 µV); the deck's high root; every default that must halt halts by name; no request text reaches ngspice; no external source; `--rail` unchanged; L697 moves no committed droop; F3 read from IR, never a value string; replay refuses doctored kind 31s and crossings without a droop; the §1.7/§1.11 text verbatim; `Target::Battery` cannot collide with a domain.
- **Mutants of the fixes** (one each, run on the host, each reverted): one rail, rail alone, the shape, combined in replay, zero discriminant, the regulator-fed rail, every subject attributed (and again on its `required` form): **8 of 8 killed**.

## What moved, before → after (§1.13)

- **Versions:** specification 17 → 18 (`ir/src/lib.rs`; the asserts in `ir/tests/{bench,board,motor,orientation,serial,validate}.rs`, `cli/tests/{bench,outline}_tier1.rs`, `ir/README.md` and its test); scenario `[1, 2, 3]` → `[1, 2, 3, 4]`; trace 14 → 15 (`chaos/src/event.rs`).
- **Fixtures and examples:** every `schema_version` (26 files: `examples/` ×4, `crates/aethelgard-cli/tests/fixtures/` ×21, the power fixture). `"esc": null` added to every motor output of `motors-dshot600`, `motors-pwm`, `sentinel-02-shaped` and `serial-dshot-9600` (four each); Sentinel-03 declares `none` on MOTOR1–4 with the source Scotty confirmed (2026-11-19; the WO's working-declaration wording is superseded by its "Ruled at writing" item); Sentinel-03 and the power fixture declare `series_milliohms` 1 mΩ (K2's wording, ASCII: `board::source` admits printable ASCII only, so "mOhm" and "MOTOR1-4").
- **Goldens:** `chaos/tests/golden.rs` trace `bcd082c2…8383` → `10d566b5…fe1a` (the version byte alone: set back to 14 it is the old hash, asserted) and state `ada1b631…d8f7` → `4b06f855…cbf2`.
- **Hash pins** (each moved only with a specification's sha256, measured): `cli/tests/solve_cli.rs` reference `6bbb87ea…fe03` → `c7cc4fc4…5a1c`, stress `22bb9cb6…e6b8` → `316928c0…d502` (with the old sha256 put back, the old digests exactly); `svd-hal/tests/order_2203.rs` `main.rs` `5c770e1c…` → `aa59544e…`, platform `88ab17b6…` → `701d03eb…`, faulted `2a8151a8…` → `35a1f8d6…`, boot image unchanged; `boards/sentinel-01/**` and `boards/sentinel-01-bench/**` (16 files), `hwdef/tests/fixtures/{sentinel-01,sentinel-02,serial-dshot-9600}.{hwdef.dat,topology.json}`, `svd-hal/tests/ui/{motors-pwm,serial-four}`, `cli/README.md`: each specification hash line, old → new (sentinel-01-minimal `89865e50…` → `e47d3e71…`, sentinel-01 `4846be8b…` → `5dbe999e…`, motors-pwm `b180c715…` → `315d0b4d…`, sentinel-02-own `2abfae5c…` → `c8b7d32d…`, serial-dshot-9600 `2e60ed27…` → `d427808e…`), the byte-for-byte generation tests passing on the result. `boards/**` is fenced: these are its hash lines only, as `4400g` moved them.
- **The bridge:** `BRIDGE_SOURCE_SHA256` `91e72fcc…cebb` → `5bdc40e4…636e` (`cli/src/analog.rs`). It is in the header of every archive whose run used the bridge: every board with an I2C bus (its rise times), every droop and every battery sag, so all 27 committed campaigns' archives (measured in the differential).
- **Decoupling (F3):** the tree boards' rails as Q6; no committed pin moved (no test pinned a tree board's decoupling before; `battery_sag_tier1` pins them now).
- **Not moved:** the solved topology (11), power-parts (4), every no-tree board's emit/solve/generate output beyond the hash, every ELF.
- **`fab/sentinel-01-bench/package/`:** re-derived by the architect's `write=true` dispatch (run 44624527136 on `08e2aaf7`, artifact `bench-package` 14579394591) and committed byte for byte. Two files moved, as `4400g` F4's did: `MANIFEST.json` (`specification_sha256` `4846be8b…4395` → `5dbe999e…e082`, `topology_sha256` `116ae1d9…bb68` → `341d07a5…cb58`, `verdict_sha256` `3e0bc749…5728` → `9165385c…a905`, and FAB-NOTES' entry; file `6f7187ba…` → `731a0f35…`) and `FAB-NOTES.md` (its specification line; `1f06d677…0845` → `7f9c43a1…db01`). The 15 Gerber, drill, placement, BOM and ELF files are byte-identical. `bench_board_tier1` passes, on the tree with `3403` merged too.

## Deviations and fence edges

- **`cli/src/fab.rs`** (not in §3's list): FAB-NOTES gains a section stating the campaign's conditions verbatim (§2 requires them there). It is empty, so no byte moves, for every campaign without a sag or a rail.
- **The tier-2 tests are in `twin_tier2.rs`** (module `battery_sag`), not a file of their own: `ci/shard.sh` counts tier 2 by binary name, and it is outside the carve-out (F-SHARD).
- **`rail_out_of_band`'s shape** lists its compositions beside it (ruled 20:40Z, item 2).
- **`ABSENT` widened by `spi_data_mismatch`** (F-IMU, D1).
- **Two scenario expectations changed after their first run:** `sentinel-03/spi-absent-u5` and `sentinel-02-power/vdd1v8-out-of-band`, each with `spi_data_mismatch` (F-IMU). Every other expectation stands as committed before the first run (`b47945aa`).

## For the architect's docs PR (not edited here: D6, K8)

- `fab/sentinel-01-bench/BENCH-EXT-BMP581.md` §0 cited `examples/sentinel-01-ext-bmp581.json`'s sha256 at specification 17 (`f8fa067e…cda4`). `3403` merged it as specification 17; this branch, landing second, bumps that example to 18. **Updated here** to `1a6aa374…2d92`, specification 18, by the architect's 23:10Z fence widening (that one line only); if `4240v` takes 18 first, it is updated again on the renumber.

- Architecture §2 / `RUL-17`: a second solve-once deck, `--chain` (the battery's `VBAT` only, constant-power sinks, one transient); the bridge's digest `5bdc40e4…`.
- Architecture: specification 18, scenario 4, trace 15, kind 31; `RUL-16`'s question (D4).
- Roadmap: L697 done (the loaded floor); L707's battery-sag half done, Sentinel-03's package waits on its acknowledgements (§7.1) and on F-MASK for SENSORS after the gates; the power fixture gets no package (`VDD_5V` has no consumer); K6's §7.1 row (IR's headroom counts no path drop).

## Gates (at `f44e80bb`, re-run in part at `ba8fb43b`; `main` `96517013` merged)

- `cargo fmt --all -- --check`: clean. `cargo clippy --workspace --all-targets --features aethelgard-cli/with-tools -- -D warnings`: clean (host and pinned image). The bridge's own `fmt`, `clippy -D warnings` and `test` (23 unit, 7 `chain.rs`, 1 `edges.rs`, 5 `rail.rs`): clean, in the image.
- The `gates` job's set (`ci/gates.sh`'s targets, with the target directory inside the repository as CI has it): **133 test binaries green; one red, as expected: `bench_board_tier1`'s `the_committed_package_is_bound_to_its_specification_scenarios_and_emit`**, until the architect's `write=true` dispatch re-derives `fab/sentinel-01-bench/package/` (AWAITING-PACKAGE).
- Tier 2 in the pinned image: `twin_tier2`'s `battery_sag::*` (2 tests) pass, 170 s; `ci/shard.sh plan`: 8 shards at 1,888 s planned each, guard ok. `TIER2_MIN` 190 → 192.
- Reduction pass (diff): −39 net · 0 duplicates removed (the one found during the build, `RailOutOfBandRefused`'s kind, was made one constant there) · 5 dead fields/methods removed (`BatterySink.cite`, `.part`, `.output`; `BatteryCapacitor.how`, `.part`; `CreditedCapacitor.how`; `BatterySide::input_milliwatts`) and one dead test closure · 0 protected kept · 0 findings below.

## CI (PR #254, run 44621743414, head `08e2aaf7`)

- Every job green but the one expected red test, `bench_board_tier1`'s `the_committed_package_is_bound_to_its_specification_scenarios_and_emit` (in `gates` and in tools shard 1, which stopped there), and `bench-package`: AWAITING-PACKAGE. `TIER2_MIN` 192 read by the aggregate.
- Both tier-2 tests pass in CI. Their weight rows are replaced from this run by rule 5 (`ok` less twin_tier2's `Running`, an upper bound): Sentinel-03's 288.767 s, the power fixture's 88.251 s (the Furnace estimates were 338.856 and 110.348).
