# AET-WO-4411 cited facts: ESP32-WROOM-32E/32UE and ESP32-C3-WROOM-02/02U

Sources (all under /home/forge/.cache/aet-4411/ds/):
- **DS-32E** = esp32-wroom-32e_esp32-wroom-32ue_datasheet_en.pdf, Datasheet Version 2.1 (revision history date 2026-08-05, p50). PDF page = printed page.
- **DS-C3** = esp32-c3-wroom-02_datasheet_en.pdf, Datasheet Version 1.7 (revision history date 2026-03-05, p44). PDF page = printed page.
- **HDG-32** = hdg-esp32.pdf, "ESP32 Hardware Design Guidelines, Release master, Nov 12, 2026". Printed page = PDF page - 3.
- **HDG-C3** = hdg-esp32c3.pdf, "ESP32-C3 Hardware Design Guidelines, Release master, Nov 12, 2026". Printed page = PDF page - 3.

Cite format: PDF p<N> (printed <M> if different). "Read from render" means the value came from viewing the rendered page image, not the text layer. Anything labelled "my reading" is my interpretation of a drawing and not a quote.

---------------------------------------------------------------------

## 1. Supply

### ESP32-WROOM-32E / WROOM-32UE
- Operating VDD33 = 3.0 / 3.3 / 3.6 V (min/typ/max). DS-32E PDF p28, Table 14 "Recommended Operating Conditions": `VDD33 Power supply voltage 3.0 3.3 3.6 V`. Also p2, 1.1 Features: "Operating voltage/Power supply: 3.0 ~ 3.6 V".
- Minimum current the external supply must deliver: 0.5 A. DS-32E p28, Table 14: `IVDD Current delivered by external power supply 0.5 — — A`.
- HDG-32 PDF p9 (printed 6), 1.3.2: "the recommended power supply voltage is 3.3 V and the output current is no less than 500 mA."
- HDG-32 p10 (printed 7), VDD3P3 text: "when ESP32 is transmitting signals, there may be a sudden increase in the current draw, causing power rail collapse. Therefore, it is highly recommended to add a 10 μF capacitor to the power rail". Also: "add an ESD protection diode and at least 10 μF capacitor at the main power entrance".

### ESP32-C3-WROOM-02 / WROOM-02U
- Operating VDD33 = 3.0 / 3.3 / 3.6 V. DS-C3 PDF p21, Table 6-2: `VDD33 Power supply voltage 3.0 3.3 3.6 V`. Also p3, 1.1 Features: "Operating voltage/Power supply: 3.0 ~ 3.6 V".
- Minimum supply current: 0.5 A. DS-C3 p21, Table 6-2: `IVDD Current delivered by external power supply 0.5 — — A`.
- HDG-C3 PDF p8 (printed 5): "the recommended power supply voltage is 3.3 V and the output current is no less than 500 mA." Also the 10 μF + ESD diode at the main power entrance.
- DS-C3 p15, Fig 4-2: the power-up diagram marks a 2.8 V level on the VDDA/VDD3P3/VDD3P3_RTC/VDD3P3_CPU rails, with CHIP_EN rising after it (read from render). The text gives no volts figure for this in Table 4-6.

---------------------------------------------------------------------

## 2. Current (Wi-Fi TX peak, RX)

Both datasheets say, in 6.4: "The current consumption measurements are taken with a 3.3 V supply at 25 °C ambient temperature. TX current consumption is rated at a 100% duty cycle. RX current consumption is rated when the peripherals are disabled and the CPU idle." (DS-32E p29; DS-C3 p22.) Neither defines how "Peak" is measured.

### ESP32-WROOM-32E / 32UE: DS-32E PDF p29, Table 16 "Current Consumption Depending on RF Modes"
The table has BOTH an Average and a Peak column. The peak is the value to design to.

| TX mode (100% duty, 3.3 V, 25 C) | Average (mA) | **Peak (mA)** |
|---|---|---|
| 802.11b, 20 MHz, 1 Mbps, @19.5 dBm | 239 | **379** |
| 802.11g, 20 MHz, 54 Mbps, @15 dBm | 190 | **276** |
| 802.11n, 20 MHz, MCS7, @13 dBm | 183 | **258** |
| 802.11n, 40 MHz, MCS7, @13 dBm | 165 | **211** |

- RX (same table; average and peak are equal): 802.11b/g/n 20 MHz 112 / 112 mA; 802.11n 40 MHz 118 / 118 mA. Condition: peripherals disabled, CPU idle.
- Quote: `TX 802.11b, 20 MHz, 1 Mbps, @19.5 dBm 239 379`.
- Wi-Fi TX power the module targets (p31, Table 20): 802.11b 1/11 Mbps 19.5 dBm typ; g 6 Mbps 18.0; g 54 Mbps 14.0; n HT20 MCS0 18.0; HT20 MCS7 13.0; HT40 MCS0 18.0; HT40 MCS7 13.0.
- The Bluetooth TX/RX current rows were not extracted; not requested.

