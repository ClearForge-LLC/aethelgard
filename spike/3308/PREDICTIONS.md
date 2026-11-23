# AET-WO-3308 §1.1: predictions, written before the runs they predict

Written 2026-11-23 ~11:55Z at `b9d5f4b6`, before any container run. Nothing below has been run yet except P1.

## P1. The ten ordered files' hashes (the WO's prediction: equal)

**Run before this file was committed:** the hashes were taken at 11:42Z, before any prediction file existed. The prediction is the WO's own ("prediction: equal", §1.1), committed by the architect in `3b59fa8d`. The results are in `ordered-hashes.txt`.

## P2. The package ELF re-derives from HEAD

`verify --against-fab fab/sentinel-01-bench/package`, with the five scenarios in MANIFEST's order, run in the pinned image (`container/IMAGE`, `1e0ab105…`) at `b9d5f4b6`:
- **Prediction:** it passes, so the ELF built from HEAD's generated crate is `f3e871bb4a16678a77ac1b81bf2f9b53e463b3c9aee435dd45010a1fde7452c4`, MANIFEST's.
- **Why:** `8fdc9422..HEAD` changes `main.rs` and `board.rs` only by the `scale2` token passed to `enable_*` (a zero-sized proof value) and the specification hash in comments. MANIFEST's `specification_sha256` (`e635e940…`) is HEAD's.
- **If it fails:** generator rows and citations are judged against `8fdc9422`'s source, and that becomes a row in the stop table.

## P3. Test (3) against today's `BENCH.md`

The rule drafted for (3): `BENCH.md` states exactly one full 64-hex ELF hash, equal to MANIFEST's `sentinel-01-bench.elf`.
- **Prediction:** it fails, **but not by naming `008a2a59`** (the WO's prediction). Today's `BENCH.md` holds **no** 64-hex string, so there is nothing to mismatch.
  - The draft's diagnostic lists the abbreviated hashes it finds: `008a2a59…` (L5), `f3e871bb…52c4` (L92), `e4e24372…1d496f11` (L224).
  - It also finds two non-ELF digests, `78212e91…b789` (L97, SNOSD82D) and `452d906c…` (L259, the `parts.json` row).
- The draft lives in `test3-draft.patch` with its output, never in the stop commit: a red test breaks the gates.

## P4. The twin's run of `f3e871bb…` (`simulate --for 100ms`, the package ELF on `boards/sentinel-01-bench/sentinel-01-bench.repl`, judged by `examples/sentinel-01.json`)

- **Boot word** `0x2100B007`; **word 1** is the twin's `FLASH_OPTSR_CUR` (level 3 presented: `acknowledge_unwitnessed`).
- **Record 0:** OK / `0x60`. The twin models a BME280 at `0x76`; this neither confirms nor refutes the U2-empty row, which rests on generator + RM0433 + `3302`'s repeat.
- **Record 1:** OK / `0x00EF4017`.
- **Record 2** (U3 `0x48`, `0x0F`, two bytes): TXIS / 0, `identity_not_witnessable`. This is a twin limit, not a prediction of the board.
- **Console:** `serial[0].twin.pattern` = the 12 bytes `43 4F 4E 53 4F 4C 45 00 FE 7F 80 FF`.
- **B2's register table:** every row the twin stores reads, under its mask, the word `BENCH.md` gives (44 rows in all, twin and ledger). The twin's stored set may now hold PWR or FLASH.ACR rows that `BENCH.md`'s table lacks; that would be a completeness finding, not a value stop.
- **B2's supply rows:** `PWR_CR3` after the boot rev V `0x42` / rev Y `0x02`; `PWR_D3CR` `0xA000`; `PWR_CSR1` under `0xE000` = `0xA000`. These rows were predicted for `e4e24372` at `d70a99e5`, and `4241` added only the U3 read after record 1, so they hold. The twin's PWR is `PwrRule` (`4240v`), so its values are the rule's.

## P5. The rows by label, against `f3e871bb…` and HEAD's generated source

- **Generator** (B2 table generator rows, `TIMINGR` words `0x10C7123C` and `0x50D32A40`, B7's clock words, B9's address `0x76`): every value holds.
  - Every `main.rs` and `board.rs` citation moves line but not statement: `main.rs` +28 before the U3 read and +41 after it; `board.rs` +24.
- **Rule** (`BRR` `0x22C`, `D2CCIP1R` `0x4000`, `SPI1.CR1`/`CFG1`/`CFG2`, SCK 15.59–16.24 MHz, the SCL floor 2.5247 µs, the B7 band, B5's Table 115, B8's thresholds, R values): every value holds.
  - The rule rows that read the solved topology are checked against the verify ledger of P2's run.
  - Datasheet rows and Before-power rows do not depend on the ELF, and are marked as such.
- **Twin:** as P4.
- **Repeat** (`3302`'s TXIS with no device, `BOR_LEV` `00` before): these are unchanged by definition, and the ELF does not enter them.
- **Text scoped to another ELF** (not a value, a decision for the architect):
  - L5: `008a2a59`.
  - L79 and L105–112: six words, and the commands that read and clear six.
  - L92–99: the eight-word aside.
  - L126: "nothing else addresses I2C1".
  - L224: "Flash that ELF", `e4e24372`.
  - L364–371: B9's "exactly these lines, once", against L99's second transaction.
  - L387: `device_unmodelled` on the ordered ELF.
  - L401: VOS3.
- **Stop prediction:** no predicted RAM word or register value fails to hold.
