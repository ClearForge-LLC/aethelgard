# AET-WO-4410 — hwdef and pincheck: Sentinel-03's battery monitor and crystal reach ArduPilot, SWD stays on, and a chip select's reset level is judged

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4410-hwdef-pincheck`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4, before any Sentinel-03 package. Written 2026-11-23. **Cadence:** a measurement, then a build with one stop.
**Order:** after `4300j` (hwdef's motor line and comment, pincheck's `implied_pupd`). Its two `aethelgard-ir` touches and its dispatch wait for `4240e` to merge (roadmap §5 wave 2: one IR work order and one package dispatch at a time). **Before `4300k`:** that work order's readiness gate needs Sentinel-03's hwdef to generate, and its crystal needs (2) below.

**Grounds** (verified at `54e16367` unless marked):
- **Review 3** (`docs/reviews/2026-11-23-mythos-5-2.md`) A3, A11, A12, A15's `#define`; owners in `….response.md`. **§7.1 rows:** L986 (`4400b` F2), L1005 (`4402` Q5), L1109 (`1218` F4), L1128 (`4408` Q3).
- **A3 cannot be built inside hwdef.** `generate` refuses any solved sense (`hwdef/src/lib.rs:421-423`, `SenseNotEmitted` at `:152-155`) and writes `define HAL_USE_ADC FALSE` unconditionally (`:537`). `SolvedSense` (`ir/src/lib.rs:772-776`) holds `pin`, `position`, `channels`: no divider (`4400b` FEEDBACK:115, `SCALE` `[N]`). hwdef may not read the `Specification` (architecture §3 L530, `RUL-16`). So the divider joins the solved topology.
- **The crystal collides with hwdef.** `generate` refuses `OscillatorNotCited` unless `clock.hse_hz` equals the MCU row's single `ardupilot.oscillator_hz` (`hwdef/src/lib.rs:409-414`), which is `0` (`ir/data/power-parts.json:1371`; `McuArduPilot`, `ir/src/power_parts.rs:582-591`). Scotty ruled a 16 MHz crystal onto Sentinel-03 (roadmap L1125). `crates/aethelgard-cli/tests/fixtures/hse-crystal-16.json` already solves at 16 MHz.
- **A11:** no `PA13`, `PA14` or `SWD` in `hwdef/src/lib.rs`. Pincheck's reset table puts PA13–PA15 at AF0 (`pincheck/src/lib.rs:93-97`), so its reverse check (`:537-541`) skips them with or without the lines: nothing judges SWD today. The solver reserves both (`ir/src/solve.rs:21`).
- **A12:** `pincheck/src/lib.rs:514` tests the mode only; `odr` is decoded at `elf.rs:307` and never read; `pincheck/README.md:56` lists `ODR` as unseen. The expected level exists: each chip select is a solved GPIO output at `high` (`ir/src/lib.rs:604-608`).
- **A15:** `defines()` (`pincheck/src/header.rs:270-280`) returns `None` when `NAME` is followed by `(`, so `#define NAME(x)` reads as undefined.
- **`1218` F4:** `check` reads the ELF (`cli/src/pinout.rs:109`) and `hwdef.h` before `judge` raises `PartMismatch` (`pincheck/src/lib.rs:389-393`) or `refuse_unjudged_gpio` (`:394`).
- **`4400b` F2 looks done:** `LabelTakesAfPrefix` (`hwdef/src/lib.rs:123-126`, `:284-287`, `:441-447`; added by `4404`, `96517013`), tested at `hwdef/tests/generate.rs:429-443` and `cli/tests/hwdef_tier1.rs:298`. **`4402` Q5 looks partly done:** the four `pincheck/fixtures/*.hwdef.dat` headers now name the later additions (lines 4–5) but still open "from the topology of specification `a277c933…`".
- **ArduPilot `Copter-4.7.2` (`fb773567`)** is the source for every line text (`hwdef/README.md:3`). Texts below marked "from memory" are unverified until read there.

> **What this is:**
> - **(1) A3.** `SolvedSense` gains the divider as `power.sense` declares it (top and bottom ohms), a solved-topology bump. hwdef emits the battery monitor from it: the ADC pin line, the voltage pin and scale defines, the monitor default (from memory: `<pin> BATT_VOLTAGE_SENS ADC1 SCALE(1)`, `HAL_BATT_VOLT_PIN`, `HAL_BATT_VOLT_SCALE`, `HAL_BATT_MONITOR_DEFAULT 4`). `HAL_USE_ADC FALSE` stays only when no sense is solved. Pincheck judges the pin and scale defines in `hwdef.h` against the solved sense, by its own arithmetic (`INV-07`).
> - **(2) The crystal.** The MCU row holds ArduPilot's SPI kernel clocks per `OSCILLATOR_HZ`, `0` and `16000000`, each cited at the tag (from memory: a `STM32_HSECLK == 16000000U` branch in `hwdef/common/stm32h7_mcuconf.h`). `hse-crystal-16` then generates.
> - **(3) A11.** Emit `PA13 JTMS-SWDIO SWD` and `PA14 JTCK-SWCLK SWD` (from memory, with the pull ArduPilot gives each). Pincheck names an ELF whose PA13 or PA14 is not AF0 (`RUL-20`: judged, not only generated), citing RM0433's reset values.
> - **(4) A12.** `ODR` compared on every topology GPIO output, the expected bit from its solved level. The README's line is updated.
> - **(5) A15.** A function-like `#define` of any name the judge reads is refused by name.
> - **(6) F4.** Every topology-only refusal runs before the ELF or `hwdef.h` is read.
> - **(7) Q3.** Find every writer of `SYSCFG->PMCR` in ArduPilot and ChibiOS at the tag. Judge PC2SO/PC3SO only from a witness `check pinout` already reads (ELF data or `hwdef.h`).
> - **(8)** F2 verified with a mutant; Q5's first lines reworded or the specification named.
> - **Each refusal or finding comes with a test, and each test with a mutant it kills.**
>
> It is NOT: any specification key or file (Sentinel-03's crystal is `4300k`'s); svd-hal's firmware (its PMCR side is a FEEDBACK row); battery current sensing (none is declared); `defaults.parm` (`4400b` D10); CI's ArduPilot matrix (`1917`, row L1129).

