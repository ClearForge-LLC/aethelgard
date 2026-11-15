# AET-WO-4300d2 — Power tree on the board: drawn, placed, routed, judged and packaged

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4300d2-board`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4. Slice B of `AET-WO-4300d`, split at the builder's seam (PR #147, D-B0, comment 6893999770). **Cadence: build, with one measured stop (§1.4).**

**Grounds, on `main` after PR #147 (`27e504a`):**
- **Slice A is merged.** The power parts are cited rows in `crates/aethelgard-ir/data/power-parts.json`. Specification 7 declares `power`, its crossings, `mcu.bor_level` and the board temperature. Every ruled static check runs at `validate` and `solve`. The tree is in solved topology 6, with each domain's source and return pads (K5: RFBT pad 1 and RFBB pad 2 for a buck; OUT and GND for the LDO).
- **The seam:** a specification that declares `power` is refused by name (`PowerNotEmitted`) at `emit`, `place`, `route`, `fab` and `campaign`. This work order removes that refusal, gate by gate, as each gate learns the power tree.
- **`1309`** (PR #146) built through-hole pads and engine-emitted thermal-via arrays in `eda`, judged from `stack::Stack`'s `joins` and `thermal`. `Stack::of` sets both empty today (`1309` D1).
- **`1308`** (PR #142) built per-net classes. Its ruling (comment 6891478212): a supply class for MCU pins is a 0.7 mm via pad at the default width.
- **`fab` refuses a drilled pad and more than one drill tool** (`1309` D4).
- **The placer places each part beside the IC it shares the most nets with** (`4300d` spike A6; `pcb.rs`). A buck's divider, inductor and output capacitor share the rail with the MCU, so they land by the MCU, not by the buck whose loop rules they serve.

> **What this is:** a specification with a power tree emits a schematic and a board that every gate judges: ERC, DRC with parity, extraction and the fabrication package. Each regulator's passives sit by the regulator pin the datasheet names, power nets route at their classes, the thermal pad gets its engine-emitted array, and the power-in connector's through-holes join solidly.
>
> It is NOT:
> - the supply-path resistance bound (`AET-WO-1303`'s build, which this work order feeds with the K5 pads);
> - the battery-sag transient (a later analog work order);
> - a switching-loop simulation. The datasheet's layout rules are checked where a gate can, and named where it cannot.

## 1. Build

1. **Symbols and footprints** for every power part, vendored with licence and sha256 (`RUL-11`), from KiCad 10.0.7's libraries where they exist.
2. **The schematic emitter draws the tree.** Regulators, their external parts, the translator, the sense divider and its reservoir, and the power-in connector, all on the nets slice A's topology names.
   - K5 holds: RFBT pin 1 is on the domain's net and RFBB pin 2 on `GND`. An emitter that differs fails at the gate, never silently.
   - **D12 (measured):** ERC on the emitted tree, with whatever power-flag convention KiCad needs. Every finding is named, and no ERC check is waived.
3. **Anchors for placement (spike A6).** Each external part is placed by its regulator pin as the datasheet's layout section names it: Cin at VIN/PGND, CBOOT at BOOT/SW, CVCC at VCC, the FB divider at FB, the inductor at SW.
   - The anchor comes from the power tree's declared ownership (each external part belongs to its regulator in `power`). It is not a new free-form key.
   - If ownership alone cannot express the pin, propose the smallest declared field. That is a FEEDBACK question, not a fork.
   - **The layout rules a gate can check** (for example Cin's distance to VIN/PGND) are checked from the placed board, with their citations. The rest are named conditions in the verdict.
4. **Routing with `1308`'s classes.**
   - Power paths (battery → bucks → rails) use declared classes wide enough for their current, each escaping its parts.
   - Supply nets at MCU pins follow the `1308` ruling: a via-pad class at the default width.
   - The verdict names each class and its width.
5. **`Stack::of` maps the power tree** (`1309` D1):
   - each buck row's `thermal_vias` rule becomes the thermal pad's array, joined to the declared board's `GND` planes (K7);
   - the power-in connector's join is **solid** (`1309` D2);
   - DRC judges both from gate-written rules.
6. **`fab`** (`1309` D4):
   - one drill tool per declared drill, with a hit count per tool;
   - plated through-holes accepted;
   - the package names every drill;
   - FAB-NOTES carries every condition the verdict states (the `4300f` F2 rule).
7. **The sense pin.** The battery-sense net's ADC pin is solved from ADC channel data. The embedded pin table carries no ADC channels today (slice A, K4).
   - Carry the channels into `aethelgard-ir`'s pin data from the same ST open pin data the table already comes from. Do not create a dependency on `svd-hal` (`INV-01`).
   - If that is larger than a table extension, **STOP** with its size (§7).
8. **K6, the MISO round trip.** The 4210 build adds the sensor rows' schema. On top of it, add the SPI device's output delay as a cited field. The round trip then joins `4300c`'s speed rules, and `OutputDelayMissing` stays the refusal when a row has none.
9. **The Sentinel-02 power fixture** (3S–6S, as ruled at D-B9) passes every gate in the pinned image: emit, ERC, DRC with parity, extract, and `fab`.
   - Remove `PowerNotEmitted` from each gate as it passes.
   - `campaign` keeps refusing a power tree until the battery-sag work order. Say so in its message.
   - **Amended 2026-11-15 (D5, comment 6896976241; the architect's error):** `fab` was listed in error. Under `INV-03`, `verify --fab-out` needs a campaign, so the fixture cannot have a package until the battery-sag work order. `fab` is proved below `verify`:
     - drill tools and hit counts on real KiCad drill files;
     - FAB-NOTES carrying the power tree's conditions, on `1309`'s fixtures and this board.

     The package is a FEEDBACK residual.
   - **Amended 2026-11-15 (D6, comment 6897291654):** `extract` refuses, by name, a line carrying a translator pin with no stated maximum capacitance. The fixture's extraction is proved on its crossing-free copy.

**§1.4 Measure first: the placer.** Before building anchors, measure where today's placer puts the fixture's power parts (slice A's fixture, with `PowerNotEmitted` bypassed in a probe only).
- **If every external part already lands within its datasheet distance,** record it and build only the checks.
- **Otherwise,** record the distances. Anchors are then built as §1.3 says.

## 2. Acceptance

- **The Sentinel-02 power fixture passes emit, ERC and DRC with parity in the image.** Its extraction passes on the crossing-free copy, and `fab` is proved below `verify` (§1.9 as amended). Each loop rule a gate can check is checked, and each one it cannot is named.
- **Every existing board's files and verdicts are byte-identical**, apart from the specification-hash lines a version bump moves.
- **Each new refusal has a tier-1 test.**
- **The CLAUDE.md gates pass.** CI is green with every shard under 20 minutes (refit by `ci/shard.sh`'s rule; its spread clause has fired), and `TIER2_MIN` is exact.

## 3. Invariants and fences

- **Invariants and rulings:** `INV-01`, `INV-03`, `INV-06`, `INV-07`; `RUL-03`, `RUL-11`, `RUL-14`, `RUL-16`, `RUL-19`.
- **Fence:**
  - `aethelgard-ir`: the power parts' pin roles and ownership, the ADC channel data and the sensor rows' output delay;
  - `aethelgard-eda`: the emitter, the placer's anchors, routing, `stack.rs`, the rules writer and extraction;
  - `aethelgard-kicad-lib`: vendored power parts;
  - the CLI: `fab` and the gates' `PowerNotEmitted` removal;
  - tests and fixtures;
  - `ci/`;
  - FEEDBACK.
- One PR, left unmerged. Write `~/.cache/aet-4300d2/pr-title-body.md`.

## 5. Adversarial pass, full

1. **Misplace a loop part:** Cin far from VIN, the FB divider at the rail's far end, the inductor away from SW. Each must fail its check, or be named in the verdict as unchecked.
2. **Break K5:** RFBT and RFBB swapped, or the divider on another net.
3. **Starve a thermal pad:** fewer vias than the rule, a via off the pad, a relieved power-in join.
4. **Lie in the drill file:** a tool missing, or a hit count off by one.

## 7. Flag-and-stop conditions

- **A power part has no vendorable footprint** under `RUL-11`.
- **The ADC channel data needs more than a table extension** (§1.7).
- **A datasheet loop rule cannot be met** by any placement the placer can make on the fixture's board.
