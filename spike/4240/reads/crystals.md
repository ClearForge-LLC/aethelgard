# AET-WO-4240 — HSE crystal candidates (STM32H753), research read

> **Corrected by the builder (2026-11-19, ABM8AIG REVISED 06-13-19 p. 1):** ABM8AIG ESR at 20.000–25.999 MHz is **50 Ω**, not 40 Ω as rows 3–4 below say. FEEDBACK §3.1 carries the corrected figures.

Compiled 2026-11-19 (UTC), from datasheets held in `/home/forge/.cache/aet-4240/ds/` (SHA256SUMS there).
Every figure below is from a held PDF (cited doc + revision + page), except: the distributor columns (LCSC page
fetch, 2026-11-19T14:50-14:51Z, plain curl) and anything marked [inferred]. Nothing was filled from memory.
"uncited" = the PDF carries no document number or revision.

## 0. Held datasheets (ds/)

| File | Doc id / revision as printed | Notes |
|---|---|---|
| abracon-ABM8-rev2020-07-29.pdf | "ABM8", REVISED: 07-29-20, no doc number | 2 pp |
| abracon-ABM8G-rev2015-08-13.pdf | "ABM8G", Revised 08.13.15, no doc number | 2 pp; 12 MHz min; text partly garbled in extraction |
| abracon-ABM8AIG-rev2019-06-13.pdf | "ABM8AIG", REVISED: 06-13-19 | 5 pp; automotive, AEC-Q200 |
| abracon-ABM8AAIG-rev2023-02-14.pdf | "ABM8AAIG Series", REVISED: 02-14-23 | 5 pp; glass sealed, 0.9 mm |
| abracon-ABM8W-rev2022-07-11.pdf | "ABM8W Series", REVISED: 07-11-22 | 9 pp; low CL |
| ecs-ECX-32-rev2017.pdf | "ECX-32", Rev.2017 | 2 pp |
| ecs-ECX-33Q-rev2023.pdf | "ECX-33Q", Rev.2023 | 2 pp; AEC-Q200 |
| ecs-ECX-33B2Q-rev2024.pdf | "ECX-33B2Q", Rev.2024 | 2 pp; AEC-Q200 |
| ndk-NX3225GA-2008E.pdf | "2008E_NX3225GA_e" | 1 p |
| ndk-NX3225GB-2008D.pdf | "2008D_NX3225GB_e" | 1 p; AEC-Q200 |
| ndk-NX3225GD-2008E.pdf | "2008E_NX3225GD_e" | 1 p; AEC-Q200 |
| ndk-NX3225SA-2112F.pdf | "2112F_NX3225SA_e" | 1 p |
| epson-FA-238-TSX-3225-norev.pdf | Epson "FA-238V / FA-238 / TSX-3225" brief, no doc no./rev (PDF metadata date 2025-10-14 only) | **uncited**; 2 pp; same bytes returned for the FA-238, FA-238V and TSX-3225 URLs |
| yxc-YSX321SL-norev.pdf | YXC "YSX321SL", no doc no./rev (PDF modDate 2025-05-08) | **uncited**; 1 p; image-only PDF, read by rendering page to PNG |
| st-AN2867-DocID15287-Rev10.pdf | ST AN2867, "DocID15287 Rev 10", August 2015 | see section 5 |

Not obtained: any TXC datasheet (7M/7V/8Y), any Murata crystal, any Epson/NDK C0 figure (NDK sheets do not state C0).
Fetched but not kept (wrong package): Abracon ABM3B (5.0x3.2), ABM10 (2.5x2.0).
Pre-existing `ds/abm8.pdf` was byte-identical (cmp) to the fresh abracon.com download; I renamed it to the convention.

## 1. Master table (one row per part; datasheet figures)

Columns: freq | orderable P/N (decode in section 2) | tol @25C | stability / range | aging | CL (options) | C0 max | ESR max at that freq | DL max | pullability | Top | package | 4-pad 3225? | cite
All parts: pullability **not stated in any held datasheet**. "Tol/stab" = ppm.

