# AET-WO-1305 — FEEDBACK

**Branch:** `wo/aet-1305-extract-bound` · **Base:** `b227c12` · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Cadence:** build-only, with the §1.3 measurement first.

Entry schema: `finding · where · type · recommendation · decision-needed`.

## §1.3 Measurement on the reference, before any acceptance number

Recorded from the first run of `aethelgard extract` on the board `aethelgard generate --spec examples/sentinel-01-minimal.json` writes (board sha256 `7b9b0e3d…2070`, specification sha256 `acd60081…49d1`), before any test held a number. The constants it used: per-length bound 280 fF/mm, 124 fF for each track's two round ends, 1,519 fF per via, `Ci` 10,000 fF per pin.

| Line | Trace | Vias | Pads | Pins | `C_bound` | Declared | Verdict |
|---|---|---|---|---|---|---|---|
| `I2C_SDA` | 9,146 fF (24.25 mm, 19 tracks) | 6,076 fF (4 vias) | 1,578 fF (4 pads) | 30,000 fF (U1.137, U2.3, U3.6) | **46,800 fF** | 100,000 fF | pass |
| `I2C_SCL` | 16,024 fF (43.50 mm, 31 tracks) | 10,633 fF (7 vias) | 1,578 fF (4 pads) | 30,000 fF (U1.136, U2.4, U3.1) | **58,235 fF** | 100,000 fF | pass |

- The lengths and via counts match an independent count of the board's text (a script over the `.kicad_pcb` in the session scratchpad): 24.25 mm and 4 vias on SDA, 43.5 mm and 7 vias on SCL.
- The pads are R1/R2's line pad and the three pins' lands. The lands are counted as copper as well as at `Ci` (F2).
- **The reference passes with 42 % (SCL) and 53 % (SDA) of the declared bound to spare.** No §7 stop.

**Prediction, before the test:** a specification declaring 20 pF (20,000 fF) on both lines fails on its pins alone (3 × 10,000 fF = 30,000 fF > 20,000 fF). `extract` exits `1` with two `capacitance_over_declared_bound` findings, SDA at 46,800 fF and SCL at 58,235 fF, each with its four terms. The same holds for any declared bound below 46,800 fF (both lines fail) or from 46,800 to 58,234 fF (SCL alone fails).
