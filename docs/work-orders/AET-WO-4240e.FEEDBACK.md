# AET-WO-4240e — FEEDBACK: the generated firmware never switches a kernel-clock mux away from a dead input

**Builder:** Claude (Opus 5.5), Furnace, 2026-11-23. Branch `wo/aet-4240e-kernel-clock-switch` from `main` `465bd860`, with `main` `8d727980` and `e5984c86` merged. PR #372, left unmerged. **READY.**

**The answer:**
- **Before:** 26 of the 32 committed specifications that generate started SPI1–3 with no kernel clock. That is every board on the HSI with SPI on `SPI123SEL`. The firmware wrote `SPI123SEL` = `per_ck` while the mux's input, `pll1_q_ck`, had never run (RM0433 Rev 9 pp. 371–372).
- **After:** 0 of 32 (`spike/4240e/out/measure-after.json`, at `2f3184b4`).
- **The fix (D1 (a)):** the bring-up writes PLL1's words from the HSI with PLL1 off, then writes `RCC_CR` whole (`HSION|PLL1ON`) and polls `PLL1RDY`, bounded. It then writes the selects, makes 8 straight-line reads through the transition, and writes `RCC_CR` whole again (`HSION`, Q11). A lock that does not come writes `0x2100B0F5` and parks (D6).
- **One rule, and three judges of the order that are not the generator:**
  - the rule `aethelgard_ir::kernel_clock::check` (`RUL-20` (1)) judges the solved topology. It is wired into generation (`check_silicon`, by its kinds) and into the verdict's silicon findings;
  - the order is judged by the svd-hal Kani harness `spi_kernel_switch_is_in_order_bounded_and_named`, which passes in the tools image;
  - by the twin's serial RCC judge (Q9), which passes on a real Renode boot of the bench ELF;
  - and, at run time, by Tviburi's RCC guard (`TVI-WO-1300a`).
- **The ADC (ruled (i)):** `clock.adc.ckmode` is a solved fact, so `d1-drv8876`'s ADC3 on `adc_sclk` is judged clocked. That corrects Tviburi's refusal of it.
- **No pin, copper or `ORDERED.sha256` byte moved.** `aethelgard-eda` reads no clock fact. The bench package moves by its ELF and hash lines once the dispatch runs (asked; §6).

## 0. How it was made

| commit | what |
|---|---|
| `e67db6cd` | predictions, before the measurement |
| `82b4d2d9` | the measurement and the STOP (§1.2) |
| `d596de2e` | the build to the 17:52Z rulings |
| `af671c0e` | `RCC_CR`'s `PLL1ON` by read-modify-write; Q8 and Q9 applied |
| `4108a0a4` | Q10 (a); the adversarial pass's fixes; the pins |
| `0aedba3d` | the twin judge without reads (found by simulating) |
| `f99462b8` | `main` `8d727980` merged; `sentinel-01-full`'s hwdef fixtures moved to solved 19 |
| `0a777ce3` | the bench package from dispatch `45252001498`; CI then red on four tier-2 tests |
| `a5eea862` | two tier-2 pins (the switch's `prove`; the campaign's droop plant) |
| `04813286` | Q11 (a), whole `RCC_CR` words and their acknowledgements; Q12, the bound to 40; the bench sheet's read-only rule |
| `de19b90b` | the tier-2 pins for whole words; FEEDBACK |
| `ea834e95` | the bench package from dispatch `45270122078` |
| `df37f064` | `main` `e5984c86` (1608) merged |
| (this) | READY: the CI line |
| `9666fa12` | the reduction pass |
| `2f3184b4` | the measurement re-run |
| `5dfd003b` | BENCH's `L…` citations moved with their lines (D6; 29 remapped, each to the identical line); FEEDBACK |

**Who wrote what:**
- I wrote the measurement, the generator, the IR's solved facts, the twin and CLI wiring, the goldens, and the review's fixes.
- **The independent rule** (`crates/aethelgard-ir/src/kernel_clock.rs`, its first 17 tests, and 54 mutants in `spike/4240e/mutants-rule.md`) was written by a separate Opus agent. It was given the manual, the IR's types and the rule's shape, and told not to open svd-hal or the solver's word choice. Its imports are only `clock::{Budget, SolvedKernelSwitch}`, `vos::Vos`, `SolvedTopology` and `SpiKernel`.
- **The adversarial pass** was a second, fresh Opus reviewer, given the code and the manual and not my conclusions (§3).

