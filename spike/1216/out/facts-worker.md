# AET-WO-1216 facts (datasheet extraction, read-only)

## Page convention

All cites are the PRINTED page number. The held .txt files carry `=== page N` markers (the PDF page index).

- TI documents (LMR33630, TLV755P, TXU0304, DRV8876, DRV8833): the printed page number equals the PDF index. Each page prints its own number in the header/footer.
- DS12117 Rev 10: printed header/footer "N/357" equals the PDF index (Table 10 sits on "103/357").
- DS-009: printed footer = PDF index minus 1. The footer reads "Pixhawk Reference Standard 5" on PDF page 6. Cited here as the printed number, and the table of contents agrees (Telemetry Port 5).

**Extraction caveats**

- The text was extracted without column layout. Where a row shows ONE number and the table has min/typ/max columns, the column cannot be recovered. Those cells are marked "column not recoverable" and are never guessed.
- DS12117 has two electrical sections in one file: "(rev Y)" pp.101-205 and "(rev V)" pp.205+. Rev Y is cited first (Tables 10/11/48 as the caller named them). The rev V twins are in F7.

---

## A. LMR33630 (SNVSAN3F)

| item | fact | quote | page | table/section |
|---|---|---|---|---|
| EN rising threshold to start switching (VEN-H) | min 1.2 / typ 1.231 / max 1.26 V | "VEN-H EN input level required to start switching ... Rising threshold 1.2 1.231 1.26 V" | 7 | 7.5 Electrical Characteristics, ENABLE |
| EN falling | falling = VEN-H minus VEN-HYS. Hysteresis 100 mV, a single value, column not recoverable. No separate falling-threshold row. | "VEN-HYS Hysteresis below VEN-H; falling 100 mV" | 7 | 7.5 |
| EN threshold to power internal LDO (standby) | VEN-VCC-H rising: single value 1 V, column not recoverable. VEN-VCC-L falling: single value 0.3 V, column not recoverable. | "VEN-VCC-H EN input level required to turn on internal LDO Rising threshold 1 V"; "VEN-VCC-L ... turn off internal LDO Falling threshold 0.3 V" | 7 | 7.5 |
| EN behaviour | Above VEN-VCC-H: standby, VCC powered, no output. Above VEN-H: start-up and soft-start. Below VEN-H by VEN-HYS: stops, standby. Below VEN-VCC-L: fully shut down. | "Applying a voltage of ≥ VEN-VCC_H causes the device to enter standby mode, powering the internal VCC, but not producing an output voltage. Increasing the EN voltage to VEN-H fully enables the device, allowing it to enter start-up mode and starting the soft-start period." | 13 | 8.3.2 Enable and Start-up |
| EN strapped to VIN | Permitted. EN must not float. | "The EN input can be connected directly to VIN if this feature is not needed. This input must not be allowed to float." Pin table: "Can be connected directly to VIN; Do not float." | 13; 5 | 8.3.2; 6 Pin Functions (EN) |
| EN leakage | 0.2 nA at VEN = 3.3 V, single value, column not recoverable | "ILKG-EN Enable input leakage current VEN = 3.3 V 0.2 nA" | 7 | 7.5 |
| Input UVLO (VIN rising/falling) | NO numeric VIN-pin UVLO threshold is tabulated. The UVLO is on the internal VCC LDO output: VCC about 3.7 V ready to start, about 3 V shuts down regardless of EN. The text says these roughly represent the input-voltage levels. Typical values only ("about"). Min operating VIN 3.8 V. A curve "Figure 7-5. UVLO Thresholds" exists (graph only, p.10). | "When VCC reaches about 3.7 V, the device is ready to receive an EN signal and start up. When VCC falls below about 3 V, the device shuts down, regardless of EN status. Because the LDO is in dropout during these transitions, the above values roughly represent the input voltage levels during the transitions." | 15 (UVLO); 7 (VIN min 3.8 V); 10 (Fig 7-5) | 8.3.4 Undervoltage Lockout and Thermal Shutdown; 7.5; 7.8 |
| Internal soft-start tSS | min 2.9 / typ 4 / max 6 ms. Internal, reference-based. | "tSS Internal soft-start time 2.9 4 6 ms"; "The LMR33630 uses a reference-based soft start that prevents output voltage overshoots and large inrush currents ... The rise time of the output voltage is about 4 ms" | 8; 13 | 7.5 SOFT START; 8.3.2 |
| Start-up delay EN to switching | not stated as a separate figure. Only the soft-start (above) and the PG delay (below). | – | – | – |
| Start into pre-biased output | not stated. No pre-bias, sink or discharge statement. The only related text is the input-below-output warning (next row). | – | – | – |
| Input falling below output | Output capacitors discharge through the VIN-SW parasitic diode, with uncontrolled current. Recommends a Schottky from input to output if likely. | "The input voltage must not be allowed to fall below the output voltage. In this scenario, such as a shorted input test, the output capacitors discharges through the internal parasitic diode found between the VIN and SW pins of the device." | 32 | 9.3 Power Supply Recommendations |
| Output discharge in shutdown | not stated. Shutdown current ISD: 5 and 10 µA at EN = 0 (columns as extracted, probably typ/max, not confirmed). | "ISD Shutdown quiescent current; measured at VIN pin EN = 0 5 10 µA" | 7 | 7.5 |
| PG thresholds | % of FB voltage. Upper rising min 105 / typ 107 / max 110. Upper falling 103 / 105 / 108. Lower rising 92 / 94 / 97. Lower falling 90 / 92 / 95. | "VPG-HIGH-UP Power-good upper threshold - rising % of FB voltage 105% 107% 110%" (and the three following rows) | 8 | 7.5 POWER GOOD (PG PIN) |
| PG output | Open-drain NMOS, needs external pull-up, may pull up to VCC or VOUT through 100 kΩ. Float if unused. Limit sink to < 5 mA DC. Internally limited to about 35 mA enabled and about 65 mA disabled. | "The power-good output consists of an open-drain NMOS, requiring an external pullup resistor to a suitable logic supply. It can also be pulled up to either VCC or VOUT, through a 100-kΩ resistor, as desired. If this function is not needed, the PG pin must be left floating." | 12 | 8.3.1 Power-Good Flag Output |
| PG deglitch tPG | two values 60 and 170 µs, columns not recoverable. Excursions shorter than tPG do not trip the flag. | "tPG Power-good glitch filter delay 60 170 µs"; "Output voltage excursions lasting less than tPG do not trip the power-good flag." | 8; 12 | 7.5; 8.3.1 |
| PG start-up delay | about 4 ms, typical, from EN assertion to PG high; only at start-up | "during initial power up, a delay of about 4 ms (typical) is inserted from the time that EN is asserted to the time that the power-good flag goes high." | 12 | 8.3.1 |
| PG when EN low / VIN low | EN low forces PG low. PG valid with EN low as long as VIN ≥ 2 V (typical). Table: VIN-PG "Minimum input voltage for proper PG function", test cond. "50-µA, EN = 0 V", value 2 V (single value, column not recoverable). VPG (PG logic low, 50 µA, EN = 0 V, VIN = 2 V): 0.2 V (single value). | "When EN is pulled low, the flag output is also forced low. With EN low, power good remains valid as long as the input voltage is ≥ 2 V (typical)." | 12; 8 | 8.3.1; 7.5 |
| PG RDSON | VIN = 12 V, EN = 4 V: 76 Ω and 150 Ω (typ/max, as extracted). EN = 0: 35 and 60 Ω. | "RPG Power-good flag RDSON VIN = 12 V, VEN = 4 V 76 150 Ω" | 8 | 7.5 |
| Pin description PG | "High = power OK, low = power bad. Flag pulls low when EN = Low. Can be left open when not used." | quoted | 5 | 6 Pin Functions |
| Sequencing statement (PG of one to EN of another) | not stated. No sequencing or tracking text anywhere. | – | – | – |
| Abs max EN | −0.3 to VIN + 0.3 V. Note (2): must not exceed VIN by more than 0.3 V. | "EN to AGND(2) –0.3 VIN + 0.3" | 6 | 7.1 Absolute Maximum Ratings |
| Abs max PG | 0 to 22 V (abs max). Recommended 0 to 18 V. Same note (2). | "PG to AGND(2) 0 22" | 6 | 7.1; 7.3 Recommended Operating Conditions (PG 0 to 18) |
| Abs max VIN | −0.3 to 38 V. Recommended 3.8 to 36 V. | "VIN to PGND –0.3 38 V" | 6 | 7.1; 7.3 |
| Hiccup restart | If FB < about 0.4 V in current limit: stops for tHC (about 94 ms) then re-starts with soft start. | "the device stops switching for tHC (see Section 7.7), or about 94 ms and then goes through a normal re-start with soft start." | 15 | 8.3.3 |
| Light-load mode | Auto mode: PFM at light load. The datasheet describes diode emulation and does not describe sinking from a pre-biased output. | "At light loads, the regulator operates in PFM." | 15 | 8.4.1 |

