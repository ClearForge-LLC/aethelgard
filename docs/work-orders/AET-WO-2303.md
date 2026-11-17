# AET-WO-2303 — Spike: does Sentinel-02's fault campaign need per-quantum lock-step, and what does it cost the archive?

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-2303-dynamic-volume` (a spike: its code stays under `spike/2303/`, and only FEEDBACK is a deliverable).
**Author:** Claude (architect) · **Builder:** strong tier.
**Phase:** P4, before `AET-WO-4500` is written (roadmap §5, from the Grok review of 2026-11-17). Written 2026-11-17. **Cadence: spike only, ending in a stop.**

**Grounds (on `main`):**
- **`OQ-03`** (architecture §2.17, §8):
  - answered for static campaigns: about 1 KB per scenario;
  - answered for declared droops (`AET-WO-2301`, Scotty D8): 140–686 B per run, with nothing per quantum, so nothing is sampled.
  - "**Per-quantum lock-step, when a firmware-dependent load needs it, reopens this.**" The estimate is 3 to 6 MB per simulated second at 10 µs quanta, against the reader's bounds (256 MiB; `MAX_RECORDS` 65,536).
- **`RUL-17`:** the bridge steps `libngspice` synchronously, with inversion of control at about 10 µs for a dynamic case.
- **Sentinel-02** (roadmap §5, `4500`) has four motors on DShot and a battery rail. A rail whose droop depends on what the firmware commands is the "firmware-dependent load" `OQ-03` names.
- **`INV-04`:** sampling is a decision that comes back to Scotty.

> **What this is:** the measurement `4500` needs before its specification is frozen. Does any fault `4500`'s campaign must contain require per-quantum lock-step? If one does, what does one such run cost the archive, and does it replay?
>
> It is NOT:
> - a merged change to the twin, the bridge or chaos (prototypes live in `spike/2303/`);
> - a sampling proposal (that would be Scotty's, under `INV-04`).

## 1. Spike

1. **The question first, at source.** List the faults `4500`'s campaign needs: roadmap §5's text, `2301`'s fault kinds, and the Sentinel-02-shaped fixture's parts. Classify each as **static**, **declared** (`2301`'s droop) or **firmware-dependent**: its electrical effect depends on what the firmware does during the run, as motor current on the battery rail does.
   - If none is firmware-dependent, say so with the reasoning and stop. `OQ-03` stays answered.
2. **If one is,** prototype the smallest per-quantum lock-step that can run it, under `spike/2303/`. Use one Sentinel-02-shaped board and one fault, at 10 µs quanta. Measure:
   - the archive's bytes per simulated second and its records per run;
   - the wall time per simulated second;
   - whether the archive replays (`replay` reproduces the verdict), and whether it fits `MAX_RECORDS` and the 256 MiB bound;
   - at which simulated length, if any, a run stops fitting.

   Label each figure measured, read at source or inferred.
3. **The options, if it does not fit,** without choosing: larger bounds, a coarser quantum with its error stated, or recording only around events. Name which, if any, is sampling under `INV-04` (Scotty's decision).

## 2. Stop

**STOP** with FEEDBACK: the classification table, the measurements, and the options. Push an `AET-WO-2303 STOPPED:` commit and open a PR for the record only; it is not merged as code.

## 3. Invariants and fences

- **Invariants:** `INV-03`, `INV-04`, `INV-05`, `INV-06`; `RUL-05`, `RUL-06`, `RUL-17`.
- **Fence:**
  - `spike/2303/`;
  - FEEDBACK.

  Read anything; change no crate on the branch.
- **Running beside you:**
  - `2302b` (evolve, chaos);
  - `1604` (CLI);
  - `4400b2` (pincheck);
  - `1313` (eda placement).

  You run Renode and ngspice: one container of yours at a time, and use `capped-run`.
- One PR, left unmerged. Write `~/.cache/aet-2303/pr-title-body.md`.

## 7. Flag-and-stop conditions

- **§2 is the stop.**
- **The prototype needs a bridge change** that cannot be made under `spike/2303/` without editing a crate. Stop and say what.
