# AET-WO-1309 FEEDBACK — router: through-hole pads and engine-emitted thermal via arrays

**Builder:** Claude (strong tier), worktree `~/aethelgard-1309`, branch `wo/aet-1309-th-thermal` from `main` `a20d332`.
**Cadence:** build.

§1 and §2 were committed before any measured run. They are not edited afterwards; corrections go in later sections.

Labels:
- **[read]**: read at source, in this repository at `a20d332` or in a held document.
- **[measured]**: run. Host cargo runs under a capped user scope; KiCad runs in the pinned image through `~/bin/capped-run`.
- **[inferred]**: neither; reasoning from the two.

## 1. Readings the work order leaves open (forks taken; each is a question for the architect)

- **D1 · Where a `thermal_vias` rule enters the specification (decision needed).**
  - **[read]** The specification cannot declare an HSOP-8 regulator or an XT30. The part tables on `main` are the sensor rows (`parts.json`: buses, identities, Renode models) and the connector rows (`connectors::PARTS`: JST GH/SH side-entry headers). The regulators table that will hold the LMR33630 is `4300d`'s build (ruled D11).
  - **[read]** The XT30 cannot be a connector row yet either. D7's ruling enforces the XT30's −20 °C floor "against the board's declared temperature", and no specification field declares a temperature.
  - **Reading taken:**
    - `aethelgard-ir` gains the rule as a type, `thermal::ThermalVias` (count as columns × rows, pitch, drill, ring, target planes, each with its citation). It has its own validation, and any part row can carry it. No row carries one on `main` yet; the regulators table will.
    - `aethelgard-eda` takes, through `stack::Stack` (the board the specification declares), the thermal pads (reference, pad, rule) and each through-hole part's plane join. `Stack::of(spec)` derives none today, so the CLI gate refuses every engine-emitted thermal via (a via other than its net's rule, by name). That is fail-closed.
    - The fixtures are therefore `eda`-level boards: footprints authored in the test from cited geometry, a `Stack` carrying the rule, and the gate's own functions and rule files. KiCad DRC runs at tier 2 on the gate-prepared copy.
    - `4300d`'s build wires `Stack::of` to its regulators table and the XT30's row; no `eda` change is then needed.
  - **Recommended:** this split, with no specification bump. A bump here would race `4210` and `4300a` for v6, and would build `4300d`'s table early, outside this fence.
- **F2 · ERC for the XT30 fixture.** ERC judges a schematic, and the schematic is emitted from the specification, which cannot name the XT30 (D1). **Reading taken:** the XT30 fixture's ERC runs on a schematic `eda` emits for a two-pin vendored symbol standing in for `Conn_01x02` (not vendored), with the XT30-shaped footprint linked. A through-hole pad changes nothing ERC reads, so this is a stand-in, named as one.
- **F3 · Fixture geometry.**
  - **XT30.** The AMASS board-type table (`~/.cache/aet-ds/xt30-amass-board-type-table-screenshot.jpg`) gives:
    - **[read]** the XT30PW: weld leg 2.0/3.0 mm, 20 A max (ΔT < 85 °C), 500 V DC;
    - no pitch, pad, drill or mechanical-hole figure.
  - **[read]** KiCad 10.0.7's `Connector_AMASS:AMASS_XT30PW-M_1x02_P2.50mm_Horizontal` (the image's library, as copied in `~/.cache/aet-4300d/kicad`) has:
    - pads 1 (roundrect, ratio 0.22) and 2 (circle), each 3.5 mm on a 1.9 mm drill, 5.0 mm apart;
    - two unnamed mechanical pads, 1.4 mm on a 1.0 mm drill, at (−8, −10) and (3, −10);
    - `*.Cu`, `(remove_unused_layers no)`.
  - The fixture takes these figures and names KiCad's footprint as their source, because the AMASS table states none of them. KiCad's file is not copied: it is CC-BY-SA and lives only in `aethelgard-kicad-lib` (`AET-WO-1304`).
  - **HSOP-8.** The plain `Texas_HSOP-8-1EP_3.9x4.9mm_P1.27mm` has eight 1.55 × 0.6 mm pads at ±2.7 mm and an exposed pad 9 of 2.95 × 4.9 mm, SMD on F.Cu **[read]**. The fixture authors that land pattern the same way.