| # | Freq | Orderable P/N | Tol | Stab / range | Aging | CL | C0 max | ESR max | DL | Top | Pkg | 4-pad? | Cite (doc, rev, page) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 8 | ABM8AIG-8.000MHZ-1Z-T | ±10 (code 1) | ±50 (Z) over -40..+125 | ±2 first yr @25C | 18 std (S, or any pF) | 2.0 pF | 500 Ω (8.000-9.999) | 100 µW (10 in typ/min col) | -40..+125 (V opt. +150, contact) | 3.2x2.5x0.8 max | Yes: pads 1/3 crystal, 2/4 GND (lid) | ABM8AIG rev 06-13-19, p1 (specs), p2 (codes), p3 (pads) |
| 2 | 16 | ABM8AIG-16.000MHZ-12-2Z-T3 [inferred from p2 table; matches a DigiKey listing name seen in search snippets, not verified] | ±20 (2) | ±50 (Z) -40..+125 | ±2 | 12 pF (coded) | 2.0 | 70 Ω (16.000-19.999) | 100 µW | -40..+125 | same | Yes | same |
| 3 | 24 | ABM8AIG-24.000MHZ-12-2Z-T3 [inferred, as #2] | ±20 | ±50 | ±2 | 12 | 2.0 | 40 Ω (20.000-25.999) | 100 µW | -40..+125 | same | Yes | same |
| 4 | 25 | ABM8AIG-25.000MHZ-<CL>-<tol><stab>-T3 [pattern only; no listing seen] | per code | per code | ±2 | per code | 2.0 | 40 Ω | 100 µW | -40..+125 | same | Yes | same |
| 5 | 8..25 | ABM8AAIG-f.fffMHZ-<CL>-<tol><stab>-T3 (e.g. 16.000, 24.000, 25.000, 8.000; no specific P/N verified) | ±50 std; 1=±30, 2=±15 | ±100 std, R ±150, Z ±50, Y ±30 (contact); -40..+125 | ±3 first yr | 8 std (or any) | 2.0 | 8.000-9.799: 500; 9.800-12.000: 120; 12.001-54: 50 | 200 µW (10 other col) | -40..+125 (D +85, J +105, V +150 only w/ stab R) | 3.2x2.5x0.9, glass sealed | Yes (p3 dimension drawing; pad table not extracted) | ABM8AAIG rev 02-14-23, p1, p2 |
| 6 | 8 | ABM8 family, not automotive: ABM8-8.000MHZ-<CL>-<tol><stab><temp>-T e.g. "...-D1G-T" family, 8 MHz exists in range (8.000 min) | ±50 std; 1=±10..4=±30 | ±50 std, -10..+60; option letters give ±10..±35 | ±2 first yr | 18 std (min 6) | 3.0 | 400 Ω (8.000-9.999) | 100 µW | -10..+60 std; D -40..+85; J +105, K +125 (K/J only stab ±50/±100) | 3.2x2.5x0.8 max (ABM81 0.65, ABM82 0.60) | Yes (outline p2) | ABM8 rev 07-29-20, p1-2 |
| 7 | 12 | ABM8-12.000MHZ-D2Y-T (LCSC name) | ±20 (2) | ±30 (Y) -40..+85 (D) | ±2 | 18 | 3.0 | 120 Ω (12.000-15.999) | 100 µW | -40..+85 | as 6 | Yes | ABM8 rev 07-29-20, p1 |
| 8 | 24 | ABM8-24.000MHZ-D1G-T | ±10 (1) | ±15 (G) -40..+85 | ±2 | 18 | 3.0 | 50 Ω (20.000-29.999) | 100 µW | -40..+85 | as 6 | Yes | same |
| 9 | 25 | ABM8-25.000MHZ-D2Y-T | ±20 | ±30 | ±2 | 18 | 3.0 | 50 Ω | 100 µW | -40..+85 | as 6 | Yes | same |
| 10 | 16 | ABM8-16.000MHZ-B2-T (LCSC name; B = -20..+70 → NOT -40; shown for reference only) | ±20 | -20..+70 | ±2 | 18 | 3.0 | 70 Ω | 100 µW | -20..+70 | as 6 | Yes | same |
| 11 | 16/24/25 (≥10 MHz only) | ABM8W-<f>MHZ-<CL 4/6/7/8>-<tol><stab><temp>-T3 | ±10..±50 | ±10 only temp codes I/E/B; ±15..±35 up to D (-40..+85); J/K (+105/+125) only stab Z(±50)/Q(±100) | ±2 /yr | 4 pF max listed ("CL 4.0" row), codes 8/7/6/4 | 2.0 (typ 1.2) | 12-15.999: 100; 16-19.999: 70; 20-29.999: 50 | 100 µW | -40..+125 | 3.2x2.5x0.8 | Yes (p7) | ABM8W rev 07-11-22, p1, p2 |
| 12 | 12 | ABM8G-12.000MHZ-… (12.000 is the minimum) | ±50 std | ±50 std, -10..+60; options up to -40..+85 (D) | ±3 first yr | 10 pF std (min 7) | 5.0 | 120 Ω @12.000 | 100 µW | -10..+60 std | 3.2x2.5x1.0 glass sealed | Yes | ABM8G rev 08.13.15, p1-2 (text partly garbled; ESR/CL read from garbled extraction, treat as [inferred] until viewed) |
| 13 | 16 | ECS-160-18-33Q-<tol><stab><temp>-TR (pattern per ECX-33Q p1; datasheet example "ECS-200-20-33Q-DU-TR") | ±30 std; A ±25, J ±20, R ±15, C ±10 | ±100 -55..+125 (DU); stab codes D ±100, E ±50, G ±30, H ±25, T/W/K ±20/15/10 "contact ECS"| ±5 first yr | 6..20 pF or series (10, 20, S coded) | 7 pF | 80 Ω (16.000-19.999) | 100 µW | temp: N -40/+85, P +105, S +125, U -55/+125 | 3.2x2.5 (height "low profile"; ECX-32 sibling says 0.8) | Yes (pads 1 In/Out, 2 Gnd, 3 Out/In, 4 Gnd) | ECX-33Q Rev.2023, p1; p2 lists 8, 12, 16, 16.384, 24, 24.576, 25 MHz as "developed frequencies" |
| 14 | 24 | ECS-240-<CL>-33Q-… | as 13 | as 13 | ±5 | as 13 | 7 | 40 Ω (24.000-54.000) | 100 µW | as 13 | as 13 | Yes | same |
| 15 | 25 | ECS-250-<CL>-33Q-… | as 13 | as 13 | ±5 | as 13 | 7 | 40 Ω | 100 µW | as 13 | as 13 | Yes | same |
| 16 | 8 | ECS-80-<CL>-33Q-… | ±30 std | ±100 (DU) | ±5 | as 13 | 7 | 500 Ω (8.000-9.999) | 100 µW | as 13 | as 13 | Yes | same |
| 17 | 8, 16, 24, 25 | ECX-33B2Q: ECS-<f>-8-33B2Q-<tol><stab><temp>-TR3; example in sheet "ECS-200-8-33B2Q-VY-TR3" | ±30 std | ±150 over -40..+150 (VY); other stab codes "contact ECS" | ±2 first yr | 8 pF std (10, 18, S coded) | 2 pF | 8-9.999: 500; 10-15.999: 100; 16-19.999: 70; 20-25.999: 50 | 200 µW | -40..+150 (N +85, P +105, S +125, U -55/+125 codes) | 3.2x2.5 | Yes | ECX-33B2Q Rev.2024, p1-2 (p2 developed-frequency list starts at 8.000) |
| 18 | 10-54 (8 not in range) | ECS-xxx-xx-33-…-TR (ECX-32): e.g. ECS-200-20-33-TR | ±50 std | ±50 over -20..+70; ext. temp via codes (N -40/+85, P +105, S +125) | ±5 first yr | 10, 20, S | 5 pF | 10-15.999: 100; 16-19.999: 80; 20-23.999: 60; 24-50: 40 | 100 µW | -20..+70 std | 3.2x2.5x0.8 | Yes | ECX-32 Rev.2017, p1 |
| 19 | 16 | NX3225GA-16.000M-STD-CRG-2 [name as on LCSC; spec no. STD-CRG-2 is printed on sheet] | ±20 | ±30 over -40..+85 | std "---" (none stated); optional max ±10 /yr "NDK will study" | 8 std; 6..32 opt | not stated | 80 Ω (13 ≤ F < 20) | 10 µW (max 200) | -40..+85 | 3.2x2.5x0.75 typ (±0.15) | 4 pads, #2/#4 "not connected, GND better" | NDK 2008E_NX3225GA_e, p1 |
| 20 | 24 | NX3225GA-24.000M-STD-CRG-2 | ±20 | ±30 -40..+85 | as 19 | 8 | n/s | 50 Ω (20 ≤ F ≤ 50) | 10 µW (max 200) | -40..+85 | as 19 | as 19 | same |
| 21 | 25 | NX3225GA-25.000M-STD-CRG-2 [pattern] | ±20 | ±30 | as 19 | 8 | n/s | 50 Ω | as 19 | -40..+85 | as 19 | as 19 | same |
| 22 | 8 | none: NX3225GA starts at 9.840 MHz (NDK 2008E p1) | | | | | | | | | | | |
| 23 | 8 | NX3225GD-8.000M-STD-CRA-3 [LCSC-style name NX3225GD-8MHZ-STD-CRA-3; spec no. STD-CRA-3 on sheet] | ±50 | **±150** over -40..+150 | std none; opt ±10 /yr | 8 std; 6..32 | n/s | 500 Ω (7.98 ≤ F < 9.8) | 10 µW (max 200) | -40..+150, AEC-Q200 | 3.2x2.5x0.8 ±0.2 | LCSC lists SMD3225-**2P** [inferred; sheet pad table not extracted] | NDK 2008E_NX3225GD_e, p1 |
| 24 | 12-25 | NX3225GB-<f>-STD-CRA-2 | ±50 | ±150 -40..+150 | opt ±10 /yr | 8 | n/s | 12-19.99: 120; 20-50: 100 | 10 µW (max 200) | -40..+150, AEC-Q200 | 3.2x2.5x0.75±0.15 | pad table not extracted | NDK 2008D_NX3225GB_e, p1 |
| 25 | 12, 16, 24, 25 | NX3225SA-<f>-STD-CQR-1 (sheet's spec no.) | ±15 std (±10 opt) | ±25 over -40..+85 | std none; opt ±3 /yr | 8 std; 6..32 | n/s | 12-12.99: 100; 13-19.99: 80; 20-64: 50 | 10 µW (max 200) | -40..+85 | 3.2x2.5x0.55±0.05 | 4 pads, #2/#4 lid GND | NDK 2112F_NX3225SA_e, p1 |
| 26 | 16 | Epson FA-238 (16-50 MHz) / TSX-3225 (16-50): Epson product no. Q22FA2380xxxx17 (FA-238), X1E000021xxxx17 (TSX-3225); name form "FA-238 16.000000MHz 12.0 +15.0-15.0" (freq, CL, tol) | ±50 std (±15..±50 avail); TSX ±10 | FA-238 ±30 over -20..+70; TSX ±10 over -20..+75 | FA ±5 /yr max; TSX ±1 /yr max (±2 ≥40 MHz), first year @25C | 7 pF..∞, specify | not stated | FA-238 (Table 1): 13<f<20: 80, 20-<25: 60, 25-<30: 50, 30-50: 40; TSX-3225 (Table 2): 16-<21: 60, 21-50: 40 | 200 µW max (rec. 10) | -40..+85 (+105 "contact") | FA-238 3.2x2.5x0.7; TSX 0.6 | Yes (#2/#4 to cover, ground) | Epson brief, no doc no./rev → **uncited**, p1 |
| 27 | 12 | FA-238V (12-15.999 MHz): Q22FA23V0xxxx17 | ±50 std | ±30 -20..+70 | ±5 /yr | 7 pF..∞ | n/s | 12-13: 100 Ω | 200 µW | -40..+85 | 3.2x2.5x0.7 | Yes | same, **uncited** |
| 28 | 8 | YXC YSX321SL (8-64 MHz); LCSC part X32258MSB4SI is 8 MHz/20 pF/±10 ppm [inferred it is YSX321SL: LCSC's own datasheet link for C2682774 resolves to this PDF] | ±10, ±20 or specify | ±20 or specify, over -40..+85 | ±3 /yr max | 10, 20 or specify | 3 pF | 8-11.2892: 180 Ω; 12-15: 80; 16-31: 50; 32-64: 40 | 200 µW max (100 typ) | -40..+85 or specify | 3.2x2.5x0.70±0.1 | Yes (4 pads, #2/#4 GND) | YXC YSX321SL, no doc no./rev → **uncited**, p1 |
| 29 | 12, 16, 24, 25 | YSX321SL variants: ordering table NOT in the PDF; no orderable code decodable | | | | | | | | | | | |

Observations that matter for H753 [inferred, not datasheet claims]:
- Only #1 (ABM8AIG), #5 (ABM8AAIG), #17 (ECX-33B2Q), #13-16 (ECX-33Q), #19-21/#23-25 (NDK GA/GB/GD/SA) combine 3225 4-pad with extended temp; of these only ABM8AIG, ABM8AAIG, ECX-33Q/33B2Q, NX3225GB/GD are marked AEC-Q200 in their datasheets.
- 8 MHz in 3225 with a **tight** (<=±50) stability over -40..+85 or better exists only in: ABM8AIG (±50 over -40..+125, via code Z), ABM8 (D option), YXC YSX321SL (±20 over -40..+85, uncited). NDK GD and ECX-33B2Q are ±150 over -40..+150.
- ESR at 8 MHz is 400-500 Ω for every maker's 8 MHz row (ABM8 400, ABM8AIG 500, ECX-33Q 500, GD 500; YXC 180). The H753 HSE gain margin (AN2867 method) should be computed against that, not against 16/24/25 MHz figures. 16-25 MHz ESR is 40-120 Ω.
- None of the held datasheets gives pullability (ppm/pF) or C1; ABM8W p5 gives C0/R1/L1/C1 as measured typicals only for 4/6 pF plating (10, 27, 50 MHz), not for 8/16/24/25.

## 2. Part-number decoding (from the datasheets' ordering tables)

**ABM8AIG (rev 06-13-19, p2):** `ABM8AIG-<MHz>-<CL>-<tol><stab><temp>-<pkg>`; CL blank = 18 pF, else pF or S.
Tol: blank ±50, 4 ±30, 3 ±25, 2 ±20, 7 ±15, 1 ±10. Stab: blank ±100, R ±150, Z ±50, P ±45(*), F ±40(*), H ±35(*), Y ±30(*), W ±25(*), X ±20(*), G ±15(*) ((*) contact Abracon). Temp: blank -40..+125, N -30..+85, D -40..+85, J -40..+105, V -40..+150 (*). Pkg: blank bulk, T 1k reel, T3 3k reel. Custom ESR: R<Ω>.
Example `ABM8AIG-8.000MHZ-1Z-T` = 8 MHz, CL 18 (blank), ±10 tol (1), ±50 stab (Z), -40..+125 (temp blank), 1k reel. The "1Z" is tol+stab; the temp position is blank. LCSC lists it as ±10 / ±50 / 18 pF / -40..+125 / 500 Ω, consistent.

**ABM8AAIG (rev 02-14-23, p2):** CL blank 8 pF; stab blank ±100, R ±150, Z ±50, Y ±30; temp blank -40..+125 (only with stab blank/R), D -40..+85, J -40..+105 (stab Z/blank/R), V -40..+150 (stab R); tol blank ±50, 1 ±30, 2 ±15; pkg blank bulk, T3 3k.

**ABM8 (rev 07-29-20, p1):** `ABM8-<MHz>-<CL>-<tol><stab><temp>-<pkg>`; tol 1 ±10, 7 ±15, 2 ±20, 3 ±25, 4 ±30, blank ±50; stab U ±10 (only E/B/-10..60), G ±15, X ±20, W ±25, Y ±30, H ±35, Q ±100, blank ±50; temp blank -10..+60, E 0..+70, B -20..+70, C -30..+70, N -30..+85, D -40..+85, J -40..+105, K -40..+125 (J/K only stab ±50 / ±100). Height blank 0.80, 1 = 0.65, 2 = 0.60 (ABM81/ABM82). Pkg T 1k, T2 250, T3 3k.
`ABM8-12.000MHZ-D2Y-T` = -40..+85, ±20 tol, ±30 stab, CL 18 (blank), 1k reel. `ABM8-24.000MHZ-D1G-T` = -40..+85, ±10, ±15. `ABM8-25.000MHZ-D2Y-T` as 12 MHz.
Note: the stab Y/G combined with D ("-40..+85") — the sheet footnote restricts only U to -10..+60 and J/K to ±50/±100, so D2Y and D1G are legal per table.

**ABM8W (rev 07-11-22, p2):** CL 8/7/6/4; tol 1..5 (5 = ±50); stab U ±10 (temp I/E/B only), G..H ±15..±35 (temp I,E,B,C,N,D), Z ±50, Q ±100; temp I 0..50, E 0..+70, B -20..+70, C -30..+70, N -30..+85, D -40..+85, J -40..+105 (Z/Q only), K -40..+125 (Z/Q only).

**ECX-33Q (Rev.2023, p1):** `ECS-<freq code>-<CL>-33Q-<tol><stab><temp>-TR[3]`; freq code 80=8.000, 160=16.000, 240=24.000, 250=25.000 (p2 "Developed frequencies"); tol blank ±30, A ±25, J ±20, R ±15, C ±10; stab D ±100, E ±50, G ±30, H ±25, T ±20†, W ±15†, K ±10† († "contact ECS for availability over extended temp range"); temp L -10..+70, M -20..+70, Y -30..+85, N -40..+85, P -40..+105, S -40..+125, U -55..+125; TR 1k, TR3 3k reel. Datasheet example: ECS-200-20-33Q-DU-TR (stab D, temp U).
Pattern examples (constructed, [inferred]): `ECS-160-18-33Q-JES-TR` = 16 MHz, 18 pF, ±20 tol, ±50 stab, -40..+125 (the real orderable LCSC listing `ECS-163.8-12-33Q-JES-TR` for 16.384 MHz/12 pF uses exactly this tail).
Note: the sheet's footnote "JES-TR & JEN-TR 3K/Reel".
**LCSC listings "ECS-160-18-33QZ-ADS-TR" / "ECS-250-18-33QZ-ADS-TR": the "33QZ" series code is not defined in the held ECX-33Q sheet** (a "Z" suffix appears nowhere). Treat as an undocumented variant; "ADS" would decode as ±25 tol / ±100 stab / -40..+125 per the table.

**ECX-33B2Q (Rev.2024, p1):** `ECS-<f>-<CL 8/10/18/S>-33B2Q-<tol><stab><temp>-TR/TR3`; tol blank ±30, A ±25, J ±20, R ±15, C ±10; stab V ±150, D ±100†, E ±50†, G ±30†, H ±25†, T ±20†, W ±15†, K ±10†; temp L..Y with Y = -40..+150, N +85, P +105, S +125, U -55/+125. Example in sheet: ECS-200-8-33B2Q-VY-TR3.

**ECX-32 (Rev.2017, p1):** `ECS-<f>-<CL>-33-<tol><stab><temp>-TR`; "33 = ECX-32". Tol blank ±50, A ±25, J ±20, R ±15, C ±10; stab blank std, D ±100, E ±50, G ±30, H ±25, T ±20†, W ±15†, K ±10†; temp blank std, L, M, Y -30..+85, N -40..+85, P -40..+105, S -40..+125, U -55..+125.
`ECS-80-18-33-JGN-TR` / `ECS-80-10-33-CGN-TR3` (listed at LCSC with 8 MHz, 400 Ω): decode = J ±20 / C ±10 tol, G ±30 stab, N -40..+85 [inferred from table]. **The ECX-32 sheet's frequency range is 10.000-54.000 MHz (p1), so an 8 MHz ECX-32 is not covered by the held datasheet** (the part exists as a listing; the 400 Ω ESR is an LCSC field, not a datasheet figure). Flagged in doubts.

**NDK (all four sheets, "Specifications Number" + ordering example):** `<model> <MHz 6 digits> S1-<temp>-<stab>-<tol>-<CL>`; sheet example `NX3225GA 38.400000MHz S1-4085-30-20-8` = fundamental, -40..+85, ±30 over temp, ±20 @25C, 8 pF. Standard spec numbers: GA STD-CRG-2, GB STD-CRA-2, GD STD-CRA-3, SA STD-CQR-1. LCSC's "NX3225GA-16.000M-STD-CRG-2" = model + MHz + spec no.

**Epson (brief):** Epson product name form "FA-238V 12.000000MHz 12.0 +15.0-15.0" = model, freq, CL (pF), tolerance (ppm); "specify temp characteristic and operating range when inquiring". Product numbers Q22FA23V0xxxx17 / Q22FA2380xxxx17 / X1E000021xxxx17 (xxxx = variable).

## 3. Per-part notes with page cites

- **ABM8AIG** (rev 06-13-19). p1: 8.000-54.000 MHz, fundamental; tol ±50 std @25C; stability ±100 (blank) or ±150 (R) over the op range; ESR R1 500/100/70/50/40/35 Ω for 8.000-9.999 / 10-15.999 / 16-19.999 / 20-25.999 / 26-39.999 / 40-54; C0 2.0 pF max; CL 18 std; DL 10-100 µW (the "10" sits in the min/typ columns, ambiguous); aging ±2 ppm first yr @25C±3; AEC-Q200; Grade 1 -40..+125, Grade 0 -40..+150 "available upon request"; note 1: +150 only in select frequencies. p3: pad 2 and 4 are GND (electrically lid). Height 0.8 mm (title block).
- **ABM8AAIG** (rev 02-14-23). p1: 8-54 MHz; ESR 500 (8.000-9.799), 120 (9.800-12.000), 50 (12.001-54.000); C0 2.0; CL 8 std; DL 10-200 µW; aging ±3 first yr; glass sealed, 0.9 mm; AEC-Q200; -40..+125 std (+150 option V).
- **ABM8** (rev 07-29-20). p1: std -10..+60 (note: not an automotive grade); ESR table 8-9.999: 400; 10-11.999: 200; 12-15.999: 120; 16-19.999: 70; 20-29.999: 50; 30-39.999: 40; 40-54: 35; C0 3.0 pF; CL 18 std; aging ±2 first yr; MSL N/A. Extended temp only via order codes D/J/K.
- **ABM8W** (rev 07-11-22). p1: 10.0000-54.0000 MHz; stab/tol ±10 listed as the table default; ESR "< typ / max" columns: 12-15.9999 MHz < 60 typ / 100 max; 16-19.9999 < 40 / 70; 20-29.9999 < 25 / 50; C0 < 1.2 typ / 2.0 max; CL min 4 pF; DL 10-100 µW; aging ±2 /yr; temp -40..+125. p5: measured C0/R1/L1/C1 at 4 and 6 pF plating for 10, 27, 50 MHz only.
- **ABM8G** (rev 08.13.15). 12.000-50.000 MHz (p1) — **no 8 MHz**. Glass-sealed 1.0 mm. Its text layer is partially scrambled; the ESR (120/80/60/50), C0 5.0, DL 10-100, aging ±3 values were read from the garbled extraction and need an eyeball render before anyone relies on them.
- **ECX-33Q** (Rev.2023). p1: 8.000-54.000 MHz; tol ±30; stab ±100 over -55..+125 (DU option; extended-temp stability other than DU "contact ECS"); C0 7 pF max; CL 6..20 pF or series; DL 100 µW; aging ±5 first yr; ESR 500/100/80/60/40 for 8-9.999/10-15.999/16-19.999/20-23.999/24-54; AEC-Q200; 4 pads (1 In/Out, 2 Gnd, 3 Out/In, 4 Gnd). p2: "developed frequencies" include 8.000, 12.000, 16.000, 24.000, 25.000; MSL 1, Au lead finish. Height not printed in the text layer [dimension drawing not read].
- **ECX-33B2Q** (Rev.2024). p1: 8-54 MHz; tol ±30; stab ±150 over -40..+150 (VY); C0 2 pF max; CL 8 pF std; DL 200 µW; aging ±2; ESR 500/100/70/50/35 for 8-9.999/10-15.999/16-19.999/20-25.999/26-54; AEC-Q200.
- **ECX-32** (Rev.2017). p1: 10-54 MHz (no 8 MHz), -20..+70 std; C0 5; aging ±5; 3.2x2.5x0.8; ESR 100/80/60/40.
- **NX3225GA** (2008E). p1: 9.840-50 MHz; ESR 9.84≤F<12: 200, 12-13: 100, 13-20: 80, 20-50: 50 Ω; no C0 stated; 3.2x2.5x0.75 typ (±0.15 on drawing); pads #2/#4 not connected.
- **NX3225GB** (2008D). p1: 12-50 MHz, -40..+150, AEC-Q200; ESR 120 (12-20), 100 (20-50). The sheet itself points 7.98-12 MHz buyers to NX3225GD.
- **NX3225GD** (2008E). p1: 7.98-12 MHz, -40..+150, AEC-Q200, "engine control CPU clock"; stab ±150 / tol ±50; ESR 500 (7.98-9.8), 300 (9.8-12); 0.8±0.2 mm.
- **NX3225SA** (2112F). p1: 12-64 MHz; tol ±15 (±10 opt), stab ±25 over -40..+85; ESR 100/80/50; 0.55±0.05 mm; #2/#4 to lid GND. Optional aging ±3 /yr only "NDK will study".
- **Epson FA-238V/FA-238/TSX-3225** (uncited). p1 as in the master table; op range stated -40..+85 (+105 on contact); storage -40..+125; page 2 is Epson's legal notice and the "automotive" mark legend (no per-part automotive marking was extracted).
- **YXC YSX321SL** (uncited). p1: 8-64 MHz, AT fundamental; "-40~+85 °C or specify"; C0 3 pF max; DL 10-200 µW (100 typ); aging ±3 ppm/yr max; ESR 180/80/50/40 for 8-11.2892/12-15/16-31/32-64; 3.20x2.50x0.70±0.1. The LCSC listing for the matching 8 MHz part gives ESR 120 Ω in its title line and 180 Ω in its attributes (see doubts).

## 4. Distributor stock and price (qty 1 / qty 100)

Method: plain `curl` GET of the public LCSC product page; the SSR JSON carried stock and the price ladder. Read at 2026-11-19T14:50:15Z-14:51:03Z, USD, LCSC "ladder" steps are the only quantities shown (not every part has a 1 or 100 step; the nearest step is stated). All Reel = 1000 or 3000 per page "minPacketNumber".
DigiKey: 403 (Cloudflare "Just a moment...") on all three detail URLs tried. Mouser: TLS/HTTP2 connection closed (curl 92 INTERNAL_ERROR) on both URLs. **DigiKey: unreadable from Furnace. Mouser: unreadable from Furnace.** Did not retry with changed client settings (not circumventing). No public documented DigiKey/Mouser/LCSC API was used (DigiKey and Mouser APIs need credentials; LCSC's JSON endpoints are not a documented public API → "unreadable from Furnace" for any API route).

| LCSC code | P/N | Stock | Min buy | Price ladder (USD) | Fetch URL |
|---|---|---|---|---|---|
| C5140594 | ABM8AIG-8.000MHZ-1Z-T | 8065 | 1 | 1: 1.0870, 10: 0.8723, 30: 0.7666, **100: 0.6592**, 500: 0.5964, 1000: 0.5634 | https://www.lcsc.com/product-detail/C5140594.html |
| C2682774 | YXC X32258MSB4SI (8 MHz, 20 pF) | 92700 | 5 | 5: 0.1963, 50: 0.1517, 150: 0.1299 (no 100 step), 500: 0.1104 | .../C2682774.html |
| C7183312 | ECS-80-10-33-CGN-TR3 (8 MHz, 10 pF, ±10/±30, -40..+85) | 107 | 1 | 1: 1.2525, 10: 1.0476, 30: 0.9353, **100: 0.8064** | .../C7183312.html |
| C5727301 | ECS-250-18-33QZ-ADS-TR (25 MHz, 18 pF, ±25/±100, -40..+125) | **0** | 1 | 1: 0.7865, 200: 0.3152 | .../C5727301.html |
| C6941862 | ECS-160-18-33QZ-ADS-TR (16 MHz) | **0** | 1 | 1: 0.9078, 200: 0.3622 | .../C6941862.html |
| C435184 | ECS-163.8-12-33Q-JES-TR (16.384 MHz, 12 pF, ±20/±50, -40..+125) | 17 | 1 | 1: 1.6299, 10: 1.3396, 30: 1.1791, **100: 0.9974** | .../C435184.html |
| C179643 | ABM8-25.000MHZ-D2Y-T | 776 | 1 | 1: 0.5415, 10: 0.4296, 30: 0.3818, **100: 0.3226** | .../C179643.html |
| C6491123 | ABM8-12.000MHZ-D2Y-T | 1127 | 1 | 1: 0.5518, 10: 0.4297, 30: 0.3679, **100: 0.3060** | .../C6491123.html |
| C1986116 | ABM8-24.000MHZ-D1G-T | 91 | 1 | 1: 0.7553, 10: 0.6032, 30: 0.5272, **100: 0.4529** | .../C1986116.html |
| C156247 | NX3225GA-16MHZ-STD-CRG-2 | 4690 | 5 | 5: 0.2578, 50: 0.2078, 150: 0.1864 (no 100 step), 500: 0.1597 | .../C156247.html |
| C156248 | NX3225GA-24MHZ-STD-CRG-2 | 5380 | 5 | 5: 0.4111, 50: 0.3267, 150: 0.2905, 500: 0.2454 | .../C156248.html |
| C889706 | NX3225GD-8MHZ-STD-CRA-3 (8 MHz, ±50/±150, -40..+150, LCSC package "SMD3225-2P") | 21485 | 5 | 5: 0.3524, 50: 0.2823, 150: 0.2481, 500: 0.1784 | .../C889706.html |
| C2908117 | NX3225SA-16.000M-STD-CRS-2 (LCSC: ±15 tol, ±50 stab, -40..+125, 8 pF, 120 Ω) | 19267 | 1 | 1: 0.4301, 10: 0.3444, 30: 0.3065, **100: 0.2620** | .../C2908117.html |
| C2762192 | NX3225SA-24MHZ-EXS00A-CS08583 (9 pF) | 5260 | 5 | 5: 0.3256, 50: 0.2570, 150: 0.2276, 500: 0.1910 | .../C2762192.html |
| (not fetched) | ABM8AIG at 12/16/24/25 MHz | — | — | no LCSC listing surfaced in search; unreadable otherwise | — |
| (not fetched) | Epson FA-238/TSX-3225, TXC, Murata | — | — | no LCSC code located | — |

Stock numbers changed between a web-search snippet and my fetch minutes later (e.g. C156247 5850 → 4690); treat all stock as volatile.

## 5. ST AN2867

Obtained from a crystal maker (Raltron), not ST (st.com not contacted — instructed not to).
URL: https://www.raltron.com/webproducts/technical/technical_notes/Oscillator-design-guide-for-STM8S-STM8A-and-STM32-microcontrollers-ST-MICRO-1.pdf (fetched 2026-11-19T14:49:29Z, HTTP 200, 5,629,560 B).
The PDF itself prints: "August 2015, DocID15287 Rev 10, 36 pages, AN2867, Oscillator design guide for STM8S, STM8A and STM32 microcontrollers" on page 1 and every footer. sha256 `93cd3c281d6e1a7b471cdb94d46d9c4e31528d2d79d1a317428272b61b72f0e2`. File: ds/st-AN2867-DocID15287-Rev10.pdf.
**This is an old revision.** A web search snippet (not a held document) reports current AN2867 as Rev 24 (Feb 2026) with a title that includes "MPUs"; Rev 10 predates the STM32H7 family and I did not check it for H7-specific HSE guidance (gm, ESR limits for H7). If the spike needs H7 HSE limits, use the STM32H753 datasheet/RM (not obtained) or ask the operator for the current AN2867.

## 6. Fetch log (UTC; plain curl unless noted; no custom UA, no proxy/mirror)

| URL | UTC time | Result |
|---|---|---|
| https://abracon.com/Resonators/abm8.pdf | 2026-11-19T14:47:18Z | 200 pdf 1535935 (== pre-existing ds/abm8.pdf) |
| https://abracon.com/Resonators/ABM8G.pdf | 14:47:19Z | 200 pdf 1385782 |
| https://abracon.com/Resonators/ABM3B.pdf | 14:47:19Z | 200 pdf (not kept: 5032) |
| https://abracon.com/Resonators/ABM10.pdf | 14:47:20Z | 200 pdf (not kept: 2520) |
| https://abracon.com/Resonators/abm8aig.pdf, ABM8AIG.pdf, ABM8AIG-T.pdf, ABM8-W.pdf, ABM8AIG_.pdf | 14:47:20Z-14:49:40Z | 404 (URL guesses) |
| https://abracon.com/Resonators/ABM8W.pdf | 14:49:38Z | 200 pdf 682306 |
| https://abracon.com/parametric/crystals/ | 14:49:41Z | 403 (Abracon's own site; not pursued) |
| https://abracon.com/AIGcrystals/ABM8AIG.pdf | 14:49:51Z | 200 pdf 746676 |
| https://abracon.com/datasheets/ABM8AAIG.pdf | 14:49:51Z | 200 pdf 436739 |
| https://abracon.com/datasheets/ABM8AIG.pdf | 14:49:51Z | 404 |
| https://download.epsondevice.com/td/pdf/app/FA-238_en.pdf, TSX-3225_en.pdf, FA-238V_en.pdf | 14:47:36-37Z | 404 (guessed path) |
| https://www.epsondevice.com/crystal/en/products/crystal-unit/fa238.html | 14:47:38Z | 200 html (used to find the /brief/ PDF link) |
| https://download.epsondevice.com/td/pdf/brief/FA-238_en.pdf (also TSX-3225_en, FA-238V_en) | 14:47:54Z | 200 pdf 524299 each, identical bytes |
| https://download.epsondevice.com/td/pdf/brief/FA-20H_en.pdf | 14:47:56Z | 200 pdf (not kept; not 3225) |
| https://www.ndk.com/images/products/catalog/c_NX3225GA_e.pdf | 14:47:39Z | 200 pdf 250475 |
| https://www.ndk.com/images/products/catalog/c_NX3225SA_e.pdf / _GD_ / _GB_ | 14:47:56-59Z | 200 pdf (285353 / 251879 / 282218) |
| https://ecsxtal.com/ecs-crystal-ecx-32/ ; /ecs-80-18-33-jgn-tr/ ; store/pdf/ECS-ECX-32.pdf ; store/pdf/ECX-33.pdf | 14:47:41Z-14:48:45Z | 404 (guesses) |
| https://ecsxtal.com/store/pdf/ECX-32.pdf | 14:48:25Z | 200 pdf 585507 |
| https://ecsxtal.com/store/pdf/ECX-33Q.pdf ; ECX-33B2Q.pdf | 14:48:45Z | 200 pdf 414998 ; 411531 |
| https://electronicsdatasheets.com/download/5253d35de34e243c148ce994.pdf?format=pdf | 14:48:27Z | 200 but HTML page, not a PDF; discarded (third-party mirror, not used) |
| https://datasheet.lcsc.com/lcsc/2403291505_YXC-X32258MSB4SI_C2682774.pdf | 14:48:30Z | 200 HTML viewer page, not a PDF |
| https://wmsc.lcsc.com/wmsc/upload/file/pdf/v2/lcsc/2403291505_YXC-X32258MSB4SI_C2682774.pdf (+ two TXC 8Y equivalents) | 14:48:43-44Z | 200, 141-byte HTML redirect stub, no PDF |
| https://datasheet.lcsc.com/datasheet/pdf/a84bd8d530dd46e4b0f6d0ee59d8a89c.pdf?productCode=C2682774 | 14:48:58Z | 200 pdf 507889 (the link LCSC's own stub redirected to) = YXC YSX321SL |
| https://www.mouser.com/datasheet/2/417/8y_accuracy-2256246.pdf ; /8y-11420.pdf | 14:48:31-32Z | curl 92 HTTP/2 INTERNAL_ERROR, no data. **Blocked; TXC 8Y datasheets not obtained.** |
| https://www.txccorp.com/ ; /product/crystal-units/mhz-crystals/ | 14:47:41Z ; 14:49:08Z | 200 html. Page lists 7M (3.2x2.5, 8-80 MHz) / 7V / 8Y (2.0x1.6) but the datasheet download links are `javascript:void(0)`; no PDF. (A WebFetch summary of this page was used only to see that; no figures from it are in the tables.) |
| https://www.raltron.com/.../Oscillator-design-guide-...ST-MICRO-1.pdf | 14:49:29Z | 200 pdf 5629560 |
| https://www.digikey.com/en/products/detail/abracon-llc/ABM8AIG-8-000MHZ-1Z-T/10313751 | 14:50:15Z | 403 Cloudflare challenge |
| https://www.digikey.com/en/products/detail/ecs-inc/ECS-80-18-33-JGN-TR/10478755 | 14:50:15Z | 403 Cloudflare challenge |
| https://www.digikey.com/en/products/detail/abracon-llc/ABM8AIG-24-000MHZ-12-2Z-T3/5817592 | 14:50:16Z | 403 Cloudflare challenge |
| https://www.mouser.com/c/?q=ABM8AIG-8.000MHZ-1Z-T ; ?q=ECS-80-18-33-JGN-TR | 14:50:15Z | curl 92 HTTP/2 INTERNAL_ERROR, no data |
| https://www.lcsc.com/product-detail/C5140594.html ; C2682774 ; C7183312 | 14:50:15-16Z | 200 html, SSR JSON readable |
| https://www.lcsc.com/search?q=ABM8AIG | 14:50:26Z | 200 html, JS-rendered, no results in markup |
| https://www.lcsc.com/product-detail/C5727301 ; C179643 ; C6491123 ; C1986116 ; C6941862 ; C435184 ; C156247 ; C156248 ; C889706 ; C2908117 ; C2762192 (.html) | 14:50:27Z-14:51:03Z | 200 html, SSR JSON readable |
| (WebSearch tool queries, ~9) | 14:46-14:51Z | used only to discover URLs / part names; no figures taken from snippets except where labelled |

Not contacted: st.com (instructed). Murata (no attempt; time). Epson/NDK/TXC distributor pages (no DigiKey/Mouser access anyway).

## 7. Doubts

1. **No part is fully "cited" on every row.** Epson and YXC sheets have no doc number/revision (uncited). Abracon/ECS sheets have a revision date but no document number; NDK sheets have an internal doc code (e.g. 2008E_NX3225GA_e). I treated a printed revision/date as a revision; if the project rule requires a document number, only the NDK rows and ST AN2867 qualify.
2. **8 MHz coverage is thin.** ABM8AIG (±50 over -40..+125 via code Z), ABM8 (D option, std ±50), YXC YSX321SL (uncited), NX3225GD/ECX-33B2Q (±150) are the only held-datasheet 3225 8 MHz options. NDK GA/SA, Epson, ABM8G, ABM8W, ECX-32 do not cover 8 MHz in their datasheets.
3. **ECX-32 vs LCSC 8 MHz listings**: the ECX-32 sheet (Rev.2017) says 10-54 MHz; LCSC lists ECS-80-xx-33-… (8 MHz, 400 Ω). Datasheet and listing disagree; the 8 MHz row cannot be cited to the held sheet.
4. **"33QZ" (LCSC ECS-160/250-18-33QZ-ADS-TR)** is not in the ECX-33Q ordering table; I have not shown which datasheet governs it. Both are out of stock at LCSC (0).
5. **NX3225SA-16.000M-STD-CRS-2** (LCSC: -40..+125, ±50 stab, ±15 tol, 120 Ω) does not match the held NX3225SA sheet (spec STD-CQR-1, -40..+85, ±25 stab, 50-100 Ω). The "CRS-2" spec number is undocumented in my held PDFs. Do not use the LCSC values as datasheet values.
6. **NX3225GD** listed at LCSC as "SMD3225-2P"; GA/SA sheets show 4 pads with #2/#4 NC or lid GND. I did not verify GD's pad connection from its PDF.
7. **Drive level column ambiguity** for Abracon tables ("10" and "100" in a Min/Typ/Max table). I report the larger as the max; the 10 may be a typical.
8. **YXC part mapping**: X32258MSB4SI → YSX321SL is inferred from LCSC's own datasheet link; the YXC PDF has no ordering table, so the "MSB4SI" suffix is not decoded. LCSC's own page shows 120 Ω (title) and 180 Ω (attributes) for the same part.
9. **Abracon ABM8G text layer is scrambled** in places; its values were not re-checked visually. Not recommended anyway (12 MHz min, -10..+60 std).
10. ABM8AIG 12/16/24/25 MHz orderable strings are constructed from the ordering table; the ABM8AIG-24.000MHZ-12-2Z-T3 / -16.000MHZ-12-2Z-T3 / -12.000MHZ-1Z-T names appeared in web-search result titles (DigiKey) but I could not open those pages (403). Stock/price for them is unknown.
11. AN2867 copy is Rev 10 (2015), well behind the current rev; not an H7-era guide.
12. No pullability, C1, or load-drift data is in any held sheet except ABM8W p5 (4/6 pF, 10/27/50 MHz). Gain-margin / drive-level calculation for the H753 HSE needs ST's H753 datasheet figures, which I did not obtain.
13. I did not render pad/dimension drawings for most sheets; "4-pad" comes from pad text (ECS, ABM8AIG, Epson, NDK GA/SA, YXC) or the Abracon outline drawings' package title, not from a measured drawing.
14. TXC (7M/7V/8Y) and Murata have no row at all. Two manufacturers requirement is met (Abracon, ECS, NDK, Epson, YXC all with held PDFs).
