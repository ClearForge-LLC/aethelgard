# AET-WO-1211f — predictions, written before any run

Base: `main` `9cb87b4c`. Committed before the measurement runs.

## P1. The baseline sweep (`spike/1319/harness/run.sh` on a `git archive` of `9cb87b4c`)

- P1.1 The CLI sweep covers 33 committed specifications (274 files), as 1319 measured.
- P1.2 `crates/aethelgard-ir/tests/fixtures/d1-drv8876.json` is among them, and its `emit` is refused at
  `board.drawn` (it declares `board.drawn: false`, a twin stand-in), **not** at CVM2. So after the
  build the CLI sweep is identical on **all 33**, D1 included. "D1 now draws" shows only at library
  level (`d1_drawn()`: the fixture with `drawn` removed), in `d1319` and in the tier-1 test.
- P1.3 `d4diff` (277 cases) declares no `bulk` and draws no DRV8876 past validate, so it is identical
  after the build.
- P1.4 In `d1319` (84 cases), only cases built on `d1()` whose drawing today ends in
  `PassiveValueUnstated{CVM2}` move. Predicted to move (draw becomes `Ok`, or another named refusal):
  `d1`, `d1-nfault-undeclared`, `d1-no-reservoir`, `d1-imode-resistor`, the `d1-ref-*` and
  `d1-pull-R4`/`d1-res-R1` renames, the `d1-own-*` renames that draw past their own-net check, and
  `d1` in the two-device tag loop. `d1-res-C2` (C1 renamed C2) now collides with the declared bulk C2
  and moves to a reference refusal. Every `d2*`, `mixed*`, `m-*` and `p-*` case is identical. The exact
  list is taken from the base run's `summary.txt` lines before the build and recorded in RESULTS.

## P2. D1 today

- P2.1 At library level, `Drawing::of(d1_drawn())` is refused `PassiveValueUnstated {U2, DRV8876PWPR,
  CVM2}` after every other part is drawn (the committed test `d1_is_refused_for_cvm2_alone`).
- P2.2 `validate` and `solve` pass on `d1_drawn()` (spec 24, with `bulk` declared).

## P3. The bulk part's footprint

- P3.1 `bulk.part` is `GCM32EC71H106KA03` (`power-parts.json`): symbol `Device:C`, footprint
  `Capacitor_SMD:C_1210_3225Metric`, 10 µF (`rated_picofarads` 10000000), 50 V, X7S. The footprint is
  vendored (`aethelgard-kicad-lib` `FILES`, KiCad 10.0.7), so §7's first stop does not fire.

## P4. Where a cited passive's source is printed today

- P4.1 **No declared passive's `source` and no arm passive's `cite` is printed in any emitted file**
  (schematic, BOM, board) or in `emit`'s notes. The BOM's `Note` column carries only a stand-in note
  (W25Q64's), and `Part.note` is a `&'static str`. `FAB-NOTES.md` lists no parts, and is written
  only by `verify --fab-out`, which D1 (a pin device, no bench firmware) does not reach. So the WO's
  "as other cited passives are" has no precedent; where the bulk's source is printed is asked on the
  bus, not assumed.

## P5. After the build

- P5.1 D1 drawn passes ERC, DRC with parity (`--spec`), `check support` and `extract`, every findings
  list empty, in the pinned image. Risk named in advance: the 1210 bulk is far larger than the 0402
  stub 1211d judged, anchored within 2.0 mm of VM beside CVM1; an `Unroutable` or a placement
  failure is possible (1211d hit `Unroutable` on CFLY's anchor).
- P5.2 The arm change is one interpreter case on the existing `value: None` slot: no new row field
  beyond 1211d §4's eight features (the slot already exists; the interpreter fills it from the
  declaration). Stated on the bus as my reading of §1.2 against §7.
- P5.3 No `fab/` byte moves (no committed board declares a pin device).
- P5.4 `TIER2_MIN` does not move: D2's image pass is spike evidence, not a tier-2 test, and D1's
  mirrors it. No weight row is added.
