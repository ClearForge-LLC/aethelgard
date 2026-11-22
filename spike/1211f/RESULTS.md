# AET-WO-1211f — results

The predictions are in `PREDICTIONS.md` (`2b50a201`), committed before any run. Evidence is in `~/.cache/aet-1211f/`: `out-base`, `out-br`, `d1-cut`, `mutants.json` and `adv/`.

## M1. Baseline, then the null sweep (1319's kit, `spike/1319/harness/run.sh` + `compare.sh`)

- **Base:** `git archive 9cb87b4c`.
- **Branch:** `git archive 3641d3c2` (the arm and the tests).
- Each tree was built into its own target dir. The branch log shows `Compiling aethelgard-eda (…/br-src/…)`.

| part | result |
|---|---|
| CLI sweep (`validate`, `solve`, `emit` on every committed specification) | **identical: 34 specifications, 280 files** |
| `d4diff` (1211d's reviewer's cases) | **identical: 277 cases, 926 files** |
| `d1319` (1319's pin-device and kind-crossing cases) | **17 of 84 cases differ** (all below) |

**The 17 cases that move are exactly the 17 whose base drawing ends in `PassiveValueUnstated{CVM2}`** (`diff` of the two sets is empty). No other case moves.
- **Now drawn, and emitted:** `d1`, `d1-analog-unsolved`, `d1-imode-resistor`, `d1-nfault-undeclared`, `d1-own-DRV_IPROPI-U2_CPH`, `d1-own-imode`, `d1-own-M1_OUT1-U2_CPH`, `d1-own-M1_OUT1-U2_VCP`, `d1-own-M1_OUT2-U2_CPL`, `d1-pull-R4`, `d1-ref-C4`, `d1-ref-R1`, `d1-ref-U1` and `d1-res-R1`. Several of these are inputs `validate` refuses. The harness draws them against a donor's topology at library level, as it did on base, up to CVM2.
- **Now drawn, then refused at `emit` by the IR's own kinds:**
  - `d1-no-reservoir`: `AdcSourceImpedance`;
  - `d1-vref-gnd`: `TripReferenceGrounded`.
- **Refused `DuplicateReference("C2")`:** `d1-res-C2`, whose reservoir C1 is renamed C2 and now collides with the declared bulk C2.
- The harness's `d1` BOM equals the CLI `emit` BOM of `spike/1211f/d1-drawn.json` byte for byte.

## M2. D1 today (base)

`validate` and `solve` pass on D1 drawn. At library level the drawing is refused `PassiveValueUnstated{U2, DRV8876PWPR, CVM2}` after every other part (the committed test `d1_is_refused_for_cvm2_alone`). CLI `emit` of the committed fixture is refused at `board.drawn` (a twin stand-in), before and after the change.

## M3. The bulk part's footprint

`GCM32EC71H106KA03` (`power-parts.json`) is `Device:C` on `Capacitor_SMD:C_1210_3225Metric`, 10 µF, 50 V, X7S. The footprint is in `aethelgard-kicad-lib` `FILES` (KiCad 10.0.7). §7's first stop does not fire.

## M4. Where a cited passive's source is printed today (base outputs, every sweep and harness file)

- A declared pull's `source` (`"1319 harness"` in the `mixed-pull-*` cases) appears in **no** emitted file.
- `SLVS…` appears in 44 emitted files, only as the authored driver symbols' own description property, never as a passive's `cite`.
- The one non-empty BOM `Note` anywhere is W25Q64's stand-in note (106 rows).
- So the WO's "as other cited passives are" has no precedent. Asked on the bus (Q1). Built to the lean: the bulk's BOM note only.

## M5. D1 drawn through every board gate (the pinned tools image, `kit/img.sh` + `kit/judge.sh`)

`spike/1211f/d1-drawn.json` (the fixture without `board.drawn`, written by `kit/d1-drawn.py`), emitted by the branch CLI and judged under capped-run (8 GiB, swap 0 read back):

| judge | verdict | findings |
|---|---|---|
| `check erc` | pass | `[]` |
| `check drc --spec --schematic` (parity) | pass | `[]` |
| `check support` | pass | `[]` |
| `extract` | pass | `[]` |

The BOM's C2 row:
`C2,10u 50V GCM32EC71H106KA03,Capacitor_SMD:C_1210_3225Metric,Device:C,"CVM2 declared, TI SLVSDS7B DRV8876 states no value: D1 fixture stand-in, not a design: SLVSDS7B 9.1 p. 31 leaves CVM2 to system testing"`

## Predictions beside results

| # | prediction | result |
|---|---|---|
| P1.1 | 33 specifications, 274 files | **missed:** 34 specifications, 280 files (4240d's `sentinel-02-bench-hse8.json` came after 1319 counted) |
| P1.2 | the CLI sweep is identical, D1 included (refused at `board.drawn`) | **holds** |
| P1.3 | `d4diff` identical | **holds** |
| P1.4 | only the d1 cases that today end at CVM2 move; `d1-res-C2` moves to a reference refusal | **holds** as a rule: exactly that set of 17. The enumerated guess was partly wrong. It missed `d1-analog-unsolved` and `d1-vref-gnd`, and the "d1 tag loop" cases never reached CVM2 |
| P2 | D1 refused CVM2 alone; validate and solve pass | **holds** |
| P3 | the footprint is vendored | **holds** |
| P4 | no passive's source or cite is printed anywhere | **holds** |
| P5.1 | D1 passes all four gates (risk: the 1210 by VM) | **holds** on the first cut |
| P5.2 | one interpreter case, no new row field | **holds** |
| P5.3 | no `fab/` byte moves | **holds** (the sweep; no committed board declares a pin device) |
| P5.4 | `TIER2_MIN` unchanged | **holds** (no tier-2 test added) |
