# S4 BOM per board, line by line (AET-WO-3400 §10.3)

**Conventions**
- Prices: digikey.sg, read 2026-11-16. URL form: `https://www.digikey.sg/en/products/result?keywords=<MPN>` unless another URL is given.
- Currency as the page showed it:
  - **SGD:** the page said "All prices are in SGD".
  - **SGD?:** `$` with the page region set to Singapore/SGD.
- Price used: q1, or q10 where the board uses ten or more of the part. Line total = quantity × that price.
- **The emitted BOM gives value and footprint only for generic passives.** An MPN marked *(spike's pick)* is a representative part chosen to price the line. It is not the BOM's.
- Sources:
  - R-B: `research-B.md`;
  - R-S4: `research-S4.md`;
  - me: `digikey-sg-reads.md`.

## (a) V0, the lean probe (34 rows, 12 unique part types)

| Qty | Emitted value / footprint | Part priced | Unit | Currency | Line | Status (as shown) | Source |
|---|---|---|---|---|---|---|---|
| 1 | STM32H753ZITx / LQFP-144 | STM32H753ZIT6 | 29.15 (one tier) | SGD? | 29.15 | not shown | R-B; re-read by me 2026-11-16 |
| 1 | BME280 / LGA-8 | BME280 | 5.97 | SGD? | 5.97 | maker states none | R-B |
| 1 | TMP117xxDRV / WSON-6 | TMP117AIDRVR | 8.63 | SGD | 8.63 | Active | R-B; re-read by me (product page) |
| 1 | W25Q64JVSSIQ / SOIC-8 | W25Q64JVSSIQ | 6.01 | SGD? | 6.01 | NRND by Winbond (4211, Scotty's evidence) | R-B |
| 2 | SM04B-GHS-TB | SM04B-GHS-TB | 0.53 | SGD | 1.06 | Active | R-B |
| 1 | SM06B-SRSS-TB | SM06B-SRSS-TB | 0.93 | SGD? | 0.93 | not stated | R-B |
| 17 | 100n / C_0402 | GRM155R71C104KA88D *(spike's pick)* | 0.034 (q10) | SGD | 0.58 | Active, 0 stock | R-S4 |
| 3 | 10k / R_0402 | RC0402FR-0710KL *(spike's pick)* | 0.14 | SGD | 0.42 | Active | R-S4 |
| 1 | 4u7 / C_0402 | GRM155R60J475ME47D *(spike's pick)* | 0.14 | SGD | 0.14 | Active | R-S4 |
| 2 | 1u / C_0402 | GRM155R61A105KE15D *(spike's pick)* | — | — | unknown | Obsolete | R-S4 |
| 2 | 2u2 / C_0402 | GRM155R61A225KE15D *(spike's pick)* | — | — | unknown | no result | R-S4 |
| 2 | 2k2 / R_0402 | not searched | — | — | unknown | — | — |

**Priced: SGD 52.89 for 28 of 34 rows** (mixed SGD and SGD?); 6 rows unknown.

## (b) and (c) V6, the loaded board at 2L and 4L, identical BOM (71 rows, 29 unique part types)

Every line of (a), with 10k at quantity 5 (0.70) instead of 3, plus:

| Qty | Emitted value / footprint | Part priced | Unit | Currency | Line | Status | Source |
|---|---|---|---|---|---|---|---|
| 2 | LMR33630ADDA / HSOP-8 | LMR33630ADDAR | 3.96 | SGD | 7.92 | out of stock, 26 wk | R-B |
| 2 | 8u2 IHLP5050FDER8R2M01 | IHLP5050FDER8R2M01 | 2.92 | SGD | 5.84 | Active (distributor) | R-B |
| 2 | 10u GCM32EC71H106KA03 / 1210 | GCM32EC71H106KA03K | 1.37 | SGD? | 2.74 | Active | R-B |
| 1 | TLV75518PDBV / SOT-23-5 | TLV75518PDBVR | 0.58 | SGD | 0.58 | Active | R-B |
| 1 (3 total) | SM04B-GHS-TB | SM04B-GHS-TB | 0.53 | SGD | 0.53 | Active | R-B |
| 3 | SM06B-GHS-TB | SM06B-GHS-TB | 0.67 | SGD | 2.01 | Active | R-B |
| 4 | 22u GRM32ER61C226KE20 / 1210 | GRM32ER61C226KE20L | 2.58 | SGD | 10.32 | **Not For New Designs** | R-S4 |
| 4 | 22u GRM32ER71A226KE20 / 1210 | GRM32ER71A226KE20L | 2.87 | SGD | 11.48 | **Not For New Designs** | R-S4 |
| 3 | 100n GCM188R71H104KA57 / 0603 | GCM188R71H104KA57D | 0.27 | SGD | 0.81 | Active, 0 stock | R-S4 |
| 2 | 220n GCM188R71H224KA64 / 0603 | GCM188R71H224KA64D | 0.23 | SGD | 0.46 | Active | R-S4 |
| 2 (5 total) | 10k / R_0402 | RC0402FR-0710KL *(spike's pick)* | 0.14 | SGD | 0.28 | Active | R-S4 |
| 4 | 1u GRM188R71C105KA12 / 0603 | GRM188R71C105KA12D | — | — | unknown | **Obsolete** | R-S4 |
| 4 | 102k, 25k5, 105k, 46k4 TNPW0603_B_E | TNPW0603 0.1 % at those values | — | — | unknown | not searched (only 10k2 was: 0.36/0.30 SGD) | R-S4 |
| 2 | 100k, 10k CRCW0603_F_K | CRCW0603 | — | — | unknown | price shown without a quantity | R-S4 |
| 1 | XT30PW-M | XT30PW-M | — | — | unknown | not on digikey.sg | R-B, R-S4 |

**Priced: SGD 95.86 for 54 of 71 rows;** 17 rows unknown.

**Not on V6, so (b)/(c) is a floor.** Each of these would add to the BOM:
- the hot-plug capacitor E0 would draw: UCZ1H470MCL1GS, SGD 1.92 (R-B);
- `4220b`'s ICM-42688-P (SGD? 6.67) and BMP581 (SGD? 4.87) (R-B);
- E1's test pads: no part;
- E2's PTS810 switch: SGD? 0.80 (me);
- the witness add-on: 2 × SN74LXC8T245PWR 4.82 (SGD, me); 2 × TPD8E003DQDR 4.94 (SGD, research-W-parts); a 2 × 10 header, SGD? 1.82; USB4105-GF-A, SGD? 1.10; USBLC6-2SC6, SGD? 0.74.
