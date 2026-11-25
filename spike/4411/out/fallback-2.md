# AET-WO-4411 fallback (2): a lower-threshold supervisor, on paper (ruled 2026-11-25T17:22Z; nothing built)

**Held documents, each hash checked against the row that cites it:**
- **SBVS157E** (TI TLV803/853/863, Rev. E, December 2020): `~/.cache/aet-held/ti-tlv803-sbvs157.pdf`, sha256 `1645cfe1…`, as in `power-parts.json`.
- **W25Q64JV** (Rev. J, 2018-03-27): `~/.cache/aet-4200/ds/w25q64jv_revj.pdf`, sha256 `452d906c…`, as in `parts.json`.
- **ESP32-C3-WROOM-02 datasheet v1.7:** `~/.cache/aet-held/espressif-esp32-c3-wroom-02-datasheet-v1.7.pdf`, sha256 `622db737…`.
- **DS12117 Rev 10** (STM32H753): sha256 `607b9ead…`.
- **The sensors' rows:** read from `parts.json`, with their own hashes (cited there). Their PDFs were not reopened.

## 1. The TLV803's thresholds (SBVS157E p. 4, Table 5-1; p. 6, 7.5 Electrical Characteristics)

| variant (orderable, SOT-23 DBZ, p. 12+) | VIT− min / typ / max (TJ −40 to 125 °C) | Vhys typ (TJ 25 °C; typical only) | rising max = VIT− max + Vhys typ |
|---|---|---|---|
| TLV803S (`TLV803SDBZR`, on the board as U12) | 2.87 / 2.93 / 2.99 V | 40 mV | 3030 mV |
| TLV803R (`TLV803RDBZR`) | 2.58 / 2.64 / 2.70 V | 35 mV | 2735 mV |
| TLV803Z (`TLV803ZDBZR`) | 2.20 / 2.25 / 2.30 V | 30 mV | 2330 mV |
| TLV8x3M (`TLV803MDBZR`) | 4.28 / 4.38 / 4.48 V | 60 mV | above a 3.3 V rail; not a candidate |

- **The family has no other threshold:** Table 5-1 lists exactly Z, R, S and M.
- **TLV853 and TLV863** are "an alternate pinout" (p. 1), so they are not pin-compatible.
- **The TLV803E** is named on p. 1 as "a pin-to-pin alternative to all of these 3". Its datasheet is not held, so none of its thresholds is cited here.

## 2. D6's arithmetic per variant

- **D6** (4300k FEEDBACK l. 156): the rising maximum must sit at or below VDD_3V3's solved minimum, floored to whole mV, less the drop limit.
- **The solved minimum:** `solve` on `examples/sentinel-03.json` (with the module) gives `vout_min_microvolts` 3,144,727, so the floor is 3144 mV.
- **So the largest limit each variant allows is 3144 − (VIT− max + Vhys typ):**

| variant | largest limit (0 mV margin) | as D6 ruled it (rounded down to 10 mV) | lever (iv) 1.54 mm: 113.0 mV | R2 2.5 mm: 118.3 mV |
|---|---|---|---|---|
| S (today) | **114 mV** | 110 mV (4 mV margin) | passes 114 by 1.0 mV; fails the ruled 110 | fails 114 by 4.3 mV |
| R | 409 mV | 400 mV | passes | passes |
| Z | 814 mV | 810 mV | passes | passes |

## 3. Every part on VDD_3V3, with its minimum supply

The floor rule is `supervisor::floors_above_reset`, ruled D12 and refused by `verify --fab-out`. It requires each judged device's VCC minimum, at its bus clock, to be at or below the reset's VIT− minimum.

| part | minimum (cited) | where | judged by the floor rule |
|---|---|---|---|
| **U30 ESP32-C3-WROOM-02** | **3.0 V** | datasheet v1.7 p. 21, Table 6-2 (VDD33 3.0 / 3.3 / 3.6 V; read in the held PDF) | no (radio; Q-SUP (a), named in the radio verdict as not judged) |
| **U4 W25Q64JV** (SPI `FLASH`, 20 MHz) | **2.7 V** | 9.2 Operating Ranges, printed p. 59: VCC 2.7–3.0 V at FR 104 MHz, 3.0–3.6 V at FR 133 MHz, fR 50 MHz (read in the held PDF). At 20 MHz, `supply_min_at` takes the 104 MHz row's 2.7 V | yes |
| U1 STM32H753ZI | 1.62 V; BOR3 falling 2.54–2.68 V | DS12117 p. 209, Table 111; p. 212, Table 115 | the MCU is the reset's subject |
| U3 TMP117 (I2C 400 kHz) | 1.8 V | SNOSD82D 6.3 | yes |
| U5 ICM-42688-P (SPI 4 MHz) | 1.71 V | DS-000347 3.3.1, Table 3 | yes |
| U7 BMP581 (I2C 400 kHz) | 1.71 V | BST-BMP581-DS004-13, Table 4 | yes |
| U12 TLV803S itself | 1.1 V | SBVS157E 7.4 | – |
| J2, J3 (off-board loads: 1 mA and 50 mA) | not declared | – | not judged |

