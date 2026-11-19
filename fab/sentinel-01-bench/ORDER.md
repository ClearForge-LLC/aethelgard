# Sentinel-01 (`sentinel-01-bench`): what to order

For Scotty, who orders (AET-WO-3401 §2.3). The board is `examples/sentinel-01.json`: V0, plus NRST and BOOT0 on the SWD header, the EXT I2C bus, a test point on every declared net, and a reset switch. The package that every gate judged is `package/` beside this file, written by CI's `bench-package` job. Its hashes are in `package/MANIFEST.json`.

> **EXT is 3.3 V on pin 1, not DS-009 5 V. Plug in only 3.3 V-tolerant I2C devices.** (J4: DS-009 I2C's pinout with `standard: null`; ruled on PR #190, Q3.)

Labels: **[read at source]** from a document or page named here; **[measured]** from this repository's tools; **[computed]**; **[inferred]**.

## 1. The bare board

| | |
|---|---|
| Outline | **46.35 × 54.85 mm**, a rectangle; 25.42 cm², 3.94 in² [measured: the emitted board's `Edge.Cuts`] |
| Layers | **2** (F.Cu, B.Cu), through vias only; one via size, 0.3 mm drill / 0.6 mm pad |
| Thickness | **1.6 mm** nominal (the specification's stackup: 0.34–2.0 mm admitted; FR-4) |
| Copper | 1 oz is what fabs quote by default. The specification's bound is "at most 70 µm" (2 oz), so 1 oz or 2 oz both satisfy it |
| Rules | 0.2 mm track, 0.2 mm clearance (the specification's `board.rules`) |
| Finish | Not specified: the bench needs none in particular. ENIG is OSH Park's only finish |
| Solder mask, silkscreen | Both sides, as the Gerbers carry them |
| **Files to upload** | `package/sentinel-01-bench-F_Cu.gtl`, `-B_Cu.gbl`, `-F_Mask.gts`, `-B_Mask.gbs`, `-F_Paste.gtp`, `-B_Paste.gbp`, `-F_Silkscreen.gto`, `-B_Silkscreen.gbo`, `-Edge_Cuts.gm1` and `sentinel-01-bench.drl`, zipped together. `FAB-NOTES.md` states the stackup the board was judged at |

**Price, one fab read at source:** OSH Park's 2-layer service is "$5 per square inch, per set of 3" (docs.oshpark.com/services, read 2026-11-16). For 3.94 in² that is **US$19.71 per set of 3**, and **US$39.42 for 6** [computed]. A second fab's price needs its interactive quote (§5).

## 2. The BOM (39 lines; the 14 test points are bare pads with no part)

**A line with no maker part chosen is flagged ⚑, not guessed.**
- The emitter writes value and footprint only for generic passives.
- The prices are distributor reads from digikey.sg on 2026-11-16, recorded in `AET-WO-3400.FEEDBACK.md` §2 and §10.3.
  - **SGD** means the page said "All prices are in SGD"; **SGD?** means a `$` price on the Singapore/SGD site.
  - No currency was converted.
  - Search link form: `https://www.digikey.sg/en/products/result?keywords=<MPN>`.

| Reference | Value | Footprint | Maker part | Distributor link, read 2026-11-16 | q1 |
|---|---|---|---|---|---|
| U1 | STM32H753ZITx | LQFP-144 20×20 mm, 0.5 mm | **STM32H753ZIT6** (ST) | digikey.sg `STM32H753ZIT6` | SGD? 29.15 (one tier) |
| U2 | BME280 | Bosch LGA-8 2.5×2.5 mm | **BME280** (Bosch Sensortec). **Not fitted on the boards ordered 2026-11-19:** see below | digikey.sg `BME280` | SGD? 5.97 |
| U3 | TMP117xxDRV | WSON-6 2×2 mm (DRV) | **TMP117AIDRVR** (TI) | digikey.sg product 9685284 | SGD 8.63 (q10 7.442) |
| U4 | W25Q64JVSSIQ | SOIC-8 5.3 mm (208 mil) | **W25Q64JVSSIQ** (Winbond; NRND, accepted in the specification) | digikey.sg `W25Q64JVSSIQ` | SGD? 6.01 |
| J1, J3, J4 | SM04B-GHS-TB | JST GH 4-pin, horizontal SMT | **SM04B-GHS-TB** (JST) | digikey.sg `SM04B-GHS-TB` | SGD 0.53 each (q10 0.453) |
| J2 | SM06B-SRSS-TB | JST SH 6-pin, horizontal SMT | **SM06B-SRSS-TB** (JST) | digikey.sg `SM06B-SRSS-TB` | SGD? 0.93 |
| SW1 | PTS810 SJM 250 SMTR LFS | SW_SPST_PTS810 | **PTS810SJM250SMTRLFS** (C&K), the part `1312` cites | digikey.sg `PTS810 SJM 250 SMTR LFS` | SGD? 0.80 |
| C1–C11, C13, C15, C19–C22 (17) | 100n | 0402 | **CC0402KRX7R7BB104** (Yageo, X7R 16 V), chosen 2026-11-17 | digikey.com `311-1338-1-ND`, read 2026-11-17: 339,251 in stock | USD 0.10 (q10 0.041) |
| C12 | 4u7 | 0402 | **GRM155R60J475ME47D** (Murata, X5R 6.3 V), chosen 2026-11-17 | digikey.com `490-5915-1-ND`, read 2026-11-17: 14,602 in stock | USD 0.10 |
| C14, C16 | 1u | 0402 | **C1005X5R1V105K050BC** (TDK, X5R 35 V), chosen 2026-11-17 (the earlier example is obsolete) | digikey.com `445-9073-1-ND`, read 2026-11-17: 254,429 in stock | USD 0.14 |
| C17, C18 | 2u2 | 0402 | **CL05A225MP5NSNC** (Samsung, X5R 10 V), chosen 2026-11-17 | digikey.com `1276-1467-1-ND`, read 2026-11-17: 713,085 in stock | USD 0.17 |
| R1, R2 | 2k2 | 0402 | **RMCF0402FT2K20** (Stackpole, 1 %), chosen 2026-11-17 (1 %: BENCH B6's pass line assumes it) | digikey.com `RMCF0402FT2K20CT-ND`, read 2026-11-17: 2,643,214 in stock | USD 0.10 |
| R3–R8 | 10k | 0402 | **RC0402FR-0710KL** (Yageo, 1 %), chosen 2026-11-17. R8 is BOOT0's judged pull-down (`1312`) | digikey.com `311-10.0KLRCT-ND`, read 2026-11-17: 4,139,753 in stock | USD 0.10 |

**Each maker part matches the footprint the emitter wrote** (the adversarial check, §5 of the work order) [read at source; the package suffixes]:
- `STM32H753ZI**T**6`: the T is LQFP;
- TMP117AI**DRV**R: DRV is WSON-6 2×2;
- W25Q64JV**SS**IQ: SS is SOIC-8 208 mil;
- BME280: LGA-8 2.5×2.5;
- the JST and C&K parts are named in their footprints.

The flash is drawn with KiCad's `W25Q32JVSS` symbol (same SOIC-8 pinout), as its BOM note says.

**Priced lines total, per board, DigiKey US, 2026-11-17: USD 40.92 for all 39 parts** (DigiKey's BOM Manager on this list; the passives at the quantities above). Not a quote: shipping and tax are excluded, and the 100 n and 10 k lines are cheaper in total at the next cut-tape break.

**Out of stock on 2026-11-17** at DigiKey US: BME280 (no date; also 0 at Mouser, Arrow, Newark and LCSC), TMP117AIDRVR (due 15 Dec 2026; LCSC had stock) and SM04B-GHS-TB (past due; LCSC had stock). The BME280's supply is a roadmap §7.1 row.

**U2 is not fitted on the boards Scotty ordered on 2026-11-19** (`AET-WO-3403`; the post-order review, `docs/reviews/2026-11-19-sentinel-01-as-ordered.md`, U1 and U2):
- **Why:** the BME280 is out of stock everywhere, and the BMP581 Scotty chose instead (#237) is an LGA-10 of 2.0 × 2.0 mm, which does not land on U2's LGA-8 2.5 × 2.5 mm pads. No pad-compatible part has been verified.
- **What it costs:** no copper. The ordered firmware reads U2 once, gets a NACK, records it and boots on (`BENCH.md`, "U2 empty"). Only the silicon witness of the firmware's own I2C read is lost; U3 (the TMP117) witnesses I2C1 instead (`BENCH.md` B9).
- R5 is U2's address strap (SDO to GND). Fitted or not, it changes nothing with U2 empty.
- If any board is assembled by the fabricator, mark U2 "do not place": `package/sentinel-01-bench-jlcpcb-bom.csv` still lists it. The parts totals in §2 and §4 still include U2.
- A BMP581 breakout can stand in on J4 (EXT) with a bench ELF and a hand-pinned cable: `BENCH-EXT-BMP581.md`.

## 3. Assembly

**Files.**
- The package carries KiCad's placement file (`package/sentinel-01-bench-pos.csv`: reference, value, package, X, Y, rotation, side, in mm) and the emitter's BOM (`package/sentinel-01-bench.bom.csv`: reference, value, footprint, symbol, note).
- **JLCPCB's formats are in the package too** (since `AET-WO-1604`, #199): `package/sentinel-01-bench-jlcpcb-bom.csv` (Comment, Designator, Footprint, JLCPCB part number left blank) and `package/sentinel-01-bench-jlcpcb-cpl.csv` (Designator, Mid X, Mid Y, Layer, Rotation).
  - The part-number column is blank by design: the fabricator's own catalogue match is the orderer's choice.
  - Each part's rotation offset is the fabricator's own convention, so the first assembled order needs its placement preview checked by eye (roadmap §7.1, `1604` O1).

**Hand-solder-friendliness, line by line** [inferred from the packages]:

| Part | By hand |
|---|---|
| U4 SOIC-8; J1–J4 JST; SW1 | **Easy** with an iron |
| 0402 passives (30 of the 39 parts) | Possible with tweezers, flux and a fine tip; slow |
| U1 LQFP-144, 0.5 mm pitch | **Hard** but routine: drag-solder with flux, then wick bridges |
| U3 WSON-6, exposed pad; U2 LGA-8 | **Not by iron.** Hot air or a hot plate with paste: the pads are under the part |

**The plan** (for Scotty to confirm):
- Order the bare boards in one batch.
- Have **2 boards assembled by the fabricator as the control**, so a bench fault on a hand-built board can be told apart from a soldering fault.
- **Scotty builds the rest:** U3 by hot air or hot plate, the rest by iron. U2 is not fitted (§2).
- A stencil is worth ordering with the boards if paste is used: the package carries both paste layers.

## 4. What it costs, as far as it is sourced

| Item | Figure | Source |
|---|---|---|
| Bare boards, OSH Park | US$19.71 per 3, US$39.42 per 6 | [computed] from the rate read 2026-11-16 |
| Assembly, JLCPCB Economic (for the 2 control boards) | $8.18 setup + $1.53 stencil per order; 259 SMT pads × $0.0016 = $0.41 per board [computed: pads on BOM parts in the emitted board]; $3.07 per extended part type (at most 13 unique types; which are extended is unknown) | jlcpcb.com/help/article/pcb-assembly-price, read by `3400` 2026-11-16 |
| Parts, per board | SGD 54.64? for 33 of 39 rows, then six rows unpriced | §2 |

USD and SGD are not summed.

## 5. Open quotes (pages Furnace could not use; `AET-WO-3400.FEEDBACK.md` §10.5, the rows this board needs)

1. **A second fab's bare-board price** for 46.35 × 54.85 mm, 2 layers, 1.6 mm, at 5 and 10 boards: JLCPCB https://cart.jlcpcb.com/quote, or PCBWay https://www.pcbway.com/orderonline.aspx.
2. **JLCPCB assembly:** the basic or extended class of each line (https://jlcpcb.com/parts), and Economic PCBA's limits.
3. **The ⚑ lines: closed 2026-11-17.** Live parts are chosen in §2, each read in stock at DigiKey US.
4. **ST AN2606** (the system bootloader's USART pins and protocol), which `BENCH.md` B8 needs. Three downloads timed out.
5. **Lifecycles behind sign-in:**
   - Winbond's own page for W25Q64JVSSIQ: NRND, from Scotty's catalogue export of 2026-11-16, already accepted;
   - ST's for STM32H753ZIT6.
