# AET-WO-4240 — Spike: the clock tree from the HSE, with a cited crystal, a PLL judged by an independent rule, and the crystal's ppm in place of the HSI's

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff (`cd43b207` at writing; verify live HEAD).
**Branch:** `wo/aet-4240-hse-clock-spike`.
**Author:** Claude (architect) · **Builder:** strong tier.
**Phase:** P4, the flight path (roadmap §5, `4240` reserved 2026-11-16, `docs/roadmap.md` L334–L338). Before `4500`. Written 2026-11-19. **Cadence: spike, ending in a stop. The crystal part is Scotty's choice; the tree's shape is the architect's ruling.**

**Grounds:**
- **The reservation** (`docs/roadmap.md` L334–L338, from `4230` Q-B and Q-C):
  - `mcu.hse_hz` with a cited crystal and its load capacitors, and the PLL judged by an independent rule;
  - the crystal's ppm replaces the HSI's worst case (−2.6 %/+1.5 %) in every timing check;
  - why: on the HSI, CRSF at TBS's 416666 baud cannot solve, and the ESCs' timing carries the HSI's spread.
- **§7.1 rows this spike answers:**
  - L659: a crystal part with its load capacitors, so `mcu.hse_hz` may be non-zero (`4400` D1).
  - L717: HSI-only clocking; CRSF 416666 refused; ESC receivers' tolerance reported, not judged (`4230` Q-B, Q-C). Owner `4240`, then `4500`.
  - L753: an RC port judged at CRSF 420000 while ArduPilot runs 416666 (`AP_RCProtocol.cpp:344, 351`; `4400b` F3).
  - L771: four HSI copies, two unlabelled (`motor.rs:278`, `svd-hal` `board.rs:173`).
  - L818–L819: Renode's RCC-following (`4400f` (b)) and its dropped D2 kernel-clock muxes.
- **Q-B, measured** (`AET-WO-4230.FEEDBACK.md` L185): at 416666 baud on the 64 MHz HSI, `BRR` 154 is 2,596 ppm slow. With the HSI's slow extreme and the cited CRSF spread (8,002 ppm), the worst case is 36,425 ppm against RM0433 Table 407's 33,300. `solve` refuses it (`SerialBaudTolerance`). 420000 solves with a 1,845 ppm margin. Ruled: the refusal stays (L286).
- **Q-C, measured** (L186): no held source gives a DShot or PWM receiver's tolerance. A PWM idle pulse is 1,026.6 µs at the slow HSI; a DShot600 bit is 1.647 to 1.716 µs. Ruled: report only (L287).
- **The HSI worst case today** (DS12117 Rev 10 Table 36 = Table 133, the −40 °C…TJmax row): −25,894 / +14,984 ppm, in four copies:
  - `aethelgard-ir` `motor.rs:270–282` (`HSI_FAST_PPM`, `HSI_SLOW_PPM`, read by `serial.rs:25` and `serial.rs:402–437`);
  - `aethelgard-ir` `silicon.rs:40–55`;
  - `aethelgard-ir` `solve.rs:640–656`;
  - `aethelgard-svd-hal` `board.rs:172–183`, the generator's own, with the `settle` spins at the fastest HSI (`4230` FEEDBACK L124: 64,958,976 Hz).
- **The reset clock is the only clock, refused three ways:**
  - `validate`: `mcu.hse_hz` ≠ 0 is `CrystalNotSupported` (`aethelgard-ir` `lib.rs:1505–1508`, `lib.rs:3106–3112`; `mcu.hse_hz` at `lib.rs:291`).
  - The solved topology: `clock` ≠ `SolvedClock::reset()` is `UnsupportedClock` (`lib.rs:4022–4023`).
  - `SolvedClock` (`motor.rs:92–152`): source `HSI`, 64 MHz, every prescaler 1, `TIMPRE` 0. Its `timer_kernel_hz` models `TIMPRE` = 0 only (`motor.rs:139–151`). `serial::kernel_hz` assumes `D2CCIP2R` at reset (`serial.rs:194–208`); so does `silicon::usart_kernel_hz` (`silicon.rs:666–676`).
