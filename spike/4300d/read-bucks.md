# Buck-converter datasheet read for AET-WO-4300d (power tree)

Read-only extraction. Page numbers are the PDF page index (= the printed page number for TI parts; Diodes prints "N of 18", same).
Every number is labelled min / typ / max. "typ only" = the table gives no guaranteed bound.
Where the text extraction scrambled columns I rendered the page and read the image (marked **[R]**).
Computations are mine (marked **[calc]**), not the datasheet's.
**Test-condition caveat that applies to every LMR33630 min/max below:** the Electrical (7.5) and Timing (7.6) tables are stated at **VIN = 12 V, VEN = 4 V**, over TJ -40..+125 C (p7-8 headers). The same note applies to the other parts' tables at their stated conditions (TPS62933: VIN 3.8-30 V, TJ -40..150; LMR51430: VIN 4.5-36 V, MIN/MAX tested at 25 C; AP6320x: VIN 12 V, TA 25 C typ). A min/max at VIN = 12 V is NOT a guarantee at VIN = 26.1 V or 6.0 V; the tON/tOFF comparisons in section 5 use it as the best available bound.

Application target used throughout: 2S-6S LiPo, VIN 6.0 V (3.0 V/cell, 2S) to 26.1 V (6S LiHV, 4.35 V/cell);
rail (a) 5 V / up to 3 A; rail (b) 3.3 V / about 1 A.  D = VOUT/VIN: (a) 0.833 at 6.0 V, 0.1916 at 26.1 V; (b) 0.550 at 6.0 V, 0.1264 at 26.1 V.

---------------------------------------------------------------------------------------------------

## 1. LMR33630ADDA (TI) - PRIMARY

### 1.1 Identity
- Doc id: **SNVSAN3F**, "AUGUST 2017 - REVISED NOVEMBER 2020" = **Revision F, November 2020** (p1-2 headers; rev history p2-3). Page footers say Copyright 2025; orderable addendum is dated 9-Nov-2025 (p41).
- Datasheet covers the whole family LMR33630 (A/B/C x DDA/RNX). Many numbers are **per package**; DDA values below.
- Orderable (p41 PACKAGE OPTION ADDENDUM): **LMR33630ADDA** (tube, 75, Active, "SO PowerPAD (DDA) 8", MSL Level-2-260C-1 YEAR, Op temp -40 to 125 C, top mark 33630A); LMR33630ADDAR (T&R 2500); also ...ADDAG4 variants. Device Comparison Table (p4): A = 400 kHz, B = 1400 kHz, C = 2100 kHz, all 3 A, adjustable VOUT.
- Package: DDA0008J, PowerPAD SOIC-8, 1.7 mm max height (p54). Body 4.8-5.0 x 3.8-4.0 mm (6.2/5.8 incl. leads), pitch 1.27 mm. **[R p54]** Exposed thermal pad 2.0-2.6 mm (W) x 2.5-3.1 mm (L) (min-max range as drawn). MS-012 variation BA.

### 1.2 VIN
- Recommended operating (p6, 7.3): VIN **min 3.8, max 36 V**. (Operating TJ -40 to 125 C.)
- Absolute max (p6, 7.1): VIN to PGND **-0.3 / 38 V** (max). EN to AGND -0.3 / VIN+0.3. SW to PGND -0.3 / VIN+0.3, < 100 ns transients -3.5 / 38. TJ max 150 C (125 C recommended; >125 C degrades lifetime, note 3).
- Electrical table (p7) "VIN Minimum operating input voltage": **max 3.8 V** (only a max given). VCC LDO spec is at 6 V <= VIN <= 36 V.

### 1.3 Output
- VOUT adjustable **1 V min to 24 V max** (p6, 7.3). Footnote 3: max can be extended to 95% of VIN, "contact TI for details".
- VFB (p7, **[R]**): **min 0.985, typ 1.0, max 1.015 V** (= +/-1.5%), valid over TJ -40 to +125 C ("Limits apply over ... TJ range of -40C to +125C"; min/max "specified through test, design or statistical correlation").
- IFB (p7): typ 0.2 nA, max 50 nA at FB = 1 V.
- System-level regulation (p9, 7.7, **[R]**; "typical components, over TJ -40..125 C, NOT ensured by production testing"):
  - VOUT = 5 V, **VIN = 7 V to 36 V**, IOUT 0 A to max: min -1.5 %, max +2.5 %; IOUT 1 A to max: -1.5 % / +1.5 %.
  - VOUT = 3.3 V, VIN = 3.8 V to 36 V: IOUT 0 A to max: -1.5 % / +2.5 %; 1 A to max: -1.5 % / +1.5 %.
  - Deviation is relative to VIN = 12 V, IOUT = 1 A. NOTE: the 5 V regulation spec starts at VIN = 7 V, not 6 V.

### 1.4 Current
- IOUT (p6, 7.3): 0 to **3 A** (max). Variant letters here are frequency (A/B/C), not current. The device table (p4) lists only LMR33630 (3 A); the thermal table header (p7) says "LMR336x0", so lower-current siblings exist in the family but are not specified in this document.

### 1.5 UVLO / EN
- No VIN UVLO table row. p15 (8.3.4), text, "about"/typ only: UVLO is on the internal LDO (VCC): device ready when VCC reaches **about 3.7 V**, shuts down when VCC falls below **about 3 V**; "LDO is in dropout during these transitions, so these values roughly represent the input voltage". typ only; no min/max.
- EN (p7, **[R]**): VEN-H rising (start switching) **min 1.2, typ 1.231, max 1.26 V**; hysteresis VEN-HYS typ 100 mV (falling, below VEN-H); VEN-VCC-H (turn on internal LDO, rising) **max 1 V**; VEN-VCC-L (turn off LDO, falling) **min 0.3 V**; ILKG-EN typ 0.2 nA at VEN = 3.3 V. EN must not float; may tie to VIN (p5 pin table, p13).
- External UVLO divider: RENB 10-100 k, RENT = (VON/VEN-H - 1)*RENB; VOFF = VON*(1 - VEN-HYS/VEN-H) (p24, Eq.10).
- VCC LDO (p7): 6 V <= VIN <= 36 V: **min 4.75, typ 5, max 5.25 V**. VBOOT-UVLO typ 2.2 V.

### 1.6 Switching frequency (p8, **[R]**, "OSCILLATOR")
| Variant | min | typ | max |
|---|---|---|---|
| A | 340 kHz | 400 kHz | 460 kHz |
| B | 1.2 MHz | 1.4 MHz | 1.6 MHz |
| C, DDA | 1.8 MHz | 2.1 MHz | 2.4 MHz |
| C, RNX | 1.8 MHz | 2.1 MHz | 2.3 MHz |
Auto mode PFM at light load (frequency folds; p15-16).

