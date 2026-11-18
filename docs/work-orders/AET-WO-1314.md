# AET-WO-1314 — Router: vias that keep their annular ring at the fabricator's declared tolerances, and vias counted against a net's capacitance bound

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1314-via-tolerance`.
**Author:** Claude (architect) · **Builder:** strong tier (the `1303b` builder, reused), with a full adversarial pass.
**Phase:** P4 board capability, before `4500`. Written 2026-11-18. **Cadence: build, with one measured stop (§1.1).**

**Grounds:**
- **The default via is too small for the fabricator.** `1303b` (ruling 6974760231) priced JLCPCB's cited tolerances: hole position ±0.075 mm, finished hole +0.13/−0.08 mm.
  - A 0.6/0.3 mm via cannot keep its plated band at the worst size and position: it needs a 0.7 mm pad.
  - A 0.554/0.254 mm thermal via needs 0.66 mm.
  - That is a manufacturability finding for every board, not only for the supply gate (roadmap §7.1, owner "the next router work order").
- **Vias are free on capacitance-bounded nets.** The router adds them without cost on nets with a declared capacitance bound (`1313` Q3, #197). Every SPI `extract` failure in `1313`'s sweeps was 0.01–4.2 pF over, and followed the via count.
- **The two collide on Sentinel-03.** Its supply gate needs ground vias of at least 0.7 mm. At 0.7 its routing moves, and `IMU_SCK`'s bound rises to 31.27 pF against its declared 30 pF (measured at `1303b`, ruling 6976355662).
- **The tolerances are now declared.** `board.stackup.fab_tolerances` holds `hole_position_um`, `hole_size_plus_um`, `hole_size_minus_um` and `track_width_permille`, each `{value, source}`, and is optional.

> **What this is:**
> - the router's via pad derived from the declared drill and tolerances, so the ring holds by construction;
> - a via's capacitance counted where the net declares a bound.
>
> It is NOT:
> - a new tolerance source (the fields exist);
> - a change for a board that declares no tolerances: today's vias stay, byte for byte;
> - the test-pad rule;
> - a capacitance-aware placement step (backlog).

## 1. Build

1. **Measure first, then STOP.**
   - **The ring rule.** From the declared tolerances, the smallest pad that keeps a stated minimum ring at the worst hole size and position. Cite the minimum ring, from JLCPCB's capabilities page or IPC-6012's class, or name it as a condition. Give the formula and its worst case.
   - **For each board that declares `fab_tolerances`** (today the Sentinel-03 fixture and the power fixture):
     - the via pads the rule gives, per class;
     - the routing that results;
     - `extract`'s capacitance bound on every bounded net;
     - the supply gate's verdict.
   - **Vias on bounded nets.** Propose how the router counts a via against a net's bound: a cost, a cap, or a pre-check against `extract`'s own per-via figure. Measure each option you try on the Sentinel-03 fixture. Report `IMU_SCK` and every other bounded net, before and after.

   **STOP** with:
   - the rule and its sources;
   - the per-board table;
   - the proposed via costing, with its measured effect;
   - every byte that would move. Sentinel-01 and every board without `fab_tolerances` must not move.
2. **After the ruling:** the rule and the costing in `aethelgard-eda`, with tests. A board whose nets cannot be routed within their bounds under the rule is refused by name, with the net and its figure, never routed over its bound.

## 2. Acceptance

- **Sentinel-03 with GND on rule-sized vias:**
  - `extract` passes every capacitance-bounded net, `IMU_SCK` included;
  - its supply gate bounds every return pin, with no `SupplyViaPadTooSmall`.

  If it cannot, that is the stop's question, not a silent fail.
- **No board without `fab_tolerances` moves a byte,** Sentinel-01 and its bench package included.
- **Each guard has a mutant that a test kills.**
- **The CLAUDE.md gates pass.** CI is green, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-03`, `INV-06`, `INV-07`; `RUL-03`, `RUL-14`, `RUL-16`.
- **Fence:**
  - `aethelgard-eda` (`pcb.rs`, `route.rs`, `stack.rs`; read `extract.rs`, `supply.rs`);
  - the fixtures' boards and goldens that the ruling names;
  - tests;
  - FEEDBACK.
- **Running beside you:** `1605` (the tools image's Renode, `container/` and the twin). You share no file with it.

  You run KiCad: one container of yours at a time, under `capped-run`.
- One PR, left unmerged. Write `~/.cache/aet-1314/pr-title-body.md`.

## 5. Adversarial pass, full

- **A ring that holds at nominal but not at the worst declared size and position.**
- **A bounded net routed over its bound** because the via costing did not count a via, or counted it wrong.
- **A board with no tolerances declared whose bytes move.**
- **A constant typed in where a declared tolerance exists.**

## 7. Flag-and-stop conditions

- **§1.1.**
- **Sentinel-03 cannot meet both its capacitance bounds and its ground-via rule** under any measured costing: stop with the numbers.
