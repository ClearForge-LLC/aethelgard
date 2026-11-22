# AET-WO-4240b — predictions, written before any run (2026-11-20)

Base: `main` `8fdc9422`. Corpus: the 29 committed specifications (every `*.json` holding an `mcu` object and a
`schema_version`, outside `spike/`, `fab/`, `docs/`, `target/`); every one reads `hse: null` (version 20's
`hse_hz: 0`). Plus ArduPilot's foreign run on `sentinel-02-own` (`4240v` kit).

## What is measured, per specification (`~/.cache/aet-4240b/kit/sweep.sh`, tools image, capped)

`validate`, `solve` (stdout + exit), `generate` (every emitted file kept, compared byte for byte), `build`
(the ELF's sha256), `simulate --for 10ms` (the verdict, its error findings by kind, the error kind).

## The mask, declared before the runs

A `null` specification may differ in exactly:
- its version line (20 → the version this WO takes; 22 by the kickoff, 23 if `1211a` lands first), and so its
  sha256;
- every `specification_sha256` / "Solved topology of specification sha256" line that follows from it;
- a solved topology's top-level `schema_version`, only if the solved schema moves (asked on the bus, Q2).

Anything else that moves on a `null` board, an emitted byte, the ELF or a verdict, is the §7 stop, **except** what
the architect rules for `4240v` F2 and R4 (asked on the bus, Q2: both change emitted `board.rs` on every board).

## Predictions

- **P1.** Under the mask, all 29 `null` specifications reproduce: every step's exit, the solved topology, every
  emitted file, the ELF and the twin verdict. The foreign run's verdict is unchanged.
- **P2. The 16 MHz crystal fixture** (`ABM8AIG-16.000MHZ-12-2Z-T3`, `4240a`'s) generates this RCC/FLASH write
  sequence after `4240v`'s supply and VOS2 (each word from the solved topology, never recomputed):
  1. `RCC_CR` |= `HSEON` (bit 16), `HSEBYP` (bit 18) 0;
  2. poll `RCC_CR.HSERDY` (bit 17), bounded; timeout → `HSE_NOT_READY`;
  3. `RCC_CR` |= `HSECSSON` (bit 19);
  4. `RCC_PLLCKSELR` `0x0000_0052`, `RCC_PLLCFGR` `0x0003_0004`, `RCC_PLL1DIVR` `0x0103_064F`;
  5. `RCC_CR` |= `PLL1ON` (bit 24); poll `PLL1RDY` (bit 25), bounded; timeout → `PLL1_NOT_LOCKED`;
  6. `FLASH_ACR` `0x0000_0011` (`LATENCY` 1, `WRHIGHFREQ` `01`, RM0433 Table 17 VOS2 ]55;110] MHz), read back;
  7. `RCC_CFGR` `SW` `011`; poll `SWS` = `011`, bounded; timeout → `SW_NOT_TAKEN`.
  No `D2CCIP1R`/`D3CCIPR` write (`SPI123SEL`, `SPI45SEL`, `SPI6SEL` at reset `000`, D6).
- **P3. The 8 MHz external fixture** is P2 with `HSEBYP` set before `HSEON` and `PLLCFGR` `0x0003_0002`,
  `PLL1DIVR` `0x0103_069F` (`4240a` P5).
- **P4. Boot words:** `HSE_NOT_READY` `0x2100_B0F4`, `PLL1_NOT_LOCKED` `0x2100_B0F5`, `SW_NOT_TAKEN`
  `0x2100_B0F6`, the CSS NMI's fault word `0x2100_B0F7` (the next free after `4240v`'s `F1`–`F3`).
- **P5. The bounds in spins** use the HSI's fastest, 64,958,976 Hz (`board.rs` `RESET_HZ` at the HSI budget's
  +14,984 ppm), one poll counted as at least one core cycle:
  - `PLL1RDY`: ⌈150 µs × 64,958,976 / 10⁶⌉ = **9,744** polls (DS12117 Table 39, 150 µs max);
  - `HSERDY`: ⌈`startup_max_us` × 64,958,976 / 10⁶⌉, the figure waiting on the ruling (Q1);
  - `SWS`: the bound needs a figure; RM0433 is to be read for one, else asked.
  No product overflows `u64` for any `u32` microsecond figure.
- **P6. After the switch,** `settle` spins and the SPI `EOT` delay take the crystal's fastest: 64 MHz ×
  (1 + 102 ppm) = 64,006,528 Hz for the fixture (±102 ppm, `4240a` P8), so each count falls by about 1.47 % against
  the HSI's. A `null` board's counts do not move.
- **P7. The twin.** Renode 1.17.0 takes `hseFrequency` from the generated `.repl`'s `rcc` line, and `sys_ck` reads
  back 64,000,000 Hz exactly after `SW` for both fixtures (16 MHz / 5 × 80 / 4; 8 MHz / 5 × 160 / 4). Both
  fixtures simulate to `pass`, the RCC witness in the P2 order.
- **P8. The boot shift** on the 16 MHz fixture: H1 adds only zero-time waits (one read each), and the settle spins
  fall by P6's ratio, so boot-to-`CEN` is earlier than the same board on the HSI's spins. Against silicon the twin
  under-counts up to `tSU` + 150 µs.
- **P9. Kani:** one harness, the size of `reset_clock_is_ok_exactly_at_reset`, with each ready poll nondeterministic
  and small symbolic bounds, finishes in under 600 s.
- **P10. `4240v` F1:** RM0433 Rev 9 gives no encoding for `ACTVOS` (measured before this file: p. 306 says only that
  it reflects the last `VOS` applied; DS12117 Rev 10 never names it). The residue stays stated.
