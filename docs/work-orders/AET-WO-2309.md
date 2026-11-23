# AET-WO-2309 — Replay and the trace writer hold their own bounds

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-2309-replay-bounds`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4, backlog wave 1. Written 2026-11-23. **Cadence:** a measurement, then a build.

**Grounds** (roadmap §7.1; read `2303`, `2302` and `3201` FEEDBACK first):
1. **`2303` F3, D3 (#200).** `TraceWriter` writes past the reader's 256 MiB `MAX_TRACE_BYTES`: it wrote 367 MB, which `replay_dir` then refused. The writer should refuse at the same bound, by name, before writing past it.
2. **`2302` Q5, A2-2 (#175).** Replay should require exactly one kind 26 per kind 24 in a booted run.
3. **`3201` adversarial 5.8.** A booted run whose registers and records are all set to unread replays as `N1`. Add a rule: all read or all unread, across kinds 10–13, 21 and 22.

> **What this is:** (1)–(3), each with a test and a mutant it kills. Only malformed archives are newly refused.
>
> It is NOT:
> - a trace-version bump;
> - a change to `campaign.rs`'s scenario format, the CLI, or the twin.

## 1. Build

1. **Measure first,** with predictions in `spike/2309/`: replay every committed archive and trace fixture before the change, and again after it.
2. **(1)–(3).**
3. **Re-run the sweep:** every committed archive replays identically.

## 2. Acceptance

- **(1)–(3) closed.**
- **Every committed archive and trace fixture** replays as before.
- **The CLAUDE.md gates pass, and CI is green.**

## 3. Fences

- **Fence:** `crates/aethelgard-chaos` (`archive.rs` `TraceWriter`, `replay.rs`, tests); `spike/2309/` and FEEDBACK.
- **Not:** `campaign.rs`'s scenario format, the CLI, the twin, the IR; workflows, `ci/` (except a test-weights row).
- **Running beside you:** `4300i`, `4407`, `1607`, `3307`. Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-2309/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A writer that crosses the bound in one large record.**
- **A booted run with kind 24 and no kind 26,** or two of kind 26, that still replays.
- **A mixed read/unread run** that slips past the rule through a kind not listed.

## 7. Flag-and-stop conditions

- **A committed archive or trace fixture fails the stricter replay.** Stop and name it.
- **A `fab/` byte moves.** Stop.
