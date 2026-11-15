# AET-WO-4230 — Silicon facts judged independently: the SPI clock, baud and NSS, I2C timing, the brown-out option byte, and a witness ledger

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4230-silicon`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4. From the second Mythos 5.2 review (`docs/reviews/2026-11-15-mythos-5-2.md`, A1, A3, A5, A8, A10, B2; response `docs/reviews/2026-11-15-mythos-5-2.response.md`). **Cadence: build, with one measured stop (§1.0).**

**Grounds, on `main` (each checked at source by the architect, 2026-11-15):**
- **A1.** The generated board crate enables four `RCC` clocks and writes no kernel-clock selection. RM0433 Rev 9: `SPI123SEL[2:0]` resets to `000`, `pll1_q_ck`, and `PLL1ON` resets to 0. So on silicon SPI1–3 have no kernel clock, and the flash read the twin witnesses cannot happen. `crates/aethelgard-svd-hal/src/board.rs:2014-2027` leaves the baud rate (`MBR`) and slave-select management (`SSM`, `SSI`) at reset.
- **A5, the root cause.** The Kani oracle and the code it proves come from the same generator calls (`board.rs:1792` and `:2536` for pin policy; `:3016-3022` for `TIMINGR`; `:2745-2749` asserts "CFG1 changed beyond DSIZE", which pins the missing baud rate as correct). Renode ignores the same fields (architecture §2.21, §2.22). Such a field passes three gates on the generator's word alone. `INV-07` holds in letter, not independently.
- **The model to follow:** `motor::check_silicon` (`crates/aethelgard-ir/src/motor.rs:298`), an independent integer check in `aethelgard-ir` beside the generator.
- **A3.** `mcu.bor_level` decides `droop-bor*` and `4300d`'s supply floor. Nothing reads or writes the option byte (`ir/src/lib.rs:156-166`: "the option-byte write is out of scope"). RM0433 Table 21's factory default is `BOR_LEV` `00`, and `AET-WO-3302.bench.md` B7 predicts that.
- **A8.** The Fm+ pull-up floor uses 20 mA (`ir/src/electrical.rs:63-68`). DS12117 guarantees that sink only on an FT_f pin with FM+ mode enabled (`SYSCFG_PMCR`), which nothing generates. The Sm/Fm floor is taken at the nominal rail (`solve.rs:615`), not its maximum.
- **A10.** I2C `TIMINGR` values are literals (`svd-hal/src/board.rs:132-136`, "placeholders for a 64 MHz kernel clock").

> **What this is:** every register family the generated firmware writes, or leaves at reset, is judged by a rule in `aethelgard-ir` that does not share code with the generator, or is named in the `verify` verdict as unwitnessed. The SPI controllers get a kernel clock, a baud rate and software slave-select. I2C timing is computed from RM0433's formula. The brown-out level is read back by the firmware and programmed by a step the fabrication package states.
>
> It is NOT:
> - ArduPilot's own clock tree (ChibiOS sets its clocks; `4400b` reports what ArduPilot will use);
> - a PLL configuration (HSI through `per_ck` is enough for every declared speed today; refuse what it cannot reach);
> - the support circuitry (`AET-WO-4300g`) or the fabrication preconditions list (`AET-WO-3303`).

## 1. Build

0. **Measure first.** In the pinned image, on today's reference:
   - what Renode 1.17.0 returns for `FLASH_OPTSR_CUR` and `RCC_D2CCIP1R`, and whether a write to either persists;
   - whether Renode's SPI model honours `CFG1.MBR` or `CFG2.SSM` at all.

   Record each one. They decide which rows of §1.6 the twin can witness. A finding here changes the ledger, not the plan.
1. **The SPI kernel clock.**
   - The solved topology names each SPI controller's kernel clock. The generated `init` selects `per_ck` (`SPI123SEL` = `100`; `per_ck` is HSI at reset, `CKPERSEL` = `00`) before enabling a controller, and `SPI45SEL` the same way if SPI4/5 are used.
   - Extend the run-time `ResetClock` check (`4300a`) to `RCC_D1CCIPR.CKPERSEL` and `RCC_D2CCIP1R`.
   - **The independent rule** in `aethelgard-ir` says which kernel clock each controller has, from RM0433's reset values and the selection the topology declares. It does not read the generator.
2. **The baud rate.**
   - `MBR` is solved from `spi_buses[].speed_hz`: the fastest kernel/2^(MBR+1) at or below the declared speed. A declared speed below kernel/256 is refused by name.
   - The achieved rate is in the solved topology and the verdict. `check speeds` judges the achieved rate, not the declared one (review D8).
   - The independent rule recomputes `MBR` by its own arithmetic.
3. **Slave-select.** In master mode with GPIO chip-selects, `SSM` = 1 and `SSI` = 1 are written.
   - Replace the oracle's "CFG1 changed beyond DSIZE" assertion with the independent rule's expected `CFG1` and `CFG2`.
   - The generator-audit claims of review A16 row 1 (no EOT wait before clearing `SPE`; no delay after an `RCC` enable; motor pins float until `start_outputs`) are verified at source. Fix each one that is real. Name each one that is not, with the line that proves it.
4. **I2C timing.**
   - `TIMINGR` is computed from the solved kernel clock and the declared mode by RM0433's formula, including the analog and digital filter delays and `tSYNC`.
   - The independent rule in `aethelgard-ir` recomputes SCL's low and high periods and checks them against UM10204's minima for the mode. The literal table goes.
   - Bind `I2c::init` to `ResetClock` as the timer `init` is bound.
5. **Fm+ and the pull-up floor (A8).**
   - Every mode's sink-current floor is taken at the domain's maximum voltage, not its nominal.
   - For Fast-mode Plus, generate the `SYSCFG_PMCR` FM+ write for the pins used, and refuse a pin whose pad is not FT_f (`FastPlusOnNonFtfPin`).
   - If ST's open pin data does not carry the pad type, Fm+ takes DS12117's 8 mA row instead, and FEEDBACK says why.
6. **The brown-out level (A3).**
   - The bring-up firmware reads `FLASH_OPTSR_CUR[3:2]` into its boot record. The judge compares it with `mcu.bor_level`. A mismatch is a named failure (`BrownOutLevelNotProgrammed`), on the bench as in the twin.
   - **In the twin:** the platform presents the declared level, as a factory step would leave it. The ledger (§1.7) names `BOR_LEV` as set by the declaration, witnessed only on silicon.
   - **The fabrication package:** whenever `bor_level` is non-zero, `FAB-NOTES` carries a factory step that programs `BOR_LEV` and reads it back. It gives the exact option-byte value, and a command for a named free tool if one does it without new code. The firmware never programs option bytes itself.
7. **The witness ledger (B2).**
   - The `verify` verdict carries one row per register field the generated firmware writes or leaves at reset. Each row is classed as one of:
     - `independent-rule` (proved against an `aethelgard-ir` rule);
     - `observed` (the twin's model reacted to it);
     - `same-source` (only the generated oracle checks it);
     - `ignored-by-model` (Renode stores or ignores it);
     - `silicon-only`.
   - **Rows that are neither `independent-rule` nor `observed` must be acknowledged by name in the specification,** as `accept_lifecycle` acknowledges an EOL part. `--fab-out` refuses an unacknowledged row (`UnwitnessedFieldNotAcknowledged`).
   - The ledger is the bench sheet's source: generate the register rows of the next bench sheet from it.
   - **Amended 2026-11-15 (`4220` D-B5, D-B7; comment 6898233775):** the ledger also carries non-register facts the twin cannot judge:
     - each sensor script's wait sufficiency is `silicon-only` (the twin runs the spin 1.92 times long and models no instruction timing, `4220` F2);
     - each register a model answers with 0x00 and a warning is `ignored-by-model`;
     - each declared pin capacitance is its own row (`4220b`).
8. **HSI tolerance (review A4, smaller).** DS12117's HSI accuracy over the declared temperature range is carried into `4300a`'s DShot and PWM timing check and the USART `BRR` tolerance check (`4300b`). The worst case is judged, not the nominal.
9. **The waits, statically (`4220` D-B5).** Each `parts.json` script's `settle_us` must cover the row's cited start-up maximum, or twice a typical-only figure as a named condition (`SettleBelowStartUp`). The generated spin is sized from `settle_us`, so this binds the firmware as well as the table.

## 2. Acceptance

- **The reference board and every fixture:**
  - each SPI controller has a kernel clock, an achieved rate at or below its declared speed, and `SSM|SSI` set;
  - each I2C bus's `TIMINGR` passes the independent rule;
  - the boot record carries `BOR_LEV`.
- **Each independent rule is killed by a mutant of the generator,** and each generator mutant is killed by the rule. Report the mutants, as `check_silicon`'s were.
- **The verdict's ledger lists every field,** and the reference specification acknowledges its unwitnessed rows by name.
- **Existing boards' board files are byte-identical.** Their firmware and verdicts change, and every change is named in FEEDBACK. The README's quickstart pin moves in the architect's next docs PR.
- **Each new refusal has a tier-1 test.** The CLAUDE.md gates pass. CI is green with every shard under 20 minutes, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-06`, `INV-07`; `RUL-16`, `RUL-19`.
- **Fence:**
  - `aethelgard-ir`: the independent rules, the kernel clock and achieved rates in the solved topology, the acknowledgement field;
  - `aethelgard-svd-hal`: the generator and its oracle;
  - `aethelgard-twin`: the option-byte presentation;
  - the CLI: `verify`'s ledger and refusal, `check speeds`, `fab`'s factory step;
  - tests and fixtures; `ci/`; FEEDBACK.
- **Next free versions:** take them from `main` at kickoff. The second to merge renumbers.
- One PR, left unmerged. Write `~/.cache/aet-4230/pr-title-body.md`.

## 5. Adversarial pass, full

1. **Mutate the generator, not the rule:** a wrong `MBR`, `SSM` cleared, `SPI123SEL` left at reset, a `TIMINGR` field off by one. Each must fail `prove` or the rule's check.
2. **Mutate the rule, not the generator.** The generator's own tests must catch it, or the mutant survives and is reported.
3. **Lie about the brown-out level:** a declared 3 with the twin presenting 0, and a package with no factory step.
4. **Hide a field from the ledger:** a field the firmware writes that has no row must fail a test.

## 7. Flag-and-stop conditions

- **A declared speed in a fixture cannot be reached** from HSI through `per_ck` without a PLL. Stop with the list; a PLL is a separate ruling.
- **§1.0 shows Renode refuses a write the platform needs** for §1.6's presentation.
- **The ledger has more than about 60 rows on the reference board.** Stop with the list. The acknowledgement form may need grouping by family.
