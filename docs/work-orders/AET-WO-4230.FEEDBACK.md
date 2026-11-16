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

## 2. §1.0, measured (2026-11-16 03:41 UTC, pinned image `aethelgard-tools@sha256:c69ff9bc…a195a`, Renode `1.17.0+20260907gitf1dd1b4af`)

The reference's emitted platform (`main` `25640c5`, `sentinel-01.repl`), driven from Renode's monitor with `LogPeripheralAccess` on `flashController`, `rcc` and `spi1`, run under `~/bin/capped-run` (scope `memory.max` 8 GiB, `memory.swap.max` 0, `pids.max` 8192, read back). The script and its whole output are in `~/.cache/aet-4230/m0/` (`m0.sh`, `out.txt`, `m0.log`). Renode's model sources were read at `renode-infrastructure` `066a7f13c052215632d469c995c89aea37c573b1`, the submodule of `renode` tag `v1.17.0` (`~/.cache/aet-4230/renode-src/SOURCE.txt` lists sha256 per file).

| Register | Reset read | Write, read back | Label |
|---|---|---|---|
| `FLASH_OPTSR_CUR` (0x5200201C) | `0x0406AAF0`: `BOR_LEV` (bits 3:2) = `00` | `0xFFFFFFFF` ignored, still `0x0406AAF0` (the field is `FieldMode.Read`) | measured; read at source (`STM32H7_FlashController.cs` L118–119) |
| `FLASH_OPTSR_PRG` (0x52002020), locked | `0x0406AAF0` | `0x0406AAFC` ignored, with the warning "Attempted to write to a program register (OptionStatusProgram) while OPTLOCK bit is set. Ignoring" | measured |
| `FLASH_OPTKEYR` `0x08192A3B`, `0x4C5D6E7F`, then `OPTSR_PRG` `0x0406AAFC` | `OPTCR` reads `0x0` (unlocked) | `OPTSR_PRG` reads `0x0406AAFC`; `OPTSR_CUR` unchanged until `OPTCR.OPTSTART` | measured |
| `OPTCR` = `0x2` (`OPTSTART`) | | `OPTSR_CUR` reads `0x0406AAFC`: `BOR_LEV` = `11` | measured |
| after `machine Reset` | | `OPTSR_CUR` and `OPTSR_PRG` both `0x0406AAFC`: kept (`OPTSR_PRG` is not soft-resettable, and `Reset` copies it into `OPTSR_CUR`) | measured; read at source (L38–47, L121–122, L147–150) |
| `RCC_D1CCIPR` (0x5802444C) | `0x0` | `0xFFFFFFFF`: reads `0x0`; "Unhandled write … Tags: … CKPERSEL (0x3)" | measured; every field a tag (L394–402) |
| `RCC_D2CCIP1R` (0x58024450) | `0x0` | `0x00004000` (`SPI123SEL` = `100`): reads `0x0`; "Unhandled bits: [14] … Tags: SPI123SEL (0x4)"; all-ones likewise | measured; every field a tag (L403–418) |
| `RCC_D2CCIP2R` (0x58024454) | `0x0` | not written | measured; `I2C123SEL`, `USART16SEL`, `USART234578SEL` are tags (L419–431) |
| `SPI1_CFG1` (0x40013008) | `0x0` | `0x70000007` (`MBR` 7, `DSIZE` 7): reads `0x7`; "Tags: MBR (0x7)". All-ones reads `0xC01F` (`DSIZE`, `RXDMAEN`, `TXDMAEN` only) | measured |
| `SPI1_CFG2` (0x4001300C) | `0x00400000` (`MASTER` always reads 1) | `0x04400000` (`SSM`): reads `0x00400000`; "Tags: SSM (0x1)". All-ones reads `0x00C00000` (`MASTER`, `LSBFRST` only) | measured |
| `SPI1_CR1` (0x40013000) | `0x0` | `0x00001000` (`SSI`): reads `0x0`; "Tags: SSI (0x1)" | measured |

**What it decides:**
- **The brown-out presentation (§1.6) is possible:** a key unlock, an `OPTSR_PRG` write and `OPTSTART` give `OPTSR_CUR` any `BOR_LEV`, and the value survives `machine Reset`. §7's second stop does not fire. Whether it survives the droop scenario's own reset is measured with the campaign (§4).
- **P1 held.** P2 was wrong: `CKPERSEL`, `SPI123SEL`, `SPI45SEL` and `I2C123SEL` are tags, so a write is logged and dropped, and every read gives 0. So the run-time `ResetClock` check reads `D1CCIPR` and `D2CCIP1R` before the selection is written and never after; a read-back would park every boot in the twin, and would tell the firmware it is in the twin (4300b Q9's class).
- **P3 held.** `MBR`, `SSM` and `SSI` are not stored (measured). They are not honoured either: the model's transfer path never reads them, and `MODF` is a tag, so no mode fault can be raised (read at source, `STM32H7_SPI.cs` L91–203; no transfer was run to show it).
- **Ledger consequences:** `SPI123SEL`, `CKPERSEL`, `I2C123SEL`, `MBR`, `SSM` and `SSI` are `ignored-by-model`. Each is judged by an independent rule, and the access log shows the value written. `BOR_LEV` is `silicon-only` as programmed: the twin presents the declared value.
- **4300b's serial judge allows RCC writes only to the nine `*ENR` registers** (4300b FEEDBACK §8, passes 3 and 4), so the generated `D2CCIP1R` write fails the reference's CONSOLE judgement unless that allowance is extended. That judge is outside this work order's named CLI fence, but §1.1 forces the change. It allows exactly the solved whole word, and nothing else (§5).

**m1, the presentation in the platform (measured 03:57 UTC, `~/.cache/aet-4230/m1/`).** An `init add:` block on `flashController`, appended to the emitted `.repl` (`OPTKEYR` 0x08192A3B, 0x4C5D6E7F; `OPTSR_PRG` 0x0406AAFC; `OPTCR` 0x2; then `OPTKEYR` 0x0), makes `OPTSR_CUR` read `0x0406AAFC` at the first read and after `machine Reset`, with `OPTCR` reading `0x1` (locked, as silicon resets) both times. Ending with `OPTCR` 0x1 instead leaves it reading `0x0` (unlocked) until the first reset, a difference a firmware could see, so the wrong-key write is the last line.
