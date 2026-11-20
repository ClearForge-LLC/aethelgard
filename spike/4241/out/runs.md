# AET-WO-4241 §1.1 runs

## C0/old

- generate rc 2, build rc , simulate rc ; elf None, main.rs None
- generate: `{"ok":false,"error":{"kind":"UnsupportedSchemaVersion","id":null,"message":"schema_version is 16, not 19"}}`
- boot log lines 0; `Unknown slave`: []
- BOOTWORD: []
- STEP hooks: []

## C0/new

- generate rc 2, build rc , simulate rc ; elf None, main.rs None
- generate: `{"ok":false,"error":{"kind":"UnsupportedSchemaVersion","id":null,"message":"schema_version is 16, not 19"}}`
- boot log lines 0; `Unknown slave`: []
- BOOTWORD: []
- STEP hooks: []

**C0: ELF old == new: True; main.rs equal: True**

## C1/old

- generate rc 0, build rc 0, simulate rc 0; elf 2bb1be79503255b9, main.rs 9440291cc9023b07
- reads in main.rs (2): i2c1 0x76 [0xd0]; spi1 Spi1Device::U4 [0x9f]
- verdict **pass**; findings: [('device_unmodelled', 'info')]
  - `device_unmodelled`: U3 (TMP117) at I2C address 0x48 has no Renode 1.17.0 model, so its identity was not read
- `absent` fields in the verdict: [[]]
- boot log lines 108; `Unknown slave`: []
- multiplexer unrouted (1): ["[00:00:00.000000000] [WARNING] Tried to finish transmission to device 0x0, but it's not connected"]
- BOOTWORD: ['00:00:00.000021060] BOOTWORD 2100b007 0 2662']
- STEP hooks: ['00:00:00.000000000] STEP 0 22020000 2102']
- RAM from the boot word: +0x0=0x2100b007 +0x4=0x0406aafc +0x8=0x22020000(ok) +0xc=0x00000060 +0x10=0x22020000(ok) +0x14=0x00ef4017 +0x18=0x00000001

## C1/new

- generate rc 0, build rc 0, simulate rc 1; elf 406af07792828dc0, main.rs 0dda50a50dfca746
- reads in main.rs (3): i2c1 0x76 [0xd0]; i2c1 0x48 [0x0f]; spi1 Spi1Device::U4 [0x9f]
- verdict **fail**; findings: [('spi_record_invalid', 'error'), ('device_unmodelled', 'info')]
  - `spi_record_invalid`: U4 on SPI1, chip select PA0: the record reads status 0x22020001 and value 0x00000000, which no completed or timed-out transfer writes
  - `device_unmodelled`: U3 (TMP117) at I2C address 0x48 has no Renode 1.17.0 model, so its identity was not read
- `absent` fields in the verdict: [['0x48']]
- boot log lines 110; `Unknown slave`: ['[00:00:00.000021330] [WARNING] i2c1: Unknown slave at address 72.']
- multiplexer unrouted (1): ["[00:00:00.000000000] [WARNING] Tried to finish transmission to device 0x0, but it's not connected"]
- BOOTWORD: ['00:00:00.003600000] BOOTWORD 2100b007 0 362757']
- STEP hooks: ['00:00:00.000000000] STEP 0 22020000 2127']
- RAM from the boot word: +0x0=0x2100b007 +0x4=0x0406aafc +0x8=0x22020000(ok) +0xc=0x00000060 +0x10=0x22020001(timeout_txis) +0x14=0x00000000 +0x18=0x22020000(ok) +0x1c=0x00ef4017 +0x20=0x00000001

**C1: ELF old == new: False; main.rs equal: False**

## C10/new