**The binding parts, per variant:**
- **S (VIT− min 2.87 V):** only the module is above it (3.0 V). That is today's Q-SUP, named and not judged. The flash has 170 mV in hand.
- **R (VIT− min 2.58 V):** the **W25Q64JV binds** (2.70 > 2.58, by 120 mV), so `floors_above_reset` refuses it, and `verify --fab-out` with it. The module is 420 mV above.
- **Z (VIT− min 2.20 V):** the **W25Q64JV binds** (by 500 mV), and the module is 800 mV above.

**"At its own pin":**
- The rule compares at U12's VIT− minimum with no drop between U12's pin and each part's.
- The gate bounds only the MCU's path (L2 to U1) and, under 1 (a'), the module's shared path. The drop from U12 to U4 or to U30 is not measured.
- So every margin above must also cover that unmeasured drop.

## 4. What follows, arithmetic only

- **No TLV803 variant both clears the floor rule and allows the measured drop.**
  - S keeps the floor (the flash's 2.7 V; the module stays unjudged, as ruled) but allows at most 114 mV.
  - R and Z allow far more drop but leave the flash out of range while the MCU runs.
- **A supervisor that would do both** needs VIT− min of at least 2700 mV (the flash) and a rising maximum of at most 3144 − 118.3 = 3025 mV for R2, or 3031 mV for lever (iv) at 1.54 mm. With the drop limit kept at 110, the rising maximum must be at most 3034 mV, and S's 3030 already meets that.
  - So the window is VIT− from 2.70 V up, with VIT− max plus hysteresis at most about 3.03 V, which is a part about 1 % tighter than the S, or set lower than it.
  - The TLV803 family offers none. The TLV803E is named by TI as pin-to-pin, but its thresholds are not held.
- **No supervisor brings the module (3.0 V) into the rule:** a VIT− min of at least 3000 mV leaves at most 3144 − 3000 − hysteresis − tolerance for the drop, a few tens of mV.

## 5. For reference: the cost of measuring pad 19 (estimate; nothing built)

**Why it is unbounded:**
- Its 2 × 2 array sits at the radio row's 1100 µm pitch (`radio-parts.json`), centred on the pad (`route::arrays`).
- Pad 19 is 2.9 × 2.9 mm at (0.96, 0.2) in the footprint, and the module's origin is at (129.15, 36.7). So the vias land at (129.56 or 130.66, 36.35 or 37.45), off the gate's 62.5 µm grid.

**Two changes are needed:**
1. **The pitch to 1125 µm** (18 × 62.5 µm). The 2 × 2 offsets become ±562.5 µm (9 × 62.5), which is on the grid whenever the array's centre is.
   - Each via moves 12.5 µm per axis from the land pattern's gap crossing (datasheet p. 37, Fig. 11-1: 0.7 mm lands, 0.4 mm gaps).
   - The via's 0.72 mm pad already spans the 0.4 mm gap, and 12.5 µm is a quarter of the fabricator's drill positional tolerance (±0.0508 mm, OSH Park, the gate's own stated condition). Citable as an engine choice: "the crossings, to 12.5 µm".
2. **The array's centre on the grid.**
   - Pad 19's centre cannot land on a 62.5 µm multiple in x under the placer's 50 µm snap (`pcb.rs` `SNAP`), whatever the placement: 50k + 960 ≡ 0 (mod 62.5) in µm has no integer k.
   - **Either** a module-scoped finer snap: a 10 µm snap works with k ≡ 4 (mod 25). This moves the placement code.
   - **Or** the array's sites taken about the grid node nearest the pad's centre, scoped to arrays a radio row declares, so U10's and U11's arrays and every other board keep their bytes.
     - The nodes are at most 31.25 µm off per axis. On this placement: 15 µm in x, 25 µm in y.
     - With change 1, every via is at most 43.75 µm per axis from its crossing: here 27.5 and 37.5 µm, 46.8 µm radially. The worst case is 62 µm radially, which is over the 50.8 µm tolerance. That needs a stated reason.
     - **Estimate:** one emitter-and-checker change in `route::arrays` (the checker derives its sites from the same function), the radio row's pitch, a planted test, and one emit plus `extract` (about 10 min). It moves Sentinel-03 only. Roughly two hours of building.

**What it can buy:** a pad-19 bound of at most 36 mΩ passes lever (iv)'s 1.54 mm board, and at most 33 mΩ the R2 board, against the ruled 110 mV with the TLV803S. The bound itself is not known. The module's VDD pad 1 is bounded at 30.3 mΩ.