---

## B. TLV755P (SBVS320D)

| item | fact | quote | page | table/section |
|---|---|---|---|---|
| EN thresholds | VHI (enabled) 1 V; VLO 0.3 V. Single values, column not recoverable. The table labels them "EN pin high voltage (enabled)" and "EN pin low voltage (enabled)". Text: enable by forcing EN above VHI, off below VLO. | "VHI EN pin high voltage (enabled) 1 V"; "VLO EN pin low voltage (enabled) 0.3 V" | 6; 12 | 5.5 Electrical Characteristics; 6.3.2 Enable (EN) |
| EN strapped to IN | Permitted. | "If shutdown capability is not required, connect EN to IN." | 12 | 6.3.2 |
| EN above VIN | EN current rises if EN > VIN. Recommends EN after VIN or tying EN to VIN. If EN > VIN, limit toggle frequency to < 10 kHz. | "If this increased flow causes problems in the application, sequence the EN pin after VIN is high, or to tie EN to VIN" | 13 | 6.3.2 |
| UVLO | VUVLO, VIN rising: min 1.21 / typ 1.3 / max 1.44 V. Hysteresis (VIN falling): 40 mV, single value. Below UVLO the output is connected to ground with 120 Ω. | "VUVLO Undervoltage lockout VIN rising 1.21 1.3 1.44 V"; "VUVLO,HYST ... 40 mV"; "When VIN is less than VUVLO, the output is connected to ground with a 120Ω pulldown resistor." | 6; 12 | 5.5; 6.3.1 |
| Start-up time tSTR | 550 µs, single value, column not recoverable. No test conditions in the row. | "tSTR Startup time 550 µs" | 6 | 5.5 |
| Soft-start / inrush | Built-in soft-start with monotonic VOUT rise. Internal soft-start lowers inrush. No inrush current or soft-start-time figure beyond tSTR. | "This device features an internal soft-start to lower inrush current" | 1 | Description |
| Start-up current limit | Foldback current limit during start-up. A constant-current load, or negative output at enable, can stop the output reaching full voltage. | "If the output is loaded by a constant-current load during start-up, or if the output voltage is negative when the device is enabled, then the load current demanded by the load potentially exceeds the foldback current limit and the device does not rise to the full output voltage." | 13 | 6.3.3 |
| Active output discharge | Yes ("P" version). Internal 120 Ω pull-down (RPULLDOWN: 120 Ω at VIN = 5.5 V). On when disabled; "on when sufficient input voltage is provided". | "When shutdown, the device actively pulls down the output"; "the output voltage is actively discharged to ground by an internal switch from the output to ground. The active pulldown is on when sufficient input voltage is provided."; "RPULLDOWN Pulldown resistance VIN = 5.5 V 120 Ω" | 1; 14; 6; 23 | Description; 6.4.3 Disabled; 5.5; Table 8-1 |
| Reverse current (output above input) | Excessive reverse current potentially damages the device. It flows through the pass-transistor body diode. The listed conditions exceed the abs max of VOUT > VIN + 0.3 V: large COUT with input collapse, "The output is biased when the input supply is not established", "The output is biased above the input supply". Use external protection (Schottky example, Fig 7-3). No allowed reverse-current figure. | "As with most LDOs, excessive reverse current potentially damages this device." | 17-18 | 7.1.4 Reverse Current |
| Abs max EN | −0.3 to 6.0 V. Recommended 0 to 5.5 V. | "Enable voltage, VEN −0.3 6.0 V" | 4 | 5.1; 5.3 |
| Abs max VOUT / VIN | VIN −0.3 to 6.0 V. VOUT −0.3 to VIN + 0.3 V (note 2: the smaller of VIN + 0.3 V or 6.0 V). | "Output voltage, VOUT −0.3 VIN + 0.3(2) V" | 4 | 5.1 |
| VIN operating range | 1.45 to 5.5 V | "VIN Input voltage 1.45 5.5 V" | 4 | 5.3 |

