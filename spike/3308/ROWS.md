# AET-WO-3308 §1.1: every predicted row of `BENCH.md`, against `f3e871bb…52c4` and HEAD's generated source

Keyed by `BENCH.md` line at `b9d5f4b6`. The 186 candidate lines came from one grep (`rows-candidates.txt`): a `0x` word, a line citation, a value with a unit, a byte string, or a source label. Lines 99 and 364–371 hold values without those patterns and are added by hand. Every candidate line appears below, alone or in a range.

**What each row was checked against**, by its own label (`INV-07`):
- **twin:** `runs/sim.json`, `simulate --for 100ms` of the package ELF (sha checked `f3e871bb4a16678a77ac1b81bf2f9b53e463b3c9aee435dd45010a1fde7452c4`), on `boards/sentinel-01-bench/sentinel-01-bench.repl`, judged by `examples/sentinel-01.json`, in the pinned image `1e0ab105…`. Verdict `pass`.
- **generator:** HEAD's `boards/sentinel-01-bench/firmware/src/main.rs` and `src/board.rs`. P2 shows HEAD's crate re-derives this ELF.
- **rule:** the verify ledger of `runs/verify.json` (`verify --against-fab`, the five scenarios), `silicon.rs`'s figures there, or the datasheet line cited.
- **repeat:** `3302`'s record. The ELF does not enter it.

**Class:**
- **H**: holds; the value was checked against its source for this ELF.
- **C**: holds, and only the line citation moves (old → new).
- **S**: the text is scoped to another ELF (`008a2a59` or `e4e24372`). No predicted value fails; the words must name `f3e871bb`. **A decision for the architect.**
- **I**: independent of the ELF (netlist, copper, `extract`, datasheet, instrument, procedure). Listed, not re-run.
- **D**: the value differs. That is the §7 stop. **None found.**

## Line citations, statement by statement

Each statement was found in `git show ee835bde:…` and then at HEAD. Every cited statement survives unchanged; only the lines move.

