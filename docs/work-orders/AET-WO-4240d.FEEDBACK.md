# AET-WO-4240d — FEEDBACK (the bypass bench, prepared)

## §0 The answer in one screen

- **The bench specification** is `examples/sentinel-02-bench-hse8.json` (sha256 `fefb053a…53c5`). It is `3304`'s `sentinel-02-bench` with:
  - `mcu.hse.external` at 8,000,000/1 Hz, ±80,000 ppm;
  - a 20,000 µs start-up bound, as ruled 2026-11-20;
  - `board.drawn: false`.

  `spike/4240d/derive.py` derives it byte for byte from the base. It validates and solves at `main` `7fc81640` (specification 23, solved 16).
- **ppm is cited, and was ruled a held source** (Q1, 2026-11-22T04:00Z):
  - RN0093 Rev 40 §4.16 p. 13: the ST-LINK-V3E's `MCO` default is "HSI / 2 = 8 MHz";
  - MB1364's `ST_LINK_V3E.SchDoc`: the F723's `STLK_MCO/PA8` (ball F15) is its `MCO1`;
  - DS11853 Rev 8 Table 42 p. 130: the F723's ACCHSI is −8/+4.5 % over −40…105 °C. The −8 % side is declared.

  DS11853 was fetched from a distributor's mirror, because st.com refuses Furnace (measured 03:43Z):
  - Rev 8 from `cdn.components.ru/media/PDF/1368336.pdf`, sha256 `3b81519fdf140b610a1351ee55129ae2d57c98b7493d89a647d3e8f4ba80fec7`;
  - Rev 7 from `static.chipdip.ru/lib/529/DOC010529813.pdf`, sha256 `796622f99c268e7aed386edb6834d5d73c4cd6d2c6c13a55b14a5e98bf276c54`, which agrees.

  Rev 8's currency is unconfirmed (F1).
- **The firmware** is ELF `4823bd30503416da7dc84e2178433a1d6d9b7dd8f2960b3ae69b4640196d6530`.
  - It was built by `main`'s CLI in the pinned image, twice, at two paths, with one sha. The board crate's input hash is `c669cefc…e53a`.
  - `simulate --for 100ms` gives **`pass`**, no findings, with image hash `f71e355d…6d95`.
  - The boot word `0x2100b007` lands at 41,539,110 ns, 2.499 ms after `main`'s `sentinel-02-bench`.
  - It is left at `~/.cache/aet-4240d/bench/` with `SHA256SUMS`, as `3304` did.
- **The sheet** is `AET-WO-4240d.bench.md`, in `3304`'s form. B-0 and B-1 come first, as the gate.
  - B-1 routes `MCO1` = HSE ÷ 4 to `PA8` on the HSI-only `sentinel-02-bench.elf` (`3f7b0507`), then counts 1 s of edges.
  - It stops at 0, within ±0.1 % of HSE/3, HSE/4 or HSE/5, or outside 2,000,000 ± 4 %. That is the gate as ruled (Q2).
- **The sheet's sharp predictions are ratios to the source,** because the source is an RC oscillator:
  - `MCO1` = `pll1_q` ÷ 15 against HSE ÷ 4 is 32/15, bracketed by two HSE captures;
  - SPI SCK equals HSE ÷ 4.

  Both rest on RM0433's integer PLL formula over the words read back, which `clock::judge_tree` judged.
- **Fence kept.** The only files changed are the specification, the sheet and this file, `spike/4240d/`, and one test row: `hse_eda_tier1`'s count of HSE specifications, 2 → 3. No generator, IR, twin, `fab/`, workflow or `ci/` file changed. `TIER2_MIN` is unchanged.

## §1 Predictions, scored (`spike/4240d/PREDICTIONS.md`, committed `f73ad74c` before any run)

| # | Prediction | Result |
|---|---|---|
| P1 | `generate` writes `board/` and no `kicad/`; the platform has `hseFrequency: 8000000` | **Confirmed**: "not written: `board.drawn: false` …"; `.repl` L109–110 |
| P2 | The bring-up's words, as `hse-external-8`'s | **Confirmed**: generated `board.rs` `Hse::start`, `CR` `0x0004_0001`, `0x0005_0001`, `0x000D_0001`, `0x010D_0001`, PLL words, `ACR` `0x11`, `CFGR` `0x3` |
| P3 | Pre-switch bounds unmoved by ppm | **Confirmed**: `HSE_POLLS` 1,299,180, `PLL_POLLS` and `SWS_POLLS` 9,744 |
| P4 | The 20 ms wait at 69.12 MHz: 1,382,400 spins | **Confirmed**: `settle(1382400)` |
| P5 | One sha at two paths, equal to neither `3f7b0507` nor `762c304d` | **Confirmed**: `4823bd30…6530` |
| P6 | `pass`, `B007`, no `clock_access`, records equal `main`'s | **Confirmed**: every record, witness line and register row equal |
| P7 | Boot word 1.5 to 3.5 ms later | **Confirmed**: +2,499,230 ns |
| P8 | The verdict names the HSE residual and the unobservables | **Confirmed**: both residuals (PWR, HSE), 14 unobservable fields |
| P9 | Only `hse_eda_tier1`'s count moves; `TIER2_MIN` unchanged | see §6 |

