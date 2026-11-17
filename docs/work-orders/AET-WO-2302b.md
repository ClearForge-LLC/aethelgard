# AET-WO-2302b — Attribution for the twin's new motor and serial findings, and the dead window code removed

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-2302b-attribution`.
**Author:** Claude (architect) · **Builder:** strong tier (the `2302` builder, reused), with a targeted adversarial pass.
**Phase:** P2/P4, twin and attribution. Written 2026-11-17 from `AET-WO-2302`'s close (PR #175, ruling comment 6940859139). **Cadence: build.**

**Grounds (`AET-WO-2302.FEEDBACK.md` §15, §18, on `main` at `ecc6be9`):**
- **Q3.** The new finding kinds have no `attribute` rule, so `attribute` reports them unexplained (`I1`):
  - `motor_anchor_unobserved`, `motor_anchor_repeated`, `motor_anchor_out_of_order`, `motor_window_past_run`;
  - `motor_edge_before_anchor`, `motor_edge_after_window`;
  - `judge_stop_missing`, `judge_stop_mismatch`;
  - `serial_wire_past_run`, `serial_injection_out_of_order`.
- **Q2.** Unreachable since `2302`:
  - evolve's `I7`;
  - chaos's `short_window` and `OUTSIDE_WINDOW_FINDING`.
- **Reduction finding 5.** Older chaos tests use `.is_err()` where several guards refuse, so a test can pass on the wrong refusal.

> **What this is:** every finding kind the twin can now produce is either attributed by a rule that cites its evidence, or named as not attributable and why. The code `2302` made unreachable is gone.
>
> It is NOT:
> - a change to what the twin observes, or to any window;
> - a trace or scenario schema change.

## 1. Build

1. **Classify each new kind first** (the table goes into FEEDBACK before any rule is written). A kind is one of:
   - **electrical:** the board, a part or the specification's declaration;
   - **software:** the firmware's behaviour, its register writes and its timing;
   - **harness:** the judge's or the run's own bound, such as `judge_stop_*` or `*_past_run`. A harness kind is not a fault of the device under test, and must never be attributed to either half.

   For each kind, cite the record that decides it: a kind 26, 29 or 30 field, or a verdict field.
   - Where the record cannot tell electrical from software, say so. The rule then reports the kind unexplained **by name**, with the missing evidence, rather than guessing (`INV-06`).
2. **The rules in `aethelgard-evolve`**, one per electrical or software kind, each:
   - citing its evidence;
   - with a seeded case or a fixture that makes it fire;
   - and one that must not fire.

   Harness kinds get an explicit rule that refuses to attribute, naming the bound.
3. **Remove** `I7`, `short_window` and `OUTSIDE_WINDOW_FINDING`, with every test that only exercised them. A future short window reintroduces them with a reachable test.
4. **Tighten** each `.is_err()` chaos test that several guards could satisfy, so it asserts the refusal by name.

## 2. Acceptance

- `attribute` reports no new kind as plain `I1`. Each is attributed, refused as harness, or reported unexplained by name with its missing evidence.
- Each rule has a firing and a non-firing case.
- The removed code has no remaining reference, and the reduction pass counts it.
- **The CLAUDE.md gates pass.** CI is green, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants:** `INV-04`, `INV-05`, `INV-06`; `RUL-20`.
- **Fence:**
  - `aethelgard-evolve` (rules, seeded cases);
  - `aethelgard-chaos` (the removals; tests);
  - `aethelgard-cli/tests/attribute*.rs` (tests only);
  - FEEDBACK.
- **Running beside you:**
  - `4400b2` (pincheck, ir validation);
  - `1313` (ir's `board` type, `pcb.rs`);
  - `1604` (CLI source: `doctor`, the MCP snapshot, the fab formats, the boot bound, `extract`'s condition);
  - `2303` (a spike, read-mostly);
  - `3401` (examples, Sentinel-01's board).

  Coordinate through the bus inbox on any CLI test file.
- One PR, left unmerged. Write `~/.cache/aet-2302b/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A harness kind attributed to the board or the firmware.**
- **A rule that fires on a kind whose evidence record is absent.**
- **A removed constant still named by a verdict or a README.**

## 7. Flag-and-stop conditions

- **A new kind whose evidence cannot be read from the trace or verdict at all.** Stop with the kind and what is missing.
