# AET-WO-1322 — predictions, committed before any run

Written 2026-11-23 07:40Z on `main` `7fbe5d11`, before the baseline is taken and before any line of the build.

## The baseline (§1.1)

- **B1.** `spike/1211d/measure/sweep.sh` selects **34** committed specifications at `7fbe5d11` (counted by its own selector before this file was written), as at `1321`.
- **B2.** `spike/1319/harness/run.sh` runs unchanged on a `git archive` of `7fbe5d11`, with the shim's anchor in place. `d4diff` writes **277** cases and `d1319` **84**, as at `1321`. `4300i` (specification 28) moved goldens, so the file counts may differ from `1321`'s 280, 926 and 331; the case counts do not.
- **B3.** `spike/1321/bulk.sh` (base against base) reports both variants identical, `emit` exit 0.
- **B4.** At the end of the build, branch against base: the sweep, `d4diff`, `d1319` and `bulk.sh` are all **identical**. Nothing in (1)–(7) touches a drawing; the only emit-path edits are guards deleted or relabelled, and an error mapping moved into a function.

## (2) `prepare_read` on a long name (§1.1)

- **M1.** A board of one footprint whose reference is 10 kB, carrying N pads, read by `board::prepare_read`: on `main` the peak resident memory grows by about **N × 10 kB** (every pad pushed onto the scan stack with its own copy of `footprint <10 kB>`): about 1 GB at N = 100 000. `1311`'s adversary measured 3.0 GB at 300 000 children.
- **M2.** After the fix (each named item's own label held once, the carrying chain shared by `Rc`, the text rendered only into a refusal), the same board's peak is the input and its tree: **under 100 MB** at N = 100 000, and it does not grow with the name's length.
- **M3.** Every existing `board::tests` case passes with its `id` unchanged (`pad 2 of footprint R7`, `zone with no net of footprint X`, `the board`).

## (1) Unreachable guards

- **U1.** Every public entry into the emitter that reaches the guards (`emit`, `emit_moved`, the placed design, the read-only drawing) runs `Specification::validate` first, so `UnsupportedAddress` (the strap's address check) and a crossed bus's `PowerNotDrawable` are unreachable with a solved topology that `solve` made. A hand-written topology passed beside a valid specification may still reach some of them (the adversarial target in §5): those are kept and labelled, each with a direct test. Low confidence on the split; an inventory decides it.

## (3) The reader's echo, and `MAX_ITEMS`

- **E1.** On `main`, a 14 MB layer name in a board read by `check drc --board` gives a refusal of about **49 MB** on stdout (`1311` J). After: the echoed text is at most 200 bytes, cut on a character boundary and escaped, and the refusal is **under 1 kB** whatever the name's length.
- **E2.** A name of at most 200 bytes with no character that `{:?}` escapes prints as today, so no committed test's expected message moves.
- **E3.** `MAX_ITEMS` counts courtyard edges beside pads, tracks and vias. The largest committed board (`d3-24-i2c-12-spi`, 3 351 copper items) stays far under 20 000, so no committed board is refused. `1311` F's board (9 900 vias, a 90 000-point courtyard, 19 803 items counted today) is refused by the count.

## (4) The IPROPI text

- **I1.** The `CurrentOutput` arm's message names the tie to `GND` that `validate` admits since `1217` F2. The guard stays: a copied row with a second `current_output` that no trip names reaches it (`1217` R1).

## (5) One reference order

- **N1.** Already built on `main`: `4300i` (`ab9d4663`) replaced eda's hand-copied chain with `aethelgard_ir::names::references(spec)`, which is `pub`. Nothing is left to change; the IR is not touched.

## (6) `1319`'s M2 and a doubled entry

- **D1.** A replacement M2 drops BMP581's second 100 nF decoupler **and** lowers its `DECOUPLERS` count to 1, so it compiles; the harness kills it by bytes (the sweep's BMP581 boards and `d1319` move).
- **D2.** `check` refuses a `DECOUPLERS` entry stated twice for one part, whichever count the second states. A mutant deleting the line is killed by the new test only.

## (7) `PadEscapeExhausted`'s shape

- **P1.** A test pins its `kind` (`PadEscapeExhausted`), `id` (the reference) and message, and the router-to-emitter mapping that builds it. A mutant mapping an exhausted run to `Unroutable` is killed by it.
