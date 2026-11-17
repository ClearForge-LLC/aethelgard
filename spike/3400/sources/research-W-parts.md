# AET-WO-3400 research: W1/W3/W6 parts (2026-11-16)

Prices: digikey.sg keyword-search pages via WebFetch, date 2026-11-16. "SGD" = fetch summary said the page states "All prices are in SGD"; "SGD? (unlabeled)" = no label seen in the fetch result. Fetches are summarised by a small model: re-verify before ordering. Digikey detail URLs given as digikey.com paths as the page linked them (page-relative); the price source is the .sg search URL.
q10 "n/l" = page showed no qty-10 break (single price only); not estimated.

## W1 probe header

| Part | Maker PN | Maker lifecycle (URL) | Package | DK.sg q1 | DK.sg q10 | Price source URL |
|---|---|---|---|---|---|---|
| 8-bit dir. translator | SN74LXC8T245PWR | ACTIVE, https://www.ti.com/product/SN74LXC8T245 | 24-TSSOP (PW); also RHL VQFN, DGS VSSOP | SGD 2.41 | SGD 1.772 | https://www.digikey.sg/en/products/result?keywords=SN74LXC8T245PWR |
| 8-bit dir. translator | SN74LVC8T245PWR | ACTIVE, https://www.ti.com/product/SN74LVC8T245 | 24-TSSOP (PW) (package from TI family, not from DK) | SGD? (unlabeled) 2.37 | n/l | https://www.digikey.sg/en/products/result?keywords=SN74LVC8T245PWR (detail: https://www.digikey.com/en/products/detail/texas-instruments/SN74LVC8T245PWR/857773) |
| 8-bit dir. translator | SN74AVC8T245PWR | ACTIVE, https://www.ti.com/product/SN74AVC8T245 | 24-TSSOP (PW), also TVSOP, VQFN | SGD? (unlabeled) 2.52 | n/l | https://www.digikey.sg/en/products/result?keywords=SN74AVC8T245PWR |
| auto-dir. translator | TXS0108EPWR | ACTIVE, https://www.ti.com/product/TXS0108E | 20-TSSOP | SGD? (unlabeled) 2.83 | n/l | https://www.digikey.sg/en/products/result?keywords=TXS0108EPWR |
| 8-ch ESD array | TPD8E003DQDR | ACTIVE, https://www.ti.com/product/TPD8E003 | 8-WSON (DQD) 1.7x1.35 mm | SGD 2.47 (cut tape; Digi-Reel 2.33) | SGD 1.565 (cut tape; Digi-Reel 1.477) | https://www.digikey.sg/en/products/result?keywords=TPD8E003DQDR |
| 2x10 header | Wurth 61302021121 (20-pos, 2.54 mm, vertical TH; "WR-PHD", bag) | DK status Active; maker page not fetched ("distributor only") | THT 2x10 | SGD? (unlabeled) 1.82 | SGD? (unlabeled) 1.55 | https://www.digikey.sg/en/products/result?keywords=61302021121 |

Voltage ranges (TI pages, quoted):
- SN74LXC8T245: "Fully configurable dual-rail design allows each port to operate from 1.1 V to 5.5 V"; "Up to 420-Mbps support for 3.3 V to 5.0 V"; A-port and DIR/OE reference VCCA. Exists, ACTIVE. NOTE: range is 1.1-5.5 V, wider than the 1.65-5.5 V the WO text assumed. 5 V on the target side is supported (either side up to 5.5 V). DK.sg stock: page said out of stock, 2,000 expected 23-Nov-2026, 26-week lead time (SGD 2.41/1.772 are shown for cut tape).
- SN74LVC8T245: "optimized to operate with VCCA and VCCB set at 1.65V to 5.5V"; up to 200 Mbps. Supports 5 V on both sides.
- SN74AVC8T245: "V CCA accepts any supply voltage from 1.2 V to 3.6 V" (same for VCCB); 5 V NOT supported on either rail; I/O 4.6 V tolerant. Not suitable for 5 V targets.
- Same-footprint alternates seen on the LXC DK page: SN74LXCH8T245PWR SGD? 2.69 (10,518 in stock; bus-hold variant); Nexperia 74LVCH8T245PW,118 SGD? 1.34 (11,971 in stock); Nexperia 74AVC8T245PW-Q100J. Not checked beyond the DK listing.

