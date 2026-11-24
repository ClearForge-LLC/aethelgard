# Sentinel-03: the bench plan

The rows to run when the boards arrive (AET-WO-4300k (8)). Each row names:
- the prediction it tests and what that prediction rests on;
- the instrument;
- the pass line, a number from a cited prediction; a reading outside it fails the row;
- what a disagreement adds to the record.

A failed row is a finding, never a retry.

The board is `examples/sentinel-03.json`, and its package is `package/` beside this file. The ELF is `package/sentinel-03.elf`, sha256 `6e4a81f299a84c37af4491d8d85d2befddeaab54cbd6787109c4f570fc0ca908`, the hash `package/MANIFEST.json` records (a tier-1 test holds the two equal, as `AET-WO-3308` did for Sentinel-01). Every row runs that ELF.

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
- **Prediction (rule, at 25 °C):** at 1,653 mA the drop from J4.2 to `VBAT` is at most 71 mV. That is 33 mΩ (DMP6023LSS RDS(on) maximum at |VGS| 4.5 V) plus 9.375 mΩ (F1's typical plus 25 %, the 7 A Bourns SF-1206F).
- **Instrument:** a bench supply at 9.0 V on J4 with the board running the ELF, a DMM across J4.2 and `VBAT` (Kelvin), and a clamp meter or the supply's ammeter.
- **Pass line:** drop ≤ 71 mV × (measured current / 1,653 mA), at an ambient near 25 °C.
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
- **Prediction (typical):** at 2 A out on VDD_5V and 1,170 mW of loss, the junction rises about 32.8 K above the board (28.0 °C/W at the 20 cm² the board joins on F.Cu, SNVSAN3F Figure 9-3). The figure's board has 2 oz/1 oz copper; this one declares 1 oz/½ oz, so expect more.
- **Instrument:**
  - an electronic load of 2 A on VDD_5V, at a connector's 5 V pin or a test lead;
  - a thermocouple on U10's case, or a thermal camera with the emissivity set for the package;
  - 30 minutes at a stable ambient.
- **Pass line:** case rise ≤ 40 K above ambient. That is 125 °C less the board's 85 °C maximum, with the case taken as the junction.
- **A disagreement adds:** the board's own θ, which replaces the screen's typical. This is Q2's (b)/(c) evidence.

### B-5 U11's temperature (`buck_junction_screened:U11`, `theta_ja_copper_mismatch:U11`)
- **Prediction (typical):** at its solved 749 mA load and 550 mW of loss, the junction rises about 19.8 K (36.0 °C/W at 10 cm²).
- **Instrument:** as in B-4, on VDD_3V3 at 749 mA.
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