---

## C. TXU0304 (SCES935A)

| item | fact | quote | page | table/section |
|---|---|---|---|---|
| Ioff / partial power-down | Feature present, and "fully specified for partial-power-down". Inputs and outputs go Hi-Z when powered down. Ioff max (A or B port, VI or VO = 0 to 5.5 V, other supply 0 V): ±1.5 µA at 25 °C, ±2 µA at −40 to 85 °C, ±2.5 µA at −40 to 125 °C. Both cases, VCCA = 0 with VCCB 0-5.5 V and VCCB = 0 with VCCA 0-5.5 V, carry the same limits. | "Ioff supports partial-power-down mode operation"; "The inputs and outputs for this device enter a high-impedance state when the device is powered down, inhibiting current backflow into the device." | 1; 21; 8 | Features; 9.3.4 Partial Power Down (Ioff); 7.5 Electrical Characteristics (continued) |
| Ioff-float | With a supply floating, VI or VO = GND: ±1.5 / ±2 / ±2.5 µA (same three temperature columns). "Floating" = "not actively driven by an external device and has leakage not exceeding 10nA". | "Floating is defined as a node that is both not actively driven by an external device and has leakage not exeeding 10nA" (sic) | 8; 9 | 7.5 and note 5 |
| VCC isolation / disconnect | If either VCC < 100 mV or disconnected (with the complementary supply within recommended operating conditions), all outputs are disabled and Hi-Z. The supply current is maintained. Recommends keeping inputs low before floating either supply. Behaviour between 100 mV and the 1.08 V minimum operating VCC is not stated. | "If either VCC input is <100 mV or disconnected, all outputs are disabled and become high-impedance"; "The outputs for this device are disabled and enter a high-impedance state when either supply is <100 mV or left floating (disconnected), with the complementary supply within recommended operating conditions. It is recommended that the inputs are kept low before floating (disconnecting) either supply." | 1; 21 | Features; 9.3.5 VCC Isolation and VCC Disconnect (also 9.1 Overview, p20) |
| Supply current when other supply is 0 or floating | ICCA/ICCB rows with the other supply at 0 V, or "Floating(5)" with the other supply at 5.5 V: values extracted unlabelled by column (e.g. floating case 1.5 / 7 / 15 µA). Not reduced to a single claim here. | see Electrical Characteristics ICCA/ICCB | 8-9 | 7.5 |
| Power-up sequencing | Any order, no ramp-order or ramp-rate requirement. Apply ground first. | "Either supply rail may be powered on or off in any order without producing a glitch on the inputs or outputs"; "Always apply a ground reference to the GND pins first. This device is designed for glitch free power sequencing without any supply sequencing requirements such as ramp order or ramp rate." | 22; 25 | 9.3.7 Glitch-Free Power Supply Sequencing; 11 Power Supply Recommendations |
| OE on power-up | OE can be left floating or pulled to ground to keep outputs Hi-Z during power up/down. OE low gives Hi-Z, "Isolation". OE can be referenced to either VCCA or VCCB (VCC(MIN) circuitry). | "The OE pin can be left floating or externally pulled down to ground to ensure the high-impedance state of the level shifter outputs during power up or power down." | 20; 23 | 9.1 Overview; 9.4 Table 9-1 Function Table (OE = L: Hi-Z, Isolation) |
| OE pin text | "Output Enable. Pull to GND to place all outputs in high-impedance mode. Pull to VCCA or VCCB to enable all outputs." | quoted | 4 | 4 Pin Configuration and Functions |
| Input abs max vs VCC | Absolute maximum is NOT relative to VCC. VI on A port, B port and OE: −0.5 to 6.5 V. VO in Hi-Z or power-off state: −0.5 to 6.5 V. VO in high/low state: −0.5 to VCCO + 0.5 V (note 3: may exceed up to 6.5 V if current rating observed). Supply: −0.5 to 6.5 V. Recommended VI 0 to 5.5 V. Recommended tri-state VO 0 to 5.5 V. | "VI Input Voltage(2) I/O Ports (A Port) –0.5 6.5 V"; "VO Voltage applied to any output in the high-impedance or power-off state(2) ... –0.5 6.5" | 5; 6 | 7.1 Absolute Maximum Ratings; 7.3 Recommended Operating Conditions |
| Inputs driven with VCC = 0 | Not stated as such. Related statements: over-voltage tolerant inputs "Input signals to this device can be driven above the supply voltage so long as they remain below the maximum input voltage value specified in the Recommended Operating Conditions" (that maximum is 5.5 V); the Ioff row covers VI = 0-5.5 V with the other supply at 0 V (leakage limits above). | quoted | 22; 8 | 9.3.6 Over-Voltage Tolerant Inputs; 7.5 |
| Internal pull-downs | Weak pull-downs on all inputs and data I/Os: 5 MΩ typical. External pull-up, if used, no larger than 1 MΩ. | "This device has 5 MΩ typical integrated weak pull-downs for each input."; footnote: "All control inputs and data I/Os of this device have weak pulldowns to ensure the line is not floating when undefined external to the device." | 21; 6 | 9.3.1.1; 7.3 note (3) |
| Recommended VCC | 1.08 to 5.5 V on both VCCA and VCCB | "VCCA Supply voltage A 1.08 5.5 V" | 6 | 7.3 |

