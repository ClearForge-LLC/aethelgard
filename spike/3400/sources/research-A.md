# AET-WO-3400 research-A (2026-11-16)

Method note: only WebFetch (markdown conversion) was available for distributor pages. LCSC and JLCPCB pages fetched but render prices by JavaScript: no price table came back. DigiKey returned HTTP 403; Mouser timed out. No account created, no estimates made. Every price cell is therefore "unknown". Lifecycle and package cells are from the maker page where stated; anything not read on a page is marked [inferred].

Price cell key: "unknown (blocked: URL)" = page fetched or attempted, no price obtained.

## T1 I2C pull-ups
| Candidate | Part | Maker | Lifecycle (maker, URL) | Price q1 / q10 | Package, area mm2 |
|---|---|---|---|---|---|
| 1k/2.2k/4.7k/10k pull-up | CRCW0603 series (e.g. CRCW06034K70FKEA) [MPN pattern inferred, not read] | Vishay | Not stated on family page https://www.vishay.com/en/product/20035/ (page says "D/CRCW e3"; status not shown) | unknown (no price on https://www.vishay.com/en/product/20035/) | 0603 ~1.6 x 0.8 mm body = 1.3 mm2 [inferred]; land pattern ~2.6 x 1.0 = 2.6 mm2 [inferred] |
| 3-pad selector jumper | no part: footprint only | | | | ~3 x 1 mm [inferred] |

## T2 bus-capacitance coupon
| Candidate | Part | Maker | Lifecycle | Price | Package |
|---|---|---|---|---|---|
| 47/100/220 pF C0G | GRM1885C1H series (e.g. GRM1885C1H101JA01D) [MPN not read] | Murata | not read (no page fetched) | unknown (no page read) | 0603, as T1 [inferred] |
| solder jumper | no part: footprint only | | | | |

## T3 current sense
| Candidate | Part | Maker | Lifecycle | Price | Package |
|---|---|---|---|---|---|
| 0.1 ohm 1% shunt | WSL / Yageo PT series, exact MPN not selected | Vishay / Yageo | not read | unknown (not fetched) | 0805 ~2.0 x 1.25 body = 2.5 mm2; 1206 ~3.2 x 1.6 = 5.1 mm2 [inferred] |
| I2C monitor | INA226AIDGSR (LCSC C49851, JLCPCB C49851, "Extended part") | Texas Instruments | ACTIVE, https://www.ti.com/product/INA226 | unknown (blocked: https://www.lcsc.com/product-detail/C49851.html and https://jlcpcb.com/partdetail/C49851, no prices rendered; DigiKey 403; Mouser timeout) | VSSOP-10 (DGS), 3 x 4.9 mm = 14.7 mm2 (TI page). Supply 2.7-5.5 V (JLCPCB page) |
| I2C monitor | INA219 (suffix not chosen) | Texas Instruments | ACTIVE, https://www.ti.com/product/INA219 | unknown (not fetched) | SOIC-8 (D) 29.4 mm2; SOT-23-8 (DCN) 4.64 mm2 (TI page) |

## T4 test points
| Candidate | Part | Maker | Lifecycle | Price | Package |
|---|---|---|---|---|---|
| TH test loop | Keystone 5000 series [MPN not verified] | Keystone Electronics | not read (page not fetched) | unknown | through-hole; area not sourced |
| bare SMD pad | no part: footprint only | | | | |

## T5 / T6 headers, shunt, solder jumpers
| Candidate | Part | Maker | Lifecycle | Price | Package |
|---|---|---|---|---|---|
| 1x2, 1x3 2.54 mm header | not selected (e.g. Wurth 61300211121 / 61300311121) [MPN from memory, not read] | Wurth | not read | unknown | 2.54 mm pitch: 1x2 ~5.1 x 2.5 = 12.7 mm2, 1x3 ~7.6 x 2.5 = 19 mm2 [inferred] |
| jumper shunt | not selected | | not read | unknown | |
| solder jumper | no part: footprint only | | | | |

## T8 programming
| Candidate | Part | Maker | Lifecycle | Price | Package |
|---|---|---|---|---|---|
| reset switch | C&K PTS810 / Panasonic EVQ-P7, MPN not selected | | not read | unknown | ~6 x 6 mm = 36 mm2 [inferred] |
| BOOT0 jumper | header as T5 | | | | |
| Tag-Connect TC2030-IDC-NL | footprint only | Tag-Connect | | | pad pattern only; cable is a tool |

