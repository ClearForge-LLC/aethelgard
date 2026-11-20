# AET-WO-4241 §1.1 — predictions, batch 3 (written before C10 ran, after C1–C4's results)

C1's measured `TXIS` wait was 360,095 instructions (3.6 ms). That is above batch 1's 100,000–300,000.

C10 is `d1`'s sibling `d3-24-i2c-12-spi.json` (16 TMP117 on I2C, 12 W25Q64 on 4 SPI buses), run with the lever
`W25Q64JVxIQ`, so all 12 SPI devices are unmodelled.

1. Every unmodelled SPI read completes with status `ok` and value 0. No SPI read times out, so the SPI reads add
   no wait beyond their transfers.
2. The 16 `TXIS` waits put the boot word at about 16 × 3.6 = 58 ms, inside the 100 ms run. The verdict is
   `pass` under today's judge, which judges the 8 BME280s; a shifted record may fail some of them (C4's
   mechanism). I predict the verdict is `fail`, because the BME280s after a TMP117 on each bus shift.
