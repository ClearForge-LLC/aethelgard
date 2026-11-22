# AET-WO-1216 — FEEDBACK: the power sequence (design spike, STOPPED at §7 for the schema ruling)

**Branch:** `wo/aet-1216-power-sequence` from `main` `9cb87b4c`.
**Builder:** Claude (Opus 5.5 orchestrating). Two workers:
- one Sonnet facts worker read the held datasheets into `spike/1216/out/facts-worker.md`;
- one Opus adversarial reviewer was given the code and §5, not my conclusions.

Every report was checked at source before use. Written 2026-11-22.

**Status: STOPPED at §7 for the schema ruling.** No crate, specification, `fab/`, workflow or `ci/` file changes.

**The answer, in five lines:**
- **34 committed boards, every path listed.** Two independent walks (the specification, and the drawn netlist of the 25 boards that draw) agree on 388 of 388 rows. Four boards were also checked by hand.
- **Two stated sources reach an MCU pin from off the MCU's domain:**
  - the battery sense divider on Sentinel-02 and Sentinel-03, **2536 mV** at a 26.1 V pack, under the 4.0 V limit;
  - D1's DRV8876 IPROPI, **4665 mV** on the engine's held 3 mA bound, over 4.0 V, **if the driver drives**. The held bound is a recommended-operating figure: the 3.5 A peak output gives 5442 mV at the typical ratio, and the abs max is 5.75 V. No held page states its level in sleep.

  Every other path is an off-board end with no stated bound: connectors on 33 boards (DS-009 prints a nominal +3.3V, no maximum), test points, the bench's foreign clock.
- **No held page orders the two LMR33630 bucks.** Both have EN on VIN and PG open; soft-start is 2.9 to 6 ms, and UVLO is typical only. The TLV755P follows 3V3 by the tree.
- **Measured refusals:**

  | option | refused |
  |---|---|
  | (c) VDD = 0, stated paths | 1 / 34 (D1) |
  | (c) with unbounded paths refused | 33 / 34 |
  | (b) EN/PG order | 10 (7 where the rail reaches a pin) |
  | (a) declared sequence | 10 (8 where a rail reaches a pin) |

  A hand sequence on the four boards refuses D1 alone.
- **Lean: (c) where it can judge, with (b)'s order as the only exemption:**
  - every path from a rail the drawn tree does not place after `mcu.supply` is judged at VDD = 0 (Table 10: 4.0 V, BOOT0 9.0 V);
  - unbounded off-board ends are named, not refused.

  It refuses D1 alone today. It needs no new field. It leaves the off-board ends and every dynamic case unjudged (§4).

## 0. How it was made

- Predictions were committed before any reader ran: `11d10be3`.
- The readers and the options as rules (`judge.py`, `RULES.md`, four hand sequences) were committed before `judge.py` first ran: `b66d5d7b`.
- The measurement: `d9cb8a7f`. The corrections after the adversarial pass, the reduction pass and this file follow it, in one commit.
- **Readers:** in `spike/1216/`. They read the CLI's output of `main`'s crates (`solve` JSON, emitted `.kicad_pcb`), built from `9cb87b4c` into scratch. They do not link the crates (D6).
- **Datasheets:** read in their held text. Every load-bearing row was then checked on the rendered page: DS12117 pp. 27, 103, 104, 136; SLVSDS7B p. 4; SNVSAN3F pp. 7, 8; SBVS320D p. 6. The PDFs' sha256 match `power-parts.json` `documents` and `parts.json` `pin_parts`.
- **D1 bound reported on the bus at once:** the over-4.0 V figure went to the architect as soon as it was found (`~/.cache/aet-bus/architect.md`, 2026-11-22). It is not §7's immediate stop, because option (c) judges it.

## 1. The measurement (§1.1–§1.2)

The table, per class and per board, is in `spike/1216/RESULTS.md` §1 and `out/summary.md`. Its one home is there; this section keeps only what the options turn on.

- **The stated sources:**
  - the battery divider, 2536 mV on PF3, on Sentinel-02 and Sentinel-03 (equal to the solved `VREF+` floor);
  - D1's IPROPI (§1.1).
