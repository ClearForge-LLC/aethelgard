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

## §6 Gates and CI

(filled at the close)

## §7 Adversarial pass (targeted)

(filled at the close)

## §8 Reduction pass

(filled at the close)

## §9 State

(filled at the close)
