# AET-WO-4300j — STOP after §1.2: the bound, the value, and the schema shape

Written 2026-11-23 on `wo/aet-4300j-motor-pull-fitment`. The measurement is `RESULTS.md`, and the derivation is `DERIVATION.md`. No build code has been written.

**No §7 stop fired other than §1.2's own:**
- an E96 value fits;
- every needed number is on a held page;
- nothing has been built yet, so no pin could move (P18 forecasts none).

## The answer

- **The bound,** derived by AN4899 §5.1.5's method (p. 18) and mirrored for a pull-down (my derivation, labelled as such, `DERIVATION.md` §2–§4):
  - **floor 211 824 Ω:** BLHeli passthrough idle, RPU max 50 kΩ against the pull-down, with the line ≥ 0.8·VDD;
  - **ceiling 895 714 Ω:** reset, pin Hi-Z, 700 nA leakage, with the line ≤ 0.2·VDD;
  - both at VDD min 3135 mV, the declared rail of every committed motor board.
- **The value: 221 kΩ, `CRCW0603_F_K`,** the smallest E96 whose worst case (±1 %, +2 % drift) lies inside both bounds:
  - in passthrough, 2513.7 mV against a VIH min of 2194.5 mV;
  - at reset, 159.3 mV against a VIL max of 940.5 mV.
  - 4300i's 40.2 kΩ holds the passthrough line at 1381.6 mV, which reads as neither level.
- **The schema shape:**

  ```json
  "pull": {
    "resistor": {"reference": "R52", "part": "CRCW0603_F_K", "ohms": 221000, "to": "GND"},
    "link":     {"reference": "R56", "part": "CRCW0402_Z_0"},
    "fitment":  "fitted",
    "source":   "…"
  }
  ```

  - `motor_outputs[].pull.fitment` takes `fitted` or `open`. It is required, with no default (`INV-06`).
  - `link` is required in both fitments.
  - The references are illustrative. The build allocates each board's free references by script, as 4300i's `pull_refs.py` did.
  - Specification **29**; solved topology **18**; power-parts **10**.

## Decisions for the ruling

### D1 · The margin and the stand-ins

- **The margin:** 10 % of VDD at both edges.
- **The stand-ins:**
  - **S1:** the ESC's VIH and VIL are the STM32's CMOS rows, on the same rail;
  - **S2:** the ESC's input current equals the STM32's largest Ilkg row, 350 nA.
- **Lean: as derived.** The alternatives are in `DERIVATION.md` §5: margin 0 gives 127 k, 5 % gives 162 k.
- **The risk I cannot close from held sources:** the cross-rail case. An ESC on its own rail at VDD max, against the FC at VDD min, puts the floor at 422 561 Ω. 221 kΩ would fail it.
  - The E96 values that pass both that floor and the ceiling are 442 k to 866 k.
  - **Alternative D1(b):** rule the cross-rail floor in and take 442 kΩ. That gives up reset-hold strength the derivation does not model.
  - Either way, the ESC's threshold is a FEEDBACK row, not a decision here (WO "It is not").

### D2 · The value-selection rule

- **Lean:** the smallest E96 inside the bound, with tolerance and drift (221 k). The strongest hold at reset.
- **Alternative:** the geometric middle of the window (about 432 k), the most margin at both edges.

### D3 · How the bound is held, and what validate judges

- **Lean (a):** hold the **inputs**, not the result. The MCU row's `io.weak_pull_down_ohms` (the RPD band) is replaced by `io.motor_pull_bound`, with these fields:
  - `pull_up_max_ohms` 50 000;
  - `input_high_min_permille` 700;
  - `input_low_max_permille` 300;
  - `leakage_max_nanoamps` 350;
  - `margin_permille` 100;

  each with its `at`. A field that is a stand-in or a choice cites this work order and its ruling, not a datasheet.
  - Validate computes the floor and ceiling **per board**, at the board's `mcu.supply` minimum, in exact integer arithmetic. It refuses a declared value outside them.
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
  - The middle net's name is derived, `<label>_PD`. Validate refuses a specification that declares that name anywhere else (`MotorPullNetTaken`).
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
  `# motor pulls (AET-WO-4300j): fitted R52 221k to GND via link R56 (MOTOR1); passthrough idle >= 0.8 VDD: bound 211824..895714 ohm at VDD 3135 mV`
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

- **Lean:** every committed board is **`fitted`**. That is Scotty's ruling: passthrough works with it fitted. All 40 outputs move to 221 kΩ with a link.
- `open` is covered by **new goldens that no twin reads:**
  - an svd-hal ui case `motors-pwm-open`;
  - an hwdef fixture `serial-dshot-9600-open`.
- **Why not a committed board:** every motor fixture also feeds the twin (`twin_tier2`, `motor_platform.rs`), which is outside this WO's fence. Flipping one to `open` changes its firmware's `PUPDR`, and so what the twin runs.
- **Alternative:** flip `serial-dshot-9600` to `open`, which would also move the `ardupilot-image` job's input.

## Fence: outside the WO's list, each needed by the build

| path | why | precedent |
|---|---|---|
| `crates/aethelgard-cli/src/ledger.rs`, `tests/ledger_tier1.rs` | `MOTOR_PUPD_RULE` follows the fitment | ruled F2 for 4300i |
| `crates/aethelgard-twin/src/tests.rs`, `tests/spi_platform.rs` | the `REFERENCE` literal's `schema_version` 17 → 18, a literal only. No twin source line or golden verdict moves | `1211c` D7 |
| CLI test literals: digests, counts, the version ladder | moved by the bump, by their generators | 4300i §8 |
| `boards/`, `fab/sentinel-01-bench` | moved by hash lines (P17). **Needs a package dispatch:** I will ask the architect when the build is green | 4300i §6 |

## Rows for FEEDBACK (none is a stop)

- **R1 · The ESC's input threshold and leakage** wait for a held ESC MCU datasheet. For the reference design that is Microchip's dsPIC33CDVC256MP506 family datasheet. For a BLHeli ESC, its MCU's (EFM8BB2 or AT32F421 class). The cross-rail case (D1) depends on it.
- **R2 · `RPU` near VDD is not held.** DS12117 states it only at `VIN = VSS` (S4, note 7 p. 137).
- **R3 · The reference ESC loads its input with 0.1 µF** (`C4`, DS70005664A p. 16). That bears on (b), the off-board drive bound.
- **R4 · Whether the ESC firmware enables an input pull** is not held. A BLHeli bootloader that pulls its input up at power-on would contest a 221 k pull-down at reset (S3).
