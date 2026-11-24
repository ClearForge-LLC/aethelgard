# AET-WO-4300k — STOP after §1.2

Stopped 2026-11-24 with the measurement (`RESULTS.md`, M0–M12) and the decisions (a)–(e) the WO asks, plus five the measurement raised (D6–D10). Nothing is built: no file under `crates/`, `examples/`, `fab/`, `docs/` or `scenarios/` has changed.

**§7 conditions, checked:**
- no held PDF's hash differs (M0);
- the crystal passes AN2867 Rev 25 (M3);
- the XT30 drawing shows the opening, away from the tails (M2);
- no board has moved, since nothing is built.

**Two "held page does not support a bound you need" items fired** (D6, D7). Each has a lean that needs no new document, and the document a ruling would need is named.

## What the measurement changes in the WO

1. **Sentinel-03 cannot get a package today for a reason the WO does not name:** 280 unwitnessed ledger rows, about 37 register/class entries, have no `acknowledge_unwitnessed` entry (M1). Each needs a written `reason`. That is in (8)'s "every clause met", but it is volume the WO does not mention (D10).
2. **A6 needs two input ceramics per buck at 4S, not three** (M6). Two give 10.87 µF effective (+8.7 %), three 16.30 µF. The WO's three was computed at 6S.
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
  - In2 (VDD_3V3) joins nothing and is named as a condition. Figure 9-3 is "copper area per layer" on a four-layer board, so the minimum is taken over the joined layers, not all four.
- **The open question under (ii):** the emitted file holds unfilled zones (KiCad fills at DRC). Today's `extract` reads a board without fills. So the measure is either (ii-a) a lower bound computed in Rust from the pour outline less every other-net item's clearance, or (ii-b) read from KiCad's filled board in the `drc` gate. Lean (ii-a): no tool, and it bounds from below as the screen needs.

## (b) The fabricator, and how its single source is declared

- **Lean: JLCPCB.**
  - Sentinel-03's package is JLCPCB's format already (`*-jlcpcb-bom.csv`, `*-jlcpcb-cpl.csv`).
  - JLCPCB publishes thickness (±10 %), every `fab_tolerances` field and track width, so the supply gate's tolerances come from it.
- **Declared as** a `board.stackup.fabricator` object: `{name, documents: [{id, title, url, read}]}`.
  - Every bound's `source` becomes structured: `{fabricator: <document id>, at: "…"}`, or `{standard: "IPC-4101" | "IPC-6012" …}`, or `{condition: "…"}`, an enumerated set.
  - A bound that cites a different fabricator is refused by name.
  - A source string that merely contains "JLCPCB" counts for nothing (§5).
