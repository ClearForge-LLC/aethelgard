AET-WO-4300i: the motor pull as a declared 40.2 kOhm part (specification 28), no internal motor pull in either generator; FT pins only at connectors; Sentinel-03 MOTOR1 on PA8; mcu.reserved_pins honoured by every solver branch

**READY.** The bench package is dispatch run 45117832704's, byte for byte. Only hash lines moved.

Records:
- `docs/work-orders/AET-WO-4300i.FEEDBACK.md`
- the measurement and stop: `spike/4300i/RESULTS.md`, `spike/4300i/STOP.md`
- the rulings: the architect's, 2026-11-23T01:15Z, on the bus

- **(1)** `motor_outputs[].pull`. It is required on all 40 outputs. `to` must be GND, and the value must lie inside DS12117 RPD's 30–50 kOhm, which is held as data (power-parts 9). It is drawn on 28 nets on 7 boards and judged by support step 8 from the netlist.
  - svd-hal writes motor `PUPDR` 00, and hwdef writes `FLOATING`.
  - `implied_pupd`, `check pinout` and the ledger judge 00.
- **(2)** At a connector the solver offers FT pins only, with non-FT candidates withheld and named. A hand TT pin there is refused (`ConnectorPinNotFt`).
- **(3)** Sentinel-03's MOTOR1 is on PA8, and PA11, PA12 and PC9 are reserved (PC9 for E5's MCO2).
- **(4)** `mcu.reserved_pins` is honoured in every branch, and a hand pin on one is refused with its reason.
- **The adversarial pass (full):** F1 (high), F2, F4, F5 and F8 are fixed. F3, F6 and F7 are recorded as decisions.
- **Mutants:** IR 42/42, B1 6/6, B2 22/22.
- **Null sweep:** 34/34. No committed pin moved except Sentinel-03 MOTOR1. Emitted bytes moved on the 7 motor boards only.
- **Reduction:** −61 net.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_0165X1yuj27R7ExC5L5EZM5C