| `BENCH.md` line | Cites | Ordered (`ee835bde`) | HEAD | Statement |
|---|---|---|---|---|
| 76, 121 | `main.rs` | L172–177 | L200–205 | the console's label bytes and tail |
| 76 | `main.rs` | L56 | L64 | `PATTERN_TAIL` |
| 76 | `main.rs` | L204–211 | L245–252 | the echo loop |
| 78 | `main.rs` | L60 | L68 | `ECHO_POLLS = 20000` |
| 79 | `main.rs` | L30 | L38 | `AETHELGARD_BOOT` (**now `[u32; 8]`**, see S rows) |
| 83, 260 | `main.rs` | L203 | L244 | the boot word written |
| 84 | `main.rs` | L148–150 | L176–178 | word 1 `FLASH_OPTSR_CUR` |
| 85 | `main.rs` | L179–190 | L207–218 | record 0, `0x76`/`0xD0` |
| 87 | `main.rs` | L191–201 | L219–229 | record 1, SPI1 `0x9F` |
| 92 | `main.rs` | L230–242 | L230–242 | record 2, `0x48`/`0x0F` (already HEAD's) |
| 116 | `main.rs` | L213 | L254 | the final loop |
| 122, 372 | `main.rs` | L179 | L207 | `i2c1.transfer(0x76, [0xD0], 1)` |
| 122 | `board.rs` | L587 | L611 | `CR2` with `START`, `AUTOEND` 0 |
| 122 | `board.rs` | L204 | L228 | `I2C_WAIT_SPINS = 20_000` |
| 122 | `board.rs` | L589–590 | L613–614 | the `TXIS` wait |
| 124 | `board.rs` | L590 | L614 | `I2cError::Txis` |
| 124 | `board.rs` | L595–611 | L619–635 | no `TC`, read or `STOPF` wait follows |
| 124 | `board.rs` | L613 | L637 | `ICR` ← `STOPCF \| NACKCF` |
| 125 | `main.rs` | L184 | L212 | `Err(I2cError::Txis) => (STATUS_TXIS, 0)` |
| 125 | `main.rs` | L190 | L218 | `record(0, …)` |
| 125 | `main.rs` | L191–211 | L219–252 | what follows record 0 (**now holds record 2**, see S rows) |
| 153 | `main.rs` | L163 | L191 | PA9 to AF7 |
| 153 | `main.rs` | L171 | L199 | USART1 configured |

## The rows

| Line(s) | Row | Predicted | Label | Checked against | Class |
|---|---|---|---|---|---|
| 5 | header | ELF `008a2a59…` | — | MANIFEST `f3e871bb…` | **S** |
| 8–12 | label definitions | — | — | no value | I |
| 14, 17, 19 | EXT pinout, 20 pF allowance, 71 pF headroom | netlist, `extract` | rule | board copper, not ELF | I |
| 36, 37 | header pins ↔ TPs | netlist | rule | — | I |
| 44–49 | Before power 3: R1–R4, R8, TP4–TP1 | 2.13–2.27 kΩ, 9.7–10.3 kΩ, >100 Ω | rule | parts, not ELF | I |
| 52–56 | NRST continuity, `C_p`, supply 3.30 V / 3.6 V / 4.0 V abs | — | rule | — | I |
| 76 | console: 12 bytes `43 4F 4E 53 4F 4C 45 00 FE 7F 80 FF`, before any I2C/SPI | twin | twin | `serial[0].twin.pattern` = the 12 bytes; L200–205 precede L207 | H, C |
| 78 | echo ≤ 20,000 polls per byte | generator | generator | `ECHO_POLLS = 20000` (L68) | C |
| 79 | records: "six words" at `0x20000000` | — | — | `[u32; 8]` (L38) | **S** |
| 83 | word 0 `0x2100B007` | twin | twin | `boot_word.observed` `0x2100b007` | H, C |
| 84 | word 1: bits 3:2 `11` after B3, `00` before | B3 (silicon-only; repeat) | — | the twin presents level 3 (`brown_out.observed & 0xc` = `0xc`); the bench judges it | C |
| 85, 86 | record 0, U2 empty: `0x22020001` / `0` | generator + RM0433; repeat | generator | `STATUS_TXIS = 0x22020001` (L43), `Err(Txis) => (STATUS_TXIS, 0)` (L212). The twin (U2 at `0x60`) neither confirms nor refutes | H, C |
| 87, 88 | record 1 `0x22020000` / `0x00EF4017` | twin | twin | `spi[0]` ok `0xef4017` | H, C |
| 90 | record 0 with U2 fitted `0x22020000` / `0x60` | twin | twin | `i2c[0]` ok `0x60` | H |
| 92 | "with `f3e871bb…52c4`, **not the ordered one**, eight words" | — | — | MANIFEST's ELF; L230–242 ✓ | **S** (the aside becomes the sheet) |
| 96 | word 6: U3 empty TXIS `0x22020001`; fitted OK | generator + RM0433; SNOSD82D | generator | L231 `transfer(0x48, [0x0f], 2 bytes)`, L236 `Txis => (STATUS_TXIS, 0)` | H |
| 97 | word 7: empty `0`; fitted `0x117` | generator; SNOSD82D | rule (datasheet) | twin's `not_witnessable.reads[0].table_value` `0x117`, mask `0xffff` | H |
| 99 | the twin's record 2 is a limit; B9 part 1 sees a second transaction at `0x48` | twin limit; generator | — | `identity_not_witnessable`, `timeout_txis`, cost 3,600,710 ns | H |
| 105, 111, 112 | read and clear **6** words | — | — | eight words with this ELF | **S** |
| 108 | residue: a dead boot still shows `0x2100B007` | — | — | NOLOAD unchanged | I |
| 116 | parked in the final loop (L213) by 1 s | generator | — | L254; `boot_word_ns` 3.63 ms in the twin | C |
| 121–124 | the U2-empty code path | generator + RM0433 | generator | `board.rs` L611, L228, L613–614, L619–635, L637 | C |
| 125 | "the SPI read, the boot word and the echo follow unchanged (L191–211)" | generator | — | the U3 read (L230–242) now follows too | **S** |
| 126 | "nothing else in the firmware addresses `0x76` **or I2C1**" | generator | — | false for this ELF: L231 addresses I2C1 at `0x48` | **S** |
| 128, 130 | `3302`'s repeat; twin's U2 limit | repeat | repeat | — | I |
| 140 | B9 with U2 empty: `0xEC`, NACK, STOP | generator; repeat | generator | L207 `0x76` | H (and see 364–371) |
| 142 | what is lost (`0x60`) | — | — | — | I |
| 147–149 | B1 instrument; NRST ≤ 0.3 V (Table 151) | rule | rule | datasheet | I |
| 152 | the flashed ELF's sha checked against MANIFEST | — | — | what test (3) binds | H |
| 153 | 12 bytes after reset; PA9 AF7 (L163) before USART1 (L171) | twin | twin | pattern; L191, L199 | H, C |
| 165–169 | the B2 class definitions | — | — | — | I |
| 175–199 | B2: the 25 rows the twin stores | twin | twin | `boot.registers`: 25 rows, each `observed & mask` = `expected` = `BENCH.md`'s word | H (25/25) |
| 200–222 | B2: the 19 ledger rows | generator / rule / not predicted | per the ledger | `b2.py` (3403's, PWR/FLASH addresses added) on `runs/verify.json`'s ledger: every row's register, address, mask, word and class regenerate identically (`b2-regenerated.md`; the only additions are the four PWR/FLASH rows below, which `BENCH.md` carries at L236–243) | H (19/19) |
| 224 | "Since `4240v`, the package ELF (`e4e24372…1d496f11`)… **Flash that ELF**" | — | — | MANIFEST's is `f3e871bb` | **S** |
| 227–229 | PWR register addresses | rule | rule | RM0433 | I |
| 236, 237 | `PWR_CR3` at reset: rev V `0x46`, rev Y `0x06`; `D3CR` `0x6000` | rule | rule | RM0433 reset values; ELF-independent | I |
| 238 | `CSR1` before: not predicted | — | — | — | I |
| 239 | `PWR_CR3` after: rev V `0x42`, rev Y `0x02` | generator | generator | `board.rs` L1089: `(v & !7) \| 2` | H |
| 240 | `CSR1 & 0xE000` = `0xA000` | rule | rule | the `VOSRDY`/`ACTVOSRDY` waits succeed in the twin (boot word `0x2100B007`, not `…F2`/`…F3`) | H |
| 241 | `D3CR` = `0xA000` | rule | rule | `board.rs` L1096 writes `0x8000` (VOS `10`); the ledger's reset row `VOSRDY` 1 (silicon-only). `b2.py` composes `0x8000` because it takes the written `VOSRDY` 0 to a read-only bit: a script artifact, not the read | H |
| 243 | `bench_rows`: `CSR1.ACTVOSRDY`, `FLASH.ACR` `LATENCY` 7, `WRHIGHFREQ` `0x30` | rule + generator | silicon-only | the ledger: `ACTVOSRDY` reset 1, `LATENCY` reset 7, `WRHIGHFREQ` reset 3, all acknowledged; the firmware writes no `FLASH.ACR`. `D3CR.VOSRDY`'s reset row is left off `bench_rows` because the firmware writes the field (`on_the_bench`), so "three" is exact | H |
| 249–251 | B3 `BOR_LEV` | silicon-only; repeat | — | — | I |
| 259 | B4 record 1 `0x22020000` / `0x00EF4017` | twin | twin | `spi[0]` | H |
| 260 | word 0 `0x2100B007`, "written after **record 1**" (L203) | twin | twin | value H; written after record 2 now (L244) | H, **S** |
| 261 | `MODF` = 0 | not predicted | — | — | I |
| 262 | `CFG2.MASTER` bit 22 = 1 | rule | rule | `0x84400000` bit 22 set (`board.rs` L792) | H |
| 263 | SCK 32 clocks at 15.59–16.24 MHz | rule | rule | `silicon.spi[0]`: `mbr` 1, `achieved_hz` 16,000,000, `achieved_max_hz` 16,239,744; `slow_kernel_hz` 62,342,784 / 4 = 15.59 MHz | H |
| 270–274 | B5 BOR sweep, Table 115 | rule | rule | datasheet | I |
| 287–303 | B6 rise times, `extract` bounds | rule | rule | copper; `extract` | I |
| 289 | `OTYPER` makes PB6, PB7, PB10, PB11 open-drain (B2) | generator | generator | `GPIOB.OTYPER` `0x0cc0` (bits 6, 7, 10, 11) regenerates | H |
| 305 | the campaign's 186 ns / 847 ns | rule | — | declared 100 pF | I |
| 311–314 | B7 grounds and method | — | — | — | I |
| 315, 321 | I2C2 `TIMINGR` `0x50D32A40`: `PRESC` 5, `SCLH` 42, `SCLL` 64; 258 / 390 cycles | generator | generator | `board.rs` L194/L223; `silicon.i2c` Standard100k `timingr` 1356016192 = `0x50D32A40`; decoded | H |
| 317–318, 320 | the probe's I2C2 writes; RM0433 p. 1975 | — | — | — | I |
| 323–330 | B7 steps; `T_k = t_low(00)/5004`, 78.19 µs | rule | rule | `BRR` 556 (`serial[0].brr`), 9 × 556 = 5004, at 64 MHz = 78.1875 µs | H |
| 331–333 | B7 band 1.22–1.80 V | rule | rule | DS12117 | I |
| 334 | analog filter on (`ANFOFF` 0) | generator | generator | `I2C2.CR1` `0x00000001` under `0x00ffdfff` regenerates: bit 12 is 0 | H |
| 340–349 | B8 thresholds, dividers | rule | rule | — | I |
| 346 | after SW1, the 12 bytes | twin | twin | pattern | H |
| 364–371 | B9 part 1: **exactly** `Address write: 76`, `NACK`, `Stop`, **once** | generator; repeat | generator | with this ELF a second I2C1 transaction follows (L99): `Address write: 48`, `NACK`, `Stop` with U3 empty | **S** |
| 372 | `0xEC` (L179) | generator | generator | L207 | C |
| 373 | SCL period ≥ 2.5247 µs | rule | rule | `0x10C7123C`: `PRESC` 1, `SCLH` 18, `SCLL` 60 → 2 × 80 = 160 cycles, + 4, at 64.958976 MHz = 2.5247 µs; `silicon.i2c` I2C1 Fast400k `timingr` 281481788 = `0x10C7123C`, `scl_max_hz` 396,092 | H |
| 374 | record 0 `0x22020001` / `0` | generator | generator | as 85 | H |
| 377–382 | B9 part 2's probe writes | — | — | — | I |
| 385–386 | `ISR` flags | rule (RM0433) | — | — | I |
| 387 | "`device_unmodelled` on the **ordered** ELF; `identity_not_witnessable` on `4241`'s" | twin | twin | this ELF: `identity_not_witnessable` | **S** |
| 388–393, 399, 400 | text, cannot-measure | — | — | — | I |
| 401 | "the ordered ELF… runs at VOS3… `4240v`'s ELF re-runs B2 and B4" | — | — | this ELF runs at VOS2 | **S** |
