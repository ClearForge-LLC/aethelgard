# AET-WO-1211e — predictions, written before the measurement runs

Committed 2026-11-20T19:13Z, before `measure.py` was run on any file.

1. **A1.** Across every committed specification and test fixture (63 files carrying `schema_version` and `mcu`, by `git ls-files`), no declared pull on a net that reaches an MCU pin exceeds that pin's tolerated input.
   - Every I2C bus pull and every crossing's CS pull is on `mcu.supply` (step 5a), so at most 3.6 V, under every structure's limit (TT 4.0 V, FT 6 V at a 3.3 V supply).
   - The only pin-device pulls are in the two IR fixtures (`d1-drv8876.json`, `d2-drv8833.json`) and their spike copies; each is to the MCU's supply or the MCU's own pull-up.
2. **Structures.** Every MCU pin a pull lands on by hand has a held structure in DS12117 Rev 10 Table 8; none is TT.
3. **A2.** The only reference-sets-trip part is the DRV8876, in `d1-drv8876.json` (and its spike copy); its VREF is tied to a domain (not `GND`), and VREF_min / (A_IPROPI x R_IPROPI,max) meets the declared full scale at the error band's worst edge.
4. **bulk.** `d1-drv8876.json` declares no `bulk`, so it is the one fixture the new rule refuses as it stands; the WO says its fixture gains one (a stand-in). No committed specification under `examples/` carries a pin device, so none moves but by its version line.
