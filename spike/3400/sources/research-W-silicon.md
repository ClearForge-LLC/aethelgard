# AET-WO-3400 research: W2-W5 silicon facts (STM32H753ZIT6, LQFP-144)

Read-only research. Sources: RM0433 Rev 9 text (`/home/forge/.cache/aet-4230/b0/rm0433.txt`), DS12117 Rev 10 text
(`/home/forge/.cache/aet-4300d-build/txt/ds12117-rev10.txt`), ST pin XML `STM32H753ZITx.xml`, ES0392 Rev 16 text.
AN4938 rev 7 was NOT consulted: no PDF text extractor exists on this box (no pdftotext/mutool/pypdf/fitz) and the Read tool's PDF path fails
("pdftoppm is not installed"), which I tried. The page-image cache /home/forge/.cache/aet-4230/b0/pages holds only other pages (not RM p.104). Nothing asked needed AN4938.

Labels: [read at source] = quoted from the text. [inferred] = my reasoning from quoted text. [inferred] = arithmetic.
RM page = the "N/3329" footer nearest the text in the extract (page number can be off by one near a page break).
DS12117 has two electrical-characteristics halves in one document: "rev Y" (400 MHz silicon, pp.~105-189) and
"rev V" (480 MHz silicon, pp.~209-301). Where they differ both are given. The ES0392 table columns are
Rev Y,W | Rev X | Rev T,V,4,5,6. Which silicon revision the bench part is has to be read off the package marking.

Table 3 (RM0433 s2.1, p.104, "Bus-master-to-bus-slave interconnect") is decoded in W2.1 below by character column, cross-checked against the prose.
Labels: only [read at source] and [inferred] are used ([inferred] includes arithmetic).

---------------------------------------------------------------------------------------------------------
## W2  logic analyzer: timer-paced DMA from GPIOx_IDR

### W2.1 Which DMA controllers reach the GPIO ports (AHB4, D3) and each SRAM

[read at source] RM0433 s2.1.6 "Bus master peripherals", p.108 (prose), then Table 3 decoded below:
- DMA1/DMA2: "The peripheral bus allows DMA data transfers between two peripherals, between two memories or between a
  peripheral and a memory. Through the system bus matrices, the peripheral bus can access all internal memories except
  ITCM and DTCM, external memories through the Quad-SPI controller and the FMC, and all AHB and APB peripherals."
  (also: "The memory bus ... can access all internal memories except ITCM and DTCM".) GPIOA..K are AHB4 (D3)
  peripherals (RM0433 Table 8 register boundary, p.~140: "0x58020000 - 0x580203FF GPIOA ... AHB4 (D3)").
  => DMA1/DMA2 reach GPIOx_IDR (AHB4) and SRAM4 and AXI SRAM and SRAM1-3, but not ITCM/DTCM.
- BDMA: "The BDMA controller uses a 32-bit bus, connected to the AHB bus matrix in D3 domain ... BDMA transfers are
  limited to the D3 domain resources. It can access the internal SRAM4, backup RAM, and AHB4 and APB4 peripherals
  through the AHB bus matrix in the D3 domain." (also s16.2 p.672: "Access to D3 domain SRAM and AHB/APB peripherals (BDMA)").
  => BDMA reaches GPIO (AHB4) but ONLY SRAM4 (64 KB) and backup SRAM (4 KB) as buffer.
- MDMA: "Through the system bus matrices and the Cortex-M7 AHBS slave bus, the MDMA can access all internal and
  external memories through the Quad-SPI controller and the FMC." It is the only DMA that reaches DTCM/ITCM
  ("The DTCM-RAM ... accessible by Cortex-M7, and by MDMA through AHBS", s2.3 p.137). The prose does not list peripherals
  for MDMA, but Table 3 (decoded below) shows an X for MDMA's AXI port on the AHB4-peripherals row, so MDMA CAN reach GPIO.
  MDMA is not timer-paced: its hardware requests mdma_str0-29 are DMA1/DMA2 stream transfer-complete (dma1_tcif0-7, dma2_tcif0-7),
  LTDC line, JPEG FIFO/EOC, QUADSPI FIFO/TC, DMA2D CLUT/TC/TW and SDMMC1 end-of-data (RM0433 interconnect Table 102 p.601 / Table 106 p.617):
  no TIMx or LPTIM source. [read at source]

[read at source] SRAM accessibility, RM0433 s2.3 "Embedded SRAM", p.136:
- AXI SRAM: "mapped at address 0x2400 0000 and accessible by all system masters except BDMA through D1 domain AXI bus matrix."
- SRAM1/SRAM2: "mapped at address 0x3000 0000 / 0x3002 0000 and accessible by all system masters except BDMA through D2 domain AHB matrix."
- SRAM3: "mapped at address 0x3004 0000 and accessible by all system masters except BDMA ... can be used as buffers to store
  peripheral input/output data for Ethernet and USB."
- SRAM4: "mapped at address 0x3800 0000 and accessible by most of system masters through D3 domain AHB matrix. AHB SRAM4 can be
  used as BDMA buffers ..."
- DTCM: "mapped at the address 0x2000 0000 and accessible by Cortex-M7, and by MDMA through AHBS slave bus".
- Backup SRAM: "mapped at the address 0x3880 0000 and is accessible by most of the system masters through D3 domain's AHB matrix".
- "The system AHB SRAM can be accessed as bytes, half-words (16-bit units) or words (32-bit units), while the system AXI SRAM
  can be accessed as bytes, half-words, words or double-words" and "These memories can be addressed at maximum system clock
  frequency without wait state."