- **The generator owns the reset clock by typestate:**
  - `Rcc::reset_clock` reads `RESET_CLOCK_FIELDS` and yields a `ResetClock` token (`svd-hal` `board.rs:1659–1700`, `board.rs:3156–3193`). The I2C, SPI and pin inits take it (for example `board.rs:2733–2740`, `board.rs:3131–3142`).
  - The Kani oracle asserts RCC is "before plus exactly the solved clock gates and selections" (`board.rs:4714–4722`).
  - The twin's RCC rule allows only the `*ENR` registers, set bits only, and each solved SPI select, whole, once (`aethelgard-twin` `serial.rs:1096–1124`). The boot image writes every RCC enable cold (`boot.rs:657`).
- **The solver leaves PH0/PH1 free** while no crystal is declared (`solve.rs:15–21`; test `aethelgard-ir/tests/solve.rs:274–291`).
- **ArduPilot's oscillator:**
  - `hwdef.dat`'s `OSCILLATOR_HZ` is `clock.hse_hz` (`aethelgard-hwdef` `lib.rs:419–422`).
  - A non-zero value is refused, `OscillatorNotCited` (`lib.rs:70–73`, `lib.rs:384–389`): the MCU row cites ArduPilot's SPI kernel clocks for `OSCILLATOR_HZ` 0 only (`aethelgard-ir` `power_parts.rs:393–407`).
  - With 0, `stm32h7_mcuconf.h` (sha256 `67f48e5f…a426`) selects HSI as the PLL source, L113–121 (`AET-WO-4400b.FEEDBACK.md` L335).
- **The twin's clock** (`4400f`, #229; ruling comment 6996346321; architecture §2.27):
  - Renode's `STM32H7_RCC` computes `sys_ck` from HSI, CSI, HSE (a constructor default of 8 MHz) or PLL1 (`DIVM1`, `DIVN1`, `DIVP1`). `PLL1FRACEN` is a tag. PLL2 and PLL3 are not computed. The kernel muxes are tags (`4400f` FEEDBACK §1.1, §3 (b)(2)).
  - It pushes a frequency to every registered timer, and to SysTick, only on writes to `RCC_CR`, `CFGR`, `D1CFGR` and `D2CFGR`. A capture/compare channel never follows (F5). `STM32F7_USART` cannot follow at all.
  - `4400f`'s `INV-07` table, (a)(2): "a generated firmware that programmed its clock would get counters at the RCC's figure and channels at F … Today `validate` refuses any clock but the reset one, so this is latent; a clock-programming engine must handle it." **This work order is that engine.**
- **`AET-WO-4400g` is being built** (`docs/work-orders/AET-WO-4400g.md`):
  - It sets every timer and USART frequency at construction, from `SolvedClock::timer_kernel_hz` and `serial::kernel_hz`.
  - It adds a declared foreign clock beside `mcu.hse_hz`, and takes `SPECIFICATION_SCHEMA_VERSION` 16 → 17 (`lib.rs:81`).
  - It decodes `PLLCKSELR`, `PLL1DIVR`, `D1CFGR`, `D2CFGR` and `CFGR` per RM0433 for the foreign cross-check (§1.2).
  - It adds a test that `validate` refuses a programmed clock, naming the RCC push (§1.6). **`4240` is where that refusal would be lifted.**
- **The bench:** `AET-WO-3304` stopped with the bench ELF `375fd31a…9c02` from `examples/sentinel-02-bench.json`, verify passing, on the NUCLEO-H753ZI (`AET-WO-3304.FEEDBACK.md` L83, L120).
  - Its SPI prediction is a band over the HSI's tolerance: 1.948 to 2.030 MHz (`AET-WO-3304.bench.md` L482).
  - Its SCL bound is taken at the HSI's fastest, 64.958976 MHz (L480).