### ESP32-C3-WROOM-02 / 02U: DS-C3 PDF p22, Table 6-4 "Current Consumption for Wi-Fi (2.4 GHz) in Active Mode"
This table has ONLY a Peak (mA) column. No average is given for TX or RX.

| TX mode (100% duty, 3.3 V, 25 C) | **Peak (mA)** |
|---|---|
| 802.11b, 1 Mbps, @20.5 dBm | **345** |
| 802.11g, 54 Mbps, @18 dBm | **285** |
| 802.11n, HT20, MCS7, @17.5 dBm | **280** |
| 802.11n, HT40, MCS7, @17 dBm | **280** |

- RX (Peak column): 802.11b/g/n HT20 82 mA; 802.11n HT40 84 mA. Condition: peripherals disabled, CPU idle.
- Quote: `802.11b, 1 Mbps, @20.5 dBm 345`.
- Wi-Fi TX power (DS-C3 p24, Table 7-2): 802.11b 1/11 Mbps 20.5 dBm typ; g 6 Mbps 20.0; g 54 Mbps 18.0; n HT20 MCS0 19.0; HT20 MCS7 17.5; HT40 MCS0 18.5; HT40 MCS7 17.0.

---------------------------------------------------------------------

## 3. EN / CHIP_PU

### ESP32-WROOM-32E / 32UE (module pin 3 "EN", chip pin CHIP_PU)
- Function. DS-32E p11, Table 3: `EN 3 I High: On; enables the chip / Low: Off; the chip shuts down / Note: Do not leave the pin floating.`
- Levels, p29, Table 15: VIH_nRST (chip reset release voltage) min 0.75 x VDD1, max VDD1 + 0.3 V. VIL_nRST (low-level input voltage of CHIP_PU to shut down the chip) max 0.6 V.
- Timing, p16-17, Fig 6 and Table 9: tSTBL "Time reserved for the 3.3 V rails to stabilize before the CHIP_PU pin is pulled high to activate the chip" min 50 us. tRST "Time reserved for CHIP_PU to stay below VIL_nRST to reset the chip" min 50 us. Fig 6 is read from render: it shows VDD rising to "VDD3P3_RTC Min" with tSTBL, then CHIP_PU rising after it.
- RC delay, DS-32E p39, notes under Fig 9: "it is advised to add an RC delay circuit at the EN pin. The recommended setting for the RC delay circuit is usually R = 10 kΩ and C = 1 µF. However, specific parameters should be adjusted based on the power-up timing of the module and the power-up and reset sequence timing of the chip." No separate RC timing figure is given beyond Fig 6.
- HDG-32 p11 (printed 8): "CHIP_PU must not be left floating." The same RC recommendation (R = 10 kΩ, C = 1 uF). A power monitor around 3.0 V is recommended for slow rise/fall, frequent power cycling, or unstable supplies: "reserve a power monitor chip to reset the chip when the power supply is abnormal, and the threshold of the power monitor chip is recommended to be around 3.0 V." HDG-32 p10: "make the CHIP_PU trace as short as possible."
- Internal pull on EN: **not stated** (neither DS-32E nor HDG-32).

### ESP32-C3-WROOM-02 / 02U (module pin 2 "EN", chip pin CHIP_EN)
- Function. DS-C3 p11, Table 3-1: `EN 2 I High: on, enables the chip. Low: off, the chip powers off. Note: Do not leave the EN pin floating.`
- Levels, p21-22, Table 6-3: VIH_nRST min 0.75 x VDD1, max VDD1 + 0.3 V. VIL_nRST (chip reset voltage) -0.3 to 0.25 x VDD1 V.
- Timing, p15, Fig 4-2 and Table 4-6: tSTBL (VDDA, VDD3P3, VDD3P3_RTC, VDD3P3_CPU stable before CHIP_EN high) min 50 us. tRST min 50 us. Fig 4-2 marks "2.8 V" on the supply rails (read from render).
- RC delay, DS-C3 p33, notes under Fig 9-1: "R = 10 kΩ and C = 1 µF" recommended, "specific parameters should be adjusted based on the power-up timing of the module and the power-up and reset sequence timing of the chip."
- HDG-C3 p10 (printed 7): "CHIP_EN must not be left floating." Same RC and 3.0 V power-monitor advice. Reset voltage "(–0.3 ~ 0.25 × VDD) V".
- Internal pull on EN: **not stated**.

---------------------------------------------------------------------

