# AET-WO-4300k — STOP after §1.2

Stopped 2026-11-24 with the measurement (`RESULTS.md`, M0–M12) and the decisions (a)–(e) the WO asks, plus nine the measurement raised (D6–D14). Nothing is built: no file under `crates/`, `examples/`, `fab/`, `docs/` or `scenarios/` has changed.

**§7 conditions, checked:**
- no held PDF's hash differs (M0);
- the crystal passes AN2867 Rev 25 (M3);
- the XT30 drawing shows the opening, away from the tails (M2);
- no board has moved, since nothing is built.

**Three "held page does not support a bound you need" items fired:** D6 (the supervisor's rising maximum), D7 (the factor 2), and (b)'s dielectric minima under JLCPCB. Each names the document a ruling would need; D6 and D7 also have a lean that needs none.

## What the measurement changes in the WO

1. **Sentinel-03 cannot get a package today for a reason the WO does not name:** 280 unwitnessed ledger rows, about 37 register/class entries, have no `acknowledge_unwitnessed` entry (M1). Each needs a written `reason`. That is in (8)'s "every clause met", but it is volume the WO does not mention (D10).
2. **A6 needs two input ceramics per buck at 4S, not three** (M6). The engine's 5.433 µF per cap is already worst case: Murata's curve × the −10 % tolerance × X7S's −22 % over temperature. So two give 10.87 µF (+8.7 %), three 16.30 µF. The WO's three was computed at 6S.
3. **No motor current crosses the board** (M5). The whole `VBAT` net is the flight controller's branch, so the "fuse on a motor path" refusal has no committed board to bite on. It is held by a planted test only.
4. **No fabricator publishes every bound** (M8), so (3)'s "one fabricator that every bound cites" cannot be met by a fabricator's pages alone.
5. **A bump moves `FAB-NOTES.md`** as well as MANIFEST's hash lines (M10), outside the fence's letter. It moved the same way at 4300j's bump.
6. **W25Q64JV's floor depends on the flash's clock** (M4): 2.7 V at 104 MHz, 3.0 V at 133 MHz. The TLV803S (2.87 V) passes the first and fails the second.

## (a) The thermal copper: measured, or pours drawn

**Measured today** (M7): each pad joins 0.15–0.24 cm² on F.Cu and only via rings elsewhere. The two planes are unfilled outlines (In1 GND, about 64 cm² once filled; In2 VDD_3V3, not joined). A measured screen would refuse `BuckJunctionOverLimit` (< 7 cm² floor) on F.Cu and B.Cu.
- **(i) Measure only.** The screen reads per-layer joined copper from the drawn board, taking the minimum over the layers the pad joins. It refuses until (ii) is built, so it does not stand alone.
- **(ii) Lean: pours drawn, then measured.**
  - `emit` draws a GND pour on F.Cu and B.Cu around each buck's thermal pad, stitched by its thermal vias, sized to keep the junction in budget at the figure's point.
  - The screen then reads the joined copper per layer, from the pour polygons as drawn (net, layer, outline less clearances, a bound from below), never from the declared number.
  - The declared `junction.copper_square_centimetres` goes. If it stays, it becomes a floor the measurement must meet.
  - **What it needs** (the row's Figure 9-3 points, the 85 °C board, the row's limit):
    - U10 (1,170 mW) needs at least **15 cm² per layer** (θJA 30.0 gives 120.1 °C; 10 cm² gives 36.0 and 127.1 °C, refused).
    - U11 (550 mW) needs the 7 cm² floor (42.0 gives 108.1 °C).
    - The board is 67.74 cm². F.Cu carries every part, so 22 cm² of GND pour on F.Cu and on B.Cu is unproven to fit.
  - **Both pads are GND, so one pour joins both.** Figure 9-3 is per device. Lean: each buck is credited only the pour area nearer its own pad (a Voronoi split of the shared pour), never the same copper twice.
  - If it does not fit: the outline may grow to `max_outline_um` (100 × 100 mm), or U10's declared 2 A (DS-009's port budget) is the lever. That is a ruling, not mine.
  - In2 (VDD_3V3) joins nothing and is named as a condition. Figure 9-3 is "copper area per layer" on a four-layer board, so the minimum is taken over the joined layers, not all four.