## F5 USB
| Candidate | Part | Maker | Lifecycle | Price | Package |
|---|---|---|---|---|---|
| USB-C 2.0 receptacle | GCT USB4105-GF-A | GCT | unknown (blocked: https://gct.co/connector/usb4105, HTTP 403) | unknown | not sourced |
| (alt seen at LCSC) | TYPE-C 16PIN 2MD(073), LCSC C2765186 | Shou Han | distributor only | unknown (no price on https://www.lcsc.com/product-detail/C2765186.html; MOQ 20 on cut tape) | not sourced |
| ESD array | USBLC6-2SC6 | ST | unknown (blocked: https://www.st.com/en/protection-devices/usblc6-2sc6.html, timeout) | unknown | SOT-23-6 ~2.9 x 1.6 body [inferred] |
| 5.1k CC x2 | CRCW0603 5.1k | Vishay | as T1 | unknown | 0603 |

## J2 DroneCAN node
| Candidate | Part | Maker | Lifecycle | Price | Package |
|---|---|---|---|---|---|
| CAN transceiver, 3.3 V | TCAN334 | TI | ACTIVE, https://www.ti.com/product/TCAN334 | unknown (not fetched) | SOT-23-8 (DCN) 2.9 x 1.6 = 4.64 mm2; SOIC-8 29.4 mm2. "3.3V single supply"; 1 Mbps classic CAN (5 Mbps only for TCAN33xG variants) |
| CAN FD transceiver | TCAN1462-Q1 | TI | Active, https://www.ti.com/product/TCAN1462-Q1 (page lists TCAN1472-Q1/1476-Q1/1463-Q1 as newer alternates) | unknown | VSON-8 (DRB) 9 mm2; SOIC-8 29.4 mm2. VCC 4.5-5.5 V: needs 5 V. VIO 1.7-5.5 V |
| CAN transceiver | TJA1051T/3 | NXP | Active (family), https://www.nxp.com/products/interfaces/can-transceivers/can-with-flexible-data-rate/high-speed-can-transceiver:TJA1051 | unknown | SO8; VIO 3-5 V per page. VCC 4.5-5.5 V is [inferred], 5 V part |
| 120 ohm term | CRCW0603 120R | Vishay | as T1 | unknown | 0603 |
| connector | JST SM04B-GHS-TB | JST | not read | unknown | not sourced |

Finding: of the transceivers, only TCAN334 runs from 3.3 V alone (page: "3.3V Single supply operation"). It is not CAN FD. FD parts need a 5 V VCC (TCAN1462-Q1) [TJA1051 5 V inferred].

## AP_Periph H743 targets at Copter-4.7.2 (listing via GitHub API and raw files, tag Copter-4.7.2)
Yes. Directories under https://github.com/ArduPilot/ardupilot/tree/Copter-4.7.2/libraries/AP_HAL_ChibiOS/hwdef/
- CubeOrange-periph: `include ../CubeOrange/hwdef.dat` and `env AP_PERIPH 1`; CubeOrange hwdef reads `MCU STM32H7xx STM32H743xx`.
- MatekH743-periph: `include ../MatekH743/hwdef.dat`, `env AP_PERIPH 1`; MatekH743 reads `MCU STM32H7xx STM32H743xx`.
- CubeOrange-periph-heavy (same family, hwdef not read).
Counter-examples: ARK_CANNODE is STM32F412 (not H7); Here4AP is STM32H757xx. No H753-specific periph target seen. H743 build on an H753 is a prediction (H753 = H743 plus crypto), not a fact.

## J3 firmware support (maker docs)
- Klipper: https://github.com/Klipper3d/klipper/blob/master/src/stm32/Kconfig has `config MACH_STM32H723` / `config MACH_STM32H743` (selectable processors). docs/Benchmarks.md lists the row `| STM32H723 | 7429K | 8619K |`. docs/Features.md has no stm32 text.
- grblHAL: https://github.com/grblHAL/Drivers lists `| Board | Teensy 4.x | Nucleo-F756ZG | Nucleo-H743ZI |` and `| Board | BTT SKR3 / WeAct MiniSTM32H743 | ...`. No H723 mention.
- Zephyr: https://github.com/zephyrproject-rtos/zephyr/blob/main/boards/st/nucleo_h743zi/doc/index.rst (`.. zephyr:board:: nucleo_h743zi`) and .../nucleo_h723zg/doc/index.rst (`.. zephyr:board:: nucleo_h723zg`) exist in main. Armfly STM32H743XIH6 also appears at https://docs.zephyrproject.org/latest/boards/index.html.
Whether any H743 build runs on the H753ZI is a prediction, not a fact.

## Could not source
- All prices (q1 and q10), every row: LCSC/JLCPCB render prices with JavaScript (WebFetch got none); DigiKey 403; Mouser timeout. I did not spoof a browser or call undocumented APIs (a curl attempt was refused by the sandbox classifier).
- Lifecycle not obtained: Vishay CRCW0603 (page silent), Murata GRM1885C1H, WSL/PT, Keystone 5000, headers, shunt, PTS810/EVQ-P7, USBLC6-2SC6 (ST timeout), GCT USB4105 (403), JST GH, Tag-Connect.
- Package areas not read from drawings: all passives, headers, switch, USB-C, USBLC6, JST (values marked [inferred] are typical, not sourced).
- CAD licence concerns: none seen, none checked.
