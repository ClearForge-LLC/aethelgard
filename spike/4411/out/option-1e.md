# AET-WO-4411 F2, lever (e): U13 set higher, on paper. STOP: no divider fits

Ruled 2026-11-26T03:22Z: F2 is real and (d) is refused. This record is (e) on paper, from held documents. Every band below is the engine's own solved output (`aethelgard solve`): the LMR33620's VFB 0.985 to 1.015 V (SNVSAW1E p. 6, 7.5), its regulation -1.5 / +2.5 % (p. 8, 7.7, not production tested), and the TNPW resistors' tolerance and life drift. `VDD_3V3_RF`'s declared domain was moved to 3420 mV +/- 5 % (3249 to 3591 mV) so it contains each raised band, inside the module's 3.0 to 3.6 V.

## The two edges at U30's RXD pin

- **VIH maximum, VDD + 0.3 V** (WROOM-02 v1.7 p. 21, Table 6-3; note 1: VDD at the power pin).
  - U1's high is at most U11's maximum, 3,417,204 uV.
  - U30's VDD at its pin is at least U13's minimum less the rail's 144 mV drop limit.
  - Needs: U13 minimum >= 3,417.2 - 300 + 144 = **3,261.2 mV**.
- **VIH minimum, 0.75 x VDD** (the same Table 6-3).
  - U1's high is at least VDD - 0.4 V (DS12117 rev 10 p. 139, Table 50, CMOS port, IIO -8 mA; the datasheet states no lighter load). Its VDD at its pin is at least U11's minimum, 3,144,727 uV, less `VDD_3V3`'s 110 mV drop limit. So it is at least 2,634.7 mV.
  - U30's VDD is at most U13's maximum (no drop).
  - Needs: 0.75 x U13 maximum <= 2,634.7, so U13 maximum <= **3,512.9 mV**.
- **Together:** U13's band must be at most 3,512.9 - 3,261.2 = **251.7 mV** wide. At these dividers it is about **285 mV** (the table). **No divider fits.** A regulator with a band about 12 % narrower would.
- **Today's design** (U13 = U11's divider, 105k / 46.4k) passes VIH minimum by 71.8 mV and fails VIH maximum by 116.6 mV.

## The candidates (E96, TNPW0603 0.1 %)

| R30 / R31 | U13 band (uV) | VIH max at pin: margin | VDD max 3.6 V (Table 6-2): margin | VIH min at pin: margin |
|---|---|---|---|---|
| 105k / 46.4k (today) | 3,144,727 - 3,417,204 | **-116.6 mV** | 182.8 mV | 71.8 mV |
| 113k / 46.4k | 3,310,403 - 3,598,296 | 49.2 mV | 1.7 mV (outside 3420 +/- 5 %) | -64.0 mV |
| 110k / 45.3k | 3,303,723 - 3,590,713 | 42.5 mV | 9.3 mV | -58.3 mV |
| 118k / 48.7k | 3,298,263 - 3,585,614 | 37.1 mV | 14.4 mV | -54.5 mV |
| 107k / 44.2k | 3,296,704 - 3,582,760 | 35.5 mV | 17.2 mV | -52.4 mV |
| 115k / 47.5k | 3,296,538 - 3,583,421 | 35.3 mV | 16.6 mV | -52.9 mV |
| 102k / 42.2k | 3,293,336 - 3,578,570 | 32.1 mV | 21.4 mV | **-49.2 mV** |

Each raised divider trades the VIH-maximum fault for a VIH-minimum one: U1's guaranteed high is no longer a guaranteed high at the module.

## The rest of the ruling's list, at the best balance (102k / 42.2k)

- **The reverse path:** U30's TX onto U1's `WIFI_RX` (PG9, FT_h).
  - Powered, FT_h takes up to VDD + 4.0 V (DS12117 p. 103, Table 10).
  - At VDD = 0 (step 25), the bound is the domain's declared maximum, 3,591 mV, under Table 10's 4.0 V. It passes by 409 mV.
- **Other parts on `VDD_3V3_RF`:**
  - R30/R31, R32 (U13's PG pull-up, under PG's 22 V absolute maximum), R90/R91 (U30's own pulls), C90, C91, C92.
  - C34-C37 (rated 10 V; the declared derating holds to 347 per mille of rated).
  - Each is rated above 3.6 V. The module's own pins are judged above.
- **The ladder and the rung:** a divider changes two resistors' values, not a footprint or a net. The placement, and so the routing, is the same. Only the value text in the board and the BOM moves. Not emitted, because (e) fails.

## So: (c), a buffer with Ioff, the ruling's fallback. Not built

What a buffer has to do:
- It is powered from `VDD_3V3_RF` and sits between U1's TX and U30's RXD.
- The module then sees only its own rail's levels.
- With Ioff, its output is high-impedance while `VDD_3V3_RF` is down, which also covers the power-down row.

Its own levels need checking at build. U1's high into it: at least 2,634.7 mV, against a gate's VIH of typically 0.7 x VCC at most 3,578.6 mV, about 2,505 mV.

Two paths, each a build:
- **(c1) The held translator, TXU0304PW** (SCES935A, sha256 in `power-parts.json`; Ioff and Ioff-float, "fully specified for partial-power-down applications", p. 1 and 9.3.4).
  - A to B on U1's TX, B to A on its RX: three A-to-B channels and one B-to-A. VCCA = `VDD_3V3`, VCCB = `VDD_3V3_RF`.
  - Its library row holds only the VCCB = 1.8 V rows. The VCCB = 3.3 V rows are in the same held datasheet (p. 14, 7.11), so they would be added from it.
  - The engine models it as an SPI crossing only. A UART crossing is a new kind.
- **(c2) A single gate with Ioff** (for example a one-gate LVC buffer), as the ruling describes it. **None is held:** a datasheet to fetch and hash, a new row, and a vendored symbol and footprint.

**I recommend (c1).** Its datasheet is already held and it is already a library part: one new row block and one new crossing kind. Its spare channels are left unused.