- **The open question under (ii):** the emitted file holds unfilled zones (KiCad fills at DRC). Today's `extract` reads a board without fills. So the measure is either (ii-a) a lower bound computed in Rust from the pour outline less every other-net item's clearance, or (ii-b) read from KiCad's filled board in the `drc` gate. Lean (ii-a): no tool, and it bounds from below as the screen needs.

## (b) The fabricator, and how its single source is declared

- **Where will Scotty order Sentinel-03?** That decides it.
  - Sentinel-01 was ordered at OSH Park (`fab/sentinel-01-bench/ORDER.md`; its upload zip is held). Its package also carries JLCPCB-format BOM and CPL files.
  - No Sentinel-03 package exists yet.
- **The matrix (M8):**
  - **JLCPCB** publishes thickness (±10 %), every `fab_tolerances` field and track width. It lacks the three dielectric minima (nominals only).
  - **OSH Park** publishes the dielectric tolerances, hole tolerances and ring. It lacks a thickness tolerance and a track-width tolerance, which the supply gate needs.
  - **Either way one document is missing,** a "held page" need: JLCPCB's dielectric tolerance (an impedance-stackup PDF or a support answer), or OSH Park's thickness and track-width tolerances.
  - A third route: rule that a missing bound may come from a named standard (IPC-4101 or IPC-6012) beside the fabricator's nominal.
- **Declared as** a `board.stackup.fabricator` object: `{name, documents: [{id, title, url, read}]}`.
  - Every bound's `source` becomes structured: `{fabricator: <document id>, at: "…"}`, or `{standard: "IPC-4101" | "IPC-6012" …}`, or `{condition: "…"}`, an enumerated set.
  - A bound that cites a different fabricator is refused by name.
  - A source string that merely contains the maker's name counts for nothing (§5).
- **The eight bounds no fabricator publishes** (copper max, permittivity max, the three copper/plating minima, copper temperature) come from `{standard}` or `{condition}`, as `copper_max_um` and permittivity do today.

## (c) Sentinel-01's `--against-fab`, byte-identical without a back-fill

- **Measured:**
  - Sentinel-01 declares no `power`, so (2)'s `drop_limits` rule never touches it, and no `fab_tolerances`.
  - `--against-fab` derives the package exactly as `--fab-out` does, the MANIFEST's precondition list included, and compares bytes (`verify.rs:2325-2396`). So **a fifth MANIFEST row moves Sentinel-01's MANIFEST** beyond its hash lines.
  - Choosing rows from the committed MANIFEST would be circular: a dropped row could never be caught.
  - The ledger's acknowledgement precondition is not a MANIFEST row. It is a pre-gate refusal under either fab flag (`UnwitnessedFieldNotAcknowledged`, `verify.rs:34-39`).
- **(i) Lean: a pre-gate refusal under `--fab-out` only, with no MANIFEST row.**
  - `--fab-out` refuses `FabricationNotReady` by name, before any gate, as the ledger does.
  - `--against-fab` does not apply it. It re-derives a package whose MANIFEST binds the specification's sha256, so the specification that passed readiness when the package was written is the only one that can match. Any edit, a deleted key included, moves `specification_sha256` and the comparison differs.
  - Sentinel-01's re-derivation is unchanged but for its hash lines and FAB-NOTES (M10).
  - A Sentinel-01 re-spin is a `--fab-out`, so it meets the gate, as `ORDERED.sha256`'s header asks a ruling for a re-spin.
  - The verdict under `--against-fab` names it ("fabrication_ready: not judged under --against-fab; the package binds the specification").
  - **The gap:** a later engine that adds a readiness key does not re-judge an existing package until it is re-written. That is acceptable for an ordered board and named in FEEDBACK.
- (ii) A fifth MANIFEST row: Sentinel-01's MANIFEST gains a row, more than its hash lines.
- (iii) A specification key exempting Sentinel-01: a back-fill in spirit, rejected.

## (d) The gate's key list and condition ids

