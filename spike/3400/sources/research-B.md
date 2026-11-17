# AET-WO-3400 research B (BOM, KiCad libs, fab cost), read 2026-11-16

Method note: pages were read with a fetch tool that summarises each page (not raw HTML), so figures are as reported by that reader.
Distributor pricing: LCSC, Mouser, DigiKey.com and TME product pages either load prices by JavaScript (no price in the text) or returned HTTP 403 / timeout. The only price tables I could read were DigiKey Singapore (digikey.sg, **SGD, not USD**) and one digikey.cn page (CNY). No currency conversion was done (that would be an estimate). Where a digikey.sg search-result page showed "$" with no currency label, it is marked "SGD? (unlabeled; site region SGD)". No USD price at q1/q10 was obtainable from any page.
Lifecycle: maker pages that were readable are cited. Where the maker page did not state a status for the exact orderable, that is said, and DigiKey's "Part Status" is given as a distributor statement only.

## Goal 1: BOM

| Part | Maker | Lifecycle (maker, URL) | Price q1 / q10 (distributor, URL, 2026-11-16) |
|---|---|---|---|
| STM32H753ZIT6 (LQFP-144) | STMicroelectronics | Not read: st.com page timed out (https://www.st.com/en/microcontrollers-microprocessors/stm32h753zi.html). Distributor only: DigiKey CN says "Active" (https://digikey.cn/en/products/detail/stmicroelectronics/STM32H753ZIT6/8257856) | DigiKey CN: CNY 175.74 / CNY 139.315 incl. 13% VAT (that URL). DigiKey SG: 29.15 / not shown, SGD? (unlabeled) (https://www.digikey.sg/en/products/result?keywords=STM32H753ZIT6). LCSC C730207 https://www.lcsc.com/product-detail/C730207.html: unknown (page has no price in text) |
| BME280 | Bosch Sensortec | Maker page states no lifecycle; shows 0 stock at all listed distributors (https://www.bosch-sensortec.com/products/environmental-sensors/humidity-sensors-bme280/). Exact orderable code not stated there | DigiKey SG 5.97 / not shown, SGD? (unlabeled) (https://www.digikey.sg/en/products/result?keywords=BME280). Orderable MPN shown there is just "BME280" (no suffix); the bare "BME280" listing is not a verified orderable code [inferred] |
| TMP117AIDRVR (WSON-6) | Texas Instruments | TMP117 "ACTIVE" on TI page; TMP117AIDRVR itself not listed there (https://www.ti.com/product/TMP117). DigiKey SG "Active" (distributor) | DigiKey SG (cut tape): SGD 8.63 / SGD 7.442; stock 0, 3,000 expected 15-Dec-2026 (https://www.digikey.sg/en/products/detail/texas-instruments/TMP117AIDRVR/9685284) |
| W25Q64JVSSIQ (SOIC-8) | Winbond | **Unknown: not stated.** winbond.com parts catalogue is a sign-in shell, no status readable (https://www.winbond.com/hq/product/code-storage-flash-memory/serial-nor-flash/?__locale=en&partNo=W25Q64JV). The NRND claim in the brief is NOT verified. DigiKey SG listing states no status | DigiKey SG 6.01 / not shown, SGD? (unlabeled) (https://www.digikey.sg/en/products/result?keywords=W25Q64JVSSIQ) |
| LMR33630ADDAR (HSOP-8 / HSOIC DDA) | Texas Instruments | LMR33630 "ACTIVE" on TI page; orderable not listed there (https://www.ti.com/product/LMR33630). DigiKey SG: out of stock, backorder, 26-week lead time | DigiKey SG: SGD 3.96 / SGD 2.956 (https://www.digikey.sg/en/products/result?keywords=LMR33630ADDAR) |
| IHLP5050FDER8R2M01 (8.2 uH) | Vishay Dale | Not read on vishay.com (the product page I tried, /product/34320/, was a different part, IHLP-3232CZ-01). DigiKey SG "Active", 256 in stock (distributor) | DigiKey SG: SGD 2.92 / SGD 2.403 (https://www.digikey.sg/en/products/result?keywords=IHLP5050FDER8R2M01) |
| XT30PW-M | AMASS | Unknown (no maker page read) | unknown: DigiKey SG has no listing (https://www.digikey.sg/en/products/result?keywords=XT30PW-M); LCSC/JLCPCB not read |
| UCZ1H470MCL1GS (47 uF 50 V) | Nichicon | Not read from nichicon.co.jp. DigiKey SG "Active", 776 in stock (distributor) | DigiKey SG: SGD 1.92 / SGD 1.233 (https://www.digikey.sg/en/products/result?keywords=UCZ1H470MCL1GS) |
| GCM32EC71H106KA03 (10 uF 50 V 1210) | Murata | Not read from murata.com. The orderable needs a packaging suffix: DigiKey SG lists GCM32EC71H106KA03K (Active, 82 in stock) and GCM32EC71H106KA03L (Active, 0 stock) | KA03K cut tape: 1.37 / no q10 break listed, SGD? (unlabeled). KA03L cut tape: 1.29 / no q10 break. (https://www.digikey.sg/en/products/result?keywords=GCM32EC71H106KA03) |
| TLV75518PDBVR (LDO) | Texas Instruments | TLV755P family "ACTIVE" on TI; this orderable not listed (https://www.ti.com/product/TLV755P). DigiKey SG "Active", 445 in stock | DigiKey SG: SGD 0.58 / SGD 0.402 (https://www.digikey.sg/en/products/result?keywords=TLV75518PDBVR) |
| TXU0304PWR (translator) | Texas Instruments | TXU0304 "ACTIVE" on TI; this orderable not listed (https://www.ti.com/product/TXU0304). DigiKey SG "Active" | DigiKey SG: SGD 1.36 / SGD 0.974 (https://www.digikey.sg/en/products/result?keywords=TXU0304PWR) |
| ICM-42688-P | TDK InvenSense | "Production" (https://invensense.tdk.com/products/motion-tracking/6-axis/icm-42688-p/) | DigiKey SG 6.67 / not shown, SGD? (unlabeled) (https://www.digikey.sg/en/products/result?keywords=ICM-42688-P) |
| BMP581 | Bosch Sensortec | Maker page states no lifecycle; 0 stock at all listed distributors; no ordering code on page (https://www.bosch-sensortec.com/products/environmental-sensors/pressure-sensors/bmp581/). DigiKey describes it "SENSOR 18.13PSIA 10LGA" | DigiKey SG 4.87 / not shown, SGD? (unlabeled) (https://www.digikey.sg/en/products/result?keywords=BMP581) |
| SM04B-GHS-TB | JST | Not read from jst-mfg.com (page tried was the ZH series). DigiKey SG "Active" (distributor) | DigiKey SG: SGD 0.53 / SGD 0.453 (https://www.digikey.sg/en/products/result?keywords=SM04B-GHS-TB) |
| SM06B-GHS-TB | JST | as above. DigiKey SG "Active", 84,567 in stock | DigiKey SG: SGD 0.67 / SGD 0.571 (https://www.digikey.sg/en/products/result?keywords=SM06B-GHS-TB) |
| SM10B-GHS-TB | JST | as above. DigiKey SG "Active" | DigiKey SG: SGD 0.81 / SGD 0.687 (https://www.digikey.sg/en/products/result?keywords=SM10B-GHS-TB) |
| SM06B-SRSS-TB (SH 1.0) | JST | Not read from maker. DigiKey SG status not stated | DigiKey SG 0.93 / not shown, SGD? (unlabeled) (https://www.digikey.sg/en/products/result?keywords=SM06B-SRSS-TB) |

Sum of q1 prices (DigiKey SG, mixed explicit-SGD and unlabeled-but-SGD-region values, GCM32 taken as KA03K 1.37, XT30PW-M excluded because unpriced, JST four connectors counted once each): **SGD 76.35** (STM32 29.15, BME280 5.97, TMP117 8.63, W25Q64 6.01, LMR33630 3.96, IHLP 2.92, UCZ 1.92, GCM32 1.37, TLV755 0.58, TXU0304 1.36, ICM 6.67, BMP581 4.87, JST 0.53+0.67+0.81+0.93). Not USD; not a complete set (XT30PW-M missing).

## Goal 2: KiCad official libraries, tag 10.0.7

Listings read via the GitLab API (tag 10.0.7):
- Symbols, Sensor_Motion.kicad_symdir: https://gitlab.com/api/v4/projects/kicad%2Flibraries%2Fkicad-symbols/repository/tree?ref=10.0.7&path=Sensor_Motion.kicad_symdir&per_page=100 . 35 entries; contains ICM-20602, ICM-20948, IIM-42652, no ICM-42688-P. **No ICM-42688-P symbol there.** (Other libraries not searched.)
- Symbols, Sensor_Pressure.kicad_symdir: https://gitlab.com/api/v4/projects/kicad%2Flibraries%2Fkicad-symbols/repository/tree?ref=10.0.7&path=Sensor_Pressure.kicad_symdir&per_page=100 . 27 entries, includes BMP280, no BMP581. **No BMP581 symbol there.**
- Footprints, Package_LGA.pretty: https://gitlab.com/api/v4/projects/kicad%2Flibraries%2Fkicad-footprints/repository/tree?ref=10.0.7&path=Package_LGA.pretty&per_page=100 . 42 entries. Footprints with the ICM-42688-P package geometry (14-pin LGA, 3 x 2.5 mm, 0.5 mm pitch) exist: `Bosch_LGA-14_3x2.5mm_P0.5mm` and `LGA-14_3x2.5mm_P0.5mm_LayoutBorder3x4y`. That they match the ICM-42688-P land pattern is [inferred] from name and size, not checked against the TDK datasheet. For the 10-pin 2x2 BMP581 package, nearest names are `ST_HLGA-10_2x2mm_P0.5mm_LayoutBorder3x2y` and `Rohm_MLGA010V020A_LGA-10_2x2mm_P0.45mm_LayoutBorder2x3y`; neither is claimed to match BMP581 [inferred, not verified]. No footprint is named for either part. Result summarised by the fetch reader from the JSON; a full-name list could be re-checked.
- Licence: CC BY-SA 4.0 with an exception that, to the extent designs using the libraries are "Adapted Material", the copyright holder waives article 3 for those designs and generated files. The word "design exception" is not used in the file. URLs: https://gitlab.com/kicad/libraries/kicad-symbols/-/raw/10.0.7/LICENSE.md and https://gitlab.com/kicad/libraries/kicad-footprints/-/raw/10.0.7/LICENSE.md . The "10.0 branch" was not checked.

## Goal 3: fabrication

| Item | Figure | URL |
|---|---|---|
| JLCPCB 2-layer, 5 pcs, 60x50 / 80x60 / 100x80 mm | unknown (needs the interactive quote). Only published: "From $2.00 / 5 pcs" (1-32 layers), size and finish for that price not stated on page | https://jlcpcb.com/ (quote page /pcb-quote and /pricing returned 404) |
| OSH Park 2-layer | $5.00 per square inch for 3 copies (Prototype service, 1.6 mm, ENIG, purple mask). Areas: 60x50 mm = 4.65 in2 -> $23.25; 80x60 mm = 7.44 in2 -> $37.20; 100x80 mm = 12.40 in2 -> $62.00 [computed from the page rate; board outline only, no panel waste] | https://oshpark.com/ |
| PCBWay | Home page: "$5.00 for 10 pcs, 1-2 layer", pre-filled 100x100 mm; this is a "from" price, no per-size table visible. Per-size: unknown (needs the interactive quote) | https://www.pcbway.com/ |
| JLCPCB PCBA setup | Economic $8.18; Standard $25.56 single-side, $51.12 double-side | https://jlcpcb.com/help/article/pcb-assembly-price |
| JLCPCB stencil | Economic $1.53; Standard $8.21 single / $16.42 double (not shipped with order) | same |
| JLCPCB "extended part" fee | Economic: $3.07 extended feeder setup (per page text; per-part vs per-order not stated clearly). Standard: $1.53 per order for basic or extended feeder loading. Wording is as read by the fetch reader; confirm against the live page | same |
| JLCPCB per-joint | Pre-reflow soldering $0.016/joint. SMT $0.0016/joint (1-100,000 Economic; 1-50,000 Standard, $0.0013 to 100,000, $0.0012 beyond). Manual assembly $0.0164/joint (1-10,000), $0.015 (10,001-30,000), $0.012 (30,001-100,000) | same |

## Could not source
- Any USD price at q1 or q10 for any part (LCSC/Mouser/DigiKey.com/TME pages blocked, 403, timed out or price loaded by script). All prices above are SGD (digikey.sg) except the one CNY page. q10 missing for STM32H753ZIT6 (SG), BME280, W25Q64JVSSIQ, ICM-42688-P, BMP581, SM06B-SRSS-TB, GCM32EC71H106KA03K/L.
- All prices for AMASS XT30PW-M, and its maker lifecycle.
- Maker-page lifecycle for: STM32H753ZIT6 (st.com timed out), W25Q64JVSSIQ (winbond.com needs sign-in; the NRND claim is unverified), IHLP5050FDER8R2M01, UCZ1H470MCL1GS, GCM32EC71H106KA03, JST parts, XT30PW-M, and the exact TI orderables (TI pages gave family status only).
- Bosch BME280 and BMP581: maker pages give no lifecycle statement and show 0 distributor stock.
- JLCPCB and PCBWay per-size 2-layer board prices (interactive quote only).
- A direct attempt to read distributor pages with curl and a browser User-Agent was denied by the sandbox classifier and was not retried; no login or account was used.