- **What JLCPCB does not publish:** the three dielectric minima (nominals only). Options:
  - (i) **lean:** the architect obtains a JLCPCB stackup document with dielectric tolerances (their impedance-stackup PDF or a support answer). This is a "held page" need.
  - (ii) Choose OSH Park instead. It publishes dielectric tolerances, but no thickness tolerance and no track-width tolerance, which the supply gate needs.
  - (iii) Rule that a dielectric minimum may come from a named standard (IPC-4101's prepreg tolerance) beside the fabricator's nominal.
- **The eight bounds no fabricator publishes** (copper max, permittivity max, the three copper/plating minima, copper temperature) come from `{standard}` or `{condition}`, as `copper_max_um` and permittivity do today.

## (c) Sentinel-01's `--against-fab`, byte-identical without a back-fill

- **Measured:** Sentinel-01 declares no `power` (so (2)'s `drop_limits` rule never touches it) and no `fab_tolerances`. Its MANIFEST lists four preconditions.
- **(i) Lean: the precondition belongs to the package.**
  - `--fab-out` writes a fifth row, `fabrication_ready`.
  - `--against-fab` judges exactly the rows the committed MANIFEST lists, so Sentinel-01's re-derivation compares four rows and stays byte-identical but for the hash lines and FAB-NOTES (M10).
  - A re-spin of Sentinel-01 is a `--fab-out`, which needs the gate, as `ORDERED.sha256`'s header already requires a ruling for a re-spin.
  - The objection: the committed package then decides which preconditions apply. It is the expectation, not the artifact under test, so `INV-07` holds; but the choice should be named in the verdict ("fabrication_ready: not judged, the committed package predates it").
- (ii) The row is always written, `not_applicable` for a board listed in an `ORDERED.sha256`. Sentinel-01's MANIFEST gains a row: more than its hash lines move.
- (iii) A specification key `board.ordered` exempts it. That is a back-fill in spirit, and rejected.

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
  - The false line (A13) is fixed, not acknowledged.

## (e) (10)'s current path, and the parts to hold

- **Measured (M5):** `VBAT` reaches J4.2, C1, the four input ceramics, R50 and the two bucks only. J5 has no VBAT. Steady input 2,747 mA, fault 10,100 mA.
- **The FET and fuse sit between J4.2 and the `VBAT` net.** The sense divider goes after the FET, so a reversed pack never reaches the ADC pin.
- **Constraints:**
  - P-FET:
    - |VDS| ≥ 34.8 V (the doubled 4S pack);
    - a zener holds VGS within its rating at 17.4 V;
    - Rds(on) stated at the gate drive the 3S floor (9.0 V) gives;
    - its drop enters the bucks' headroom, which today counts "no input-path drop".
  - Fuse: about 5.5 A fast-blow (2 × 2,747 mA), DC rating ≥ 17.4 V, I²t above the hot-plug charge of C1 and the ceramics.
- **Datasheets to hold, candidates only (none asserted):**
  - a −40 V or −60 V P-FET with Rds(on) at VGS −4.5 V (look at Diodes DMP6023LSS, Vishay Si7461DP);
  - a 12 V zener (MMSZ5242B or BZT52C12);
  - a fast-acting 1206 fuse, 5 A, 32 V (Littelfuse 466 series, 0466005).
  - **I ask the architect to fetch the datasheets** for the FET, zener and fuse they pick; each part is declared only from them.

## Further decisions the measurement raised

- **D6 · The TLV803S's rising threshold** (M4). SBVS157E p. 6 bounds VIT− (2.87 / 2.99 V) but gives Vhys only typical (40 mV at 25 °C).
  - (i) **Lean:** the rule judges `VIT− max + Vhys typ` (3.03 V) against VDD_3V3's solved minimum (3,144.7 mV), and names it as a condition (`supervisor_hysteresis_typical`) the specification must acknowledge.
  - (ii) A supervisor whose datasheet bounds its hysteresis. That is a part change; the document needed is that part's datasheet.
  - (iii) The rule checks only VIT− max against the rail. That is weaker than the WO's rule.
- **D7 · The factor 2** (M6). No held page states it.
  - (i) **Lean:** label it a derivation: an ideal series LC stepped from rest peaks at twice the step, and loss only lowers it. The WO allows this.
  - (ii) Cite it. The document a ruling would need is an application note on hot-plug input transients with ceramic input capacitors, such as Linear Technology Application Note 88, "Ceramic Input Capacitors Can Cause Overvoltage Transients". It is not held, so its content is not asserted here.
- **D8 · A6** (M6): two or three ceramics per buck at 4S. **Lean three:** the curve is Murata's typical at 25 °C, interpolated, and two clear it by 8.7 %.
- **D9 · U5's orientation and `board.forward`** (M9). Three of four turns pass with forward at the top (right, left, aft); `x: forward` overruns IMU_SCK. U5's turn is a drawn-board fact: the architect's or Scotty's. I re-measure after J4's turn.
- **D10 · The 280 ledger rows** (M1). (8) needs them acknowledged. **Lean:** I write the ~37 register/class entries, each reason patterned on Sentinel-01's and checked against the ledger's own class, listed in FEEDBACK for review.
- **D11 · (9)'s VCC minimum by clock** (M4). **Lean:** the parts row carries W25Q64JV's two §9.2 rows keyed by the read clock, and the rule picks by the bus's solved rate. Sentinel-03's FLASH bus is declared at 20 MHz (`spi_buses[0].speed_hz`), under 104 MHz, so 2.7 V applies and the TLV803S passes.
- **D12 · (9)'s scope** (M12). As a `validate` rule it refuses 33 of 36 specifications, **ordered Sentinel-01 among them** (W25Q64JV 2,700 mV > BOR3's 2,540 mV). At BOR0, every 1.71 V device is over POR/PDR's 1.58 V. A refusal of `examples/sentinel-01.json` breaks its `--against-fab`, and a §7 "another board" condition sits behind it.
  - (i) **Lean:** the rule is a `fabrication_ready` precondition (new packages, as (c)), and a named condition in every other verdict ("U4's 2.7 V floor is above BOR3's 2.54 V falling minimum; no supervisor").
    - Sentinel-03 meets it by the TLV803S.
    - The planted tests (the wide-range part, W25Q64JV with no supervisor, a conforming one, two non-conforming ones) judge the precondition.
    - Sentinel-01 keeps its four-row MANIFEST. Its finding is a bench row: did the board ever run under 2.7 V?
  - (ii) `validate` refuses only at `bor_level` ≥ 1, on the reading that a declared BOR is a claim to supervise. That still refuses 14, Sentinel-01 among them.
  - (iii) `validate` refuses everywhere. That means 33 edits, Sentinel-01 back-filled with a supervisor: rejected.
- **D13 · VDD_5V's drop limit judges nothing** (M12). `SupplyTerminalMissing`: no on-board device has a pin on VDD_5V, and its 2 A load leaves at J6.1 and J7.1. So "`drop_limits` required wherever `power` is declared" cannot mean a limit per domain.
  - (i) **Lean:** a limit is required for every domain with an on-board load pin. A domain whose load leaves only by connector pins is named as a condition ("VDD_5V: drop not judged, its load leaves at J6.1, J7.1"), acknowledged by id.
  - (ii) The supply gate takes a connector pin as a load terminal, so VDD_5V is judged to J6.1 and J7.1. That is new gate scope.
  - VDD_3V3, measured with test minima: 90.0 mV at U1 against 165 mV, passing.
- **D14 · sentinel-02-power, the second `power` fixture,** is touched by (2), (6) and (10) too: 6S with no clamp, `power.input` with no protection, no `drop_limits`. It sits in `aethelgard-ir`'s tests (inside the fence).
  - **Lean:** `cells.max` 4, a `drop_limits` row (VDD_3V3), and the same FET and fuse, so it stays a passing fixture of the rules.
  - Its 6S form is row L1123's.
