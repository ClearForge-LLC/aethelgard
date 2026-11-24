# AET-WO-4300k — measurement (§1.1)

Measured 2026-11-24 on this branch at `521dcd06` (= `main` `0faaaec3` + the predictions), host CLI `target/debug/aethelgard` (debug), and the tools image `1e0ab105…` under `~/bin/capped-run --mem 16G` (one container). Scratch is `~/.cache/aet-4300k/`. Nothing under `crates/`, `examples/`, `fab/`, `docs/` or `scenarios/` has changed.

Sub-results, each with its scripts: `sweep/SWEEP.md` (the key-deletion sweep), `blast/BLAST.md` (the host baseline over the 36 specifications, and each new rule's blast radius), `thermal/THERMAL.md` (pad copper, θJA, the fabricator matrix, held pages). Each was delegated to a worker and spot-checked against its artifact (§10).

**PDF reads.** Every number below that gates a §7 stop was read from a rendered page (`~/.cache/aet-4300k/kit/render.py`, PyMuPDF from the local uv cache), never from a text extract. The extracts were used only to find pages.

## M0 · The held documents' hashes

| document | sha256 | pinned | |
|---|---|---|---|
| AMASS XT30PW-M V1.2 (`~/.cache/aet-ds/aet-2304/amass-xt30pw-m/xt30pw-m-spec.pdf`) | `c57b5a37…cdf185` | `AMASS_XT30PW_M_V12_SHA256` (`connectors.rs:157`) | match |
| ST AN2867 Rev 25 (`~/.cache/aet-held/st-AN2867-rev25.pdf`) | `cc868fd2…b5d578` | the WO | match |
| TI SBVS157E TLV803 (`~/.cache/aet-held/ti-tlv803-sbvs157.pdf`) | `1645cfe1…9aabb` | the WO (`1645cfe1…`) | match |
| Abracon ABM8AIG REVISED 06-13-19 (`~/.cache/aet-4240/ds/`) | `96dfa791…4acc0` | `power-parts.json` `documents.ABM8AIG` | match |
| TI SNVSAN3F LMR33630 (`~/.cache/aet-4300d/ds/lmr33630.pdf`) | `3b0920a4…c50c04a` | `power-parts.json` `SNVSAN3F` | match (worker) |
| Vishay 20035 CRCW (`~/.cache/aet-4300d/ds/vishay-crcw.pdf`) | — | `VISHAY_20035` | match (worker) |

No hash differs: no §7 stop.

## M1 · Sentinel-03's baseline

- **`validate`, `solve`, `emit`, `extract`:** all pass on the committed fixture (`specification_sha256` `c85cec0d…`). `extract` judges eight lines (four I2C, four SPI) and **no supply** (`drop_limits` absent). Every condition `solve` prints is in `blast/BLAST.md` §S03, verbatim.
- **`generate hwdef`:** refused `ImuOrientationUndeclared` (U5). P5 holds.
- **`verify --fab-out` (image):** refused in 20 s, before any gate: `UnwitnessedFieldNotAcknowledged`, **280 ledger rows** no `acknowledge_unwitnessed` entry names (first `RCC.AHB4ENR.GPIOCEN`; GPIOC/GPIOE OTYPER/PUPDR, I2C2, SPI2, USART2 …). P4 was wrong: a package is refused today before any gate runs, by the acknowledgement precondition (`4230`).
- **`verify` without `--fab-out` (image):** see M1a.

### M1a · `verify` without `--fab-out`

- **Passes** in **937 s** in the image (16 GiB cap, swap 0). Exit 0, `verdict: pass`, no findings, `judged_nothing: []`. The verdict is 490 KB, kept at `~/.cache/aet-4300k/verify-s03-nofab.json`.
- **Gates:**
  - erc, drc, support and extract pass;
  - 20 `prove` harnesses pass;
  - build and simulate pass;
  - attribute ×7 and the campaign (11 scenarios, every one `matched`) pass.
- **The ran-or-not table's `off` rows** (`1609`) are the gates an absent key turns off today:

  | gate | off by |
  |---|---|
  | `extract.supply` | `power.drop_limits` |
  | `extract.oscillator` | `mcu.hse.crystal` |
  | `simulate.hse_clock` | `mcu.hse` |
  | `drc.pairs`, `extract.pairs` | `diff_pairs` |
  | `gpio` | `pin_devices[].signals` |
  | `adc` | `pin_devices[].analog` |
  | `bridge` | `pin_devices[].bridges` |

  - Of these, `power.drop_limits` and `mcu.hse` are Sentinel-03's to declare (2, 6).
  - `diff_pairs` and `pin_devices` describe parts the board does not have. Their absence is the conservative case (`RUL-14`, 2026-11-17).
- So the pass is real for what ran. **Today's precondition that refuses the package is the ledger's (280 rows), not a gate.**

## M2 · The XT30 drawing (A1)

- **The page:** the sheet's one page, "客户图纸 (Drawing)" (`render/xt30-drawing.png`, 400 dpi crop).
  - The top view shows a 9.00 mm body, 13.95 mm overall. The two contact tails leave the body's far end as the rounded bars past the 9.00 dimension, and the two mounting ears sit near the near end.
  - The end view looks into the mating opening (the two slotted male contacts, 5.00 apart), with the tails and the 2.00 mm pegs on the board side.
  - The hole diagram puts the pegs (2-Ø1.0, 11.00 apart) 10.00 from the contact holes (2-Ø1.7, 5.00 apart).
- **The KiCad footprint** (`AMASS_XT30PW-M_1x02_P2.50mm_Horizontal.kicad_mod`) agrees: pads at y 0 (5.00 apart), pegs at y −10 (x −8 and 3, 11.00 apart), F.Fab body y −13.1 to −4.1 (9.0 long). So **the opening is the body end at y −13.1, and the tails are at +y.**
- **Today:** `place_connectors` puts the courtyard's +y side on the edge, so J4's tails meet the edge and **its opening faces into the board** (J4 at rotation 270 on the emitted board). P11 holds.
- **No §7 stop:** the drawing shows the opening, on the side away from the tails.

## M3 · AN2867 Rev 25 and the crystal

- **The criterion Rev 25 applies to the H7 (§3.4, p. 13, rendered):** "If the oscillation loop maximum critical transconductance parameter (Gm_crit_max) is specified, ensure that it is greater than the oscillation loop critical crystal gain (gmcrit)". The gain margin > 5 applies only where `gm` is specified.
  - **p. 14:** "Gm_crit_max = gm / 5", so the factor 5 is inside DS12117's 1.5 mA/V.
  - **§4.2 Step 1 (p. 26):** "make sure that gmcrit for the oscillation loop is smaller than the specified Gm_crit_max value".
- **gmcrit's formula (p. 13, rendered):** `4 × ESR × (2πF)² × (C0 + CL)²`, assuming CL1 = CL2 and the crystal seeing its CL. It is the same as Rev 10's.
- **The row (Abracon p. 1, rendered):** ESR max 70 Ω at 16.000–19.999 MHz, C0 max 2.0 pF, CL 12 pF, fundamental.
  - gmcrit = 4 · 70 · (2π · 16 MHz)² · (14 pF)² = **0.555 mA/V < 1.5 mA/V**.
  - The ratio Gm_crit_max / gmcrit is 2.70, which is 13.5 as a gain margin (`gm` = 5 · Gm_crit_max).
- **CL (§3.3, pp. 12–13):** CL = CL1·CL2/(CL1 + CL2) + Cs. With 18 + 18 pF the caps give 9 pF, and Cs = 3 pF meets CL 12 pF. Rev 25 states no Cs range: its example uses Cs = 5 pF.
- **The crystal passes Rev 25 on gain margin, CL and ESR: no §7 stop.** P12 is right on the result and wrong on its reason: Rev 25 does not ask 5 of a Gm_crit_max part.
- **Where Rev 25 differs from `clock.rs`'s Rev 10 text:**
  - `clock.rs:279`: "the formula is AN2867 Rev 10 §3.4's" becomes Rev 25 §3.4, p. 13 (same formula).
  - `clock.rs:291`: "AN2867 Rev 10 §3.3, pp. 13–14, assumes CL1 = CL2 and the crystal seeing its CL" becomes Rev 25 §3.3 p. 12 (the CL equation) and §3.4 p. 13 (the assumption).
  - `clock.rs:420-426`: `an2867_margin_hundredths_uncited` is "uncited … only Rev 10 held". Rev 25 p. 14 now cites it, but as Gm_crit_max / gmcrit. That is not what Rev 25 calls a gain margin (`gm` / gmcrit, which is 5 times it). The field's name would mislead once cited.
  - **Strictness:** Rev 25 says Gm_crit_max "greater than" gmcrit, and §4.2 says gmcrit "smaller than". The rule holds at equality (`nano <= GM_CRIT_MAX`). gmcrit is rounded up, so equality is a corner, but the text is strict.
  - **Two steps Rev 25 names that no datasheet computes:** Step 3, the safety factor (§3.8, a negative-resistance measurement), and Step 4, the drive level (§3.5, needs a measured Vpp; `power_parts.rs:428` already says so). Each is a condition for the readiness gate's acknowledgement list, not a failure.

## M4 · The TLV803S (9, Path B)

- **SBVS157E §7.5 Electrical Characteristics, p. 6 (rendered):** VIT− TLV803S **2.87 / 2.93 / 2.99 V** min/typ/max, TJ −40 to 125 °C. **Vhys TLV803S 40 mV, TYP column only, at TJ = 25 °C, IOL = 50 µA: no min, no max.**
- **p. 8 (§8.3):** RESET stays low until VDD exceeds the positive-going threshold `VIT− + Vhys`.
- **Recommended VDD:** 1.1 to 6 V (§7.4 p. 5). Open-drain output with a "10-kΩ to 1-MΩ pullup" (p. 4).
- **Against the board:**
  - The falling minimum, 2.87 V, is at or above W25Q64JV's 2.7 V: met.
  - VDD_3V3's solved minimum is 3,144.727 mV (`solve`: U11's VFB min 0.985 V through R20/R21).
  - Rising = VIT− max + Vhys = 2.99 V + 40 mV (typical) = 3.03 V, 114 mV under it.
