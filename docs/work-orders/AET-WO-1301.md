# AET-WO-1301 — Placement: `emit` writes a placed board that matches its schematic

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` after `AET-WO-1300` (cfcd44a) and `AET-WO-1400` (3518ce8), named in the kickoff.
**Branch:** `wo/aet-1301-placement`.
**Author:** Claude (architect) · **Builder:** a **strong** builder, with a full adversarial pass on determinism and parity (`docs/roadmap.md` §6). A fresh session on Furnace.
**Phase:** P1 · **Gate clause served:** clause 1, its board half up to routing. `kicad-cli pcb drc --schematic-parity` reports **no parity finding and no violation other than unconnected items** on the emitted, placed board. Routing (`AET-WO-1302`) removes the unconnected items.

**Grounds:**
- **`docs/work-orders/AET-WO-1300.FEEDBACK.md` S4 and D8** (ruled on PR #40), the shape:
  - a deterministic Rust grid or row placer over courtyards, keyed by reference;
  - footprints copied from the vendored library, with reference, value and library id equal to the schematic's;
  - the symbol's fields as hidden properties, and `(path "/<symbol uuid>")` from the shared derivation.
  - Measured there: parity matches footprints by reference, and requires the library id, the value, every symbol field and KiCad's net names (root-sheet labels prefixed `/`).
- **That FEEDBACK's build findings:**
  - B2: zones are emitted without `(connect_pads (clearance …))`, because the gate refuses it.
  - B7: the vendored footprints are format `20260206`; choose the board's version and measure it.
  - B8: WSON-6's exposed pad 7 has no symbol pin; connect it to its rail.
- `docs/architecture.md` `RUL-03` as amended (the gate's own files, `--refill-zones`, never `--save-board`), and §2.12.
- `docs/northstar.md` §4: `INV-01` (a function of the specification and the vendored data) and `INV-03`.
- **The seams:**
  - `crates/aethelgard-eda` (`schematic.rs`, `library.rs`, `sexpr.rs`, and `board.rs`, which prepares a board for the gate and is not an emitter);
  - `crates/aethelgard-cli` (`emit`, `check drc`).

> **What this is:** `aethelgard emit` also writes `<project_name>.kicad_pcb`. It holds:
> - a board outline;
> - every footprint from the schematic, placed without overlap and keyed by reference;
> - the nets, assigned to pads exactly as the schematic connects them;
> - no tracks and no filled zones.
>
> It is NOT:
> - routing, vias or zone fills, which are `1302`;
> - parasitic extraction;
> - fabrication output;
> - any change to the gate's rule files.

**Cadence:** build-only. The shape is ruled by 1300's spike. Stop only on §7. One PR, left unmerged.

## 1. Scope

1. **The board emitter, `aethelgard-eda`'s `pcb` module.**
   - **Footprints:** each one copied byte for byte from the vendored `.kicad_mod`, then given:
     - its reference, value, library id and the symbol's fields, as the schematic's;
     - its `path` from the schematic's content-path UUID derivation;
     - each pad's net, from the schematic's connectivity, with net names exactly as KiCad writes them.
   - **The exposed pad (B8):** joined to the net its datasheet names, which is the domain's ground. The source is recorded in FEEDBACK.
   - **The outline:** a rectangle on `Edge.Cuts`, sized from the placement plus a fixed margin.
   - **Format:** the board's format version is chosen and recorded (B7).
   - **The setup block:** none of the board's own beyond what the format requires. The gate writes its own.
2. **The placer.** It is deterministic and integer-only, with no floating point in any decision.
   - The MCU goes at a fixed origin.
   - Each other part goes in order of reference, beside the IC it connects to most. Ties are broken by reference.
   - A decoupling capacitor goes beside the power pin its net names.
   - Courtyards never overlap, and every courtyard stays inside the outline, with the board-edge clearance from `rules::FLOOR_NM`.
   - The choices are spelled out in FEEDBACK. They are a first placer, and routability is `1302`'s measure.
3. **`emit`** writes the board beside the schematic and the BOM. Two emissions are byte-identical.
4. **The gate runs parity.** `check drc --spec <file> --schematic <sch>` adds `--schematic-parity`, with the schematic copied into the scratch copy beside the board. Its JSON gains `schematic_parity` findings, which are judged like every other finding. Without `--schematic`, the behaviour is unchanged.
5. **Tests:**
   - **tier 1:** two emissions are byte-identical, across processes and environments as 1300's tests do. No courtyard overlaps, by the emitter's own geometry. Every schematic pin's net is on the matching pad.
   - **tier 2, in `tools`:** on the reference specification's emitted board and schematic, `check drc --spec --schematic` reports **zero parity findings, and no finding other than `unconnected_items`**. The unconnected count is recorded.
   - **The gate's 1300 tests** pass unchanged, including the adversarial ones. The emitted board passes the gate's own refusals: no local clearance, no net class and no waiver.

## 2. Invariants and fences

- `INV-01`, `INV-03`, `INV-07`. No `unsafe`. No floating point in placement decisions. No new dependency without a ruling.
- **Fence:** `crates/aethelgard-eda` (new modules, plus `schematic.rs` only to expose what the board needs), `crates/aethelgard-cli`'s `emit` and `check drc` and their tests and README, and this work order's FEEDBACK. Protected surfaces diff to empty, as in every work order. `container/` and `.github/workflows/` are protected, and so is `aethelgard-ir`'s types.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance

1. §1.5 passes in `gates` and `tools`.
2. FEEDBACK records:
   - the unconnected count;
   - the board's size;
   - the placer's rules;
   - the format version.
3. CI is green on the first push.

## 5. Adversarial pass — full

1. **Make two emissions differ:** environment, locale, iteration order, or a footprint file read twice.
2. **Make parity pass on a board that does not match its schematic:** a swapped value, a missing field, or a net renamed on one side.
3. **Make the placer overlap two courtyards, or put one outside the outline,** with a specification that has more parts or longer references.

## 5a. Reduction pass — diff-sized

Run the `reduction-pass` skill on the finished diff. One line in FEEDBACK's gates.

## 6. Upward-feedback directive

`docs/work-orders/AET-WO-1301.FEEDBACK.md`, same schema. It carries the §3.2 numbers, what `1302` should know (escape from the LQFP's 0.5 mm pitch, zones), and the gates line.

## 7. Flag-and-stop conditions

- Parity that cannot pass without changing the schematic emitter's output for an existing test.
- A placement decision that depends on anything but the specification and the vendored data.
- A change outside the fence, `unsafe`, or an image change.
- A test that can only go green by being skipped, ignored, or weakened.
- Any step that needs `sudo`, a system package, or a secret beyond the push route above.

## 8. Kickoff prompt

```
/goal AET-WO-1301: §1 is built and §3 passes in gates and tools, with zero parity findings and nothing but unconnected items on the emitted reference board, or a §7 stop is recorded in FEEDBACK; the full adversarial and reduction passes and FEEDBACK are written; one open unmerged PR from wo/aet-1301-placement.
```
