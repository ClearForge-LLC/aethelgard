# AET-WO-1217 — the measurement (§1.1) and the stop (§7)

Measured 2026-11-22 on Furnace, at main `22a1b3c0` (specification 25, solved 17), with main's own `aethelgard` (debug build). Predictions were committed first (`f70d6972`). Reproduce with `bash spike/1217/measure.sh <binary> <repo> <out>`. Outputs are in `out/`.

## 1. What was measured

- **Baseline** (`out/baseline-exits.tsv`). 34 committed specifications, 280 files, by `spike/1211d/measure/sweep.sh`. That is the count `1216b` recorded. 25 validate, solve and emit. The rest are refused where they were before:
  - `d2-drv8833` at step 24, `PartLimitUndeclared`;
  - `invalid-zero-domain` at validate;
  - three solver fixtures by `Infeasible`;
  - four emits, each `DrawsNoBoard` (`board.drawn: false`): D1, `imu-baro-1v8`, `hse-external-8` and `sentinel-02-bench-hse8`.
- **F4, PC2_C and PC3_C** (`out/pc2-pc3.tsv`). **No committed specification places any signal on `PC2` or `PC3`**: no bus line, chip select, GPIO, motor output or ADC input. The positive control finds `PB6` in `solve/stress`.
  - A throwaway prototype withheld both pins from **every** offer, analog included (`proto-f4-withheld.patch`), which bounds the real change from above. Against main, the sweep is **identical: 280 files** (`out/f4-proto-sweep.txt`). The three refused solver fixtures keep their kinds, claimants and messages.
  - **No committed board moves, so (2)'s pinout stop does not fire.**
- **F2, F9, F10 probes** (`out/probes.tsv`; `probes.py` writes each one as a single edit of a committed file):

| Probe | Today (validate = solve) |
|---|---|
| `f2-ipropi-gnd`: IPROPI tied to GND, `analog: []` | `SignalKindMismatch` @ `pin_devices[0].straps.IPROPI` |
| `f2-ipropi-gnd-vref-gnd`: the same, with VREF on GND | `SignalKindMismatch` @ `…straps.IPROPI` (BTreeMap order reaches IPROPI before VREF) |
| `f2-ipropi-gnd-and-analog`: the tie, with the analog input kept | `SignalKindMismatch` @ `…straps.IPROPI` |
| `f2-no-analog` | `TripSenseUnbound` @ `pin_devices[0].analog` |
| `f9-vref-gnd-over-vdda`: VREF on GND, R2 2 kΩ (full scale above VDDA) | **`AdcFullScaleExceeded`** @ `…analog[0].full_scale`. This hides `TripReferenceGrounded`, which is F9 |
| `f9-vref-gnd` | `TripReferenceGrounded` @ `…straps.VREF` |
| `f9-no-reservoir`, `f9-vref-gnd-no-reservoir` | `ok`, and `TripReferenceGrounded`: R2 is 1.07 kΩ since `1216b`, under RAIN, so no reservoir is needed |
| `f10-d1-other-mcu`: D1 with `mcu.part` `STM32H743ZITx` | `PowerPartUnknown` @ **`pin_devices[0].mcu`**, a path that names no field (F10) |
| `f10-minimal-other-mcu` | validate: `UnwitnessedAcknowledgementStale`; solve: `UnknownPart` |

- **F10's scope** (`out/f10-scope.tsv`). Every committed specification was re-run with `mcu.part` set to a part with no power-parts row:
  - 23 of 34 still validate;
  - 6 are refused earlier (`UnwitnessedAcknowledgementStale`);
  - 2 are refused at step 16 (`PowerPartUnknown`, no path);
  - 1 is refused at step 24 (D1, the F10 path);
  - `d2` and `invalid-zero-domain` keep their own refusals.

  Today an unknown MCU is refused by `solve` and `emit`, not by `validate`: `solve_cli::an_unknown_part_is_exit_2` asserts `UnknownPart` from `solve`.
- **F8.** The lowest-rated held capacitor is `GRM32ER71A226KE20`, at 10 V. `mcu.supply` is at most 3.6 V, and `analog()` reads the embedded power-parts table. So no specification can tell VDDA's minimum from its maximum, and a test that fails without the fix needs a seam (§3).

## 2. Predictions beside results

