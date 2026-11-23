# AET-WO-4300j (1) — the motor pull-down's bound, derived by AN4899 §5.1.5's method

Every number below was read from the rendered PDF page, never from a text extract. Each document's sha256 was checked on Furnace on 2026-11-23 against the work order or `power-parts.json`. `derive.py` computes every result in exact fractions, and its output is `derive.out`.

## 0. Held sources

| id | document | sha256 | pages read |
|---|---|---|---|
| AN4899 | ST AN4899 Rev 4, "STM32 GPIO configuration for hardware settings and low-power consumption" | `a0446389…6bd6` | p. 18 (§5.1.5, Figure 5, the inequality) |
| DS12117 | ST DS12117 Rev 10, STM32H753xI | `607b9ead…c032` | p. 136 and p. 137 (Table 49, rev Y); p. 241 (Table 146, rev V). The two tables agree in every figure used here |
| DS70005664A | Microchip drone ESC reference design user guide (dsPIC33CDVC256MP506) | `bef7e275…ef39` | p. 7 (§2.1.2, Table 2-3); p. 16 (Figure 3-3, schematic sheet 3, "External PWM/ADC Control") |
| VISHAY_20035 | Vishay D/CRCW e3, 14-Apr-2026 | `113d2d57…ae8b3` | p. 1 (technical specifications); p. 2 (resistance change, TCR and range, jumper rows) |

## 1. What AN4899 states (p. 18)

- §5.1.5 "Pull-up calculation" covers an **external pull-up on an STM32 open-drain output** that drives an external device (Figure 5).
- The bound is
  `(VDDmax − VOLmax) / (IOL + Ilkg) < RPU < (VDDmin − VOHmin) / (IIH + Ilkg)`.
- Each edge comes from one level:
  - **the upper edge** from the high level. The pull resistor must still hold the line at or above the high threshold while the sum of the input currents (the external input's `IIH` and the STM32 pad's `Ilkg`) flows through it, at `VDDmin`;
  - **the lower edge** from the low level. The resistor must be weak enough for the active driver to pull below the low threshold, at `VDDmax`.

AN4899 states it for a pull-up only. What follows is **my derivation** of the same method for a pull-down on a motor line, written step by step so that it can be checked.

## 2. The circuit, in the two states that matter

The motor line (net `MOTORn`) joins three things:
- the STM32 pin;
- the ESC's signal input, off-board through the motor connector;
- the external pull-down `R` to `GND`.

The two states:

| state | STM32 pin | ESC input | what must hold |
|---|---|---|---|
| **A · BLHeli passthrough, line idle** (ArduPilot `RCOutput.cpp:2300-2313`, `4300i` F7) | input, internal pull-up `RPU` to VDD | input, idle, waiting for the bus | the line reads **high** at both ends: `V ≥ VIH min` |
| **B · reset, before any firmware write** | Hi-Z, no pull (D0: no internal motor pull) | input | the line reads **low** at the ESC: `V ≤ VIL max` |

In state A the external `R` works against `RPU`. That is the AN4899 situation with the roles turned round: a resistor to one rail against an opposing current, judged at a threshold. In state B the only thing that sets `V` is leakage through `R`, which is AN4899's `(IIH + Ilkg)` term with the rail and threshold mirrored.

## 3. The floor, from state A (my derivation)

- **Step A1. Node equation.** Let `I` be the sum of the input currents leaving the node, the analogue of AN4899's `IIH + Ilkg`. Then
  `(VDD − V) / RPU = V / R + I`.
- **Step A2. Condition.** `V ≥ (k_IH + m) · VDD`, where:
  - `k_IH · VDD` is VIH min;
  - `m · VDD` is the stated margin (§5).
- **Step A3. Solve for R.** `R` increases monotonically in `V`, so the condition becomes
  `R ≥ V_hi / ((VDD − V_hi) / RPU − I)`, with `V_hi = (k_IH + m) · VDD`.
- **Step A4. Worst case.** The worst case is `RPU` at its maximum (the weakest pull-up), `I` at its maximum, and `VDD` at its minimum. `VDD min` is worst because `I` does not scale with VDD.

