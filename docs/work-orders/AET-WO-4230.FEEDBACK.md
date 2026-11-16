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