| # | Prediction (`f70d6972`) | Result | Holds |
|---|---|---|---|
| 1 | 34 specifications, exits as `1216b` recorded, none newly refused at step 24 | 34 and 280 files, reproduced twice | **holds** |
| 2 | No signal of any kind on PC2/PC3; the solver change moves no board | none; prototype 280/280 identical | **holds** |
| 3 | The ADC kind hides A2 on "VREF on GND, reservoir removed", as `AdcSourceImpedance` | That probe validates the reservoir as unneeded (R2 1.07 kΩ < RAIN) and reports `TripReferenceGrounded`. The hiding is real on `f9-vref-gnd-over-vdda` (`AdcFullScaleExceeded`) | **fails** (wrong probe; F9 confirmed on another) |
| 4 | `PassivePartUnknown` at `pin_devices[0].mcu` | The variant is `PassivePartUnknown`, but its JSON kind is **`PowerPartUnknown`**; the path is as predicted | **holds**, with the kind's JSON name corrected |
| 5 | The tie is refused `SignalKindMismatch` at `straps.IPROPI`, with or without the analog input | as predicted, all three | **holds** |
| 6 | No held capacitor is rated between a legal VDDA's minimum and maximum | the lowest is 10 V | **holds** |

The predictions file's header says "solved 16". Main is at **solved 17**.

## 3. The stop: what is asked

### (1) F2's schema shape — lean (a)

**(a) Lean: no new key.** Admit the existing strap `pin_devices[].straps.<pin>: {"tie": "GND"}` on a `current_output` pin, and only on the pin that the row's `trip.sense` names.
- **Why the row field.** `trip.page` already cites SLVSDS7B p. 13 (7.3.3.2): "regulation is off only with IPROPI on GND and VREF above GND". The admission rests on the part's own statement, not on every `current_output` pin.
- **What a tie on a pin that no `trip.sense` names gets.** It stays `SignalKindMismatch`. That answers §5's "second current output on the same device".
- **How `trip()` changes:**
  - It judges `TripReferenceGrounded` first, as today, so VREF on GND is still refused with the tie in place (acceptance (1)).
  - With the sense tied to GND, it skips `TripSenseUnbound` and `TripBelowFullScale`.
  - A tie together with an `analog[]` entry on the same pin is `DevicePinBoundTwice`, since the strap binds the pin first.
  - "VREF above GND" is any declared domain. Step 2 holds `tolerance_permille` to 1–500 and refuses nominal 0, so every domain's minimum is at least half its nominal. VREF's own 0–3600 mV range is still judged.
- **Precedent.** `sense_return` already takes exactly this strap: `(PinKind::SenseReturn, Strap::Tie(GND))`, at `pin_devices.rs:971`.
- **Zero edits outside the fence:**
  - eda draws every `Strap::Tie` generically (`schematic_arms.rs:1233–1239`), so IPROPI lands on GND.
  - The twin, hwdef, pincheck, svd-hal and the CLI read the solved topology, which simply carries no ADC input for IPROPI.
  - Only eda's guard message (`schematic_arms.rs:1358`, "a pin device cannot declare that tie") goes stale. It becomes a FEEDBACK row for eda's owner.
- **Versions.** Specification **26** (main is at 25), solved unchanged at 17, parts schema unchanged.
  - `versions::CHANGES` row 26 is `requires []`, `optional []`, `rules: [<(2)'s new kind>]`, by row 25's precedent (rules only).
  - The tie admits content that 25 refused, so it adds no rule.

**Alternatives, not leaned:**
- **(b)** A new `Strap` variant, for example `{"no_feedback": "GND"}`. It forces an eda edit, which is outside the fence, and says nothing (a) does not.
- **(c)** A device key, for example `pin_devices[].regulation: "off"`. It is a new required or optional key, and an eda arm to read it.

### (2) F4's measurement and its shape — lean as below

