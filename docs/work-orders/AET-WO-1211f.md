# AET-WO-1211f — The declared bulk capacitor drawn: D1 (the DRV8876) passes every board gate, and `PassiveValueUnstated` is lifted only where `bulk` answers it

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1211f-bulk-drawn`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4, before the baseline. Written 2026-11-22. **Cadence:** a measurement, then a small build. **After `1211e`** (#306) and `1319` (#310), both merged.

**Grounds:**
- **`1211d` S1, ruled (b):** SLVSDS7B states no value for the DRV8876's VM bulk capacitor (CVM2). The arm refuses `PassiveValueUnstated`, so D1 is drawn only up to that refusal.
- **`1211e` built the declaration:** `pin_devices[].bulk {reference, part, source}`, required where the row leaves the bulk value unstated. The part is a `capacitor` or `bulk_capacitor` row rated above the domain's maximum. D1's fixture declares a stand-in, labelled as not a design.
- **`1211e` F3:** "the eda arm draws `bulk`, lifting `PassiveValueUnstated{CVM2}`", the follow-up after `1211d`.
- **`1319`:** the arms are now rows (`ARMS`) and one interpreter (`arm()`) in `crates/aethelgard-eda/src/schematic_arms.rs`. The bulk capacitor is drawn there.
- Read `1211d` FEEDBACK (S1, the D1 stub in `spike/1211d/draw/kit/d1-stub.sh`), `1211e` FEEDBACK (F3, F13) and `1319` FEEDBACK (§4's hazards, F2, F3) first.

> **What this is:**
> - **The arm draws the declared bulk capacitor** on the DRV8876's VM pin, from the part row `bulk.part` names: its value, rating and footprint come from that row, its reference from `bulk.reference`, and its source in the BOM and `FAB-NOTES.md` as other cited passives are.
> - **`PassiveValueUnstated` is lifted only for a passive the specification's `bulk` answers.** A row with an unstated passive and no `bulk` is still refused by name, at `validate` (`BulkUndeclared`) and again here.
> - **D1 passes** ERC, DRC with parity (`--spec`), `check support` and `extract`, every findings list empty, as D2 does.
> - **Two eda rows ride along:**
>   - `1319` F2: `parts_arms_tier1.rs`'s doc line names the deleted source scan. Correct it.
>   - `1211e` F13: eda's `PinDeviceNotDrawable` guard on an undeclared IPROPI is now behind `validate`'s `TripSenseUnbound`. Keep it as eda's own guard with a direct test, as `1214` kept the generator's `ConsoleBaud`. Remove it only if you show why; never leave it untested.
>
> It is NOT:
> - a real CVM2 value. D1's bulk is a stand-in, and the drawing says so wherever the part's source is printed.
> - any change to the IR, the generator, the twin or `verify`;
> - passives as power-parts rows (`1211d` R9/A4).

## 1. Build

1. **Measure first,** with predictions committed before any run, in `spike/1211f/`:
   - every committed specification's `emit` output, exit code and stdout on `main`, as the baseline;
   - D1 today: where it is refused, and with what;
   - whether `bulk.part`'s row carries a footprint the vendored KiCad library holds. If it does not, stop (§7).
2. **The arm:** one row feature or one interpreter case for `bulk`, whichever `1319`'s table can say. A new interpreter feature beyond `1211d` §4's eight is named in FEEDBACK with its reason.
3. **D1 through every board gate,** in the pinned image.
4. **The two rows** above.
5. **Tests and mutants:**
   - D1's bulk is drawn with its row's value and footprint;
   - a row with an unstated passive and no `bulk` is still refused;
   - a `bulk` on a row that states its value is refused (`BulkNotAllowed`) before eda;
   - one mutant per new line of the arm.

## 2. Acceptance

- **Every committed specification's emitted bytes, exit codes and stdout are identical** to the baseline, except D1, which now draws. Show it by the sweep `1319` committed.
- **D1 passes ERC, DRC with parity, `check support` and `extract`,** every findings list empty.
- **The CLAUDE.md gates pass, and CI is green.** `TIER2_MIN` moves only by the tests this work order adds, stated, with their weight rows in `ci/test-weights.tsv` (`1606` found the planner refuses a test without one).
- If a `fab/` package's bytes move, stop before READY: the package queue is serial.

## 3. Fences

- **Fence:**
  - `crates/aethelgard-eda` (the arm and its tests);
  - `crates/aethelgard-kicad-lib`, only for a footprint the bulk row names, vendored byte for byte from KiCad 10.0.7 with its sha;
  - `crates/aethelgard-cli/tests/parts_arms_tier1.rs`, one doc line;
  - `ci/test-weights.tsv`, weight rows only;
  - `spike/1211f/` and FEEDBACK.
- **Not:**
  - the IR (`parts.json`, `power-parts.json`, the schema);
  - the generator, the twin, chaos or `verify`;
  - workflows.
- **Running beside you:**
  - `1211c` (twin, chaos, CLI `verify` and `simulate`, and four IR solved fields);
  - `1606` (the bridge, `doctor`, `build`).
  Neither touches eda. Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-1211f/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A passive drawn with a value no source states:** any path where `PassiveValueUnstated` lifts without a `bulk`, or for a passive other than the one `bulk` names.
- **A stand-in drawn as if it were a design:** the BOM, `FAB-NOTES.md` or the schematic losing the "not a design" label.
- **A byte of another board that moves.**

## 7. Flag-and-stop conditions

- **`bulk.part`'s row names a footprint the vendored library does not hold.** Stop with what is held and what KiCad 10.0.7 ships.
- **A row cannot say `bulk` without an interpreter feature beyond `1211d` §4's eight.** Stop and report it.
- **Any emitted byte of another committed specification moves.** Stop and report it with the diff.
