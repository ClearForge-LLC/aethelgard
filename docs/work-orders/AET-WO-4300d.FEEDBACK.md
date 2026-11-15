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

## Results — measured or read after the predictions commit (`7508585`)

### Where it ran

- **Datasheets:**
  - `spike/4300d/fetch.sh`, curl from Furnace into `~/.cache/aet-4300d/ds/`, with `fetch.log` and `SHA256SUMS`;
  - text extracted with PyMuPDF (`uv --with pymupdf`);
  - pages rendered wherever a column was in doubt.
- **KiCad 10.0.7's libraries:** copied out of the pinned tools image `c69ff9bc…` with `podman create` plus `podman cp` (`kicadlib.sh`). The container never starts, so no process runs and no cgroup or caps apply. `kicadlist.py` writes `kicadlib.out`.
- **No container ran.** S4 asked for reading, not routing (`1307`/`1308`), so rule 3's scope was never needed.
- **The static checks:** `checks.py` writes `checks.out`, Python arithmetic on guaranteed fields only.
- **Two delegated reads:** subagents wrote `read-bucks.md` and `read-ldo-xlat.md`. I spot-checked these rows against rendered pages, and each matched:
  - LMR33630 p7–8: VFB 0.985–1.015 V; fSW A 340–460 kHz; DDA RDS(on) max 160/110 mΩ; tON-MIN max 108 ns; tOFF-MIN max 85 ns; ISC 3.85–5.05 A;
  - TLV755P p5–6: ±1 %/±1.5 %; dropout 380/400 mV for 1.8 V DBV; UVLO 1.21–1.44 V; PSRR typical;
  - TXU0304 p9: TMAX 50 Mbps (3.3 V to 1.8 V) and 100 Mbps (1.8 V to 3.3 V).

  The buck reader missed one guaranteed field. LMR33630 p7 gives "VIN minimum operating input voltage" at a **maximum of 3.8 V**, so the VIN-range check needs no UVLO typical.
- **DS12117 Rev 10** (`607b9ead…c032`) was read and rendered by me.

### Each prediction beside its result

