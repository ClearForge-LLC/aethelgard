# AET-WO-4400h predictions

Written before the run each part predicts. Part 1 is committed before the first `simulate` run. Parts 2–4 are committed before the first injection run.

Each line carries a figure or a yes/no and its source:
- **[S]** is source read at a pin, given as file:line;
- **[R]** is a prior record;
- **[M]** is memory or judgement.

## Inputs, fixed for every run

- **ELF:** `5eace4ca7de47819e8728c403ff9506e72a9f79e2d16ca5a38e819b522c9dfe1`. It is CI's `generated-sentinel-02` artifact of `ardupilot-image` run 45410032458 on main `29b3a97d`, Copter-4.7.2 `fb773567`. Byte-identical with `AET-WO-2314`'s copy.
- **Specification:** `crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json` at `29b3a97d`.
- **CLI:** main `29b3a97d` plus `patches/window.patch`, built in the pinned tools image (`container/IMAGE`, `1e0ab105…`). The patch changes three things:
  - `FOREIGN_MAX_VIRTUAL_NS` goes from 5 s to 30 s;
  - the foreign run's wall and RSS bounds come from `AET4400H_WALL_S` and `AET4400H_RSS_KIB`, defaulting to main's `BOOT_BOUNDS` (30 s, 1,572,864 KiB);
  - `AET4400H_NO_USART_LOG` drops `2311`'s `LogPeripheralAccess` lines.

  Unset, the patched CLI is main's, except for the window's ceiling.
- **The board has no motor output.** `motor_outputs` is `[]`, and the hwdef has no PWM line. This is asked on the bus (from 4400h, 2026-11-24).

## Part 1: the window's cost

- **P1.1 The bound that binds first, with main's bounds unlifted, is the 30 s wall bound.** It is not the RSS cap.
  - At 5 s virtual: wall 11–16 s.
  - At 10 s: wall 24–34 s, so under today's load (12–16 on 20 cores) a 10 s run is killed by the wall bound in at least one of two runs.
  - At 20 s and 30 s: killed by the wall bound every time.
  - [R] `4400e`: 10.6–12.0 s of wall for 4 s virtual. `4400f` F1: 29.7 s for 4 s at load 26.
- **P1.2 Wall time grows linearly with the window:** 2.4–3.2 s of wall per virtual second at load 12–16 [R, M].
- **P1.3 Peak tree RSS grows with wall time, not with virtual time alone** [R: `4400f` F1]. With the wall bound lifted (`AET4400H_WALL_S=600`) and the log on:

  | window | peak (MiB) | share of 1,572,864 KiB |
  |---|---|---|
  | 5 s | 800–1,000 | 520–650‰ |
  | 10 s | 950–1,200 | 620–780‰ |
  | 20 s | 1,200–1,500 | 780–980‰ |
  | 30 s | 1,400–1,800 | 900‰ or more, possibly killed at the cap |

- **P1.4 Without `2311`'s access log, the peak is lower by under 10 %** at every window [M]. In a normal run, USART1's logged accesses are the console's DMA-fed TX and the driver's register traffic, not a polling loop. `1609`'s 2.59 against 1.10 GB was CrashCatcher polling USART1, which does not happen here.
- **P1.5 The no-log run's verdict still carries `memory`, but it fails the serial-options judge** (`serial_options_unwritten` or `unjudgeable`): the log is that judge's input [S: `foreign.rs` `judge_serial`].
- **P1.6 Log volume.** The kept log grows with the window and stays under the 8 MiB budget at 30 s, so no line is dropped [M; held loosely]. `4400e` kept 1.21 MB at 4 s, before `2311`'s log.
- **P1.7 The §7 stop "the cap fails before 10 s" does not fire:** no 5 s or 10 s run passes 1,572,864 KiB.
- **P1.8 The proposal.** The longest window whose worst peak stays at or under 900‰ is **10 s** (20 s is marginal), and it needs main's 30 s wall bound raised with it. Keeping the 30 s wall bound means keeping 5 s.
- **P1.9 A CPU-restricted run passes a higher share of the cap than an unrestricted one at the same window,** because RSS follows wall time. Restricting to 2 cores with `taskset` stands in for a loaded host.
