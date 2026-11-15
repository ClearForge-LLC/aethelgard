# AET-WO-1308 — Router: per-net widths, a via-pad option, and coupled pairs

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1308-router-classes`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4. Ruled at the `1303` spike (PR #133, D8). **Cadence: build, with one measured stop (§1.4).** It starts after `1307` merges, because both change `route.rs`.

**Grounds, all measured in the `1303` spike (`AET-WO-1303.FEEDBACK.md`), on `main`:**
- **Every track is 0.2 mm.** No net has its own width, so a supply path's resistance is set by the narrowest track the router happens to lay (S4).
- **The router's via** is `VIA_NM` 0.6 mm pad and `DRILL_NM` 0.3 mm drill. That leaves a 0.15 mm ring.
  - At JLCPCB's ±0.075 mm hole-position tolerance, the supply bound made rigorous for every offset covers 2 of 16 MCU supply pins at 4 layers, against 11 of 16 with the hole concentric (F10).
  - The ring, not the copper, limits the bound.
- **No pair is routed coupled.** USB full speed's < 10 % conductor-capacitance mismatch is ruled a condition of symmetric pair routing (`1303` D2), which the router cannot provide today.

> **What this is:** the router lays declared nets at a declared width; it can use a larger via pad where the specification asks; and it routes a declared differential pair as a symmetric coupled pair. Each is judged by the gates already in place.
>
> It is NOT:
> - length tuning beyond pair symmetry;
> - impedance control (`1303` D1 (c));
> - a new placer.

## 1. Build

1. **Net classes in the specification:** `net_classes`, each with a name, a track width, an optional via pad and drill, and its nets.
   - Every value is checked against the engine floor and the declared `board.rules` (`RUL-03`, `RUL-14`).
   - A net in two classes is refused by name, and so is a class naming no existing net.
   - The specification takes the next free `schema_version`, once.
2. **The router honours them:**
   - the width per segment;
   - the via per class;
   - clearance at the larger of the two classes;
   - DRC's rules written from the same classes.

   `check drc --spec` judges per-class widths (`RUL-03`'s gate-written rules).
3. **Coupled pairs:** `diff_pairs`, each with a name, two nets, a gap and a width.
   - The router lays both conductors together: the same layers, the same via count, a constant gap, and a length difference within a stated bound.
   - The verdict reports each pair's length difference and via count per conductor.
   - A pair the router cannot couple is refused by name, never routed apart.
4. **Sentinel-01 is unchanged** unless its specification declares classes. Its committed files stay byte-identical (it declares none).
5. **The fixtures:**
   - **a 4-layer fixture** with a `power` class (for example 0.5 mm track, 0.7 mm via pad), routed, ERC and DRC clean;
   - **a fixture with one declared pair** (USB D+/D−), routed coupled and reported.

**§1.4 Measure first, and STOP if it is large: the supply coverage.** With the `power` class on the 4-layer fixture, re-run `spike/1303/s4/rbound2.py`'s coverage measurement.
- **If every supply pin is now covered by the closed-form bound** (concentric and at ±0.075 mm), record it.
- **Otherwise, record the numbers.** They decide `1303` D5 (b). This is not a stop for this work order.
- **STOP only if** the class machinery breaks an existing verdict.

## 2. Acceptance

- **Every existing board's verdicts and files are byte-identical.**
- **Per-class widths, vias and pairs are judged by DRC** from rules the gate writes, never from the board.
- **Each new refusal has a tier-1 test:**
  - a net in two classes;
  - a width under the floor;
  - a via whose ring is under the rules;
  - an uncouplable pair.
- **The CLAUDE.md gates pass.** CI is green with every shard under 20 minutes, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-03`, `INV-06`, `INV-07`; `RUL-03`, `RUL-14`.
- **Fence:**
  - `aethelgard-ir` (the specification's classes and pairs);
  - `aethelgard-eda` (`route.rs`, the rules writer, DRC);
  - the CLI's tests and fixtures;
  - `ci/`;
  - FEEDBACK.
- One PR, left unmerged. Write `~/.cache/aet-1308/pr-title-body.md`. **Push and revoke by the FUR-WO-0301 F7 route.**

## 5. Adversarial pass, full

1. **Make a class lie:** a board whose emitted width differs from its class, which the gate must catch from its own rules, never the board's.
2. **Break a pair:** asymmetric vias, a gap that widens on one layer, one conductor detoured.
3. **Starve the router:** a class too wide to escape the LQFP-144's 0.5 mm pitch, which must be refused by name with the pin.
