# AET-WO-4240 FEEDBACK — Spike: the clock tree from the HSE

**Status: STOPPED at §2, for the architect's ruling and Scotty's crystal choice.**
- **Branch:** `wo/aet-4240-hse-clock-spike`, from `main` `cd43b207` (the kickoff's SHA; live `main` at the stop is named in §12).
- **Builder:** Claude (Opus 5.5), with read-only subagents (Sonnet) for the source sweeps and one Opus subagent for the scratch measurements. Every subagent figure that a recommendation rests on was re-read by the builder: each is marked "builder re-read".
- **Fence kept:**
  - only `spike/4240/` and this file are committed;
  - scratch trees, the knob, CLIs and ELFs are under `~/.cache/aet-4240/`;
  - no crate, model, parts table, golden, board, test, workflow, `ci/`, `container/` or Renode file changed;
  - no ST-LINK change, no `sudo`, no packages.

**Labels:**
- **[measured]**: run here, with the log named.
- **[read]**: read at source, with document, section and page, or file and line.
- **[inferred]**: reasoning or desk arithmetic.
- **[uncited]**: a source without a document number or revision. No recommendation rests on one.

## 0. The answer in one screen

- **D1, the tree: H1** (HSE → PLL1 → 64 MHz, every prescaler 1), plus `VOS` Scale 2.
  - Measured on the 31 fixtures: the 22 that solve on `main` gain no refusal. The 4 refused by design stay refused. The 5 stored topologies are refused `UnsupportedClock` only as a knob artefact (§7.1).
  - No `PSC`/`ARR`/`CCR`/`BRR`/`MBR` moves; only one field of each I2C `TIMINGR` moves.
  - CRSF 416666 solves with **+22,652 ppm** (−3,125 on the HSI today).
  - `4400g`'s F stays 64 MHz, and the twin's push residue vanishes.
- **D2, the crystal (Scotty's), PROVISIONAL: 16 MHz, 3225, CL ≤ 12 pF, ABM8AIG, ordered as `ABM8AIG-16.000MHZ-12-2Z-T3`** (a part number **constructed** from the ordering table, not seen in a listing).
  - It passes AN2867's oscillation rule with 2.7× room. That is **[uncited]** until the current AN2867 is held: only Rev 10 is, so the rule is uncited by the ruling (§3).
  - **Every stocked CL 18 pF part at 8, 24 and 25 MHz fails** DS12117's `Gm_crit_max` (1.5 mA/V).
  - Stock is readable at LCSC only, so §3.3 is a stocking plan for Scotty's hand read.
- **New, outside this spike's fence: F-VOS.**
  - Every generated firmware enables peripherals with `pclk` and the USART kernels at 64 MHz, at the reset voltage scale VOS3.
  - DS12117 Table 111 and RM0433 Table 60 cap them at **50 MHz** there.
  - The fix is `PWR_CR3` (a declared supply mode, written once), then `ACTVOSRDY`, then `VOS` Scale 2, then `VOSRDY`. It is proposed as its own small slice, `4240v`, landing first.
  - **Ruled** a real finding; `3304`'s bench is not held.
- **The bench (D7).**
  - The Nucleo runs HSE in **bypass** from the ST-LINK's `MCO`, "fixed at 8 MHz", derived from an undocumented source.
  - **PH0 is not on CN11 by default** (SB46 OFF).
  - Rows B-0 and B-1 read it without touching the board's solder.
  - `sentinel-02-bench` does not move until `3304`'s bench has run on `375fd31a`. Unwired, the change is a no-op, measured.
- **Predictions:** P1, P2, P3, P6 and P9 confirmed. P4 and P7 half (DShot's centre is 107 ticks, not 106; every `TIMINGR` moves). P8 refuted: H0 at 8 MHz **panics** in `solve` (F-PANIC). P5 open.
- **D5:** halt with a named boot word on `HSERDY`, `PLL1RDY` or `SWS` timeout, and treat CSS's NMI as a halt.
  - Silicon's CSS **is** a hardware fallback to the HSI. The firmware must not continue on it.
- **D4:** `mcu.hse` replaces `mcu.hse_hz` as an object, `null` for none: `crystal` or `external` mode, a rational frequency (8.33 MHz is not an integer, F-HZ), the part and capacitors, and the budget's declarations. Spec schema 18 or 19 (after `4400g`'s 17 and `4240v`).
- **D10:** `4240v` (VOS), then `4240a` (ir, after `4400g`), `4240b` (generator and twin), `4240c` (eda, after `1315`), then a bypass bench of the bring-up before `4500`.

## 1. Sources read (§1.0)

| Source | Revision, sha256 | Where read | Full read |
|---|---|---|---|
| RM0433 | Rev 9, `672a9433…b6a0` | §8 (RCC), §6 (PWR), §4.3.8 (flash), §11 (GPIO) | `spike/4240/reads/rm0433-ds12117.md` |
| DS12117 | Rev 10, `607b9ead…c032` | HSE Tables 31/33 (rev Y) = 128/130 (rev V); PLL 39/40 = 136/137; HSI 36 = 133; general operating conditions Table 111 | same |
| AN2867 | DocID15287 **Rev 10** (2015), `93cd3c28…2f0e2`; from a crystal maker's copy (st.com refuses Furnace) | §3.3–§3.5, pp. 13–16 | §3 here; `reads/crystals.md` §5 |
| UM2407 | Rev 6, `b493e14f…a4cd` (= `3304` F L19) | §7.5.1 p. 26; Table 14 p. 32; Table 22 p. 43 | §2 here |
| MB1364 schematic, RN0093 | **not held** (st.com) | — | asked on the bus |
| Renode `STM32H7_RCC.cs` | `cd4b002a`, `7f10d491…` (re-fetched by commit, equal) | whole file; `LimitTimer.cs`, `STM32_Timer.cs`, `STM32F7_USART.cs` | `reads/renode-ardupilot.md` A |
| ArduPilot `stm32h7_mcuconf.h` | `Copter-4.7.2` (`fb773567`), `67f48e5f…a426` | L113–343; `chibios_hwdef.py`; ChibiOS `hal_lld.c`; Nucleo hwdefs | `reads/renode-ardupilot.md` B |
| Crystal datasheets (15) | `spike/4240/ds/SHA256SUMS` | per part | `reads/crystals.md` |
| KiCad 10.0.7 library | GitLab files API, ref `10.0.7` | `Crystal.pretty/Crystal_SMD_3225-4Pin_3.2x2.5mm.kicad_mod` (blob `11874e16…`), `Device.kicad_symdir/Crystal_GND24.kicad_sym` (blob `97e45475…`): **both exist upstream; neither is vendored** | §8 |
| `4400g` | PR #238 head `afb91036` (read only) | `motor.rs` `from_rcc`, `pll1_p` | §6.1, D8 |
| `1315`, `2304` | the docs branch `claude/argon-repo-docs-synthesis-rjrb0d` (no `1315` branch exists) | the work orders | D10 |

**Spot-checks by the builder** of subagent reads:
- RM0433 `DIVP1`/`PLL1RGE` text; Table 60 p. 356 (page image);
- DS12117 Table 111 p. 210 (page image);
- UM2407 §7.5.1 and Table 14;
- RM0433 §8.5.2, §8.5.3, the `MCO1` bits, `RCC_AHB4ENR`;
- Renode L24–28, L183–188, L1396–1400;
- `mcuconf` L113–343;
- ST open pin data for PH0/PH1;
- `solve.rs:410, 664–676`; `svd-hal` `board.rs:1957–1964`.

**One subagent reading was not adopted:** the Renode read called SysTick's `sys_ck / D1CPRE` a modelling error. RM0433 Figure 49 says otherwise (§6). **One subagent figure was wrong and corrected:** ABM8AIG's ESR at 20–25.999 MHz (§3).

## 2. The bench board's HSE: the NUCLEO-H753ZI (§1.1)

**Source.** UM2407 Rev 6, held at `~/.cache/aet-3304/ds/um2407-rev6.pdf` (sha256 `b493e14f…a4cd`, `3304` FEEDBACK L19). The MB1364 schematic and RN0093 are **not held**: st.com resets Furnace's connections over HTTP/2 and HTTP/1.1 (measured 2026-11-19 14:45Z). Both are asked for on the bus.

| Question | Answer | Where (read at source) |
|---|---|---|
| What drives `PH0`/`OSC_IN` by default | The ST-LINK-V3E's `MCO` (its MCU is an `STM32F723IEK6`), through **SB45 ON**. SB44 and SB46 are OFF, and SB3 and SB4 are OFF | UM2407 §7.5.1, p. 26: "MCO from ST-LINK (Default) … SB44 and SB46 OFF, SB45 ON, SB3 and SB4 OFF". Table 14 (continued), p. 32: "SB45 ON: MCO of ST-LINK (STM32F723IEK6) is connected to PF0/PH0 of STM32H7" |
| Through which resistor | UM2407 names only SB45, a 0 Ω bridge (p. 10: "SBx connections closed by 0 Ω resistor"). Whether a series resistor sits on the `STLK_MCO` net is **not stated in UM2407**. It needs the MB1364 schematic, which is not held | — (open) |
| The frequency | "By default, it is fixed at 8 MHz … The frequency may be changed during ST-Link firmware upgrade (for more details, refer to RN0093)" | §7.5.1, p. 26 |
| What the 8 MHz is derived from | **Not stated in UM2407.** P5 predicts HSI/2 of the `STM32F723`, because the ST-LINK's own 25 MHz crystal (see X3's figures below for the H7 side; the ST-LINK side is on MB1364) divides by `MCO1PRE` (1 to 5 on the F7) to 8.33 MHz, never 8.000. **Unconfirmed**: it needs RN0093 or a bench measurement (row B-1 below) | — (open, inferred) |
| `PH0` on the morpho connector | **Not by default.** CN11 pin 29 is `PH0` (Table 22, p. 43), but it is connected only with SB46 ON. Table 14's row for SB44, SB46 = OFF, OFF (the default, §7.5.1) reads: "PF0/PH0 and PF1/PH1 are not connected to the ST morpho connector (CN11)". **UM2407 contradicts itself here:** the same row goes on "X3, C69, C70, SB3, and SB4 provide a clock. In this case, SB45 must be OFF", while §7.5.1's default is SB45 ON with SB3/SB4 OFF. The builder takes §7.5.1 and the SB45 row ("ON: MCO of ST-LINK … is connected to PF0/PH0") as the default; B-1 confirms it on the board | Table 14 (continued), p. 32 |
| `X3` | Not fitted ("not provided"). If fitted: "25 MHz, 6 pF, 20 ppm", with SB3 and SB4 ON, C69 and C70 fitted "with 5.6 pF capacitors", and SB45 OFF | §7.5.1, p. 26; Table 14, p. 32 |
| What fitting `X3` costs | One 25 MHz crystal, two 5.6 pF capacitors (C69, C70), two bridges closed (SB3, SB4), and one opened (SB45, which takes the ST-LINK `MCO` off `PH0`). That is soldering on Scotty's only bench board | as above |
| An external oscillator instead | Through CN11 pin 29, with SB46 ON, SB45 OFF, and SB3 and SB4 OFF | §7.5.1, p. 26 |
| `PA8` (`MCO1` out of the H753) | CN12 pin 23 (Table 22, p. 43), and test point TP4 ("USB SOF", p. 30) | Table 22, p. 43 |

**So the bench runs HSE in bypass** (`RCC_CR.HSEBYP` = 1), fed a square wave by the ST-LINK. A Sentinel with a crystal runs HSE in **crystal** mode (`HSEBYP` = 0). The specification must say which (D4), because the two differ in the register word, in the pins the solver reserves (bypass needs `PH0` only: RM0433 Rev 9 §8.5.2, Figure 46, p. 339 draws `OSC_OUT` as "GPIO" in the external-clock configuration, and p. 340 says the source "has to drive the OSC_IN pin" with "~50% duty cycle"), and in the ppm budget (bypass: the source's ppm, not a crystal's).

**The ST-LINK firmware version on t9o777 was not read.** This builder has no access to t9o777 or the bench (the work order's fence). Rows B-0 and B-1 below are for Scotty. Neither changes the ST-LINK's firmware, and neither installs a package.

- **B-2, the supply state at reset (read only).**
  - **After a power cycle** (unplug USB; `PWR_CR3` is reset only by POR, so a warm reset shows whatever earlier firmware wrote), attach under reset (`probe-rs attach --connect-under-reset`, flag to be checked against the installed `probe-rs` 0.32.0's `--help`):
    - `probe-rs read b32 0x5802480C` (`PWR_CR3`: `LDOEN` bit 1, `BYPASS` bit 0, `SCUEN` bit 2);
    - `probe-rs read b32 0x58024804` (`PWR_CSR1`: `ACTVOSRDY` bit 13);
    - `probe-rs read b32 0x58024818` (`PWR_D3CR`: `VOS` bits 15:14, `VOSRDY` bit 13).
  - Addresses: PWR at 0x5802 4800 (`stm32h7.repl` L728, the RM0433 memory map), `CR3` +0x0C, `CSR1` +0x04, `D3CR` +0x18 (§6.8).
  - This answers whether the default supply configuration already reads `ACTVOSRDY` 1 (F-VOS), and confirms VOS3 at reset.

**What P3 means for the bench.**
- If the `MCO` is 8.000 MHz, an integer PLL1 reaches 64 MHz exactly (§5), so the bench can run H1's tree.
  - But the source is then the ST-LINK MCU's HSI, so the bench witnesses H1's **register sequence**, not a crystal's ppm.
- If the `MCO` is 8.33 MHz (25 MHz ÷ 3), it is not an integer in Hz (8,333,333.3…), and `mcu.hse_hz` (a `u32`) cannot state it exactly (finding F-HZ).
  - 64 MHz is then unreachable with an integer PLL (P3, scored in §5).
- **The bench's options:**
  - (a) Leave the `MCO` at its default, after confirming 8.000 MHz (B-1). The bench then witnesses bring-up and switch, but no crystal tolerance.
  - (b) Fit X3 (25 MHz). The bench then runs a crystal, but at 25 MHz, not the Sentinel's frequency, so H1's `DIVM1`/`DIVN1`/`DIVP1` words differ from flight's. 64 MHz is reachable from 25 MHz, for example `DIVM1` 5 (5 MHz reference), `DIVN1` 128 (640 MHz VCO) and `DIVP1` 10; the enumeration against the cited limits is in §5.
  - (c) Accept that the bench clock is unlike flight's, and say so in the bench specification.

**Bench rows for Scotty (t9o777; nothing changes on the board).**
- **B-0, the ST-LINK firmware version.**
  - Run `~/tools/probe-rs/bin/probe-rs list` and paste the line. It names the probe and its firmware, for example `V3J…M…`.
  - **Whether `list` prints the firmware version is not verified here** (no `probe-rs` on Furnace) [inferred]. If it does not, try `probe-rs info`, or `lsusb -v -d 0483:` (the ST-LINK's descriptors: `bcdDevice` and `iProduct`).
  - That version and RN0093 fix the `MCO` setting.
- **B-1, the `MCO` frequency at the H753, non-invasively.**
  - **Probe.** The FX2 analyzer's D0 goes on CN12 pin 23 (`PA8`), and GND on CN12 pin 20.
  - **Writes.** With the board halted (`probe-rs` attached), route `MCO1` = HSE ÷ 4 to `PA8` with these writes. Each is cited to RM0433 Rev 9, and each is a read-modify-write through `probe-rs read`/`write`:
    1. `RCC_CFGR` (0x5802 4410): `MCO1[24:22]` = `010` (`hse_ck`), `MCO1PRE[21:18]` = `0100` (÷4). §8.7.7 says to set these before enabling the external oscillators.
    2. `RCC_AHB4ENR` (0x5802 44E0): `GPIOAEN` (bit 0) = 1.
    3. `GPIOA_MODER` (0x5802 0000): bits 17:16 = `10` (alternate function).
    4. `GPIOA_AFRH` (0x5802 0024): bits 3:0 = `0000` (AF0 = `MCO1`).
    5. `RCC_CR` (0x5802 4400): `HSEBYP` (bit 18) = 1, then `HSEON` (bit 16) = 1. Then read `HSERDY` (bit 17) and expect 1.
  - **Capture.** `sigrok-cli -d fx2lafw --config samplerate=24m --samples 24000000 -C D0 -O csv > mco.csv`. That is 1 s; at 2 MHz there are 12 samples per period.
  - **Count rising edges.** About 2,000,000 means 8.000 MHz. About 2,083,333 means 8.333 MHz.
  - **The analyzer's own timebase bounds the figure's accuracy.** Its crystal is typically tens of ppm, uncited, so B-1 distinguishes HSI/2 from crystal ÷ 3 (a 4 % difference). It does not measure a crystal's ppm.
  - **Restore.** A reset (`probe-rs reset`) returns every register written to its reset value.
  - **Every address is read at source (builder re-read):** RCC 0x5802 4400 and GPIOA 0x5802 0000 (RM0433 Rev 9 Table 8); `RCC_AHB4ENR` offset 0x0E0 (§8.7.43, Table 69), `GPIOAEN` bit 0; `GPIOx_MODER` 0x00 (§11.4.1); `GPIOx_AFRH` 0x24 (§11.4.10); `MCO1`/`MCO1PRE` (§8.7.7, p. 392). RM0433 §8.5.4 (p. 345): the MCO pin must be in alternate-function mode, and its clock must not exceed the pin's speed. 2 MHz is within the reset (low) speed [inferred].

## 3. The crystal (§1.2)

**Sources.**
- **Datasheets:** fifteen, held in `spike/4240/ds/SHA256SUMS`. The PDFs stay in `~/.cache/aet-4240/ds/` (their makers' copyright). The full read is in `spike/4240/reads/crystals.md`.
- **AN2867:** ST AN2867 **DocID15287 Rev 10** (August 2015), sha256 `93cd3c28…2f0e2`.
  - It was obtained from a crystal maker's site (Raltron), because st.com refuses Furnace.
  - The PDF itself prints ST's document number and revision on every page.
  - **It is old:** a search snippet, not a held document, names Rev 24 (2026-02) as current. Rev 10 predates the H7, so every AN2867 figure below carries "Rev 10" and the current revision is asked for on the bus.
- **Stock:** read at LCSC (public product pages, plain `curl`, 2026-11-19 14:50–14:51Z).
  - DigiKey returns a Cloudflare 403 and Mouser closes the connection. **Both are unreadable from Furnace.** No retry or workaround was attempted (`4201`'s lesson).
  - **No candidate can be shown in stock at two distributors.** That is unknown, not "no": WO §7's stop needs a reading of two distributors, and only one is readable. The stocking plan is below.

**Two corrections and a ruling apply to this section.**
- **Corrected after the builder's re-read.** ABM8AIG's ESR at 20.000–25.999 MHz is **50 Ω, not the 40 Ω** the crystal read gave (ABM8AIG REVISED 06-13-19, p. 1). The 24 and 25 MHz rows were recomputed; both still pass.
- **Re-read by the builder** (rendered pages): the 16 MHz row (ESR 70 Ω, C0 2.0 pF, drive level 10 typ / 100 max µW, aging ±2 ppm first year, p. 1) and the option codes (p. 2: CL "Blank: 18pF" else specified; tolerance `2` ±20 ppm; stability `Z` ±50 ppm, standard; temperature blank −40…+125 °C).
- **The architect's ruling (bus, 2026-11-19 15:20Z):** until the current AN2867 is held, every row resting on it is **uncited**.
  - The threshold, 1.5 mA/V, is DS12117's: cited, and re-read by the builder on p. 125.
  - The formula and the load-capacitor rule are AN2867 Rev 10's.
  - So §3.1's verdicts are **[uncited: current AN2867 not held]**, and **D2 is provisional**, per WO §7.

**The rules applied** (each read at source):
- **Oscillation (AN2867 Rev 10 §3.4, pp. 13–14).** "If the oscillation loop maximal critical gain parameter (gm_crit_max) is specified, it is important to make sure that the oscillation loop critical gain (gmcrit) is smaller than the specified parameter."
  - `gmcrit = 4 · ESR · (2πF)² · (C0 + CL)²`.
  - DS12117 Rev 10 Table 33 (rev Y, p. 125) = Table 130 (rev V, p. 229): `Gmcritmax` **1.5 mA/V** (startup).
  - The ×5 margin applies only where the datasheet gives the oscillator's `gm`. DS12117 gives `Gm_crit_max` instead, which AN2867 relates as `Gm_crit_max = gm / 5` (p. 14). So the margin is already inside 1.5 mA/V.
- **Load capacitors (AN2867 Rev 10 §3.3, p. 13).** `CL = CL1·CL2/(CL1+CL2) + Cs`, so `C1 = C2 = 2(CL − Cs)`. AN2867's worked example *assumes* `Cs` = 5 pF; it gives no bound.
- **Drive level (AN2867 Rev 10 §3.5, pp. 15–16).** It is `DL = I²Q,RMS × ESR`, measured with a probe of at most 1 pF at the amplifier input. **It cannot be computed from datasheets:** DS12117 gives no HSE drive figure (`reads/rm0433-ds12117.md` §3.1, "NOT FOUND"). So it is a bench measurement on the first board, against the crystal's maximum (100–200 µW for every candidate).

### 3.1 The oscillation margin, per candidate (computed; script `spike/4240/py/gmcrit.py`)

| Part | F MHz | ESR Ω | C0 pF | CL pF | gmcrit mA/V | ratio to 1.5 | verdict | C1=C2 at Cs 3 / 5 pF | cite | stock |
|---|---|---|---|---|---|---|---|---|---|---|
| ABM8AIG-8.000MHZ-1Z-T | 8 | 500 | 2.0 | 18 | 2.021 | 0.74× | **FAIL** | 30 / 26 pF | ABM8AIG 06-13-19 p1-2 | LCSC C5140594 8065 |
| ABM8AIG-8.000MHZ-10-1Z-T [CL 10 variant, constructed] | 8 | 500 | 2.0 | 10 | 0.728 | 2.06× | PASS | 14 / 10 pF | same | unknown |
| ABM8AIG-8.000MHZ-8-1Z-T [CL 8 variant, constructed] | 8 | 500 | 2.0 | 8 | 0.505 | 2.97× | PASS | 10 / 6 pF | same | unknown |
| ABM8AIG-16.000MHZ-12-2Z-T3 [constructed] | 16 | 70 | 2.0 | 12 | 0.555 | 2.70× | PASS | 18 / 14 pF | ABM8AIG 06-13-19 p1 (ESR 70 Ω 16.000–19.999, C0 2.0), p2 (codes): builder re-read | unknown (DigiKey 403) |
| ABM8AIG-24.000MHZ-12-2Z-T3 [constructed] | 24 | 50 | 2.0 | 12 | 0.891 | 1.68× | PASS | 18 / 14 pF | same | unknown |
| ABM8AIG-25.000MHZ-12-2Z-T3 [constructed] | 25 | 50 | 2.0 | 12 | 0.967 | 1.55× | PASS | 18 / 14 pF | same | unknown |
| ABM8-24.000MHZ-D1G-T | 24 | 50 | 3.0 | 18 | 2.006 | 0.75× | **FAIL** | 30 / 26 pF | ABM8 07-29-20 p1 | LCSC C1986116 91 |
| ABM8-25.000MHZ-D2Y-T | 25 | 50 | 3.0 | 18 | 2.176 | 0.69× | **FAIL** | 30 / 26 pF | same | LCSC C179643 776 |
| ABM8-12.000MHZ-D2Y-T (12 MHz: not an ArduPilot branch) | 12 | 120 | 3.0 | 18 | 1.203 | 1.25× | PASS (not AP) | 30 / 26 pF | same | LCSC C6491123 1127 |
| ECS-160-12-33Q-JES (constructed) | 16 | 80 | 7.0 | 12 | 1.167 | 1.28× | PASS | 18 / 14 pF | ECX-33Q Rev.2023 p1 | unknown |
| ECS-80-18-33Q (8 MHz) | 8 | 500 | 7.0 | 18 | 3.158 | 0.47× | **FAIL** | 30 / 26 pF | same | unknown |
| ECS-160-8-33B2Q-VY (constructed) | 16 | 70 | 2.0 | 8 | 0.283 | 5.30× | PASS | 10 / 6 pF | ECX-33B2Q Rev.2024 p1-2 | unknown |
| NX3225GA-16MHZ-STD-CRG-2 | 16 | 80 | n/s | 8 | — (C0 not stated); 0.39 if C0 ≤ 3 [inferred] | — | uncomputable | 10 / 6 pF | NDK 2008E_NX3225GA_e (no C0) | LCSC C156247 4690 |
| NX3225GA-24MHZ-STD-CRG-2 | 24 | 50 | n/s | 8 | — (C0 not stated); 0.55 if C0 ≤ 3 [inferred] | — | uncomputable | 10 / 6 pF | same | LCSC C156248 5380 |
| NX3225GD-8MHZ-STD-CRA-3 | 8 | 500 | n/s | 8 | — (C0 not stated); 0.61 if C0 ≤ 3 [inferred] | — | uncomputable | 10 / 6 pF | NDK 2008E_NX3225GD_e (no C0) | LCSC C889706 21485 |
| YXC X32258MSB4SI (YSX321SL) | 8 | 180 | 3.0 | 20 | 0.962 | 1.56× | PASS | 34 / 30 pF | UNCITED (no doc no./rev) | LCSC C2682774 92700 |

**Read:**
- **CL 18 pF fails at 8, 24 and 25 MHz.** That includes the only fully cited 8 MHz part LCSC stocks (ABM8AIG-8.000MHZ-1Z-T, 8,065 in stock, 2.02 mA/V).
- At 8 MHz, the cited sheets' ESR is 400–500 Ω (YXC's uncited sheet says 180 Ω), so ABM8AIG-8 passes only below CL ≈ 15 pF (CL 12: 0.99 mA/V, 1.51×; CL 10: 2.06×).
- 16 MHz at CL 8–12 pF passes: 2.7× (ABM8AIG, C0 2 pF), 5.3× (ECX-33B2Q), but only 1.28× for ECX-33Q (C0 7 pF).
- **NDK's sheets state no `C0`,** so their parts are uncomputable by the rule and cannot carry a recommendation (WO §7). They are the cheapest and best stocked at LCSC.
- YXC's sheet passes but has no document number or revision, so it is uncited.

### 3.2 The ppm budget (D3), per candidate

**The formula proposed:**
- `fast = tol₂₅ + stab(T_declared) + aging(life_declared) + pull⁺`, and the same for `slow`, each term as its maker states it (ppm, symmetric in every held sheet).
- **Cited from the part:** `tol₂₅`, `stab` over the part's stated range, and `aging` (every held sheet gives "first year" or "per year" only).
- **Declared:**
  - the temperature range, which must sit inside the part's stated range (else refused);
  - the service life, using `4300d` D-B4's 8,000 h precedent (under one year, so one "first year" term);
  - `pull`, the pulling from the load's departure from `CL`.
- **`pull` has no cited figure.** No held datasheet states pullability or the motional capacitance `C1` (`reads/crystals.md` §1: "pullability not stated in any held datasheet"). So it is a **declared bound** in the specification, `RUL-14` style, until a maker's figure is held.
  - **What it bounds** is the departure of the board's load from `CL`: the capacitors' tolerance (C0G ±0.25 pF or ±1 %), plus `Cs`'s uncertainty.
  - **`Cs` can be bounded by `extract`** (`1305`'s copper bound on a line's capacitance) plus the pin capacitance. That pin figure is DS12117's I/O `C_IO`, to be cited at build.
- **In bypass** (the Nucleo), the source's ppm replaces the crystal's terms entirely. It is unknown until B-0/B-1 and RN0093.

| Candidate | tol₂₅ | stab, range | aging, 1st yr | Cited sum | P1's "±50 total" |
|---|---|---|---|---|---|
| ABM8AIG-16.000MHZ-12-2Z-T3 | ±20 | ±50, −40…+125 °C | ±2 | **±72 ppm + pull** | more than P1 assumed |
| ABM8AIG-8.000MHZ-(CL ≤ 10)-1Z-T | ±10 | ±50, −40…+125 °C | ±2 | ±62 + pull | |
| ECS-160-8-33B2Q-VY-TR3 | ±30 | ±150, −40…+150 °C | ±2 | ±182 + pull | |

**The budget is not tight.** At the 64 MHz kernel, the largest crystal deviation at which CRSF 416666 still solves is about 22,760 ppm (desk arithmetic with `serial::check_silicon`'s formula; measured in §7). Any cited crystal plus any plausible declared pull is two orders of magnitude inside it. **The budget matters for honesty, not for margin:** `RUL-14` forbids putting a nominal in a minimum's place.

### 3.3 Stocking plan (the recommendation, WO §7)

1. **Scotty's hand read.** DigiKey and Mouser stock and price for:
   - **ABM8AIG-16.000MHZ-12-2Z-T3** (the recommendation);
   - ABM8AIG-8.000MHZ-10-1Z-T (or `-8-`), if 8 MHz is preferred;
   - ECS-160-8-33B2Q-VY-TR3 (second maker).
   - None of the three was found by LCSC's search (2026-11-19). That search page renders in JavaScript, so this is **unknown, not absent**.
2. **A second-maker part with a stated `C0`** at 16 MHz and CL ≤ 12 pF, in stock at two distributors. ECX-33B2Q (C0 2 pF) qualifies on paper, at ±150 ppm stability.
3. **NDK** becomes eligible the moment NDK's `C0` is held (ask NDK, or a full datasheet beyond the one-page brief). NX3225GA-16MHZ-STD-CRG-2 is 4,690 at LCSC and $0.19 at 150.

**JLCPCB assembly**, the route Sentinel boards take: LCSC's stock is what an assembled order draws on. A part LCSC is not shown to carry is a consignment or a substitution, which `1604`'s ruled JLCPCB files would have to say.

## 4. The independent PLL rule (§1.3), cited per register and per limit

**Read at source:** RM0433 Rev 9 (footer `NNN/3329`; the printed page is the PDF page) and DS12117 Rev 10 (`NNN/357`). The full table, with a verbatim quote per field, is `spike/4240/reads/rm0433-ds12117.md`.
- It was extracted by a read agent. The builder re-read `DIVP1` and `PLL1RGE` in the text and DS12117 Table 111 (p. 210) and RM0433 Table 60 (p. 356) from page images.
- **The work order's "`CSSHSEON`" is `HSECSSON`** (`RCC_CR` bit 19, set-only).
- No later RM0433 revision is held, so no change after Rev 9 is claimed.

### 4.1 Registers

| Register (§, offset, reset, page) | Field: bits, encoding |
|---|---|
| `RCC_CR` (§8.7.2, 0x000, `0x0000 0001`, pp. 383–386) | `HSEON` 16; `HSERDY` 17 (r); `HSEBYP` 18 ("can be written only if the HSE oscillator is disabled"); `HSECSSON` 19 (rs); `PLL1ON` 24; `PLL1RDY` 25 (r); `HSIDIV` 4:3 |
| `RCC_PLLCKSELR` (§8.7.11, 0x028, `0x0202 0200`, p. 399) | `PLLSRC` 1:0: `00` HSI, `01` CSI, `10` HSE, `11` none ("can be written only when all PLLs are disabled"); `DIVM1` 9:4: `0` prescaler disabled, 1–63 the divisor, reset 32 |
| `RCC_PLLCFGR` (§8.7.12, 0x02C, `0x01FF 0000`, pp. 400–402) | `PLL1FRACEN` 0; `PLL1VCOSEL` 1: `0` wide 192–960 MHz, `1` medium 150–420 MHz; `PLL1RGE` 3:2: `00` 1–2, `01` 2–4, `10` 4–8, `11` 8–16 MHz; `DIVP1EN` 16, `DIVQ1EN` 17, `DIVR1EN` 18 (reset 1) |
| `RCC_PLL1DIVR` (§8.7.13, 0x030, `0x0101 0280`, pp. 403–404) | `DIVN1` 8:0 = N − 1, N 4–512 ("Others: wrong configurations"); `DIVP1` 15:9 = P − 1, "odd division factors are not allowed", `0` = ÷1, `2` "Not allowed"; `DIVQ1` 22:16 (÷1–÷128, no parity rule); `DIVR1` 30:24 (`0` not allowed) |
| `RCC_PLL1FRACR` (§8.7.14, 0x034, p. 405) | `FRACN1` 15:3; VCO = ref × (DIVN1 + FRACN1/2¹³) |
| `RCC_CFGR` (§8.7.7, 0x010, `0`, pp. 391–393) | `SW` 2:0 and `SWS` 5:3: `000` HSI, `001` CSI, `010` HSE, `011` PLL1 (`pll1_p_ck`); `STOPWUCK` 6 ("also used as emergency clock for the Clock Security System on HSE"); `TIMPRE` 15; `MCO1PRE` 21:18; `MCO1` 24:22 |
| `RCC_D1CFGR` (§8.7.8, 0x018, p. 394–395) | `D1CPRE` 11:8 (no ÷32), `D1PPRE` 6:4, `HPRE` 3:0 |
| `RCC_D2CFGR` (§8.7.9, 0x01C, p. 396) | `D2PPRE2` 10:8, `D2PPRE1` 6:4 |
| `RCC_D3CFGR` (§8.7.10, 0x020, p. 397) | `D3PPRE` 6:4 |
| `RCC_D1CCIPR` (0x04C, p. 412) | `CKPERSEL` 29:28: `00` `hsi_ker_ck` (reset), `01` CSI, `10` `hse_ck` |
| `RCC_D2CCIP1R` (0x050, p. 414) | `SPI123SEL` 14:12: `000` **`pll1_q_ck` (reset)**, `100` `per_ck`; `SPI45SEL` 18:16: `000` APB (reset), `011` `hsi_ker_ck`, `101` `hse_ck` |
| `RCC_D2CCIP2R` (0x054, pp. 416–417) | `USART16SEL` 5:3 and `USART234578SEL` 2:0: `000` `pclk` (reset), `001` `pll2_q`, `010` `pll3_q`, `011` `hsi_ker_ck`, `100` CSI, `101` LSE. **No HSE option.** `I2C123SEL` 13:12: `00` `pclk1` (reset), `10` `hsi_ker_ck` |
| `RCC_D3CCIPR` (0x058, pp. 418–420) | `SPI6SEL` 30:28 (`101` `hse_ck`), `I2C4SEL` 9:8, `LPUART1SEL` 2:0 (reset `pclk4`/`pclk_d3`) |
| `PWR_D3CR` (§6.8.6, 0x018, `0x0000 4000`, p. 310) | `VOS` 15:14: `01` Scale 3 (reset), `10` Scale 2, `11` Scale 1; `VOSRDY` 13. VOS0 is `SYSCFG_PWRCR.ODEN` from VOS1 (§6.6.2, pp. 280–281; p. 561) |
| `FLASH_ACR` (p. 201, reset `0x37`) | `LATENCY` 3:0, `WRHIGHFREQ` 5:4. "No check is performed by hardware" |

**Sequence rules** (RM0433 §8.5.5, pp. 347–349; §8.5.2, p. 340):
- `PLLSRC`, `DIVM1`, `PLLCFGR` and `PLL1DIVR` are written before `PLL1ON`. Then wait for `PLL1RDY` ("lower than ±2 %" when it rises).
- The PLL parameters "cannot be changed once the PLL is enabled".
- An unused post-divider has its `DIVyEN` and divider cleared.
- A switch to a source that is not ready "occurs when the clock source is ready".
- `HSERDY`: "the hse_ck clock is not released until this bit is set".
- The voltage scale is raised before the frequency (`PWR_D3CR` text, p. 310).

### 4.2 Limits

| Limit | Value | Cite |
|---|---|---|
| HSE crystal frequency | 4–48 MHz | DS12117 Table 33 (rev Y, p. 125) = Table 130 (rev V, p. 229) |
| HSE bypass frequency | 4 (min), 25 (typ), 50 (max) MHz; `VHSEH` ≥ 0.7 `VDD`, `VHSEL` ≤ 0.3 `VDD`, `tW` ≥ 7 ns | Table 31 (p. 123) = Table 128 (p. 227) |
| PLL reference | 1–16 MHz; wide VCO needs 2–16 MHz, medium 1–2 MHz | RM0433 p. 347 ("lower or equal to 2 MHz, then VCOL"); DS12117 Tables 136/137 (pp. 234–235) |
| `PLL1RGE` ranges | 1–2 / 2–4 / 4–8 / 8–16 MHz. **The boundaries are shared** (an 8 MHz reference is in both `10` and `11`); RM0433 does not say which is inclusive | p. 402 |
| VCO, wide | 192–960 MHz (RM0433 p. 402; DS rev V Table 136, p. 234). **836 MHz max on rev Y** (DS Table 39, p. 130) | |
| VCO, medium | 150–420 MHz | p. 402; Tables 40/137 |
| `DIVM1` / `DIVN1` / `DIVP1` | 1–63 / 4–512 / even 2–128 (÷1: listed on p. 404, excluded by p. 348's "can only take odd values"; refused, below) | pp. 348, 399, 404 |
| PLL lock time | wide: 50 µs typ, **150 µs max**; medium 100 µs max | Tables 39/136 |
| `fCPU` / `fHCLK` / `fPCLK` max, rev V | VOS3 200/100/**50**; VOS2 300/150/75; VOS1 400/200/100; VOS0 480/240/120 (VOS0: `TJmax` 105 °C, LDO only) | DS12117 Table 111 (cont.), p. 210 (image) |
| The same, rev Y | VOS1/2/3 400/300/200, no VOS0 [inferred: DS rev Y Table 13 has no frequency rows; from Table 39 note 2 and p. 115] | |
| Kernel max per VOS | USART, I2C, SPI4–6, LPUART: 50 (VOS3), 75 (VOS2), 100 (VOS1). Timers: 100/150/200. `SPI1,2,3`: 100/150/200 | RM0433 Table 60, pp. 355–356 (p. 356 image) |
| Flash wait states | VOS2: 0 WS ≤ 55 MHz, 1 WS ≤ 110, 2 WS ≤ 165, 3 WS ≤ 225 (`WRHIGHFREQ` `01`/`10`); VOS1: 2 WS + `WRHIGHFREQ` `10` for ]185; 210] MHz | RM0433 Table 17, p. 160 |
| HSE start-up `tSU` | **2 ms typ, no maximum given**; "can vary significantly with the crystal manufacturer" | DS12117 Table 33/130 note 4 |
| `Gm_crit` max | **1.5 mA/V** (startup) | Table 33/130 |
| HSE drive level | **not stated** in DS12117 Rev 10 | — |

**`DIVP1` ÷1 is contradictory in the RM.** p. 348: "For PLL1, DIVP bitfield can only take odd values" (which excludes field value 0, ÷1); p. 404 lists `0000000: pll1_p_ck = vco1_ck`. The rule should **refuse ÷1** (the stricter reading).
- Re-run without ÷1: no 64 MHz count changes, and every 400 MHz case still has a ÷2 setting (`py/pll-enum-nodiv1.txt`).
- H1 (÷4) and H2 (÷2) are unaffected.

**Two limits are open, for the silicon revision:** wide VCO 836 or 960 MHz, and no VOS0 on rev Y. A Sentinel's silicon revision is not declared anywhere (finding F-REV). H1's VCO (256 MHz from 8 MHz, `DIVN1` 32) and H2's (800 MHz) both fit under 836 MHz, so neither tree depends on the answer.

### 4.3 The rule's shape (designed, not written into a crate)

**Home:** `aethelgard-ir::clock` (a new module beside `silicon`), or `silicon` itself. It shares no code with `aethelgard-svd-hal` (`RUL-20` (1)). `svd-hal` may not depend on it for the words it emits: it computes its own and the rule judges them.

```text
pub struct RccWords {            // every word the bring-up writes, as written
    cr: u32, pllckselr: u32, pllcfgr: u32, pll1divr: u32, pll1fracr: u32,
    cfgr: u32, d1cfgr: u32, d2cfgr: u32, d3cfgr: u32,
    d1ccipr: u32, d2ccip1r: u32, d2ccip2r: u32, d3ccipr: u32,
    pwr_d3cr_vos: u32, flash_acr: u32,
}
pub struct Source { hz_num: u64, hz_den: u64, fast_ppm: u32, slow_ppm: u32 } // exact rational: 25 MHz/3 is 25_000_000/3
pub struct ClockFacts {          // every figure exact: a rational in Hz
    sys_ck, cpu, hclk, pclk1, pclk2, pclk3, pclk4,
    tim_apb1, tim_apb2,          // RM0433 Table 57, both TIMPRE values
    pll1_p, pll1_q, pll1_r, per_ck,
    kernel: BTreeMap<&'static str, Rational>,   // USART1..8, I2C1..4, SPI1..6, by their selects
}
pub fn clock_facts(w: &RccWords, hse: Option<Source>) -> Result<ClockFacts, ClockRuleError>;
```

**Judgements, each a named refusal citing its row of §4.2:**
- reserved encodings;
- `DIVP1` odd;
- `DIVN1` out of range;
- the reference outside `PLL1RGE`'s range;
- the VCO outside `PLL1VCOSEL`'s range, or a VCO select whose input range excludes the reference;
- `PLL1FRACEN` = 1 (refused: unmodelled by the twin, §6);
- `DIVx1EN` = 0 on an output the tree uses;
- `sys_ck`, `hclk` or `pclk` above the `VOS` row's maximum for the declared silicon revision;
- `FLASH_ACR.LATENCY` or `WRHIGHFREQ` below the table's row for `hclk` and `VOS`;
- `HSEBYP` disagreeing with the declared mode;
- `PLLSRC` = HSE with no HSE declared;
- a kernel select naming a disabled or unready output;
- **a peripheral the topology uses whose kernel is the HSI while an HSE is declared.** No kernel may stay on the HSI silently: the rule names each one, and the specification must acknowledge it.

**Arithmetic.** Exact rationals with `u128` numerators and no floating point. The tolerance bounds become `source × (10⁶ ± ppm) / 10⁶`, floored and ceiled exactly as `serial::check_silicon` does today.

**It replaces** the equality test `clock != SolvedClock::reset()` (`lib.rs:4022`). It also replaces the three `aethelgard-ir` copies of the HSI ppm: their `RUL-20` separation becomes one budget function in `ir`, read by the serial, motor and silicon checks. The generator keeps its own copy (copy 4), because that is the independence `RUL-20` requires.

### 4.4 CSS on HSE, read at source

**RM0433 Rev 9 §8.5.3, p. 344, "CSS on HSE" (read at source):**
- "If a failure is detected on the HSE clock, the system automatically switches to the HSI in order to provide a safe clock."
- "The HSE is then automatically disabled, a clock failure event is sent to the break inputs of advanced-control timers (TIM1, TIM8, TIM15, TIM16, and TIM17), and an interrupt is generated."
- "If the HSE output was used as clock source for PLLs when the failure occurred, the PLLs are also disabled."
- "the CSS generates an interrupt which causes the automatic generation of an NMI … The NMI routine is executed indefinitely until the HSECSSF bit is cleared."
- "It is not possible to clear directly the HSECSSON bit by software."
- **Startup:** "At startup, the hse_ck clock is not released until this bit [HSERDY] is set by hardware" (§8.5.2, p. 340).

### 4.5 The exact-case tests, proposed (§1.3.5)

Every word below is decoded by hand against §4.1, with every field cited there [inferred: desk decode; the rule does it in `4240a`'s test].

**ArduPilot's five words** (`4400f` FEEDBACK §2, read back from Renode):
- `PLLCKSELR` `0x0040_8040`: `PLLSRC` `00` (HSI), `DIVM1` 4, `DIVM2` 8, `DIVM3` 4. The reference is 64/4 = **16 MHz**, at `PLL1RGE` `11`'s top endpoint (F-RGE).
- `PLL1DIVR` `0x0109_0231`: `DIVN1` 0x31 + 1 = 50, VCO 800 MHz; `DIVP1` 1 → ÷2 → **400 MHz**; `DIVQ1` 9 → ÷10 → **80 MHz**; `DIVR1` 1 → ÷2.
- `D1CFGR` `0x48`: `HPRE` `1000` ÷2 (`hclk` 200 MHz), `D1PPRE` `100` ÷2, `D1CPRE` ÷1.
- `D2CFGR` `0x440`: `D2PPRE1` = `D2PPRE2` = ÷2 (`pclk` 100 MHz).
- `CFGR` `0x801B`: `SW` = `SWS` = `011` (PLL1); **`TIMPRE` 1**, so `D2PPRE` ÷2 gives timers = `hclk` = **200 MHz**.
- **Expected:** `sys_ck` 400 MHz, timers 200 MHz, `pll1_q` 80 MHz. With `D2PPRE` ÷2, Table 57 gives 200 MHz whether `TIMPRE` is 0 or 1, so this case **cannot** catch `timer_kernel_hz` ignoring `TIMPRE`. A distinguishing case needs `D2PPRE` ÷4 or more (e.g. `D2CFGR` `0x550` with `TIMPRE` 1: timers = `hclk`, not `hclk`/2).

**H1 at the recommended 16 MHz crystal** (the bring-up's written words; reset values elsewhere):
- `PWR_CR3`: the declared supply mode (F-VOS: written once, first).
- `PWR_D3CR` `0x0000_8000`: `VOS` `10`, Scale 2.
- `FLASH_ACR` `0x11`: `LATENCY` 1, `WRHIGHFREQ` `01` (Table 17, VOS2, ]55; 110] MHz).
- `RCC_CR` |= `HSEON` (bit 16), with `HSEBYP` 0 for a crystal or 1 for external; then `HSECSSON` (bit 19) after `HSERDY`; then `PLL1ON` (bit 24).
- `PLLCKSELR` `0x0000_0052`: `PLLSRC` `10` (HSE), `DIVM1` 5 (3.2 MHz), `DIVM2` = `DIVM3` = 0 (unused PLLs off, §8.7.11).
- `PLLCFGR` `0x0003_0004`: `PLL1RGE` `01` (2–4 MHz), `PLL1VCOSEL` 0 (wide), `PLL1FRACEN` 0, `DIVP1EN` 1, `DIVQ1EN` 1, `DIVR1EN` 0.
- `PLL1DIVR` `0x0103_064F`: `DIVN1` 79 (×80, VCO **256 MHz**), `DIVP1` 3 (÷4, **64 MHz**), `DIVQ1` 3 (÷4, `pll1_q` **64 MHz**), `DIVR1` left at its reset 1.
  - **The RM is self-contradictory here.** It says to clear an unused post-divider's bits (p. 347), but `DIVR1` = 0 is "Not allowed" (p. 403). The proposal is to leave `DIVR1` at reset with `DIVR1EN` 0.
- `CFGR` `0x0000_0003`: `SW` `011`; read back `0x1B` when `SWS` follows.
- `D2CCIP1R` `SPI123SEL` `000` and `SPI45SEL` `000`; `D3CCIPR` `SPI6SEL` `000` (reset values, D6).
- **Expected:** `sys_ck` = `hclk` = every `pclk` = timers = USART = I2C = SPI = **64 MHz**; the crystal's ppm on every one.

**H2 at 16 MHz:**
- `PLLCKSELR` `0x52`; `PLLCFGR` `0x0003_0004`;
- `PLL1DIVR` `0x0109_02F9`: ×250, VCO 800; ÷2, **400**; ÷10, **80**;
- `D1CFGR` `0x48`, `D2CFGR` `0x440`, `D3CFGR` `0x40`, `CFGR` `0x3`;
- `PWR_D3CR` `0xC000` (VOS1); `FLASH_ACR` `0x22` (2 WS, `WRHIGHFREQ` `10`, AXI 200 MHz in ]185; 210]).
- **Expected:** 400 / 200 (`hclk`) / 100 (`pclk`) / 200 (timers, `TIMPRE` 0) / 80 (SPI1–3).

**H0 at 16 MHz:** `CFGR` `0x2` (`SW` HSE), `CR` `HSEON`. **Expected:** every clock 16 MHz. The solver panics on it (F-PANIC).

**Refusal cases:** `DIVP1` odd; `DIVN1` 3; a 1 MHz reference with `PLL1VCOSEL` 0; a VCO of 961 MHz; `PLLSRC` HSE with no HSE declared; `DIVQ1EN` 0 while `SPI123SEL` names `pll1_q`; `pclk` 64 MHz at VOS3 (F-VOS); `LATENCY` 0 at 64 MHz/VOS2.

## 5. Predictions, scored

| # | Prediction | Result | Score |
|---|---|---|---|
| P1 | ±50 ppm crystal, 64 MHz, CRSF 416666: margin about 22,650 | `BRR` 154, quantization 2,596, receiver 2,646, peer 8,002, worst 10,648, Table 407 33,300, **margin 22,652** [measured, M2] | **Confirmed** |
| P2 | HSI-PLL at 100/200/240 MHz: 416666 margin about 3,605; 420000 fails at 100 and 200 (about −207) | 416666 **+3,605** at all three in the serial check; 420000 **−207** at 100 and 200, +135 at 240 [measured, M2]. At 240 MHz `solve` itself panics on I2C under the measurement's kernel choice, an artefact (§7.1, F-PANIC), so "solves" there means the serial check holds. 3,605 holds only because Table 406/407's 37,500 row applies when `BRR`'s fraction is 0 (`BRR` 240, 480, 576). **Only a crystal closes both bauds** | **Confirmed** |
| P3 | Integer PLL1 reaches exactly 64 MHz from 8, 16, 24, 25 MHz, not from 8.333 MHz | 8 MHz: 32 settings; 16: 64; 24: 54; 25: 4 (e.g. `DIVM1` 5, `DIVN1` 128, `DIVP1` 10); **8.333 MHz: 0**. 8.333 MHz still reaches 400 MHz (`DIVM1` 1, `DIVN1` 96, `DIVP1` ÷2). Rev Y's 836 MHz VCO changes no conclusion [measured by enumeration against §4.2's cited limits, `py/pll-enum*.txt`] | **Confirmed** |
| P4 | DShot600 bit within ±0.1 ns of 1,656.25 ns; PWM idle 1,000.0 ± 0.1 µs | PWM: **999.950–1,000.050 µs** (from 985.24–1,026.58 µs on the HSI). DShot600: band **±0.084 ns** about **1,671.875 ns**, not 1,656.25. The solved `ARR` is 106, so a bit is 107 ticks, and 1,656.25 ns is 106 ticks [measured, M3] | **Half:** the width holds, the centre was wrong |
| P5 | The Nucleo's `MCO` is HSI/2 of the ST-LINK's MCU | **Not measured.** UM2407 says "fixed at 8 MHz" and no more. Rows B-0 and B-1 are for Scotty | **Open** |
| P6 | Bench ELF reproduces from `cd43b207`; knob compiled in but unset gives the same sha | `375fd31a…9c02` from `main` twice (two paths), then with the knob unset, and with it set to `HSI` [measured] | **Confirmed** |
| P7 | H1: no refusal appears; `PSC`/`ARR`/`BRR` unchanged; no `TIMINGR` moves | No refusal, and every `PSC`/`ARR`/`CCR`/`BRR` and `MBR` unchanged. But **all 99 `TIMINGR` words move**, one field each [measured, M1, M4] | **Half:** wrong on `TIMINGR` |
| P8 | H0 at 8 MHz: a motor fixture gains a refusal | No motor refusal: DShot600 solves `ARR` 12 and holds. Instead, **every solvable fixture panics** in `solve` (I2C, F-PANIC) [measured, M1] | **Refuted** |
| P9 | H2: every motor `PSC`/`ARR` moves, no new refusal | PWM `PSC` 63 → 199; DShot `ARR`/`CCR` 106/40 → 332/125; no refusal [measured] | **Confirmed** |

## 6. The twin once the firmware programs the PLL (§1.7), and ArduPilot (§1.8)

**Source:** Renode `renode-infrastructure` `cd4b002a`, `STM32H7_RCC.cs`.
- The local copy (sha256 `7f10d491…`, 1,860 lines) equals the file fetched by commit.
- `1605`'s three NVIC patches touch no clock code (`4400f` FEEDBACK §1.1).
- The full read, with line numbers and quotes, is `spike/4240/reads/renode-ardupilot.md`.
- The builder re-read L183–188, L24–28 and L1396–1400.

| Question | Answer [read at source] | Line |
|---|---|---|
| `HSERDY`, `PLL1RDY` | **Each reads its ON bit, with no delay** (`valueProviderCallback: _ => hseon.Value`). A `while (!HSERDY)` passes on the first read, and so does `PLL1RDY`; `SWS` mirrors `SW` at once, unchecked against readiness. **No poll can hang, and none waits where silicon would** (up to `tSU` 2 ms typ, no max, plus 150 µs PLL lock) | L185, L197, L219–226 |
| `PWR` (`VOS`, `VOSRDY`) | A Python stub: `D3CR` reads `0x2000` (`VOSRDY` 1), `CSR1` `0x6000`. A `VOS` write is not modelled. `CR2`/`CPUCR` log "Unhandled access" | `stm32h7.repl` L728–745 |
| The HSE frequency | A constructor parameter, `hseFrequency`, default 8,000,000 (L1633), read-only after construction. A `.repl` can set it on the `rcc` line (`hseFrequency: 16000000`, Renode's constructor-parameter syntax [inferred]; the stock line carries only `systick: nvic`). **`HSEBYP` is a tag with no effect** | L26, L36, L186 |
| `sys_ck` | From `SW`: HSI (`HSIDIV`), CSI, HSE, or PLL1 = ref/`DIVM1` × `DIVN1` / `DIVP1`. Refuses `DIVN1` outside 4–512 and odd `DIVP1` (returns null). **The reference-range check is dead code** (`ref < min && ref > max`). `PLL1RGE`, `PLL1VCOSEL` and the `DIVxEN` bits are tags. `FRACN1`/`PLL1FRACEN` are tags. `DIVQ1`/`DIVR1` are stored, never used | L92–135, L122, L291, L301–303, L370 |
| PLL2, PLL3 | Stored, **never computed** | L372–393 |
| Kernel muxes | All four registers are **tags**: they read 0 and drop writes (`D2CCIP2R` reads 0, `4400f` F2) | L394–448 |
| What is pushed | On a change to `CR`, `CFGR`, `D1CFGR` or `D2CFGR` only, it pushes `sys_ck / (D1CPRE × HPRE × k)` (k = RM0433 Table 57's ratio, `TIMPRE` applied) to every registered timer, and `sys_ck / D1CPRE` to SysTick. Capture/compare channels never follow (`4400f` F5). `STM32F7_USART` cannot follow (constructor-only) | L1450–1511, L1398 |
| After a timer's peripheral reset | The timer returns to its `.repl` `frequency` (`LimitTimer.cs` L324), not the RCC's figure | |
| CSS | `HSECSSON` is a set-only flag. **No detection, no NMI, no switch to HSI.** `HSECSSF`/`HSECSSC` are tags | L187, L468, L481 |

**The `INV-07` residue (§1.7.3), stated:**
- Under `4400g`'s (a), every timer and USART is constructed at F, the solved kernel. A bring-up that programs the RCC then pushes, on each `CR`/`CFGR`/`D1CFGR`/`D2CFGR` write, the RCC's *intermediate* figure to every timer counter, and the final figure after `SW`.
- **H1:** the final `sys_ck` is 64 MHz with every prescaler 1, so the final push equals F. Counters and channels agree after the switch.
  - Between the first push and the switch, the counters carry intermediate figures (for H1, HSI 64 MHz, because no prescaler changes, so equal to F throughout).
  - **The generated bring-up starts no timer before the switch** (`ResetClock`'s successor gates `start_outputs`, `board.rs:3142`), so no counter runs in that interval.
  - **The residue vanishes under H1,** provided the PLL reference is an integer in Hz. Renode computes `hseFrequency / DIVM1` in integer arithmetic (L104–129), so 8 MHz with `DIVM1` 3 would give 63,999,936 Hz. The 16 MHz choice (`DIVM1` 5, 3.2 MHz) is exact. The rule should require an integer reference for the twin's sake.
- **H2:** the final push is 200 MHz, which equals F once `4400g` constructs at the solved kernel (200 MHz). The intermediates differ (`4400f` measured 32 MHz at 3.77 µs for ArduPilot), but no generated timer runs then.
  - After a timer's peripheral reset, F is restored, which equals the solved figure. The residue vanishes too, provided F is the solved kernel. It is `4400g`'s (a), "re-check at merge".
- **H0:** `sys_ck` = HSE = 8 or 16 MHz, pushed on `SW`, equal to F. It vanishes too.
- **No tree leaves a residue that `4400g`'s (a) does not already carry,** because the bring-up starts no timer before the switch.
  - The residue that remains is **SysTick**: it is pushed `sys_ck / D1CPRE` (L1398). Under H1 that is 64 MHz, the same as today. Under H2 it is 400 MHz, the CPU clock.
  - The read agent called the missing `HPRE` a modelling inaccuracy. **The builder does not adopt that, and settled it at source:** RM0433 Rev 9 Figure 49 (p. 351) feeds the "CPU Systick clock" from `rcc_c_ck`, after `D1CPRE` and before `HPRE` (480 MHz max), with a ÷8 option. Renode's `sys_ck / D1CPRE` is the processor-clock case (`CLKSOURCE` = 1) [read].
  - DWT's `CYCCNT` counts the core clock, so it follows `sys_ck` the same way.

**What the twin cannot model, per tree (§1.7.4), and so is judged by the rule (§4) and the bench alone:**

| | H0 | H1 | H2 |
|---|---|---|---|
| Crystal start-up time, an HSE that never starts, CSS | rule + bench | rule + bench | rule + bench |
| `HSEBYP` (bypass vs crystal) | tag | tag | tag |
| `PLL1RGE`, `PLL1VCOSEL`, VCO range, `DIVxEN` | n/a | rule only | rule only |
| PLL lock time (150 µs max) | n/a | rule (a declared bound in the wait) + bench | same |
| `VOS`, `VOSRDY`, flash latency | rule only (stub) | rule only (stub) | **rule only**; H2 cannot run without them on silicon |
| Kernel muxes (SPI1–3 on `pll1_q`) | n/a | **tags**: the twin's SPI kernel is what `4400g` declares, not what the firmware selects | same |
| PLL2/PLL3, fractional | not used | not used | not used by H2 as proposed (ArduPilot's own tree uses PLL2/3) |

### 6.1 ArduPilot `Copter-4.7.2` (§1.8)

**Source:** `stm32h7_mcuconf.h`, sha256 `67f48e5f…a426` (checked), checkout HEAD `fb773567` (tag `Copter-4.7.2`). The ChibiOS and Nucleo hwdefs were read from the same commit in the pinned ArduPilot image's layer (path in `reads/renode-ardupilot.md` B0). **Read, not built.**

| `STM32_HSECLK` | PLL1 M/N/P/Q → VCO, `sys_ck`, `pll1_q` | PLL2 Q (USART, SPI4–6) | PLL3 R (I2C) | Lines |
|---|---|---|---|---|
| 0 (HSI) | 4/50/2/10 → 800, **400**, 80 | **72** | 30 | L113–122, L168–183 |
| 8,000,000 | 1/100/2/10 → 800, **400**, 80 | **100** | 32 | L124–130, L184–208 |
| 16,000,000 | 2/100/2/10 → 800, 400, 80 | 100 | 32 | L132–138 |
| 24,000,000 | 3/100/2/10 → 800, 400, 80 | 100 | 32 | L140–146, L209–235 |
| 25,000,000 | 2/64/2/10 → 800, 400, 80 | 100 | 30 | L148–154, L236–253 |
| 32,000,000 | 4/100/2/10 → 800, 400, 80 | 100 | 32 | L156–162 |
| anything else | `#error` | | | |

- **Timers stay 200 MHz** in every branch. **`USART16SEL` stays `PLL2_Q`, but at 100 MHz with any crystal, not 72 MHz** (72 is the HSI branch only). `CKPERSEL` becomes HSE.
- **Bypass:** the hwdef line `define STM32_HSE_BYPASS` (`chibios_hwdef.py` L1005–1011). `OSCILLATOR_HZ` maps to `STM32_HSECLK` (L885).
- **Nucleo hwdefs:** `NucleoH753ZI` declares `OSCILLATOR_HZ 8000000` with **no bypass**, although its board's `PH0` is driven by the ST-LINK `MCO` (§2). `NucleoH743` declares 8 MHz with bypass. This is finding F-NUC (upstream, information only).
- **An HSE that does not start:** ChibiOS `hal_lld.c` L299–300 waits on `HSERDY` **unbounded**, with no timeout and no HSI fallback. The PLL, `SWS` and flash-latency waits are unbounded too. ArduPilot hangs at boot; it does not fly on the HSI.
- **`4400g`'s declared foreign clock (§1.8.2), read on its PR head `afb91036`, "re-check at merge":**
  - `SolvedClock::from_rcc` accepts `PLLSRC` = HSE when `mcu.hse_hz` ≠ 0 (else `HseUndeclared`).
  - For an 8 MHz ArduPilot build it reads `PLLCKSELR` with `DIVM1` 1, giving 8/1 × 100 / 2 = 400 MHz `sys_ck` and 200 MHz timers. Its decode needs no change for an HSE ArduPilot.
  - Its USART kernel cannot be read back (`D2CCIP2R` reads 0). So a declared foreign clock at 8 MHz HSE must declare **100 MHz** USART kernels, where the HSI branch's figure is 72.
- **The re-citation `OscillatorNotCited` asks for (§1.8.3):** the MCU row's `spi_kernel_hz` (`power-parts.json:1273–1313`) becomes, at `OSCILLATOR_HZ` 8 or 16 MHz:
  - SPI1–3 = `PLL1_Q` = **80 MHz**, unchanged (8 × 100 / 10; L184–208);
  - SPI4–6 = `PLL2_Q` = **100 MHz** (was 72; 8 × 75 / 6).
  - The row becomes a map keyed by `oscillator_hz` (§8).

## 7. The trees measured (§1.4–§1.5)

**Method.** A scratch export of `cd43b207` at `~/.cache/aet-4240/scratch` carried a **labelled deviation**, never committed: `knob.patch`, sha256 `6c7812c6…9ac2`, 6 files.
- **The knob.** A compile-time `option_env!("AET4240_TREE")` (`4400f`'s env-clear lesson) routes `SolvedClock::reset()`, all four HSI ppm copies, `RESET_HZ`, `solve::spi_kernel` and the two `silicon` kernel functions.
- **Coverage.** A grep census (567 hits, classified in `spike/4240/measured.md`) covers every copy. A `±0` run then contains neither 64,958,976 nor 62,342,784 anywhere.
- **Acceptance.** Unset, and set to `HSI`, it reproduces `375fd31a`. **"Additive or no-op when unwired" holds** [measured].
- **The only unrouted literal** is `aethelgard-chaos` `campaign.rs:2397` `RESET_KERNEL_HZ`, which no measurement reaches. The build must route it (§8).
- **Who measured.** An Opus subagent, single-threaded on the scratch. The builder re-derived P1, P2's 100 MHz rows, the 22,760 ppm bound, the settle spins (40 ms × 64,003,200 Hz = 2,560,128) and DShot's 107 ticks by hand, and all agree.
- **Disclosure, the subagent's.** One run rebuilt the `main` baseline CLI with the knob set. A read-back caught it, the baseline was rebuilt from the pristine export, and `375fd31a` reproduced again. No reported figure came from that binary (`measured.md` §0).

**The trees as measured.** `hse` 8 MHz ±50 ppm unless named:

| Tree | `SolvedClock` | Kernels |
|---|---|---|
| **H0** | `sys_ck` = HSE, prescalers 1 | every kernel = HSE (SPI1–3 `per_ck` = `hse_ck`, SPI4–6 `hse_ck`) |
| **H1** | HSE → PLL1 → 64 MHz, prescalers 1, `TIMPRE` 0 | timers, USART and I2C 64 MHz (`pclk`); SPI1–3 `pll1_q` 64 MHz; SPI4–6 `pclk` 64 MHz |
| **H2** | HSE → PLL1 → 400 MHz; `HPRE` 2, every `PPRE` 2, `TIMPRE` 0 | timers 200 MHz; USART and I2C 100 MHz (`pclk`, at reset selects, **not** ArduPilot's `pll2_q`/`pll3_r`); SPI1–3 `pll1_q` 80 MHz (ArduPilot's figure); SPI4–6 100 MHz |

**No kernel stays on the HSI** in any tree as measured. `CKPERSEL` is left at `hsi_ker_ck` in H1 and H2, but nothing selects `per_ck`. Named per §1.4.

### 7.1 Fixtures (M1), 31 files

- **Census:** 26 specifications and 5 stored topologies. On `main`, 22 solve and 4 are refused by design (3 `Infeasible`, 1 `ZeroNominalVoltage`).

| Tree | Refusals that lift | Refusals that appear | Words that move vs HSI |
|---|---|---|---|
| **H1** (8, 16, 25 MHz; H1-sized, §7.7) | none | **none** | **only `TIMINGR`**: 99/99, one field each. `0x50d32a40` → `0x50d32a3f` (SCLL 64 → 63), `0x10c7123c` → `0x10c7123b` (SCLL 60 → 59), `0x00b20e2d` → `0x00a20e2d` (SCLDEL 11 → 10). No `PSC`/`ARR`/`CCR`/`BRR`/`MBR` moves (0/33 `MBR`). H1's words are identical at 8, 16 and 25 MHz (426 words compared) |
| **H2** | none | none | every motor `PSC` or `ARR`/`CCR`; every `BRR` (e.g. 556 → 868, 152 → 238); one `MBR` (1 → 2); every `TIMINGR` |
| **H0** at 25 MHz | none | none | every `BRR`, `PSC`/`ARR`/`CCR`, `MBR` and `TIMINGR` |
| **H0** at 8 and 16 MHz | — | **22/22 solvable specifications panic** | (F-PANIC) |

- **The 5 stored topologies** are refused `UnsupportedClock` under every non-HSI tree, because their stored clock is the HSI's. That is a knob artefact, not a finding.

**F-PANIC [measured].**
- **The panic.** At `solve.rs:410` the solver calls `i2c_timing(..).expect("the reset clock gives every I2C peripheral 64 MHz, …")`. Every I2C controller is solved for all three modes, and `FastPlus1M` needs a kernel of at least 17 MHz at its slowest, so `synthesize_timingr` returns `None` below that. The solver then **panics** instead of refusing by name.
- **Reachability.** It cannot happen on `main`, where the clock is always the reset one. It becomes reachable the moment any clock other than 64 MHz is accepted: H0 below 17 MHz, or a `HPRE`/`PPRE` that divides `pclk` low.
- **For `4240a`:** the `.expect` becomes a named refusal (`INV-06`). The same holds for the HSI-PLL 240 MHz case, where `Standard100k`'s `SCLDEL × PRESC` ceiling (256 kernel cycles) is below 1,250 ns at 243.6 MHz. That case is a measurement artefact of a kernel choice, but the same code path.

### 7.2 CRSF and the serial columns (M2)

Measured on `serial-four`, `sentinel-02-shaped` and `sentinel-03`, which give identical CRSF columns, with only the CRSF baud changed:

| Tree | Baud | Kernel | `BRR` | Quant. | Clock term | Receiver | Peer | Worst | Table 406/407 | Margin | Verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|
| HSI | 416666 | 64 M | 154 | 2,596 | 25,894 | 28,423 | 8,002 | 36,425 | 33,300 | **−3,125** | `SerialBaudTolerance` |
| HSI | 420000 | 64 M | 152 | 2,507 | 25,894 | 23,453 | 8,002 | 31,455 | 33,300 | +1,845 | ok |
| **H1 ±50** | **416666** | 64 M | 154 | 2,596 | 50 | 2,646 | 8,002 | 10,648 | 33,300 | **+22,652** | ok |
| H1 ±50 | 420000 | 64 M | 152 | 2,507 | 50 | 2,557 | 8,002 | 10,559 | 33,300 | +22,741 | ok |
| H1 ±200 | 416666 | 64 M | 154 | 2,596 | 200 | 2,796 | 8,002 | 10,798 | 33,300 | +22,502 | ok |
| H1 ±22,760 | 416666 | 64 M | 154 | 2,596 | 22,760 | 25,297 | 8,002 | 33,299 | 33,300 | **+1** | ok |
| H1 ±22,761 | 416666 | | | | | | | 33,300 | 33,300 | 0 | refused |
| H2 ±50 | 416666 | 100 M | 240 | 2 | 50 | 52 | 8,002 | 8,054 | 37,500 | +29,446 | ok |
| H2 ±50 | 420000 | 100 M | 238 | 401 | 50 | 451 | 8,002 | 8,453 | 33,300 | +24,847 | ok |
| HSI-PLL 100 | 416666 | 100 M | 240 | 2 | 25,894 | 25,893 | 8,002 | 33,895 | 37,500 | +3,605 | ok (`serial-four` refused by its **SBUS** port, −594) |
| HSI-PLL 100/200 | 420000 | | 238/476 | 401 | 25,894 | 25,505 | 8,002 | 33,507 | 33,300 | −207 | refused |

- **The bound.** The largest crystal deviation at which 416666 solves at 64 MHz is **±22,760 ppm** [measured: +1 at 22,760, refused at 22,761]. Only the slow side binds, because `BRR`·baud = 64,166,564 > 64 MHz.
- **The margin moves about 1 ppm per ppm of crystal** (slope ≈ 0.997). The crystal is therefore never the binding term; the peer's 8,002 and the quantization are.
- **L753 closes under H1 or H2:** Sentinel-02 can declare 416666, ArduPilot's own baud (`AP_RCProtocol.cpp:344, 351`), with more than 22,000 ppm to spare. The other ports' columns are in `measured.md` M2 (`m2/*.jsonl`).

### 7.3 DShot and PWM (M3)

| Tree | DShot600 `PSC/ARR/CCR` | Bit, slowest–fastest | PWM idle pulse |
|---|---|---|---|
| HSI | 0/106/40 | 1,716,317–1,647,193 ps | 1,026.58–985.24 µs |
| **H1 ±50** | 0/106/40 | **1,671,958–1,671,791 ps** | **1,000.050–999.950 µs** |
| H2 ±50 | 0/332/125 | 1,665,083–1,664,916 ps | same as H1 |
| H0 8 MHz | 0/12/5 | 1,625,081–1,624,918 ps | same as H1 |

- **Under a crystal the band is ±0.084 ns.** The solved bit's own quantization is **+3,125 ppm** (107 ticks for a 106.67-tick bit at 64 MHz), 60 times the crystal's ±50.
- At H2's 200 MHz the quantization falls to −1,000 ppm (333 ticks for a 333.33-tick bit).
- **Q-C stays open** (D9): no source cites what an ESC accepts.

### 7.4 I2C `TIMINGR` and SPI `MBR` (M4)

- **Every `TIMINGR` word moves under H1,** because the judge's extremes go from 64,958,976 / 62,342,784 to 64,003,200 / 63,996,800 Hz. Each moved word still holds `check_i2c`.
  - Standard: the `tHIGH` minimum margin grows from 2.5 ns to 62.3 ns.
  - Fast: SCL max is 396,092 → 395,082 Hz.
  - Fast-mode Plus: SCL max is 999,369 → 984,665 Hz.
- **No `MBR` moves** (0/33). Every `achieved_max_hz` narrows (e.g. 16,239,744 → 16,000,800).
- At ±0 ppm more would move (`MBR` 4 → 3 where the declared speed is exactly kernel/2^(MBR+1)). **A crystal is not the nominal:** `RUL-14`'s point, measured.

### 7.5 The waits (M5)

- **Formula:** `settle_spins(us) = ⌈us × core_fastest / 10⁶⌉` (`svd-hal` `board/firmware.rs:316–321`).

| | HSI (64,958,976 Hz) | H1 ±50 (64,003,200 Hz) |
|---|---|---|
| `sentinel-02-bench`, 20 ms | `settle(1299180)` | `settle(1280064)` |
| `imu-baro-1v8`, 40 ms | 2,598,360 | 2,560,128 |
| SPI EOT delay, `sentinel-02-bench` SPI1 `MBR` 4 | 34 reads | 33 reads (3 rounds of 16 either way) |

- **The twin run, `imu-baro-1v8`'s boot word:** **78,016,480 ns** (HSI firmware) against **76,869,520 ns** (H1-sized firmware), inside its 100 ms window both times. The 1,146,960 ns difference is exactly 38,232 fewer spins × 3 instructions at Renode's 100 MIPS.
- **Label: twin at `main`'s stock platform clock; re-check after `4400g`.** The knob firmware programs no RCC.
- **On silicon,** H1 runs the core at 64 MHz for real, so the spins mean 40 ms there. H1 also adds the `HSERDY` and `PLL1RDY` waits: up to `tSU` (2 ms typ, no max) + 150 µs. The twin counts these as zero, because its RDY bits are immediate.
- **For `2304`:** that is a twin-to-silicon boot divergence of `tSU` + up to 150 µs before `CEN`. `tSU` is 2 ms **typical with no maximum** (DS12117 Table 33/130), so about 21 of its 100 µs quanta typically, and unbounded by any cited figure, and `2304` should name it if it times boot (D10 item 6).
- **Aside:** the CLI's `generate` refuses `imu-baro-1v8` on `main` (`UnknownPart` U6 ICP10111), so the test path (`generate_with`) was used. That refusal is unchanged by this spike.

### 7.6 PH0/PH1 (M6)

- **Reserving PH0+PH1 (crystal) or PH0 alone (bypass) moves no placement in any fixture** [measured].
- **Why** [read]: on the H753ZI, PH0 and PH1 carry only `RCC_OSC_IN`/`RCC_OSC_OUT` and plain GPIO, with no alternate function (`svd-hal` `data/st-open-pin-data/STM32H753ZITx.xml:293–300`).
- So reservation can only ever move a GPIO-role net, and no fixture places one there. `ir/tests/solve.rs:274–291` assigns I2C3 there only on a synthetic pin table.

### 7.7 The bench ELF (M7)

- **`main`:** `375fd31a…9c02` [measured, twice, two paths].
- **Knob unset or `HSI`:** the same [measured].
- **H1-sized ±50:** `b86459a7…2601` [measured]. This is **not an H1 build**: the knob firmware still writes `main`'s SPI selects and no RCC, PWR or flash word. A real H1 build also adds the bring-up words of §4.5 and moves the SPI selects (D6). `diff -ru` of the two generated `board/` trees shows exactly:
  - `settle(1299180)` → `settle(1280064)`, and its doc figure;
  - I2C1 `TIMINGR` `0x50d32a40` → `0x50d32a3f` and `0x10c7123c` → `0x10c7123b` in `board.rs` and `proofs.rs`;
  - the EOT-delay Kani bound 34 → 33 (rounds unchanged).
- No `PSC`/`ARR`/`BRR`, no RCC write and no platform change. **Once a specification declares an HSE, the bench's I2C SCL prediction (`3304.bench.md` L480) moves**, and its SPI band (L482) narrows to 2.000 MHz ± 100 Hz.

## 8. What would move (§1.6)

**Read at source** on `cd43b207`, by a read-only sweep (`~/.cache/aet-4240/reads/move-list.md`). Every line below was re-read; the builder spot-checked `solve.rs:667`, `verify.rs:1822`, `tests/board.rs:408` and `campaign.rs:2397` by hand.
- **Correction to the sweep:** `cli/src/verify.rs:1822–1838` is test code (a reference topology inside a test module), not a production hard-code.

**Columns.** "All" means every tree moves it. **H1** keeps every solved `PSC`/`ARR`/`CCR`/`BRR` (§7); **H2** and **H0** move them.

### `aethelgard-ir`

| What | Where | Trees | What changes |
|---|---|---|---|
| The declaration | `lib.rs:291` `Mcu.hse_hz: u32` (doc 282–283) | All | It becomes D4's object: mode `crystal`/`external`, frequency, part, capacitors. A `u32` cannot carry 8,333,333.3 Hz (F-HZ) |
| Specification schema | `lib.rs:81` (16); rule `lib.rs:42–80` | All | 17 is `4400g`'s, so this is **18**, or **19** if `4240v` takes 18 first (D10). Pinned by `tests/{bench:228, orientation:158, serial:1569, board:482}` and `readme_versions.rs:13,36` |
| Solved schema | `lib.rs:447` (10); reader fallback `lib.rs:964–1007` (`984`: an old file gets `reset()`) | All | 11. Embedded solved JSON regenerates: `twin/src/tests.rs:9`, `twin/tests/spi_platform.rs:14`, `cli/src/renode.rs:3662, 3977`, `cli/src/analog.rs:947`, `pincheck/tests/identity.rs:282` |
| `SolvedClock` | `motor.rs:102–115`, `reset()` 120–131, emitted by `solve.rs:293` | All | It gains PLL1 (`PLLSRC`, `DIVM1`, `DIVN1`, `DIVP1`, `DIVQ1`, `DIVR1`, range, VCO select), the kernel selects, and for H2 `VOS` and flash latency. No field holds these today |
| `timer_kernel_hz` | `motor.rs:145–152` | H2 if `TIMPRE` = 1 | It ignores `timpre`. That only matters for `D2PPRE` ÷4 or more (Table 57), which no proposed tree uses. ArduPilot's `TIMPRE` 1 at ÷2 gives the same 200 MHz |
| `CrystalNotSupported` | def `lib.rs:1505–1508`; sole raise `3107–3111`; tests `ir/tests/motor.rs:172–183`, `ir/tests/solve.rs:286–289` | All | Lifted when a crystal part and its capacitors are declared |
| `UnsupportedClock` | def `lib.rs:1624–1627`; sole raise `4022–4023` (`clock != reset()`); tests `ir/tests/motor.rs:385–395`, `ir/tests/serial.rs:1301–1305`, pin `motor.rs:213` | All | It becomes "not a clock the rule (§4) accepts". The equality test is replaced by the rule |
| `validate` feeds the reset clock | `lib.rs:2728–2731` (`motor_timing`, `serial_timing`) | All | It must feed the declared tree |
| HSI copy 1 | `motor.rs:278–282` (`HSI_FAST_PPM`/`HSI_SLOW_PPM`), read at `motor.rs:337–338`, `serial.rs:25`, `428–437` | All | D3's budget |
| HSI copy 2 | `silicon.rs:43–56` (`fast_hz`, `slow_hz`), read by `check_spi` 190 and `check_timingr` 568 | All | D3's budget, as the rule's own copy (`RUL-20`) |
| HSI copy 3 | `solve.rs:644–656`, read by `solve_mbr` 682, `spi_controller` 699, `synthesize_timingr` 793, 796 | All | D3's budget, as the solver's copy |
| HSI-only kernels | `serial.rs:201` `kernel_hz`; `silicon.rs:671` `usart_kernel_hz`; `silicon.rs:60–77, ~139–150` (`RESET_*`, `per_ck_at_reset`, `kernel_at_reset`, `spi_kernel_hz`); `solve.rs:663–676` `spi_kernel` (**returns `None` unless `source == "HSI"`**); `SpiKernel` `lib.rs:536` (`PerCk`, `HsiKerCk` only) | All | SPI1–3 (written `per_ck` today) move to `pll1_q` and SPI4–6 (written `hsi_ker_ck` today, `board.rs:1957–1964`) move to `pclk`: both are reset values (D6). `SpiKernel` gains `Pll1Q` and `Pclk`. USART and I2C follow `pclk` at their reset selects |
| Error texts naming the HSI | `lib.rs:1883–1908` (`SerialBaudTolerance.hsi_ppm`), `solve.rs:119, 123`, `serial.rs:303, 363–403` | All | Renamed to a clock term. `hsi_ppm` is a serialized field: verdict shape |
| Solver's reserved pins | `solve.rs:15–21` `RESERVED_PINS`; test `ir/tests/solve.rs:274–291` (PH0/PH1 go to I2C3 today) | All | PH0+PH1 (crystal) or PH0 (external). §7 lists what moves |
| MCU ArduPilot row | `power_parts.rs:393–407`; `data/power-parts.json:1264–1313` (`oscillator_hz` 0; SPI1–3 80 MHz `PLL1_Q`, SPI4–6 72 MHz `PLL2_Q`) | All, once a crystal reaches hwdef | Re-cited at the chosen `OSCILLATOR_HZ` (§6). `POWER_PARTS_SCHEMA_VERSION` 4 → 5 if the row becomes a map keyed by oscillator |
| Crystal and capacitor rows | `parts.rs:70` (`PARTS_SCHEMA_VERSION` 10) or `power_parts.rs:35` (4); no crystal row exists | All | A crystal row (frequency, tolerance, stability with range, aging, CL, C0, ESR, drive level) and C0G capacitor rows. D4 decides which table |

### `aethelgard-svd-hal` (the generator)

| What | Where | Trees | What changes |
|---|---|---|---|
| HSI copy 4 and `RESET_HZ` | `board.rs:171–193` | All | The crystal's fastest replaces 64,958,976 Hz |
| `settle` spins | `board/firmware.rs:316–321` (`settle_spins`); golden `tests/board.rs:404–408` (`settle(2598360)`) | All | H1: at 64 MHz × (1 + crystal fast); measured in §7 (M5). H2: the core runs at 400 MHz, so the count scales by about ×6.25 |
| SPI `EOT` delay | `board.rs:643` `eot_delay_reads` | All | It takes both extremes from the HSI today |
| The reset-clock token | `board.rs:1659–1703` (`RESET_CLOCK_FIELDS`, `clock_fields`); `2491–2505` (`ResetClock`, `NotResetClock`); `3156–3200` (`emit_reset_clock`) | All | A solved-clock token produced by the bring-up, after `HSERDY`, `PLL1RDY` and `SWS` |
| Functions taking the token | `board.rs:2740` (I2C `init`), `2845` (SPI `init`), `3142` (`start_outputs`), `3236` (`select_spi_kernels`), `3350` (USART `configure`); main `firmware.rs:942` | All | Each takes the new token |
| The RCC bring-up | none today | All | HSE on (with or without bypass), a bounded wait on `HSERDY`, PLL1 config, `PLL1ON`, a bounded wait on `PLL1RDY`, H2 only (`VOS` then `VOSRDY`, `FLASH_ACR` before the switch), `SW`, wait `SWS`, then the kernel selects. Each wait ends in D5's named boot word |
| Kani oracle | `board.rs:4714–4722` (4720: "RCC is not before plus exactly the solved clock gates and selections"), 4745, 5595; harness `reset_clock_is_ok_exactly_at_reset` `5287–5372` | All | "before plus gates and selects" becomes "before plus the solved RCC words", with the ready polls bounded and modelled nondeterministic |
| Ledger | `svd-hal/src/board/ledger.rs:10–23, 535–560` | All | A row for every RCC field the bring-up writes |
| Goldens | `tests/order_2203.rs:1–110`; `tests/ui/{motors-pwm,serial-four}/board.rs` and `topology.json`; `boards/sentinel-01{,-bench}` (byte-compared by `cli generate_tier1`) | All, only for a fixture that declares a crystal | A specification still declaring the HSI moves no byte (§7, M7) |

### `aethelgard-twin`

| What | Where | Trees | What changes |
|---|---|---|---|
| RCC rule | `serial.rs:1110–1290` (`rcc_rule`, `judge_rcc`; 1259: "not an enable register"); tests `serial.rs:2107–2308` | All | Today any write to `CR`, `CFGR` or a PLL register is a finding. The rule must allow exactly the solved words, once each, in order |
| Cold image | `boot.rs:657–690, 847–848, 917–945` | All | `*ENR` only today. It is unchanged unless the bring-up's words are judged cold |
| Platform | stock `tests/renode-1.17.0/platforms/cpus/stm32h7.repl:513` (`rcc: Miscellaneous.STM32H7_RCC @ sysbus 0x58024400`, no HSE setting); `platform.rs:45` | All | The generated `.repl` sets the RCC's HSE frequency, if Renode takes it (§6) |
| Timer frequency | `timer.rs:31` `TIMER_HZ` 250 MHz (`4400g` replaces this with the solved kernel) | H2 | Under `4400g`'s (a), the solved kernel. H1 keeps F = 64 MHz |

### Other crates

| Crate | Where | Trees | What changes |
|---|---|---|---|
| `aethelgard-hwdef` | `lib.rs:70–73` (`OscillatorNotCited`, sole raise `384–389`, **no test**); `419–422` `OSCILLATOR_HZ {clock.hse_hz}` | All, once a crystal reaches hwdef | The re-citation (§6), and a first test of the refusal |
| `aethelgard-eda` | nothing reads PH0/PH1 or a crystal today. `support.rs` (`judge` 358) is driven by `power_parts` `Support` (`power_parts.rs:411–440`; groups VCAP, NRST, BOOT0 only, `power-parts.json:972–1090`). The MCU symbol has `RCC_OSC_IN`/`RCC_OSC_OUT` (`STM32H753ZITx.kicad_sym:617–650`) | All (crystal mode) | An HSE support group (crystal + 2 C0G), the symbol and footprint vendored (no `Crystal` symbol and no `Crystal_SMD_3225-4Pin` are held today), and routing rules for the oscillator nets (short, guarded, no vias). In external mode, nothing beyond a net to PH0 |
| `aethelgard-chaos` | `record.rs:511–523` (kind 25 `kernel_hz` 518), kinds 27 and 28 (578, 595); `campaign.rs:2397` `RESET_KERNEL_HZ` gating kind 25 (2428) and kind 28 (1995) | All | `RESET_KERNEL_HZ` becomes the solved kernel, carried in the trace. H1 keeps 64 MHz, so no replay figure moves. H2 moves kinds 25, 27 and 28 |
| `aethelgard-cli` | `ledger.rs:418, 509–597` (RCC reset rows; **an unclassified field is a hard error**); `renode.rs:1122, 1192` (the static checks on `topology.clock`); `main.rs:180–186`, `renode/foreign.rs:84` (`simulate --foreign` declares no clock on `main`: `4400g` adds it) | All | Ledger rows for `CR.HSEON/HSEBYP/PLL1ON`, `PLLCKSELR`, `PLLCFGR`, `PLL1DIVR`, `CFGR.SW`, and for H2 `PWR_D3CR.VOS` and `FLASH_ACR` |

**Sentinel-01 does not move** if it keeps `mcu.hse_hz` 0, or D4's equivalent "no HSE". It declares no CRSF port, and M7 shows that a specification declaring the HSI moves no byte. That is ahead of its order gate (roadmap L434).

## 9. What was deliberately not done

- **No crate, model, parts table, golden, board, test, workflow, `ci/`, `container/` or Renode change.** The knob is in the scratch only (sha256 `6c7812c6…`).
- **No twin run of a clock-programming firmware.** The knob firmware programs no RCC, and on `main` the twin's timers run at 250 MHz. §1.7.3 is therefore answered from the source and `4400f`'s measurements, and **is to be re-checked when `4400g` merges.** At this stop `4400g` (#238, head `afb91036`) is open, unmerged.
- **No ArduPilot build at a crystal frequency (§1.8.4).** The branches were read, not built. A build would add one container and confirm only what `mcuconf` states.
- **No C0G load capacitors picked (§1.2.3).** Their value is `2(CL − Cs)`, and `Cs` is not yet bounded: it comes from `extract`'s copper bound plus the OSC pins' capacitance, both build-time figures. The value is 18 pF at `Cs` 3 pF, or 14 pF at 5 pF, for CL 12. A part is picked in `4240c`, against a parts row it would add. The family is C0G/NP0 0402 at ±0.25 pF or ±1 %.
- **Per-port serial columns other than CRSF are not committed.** They are in `~/.cache/aet-4240/m/m2/*.jsonl` (MAVLINK, GPS, SBUS, CONSOLE, per tree). §7.2 commits CRSF's, which is the port §1.5.2 turns on; the others hold in every tree but HSI-PLL 100 MHz's SBUS (−594).
- **No bench work.** No access to t9o777. Rows B-0 and B-1 are for Scotty.
- **No stock at DigiKey or Mouser.** Both are unreadable from Furnace. No workaround was tried.
- **No document from st.com.** AN2867 is Rev 10 from a crystal maker's copy. MB1364 and RN0093 are not held; they are asked for on the bus.
- **USB's clock, the LSE and the RTC:** out of scope (WO).
- **H0 at a frequency between 17 and 25 MHz** (e.g. 24 MHz) was not measured. It would solve, like 25 MHz does, and move every word. D1 does not depend on it.
- **The `UnknownPart` refusal of `imu-baro-1v8` by the CLI's `generate` on `main`** was observed and not pursued. It is outside this spike, and the twin test path that `4230` used still runs.

## 10. Findings

Format: `finding · where · type · recommendation · decision-needed`.

- **F-VOS · the reset clock exceeds DS12117's and RM0433's VOS3 limits · every generated firmware; `aethelgard-ir` has no rule · correctness · RULED (architect, bus 2026-11-19 15:20Z): a real finding; `3304`'s bench is not held; the fix is independent of the HSE.**
  - **The limits** (each read from the page image by the builder):
    - DS12117 Rev 10 Table 111 (continued), p. 210 (rev V): at **VOS3**, `fCPU` 200, `fHCLK` 100, `fPCLK` **50** MHz. Footnote 6: "Maximum APB clock frequency when at least one peripheral is enabled".
    - RM0433 Rev 9 Table 60, pp. 355–356, "Maximum allowed frequency" at VOS3: TIM 100, SPI1,2,3 100, I2C1,2,3 50, I2C4 50, SPI4,5 50, SPI6 50, USART1,6 50, USART2,3 UART4,5,7,8 50, LPUART1 50. The table note: "It is up to the user to respect these requirements".
  - **VOS3 is the reset scale:** "After reset, the system starts on the lowest Run mode voltage scaling (VOS3)" (RM0433 §6.6.2, p. 280). `PWR_D3CR` resets to `0x4000`, `VOS` `01` Scale 3 (§6.8.6, p. 310).
  - **The reset tree against them, as the generated firmware runs it** (HSI 64 MHz, every prescaler 1; SPI selects written per `board.rs:1957–1964`):

    | Clock | At reset | VOS3 max | |
    |---|---|---|---|
    | `fCPU` | 64 | 200 | within |
    | `fHCLK` | 64 | 100 | within |
    | `fPCLK` 1–4 | 64 | **50** | **over** |
    | TIM kernels | 64 | 100 | within |
    | SPI1–3 (`per_ck` = `hsi_ker_ck`) | 64 | 100 | within |
    | SPI4,5 and SPI6 (`hsi_ker_ck`, as written) | 64 | **50** | **over** |
    | I2C1–3 (`pclk1`), I2C4 (`pclk4`) | 64 | **50** | **over** |
    | Every USART (`pclk`) | 64 | **50** | **over** |

    At **VOS2** every one of them is within (`fPCLK` 75, I2C/USART/SPI4–6 75, TIM and SPI1–3 150).
  - **The sequence the fix needs** (RM0433 §6.4.1, p. 262; §6.8.4, pp. 307–308), which is **more than one `PWR_D3CR` write**:
    1. **Program the supply configuration in `PWR_CR3` first.** "The software shall then initialize the system including supply configuration programming in PWR control register 3". Its lower byte "is written once after POR and shall be written before changing VOS level or ck_sys clock frequency" (p. 308). An invalid `LDOEN`/`BYPASS` combination "will lock the register … The system shall be power cycled before writing a new value".
    2. **Wait for `PWR_CSR1.ACTVOSRDY`** (bounded). "As long as ACTVOSRDY indicates that voltage levels are invalid, the system is in Run* mode, write accesses to the RAMs are not permitted and VOS shall not be changed."
    3. **Write `PWR_D3CR.VOS`** = `10` (Scale 2), then **wait for `VOSRDY`** (bounded). Raise the voltage scale before raising the frequency (p. 310).
  - **`PWR_CR3` "is reset only by POR"** (RM0433 §6.8.4, p. 307): reset value `0x0000 0006` on rev Y, `0x0000 0046` on rev V.
    - **Its write-once lock survives the RESET pin and a debugger reset.** After a warm reset, `4240v`'s `CR3` write is silently ignored, so the bring-up must read `CR3` back and accept "already the declared value" rather than refuse.
    - The Kani clause and the twin rule must allow that case.
  - **What `PWR_CR3` holds is board-dependent** (LDO or bypass; RM0433 Table 34). So it is a **declaration**, not an engine constant (`INV-06`). A Sentinel's power tree has the LDO's VCAP support group, and the Nucleo's supply is the board's. ArduPilot writes `LDOEN | USB33DEN` (`4400c` FEEDBACK L30).
  - **Today's firmware writes none of it** (no `PWR` write in `svd-hal`). It has run on silicon in the default configuration (`3302`'s first silicon).
    - **RM0433 says it does not** (adversarial pass, builder re-read). Figure 21 (p. 263) draws `ACTVOSRDY` low through "Default configuration" until "Supply configuration programmed" (steps 5a–5b). Its note 1 says "In Run* mode, write operations to RAM are not allowed".
    - **So a firmware that never writes `PWR_CR3` runs in Run*, writing RAM**: a second departure from the RM, beyond the 50 MHz one.
    - `3302`'s silicon run shows that the board nonetheless booted and ran. What silicon does in Run* is not something this spike measured. Bench row B-2 reads `ACTVOSRDY` after a power cycle.
  - **What the fix moves** (the architect asked):
    - **The bring-up:** three PWR steps, ahead of every peripheral and of any clock change.
    - **The Kani oracle:** `board.rs:4714–4722`'s "RCC is before plus exactly the solved clock gates and selections" gains a PWR clause (`CR3` = declared, `D3CR.VOS` = Scale 2), and the two ready polls become bounded nondeterministic loops.
    - **The ledger:** rows for `PWR_CR3`'s supply bits and `PWR_D3CR.VOS`, classed `ignored-by-model` (the twin's PWR is a stub).
    - **The twin's stub reads `CR3` = `0x0400_0000`** (`LDOEN` 0, `BYPASS` 0) and `D3CR.VOS` = `00` (`stm32h7.repl` L728–745), so a `4240v` read-back of either would see neither supply mode nor Scale 2. The bring-up must judge by its own writes plus the ready bits, or the stub must change (a twin change, in `4240v`'s fence).
    - **The twin's rule:** the twin judges RCC writes only today (`serial.rs:1110–1290`). The PWR stub drops every write and reads `CSR1` `0x6000` and `D3CR` `0x2000` (`stm32h7.repl` L728–745), so `ACTVOSRDY` and `VOSRDY` are always 1. A rule that allows exactly the declared `PWR_CR3` word and `VOS` = Scale 2, once, in that order, is new.
    - **Every generated ELF and golden** (`order_2203.rs`, the committed boards, `375fd31a` and its successor). Under VOS2, **no solved word moves** (the clock is unchanged).
  - **Recommendation:** its own slice, **`4240v`**, landing first, ahead of `4240a`/`4240b` (D10). The architect leans the same way.

- **F-PANIC · `aethelgard-ir` `solve.rs:410` (`i2c_timing(..).expect(…)`) · correctness (`INV-06`), latent on `main` · `4240a` turns it into a named refusal · decision-needed (no).**
  - Below a 17 MHz I2C kernel (Fast-mode Plus), the solver **panics** instead of refusing [measured: H0 at 8 and 16 MHz, 22/22 solvable fixtures, §7.1].
  - It is unreachable while the clock is always the reset one, and reachable the moment any other clock is accepted.
- **F-CITE · WO §7 "no document number and revision" · rule interpretation · decision-needed (yes, with D2).**
  - Abracon's and ECS's sheets carry a printed revision date and no document number. NDK's carry a document code but **no `C0`**. Epson's and YXC's carry neither.
  - **Lenient reading** (a revision date counts): the ABM8AIG and ECX-33B2Q rows are cited, and D2 stands as provisional.
  - **Strict reading** (a document number is required): **no candidate is both cited and computable**, because the only sheets with document codes, NDK's, lack `C0`. D2 then becomes a stocking plan only: obtain NDK's `C0` (or a document-numbered sheet) for an NDK NX3225GA-16 MHz part, which LCSC stocks at 4,690.
  - The builder took the lenient reading and flags it here.
- **F-HZ · `aethelgard-ir` `lib.rs:291` (`mcu.hse_hz: u32`) · specification shape · D4's object carries a rational frequency (`hz_num`/`hz_den`), or refuses a non-integer source · decision-needed (D4).**
  - The Nucleo's `MCO` may be 25 MHz ÷ 3. That is 8,333,333.3… Hz, which no integer field states.
- **F-REV · nowhere · missing declaration · the specification declares the MCU's silicon revision, or the rule takes the stricter row of each pair · decision-needed (with D4).**
  - Two limits differ by revision: the wide VCO maximum (836 MHz rev Y, DS12117 Table 39 p. 130; 960 MHz rev V, Table 136 p. 234) and VOS0's existence (rev V only).
  - Neither H1 (VCO 256 MHz) nor H2 (800 MHz) depends on the answer.
- **F-RGE · RM0433 Rev 9 p. 402 · source ambiguity · the generated tree's judge chooses a reference strictly inside a `PLL1RGE` range, and refuses a generated tree whose reference sits on a boundary. A foreign firmware's boundary reference (ArduPilot's 16 MHz from the HSI, 8 MHz from every HSE branch) is decoded and reported, never refused · RULED (comment 7014986899): scoped to the generated tree's judge.**
  - Adjacent ranges share their endpoints (1–2, 2–4, 4–8, 8–16 MHz), and RM0433 does not say which side is inclusive.
  - ArduPilot's own HSI tree runs a 16 MHz reference (`DIVM1` 4 from 64 MHz), at the top endpoint, and every ArduPilot HSE branch runs an 8 MHz reference (§6.1), so §4.5's ArduPilot case passes the decode.
  - The rule's choice for 16 MHz is `DIVM1` 5 (3.2 MHz, inside `01`'s 2–4 MHz), which also keeps the reference an integer in Hz.
- **F-DEC · `4400g` PR #238 `motor.rs` `from_rcc`/`pll1_p` (read only) · information · `4240`'s judge reads `PLLCFGR` itself on top of the shared decode (D8) · decision-needed (no).**
  - The decode reads no `PLLCFGR`, judging the VCO against the union 150–960 MHz and the reference against 1–16 MHz.
  - It accepts `PLLSRC` = HSE with a declared HSE.
- **F-NUC · ArduPilot `NucleoH753ZI/hwdef.dat` L68–74 (Copter-4.7.2) · upstream, information · nothing for this repository.**
  - It declares `OSCILLATOR_HZ 8000000` without `STM32_HSE_BYPASS`, while the board's `PH0` is driven by the ST-LINK `MCO` (UM2407 §7.5.1); `NucleoH743` declares bypass.
- **F-NDK · NDK NX3225GA/GD sheets (2008E) · source gap · no `C0`, so `gmcrit` cannot be computed · decision-needed (no; the stocking plan, §3.3).**
  - These are the best-stocked LCSC parts.
- **F-AN · AN2867 held at Rev 10 (2015) only · source gap · the current revision dropped on the bus before D2 is ruled · decision-needed (no).**
- **F-DL · DS12117 Rev 10 · source gap · drive level becomes a first-board bench row (AN2867 Rev 10 §3.5.2) · decision-needed (no).**
  - The HSE drive level is not stated, so it cannot be judged from documents.
- **F-REN · Renode `STM32H7_RCC.cs` L122 (`cd4b002a`) · upstream, information · the rule (§4) judges the reference; record it beside `4400f` (b)'s upstream row (roadmap L818).**
  - The PLL reference-range check is dead code (`ref < min && ref > max`).

## 11. Decisions

Each carries a recommendation and its reason. **D2 is Scotty's**; the rest are the architect's.

- **D1, the tree.** Recommended: **H1, HSE → PLL1 → 64 MHz `sys_ck`, every prescaler 1, `TIMPRE` 0, plus `VOS` Scale 2 (F-VOS).** The reasons, each measured unless labelled:
  - **What moves.** It moves the fewest solved words of any HSE tree: no `PSC`, `ARR`, `CCR`, `BRR` or `MBR` in any of 22 fixtures, and only one field of each `TIMINGR` (§7.1). `4400g`'s F stays 64 MHz.
  - **CRSF.** It closes Q-B: 416666 solves with +22,652 ppm at ±50 ppm, and the crystal could be off by ±22,760 ppm before it failed (§7.2).
  - **The twin.** The `INV-07` residue of §6 vanishes: the final RCC push equals F, and no generated timer runs before the switch.
  - **The PLL.**
    - H1's **solved peripheral words** are independent of the crystal frequency: identical at 8, 16 and 25 MHz [measured]. Its PLL words are not (`DIVM1`/`DIVN1`/`DIVP1` differ per crystal, §4.5, §2).
    - The VCO (256 MHz) is under both revisions' maxima (F-REV), and every reference can sit strictly inside a `PLL1RGE` range (F-RGE).
    - It needs VOS2, not VOS1/0. **Flash:** RM0433 Table 17 (p. 160) gives, for VOS2 and an AXI clock in ]55; 110] MHz, `LATENCY` 1 WS with `WRHIGHFREQ` `01` [read].
      - The reset value `0x37` (7 WS, `WRHIGHFREQ` `11`, p. 201) is not a row of that table [read].
      - RM0433 p. 160 says the default 7 WS gives "a safety margin", so staying at reset is permitted; writing the row is a speed-up. The recommendation is to write the row before the switch, for the twin's ledger to name one value [read: p. 160].
      - The bring-up programs no flash, so `WRHIGHFREQ` (a programming delay) is written for completeness, not need [inferred].
  - **Against H2:**
    - It moves every motor word, every `BRR`, a `MBR` and every `TIMINGR`.
    - It needs VOS1 (400 MHz), flash 2 WS + `WRHIGHFREQ` `10` (RM0433 Table 17, ]185; 210] MHz AXI), and `HPRE`/`PPRE` ÷2, all invisible to the twin.
    - It buys nothing the bring-up firmware uses. ArduPilot, which flies, already runs its own 400 MHz tree from the same crystal (§6.1).
  - **Against H0:**
    - At 8 or 16 MHz the solver panics on every fixture (F-PANIC; Fast-mode Plus needs 17 MHz).
    - At 25 MHz it moves every word, and the stocked 25 MHz parts fail the oscillation rule at CL 18 pF (§3.1).

- **D2, the crystal (Scotty's choice). PROVISIONAL** (the AN2867 ruling, §3; F-CITE). Recommended: **16 MHz, 3225 four-pad, CL ≤ 12 pF, automotive temperature: Abracon ABM8AIG, ordered as `ABM8AIG-16.000MHZ-12-2Z-T3`.** That string is **constructed** from the ordering table (p. 2) and not seen in a distributor listing.
  - **Oscillation:** `gmcrit` 0.555 mA/V, 2.7× under DS12117's 1.5 mA/V (AN2867 Rev 10 §3.4).
  - **Budget:** ±72 ppm cited (±20 at 25 °C, ±50 over −40…+125 °C, ±2 first year), plus a declared pull bound.
  - **Load capacitors:** C1 = C2 = 18 pF at `Cs` 3 pF, or 14 pF at 5 pF. Two C0G 0402 parts, `Cs` bounded by `extract`.
  - **Why 16 MHz and not 8:**
    - The cited 8 MHz ESRs are 400–500 Ω, so an 8 MHz ABM8AIG passes only below CL ≈ 15 pF, with less room than 16 MHz at the same CL (1.51× against 2.7× at CL 12).
    - The one fully cited 8 MHz part LCSC stocks (CL 18 pF) **fails** at 2.02 mA/V.
    - 16 MHz is an ArduPilot branch with the same PLL outputs as 8 MHz (§6.1).
    - It reaches 64 MHz with an integer reference strictly inside a `PLL1RGE` range (`DIVM1` 5, 3.2 MHz; the words are in §4.5).
  - **Conditional:**
    - on Scotty's hand read of DigiKey/Mouser stock (§3.3);
    - on the current AN2867 revision not changing the rule (Rev 10 held);
    - on a drive-level measurement on the first board (AN2867 §3.5: not computable from datasheets).
  - **Second maker:** ECS-160-8-33B2Q-VY-TR3 (5.3×, but ±150 ppm stability).

- **D3, the ppm budget.** Recommended: `tol₂₅ + stab(T) + aging(life) + pull` per side (§3.2).
  - **Cited from the part:** tolerance, stability over its stated range, aging.
  - **Declared:**
    - **the operating temperature range** (a new declaration; today junction temperature is "bounded by no declaration", `motor.rs:274–276`), checked to lie inside the part's range;
    - **the service life** (8,000 h, `4300d` D-B4);
    - **the pull bound**, because no held datasheet states pullability.
  - **One budget function in `aethelgard-ir`** replaces the three `ir` copies of the HSI ppm. The generator keeps its own copy (`RUL-20`).
  - **In bypass**, the source's declared ppm alone, with a citation (for the Nucleo, RN0093 and B-1).

- **D4, the specification's shape.** Recommended: **replace `mcu.hse_hz` with an optional object `mcu.hse`**, `null` meaning no HSE:
  - `{ "mode": "crystal", "hz": 16000000, "part": "<crystal row>", "load_caps": ["<C row>", "<C row>"], "pull_ppm": N, "temperature_c": [lo, hi], "life_h": 8000 }`;
  - or `{ "mode": "external", "hz_num": 25000000, "hz_den": 3, "ppm": N, "source": "<citation>" }`.
  - **Why an object:** a bare `u32` cannot state 8,333,333.3 Hz (F-HZ), cannot say crystal or bypass, and cannot carry the budget's declarations.
  - **Why `null` and not 0:** absence is explicit (`INV-06`).
  - **Schema:** `SPECIFICATION_SCHEMA_VERSION` **18** after `4400g`'s 17, by the numbering rule (`lib.rs:42–80`). It is **19** if `4240v` lands first with its own bump for the supply declaration (D10). `SOLVED_SCHEMA_VERSION` 11 (`SolvedClock` gains the PLL, the selects and `VOS`).
  - `mcu.hse_hz` 0 in a version-17 file reads as `null`. A non-zero value was always refused, so no file holds one.

- **D5, an HSE that does not start.** Recommended: **halt with a named boot word, never fall back to the HSI.**
  - **Boot:** the bring-up waits on `HSERDY` with a bound (the declared crystal's start-up maximum; DS12117 gives only "2 ms typ", so the bound is declared, `RUL-14`), then on `PLL1RDY` (150 µs max, DS12117 Table 39/136), then on `SWS`. Each timeout ends in a distinct boot word (`HSE_NOT_READY`, `PLL1_NOT_LOCKED`, `SW_NOT_TAKEN`) and a halt.
  - **CSS:** set `HSECSSON`. In hardware, CSS **itself is a fallback:** on an HSE failure it switches to the HSI, disables the PLLs, and breaks TIM1/8/15/16/17 (RM0433 §8.5.3, p. 344; quoted in §4.4).
    - The emergency clock is the HSI while `RCC_CFGR.STOPWUCK` = 0 (reset), and the CSI if it is 1: "The selected clock is also used as emergency clock for the Clock Security System on HSE" (§8.7.7, p. 393). The bring-up leaves `STOPWUCK` at 0.
    - The recommendation is that the NMI it raises halts with a named fault word, motor outputs held at their break state, and never continues on the HSI.
    - **The break reaches only TIM1/8/15/16/17.** A motor on TIM2–5 gets no hardware break, so the NMI handler must drive those outputs to their safe level itself, and the verdict states which motors have no CSS break. The committed `motors-pwm` board drives TIM1 (`svd-hal` `tests/ui/motors-pwm/board.rs`), so it is covered.
    - With CSS set, a silent continuation on the HSI is impossible: the NMI repeats until `HSECSSF` is cleared. Without CSS, a dead HSE would stop the core clock entirely.
  - **ArduPilot does the same at boot** (an unbounded `HSERDY` wait, §6.1), so the two agree on "never fly on the HSI".
  - **The twin cannot model either** (immediate RDY, CSS a bare flag, §6). The rule judges the bound's presence, and the bench measures it (a bypass board with SB45 off would show `HSE_NOT_READY`).

- **D6, the kernel sources.** Recommended per peripheral, constrained by §6's tags:
  - **USART1–8:** `pclk` at its reset select (the only HSE-derived choice; `USARTxSEL` has no `hse_ck`).
  - **I2C1–4:** `pclk` at reset select.
  - **TIM:** the Table 57 kernel.
  - **SPI1–3:** today the generator **writes** `SPI123SEL` = `100` (`per_ck`) with `CKPERSEL` at reset = `hsi_ker_ck` (`svd-hal` `board.rs:1957–1964`). Under H1, move to `SPI123SEL` = `000`, `pll1_q`, the register's reset value, with `DIVQ1` = `DIVP1`, so `pll1_q` = `sys_ck` = 64 MHz. This moves no `MBR`, and the bring-up then leaves `SPI123SEL` at reset, which the twin's tag cannot misreport. The alternative, `CKPERSEL` = `10` (`hse_ck`), runs SPI at 16 MHz and moves every `MBR`.
  - **SPI4–6:** today it is written `011` (`hsi_ker_ck`, `board.rs:1963–1964`). Under H1, move to `000` (`pclk2`/`pclk4`, the reset value) = 64 MHz: no `MBR` moves. `101` (`hse_ck`) is the other choice and moves `MBR`.
  - **`CKPERSEL`:** left at reset (`hsi_ker_ck`) and unused. **No kernel stays on the HSI silently:** the rule (§4.3) names any peripheral whose select resolves to `hsi_ker_ck` while an HSE is declared.
  - Because the twin's muxes are tags, **every select that differs from reset is judged by the rule alone.** The recommendation leaves every kernel select at its reset value, so the twin's tags (which read 0, the reset value) report the truth by construction.

- **D7, the bench.** Recommended:
  - **(a)** Read B-0 and B-1 first (§2).
  - **(b)** `sentinel-02-bench` **does not move until `3304`'s bench has run on `375fd31a`.** M7 proves a specification still declaring no HSE moves no byte.
  - **(c)** Before `4500`, a **bench of the bring-up in bypass** on the Nucleo: a scratch `sentinel-02-bench` variant with `mcu.hse = external, 8 MHz` (if B-1 reads 8.000 MHz).
    - **Its PLL words,** an integer reference strictly inside a range (F-RGE) [inferred: desk decode against §4.1]:
      - `PLLCKSELR` `0x0000_0052`: `PLLSRC` HSE, `DIVM1` 5, so **1.6 MHz**;
      - `PLLCFGR` `0x0003_0002`: `PLL1RGE` `00` (1–2 MHz), `PLL1VCOSEL` 1 (medium, 150–420 MHz, which a ≤ 2 MHz reference requires), `DIVP1EN` and `DIVQ1EN` 1, `DIVR1EN` 0;
      - `PLL1DIVR` `0x0103_069F`: `DIVN1` 159 (×160, VCO **256 MHz**), `DIVP1` 3 (÷4, **64 MHz**), `DIVQ1` 3 (÷4, `pll1_q` 64 MHz), `DIVR1` at reset;
      - `RCC_CR` `HSEBYP` 1, then `HSEON`.
    - The 8 MHz power-of-two divisors land on range boundaries (8, 4, 2 MHz), and `DIVM1` 3 gives a non-integer reference, which Renode would round (§6). So `DIVM1` 5 is the setting that satisfies both.
    - It witnesses the register sequence, `SWS` = PLL1, `PA8` (`MCO1` = `pll1_q` ÷ N, which equals `sys_ck` under H1; `MCO1` cannot select `sys_ck`, RM0433 p. 392, while `MCO2` on `PC9` can) at the predicted frequency, and SPI's SCK at the predicted rate.
    - It does **not** witness a crystal's ppm (the source is the ST-LINK's).
  - **(d)** A crystal's ppm and drive level are first witnessed on Sentinel-02 hardware, which `4500` builds.
  - Fitting X3 (25 MHz) is not recommended. It is soldering on the only bench board, at a frequency flight doesn't use.

- **D8, the rule's home, and one decoder or two.** Recommended: **one decoder function in `aethelgard-ir`** that maps register words to frequencies, shared by `4400g`'s foreign cross-check and `4240`'s rule. On top of it, `4240` adds a **judge** of the limits (§4.2).
  - `RUL-20` is kept: neither is the generator.
  - `4400g`'s `SolvedClock::from_rcc` (PR #238 `afb91036`, read only, "re-check at merge") already accepts `PLLSRC` = HSE given a declared HSE. **That is not a finding against it.** But it reads no `PLLCFGR`: it judges the VCO against the union 150–960 MHz and the reference against 1–16 MHz, and does not read `DIVP1EN`, `PLL1RGE`, `PLL1VCOSEL` or `D3CFGR`.
    - That is right for a foreign decode of Renode's read-back, where those are tags.
    - It is not enough for `4240`'s judge.
  - **So:** the judge reads `PLLCFGR` itself and calls the shared decode for frequencies. This is finding F-DEC, information for `4400g`'s builder through the architect, with no change asked.

- **D9, Q-C (DShot and PWM `holds`).** Recommended: **report only, as ruled (L287), until a receiver's tolerance is declared.** Then `holds` takes the narrowed band against that declared term, as serial's `peer_deviation_ppm` does.
  - **The measurements:**
    - Under H1 ±50 the band is ±0.084 ns on a DShot600 bit and ±0.05 µs on a PWM pulse.
    - The solved DShot600 bit's own quantization is **+3,125 ppm** (107 ticks for 106.67), 60 times the crystal.
  - **So the crystal removes the HSI's −2.6/+1.5 % from the question, and what is left is quantization.** Quantization is a solved fact, judged already ("within half a tick").
  - **Proposed:**
    - a per-output `receiver_tolerance_ppm`, declared with a citation (an ESC firmware's source: BLHeli_32/AM32/Bluejay DShot decoders);
    - `holds` = quantization + crystal band < the declared term;
    - absent the declaration, report the band and say so (`RUL-19`'s habit).
  - No receiver source is held, so no figure is proposed.

- **D10, the split and sequencing.** Recommended:
  0. **`4240v` (new, small), VOS (F-VOS), first, ahead of `4240a`/`4240b`** (the architect's lean, and the builder's):
     - **Specification:** a declared supply configuration (LDO or bypass) for `PWR_CR3`. That is a specification field, so it takes `4400g`'s 17 + 1 = **18**, and `4240a`'s HSE object then takes 19. Alternatively, `4240v` and `4240a` share one bump if they land together.
     - **The rule** judges DS12117 Table 111 and RM0433 Table 60 per declared `VOS`, for every clock including the HSI's, **at the nominal frequency**.
       - At the source's fastest, ArduPilot's and H2's 400 MHz would exceed VOS1's 400 by the crystal's ppm, so the limit is a nominal-frequency limit [inferred: the tables state none]. This is for the architect to rule.
       - The `fPCLK` row is cited for **rev V only**: rev Y's section has no frequency rows (F-REV). The kernel half of F-VOS holds on both revisions through RM0433 Table 60.
     - **The bring-up writes:** `PWR_CR3` (declared), then a bounded `ACTVOSRDY` wait, then `VOS` = Scale 2, then a bounded `VOSRDY` wait, each timeout a named boot word.
     - **It moves** the Kani oracle's RCC/PWR assertion, the ledger, the twin's rule (`PWR_CR3` and `PWR_D3CR` words allowed exactly once, in order), and **every ELF and golden.** No solved word moves.
     - **Bench:** the architect ruled that `3304`'s bench is not held (the sheet will say the run is outside ST's guarantee at VOS3). `4240v` lands after `4400g`, which re-derives the bench ELF anyway.
  1. **`4240a`, `aethelgard-ir`:**
     - the `mcu.hse` object (spec 18, or 19 after `4240v`), crystal and C0G parts rows, the budget (D3), the decode+judge (D8), the solver's pin reservation (PH0+PH1 crystal, PH0 external), and the hwdef re-citation;
     - `OscillatorNotCited`'s first test (it has none, §8);
     - a crystal specification **solves**, and `generate`, `emit` and `simulate` refuse it by name (`HseNotGenerated`, `4300d` slice A's precedent).
     - After `4400g` merges.
  2. **`4240b`, generator and twin:**
     - the bring-up (H1's words, `FLASH_ACR` per Table 17's VOS2 row, D5's waits and words, CSS), the token, the Kani oracle (bounded nondeterministic RDY), the ledger rows, the twin's RCC rule (exactly the solved words, once, in order), and the `.repl`'s `hseFrequency`;
     - `4400g` §1.6's `validate` refusal is lifted here.
  3. **`4240c`, `aethelgard-eda`:**
     - the HSE support group, `Crystal_SMD_3225-4Pin_3.2x2.5mm` and `Crystal_GND24` vendored from KiCad 10.0.7 (blob shas in §1), oscillator routing rules, `Cs` from `extract`.
     - After `1315` (same vendoring path: `PINS`, `FILES`, the README row). `1315` has no branch yet (`git branch -r`, 2026-11-19), so its emitter was read only in its work order.
  4. **The bench of the bring-up (D7 (c))** after `4240b`. Sentinel-02's own crystal is drawn in `4240c` and built by `4500`.
  5. **Heavy tests the build would add (for `1910` D6's planner):**
     - Kani: the bring-up harness, with nondeterministic RDY and bounded waits (one harness, the size of `reset_clock_is_ok_exactly_at_reset`);
     - tier 2: one simulate of a crystal fixture (one twin run);
     - no new campaign.
  6. **`2304`:**
     - Under H1, boot gains `HSERDY` and `PLL1RDY` waits. These are instant in the twin and up to `tSU` + 150 µs on silicon, so the twin's boot-to-`CEN` is shorter than silicon's by that much. At 100 µs quanta that is about 21 quanta typically, and no cited maximum exists (D5 declares the bound).
     - `2304` should name it if it times boot.

## 12. State at the stop

- **Live `main` at the stop:** `cd43b207`, the base, unchanged. **`4400g` (#238) is open, unmerged** (head `afb91036`), so every row labelled "re-check at merge" stands, and `375fd31a` is still the bench ELF to compare against.
- **The architect's rulings received (bus, 2026-11-19 15:20Z):**
  - F-VOS is a real finding; `3304`'s bench is not held;
  - the VOS fix lean is its own first slice (D10 0);
  - rows resting on AN2867, MB1364 or RN0093 are uncited until those land (§3, D2 provisional);
  - the bench rows are for Scotty's next session (t9o777 offline).
- **The builder's corrections on the bus:** the F-VOS sequence (`PWR_CR3` first, not one `D3CR` write), its strengthening (Figure 21, POR-only `CR3`, the twin stub), and two timestamps that were written without reading the clock.
- **Gates (spike-only branch):**
  - `cargo fmt --all -- --check` passes.
  - No `.rs`, `Cargo.*`, workflow or `ci/` file changed (`git diff --name-only origin/main` lists only `spike/4240/` and this file), so clippy and `cargo test --workspace` are not re-run: they test bytes this branch does not touch.
  - `aethelgard-cli --test readme_status`, the one test that reads `docs/`, passes (2/2).
  - Nothing added runs in CI, so no heavy test is added (`1910` D6).
- **Adversarial pass:** a fresh Opus agent read the FEEDBACK against the primary PDFs and the code, not against this spike's reads (`~/.cache/aet-4240/adv/findings.md`).
  - **Nothing broke D1 or F-VOS:** every §4.5 word decodes bit for bit, and H1 is inside every limit read.
  - **Fixed:** S1–S15 and the cited nits, each re-read by the builder where it changed a claim:
    - S1, Figure 21;
    - S2, `PWR_CR3` POR-only;
    - S3, the twin stub;
    - S4, `MCO1` cannot select `sys_ck`;
    - S5, `DIVP1` ÷1 contradictory, refused;
    - S6, "PLL words independent" was false;
    - S7, the integer reference for Renode;
    - S8, ArduPilot's words cannot catch `TIMPRE`;
    - S9, p. 160's "safety margin";
    - S10–S11, the 8 MHz/16 MHz crystal wording;
    - S12, the schema numbers;
    - S13, `tSU` has no maximum;
    - S14, the VOS judge at nominal frequency, for the architect;
    - S15, `fPCLK` rev V only;
    - the nits: Table 406/407, UM2407 p. 10, `firmware.rs:316–321`, the §1 blob shas, the slope ≈ 0.997.
  - **Also fixed:** the stitched PH0 quote (now Table 14's OFF, OFF row), with UM2407's SB45 self-contradiction stated in §2.
  - **Left as stated, with reason:** `probe-rs` command syntax is unverifiable here (no binary on Furnace), and B-0/B-2 say to check `--help`.
  - **Also corrected by the builder before the pass:** ABM8AIG ESR (40 → 50 Ω, 20–26 MHz); F-SPI45 dropped (the generator selects `per_ck`/`hsi_ker_ck` deliberately, `board.rs:1957–1964`); SysTick settled from Figure 49.
- **Reduction pass (diff): −110 net in `spike/4240/` (12 insertions, 122 deletions) · 3 duplicates removed · 0 dead paths · 0 protected · 0 findings.**
  - `py/gmcrit.md` duplicated §3.1's table.
  - The spike copy of `reads/move-list.md` was superseded by §8 and still carried the uncorrected `verify.rs` claim.
  - Two `limits-*.json` files were `sed` derivations of the one kept, now named in their outputs' first line.
- **Scratch:** `~/.cache/aet-4240/` keeps the scratch tree, the knob (`knob.patch`), the harness, `m/` and `adv/` for the ruling. The build targets (`target-scratch` 4.4 GB, `target-harness`, `target-host`, `target-main`) are deleted after the push.
- **Open for Scotty:** B-0, B-1 and B-2; the DigiKey/Mouser hand read (§3.3); D2.
- **Open for the architect:** D1, D3–D10; F-CITE's reading; S14's nominal-or-fastest; whether `4240v` and `4240a` share one schema bump.
