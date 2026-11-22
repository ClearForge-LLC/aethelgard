# AET-WO-1211f — FEEDBACK: the declared bulk capacitor drawn

**Branch:** `wo/aet-1211f-bulk-drawn` from `main` `9cb87b4c`. **Builder:** Claude (Opus 5.5 orchestrating), with one fresh Opus subagent for the adversarial pass. Its report was checked at source before use. Written 2026-11-22.

**Status: READY** (see §7 for CI). **The answer, in five lines:**
- **The arm draws the declared `bulk` in CVM2's place.** CVM2 is the DRV8876's `value: None` passive, from VM to GND.
  - Its reference comes from `bulk.reference`.
  - Its symbol, footprint, capacitance and rating come from the `bulk.part` power-parts row (`10u 50V GCM32EC71H106KA03`, `Capacitor_SMD:C_1210_3225Metric`).
  - Its source is printed verbatim in the BOM's note. For D1 that reads "stand-in, not a design".
- **D1 passes ERC, DRC with parity (`--spec`), `check support` and `extract`**, every findings list empty, in the pinned tools image.
- **`PassiveValueUnstated` is lifted only where `bulk` answers it:** the parts row's `bulk_unstated` supply pin to GND (`answers()`). With no `bulk` it is still refused, at `validate` (`BulkUndeclared`) and again in eda. A `bulk` on a row that states its value is refused at `validate` (`BulkNotAllowed`). eda, passed by, refuses to drop it (`PinDeviceNotDrawable`).
- **No other committed specification's byte moved:** 34 specifications and 280 files, plus d4diff's 277 cases. Of d1319's 84 cases, exactly the 17 that base refused at CVM2 moved.
- **Mutants: 17 of 17 killed.** `TIER2_MIN` is unchanged, with no weight row, as for D2. No `fab/` byte moved.

## 0. How it was made

