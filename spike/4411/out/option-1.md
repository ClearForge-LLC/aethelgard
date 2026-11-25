# AET-WO-4411 option 1: the module on its own regulator, the scratch measurement

Ruled 2026-11-25T20:10Z. The predictions were committed before any emission (ae9146bf, `PREDICTIONS.md`). Those predictions are for a **battery-fed** LMR33630; that board is **c** below, measured with the J4 screen bypassed in scratch. Nothing in `crates/` is committed. The scratch edits are in `option1/`:
- `scratch.patch`: `radio.rs`'s `RadioSupplyNotMcu` refusal off; `supply.rs` lets a domain U1 has no pin on take its radio module as the load.
- `scratch-battery.patch`: the same, plus J4's fault sum capped at 15000 mA.

The crates are reverted. The specifications are built by `option1/mk.py` plus the edits under "What was emitted".

## The answer

**Option 1 is in reach, but not with a part the library holds.** It needs a new battery-fed buck plus three build items. No held regulator passes the rules:

| Route | What refuses it |
|---|---|
| TLV755P LDO (the one held LDO family) | **The junction.** (5.237 - 3.3) V x 0.5 A = 0.97 W at θJA 231.1 °C/W (SBVS320D p. 5, 5.4), a 224 °C rise against 125 °C. Its 500 mA rating (p. 4, 5.3) equals the module's 500 mA (WROOM-02 v1.7 p. 21, Table 6-2). A battery-fed LDO is also refused outright: `PowerInputOverloaded`, no cited fault bound. |
| Third LMR33630, battery-fed | **`validate`: `PowerInputOverloaded`, J4.** "fault: 15150 mA through it, above its rated 15000 mA": three current-limit maxima of 5050 mA (SNVSAN3F p. 7, 7.5) against the XT30's 15 A. Over by 150 mA. |
| Third LMR33630, fed from `VDD_5V` (board b) | **U10.**<br>- U10's load becomes 2502 mA. Figure 9-7 at 2.5 A reads 90.0-90.6 %, taken as 90 %, so U10 loses 1460 mW.<br>- That needs the θJA figure's 30 cm² point (27.0 °C/W): Tj = 124.3 °C, 0.7 °C under the limit. At 20 cm² (28.0 °C/W) it reaches 125.8 °C. So U10 needs at least 30 cm² per joined layer, against 15 cm² today.<br>- This holds for any regulator fed from `VDD_5V`, because the model counts the child's output current on U10.<br>- On this placement the outline also breaks the declared 100 x 100 mm (98.4 x 111.25). |

**The paper pick: a battery-fed sibling.** Both datasheets were fetched 2026-11-25 with `curl -fsSL` from ti.com and are held in `~/.cache/aet-held/incoming/4411-opt1/`.

