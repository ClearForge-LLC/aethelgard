# AET-WO-1216 — targeted adversarial pass (§5)

Reviewer: Opus, read-only on `/home/forge/aethelgard-1216` (branch `wo/aet-1216-power-sequence`, head `d9cb8a7f`).
Probes and outputs: `~/.cache/aet-1216/adv/` (`supply_pads.py/.out`, `dropped.py/.out`, `judge_probe.py/.out`, `arith.out`, `options.rerun.tsv`, `spec-rerun/`).
Repo hygiene: one probe imported `drawn.py` and left `spike/1216/__pycache__/drawn.cpython-314.pyc`. I removed it at once, and `git status` is clean again.

**Reproduction.** `judge.py` re-run from the committed `out/spec` gives `options.tsv` byte-identical. `spec.py` re-run over all 34 files from `w/solve` gives `out/spec/` identical. The §4 tallies (c1 1, c2 33, b1 10, b2 7, a1 10, a2 8, sidecar 1/4, 1/1, 0/1) match `options.tsv` row by row. **No count changes as run; finding 3 moves c1 from 1 to 0 if its reading is accepted.**

## Findings

### 1. defect — The board counts in RESULTS §1 are wrong: 25 boards draw, not 27
- **Evidence:**
  - `out/boards.tsv` has 25 rows with emit = 0.
  - `ls out/drawn | wc -l` gives 25, and `w/emit/` has 25 directories.
  - `compare.tsv` covers 25 boards.
  - The 9 that do not draw: 4 `DrawsNoBoard`, 2 refused by validate, 3 refused by solve. That is 9, not "the 7".
- **Wrong text:** RESULTS.md:22 ("27 of 34 boards emit"), :26 ("The 7 that do not emit") and :28 ("388 of 388 rows on the 27 drawn boards").
- **What it changes:** the 388/388 figure stands; the board counts are 25 and 9.

### 2. defect — D1's IPROPI figure uses a temperature range nobody declared: 4665 mV, not 4683
- **The cause:** `spec.py:123-124` falls back to −40..125 °C when `power` is null. D1 declares `pin_devices[0].temperature_celsius` −20..85 °C, and that is the range the engine judges R2 over (`pin_devices.rs:1864-1873`).
  - The engine never invents a range: it refuses `TemperatureUndeclared` when none is declared.
- **The recomputation** (TCR reference 20 °C, from the CRCW0603_F_K row; `arith.out`):
  - Excursion 65 K → 36 500 ppm → R2 max 1554.75 Ω → **4665 mV**.
  - The spike's 105 K gives 40 500 ppm → 1560.75 Ω → 4683 mV.
- **What it changes:** RESULTS §1, §2, §4 and the bus report should read 4665 mV through 1555 Ω. The c1 verdict is unchanged (it is still above 4000).

### 3. defect — 4665/4683 mV is not D1's worst; the stated data do not bound IPROPI below 5.75 V
- **The 3 mA is a recommended condition, not an output limit.** IIPROPI 0–3 mA is a recommended operating condition (SLVSDS7B p. 4 §6.3), which RESULTS §2 concedes.
- **The current can exceed it, by a rendered row:** the same table (p. 4 §6.3, `render/drv8876-p4.png`) allows a peak output current IOUT of 0–3.5 A.
  - At the typical 1000 µA/A that is 3.5 mA, which is 5442 mV into 1555 Ω, already over 3 mA. AERR is stated only up to 2 A.
  - IOCP is "3.5 5.5 A" in the flat text of p. 6 (no render; column unverified), so up to about 5.5 mA (8551 mV) before tOCP.
- **The clamp has no figure** (p. 12). The only stated ceilings are the abs-max rows: 5.75 V and VVM + 0.3 V.
- **What it changes:** the honest worst is "not stated, the abs max being 5.75 V", or 5.75 V if an abs-max row is taken as a bound.
  - If "not stated": **c1 refuses 0/34**, not 1/34, and D1 moves to c2's unbounded class.
  - The c1 = 1 result rests entirely on treating a recommended condition as a bound. RESULTS should say so beside the §4 table, not only in §2.

### 4. defect — A held page states a voltage on the very connector pins RESULTS calls "not stated"
- **DS-009 states +3.3 V.** Its "Volt" column gives +3.3V for every signal pin, and +5V only for VCC (`ds009.txt`):
  - Telemetry port TX/RX/CTS/RTS (p. 6, lines 519–560);
  - the GPS ports' TX/RX/SCL/SDA (pp. 7–8, lines 584–702);
  - the I2C port's SCL/SDA, "1.5K pullup on autopilot" (p. 9, lines 773–791).
