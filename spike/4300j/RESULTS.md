# AET-WO-4300j — the measurement (§1.1)

Measured 2026-11-23 on `main` `cd79024c`, after the predictions were committed (`953e6413`).

**The binary:** `aethelgard` built `--release --locked` from `git archive cd79024c`, kept in `~/.cache/aet-4300j/base`.

## M1 · the baseline (P1–P4): all hold

- `universe.sh` lists **34** committed specifications (P1).
- `run.sh` validated, solved and emitted each twice. The output is `out/baseline.tsv` and `out/emit-files/`:

  | validate / solve / emit exit | boards |
  |---|---|
  | 0 / 0 / 0 | 25 |
  | 0 / 0 / 2 | 4 |
  | 0 / 2 / 2 | 3 |
  | 2 / 2 / 2 | 2 |

- **Every column of every row equals `4300i`'s merged-head record** (`spike/4300i/out/after/baseline.tsv`). That covers the exit codes, the refusal kinds, the solve hashes, the emit file counts and the emit tree hashes (P2, P3).
- Every repeat is byte-identical (P4).

## M2 · the motor boards (P5–P7): all hold

`motors.py` writes `out/motors.tsv`:

| board | outputs | validate | emit |
|---|---|---|---|
| d1-drv8876 | 3 | 0 | 2 |
| d2-drv8833 | 5 | 2 | 2 |
| driver-judges-nothing | 4 | 0 | 0 |
| hse-crystal-16 | 4 | 0 | 0 |
| hse-external-8 | 4 | 0 | 2 |
| motors-dshot600 | 4 | 0 | 0 |
| motors-pwm | 4 | 0 | 0 |
| sentinel-02-shaped | 4 | 0 | 0 |
| sentinel-03 | 4 | 0 | 0 |
| serial-dshot-9600 | 4 | 0 | 0 |

- **10 boards and 40 outputs.** 28 motor nets are drawn, on 7 boards (P5).
- **All 40 outputs declare `CRCW0603_F_K`, 40 200 Ω, to `GND`** (P6).
- **All 10 boards' `mcu.supply` is `VDD_3V3`, at 3300 mV ±50 ‰** (P7). So the bound at VDD min 3135 mV is the same on every committed board.

## M3 · the derivation (P8–P10): computed, then revised at the STOP

See `DERIVATION.md` and `derive.out`.

- **As predicted (same-rail):** floor 211 824 Ω and ceiling 895 714 Ω, both at VDD min. The value is 221 kΩ `CRCW0603_F_K`, worst case 214 370 to 227 630 Ω. The window holds every E96 value from 221 k to 866 k.
- **Revised:** reading DS70005664A p. 16 (M4) showed the ESC on its own rail. P8–P10 assumed one rail shared with the FC. Under the cross-rail case (`DERIVATION.md` §5a) the floor is 422 561 Ω. The value is then **442 kΩ**, and the window holds 442 k to 866 k.
- `STOP.md` leans to the cross-rail reading, D1(a), and keeps same-rail as D1(b).
- No E96 stop under either reading.

## M4 · the held pages, read rendered

Each page was rendered with PyMuPDF, read as an image, and checked against the figure cited:
- AN4899 p. 18;
- DS12117 p. 136, p. 137 and p. 241;
- DS70005664A p. 7 and p. 16. J2 was read cropped at 400 dpi;
- VISHAY_20035 p. 1, p. 2 and p. 3.

Every document's sha256 matches its record. **Every MCU number the bound needs is on a held page.**

**The ESC's numbers are not held,** and stand in under S1 and S2:
- its VIL, which the WO directs;
- its VIH, its input current and, under D1(a), its rail's tolerance, which go beyond that.

`STOP.md` names this as a near-stop, with the document a ruling would need.

**Two facts on the ESC reference page that the WO's grounds do not mention:**

- **The ESC's own rail.** Figure 3-3, "Power Supply": the ESC's `VDD` comes from `VREG` through `R54` 0R, or from `+3.3V_EXT`, and J2 pin 2 is that `VDD`. This revises S1 (M3).
- **The input capacitor:** `C4`, 0.1 µF from the signal line to GND, beside the 0 Ω `R11` (DS70005664A p. 16). It does not touch either DC bound. It bears on (b), the off-board drive bound, and goes to FEEDBACK as a row.

## Not measured until the build: P11–P18

P11–P18 forecast which bytes the build moves. The §1.4 null sweep judges them, against `out/baseline.tsv`.
