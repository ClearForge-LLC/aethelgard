# AET-WO-4240a — FEEDBACK (build)

Branch `wo/aet-4240a-hse-ir`, from `main` `d70a99e5` (the kickoff's base). The work order is `docs/work-orders/AET-WO-4240a.md` on the docs branch (copy at `~/.cache/aet-4240a-wo.md`); it is not on `main`.

Every answer is labelled **measured** (run here, the log or file named), **read at source** (document, revision, page) or **inferred**.

## §0 The answer in one screen

- **The §7 stop did not fire.** No solved word of an `hse: null` specification moved [measured]:
  - **Stage A** (one budget, F-PANIC; no schema change): all 27 committed specifications and 5 stored topologies byte for byte, with **no mask**. That covers `validate`, the whole solved topology, `SolvedTopology::validate` and every timing verdict.
  - **Stage B** (the schema): the same 32 under the mask declared before the runs. Only a topology's own `schema_version` (12 → 13) and `specification_sha256` (the version line) differ.
  - The CLI's `validate` and `solve` stdout and exit codes, 54 files, are identical under the same mask.
- **`mcu.hse` replaces `mcu.hse_hz`, in specification 20.** It is `null`, `{"crystal": {...}}` or `{"external": {...}}`. Version 19's `hse_hz: 0` reads as `null`, so the 27 committed specifications moved by their version line alone.
- **Solved schema 13.** `SolvedClock` gained one field, `hse`, which is absent for `null`. That is the only reason the solved schema moved.
- **The crystal row:** `ABM8AIG-16.000MHZ-12-2Z-T3` in `power-parts.json` (schema 5, kind `crystal`). Every figure is cited to ABM8AIG "REVISED 06-13-19", held by sha256.
- **The load capacitors: option (a).** They are declared by value; `4240c` adds the part.
- **The budget is one function**, `clock::declared_budget`. For the ABM8AIG-16 it is 20 + 50 + 2 + pull per side, each term cited; with a declared pull of 30 it is ±102 ppm.
- **The two HSE specifications solve to the predicted words** [measured, tier 1]:
  - 16 MHz crystal: `PLLCKSELR` `0x52`, `PLLCFGR` `0x0003_0004`, `PLL1DIVR` `0x0103_064F`;
  - 8 MHz external: `DIVM1` 5, `DIVN1` 159, `DIVP1` 3 (`0x52`, `0x0003_0002`, `0x0103_069F`).
- **The judge** (`clock::judge_tree`) reads `PLLCFGR` itself and decodes with the shared decoder (`SolvedClock::from_rcc`). It refuses each case by name; `Gm_crit` is judged against DS12117's `Gm_crit_max`, and every clock against its VOS limit at the HSE's fastest.
- **AN2867's 2.70× margin** is reported, uncited, and judges nothing.
- **Also built:**
  - PH0+PH1 are reserved beside a crystal, and PH0 alone beside an external clock;
  - `HseNotGenerated` at `generate`, `emit` and `simulate`;
  - F-PANIC is `NoI2cTiming`;
  - `OscillatorNotCited` has its first test.
- **No emitted byte or ELF moved** beyond the spec-sha lines [measured]:
  - all 66 KiCad files of 22 generated specifications are byte-identical;
  - every board-crate file differs by its "Solved topology of specification sha256" line alone;
  - the bench ELF is `e4e24372…`, on `main` and on the branch.
- **The bench package moves by the version line:** its `FAB-NOTES.md` and `MANIFEST.json` name the specification's sha256. **The architect dispatches** (ruled 10:05Z); until then `bench_board_tier1` fails by design.
- **`TIER2_MIN` does not move:** every new test is tier 1.

## §1 Predictions, scored

The predictions are `spike/4240a/PREDICTIONS.md`, committed at `d94a5782` before any run.

| # | Prediction | Result | Score |
|---|---|---|---|
| P1 | Stage A byte for byte | 27 + 5 identical, sha256 of the result files equal (`2ffa3df3…`, `0f01919a…`) [measured] | Confirmed |
| P2 | Stage B identical under the mask; serial verdict keeps `hsi_ppm` | 0 differences (`kit/compare.py`) [measured] | Confirmed |
| P3 | 3 `Infeasible` + 1 `ZeroNominalVoltage` stay | as on `main` [measured] | Confirmed |
| P4 | 16 MHz crystal: `0x52` / `0x0003_0004` / `0x0103_064F` | exactly [measured, `tests/hse.rs`] | Confirmed |
| P5 | 8 MHz external: `DIVM1` 5, `DIVN1` 159, `DIVP1` 3 | exactly [measured] | Confirmed |
| P6 | the selection rule gives P4 and P5 | it does: `solve::h1_words` is that rule | Confirmed |
| P7 | 25 MHz / 3 refused by name | `HseTreeUnreachable` [measured] | Confirmed |
| P8 | ABM8AIG-16 budget ±(20 + 50 + 2 + pull) | ±102 at pull 30 [measured] | Confirmed |
| P9 | `Gm_crit` 0.5546 mA/V; margin 2.70× | 554,644 nA/V (π² rounded up; 554,643.3 with π exact, computed apart); margin 2.70× [measured] | Confirmed |
| P10 | CRSF 416666 on ±102 ppm: margin near 22,600 | **22,600** (receiver 2,698, peer 8,002, Table 407's 33,300) [measured]; refused on the HSI as before | Confirmed |
| P11 | no `null` board's pin moves | 0 assignment differences in 23 solved topologies [measured] | Confirmed |
| P12 | 8 MHz H0: a named refusal at `FastPlus1M` | a named refusal (`NoI2cTiming`), **but at `Fast400k`**, the first mode to fail: its least kernel is 8 MHz, and 8 MHz at the HSI's slowest is 7.79 MHz | **Half** |
| P13 | only sha lines move; ELF unchanged; package by its version line | as predicted [measured, §2.3] | Confirmed |

## §2 Measured

### §2.1 The 32 files, `main` against the branch

- **The corpus:**
  - the 27 committed specifications (every `*.json` holding an `mcu` object and a `schema_version`, outside `spike/`, `fab/`, `docs/`, `target/`);
  - the 5 stored topologies.
  - That is `4240`'s 31 plus `examples/sentinel-01-ext-bmp581.json`, added by `3403` after it.
- **The harness** (`spike/4240a/kit/harness`) runs, per specification:
  - `validate` and `solve`;
  - `SolvedTopology::validate`;
  - the `Debug` text of `motor::check_silicon`, `serial::check_silicon`, `silicon::check_spi`, `silicon::check_i2c` and `vos::judge_generated`.
  - Per topology, it runs `parse`, `validate` and `vos::judge_generated`.
- **The baseline** is a `git archive` of `d70a99e5`, built apart. The results are in `~/.cache/aet-4240a/{base,stageA,stageB}`.
- **The CLI** (`kit/cli-sweep.sh`): `validate` and `solve` stdout and exit code, for `main`'s binary and the branch's.

| Stage | Specifications | Topologies | CLI | Mask |
|---|---|---|---|---|
| A (`06a0ccd1`) | 27/27 identical | 5/5 identical | — | none |
| B (`57a3a5c2`) | 27/27 identical | 5/5 identical | 54/54 identical, exit codes identical | top-level `schema_version`, `specification_sha256` |

### §2.2 Emitted bytes (`kit/gen-sweep.sh`)

- **`generate`** (the board crate and the KiCad emission) was run on all 27 specifications with `main`'s CLI and the branch's: 22 generate on both, 5 refuse on both.
- **Of 336 files:**
  - 138 are identical;
  - 176 differ by the line "Solved topology of specification sha256 `…`" alone;
  - the other 22 are `generate`'s stdout summaries, which hash those files.
- **All 66 KiCad files are byte-identical.**
- **`boards/sentinel-01*/`** were regenerated by `generate` and copied file by file, 8 files each, sha lines only (ruled 10:05Z). `fab/sentinel-01-bench/BENCH-EXT-BMP581.md` L9 names the specification-20 sha.

### §2.3 The ELF and the bench package

- **The ELF:** `boards/sentinel-01-bench` built with `aethelgard build` in the tools image (`kit`: `capped-run`, 8 GiB, swap 0 read back). It gives `e4e24372ef2d4bb4…`, on `main`'s board and on the branch's [measured]. That is the package's `sentinel-01-bench.elf` (`MANIFEST.json`).
- **The package** (`fab/sentinel-01-bench/package/`) moves by the specification's sha256 in `FAB-NOTES.md` and `MANIFEST.json`. `bench_board_tier1` refuses the committed package until the architect's write=true dispatch replaces it (ruled 10:05Z).

## §3 What was built

- **`aethelgard_ir::clock`** (new):
  - `Budget` (ppm per side; `Budget::HSI` the HSI's 14,984/25,894, unchanged);
  - `Hse`, `HseCrystal`, `HseExternal`, `SolvedHse`;
  - `declared_budget`, `gm_crit`, `judge_tree`;
  - the refusals `HseRefusal`, each its own kind through `IrValidationError::Hse`.
- **One budget.** `motor::HSI_FAST_PPM`/`HSI_SLOW_PPM`, `silicon`'s pair and `solve`'s pair are gone (`4240` D3, ruled). Two of them were labelled `belt-and-suspenders:`; D3's ruling replaces them.
  - Every check takes `SolvedClock::budget()`: serial (`serial::check_silicon`), DShot and PWM (`motor::check_silicon`), I2C `TIMINGR` (the solver's `synthesize_timingr` and the rule's `silicon::check_i2c_on`) and the VOS rule (`vos::judge_generated`, `flash_at_reset`).
  - Each site keeps its own rounding.
  - `silicon::check_i2c`/`check_timingr`/`fast_hz` keep their signatures on the HSI, for `aethelgard-svd-hal` (out of the fence); the generator builds only the HSI's tree until `4240b`.
- **The schema:**
  - `Mcu.hse` (`lib.rs`), with a reader that maps `hse_hz: 0` to `null` and refuses any other `hse_hz`, or both keys, at parsing;
  - `SolvedClock.hse` (absent for `null`; an explicit `null` refused);
  - `SPECIFICATION_SCHEMA_VERSION` 20 and `SOLVED_SCHEMA_VERSION` 13.
- **The crystal row:** `power-parts.json` schema 5, `Kind::Crystal`.
  - Fields: frequency, CL, tolerance, stability (over the row's temperature range), aging with the span it is stated for, ESR max, C0 max, drive level max.
  - The parser refuses a deviation range that does not straddle zero.
- **The solver** (`solve.rs`):
  - `solved_clock(mcu)` gives the reset clock for `null`, or H1 with `h1_words`' choice;
  - `reserved_pins(hse)`;
  - `NoI2cTiming` (F-PANIC);
  - `spi_kernel` no longer depends on the system clock: `hsi_ker_ck` runs whatever `SW` selects.
- **`validate`:**
  - step 2a judges the declaration and the solver's tree;
  - step 21 feeds the declared tree to the motor and serial checks;
  - `SolvedTopology::validate` keeps exact `reset()` equality for `null` and runs `judge_tree` otherwise.
- **The CLI:**
  - `generate::generable` (`CoreSupplyNotGenerated`, then `HseNotGenerated`) at `generate`, `emit` and `simulate`; `verify` and `campaign` pass through them;
  - `validate`'s acknowledgement check leaves a declared HSE's `acknowledge_unwitnessed` unjudged, as it does `bypass`'s.
- **hwdef:** no source change; `OscillatorNotCited`'s first test.

## §4 Decisions and deviations

- **D-1, `hse_hz` in a version-20 file.** "Replaces" and "version lines only" reconcile only if version 20 still reads `hse_hz: 0` as `null`, so it does.
  - Any other value is refused at parsing, as is `hse_hz` beside `hse`. Neither key present is `missing field hse`.
  - A specification written back by the engine says `"hse": null` (only `campaign` serializes one, and it hashes the result, which the version line moves anyway).
- **D-2, the shape.** The object carries one key, its mode (`{"crystal": {...}}`), the precedent of `esc`'s `{"none": {...}}`, rather than `4240` D4's `"mode"` field. Externally tagged, it refuses a second mode or an unknown key at parsing.
- **D-3, the load capacitors: (a), by value** (`load_caps_femtofarads: [CL1, CL2]`). No cited `Cs` exists until `extract` bounds it (`4240c`), so (b) would have needed an uncited `Cs`.
  - After the adversarial pass (A3), the values are judged as far as they can be without `Cs`: `CL1` = `CL2` (AN2867 Rev 10 §3.3's assumption), and `CL1·CL2/(CL1+CL2)` at most the row's CL, since stray capacitance only adds (`HseLoadCapsUnfit`).
  - Their remaining effect on frequency is inside the declared `pull_ppm`.
- **D-4, the row's table.** The crystal is a `power-parts.json` row, not a `parts.json` row: that table is for bus devices with a Renode model.
  - Its symbol and footprint name KiCad 10.0.7's `Crystal:Crystal_GND24` and `Crystal:Crystal_SMD_3225-4Pin_3.2x2.5mm`, neither vendored yet (`4240c`). No test binds a power row to a vendored symbol [read].
  - Its `lifecycle` is `unread`: no maker page with a lifecycle field was read.
- **D-5, aging's span:** "First year" (p. 1) is entered as 8,760 h, a year taken as 365 days [inferred]. A declared life beyond it is refused (`HseLifeBeyondAging`), never extrapolated. The row's aging is stated "@25 °C ± 3 °C"; the declared range is wider, and no held sheet gives aging over temperature.
- **D-6, pull 0 is refused** (`HseDeclarationZero`), as is any declared zero (life, a load capacitor, an external `ppm` or `hz_num`/`hz_den`). A crystal under real capacitors always pulls; a zero would be a number standing in for an absent one (`INV-06`).
- **D-7, `Gm_crit`'s citation is split.** The 1.5 mA/V threshold is DS12117 Rev 10's (Table 33 rev Y p. 125 = Table 130 rev V p. 229, read at source). The formula, `4 · ESR · (2πF)² · (C0 + CL)²`, is AN2867 Rev 10 §3.4's: DS12117 states only the threshold and refers to AN2867 [read]. The refusal is built as the work order asks, and its message names both halves.
  - The judgement rounds against the crystal: ESR and C0 at their maxima, π² rounded up to (355/113)², and the quotient rounded up.
  - The margin, Gm_crit_max / Gm_crit, is AN2867's and is reported only (`an2867_margin_hundredths_uncited`).
- **D-8, the VCO's wide range is 192–836 MHz**, revision Y's maximum (DS12117 Table 39, p. 130), the stricter of the two revisions (`4240` F-REV). H1's 256 MHz is under both.
- **D-9, SPI stays on `hsi_ker_ck`** (`per_ck` for SPI1–3) and the HSI's budget on an HSE board.
  - The work order lists serial, DShot, PWM and `TIMINGR`; SPI's kernel is the HSI's whatever the system clock, so the HSI's budget is the true one.
  - `4240` D6 moves every select to its reset value (`pll1_q`, `pclk`) in `4240b`. Adding the `SpiKernel` variants now would break an exhaustive match in `aethelgard-svd-hal` `board.rs`, outside this fence.
- **D-10, the solver requires a whole-Hz reference** (`4240` §6: Renode divides in integers). A non-whole source therefore never reaches a tree, and `SolvedClock.hse_hz` stays a whole `u32`.
- **D-11, the crate READMEs.** `aethelgard-ir/README.md` (its version lines are pinned by `readme_versions.rs`; the new kinds; the reserved pins) and `aethelgard-cli/README.md` (`HseNotGenerated`) were edited: the tests require the first, and the second states the refusal. Neither is in the fence's list. The root README is untouched.
- **D-12, `belt-and-suspenders:`.** The labelled copies in `silicon.rs` and `solve.rs` went by D3's ruling, not by the reduction pass.

## §5 Adversarial pass (full)

A fresh Opus subagent, pinned to `24d21b42`, given the code and the work order's §5 list, not my conclusions. Its findings are in `~/.cache/aet-4240a/adv/findings.md`, and its probes in `~/.cache/aet-4240a/adv/src/crates/aethelgard-ir/tests/adv.rs`. Each finding below was reproduced or re-read before it was acted on.

| # | Finding | Severity | Done |
|---|---|---|---|
| A1 | `ppm`/`pull_ppm` ≥ 1,000,001 panicked `validate` and `solve` (`serial.rs` `1_000_000 - slow_ppm`; `motor.rs` the same in `u32`); release builds wrap | blocker | **Fixed** (`b19e8706`): a budget side of 1,000,000 ppm or more is `HseBudgetUnbounded`, and both sites saturate. Tested (`an_unbounded_budget_is_refused_not_a_panic`) |
| A2 | The `Mcu` reader buffered values as `serde_json::Value`, so a duplicate key nested under `mcu` (`core_supply.mode`, `supply_source.pin`, `foreign_clock.usarts`, `hse.crystal.pull_ppm`) was accepted, last one winning: **a regression on `null` boards** | should-fix | **Fixed**: read through a derived wire struct (`McuWire`), so the inner derives refuse duplicates as on `main`. Tested (`a_duplicate_key_under_mcu_is_refused`) |
| A3 | `load_caps_femtofarads` was never read: caps of 100 pF passed, and `Gm_crit` at that load is 7.65 mA/V | should-fix | **Fixed**: `HseLoadCapsUnfit` (D-3). Tested |
| A4 | The judge read only some fields: `DIVR1` 0 (RM0433 p. 403 "Not allowed"), reserved bits, PLL2/PLL3 fields, and an unjudged `pll1_q` of 768 MHz | should-fix | **Fixed**: any bit outside the read fields, and `DIVR1` 0, is `PllWordsUndecodable`; `pll1_q_ck`, SPI1–3's kernel from `4240b` (D6), is held to that kernel's Table 60 limit at Scale 2 (`VosLimitExceeded`). Tested |
| A5 | The VOS rule judged the SPI kernels (`hsi_ker_ck`) at the HSE's budget, contradicting D-9; no verdict flipped | should-fix | **Fixed**: SPI families are judged at `Budget::HSI`. Tested |
| A6 | The judge and the solver share `declared_budget`, `source_hz` and `crystal_row`, and the rounding of `Budget`. This replaced the `belt-and-suspenders` copies; the README said "sharing no code" | ruled (a) | **Ruled (a), 11:10Z (§7 F1).** The README now says the judge shares no word-choice code, and that the budget is one function by D3's ruling. The hand-written figures in `tests/hse.rs` (102 ppm, the words, 22,600) and `clock.rs`'s HSI test stand in for the removed copies; mutants M2, M3 and M9 show they bite |
| A7 | A consistently edited topology validates: the declaration copy, the budget and the caps changed together | note | §7 F3, `4240b` |
| A8 | `HseExternal.source` was documented "F-CITE strict", but only its form is checked | note | **Fixed** (the doc now says so) |
| A9 | Aging is stated "@25 °C ± 3 °C, first year" (from manufacture); the budget applies it over the declared range and service life | note | D-5 |
| A10 | `simulate --foreign` refuses an HSE specification (`generable` runs in `ready()`), and `judge_foreign` says "no deviation is declared" for `PLLSRC` = HSE | note | §7 F4, `4240b`/`4400` |
| A11 | External clocks from 4 to 50 MHz in 1 kHz steps: 178 reach H1, 45,823 are `HseTreeUnreachable`, 0 judge refusals, no panic. 26 MHz and 4.000 MHz are unreachable; 19.2 MHz reaches it | note | On record here (it follows from H1 plus F-RGE) |

**Attacked and held** (the subagent's list, with spot-checks by the builder):
- the crystal row against the rendered PDF (also re-read by the builder: p. 1 C0 2.0 pF in the Max column, ESR 70 Ω Max);
- DS12117 Tables 31, 33, 39 and 136;
- `gm_crit`'s units, rounding and overflow;
- the budget's sides, and the temperature and life edges;
- `hz_den` exactness, and PLL1RGE strictness in both solver and judge;
- `from_rcc`'s flooring, caught by the judge's exact check;
- the judge never calls `h1_words`;
- the legacy reader (`-0`, `0.0`, `"0"`, `null` and `1` refused);
- `reserved_pins` on both solver paths;
- `HseNotGenerated` reached from generate, emit, simulate, campaign, verify, fab and prove (mcp inherits it; `generate hwdef --topology` refuses as `OscillatorNotCited`);
- the `null`-board timing paths.

**After the fixes, the `null` boards were measured again:** `kit/compare.py` against `main`'s baseline, 0 differences in 32 files.

## §6 Mutants

**31 mutants, one per guard this work order adds or changes, all killed** (`spike/4240a/kit/mutants.py`, in a copy of the tree, each against the suites it lives in):
- **The three the work order names:**
  - M1, a reference on a boundary passes: killed by `the_judge_refuses_each_tree_by_name`;
  - M2, the budget drops aging: killed by `the_16_mhz_crystal_solves_to_the_predicted_words_and_budget` and the `Gm_crit` test;
  - M3, the HSI's ppm for a crystal board: killed by three tests, the VOS one among them.
- **The rest:**
  - M4–M10: each judge refusal;
  - M11–M16: each declaration refusal;
  - M17–M18: PH1 and PH0;
  - M19: the solver's strictness;
  - M20: F-PANIC;
  - M21–M22: the legacy reader;
  - M23: a PLL1 clock without its tree;
  - M24: `HseNotGenerated`;
  - M25–M31: the adversarial fixes.
- **Two survived the first run, and were closed by new tests:**
  - **M7 (`PLLSRC`):** the shared decoder refused my test's input first. A new input (HSI as the source, words that decode to exactly 64 MHz) reaches the guard alone.
  - **M15 (`Gm_crit`):** the refusal had no test, only the arithmetic. `crystal_budget(c, row)` was split out so a test feeds it the 8 MHz CL 18 pF row.
- The first harness run also misread `cargo test -q` output as survivals; it was fixed to read the per-test lines before these results were taken.
- **Rerun on the reduced tree** (`a5a19433`): 31 of 31 killed. M21 and M22 were re-anchored on the wire-struct reader, which A2 introduced.

## §7 Findings

- **F1 · `aethelgard-ir` `clock.rs`, `solve.rs`, `silicon.rs` · the budget's independence (A6) · decision-needed.** D3 ordered one budget function, so the solver and the judges share it, and the `belt-and-suspenders` copies in `silicon.rs` and `solve.rs` went.
  - The independence `RUL-20` asked of them now rests on hand-written figures in the tests: the HSI test in `clock.rs`, and 102 ppm and 22,600 in `tests/hse.rs`.
  - (a) Accept: this is D3 as ruled.
  - (b) Restore a second copy for the judges.
  - Lean: (a). The budget is a fact of the part and the declaration, not a choice the solver makes.
  - **RULED (a)** (architect, bus 2026-11-20 11:10Z): one budget shared by the solver and the judges inside `aethelgard-ir` is what `4240` D3 recommended and the split accepted. `RUL-20` is kept because the generator (`aethelgard-svd-hal`) keeps its own copy.
- **F2 · `power-parts.json` · ABM8AIG's 70 Ω is labelled "(R1)" · information.** If that is the motional resistance, AN2867's ESR is R1·(1 + C0/CL)² = 95.3 Ω, and `Gm_crit` is 0.755 mA/V: still under 1.5 mA/V [inferred]. The row cites the sheet's figure as ESR max, as its own table heading names it.
- **F3 · the topology's copied declaration (A7) · for `4240b`.** A topology edited consistently (its declaration, its budget, its caps) validates. A path that reads only a topology (`generate hwdef --topology`, `check pinout`) trusts the copy. `4240b`'s generator should re-solve from the specification, or compare the copy against `spec.mcu.hse`.
- **F4 · `simulate --foreign` on an HSE specification (A10) · for `4240b`/`4400`.** It is refused `HseNotGenerated`, and `vos::judge_foreign` names no deviation for `PLLSRC` = HSE. A foreign HSE board waits for those work orders.
- **F5 · the re-serialized specification · information.** `campaign`'s `proposed-spec.json` and any specification the engine writes say `"hse": null` where the base said `hse_hz: 0`. Those bytes move by that key as well as by the version line. They are hashed, not compared to a committed file.
- **F6 · `fab_tier1` `a_rail_or_bus_no_fault_kind_reaches_is_named` · environment.** Run locally with `CARGO_TARGET_DIR` outside the repository, its scenario base is outside the CLI's root (`BaseOutsideRoot`). It is the `ci-img` lesson (the target in the repository in CI); not touched by this work order.
- **F7 · the bench package's version line · for the architect's dispatch.** As ruled 10:05Z.
- **Backlog, unchanged:** the HSI's 14,984/25,894 are not the plain sum of Table 36's terms (`4240v` F5); `Budget::HSI` keeps them.

## §8 Reduction pass

**Reduction pass (diff): −17 net · 2 duplicates removed · 2 dead paths removed · 0 protected kept · 3 findings below.**

A fresh subagent proposed the cuts, reading only; the builder ruled on each and made the edits (`reduction` commit).
- **Removed:**
  - `solve::i2c_timing`, a wrapper over `i2c_controller` (two mechanisms, one now);
  - `solve::hsi_fastest` and `HSI_KER_HZ`, wrappers over `Budget::HSI` used twice;
  - `GmCrit.max_nano_amps_per_volt`, a stored copy of `GM_CRIT_MAX_NANO_AMPS_PER_VOLT`;
  - `TreeVerdict.vco_range_hz`, never read;
  - a stale doc line on the caps.
- **Proof:**
  - every IR test passes;
  - clippy is clean;
  - the 31 mutants were rerun on the reduced tree (§6);
  - the `null`-board measurement is byte-identical to Stage B's.
- **Kept:**
  - `GmCrit.an2867_margin_hundredths_uncited`: the work order asks for the 2.7× figure shown as information;
  - `Crystal.drive_level_max_microwatts`: a cited bound for the first board's bench measurement (AN2867 §3.5), which `4240c` or the bench reads;
  - the three `HsePartNotCrystal` sites: one is the only path a test can feed a non-crystal row through, and merging the rest is churn.
- **Rejected:** the solver's and the judge's PLL ranges are the deliberate `RUL-20` pair (neither states one twice), and `check_i2c`/`check_timingr` beside their `_on` forms both have callers in `aethelgard-svd-hal`.
- **Reported, not cut (outside the diff):** `silicon::fast_hz`/`slow_hz` and `vos::hsi_fastest` are thin wrappers over `Budget::HSI` with callers in other crates; a later work order may fold them.

## §9 Gates and CI

- **Local, at `a5a19433`** [measured, `~/.cache/aet-4240a/final.log`]:
  - `cargo fmt --all -- --check`: 0;
  - `cargo clippy --workspace --all-targets -- -D warnings`: 0;
  - the `gates` job's 110 test targets (`ci/gates.sh targets`, run `--no-fail-fast`) and the doctests: **2,356 passed, 2 failed, both known.**
    - `bench_board_tier1` refuses the committed bench package, made from specification 19, until the architect's write=true dispatch replaces it (F7, ruled).
    - `fab_tier1`'s `a_rail_or_bus_no_fault_kind_reaches_is_named` fails only with the target directory outside the repository (F6).
- **Tier 2** runs in CI. No tier-2 test was added, so `TIER2_MIN` stays 194.
  - Before the push, the repository was grepped for every committed specification's and every solved topology's `main` sha256 (outside `spike/` and `docs/`). Only the bench package names one.
- **CI:** at the PR; its run ids go in §10 at READY.
- Reduction pass (diff): −17 net · 2 duplicates removed · 2 dead paths removed · 0 protected kept · 3 findings (§8).

## §10 State at READY

- **AWAITING-PACKAGE.** Pushed for CI and for the architect's write=true bench-package dispatch. The package is then committed byte for byte, CI must be green, and a READY commit closes this section.
- **The specification version:** 20 is this branch's (ruled 11:10Z: `1210` renumbers to 21 after this). Were `1210` to merge first, `python3 -I spike/4240a/kit/renumber.py . 20 21 13 13` renumbers by version lines only, and `boards/` and `BENCH-EXT-BMP581.md` L9 are regenerated as ruled.
