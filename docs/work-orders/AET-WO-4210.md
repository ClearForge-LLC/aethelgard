# AET-WO-4210 — Sentinel-02's IMU and barometer: chosen, read more than once, witnessed

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4210-imu-baro`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass at the build.
**Phase:** P4. **Cadence: spike (§1), then STOP for the architect's ruling, then build (§2).** The build waits for `4300c` to merge.

**Grounds, each read on `main` at `c621461`:**
- **No common flight-controller IMU or SPI barometer is modelled in Renode 1.17.0** (`architecture.md` §2.22). The modelled stand-ins: `ICM20948` and `LSM6DSO` on SPI; `BMP180`, `LPS25HB` and `ICP-101xx` on I2C.
- **ArduPilot ships drivers for several of the modelled parts:**
  - the ICM-20948 (`AP_InertialSensor_Invensensev2`);
  - LPS2xH barometers (`AP_Baro_LPS2XH`);
  - ICP-101xx (`AP_Baro_ICP101XX`).

  Verify each against the ArduPilot tag `4400` pins.
- **The ICM-20948's `VDDIO` is 1.71 to 1.95 V** (`4200` F10): on a 3.3 V bus it needs a level shifter or a 1.8 V I/O domain.
- **The witness allows one transaction per device** (`2204` A4). An IMU's bring-up needs at least a configuration write and a data read. `i2c_witness_out_of_order` fails that today. SPI's multiplexer lines are collected per controller, not per read (`4200` F16).
- **"Parked" does not exclude an interrupt after the boot word** (`2204` A2). An IMU's data-ready line is the first interrupt P4's firmware might want.
- **Kind 13 can reach the event bound** (`4200` F18), and a device's `Warning` lines are not judged (`4200` F22).

> **What this is:** Sentinel-02's IMU and barometer chosen from what the twin can witness and ArduPilot can drive, and the witness grown from one identity read to a short, ordered transaction script per device.
>
> It is NOT:
> - a Renode model of our own (a ruling, and `RUL-12` says Rust);
> - sensor fusion;
> - ArduPilot in the twin.

## 1. Spike

Probes under `spike/4210/`. **Predictions go into FEEDBACK before any run.**

**S1. The parts, by evidence.** For each modelled IMU and barometer, gather four things:
- **What the Renode model implements:** registers, and what it logs per transaction, read at the pinned commit.
- **ArduPilot's driver:** read at `4400`'s tag. Which registers it touches at probe and init, and whether the model answers them.
- **The datasheet:** supply and `VDDIO` ranges, bus modes and maximum clock. Fetch it from the vendor and hold it by sha256, as `4200` did.
- **Availability:** whether the part is in current production. Read the vendor page.

**Recommend one IMU and one barometer for Sentinel-02, with a reason.** A part needing a 1.8 V I/O domain is acceptable only if `4300d` can declare that domain; say so.

**S2. The transaction script.** Propose how a device's bring-up becomes a short, declared script instead of one identity read:
- the identity read;
- one configuration write;
- one data read.

Each step is witnessed by the model's own lines, in order, per device. Measure on the chosen parts (or the stand-ins) whether each model logs enough to witness a write and a data read, not just an identity. Decide where the script lives: the parts table, beside the identity, cited. Then:
- **Per-read windows for SPI (F16):** propose and measure markers between reads, so that one controller's multiplexer lines are attributed to the right read.
- **The event bound (F18):** size kind 13 and kind 22 for a script, not one read.
- **Device warnings (F22):** say whether a device `Warning` during a script fails the read, and measure what the chosen models warn about in a correct run.

**S3. Interrupts (2204 A2).** Measure whether the chosen IMU's model drives a data-ready GPIO. Then propose:
- whether the bring-up firmware stays polled, keeping "parked" as `RUL-19` (5) states it;
- or arms one EXTI and the judge learns the interrupt. That is a ruling.

**S4. What the specification gains.** Say which of these the specification needs, and what the solver does with each:
- an IMU's bus and select;
- its data-ready pin;
- a barometer's address;
- a 1.8 V domain if chosen.

**Stop** with the table, the recommendation for each part and each question, and the decisions needed.

## 2. Build, after the ruling (expected; the ruling confirms)

1. **The chosen parts in `parts.json`,** cited, with their transaction scripts.
2. **The generated firmware runs each device's script.** The board crate's typestates cover it, and the Kani harness proves it.
3. **The judge witnesses each script step per device, in order,** with per-read windows on SPI. Kinds 13 and 22 are budgeted for scripts. Device warnings are judged as ruled. The trace version is bumped once.
4. **A test-fixture specification with the chosen IMU and barometer** passes every gate in the image, with one seeded fault per part.
5. **2204 A4, A2 (as ruled), and 4200 F16, F18 and F22 are closed.**

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01` to `INV-07`; `RUL-12`, `RUL-16`, `RUL-19`.
- **The spike fence:** `spike/4210/` and FEEDBACK only. Datasheets go in `~/.cache/aet-4210/ds/`.
- **Other builders on Furnace:** `1307` (router), `4300c` (pin speeds), `4400` (ArduPilot spike). Never touch their worktrees. To read ArduPilot, clone your own copy at `4400`'s tag into `~/.cache/aet-4210/`, or read `~/.cache/aet-4400/`'s clone without writing to it. One container of yours at a time.
- One PR, left unmerged. Write `~/.cache/aet-4210/pr-title-body.md`. **Push and revoke by the FUR-WO-0301 F7 route.**

## 7. Flag-and-stop conditions

- No modelled IMU or barometer is both drivable by ArduPilot and witnessable beyond its identity. The choice becomes a model of our own, which is a ruling.
- A vendor page or datasheet host refuses Furnace. Stop with the URL, for Scotty.