- generate rc 0, build rc 0, simulate rc 1; elf 1ba2683a0c303f65, main.rs e06c4c3a920cbf86
- reads in main.rs (36): i2c1 0x76 [0xd0]; i2c1 0x77 [0xd0]; i2c1 0x48 [0x0f]; i2c1 0x49 [0x0f]; i2c1 0x4a [0x0f]; i2c1 0x4b [0x0f]; i2c2 0x76 [0xd0]; i2c2 0x77 [0xd0]; i2c2 0x48 [0x0f]; i2c2 0x49 [0x0f]; i2c2 0x4a [0x0f]; i2c2 0x4b [0x0f]; i2c3 0x76 [0xd0]; i2c3 0x77 [0xd0]; i2c3 0x48 [0x0f]; i2c3 0x49 [0x0f]; i2c3 0x4a [0x0f]; i2c3 0x4b [0x0f]; i2c4 0x76 [0xd0]; i2c4 0x77 [0xd0]; i2c4 0x48 [0x0f]; i2c4 0x49 [0x0f]; i2c4 0x4a [0x0f]; i2c4 0x4b [0x0f]; spi1 Spi1Device::S1000 [0x9f]; spi1 Spi1Device::S1001 [0x9f]; spi1 Spi1Device::S1002 [0x9f]; spi2 Spi2Device::S1003 [0x9f]; spi2 Spi2Device::S1004 [0x9f]; spi2 Spi2Device::S1005 [0x9f]; spi3 Spi3Device::S1006 [0x9f]; spi3 Spi3Device::S1007 [0x9f]; spi3 Spi3Device::S1008 [0x9f]; spi4 Spi4Device::S1009 [0x9f]; spi4 Spi4Device::S1010 [0x9f]; spi4 Spi4Device::S1011 [0x9f]
- verdict **fail**; findings: [('i2c_timeout', 'error'), ('i2c_timeout', 'error'), ('i2c_timeout', 'error'), ('i2c_timeout', 'error'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info')]
  - `i2c_timeout`: S6 at I2C address 0x76: the firmware's wait for TXIS expired reading register 0xd0 (expected 0x60)
  - `i2c_timeout`: S7 at I2C address 0x77: the firmware's wait for TXIS expired reading register 0xd0 (expected 0x60)
  - `i2c_timeout`: S12 at I2C address 0x76: the firmware's wait for TXIS expired reading register 0xd0 (expected 0x60)
  - `i2c_timeout`: S13 at I2C address 0x77: the firmware's wait for TXIS expired reading register 0xd0 (expected 0x60)
  - `device_unmodelled`: S2 (TMP117) at I2C address 0x48 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S3 (TMP117) at I2C address 0x49 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S4 (TMP117) at I2C address 0x4a has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S5 (TMP117) at I2C address 0x4b has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S8 (TMP117) at I2C address 0x48 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S9 (TMP117) at I2C address 0x49 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S10 (TMP117) at I2C address 0x4a has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S11 (TMP117) at I2C address 0x4b has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S14 (TMP117) at I2C address 0x48 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S15 (TMP117) at I2C address 0x49 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S16 (TMP117) at I2C address 0x4a has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S17 (TMP117) at I2C address 0x4b has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S20 (TMP117) at I2C address 0x48 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S21 (TMP117) at I2C address 0x49 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S22 (TMP117) at I2C address 0x4a has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S23 (TMP117) at I2C address 0x4b has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S1000 (W25Q64JVxIQ) on SPI1, chip select PA0 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S1001 (W25Q64JVxIQ) on SPI1, chip select PA1 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S1002 (W25Q64JVxIQ) on SPI1, chip select PA10 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S1003 (W25Q64JVxIQ) on SPI2, chip select PA11 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S1004 (W25Q64JVxIQ) on SPI2, chip select PA15 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S1005 (W25Q64JVxIQ) on SPI2, chip select PA2 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S1006 (W25Q64JVxIQ) on SPI3, chip select PA3 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S1007 (W25Q64JVxIQ) on SPI3, chip select PA4 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S1008 (W25Q64JVxIQ) on SPI3, chip select PA9 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S1009 (W25Q64JVxIQ) on SPI4, chip select PB0 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S1010 (W25Q64JVxIQ) on SPI4, chip select PB1 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S1011 (W25Q64JVxIQ) on SPI4, chip select PB12 has no Renode 1.17.0 model, so its identity was not read
- `absent` fields in the verdict: [['0x48', '0x49', '0x4a', '0x4b'], ['0x48', '0x49', '0x4a', '0x4b'], ['0x48', '0x49', '0x4a', '0x4b'], ['0x48', '0x49', '0x4a', '0x4b'], ['0x48', '0x49', '0x4a', '0x4b'], ['0x48', '0x49', '0x4a', '0x4b'], ['0x48', '0x49', '0x4a', '0x4b'], ['0x48', '0x49', '0x4a', '0x4b']]
- boot log lines 322; `Unknown slave`: ['[00:00:00.000065910] [WARNING] i2c1: Unknown slave at address 72.', '[00:00:00.004866970] [WARNING] i2c1: Unknown slave at address 73.', '[00:00:00.009668050] [WARNING] i2c1: Unknown slave at address 74.', '[00:00:00.014469100] [WARNING] i2c1: Unknown slave at address 75.', '[00:00:00.019275420] [WARNING] i2c2: Unknown slave at address 72.', '[00:00:00.024000000] [WARNING] i2c2: Unknown slave at address 73.', '[00:00:00.028800000] [WARNING] i2c2: Unknown slave at address 74.', '[00:00:00.033600000] [WARNING] i2c2: Unknown slave at address 75.', '[00:00:00.038400000] [WARNING] i2c3: Unknown slave at address 72.', '[00:00:00.043200000] [WARNING] i2c3: Unknown slave at address 73.', '[00:00:00.048000000] [WARNING] i2c3: Unknown slave at address 74.', '[00:00:00.052800000] [WARNING] i2c3: Unknown slave at address 75.', '[00:00:00.057600000] [WARNING] i2c4: Unknown slave at address 72.', '[00:00:00.062400000] [WARNING] i2c4: Unknown slave at address 73.', '[00:00:00.067200000] [WARNING] i2c4: Unknown slave at address 74.', '[00:00:00.072000000] [WARNING] i2c4: Unknown slave at address 75.']
- multiplexer unrouted (168): ["[00:00:00.000000000] [WARNING] Tried to finish transmission to device 0x2, but it's not connected", "[00:00:00.000000000] [WARNING] Tried to finish transmission to device 0x2, but it's not connected", "[00:00:00.000000000] [WARNING] Tried to finish transmission to device 0x1, but it's not connected", "[00:00:00.000000000] [WARNING] Tried to finish transmission to device 0x2, but it's not connected"] …
- BOOTWORD: ['00:00:00.076900000] BOOTWORD 2100b007 0 7699763']
- STEP hooks: ['00:00:00.000000000] STEP 0 22020000 6321', '00:00:00.000063250] STEP 1 22020000 6587', '00:00:00.004800000] STEP 2 22020001 486696', '00:00:00.009600000] STEP 3 22020001 966804', '00:00:00.014400000] STEP 4 22020001 1446910', '00:00:00.019200000] STEP 5 22020001 1927014', '00:00:00.019270140] STEP 6 22020000 1927276', '00:00:00.019272780] STEP 7 22020000 1927540']
- RAM from the boot word: +0x0=0x2100b007 +0x4=0x0406aaf0 +0x8=0x22020000(ok) +0xc=0x00000060 +0x10=0x22020000(ok) +0x14=0x00000060 +0x18=0x22020001(timeout_txis) +0x1c=0x00000000 +0x20=0x22020001(timeout_txis) +0x24=0x00000000 +0x28=0x22020001(timeout_txis) +0x2c=0x00000000 +0x30=0x22020001(timeout_txis) +0x34=0x00000000 +0x38=0x22020000(ok) +0x3c=0x00000060 +0x40=0x22020000(ok) +0x44=0x00000060 +0x48=0x22020001(timeout_txis) +0x4c=0x00000000

## C2/old

- generate rc 0, build rc 0, simulate rc 0; elf e4e24372ef2d4bb4, main.rs ea6999104032c800
- reads in main.rs (2): i2c1 0x76 [0xd0]; spi1 Spi1Device::U4 [0x9f]
- verdict **pass**; findings: [('device_unmodelled', 'info')]
  - `device_unmodelled`: U3 (TMP117) at I2C address 0x48 has no Renode 1.17.0 model, so its identity was not read
- `absent` fields in the verdict: [[]]
- boot log lines 110; `Unknown slave`: []
- multiplexer unrouted (1): ["[00:00:00.000000000] [WARNING] Tried to finish transmission to device 0x0, but it's not connected"]
- BOOTWORD: ['00:00:00.000023490] BOOTWORD 2100b007 0 2905']
- STEP hooks: ['00:00:00.000000000] STEP 0 22020000 2345']
- RAM from the boot word: +0x0=0x2100b007 +0x4=0x0406aafc +0x8=0x22020000(ok) +0xc=0x00000060 +0x10=0x22020000(ok) +0x14=0x00ef4017 +0x18=0x00000001

## C2/new

- generate rc 0, build rc 0, simulate rc 1; elf 3fa1832d60b6a2a6, main.rs 8f3cbb9446447674
- reads in main.rs (3): i2c1 0x76 [0xd0]; i2c1 0x48 [0x0f]; spi1 Spi1Device::U4 [0x9f]
- verdict **fail**; findings: [('spi_record_invalid', 'error'), ('device_unmodelled', 'info')]
  - `spi_record_invalid`: U4 on SPI1, chip select PA0: the record reads status 0x22020001 and value 0x00000000, which no completed or timed-out transfer writes
  - `device_unmodelled`: U3 (TMP117) at I2C address 0x48 has no Renode 1.17.0 model, so its identity was not read
- `absent` fields in the verdict: [['0x48']]
- boot log lines 112; `Unknown slave`: ['[00:00:00.000023760] [WARNING] i2c1: Unknown slave at address 72.']
- multiplexer unrouted (1): ["[00:00:00.000000000] [WARNING] Tried to finish transmission to device 0x0, but it's not connected"]
- BOOTWORD: ['00:00:00.003600000] BOOTWORD 2100b007 0 363000']
- STEP hooks: ['00:00:00.000000000] STEP 0 22020000 2370']
- RAM from the boot word: +0x0=0x2100b007 +0x4=0x0406aafc +0x8=0x22020000(ok) +0xc=0x00000060 +0x10=0x22020001(timeout_txis) +0x14=0x00000000 +0x18=0x22020000(ok) +0x1c=0x00ef4017 +0x20=0x00000001

**C2: ELF old == new: False; main.rs equal: False**

## C3/old

- generate rc 0, build rc 0, simulate rc 0; elf 29c978e1ac6cf387, main.rs e6e68ad3ece0b59f
- reads in main.rs (3): i2c1 0x76 [0xd0]; i2c1 0x77 [0xd0]; spi1 Spi1Device::U4 [0x9f]
- verdict **pass**; findings: [('device_unmodelled', 'info')]
  - `device_unmodelled`: U3 (TMP117) at I2C address 0x48 has no Renode 1.17.0 model, so its identity was not read
- `absent` fields in the verdict: [[], []]
- boot log lines 58; `Unknown slave`: []
- multiplexer unrouted (1): ["[00:00:00.000000000] [WARNING] Tried to finish transmission to device 0x0, but it's not connected"]
- BOOTWORD: ['00:00:00.000013850] BOOTWORD 2100b007 0 1938']
- STEP hooks: ['00:00:00.000000000] STEP 0 22020000 1219', '00:00:00.000012220] STEP 1 22020000 1382']
- RAM from the boot word: +0x0=0x2100b007 +0x4=0x0406aaf0 +0x8=0x22020000(ok) +0xc=0x00000060 +0x10=0x22020000(ok) +0x14=0x00000060 +0x18=0x22020000(ok) +0x1c=0x00ef4017 +0x20=0x00000001

## C3/new

- generate rc 0, build rc 0, simulate rc 1; elf 298d3d4865766ed9, main.rs f435237c64021e89
- reads in main.rs (4): i2c1 0x76 [0xd0]; i2c1 0x77 [0xd0]; i2c1 0x48 [0x0f]; spi1 Spi1Device::U4 [0x9f]
- verdict **fail**; findings: [('spi_record_invalid', 'error'), ('device_unmodelled', 'info')]
  - `spi_record_invalid`: U4 on SPI1, chip select PA0: the record reads status 0x22020001 and value 0x00000000, which no completed or timed-out transfer writes
  - `device_unmodelled`: U3 (TMP117) at I2C address 0x48 has no Renode 1.17.0 model, so its identity was not read
- `absent` fields in the verdict: [['0x48'], ['0x48']]
- boot log lines 60; `Unknown slave`: ['[00:00:00.000014040] [WARNING] i2c1: Unknown slave at address 72.']
- multiplexer unrouted (1): ["[00:00:00.000000000] [WARNING] Tried to finish transmission to device 0x0, but it's not connected"]
- BOOTWORD: ['00:00:00.003600000] BOOTWORD 2100b007 0 362030']
- STEP hooks: ['00:00:00.000000000] STEP 0 22020000 1229', '00:00:00.000012310] STEP 1 22020000 1402']
- RAM from the boot word: +0x0=0x2100b007 +0x4=0x0406aaf0 +0x8=0x22020000(ok) +0xc=0x00000060 +0x10=0x22020000(ok) +0x14=0x00000060 +0x18=0x22020001(timeout_txis) +0x1c=0x00000000 +0x20=0x22020000(ok) +0x24=0x00ef4017 +0x28=0x00000001

**C3: ELF old == new: False; main.rs equal: False**

## C4/old

- generate rc 0, build rc 0, simulate rc 0; elf a3618b2cea26d8ff, main.rs ee570219ac00607e
- reads in main.rs (5): i2c1 0x76 [0xd0]; i2c2 0x47 [0x01]; i2c2 0x47 [0x36, 0x40, 0x81]; spi1 Spi1Device::U4 [0x9f]; i2c2 0x47 [0x1d]
- verdict **pass**; findings: [('device_unmodelled', 'info')]
  - `device_unmodelled`: U3 (TMP117) at I2C address 0x48 has no Renode 1.17.0 model, so its identity was not read
- `absent` fields in the verdict: [[], [], [], []]
- boot log lines 120; `Unknown slave`: []
- multiplexer unrouted (1): ["[00:00:00.000000000] [WARNING] Tried to finish transmission to device 0x0, but it's not connected"]
- BOOTWORD: ['00:00:00.019524480] BOOTWORD 2100b007 0 1952471']
- STEP hooks: ['00:00:00.000000000] STEP 0 22020000 2353', '00:00:00.000024580] STEP 1 22020000 2538', '00:00:00.000026790] STEP 2 22020000 2727', '00:00:00.019521610] STEP 4 22020000 1952448']
- RAM from the boot word: +0x0=0x2100b007 +0x4=0x0406aafc +0x8=0x22020000(ok) +0xc=0x00000060 +0x10=0x22020000(ok) +0x14=0x00000050 +0x18=0x22020000(ok) +0x1c=0x00000000 +0x20=0x22020000(ok) +0x24=0x00ef4017 +0x28=0x22020000(ok) +0x2c=0x00000000 +0x30=0x1940f362 +0x34=0x00000001

## C4/new

- generate rc 0, build rc 0, simulate rc 1; elf 7469f0ff0820481c, main.rs 2dafba4d52d3ce03
- reads in main.rs (6): i2c1 0x76 [0xd0]; i2c1 0x48 [0x0f]; i2c2 0x47 [0x01]; i2c2 0x47 [0x36, 0x40, 0x81]; spi1 Spi1Device::U4 [0x9f]; i2c2 0x47 [0x1d]
- verdict **fail**; findings: [('i2c_timeout', 'error'), ('i2c_configure_mismatch', 'error'), ('spi_identity_mismatch', 'error'), ('i2c_data_mismatch', 'error'), ('device_unmodelled', 'info')]
  - `i2c_timeout`: U5 at I2C address 0x47: the firmware's wait for TXIS expired reading register 0x01 (expected 0x50)
  - `i2c_configure_mismatch`: U5 at I2C address 0x47, its configure step: register 0x36 0x40 0x81 read 0x50, the part table says 0x0 under mask 0x0
  - `spi_identity_mismatch`: U4 on SPI1, chip select PA0: command 0x9f read 0x0, the part table says 0xef4017
  - `i2c_data_mismatch`: U5 at I2C address 0x47, its data step: register 0x1d read 0xef401722020000, the part table says 0x1940f362
  - `device_unmodelled`: U3 (TMP117) at I2C address 0x48 has no Renode 1.17.0 model, so its identity was not read
- `absent` fields in the verdict: [['0x48'], [], [], []]
- boot log lines 122; `Unknown slave`: ['[00:00:00.000023730] [WARNING] i2c1: Unknown slave at address 72.']
- multiplexer unrouted (1): ["[00:00:00.000000000] [WARNING] Tried to finish transmission to device 0x0, but it's not connected"]
- BOOTWORD: ['00:00:00.023122490] BOOTWORD 2100b007 0 2312559']
- STEP hooks: ['00:00:00.000000000] STEP 0 22020000 2371', '00:00:00.003600000] STEP 1 22020001 362444', '00:00:00.003625490] STEP 2 22020000 362629', '00:00:00.003627700] STEP 4 22020000 363372']
- RAM from the boot word: +0x0=0x2100b007 +0x4=0x0406aafc +0x8=0x22020000(ok) +0xc=0x00000060 +0x10=0x22020001(timeout_txis) +0x14=0x00000000 +0x18=0x22020000(ok) +0x1c=0x00000050 +0x20=0x22020000(ok) +0x24=0x00000000 +0x28=0x22020000(ok) +0x2c=0x00ef4017 +0x30=0x22020000(ok) +0x34=0x00000000 +0x38=0x1940f362 +0x3c=0x00000001

**C4: ELF old == new: False; main.rs equal: False**

## C5/old

- generate rc 0, build rc 0, simulate rc 0; elf 73c34930412eb436, main.rs 5293862924227597
- reads in main.rs (9): i2c1 0x46 [0x01]; i2c1 0x46 [0x36, 0x40, 0x81]; spi1 Spi1Device::U4 [0x9f]; spi2 Spi2Device::U5 [0xf5]; spi2 Spi2Device::U5 [0x4c, 0xc0]; spi2 Spi2Device::U5 [0x50, 0x66]; spi2 Spi2Device::U5 [0x4e, 0x03]; i2c1 0x46 [0x1d]; spi2 Spi2Device::U5 [0x9d]
- verdict **pass**; findings: [('device_unmodelled', 'info')]
  - `device_unmodelled`: U3 (TMP117) at I2C address 0x48 has no Renode 1.17.0 model, so its identity was not read
- `absent` fields in the verdict: [[], [], []]
- boot log lines 9916; `Unknown slave`: []
- multiplexer unrouted (2): ["[00:00:00.000000000] [WARNING] Tried to finish transmission to device 0x0, but it's not connected", "[00:00:00.000000000] [WARNING] Tried to finish transmission to device 0x0, but it's not connected"]
- BOOTWORD: ['00:00:00.039128390] BOOTWORD 2100b007 0 3912839']
- STEP hooks: ['00:00:00.000067440] STEP 0 22020000 6845', '00:00:00.000070320] STEP 1 22020000 7093', '00:00:00.039105210] STEP 7 22020000 3910887']
- RAM from the boot word: +0x0=0x2100b007 +0x4=0x0406aafc +0x8=0x22020000(ok) +0xc=0x00000050 +0x10=0x22020000(ok) +0x14=0x00000000 +0x18=0x22020000(ok) +0x1c=0x00ef4017 +0x20=0x22020000(ok) +0x24=0x00000047 +0x28=0x22020000(ok) +0x2c=0x00000000 +0x30=0x22020000(ok) +0x34=0x00000000 +0x38=0x22020000(ok) +0x3c=0x00000000 +0x40=0x22020000(ok) +0x44=0x00000000 +0x48=0x1940f362 +0x4c=0x22020000(ok)

## C5/new

- generate rc 0, build rc 0, simulate rc 1; elf 61a7e67aab10fb7d, main.rs 513a829a05767555
- reads in main.rs (10): i2c1 0x48 [0x0f]; i2c1 0x46 [0x01]; i2c1 0x46 [0x36, 0x40, 0x81]; spi1 Spi1Device::U4 [0x9f]; spi2 Spi2Device::U5 [0xf5]; spi2 Spi2Device::U5 [0x4c, 0xc0]; spi2 Spi2Device::U5 [0x50, 0x66]; spi2 Spi2Device::U5 [0x4e, 0x03]; i2c1 0x46 [0x1d]; spi2 Spi2Device::U5 [0x9d]
- verdict **fail**; findings: [('i2c_timeout', 'error'), ('i2c_configure_mismatch', 'error'), ('spi_identity_mismatch', 'error'), ('spi_identity_mismatch', 'error'), ('spi_configure_mismatch', 'error'), ('i2c_data_mismatch', 'error'), ('spi_record_invalid', 'error'), ('device_unmodelled', 'info')]
  - `i2c_timeout`: U7 at I2C address 0x46: the firmware's wait for TXIS expired reading register 0x01 (expected 0x50)
  - `i2c_configure_mismatch`: U7 at I2C address 0x46, its configure step: register 0x36 0x40 0x81 read 0x50, the part table says 0x0 under mask 0x0
  - `spi_identity_mismatch`: U4 on SPI1, chip select PA0: command 0x9f read 0x0, the part table says 0xef4017
  - `spi_identity_mismatch`: U5 on SPI2, chip select PB12: command 0xf5 read 0xef4017, the part table says 0x47
  - `spi_configure_mismatch`: U5 on SPI2, chip select PB12, its configure 1 step: command 0x4c 0xc0 read 0x47, the part table says 0x0 under mask 0x0
  - `i2c_data_mismatch`: U7 at I2C address 0x46, its data step: register 0x1d read 0x22020000, the part table says 0x1940f362
  - `spi_record_invalid`: U5 on SPI2, chip select PB12, its data step: the record reads status 0x00000000 and value 0x1940f36222020000, which no completed or timed-out transfer writes
  - `device_unmodelled`: U3 (TMP117) at I2C address 0x48 has no Renode 1.17.0 model, so its identity was not read
- `absent` fields in the verdict: [['0x48'], [], []]
- boot log lines 9918; `Unknown slave`: ['[00:00:00.000057850] [WARNING] i2c1: Unknown slave at address 72.']
- multiplexer unrouted (2): ["[00:00:00.000000000] [WARNING] Tried to finish transmission to device 0x0, but it's not connected", "[00:00:00.000000000] [WARNING] Tried to finish transmission to device 0x0, but it's not connected"]
- BOOTWORD: ['00:00:00.043929490] BOOTWORD 2100b007 0 4392949']
- STEP hooks: ['00:00:00.004800000] STEP 0 22020001 486699', '00:00:00.004868460] STEP 1 22020000 486946', '00:00:00.004919060] STEP 7 22020000 492940']
- RAM from the boot word: +0x0=0x2100b007 +0x4=0x0406aafc +0x8=0x22020001(timeout_txis) +0xc=0x00000000 +0x10=0x22020000(ok) +0x14=0x00000050 +0x18=0x22020000(ok) +0x1c=0x00000000 +0x20=0x22020000(ok) +0x24=0x00ef4017 +0x28=0x22020000(ok) +0x2c=0x00000047 +0x30=0x22020000(ok) +0x34=0x00000000 +0x38=0x22020000(ok) +0x3c=0x00000000 +0x40=0x22020000(ok) +0x44=0x00000000 +0x48=0x22020000(ok) +0x4c=0x00000000

**C5: ELF old == new: False; main.rs equal: False**

## C6/new

- generate rc 0, build rc 0, simulate rc 0; elf 406af07792828dc0, main.rs 0dda50a50dfca746
- reads in main.rs (3): i2c1 0x76 [0xd0]; i2c1 0x48 [0x0f]; spi1 Spi1Device::U4 [0x9f]
- verdict **pass**; findings: [('device_unmodelled', 'info'), ('device_unmodelled', 'info')]
  - `device_unmodelled`: U3 (TMP117) at I2C address 0x48 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: U4 (W25Q64JVxIQ) on SPI1, chip select PA0 has no Renode 1.17.0 model, so its identity was not read
- `absent` fields in the verdict: [['0x48']]
- boot log lines 102; `Unknown slave`: ['[00:00:00.000021330] [WARNING] i2c1: Unknown slave at address 72.']
- multiplexer unrouted (7): ["[00:00:00.000000000] [WARNING] Tried to finish transmission to device 0x0, but it's not connected", "[00:00:00.000000000] [WARNING] spi1.spi1mux: Tried to finish transmission to device 0x0, but it's not connected", "[00:00:00.003600000] [WARNING] spi1.spi1mux: Tried to transmit byte 0x9F to device 0x0, but it's not connected - ignoring transfer and returning a dummy byte", "[00:00:00.003600000] [WARNING] spi1.spi1mux: Tried to transmit byte 0x0 to device 0x0, but it's not connected - ignoring transfer and returning a dummy byte"] …
- BOOTWORD: ['00:00:00.003600000] BOOTWORD 2100b007 0 362757']
- STEP hooks: ['00:00:00.000000000] STEP 0 22020000 2127']
- RAM from the boot word: +0x0=0x2100b007 +0x4=0x0406aafc +0x8=0x22020000(ok) +0xc=0x00000060 +0x10=0x22020001(timeout_txis) +0x14=0x00000000 +0x18=0x22020000(ok) +0x1c=0x00ffffff +0x20=0x00000001

## C7/new

- generate rc 0, build rc 0, simulate rc 0; elf 406af07792828dc0, main.rs 0dda50a50dfca746
- reads in main.rs (3): i2c1 0x76 [0xd0]; i2c1 0x48 [0x0f]; spi1 Spi1Device::U4 [0x9f]
- verdict **pass**; findings: [('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info')]
  - `device_unmodelled`: U2 (BME280) at I2C address 0x76 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: U3 (TMP117) at I2C address 0x48 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: U4 (W25Q64JVxIQ) on SPI1, chip select PA0 has no Renode 1.17.0 model, so its identity was not read
- boot log lines 100; `Unknown slave`: ['[00:00:00.000000000] [WARNING] i2c1: Unknown slave at address 118.', '[00:00:00.003600000] [WARNING] i2c1: Unknown slave at address 72.']
- multiplexer unrouted (7): ["[00:00:00.000000000] [WARNING] Tried to finish transmission to device 0x0, but it's not connected", "[00:00:00.000000000] [WARNING] spi1.spi1mux: Tried to finish transmission to device 0x0, but it's not connected", "[00:00:00.007200000] [WARNING] spi1.spi1mux: Tried to transmit byte 0x9F to device 0x0, but it's not connected - ignoring transfer and returning a dummy byte", "[00:00:00.007200000] [WARNING] spi1.spi1mux: Tried to transmit byte 0x0 to device 0x0, but it's not connected - ignoring transfer and returning a dummy byte"] …
- BOOTWORD: ['00:00:00.007200000] BOOTWORD 2100b007 0 722655']
- STEP hooks: []
- RAM from the boot word: +0x0=0x2100b007 +0x4=0x0406aafc +0x8=0x22020001(timeout_txis) +0xc=0x00000000 +0x10=0x22020001(timeout_txis) +0x14=0x00000000 +0x18=0x22020000(ok) +0x1c=0x00ffffff +0x20=0x00000001

## C8/old

- generate rc 0, build rc 0, simulate rc 0; elf 5fda85db6ac73c45, main.rs c8f5573dcb656631
- reads in main.rs (9): i2c1 0x76 [0xd0]; i2c1 0x77 [0xd0]; i2c2 0x76 [0xd0]; i2c2 0x77 [0xd0]; i2c3 0x76 [0xd0]; i2c3 0x77 [0xd0]; i2c4 0x76 [0xd0]; i2c4 0x77 [0xd0]; spi1 Spi1Device::S1000 [0x9f]
- verdict **pass**; findings: [('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info')]
  - `device_unmodelled`: S2 (TMP117) at I2C address 0x48 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S3 (TMP117) at I2C address 0x49 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S4 (TMP117) at I2C address 0x4a has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S5 (TMP117) at I2C address 0x4b has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S8 (TMP117) at I2C address 0x48 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S9 (TMP117) at I2C address 0x49 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S10 (TMP117) at I2C address 0x4a has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S11 (TMP117) at I2C address 0x4b has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S14 (TMP117) at I2C address 0x48 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S15 (TMP117) at I2C address 0x49 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S16 (TMP117) at I2C address 0x4a has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S17 (TMP117) at I2C address 0x4b has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S20 (TMP117) at I2C address 0x48 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S21 (TMP117) at I2C address 0x49 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S22 (TMP117) at I2C address 0x4a has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S23 (TMP117) at I2C address 0x4b has no Renode 1.17.0 model, so its identity was not read
- `absent` fields in the verdict: [[], [], [], [], [], [], [], []]
- boot log lines 91; `Unknown slave`: []
- multiplexer unrouted (1): ["[00:00:00.000000000] [WARNING] Tried to finish transmission to device 0x0, but it's not connected"]
- BOOTWORD: ['00:00:00.000031880] BOOTWORD 2100b007 0 3741']
- STEP hooks: ['00:00:00.000000000] STEP 0 22020000 2050', '00:00:00.000020520] STEP 1 22020000 2212', '00:00:00.000022140] STEP 2 22020000 2374', '00:00:00.000023760] STEP 3 22020000 2536', '00:00:00.000025380] STEP 4 22020000 2698', '00:00:00.000027000] STEP 5 22020000 2860', '00:00:00.000028620] STEP 6 22020000 3022', '00:00:00.000030250] STEP 7 22020000 3185']
- RAM from the boot word: +0x0=0x2100b007 +0x4=0x0406aaf0 +0x8=0x22020000(ok) +0xc=0x00000060 +0x10=0x22020000(ok) +0x14=0x00000060 +0x18=0x22020000(ok) +0x1c=0x00000060 +0x20=0x22020000(ok) +0x24=0x00000060 +0x28=0x22020000(ok) +0x2c=0x00000060 +0x30=0x22020000(ok) +0x34=0x00000060 +0x38=0x22020000(ok) +0x3c=0x00000060 +0x40=0x22020000(ok) +0x44=0x00000060 +0x48=0x22020000(ok) +0x4c=0x00ef4017

## C8/new

- generate rc 0, build rc 0, simulate rc 1; elf e1087d4d9ef5edc1, main.rs 391c868ba133cc7c
- reads in main.rs (25): i2c1 0x76 [0xd0]; i2c1 0x77 [0xd0]; i2c1 0x48 [0x0f]; i2c1 0x49 [0x0f]; i2c1 0x4a [0x0f]; i2c1 0x4b [0x0f]; i2c2 0x76 [0xd0]; i2c2 0x77 [0xd0]; i2c2 0x48 [0x0f]; i2c2 0x49 [0x0f]; i2c2 0x4a [0x0f]; i2c2 0x4b [0x0f]; i2c3 0x76 [0xd0]; i2c3 0x77 [0xd0]; i2c3 0x48 [0x0f]; i2c3 0x49 [0x0f]; i2c3 0x4a [0x0f]; i2c3 0x4b [0x0f]; i2c4 0x76 [0xd0]; i2c4 0x77 [0xd0]; i2c4 0x48 [0x0f]; i2c4 0x49 [0x0f]; i2c4 0x4a [0x0f]; i2c4 0x4b [0x0f]; spi1 Spi1Device::S1000 [0x9f]
- verdict **fail**; findings: [('i2c_timeout', 'error'), ('i2c_timeout', 'error'), ('i2c_timeout', 'error'), ('i2c_timeout', 'error'), ('spi_record_invalid', 'error'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info'), ('device_unmodelled', 'info')]
  - `i2c_timeout`: S6 at I2C address 0x76: the firmware's wait for TXIS expired reading register 0xd0 (expected 0x60)
  - `i2c_timeout`: S7 at I2C address 0x77: the firmware's wait for TXIS expired reading register 0xd0 (expected 0x60)
  - `i2c_timeout`: S12 at I2C address 0x76: the firmware's wait for TXIS expired reading register 0xd0 (expected 0x60)
  - `i2c_timeout`: S13 at I2C address 0x77: the firmware's wait for TXIS expired reading register 0xd0 (expected 0x60)
  - `spi_record_invalid`: S1000 on SPI1, chip select PA0: the record reads status 0x22020001 and value 0x00000000, which no completed or timed-out transfer writes
  - `device_unmodelled`: S2 (TMP117) at I2C address 0x48 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S3 (TMP117) at I2C address 0x49 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S4 (TMP117) at I2C address 0x4a has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S5 (TMP117) at I2C address 0x4b has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S8 (TMP117) at I2C address 0x48 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S9 (TMP117) at I2C address 0x49 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S10 (TMP117) at I2C address 0x4a has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S11 (TMP117) at I2C address 0x4b has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S14 (TMP117) at I2C address 0x48 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S15 (TMP117) at I2C address 0x49 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S16 (TMP117) at I2C address 0x4a has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S17 (TMP117) at I2C address 0x4b has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S20 (TMP117) at I2C address 0x48 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S21 (TMP117) at I2C address 0x49 has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S22 (TMP117) at I2C address 0x4a has no Renode 1.17.0 model, so its identity was not read
  - `device_unmodelled`: S23 (TMP117) at I2C address 0x4b has no Renode 1.17.0 model, so its identity was not read
- `absent` fields in the verdict: [['0x48', '0x49', '0x4a', '0x4b'], ['0x48', '0x49', '0x4a', '0x4b'], ['0x48', '0x49', '0x4a', '0x4b'], ['0x48', '0x49', '0x4a', '0x4b'], ['0x48', '0x49', '0x4a', '0x4b'], ['0x48', '0x49', '0x4a', '0x4b'], ['0x48', '0x49', '0x4a', '0x4b'], ['0x48', '0x49', '0x4a', '0x4b']]
- boot log lines 123; `Unknown slave`: ['[00:00:00.000022360] [WARNING] i2c1: Unknown slave at address 72.', '[00:00:00.003623070] [WARNING] i2c1: Unknown slave at address 73.', '[00:00:00.007223780] [WARNING] i2c1: Unknown slave at address 74.', '[00:00:00.010824490] [WARNING] i2c1: Unknown slave at address 75.', '[00:00:00.014428660] [WARNING] i2c2: Unknown slave at address 72.', '[00:00:00.018000000] [WARNING] i2c2: Unknown slave at address 73.', '[00:00:00.021600000] [WARNING] i2c2: Unknown slave at address 74.', '[00:00:00.025200000] [WARNING] i2c2: Unknown slave at address 75.', '[00:00:00.028800000] [WARNING] i2c3: Unknown slave at address 72.', '[00:00:00.032400000] [WARNING] i2c3: Unknown slave at address 73.', '[00:00:00.036000000] [WARNING] i2c3: Unknown slave at address 74.', '[00:00:00.039600000] [WARNING] i2c3: Unknown slave at address 75.', '[00:00:00.043200000] [WARNING] i2c4: Unknown slave at address 72.', '[00:00:00.046800000] [WARNING] i2c4: Unknown slave at address 73.', '[00:00:00.050400000] [WARNING] i2c4: Unknown slave at address 74.', '[00:00:00.054000000] [WARNING] i2c4: Unknown slave at address 75.']
- multiplexer unrouted (1): ["[00:00:00.000000000] [WARNING] Tried to finish transmission to device 0x0, but it's not connected"]
- BOOTWORD: ['00:00:00.057600000] BOOTWORD 2100b007 0 5764965']
- STEP hooks: ['00:00:00.000000000] STEP 0 22020000 2061', '00:00:00.000020630] STEP 1 22020000 2234', '00:00:00.003600000] STEP 2 22020001 362307', '00:00:00.007200000] STEP 3 22020001 722378', '00:00:00.010800000] STEP 4 22020001 1082449', '00:00:00.014400000] STEP 5 22020001 1442520', '00:00:00.014425200] STEP 6 22020000 1442691', '00:00:00.014426930] STEP 7 22020000 1442864']
- RAM from the boot word: +0x0=0x2100b007 +0x4=0x0406aaf0 +0x8=0x22020000(ok) +0xc=0x00000060 +0x10=0x22020000(ok) +0x14=0x00000060 +0x18=0x22020001(timeout_txis) +0x1c=0x00000000 +0x20=0x22020001(timeout_txis) +0x24=0x00000000 +0x28=0x22020001(timeout_txis) +0x2c=0x00000000 +0x30=0x22020001(timeout_txis) +0x34=0x00000000 +0x38=0x22020000(ok) +0x3c=0x00000060 +0x40=0x22020000(ok) +0x44=0x00000060 +0x48=0x22020001(timeout_txis) +0x4c=0x00000000

**C8: ELF old == new: False; main.rs equal: False**

## C9/old

- generate rc 0, build rc 0, simulate rc 0; elf 2bb1be79503255b9, main.rs 5669f081acbda102
- reads in main.rs (2): i2c1 0x76 [0xd0]; spi1 Spi1Device::U4 [0x9f]
- verdict **pass**; findings: []
- `absent` fields in the verdict: [[]]
- boot log lines 108; `Unknown slave`: []
- multiplexer unrouted (1): ["[00:00:00.000000000] [WARNING] Tried to finish transmission to device 0x0, but it's not connected"]
- BOOTWORD: ['00:00:00.000021060] BOOTWORD 2100b007 0 2662']
- STEP hooks: ['00:00:00.000000000] STEP 0 22020000 2102']
- RAM from the boot word: +0x0=0x2100b007 +0x4=0x0406aafc +0x8=0x22020000(ok) +0xc=0x00000060 +0x10=0x22020000(ok) +0x14=0x00ef4017 +0x18=0x00000001

## C9/new

- generate rc 0, build rc 0, simulate rc 0; elf 2bb1be79503255b9, main.rs 5669f081acbda102
- reads in main.rs (2): i2c1 0x76 [0xd0]; spi1 Spi1Device::U4 [0x9f]
- verdict **pass**; findings: []
- `absent` fields in the verdict: [[]]
- boot log lines 108; `Unknown slave`: []
- multiplexer unrouted (1): ["[00:00:00.000000000] [WARNING] Tried to finish transmission to device 0x0, but it's not connected"]
- BOOTWORD: ['00:00:00.000021060] BOOTWORD 2100b007 0 2662']
- STEP hooks: ['00:00:00.000000000] STEP 0 22020000 2102']
- RAM from the boot word: +0x0=0x2100b007 +0x4=0x0406aafc +0x8=0x22020000(ok) +0xc=0x00000060 +0x10=0x22020000(ok) +0x14=0x00ef4017 +0x18=0x00000001

**C9: ELF old == new: True; main.rs equal: True**