> **What this is:**
> - the crystal (or crystals) chosen by evidence, each with its datasheet, its ppm budget and its oscillator margin;
> - the NUCLEO-H753ZI's own HSE source, read at source, because the bench board decides what the bench can witness;
> - the independent PLL rule designed, cited per register and per limit to RM0433;
> - the candidate trees measured in scratch: which solves move, which refusals lift, which bands shrink;
> - every crate, golden, board and test that the build would move, listed;
> - what the twin can and cannot model once the firmware programs the PLL;
> - the build's split and sequencing against `4400g`, `3304`'s bench and `4500`.
>
> It is NOT:
> - the build. It would touch `aethelgard-ir` (a schema bump that collides with `4400g`'s 16 → 17), `aethelgard-svd-hal`'s RCC bring-up and its Kani oracle, `aethelgard-eda` (the crystal and its capacitors drawn), the twin's clocks (`4400g`'s ground), and the generated firmware's ELF, which `3304`'s bench predictions rest on;
> - the crystal choice. That is Scotty's;
> - a change to any crate, model, parts table, board, golden, test, workflow or CI file;
> - USB's clock (`4300e1` measures HSI48 + CRS first, L659), the LSE, or the RTC;
> - Renode's RCC-following (`4400f` (b)): upstream, backlog (L818).

## 1. Spike

Every figure is cited or measured, and labelled which. `RUL-14`: no nominal stands in for a minimum. Scratch lives in `~/.cache/aet-4240/`; evidence worth keeping goes in `spike/4240/`.

### 1.0 Read at source, then predict

