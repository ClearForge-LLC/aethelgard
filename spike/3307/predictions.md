# AET-WO-3307 — predictions, written before any run

Written 2026-11-23 from `main` at `28fba951`, before any run of the sweep and before the change. **Never edit a prediction: a disagreement is the result** (`INV-07`).

## The change these predictions assume

`witness_fields` (`crates/aethelgard-evolve/src/lib.rs`) reads a witness line in the form our BMP581 model logs today (`models/renode/BMP581.cs` `FinishTransmission`, L119–130), as well as the `Write XX`/`Read XX` form it reads now. Each BMP581 line is one whole transaction:

| Line (after `[NOISY] {device}: `) | Read as |
|---|---|
| `write 0xRR <- 0xB1 0xB2 …` | Write `RR B1 B2 …` |
| `write 0xRR <- (none)` | Write `RR` |
| `read 0xRR -> 0xV1 0xV2 …` | Write `RR`, then Read `V1 V2 …` |

- Each byte is `0x` and two upper-case hex digits, single spaces between, as `Join` writes them; the pointer is at most `0x7F`, as the model masks it. Anything else (an ICM42688P `bank N …` line, a `[WARNING]`, lower case, a missing `0x`, `(none)` after `->`) is not a witness line.
- The parse is local to `witness_fields`' reading of the register written and the value read. `transactions`, `shape`, `surplus` and `sent_elsewhere` (`AET-WO-3306`) still read only `Write`/`Read` lines, so a BMP581 log is never a surplus (out of this order's fence).
- `witness_fields` still refuses a read wider than 4 bytes, so the BMP581's 6-byte data step stays no evidence.

## The case the order is for

**bmp-0x47** (`spike/3306/spec/bme-bmp.json`, U2's identity read sent to 0x47, where U5, a BMP581, answers; re-measured on `28fba951`):

| | `main` | branch |
|---|---|---|
| U2's identity witness (logged nothing, check claims `ok`) | software (silent), already | unchanged |
| U5's identity witness (`[WARNING] … unmodelled register 0x50` only) | no evidence | **sound, nothing wrong** (no line it can read) |
| U5's configure witness (expected `write 0x36 <- 0x40 0x81`, observed `read 0x50 -> 0x00`) | no evidence | **software:** `i2c1.u5.Write` solved `0x364081`, observed `0x50` |
| U5's data witness (6-byte read) | no evidence | no evidence |
| rule | **I4**, evidence the two rises | **S1**, evidence the two rises, U2's identity witness and U5's configure witness; masked `[]` |
| `propose` | not run (no S rule) | a software rule; the fields above, U2's `Write`/`Read` wrong by silence |

- The evidence lands on U5's **configure** witness because the line distribution (`renode.rs` `attribute`: each step takes as many lines as it expects) puts U2's transfer in it. The field says U5's configure saw a read of 0x50, which is the misdirected transfer, not U5's configure. That is true of the record and is how a `Write`/`Read` witness reads too; it goes to FEEDBACK, not this order.
- U2's write `0xD0` is logged as `0x50` (the model masks the pointer with `0x7F`), so no step of U2's shape matches, even if surplus read this format. No step address field is named.

## Every other run: unchanged

Byte-identical `attribute` output from `main`'s CLI and the branch's on the same archive, and for any software rule the same `propose` files and exit code:

- **the seeded set** (`scenarios/sentinel-01/seeded/cases.json`: the unmutated cases in the sentinel-01-minimal group, and the 13 mutants over their own cases): every rule, evidence and masked as `cases.json` pins. No device there is a BMP581, and the `Write`/`Read` reading does not change.
- **every committed scenario group:** sentinel-01-minimal, sentinel-01-bench, motors, serial, sentinel-03 (U7 is a BMP581), sentinel-02-power. On sentinel-03 every BMP581 witness logs its script (passing runs, droop, BOR) or nothing (held-low SDA, a timeout): readable now, never wrong, so no rule, evidence or mask moves.
- **generated two-device and BMP581 cases:** ext-bmp581 and two-bme (baseline, held-low SDA); two-bme with U2's step sent to 0x75 and 0x77; bme-bmp baseline and held-low SDA; sentinel-02-own (U7 a BMP581 on SENSORS) baseline and held-low SDA.
- **the bench package:** `verify --against-fab fab/sentinel-01-bench/package` in CI's layout, byte-identical JSON from `main` and the branch. The bench board (`examples/sentinel-01.json`) has no BMP581. No `fab/` byte moves.

## One demonstration that is not a collide (not committed)

**ext-cfg37:** ext-bmp581 with U5's configure write sent to register 0x37 instead of 0x36 (generated board crate patched once, as 3306's mutants are).
- `main`: no BMP581 witness is evidence, so the U5 symptom is **I4**.
- branch: U5's configure witness logs `write 0x37 <- 0x40 0x81` against `write 0x36 <- 0x40 0x81`, so **S1**, evidence U5's configure witness, field `i2c1.u5.Write` (or EXT's device path) solved `0x364081`, observed `0x374081`.

## Stop (§7)

Any move in any run but bmp-0x47 and ext-cfg37, or any `fab/` byte, stops the order with the diff.
