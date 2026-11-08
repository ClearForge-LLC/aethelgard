# aethelgard-eda

Schematic and bill-of-materials emission, and the design-rule files the KiCad gates write for themselves (`docs/architecture.md` §3, `RUL-03`). It never writes firmware. Built in `AET-WO-1300`; the decisions it implements are the architect's ruling on PR #40 (`docs/work-orders/AET-WO-1300.FEEDBACK.md`).

| Module | What it does |
|---|---|
| `schematic` | `emit(&Specification, &SolvedTopology)`: a `.kicad_sch` in KiCad 10's format (`20260306`) and a CSV bill of materials. The output is byte-identical for the same inputs. Every UUID is a sha256 of a content path. Pins are connected by the ERC rules `AET-WO-1300` S2 measured. Refusals name what is missing: an unknown part, an address the part cannot take, a net the solver did not place |
| `pcb` | The placed board, `.kicad_pcb` in KiCad 10's format (`20260206`), from the same parts as the schematic. Each footprint is the vendored one with the schematic's reference, value and symbol fields, the symbol's `path`, and on each pad the net KiCad's netlist gives its pin (a root-sheet label is `/<label>`, a rail its domain id, a no-connect `unconnected-(<ref>-<pin>-Pad<n>)`). A pad with no symbol pin, WSON-6's exposed pad, carries no net: TI's TMP117 datasheet (SNOSD82D §9) lets it float, and KiCad's parity check refuses a netted pad with no pin. The placer is below |
| `library` | The vendored KiCad 10.0.7 symbols and footprints in `data/kicad-10.0.7/`, each checked against its sha256 before any is used. Symbols come out flattened (`extends` resolved) and without `ki_fp_filters` |
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

Ground and unconnected nets are not counted, and ties go to the lower reference. "Beside" means the first free spot, in rings of 0.25 mm out from a target, where the part's keep-out box (its courtyard, silkscreen and reference text, every glyph as wide as KiCad's widest, `m`) clears every placed one by 0.5 mm. The target is the anchor's lowest-numbered pad on the first net they share (signal nets before rails), so a decoupling capacitor sits by the power pin its rail names. Every part is at rotation 0 on the front. The outline is a rectangle 2 mm outside every keep-out box, so every courtyard keeps more than the board-edge clearance (`rules::FLOOR_NM`).

**Licence:** `(MIT OR Apache-2.0) AND CC-BY-SA-4.0`. The code is the workspace's. The vendored KiCad files stay CC-BY-SA 4.0, and their licence sits beside them in `data/kicad-10.0.7/LICENSE.md`. Schematics and boards emitted from them fall under KiCad's design exception and carry no obligation (`RUL-11`).
