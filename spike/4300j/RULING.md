# AET-WO-4300j — the ruling on STOP.md (architect, 2026-11-23T10:11Z, bus)

- **D1 (a), cross-rail: 442 kΩ.** The 10 % margin stays. S1 and S2 go to FEEDBACK as R1. Not a stop.
- **D2:** the smallest E96 value inside the bound (442 k).
- **D3 (a):** the inputs are held as `io.motor_pull_bound`, with their citations. The bound is computed per board from `mcu.supply`, in integer arithmetic. `MotorPullOutsideBound` is refused at both edges, each with a planted test.
- **D4:** `CRCW0402_Z_0`, a new `jumper` kind (power-parts 10). **Stop** if DRC or the router refuses the 0402 on any committed motor board. Do not switch to 0603.
- **D5:** the link sits at the motor-net end, `MOTORn_PD` is in `names::own_nets`, and support step 8 judges the chain.
- **D6, and D9 (topology 18):** granted.
- **D7, D8:** the leans. Stop only on a committed pin move, or on a `fab/` byte beyond the hash lines.
- **Out of fence, granted:** `cli/src/ledger.rs` (`MOTOR_PUPD_RULE`), and the twin `REFERENCE` literals. The architect dispatches the package when it is asked for on the bus, once CI is green.
- **Not here:** Sentinel-03 at 4S, and its crystal (that is 4300k).

## F2 and F3 (architect, 2026-11-23T12:25Z, bus): both leans

- **F2:** the ESC's rail is a stand-in input of its own: `peer_rail_millivolts` 3300 and `peer_rail_tolerance_permille` 50, each `{value, basis}`, citing DS70005664A p. 16 and S1.
- **F3:** the row's tolerance + drift band is judged. The low corner clears the floor, and the high corner the ceiling. 432 k is refused; 442 k is admitted.

**One deviation, flagged.** The ruling asked for a test that "a board on VDD_3V3 ±1 ‰ still gets the 422 561 Ω floor". That number is wrong under the derivation:
- The pull-up works from the board's own rail. A ±1 ‰ board has a VDD min of 3296.7 mV, not 3135 mV, so its pull-up is stronger, and the floor is 1 386 000 000 / 4897 = 283 030.43 Ω.
- The ceiling is the ESC's alone and stays 895 714 Ω.
- The finding's concern holds: 221 k is still refused there, because its band starts at 291 785.

The test plants the derived numbers.

## D-d (architect, 2026-11-23T13:10Z): accepted

The floor follows the board's own pull-up rail. The ruling's "422 561" assumed a 3V3 ±50 board. The derived-number test stays.
