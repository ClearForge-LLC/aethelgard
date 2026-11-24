# AET-WO-4300k D10: Sentinel-03 acknowledgement reasons, the facts each one rests on

Output: `acks.json`, 46 entries in the order of `me/entries.json`. Each entry's fields were copied from that file by a script, not retyped.

## Sources used throughout (abbreviations below)

- **LEDGER**: the scratch test `zz_scratch_acks.rs` (now deleted). It built Sentinel-03's ledger exactly as `zz_scratch_4300k.rs` does and dumped every unwitnessed row with its judge, note, value and `on_the_bench` to `rows.json`, `unack.txt`, `bench.txt` and `rcc_gpio.txt` in this directory.
- **TOPO**: `./target/debug/aethelgard solve --file examples/sentinel-03.json`, saved as `solve.json`.
  - The binary dates from 10:11 today, while the working tree has uncommitted edits. Every pin fact taken from it was cross-checked against the LEDGER values, which come from the live tree.
- **SPEC**: `examples/sentinel-03.json`:
  - `connectors`: J6 CRSF, J7 MAVLINK, J8 EXT I2C (DS-009);
  - `mcu.reserved_pins` (L441-455): PA11/PA12 USB, PC9 MCO2 per 4300h D3;
  - `i2c_buses`: EXT 100 kHz;
  - `motor_outputs`.
- **F4300**: `docs/work-orders/AET-WO-4300.FEEDBACK.md` section 0.
  - Timer to pin: `STM32_GPIOPort` forwards a channel only while MODER is AF and AFR selects it.
  - BSRR drives a pin in any mode.
  - `BDTR.MOE` is a tag; `CCxP` is read only for inputs.
- **F4300a**: `AET-WO-4300a.FEEDBACK.md` section 0 (L27-28).
  - Tags: `CR1.CKD`, `CCMRx.OCxFE/OCxPE/OCxCE`, `CCER.CCxNE` and every BDTR field (L377-386).
  - `OCxM[3]` is reserved in the model.
  - Renode's timer1 channel 0 lists PA7 and PB13 as TIM1_CH1N.
- **F4300b**: `AET-WO-4300b.FEEDBACK.md` section 0. Line numbers are file lines.
  - L23: validate refuses a non-8N1 or inverted CRSF.
  - L28-31: the USART stored/tag lists.
  - L36: "No pin on Renode's USART path".
  - L26: the receiver-tolerance columns are taken at OVER8 = 0.
  - Section 10: OTYPER is unstored (measured). H1: FIFOEN.
- **F4240b**: `AET-WO-4240b.FEEDBACK.md`.
  - L15-18: the bring-up order, with bounded HSERDY, HSECSSON, then PLL1ON and a bounded PLL1RDY.
  - L128: BDTR is written as MOE alone.
- **F4240d**: `AET-WO-4240d.FEEDBACK.md` F3 (L86) and `.bench.md` L433: the bench shows that HSERDY and PLL1RDY came within their bounds, not how long they took.
- **F1211**: `AET-WO-1211.FEEDBACK.md` L370: MOE 0 or CC1E 0 disables the output (Table 337); the gating is unmodelled.

## Per entry

1. **GPIOA.AFRH ss** [AFSEL8,9,10].
   - TOPO: MOTOR1 PA8 is TIM1_CH1 at AF1; PA9/PA10 are USART1 at AF7.
   - LEDGER note: "acts on it only for a timer channel's pin ... the table cannot see the pin's role, so the weaker class".
   - F4300b section 0: no USART pin. `bench.txt`: on the bench.
2. **GPIOA.AFRL ss** [2,3,5,6,7].
   - TOPO: CRSF PA2/PA3 are USART2 at AF7; LEDGER AFSEL2/3 = 7. SPI1 PA5-PA7 is at AF5.
   - The held reason's "3302 B2 predicted 0x555" was dropped, because the word now also holds PA2/PA3.
