# AET-WO-1313c — An order-independent placement under a declared bound: canonical rank, centroid target, sidestep compaction

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1313c-placement`.
**Author:** Claude (architect) · **Builder:** strong tier (the `1313b` builder, reused), with a full adversarial pass.
**Phase:** P1 board capability, for Sentinel-03 (roadmap §5). Written 2026-11-17 from `AET-WO-1313b`'s stop (PR #208, ruling comment on that PR). **Cadence: build, no stop unless §7 fires.**

**Grounds (`1313b` FEEDBACK, measured on Sentinel-03 at 100 × 100 mm, four layers):**
- **(k)+(m)+(s)** gives **85.25 × 80.95 mm**, with every gate passing and `extract`'s least slack 9.2 pF:
  - (k) a canonical, name-free rank;
  - (m) each part's target the centroid of its anchor's pads on every shared net;
  - (s) sidestep compaction toward the MCU.
- **It is order-independent:** 10 parts shuffles, the reversed tie-break and 20 order-only renamings give one identical placement.
- **It leaves room for the parts still to come.** With the witness port, USB FS and E0 as 495 mm² of keep-out blocks, the outline is 89.20 × 86.25 mm [outline only].
- **`main` today:** 85.25 × 99.05, and under a bound its outline depends on names (F2). With the blocks it overflows (103.05 mm).
- **F1:** `main`'s placer depends on the order of the parts list on every real board, because anchored parts go in list order. Ordering them by the emitter's draw sequence (`Anchor::seq`) is byte-neutral on all 24 corpus specifications [measured].
- **The `extract` margin is geometry's, not the method's:** a 2-character reference prefix fails `extract` by 5.1 pF.

> **What this is:** `board.max_outline_um`'s placement, rebuilt from `spike/1313b/placer.patch` as reviewed code. With no bound declared, today's placer stays, byte for byte, except for F1's fix, which moves no byte.
>
> It is NOT:
> - the test-pad rule (`1313b` §2.4, not built);
> - a capacitance-aware placement step (backlog);
> - a router change;
> - bottom-side parts.

## 1. Build

1. **F1 first, as its own commit.**
   - `Anchor::seq` is carried from the emitter's draw sequence (`schematic.rs`, `schematic_power.rs`, `schematic_support.rs`).
   - `place` takes an IC's anchored parts by `seq`, never by list position.
   - `the_board_does_not_depend_on_the_parts_order` is extended to a design with anchored parts, so the test means what it says.
   - Prove it byte-neutral: the 24-specification corpus, `emit` and `generate`, byte-identical against `main`.
2. **Under a declared bound only,** in `aethelgard-eda`'s `pcb.rs`, from the prototype:
   - **(k)** the canonical rank. It is computed from footprints, ownership, `seq` and the net graph alone, never from a reference or a net name except to break a tie that remains. It is used for the anchor tie-break, the connectors' edge assignment and the bench parts. It replaces (r)'s owners-first rule, which the rank includes.
   - **(m)** the centroid target.
   - **(s)** the sidestep compaction, with its gap rule and groups. It ends by a strictly falling potential, so state that in a comment where the loop is.
   - Refusals stay as they are: `OutlineTooSmall` and `OutlineUnreachable`, by name, with nothing written.
3. **No environment switch.** The prototype's `AET_1313B` letters do not survive: the declared bound is the only switch (`INV-06`).
4. **Delete `spike/1313b/`** in this PR, once its figures are cited in FEEDBACK. Keep `rename_len.py`'s logic if the tests need it, as test code.

## 2. Acceptance

- **Sentinel-03's fixture** (`crates/aethelgard-cli/tests/fixtures/sentinel-03.json`) at four layers:
  - places at the spike's 85.25 × 80.95 mm, or the difference is explained in FEEDBACK;
  - passes every gate.
- **Order independence, as tests:**
  - parts shuffles and the reversed tie-break under a bound, as unit tests;
  - order-only renamings, as an identity test.
- **Size samples:** reference prefixes at two or three lengths, as `rename_len.py` ran them. Each sample either passes every gate or is refused by name. A sample that fails `extract` is a refused board, not a wrong one: report each sample's outline and least slack in FEEDBACK. No acceptance claims a margin the method does not hold.
- **No bound declared:** every existing board and golden is byte-identical, the 24-specification corpus included.
- **Only the fixture's board bytes move.** The bench package does not move: no bound is declared on Sentinel-01.
- **The CLAUDE.md gates pass.** CI is green, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-03`, `INV-06`; `RUL-03`, `RUL-16`.
- **Fence:**
  - `aethelgard-eda` (`pcb.rs`; `Anchor::seq` in the three schematic files);
  - its tests and the fixture's goldens;
  - `spike/1313b/` (deleted);
  - FEEDBACK.
- **Running beside you:**
  - `4400d` turns the IMU's footprint in `pcb.rs` (its keepout zones too) and bumps the specification to 16. It merges first. Merge `main` after it lands and re-gate before READY.
  - `4400c2` (the twin).

  You run KiCad: one container of yours at a time, under `capped-run`.
- One PR, left unmerged. Write `~/.cache/aet-1313c/pr-title-body.md`.

## 5. Adversarial pass, full

- **A name or list position that still decides a placement under a bound:** find one, or show by test that none does.
- **Compaction that breaks a placer gap,** enters an approach strip, or leaves the outline (the spike's `gaps.py`, as a test).
- **A loop rule (a buck's input capacitor, CBOOT) that fails** after compaction.
- **A board that passes with no bound declared and moves bytes.**

## 7. Flag-and-stop conditions

- **The fixture does not place within 2 mm of the spike's outline,** or does not pass every gate: stop with the difference measured.
- **F1's fix moves any byte:** stop.