- **The held page bounds the rising threshold's maximum only with a typical hysteresis.** That is a "held page does not support a bound you need" (WO §7) for (9)'s second check. Decision **D6** below.
- **W25Q64JV's 2.7 V floor** (Rev J, printed p. 59 = PDF p. 60, worker, page checked): it holds for the 104 MHz class (2.7–3.0 V). At 133 MHz the range is 3.0–3.6 V. The flash's SPI clock decides which floor applies. Today's `consumers` floor is 2,700 mV.

## M5 · (10): the current path, and the branch's numbers

- **The battery net on the emitted board** (`VBAT`): J4.2, C1 (UCZ 47 µF), C10, C11, C20, C21 (the bucks' input ceramics), R50 (the sense's top), U10.2/.3 and U11.2/.3 (VIN, EN). Nothing else.
  - J4.1 is GND.
  - **J5 carries MOTOR1–4 and GND only.** No ESC current crosses Sentinel-03's copper, which agrees with the fixture's `esc.none` source (Scotty, 2026-11-19).
  - **So the whole `VBAT` net is the flight controller's own supply branch.** The FET and fuse go between J4.2 and it, and no motor path exists to refuse.
- **The power tree's own numbers** (`solve`, `power`): `input_steady_milliamps` **2,747**, `input_fault_milliamps` **10,100**.
  - "About twice the branch's maximum input current" is about **5.5 A**: fast-blow, at or above 17.4 V DC rated (4S full charge).
  - Its I²t must pass the hot-plug charge of C1 and the ceramics.
