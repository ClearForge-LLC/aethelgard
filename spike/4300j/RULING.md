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