Table 3 decoded [read at source for the X marks; column assignment is [inferred] from character positions]. The extract has 18 master columns:
CM7 AXIM, CM7 AHBP, CM7 ITCM, CM7 DTCM, SDMMC1, MDMA(AXI), MDMA(AHBS), DMA2D, LTDC, DMA1_MEM, DMA1_PERIPH, DMA2_MEM, DMA2_PERIPH, Eth MAC, SDMMC2, USBHS1, USBHS2, BDMA.
I assigned columns by the x-position of each "X" and validated the mapping on the Flash/AXI-SRAM rows (13 X's = AXIM, SDMMC1, MDMA-AXI, DMA2D, LTDC,
DMA1 mem+periph, DMA2 mem+periph, Eth, SDMMC2, USBHS1, USBHS2; BDMA is "-") and on SRAM1/2/3 (11 X's: same minus SDMMC1 and LTDC, BDMA "-"), which match the prose
("accessible by all system masters except BDMA", SDMMC1/LTDC reach AXI SRAM/Flash only). The AHB4 row's last eight X's are printed run together, so those are counted, not placed; the
count (11) is only consistent with every D2 master + BDMA + AXIM + MDMA-AXI. Result:
| Slave | X for |
| AHB4 peripherals (GPIO) | CM7 AXIM, MDMA(AXI), DMA1_MEM, DMA1_PERIPH, DMA2_MEM, DMA2_PERIPH, Eth, SDMMC2, USBHS1, USBHS2, BDMA (11) |
| APB4 peripherals | same 11 |
| AXI SRAM | AXIM, SDMMC1, MDMA(AXI), DMA2D, LTDC, DMA1 mem+periph, DMA2 mem+periph, Eth, SDMMC2, USBHS1, USBHS2 (13; no BDMA) |
| SRAM1 / SRAM2 / SRAM3 | AXIM, MDMA(AXI), DMA2D, DMA1 mem+periph, DMA2 mem+periph, Eth, SDMMC2, USBHS1, USBHS2 (11; no BDMA, no SDMMC1, no LTDC) |
| SRAM4 | AXIM, MDMA(AXI), DMA1 mem+periph, DMA2 mem+periph, SDMMC2, BDMA (8; NOT Eth, NOT USBHS1/2) |
| Backup RAM | AXIM, MDMA(AXI), DMA1/2 mem+periph, Eth, SDMMC2, USBHS1/2, BDMA (11) |
| DTCM / ITCM | only the CM7 TCM masters and MDMA(AHBS): no DMA1/2, no BDMA |
=> DMA1/DMA2 (both ports), MDMA and BDMA each have an X for GPIO (AHB4); the table is consistent with the prose.

[inferred] Cache caveat: AXI SRAM/SRAM1-3 default attributes are "Write-back, write allocate" (RM0433 Table 7, p.131).
Data written by DMA into those regions needs the region set non-cacheable via MPU, or a D-cache invalidate before the CPU reads.
SRAM4 is in the same Table 7 RAM class. (Design implication, not a stated number.)

### W2.2 Timer update/capture -> DMA via DMAMUX1 / DMAMUX2

[read at source] DMAMUX1 serves DMA1/DMA2, DMAMUX2 serves BDMA, RM0433 s17.3.1, p.696:
"The product integrates two instances of DMA request multiplexer: DMAMUX1 for DMA1 and DMA2 (D2 domain); DMAMUX2 for BDMA (D3 domain)".
Features s17.2: "Up to 8-channel DMA request generator; Up to 32 trigger inputs to DMA request generator ... Up to 107 input DMA request lines from peripherals".

[read at source] RM0433 Table 124/125 "DMAMUX1: assignment of multiplexer inputs to resources", pp.697-698. Timers are request inputs:
TIM1_CH1..CH4 = inputs 11-14, TIM1_UP = 15, TIM1_TRIG = 16, TIM1_COM = 17, TIM2_CH1..CH4 = 18-21, TIM2_UP = 22,
TIM3_CH1..CH4 = 23-26, TIM3_UP = 27, TIM3_TRIG = 28, TIM4_CH1..CH3 = 29-31, TIM4_UP = 32, TIM8_CH1..CH4 = 47-50, TIM8_UP = 51, TIM8_TRIG = 52,
TIM8_COM = 53, TIM5_CH1..CH4 = 55-58, TIM5_UP = 59, TIM5_TRIG = 60, TIM6_UP = 69, TIM7_UP = 70, TIM17_UP = 112.
Timer side enable bits: TIMx_DIER "Bit 8 UDE: Update DMA request enable; Bit 9 CC1DE: Capture/Compare 1 DMA request enable" (TIM1/8 p.1595; TIM2-5 p.1684).
Also: DMAMUX1 trigger inputs include "lptim1_out, lptim2_out, lptim3_out, extit0, TIM12_TRGO" (Table 126, p.699) which can drive the
DMAMUX request generator ("The DMAMUX may also be used as a DMA request generator from programmable events on its input trigger signals", s17.1 p.695).

[read at source] DMAMUX2 (BDMA) Table 128, p.700: inputs are dmamux2_req_gen0-7, lpuart1_rx/tx_dma, spi6_rx/tx_dma, i2c4_rx/tx_dma, sai4_a/b_dma, adc3_dma. There is NO
timer request line on DMAMUX2. Table 129 (DMAMUX2 trigger inputs) includes "lptim2_wkup (9), lptim2_out (10), lptim3_wkup (11),
lptim3_out (12), Lptim4_ait (13), Lptim5_ait (14), Syscfg_exti0 (20), Syscfg_exti2 (21)", so a BDMA channel can only be
timer-paced via the DMAMUX2 request generator triggered by an LPTIM output (or an EXTI line), not by TIMx update/capture.
[inferred] Consequence: DMA1/DMA2 + TIMx_UP (or TIMx_CHy compare/capture) is the direct path; BDMA needs
LPTIM2/LPTIM3 out -> DMAMUX2 request generator -> BDMA channel.

[inferred] To use a timer update as the sample clock: TIMx_DIER.UDE=1, DMAMUX1 channel input = TIMx_UP, DMA stream p2m, PA=&GPIOx->IDR.
Equally a compare event (CCxDE with CCx as output compare) paces at the same timer rate.

### W2.3 Byte / half-word access to GPIOx_IDR

[read at source] RM0433 s11.3.1 (p.531): "Each I/O port bit is freely programmable, however the I/O port registers have to be accessed
as 32-bit words, half-words or bytes." IDR layout (s11.4.5, p.543): "Bits 31:16 Reserved ... Bits 15:0 IDR[15:0]: Port x input data I/O pin y.
These bits are read-only." So the useful data is the low half-word.
[inferred] 8 channels = byte read at IDR+0 (pins 0-7, PSIZE=byte) or IDR+1 (pins 8-15); 16 channels = half-word read at IDR+0 (PSIZE=half-word).
(Little-endian byte addressing assumed; DMA PSIZE byte/half-word are supported: "Independent source and destination transfer width (byte, half-word, word)", RM0433 s15.2 p.638 and s16.2 p.672.)
Sampling granularity: [read] "The input data register (GPIOx_IDR) captures the data present on the I/O pin at every AHB clock cycle." (s11.3.1, p.534)

### W2.4 Throughput / clock ceilings

[read at source, absence] I found NO DMA transfers-per-second, cycles-per-transfer or D2->D3 bridge latency figure. What I searched: RM0433 lines 24700-27200
(DMA/BDMA/DMAMUX chapters 15-17) for throughput|bandwidth|latency|AHB/APB clock cycles (only the generic "optimize the bandwidth" sentence, s15.1 p.638),
and all of DS12117 for throughput|bandwidth|latency|transfers per second|cycles per transfer|wait state (hits: op-amp bandwidth, CPU/TCM latency prose, flash/FMC wait states: nothing about DMA or bus rate). Only ceilings:
- DS12117 Table 111 (rev V, p.210): "fHCLK AHB clock frequency: VOS3 100 / VOS2 150 / VOS1 200 / VOS0 240 MHz (5)"; "fPCLK APB clock: VOS3 50(6) / VOS2 75 / VOS1 100 / VOS0 120 MHz"; fCPU 200/300/400/480.
- DS12117 block diagram (p.23): "AHB4 (200MHz)" and "APB4 100 MHz (max)"; AHB1/AHB2/AXI-AHB34 also drawn "(200MHz)".
- RM0433 Figure 49 "Core and bus clock generation" (p.351): "240 MHz Max" at the HPRE output feeding rcc_hclk3/hclk[2:1]/hclk4; D3PPRE divides rcc_hclk4 to rcc_pclk4.
  [inferred] AHB4 = rcc_hclk4 = HCLK, so the ceiling is HCLK 240 MHz (rev V at VOS0) or 200 MHz (VOS1 / rev Y silicon, and the DS diagram label).
- Timer kernel clock: DS12117 Table 190 (rev V, p.297) footnote: "The maximum timer frequency on APB1 or APB2 is up to 240 MHz, by setting the TIMPRE bit in the RCC_CFGR register ...";
  "tres(TIM) timer resolution time: 1 tTIMxCLK at fTIMxCLK = 240 MHz"; fEXT max fTIMxCLK/2.
[inferred] Hard upper bound for a timer-paced IDR sampler is therefore one sample per AHB clock (IDR is refreshed every AHB clock), i.e. <= HCLK; the realised rate
is set by the D2->D3 AHB path and bus-matrix arbitration which no document quantifies. A sample rate must be measured on the bench, not derived.
(RM0433 s2.1.1: AHB matrices use "a round-robin algorithm"; BDMA s16.4.1: "DMA requests may stop the CPU access to the system bus for a number of bus cycles, when CPU and BDMA target the same destination".)

### W2.5 SRAM sizes, addresses, buffer depth

[read at source] RM0433 Table 7 (p.131) memory map:
| Region | Address range | Size (from address range) | Reachable by DMA1/2 | BDMA |
| AXI SRAM | 0x24000000-0x2407FFFF | 512 KiB = 524,288 B | yes | no |
| SRAM1 | 0x30000000-0x3001FFFF | 128 KiB | yes | no |
| SRAM2 | 0x30020000-0x3003FFFF | 128 KiB | yes | no |
| SRAM3 | 0x30040000-0x30047FFF | 32 KiB | yes | no |
| SRAM4 | 0x38000000-0x3800FFFF | 64 KiB | yes (s2.1.6: "all internal memories except ITCM and DTCM") | yes |
| Backup SRAM | 0x38800000-0x38800FFF | 4 KiB | per s2.3 "most of the system masters" | yes |
| DTCM | 0x20000000-0x2001FFFF | 128 KiB | NO (MDMA/CPU only) | no |
| ITCM | 0x00000000-0x0000FFFF | 64 KiB | NO | no |
RM s2.3 p.136 states totals: "Up to 864 Kbytes of System SRAM, 128Kbytes of data TCM RAM, 64Kbytes of instruction TCM RAM, 4Kbytes of backup SRAM"
(512+128+128+32+64 = 864 [inferred]; matches the pin XML "<Ram>864</Ram>").

HEADLINE (read this first): the unattended single-stream depth is 65,535 samples, not the SRAM size. NDT is a 16-bit counter. Region-size depths need double-buffer mode
with software re-pointing at each TCIF. BDMA also has double-buffer mode, but its memory is only SRAM4 (64 KiB) + backup SRAM (4 KiB).

[read at source] Counter limits: DMA1/DMA2 "DMA flow controller: the number of data items to be transferred is software-programmable from 1 to 65535" (s15.2 p.639);
DMA_SxNDTR "NDT[15:0]: Number of data items to transfer (0 up to 65535)". BDMA "Programmable number of data to be transferred: 0 to 2^16 -1" (s16.2 p.672).
Circular mode (s15.3.10, p.647): "the number of data items to be transferred is automatically reloaded with the initial value programmed during the stream configuration phase":
the circular window is NDT items long; it does not cover a bigger region.
DMA1/DMA2 double-buffer (s15.3.11, p.647-648): "This mode is available for all the DMA1 and DMA2 streams." "A double-buffer stream works as a regular (single buffer) stream
with the difference that it has two memory pointers ... at each end of transaction, the memory pointers are swapped." "In double-buffer mode, it is possible to update the base address
for the AHB memory port on-the-fly (DMA_SxM0AR or DMA_SxM1AR) when the stream is enabled ... When the CT bit is 0 ... the DMA_SxM1AR register can be written. Attempting to write to this register while CT = 1
sets an error flag (TEIF) and the stream is automatically disabled ... it is advised to change the base address as soon as the TCIF flag is asserted".
BDMA double-buffer (s16.4.x, p.678): "The BDMA channels can operate in double-buffer mode. ... the BDMA controller toggles between two memory address pointers at the end of each BDMA transfer"; Table 118 "Number of channels (double-buffer): 8"; configured by DBM and CIRC in BDMA_CCRx with BDMA_CM1ARx as the second pointer.
[inferred] Because each half is at most NDT = 65,535 items, a double-buffer ring covers 2 x NDT items without re-pointing, and longer rings need the CPU to rewrite the idle pointer (M0AR/M1AR, or BDMA_CM0ARx/CM1ARx) on every TCIF.

[inferred] Depth in samples (1 sample = 1 B for 8 ch, 2 B for 16 ch):
| Case | 8 ch (1 B) | 16 ch (2 B) |
| DMA1/2, one stream, circular or normal, NDT max 65,535 (no CPU in the loop) | 65,535 | 65,535 |
| DMA1/2 double-buffer, two 65,535-item halves, no re-pointing | 131,070 | 131,070 |
| DMA1/2 double-buffer with the CPU re-pointing at each TCIF over the 512 KiB AXI SRAM | 524,288 | 262,144 |
| same over SRAM1+2+3 (contiguous 0x30000000-0x30047FFF = 288 KiB = 294,912 B) | 294,912 | 147,456 |
| BDMA, one channel, NDT max 65,535 (SRAM4 = 65,536 B) | 65,535 (one byte short of SRAM4) | 32,768 (2 B x 32,768 = 64 KiB = all of SRAM4) |
| BDMA double-buffer / any BDMA ring | bounded by SRAM4 64 KiB (+4 KiB backup SRAM, separate window): 65,536 (+4,096) | 32,768 (+2,048) |
Region sizes [read at source] (RM0433 Table 7, p.131 memory map; the sizes are from the address ranges and match the s2.3 total "Up to 864 Kbytes of System SRAM"):
| Region | Address range | Size | Reachable by DMA1/2 | BDMA |
| AXI SRAM | 0x24000000-0x2407FFFF | 512 KiB = 524,288 B | yes | no |
| SRAM1 | 0x30000000-0x3001FFFF | 128 KiB | yes | no |
| SRAM2 | 0x30020000-0x3003FFFF | 128 KiB | yes | no |
| SRAM3 | 0x30040000-0x30047FFF | 32 KiB | yes | no |
| SRAM4 | 0x38000000-0x3800FFFF | 64 KiB | yes (Table 3 X) | yes |
| Backup SRAM | 0x38800000-0x38800FFF | 4 KiB | yes (Table 3 X) | yes |
| DTCM | 0x20000000-0x2001FFFF | 128 KiB | NO | no |
| ITCM | 0x00000000-0x0000FFFF | 64 KiB | NO | no |
(512+128+128+32+64 = 864 KiB, matching the pin XML "<Ram>864</Ram>".)
DTCM (131,072 / 65,536 samples) is reachable only by the CPU and MDMA (Table 3), so not by a timer-paced DMA. A DMA-written buffer in cacheable AXI SRAM/SRAM1-3 needs the MPU or cache maintenance (RM0433 Table 7 default "Write-back, write allocate").
[inferred] The CPU re-point deadline is one NDT block: 65,535 x the sample period.

### W2.6 ES0392 Rev 16 errata (DMA / BDMA / DMAMUX / GPIO read)
[read at source] Applicable entries (ES0392 summary Table, status cols Rev Y,W | X | T,V,4,5,6; A = workaround, N = none, P = partial, - = n/a):
- 2.2.10 "Reading from AXI SRAM may lead to data read corruption" (p.13-14): A | - | -  (Rev Y/W only). "The read data may be corrupted when ... Several read transactions are performed to the AXI SRAM, and A controller device delays its data acceptance while a new transfer is requested."
  Workaround: "Set the READ_ISS_OVERRIDE bit in the AXI_TARG7_FN_MOD register." Applies to a capture buffer in AXI SRAM on rev Y/W silicon.
- 2.4.1 BDMA "disable failure and error flag omission upon simultaneous transfer error and global flag clear" (p.20): A|A|A. "Do not clear GIFx flags when the channel is active. Instead, use HTIFx, TCIFx, and TEIFx".
- 2.5.1 SOFx not asserted when writing DMAMUX_CFR (p.20): N|N|N, matters only for synchronization mode with concurrent channels.
- 2.5.2 "OFx not asserted for trigger event coinciding with last DMAMUX request" (p.21): N|N|N. Applies to DMAMUX request generator with GNBREQ>1; "Make the trigger period longer than the duration required for serving the programmed number of DMA requests". Relevant only to the BDMA (LPTIM-triggered generator) path.
- 2.5.3 "OFx not asserted when writing into DMAMUX_RGCFR" (p.21): N|N|N, concurrent request-generator channels only.
- 2.5.4 "Wrong input DMA request routed upon specific DMAMUX_CxCR register write coinciding with synchronization event" (p.21): A|A|A; workaround: SPOL=00 whenever SE=0.
- 2.5.5 DMAMUX_RGCFR is write-only (documentation).
- 2.2.28 HASH input data may be corrupted when DMA1/2 are used (HASH-only; not relevant).
- 2.20.4 USART and 2.21.1 LPUART "DMA stream locked when transferring data to/from USART/LPUART": rev X, T/V: A; workaround DMA_SxCR bit 20 on the stream serving the USART (relevant if USART RX sniffing uses DMA1/2).
- 2.10.2 DAC DMA underrun flag (DAC only).
- No erratum about GPIOx_IDR reads over DMA, AHB4/D3 reach, or TIMx_UP/CCx DMA requests. 2.2.19 (GPIO assigned to DAC output) is DAC-only. 2.16.2 TIM "Consecutive compare event missed in specific conditions" (N|N|N), body read: a second compare match in two consecutive counter cycles is lost when CCR changes (e.g. edge-aligned, CNT=CCR=ARR then CNT=CCR=0). It only bites a compare-paced sampler that changes CCR; a fixed-period UP- or fixed-CCR-paced sampler is not affected [inferred].
- 2.2.26: Pxy_C pins limited to 1 mA when the analog switch is closed (relevant to PC2_C/PC3_C if used as capture inputs/ULPI).

---------------------------------------------------------------------------------------------------------
## W3  host-link pins on LQFP-144 (from STM32H753ZITx.xml)

### W3.1 Pin lists (every pin carrying each signal; XML signal names). No signal has an alternate pin unless listed.
USB OTG FS (internal FS PHY, OTG_HS2): USB_OTG_FS_DM = PA11 (pos 103), USB_OTG_FS_DP = PA12 (pos 104). Optional: USB_OTG_FS_VBUS PA9, USB_OTG_FS_ID PA10, USB_OTG_FS_SOF PA8.
USB OTG HS internal FS PHY (OTG_HS1 w/o ULPI): USB_OTG_HS_DM = PB14 (pos 75), USB_OTG_HS_DP = PB15 (pos 76); USB_OTG_HS_ID PB12, USB_OTG_HS_VBUS PB13.
USB OTG HS ULPI (12 signals, XML shows exactly one pin each):
- ULPI_CK = PA5 (41)   - ULPI_DIR = PC2_C (28)   - ULPI_NXT = PC3_C (29)   - ULPI_STP = PC0 (26)
- ULPI_D0 = PA3 (37)   - D1 = PB0 (46)   - D2 = PB1 (47)   - D3 = PB10 (69)   - D4 = PB11 (70)   - D5 = PB12 (73)   - D6 = PB13 (74)   - D7 = PB5 (135)
Ethernet (RMII subset; the XML names are ETH_*, the RMII role in parentheses):
- ETH_REF_CLK (RMII REF_CLK) = PA1 (35)  [shared with ETH_RX_CLK]
- ETH_CRS_DV (RMII CRS_DV) = PA7 (43)  [XML also lists ETH_RX_DV on PA7]
- ETH_RXD0 = PC4 (44);  ETH_RXD1 = PC5 (45)
- ETH_TX_EN = PB11 (70) or PG11 (126)
- ETH_TXD0 = PB12 (73) or PG13 (128)
- ETH_TXD1 = PB13 (74) or PG12 (127) or PG14 (129)
- ETH_MDIO = PA2 (36);  ETH_MDC = PC1 (27)
(MII-only extras in XML: TXD2 PC2_C, TXD3 PE2/PB8, RXD2 PB0, RXD3 PB1, TX_CLK PC3_C, RX_CLK PA1, COL PA3, CRS PA0, RX_ER PB10, TX_ER PB2/PA9. Not needed for RMII.)
Cross-check against DS12117 Table 8 pin text (AF names OTG_HS_ULPI_*, ETH_MDIO, OTG_FS_DM, OTG_HS_DM): consistent.

### W3.2 Collisions with the already-assigned set (PA0 PA2 PA3 PA5 PA6 PA7 PA9 PA10 PB6 PB7 PB10 PB11 PC10 PC11 PE9 PE11 PE13 PE14 PF3 PA13 PA14)
| Signal | Pin | Collides? |
| FS DM / DP | PA11 / PA12 | NO |
| FS VBUS / ID / SOF | PA9 / PA10 / PA8 | PA9, PA10 collide (optional signals only) |
| HS-FS-PHY DM / DP | PB14 / PB15 | NO |
| ULPI_CK | PA5 | YES |
| ULPI_DIR | PC2_C | no |
| ULPI_NXT | PC3_C | no |
| ULPI_STP | PC0 | no |
| ULPI_D0 | PA3 | YES |
| ULPI_D1, D2 | PB0, PB1 | no |
| ULPI_D3 | PB10 | YES |
| ULPI_D4 | PB11 | YES |
| ULPI_D5, D6, D7 | PB12, PB13, PB5 | no |
| RMII REF_CLK | PA1 | no |
| RMII CRS_DV | PA7 | YES (no alternate pin) |
| RMII RXD0 / RXD1 | PC4 / PC5 | no |
| RMII TX_EN | PB11 | YES; alternate PG11 is free |
| RMII TXD0 | PB12 | no (alt PG13) |
| RMII TXD1 | PB13 | no (alt PG12, PG14) |
| ETH_MDIO | PA2 | YES (no alternate pin) |
| ETH_MDC | PC1 | no |
[read at source] (pin XML) for the pin-to-signal map; [inferred] for the collision verdicts (set comparison).

### W3.3 Collision-free choices
- ULPI: NONE. Every ULPI signal has a single pin on this package (XML), and four of the twelve land on assigned pins (PA5 CK, PA3 D0, PB10 D3, PB11 D4). ULPI/HS-480 is impossible with the assigned set unless those four assignments move. [inferred]
- RMII: NONE as assigned, but only because of two single-pin signals: CRS_DV (PA7) and MDIO (PA2). TX_EN/TXD0/TXD1 can be collision-free as PG11/PG13/PG12 (or PG14); REF_CLK PA1, RXD0 PC4, RXD1 PC5, MDC PC1 are free. If PA7 (and PA2 for PHY management; a strapped PHY can run without MDIO [inferred, not in the sources]) cannot be released, RMII does not fit. RMII CRS_DV is required by the RMII block (RM0433 RMII s: "CRS_DV: carrier Sense (CRS) and RX_Data Valid (RX_DV) multiplexed on alternate clock cycles", p.2857).
- Collision-free host links that DO exist: USB FS device on PA11/PA12 (OTG_HS2 internal FS PHY), or on PB14/PB15 (OTG_HS1 internal FS PHY, FS speed only). Both 12 Mbit/s.
- Errata 2.2.27 [read] (ES0392 p.17): "Ethernet MII mode is not available on packages where only the PC2_C/PC3_C pins are available. Workaround: Instead, use the RMII Ethernet mode." LQFP-144 has PC2_C/PC3_C (XML pos 28/29), so RMII is the only Ethernet option here. SYSCFG_PMCR EPIS[2:0] selects it: "100: RMII" (RM0433 p.549).
- PC2_C/PC3_C: DS Table 8 footnote 4: "There is a direct path between Pxy_C and Pxy pins/balls, through an analog switch. Pxy alternate functions are available on Pxy_C when the analog switch is closed." RM0433 12.3.1 (p.549): "PXnSO reset value is 0 when the corresponding PXn_C pin is available on the package but PXn is not" (0 = switch closed), so on LQFP-144 the switch resets closed. ES0392 2.2.26: "current sunk or sourced by Pxy_C pins is limited to 1 mA when the analog switch ... is closed".

### W3.4 Maximum rates; internal HS PHY?
[read at source] DS12117 s3.41 (p.52): "The devices embed two USB OTG high-speed (up to 480 Mbit/s) device/host/OTG peripheral. OTG_HS1 supports both full-speed and high-speed operations, while OTG_HS2 supports only full-speed operations. They both integrate the transceivers for full-speed operation (12 Mbit/s) ... OTG_HS1 features a UTMI low-pin interface (ULPI) for high-speed operation (480 Mbit/s). When using the USB OTG_HS1 in HS mode, an external PHY device connected to the ULPI is required."  "Internal FS OTG PHY support"; "The OTG PHY is connected to the microcontroller ULPI port through 12 signals. It can be clocked using the 60 MHz output."
=> CONFIRMED: no internal HS PHY. HS (480 Mbit/s) needs an external ULPI PHY. RM0433 Table 517 (p.2590): "ULPI available to primary IOs via, muxing: OTG_HS1 X / OTG_HS2 -; Integrated PHY: FS / FS". RM Table 516 (p.2588): HS (480 Mb/s), FS (12 Mb/s), LS. 
Pin mapping of the two integrated FS PHYs [read] RM Fig 750/751 (p.2591): OTG_HS1 -> OTG_HS_DP/DM, OTG_HS2 -> OTG_FS_DP/DM. [inferred] so PB14/PB15 are OTG_HS1's FS-PHY pins and PA11/PA12 are OTG_HS2's. DS: "Combined Rx and Tx FIFO size of 4 Kbytes", "9 bidirectional endpoints", "16 host channels".
Ethernet: DS12117 s3.42 (p.52-53): "Supports 10 and 100 Mbit/s rates"; "The PHY is connected to the device MII port using 17 signals for MII or 9 signals for RMII"; "Internal FIFOs ... both 2 Kbytes". RM0433 RMII: "Supports 10 Mbit/s and 100 Mbit/s operating rates. It does not support the 1000 Mbit/s operation." and "RMII_REF_CLK: continuous 50 MHz reference clock input" (p.2857).
ES0392 OTG/ETH items: 2.2.13 USB OTG_FS PHY "avoid to load more than 5 mA on OTG_FS_DP/DM pins" (rev Y/W, N, p.14); 2.25.x OTG_HS errata (2.25.1 PHY pull-up drift P on Y/W; 2.25.3 host hang with FS device through hub to a LS device N|N|N; 2.25.8 false chirp-K A|A|A).

---------------------------------------------------------------------------------------------------------
## W4  protocol capture

(a) SPI slave receive-only (simplex receiver). [read] RM0433 s50.4.5 "Simplex communications" (p.2161): "Receive-only mode: COMM[1:0]=10 ... In slave configuration, the MISO output is disabled and the pin can be used as a GPIO. The slave continues to receive data from the MOSI pin while its slave select signal is active (see Section 50.4.7: Slave select pin (NSS) management)." SPI_CFG2 COMM[1:0] (p.2222): "10: simplex receiver". IOSWP "swap functionality of MISO and MOSI pins" (SPI_CFG2 bit 5, may only be modified when the SPI is disabled). Slave max clock: see W5.
[inferred] One SPI instance sniffs one data line; to sniff MOSI and MISO of a bus use two SPI instances in simplex-receiver slave mode, one with IOSWP=1 (so its "MOSI" input pin is wired to the bus MISO), both sharing SCK and NSS. The slave MISO output is disabled in this mode, so it does not drive the bus. NSS can be software-managed (s50.4.7), but the slave still needs SCK from the master.

(b) USART receiver-only. [read] RM0433 s48.2 main features (p.2017): "Separate enable bits for transmitter and receiver"; USART_CR1 "Bit 2 RE: Receiver enable. 0: Receiver is disabled; 1: Receiver is enabled and begins searching for a start bit" (p.2070). Character reception procedure (p.2028): "Set the RE bit USART_CR1. This enables the receiver which begins searching for a start bit". "Select DMA enable (DMAR) in USART_CR3 if multibuffer communication is to take place". Start bit detection (s48.5.6, p.2027): "This sequence is: 1 1 1 0 X 0 X 0X 0X 0 X 0X 0." Also "Auto baud rate detection", "Swappable Tx/Rx pin configuration", "Continuous communications using DMA".
[inferred] TE=0/RE=1 gives a pure receiver that never drives the line. Errata 2.20.3 "Data corruption due to noisy receive line" (A|A|A) and 2.20.4 (DMA stream lock with USART, workaround DMA_SxCR bit 20) apply.

(c) Timer input capture with DMA. [read] RM0433 TIM2-5 s39.3.5 "Input capture mode" (p.1652): "In Input capture mode, the Capture/Compare Registers (TIMx_CCRx) are used to latch the value of the counter after a transition detected by the corresponding ICx signal. When a capture occurs, the corresponding CCXIF flag (TIMx_SR register) is set and an interrupt or a DMA request can be sent if they are enabled. If a capture occurs while the CCxIF flag was already high, then the over-capture flag CCxOF (TIMx_SR register) is set." Step 7: "enable ... the DMA request by setting the CC1DE bit in the TIMx_DIER register." "A DMA request is generated depending on the CC1DE bit." DMAMUX1 routes TIMx_CHy requests to DMA1/DMA2 (Table 124, W2.2 above).
Both edges: TIMx_CCER (p.1695) "CC1NP=1, CC1P=1: non-inverted/both edges. The circuit is sensitive to both TIxFP1 rising and falling edges (capture or trigger operations ...)". Filter: TIMx_CCMR1 IC1F (p.1688): "0000: No filter, sampling is done at fDTS ... 0011: fSAMPLING=fCK_INT, N=8 ... 1100: fSAMPLING=fDTS/16, N=8" (an event counter of N consecutive samples validates a transition). PWM input mode (s39.3.6, p.1653) measures period and duty on TI1/TI2 only: "The PWM input mode can be used only with the TIMx_CH1/TIMx_CH2 signals".

(d) I2C monitor / sniff mode. [read] RM0433 s47.2 main features (p.1951) lists: "Target and controller modes; Multicontroller capability; ... Multiple 7-bit target addresses (2 addresses, 1 with configurable mask); All 7-bit-addresses acknowledge mode; General call; ... Clock stretching (optional)". I grepped RM0433 lines 76297-78770 (s47.2 through the end of the I2C chapter, up to the start of s48 USART) case-insensitively for monitor|listen|sniff|promiscuous|snoop|passive: no hits, and the main-features list above has no such mode. So NO monitor/sniff/listen-only mode exists. [read at source: absence]
Address match behaviour [read] s47.4.8 (p.1962): "When the I2C peripheral is selected by one of its enabled addresses, the ADDR interrupt status flag is set"; "If OA2MSK[2:0] = 7, all received 7-bit addresses are acknowledged (except reserved addresses)." and "By default, the target uses its clock stretching capability, which means that it stretches the SCL signal at low level when required ... The stretch is released when the software clears the ADDR flag by setting the ADDRCF bit." (p.1963).
Why it cannot observe without participating [inferred from the quoted behaviour]: the peripheral only passes data after an address match, and a match is signalled by the peripheral driving SDA low for the ACK slot and (unless NOSTRETCH=1) holding SCL low while ADDR is set and while the RX register is unread. Those are bus actions, so an I2C instance in the address-match path cannot be a passive tap. NACKing instead (SBC + per-byte ACK control, s47.4.8 "Target byte control mode") would also drive the bus and aborts the controller's transfer. A passive sniffer must therefore read SCL/SDA as plain inputs (EXTI/timer-capture on SCL and SDA edges, or the W2 IDR sampler) and decode in software.

---------------------------------------------------------------------------------------------------------
## W5  stimulus

I2C target with clock stretching / NOSTRETCH [read] RM0433 s47.4.8 (pp.1962-1964):
- "By default, the target uses its clock stretching capability, which means that it stretches the SCL signal at low level when required, to perform software actions. If the controller does not support clock stretching, I2C must be configured with NOSTRETCH = 1 in the I2C_CR1 register."
- "As long as the NOSTRETCH bit ... is zero (default), the I2C peripheral operating as an I2C-bus target stretches the SCL signal in the following situations: The ADDR flag is set and the received address matches ...; In transmission, the previous data transmission is completed and no new data is written in I2C_TXDR ...; In reception, the I2C_RXDR register is not read yet ...".
- "As long as the NOSTRETCH bit ... is set ... The SCL clock is not stretched while the ADDR flag is set. In transmission, the data must be written in the I2C_TXDR register before the first SCL pulse corresponding to its transfer occurs. If not, an underrun occurs, the OVR flag is set ... In reception, the data must be read from the I2C_RXDR register before the ninth SCL pulse (ACK pulse) of the next data byte occurs."
- "The target byte control mode is not compatible with NOSTRETCH mode. Setting SBC when NOSTRETCH = 1 is not allowed."
- Speeds: s47.2: "Standard-mode (up to 100 kHz), Fast-mode (up to 400 kHz), Fast-mode Plus (up to 1 MHz)". ES0392 I2C items: 2.19.8 "OVR flag not set in underrun condition" N|N|N (so NOSTRETCH underrun is not reported on all silicon); 2.19.9 "Transmission stalled after first byte transfer" A; 2.19.3 wrong sampling when tSU;DAT < one I2C kernel clock period (P).

SPI slave max clock [read] DS12117 SPI dynamic characteristics:
- Rev V half, Table 194 (p.301): "fSCK SPI clock frequency: Master mode 1.62<VDD<3.6 V SPI1,2,3: 80 MHz; Master 2.7<VDD<3.6 V SPI1,2,3: 100; Master 1.62<VDD<3.6 V SPI4,5,6: 50; Slave receiver mode 1.62<VDD<3.6 V: 100; Slave mode transmitter/full duplex 2.7<VDD<3.6 V: 31; Slave mode transmitter/full duplex 1.62<VDD<3.6 V: 29" (MHz). Slave data output valid tv(SO) 12.5 typ / 16 max (2.7-3.6 V), 17 max (1.62-3.6 V) ns; ta(SO) 9/13/27 ns.
- Rev Y half, Table 95 (p.189): "Slave receiver mode 1.62-3.6 V SPI1,2,3: 150; SPI4,5,6: 100; Slave mode transmitter/full duplex 2.7-3.6 V: 31; 1.62-3.6 V: 25" (MHz).
Takeaway: as a slave RECEIVER (sniffer) up to 100 MHz (rev V; 150 MHz SPI1-3 rev Y); as a slave TRANSMITTER / full-duplex stimulus only 29-31 MHz (rev V) or 25-31 MHz (rev Y). Table text is a flattened extraction; the order of rows above is how it reads, the numbers are quoted as they appear.

Timer input capture resolution for DShot600 (bit period 1.67 us) [read at source / inferred]
- DS12117 Table 190 (rev V, p.297): "tres(TIM) timer resolution time: 1 tTIMxCLK (min) at fTIMxCLK = 240 MHz (AHB/APBx prescaler 1, 2 or 4)"; 1 tTIMxCLK at fTIMxCLK = 120 MHz (prescaler > 4). "ResTIM 16/32 bit"; "tMAX_COUNT with 32-bit counter 65536 x 65536 tTIMxCLK".
  Rev Y Table 92 (p.187): same with "fTIMxCLK = 240 MHz" and "140 MHz" for prescaler > 4 as printed.
- [inferred] 1/240 MHz = 4.17 ns per tick => 1.67 us = ~400 ticks per bit at 240 MHz (~200 at 120 MHz). Capturing both edges gives pulse widths to +/-1 tick (+ filter delay if ICxF is used: filter of N samples at fSAMPLING adds latency, step 3 of 39.3.5). DShot high-time ratios and the signal levels are NOT in the local sources and are not asserted here.
- [read] Minimum pulse: no minimum-pulse-width spec is given for timer input captures other than the I/O input characteristics; the TIMx input filter (IC1F, p.1688) rejects glitches shorter than N samples. Counting capture events through DMA is one request per edge: DMA path limits are in W2.4 (no documented figure).

---------------------------------------------------------------------------------------------------------
## Summary table
| row | answer | label | citation |
| W2.1 DMA reach to GPIO | DMA1/DMA2 (peripheral port: "all AHB and APB peripherals"), BDMA ("AHB4 and APB4 peripherals") and MDMA (Table 3 X on the AHB4 row for its AXI port) all reach GPIO/AHB4 | read (prose, X marks), inferred (column assignment) | RM0433 s2.1.6 p.108; Table 3 p.104; Table 8 GPIO = AHB4 |
| W2.1 SRAM reach | AXI SRAM, SRAM1-3: DMA1/2 yes, BDMA no; SRAM4: DMA1/2, BDMA, MDMA, SDMMC2 yes, Eth/USB no; DTCM: CPU + MDMA only | read (prose + Table 3 X marks), inferred (column map) | RM0433 s2.3 p.136; Table 3 p.104 |
| W2.2 timer to DMA | TIM1/2/3/4/5/8 CHx, UP, TRIG, COM and TIM6/7 UP feed DMAMUX1 (DMA1/2); DMAMUX2 (BDMA) has NO timer inputs, only LPTIM out / EXTI triggers via request generator; MDMA has no timer request either | read | RM0433 Tables 124-126, 128-129 pp.697-700; TIM DIER UDE/CCxDE |
| W2.3 IDR access | "accessed as 32-bit words, half-words or bytes"; IDR[15:0] only | read | RM0433 s11.3.1 p.531; s11.4.5 p.543 |
| W2.4 sample rate | NO throughput figure anywhere; ceilings only: IDR sampled every AHB clock; HCLK max 240 MHz (rev V, VOS0), 200 MHz otherwise; AHB4 labelled 200 MHz and APB4 100 MHz in DS diagram; PCLK 120 MHz; timer clock 240 MHz | read | RM0433 s11.3.1 p.534, Fig 49 p.351; DS Table 111 p.210, p.23, Table 190 p.297 |
| W2.5 unattended depth | NDT is 16-bit: 65,535 samples per DMA1/2 stream (8 or 16 ch); BDMA 65,535 at 8 ch, 32,768 at 16 ch (SRAM4 64 KiB); circular mode wraps over NDT items only | read (NDT limit, circular text), inferred (arithmetic) | RM0433 s15.2 p.639, s15.3.10 p.647, s16.2 p.672 |
| W2.5 region depth | DMA1/2 double-buffer + CPU re-pointing M0AR/M1AR at each TCIF: AXI SRAM 512 KiB = 524,288 / 262,144 samples (8/16 ch); SRAM1-3 288 KiB = 294,912 / 147,456. BDMA double-buffer exists but SRAM4 caps it at 65,536 / 32,768 (+4 KiB backup SRAM) | read (double-buffer text, Table 7), inferred (arithmetic) | RM0433 s15.3.11 p.647-648, s16.4 p.678, Table 7 p.131 |
| W2.6 errata | 2.2.10 AXI SRAM read corruption (rev Y/W, workaround AXI_TARG7_FN_MOD.READ_ISS_OVERRIDE); 2.4.1 BDMA GIFx clear; 2.5.1-2.5.4 DMAMUX (2.5.2 request-generator OFx for GNBREQ>1); 2.20.4/2.21.1 USART/LPUART DMA lock; nothing on IDR/GPIO reads | read | ES0392 Rev 16 pp.13,20,21 |
| W3 FS pins | OTG_FS DM/DP = PA11/PA12 (free); OTG_HS FS-PHY DM/DP = PB14/PB15 (free) | read (XML) | STM32H753ZITx.xml |
| W3 ULPI pins | CK PA5, DIR PC2_C, NXT PC3_C, STP PC0, D0 PA3, D1 PB0, D2 PB1, D3 PB10, D4 PB11, D5 PB12, D6 PB13, D7 PB5; no alternates | read (XML) | STM32H753ZITx.xml |
| W3 ULPI collisions | PA5, PA3, PB10, PB11 collide; no collision-free ULPI exists | inferred | pin XML vs assigned set |
| W3 RMII pins | REF_CLK PA1, CRS_DV PA7, RXD0 PC4, RXD1 PC5, TX_EN PB11/PG11, TXD0 PB12/PG13, TXD1 PB13/PG12/PG14, MDIO PA2, MDC PC1 | read (XML) | pin XML |
| W3 RMII collisions | PA7 (CRS_DV) and PA2 (MDIO) collide and have no alternates; TX_EN/TXD0/TXD1 can be PG11/PG13/PG12 | inferred | pin XML |
| W3 MII | not available on this package (PC2_C/PC3_C present), RMII only | read | ES0392 2.2.27 p.17 |
| W3 rates, HS PHY | OTG up to 480 Mbit/s but needs external ULPI PHY (no internal HS PHY); internal PHYs are FS 12 Mbit/s; Ethernet 10/100 Mbit/s | read | DS s3.41-3.42 p.52; RM Table 516/517 pp.2588-2590 |
| W4a SPI | simplex receiver COMM=10, slave MISO disabled, receives MOSI while NSS active; IOSWP swaps pins; two instances for MOSI+MISO | read + inferred | RM0433 s50.4.5 p.2161, p.2222 |
| W4b USART | separate TE/RE; RE=1 receiver-only; start bit pattern 1 1 1 0 X 0 X 0X 0X 0 X 0X 0; DMAR for DMA | read | RM0433 s48.2 p.2017, s48.5.6 p.2027 |
| W4c timer IC+DMA | capture latches counter, sets CCxIF, DMA request with CCxDE; both edges CC1NP=1/CC1P=1; ICxF filter | read | RM0433 s39.3.5 p.1652, CCER p.1695, CCMR1 p.1688 |
| W4d I2C monitor | none exists; matches ACK (OA2MSK=7 acks all 7-bit addresses) and stretch SCL unless NOSTRETCH, so it cannot passively observe | read (absence + quotes), inferred (consequence) | RM0433 s47.2 p.1951, s47.4.8 pp.1962-1963 |
| W5 I2C target | clock stretching default, NOSTRETCH=1 disables, SBC incompatible with NOSTRETCH; FM+ 1 MHz | read | RM0433 s47.4.8 pp.1962-1964 |
| W5 SPI slave max | rx 100 MHz (rev V; 150 SPI1-3 rev Y); tx/full-duplex 29-31 MHz (rev V), 25-31 (rev Y) | read | DS Table 194 p.301, Table 95 p.189 |
| W5 timer resolution | 1 tTIMxCLK; 240 MHz timer clock => 4.17 ns; ~400 ticks per 1.67 us bit; no min-pulse spec | read + inferred | DS Table 190 p.297 |
