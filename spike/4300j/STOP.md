# AET-WO-4300j — STOP after §1.2: the bound, the value, and the schema shape

Written 2026-11-23 on `wo/aet-4300j-motor-pull-fitment`. The measurement is `RESULTS.md`, and the derivation is `DERIVATION.md`. No build code has been written.

**No §7 stop fired other than §1.2's own:**
- an E96 value fits under every reading;
- nothing has been built yet, so no pin could move (P18 forecasts none);
- every MCU number is on a held page.

**One near-stop, named:** the ESC's numbers are stand-ins, not held. The WO allows a stand-in for the ESC's VIL. Under D1(a), the stand-ins also cover the ESC's VIH, its input current and its rail's tolerance. The document a ruling would need is the ESC MCU's datasheet: for the held reference design, Microchip's dsPIC33CDVC256MP506 family datasheet (row R1). I have treated this as part of the §1.2 ruling, not as a separate stop.

## The answer

- **The bound,** derived by AN4899 §5.1.5's method (p. 18) and mirrored for a pull-down (my derivation, labelled as such, `DERIVATION.md` §2–§5a). **Lean: the cross-rail reading:**
  - **floor 422 561 Ω:** BLHeli passthrough idle, with RPU max 50 kΩ against the pull-down. The FC's rail is at VDD min (3135 mV), the ESC's own rail at VDD max (3465 mV), and the line must be ≥ ESC VIH + 10 %, that is ≥ 0.8 × 3465 mV;
  - **ceiling 895 714 Ω:** reset, pin Hi-Z, 700 nA leakage, with the line ≤ 0.2·VDD at VDD min.
- **Why cross-rail:** DS70005664A p. 16 shows the ESC making its own 3.3 V (`VREG` → `R54` 0R → `VDD`). So the two rails vary independently. I found this after the predictions: P8–P10 assumed one rail.
- **The value: 442 kΩ, `CRCW0603_F_K`,** the smallest E96 whose worst case (±1 % tolerance, ±2 % drift) lies inside both bounds:
  - in passthrough, cross-rail, 2776.2 mV against ESC VIH + margin of 2772.0 mV;
  - at reset, 318.7 mV against a VIL max of 940.5 mV (VIL − margin is 627.0 mV).
- **The alternative (D1(b)), same-rail:** floor 211 824 Ω, value **221 kΩ**:
  - same-rail, it holds 2513.7 mV against 2194.5 mV;
  - cross-rail, it clears the ESC's bare VIH (2425.5 mV) by 88.2 mV, but not the margin.
- **For comparison:** 4300i's 40.2 kΩ holds the passthrough line at 1381.6 mV, between VIL max and VIH min, so it reads as neither level.
- **The schema shape:**

  ```json
  "pull": {
    "resistor": {"reference": "R52", "part": "CRCW0603_F_K", "ohms": 442000, "to": "GND"},
    "link":     {"reference": "R56", "part": "CRCW0402_Z_0"},
    "fitment":  "fitted",
    "source":   "…"
  }
  ```

  - `motor_outputs[].pull.fitment` takes `fitted` or `open`. It is required, with no default (`INV-06`).
  - `link` is required in both fitments.
  - The references are illustrative. The build allocates each board's free references by script, as 4300i's `pull_refs.py` did.
  - Specification **29**, and power-parts **10** (D4). Solved topology **18** is its own decision (D9).

## Decisions for the ruling

### D1 · The rails, the margin and the stand-ins

- **The margin:** 10 % of VDD at both edges. It covers what the stand-ins do not hold.
- **The stand-ins:**
  - **S1:** the ESC's VIH and VIL are the STM32's CMOS rows, on a 3.3 V rail whose tolerance stands in at the board's ±50 ‰. The ESC regulator's tolerance is not held;
  - **S2:** the ESC's input current is the STM32's largest Ilkg row, 350 nA.
- **D1(a), the lean: cross-rail.** The ESC has its own rail (DS70005664A p. 16), so the floor is taken with the FC at VDD min and the ESC at VDD max: **422 561 Ω → 442 kΩ.**
- **D1(b): same-rail,** as P8–P10 predicted: **211 824 Ω → 221 kΩ.** It is the plainest reading of the WO's "VIH min", but a held page contradicts its premise.
- **The zero-margin numbers, so the margin's weight is visible:** cross-rail 179 800 Ω (→ 187 k), and same-rail 121 176 Ω (→ 127 k). More rows are in `DERIVATION.md` §5.
- Either way, **the ESC's threshold is a FEEDBACK row (R1), not a decision here** (WO "It is not").

### D2 · The value-selection rule