- **F4 · The rule's five fields against TI's text.** **[read]** LMR33630 SNVSAZ9 §10.1.1 (p34) states a "minimum 4 × 3 array of 10-mil thermal vias" under the PAD, "evenly distributed", into the ground planes. Count (4 × 3) and drill (10 mil = 0.254 mm, read as the drilled hole **[inferred]**) are cited there.
  - **Pitch and ring are not stated by TI.** The rule carries them as declared values whose source says "design choice, not in the datasheet", which `RUL-14`'s sourced-bound rule permits only for a decision, not a measurement.
  - The fixture's choices:
    - 4 rows along the pad's 4.9 mm and 3 columns across its 2.95 mm, at a 1.0 mm pitch both ways;
    - a 0.15 mm ring (0.554 mm pad), the board's own via ring (`1308` F3).
- **F5 · The join, "as declared".** Two modes, solid and thermal relief, declared per through-hole part. The gate writes a `.kicad_dru` `zone_connection` rule per declared part, and refuses a pad carrying its own `zone_connect`, `thermal_*` or `padstack` token.
  - A through-hole pad of a plane's net with no declared join is refused by name (`INV-06`).
  - **The judge for relief is conservative and angle-free:** the pad joins the plane only if the whole ring of fill around its relief gap is sure copper, so any spoke angle lands in fill.
  - Engine-emitted thermal vias join solidly, as every via does (`1303` P4.7).
- **F6 · Extraction's barrel term.** **[read]** `extract.rs` bounds line capacitance only. `main` has no supply-path resistance: `1303`'s build is deferred, and its S4 bound (`ρH/(π t_p(2r − t_p))` per barrel) lives in the spike.
  - **Reading taken:** extraction bounds a through-hole pad on a line as copper on every layer it crosses: both outer pads, and the barrel through the declared maximum thickness, as it bounds a via.
  - The supply-path barrel term has no consumer on `main`. I put nothing in for it, and record the formula's place for `1303`'s build (D2 below, §6).
- **F7 · Fab.** **[read]** `cli/fab.rs` refuses a drilled pad ("every hole is a via") and a drill file with more than one tool. Both are outside this fence. A board with through-hole pads or thermal vias is therefore refused by `fab`, which fails closed. Recorded for `4300d` or `3300`'s successor.

## 2. Predictions (committed before any measured run)

- **P1. Identity.** Every existing fixture's emitted `.kicad_sch`, `.bom.csv` and `.kicad_pcb`, and `boards/sentinel-01`, are byte-identical to `main`'s binary. So are `extract`'s verdicts and the gate's rule files, for every specification on `main` (none declares a thermal pad or a through-hole part). No hash line moves; no specification version changes.
- **P2. KiCad's zone fill and a through-hole pad.** Under the gate's prepared copy with `--refill-zones`:
  - a foreign plane keeps the declared clearance from a through-hole pad on that inner layer;
  - a same-net through-hole pad gets thermal relief spokes by default (zone default: relief for pads; `1303` P4.7 found vias solid);
  - a gate-written `.kicad_dru` rule `(constraint zone_connection solid)` conditioned on the part makes it solid.
- **P3. KiCad's hole rules.** With the gate's project as on `main`, a 0.254 mm thermal via drill is reported by DRC (`hole_size` against the board minimum through-hole, default 0.3 mm), until the gate writes a rule for the array keyed on its part. With that rule, `via_diameter`, `hole_size` and `annular_width` min = max pass the declared array and report a via that differs.
- **P4. The XT30 fixture.** On 2 and 4 layers:
  - both pins route, the battery net on F.Cu or B.Cu;
  - the mechanical holes cut both planes;
  - DRC is clean with parity, and ERC is clean on the stand-in schematic (F2).
- **P5. The thermal fixture.** At 4 layers the 12 vias sit at the computed positions, all on GND, and join In1's GND plane solidly. `check`, `connectivity` and KiCad DRC pass. A 4 × 2 copy, a pitch-shifted copy and a 0.3 mm-drill copy each fail the gate's own array judge by name, and KiCad DRC catches the drill change too.
- **P6. Refusals.**
  - KiCad's `_ThermalVias` footprint against the 4 × 3 rule is refused by name: its eight 0.3 mm through-hole pads numbered 9 contradict the rule.
  - A rule whose pitch is under drill + hole gap, or whose array does not fit inside the pad, is refused by name.
  - Each has a tier-1 test.
