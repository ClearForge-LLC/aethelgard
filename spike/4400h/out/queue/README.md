# The armed queue, in Tviburi's format

**Replay, outside the CLI's bounds** (ruling 2026-11-24T15:50Z). Each file is `TVI-WO-1150` (v)'s `queue-ap.tsv` format, made by `sh/queue.sh` from Tviburi's own readers, unchanged, at Tviburi main `d22dad8` (`../tviburi-readers.sha256`). Tviburi's columns are kept; there is no header, as in (v).

| file | ELF (sha256) | board | window | run |
|---|---|---|---|---|
| `queue-ap-unarmed-sentinel-02-own.tsv` | `5eace4ca7de47819…` (CI run 45410032458) | `sentinel-02-own` (main `29b3a97d`); no motor output | 13.000 s virtual | `s02-none-s`: no injection |
| `queue-ap-armed-sentinel-02-own.tsv` | the same | the same | 13.000 s; armed at 9.0 s | `s02-arm-s` |
| `queue-ap-unarmed-sentinel-03.tsv` | `dbf598d8b77f57d1…` (local build, below) | Sentinel-03, derived (below); TIM1 CH1–4 motors | 13.000 s | `s03-none-s`: no injection |
| `queue-ap-armed-sentinel-03.tsv` | the same | the same | 13.000 s; armed at 9.0 s | `s03-arm-s` |

- **Every run is the CLI's own Renode invocation, captured and replayed** (`sh/capture.sh`, `sh/replay.sh`; argv in `../inj/<run>/argv.txt`). It adds:
  - `ReceiveDmaRequest -> dmamux1@41` on `usart1` (`../../patches/usart1-rx-request.diff`);
  - `sysbus LogAllPeripheralsAccess true`;
  - Tviburi 1100's `regmap.py`;
  - the plan's frames and stimuli.

  It ran under `~/bin/capped-run --mem 4G`, one container at a time.
- **The stimuli, in every run alike:**
  - the IMU at rest with z up (`AccelerationX/Y/Z` 0, 0, 1 g), overriding the parts table's witness stimulus `acceleration_y` 1 g;
  - the BMP581's `Pressure` alternating by 0.5 Pa every 0.5 s from 6 s.

  The plans are in `../../py/plans.py`. The injected bytes are in `../inj/<run>/injected.tsv`.
- **The arm:**
  - parameters: `COMPASS_USE` 0, `FRAME_CLASS` 1, `FS_THR_ENABLE` 0, `DISARM_DELAY` 0, `INS_ACCOFFS_X` 0.001;
  - `MAV_CMD_PREFLIGHT_CALIBRATION` param5 = 76;
  - `MAV_CMD_COMPONENT_ARM_DISARM` 1 with **no force and no `ARMING_SKIPCHK` bit**;
  - then GUIDED (refused), `NAV_TAKEOFF` (refused), GUIDED_NOGPS, and `SET_ATTITUDE_TARGET` thrust 0.6.
- **Sentinel-03's ELF:**
  - **Spec:** `crates/aethelgard-cli/tests/fixtures/sentinel-03.json` (main, sha256 `c85cec0d…`), with three declarations copied from `sentinel-02-own` by `../../py/derive.py`: `mcu.foreign_clock`, U5's orientation and `board.forward`. The result is sha256 `135c96fa…`. `AET-WO-4300k`'s `examples/sentinel-03.json` (`e82feda9`, schema 30) declares the same orientation and forward.
  - **hwdef.dat:** `2b5a8fb24235f3db…`.
  - **Build:** unmodified Copter-4.7.2 `fb773567`, built by CI's own `container/ardupilot-build.sh boards sentinel-03` in the pinned ArduPilot image (`container/ARDUPILOT-IMAGE`), on 6 cores (`../../sh/s03.sh`). That gives `arducopter.elf` `dbf598d8b77f57d1945315b9fff611848f7d7f57992443a33eec0790a511d451` and `hwdef.h` `aa4f4e09…`. No ELF is committed. `check pinout` passes it against its topology (exit 0, ok, no failing finding), so `AET-WO-4406`'s `ParamDefaultsOverride` guard finds no defaults route (`../s03/pinout.json`). The derived spec, the hwdef, the hashes and the build line are in `../s03/`.

**How to read the pairs:**
- `s0N-delta.tsv` holds every register whose firmware writes differ between the arm and the same frames without it (`params-s`).
- `s0N-injection.tsv` holds the zero frames against no injection: the injection's own traffic.

Both come from `../../py/delta.py`.
