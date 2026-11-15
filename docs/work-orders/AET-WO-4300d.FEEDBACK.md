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
| P1.5 | 3.3 V from an LDO off 5 V at 0.68 A is about 1.2 W, more than a SOT-223 can shed. **3.3 V from a second LMR33630 straight from the battery** passes every check (`checks.out`). The LDO's thermal arithmetic used no datasheet; it is a decision (D1). | inferred / measured arithmetic | right |
| P1.6 | **Half wrong.** No efficiency is guaranteed (all four bucks give typical curves only). **But** a buck's average input current is bounded above by its high-side switch current: ISC max 5.05 A (p7), a guaranteed field. So a buck behind a regulator is not unbounded, only loosely bounded: 5.05 A against a 3 A upstream rating refuses a buck fed from the 5 V buck. **TI's note 4:** ISC is "tested, open loop, in production … may differ … in a closed loop application", so the bound carries that condition. | read | partly wrong (D2) |
| P1.7 | **TLV75518PDBV (SBVS320D, ACTIVE) fits:** 1.773–1.827 V over Tj −40…125 °C (±1.5 %, at IOUT 1 mA), inside VDDIO 1.71–1.95 V. Dropout is 400 mV at 500 mA and 125 °C. **AP2112K-1.8:** guaranteed ±1.5 % only at 25 °C, with a typical-only tempco; rejected. **TXU0304PW (SCES935A, ACTIVE),** not the TXB0104, is the translator. See the 1.8 V section. | read | partly: the translator differs |
| P1.8 | **Vishay CRCW0603 e3 (doc 20035, 14-Apr-2026):** ±1 % (and ±5 %) only, ±100 ppm/K, 75 V maximum working voltage. Yageo and Panasonic were not fetchable. `Device:R` and `R_0402` are vendored today; `R_0603` is not. **The full-scale check passes, but the sense fails on source impedance:** a 100 k/10 k divider is 9.09 kΩ, against RAIN ≤ 1150 Ω at 12 bits (DS12117 Table 173, p273, rev V). Not predicted. No current-sense part is recommended. | read / measured | partly; RAIN not predicted |
| P1.9 | **The image carries no `LICENSE.md` beside `symbols/` or `footprints/`** (`podman cp` found none). That is `1300` D10's finding, still true. The libraries are CC-BY-SA 4.0 with the design exception, read from `aethelgard-kicad-lib`'s vendored `LICENSE.md` (`RUL-11`). | measured / read | right |
| P2.1–P2.3 | The shape below, as predicted, plus the power-in connector, `crossings` and feedback dividers. | proposal | — |
| P3.1 | **Battery min ≥ VIN min-operating max 3.8 V** (p7: a guaranteed maximum, so no UVLO typical is needed), and **battery max ≤ VIN recommended max 36 V.** Passes for 2S, 6S and 6S-HV. | measured arithmetic | right, with a better field |
| P3.2 | **LDO:** VIN min (3.191 V, the 3.3 V buck's guaranteed minimum) ≥ 1.827 + 0.400 V. Passes. **Buck at 2S:** a bound from guaranteed fields alone, D ≤ 1 − tOFF-MIN(max) × fSW(max) = 0.961 (no credit for foldback). That leaves 5.389 V available at 2 A against VOUT max 5.228 V. **Conditions:** the inductor's DCR (30 mΩ assumed, from an inductor row not yet held), and the datasheet's "VIN = 12 V unless stated" condition on tOFF. TI's own system table states 5 V regulation only from 7 V, and is not production tested. | measured arithmetic / read | right |
| P3.3 | **VOUT including FB divider tolerance and TCR:**<ul><li>5 V: 4.812–5.228 V;</li><li>3.3 V: 3.191–3.443 V;</li><li>1.8 V: 1.773–1.827 V.</li></ul>All lie inside their ±5 % domains. The 3.3 V regulator leaves **56 mV** above 3.135 V for VDD+VSS copper drop at 658 mA, which is **85 mΩ**. Measured against VBOR3's falling maximum instead, the room is 511 mV (777 mΩ). D3. | measured arithmetic | right |
| P3.4 | **BOR:** rev V Table 115 (p212) equals rev Y Table 16, which `power.rs` cites, value for value (F1). The BOR level is still only a scenario field (D4). | read | right |
| P3.5 | Full scale at 6S-HV, worst ratio (1 % + 100 ppm/K × 65 K): 2.445 V ≤ VREF+ = VDDA min 3.135 V. Passes. **Source impedance fails** (P1.8). | measured arithmetic | right, plus RAIN |
| P3.6 | **The LMR33630's external parts:**<ul><li>Cin ≥ 10 µF ceramic plus a mandatory 220 nF;</li><li>CBOOT 100 nF;</li><li>CVCC 1 µF;</li><li>RFBT 100 kΩ, maximum 1 MΩ;</li><li>Cout at most 10 × the design value or 1000 µF;</li><li>L from Eq. 4 at K = 20–40 %, with Isat ≥ ISC max ideally and never below the low-side limit (p21–23).</li></ul>**Gaps:**<ul><li>Table 9-2 gives **rated** C. The datasheet defines effective C (bias and temperature) as what matters, and a vendor's DC-bias curve is typical, so effective C has no guaranteed source.</li><li>p22 contradicts itself on the inductor: Eq. 5 is labelled L_MIN, while the sentence above it describes a maximum from a 10 % ripple floor.</li><li>An L sized for 30 % ripple at 26.1 V gives 6 % ripple at 6.0 V, below that floor. The datasheet does not resolve this trade-off for a 2S–6S range.</li></ul>D6. | read | right, with the Eq. 5 finding |
| P3.7 | tON at 26.1 V and fSW max: 401 ns (5 V) and 266 ns (3.3 V), both ≥ 108 ns. The B (1.4 MHz) and C (2.1 MHz) variants fail at 3.3 V. | measured arithmetic | right |
| P3.8 | Raised as D1; the LDO's own rise is 7.8 K (inferred). | — | — |
| P4.1 | **The router refuses** every pad that is not a front SMD pad (`crates/aethelgard-eda/src/route.rs:1876`). That covers the ThermalVias variant (8 `thru_hole` pad-9 copies plus a `B.Cu` pad 9) **and both XT30 and XT60** power-in footprints (all `thru_hole`). The plain HSOP-8 passes that test. | read at source / measured | right, wider than predicted |
| P4.2 | **LMR33630 §10.1 (p33–34) states:**<ul><li>Cin closest to VIN/PGND (pins 2/1, adjacent);</li><li>CVCC close;</li><li>CBOOT short and wide;</li><li>the FB divider close to FB, its VOUT leg away from SW;</li><li>a ground plane on a middle layer;</li><li>the EP soldered to ground;</li><li>wide VIN/VOUT/GND;</li><li>a minimum SW node;</li><li>a **4 × 3 array of 10-mil thermal vias** under the PAD;</li><li>a 4-layer board, 2/1/1/2 oz.</li></ul> | read | right |
| P4.3 | `pcb.rs` places by courtyard in rows (`1301`); "near" exists nowhere. | read | right |
| P5.1 | 1303's probe addresses a pad as (reference, pad number) and takes every non-U1 pad on `VDD_3V3` as a stand-in (`spike/1303/s4/run_paths2.py` L20–24). | read | right |
| P5.2 | **The LMR33630 has no VOUT pin.** Pin 6 `VCC` is the internal 5 V bias (power_out in KiCad). The output is sensed only at FB (pin 5) through RFBT. The EP is AGND, "all electrical parameters measured with respect to this pin" (p5). The TLV75518's OUT is pin 5 and GND pin 2. | read | right; see S5 |

### S1 — the table

| Part | Role | Datasheet (held) | Production | KiCad 10.0.7 symbol → footprint | Fields a static check reads |
|---|---|---|---|---|---|
| **TI LMR33630ADDA** (A, 400 kHz) | buck, battery → 5 V and battery → 3.3 V | SNVSAN3F, Nov 2020, `3b0920a4…c04a` | ACTIVE (ti.com) | `Regulator_Switching:LMR33630ADDA` → `Package_SO:Texas_HSOP-8-1EP…_ThermalVias` (TH vias; the plain variant routes) | VIN 3.8 (max of min-operating)–36 V; VFB 0.985–1.015; fSW 340–460 kHz; tON-MIN ≤ 108 ns; tOFF-MIN ≤ 85 ns; RDS(on) ≤ 160/110 mΩ; ISC 3.85–5.05 A; IOUT 0–3 A; IQ ≤ 34 µA; EN 1.2–1.26 V |
| TI TPS62933 | buck (alternative) | SLUSEA4D, Aug 2022, `16ec2eac…` | ACTIVE | `TPS62933` → `SOT-583-8` | VIN 3.8–30 V; timing and RDS(on) typical only |
| TI LMR51430 | buck (alternative) | SLUSEF4A, Nov 2022, `f7d80538…` | ACTIVE | `LMR51430` → `SOT-23-6` | UVLO rising max 4.5 V; timing typical only |
| Diodes AP63205WU | buck (alternative, 2 A) | `ef99daa3…` | **unreadable** (Cloudflare): Scotty | `AP63205WU` → `TSOT-23-6` | no tOFF, DMAX or dropout |
| **TI TLV75518PDBV** | LDO, 3.3 V → 1.8 V | SBVS320D, Sep 2024, `d617843b…2e97` | ACTIVE | `Regulator_Linear:TLV75518PDBV` → `SOT-23-5` | VIN 1.45–5.5; VOUT ±1.5 % (Tj ≤ 125 °C, 1 mA); VDO ≤ 400 mV at 500 mA; IOUT 500 mA; IGND ≤ 40 µA; Cout 1–200 µF; PSRR typical only; RθJA 231.1 °C/W |
| Diodes AP2112K-1.8 | LDO (rejected) | DS39724 Rev 2-2, `ef8d376f…` | **unreadable**: Scotty | `AP2112K-1.8` → `SOT-23-5` | ±1.5 % at 25 °C only |
| **TI TXU0304PW** | translator, SPI 3.3 V ↔ 1.8 V | SCES935A, Nov 2021, `425338d9…550f` | ACTIVE | `Logic_LevelTranslator:TXU0304PW` → `TSSOP-14` | VCCA, VCCB 1.08–5.5 V (no order); A1–A3 → B, B4 → A; TMAX ≥ 50 Mbps down, ≥ 100 Mbps up; tpd ≤ 19/15 ns; skew ≤ 8 ns; Schmitt inputs |
| TI TXB0104PW | translator (rejected) | SCES650K, `9a852f7b…` | ACTIVE | `TXB0104PW` → `TSSOP-14` | VCCA ≤ VCCB; ±20 µA drive; load ≤ 70 pF; pull-ups > 50 kΩ |
| **Vishay CRCW0603 e3, 1 %** | FB dividers, battery sense | doc 20035, 14-Apr-2026, `113d2d57…` | not read (product page not fetched) | `Device:R` (vendored) → `Resistor_SMD:R_0603_1608Metric` (not vendored; `R_0402` is) | ±1 %; ±100 ppm/K; 75 V; 0.1 W |
| **AMASS XT30PW-M** | power-in | AMASS spec V1.2 (distributor copy), `c57b5a37…` | **unread**: Scotty | `Connector_AMASS:AMASS_XT30PW-M_1x02_P2.50mm_Horizontal` (TH) | rated 15 A, momentary 30 A, 0.80 mΩ, DC 500 V, −20…120 °C |

**For Scotty** (each one was not fetchable or not readable from Furnace):
- Diodes' product status for AP63205 and AP2112: www.diodes.com/part/view/AP63205 and /AP2112, which answer with a browser challenge.
- The AMASS XT30PW-M spec from AMASS itself: www.china-amass.net, whose `/uploads/XT30PW-M-SPEC.pdf` returns 404. tme.eu returns 403 for each of `Document/6eb2005a…/XT30PW-M.pdf`, `Document/e5863b89…/XT30PB-M.pdf` and `Document/b1362971…/XT60PW-M.pdf`.
- The held copy came from a distributor. Distributors also disagree on the rating: some list 20 A from an AMASS catalogue table, others 15 A from the spec. So the maker's copy is the citation the build should hold.
- Yageo's RC_L datasheet (404 at the URL tried) and Panasonic ERJ (timed out). Only needed if a second resistor maker is wanted.

### The recommended parts

- **Buck:** TI **LMR33630ADDA** (400 kHz), twice:
  - battery → `VDD_5V`, with FB divider 100 k/24.9 k;
  - battery → `VDD_3V3`, with FB divider 100 k/43.2 k (TI's Table 9-2 values).

  **Why:**
  - It is the only candidate with VIN margin over 6S-HV.
  - It has guaranteed timing and RDS(on) maxima, so the headroom and on-time checks are bounds, not typical curves.
  - One part serves both rails.
- **Second rail:** the 3.3 V buck straight from the battery, not an LDO off 5 V (D1).
- **1.8 V:** TI **TLV75518PDBV** off `VDD_3V3`, plus TI **TXU0304PW** for the SPI crossing (below).
- **Battery sense:** a 100 k/10 k Vishay CRCW0603 1 % divider, which needs a reservoir capacitor at the ADC pin (D10). **Current sense: none.** An analog INA18x is checkable only by full scale, and the ArduPilot side reads the DS-009 module when one is used.
- **Power-in:** AMASS **XT30PW-M**, rated 15 A, against the two bucks' guaranteed input bound of 10.1 A (D7).

### The 1.8 V question (`4210` D7, ruled to this work order)

**Recommended: a 1.8 V LDO plus a fixed-direction translator, both declared parts. Not a refusal.**

- **The LDO:** TLV75518PDBV holds 1.773–1.827 V over its full junction range. That sits inside the ICM-20948's VDDIO of 1.71–1.95 V, with 63 mV and 123 mV to spare. It is also inside the ICP-10111's 1.71–1.89 V.
- **The translator: TXU0304PW,** with the MCU on port A (VCCA = `VDD_3V3`) and the IMU on port B (VCCB = `VDD_1V8`):
  - A1, A2, A3 → B1Y, B2Y, B3Y carry SCK, MOSI and CS, which are the MCU's outputs. B4 → A4Y carries MISO.
  - This matches TI's own SPI example (`read-ldo-xlat.md` §2).
  - TMAX is at least 50 Mbps for 3.3 V to 1.8 V (SCES935A p9, rendered), against the ICM-20948's 7 MHz.
  - Its B outputs go Hi-Z when OE is low or a supply is below 100 mV, so CS needs a pull-up on the IMU side (≤ 1 MΩ per the datasheet).
- **Why not the TXB0104:**
  - it requires VCCA ≤ VCCB, so the MCU would have to sit on B;
  - its outputs are guaranteed only at ±20 µA;
  - it limits pull-ups to > 50 kΩ and the load to 70 pF;
  - its auto-direction sits on a tri-stated MISO.
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
  - for an adjustable part, `feedback` `{top_ohms, bottom_ohms, part}`.
- **`power.loads[]`:** `{domain, milliamps, source}` for loads no parts row covers. The MCU's 658 mA comes from its row (Table 120), never typed by hand.
- **`power.sense[]`:** the divider, its resistor part, the battery as source, the ADC net and its pin (solved).
- **`power.crossings[]`:** `{reference, part, bus, a_domain, b_domain}`. Each channel's direction is the part row's.
- **`voltage_domains` is unchanged** (`INV-01`): `power` names domains and never restates them.
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
  - a buck child: its ISC max (D2).

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
| Battery against VIN | cells × cell min/max; the row's VIN min-operating (max) and VIN recommended max | `BatteryOutsideRegulatorInput` | pass, all of 2S–6S-HV |
| LDO dropout | the input domain's guaranteed minimum (its own regulator's), VOUT max, VDO max at the row's current and Tj | `DropoutViolated` | pass: 3.191 ≥ 2.227 V |
| Buck headroom | D ≤ 1 − tOFF-MIN(max) × fSW(max), RDS(on) max, the inductor's DCR max, the load | `BuckHeadroomViolated` | pass at 2 A: 5.389 ≥ 5.228 V (conditions: DCR, VIN = 12 V test point) |
| Minimum on-time | VOUT min / VIN max / fSW max against tON-MIN max | `OnTimeTooShort` | pass: 401 and 266 ns ≥ 108 ns |
| VOUT in the domain | VFB min/max and the divider at tolerance + TCR (or fixed VOUT min/max) against nominal ± tolerance | `RegulatorOutsideDomain` | pass; margins 56/22 mV at 3.3 V |
| MCU supply | VOUT max ≤ 3.6 V (Table 111); at each supply pin, VOUT min − R(VDD+VSS) × 658 mA ≥ the floor (D3) | `SupplyBelowFloor` (with `1303`) | 85 mΩ (domain floor) or 777 mΩ (BOR3 floor) |
| BOR margin | the declared BOR level's falling maximum (Table 115) | part of the above | VBOR3 2.68 V |
| Load against IOUT | S2's sum | `RegulatorOverloaded` | 0.678/3, 2/3 and 0.02/0.5 A |
| Power-in | Σ ISC max against the connector's rated current | `PowerInputOverloaded` | 10.1 ≤ 15 A |
| Sense full scale | battery max × the ratio at worst tolerance + TCR, against VREF+ min (VDDA's domain minimum) | `SenseFullScaleExceeded` | pass: 2.445 ≤ 3.135 V |
| Sense source impedance | the divider's Thevenin R against RAIN max at the declared resolution (Table 173), or a declared reservoir C | `SenseSourceImpedance` | **fail: 9.09 kΩ > 1150 Ω** (D10) |
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

**The placer:** today it places in courtyard rows. The datasheet's loop rules need adjacency: Cin at VIN/PGND, CBOOT at BOOT/SW, CVCC at VCC, the FB divider at FB, and the inductor at SW.

**The router:**
- **It refuses every through-hole pad** (`route.rs:1876`). That means the LMR33630's default ThermalVias footprint and every AMASS power-in footprint. The plain HSOP-8 routes, but loses the datasheet's **4 × 3 thermal-via array** (§10.1.1).
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
  - the return pad is **the regulator's EP (pad 9)**.
  - Copper between the inductor and RFBT's top is inside the loop, and is not the MCU's drop.
- **What it needs:**
  - the regulators table naming pin roles;
  - the emitter giving the divider's resistors references tied to their regulator;
  - `SolvedTopology` carrying `{domain: (source ref, pad), (return ref, pad)}`.

  Then `1303` replaces its stand-ins with one source, and D7's VDD+VSS sum is the two R bounds from those two pads.

## Decisions needed

- **D1 · The tree, and thermal.** Recommended: two LMR33630s from the battery (5 V and 3.3 V), and the TLV75518 off 3.3 V. The alternatives:
  - 3.3 V by LDO off 5 V, about 1.2 W in SOT-223. That needs a thermal check S3 does not have.
  - A 3.3 V buck off the 5 V buck, refused by D2's bound: 5.05 A against 3 A.

  Should a junction-temperature check (P × RθJA against Tj max) join S3 for LDOs? RθJA is a JEDEC-board figure, which TI says "cannot be used for design" for the buck, so for the buck it would be a named condition.
- **D2 · A regulator behind a buck.** Bound its input by the buck's ISC max (guaranteed, but tested open loop: TI note 4), or refuse a buck fed from any regulator (`BuckFedByRegulator`)? **Recommended:** the ISC bound with the note as a named condition. The same bound gives the power-in check its input current, since no efficiency is guaranteed by any of the four bucks.
- **D3 · Where the copper drop is judged.** The 3.3 V regulator uses all but 56 mV of the declared ±5 %. Judging the MCU's pins against the domain floor allows 85 mΩ for VDD+VSS at 658 mA, below `1303`'s 50–300 mΩ measured on VDD alone at 0.2 mm. **Recommended:**
  - the domain's tolerance holds at the source pad (`RegulatorOutsideDomain`);
  - each MCU pin is judged against the declared BOR level's falling maximum (VBOR3 2.68 V gives 777 mΩ) and VDD's 1.62 V. This is the MCU's own floor, and needs D4.
- **D4 · The BOR level in the specification.** Today the scenario declares it (`power::schedule`). One source (`INV-01`) suggests `mcu.bor_level` in the specification, which the scenario and the D3 check both read. The option-byte write that sets it in firmware is out of this work order.
- **D5 · The buck's headroom at 2S.** Accept the computed bound from guaranteed fields, with "tOFF limit stated at VIN 12 V" and the inductor's DCR as conditions? Or require the battery minimum to be ≥ 7 V, TI's own (untested) system-table floor for 5 V?
- **D6 · External parts.** Capacitors' effective capacitance under DC bias has no guaranteed source; vendors publish typical curves. **Recommended:**
  - the check reads **rated** values against the datasheet's rated table (Table 9-2), with effective C named as a condition;
  - the inductor is a cited part row: L, Isat minimum, DCR maximum.

  p22's Eq. 5 contradiction and the 6–26 V ripple trade-off go to the build as a stated choice (TI's 8 µH at 400 kHz).
- **D7 · Power-in.** AMASS XT30PW-M, 15 A (spec V1.2, a distributor copy; Scotty for the maker's copy). It is through-hole, and the router refuses through-hole pads (`route.rs:1876`). Either `1308` gains through-hole pads, or the build uses an SMD power-in (none is in KiCad's AMASS library), or wire pads with a declared rating. **Recommended:** `1308` gains through-hole pads.
- **D8 · Thermal vias under the buck's EP.** The datasheet requires a 4 × 3 array. The router refuses the footprint that carries them. **Recommended:** `1308`'s via option includes vias on a pad, and the build uses the ThermalVias footprint once that lands. Until then the plain footprint, with the rule named as an unverified condition.
- **D9 · The 1.8 V answer:** TLV75518PDBV plus TXU0304PW (MCU on A), with `crossings` in the specification and the CS pull-up on B. The alternative is a 3.3 V-VDDIO IMU (`4210` D1, Scotty). Also: does the MISO round-trip check join `4300c`'s speed rules?
- **D10 · Battery sense against RAIN.** A 9.09 kΩ divider exceeds RAIN's 1150 Ω at 12 bits. Options:
  - (a) a reservoir capacitor at the pin, its rule cited (ST AN2834, not held here);
  - (b) a low-impedance divider (about 10 k/1.1 k, 2.3 mA at 25 V);
  - (c) leave it to the firmware's sampling time (ArduPilot's, which the board cannot check).

  **Recommended:** (a), with the capacitor a declared part and the rule cited once the application note is held.