- **P7. CI.** Every shard stays under 20 minutes. `TIER2_MIN` rises by exactly the tier-2 tests added: two predicted, the XT30 fixture's DRC/ERC and the thermal fixture's DRC.

# Build

## B1. KiCad, measured in the pinned image (P2, P3) [measured]

Every KiCad run went through `~/bin/capped-run`, with the caps read back: `memory.max` 8 GiB, `memory.swap.max` 0, `pids.max` 8192. Each ran on a copy prepared exactly as `check drc` prepares one (`kicad.rs`): the strict reading with the gate's setup, the gate's project and `.kicad_dru`, and the vendored libraries with their tables. Probes are under `~/.cache/aet-1309/p2`.

- **P3 holds.** The prepared thermal fixture's 0.254 mm thermal vias, judged with no array rule written, are each reported by KiCad 10.0.7 as `drill_out_of_range` ("board setup constraints min hole 0.3000 mm; actual 0.2540 mm"). With the gate's rule, keyed on the part's courtyard and placed after the board minimum, they pass. One via drilled 0.3 mm is then reported twice:
  - "rule 'thermal_U5_9' max hole 0.2540 mm" (`drill_out_of_range`; KiCad's kind, not `hole_size` as predicted);
  - "min annular width 0.1500 mm; actual 0.1270 mm" (`annular_width`).
- **P2 is wrong for the custom rule (B2).**

## B2. How a fill meets a through-hole pad [measured]

The plan in F5 was a gate-written `.kicad_dru` rule, `(constraint zone_connection solid|thermal_reliefs)`, per declared part.
- **KiCad 10.0.7's refill ignores it.** Measured on J1's GND pad (3.5 mm on a 1.9 mm drill) on the 4-layer XT30 fixture, under `pcb drc --refill-zones --save-board`, then plotted from the saved board:
  - four variants (relief or solid; condition `A.Parent.Reference == 'J1'` or `A.memberOfFootprint('J1')`) and the no-rule control all fill the same relief;
  - the fill's edge is a ring 2.25 mm from the pad's centre, the pad plus the zone's 0.5 mm `thermal_gap`, with 45° spokes;
  - `pcb export gerbers --check-zones` gives the same.
- **The pad's own token works.**
  - `(zone_connect 2)` leaves no fill edge between the hole (0.95 mm) and 2.6 mm: solid.
  - `(zone_connect 1)` gives the 2.25 mm ring: relief.
  - The saved board shows the zone's own setting is KiCad's default, thermal reliefs (`connect_pads` with no mode).
- **What was built instead:**
  - the engine writes the declared join as each through-hole pad's own `zone_connect` (`pcb::join`);
  - `route::check` requires exactly the declared one on every through-hole pad of the part, refuses a pad that carries one with none declared, and refuses any other value;
  - the `.kicad_dru` carries no join rule;
  - `route::read` pins the zone's `thermal_gap` and `thermal_bridge_width` to the engine's 0.5/0.5 mm, which the relief judge assumes.
- **Why this is still a rule from the specification:**
  - the pass criterion, which join, is the declared one, compared with the pad's token by the gate;
  - the gate's own join judge (`meets`) decides from copper whether the fill reaches the pad;
  - KiCad fills as the pad says, so a board whose token disagrees with the specification is refused before KiCad runs.
- **The tier-2 fixture test proves it on every run.** It reads KiCad's saved fill around J1's GND pad: relief puts all of its 98 edge vertices within 1.0–2.6 mm on the 2.25 mm ring; solid puts none there. Both hold at 2 and 4 layers.

## B3. What was built

- **`aethelgard-ir::thermal::ThermalVias`** (subagent, spot-checked; `baeaa6d`): columns × rows, pitch, drill, ring and target layers, each with a source. It refuses, in order:
  - a zero (`ThermalViasZero`);
  - an empty source (`ThermalViasUnsourced`);
  - a ring under the board's own via ring (`ThermalViaRingUnderRules`, rounding the half-micrometre up);
  - a pitch under drill + KiCad's 0.25 mm hole gap (`ThermalViaPitch`);
  - a layer that is not one the vias can join (`ThermalViaLayers`: a declared plane, or B.Cu at 2 layers).

  11 tier-1 tests. `route::HOLE_GAP_NM` now derives from `thermal::HOLE_GAP_UM`.
- **`stack::Stack`** carries `joins` (reference → solid or relief) and `thermal` (`ThermalPad`: reference, pad, the rule in nm). `Stack::of(spec)` sets both empty (D1).
- **`route::part`** reads a plated through-hole pad:
  - accepted: `*.Cu`, one round drill at the centre, a copper ring, `(remove_unused_layers no)`, and `zone_connect` 1 or 2;
  - refused by name: an oval or offset drill, a drill as wide as the pad, `np_thru_hole`, a partial layer list, its own `thermal_*`, `padstack`, `keep_end_layers` or `clearance`, `remove_unused_layers yes`, and any other `zone_connect`.
- **The router.**
  - A through-hole pad claims F.Cu and B.Cu, and keeps every via's hole the gap from its drill.
  - Its terminals are on both layers.
  - Through-hole pads and array vias are fixed copper. They cut every other net's fill, unnetted mechanical pads included, and anchor their own net's fill: a through-hole pad by its declared join, an array via at its site.
  - A pad the fill reaches itself (a through-hole pad, or a pad over an array) is not tracked to a via.
  - At 2 layers the B.Cu zone is emitted whenever ground has a via or an anchor.
- **`route::arrays`** computes each declared array's sites from the rule: centred on the pad, turned with the footprint, `pitch` apart. It refuses, by name:
  - a footprint carrying vias of its own under the pad;
  - a declared pad that is not exactly one SMD pad;
  - a pad with no net;
  - a rule layer that is not a fill of the pad's net;
  - a site whose via leaves the pad's copper (`fits`, exact for rounded pads);
  - two arrays under one pad.
- **`route::read`** reads a via whose centre lies in a declared pad as that array's, with its own size and net. Every other via must still be its net's. `check` then requires the arrays to be exactly `arrays(board)`: count, size, sites and net (`array_disagrees` names the first difference).
- **`route::check`** also:
  - keeps every drilled hole (via, thermal via, through-hole pad) the hole gap from every other;
  - judges thermal vias as copper on both outer layers, and against the edge;
  - refuses a declared reference or pad that is not a plain literal (it is quoted in the gate's KiCad rule);
  - requires each through-hole pad's join as declared.
- **`route::connectivity`** joins a track to a through-hole pad on either outer layer, and each thermal via to its pad. A fill is judged with foreign fixed copper cut out and its anchors as seeds; a thermal via that joins no fill is refused.
- **`pcb`**:
  - `write` now places the footprints and calls `write_placed`, which the fixtures call with footprints placed by hand;
  - `join` writes the declared `zone_connect`;
  - the arrays' vias are emitted after the routed vias, keyed by content;
  - the zone's relief values come from the router's constants (the same bytes).
- **`rules`:** `Rules::from_stack`, with `from_specification` delegating to it. Each array becomes a `thermal_<ref>_<pad>` rule: vias intersecting its part's courtyard, `via_diameter` and `hole_size` min = max, and `annular_width` min.
- **`extract`** (F6):
  - a through-hole pad on a line is bounded as a via is bounded, around its whole box through the maximum thickness (`through_hole_ff`);
  - an array via on a line is bounded at its own size;
  - the reader admits a pad's `drill`, its `remove_unused_layers`, and the `circle` shape (inside its box, which the bound prices).
- **Fixtures** (`tests/common/th.rs`) are footprints authored from cited geometry (F3), placed by hand, routed by `write_placed`:
  - the XT30 shape on `VBAT`/`GND` with an 0402 load, at 2 and 4 layers, with each join;
  - the HSOP-8 shape with the LMR33630 rule (4 × 3, 1.0 mm, 0.254 mm drill, 0.15 mm ring) into In1's GND plane, and into the B.Cu zone at 2 layers.
- **Tests:**
  - tier 1, `through_hole_tier1.rs` (9): the fixtures route and pass the gate's judges; one test per new refusal; the array judged from the rule (a missing via, every drill changed, an array laid by a 3 × 2 rule, one laid at 1.1 mm, the array with no rule declared); joins required and compared; extraction reads the fixtures;
  - tier 1, the reader's through-hole refusals in `route::tests`;
  - tier 2, `with_tools.rs` (2): every fixture passes KiCad DRC with no item of any kind under the gate's rules, and each join is filled as declared; one thermal via drilled other than its rule fails DRC by the gate's thermal rule.

## B4. Predictions against the results

| # | Prediction | Result | Label |
|---|---|---|---|
| P1 | Every existing output byte-identical | **Holds.** 17 specifications emitted by `main`'s binary and this branch's at the same output path. Every output file, `emit`'s stdout and `extract`'s output are identical (`~/.cache/aet-1309/bin/cmp.sh`). No hash line or specification version moved | measured |
| P2 | Default relief; a gate `.kicad_dru` rule makes it solid | **Half right.** The default is relief, but KiCad's refill ignores the custom rule; the pad's own `zone_connect` works (B2) | measured |
| P3 | 0.254 mm drill reported until the gate writes a rule | **Holds**, kind `drill_out_of_range` (B1) | measured |
| P4 | XT30 routes and passes DRC at 2 and 4 layers; ERC on a stand-in | **DRC holds**, with no item of any kind, at both counts with each join. **No ERC was run** (F2, corrected in §3) | measured |
| P5 | 12 vias at the sites, joined to In1 solidly, every judge passes; 4 × 2, pitch and drill copies refused | **Holds**: 12 vias, no unconnected item. The gate refuses the 3 × 2, 1.1 mm and 0.3 mm copies by name, and KiCad catches the drill too | measured |
| P6 | Refusals named, each with a tier-1 test | **Holds** | measured |
| P7 | Shards under 20 minutes; `TIER2_MIN` +2 | `TIER2_MIN` 101 → 103; CI below | pending |

## 3. Corrections to §1

- **F2 is withdrawn.** `eda` has no public way to emit a schematic from a list of parts; `emit` reads a specification. A stand-in schematic would need new `eda` API for a check that reads nothing through-hole. **No ERC ran on the XT30 fixture.** ERC arrives with the specification carrier (D1).
- **F5's mechanism is replaced** by the pad's own `zone_connect` (B2). The decision is unchanged: the join is declared per part, and refused when absent.

## 4. Decisions for the architect

- **D1 · The specification carrier (decision needed; §1).**
  - **Built:** everything up to `Stack`, with `Stack::of(spec)` declaring no join and no array.
  - **What it means today:** through the CLI, a board with an engine-emitted thermal array is refused (the vias are not its net's), and so is a through-hole pad on a filled net (no declared join). Both fail closed, by name.
  - **Recommended:** `4300d`'s regulators table carries a `thermal_vias` row field (`aethelgard_ir::thermal::ThermalVias`, validated by `ThermalVias::validate` at its step), and its power-in connector row carries the join. `Stack::of` maps both. Nothing in `eda` changes.
  - **The alternative:** a specification key now (v6), which builds `4300d`'s table early and races `4210` and `4300a` for the number.
- **D2 · The supply-path barrel (decision needed; F6).**
  - **[read]** `main` computes no supply-path resistance, because `1303`'s build is deferred.
  - **What extraction counts:** a through-hole pad's barrel and pads on a line, as it counts a via's.
  - **Not built:** a resistance term for a supply path through a barrel. There is nothing on `main` to add it to.
  - **Recommended for `1303`'s build:** a through-hole pad's barrel is priced as `1303` S4 prices a via's: ρ_max·H/(π t_p (2r_d − t_p)), with r_d half the pad's drill, and the pad's annulus as a radial term. A relieved pad's spokes, 0.5 mm wide across the 0.5 mm gap, are four bars in parallel at most; until priced, a relieved pad on a supply path is refused by name, never treated as solid.
- **D3 · The join is a token on the board (decision needed; B2).**
  - **[measured]** KiCad 10.0.7's refill ignores a custom `zone_connection` rule and takes the pad's own `zone_connect`. So the join must be written on the board for KiCad to fill it as declared.
  - **What the gate does:** it requires the token to equal the declaration, and refuses any other.
  - **Why this keeps `INV-07`:** the value is never read as a pass criterion; a disagreeing token fails.
  - **Recommended:** accept, and record it beside `RUL-03`'s amendment. The alternative, the gate rewriting each pad's token in its own scratch copy, adds an editor to the gate's preparation for the same verdict.
- **D4 · Fab (F7).** **[read]** `fab` refuses a drilled pad and a drill file with more than one tool, so no board with a through-hole pad or a thermal array gets a fabrication package. That fails closed. `cli/fab.rs` is outside this fence; **recommended** for `4300d`'s build or a `fab` work order: one drill tool per declared drill, and a hit count per tool.
