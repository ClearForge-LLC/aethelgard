# AET-WO-2304b — The battery-sag build: a declared chain solved once, so a board with a power tree runs its campaign, covers its battery, and gets a fabrication package (Sentinel-03 first)

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` **after `AET-WO-4400g` merges**, named in the kickoff (`56639a89` when written; verify live HEAD).
**Branch:** `wo/aet-2304b-battery-sag`.
**Author:** Claude (architect) · **Builder:** strong tier (the `2304` builder, reused), with a full adversarial pass.
**Phase:** P4, Sentinel-03 before Sentinel-02 (roadmap L367, L375). Scotty's fabrication quote waits on Sentinel-03's package (L419–424). Written 2026-11-19. **Cadence: build, with stops (§7).**

**Grounds:**
- **The spike, merged as the record** (#241). Its ruling is comment 7014981925. `docs/work-orders/AET-WO-2304.FEEDBACK.md` is the oracle: D1–D7, F1–F11, §1.4 (the chain), §1.6 (costs), §1.7 (coverage), §1.8 (decoupling), §1.10 (verify's weight). `spike/2304/` holds the evidence: `chain/chain.py` (the static mirror), `lockstep/src/main.rs` (the deck), `decouple/`, `out/`.
- **The ruling, as given:**
  - **D1:** (a), a declared chain solved once for the campaign. (b), lock-step at 100 µs, goes to the flight-loop work order. The margin is the 1.47 A ESC lever at 1.0 Ω. **(a) must refuse wherever it would be wrong (F5).**
  - **D2:** the `esc` power-parts kind, as shaped, is built with `4500`, not here.
    - Each motor output names an ESC or declares `none` with a source.
    - A battery-sag scenario on a board whose outputs declare neither **halts by name** (`EscUndeclared`).
    - **Sentinel-03: `none`, source "J5 is a bench header".** It is asked of Scotty and recorded when he answers. Until then the build uses it as the working declaration and says so.
  - **D3:** the healthy path is `Battery.series_milliohms {value, source}` in the specification. The fault is `battery_sag.series_milliohms` in the scenario. Refuse an R with no operating point, or below a battery-fed regulator's declared shutdown (F5).
  - **D4:**
    - `battery` is met by the new kind. This is a rule addition, not a relaxation (the architect's).
    - `VDD_5V`, **option 1**: an excursion is rendered as its consumers' absence. CRSF and MAVLink RX go open, and the serial judge detects it. It is a modelling assumption, and **every verdict that uses it names it.**
    - `VDD_1V8`: option 1, with its own consumers. Counting it covered by the BOR3 argument is not adopted.
    - `4201` F5 goes to the next coverage work order, not here.
  - **D5:** (a0) + F3 + (a), Sentinel-03 first, refusing per D-B9 (F11).
    - `battery` is covered by a **tolerated** sag.
    - Admitting a brownout below `regulation_vin_min`, with `VDD_5V` named unstated, would relax what the verdict covers. That is Scotty's. It is put to him; the build does not wait on it.
    - The buck's enable edge is the stated 3.8 V operating minimum, the conservative edge. Typicals are not ruled in.
    - L697 (the loaded floor) rides along. L799 (the writer's bound) does not.
  - **D6:** close L696 and reword L722. The architect's docs PR does that, not the builder. L695's pull-up half and L723 go to a static work order.
  - **D7:** precede `4500`.
  - **F9:** `4400g` bumps the solved topology 10 → 11.
- **The block, at source** (main `56639a89`):
  - `PowerNotSimulated`: `crates/aethelgard-cli/src/campaign.rs` L376–390, inside `effective` (L372). Its test is `crates/aethelgard-cli/tests/campaign_tier1.rs` L2701–2722 (`a_scenario_on_a_power_tree_is_refused_before_any_run`).
  - `--fab-out` requires `--scenarios` (`cli/src/main.rs` L124–126, `INV-03`).
- **The droop path you extend:**
  - `resolve_droop`: `cli/src/campaign.rs` L266. It refuses `BorLevelContradictsSpecification` (L276) and `DroopNotMcuSupply` (L285).
  - `Override` and `Droop`: `chaos/src/record.rs` L173–216. The wire encoding is at L903–913 and L963–967; `name_ok` at L1917.
  - `RawDroop`: `chaos/src/campaign.rs` L176, `droop_from` L381.
  - `tau_ns` L421, `rail_levels` L431 (the loaded floor), `droop_end_ns` L450, `check_droop` L474.
  - The version gates: L580 and L643–657.
  - `check_run`'s ranks: L755–780. `check_droop_records`: L1265. `check_joins`: L2583. `tools_of`: L2870. `check_archived`: L2893.
  - `SCENARIO_VERSIONS = [1, 2, 3]`: `chaos/src/campaign.rs` L32.
  - `TRACE_VERSION = 13`: `chaos/src/event.rs` L43. The last record kind is 30, `JudgeStop` (`record.rs` L1869).
  - `MAX_RECORDS` 65,536: `replay.rs` L74. `MAX_TRACE_BYTES` 256 MiB: `archive.rs` L26.
  - `power::schedule`: `chaos/src/power.rs` L152.
- **Coverage today** (`cli/src/verify.rs`):
  - `Target` at L650, with `Rail(String)` at L656. `stress` at L705. `SerialRxOpen` at L774.
  - The droop arm, L785–796, compares the **declared** floor with the band. That is L697, and L706 on main.
  - `coverage` at L882–946:
    - every rail but the MCU's supply is unmet (L929–937);
    - `rail \`battery\` (no fault kind droops it)` is added whenever a tree exists (L941–944).
  - The `VDD_5V` assert is at L2064.