- **Unbounded:**
  - connector pins on 33 boards (6 also carry `VDD_5V`);
  - test points;
  - the foreign clock.
- **Not off-domain:** I2C pulls (98, all on `mcu.supply`), pin-device pulls, the crossing, control wires.
- **Non-FT pins on a path:**
  - PE9 (TT_ha), Sentinel-03's MOTOR1 at J5;
  - test points on PA5 (TT_ha) and PA7 (TT_a) on three Sentinel-01 boards;
  - NRST and BOOT0.

### 1.1 D1's path, and the three facts no held page states (kept by the architect's note, 07:35Z)

`crates/aethelgard-ir/tests/fixtures/d1-drv8876.json` (`board.drawn: false`):
- **The path:** U2 DRV8876 IPROPI → R2 1.5 kΩ → PF3 (FT_ha, ADC).
- **The rails:** VMOTOR (12 V ±5 %, J5.3) and VDD_3V3 (J3.1) arrive from two external supplies, and nothing orders them.
- **The worst:** 4665 mV.
  - It is IIPROPI's 3 mA (SLVSDS7B p. 4, §6.3, recommended operating, the engine's `output_max_ua`) × R2 at its worst: 1 % + 100 ppm/K × 65 K over D1's declared −20 to 85 °C + 2 % drift.
  - Against 4.0 V (DS12117 Table 10 p. 103; note 5 p. 104).
  - The over-4.0 V figure went to the architect, who ruled at 07:35Z that it is not the §7 stop: D1 draws no board, and (c) judges the path.
  - The held bound is not IPROPI's worst. The same table (p. 4) allows a 3.5 A peak output current: 3.5 mA at the typical 1000 µA/A, or 5442 mV, with no AERR stated above 2 A. The abs max is 5.75 V. The first figure, 4683 mV, took R2's TCR over −40 to 125 °C; the adversarial pass corrected it to the device's declared range.

The worst case is reached only if all three of these go the unfavourable way, and **no held page states any of them:**
1. **IPROPI's level in sleep.**
   - With the MCU down, nSLEEP sees only the unpowered MCU pin and the driver's 100 kΩ typical pull-down (p. 3, p. 5).
   - Sleep "disables the H-bridge, charge pump, internal 5-V regulator, and internal logic" (p. 18), and IPROPI mirrors the low-side current (p. 12).
   - No page says IPROPI sources nothing in sleep.
2. **The clamp's figure.** "An internal IPROPI voltage clamp circuit to limit VIPROPI with respect to VVREF" (p. 12). VREF is tied to VDD_3V3, the rail that is down, but the page gives no figure, offset or response time. The abs max is printed as two rows: −0.3 to 5.75 V and −0.3 to VVM + 0.3 V (p. 4).
3. **nSLEEP in the brown-out window.**
   - nSLEEP's VIL max is 0.8 V at VVM ≥ 5 V (D1's VM is 12 V) and its VIH min 1.5 V (p. 5).
   - As VDD falls through 1.5 V to 0.8 V with VMOTOR still up, the MCU's nSLEEP output follows VDD, so the driver's state is undefined while VDD is already below 1 V.
   - No page states what the bridge does there.

## 2. What the held parts state about order (§1.3)

The full table is in `RESULTS.md` §3. The figures that decide the options:

- **LMR33630 (U10 5 V, U11 3V3), from one battery:**
  - EN is strapped to VIN on both; VEN-H is 1.2/1.231/1.26 V (SNVSAN3F p. 7).
  - Soft-start is internal, 2.9/4/6 ms (p. 8).
  - Input UVLO: no threshold is tabulated (the minimum operating input is 3.8 V max, p. 7). VCC starts at "about 3.7 V" (p. 15, §8.3.4), typical only.
  - PG goes high "about 4 ms (typical)" after EN (p. 12). PG is open drain, abs max 22 V (p. 6).
  - **No page states a start order, a skew, or a sequencing method**, so the two race.
  - A held-page order exists: PG pulled to its own VOUT through 100 kΩ (p. 12) and wired to the other's EN. The schema cannot declare the pull-up a control net needs (F2).
