# AET-WO-1315 — FEEDBACK

Labels: **[measured]** by a run named beside it; **[read]** at the source named beside it; **[inferred]** from arithmetic or reasoning.

## 0. Prediction (§1.0), committed before the first run of the new emitter

The baseline is `main` `cd43b207`'s release binary, swept over all 36 JSON files under `examples/` and `crates/**/tests/fixtures/` (`emit`, `solve`, `generate`) into `~/.cache/aet-1315/base`.

| Figure | `main` | Predicted | Label |
|---|---|---|---|
| Loop rules on the power fixture (`board_tier2.rs:1186`) | 17 | **18**: one more, C1's, anchored to U10's VIN and PGND | inferred |
| Loop rules on Sentinel-03 | as measured on `main` | **+1**, the same rule | inferred |
| Sentinel-03 outline under its 100 × 100 mm bound | 85.25 × 80.95 mm | **grows in one axis, by under 10 mm**, still inside the bound: CP_Elec_8x10's courtyard (about 10 × 9 mm) is the largest passive on the board and joins U10's group | inferred |
| Sentinel-03 least `extract` slack | 9.2 pF | **9.2 ± 0.5 pF**: C1 is on `VBAT`, no judged I2C or SPI net; slack moves only if placement moves a judged line | inferred |
| Emitter-numbered capacitors on both power boards | from `C1` | **each moves up by one** below the first declared number (`C1` now claimed); none above it moves | read (`schematic.rs:643`, `Builder::reference`) |
| Every board without a power tree, Sentinel-01's package | — | **byte-identical**: every changed path reads `spec.power` or a `Stack::of` error that no committed specification raises | read |
| Controls drawn from the declaration (step 4 alone) | — | **no byte moves** on either power board: both declare EN `input`, PG `open`, OE `vcca`, which is what the emitter drew | read (WO §0, both fixtures) |
| Pinned figures that move | — | `board_tier2.rs` loop count; `outline_tier1.rs:330` outline and the name-free placement digest; the `ANSWER 1313c-fixture` line; any test naming an emitter-numbered capacitor of a power board | inferred |