**Measured before the predictions, and stated as such there:** validate and solve exit 0; the solved words; SPI1 on `pll1_q` at `MBR` 4; `I2C1.TIMINGR` Fast400k `0x10D8_1340`.

## §2 Measured

- **The specification's sha moved after `PREDICTIONS.md`** (`a5f09d12…` → `fefb053a…`). That is Q1's ruled `source` text, and `PREDICTIONS.md` is left as committed. `main` `171f2a60` (docs only) is merged at `486d6641`.

- **The build of record** is at `main` `7fc81640`, merged at `0e3c17c1`, with the ruled `source` text. The run before the ruling, at `e3aea224`, gave the same ELF. Neither the source string nor 1211b's merge reaches the ELF; the board crate's input hash moved (`fa061817…` → `c669cefc…`) by the specification's sha lines.
- **The baseline:** `main`'s `sentinel-02-bench` in the same run gives ELF `3f7b0507…`, the ELF `4202`'s note names, and `pass` at 39,039,880 ns.
- **Kit:** `~/.cache/aet-4240d/kit/`. It holds `img.sh` (capped-run, 8 GiB, swap 0, one container, nice 10), `run.sh` and `sim.sh`. The logs are `run2.log` and `sim2.log`.

## §3 Rulings applied

- **Q1 (2026-11-22T04:00Z): a held source, on the AN2867 precedent.**
  - The specification's `source` names the mirror, the sha prefix, Rev 8, Table 42 p. 130, that Rev 7 agrees, and that currency is unconfirmed (195 bytes of the field's 200).
  - The full URLs and shas are in §0.
  - ppm is 80,000.
- **Q2: the B-1 gate as proposed.** It is written with the reason the ruling gives: an RC source has no "8.000 within the analyzer's resolution", and HSE/3 is told from HSI/2 by its exact ratio. B-0's version corroborates.

## §4 Decisions and deviations

- **D-1, B-1 runs on a known image.** It flashes `main`'s HSI-only `sentinel-02-bench.elf` (`3f7b0507`) if the board runs anything else.
  - Whatever is on the board now is not known to this builder. An image that has already started the HSE would make `HSEBYP` unwritable (RM0433 p. 384) and the read-modify-write words wrong.
  - That image never writes `RCC_CR` or `RCC_CFGR`, so B-1's reads are predicted from reset values.
  - This is a flash of `3304`'s image, not of the hse8 ELF, so the gate's "do not flash" still holds for the hse8 ELF.
- **D-2, every write is a literal word, conditional on its read.** It is the read-modify-write of the predicted read, and the sheet says to stop and record if the read differs. No write is computed at the bench.
- **D-3, N = 15 and a bracket.** The WO asks for `MCO1` = `pll1_q` ÷ N with N chosen for the FX2.
  - N = 15 is the largest prescaler. It keeps ≥ 2 samples per phase at the declared fastest with no duty cycle assumed (RM0433 states none), and stays under the pin's 12 MHz at reset speed (DS12117 Table 149).
  - The prediction is the ratio C2 / C1 = 32/15, bracketed by two HSE ÷ 4 captures, because the F723's HSI drifts and the analyzer's timebase is uncited. Both cancel in the ratio.
  - `edges.py synth` checked the count on a synthetic capture.
- **D-4, the SPI step's "repeat" has nothing to repeat yet.** `3304`'s bench has not run: there is no result on `main` or the bus. The sheet sets SCK = C1 (rule) and 2.000 MHz absolute beside `3304`'s predicted 2.000 MHz at the HSI. A measured `3304` rate is recorded beside it only if both run in one session.
- **D-5, no `verify`.** `board.drawn: false` has no board for ERC, DRC or extract, so this ELF carries no `INV-03` claim. The sheet says so, where `3304`'s carried one.

## §5 Findings