- **Those are nearly all the rows RESULTS singles out.** Every connector row with `powered_by` VDD_5V is on a `standard: DS-009 …` connector, except 3. Those 3 are `connectors`' J2 EXT SPI pins, which have no standard and stay "not stated". The DS-009 ones are J6/J7/J8 on `sentinel-03` and the TELEM/GPS/I2C ports on `connectors`, `hse-crystal-16`, `hse-external-8`, `sentinel-02-shaped` and `serial-four`. These are the rows RESULTS §1 flags as "a peer those connectors power can drive the MCU pins while 3V3 is down".
- **`spec.py:220` writes "not stated" for every connector.** It prints `standard` as a label (:218) but never takes a voltage from it.
- **Caveat:** +3.3 V is nominal. It has no tolerance and no power-off clause, so it is a stated level, not a stated worst.
- **What it changes:**
  - The §1 class row should cite DS-009 (+3.3 V nominal, under the 4.0 V limit).
  - c2's count does not move. I checked each of the 6: every one keeps unbounded rows not on a DS-009 connector (`connectors`: EXT SPI on J2; `serial-four`: SBUS on J4 and SWD; the rest: SWD, console or motor).
  - b2 does not move either: it refuses on "reaches", whatever the voltage.

### 5. defect — "Every path is FT except PE9" is false (RESULTS §1)
- **The non-FT path rows** (probe over `out/spec`, `can_lead` = yes):
  - test points on **PA5 (TT_ha)** and **PA7 (TT_a)** on `sentinel-01`, `sentinel-01-bench` and `sentinel-01-ext-bmp581`;
  - NRST (RST) and BOOT0 (B) connector and test-point rows on 4 boards;
  - 3 `unheld` rows on D2 (unsolved).
- **PA5 is a special pin.** DS12117 Table 11 (p. 104) gives PA4/PA5 injected current −0/0. Table 48 (p. 136) lists PA5 at 0/0, and PA7 at positive 0.
- The c1 limit is still 4.0 V for all of these (RST and TT are "any other"/TT_xx), so no count moves.
- **PE9 TT_ha is correct:** the held text wraps the cell as `TT_h` / `a`, which is not a mismatch.

### 6. defect — `esc` rows are read backwards (`spec.py:233-237`)
- **`{"none": …}` means no ESC.** It is `Esc::NoEsc` (`motor.rs:182`). `sentinel-03`'s value says "J5 is a bench header … ESCs, *when added*, are fed from the pack". `spec.py` emits an "esc" path for any truthy `esc`, so `sentinel-03` gets 4 rows.
- **The boards most likely to drive an ESC get none.** 9 boards have `esc: null`. 6 of them carry MOTOR nets on a connector: `motors-dshot600`, `motors-pwm`, `serial-dshot-9600`, `hse-crystal-16`, `hse-external-8` and `sentinel-02-shaped`. On D1, D2 and `driver-judges-nothing` the "motor outputs" are driver inputs.
- **What it changes:** RESULTS §1's row should read "declared none on `sentinel-03`; unstated on 6 boards".
  - Every motor net is already a connector row, so no path or count changes.

### 7. gap — The pin-device classes were never reconciled, and the second walk would disagree on D1
- **Nothing reconciled them.** Only D1 and D2 have `pin_devices`, and neither draws (DrawsNoBoard, validate-refused). So IPROPI, the nFAULT pull and every `pin_device_*` class are walk-1 only.
- **`spec.py:198` lists only output-kind pins.** It skips `logic_input`, `enable_input`, `open_drain_output` and unknown kinds (None).
- **The second walk would flag them.** `drawn.py:72-78` marks any non-two-terminal U whose pads touch a non-supply rail. U2 touches VMOTOR, so on a drawn D1 the rows on DRV_NSLEEP, DRV_EN, DRV_PH and DRV_NFAULT would be `ic yes:VMOTOR`. `compare.py:117-118` would find no spec row for any of them: **4 "NO" rows**. This comes from reading the code, not from a run; DrawsNoBoard hides the disagreement.
- **Two latent faults on a multi-supply pin device:**
  - `spec.py:200,210` writes `rail` as `"A/B"`.
  - `judge.reaches` (judge.py:235) and the sidecar's `rails & led` (:279) then never match a domain, so b2 and a-sidecar skip that row.

