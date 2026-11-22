# AET-WO-4240d — predictions, written before generate, build and simulate (2026-11-22)

Base: `main` `e3aea224`, specification 23, solved 15. The specification is
`examples/sentinel-02-bench-hse8.json` (sha256 `a5f09d12…1f5d`), derived from
`examples/sentinel-02-bench.json` (sha256 `a5edf31a…f747`) by `spike/4240d/derive.py`. The derivation:
- `project_name` is `sentinel-02-bench-hse8`;
- `mcu.hse_hz: 0` becomes `mcu.hse.external` at 8,000,000/1 Hz, ±80,000 ppm, with a 20,000 µs start-up;
- `board.drawn` is `false`;
- nothing else moves.

**Already measured before this file** (host debug build of `main`'s CLI). These are not predictions:
- `validate` exits 0, and `solve` exits 0.
- The solved clock is PLL1 at 64,000,000 Hz, VOS2, ±80,000 ppm. The words are `PLLCKSELR` `0x52`, `PLLCFGR` `0x0003_0002`, `PLL1DIVR` `0x0103_069F` and `FLASH_ACR` `0x11`.
- SPI1 is on `pll1_q` at 64 MHz with `MBR` 4, so 2,000,000 Hz, and at most 2,160,000 Hz at +8 %.
- `I2C1.TIMINGR` Fast400k is `0x10D8_1340`, where `sentinel-02-bench` has `0x10C7_123C`.

## Predictions

- **P1. `generate`** exits 0 and writes `board/`, with the board crate, `firmware/` and
  `board/sentinel-02-bench-hse8.repl`, and no `kicad/` (`board.drawn: false`, `4240b` D-9). The platform carries
  `rcc:` / `    hseFrequency: 8000000`.
- **P2. The bring-up** writes the words `4240b` P3 confirmed for `hse-external-8`, in order and each whole:
  - `RCC_CR` `0x0004_0001`, then `0x0005_0001`;
  - `HSERDY` is polled, and the CSS word is `0x000D_0001`;
  - `PLLCKSELR` `0x0000_0052`, `PLLCFGR` `0x0003_0002` and `PLL1DIVR` `0x0103_069F`;
  - `0x010D_0001`, then `PLL1RDY` is polled;
  - `FLASH_ACR` `0x0000_0011`, read back;
  - `CFGR.SW` `011`, then `SWS` is polled.

  No kernel-select write.
- **P3. The pre-switch bounds do not move with ppm.** They are counted at the HSI's fastest, 64,958,976 Hz:
  - `HSERDY` 1,299,180 polls;
  - `PLL1RDY` 9,744;
  - `SWS` 9,744.
- **P4. The post-switch 20 ms wait** is counted at the source's fastest, 64 MHz × 1.08 = 69,120,000 Hz.
  That gives 1,382,400 spins, against `sentinel-02-bench`'s 1,299,180. That one difference is the bench
  firmware's only timing change beyond the bring-up.
- **P5. `build`** at two different paths gives one ELF sha256. It equals neither `main`'s `sentinel-02-bench` ELF nor
  `hse-external-8`'s `762c304d…`.
- **P6. `simulate --for 100ms`** gives `pass`:
  - the boot word is `0x2100_B007`;
  - the twin's clock rule raises no `clock_access` finding;
  - the eight device records and their values equal `sentinel-02-bench`'s on `main`, since the same devices run in the same order.
- **P7. The boot word's virtual time** is later than `sentinel-02-bench`'s on `main` (same image, same `--for`) by
  1.5 to 3.5 ms. P4 adds 83,220 spins at about 30 ns each (`4240b` §2.3: three instructions at 100 MIPS), about
  2.5 ms, less the bring-up's instant waits.
- **P8. The verdict** names the HSE residual and the unobservables (`PLLCFGR` and `HSEBYP` as tags, CSS with no
  detection), as `hse-external-8`'s does.
- **P9. The tests that walk `examples/`** all pass, except
  `hse_eda_tier1::no_committed_specification_without_a_crystal_draws_one`, whose count of HSE specifications goes
  from 2 to 3. No `TIER2_MIN` moves (no tier-2 test is added).