---

## D. DRV8876 (SLVSDS7B)

| item | fact | quote | page | table/section |
|---|---|---|---|---|
| IPROPI in sleep (nSLEEP low) | NOT STATED. Sleep text lists what is disabled (H-bridge, charge pump, internal 5-V regulator, internal logic) and says nothing about IPROPI output state or Hi-Z. | "In sleep mode, the H-bridge, charge pump, internal 5-V regulator, and internal logic are disabled. The device relies on a weak pulldown to ensure all of the internal MOSFETs remain disabled." | 18 | 7.4.2 Low-Power Sleep Mode |
| IPROPI function | Analog current proportional to the low-side MOSFET current: AIPROPI = 1000 µA/A (single value; the AERR error terms are separate). An internal clamp limits VIPROPI with respect to VVREF to protect the ADC. No clamp voltage value is stated. | "the DRV887x devices implements an internal IPROPI voltage clamp circuit to limit VIPROPI with respect to VVREF on the VREF pin and protect the external ADC" | 12; 6 | 7.3.3.1 Current Sensing; 6.5 |
| IIPROPI max | Recommended operating: 0 to 3 mA ("Current sense output current"). No absolute-max current for IPROPI. | "IIPROPI Current sense output current IPROPI 0 3 mA" | 4 | 6.3 Recommended Operating Conditions |
| IPROPI abs max voltage | The row has TWO value pairs in the extraction: "–0.3 5.75 V" and "–0.3 VVM + 0.3 V", so the grouping is ambiguous. | "Proportional current output pin voltage IPROPI –0.3 5.75 V –0.3 VVM + 0.3 V" | 4 | 6.1 Absolute Maximum Ratings |
| VM UVLO | VM rising: min 4.3 / typ 4.45 / max 4.6 V. VM falling: 4.2 / 4.35 / 4.5 V. Hysteresis 100 mV. Deglitch tUVLO 10 µs. Recommended VM 4.5 to 37 V. | "VUVLO Supply undervoltage lockout (UVLO) VVM rising 4.3 4.45 4.6 V VVM falling 4.2 4.35 4.5 V" | 6; 4 | 6.5 PROTECTION CIRCUITS; 6.3 |
| UVLO behaviour | All MOSFETs disabled, nFAULT driven low, charge pump disabled. Resumes when VM > VUVLO. | "all MOSFETs in the H-bridge will be disabled and the nFAULT pin driven low. The charge pump is disabled in this condition." | 16 | 7.3.4.1 VM Supply Undervoltage Lockout; Table 7 |
| nFAULT | Open-drain. Abs max −0.3 to 5.75 V. Recommended pull-up voltage 0 to 5.5 V, IOD 0 to 5 mA. VOL 0.3 V at 5 mA (single value). IOZ ±2 µA at VOD = 5 V. Pulled low on faults (UVLO, CPUV, OCP, TSD) and as the ITRIP indicator. Behaviour in sleep: not stated. | "Open-drain output pin voltage nFAULT –0.3 5.75 V"; "nFAULT ... Fault indicator output. Pulled low during a fault condition." | 4; 3; 6; 16 | 6.1; 6.3; Pin Functions; 6.5 OPEN-DRAIN OUTPUTS; Table 7 |
| VREF abs max | −0.3 to 5.75 V. Recommended 0 to 3.6 V. | "Reference input pin voltage VREF –0.3 5.75 V" | 4 | 6.1; 6.3 |
| nSLEEP pull-down | Internal pull-down resistor. Input pulldown resistance RPD = 100 kΩ (single value, column not recoverable). The table heading covers EN/IN1, PH/IN2, nSLEEP. | "Sleep mode input. Logic high to enable device. Logic low to enter low-power sleep mode. ... Internal pulldown resistor." ; "RPD Input pulldown resistance To GND 100 kΩ" | 3; 5 | Pin Functions; 6.5 LOGIC-LEVEL INPUTS |
| Logic-input abs max | EN/IN1, IMODE, nSLEEP, PH/IN2, PMODE: −0.3 to 5.75 V. Recommended VIN 0 to 5.5 V. VIL max 0.7 V (VVM < 5 V) / 0.8 V (VM ≥ 5 V). VIH min 1.5 V. | "Logic pin voltage EN/IN1, IMODE, nSLEEP, PH/IN2, PMODE –0.3 5.75 V" | 4; 5 | 6.1; 6.3; 6.5 |
| VM-logic sequencing | Only statement: inputs may be powered before VM. nSLEEP high plus tWAKE (1 ms) after VM > VUVLO to become active. | "The device input pins can be powered before VM is applied with no issues." | 11; 17 | 7.3.2 Control Modes; 7.4.1 Active Mode |
| Sleep timing and current | IVMQ sleep current at VM = 24 V, nSLEEP = 0, TJ = 25 °C: values 0.75 and 1 µA (columns not recoverable); with nSLEEP = 0 over temperature 5 µA. tWAKE 1 ms; tSLEEP 1 ms (single values). | "IVMQ VM sleep mode current VVM = 24 V, nSLEEP = 0 V, TJ = 25°C 0.75 1 µA" | 5 | 6.5 |
| V(IPROPI) > V(VREF): ITRIP | The ITRIP comparator compares the voltage across RIPROPI to VVREF: ITRIP × AIPROPI = VVREF / RIPROPI. When ITRIP is exceeded, the outputs enter current chopping per IMODE. (The "limit VIPROPI with respect to VVREF" clamp is the separate mechanism above.) Regulation can be disabled by tying IPROPI to GND with VREF > GND. | "When the ITRIP threshold is exceeded, the outputs will enter a current chopping mode according to the IMODE setting." ; "ITRIP (A) x AIPROPI (μA/A) = VVREF (V) / RIPROPI (Ω)" | 13 | 7.3.3.2 Current Regulation |
| Current-regulation timing | tOFF 25 µs; tDELAY 1.6 µs; tDEG 0.6 µs; tBLK 1.1 µs (single values) | rows quoted | 6 | 6.5 CURRENT SENSE AND REGULATION |

