# Datasheet read: LDOs, level translators, CRCW resistors (AET-4300d spike)

Method: text extracted per page, then every table whose column or condition was in doubt was RENDERED and read from the image
(renders in /home/forge/.cache/aet-4300d/render/). "typ only" = no guaranteed min/max in the document. Page numbers are the printed
page numbers, which equal the PDF page index in all five files.

Rendered and checked: TLV755P p4, p5, p6; AP2112 p1, p2, p4, p5, p8; TXU0304 p7, p8, p9, p12, p14; TXB0104 p6, p8, p9, p12;
Vishay p1, p2, p8.

---------------------------------------------------------------------------------------------------
## 1. LDOs

### 1.1 Document identity
| Part | Document | Revision / date |
|---|---|---|
| TLV75518P, TLV75533P | TI TLV755P, literature no. SBVS320D | Rev D, "November 2017 - Revised September 2024" (p1 header) |
| AP2112K-1.8, AP2112K-3.3 | Diodes Inc. AP2112, "Document number: DS39724 Rev. 2 - 2" | June 2017 (every page footer) |

Both fixed-voltage TLV variants are in the same document. The AP2112 document is one datasheet with separate electrical tables per voltage.

### 1.2 Input voltage
| | VIN recommended | VIN abs max | Source |
|---|---|---|---|
| TLV75518P / TLV75533P | 1.45 V min, 5.5 V max | 6.0 V (VIN); VOUT abs max = min(VIN+0.3, 6.0) | TLV p4, 5.3 Recommended Operating Conditions; 5.1 Absolute Maximum Ratings |
| AP2112K-1.8 / -3.3 | 2.5 V min, 6.0 V max (TA -40..+85 C) | 6.5 V ("VCC Power Supply Voltage") | AP p3, "Recommended Operating Conditions" and "Absolute Maximum Ratings" |

TLV also: UVLO rising 1.21 min / 1.3 typ / 1.44 max V, hysteresis 40 mV typ (p6, 5.5).
TLV TJ recommended -40..125 C (p4). AP ambient -40..+85 C (p3); AP production-tests at 25 C only, over-temperature "guaranteed by design only" (Note 6, p4/5/8).