**The kit** (`~/.cache/aet-4240e/kit`):
- `img.sh`: capped-run, swap 0 read back, its own cargo home;
- `/cache/prove.sh`, `/cache/sim.sh`, `/cache/adcelf.sh`;
- the mutant runners `mutants.py`, `mutants-rerun.py`, `mutants-review.py` and `mutants-twin.py`.

## 1. The measurement and the stop (§1.1–§1.2, §1.4)

The record is `spike/4240e/RESULTS.md`; the predictions are `PREDICTIONS.md`, committed first.

**After the stop:**
- **P4.1 was partly wrong.** (a) also moved the solved topologies (D7) and needed one new acknowledgement (Q8).
- **I miscounted once:** I first wrote 27 boards for 26, and corrected RESULTS before it was committed.
- **Two pages in RESULTS are wrong**, found later and left as the record:
  - §6 cites `PLL1RGE` on p. 401. It is on p. 402 (text after p. 401's footer); the code cites p. 402.
  - §7 says a mux's input at entry is its reset input "after every system reset". That holds only from power-on: after a system reset a mux keeps an input the reset stopped (p. 372's USART1 example). The rule judges from power-on, and B10/B11 start from a power-on reset (§3).

**Rulings:**
- **17:52Z:** D1 (a); D2 granted (deletions); D4 planted topologies with the 26 pre-fix ones; D5 the three kinds; D6 the fault arm; D7 solved 19; the ADC (i).
- **18:30Z:** Q8 granted (one acknowledgement); Q9 granted (the twin judge; `aethelgard-twin/src/serial.rs` joins the fence for it).
- **20:58Z:** Q11 (a): whole `RCC_CR` words. On the six Q8 specs, `RCC.CR`'s `ignored-by-model` acknowledgement gains 12 fields and its `silicon-only` one gains `CSSHSEON`. Q12 (a): the 4241 bound goes to 40 instructions.
- **19:08Z:** Q10 (a): **D2 reverted.**
  - The reset selector rows went back to `ignored-by-model`, and their 3×7 acknowledgements were restored byte for byte.
  - Why: the adversarial reviewer showed those rows meet neither limb of `independent-rule`. The rule never reads them, and nothing in them can make it fail.
  - Each of the seven specifications now differs from `main` by Q8's one entry or not at all.

**§1.4, the re-run:** 0 of 32 boards start a peripheral unclocked, and the 26 switch with PLL1 running.

## 2. What was built

**IR:**
- `clock.kernel_switch` `{pllckselr, pllcfgr, pll1divr}` and `clock.adc` `{ckmode}`, each absent when unused. Solved schema is 19.
- `solve::hsi_switch_words`: across the HSI's whole band, `0x50` / `0x0002000C` / `0x0101020F`. That is `DIVM1` 5, ×16, Q ÷2, giving `pll1_q_ck` 99.7–103.9 MHz. `main`'s reset `DIVM1` 32 gives a 2 MHz reference, below the wide VCO's floor at HSI's slow edge.
- `solve::kernel_switch` is shared by `solve` and `derive_controllers`.
- `validate` sets both facts aside when it holds the tree to the reset one or to the HSE's.
- **`kernel_clock::check`'s refusals:** `KernelClockDeadSwitch`, `KernelClockNotRunning` and `KernelClockUnknown`, each naming its peripheral, selector, inputs and pages. Since the review it also refuses:
  - `PLL1DIVR` codes the manual does not allow, and `PLL1FRACEN`;
  - SPI1–3 solved on different kernels (one `SPI123SEL`).

