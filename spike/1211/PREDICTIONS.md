# AET-WO-1211 — predictions, written before any reading fan-out or measurement

Written 2026-11-20T13:55Z, on `main` `472d2d84`, before any code read past `ir/src/motor.rs` and
`ir/src/lib.rs` L95–470. I had read neither datasheet's body, nor Renode's source, nor any
committed specification beyond its `schema_version` and its `motor_outputs` count. This file
has exactly one commit. FEEDBACK puts each result beside its prediction.

## P1 — what moves under the schema with `idle` = `pulse_min_us` and `resolution` = 1 µs (§1.4)
- **P1a.** On every committed specification and solved topology with a PWM output, no solved
  `PSC`, `ARR` or `CCR` moves. Those words come from `solve`'s real output, not from a copy of
  `solve_timing`. DShot outputs do not move either, since they declare no `pwm` and take no
  `idle` value.
- **P1b.** No emitted byte moves **by value**: register words, init order, the ELF's code and
  the board files. Bytes **do** move **by version or hash**: `schema_version`, the specification
  file and its sha256, the solved topology JSON (which copies `pwm`), and every artifact that
  embeds the specification's or topology's hash or serialized form. FEEDBACK names each such
  artifact at source.
- **P1c.** Every PWM output in every committed specification must gain an `idle` and a
  `resolution`. No other field is forced on a specification that declares no driver.

## P2 — the datasheets
- **P2a.** On both the DRV8833 and the DRV8876 in IN/IN mode, both inputs high is **brake**
  (low-side slow decay), a legal state that slow-decay PWM uses every period. So "a pair never
  driven high together" as worded forbids slow-decay drive.
- **P2b.** In PH/EN mode, PH is a direction level and EN carries the PWM, so both high is
  forward drive. The pair rule is not "never high together" there. Each mode needs its own
  bridge rule.
- **P2c.** Each part's nSLEEP has an internal pull-down, so the part sleeps from MCU reset until
  the firmware's first write. nFAULT is open-drain and needs an external pull-up; the datasheet
  states its value.
- **P2d.** IPROPI's transfer is 1000 µA/A (A_IPROPI) with a tolerance band, and its voltage
  range is bounded by VREF or ILIM, which also sets the regulation limit.

## P3 — RM0433 / DS12117
- **P3a.** Most GPIO pins reset to analog mode (`MODER` = `11`). A driver's inputs therefore
  float, or follow the part's internal pulls, from reset until the first `MODER` write. A
  declared initial level covers only the time after that write. Before it, the pins are the
  board's job: a strap resistor, or the part's internal pull.
- **P3b.** Writing `ODR` (or `BSRR`) before `MODER` = output gives a glitch-free initial level.
- **P3c.** Each of ADC1–3 converts 0 V to VREF+. The sampling time is set per channel in
  `SMPR`. A channel's pin is fixed by the package's pin table (DS12117). ADC3's channels sit on
  ports F, C and H.

## P4 — the twin (Renode `cd4b002a`, the pinned image)
- **P4a.** `STM32_GPIOPort` models `MODER`, `ODR`, `BSRR` and `IDR`. A monitor script can read a
  pin's output state and can drive an input pin's state, so a scenario can pull nFAULT low and
  `IDR` follows.
- **P4b.** `PUPDR` is a stored register with no effect on a pin that nothing drives, so the twin
  cannot witness an input's pull. It is judged by rule only.
- **P4c.** The twin can witness the order of the initial-level write before `CEN` as register
  writes, but not as an electrical level in the reset-to-first-write window.
- **P4d.** The H7 ADC model exists in source. Whether the stock `stm32h753.repl` instantiates it
  is a coin toss; I predict it does **not**. If it does, a script can set a channel's value, so
  `DR` returns a declared sample.

## P5 — the schema
- **P5a.** `resolution` must be stated in a unit that is exact at a 64 MHz kernel clock (one
  tick = 15.625 ns), as picoseconds or as kernel ticks, not as whole ns. 20 kHz at `PSC` 0 gives
  `ARR` 3199, or 3200 steps.
- **P5b.** `PSC` and `ARR` belong to the timer, so two outputs on one timer must agree in rate
  and resolution, or be refused by name.
