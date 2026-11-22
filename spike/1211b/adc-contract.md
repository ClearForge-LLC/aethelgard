# AET-WO-1211b — ADC bring-up register contract (STM32H753ZITx, rev Y and rev V)

Reading only; no repository file edited. Sources (plain-text extracts, pages are PDF page numbers = printed page numbers):
- **RM** = RM0433 Rev 9, `/home/forge/.cache/aet-1211/ds/rm0433-rev9.txt`
- **DS** = DS12117 Rev 10, `/home/forge/.cache/aet-1211/ds/ds12117-rev10.txt` (DS p.167 and p.273 rendered from the PDF to resolve Min/Typ/Max columns)
- **H** = `crates/aethelgard-svd-hal/data/st-cmsis-device-h7/stm32h753xx.h`

Clock premise (the brief's): sys_ck = HSI, RCC_D1CFGR reset 0x0000_0000 (D1CPRE 0xxx = /1, HPRE 0xxx = /1; RM p.394–395), so rcc_hclk1 = rcc_hclk4 = sys_ck (RM Fig. 49 p.351: rcc_hclk[2:1] and rcc_hclk4 are after HPRE).
HSI range used below: **f_max = 64.958976 MHz** (caller's figure); **f_min = 63.7 × (1 − 0.0012) × (1 − 0.02) = 62.351 MHz** (DS Table 36 p.128 = Table 133 p.232: 63.7 MHz min at 30 °C, ΔVDD −0.12 %, ΔTEMP −2 % for TJ −40 °C..TJmax; both tables read).
- **Note:** the same rows give a maximum of 64.3 × 1.0003 × 1.01 = 64.9625 MHz, slightly above the caller's 64.958976 MHz. Every spin below changes by < 0.01 %, and the 10 µs regulator spin is 650 cycles either way.

Notation: "ADC cycle" = 1/fADC. In the chosen synchronous mode, the ADC clock is a fixed division of HCLK = CPU clock. So every bound is also a **fixed CPU-cycle count, independent of HSI trim**: 4 CPU cycles per ADC cycle on rev Y, 8 on rev V. Sizing by the rev V figure covers both revisions.

---

## 0. Structure

- Steps 1–7 run **once per ADC instance** (ADC1 and/or ADC3).
- The ADCx_CCR write (step 2) is **once per common block**: ADC12_COMMON for ADC1, ADC3_COMMON for ADC3.
- Steps 8–9 run **once per solved input**.
- ADC2 is never touched. It must stay disabled, because ADC12_COMMON.CCR may be written only while every ADC of that block is disabled (step 2).

---

## 1. Bus clock enables

| Write | Field | Kind | Cite |
|---|---|---|---|
| RCC_AHB1ENR | ADC12EN (bit 5) = 1 (ADC1) | RMW (other enables live there) | RM §8.7.41 p.455 (bit text): "ADC1 and 2 peripheral clocks … the kernel clock selected by ADCSEL … and the rcc_hclk1 bus interface clock" |
| RCC_AHB4ENR | ADC3EN (bit 24) = 1 (ADC3) | RMW | RM §8.7.43 p.458: "… ADCSEL … and the rcc_hclk4 bus interface clock" |

The sequence after the enable is RM §8.5.10 ("Clock enabling delays", p.372):
- **(a)** Write the ENR register.
- **(b)** Read the ENR register back, so the write is not left in a write buffer.
- **(c)** Only for a peripheral in another domain: poll `DxCKRDY` and clear/read back `SBF_Dx`. ADC1 is in D2 (H: `ADC1_BASE` = D2_AHB1PERIPH_BASE + 0x2000). ADC3 is in D3 (`ADC3_BASE` = D3_AHB1PERIPH_BASE + 0x6000). The CPU is in D1.
- **(d)** Do a dummy read of a register in the enabled peripheral (e.g. ADC_ISR). It "takes at least 2 clock cycles, which is equal to the max delay". The bus clock appears ≤ 2 enabled-clock periods after the enable (same page).

No poll in this step, so no bound.
- **FINDING F1.** Step (c) applies to both ADCs. p.372 names only D1CKRDY/D2CKRDY and SBF_D1/SBF_D2, so there is no D3 flag for ADC3. Do whatever the generator already does for its other D2 peripherals (timers, USARTs) and D3 peripherals (GPIO is on AHB4/D3). Do not invent a new step here.

ADCSEL (RCC_D3CCIPR[17:16]) is **not written**. Reset = 00 = pll2_p_ck (RM p.419), which this firmware never starts.
- RM §8.5.10 p.371–372: switching a kernel mux "can only be performed if a clock is present on both inputs. Otherwise, no clock is provided to the peripheral". After reset the PLLs are off and "the switching does not take place".
- So a move to per_ck would not happen. Synchronous mode (CKMODE ≠ 00) does not use adc_ker_ck_input (RM Fig. 136/137 p.921–922: CKMODE muxes between adc_ker_ck_input and adc_hclk/adc_sclk).

## 2. Kernel clock: ADCx_CCR.CKMODE

**Choice: CKMODE = 11, PRESC = 0000, DUAL = 00000. Whole word `ADCx_COMMON.CCR = 0x0003_0000`** (reset 0x0000_0000, RM §25.7.2 p.1038).

What each CKMODE means (RM §25.7.2 p.1039, §25.4.3 p.920–921):
- **Rev Y:** 01/10/11 = adc_hclk/1, /2, /4. The analog ADC is clocked directly (Fig. 137 p.922 has no /2).
- **Rev V:** 01/10/11 = adc_sclk/1, /2, /4, and "Whatever CKMODE[1:0] settings, an additional divider factor of 2 is applied to the clock delivered to the analog ADC block" (p.1039; also p.920; Fig. 136 p.921 labels the input "Fadc_ker_ck x 2" → /2 → "Fadc_ker_ck"). adc_sclk = sys_ck when HPRE = 0xxx (p.920, p.921 note; RCC Fig. 58 p.364).
- **Verified:** rev V has the extra /2 and rev Y does not.

The DS fADC is the post-/2 clock, inferred:
- DS Table 173 p.273 gives a 12-bit rate of 5.50 MSps at fADC = 50 MHz with SMP = 2.5.
- tCONV = 2.5 + 0.5 + 12/2 = 9 cycles, and 50/9 = 5.56 MSps. That only fits if fADC is the conversion clock, Fadc_ker_ck.
- **FINDING F2.** The DS never states fADC ≡ Fadc_ker_ck; this is an inference.

| CKMODE | rev Y fADC (nom / max) | rev Y allowed? | rev V fADC (nom / max) | BOOST needed rev Y / rev V | Common? |
|---|---|---|---|---|---|
| 01 (/1) | 64 / 64.96 MHz | **No.** DS Table 76 p.166 max 36 MHz (BOOST=1). Also p.1039: 01 needs a 50 % duty system clock, and HSI DuCy is 45–55 % (DS Table 36). | 32 / 32.48 | — | No |
| 10 (/2) | 32 / 32.48 | Yes, BOOST=1 (≤ 36) | 16 / 16.24 | 1 (bit 8) / **10** (bit 9) | **No.** Bit 9 is reserved on rev Y. |
| **11 (/4)** | **16 / 16.240** | Yes | **8 / 8.120** | **1 / 01**, both = bit 8 only | **Yes** |

fADC at the chosen setting, with its limits:
- **Rev Y:** 15.588–16.240 MHz (62.351/4 .. 64.958976/4). DS Table 76 p.166: BOOST=1 max 36 MHz, BOOST=0 max 20 MHz, no minimum. Those rows are conditioned on "2 V ≤ VDDA ≤ 3.3 V".
- **Rev V:** 7.794–8.120 MHz (/8). DS Table 173 p.272: BOOST=01 0.12–12.5 MHz (1.62 ≤ VDDA ≤ 3.6 V). RM p.1009: BOOST 01 = "6.25 MHz < ADC clock ≤ 12.5 MHz".

PRESC "is applied only when CKMODE[1:0] = 0b00" (p.1039), so it is left at 0.

Write rules:
- CKMODE: "allowed to write these bits only when the ADCs are disabled (ADCAL=0, JADSTART=0, ADSTART=0, ADSTP=0, ADDIS=0 and ADEN=0)" (p.1039).
- §25.4.10 p.934: ADCx_CCR may be written "only if the ADC is disabled (ADEN must be equal to 0)".
- So write CCR after the bus enable and before any ADEN in that common block, which means ADC2 too for ADC12_COMMON.

**FINDING F3** (only matters if ADC1 and ADC3 are both solved). RM p.922 says "When several ADC interfaces are used simultaneously, it is mandatory to use the same clock source from the RCC block without prescaler ratio, for all ADC interfaces."
- In synchronous mode ADC1 takes rcc_hclk1 (rev Y) or adc_sclk (rev V). ADC3 takes rcc_hclk4 or adc_sclk.
- Both come from the same HPRE output at the same frequency, and both use CKMODE /4.
- Whether a /4 counts as a "prescaler ratio" is not defined. D1 solves ADC3 only.

## 3. ADC_CR.BOOST

- Rev Y: one bit, bit 8. 0 is "used when ADC clock < 20 MHz to save power"; 1 "must be used when ADC clock > 20 MHz". Bits 15:9 are reserved (RM p.1005).
- Rev V: BOOST[1:0] = bits 9:8: 00 ≤ 6.25, 01 6.25–12.5, 10 12.5–25, 11 25–50 MHz (RM p.1009).
- **Verified:** the field widths differ between revisions.

**Choice: bits[9:8] = 01, i.e. `ADC_CR_BOOST_0` (0x100).**
- Rev V: 7.79–8.12 MHz sits inside 01's range.
- Rev Y: BOOST=1 at 16.2 MHz. DS Table 76 allows fADC ≤ 36 with BOOST=1 and gives no minimum. The RM only says 0 saves power below 20 MHz, so the residual cost is power, not correctness.
- Bit 9 is written 0, which is its reset value on rev Y.
- Write rule: only when ADSTART=0 and JADSTART=0 (p.1005/1009).
- Set it before calibration, because BOOST "must be set according to the ADC clock setting" (p.922). **FINDING F4:** the RM does not order BOOST relative to ADCAL. Setting it before calibration is the conservative choice.

## 4. Deep power-down exit, regulator, wait

Source: RM §25.4.6 p.926. The order is: first DEEPPWD=0, "Then … ADVREGEN=1 … The software must wait for the startup time … (TADCVREG_STUP) before launching a calibration or enabling the ADC. This delay must be implemented by software."
- ADC_CR reset = 0x2000_0000 (p.1003; it is stated once, before both revision layouts).
- DEEPPWD and ADVREGEN may be written only when the ADC is disabled (p.1003/1008).

| # | Write (whole word) | Value |
|---|---|---|
| 4a | ADC_CR | `0x0000_0000` (DEEPPWD=0) |
| 4b | ADC_CR | `0x1000_0100` (ADVREGEN=1, BOOST=01) |
| 4c | fixed spin, **no poll** | ≥ 10 µs |

- **Wait:** tADCVREG_STUP max 10 µs on rev V (DS Table 173 p.273, typ 5). Rev Y spells it "tADCREG_STUP", also max 10 µs (DS Table 76 p.167, typ 5).
- **Spin length at the fastest HSI:** 10 µs × 64.958976 MHz = 649.6, so **≥ 650 CPU cycles**. The spin is independent of the ADC clock.
- **LDORDY** (ADC_ISR bit 12) exists only on rev V. RM Table 206 p.917, "LDO voltage regulator status … Available only on devices revision V"; the ISR p.998 defers to §25.3.
- On rev Y, what bit 12 reads is unspecified (the RM marks the feature rev V only), which is reason enough not to poll it. **LDORDY is not polled.** The fixed 10 µs wait satisfies both revisions, and RM p.926 already makes the delay a software duty.

## 5. Resolution: ADC_CFGR.RES

RM §25.6.4 p.1016 RES[2:0]:

| Code | Rev Y | Rev V |
|---|---|---|
| 000 | 16 bits | 16 bits |
| 001 | 14 bits | 14 bits, "standard mode" |
| 101 | — | 14 bits |
| 010 | 12 bits | 12 bits, "standard mode" |
| 110 | — | 12 bits |
| 011 | 10 bits | 10 bits |
| 100 | 8 bits | — |
| 111 | — | 8 bits |

Other codes are reserved.

- **12-bit: the common code is `010`** (rev V calls it "standard mode, not optimized power consumption"). DS Table 173 note 3 p.275 confirms standard mode is a characterised 12-bit mode: "The values provided for the 16-bit resolution also apply to the 14-bit and 12-bit resolutions in standard mode."
- 16, 14 (001), 12 (010) and 10 bits have common codes.
- **FINDING F5: 8-bit has NO common code** (100 on Y, 111 on V). The generator must refuse an 8-bit declaration while the revision is unknown.

ADC_CFGR reset = 0x8000_0000 (JQDIS=1; p.1012). There is one reset value, with no per-revision split.

**Whole word `ADC_CFGR = 0x8000_0008`** sets JQDIS=1 (kept), RES=010, and leaves every other field 0:
- EXTEN=00: software trigger.
- CONT=0: single conversion.
- DISCEN=0.
- DMNGT=00: "stored in DR only".
- OVRMOD=0.

Notes:
- JQDIS is kept at 1, because "A set or reset of JQDIS bit causes the injected queue to be flushed" (p.1012).
- Write rule: ADSTART=0 and JADSTART=0 (p.1016).

## 6. Calibration

Procedure, RM §25.4.8 p.928, steps 1–5:
1. DEEPPWD=0, ADVREGEN=1, and the regulator wait has elapsed.
2. ADEN=0.
3. Select ADCALDIF and ADCALLIN.
4. Set ADCAL=1.
5. Wait until ADCAL=0.

Write rules:
- ADCALDIF and ADCALLIN: only while the ADC is disabled and not calibrating (p.1003/1005).
- ADCAL: only when ADEN=0 (p.1003).
- DUAL must be 00000 (p.928 note); CCR already sets it.

| # | Write (whole word) | Value |
|---|---|---|
| 6a | ADC_CR | `0x1001_0100` (ADVREGEN, ADCALLIN=1, ADCALDIF=0 single-ended, BOOST=01) |
| 6b | ADC_CR | `0x9001_0100` (+ADCAL) |
| 6c | **poll ADC_CR.ADCAL == 0** | bound below |

**Duration of the ADCAL poll:**
- **tCAL (offset + linearity) = 165,010 ADC cycles.** Rev Y: DS Table 76 p.167, a single value spanning Min/Typ/Max. Rev V: DS Table 173 p.273, in the **Min** column (rendered).
- In CPU cycles: rev V 165,010 × 8 = **1,320,080**; rev Y × 4 = 660,040.
- In µs at the slowest fADC: rev V 165,010 / 7.794 MHz = **21,172 µs**; rev Y 10,586 µs. At the fastest HSI the rev V figure is 20,322 µs.
- Sizing 21,172 µs at 64.958976 MHz gives 1,375,300 CPU cycles, which is more conservative than the exact 1,320,080.

**FINDING F6: the DS gives no maximum for tCAL on rev V** (165,010 is a Min), and no Min/Typ/Max qualifier on rev Y. Any bound above 165,010 ADC cycles is an uncited engineering margin.

If ADCALLIN=0 is chosen instead:
- tOFF_CAL = 1,280 ADC cycles (DS same tables; Min column on rev V).
- That is 10,240 CPU cycles on rev V, or 164.2 µs at 7.794 MHz.

**FINDING F7: ADCALLIN=1 makes later whole-word ADC_CR writes illegal.**
- When linearity calibration completes, LINCALRDYW1..6 (bits 27:22) are set by hardware (p.1004, p.931).
- Software "is allowed to toggle a single LINCALRDYWx bit at once … otherwise causing unexpected behavior", and only with ADEN=1 (p.931–932, p.1004).
- A whole-word write of `0x1001_0101` for ADEN would clear all six at once while ADEN=0.

There are two options:
- **(a) ADCALLIN=1, with ADEN and ADSTART written read-modify-write.** This is my lean, because it gives the full calibration RM p.927 describes ("compensate offset and linearity deviation"). Read ADC_CR, clear the `rs` command bits, OR in the one bit, and write it back (see 9a). The expected value after ADEN is `0x1FC1_0101`, but it is not asserted.
- **(b) ADCALLIN=0.** Every ADC_CR write then stays a known whole word, **by inference only**. p.1004 says LINCALRDYWx is set "When the linearity calibration is complete", and p.932 says the bits are "reset when the calibration has not yet been done". No text states outright that an offset-only calibration leaves them at 0.

ADCALLIN must keep its calibrated value in every later word, since it is writable only while disabled. This is a decision for the architect.

## 7. Enable

Procedure, RM §25.4.9 p.933: clear ADRDY by writing 1, set ADEN=1, then wait until ADRDY=1.

| # | Write | Kind | Value |
|---|---|---|---|
| 7a | ADC_ISR | whole word, W1C. ISR bits are rc_w1 (p.998), and RM §1.2 p.101 defines rc_w1 as "Writing 0 has no effect" | `0x0000_0001` (ADRDY) |
| 7b | ADC_CR | RMW under option (a); whole word `0x1000_0101` under option (b) | ADEN=1 |
| 7c | poll ADC_ISR.ADRDY == 1 | bound below | |

**FINDING F8: the ADEN note contradicts the procedure.**
- The ADEN note says "allowed to set ADEN only when all bits of ADC_CR registers are 0 … except for bit ADVREGEN" (p.1007/1011). Read literally, that forbids BOOST ≠ 0 and the hardware-set LINCALRDYWx bits.
- The RM's own "Linearity calibration reading procedure" (p.931) sets ADEN right after an ADCALLIN=1 calibration, which leaves LINCALRDYWx = 1.
- So the governing condition is the parenthetical command-bit list (ADCAL, JADSTART, ADSTART, ADSTP, ADDIS, ADEN = 0).

**Duration of the ADRDY poll:**
- tSTAB = "1 conversion cycle" with the LDO already started. Rev Y: DS Table 76 p.167, spanning the columns. Rev V: DS Table 173 p.273, in the Min column.
- **FINDING F9: "conversion cycle" is undefined, and there is no maximum.** The worst reading is one tCONV at the longest SMP and 16 bits: 810.5 + 0.5 + 8 = **819 ADC cycles**.
- That is 6,552 CPU cycles on rev V, or **105.1 µs** at 7.794 MHz. Any margin on top is uncited.

## 8. Channel set-up (per input; e.g. D1: ADC3, channel 5)

Ordering:
- PCSEL comes before SQR1: "For each channel selected through SQRx or JSQRx, the corresponding ADC_PCSEL bit must be previously configured" (RM §25.4.12 p.935).
- PCSEL, SMPR, SQR and CFGR all require ADSTART=0 and JADSTART=0. They may therefore be written either before calibration or after ADRDY.
- My lean is before calibration (after step 4), so the ADC is calibrated in its final configuration. **FINDING F10:** the RM is silent on whether RES or SMP affect calibration.

| Reg | Whole word (reset 0 for all three: p.1019, 1020, 1021, 1023) | D1 value | Cite |
|---|---|---|---|
| ADC_PCSEL | bit ch = 1 | `0x0000_0020` | p.1021: "0: … not pre selected for conversion, **the ADC conversion result with this channel shows wrong result**" |
| ADC_SMPR1 (ch 0–9) / ADC_SMPR2 (ch 10–19) | SMPch = 111 at bits 3·ch (SMPR1) or 3·(ch−10) (SMPR2) | SMPR1 = `0x0003_8000` (SMP5 = 111) | p.1019–1020, 111 = 810.5 ADC cycles |
| ADC_SQR1 | L = 0000 (1 conversion), SQ1 = ch at bits 10:6 | `0x0000_0140` | p.1023; §25.4.14 p.937 "To convert a single channel, program a sequence with a length of 1" |

Channel mapping:
- ADC3_INP5 → VINP[5], a **Fast channel** (RM Fig. 140 p.925, rendered).
- PF3 is also listed as a fast channel in DS Table 174 note 4 p.277.
- ADC_DIFSEL reset 0x0000_0000 means single-ended (RM p.1034). DIFSEL can only be written with ADEN=0 (p.927), and it is not written.

Sampling time against the DS minimum:
- The worst case is the fastest fADC, which gives the shortest sample. 810.5 / 16.240 MHz = **49.91 µs** on rev Y; 810.5 / 8.120 MHz = 99.82 µs on rev V.
- DS Table 174 (p.276–277) has a largest 12-bit entry of **3.54E-07 s** (slow channel, 680 Ω); the fast-channel 680 Ω entry is 3.29E-07 s.
- 49.91 µs / 0.354 µs = 141, so it **exceeds every 12-bit minimum by ≥ 141×**. It also exceeds the largest entry in the whole table (8-bit 15 kΩ, 3.69E-06 s) by 13.5×.
- Rev Y has no table of its own. Its Fig. 38 note 5 (DS p.168) points to Table 174.

**FINDING F11 (board-side, not firmware):**
- Table 174's 12-bit rows stop at RAIN = 680 Ω and assume 47 pF and VDDA = 1.6 V (note 2).
- Rev V RAIN max at 12 bits = **1150 Ω** (DS Table 173 p.273, footnote 7: 2-LSB tolerance). Rev Y RAIN max = 50 kΩ (Table 76 p.167).
- The IPROPI source impedance is its sense resistor. If that resistor is above 1150 Ω, rev V 12-bit accuracy is outside the DS. The long SMP does not fix this by citation.
- Also: DS Table 173 note 2 says the SYSCFG_PMCR.BOOSTE switch booster "must be used for VDDA < 2.4 V", and rev Y fADC rows assume 2 V ≤ VDDA ≤ 3.3 V. The board's VDDA decides whether either applies.

## 9. One conversion

| # | Action | Kind |
|---|---|---|
| 9a | ADC_CR.ADSTART = 1 | Under (a): read, **clear the command bits** (ADCAL, ADEN, ADDIS, ADSTART, ADSTP, JADSTART, JADSTP), OR in ADSTART, write. Under (b): whole word `0x1000_0104` |
| 9b | poll ADC_ISR.EOC == 1 | bound below |
| 9c | read ADC_DR | 12-bit result right-aligned in RDATA[11:0] |

- **Why the command bits are written 0:** they are `rs`, and RM §1.2 p.101 defines `rs` as "Writing 0 has no effect on the bit value", so ADEN stays 1. Writing ADEN=1 again would be a fresh "set", which the ADEN note allows only when ADEN=0 (p.1007/1011). Writing ADCAL=1 by accident would be illegal with ADEN=1.
- ADSTART is allowed only when ADEN=1 and ADDIS=0 (p.1007/1011). Do not set it while EOC is high (p.939); EOC is 0 after the enable.
- With CONT=0 and EXTEN=00, ADSTART clears itself at EOS (p.938).
- EOC is set when the data is in ADC_DR, and reading ADC_DR clears it (p.1000; §25.4.24 p.955).
- Alignment is set by CFGR2's OVSS/LSHIFT, whose reset is 0 (§25.4.27 p.957, Fig. 166 p.959; CFGR2 reset 0x0000_0000 p.1017).

**Duration of the EOC poll:**
- tCONV = tS + 0.5 + N/2 = 810.5 + 0.5 + 6 = **817 ADC cycles** (DS Table 76 p.167 and Table 173 p.274). RM Table 214 p.955 agrees: 12-bit TSAR = 6.5.
- Add tLATR max: 2.25 (rev V CKMODE=11, Table 173 p.273) or 2.125 (rev Y, Table 76 p.167). The total is **819.25 ADC cycles**.
- In CPU cycles: rev V 6,554; rev Y 3,277.
- In µs at the slowest fADC: rev V **105.1 µs**; rev Y 52.6 µs.
- **FINDING F12:** RM §25.4.13 p.936 states "TCONV = Sampling time + 7.5 ADC clock cycles", which is the 14-bit TSAR, against Table 214's 6.5 for 12 bits. The difference of one cycle is absorbed by any margin.

## 10. Reset values and the pin

Reset values left as they are:
- ADC_IER = 0 (p.1001).
- ADC_CFGR2 = 0 (p.1017).
- ADC_DIFSEL = 0.
- ADCx_CCR.DUAL = 0.
- ADC_SQR2..4 and ADC_SMPR2 = 0 for D1.

The only non-zero reset value carried forward is ADC_CFGR.JQDIS = 1.

Pin (PF3):
- PF3 is LQFP144 pin 13, I/O structure "FT_ha" (`_a` = "analog option (supplied by VDDA)", Table 7 p.63), additional function ADC3_INP5 (DS Table 8 p.65, rendered).
- It is **not** a dual pad. The SYSCFG_PMCR analog switches apply only to PA0_C, PA1_C, PC2_C and PC3_C (RM p.539, §12.3.1 p.550; DS Table 174 note 3).
- So PF3 has **no SYSCFG requirement**.
- The pin must be in analog mode: GPIOF_MODER.MODE3 = 11, "Analog mode (reset state)". Port F's MODER reset is 0xFFFF_FFFF (RM §11.4.1 p.541).
- Write MODE3 explicitly by RMW so the state does not rest on reset, using the generator's existing pin-mode path (GPIOFEN in RCC_AHB4ENR).
- **FINDING F13:** the DS Table 7 legend (p.63) says "all I/Os are set as floating inputs during and after reset", which contradicts the RM's MODER reset. The explicit write makes this moot.

---

## CMSIS names (all present in H; line numbers in H)

| Field | `_Pos` / `_Msk` (H line) |
|---|---|
| RCC_AHB1ENR.ADC12EN | `RCC_AHB1ENR_ADC12EN_Pos`/`_Msk` (15609) |
| RCC_AHB4ENR.ADC3EN | `RCC_AHB4ENR_ADC3EN_Pos`/`_Msk` (15720) |
| CCR.CKMODE | `ADC_CCR_CKMODE_Pos`/`_Msk` (4043), `_0`/`_1` |
| CCR.PRESC | `ADC_CCR_PRESC_Pos`/`_Msk` (4049) |
| CCR.DUAL | `ADC_CCR_DUAL_Pos`/`_Msk` (4019) |
| CR.DEEPPWD | `ADC_CR_DEEPPWD_Pos`/`_Msk` (2935) |
| CR.ADVREGEN | `ADC_CR_ADVREGEN_Pos`/`_Msk` (2932) |
| CR.BOOST | `ADC_CR_BOOST_Pos`/`_Msk` (2906), `_Msk` = 0x3 << 8 (**the rev V shape**); write `ADC_CR_BOOST_0` only |
| CR.ADCALLIN | `ADC_CR_ADCALLIN_Pos`/`_Msk` (2911) |
| CR.ADCALDIF | `ADC_CR_ADCALDIF_Pos`/`_Msk` (2938) |
| CR.ADCAL | `ADC_CR_ADCAL_Pos`/`_Msk` (2941) |
| CR.ADEN | `ADC_CR_ADEN_Pos`/`_Msk` (2888) |
| CR.ADSTART | `ADC_CR_ADSTART_Pos`/`_Msk` (2894) |
| CR.LINCALRDYW1..6 | `ADC_CR_LINCALRDYW1_Pos` (2914) … (read only for the RMW expectation) |
| ISR.ADRDY | `ADC_ISR_ADRDY_Pos`/`_Msk` (2815) |
| ISR.EOC | `ADC_ISR_EOC_Pos`/`_Msk` (2821) |
| ISR.LDORDY | `ADC_ISR_LDORDY_Pos`/`_Msk` (2848): **present, but rev V only; do not poll** |
| CFGR.JQDIS | `ADC_CFGR_JQDIS_Pos`/`_Msk` (3023) |
| CFGR.RES | `ADC_CFGR_RES_Pos`/`_Msk` (2952), `_0/_1/_2`; 12-bit = `ADC_CFGR_RES_1` |
| PCSEL | `ADC_PCSEL_PCSEL_Pos`/`_Msk` (3228) plus per-bit `ADC_PCSEL_PCSEL_0..19` (3231–3250). **There is no per-channel `_Pos`**: use `1 << (ADC_PCSEL_PCSEL_Pos + ch)` or `ADC_PCSEL_PCSEL_<ch>`. |
| SMPR1.SMPx / SMPR2.SMPx | `ADC_SMPR1_SMP0..9_Pos`/`_Msk` (3086…3149), `ADC_SMPR2_SMP10..19_Pos` (3157…3220); D1 uses `ADC_SMPR1_SMP5_Pos` (3121) |
| SQR1.L / SQ1 | `ADC_SQR1_L_Pos`/`_Msk` (3264), `ADC_SQR1_SQ1_Pos`/`_Msk` (3272) |
| DR.RDATA | `ADC_DR_RDATA_Pos`/`_Msk` (3419) |
| GPIO MODER | `GPIO_MODER_MODE3_Pos` is an **alias** of `GPIO_MODER_MODER3_Pos` (11832) |

Instances and registers:
- `ADC1`, `ADC3` (`ADC_TypeDef`).
- `ADC12_COMMON`, `ADC3_COMMON` (`ADC_Common_TypeDef`, member `CCR` at +0x308 from the master base).
- `ADC_TypeDef` members: ISR 0x00, CR 0x08, CFGR 0x0C, SMPR1 0x14, SMPR2 0x18, PCSEL 0x1C, SQR1 0x30, DR 0x40.

Naming flags:
- H carries only the rev V layout of BOOST. There is no rev Y name for a 1-bit BOOST, so the bit-8 write is the same address either way.
- **RM naming slip:** RM §25.4.3 p.921 and §25.7.2 p.1039 say "HPRE[3:0] … in RCC_CFGR register". On this part the field is **RCC_D1CFGR**.HPRE (RM p.395; H `RCC_D1CFGR_HPRE_Pos` 14928).

## Full ordered sequence per ADC (option (a), D1 = ADC3/ch5)

1. Set `RCC_AHB4ENR |= ADC3EN`, read RCC_AHB4ENR back, do the domain step per F1, then a dummy read of ADC3.ISR.
2. Write `ADC3_COMMON.CCR = 0x0003_0000`.
3. Write `ADC3.CR = 0x0000_0000`.
4. Write `ADC3.CR = 0x1000_0100`, then spin ≥ 650 CPU cycles (≥ 10 µs).
5. Write `ADC3.CFGR = 0x8000_0008`, `PCSEL = 0x20`, `SMPR1 = 0x0003_8000`, `SQR1 = 0x140`.
6. Write `ADC3.CR = 0x1001_0100`, then `ADC3.CR = 0x9001_0100`. Poll `CR.ADCAL == 0` for ≥ 1,320,080 CPU cycles worth (21,172 µs at slowest fADC; no DS max, F6).
7. Write `ADC3.ISR = 0x1`, then `ADC3.CR |= ADEN` (RMW). Poll `ISR.ADRDY` for ≥ 6,552 CPU cycles (105.1 µs; F9).
8. `ADC3.CR = (ADC3.CR & ~command_bits) | ADSTART`. Poll `ISR.EOC` for ≥ 6,554 CPU cycles (105.1 µs). Read `ADC3.DR`.

Under option (b), replace "1001" with "1000" in step 6, and step 7 / 8 become the whole words `0x1000_0101` and `0x1000_0104`.