## 4. Strapping pins

### ESP32-WROOM-32E / 32UE (DS-32E PDF p13-16, section 4 "Boot Configurations")
Strapping pins: GPIO0, GPIO2, MTDI, MTDO, GPIO5 (p13: "Strapping pin: GPIO0 and GPIO2 / MTDI / MTDO / MTDO and GPIO5"; p2: "5 strapping GPIOs").

Module pin numbers (Table 3, p11-12): GPIO0 = IO0 pin 25; GPIO2 = IO2 pin 24; MTDI = IO12 pin 14; MTDO = IO15 pin 23; GPIO5 = IO5 pin 29.

Default at reset (p13, Table 4 "Default Configuration of Strapping Pins"): GPIO0 pull-up = 1; GPIO2 pull-down = 0; MTDI pull-down = 0; MTDO pull-up = 1; GPIO5 pull-up = 1. The text says defaults are "determined by pins' internal weak pull-up/pull-down resistors at reset if the pins are not connected to any circuit, or connected to an external high-impedance circuit." Internal resistor value 45 kOhm typ (p29, Table 15 RPU/RPD).

Boot mode (p14, Table 6 "Chip Boot Mode Control"; bold = default, read from render):
- **SPI Boot Mode: GPIO0 = 1, GPIO2 = Any value** (default row).
- **Joint Download Boot Mode: GPIO0 = 0, GPIO2 = 0.** Methods: "SDIO Download Boot" and "UART Download Boot".
- p15: uart_download_dis permanently disables Download Boot when set to 1 ("valid only for ESP32 chip revisions v3.0 and higher").
- Note the default GPIO2 pull-down already satisfies the download-mode GPIO2 = 0. A low GPIO0 alone selects download.

VDD_SDIO voltage strap (p15, 4.2): "MTDI = 0 (by default), VDD_SDIO pin is powered directly from VDD3P3_RTC. Typically this voltage is 3.3 V." "MTDI = 1, VDD_SDIO pin is powered from internal 1.8 V LDO." This can be overridden by EFUSE_SDIO_FORCE / EFUSE_SDIO_TIEH.
- **What the module requires:** DS-32E p39, note on Fig 9 "Peripheral Schematics": "IO12 should be kept low when the module is powered on." That is MTDI = 0, so VDD_SDIO = 3.3 V.
- The module datasheet text never states the flash supply voltage explicitly. Table 17 (p30) lists flash VCC both as 1.8 V (1.65/1.80/2.00) and 3.3 V (2.7/3.3/3.6). The Fig 7 reference schematic shows the flash on VDD_SDIO. So the "keep IO12 low" requirement is the only explicit statement; the flash voltage itself is **not stated** in text.
- HDG-32 PDF p10 (printed 7), Table 1 "VDD_SDIO Voltage Control": EFUSE_SDIO_FORCE = 0, MTDI = 0 -> 3.3 V, "VDD3P3_RTC via RSPI (default)". EFUSE_SDIO_FORCE = 0, MTDI = 1 -> 1.8 V (flash voltage regulator). HDG-32 p9: "VDD_SDIO ... at either 1.8 V or 3.3 V (default)." HDG-32 p10: when VDD_SDIO is 3.3 V "ensure that VDD3P3_RTC remains above 3.0 V".

Other straps:
- MTDO (GPIO15) controls U0TXD printing (p16, Table 7): Enabled = 1 (default), Disabled = 0. MTDO + GPIO5 set SDIO slave timing (Table 8; default 1,1).
- DS-32E p39, Fig 9: "In module variants that have embedded QSPI PSRAM, i.e., that embed ESP32-D0WDR2-V3, IO16 should be pulled-up and can not be used for other function." (p12 note 3: IO16 is connected to the PSRAM.)

Latch / timing (p14, Table 5 and Fig 4, "Visualization of Timing Parameters for the Strapping Pins"): tSU (setup, rails stable before CHIP_PU high) min 0 ms; tH (hold, "time reserved for the chip to read the strapping pin values after CHIP_PU is already high") min **1 ms**. Latches: "All strapping pins have latches. At system reset, the latches sample the bit values of their respective strapping pins and store them until the chip is powered down or shut down." (p13-14.) Pins are free to be normal IO after reset.
- **Disagreement:** HDG-32 PDF p16 (printed 13), Table 7, gives tH minimum **3 ms** for the same ESP32 parameter, versus 1 ms in DS-32E. See section 11.
- HDG-32 p17 (printed 14): "It is recommended to place a pull-up resistor at the GPIO0 pin." "Do not add high-value capacitors at GPIO0, or the chip may enter download mode." HDG-32 p16: "GPIO0, GPIO2, GPIO5, MTDI, and MTDO are strapping pins."

