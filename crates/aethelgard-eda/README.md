# aethelgard-eda

Schematic and bill-of-materials emission, and the design-rule files the KiCad gates write for themselves (`docs/architecture.md` §3, `RUL-03`). It never writes firmware. Built in `AET-WO-1300`; the decisions it implements are the architect's ruling on PR #40 (`docs/work-orders/AET-WO-1300.FEEDBACK.md`).

| Module | What it does |
|---|---|
| `schematic` | `emit(&Specification, &SolvedTopology)`: a `.kicad_sch` in KiCad 10's format (`20260306`) and a CSV bill of materials. The output is byte-identical for the same inputs. Every UUID is a sha256 of a content path. Pins are connected by the ERC rules `AET-WO-1300` S2 measured. Refusals name what is missing: an unknown part, an address the part cannot take, a net the solver did not place |
| `pcb` | The placed and routed board, `.kicad_pcb` in KiCad 10's format (`20260206`), from the same parts as the schematic. Each footprint is the vendored one with the schematic's reference, value and symbol fields, the symbol's `path`, and on each pad the net KiCad's netlist gives its pin (a root-sheet label is `/<label>`, a rail its domain id, a no-connect `unconnected-(<ref>-<pin>-Pad<n>)`). A pad with no symbol pin, WSON-6's exposed pad, carries no net: TI's TMP117 datasheet (SNOSD82D §9) lets it float, and KiCad's parity check refuses a netted pad with no pin. The placer is below; the copper is `route`'s |
| `route` | The router (`AET-WO-1302`): tracks on F.Cu and B.Cu, 0.6/0.3 mm vias, and an unfilled ground zone on B.Cu, from a deterministic integer grid search. Before it returns, its own exact checks judge every pair of copper items for clearance and every net for connectivity, and `read` reads a written board back for the same checks. A board it cannot complete is refused (`EmitError::Unroutable`), naming the net. Its rules are below |
| `extract` | Extraction's first step (`AET-WO-1305`): an upper bound on each I2C line's capacitance from a routed board's copper, for `aethelgard extract`. `read` refuses a board the bound does not hold for (copper `route::read` would not see, labels `route::connectivity` refuses, clearances `route::check` refuses) before any estimate. Each term is the energy of a trial potential that is 1 on the copper and 0 at the router's clearance, with the permittivity at its FR-4 maximum, so it bounds from above; every constant is derived from the router's geometry and cited in the module, and a test pins each to its formula |
| `library` | The vendored KiCad 10.0.7 symbols and footprints, held as bytes by `aethelgard-kicad-lib`, each checked against the sha256 pinned here before any is used. Symbols come out flattened (`extends` resolved) and without `ki_fp_filters` |
| `rules` | The rules DRC judges by, and the gate's files: a setup block with no clearance token, a `.kicad_pro`, a `.kicad_dru`, and the `.kicad_pro` ERC judges with |
| `board` | The board under judgement. It refuses any local `(clearance …)` (pad, footprint, zone), any in-board net class, and any footprint token that waives a check, and it writes the gate's setup block over the board's |
| `parity` | What `check drc --schematic` refuses before KiCad's parity check: a schematic with a hierarchical sheet or a name starting `unconnected-`, and a board with a `board_only` footprint, a `no_connect` pad on a real net, or one `unconnected-` net on two pads. KiCad's parity check was measured passing each unmatched |
| `sexpr` | KiCad's S-expressions. The reader refuses whatever KiCad's reader could read differently (control bytes, `#` lines, a quote touching an atom), and it caps nesting and tokens. The writer's output is a function of the tree |

**Parts with a model:**
- the MCU `STM32H753ZITx`;
- on I2C, `BME280` (address 118 or 119) and `TMP117` (72 to 75);
- on SPI, `W25Q64JV`, drawn with the `W25Q32JVSS` symbol (same SOIC-8 pinout; the bill of materials notes it, D2).

Any other part is refused, never approximated (`INV-06`).

**The placer** (`pcb::place`) is integer-only, and the board is a function of the parts alone (`INV-01`), whatever their order:
1. the MCU's origin goes at (100 mm, 100 mm);
2. every other IC, in reference order, goes beside the placed IC it shares most nets with;
3. every passive, in reference order, goes beside the IC it shares most nets with. A decoupling capacitor goes beside the IC it was drawn for.

Ground and unconnected nets are not counted, and ties go to the lower reference. "Beside" means the first free spot, in rings of 0.25 mm out from a target, where the part's keep-out box (its courtyard, silkscreen and reference text, every glyph as wide as KiCad's widest, `m`) clears every placed one by 0.5 mm, or by 1 mm when both are ICs, so that the router has room between their pads (`AET-WO-1302`). The target is the anchor's lowest-numbered pad on the first net they share (signal nets before rails), so a decoupling capacitor sits by the power pin its rail names. Every part is at rotation 0 on the front. The outline is a rectangle 2 mm outside every keep-out box, so every courtyard keeps more than the board-edge clearance (`rules::FLOOR_NM`).

**The router** (`route::route`) is integer-only, and its copper is a function of the placed pads alone:
- **Grid:** 0.125 mm, which divides the LQFP's 0.5 mm pitch, so every LQFP pad centre is a node. Moves are axis-aligned on F.Cu and B.Cu, with a via between; the search is A* over (layer, node, entering direction), with a cost per step (B.Cu three times F.Cu's), per bend and per via. Ties go to the lower state index.
- **Spacing:** every track (0.2 mm) and via keeps 0.2 mm from foreign copper, pads with no net included, and 0.5 mm from the board edge (KiCad 10.0.7's default edge clearance, measured); holes keep 0.25 mm. On the grid each reach carries a 10 µm margin, which a compile-time assertion proves covers the chord between two nodes.
- **Escape:** on F.Cu no track crosses a footprint's pad field (the box round its pads) except inside its own pad, so every pad is left straight out. Nodes straight out from a pad not yet routed cost more the nearer they are, for a track or for a via whose copper would reach them.
- **Order:** ground first, then every net with two or more pads (`unconnected-…` aside) by pad count, then name. A net grows as a tree from its first pad (by reference, then pad number), joining the nearest pad next. Each ground pad is joined by F.Cu to a via in the largest piece of the B.Cu zone, or to ground copper that already has one; the zone joins the vias.
- **Rip-up:** when a connection finds no free path, a search that may cross other nets' copper names the nets in the way; they and the stuck net are taken up and routed again, the stuck net first, and every node contested costs more from then on. A zone split into pieces holding ground vias takes up the nets that fence it, and ground is stitched again after them. At most 500 nets are taken up; past that the net is named and routing fails closed.

**Licence:** `MIT OR Apache-2.0`. KiCad's vendored files live in the separate `aethelgard-kicad-lib` crate (CC-BY-SA 4.0, with KiCad's design exception; its licence sits beside them). Schematics and boards emitted from them carry no obligation (`RUL-11`).