- **TLV755P (U12, 1V8 from 3V3):**
  - VUVLO is 1.21/1.3/1.44 V. tSTR is 550 µs, typical only (SBVS320D p. 6).
  - VOUT ≤ VIN + 0.3 V abs max (p. 4).
  - Its input is 3V3, so 1V8 cannot lead `mcu.supply`.
- **TXU0304:** "either supply rail may be powered on or off in any order" (SCES935A pp. 20, 22). Outputs are Hi-Z below 100 mV on either VCC; 100 mV to 1.08 V is not stated.
- **DRV8876/DRV8833 (D1, D2):**
  - VM and the logic rail arrive on two connectors, and no page orders them.
  - DRV8876 VM UVLO rising is 4.3/4.45/4.6 V (p. 6).
- **STM32H753 (DS12117 Rev 10):**
  - §3.5.1 p. 27 sequences only the supplies: with VDD < 1 V, the others stay below VDD + 300 mV.
  - Table 10 p. 103 at VDD = 0: FT 4.0 V, TT 4.0 V, BOOT0 9.0 V, other 4.0 V.
  - Table 11 p. 104: IINJ +0 on FT, TT, RST and B. Note 3: "Positive injection is not possible on these I/Os and does not occur for input voltages lower than the specified maximum value."
  - Table 48 p. 136: functional susceptibility, positive injection 0 or NA on every pin.
  - **Nothing is stated about an I/O pin with VDD absent** beyond Table 10's formula.

**Where the declared tree fixes an order:** only Sentinel-02's 1V8, after 3V3. Everywhere else it fixes none:
- the two bucks race on both boards with a tree;
- the other eight multi-domain boards declare no source for their other domains.

## 3. The options (§1.4), each with its cost, its files and its measured refusals

The rules are in `spike/1216/judge.py`, written up in `RULES.md` and committed before they ran. The results are in `out/options.tsv`.

### (a) A declared order, `power.sequence`, a source per step, A1 judged at each step

- **Shape:**
  - `power.sequence: [ { "rails": ["battery"], "source": "…" }, { "rails": ["VDD_5V", "VDD_3V3"], "source": "…" }, … ]`.
  - Rails in a step at or before `mcu.supply`'s are judged at VDD = 0.
  - Later rails are judged by A1 with the MCU powered.
  - A step with no source is refused.
- **Refuses today:**
  - **a1**, required of any board with a second domain or a tree: **10 / 34**.
  - **a2**, required only where a non-supply rail reaches a pin: **8 / 34**.
  - Both by `SequenceUndeclared`, until each of those files declares one.