### ESP32-C3-WROOM-02 / 02U (DS-C3 PDF p12-15, section 4 "Boot Configurations")
Strapping pins: GPIO2, GPIO8, GPIO9 (p12; p2 "3 strapping GPIOs"). Module pins (Table 3-1, p11): GPIO2 = IO2 pin 16; GPIO8 = IO8 pin 7; GPIO9 = IO9 pin 8.

Default at reset (p12, Table 4-1): GPIO2 "Floating" (no bit value); GPIO8 "Floating" (no bit value); GPIO9 "Weak pull-up" = 1.

Boot mode (p13, Table 4-3; bold = default, read from render):
- **SPI boot mode: GPIO2 = 1, GPIO8 = Any value, GPIO9 = 1** (default row; GPIO9 is the bold bit).
- **Joint download boot mode: GPIO2 = 1, GPIO8 = 1, GPIO9 = 0.** Methods: USB-Serial-JTAG Download Boot and UART Download Boot.
- Footnote 2 (verbatim): "GPIO2 actually does not determine SPI Boot and Joint Download Boot mode, but it is recommended to pull this pin up due to glitches."
- GPIO8 is the ROM-print strap (section 4.2, Table 4-4, below). So GPIO8 must be 1 to enter download, and it is floating by default: it needs a defined level (external pull-up). HDG-C3 Table 3 shows the same "Default Config: -(Floating) -(Floating) 1 (Pull-up)".

Voltage-setting strap: none. The C3 module's flash is a 3.3 V part on VDD_SPI: the module schematic (DS-C3 p31, Fig 8-1) labels the flash "FLASH-3V3". The text does not state a strap for it.

Latch / timing (p13, Table 4-2 and Fig 4-1): tSU min 0 ms; tH min **3 ms** ("time reserved for the chip to read the strapping pin values after CHIP_EN is already high"). "All strapping pins have latches. At Chip Reset, the latches sample the bit values ... until the chip is powered down or shut down."
- HDG-C3 PDF p15 (printed 12): same tH = 3 ms (no disagreement for C3). "It is recommended to place a pull-up resistor at the GPIO9 pin." "Do not add high-value capacitors at GPIO9, or the chip may enter download mode."

---------------------------------------------------------------------

## 5. UART

### ESP32-WROOM-32E / 32UE
- U0TXD = module pin 35 "TXD0" = GPIO1 (Table 3, p12: `TXD0 35 I/O GPIO1, U0TXD, CLK_OUT3, EMAC_RXD2`).
- U0RXD = module pin 34 "RXD0" = GPIO3 (`RXD0 34 I/O GPIO3, U0RXD, CLK_OUT2`).
- Levels, p28-29, Table 15 "DC Characteristics (3.3 V, 25 °C)": VIH min 0.75 x VDD1, max VDD1 + 0.3 V; VIL min -0.3 V, max 0.25 x VDD1; VOH min 0.8 x VDD1; VOL max 0.1 x VDD1. VDD1 = "the I/O voltage for a particular power domain of pins" (footnote 1, p29; per-pin domain is in the ESP32 Series Datasheet Appendix IO MUX, not in this document). IOH 40 mA typ (VDD1 = 3.3 V, VOH >= 2.64 V, max drive strength, VDD3P3_CPU/RTC domain); IOL 28 mA typ (VOL = 0.495 V). IIH/IIL max 50 nA.
- 5 V tolerant: **not stated**. The only upper input bound given is VIH max = VDD1 + 0.3 V (so about 3.6 V + 0.3 at VDD1 = 3.3 V). That bound is not a 5 V tolerance statement.
- ROM boot messages: DS-32E p16, Table 7: "U0TXD Printing Control: Enabled = MTDO 1 (default); Disabled = MTDO 0", "During booting, the strapping pin MTDO can be used to control the U0TXD Printing". **Baud rate of ROM messages: not stated** in DS-32E or HDG-32.
- DS-32E p39: "UART0 is used to download firmware and log output." The UART pins can be re-assigned to any GPIO by the GPIO matrix (p19, 5.2.3). Up to 5 MBaud.
- HDG-32 p16 (printed 13): "U0TXD and U0RXD are GPIO1 and GPIO3 by default." "It is recommended to connect a 499 Ω series resistor to the U0TXD line to suppress harmonics." "For application communication, use UART interfaces other than UART0 if possible."

