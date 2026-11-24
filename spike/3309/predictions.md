# AET-WO-3309 §1.1: predictions, committed before any run

Written 2026-11-24, from `main` at `9c3aeee3`, before any campaign or analysis ran on this branch.
They come from reading the source and 3307's records (`spike/3307/out/bmp581-records.txt`, made at
`28fba951`). Those records are hand-read for the prediction; they are not a measurement of today.

## What is measured

1. **Baseline:** every committed scenario group's campaign, run once by `main`'s CLI in the tools
   image, then `attribute` on each archive. The spike cases are run the same way: 3306/3307's two-bme
   and bme-bmp cases, and ext-cfg37.
2. **Where the count rule decides:** from each archive's kind 12 and 13 records, each I2C device's
   whole log (its witnesses' `observed`, concatenated in record order) is re-placed under each
   candidate rule below. Each step whose lines would differ from today's is listed.
3. **What would move:** the same re-placed records go to `main`'s `aethelgard_evolve::attribute` in
   memory (`spike/3309/tool`, no Renode), once for each rule and carrier. Kind 14 is kept as recorded,
   and the boot verdict under each rule is derived beside it.

## The candidate rules for (3)

A device's log is read as transactions in its script's form: a BMP581 line, or a `Write` line and the
`Read` lines after it. A transaction's key is its pointer (the first byte written) and its direction:
a read if it reads, a write otherwise.
- **R1 (literal):** a transaction goes to the device's step with the same key. With two such steps,
  the k-th one goes to the k-th step and any later one to the last. A transaction with no such step
  is unplaced, and so is any line that is no transaction (a warning).
- **R2 (R1, then by direction):** a transaction no step's key matches goes to the device's one step
  of its direction, if there is exactly one; otherwise it is unplaced.

Where an unplaced line lives (kind 13 has no field for it, and the record is outside the fence):
- **A:** in the observed lines of the step under way: the last step whose first placed line came
  before it, or the first step if none did (as an `[ERROR]` line goes today, D-A1).
- **B:** in no witness: it becomes a boot finding of its own, quoted, and leaves kind 13.

## Predictions

- **P1. Baseline:** every committed group gives 3307's measured rule and evidence kinds
  (`spike/3307/out/final/table.md`), and the seeded set gives what `cases.json` pins. Indices may
  shift where records were added since `28fba951`. bmp-0x47 is S1 and ext-cfg37 is I4, as 3307 found.
- **P2. Where the count rule decides:**
  - Only a device with more than one step can be decided by the count. Each BME280 in the sweep has
    one step (identity), so it takes every line it logs under any rule.
  - The multi-step devices are the BMP581s: ext's U5, Sentinel-03's U7 and bme-bmp's U5.
  - In every committed run, each BMP581's whole log is its script, or empty, or a prefix of it under a
    fault. So R1 and R2 give every committed kind 13 byte for byte as today. The judge reads the last
    boot's log only, so a reset does not repeat a script.
  - The count rule decides only in two spike cases: ext-cfg37 (a warning shifts the lines) and
    bmp-0x47 (a warning and a stray read shift them).
- **P3. ext-cfg37 under (3):**
  - **R1:** `write 0x37 <- 0x40 0x81` matches no key, so it is unplaced, as is the warning.
    - Under carrier B, configure's witness is empty while its record claims `ok`. That is a silent
      witness, so S1, with configure's kind 13 as evidence, by silence, not by the wrong write.
    - Under carrier A, both lines go to identity. Its fields read their first line, `read 0x01 ->
      0x50`, which is right. Configure is again silent, so S1 by silence.
    - So R1 attributes S1 but does not meet the acceptance's "the wrong write as its evidence".
  - **R2:** the write goes to configure, the device's only write step. Its `Write` field is
    `0x374081` against `0x364081`, wrong, so S1, with configure's kind 13 as evidence. The warning is
    unplaced. The data step holds its own read only, 6 bytes, refused as before.
  - The evidence index is configure's kind 13 under all four combinations; the generator report
    differs.
- **P4. bmp-0x47 under (3):**
  - `[WARNING] … unmodelled register 0x50` and `read 0x50 -> 0x00` are unplaced under R1 and R2. The
    read has two read steps to choose from (identity, data), so R2 does not place it either.
  - Each U5 step then holds exactly its scripted line.
  - **Carrier B:** U5's configure evidence (32:13) leaves, and U2's silent witness (30:13) is all that
    stays. Before 3307 that was I4, so this predicts S1 → I4. That is a spike case, not committed.
  - **Carrier A:** both unplaced lines precede every placed U5 line, so they go to identity. Its
    `Write` reads `0x50` against `0x01`, wrong, so S1 with U5's identity witness (31:13) in place of
    configure's.
- **P5. (1), counts:** no witness in the sweep shows a right value at a wrong byte count. No verdict
  moves.
- **P6. (2), the BMP581 in the transaction readers:** no committed BMP581 log holds a transaction its
  script does not. bmp-0x47 gains no surplus field while U5's log holds the warning, since
  `transactions` refuses a log with a line that is no transaction.
- **P7. (4), no stitch:** no witness in the sweep takes its `Write` and its `Read` from different
  transactions.
- **P8. The boot verdict:**
  - No committed run's kind 14 moves under any rule or carrier, since every committed placement is
    unchanged (P2).
  - ext-cfg37 still fails. Configure's witness differs from its script (wrong write, or empty), and
    the data witness differs (`0x7F` values).
