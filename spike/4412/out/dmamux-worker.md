# AET-WO-4412 DMAMUX timer request lines (RM0433 Rev 9)

Held PDF: /home/forge/.cache/aet-held/rm0433-rev9.pdf, sha256 672a9433d6602da224e5215f571beed2be0e683cb025526f4afe9db61c26b6a0.
Matches the committed record spike/4240/ds/SHA256SUMS.st:1 (same hash, name rm0433-rev9.pdf). Also cited as "672a9433..." in spike/1219/citations.tsv and crates/aethelgard-twin/data/dmamux1-usart-rx.tsv:3.

## Page method
No pdftotext/PDF library on the box. I read the .txt footers ("N/3329") and, independently, parsed the PDF directly (PDF-1.3, no object streams; page tree walked, 3329 pages; content streams inflated). PDF page index N contains the footer string "N/3329" (e.g. page 697's stream has "RM0433 Rev 9 697/3329" and TIM1_UP). So printed page = PDF page index for this document. Text-layer order inside a page stream differs from visual order, so row-to-page assignment for Table 125 (split across 698/699) comes from the .txt footer positions; treat the 698/699 split as high-confidence, not pixel-verified.

## 1. The tables (real captions)
- Table 124, "DMAMUX1: assignment of multiplexer inputs to resources", p697 (continued p698). TOC (txt:2821) says 697.
- Table 125, caption IDENTICAL ("DMAMUX1: assignment of multiplexer inputs to resources"), p698 (continued p699). TOC says 698. It is a verbatim duplicate of Table 124 (every row same); ST's Rev 9 prints the table twice under two numbers. Surprising; no stated reason. Existing repo code cites Table 124 p697 (dmamux1-usart-rx.tsv:2-3, spi-rx).
- Tables 126/127 (p699): DMAMUX1 trigger / synchronization inputs; input 7 = TIM12_TRGO (not a request line).
- Table 128 (p700), "DMAMUX2: assignment of multiplexer inputs to resources": inputs 1-8 req_gen, 9-10 lpuart1, 11-12 spi6, 13-14 i2c4, 15-16 sai4, 17 adc3, 18-32 Reserved. NO timer. Table 129 (p700) DMAMUX2 triggers: no TIM either.
- Cross-check: Table 103 "DMAMUX1, DMA1 and DMA2 connections" (block interconnect, printed pp603-607; TOC says 603) lists the same lines as timN_xxx_dma -> dmamux1_req_inN. Agrees on every number. ST typos there: "tim17_ch1_mda"; the TIM5 rows are labelled peripheral "TIM3" (D1 APB1 TIM3 tim5_ch3_dma). Table 104 (DMAMUX2/BDMA, p608) has no timer.

## 2. Timer -> requests (DMAMUX1 input number; full per-row quotes/pages in timers.tsv)
- TIM1: 11 CH1, 12 CH2, 13 CH3, 14 CH4, 15 UP, 16 TRIG, 17 COM
- TIM2: 18 CH1, 19 CH2, 20 CH3, 21 CH4, 22 UP
- TIM3: 23 CH1, 24 CH2, 25 CH3, 26 CH4, 27 UP, 28 TRIG
- TIM4: 29 CH1, 30 CH2, 31 CH3, 32 UP (no CH4, no TRIG)
- TIM5: 55 CH1, 56 CH2, 57 CH3, 58 CH4, 59 UP, 60 TRIG
- TIM6: 69 UP.  TIM7: 70 UP
- TIM8: 47 CH1, 48 CH2, 49 CH3, 50 CH4, 51 UP, 52 TRIG, 53 COM (54 is Reserved)
- TIM12, TIM13, TIM14: NONE
- TIM15: 105 CH1, 106 UP, 107 TRIG, 108 COM
- TIM16: 109 CH1, 110 UP
- TIM17: 111 CH1, 112 UP
Pages: Table 124 holds all of them on p697. Table 125: requests <=25, 44-68, 87-111 on p698; the rest (TIM3 CH4/UP/TRIG, TIM4, TIM6, TIM7, TIM17_UP) on p699. Table 103: TIM1-4 p603, TIM8 and TIM5 p604, TIM6/TIM7 p605, TIM15-17 p606.
Numbers are 1-based (DMAMUX input 0 does not exist; "dmamux1_req_in1"..); channel registers take this number as DMAREQ_ID.

## 3. Chapter cross-check (TIM12/13/14, chapter 40)
- 40.2.1 TIM12 main features, p1709: "Interrupt generation on the following events: Update ... Trigger event ... Input capture ... Output compare". No DMA bullet.
- 40.2.2 TIM13/TIM14 main features, p1710: "Interrupt generation on the following events: Update ... Input capture ... Output compare". No DMA bullet.
- Stray: Figure 455 (TIM13/TIM14 block diagram, p1711) labels an output "Interrupt & DMA output"; the text and DMAMUX tables give no DMA request, so it is an ST figure artefact (not established further).
- TIM12 appears in the DMAMUX only as tim12_trgo -> dmamux1_gen7 (Table 103 p606) and dmamux1_trg7 (p607); Table 126/127 input 7.
Conclusion: TIM12/13/14 cannot raise a DMA request; a DMA-driven DShot (ArduPilot's bdshot/DMA scheme) cannot run on them.

## 5. Repo locations (read-only)
(a) Solver offers timers by pin alternate function only, not by DMA capability: crates/aethelgard-ir/src/motor.rs:733 `TIMERS = [1,2,3,4,5,8,12,13,14,15,16,17]` (doc: "timers with capture/compare channels"; TIM6/7 excluded as channelless); motor.rs:754 `timer_channel()` parses `TIMn_CH1..4`; crates/aethelgard-ir/src/solve.rs:1846-1858 builds the channel list from every unreserved pin's `TIMn_CHk` AF; solve.rs:2100-2127 `check_motors`/`NoTimerChannel`. There is no DShot-specific or DMA-aware filter (grep of solve.rs/motor.rs for dma finds only unrelated ADC1 text, solve.rs:669, 1217). So TIM12/13/14 (no DMA request) are currently offered to a DShot motor output. Also offered: TIM5/TIM2 even though hwdef reserves SYSTEM_TIMERS (crates/aethelgard-hwdef/src/lib.rs:61 [5, 2]) - separate concern.
(b) Twin wiring: only SPI and USART receive requests are wired. platform.rs:809 `DMARecieve -> dmamux1@{n}` (SPI, from data/dmamux1-spi-rx.tsv via spi_rx_request); platform.rs:1043 `ReceiveDmaRequest -> dmamux1@{n}` (USART, foreign-clock board only, data/dmamux1-usart-rx.tsv). No timer request line is wired anywhere (grep of platform.rs for tim+dma finds nothing). Timer Renode blocks come from the stock file (crates/aethelgard-twin/src/timer.rs:137 STOCK_TIMERS, 14 timers) with `Timers.STM32_Timer`; I did not examine whether that Renode model exposes any DMA output (not established).
(c) Existing request-number tables: crates/aethelgard-twin/data/dmamux1-spi-rx.tsv and dmamux1-usart-rx.tsv (loaded by platform.rs:100 `DMAMUX1_SPI_RX`, :132 `DMAMUX1_USART_RX`, parsed by `dma_rows` platform.rs:109); tests at crates/aethelgard-twin/src/tests.rs:993. Format: peripheral<TAB>resource<TAB>number, `#` comments; a TIM table could reuse `dma_rows`. No timer rows exist. Prior WO notes (docs/work-orders/AET-WO-4400i.FEEDBACK.md:22) already record that 4400h mis-cited "Table 121"; Rev 9 = Table 124.

## 6. "request 15 = TIM1_UP"
CONFIRMED: Table 124 p697 row "15 TIM1_UP" (also Table 125 p698, Table 103 p603 "tim1_up_dma dmamux1_req_in15"). Neighbours 14 TIM1_CH4, 16 TIM1_TRIG. Only the number-to-signal mapping is established here; I did not check which DMAMUX channel number (9) ArduPilot used.

## Not established
- PDF visual rendering was not inspected (no renderer); Table 125 page split 698/699 inferred from text order and footers.
- Why ST prints Table 124 and 125 identically (Rev 9 quirk vs. one table meant for a different device subset; the text gives no distinction).
- Whether Renode's STM32_Timer exposes any DMA request output.
- RM0433 may be superseded by later revisions with different table numbering; only Rev 9 read.