---

## E. DRV8833 (SLVSAR1E)

| item | fact | quote | page | table/section |
|---|---|---|---|---|
| nFAULT | Open-drain output, "Logic low when in fault condition (overtemperature, overcurrent)" in the pin table. VOL 0.5 V at IO = 5 mA. IOH leakage 1 µA at VO = 3.3 V. Driven low in UVLO. Abs max for the nFAULT pin: not stated. The only abs max voltage is "Digital input pin voltage". | "nFAULT is driven low in the event of an undervoltage condition." | 4; 6; 11 | Pin Functions; 6.5 nFAULT OUTPUT; UVLO text |
| VM UVLO | VM falling 2.6 V (single value, column not recoverable). Hysteresis VHYS 90 mV. Rising threshold not tabulated. Recommended VM 2.7 to 10.8 V. All circuitry disabled and internal logic reset when VM < threshold. | "VUVLO VM undervoltage lockout voltage VM falling 2.6 V"; "all circuitry in the device will be disabled, and all internal logic will be reset" | 6; 5; 11 | 6.5; 6.3; Undervoltage Lockout (UVLO) |
| Inputs abs max | Digital input pin voltage −0.5 to 7 V (abs max). Recommended −0.3 to 5.75 V. VM abs max −0.3 to 11.8 V. | "Digital input pin voltage –0.5 7 V" | 5 | 6.1; 6.3 |
| nSLEEP pull-down | Internal 500 kΩ to GND, plus a Zener clamp at 6.5 V. Currents > 250 µA can damage the pin. Recommended pull-up 20-75 kΩ if tied to VM. Other inputs: pull-down 150 kΩ (RPD, single value). | "Internally, the nSLEEP pin has a 500-kΩ resistor to GND. It also has a clamping Zener diode that clamps the voltage at the pin at 6.5 V." | 11; 6 | 7.3.4 nSLEEP Operation; 6.5 |
| Sleep | H-bridges disabled (Hi-Z), charge pump stopped, logic reset. IVMQ 1.6 / 2.5 µA (columns not recoverable). Up to 1 ms to wake. | "Driving nSLEEP low will put the device into a low power sleep state." | 11; 6 | 7.3.4; 6.5 |
| Sequencing | No specific order. Digital inputs may be present before VM. | "There is no specific sequence for powering up the DRV8833. The presence of digital input signals is acceptable before VM is applied." | 14 | Power Supply and Logic Sequencing |

