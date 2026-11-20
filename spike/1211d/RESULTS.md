# AET-WO-1211d — results

Predictions: `PREDICTIONS.md` (commit `661ff706`, before any run). Base: `main` `25203c58`.

## N · The null sweep (§1.1)

`measure/sweep.sh` runs `validate`, `solve` and `emit` on every committed specification (31) with one binary. `measure/same.sh` compares two runs byte for byte: every emitted file and exit code raw, and each command's printed JSON with its own output directory's path replaced (`emit` prints where it wrote).

| point | binary | result |
|---|---|---|
| base | `main` `25203c58` | 31 specifications; `emit` exit 0 on 24, 2 on 7 |
| after vendoring (`76e8cafe`) | branch | **identical: 259 files** |
| after the arm (`89b08ec9`) | branch | **identical: 259 files** |
| D4's table and interpreter (spike only, `d4/`) | experiment | **identical: 259 files** |

N1, N2 and N3 hold: no committed specification reaches either arm, and `validate`'s output, which `own_nets` feeds, is unchanged on all 31.

## A · The arms

**D2 drawn** (the committed D2 with `board.drawn` absent and each bridge input on a GPIO output; see FEEDBACK for why), at library level (`draw/`), judged in the tools image (`kit/img.sh`, capped-run 8 GiB, swap 0 read back):

| judge | result |
|---|---|
| `validate` | ok |
| `check drc --spec --schematic` (parity) | **pass**, findings `[]` |
| `check support` | **pass** |
| `extract` | **pass** |
| `check erc` | **fail**: 2 `pin_to_pin` warnings, AISEN and BISEN (KiCad's `bidirectional`) on GND's flag. Q3, asked 2026-11-20 |