### 1.3 Output accuracy
**TLV75518P / TLV75533P** (p5, 5.5 Electrical Characteristics, "Output accuracy"; conditions in table header: TJ -40..125 C,
VIN = VOUT(NOM)+0.5 V or 2.0 V whichever is greater, IOUT = 1 mA, VEN = VIN, CIN = COUT = 1 uF):
- -40 C <= TJ <= +85 C, DBV and DRV package: -1 % / +1 % (guaranteed min/max)
- VOUT >= 1 V: -1.5 % / +1.5 % (the row states no temperature; the table header range is -40..125 C, so this is the full-TJ figure. Rendered: this row sits after the +/-10 mV row, which is for 0.6 V <= VOUT < 1.0 V at -40..+85 C.)
- DQN package has its own +/-1.2 % row (not our package).
- So for 1.8 V DBV: 1.782..1.818 V (TJ <= 85 C), 1.773..1.827 V (TJ up to 125 C). For 3.3 V: 3.267..3.333 V and 3.2505..3.3495 V.
- Test is at IOUT = 1 mA and VIN = VOUT+0.5 V (= 2.3 V for the 1.8 V part, 3.8 V for the 3.3 V part). Line regulation (VOUT+0.5 V <= VIN <= 5.5 V, VOUT > 1.5 V): 2 mV, **typ only**. Load regulation 0.1 mA..500 mA, DBV and DYD: 0.060 V/A, **typ only** (= 30 mV at 500 mA, 1.7 % of 1.8 V; negligible at an IMU's few-mA load). The datasheet does NOT say whether the +/-1 % includes line/load; it is stated at the single test point above. Undetermined; flagged.

**AP2112K-1.8** (p5, "AP2112-1.8 Electrical Characteristics", VIN = 2.8 V, CIN = COUT = 1.0 uF, TA = +25 C):
- VOUT = VOUT*98.5 % min / 1.8 typ / VOUT*101.5 % max, i.e. 1.773..1.827 V, at **VIN = 2.8 V, 1 mA <= IOUT <= 30 mA, TA = 25 C**. Note 6: production-tested at 25 C, over-temperature "guaranteed by design only".
- Temperature coefficient: +/-100 ppm/C, **typ only**, IOUT = 30 mA, TA -40..+85 C. No guaranteed min/max VOUT over temperature is published.
- Load regulation: -1 / 0.2 typ / +1 %/A (VIN = VOUT+1 V, 1 mA..600 mA). Line regulation: -0.1 / 0.02 / +0.1 %/V (2.8 V <= VIN <= 6 V, IOUT = 30 mA).
- IOUT(MAX) condition: VIN = 2.8 V, VOUT held within 1.773..1.827 V -> 600 mA min.

**AP2112K-3.3** (p8, "AP2112-3.3 Electrical Characteristics", VIN = 4.3 V): VOUT 98.5 %..101.5 % = 3.2505..3.3495 V at IOUT 1..30 mA; IOUT(MAX) 600 mA min with VOUT 3.251..3.350 V; same load/line/tempco as above.

### 1.4 Dropout voltage
**TLV** (p6, 5.5 Electrical Characteristics, "VDO Dropout voltage", IOUT = 500 mA; rendered; max column only, no min; typ column given too):
| VOUT range of the part | TJ -40..+85 C: typ / max (mV) | TJ -40..+125 C: max (mV) |
|---|---|---|
| 1.5 V <= VOUT < 1.8 V | 350 / 400 | 425 |
| **1.8 V <= VOUT < 2.5 V (applies to TLV75518P)** | 325 / **380** | **400** |
| 2.5 V <= VOUT < 3.3 V | 250 / 300 | 325 |
| **3.3 V <= VOUT < 5.0 V (applies to TLV75533P)** | 150 / **215** | **238** |
(DYD package has slightly larger numbers; not our package.) The table is banded by VOUT and covers 1.8 V explicitly; there is no "only for VOUT >= x" restriction. Headline "238 mV max at 500 mA (3.3 V OUT)" on p1 is the 125 C figure. Dropout is specified only at 500 mA; text (p15, 7.1.2) says "VDO scales linearly with the output current" (no guaranteed value at lower current).

**AP2112K-1.8** (p5): VDROP = 500 mV typ / **700 mV max** at IOUT = 10 mA, 300 mA and 600 mA (the same 500/700 is printed for all three currents; verified in the render). Specified in the 1.8 V table, so it applies to this variant. TA conditions not stated beyond Note 6 (25 C production; over temp by design).
**AP2112K-3.3** (p8): 10 mA: 5 typ / 8 max mV; 300 mA: 125 / 200; 600 mA: 250 / **400** mV max.

### 1.5 IOUT and current limit
| | IOUT max (recommended) | Current limit |
|---|---|---|
| TLV75518P / 75533P | 500 mA (p4, 5.3: 0..500) | ICL min 560 / typ 720 / max 865 mA (p5; VIN = VOUT+VDO(MAX)+0.25 V; foldback); ISC (VOUT = 0 V) 355 mA **typ only** |
| AP2112K-1.8 / -3.3 | 600 mA min guaranteed ("IOUT(MAX)", p5/p8), "Output Current: 600mA (Min.)" p1 | No guaranteed current-limit min/max. ISHORT (VOUT = 0 V) 50 mA **typ only** (foldback) |

### 1.6 Quiescent / ground current
- TLV (p5, "IGND Ground current", IOUT = 0 mA): TJ = 25 C: 14 min / 25 typ / **31 max** uA; -40..+85 C: **33 max** uA; -40..+125 C: **40 max** uA. Shutdown (VEN <= 0.4 V, 1.4 V <= VIN <= 5.5 V, -40..125 C): 0.1 typ / **1 max** uA (ISHDN).
- AP2112 (p5/p8, IQ, VIN = 2.8 V / 4.3 V, IOUT = 0): 55 typ / **80 max** uA. Standby (VEN off): 0.01 typ / **1.0 max** uA. (Max at 25 C production test; temperature not separately guaranteed.)

### 1.7 PSRR (all typ only)
- TLV (p6): 52 dB @ 1 kHz, 46 dB @ 100 kHz, 52 dB @ 1 MHz, with VIN = VOUT+1 V, IOUT = 50 mA, COUT = 1 uF. Typ only. (p1 features: "46 dB at 100 kHz".) Output noise 71.5 uVrms typ (BW 10 Hz..100 kHz, VOUT = 1.2 V, 50 mA).
- AP2112 (p5/p8): 65 dB @ 100 Hz and 65 dB @ 1 kHz, ripple 0.5 Vp-p, IOUT = 100 mA, VIN 2.8 V (1.8 V part) / 4.3 V (3.3 V part). Typ only. No 100 kHz or 1 MHz value. Noise 50 uVrms typ (10 Hz..100 kHz, no load).

### 1.8 Output / input capacitor
**TLV755P** (p4, 5.3 Recommended Operating Conditions, rendered): CIN **1 uF min** (no max); COUT **1 uF min, 200 uF max**; no ESR range in the table.
Pin table footnote (p3): "Make sure the nominal input and output capacitance is greater than 0.47 uF. Throughout this document the nominal derating on these capacitors is 50 %. Make sure the effective capacitance at the pin is greater than 0.47 uF."
Application section (p15, 7.1.1): "The TLV755P requires an output capacitance of 0.47 uF or larger for stability. Use X5R- and X7R-type ceramic capacitors because these capacitors have minimal variation in capacitance value and equivalent series resistance (ESR) over temperature. ... Higher output voltages cause a significant derating of the capacitor. Generally, derate ceramic capacitors by 50 %. For best performance, use a maximum output capacitance value of 200 uF. Place a 1 uF or greater capacitor on the input pin of the LDO. ..." No numeric ESR range is given anywhere in the document (grep). Features (p1): "Stable with a 1 uF ceramic output capacitor". Note the internal inconsistency: Rec. Op. Conditions say 1 uF (nominal), text/footnote say 0.47 uF effective (= 1 uF nominal at 50 % derating). Electrical characteristics tested at CIN = COUT = 1 uF.
**AP2112** (p1 Features): "Stable with 1.0 uF Flexible Cap: Ceramic, Tantalum and Aluminum Electrolytic". Typical Applications Circuit (p2): CIN = 1 uF, COUT = 1 uF. Note 4 (p2): "It is recommended to use X7R or X5R dielectric capacitor if 1.0 uF ceramic capacitor is selected as input/output capacitors." No min-effective-capacitance, max-capacitance, or ESR range is stated anywhere in the document; no application/layout section exists. Undetermined.

### 1.9 Thermal, SOT-23-5
- TLV DBV (p5, 5.4 Thermal Information, rendered): RthetaJA **231.1 C/W (JEDEC)**; **100.8 C/W (EVM)**. The EVM footnote says EVM metrics apply to the LP087A EVM with the exposed-pad DYD layout, so the DBV-EVM figure's board is not a DBV layout; use 231.1 as the conservative, unambiguous number. (DYD, not ours: 92.5 JEDEC / 60.3 EVM; the 60.3 on the p1 feature list is the DYD.) RthetaJC(top) 118.4, RthetaJB 64.4 C/W (JEDEC).
- AP2112K SOT25 (p3, listed under Absolute Maximum Ratings): thetaJA 184 C/W "(No Heatsink)"; thetaJC 96 C/W typ (p5). Thermal shutdown 160 C, hysteresis 25 C (typ).

### 1.10 Pins (SOT-23-5)
| Pin | TLV755P DBV (p3, Table 4-1) | AP2112K SOT25 (p1 diagram, p2 Pin Descriptions) |
|---|---|---|
| 1 | IN | VIN |
| 2 | GND | GND |
| 3 | EN | EN |
| 4 | NC | NC (shown in the p1 top-view; the p2 table lists NC only for SOT89/SO-8, but the p1 SOT25 drawing labels pin 4 NC) |
| 5 | OUT | VOUT |
Footprints and pinouts are identical, so the two are drop-in alternatives. EN: TLV VHI 1 V min, VLO 0.3 V max (p6); AP VIH 1.5 V min, VIL 0.4 V max, 3 Mohm internal pulldown typ (p5). Both have active output discharge (TLV 120 ohm typ; AP 60 ohm typ).

### 1.11 Layout guidelines as written
- TLV755P (p21, 7.4.1): "Place input and output capacitors as close as possible to the device. Use copper planes for device connections to optimize thermal performance. Place thermal vias around the device to distribute the heat. For packages with thermal pads, solder the thermal pad to copper to achieve best thermal resistance. Thermal resistance increases significantly when the thermal pad is not soldered." (DBV has no pad.) Also p15 7.1.1: use higher-value input cap "if the device is located several inches from the input power source".
- AP2112: no layout guidance in the document (only suggested land pattern drawings, p14 and later).

---------------------------------------------------------------------------------------------------
## 2. Translators

### 2.1 Document identity
- TXU0304: TI SCES935A, "MAY 2021 - REVISED NOVEMBER 2021" (revision A; header says October 2021 in revision history, footer Nov 2021). Production data for TXU0304PW.
- TXB0104: TI SCES650K, "APRIL 2006 - REVISED MARCH 2025".

### 2.2 Supply ranges
| | VCCA | VCCB | Constraint |
|---|---|---|---|
| TXU0304 | 1.08..5.5 V (p6, 7.3; pin table p4 says 1.1 V <= VCCA <= 5.5 V) | 1.08..5.5 V (same) | None: "fully configurable dual-rail"; either can be higher; "can also be operated with VCCA = VCCB" (p20). Abs max 6.5 V each. |
| TXB0104 | 1.2..3.6 V | 1.65..5.5 V (p6, 5.3) | **VCCA <= VCCB at all times** (p1, p4, p6 footnote 2, p25: "ensure that VCCA <= VCCB at all times"). A port abs max 4.6 V. TA recommended -40..85 C. |
TXU0304 TA recommended -40..125 C.

### 2.3 Directions and OE
**TXU0304** (p4 Table 6-1, p20 Block diagram; rendered pin names): **A1, A2, A3 are inputs (referenced to VCCA) driving B1Y, B2Y, B3Y (outputs, VCCB): A -> B.** **B4 is an input (VCCB) driving A4Y (output, VCCA): B -> A.** Fixed direction, push-pull, 3 channels A->B + 1 channel B->A. Output labels "Y" mark the output pins. OE (pin 8): high enables all four channels, low = all outputs Hi-Z (isolation) (Table 9-1, p23). OE "can be referenced to either VCCA or VCCB" (VCC(MIN) circuitry, p21 9.3.2). OE has the integrated 5 Mohm typ pulldown, so floating OE = outputs disabled; OE thresholds identical to data (VT+ max 1.92 V at 3 V, p7).
**TXB0104** (p20): bidirectional, automatic direction sensing via one-shot edge-rate accelerators and 4 kohm series resistors; all four channels any direction. OE is referenced to VCCA (p4, p25): VIH(OE) = 0.65 x VCCA min, VIL(OE) = 0.35 x VCCA max (p6). "To ensure the high-impedance state during power up or power down, OE must be tied to GND through a pulldown resistor" and not enabled until VCCA and VCCB are stable (p25).

### 2.4 Speed at 1.8 V <-> 3.3 V
**TXU0304** (p9, 7.6 "Switching Characteristics: Tsk, TMAX", -40..125 C, rendered). The MIN column is the guaranteed max data rate. "Up translation" = VCCI < VCCO; "Down translation" = VCCI > VCCO:
- VCCI 3.0-3.6 V -> VCCO 1.65-1.95 V (down): **TMAX 50 Mbps min** (3.3 -> 1.8: the SCK/MOSI/CS direction when MCU = A port).
- VCCI 1.65-1.95 V -> VCCO 3.0-3.6 V (up): **TMAX 100 Mbps min** (1.8 -> 3.3: the MISO direction).
- Output skew tsk max: 8 ns in both cases (3.0-3.6 / 1.65-1.95 V pair, p9).
- Propagation delay (p14, 7.11 VCCA = 3.3 +/- 0.3 V, B port 1.8 +/- 0.15 V): A->B 0.5 min / **19 ns max** (both -40..85 and -40..125 C); B->A 0.5 / **14 ns max** (-40..85), **15 ns max** (-40..125). Disable time OE->A 42/46 ns max, OE->B 59/63 ns; enable time OE->A 37/40, OE->B 55/57 ns max.
- If instead MCU = B (VCCA = 1.8): p12 (7.9, A 1.8 +/- 0.15 V, B 3.3 +/- 0.3 V): A->B 14/15 ns max, B->A 19/19 ns max.
- Conditions: TMAX tested 50 % duty, one channel switching, 20 % of pulse above 0.7 VCCO / 20 % below 0.3 VCCO (p9).
**TXB0104** (PW package uses the "Other Packages" tables):
- Timing Requirements (p9, 5.9, VCCA = 1.8 +/- 0.15 V, VCCB = 3.3 +/- 0.3 V): **Data rate 60 Mbps max**, pulse duration 17 ns min. Same in 5.17 Switching Characteristics "Maximum data rate" 60 Mbps (p12; the 5.17 table shows 60 in the MIN-position column, same figure as 5.9).
- tpd (p12, 5.17 Other Packages, A 1.8 V / B 3.3 V): A->B 1.3 min / **6.8 ns max**; B->A 1 min / **7.6 ns max**. Channel skew 0.5 ns max. Rise/fall (input) A port 1.1/4.1 ns, B port 0.5/2.8 ns (min/max). Enable time 1 us max; disable time OE->A 5.0..19.3 ns, OE->B 4.2..17.9 ns (Other Packages).
- Test load: 15 pF / 1 Mohm (p17). The drivers only handle "capacitive loads of up to 70 pF" (p22).

### 2.5 Thresholds and output levels
**TXU0304** (Schmitt-trigger inputs; p7, 7.5, -40..85 and -40..125 C columns identical; rendered; VT+ is the effective VIH, VT- the effective VIL; the datasheet defines no VIH/VIL):
| VCC (input port) | VT+ min / max (V) | VT- min / max (V) | Hysteresis min / max |
|---|---|---|---|
| 1.65 V (nearest row to 1.8 V +/- 0.15 V) | 0.76 / **1.13** | **0.35** / 0.69 | 0.30 / 0.55 |
| 3 V (nearest row to 3.3 V +/- 0.3 V; no 3.3 V row) | 1.48 / **1.92** | **0.89** / 1.50 | 0.46 / 0.72 |
| 4.5 V (for interpolation) | 2.19 / 2.74 | 1.51 / 1.97 | 0.58 / 0.93 |
Values at VCC between the tabulated rows are not guaranteed by the table (interpolation only). Design to VIH >= VT+ max, VIL <= VT- min.
Outputs (p8): VOH min (-40..125 C): VCCO-0.1 V @ -0.1 mA; 1.2 V @ -4.5 mA (VCC 1.65 V); 2.2 V @ -10 mA (VCC 3 V). VOL max: 0.1 V @ 0.1 mA; 0.45 V @ 4.5 mA (1.65 V); 0.8 V @ 10 mA (3 V). Recommended IOH/IOL: 4.5 mA at VCCO = 1.65 V, 10 mA at 3 V (p6). VOH tested at VI = VT+(MAX), VOL at VI = VT-(MIN) (note 3/4, p9).
**TXB0104** (p6, 5.3 Recommended Operating Conditions): VIH data = 0.65 x VCCI min (to VCCI); VIL = 0 to 0.35 x VCCI max. At VCCI 1.8 V: VIH >= 1.17 V, VIL <= 0.63 V; at 3.3 V: VIH >= 2.145 V, VIL <= 1.155 V (computed from the stated ratios). Output levels guaranteed only at 20 uA (p8, 5.6 Other Packages): VOH >= VCCx - 0.4 V (IOH = -20 uA), VOL <= 0.4 V (IOL = 20 uA), -40..85 C. No VOH/VOL at mA loads are given (weak drivers). Driver requirement (p21, 7.3.2): "the device driving the data I/Os of the TXB0104 device must have drive strength of at least +/-2 mA."

### 2.6 Pull-ups, load, SPI/open-drain
**TXB0104** (p22, 7.3.5): "The device is designed to drive capacitive loads of up to 70 pF. The output drivers of the TXB0104 device have low dc drive strength. If pullup or pulldown resistors are connected externally to the data I/Os, their values must be kept higher than 50 kohm to ensure that they do not contend with the output drivers of the TXB0104 device. For the same reason, the TXB0104 device must not be used in applications such as I2C or 1-Wire where an open-drain driver is connected on the bidirectional data I/O. For these applications, use a device from the TI TXS01xx series." p19: "This device can only translate push-pull CMOS logic outputs." p21 (7.3.3): keep traces short; one-shot is ~10 ns, "reflection round trip delay must be less than the one-shot duration"; "With very heavy capacitive loads, the one-shot can time-out before the signal is driven fully to the positive rail." p24: "External pullup or pulldown resistors are not recommended. If mandatory ... larger than 50 kohm"; VOH = VCCx*RPD/(RPD+4.5 kohm), VOL = VCCx*4.5 kohm/(RPU+4.5 kohm). The datasheet's application curve is a 2.5 MHz signal (p24); the datasheet contains no SPI example.
**TXU0304** (p21, 9.3.1.1): 5 Mohm typ integrated pulldown on every input; "If an external pull-up is required, it should be no larger than 1 Mohm to avoid contention with the 5 Mohm internal pull-down." Applications (p1): "Push-pull level shifting (UART, SPI, JTAG, and so forth)". Typical application (p24, Fig 10-1) is an SPI interface: SoC SCK, PICO, CS on A1-A3 -> B1Y-B3Y to the slave; POCI from slave B4 -> A4Y to SoC. p24: "ideal for applications where a push-pull driver is connected to the data Inputs"; the TXU is positioned as the replacement for the TXB where high drive / fixed direction is needed (p3, "which the TXB0104 is not recommended to support"). Outputs: "high drive capability creates fast edges into light loads, so routing and load conditions should be considered to prevent ringing" (p21, p25). Layout (p25): 0.1 uF bypass per supply, close to the pins; 1 uF // 0.1 uF improves transient performance.

### 2.7 Packages and PW-14 pin map (both devices identical; from p4 of each)
TXU0304 packages: BQA VQFN-14, PW TSSOP-14 (5.00 x 4.40 mm), RUT UQFN-12, DTR X2SON-12. TXB0104: RUT, D SOIC-14, ZXU/GXU, PW TSSOP-14, RGY, YZT, NMN, BQA, DYY.
| PW pin | TXU0304 | TXB0104 |
|---|---|---|
| 1 | VCCA | VCCA |
| 2 | A1 (in) | A1 (I/O) |
| 3 | A2 (in) | A2 (I/O) |
| 4 | A3 (in) | A3 (I/O) |
| 5 | A4Y (out) | A4 (I/O) |
| 6 | NC | NC |
| 7 | GND | GND |
| 8 | OE | OE |
| 9 | NC | NC |
| 10 | B4 (in) | B4 (I/O) |
| 11 | B3Y (out) | B3 (I/O) |
| 12 | B2Y (out) | B2 (I/O) |
| 13 | B1Y (out) | B1 (I/O) |
| 14 | VCCB | VCCB |
Thermal: PW RthetaJA 135.8 C/W (TXU, p6); 121.0 C/W (TXB, p6).
KiCad symbols TXU0304PW / TXB0104PW should therefore carry A4Y / B4 as the opposite-direction pair; verify the symbol pin names against the table above.

### 2.8 Ioff and sequencing
- TXU0304: Ioff partial-power-down: A or B port with VCC = 0 V, VI/VO 0..5.5 V: -2.5/+2.5 uA max at -40..125 C (p8). Ioff(float) same limits. VCC isolation: if either VCC is < 100 mV or floating, all outputs Hi-Z (p1, p21). "Glitch-free power supply sequencing ... without any supply sequencing requirements such as ramp order or ramp rate"; apply GND first (p25 Section 11). OE can be left floating/pulled down to hold Hi-Z during power up (p20).
- TXB0104: Ioff A/B port: -2 / +2 uA max (-40..85 C), +/-1 uA at 25 C (p8). Outputs disabled when either VCC = 0 V. "During power-up sequencing, VCCA >= VCCB does not damage the device, so any power supply can be ramped up first," but "during operation, ensure VCCA <= VCCB"; OE pulldown, and "must not be enabled until VCCA and VCCB are fully ramped and stable" (p25). Layout figure note: "Keep OE low until VCCA and VCCB are powered up".

---------------------------------------------------------------------------------------------------
## 3. Vishay D/CRCW e3 (D11/CRCW0603 e3)

1. Document: "D/CRCW e3 - Standard Thick Film Chip Resistors", Document Number **20035**, **Revision 14-Apr-2026**, 13 pages.
2. Technical specifications (p1, rendered), 0603 column: 1 ohm..10 Mohm; tolerance +/-5 %, +/-1 % **only (no 0.5 % or 0.1 %)**; TCR +/-200 or +/-100 ppm/K; rated dissipation P70 **0.125 W** in the headline table (this is the "extended" mode figure; see below); operating voltage Umax 75 V AC rms/DC; film temperature max 155 C; operating temperature range **-55..+155 C**; insulation 100 V (1 min).
   TCR / tolerance / range matrix (p2, "Temperature coefficient and resistance range", D11/CRCW0603 e3): +/-5 %, +/-200 ppm/K, 1 ohm-10 Mohm (E24); +/-1 %, +/-200 ppm/K, **1 ohm-9.76 ohm** only (E24/E96); **+/-1 %, +/-100 ppm/K, 1 ohm-10 Mohm (E24; E96)**. Part-number code: ...FK... = +/-1 %, K = +/-100 ppm/K (p3 e.g. CRCW0603562RFKEA); N = +/-200 ppm/K; J = +/-5 %. So a divider should order F/K parts (+/-1 %, +/-100 ppm/K).
   Operation modes (p2, "Maximum resistance change at rated dissipation and operating voltage"): 0603 **Standard 0.10 W, 75 V; Extended 0.125 W, 75 V.** Resistance change |dR/R| after 1000 h: <= 1 % (standard) / <= 2 % (extended); after 8000 h: <= 2 % (standard). Footnote: "(1) Applies to components with stability class 1" (i.e. the +/-100 ppm/K parts).
   The document does not state a 70 C-derating curve in the text I read; derate to zero at 155 C is not extracted here (graph pages not read).
3. Stability (p8, "Test procedures and requirements", rendered): permissible dR for stability class 1 or better: endurance at 70 C rated power (U = min(sqrt(P70*R), Umax), 1.5 h on/0.5 h off): **1000 h: +/-(1 % R + 0.05 ohm); 8000 h: +/-(2 % R + 0.1 ohm)**; extended operating mode 1000 h: +/-(2 % R + 0.1 ohm). Endurance at 155 C, 1000 h: +/-(1 % R + 0.05 ohm). Damp heat steady state (40 C, 93 % RH, 56 days): +/-(1 % R + 0.05 ohm). Damp heat accelerated (85 C/85 % RH, <= 100 V, 1000 h): +/-(1 % R + 0.05 ohm). TCR test: between 20/-55/20 C and 20/155/20 C; class 1: +/-100 ppm/K or +/-200 ppm/K per type. Class 2 limits are twice class 1. These are test acceptance limits (a ceiling), not typical drift; a lightly loaded divider drifts far less, but no figure for that is published. p1 Features: "dR/R <= 1 % (1000 h rated power at 70 C)".
   Related product pointer (p4): tighter tolerance / better TCR parts are in "Lead (Pb)-Free Thick Film, Rectangular, Semi-Precision Chip Resistors", Vishay doc 20036 (NOT held, so 0.5 %/0.1 % options and their TCR are undetermined from our documents).

---------------------------------------------------------------------------------------------------
## 4. Answers

### (a) Does the 1.8 V LDO stay within 1.71-1.95 V from 3.3 V +/-5 % (VIN 3.135..3.465 V)?
**Yes, both do; TLV75518P with more margin and with a guaranteed bound.**

Dropout headroom (needs VIN >= VOUT(NOM) + VDO, TLV p14 Table 6-1 / AP VDROP):
- TLV75518P: VDO max 380 mV (TJ <= 85 C) or 400 mV (<= 125 C) at 500 mA. Required VIN >= 1.8 + 0.400 = 2.200 V. Actual minimum 3.135 V -> headroom 3.135 - 2.200 = 0.935 V (1.335 V above VOUT; 3.3x the dropout). Also well inside VIN 1.45..5.5 V and above VUVLO max 1.44 V.
- AP2112K-1.8: VDROP max 700 mV (any of 10/300/600 mA). Required VIN >= 1.8 + 0.700 = 2.500 V, which coincides with its recommended VIN minimum of 2.5 V. 3.135 V - 2.500 V = 0.635 V headroom. In regulation.

VOUT bound:
- TLV75518P: 1.8 V +/-1 % (TJ -40..+85 C) = **1.782..1.818 V**; +/-1.5 % (TJ to 125 C) = **1.773..1.827 V**. Margin to the ICM limits: low 1.773 - 1.71 = 63 mV, high 1.95 - 1.827 = 123 mV. These are guaranteed min/max. Caveat: the accuracy is stated at VIN = 2.3 V, IOUT = 1 mA; line regulation (2 mV, typ only) and load regulation (0.06 V/A, typ only = 0.3 mV at 5 mA) add negligibly.
- AP2112K-1.8: guaranteed only at 25 C, VIN = 2.8 V, 1-30 mA: 1.773..1.827 V. Over -40..+85 C there is only the typ tempco +/-100 ppm/C: 100e-6 x 65 K (to +85 C from 20-25 C ... using 65 K) = 0.65 %, so about +/-2.15 % = 1.761..1.839 V; plus line regulation 0.1 %/V max x (3.465 - 2.8 = 0.665 V) = 0.07 %, giving 1.760..1.840 V; load regulation 1 %/A max x 0.03 A = 0.03 %. All inside 1.71..1.95 V (low margin 50 mV, high 110 mV), but the over-temperature part rests on a typ-only tempco and a "by design" statement, not a guaranteed number.

Recommendation: TLV75518P (guaranteed over temperature, wider VIN margin, 0.38 V dropout). Neither 3.3 V LDO variant can regulate 3.3 V from a 3.3 V rail: TLV75533P needs VIN >= 3.3 + 0.238 = 3.538 V (500 mA); AP2112K-3.3 needs >= 3.3 + 0.400 = 3.700 V (600 mA).

### (b) Can TXU0304 or TXB0104 carry SPI at 7 MHz between 3.3 V MCU and 1.8 V IMU?
**TXU0304: yes, recommended. TXB0104: possible on paper (60 Mbps rate) but not recommended.**
- TXU0304: **MCU on the A port (VCCA = 3.3 V), IMU on the B port (VCCB = 1.8 V).** Channels: **A1 = SCK, A2 = MOSI, A3 = CS (MCU out) -> B1Y/B2Y/B3Y (IMU)**; **MISO from the IMU into B4 -> A4Y to the MCU**. Same assignment as TI's own SPI example (p24 Fig 10-1: SCK, PICO, CS on A1-A3, POCI on B4/A4Y). PW pins: A1 = 2, A2 = 3, A3 = 4, A4Y = 5, B1Y = 13, B2Y = 12, B3Y = 11, B4 = 10, VCCA = 1, VCCB = 14, OE = 8, GND = 7. Speed: guaranteed max data rate **50 Mbps min** 3.3 -> 1.8 V (p9) and **100 Mbps min** 1.8 -> 3.3 V; propagation delay max 19 ns (A->B) and 15 ns (B->A, to 125 C) (p14). 7 MHz SCK has a 143 ns period (71 ns half period); 19 ns + 15 ns of translator delay on a round trip is 34 ns = 24 % of the period, which the IMU's SDO clock-to-out then has to fit into (check against ICM-20948 SPI timing; not in these documents). Thresholds: MCU high on A1-A3 must exceed VT+ max 1.92 V (at VCC = 3 V row); IMU high on B4 must exceed VT+ max 1.13 V (1.65 V row). Design points from the data: OE must be high (or the channel is Hi-Z; floating OE is pulled low by the 5 Mohm pulldown); B-port outputs are Hi-Z whenever OE is low or either supply is < 100 mV, so CS at the IMU needs a pull-up to 1.8 V (external pull-up allowed, <= 1 Mohm, p21; use e.g. 100 kohm, as TXU outputs are strong push-pull and a 100 kohm pull-up costs nothing).
- TXB0104: **MCU must be on the B port (VCCB = 3.3 V), IMU on A (VCCA = 1.8 V)** because VCCA <= VCCB is mandatory. Cited speed 60 Mbps max (p9, p12), tpd 1.3..6.8 ns (A->B) / 1..7.6 ns (B->A), so 7 MHz is far inside the rating. Reasons not to use it: weak 4 kohm-series auto-direction drivers needing >= +/-2 mA from the MCU pins, no VOH/VOL specified above 20 uA, no pull-up below 50 kohm on any line (p22), 70 pF load limit, and the document contains no SPI example; with a tri-stating IMU SDO, auto-direction cannot be relied on. (The last clause is my engineering judgement, not a datasheet statement.) The TXU0304 datasheet itself positions TXU as the SPI/high-drive part.

### (c) Battery-sense divider, 26.1 V max (e.g. 6S at 4.35 V) to an ADC full scale
Assumption: ADC full scale 3.3 V (the document set does not fix the ADC reference), so nominal ratio k = Rb/(Rt+Rb) = 3.3/26.1 = 0.12644 (e.g. Rt = 100 k / Rb = 14.5 k class; any E96 pair near that ratio gives the same spread, since the spread depends on k only through (1 - k)).
Method: each resistor has relative error +/-e, where e = tolerance + TCR x dT. First order, dk/k = (1 - k)(d_b - d_t), worst case opposing, so spread = +/-2e(1 - k) = +/-2e x 0.8736; exact formula k' = k(1 +/- e)/[(1-k)(1 -/+ e) + k(1 +/- e)] was used below.
TCR dT: reference 20 C per Vishay's TCR test (p8), worst excursion to +85 C = 65 K (to -40 C is 60 K). Part: CRCW0603 ...FK.. (+/-1 %, +/-100 ppm/K, valid for 1 ohm..10 Mohm, p2).
| Case | e per resistor | Ratio error (worst low / high) | ADC voltage at 26.1 V in |
|---|---|---|---|
| 1 % tolerance only | 1.000 % | -1.73 % / +1.76 % | 3.243..3.358 V |
| **1 % + 100 ppm/K x 65 K (0.65 %)** | **1.650 %** | **-2.85 % / +2.92 %** | **3.206..3.396 V** |
| 1 % + 200 ppm/K x 65 K (0.65 -> 1.30 %): the +/-200 ppm/K parts | 2.300 % | -3.95 % / +4.09 % | 3.170..3.435 V |
| 1 % + 100 ppm/K + class-1 drift ceiling 1 % per resistor (1000 h at rated power, p2/p8) | 2.650 % | -4.54 % / +4.72 % | 3.150..3.456 V |
Illustration only (NOT offered in the held CRCW e3 document, which lists +/-1 % and +/-5 %; TCR for such grades is in doc 20036, which we do not hold; the TCR of 100 ppm/K is assumed for the lines below):
| 0.5 % + 100 ppm/K | 1.150 % | -1.99 % / +2.03 % | 3.234..3.367 V |
| 0.1 % + 100 ppm/K | 0.750 % | -1.30 % / +1.32 % | 3.257..3.344 V |
| tolerance only: 0.5 % / 0.1 % | 0.5 / 0.1 % | +/-0.87 % / +/-0.18 % | |
Note the drift ceiling is an acceptance limit under rated power, never reached by a divider dissipating (26.1 V)^2/114.5 kohm = 6 mW (below the 0.10 W rating), but no smaller guaranteed figure exists. Self-heating on top of ambient is also not in the table. With 1 % parts the ratio spread is about +/-2.9 % over temperature; a one-point calibration removes the initial-tolerance part, leaving the TCR term: +/-2 x 0.65 % x 0.8736 = +/-1.14 %. Max voltage on a 0603 resistor is 75 V; each carries well below that.

---------------------------------------------------------------------------------------------------
## 5. Could not determine / flagged
- TLV755P: whether the +/-1 % / +/-1.5 % accuracy includes line and load regulation (stated at one test point; line/load are separate typ-only rows). The temperature condition of the +/-1.5 % row is implied by the table header (-40..125 C) rather than printed on the row. The DBV EVM RthetaJA (100.8) is measured on a DYD-style EVM board; no ESR range is given.
- AP2112: no guaranteed over-temperature output accuracy, no current-limit min/max, no ESR/min-effective-C/max-C, no layout guidelines, no PSRR above 1 kHz.
- TXU0304: no VIH/VIL defined (Schmitt VT+/VT- used); no 3.3 V (only 3 V) row for thresholds/VOH/VOL. SCES935A is an early revision (A).
- TXB0104: no SPI statement or mA-level VOH/VOL.
- Vishay: 0.5 % / 0.1 % not offered in doc 20035; ADC full-scale reference not given (3.3 V assumed in (c)); power derating graph not read.
