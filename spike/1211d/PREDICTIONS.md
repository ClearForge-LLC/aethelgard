# AET-WO-1211d — predictions, committed before any run

Written 2026-11-20, on `25203c58`, after the 18:18Z rulings (S1 (b), Q1, Q2), before any line of the build.

## The null sweep (§1.1)

- **N1.** `emit` (schematic, board, BOM and notes) on every committed specification: **byte-identical**, branch against `main` `25203c58`, at each of three points: after the vendoring commit, after the arms, and at READY (main merged). The same exit codes and refusal kinds for `validate`, `solve` and `emit`.
- **N2.** No committed specification declares a pin device on a board it draws (`1210`'s rule held that until now), so no committed specification reaches either arm.
- **N3.** `aethelgard_ir::names::own_nets` is unchanged on every committed specification (Q1: a specification with no pin device lists exactly what it listed).

## The arms

- **A1.** D2 (DRV8833PWPR, two IN/IN bridges), drawn at library level from an eda-side drawn copy, passes ERC, DRC (`--spec`, with parity), `check support` and `extract`.
- **A2.** D1 (DRV8876PWPR, PH/EN) drawn at library level is refused `PassiveValueUnstated {U2, DRV8876PWPR, CVM2}` and nothing else; with that one refusal stubbed in a test-only path, its symbol and footprint place and its every other passive is drawn.
- **A3.** The DRV8833 symbol and the PWP0016C footprint, vendored, match their GitLab blobs at tag 10.0.7 byte for byte. The DRV8876 symbol and PWP0016A footprint, authored, survive `kicad-cli … upgrade --force` unchanged in every cited dimension.
- **A4.** The WO's three mutants (a strap left floating, nFAULT's pull-up missing, a bridge output to no connector) are each killed by a test.

## 4600 D4 (measured in `spike/1211d/d4/`)

- **D4a.** The six device arms (BME280, BMP581, TMP117, W25Q64JVxIQ, ICM20948, ICM42688P) can be written as one table of rows (a pin map onto roles, straps, decoupling and support passives) read by one interpreter, with `emit` byte-identical on every committed specification.
- **D4b.** What the rows cannot hold as plain data, each needing an interpreter feature rather than a row field: the address-strap choice (`strapped`: BME280, BMP581, TMP117), the translator's B-side substitution (`power::behind`, SPI parts), W25Q's value and stand-in note, and the order of passives (BMP581's second capacitor, after the device).
- **D4c.** So the per-part arm can be row data without moving any existing part's bytes, provided the row order reproduces today's reference order. It is not free: it is a proposal, not built here.
