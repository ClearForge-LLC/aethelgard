> Stored by `AET-WO-3403` from `~/.cache/aet-rev-s01/REVIEW.md` on Furnace (sha256 `cb488b65538ac26ff164f5b1865855a7bca66747d6b784a5dfad837cc578a4cd`), written 2026-11-19 by the read-only review session `rev-s01` (brief: `~/.cache/aet-rev-s01/brief.md`); everything below this line is that file, unchanged.

# Independent review: Sentinel-01 as ordered (held after the order)

Reviewer: rev-s01 (Claude Opus 5.5, read-only), 2026-11-19. Repository `/home/forge/aethelgard-rev-s01`, detached at `cd43b207`. Brief: `~/.cache/aet-rev-s01/brief.md`. Nothing in the repository was changed; nothing was pushed.

## Verdict: does anything in the order need a reorder?

**No.** I found nothing in the bare-board copper or in the component list that forces a reorder.
- **The copper.** I checked it against the makers' own drawings, not the KiCad library the gates used. U1's 144 pins match ST's LQFP-144 pinout (DS12117 Rev 10, Figure 6) one for one, and every routed signal sits on a pin whose AF table carries that function. U2's clockwise LGA-8 matches Bosch, U3 and U4 match TI and Winbond, and J1–J4 open toward the board edge with pin 1 where JST draws it.
- **Connectivity.** A label-free geometric pass over F_Cu, B_Cu and the drill file finds no short, no open and no undrilled via. The same tool caught a planted short and a planted open.
- **The package.** Its bytes are what `MANIFEST.json` names. CI's `verify --against-fab` derived it byte for byte on `main` at `cd43b207` (run 44542447316). The copper files have not changed since `ac5f8c82`, so any commit Scotty ordered from carries the same copper.
- **U2.** Leaving it empty costs no copper: the ordered firmware boots past the missing part.
- **What is wrong is on paper and in the bench plan, all fixable before the boards arrive:**
  - three `BENCH.md` rows expect the console to print a boot record it never prints (F1);
  - B7's method cannot isolate the threshold it claims to measure (F2);
  - B1, B6 and B8 each have a pass line that a real fault can slip through (F3–F5).
- **A BMP581 can stand in for U2 on J4 EXT with no copper change.** It needs firmware that addresses it, which no input of the ordered package provides, and a custom-pinned cable (U3). The firmware can come from either of two places:
  - today, a bench-only specification, as `AET-WO-3304` did for the Nucleo;
  - or, cleanly, a schema change that can declare a device behind a connector.

## Findings

Severity: **reorder** (the ordered copper or parts are wrong), **bench-blocking** (a bench row cannot be run or judged as written), **firmware-fixable**, **note**.

### F1 · B1, B4 and B8 expect the console to print a boot record it never prints · bench-blocking
- **Where:** `fab/sentinel-01-bench/BENCH.md` L39 (B1 pass 3), L66 (B4: "the console's boot record for the flash's JEDEC ID") and L133 (B8.1). Also `boards/sentinel-01-bench/firmware/src/main.rs` L27–50, L170–212 and L56.
- **Evidence [read]:**
  - USART1 sends only `CONSOLE` followed by `00 FE 7F 80 FF` (`PATTERN_TAIL`, L56), then echoes at most 8 received bytes.
  - The boot record is the RAM array `AETHELGARD_BOOT`, which the console never sends:
    - word 0 is `BOOT_MAGIC` 0x2100B007;
    - word 1 is `FLASH_OPTSR_CUR`;
    - words 2–3 are record 0 (status, value) from the read of 0x76 register 0xD0;
    - words 4–5 are record 1 (status, value) from the SPI 0x9F read, value `0x00EF4017` for `EF 40 17`.
  - So B4's identity can only be read over SWD.
  - Read literally, B1 pass 3 and B8.1 cannot pass. Read loosely, they pass on any bytes at all.
- **Recommendation:** rewrite the three pass lines:
  - **Console (B1, B8.1):** the 12 bytes `43 4F 4E 53 4F 4C 45 00 FE 7F 80 FF` at 115200 8N1.
  - **Records (B4):** over SWD, read word 0 = 0x2100B007 and record 1 = (0x22020000, 0x00EF4017).
  - **Record 0:** see F6 and U1.