### ESP32-C3-WROOM-02 / 02U
- U0RXD = module pin 11 "RXD" = GPIO20 (Table 3-1, p11: `RXD 11 I/O/T GPIO20, U0RXD`).
- U0TXD = module pin 12 "TXD" = GPIO21 (`TXD 12 I/O/T GPIO21, U0TXD`).
- Levels, p21-22, Table 6-3: VIH min 0.75 x VDD1, max VDD1 + 0.3 V; VIL min -0.3, max 0.25 x VDD1; VOH 0.8 x VDD1 min; VOL 0.1 x VDD1 max (measured with a high-impedance load, footnote 2); IOH 40 mA typ (VOH >= 2.64 V, PAD_DRIVER = 3); IOL 28 mA typ (VOL = 0.495 V). VDD1 = "voltage from a power pin of a respective power domain".
- 5 V tolerant: **not stated**.
- ROM boot messages: DS-C3 p14, 4.2 "ROM Messages Printing Control": "During the boot process, the messages by the ROM code can be printed to: (Default) UART0 and USB Serial/JTAG controller / UART0 / USB Serial/JTAG controller." Table 4-4 (read from render): UART0 ROM printing Enabled when EFUSE_UART_PRINT_CONTROL = 0 (GPIO8 ignored), or = 1 with GPIO8 = 0, or = 2 with GPIO8 = 1; Disabled when = 1 with GPIO8 = 1, or = 2 with GPIO8 = 0, or = 3 (GPIO8 ignored). Default eFuse = 0, so printing is enabled and GPIO8 is ignored by default. **Baud rate: not stated.**
- DS-C3 p33: "UART0 is used to download firmware and log output." HDG-C3 p14 (printed 11): same 499 Ohm series resistor advice on U0TXD.

---------------------------------------------------------------------

## 6. Antenna keep-out

### What the module datasheets draw
- **ESP32-WROOM-32E**: DS-32E PDF p43, Fig 13 "ESP32-WROOM-32E Recommended PCB Land Pattern" (read from render). The "Antenna Area" box is **18 mm wide** (matching the 18 mm module width) and **6.19 mm deep** (measured at the module's antenna end). A second dimension, 7.49, runs from the same edge to the level of pad 1/38 (my reading of the drawing: the top of the first pad row). The land pattern shows copper only for the pads (38 x 1.5 x 0.9; 17.5 across the pad rows; overall length 25.5). Nothing is dimensioned beyond the 18 x 6.19 box.
- p40, Fig 10: antenna end dimension 6.19 (module 25.5 +/- 0.15 long, 18 +/- 0.15 wide, antenna section 6.19 of the 25.5).
- Pin layout p10, Fig 3 marks a dotted "Keepout Zone" over the antenna end, note A: "The zone marked with dotted lines is the antenna keepout zone. The pin layout of ESP32-WROOM-32UE is the same as that of ESP32-WROOM-32E, except that ESP32-WROOM-32UE has no keepout zone." It sends the reader to the ESP32 Hardware Design Guidelines.
- **ESP32-WROOM-32UE**: p44, Fig 14: land pattern 19.2 mm overall, **no Antenna Area box**, and p10 says "no keepout zone".
- **ESP32-C3-WROOM-02**: DS-C3 PDF p37, Fig 11-1 (read from render): "Antenna Area" box **18 mm wide x 6 mm deep**; a second dimension 7.1 runs to the first pad row (my reading). Overall land pattern length 20, pad span 17.5.
- DS-C3 p10, Fig 3-1 note A: "The pin diagram is applicable to ESP32-C3-WROOM-02 and ESP32-C3-WROOM-02U, but the latter has no antenna keepout zone." It sends the reader to "ESP32-C3 Hardware Design Guidelines > Section General Principles of PCB Layout for Modules".
- **ESP32-C3-WROOM-02U**: p38, Fig 11-2: land pattern 14.3 mm long, no antenna area, no keepout zone.
- Both module datasheets (DS-32E p45, DS-C3 p38) defer module placement to the HDG and give no keepout dimensions of their own.

### What the HDG dimensions for the base board

**HDG-32 PDF p32 (printed 29), Fig 25 "Keepout Zone for ESP32 Module's Antenna (Antenna feed point on the right)"** (read from a zoomed render; unit mm; red dashed box = "Clearance Area"). The antenna end of the module sits outside the base board edge, in a cut-out in the base board:
- **Min 15**: horizontal. It spans from the left edge of the red clearance area to the left edge of the module (the side of the antenna away from the feed point). The base board must be cut away at least 15 mm beyond the module's side on that side.
- **Max 2**: horizontal. It spans from the module's right edge (the feed-point side) to the right edge of the red clearance area. The clearance on the feed-point side extends at most 2 mm beyond the module edge. (The base board's own right edge sits at this side.)
- **Max 1**: the label sits right of the antenna with a long leader arrow below it. By scale (the 6.19 dimension is about 177 px; the small gap above the antenna is about 30 px = about 1 mm) it dimensions the small vertical gap between the top of the module (antenna end) and the top of the red clearance area. **My reading:** the clearance extends at most about 1 mm past the module's antenna end. The label placement is ambiguous.
- **6.19**: vertical. The depth of the module's antenna section (matches DS-32E 6.19), from the module's antenna end to the base-board edge line where the pad area begins. The red clearance box is that 6.19 mm (plus the 1 mm margin) deep and (15 + 18 + up to 2) mm wide.
- Placement rules (HDG-32 PDF p30, printed 27, 1.4.8): "It is suggested to place the module's on-board PCB antenna outside the base board, and the feed point of the antenna close to the edge of the base board." Fig 23: positions marked with a check are strongly recommended (antenna out past the board edge, at a corner, feed point on the board edge side); the rest are not. "If the antenna cannot extend beyond the board edge, the feed point should still be placed as close to the board edge as possible. Then cut off the base board on both sides of the antenna and below it to minimize the impact of the base board material on the PCB antenna and provide a sufficiently large clearance area for the PCB antenna. Note that the module should not be placed in the center of the board with clearance created by hollowing out on all four sides." "sufficient ground copper and dense ground vias should be placed on the base board near the antenna." Housing: "A clearance of at least 15 mm is recommended in all directions."
- No dimension in the figure for how far the clearance extends in the third direction below the antenna (the copper-free layers are not described: no layer-count statement, e.g. whether inner-layer copper must also be cleared). **Not stated** whether the clearance applies to all layers.