---

## F. DS12117 Rev 10 (STM32H753xI), Electrical characteristics (rev Y) unless stated

Extraction note: Table 10 and Table 11 are interleaved with their footnotes in the text. The grouping below is reconstructed from the order of cells and checked for consistency (the rev V twin has the same layout). Verify against the PDF if a cell matters.

### F1. Table 10, Voltage characteristics (printed p.103, notes 3-5 on p.104)

| item | fact | quote | page | table/section |
|---|---|---|---|---|
| VDDX − VSS | External main supply −0.3 to 4.0 V (including VDD, VDDLDO, VDDA, VDD33USB, VBAT) | "VDDX - VSS External main supply voltage (including VDD, VDDLDO, VDDA, VDD33USB, VBAT) −0.3 4.0 V" | 103 | Table 10 |
| VIN, FT_xxx pins | min VSS−0.3 V. Max = Min(Min(VDD, VDDA, VDD33USB, VBAT) + 4.0, 6 V), notes (3)(4)(5). | "Input voltage on FT_xxx pins VSS−0.3 Min(Min(VDD, VDDA, VDD33USB, VBAT) + 4.0, 6 V)(3)(4)(5) V" | 103 | Table 10 |
| VIN, TT_xx pins | VSS−0.3 to 4.0 V | "Input voltage on TT_xx pins VSS-0.3 4.0 V" | 103 | Table 10 |
| VIN, BOOT0 | VSS to 9.0 V | "Input voltage on BOOT0 pin VSS 9.0 V" | 103 | Table 10 |
| VIN, any other pins | VSS−0.3 to 4.0 V | "Input voltage on any other pins VSS-0.3 4.0 V" | 103 | Table 10 |
| ΔVDDX / VSSx-VSS | 50 mV max each | "Variations between different VDDX power pins of the same domain - 50 mV" | 103 | Table 10 |
| Note 1 | "All main power (VDD, VDDA, VDD33USB, VBAT) and ground (VSS, VSSA) pins must always be connected to the external power supply, in the permitted range." | quoted | 103 | Table 10 note 1 |
| Note 2 (on VIN) | "VIN maximum must always be respected. Refer to Table 48 for the maximum allowed injected current values." | quoted | 103 | Table 10 note 2 |
| Note 3 | "This formula has to be applied on power supplies related to the IO structure described by the pin definition table." | quoted | 104 | Table 10 note 3 |
| Note 4 | "To sustain a voltage higher than 4V the internal pull-up/pull-down resistors must be disabled." | quoted | 104 | Table 10 note 4 |
| Note 5 | "When an FT_a pin is used by an analog peripheral such as an ADC, the maximum VIN is 4 V." | quoted | 104 | Table 10 note 5 |

Recommended-operating (not abs max), Table 13 General operating conditions, p.105: VIN "All I/O except BOOT0 and TT_xx: −0.3 to Min(VDD, VDDA, VDD33USB) +3.6V < 5.5V(2)(3)"; "TT_xx I/O −0.3 to VDD+0.3"; "BOOT0 0 to 9". Note 2 is the same formula note. Note 3: "For operation with voltage higher than Min (VDD, VDDA, VDD33USB) +0.3V, the internal Pull-up and Pull-Down resistors must be disabled."

### F2. Table 11, Current characteristics (p.104)

| item | fact | quote | page | table/section |
|---|---|---|---|---|
| IINJ(PIN), FT_xxx, TT_xx, RST and B pins except PA4, PA5 | −5/+0 (mA). The unit column of Table 11 reads mA for the neighbouring rows. | "Injected current on FT_xxx, TT_xx, RST and B pins except PA4, PA5 −5/+0" | 104 | Table 11 |
| IINJ(PIN), PA4, PA5 | −0/0 | "Injected current on PA4, PA5 −0/0" | 104 | Table 11 |
| ΣIINJ(PIN) | ±25 (mA), total injected current, note (5) | "Total injected current (sum of all I/Os and control pins)(5) ... ±25" | 104 | Table 11 |
| Note 3 | "Positive injection is not possible on these I/Os and does not occur for input voltages lower than the specified maximum value." | quoted | 104 | Table 11 note 3 (on IINJ(PIN)) |
| Note 4 | "A positive injection is induced by VIN>VDD while a negative injection is induced by VIN<VSS. IINJ(PIN) must never be exceeded. Refer also to Table 10: Voltage characteristics for the maximum allowed input voltage values." | quoted | 104 | Table 11 note 4 |
| Note 5 | "When several inputs are submitted to a current injection, the maximum ∑IINJ(PIN) is the absolute sum of the positive and negative injected currents (instantaneous values)." | quoted | 104 | Table 11 note 5 |
| IIO | 20 (sunk by any I/O, except Px_C); Px_C pins 1. ΣI(PIN) 140 sunk, 140 sourced. | rows quoted | 104 | Table 11 |