3. **GPIOA.MODER ss** [0,2,3,5-10].
   - TOPO `gpio`: FLASH_CS PA0 is an output; LEDGER MODER0 = 1 and the others = 2.
   - F4300 and the LEDGER note: MODER is acted on only for a timer channel (PA8), and BSRR drives any mode (4200 FEEDBACK, as the note cites it).
4. **GPIOA.OSPEEDR ss**.
   - LEDGER: OSPEED5 and OSPEED7 = 1, the rest 0; TOPO `pin_speeds`: SPI_SCK and SPI_MOSI = 1.
   - TOPO `spi_controllers`: SPI1 achieves 16 MHz.
   - LEDGER note: no edge rate is modelled, and check speeds uses svd-hal's own table.
5. **GPIOA.PUPDR ss** [0-7, 9-15].
   - LEDGER: PUPD13 = 1, PUPD14 = 2, PUPD15 = 1; every other listed pin is 0.
   - SPEC: PA11/PA12 are reserved for USB.
   - LEDGER: PUPD8 is independent-rule (`implied_pupd`), which is why it is absent.
   - PUPD11 is new because PA11 was a motor pin earlier (3400 FEEDBACK L169).
6. **GPIOB.AFRH ss** [10,11,13,14,15].
   - TOPO: EXT_SCL/SDA PB10/PB11 are I2C2 at AF4; LEDGER AFSEL10/11 = 4.
   - TOPO: IMU PB13/14/15 are SPI2 at AF5; LEDGER = 5.
   - SPEC: J8 is DS-009 I2C.
7. **GPIOB.MODER ss** [6,7,10-15].
   - LEDGER: MODER12 = 1 (TOPO `gpio`: IMU_CS PB12 output); the others = 2.
   - "No timer pin here" means no timer channel is assigned on port B.
8. **GPIOB.OSPEEDR ss**.
   - LEDGER: all 0. TOPO: `i2c_speeds` EXT 100 kHz and SENSORS 400 kHz; SPI2 achieves 2 MHz (declared 4 MHz).
   - PB12 is the GPIO chip select, grouped under "SPI2".
9. **GPIOC.AFRH ss** [10,11].
   - TOPO: MAVLINK PC10/PC11 are USART3 at AF7; SPEC: J7.
   - F4300b L36: "a USART's RX pin on the wrong AF still receives an injected byte in Renode".
10. **GPIOC.MODER ss** [10,11].
    - LEDGER: both = 2.
    - LEDGER note: "the twin's register image and serial pin judge compare it with the topology".
11. **GPIOC.OSPEEDR ss**.
    - LEDGER: both 0; TOPO: MAVLINK at 57600.
12. **GPIOC.OTYPER ibm**.
    - LEDGER: all OT = 0. LEDGER judge: OTYPER is not stored, F4300b section 10 pass 3, measured.
13. **GPIOC.PUPDR ss**.
    - LEDGER: all 0.
    - TOPO `adc_inputs`: VBAT_SENSE is PC0, ADC1 INP10. PC0's MODER is not in the ledger's written GPIOC rows (`rcc_gpio.txt`).
14. **GPIOE.AFRH ss** [11,13,14].
    - TOPO: MOTOR2-4 are TIM1_CH2-4 at AF1; LEDGER = 1. F4300 section 0.
    - `aethelgard-twin/src/platform.rs` `emit_observers` (L1037-1071) puts an LED observer on each motor pin.
15. **GPIOE.MODER ss**: as 14, with LEDGER values = 2.
16. **GPIOE.OSPEEDR ss**.
    - LEDGER: 0 (low); TOPO `pin_speeds`: MOTOR2-4 = 0, DShot600.
17. **GPIOE.OTYPER ibm**: as 12, port E.
18. **GPIOE.PUPDR ss** [0-10,12,15].
    - LEDGER: all 0. PUPD11/13/14 are independent-rule (motor), so absent.
    - PE9's history comes from three sources:
      - 2304 FEEDBACK L116: MOTOR1-4 on PE9/11/13/14;
      - SPEC L452: PC9's reason, "moved from MCO1/PA8 when MOTOR1 took PA8 (AET-WO-4300h D3)";
      - 4300h FEEDBACK L9.