| input | value | source |
|---|---|---|
| `RPU` max | 50 kΩ | DS12117 Table 49 p. 137 = Table 146 p. 241, "Weak pull-up equivalent resistor", `VIN = VSS`, 30 / 40 / 50 kΩ |
| `k_IH` | 0.7 | DS12117 Table 49 p. 136 = Table 146 p. 241, VIH min "0.7VDD", note 1 "compliant with CMOS requirement(s)" |
| `Ilkg` (STM32 pad) | 350 nA | DS12117 Table 49 p. 137 = Table 146 p. 241, Ilkg at `0 < VIN ≤ Max(VDDXXX)`: FT_xx ±250, **FT_u ±350**, TT_xx ±250 nA. The largest row is taken |
| `IIH` (ESC input) | 350 nA | **assumption S2** (§4) |
| `VDD` min | 3135 mV | every committed motor board's `mcu.supply` is `VDD_3V3`: 3300 mV × (1 − 50 ‰). P7 is checked in RESULTS |

**Result: floor = 211 824 Ω** at VDD 3135 mV. It is 211 200 Ω at 3300 mV and 210 638 Ω at 3465 mV.

## 4. The ceiling, from state B (my derivation)

- **Step B1.** With the STM32 pin Hi-Z and the ESC an input, `V = I · R`. Here `I` is the same leakage sum, of either sign. A positive `I` (leakage into the node) is the case that raises `V`.
- **Step B2.** The condition is `V ≤ (k_IL − m) · VDD`, where `k_IL · VDD` is the ESC's VIL max.
- **Step B3.** Solving gives `R ≤ (k_IL − m) · VDD / I`. The worst case is `VDD` min and `I` max.

| input | value | source |
|---|---|---|
| `k_IL` | 0.3 | DS12117 Table 49 p. 136 = Table 146 p. 241, VIL max "0.3VDD", note 1 (CMOS). **Taken as the ESC's by assumption S1** |
| `I` | 700 nA | 350 + 350, as in §3 |

**Result: ceiling = 895 714 Ω** at VDD 3135 mV.

### The assumptions, named

- **S1 · The ESC's thresholds are an STM32H753's (VIH min 0.7·VDD, VIL max 0.3·VDD), on a 3.3 V rail of the board's declared tolerance.**
  - **The ESC has its own rail.** DS70005664A p. 16, Figure 3-3, "Power Supply": the ESC's `VDD` comes from the dsPIC's internal regulator (`VREG` → `R54` 0R → `VDD`; `R19` DNP), or from `+3.3V_EXT` on J5. J2 pin 2 is that `VDD`. So the ESC's rail and the FC's rail vary independently. That is the cross-rail case of §5a, the bound this STOP leans to. The same-rail reading of §3 is kept as the alternative.
  - The ESC regulator's tolerance is not held, so it stands in at the board's ±50 ‰.
  - No held document states an ESC's input threshold. DS70005664A p. 7, Table 2-3, names the input pin (`AD_PWM_IN1`, the dsPIC pin `AN19/CMP2C/RP75/PMA0/PMALL/PSA0/RD11`), but no dsPIC33CDVC256MP506 datasheet is held.
  - **Why the CMOS rows** and not the note-2 rows (VIH 0.47·VDD + 0.25 V, VIL 0.4·VDD − 0.1 V, "guaranteed by design"): across 1.62–3.6 V, the CMOS rows are the stricter at both edges (0.7·VDD ≥ 0.47·VDD + 0.25 for VDD ≥ 1.09 V, and 0.3·VDD ≤ 0.4·VDD − 0.1 for VDD ≥ 1 V).
  - At the STM32 end, the floor's VIH is held fact. At the ESC end, it is a stand-in.
- **S2 · The ESC input's leakage equals the STM32's largest Ilkg row, 350 nA.** This stands in for AN4899's `IIH`, the external device's input current.
- **S3 · The ESC adds no pull of its own.** DS70005664A p. 16, Figure 3-3, draws J2 pin 1 → `R11` 0R 0402 → `AD_PWM_IN1`, with `C4` 0.1 µF to GND and no pull resistor. Whether the dsPIC's firmware enables an internal pull is not held.
- **S4 · `RPU` is the equivalent resistor DS12117 states at `VIN = VSS`.** In state A the pin sits near 0.8·VDD, not at VSS. DS12117 note 7 (p. 137): "designed with a true resistance in series with a switchable PMOS/NMOS. This PMOS/NMOS contribution to the series resistance is minimal (~10% order)". No held page gives `RPU` at `VIN` near VDD. The work order names `RPU` max, so that is the figure used.

## 5. The margin

One margin is used at both edges: **10 % of VDD** (`m = 0.1`). That means the line is held at ≥ 0.8·VDD in state A and at ≤ 0.2·VDD in state B.

**Basis:** the margin covers what S1 and S2 do not hold, namely a real ESC whose threshold or leakage differs from the stand-in. 10 % of VDD is a third of the gap between the CMOS VIL and VIH rows (0.3 → 0.7·VDD).

The alternatives were computed as sensitivity only; they are not the bound:

| margin or reading | floor | ceiling | E96 by §6's rule |
|---|---|---|---|
| 0 | 121 176 | 1 343 571 | 127 k |
| 5 % VDD | 157 012 | 1 119 643 | 162 k |
| **10 % VDD (chosen)** | **211 824** | **895 714** | **221 k** |
| 10 %, Ilkg 250 nA at each end | 208 306 | 1 254 000 | 215 k |
| 10 %, ESC VIH from the note-2 row | 95 807 | 895 714 | 100 k |
| 10 %, the cross-rail case (§5a) | 422 561 | 895 714 | 442 k |
| 0, the cross-rail case | 179 800 | 1 343 571 | 187 k |

## 5a. The cross-rail case: the ESC on its own rail (S1, DS70005664A p. 16)

- In state A, the FC's pull-up works from the FC's rail, and the ESC judges against its own rail's VIH.
- **The worst case** is the FC at VDD min (3135 mV) and the ESC at VDD max (3465 mV), with ESC VIH min = 0.7 × 3465 = 2425.5 mV.
- Steps A1–A3 are unchanged, with `V_hi = (k_IH + m) · VDD_ESC,max` and the node at `VDD_FC,min`:

  `R ≥ V_hi / ((VDD_FC,min − V_hi) / RPU − I)`

- **The floor is 422 561 Ω** with the 10 % margin, or 179 800 Ω with none.
- **The ceiling does not change** (895 714 Ω). The ESC's VIL is judged at its own VDD min, the same 3135 mV.
- At the low corner, each value against the ESC's VIH:
  - **221 kΩ** (214 370 Ω) holds the idle line at 2513.7 mV. That clears VIH (2425.5 mV) bare, by 88.2 mV, but not VIH + 10 % (2772.0 mV).
  - **442 kΩ** (428 740 Ω) holds it at 2776.2 mV. That clears both.

## 6. The value

- **Rule:** the smallest E96 value whose worst case lies inside both bounds. The worst case is the declared value × (1 ∓ (tolerance + drift)).
  - Tolerance is ±1 % and drift ±2 % (Vishay gives the drift as a magnitude) (VISHAY_20035 p. 2: the CRCW0603 e3 ±100 ppm/K row; and "max. resistance change at P70 … 8000 h ≤ 2 %"). These are the row's `tolerance_ppm` and `drift_ppm`.
  - **Why the smallest:** the pull-down's other job, in state B, is to hold the line low against coupling and leakage the derivation does not model. The strongest pull that still clears the floor serves that job best.
- **Under the same-rail reading (§3–§4):** **221 kΩ, `CRCW0603_F_K`** (an existing row; E96 is held for 1 Ω to 10 MΩ at ±1 %, p. 2). Its worst case is 214 370 to 227 630 Ω.

What 221 kΩ does, same-rail:
- **in state A**, at VDD min with the low corner, the line sits at **2513.7 mV**, against a VIH min of 2194.5 mV;
- **in state B**, at the high corner, the line sits at **159.3 mV**, against a VIL max of 940.5 mV.

- **Under the cross-rail reading (§5a), the STOP's lean:** **442 kΩ, `CRCW0603_F_K`**, worst case 428 740 to 455 260 Ω.
  - In state A, cross-rail, the line sits at 2776.2 mV, against ESC VIH + margin of 2772.0 mV.
  - In state B, the line sits at 318.7 mV, against VIL max 940.5 mV and VIL − margin 627.0 mV.

TCR is not in the rule: 9 of 10 motor boards declare no temperature (`4300i` F6). At Sentinel-02's declared −20 to 85 °C, ±100 ppm/K adds ±0.65 %. The low corners are then 212 934 Ω for 221 k (same-rail floor 211 824 Ω) and 425 867 Ω for 442 k (cross-rail floor 422 561 Ω). Both are still above their floors, 442 k by 0.8 %.

**4300i's 40.2 kΩ, for comparison:** in state A it holds the line at 1381.6 mV. That lies between VIL max (940.5 mV) and VIH min (2194.5 mV), so it reads as neither level. This is `4300i` F7, and it is why this work order exists.

## 7. The link (for §3 of the WO)

VISHAY_20035 p. 2, D10/CRCW0402 e3: "Jumper, Imax. = 1.5 A", tolerance "≤ 20 mΩ", resistance "0 Ω". p. 2's note: "The temperature coefficient of resistance (TCR) is not specified for 0 Ω jumpers".

The ESC reference design uses the same kind of part, 0R 0402, as its own series links on J2 (DS70005664A p. 16: `R11`, `R12`, `R1`).