| id | result | label | verdict |
|---|---|---|---|
| P1.1 | **TI:** `ti.com` serves every PDF (HTTP 200) and the product pages. **Diodes:** `diodes.com` serves both PDFs, but its product pages return a Cloudflare challenge ("Just a moment…"), so their production status is unreadable. **st.com:** fails (HTTP/2 `INTERNAL_ERROR`). **Others:** `tme.eu` 403 (AMASS); Yageo 404; Panasonic timed out. **Vishay** serves its PDF. AMASS's V1.2 spec came from a distributor copy (`store.comet.bg`). | measured | partly: Diodes' pages and tme.eu were not predicted |
| P1.2 | Envelope: 2S at 3.0 V/cell is 6.0 V; 6S is 25.2 V; LiHV is 26.1 V. The LMR33630 recommends VIN 3.8–36 V (p6 §7.3). | read | right |
| P1.3 | **All predicted symbols ship:**<ul><li>`LMR33630ADDA` (`extends` `LMR33640ADDA`);</li><li>`AP63205WU`/`AP63203WU`;</li><li>`LM2596S-5`;</li><li>also `TPS62933`, `LMR51430` and many more (`kicadlib.out`).</li></ul>**Each default footprint is in the standard libraries, but:**<ul><li>the LMR33630's is `Texas_HSOP-8-1EP_3.9x4.9mm_P1.27mm_ThermalVias`, whose 8 thermal vias are `thru_hole` pads;</li><li>the plain `Texas_HSOP-8-1EP_3.9x4.9mm_P1.27mm` exists, with all pads `smd` on F.Cu.</li></ul> | measured | right, plus the footprint finding (S4) |
| P1.4 | **The LMR33630ADDA, A (400 kHz), is confirmed as the only candidate with:**<ul><li>VIN margin (36 V recommended, 38 V absolute, against 26.1 V);</li><li>guaranteed maxima on tON-MIN and tOFF-MIN.</li></ul>**The alternatives fall short:**<ul><li>TPS62933: VIN max 30 V, timing and RDS(on) typical only;</li><li>LMR51430: timing typical only, and its typical curve cannot make 5 V/3 A from 6.0 V;</li><li>AP63205: 2 A, with no tOFF, maximum duty or dropout data.</li></ul>**Status and source:** TI's page shows **ACTIVE**. The datasheet is SNVSAN3F (Nov 2020), `3b0920a4…c04a`. | read / measured | right |
| P1.5 | 3.3 V from an LDO off 5 V at 0.68 A is about 1.2 W, more than a SOT-223 can shed. **3.3 V from a second LMR33630 straight from the battery** passes every check except VOUT on 1 % dividers (P3.3, D13; `checks.out`). The LDO's thermal arithmetic used no datasheet; it is a decision (D1). | inferred / measured arithmetic | right |
| P1.6 | **Wrong** (amended after the adversarial pass, A3). No efficiency is guaranteed: all four bucks give typical curves only. But none is needed. In steady state a buck's average input current is D × IOUT ≤ IOUT, plus its own bias, which is not guaranteed. The high-side current limit, ISC max 5.05 A (p7; TI note 4: "tested, open loop"), bounds the fault and start-up case. So a 3.3 V buck off the 5 V buck is bounded at 2.68 A against 3 A, and passes (`checks.out`). My first version of this row said ISC was the only bound, and that it refused the chain. | read | wrong (D2) |
| P1.7 | **TLV75518PDBV (SBVS320D, ACTIVE) fits:** 1.773–1.827 V over Tj −40…125 °C (±1.5 %, at IOUT 1 mA), inside VDDIO 1.71–1.95 V. Dropout is 400 mV at 500 mA and 125 °C. **AP2112K-1.8:** guaranteed ±1.5 % only at 25 °C, with a typical-only tempco; rejected. **TXU0304PW (SCES935A, ACTIVE),** not the TXB0104, is the translator. See the 1.8 V section. | read | partly: the translator differs |
| P1.8 | **Vishay CRCW0603 e3 (doc 20035, 14-Apr-2026):** ±1 % (and ±5 %) only, ±100 ppm/K, 75 V maximum working voltage. Yageo and Panasonic were not fetchable. `Device:R` and `R_0402` are vendored today; `R_0603` is not. **The full-scale check passes, but the sense fails on source impedance:** a 100 k/10 k divider is 9.09 kΩ, against RAIN ≤ 1150 Ω at 12 bits (DS12117 Table 173, p273, rev V). Not predicted. No current-sense part is recommended. | read / measured | partly; RAIN not predicted |
| P1.9 | **The image carries no `LICENSE.md` beside `symbols/` or `footprints/`** (`podman cp` found none). That is `1300` D10's finding, still true. The libraries are CC-BY-SA 4.0 with the design exception, read from `aethelgard-kicad-lib`'s vendored `LICENSE.md` (`RUL-11`). | measured / read | right |
| P2.1–P2.3 | The shape below, as predicted, plus the power-in connector, `crossings` and feedback dividers. | proposal | — |
| P3.1 | **Battery min ≥ VIN min-operating max 3.8 V** (p7: a guaranteed maximum, so no UVLO typical is needed), and **battery max ≤ VIN recommended max 36 V.** Passes for 2S, 6S and 6S-HV. | measured arithmetic | right, with a better field |
| P3.2 | **LDO:** VIN min (3.146 V after the amendment, the 3.3 V buck's minimum) ≥ 1.827 + 0.400 V. Passes. **Buck at 2S:** a bound from guaranteed fields alone, D ≤ 1 − tOFF-MIN(max) × fSW(max) = 0.961 (no credit for foldback). That leaves 5.389 V available at 2 A against VOUT max 5.245 V (amended; break-even DCR 102 mΩ). **Conditions:** the inductor's DCR (30 mΩ assumed, from an inductor row not yet held), and the datasheet's "VIN = 12 V unless stated" condition on tOFF. TI's own system table states 5 V regulation only from 7 V, and is not production tested. | measured arithmetic / read | right |
| P3.3 | **Amended after the adversarial pass (A1, A2).** The first version counted only VFB and the divider at 1 % + 100 ppm/K over 65 K, and passed. It left out two things: the buck's own regulation band, −1.5/+2.5 % from 0 A to maximum load (p9 §7.7, "not ensured by production testing", rendered); and IFB. Its resistor temperature was also wrong.<br>**Now, with the resistor at 125 °C (100 K) and its drift:**<ul><li>**1 % CRCW fails both buck domains** (5 V: 4.640–5.482 V; 3.3 V: 3.085–3.603 V).</li><li>**0.1 %/25 ppm TNPW, divider centred in the window** against the asymmetric band (every E96 pair searched). Both rails pass:<ul><li>with RFBT near 100 k: 102 k/25.5 k gives 5 V at 4.816–5.245 V (66/5 mV), and 105 k/46.4 k gives 3.3 V at 3.146–3.421 V (11/44 mV);</li><li>with RFBT above 100 k, plus TI's feedforward capacitor: 422 k/107 k gives 12/48 mV, and 232 k/102 k gives 22/25 mV.</li></ul>A first amendment aimed the divider at nominal and missed 3.3 V by 2 mV; that was an artifact of aiming at nominal, caught at review.</li><li>1.8 V: 1.773–1.827 V, a pass.</li></ul>**So the LMR33630 alone uses about 4 % of a ±5 % domain:** 0.1 % dividers fit, 1 % cannot, and the 3.3 V rail's minimum leaves 11 mV of copper room at the domain floor. D3 and D13. | measured arithmetic | **wrong** |
| P3.4 | **BOR:** rev V Table 115 (p212) equals rev Y Table 16, which `power.rs` cites, value for value (F1). The BOR level is still only a scenario field (D4). | read | right |
| P3.5 | Full scale at 6S-HV, worst ratio (1 % + 100 ppm/K × 100 K + 1 % drift, amended): 2.505 V ≤ VREF+ = VDDA min 3.135 V. Passes. **Source impedance fails** at 16, 14, 12 and 10 bits (Table 173: 170, 435, 1150 and 5650 Ω, with no sampling-time condition), and passes only at 8 bits (P1.8, A4). | measured arithmetic | right, plus RAIN |
| P3.6 | **The LMR33630's external parts:**<ul><li>Cin ≥ 10 µF ceramic plus a mandatory 220 nF;</li><li>CBOOT 100 nF;</li><li>CVCC 1 µF;</li><li>RFBT 100 kΩ, maximum 1 MΩ;</li><li>Cout at most 10 × the design value or 1000 µF;</li><li>L from Eq. 4 at K = 20–40 %, with Isat ≥ ISC max ideally and never below the low-side limit (p21–23).</li></ul>**Gaps:**<ul><li>Table 9-2 gives **rated** C. The datasheet defines effective C (bias and temperature) as what matters, and a vendor's DC-bias curve is typical, so effective C has no guaranteed source.</li><li>p22 contradicts itself on the inductor: Eq. 5 is labelled L_MIN, while the sentence above it describes a maximum from a 10 % ripple floor.</li><li>An L sized for 30 % ripple at 26.1 V gives 6 % ripple at 6.0 V, below that floor. The datasheet does not resolve this trade-off for a 2S–6S range.</li></ul>D6. | read | right, with the Eq. 5 finding |
| P3.7 | tON at 26.1 V and fSW max: 401 ns (5 V) and 266 ns (3.3 V), both ≥ 108 ns. The B (1.4 MHz) and C (2.1 MHz) variants fail at 3.3 V. | measured arithmetic | right |
| P3.8 | Raised as D1; the LDO's own rise is 7.8 K (inferred). | — | — |
| P4.1 | **The router refuses** every pad that is not a front SMD pad (`crates/aethelgard-eda/src/route.rs:1876`). That covers the ThermalVias variant (8 `thru_hole` pad-9 copies plus a `B.Cu` pad 9) **and both XT30 and XT60** power-in footprints (all `thru_hole`). The plain HSOP-8 passes that test. | read at source / measured | right, wider than predicted |
| P4.2 | **LMR33630 §10.1 (p33–34) states:**<ul><li>Cin closest to VIN/PGND (pins 2/1, adjacent);</li><li>CVCC close;</li><li>CBOOT short and wide;</li><li>the FB divider close to FB, its VOUT leg away from SW;</li><li>a ground plane on a middle layer;</li><li>the EP soldered to ground;</li><li>wide VIN/VOUT/GND;</li><li>a minimum SW node;</li><li>a **4 × 3 array of 10-mil thermal vias** under the PAD;</li><li>a 4-layer board, 2/1/1/2 oz.</li></ul> | read | right |
| P4.3 | **Amended (A6).** `pcb.rs` L413–426 places each part in rings beside the IC it shares the most nets with, at that IC's lowest pad on the first shared net. So nearness exists, but only to the anchor IC. The LMR33630 has no VOUT pin, so RFBT, the inductor and Cout share `VDD_3V3` with U1 (the MCU) and with the regulator equally (ties go to the lower reference). They would be placed by the MCU, not by the buck. | read | wrong as first written |
| P5.1 | 1303's probe addresses a pad as (reference, pad number) and takes every non-U1 pad on `VDD_3V3` as a stand-in (`spike/1303/s4/run_paths2.py` L20–24). | read | right |
| P5.2 | **The LMR33630 has no VOUT pin.** Pin 6 `VCC` is the internal 5 V bias (power_out in KiCad). The output is sensed only at FB (pin 5) through RFBT. The EP is AGND, "all electrical parameters measured with respect to this pin" (p5). The TLV75518's OUT is pin 5 and GND pin 2. | read | right; see S5 |

### S1 — the table

| Part | Role | Datasheet (held) | Production | KiCad 10.0.7 symbol → footprint | Fields a static check reads |
|---|---|---|---|---|---|
| **TI LMR33630ADDA** (A, 400 kHz) | buck, battery → 5 V and battery → 3.3 V | SNVSAN3F, Nov 2020, `3b0920a4…c04a` | ACTIVE (ti.com) | `Regulator_Switching:LMR33630ADDA` → `Package_SO:Texas_HSOP-8-1EP…_ThermalVias` (TH vias; the plain variant's pads pass `route.rs:1876`'s test, not routed here) | VIN 3.8 (max of min-operating)–36 V; VFB 0.985–1.015; fSW 340–460 kHz; tON-MIN ≤ 108 ns; tOFF-MIN ≤ 85 ns; RDS(on) ≤ 160/110 mΩ; ISC 3.85–5.05 A; IOUT 0–3 A; IQ ≤ 34 µA; EN 1.2–1.26 V |
| TI TPS62933 | buck (alternative) | SLUSEA4D, Aug 2022, `16ec2eac…` | ACTIVE | `TPS62933` → `SOT-583-8` | VIN 3.8–30 V; timing and RDS(on) typical only |
| TI LMR51430 | buck (alternative) | SLUSEF4A, Nov 2022, `f7d80538…` | ACTIVE | `LMR51430` → `SOT-23-6` | UVLO rising max 4.5 V; timing typical only |
| Diodes AP63205WU | buck (alternative, 2 A) | `ef99daa3…` | **unreadable** (Cloudflare): Scotty | `AP63205WU` → `TSOT-23-6` | no tOFF, DMAX or dropout |
| **TI TLV75518PDBV** | LDO, 3.3 V → 1.8 V | SBVS320D, Sep 2024, `d617843b…2e97` | ACTIVE | `Regulator_Linear:TLV75518PDBV` → `SOT-23-5` | VIN 1.45–5.5; VOUT ±1.5 % (Tj ≤ 125 °C, 1 mA); VDO ≤ 400 mV at 500 mA; IOUT 500 mA; IGND ≤ 40 µA; Cout 1–200 µF; PSRR typical only; RθJA 231.1 °C/W |
| Diodes AP2112K-1.8 | LDO (rejected) | DS39724 Rev 2-2, `ef8d376f…` | **unreadable**: Scotty | `AP2112K-1.8` → `SOT-23-5` | ±1.5 % at 25 °C only |
| **TI TXU0304PW** | translator, SPI 3.3 V ↔ 1.8 V | SCES935A, Nov 2021, `425338d9…550f` | ACTIVE | `Logic_LevelTranslator:TXU0304PW` → `TSSOP-14` | VCCA, VCCB 1.08–5.5 V (no order); A1–A3 → B, B4 → A; TMAX ≥ 50 Mbps down, ≥ 100 Mbps up; tpd ≤ 19/15 ns; skew ≤ 8 ns; Schmitt inputs |
| TI TXB0104PW | translator (rejected) | SCES650K, `9a852f7b…` | ACTIVE | `TXB0104PW` → `TSSOP-14` | VCCA ≤ VCCB; ±20 µA drive; load ≤ 70 pF; pull-ups > 50 kΩ |
| **Vishay CRCW0603 e3, 1 %** | FB dividers, battery sense | doc 20035, 14-Apr-2026, `113d2d57…` | not read (no lifecycle field on the page): Scotty | `Device:R` (vendored) → `Resistor_SMD:R_0603_1608Metric` (not vendored; `R_0402` is) | ±1 %; ±100 ppm/K; 75 V; 0.1 W |
| **Vishay TNPW0603 e3, 0.1 %, 25 ppm/K** | FB dividers (added after A1) | doc 28758, 10-Apr-2026, `6f0508c5…` | not read (no lifecycle field on the page): Scotty | `Device:R` → `R_0603_1608Metric` | ±0.1 %; ±25 ppm/K (TCR to ±10 ppm/K offered); ≤ 0.10 % after 8000 h at P70 (general mode, p2) |
| **AMASS XT30PW-M** | power-in | AMASS spec V1.2 (distributor copy), `c57b5a37…` | **unread**: Scotty | `Connector_Generic:Conn_01x02` + `Connector_AMASS:AMASS_XT30PW-M_1x02_P2.50mm_Horizontal` (TH) | rated 15 A, momentary 30 A, 0.80 mΩ, DC 500 V, −20…120 °C |

**For Scotty** (each one was not fetchable or not readable from Furnace):
- Diodes' product status for AP63205 and AP2112: www.diodes.com/part/view/AP63205 and /AP2112, which answer with a browser challenge.
- The AMASS XT30PW-M spec from AMASS itself: www.china-amass.net, whose `/uploads/XT30PW-M-SPEC.pdf` returns 404. tme.eu returns 403 for each of `Document/6eb2005a…/XT30PW-M.pdf`, `Document/e5863b89…/XT30PB-M.pdf` and `Document/b1362971…/XT60PW-M.pdf`.
- The held copy came from a distributor. Distributors also disagree on the rating: some list 20 A from an AMASS catalogue table, others 15 A from the spec. So the maker's copy is the citation the build should hold.
- Yageo's RC_L datasheet (404 at the URL tried) and Panasonic ERJ (timed out). Only needed if a second resistor maker is wanted.
- **ST AN2834** (ADC accuracy, the reservoir-capacitor rule D10 (a) needs): st.com is unreachable from Furnace.
- **Production status not read:** Vishay TNPW and CRCW (their product pages carry no lifecycle field), and AMASS XT30PW-M (no maker page). TI's six are read (`spike/4300d/status.log`, each page's sha256).