### 8. gap — RULES.md weaknesses that `judge.py` implements faithfully (`judge_probe.out`)
- **(b) An upstream PG counts as "after".** `pg_nets_on_supply_chain` (judge.py:210-216) takes PGs from every regulator in mcu.supply's input chain, including its *ancestors*.
  - Synthetic tree: battery → U1 5V → U2 3V3 LDO, and VDD_X with its EN on U1's PG. `domains_status` returns `VDD_X: after`, although X follows 5V, not 3V3.
  - The same probe labels VDD_5V, the 3V3's *parent*, `RailsRace`. `spec.py:112` likewise calls it "a sibling". An ancestor leads by construction and is not racing.
  - `spec.py:can_lead` ignores EN/PG wiring altogether, so on a wired board walk 1 and judge (b) would disagree.
  - Latent: no committed board has a control wire.
- **(a-sidecar) It accepts sequences that should fail:**
  - one naming only VDD_3V3 on `sentinel-03`: the battery and 5V are omitted and treated as "after";
  - one putting the battery **after** 3V3 on `sentinel-02-power`, which contradicts the tree: the divider row is never led;
  - any non-empty `source` string ("the designer says so").
  
  The one refusal it makes (`5v-after-3v3-unsourced`) is for an empty string only. Option (a) as run checks neither completeness nor consistency with the tree.
- **The battery's name is inconsistent.** `spec.py:214` puts a connector's battery net into `powered_by` as `VBAT`, but rows and sequences call the battery `battery`. A connector carrying VBAT would never meet `led` (judge.py:279) or `reaches` (:235). Latent: no committed connector carries VBAT beside an MCU net.

### 9. note — Figures in RESULTS §2–§3 against the held pages
- **DRV8876 nSLEEP VIL max 0.7 V is the VVM < 5 V row.** D1's VMOTOR is 12 V, so the row is **0.8 V** (p. 5). The undefined window is 0.8–1.5 V. Separately, below VPOR (rising 1.62 V min, DS12117 p. 107; §3.5.2, "the devices remain in Reset mode") the MCU is in reset. The I/O reset state is in RM0433, which is not held, so no held page says the MCU drives nSLEEP below 1 V at all.
- **LMR33630:**
  - "No VIN figure is tabulated" stands beside p. 7 §7.5, which gives VIN minimum operating input voltage **3.8 V max**. That is a stated ceiling on when each buck is running; the skew stays unbounded below it.
  - "PG thresholds 92–110 %" is incomplete: those are the rising rows, and VPG-LOW-DN falls to **90 % min** (p. 8).
  - The `sentinel-03` sequence source cites UVLO as "p. 15 8.3.5". It is **§8.3.4**.
  - Checked and correct against the renders: VEN-VCC-H 1 V max, VEN-VCC-L 0.3 V min, VEN-H 1.2/1.231/1.26 V, tSS 2.9/4/6 ms, tPG 60 min / 170 max, VIN-PG 2 V max, PG abs max 22 V / recommended 18 V.
- **TLV755P:** the 120 Ω pull-down "below UVLO" (p. 12) is qualified on p. 14: "the active pulldown is on when sufficient input voltage is provided". The other figures match the p. 6 render.
- **DRV8833:** RPD 500 kΩ is on p. 6. Page 3 says only "internal pulldown".
- **DS12117:**
  - Table 10 and notes 3–5, Table 11 note 3 and §3.5.1 p. 27 all match.
  - p. 105 Table 13 also states operating VIN: TT_xx up to VDD + 0.3, and the others Min(VDD, VDDA, VDD33USB) + 3.6 V. These are operating conditions, not abs max, so RESULTS' "nothing beyond Table 10" holds at VDD < 1 V. That rests on my reading that Table 10 note 1 ("in the permitted range") admits VDD = 0 V (abs max −0.3 V); no page says so.
- **"All 132 bus pulls" is not this universe's figure.** The 34 files declare 49 I2C buses, which is **98** pulls. RESULTS credits 132 to 1211e.

