# AET-WO-4300i — the measurement (§1.1), on `main` `f50c29f4`

**Kit.** The CLI was built from `45412fe0` (its crates equal `f50c29f4`) into `~/.cache/aet-4300i/target`, with scratch in `~/.cache/aet-4300i/w`.

| script | does |
|---|---|
| `universe.sh` | R0 |
| `run.sh` | R1, writing `out/baseline.tsv` and `out/emit-files/` |
| `motors.py` | R3, writing `out/motors.tsv` and `motors-summary.md` |
| `facing.py` | R2, writing `out/facing.tsv` and `facing-summary.md` |
| `repin.sh` / `repin.py` | R4 and R5, writing `out/repin*.tsv` and `reserved.tsv` |

The scripts were written and run by a worker agent. The builder re-derived the motor counts and the connector ends with a separate reader of the specifications, and they agree (10 boards, 40 nets, 34 connector-facing, no idle high).

## 1. Baseline (R1)

- **The universe:** 34 specifications.
- **Validate, solve, emit:**
  - Validate exits 0 on 32. `d2-drv8833` refuses `PartLimitUndeclared` and `invalid-zero-domain` refuses `ZeroNominalVoltage`, both by design.
  - Solve exits 0 on 29. The three `solve-*` stress files refuse `Infeasible`, by design.
  - Emit exits 0 on 25. Four declared stand-ins refuse `DrawsNoBoard`: `hse-external-8`, `imu-baro-1v8`, `d1-drv8876` and `sentinel-02-bench-hse8`.
- **Repeats:** each command ran twice on all 34, and the hashes are identical on all 34. Those hashes are the base for the null sweep.

## 2. (1) The motor pull (R3)

- **Who declares motor outputs:** 40 motor outputs on 10 boards.
  - The 9 boards that validate carry **35 nets**. That is `4300h`'s 35.
  - `d2-drv8833` carries 5 and refuses at validate.
- **Idle:**
  - DShot 24;
  - `{"level":"low"}` 6;
  - `{"pulse_us":…}` 10;
  - **`{"level":"high"}` 0**, so `implied_pupd` is pull-down on all 40.
- **Far ends:**
  - **34 at a connector pin.** Sentinel-03's four nets also reach TP19–TP22.
  - **6 at a pin-device bridge input only:**
    - `d1-drv8876`: `DRV_EN` and `DRV_PH`;
    - `d2-drv8833`: `A_IN1`/`A_IN2`/`B_IN1`/`B_IN2`.

    Each of these inputs carries its own RPD in its part row.
  - 0 with no end.
- **Drawn:** 7 boards emit a board with motor nets, **28 nets**, and every one of those nets is on its board (`(net "/MOTORn")`).
  - `hse-external-8` (4 nets) and `d1-drv8876` (3 nets) declare stand-ins and draw no board.
- **Which boards gain the resistor** (35 nets that validate):

  | board | nets | drawn |
  |---|---|---|
  | `driver-judges-nothing` | 4 | yes |
  | `hse-crystal-16` | 4 | yes |
  | `motors-dshot600` | 4 | yes |
  | `motors-pwm` | 4 | yes |
  | `sentinel-02-shaped` | 4 | yes |
  | `sentinel-03` | 4 | yes |
  | `serial-dshot-9600` | 4 | yes |
  | `hse-external-8` | 4 | stand-in |
  | `d1-drv8876` | 3 (2 bridge-only) | stand-in |
  | `d2-drv8833` (refused) | 5 (4 bridge-only) | — |

- **`boards/` and `fab/`:** they hold the `sentinel-01` family only, which has no motor outputs.

## 3. (2) and (4): which pins move (R2, R4, R5)

- **Connector-facing assignments:** 90 on the 17 solved boards that have connectors. 78 are solved and 12 are hand; 89 are FT and 1 is non-FT.
- **Moves under the preference** (solved and non-FT): **0.**
- **Refused as a hand TT pin at a connector:** **1**, Sentinel-03 MOTOR1 on PE9 (`TT_ha`), at J5.1.
- **The re-pin copy (MOTOR1 → PA8; the parsed specifications differ in that value alone):**
  - `validate` 0 and `solve` 0;
  - the solve output differs in exactly two paths: `/assignments/MOTOR1/pin` PE9 → PA8, and `/specification_sha256`. Timer, channel and AF are unchanged.
- **PA11, PA12, PC9 on Sentinel-03:** no pin sits on them, in the baseline or after the copy. The reservation moves nothing. Other boards that use them declare no reservation; they are listed in `out/reserved.tsv` for information only.

## 4. Predictions beside results

| # | prediction | result |
|---|---|---|
| P1 | 34 | **holds** |
| P2 | refusals named | **holds** (2) |
| P3 | validated ⇒ solves | **holds**, apart from the 3 `solve-*` files, which are meant to refuse |
| P4 | 25 emit | **holds** |
| P5 | repeats identical | **holds**, 34/34 |
| P6 | 35 nets on 9 boards | **holds for the validating boards.** 40 on 10 counts `d2-drv8833`, which refuses. The 37 in `4300h` is not reproduced here |
| P7 | no idle high | **holds** |
| P8 | every drawn board draws each net | **holds**, 28/28 |
| P9 | bridge-input-only nets exist | **holds**, 6, on `d1` and `d2` |
| P10 | no motor board in `boards/`/`fab/` | **holds** |
| P11 | generators write the pull | from code, not run (`svd-hal` `motor_pull`, `hwdef` `PULLDOWN`, the CLI ledger) |
| P12 | no held value (not blind) | **holds**; see `FEEDBACK` / the STOP |
| P13 | 0 moves, 1 refusal | **holds** |
| P14 | PA8 copy validates and solves, one pin moves | **holds** |
| P15 | no reserved pin used | **holds** |
| P16 | versions | for the build |
| P17 | no other pin moves | **holds** under R2 and R5 |
