# AET-WO-1915 — Spike: where `verify` and `prove`'s 9–13 GB memory peak comes from

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1915-memory-peak`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4, backlog wave 1 (the optional spike). Written 2026-11-23. **Cadence:** a measurement that stops. No crate code.

**Grounds** (roadmap §7.1, `3304` F3 and `4201` F4):
- `verify` and `prove` on a board with Kani harnesses need more than 8 GiB: a 9.1 GiB peak (`3304`), and an OOM at 8 GiB (`4201` F4).
- CI's 16 GB runners hold it. Local `capped-run` callers need `--mem 16G` until the peak is measured to its source.
- The Kani proofs are the guess, and no one has measured it.

> **What this is:**
> - A measurement of the peak resident memory for `verify` and `prove` on the committed boards that carry Kani harnesses. It is split by phase and process:
>   - the `cargo` build;
>   - each Kani harness, through CBMC and the solver;
>   - the campaign;
>   - ngspice;
>   - Renode.
> - The peak per harness.
> - Whether the peak is serial or concurrent: the harnesses run in parallel, or `cargo`'s jobs overlap.
> - **A lean:** the one change that brings the peak under 8 GiB, if any exists, with its cost in wall time. For example, Kani `--jobs`, a harness `#[kani::unwind]` bound, a solver choice, or serializing a phase. Then stop.
>
> It is NOT:
> - any crate, workflow or `ci/` change;
> - a change to a harness or a proof bound.

## 1. Measure

1. **Predictions first,** in `spike/1915/`.
2. **The run.** Use `capped-run --mem 16G`, one container at a time, and sample the cgroup's `memory.current` and `memory.peak` at 1 s.
   - Attribute each sample to a process tree by `/proc`, from a scratch reader, with no `Cargo.toml` under `spike/`.
   - Measure `prove` on its own, then `verify`.
3. **Repeat each run** to show the peak is stable within 5 %.
4. **The lean,** measured, not estimated: re-run the one candidate change from a scratch copy, outside the branch.

## 2. Acceptance

- **The peak is attributed** to a phase and a process, with its timeline.
- **The candidate change is measured** for its peak and its wall time.
- **No crate file changes.** The gates are not run on a spike-only branch; say so.

## 3. Fences

- **Fence:** `spike/1915/` and FEEDBACK.
- **Not:** any crate, specification, `fab/`, workflow or `ci/` file.
- **Furnace's memory:**
  - two builders run beside you (`4300i`, `2310`) and `1607`;
  - keep to one 16G container at a time;
  - stop and say so if free memory falls under 12 GB.
- One PR, spike-only, left unmerged. Write `~/.cache/aet-1915/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A peak missed between samples:** cross-check against `memory.peak`.
- **A process attributed to the wrong phase:** a `cargo` job still alive during Kani.

## 7. Flag-and-stop conditions

- **Stop after §1.4** with the attribution and the lean.
