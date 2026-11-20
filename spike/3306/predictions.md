# AET-WO-3306 §1.1 — predictions, written before any run

Committed before `spike/3306/sh/measure.sh` runs. Main is `a75fffcc`. Every run is made **once**, by main's CLI. `attribute`, and `propose` where software, are then judged on the same archives: first by main's CLI, then by the branch's. This order changes no twin, trace or firmware.

## What runs

| Group | Firmware | Scenarios |
|---|---|---|
| min | sentinel-01-minimal (= `boards/sentinel-01`) | `scenarios/sentinel-01/*.json`, and the seeded set's unmutated scenarios |
| s01 | sentinel-01 (bench) | `scenarios/sentinel-01-bench/*` (5) |
| dshot, pwm, serial | the motor and serial fixtures | `scenarios/motors/*`, `seeded/motors-pwm`, `scenarios/serial/*` |
| s03 | sentinel-03 fixture | `scenarios/sentinel-03/*` (11, `droop-bor3` included) |
| pow | sentinel-02-power | `scenarios/sentinel-02-power/*` (3) |
| ext | sentinel-01-ext-bmp581 | generated baseline, held-low SDA on EXT |
| two | two-bme | generated baseline, held-low SDA |
| two-0x75 | two-bme, U2's step sent to 0x75 (no device) | one run |
| **two-0x77** | two-bme, U2's step sent to 0x77 (U5, a BME280, answers): **F-COLLIDE** | one run |
| bmp | bme-bmp (`spike/3306/spec`: two-bme with U5 a BMP581 at 0x47) | baseline, held-low SDA |
| **bmp-0x47** | bme-bmp, U2's step sent to 0x47 (U5, a BMP581, answers): a different-part collide | one run |
| seeded-* | each of the seeded set's 13 mutants | its own cases' scenarios |

## Per-device classification (`dump3306`)

Each device's whole observed sequence is its witnesses' `observed` concatenated in record order. The I2C attribution in `renode.rs` is "each step takes its count, the last step the rest", so the concatenation is the device's whole log. It is compared with its `expected` steps.

| Prediction | Runs |
|---|---|
| `equal` | every device of every passing run, and every droop, pull-up, capacitance, motor, serial and SPI run |
| `empty` | every device on a held-low bus (ext, two, bmp, s03, seeded held-low); U2 in two-0x75, two-0x77, bmp-0x47 and seeded wrong-address |
| `other` | U2 in seeded wrong-register (`Write D1`) and wrong-nbytes (`Read 60 xx`, one line of two bytes) |
| `surplus` | **two-0x77's U5 only** (extra `Write D0`, `Read 60`), and **bmp-0x47's U5** (extra `Write D0`, `Read <the BMP581's 0xD0>`) |

The bmp-0x47 surplus is the uncertain one. If the BMP581 model logs a warning for register 0xD0, the class is `other`.

Seeded `leave-firmware` and `no-boot-word` keep every I2C witness `equal`: the step runs before the fault.

## Attribution on main (rule, evidence, masked)

- **The seeded set:** exactly as `scenarios/sentinel-01/seeded/cases.json` pins it (30 cases; CI green on `a75fffcc`).
- **Committed scenarios:** each passes or fails as its expectation says.
  - Each passing run is N1.
  - `spi-absent-*` is E2.
  - `serial-rx-open-*` is E3.
  - `vdd*-out-of-band` and battery-sag are as the committed tier-2 tests pin them.
  - `droop-bor3` is as 2305 merged it.
- **two-bme:** baseline N1; held-low E1, masked `[]`; 0x75 S1 with U2's witness.
- **two-0x77: I4.** 3305 measured this at trace 16; the archive is re-made at main's trace 17.
  - **Per symptom:** U2's `i2c_transaction_unwitnessed` decides **S1**, because its check claims `ok` and its witness is empty (`silent`, `witness_fields`).
  - U5's decides **I4**: its first `Write` and first `Read` are its own, its address field is not wrong, and the bus is healthy.
  - An indeterminate rule wins the run (`attribute`), so the run is I4.
- **bmp-0x47: S1.**
  - U2's check reads the BMP581's 0xD0, not 0x60, so it fails with `i2c_identity_mismatch`; its witness is empty, so its symptom decides S1.
  - U5's identity witness logs `Write D0` first, not its own `Write 01`, so it is wrong today and decides S1.

## After the rule (the branch's `attribute` on the same archives)

- **two-0x77 moves:** I4 → **S1**. Evidence is U2's witness `[28,13]`, **U5's witness `[29,13]`**, and the rises; masked `[]`.
  - U5's witness carries a wrong field naming the extra transaction at 0x77 (U5's address, the one the firmware used).
  - The field also names U2's `identity` step: U2 is the one other device on the bus whose step has that write and read width.
- **Every other run's rule, evidence and masked are unchanged,** bmp-0x47 included: its U5 witness is already evidence.
- **The `propose` generator report moves for bmp-0x47 only:** U5's witness gains the surplus field.
  - Every other S-rule run's report is unchanged, because no other run has a surplus.

**If any run besides two-0x77 and bmp-0x47 classifies `surplus`, or moves its rule, evidence or masked, that is the WO's §7 stop.**