19. **I2C2.CR1 ss** [PE].
    - LEDGER note: "the rule's CR1 has PE clear and the enable adds it ...". LEDGER PE = 1, on the bench.
    - TOPO: EXT is 100 kHz on PB10/PB11.
20. **I2C2.CR2 ss**.
    - LEDGER: all 0 except AUTOEND, whose value is null (run time) and so is not on the bench (`bench.txt`). The other six are on the bench.
    - `svd-hal/src/board/ledger.rs` L410-427: CR2 is the transfer's word.
    - DOUBT, "EXT declares no device, so none runs":
      - TOPO `i2c_devices` holds only U3 and U7, both on SENSORS/I2C1.
      - `svd-hal/src/board/firmware.rs` L1038-1077 emits `{bus}.transfer(` only for the device reads, so no I2C2 transfer is generated.
      - I read the emitter, not a generated `main.rs`. If any other path calls I2C2's transfer (a script configuration write, for example), the clause "so none runs" is wrong.
21. **I2C2.ICR ss**.
    - The LEDGER note cites RM0433 Rev 9 section 47.9.8.
    - `svd-hal/src/board.rs` L8853: the Kani assert "ICR cleared more than STOPCF and NACKCF".
    - Not on the bench: ICR is in `WRITE_ONLY` (`ledger.rs` L2781).
    - Same doubt as 20 for "no record shows a clear". Unlike I2C1, no device record exists on I2C2.
22. **RCC.AHB4ENR ss** [GPIOCEN, GPIOEEN].
    - LEDGER note: "no held measurement shows this one registered"; the reason uses that wording.
    - `ledger.rs` L408 GATE_REF: 3302 B2's Renode column reads AHB4ENR 0x3 and APB1LENR 0x00200000.
23. **RCC.APB1LENR ss** [I2C2EN]: same GATE_REF. 0x00200000 is I2C1EN only.
24. **RCC.CFGR ibm** [HRTIMSEL, MCO1, MCO1PRE, MCO2, MCO2PRE, RTCPRE, STOPKERWUCK, STOPWUCK, SWS].
    - LEDGER judges: L226 (SWS read-only); L227-230 and L233-236 are tags. LEDGER SW = 3 (PLL1, observed).
    - TOPO `clock`: hse_hz 16000000, source PLL1.
    - `bench.txt`: SWS is off the bench (read-only) and the other 8 are on it.
    - PC9 (SPEC reserved for MCO2) has no GPIOC MODER row, so it is not set to AF.
    - I did not claim what MCOxPRE = 0 does on silicon; the RM text was not opened.
25. **RCC.CR si** [CSSHSEON, HSERDY, PLL1RDY].
    - LEDGER:
      - HSERDY@reset and PLL1RDY@reset are polled bounded waits; the twin reads each as its ON bit (L185, L197).
      - CSSHSEON is written 1, "set after HSERDY: the model stores the flag (L187) with no detection".
    - SPEC `mcu.hse`: crystal ABM8AIG-16.000MHZ, `startup_max_us` 20000, "architect ruling 2026-11-20".
    - All three are on the bench.
    - The held reason it replaces said "CSSHSEON: written 0 whole" and "polled by the switch (AET-WO-4240e)". Both are false on this HSE board: CSSHSEON = 1, and the waits belong to the HSE bring-up (F4240b L15-18).
    - Per F4240d F3, the reason does not claim that the bench times the waits.
26. **SPI2.CR2 ibm** [TSER].
    - LEDGER judge: L149, a tag.
    - TOPO `spi_devices`: U5 ICM42688P on SPI2, CS IMU_CS.
    - LEDGER: TSIZE is observed, written at run time.
