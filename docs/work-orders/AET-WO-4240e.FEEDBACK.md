# AET-WO-4240e — FEEDBACK: the generated firmware never switches a kernel-clock mux away from a dead input

**Builder:** Claude (Opus 5.5), Furnace, 2026-11-23. Branch `wo/aet-4240e-kernel-clock-switch` from `main` `465bd860`; PR #372.

**The answer in one screen:**
- **Before:** 26 of the 32 committed specifications that generate started SPI1–3 with no kernel clock. That is every board on the HSI with SPI on `SPI123SEL`. The firmware wrote `SPI123SEL` = `per_ck` while the mux's input, `pll1_q_ck`, had never run (RM0433 Rev 9 pp. 371–372).
- **After:** 0 of 32 (`spike/4240e/out/measure-after.json`).
- **The fix (D1 (a), ruled):**
  - The bring-up writes PLL1's words from the HSI with PLL1 off.
  - It sets `PLL1ON` by a read-modify-write and polls `PLL1RDY`, bounded.
  - Then it writes the selects, makes 8 straight-line reads through the transition, and clears `PLL1ON` the same way.
  - A lock that does not come writes `0x2100B0F5` and parks (D6).
- **One rule and three judges of the order, none of them the generator:**
  - the rule `aethelgard_ir::kernel_clock::check` (`RUL-20` (1)) judges the solved topology;
  - the order is judged by the svd-hal Kani harness `spi_kernel_switch_is_in_order_bounded_and_named`;
  - by the twin's serial RCC judge (Q9);
  - and, at run time, by Tviburi's RCC guard (`TVI-WO-1300a`).
- **The ADC (ruled (i)):** `clock.adc.ckmode` is a solved fact. `d1-drv8876`'s ADC3 on `adc_sclk` is judged clocked, which corrects Tviburi's refusal of it.
- **No pin, copper or `ORDERED.sha256` byte moved.** The bench package moves by its ELF and hash lines once the dispatch runs (asked).

TODO-FILL: commits table, mutant summary, CI, Q10.

## 0. How it was made

| commit | what |
|---|---|
| `e67db6cd` | predictions, before the measurement |
| `82b4d2d9` | the measurement and the STOP (§1.2) |
| `d596de2e` | the build to the 17:52Z rulings |
| `af671c0e` | `RCC_CR` by read-modify-write; Q8, Q9 |
| TODO | the adversarial pass's fixes; Q10; the reduction pass; main merged |

**Who wrote what:**
- I wrote the measurement, the generator, the IR's solved facts, the twin and CLI wiring, and the goldens.
- **The independent rule** (`crates/aethelgard-ir/src/kernel_clock.rs`, its 17 first tests and 54 mutants in `spike/4240e/mutants-rule.md`) was written by a separate Opus agent. It was given the manual, the IR's types and the rule's shape, and told not to open svd-hal, the generator's crates or the solver's word choice. Its imports confirm that: `clock::{Budget, SolvedKernelSwitch}`, `vos::Vos`, `SolvedTopology`, `SpiKernel`.
- **The adversarial pass** was a second, fresh Opus reviewer, given the code and the manual and not my conclusions (§3).

**The kit** (`~/.cache/aet-4240e/kit`):
- `img.sh` (capped-run, swap 0 read back, its own cargo home);
- `prove.sh`, `sim.sh`, `adcelf.sh`;
- `mutants.py`.

## 1. The measurement and the stop (§1.1–§1.2)

The record is in `spike/4240e/RESULTS.md`, and the predictions are `PREDICTIONS.md`, committed first. After the stop:
- **P4.1 was partly wrong.** (a) also moved the solved topologies (D7) and seven specifications' acknowledgements (D2).
- **The predictions file had one count wrong.** I wrote 27 boards before recounting to 26; RESULTS was corrected before the commit.
- **The rule agent found a page error:** RESULTS §6 cites `PLL1RGE` on p. 401, but it is on p. 402 (text after the p. 401 footer). The code and README cite p. 402; RESULTS is left as the record of the stop.

**Rulings (17:52Z):**
- D1 (a);
- D2 granted (deletions only);
- D4 planted topologies with the 26 pre-fix ones as the scar's fixtures;
- D5 the three kinds;
- D6 the `main.rs` fault arm;
- D7 solved 19;
- ADC (i).

**Q8 and Q9 (18:30Z):** both granted.

**Q10:** TODO.

## 2. What was built

**IR:**
- `clock.kernel_switch` `{pllckselr, pllcfgr, pll1divr}` and `clock.adc` `{ckmode}`, each absent when unused. Solved schema is 19.
- `solve::hsi_switch_words`: across the HSI's whole band, `0x50` / `0x0002000C` / `0x0101020F`. That is `DIVM1` 5, ×16, Q ÷2, giving `pll1_q_ck` 99.7–103.9 MHz.
- `solve::kernel_switch` is shared by `solve` and `derive_controllers`.
- `validate` sets both facts aside when it holds the tree to the reset one or to the HSE's. `kernel_clock::check` judges them.

**svd-hal:**
- **`select_spi_kernels`** writes the switch and returns `Result<SpiKernels, KernelClockFault>`.
- **The transition wait** is `SWITCH_READS` = 8, emitted straight-line. A loop there met every other harness's unwind bound through the token stub.
- **`main.rs`** gains the fault arm and `PLL_POLLS` = 9744 (150 µs at the HSI's fastest).
- **Refusals by name:**
  - `check_silicon` refuses by the rule's kind;
  - `AdcClockNotGenerated`: an ADC with `clock.adc` absent or `CKMODE` ≠ `11`;
  - `SwitchQBelowPerCk`: the settle wait's premise.
- **The Kani harness** holds every switch write to the topology's words, in order. No select comes before `PLL1RDY` reads 1. Each `RCC_CR` write is the word read just before it with `PLL1ON` set, then cleared. The other harnesses take `spi_kernels_token`.
- **Tests:** `kernel_switch.rs` (the order) and `kernel_scar.rs` (the 26).

**CLI:**
- `ledger::Silicon.kernel_clock`, carried in the verdict, and `silicon_rule_fails` messages that name the kind.
- Ledger rows: the switch's PLL words cite the rule; `PLL1ON` notes the switch.
- The selector reset rows: D2, and Q10.

**Twin (Q9):** `RccRule.switch`: the expected writes, from the topology and the header, in order, each once per boot, every select inside the PLL1 window.

**Specifications (D2, Q8):** the stale acknowledgements were deleted from 7 specifications, and `RCC.CR` `silicon-only` `PLL1RDY` was added to 6 (§4). Each was edited as text, with every other byte checked equal by JSON comparison.

**BENCH.md:**
- **B10:** the switch on the Nucleo: `D2CCIP1R`, `RCC_CR`, SCK on PA5, power-on reset first.
- **B11:** the ADC on `adc_sclk` with PLL2 off. Its ELF is `d1-drv8876`'s, sha256 `441a7ee785876c5981ce087d97f1e8db1acb2609d687921a49e7e991720cac4a`, built twice at two paths in the image, byte for byte. The sheet may name only the package's ELF (ruling D3 on #362), so the digest lives here.

## 3. The adversarial pass (§5, full)

TODO-FILL

## 4. Versions and what moved

TODO-FILL

## 5. Decisions and rows

TODO-FILL

## 6. Gates, CI, package

TODO-FILL
