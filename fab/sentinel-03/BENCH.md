# Sentinel-03: the bench plan

The rows to run when the boards arrive (AET-WO-4300k (8)). Each row names:
- the prediction it tests and what that prediction rests on;
- the instrument;
- the pass line, a number from a cited prediction; a reading outside it fails the row;
- what a disagreement adds to the record.

A failed row is a finding, never a retry.

The board is `examples/sentinel-03.json`, and its package is `package/` beside this file. The ELF is `package/sentinel-03.elf`, sha256 `66d9b7f61d7057929ec14f83493d8ac9ae137476ed89b709fb1c68a787b6074c`, the hash `package/MANIFEST.json` records (a tier-1 test holds the two equal, as `AET-WO-3308` did for Sentinel-01). Every row runs that ELF.

**What each prediction rests on** (`RUL-20`'s terms, as Sentinel-01's sheet labels them):
- **rule:** an independent rule in `aethelgard-ir`, or a datasheet limit cited at source.
- **twin:** the twin's own run of this ELF.
- **typical:** a maker's typical figure. The row measures the board, because nothing guarantees the figure.
- **not predicted:** said plainly. The row records it.

The rows are the conditions `examples/sentinel-03.json` acknowledges "a bench row" for (FEEDBACK §8 lists every acknowledgement).

> **The pack is 3S to 4S only** (ruled at the work order): 9.0 V to 17.4 V on J4 (XT30PW-M). F1 (the 7 A Bourns SF-1206F) and Q1 (DMP6023LSS-13, reverse-polarity P-FET) protect the flight-controller branch only. No motor current crosses this board.

## Before power

1. **Orientation, by eye.** Every connector's mating opening faces out of the board at its edge; J4 is turned 180° (`extract.openings`, rule). A connector facing the board is a placement finding.
2. **No short on the input, ohmmeter.** Measure J4.2 to J4.1 both ways, and VBAT to GND. Neither may read under 100 Ω (not predicted; the bulk and ceramics charge, so read after settling). A short is an assembly finding.

## Rows