- **The bridge** (`bridge/`, outside the workspace, `RUL-17`):
  - `--rail` takes a closed 12-key request with one trapezoid (`bridge/src/rail.rs` L17–30).
  - The CLI's `rail()` is `cli/src/analog.rs` L280. `rail_answer` (L347) refuses a level crossed more than twice (L393–398).
  - The bridge's modes are at `bridge/src/main.rs` L75–89. CI runs its `fmt` and `clippy` (`ci.yml` L35–36) and builds its tests (L186).
- **IR's static chain** (`crates/aethelgard-ir/src/power.rs`):
  - `Battery` L200–207 has no R. `BATTERY` L78, `BATTERY_NET` L82.
  - `battery_range` L609. D-B9's refusal is `BatteryOutsideRegulatorInput`, at L1316–1335. `headroom` L1448, which counts "no input-path drop" (L1504). `power_in` L1587.
  - **Power-parts** (`power_parts.rs`, version 4 at L35):
    - `Buck.vin_millivolts` is "the maximum of the minimum operating input" (L89–91). It is 3,800 mV for the LMR33630ADDA (`data/power-parts.json`, SNVSAN3F p. 7).
    - `regulation_vin_min` (L105–108) is 7,000 mV for outputs above 3.3 V and 3,800 mV from 0 mV (p. 9, 7.7).
    - Nothing requires `regulation_vin_min ≥ vin_millivolts.min` (L1656–1680).
    - `Kind` is at L534.
  - `MotorOutput` is `ir/src/motor.rs` L82: "Every key is required (`INV-06`)" (L77–79).