**HDG-C3 PDF p26 (printed 23), Fig 21 "Keepout Zone for ESP32-C3 Module's Antenna (Antenna feed point on the right)"** (read from render; unit mm):
- **Min15** on the left: from the left edge of the red clearance area to the module's left edge.
- **Min15** on the right: from the module's right edge to the right edge of the red clearance area.
- **Min15** above: from the antenna's top edge up to the top of the red clearance area.
- **6**: vertical, the module's antenna section depth (the same 6 as DS-C3's Antenna Area).
- So the C3 figure asks for 15 mm minimum on all three open sides (left, right, top), while the ESP32 figure asks 15 mm on one side only and 2 mm / 1 mm maximum on the others.
- Board-edge text: HDG-C3 PDF p24-25 (printed 21-22), 1.4.7, is word-for-word the same as HDG-32's: antenna "outside the base board", feed point "close to the edge", cut off on both sides and below, "not ... in the center of the board with clearance created by hollowing out on all four sides", "A clearance of at least 15 mm is recommended in all directions."

---------------------------------------------------------------------

## 7. Ground pads

### ESP32-WROOM-32E / 32UE
- GND pins on the edge: pins **1, 15, 38** (Table 3, p11-12: `GND 1 P Ground`, `GND 15`, `GND 38`).
- Exposed thermal pad: **pad 39**, labelled "39 GND" in Fig 3 (p10, pin layout) and "P_GND 39" / "EPAD" in Figs 7-9. Table 3 does **not** list pin 39 (the text says "The module has 38 pins").
- Bottom pad pattern (p43, Fig 13; also p40 Fig 10 bottom view): a **3 x 3 array of 0.9 mm pads, 0.5 mm gaps, overall 3.7 x 3.7 mm**; the array is 7.5 from the pad-row dimension and 10.29 from the bottom (read from render). The land pattern legend shows "Via for thermal pad" (red circles) placed in the gaps between the nine pads (I did not count them; in the render they sit between adjacent pads). Fig 3 (p10) also draws a row of nine unnumbered squares labelled GND along the bottom edge; they are not listed in Table 3 (their function beyond "GND" is not stated).
- Soldering note, p39: "Soldering the EPAD to the ground of the base board is not a must, however, it can optimize thermal performance. If you choose to solder it, please apply the correct amount of soldering paste. Too much soldering paste may increase the gap between the module and the baseboard."
- HDG-32 PDF p23 (printed 20), chip-level text: "The ground pad at the bottom of the chip should be connected to the ground plane through at least nine ground vias." This is written for the chip, not specifically for the module EPAD. Also: "If you need to add a thermal pad EPAD under the chip on the bottom of the module, it is recommended to employ a square grid on the EPAD, cover the gaps with solder paste, and place ground vias in the gaps".

### ESP32-C3-WROOM-02 / 02U
- GND pins: **9 and 19** (Table 3-1, p11: `GND 9,19 P Ground`). Pin 19 is the EPAD (Fig 3-1 p10 "Pin 19 GND"; Fig 9-1 "EPAD 19").
- Bottom pad pattern (p37 Fig 11-1; p34 Fig 10-1; read from render): a **3 x 3 array of 0.7 mm pads with 0.4 mm gaps, overall 2.9 x 2.9 mm**, 6.7 from the bottom pad row and 9.96 from the left (dimension drawn to the array centre). "Via for thermal pad" circles sit in the gaps between pads (not counted).
- Soldering note, p33: same wording as the ESP32 one.
- HDG-C3 PDF p19 (printed 16): same chip-level "at least nine ground vias" and EPAD square-grid text, plus: "For optimal grounding, connect the EPAD to a large external ground area using wide traces or copper planes."

