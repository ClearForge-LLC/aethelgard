# AET-WO-1211c — predictions, committed before any run

Written 2026-11-22 on `wo/aet-1211c-pin-devices-twin` at `main` `7fc81640`, before any measurement in this directory ran. Each is judged in FEEDBACK §6. Kit: `kit/`. Image: `container/IMAGE` `e5fc7ddb…`, the pin `1211` §4 measured on.

## M1 · `1211` §4's G and T rows, re-run on `main`'s Renode pin (`kit/twin-rerun.sh`)

- **P1a:** the image is the one `1211` ran (`e5fc7ddb…`), so every value line of G0–G6, T1–T6, O1–O2, H1–H2 and A0–A6 equals `spike/1211/twin/twin.log` line for line (timestamps, seeds and log interleaving aside).
- **P1b:** Renode's version line is `1.17.0+20260907gitf1dd1b4af.aet1605`, as in 1211.

## M2 · D1 and D2 under `simulate`, today (`kit/today.sh`)

- **P2a, D1 as committed** (`crates/aethelgard-ir/tests/fixtures/d1-drv8876.json`): `validate`, `solve` and `generate` exit 0; the generated tree holds **no** `.repl` (`1211b` S2); `build` exits 0; `simulate --firmware --spec` exits 2 with kind `PinDeviceNotSimulated`, its message naming `U2` (`DRV8876PWPR`), before any tool runs.
- **P2b, D2 as committed:** `validate` exits 2, `PartLimitUndeclared` (`U2`, `input_pwm_max_hz`); `solve`, `generate` and so `simulate` stop on the same kind before any twin step.
- **P2c, D2g** (D2 with each bridge input a GPIO output that starts low and the board drawn, built exactly as `1211d`'s `pin_devices_drawn_tier1.rs::d2_drawn` builds it): `validate`, `solve`, `generate` and `build` exit 0; `simulate` refuses `PinDeviceNotSimulated` (`U2`, `DRV8833PWPR`).
- **P2d:** `verify` on D1 as committed refuses before any gate, because its board is not drawn (`draws_a_board`, `1210` §1.4); it is not a `PinDeviceNotSimulated`.

## M3 · The motor rail's coverage (`kit/today.sh`, `kit/coverage-d1.sh`)

- **P3a, D2g with one baseline scenario, `verify --fab-out`:** refused `CampaignCoverageShort` before any gate. Its unmet list is exactly: serial port `CMD`, rail `VDD_3V3`, and rail `VMOTOR` with a reason from `campaign::rendered` (no consumer the campaign renders on it). No GPIO, ADC or bridge appears: no such target exists yet.
- **P3b, D1:** the same three unmet. D1 cannot reach `verify`'s coverage today (P2d), so it is measured with a copy of `main`'s CLI whose `draws_a_board` check alone is removed (the copy is scratch, never committed; the one-line edit is in `kit/coverage-d1.sh`).
- **P3c, a droop on `VMOTOR`** (D2g, a scenario modelled on `sentinel-03/droop-bor3.json` with `domain: VMOTOR` and no `bor_level`): refused by name before any run, since no rail but the MCU's supply is rendered as a droop. **A `rail_out_of_band` on `VMOTOR`:** refused `RailOutOfBandRefused` (nothing on it the campaign renders).
- **P3d:** no committed specification declares a second, sourceless domain except D1 and D2 (and their spike copies), so the motor rail's ruling moves no committed verdict.

## M4 · D1's own firmware on the stock platform (`kit/d1-stock.sh`, a hand-written script; no twin code)

The ELF from M2 boots on `platforms/cpus/stm32h753.repl` for 100 ms of virtual time, with the GPIO and ADC3 access logs on, one channel's voltage set before the run and nFAULT driven low:
- **P4a:** the boot word (the first word of RAM) reads `BOOT_MAGIC` `0x2100B007`. Renode's ADC3 completes calibration and `ADRDY` at once (A1), so no `AdcFault` word (`0x2100B0A1`–`A3`) is written.
- **P4b:** the access log shows PA1's `BSRR` write `0x00020000` (BR1) **before** the `GPIOA_MODER` write that makes PA1 an output.
- **P4c:** at the end, `GPIOA_MODER` holds PA11 at `00`; `GPIOA_PUPDR` holds PA11 at `00` (D1's pull is the external R3).
- **P4d:** PA8 and PA9 (TIM1 CH1/CH2, `DRV_EN`/`DRV_PH`, idle low, CCR 0) read 0 at every sampled instant after `CEN`: zero edges, at level 0 (T1–T2 on real firmware).
- **P4e:** ADC3's `DR` after the bring-up's conversion, with 1.5 V set on INP5, is **not** the 12-bit code 1861 (round(1.5/3.3 × 4095)): `1211b` found Renode reads `RES` as two bits at bit 2, so `010` reads as another resolution. Which code it is, I do not predict.
- **P4f:** nFAULT held low from before the run changes nothing the firmware does (it never reads PA11): the boot word is still `BOOT_MAGIC`.

## The motor rail (the stop)

- **P5:** the measurement shows `VMOTOR` unmet on both boards with no fault kind able to reach it, and no consumer of it that the twin models. The lean I expect to write: **exclude a sourceless domain whose only consumers are pin-device supply pins** from the `Rail` targets, named in the verdict with its reason, rather than render a droop nobody models a response to. Not decided here: it is the architect's.

## Seen before this file was written (orientation, not measurement)

- `validate` on D1 and D2 as committed, and `solve` on D1, with `main`'s host CLI: D1 valid, D2 `PartLimitUndeclared`; D1's pins (PA1 nSLEEP, PA11 nFAULT, PF3 ADC3 INP5, PA8/PA9 TIM1, PA0 TIM2). P2b and the pins in M4 rest on that, not on a prediction.