- **Constraints a candidate must meet, at 4S** (`cells.max` 4, 4,350 mV):
  - P-FET:
    - |VDS| ≥ 34.8 V, the doubled pack (see M6), so 40 V class at least, 60 V preferred.
    - VGS held by a zener under its |VGS| max at 17.4 V, and Rds(on) stated at the VGS the 3S floor gives (9.0 V less nothing).
    - ID ≥ the fault current, or the fuse clears first.
    - Rds(on) + fuse R small against the buck headroom, which today counts "no input-path drop" (a `solve` condition: the headroom line will need the drop).
  - Fuse: rated ≥ 17.4 V DC; I²t against the hot-plug inrush; held at the fault current (10.1 A) for less than a buck tolerates.
- **Candidates (not held, not declared; each needs its datasheet fetched by the architect):**
  - P-FET: a −40 V or −60 V, single P-channel in SOT-23 or PowerPAK SO-8, with Rds(on) stated at VGS −4.5 V. Families to look in: Diodes Inc. DMP-series (e.g. DMP6023LSS, −60 V), Vishay Si7461DP (−60 V), onsemi NVTFS5124PL-class.
  - Zener: 12 V SOD-123 (e.g. MMSZ5242B / BZT52C12) with a 100 kΩ gate resistor.
  - Fuse: Littelfuse 466-series 1206 fast-acting 5 A 32 V (0466005), or the 437 series. Its I²t is from the datasheet.
  - None of these part numbers is asserted; they are where to look.

