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