**svd-hal:**
- **`select_spi_kernels`** writes the switch and returns `Result<SpiKernels, KernelClockFault>`.
- **The transition wait** is `SWITCH_READS` = 8, emitted straight-line. A loop there met every other harness's unwind bound through `spi_kernels_token`.
- **`main.rs`** gains the fault arm and `PLL_POLLS` = 9744 (150 µs at the HSI's fastest).
- **Refusals by name:**
  - `check_silicon` refuses by the rule's kind (boxed, for clippy);
  - `AdcClockNotGenerated`: an ADC with `clock.adc` absent or `CKMODE` ≠ `11`;
  - `SwitchQBelowPerCk`: `SWITCH_READS` is sized for a `pll1_q_ck` at least as fast as `per_ck`.
- **The Kani harness** holds every switch write to the contract, in order:
  - the PLL words are the topology's;
  - the selects are `silicon::spi_words`';
  - each `RCC_CR` word whole: `HSION|PLL1ON`, then `HSION` (Q11);
  - no select comes before `PLL1RDY` reads 1;
  - a lock that does not come is named, with nothing written after it.
- **Tests:**
  - `kernel_switch.rs`: the order, the fault arm, a live-input select, the HSE board, the ADC, a slow Q, the ledger rows;
  - `kernel_scar.rs`: each of the 26, clocked as solved, then put back as it was and refused by name with exactly the controllers the measurement found.

**Twin (Q9):** `RccRule.switch`:
- the switch's writes in order, from the topology and the header, each once per boot;
- every select inside the PLL1 window;
- `RCC_CR`'s two words compared exactly (Q11).

Two earlier forms went wrong. The first compared each `RCC_CR` write with the last read, but Renode's access log holds no reads, so the real boot failed `serial_tag_mismatch`; I found that by simulating, not by a test. The second masked the read-only flags, and Q11's whole words removed the need for it.

**CLI:**
- `ledger::Silicon.kernel_clock`, carried in the verdict, and `silicon_rule_fails` messages that name the kind.
- **The bench sheet's read-only rule:** `Entry.read_only`, taken from the table's own `read-only` entries (`RCC_CR`'s flags, `CFGR.SWS`). A whole word writes 0 into such a field and it reads back whatever the hardware says, so its written row is no bench prediction. It also no longer displaces the field's reset row, so `D2CKRDY` stays predicted at 1 and `PLL1RDY` at 0.
- The switch's PLL rows cite the rule; `PLL1ON`'s note names the switch.

**Specifications (Q8, Q11):** `RCC.CR` `silicon-only` `PLL1RDY` and `CSSHSEON`, and `RCC.CR` `ignored-by-model` widened from `D2CKRDY` to the 13 tag and read-only fields of the whole words, each with a reason naming why the twin cannot judge it. Each is added as text (every other byte checked equal by JSON) to:
- `examples/sentinel-01.json` (the bench package's);
- `examples/sentinel-01-minimal.json` (`fab_tier1`'s acknowledged reference; its tests refuse without it);
- the CLI fixtures `four-layer` (`four_layer_fab_tier2`), `logger-4600`, `sentinel-01-bench` (`board_tier2`) and `sentinel-03` (`battery_sag_tier1`, `board_tier2`). These are every acknowledged specification with the switch.

**BENCH.md (new rows only, plus the power-on clause in each):**
- **B10:** the switch on the Nucleo: `D2CCIP1R`, `RCC_CR`, SCK on PA5.
- **B11:** the ADC on `adc_sclk` with PLL2 off. Its ELF is `d1-drv8876`'s, sha256 `441a7ee785876c5981ce087d97f1e8db1acb2609d687921a49e7e991720cac4a`, built twice at two paths in the image, byte for byte, and passing `simulate`. The sheet may name only the package's ELF (ruling D3 on #362), so the digest lives here.

## 3. The adversarial pass (§5, full)

**The WO §5 items:**
- **A selector switched twice or after its peripheral starts, or a PLL stopped under a user.**
  - None on a generated board. The order is `scale2` → `reset_clock` → `select_spi_kernels` → every `enable_*`, and `enable_spi1` needs the `SpiKernels` token.
  - PLL1 is stopped only after the selects. No enabled peripheral's input is a PLL1 output on an HSI board, and the rule refuses one (`Pll1StoppedAfterWindow`).
  - The Kani harness, the twin judge and the scar test each hold this.
- **A rule that reads the generator's sequence.** None: `check` reads the topology and its own table. It shares `Budget::HSI` with the solver (stated in its doc) and nothing else.
- **A missed peripheral.** Every gate the generator can emit is a table row or a timer with no mux: ADC12/3, I2C1–4, SPI1–6, TIM1–5/8/12–17, USART1–3/6, UART4/5/7/8. Anything else enabled is `KernelClockUnknown`.

**The independent reviewer's findings, and what was done:**

| # | Finding | Done |
|---|---|---|
| 1 | `ledger_tier1` red (pins moved) | pinned after Q10, each with its reason (§4) |
| 2 | the generated `PLL1RDY` bench row expected 1, and the board reads 0 once the switch stops PLL1 | expects 0; the poll's 1 shows as a boot word that is not `B0F5`. Test and mutant: G19 |
| 3 | the warm-reset premise was misquoted; B10's conclusion depended on it | ledger text restored with Q10; B10/B11 start from a power-on reset, and B10's disagreement clause says why |
| 4 | a legal but slow `pll1_q_ck` (÷128, 1.56 MHz) passed and would have been stopped mid-transition | `SwitchQBelowPerCk`. Test and mutant: R06 |
| 5 | the reset selector rows did not meet `independent-rule` | Q10 (a): reverted |
| 6 | forbidden `PLL1DIVR` codes and `PLL1FRACEN` passed the rule | `CodeNotAllowed`, `FractionalMode`; `solve.rs`'s `DIVP1` claim corrected. Test and mutants: R01–R04 |
| note | SPI1–3 on different kernels passed (one select) | `SelectSharedApart`. Test and mutant: R05; the agent's two tests that used such a board were split |
| note | the ADC with `CKMODE` `01` needs `HPRE` = 1 on revision Y (p. 1039); the rule does not judge it | not built: generation refuses any `CKMODE` but `11`. §5 row |
| CI | my read-modify-write of `RCC_CR` (af671c0e) "saved" 14 ledger rows. It did not: it writes back every field it read, so the ledger owes those rows anyway. `twin_tier2`'s coverage test caught it on CI | Q11 (a): whole words, the rows acknowledged |
| found then | with whole words the bench sheet predicted read-only flags from their written 0 (`HSIRDY`, `D1CKRDY`, `D2CKRDY` 0, which silicon reads 1) | the read-only rule above. Mutants B01, B02 |

**Mutants:**
- **The rule:** 54 of 54 (`spike/4240e/mutants-rule.md`).
- **The build:** G01–G23 (`spike/4240e/mutants-gen.md`), all killed but G10, which is equivalent and was removed in the reduction.
- **The review's fixes:** R01–R06, 6 of 6.
- **The twin judge:** T03 (exact words) killed. T01–T02, for the masked comparison, went with the mask.
- **Q11's guards:** B01–B02 killed; L01 equivalent (recorded).
- **Not mutated on the host:** the Kani-only guards (the proofs' token replacement and the schedule footprint). The harnesses were proved in the tools image instead (§6).

## 4. Versions and what moved

- **Solved schema 18 → 19** (D7). These moved by `solve`, by the version and `kernel_switch` alone (the back-conversion in `solve_cli` holds the reference and the stress fixture to `1211c`'s digests):
  - the 9 committed topologies: `hwdef` ×6 with `sentinel-01-full` from `main`, `svd-hal ui` ×2, `pincheck` ×1;
  - the twin's and the CLI's topology literals.
- **Generated crates moved by their generators:**
  - `boards/sentinel-01/**` and `boards/sentinel-01-bench/**`;
  - the `svd-hal ui` boards `motors-pwm`, `motors-pwm-open` and `serial-four`;
  - one `ui/fail/*.stderr`.
- **Digests moved, each with a dated reason:**
  - `order_2203`: the platforms by their spec sha line alone, measured with `spike/4240e/order_back.rs`;
  - `serial_ports`.
- **`ledger_tier1`:**
  - 301 → 351 rows: +50. The switch's whole words add `PLLCKSELR` 5, `PLLCFGR` 18, `PLL1DIVR` 4 and `RCC_CR` 22 fields, plus `PLL1RDY` polled.
  - By class: `independent-rule` 75 → 102, `observed` 58 → 66, `ignored-by-model` 80 → 93, `silicon-only` 5 → 7.
  - 30 → 31 acknowledgement entries; 168 → 182 fields (183 rows: `D2CKRDY` is written and read under one field).
  - The bench sheet 148 → 153 rows: `PLL1RDY` (0), and `HSIKERON`, `CSIKERON`, `HSEBYP` and `CSSHSEON` (0); the four reset reads kept; no read-only flag.
- **`twin_tier2`'s 4241 bound:** 32 → 40 instructions (Q12). U4's status write moves 35: 2875 against 2840 with whole words, as built (2898 against 2863 when Q12 was asked, under the read-modify-write). The test's name keeps 32, because `ci/` routes it by name.
- **The CLI README's example line:** solved 19, the reference's hash `b62f2ad3…`.

## 5. Decisions and rows

- **The read-modify-write of `RCC_CR` (af671c0e) was a mistake, reverted by Q11.** I believed one bit left one row. It writes back every field it read, so the rows are owed either way, and whole words are judged exactly by the Kani harness and the twin.
- **Rows proposed for §7.1:**
  - **B10 and B11 on the Nucleo** (Scotty): each settles a reading, p. 371 and p. 920.
  - **Today's `main` firmware on the Nucleo from a power-on reset:** SPI1 SCK or none. That is the other half of roadmap §7.1's row.
  - **The rule does not judge `CKMODE` `01`'s `HPRE` premise on revision Y** (p. 1039). It is harmless while generation takes only `11`.
  - **The twin's judge holds `RCC_CR` against the word from reset,** so a later firmware that sets another `RCC_CR` bit before the switch would be named. That is right today; the next clock work order should know it.
- **The reduction pass (diff):**
  - −14 net;
  - 2 duplicates removed (the switch's oracle offsets are `ORACLE_RCC_HSE_WORDS`' last three; the notes test's acknowledgement lines come from one closure);
  - 1 dead path removed (G10's HSE check);
  - 1 protected kept (the rule's own select table beside `silicon::SpiSelect`, by `RUL-20`'s independence, stated in its module doc rather than labelled `belt-and-suspenders`);
  - 0 findings outside the fence.

## 6. Gates, CI, package

- **Local, at `df37f064`** (`main` `e5984c86` merged, with the package):
  - `cargo fmt --all -- --check`: clean.
  - `cargo clippy --workspace --all-targets -- -D warnings`: clean.
  - `cargo test --workspace`, in a capped scope: 3,005 passed, 0 failed.
- **In the tools image (Kani 0.68.0, Renode 1.17.0):**
  - **prove** on `boards/sentinel-01-bench`, each passing: `spi_kernel_switch_is_in_order_bounded_and_named`, `spi1_transfer_is_bounded_and_ordered`, `spi1_transfer_refuses_too_many_frames`, `every_valid_sequence_spi1`.
  - **simulate:** `sentinel-01-bench` (ELF `eb1d9bfe…`) and `d1-drv8876` (ELF `441a7ee7…`) pass, boot word `0x2100B007`.
- **The bench package** is dispatch run `45270122078` (`ci.yml`, `write`, at `de19b90b`), committed byte for byte; the first dispatch, `45252001498`, was superseded by Q11. Three files moved:
  - `sentinel-01-bench.elf`: `f3e871bb…` → `fadfa1ef…`, identical to my image build;
  - `MANIFEST.json`: its ELF and `FAB-NOTES.md` hashes, and the specification (`0cf10573…`), topology (`1f04dae2…`) and verdict (`59e26a4b…`) hashes;
  - `FAB-NOTES.md`: the specification hash (L4) and its `RCC.CR` acknowledgement lines (Q8, Q11), which the notes list like every other.
  `BENCH.md` L5 names the new ELF. Every file `ORDERED.sha256` lists is byte-identical (`sha256sum -c` on the artifact), and so are the other ten files.
- **CI:**
  - green at `df37f064`: `ci` run `45276505320`, every job, all 12 tools shards and `bench-package`;
  - green at `ea834e95` too, the package commit before `main` was merged: `ci` `45271847684`, `engine-image` `45271847618`, `ardupilot-image` `45271847669`;
  - red at `0a777ce3` (`45253373975`), the four tier-2 tests answered by Q11 and Q12 and the two pins (§3, §4).
