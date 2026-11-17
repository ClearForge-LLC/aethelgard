# digikey.sg reads by the builder, 2026-11-16 (WebFetch; quoted as the fetch tool returned them)

| Part | URL | q1 | q10 | Currency on page | Status |
|---|---|---|---|---|---|
| TMP117AIDRVR (re-read of research B) | https://www.digikey.sg/en/products/detail/texas-instruments/TMP117AIDRVR/9685284 | 8.63000 | 7.44200 | "All prices are in SGD" | Active |
| INA226AIDGSR | https://www.digikey.sg/en/products/result?keywords=INA226AIDGSR | 4.38000 | 3.27300 | "All prices are in SGD" | Active |
| TCAN334DCNR | https://www.digikey.sg/en/products/result?keywords=TCAN334DCNR | 3.90000 | 2.90500 | "All prices are in SGD" | Active (backorder) |
| USBLC6-2SC6 (ST listing) | https://www.digikey.sg/en/products/result?keywords=USBLC6-2SC6 | 0.74000 | not shown | "$", region Singapore/SGD | not shown |
| USB4105-GF-A | https://www.digikey.sg/en/products/result?keywords=USB4105-GF-A | 1.10000 | not shown | "$", region Singapore/SGD | not shown |
| PTS810SJM250SMTRLFS | https://www.digikey.sg/en/products/result?keywords=PTS810%20SJM%20250%20SMTR%20LFS | 0.80000 | not listed | region Singapore/SGD | Active |
| Keystone 5000 | https://www.digikey.sg/en/products/result?keywords=Keystone%205000 | not listed | | | |

Also re-read: https://oshpark.com/ ("3 copies for $5/square inch", 1.6 mm, ENIG) and https://www.ti.com/product/TCAN334 (ACTIVE; "3.3V Single supply operation"; TCAN334 up to 1 Mbps; SOIC (D), SOT-23 (DCN)).
AN2606 (https://www.st.com/resource/en/application_note/an2606-stm32-microcontroller-system-memory-boot-mode-stmicroelectronics.pdf): two downloads and one fetch timed out; not read.

## Amendment spot-checks (builder, 2026-11-16 ~23:10 UTC)
- https://docs.oshpark.com/services/ : 2-layer Prototype "$5 per square inch, per set of 3"; 4-layer Prototype "$10 per square inch, per set of 3".
- https://www.digikey.sg/en/products/result?keywords=SN74LXC8T245PWR : 2.41000 / 1.77200, "All prices are in SGD", Active.
- https://sigrok.org/wiki/Openbench_Logic_Sniffer : 32 probes; infobox 200MHz, text "up to 100MHz signals" (the page is inconsistent); USB ACM; short commands reset 0x00, run 0x01, ID 0x02, XON 0x11, XOFF 0x13; long 0xc0-0xce trigger, 0x80 divider, 0x81 read & delay count, 0x82 flags.
- RM0433 Rev 9 text: L20852 GPIO registers "accessed as 32-bit words, half-words or bytes"; chapter 47 (I2C, L76288-78755) contains no "monitor", "listen", "sniff" or "promiscuous".

## Second round of checks (builder, after the advisor's review)
- Pin XML re-read with a real XML parser (`xml.etree`). The earlier regex read past self-closing `<Pin/>` elements: it put ULPI_STP on NRST and D5 on VCAP. With the parser, all twelve ULPI signals are as research-W-silicon says (STP PC0, D5 PB12). ETH and USB FS are as stated; FDCAN1 also has PB8/PB9. There are 114 I/O pins, not the 100 the regex counted.
- DS12117 Rev 10 text, L37300-37320 (Table 194, rev V): master 80/100 (SPI1-3), 50 (SPI4-6); slave receiver 100; slave transmitter/full duplex 31 (2.7-3.6 V), 29 (1.62-3.6 V) MHz. As adopted.
- ES0392 Rev 16 text L284 and L1395: "2.2.27 Ethernet MII mode is not available on packages with PC2_C/PC3_C pins", A A A. As adopted.
- digikey.sg STM32H753ZIT6: "$29.15000", one price, region Singapore/SGD, no status shown.
- Second fabs: pcbgogo.com/pcb-price.html 404; eurocircuits.com/pcb-price-calculator/ 404; seeedstudio.com/fusion_pcb.html, calculator placeholders only, "will come back to you with a price in 2 working days".
- W2 rate: WebSearch "STM32H7 logic analyzer DMA GPIO IDR timer sample rate MHz SUMP firmware" (standard, 2026-11-16) found no cited H7 timer-DMA capture rate.