### F3. Table 48, I/O current injection susceptibility (p.136). Unit mA. Columns: negative injection, positive injection. Footnote 1: "Guaranteed by characterization."

| pins | negative | positive |
|---|---|---|
| PA7, PC5, PG1, PB14, PJ7, PA11, PA12, PA13, PA14, PA15, PJ12, PB4 | 5 | 0 |
| PA2, PH2, PH3, PE8, PA6, PA7, PC4, PE7, PE10, PE11 | 0 | NA |
| PA0, PA_C, PA1, PA1_C, PC2, PC2_C, PC3, PC3_C, PA4, PA5, PH4, PH5, BOOT0 | 0 | 0 |
| All other I/Os | 5 | NA |

- PA7 appears in both the first and second rows, as extracted. The datasheet text is not self-consistent on it. Treat it as unresolved.
- The row-to-number pairing comes from the extraction order and should be spot-checked on p.136.
- Intro text, "6.3.14 I/O current injection characteristics", p.135: "As a general rule, a current injection to the I/O pins, due to external voltage below VSS or above VDD (for standard, 3.3 V-capable I/O pins) should be avoided during the normal product operation."
- The failure criterion includes induced leakage on adjacent pins "out of –5 µA/+0 µA range" (p.135-136).

### F4. Power sequence, §3.5.1 Power supply scheme (printed p.27, Figure 2 on p.28)

| item | fact | quote | page | table/section |
|---|---|---|---|---|
| VDD < 1 V | Other supplies (VDDA, VDD33USB, VDD50USB) must stay below VDD + 300 mV | "When VDD is below 1 V, other power supplies (VDDA, VDD33USB, VDD50USB) must remain below VDD + 300 mV." | 27 | 3.5.1 Power supply scheme |
| VDD > 1 V | Supplies independent | "When VDD is above 1 V, all power supplies are independent." | 27 | 3.5.1 |
| Power-down | VDD may temporarily be lower than other supplies only if energy to the MCU stays below 1 mJ | "During the power-down phase, VDD can temporarily become lower than other supplies only if the energy provided to the microcontroller remains below 1 mJ." | 27 | 3.5.1 |
| Scope | The requirement applies "During power-up and power-down phases". VBAT is "power supply for the VSW domain when VDD is not present". | quoted | 27 | 3.5.1 |

### F5. "VDD = 0 / unpowered / not present" text, and the 5 V statement

| item | fact | quote | page | table/section |
|---|---|---|---|---|
| I/O pin voltage with VDD = 0 or unpowered | NOT STATED. Full-text search ('not present', 'unpowered', 'power-down', 'VDD = 0', 'injection', 'powered off', 'back-power') found no statement on pin voltage or back-powering with VDD = 0 or unpowered. "VDD not present" occurs only for the VBAT domain (pp.27, 36). The only governing text is the Table 10 formula (FT-pin VIN max follows Min(VDD, VDDA, VDD33USB, VBAT) + 4.0). Evaluating that formula with VDD = 0 is my derivation, not a datasheet statement. Table 11 note 4 defines positive injection as VIN > VDD. | – | – | – |
| Table 7 legend | "FT 5 V tolerant I/O"; "TT 3.3 V tolerant I/O"; "B Dedicated BOOT0 pin"; "RST Bidirectional reset pin with embedded weak pull-up resistor"; "_f I2C FM+ option"; "_a analog option (supplied by VDDA)"; "_u USB option (supplied by VDD33USB)"; "_h High-speed low-voltage I/O" | quoted | 63 | Table 7 Legend/abbreviations used in the pinout table |
| 5 V-tolerant only when VDD present | NOT STATED as such. The practical limit is Table 10/13 (VIN max = Min(VDD, VDDA, VDD33USB, VBAT) + 4.0 capped at 6 V; Table 13: + 3.6 V, < 5.5 V). Leakage for FT_xx I/O with Max(VDDXXX) < VIN ≤ 5.5 V is tabulated: 1500 (nA, all FT_xx except FT_lu, FT_u, PC3), FT_u 5000 nA. | "Max(VDDXXX) < VIN ≤ 5.5 V ... 1500" | 137 | Table 49 I/O static characteristics, Ilkg, notes 3-8 |
| Table 49 notes | Note 5: "VIN must be less than Max(VDDXXX) + 3.6 V." Note 6: "To sustain a voltage higher than MIN(VDD, VDDA, VDD33USB) +0.3 V, the internal pull-up and pull-down resistors must be disabled." Note 4: "All FT_xx IO except FT_lu, FT_u and PC3." Note 8: "Max(VDDXXX) is the maximum value of all the I/O supplies." | quoted | 137 | Table 49 notes |
| RPU / RPD | 30 / 40 / 50 kΩ (min/typ/max) | "RPU Weak pull-up equivalent resistor VIN=VSS 30 40 50 kΩ" | 137 | Table 49 |

### F6. Table 8 pin/ball definition, I/O structure, LQFP144 (Table 8 columns: LQFP100, TFBGA100, LQFP144, ...)

I/O structure as extracted. A trailing "a" on its own line after FT_h means the option string FT_ha ("_h" high-speed low-voltage, "_a" analog option). The pin-type column shows I/O for all.

