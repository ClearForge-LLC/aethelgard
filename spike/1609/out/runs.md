| run | plant site | exit | verdict | non-info kinds | U5 data (reads[0]) ns | final PC | ICSR | memory permille |
|---|---|---|---|---|---|---|---|---|
| control | none | 0 | pass | — | 4392420560 | 0x0806187a | 0x00000800 | 795 |
| count-panic | after FIFO_COUNTH read | 1 | fail | spi_transaction_unwitnessed | None | 0x08103c20 | 0x00000800 | 767 |
| count-halt | after FIFO_COUNTH read | 1 | fail | spi_transaction_unwitnessed | None | 0x08036056 | 0x00042800 | 756 |
| count-udf | after FIFO_COUNTH read | 1 | fail | memory_cap_exceeded | None | None | None | 1006 |
| count-reset | after FIFO_COUNTH read | 1 | fail | core_in_exception, foreign_clock_contradicted, pc_outside_firmware, register_mismatch, spi_step_error, spi_transaction_unwitnessed | None | 0xeffffffe | 0x00000803 | 750 |
| panic | after FIFO data read | 0 | pass | — | 4392420560 | 0x08103c20 | 0x00000800 | 754 |
| halt | after FIFO data read | 0 | pass | — | 4392420560 | 0x080360ac | 0x00042800 | 750 |
| udf | after FIFO data read | 1 | fail | memory_cap_exceeded | None | None | None | 1007 |
| reset | after FIFO data read | 1 | fail | core_in_exception, foreign_clock_contradicted, pc_outside_firmware, register_mismatch, spi_step_error | 4392420560 | 0xeffffffe | 0x00000803 | 755 |

Hooked re-runs of the CLI's own captured script (sh/capture.sh, measure.py), no memory bound:

- **control**: hooks none; CFSR 0x00000000, HFSR 0x00000000, instructions 89,113,558, PC 0x806187a, ICSR 0x00000800; U5 log lines after 4.393 s: 2365; log bytes 8,522,550
- **panic**: hooks ['[00:00:04.392440850] [WARNING] cpu: AETHELGARD-HOOK panic pc=80c033c']; CFSR 0x00000000, HFSR 0x00000000, instructions 78,253,743, PC 0x8103c20, ICSR 0x00000800; U5 log lines after 4.393 s: 0; log bytes 1,126,552
- **halt**: hooks none; CFSR 0x00000000, HFSR 0x00000000, instructions 72,576,093, PC 0x80360ac, ICSR 0x00042800; U5 log lines after 4.393 s: 0; log bytes 1,034,966
- **udf**: lines=20258612 bytes=2350079600; hook `[00:00:04.392440830] HardFault_Handler pc=8000340`; end: CFSR 0x00010000 (UNDEFINSTR), HFSR 0x40000000 (FORCED), PC 0x8001434 (CrashCatcher_getc), ICSR 0x00000803 (VECTACTIVE 3); 20,250,235 log lines `usart1: [cpu: 0x8001432] ReadUInt32 from 0x1C (InterruptAndStatus)`.

