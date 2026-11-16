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
