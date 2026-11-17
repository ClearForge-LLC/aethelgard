# AET-WO-4400e2 — Spike: the hard fault unmodified ArduPilot meets on the twin, 0.67 ms after the IMU probe

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4400e2-hard-fault`. A spike: its code stays under `spike/4400e2/`, and FEEDBACK is the deliverable.
**Author:** Claude (architect) · **Builder:** strong tier (the `4400e` builder, reused: it found the fault and holds the hooks).
**Phase:** P4. Written 2026-11-17 from `AET-WO-4400e`'s stop (PR #212, ruling comment on that PR, D1 (a)). **Cadence: spike only, ending in a stop.** `4400e` waits on it, parked at its stop.

**Grounds (`4400e` FEEDBACK, B1, measured):**
- CI's unmodified `sentinel-02-own` ArduPilot enters `HardFault_Handler` at 3.570806620 s virtual, every run, hooked or not. The fault comes 0.672 ms after the ICM-42688-P's identity read.
- The fault status: `CFSR` `0x00020000` (`INVSTATE`), `HFSR` `0x40000000`, `EXC_RETURN` `0xFFFFFFED`.
- In ChibiOS's `__port_switch`, the switch goes from the idle thread to `ch0`, the ready list's header, after a preemption that no interrupt requested.
- **`4400c2` D3** has the same signature (USART1's receive line with the SPI lines, during the dataflash erase). It is inferred to be the same fault, and here no USART line is emitted.
- **Cause not established:** the twin's Cortex-M emulation (Renode 1.17.0's tlib: exception entry and return, tail-chaining, the FPU's lazy stacking) or ArduPilot/ChibiOS.
- **Inferred, not measured:** this driver runs on silicon at every boot of a board with this IMU.
- **Why it matters (`INV-07`):** a twin that manufactures a fault silicon does not have would fail a firmware for the twin's own error. A twin that hides a real one would pass it. Either way the verdict is not a witness until the cause is known.

> **What this is:** the measurement that says where the fault comes from, and what would remove it without weakening any witness.
>
> It is NOT:
> - a change to any crate, model, workflow or image on the branch (prototypes and deviations stay under `spike/4400e2/`, each labelled);
> - the foreign judge (`4400e`);
> - the ICM-42688-P's FIFO (`4220c`).

## 1. Spike

1. **Predictions first,** committed to FEEDBACK before any run.
2. **Settle the identity.** Run `4400c2`'s repro (`spike/4400c2/sh/crash.sh` at `5a1bccaf`, reachable from PR #204's head `1061fd4a`) with `4400e`'s PSP-frame hook. Is the frame the same switch to `ch0`?
3. **Locate it.** Measure, do not reason from a single PC:
   - `ICSR.RETTOBASE` at each ISR's exit, not only its entry;
   - TIM5's and every other interrupt in the last millisecond;
   - `FPCCR` and `FPCAR` (lazy stacking) at each exception entry and return, and whether each frame is the extended (FPU) frame `EXC_RETURN` says it is;
   - the `PendSV` and `SVC` priorities ChibiOS sets, against what tlib honours.

   Then bisect with **labelled diagnostic deviations**, each recorded with what a build would do instead:
   - an ArduPilot built with lazy stacking off, or the FPU context off, in the pinned ArduPilot image;
   - a newer Renode release than 1.17.0, in a scratch image;
   - a tlib setting or a monitor command, if one exists.

   Read at source where it decides: ChibiOS's `chcore.c` and `chcore_v7m.h` at the pinned ArduPilot tag, and tlib's Cortex-M exception code at the pinned Renode commit.
4. **`4400e` D4, the SPI `[ERROR]`.** Renode's `STM32H7_SPI` logs "Attempted to set peripheral into SPI slave mode" once per ArduPilot SPI transaction but one.
   - Which register write triggers it?
   - What does RM0433 say that write does on silicon?
   - Is the model's message a true divergence or a false alarm?

   Read the model's source at `renode-infrastructure` `cd4b002a` (`4400c2` read it there).
5. **The cost:** wall time and peak RSS per diagnostic run, under `capped-run`.

## 2. Stop

**STOP** with FEEDBACK holding:
- the predictions, scored;
- the identity with `4400c2` D3;
- the cause, classed as **twin emulation**, **firmware** or **unknown**. For unknown, give the bounds: what was excluded, and by which run.
- for a twin cause, the smallest fix and where it lives. The candidates:
  - a Renode patch upstream;
  - a pinned-image change;
  - a platform or monitor setting.

  For each candidate, say whether it weakens any witness. For a firmware cause, the evidence that silicon would fault too.
- D4's answer.

Push an `AET-WO-4400e2 STOPPED:` commit and open a PR. It is merged as the record. Keep large binaries out of git: name each ELF by its sha256 and the command that rebuilds it.

## 3. Invariants and fences

- **Invariants:** `INV-03`, `INV-06`, `INV-07`; `RUL-05`, `RUL-13`, `RUL-16`.
- **Fence:**
  - `spike/4400e2/`;
  - FEEDBACK.

  Read anything. Change no crate, model, workflow or `container/` file on the branch.
- **Running beside you:**
  - `1313c` (eda placement);
  - `4220c` (the ICM model's FIFO, `models/renode/`), when it starts. You read that model; you do not change it.

  You run the ArduPilot and tools images: one container of yours at a time, under `capped-run`.
- One PR. Write `~/.cache/aet-4400e2/pr-title-body.md`.

## 7. Flag-and-stop conditions

- **§2 is the stop.**
- **A cause in tlib that cannot be shown by a run inside the pinned image or a scratch image:** stop with the bounds.
