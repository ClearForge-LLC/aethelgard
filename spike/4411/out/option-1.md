# AET-WO-4411 option 1: the module on its own regulator, the scratch measurement

Ruled 2026-11-25T20:10Z. The predictions were committed before any emission (ae9146bf, `PREDICTIONS.md`). Nothing in `crates/` is committed. The two scratch edits (`radio.rs`: the `RadioSupplyNotMcu` refusal off; `supply.rs`: a domain U1 has no pin on takes its radio module as the load) are in `option1/scratch.patch`, and the crates are reverted. The specifications were built by `option1/mk.py`, plus the edits listed under "What was emitted".

## The answer

**Option 1 cannot be built with any regulator the library holds, inside the rules as they stand.** Each of the three held routes is refused by a rule, not by a layout choice:

| Route | What refuses it | Margin |
|---|---|---|
| A TLV755P LDO (the one held LDO family) | The junction: (5.237 - 3.3) V x 0.5 A = 0.97 W at θJA 231.1 °C/W (SBVS320D p. 5, 5.4), a 224 °C rise against 125 °C. Its 500 mA rating (p. 4, 5.3) equals the module's 500 mA (WROOM-02 v1.7 p. 21, Table 6-2), with no margin. A battery-fed LDO is also refused outright (`PowerInputOverloaded`: no cited fault bound). | Far |
| A third LMR33630 fed from the battery | `validate`: `PowerInputOverloaded`, J4: "fault: 15150 mA through it, above its rated 15000 mA". Three buck current-limit maxima, 3 x 5050 mA (SNVSAN3F p. 7, 7.5), against the XT30's 15 A (AMASS V1.2). | 150 mA (1 %) |
| A third LMR33630 fed from `VDD_5V` (measured below) | U10's load becomes 2502 mA. Figure 9-7 at 2.5 A reads 90.0-90.6 %, taken as 90 %, so U10 loses 1460 mW. Only the figure's 30 cm² point (27.0 °C/W) keeps it under 125 °C: Tj = 85 + 1.456 x 27.0 = 124.3 °C. At 20 cm² (28.0 °C/W) it reaches 125.8 °C. U10 needs **at least 30 cm² per joined layer** (15 cm² today). It measured 29.74 cm² on the lever (1) board and **3.91 cm²** on F.Cu here. **The outline also breaks the declared 100 x 100 mm**: 98.4 x 111.25 mm. | U10: 0.7 °C at 30 cm²; outline: 11.25 mm |

Any regulator fed from `VDD_5V` adds its input current to U10. The model counts a child buck's output current as its parent's consumer, so U10's 2 A port budget (DS-009) and its thermal need move with every 5 V-fed choice, not only this one. A route that leaves U10 alone has to be fed from the battery. It then needs a buck whose current-limit maximum is at most 4900 mA (15000 - 10100), and the library holds none: SNVSAN3F covers only the LMR33630, and the vendored KiCad library holds the LMR33630 and LMR33640 symbols only.

## What was emitted

