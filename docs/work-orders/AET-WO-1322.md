# AET-WO-1322 — eda's reduction and reader hardening: unreachable guards, the reader's echo and name clones, one reference order, the IPROPI text, and the harness's lost mutant

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1322-eda-hardening`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4, backlog wave 2. Written 2026-11-23. **Cadence:** a measurement, then a build. **After `4300i`** (#338), which drew the motor pull in eda.

**Grounds** (roadmap §7.1; the rows are listed in `docs/reviews/2026-11-23-backlog-triage.md` under W2-EDA):
1. **`1208` F3.** Some eda guards can no longer be reached from `emit`, because it validates first: `UnsupportedAddress`, a crossed bus's `PowerNotDrawable`, and others. Delete each one, or label it as a defensive re-check with a test that reaches it directly.
2. **`1311` adversarial C.** `board::prepare_read` clones the carrying footprint's or pad's name onto every child it scans. A 10 kB name blew up to a 24 GB kill. Borrow, or bound, instead.
3. **`1311` adversarial J, F, the eda half.** The reader echoes refused text whole. Bound the echo and escape it, as `4407` did for the pin check. `MAX_ITEMS` does not count courtyard edges: count what `check` actually scans.
4. **`1211f` F3, `1217` Q1.** The IPROPI guard's text (`schematic_arms.rs`) still says an undeclared IPROPI is the only case. Name the GND tie that `1217` admits.
5. **`1218` F2.** eda's reference claim chain is a hand copy of `names::references`' order. eda should call it instead: one `pub` in `aethelgard-ir/src/names.rs`.
6. **`1321` F1, F2.** `1319`'s harness mutant M2 no longer compiles under `1320`'s count check. Restore a mutant that measures the same case. A `DECOUPLERS` entry stated twice for one part is not refused.
7. **`1310`.** `PadEscapeExhausted`'s JSON shape has no test.

> **What this is:** (1)–(7), each with a test or a mutant that fails without it.
>
> It is NOT:
> - any change to an emitted byte;
> - a new refusal reachable from a committed specification;
> - any change to the IR beyond (5)'s one visibility change.

## 1. Build

1. **Measure first,** with predictions in `spike/1322/`: the emit baseline of every committed specification, and (2)'s memory on a long name.
2. **(1)–(7).**
3. **Re-run the baseline:** identical.

## 2. Acceptance

- **Every committed specification's emitted bytes** are identical.
- **(2)'s memory** stays bounded on a 10 kB name, with a test.
- **The CLAUDE.md gates pass, and CI is green.**

## 3. Fences

- **Fence:**
  - `crates/aethelgard-eda` and its tests;
  - `crates/aethelgard-ir/src/names.rs`, one `pub`;
  - `spike/1319/harness/` for (6);
  - `spike/1322/` and FEEDBACK.
- **Not:** the CLI, the generators, the twin, chaos; workflows; `ci/` except weight rows.
- **Running beside you:** `2310` (chaos, CLI renode/campaign); `1916` (`ci/`, CLI tests); `4408` (images).
- **The S25 window:** if it is closed, `gh-write` cannot mint. Build and commit locally, and post PUSH-READY on the bus. Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-1322/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A guard deleted as unreachable** that a hand-written topology still reaches.
- **An echo bound** that a multi-byte character splits.

## 7. Flag-and-stop conditions

- **An emitted byte moves.** Stop.