- **The key list (M11, `sweep/SWEEP.md`):** five keys relax a judgement while `validate` passes.
  - `power.drop_limits` (absent or `[]`). The readiness gate refuses it absent or empty wherever `power` is declared, and a domain with an on-board load pin left unlimited (D13).
  - `board.stackup.fab_tolerances` (absent), refused for any new package ((2), A15). Sentinel-01 is untouched by (c).
  - `power.conditions.dc_bias_derating` (`[]`), refused when any ceramic role is judged under DC bias. Today the line silently falls back to "judged at rated capacitance".
  - `power.loads` (`[]`, or a regulated domain with none). A13's rule: `validate` refuses a regulated domain with no load, or the readiness gate does. Measured: zero committed boards have one (M12), so `validate` is safe.
  - `diff_pairs` (`[]`). **Lean: not on the list.** A board with no declared pair draws none. pair-standin's case (a declared pair removed) is a different specification, not a silent off: `extract`'s ran table already says `off: diff_pairs`. Named as a condition when a reserved USB pin pair exists (Sentinel-03's PA11/PA12), so the reader sees it.
  - **Shape:** the gate reads the specification's keys (cheap), never re-runs a gate per variant. A gated `extract` is 5 min 22 s in release.
  - It also refuses any `ran` row that is `off` by a key on the list, so a key added later that `1609`'s table reports is caught by the same row.
- **Condition ids:**
  - Each "screened, not guaranteed", "typical", "not computed", "declared, no source" and "not production tested" line `solve` prints becomes a condition with a stable id, `<kind>:<reference>`, e.g.:
    - `buck_junction_screened:U10`;
    - `cin_dc_bias_typical:U10`;
    - `isat_typical:L1`;
    - `regulation_not_production_tested:U10`;
    - `reservoir_cadc_typical:VBAT_SENSE`;
    - `load_declared_unsourced:J3`;
    - `lifecycle_unread:CRCW0603_F_K`;
    - `hse_safety_factor_unmeasured:Y1`, `hse_drive_level_unmeasured:Y1` (M3);
    - `supervisor_hysteresis_typical:U?` (D6).
  - The specification acknowledges each by id in a new list, `acknowledge_conditions: [{id, reason}]`, shaped like `acknowledge_unwitnessed`.
  - An id no condition produces is refused as stale. That closes §5's "a condition acknowledged that never occurs", and an id names its reference, so "one named under another reference" fails.
  - **Sentinel-03's ids today, from its `solve` conditions (M5's run) and M3/M4:**
    - `cin_dc_bias_typical`:U10, U11;
    - `cout_dc_bias_declared`:U10, U11 (a placeholder derating);
    - `cvcc_cboot_cin_hf_rated`:U10, U11;
    - `isat_typical`:L1, L2;
    - `dcr_at_25c`:U10, U11;
    - `regulation_not_production_tested`:LMR33630ADDA;
    - `timing_at_vin_12v`:LMR33630ADDA;
    - `ripple_not_met`:U10, U11 (ruled D6 of 4300d);
    - `buck_junction_screened`:U10, U11;
    - `hot_plug_ring_uncomputed`:U10, U11 (replaced by the doubled-pack judgement once (6) is built);
    - `bulk_esr_cold`:C1;
    - `dielectric_accepted`:C10, C20 (X7S);
    - `lifecycle_unread`:each of 9 power parts;
    - `vref_is_vdda`:VBAT_SENSE;
    - `reservoir_cadc_typical`:VBAT_SENSE;
    - `bor_rising_at_domain_min`:VDD_3V3;
    - `mcu_load_table_120`:U1;
    - `load_declared_unsourced`:U5, J2, J3;
    - `xt30_rating_disagrees`:J4;
    - `derating_declared`:power;
    - from `extract`: `pin_c_declared`:U5, U7;
    - from M3/M4: `hse_safety_factor_unmeasured`:Y1, `hse_drive_level_unmeasured`:Y1, `supervisor_hysteresis_typical`:the TLV803S.
  - About 45 ids. The build fixes the list from the code's own condition sites (one id per site, its reference from the line), not from this transcript.
- **Not swept (a stated gap):** `check support`, `check erc` and `check drc` (`drc.power`, 16 judged; `support`, 23 judged) need KiCad, which the host lacks. Three of the five relaxations were silent, so `verify`'s own `off` rows (M1a) do not cover what these gates might drop. The build sweeps them in the image before the gate's key list is fixed: delete-variants of Sentinel-03's accepted keys through `verify`'s board gates.
  - The false line (A13) is fixed, not acknowledged.