- **D11 · A regulators table** beside the sensor rows (not a widened sensor row), with pin roles, per-field citations and `eol`. The same table holds translators.
- **D12 · The schematic emitter** omits `PWR_FLAG` on a rail whose source pin is `power_out` (an LDO), and keeps it on a buck's rail. That is a read at KiCad's source (S4), not yet measured with ERC; the build measures it.

## Findings

| id | finding | where | kind | decision |
|---|---|---|---|---|
| F1 | DS12117 Rev 10 has two electrical sections, §6 (rev Y) and §7 (rev V). `power.rs` cites Table 16 (§6). Table 115 (§7, p212), the rev V table beside Table 120's 658 mA, carries the same BOR values. No value changes; the citation could name both | `aethelgard-chaos/src/power.rs` L22–28 | read | none (backlog) |
| F2 | The buck reader read the UVLO as "about 3.7 V, typical, text only" and missed p7's VIN minimum-operating **maximum 3.8 V**, a guaranteed field. Caught at spot-check | `read-bucks.md` §1.5 | process | none |
| F3 | My P1.6 was half wrong: a buck's input is bounded by its high-side current limit. The prediction stands as written beside the result | P1.6 | lesson | D2 |
| F4 | The pinned image ships no licence text beside KiCad's libraries (`1300` D10, still open) | `kicadlib.sh` | measured | none here |
| F5 | The LMR33630 datasheet's p22 labels Eq. 5 L_MIN, but the sentence above it describes a maximum | `read-bucks.md` §1.13 | read | D6 |
| F6 | The LMR51430 datasheet labels its package DBV in places; every orderable is DDC | `read-bucks.md` §3 | read | none |