- **F1, a §7.1 row (ruled with Q1):** re-check DS11853's current revision when st.com is readable. Only Rev 8 (3-Feb-2022) is held, from a mirror.
- **F2, `probe-rs write`'s help text says "Only supports RAM addresses"** (`cmd/write.rs`, v0.32.0). The command writes through the same `write_memory_32` as any address. The sheet reads back after every write and stops on a write that did not take. If it does not take on silicon, the clock-out steps need another tool; that is a bench result, not a guess here.
- **F3, the bench cannot time `HSERDY`.** `4240b` F3 said the bypass bench "measures the real `HSERDY` time and so tests the ruled 20,000 µs". It does not: nothing toggles a pin before the switch, and `probe-rs` reads after the boot. It shows only that `HSERDY` came within the bound. Timing it needs a pin toggled around the wait, which is a generator change and is fenced here. `4240b` F3 stays open.
- **F4, `probe-rs list` does not print the firmware version** (`cmd/list.rs` prints the probe's `Display`). B-0 reads it from the ST-LINK's `DETAILS.TXT` instead (RN0093 §4.16 p. 13), and from `lsusb`.
- **F5, a ledger finding outside this fence (adversarial F4).** `ledger.rs` classes every field of the bring-up's whole `RCC_CR` and `RCC_CFGR` words as `independent-rule` through `clock_rule` (L744, L857–905). Those fields are `HSION`, the PLL2/PLL3 bits and `CFGR`'s other fields.
  - That rule is `aethelgard_twin::hse::judge_clock`, a twin-side judge of the access log, not an `aethelgard-ir` rule as `RUL-20` (1) requires.
  - The sheet labels those fields generator and says why. The class is the ledger owner's call, so nothing is changed here.
- **F6, CN12's fitting is contradictory at source.** UM2407 Rev 6 §9.2 p. 43 says CN11 and CN12 are "both soldered by default". MB1364's BOM row 106 (H753ZI C-01 and E-01) says "Not Fitted".
  - `3302` and `3304` used CN11/CN12 pins on this board, so it is probably fitted.
  - The sheet looks first, falls back to TP4 by hook clip, and otherwise stops.
- **F7, SPI SCK stalls between bytes on silicon** (adversarial F1). The generated driver writes one byte and waits for `RXP` before the next, and RM0433 p. 2176 says the master's clock "stops, waiting for additional data" when the TxFIFO empties.
  - So `3304` B6's SCK row, read over a byte, is right, but any frame-span average would read low.
  - The gap is unpredicted, and no source here times it.

## §6 Gates and CI

- **Local, at `59c008f1`** (main `7fc81640` merged; every later commit is docs or `spike/` only):
  - `cargo fmt --all -- --check` clean;
  - `cargo clippy --workspace --all-targets -- -D warnings` clean;
  - `cargo test --workspace`: 156 binaries, 2,624 passed, 0 failed.

  The gates ran under a capped scope (32 GiB, swap 0).
- **Every test that walks `examples/` passes on the new specification:**
  - `hse_eda_tier1::no_committed_specification_without_a_crystal_draws_one`, now counting 3;
  - `silicon_rules::every_solved_fixture_s_controllers_hold_under_the_independent_rules`;
  - `ledger_tier1::every_specification_that_solves_has_a_ledger`;
  - `hwdef_tier1::the_board_crate_s_speeds_equal_the_stored_ones`;
  - `svd-hal` `silicon_words`;
  - `board_names`.
- **No CI job loops over `examples/`.** `ci.yml` and `engine-smoke.sh` name their files. `ardupilot-image` triggers on `examples/**` paths only.
- **P9 confirmed.** No tier-2 test is added, so `TIER2_MIN` is unchanged.
- **Mutant** (the one guard this WO changes): with `mcu.hse` set to `null` in the new example, `no_committed_specification_without_a_crystal_draws_one` fails (left 2, right 3). Restored, it passes, and the sha is back to `fefb053a…`.
- **CI:** see §9.

## §7 Adversarial pass (targeted)

A fresh Opus subagent read the sheet at `59c008f1` against the primary PDFs, the generated firmware, the ledger, and the probe-rs and libsigrok sources, not against this builder's reads. Its findings are in `~/.cache/aet-4240d/adv/findings.md`. **No step can harm the board.** The builder re-read F1, F2, F6, F4's `TIMINGR` class, N2's p. 3195, N10's §4.11 and F1's p. 2176 at source before adopting them.

| # | Finding | Severity | Done |
|---|---|---|---|
| F1 | B5's SCK rate averaged a frame's span across the stalls between bytes (−4 to −7 % on a synthetic capture with a 333 ns gap) | blocker | **Fixed**: `edges.py frames` measures within bytes (7 periods per 8 rising edges), band 1.5 %, the gap a not-predicted row (F7) |
| F2 | Word 0 is `NOLOAD` and survives a warm reset, so a parked boot shows the last run's `B007` | should-fix | **Fixed**: word 0 is written 0 and read back before every reset whose word is read; B-1 always flashes `3f7b0507` |
| F3 | `PWR_CSR1`/`D3CR` were labelled rule; the ledger has `VOS` rule and `ACTVOSRDY`/`VOSRDY` silicon-only | should-fix | **Fixed**: four rows, each with its class |
| F4 | `HSION`, PLL2/3 and `CFGR`'s other fields: the ledger says independent-rule via the twin's clock rule. `TIMINGR` is rule (`check_timingr`) | should-fix | **Fixed**: the legend explains, F5 here; `TIMINGR` relabelled |
| F5 | Reset values were labelled rule | should-fix | **Fixed**: a `manual` label; `twin (Renode reset)` defined |
| F6 | CN12 may not be fitted (BOM vs UM2407) | should-fix | **Fixed**: look first, TP4 fallback, else stop (F6 here) |
| F7 | C2's ±3 was under its own quantization worst case (3.13) | should-fix | **Fixed**: ±4, the monotonic-drift assumption stated |
| F8 | B4's preconditions had no whole-word `MODER`/`AFRH` reads in B3 | should-fix | **Fixed**: two rows |
| N1–N10 | read-backs; `DBGMCU` not reset by a system reset; "no halt" unproven; "output disabled"; X1's 25 MHz ±50 ppm; `SH_REG`; `details.txt` case; a stop still restores; the pre-boot frame; the default after V3J7 inferred, and E normalised | nit | **All fixed** |

**Round 2** (the same agent, given the diff `59c008f1..fdd518f3` only). Every new row, mask, address and label held except:

| # | Finding | Severity | Done |
|---|---|---|---|
| R2-1 | `frames` chunked edges by 8 blindly: a spurious plus a missed edge in one frame printed −15.6 % unflagged; an open frame at the capture's end was dropped | should-fix | **Fixed**: no rate for a frame whose edges are not whole bytes, whose byte's 7 intervals differ by more than 2 samples, or that is unterminated; each named. Re-tested on clean, spurious, spurious-and-missed and open captures |
| R2-2 | TP4 is a bare 1 mm pad (MB1364: "Test Point", mechanical), so a hook clip cannot hold | should-fix | **Fixed**: a probe tip or pogo pin, off neighbouring pads; a slip means a repeat. CN7 pin 8 confirmed GND (UM2407 Table 18 p. 40) |
| R2-3 | The gap figure included one SCK period | nit | **Fixed**: `max_stall_samples`, the boundary less the median in-byte interval (8 on an 8-sample stall) |
| R2-4 | B5's and B6's zeroing erased the power-on boot's word unread | nit | **Fixed**: read and record word 0 before zeroing |
| R2-5 | `ACTVOS` 10's support: both reset values (`0x4000`) agree with `VOS`'s encoding | nit | **Added** to the row, still inferred |

**Attacked and held:**
- every write as the correct read-modify-write of its predicted read (`SW` always kept, `HSEBYP` only with `HSEON` 0, reserved bits 0);
- `PA8` alone on TP4 and CN12-23;
- `probe-rs reset` = `SYSRESETREQ`, not overridden for the H7, restoring every written register;
- every address, mask, word, reset value, page and table, recomputed;
- the gate windows;
- 32/15;
- N = 15;
- the frames 16/16/16/16/56;
- the sigrok format;
- S1's equality with `main`.

## §8 Reduction pass

Reduction pass (diff): −9 net · 0 duplicates removed · 2 dead paths removed · 0 protected kept · 2 findings below.
- **Removed from `derive.py`:**
  - a no-op `if k == "forward": pass` inside a copy loop;
  - a one-element `for` over `("source",)`.

  It still reproduces the specification byte for byte.
- **Proposed, not taken (a prose cut is the owner's):** replacing B3's 20-row register-image table with a citation of `3304` B2's. It was cut, then restored before the push.
  - B4 cites B3's `AHB4ENR` row.
  - `3304`'s sheet is frozen for another ELF, and its neighbouring SPI table predicts `D2CCIP1R` `0x4000` where this ELF holds `0x0000`.
  - Sending the operator there would mislead.
- **Kept, not duplicates:**
  - `PREDICTIONS.md` beside §1: the record before the runs, and its score;
  - `edges.py synth`: the evidence D-3 cites;
  - the HSE count's comment naming the three specifications.
- **Finding (outside the fence):** F5's ledger class.

## §9 State

- Branch `wo/aet-4240d-bypass-bench`. PR and CI: on the bus. The builder merges nothing.
- ELF and evidence: `~/.cache/aet-4240d/bench/` (`SHA256SUMS`).
- Kit: `~/.cache/aet-4240d/kit/`.
- Reads: `~/.cache/aet-4240d/reads/`, `ds/`, `probe-rs/`, `sigrok/`.
- Adversarial pass: `adv/`.
- The bench is Scotty's: B-0 and B-1 first, then the rest only if B-1 says proceed.