### 1.7 Timing (p8, 7.6, **[R]** - columns are MIN / NOM / MAX; no value in MIN column)
| Parameter | RNX | **DDA (our package)** |
|---|---|---|
| tON-MIN minimum switch on-time | nom 68, max 80 ns | **nom 75, max 108 ns** |
| tOFF-MIN minimum switch off-time | nom 52, max 70 ns | **nom 50, max 85 ns** |
| tON-MAX maximum on-time | nom 7, max 9 us (both pkgs) | nom 7, max 9 us |
- Conditions: these timing values are given at **VIN = 12 V, VEN = 4 V**; no VIN dependence is tabulated. Frequency-foldback figures (Fig 8-13, p18) show the effective on-time behaviour vs VIN as typ plots only.
- The Features list on p1 says "Short minimum on-time of 68 ns" and "75-mOhm/50-mOhm RDS-ON": those are the RNX numbers. **The DDA is 75 ns nom / 108 ns max and 95/66 mOhm typ (below).**
- DMAX (p9, 7.7, **[R]**, not production tested): **typ 98 %** at VIN = VOUT = 12 V, IOUT = 1 A. Footnote 2: in dropout fSW falls to about fMIN = 1/(tON-MAX + tOFF-MIN); DMAX = tON-MAX/(tON-MAX + tOFF-MIN). No guaranteed DMAX min (tON-MAX has no min given).
- Dropout (p9, **[R]**): VDROP (VIN-VOUT) **typ 150 mV** at VOUT = 5 V, IOUT = 1 A, "dropout at -1% of regulation", fSW = 140 kHz (foldback clamp). typ only, not ensured by production test. p16 (8.4.2): dropout defined at -1% output, fSW then at "about 140 kHz".
- Fig 8-10 (p17) typ dropout voltage vs IOUT at 140 kHz foldback: about 0.42 V at 3 A (read off the curve, **[calc/R]**, typ only). Fig 8-9 (p17, **[R]**) VOUT=5 V: the 3 A curve reaches about 4.95 V at VIN 5.4 V and flattens at about 5.0 V by VIN 5.6 V (typ). So at VIN = 6.0 V typ margin is about 0.4 V; not a guarantee.
- Min on-time behaviour (p17, 8.4.3): when tON-MIN is reached the controller folds frequency back and keeps VOUT in regulation. Eq.2: VIN <= VOUT/(tON * fSW) before foldback.

### 1.8 MOSFETs (p8, **[R]**, typ and max only; TJ -40..125 C limits, typ at 25 C)
| | RNX | **DDA** |
|---|---|---|
| RDS-ON-HS | typ 75, max 145 mOhm | **typ 95, max 160 mOhm** |
| RDS-ON-LS | typ 50, max 95 mOhm | **typ 66, max 110 mOhm** |

### 1.9 Current limit (p7-8, **[R]**; tested open loop in production, note 4)
- ISC high-side peak limit: **min 3.85, typ 4.5, max 5.05 A**.
- ILIMIT low-side (valley) limit: **min 2.9, typ 3.5, max 4.1 A**.
- IPEAK-MIN minimum peak inductor current: typ 0.69 A. IZC zero-current detector: typ -0.106 A. tSS soft start: min 2.9, typ 4, max 6 ms. Hiccup: FB < 0.4 V (typ), tHC typ 94 ms (p9). Max output current approx (ISC+ILIMIT)/2 (Eq.1, p15).

### 1.10 Quiescent / shutdown (p7, **[R]**, VIN = 12 V, VEN = 4 V)
- IQ non-switching (VFB = 1.2 V): **typ 24, max 34 uA**. ISUPPLY in regulation (p9, system, VIN 12 V, VOUT 3.3 V, RFBT = 1 MOhm): typ 25 uA.
- ISD shutdown (EN = 0): **typ 5, max 10 uA**.

### 1.11 Thermal (p7, 7.4, **[R]**) DDA (HSOIC-8): RthJA **42.9** C/W (JEDEC 4-layer; "only valid for comparison ... cannot be used for design"), RthJC(top) **54**, RthJB 13.6, psiJT 4.3, psiJB **13.8**, RthJC(bot) 4.3 C/W. (RNX: 72.5 / 35.9 / 23.3 / 0.8 / 23.5 / N/A.)
- Junction temp: recommended/spec'd -40 to +125 C; abs-max 150 C (p6). Thermal shutdown typ 165 C, recovery typ 148 C (p9, not ensured).
- p25 Fig 9-3: typical RthJA vs copper area for DDA 4-layer about 20-44 C/W (curve; typ). Fig 9-5/9-6: max IOUT vs ambient at RthJA 30 / 50 C/W (VIN 12 V, VOUT 5 V, 400 kHz).
- Design guidance p24 Eq.11: max TJ must be limited to 125 C; do not use p7 RthJA for design.

### 1.12 Efficiency
- **No efficiency min/max/typ in any electrical table.** Only: Features (p1) "Peak efficiency > 95%" (headline, no conditions); typical curves (p1 figure; p27-30 Figs 9-7..9-16 at various VIN/VOUT/fSW). Eq.11 (p24) tells the user to estimate efficiency from the curves or measure on the EVM. => typical curves only.