## M6 · 4S, the doubled pack, and A6

- **4S:** 4 × 4,350 = 17,400 mV, doubled 34,800 mV < VIN abs max 38,000 mV (SNVSAN3F p. 6, §7.1). At 6S, doubled, it is 52,200 mV: refused with no clamp.
- **The factor 2:** no held document states it.
  - SNVSAN3F p. 32 says only that the leads' inductance and low-ESR ceramics "can form an under damped resonant circuit, resulting in overvoltage transients", and it recommends 20–100 µF of damping.
  - p. 23's "preferably twice the maximum input voltage" is a capacitor voltage rating, not a transient factor.
  - Searched: every `~/.cache/aet-held/*.txt`, the SNVSAN3F extract.
  - The WO permits labelling it a derivation (an ideal series LC driven by a step from rest peaks at twice the step, and any loss lowers the peak). Decision **D7**.
- **A6, `cin` effective at the pack's maximum** (`solve`, Murata curve):
  - At 6S (26,100 mV), today: 4.092 µF per cap, so three per buck = 12.28 µF (the WO's ~12.3 is this).
  - **At 4S (17,400 mV), a scratch copy with `cells.max` 4: 5.433 µF per cap. Two per buck = 10.87 µF ≥ 10 µF, a margin of 8.7 %. Three per buck = 16.30 µF.**

## M7 · A4: thermal copper (`thermal/THERMAL.md`)

- **The pads:** U10 and U11 each have pad 9 (GND), 2.95 × 4.9 mm on F.Cu (0.145 cm²), with 12 thermal vias (0.7 mm pad, 0.254 mm drill, raised by `emit`'s note).
- **The emitted board has no filled zone.** In1.Cu (GND) and In2.Cu (VDD_3V3) are board-sized outlines (67.74 cm² each), unfilled until KiCad refills. There is no F.Cu or B.Cu pour.
- **Copper joined to each pad, local, without the plane** (cm²):

  | layer | U10 | U11 |
  |---|---|---|
  | F.Cu | 0.146 | 0.244 |
  | In1.Cu | 0.048 | 0.071 |
  | In2.Cu | 0.048 | 0.071 |
  | B.Cu | 0.048 | 0.071 |

  - With the GND plane, In1.Cu joins about 64 cm² once filled (an estimate, after antipads; not a fill).
  - In2.Cu is VDD_3V3 and joins nothing. P9 holds.
- **The screen** (`power.rs:2631-2715`) reads only the declared 20 cm² and picks the figure's point at or under it. A measured per-layer figure under the 7 cm² floor (`BuckJunctionOverLimit`) would refuse on F.Cu and B.Cu today.
- **θJA at source (SNVSAN3F):**
  - **Table 7.4, p. 7: RθJA (DDA) 42.9 °C/W**, JESD 51-7 on a 4-layer JEDEC board, "only valid for comparison with other packages and can not be used for design purposes". The sub-reviewer's 43 °C/W is this, rounded. P10 holds.
  - **Figure 9-3, p. 25** (the row's figure): "Typical RθJA versus Copper Area for a Four-Layer Board", copper area per layer, 2 oz outer and 1 oz inner, "for illustration purposes only". It runs from about 42 °C/W at 6 cm² to about 24.5 °C/W at 58 cm².
  - The row holds 7 → 42.0, 20 → 28.0 and 58 → 25.0 °C/W.

## M8 · The fabricator (`thermal/THERMAL.md` §3)

Bounds a 4-layer specification needs for every gate:
- finished thickness, min and max;
- three dielectric minima;
- `copper_max_um` and `relative_permittivity_max_permille`;
- the supply gate's `outer_copper_min_um`, `plane_copper_min_um`, `plating_min_um` and `copper_temperature_max_c`;
- `fab_tolerances`: hole position, hole size ±, track width, via ring.

Read live 2026-11-24 (worker's WebFetch, quoted in THERMAL.md):

| bound | JLCPCB | OSH Park |
|---|---|---|
| thickness min/max | yes, "≥1.0mm ±10%" | no, 1.6 mm nominal |
| dielectric minima | no, nominals only | yes, ±0.0202 / ±0.0991 mm |
| copper max | no | no |
| permittivity max | no, nominals | no, 3.61 nominal |
| outer / plane copper min | no | no |
| plating min | no, "average 18 µm" | no, 1 mil nominal |
| copper temperature max | no | no |
| hole position | yes, ±0.075 mm | yes, 2 mil |
| hole size | yes, +0.13 / −0.08 mm | yes, ±2.5 mil |
| track width | yes, ±20 % | no |
| via ring | yes | yes, 4 mil |

**No fabricator publishes every bound.** Eight bounds (copper maximum, permittivity maximum, three copper or plating minima, copper temperature) come only from a standard or the engine's stated condition, as today's fixture does for two of them. P14 is wrong on JLCPCB: it publishes no dielectric tolerance. Sentinel-03 today mixes OSH Park's dielectrics with JLCPCB's tolerances. Decision **(b)**.

## M9 · The carried items

- **4410 D3 / Q11:** U5 has no declared orientation and the board no `forward`. With `board.forward: top`, U5 at each `x`, `emit` then `extract` gave (IMU_SCK / IMU_MOSI fF, bound 30,000):

  | U5 `x` | result |
  |---|---|
  | right | 25,094 / 27,152, pass |
  | forward | **refused, `CapacitanceOverBound` IMU_SCK 31,879** |
  | left | 22,021 / 18,544, pass |
  | aft | 24,340 / 23,041, pass |

  - With `x: right` and the crystal declared (Y1 drawn), `extract` passes: IMU_SCK 21,537, IMU_MOSI 21,398.
  - J4's 180° turn moves the board, so these are re-measured after it. Which edge is forward is U5's drawn fact: the architect's (D9).
- **With the keys and the crystal:** `solve` passes, and `generate hwdef` writes `OSCILLATOR_HZ 16000000`, the IMU line `ROTATION_ROLL_180_YAW_90` for `x: right`, and `PC0 BATT_VOLTAGE_SENS ADC1`.
- **4300j R8:** the link's BOM row reads `0`, not `0 CRCW0402_Z_0`. Still so at `0faaaec3`.
- **1609 F2:** `RingRule::of` is `None` without `fab_tolerances` (`stack.rs:64`), so the ring rule is a threshold, not a gate. Sentinel-03 declares `fab_tolerances`, so its ring is judged today (two `emit` notes raise U10's and U11's pad-9 via pads to 0.7 mm). The gap is a specification that omits the block, which (2) closes for new packages.
- **A13's false line, measured:** `solve` prints "VDD_3V3: the supply currents of U3, U7, U4, U5 are not in its load, which counts only declared `loads` and the MCU's row (A1)". But `power.loads` names U3, U4, U5 and U7 by `reference`. False, as the review says.
- **A13's gap, measured:** U11's junction line says "550 mW of loss at 747 mA (declared: … at 0.68 A …679 mA)", so sourced 679 mA against solved 747 mA, a gap of 68 mA. U10's is 2,000 against 2,000.

## M10 · Which bytes move, by board

- **The last specification bump (4300j, 28 → 29, `465bd860`)** moved `fab/sentinel-01-bench/package/MANIFEST.json`'s `specification_sha256`, `topology_sha256`, `verdict_sha256` and the `FAB-NOTES.md` hash, plus `FAB-NOTES.md` itself. **No ELF.** The ELF has moved only with firmware changes (`4240v`, `4241`, `4240e`).
- **Gerbers and the drill file carry no specification hash or version** (`%TF.ProjectId,input,…`, no hash). So no `ORDERED.sha256` byte moves by a bump.
- **Predicted for this build:** Sentinel-03's topology and board (J4's turn, the crystal, the FET and fuse, the supervisor, `cin`); every committed specification's version literal; Sentinel-01's MANIFEST hash lines and `FAB-NOTES.md`.
  - **`FAB-NOTES.md` is outside the fence's letter** ("MANIFEST.json's hash lines"). It moved at the last bump too.
  - A new precondition row in every MANIFEST is decision (c).
- P17 was wrong on the ELF and missed FAB-NOTES.

## M11 · The key-deletion sweep (`sweep/SWEEP.md`)

- **Method:**
  - Every `Option`, defaulted and list field was enumerated from the IR types (`sweep/enumerate.py`).
  - Variants: delete, `null`, `[]`/`{}`, rename. 525 rows.
  - Carriers: Sentinel-03, a copy gated with `drop_limits` and supply_tier1's test minima, and the committed specification that holds each key otherwise.
  - Detector: what `validate`, `solve`, `emit`, `extract` and `check speeds` judged (the `ran` rows, condition lines, judged counts, the BOM), not exit status.
- **Five keys relax a judgement with `validate` at exit 0:**

  | key | type | how | what goes |
  |---|---|---|---|
  | `power.drop_limits` | `Option`, `power.rs:128` | delete, `[]` (`null` refused) | `extract.supply` judged 1 → `off`; VDD_3V3's 90.0/165 mV row |
  | `board.stackup.fab_tolerances` | `Option`, `board.rs:184` | delete | the ring rule (`stack.rs:64`) and the capacitance ladder (`schematic.rs:957`); the two pad-9 ring notes. **Nothing says off** |
  | `diff_pairs` | required, `[]` accepted, `lib.rs:197` | `[]` | `extract.pairs` judged 2 → `off` (on pair-standin) |
  | `power.conditions.dc_bias_derating` | required, `[]` accepted, `power.rs:147` | `[]` | "`cout` would hold … effective" → "judged at rated capacitance" (U10, U11). **Nothing says off** |
  | `power.loads` | required, `[]` accepted, `power.rs:116` | `[]` | VDD_5V load 2,000 → 0 mA; the junction line "at 0 mA". **Nothing says off**, and A13's unloaded rail is neither refused nor named |

  Both spot-checked by `solve` on scratch copies: derating `[]` and loads `[]`.
- **Conservative** (absence removes a part with its judgement, or tightens): `mcu.hse`, `mcu.reset_switch`, `power.sense`, `test_points`, `i2c_devices[].pin_capacitance` (the bound tightens 20,000 → 30,000 fF on the term), `board.drawn`.
- **Refused downstream when partly declared:**
  - `fab_tolerances` partial is `FabToleranceUndeclared` at `emit`;
  - a stackup minimum missing with `drop_limits` is `SupplyInputUndeclared`.
  - But `hole_size_minus_um` and `track_width_permille` are read only by the supply gate (`supply.rs:1384,1386`), so without `drop_limits` their absence changes nothing.
- **Not swept:** `check support`/`erc`/`drc` (no KiCad on the host), and `generate`, `verify`, `build`, `prove`, `simulate`, `check pinout`.
  - `verify`'s own ran table (M1a) covers its `off` rows: `mcu.hse` (`simulate.hse_clock`) and `pin_devices[]` (gpio, adc, bridge), all conservative for a board without those parts.
- **Cost:** a gated `extract` took 5 min 22 s in release on the box. A readiness gate re-running `extract` per variant would be slow. The gate should read the specification's keys, not re-run gates.

## M12 · Blast radius of each new rule (`blast/BLAST.md`)

**Host baseline, 36 specifications:** 20 pass every step. Seven fail `extract` with `capacitance_over_declared_bound` on `main` as it stands (d3-24-i2c-12-spi, four-port, hse-crystal-16, sentinel-01-ext-bmp581, sentinel-02-shaped, serial-four, solve-stress; nothing here changed them), and nine are refused by name before it. The table, with per-board outputs, is in BLAST.md. Every board that reaches `extract` has `extract.supply` off (`power.drop_limits`). **Two specifications declare `power`:** `sentinel-03` and the IR fixture `sentinel-02-power`.

| new rule | specifications it would refuse or force to edit |
|---|---|
| (2) `drop_limits` required with `power` | 2: sentinel-03, sentinel-02-power. **Measured on Sentinel-03** with supply_tier1's four "test:" minima: VDD_3V3 at 165 mV gives a 90.0 mV bound at U1 (747 mA), passing. **VDD_5V cannot be limited:** `SupplyTerminalMissing`, "the load U1 has no pin on `VDD_5V`, so the drop limit on `VDD_5V` judges nothing". The 5 V load leaves by J6.1 and J7.1. sentinel-02-power is refused `TranslatorPinUnbounded` before the supply gate, so its drop is unmeasured (roadmap §7.1 records 206–544 mV on its 2-layer form). |
| (3) one fabricator | 1 mixes: sentinel-03 (OSH Park dielectrics, JLCPCB tolerances). OSH Park only: d1-24-i2c, d3-24-i2c-12-spi, four-layer, four-layer-power; JLCPCB only: sentinel-02-power. The rest name none. |
| (6) doubled pack ≤ every VIN abs max, else a clamp | 2: sentinel-03 and sentinel-02-power, both 6S (52,200 mV > 38,000 mV) with no clamp. |
| (9) VCC min ≤ the reset's falling minimum, no supervisor | **33 of 36.** DS12117 Rev 10 Table 16, p. 107 (read): falling minima BOR0 (POR/PDR) 1.58 V, BOR1 1.95 V, BOR2 2.25 V, BOR3 2.54 V. At BOR3, 14 specifications are refused, each by W25Q64JV's 2,700 mV, **`examples/sentinel-01.json` among them**. At BOR0, 19 more are refused, every 1.71 V device above 1.58 V. The 3 that pass have no device on the MCU rail. |
| (10) a reversed input refused for want of protection | 2: sentinel-03 and sentinel-02-power (both `power.input` XT30). |
| A13 a regulated rail with no load | 0. Every regulated domain has a declared load. |

**So (9) as a `validate` rule refuses the ordered board** and nearly the whole corpus, and the bench fixtures at BOR0 fall to a 1.71 V device. It is decision D12.

## Predictions, scored

| P | held? | |
|---|---|---|
| P1 | yes | 36 |
| P2 | partly | the conditions print from `solve`, not `emit`; the A13 false line is there (M9) |
| P3 | yes | `extract.supply` off |
| P4 | **no** | `verify --fab-out` is refused before any gate, by 280 unacknowledged ledger rows; without `--fab-out` every gate passes (M1, M1a) |
| P5 | yes | `ImuOrientationUndeclared` (U5) |
| P6 | partly | `drop_limits` and `fab_tolerances` yes; `hot_plug`, `junction`, `offboard`, `net_constraints` and `max_outline_um` no; `dc_bias_derating` yes, as `[]` |
| P7 | yes | |
| P8 | yes | `power.loads` `[]` and `diff_pairs` `[]` |
| P9 | yes | F.Cu 0.15–0.24 cm², via rings elsewhere, planes unfilled; In2 is VDD_3V3, not GND |
| P10 | yes | 42.9 °C/W, JESD 51-7, Table 7.4 p. 7 |
| P11 | yes | opening at −y, into the board |
| P12 | result yes, reason no | Rev 25 asks Gm_crit_max > gmcrit, not a margin of 5 |
| P13 | partly | same sections and formula; the margin field's meaning differs; two unmeasurable steps added |
| P14 | **no** | JLCPCB publishes no dielectric tolerance; no fabricator publishes every bound |
| P15 | yes | |
| P16 | **no** | 2,747 mA steady (solve's number), so a fuse near 5.5 A, not 2.5–3 A |
| P17 | **no** on the ELF | no ELF moves at a bump; `FAB-NOTES.md` does |
