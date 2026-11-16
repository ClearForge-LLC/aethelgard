# AET-WO-1311 — eda hygiene: the readers' bounds, layer names, drill positions, straps from one table, and naming the blocking pair

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1311-eda-hygiene`.
**Author:** Claude (architect) · **Builder:** strong tier, with a targeted adversarial pass (the readers are untrusted-input boundaries).
**Phase:** backlog (roadmap §7.1), written 2026-11-16. **Cadence: build only.** It runs after `AET-WO-4300g` merges, because both change `aethelgard-eda`.

**Grounds (§7.1 rows, each with its source):**
- **`3303` F4:** a layer name outside the board's layer table passes `extract` alone. Examples are `"F&B.Cu"`, or `"Top"` on a footprint line. KiCad then refuses to load the board, so `check drc` and `verify` fail late.
- **Review 2 A16; `3303` §9:**
  - `route::read` has no item cap;
  - the strict reader builds per-node path strings;
  - both are bounded only by the 1 M-token cap;
  - `connectivity` is O(segments × pads²) (`route.rs:3069`, `strict.rs:690` at review time).
- **`4300d2` A4:** the drill check compares hit counts, not positions.
- **`3300` F21:** `fab::LAYERS` and `fab::describe` both map each layer.
- **`4200` F12:** the I2C strap address lists live in both `schematic.rs` and `parts.json`.
- **`1308` A4:** when a pair's copper stops a neighbouring net from routing, the router fails closed naming the other net, not the fixed pair that blocks it.
- **Review 2 A16 (subagent claims, unconfirmed):**
  - a non-square `circle` pad is under-sized in `route::part` (`w.min(h)/2`);
  - a reference named `via1` or `track_x` is falsely refused.

  Confirm each before changing it.

> **What this is:** each row closed with a fail-before and pass-after test, the readers bounded explicitly, and no emitted byte changed on any board that passes today.
>
> It is NOT:
> - placement order (`1307` K3, its own row);
> - the `r1`/`c1` folding (`1204` review), which `AET-WO-1206` settles at `validate`;
> - any router search change.

## 1. Build

1. **Layer names:** `extract` and the readers refuse a layer outside the board's declared table, by name, before any gate.
2. **Reader bounds:** an explicit item cap on `route::read`, and on the strict reader's path strings (or no per-node strings at all), each with a stated limit and a refusal at the limit. Measure `connectivity` on the largest fixture, and on the adversarial 4 × LQFP-144 board from `1307` A4 if it is held. Bound it, or name the cost.
3. **Drill positions:** the drill check compares each hole's position and size, not only the count.
4. **`fab::LAYERS`:** one table carries each layer's description.
5. **Straps from one table:** the emitter reads I2C strap addresses from `parts.json` and keeps no list of its own; a test pins that no copy exists.
6. **The blocking pair:** when a fixed pair's copper is the blocker, the refusal names the pair.
7. **The A16 claims:** measure the `circle` pad and the `via1`/`track_x` refusals. Fix what reproduces; record what does not.

## 2. Acceptance

- Every row has a test that fails before and passes after.
- Every existing board's emitted files are byte-identical.
- **The CLAUDE.md gates pass.** CI is green on the balanced plan, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-06`, `INV-07`; `RUL-03`, `RUL-11`.
- **Fence:** `aethelgard-eda` (`route.rs`'s reader and refusals, `strict.rs`, `extract.rs`, `fab.rs`, `schematic.rs`'s strap lookup), tests, FEEDBACK.
- No version number.
- One PR, left unmerged. Write `~/.cache/aet-1311/pr-title-body.md`.

## 5. Adversarial pass, targeted

Hostile boards at each new bound:
- one item over the cap;
- a path at the string limit;
- a layer name differing only in case or by a Unicode look-alike;
- two holes swapped in position with the same count.

## 7. Flag-and-stop conditions

- **A fix changes an emitted byte on a board that passes today.**
- **The strap table and the emitter disagree on an address.** Stop with the list.