- **Hand sequences:**
  - `as-held`, the only order the held pages support, on the four boards: **refuses D1 alone**. VMOTOR and 3V3 share a step, so IPROPI is judged at VDD = 0.
  - `5v-after-3v3-unsourced` on Sentinel-03: refused (`SequenceStepUnsourced`).
  - `bench-procedure` on D1 (3V3 first, by the operator's procedure): passes.
- **Cost:** a new specification field (specification 25). The files it touches:
  - `crates/aethelgard-ir/src/power.rs` (or a new module): the type, serde, and a validate step;
  - `versions.rs` (a `CHANGES` row 25);
  - the CLI README's kinds;
  - the 8–10 fixtures that need a sequence;
  - the walker of (c), run per step;
  - tests.
- **A worst that rests on an unstated fact:**
  - The sequence decides only *whether* a path is judged at VDD = 0. When it is, (a) judges the held bound, as (c) does.
  - On D1, `as-held` judges 4665 mV and refuses it, whatever the sleep level or the clamp.
  - `bench-procedure` (3V3 first) takes IPROPI out of the VDD = 0 judgement entirely. It does so on the operator's word, not a part's page.
  - It also leaves fact 3 untouched. A power-up order says nothing about power-down unless the sequence declares that order too, and the brown-out window is a power-down case.
  - So (a) can pass a path whose worst rests on an unstated fact by declaring it away.
- **What it buys:** a bench procedure can be declared, as D1's `bench-procedure` shows.
- **What it costs in truth:**
  - a declared order is only as true as its source, and no gate checks that the hardware enforces it;
  - on today's held parts, an honest sequence puts the racing bucks in one step, and then (a) gives exactly (c)'s answer.

### (b) The order derived from the tree's EN/PG wiring alone; an undeclared or racing pair refused by name

- **Shape:** no new field. A domain follows `mcu.supply` only if:
  - its input chain passes through `mcu.supply`, or
  - its regulator's EN is wired to a PG on `mcu.supply`'s chain.

  Otherwise it is `RailsRace` (a sibling) or `SourceUndeclared` (no tree). The battery leads, and its paths are judged at VDD = 0.
- **Refuses today:**
  - **b1**, every racing or undeclared domain: **10 / 34**. `RailsRace` on Sentinel-02 and Sentinel-03; `SourceUndeclared` on six 5 V/1V8 fixtures, D1 and D2.
  - **b2**, only where that rail reaches an MCU pin: **7 / 34**. Sentinel-02's racing 5V reaches no pin.
- **Cost:**
  - a validate step in `power.rs`;
  - 7–10 fixtures changed: each needs a tree, or a wired EN with a pull-up the schema cannot declare (F2);
  - Sentinel-03 itself needs a drawn change, U10.EN from U11.PG;
  - tests.
- **A worst that rests on an unstated fact:**
  - On D1, (b) refuses first by `SourceUndeclared` (VMOTOR has no tree), before any voltage is computed, so the three facts never enter.
  - If a tree placed VMOTOR's regulator after 3V3 by EN ← PG, the exemption would take IPROPI out of the VDD = 0 judgement.
  - That would rest on stated figures for power-up (PG high about 4 ms after EN, p. 12).
  - On power-down, PG falls at 90–95 % of VFB (p. 8), above 1 V, while PG is valid down to VIN 2 V. But the LMR33630 states **no output discharge** in shutdown, so VMOTOR's stored charge could still be up while VDD is below 1 V. Fact 3 is then unjudged.
- **What it buys:** the exemption is read from wiring that is drawn, so it cannot be wished.
- **What it cannot see:** a bench supply on a connector (D1, D2), and any off-board end.

### (c) No sequence; every off-domain path judged at VDD = 0 (4.0 V; BOOT0 9.0 V)

- **Shape:** no new field. A validate step walks the paths walk 1 lists:
  - the sense divider;
  - pin-device pulls and current outputs;
  - I2C pulls;
  - crossings;
  - control wires.

  Each is judged against Table 10 at VDD = 0.
- **Refuses today:**
  - **c1**, a stated worst above the limit: **1 / 34**, D1 (`OffDomainAboveLimit`, 4665 > 4000 mV).
  - **c2**, also every unbounded path, `INV-06`'s reading: **33 / 34**. Every board has a connector pin on an MCU net, and nothing in the schema bounds an off-board end.
- **Cost:**
  - one walker in `crates/aethelgard-ir` (the path classes are already typed in `power.rs`, `pin_devices.rs` and `lib.rs`);
  - Table 10's limits as data on the MCU row (`power-parts.json` `io`, beside 1211e's Table 8 structures);
  - a `CHANGES` row naming the rule;
  - the CLI README;
  - D1's fixture (D3);
  - tests.
- **It is the most conservative reading:** it judges a 5 V rail as if it always led.
- **A worst that rests on an unstated fact:**
  - (c) takes the held bound and assumes every unstated fact goes the unfavourable way: the driver driving, no clamp, nSLEEP anywhere.
  - On D1 that refuses (c1 and c2 alike).
  - It clears only by a remedy resting on a stated figure, such as R2 (D3), never by an unstated favourable behaviour.
  - Where no held bound exists at all (an off-board end), c1 names the path and c2 refuses it.
  - Fact 3, the window, is covered on D1 only because the bound is static: the held bound applies at any nSLEEP. (c) does not model the window itself.

## 4. The lean

**Lean: (c1), with (b)'s derivation as its only exemption.** Its parts:
- every path from a rail the drawn tree does not place after `mcu.supply` is judged at VDD = 0 against DS12117 Table 10 (4.0 V; BOOT0 9.0 V);
- a rail the tree places after `mcu.supply` (downstream of it, or EN wired to a PG on its chain) is judged by A1 alone, as today;
- a path with no stated bound is named in `validate`'s output, not refused.