1. **Read before any run:**
   - RM0433 (the revision the repository cites, Rev 9; say if a later one changes a limit) §8, for every register and limit in §1.3;
   - DS12117 Rev 10: the HSE oscillator's characteristics (frequency range, `Gm_crit` max, start-up time, drive level), the bypass input's characteristics, and the `sys_ck` maximum per voltage scale and silicon revision;
   - ST AN2867 (oscillator design guide; cite its revision): the gain-margin rule, the load-capacitance formula, drive level, layout rules;
   - UM2407 Rev 6 (held, sha256 `b493e14f…a4cd`, `3304` FEEDBACK L19) and the MB1364 schematic: the HSE source (§1.1);
   - Renode `STM32H7_RCC` at the pinned build (`cd4b002a` plus `1605`'s three patches; `4400f` FEEDBACK §1.1): §1.7's questions;
   - ArduPilot `Copter-4.7.2`'s `stm32h7_mcuconf.h` (sha256 `67f48e5f…a426`): its HSE branches (§1.8).
2. **Predictions, committed before any measured run** (`4400f` §1.2 is the precedent). The architect's desk figures below are predictions to score, not results:
   - P1: on a crystal of ±50 ppm total at the 64 MHz kernel, CRSF 416666 solves with about 22,650 ppm of margin (`BRR` 154; quantization 2,596; peer 8,002; Table 407 33,300) [inferred, desk arithmetic with `serial::check_silicon`'s formula].
   - P2: a PLL from the HSI does not close Q-B. At 100, 200 or 240 MHz kernels, 416666 solves (margin about 3,605 ppm) but 420000 fails at 100 and 200 MHz (about −207 ppm) [inferred]. Only the crystal closes both bauds.
   - P3: with an integer PLL1 and the limits of §1.3, 64 MHz exactly is reachable from 8, 16, 24 and 25 MHz, and unreachable from 8.333 MHz (25 MHz ÷ 3) [inferred; the limits are not yet cited].
   - P4: DShot600's bit band shrinks from 1.647–1.716 µs to within ±0.1 ns of 1,656.25 ns, and PWM's idle pulse from up to 1,026.6 µs to 1,000.0 ± 0.1 µs [inferred].

### 1.1 The bench board's HSE (NUCLEO-H753ZI)

1. **Read UM2407 and MB1364 at source.** State, with section and page:
   - what drives `PH0`/`OSC_IN` by default: the ST-LINK-V3E's `MCO`, through which resistor and solder bridge;
   - the frequency, and **what it is derived from**. ST's article on the STLINK-V3 `MCO` lists HSE/3 (8.33 MHz), HSE/4, HSE/5, HSI/2 (8 MHz) and off, set by the ST-LINK firmware. An `MCO` from HSI/2 carries the ST-LINK MCU's own HSI tolerance, and gains the bench nothing [read at a secondary source; verify at UM2407];
   - whether `X3` (a crystal footprint) is fitted, and what fitting it costs (bridges);
   - that the bench therefore runs HSE in **bypass** (`HSEBYP` = 1), while a Sentinel with a crystal runs in **crystal** mode. The specification must say which.
2. **Read the bench's actual figure without changing it.** The ST-LINK firmware version on t9o777, read by a tool already installed (`probe-rs` 0.32.0, or the ST-LINK's USB descriptors). No ST-LINK firmware upgrade and no `STM32CubeProgrammer` install (no system packages). If it cannot be read, write a bench row for Scotty: the `MCO` frequency measured on `PH0` (CN11 per UM2407) with the analyzer already used in `3304`.
3. **Say what P3 means for the bench.** If the `MCO` is 8.33 MHz, the bench cannot run a 64 MHz integer-PLL tree that a crystal board runs. Name the bench's options: a different `MCO` setting, fitting `X3`, or a bench clock unlike flight's.

### 1.2 The crystal

1. **Candidates.** At least three frequencies among those ArduPilot's `stm32h7_mcuconf.h` supports (§1.8), from at least two manufacturers. 8 MHz in 3.2 × 2.5 mm is the expected centre; say why if not.
2. **For each, the datasheet with document number and revision.** Record:
   - frequency;
   - frequency tolerance at 25 °C;
   - stability over temperature, and over which range;
   - aging, per year or per declared service life (`4300d` D-B4's 8,000 h precedent, L679);
   - load capacitance `CL`, shunt capacitance `C0`, maximum ESR, maximum drive level;
   - pullability (ppm/pF), if stated;
   - package and footprint, and whether KiCad 10.0.7's library carries it (the vendored subset in `aethelgard-kicad-lib/data` does not today);
   - stock and unit price at DigiKey, Mouser and LCSC, with date and source (`4201` §1.3: if Furnace cannot reach one, say so; do not guess).
3. **The load capacitors.** `C1 = C2 = 2(CL − Cstray)` by AN2867, cited. Name where `Cstray` comes from: a declared bound, or `extract`'s copper bound (`1305`), which already bounds a line's capacitance from the routed board. Pick C0G parts that the parts table can hold.
4. **The oscillator margin, an independent rule.** AN2867's `gm_crit = 4 · ESR · (2πF)² · (C0 + CL)²` against DS12117's `Gm_crit` max, and the gain margin AN2867 asks for. Compute it per candidate. Drive level against the crystal's maximum.
5. **The ppm budget.** Propose the formula that replaces `HSI_FAST_PPM`/`HSI_SLOW_PPM`: tolerance + stability over the declared temperature + aging over the declared life + pulling from the capacitors' tolerance and `Cstray`'s uncertainty. Say which terms are cited from the part and which are declarations, and what declares the temperature (today junction temperature is "bounded by no declaration", `motor.rs:274–276`). In bypass, the source's ppm is the `MCO`'s (§1.1).

### 1.3 The independent PLL rule

`RUL-20` (1): a rule in `aethelgard-ir` that shares no code with the generator. Design it; do not write it into a crate.
1. **Cite RM0433 per register and per limit** (section, page, table):
   - `RCC_CR` (`HSEON`, `HSEBYP`, `HSERDY`, `PLL1ON`, `PLL1RDY`, `CSSHSEON`);
   - `RCC_PLLCKSELR` (`PLLSRC`, `DIVM1`);
   - `RCC_PLLCFGR` (`PLL1RGE`, `PLL1VCOSEL`, `PLL1FRACEN`, `DIVP1EN`, `DIVQ1EN`, `DIVR1EN`);
   - `RCC_PLL1DIVR` (`DIVN1`, `DIVP1`, `DIVQ1`, `DIVR1`), and `PLL1FRACR`;
   - `RCC_CFGR` (`SW`, `SWS`, `TIMPRE`), `D1CFGR`, `D2CFGR`, `D3CFGR`;
   - the kernel selects each candidate tree uses (`D1CCIPR.CKPERSEL`, `D2CCIP1R`, `D2CCIP2R`, `D3CCIPR`).
2. **The limits, each cited:** the PLL input range per `PLL1RGE`; the VCO range per `PLL1VCOSEL` and the input it allows; `DIVM`, `DIVN` and each output divider's range; any parity rule on `DIVP1`; `sys_ck`, `hclk`, `pclk` maxima per voltage scale (`PWR_D3CR.VOS`) and per silicon revision; the flash latency (`FLASH_ACR.LATENCY`, `WRHIGHFREQ`) per `hclk` and VOS.
3. **The rule's shape:** a function from the register words to every kernel clock, plus a judgement of each limit, with exact integer arithmetic. Propose its home: `aethelgard-ir::silicon`, beside `check_brr`.
4. **One decoder or two.** `4400g` §1.2 decodes the same five registers for the foreign cross-check. Read where `4400g` puts it (its branch, read only). Say whether `4240`'s rule and `4400g`'s decoder should be one function, and whether `4400g`'s handles `PLLSRC` = HSE. If it does not, that is a finding for `4400g`'s builder, sent through the architect.
5. **An exact-case test, proposed:** ArduPilot's five words (`0x408040`, `0x01090231`, `0x48`, `0x440`, `0x801B`) decode to 400 MHz `sys_ck` and 200 MHz timers (`4400f` FEEDBACK §2), and each candidate tree's words decode to its own figures.

### 1.4 The candidate trees

Measure at least these three. Each is a `SolvedClock` and a set of kernel selects.
- **H0, HSE as `sys_ck`, no PLL.** Every kernel at the crystal's frequency.
- **H1, HSE → PLL1 → 64 MHz `sys_ck`, every prescaler 1.** Every `PSC`/`ARR`/`BRR`/`TIMINGR`/`MBR` that is solved today stays numerically the same; only the RCC bring-up and the tolerance move. `4400g`'s F stays 64 MHz.
  - SPI1–3 today take `per_ck` = HSI (`4230` §1.1). Under H1 they would keep the HSI's spread unless their select moves to `pll1_q` (`SPI123SEL`'s reset value) or `CKPERSEL` moves to HSE. Say which, and what it moves.
- **H2, HSE → PLL1 → 400 MHz `sys_ck`, timers at 200 MHz** (ArduPilot's figures, `4400f` FEEDBACK §1.1). It needs VOS and flash latency, and moves every solved value.

For each tree, list every kernel clock's source after the change. **No kernel may stay on the HSI silently**: name each one that does.

### 1.5 Which solves move (measured in scratch)

On a scratch tree outside the repository, with a labelled deviation (`4400f` §2's knob precedent) that lets `solve` take a candidate `SolvedClock` and a crystal budget:
1. **Every fixture** (the 31 JSON files `grep -l '"serial_ports"'` finds outside `target/`, the motor fixtures among them): `solve` under HSI and under each tree. Record each refusal that lifts and each one that appears.
2. **CRSF:** 416666 and 420000 on `serial-four`, `sentinel-02-shaped` and `sentinel-03` (today all declare 420000 with a peer term of 8,002). P1 and P2 scored. Say whether L753 closes: Sentinel-02 can then declare 416666, ArduPilot's own baud.
3. **The serial check's columns:** quantization, clock term, receiver, peer, worst case, margin, per port, per tree.
4. **DShot and PWM:** `motor::check_silicon`'s slowest and fastest figures per tree (P4 scored). Q-C stays open: the crystal shrinks the band but cites no receiver. Propose whether `holds` should now include the band, against a declared term or a source.
5. **I2C `TIMINGR` and SPI `MBR`:** both are judged at the kernel's extremes (`4230` FEEDBACK L113). List every word that changes when the extremes narrow.
6. **The waits:** `settle` spins at the crystal's fastest instead of 64,958,976 Hz, and the boot word's virtual time on `imu-baro-1v8` (78.1 ms today, `4230` FEEDBACK L289) against its 100 ms window.
7. **PH0/PH1:** solve every fixture with PH0 and PH1 reserved (crystal) and with PH0 only (bypass). List every placement that moves.
8. **The bench ELF stays put.** Rebuild `sentinel-02-bench.elf` from `main` and confirm `375fd31a…9c02`. Then show, by the scratch build's own run, that a specification still declaring HSI moves no byte of it. **Additive or no-op when unwired is the build's first acceptance.**

### 1.6 What would move

List, per tree, every crate, golden, board and test, each with file:line:
- `aethelgard-ir`: the declaration and its schema version (after `4400g`'s 17, so 18 by the numbering rule, `lib.rs:42–80`); `SolvedClock` and `SOLVED_SCHEMA_VERSION` (10, `lib.rs:447`); `CrystalNotSupported` and `UnsupportedClock`; the parts tables (a crystal row, its capacitors, their schema versions); the four HSI copies (L771); `serial::kernel_hz`, `silicon::usart_kernel_hz`, `timer_kernel_hz`'s `TIMPRE` = 0 limit; the solver's reserved pins.
- `aethelgard-svd-hal`: the RCC bring-up (HSE on, ready, PLL, switch, VOS and flash latency if H2); `ResetClock` replaced by a solved-clock token; `RESET_CLOCK_FIELDS`; the Kani oracle's RCC assertion and its bounded ready polls; the order and regeneration goldens (`tests/order_2203.rs`).
- `aethelgard-twin`: the RCC rule (`serial.rs:1096–1124`), the boot image's cold RCC (`boot.rs:657`), the platform's HSE declaration, and §1.7's residues.
- `aethelgard-eda`: the crystal, its two capacitors and their nets drawn; footprints vendored (licence `CC-BY-SA-4.0`, `README.md`); the support gate (`4300g`); routing rules for the oscillator nets.
- `aethelgard-hwdef`: `OSCILLATOR_HZ`, `OscillatorNotCited`, and the MCU row's ArduPilot SPI kernel clocks re-cited for the chosen frequency.
- `aethelgard-chaos`: replay's kind 25 `kernel_hz` (only if a kernel moves).
- `aethelgard-cli`: `verify`'s ledger rows for every RCC field the bring-up writes; `simulate --foreign`'s declared clock, if ArduPilot's moves (§1.8).
- **Boards and goldens:** every committed `boards/` platform and pinned hash that a crystal-declaring fixture moves; the bench sheet's predictions (`3304.bench.md` L480–L482).
- **Sentinel-01:** say whether it moves at all. It needs no CRSF. If it declares no crystal it should not move, ahead of its order gate (roadmap L434).

### 1.7 The twin once the firmware programs the PLL

From the Renode source at the pinned build, and from `4400f`'s measurements:
1. **Does the RCC model `HSERDY` and `PLL1RDY`?** A firmware that polls them must not hang, and must not pass where silicon would wait.
2. **Can the platform set the RCC's HSE frequency** to `mcu.hse_hz` (a constructor parameter), or does it stay at 8 MHz?
3. **What the push means.** On each `CR`/`CFGR`/`D1CFGR`/`D2CFGR` write the RCC pushes its figure to every registered timer and to SysTick; channels stay at F (`4400f` F5). Under `4400g`'s (a), F is the solved kernel. So:
   - if the RCC's final figure equals F, counters and channels agree after the switch;
   - between the first write and the switch, intermediate figures are pushed (ArduPilot: 32 MHz at 3.77 µs, `4400f` FEEDBACK §2). Say whether any timer runs in that interval;
   - a later peripheral reset restores the constructor value, F (`LimitTimer.cs` L324).
   State this as an `INV-07` residue, with the tree that makes it vanish.
4. **What it cannot model:** fractional PLL; PLL2 and PLL3; the kernel muxes (tags; `D2CCIP2R` reads 0, L819); crystal start-up time; an HSE that never starts; CSS. A tree that needs one of these is judged by the rule and the bench alone. Say so per tree.
5. **SysTick and DWT** after the switch (`4400f` F4, `4400g` §1.6).

### 1.8 ArduPilot

1. **Read `stm32h7_mcuconf.h`'s HSE branches.** Which `STM32_HSECLK` values it supports; what PLL1, PLL2 and PLL3 become; whether timers stay 200 MHz and `USART16SEL` stays `PLL2_Q` at 72 MHz.
2. **What `4400g`'s declared foreign clock then reads,** and whether its RCC decode accepts `PLLSRC` = HSE.
3. **The re-citation `OscillatorNotCited` asks for** (`hwdef` `lib.rs:384`): the MCU row's SPI kernel clocks at the chosen `OSCILLATOR_HZ`.
4. **Measure, don't guess,** only if cheap: one scratch ArduPilot build at the chosen `OSCILLATOR_HZ` in the pinned ArduPilot image, one container, under `capped-run`. Otherwise say it was read, not built.

### 1.9 The build's split and sequencing

Propose it. The architect's starting shape, to be argued with:
- **`4240a`, `aethelgard-ir`:** the declaration (mode `crystal` or `external`, frequency, the part, its capacitors), its schema version, the crystal parts rows, the ppm budget replacing the HSI copies, the PLL rule, the solver's pins. A crystal specification solves; `generate`, `emit` and `simulate` refuse it by name until their slices land (`4300d` slice A's `PowerNotEmitted` precedent, roadmap L309).
- **`4240b`, the generator and the twin:** the RCC bring-up, the token, the Kani oracle, the twin's RCC rule and HSE declaration. The `validate` refusal of `4400g` §1.6 is lifted here.
- **`4240c`, `aethelgard-eda`:** the crystal and its capacitors drawn and judged. After `1315`.
- **`hwdef`'s re-citation** rides with whichever slice first lets a crystal reach `hwdef`.

## 2. Stop

**STOP** with `docs/work-orders/AET-WO-4240.FEEDBACK.md` holding:
- the read-at-source tables (§1.0, §1.1, §1.7, §1.8);
- the predictions, scored;
- the crystal table (§1.2): datasheet, figures, margin, ppm budget, stock and price with date and source;
- the PLL rule (§1.3), cited per register and limit;
- the trees measured (§1.4–§1.5), with every refusal that lifts or appears;
- the move list (§1.6);
- findings, as `finding · where · type · recommendation · decision-needed`;
- what was deliberately not done;
- the decisions, each with a recommendation and its reason:
  - **D1, the tree:** H0, H1 or H2, or another measured one.
  - **D2, the crystal:** frequency and part (Scotty's choice; the recommendation is the architect's input to it).
  - **D3, the ppm budget:** its terms, which are cited, which declared, and what declares the temperature and service life.
  - **D4, the specification's shape:** keep `mcu.hse_hz` and add beside it, or replace it with an object; `crystal` and `external` modes; the schema version it takes after `4400g`.
  - **D5, an HSE that does not start:** halt with a named boot word, or fall back to the HSI. A silent fallback substitutes a default (`INV-06`); say what the recommendation does with CSS.
  - **D6, the kernel sources:** per peripheral, constrained by what the twin can model (§1.7.4).
  - **D7, the bench:** the Nucleo's HSE source and figure; whether a bench of the HSE path is needed before `4500`; whether `sentinel-02-bench` moves, and if so only after `3304`'s bench has run on `375fd31a`.
  - **D8, the rule's home,** and one decoder shared with `4400g` or two.
  - **D9, Q-C:** whether DShot and PWM `holds` takes the narrowed band, and against what.
  - **D10, the split and sequencing** (§1.9).

Push one `AET-WO-4240 STOPPED:` commit and open a PR. It is merged as the record after the ruling. Write `~/.cache/aet-4240/pr-title-body.md`.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-02`, `INV-06`, `INV-07`; `RUL-14` (no nominal as a minimum; every value cited), `RUL-16`, `RUL-20` (a rule that shares no code with the generator).
- **Fence:**
  - `spike/4240/` and the FEEDBACK only.
  - Scratch trees, patched CLIs and built ELFs stay outside the repository (`~/.cache/aet-4240/`). A knob in a scratch tree is a labelled deviation, never committed.
  - No change to any crate, model, parts table, golden, board, test, workflow or `ci/` file. No change to `container/` or to any Renode source.
  - The steering documents and `README.md` are protected: findings about them go in FEEDBACK.
  - No ST-LINK firmware change, no `sudo`, no system packages, no credentials.
- **Containers:** one of yours at a time, under `capped-run`.
- **Running beside you:**
  - **`AET-WO-4400g`** (the twin's clocks). It owns the ground you read: the frequency at construction, the declared foreign clock and its RCC decode, schema 17. You share no file with it.
    - **Take on `main` now:** everything read at source; all arithmetic (§1.2–§1.5); the fixture solves; the bench ELF's reproduction (§1.5.8); ArduPilot's `mcuconf` reading.
    - **Re-check after it merges, or take on its head and label "re-check at merge":** any twin run of a clock-programming scratch firmware (§1.7.3). On `main` the twin's timers run at 250 MHz, so such a run would measure the stock platform, not the clock `4240` would ship. Also `4400g`'s decoder (§1.3.4) and its declared foreign clock (§1.8.2).
    - If `4400g` merges while you run, merge `main` into the spike branch and re-take the labelled rows.
  - **`AET-WO-1910` and `AET-WO-1911`** (CI). Nothing you write runs in CI. Name in FEEDBACK each new heavy test the build would add, so the planner gets its weight (`1910` D6).
  - **`AET-WO-2304`** (the battery-sag spike). No shared file. Its lock-step quanta and boot timing meet §1.5.6's waits; say if they interact.
  - **`AET-WO-1315`** (the eda emitter). `4240c` follows it. Read its branch, if one exists, before proposing §1.9's eda slice.
  - (`2304` and `1315` are written on the docs branch, `claude/argon-repo-docs-synthesis-rjrb0d`, and are not yet on `main`. Read them there.)
- One PR, left unmerged.

## 7. Flag-and-stop conditions

- **§2 is the stop.**
- **A datasheet figure cannot be cited** (no document number and revision): the row is marked uncited, and no recommendation rests on it.
- **No crystal passes AN2867's gain margin** at a frequency ArduPilot supports: stop with the table.
- **No candidate is in stock at two distributors:** stop with the table; the recommendation becomes a stocking plan (`4201` precedent).
- **The bench ELF does not reproduce** `375fd31a…9c02` from `main` at the kickoff's SHA: stop. `3304`'s bench rests on it. (`4400g` re-derives that ELF at schema 17 and records the new sha beside the old one. After it merges, compare against that recorded figure, not against `375fd31a`.)
- **A measurement would need a change inside the fence** (a crate, a board, `container/`, Renode, the ST-LINK's firmware): stop and say which, rather than make it.