### 1.13 External parts and their STATED ranges
- **Inductor** (p21-22, 9.2.2.4): L from Eq.4: L = (VOUT/(fSW*K*IOUTmax)) * ((VIN-VOUT)/VIN)  [equation extracted, symbols scrambled; consistent with Eq 11 form]. K = 20-40 % of max load current, "best value 30 %", use the device max (3 A) when load is much smaller. **Eq.5 as printed on p22 [R]: LMIN >= 0.28 * VOUT / fSW** (3.5 uH at 5 V/400 kHz if fSW in MHz and L in uH - my unit reading [calc]; 2.3 uH at 3.3 V). **Datasheet mismatch:** the sentence right above Eq.5 says "the *maximum* inductance is limited by the minimum current ripple required ... minimum inductor ripple current must be no less than about 10 % of the device maximum rated current (3 A)", i.e. a ripple FLOOR (ΔIL >= about 0.3 A) which bounds L from above; Eq.5 is labelled LMIN. I could not reconcile the two; both are recorded as written. [calc] ripple floor check at the VIN = 6.0 V corner (smallest ripple), 5 V, ΔIL = VOUT(1-D)/(L fSW): L = 8 uH: 0.31/0.26/0.23 A (10.2/8.7/7.5 % of 3 A) at 340/400/460 kHz; L = 11.2 uH (the 30 %-at-26.1 V value): 0.22/0.19/0.16 A (7.3/6.2/5.4 %), i.e. BELOW the 10 % rule-of-thumb at 6.0 V. At 26.1 V L = 8 uH gives 1.26 A (42 %) at 400 kHz, peak 3.63 A < 3.85 A min HS limit. So an inductor sized for 30 % ripple at 26.1 V violates the 10 % floor at 6.0 V; the L choice for a 6-26.1 V range is a trade-off the datasheet does not resolve (TI's own 8 uH example is computed at VIN = 12 V, Eq.4 gives 8.1 uH). 3.3 V/1 A rail at 400 kHz: L = 8 uH gives 15.5 % (6.0 V) to 30 % (26.1 V) of 3 A. Saturation: "Ideally at least ISC (high-side limit, 5.05 A max)"; "In any case must not be less than ILIMIT (low-side limit)". Ferrite hard saturation, powdered iron soft.
  - **[calc]** Eq.4 at VIN = 26.1 V: 5 V/3 A, K = 0.3, 400 kHz: L = 11.2 uH (13.2 uH at 340 kHz). TI's own 5 V/400 kHz example: 8 uH (p20, Fig 9-1, computed at 12 V).
- **Output capacitance** (p22-23, 9.2.2.5): lower bound from Eq.6 (transient driven); example 52 uF min -> 72 uF "minimum capacitance" after 20 % tolerance and 10 % bias derate -> 4 x 22 uF 16 V 1210. "Maximum value of total output capacitance must be limited to about 10 times the design value, or 1000 uF, whichever is smaller" (p23). Use >= 10 V rated for VOUT <= 3.3 V and >= 16 V for VOUT >= 5 V. Definition p19: "effective value of capacitance is the actual capacitance under D.C. bias and temperature, not the rated or nameplate values" - the p19 note applies to the whole datasheet; **Table 9-2 itself is labelled "COUT (RATED CAPACITANCE)"** i.e. nameplate, so effective C is lower. No separate stated stability minimum.
- **Input capacitance** (p23, 9.2.2.6): "A **minimum of 10 uF** of ceramic capacitance is required on the input", rated at least max VIN, preferably 2x; plus "a small case size **220 nF** ceramic must be used at the input, as close as possible to the regulator". Example p23: 4.7 uF 50 V X7R (example text); Fig 9-1 / Table 9-2 use 10 uF + 220 nF. X5R or better (p19). ICIN,RMS ~ IOUT/2 (Eq.8). Optional 20-100 uF electrolytic/tantalum in parallel for long leads (p23, p32).
- **CBOOT** (p23, 9.2.2.7): **100 nF, >= 10 V**, high-quality ceramic, BOOT to SW (pin 7 to pin 8).
- **CVCC** (p23): **1 uF, 16 V** ceramic VCC to GND; do not load VCC externally (may pull up PG through 100 k).
- **FB divider** (p21, 9.2.2.3): RFBT recommended **100 kOhm, maximum 1 MOhm**; if 1 MOhm a feedforward cap is required. RFBB = RFBT/(VOUT/VREF - 1) (Eq.3), VREF nominally 1 V. 5 V: 100 k / 24.9 k; 3.3 V: 100 k / 43.2 k (Table 9-2, p20 **[R]**).
- **CFF** (p23-24): optional; "especially when RFBT > 100 k"; Eq.9: CFF < (VOUT*COUT)/(120*RFBT*sqrt(VREF/VOUT)) (**[R p24]**). Table 9-2 lists CFF = "open".
- **Table 9-2 Typical External Component Values (p20, [R])** - columns fSW | VOUT | L | COUT (rated) | RFBT | RFBB | CIN+CHF | CBOOT | CVCC | CFF:
  - 400 kHz, 3.3 V: 6.8 uH | 4 x 22 uF | 100 k | 43.2 k | 10 uF + 220 nF | 100 nF | 1 uF | open
  - 1400 kHz, 3.3 V: 2.2 uH | 2 x 22 uF | 100 k | 43.2 k | same | same | same | open
  - 2100 kHz, 3.3 V: 1.2 uH | 2 x 22 uF | 100 k | 43.2 k | ...
  - **400 kHz, 5 V: 8 uH | 4 x 22 uF | 100 k | 24.9 k | 10 uF + 220 nF | 100 nF | 1 uF | open**
  - 1400 kHz, 5 V: 2.2 uH | 2 x 22 uF | 100 k | 24.9 k | ...
  - 2100 kHz, 5 V: 1.5 uH | 2 x 22 uF | 100 k | 24.9 k | ...
  - 400 kHz, 12 V: 15 uH | 4 x 22 uF | 100 k | 9.09 k; 1400 kHz 12 V: 4.7 uH | 4 x 10 uF; 2100 kHz 12 V: 3.3 uH | 4 x 10 uF.
  - [calc] The table values correspond to about 12 V VIN (Eq.4 gives 8.1 uH at 12 V); at VIN = 26.1 V Eq.4 gives larger L (see ripple-floor note above). TI does not state the table's VIN.
- PG: open drain, pull up to VCC or VOUT through 100 k; limit current to < 5 mA (p12).

### 1.14 Layout guidelines (p33-34, 10.1; Figs 10-1..10-3 p34-36)
1. Input capacitor(s) as close as possible to VIN and GND terminals; VIN and PGND pins are adjacent (HSOIC pins 2/1). (VQFN: two VIN/PGND pairs, wide VIN plane on a lower layer.)
2. VCC bypass capacitor close to the VCC pin, short wide traces to VCC and GND.
3. Wide traces for CBOOT; CBOOT close to the device, short/wide to BOOT and SW. (VQFN-only: route SW under the device to the NC pin.)
4. Feedback divider as close as possible to FB; RFBB, RFBT, CFF physically close; connections to FB and GND short; the VOUT connection can be longer but must not be routed near noise sources such as SW.
5. At least one ground plane in a middle layer (noise shield and heat path).
6. Connect the thermal pad to the ground plane; SOIC pad must be soldered down; solder integrity directly affects RthJA.
7. Wide paths for VIN, VOUT, GND.
8. Enough PCB area for heat sinking; top and bottom layers 2 oz copper, no less than 1 oz; with SOIC use an array of heat-sinking vias from the PAD to the bottom ground plane (and inner planes).
9. Keep switch area small: SW-to-inductor copper short and wide, total node area minimized.
- 10.1.1 (p34): AGND and PGND connect to ground planes through vias next to the bypass capacitors; PGND, VIN and SW traces constrained to one side of the ground plane; **minimum 4 x 3 array of 10-mil thermal vias** under the PAD, evenly distributed; use a 4-layer board with 2 oz / 1 oz / 1 oz / 2 oz.
- p32: input must not fall below output (shorted-input test) - use a Schottky from input to output if likely; avoid snap-back (thyristor) TVS.

### 1.15 Pin table (p5, Table 6-1, HSOIC DDA; type A = analog, P = power, G = ground)
| Pin | Name | Type | Notes |
|---|---|---|---|
| 1 | PGND | G | power ground, connect to system ground and AGND, to bypass cap with short wide traces |
| 2 | VIN | P | input supply, bypass caps directly to this pin and PGND |
| 3 | EN | A | High = ON; may tie to VIN; do not float |
| 4 | PG | A | open-drain power-good, pull-up through current-limiting resistor; can be left open |
| 5 | FB | A | feedback input; **this is the output-sense pin (divider tap); do not float, do not ground** |
| 6 | VCC | P | internal 5 V LDO; 1 uF to GND; no external loads (PG pull-up allowed) |
| 7 | BOOT | P | 100 nF to SW |
| 8 | SW | P | to inductor |
| THERMAL PAD (EP) | AGND | G | analog ground; "all electrical parameters measured with respect to this pin"; must be soldered to ground plane; no pin number in the table (VQFN lists it as pin 6) |
There is no VOUT-sense pin on the HSOIC; regulated output is sensed only through FB (VREF 1.0 V). (VQFN adds pin 3 NC, pin 11 PGND, pin 10 VIN.)

### 1.16 Package / KiCad name
Orderable matches the KiCad symbol name: **LMR33630ADDA** (DDA0008J, HSOIC-8 with exposed pad, p41, p54). Reel variant LMR33630ADDAR.

---------------------------------------------------------------------------------------------------

## 2. TPS62933 (TI)  (family: TPS62932 / 62933 / 62933F / 62933P / 62933O)

### 2.1 Identity
- **SLUSEA4D**, "JUNE 2021 - REVISED AUGUST 2022" = **Revision D, August 2022** (p1-3). Orderable addendum dated 31-Oct-2025 (p44).
- Plain **TPS62933** = 3 A, PFM, with SS pin (device table p3). P = PG pin PFM, F = FCCM with SS, O = out-of-audio with PG. Orderable: **TPS62933DRLR** (T&R 4000, "SOT-5X3 (DRL) 8", MSL Level-1-260C-UNLIM, -40 to 150 C, mark 2933) (p44). Package drawing DRL0008A (p45+). Body 1.6 x 2.1 mm (p1). No exposed pad.

### 2.2 VIN
- Recommended (p5, 8.3): **min 3.8, max 30 V**. Absolute max (p5, 8.1): VIN -0.3 / **32 V**; SW DC -0.3 / 32, SW transient < 10 ns -3 / 33.
- (EN abs max 6 V and recommended max 5.5 V: EN must not be tied directly to a 26 V pack; leave it floating (internal pull-up enables, p3/p20) or use a divider; same 5.5 V limit for FB.)

### 2.3 Output
- VOUT 0.8 to 22 V (p5; p1). VFB (p6, **[R]**): TJ = 25 C: **min 792, typ 800, max 808 mV (+/-1 %)**; TJ = 0 to 85 C: 788 / 800 / 812; TJ = -40 to 150 C: **784 / 800 / 816 mV (+/-2 %)**. IFB input leakage: **max 0.15 uA** (VFB = 0.8 V; MAX column only).

### 2.4 Current
- IOUT (p5): TPS62933/933x 0 to **3 A** (max); TPS62932 0-2 A.

### 2.5 UVLO / EN (p6, **[R]**)
- VIN_UVLO rising: **min 3.4, typ 3.6, max 3.8 V**; falling: **min 3.1, typ 3.3, max 3.5 V**; hysteresis typ 300 mV.
- EN rising VEN_RISE: **typ 1.21, max 1.28 V** (no min); falling VEN_FALL: **min 1.1, typ 1.17 V** (no max). EN pullup Ip typ 0.7 uA (VEN = 1.0 V), hysteresis current Ih typ 1.4 uA (VEN = 1.5 V). EN floating = enabled (p20). External UVLO: hysteresis > 500 mV recommended; VEN must stay < 5.5 V at max VIN (p21).

### 2.6 Switching frequency (p7, **[R]**)
RT floating: **min 450, typ 500, max 550 kHz**; RT = GND: **min 1000, typ 1200, max 1350 kHz**; RT = 71.5 kOhm: typ 310 kHz; RT = 9.09 kOhm: typ 2100 kHz (typ only). Range 200-2200 kHz via RT (p19-20). Frequency spread spectrum +/-6 % (typ, fm = fSW/128) on PFM parts (p7-8). TPS62933O only: fSW_min typ 30 kHz.

### 2.7 On/off time, duty, dropout (p7 **[R]**, footnote 1 "not production tested, specified by design")
- tON_MIN **typ 70 ns only**; tOFF_MIN **typ 140 ns only**; tON_MAX **typ 7 us only**. No min/max on any of the three.
- Max duty: Features (p1) "Maximum 98% duty cycle operation" (no conditions/limit type); p16 same. Lowest fSW in dropout about 140 kHz = period tON_MAX + tOFF_MIN = 7.14 us (p27).
- Eq 7-10 (p22-23): DMIN = tON_MIN*fSW; DMAX = 1 - tOFF_MIN*fSW; VIN_MAX = VOUT/(fSW*tON_MIN); VIN_MIN = VOUT/(1 - fSW*tOFF_MIN) (without foldback). With foldback (p20, p27) the output remains in regulation "except for current limit operation".
- Dropout: no table value. p27 Fig 9-9 (**[R]**) typ curve VOUT = 5 V: with IOUT = 3 A: about 3.45 V at VIN 4 V, 4.47 V at 5 V, 4.91 V at 5.5 V, **about 5.08 V at VIN 6.0 V (knee at 6.0 V)**; with 0.5 A: regulated from about VIN 5.5 V. typ only. Design example (p28) lists VIN min 5.5 V for 5 V/3 A with VSTART 8 V.

### 2.8 MOSFETs (p6): RDSON_HS **typ 76 mOhm**, RDSON_LS **typ 32 mOhm** (TJ = 25 C, VBST-SW = 5 V). **typ only, no max.**

### 2.9 Current limit (p6-7, **[R]**) TPS62933/x: IHS_LIMIT **min 4.2, typ 5, max 5.8 A**; ILS_LIMIT (valley) **min 2.9, typ 3.8, max 4.5 A**; IPEAK_MIN typ 0.75 A. (TPS62932: 2.8/3.4/4 and 2/2.5/3.) UVP 65 %; hiccup on 256 us, off 10.5 x tSS (p7, p25).

### 2.10 Quiescent / shutdown (p6, **[R]**): IQ (EN = 5 V, VFB = 0.85 V, TPS62933 and P) **typ 12 uA only**; F version typ 125 uA; O version typ 45 uA. ISHDN **typ 2 uA only**.

### 2.11 Thermal (p6, 8.4, **[R]**): DRL JEDEC RthJA **112.2**, RthJC(top) **29.1**, RthJB 19.3, psiJT **1.6**, psiJB **19.2** C/W; RthJA on official EVM (2-layer, 2 oz) 60.2 C/W. TJ -40 to 150 C (recommended; abs-max operating TJ 150). TSD typ 165 C, hysteresis typ 30 C (design).

### 2.12 Efficiency: **No efficiency number in any table.** Typical curves only (p1, Typical Characteristics p9-15). Eq.24 (p33) says estimate efficiency from the curves.

### 2.13 External parts
- **FB divider** (p19, 9.3.4): RFBB = 10 kOhm and **RFBT in the range 10 kOhm to 300 kOhm recommended**; RFBT = RFBB*(VOUT/VREF - 1) (Eq.1), VREF 0.8 V; 1 % resistors. p29: example R7 = 10.2 k, R6 = 53.6 k for 5 V.
- **Inductor** (p30-31): K (ripple ratio) 20-60 %, "best 40 %", Eq.16 L = (VOUT/(fSW*K*IOUTmax))*((VIN-VOUT)/VIN) using VIN_MAX. Saturation: "ideally at least IHS_LIMIT (5.8 A max); in any case not less than max peak inductor current at full load" (Eq.17). Minimum ripple about 10 % of 3 A for subharmonic stability. Example 6.8 uH, Isat 10 A, Irms 6.5 A, K 0.408.
- **Output capacitance** (p31-32): Table 10-2 gives **effective** capacitance ("These values are the effective figures"): Table 10-2 (**[R p32]**, VOUT | fSW | RTOP kOhm | RDOWN kOhm | Typical L | Typical effective COUT | Minimum effective COUT):
  - 3.3 V | 500 kHz | 31.3 | 10.0 | 4.7 uH | 40 uF | 15 uF
  - 3.3 V | 1200 kHz | 31.3 | 10.0 | 2.2 uH | 30 uF | 10 uF
  - **5 V | 500 kHz | 52.5 | 10.0 | 6.8 uH | 20 uF | 10 uF**
  - 5 V | 1200 kHz | 52.5 | 10.0 | 3.3 uH | 20 uF | 10 uF
  - 12 V | 500 kHz | 140.0 | 10.0 | 12 uH | 15 uF | 10 uF
  Minimum values require bode and transient testing over VIN/IOUT/temperature. Max COUT: refer to the TPS62933 Thermal Performance app report (not given in the datasheet) (p32).
- **Input**: 10 uF decoupling typical + 0.1 uF at VIN/GND (p32); X5R/X7R, effective capacitance after DC bias (example 3.45 uF effective for a 10 uF 50 V X7R at 24 V); rating above max VIN.
- **CBST**: 0.1 uF ceramic, X5R or better, >= 16 V; optional resistor < 10 Ohm in series (p29).
- **SS capacitor**: minimum 6.8 nF, do not float (SS pin 7) (p4, p29); 33 nF = about 5 ms.
- **CFF**: optional; example 10 pF (p33); "especially when RFBT > 100 k".
- RT pin: floating = 500 kHz, tie to GND = 1.2 MHz (p4, p20).

### 2.14 Layout (p40, 12.1)
2-layer 2 oz recommended; inductor, input/output caps and IC on the same layer; input and output caps as close as possible, wide VIN/GND with vias; 0.1 uF decoupling cap(s) as close as possible to VIN and GND pins ("key to EMI reduction"); SW trace short and wide; BST capacitor and resistor close to BST and SW, > 10 mil trace; feedback divider close to FB, > 10 mil trace, separate VOUT trace to the upper feedback resistor, feedback loop away from the high-voltage switching trace and preferably ground-shielded; SS capacitor and RT resistor close to the IC with minimal trace length. Fig 12-1 current-loop figure.

### 2.15 Pin table (p3-4, Table 7-1; 8-pin SOT583 DRL; type A = analog, P = power, G = ground)
| Pin | Name | Type | Note |
|---|---|---|---|
| 1 | RT | A | float = 500 kHz, GND = 1.2 MHz, or resistor to GND |
| 2 | EN | A | floating = enabled |
| 3 | VIN | P | |
| 4 | GND | G | |
| 5 | SW | P | |
| 6 | BST | P | 100 nF to SW |
| 7 | SS (TPS62932/62933/62933F) or PG (TPS62933P/62933O) | A | SS: min 6.8 nF, do not float |
| 8 | FB | A | output feedback = the sense pin |
No exposed pad (only the 8 pins).

---------------------------------------------------------------------------------------------------

## 3. LMR51430 (TI)

### 3.1 Identity
- **SLUSEF4A**, "JUNE 2022 - REVISED NOVEMBER 2022" = **Revision A, November 2022** (p1-2 footers). Addendum dated 31-Oct-2025.
- Orderable (p3, p26): LMR51430**X**DDCR (500 kHz PFM), LMR51430**XF**DDCR (500 kHz FPWM), LMR51430**Y**DDCR (1.1 MHz PFM), LMR51430**YF**DDCR (1.1 MHz FPWM); package **SOT-23-THIN DDC0006A, 6 pins**, 3000 T&R, MSL 1, top mark e.g. 4BXP. **There is no bare "LMR51430" orderable.**
- **Datasheet internal inconsistency**: p1 Device Information and the Figure 6-1 caption say "DBV (SOT-23, 6) 2.90 x 1.60 mm"; the thermal table (p5) says "DDC (SOT-23-6)", and every orderable is DDC. DDC (SOT-23-THIN) is the real package; the DBV text is a leftover.

### 3.2 VIN
Recommended (p4, **7.3**): **min 4.5, max 36 V**. Absolute max (p4): VIN -0.3 / **38 V**; SW -0.3 / 38. EN -0.3 / VIN + 0.3. TJ -40 to 150 C.

### 3.3 Output
VOUT **0.6 V min to 95 % of VIN max** (p4). VFB (p5, **[R]**): **min 0.591, typ 0.6, max 0.609 V** (+/-1.5 %), TJ -40 to +150 C; footnote (p6): "MIN and MAX limits are 100% production tested at 25 C. Limits over the operating temperature range verified through correlation using SQC". System regulation (p6): -1.5 % / +2.5 % (PFM, VIN = 24 V, IOUT 0 to full load; not production tested).

### 3.4 Current: 3 A max continuous (p4), can be derated at high fSW / ambient (note 4).

### 3.5 UVLO / EN (p5, **[R]**): VIN UVLO rising **typ 3.89, max 4.5 V** (no min); falling **min 3.35, typ 3.58 V**; hysteresis typ 0.3 V. EN rising **min 1.1, typ 1.227, max 1.36 V**; EN falling **min 0.95, typ 1.08, max 1.22 V**; IEN(P2) typ 10 nA, max 200 nA. EN must not float (disabled if floating); EN <= VIN + 0.3 V (p3, p12).

### 3.6 Switching frequency (p5, **[R]**): 500 kHz option **min 450, typ 500, max 560 kHz**; 1.1 MHz option **min 0.95, typ 1.1, max 1.25 MHz**.

### 3.7 On/off time, duty, dropout
- tON(min) **typ 70 ns only** (VIN = 12 V, IOUT = 3 A); tON(max) typ 6.76 us; tOFF(min) **typ 150 ns only** (VIN = 4.5 V) (p5, **[R]**).
- DMAX **typ 98 %** (p6 system char., not production tested). Frequency can fall to about 133 kHz in foldback (p13). "Lower effective dropout" statement is qualitative.
- Dropout: no table value. Fig 7-6 (p7, **[R]**, zoomed) VOUT = 5 V, fSW 500 kHz, PFM, typ: IOUT = 3 A regulates (about 5.0 V) only from VIN **about 6.5 V**; at VIN = 6.0 V, 3 A: VOUT about 4.6 V. 2 A: knee about 5.9 V; 1 A: about 5.5 V.

### 3.8 MOSFETs (p5): RDSON(HS) **typ 0.12 Ohm**, RDSON(LS) **typ 0.07 Ohm** (TJ = 25 C). typ only.
### 3.9 Current limit (p5-6, **[R]**; open loop, tested in production): HS peak **min 3.67, typ 4.76, max 6.68 A**; LS valley **min 2.75, typ 3.5, max 4.2 A**; FPWM LS negative -1.6 A typ; IZC typ 0.02 A. Hiccup: FB < 40 % VREF (system table VHC typ 0.24 V), tHICCUP typ 135 ms (p14).
### 3.10 Quiescent: IQ(VIN) PFM variant, VEN = 3 V: **typ 40, max 65 uA**; ISD (VEN = 0) **typ 3, max 15 uA** (p5).
### 3.11 Thermal (p5): DDC (SOT-23-6) RthJA **107.8**, RthJC(top) **52.4**, RthJB 23.3, psiJT **9.3**, psiJB **23.0** C/W; "a 2-layer PCB, RthJA = 80 C/W can be achieved". TJ -40..150 C; TSD typ 163 C, hysteresis 22 C (design).
### 3.12 Efficiency: **No efficiency in tables**; typical curves only (p1, Figs 7-1..7-3, p7).
### 3.13 External parts
- FB: RFBT/RFBB per Eq.3: RFBT = RFBB*(VOUT/VREF - 1), VREF 0.6 V; Table 9-1/9-2 use RFBT = **100 kOhm** (RFBB 13.7 k for 5 V, 22.1 k for 3.3 V, 5.23 k for 12 V). No stated min/max for RFBT.
- Inductor (p18): L_MIN = ((VIN_MAX - VOUT)/(IOUT*K_IND))*(VOUT/(VIN_MAX*fSW)); K_IND 20-60 %, example 0.3; saturation current "must be higher than peak current limit level" (HS limit max 6.68 A).
- COUT: no stated min/max; Tables 9-1/9-2 give **ceramic** (nameplate) values: 500 kHz: 3.3 V 5.6 uH / 2 x 22 uF 25 V; **5 V 6.8 uH / 2 x 22 uF / 25 V**; 12 V 12 uH / 3 x 22 uF. 1.1 MHz: 3.3 V 2.2 uH / 2 x 10 uF; 5 V 3.3 uH / 2 x 10 uF; 12 V 6.8 uH / 3 x 10 uF (p16, from extracted text only, not rendered). Eq.14 transient minimum; example needs >= 48 uF.
- CIN: "typical recommended value for the high frequency decoupling capacitor is **4.7 uF or higher**", X5R/X7R, voltage rating > max VIN, twice max VIN recommended; plus 0.1 uF close to pins (p19). Fig 9-1 uses 2.2 uF (inconsistent with the 4.7 uF text).
- CBOOT: **0.1 uF, >= 16 V**, X7R/X5R (p14, p19), CB to SW.
### 3.14 Layout (p21-22, 9.4.1)
CIN as close as possible to VIN and GND pins; input/output grounds are localized top-side planes connected to GND pin; minimize FB trace length, both feedback resistors close to FB, VOUT sense at the load, route VOUT sense away from noisy nodes and preferably on another layer behind a shield plane; ground plane in a middle layer; wide VIN/VOUT/GND; heat-sinking: GND, VIN, SW pins are the main heat path, large planes, array of thermal vias to bottom ground plane; keep TJ < 125 C; SW pin trace as short as possible and just wide enough; output caps near the VOUT end of the inductor and closely grounded. Figure on p23.
### 3.15 Pin table (p3, Table 6-1; type A = analog, P = power, G = ground)
| Pin | Name | Type | Note |
|---|---|---|---|
| 1 | GND | G | power ground (source of LS FET) |
| 2 | SW | P | to inductor |
| 3 | VIN | P | |
| 4 | FB | A | output sense (divider tap); never short to ground during operation |
| 5 | EN | A | do not float (floating = disabled); may tie to VIN |
| 6 | CB | P | bootstrap, 100 nF to SW |
No exposed pad.

---------------------------------------------------------------------------------------------------

## 4. AP63200 / AP63201 / AP63203 / AP63205 (Diodes Inc.)

### 4.1 Identity
**DS41326 Rev. 3 - 2, November 2024** (every page header). "3.8V TO 32V INPUT, 2A LOW IQ SYNCHRONOUS BUCK". Orderable (p16): AP63200WU-7, AP63201WU-7, **AP63203WU-7 (3.3 V, 1.1 MHz, PWM/PFM)**, **AP63205WU-7 (5 V, 1.1 MHz, PWM/PFM)**; package **TSOT26 (6 pin)**, 3000 T&R. Body 2.8-3.0 x 1.5-1.7, pitch 0.95 BSC, A max 1.00 mm (p17).
### 4.2 **IOUT = 2 A (continuous), not 3 A** (p1, p9). 125/68 mOhm. HS peak limit min 2.5 / typ 2.8 / max 3.1 A (p5).
### 4.3 VIN: recommended **3.8-32 V** (p4); abs max **-0.3 to +35 V DC, -0.3 to +40 V for 400 ms** (p4); VEN -0.3 to +35 V; EN may tie to VIN (p11). Recommended **ambient** TA -40 to +85 C (not a TJ range). Abs-max TJ +160 C (p4), but p12 text says "should not exceed ... +125 C"; TSD typ +160 C in table (p5) vs "+150 C" in the text (p11) - internally inconsistent.
### 4.4 Output (p5, **[R]**; "Min/Max limits apply across -40..+85 C ambient and VIN 3.8-32 V", TA = 25 C typ): adjustable (200/201) VFB **min 792, typ 800, max 808 mV**; **AP63203 3.27 / 3.30 / 3.33 V**; **AP63205 4.95 / 5.00 / 5.05 V** (CCM). For fixed versions FB pin 1 connects directly to the output (**[R p9, Fig 21]**, rendered; no divider).
### 4.5 UVLO/EN (p5): UVLO rising **min 3.30, typ 3.50, max 3.70 V**, hysteresis typ 440 mV; falling threshold "below 3.1 V" (p11 text, nominal). EN high **min 1.15, typ 1.18, max 1.23 V**; EN low **min 1.05, typ 1.10, max 1.15 V**; 1.5 uA pull-up, 4 uA hysteresis current.
### 4.6 Frequency (p5): AP63200/01 **typ 500 kHz only**; AP63203/05 **typ 1100 kHz only**; FSS +/-6 % typ. No min/max.
### 4.7 On/off, duty, dropout: tON (min on-time) **typ 80 ns only** (p5). **No tOFF-MIN, no DMAX number, no dropout figure** anywhere (p14 qualitative: "effective duty cycle approximately 100%, acts as an LDO"). Nothing guaranteed for 5 V at VIN 6 V.
### 4.8 MOSFETs (p5): RDS(ON)1 HS **typ 125 mOhm**; RDS(ON)2 LS **typ 68 mOhm**; typ only; Note 8: "compliance to datasheet limits is assured by production test, characterization, and/or design".
### 4.9 Quiescent (p5): IQ PFM parts **typ 22 uA** (AP63201 258 uA); ISHDN **typ 1, max 3 uA**.
### 4.10 Thermal (p4): thetaJA **89 C/W**, thetaJC **39 C/W** (TSOT26, single-layer FR-4, 2 oz, minimum pad); no psi values.
### 4.11 Efficiency: **No min.** Features (p1) "Up to 80% Efficiency at 1mA Light Load; Up to 88% Efficiency at 5mA Light Load" (marketing "up to", no conditions); curves Figs 2, 4, 5 (p2, p6) typical.
### 4.12 External parts (p13-14; for AP63203/05, Table 2/3): AP63203 (3.3 V): L **3.9 uH**, C1 10 uF, C2 2 x 22 uF, C3 100 nF. AP63205 (5 V): L **4.7 uH**, C1 10 uF, C2 2 x 22 uF, C3 100 nF. Inductor: ripple 30-50 % of 2 A; "approximately 2.2 uH to 10 uH with DC current rating at least 35 % higher than max load"; DCR < 100 mOhm preferred. Input: ceramic > 10 uF "sufficient for most applications", RMS rating > half max load current. Output: "22 uF to 68 uF ceramic sufficient"; nominal vs effective not stated. BST: 100 nF ceramic. FB divider (adjustable parts only): R1 = R2*(VOUT/0.8 - 1); Table 1 uses R2 = 62 k (<= 3.3 V), 30 k (5 V), 18 k (12 V).
### 4.13 Layout (p15): heat dissipation major concern; 2 oz copper top and bottom; sufficient vias at the GND side of the input/output caps to the bottom layer; bottom layer under the device a large GND layer; VIN capacitors as close as possible; feedback components as close to FB as possible; Fig 25.
### 4.14 Pins (p2): 1 FB (feedback sensing terminal; fixed versions: connect to output), 2 EN, 3 VIN, 4 GND, 5 SW, 6 BST. No exposed pad.

---------------------------------------------------------------------------------------------------

## 5. Min on-time vs duty x period (all frequency variants)  **[calc]**

tON_required = D / fSW with D = VOUT/VIN at VIN = 26.1 V (ideal, losses neglected; actual D is a bit higher because of drops, which only helps).
(a) 5 V: D = 0.1916;  (b) 3.3 V: D = 0.1264.
Compare with tON-MIN: LMR33630 DDA **nom 75 / max 108 ns** (guaranteed bound); TPS62933 typ 70 only; LMR51430 typ 70 only; AP6320x typ 80 only.
Worst case for the on-time constraint is the highest frequency, so the (max) column is the critical one.

| Part / variant | fSW min / typ / max | tON for 5 V (ns) at those fSW | tON for 3.3 V (ns) | vs tON-MIN | Max VIN w/o foldback (VOUT/(tON-MIN*fSW)) at fSW max, 5 V / 3.3 V |
|---|---|---|---|---|---|
| LMR33630 **A** DDA | 340 / 400 / 460 kHz | 563 / 479 / 417 | 372 / 316 / 275 | >= 3.9x of 108 ns max (5 V); >= 2.5x (3.3 V). **No foldback** | 100.6 V / 66.4 V (at 108 ns) |
| LMR33630 B DDA | 1.2 / 1.4 / 1.6 MHz | 160 / 137 / 120 | 105 / 90 / **79** | 5 V ok (>108). **3.3 V: 79-105 ns, equal or below the 108 ns max bound** -> foldback not excluded; above 75 ns nom only by 4-30 ns | 28.9 V / **19.1 V** |
| LMR33630 C DDA | 1.8 / 2.1 / 2.4 MHz | 106 / 91 / **80** | **70 / 60 / 53** | 5 V: 80-106 ns is at or below 108 max -> foldback not excluded. **3.3 V: 53-70 ns < 75 nom -> foldback certain at 26.1 V** | 19.3 V / **12.7 V** |
| TPS62933 RT float | 450 / 500 / 550 kHz | 426 / 383 / 348 | 281 / 253 / 230 | >= 3.3x of 70 typ; no max given | 129.9 V / 85.7 V (at 70 ns) |
| TPS62933 RT = GND | 1000 / 1200 / 1350 kHz | 192 / 160 / 142 | 126 / 105 / 94 | ok vs 70 typ (1.3-1.8x); no max given | 52.9 V / 34.9 V |
| TPS62933 RT = 71.5 k | 310 kHz typ only | 618 | 408 | ok (>> 70 typ) | 230 V / 152 V (typ f) |
| TPS62933 RT = 9.09 k | 2100 kHz typ only | 91 | **60** | 3.3 V: 60 < 70 typ -> foldback | n/a |
| LMR51430 **X** | 450 / 500 / 560 kHz | 426 / 383 / 342 | 281 / 253 / 226 | >= 3.2x of 70 typ; no max given | 127.6 V / 84.2 V |
| LMR51430 Y | 950 / 1100 / 1250 kHz | 202 / 174 / 153 | 133 / 115 / 101 | 1.4-1.9x of 70 typ; no max given | 57.1 V / 37.7 V |
| AP63200/01 | 500 kHz typ only | 383 | 253 | 3.2x of 80 typ | n/a |
| AP63203/05 | 1100 kHz typ only | 174 | 115 | 1.4-2.2x of 80 typ | 56.8 V / 37.5 V |

Foldback is not a failure on any of these parts (the datasheets say VOUT stays in regulation; LMR33630 p17, TPS62933 p20/p27, LMR51430 p13), but switching frequency drops (more ripple, audible/EMI spectrum moves), and it voids the "constant frequency" assumption of the design check. Only LMR33630 gives a **guaranteed max** on-time number (108 ns DDA) so only LMR33630 can have the check closed on guaranteed values: at LMR33630**A**, 5 V and 3.3 V rails keep >= 2.5x margin even against 108 ns.

Off-time / high-duty check at VIN = 6.0 V, **[calc]** DMAX_fixed_freq = 1 - tOFF-MIN*fSW:
- Needed D: 5 V rail 0.833 ideal; with drops at 3 A **[calc, LMR33630 DDA max RDS 160 mOhm HS / 110 mOhm LS and 30 mOhm assumed DCR]** D ~ (5 + 3*0.14)/(6 + 3*(0.11-0.16)) = 0.93. 3.3 V rail 0.55 (0.57 with drops at 1 A).
- LMR33630 DDA (tOFF-MIN max 85 ns): A at 460 kHz max: **0.961 (guaranteed-bound)** -> covers 0.93 with 0.03 margin; B at 1.6 MHz: 0.864; C at 2.4 MHz: 0.796 -> for the 5 V rail B and C cannot hold fixed frequency at 6.0 V (foldback required). (Typ tOFF 50 ns: A 0.980, B 0.930, C 0.895.)
- TPS62933 (tOFF-MIN typ 140 ns): 500 kHz max 550 kHz: 0.923; 1.2 MHz max 1.35 MHz: 0.811 -> the 500 kHz option needed for 5 V at 6 V; typ only.
- LMR51430 (tOFF typ 150 ns, measured at VIN 4.5 V): X 560 kHz max: 0.916; Y 1.25 MHz: 0.812.
- AP6320x: no tOFF given -> cannot be computed.

---------------------------------------------------------------------------------------------------

## 6. Comparison and verdict

Guaranteed-field scorecard (columns: what has a min/max bound rather than "typ only"):
| | LMR33630ADDA | TPS62933 | LMR51430 | AP63203/05 |
|---|---|---|---|---|
| IOUT | 3 A (rec. max) | 3 A | 3 A | **2 A only** |
| VIN rec max / abs max | 36 / 38 V | **30 / 32 V** | 36 / 38 V | 32 / 35 V (40 V 400 ms) |
| VIN margin to 26.1 V (rec / abs) | 9.9 / 11.9 V | 3.9 / 5.9 V | 9.9 / 11.9 V | 5.9 / 8.9 V |
| VFB min/max | 0.985-1.015 V (TJ -40..125) | 784-816 mV (TJ -40..150) | 0.591-0.609 V (25 C tested) | 792-808 mV, fixed 3.27-3.33 / 4.95-5.05 (TA -40..85) |
| tON-MIN | **nom 75, max 108 ns** | typ 70 only | typ 70 only | typ 80 only |
| tOFF-MIN | **nom 50, max 85 ns** | typ 140 only | typ 150 only | none |
| fSW | min/typ/max | min/typ/max (RT float, GND) | min/typ/max | typ only |
| RDS(on) | typ + max | typ only | typ only | typ only |
| Current limit | min/typ/max | min/typ/max | min/typ/max | min/typ/max |
| VIN UVLO | VCC UVLO typ only (about 3.7 / 3.0 V) | min/typ/max both edges | rising typ/max, falling min/typ | rising min/typ/max |
| Dropout | typ 150 mV at 1 A, 140 kHz (not ensured) + typ curves | typ curve only | typ curve only | none |
| Efficiency min | none (typ >95 % peak, curves) | none | none | none ("up to") |

(a) 5 V at up to 3 A from 6.0-26.1 V:
- **LMR33630ADDA (400 kHz A variant): best fit.** VIN margin large; at 26.1 V tON = 417-563 ns vs 108 ns max (>= 3.9x); at 6.0 V needs D about 0.83-0.93 and DMAX bound from guaranteed tOFF-MIN is 0.961 at 460 kHz; dropout curve (typ) shows 3 A regulation from about 5.6 V, TI's own 5 V/3 A example circuit states 6-36 V (p20, Fig 9-1 / Table 9-1). **Caveat: the datasheet only specifies 5 V regulation (system table, not production tested) from VIN = 7 V, and the dropout is typ only, so at 6.0 V (2S at the 3.0 V/cell cutoff) 5 V/3 A is typically OK (about 0.4 V margin) but not guaranteed.** B and C variants cannot hold fixed frequency at 6.0 V (DMAX bound 0.864 / 0.796) and are worse on tON-MIN; do not use them for 5 V.
- TPS62933: viable but weaker. VIN 30 V rec (3.9 V margin at 26.1 V; abs max 32 V leaves only 5.9 V transient headroom for a motor-bus battery); tON and tOFF typ only; typ dropout knee exactly at VIN = 6.0 V at 3 A (VOUT about 4.91 V at 5.5 V, about 5.08 V at 6.0 V), so zero typ margin at 6.0 V; its own design example uses VIN min 5.5 V. EN max 5.5 V -> EN cannot be tied to the pack: float it (enabled by internal pull-up) or use a divider.
- LMR51430: **fails at 6.0 V for 3 A** (typ curve Fig 7-6: 5 V/3 A only above about 6.5 V; about 4.6 V at 6.0 V). Everything else is typ only.
- AP63205: **cannot supply 3 A (2 A part, HS limit min 2.5 A)**; no dropout/tOFF data. Reject for rail (a).

(b) 3.3 V at about 1 A:
- Any of the four can carry 1 A. VIN 6.0 V is not an issue for any (D = 0.55; far below DMAX bounds).
- On-time at 26.1 V: LMR33630**A** 275-372 ns (>= 2.5x of the 108 ns max); LMR33630 B (79-105 ns) and C (53-70 ns) fail to guarantee fixed-frequency operation; TPS62933 at 500 kHz 230-281 ns, at 1.2 MHz 94-126 ns (typ 70 ns only); LMR51430X 226-281 ns, Y 101-133 ns; AP63203 115 ns vs 80 typ.
- Best on guaranteed fields: LMR33630ADDA again (single part for both rails; same BOM family). Smaller alternates for rail (b) only if size matters: TPS62933 at 500 kHz (VIN abs max 32 V; typ-only timing) or LMR51430X 500 kHz (36 V; typ-only timing).

**Verdict: LMR33630ADDA (A = 400 kHz) for both rails.** It is the only candidate that (i) has VIN margin (36 V rec / 38 V abs vs 26.1 V), (ii) gives guaranteed max tON-MIN (108 ns) and tOFF-MIN (85 ns) so the duty-vs-on-time and 6 V-duty checks close on guaranteed numbers, and (iii) carries 3 A at 5 V at the low end typically. Open items: all LMR33630 min/max timing and current-limit numbers are given at VIN = 12 V (no VIN dependence tabulated), so the 108 ns / 85 ns bounds at 26.1 V / 6.0 V are best-available, not conditioned guarantees; the 6.0 V duty margin (0.961 bound vs about 0.93 needed incl. drops, my estimate) is thin; the inductor ripple-floor conflict (section 1.13); 5 V at VIN = 6.0 V is only typical; 5 V regulation is specified from 7 V; 2S at the cutoff voltage should be treated as typ-margin design unless the system accepts dropout below 7 V.

---------------------------------------------------------------------------------------------------

## 7. Could not determine / caveats
- LMR33630 dropout at 3 A: only the typ curve (Fig 8-9/8-10, read off the plot); no guaranteed max dropout; 5 V rail guaranteed from 7 V only.
- LMR33630 guaranteed min of tON-MAX / DMAX: tON-MAX has only nom/max; DMAX 98 % typ only.
- LMR33630 VIN UVLO thresholds: only "about 3.7 V / about 3 V" from text (VCC UVLO), no table, no min/max.
- TPS62933: no max for IQ, ISHDN, RDS(on), or any of tON_MIN/tOFF_MIN/tON_MAX; no min for VEN_RISE or max for VEN_FALL; max COUT not in the datasheet (refers to an app report).
- LMR51430: no max for tON/tOFF/RDS; VIN UVLO rising has only typ/max; DBV vs DDC package labelling inconsistent inside the datasheet (orderables are DDC).
- AP6320x: no tOFF, DMAX, dropout; frequency typ only; TSD/TJ limits internally inconsistent (160 vs 150 vs 125 C); effective vs nominal output capacitance not stated; no recommended-op TJ.
- No datasheet in this set gives a guaranteed efficiency minimum.
- AP6320x pp.7-8 figure captions scanned: no dropout/VIN-vs-VOUT curve (only IQ, ISHDN, fSW, UVLO vs temperature, startup/short plots), consistent with 'no dropout data'.
- Datasheet Eq.5 (LMIN >= 0.28 VOUT/fSW) vs its own 'maximum inductance / 10 % ripple floor' sentence on p22 are inconsistent; not resolved.
- I did not read the typical-characteristic plots beyond the dropout figures (Figs 8-9/8-10 LMR33630, 9-9 TPS62933, 7-6 LMR51430) and the one LMR33630 efficiency curve cover; curve-reading values are typ and approximate.
- KiCad symbol/footprint library names were not checked; datasheet orderables are LMR33630ADDA (DDA0008J), TPS62933DRLR (DRL0008A), LMR51430XDDCR / YDDCR (DDC0006A), AP63205WU-7 (TSOT26). LMR33630 exposed-pad drawing dims given as a min/max range (2.0-2.6 x 2.5-3.1 mm); not verified against any KiCad footprint.