**Reasons:**
- **It never takes an unstated fact as favourable.** A worst that rests on one is judged at the held bound (D1: the three facts of §1.1). A path with no held bound is named, not passed silently.
- **It needs no new field.** The exemption comes from drawn wiring, so the gate never trusts an order no part enforces. That is the risk (a) carries, and it is `INV-07`'s concern in another form.
- **It is the most conservative reading wherever a bound is stated.** It refuses exactly the one committed path that exceeds 4.0 V on a figure the engine holds: D1's IPROPI.
- **(a) adds nothing over it on today's held parts.** The honest `as-held` sequences reproduce c1's single refusal, and a convenient order fails for want of a source.
- **(b) refuses 7–10 fixtures for an order none of them needs.** Sentinel-02's racing 5V reaches no MCU pin, and the remedy on Sentinel-03 needs a schema addition (F2).
- **c2 would refuse 33 of 34 boards today,** for a bound no specification can yet declare.

**What it leaves unjudged:**
- **Off-board ends,** on 33 boards: connectors, test points, the foreign clock. On 6 boards a 5 V-powered peer shares the connector. This is D2's question and F5's row.
- **Every dynamic case:**
  - a battery hot-plug ring through the divider;
  - power-down;
  - the band where VDD is between 0.8 V and 1 V and D1's nSLEEP is undefined (VIL max 0.8 V at VVM ≥ 5 V, VIH min 1.5 V, SLVSDS7B p. 5);
  - VDD between 1 V and 1.62 V, where Table 10's formula gives VDD + 4.0 V and no rule runs;
  - the WO excludes a dynamic simulation.
- **Table 13, by design:** 1211e's D-A1 judges with the MCU powered at Table 13's operating conditions (VDD 1.62–3.6 V). Table 13 states nothing below 1.62 V, so below 1 V the only stated limit is Table 10's absolute maximum. The lean uses an abs-max rating where A1 uses an operating one, and says so.
- **Table 48 (functional susceptibility):** positive injection is 0 or NA everywhere. It concerns function, not damage, and the MCU is not running.
- **Back-powering:** what a path's current does inside an unpowered MCU (through its I/O structure into VDD). Table 11 note 3 says no positive injection occurs below the maximum, so the voltage is the whole test the datasheet gives.
- **IPROPI's real level:** no page states it in sleep or at the clamp. The lean judges the engine's held 3 mA bound.

## 5. Adversarial pass (§5, targeted)

The reviewer was an Opus agent given the code and §5, not my conclusions. Its report is `spike/1216/out/adversarial.md`, with probes in scratch.

**Reproduction:** it re-ran both readers and `judge.py` and got byte-identical outputs.

**Missed paths: none.**
- No missed path on any committed board.
- Supply pads, the `rail()` heuristic, NRST, BOOT0, SWD and the oscillator, the crossing's low side, the nFAULT outputs, the USB pins and the c1 limits are clean.
- A data-driven walk of every IR field naming a domain or a net found none that spec.py misses.
- No diode is drawn anywhere.

**What it found, checked at source and applied** (the list is `RESULTS.md` §6):
- **Counts:** 25 drawn and 9 not, not 27 and 7.
- **D1's bound:** R2's TCR range taken from the pin device's −20 to 85 °C, giving 4665 mV instead of 4683.
- **The 3 mA is not IPROPI's worst:** the 3.5 A peak output gives 5442 mV (D5).
- **DS-009's +3.3V nominal** (F5).
- **`esc` read backwards:** `none` declares no ESC fed through the board, so 4 rows were removed.
- **Non-FT pins** listed (F1).
- **Citations:** nSLEEP VIL 0.8 V at VVM ≥ 5 V; LMR33630 §8.3.4 and its 3.8 V minimum input; PG 90–110 %; 98 I2C pulls, not 132.
- **The battery's connector name** made consistent.

**No option count moved.**

