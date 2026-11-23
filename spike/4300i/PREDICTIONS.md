# AET-WO-4300i — predictions, written before any run (§1.1)

Written 2026-11-23 on `main` `f50c29f4`, before the kit in this directory had run.

**Grounds.** These are taken from `4300h`'s record (FEEDBACK §1–§3, `RESULTS.md`), from code read on `f50c29f4`, and from the rules in `RULES.md`, which are committed with this file. No `validate`, `solve` or `emit` had run on this branch.

**One prediction is not blind:** P12, on the pull value. The held-document search reported before this file was written. P12 records what that search found; it is not a forecast.

## The universe

- **P1.** The committed current specifications are 34 files. That is the count `1218` and `1219` reported. A difference is a finding, and it is reconciled in RESULTS.

## The baseline (validate / solve / emit on `f50c29f4`)

- **P2.** Every committed specification validates (exit 0) except the ones meant to refuse. Each of those refuses with a named kind.
- **P3.** Every specification that validates also solves (exit 0).
- **P4.** `emit` succeeds on 25 boards, the same set `4300h` drew. A board `emit` refuses is refused by name.
- **P5.** Two runs of each command give byte-identical output. This is the hash base for the null sweep (§1.4).

## (1) The motor pull: which boards and nets gain the resistor

- **P6.** 9 boards declare `motor_outputs`. `4300h` counted 35 motor nets on 9 boards in one place and 37 rows in another. P6 says **35 nets**, and RESULTS reconciles 35 against 37.
- **P7.** Every one of those nets has `implied_pupd` = pull-down (`10`) today. **No committed motor output has `idle: {"level": "high"}`.** If one does, `idle_pupd` gives it a pull-up today, and "a pull-down on every motor line" collides with it. That collision would be a decision for the ruling.
- **P8.** Every board with motor outputs that `emit` draws draws each motor net, so each net can carry a drawn resistor.
- **P9.** Of the 35 nets, some end at a connector pin (Sentinel-03's J5, the DS-009 or motor headers) and some only at a pin device's bridge input (DRV8833 or DRV8876 `xIN`, which have their own 150 kΩ RPD typical). Both kinds are counted, by far end. Prediction: at least one board has bridge-input-only motor nets (`d1-drv8876`, `d2-drv8833`).
- **P10.** `boards/` and `fab/` hold no board with motor outputs. So (1) moves no `boards/` or `fab/` byte except through the version bump (P16).
- **P11.** The generated firmware writes a motor pull-down on every motor pin today:
  - `svd-hal`'s `motor_pull`, `PUPDR` `10`;
  - `hwdef`'s `PUSHPULL PULLDOWN` on every `PWM(n)` line.

  The CLI's witness ledger (`aethelgard-cli/src/ledger.rs`, `MOTOR_PUPD_RULE`) and `ledger_tier1` read the same rule. That is outside the WO's fence (F-fence below).

## The pull value

- **P12 (not blind: the search reported first).** No held document states an external pull-down value for a motor or ESC signal line, or an ESC's signal-input impedance or pull. AN4899 is not held. The one pull value held is DS12117 Rev 10's internal RPD, 30/40/50 kΩ min/typ/max (Table 49, p. 137, rev Y; Table 146, p. 241, rev V). That figure belongs to the pin, not to a board resistor, so **§7's no-held-source stop fires.**

## (2) FT at connectors, and (4) reservations: which pins move

- **P13.** Under `RULES.md` R2 (a stable FT-first partition of each connector-facing signal's candidates), the list of solver-chosen, non-FT, connector-facing pins is **empty on all 34**. So **0 pins move** by the preference. The hand TT pins at a connector are one, Sentinel-03 MOTOR1 on PE9, and that one is what (2) refuses before the re-pin.
- **P14.** On a scratch copy of Sentinel-03 with MOTOR1 on PA8:
  - `validate` passes (no `acknowledge_unwitnessed` entry names PE9 or PA8; `4300h` F6);
  - `solve` passes;
  - the solved topology differs from the baseline in MOTOR1's pin alone (PE9 → PA8; timer, channel, AF and timing unchanged).
- **P15.** In Sentinel-03's solve after the PA8 move, no pin sits on PA11, PA12 or PC9. So the reservation moves nothing, and no hand pin is on a reserved pin. On every other board no reservation is declared, so (4) moves nothing there by construction.

## The versions a build would move

- **P16.** Only the specification version moves, 27 → 28, and every committed specification moves by its version line alone. Through each one's hash, the generated `boards/` crates and `fab/sentinel-01-bench` move by a hash line. The solved-topology schema (17) moves only if the motor pull is carried into `SolvedMotorOutput`. The parts tables do not move, because `CRCW0603_F_K` is already a resistor row.
- **P17.** No committed board's pin moves other than Sentinel-03's MOTOR1. So §7's pin-move stop does not fire.
