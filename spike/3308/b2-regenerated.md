| Register | Address | Mask | Expected | Source |
|---|---|---|---|---|
| USART1.CR1 | `0x40011000` | `0x040087ed` | `0x0000000d` | twin; generator |
| USART1.CR2 | `0x40011004` | `0x00003000` | `0x00000000` | generator |
| USART1.CR3 | `0x40011008` | `0x000000c0` | `0x00000000` | twin (Renode's reset value: the firmware never writes it) |
| USART1.BRR | `0x4001100c` | `0x0000ffff` | `0x0000022c` | rule |
| GPIOA.MODER | `0x58020000` | `0x003cfc03` | `0x0028a801` | generator |
| GPIOA.AFRL | `0x58020020` | `0xfff00000` | `0x55500000` | generator |
| GPIOA.AFRH | `0x58020024` | `0x00000ff0` | `0x00000770` | generator |
| GPIOB.MODER | `0x58020400` | `0x00f0f000` | `0x00a0a000` | generator |
| GPIOB.AFRL | `0x58020420` | `0xff000000` | `0x44000000` | generator |
| GPIOB.AFRH | `0x58020424` | `0x0000ff00` | `0x00004400` | generator |
| RCC.AHB3ENR | `0x580244d4` | `0xffffffff` | `0x00000000` | twin (Renode's reset value: the firmware never writes it) |
| RCC.AHB1ENR | `0x580244d8` | `0xffffffff` | `0x00000000` | twin (Renode's reset value: the firmware never writes it) |
| RCC.AHB2ENR | `0x580244dc` | `0xffffffff` | `0x00000000` | twin (Renode's reset value: the firmware never writes it) |
| RCC.AHB4ENR | `0x580244e0` | `0xffffffff` | `0x00000003` | twin |
| RCC.APB3ENR | `0x580244e4` | `0xffffffff` | `0x00000000` | twin (Renode's reset value: the firmware never writes it) |
| RCC.APB1LENR | `0x580244e8` | `0xffffffff` | `0x00600000` | twin; generator |
| RCC.APB1HENR | `0x580244ec` | `0xffffffff` | `0x00000000` | twin (Renode's reset value: the firmware never writes it) |
| RCC.APB2ENR | `0x580244f0` | `0xffffffff` | `0x00001010` | twin |
| RCC.APB4ENR | `0x580244f4` | `0xffffffff` | `0x00010000` | twin (Renode's reset value: the firmware never writes it) |
| SysTick.CSR | `0xe000e010` | `0x00010003` | `0x00000000` | twin (Renode's reset value: the firmware never writes it) |
| NVIC.ISER0 | `0xe000e100` | `0xffffffff` | `0x00000000` | twin (Renode's reset value: the firmware never writes it) |
| NVIC.ISER1 | `0xe000e104` | `0xffffffff` | `0x00000000` | twin (Renode's reset value: the firmware never writes it) |
| NVIC.ISER2 | `0xe000e108` | `0xffffffff` | `0x00000000` | twin (Renode's reset value: the firmware never writes it) |
| NVIC.ISER3 | `0xe000e10c` | `0xffffffff` | `0x00000000` | twin (Renode's reset value: the firmware never writes it) |
| NVIC.ISER4 | `0xe000e110` | `0xffffffff` | `0x00000000` | twin (Renode's reset value: the firmware never writes it) |
| SCB.CPACR | `0xe000ed88` | `0x00f00000` | `0x00f00000` | generator |
| PWR.CR3 | `0x5802480c` | `0x00000007` | `0x00000002` | rule |
| PWR.D3CR | `0x58024818` | `0x0000e000` | `0x00008000` | rule |
| RCC.D2CCIP1R | `0x58024450` | `0xb13771c7` | `0x00004000` | rule |
| GPIOA.OTYPER | `0x58020004` | `0x0000ffff` | `0x00000000` | not predicted (no model reacts) |
| GPIOA.PUPDR | `0x5802000c` | `0xffffffff` | `0x64000000` | generator |
| GPIOB.OTYPER | `0x58020404` | `0x0000ffff` | `0x00000cc0` | not predicted (no model reacts) |
| GPIOB.PUPDR | `0x5802040c` | `0xffffffff` | `0x00000100` | generator |
| GPIOA.OSPEEDR | `0x58020008` | `0x003cfc03` | `0x00004400` | generator |
| GPIOB.OSPEEDR | `0x58020408` | `0x00f0f000` | `0x00000000` | generator |
| I2C1.CR1 | `0x40005400` | `0x00ffdfff` | `0x00000001` | generator; rule |
| I2C1.TIMINGR | `0x40005410` | `0xf0ffffff` | `0x10c7123c` | generator, read back (the rule holds the fields by inequalities, not the word) |
| I2C1.CR2 | `0x40005404` | `0x0500d800` | `0x00000000` | generator |
| I2C2.CR1 | `0x40005800` | `0x00ffdfff` | `0x00000001` | generator; rule |
| I2C2.TIMINGR | `0x40005810` | `0xf0ffffff` | `0x50d32a40` | generator, read back (the rule holds the fields by inequalities, not the word) |
| I2C2.CR2 | `0x40005804` | `0x0500d800` | `0x00000000` | generator |
| SPI1.CR1 | `0x40013000` | `0x0001ff01` | `0x00001000` | rule |
| SPI1.CFG1 | `0x40013008` | `0x705fdfff` | `0x10070007` | rule |
| SPI1.CFG2 | `0x4001300c` | `0xf7fe80ff` | `0x84400000` | rule |
| PWR.CSR1 | `0x58024804` | `0x00002000` | `0x00002000` | not predicted (silicon-only) |
| FLASH.ACR | `0x52002000` | `0x0000003f` | `0x00000037` | not predicted (silicon-only) |
| RCC.CR | `0x58024400` | `0x00008018` | `0x00008000` | twin; not predicted (no model reacts) |
| RCC.CFGR | `0x58024410` | `0x00008038` | `0x00000000` | twin |
| RCC.D1CFGR | `0x58024418` | `0x00000f7f` | `0x00000000` | twin |
| RCC.D2CFGR | `0x5802441c` | `0x00000770` | `0x00000000` | twin |
| RCC.D1CCIPR | `0x5802444c` | `0x30000000` | `0x00000000` | not predicted (no model reacts) |
| RCC.D2CCIP2R | `0x58024454` | `0x0000303f` | `0x00000000` | not predicted (no model reacts) |
