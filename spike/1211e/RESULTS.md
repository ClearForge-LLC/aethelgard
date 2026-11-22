# AET-WO-1211e — the measurement (§1.1), against PREDICTIONS.md

Run 2026-11-20 at `76e09a95` (main `adb8f61a` and this spike only), by `measure.py` over every file `git ls-files '*.json'` lists with `schema_version` and `mcu` (63; 14 are topologies or data files and are skipped by shape).

## Sources

- **I/O structure:** DS12117 Rev 10 Table 8 (pp. 64–87), legend Table 7 (p. 63: FT 5 V tolerant, TT 3.3 V tolerant, B BOOT0, RST; `_a` VDDA, `_u` VDD33USB).
  - Read twice, independently: `table8.py` from the held PDF's text (`out/table8-text.tsv`, 117 non-supply LQFP144 pins, each matched to one row by the vendored open pin data's name, and each row's LQFP144 column equal to the XML's position), and a visual read of the rendered pages (`out/table8-visual.tsv`, all 144 positions).
  - The two agree on all 117 rows, structure and page. FT 95, TT 12 (PC2_C, PC3_C, PA4, PA5, PA7, PC4, PC5, PB1, PG1, PE7, PE8, PE9), B 1 (BOOT0), RST 1 (NRST).
- **Tolerated input:** DS12117 Rev 10 Table 10 p. 103 (rev Y) = Table 108 p. 207 (rev V), word for word: FT_xxx `Min(Min(VDD, VDDA, VDD33USB, VBAT) + 4.0, 6 V)`; TT_xx 4.0 V; BOOT0 9.0 V; any other pin 4.0 V. Notes p. 104 = p. 208: (1) every main supply connected and in range; (3) the formula on the supplies of the pin's structure; (4) above 4 V the internal pull-up/pull-down off; (5) an FT_a pin used by an analog peripheral, 4 V.
  - VDDA, VDD33USB and VBAT are all on `mcu.supply` (the emitter's tie, `aethelgard-eda` `schematic_support.rs`), so `Min(...)` is `mcu.supply`'s minimum.
  - `measure.py` takes VDD at the domain's **minimum**, where the limit is tightest. At any supply minimum of 2.0 V or more the 6 V cap binds.

## Rows

| prediction | measured | holds |
|---|---|---|
| 1. no declared pull exceeds its pin's tolerance | 128 I2C line pulls, 1 crossing CS pull, 4 pin-device pulls (2 resistor to `VDD_3V3`, 2 MCU pull-up): every one at most 3465 mV against at least 4000 mV | **holds** |
| 2. every hand pin a pull lands on has a held structure; none TT | 8 hand I2C pins (PB6, PB7, PB9, PB10, PB11, PD13 …): all FT_f, FT_fa or FT_fh | **holds** |
| 3. the only reference-sets-trip part is D1's DRV8876; VREF on a domain; trip meets full scale | `d1-drv8876.json`: VREF on `VDD_3V3` (min 3135 mV); at 1900 mA with the 5 % band, IPROPI sources 1995 µA into R2 at most 1555 Ω (1 % + 100 ppm/K × 65 K + 2 % drift): 3.102 V ≤ 3.135 V | **holds** (33 mV of margin) |
| 4. D1 is the one fixture `bulk` refuses; no example carries a pin device | D1 (and its `spike/1211` copy) only | **holds** |

`spike/1211/fixtures/*.json` are 1211's frozen spike records (`schema_version` `"22-or-23"`); no test reads them, and the spike copy of D1 has no `temperature_celsius`.

## Positive control

The same script over `1211d`'s adversarial specifications (`out/measure-1211d-adv.tsv`) flags each case the rules exist for:
- `nfault_to_vmotor` (D2, nFAULT by 10 kΩ to a 6.0 V ±5 % `VMOTOR`): 6300 mV, above every structure;
- `d1_nfault_vmotor`: 12600 mV;
- `d1_vref_gnd`: VREF on `GND`, ITRIP 0;
- `d1_vref_low` (VREF on a 0.5 V ±5 % domain, min 475 mV): IPROPI reaches 3.102 V at full scale; it trips near 305 mA at the typical gain, against 1900 mA declared;
- every D1 variant: no `bulk`.

No committed specification is refused by any rule: **no stop.**

## Addendum, 2026-11-22: the final rule

The rows above judge A1 by Table 10, as the WO first wrote it. Two rulings changed the rule:
- **D-A1** (2026-11-20 19:58Z): Table 13, the operating conditions (p. 105), with a pull to `mcu.supply` exempt.
- **D-A2:** `TripSenseUnbound`.

The same script, with `--table13`, re-ran:
- over the committed files at specification 23 (`out/measure-committed-table13.tsv`);
- over the merged tree at specification 24 (`out/measure-committed-spec24.tsv`, 65 files, 132 I2C pulls);
- over 1211d's boards (`out/measure-1211d-adv-table13.tsv`).

Every committed pull is on `mcu.supply`, so it is exempt. Nothing committed is refused. The positive control still flags each adversarial board.
