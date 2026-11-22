# AET-WO-4300h — the options as rules, committed before they ran

`judge.py` is the rule and this page is its summary. Both were committed before `judge.py` first ran. The rows it judges are walk 1's (`out/facing/*.tsv`). Walk 2 (`out/inward/*.tsv`) reconciles them on every drawn board (`out/compare.tsv`). The pins a move lands on, and what each move changes, are `out/alts.tsv` and `out/moves.tsv`.

## The rows

- **An off-board end** is a `connectors[]` pin or a `test_points[]` entry on an MCU net.
- Two more classes no option of the work order sees are listed and named, never judged:
  - `undeclared_offboard`: the bench's header wiring, from `sidecars/undeclared-offboard.tsv`;
  - `foreign_clock`: `mcu.hse.external` onto PH0.
- **The walk-2 pass-through rule** (`inward.py`):
  - from each J/TP pad, a two-pad R, L, FB or D passes a DC path, up to 3 hops, onto signal nets only;
  - anything else ends the walk and is listed by name (`ic_bridge` when its other pads carry MCU nets);
  - an end on a rail reaches MCU pins only through on-board parts (`via_rail`), which A1 and `1216b` judge.

## The limits

**With VDD up:** DS12117 Rev 10 Table 13, p. 105, read as A1 reads it (`1211e`, ruled D-A1), with `mcu.supply` at its minimum (`nominal × (1 − tolerance)`):
- `TT_xx`: VDD + 0.3 V.
- `FT_xxx`: `Min(VDD + 3.6 V, 5499 mV)`, from the row "all I/O except BOOT0 and TT_xx" (< 5.5 V).
- BOOT0 (`B`): 9.0 V.
- `RST` (NRST): FT's formula by the same row, but Table 7 gives NRST an embedded weak pull-up and note 3 requires the pulls off above VDD + 0.3 V. It cannot be disabled, so it is **VDD + 0.3 V**.
- **Note 3, for a pin holding an internal pull from reset** (PA13 up, PA14 down, PA15 up, PB4 up; RM0433 Rev 9 p. 542; A1's `MCU_RESET_PULL_UPS`/`_DOWNS`): VDD + 0.3 V whatever its structure.
  - A pull the generated firmware enables is not in the solved topology. That case is not judged here, and the same is true of A1.

**With VDD = 0:** DS12117 Rev 10 Table 10, p. 103, as `1216b` reads it: every pin **4.0 V**, BOOT0 **9.0 V**. An off-board end can always lead `mcu.supply` (`1216` walk 1, `can_lead` "yes: off-board"). The one exception is a bound declared to track `mcu.supply` (below).

## The options

### (a) The solver prefers FT for a signal an off-board end reaches; a hand TT pin there is refused unless the specification states why

- **`a`:** every `connector` or `test_point` row on a `TT_xx` pin.
  - **A hand pin is refused** (`OffBoardPinNotFt`). The specification has no field for a stated reason, so every such hand pin is refused today.
  - **A solver-chosen pin is moved** to the first free FT pin carrying the same signal, in the solver's pin order (`alts.tsv`).
    - The move is emulated by hand-pinning a scratch copy (`moves.tsv`), because no crate changes.
    - A move whose copy does not `solve` is refused (`OffBoardNoFtPin`).
    - A move that solves but fails `validate` on a stale acknowledgement is **moved, with a cost**: the acknowledgement names the old pins' fields.
- **What (a) cannot judge:**
  - RST and B: they are not GPIO and cannot move;
  - `undeclared_offboard` and `foreign_clock` rows;
  - any VDD = 0 case, because Table 10 gives FT and TT the same 4.0 V.

### (b) A declared off-board drive bound per connector pin, judged by A1 with VDD up and by `1216b`'s rule at VDD = 0

- **The shape:** `connectors[].pins[].offboard.drive_max_millivolts` with a source.
- **Inputs:** today, no specification declares a bound. The bounds a specification could declare are hand sidecars (`sidecars/drive.tsv`), one scenario each:
  - **`ds009-nominal`:** 3300 mV on every DS-009 signal pin, DS-009's printed nominal (`kind` nominal: not a maximum).
  - **`ds009-plus5`:** 3465 mV, the nominal read as +5 %. Hypothetical: no held page states a tolerance.
  - **`j5-5v-esc`:** 5250 mV on Sentinel-03's J5.1–J5.4, a 5 V-logic peer (`1216` F1). Hypothetical: `esc` is `none`.
  - **`probe-vtref`:** the SWD/NRST/BOOT0 pins, bounded by `mcu.supply` itself. Hypothetical: no probe datasheet is held.
    - Judged like A1's same-rail exemption (VIN never exceeds VDD), and at VDD = 0 as tracking it (0 V).
    - This is a declaration, not drawn wiring.
- **`b-required`:** a connector row with no bound is refused (`OffBoardDriveUndeclared`).
- **`b-declared`:** a connector row with no bound is named, not refused (as `1216` D2 ruled for unbounded paths). A declared bound is refused:
  - above the VDD-up limit (`OffBoardDriveAboveTolerance`);
  - or above the VDD = 0 limit (`OffBoardDriveAboveLimitUnpowered`).
- **Test points are not `connectors[]` pins.** (b) bounds none of them. They are named.

### (c) Both

(a)'s refusals and moves first, then (b) on the pins as moved, by the same scenario. Also run on Sentinel-03 as the ruling's build would leave it: MOTOR1 on PA8 (`moves.tsv`).

## The count

- **Per option:** the boards refused, the rows refused by kind, the boards moved, and the rows named (unbounded or unjudged), over the 34.
- **A board refused by validate or solve today** (D2, `invalid-zero-domain`, three `solve/` fixtures) is judged on the rows walk 1 could read. A row whose pin is `unsolved` is named, not judged.
