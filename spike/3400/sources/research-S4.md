# AET-WO-3400 research S4 (all figures dated 2026-11-16)
Labels: [read at source] quoted; [computed] arithmetic on a quoted rate.

## Part 1 - Fabrication
### OSH Park (https://oshpark.com/ , https://docs.oshpark.com/services/)
- [read] 2-layer Prototype "$5 per square inch, per set of 3", 63 mil (1.6 mm), ENIG, 9-12 days.
- [read] 4-layer Prototype "$10 per square inch, per set of 3", 63 mil (1.6 mm), ENIG, 9-14 days.
- [read] Medium Run 2L $1/in2 (100 in2 min, multiple of 10); 4L $2/in2 (100 in2 min, "Must be in multiple of 3"). Not applicable at these sizes.
- Copper weight not stated on the fetched pages. Whether Prototype copies can be ordered in other multiples than 3: NOT stated (the only "multiple of 3" text is for 4L Medium Run). Scotty list item 1.
- Area [computed]: (a) 44.10x48.50 = 2138.85 mm2 /645.16 = 3.3153 in2; (b)/(c) 100.05x107.60 = 10765.38 mm2 /645.16 = 16.687 in2. (Assumes OSH Park bills the rectangle area; not stated whether it rounds.)
- 5 boards = 2 sets (6 boards); 10 boards = 4 sets (12 boards), assuming sets of 3 only.

| Board | per set of 3 [computed] | "5 boards" = 2 sets (6) [computed] | "10 boards" = 4 sets (12) [computed] |
|---|---|---|---|
| (a) 44.10x48.50, 2L | 3.3153 x $5 = $16.58 | $33.15 | $66.31 |
| (b) 100.05x107.60, 2L | 16.687 x $5 = $83.44 | $166.87 | $333.75 |
| (c) 100.05x107.60, 4L | 16.687 x $10 = $166.87 | $333.75 | $667.49 |
USD. Shipping not read. OSH Park home page also mentions a Bulk Order service, 40% reduction for prototype/4-layer, qualifying quantity not stated.
- Assembly: neither page mentions assembly [read: docs/services covers fabrication only]; "no assembly" is not positively stated by the page.

### Second fabricator - none gave a computable rule
- JLCPCB https://jlcpcb.com/ : [read] "From $2.00 / 5 pcs" (1-32 layers), "Costs depend on size, layer count, surface finish, and quantity"; USD; no table. Not computable.
- PCBWay https://www.pcbway.com/ : [read] "From $5/10pcs"; special offer "$5.00 for 10 pcs, 1-2 layer" at 100x100 mm 1.6 mm (promo, not a table; 4L, 5 pcs, and sizes beyond 100x100 absent). Not computable.
- Aisler https://aisler.net/en/pricing : HTTP 404; https://aisler.net/ : too many redirects.
- Eurocircuits, Seeed Fusion, PCBgogo: not tried.
Hence all three boards for a second fab go on the Scotty list. Note PCBWay 100x100 limit: (b)/(c) at 100.05x107.60 exceed the 100x100 promo tier.

## Part 2 - Assembly (https://jlcpcb.com/help/article/pcb-assembly-price), USD, read 2026-11-16
| Item | Economic | Standard |
|---|---|---|
| Setup | $8.18 | $25.56 single-sided / $51.12 double-sided |
| Stencil | $1.53 | $8.21 single / $16.42 double |
| SMT per joint | $0.0016 (1-100,000) | $0.0016 (1-50,000); $0.0013 (50,001-100,000); $0.0012 (100,001-1,000,000) |
| Extended part feeder loading | $3.07 "Feeder Loading" [page does not say per unique part; confirm on live page/BOM quote] | not read |
| Manual (hand) soldering | not read for Economic specifically | $0.0164/joint (1-10,000), $0.015 (10,001-30,000), $0.012 (30,001-100,000); plus $3.58 per order hand-soldering labour |
- Size/layer eligibility: only a "Large Size" charge of $57.46 when PCB area exceeds 650 cm2 [read]. Page states no Economic layer limit, nor a 100x100 limit. (b)/(c) area = 10765.38 mm2 = 107.65 cm2 [computed], well under 650. Pages pcb-assembly-capabilities, pcb-assembly-service, Economic-PCBA-or-Standard-PCBA returned 404. Economic size/layer limits therefore NOT confirmed (Scotty list).

