# AET-WO-4300k measurements (thermal, TI theta-JA, fab bounds, held pages)

Measured at 521dcd06. Scripts: spike/4300k/thermal/{thermal_copper.py,findpages.py,render.py}.
Scratch: ~/.cache/aet-4300k/thermal/ (emit/, pages/*.png). Page numbers are printed page = PDF page unless stated.

## Task 1: copper joined to U10 / U11 EP pad (pad 9, net GND)

Emit: `aethelgard emit --file sentinel-03.json --out ~/.cache/aet-4300k/thermal/emit` ok; board sha256 da09ab13...
Pad 9: Texas_HSOP-8-1EP footprint, F.Cu only, 2.95 x 4.9 mm = 14.455 mm2 = 0.145 cm2, net GND. U10 at (133.5,79.9), U11 at (133.55,106.05).
12 thermal vias inside each pad, 0.7 mm pad / 0.254 mm drill (emit notes: raised from 0.554 to 0.7 for the declared ring).
Zones in the file: In1.Cu GND (whole-board outline 98.75 x 68.6 mm = 67.74 cm2) and In2.Cu VDD_3V3 (same outline). No F.Cu, no B.Cu pour. Other zones are keepouts.
**No zone has a filled_polygon: both are outlines only, UNFILLED** (`filled_areas_thickness no`). A refill in KiCad is needed to get a real copper area.

Method (pure python, no shapely): parse the s-expression; items = pads, segments (capsules), vias (circles through all 4 layers); join items of the same net that touch on a layer; vias join layers; a same-net zone joins every same-net item inside its outline on its layer (what a KiCad refill would do). Area = union of shapes rasterised at 0.02 mm. Two runs:
(a) NOZONE=1: only what tracks, pads, vias join (local copper, no plane).
(b) with zones: the joined set becomes the whole GND net (452 items), so F.Cu = every GND pad/track on the board (not local).

(a) Local copper, cm2 per layer (the number the screen's "per layer" figure would honestly be):

| layer | U10 | U11 | what |
|---|---|---|---|
| F.Cu | 0.146 | 0.244 | pad 0.145 + via annuli; U11 adds 0.020 of tracks and 5 more vias |
| In1.Cu | 0.048 | 0.071 | via annuli only (12 / 17 vias) |
| In2.Cu | 0.048 | 0.071 | via annuli only; the In2 zone is VDD_3V3, so GND is not a plane here |
| B.Cu | 0.048 | 0.071 | via annuli only; no B.Cu tracks or pour |

(b) With the In1 GND zone (unfilled outline): In1.Cu = 67.74 cm2 outline (my estimate of antipads for 302 non-GND vias at 0.2 mm clearance ~3.1 cm2, so ~64 cm2 once filled; an estimate, not a fill). Via-joined, so U10 and U11 share it. F.Cu joined-set union = 1.830 cm2 (whole-board GND pads/tracks, 0.269 cm2 of tracks), same for both: not thermally local, listed for completeness. In2/B.Cu add nothing but via rings (0.561 cm2 union over all 100 GND vias).

Reading: the plane that makes the number big exists only as an outline; the F.Cu copper on the pad's own layer is 0.15-0.24 cm2 against the declared 20 cm2 "per layer".

power.rs 2631-2715 (`junctions`): takes `r.junction.copper_square_centimetres.value` (declared, i64), finds the last curve point with area <= declared (`curve.points.iter().rev().find(|(a,_)| *a <= area)`); smaller than the first point (7 cm2) refuses `BuckJunctionOverLimit`. Tj = Tboard_max + loss x theta. The only layout fact used is `board.copper_layers` vs `curve.board_copper_layers` (4): a mismatch is a note, not a refusal. Nothing reads the emitted board. Any honestly measured per-layer F.Cu figure (0.15-0.24 cm2) is below the table's 7 cm2 floor and would refuse; the screen's own figure is "area on each layer" with 2 oz outer / 1 oz inner copper.

## Task 2: TI theta-JA at source

Datasheet ~/.cache/aet-4300d/ds/lmr33630.pdf sha256 3b0920a4e56a0b3f5f214a6317d3c06cff6e541e5054024adc31c30acc50c04a = power-parts.json documents.SNVSAN3F.sha256 (revision F, Nov 2020 header; footer Copyright 2025). **MATCH.**

| where | figure | board | cite |
|---|---|---|---|
| 7.4 Thermal Information | RthetaJA DDA (HSOIC, 8 pins) = **42.9 C/W** (VQFN RNX 72.5); RthetaJC(top) 54, RthetaJB 13.6, psiJT 4.3, psiJB 13.8, RthetaJC(bot) 4.3 | "calculated in accordance with JESD 51-7, and simulated on a 4-layer JEDEC board", "only valid for comparison with other packages and can not be used for design purposes" (notes 1,2) | p. 7 (rendered) |
| Figure 9-3 "Typical RthetaJA versus Copper Area for a Four-Layer Board and the HSOIC (DDA) Package" | curve from ~42 C/W at ~6 cm2, ~28 at 20, ~26 at 40, ~24.5 at 58 cm2 (axis 20-44 C/W, 0-70 cm2) | text: "The copper area given in the graph is for each layer; the top and bottom layers are 2 oz. copper each, while the inner layers are 1 oz"; "for illustration purposes only" | p. 25 (rendered; text in 9.2.2.x Maximum Ambient Temperature) |
| Figures 9-5 / 9-6 | derating curves at RthetaJA = 30 and 50 C/W | VIN 12 V, VOUT 5 V, 400 kHz | p. 25 |

power-parts.json LMR33630ADDA `theta_ja` cites SNVSAN3F "p. 25, Figure 9-3 ... read from the plot, each point rounded up to 0.5 C/W", 4 layers, points 7:42.0, 10:36.0, 15:30.0, 20:28.0, 30:27.0, 40:26.5, 50:25.5, 58:25.0 C/W. So the 20 cm2 declaration gives 28.0 C/W. Not the Table 7.4 number.
The "43 C/W" a sub-reviewer quoted is the Table 7.4 value 42.9 C/W (p. 7), rounded; board = 4-layer JEDEC simulated (JESD 51-7) and the datasheet itself says not for design. It is not an EVM figure. No "43" appears elsewhere with C/W in the text.

## Task 3: which fabricator can source every stackup bound

Bounds (board.rs 137-262; supply.rs 1356-1400; stack.rs 50-80, 279):
finished_thickness_um min / max (nominal too); each of 3 dielectrics `min_thickness_um` (`max_thickness_um` optional but prices vias); copper_max_um; relative_permittivity_max_permille; supply gate's four needs (each refused `SupplyInputUndeclared` if absent): outer_copper_min_um, plane_copper_min_um, plating_min_um, copper_temperature_max_c; plus fab_tolerances.{hole_position_um, hole_size_plus_um, hole_size_minus_um, track_width_permille} (supply gate), via_ring_um (ring rule, stack.rs).
(The task text said "four minima"; the code needs those four plus four tolerances; via_ring_um is read by stack.rs.)

Fetched live with WebFetch today (web available). Current fixture: OSH Park for thickness/dielectrics, JLCPCB for fab_tolerances (a hybrid: one board cannot have both fabricators).

| bound | JLCPCB (jlcpcb.com/capabilities/pcb-capabilities; impedance page jlcpcb.com/impedance) | OSH Park (docs.oshpark.com/services/four-layer/) |
|---|---|---|
| finished thickness min/max | YES: "Thickness Tolerance (Thickness>=1.0mm) +/-10%" (1.6 mm -> 1.44-1.76); impedance page gives no total | NO: "63mil (1.6mm) nominal", no tolerance (repo derives 1250/1851 from layer tolerances) |
| dielectric min thickness (each) | NO on capabilities page ("Dielectric/prepreg thickness: not on the page"); impedance page lists nominals only, e.g. 7628 0.21040 mm, core 1.065 mm w/ Cu, "no tolerance" | YES: prepreg "7.87 mil (0.1999mm) FR408HR 2113 +/-.797mil (0.0202mm)"; core "39 mil (0.9906mm) +/-3.9mil (0.0991mm)" |
| copper_max_um | NO: weights in oz only (multilayer outer 1 oz/2 oz; inner 0.5/1/2 oz), no tolerance | NO: "1oz outer" (1.7 mil), "1/2oz inner" (0.68 mil), no tolerance |
| relative_permittivity_max | NO: nominals 7628 4.4, 3313 4.1, 2116 4.16, solder mask 3.8; no max | NO: prepreg "3.61 at 1GHz" nominal; core not given |
| outer_copper_min_um | NO | NO |
| plane_copper_min_um | NO | NO |
| plating_min_um | NO min: "Average Hole Plating Thickness 18um" (average) | NO min: "Via plating thickness 1mil (0.0254mm)" nominal |
| copper_temperature_max_c | NO (only TG170 for HDI) | NO |
| hole_position_um | YES "+/-0.075 mm" | YES "Max: 2mil (0.0508mm)" (typical <1 mil) |
| hole_size_plus / minus | YES "Through-holes: +0.13 / -0.08 mm" (asymmetric) | YES, symmetric "Max: +/- 2.5mil (0.0635mm)" (typical 1.0 mil) |
| track_width_permille | YES "+/-20%" | NO: only "5mil (0.127mm)" min trace; no tolerance |
| via_ring_um | YES derivable: "via diameter ... 0.1mm (0.15mm preferred) larger than via hole" (ring 0.05); PTH annular ring ">=0.20mm", multilayer 1 oz absolute min 0.15 mm, 2 oz 0.254 mm | YES "Minimum Annular Ring 4mil (0.1016mm)" |

Neither fabricator publishes a min/max for copper_max, permittivity max, the three copper/plating minima or copper temperature. JLCPCB covers thickness (both bounds), hole tolerances, track width, ring; lacks dielectric minima. OSH Park covers dielectrics, hole tolerances, ring; lacks thickness tolerance and track width. Neither alone sources every bound; the sources a fixture cites for the unsourced ones must be the engine's stated condition or a standard (IPC-4101, IPC-6012), as the current fixture does for copper_max and permittivity.

## Task 4: held pages

(i) LMR33630 (SNVSAN3F; ~/.cache/aet-4300d/txt/lmr33630.txt, same text in aet-1216/ds):
- Absolute maximum VIN to PGND: -0.3 to **38 V**; EN to AGND max VIN+0.3 (7.1, p. 6).
- Operating VIN: **3.8 to 36 V** (7.3 Recommended Operating Conditions, p. 6). 7.5 table lists "minimum operating input voltage" max 3.8 V (p. 7).
- High-side current limit ISC LMR33630: min **3.85 A**, typ 4.5, max 5.05 A; low-side 2.9/3.5/4.1 A (7.5, p. 7; rendered).
- Hot plug / overshoot: no "hot plug" or "twice battery" statement. p. 32 (Power Supply Recommendations): long wires or traces "can form an under damped resonant circuit, resulting in overvoltage transients at the input to the regulator"; fix: aluminum/tantalum capacitor in parallel, "A value in the range of 20 uF to 100 uF is usually sufficient to provide input damping"; no factor given. p. 23 (9.2.2.6): "rated for at least the maximum input voltage ... preferably twice the maximum input voltage" is about capacitor voltage rating, not a transient factor; p. 23 also "moderate ESR ... can help damp any ringing on the input supply caused by the long power leads". grep of aet-held, aet-ds, aet-4300d/txt, aet-1216/ds, aet-4300c/ds for hot-plug / undamped / "twice the input": **no held document supports a factor of 2** (the 2x is the textbook undamped LC step, unsourced here). aet-ds files mentioning "hot plug" are IMU/baro/ES0392/ARM docs, not on this topic.
(ii) DS12117 Rev 10 (~/.cache/aet-held/ds12117-rev10.pdf sha256 607b9eadaef1...; aet-1216 txt is a differently-extracted copy):
- Two silicon-revision tables. **rev Y**, Table 76 (6.3.20), p. 167: RAIN external input impedance max **50 kohm**, one figure, no sampling-time dependence. **rev V**, Table 173 (7.3.20), p. 273 (rendered): RAIN max at TJ=125C: 16-bit 170 ohm, 14-bit 435, 12-bit 1150, 10-bit 5650, 8-bit **26500 ohm**.
- Table 174 "Minimum sampling time vs RAIN" (rev V), pp. 276-277 (guaranteed by design, 125C, 47 pF PCB capacitor, VDDA 1.6 V): the R-vs-time table. At 8 bit, 10000 ohm: 2.36 us direct / 2.47 us fast / 2.45 us slow; 15000 ohm 3.50/3.69/3.65 us. 10-bit tops at 3300 ohm (1.10/1.14/1.14 us); 12-bit at 680 ohm.
- Divider R50 100k / R51 10k: source impedance 100k||10k = 9.09 kohm. Within rev Y's 50 kohm; within rev V only at 8-bit (26.5 kohm), above rev V's 10-bit (5650), 12-bit (1150), 16-bit (170) limits unless a cap at the pin sources the charge. Which silicon revision the part is not stated in the repo files I read.
- Vishay CRCW: held at ~/.cache/aet-4300d/ds/vishay-crcw.pdf, sha256 113d2d57a320...c8b3 = power-parts.json VISHAY_20035 (rev 14-Apr-2026). D11/CRCW0603 e3 rated dissipation P70: **0.10 W standard, 0.125 W extended**; Umax 75 V; 1 Ohm-10 Mohm +/-1% (p. 2, rendered). Operating -55 to 155 C (p. 1). power-parts.json has CRCW0402_Z_0 and CRCW0603_F_K rows citing VISHAY_20035 (p. 1-2: temperature, tolerance +/-1%, TCR +/-100 ppm/K, stability class 1). parts.json has no CRCW rows. Also held: vishay-tnpw.pdf (VISHAY_28758).
(iii) W25Q64JV Rev J (~/.cache/aet-4200/ds/w25q64jv_revj.pdf; txt in aet-4300c/ds): 9.2 Operating Ranges, printed p. 59 (PDF p. 60, rendered): VCC **2.7 to 3.0 V** at FR=104 MHz (fR=50 MHz), **3.0 to 3.6 V** at FR=133 MHz. So 2.7 V min holds only for the 104 MHz class; at 133 MHz VCC min is 3.0 V. Abs max VCC -0.6 to 4.6 V (9.1, p. 59). Power-up timing p. 60: tVSL 20 us min, tPUW 5 ms, VWI 1.0-2.0 V.