### 10. note — Arithmetic in `spec.py`
- **The divider corner is taken the right way:** top at −3.65 %, bottom at +3.65 %, 26 100 × 10 365 / 106 715 = 2535.04 → 2536.
- **The third read agrees:** the solve JSON's `VREF+ (VBAT_SENSE full scale)` floor is 2536 on both boards.
- **`dom_max` matches the engine.** It is the same as `maximum_millivolts` (lib.rs:838, `div_ceil`), and the error composition is the same as `power.rs:544`.
- **`series_ohms` for the divider is nominal (9091 Ω).** The low corner, which gives the most injected current for F5, is **8759 Ω**.
- **The battery worst is steady-state** (cells.max × cell_max). Plug-in, when VDD < 1 V by definition, rings the input. I found no ring-peak figure in the held pages, in `power.rs` (`grep hot_plug|damp`: only the `BatteryInputUndamped` presence rule) or in the solve output (C1 "damps the input's ring through its ESR"; no figure). R50 with C50 (9.1 kΩ, 100 nF) would filter a µs ring, but that is unstated.
- **Dead or over-broad code:**
  - `spec.py:148-149` is a no-op loop.
  - `spec.py:150` collects every MISO on the board, not only the device's bus.
  - `spec.py:171-172` finds the crossing's MISO only by the net-name prefix `IMU_`, because `spi_lines` lists no MISO. A bus whose MISO is named differently would be missed; walk 2 caught the CS case only because the board draws.

## Targets that turned up nothing (what I checked)
- **MCU supply pads that `drawn.py` skips** (regex at :45): on all 25 drawn boards VDD, VDDA, VREF+, VDD33USB, VBAT and PDR_ON are on `VDD_3V3`, VSS/VSSA on GND, and VCAP on local `/U1_VCAP*` (`supply_pads.out`).
  - The MCU's VBAT pin is never on the board battery net `VBAT`.
  - All 144 pads are mapped in `table8-visual.tsv`. The regex skips every S-type pad and also PDR_ON (type I, FT, not a supply), which is on VDD_3V3 on all 25 boards. BOOT0 and PC2_C/PC3_C are correctly walked.
- **`rail()`:** the nets without "/" are exactly GND, VDD_3V3, VDD_5V, VDD_1V8 and VBAT. Every regulator-internal net (`/U10_SW`, `/U10_FB`, `/U10_BOOT`) is slash-named. No misclassification.
- **Support parts on NRST, BOOT0, SWD and OSC:** R to GND, C to GND, SW to GND, crystal plus load caps to GND. No `hop` row exists on any board.
  - There is no D part. The only hot-plug part is C1 (no TVS).
- **Spec fields, read data-driven** (`fieldwalk.py/.out`): I walked every JSON path in the 34 files whose string value is a domain, VBAT or an MCU net.
  - The paths `spec.py` does not read are `board.layers[].net`, `net_classes`, `net_constraints` and `diff_pairs` (routing only), `power.loads[].domain` (not a net), `pin_devices[].straps.*.tie` (a domain) and `pin_devices[].bridges[].inputs` (driver logic inputs, finding 7).
  - None carries another domain onto an MCU net on a committed board.
  - I did not open `connectors.rs`, `names.rs`, `clock.rs` or `bench.rs` themselves. Of the structs, I read `lib.rs`, `power.rs`, `pin_devices.rs`, `motor.rs` and `serial.rs`. Also:
  - `spi_buses[].domain` all equal `mcu.supply`;
  - `core_supply` is `ldo` everywhere;
  - strap ties go only to domains or GND (`pin_devices.rs:946-970`);
  - there are no control wires.
- **The non-emitting boards:** the 4 DrawsNoBoard have solve output, so `pin_of` is complete.
  - The unsolved fallback misses `spi_buses` sck/mosi/miso (`dropped.out`: pigeonhole, thrash, invalid-zero-domain).
  - No connector or test point sits on those nets, so nothing is lost today; the only nets it drops are motor-bridge outputs, which are not MCU nets.
- **c1 limits:** B 9.0 V and every other structure 4.0 V at VDD = 0 matches the p. 103 render.
- **The crossing's B side:** the TXU0304 B pins touch only U5 (ICM20948) on VDD_1V8, downstream of 3V3. The drawn rows that touch an MCU pin are the A-side nets, flagged `yes:VDD_1V8` by the IC's rails and matched.
- **An open-drain output pulled elsewhere:** D1's nFAULT goes through R3 to VDD_3V3 (`mcu.supply`), and D2's uses the MCU's pull. Every I2C device is on `mcu.supply`.
- **USB:** VDD33USB is on 3V3. PA11/PA12 (FT_u) carry MOTOR or IMU nets, and no connector carries VBUS or USB.
- **Judge mechanics checked:**
  - b's `reaches` does count a connector a domain powers, through `powered_by` (the 5V boards).
  - a2 behaves as written: `sentinel-02-power` is refused on its battery row alone, and D2 is not refused.
  - The battery is judged the same way in b (by c1's test), in a2 (it triggers) and in the sidecar (when its step is at or before the supply's). The exceptions are finding 8's VBAT naming and the battery-last sequence.
