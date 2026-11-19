# AET-WO-1315 — FEEDBACK

Labels: **[measured]** by a run named beside it; **[read]** at the source named beside it; **[inferred]** from arithmetic or reasoning.

## Status: built; READY waits on one ruling, D1 (§4)

- **Everything in §1 is built.** The hot-plug bulk capacitor is drawn, placed and judged by the loop gate. Each control pin is drawn from its declaration. The hot-plug reference is claimed. The verdict no longer says "not drawn". `Stack::of` refuses a partial tolerance block at every reader.
- **Every board without a power tree is byte-identical to `main`.** That covers 34 of the 36 specifications, Sentinel-01 among them; Sentinel-01's package is covered in §3.2.
- **The two power boards pass every gate inside their bounds** [measured, tier 2 in the image].
- **D1, decision needed (§4).** Drawing C1 where §1.3 says (in U10's group) reshapes Sentinel-03's placement: 85.25 × 80.95 becomes **96.50 × 69.35 mm**. Every size sample of 2 to 6 characters, which `main` places, is now refused `OutlineUnreachable`. The cause is the anchor, not its distance bound. Reported to the architect on the bus.
- **No schema change, no stop condition hit.** `aethelgard-ir` moved only inside the carve-out (§2.5).

## 0. Prediction (§1.0), committed before the first run of the new emitter

The baseline is `main` `cd43b207`'s release binary, swept over all 36 JSON files under `examples/` and `crates/**/tests/fixtures/` (`emit`, `solve`, `generate`) into `~/.cache/aet-1315/base`.

| Figure | `main` | Predicted | Label |
|---|---|---|---|
| Loop rules on the power fixture (`board_tier2.rs:1186`) | 17 | **18**: one more, C1's, anchored to U10's VIN and PGND | inferred |
| Loop rules on Sentinel-03 | as measured on `main` | **+1**, the same rule | inferred |
| Sentinel-03 outline under its 100 × 100 mm bound | 85.25 × 80.95 mm | **grows in one axis, by under 10 mm**, still inside the bound: CP_Elec_8x10's courtyard (about 10 × 9 mm) is the largest passive on the board and joins U10's group | inferred |
| Sentinel-03 least `extract` slack | 9.2 pF | **9.2 ± 0.5 pF**: C1 is on `VBAT`, no judged I2C or SPI net; slack moves only if placement moves a judged line | inferred |
| Emitter-numbered capacitors on both power boards | from `C1` | **each moves up by one** below the first declared number (`C1` now claimed); none above it moves | read (`schematic.rs:643`, `Builder::reference`) |
| Every board without a power tree, Sentinel-01's package | — | **byte-identical**: every changed path reads `spec.power` or a `Stack::of` error that no committed specification raises | read |
| Controls drawn from the declaration (step 4 alone) | — | **no byte moves** on either power board: both declare EN `input`, PG `open`, OE `vcca`, which is what the emitter drew | read (WO §0, both fixtures) |
| Pinned figures that move | — | `board_tier2.rs` loop count; `outline_tier1.rs:330` outline and the name-free placement digest; the `ANSWER 1313c-fixture` line; any test naming an emitter-numbered capacitor of a power board | inferred |

**How the prediction did** [measured, §3]:
- **Loop count: right.** 17 became 18 on the power fixture; Sentinel-03 has 16, the full power fixture 20.
- **The renumbering: wrong.** Every emitter-numbered capacitor moves one slot along the free numbers, not only those below the first declared number. On Sentinel-03, C9 becomes C18 and C43 becomes C44 (§2.4).
- **The outline: wrong, in size and in shape.** It grew 11.25 mm in x and shrank 11.6 mm in y, against a predicted growth of under 10 mm in one axis.
- **Slack: just outside the band.** It went from 9.2 to 8.6 pF, against a predicted 9.2 ± 0.5.
- **Controls: right.** Both power boards are byte-identical after step 4 alone (§3.1).

## 1. Commits, in build order

| Commit | Step | Sweep against `main` (36 specifications; `emit`, `solve`, `generate`; unmasked) |
|---|---|---|
| `088011db` | §1.0 the prediction | — |
| `0f3fec05` | §1.2 the two library files | identical, 36/36 |
| `17e37a9f` | §1.6 `Stack::of` | identical, 36/36 |
| `610256df` | §1.4 controls from their declarations | identical, 36/36, both power boards included |
| `009c43e3` | §1.1, §1.5 `references`; the verdict strings | only the two power boards move (renumbering) |
| `cfabab05` and later | §1.3 the draw, the re-pins, then review-pass fixes | only the two power boards move |

The sweep script is `~/.cache/aet-1315/sweep.sh`; each binary writes into one fixed scratch path, so no embedded path differs. `main`'s binary was swept twice and came out byte-identical both times, so the comparison is deterministic.

## 2. What is built

### 2.1 The library files (§1.2)

- **`symbols/Device.kicad_symdir/C_Polarized.kicad_sym`:** sha256 `e0371fff…cc87a`, git blob `5e34d52e469f0a9e2fc017d87f285ccde14549f8`.
- **`footprints/Capacitor_SMD.pretty/CP_Elec_8x10.kicad_mod`:** sha256 `03187d2e…40cc3f`, git blob `48b4961d557fd1bdc14a7262a1b635e4c0084a2e`.
- **Provenance:** fetched raw from GitLab at tag `10.0.7`. GitLab's files API reports the same blob ids at `ref=10.0.7`, and `git hash-object` on the bytes agrees.
- **The lists:** `PINS`, `FILES` and the README table go from 50 to 52. `workspace_shape`'s list of files gains the two.
- **The `+` terminal is 1 in both files** [read, and tested]:
  - The symbol draws its `+` (two crossing strokes) at (−1.27, 2.286), beside pin 1 at (0, 3.81). Pin 2 is at (0, −3.81), with the filled plate.
  - The footprint's silkscreen `+` is at x = −5 mm, beside pad 1 at −3.25 mm; pad 2 is at +3.25 mm.
  - `the_bulk_capacitors_positive_terminal_is_where_the_vendored_files_mark_it` reads both from the vendored files. The row's `pins` is `{}` and carries no polarity role, so the emitter holds the constant `BULK_POSITIVE = "1"`, cited to these files. A role in the table would have been a schema change.

### 2.2 The hot-plug parts (§1.3, `schematic_power.rs`)

- **`bulk(table, hot_plug)`** returns the row when it is a `BulkCapacitor` on the board's side. Anything else is `PowerNotDrawable` by name:
  - a TVS (tested on a test-only table, the embedded one plus a `TVS_TEST` row);
  - a ceramic row;
  - `side: pack`, which validate refuses as `BatteryInputUndamped`, refused again here for an unvalidated caller;
  - a part the table lacks.
  Every declared part is judged before any is drawn.
- **Drawn:** `C1` with value `47u UCZ1H470MCL1GS`, using the row's symbol and footprint, pin 1 on `VBAT`, pin 2 on `GND`.
- **Placed:** as the last part of the first battery-fed regulator's sequence, anchored to its VIN and PGND pads (IN and GND for an LDO). With no battery-fed regulator it is drawn unanchored.
  - **The loop rule is 10.0 mm**, an engine choice set after measuring, which its text says (the `CIN_BULK_NM` precedent, `4300d2` D3).
  - **Measured** [`loops::judge` on the placed board]: 9.852 mm from + to VIN and 4.659 mm from − to PGND, the same on Sentinel-03 and on the power fixture. CP_Elec_8x10's pads are 6.5 mm apart and VIN and PGND are adjacent pins, so one pad is necessarily far.
  - **With the bound lifted to 30 mm, the placement is unchanged** [measured]: the bound does not drive the placer. A second declared bulk capacitor (a probe, not committed) lands at 9.143 / 7.975 mm and passes.
- **Draw order follows the declaration, never the parts list:** placement goes by `Anchor::seq`. `under_a_bound_the_board_does_not_depend_on_the_parts_order` is unchanged and passes.

### 2.3 Control pins (§1.4, `schematic_power.rs` `controls`)

- **Straps:** `input` goes on the regulator's input rail, `open` is a no-connect, and `vcca` and `vccb` go on the translator's A and B rails.
- **Wires:** drawn as a label on the declared net, only when a declared connector pin or test point ends that net. The set of ended nets is computed in `schematic.rs`, the one place pinned to read `declared_connectors` (`solved_devices.rs` `the_eda_reader_is_exactly_the_schematic`).
- **Refused by name** (`PowerNotDrawable`), none falling back to the old tie:
  - a row's control pin with no entry;
  - an entry for a pin the row does not list, or for one pin twice;
  - a strap the row does not allow, or one this part has no rail for;
  - a wire to a rail (`GND`, `VBAT` or a domain, any case);
  - a wire to a net nothing else ends;
  - both a strap and a wire, or neither.
- **One belt-and-suspenders branch:** a wire on a pin the row does not let be wired. `validate` refuses it (`ControlPinUnwired`), and every held row's control pins are wirable.

### 2.4 The reference claimed (§1.1, `aethelgard-ir` `power::references`)

- **The change:** each `input.hot_plug` reference is listed after `input.reference`. The emitter's claim list and validate's name checks read that one list.
- **Renumbering** [measured, the BOMs of the `009c43e3` sweep]. Every emitter-numbered capacitor takes the next free number, so all of them move one slot:
  - **Sentinel-03, before:** C1–C9, C18, C19, C28–C43. **After:** C2–C9, C18, C19, C28–C44.
  - **Power fixture, before:** C1–C9, C18, C19, C28, C29, C32–C39, C42–C45. **After:** C2–C9, C18, C19, C28, C29, C32–C39, C42–C46.
- **Name checks on a hot-plug reference** [`a_hot_plug_reference_the_emitter_refuses_is_refused_at_validate`]. In each case below, `validate` refuses and the emitter refuses too:
  - `#C1` gives `ReservedName`;
  - `c1` gives `DuplicateReference "C1 and c1"`;
  - `C10` (a declared CIN) gives `DuplicateReference`;
  - a test point's `TP9` gives `TestPointReferenceTaken`, while the emitter refuses it as `DuplicateReference`.
- **`C01` is refused by validate** [probe].
- **`unconnected-C1` is accepted by both** (F2, §5).

### 2.5 The verdict (§1.5, `power.rs`)

- **Removed:** the note that "the board emitter does not draw them … (FEEDBACK D5)", and the wire condition's clause "and the board does not draw it (FEEDBACK D5)".
- **Kept:** "neither the net's driver nor its level is judged".
- **Tested:** `ir/tests/power.rs` asserts that neither string appears, and that the wire condition ends with the kept clause. Nothing else in the repository (tests, goldens, docs outside `docs/work-orders`) held either string [grep].

### 2.6 `Stack::of` (§1.6)

- **Failing first:** `every_reader_of_a_board_refuses_a_partial_tolerance_block_by_name` failed on `main`. `check drc --spec` with `via_ring_um` removed answered `BoardUnreadable` ("a via Some(750000)/Some(300000)…"), because the board was read with the engine's via.
- **The shape:** `Stack::of` returns `Result<Stack, &'static str>`, the field path `RingRule::of` names. Every production caller refuses with `emit`'s kind, id and message (`FabToleranceUndeclared`, exit 2) through one CLI helper, `undeclared`: `check drc`, `check support`, `extract`, and `fab.rs` derive. `fab.rs` `notes` takes the stack derive already holds.
- **The test** covers `check drc`, `check support` and `extract`. Each refuses before any tool is looked for, so it stays tier 1.
- **`rules.rs` is outside the eda fence list but is a named caller** (WO grounds). `Rules::from_specification` returns the same `Result`, which is the only change there. Its production callers now build `Rules::from_stack` from the stack they hold.
- **Tests unwrap:** 42 call sites, mechanically.
- **Every production board reader takes its stack from `Stack::of`** [grep of `route::read`, `route::declared`, `extract::read*` and `bound::read` in `cli/src` and `eda/src`].
