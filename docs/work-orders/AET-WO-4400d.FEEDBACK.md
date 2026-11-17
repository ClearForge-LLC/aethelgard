# AET-WO-4400d — FEEDBACK

Builder: Claude Code (Opus 5.5), branch `wo/aet-4400d-imu-orientation`, cut from `origin/main` 9d91ea4a (2026-11-17).
The work order is at `~/.cache/aet-4400d-wo.md` until docs PR #206 lands it on main.

## 0. Predictions (written before any measurement; every one inferred)

| # | Question | Prediction |
|---|---|---|
| P1 | ArduPilot's rotation enum (`AP_Math/rotations.h`) | `enum Rotation : uint8_t`, `ROTATION_NONE = 0` up to roughly 45 named rotations (yaw in 45° steps, roll 180 and its yaws, pitch 90/180/270 and so on), plus custom values near 100. Each is a fixed rotation of the body frame (NED: x forward, y right, z down), and positive yaw is clockwise seen from above |
| P2 | What `IMU <driver> SPI:<dev> ROTATION_x` makes | `chibios_hwdef.py` writes `HAL_INS_PROBE_LIST` as `ADD_BACKEND(AP_InertialSensor_Invensensev3::probe(*this,hal.spi->get_device_ptr("<dev>"),ROTATION_x))`. The driver applies the rotation to each sample, so `ROTATION_x` maps the sensor's frame (as the driver reads it) to the board's frame |
| P3 | How it composes with `AHRS_ORIENTATION` | `AHRS_ORIENTATION` is a second rotation, applied after the per-IMU one, from the board frame to the vehicle frame. Its default `0` means the board's forward is the vehicle's forward. The `IMU` line is board-relative, so this order owns only the per-IMU rotation |
| P4 | The driver's axis remap | Invensensev3 reads the chip's X, Y, Z and remaps them to the board frame before applying the rotation (as the older Invensense driver does: x and y swapped, z negated). So `ROTATION_NONE` does **not** mean "chip +X forward" |
| P5 | The ICM-42688-P's axes against its footprint | The datasheet's orientation figure is a top view, with pin 1 marked: +X along one package edge, +Y along the other, +Z out of the top face (right-handed) |
| P6 | Where the IMU's axes live in `parts.json` | Nowhere today; a new cited column |
| P7 | How the placer picks a rotation today | Most footprints at 0°. Connectors are rotated toward their board edge, the side-edge rotations `1313` added. No rule names an IMU |
| P8 | Proposed shape | `board.forward`: one edge of four. Per IMU device: an orientation restricted to what a top-side footprint can reach, the four yaws a multiple of 90° apart. Bottom side (the `ROLL_180` family) is refused until the placer places on `B.Cu` |
| P9 | Solved field | Yes: `SolvedTopology` carries each IMU's ArduPilot rotation (or the rotation in degrees), so `generate hwdef` and the judge never read the specification (`RUL-16`). It is a solved-schema bump |
| P10 | Moved bytes | Every specification fixture and example (the version line), the hwdef goldens' version line, and `sentinel-02`'s hwdef golden (a new `IMU` line). A `.kicad_pcb` moves only if the IMU's placed rotation changes. The Sentinel-01 bench package moves only through its specification version |
