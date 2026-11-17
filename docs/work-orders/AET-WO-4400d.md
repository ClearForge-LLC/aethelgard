# AET-WO-4400d — The IMU's orientation, declared once: kept by the placer, emitted as ArduPilot's `IMU` line, judged by the identity judge

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff, **after `AET-WO-1313` merges** (specification 15).
**Branch:** `wo/aet-4400d-imu-orientation`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4 (exit clause 3; the flight path `4400b` → `4400c` → flight loop). Written 2026-11-17 from `AET-WO-4400c`'s (h) (PR #202, ruling comment 6944720041, D1 (a)). **Cadence: build, with one measured stop (§1.1).**

**Grounds:**
- **`4400c`'s (h), measured, with two witnesses.** The generated `hwdef.dat` for `sentinel-02-own` holds `SPIDEV icm42688 …` but no `IMU` line. Its header says why: "its rotation is a mechanical fact no specification declares".
  - Unmodified ArduPilot therefore compiles no IMU driver: no `Invensensev3` symbol is in the ELF.
  - At 5.2055 s it ends in `INS: unable to initialise driver`, then `config_error`.
  - The ICM model sees no transaction in 15 s.
- **The ruling:** the orientation is declared in the specification, never a generator constant (`INV-06`). Until it is declared, `generate hwdef` refuses an IMU part by name instead of writing a board that cannot initialise.
- **`INV-01`.** One source and two projections: the same declared fact decides the footprint's rotation on the board and the `ROTATION_…` in the firmware. If the two projections disagree, the drone flies with its axes wrong.

> **What this is:** an IMU device declares its orientation against the board's declared forward direction. The placer rotates the IMU's footprint so that the two agree, or refuses by name. `generate hwdef` emits `IMU <driver> SPI:<dev> ROTATION_…` from them. `check pinout`'s identity judge holds the `IMU` line against the topology as a sixth fact.
>
> It is NOT:
> - a compass or a second IMU (later);
> - the foreign-firmware judge (after this order and `4400c2`);
> - board mounting in a frame beyond the declared forward direction.

## 1. Build

1. **Measure first, then STOP.** Read at source and cite:
   - ArduPilot's rotation enum and its axis convention at the pinned tag (`AP_Math/rotations.h`), and how `IMU … ROTATION_x` composes with the board's own orientation (`AHRS_ORIENTATION`);
   - each IMU part's axes against its footprint, from the datasheet's orientation figure (the ICM-42688-P first), and where that fact lives in `parts.json`;
   - how the placer chooses a footprint's rotation today (`pcb.rs`), and how `1313`'s side-edge rotations interact with it.

   **STOP** with:
   - the proposed specification shape: the board's forward direction (which edge), and the IMU's orientation (an enum of the rotations ArduPilot can express, or a pair of axes);
   - the specification version (16);
   - the solved-schema field, if any, that carries the IMU's rotation to `generate hwdef` without reading the specification (`RUL-16`);
   - every board and golden whose bytes would move.
2. **After the ruling:**
   - **`aethelgard-ir`:** the fields, validated (`INV-06`: an IMU part with no declared orientation is refused by `generate hwdef`, by name). The part's cited axes come from the parts data.
   - **`aethelgard-eda`:** the placer keeps the IMU's footprint at the rotation that makes its axes agree with the declared orientation, or refuses as `ImuRotationUnplaceable`.
   - **`aethelgard-hwdef`:** emits the `IMU` line, computed from the solved fact. The header no longer names the IMU as "not emitted".
   - **`aethelgard-pincheck`:** the identity judge reads `HAL_INS_PROBE_LIST` as the compiler expands it, as `4400b2` reads the other lists, and judges the driver, the SPI device and the rotation against the topology. It shares no code with `aethelgard-hwdef` (`RUL-20`).
   - **CI:** `sentinel-02-own`'s generated board builds with the `IMU` line, and its ELF holds the `Invensensev3` driver.

## 2. Acceptance

- Generated `sentinel-02-own` builds unmodified ArduPilot with an `IMU` line. The ELF carries the driver, and the identity judge passes.
- **Mutations, each refused by name:**
  - the `IMU` line removed;
  - the wrong rotation;
  - the wrong SPI device;
  - an IMU part with no declared orientation.
- **The placed footprint's rotation agrees with the declared orientation.** A tier-1 test holds the emitted board's rotation against the specification through the part's cited axes.
- **The CLAUDE.md gates pass.** CI is green, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-06`, `INV-07`; `RUL-16`, `RUL-20`.
- **Fence:**
  - `aethelgard-ir` (the fields, validation, parts data);
  - `aethelgard-eda` (the placer's IMU rotation);
  - `aethelgard-hwdef`;
  - `aethelgard-pincheck`;
  - the fixtures and goldens that move;
  - `.github/workflows/ardupilot-image.yml` if a step changes;
  - FEEDBACK.
- **Running beside you:** coordinate through the bus:
  - `1313b` (a placement spike);
  - `4400c2` (the twin);
  - `3402` (Sentinel-01).
- One PR, left unmerged. Write `~/.cache/aet-4400d/pr-title-body.md`.

## 5. Adversarial pass, full

- **The board and the firmware disagree:** a footprint rotated by hand after placement, or a `hwdef.h` edited, each refused.
- **A rotation ArduPilot cannot express:** refused at `validate`, never rounded.
- **An IMU whose datasheet axes are not cited:** refused, never assumed.

## 7. Flag-and-stop conditions

- **§1.1.**
- **The datasheet's axis figure is ambiguous about pin 1 or the view side:** stop, with the figure cited.