### The recommended parts

- **Buck:** TI **LMR33630ADDA** (400 kHz), twice:
  - battery → `VDD_5V`, with FB divider 102 k/25.5 k;
  - battery → `VDD_3V3`, with FB divider 105 k/46.4 k (centred in the window; or 422 k/107 k and 232 k/102 k with TI's feedforward capacitor, for wider margins).

  Each divider is Vishay **TNPW0603 e3, 0.1 %, 25 ppm/K** (doc 28758, `6f0508c5…`). That replaces TI's Table 9-2 values on 1 % parts, which fail the ±5 % domains (P3.3, amended).

  **Why:**
  - It is the only candidate with VIN margin over 6S-HV.
  - It has guaranteed timing and RDS(on) maxima, so the headroom and on-time checks are bounds, not typical curves.
  - One part serves both rails.
- **Second rail:** the 3.3 V buck straight from the battery, not an LDO off 5 V (D1). It could hang off the 5 V buck: 2.68 A against 3 A passes in steady state (A3). The battery is preferred for **fault isolation**: a short on a DS-009 port's 5 V would hold the 5 V buck in current limit and starve the MCU (inferred). It also leaves the 5 V buck's thin 2S headroom out of the MCU's path.
- **1.8 V:** TI **TLV75518PDBV** off `VDD_3V3`, plus TI **TXU0304PW** for the SPI crossing (below).
- **Battery sense:** a 100 k/10 k Vishay CRCW0603 1 % divider, which needs a reservoir capacitor at the ADC pin (D10). **Current sense: none.** An analog INA18x is checkable only by full scale, and the ArduPilot side reads the DS-009 module when one is used.
- **Power-in:** AMASS **XT30PW-M**, rated 15 A, against the bucks' steady-state input of 2.68 A and their fault bound of 10.1 A (D2, D7).

### The 1.8 V question (`4210` D7, ruled to this work order)

**Recommended: a 1.8 V LDO plus a fixed-direction translator, both declared parts. Not a refusal.**

- **The LDO:** TLV75518PDBV holds 1.773–1.827 V over its full junction range. That sits inside the ICM-20948's VDDIO of 1.71–1.95 V, with 63 mV and 123 mV to spare. It is also inside the ICP-10111's 1.71–1.89 V.
- **The translator: TXU0304PW,** with the MCU on port A (VCCA = `VDD_3V3`) and the IMU on port B (VCCB = `VDD_1V8`):
  - A1, A2, A3 → B1Y, B2Y, B3Y carry SCK, MOSI and CS, which are the MCU's outputs. B4 → A4Y carries MISO.
  - This matches TI's own SPI example (`read-ldo-xlat.md` §2).
  - TMAX is at least 50 Mbps for 3.3 V to 1.8 V (SCES935A p9, rendered), against the ICM-20948's 7 MHz.
  - **CS during MCU reset** (amended, A7). Every TXU0304 input has a 5 MΩ pull-down. So while the MCU's CS pin floats (reset, boot), A3 reads low and B3Y drives CS low, which selects the IMU. The pull-up belongs on A3, on the MCU side (the datasheet allows ≤ 1 MΩ against the 5 MΩ pull-down), or OE is driven by a GPIO held low until the MCU configures its pins. The B outputs also go Hi-Z when OE is low or a supply is below 100 mV, so a B-side pull-up on CS covers that case.
- **Why not the TXB0104:**
  - its outputs are guaranteed only at ±20 µA;
  - it limits pull-ups to > 50 kΩ and the load to 70 pF.

  Its VCCA ≤ VCCB rule only puts the MCU on B, and is not by itself a disqualifier (A12).
- **A timing check the crossing adds** (inferred; the ICM-20948's tDO was not read here). The MISO path delay, tpd(SCK, A→B) + the IMU's output delay + tpd(MISO, B→A), must fit inside the MCU's sample window. That is 19 + tDO + 15 ns against half of 143 ns at 7 MHz. It is a static check on cited fields, and belongs with `4300c`'s speed rules.
- **The ICP-10111 (I2C, 1.8 V only)** would need an open-drain-capable crossing: the TXS0102 or LSF-family parts ship in KiCad's `Logic_LevelTranslator`. Their datasheets were not fetched here, because both 4210 parts are EOL stand-ins pending Scotty (inferred).
- **The other option** is an IMU whose VDDIO takes 3.3 V (`4210` D1, Scotty's purchase). It removes the domain and the crossing entirely, and the machinery above stays for any 1.8 V part.

### S2 — the specification's shape (`spike/4300d/power-shape.json`)

- **`power.input`:** the power-in connector (a row in `4300f`'s connectors table, with its rated current).
- **`power.battery`:**
  - `cells`;
  - `chemistry`;
  - `cell_min_millivolts` and `cell_max_millivolts`, each `{value, source}`, as `4100`'s stackup bounds carry a source. No standard publishes a pack's limits, so the source is the user's pack datasheet.
- **`power.regulators[]`:**
  - `reference`, `part` (a new regulators table, below);
  - `input` (`"battery"` or a domain id), `output` (a domain id);
  - for an adjustable part, `feedback` `{top: {reference, ohms}, bottom: {reference, ohms}, part}`;
  - `external`: the inductor and capacitors, each a declared part with a reference (D6).
- **`power.loads[]`:** `{domain, milliamps, source}` for loads no parts row covers. The MCU's 658 mA comes from its row (Table 120), never typed by hand.
- **`power.sense[]`:** the divider (`top` and `bottom`, each with a reference), its resistor part, a `reservoir` capacitor (D10), the battery as source, the ADC net, its pin (solved), and `adc_resolution_bits` (A4).
- **`power.conditions`:** `board_temperature_celsius` `{min, max, source}`, the temperature every resistor and part check evaluates at (A2).
- **`power.crossings[]`:** `{reference, part, bus, a_domain, b_domain}`. Each channel's direction is the part row's.
- **`voltage_domains` is unchanged** (`INV-01`): `power` names domains and never restates them.
- **How a crossing meets `4210`'s refusal** (its build, mailbox note at `ef8fc97`). `4210` adds `io_millivolts` to `parts.json` and refuses `IoOutsidePart` when a device's bus domain leaves the part's I/O range. A crossing therefore splits one SPI bus into two segments: the MCU side, in the bus's declared domain (`VDD_3V3`), and the device side, in the crossing's other domain (`VDD_1V8`). `IoOutsidePart` reads the device-side domain, and refuses an ICM-20948 on a 3.3 V bus with no crossing, as it should.
- **A regulators table** beside `parts.json`'s sensor rows: a row has no bus, identity or Renode model. Each row holds:
  - its cited limits (S3's fields);
  - its pin roles (input, output or feedback, ground, EP);
  - its KiCad symbol and footprint;
  - its `eol` flag (`4210` D1's new refusal applies);
  - one citation per field group, with sha256.

  A translator is a row there too, with per-channel directions and both VCC ranges.

**The solver's tree:**
- Every domain has exactly one source; following inputs from any domain reaches `battery` with no repeat.
- A regulator's load is the sum of its domain's loads, plus each child's input current:
  - an LDO child: IOUT + IGND max;
  - a buck child: its own load (input ≤ output in steady state), with its bias as a named condition; ISC max is the fault bound (D2, amended after A3).

**Refusals, each by name:**
- `DomainWithoutSource`;
- `DomainWithTwoSources`;
- `PowerLoop`;
- `RegulatorOverloaded` (load against IOUT max, naming the sum's terms);
- `PowerInputOverloaded` (Σ buck ISC max against the connector's rating);
- `CrossingDomainMismatch` (a device's I/O range against its side of the crossing; `4210`'s new refusal on the device side).

**`SolvedTopology` gains the tree:**
- each regulator's reference, input and output nets;
- the **source and return pads** (S5), from the row's pin roles (`RUL-16`: `1303` and the generators never read the specification).

### S3 — the checks, each with its data and refusal

| Check | Reads | Refusal | On the proposed tree |
|---|---|---|---|
| Battery against VIN | cells × cell min/max; the row's VIN min-operating (max) and VIN recommended max | `BatteryOutsideRegulatorInput` | pass, all of 2S–6S-HV. Named condition: hot-plugging a pack through long leads rings the input above its DC value; TI (p23, p32) asks for 20–100 µF bulk capacitance in parallel then, and the 38 V absolute maximum is the ceiling |
| LDO dropout | the input domain's guaranteed minimum (its own regulator's), VOUT max, VDO max at the row's current and Tj | `DropoutViolated` | pass: 3.146 ≥ 2.227 V |
| Buck headroom | D ≤ 1 − tOFF-MIN(max) × fSW(max), RDS(on) max, the inductor's DCR max, the load | `BuckHeadroomViolated` | pass at 2 A: 5.389 ≥ 5.245 V (conditions: DCR, the VIN = 12 V test point for tOFF, RDS(on) and fSW, VCC at VIN ≥ 6 V, the input-path drop) |
| Minimum on-time | VOUT min / VIN max / fSW max against tON-MIN max | `OnTimeTooShort` | pass: 401 and 266 ns ≥ 108 ns |
| VOUT in the domain | VFB min/max, IFB × RFBT, the divider at tolerance + TCR + drift at a declared resistor temperature, times the regulation band (design limit, named) against nominal ± tolerance; or fixed VOUT min/max | `RegulatorOutsideDomain` | **1 % fails both buck rails; 0.1 % centred passes both** (5 V by 5 mV, 3.3 V by 11 mV at RFBT ≈ 100 k) (D13) |
| MCU supply | VOUT max ≤ 3.6 V (Table 111); at each supply pin, VOUT min − R(VDD+VSS) × 658 mA ≥ the floor (D3) | `SupplyBelowFloor` (with `1303`) | 17 mΩ (domain floor), 222 mΩ (3.0 V consumer floor) or 708 mΩ (BOR3) |
| BOR margin | the declared BOR level's falling maximum (Table 115) | part of the above | VBOR3 2.68 V |
| Load against IOUT | S2's sum | `RegulatorOverloaded` | 0.678/3, 2/3 and 0.02/0.5 A |
| Power-in | steady state: Σ the bucks' output loads (input ≤ output); fault bound: Σ ISC max; each against the connector's rated current | `PowerInputOverloaded` | 2.68 and 10.1 ≤ 15 A |
| Sense full scale | battery max × the ratio at worst tolerance + TCR + drift, against VREF+ min (VDDA's domain minimum) | `SenseFullScaleExceeded` | pass: 2.505 ≤ 3.135 V |
| Sense source impedance | the divider's Thevenin R against RAIN max at the declared ADC resolution (Table 173), or a declared reservoir C | `SenseSourceImpedance` | **fail: 9.09 kΩ > 1150 Ω at 12 bits** (D10) |
| External parts | Cin ≥ 10 µF + 220 nF, CBOOT, CVCC, RFBT ≤ 1 MΩ, Cout ≤ 10× design or 1000 µF, L by Eq. 4, Isat ≥ ISC max | `ExternalPartMissing` / `ExternalPartOutOfRange` | rated values only (D6) |
| Thermal | not in the work order's list | — | D1 |

**Typical-only fields are never a bound.** Where the datasheet offers nothing else, the check is a named condition in the verdict:
- PSRR;
- efficiency;
- load regulation;
- dropout curves;
- the TLV755's accuracy beyond 1 mA;
- ISC's open-loop test.

### S4 — the board

**Symbols and footprints:**
- Every recommended part ships in the image: `kicadlib.out` lists each part's pins and the footprint's sha256.
- **None is vendored yet.** The build vendors these into `aethelgard-kicad-lib`, with KiCad's licence, as the 17 were (`RUL-11`):
  - `LMR33630ADDA`;
  - `TLV75518PDBV`;
  - `TXU0304PW`;
  - the plain HSOP-8, `SOT-23-5`, `TSSOP-14`;
  - `R_0603`, an inductor footprint and an XT30.

**The emitter:**
- The LDO's OUT is `power_out`, and the emitter puts a `PWR_FLAG` (also `power_out`) on every rail (`schematic.rs:6`). KiCad 10.0.7's default pin map gives `PwrO × PwrO = ERR` (`eeschema/erc/erc_settings.cpp` at tag `10.0.7`, `99c3b067…`, held in `~/.cache/aet-4300f/kicad-src/`). **So a rail sourced by an LDO must not get a flag.**
- The buck's rail is driven only through the inductor (`passive`), and SW is `output`, so the buck's rails still need one.
- **The LMR33630's own pins:**
  - Pin 6 `VCC` is `power_out` and takes its own net and 1 µF, as the MCU's VCAPs do (`schematic.rs:7`).
  - Pin 9 (EP) `GND` is `passive` and joins `GND`.

**The placer** (amended, A6) places each part beside the IC it shares most nets with (`pcb.rs` L413–426). A buck's divider, inductor and Cout share the output rail with the MCU, so they land by the MCU. The datasheet's loop rules need each placed by its regulator pin: Cin at VIN/PGND, CBOOT at BOOT/SW, CVCC at VCC, the FB divider at FB, and the inductor at SW. So a part's anchor must be declarable ("this passive belongs to U10").

**The router:**
- **It refuses every through-hole pad** (`route.rs:1876`). That means the LMR33630's default ThermalVias footprint and every AMASS power-in footprint. The plain HSOP-8's pads pass that test (read; not routed here), but it carries none of the datasheet's recommended **4 × 3 thermal-via array** (§10.1.1).
- **No per-net widths:** the power-in and battery nets carry amperes. These are `1308`'s per-net widths and via option (ruled at `1303` D8). Through-hole pads are not in `1308`'s list as ruled (D7, D8).

**Which layout rules a gate can check, and which stay named conditions:**

- **Gateable from the placed or routed board:**
  - Cin's distance to VIN/PGND, and CBOOT's and CVCC's, each against a declared maximum (centre-to-pad, from placement);
  - SW-node copper area (the routed net's copper);
  - the EP joined to `GND` with at least N vias, once vias on a pad exist;
  - the FB net not routed parallel to SW within a declared distance;
  - 4 layers when the row says so.
- **Named conditions:**
  - "as close as possible" itself, unless the specification declares a number;
  - copper weights of 2/1/1/2 oz (a stackup field from `1303` D3/D6, if ruled);
  - solder integrity under the EP;
  - the Schottky for an input that can fall below the output (p32).

### S5 — the output pad

- **1303's bound needs two terminals per domain:** a source pad and a return pad, each `(reference, pad number)` from `SolvedTopology`. Today its probe takes every non-MCU pad as a stand-in (`run_paths2.py` L20–24).
- **The LDO:** the source is the row's output pin (TLV75518: pad 5, OUT), and the return is its ground pin (pad 2).
- **The LMR33630 has no output pin.** It regulates the voltage at RFBT's top, through FB (pad 5), and measures everything against the EP (AGND, pad 9).
- **Recommended:**
  - the buck's source pad is **RFBT's top pad**, the point the loop holds at VOUT, so the regulator's VOUT range from S3 is a statement about that pad;
  - the return pad is **RFBB's ground pad** (amended, A8). The loop regulates RFBT's top against RFBB's ground end, and the load current returns to PGND. The EP (AGND) serves as the return only under the named condition that RFBB is grounded at the EP, which the datasheet's layout rule 4 implies.
  - Copper between the inductor and RFBT's top is inside the loop, and is not the MCU's drop.
- **What it needs:**
  - the regulators table naming pin roles;
  - the emitter giving the divider's resistors references tied to their regulator;
  - `SolvedTopology` carrying `{domain: (source ref, pad), (return ref, pad)}`.

  Then `1303` replaces its stand-ins with one source, and D7's VDD+VSS sum is the two R bounds from those two pads.

## Decisions needed

- **D1 · The tree, and thermal.** Recommended: two LMR33630s from the battery (5 V and 3.3 V), and the TLV75518 off 3.3 V. The alternatives:
  - 3.3 V by LDO off 5 V, about 1.2 W in SOT-223. That needs a thermal check S3 does not have.
  - A 3.3 V buck off the 5 V buck. It passes in steady state (2.68 A ≤ 3 A, amended after A3), but loses fault isolation.

  Should a junction-temperature check (P × RθJA against Tj max) join S3 for LDOs? RθJA is a JEDEC-board figure, which TI says "cannot be used for design" for the buck, so for the buck it would be a named condition.
- **D2 · A buck behind a regulator, and power-in** (amended after A3). In steady state a buck's average input is ≤ its output current, whatever its efficiency. Its own bias (gate drive, VCC) is not guaranteed and is named. ISC max (tested open loop, TI note 4) bounds the fault and start-up case. **Recommended:**
  - load the upstream regulator with the child's output current plus a named bias;
  - judge the power-in connector twice: steady state (Σ outputs, 2.68 A) and fault (Σ ISC max, 10.1 A), each against its rating.
- **D3 · Where the copper drop is judged** (amended after A9). The 3.3 V regulator (0.1 % divider, centred) leaves 11 mV above 3.135 V. Judged at the domain floor, that allows 17 mΩ for VDD+VSS at 658 mA, below `1303`'s 50–300 mΩ measured on VDD alone at 0.2 mm. **The floor is the highest any consumer of the domain declares,** not the MCU's BOR alone:
  - VDD33USB 3.0 V when USB is used (Table 111);
  - the TXU0304's 3.0–3.6 V timing rows;
  - VREF+ for the sense's full scale;
  - the BOR level's falling maximum (VBOR3 2.68 V).

  At 3.0 V the room is 146 mV (222 mΩ); at VBOR3, 466 mV (708 mΩ). The R × 658 mA bound counts the MCU's current only; other loads' return currents in a shared ground (the 5 V rail's 2 A) add to the VSS drop, and the bound must count them or name them. **Recommended:** the domain's tolerance holds at the source pad; each consumer's pins are judged against that consumer's own floor; the VSS term uses the total current through the shared ground path.
- **D4 · The BOR level in the specification.** Today the scenario declares it (`power::schedule`). One source (`INV-01`) suggests `mcu.bor_level` in the specification, which the scenario and the D3 check both read. The option-byte write that sets it in firmware is out of this work order.
- **D5 · The buck's headroom at 2S.** Accept the computed bound from guaranteed fields, with "tOFF limit stated at VIN 12 V" and the inductor's DCR as conditions? Or require the battery minimum to be ≥ 7 V, TI's own (untested) system-table floor for 5 V?
- **D6 · External parts.** Capacitors' effective capacitance under DC bias has no guaranteed source; vendors publish typical curves. **Recommended:**
  - the check reads **rated** values against the datasheet's rated table (Table 9-2), with effective C named as a condition;
  - the inductor is a cited part row: L, Isat minimum, DCR maximum.

  p22's Eq. 5 contradiction and the 6–26 V ripple trade-off go to the build as a stated choice (TI's 8 µH at 400 kHz).
- **D7 · Power-in.** AMASS XT30PW-M, 15 A (spec V1.2, a distributor copy; Scotty for the maker's copy). It is through-hole, and the router refuses through-hole pads (`route.rs:1876`). Either `1308` gains through-hole pads, or the build uses an SMD power-in (none is in KiCad's AMASS library), or wire pads with a declared rating. **Recommended:** `1308` gains through-hole pads. Also: the XT30 is rated −20…120 °C, so the board's declared temperature minimum must be −20 °C or above with it, or the part refused (A13). KiCad ships no XT30 symbol; the build uses `Connector_Generic:Conn_01x02` (passive pins; in the image) with the AMASS footprint, whose two extra unnamed mechanical pads are through-hole as well.
- **D8 · Thermal vias under the buck's EP.** TI recommends a 4 × 3 array (minimum) of 10-mil vias (p34). KiCad's ThermalVias footprint has 8, at 0.3 mm drill, so it does not meet that rule either (amended, A5), and the router refuses it. **Recommended:** `1308`'s via option includes a declared via array on a pad, emitted by the engine from the regulator row's rule, not taken from KiCad's footprint. Until then, the plain footprint with the rule named as an unverified condition.
- **D9 · The 1.8 V answer:** TLV75518PDBV plus TXU0304PW (MCU on A), with `crossings` in the specification and the CS pull-up on B. The alternative is a 3.3 V-VDDIO IMU (`4210` D1, Scotty). Also: does the MISO round-trip check join `4300c`'s speed rules?
- **D10 · Battery sense against RAIN.** A 9.09 kΩ divider exceeds RAIN's 1150 Ω at 12 bits. Options:
  - (a) a reservoir capacitor at the pin, its rule cited (ST AN2834, not held here);
  - (b) a low-impedance divider (about 10 k/1.1 k, 2.3 mA at 25 V).

  The first version offered (c), leaving it to the firmware's sampling time. Table 173 gives RAIN with no sampling-time condition, and Table 174 lists 12-bit sources only to 680 Ω, so nothing supports (c) (A4). The ADC resolution must be a declared field: RAIN is 170 Ω at 16 bits and 26.5 kΩ at 8. ArduPilot's H7 resolution was not read here. **Recommended:** (a), with the capacitor a declared part, the rule cited once the application note is held, and the resolution declared.
- **D11 · A regulators table** beside the sensor rows (not a widened sensor row), with pin roles, per-field citations and `eol`. The same table holds translators.
- **D12 · The schematic emitter** omits `PWR_FLAG` on a rail whose source pin is `power_out` (an LDO), and keeps it on a buck's rail. That is a read at KiCad's source (S4), not yet measured with ERC; the build measures it.

## Findings

| id | finding | where | kind | decision |
|---|---|---|---|---|
| F1 | DS12117 Rev 10 has two electrical sections, §6 (rev Y) and §7 (rev V). `power.rs` cites Table 16 (§6). Table 115 (§7, p212), the rev V table beside Table 120's 658 mA, carries the same BOR values. No value changes; the citation could name both | `aethelgard-chaos/src/power.rs` L22–28 | read | none (backlog) |
| F2 | The buck reader read the UVLO as "about 3.7 V, typical, text only" and missed p7's VIN minimum-operating **maximum 3.8 V**, a guaranteed field. Caught at spot-check | `read-bucks.md` §1.5 | process | none |
| F3 | My P1.6 was half wrong: a buck's input is bounded by its high-side current limit. The prediction stands as written beside the result | P1.6 | lesson | D2 |
| F4 | The pinned image ships no licence text beside KiCad's libraries (`1300` D10, still open) | `kicadlib.sh` | measured | none here |
| F5 | **Mischaracterised as first written** (A11). p22's Eq. 5 is a separate minimum-inductance rule (L_MIN ≥ 0.28 × VOUT/fSW). The sentence above it is a ripple floor bounding L from above; the two do not contradict. The real inconsistency is p23: "a minimum of 10 µF is required" beside the example's 4.7 µF | `read-bucks.md` §1.13 | read | D6 |
| F6 | The LMR51430 datasheet labels its package DBV in places; every orderable is DDC | `read-bucks.md` §3 | read | none |

- **D13 · A buck's accuracy against a ±5 % domain** (found by the adversarial pass, A1). The LMR33630 alone takes about 4 % of the window: VFB ±1.5 % plus a regulation band of −1.5/+2.5 % that TI states by design and does not production-test. With the resistors at 125 °C, 1 % dividers fail both rails, and 0.1 % dividers fit only when centred against the asymmetric band (by 5–48 mV), leaving 11 mV of copper room at 3.3 V. The options:
  - (a) the domain's tolerance is declared from its consumers' ranges, for example 3.3 V ±90 ‰ (3.003–3.597 V), inside the MCU's 1.62–3.6 V, VDD33USB's 3.0–3.6 V and the TXU0304's 3.0–3.6 V rows. A new check then confirms that the domain's range lies inside every consumer's supply range (`parts.json`'s `supply_millivolts`, which has had no consumer since `4200` F19);
  - (b) a regulator with tighter guaranteed accuracy, which no candidate here has;
  - (c) count the design-only regulation band as a named condition rather than a bound.

  **Recommended: (a) with the consumer check, and the band counted (not waived).** At ±90 ‰ (3.003–3.597 V) the 3.3 V rail passes with 0.1 % resistors with wide margins (3.146–3.421 V), and with 1 % centred (105 k/46.4 k: 3.038–3.546 V); TI's 100 k/43.2 k on 1 % fails (3.603 V). The copper room then becomes the consumer floors of D3.

## Adversarial pass

A fresh Opus subagent was handed the artifacts and sources, not my conclusions. Its full record is `~/.cache/aet-4300d/adversarial.md`, with renders `render/adv-*.png`. It confirmed every table value in `checks.py` and every code claim:
- `route.rs:1876`;
- `erc_settings.cpp` L54, `PwrO × PwrO = ERR`;
- the vendored `PWR_FLAG` is `power_out`;
- the plain HSOP-8 is all-SMD;
- `run_paths2.py` L20–24.

Its findings, with their dispositions:

| id | severity | finding | disposition |
|---|---|---|---|
| A1 | high | "VOUT in domain: PASS" left out the LMR33630's regulation band (−1.5/+2.5 %, p9, design limit) and IFB × RFBT | **fixed**: `checks.py` v2 counts both; the 1 % dividers fail; P3.3 and S3 amended; D13 |
| A2 | high | the resistor temperature span (65 K mislabelled "25 to 85 °C") and the resistors' drift were not stated | **fixed**: 100 K (to 125 °C) as a declared condition, drift counted (CRCW 1 % at 1000 h; TNPW 0.10 % at 8000 h, doc 28758 p2, rendered); `conditions.board_temperature_celsius` in S2 |
| A3 | high | a buck's steady-state input current is ≤ its output current whatever the efficiency; ISC is the fault bound. P1.6's "refused chain" was an artifact | **fixed**: P1.6 marked wrong; D1, D2, S2's tree and the power-in check amended; the battery-direct tree kept for fault isolation |
| A4 | medium | Table 173's RAIN has no sampling-time condition (Table 174 lists 12-bit only to 680 Ω), so D10(c) is unsupported; the resolution is an assumption | **fixed**: (c) withdrawn; RAIN checked at every resolution; resolution a declared field |
| A5 | medium | the ThermalVias footprint (8 vias, 0.3 mm) does not meet TI's recommended minimum 4 × 3 either | **fixed**: D8 amended |
| A6 | medium | the placer places in rings beside the anchor IC, not in courtyard rows; with no VOUT pin, the buck's passives tie to the MCU | **fixed**: P4.3 and S4 amended; an anchor declaration is named |
| A7 | medium | the TXU0304's 5 MΩ input pull-downs drive CS low while the MCU's pin floats | **fixed**: the pull-up goes on A3 (or OE by GPIO); the 1.8 V section amended |
| A8 | medium | S5's return pad: the loop regulates against RFBB's ground end; the EP is a condition | **fixed**: RFBB's ground pad, with the EP under a named condition |
| A9 | medium | D3's BOR floor conflicts with VDD33USB 3.0 V, the TXU0304's 3.0 V rows and VREF+; the VSS bound ignores other loads' ground currents | **fixed**: D3 amended to "each consumer's floor" and the total ground current |
| A10 | medium | the headroom's conditions also cover RDS(on) and fSW (VIN 12 V), VCC at VIN ≥ 6 V exactly at 2S minimum, and the input-path drop; the break-even DCR should be stated | **fixed**: named in `checks.out`; break-even DCR 102 mΩ at 2 A |
| A11 | medium | the external-part list omits Cin's voltage rating, the 220 nF 50 V X7R, Eq. 5's L_MIN, and capacitor footprints above 0402; F5 mischaracterised | **fixed**: F5 amended; the S3 external-parts row below |
| A12 | low | VCCA ≤ VCCB is not a disqualifier; "tri-stated MISO" was opinion | **fixed**: the rejection rests on drive strength, pull-ups and load |
| A13 | low | TLV75518 ±1.5 % is tested at VIN 2.3 V and 1 mA; IGND at IOUT = 0; TMAX is Mbps against SCK in MHz (7 MHz = 14 Mbps, still a pass); no input-threshold check; XT30 rated −20…120 °C against a −40 °C board; 658 mA is a VOS0 figure at Tj 125 °C, which Table 111 note 5 caps at 105 °C (`1303` D4's ruling, conservative); CRCW's TCR depends on the ordering code | **recorded**: conditions in `checks.out`; the XT30's −20 °C floor joins D7; the part row pins the ordering code |
| A14 | low | the 3.3 V buck's headroom at 2S was not computed | **fixed**: 5.638 V ≥ 3.467 V |
| A15 | medium | S2's shape lacked a temperature range, the ADC resolution, the external parts and per-resistor references | **fixed**: `power-shape.json` amended |

**The external parts the check reads** (A11, LMR33630 p21–23):
- Cin ≥ 10 µF ceramic, rated at least VIN max (preferably 2×), X5R or better;
- the mandatory small 220 nF at VIN/PGND;
- CBOOT 100 nF, ≥ 10 V;
- CVCC 1 µF, 16 V;
- RFBT ≤ 1 MΩ, with a feedforward capacitor at 1 MΩ;
- Cout ≥ the transient design value (Eq. 6) and ≤ 10× it or 1000 µF, rated ≥ 10 V at 3.3 V and ≥ 16 V at 5 V;
- L by Eq. 4 at K = 20–40 % and ≥ Eq. 5's L_MIN, with Isat ≥ ISC max ideally and never below the low-side limit.

Vendoring needs capacitor footprints above 0402 (0805, 1210).

## Gates

- On Furnace, under a capped user scope (`memory.swap.max` 0), at `1ed4f9c` (every later commit touches this file only):
  - `cargo fmt --all -- --check`: exit 0;
  - `cargo clippy --workspace --all-targets -- -D warnings`: exit 0;
  - `cargo test --workspace`: exit 0 (79 result lines, all ok).
- The merge of `origin/main` at `08d6206` (#136) changed only `docs/`. The first run, at `9648f38`, also passed; the run above is at the head after the adversarial amendments.
- The spike adds no Rust and touches nothing outside `spike/4300d/` and this file. Tier 2 is CI's.
- **Reduction pass (diff): nothing to remove.** No fact is held twice as a source. The two reader records are evidence (the cache copies are scratch). Every `checks.py` source key is read, and the two shell scripts do different jobs.

## Stop

**The spike is done; S1–S5 were measured or read.**
- **Predictions:** committed first (`7508585`).
- **Datasheets:** 11 held by sha256 (`spike/4300d/SHA256SUMS`). Scotty's list is under S1.
- **The adversarial pass:** fifteen findings, three high, all applied.
- **For the architect:** the table (S1), the recommended parts, the specification's shape (S2), the checks (S3), the board (S4), the output pad (S5), and D1–D13.
- **The build waits** for the ruling, and for `4300f` and `1308`.
