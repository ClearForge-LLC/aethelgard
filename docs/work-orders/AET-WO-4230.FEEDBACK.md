# AET-WO-4230 — FEEDBACK

**Branch:** `wo/aet-4230-silicon`, cut from `main` at `25640c5` (specification 9, topology 7, trace 11, `TIER2_MIN` 155).
**Builder:** Claude Opus 5.5 (strong tier), as project manager with subagents.
**Amendments in force** (kickoff; PR #152 comment 6903399822 Q3 and Q9): §1.8 takes the remote transmitter's deviation as a declared term; a new §1.10 forbids a read-modify-write of any field Renode does not store, so those fields are written whole from the solved values and each is an `ignored-by-model` ledger row.

Every answer below is labelled **measured**, **read at source** or **inferred**.

## 1. Predictions (committed before any measured run, and before reading Renode's model sources)

### P1. `FLASH_OPTSR_CUR` in Renode 1.17.0 (§1.0)
- The platform includes `MTD.STM32H7_FlashController` at `0x52002000`, so the register exists and is not a tag (inferred from `stm32h7.repl` line 66).
- It reads a fixed reset image with `BOR_LEV` = `00` (inferred: Renode models defaults, and RM0433 Table 21's factory value is `00`).
- A CPU or monitor write to `OPTSR_CUR` does not persist: it is read-only on silicon (RM0433 §4.9), and I expect the model to follow. The way to present a level is `OPTSR_PRG` under the option-key unlock plus `OPTSTART`, which I expect the model to copy into `OPTSR_CUR`. If it does not, the presentation needs a monitor-side write, and §7's second stop may fire.

### P2. `RCC_D2CCIP1R` and `RCC_D1CCIPR` (§1.0)
- Both read `0` at reset (RM0433's reset value is `0x0000_0000` for both).
- A write persists (inferred: `STM32H7_RCC` keeps most selection registers as plain stored fields), but the model does not change any SPI behaviour on it: Renode has no kernel clock.

### P3. `SPI` `CFG1.MBR` and `CFG2.SSM`/`SSI` (§1.0)
- Not stored: 4300b FEEDBACK §10 measured `MBR`, `SSM`, `SSI` among the `SPI1` fields that read back 0 after an all-ones write.
- Not honoured: a transfer completes at once whatever `MBR` is, and `SSM` = 0 with no NSS pin raises no mode fault (inferred: today's firmware leaves `SSM` = 0 and the flash read is witnessed).
- So `MBR`, `SSM`, `SSI` and the kernel-clock selection are `ignored-by-model` rows, judged by the independent rules.

### P4. The reference's speeds (§7 first stop)
- With `per_ck` = HSI = 64 MHz, SPI's fastest rate is 32 MHz (`MBR` = 0). The reference declares `speed_hz` 20 MHz on `FLASH`, so `MBR` = 1, 16 MHz achieved (inferred from the topology's `rate_hz` 20,000,000).
- Every fixture's declared SPI speed is ≥ 250 kHz (64 MHz / 256), so none is refused; none needs a PLL. If a fixture declares a speed above 32 MHz, §7's first stop fires.
- I2C at a 64 MHz kernel: Standard, Fast and Fast-mode Plus are all reachable from HSI.

### P5. The ledger's size (§7 third stop)
- The reference's generated firmware writes RCC enables, GPIO `MODER`/`OTYPER`/`OSPEEDR`/`PUPDR`/`AFR`/`BSRR`, I2C `CR1`/`CR2`/`TIMINGR`/`TXDR`, SPI `CR1`/`CR2`/`CFG1`/`CFG2`/`TXDR`, USART `CR1`/`CR2`/`CR3`/`BRR`/`TDR`, plus the new `D2CCIP1R` selection and the `OPTSR_CUR` read. Grouped by register family and field, I expect 40 to 55 rows on the reference: under §7's 60, so no stop.

### P6. Mutants (§5)
- Generator mutants (a wrong `MBR`, `SSM` cleared, `SPI123SEL` left at reset, a `TIMINGR` field off by one) fail `prove`, because the oracle will compare against the solved topology's values that the independent rule judged, not against the generator's own computation.
- Rule mutants are caught by the rule's own unit tests against hand-computed values; I expect none to survive, but any that does is reported.

### P7. Existing boards
- Board files (schematic, PCB, rules) are byte-identical: nothing here touches `aethelgard-eda`.
- Firmware changes on every board with SPI or I2C. Verdicts change by the ledger and the boot record's `BOR_LEV`; the reference declares `bor_level` 3, so its twin must present 3 to pass.

## 2. §1.0, measured (2026-11-16 03:41 UTC, pinned image `aethelgard-tools@sha256:c69ff9bc…a195a`, Renode `1.17.0+20260907gitf1dd1b4af`)

The reference's emitted platform (`main` `25640c5`, `sentinel-01.repl`), driven from Renode's monitor with `LogPeripheralAccess` on `flashController`, `rcc` and `spi1`, run under `~/bin/capped-run` (scope `memory.max` 8 GiB, `memory.swap.max` 0, `pids.max` 8192, read back). The script and its whole output are in `~/.cache/aet-4230/m0/` (`m0.sh`, `out.txt`, `m0.log`). Renode's model sources were read at `renode-infrastructure` `066a7f13c052215632d469c995c89aea37c573b1`, the submodule of `renode` tag `v1.17.0` (`~/.cache/aet-4230/renode-src/SOURCE.txt` lists sha256 per file).

| Register | Reset read | Write, read back | Label |
|---|---|---|---|
| `FLASH_OPTSR_CUR` (0x5200201C) | `0x0406AAF0`: `BOR_LEV` (bits 3:2) = `00` | `0xFFFFFFFF` ignored, still `0x0406AAF0` (the field is `FieldMode.Read`) | measured; read at source (`STM32H7_FlashController.cs` L118–119) |
| `FLASH_OPTSR_PRG` (0x52002020), locked | `0x0406AAF0` | `0x0406AAFC` ignored, with the warning "Attempted to write to a program register (OptionStatusProgram) while OPTLOCK bit is set. Ignoring" | measured |
| `FLASH_OPTKEYR` `0x08192A3B`, `0x4C5D6E7F`, then `OPTSR_PRG` `0x0406AAFC` | `OPTCR` reads `0x0` (unlocked) | `OPTSR_PRG` reads `0x0406AAFC`; `OPTSR_CUR` unchanged until `OPTCR.OPTSTART` | measured |
| `OPTCR` = `0x2` (`OPTSTART`) | | `OPTSR_CUR` reads `0x0406AAFC`: `BOR_LEV` = `11` | measured |
| after `machine Reset` | | `OPTSR_CUR` and `OPTSR_PRG` both `0x0406AAFC`: kept (`OPTSR_PRG` is not soft-resettable, and `Reset` copies it into `OPTSR_CUR`) | measured; read at source (L38–47, L121–122, L147–150) |
| `RCC_D1CCIPR` (0x5802444C) | `0x0` | `0xFFFFFFFF`: reads `0x0`; "Unhandled write … Tags: … CKPERSEL (0x3)" | measured; every field a tag (L394–402) |
| `RCC_D2CCIP1R` (0x58024450) | `0x0` | `0x00004000` (`SPI123SEL` = `100`): reads `0x0`; "Unhandled bits: [14] … Tags: SPI123SEL (0x4)"; all-ones likewise | measured; every field a tag (L403–418) |
| `RCC_D2CCIP2R` (0x58024454) | `0x0` | not written | measured; `I2C123SEL`, `USART16SEL`, `USART234578SEL` are tags (L419–431) |
| `SPI1_CFG1` (0x40013008) | `0x0` | `0x70000007` (`MBR` 7, `DSIZE` 7): reads `0x7`; "Tags: MBR (0x7)". All-ones reads `0xC01F` (`DSIZE`, `RXDMAEN`, `TXDMAEN` only) | measured |
| `SPI1_CFG2` (0x4001300C) | `0x00400000` (`MASTER` always reads 1) | `0x04400000` (`SSM`): reads `0x00400000`; "Tags: SSM (0x1)". All-ones reads `0x00C00000` (`MASTER`, `LSBFRST` only) | measured |
| `SPI1_CR1` (0x40013000) | `0x0` | `0x00001000` (`SSI`): reads `0x0`; "Tags: SSI (0x1)" | measured |

**What it decides:**
- **The brown-out presentation (§1.6) is possible:** a key unlock, an `OPTSR_PRG` write and `OPTSTART` give `OPTSR_CUR` any `BOR_LEV`, and the value survives `machine Reset`. §7's second stop does not fire. Whether it survives the droop scenario's own reset is measured with the campaign (§4).
- **P1 held.** P2 was wrong: `CKPERSEL`, `SPI123SEL`, `SPI45SEL` and `I2C123SEL` are tags, so a write is logged and dropped, and every read gives 0. So the run-time `ResetClock` check reads `D1CCIPR` and `D2CCIP1R` before the selection is written and never after; a read-back would park every boot in the twin, and would tell the firmware it is in the twin (4300b Q9's class).
- **P3 held.** `MBR`, `SSM` and `SSI` are not stored (measured). They are not honoured either: the model's transfer path never reads them, and `MODF` is a tag, so no mode fault can be raised (read at source, `STM32H7_SPI.cs` L91–203; no transfer was run to show it).
- **Ledger consequences:** `SPI123SEL`, `CKPERSEL`, `I2C123SEL`, `MBR`, `SSM` and `SSI` are `ignored-by-model`. Each is judged by an independent rule, and the access log shows the value written. `BOR_LEV` is `silicon-only` as programmed: the twin presents the declared value.
- **4300b's serial judge allows RCC writes only to the nine `*ENR` registers** (4300b FEEDBACK §8, passes 3 and 4), so the generated `D2CCIP1R` write fails the reference's CONSOLE judgement unless that allowance is extended. That judge is outside this work order's named CLI fence, but §1.1 forces the change. It allows exactly the solved whole word, and nothing else (§5).

**m1, the presentation in the platform (measured 03:57 UTC, `~/.cache/aet-4230/m1/`).** An `init add:` block on `flashController`, appended to the emitted `.repl` (`OPTKEYR` 0x08192A3B, 0x4C5D6E7F; `OPTSR_PRG` 0x0406AAFC; `OPTCR` 0x2; then `OPTKEYR` 0x0), makes `OPTSR_CUR` read `0x0406AAFC` at the first read and after `machine Reset`, with `OPTCR` reading `0x1` (locked, as silicon resets) both times. Ending with `OPTCR` 0x1 instead leaves it reading `0x0` (unlocked) until the first reset, a difference a firmware could see, so the wrong-key write is the last line.

## 3. How it was built

The PM worked single-threaded on the merges and the gates, and handed disjoint crates to subagents in their own worktrees. Each subagent worked from the shared contract `~/.cache/aet-4230/contract.md` (decisions D1–D10 and Amendment 1); each report was spot-checked against its commits before merging. The slices:

| Slice | Crate | What | Merged |
|---|---|---|---|
| A1 | `aethelgard-ir` | topology 8 (`spi_controllers`, `i2c_timings`, `bor_level`); the solver's SPI kernel, `MBR` and I2C `TIMINGR`/`CR1`; the independent rules in `silicon.rs` | `04ee316`, `045d873`, `d2070cc` |
| A2a | `aethelgard-ir` | §1.5 floors at the domain maximum, Fm+ at DS12117's 8 mA (4 mA below 2.7 V); §1.8 the HSI worst case and the peer's declared deviation in the serial check; the motor check's worst-case figures; specification 11 (`peer_deviation_ppm`) | `f1a8f09` |
| A2b | `aethelgard-ir` | §1.9 parts schema 6, a cited start-up per script row, `SettleBelowStartUp` | `9fe73c9` |
| B0 | `aethelgard-svd-hal` | §1.10 `OTYPER` whole; `ResetClock` extended and read first; the inits take it; A16 row 1 | `2283f0c`, `49b2312` |
| B1 | `aethelgard-svd-hal` | the values from the topology, whole words, the kernel select, the oracle from `aethelgard-ir`, the brown-out read, the ES0392 delay, the MODF guard, `register_fields` | `6dd18a5`, `13118ea`, `a77f6ea` |
| C1 | twin, CLI, chaos, evolve | the presentation and judgement of `BOR_LEV`, the serial judge's select allowance, `STATUS_EOT`, `check speeds` on the achieved rate, the factory step; trace 12 | `4cf0e56` … `16f3c49` |
| C2 | CLI | the witness ledger in `verify` (§1.7), its count and its completeness check | (below) |

`main` was merged once (`0570ffe`, 4300d3's specification 10), and this branch renumbered to specification 11.

## 5. Decisions taken inside the work order (each with its source)

- **Values live in the topology; rules are separate code (D1).** The solver writes each new register value into the solved topology. The generator copies it, and `aethelgard-ir::silicon` judges it with its own RM0433 constants, sharing no function with the solver or the generator. The Kani oracle takes the SPI words, the select word and I2C `CR1` from `silicon`; `TIMINGR` is the topology's word, which `silicon::check_i2c` judged.
- **The HSI worst case binds every HSI-derived value (Amendment 1).** DS12117 Rev 10 Table 36/133, the −40…TJmax row (junction temperature is bounded by no declaration): −25,894 / +14,984 ppm. A tight `TIMINGR` synthesized at the nominal 64 MHz broke UM10204's minima at +1.5 %, so minima are judged at the fastest kernel and maxima at the slowest. SPI's `MBR` is chosen against the fastest kernel: 1 MHz gives `MBR` 6 (500 kHz nominal), not 5.
- **SPI kernels:** SPI1–3 `per_ck` (`SPI123SEL` = `100`, `per_ck` = `hsi_ker_ck` at `CKPERSEL` reset); SPI4/5 and SPI6 `hsi_ker_ck` (`011`), since `per_ck` is not among their choices. The select is written whole once, after `ResetClock`, before the first SPI gate.
- **`CFG1` keeps `CRCSIZE` at reset (`0x0007_0007`, RM0433 p.2218–2219).** Contract D4's "every other field 0" was wrong at source.
- **The SPI below-range refusal is at `fast >> 8` = 253,746 Hz**, so that no declared speed falls between a refusal and an `MBR`.
- **The shortest `tSYNC` is two kernel cycles each, with no `tAF` credit** (RM0433 Tables 391–393 notes 2–4, pp.1983–1984). This is more conservative than §1.4's "including the filter delays"; `tAF` enters only the `SDADEL` bounds.
- **Fm+'s data-valid maximum is reported, not gated.** At DS12117's `tAF` max of 260 ns it cannot be met at any `PRESC`, and RM0433 p.1958 says it "can be violated when NOSTRETCH = 0", which ours is. No fixture declares Fm+.
- **Fm+ takes 8 mA (4 mA below a 2.7 V maximum rail).** ST's open pin data carries no pad type (contract D8; DS12117 Table 50 p.139, §6.3.32 p.188), so there is no `FT_f` check and no `SYSCFG_PMCR` write.
- **The serial check takes the worse of the HSI's two extremes,** not the one that adds by sign, which understates. It reports quantization, HSI, receiver, peer, worst case and margin.
- **The SPI bring-up order is `CR1` (`SSI`), then `CFG1`, then `CFG2`.** `CFG` writes are allowed only with `SPE` 0 (§50.11.1 p.2216), and `SSI` must already be high when `MASTER|SSM` land (§50.5.2 p.2184).
- **ES0392 §2.22.6:** the erratum gives no figure. The delay is `ceil(2^(MBR+1) × fast core / slow kernel)` reads of `SR` (5 at `MBR` 1), in rounds of 16 (a plain 267-iteration loop exhausted Kani's 8 GiB). It is sized, not witnessed.
- **RCC enable delay (A16):** read back each `*ENR` (RM0433 §8.5.10 step b), and do a dummy read of the block on GPIO, I2C and SPI (step d). TIM and USART blocks are never read, by their proofs: a residual.
- **The waits run at the fastest HSI:** `settle` spins are `ceil(µs × 64,958,976 / 10⁶)`.
- **The serial judge's RCC rule now requires each solved select word,** written whole once, before its controller's SPI gate. A superset value, a second write or a late write is refused.
- **The factory word:** RM0433 Table 21 (p.177) gives `0x0006_AAF0`. Renode's image is `0x0406_AAF0`, so contract D10's figure was Renode's. `FAB-NOTES` programs `BOR_LEV` into the word read from `OPTSR_CUR`, keeping `RDP` and the reserved bits as read.

## 4. Runs and results beside the predictions

| # | Prediction | Result | Label |
|---|---|---|---|
| P1 | `OPTSR_CUR` reads `BOR_LEV` 00, writes do not persist, `OPTSR_PRG` + `OPTSTART` presents a level | Held (§2). The platform presents it by `init add:` (m1), and it survives the campaign's reset (m2, below) | measured |
| P2 | `D1CCIPR`/`D2CCIP1R` writes persist | **Wrong:** every select field is a tag (§2) | measured |
| P3 | `MBR`, `SSM`, `SSI` neither stored nor honoured | Held. First silicon (the architect's 3302 bench, 05:05 UTC) adds what the twin cannot show: with `SSM` = 0 the first `SPE` raised `MODF` (`SR` `0x0000_1202`) and cleared `MASTER` | measured; bench |
| P4 | No fixture needs a PLL | Held: every SPI speed is 1–20 MHz and every I2C bus 400 kHz; all 17 solvable fixtures give a kernel (26 SPI and 26 I2C controllers hold their rules) | measured (tier 1) |
| P5 | 40 to 55 ledger rows | **Wrong by a factor of five:** 264 rows at field granularity. §7's third stop fires (§7 below) | measured |
| P6 | Generator mutants fail `prove`; rule mutants fail the rule's tests | Mostly held (§6). G7 (an I2C `CR1` read-modify-write) first survived Kani, and is killed by an image rule added for it. Four rule mutants leave the generated text byte-identical, so only `aethelgard-ir`'s tests kill them | measured |
| P7 | Board files byte-identical; firmware and verdicts change | Board files: §10. Firmware changes on every board; verdicts gain the ledger, `silicon` and the brown-out report | measured |

**m2, the campaign's reset** (C1, measured in the image, `~/.cache/aet-4230/c1/m2/`): the reset is `machine Reset`, the cold RCC writes, the vector table offset, then halt. After it, `OPTSR_CUR` and `OPTSR_PRG` read `0x0406AAFC` and `OPTCR` reads `0x1`; the `init` block does not re-run. Flash-controller access logging is off by default, so the judge switches it on for every boot.

**Tier 2 on the integrated head** (the pinned image, `~/.cache/aet-4230/t2/`):

| Suite | Result |
|---|---|
| `twin_tier2` | 62 of 66 passed on `82c2bc2`. One failure was a stale anchor (fixed in `b8fd4ed`). Three fail on `imu-baro-1v8` for the reason in Q-E (§8). The new completeness test (C2) passes in 2.6 s |
| `board_tier2` | 17 of 18; the vacuous-harness seed named a read-modify-write that §1.10 removed (fixed in `86eb013`) |
| `campaign_tier2` | 9 of 9 |
| `models_tier2` | 7 of 7 |
| `verify_2k2_tier2` | Every gate of the reference passed. The test expected six harnesses where there are now seven (the flash read): fixed in `86eb013` |
| `verify_47k_tier2` | 2 of 2 |
| `against_fab_tier2` | 2 of 2 |

## 6. Mutants, both ways (WO §2, §5.1, §5.2)

Each slice applied every mutant by editing a file, ran the test that should kill it, and restored the file from its own saved copy. The full tables are under `~/.cache/aet-4230/<slice>/mutants.md`.

- **Rules killed by generator (solver) mutants** (A1, `cli --test silicon_rules`):
  - `MBR` ±1;
  - the wrong kernel select;
  - `SCLL`, `SCLH`, `SCLDEL`, `SDADEL` each ±1;
  - `PRESC` +1, both re-synthesized tight and in the word only.
  13 of 13 were killed by the rule's `holds`, never by a skipped fixture.
- **The generator killed by its oracle** (B1, Kani in the image):
  - a wrong `MBR`, `SSM` cleared, `SSI` cleared, `SPI123SEL` left at reset (no select write);
  - a `TIMINGR` field off by one in the emitted code;
  - `CR1` read-modify-written (G7, which needed a new image rule);
  - the ES0392 delay one round short (G8; tier 1 does not catch it);
  - `SPE` before `CFG2` (G10).
  All were killed. B0's 16 (`OTYPER`, the reset clock, the EOT wait, the `*ENR` read-back) were all killed.
- **Rule mutants against the rule's own tests** (A1 R1–R5, B1 R6–R12): all killed. **R1, R4, R5 and R11 survive the generator's suites**, because they leave the generated text byte-identical. Only `aethelgard-ir`'s unit tests kill them. That is reported as the WO asks: a rule mutant the generator cannot see.
- **A2a** 12 (floors, the Fm+ row, the 2.7 V boundary, the HSI extremes, the peer term, strictness, `PeerDeviationBelowCited`, the motor slow bound), **A2b** 8 (`SettleBelowStartUp`), **C1** 21 (the brown-out judge, the flash access rule, the RCC select allowance; R2 needed a superset case), **C2** 5 plus the tier-2 one (an unknown field accepted, a row dropped, `observed` without a citation, a failing silicon finding passing, `MBR` hidden from the ledger): all killed.
- **§5.3, lying about the brown-out level:** a declared 3 with the twin presenting 0 fails `brown_out_level_not_programmed`. A firmware that programs the option bytes itself fails `flash_controller_access`. A package with no factory step for a non-zero level is a tier-1 test.
- **§5.4, a field hidden from the ledger:**
  - an unknown field is refused before any gate;
  - the tier-2 completeness test rebuilds every CPU write of the reference's boot from Renode's access log and names any bit no row covers;
  - its first run found a real hole: the generated `Reset` read-modify-writes `SCB.CPACR`, which `register_fields` did not list (now two `CORE` rows).

## 7. The stop: the ledger has 264 rows on the reference (WO §7, third condition)

`aethelgard verify`'s verdict now carries the ledger: one row per field, with a family key on each, so any grouping is a view. On the reference (`examples/sentinel-01-minimal.json`) it has **264 rows**: 248 written, 16 at reset or read (counted from the list's `access` column).

| Class | Rows | | Family | Rows |
|---|---|---|---|---|
| `independent-rule` | 67 | | GPIO | 95 |
| `observed` | 58 | | USART | 47 |
| `same-source` | 59 | | SPI | 46 |
| `ignored-by-model` | 79 | | I2C | 46 |
| `silicon-only` | 1 (`BOR_LEV`) | | RCC | 27 |
| | | | CORE | 2 |
| | | | FLASH | 1 |

The **139 rows that need an acknowledgement** collapse as follows:
- to **24 entries** if acknowledged per register (`GPIOA.OTYPER`, `SPI1.CFG1`, …);
- to **26** per register and class;
- to **11** per peripheral instance and class;
- to **9** per family and class (GPIO ×2, USART ×2, and one each for I2C, RCC, SPI, CORE, FLASH).

The full list (id, class, family, access, value) is `~/.cache/aet-4230/c2/ledger-reference.tsv`. The bench rows generated from it are in `~/.cache/aet-4230/c2/bench-rows-reference.md`: 120 rows, one per field, where B8 groups by register.

**What is built, and what is not:**
- Built: the ledger in the verdict; the `silicon` findings, which fail the verdict by name; the completeness check (tier 2); the bench rows.
- **Not built:** the acknowledgement field in the specification and `--fab-out`'s `UnwitnessedFieldNotAcknowledged`. Their form is what this stop asks the architect to rule, together with the questions in §8.
- Not yet run: the full adversarial pass and the reduction pass. They run after the ruling, on the finished acknowledgement form.

**Proposal (the PM's, for the ruling):** acknowledge **per register and class** (26 entries on the reference). The acknowledgement names the register, the class, and the fields it covers, so that a new field in a register already acknowledged fails until the list names it. The verdict keeps rows at field granularity. Grouping by family alone (9) would let a new register of a family pass unseen.

## 8. Questions for the architect (in this stop)

- **Q-A, the peer's deviation (§1.8 amendment).** The HSI's worst case alone is 25,894 ppm. A 10,000 ppm peer term refuses a 115200 console and SBUS. `peer_deviation_ppm` is therefore declared 5,000 for the console, MAVLink, GPS and SBUS ports, a declaration with no datasheet behind it, which the specification says. CRSF carries the cited 8,002 ppm (`CRSF_BAUDS`). Rule the figure, or require a cited peer part.
- **Q-B, CRSF at TBS's default 416666 baud cannot solve on HSI.** `BRR` 154 is 2,596 ppm slow; with the HSI and the cited spread the total is 36,425 ppm against RM0433 Table 407's 33,300. `solve` refuses it (`SerialBaudTolerance`), and 420000 solves with a 1,845 ppm margin. Sentinel-02 needs CRSF: a crystal (`hse_hz`, unowned in §7.1) would close it.
- **Q-C, DShot and PWM receivers' tolerance (§1.8).** No source the repository holds gives one. The motor check now reports the worst-case period and high time at both HSI extremes without changing `holds`. A PWM idle pulse is 1,026.6 µs at the slow HSI, and DShot600's bit is 1.647 to 1.716 µs. Rule a source or a declared term.
- **Q-D, `CFG2.AFCNTR` (B1, RM0433 §50.11.4).** With `AFCNTR` = 0 the controller does not drive its pins while `SPE` is 0. The transfer drops the chip select before `SPE` rises and raises it after `SPE` falls, so SCK and MOSI float while the device is selected. Options: `AFCNTR` = 1 for a master, or select after `SPE`. Not changed.
- **Q-E, §1.9 against the twin's 100 ms window.** `imu-baro-1v8` carries two ICM20948 stand-ins. Each now waits 40 ms, twice the 20 ms typical-only start-up (DS Table 2, p.12; it was `null` before). The boot word reads 0 at 100 ms (measured, `twin_tier2` on `82c2bc2`). That the cause is the waits, Renode running each spin about 1.92 times long (4220 F2) for about 154 ms in all, is inferred by arithmetic, not measured. Three `twin_tier2` tests on that fixture fail. They fail closed. Options:
  - (a) the twin-window work order (4300b Q4's owner) widens the window;
  - (b) the generator overlaps independent devices' waits;
  - (c) stand-ins are exempt from §1.9, which contradicts its text.
  Not weakened. The head's CI fails on those three tests for this reason; §12 names its other red, the two shards cancelled at the job limit.
- **Q-F, `RCC` step c (B0).** RM0433 §8.5.10 also asks to poll `D2CKRDY` after an enable. That flag resets to 0 and is a tag in Renode (`STM32H7_RCC.cs` L175), so a wait on it would park every twin boot. Not done.
- **Q-G, motor pins float until `start_outputs` (A16, B0).** True: they are in analog mode from reset, then Hi-Z with `MOE`/`CCxE` clear (RM0433 Table 337 p.1608, Table 342 p.1695). No source the repository cites gives a level before start. A pull-down or a low GPIO is a policy ruling.
- **Q-H, `BRR`'s rule.** `serial::check_silicon`'s exactness test re-runs the solver's own `solve_brr` (4300b). The tolerance judgement is independent, so the row is `independent-rule` with that note. An independent exactness check (an inequality) would close it.
- **Q-I, `PIN_C` rows.** 4220b's declared pin capacitance is not on `main`, so no board has `PIN_C` rows. Should connectors' off-board capacitances be rows?
- **Q-J, a denied fetch.** C1's retry of ST's STM32CubeProgrammer manual (UM2237) was denied by the permission classifier after a network error, and it did not work around the denial (kickoff rule). `FAB-NOTES` gives the register procedure and value with RM0433 citations, and no programmer command. The operator decides whether to fetch UM2237.

## 9. Residuals, named

- **The twin presents `OPTSR` as Renode's image, `0x0406_AAF0`.** RM0433 Table 21 (p.177) gives the factory word as `0x0006_AAF0`, so a firmware reading the whole word can tell the twin from silicon. `BOR_LEV` alone is judged.
- **Fm+'s data-valid maximum is unmet** at DS12117's `tAF` max (§5). No fixture declares Fm+.
- **The ES0392 §2.22.6 delay is sized, not witnessed.**
- **`reset_clock` can be called a second time** after the select. On silicon it then reads `SPI123SEL` = `100` (an error); the twin reads 0 (ok). Only the bring-up's order prevents it.
- **TIM and USART blocks get no dummy read after their enable** (§8.5.10 step d), because their proofs forbid reads of those blocks.
- **`SolvedTopology::validate` checks pull-ups at the nominal** `AnalogLine.domain_millivolts`. It is weaker than `solve`, and only hand-edited topologies reach it.
- **Replay's copy of the serial check** (`aethelgard-chaos` `serial_silicon`) judges quantization only, so a topology failing only on the worst case would replay `matched = false`.
- **The ICP10111 data command (`0x7866`) has no wait before its read;** the part measures for up to 23.8 ms (Table 7, p.9). It is not §1.9's configure start-up.
- **The two `CORE` rows are hand-copied** from `board/firmware.rs`'s `Reset` into the CLI's ledger. They belong in `register_fields`.
- **`choose_speed` picks `OSPEEDR` on the declared rate,** not the achieved one: conservative.
- **The I2C `CR2` rows are `observed` on 4220's probe platform,** not on the reference's. The ENR gates are `observed` from the 3302 sheet's Renode column.

## 10. Existing boards' board files (WO §2)

`~/.cache/aet-4230/boardcmp.sh` generates every specification in the repository twice and diffs the `kicad/` trees:
- once with `main`'s CLI (`25640c5`, saved before any change) from the file as it is on `25640c5`;
- once with this branch's CLI from the file as it is here.

Results (`~/.cache/aet-4230/boardcmp/`):
- **The reference and 12 fixtures are byte-identical**, in all three files each: `.kicad_sch`, `.kicad_pcb` (the board as `generate` emits it; routed boards are exercised by `board_tier2`) and `.bom.csv`: `connectors`, `d1-24-i2c`, `d3-24-i2c-12-spi`, `four-layer-power`, `four-layer`, `motors-dshot600`, `motors-pwm`, `pair-standin`, `serial-dshot-9600`, `serial-four`, `spi-only`, `two-bme`.
- `imu-baro-1v8` and `sentinel-02-own` are refused by both binaries alike (`UnknownPart`: no vendored KiCad model, `4220b`'s).
- The `solve/` fixtures and `invalid-zero-domain` are refused by both, as designed.
- `aethelgard-eda` is untouched: `git diff 8030fa5 HEAD -- crates/aethelgard-eda` is empty (8030fa5 is the merge base with `main`).

Firmware and verdicts change on every board, each change named in §3 and §5:
- the reset clock is read first;
- the select write and whole words;
- the brown-out word (record offset 8);
- the EOT wait and the ES0392 delay;
- the `*ENR` read-backs;
- spins sized at the fastest HSI;
- the ledger and `silicon` in the verdict;
- trace 12 (kind 10's `brown_out`, `flash_access`).

## 11. Versions and counts on this head

- Specification 11 (`peer_deviation_ppm`; 4300d3 took 10 on `main`).
- Topology 8; parts schema 6; power parts 2 (`main`'s); trace 12.
- `TIER2_MIN` 156: C2's completeness test is the one new tier-2 test, counted on the host. 4300g, which also takes specification 11, renumbers if it merges second.

## 12. CI on the PR head (run 43826691595, head `3a779b0`)

| Job | Result | Minutes |
|---|---|---|
| `gates` | success | 15.6 |
| tools-shard 0 | success | 28.1 |
| tools-shard 1 | **cancelled at the 30-min job limit**, inside `against_fab_tier2` (its first test passed) | 30.2 |
| tools-shard 2 | failure: `the_imu_and_barometer_fixture_runs_every_script_step_witnessed` (Q-E) | 22.6 |
| tools-shard 3 | success | 25.4 |
| tools-shard 4 | failure: `a_seeded_imu_fault_is_named_by_its_step`, `a_seeded_barometer_fault_is_named_by_its_step` (Q-E) | 27.1 |
| tools-shard 5 | success | 23.7 |
| tools-shard 6 | **cancelled at the 30-min job limit**, inside `verify_2k2_tier2` | 30.2 |
| tools-shard 7 | success | 18.7 |

- **The failures are exactly Q-E's three tests.** No other test failed on any shard.
- **The two cancellations are a planning consequence of this branch.** Every `prove`-heavy tier-2 test grew: a seventh harness (the flash read), a 14-step schedule proof where it was 13, the MODF and image rules, and the ES0392 rounds. The in-shard weights (4300b's) now under-plan the shards holding `verify_2k2_tier2` and `against_fab_tier2`.
- **A measured refit is owed, and it is not done here.** The shards' times sum to about 206 minutes, so a balanced plan lands near 26 minutes per shard, under the job limit. The refit (`ci/shard.sh`'s rule 5, worst in-shard times) runs on the head after the ruling, since the acknowledgement form changes the tests again. Until then, the cancelled shards' remaining tests were run in the pinned image on Furnace (§4): every tier-2 suite passes there except Q-E's three.
- **Q-K:** refit only, or also lift the 30-minute job limit, given that 20 minutes is now a target and not a gate (Scotty's ruling, 04:35)?

## 13. Where it stands

**STOPPED at WO §7's third condition:** the reference's ledger has 264 rows (§7).

The architect's ruling is needed on:
- the acknowledgement form (the proposal: per register and class, 26 entries on the reference);
- the questions Q-A to Q-K (§8, §12).

After the ruling:
- build the acknowledgement field and `UnwitnessedFieldNotAcknowledged`, and acknowledge the reference;
- apply the rulings;
- refit the shards;
- run the full adversarial pass (fresh subagent, read-only) and the reduction pass;
- then READY.