- **The decoupling (F3):**
  - `decoupling_picofarads` is `eda/src/schematic.rs` L1118. It counts only parts whose owner `decouples`, parsed by `picofarads` (L1150).
  - A buck's `Cout` is drawn with no owner (`eda/src/schematic_power.rs` L431–433).
  - Measured: 10.81 µF counted on Sentinel-03's `VDD_3V3`, against 98.81 µF with U11's four outputs (54.81 µF at the fixture's 50 % DC-bias rule). `dshot600-droop`'s shape is then `DroopRefused` (FEEDBACK §1.8).
- **The two power trees** (the only committed ones, per `1315`):
  - `crates/aethelgard-cli/tests/fixtures/sentinel-03.json`: U10 → `VDD_5V` (2,000 mA), U11 → `VDD_3V3` (748 mA). Four DShot outputs leave on J5, which carries no `VBAT`. J6 (CRSF) and J7 (MAVLINK) carry `VDD_5V`. J8 carries `VDD_5V` to `EXT`, which has no device.
  - `crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json`: as above, plus U12 → `VDD_1V8`, whose one consumer is U5 (ICM20948 on SPI `IMU`). It has no motor outputs, no serial ports, and no `VDD_5V` consumer.
- **Verify's weight (FEEDBACK §1.10):** Sentinel-03's `verify` with no scenarios takes **1,075 s** and peaks at **3.40 GiB** (213 ‰ of 16 G). It passes 24 gates. A `--fab-out` run adds about 9 s per scenario boot, plus the chain's solve.
- **CI:**
  - `TIER2_MIN` is 190 (`ci.yml` L390). There are 8 shards.
  - `1910`'s planner refuses a heavy test with no row in `ci/test-weights.tsv`, and refuses a stale row.
  - `bench-package` runs on every PR. Its `write` dispatch (`ci.yml` L7–12) is how `fab/sentinel-01-bench/package/` is re-derived (`MANIFEST.json` L48 `verdict_sha256`, `cli/src/fab.rs` L694). `4400g` F4 is the precedent.

> **What this is:**
> - `PowerNotSimulated` lifted. A board with a power tree runs its campaign;
> - F3: a regulator's output capacitors counted in the rail's decoupling;
> - a `battery_sag` fault kind:
>   - the path R steps from the healthy R to a declared fault R;
>   - the load is the tree's declared static load;
>   - VBAT is solved once by a bridge chain deck;
>   - it is refused wherever (a) would be wrong: no operating point, below a battery-fed regulator's shutdown, or below its `regulation_vin_min` (D-B9);
> - the two specification fields it needs, `battery.series_milliohms` and `motor_outputs[].esc` (`none` only);
> - `battery` covered by a tolerated sag;
> - `VDD_5V` and `VDD_1V8` covered by their consumers' absence, the assumption named;
> - L697: the droop's coverage judged at its loaded floor;
> - **Sentinel-03's fabrication package, with the chain declared and its verdict saying what it does not claim.**
>
> It is NOT:
> - (b) lock-step, a per-quantum sample kind, the DShot stop at the run's end, or the writer's 256 MiB bound (L799 / main L808). Those go to the flight-loop work order;
> - the `esc` power-parts kind, an ESC's idle or throttle current, or an ESC step profile. Those are `4500`'s (D2, K5);
> - a brownout below D-B9 (Scotty's), a reboot loop, or any `VDD_3V3` crossing on a tree board from the battery;
> - regulator outputs, enable hysteresis or hiccup in the deck (K4), or the datasheet's typical UVLO;
> - IR's static headroom taking the healthy R. Propose it in FEEDBACK (K6);
> - ESR on ceramics, or L695's pull-up and capacitance half (L723, L695: a static work order);
> - `4201` F5's bus-coverage rule (the next coverage work order);
> - the Sentinel-02 power fixture's package: its `VDD_5V` has no consumer to render (§1.7);
> - any edit to the roadmap, architecture or northstar. The rows go in FEEDBACK for the architect's docs PR (D6, K8).

**Ruled at writing (the architect, 2026-11-19).** Each item was open when the order was drafted:
- **`VDD_5V` and `VDD_1V8` get a seeded fault of their own (K1).** A scenario-4 override, `rail_out_of_band {domain}`, runs for the whole run and is rendered through the existing `serial_rx_open` and `spi_device_absent` compositions. It is a fault authored apart from the board (`INV-07`), and every verdict that uses it names the modelling assumption. The power fixture's `VDD_5V` has no consumer, so it stays unmet, and that fixture gets no package. Say so.
- **Sentinel-03's healthy `series_milliohms` (K2)** is J4's stated 0.80 mΩ, rounded up to 1 mΩ.
  - Label it: "connector only; pack and leads unmeasured (asked of Scotty)".
  - It is a lower bound on the path, and the verdict says so.
- **Power-parts stays at schema 4 (K3).** The ruled 3.8 V edge and the D-B9 floors are already held.
- **The deck solves `VBAT` only (K4).** No unsourced deck figure enters the product.
- **No ESC step profile here (K5).** It belongs to `4500`.
- **The battery's "past the tolerance" (K6)** rests on the fault's R being above the healthy R.
  - Record as a finding that IR's static headroom counts no input-path drop.
  - It is not fixed here. The architect opens a §7.1 row.
- **`TIER2_MIN` (K7):** the one line in `ci.yml` is a carve-out. Say so on the bus to `1911` before you push it.
- **Sentinel-03 declares no ESC (Scotty, 2026-11-19, confirmed).** J5 is a bench header. When ESCs are added they are fed from the pack, not through the board's `VBAT` copper. Use `none` with that source.
- **The `esc` field is built now,** as a required key that takes `null` (the `motor.rs` house rule). Every motor fixture gains `"esc": null`, and Sentinel-03 declares `none` with its source. Without it, Sentinel-03 cannot get a package.
- **F3's capacitance is DC-bias-derated where the part's curve is held.** Where it is not, use the part's minimum by tolerance and name the condition. Never use the bare nominal (`RUL-14`). This is one derivation, shared with IR.
- **EXT keeps today's rule:** named in the verdict, not counted as covered. It is not a relaxation, because nothing about it changes.

## 1. Build

0. **Predict, then build.** Commit a prediction table to FEEDBACK before the first measured run. Label each line measured, read or inferred, as the spike did. It covers at least:
   - Sentinel-03's battery-side input power, and the R at each refusal edge. Inferred from the spike's numbers with `P_in = Σ (each battery-fed regulator's output maximum × its load + its declared loss_milliwatts)`: about 14.75 W, so D-B9's edge is near 0.95 Ω and the collapse near 1.37 Ω. The spike's 14.16 W used the η in the source text;
   - the decoupling on each tree board's rails, before and after F3;
   - every pinned figure that moves, and the tier-2 count.
1. **Versions** (`RUL-14`'s numbering, `ir/src/lib.rs` L42–80):
   - After `4400g` merges, `main` is at specification 17, solved 11, trace 14, power-parts 4, scenario `[1, 2, 3]`.
   - **This build:** specification **18**, scenario **4**, trace **15**.
     - Solved stays 11: no solved field is added.
     - **Power-parts:** 5 is the spike's figure, for the buck's enable edge. Read at source, the ruled edge is already held as `vin_millivolts.min` (3,800), and D-B9's floors as `regulation_vin_min`. So the expected answer is **no new field, power-parts stays 4** (K3, default unless ruled). Add a field only if the chain needs a figure the row lacks, and say which in FEEDBACK.
   - **`4240v` runs beside you and also bumps the specification.** Whichever merges second merges `main`, takes the next number, and re-pins every assert, golden and fixture line. If `4240v` lands first, you are 19.
2. **The specification (`INV-01`: the types live in `aethelgard-ir` only):**
   - **`power.battery.series_milliohms`:** `{value, source}` or `null`. It is the healthy path: pack, leads, the input connector and `VBAT` copper (D3).
     - The key is required, as every key is. `null` means undeclared. It is never defaulted (`INV-06`).
     - `value` is a positive integer. An empty `source` is refused.
     - `validate` reads it and judges nothing new with it (K6).
   - **`motor_outputs[].esc`:** `{"none": {"source": "…"}}` or `null`, a required key.
     - `null` is undeclared. `none` needs a non-empty source. `4500` adds the part variant beside `none`.
     - **Why now and not with `4500`:** Sentinel-03 has four motor outputs. Without the field, its battery sag halts `EscUndeclared`, and it gets no package. The field rides the same bump, so it costs nothing extra.
     - Every committed specification with motor outputs gains `"esc": null`. Five CLI fixtures carry them (`motors-dshot600`, `motors-pwm`, `sentinel-02-shaped`, `serial-dshot-9600`, `sentinel-03`). List each one.
   - **Sentinel-03 declares:**
     - `esc: {"none": {"source": "J5 is a bench header (J5 carries MOTOR1–4 and GND, no VBAT; AET-WO-2304 §1.2). Working declaration, asked of Scotty 2026-11-19, not yet confirmed"}}` on each output;
     - `series_milliohms` as K2 rules. **Default unless ruled:** `{"value": 1, "source": "J4 XT30PW-M contact resistance 0.80 mΩ (AMASS spec V1.2), rounded up: the connector alone; the pack and leads are unmeasured (F2), asked of Scotty"}`.
     - The power fixture declares the same shape for J1.
   - Propose the names in FEEDBACK. Ask only if the shape cannot be the one above.
3. **Lift `PowerNotSimulated`** (a0).
   - Delete the refusal (`cli/src/campaign.rs` L376–390) and its comment.
   - Its test flips: a tree board's baseline campaign runs and passes.
   - A tree board's `droop` keeps every refusal it has (`DroopNotMcuSupply` stays).
4. **F3: count a regulator's output capacitors.**
   - Each `Role::Cout` part (`schematic_power.rs` L431–433) counts toward its rail's `decoupling_picofarads`.
   - Take its capacitance from its row, never by parsing the value string (`"22u GRM…"` does not parse in `picofarads`).
   - **Nominal or derated:** take the figure IR's static checks credit that part (the fixture's `dc_bias_derating` where declared). One derivation, not two (`RUL-20`). State both numbers per rail in FEEDBACK. If they cannot share one function across the `eda` → `ir` direction, stop and say why.
   - **`1315` rewrites `schematic_power.rs`** (#243, READY, +425 lines). If it lands first, merge `main` and apply F3 to its code. Do not resolve by taking either side whole.
   - A board without a power tree has no regulator, so no byte moves. Prove it (§2).
5. **The chain's arithmetic: one function, one home each.**
   - **In `aethelgard-ir`** (it reads the specification and the parts table): one public derivation of the battery side. It returns:
     - V_open = `cells.min × cell_min_millivolts`;
     - the healthy R;
     - each battery-fed regulator's input power, as its output maximum × its load + its declared `loss_milliwatts`, from the same `loads` and output ranges `headroom` uses;
     - each one's **floor**: for a buck, max(`vin_millivolts.min`, its `regulation_vin_min` for its output); for an LDO, max(`vin_millivolts.min`, VOUT max + dropout max);
     - the bulk capacitance on `VBAT` (the `hot_plug` rows, with ESR max) and the battery-fed regulators' input ceramics.
     - It shares no code with the generator (`RUL-20` (1)). The deck takes its numbers from this one derivation (the spike's reduction finding).
   - **In `aethelgard-chaos`** (the format's crate, so replay can re-judge without IR, as `4400g` placed its realized period): the operating point.
     - Constant power P behind R from V_open, with no ESC current in this build. V_bat = (V_open + √(V_open² − 4·R·P)) / 2, in integers.
     - `None` when V_open² < 4·R·P.
   - Integer units throughout: µV, mΩ, mW.
6. **The `battery_sag` override (scenario 4), and its refusals.**
   - **Fields:** `series_milliohms` (the whole path R under the fault) and `start_ns` (when the path steps from the healthy R to it). It is held to the run's end, and no recovery is claimed. Propose the names in FEEDBACK.
   - **Refused by name, before any run** (the `check_droop` precedent: one kind, `BatterySagRefused`, with a distinct message per clause, unless separate kinds read better; say which):
     - **no power tree** in the base;
     - **`series_milliohms` undeclared** (`null`) in the specification: `SeriesResistanceUndeclared`;
     - **a motor output with `esc: null`**: `EscUndeclared`, naming the first such output;
     - **not a fault**: the declared R is at or below the healthy R;
     - **no operating point** at the declared R (F5);
     - **below shutdown**: V_bat under any battery-fed regulator's `vin_millivolts.min` (F5);
     - **below regulation**: V_bat under any battery-fed buck's `regulation_vin_min` for its output (D-B9, F11), naming the regulator and both figures;
     - **past the span**: its effects run past the boot's span (the `droop_end_ns` precedent);
     - **combined**: more than one `battery_sag`, or a `battery_sag` with a `droop`, in one scenario. Their interaction is not modelled.
   - **The shutdown clause cannot fire on a real table** while every `regulation_vin_min` is at least `vin_millivolts.min`. Test it through a test-only table that validation admits (the `1315` precedent). If you show it is dominated by construction, fold the two clauses into one message naming both floors, and say so.
   - On Sentinel-03, every admissible sag lies between the healthy R and D-B9's edge (about 0.95 Ω, inferred). The spike's 0.5 Ω gives V_bat ≈ 8.09 V.
7. **`VDD_5V` and `VDD_1V8`: option 1 (D4), and the fault that seeds it (K1).**
   - Under D5's refusal a battery sag never moves `VDD_5V` out of its band. So option 1 needs its own fault.
   - **Default unless ruled:** a scenario-4 override `rail_out_of_band {domain}`. The named rail is held below its band for the whole run, and rendered as each declared consumer's absence through the compositions that exist today:
     - a serial port whose RX net is on a connector that also carries the rail → `serial_rx_open` on that port. On Sentinel-03, `VDD_5V` gives CRSF (J6) and MAVLINK (J7);
     - a device whose `supply` is the rail → its absence. On the power fixture, `VDD_1V8` gives U5 → `spi_device_absent`.
   - **It adds no composition.** If it needs one, stop.
   - **Refused by name:**
     - on the MCU's supply (that is `droop`'s);
     - on an unknown domain;
     - on a rail with no consumer a judge observes. The power fixture's `VDD_5V` is refused, and stays unmet.
   - **A consumer with nothing to render** is named in the verdict, never counted. Sentinel-03's J8 → `EXT` has no device.
   - **Every verdict that uses it carries the condition**, verbatim in FAB-NOTES: "`VDD_5V` below its band is rendered as its consumers' absence (CRSF and MAVLINK RX open; `EXT` has no device and is not rendered). How a DS-009 peer behaves below 4.75 V is not declared; absent is a modelling assumption (AET-WO-2304 D4)."
8. **The deck: VBAT, solved once** (`bridge/`, `RUL-17`).
   - **A new mode, `--chain <request.json>`**, beside `--rail`, which stays byte-identical. It takes a strict JSON object of integers with its own closed key list. The bridge writes the deck itself, so no request text reaches ngspice (`INV-06`).
   - **The deck:**
     - V_open behind the path R. R steps from healthy to fault at `start_ns`;
     - the bulk capacitor with its ESR max;
     - the input ceramics as ideal capacitors (their ESR is not held: L723), labelled so;
     - one constant-power sink per battery-fed regulator.
   - **What it does not model (K4, default unless ruled):** regulator outputs, enable hysteresis, hiccup, an ESC.
     - Under §1.6's refusals every battery-fed regulator is inside its stated input range for the whole run. So each output is in its stated band **by IR's D-B9 rule**, and `VDD_3V3` cannot cross BOR3.
     - The record says so: "in band by rule (D-B9), not solved".
     - This keeps the spike's unsourced figures out: the 50 mΩ regulator R, the 1 ms ESC lag, and `avail` read as an output.
   - **The answer:** V_bat's minimum, and the time of any crossing of each floor.
     - Any crossing refuses the run by name (`BatterySagSolvedBelowFloor`). That is the dynamic counterpart of §1.6. Static and solved must agree, and the adversarial pass hunts where they do not.
     - A sink below its floor is not solved through. The deck guards constant power near 0 V.
   - **F6:** any `external` source is written `dc 0`. A solve-once deck should need none.
   - `bridge/tests/chain.rs`:
     - the solved steady state equals §1.5's operating point at three R points, within a stated tolerance;
     - every malformed request is refused.
   - The CLI calls it beside `analog::rail` (L280), with the same sandboxing.
   - **The crossings of `VDD_3V3` go through `power::schedule`.** Under the refusals there are none, so no reset is rendered. A non-empty list is an internal error, never a verdict.
9. **The trace (15) and replay (`INV-04`).**
   - **One new kind, 31:** the battery-sag declaration. It carries:
     - V_open, the healthy and fault R, and `start_ns`;
     - each sink's input power and floor, by reference;
     - the static operating point, the solved minimum, and the binding floor.
   - **The override encodings:** `battery_sag` and `rail_out_of_band` take the next two wire tags after `[5, port]`. Add them to `name_ok`.
   - Give it a rank in `check_run`, a place in `check_joins`, and a tool in `tools_of` (the bridge, as `RailDeclared` gets).
   - **Replay re-judges kind 31 from its own fields**, with §1.5's chaos function: the operating point, every refusal clause, and the solved minimum against the floor. It refuses a record any clause would have refused. This is the `check_droop_records` precedent (L1265).
   - Replay refuses an archive at trace 14 by name, as it does today.
10. **Coverage** (`verify.rs`).
    - **`battery`.** A `battery_sag` stresses `Target::Battery`, a variant of its own (no domain id can collide with it). It does so when its static V_bat lies below `battery_range`'s minimum, the declared quantity moved past its declared tolerance (L704's principle). A sag that passes covers it.
      - K6: with the healthy R declared, the healthy board already sits below that minimum by I·R_healthy, because IR counts no path drop. Report both figures. The clause "the R exceeds the healthy R" (§1.6) is what keeps a non-fault from covering.
    - **`rail_out_of_band`** is a detected fault: `Seeded::Detect(Target::Rail(id), mechanism)`. Its mechanism's kinds and subjects are its rendered consumers' (E3 for the serial ports; the SPI absence's for a device). It covers its rail only beside a baseline, as every detected fault does.
    - **`coverage`:**
      - a non-MCU rail is met when covered;
      - otherwise it is unmet, with its reason. "No consumer a judge observes" is the power fixture's `VDD_5V`;
      - `battery` is met when covered.
      - The blanket lines at L929–937 and L941–944 go.
    - **L697 (main L706):** the droop arm (L785–796) judges the band at `rail_levels`' loaded floor. The scheduler and coverage then read one floor.
      - Inferred: no committed scenario moves. `sentinel-01/droop-tolerated.json` is 3,200 − 50 = 3,150 mV, still above the 3,135 mV band; the two `droop-bor3` files stay below it. Show it.
11. **The verdict says what it does not claim** (`RUL-19`'s habit). Every campaign verdict with a `battery_sag` carries these, verbatim in FAB-NOTES:
    - "The battery's load is the tree's declared static load; no motor current is commanded, so no reboot loop is shown or excluded."
    - "A battery sag is admitted only while every battery-fed regulator stays at or above its stated regulation floor (D-B9); no brownout is shown or excluded."
    - The healthy R's source, and each output's `esc` declaration with its source (the working-declaration wording included).
12. **Sentinel-03's scenarios** (`scenarios/sentinel-03/`, authored apart from the board, `INV-07`).
    - **The set:**
      - a baseline;
      - a tolerated `battery_sag`;
      - a survived `droop` on `VDD_3V3` below its band at the loaded floor, at F3's capacitance (hold ≥ 10 τ);
      - `rail_out_of_band` on `VDD_5V`;
      - `serial_rx_open` on CONSOLE, CRSF and MAVLINK;
      - an I2C fault on SENSORS;
      - `spi_device_absent` on FLASH and IMU.
    - `EXT` is named "no device: nothing to fault".
    - **Write each expectation from the specification before the first run.** A changed expectation after a run is a finding, with its reason.
    - **The power fixture's set:** a baseline, a tolerated sag, and `rail_out_of_band` on `VDD_1V8`. It is for the campaign only.
13. **Every moved figure is re-pinned from a measured run, each cited to it:**
    - the specification asserts (`ir/tests/{orientation,serial,board,bench}.rs`, `readme_versions.rs`);
    - every fixture's and example's `schema_version`, `examples/sentinel-02-bench.json` included;
    - the chaos goldens (`chaos/tests/golden.rs`, `archive.rs`);
    - the coverage strings (`verify.rs` L2064);
    - the decoupling pins.
    - Name each in FEEDBACK with before and after. Architecture's and the roadmap's figures go in FEEDBACK for the docs PR.

## 2. Acceptance

- **Sentinel-03's package.**
  - `aethelgard verify --spec crates/aethelgard-cli/tests/fixtures/sentinel-03.json --scenarios scenarios/sentinel-03/… --fab-out <dir>` passes and writes the package.
  - Its verdict holds the chain as declared (kind 31's fields, the solved minimum, the binding floor), with coverage met on every row: `VDD_3V3`, `VDD_5V`, `battery`, CONSOLE, CRSF, MAVLINK, SENSORS, FLASH and IMU. `EXT` is named no-device.
  - **Its verdict and FAB-NOTES carry §1.7's and §1.11's conditions verbatim**, the "no motor current is commanded, so no reboot loop is shown or excluded" sentence above all.
  - Run it once yourself under `capped-run --mem 16G`. Record the wall time and peak memory against §1.10's 1,075 s and 3.40 GiB.
- **The power fixture's campaign runs:**
  - its baseline, tolerated sag and `VDD_1V8` absence pass as expected;
  - its verdict reports `VDD_5V` unmet, with its reason;
  - no package is claimed.
- **Each refusal has a tier-1 test and a mutant a test kills:**
  - no operating point;
  - below shutdown (through the test-only table);
  - below `regulation_vin_min`;
  - `EscUndeclared`;
  - `SeriesResistanceUndeclared`;
  - not a fault, past the span, combined, and no tree;
  - `BatterySagSolvedBelowFloor`, through a derived deck or request;
  - `rail_out_of_band`'s three refusals;
  - replay's re-judgement of a doctored kind 31;
  - a trace-14 archive.
- **Determinism:** a tree board's campaign run twice is byte-identical in every trace, and `replay_dir` reproduces each state-sequence hash.
- **Every board without a power tree is byte-identical, except its version lines.**
  - Run `main`'s binary and this branch's over every specification under `examples/` and `crates/**/tests/fixtures/` (`emit`, `solve`, `generate`), and over every committed scenario's campaign.
  - The only differences allowed are `schema_version`, `"esc": null`, the trace version, and the hashes that bind them. List every one.
- **The bench package is re-derived by the architect.**
  - `fab/sentinel-01-bench/package/MANIFEST.json` moves through the specification hash and the verdict line.
  - Park the PR **AWAITING-PACKAGE** and ask for the `ci.yml` `write=true` dispatch on your branch.
  - Commit the artifact byte for byte, then `bench_board_tier1` passes. Name each moved file and hash, as `4400g` F4 did.
- **The CLAUDE.md gates pass**, and the bridge's own (`cargo fmt|clippy|test --manifest-path bridge/Cargo.toml`). CI is green. **`TIER2_MIN` is exact**, each new tier-2 test named in its comment.
- **`1910`'s planner passes:** every heavy test has a weight row from a measured CI run, and no stale row is left. Sentinel-03's `--fab-out` test is the heavy one. A tools shard over 55 minutes is a stop.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-03`, `INV-04`, `INV-06`, `INV-07`; `RUL-05`, `RUL-14`, `RUL-16`, `RUL-17`, `RUL-19`, `RUL-20`.
- **Fence:**
  - `aethelgard-ir`: the power types, the battery-side derivation, the `esc` field, validation and the specification schema;
  - `aethelgard-chaos`: the overrides, the scenario version, the operating point, the record kind, the trace version, replay and the campaign checks;
  - `aethelgard-cli`: `campaign.rs`, `analog.rs` (the chain call), and `verify.rs` (stress, coverage, the verdict's conditions);
  - `bridge/`: the `--chain` mode, its deck and its tests;
  - `aethelgard-eda`: **the decoupling count (F3) only**;
  - tests, goldens, fixtures, `examples/**` version lines, `scenarios/sentinel-03/`, the power fixture's scenarios;
  - FEEDBACK.
  - **Carve-outs, said on the bus to `1911` before you push:** the rows your tests need in `ci/test-weights.tsv`, and the `TIER2_MIN` line with its comment in `ci.yml` (K7).
  - **Not:**
    - `.github/workflows/` beyond that one line;
    - the rest of `ci/`;
    - `container/`;
    - Renode models, `boards/**` and every `.repl`;
    - `aethelgard-twin`;
    - the emitter beyond F3;
    - `fab/**` except the dispatched artifact;
    - the steering documents, the root `README.md`, `LICENSE-*` and `docs/inputs/**`.
- **Running beside you:**
  - **`4400g`** (#238) is your base. Do not start before it merges. Re-read every cite above on the new base.
  - **`4240v`** also takes a specification bump. The second to merge takes the next number (§1.1).
  - **`1911`** owns CI. Coordinate the weight rows and `TIER2_MIN`. Merge `main` before READY.
  - **`4404`** (hwdef) reads `aethelgard-ir`'s types. `MotorOutput` gains `esc`, so say so on the bus before you push `ir`.
  - **`3403`** adds `examples/sentinel-01-ext-bmp581.json`. The second to land bumps its version line.
  - **`1315`** (#243, READY) rewrites `schematic_power.rs`. F3 sits in it. If it lands first, merge `main`.
- One container of yours at a time, under `capped-run`, at `nice -n 10`; `--mem 16G` for `verify`.
- **Close:** the adversarial pass, the reduction pass (diff-sized; one line in the gates), then `docs/work-orders/AET-WO-2304b.FEEDBACK.md`, then one PR, left unmerged. Write `~/.cache/aet-2304b/pr-title-body.md`.

## 5. Adversarial pass, full

Prefer a fresh subagent. It attacks the finished branch.

- **(a) where it would be wrong:**
  - a declared R that passes the static check but has no operating point, or one below a floor, in the solved deck (or the reverse);
  - an R at an edge, within one unit of each clause;
  - the shutdown and regulation clauses reached in either order.
- **A default where absence must halt:**
  - `series_milliohms` `null` or missing;
  - `esc` `null` on one output of four;
  - a `none` with an empty source;
  - a regulator with no `loss_milliwatts`;
  - a tree with no `hot_plug`;
  - no bridge on the path.
  Each must halt by name.
- **A gate read from the artifact (`INV-07`):**
  - a scenario expectation copied from a run;
  - a floor or load taken from the emitter rather than the specification and the table;
  - a decoupling figure parsed from a value string.
- **Coverage without observation:**
  - `battery` covered by a sag that moves nothing (R at the healthy R);
  - `VDD_5V` covered with CRSF or MAVLINK not rendered;
  - `EXT` counted;
  - the power fixture's `VDD_5V` met;
  - `Target::Battery` met by a voltage domain named `battery`.
- **A condition that goes missing:** a verdict with a `battery_sag` or a `rail_out_of_band` that lacks §1.7's or §1.11's text, or a FAB-NOTES that drops it.
- **Replay:**
  - a kind 31 whose fields would have been refused;
  - a doctored solved minimum;
  - a trace-14 archive;
  - a run whose `VDD_3V3` crossing list is non-empty.
- **The unwired path:**
  - a board without a tree whose bytes move beyond version lines;
  - `--rail` changed in any byte;
  - a committed droop whose coverage moved under L697.
- **The deck:**
  - request text reaching ngspice;
  - a constant-power sink solved below its floor;
  - an `external` source not at `dc 0`.

## 7. Flag-and-stop conditions

- **A brownout appears to be needed:** any acceptance item that only a crossing of BOR3, or a V_bat below D-B9, can meet. Stop. That is Scotty's (D5).
- **The coverage rule can be met only by relaxing it:** `battery`, `VDD_5V` or `VDD_1V8` met without a judge observing something, or `rail_out_of_band` needing a new composition. Stop.
- **A figure no source states is needed** (an ESC current, a pack R, a regulator output model, a typical threshold). Stop with what is missing. Never estimate it (`INV-06`).
- **Sentinel-03's `--fab-out` is refused for a reason outside this fence:** an unacknowledged ledger row (`UnwitnessedFieldNotAcknowledged`), a fab, extract or simulate gate. Stop with it. Never edit an acknowledgement, a bound or a fixture's declared figure to pass.
- **A board without a power tree moves beyond its version lines,** or `--rail`'s output moves. Stop with the bytes.
- **F3 cannot share IR's capacitance derivation** across the crate direction, or the layering forbids a dependency the chain needs. Stop and say which rule.
- **A shard passes 55 minutes, or a run passes 900 ‰ of its cap or is killed.** Stop with the numbers. A cap is never widened here.
- **A schema bump beyond §1.1:** power-parts 5, or a solved field. Say why first.