- **Measured:** 0 committed boards place anything on PC2/PC3, and the bounding prototype leaves 280/280 files identical. **Nothing moves; no stop.**
- **Data.** The MCU row's `io` gains `analog_pads: {pins: ["PC2", "PC3"], at: …}`, power-parts schema 7 → 8. It cites DS12117 Rev 10 Table 8 p. 67: `PC2_C`/`PC3_C` are type `ANA`, and note 4 says the Pxy alternate functions reach the pad only through the analog switch. It also cites Table 11 p. 104: 1 mA sunk by a Px_C pin. The pins are written from `spike/1211e/table8.py`'s type column (rows whose type is `ANA`), so the list is reproducible.
- **The rule.**
  - The solver offers neither pad to any digital claimant: bus lines, serial ports, chip selects, pin-device GPIO, and motor outputs (which carry no timer channel there anyway).
  - A digital hand pin on one is refused in `solve`, by a new kind beside `ReservedPin`: **`AnalogPadPin`** `{pin, claimant}`, citing Table 8 and Table 11. `ReservedPin`'s message says "debug port or oscillator", so it is not reused.
  - Lean: `solve`, where every hand pin is already checked (`Model::new`'s `hand`, `pin_devices::hand_free`), rather than `validate`.
  - `SolvedTopology::validate` refuses a digital assignment on a pad too, so a hand-edited topology is caught where every reader re-validates it.
- **Asked: do ADC inputs stay offered on the pads?** Lean **yes**. Table 8 lists `ADC3_INN1`/`ADC3_INP0` on `PC2_C` and `ADC3_INP1` on `PC3_C`, which is what the pads are for. The battery sense and the pin-device ADC passes keep them. Nothing committed lands there today.
- **One test outside the IR breaks: `aethelgard-cli/tests/emit_tier1.rs:577`** (`a_solved_pin_is_drawn_by_its_package_position`). It hand-pins `spi_buses[0].miso` on `PC2` and expects emit to exit 0, as the witness that the emitter draws by package position. PC2/PC3 are the only two pins KiCad renames (`emit_tier1.rs:183`).
  - Lean: keep the witness. Draw the drawn D1's IPROPI analog input hand-pinned on `PC2` (`ADC3_INP0`), and assert its net on pad 28.
  - This is a test-only edit in `aethelgard-cli/tests`, a fence extension I am asking for.
  - The MISO-on-PC2 case becomes a refusal test (`AnalogPadPin`).

### (3)–(5): how each is built, for the record (none asked)

- **F8.** The reservoir's rating is judged against `mcu.supply`'s **maximum**, rounded up, as `bulk` does: rated at least that.
  - **The seam:** the comparison becomes one private helper in `pin_devices.rs`, with an in-crate `#[cfg(test)]` unit test at the edge (rated = max − 1 refused, = max passed). The crate already does this in `solve.rs`, `motor.rs` and others.
  - That test fails with the minimum. No embedded row can show it (§1).
- **F9.** `trip()` runs before `analog()` in `Device::check`. `f9-vref-gnd-over-vdda` then reports `TripReferenceGrounded`.
  - This **changes the refusal kind of a board that already refuses** (§5): that probe, and any input like it.
  - No committed specification changes kind. None is refused at step 24 but `d2`, which has no trip row and reports `PartLimitUndeclared` from `limits()`.
- **F10: asked, lean narrow.**
  - **Narrow (lean).** At the head of step 24, once, and only when `pin_devices` is non-empty (step 24 is the reader that needs the row), refuse at path **`mcu.part`** with `PowerPartUnknown`, id = the part. The 23 specifications that validate with an unknown part still do; only D1-like probes move, from `pin_devices[0].mcu` to `mcu.part`.
  - **Global (not leaned).** Refuse every unknown `mcu.part` in `validate`. This moves 23 probe files from ok to refused, and it **breaks `solve_cli::an_unknown_part_is_exit_2`** (the CLI validates before `solve_spec`, so `UnknownPart` would become `PowerPartUnknown`). It also contradicts `power.rs`'s "a part with no MCU row is the emitter's to refuse".
  - **The same fix, unreachable today.** `mcu_domain()`'s `.mcu.supply` path has the same flaw, but step 5 refuses an unknown `mcu.supply` first, so it is never reached. It gets fixed alongside.

## 4. Logistics

- **The bump moves the sha lines of `fab/sentinel-01-bench/package`** (FAB-NOTES and MANIFEST), so this work order joins the package queue behind `2308` for a write=true dispatch.
- Every other committed specification moves by its version line only. That is to be shown by the null sweep against this baseline (`~/.cache/aet-1217/sweep-base`).