**Not fixed, reported** (rows F7, F8):
- **A gap: no drawn cross-check of pin-device paths.** D1 and D2 do not draw.
  - Read from the code: a drawn D1 would give walk 2 four rows that walk 1 does not list (EN, PH, nSLEEP, nFAULT touch the VMOTOR-powered U2).
  - Walk 1 classifies them as non-sources: logic inputs with internal pull-downs, and an open drain.
  - 1211f is making D1 drawable, and the reconciliation should be re-run then.
- **Weaknesses in `RULES.md` that `judge.py` implements faithfully** (synthetic inputs, no committed board affected):
  - (b) counts an EN wired to the PG of a regulator *upstream* of `mcu.supply` as "after", and labels a parent rail racing;
  - the sidecar admits a sequence that omits rails, puts the battery after 3V3, or gives any non-empty source;
  - a device with two supplies slips past b2 and the sidecar.

  These are design inputs for the build the ruling orders, not results.
- **The divider's series ohms:** the nominal Thevenin, 9091 Ω; the low corner is 8759 Ω.
- **The battery's worst is steady state.** No page or engine field gives a hot-plug ring peak.

## 6. Predictions beside results

The full table is in `RESULTS.md` §5:
- **Every count holds:** the universe (34), the boards with a path (33), the two stated sources, the D1 candidate, and the option counts (a 8–10, b 7–10, c 1 and 33).
- **One part is wrong:** I predicted no soft-start minimum is stated; SNVSAN3F p. 8 states 2.9 ms.
- **One figure is 5 % low:** the divider's 2.4 V, against 2536 mV measured.

## 7. Decisions for the architect, and rows

**Decisions:**
- **D1 · The schema.**
  - **Lean:** (c1) with (b)'s exemption (§4).
  - **Alternatives:** (a) declared sequence; (b) refuse races; c2.
- **D2 · Unbounded off-board ends.**
  - **Lean:** named, not refused.
  - **Proposed row:** "a connector pin's off-board drive bound (`connectors[].pins[].offboard.drive_max_millivolts` with a source), judged at VDD = 0 like any other path; DS-009 states none (F5). Owner: a connectors WO."
- **D3 · D1's fixture under the lean.** c1 refuses it (4665 > 4000 mV).
  - **Options:**
    - (i) R2 1.27 kΩ E96: 3 mA × 1316 Ω = 3949 mV, under 4.0 V (on the held bound; at a 3.5 A peak it would be 4.6 V, D5). 1211e's A2 still holds: full scale 1995 µA × 1316 Ω = 2.63 V ≤ VREF min 3.135 V.
    - (ii) Keep 1.5 kΩ and hold a stated clamp figure. TI states none.
    - (iii) Under (a), declare the bench procedure.
  - **Lean:** (i).
  - **Sequencing:** 1211f is drawing D1's bulk now, so the fixture edit belongs to the build after 1211f merges.
- **D4 · The limit's reading.**
  - Table 11 note 4 calls any VIN > VDD positive injection, and IINJ is +0. Note 3 says positive injection "does not occur for input voltages lower than the specified maximum value".
  - I read note 3 as admitting VIN up to Table 10's maximum. The strict reading, no VIN > VDD at all, would refuse every battery divider: Sentinel-02 and Sentinel-03.
  - **Lean:** note 3's reading.
- **D5 · A current output's bound.**
  - The engine holds DRV8876 `output_max_ua` 3000 from p. 4's recommended conditions; it is not a stated output limit. The 3.5 A peak output current gives 5442 mV.
  - Read as "not stated", c1 refuses **0** and D1 joins c2's unbounded paths (the adversarial pass's reading).
  - **Lean:** judge the held bound and name it in the message as a recommended-operating figure, so the one committed over-4.0 V path is not passed by c1.
  - If the ruling prefers "not stated", D3 still applies: the remedy is the same R2 change, judged against 5.75 V or a stated clamp.
- **D6 · Readers read CLI output, not linked crates.**
  - §3 says "built against main's crates as a path dependency". The readers instead parse `solve` and `emit` output of a binary built from `main`'s crates, so `spike/1216/` carries no `Cargo.toml` that a workspace or CI sweep could pick up.
  - The walk-2 reader is independent of every crate by design: it reads only the drawn netlist.