## (e) (10)'s current path, and the parts to hold

- **Measured (M5):** `VBAT` reaches J4.2, C1, the four input ceramics, R50 and the two bucks only. J5 has no VBAT.
- **The branch current, two figures:**
  - The engine's `input_steady_milliamps` 2,747 is the sum of the bucks' *output* loads (2,000 + 747, `power.rs:1630-1640`). For a buck stepping down it is an upper bound on input current, not the current.
  - The input current from the engine's own losses, at the 3S floor (9.0 V): (5.237 V × 2,000 mA + 1,170 mW) + (3.417 V × 747 mA + 550 mW) = 14.75 W, so **about 1.64 A** [computed]. At 4S's floor (12.0 V) it is about 1.23 A.
  - `input_fault_milliamps` 10,100 is the two high-side limits' maxima (2 × 5,050). A shorted buck output draws little input current, so it is no input fault current.
  - **"About twice the branch's maximum input current" is about 3.3 A** on the computed figure, or 5.5 A on the engine's bound. **Which the WO means decides the fuse datasheet to fetch.** Lean: 2 × the computed 1.64 A, a 3.15–3.5 A part, with the engine's output-sum named as the bound it is not.
- **The FET and fuse sit between J4.2 and the `VBAT` net.** The sense divider goes after the FET, so a reversed pack never reaches the ADC pin.
- **Constraints:**
  - P-FET:
    - |VDS| ≥ 34.8 V (the doubled 4S pack);
    - a zener holds VGS within its rating at 17.4 V;
    - Rds(on) stated at the gate drive the 3S floor (9.0 V) gives;
    - its drop enters the bucks' headroom, which today counts "no input-path drop".
  - Fuse: fast-blow at about twice the input current (above), DC rating ≥ 17.4 V, I²t above the hot-plug charge of C1 and the ceramics.
- **Datasheets to hold, candidates only (none asserted):**
  - a −40 V or −60 V P-FET with Rds(on) at VGS −4.5 V (look at Diodes DMP6023LSS, Vishay Si7461DP);
  - a 12 V zener (MMSZ5242B or BZT52C12);
  - a fast-acting 1206 fuse, 32 V, at the ruled current (Littelfuse 466 series: 0466003 or 0466005 by the ruling).
  - **I ask the architect to fetch the datasheets** for the FET, zener and fuse they pick; each part is declared only from them.

## Further decisions the measurement raised

