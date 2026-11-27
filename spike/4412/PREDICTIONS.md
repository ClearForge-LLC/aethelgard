# AET-WO-4412 predictions (part 1)

Committed before any Renode run of the ELF below, smoke runs included, and before its hash was read. Graded in `RESULTS.md`.

**Ungraded, said plainly:** the DMAMUX table (P5) and the triage (P8) were written after reading two research workers' reports (`~/.cache/aet-4412/dmamux/report.md`, `~/.cache/aet-4412/triage/triage.md`). Their predictions are therefore not predictions, and are listed only so the table has a row for them. Both reports are claims, which I check against the PDF and the code in `RESULTS.md`.

## Inputs, named

- **Board:** main's Sentinel-03 (`examples/sentinel-03.json` at `eea3198e`, sha256 `f6d409a3…`, unchanged since `4400j`).
  - *Today's row:* the file as committed.
  - *The runnable row:* `py/derive.py` (`4400j`'s, unchanged) adds `mcu.foreign_clock` from the pinned ArduPilot image's `stm32h7_mcuconf.h` for `OSCILLATOR_HZ 16000000`. The derived spec is `0c5bb563…`, the same bytes `4400j` ran.
- **ELF:** unmodified Copter-4.7.2, built by CI's own `container/ardupilot-build.sh` in the pinned image `e1809b90…`, from the hwdef `generate hwdef` writes for that spec (`7f2b62aa…`, the same as `4400j`'s).
- **CLI:** main `eea3198e`, **unpatched**, for every "today" row. A patched CLI (bounds lifted, a reset macro) is used only to read past a bound, and every such row says so.
- **Inputs:**
  - `arm12` (`4400i`'s, `7f231564…`);
  - `dshot12`: `arm12` with `MOT_PWM_TYPE` 6 sent before `FRAME_CLASS` each time;
  - `reboot12`: `arm12`'s parameters, then `MAV_CMD_PREFLIGHT_REBOOT_SHUTDOWN` (246, param1 1), then reads of the parameters after the second boot.
- **Renode's stdin:** as `run.rs` gives it now (`HELD_STDIN`). The `4400j` harness defaults to `/dev/null` for replays, and is set to the held pipe here.

## P1. The board (HSE)

| # | prediction |
|---|---|
| P1.1 | Main's `examples/sentinel-03.json`, unmodified, is refused before Renode by `simulate --foreign` with `ForeignClockUndeclared` (exit 2), not by `HseNotGenerated`. `HseNotGenerated` fires only for an HSE tree other than H1, and Sentinel-03 solves to H1 |
| P1.2 | With the derived `foreign_clock`, the run is accepted. `judge_foreign` judges **nothing**: every clock is in `not_judged` as "the tree is PLL1 (PLLSRC 10), whose deviation is not declared". There is no `foreign_clock_contradicted`, because the decode gives 400 MHz sys_ck from the 16 MHz HSE |
| P1.3 | A committed `foreign_clock` in `examples/sentinel-03.json` is refused on every generated-firmware path (`ForeignClockOnGeneratedRun`), so `verify` and the bench package for Sentinel-03 would fail. Committing it is a decision for the stop: a separate foreign specification, or the refusal lifted |
| P1.4 | Among the committed specs, the only HSE trees are H1. No committed board other than Sentinel-03 runs `--foreign` with an HSE |

## P2. The ELF in CI

| # | prediction |
|---|---|
| P2.1 | Two local builds of the ELF are byte-identical to each other **and** to `4400j`'s `97afa171…` (same image, hwdef and build script). CI's `ardupilot-image` matrix names no Sentinel-03 |
| P2.2 | The build takes 3–8 min on Furnace, at 6 cores |

## P3. `MOT_PWM_TYPE` (the armed run, `arm12`, 12 s, CLI path, unpatched)

| # | prediction |
|---|---|
| P3.1 | **Pass, armed** (`base_mode` 209), at 8.70–8.80 s, as `4400j` measured. Peak 540–600 MB (343–381‰). Wall 35–55 s at 4 pinned cores, load under 10 |
| P3.2 | TIM1 runs 400 Hz PWM (`ARR` 0x9C3, `PSC` 0xC7), `DIER` 0, and there is no DMAMUX1 write of request 15. ArduPilot runs PWM on a board declared `dshot600` |
| P3.3 | The verdict names no motor finding: the foreign path drops motor outputs ("motor outputs … dropped"), so PWM-on-a-DShot-board passes unremarked. There is no `DSHOT_STOP_RESIDUAL` and no `stops`: the end-of-window stop is generated-firmware only |

## P4. DShot over MAVLink (`dshot12`, 12 s)

| # | prediction |
|---|---|
| P4.1 | `--input` accepts `PARAM_SET MOT_PWM_TYPE`. No input rule refuses it |
| P4.2 | DShot engages at the first motor init after `MOT_PWM_TYPE` 6 and `FRAME_CLASS` 1 are in force: `DIER` `UDE`, `PSC` 0x29, `ARR` 7, `DCR` 0x30D, and DMAMUX1 channel 9 = 15, re-armed once per frame |
| P4.3 | **The out-of-memory death survives the stdin fix** (p = 0.7). It is a per-frame re-arm cost in Renode's DMA path, not `4400j`'s console queue. On main's CLI the run is killed `memory_cap_exceeded` 0.3–1.2 s of virtual time after DShot engages, before the arm. The verdict is `fail`, naming the bound, and no DShot bit leaves the twin |
| P4.4 | If it does not die (p = 0.3), RSS stays within 15 % of the `arm12` run, the arm happens at about the same time, and nothing judges the motors |
| P4.5 | With the cap lifted (patched, 8 GiB container), RSS grows at least 1 GiB per virtual second from DShot's start. The dump's growth is dominated (over 60 %) by one Renode DMA or timer type, not ours |

## P5. DMA per timer (ungraded: written after the report)

| # | prediction |
|---|---|
| P5.1 | RM0433 Rev 9 Table 124 (p. 697) gives TIM1–5, TIM8 and TIM15–17 their requests; TIM6 and TIM7 UP only; **TIM12, TIM13 and TIM14 none.** The solver's `TIMERS` offers 12–14 for DShot anyway |
| P5.2 | No committed DShot board sits on TIM12–14. TIM4 has UP (32) but no CH4 request |

## P6. The judge's end-of-window stop

| # | prediction |
|---|---|
| P6.1 | On the foreign path there is no stop: the timer runs to the window's end. The row's risk is not met there, because nothing judges the foreign motors |
| P6.2 | On the generated path the stop stands as `2302` D2 built it. Every DShot board's generated verdict names `DSHOT_STOP_RESIDUAL` |

## P7. The reboot (`reboot12`)

| # | prediction |
|---|---|
| P7.1 | Today (CLI, unpatched), `MAV_CMD_PREFLIGHT_REBOOT_SHUTDOWN` is accepted by the input reader. ArduPilot writes `SCB_AIRCR` `SYSRESETREQ`, and Renode logs "No action for reset … machine_0.reset is not registered". The core halts or spins, and the verdict is **not** a reboot. Either it passes on what came before (the end reads find a stopped firmware), or a witness after the reset is missing |
| P7.2 | Set, reboot, second boot and read-back fit in 12 s: reboot at about 7 s, second boot `ArduPilot Ready` about 5.2 s later (about 12.2 s). **12 s does not hold a second boot to Ready.** A reboot row needs the window or an earlier reboot (about 5.5 s), and reads parameters at the second boot's first `PARAM_VALUE` (about 1–2 s after reset), not at Ready |
| P7.3 | With a reset macro (patched: `machine Reset`, then the CLI's boot lines), the second boot runs out of memory under the 1.5 GiB cap. With the stdin fix the death moves later or disappears (p = 0.5 each way) |
| P7.4 | Renode's `machine Reset` resets RAM and flash as `MappedMemory`: **parameters in flash pages 14/15 are lost.** A reboot by the macro alone is "a reboot that isn't one". The RTC backup registers are reset too, where silicon keeps them on a system reset. `RCC_RSR` reads its power-on value, not `SFTRSTF` |
| P7.5 | The dump (if it dies): the growth is not the stdin queue. It is a second machine's worth of something, such as peripherals or memories re-created per reset, or the ELF's symbol lookup loaded again |

## P8. Triage (ungraded: written after the report)

| # | prediction |
|---|---|
| P8.1 | R2, R3 (bidirectional half), R4, R5 and R6: after P4. R7 and R8: closed. The RC baud row: closed, 416666 inside the declared tolerance with margin |
| P8.2 | None of R2–R8 needs slice B |
