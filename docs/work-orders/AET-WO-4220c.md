# AET-WO-4220c — The ICM-42688-P model's FIFO, as the datasheet defines it

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4220c-icm-fifo`.
**Author:** Claude (architect) · **Builder:** strong tier (the `1313c` builder, reused, after `1313c` merges), with a targeted adversarial pass.
**Phase:** P4. Written 2026-11-17 from `AET-WO-4400e`'s stop (PR #212, ruling comment on that PR, D2). **Cadence: build, with one measured stop (§1.1).**

**Grounds:**
- **`4400e` FEEDBACK, B2, read at source:** `models/renode/ICM42688P.cs` L7–8 says "Not modelled: the FIFO (FIFO_COUNT reads 0, FIFO_DATA reads its reset 0xFF)".
- **ArduPilot reads this IMU only through its FIFO.** `read_fifo` reads `FIFO_COUNTH` (`0x2E`) and stops on a zero count (`AP_InertialSensor_Invensensev3.cpp` L583–590). `wait_for_sample` never leaves its loop without a sample (L2058–2120).
- So even past `4400e2`'s fault, unmodified ArduPilot would wait in `init_gyro` for good on this twin: a model gap, not a firmware fault.
- **The model is this project's own** (`AET-WO-4220`), so the gap is ours to close, from the datasheet (`INV-06`: no register value without its source).

> **What this is:** the FIFO of the ICM-42688-P model, as DS-000347 defines it, filled from the same stimulus the data registers already serve.
>
> It is NOT:
> - a change to the data registers' behaviour (the generated Rust firmware reads those, and its verdicts must not move);
> - interrupts or the INT pins, unless §1.1 shows ArduPilot needs them;
> - the APEX features.

## 1. Build

1. **Measure first, then STOP.** Read at source and cite:
   - DS-000347's FIFO chapter and register map: `FIFO_CONFIG`, `FIFO_CONFIG1`–`3`, `FIFO_COUNTH`/`L` (and its record and endian modes), `FIFO_DATA`, the packet formats and their headers, `FIFO_LOST_PKT`, and what a read of an empty FIFO returns;
   - exactly which FIFO registers, modes and packet format ArduPilot's `Invensensev3` driver sets and reads at the pinned tag, in order;
   - how the model's stimulus reaches the data registers today, and its time base: the configured ODR against virtual time.

   **STOP** with:
   - the registers and behaviours you propose to model, and those left out;
   - each with its datasheet citation;
   - the packet ArduPilot gets for the model's stimulus, worked byte by byte;
   - every golden and verdict that would move. The expected answer is none of the generated firmware's.
2. **After the ruling:** the FIFO in `ICM42688P.cs`, and its tests. A register or mode outside the ruled set is logged by the model as unmodelled, never served a guessed value.

## 2. Acceptance

- **The model's tests:** for the stimulus, the FIFO count grows at the configured ODR in virtual time, and each packet matches the datasheet's format byte for byte (worked in a test). A read past the count returns what the datasheet says.
- **The generated Rust firmware's verdicts do not move:** the reference, the bench board and every scenario.
- **A twin run of unmodified ArduPilot past the IMU probe is not required here.** `4400e2`'s fault comes first. If that fault is fixed before this merges, record what ArduPilot's `read_fifo` receives.
- **The CLAUDE.md gates pass.** CI is green, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-06`, `INV-07`; `RUL-05`, `RUL-20`.
- **Fence:**
  - `models/renode/ICM42688P.cs` and its tests;
  - the model's README entry;
  - FEEDBACK.
- **Running beside you:** `4400e2` (a spike that reads this model, and changes none). One container of yours at a time, under `capped-run`.
- One PR, left unmerged. Write `~/.cache/aet-4220c/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A packet that passes the test but not the datasheet:** byte order, the header bits, the timestamp field.
- **A count that grows with wall time instead of virtual time.**
- **A data register whose behaviour changed.**

## 7. Flag-and-stop conditions

- **§1.1.**
- **ArduPilot needs an interrupt or a mode the datasheet leaves ambiguous:** stop, with the text cited.
