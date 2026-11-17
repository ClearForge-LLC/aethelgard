# AET-WO-4400e — The foreign-firmware judge: unmodified ArduPilot booted, probing and reading its sensors on the twin, witnessed by the models' own lines

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff, **after `AET-WO-4400d` merges** (the `IMU` line).
**Branch:** `wo/aet-4400e-foreign-judge`.
**Author:** Claude (architect) · **Builder:** strong tier (the `2303`/`4400c`/`4400c2` builder, reused), with a full adversarial pass.
**Phase:** P4 (exit clause 3; the flight path `4400b` → `4400c` → `4400c2`, `4400d` → this order → the flight loop). Written 2026-11-17. **Cadence: build, with one measured stop (§1.1).**

**Grounds (on `main`):**
- **`4400c` §1.5, measured: every rule of `simulate --firmware`'s judge, read against unmodified ArduPilot.** The judge was written for the generated Rust firmware. ArduPilot fails or cannot be judged on:
  - the boot word;
  - 100 MIPS × virtual time (`WFI` in `__idle_thread`);
  - the self-loop;
  - the exact cold RCC set, `SysTick` and `NVIC`;
  - USART1's registers (its own clock tree);
  - `--for` ≤ 100 ms (the barometers at 1.97 s, INS at 5.21 s);
  - the `4210` scripts' configure steps (the BMP581 row, roadmap §7.1);
  - the serial pattern;
  - and it is refused before boot by `FirmwareOutsideMemory` (`.data` at `0x30002200`).
- **What passes unchanged:** the solved pins' `MODER`/`AFR` (6 of 6), and `device_unmodelled`.
- **Measured costs:** 1.09 s of wall time per simulated second, and a peak Renode RSS of 460 MiB.
- **`4400c2` (#204):** the twin's SPI receive DMA lines are emitted. Unmodified ArduPilot finds its dataflash on the emitted platform with no overlay.
- **`4400d`:** the `IMU` line is emitted, so ArduPilot compiles and probes the ICM-42688-P.

> **What this is:** a second judge, for a firmware this project did not generate. It is chosen by a declaration, never inferred (`INV-06`). It says "booted", "probed" and "read" from witnesses the firmware cannot write: the models' own transaction lines and the platform's own state (`INV-07`). Every rule of today's judge is either kept, replaced by a named equivalent, or dropped with its weakening stated.
>
> It is NOT:
> - a change to any verdict of the generated firmware (that judge is unchanged, byte for byte);
> - the flight loop;
> - a Renode model change (USART IDLE and timer DMA stay backlog rows);
> - the USART receive DMA line (held; roadmap §7.1), unless §1.1 shows the judge needs SERIAL0 receive, in which case stop.

## 1. Build

1. **Measure first, then STOP.** On the CI-built ELF from `4400d`'s generated `sentinel-02-own` board, booted on the emitted platform under the CLI's own monitor script, with only the ELF swapped (as `4400c` did), measure:
   - the virtual time of each witness:
     - the first model identity line;
     - each device's identity transaction;
     - the ICM-42688-P's probe and its first data read;
     - each barometer's first data read;
   - each device's transactions against its `4210` script: which steps hold for ArduPilot's traffic and which do not;
   - the window the judge needs, with its wall time and peak RSS;
   - whether anything the judge needs requires SERIAL0 receive (the held USART line).

   **STOP** with:
   - **How a foreign firmware is declared.** A CLI flag, a field in the specification, or a manifest beside the ELF. Recommend one. Nothing may infer it from the ELF.
   - **The rule table:** for each rule of today's judge, keep, replace (with what) or drop, with the weakening named (`INV-07`).
   - **"Booted":** which witness. The console banner alone is refused, because firmware-chosen text witnesses nothing.
   - **The per-device witness:** identity and data only, or a script per firmware. If identity and data only, say what the configure step witnessed and what is lost.
   - **The memory rule:** which regions `PT_LOAD` may use for a foreign firmware, from ST's header (`RUL-16`), never typed in.
   - **The window,** at most what fits the CLI's 30 s wall bound with margin, measured.
   - Every golden and verdict whose bytes would move. The expected answer is none of the generated firmware's.
2. **After the ruling:**
   - the judge in `aethelgard-cli` (or the crate the ruling names), sharing no code with the generator's facts it judges (`RUL-20`);
   - CI runs it on the generated `sentinel-02-own` ELF.

## 2. Acceptance

- **The positive case.** Unmodified ArduPilot on the emitted Sentinel-02 platform is judged `pass` by the foreign judge, with each witness's virtual time in the verdict:
  - booted;
  - the ICM-42688-P probed and read;
  - both barometers probed and read.
- **Mutations, each refused by name:**
  - the `IMU` line removed (no probe);
  - an SPI receive DMA line dropped from the platform (no data);
  - a device's model removed;
  - the window too short for a declared witness;
  - an ELF declared foreign that writes outside its regions;
  - the generated firmware judged as foreign without a declaration, or ArduPilot without one.
- **Today's judge is unchanged:** every generated-firmware verdict is byte-identical, the reference, the bench board and every scenario included.
- **The CLAUDE.md gates pass.** CI is green, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-03`, `INV-06`, `INV-07`; `RUL-05`, `RUL-13`, `RUL-16`, `RUL-20`.
- **Fence:**
  - the judge's crate, as the ruling names it;
  - the CLI's `simulate` surface and its README section *Booting firmware*;
  - the CI job that runs it (`.github/workflows/ardupilot-image.yml` or `ci.yml`);
  - its tests;
  - FEEDBACK.
- **Running beside you:** `1313c` (`aethelgard-eda` placement). You share nothing with it.

  You run the ArduPilot and tools images: one container of yours at a time, under `capped-run`.
- **The roadmap §7.1 rows this order owns:**
  - the model divergences (TIM5 at a fixed 250 MHz, `usart1` at 200 MHz);
  - the BMP581 configure row;
  - `4400c2` D2 and D3: the USART line, only if §1.1 shows receive is needed.

  Close each in FEEDBACK or re-own it.
- One PR, left unmerged. Write `~/.cache/aet-4400e/pr-title-body.md`.

## 5. Adversarial pass, full

- **A witness the firmware can forge:** any rule that reads memory or text the firmware writes, standing alone.
- **A foreign judge that passes the generated firmware's failures,** or today's judge weakened by shared code.
- **A pass that depends on wall time or host speed.**

## 7. Flag-and-stop conditions

- **§1.1.**
- **The judge needs SERIAL0 receive:** stop. The held USART line and its hard fault (`4400c2` D3) come first.
- **A witness needs a Renode model change:** stop and say what.