| | LMR33620ADDA (SNVSAW1E, Feb 2018 rev. Nov 2020) | LMR33610ADDA (SNVSBI9A, Oct 2019 rev. Jun 2022) |
|---|---|---|
| sha256 | 865af73e62240338821cc0915dae0eeabec94f4602e0e7cc5fb1c1d49d3daf2b | 777b3966aff2772cd6680193c7ba7cd7a0b79778b418aafbcf7ce2c147e39611 |
| High-side current limit max | 4.0 A (p. 6, ISC) | 4.0 A (p. 5, ISC) |
| J4 fault with it | 10100 + 4000 = **14100 mA**, under 15000 | the same |
| VIN to PGND absolute maximum | 38 V (p. 5, 7.1), above the 34.8 V hot-plug ring | 38 V (p. 4, 7.1) |
| Rated output | 2 A | 1 A, twice the module's 500 mA |
| θJA versus copper, DDA, four layers | Figure 9-3 (p. 24); the axis tops out at 44 °C/W | Figure 9-3 (p. 22); the axis tops out at 44 °C/W |
| Efficiency at 0.5 A, 3.3 V, 400 kHz, 36 V curve | 78.96 % (Figure 9-8, p. 26) | 78.93 % (Figure 9-7, p. 24) |
| Loss, taken at 78 % | 3.417 V x 500 mA x (1/0.78 - 1) = **482 mW**; Tj at most 85 + 0.482 x 44 = **106.2 °C** at any area on the figure | the same |
| Table 9-2 at 3.3 V, 400 kHz | L 10 µH, **COUT 4 x 22 µF**, CIN 10 µF + 220 nF (U11's block) | L 10 µH, **COUT 2 x 22 µF**, CIN 4.7 µF + 220 nF |
| KiCad 10.0.7 symbol | **`Regulator_Switching:LMR33620ADDA`** upstream; it extends `LMR33640ADDA`, already vendored (fetched file sha256 6a19ba51…) | **none** upstream (404), so the symbol would be our own, as the TLV803S's is |
| Footprint | HSOP-8 DDA, the LMR33630's, vendored | the same; the parts are pin-compatible (SNVSBI9A p. 1) |

- **I recommend the LMR33620ADDA.** It meets every constraint, including KiCad's own symbol. Its block is U11's, and that block size has now been measured to fit (board c).
- The efficiency figures were read from the PDFs' vectors (`option1/vec-eff.py`). The reader was checked on the held SNVSAN3F Figure 9-8: 77.73 / 81.84 / 86.32 % at 0.5 / 0.679 / 1.249 A, each inside my earlier pixel reads (76.7-78.7, 81.2-82.4, 85.9-86.5).
- **Not yet judged:**
  - The inductor. Both datasheets size it by ripple at K = 0.3 of the device's own maximum (Equation 4), giving 10 µH. Whether the held 8.2 µH IHLP5050FDER8R2M01 passes the part's row is a build check; if not, a 10 µH part means one more datasheet.
  - The library row's other fields (VFB, RDS(on), timing), which come from these datasheets at build.

## What was emitted (three boards)

All three start from `examples/sentinel-03.json` (3a35be0f) run through `option1/mk.py`:
- U13 = LMR33630ADDA, a copy of U11's block (R30/R31 105 k / 46.4 k, C30/C63, C31-C33, C34-C37, L3). It stands in for the LMR33620, which has the same block, footprint and package.
- `VDD_3V3_RF` (3300 mV, 50 permille). U30, C91, C92, R90 and R91 moved to it. R30's load row is derived as R20's is.
- A scratch drop limit of floor(3,144,727 µV) - 3.0 V = **144 mV**.
- `VDD_3V3_RF` in the POWER class (500 µm).
- `max_outline_um` lifted to 100 x 115 mm. 5 V-fed at 100 x 100, the emit refuses with `OutlineUnreachable` (98.4 x 111.25).

The boards differ in:
- **b:** U13 fed from `VDD_5V`, with re-read loss rows: U10 1460 mW at 2502 mA, U11 570 mW at 749 mA (Figure 9-8 82.2-83.5 %, taken as 82 %), U13 540 mW at 500 mA.
- **c:** U13 fed from the battery (J4 bypassed in scratch), U11 at 570 mW, U10 unchanged (1170 mW at 2000 mA).
- **a:** board b without the POWER class (rail at default width). Only its module-rail figure is quoted.

## The table

| Measure | b, 5 V-fed | c, battery-fed | Predicted (for c) |
|---|---|---|---|
| Outline | **98.4 x 111.25**, over 100 x 100 | **97.65 x 99.6**, inside | does not hold, grows 10-20 mm: **wrong** |
| U1's drop (`VDD_3V3`, 749 mA, limit 110) | 101.5 mV, pass by 8.5. VDD 17 55.6; VSS **31 79.9** mOhm | **80.5 mV, pass**. VDD **30 65.2**, 17 59.4; VSS 107 42.2, 31 41.6 mOhm | about 88: within 8 mV |
| Module rail (`VDD_3V3_RF`, 503 mA, limit 144) | supply U30.1 619 mOhm (a: 1410); return U30.9 38.8; U30.19 unbounded. At least 331 mV, fail | supply U30.1 **200 mOhm**; return U30.9 37.7; **U30.19 unbounded** (`SupplyPinUnbounded`). Without pad 19, 0.503 A x (200 + 37.7) = **120 mV**; the gate as built refuses it | at most about 61 mV, pass: wrong on the supply side |
| Where U13 went | (72.5, 131.15), about 108 mm from U30; 233 mm of rail track | (86.05, 67.3), about 61 mm from U30 (140.7, 36.45); 191 mm of track | not predicted |
| U10 copper, least layer (needs 700; 3000 in b) | **391** F.Cu (In1 3630, B.Cu 3409), `BuckJunctionOverLimit` | **466** F.Cu (In1 2667, B.Cu 2540), `BuckJunctionOverLimit` | at least 700: **wrong** |
| U11 copper (needs 700) | 2433, pass | **109** F.Cu (In1 2276, B.Cu 2096), `BuckJunctionOverLimit` | at least 700: **wrong** |
| U13 copper (needs 700) | 2184, pass | 2315, pass | at least 700: right |
| DRC, raw KiCad (image, capped 8 GiB) | 36 `drill_out_of_range` (three 0.254 mm thermal arrays), nothing else | not run | 36: right |
| `check drc --spec` | pass: `drc.power` 31, `drc.radio` 1, `drc.outline` 1 (lifted bound) | not run | pass: right |
| Capacitance ladder | rung 7 (was 5), IMU_SCK 24.3 of 30 pF | not read | |
| Timing | emit 35.7 s; extract 573 s, 1.2 GB | emit 42.0 s; extract 488 s, 0.6 GB | |
| Sentinel-01 | not emitted: nothing in `crates/` changed | | |

**What c shows:**
- **The 100 x 100 outline and U1's drop hold with a third U11-sized block, once it is battery-fed.** b's outline miss came from the 5 V-fed placement, not the block's size.
- **Two misses are layout, not limits, and three are rules:**
  - The F.Cu pours: on both boards U10 and U11 keep their In1 and B.Cu copper, but F.Cu falls under the figure's 7 cm².
  - The module rail: U13 placed 61-108 mm from U30, because no rule ties a module's regulator to it.
  - Pad 19, still unbounded, carries the return gate.

## Sequencing and back-powering (held documents, read and spot-checked)

- **U1 (STM32H753), PG9 (LQFP144 pin 124) and PG14 (pin 129):** both are `FT_h` (DS12117 rev 10, Table 8, p. 84 and p. 85; checked from the PDF's word positions).
  - Table 10 (p. 103) gives VIN on FT_xxx pins a maximum of Min(Min(VDD, VDDA, VDD33USB, VBAT) + 4.0 V, 6 V). Table 11 note 3 (p. 104): "Positive injection is not possible on these I/Os and does not occur for input voltages lower than the specified maximum value."
  - **Read literally, at VDD = 0 the formula gives 4.0 V.** The datasheet does not state the unpowered case in words; this is my reading. Note 1 asks that every supply pin "must always be connected to the external power supply, in the permitted range".
  - Table 49 note 5 (p. 136-137) is tighter: "VIN must be less than Max(VDDXXX) + 3.6 V". At VDD = 0 that is 3.6 V, against the module's high of at most 3.465 V.
  - **The unpowered case is silent, so it is a named condition, not a pass.** Table 10 note 1 puts an unpowered U1 outside the table's own conditions. Read literally, both figures (4.0 V and 3.6 V) sit above the module's high, but nothing states the case.
- **U30 (ESP32-C3-WROOM-02), RXD (pin 11):**
  - VIH max = VDD + 0.3 V (v1.7 p. 21, Table 6-3; note 1: "VDD – voltage from a power pin of a respective power domain"). Absolute maximum ratings (Table 6-1) state VDD33 only.
  - Unpowered, the pin's stated ceiling is 0.3 V. **U1's TX driving high into an unpowered module is outside a stated limit.** Neither the datasheet nor the HDG states anything about I/O while unpowered.
  - The module's EN does not help here. EN low powers the chip off but leaves the pin's VDD domain at 0 V.
  - **This is a condition, not yet fixed.** The fixes in reach:
    - (i) U1 holds PG14 low or high-impedance until the module's rail is up: a firmware condition, judged by nothing here.
    - (ii) A series resistor limits the current, but VIH max is still exceeded as written.
    - (iii) U30's rail sequenced before U1's: U11's EN held by U13's PG, so U1's rail rises only after the module's. That is a new topology rule. Power-down order is still not guaranteed by it.
  - Which one is a ruling.
- **EN timing:** tSTBL and tRST, 50 µs min each (v1.7 p. 15, Table 4-6). The datasheet's RC (10 kOhm, 1 µF; v1.7 p. 33, §9) is what R90/C90 carry. On their own rail they move with it.

## The supervisor row (Q-SUP)

- **No brown-out detector is stated** in the module datasheet v1.7 or the ESP32-C3 HDG. The words brown, BOD, undervolt, supervis and "power monitor" were searched for: only the HDG's external monitor appears.
- The HDG (PDF p. 10, printed p. 7, Table 1's attention box) says: for "slow power rise or fall … frequent power on/off … unstable power supply … the RC circuit alone may not meet the timing requirements … please reserve a power monitor chip to reset the chip when the power supply is abnormal, and the threshold of the power monitor chip is recommended to be around 3.0 V."
- **The module's own circuit does not cover its 3.0 V minimum.** It stays a row: a monitor on `VDD_3V3_RF` that holds EN.
- A TLV803S (open-drain RESET, SBVS157E p. 1, held) on EN gives VIT- 2.87-2.99 V (SBVS157E). That is "around 3.0 V" in the HDG's words, but it is not at or above the module's 3.0 V minimum (Table 6-2). It reproduces the 2.87-against-3.0 gap on the module's own rail.

## For the ruling

1. **The regulator:** U13 = **LMR33620ADDA**, battery-fed, with U11's block (or the LMR33610ADDA with a symbol of our own and a half-size output bank). It needs:
   - its datasheet held, with its sha256 in `power-parts.json`;
   - the upstream symbol vendored;
   - the inductor judged.

   Fed from `VDD_5V`, U10 needs 30 cm² per joined layer at 0.7 °C of margin, and the outline broke the bound.
2. **Build items before a board can pass:**
   - (a) A placement rule that puts a module's regulator beside the module. The supply bound is 200-619 mOhm with U13 61-108 mm away.
   - (b) The F.Cu thermal pours for U10 and U11 kept at 7 cm² or more, with three bucks on the board.
   - (c) Pad 19 bounded: the 1125 µm pitch build costed in `fallback-2.md`. Otherwise the module rail's return gate is refused as unbounded, whatever the regulator.
3. **Back-powering (U1 TX into an unpowered U30)** needs (i), (ii) or (iii) above ruled.
   - The H753 side, U30's TX into an unpowered U1, is silent in DS12117. It is named as a condition.
4. **Q-SUP on the new rail:** a monitor holding EN (the HDG's "around 3.0 V") or an acknowledged row.
