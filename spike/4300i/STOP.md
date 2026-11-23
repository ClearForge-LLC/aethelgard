# AET-WO-4300i — STOP after §1.2, with §7's no-held-source stop

**Written:** 2026-11-23, on `main` `f50c29f4`. `main` has since moved to `28fba951`, in docs only; no crate or specification changed.
**Branch state:** no crate, specification, `fab/` or workflow byte has changed. The branch holds `spike/4300i/` alone.

**Commits:**
- predictions and rules: `45412fe0`, before any run;
- the measurement: `0de4f99f`.

**Grounds:** the measurement is `RESULTS.md`; the predictions are `PREDICTIONS.md` (16 of 17 hold, one is for the build).

## The measurement, in brief

- **Baseline, 34 specifications:**
  - validate 32/34; the 2 refusals are by design;
  - solve 29; the 3 `solve-*` refusals are by design;
  - emit 25; 4 refuse `DrawsNoBoard`, as declared stand-ins;
  - 34/34 byte-identical on a repeat, which gives the null-sweep base.
- **(1) Which boards and nets gain the pull-down:** 35 motor nets on the 9 validating boards, plus 5 on `d2-drv8833`, which refuses.
  - 28 nets on 7 boards are drawn; 7 more are on stand-ins (`hse-external-8`, `d1-drv8876`).
  - 34 nets face a connector, and 6 end at a DRV bridge input only (`d1`, `d2`).
  - No output idles high.
  - `boards/` and `fab/` hold no motor board.
- **(2) FT at connectors:**
  - 90 connector-facing assignments; 0 solver-chosen non-FT, so **0 pins move**;
  - **1 hand TT pin refused**, Sentinel-03 MOTOR1 PE9 at J5.1.
- **(3) MOTOR1 → PA8 on a copy:**
  - `validate` 0 and `solve` 0;
  - the solve differs only in `/assignments/MOTOR1/pin` (and the specification hash);
  - **no other pin moves on any committed board.**
- **(4) `reserved_pins`:** Sentinel-03 has nothing on PA11, PA12 or PC9 before or after the re-pin, so the reservation moves nothing.

## (1)'s schema shape, proposed

- **The field:** `motor_outputs[].pull`, a required key on `MotorOutput` (`crates/aethelgard-ir/src/motor.rs`). It is never `null`, so a missing key is refused at parse (`INV-06`). It reuses the pin-device pull that A1 judges (`pin_devices::PullResistor` plus `source`):

  ```json
  "pull": {"resistor": {"reference": "R61", "part": "CRCW0603_F_K", "ohms": <held>, "to": "GND"},
           "source": "<document, revision, table, page>"}
  ```

  - It has no `mcu` arm (D0).
  - `to` other than `GND` is refused (`MotorPullNotDown`).
  - `part` must be a resistor row. `CRCW0603_F_K` is one already, so there is no parts bump.
  - The reference joins `names::references` (step 15a).
- **Who reads it:**
  - **A1** (`PullAboveMcuTolerance`) judges it on the motor pin.
  - **`silicon::implied_pupd`** gives 0 on a motor pin.
  - **`svd-hal`** stops writing `motor_pull`.
  - **hwdef** writes `FLOATING` in place of `PULLDOWN`.
  - **`check pinout`** follows.
  - **eda** draws R from the net to GND with the row's footprint, reading the specification, as it reads pin-device pulls.
- **Versions:**
  - **specification 28** (wave 1 takes no bump);
  - solved stays 17 (lean). The alternative is to carry the pull on `SolvedMotorOutput`, which takes solved 18;
  - parts tables unchanged.
  - Every committed specification moves by its version line. `boards/sentinel-01*` and `fab/sentinel-01-bench` move by hash lines, so a package dispatch is queued at the build.

## The §7 stop: no held document states a pull value

- **What was searched:** every held ESC document, read through `spike/1219/pdftext.py`:
  - Hobbywing XRotor Micro 40A BLH32 manual;
  - Holybro Tekko32 Mini 45A and BigFET 50A specs;
  - the ModalAI VOXL ESC page;
  - the Hobbywing BLHS manual, which is image-only and was **not readable**.

  None gives an ESC signal-input impedance or pull. Nor do DS-009, AN4938 Rev 7 §7.6 ("pull-up or pull-down to the unused I/O pins", no value), AN5419 Rev 3 or RM0433 Rev 9.
- **AN4899 is not held.** DS12117 cites it (pp. 136, 241).
- **The one pull value held:** DS12117 Rev 10, RPD 30/40/50 kΩ min/typ/max (Table 49 p. 137, rev Y; Table 146 p. 241, rev V), read on the held text. It is the internal pull Q-G uses today, a fact about the pin and not a board resistor's value. **Not adopted.**
- **What is needed (any one):**
  - (i) **AN4899** (ST, "STM32 GPIO configuration for hardware settings and low-power consumption"), if it states a pull-down value or range;
  - (ii) the **input-stage datasheet or schematic of the ESC J5 will drive.** J5 declares `esc: none` today, so an ESC figure cites an undeclared peer;
  - (iii) a ruling that DS12117's RPD range, the strength Q-G held the line with, is the held source for the external value. The value would be one whose worst case over the row's tolerance stays inside 30–50 kΩ, and the ruling names it.

## Decisions asked

- **V · The value and its source:** (i), (ii) or (iii) above. Lean: (iii) only if neither (i) nor (ii) can be held soon. (i) is a single ST document.
- **B · The 6 bridge-input-only motor nets** (`d1-drv8876` `DRV_EN`/`DRV_PH`; `d2-drv8833` `xIN1`/`xIN2`, whose part rows carry RPD 150 kΩ typ). **Lean:** the field is required on every motor output, as D0 reads. Alternative: the bridge input's part-row RPD counts as the declared pull.
- **I · Idle high:** none today. **Lean:** `to` must be `GND`, so a future idle-high output is refused by name until ruled.
- **F1 · Fence:** Sentinel-03 is `crates/aethelgard-cli/tests/fixtures/sentinel-03.json`; no `examples/sentinel-03*.json` exists. Ask: is that file in the fence?
- **F2 · Fence:** the CLI's witness ledger reads the motor pull (`crates/aethelgard-cli/src/ledger.rs` `MOTOR_PUPD_RULE` / `MOTOR_PUPD_HIGH_RULE`, `tests/ledger_tier1.rs` L515–536). With no motor `PUPDR`, those rows must change, and the fence excludes `verify`. Ask: may the build move those rows and tests?
- **F3 · Where E5's note moves:** `3400`'s E5/MCO1 reservation lives in `docs/work-orders/AET-WO-3400.FEEDBACK.md` (L169, L268, L280), another work order's record. **Lean:** the build states MCO2/PC9 in Sentinel-03's `reserved_pins` reason and in this FEEDBACK, and the roadmap row is yours.

Nothing is built past this stop until the ruling arrives on the bus.
