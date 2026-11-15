# AET-WO-1309 — Router: through-hole pads and engine-emitted thermal via arrays

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1309-th-thermal`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4. Ruled at the `4300d` spike (PR #137, D7 and D8). **Cadence: build.** It starts after `1308` merges, because both change `route.rs` and `pcb.rs`.

**Grounds, from the `4300d` spike (`AET-WO-4300d.FEEDBACK.md`), on `main`:**
- **The router refuses through-hole pads** (`route.rs:1876`). The ruled power-in connector, the AMASS XT30PW-M at 15 A, is through-hole, and so are most battery connectors on flight controllers.
- **The LMR33630's exposed pad** needs a thermal-via array: TI recommends a 4 × 3 minimum of 10-mil vias (p34). KiCad's ThermalVias footprint has 8 vias at 0.3 mm, which meets neither TI's rule nor the router.

> **What this is:**
> - through-hole pads routed and judged like SMD pads, on every copper layer;
> - a thermal-via array on a pad, emitted by the engine from the part row's cited rule;
> - both judged by DRC and by extraction.
>
> It is NOT:
> - press-fit or slotted holes beyond what the vendored footprints need;
> - thermal simulation.

## 1. Build

1. **Through-hole pads.**
   - **The reader and router** accept a pad whose layers span the stack, with drill and annular ring from the footprint.
   - **Each copper layer** has its own keep-out, at its own clearance.
   - **A plane joins** through a thermal relief, or solid, as declared.
   - **`check` and connectivity** treat the pad on every layer.
   - **Extraction** counts the barrel on supply paths, consistently with `1303`'s via bound.
2. **Thermal-via arrays.**
   - **The rule:** a part row may declare `thermal_vias`: count, pitch, drill, ring and target layers, each cited to the datasheet's layout section.
   - **The emitter** places the array on the pad, and every via joins the pad's net and plane.
   - **DRC** judges it from gate-written rules.
   - **Refusals:** a footprint whose own vias contradict the rule is refused by name, and so is a rule the via geometry cannot meet at the declared board rules.
3. **Fixtures:**
   - **XT30:** a board with an XT30-shaped two-pin through-hole connector on the battery net, routed, with ERC and DRC clean.
   - **Thermal pad:** an HSOP-8 exposed-pad part with a 4 × 3 array, routed into a 4-layer ground plane, DRC clean.
4. **Every existing board stays byte-identical.**

## 2. Acceptance

- **Through-hole and thermal-via copper are judged by DRC** from rules the gate writes, never from the board.
- **Each new refusal has a tier-1 test.**
- **The CLAUDE.md gates pass.** CI is green with every shard under 20 minutes, and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-03`, `INV-06`, `INV-07`; `RUL-03`, `RUL-14`.
- **Fence:**
  - `aethelgard-ir` (the part rows' `thermal_vias`);
  - `aethelgard-eda` (`route.rs`, `pcb.rs`, the rules writer, extraction's barrel term);
  - tests and fixtures;
  - `ci/`;
  - FEEDBACK.
- Run containers with `~/bin/capped-run`. One PR, left unmerged. Write `~/.cache/aet-1309/pr-title-body.md`.

## 5. Adversarial pass, full

1. **A through-hole pad that shorts on an inner layer** the router did not check.
2. **A thermal array that DRC passes** but that violates the cited rule (count, pitch or drill).
3. **A plane that floods a through-hole pad of another net.**
