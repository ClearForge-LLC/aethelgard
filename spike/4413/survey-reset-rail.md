# AET-WO-4413 survey: a reset for VDD_3V3_RF (read-only; no repo file edited, no cargo run)

Branch wo/aet-4413-s03-rf-reset-ground, repo ~/aethelgard-4310. Held copies in ~/.cache/aet-held/.
Page numbers are the printed pages the held text shows (the repo's own citations use the same).

## Answer first

1. **No TLV803-family variant fits, at any set point.** The family has four thresholds (Z, R, S, M). The only one near
   3 V is S, with VIT- min 2870 mV, which is 130 mV under U30's 3000 mV. U30's VDD33 minimum is a fixed figure, so
   moving the rail does not fix that. M (VIT- 4280 to 4480 mV) is above U30's 3.6 V maximum. No other supervisor is
   held anywhere (power-parts.json holds one supervisor row, TLV803SDBZR).
2. **The IR cannot express a second supervisor.** `mcu.reset_supervisor` is a single optional object under `mcu`.
   It has no rail field and no output target, and every consumer is hard-wired to `spec.mcu.supply` and `NRST`. U30's
   EN is modelled only as pull-up plus capacitor. U31's OE can only be strapped or `wire`d to a net the engine does
   not judge. Details in section 2.
3. **The rail today is 3144.727 to 3417.204 mV** (formula and replica below). `VDD_3V3_RF` is declared 3300 mV
   +/-50 per mille, so the domain is 3135 to 3465 mV and the solved band must sit inside it.
4. **The upper limit for the rail maximum is 3600 mV.** U30's Table 6-1 absolute maximum and Table 6-2 recommended
   maximum are both 3.6 V, and U31.B's stated rows go to 3.6 V. The domain's present 3465 mV ceiling is a spec
   choice, not a part limit. The solved band is about 270 to 290 mV wide for every E96 pair, so a set point cannot
   make a part that is not held fit.
5. **Corrections to the brief.**
   - Direction of the rule: in `supervisor.rs`, a part **passes when its minimum is at or below the monitor's
     falling minimum** (`Some(min) if *min > reset_min => Floor`, else `Pass`). The brief says the reverse. Item 1's
     test (VIT- min >= 3000 mV) is the correct one.
   - Where the 144 mV figure comes from: the spec's `drop_limits` for VDD_3V3_RF is 144 mV, not the MCU rail's 110 mV.

## 1. Supervisors

### 1a. Rows the repo holds (power-parts.json)

Only one: **TLV803SDBZR** (SOT-23-3; pins GND 1, RESET 2, VDD 3; lifecycle active; TJ -40 to 125 C).

| field | value | citation in the row |
|---|---|---|
| VIT- (falling) | min 2870, max 2990 mV (typ 2930 not stored) | SBVS157E p. 6, 7.5, TJ -40 to +125 C |
| hysteresis | 40 mV, **typical column only**, at TJ 25 C, IOL 50 uA | p. 6, 7.5 |
| VDD | 1100 to 6000 mV | p. 5, 7.4 |
| output | open-drain RESET, pull-up 10 kOhm to 1 MOhm | p. 4, Pin Functions |
| delay | 120 / 200 / 280 ms at VDD >= VIT- + 0.2 V | p. 6, 7.6 (td) |

No other supervisor row exists in power-parts.json or parts.json.

### 1b. Every variant in the held TLV803 datasheet

Source: ti-tlv803-sbvs157.txt, SBVS157E, Dec 2020. Table 5-1 (p. 4) lists thresholds. The electrical table is p. 6, 7.5.
Orderables are on p. 16-17, the packaging addendum dated 15-Jul-2026.

| variant | VIT- min / typ / max (mV) | Vhys typ (mV) | rising max (VIT- max + Vhys typ) | orderable names (p. 16-17) |
|---|---|---|---|---|
| Z | 2200 / 2250 / 2300 | 30 | 2330 | TLV803ZDBZR, TLV803ZDBZT |
| R | 2580 / 2640 / 2700 | 35 | 2735 | TLV803RDBZR, TLV803RDBZRG4, TLV803RDBZT |
| **S** | 2870 / 2930 / 2990 | 40 | 3030 | TLV803SDBZR, TLV803SDBZRG4, TLV803SDBZT |
| M (TLV803M, TLV853M, TLV863M) | 4280 / 4380 / 4480 | 60 | 4540 | TLV803MDBZR/T, TLV853MDBZR/T, TLV863MDBZR/T |

- The rows above are also given with ".A" suffixes in the addendum.
- The table is "TLV8x3M" for M, so one row covers the three families.
- Hysteresis is stated typical only (p. 6, 7.5, Vhys). It has no min or max.
- Release is VIT- + Vhys (p. 8, 8.1 Overview), so the repo's "rising max" is a screen, not a guarantee (ruled D6).
- Power-up reset voltage is 1.1 V (p. 6).
- Pull-up: 10 kOhm to 1 MOhm (p. 4). U30's R90 is exactly 10 kOhm, the inclusive lower end of the repo's check.

### 1c. Fit against U30 on VDD_3V3_RF

Conditions, as `supervisor.rs` states them:

- (i) Floor: VIT- min >= U30's VDD33 min, 3000 mV (radio-parts.json, ESP32-C3-WROOM-02 v1.7 p. 21, Table 6-2).
- (ii) Release: `check()` refuses if VIT- max + Vhys typ >= the rail's minimum at the pin
  (`rising_u >= rail_min` -> `SupervisorAboveRail`). Rail minimum at pin = solved minimum - drop limit =
  3144.727 - 144 = **3000.727 mV**.

Results:

- No variant has VIT- min >= 3000 mV except M (4280 mV), and M's rising max of 4540 mV is far above the
  rail's 3417.2 mV maximum, above U30's 3.6 V maximum, and above the 3000.727 mV minimum at the pin.
  The M reset would hold the board in reset forever.
- S is the nearest. It fails (i) by 130 mV (2870 vs 3000). It also fails (ii): its rising max is 3030 mV, above the
  3000.727 mV minimum at the pin. (The MCU rail passes the same part because its drop limit is 110 mV and the
  spec's drop-limit text says its solved minimum less 110 mV stays above 3030 mV.)
- **The window is empty.** Both conditions together require
  3000 <= VIT- min <= VIT- max < (VIT- max + Vhys) < rail minimum at pin.
  With drop limit 144 mV and the present set point, the upper bound is 3000.727, so the window is
  3000.0 to 3000.727 mV wide, less than any variant's own tolerance band (S's band is 120 mV wide).
- **Nearest variant, and what the rail minimum would need to be.** Raising the rail does not help S, because (i)
  fails on the part's own figure. Arithmetic for a hypothetical part of S's shape (band 120 mV wide, Vhys 40 mV) with
  VIT- min exactly 3000 mV: rising max = 3000 + 120 + 40 = 3160 mV, so the rail minimum at the pin would have
  to be at least 3161 mV. That is 3305 mV solved minimum at the 144 mV drop limit, or 3161 mV plus any smaller drop
  limit. **No such part is held. This is arithmetic, not a candidate.**

## 2. How the spec and IR model a supervisor

### 2a. The object (examples/sentinel-03.json:1135)

```
"reset_supervisor": { "reference": "U12", "part": "TLV803SDBZR",
  "pull_up": { "reference": "R61", "part": "CRCW0603_F_K", "ohms": 100000 },
  "bypass":  { "reference": "C62", "part": "GCM188R71H104KA57" } }
```

- Struct: `ResetSupervisor { reference, part, pull_up: Resistor, bypass: Placed }`
  (crates/aethelgard-ir/src/supervisor.rs:~31, `deny_unknown_fields`, object-only).
- Slot: `Mcu.reset_supervisor: Option<supervisor::ResetSupervisor>` (crates/aethelgard-ir/src/lib.rs:429; version
  entry at versions.rs:317 "mcu.reset_supervisor" under v30).
- Row kind `Kind::Supervisor` (power_parts.rs:525): `falling_millivolts`, `hysteresis_typ_millivolts`,
  `vdd_millivolts`, `pull_up_ohms`, `delay_overdrive_millivolts`.
- References are claimed in names.rs:142. The emitter draws it in schematic_power.rs:508 `supervisor(b, s, supply)`.
  The net is hard-wired to `NRST` and the supply is passed in as `spec.mcu.supply` (schematic.rs:1956).

### 2b. How a rail's monitor is found: there is only one

- `reset_floor(spec)` (supervisor.rs ~70): if `spec.mcu.reset_supervisor` is Some, the monitor's reset minimum is its
  VIT- min. Otherwise it is the MCU BOR level's falling minimum.
- `judged(spec)` (supervisor.rs ~275): calls `reset_floor` once. For every `Powered` part:
  - `elsewhere` (battery-fed regulator input) -> `Elsewhere`;
  - no held minimum -> `Named`;
  - **`p.rail != spec.mcu.supply` -> `Named`** ("held by no monitor": this is what names U30 and U31.B today);
  - `min > reset_min` -> `Floor`; else `Pass`.
- So "a rail is held" means precisely "it is `spec.mcu.supply`". Readiness refuses a package with any `Named` or
  `Floor` (cli/src/readiness.rs:169-177; ruled 3, 2026-11-27T07:42Z).
- `check()` / `check_against()` (supervisor.rs ~440) validates the one supervisor against the MCU's supply:
  - devices on the MCU's supply (I2C and SPI only);
  - rising max < the MCU supply's solved minimum less its drop limit;
  - VDD range covers the domain;
  - pull-up within the row's range.
- It is called once from lib.rs:3356 and cli/readiness.rs:50.
- radio.rs:975-1000 (`not_judged`) names a module on another rail as "held by no monitor".
- power.rs:1961 (the translator line-level check) judges the MCU's side at the release floor from `reset_floor`.

### 2c. What is missing for a second supervisor on VDD_3V3_RF

1. **No slot.** `reset_supervisor` is a single `Option` under `mcu`. A second one needs a list (for example
   `power.supervisors[]` or `radio_modules[].supervisor`) with a `rail` (domain id) field and an `output` target.
   `ResetSupervisor` has neither.
2. **`reset_floor`, `judged` and `check` need a monitor per rail.** All three take the MCU supply implicitly.
   `judged` names every non-MCU-rail part unconditionally (the `p.rail != spec.mcu.supply` arm). It needs to look up
   a monitor for `p.rail`.
3. **`powered()`** pushes the supervisor as a part on `spec.mcu.supply`. A second one is a part on VDD_3V3_RF.
   Its own VDD minimum (1.1 V) is judged, so it needs a monitor of its own. The row's 1.1 V power-up reset
   voltage (p. 6) would be the justification.
4. **No way to say what its output drives.**
   - U30's EN is modelled as `RadioModule.enable { resistor: PullResistor, capacitor: Capacitor }` (radio.rs:61).
     radio.rs:755 requires the resistor to go to the module's supply, and :910 checks the RC delay holds EN below
     VIH_nRST (0.75 x VDD) for tSTBL (50 us). There is no field for a driver on EN.
   - U31's OE (translator control) takes `strap` ("vcca" | "vccb"), `wire { net, role }`, or `after { regulator,
     pull_up, source }` (power.rs:329). `wire` is recorded but "neither the net's driver nor its level is judged"
     (power.rs ~4768). `after` follows a *regulator's* PG, not a supervisor. U11's EN already uses `after` U13
     with R32.
5. **Emitter.** `schematic_power::supervisor` draws RESET to `NRST` only.
6. **Schema version.** A new field is a versioned change (versions.rs `VersionChange`).
7. **R90 as pull-up.** R90 (10 kOhm, to VDD_3V3_RF) is already U30's EN pull-up, within the TLV803's 10 kOhm to
   1 MOhm. Wiring a TLV803 RESET to EN would leave C90 on the same node. `RadioModule.enable` would need an
   optional `driver` that replaces or joins the RC check.
8. **The `Named` text** for U30 would be replaced by a `Pass` line only if (a) above existed and a supervisor
   with VIT- min >= 3000 mV were held (it is not, see section 1).

## 3. The rail set point

### 3a. LMR33620, held SNVSAW1E (ti-lmr33620-snvsaw1e.txt)

- VFB (ADJ option): **0.985 / 1.000 / 1.015 V** (p. 6, 7.5 Voltage reference (FB pin)). min and max guaranteed over
  TJ -40 to 125 C, typ 1.000.
- IFB: 0.2 typ, **50 nA max**, FB = 1 V (p. 6, 7.5).
- Output regulation, VOUT = 3.3 V rows (VIN 3.8 to 36 V): **-1.5 % / +2.5 %**, a design limit, **not production
  tested** (p. 8, 7.7). The repo logs this as the `regulation_not_production_tested` screen.
- PG, % of FB voltage (p. 7, 7.5): VPG-LOW-DN (falling) **90 / 92 / 95 %**, VPG-LOW-UP (rising) 92 / 94 / 97 %,
  VPG-HIGH-UP 105 / 107 / 110 %, VPG-HIGH-DN 103 / 105 / 108 %, tPG glitch filter 60 to 170 us. The brief's
  "90 % of FB at minimum" is the minimum of the falling band.
- Section 8.3.1 (p. 11) states the PG flag is usable "to reset a system microprocessor".
- Recommended RFBT 100 kOhm, max 1 MOhm (p. 20, 9.2.2.3; repo row `rfbt_max_ohms`).

### 3b. Resistors: TNPW0603_B_E

Per the repo's `TNPW0603_B_E` row (VISHAY_28758):

| item | value | held at |
|---|---|---|
| tolerance | 1000 ppm (0.1 %) | p. 1 |
| TCR | 25 ppm/K | p. 3 |
| drift | 1000 ppm, 0.10 % after 8000 h | p. 2 |
| TCR reference | 20 C | row `tcr_reference_celsius` |

Board range is -20 to 85 C (sentinel-03.json `power.conditions`), so the excursion is max(85 - 20, 20 + 20) = 65 K.
Worst error per resistor = 1000 + 25 x 65 + 1000 = **3625 ppm** (`resistor_error_ppm`, power.rs:763).
The same 3625 ppm applies to each of RT and RB. The Vishay datasheet itself and its E-series value list are not
held in this repo; the repo row names no E-series, only the tolerance letter.

### 3c. The solver (crates/aethelgard-ir/src/power.rs, `outputs()` ~1310-1400)

With e = 3625 ppm on each resistor, RT = top, RB = bottom, in nV with ceiling division:

```
lo = floor( [ VFB_min x (RB(1+e) + RT(1-e)) / (RB(1+e))  - ceil(IFB x RT(1+e)) ] x (1000 + reg_min) / 1e6 )
hi = ceil ( [ ceil(VFB_max x (RB(1-e) + RT(1+e)) / (RB(1-e))) + ceil(IFB x RT(1+e)) ] x (1000 + reg_max) / 1e6 )
with reg_min = -15, reg_max = +25 per mille, IFB = 50 nA
```

Then `lo` must be >= the domain minimum and `hi` <= the domain maximum, or `RegulatorOutsideDomain` is raised.

For U13: R30 = 105 kOhm (top), R31 = 46.4 kOhm (bottom), both TNPW0603_B_E.
**Solved: 3144.727 mV minimum, 3417.204 mV maximum.** I reproduced both to the microvolt in a standalone replica of
the formula. It matches the spec's own `drop_limits` text ("U13's guaranteed 3144.727 mV (solved)").

- Minimum at U30's pin = 3144.727 - 144 = 3000.727 mV (drop limit, sentinel-03.json ~2600).
- Domain `VDD_3V3_RF`: nominal 3300 mV, tolerance 50 per mille -> **3135 to 3465 mV** (sentinel-03.json:1223).
  The present band (3144.727 to 3417.204) sits inside it with 9.7 mV of room below and 47.8 mV above.

## 4. Consumers of VDD_3V3_RF, maxima, and the window

Consumers (spec `power.loads`, `radio_modules`, `uart_crossings`): U30 (3V3 pin), U31.B (VCCB), R90 (U30 EN
pull-up, 10 kOhm), R91 (U30 IO2 strap pull-up, 10 kOhm), R32 (U13 PG pull-up, 100 kOhm, used by U11's `after`),
and R30 (the divider's top end). U31's OE is strapped to VCCA (VDD_3V3), not to this rail.

| consumer | maximum supply | held at |
|---|---|---|
| U30 ESP32-C3-WROOM-02-N4 VDD33 | **3.6 V absolute maximum and 3.6 V recommended maximum** (min 3.0, typ 3.3) | ESP32-C3-WROOM-02 v1.7 p. 21, Tables 6-1 and 6-2. Read from ~/.cache/aet-4310/held-text/esp32c3-wroom02-v1.7.txt (the .pdf in aet-held has no .txt beside it). The repo row's sha256 622db737... matches the held .pdf. |
| U31 TXU0304PW VCCB | operating 1.08 to 5.5 V (p. 6, 7.3), absolute max 6.5 V (p. 5, 7.1). The row the repo selects for the UART crossing is the VCCB = 3.3 V +/- 0.3 V row, **3.0 to 3.6 V** (p. 14, 7.11) | SCES935A. Read from ~/.cache/aet-1216/ds/txu0304.txt (revision SCES935A, May 2021, revised Nov 2021). That file is from another copy of the PDF: sha256 e9d380b0..., not the held 425338d9.... Same revision, but the held file itself has no .txt and **pdftotext is not installed**, so I did not extract it. |
| R90, R91, R30/R31, R32 (resistors) | no supply maximum held | n/a |
| C90 GRM188R71C105KA12 (EN capacitor), C91-C94 | rated voltages, not examined | n/a |

- **Highest rail maximum the parts allow: 3600 mV**, set by U30 (absolute = recommended = 3.6 V, no margin) and the
  U31.B row (3.6 V).
- **Highest the spec allows today: 3465 mV**, the domain's declared maximum. Raising it to 3600 would mean
  redeclaring `VDD_3V3_RF`'s tolerance. That is a spec edit and would need a ruling.
- **Lowest minimum the chosen supervisor needs:** with no variant fit, there is none to state. For a hypothetical
  S-shaped part with VIT- min = 3000 mV, section 1c gives 3161 mV at the pin.
- **Does a window exist?** With the TLV803 family: **no**, at any E96 pair, because the family has no VIT- min
  between 2870 and 4280 mV (condition (i) fails for S, and M cannot release below 4540 mV).

### E96 feedback pairs (TNPW0603 B 0.1 %), solved by the repo's formula

TNPW0603 at 0.1 % is stocked in E96 series or finer; E192 is also named in the brief, but the value list is not held
here, so these are standard E96 mantissas only (my sweep: RT and RB each 10 kOhm to 1 MOhm, ratio 1.9 to 3.0).
The band is 270 to 290 mV wide for every pair near 3.3 V, and the narrowest I found anywhere in the sweep is 233.5 mV.

Present: R30/R31 = 105k / 46.4k -> **3144.727 / 3417.204** (width 272.477).

Highest minimum with the rail maximum <= 3465 mV (domain unchanged):

| RT / RB | solved min | solved max | width |
|---|---|---|---|
| 45.3k / 19.6k | 3194.194 | 3464.742 | 270.548 |
| 26.1k / 11.3k | 3193.708 | 3462.193 | 268.485 |
| 40.2k / 17.4k | 3193.599 | 3463.558 | 269.959 |
| 102k / 44.2k (RT near the datasheet's 100 kOhm) | 3187.990 | 3463.957 | 275.967 |
| 107k / 46.4k | 3186.146 | 3462.477 | 276.331 |

Highest minimum with the rail maximum <= 3600 mV (needs the domain redeclared):

| RT / RB | solved min | solved max | width |
|---|---|---|---|
| 24.9k / 10.2k | 3320.375 | 3599.875 | 279.500 |
| 26.1k / 10.7k | 3318.462 | 3597.921 | 279.459 |
| 113k / 46.4k | 3310.403 | 3598.296 | 287.893 |
| 105k / 43.2k | 3306.185 | 3592.865 | 286.680 |
| 100k / 41.2k | 3303.185 | 3589.075 | 285.890 |

For a hypothetical S-shaped supervisor (3161 mV at the pin):

- If the drop limit stays 144 mV, the solved min must be >= 3305 mV. 26.7k / 11k gives 3306.893 / 3585.398, which
  needs the domain redeclared (max above 3465).
- If the drop limit is cut so the domain stays at 3465 mV, the solved min must be >= 3161 + drop limit. 31.6k / 13.7k
  gives 3190.388 / 3459.159, so the drop limit would have to be under 29.4 mV, which is likely not achievable
  in copper.

Side note on U13's own PG as the reset source (not modelled by the IR at all today):

- PG falls at 90 to 95 % of FB (p. 7, 7.5). A guaranteed >= 3000 mV falling threshold at 90 % needs a rail minimum
  of >= 3333.34 mV. By my sweep the smallest solved maximum with a minimum that high is **3638.494 mV**
  (28k / 11.3k: 3355.571 / 3638.494), and 3640.049 for 102k / 41.2k.
- Both are above the 3600 mV limit, so PG alone cannot guarantee 3.0 V either, at 0.1 % resistors.

## 5. Not held anywhere (plainly)

- Any supervisor or reset IC with a VIT- minimum >= 3000 mV and a guaranteed release under ~3.1 V. Only the TLV803
  datasheet (SBVS157E) is held. TI's TPS3808/TPS3839-class adjustable or 3.0 V parts, and similar, would each need
  their own datasheet's electrical table with VIT- min/typ/max, hysteresis min/typ/max and the orderable names.
- TLV803 hysteresis has **no min or max**, only typ. A guaranteed release is therefore never held.
- TNPW0603 (Vishay document 28758) orderable E96/E192 value list: the repo row holds the tolerance and TCR but not
  the value series.
- TXU0304 held PDF has no `.txt`. The VCCB numbers above are from a same-revision extraction of another PDF copy.
  If a .txt of the held file (sha 425338d9...) is wanted, it needs `pdftotext`, which is not installed here.
- U30's EN pin: its absolute maximum voltage when driven by an open-drain RESET, and any supply sequencing
  requirement beyond tSTBL (50 us, p. 15 Table 4-6), were not re-read here beyond what the repo row quotes.
- The datasheet's page headers vs the repo's citations: I used the repo's page numbers and the held text's own
  "=== page N" markers where they agreed (TLV803 p. 4, 5, 6, 16-17; LMR33620 p. 6, 7, 8, 11).
