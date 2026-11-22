# AET-WO-1216 — the measurement (§1.1–§1.4), against PREDICTIONS.md

Run 2026-11-22 at `main` `9cb87b4c` plus this spike. The binary is `aethelgard` built from `9cb87b4c` (release, `CARGO_TARGET_DIR` in scratch). Predictions were committed first, at `11d10be3`. The rules were committed before `judge.py` first ran, at `b66d5d7b`.

## The kit

| file | what it does |
|---|---|
| `universe.sh` | Lists the committed boards: every file `git ls-files '*.json'` lists outside `spike/` and `docs/` with `schema_version` 24, an `mcu` object and no `base`. **34 files.** |
| `run.sh` | Runs `validate`, `solve` and `emit` on each, then walk 2 on every emitted board. Writes `out/boards.tsv` and `out/drawn/*.tsv`. |
| `drawn.py` | **Walk 2**, from the drawn netlist. It reads an emitted `.kicad_pcb` and knows nothing of the specification's fields. For every MCU pad that is not a supply or ground pad (pad → pin and structure by DS12117 Table 8, `spike/1211e/out/table8-visual.tsv`), it lists every other pad on the net, the rails that pad's part touches, and one hop through any two-terminal part. A row is off-domain when the peer touches a rail other than GND and `mcu.supply`, or is a connector or test point. It flags by rail, never by reference prefix, so a part named `S1000` (the solve stress fixture's flashes) is judged the same way as `U4`. |
| `spec.py` | **Walk 1**, from the specification and its solved topology. Per source class, it lists every path onto an MCU net from anything but `mcu.supply` or GND: I2C pulls, devices on another domain, the battery sense divider, crossings, control wires, pin-device pulls, outputs and current outputs, connectors, test points and a foreign clock. For each it gives the worst open-circuit voltage while `mcu.supply` < 1 V, the series ohms and the source, or "not stated". |
| `compare.py` | Reconciles the two walks on every drawn board, in both directions: `out/compare.tsv`. |
| `judge.py`, `RULES.md`, `sequences/` | The options as rules (§4). |
| `summary.py` | One line per board: `out/summary.md`. |

Readers read the CLI's output of `main`'s crates (`solve` JSON, emitted `.kicad_pcb`). They do not link the crates, so nothing under `spike/1216/` has a `Cargo.toml` (FEEDBACK §7, D1).

## 1. Every committed board's off-domain paths (§1.2)

- **The run:**
  - 25 of 34 boards emit a board.
  - 4 draw none by declaration (`DrawsNoBoard`: D1, `hse-external-8`, `imu-baro-1v8`, `sentinel-02-bench-hse8`).
  - `validate` refuses 2: D2 (`PartLimitUndeclared`, by design) and `invalid-zero-domain` (`ZeroNominalVoltage`).
  - `solve` refuses 3, all `solve/` stress fixtures.
- **The 9 that do not emit** were walked from the specification alone. For the refused ones, the pins are the declared pins or "unsolved".
- **The reconciliation:**
  - Walk 1 and walk 2 agree on **388 of 388** rows on the 25 drawn boards.
  - The first run disagreed on one: `sentinel-02-power` `IMU_CS`, the TXU0304's A3 input, which walk 1 had not listed among the crossing's A-side nets. Walk 1 was fixed (it now lists the CS) and both re-ran.
- **Part classes on every drawn board:** C, J, L, R, SW, TP, U, Y and the stress fixture's S. **No diode is drawn anywhere**, so there is no ESD clamp to walk.

**Paths, by class, over the 34 boards** (`out/spec/*.tsv`; `can_lead` = yes):

| class | boards | worst at VDD < 1 V | source |
|---|---|---|---|
| battery sense divider | `sentinel-03`, `sentinel-02-power` | **2536 mV** on PF3 (FT_ha), through 9091 Ω | 6 × 4350 mV = 26.1 V; R50/R51 100 k/10 k CRCW0603, each ±3.65 % (1 % + 100 ppm/K × 65 K + 2 % drift, the rows' figures). It equals the solved topology's own `VREF+ (VBAT_SENSE full scale)` floor, 2536 mV, a third read that agrees. |
| pin-device current output | D1 (`d1-drv8876.json`) | **4665 mV** on PF3 (FT_ha), through 1555 Ω, **if the driver drives**: on the held bound, not a stated maximum | DRV8876 IPROPI into R2 1.5 kΩ: IIPROPI 0–3 mA, SLVSDS7B p. 4 §6.3 (**recommended operating**; the engine's `output_max_ua`) × R2 at 1555 Ω, which is 1 % + 100 ppm/K × 65 K over D1's declared −20 to 85 °C + 2 % drift. The same table allows a 3.5 A peak output current. At the typical 1000 µA/A that is 3.5 mA, or 5442 mV (no AERR is stated above 2 A). So the held bound is not IPROPI's worst. No page states one: the abs max is 5.75 V. See §2. |
| connector pin on an MCU net | 33 of 34 (all but `invalid-zero-domain`) | **not stated** | No specification bounds what an off-board end drives. DS-009 prints +3.3V in its Volt column for every TELEM, GPS and I2C signal pin (pp. 6–9): a nominal level with no maximum, carried in each such row's source. **6** boards carry `VDD_5V` on a connector beside MCU signals: `connectors`, `hse-crystal-16`, `hse-external-8`, `sentinel-02-shaped`, `sentinel-03` and `serial-four`. A peer those connectors power can drive the MCU pins while 3V3 is down. On the other **27**, the connectors carry no rail but `mcu.supply` (a self-powered peer, such as a USB-serial adapter or a debug probe, can still drive them). |
| test point on an MCU net | the `sentinel-01` family, `sentinel-03` | not stated | a probe, when attached |
| foreign clock on PH0 | `hse-external-8`, `sentinel-02-bench-hse8` | not stated | the bench's external 8 MHz (ST-LINK MCO), off-board |
| I2C pull to another domain | **none** | — | all 98 bus pulls in these 34 files (2 per bus) are on `mcu.supply`; walk 1 found no `i2c_pull` row |
| pin-device pull to another domain | **none** | — | D1's nFAULT: R3 to `VDD_3V3` = `mcu.supply`. D2's nFAULT: the MCU's own pull-up. |
| crossing (TXU0304, `sentinel-02-power`) | **not off-domain** | ≤ VDD | A side on `VDD_3V3` = `mcu.supply`; B side `VDD_1V8` from the TLV755P fed by 3V3 (downstream); outputs Hi-Z when either VCC < 100 mV (SCES935A p. 20), unstated from 100 mV to 1.08 V, and powered from VCCA in any case |
| power-tree control wire | **none** | — | every EN is strapped to its input, every PG open, every OE to VCCA |
| device on a bus from another domain | **none** outside a crossing | — | — |

**The pin structures these paths land on:** FT, except for these:
- `sentinel-03`'s `MOTOR1` on PE9, **TT_ha**, at J5 and TP19 (FEEDBACK F1);
- test points on PA5 (**TT_ha**; Table 11 and Table 48 admit no injection there, −0/0) and PA7 (**TT_a**) on `sentinel-01`, `sentinel-01-bench` and `sentinel-01-ext-bmp581`;
- NRST (RST) and BOOT0 (B) on J2 and their test points on those three boards and `sentinel-03`.

At VDD = 0 every one of these is limited to 4.0 V, except BOOT0 at 9.0 V (Table 10). The per-board table is `out/summary.md`.

### Cross-checked by hand on Sentinel-02, Sentinel-03, D1 and D2

Read from the specification JSON line by line, not from the readers:

- **`sentinel-03.json`:**
  - **Domains:** 3V3 (U11) and 5V (U10), both LMR33630 from the battery, each with EN on VIN and PG open.
  - **Paths:**
    - VBAT_SENSE on PF3 through R50/R51;
    - J1 (console, no rail);
    - J2 (SWD, NRST, BOOT0, 3V3);
    - J5 (motors 1–4, no rail; `esc: none`, no ESC fed through the board);
    - J6/J7 (CRSF, MAVLINK; **VDD_5V** on pin 1);
    - J8 (EXT I2C; **VDD_5V** on pin 1; its 10 k pulls are on 3V3);
    - 19 test points on MCU nets.
  - **Not off-domain:** the I2C pulls (3V3), the flash and IMU (3V3).
  - **Agrees with both walks:** 36 rows.
- **`sentinel-02-power.json`:**
  - **Domains:** 3V3 (U11), 5V (U10), 1V8 (U12 TLV755P from 3V3).
  - **The crossing:** U13 TXU0304, A on 3V3, B on 1V8, OE to VCCA, R40 CS pull to 3V3; the ICM20948 on 1V8 behind it.
  - **Paths:** VBAT_SENSE on PF3; J2 (SWD, 3V3).
  - **The 5V rail** reaches no MCU pin: no connector or part on it touches an MCU net.
  - **Agrees:** 3 paths.
- **`d1-drv8876.json`:**
  - **Domains:** 3V3 by J3.1, VMOTOR by J5.3, so two external supplies and no tree.
  - **U2 DRV8876:**
    - nSLEEP is an MCU output with the driver's internal pull-down;
    - nFAULT is open drain, R3 to 3V3;
    - IPROPI → R2 → PF3;
    - VREF is tied to 3V3;
    - PMODE and IMODE are tied to GND;
    - EN/IN1 and PH/IN2 are MCU outputs.
  - **Paths:** IPROPI, J1 (console), J2 (SWD), J4 (servo).
  - **Agrees:** 6.
- **`d2-drv8833.json`:**
  - **Refused by `validate`:** `PartLimitUndeclared` on `input_pwm_max_hz`, the fixture's purpose.
  - **Domains:** 3V3 by J3.1, VMOTOR by J5.3.
  - **U2 DRV8833:** nFAULT is open drain with the MCU's pull-up. It has no current output (xISEN is strapped to GND).
  - **Paths:** J1, J2, J4.
  - **Agrees:** 5.

## 2. D1's IPROPI, read at source

- **What is stated:**
  - The abs max is −0.3 to 5.75 V **and** −0.3 to VVM + 0.3 V, two rows for one pin (SLVSDS7B p. 4, §6.1, rendered).
  - IIPROPI is 0 to 3 mA, recommended (p. 4, §6.3).
  - IPROPI mirrors the low-side current (p. 12, §7.3.3.1).
  - An "internal IPROPI voltage clamp circuit to limit VIPROPI with respect to VVREF" (p. 12), **with no figure**.
  - Sleep "disables the H-bridge, charge pump, internal 5-V regulator, and internal logic" (p. 18).
- **Not stated:** IPROPI's level in sleep, and the clamp's level.
- **The MCU's limit:** 4.0 V on FT_ha (Table 10, p. 103; note 5, an FT_a pin used by the ADC: 4 V).
- **The reading:**
  - With the MCU down, the driver's nSLEEP sees only the unpowered MCU pin and the driver's 100 kΩ typical pull-down. The driver should sleep and IPROPI should carry nothing, but no page says so.
  - With VREF tied to the very rail that is down, the clamp would hold VIPROPI near 0 V, but no page gives the figure.
  - The 4665 mV is the bound the engine holds (`pin_parts` `output_max_ua` 3000), applied as if the driver were driving. It is a recommended-operating figure, not IPROPI's maximum. The 3.5 A peak output current gives 5442 mV at the typical mirror ratio, and the abs max is 5.75 V.
- **Brown-out is not covered:** with VDD between nSLEEP's VIL max (0.8 V at VVM ≥ 5 V, which applies to D1's 12 V) and VIH min 1.5 V (p. 5), the enable is undefined while VDD is still under 1 V.
- **Reported on the bus at once.** It is not §7's immediate stop, because option (c) judges it; see FEEDBACK §3.

## 3. What the held parts state about order (§1.3)

Each figure was read in the held text, then the load-bearing rows were checked on the rendered page. The pages are in `~/.cache/aet-1216/render/`; the hashes match `power-parts.json` `documents`.

| part (where) | enable | start-up | power-good | order it can fix |
|---|---|---|---|---|
| **LMR33630ADDA** (SNVSAN3F rev F): U10 5 V, U11 3V3 on `sentinel-03` and `sentinel-02-power` | VEN-H rising 1.2/1.231/1.26 V; hysteresis 100 mV typ; VEN-VCC-H max 1 V (standby); VEN-VCC-L min 0.3 V (off) (p. 7, §7.5). "Can be connected directly to VIN … must not be allowed to float" (p. 13 §8.3.2; p. 5). Abs max EN −0.3 to VIN + 0.3 V (p. 6). | tSS internal 2.9/4/6 ms (p. 8 §7.5); "the rise time of the output voltage is about 4 ms" (p. 13). Input UVLO: **no threshold is tabulated**. The minimum operating input is 3.8 V max (p. 7). VCC starts at "about 3.7 V" and stops at "about 3 V", which "roughly represent the input voltage levels" (p. 15, §8.3.4), typical only. Pre-bias and output discharge: not stated. | Open drain; thresholds 90–110 % of VFB across the four rows (p. 8); tPG 60–170 µs; "about 4 ms (typical)" from EN to PG high at start-up (p. 12); valid with EN low for VIN ≥ 2 V max (VIN-PG, p. 8); abs max 0–22 V, recommended 0–18 V (pp. 6–7). | **None as declared**: both EN on VIN, PG open. The two start in an unstated order with an unstated skew (no UVLO limits). **No page states a sequencing method.** One the held pages allow: PG pulled up to its own VOUT through 100 kΩ (p. 12) and wired to the other buck's EN, which then rises past VEN-H after PG. PG cannot be pulled to the battery: 6S reaches 26.1 V against PG's 22 V abs max. |
| **TLV75518PDBV** (SBVS320D rev D): U12 1V8 from 3V3 on `sentinel-02-power` | VHI min 1 V, VLO max 0.3 V (p. 6); "connect EN to IN" (p. 12); EN abs max −0.3 to 6.0 V (p. 4). | VUVLO rising 1.21/1.3/1.44 V, hysteresis 40 mV typ (p. 6). tSTR 550 µs, **typical only** (p. 6). 120 Ω typ active pull-down, also below UVLO (pp. 6, 12). VOUT ≤ VIN + 0.3 V abs max (p. 4). | none | **Fixed by the tree:** its input is 3V3, so 1V8 cannot lead `mcu.supply`. |
| **TXU0304PW** (SCES935A rev A): U13 on `sentinel-02-power` | OE "can be left floating or externally pulled down to ground to ensure the high-impedance state … during power up or power down" (p. 20); strapped to VCCA here. | "Either supply rail may be powered on or off in any order" (pp. 20, 22). Outputs Hi-Z when either VCC < 100 mV (pp. 1, 20, 21); 100 mV to 1.08 V **not stated**. Ioff ±2.5 µA max to 125 °C (p. 8). | — | Needs none (any order). |
| **DRV8876 / DRV8833** (D1, D2) | nSLEEP with an internal pull-down (SLVSDS7B p. 3; DRV8833 SLVSAR1E p. 3, RPD 500 kΩ typ) | VM UVLO rising 4.3/4.45/4.6 V (DRV8876 p. 6) | nFAULT open drain | None: VM and the logic rail come from two connectors. **No page orders them.** |
| **STM32H753** (DS12117 Rev 10) | — | §3.5.1 p. 27: with VDD < 1 V, VDDA, VDD33USB and VDD50USB stay below VDD + 300 mV; above 1 V all supplies are independent; at power-down VDD may sit below the others only while the energy into the MCU stays under 1 mJ. **Nothing is stated about an I/O pin's voltage with VDD absent** beyond Table 10's formula. | — | — |

**Where the declared tree fixes an order:**
- `sentinel-02-power`'s 1V8 follows 3V3 (the LDO's input).
- Nothing else is fixed:
  - the two bucks race on both boards with a tree;
  - every other multi-domain board has no tree, so no source for its other domains.

## 4. The options, run (§1.4)

**The run:** `judge.py` over the 34 boards, giving `out/options.tsv`.
- **A fix after the first run:** the sidecar rule stopped reading steps at `mcu.supply`'s own step, so it never checked a later step's source. `RULES.md` says every step is checked, so the rule was fixed to match it and re-run. The fix moved one count: `5v-after-3v3-unsourced`, 0 refused before and 1 after.

| option | refused | which, by kind |
|---|---|---|
| **c1** VDD = 0, stated paths | **1 / 34** | D1: `OffDomainAboveLimit` DRV_IPROPI@PF3 4665 > 4000 mV, on the held bound. If IPROPI is read as having no stated maximum (§2), c1 refuses **0** and D1 moves to c2's unbounded column. |
| **c2** + unbounded refused | **33 / 34** | every board with a connector pin on an MCU net (`OffDomainUnbounded`); D1 also above |
| **b1** EN/PG order, every undeclared or racing domain | **10 / 34** | `RailsRace` VDD_5V: `sentinel-03`, `sentinel-02-power`. `SourceUndeclared`: `connectors`, `hse-crystal-16`, `hse-external-8`, `sentinel-02-own`, `sentinel-02-shaped`, `serial-four` (VDD_5V, VDD_1V8); D1, D2 (VMOTOR) |
| **b2** only where the rail reaches an MCU pin | **7 / 34** | as b1 minus `sentinel-02-power` (its 5V reaches no MCU pin), `sentinel-02-own` and D2 |
| **a1** sequence required with any second domain or a tree | **10 / 34** | `SequenceUndeclared`: the same 10 as b1 |
| **a2** required only where a non-supply rail reaches a pin | **8 / 34** | b2's 7 + `sentinel-02-power` (its battery divider) |
| **a-sidecar** `as-held` (four hand boards) | **1 / 4** | D1 (VMOTOR and 3V3 in one step: IPROPI 4665 > 4000). `sentinel-03` and `sentinel-02-power` pass (battery first: the divider at 2536 mV; both bucks in one step: 5V reaches only off-board ends, unjudged). D2 passes. |
| a-sidecar `5v-after-3v3-unsourced` (`sentinel-03`) | 1 / 1 | `SequenceStepUnsourced`: the order a designer wants has no held source |
| a-sidecar `bench-procedure` (D1) | 0 / 1 | 3V3 declared first by a bench procedure: IPROPI is then judged only with the MCU powered |

## 5. Predictions beside results

| # | prediction (`11d10be3`) | measured | holds |
|---|---|---|---|
| universe | 34 files | 34 | **holds** |
| 1 | at least 30 of 34 have a path, the connector ones "not stated" | 33 of 34; every connector path is unstated | **holds** |
| 2 | sense divider on `sentinel-03` and `sentinel-02-power`, about 2.4 V | 2536 mV (the TCR and drift corners add 0.12 V to the tolerance-only figure) | **holds**, figure 5 % low |
| 2 | D1 IPROPI: 0 V asleep if stated, else 3 mA × R2 ≈ 4.6 V | sleep level not stated; 4665 mV | **holds** |
| 2 | nFAULT, crossing, I2C pulls, control wires not off-domain; foreign clock on two boards | as predicted | **holds** |
| 3 | no stated path over 4.0 V but D1's candidate | D1 only | **holds** |
| 4 | the bucks' order fixed by no page; LDO after 3V3; no stated soft-start minimum | no order, LDO after 3V3; **tSS min 2.9 ms is stated** | **one part wrong** (a minimum is stated) |
| 5 | (a) ≈ 10; (b) ≈ 10, the two trees race; (c) 0–1 or ≈ 30 | a1 10, a2 8; b1 10 (both trees), b2 7 (one tree); c1 1, c2 33 | **holds** |

## 6. Corrections after the adversarial pass

The Opus reviewer's report is `out/adversarial.md`. It re-ran the readers and `judge.py` and got byte-identical outputs, then found these, each checked at source and applied:
- **Board counts:** 25 boards draw and 9 do not, not 27 and 7.
- **D1's IPROPI bound:** `spec.py` now takes the resistor's temperature excursion from the pin device's declared range (−20 to 85 °C), as the engine does. The bound is 4665 mV through 1555 Ω, no longer 4683 through 1561.
- **The 3 mA is not IPROPI's worst:** the 3.5 A peak output current is noted (§2).
- **DS-009's +3.3V (nominal)** is now carried in its connector rows.
- **`esc` read backwards:** `esc: {"none": …}` declares no ESC fed through the board, so the 4 ESC rows on `sentinel-03` are removed. Its motor nets still reach J5 as connector rows.
- **TT and other non-FT pins** are listed (§1).
- **Citations:**
  - nSLEEP VIL is 0.8 V at VVM ≥ 5 V;
  - the LMR33630 UVLO section is §8.3.4, and its 3.8 V minimum operating input is stated;
  - PG thresholds span 90–110 %;
  - 98 I2C pulls in these 34 files.
- **The battery's connector name:** `VBAT` is now read as `battery` in `powered_by`. No committed board carries it.
- **Dead code:** a loop in `spec.py` removed.

**What did not move:** every option count (§4), and 388/388 reconciliation.
