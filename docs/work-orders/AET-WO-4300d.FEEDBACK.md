# AET-WO-4300d — FEEDBACK (spike, then STOP for the architect's ruling)

**Branch:** `wo/aet-4300d-power` from `origin/main` `d5bc841` · **Work order:** `origin/claude/argon-repo-docs-synthesis-rjrb0d`, `docs/work-orders/AET-WO-4300d.md` (lands through docs PR #136; not committed here) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Cadence:** spike (§1), then STOP. No build code before the ruling.

The predictions below were committed before any measured run, and before any datasheet was fetched or any library file was read from the image. They are not edited afterwards. The results section sets each result beside its prediction.

**Labels.**
- **[read]:** read at source, either this repository at `d5bc841`, another builder's branch read with `git show`, or a cited document.
- **[measured]:** run here, with the command and output kept under `spike/4300d/`.
- **[inferred]:** reasoned, neither read nor run.

**Seen before these were written (orientation, not measurement):**
- the work order, the kickoff and `RULE3.md`;
- `AET-WO-4300.FEEDBACK.md` S4 and `AET-WO-1303.FEEDBACK.md`, with its ruling (PR #133, comment 6888926396);
- `4210`'s FEEDBACK on `origin/wo/aet-4210-imu-baro` (the 1.8 V domain, D7);
- `4300f`'s FEEDBACK on `origin/wo/aet-4300f-connectors` (DS-009's "Analog Power" port: VCC +5.3 V, at least 2.5 A over two GH contacts rated 1.0 A each; its D5 and D6);
- `aethelgard_chaos::power` (DS12117 Rev 10 Table 16's BOR thresholds), `aethelgard-ir`'s `Specification` and `parts.rs`;
- `podman images`, to confirm that the pinned tools image `c69ff9bc…` is present.

No datasheet was fetched, no image file read and no container run before this commit.

## Predictions — committed before any measured run

### What the predictions rest on [read]

- `VoltageDomain` is `id`, `nominal_millivolts` and `tolerance_permille`, and the MCU names its `supply` domain (`lib.rs` L56–71). No part supplies a domain today. Sentinel-01 has one domain, `VDD_3V3` at 3300 mV ±50 ‰.
- A `parts.json` row is an I2C or SPI sensor: `bus`, `identity`, `supply_millivolts`, `renode` and three citations (`parts.rs` L78–121). A regulator has no bus, no identity and no Renode model, so it does not fit this row.
- BOR thresholds live in `aethelgard_chaos::power::BOR`, at Table 16's min, typ and max for each edge. VBOR3 falls at 2.54 V minimum. The scenario declares a BOR level, and the specification has none.
- `1303` D4 (ruled): the supply bound's source is `4300d`'s regulator output pad, and the drop is per-pin R × 658 mA (Table 120, Tj 125 °C). D7 (ruled): the VSS return is bounded too, and the limit applies to the sum.

### S1 — the parts

| id | prediction | label |
|---|---|---|
| P1.1 | **Reachability:** `ti.com` serves product PDFs to `curl` from Furnace, and `st.com` does not (the work order says so). `diodes.com` and `analog.com` answer. A vendor may refuse a non-browser user agent. | inferred |
| P1.2 | **The 2S–6S envelope:** 2S at 3.0 V/cell gives 6.0 V; 6S at 4.2 V/cell gives 25.2 V, or 26.1 V for LiHV at 4.35 V/cell. So the buck needs VIN max ≥ about 28 V with margin, and a 5 V output from 6.0 V needs a dropout of 1 V or less at load. | inferred |
| P1.3 | **Buck candidates the pinned KiCad 10.0.7 image ships in `Regulator_Switching`:** TI `LMR33630ADDA` (3.8–36 V, 3 A, SO-8 PowerPAD), Diodes `AP63205WU` (3.8–32 V, 2 A, fixed 5 V, TSOT-23-6) and `LM2596S-5` (40 V, 3 A, 150 kHz, TO-263; old). At least two of the three are present, each with a footprint in the standard libraries. | inferred |
| P1.4 | **Recommended buck: TI LMR33630** (the 400 kHz `A` variant), because 36 V covers LiHV 6S with about 10 V of margin and the datasheet gives guaranteed UVLO and FB limits. It is in production. If its symbol is absent, the `AP63205` is the fallback. | inferred |
| P1.5 | **The 3.3 V rail cannot be an LDO from 5 V at the ruled load:** (5.0 − 3.3) V × about 0.71 A (658 mA plus the sensors) is about 1.2 W, more than a SOT-223's thermal budget. **Recommended:** a second buck for 3.3 V **directly from the battery**, beside the 5 V one, not fed from 5 V. | inferred |
| P1.6 | **Why both bucks run from the battery:** a buck's input current is bounded above only through a guaranteed minimum efficiency, and no datasheet guarantees one (efficiency is a typical curve). So a buck fed from another regulator gives that regulator a load nothing bounds. An LDO's input current is its output current plus its maximum quiescent current, which is guaranteed, so an LDO may hang off any rail. | inferred |
| P1.7 | **The 1.8 V question:** TI `TLV75518P` or Diodes `AP2112K-1.8` from 3.3 V, with ±1.5 % or better over temperature, fits the ICM-20948's VDDIO of 1.71–1.95 V. The SPI crossing needs a translator. TI `TXB0104` (auto-direction, 4 bits: SCK, MOSI, CS out, MISO in) ships in KiCad's `Logic_LevelTranslator` library. **Recommendation:** LDO plus translator as declared parts, with the alternative (an IMU whose VDDIO takes 3.3 V, `4210` D1) named. | inferred |
| P1.8 | **Sense divider:** a pair of 1 % thick-film 0603 resistors (KiCad `Device:R`, `Resistor_SMD:R_0603_1608Metric`, both already in the 17 vendored files). Tolerance and TCR come from the resistor maker's datasheet (Yageo RC0603 or Panasonic ERJ-3EKF). No current-sense part is recommended: an analog INA18x output would be a static full-scale check only, and DS-009's power port already carries CURRENT from an external module. | inferred |
| P1.9 | **Footprint licence:** every regulator symbol and footprint in the image is CC-BY-SA 4.0 with KiCad's design exception, so they vendor into `aethelgard-kicad-lib` as the existing 17 did (`RUL-11`). | inferred |

### S2 — the specification's shape

| id | prediction | label |
|---|---|---|
| P2.1 | `power` names, without restating any domain (`INV-01`):<ul><li>`battery`: `cells`, `chemistry` and per-cell `min_millivolts` and `max_millivolts`, each with a `source`;</li><li>`regulators`: each with `reference`, `part` (a new regulator table), `input` (the battery or a domain id) and `output` (a domain id);</li><li>`loads`: each with `domain`, `milliamps` and a `source`.</li></ul>The MCU's load is not written by hand: it comes from a cited parts row (Table 120). | inferred |
| P2.2 | **The solver's tree:** every domain has exactly one source (a regulator whose `output` it is); every input is the battery or a domain; following inputs from any domain reaches the battery with no repeat. **Refusals:** `DomainWithoutSource`, `DomainWithTwoSources`, `PowerLoop`, `RegulatorOverloaded` (its own domain's loads plus each LDO child's input current, above IOUT max) and `BuckFedByRegulator` (P1.6). | inferred |
| P2.3 | `SolvedTopology` gains the tree: each regulator's reference, its input and output nets, and the pin numbers of its output and ground pads from the part row. That is what `1303` reads (S5). | inferred |

### S3 — the static checks

| id | prediction | label |
|---|---|---|
| P3.1 | **Input range:** battery min (cells × cell min) ≥ the buck's UVLO rising **maximum** and its VIN minimum. Battery max (cells × cell max) ≤ its recommended VIN maximum, not the absolute maximum. Refused `BatteryOutsideRegulatorInput`. | inferred |
| P3.2 | **Dropout:** for an LDO, input domain min ≥ VOUT max + dropout max at the load's current. For a buck at the 2S minimum, the datasheet gives a maximum duty or a minimum off-time; the check is VIN min × D_max ≥ VOUT + I × R_DS(on)max + I × DCR. The LMR33630 gives a dropout curve, typical only, so the buck's dropout is a named condition or a computed bound from guaranteed fields. Flagged as a decision. | inferred |
| P3.3 | **VOUT tolerance:** a fixed part's VOUT min/max must lie inside the domain's nominal ± tolerance. An adjustable part's VOUT uses VFB min/max and the divider's resistor tolerance. The domain's tolerance must also leave room for `1303`'s VDD+VSS drop, so how the 165 mV is split between the regulator and the copper is a decision. | inferred |
| P3.4 | **BOR margin:** the MCU domain's minimum (nominal − tolerance − the copper drop budget) must exceed the declared BOR level's falling **maximum** (VBOR3: 2.68 V). The BOR level is not in the specification today; it should be (one source, `INV-01`), with the scenario reading it. Also 3.3 V − 5 % = 3.135 V ≥ the MCU's 1.62 V and ≤ 3.6 V. | inferred |
| P3.5 | **Sense full scale:** battery max × the divider's maximum ratio (resistors at worst tolerance) ≤ the ADC pin's range, which is 0 to VREF+ (VREF+ ≤ VDDA), at VDDA's minimum. DS12117 states VAIN as 0 to VREF+. | inferred |
| P3.6 | **External parts:** the datasheet's stated ranges are for the inductor (value, saturation current ≥ the peak), the output capacitance (effective, after DC bias derating) and the input capacitance. A capacitor's effective value under bias is not a value the specification can cite from a generic `Device:C` row, so the check needs a capacitor part row with a cited derating, or it becomes a named condition. **Inferred to be a decision.** | inferred |
| P3.7 | **Minimum on-time:** a 3.3 V buck from 26.1 V runs at a duty of about 12.6 %. At 2.1 MHz that is 60 ns, below the LMR33630's minimum on-time. At 400 kHz it is 316 ns, which is fine. So a static check `VOUT / VIN_max ≥ t_on_min × f_sw max` belongs in S3, though the work order's list does not name it. | inferred |
| P3.8 | **Thermal is not in S3's list** but decides P1.5. It is raised as a decision, not built in silently. | inferred |

### S4 — the board

| id | prediction | label |
|---|---|---|
| P4.1 | The buck's footprint has an exposed thermal pad, so the placer and router meet a pad they have not handled: a thermal pad joined to GND with vias. | inferred |
| P4.2 | **The datasheet's layout rules** include the input capacitor as close as possible to VIN and GND (the hot loop), a short SW node, the FB trace away from SW, and thermal vias. **Gateable:** the input-capacitor distance (centre to pin, from the placed board) and the SW node's copper area. The rest stay named conditions. | inferred |
| P4.3 | Placement today has no notion of "close to"; it needs a declared group or adjacency constraint. | inferred |

### S5 — the output pad

| id | prediction | label |
|---|---|---|
| P5.1 | `1303`'s probe picks each non-MCU pad on the rail as a stand-in source (`spike/1303/s4/run_paths2.py`). With `4300d`, the source is one pad named by (regulator reference, the part row's output pin number) from `SolvedTopology`, and the VSS terminal is the same part's ground pin (`1303` D7). | inferred |
| P5.2 | **For a buck, the regulated node is the FB divider's top or the output capacitor**, not the SW pin or the inductor's switch end. So the source pad is the output capacitor's pad (or the FB sense point), and the part row must name which. For an LDO it is the VOUT pin. Flagged as a decision. | inferred |