## 1. Build

1. **Measure first.** Commit predictions in `spike/4410/`, then record:
   - `generate hwdef` on every committed specification's topology (`examples/*`, the CLI fixtures, `sentinel-03`, `hse-crystal-16`): ok or the refusal. For Sentinel-03, the next refusal once `SenseNotEmitted` is bypassed in a scratch build;
   - each ArduPilot line text for (1)–(3) and the 16 MHz clock tree, by file:line at `fb773567`, and (7)'s PMCR writers;
   - which committed bytes move. **Predicted:** the four `hwdef/tests/fixtures/*.hwdef.dat` by (3) and (1); every committed topology's `schema_version` by (1); in `fab/`, only `sentinel-01-bench/package/MANIFEST.json`'s `topology_sha256`. No Gerber, drill, BOM, CPL or ELF byte moves: the bench ELF is svd-hal's, not ArduPilot's. It embeds the specification's hash (`svd-hal/src/board/firmware.rs:191`), not the topology's. Verify that. `spike/**` outputs are history and do not move.
2. **Stop with the measurement,** the line texts, (1)'s field shape and (7)'s witness, if any.
3. **Build to the ruling,** goldens moved by their generators. `MANIFEST.json` moves only by a dispatch, after `4240e` merges: ask on the bus.
4. **Prove it.** Sentinel-03's and `hse-crystal-16`'s hwdef generate. On Furnace, build Sentinel-03's ArduPilot once in the pinned image (`~/bin/capped-run`, one at a time), then run `check pinout` on it: no finding. Every CI board still builds and passes.

## 2. Acceptance

- **Sentinel-03's hwdef generates** with its battery monitor, and so does a 16 MHz topology. A topology whose oscillator has no row is still refused by name.
- **Every generated hwdef keeps SWD.** A planted ELF with PA13 as input, or a chip select's `ODR` bit low, is named by `check pinout`, and so are a wrong `HAL_BATT_VOLT_SCALE` and a function-like `#define`, each with its test.
- **A topology refusal is reported** with an unreadable ELF beside it.
- **In `fab/`, only `MANIFEST.json`'s hash lines move.** No pin moves.
- **The CLAUDE.md gates pass, and CI is green.**

## 3. Fences

- **Fence:**
  - `crates/aethelgard-hwdef` and `crates/aethelgard-pincheck`, with their tests, fixtures and READMEs;
  - `cli/src/pinout.rs` (ordering only) and its tests;
  - in `aethelgard-ir`, only `SolvedSense`, `SOLVED_SCHEMA_VERSION` (`lib.rs:585`), the MCU row's `ardupilot` block and `McuArduPilot`, with their version literal;
  - topology goldens and version literals, moved by their generators;
  - `fab/sentinel-01-bench/package/MANIFEST.json`, by dispatch only;
  - `spike/4410/` and FEEDBACK.
- **Not:** the `Specification` and every specification file; svd-hal; eda; the twin; chaos; `verify`; workflows and `ci/`.
- **Running beside you:** the backlog in roadmap §5. Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-4410/pr-title-body.md`.

## 5. Adversarial pass, full

- **A scale ArduPilot reads differently:** decimal rounding of `(top + bottom) / bottom`; a 100 k / 9.09 k divider; a sense pin with only an ADC3 channel.
- **A judge that borrows the generator:** the scale, the SWD pins or the expected `ODR` taken from hwdef's tables or text (`INV-07`).
- **PA13 or PA14 solved to a net,** or an allow-list entry naming them: refused, never overwritten by the SWD line.
- **A `#define` spelled to slip through:** `NAME (x)` (object-like, legal), `NAME/**/(x)`, `%:define NAME(x)`, a line continuation before `(`.
- **A 16 MHz topology judged against the `0` row's kernel clocks,** or the reverse.

## 7. Flag-and-stop conditions

- **After §1.2:** stop with the measurement, the line texts and the field shape.
- **A line or the 16 MHz clock tree cannot be cited at `fb773567`.** Stop with the file.
- **(7) has no witness `check pinout` reads.** Stop with the source lines. Do not invent one.
- **Any `fab/` byte other than `MANIFEST.json`'s hash lines would move, or any pin.** Stop with the diff.
- **The SWD lines fail any CI board's ArduPilot build or `check pinout`.** Stop with the log.