### F2 · B7 attributes the controller's synchronisation delay and HSI's error to the pin threshold · bench-blocking (B7 only)
- **Where:** `BENCH.md` L110–122. RM0433 Rev 9 §47.4.9, p. 1971. DS12117 Rev 10, tAF on p. 298 (rev V, 50–80 ns) and p. 188 (rev Y, 50–260 ns). `crates/aethelgard-ir/src/silicon.rs` L316–322 (ANFOFF 0, DNF 0). `docs/work-orders/AET-WO-3302.bench.md` L242–249.
- **Evidence:**
  - **RM0433 [read]:** the I2C "detects its own SCL high level after a tSYNC2 delay". That delay depends on "the SCL rising slope", the analog filter (on here) and "two to three i2c_ker_ck periods" (31–47 ns at 64 MHz). SCLH counts from the detection, not from the pin threshold.
  - **The method absorbs the delay.** B7 step 3 places V_th where the high phase lasts 4.031 µs before the falling edge, so tSYNC2 is counted as part of the rise.
  - **The filter's delay.** DS12117 states the filter's spike-suppression width, 50–80 ns on rev V (the Nucleo was rev V), not its delay. A delay of the same order is [inferred]. That puts tSYNC2 at about 80–130 ns.
  - **The size of the shift [computed].** At 10 kΩ and about 45–60 pF (with C_p), the edge near 1.7 V climbs about 3 mV/ns. So tSYNC2 alone reads as V_th roughly 0.25–0.4 V too high.
  - **HSI's error is not small either.** Step 3 does take HSI's −2.6/+1.5 % into account. On 4.031 µs that is −105/+60 ns, about ±0.2–0.3 V on the same edge.
  - **The band is narrower than the method's error.** The method's combined uncertainty exceeds the ±0.2 V band before any threshold difference.
  - **The 1.7 V is not a threshold measurement.** It is the peak of a 100 kΩ sawtooth, which also includes SCLH counting time (3302.bench L245–249).
  - **So B7 can fail on a correct board**, and a pass does not show the threshold is 1.7 V. 3401's adversarial answer (FEEDBACK L307–308) counted "V_th within 1.7 ± 0.2 V" as a line that can fail. It can, but not only for the reason the row names.
  - B7's second pass clause ("measurably shorter from 0.7·VDD") follows from the first for any V_th below 2.31 V.
- **Recommendation:** measure at two edge rates, J4 empty and then with a known C0G (for example 47 pF) on TP11 to GND, and solve jointly. A threshold is fixed in volts and a delay is fixed in time, so the two edges separate them. Calibrate the kernel clock from the SCL low phase, whose start is the controller's own fast falling edge. Then state the band from that derivation.

### F3 · B1 can pass with NRST open on J2 · note
- **Where:** `BENCH.md` L35–40.
- **Evidence [inferred]:** the firmware never disables SWD, so a probe attaches whether or not NRST reaches J2 pin 3. A failure implicates NRST; a pass proves nothing about it.
- **Recommendation:** while the probe asserts reset, read TP3 at or below 0.3 V on a DMM or scope. Alternatively, show the console stays silent while reset is held.

### F4 · B6 has only an upper bound, so a wrong pull-up passes · note
- **Where:** `BENCH.md` L80–108. R1–R4 are all 0402 parts.
- **Evidence [computed]:**
  - A 2.2 kΩ fitted at R3/R4 (EXT) instead of 10 kΩ makes EXT about 4.5× faster. That passes B6 and silently removes the weak-bus operating point B7 and the campaign rely on.
  - A 10 kΩ at R1/R2 fails B6 correctly.
  - Separately, the bounds sit well above the likely copper (3302 measured about 55 pF for a whole breadboard rig), so an extractor error under roughly 2× still passes. That is acceptable for a bound, but it is not a measurement of the extractor.
- **Recommendation:**
  - Before power, read each pull-up from its test pad to TP4 with a DMM (2.2 k / 10 k).
  - Record the implied C (`t / (0.8473·R)` minus C_p) on every line, pass or fail, so the extractor's margin is measured and not just bounded.

### F5 · B8.1 cannot see a missing R8, and B8.2 cannot fail · note
- **Where:** `BENCH.md` L124–142.
- **Evidence [inferred]:**
  - With R8 open, BOOT0 floats. A DMM's 10 MΩ input pulls TP2 to about 0 V, so B8.1 still reads ≤ 0.727 V.
  - A stiff 3.3 V on J2 pin 2 reads 3.3 V whether R8 is there or not, so B8.2 fails only on a short.
  - The real evidence in B8 is the bootloader's answer, which is still gated on AN2606 (§7.1 L757, known).
- **Recommendation:**
  - Unpowered, read TP2 to TP1 = 10 kΩ ±1 %.
  - Or drive J2 pin 2 through a known 100 kΩ: about 0.30 V with R8 fitted, near 3.3 V without.