- **Lean:** the smallest E96 inside the bound, with tolerance and drift: 442 k under D1(a), 221 k under D1(b). It gives the strongest hold at reset.
- **Alternative:** the geometric middle of the window: about 619 k under D1(a), 432 k under D1(b). It gives the most margin at both edges.

### D3 · How the bound is held, and what validate judges

- **Lean (a):** hold the **inputs**, not the result. The MCU row's `io.weak_pull_down_ohms` (the RPD band) is replaced by `io.motor_pull_bound`, with these fields:
  - `pull_up_max_ohms` 50 000 (DS12117);
  - `input_high_min_permille` 700 (DS12117);
  - `input_low_max_permille` 300 (DS12117);
  - `leakage_max_nanoamps` 350 (DS12117, the pad);
  - `peer_input_current_nanoamps` 350 (S2, a stand-in);
  - `margin_permille` 100 (the margin).

  Each field has its `at`. A field that is a stand-in or a choice cites this work order and its ruling, not a datasheet.
  - Validate computes the floor and ceiling **per board**, in exact integer arithmetic, from the board's `mcu.supply` nominal and tolerance:
    - the floor with the pull-up at that rail's minimum and the ESC's VIH at its maximum (D1(a));
    - the ceiling at the minimum.

    It refuses a declared value outside them.
  - With these fields, every committed board gives 422 561 and 895 714 Ω.
  - Like 4300i D3, it judges the **declared ohms**: the margin is in the bound, and tolerance is in the selection rule.
  - RPU becomes data. That closes 4300i's reduction finding R-g.
- **Alternative (b):** hold the two derived numbers (211 824 and 895 714) with a citation to `DERIVATION.md`. That is simpler, but true only on a 3.3 V ±5 % rail. A board on another rail would be judged against the wrong bound with nothing refusing it.
- **Refusal:** `MotorPullOutsideBound` replaces `MotorPullOutsideRpd`. The message names the floor, the ceiling, the rail and the citation. Both edges get a planted test: floor − 1 and ceiling + 1 Ω refused, the edges themselves admitted.

### D4 · The link part

- **Lean:** a new power-parts row, **`CRCW0402_Z_0`**: the Vishay D10/CRCW0402 e3 jumper, ordering `CRCW04020000Z0ED`. Its sources:
  - VISHAY_20035 p. 2: 0 Ω, ≤ 20 mΩ, Imax 1.5 A;
  - VISHAY_20035 p. 3: `0000` jumper, `Z` jumper tolerance, `0` jumper TCR.
- It needs a **new kind, `jumper`**, with `max_milliohms` 20 and `max_milliamps` 1500.
  - The resistor kind's required `tolerance_ppm`, `tcr_ppm_per_kelvin` and `drift_ppm` cannot be filled honestly: p. 2 says TCR "is not specified for 0 Ω jumpers".
  - So power-parts moves to 10.
- **Its drawing:**
  - symbol `Device:R`, value `0`;
  - footprint `Resistor_SMD:R_0402_1005Metric`, vendored and already loaded by the tool tests, though no emitted board has yet placed it. That is new to DRC on a routed board.
- **Why not a solder jumper:** no `Jumper` symbol library or `SolderJumper` footprint is vendored, so eda draws no such part today.
- **Why not 0603:** 0603 would reuse the drawn footprint. The WO names 0402, and the reference ESC uses 0R 0402 links (DS70005664A p. 16).
- **Alternative:** the CRCW0603 jumper (Imax 2.0 A, p. 2), on the footprint every board already places.

### D5 · Where the link sits, and the net between

- **Lean:** `MOTORn` → link → **`MOTORn_PD`** → resistor → `GND`. The link is at the motor-net end.
  - With the link open, the motor net carries one unpopulated 0402 pad. The resistor's stub hangs off a net that is not the motor line.
  - The middle net's name is derived, `<label>_PD`. It joins `names::own_nets`, the engine's single list of derived nets (`1206`). The existing check then refuses a declared net of that name, case-folded, as `ReservedName`, so **no new kind** is needed.
- **Alternative:** the link at the GND end. An open link then leaves the resistor's pad on the motor line as a stub.
- **Support step 8 changes** to judge the chain:
  - the link must be the declared jumper, between `MOTORn` and `MOTORn_PD` (`motor_link_*` findings);
  - the resistor must sit between `MOTORn_PD` and `to`.
  - `motor_net_extra_resistor` stops counting the link. It still refuses any other two-pin part on `MOTORn`.

### D6 · The fitment in the firmware