- **Predictions first:** `spike/1211f/PREDICTIONS.md`, committed at `2b50a201` before any run. The results beside them are in `spike/1211f/RESULTS.md`.
- **Bus:** Q1–Q4 were posted at the start (`~/.cache/aet-bus/architect.md`, "from 1211f"). Built to the leans; **Q1–Q3 were ruled to them at 07:35Z** (`~/.cache/aet-bus/1211f.md`), and G1 was accepted as F1.
- **Kit (`spike/1211f/kit/`):**
  - `d1-drawn.py` writes `spike/1211f/d1-drawn.json`, the D1 fixture without `board.drawn`, as `d1_drawn()` builds it.
  - `img.sh` and `judge.sh` run the four judges in the image (1211d's kit, re-pointed).
  - `mutants.py` runs the mutants.
  - The null sweep is 1319's `spike/1319/harness/run.sh` and `compare.sh`, unchanged.
- **Evidence:** `~/.cache/aet-1211f/`: `out-base`, `out-br`, `d1-cut`, `mutants*.json` and `adv/`.

## 1. What was built

- **`crates/aethelgard-eda/src/schematic_arms.rs`:**
  - **One interpreter case** on the existing `value: None` slot. Where `answers(r, bulk_unstated)` holds and the declaration has a `bulk`, `declared_bulk()` draws that part: pin 1 on the supply end, pin 2 on GND, so a polarised row's positive is on VM. It is anchored like the row's other passives. Anything else is `PassiveValueUnstated`, as before.
  - **No row field was added.** CVM2's row already says "unstated". The interpreter now asks the declaration about it. This is the one interpreter feature beyond `1211d` §4's eight: *a declared part answers an unstated passive*. Its reason is that the value comes from the specification, never from the arm (S1). I read it as §1.2's case, not §7's stop (Q2).
  - **Both `validate` kinds are drawn:** a `capacitor` row (`Device:C`) and a `bulk_capacitor` row (`Device:C_Polarized`, `CP_Elec_8x10`, + on VMOTOR). Each has a test.
  - **`references()` claims `bulk.reference`** before any emitter-numbered reference (mutant M16).
  - **Two `belt-and-suspenders:` guards:**
    - a bulk part that is not a capacitor row (`validate` refuses it as `PassivePartUnknown`);
    - a declared `bulk` that no passive answered (`validate` refuses it as `BulkNotAllowed`), so a declared part is never dropped silently.
- **`schematic.rs`:** `Part.note` is now `Option<String>`, so a declared source can be printed. W25Q's stand-in note gives the same bytes (sweep; the adversarial emit hashes of `four-layer-power` and `motors-pwm`).
- **Tests:**
  - `crates/aethelgard-cli/tests/pin_devices_drawn_tier1.rs` (Q3):
    - `d1_is_drawn_with_its_declared_bulk`, which replaces `d1_is_refused_for_cvm2_alone`: value, footprint, symbol, both ends, the anchor, CVM1 beside it, and the BOM row byte for byte;
    - `a_declared_electrolytic_bulk_is_drawn_positive_on_vm`;
    - `an_unstated_passive_with_no_bulk_is_still_refused` (`validate`, then eda passed by);
    - `a_bulk_on_a_row_that_states_its_value_is_refused_before_eda`;
    - the IPROPI test now also holds eda's own guard directly (F13 below).
  - **The eda unit test** `a_bulk_answers_only_its_supply_to_ground`.
- **The two rows that ride along:**
  - **`1319` F2:** `parts_arms_tier1.rs`'s doc line now names the rows-against-`KNOWN_PARTS` test in `schematic_arms.rs`, not a source scan.
  - **`1211e` F13:** eda's `PinDeviceNotDrawable` on an undeclared IPROPI is **kept** as eda's own guard. It is now tested directly: D1 with `analog: []`, drawn against D1's solved topology without `validate`, is refused `PinDeviceNotDrawable` naming `` `IPROPI` ``. This mirrors `1214`'s `ConsoleBaud`. It is not labelled `belt-and-suspenders:`, because the WO asks for it to be kept as eda's own guard with a test, which it now has.

## 2. Measurement and acceptance

| what | result |
|---|---|
| CLI sweep, base `9cb87b4c` against the branch | **identical: 34 specifications, 280 files** (exit codes, printed JSON, every emitted file) |
| d4diff | **identical: 277 cases, 926 files** |
| d1319 | **17 of 84 cases move**: exactly the 17 whose base drawing ended at `PassiveValueUnstated{CVM2}`. 14 now draw and emit. `d1-no-reservoir` and `d1-vref-gnd` now draw and are then refused at `emit` by the IR's own kinds (`AdcSourceImpedance`, `TripReferenceGrounded`). `d1-res-C2` (C1 renamed C2) is refused `DuplicateReference("C2")` against the declared bulk |
| D1 in the image | ERC pass `[]`, DRC (`--spec --schematic`) pass `[]`, support pass `[]`, extract pass `[]` |
| `fab/` | no byte moved (no committed board declares a pin device) |

- **The committed D1 fixture declares `board.drawn: false`.** CLI `emit` refuses it at `board.drawn`, before and after, so the sweep is identical on D1 too. "D1 now draws" is D1 without `board.drawn` (P1.2).
- **The footprint stop (§7) does not fire:** `C_1210_3225Metric` is vendored at 10.0.7. No `aethelgard-kicad-lib` file changed.

## 3. Decisions and questions

| # | what | status |
|---|---|---|
| Q1 | **"Its source in the BOM and FAB-NOTES.md as other cited passives are" has no precedent** (measured, RESULTS M4). No declared passive's `source` and no arm `cite` reaches any emitted file. The only BOM note on any board is W25Q's stand-in. FAB-NOTES lists no parts and is written only by `verify --fab-out`, outside the fence. **Built to the lean:** the bulk's BOM `Note`, `CVM2 declared, <row document> states no value: <bulk.source>`. The source is verbatim, so D1's "stand-in, not a design" goes wherever the source is printed. No FAB-NOTES and no schematic change | **ruled (07:35Z): the BOM note only.** No precedent reaches FAB-NOTES today, and `fab.rs` is outside the fence |
| Q2 | §1.2 against §7: one interpreter case on the existing `None` slot, no row field | **ruled (07:35Z): §1.2, not a §7 stop.** Filled only from `bulk`, only for the row's `bulk_unstated` supply pin; named in §1 with its reason |
| Q3 | the tests in `cli/tests/pin_devices_drawn_tier1.rs`, outside the named fence. It holds the D1-drawn test that had to change | **ruled (07:35Z): inside the fence**, for exactly those tests |
| Q4 | the CLI sweep is identical, D1 included; `TIER2_MIN` and the weight rows do not move | measured |

## 4. Adversarial pass (§5)

A fresh Opus reviewer attacked `64a6f916`, given the code and §5, not my conclusions. Its report is `~/.cache/aet-1211f/adv/report.md`; the probes are in `adv/`. **No defect.**

- **Target 1, a value no source states:** held.
  - With `bulk: null`, the passive is refused at both layers.
  - DRV8833 with a `bulk` is refused at both layers.
  - `answers()` needs the supply-to-GND order.
  - Two DRV8876s, each with its own bulk, or one of them without, behave right at both layers.
  - Every reference collision, case variants included, is refused at both layers.
  - Non-capacitor parts are refused at both layers.
- **Target 2, a stand-in drawn as a design:**
  - The BOM holds: RFC 4180 quoting of a source with quotes and commas, read back by `fab::bom_rows`. CR, LF and non-ASCII cannot reach it (the source rule).
  - **Gap G1 (F1 below):** the JLCPCB BOM, the sheet and FAB-NOTES do not carry the note.
- **Target 3, another board's byte:** held.
  - Library-level `emit` hashes are byte-identical, base against branch, for `d2_drawn` (which the CLI sweep cannot reach), `sentinel-03`, `sentinel-01-bench`, `pair-standin`, and `four-layer-power` and `motors-pwm` (which carry W25Q's note).
  - By reading, nothing changes without a `bulk`.
- **Notes:**
  - N1, a misplaced doc comment in the arms tests: fixed.
  - N2–N4 are below.

## 5. Mutants (§1.5): 17 of 17 killed

One mutant per new line of the arm, each killed by a failing test and none by a build break. The table is in `spike/1211f/RESULTS.md` M6; the records are in `spike/1211f/out/`. M4 first broke the build (a double borrow); it was rewritten and then killed.

## 6. Findings, not fixed here

- **F1 · G1: the bulk's note is not in the JLCPCB BOM, on the sheet, or in FAB-NOTES.**
  - `fab::bom_rows` discards the BOM's note (`_note`, `crates/aethelgard-eda/src/fab.rs`). The sheet never prints `Part.note`.
  - It is unreachable today. `generate::simulable` refuses any pin device (`PinDeviceNotSimulated`), so `verify --fab-out` cannot package a board with a bulk.
  - **It may become reachable when `1211c` (#313) merges,** which moves that refusal into the twin's plan.
  - W25Q's stand-in note is dropped the same way already.
  - **Proposed row:** "before a pin-device board can be packaged, the JLCPCB BOM or FAB-NOTES carries each part's note (a stand-in's, a declared bulk's source), or a package with a stand-in is refused. Owner: the fab package, after 1211c."
  - **Ruled (07:35Z): accepted as a real hazard.** It becomes a §7.1 row owned by the verify backlog work order after 1211c, which must land before any board with a stand-in writes a fab package.
- **F2 · N2: `check()` allows two `value: None` passives on one supply pin to GND.** One `bulk` would answer both, and the sheet writer's duplicate-reference check would refuse it with a misleading kind. No row has two today. **Proposed row:** "`check()`: at most one `value: None` passive per pin row." It is not built, because the scope test sends a further round on a working safeguard to the backlog.
- **F3 · N3: the declared bulk's value is `10u 50V MPN`; the power tree's declared capacitors are `10u MPN`.** The WO asks for the value and rating to come from the row, so the rating is printed. `declared_bulk` repeats `schematic_power::two_pin`'s row push, because that helper's `note` is fixed to `None`. Folding them would touch the power tree's call sites.
- **F4 · N4 (validate, outside the fence):** `bulk` has no capacitance floor; an 18 pF C0G or a 100 nF validates. This is `1211e` F6, standing.
- **F5 · Prediction P1.1 missed:** the sweep is 34 specifications and 280 files, not 33 and 274. 4240d's `sentinel-02-bench-hse8.json` was committed after 1319 counted.

## 7. Gates, fence, reduction pass

- **Gates at `70bed14b`** (the code head; nothing in `crates/` changed after it):
  - `cargo fmt --all -- --check`: clean;
  - `cargo clippy --workspace --all-targets -- -D warnings`: clean;
  - `cargo test --workspace --no-fail-fast` (target in the repository, as CI builds it): **2673 passed, 0 failed**, 157 test binaries.
- **CI on `70bed14b` (PR #316): green.**
  - Runs: 44998447085 (`ci`), 44998447053 (`engine-image`) and 44998447100 (`ardupilot-image`).
  - 33 checks pass and 3 are skipped by the workflow; none failed.
  - Tier 2 was 202 of 202 against a floor of 201. `TIER2_MIN` is unchanged and no weight row was added.
- **The READY commit** adds the rulings (§3), this gates section, and `main` `a58a9c67` (docs only: the WO, 1216's WO, roadmap rows). Nothing is pushed after it.
- **Fence:**
  - `crates/aethelgard-eda/src/{schematic_arms.rs, schematic.rs}`;
  - `crates/aethelgard-cli/tests/parts_arms_tier1.rs` (one doc sentence);
  - `crates/aethelgard-cli/tests/pin_devices_drawn_tier1.rs` (inside the fence by the Q3 ruling);
  - `spike/1211f/`;
  - this file.

  Nothing in the IR, the generator, the twin, chaos, `verify`, `aethelgard-kicad-lib`, `ci/` or the workflows changed, and no committed specification changed.
- **Reduction pass (diff): −2 net · 1 duplicate removed · 0 dead paths removed · 2 protected kept · 1 finding below.**
  - The duplicate removed: the lift's condition was stated in an inline comment and again in `answers()`'s doc. The comment is now a pointer.
  - Protected: the two `belt-and-suspenders:` guards (§1).
  - Finding: F3, the duplicated row push.
