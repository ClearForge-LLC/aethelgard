# AET-WO-4400c — Spike: unmodified ArduPilot boots on the twin's Sentinel-02 platform until its drivers probe the IMU and the barometer

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4400c-ardupilot-twin` (a spike: its code stays under `spike/4400c/`, and FEEDBACK is the deliverable).
**Author:** Claude (architect) · **Builder:** strong tier (the `2303` builder, reused: it has driven Renode per quantum from the CLI's own monitor script on the Sentinel-02 shape).
**Phase:** P4 (roadmap §5: `4400b`, then `4400c`, then a flight-loop order). Written 2026-11-17, after `4400b` merged (#189) and with `4400b2` (#196) in review. **Cadence: spike only, ending in a stop.**

**Grounds (on `main`):**
- **Roadmap §5, `4400c` (review B4):** "Boot the CI-built ELF on the generated platform until its drivers probe the IMU and barometer, witnessed by `4210`'s scripts."
- **CI already builds the ELF.** `ardupilot-image.yml`'s `generated` job builds unmodified `Copter-4.7.2` from `generate hwdef` on `crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json`: the ICM-42688-P on SPI, the BMP581 on I2C, both modelled by this project (`models/renode/`, `AET-WO-4220`).
- **The twin emits that board's platform** (`aethelgard-twin`), and `simulate --firmware` judges a boot on it. Every rule of that judge was written for the generated Rust firmware (CLI README, *Booting firmware*):
  - the boot word `0x2100B007` at `0x20000000`;
  - exactly 100 MIPS × the virtual time (a core that sleeps, `WFI`, is `cpu_halted`);
  - the core parked on a `yield; b.n` self-loop;
  - every solved register read back as the topology computes it;
  - `--for` at most 100 ms;
  - each device's `4210` transaction script, witnessed from the model's own lines, in order.

  ArduPilot does none of the first three, and its probe sequence is not the generated firmware's script.
- **Rows owned here (roadmap §7.1):**
  - `SPI_MAX_FRAMES` 8 against ArduPilot's FIFO reads of up to 129 bytes (`4220` D-B4, F-B5);
  - the judge's DShot stop as a twin artifact a firmware that reads its timer later would see (`2302` D2);
  - the RC port judged at its declared 420000 baud while ArduPilot runs 416666 (#189 F3);
  - `4400b2` D2: each port's `DEFAULT_SERIALn_PROTOCOL` and `_OPTIONS` (#196, comment 6943747101).

> **What this is:** the measurement a build needs before any judge is changed. How far does unmodified ArduPilot get on the platform the twin emits for Sentinel-02? What stops it, and why? When its drivers probe the IMU and the barometer, what do the twin's own models witness? Which rules of `simulate --firmware`'s judge would a foreign firmware fail, and what would each need?
>
> It is NOT:
> - a merged change to the twin, the CLI's judge, a model or the generator (prototypes live in `spike/4400c/`);
> - a change to ArduPilot's tree (the ELF is CI's, unmodified);
> - the flight loop.

## 1. Spike

1. **Predict first, at source.** Before any run, commit predictions to FEEDBACK:
   - ChibiOS's start-up on the STM32H753 (RCC, PWR, the PLL from `OSCILLATOR_HZ`, flash wait states, the MPU, DMA);
   - ArduPilot's init order up to `AP_InertialSensor` and `AP_Baro` probing;
   - which Renode 1.17.0 models and which emitted platform entries each step touches;
   - where you expect it to stop first.
2. **Boot it.** Use CI's ELF, or one built byte-identically with `container/ardupilot-build.sh` in the pinned ArduPilot image (record its sha256 against CI's). Load it on the platform `aethelgard-twin` emits for `sentinel-02-own.json`, driven by Renode 1.17.0 in the pinned tools image, outside the CLI's 100 ms bound, as `2303`'s probe drove it. **For each stall, in order:**
   - say where it stopped (PC, symbol, the register polled);
   - classify the stall:
     - **(m)** a Renode model missing or incomplete (name the peripheral and register);
     - **(p)** the emitted platform lacks an entry;
     - **(h)** the generated `hwdef.dat` is wrong for this board: a real finding, reported first;
     - **(x)** ArduPilot needs hardware the board does not declare (USB, SD card, …);
   - work past it only inside `spike/4400c/` (a monitor-script stub or a platform overlay), each labelled as a deviation, with what a build would do instead.

   Stop advancing at the first stall a stub cannot pass, and say why.
3. **The probe, witnessed.** Once `AP_InertialSensor` and `AP_Baro` probe, collect the ICM-42688-P's and the BMP581's own model lines. Label every figure measured, read at source or inferred, and report:
   - the virtual time of each probe;
   - every transaction, in order, against each part's `4210` script in `parts.json`: which steps ArduPilot's traffic satisfies, which it violates, and what it does that no script names;
   - the SPI transfer lengths (the `SPI_MAX_FRAMES` row) and whether the twin's model serves them;
   - a second witness, independent of the first: ArduPilot's own console on SERIAL0, if it prints its sensor detection. Say whether the two agree. A console line alone witnesses nothing (`INV-07`).
4. **The owned rows, where the run reaches them.** Report the baud ArduPilot programs on each UART (`BRR` read from the model; the RC port's 416666), whether DShot starts unarmed and whether anything reads a motor timer later, and each port's protocol as ArduPilot set it. Where the run does not reach a row, say so and why.
5. **The judge, rule by rule.** For each rule of `simulate --firmware`'s judge, say whether this boot passes it, fails it or cannot be judged, and what a build would need. Choose nothing. For example:
   - a boot word written by an ArduPilot hook;
   - a different witness of "booted";
   - a separate judge for foreign firmware;
   - the instruction-count rule under `WFI`;
   - the virtual length, with wall time and Renode memory per simulated second measured.

   Name any option that would weaken a witness, and how (`INV-07`).

## 2. Stop

**STOP** with FEEDBACK containing:
- the predictions, then the stall table;
- the probe's witness table;
- the owned rows;
- the rule-by-rule table;
- the measurements: wall time per simulated second, peak RSS, and the virtual time to each probe.

Push an `AET-WO-4400c STOPPED:` commit and open a PR. It is merged as the record, as `2303`'s was: FEEDBACK plus `spike/4400c/`, with no crate touched. Keep large binaries out of git: an ELF is named by its sha256 and the command that rebuilds it.

## 3. Invariants and fences

- **Invariants:** `INV-03`, `INV-06`, `INV-07`; `RUL-05`, `RUL-13`, `RUL-16`, `RUL-20`.
- **Fence:**
  - `spike/4400c/`;
  - FEEDBACK.

  Read anything; change no crate, model, workflow or `container/` file on the branch.
- **Running beside you:**
  - `2302b` (evolve, chaos);
  - `1604` (CLI);
  - `3402` (ir, svd-hal, eda's FAB-NOTES);
  - `1313` (eda placement);
  - `4400b2` (pincheck; merging).

  You run the ArduPilot and tools images: one container of yours at a time, under `capped-run`.
- One PR. Write `~/.cache/aet-4400c/pr-title-body.md`.

## 7. Flag-and-stop conditions

- **§2 is the stop.**
- **An (h) stall:** the generated board is wrong for ArduPilot. Stop at once with the line, the generator's rule and ArduPilot's reading, before working past it.
- **Booting needs a change** to a crate, a model or the image that cannot be prototyped under `spike/4400c/`. Stop and say what.