- **D6 · The TLV803S's rising threshold** (M4). SBVS157E p. 6 bounds VIT− (2.87 / 2.99 V) but gives Vhys only typical (40 mV at 25 °C).
  - **The copper drop matters, so D6 and D13 are one decision.** VDD_3V3's solved minimum is 3,144.7 mV at the regulator, but at U1 the measured drop is 90.0 mV (M12), giving 3,054.7 mV. That is 24.7 mV over the 3.03 V rising threshold (typical Vhys).
    - A 165 mV limit (5 %, the blast run's choice) would permit 2,979.7 mV, under the TLV803S's 2.99 V falling maximum, so a healthy board at full load could sit in reset.
    - So VDD_3V3's drop limit must be at most vout_min − (VIT− max + Vhys) at the supervisor's own pin: 114.7 mV with typical Vhys. The rule judges the supervisor at its VDD pin, with the copper drop to that pin.
  - **Also:** the twin's droop scenarios and the 2,680 mV MCU consumer floor treat BOR3 as the reset floor. With a supervisor on NRST the reset floor is its VIT−, which the twin (fenced out) does not model. That is a FEEDBACK row for the twin work order, and a named condition here.
  - (i) **Lean:** the rule judges `VIT− max + Vhys typ` against the rail's minimum at the supervisor's pin (the solved vout minimum less the supply gate's drop to that pin), names `supervisor_hysteresis_typical` for the specification to acknowledge, and sets VDD_3V3's limit at 110 mV or less.
  - (ii) A supervisor whose datasheet bounds its hysteresis. That is a part change; the document needed is that part's datasheet.
  - (iii) The rule checks only VIT− max against the rail. That is weaker than the WO's rule.
- **D7 · The factor 2** (M6). No held page states it.
  - (i) **Lean:** label it a derivation: an ideal series LC stepped from rest peaks at twice the step, and loss only lowers it. The WO allows this.
  - (ii) Cite it. The document a ruling would need is an application note on hot-plug input transients with ceramic input capacitors, such as Linear Technology Application Note 88, "Ceramic Input Capacitors Can Cause Overvoltage Transients". It is not held, so its content is not asserted here.
- **D8 · A6** (M6): two or three ceramics per buck at 4S. **Lean two:** the engine's figure is already worst case (curve × tolerance × temperature), and two clear 10 µF by 8.7 %. Three only if the ruling wants margin over the ±1 % chart read.
- **D9 · U5's orientation and `board.forward`** (M9). All 16 pairs measured: the result depends only on U5's physical rotation. Three of its four rotations pass under any forward edge, and one overruns IMU_SCK (31,879 fF), which is 4410 Q11's. So the choice is the forward edge (free) and a U5 turn that avoids that rotation. U5's turn is a drawn-board fact: the architect's or Scotty's. **Lean:** `forward: top`, U5 `x: right` (sentinel-02-own's declaration), 25,094/27,152 fF today. I re-measure after J4's turn.
- **D10 · The 280 ledger rows** (M1). (8) needs them acknowledged. **Lean:** I write the ~37 register/class entries, each reason patterned on Sentinel-01's and checked against the ledger's own class, listed in FEEDBACK for review.
- **D11 · (9)'s VCC minimum by clock** (M4). **Lean:** the parts row carries W25Q64JV's two §9.2 rows keyed by the read clock, and the rule picks by the bus's solved rate. Sentinel-03's FLASH bus is declared at 20 MHz (`spi_buses[0].speed_hz`), under 104 MHz, so 2.7 V applies and the TLV803S passes.
- **D12 · (9)'s scope** (M12). As a `validate` rule it refuses 33 of 36 specifications, **ordered Sentinel-01 among them** (W25Q64JV 2,700 mV > BOR3's 2,540 mV). At BOR0, every 1.71 V device is over POR/PDR's 1.58 V. A refusal of `examples/sentinel-01.json` breaks its `--against-fab`, and a §7 "another board" condition sits behind it.
  - (i) **Lean:** the rule is part of `--fab-out`'s `FabricationNotReady` refusal (new packages, as (c)), and a named condition in every other verdict ("U4's 2.7 V floor is above BOR3's 2.54 V falling minimum; no supervisor").
    - Sentinel-03 meets it by the TLV803S.
    - The planted tests (the wide-range part, W25Q64JV with no supervisor, a conforming one, two non-conforming ones) judge the precondition.
    - Sentinel-01 keeps its four-row MANIFEST (the refusal is `--fab-out`'s, (c)). Its finding is a bench row: did the board ever run under 2.7 V?
  - (ii) `validate` refuses only at `bor_level` ≥ 1, on the reading that a declared BOR is a claim to supervise. That still refuses 14, Sentinel-01 among them.
  - (iii) `validate` refuses everywhere. That means 33 edits, Sentinel-01 back-filled with a supervisor: rejected.
- **D13 · VDD_5V's drop limit judges nothing** (M12). `SupplyTerminalMissing`: no on-board device has a pin on VDD_5V, and its 2 A load leaves at J6.1 and J7.1. So "`drop_limits` required wherever `power` is declared" cannot mean a limit per domain.
  - (i) **Lean:** a limit is required for every domain with an on-board load pin. A domain whose load leaves only by connector pins is named as a condition ("VDD_5V: drop not judged, its load leaves at J6.1, J7.1"), acknowledged by id.
  - (ii) The supply gate takes a connector pin as a load terminal, so VDD_5V is judged to J6.1 and J7.1. That is new gate scope.
  - VDD_3V3, measured with test minima: 90.0 mV at U1 against 165 mV, passing.
- **D14 · sentinel-02-power, the second `power` fixture,** is touched by (2), (6) and (10) too: 6S with no clamp, `power.input` with no protection, no `drop_limits`. It sits in `aethelgard-ir`'s tests (inside the fence).
  - **Lean:** `cells.max` 4, a `drop_limits` row (VDD_3V3), and the same FET and fuse, so it stays a passing fixture of the rules.
  - Its 6S form is row L1123's.
