# AET-WO-1310 — Router: a pin-centred escape for pads narrower than the grid allows

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1310-escape`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4, router. From `AET-WO-4300d2`'s close (PR #149, comment 6899426595). **It runs before `AET-WO-4220b`.** **Cadence: spike, then stop for review, then build.**

**Grounds:**
- **The grid and the floor.** The router lays copper on a 125 µm grid (`crates/aethelgard-eda/src/route.rs`, `PITCH_NM`), and the engine floor is 200 µm track and clearance (`FLOOR_UM`, `RUL-03`).
- *Corrected 2026-11-16 (the spike, PR #160):* the sub-grid pad on the power fixture was the ICM-20948's, not the TXU0304's. The grounds below are left as first written; the scope is unchanged.
- **What `4300d2` measured.** On the full power fixture, the TXU0304's pads (0.4 mm centres, 0.2 mm gaps) can be escaped only by a track centred exactly on the pad, and the grid rarely puts one there. `4300d2` therefore closed with the crossing's board waiting. Its FEEDBACK holds the numbers.
- **The same limit is coming.** Sentinel-02's sensors are fine-pitch LGA parts (the ICM-42688-P and the BMP581; their land patterns are held, `4220b`). The DShot and USB parts that follow are fine-pitch as well.

> **What this is:** a pad whose pitch the grid cannot escape at the floor gets an off-grid escape segment on its own axis, from the pad's centre to the first grid point outside its neighbours' clearance. It is laid by the router and judged by the same DRC. Every existing board stays byte-identical, because the escape applies only to pads the grid cannot reach.
>
> It is NOT:
> - a finer global grid (measure its cost in the spike, but do not adopt it here);
> - a change to the floor, which is `RUL-03`'s;
> - via-in-pad.

## 1. Spike (measured, then stop)

1. **Which pads the grid cannot escape**, by pitch, width and rotation, at the 200 µm floor:
   - the TXU0304;
   - the ICM-42688-P and BMP581 land patterns (held, `4220b`);
   - one 0.5 mm QFN.
2. **Three candidates, each measured on those parts:**
   - the pin-centred off-grid escape;
   - a local half-pitch sub-grid inside each part's courtyard;
   - a global 62.5 µm grid (its router time on today's boards).
3. **For each candidate:**
   - DRC with parity;
   - byte-identity of existing boards;
   - router wall time (`1307` A4's bound);
   - what the strict reader and `extract` must learn.

**STOP** with the table and a recommendation.

## 2. Build (after the ruling)

The ruled candidate, with tier-1 tests per pad class. The TXU0304 board from `4300d2` routes and passes every gate it reaches. Existing boards stay byte-identical.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-06`; `RUL-03`, `RUL-14`.
- **Fence:** `aethelgard-eda` (the router, the reader as needed, `extract` as needed); tests and fixtures; `ci/`; FEEDBACK.
- One PR, left unmerged. Write `~/.cache/aet-1310/pr-title-body.md`.

## 5. Adversarial pass, full

- An escape that crosses a neighbour's clearance.
- An escape that leaves the pad off its axis.
- An escape on a pad the grid could already reach, which would break byte-identity.
- A rotated part.

## 7. Flag-and-stop conditions

- **No candidate escapes the held LGA parts at the 200 µm floor.** The floor itself is then the question for a ruling: fabricators' 0.1 mm classes exist, and the floor is `RUL-03`'s.