| pin | LQFP144 pin no. | I/O structure | page |
|---|---|---|---|
| PF3 | 13 | FT_h + a (FT_ha) | 65 |
| PF4 | 14 | FT_h + a (FT_ha) | 65 |
| PF5 | 15 | FT_h + a (FT_ha) | 66 |
| PF10 | 22 | FT_h + a (FT_ha) | 66 |
| PH0 (OSC_IN) | 23 | FT | 66 |
| PH1 (OSC_OUT) | 24 | FT | 66 |
| PC0 | 26 | FT_a | 67 |
| PC1 | 27 | FT_h + a (FT_ha) | 67 |
| PA0 | 34 | FT_a | 68 |
| PA3 | 37 | FT_h + a (FT_ha) | 69 |
| PA4 | 40 | TT_a | 69 |
| PC4 | 44 | TT_a | 70 |
| PC5 | 45 | TT_a | 70 |

### F7. Rev V twins (same text, in the second electrical section)

- Table 108 Voltage characteristics: p.207, notes on p.208.
- Table 109 Current characteristics: p.208.
- Table 145 I/O current injection susceptibility: p.240.
- The extracted rows and notes are word-for-word the same as rev Y. The notes refer to "Table 145".

---

## G. DS-009 (Pixhawk Reference Standard, rev 0.1.0). Printed page numbers.

| item | fact | quote | page | table/section |
|---|---|---|---|---|
| Telemetry port | TX/RX/CTS/RTS +3.3V in the Volt column. VCC +5V. Ports without flow control leave CTS and RTS floating. | "2 (blk) TX (OUT) +3.3V ... 6 (blk) GND GND" | 5 | Telemetry Port |
| Full GPS plus Safety Switch | TX, RX, I2C1 SCL/SDA, safety switch in, safety LED, VDD_3V3 all +3.3V. Buzzer +5-24V. VCC +5V. | "4 (blk) I2C1 SCL +3.3V" | 6 | Full GPS plus Safety Switch Port |
| Basic GPS | TX, RX, I2C1 SCL/SDA +3.3V. VCC +5V. | quoted | 7 | Basic GPS port |
| I2C port | VCC +5V. SCL and SDA labelled "(1.5K pullup on autopilot)", signal voltage +3.3V. The text does not name the voltage the pull-up is tied to. The 3.3 V in the Volt column is the signal level. | "I2C SCL (1.5K pullup on autopilot) +3.3V"; "I2C SDA (1.5K pullup on autopilot) +3.3V" | 8 | I2C port |
| SPI port | SPI signals +3.3, VCC +5V | quoted | 8 | SPI port |
| Debug (full and mini) | Signals "+1.8V-3.3V", nominal 3.3 V; the probe should shift to the target sense voltage. Up to 100 mA on TARGET PROCESSOR VOLTAGE. CONSOLE TX/RX +1.8V-3.3V. | "The indicated voltage is nominal 3.3V, but for a target running at a lower voltage (e.g. 1.8V) the debug probe should shift its voltage to the target sense voltage." | 10; 11 | Pixhawk Debug Full; Pixhawk Debug Mini |
| 5 V tolerance of signal lines | not stated. Volt column gives only nominal levels. | – | – | – |
| Peripherals driving lines while autopilot unpowered | not stated | – | – | – |
| VCC | +5V on the connectors; analog power connector VCC +5.3V: "The VCC lines have to offer at least 2.5A continuous and should default to 5.3V. A lower voltage of 5V is still acceptable, but discouraged." | quoted | 9 (the sentence follows the Analog Power heading on p.8) | Analog Power |

---

## "Not stated" list

1. LMR33630: pre-biased output start-up behaviour (sink or discharge). Not stated.
2. LMR33630: output discharge in shutdown. Not stated.
3. LMR33630: numeric VIN-pin UVLO rising/falling. Not stated. Only VCC-LDO UVLO "about" 3.7 V / "about" 3 V, plus minimum VIN 3.8 V and a graph (Fig 7-5).
4. LMR33630: a separate EN-to-switching delay figure. Not stated beyond tSS and the PG delay.
5. LMR33630: PG-to-EN sequencing statements. Not stated.
6. LMR33630: column (min/typ/max) for VEN-VCC-H, VEN-VCC-L, VEN-HYS, VIN-PG and tPG single values. Not recoverable from the extraction.
7. TLV755P: soft-start duration separately from tSTR (550 µs, column not recoverable). Not stated. Max reverse current. Not stated.
8. TXU0304: output state for a VCC between 100 mV and 1.08 V. Not stated. Explicit "inputs driven while VCC = 0" statement. Not stated, only the Ioff limits.
9. DRV8876: IPROPI state or current in sleep. Not stated. nFAULT state in sleep. Not stated. IPROPI clamp voltage value. Not stated. Absolute-max current for IPROPI. Not stated. IPROPI abs max grouping, ambiguous.
10. DRV8833: nFAULT abs max voltage. Not stated. VM UVLO rising threshold. Not stated. Typ/max column of "2.6 V". Not recoverable.
11. DS12117: I/O pin voltage with VDD = 0 or unpowered. Not stated. Statement that FT pins are 5 V tolerant only with VDD present. Not stated explicitly. Only the Table 10/13 formulas.
12. DS12117: Table 48 PA7 appears in two rows (extraction). Unresolved.
13. DS-009: 5 V tolerance, peripherals driving while the autopilot is unpowered, and the pull-up rail voltage. Not stated.