- `fitted`: svd-hal writes `PUPDR` `00`, and hwdef writes `FLOATING`, as 4300i built.
- `open`: svd-hal writes `PUPDR` `10`, and hwdef writes `PULLDOWN`, as before 4300i.
- `SolvedMotorOutput` gains `pull: "fitted" | "open"`. That makes topology 18, because both generators read the topology alone (`RUL-16`; `hwdef::generate(&SolvedTopology)`).
- `implied_pupd`, `check pinout`'s `WrongPull` and the ledger's `MOTOR_PUPD_RULE` all read that field.
- **The hwdef comment line,** one per board above the motor lines, for example:
  `# motor pulls (AET-WO-4300j): fitted R52 442k to GND via link R56 (MOTOR1); passthrough idle >= ESC VIH + 10 % VDD: bound 422561..895714 ohm, VDD_3V3 3300 mV +/-50`
- No `HAL_SERIAL_ESC_COMM_ENABLED` is written.
- **An `open` output on a pin with a reset pull** stays refused (`MotorPinResetPull`, unchanged).

### D7 · The BOM's `DNP`

- **Lean:** for an `open` output:
  - the emitter's BOM writes `DNP` in the link row's `Note`;
  - the schematic symbol carries `(dnp yes)`;
  - the footprint carries KiCad's `dnp` attribute;
  - the resistor stays fitted.
- The header does not change, so no BOM on a non-motor board moves.
- JLCPCB's BOM and placement omit a `DNP` designator. No committed board reaches the fab package with motors, so that path is tested in its own test only.

### D8 · Which committed boards declare which fitment

- **Lean:** every committed board is **`fitted`**. That is Scotty's ruling: passthrough works with it fitted. All 40 outputs move to the ruled value (442 kΩ under D1(a)), each with a link.
- `open` is covered by **new goldens that no twin reads:**
  - an svd-hal ui case `motors-pwm-open`;
  - an hwdef fixture `serial-dshot-9600-open`.
- Each golden is solved from a source specification committed under `spike/4300j/open/`.
  - `universe.sh` skips `spike/`, so it is not a 35th board.
  - `hwdef_tier1` and `solved_devices` sweep `examples/` and `crates/aethelgard-cli/tests/fixtures/`. Neither sweeps `spike/`.
  - The twin names its motor fixtures one by one.
- **Why not a committed board:** every motor fixture also feeds the twin (`twin_tier2`, `motor_platform.rs`), which is outside this WO's fence. Flipping one to `open` changes its firmware's `PUPDR`, and so what the twin runs.
- **Alternative:** flip `serial-dshot-9600` to `open`, which would also move the `ardupilot-image` job's input.

### D9 · Solved topology 18

- The WO names only the specification version. The fitment must still reach both generators, and each reads the topology alone (`RUL-16`: `hwdef::generate(&SolvedTopology)`, svd-hal's board generator).
- **Lean:** `SolvedMotorOutput.pull`, `"fitted"` or `"open"`. Solved moves 17 → 18.
- **What it moves outside the fence:** the twin's `REFERENCE` literals (`src/tests.rs`, `tests/spi_platform.rs`), `schema_version` 17 → 18. Those topologies have no motors.
- **Forecast, not measured:** no twin source line, trace version or committed verdict moves. The twin reads no field this adds. The null sweep and the twin's tests will judge that.
- **Alternative:** none that keeps `RUL-16`. A generator reading the specification would breach it.

## Fence: outside the WO's list, each needed by the build

| path | why | precedent |
|---|---|---|
| `crates/aethelgard-cli/src/ledger.rs`, `tests/ledger_tier1.rs` | `MOTOR_PUPD_RULE` follows the fitment | ruled F2 for 4300i |
| `crates/aethelgard-twin/src/tests.rs`, `tests/spi_platform.rs` | the `REFERENCE` literal's `schema_version` 17 → 18 (D9). Forecast: no twin source line or verdict moves | `1211c` D7 |
| CLI test literals: digests, counts, the version ladder | moved by the bump, by their generators | 4300i §8 |
| `boards/`, `fab/sentinel-01-bench` | moved by hash lines (P17). **Needs a package dispatch:** I will ask the architect when the build is green | 4300i §6 |

## Rows for FEEDBACK (none is a stop)

- **R1 · The ESC's input threshold, its leakage and its rail's tolerance** wait for a held ESC MCU datasheet. For the reference design that is Microchip's dsPIC33CDVC256MP506 family datasheet. For a BLHeli ESC, its MCU's (EFM8BB2 or AT32F421 class). The cross-rail case (D1) depends on it.
- **R2 · `RPU` near VDD is not held.** DS12117 states it only at `VIN = VSS` (S4, note 7 p. 137).
- **R3 · The reference ESC loads its input with 0.1 µF** (`C4`, DS70005664A p. 16). That bears on (b), the off-board drive bound.
- **R4 · Whether the ESC firmware enables an input pull** is not held. A BLHeli bootloader that pulls its input up at power-on would contest a pull-down of a few hundred kΩ at reset (S3).
