# aethelgard-eda

Schematic and bill-of-materials emission, and the design-rule files the KiCad gates write for themselves (`docs/architecture.md` §3, `RUL-03`). It never writes firmware. Built in `AET-WO-1300`; the decisions it implements are the architect's ruling on PR #40 (`docs/work-orders/AET-WO-1300.FEEDBACK.md`).

| Module | What it does |
|---|---|
| `schematic` | `emit(&Specification, &SolvedTopology)`: a `.kicad_sch` in KiCad 10's format (`20260306`) and a CSV bill of materials. The output is byte-identical for the same inputs. Every UUID is a sha256 of a content path. Pins are connected by the ERC rules `AET-WO-1300` S2 measured. Refusals name what is missing: an unknown part, an address the part cannot take, a net the solver did not place |
| `library` | The vendored KiCad 10.0.7 symbols and footprints in `data/kicad-10.0.7/`, each checked against its sha256 before any is used. Symbols come out flattened (`extends` resolved) and without `ki_fp_filters` |
| `rules` | The rules DRC judges by, and the gate's files: a setup block with no clearance token, a `.kicad_pro`, a `.kicad_dru`, and the `.kicad_pro` ERC judges with |
| `board` | The board under judgement. It refuses any local `(clearance …)` (pad, footprint, zone), any in-board net class, and any footprint token that waives a check, and it writes the gate's setup block over the board's |
| `sexpr` | KiCad's S-expressions. The reader refuses whatever KiCad's reader could read differently (control bytes, `#` lines, a quote touching an atom), and it caps nesting and tokens. The writer's output is a function of the tree |

**Parts with a model:**
- the MCU `STM32H753ZITx`;
- on I2C, `BME280` (address 118 or 119) and `TMP117` (72 to 75);
- on SPI, `W25Q64JV`, drawn with the `W25Q32JVSS` symbol (same SOIC-8 pinout; the bill of materials notes it, D2).

Any other part is refused, never approximated (`INV-06`).

**Licence:** `(MIT OR Apache-2.0) AND CC-BY-SA-4.0`. The code is the workspace's. The vendored KiCad files stay CC-BY-SA 4.0, and their licence sits beside them in `data/kicad-10.0.7/LICENSE.md`. Schematics and boards emitted from them fall under KiCad's design exception and carry no obligation (`RUL-11`).