27. **SPI2.IFCR ibm**.
    - LEDGER judges: L258-266. EOTC = 1 is observed. IFCR is write-only, so off the bench.
    - `aethelgard-ir/data/parts.json` L450-457: the ICM42688P identity writes 0xF5, reads 1 byte and expects 0x47.
28. **SPI2.SR ibm** [MODF].
    - LEDGER judge: L238 tag; L188 MASTER reads 1.
    - LEDGER note: "SSM | SSI before SPE is proved by Kani (4230 B1)". LEDGER SPI2 SSM = 1, SSI = 1.
    - MODF@reset = 0 is on the bench.
    - The 3302 sighting of MODF was SPI1 on a NUCLEO, so it is not repeated here.
29. **TIM1.BDTR ibm**.
    - F4240b L128: written as MOE alone. LEDGER: MOE = 1, all other fields 0.
    - F4300a: every BDTR field is a tag. F1211 L370: MOE gating is unmodelled.
    - `AET-WO-2302.FEEDBACK.md` L394, A2-4: "`TIM1.BDTR.MOE` toggled after the window moves no observer: a Renode tag". The reason cites this held finding, not an inference.
30. **TIM1.CCER ibm** [CC1NE-CC3NE].
    - F4300a L31 (the timer1 channel 0 list includes PA7 and PB13, TIM1_CH1N).
    - TOPO: PA7 is SPI_MOSI at AF5 and PB13 is IMU_SCK at AF5.
31. **TIM1.CCER ss**.
    - LEDGER: CC1E-CC4E = 1, all others 0.
    - F4300 section 0: "CCxP is read only for inputs ... the output path never consults polarity".
32. **TIM1.CCMR1 ibm**.
    - LEDGER: OC1PE and OC2PE = 1. The judge: "a tag (4300a section 0); the twin lists OCxPE as unobservable"; "OCxM[3] ... reserved".
33. **TIM1.CCMR1 ss**: LEDGER note: "stored (STM32_Timer.cs L425-445 per aethelgard-twin boot.rs)".
34. **TIM1.CCMR2 ibm**: as 32 for channels 3-4 (MOTOR3 PE13, MOTOR4 PE14).
35. **TIM1.CCMR2 ss**: as 33.
36. **TIM1.CR1 ibm**: LEDGER judge: "outside the bits the model stores (CKD a tag; CR1 stored bits 0-7)".
37. **TIM1.CR1 ss**.
    - LEDGER: ARPE = 1; UDIS, URS, DIR and CMS = 0.
    - TOPO: dshot600, ARR 106.
38. **TIM1.EGR ss**.
    - LEDGER: UG = 1 (observed), the others 0. CEN (CR1) starts the counter, so the reason says only that EGR carries UG alone.
    - Note: "no measurement or held source line shows the model's handling".
    - EGR is in WRITE_ONLY, so off the bench.
39. **USART2.CR1 ibm**. All 13 fields are covered:
    - UESM, WAKE and MME: wake-up and mute;
    - DEDT and DEAT: DE timing;
    - IDLEIE, CMIE, EOBIE, TXFEIE and RXFFIE: interrupts.
    - LEDGER: FIFOEN = 1, M0 = M1 = 0, the rest 0.
    - The judge cites P4.1 and 4300b fix 2 f1. The note: "the serial judge reads M0, M1 and FIFOEN only in the words the access log shows written".
    - TOPO: CRSF on USART2 at 420000; SPEC J6.
40. **USART2.CR1 ss**.
    - F4300b L23: validate refuses a non-8N1 or inverted CRSF. L26: the tolerance check takes the OVER8 = 0 columns.
    - The LEDGER note gives the stored list.
41. **USART2.CR2 ibm**: F4300b L23 and L30: SWAP, RXINV and TXINV are tags; P4.1 measured RXINV and TXINV.
42. **USART2.CR2 ss** [STOP].
    - LEDGER: STOP = 0, the "only stored CR2 field".
    - TOPO: framing stop_bits 1.