## Part 3 - digikey.sg (all "All prices are in SGD"; Cut Tape/Digi-Reel; read 2026-11-16)
URL form: https://www.digikey.sg/en/products/result?keywords=<MPN>
| MPN | Status | q1 SGD | q10 (each) SGD |
|---|---|---|---|
| GRM32ER71A226KE20L | Not For New Designs | 2.87 | 1.917 (ext 19.17) |
| GRM32ER61C226KE20L | Not For New Designs | 2.58 | 1.713 |
| GCM188R71H104KA57D | Active, 0 stock, backorder | 0.27 | 0.156 |
| GCM188R71H224KA64D | Active | 0.23 | 0.134 |
| GRM188R71C105KA12D | Obsolete | no price shown (substitute listed: YAGEO CC0603KRX7R7BB105 $0.33000, one qty only, not a q1/q10 break) | - |
| TNPW060310K2BEEA (0.1 % 10.2k; exact MPN as given) | Active | 0.36 | 0.30 |
| CRCW060310K0FKEA | not stated | only "$0.14000" with no quantity; no q1/q10 break; currency not labelled | - |
| GRM155R71C104KA88D (0402 100 nF) | Active, 0 stock, ~10,000 expected 16-Dec-2026 | 0.14 | 0.034 |
| RC0402FR-0710KL (0402 10k 1 %) | Active | 0.14 | 0.032 (ext 0.32) |
| GRM155R61A105KE15D (0402 1 uF) | Obsolete | no price | - |
| GRM155R61A225KE15D (0402 2.2 uF) | no results | - | - |
| GRM155R60J475ME47D (0402 4.7 uF, 6.3 V X5R) | Active | 0.14 | 0.067 |
| XT30PW-M | no results on digikey.sg | - | - |
Fetch caveat: WebFetch uses a summarising model; spot-check high-stakes numbers. TNPW0603 10.2k is not 10k; a 10.0k 0.1 % MPN was not searched.

## For Scotty to fetch
1. OSH Park multiples: whether Prototype can be ordered in copies other than sets of 3 (e.g. 6, 9), and rounding/min area, copper oz, 4L stackup: https://docs.oshpark.com/services/ and the order page https://oshpark.com/ (upload-free: the "Order a PCB" form shows live price); also whether area is billed by outline or rounded.
2. Second fab prices for (a), (b), (c) at 5 and 10 pcs: JLCPCB instant quote https://cart.jlcpcb.com/quote (enter sizes/layers/1.6 mm/1 oz); or PCBWay https://www.pcbway.com/orderonline.aspx; or Aisler https://aisler.net (their price page 404/redirect-loops here). Read the board-only price (5 and 10 pcs) and shipping.
3. JLCPCB Economic PCBA eligibility: size/layer limits for Economic vs Standard (check https://jlcpcb.com/help/article/pcb-assembly-capabilities alternatives via https://jlcpcb.com/help and the "Economic" tab in the quote page); whether 4 layers or >100x100 mm disqualifies; whether the $3.07 extended-part fee is per unique extended part.
4. JLCPCB part pages (extended vs basic classification and price) for each unique BOM part: https://jlcpcb.com/parts (search the MPN).
5. XT30PW-M: https://www.lcsc.com/search?q=XT30PW-M (C431064-style listing; read price at q1/q10, stock, currency); also try https://www.digikey.sg/en/products/result?keywords=AMASS%20XT30 .
6. GRM188R71C105KA12D (obsolete): pick a live 0603 1 uF X7R replacement and price it; GRM155R61A105KE15D (obsolete) and 2.2 uF 0402 (GRM155R61A225KE15D not found; DigiKey suggests GRM155R61A224KE19D, which is 0.22 uF, not 2.2 uF): choose a live MPN, e.g. check https://www.digikey.sg/en/products/result?keywords=GRM155R60J225ME15D .
7. CRCW060310K0FKEA q1/q10 and a TNPW0603 10.0k 0.1 % MPN: https://www.digikey.sg/en/products/result?keywords=CRCW060310K0FKEA (open the product page, read the quantity price table).