**Findings (rows for §7.1):**
- **F1 · Sentinel-03's MOTOR1 is on PE9, a TT_ha pin, at J5.** It is the one connector-facing TT pin on any committed board. Test points also land on PA5 (TT_ha, injection −0/0 in Tables 11 and 48) and PA7 (TT_a) on three Sentinel-01 boards.
  - With VDD up, Table 13 holds a TT pin to VDD + 0.3 V. A 5 V-logic peer on J5 would exceed it even when powered.
  - **Proposed row:** "connector-facing signals on FT pins, or the off-board level bounded (D2)."
- **F2 · (b)'s remedy cannot be declared today.** LMR33630 PG is open drain (abs max 22 V, so it cannot be pulled to a 6S pack), and EN "must not be allowed to float". So U11.PG → U10.EN needs a pull-up to U11's VOUT on the control net, and the schema has no pull on a control wire.
  - **Proposed row:** "a declared pull on a control-wire net," if (b) is ever ruled.
- **F3 · DRV8876's IPROPI abs max is printed as two rows** (SLVSDS7B p. 4): −0.3 to 5.75 V and −0.3 to VVM + 0.3 V. The clamp to VREF (p. 12) has no figure. The engine holds neither.
- **F4 · DS12117 states nothing about an I/O pin with VDD absent** beyond Table 10's formula at VDD. Its power-down energy clause (p. 27, under 1 mJ) concerns the supplies, not the I/Os.
- **F5 · DS-009 prints +3.3V for its TELEM, GPS and I2C signal pins** (pp. 6–9, the Volt column). This is a nominal level with no maximum, and DS-009 says nothing on whether a peripheral may drive a line while the autopilot is unpowered.
  - The rows now carry it. A `"DS-009 TELEM"` connector still bounds nothing a gate can judge.
  - It is the obvious seed for D2's drive bound: "3.3 V nominal" would need a stated maximum.
- **F6 · Excluded from the universe, by name:**
  - `spike/**` records: 1211's and 1211d's D1/D2 copies, `spike/4600`'s v16 trees, and `spike/1215/in/v24-newer.json`;
  - `docs/work-orders/*.repro`;
  - the scenario overlays, which override only I2C pulls on `mcu.supply`.

- **F7 · A pin device on a drawn board has no reconciliation yet** (adversarial gap).
  - **Proposed row:** "re-run 1216's two walks on D1 once 1211f draws it; walk 2's four U2 rows (EN, PH, nSLEEP, nFAULT) should be classified by the pin row's kind, not the part's rails."
- **F8 · The rule weaknesses above,** for whichever option is ruled:
  - (b): the order must point from `mcu.supply`'s chain *downstream*;
  - (a): a sequence must name every rail once, with the battery first;
  - every option: a device with two supplies is judged on each.

## 8. Gates, fence, reduction pass

- **Gates not run.** This branch is spike-only: no crate, specification, `fab/`, workflow or `ci/` file changes, and the WO says the gates are not run on it. `git diff --stat origin/main` lists only `spike/1216/` and this file.
- **Fence:** `spike/1216/` and this FEEDBACK only.
- **Reduction pass (diff):** −5 net in the readers since `d9cb8a7f` (618 → 613 lines, the adversarial fixes included) · 2 duplicates removed · 1 dead path removed · 0 protected kept · 0 findings below.
  - **Duplicates:**
    - the board-name rule (the file's stem, `solve-` before the solve fixtures), in 4 copies (`run.sh`, `walk.sh`, `judge.py`, `summary.py`), now lives in `universe.sh` alone;
    - the class column `drawn.py` printed twice.
  - **Dead path:** a loop in `spec.py`.
  - **Proof:** every output re-run and diffed against the pre-reduction copy. `out/spec`, `options.tsv`, `compare.tsv` and `summary.md` are identical; `boards.tsv` and `out/drawn` differ only by the name key and the dropped column.
  - **FEEDBACK §1** now cites `RESULTS.md` §1 instead of restating its table.
- **Scratch:** `~/.cache/aet-1216`, holding the release target, emitted boards, renders and the held texts. It is removed at close except `facts.md` and `adversarial.md`, which are copied into `spike/1216/out/`.
