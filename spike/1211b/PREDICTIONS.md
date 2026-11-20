# AET-WO-1211b — predictions, written before any build line

Base: `main` `25203c58` (1211a merged). Baseline taken with `kit/sweep.sh` in the tools image
(`kit/img.sh`, capped-run 8G): every committed specification validated, solved, generated and built;
`base-rows-25203c58.tsv` holds each exit and ELF sha256. 27 generate and build; `d1-drv8876.json` is
refused `PinDeviceNotGenerated` at generate; `d2-drv8833.json`, `invalid-zero-domain.json` and three
solver fixtures are refused earlier, as on main.

| # | prediction |
|---|---|
| B1 | Every one of the 27 generated trees is byte-identical to main's, and every ELF sha256 is equal: no topology on main has a pin device, an idle `high`, a non-chip-select GPIO or an ADC input, and every new line is emitted only when its feature is present |
| B2 | The verdict ledger of every existing specification is unchanged row for row and reason for reason: the new classes are new rows, never a reworded old one |
| B3 | D1 (`DRV8876`, PH/EN, nSLEEP, nFAULT, IPROPI) generates, builds and proves once the refusals lift; D2 stays refused `PartLimitUndeclared` at validate |
| B4 | D1's words: TIM1 PSC 0, ARR 3199, CCR1 = CCR2 = 0 (idle low); TIM2 PSC 63, ARR 19999, CCR1 1500 (idle pulse); nSLEEP (PA1) written low by BSRR reset (bit 17) before MODER = 01; nFAULT (PA11) MODER 00, PUPDR 00 (pull none, the 10 kΩ is external) |
| B5 | Each new or widened Kani harness proves in under 600 s |
| B6 | Each of the work order's four mutants (idle after CEN, a dropped initial level, an enable before its CEN, an unclassified ledger field) fails a test or a proof |