### F6 · No bench row witnesses I2C1 (PB6/PB7 AF4) on silicon, and B2 is common-mode with the generator · note
- **Where:** `BENCH.md` B2 (L42–56); `crates/aethelgard-twin/src/boot.rs` L856 and L901–908 (subagent trace); `examples/sentinel-01.json` `acknowledge_unwitnessed` (AFRL/MODER `same-source`).
- **Evidence:**
  - B2 compares silicon with the twin's record of the same topology. An AF wrong in both would agree.
  - The physical rows witness these:
    - USART1 PA9/PA10 AF7 (B1);
    - SPI1 PA5–7 AF5 and PA0 (B4);
    - PB10 AF4 (B7's pulses).
  - Nothing in the plan witnesses PB6/PB7 AF4 or PB11, and nothing reads record 0.
- **Recommendation:** add a row with a scope on TP13/TP14 across reset:
  - an address phase `0xEC` (0x76, write) clocked at 400 kHz;
  - with U2 fitted, record 0 = (0x22020000, 0x60);
  - with U2 empty, (0x22020001, 0) and a NACK on the scope (U1).

### F7 · INV-07: AF numbers and pin positions have a single source in every `verify` gate · note (did not occur on this board)
- **Where:**
  - `crates/aethelgard-cli/src/verify.rs` L1529–1560: erc, drc, support, extract, prove, build, simulate, campaign; no `check pinout`.
  - `crates/aethelgard-ir/src/solve.rs` L1290 (AF taken from `svd_hal::pin_table`, ST's XML).
  - `crates/aethelgard-eda` `schematic.rs` L1259–1279 (pad by ST `Position`).
  - `.github` `ardupilot-image.yml` L258–259 and L410 (the ArduPilot cross-check runs `sentinel-01-minimal.json` only).
  - Full trace: `~/.cache/aet-rev-s01/inv07/findings.md`.
- **Evidence:**
  - **The shared source.** The solver, the board crate, the twin's expected image and the schematic all read one ST pin table. A wrong AF or package position in it would pass every `verify` gate.
  - **The independent judge never sees this board.** `check pinout`, whose AF comes from unmodified ArduPilot's `STM32H753xx.py`, is not a `verify` gate. It has never run on `examples/sentinel-01.json`, and so never on I2C2 PB10/PB11.
  - **The independence it has is real.** `hwdef.dat` names signals, never AF numbers [read, subagent].
  - **It did not occur here.**
    - My own check against DS12117: all 144 pad nets equal ST's pinout, and all nine routed AFs equal DS12117's AF table (Tables at pp. 86–91): PB6/PB7/PB10/PB11 AF4, PA5–7 AF5, PA9/PA10 AF7.
    - The subagent found ArduPilot's pinned table agrees.
- **Recommendation (an engine work order, not this board):**
  - a tier-1 test holding `pin_table("STM32H753ZITx")` against ArduPilot's pinned `STM32H753xx.py`;
  - `examples/sentinel-01.json` added to the ArduPilot matrix;
  - a hand-entered DS12117 pad table as a third source for the MCU pad map.

**What the generator and `check pinout` share** (the brief's question, per attribute). Read in `crates/aethelgard-pincheck/src/lib.rs` L366–400; the subagent's trace is the source of the line numbers. `grep -c -E '\bAF[0-9]+|ALT\('` over the three hwdef fixtures returns 0, 0, 0.

| Attribute | Generator's source | Checker's expected value | Checker's observed value | Common-mode? |
|---|---|---|---|---|
| AF number | `solve` from ST XML (`solve.rs` L1290) | `topology.assignments[].alternate_function`, the same solved value (L368–371) | `gpio_default_config` in the ELF that unmodified ArduPilot built from `hwdef.dat`. `hwdef.dat` names signals only, so ArduPilot computes the AF from its own `STM32H753xx.py` | **No**: two tables meet |
| pin / signal | ST XML Position and signal names | topology | ArduPilot's own resolution of the signal name | No |
| speed | `pin_speeds`, stored by `solve` and emitted by hwdef (`hwdef/src/lib.rs` L395) | the same stored `pin_speeds` (pincheck L395) | ELF | **Yes**: both carry the stored value, and its adequacy is judged only by `check speeds` (not a `verify` gate). Already disclosed by 4400b FEEDBACK L435 (A7) |
| output type, pull | the hwdef generator does not set them | `ir::silicon::implied_open_drain` / `implied_pupd`, "which no generator calls" (L389–400) | ELF | No for the hwdef path. The Rust firmware's own OTYPER/PUPDR come from svd-hal's plan, and B2 reads them on silicon |
| identity (SPI dev, baro, driver) | `parts::table` ArduPilot columns, `DRIVEN_SPI_MODE` (hwdef L376, L752) | the same columns (`identity.rs` L360–407) | `hwdef.h` | **Yes** for ArduPilot boards (subagent F8); not used by Sentinel-01's Rust firmware |

**Sentinel-01's firmware is not ArduPilot.** `check pinout` therefore judges its sibling (`sentinel-01-minimal`) through ArduPilot, not the ordered ELF. For the ordered ELF the AF row above is single-source, and this review's DS12117 read is what makes it two.

### F8 · U3's land pattern is KiCad's IPC pattern, not TI's · note
- **Where:** `F_Cu.gtl` U3. TI SNOSD82D DRV0006B, pp. 40–41.
- **Evidence [measured, read]:**
  - The board's pads are 0.375 × 0.40 mm at 1.776 mm centres. TI's example is 0.45 × 0.30 mm at 1.95 mm. The pads still cover the leads (0.70–1.075 mm against leads at 0.70–1.00 mm).
  - The thermal pad is copper on net `N/C` (floating), with a mask opening and no paste. TI: "not connected to the device ground and should be left unsoldered for best measurement accuracy… if soldered, it must be left floating or grounded."
- **Recommendation:** none for the order. Hand assembly: hot air, and keep paste off pad 7 as the stencil already does.

### F9 · J3 feeds the MCU rail directly with no protection, and three identical GH-4 headers sit side by side · note
- **Where:** `F_Cu.gtl` J1/J3/J4 (all `SM04B-GHS-TB`, at x = 79.45 / 100.85 / 111.40 mm on the bottom edge). DS12117 Table "Voltage characteristics": VDD absolute maximum 4.0 V.
- **Evidence [read]:**
  - J3 pin 1 is `VDD_3V3`, straight to every VDD pin, with no regulator, no reverse-polarity element and no clamp.
  - A supply cable moved from J3 to J1 puts 3.3 V on PA9 (CONSOLE_TX) of an unpowered MCU.
  - J4 shares J3's pin 1 and pin 4, so power into J4 is benign.
- **Recommendation:** state in `BENCH.md`:
  - the supply never above 3.6 V, current-limited as written;
  - each cable labelled for its header;
  - J1 and J3 never swapped.

### F10 · J2's pinout is the project's own, not Dronecode's · note
- **Where:** `BENCH.md` L36 (1 VDD, 2 BOOT0, 3 NRST, 4 SWDIO, 5 SWCLK, 6 GND).
- **Evidence [inferred]:**
  - The Dronecode/Pixhawk JST-SH 6 debug port carries UART on pins 2 and 3. Such a cable would put its UART TX on NRST: idle high, so it works until it transmits, and every byte resets the board.
  - Connect-under-reset (B1) would also be unavailable through it.
- **Recommendation:** use a cable wired to J2's own pinout, and say so in B1.

### F11 · PTS810's internal pairing was not read at source · note
- **Where:** SW1, `SW_SPST_PTS810`: pads `1` on NRST, pads `2` on GND.
- **Evidence:** C&K/Littelfuse's datasheet returned "Access Denied", and I did not work around it.
  - If KiCad's pairing were wrong, NRST would be tied to GND and the board would never leave reset.
  - SW1 is optional: reset works through J2 pin 3 and power-cycling. So this cannot force a reorder.
- **Recommendation:** before fitting SW1, check with a DMM that pads 1–1 and 2–2 are each continuous and 1–2 is open until pressed.

### F12 · The board has no mounting holes and no fiducials · note
- **Where:** `sentinel-01-bench.drl`: one tool, 0.3 mm, 136 via hits. `Edge_Cuts.gm1`: a plain 46.35 × 54.85 mm rectangle.
- **Evidence:** the specification declares none. JLCPCB places small boards without fiducials, at its own discretion [inferred].
- **Recommendation:** none for this order. Bench: use a PCB vise or adhesive feet.

### F13 · Decoupling follows DS12117 Figure 65, and C12's effective capacitance is below 4.7 µF · note
- **Where:** FAB-NOTES' support section; ORDER.md §2 C12.
- **Evidence:**
  - **[read: the support gate's verdict, cross-checked on the Gerber nets]** Every capacitor DS12117 Figure 65 calls for is placed, each within the gate's bound of its pin:
    - one 100 nF at each of the 11 VDD pins;
    - 4.7 µF bulk;
    - VDDA: 100 nF and 1 µF;
    - VDD33USB: 100 nF and 1 µF;
    - VCAP 71 and 106: 2.2 µF each, at 1381 and 1213 µm;
    - NRST: 100 nF.
  - **C12** is a 4.7 µF X5R 6.3 V part in 0402 (GRM155R60J475ME47D). At 3.3 V its DC-bias loss is large: a 0402 6.3 V part typically keeps well under 70 % [inferred, Murata's curve not read].
  - The VCAP parts (CL05A225MP5NSNC, 10 V X5R) see about 1.2–1.35 V, so their bias loss is small [inferred]. The gate states it judges no dielectric, rating or ESR (FAB-NOTES' condition line).
- **Recommendation:** none for the order: bulk on a bench-supplied rail is not critical. For Sentinel-03, judge the effective capacitance at the rail voltage.

### F14 · The ordered ELF runs at VOS3 with 64 MHz peripheral clocks and never programs PWR_CR3 · firmware-fixable (known: 4240's F-VOS, not this review's)
- **Where:** `~/.cache/aet-bus/architect.md`, 4240's notes of 2026-11-19 (F-VOS, its addendum, the sequence correction and "F-VOS is stronger"). DS12117 Rev 10 Table 111, p. 210 (fPCLK max 50 MHz at VOS3). RM0433 Rev 9 §6.4.1 and Figure 21.
- **Evidence:** I did not read these pages myself; this is 4240's reading, cross-referenced here because it reaches this ELF (`008a2a59…`).
  - The generated firmware keeps the reset scale VOS3, with every pclk, the USART kernels and I2C1–4 at 64 MHz, above the 50 MHz VOS3 limit.
  - It writes no PWR register. RM0433 says RAM writes are "not permitted" until ACTVOSRDY, which follows the PWR_CR3 supply configuration.
  - 3302's silicon run of the same generated code booted and was read back. So the effect is "outside the datasheet's conditions", not "known broken".
- **Recommendation:**
  - Nothing for the copper.
  - On the bench, B2's register image and B4/U1's RAM records come from an ELF outside DS12117's operating conditions. Record PWR_CR3, PWR_CSR1.ACTVOSRDY and PWR_D3CR beside B2, as 4240's bench row B-2 proposes. Re-run B2/B4 with the ELF that 4240v produces, once it lands.

## U2: what Sentinel-01 can do with U2 empty, and the BMP581 on J4

### U1 · U2 left empty: the ordered firmware boots, and only the I2C identity witness is lost · note
- **Evidence [read]:**
  - `main.rs` L179–190 reads 0x76 register 0xD0 once. Every wait is bounded by `I2C_WAIT_SPINS` = 20 000 (`board.rs` L204).
  - On a NACK, TXIS never sets, so the read records `STATUS_TXIS` 0x22020001 with value 0. The firmware then reads the flash ID, writes `BOOT_MAGIC` and echoes the console.
  - **Measured on silicon** (`AET-WO-3302.bench.md` L242–246). With no sensor wired, a sibling ELF on the Nucleo (rev V) put `0xEC` on SDA, then NACK, then STOP. Its status was `0x22020001`, with no lockup. That firmware's I2C transfer is the same generated code path; it is not the ordered ELF.
  - Nothing else in the firmware touches U2.
  - B1–B5 and B8 are unaffected.
  - B6's SENSORS bounds still hold. They are upper bounds that include U2's pads and Ci, so with U2 off they are looser.
  - Only the silicon witness of a full I2C transaction (record 0 = 0x60) is lost.
- **What 4201's F5 measured, corrected:**
  - "U2 removed passes verify" (AET-WO-4201 FEEDBACK §4.3, PR #237) removed U2 from the specification. That deletes its footprint and its pads from the copper.
  - It also deletes the firmware's only I2C transfer, because the TMP117 is `renode: null` and the firmware reads only modelled devices (4201 §4.3).
  - So it judged neither the ordered copper nor the ordered ELF.
  - The ordered pair with U2 unfitted was judged by nothing. The reading above is a trace of the code, not a gate.
- **A witness still on the board:**
  - U3 (TMP117 at 0x48) stays on SENSORS. A debugger-driven I2C1 read of register 0x0F returns 0x0117 (TI SNOSD82D Table 7-15).
  - That witnesses PB6/PB7 AF4 and the SENSORS copper with no firmware change: write `I2C1.CR2` for a 1-byte write then a 2-byte read, as 3302 drove I2C over SWD.
- **The U2 options:**
  - **(a) Leave U2 empty.**
  - **(b) Fit a pin- and pad-compatible part.** None is verified. The BMP280 is register-compatible (chip ID 0x58, BME280 DS §5.2) on a 2.0 × 2.5 mm body, and whether it lands on U2's pads is unread. 4201's DPS368 note (same pin order, 2.0 × 2.5 mm) has the same open question.
  - **(c) Wait for BME280 stock** (§7.1 L829).

### U2 · A BMP581 cannot go in U2 on this copper · note (confirms the brief)
- **Evidence:** U2's land pattern is Bosch's BME280 LGA-8: 0.35 × 0.5 mm pads at 0.65 mm pitch, rows 2.05 mm apart (BME280 DS Figure 21, matched on the Gerber). The BMP581 is an LGA-10 of 2.0 × 2.0 mm.
- 4201's "BMP581 in U2" swap (FEEDBACK §4.1–4.2) regenerated the board, so it is a recommendation for a respin, not for this order.

### U3 · A BMP581 breakout on J4 EXT needs a specification and firmware change and a custom cable, but no copper · firmware-fixable
- **The copper is ready [read, Gerber].**
  - J4: pin 1 `VDD_3V3`, pin 2 `/EXT_SCL`, pin 3 `/EXT_SDA`, pin 4 `GND`.
  - EXT_SCL/EXT_SDA run to PB10/PB11 (U1 pads 69/70), which are I2C2 at AF4.
  - R3/R4 are 10 kΩ pull-ups to 3.3 V.
- **The firmware does not use EXT.** The ordered ELF enables I2C2 but starts no transfer on it (`main.rs` L164; FAB-NOTES ledger `I2C2.CR2` "the firmware starts no transfer on EXT").
- **The specification cannot say "behind J4".** `I2cDevice` (`crates/aethelgard-ir/src/lib.rs` L363–373: reference, part, supply, bus, address, pin_capacitance) has no placement field. A connector pin's `offboard` (`crates/aethelgard-ir/src/connectors.rs` L217–233) holds only `capacitance_femtofarads` and `source`. A device on bus `EXT` is therefore an on-board part to `emit`, so its specification describes a board with one more footprint than the ordered one [inferred from the schema; not emitted here].
- **Two paths, neither touching the copper:**
  - **(a) The 3304 precedent, now.** `AET-WO-3304` built the Nucleo's bench firmware from a bench-only specification, `examples/sentinel-02-bench.json`, which declares the BMP581 (U7) as a board part and was never fabricated (3304.bench L26).
    - Here that is a `sentinel-01-ext-bmp581` bench specification: `examples/sentinel-01.json` plus a BMP581 on `EXT` at 0x46/0x47.
    - **EXT's pins must be pinned** to PB10/PB11 (`pin` set, not `null`), so the solver cannot move the bus off J4's copper.
    - Then `generate`/`build`, and flash its ELF.
    - It has its own verify verdict. That verdict judges a board that was never made, so it says nothing about J4's copper with a breakout on it. The ordered package's verdict does not cover this ELF.
  - **(b) An engine work order:** a device declared behind a connector pin with its off-board capacitance, so that the ordered specification itself names the breakout and `--against-fab` shows the copper unchanged.
- **The twin.** The BMP581 Renode model exists (`Sensors.BMP581`, `parts.rs` L87). But 4201 measured a twin stamp refusal for a BMP581 through `droop-bor3` (4201 FEEDBACK §4.2). Path (a)'s verify must either avoid that scenario or wait for the fix.
- **The cable [inferred: the Qwiic/STEMMA QT convention].**
  - J4 is JST GH 1.25 mm in DS-009 order: 1 VDD, 2 SCL, 3 SDA, 4 GND.
  - Qwiic/STEMMA QT breakouts use JST SH 1.0 mm in the order 1 GND, 2 3V3, 3 SDA, 4 SCL.
  - A straight-through adapter puts 3.3 V on the breakout's GND, which reverses its supply. The cable must be pinned by hand: J4.1→3V3, J4.2→SCL, J4.3→SDA, J4.4→GND.
- **The load [read: BENCH header; inferred: breakout values].**
  - A breakout's own pull-ups parallel R3/R4. The bench's BMP581 breakout is unnamed and its pull-ups are unrecorded (`AET-WO-3304.bench.md` B3, L175–182, left blank for the bench). At 10 kΩ the line would be 5 kΩ; at 2.2 kΩ, 1.8 kΩ.
  - Its board, regulator and level shifter add capacitance, and J4's declared off-board allowance is 20 pF per line.
  - So B6's EXT rows and B7 must run with J4 empty, as B7 already says. With a breakout fitted, EXT is outside the bound the gates judged.
  - At 100 kHz with ≤ 5 kΩ the rise stays far under 1000 ns, so the breakout works electrically.

## What I measured

All commands ran on Furnace, read-only against the repository. Container runs used `~/bin/capped-run` with `nice -n 10`; caps were read back in the scope. Evidence is in `~/.cache/aet-rev-s01/` (`work/`, `tools/`, `ds/`, `dl/`, `inv07/`).

1. **Package bytes against MANIFEST.**
   - `sha256sum fab/sentinel-01-bench/package/*` against `MANIFEST.json` `files`: 16 of 16 equal.
   - `examples/sentinel-01.json` is `8e069b3e…921ef`, which equals `specification_sha256`.
2. **CI derivation on `main`.**
   - `gh run view 44542447316` (push, `cd43b207`): job `bench-package`, step "verify --against-fab, the committed package derived again" **success** (14:19–14:56Z).
   - The previous failure, run 44448262252 at `b14be5bf`, is the 1605 image re-pin moving `verdict_sha256`. #232 then re-derived MANIFEST only (`git diff --stat 0829fe83 2fc347fa -- fab/…/package`: MANIFEST.json, 1 line).
   - `--against-fab` re-runs every gate, then compares every derived file, MANIFEST included, as differs, missing or extra (`verify.rs` L1405–1490).
3. **Copper history.** `git log -- <each Gerber and the .drl>`: every copper, mask, paste and edge file last changed at `ac5f8c82` (3401's merge). They are byte-identical through `cd43b207`.
4. **Furnace derivation.**
   - Build: `cargo build --locked -p aethelgard-cli` and the bridge in `ghcr.io/clearforge-llc/aethelgard-tools@sha256:e5fc7ddb…` (`container/IMAGE`), with network.
   - Verify: `--network none`, `capped-run --mem 16G` (read back: `memory.max` 17179869184, `memory.swap.max` 0, `pids.max` 8192):
     `aethelgard verify --spec examples/sentinel-01.json --scenarios <the five in MANIFEST order> --against-fab fab/sentinel-01-bench/package`.
   - Result: see "Furnace verify result" below.
5. **U1 pin map against ST.**
   - `tools/gpads.py` reads every F_Cu flash's X2 `.P` (reference, pin) and `.N` (net).
   - DS12117 Rev 10 (sha256 `607b9ead…`, the copy the support gate cites) p. 57, Figure 6, rebuilt from word positions with PyMuPDF. Nine corner and two-word names were resolved by hand: 8 PC14, 9 PC15, 23 PH0, 24 PH1, 32 VREF+, 37 PA3, 72 VDD, 109 PA14, 144 VDD.
   - **144/144 agree.** Every VDD/VDDA/VREF+/VDD33USB/VBAT/PDR_ON pin is on `VDD_3V3`, every VSS/VSSA on `GND`, VCAP 71/106 on their own nets, NRST 25, BOOT0 138. Signals: 34 PA0 FLASH_CS; 41–43 PA5–7 SPI; 69/70 PB10/11 EXT; 101/102 PA9/10 console; 105/109 PA13/14 SWD; 136/137 PB6/7 I2C.
   - Pin 1 is top-left, numbered counter-clockwise, on F.Cu. Pads are 0.30 × 1.475 mm with inner/outer spans of 19.85/22.80 mm (ST's example: 0.35 × 1.35, 19.90/22.60).
6. **The AF table.** DS12117 pp. 86–91: PB6 and PB7 I2C1 AF4; PB10 and PB11 I2C2 AF4; PA5, PA6 and PA7 SPI1 AF5; PA9 and PA10 USART1 AF7. All match FAB-NOTES' ledger and the spec's acknowledgements.
7. **Label-free connectivity (`tools/gconn.py`, shapely 2.2.0).**
   - Every flash, track and region on F_Cu and B_Cu (1273 objects) was unioned by geometric intersection. Each of the 136 0.3 mm drill hits joins both layers.
   - Result: 0 shorts, 0 opens over 128 nets; 0 orphan holes; 0 undrilled via flashes (272).
   - **Mutation check:**
     - a 0.2 mm track drawn between U4 pads 1 and 2 was reported as the short `/FLASH_CS`–`/SPI_MISO`;
     - deleting `/SPI_SCK`'s 11 draws was reported as an open, 3 pieces.
8. **Spacing (`tools/gspace.py`).**
   - Minimum different-net copper: F_Cu 0.200 mm, B_Cu 0.2005 mm.
   - Copper to the outline: at least 0.50 mm.
   - Tracks: 0.2 mm only.
   - Outline: one closed rectangle, 72.2–118.55 × −75.85 to −130.7 mm (46.35 × 54.85).
9. **Mask and paste.**
   - F_Mask: 271 of 271 pads have an opening, the 14 test pads included.
   - F_Paste: every pad except the 14 test pads and U3's thermal pad, which TI recommends leaving unsoldered.
10. **Footprints against the makers' drawings.**
    - **BME280** (`dl/bme280/bst-bme280-ds002.pdf`, sha256 `a2ccdb44…`, downloaded): p. 38 shows pin 1 GND top-right with the numbering clockwise in top view; p. 43 gives pads of 0.35 × 0.5 mm, pitch 0.65 mm, centres 2.05 mm apart. The board matches, and the pin nets are 1 GND, 2 CSB→VDD (I2C), 3 SDA, 4 SCL, 5 SDO→R5→GND (0x76), 6 VDDIO, 7 GND, 8 VDD.
    - **TMP117** (`dl/tmp117/tmp117.pdf`, `78212e91…`): pinout p. 4; outline and land pattern pp. 40–41; thermal-pad notes pp. 35–36.
    - **W25Q64JV** (rev J text, `~/.cache/aet-4300c/ds/w25q64jv_revj.txt` §10.1): H 7.70–8.10 mm, L 0.50–0.80 mm, b 0.35–0.48 mm. The board's pads run 2.775–4.40 mm from the centre and are 0.65 mm wide, so they cover the leads. Pins: 1 CS, 2 MISO, 3 WP→R6↑, 4 GND, 5 MOSI, 6 SCK, 7 HOLD→R7↑, 8 VCC.
    - **JST GH and SH** (`eGH.pdf` `b1dcb317…` and `eSH.pdf` `ea3071ca…`, p. 3, side entry):
      - No. 1 is on the left looking into the opening, and the contact tails are at the rear. On the board, footprint pin 1 is at −x and the mating face points to the bottom edge.
      - Body fronts sit 0.78–0.80 mm inside the edge; courtyards are inside the outline.
      - Bodies are 4.05 mm deep (GH) and 4.25 mm (SH).
11. **The component list.**
    - `sentinel-01-bench.bom.csv` (39 rows), grouped by value and footprint, equals ORDER.md §2's 13 lines and the JLCPCB BOM's 13 rows:
      - 17 × 100n (C1–C11, C13, C15, C19–C22), C12 4u7, C14/C16 1u, C17/C18 2u2;
      - R1/R2 2k2, R3–R8 10k;
      - U1–U4, J1/J3/J4 GH-4, J2 SH-6, SW1.
    - Package suffixes against footprints: STM32H753ZI**T**6 → LQFP-144; TMP117AI**DRV**R → WSON-6 (TI's DRV0006B, which has a thermal pad); W25Q64JV**SS**IQ → SOIC-8 208 mil; BME280 → LGA-8 2.5 × 2.5 mm.
    - The 14 test pads' nets on F_Cu equal BENCH's table, TP1 GND through TP14 I2C_SDA.
    - R8 is BOOT0 to GND; C19 is NRST to GND; SW1 runs NRST to GND; J2 is 1 VDD, 2 BOOT0, 3 NRST, 4 SWDIO, 5 SWCLK, 6 GND (Gerber `.N`).
12. **INV-07 trace.** A subagent (Opus) traced the generator's and checker's code paths read-only. I spot-checked two of its claims: `verify` has no pinout gate (`verify.rs` L1529–1560), and the ordered ELF contains no `gpio_default_config` (`grep -a -c`: 0).

### Furnace verify result

**`--against-fab` on Furnace** (`work/against.log`; 18 min 44 s wall; the CLI exits 1, as expected):
- **Gates:** all 17 pass: erc, drc, support, extract, 8 × prove, build, simulate, 2 × attribute, campaign.
- **Fab report:** `status: derived`, `tool_version: 10.0.7`, `uid_boundary: false` (rootless podman).
- **The 16 files MANIFEST names:** each derived sha256 equals MANIFEST's. That covers every Gerber, the drill, BOM, CPL, pos, FAB-NOTES and the ELF `008a2a59…`.
- **The one difference:** `"MANIFEST.json differs from the derived file"` (derived sha256 `c7516b34…`). So `against_fab: differs` and `verdict: fail`.
- **Why that is the documented case:** FAB-NOTES L10–13 says the verdict states each tool's sandbox, so "a run under another uid or user namespace finds MANIFEST.json different while every other file is the same". CI, running as root in docker, derived the same MANIFEST byte for byte (item 2).
- **Measured on Furnace:** 15 files plus the ELF, byte for byte. **MANIFEST.json:** by CI.

**`--fab-out` on Furnace into scratch** (`work/fabout.log`, `work/tree/target/rev-fab/`, exit 0, `fab.status: written`):
- `sha256sum` of all 17 written files against the committed `package/`: 16 identical. Only `MANIFEST.json` differs: committed `2b499cc4…`, Furnace `c7516b34…`, the same hash the `--against-fab` run derived.
- `diff` of the two MANIFESTs: exactly one line, `verdict_sha256` (committed `3b585ef4…`, Furnace `a3e48664…`). Project, specification, topology, the five scenario hashes, the preconditions and all 16 file hashes are identical.
- **What that makes measured:** FAB-NOTES' claim that another uid or namespace moves only the verdict. **What stays inferred:** that the verdict lines differ only in the sandbox report, because CI's verdict line was not recoverable from its log.

## What I could not check

- **Moving after this review.** PR #238 (4400g, unmerged) moves `examples/sentinel-01.json` to specification 17 (`8e069b3e…` becomes `4846be8b…`). Its builder reports `MANIFEST.json` and `FAB-NOTES.md` re-derived, and the other 15 of the package's 17 files byte-identical, the ELF among them (`architect.md`, 4400g's 15:30Z note; not measured by me). The ordered copper is unaffected, but once #238 merges, `package/` on `main` will no longer carry this MANIFEST.
- **Whether the zip Scotty uploaded is these files.** The copper is unchanged since `ac5f8c82`, so any upload from `main` after 2026-11-17 09:00Z carries it. Scotty should compare the uploaded files' sha256 with `MANIFEST.json`, or with the fab's Gerber viewer against `package/`.
- **The fab's own CAM output, and the JLCPCB placement preview.** U1 is at rotation 0 in KiCad's frame, and pin 1 must be top-left. Only the order's preview shows this (FAB-NOTES' JLCPCB section; subagent F7).
- **PTS810's internal pairing** (F11): the download was denied.
- **The BMP280 and DPS368 against U2's pads** (U1 (b)): not read.
- **AN2606** (known, §7.1 L757). It is also unknown whether the bootloader's SPI1 slave (PA4 NSS floating, PA5–7 on the flash) could take the bootloader before USART1 [inferred risk, unread].
- **CI's verdict line.** The `gh run view --log` for runs 44542447316 and 44448262252 did not carry it; I have only the step conclusions. The Furnace run supplies the verdict.
- **The ArduPilot workflow.** `ardupilot-image.yml` runs on PR events, and its matrix does not include `examples/sentinel-01.json`, so its status says nothing about the ordered board.
- **Which fab took the order.** The via annular ring is 0.15 mm (0.6 mm pad on a 0.3 mm drill) and was not held against that fab's stated minimum. OSH Park's and JLCPCB's published minima are at or below it [inferred, not re-read].
- **Murata's DC-bias curve** for C12 (F13).
- **Electrical rules beyond geometry:** current capacity was not computed; all tracks are 0.2 mm and the MCU draws well under 100 mA at 64 MHz [inferred]. No check against IPC-2221.
- **B_Silkscreen and F_Silkscreen** were not checked for silk on pads. Fabs clip it.
- **The other rows of the B2 register image:** their values were not re-derived; they come from the twin's run of the ELF.
- **The `ardupilot-image.yml` status on the package's commit** (subagent: not checked).
