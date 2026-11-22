# AET-WO-1216 — predictions, written before any reader ran

Written 2026-11-22 ~07:05Z at `9cb87b4c` (main), from the work order, `1211e` FEEDBACK F1/F5, and a key-level inventory of the committed specifications (which top-level keys each file has, which voltage domains it declares, whether it has a `power` tree, `pin_devices` or `connectors`). No path reader, `solve` walk or netlist read had run.

## The universe

- **Committed boards:** every file `git ls-files '*.json'` lists with `schema_version` 24 and an `mcu` object, that is not a scenario overlay (`base` + `overrides`). Predicted: **34** files (25 CLI fixtures, 3 IR fixtures, 6 examples).
- **Excluded, named:** spike records (`spike/**`, frozen at older versions, including `spike/1215/in/v24-newer.json`), `docs/work-orders/*.repro` (specification 1), topologies and data tables, and the scenario overlays (they override I2C pulls on `mcu.supply` only).
- A file `validate` or `solve` refuses (expected: `invalid-zero-domain.json` and at least one `solve/` stress fixture) is listed as refused and walked only at the specification level.

## What counts as an off-domain path

A path onto an MCU pin from anything but `mcu.supply` or GND: a pull or divider to another rail or the battery, a pin device's output powered from another rail, a crossing's MCU-side output, a power-tree control wire, a connector pin carrying an MCU net (the far end is off-board), a foreign clock.

## Predictions

1. **Boards with at least one off-domain path: all but a handful.** Every board with a connector pin on an MCU net (console, SWD, telemetry, motor outputs) has one, with its worst voltage **not stated**: no specification bounds what an off-board device drives. Predicted: **at least 30 of 34**; the exceptions are files with no connector on an MCU net, if any.
2. **Paths with a stated source, by board:**
   - `sentinel-03.json` and `sentinel-02-power.json`: **1 each**, the battery sense divider (`power.sense`, VBAT through 100 kΩ / 10 kΩ). Worst at VDD < 1 V: the 6S LiHV maximum (26.1 V) through the divider's tolerance corners, **about 2.4 V**, under 4.0 V. It injects current into an unpowered pin, which Table 11 note 4 calls positive injection.
   - `d1-drv8876.json`: **1**, IPROPI (a current output from VM onto an ADC pin). Worst while the MCU is down: **0 V if the driver is asleep** (nSLEEP is the MCU's pin; the device's internal pull-down holds it low). If the held page states only the 3 mA maximum, the stated bound is 3 mA × R2 = about 4.6 V, above 4.0 V, and that would be the §7 stop, unless the sleep behaviour is stated.
   - `d1` and `d2`'s nFAULT: open drain, pulled to `VDD_3V3` (= `mcu.supply`): **not off-domain**.
   - `sentinel-02-power.json`'s crossing (U13): its MCU side is on `VDD_3V3` = `mcu.supply`, and its B side's LDO is fed from 3V3: **not off-domain**.
   - I2C pull-ups: all on `mcu.supply` (1211e measured 132): **none**.
   - Power-tree control wires on an MCU net: **none** (every EN strapped to its input, every PG open).
   - A foreign clock: `hse-external-8.json` and `examples/sentinel-02-bench-hse8.json`, PH0 from the ST-LINK's MCO, off-board: **not stated**.
3. **No committed board exceeds 4.0 V on an unpowered MCU pin by a stated path.** The one candidate is D1's IPROPI (above).
4. **Regulators:** two LMR33630 bucks from one battery, each EN strapped to VIN and PG open, start in an order **no held page fixes** (each self-enables at its own UVLO and soft-start); the TLV755P is fed from 3V3, so 1V8 cannot lead 3V3. The LMR33630's soft-start time is stated (internal, a few ms); whether a minimum is stated I expect **not**.
5. **The options, refused on the committed boards:**
   - (a) a declared `power.sequence`: refuses **every board with more than one domain and no sequence** (they all lack one): about **10**.
   - (b) the order from EN/PG wiring: refuses the two trees' racing bucks (`sentinel-03`, `sentinel-02-power`), and every multi-domain board with no tree has no wiring to read: about **10** as well, by name.
   - (c) VDD = 0 for every off-domain path: refuses **none** by a stated voltage (2.4 V < 4.0 V), and every connector path is unbounded, so either it refuses about **30** boards for "not stated" or it judges only stated sources and refuses **0 to 1** (D1's IPROPI).
