# AET-WO-1211a — predictions, written before any run

Written 2026-11-20 on `main` `824c3ff3`, before the baseline run and before any line of the build.
This file has exactly one commit. FEEDBACK puts each result beside its prediction.

**The measurement** (`measure/run.sh`, `measure/compare.py`): `validate`, `solve`, `emit` and
`generate` on every committed specification (29 files: a tracked `.json` outside `spike/` with
`schema_version`, `mcu`, `motor_outputs` and `project_name`), once with `main`'s binary on the files
as committed (the baseline) and once with the branch's binary on the files as migrated: the next
specification version, and `pwm.idle` = `{"pulse_us": pulse_min_us}` and `pwm.resolution_ps` =
1,000,000 on each of the 8 PWM outputs (in `driver-judges-nothing.json` and `motors-pwm.json`).

- **M1.** Every file `main` accepts, the branch accepts; every file `main` refuses, the branch
  refuses with the same kind. Exit codes agree on all four commands for all 29 files.
- **M2.** `solve`: no solved word moves. Every `assignments` entry, every `PSC`, `ARR` and `CCR`,
  every serial, SPI, I2C, clock and power word is equal. What differs is exactly:
  - `schema_version` (13 → 14) and `specification_sha256`;
  - `gpio`, each `{net: pin}` becoming `{net: {pin, direction: output, initial: high}}` (a chip
    select, which the generator drives high today), with the same pin;
  - each PWM output's copied `pwm` gaining `idle` and `resolution_ps`;
  - a new `adc_inputs` map: empty on 27 files; on the 2 with a `power.sense` (`sentinel-03.json`,
    `sentinel-02-power.json`) one entry each, on the pin `power.sense` already solves, with peripheral
    `ADC3` where that pin is `PF3` (ADC3-only). `power.sense` itself does not move.
- **M3.** `emit`: every file byte-identical (the spike found no specification hash in the emitted
  schematic, board or BOM).
- **M4.** `generate`: every file byte-identical except the lines that name the specification's
  sha256 (the spike's 8-file list); no generated file names the solved version.
- **M5.** The 4600 driver (`spike/4600/driver.json`), migrated in the spike with its `HB_IN1`/`HB_IN2`
  pair declared as a bridge of a pin device whose inputs idle at `pulse_us` = `pulse_min_us`, is
  refused `BridgeIdlePulsed`.
- **M6.** D1 (DRV8876, PH/EN) validates and solves; D2 (DRV8833) is refused `PartLimitUndeclared`
  (no maximum input PWM rate in SLVSAR1E).