43. **USART3.CR1 ibm**: as 39. TOPO: MAVLINK on USART3 at 57600; SPEC J7.
44. **USART3.CR1 ss**: TOPO: BRR 1111, 8N1.
45. **USART3.CR2 ibm**: as 41, with no CRSF framing claim (MAVLINK has no such validate rule that I read).
46. **USART3.CR2 ss**: as 42.

## Checks run

The Rust check is the scratch test `merged_acks_acknowledge_everything`, now deleted. It ran once against the spec as written at 10:12:47Z, and passed.

Six reasons were reworded after it: USART2/3.CR1 ibm, TIM1.EGR, TIM1.BDTR, RCC.AHB4ENR and RCC.APB1LENR. Their fields are unchanged. `mk_acks.py` re-linted them against the same `board::source` rule, the ledger's judge and note texts, the held reasons, and uniqueness.

Re-run the merge check at merge. That run gave:

The check merged `acks.json` into an in-memory copy of `examples/sentinel-03.json`. It replaced 8 held entries (same register and class) and appended 38, for 68 entries in total. Then:

- `Specification::validate` passes, so `board::source` accepts every reason: 1-200 bytes, printable ASCII, no edge spaces.
- `solve` passes, `ledger::stale` is None, and `ledger::acknowledge` returns empty, so every row is acknowledged.
- No reason equals any ledger row's judge or note, and none copies a held reason.
- No two reasons are identical. The longest is 193 bytes.

## Doubts and findings outside the 46 (for the caller; not fixed)

0. **The scratch test passed through the branch history.** WIP commit b52c57d5 added an early 74-line `zz_scratch_acks.rs`, and 01c65708 recorded my deletion.
   - It is no longer tracked at the tip, but it stays in the history between those two commits.
   - That version writes into `/home/forge/.cache` with `.unwrap()`; a squash at merge drops it.

1. **I2C2 has no device** (entries 20 and 21): see the doubt under 20.
2. **TIM1.BDTR** (29) now cites 2302 A2-4 (MOE toggled moves no observer), not an inference.
3. **Held `USART1.CR1` ignored-by-model is now false.** Its reason says "FIFO off", but the ledger writes FIFOEN = 1 on USART1, USART2 and USART3.
4. **Held `GPIOB.OTYPER` is now false.** Its reason says "PB6/PB7 open-drain for I2C1, the rest push-pull", but OT10 and OT11 = 1 (open-drain, I2C2 EXT).
5. **Held `RCC.CR` ignored-by-model should be re-read.** It says the words are written by "the kernel-clock switch (AET-WO-4240e)". On this HSE board the HSE bring-up writes them; the ledger notes say "the HSE bring-up, or the kernel-clock switch". It also says "Bench rows read CR", but most of its fields are read-only written rows that are off the bench. Only HSIKERON, CSIKERON, HSEBYP and D2CKRDY@reset are on it (`rcc_gpio.txt`).
6. **Held `RCC.D1CCIPR` should be re-read.** It says "SPI1's kernel per_ck assumes HSI here", but TOPO gives SPI1 and SPI2 kernel pll1_q.
7. **The held `RCC.CR` silicon-only reason I replaced** cited "Tviburi RCC guard". Tviburi is only a companion-project proposal (`docs/reviews/2026-11-21-grok-tviburi.md`), not a guard in this tree, so I dropped it.
8. **`ledger_tier1.rs` `sentinel_03_s_entries_never_carry_the_ledger_s_judge_text` pins old numbers.** It asserts (280, 36, 44, 7) and 315 fields. The tree now gives 290 unacknowledged rows and 46 entries, 8 of them held. That test needs re-pinning, or restructuring once the acks are merged, since `missing_entries` will then be empty.
9. **The task text said "PE9/11/13/14 motors".** On this board MOTOR1 is PA8 and PE9 is a plain pin (no pull), and the reasons follow the topology.
