# AET-WO-2302 — The twin's windows: one board with DShot, serial ports and slow sensors, witnessed whole

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-2302-windows`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass (the window decides what is witnessed: `OQ-02`'s risk, roadmap §3).
**Phase:** backlog that blocks P4 (roadmap §7.1: "must be reconciled before `AET-WO-4500`"), written 2026-11-16. **Cadence: spike, then stop for review, then build.** It runs after `AET-WO-4230` merges, because both change the judge in `renode.rs`.

**Grounds (§7.1 rows and the questions they came from):**
- **`4220` F-B1:** a DShot board's boot window is 2 ms (`AET-WO-4300a` D1, `DSHOT_WINDOW_NS`, architecture §2 notes). The IMU's and barometer's start-up waits are 10 and 20 ms (38.4 ms in the twin). Such a board fails, named, so the real Sentinel-02 (DShot motors with these sensors) is witnessed only in parts.
- **`4300b` Q4:** with the wire budget, a DShot board holds a 115200 console only if its last release is before about 75.6 µs, and never holds a 57600 MAVLink.
- **`4300b` Q8:** a serial port's campaign coverage. `3303` ruled that a declared bus needs a seeded fault the campaign survives or detects by its named mechanism; a serial port is covered by `serial_rx_dead`. There is no campaign override kind for a serial port, so `Target::Serial` is missing.
- **`4300a` Q6:** a board mixing DShot with PWM below about 260 Hz cannot witness the PWM in 2 ms. It fails closed today.
- **`4230` Q-E** (if `4230` stopped on it): a single start-up wait sized to the largest start-up may still miss 100 ms on `imu-baro-1v8`.
- **`3302` bench, 100K:** the H7's I2C clock synchronisation times SCL high from the pin's own threshold (about 1.7 V), not 0.7·VDD. The twin does not model clock synchronisation at all.
- **The CLI README** documents only version-1 scenarios. Eight merged work orders touched it and carried this.

> **What this is:** each witness gets the window its own evidence needs, anchored where that evidence starts. For example, a DShot window that opens at `start_outputs`, after the devices are up, rather than at reset. The archive stays bounded (`OQ-03`), and every event outside its window is refused, never truncated (the `4300a` ruling).
>
> It is NOT:
> - a longer fixed window for every board;
> - a change to what any witness judges inside its window;
> - the ArduPilot-in-the-twin spike (`4400c`).

## 1. Spike (measured, then stop)

1. **Where each window would anchor**, from the generated firmware's own sequence:
   - reset to boot word;
   - the device scripts' start-up and data steps;
   - `start_outputs`;
   - each serial port's pattern and echo.

   Measure each phase's virtual time on the reference, `motors-dshot600`, `serial-dshot-9600`, `imu-baro-1v8` and a Sentinel-02-shaped fixture with DShot, two serial ports and both sensors (write one if none exists).
2. **Three candidates, each measured for trace bytes, wall time and what it witnesses:**
   - **(a)** windows anchored per witness (DShot from `start_outputs`, serial from its first byte, devices from reset), each with its own length;
   - **(b)** one boot window of 100 ms, with DShot witnessed only in a short sub-window after `start_outputs`;
   - **(c)** two boots per board, a device boot and an output boot.

   Give `OQ-03`'s bytes per run for each.
3. **Q6 under the winner:** can a mixed board get two of its slowest model periods? Give the cost.
4. **Q8:** the `Target::Serial` shape for `3303`'s `stress()`, with the seeded firmware mutant as its mechanism.
5. **The 3302 synchronisation note:** does any verdict depend on the controller's SCL period (an identity read's timing, a window edge)? If none does, it is recorded as a model divergence in architecture §2, not modelled.

**STOP** with the measurements and a recommendation.

## 2. Build (after the ruling)

As ruled. Tier-1 and tier-2 tests per window, `Target::Serial`, and the CLI README's scenario section brought up to the current scenario versions.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-05`, `INV-06`; `RUL-17`, `RUL-19`; `OQ-02`, `OQ-03`.
- **Fence:** `aethelgard-cli` (`renode.rs`'s windows and judge, `campaign.rs`, the README); `aethelgard-chaos` (the archive if a window is recorded, and `stress()`); fixtures; `ci/` weights for new tier-2 tests; FEEDBACK.
- **Versions:** a trace or specification change takes the next free number at kickoff.
- One PR, left unmerged. Write `~/.cache/aet-2302/pr-title-body.md`.

## 5. Adversarial pass, full

- A firmware that delays `start_outputs` to push a bad DShot frame outside its window.
- A firmware that emits before its anchor.
- A serial echo that straddles two windows.
- A device read timed to the edge of its window.

Each must fail by name.

## 7. Flag-and-stop conditions

- **The spike's stop** (§1).
- **No candidate keeps a Sentinel-02-shaped board's archive within `OQ-03`'s stated bound.**
- **An anchored window lets a firmware choose its own anchor** in a way the judge cannot bound.