---------------------------------------------------------------------

## 8. Dimensions and pin count

| Module | W x L x H (mm) | Pins | Source |
|---|---|---|---|
| ESP32-WROOM-32E | 18.0 x 25.5 x 3.1 (+/-0.15) | 38 (+ EPAD 39) | DS-32E p3 Table 1; p40 Fig 10; p11 "The module has 38 pins" |
| ESP32-WROOM-32UE | 18.0 x 19.2 x 3.2 (+/-0.15) | 38 (+ EPAD 39) | DS-32E p3 Table 2; p40 Fig 11 |
| ESP32-C3-WROOM-02 | 18.0 x 20.0 x 3.2 (+/-0.15) | 19 (18 edge + EPAD pin 19) | DS-C3 p3 Table 1-1; p34 Fig 10-1; p10 "The module has 19 pins" |
| ESP32-C3-WROOM-02U | 18.0 x 14.3 x 3.2 (+/-0.15) | 19 | DS-C3 p4 Table 1-2; p34 Fig 10-2 |

- Quote: "ESP32-WROOM-32E-N4 4 MB (Quad SPI) — –40 ~ 85 18.0 × 25.5 × 3.1" (DS-32E p3). "ESP32-C3-WROOM-02-N4 ... 18.0 × 20.0 × 3.2" (DS-C3 p3).
- u.FL variant connector: first-generation, compatible with Hirose U.FL, I-PEX MHF I, Amphenol AMC (DS-32E p41, 10.2; DS-C3 p35, 10.2). Test antenna gain: DS-32E p42 "maximum gain does not exceed 2.33 dBi"; DS-C3 p36 2.33 dBi (non-FCC) or 1.57 dBi (FCC). DS-C3 p36 note: "If RF function is enabled, make sure an antenna is connected. Operation without an antenna may result in unstable behavior or potential damage to the RF circuit."
- Flash: 32E 4/8/16 MB (N4/N8/N16, H4/H8, R2 PSRAM options); C3-02 N4/H4/N8 (p3, Table 1-1 and revision history v1.7).
- Temperature: both 3 V ranges -40 ~ 85 C (N) or -40 ~ 105 C (H) variants (DS-32E p2-3; DS-C3 p3-4).
- MSL 3; ESD HBM +/-2000 V, CDM +/-500 V (DS-32E p46; DS-C3 p39).

---------------------------------------------------------------------

## 9. Product status and versions

- **Lifecycle status (mass production / NRND / EOL): not stated** for either module by name. Neither cover page carries an NRND or EOL watermark (cover text is the title block only).
- Datasheet Versioning table (DS-32E p48, DS-C3 p41): "v1.0 and higher | Official release | — | This datasheet is publicly released for products in mass production." NRND/EOL watermarks "will be added to the datasheet title page only when all the product variants covered by this datasheet are" NRND/EOL. Both documents are at or above v1.0 and have no watermark, which implies official release / mass production, but no sentence says so for these modules. This is inference.
- **DS-32E**: Datasheet Version 2.1, revision history 2026-08-05 (p50). v1.0 official release 2020-05-29 (p51).
- **DS-C3**: Datasheet Version 1.7, revision history 2026-03-05 (p44). v1.0 2021-08-20 (p45). Chip revision embedded in the module: v0.4 per Tables 1-1/1-2 (p3-4; footnote 6 mentions v1.1).
- HDG-32 and HDG-C3: "Release master", dated Nov 12, 2026 (PDF p1 of each).

---------------------------------------------------------------------

## 10. Absolute maximum ratings

### ESP32-WROOM-32E / 32UE: DS-32E PDF p28, Table 13 "Absolute Maximum Ratings"
- `VDD33 Power supply voltage -0.3 (min) 3.6 (max) V`.
- `TSTORE Storage temperature -40 / 105 C`.
- I/O pin voltage / current abs max: **not stated**. There is a footnote on the table: "* Please see Appendix IO MUX of ESP32 Series Datasheet for IO's power domain." The nearest I/O bound in this document is VIH max = VDD1 + 0.3 V (Table 15), a DC characteristic, not an absolute maximum.

### ESP32-C3-WROOM-02 / 02U: DS-C3 PDF p21, Table 6-1 "Absolute Maximum Ratings"
- `VDD33 Power supply voltage -0.3 (min) 3.6 (max) V`. No storage temperature row in the extracted table.
- I/O pin voltage / current abs max: **not stated**. VIH max = VDD1 + 0.3 V (Table 6-3) as above.

