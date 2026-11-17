# AET-WO-4400d — FEEDBACK

Builder: Claude Code (Opus 5.5), branch `wo/aet-4400d-imu-orientation`, cut from `origin/main` 9d91ea4a (2026-11-17).
The work order is `docs/work-orders/AET-WO-4400d.md` (docs #206, 60946a02; identical to the copy read at kickoff).

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

## 1. §1.1, measured at source

Labels: **read at source** (cited file and line), **computed** (from source text by a script whose method is given), **inferred**.
ArduPilot is read in the pinned image (`container/ARDUPILOT-IMAGE`, `Copter-4.7.2`, `fb773567`), extracted under `capped-run --mem 2G --network none`.

### 1.1 ArduPilot: what `IMU … ROTATION_x` means

- **The `hwdef.dat` line** (read at source, `libraries/AP_HAL/hwdef/scripts/hwdef.py` L382–425, L325–328): `IMU <driver> SPI:<dev> <rotation>` becomes
  `#define HAL_INS_PROBE<n> ADD_BACKEND(AP_InertialSensor_<driver>::probe(*this,hal.spi->get_device("<dev>"),<rotation>))`.
  Then `#define HAL_INS_PROBE_LIST HAL_INS_PROBE1;…`, joined by `;`. A duplicate `IMU` line is an error. With fewer than three IMUs it also writes the defaulting define `INS_MAX_INSTANCES`.
- **The driver's frame:** Invensensev3 assembles each sample from the chip's registers in order, with no swap and no sign: `Vector3f accel{d.accel[0], d.accel[1], d.accel[2]}` (`AP_InertialSensor_Invensensev3.cpp` L463–464; high-resolution FIFO L522–527). `accel_scale` is positive (`…Invensensev3.h` L120 `ACCEL_SCALE_32G = GRAVITY_MSS / (SCALE_RANGE_16BIT / 32)`). So the vector the rotation acts on is the **chip's own X, Y, Z**. Read at source. **P4 failed:** I predicted the older Invensense driver's swap.
- **The order of rotations** (read at source):
  - `_rotate_and_correct_accel` first applies `rotate(_accel_orientation[instance])`, the `IMU` line's rotation set at L379–380 (`AP_InertialSensor_Backend.cpp` L113).
  - Then it applies the offsets and scale.
  - Then it applies `rotate(_imu._board_orientation)`, which is `AHRS_ORIENTATION` (`AP_AHRS.cpp` L148, default 0) (L144; gyro L150, L172).
  - So `v_vehicle = R(AHRS_ORIENTATION) · R(IMU line) · v_chip`. The `IMU` line maps the chip frame to the **board** frame; `AHRS_ORIENTATION` maps the board to the vehicle. **P3 held.**
- **The frame:** body NED, x forward, y right, z down. Level at rest, the accelerometer reads `(0, 0, −g)` (`AP_InertialSensor.cpp` L2546, `rotated_gravity(0, 0, -GRAVITY_MSS)`). Read at source.
- **The enum** (`AP_Math/rotations.h` L27–83): values 0–43 (`ROTATION_PITCH_7` = 41 is in the enum but not in the `@Values` list, L93), then `ROTATION_MAX`, then the custom values 100–102. Values are stored to EEPROM (L25–26) and must match `MAV_SENSOR_ORIENTATION` (L72–77). **P1 held.**
- **Every rotation as a matrix** (computed). A script evaluates each `case` of `Vector3<T>::rotate` (`vector3.cpp` L28–…) on the three basis vectors, from the source text itself (`HALF_SQRT_2` substituted, casts dropped).
  - 49 cases were evaluated; 26 names are axis-aligned; those 26 give **24 distinct matrices**, the whole cube group. The two duplicates are the aliases the source comments on: 26 = 14 and 27 = 10 (L54–55).
  - **Every orientation whose axes are along the board's is therefore expressible in ArduPilot.** Only a non-right-angle or malformed declaration has no name.
  - Spot check against the source: `ROTATION_YAW_90` gives `x' = −y, y' = x` (L41–43), the matrix `((0,−1,0),(1,0,0),(0,0,1))`.
- **The census** (read at source, in the image): the `IMU Invensensev3` lines in the tag's `hwdef/*/hwdef.dat`, by last word, are:

  | Rotation | Lines |
  |---|---|
  | `ROTATION_NONE` | 28 + 9 |
  | `ROLL_180` | 23 |
  | `PITCH_180_YAW_270` | 23 |
  | `PITCH_180` | 22 |
  | `ROLL_180_YAW_90` | 20 |
  | `ROLL_180_YAW_270` | 20 |
  | `YAW_180` | 17 |
  | `PITCH_180_YAW_90` | 15 |
  | `YAW_270` | 13 |
  | other | fewer |

  The five z-flipped rows lead. That is what §1.2 predicts for a part on the top side (inferred: the census does not record which face each part sits on).

### 1.2 The ICM-42688-P's axes against its footprint

The source is DS-000347 rev 1.9, `~/.cache/aet-ds/aet-4220/ds/imu/icm42688p_ds.pdf`, sha256 `9c6b57b0a2e45cf0251dd2423878573bd4da46e0cc2faa93dc558c28af51300a`. Pages 53 and 54 were rendered at 200 dpi and read.

- **§10.1 Fig. 16, p.53, "Orientation of Axes of Sensitivity and Polarity of Rotation":** "Note the pin 1 identifier (•) in the figure." The drawing is isometric, with the dot on the marked top face at one corner and +Z out of that face.
  - +X is drawn parallel to the edge that leaves the dotted corner on the right.
  - +Y is drawn parallel to the other edge, pointing **into** the dotted corner.
- **§10.2, p.54, the package drawing, captioned "(TOP VIEW)":**
  - The A1 corner index (solid dot) is at the top-left, and pin 1 heads the left column.
  - Pins 14 and 12 are on the top edge, 5 and 7 on the bottom, 8 and 11 on the right.
  - E (3.0 mm) is horizontal. The bottom view is drawn mirrored, with pin 1 at the bottom-left.
- **Reading** (read at source, plus one geometric step):
  - Seen from the top with pin 1 at the top-left, the two edges at the pin-1 corner run right (the 14…12 edge) and down (the 1…4 edge).
  - Fig. 16 puts +X leaving the corner along one edge and +Y arriving along the other, with +Z out of the face.
  - Right-handedness admits only **+X toward pins 8–11 (right) and +Y toward pins 12–14 (up)**. The other assignment, X down and Y left, gives Z into the face.
  - This does not rest on the isometric's edge lengths, which are drawn near-square.
- **The footprint** (read at source, `crates/aethelgard-kicad-lib/data/aethelgard/footprints/Aethelgard_Sensor.pretty/TDK_LGA-14_3x2.5mm_P0.5mm.kicad_mod`):
  - Pin 1 is at (−1.1625, −0.75) and pins 8–11 at x = +1.1625. Pins 12–14 are at y = −0.9125, that is up, since KiCad's y points down. Pins 5–7 are at y = +0.9125.
  - The pads run counter-clockwise 1…14 as seen from F.Cu, so this is the top view.
  - **At rotation 0: chip +X is the board file's +x (right), +Y is its −y (up), and +Z points out of F.Cu.**
- **Not in the figure (inferred):** the sign of the accelerometer's output, that is, that resting with +Z up reads +1 g on Z. That is the MEMS convention, and the census agrees (the z-flipped rotations lead). No sentence of the datasheet I could find states it.
- **The §7 condition** (pin 1 or the view side ambiguous). I judge that it does not trip: p.54 is captioned "TOP VIEW", and Fig. 16's dot and +Z fix the face and the corner. The architect rules.
- **In `parts.json` today:** nowhere. The ICM42688P row (`crates/aethelgard-ir/data/parts.json` L415–589) carries its bus, identity, supplies, pin capacitance and `ardupilot` row (kind `imu`, driver `Invensensev3`, spidev `icm42688`), and no axes. **P6 held.** It is the only IMU part (computed over the `ardupilot` rows: BME280, BMP581, ICM42688P, W25Q64JVxIQ).

### 1.3 The placer today, and 1313's side edges

Read at source; a subagent's report, spot-checked at `pcb.rs` L1005–1033 and the footprint's zones.

- **Every footprint sits on F.Cu.** `footprint()` writes `(layer "F.Cu")` and there is no mirror path (`pcb.rs` L23–29, ~L990). Rotations are 0, 90, 180 or 270 only, counter-clockwise on the board as KiCad's file holds them, with y down (`route/read.rs` L42–66, `rotate`: 90 maps `(x, y)` to `(y, −x)`; pinned by `classes_tier1.rs` `rotation_is_kicads_as_measured`).
- **What is turned:**
  - ICs, the MCU, passives and bench parts are placed at 0 (`pcb.rs` L595, L675, L895).
  - Only anchored parts are searched over the four angles (`place_anchored`, about L760–823, least squared pad distance, ties to the smaller angle).
  - A `Move` can override, but only tests pass one (`schematic::emit_moved`, L596).
  - **The ICM-42688-P (U5) is an ordinary IC at 0. No rule names an IMU.**
- **A footprint with a zone cannot be turned:** `pcb.rs` L1018–1027, labelled `belt-and-suspenders:`, refuses `rotation != 0` for any footprint with a `zone` ("a footprint with a keepout zone is not turned", from `AET-WO-4300d2` D2). **The ICM-42688-P footprint carries two keepout zones** (F.Cu inside the pad ring, B.Cu under the body, L221–280). Today, then, **the IMU can be placed only at 0°.**
- **1313's side edges** (`origin/wo/aet-1313-outline`, `pcb.rs` `connector_edges`, FEEDBACK L106): under a declared outline bound, connectors (`lib_id` `Connector_…`) go to the left edge at 270 or the right at 90, cable side out; without a bound they stay at the bottom edge at 0. Nothing but connectors is turned for an edge, and no IC is affected. **No direction is named anywhere in the board** (no forward, no north). "Bottom" means the edge at maximum y.
- **The route the placer would read** (1313's pattern): `schematic::emit_moved` reads `spec.board.max_outline_um` and passes it to `pcb::write_moved` (1313 `schematic.rs` L639–656). eda's only readers of the declared device lists are pinned at `schematic.rs` and `support.rs` (`cli/tests/solved_devices.rs` `the_eda_reader_is_exactly_the_schematic`). An orientation read from a declared SPI device in `schematic.rs` keeps that pin.

### 1.4 The table: footprint angle and forward edge to ArduPilot's rotation (computed)

Computed with the part on F.Cu and F.Cu facing up, so that the board frame's z points from F.Cu through to B.Cu.

The forward edge is named as the board file draws it, seen from F.Cu. Right is the forward edge turned clockwise. The chip's axes come from §1.2, turned by KiCad's `rotate`. The ArduPilot name is the one whose matrix (§1.1) equals the body-from-chip matrix.

| forward edge | 0° | 90° | 180° | 270° |
|---|---|---|---|---|
| top (−y) | `ROLL_180_YAW_90` | `ROLL_180` | `ROLL_180_YAW_270` | `PITCH_180` |
| right (+x) | `ROLL_180` | `ROLL_180_YAW_270` | `PITCH_180` | `ROLL_180_YAW_90` |
| bottom (+y) | `ROLL_180_YAW_270` | `PITCH_180` | `ROLL_180_YAW_90` | `ROLL_180` |
| left (−x) | `PITCH_180` | `ROLL_180_YAW_90` | `ROLL_180` | `ROLL_180_YAW_270` |

Two of the four matrices have two names each: `ROLL_180_YAW_90` = `PITCH_180_YAW_270` (27), and `ROLL_180_YAW_270` = `PITCH_180_YAW_90` (26).

What follows from the table:
- **A part on the top side reaches exactly four rotations, all z-flipped.** **P8 failed:** I predicted "the four yaws", which need the chip upside down, on B.Cu.
- **Hand check:** forward = top at 0°. Chip X is the board's right (body +y), chip Y is forward (body +x), and chip Z is up (body −z). So `v_body = (Y, X, −Z)`, which is `ROLL_180_YAW_90` (`vector3.cpp` L80–82: `x = y; y = x; z = −z`).

## 2. STOP (§1.1): the proposal

### 2.1 The specification shape (version 16, on 1313's 15)

```json
"board": { …, "forward": "top" },
"spi_devices": [ { "reference": "U5", "part": "ICM42688P", …, "orientation": { "x": "right", "z": "up" } } ]
```

- **`board.forward`:** the edge the vehicle's nose points to, as the board file draws it seen from F.Cu: `top` (−y), `right`, `bottom` or `left`.
  - It is optional, under the precedent of 1313's `max_outline_um` and #191 Q1: absence is today's board, and an explicit `null` is refused.
  - It is required when any device declares an `orientation` (`ForwardUndeclared`, by name).
- **`orientation`, on an SPI device:** where the chip's own +X and +Z point in the board frame, each one of `forward`, `aft`, `right`, `left`, `up` or `down`. The pair fixes the rotation, and Y is Z × X.
  - It is optional on the device, and refused on a part whose `ardupilot` kind is not `imu` (`OrientationOnNonImu`).
  - Malformed pairs (X and Z parallel) are refused at `validate` (`ImuAxesNotPerpendicular`). That is the only case "ArduPilot cannot express" can mean in this form, because all 24 right-angle orientations have names (§1.1). Nothing is rounded, because a non-right angle cannot be written.
  - For an I2C IMU, the same field would go on `I2cDevice`. No I2C IMU part exists, so I propose SPI only.
- **Why axes and not ArduPilot's enum (recommended, D1):**
  - The enum names a rotation of the driver's frame, not of the chip. Invensensev3 passes the chip's axes raw (§1.1), but another driver of the same part might not, so the same enum value would then mean a different physical mount.
  - The axis pair is a mechanical fact, the one a person can check against the board's silkscreen. The firmware value is computed from it, as `INV-01` asks.
  - (b) The enum (`"orientation": "ROTATION_ROLL_180_YAW_90"`) is the alternative. It ties the specification to ArduPilot's vocabulary and to a driver's frame.
- **Who refuses what:**
  - `validate` refuses a malformed pair, a missing `forward`, and an orientation on a non-IMU part.
  - The placer refuses a pair a top-side footprint cannot reach (`ImuRotationUnplaceable`, naming the device and the pair): any `z` but `up`, and so any orientation on B.Cu or on edge. That is 20 of the 24.
  - `generate hwdef` refuses an IMU part with no orientation (`ImuOrientationUndeclared`, by reference, `INV-06`).
  - A specification with an undeclared IMU stays valid and its board is placed as today. The WO's §1.2 puts the refusal at `generate hwdef`. The other IMU fixtures (`imu-baro-1v8`, `sentinel-02-shaped`, `sentinel-02-power`) then need not move.

### 2.2 The solved field (`RUL-16`)

- **`SolvedSpiDevice.orientation: Option<{x, z}>`**, copied by `solve` and absent when undeclared (`skip_serializing_if`). **Solved schema 9 → 10.**
  - `generate hwdef` and the identity judge need only this, plus the driver's frame. Neither needs the forward edge or the footprint.
  - The forward edge and the part's axes stay with the placer, which reads them in `schematic.rs` as it reads 1313's bound. The pinned reader set is unchanged.
- **Two cited facts, in `parts.json`; absence of either is refused at load:**
  - **The part's axes** (for the placer), stated by pins so that they hold for any footprint of the part:
    - `"axes": {"x_toward_pins": ["8","9","10","11"], "y_toward_pins": ["12","13","14"], "z": "top", "at": {DS-000347 rev 1.9, §10.1 Fig. 16 p.53 and §10.2 p.54, sha256 above}}`.
    - The placer reads each direction from the footprint's own pad positions (pin-group centroid minus package centre) and refuses a part with no `axes` (`ImuAxesNotCited`, the WO's §5 "never assumed").
  - **The driver's frame** (for hwdef and the judge): on the `ardupilot` row of kind `imu`, `"frame": "chip"`, meaning the driver passes the chip's axes unchanged. It is cited at `AP_InertialSensor_Invensensev3.cpp` L463–464 and L522–527 at `fb773567`. It is the only value this order would accept; any other is refused until a driver needs it.
- **The judge** (`RUL-20`):
  - It reads `HAL_INS_PROBE_LIST` as the compiler expands it (4400b2's `expand`, `;`-joined) down to `ADD_BACKEND(AP_InertialSensor_<driver>::probe(*this,hal.spi->get_device("<dev>"),ROTATION_<x>))`.
  - It judges the driver and the device against the parts row of the topology's IMU device. It judges the rotation by **matrix**: its own 24-row table of name to matrix, cited at `vector3.cpp`, against the matrix of the solved `{x, z}`, so either alias passes (D4).
  - hwdef keeps its own `{x, z}` to name table and shares no code with the judge.
  - The ELF must carry `AP_InertialSensor_Invensensev3`'s symbols when the line is present (the WO's §2).

### 2.3 Decisions for the architect

- **D1. The shape:** (a) the axis pair `{x, z}` in board words, recommended; or (b) ArduPilot's enum. §2.1 gives the reasons.
- **D2. The keepout refusal**, `pcb.rs` L1018–1027, labelled `belt-and-suspenders:`:
  - (a) Turn a footprint's zone polygons by the footprint's angle, with the same `route::rotate` its pads use, so all four top-side rotations are placeable. This replaces labelled code, so it is the owner's call. It is in this order's eda fence.
  - (b) Keep the refusal: the IMU is placeable at 0° only, and the other three are `ImuRotationUnplaceable`, with the forward edge choosing among the four rotations.
  - I recommend (a). Under (b) a declaration's reachability depends on a drawing artefact, not the part.
- **D3. Which face is up.** Every row of §1.4 assumes the board is mounted F.Cu up. Nothing declares it.
  - (a) State it as the engine's constant: this order's z is "through F.Cu to B.Cu", and a board mounted otherwise is `AHRS_ORIENTATION`'s, a parameter outside the WO's fence ("board mounting in a frame beyond the declared forward direction"). I recommend (a).
  - (b) Declare it, `board.up: "F.Cu"`.
- **D4. Aliases:** hwdef writes the lower-numbered name (`ROLL_180_YAW_90`, `ROLL_180_YAW_270`), and the judge compares matrices. Alternatively, the judge compares names.
- **D5. The value for `sentinel-02-own`:**
  - `board.forward: "top"` (away from the connector row at the bottom edge) with U5 `{x: "right", z: "up"}` gives **0°, so no board bytes move**, and `IMU Invensensev3 SPI:icm42688 ROTATION_ROLL_180_YAW_90`.
  - `{x: "forward", z: "up"}` gives 90° (`ROLL_180`) and needs D2 (a).
  - Which way the board flies is a mechanical choice: the operator's or the architect's.

### 2.4 Bytes that would move (three sets)

Computed against 1313's tip, db8b210e; 1313 is not on main yet. A merge of main after 1313 lands precedes the build.

1. **Specification 15 → 16:** the set 1313's own bump moved.
   - The 25 specification JSON files at version 15: 17 under `cli/tests/fixtures`, 4 under `…/fixtures/solve`, 1 under `ir/tests/fixtures`, and 3 in `examples`.
   - Through each one's sha256: the three `aethelgard-hwdef` goldens' header line and their `topology.json`; `svd-hal/tests/ui/*` (`topology.json`, `board.rs`); and `boards/sentinel-01-bench/*` (8 generated files carry the specification's hash).
   - `fab/sentinel-01-bench/package` (`MANIFEST.json`, `FAB-NOTES.md`), written by CI's `bench-package`.
   - Inline versions in tests and READMEs: `ir/tests/{validate,serial,bench}.rs`, `svd-hal/tests/order_2203.rs`, `cli/tests/{generate_tier1,solve_cli,verify_2k2_tier2}.rs`, `cli/README.md`, `ir/README.md`.
   - **Collision:** 3402 and 4400c2 also re-bless `boards/sentinel-01-bench` and the package; whoever merges second regenerates. Noted to both.
2. **Solved 9 → 10:** five `topology.json` files and five inline copies.
   - The `topology.json` files: `hwdef/tests/fixtures/{sentinel-01,sentinel-02,serial-dshot-9600}`, `pincheck/tests/common`, `svd-hal/tests/ui/motors-pwm`.
   - The inline copies: `cli/README.md`, `cli/src/{analog,renode}.rs`, `twin/src/tests.rs`, `twin/tests/spi_platform.rs`.
   - And the package's `topology_sha256`.
3. **Content:**
   - `sentinel-02-own.json` gains `board.forward` and U5's `orientation`.
   - `hwdef/tests/fixtures/sentinel-02.topology.json` gains U5's orientation.
   - `sentinel-02.hwdef.dat` gains the `IMU` line, and its "Not emitted: U5 …" header goes.
   - `parts.json` gains ICM42688P's `axes` and its `ardupilot.frame`.
   - Boards: none at 0° (D5). At 90°, 180° or 270°, the generated `sentinel-02-own` board, which is not committed. Sentinel-01 has no IMU, so its board does not move.
   - The hand seeds (`pincheck/fixtures/*.hwdef.dat`) and `container/ardupilot-pinout.sh` have no `IMU` line, and their topologies have no IMU, so they do not move (read at source).
   - `ardupilot-image.yml` builds `sentinel-02-own` already; its step does not change (inferred until built).

## 3. Predictions against measurement

| # | Held? |
|---|---|
| P1 | held |
| P2 | held: `get_device`, not `get_device_ptr` |
| P3 | held |
| P4 | **failed**: Invensensev3 passes the chip's axes raw |
| P5 | held: +X toward pins 8–11, +Y toward 12–14, +Z out of the top |
| P6 | held |
| P7 | held in part: connectors are turned only under a bound (1313); anchored parts are also searched; and **a zone forbids turning the IMU** (not predicted) |
| P8 | **failed**: the top side reaches four z-flipped rotations, not the four yaws |
| P9 | held: `SolvedSpiDevice.orientation`, solved 10 |
| P10 | held, with additions: `boards/sentinel-01-bench/*` and the inline solved topologies |

## 4. The ruling, and what was built

**Ruled on #207** (comment 6946857533, 11:33 UTC):
- **D1 (a):** the axis pair.
- **D2 (a):** zones turn with their pads, and the labelled guard becomes a test.
- **D3 (a):** F.Cu up is the engine's constant.
- **D4:** the lower alias, and the judge compares matrices.
- **D5:** `sentinel-02-own` declares forward `top` and U5 `{x: right, z: up}`.

Built on 1313's READY tip (cc11ec68, merged before it landed; main's f845b6f is that tree plus docs #206, merged since).

- **`aethelgard-ir`.**
  - `orientation.rs`: `Edge`, `Toward`, `Orientation`.
  - `board.forward` and `SpiDevice.orientation`, each optional, with an explicit `null` refused (specification 16).
  - `SolvedSpiDevice.orientation`, copied by `solve` (solved 10).
  - `validate`, in step 13: `ImuAxesNotPerpendicular`, then `ForwardUndeclared`.
  - `validate`, in step 14: `OrientationOnNonImu`, then `ImuAxesNotCited` (for `axes` or `ardupilot.frame`).
  - `SolvedTopology::validate` refuses a copied pair not at right angles.
  - Parts schema 9 adds:
    - `axes` (`x_toward_pins`, `y_toward_pins`, `z: "top"`, cited to DS-000347 Fig. 16 and the p.54 top view), only on an `imu` row;
    - `ardupilot.frame` (`{is: "chip", at}`, cited to `Invensensev3.cpp` L463–464 and L522–527), only on an `imu` row.

    Each shape is refused at load by name.
  - The README states D3 beside the fields.
- **`aethelgard-hwdef`.**
  - It emits `IMU <driver> SPI:<name> ROTATION_<x>`.
  - The rotation comes from its own 24-row table, `ROTATIONS`, choosing the matrix whose columns are the chip's X, Y = Z × X and Z in the board frame. Of two aliases it writes the lower-numbered.
  - With an IMU line present, the header names the board frame (D3).
  - It refuses `ImuOrientationUndeclared` (`INV-06`) and `ImuFrameNotCited`.
  - The `Not emitted: U5 …` line is gone.
- **`aethelgard-pincheck`, the identity judge's sixth fact.**
  - `HAL_INS_PROBE_LIST` is read as the compiler expands it, down to `ADD_BACKEND(AP_InertialSensor_<driver>::probe(*this,hal.spi->get_device("<name>"),ROTATION_<x>))`.
  - The driver and the device are held against the parts row, and the rotation **by matrix** from the judge's own 26-row table, both aliases included (D4).
  - The expected matrix is computed from the solved `{x, z}` by the judge's own arithmetic.
  - The ELF must define the driver's `probe(AP_InertialSensor&, OwnPtr<Device>, Rotation)` under its mangled name (measured with `nm` on the built ELF, §5).
  - A rotation that is not a right-angle enumerator, a rotation redefined as a macro, or a probe in another form is refused (`MalformedHeader`).
  - Findings: `WrongImuProbe`, `WrongImuRotation`, `MissingImuProbe`, `UnexpectedImuProbe`, `ImuDriverNotLinked`.
  - `identity::judge` now takes the ELF's bytes, not its decoded serial table, because it reads a symbol too.
- **`aethelgard-eda`.**
  - `schematic::seats` is the one reading of each IMU's seat (forward, orientation, cited pins), kept in `schematic.rs` so eda's pinned readers stay `schematic.rs` and `support.rs`.
  - The placer gives a seated part the right angle that takes its footprint's pin-group axes (measured from the footprint's own pads) to the declared directions, or refuses `ImuRotationUnplaceable` (Z not up, X up or down, mirrored or diagonal pins, a pin with no pad).
  - The part's pads and geometry are carried at that angle, so its capacitors target the turned pins.
  - **D2:** footprint zones turn by `route::rotate`, as pads do. The labelled refusal is gone, and its intent is the tier-2 test `a_turned_keepout_is_where_kicad_draws_it`.
  - **§5's hand-turned footprint:** `check drc --spec` holds each declared IMU's footprint to its declaration from the board's own pads, before any tool runs (`route::declared` → `ImuOrientationDisagrees`), with arithmetic of its own: the body-frame cross product.

## 5. Acceptance (§2), measured

Built in the pinned ArduPilot image (`Copter-4.7.2`, `fb773567`) by `container/ardupilot-build.sh boards <name>`, under `capped-run --mem 8G --network none`, one container at a time. Judged by this branch's `aethelgard check pinout --topology --elf --hwdef-h`.

- **Generated `sentinel-02-own` with its `IMU` line** (`IMU Invensensev3 SPI:icm42688 ROTATION_ROLL_180_YAW_90`) builds unmodified ArduPilot in 21.9 s.
  - `hwdef.h` L140: `#define HAL_INS_PROBE1  ADD_BACKEND(AP_InertialSensor_Invensensev3::probe(*this,hal.spi->get_device("icm42688"),ROTATION_ROLL_180_YAW_90))`.
  - `hwdef.h` L144: `#define HAL_INS_PROBE_LIST HAL_INS_PROBE1`.
  - The ELF holds 30 `AP_InertialSensor_Invensensev3` symbols (`nm`), among them `_ZN30AP_InertialSensor_Invensensev35probeER17AP_InertialSensorN6AP_HAL6OwnPtrINS2_6DeviceEEE8Rotation`.
  - `check pinout`: `ok`, no finding, the identity judge included.
- **The four mutations, each refused by name.** The first three are hand edits of the generated `hwdef.dat`, each built in unmodified ArduPilot (27–29 s):

  | Mutation | Result |
  |---|---|
  | `IMU` line removed | `MissingImuProbe HAL_INS_PROBE_LIST`. Its ELF has **0** Invensensev3 symbols (4400c's (h), reproduced) |
  | `ROTATION_ROLL_180` | `WrongImuRotation HAL_INS_PROBE1` [expected `ROTATION_ROLL_180_YAW_90 or ROTATION_PITCH_180_YAW_270` \| `ROTATION_ROLL_180`] |
  | `SPI:dataflash` | `WrongImuProbe HAL_INS_PROBE1` [`Invensensev3 on SPI device icm42688` \| `… dataflash`] |
  | no declared orientation (`board.forward` and U5's `orientation` removed) | `solve` passes. `generate hwdef` refuses `ImuOrientationUndeclared` (`U5`), exit 2, and no file is written |

- **The placed footprint agrees with the declaration.** `orientation_tier1`'s `the_placed_imu_agrees_with_the_declared_orientation` reads only the emitted board: U5's `(at x y θ)`, its pads turned as KiCad turns them, and each axis from the pads' mean to the mean of the cited pins. It holds them to the declaration worked through the board frame.
  - Seven declarations are covered: forward `top` with X right, forward, left and aft (0, 90, 180 and 270°); `right`/right; `bottom`/forward; `left`/aft.
  - At each angle the F.Cu keep-out is the pad ring's turned box, centred on the origin.
  - The DRC gate's own reading (`route::declared`) passes each one.
- **The rotation tables, measured against ArduPilot's compiled code.** `Vector3<T>::rotate`'s body, verbatim from `vector3.cpp` at the tag, was compiled with host `g++` on a minimal `Vector3` beside `rotations.h` verbatim. Applied to the three basis vectors for every `Rotation` below `ROTATION_MAX`, it gives 26 right-angle names with 24 matrices.
  - hwdef's 24 rows match exactly; it leaves out only the aliases 26 and 27, by D4.
  - The judge's 26 rows match exactly.

  This replaces §1.1's Python evaluation of the same source as the evidence.
- **D2 in KiCad.** `with_tools`' `a_turned_keepout_is_where_kicad_draws_it` emits `sentinel-02-own` with U5 at 90, 180 and 270° and its two keep-outs. It routes, and `check drc --spec --schematic` passes with no finding (tools image, 10.8 s).
  - **Mutant:** the zone not turned. KiCad reports `lib_footprint_mismatch` on U5 at 90°, so the test fails: killed.