### B-1 The input path's drop (`input_path_resistance_at_25c:Q1`)
- **Prediction (rule, at 25 °C):** at 1,906 mA the drop from J4.2 to `VBAT` is at most 81 mV. That is 33 mΩ (DMP6023LSS RDS(on) maximum at |VGS| 4.5 V) plus 9.375 mΩ (F1's typical plus 25 %, the 7 A Bourns SF-1206F).
- **Instrument:** a bench supply at 9.0 V on J4 with the board running the ELF, a DMM across J4.2 and `VBAT` (Kelvin), and a clamp meter or the supply's ammeter.
- **Source line** (this verdict's condition `input_path_resistance_at_25c:Q1`): "the input path's drop is 81 mV at 1906 mA through DMP6023LSS-13's RDS(on) 33 mΩ (|VGS| 4500 mV) and SF-1206F700-2's 9375 µΩ, each stated at 25 °C". 42.375 mΩ × 1,906 mA is 80.77 mV, rounded up; the acknowledgement's reason says 80.
- **Pass line:** drop ≤ 81 mV × (measured current / 1,906 mA), at an ambient near 25 °C.
- **A disagreement adds:** the measured R at 25 °C. Repeated hot (B-4 run), it gives the temperature rise the condition names.

### B-2 The supervisor's thresholds (`supervisor_hysteresis_typical:U12`)
- **Prediction (rule and typical):**
  - U12 (TLV803S) asserts `NRST` falling between 2,870 and 2,990 mV (SBVS157E p. 6, VIT−).
  - It releases rising at most 3,030 mV: VIT− maximum plus the 40 mV typical hysteresis.
- **Instrument:** a programmable supply on VDD_3V3, with U11's output isolated or back-fed per its datasheet, ramped at ≤ 10 mV/s; a scope on `NRST` and VDD_3V3.
- **Pass line:** falling 2,870–2,990 mV; rising ≤ 3,030 mV.
- **A disagreement adds:** the measured hysteresis. A release above 3,034 mV (VDD_3V3's floor at its pin, ruled D6) is a finding against the board, not a retry.

### B-3 The supervisor's delay (`supervisor_delay_unstated:U12`)
- **Prediction (rule above 3.19 V; not predicted below):** with VDD_3V3 stepped from 0 to its regulated value (above 3,190 mV), `NRST` releases 120–280 ms after VDD crosses VIT− + 0.2 V (SBVS157E p. 6, §7.6).
- **Instrument:** the scope, as in B-2, triggered on VDD_3V3 rising.
- **Pass line:** 120–280 ms. Also record the delay at a slow ramp that stops at 3,034 mV, which TI does not state.
- **A disagreement adds:** the delay at the rail's worst case. This is the twin's reset row's figure (FEEDBACK §6).

### B-4 U10's temperature (`buck_junction_screened:U10`, `theta_ja_copper_mismatch:U10`)
- **Prediction (typical):** at 2 A out on VDD_5V and 1,170 mW of loss, the junction rises about 32.8 K above the board (28.0 °C/W, SNVSAN3F Figure 9-3's point at 20 cm², for the 20.82 cm² the board joins per layer). The figure's board has 2 oz/1 oz copper; this one declares 1 oz/½ oz, so expect more.
- **Instrument:**
  - an electronic load of 2 A on VDD_5V, at a connector's 5 V pin or a test lead;
  - a thermocouple on U10's case, or a thermal camera with the emissivity set for the package;
  - 30 minutes at a stable ambient.
- **Source line** (`package/FAB-NOTES.md`, the verdict's condition): "U10: its junction is screened, not guaranteed: 1170 mW of loss … at θJA 28.0 °C/W, the figure's point at 20 cm² for the 20.82 cm² per joined layer measured on the drawn board … is 117760 m°C on a 85 °C board, 7240 mK under 125 °C".
- **Pass line:** case rise ≤ 40 K above ambient. That is 125 °C less the board's 85 °C maximum, with the case taken as the junction.
- **A disagreement adds:** the board's own θ, which replaces the screen's typical. This is Q2's (b)/(c) evidence.

### B-5 U11's temperature (`buck_junction_screened:U11`, `theta_ja_copper_mismatch:U11`)
- **Prediction (typical):** at its solved 750 mA load and 570 mW of loss, the junction rises about 20.5 K (36.0 °C/W at 10 cm², for the 13.10 cm² the board joins).
- **Source line** (`package/FAB-NOTES.md`): "U11: its junction is screened, not guaranteed: 570 mW of loss (… 82 % on the 36 V curve at 0.750 A (82.81 % read from its vectors, AET-WO-4411 (c1)) …) at θJA 36.0 °C/W, the figure's point at 10 cm² for the 13.10 cm² per joined layer … is 105520 m°C on a 85 °C board, 19480 mK under 125 °C".
- **Instrument:** as in B-4, on VDD_3V3 at 750 mA.
- **Pass line:** case rise ≤ 40 K above ambient.
- **A disagreement adds:** the board's own θ.

### B-6 The crystal's drive level (`hse_drive_level_unmeasured:HSE`)
- **Prediction (not predicted):** AN2867 Rev 25 §3.5.2 (p. 16) computes the drive level from the voltage swing at the amplifier input (OSC_IN).
- **Instrument:** a probe of at most 1 pF on OSC_IN, with the HSE running the ELF's bring-up.
- **Pass line:** DL ≤ 100 µW, ABM8AIG's maximum (p. 1). Compute DL by §3.5.2's formula with the measured Vpp, CL1 = CL2 = 18 pF as drawn, and the probe's capacitance.
- **A disagreement adds:** the need for an external resistor (AN2867 Step 4). That is a board change and a ruling.

### B-7 The crystal's safety factor (`hse_safety_factor_unmeasured:HSE`)
- **Prediction (rule, partly):** gmcrit is 0.555 mA/V, under the 1.5 mA/V limit (AN2867 Rev 25 §3.4). The negative-resistance safety factor needs the board (§3.8).
- **Instrument:** a series resistor added between OSC_OUT's capacitor node and the crystal (§3.8.2), raised until the HSE fails to start, on three power-ups each.
- **Pass line:** Sf ≥ 5, AN2867 Rev 25 p. 19, Table 4 ("safe" for an HSE). Sf is the added resistance at failure over the crystal's 70 Ω ESR maximum.
- **A disagreement adds:** Sf, with the starts that failed. An Sf under 5 is a crystal or load-capacitor finding.

### B-8 U13's temperature (`buck_junction_screened:U13`, `theta_ja_copper_mismatch:U13`)
- **Prediction (typical):** at its 505 mA load on `VDD_3V3_RF` and 490 mW of loss, the junction rises about 17.6 K (36.0 °C/W at 10 cm², for the 13.26 cm² the board joins; SNVSAW1E Figure 9-3). The figure's board has 2 oz/1 oz copper; this one declares 1 oz/½ oz, so expect more.
- **Source line** (`package/FAB-NOTES.md`): "U13: its junction is screened, not guaranteed: 490 mW of loss (declared: SNVSAW1E p. 26, Figure 9-8, typical at TA 25 C: 78 % on the 36 V curve at 0.505 A (79.09 % read from its vectors, AET-WO-4411 (c1)) …) at θJA 36.0 °C/W, the figure's point at 10 cm² for the 13.26 cm² per joined layer … is 102640 m°C on a 85 °C board, 22360 mK under 125 °C". The θJA curve's copper (2 oz/1 oz against the board's 1 oz/½ oz) is `theta_ja_copper_mismatch:U13`.
- **Instrument:** as in B-4, on `VDD_3V3_RF` at 505 mA, with U30 held in reset (C90 shorted, so `/U30_EN` is low) so the load sets the current.
- **Pass line:** case rise ≤ 40 K above ambient.
- **A disagreement adds:** the board's own θ for U13.

### B-9 U31's input thresholds (`translator_threshold_inferred:U31`)
- **Prediction (rule, by inference; ruled 04:51Z G1 (a)):** each data input switches with VT+ at most 640 ‰ of its own supply and VT− at least 296 ‰. Those are SCES935A p. 7, 7.5's VCCI 3 V row (1.92 V maximum, 0.89 V minimum) scaled to the rail. The datasheet states no 3.3 V row. The line levels `LineLevelUnmet` judged rest on these figures.
- **Prediction at the release floor (assumption; `AET-WO-4310` (2), ruled 2026-11-27T07:42Z 2 (a)):** at VCCI 2.87 V, U12's falling minimum, each data input switches with VT+ at most 679 ‰ (1.949 V) and VT− at least 243 ‰ (0.697 V). The datasheet states VT+ and VT− at VCCI 2.3 V (1.56 V maximum, 0.56 V minimum) and 3 V and at no supply between them, so the line levels judged at the release floor take the worse of the two rows: an assumption, which this measurement turns into a figure.
- **Source line** (this verdict's condition): "U31: VT+ and VT- are inferred, not stated: TI TXU0304 datasheet SCES935A A (November 2021), p. 7, 7.5 VT+ data inputs, VCCI 3 V, TA -40 to 125 C: max 1.92 V = 640 per mille … ruled 2026-11-26T04:51Z G1 (a), an inference; a bench row (RUL-20 (3)): VT+ and VT- measured at the board's rails at bring-up; at the reset's release floor of 2870 mV its A side's inputs are judged with VT+ 679 and VT- 243 per mille". VT−'s row: "min 0.89 V, 296 per mille rounded down" (`power-parts.json`).
- **Instrument:**
  - a programmable supply, ramped at ≤ 10 mV/s up to the input's rail and back to 0 V, through 1 kΩ into the input;
  - a scope on the input and on the channel's output; a DMM on the input's rail.
  - **A1** (pad 2, `/WIFI_TX`, VCCA = VDD_3V3): U1 held in reset through J2's NRST; output B1Y (pad 13).
  - **B4** (pad 10, `/U31_B4`, VCCB = VDD_3V3_RF): U30 held in reset (C90 shorted); output A4Y (pad 5).
  - **At VCCI 2.87 V:** a TXU0304PW from the same reel on a breakout, VCCA from the programmable supply at 2.87 V (DMM-read) and VCCB at 3.3 V, the same ramp into A1, output B1Y. The board's VDD_3V3 is not lowered: U11 regulates it, and the MCU would reset.
- **Pass line:** at each input, rising switch ≤ 640 ‰ × that rail's measured voltage, and falling switch ≥ 296 ‰ × it. At VCCI 2.87 V, rising switch ≤ 679 ‰ × the measured VCCA and falling switch ≥ 243 ‰ × it.
- **A disagreement adds:** the measured VT+ and VT− at 3.3 V, which replace the inference in the parts row; at 2.87 V, the measured pair, which replaces the assumption in the parts row's `lower_band`. A rising switch above the line is a finding against the UART levels (FEEDBACK §1, (c1)).