---------------------------------------------------------------------

## 11. Comparison and discrepancies

**Placement difficulty on a ~100 x 100 mm, 4-layer FC.**
- **Antenna keep-out is bigger for the C3.** HDG-C3 Fig 21 asks for 15 mm minimum on both sides and 15 mm above the antenna (a 15 + 18 + 15 = 48 mm wide, 6 + 15 mm deep clearance), versus HDG-32 Fig 25: 15 mm on one side, up to 2 mm on the feed side and about 1 mm at the end. On my reading the ESP32 figure is the smaller clearance to provide at a board corner or edge. (HDG-32's "15 mm all directions" end-product housing advice applies to both.)
- **C3 strapping needs more care**: GPIO2 and GPIO8 are "Floating" by default (no internal pull), and GPIO8 must be 1 to enter download mode, so the STM32 or a pull-up has to define them. ESP32's five straps all have internal pulls; the one hazard is that MTDI/IO12 must stay low at power-up (DS-32E p39), and GPIO0 is the pin to drive low to enter download.
- **TX peak current**: the ESP32 module peaks higher in 802.11b (379 mA at 19.5 dBm vs the C3's 345 mA at 20.5 dBm). In the OFDM rows the C3 peaks higher (285/280/280 mA vs 276/258/211 mA), at higher output power. Both datasheets require a 0.5 A supply, and HDG says to add 10 uF at the VDD3P3 rail and at the power entrance.
- **Module size**: WROOM-32E is 18.0 x 25.5 mm (6.19 mm antenna end), WROOM-02 is 18.0 x 20.0 mm (6 mm antenna end). The u.FL variants have no on-board antenna and no keep-out (32UE 18.0 x 19.2; 02U 18.0 x 14.3), which is far easier to place but needs an external antenna.
- **Pin 39 / pin 19 EPAD** is a 3x3 pad array on both; its thermal vias and solder paste volume matter for the module sitting flat (both datasheets warn that too much paste raises the module off the board).
- **ESP32 pin count (38 vs 19)** and its larger GPIO set (not needed for a UART link) is not a placement issue.

**Disagreements and gaps between the documents.**
1. **Strapping hold time tH**: DS-32E p14 Table 5 says min **1 ms**; HDG-32 PDF p16 (printed 13) Table 7 says min **3 ms** for ESP32. The C3 documents agree (3 ms, DS-C3 p13 and HDG-C3 p15). The conservative value for ESP32 is 3 ms, but the module datasheet says 1 ms.
2. **VIL_nRST**: DS-32E p29 gives max 0.6 V, and HDG-32 p10 gives "(NA ~ 0.6) V" (consistent). For the C3, DS-C3 and HDG-C3 both give -0.3 to 0.25 x VDD. The module datasheets and HDGs agree on these, but the two chips differ in kind (0.6 V absolute vs 0.25 x VDD, about 0.83 V at 3.3 V).
3. **Keep-out**: the module datasheets give only the antenna area (18 x 6.19 for the 32E, 18 x 6 for the C3-02) and refer to the HDG. The HDG figures carry the board-level dimensions. The ESP32 figure's "Max 1" label placement is ambiguous (my reading above), and neither figure states which copper layers are cleared or whether the 15 mm "Min" is measured from the antenna element or the module edge (the drawings measure from the module edge).
4. **Peak vs average**: DS-C3 gives only peak values; DS-32E gives both. Neither defines "peak" (window, bandwidth) and neither gives the burst duration or the supply rise time to size bulk capacitance.
5. **Not stated in any of the four documents**: internal pull on EN/CHIP_PU/CHIP_EN; 5 V tolerance; ROM boot-message baud rate; I/O absolute-maximum voltage or current; explicit product lifecycle status; the flash supply voltage of the 32E module in words (only the IO12-low requirement and Table 17's two ranges); a figure for the recommended RC delay's resulting time constant versus the 50 us tSTBL (R = 10 kOhm, C = 1 uF gives a 10 ms time constant by arithmetic, but no document quotes it).
6. DS-32E Table 3 lists 38 pins and omits pin 39 (EPAD), while Fig 3 and Fig 9 show it; DS-C3 Table 3-1 does list the EPAD as pin 19.
7. HDG-32 Table 6 has an explicit "Default Config" row (GPIO0 = 1, GPIO2 = 0) that DS-32E Table 6 only marks by bold. No conflict in values.
8. HDG nine-ground-via guidance and "ground pad at the bottom of the chip" is written for the bare chip; the module's own EPAD via count is only shown as drawing symbols in the land pattern.
