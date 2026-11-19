# AET-WO-4240 predictions, committed before any measured run

Written 2026-11-19 by the builder, before any scratch solve, build or run of this spike.
Each is scored in `docs/work-orders/AET-WO-4240.FEEDBACK.md`. Labels: [inferred] = desk arithmetic or
reasoning; nothing here is measured.

## The architect's (from the work order, §1.0.2, verbatim in substance)

- **P1.** On a crystal of ±50 ppm total at the 64 MHz kernel, CRSF 416666 solves with about 22,650 ppm
  of margin (`BRR` 154; quantization 2,596; peer 8,002; Table 407 33,300) [inferred].
- **P2.** A PLL from the HSI does not close Q-B. At 100, 200 or 240 MHz kernels, 416666 solves (margin
  about 3,605 ppm) but 420000 fails at 100 and 200 MHz (about −207 ppm) [inferred]. Only the crystal
  closes both bauds.
- **P3.** With an integer PLL1 and RM0433's limits, 64 MHz exactly is reachable from 8, 16, 24 and
  25 MHz, and unreachable from 8.333 MHz (25 MHz ÷ 3) [inferred; limits not yet cited].
- **P4.** DShot600's bit band shrinks from 1.647–1.716 µs to within ±0.1 ns of 1,656.25 ns, and PWM's
  idle pulse from up to 1,026.6 µs to 1,000.0 ± 0.1 µs [inferred].

## The builder's

- **P5.** The NUCLEO-H753ZI's default `PH0` source is the ST-LINK-V3E's `MCO` at a nominal 8 MHz
  (UM2407 §7.5.1 says so). The builder predicts it is derived from the ST-LINK MCU's **HSI/2**, not from
  a crystal, because the ST-LINK's own crystal divides to 8.33 MHz, not 8.000 [inferred]. If so, the
  bench in its default state witnesses no crystal ppm at all.
- **P6.** The bench ELF `375fd31a…9c02` reproduces from `cd43b207`, and a scratch build with the
  clock knob compiled in but unwired gives the same sha [inferred].
- **P7.** Under H1 (HSE → PLL1 → 64 MHz) no fixture's `solve` gains a refusal, and every `PSC`, `ARR`
  and `BRR` stays numerically the same [inferred]. At most the I2C `TIMINGR` words may move, because
  they are judged at the kernel's extremes; the builder predicts **none** moves [inferred, weak].
- **P8.** Under H0 (an 8 MHz crystal as `sys_ck`) at least one motor fixture gains a refusal, because a
  DShot600 bit at 8 MHz is 13.3 ticks [inferred].
- **P9.** Under H2 (400 MHz `sys_ck`, 200 MHz timers) every motor fixture's `PSC`/`ARR` moves and no
  refusal appears that is not also present under the HSI [inferred, weak].