TXS0108E (TI page): "1.4V to 3.6V on A port and 1.65V to 5.5V on B port (VCCA <= VCCB)"; "110Mbps (push pull)"; "1.2Mbps (open drain)"; auto-direction sensing, edge-rate accelerator. The page lists I2C/SMBus as an application. It does NOT state the poor-fit reasons. The TI app notes (SCEA043/SCEA044) PDFs were fetched but text was compressed/unreadable, so there is no TI quote for "why auto-direction is a poor fit for open-drain I2C / fast push-pull". Only the TI page's own figures above (1.2 Mbps open-drain, 110 Mbps push-pull, A-port max 3.6 V so 5 V cannot sit on the A side) are citable. Design inference (mine, not TI-quoted): the architecture senses direction from the pin levels, so a bus with the target driving and the instrument driving ambiguously or with heavy pull-ups/slow edges can mis-switch; the 5 V target must be on the B side only.

## W3 host link

| Part | Maker PN | Maker lifecycle | Package | DK.sg q1 | DK.sg q10 | Price source URL |
|---|---|---|---|---|---|---|
| USB HS ULPI PHY | USB3320C-EZK-TR | DK status Active; microchip.com returned HTTP 403: "distributor only" | 32-QFN 5x5 (32-VFQFN EP) | SGD 2.98 (cut tape) | SGD 2.98 (page showed breaks at 1 and 25 only, q10 falls in the 1 tier: fetch-model inference) | https://www.digikey.sg/en/products/result?keywords=USB3320C-EZK-TR |
| USB HS ULPI PHY | USB3300-EZK-TR | DK status Active; distributor only | 32-QFN 5x5 | SGD 2.37 (cut tape) | n/l (q25 SGD 2.1192) | https://www.digikey.sg/en/products/result?keywords=USB3300-EZK-TR |
| Ethernet RMII PHY | LAN8742A-CZ-TR | DK status Active; distributor only | 24-SQFN 4x4 (24-VFQFN EP) | SGD 2.10 (cut tape) | n/l (q25 SGD 1.826) | https://www.digikey.sg/en/products/result?keywords=LAN8742A-CZ-TR |
| MagJack RJ45 | Wurth 7499010211A (1 port RJ45 TH 10/100 Base-T, AutoMDIX) | DK status Active; distributor only | THT | SGD 11.96 | SGD 11.761 | https://www.digikey.sg/en/products/result?keywords=7499010211A |
| USB-C receptacle | GCT USB4105-GF-A (USB2.0 Type-C 16P SMD RA) | not fetched; distributor only | SMD right-angle | SGD? (unlabeled) 1.10 | n/l | https://www.digikey.sg/en/products/result?keywords=USB4105-GF-A (detail: https://www.digikey.com/en/products/detail/gct/USB4105-GF-A/11198441) |

Cheap cross-checks from the LAN8742A DK page (SGD, no q10): Pulse H1102NL 16-SOIC magnetics SGD 7.45; NDK STDCZS1-25M 25 MHz crystal SGD 0.71; STM32H753ZIT6 SGD 29.15 (144-LQFP).
Clock/crystal needs: NOT SOURCED (microchip.com product and datasheet URLs returned HTTP 403). The only hint is DK related-parts lists showing an Abracon ABM12W-25.0000MHZ crystal next to USB3320 and an NDK 25 MHz crystal next to LAN8742A, which is not a quote of a requirement. Datasheet recall (unsourced, verify): USB3320 takes 24 MHz crystal or 13/19.2/24/26 MHz ref clock and outputs 60 MHz CLKOUT; LAN8742A takes 25 MHz crystal and can output 50 MHz REFCLKO for RMII.

## W6 host protocol (sigrok)

