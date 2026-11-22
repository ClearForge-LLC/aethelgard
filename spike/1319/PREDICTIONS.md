# AET-WO-1319 — predictions, committed before any run

Written 2026-11-22, on `main` `c1e444dd` (`TIER2_MIN` 201), before the baseline is taken and before any line of the port.

## The baseline (§1.1)

- **B1.** The committed specifications are the 33 that `spike/1211d/measure/sweep.sh` selects at `c1e444dd` (31 at 1211d's start; `hse-crystal-16` and `hse-external-8` since).
- **B2.** `d4diff.rs`, the 1211d reviewer's harness, copied unchanged to `spike/1319/harness/`, still compiles against `c1e444dd` once `Drawing::dump` is supplied. `dump` was the reviewer's shim and is not on `main`. Here it is `format!("{:#?}", parts)`, applied identically to both trees and never committed. It still writes 274 cases.
- **B3.** `d1319.rs` is this work order's own harness (§5: inputs the 274 do not cover). Every one of its cases declares a pin device, or crosses kinds: a pin device's part on a bus, or a bus part as a pin device.

## The port (§1.2–§1.4)

- **P1.** **The committed specifications: identical.** Every committed specification's `validate`, `solve` and `emit` exit code and printed JSON (with the output directory normalised by `same.sh`), and every emitted file, are byte-identical, branch against base.
- **P2.** **Both harnesses: identical.** For `d4diff`'s 274 cases and every `d1319` case, every summary (validate, solve, draw, emit kinds and messages), every part dump and every emitted file is byte-identical, branch against base (`diff -r` empty).
- **P3.** **Folding in the pin-device arms needs no interpreter feature beyond what exists today.** The pin-device arms already work through a table of the same shape. Their features (thermal pad, the VM-derived rating, a pin-to-pin passive, the anchor and its `near` rule, `PassiveValueUnstated`, the pin map from the IR row plus the declaration) are moved, not invented.
- **P4.** **Net line count.** The patch measured +108 for the six bus arms. The pin-device arms are already table-shaped, so folding them in costs little. Predicted net change in `crates/`: **+60 to +200 lines**, the table longer than the arms it replaces.
- **P5.** `parts_arms_tier1` passes with every part drawn. `TIER2_MIN` is unchanged at 201.

## Mutants (the harnesses can fail)

1211d's two planted mutants are not recorded by name. Its saved `d4out-mut` shows one of them, the W25Q `WP`/`HOLD` order swapped. Here they are reconstructed, with one added for the pin devices:

- **M1** (the saved one): W25Q64JVxIQ's `WP` and `HOLD` rows swapped. Predicted to move every W25Q case in `d4diff`, and every committed specification that draws a W25Q.
- **M2** (the I2C side): BMP581's second 100 nF decoupler left out. Predicted to move every BMP581 case in `d4diff`, and `sentinel-01-ext-bmp581`.
- **M3** (the pin side): DRV8833's CVCP and CVINT drawn in the other order. Predicted to move every `d1319` case that draws a DRV8833, and no committed specification's emitted bytes, because `emit` refuses a pin device at the CLI until `1211b` lands.

## The four hazards (§5)

- **H1.** An own-net capacitor's net: the interpreter takes it from the row's `Own` pin, not from a second `format!`. A row whose capacitor names no `Own` pin of its own is refused by the table check, and a test shows it.
- **H2.** An address role on an SPI row is refused by the table check, which is evaluated at compile time over the real table and by a test over a bad one.
- **H3.** A duplicate row is refused the same way.
- **H4.** An I2C row without its leading 100 nF decoupler is refused the same way.