The design is `examples/sentinel-03.json` (3a35be0f), run through `option1/mk.py` with these changes:
- U13 = LMR33630ADDA, a copy of U11's block: R30/R31 105 k / 46.4 k, C30/C63 cin, C31 cin_hf, C32 cboot, C33 cvcc, C34-C37 cout, L3.
- `VDD_3V3_RF` (3300 mV, 50 permille).
- U30, C91, C92, R90 and R91 moved to the new rail. R30's load row is derived as R20's is.
- Then, by hand:
  - U13's input is `VDD_5V`.
  - The loss rows are re-read on the held Figure 9-7 and 9-8 (`option1/read-9-7.py`, `read-9-8.py`; the page-27 render of SNVSAN3F, sha256 3b0920a4…):
    - U13: 540 mW at 500 mA (36 V curve 76.7-78.7 %, taken as 76 %).
    - U11: 570 mW at 749 mA (82.2-83.5 %, taken as 82 %).
    - U10: 1460 mW at 2502 mA (Figure 9-7, whose axis runs 55-100 %. At 2.0 A it reads 90.4-90.8 %, which matches U10's committed 90 %).
  - A scratch drop limit on `VDD_3V3_RF` of floor(3,144,727 µV) - 3.0 V = **144 mV**.
  - `max_outline_um` lifted to 100 x 115 mm, because the declared bound refuses the emit (`OutlineUnreachable`: "the placer's best outline is 98.400 x 111.250 mm, over by 0.000 x 11.250 mm"). Every figure below is from a board outside the declared bound.
  - Board **b**: `VDD_3V3_RF` added to the POWER class (500 µm, as `VDD_5V`). Board **a**: the rail left at the default width.

## The table (board b; board a where it differs)

| Measure | Result | Predicted |
|---|---|---|
| U1's drop (`VDD_3V3`, 749 mA, limit 110) | **101.5 mV, pass**. Worst VDD 17, 55.6 mOhm; worst VSS **31, 79.9** mOhm (107 51.5, 51 47.6) | about 88 (80-95): **wrong**. U13's block moved the escapes, and pin 31's VSS rose from about 62.5 to 79.9 |
| Module rail's drop (`VDD_3V3_RF`, 503 mA, limit 144) | **not bounded, fail**. Supply U30.1 **619 mOhm** (a: 1410). Return U30.9 38.8 mOhm; **U30.19 unbounded** (`SupplyPinUnbounded`). Without pad 19, at least 0.503 A x (619 + 38.8) = **331 mV** | at most about 61 mV, pass: **wrong** |
| Why the rail fails | The placer put U13 at (72.5, 131.15) and L3 at (75.2, 119.3), and U30 is at (129.15, 39.3): about 108 mm apart, 233 mm of `VDD_3V3_RF` track. No rule ties a module's regulator to the module (a build item, not a limit) | |
| U10 copper (needs 3000 mm² at 1460 mW) | **391 mm² on F.Cu**; In1 3630, B.Cu 3409. `BuckJunctionOverLimit` (under the figure's smallest point, 7 cm²) (a: 402) | at least 700, unchanged: **wrong** |
| U11 copper | 2433 mm² (F.Cu least), pass | at least 700: right |
| U13 copper (needs 700 mm² at 540 mW) | 2184 mm² (F.Cu least), pass | at least 700: right |
| Outline | **98.4 x 111.25 mm**, over the declared 100 x 100 by 11.25 mm | does not hold, grows 10-20 mm: right |
| DRC, raw KiCad (in the image, `capped-run` 8 GiB) | 36 `drill_out_of_range` (U10, U11 and U13's 0.254 mm thermal drills), nothing else | 36: right |
| `check drc --spec` | pass; `drc.power` judged 31 (U13's 8 loops added), `drc.radio` 1, `drc.outline` 1 (against the lifted bound) | pass: right |
| Capacitance ladder | rung 7 (was 5), smallest margin IMU_SCK 24.3 of 30 pF | not predicted |
| Timing | emit 35.7 s; extract 573 s and 1.2 GB (a: 610 s) | not predicted |
| Sentinel-01 | not emitted: nothing in `crates/` changed | |

## Sequencing and back-powering (held documents, read and spot-checked)

- **U1 (STM32H753), PG9 (LQFP144 pin 124) and PG14 (pin 129):** both are `FT_h` (DS12117 rev 10, Table 8, p. 84 and p. 85; checked from the PDF's word positions).
  - Table 10 (p. 103) gives VIN on FT_xxx pins a maximum of Min(Min(VDD, VDDA, VDD33USB, VBAT) + 4.0 V, 6 V). Table 11 note 3 (p. 104): "Positive injection is not possible on these I/Os and does not occur for input voltages lower than the specified maximum value."
  - **Read literally, at VDD = 0 the formula gives 4.0 V.** The datasheet does not state the unpowered case in words; this is my reading. Note 1 asks that every supply pin "must always be connected to the external power supply, in the permitted range".
  - So the module's TXD driving PG9 while U1 is unpowered is within a stated maximum, read literally.
- **U30 (ESP32-C3-WROOM-02), RXD (pin 11):**
  - VIH max = VDD + 0.3 V (v1.7 p. 21, Table 6-3; note 1: "VDD – voltage from a power pin of a respective power domain"). Absolute maximum ratings (Table 6-1) state VDD33 only.
  - Unpowered, the pin's stated ceiling is 0.3 V. **U1's TX driving high into an unpowered module is outside a stated limit.** Neither the datasheet nor the HDG states anything about I/O while unpowered.
  - The module's EN does not help here. EN low powers the chip off but leaves the pin's VDD domain at 0 V.
  - **This is a condition, not yet fixed.** The fixes in reach:
    - (i) U1 holds PG14 low or high-impedance until the module's rail is up: a firmware condition, judged by nothing here.
    - (ii) A series resistor limits the current, but VIH max is still exceeded as written.
    - (iii) U30's rail sequenced before U1's: U13 enabled from `VDD_5V`, and U11's EN held by U13's PG. That is a new topology rule.
  - Which one is a ruling.
- **EN timing:** tSTBL and tRST, 50 µs min each (v1.7 p. 15, Table 4-6). The datasheet's RC (10 kOhm, 1 µF; v1.7 p. 33, §9) is what R90/C90 carry. On their own rail they move with it.

## The supervisor row (Q-SUP)

- **No brown-out detector is stated** in the module datasheet v1.7 or the ESP32-C3 HDG. The words brown, BOD, undervolt, supervis and "power monitor" were searched for: only the HDG's external monitor appears.
- The HDG (PDF p. 10, printed p. 7, Table 1's attention box) says: for "slow power rise or fall … frequent power on/off … unstable power supply … the RC circuit alone may not meet the timing requirements … please reserve a power monitor chip to reset the chip when the power supply is abnormal, and the threshold of the power monitor chip is recommended to be around 3.0 V."
- **The module's own circuit does not cover its 3.0 V minimum.** It stays a row: a monitor on `VDD_3V3_RF` that holds EN.
- A TLV803S (open-drain RESET, SBVS157E p. 1, held) on EN gives VIT- 2.87-2.99 V (SBVS157E). That is "around 3.0 V" in the HDG's words, but it is not at or above the module's 3.0 V minimum (Table 6-2). It reproduces the 2.87-against-3.0 gap on the module's own rail.

## For the ruling

1. **Battery-fed LMR33630 (refused at `validate`):** it misses by 150 mA (1 %). The screen sums every battery-fed buck's current-limit maximum. Whether that sum is the right fault bound for J4 is a ruling on the screen. It is named here only because it is the nearest miss, not as a proposal.
2. **5 V-fed (measured):** it needs all three of:
   - (a) the outline bound raised to about 100 x 115 mm, Scotty's call, since the bound is 1313's;
   - (b) U10 at least 30 cm² per joined layer at 0.7 °C of margin, or U10's 2 A port budget lowered (4300k's option (c));
   - (c) a build rule placing a module's regulator beside the module.
3. **A new, smaller battery-fed buck,** with a current-limit maximum of at most 4900 mA. It needs:
   - a fetched and hashed datasheet with a θJA-versus-copper figure (the junction screen's input), or a ruling that admits a buck row with a single θJA, as the LDO rows have;
   - a vendored KiCad symbol and footprint.

   This leaves U10 and J4 alone and should shrink the block, but it is not measured.
4. **Back-powering (U1 TX into an unpowered U30)** needs (i), (ii) or (iii) ruled, whichever regulator is chosen.