| Item | Finding | URL |
|---|---|---|
| OLS driver page | Wiki says source dir `openbench-logic-sniffer`; driver `ols` (named on SUMP-compatibles page). 32 channels; infobox 200 MHz but "up to 100MHz signals", driver does not support 200 MHz demux; 24 ksamples; PIC18F24J50 "presents a standard USB ACM profile" (host sees serial port; 8N1). Protocol from original SUMP docs; hw 0.6 = protocol v0, 0.7 = v1. | https://sigrok.org/wiki/Openbench_Logic_Sniffer |
| Command set (same page) | Short 1-byte: Reset 0x00, Run 0x01, ID 0x02, XON 0x11, XOFF 0x13. Long 5-byte: Set Trigger Mask 0xc0/c4/c8/cc, Trigger Values 0xc1/c5/c9/cd, Trigger Config 0xc2/c6/ca/ce, Set Divider 0x80, Set Read & Delay Count 0x81, Set Flags 0x82. Driver: no serial trigger mode; parallel mode up to 4 stages; RLE broken. The "1ALS" ID reply string was NOT shown in the fetched text. | same |
| SUMP compatibles page | Lists OLS, Saanlima Pipistrello OLS, Buspirate, USB IR Toy, Logic Shrimp, AGLA, JTAGulator. "Up to 32 channels are supported." "sample rates in the 60MHz range were seen." AGLA 5-6 ch, <=4 MHz, 532-7168 samples; JTAGulator 24 ch, 1.2 MSa/s, 1024 Sa. No command codes (refers to sump.org). | https://sigrok.org/wiki/SUMP_compatibles |
| Arduino (AGLA) | Sketch implements SUMP for the `ols` driver; "extended SUMP protocol"; 6 ch, 4 MHz; PulseView does not show device-reported max rate; trigger only well below ~1 MHz; status in progress. | https://sigrok.org/wiki/Arduino |
| Logic Shrimp | PIC18F24J50, 4 ch, 20 MHz, extended SUMP via OLS driver; firmware does not publish metadata correctly. | https://sigrok.org/wiki/Logic_Shrimp |
| MCU logic analyzer over USB CDC | sigrok-pico (GitHub, not on the sigrok wiki; searches of sigrok wiki returned nothing): driver `raspberrypi_pico`, Pico SDK CDC serial; 21 digital + 3 analog channels; "Merged to mainline sigrok" Sept 2023; PulseView 0.4.2 / sigrok-cli 0.7.2 do not include it, need newer build. | https://github.com/pico-coder/sigrok-pico |
| sigrok-pico protocol | ASCII: `*` reset, `+` abort, `i` identify -> `SRPICO,AxxDyy,00`, `aX` analog scale, `R<rate>`, `L<count>`, `A<en><ch>`, `D<en><ch>` (ACK `*`), `F` fixed, `C` continuous, `!` overflow abort. Sample-rate limits not listed. | https://github.com/pico-coder/sigrok-pico/blob/main/TECHNICAL.md |

Recommendation: speak SUMP (extended, OLS-compatible) over USB CDC. It is the only microcontroller-analyzer protocol documented on sigrok.org itself (OLS, SUMP-compatibles and Arduino pages), the `ols` driver ships in every PulseView/sigrok-cli release including 0.4.2/0.7.2 (the Arduino page and OLS page confirm in-tree support), and the command set is small and fully listed above. Caveats: ols limits are 32 channels (our 16 fit), 5-byte long commands, trigger parallel mode only with 4 stages, and metadata (`0x04` etc.) must be published correctly or PulseView misreads rate/memory (Logic Shrimp page). The sigrok-pico protocol is merged upstream but only in releases newer than 0.4.2 / 0.7.2 and is Pico-specific (`SRPICO` ID), so it is a poor fit for day-one on stock PulseView. Verify the "1ALS" ID reply and the metadata token set against sump.org / libsigrok protocol.h (not fetched).

## Could not source
- microchip.com lifecycle (USB3320, USB3300, LAN8742A): HTTP 403 on product and datasheet URLs; "distributor only" (DK Active).
- Crystal/clock requirements quotes (USB3320, LAN8742A): datasheets 403.
- TI quotes on why auto-direction translators are poor for open-drain I2C / fast push-pull: app notes SCEA043/SCEA044 unreadable (compressed PDF); only TI-page speed figures available.
- q10 for SN74LVC8T245PWR, SN74AVC8T245PWR, TXS0108EPWR, USB4105-GF-A, USB3300, LAN8742A-CZ-TR, and the 61302021121 currency label: search page did not show it. LVC8T245/AVC8T245 package taken from TI family page (not a DK field).
- Wurth, GCT maker lifecycle pages: not fetched.
- "1ALS" and metadata keys on sigrok wiki; sump.org protocol page.
- Sample-rate fields for sigrok-pico.
