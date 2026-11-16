# AET-WO-4300g — FEEDBACK (build, with one measured stop: §1.1, Figure 14)

**Branch:** `wo/aet-4300g-support` from `origin/main` `dbf049e` · **Work order:** `docs/work-orders/AET-WO-4300g.md` on `main` · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace, orchestrating subagents · **Cadence:** build, with a full adversarial pass.

Each answer below is labelled **measured**, **read** (at source), or **inferred**. The predictions in §1 were committed before any measured run, including the render of DS12117, and they are not edited afterwards.

## 0. Readings taken before the predictions [read]

- **R1 · DS12117 is held.** `~/.cache/aet-ds/rescued/aet-2301/ds/ds12117-rev10.pdf`, sha256 `607b9ead…`, equal to `power-parts.json`'s `DS12117` pin. Its text layer (`~/.cache/aet-4300d-build/txt/ds12117-rev10.txt`, p. 102) gives Figure 14's value strings, `2 x 2.2μF`, `N(1) x 100 nF + 1 x 4.7 μF`, `100 nF`, `100 nF + 1 x 1 μF` (twice), `4.7μF`, `1μF`, but not which pin each belongs to. That is why the figure is rendered.
- **R2 · The symbol has 11 pins named `VDD`.** In the vendored `STM32H753ZITx.kicad_sym`, `VDD` is at 17, 30, 39, 52, 62, 72, 84, 108, 121, 131 and 144. The rail's other pins are `VBAT` (6), `VREF+` (32), `VDDA` (33), `VDD33_USB` (95) and `PDR_ON` (143), 16 in all, the review's count. `NRST` is 25, `BOOT0` 138, `PA13` 105, `PA14` 109, `VCAP` 71 and 106.
- **R3 · The droop gate sums the emitter's decoupling.** `campaign::resolve_droop` reads `schematic::decoupling_picofarads`, which sums every part marked `decouples`, and `chaos::campaign::tau_ns` scales with it. Today `VDD_3V3` carries 4 × 100 nF.
- **R4 · No reader of a schematic's netlist exists.** `strict` reads `.kicad_sch` and `.kicad_pcb` as trees. No code computes a schematic's connectivity, and `loops::rules` takes its rules from `schematic::design`, the emitter.
- **R5 · `"DS-009 DEBUG MINI"` cannot validate today, or after `4300b` (PR #152 at `25c7ebb`).** `ds009::judge` refuses every net on an `Undeclared` role, `null` included, and the port's CONSOLE TX/RX stay `Undeclared` in both.
- **R6 · `PA13` and `PA14` are already never assigned** (`solve::RESERVED_PINS`).
- **R7 · `SM06B-SRSS-TB` (JST SH 6) is already vendored** (`4300f`), with its symbol and footprint. §7's third stop does not arise.

## 1. Predictions (committed before any measured run)

| # | Prediction | Basis |
|---|---|---|
| P1 | **Figure 14, by pin.** <ul><li>VCAP: 2 × 2.2 µF.</li><li>VDD: N × 100 nF plus one 4.7 µF.</li><li>VBAT: 100 nF.</li><li>VDDA: 100 nF + 1 µF.</li><li>VREF+: 100 nF + 1 µF.</li><li>VDDLDO: 4.7 µF; VDD33USB: 1 µF; VDD50USB: 4.7 µF or none.</li></ul>Note 2's ±20 % is the value class. | inferred from R1 |
| P2 | **N on LQFP-144 is 11, not 16.** The work order's "16 per-pin capacitors" counts every pin on the rail (R2). Built to the figure, the reference gets 11 VDD capacitors, plus one each for VDDA, VREF+ and VBAT, and whatever VDD33USB's row gives. | read (R2), inferred |
| P3 | **NRST: 100 nF to ground** (Figure 22, p. 144). **BOOT0:** tied low, with no resistor value in DS12117. **PDR_ON:** tied to VDD (reset on). | inferred |
| P4 | **Placement:** with each per-pin capacitor anchored by its pin, at a 2.0 mm bound (an engine choice, as `4300d2`'s), every capacitor on the two-layer reference meets it, and the router still completes. Confidence is moderate: about 15 0603s ring an LQFP-144 whose signal pins must escape past them. | inferred |
| P5 | **Droop:** `VDD_3V3`'s decoupling goes from 0.4 µF to about 9 µF, so tau grows about 20×. At least one existing droop test's expected numbers change, and none is narrowed to keep them. | inferred (R3) |
| P6 | **CI:** the tools shard (19.3 min on `main`) crosses 20 minutes once the reference board grows and the support gate runs in `verify`, so a refit by `ci/shard.sh`'s rule is needed. | inferred |
| P7 | **Fixtures:** every one of the 19 JSON specifications lacks a programming path and a source. Each is refused by `NoProgrammingPath` first, until it declares both. | inferred |
| P8 | **`TIER2_MIN`** rises by 1 or 2, for the support gate on the reference in the image. | inferred |

## 2. §1.1, the figure read (the measured stop: it did not fire)

**How.** The held PDF (R1) was copied into the pinned tools image's `/tmp` and rendered there with Ghostscript at 130 dpi, under `~/bin/capped-run` (scope `memory.max` 4 GiB, `memory.swap.max` 0, `pids.max` 8192, read back) [measured]. No text was extracted. Each value below was read by eye from the rendered page. RM0433 Rev 9 (`~/.cache/aet-ds/rescued/aet-2301/ds/rm0433-rev9.pdf`) was rendered the same way for BOOT0. The renders are in the session scratchpad and are not committed.

**DS12117 prints the power chapter twice:** "rev Y" (§6, Figure 14, p. 102) and "rev V" (§7, Figure 65, p. 206). The two figures are the same drawing, `MSv46116V5`, with the same values. The VCAP tables (Table 14, p. 106; Table 113, p. 211) agree, and so do the NRST figures (Figure 22, p. 144; Figure 73, p. 248). The rows cite the rev V pages, as the MCU row's existing citations (pp. 209–216) do [read].

| What | Value | Where [read, rendered] |
|---|---|---|
| VDD | N × 100 nF + 1 × 4.7 µF; note 1: "N corresponds to the number of VDD pins available on the package"; note 2: ±20 % tolerated | Figure 65, p. 206, and its notes |
| Placement | "placed as close as possible to, or below, the appropriate pins on the underside of the PCB"; no distance is stated | p. 206 Caution, continued p. 207 |
| VDDA with VREF+ tied to it (this board: both on the supply) | one 1 µF // 100 nF across VREF+/VDDA to VSSA; "the 100 nF capacitors … as close as possible to the chip" | Figure 90, p. 280 (Figure 40, p. 170, in rev Y). Figure 65 draws VDDA and VREF separately, each 100 nF + 1 × 1 µF, with VREF+ tied to VDDA as an option (dashed) |
| VDD33USB | 1 µF and 100 nF | Figure 65, top |
| VDD50USB, VDDLDO | 100 nF; 4.7 µF | Figure 65. **Neither pin exists on LQFP-144** (ST's `STM32H753ZITx.xml` lists no such pin), so neither role applies |
| VBAT | no capacitor is drawn: a 1.2–3.6 V source. "The voltage on the VBAT pin could be provided by an external battery, a supercapacitor or directly by VDD, in which case the VBAT mode is not functional" | Figure 65; §3.19, p. 36 |
| VCAP | CEXT 2.2 µF (typical, ±20 %), ESR < 100 mΩ, one per VCAP pin ("Two external capacitors can be connected to VCAP pins") | §7.3.2, Table 113, p. 211; Figure 65 "2 x 2.2µF" |
| NRST | 0.1 µF to ground, the "recommended NRST pin protection"; RPU 30–50 kΩ internal | Figure 73 and Table 151, p. 248 |
| PDR_ON | enables the PDR supervisor; tied to VSS, an external supervisor is needed, and "otherwise the supply voltage must stay above 1.71 V with the embedded power voltage detector enabled" | §3.5.2, p. 28; p. 22 note 6 |
| BOOT0 | BOOT = 0 boots from BOOT_ADD0, ST-programmed to flash at 0x0800 0000; BOOT = 1 boots from system memory (the bootloader) | RM0433 Rev 9 §2.6, Table 9, p. 138; DS12117 §3.4, p. 26 |
| Package pins (LQFP-144) | VDD at 17, 30, 39, 52, 62, 72, 84, 108, 121, 131, 144 (N = 11); VDDA 33; VREF+ 32; VSSA 31; VDD33_USB 95; VBAT 6; VCAP 71, 106; NRST 25; BOOT0 138; PDR_ON 143 | ST's open pin data `STM32H753ZITx.xml` (vendored in `svd-hal`), which agrees with the KiCad symbol (R2) |

**Against P1–P3.**
- **P1** was right for VCAP, VDD and VDDA/VREF+, and for the VDD33USB 1 µF.
- **P1** was wrong on VBAT (no capacitor) and on VDD33USB, which also takes a 100 nF.
- **P2** was right: N = 11.
- **P3** was right for NRST and PDR_ON. BOOT0's level is from RM0433, not DS12117.

**What this makes the reference's network [inferred from the table]:**
- 11 VDD capacitors of 100 nF, one per pin;
- one 4.7 µF bulk;
- one 100 nF and one 1 µF at VDDA/VREF+;
- one 100 nF and one 1 µF at VDD33USB;
- two 2.2 µF at the VCAPs;
- one 100 nF at NRST;
- BOOT0 tied to ground and PDR_ON tied to the supply.

That is 13 per-pin 100 nF capacitors, not the work order's 16 (D1).

## 3. Placement, measured (§7's second stop: it did not fire)

**The bound was stated before measuring** (P4): 2.0 mm, copper edge to copper edge, from the capacitor's pad on the owner's net to the owner pin's pad. That is `4300d2`'s near bound, an engine choice, because DS12117 asks only for "as close as possible to, or below, the appropriate pins" (p. 206). The bulk capacitors' bound, 5.0 mm, was also set before measuring (`4300d2`'s bulk precedent).

**A throwaway probe** anchored 18 capacitors to their pins on every fixture that emits. Those were 11 VDD, VDDA 100 nF and 1 µF, VDD33USB 100 nF and 1 µF, two VCAP, and NRST. The probe lived on a local branch that was never pushed, and the gaps were measured with `loops::judge` [measured]:
- **First run:** 15 of 18 met 2.0 mm (0.99 to 1.67 mm). The three on the MCU's top side (pins 121, 131 and 144) sat at 2.49 mm on every fixture. The cause was the MCU's silkscreen reference, which sits above the top row of pins and widens the MCU's keep-out box there.
- **With the MCU's reference on `F.Fab`,** as `4300d2`'s K12 moved anchored parts' references, all 18 met it on every fixture that emitted. The worst gap was 1.669 mm. That is fork K1 (§5).
- **Two fixtures stopped routing with the network:**
  - `d1-24-i2c.json`: "cannot reach S9 pad 1";
  - `pair-standin.json`: "pair `USB_STANDIN` cannot be coupled: no run on F.Cu holds both conductors 300000 nm apart".

  Both route on `main`. They are taken up in §6.

**On the built reference** (the real emitter, with J1 and J2), judged from KiCad's own netlist export (KiCad 10.0.7 in the pinned image, 0.3 s), every role passes [measured]:
- the VDD capacitors at 0.99 to 1.19 mm;
- the VCAPs at 1.38 and 1.21 mm;
- VDDA and VDD33USB's 100 nF at 0.99 and 1.19 mm;
- the bulks at 1.53 to 1.67 mm.

## 4. Decisions needed (the build continues around them; each is built as stated)

- **D1 · 13 per-pin capacitors, not 16.** §2's acceptance says "16 per-pin capacitors". DS12117's note 1 gives N as "the number of VDD pins available on the package": 11 on LQFP-144 (§2, read). The other five pins on the rail take something else:
  - VBAT: no capacitor (Figure 65; §3.19 ties it to VDD);
  - VREF+: tied to VDDA, so it shares VDDA's 1 µF // 100 nF (Figure 90);
  - VDDA and VDD33USB: their own 100 nF and 1 µF each;
  - PDR_ON: a strap.

  So the reference carries 11 + 1 + 1 = 13 per-pin 100 nF capacitors, plus the 4.7 µF bulk and two 1 µF. **Built to the figure, never padded to 16.** The architect may rule otherwise.
- **D2 · Where the support gate reads, and where it runs.**
  - It reads KiCad's own netlist of the emitted schematic: `kicad-cli sch export netlist --format kicadsexpr`, 0.3 s in the pinned image [measured]. The strict reader has a third kind for it, `Netlist`.
  - The board supplies geometry only, for the bounded distances. All per-pin capacitors share one net pair, so ownership is a distance: each role at each pin takes a distinct capacitor, one to one.
  - It runs as a new subcommand, `check support --schematic --board --spec`, and as its own gate in `verify`, after ERC and DRC. That puts it in `verify`'s gate list, as §3's fence names.
  - The package gains a precondition, `mcu_support_network_judged`.
  - The alternative was to read the board's pads alone, whose nets parity already ties to the schematic. It was not taken because §1.5 says "the emitted schematic's netlist".
- **D3 · How a supply source is declared: specification v9, `mcu.supply_source`.**
  - It is a required key: `{"connector", "pin"}`, or `null` when a regulator of `power` outputs the supply.
  - A connector pin cannot be taken as a source by its net alone. The debug port's TARGET PROCESSOR VOLTAGE pin is on the supply and is a sense line, so `SupplyHasNoSource` would never fire on a board with a debug port.
  - A pin named while a regulator also outputs the supply is refused under the same kind (`SupplyHasNoSource`, "no single declared source … outputs it too") rather than as a third refusal kind. Whether two sources deserve their own name is the architect's call.
- **D4 · `"DS-009 DEBUG MINI"` cannot validate, so Sentinel-01's J1 is an explicit map.**
  - Its CONSOLE TX/RX roles are `Undeclared` on `main` and in `4300b`'s PR #152, so a `standard` of that port is refused whatever its SWD pins carry (R5).
  - J1 is therefore `standard: null` with DS-009's pin order: 1 the supply, 2 and 3 not connected, 4 SWDIO, 5 SWCLK, 6 GND.
  - `ds009.rs` is unchanged: it is `4300b`'s file, and making SWDIO and SWCLK declarable roles is useful only once the console is.
  - **Question:** should the debug port's console float (`MayFloat`) until a work order declares it? That is a reading of DS-009 this builder does not hold.

## 5. Forks taken (each for the architect to overturn)

- **K1 · The MCU's reference is on `F.Fab` when its support network rings it** (`pcb`, generalising `4300d2`'s K12). On the silkscreen, its box above the top row of pins put that row's capacitors 2.49 mm out (§3).
- **K2 · VDDA, VDD33USB and VBAT are on the MCU's supply.** No other domain is declared for them. VBAT's tie is DS12117's (§3.19), and the row's straps carry it. VDDA's and VDD33USB's are engine choices: the row's `powered` rule requires only that each be on a declared domain (Table 108 note 1).
- **K3 · BOOT0 is tied directly to GND.** RM0433 Table 9 gives the level; neither document asks for a resistor. A tie leaves no way to strap BOOT0 high for the system bootloader, which SWD does not need.
- **K4 · `SWDIO`, `SWCLK` and `NRST` are engine nets.** The emitter puts PA13, PA14 and NRST on them; a connector pin may name them; any other declared net or domain spelling one, in any case, is refused by the existing kinds `DuplicateNetName` and `NetShadowsDomain`.
- **K5 · The bounds:**
  - 2.0 mm for each per-pin 100 nF and each VCAP: `4300d2`'s near bound, an engine choice, because DS12117 states no distance;
  - 5.0 mm for the 4.7 µF and both 1 µF bulks: `4300d2`'s bulk precedent;
  - none for NRST: Figure 73 states no placement, so it is a named condition. It is still anchored for placement.
- **K6 · The value class is ±20 %** (note 2), applied to every role. Figure 73 states no tolerance for NRST's 0.1 µF, so note 2's is taken.
- **K7 · J2, Sentinel-01's 3.3 V input, is a JST GH 4-pin** (`SM04B-GHS-TB`, already vendored): pin 1 the supply, pin 4 GND. No vendored part is a 2-pin input other than the power tree's XT30. Its current rating against the MCU's draw is not judged.
- **K8 · Every support capacitor is a 0402,** the emitter's passive footprint. The 4.7 µF and 2.2 µF exist in 0402 (6.3 V X5R). Dielectric, rated voltage and ESR (VCAP's must be below 100 mΩ, Table 113) are not chosen or judged. That is a named condition in the gate's verdict and in FAB-NOTES.
- **K9 · `power-parts.json` is schema 2.** The MCU row's `kind.mcu.support` holds:
  - the pin groups, cited to ST's pin data (`STM32H753ZITx.xml`, as vendored in `svd-hal`, by sha256);
  - the supply, ground and powered groups;
  - eight capacitor roles and four straps, each cited.

  `RM0433` and that XML join the documents. `4300d3`, which shares the file, was told (bus note, 21:40 UTC).
- **K10 · The support gate takes from the specification only what it declares:** the MCU's part and supply, the domains, the connectors' pin map, and the source. It takes from the row the groups, roles, straps and bounds, and nothing from the emitter. It finds the MCU in the netlist by part number, not by reference.
- **Finding, for `4300d3`, not built here:** with PDR_ON tied to VDD (this emitter), DS12117 p. 22 note 6 puts the supply's floor at 1.71 V, while the row's `vdd_millivolts.min` is 1.62 V (Table 111). That was sent to `4300d3`'s inbox.

## 6. Routing with the network, measured: D5 (decision needed)

**Which fixtures emit** (`emit` on every specification fixture at v9; `~/.cache/aet-4300g/emit-*`) [measured]:

| fixture | `main` (v8) | this branch | the mirror (below) |
|---|---|---|---|
| `d1-24-i2c` | routes | **Unroutable `/SCL1`: cannot reach S9 pad 1** | routes |
| `pair-standin` | routes | **Unroutable `/SPI_SCK`: pair `USB_STANDIN` cannot be coupled** | Unroutable `VDD_3V3`: cannot reach C10 pad 1 |
| `d3-24-i2c-12-spi` | routes | routes | **Unroutable `/SCL3`: cannot reach S18 pad 4** |
| `solve/stress`, `sentinel-02-power` | Unroutable (already on `main`) | the same | the same |
| every other emitting fixture | routes | routes | routes |

**The cause, measured on `pair-standin`.**
- With the top row of pins ringed by their capacitors, the placer puts the I2C sensor U3 at (100.25, 86) mm, in front of the hand-pinned SPI6 pins PG13 and PG14.
- That pushes the flash U4 from (102.75, 83) on `main` to (87.5, 82.25).
- The pair is routed first, on the bare board (`AET-WO-1308`). It then has no coupled run.

**Two variants were tried and are not kept** (time-boxed):
- **The mirror** of `4300d2`'s reserve: a part not owned by an IC that owns anchored parts keeps that IC's room. It routes `d1` and breaks `d3`, and `pair-standin`'s pair then walls off C10's pad.
- **The mirror for ICs only:** the same result.

A third lever, re-pinning `pair-standin`'s I2C bus to I2C2 on PF0/PF1, routes the pair, and then C10's pad fails the same way.

**The tier-1 tests this leaves failing** [measured]:
- `adversarial_1301::attack_a_wide_reference_stays_clear_of_every_other_part` (d1);
- `stackups_tier1::d1_routes_at_four_layers_and_at_two`;
- `escape_tier1::twenty_renamings_route_d1_to_its_own_copper` (d1);
- `classes_tier1::{the_stand_in_pair_is_routed_coupled_and_measured, the_pair_judge_refuses_each_breakage, extract_bounds_a_classed_line_at_its_own_width}` (pair-standin);
- `escape_tier1::shifted_placements_route_without_reaching_the_exact_judge` (seed 22: `VDD_3V3` cannot take U3 pad 5 to a via);
- `classes_tier1::a_part_on_each_edge_is_routed_and_read_back_where_kicad_puts_it` (`/I2C_SDA`: R1 pad 2 has no node clear to end a track).

None is weakened or retargeted: d1's tests are the router's capacity claim, and editing the fixture would hide the regression.

**D5, for the architect:** the network fills the ring around the MCU, and the router, which lays a pair first and escapes a pad on its own grid, now fails eight tests on two dense fixtures and two moved placements. The options:
- (a) the router work order (`AET-WO-1310`, sub-grid escape, or a successor that routes each anchored part's pin connection before pairs) lands first, and this one waits on it;
- (b) the fixtures change: `d1` loses devices, or `pair-standin` moves its pins, and the tests are re-read as the smaller claim;
- (c) the placer gains a rule this builder did not find in its time box.

Until a ruling, this work order's final line is STOPPED on D5.

## 7. Adversarial pass (full; a fresh Opus subagent, frozen at `2cf2c19`)

It ran the four §5 attacks as worded and went beyond them, all at tier 1: it emitted the reference, built a KiCad-shaped netlist from the board, mutated it, and judged it. It also wrote emitter mutants in a scratch `schematic_support.rs`. Its harness is under `~/.cache/aet-4300g/adv/`, not committed.

| Id | Severity | Finding | Disposition |
|---|---|---|---|
| A1 | high | The gate had no rule for which net NRST, VCAP, SWDIO and SWCLK must be on. Three emitter mutants (NRST or a VCAP on the supply) and seven netlist edits passed with no finding, among them SWDIO shorted to SWCLK and NRST on `VDD_3V3`. The reset role is unbounded, so any 100 nF anywhere filled it. | **Fixed.** The row's `own_nets` (VCAP, NRST, SWDIO, SWCLK, each cited) requires each pin on a net that is no declared domain, not `GND`, and no other support pin's. Tested: five renames each fail by name. |
| A2 | high | A powered group was accepted on any declared domain, VDDA on 5 V included. Table 108 note 1 says "in the permitted range". | **Fixed.** Each powered group carries its range: VDDA 1.62–3.6 V and VDD33USB 0–3.6 V (Table 111, p. 209, read by rendering), VBAT 1.2–3.6 V (p. 27). The domain's whole tolerance band must lie inside it. Tested. |
| A3 | medium | `supply_source` could name the debug port's own supply pin, the sense line D3 excludes. | **Fixed.** Step 18 refuses a source on the programming connector (`SupplyHasNoSource`, naming why). Tested. |
| A4 | medium | The gate read no population flag and no symbol, and `100n DNP` passed. | **Fixed.** The netlist reader reads `libsource` and the properties. A capacitor is `Device:C` with no `dnp`, `exclude_from_bom` or `exclude_from_board`. A value is a capacitance, optionally followed by a capacitor part of the table. Tested on all five forms. KiCad's property shape `(property (name …) (value …))` was checked against a real export (§3); the flags' names are KiCad's, as the reviewer recalls them, not yet seen in an export. |
| A5 | low-medium | A connector pin declared not connected could carry a net. | **Fixed.** It must be on no net (`connector_pin_off_net`). Tested. |
| A6 | low | §5.3 (a regulator's output renamed) is refused, but at step 16 (`UnknownDomain`, `DomainWithoutSource`), before step 18. | **Covered by a test as worded.** `SupplyHasNoSource`'s null branch is reachable only with no power tree. |
| A7 | low | Three points against RUL-20: <ul><li>the gate sits in `eda`, beside the emitter (as §3's fence places it);</li><li>its distances reuse `loops::gap_nm` and `route::read`, geometry shared with the router;</li><li>a condition restated the emitter's "0402";</li><li>the emitter took the first MCU row while the gate looks it up by part.</li></ul> | The condition and the row lookup are **fixed**. The placement in `eda` and the shared geometry readers are **for the architect**: neither reads an emitter choice, and moving the gate to `ir` would need `ir` to read boards. |

**Held** [measured by the reviewer]:
- §5.1: each of the 19 role capacitors removed; one moved to four wrong nets; seven wrong values.
- §5.2: SWDIO and SWCLK swapped at either end; GND or the supply dropped.
- §5.4: six emitter mutants that fail correctly.
- The value edges: 79n and 121n out; 80n and 120n in.
- One capacitor per pin.
- The MCU's identity.
- The strict reader: duplicated sections, a doubled node, version `D`.
- Every `validate` refusal path.
- Every debug-net spelling.
- Every rendered citation in the row.

## 8. Mutation pass (diff-sized)

One mutant per guard this work order adds, each against the suite its test lives in (`~/.cache/aet-4300g/tools/mutate.py`, run in its own worktree) [measured]:
- **First run, 27 mutants:** 24 caught, 3 survived.
  - m14 (the nets check): the "another pin's net" test was caught by the distance check by accident.
  - m26 (ground pins) and m27 (powered pins): no test exercised them.

  Three discriminating tests were added: a capacitor between the wrong nets within its bound, a VSS pin off `GND`, and VDDA on no domain. All three mutants are now caught.
- **The fixes of §7 add 7 guards,** m28–m34 (own nets, powered range, population, symbol, value tail, NC pins, source not on the debug port). Each mutant is caught.
- **Total: 34 mutants, 34 caught.**

## 9. Reduction pass (diff)

A fresh subagent proposed cuts against the four questions. All four were applied; the fifth was labelled instead.

**Reduction pass (diff): −10 net · 3 duplicates removed · 1 dead path removed · 1 protected kept (labelled) · 5 findings below.**
- **Duplicates removed:**
  - `programming_path` is now `programming_connector(spec).is_some()`, one predicate for the validator and the gate;
  - one notes-section writer for the power tree's and the support network's sections, byte-identical (the notes goldens hold);
  - one wording, `quoted`, for a missing net, where there were six copies.
- **Dead path removed:** the gate's three `Err` fallbacks on the parts table, which `row()` had already unwrapped.
- **Protected, labelled `belt-and-suspenders:` in this pass:** the emitter's refusal of a row pin the symbol lacks. It is backed by `the_mcu_rows_support_pins_are_the_symbols` for the one MCU row today.
- **Protected by design (`RUL-20`), kept unlabelled:** the gate's own value reader beside the emitter's, and the gate's and emitter's row lookups.
- **Proved:**
  - the tier-1 suite: every failure left is D5's (§6);
  - clippy `-D warnings` with tier 2 compiled;
  - `fmt`;
  - all 34 mutants re-run on the reduced code: 33 caught, and m27 is superseded, its guard replaced by A2's range check, which m29 covers.
- **Findings, for the architect, not cut:**
  1. `verdict::{SupportRole, Programming}` mirror `support::{RoleRecord, Programming}`, because `eda` carries no serde.
  2. `Stated` exists for clippy's argument count. It copies three `Bound` fields.
  3. The netlist's strict rules name `once` children no gate reads. That is stricter than the module's convention.
  4. `eda::support::row` could be private.
  5. `check support`'s two refusal mappers repeat `main.rs`'s `Stop` shape, outside this diff.

## 10. Tier 2, CI and the predictions against the results

**Tier 2 in the pinned image** (every tier-2 binary, `~/.cache/aet-4300g/logs/t2-*.log`) [measured]. These classes of failure were found and fixed:
- **The verdict printed a scratch path.** `verify`'s support record carried the board's scratch path, so two runs printed different bytes, and `--against-fab`'s MANIFEST differed. It now names the board by sha256 only.
- **The swap test is caught one gate earlier.** It now times its swap after the support gate's copy, so `extract`'s binding is still the one exercised.
- **Fixed coordinates against the moved board:**
  - 1302's floating loop;
  - 1301's extra parity footprint;
  - two corpus edits (an F.Cu rectangle and a copper text).

  Each is placed clear of the support network, its meaning unchanged.
- **Expected lists gain the new gate:** `verify`'s gate order, and the package's preconditions.
- **P5:** see the table below.

The tier-2 tests left failing are D5's four:
- `adversarial_1301_tools::attack_wide_references_leave_the_emitted_board_clean` (d1's S9);
- `with_tools::{a_part_on_each_edge_passes_erc_and_drc, the_class_and_pair_fixtures_pass_erc_and_drc_and_the_pair_is_reported, d3_and_d1_pass_erc_and_drc_with_parity_at_four_layers}`.

**The support gate on KiCad's real export:** the new tier-2 test passes. All 19 roles hold; a VDD capacitor moved 25 mm off fails by its role alone. `TIER2_MIN` is 130, plus 1 for it, exact.

**CI on `a4cdb58`** (run 43772121004) [measured]:
- **Every tools shard ran under 20 minutes.** The slowest was 14 m 50 s. Every shard failed, on D5's tests and on the tier-2 items fixed above.
- **The image and ardupilot jobs passed.**
- **The `gates` job failed on D5's tier-1 test** `adversarial_1301::attack_a_wide_reference_stays_clear_of_every_other_part`.

`fmt` and `clippy -D warnings`, tier 2 included, pass locally.

| # | Prediction | Result | Right? |
|---|---|---|---|
| P1 | Figure 14 by pin | VCAP, VDD and VDDA/VREF+ right. VBAT has no capacitor; VDD33USB also takes 100 nF (§2) | partly |
| P2 | N = 11, not 16 | 11 [read], so 13 per-pin capacitors (D1) | right |
| P3 | NRST 100 nF, BOOT0 low, PDR_ON to VDD | right; BOOT0's level is from RM0433 | right |
| P4 | Every capacitor within 2.0 mm, and the router completes | Within 2.0 mm once the MCU's reference moved to F.Fab: worst 1.67 mm (§3). The router does not complete on two dense fixtures (D5) | half |
| P5 | Decoupling about 0.4 to 9 µF, and a droop test's numbers move | 0.4 to 8.3 µF. The reference droop's crossings moved: VBOR3 rising 1 012 422 222 to 1 016 291 380 ns, falling 1 014 422 222 to 1 018 322 818 ns, then 1 225 977 778 to 1 229 512 382 and 1 227 977 778 to 1 231 681 149 ns. Resets 1 014 430 / 1 227 980 to 1 018 330 / 1 231 690 µs; the firmware is observed at 1 331 690 µs, not 1 327 980; the boot's instructions 9 877 201 to 9 876 830. The test is re-pinned to the measurement; the verdict and the reset count are unchanged | right |
| P6 | The tools shard crosses 20 minutes | the slowest shard ran 14 m 50 s, though shards that failed may have stopped early | not yet shown |
| P7 | Every fixture refused `NoProgrammingPath` first | all 18 valid fixtures (subagent's table, re-read here); `invalid-zero-domain` keeps `ZeroNominalVoltage` | right |
| P8 | `TIER2_MIN` +1 or +2 | +1 (130) | right |

## 11. Close: STOPPED on D5

**Built:**
- the MCU row's support network (`power-parts.json` schema 2), every value read by rendering;
- specification v9 with `mcu.supply_source`, and the debug nets;
- `NoProgrammingPath` and `SupplyHasNoSource`;
- the emitter drawing the network from the row, anchored by owning pin;
- the support gate on KiCad's netlist: `check support`, `verify`'s gate, the package precondition, FAB-NOTES;
- Sentinel-01's J1 and J2;
- every fixture moved to v9.

**Passes made:** the adversarial pass (§7, A1–A5 fixed), the mutation pass (§8, 34 of 34), and the reduction pass (§9). `fmt` and clippy are green.

**Why STOPPED:**
- **Not met:** acceptance's "the power fixture and every other fixture pass", and CI green. D5 (§6) leaves eight tier-1 and four tier-2 tests failing, all on router capacity around the ringed MCU. None is weakened.
- **Rulings that unblock it:** D5, plus D1–D4, which carry the schema.
- **Next free versions:** `4300d3` (PR #158) also takes specification 9 and parts schema 2; whichever merges second takes 10 and 3.

**The architect's CI clause of 23:55 UTC** (bus): a shard plan at or under 1006 s per shard, and a green final run with every shard under 20 minutes. It is not met at this STOPPED head:
- the run cannot be green while D5 stands;
- the plan is to be taken after `4300b` merges and its weights land, per the clause.

The last measured run's shards were all under 20 minutes (§10).

## 12. The rulings of PR #159 comment 6904190430, applied, and D5's spike: STOPPED a second time

**Applied:**
- **D3:** two declared sources are their own refusal, `SupplyHasTwoSources`, naming the regulator. It has a tier-1 test, and the ir README's step 18 names it.
- **D1, D2, D4 and the forks stand as built.** `ds009.rs` is untouched, and the console is not `MayFloat`.

**D5's spike** (about one hour of the two allowed; `route.rs` ordering and the placer's reserve only; no fixture edit and no re-pinning) [measured]:
- **Lever 1, stubs first.** Before any pair or net, each two-pad part's pad within 2.0 mm (copper edge to copper edge) of a multi-pad IC's pad on the same net, other than a filled net, is joined to that pad on F.Cu. `route_net` then reaches the two as one, through the stub's copper.
  - Stubs are found from the board alone (`STUB_NM` 2.0 mm, `4300d2`'s near bound). The tests that route a board read back from its own file therefore lay the same stubs.
  - On `pair-standin`, all 20 stubs were laid, the 19 support capacitors' among them.
- **Lever 2, the pair's approach.** The placer keeps a strip clear in front of each declared pair's MCU pins: their span plus 0.5 mm each way, from the pins to 5.0 mm past the MCU's courtyard. It is a taken rectangle for every part placed after the MCU.
- **Lever 2 with the mirror reserve** (§6) was also measured, the placer's reserve being in the widened fence. It is not proposed.

| fixture or test | lever 1 | levers 1+2 | levers 1+2 with the mirror |
|---|---|---|---|
| `pair-standin` (3 tier-1 tests in `classes_tier1`, 1 tier-2 in `with_tools`) | Unroutable: pair cannot be coupled | **routes** | Unroutable `VDD_3V3`: C10 pad 1 |
| `d1-24-i2c` (`stackups_tier1::d1…`, `escape_tier1::twenty_renamings…`, `adversarial_1301::attack_a_wide_reference…`; tier 2: `with_tools::d3_and_d1…`, `adversarial_1301_tools::attack_wide_references…`) | Unroutable `/SCL1`: S9 pad 1 | Unroutable `/SCL1`: S9 pad 1 | routes |
| `d3-24-i2c-12-spi` | routes | routes | **Unroutable `/SCL3`: S18 pad 4** |
| `escape_tier1::shifted_placements…` seed 22 | — | Unroutable `VDD_3V3`: U3 pad 5 to a via | — |
| `classes_tier1::a_part_on_each_edge…`, `with_tools::a_part_on_each_edge…` | — | Unroutable `/I2C_SDA`: R1 pad 2 has no node clear to end a track | — |
| every other fixture that routes today | routes | routes | routes |

**Not every one routes, so this stops as ruled.**
- **The spike's code is not in the branch.** It is kept as `~/.cache/aet-4300g/spike-d5-levers.patch` (350 lines, against `b2ec444`; its `AET_MIRROR` and `AET_STUBS` switches are probes). It is for `AET-WO-1310`, which takes routing order with its escape work. This PR waits on it.
- **Where the spike leaves 1310 [measured]:** lever 2 is what routes `pair-standin`. Lever 1 lays every stub but does not by itself change any outcome measured here.
- **d1's S9 [measured]:** S9 sits about 4 mm off the MCU's lower-right corner, behind the ring's right column, beside C29, C30, R12 and S10. That failure, seed 22's and the edge test's are escape failures at a crowded terminal, the kind `1310`'s sub-grid escape addresses.
- **Kept out of the branch with the patch:** the spike also moved routed copper on every board, so the extraction terms, the corpus edits and the tests' fixed copper positions would each need re-taking.

## 13. Close: AET-WO-1310 and `main` merged, specification 11, power parts 3, READY

**The merges** (the architect's comment 6911284656 on PR #160):
- `AET-WO-1310` was squash-merged into this branch as `046089b`. Its escapes and pair approach close D5: every fixture routes, with no fixture edit and no test re-pinned for it. The twelve D5 tests pass.
- `origin/main` was merged with a merge commit (`e8a567c`). It brought `4300b` (specification 9, serial ports, topology 7, trace 11, `TIER2_MIN` 155, the in-shard weights) and `4300d3` (specification 10, power parts 2, the DC-bias curve). Every conflict kept both sides:
  - **ir.** `NetKind` holds both `Debug` and the serial kinds. The id mapping holds both arms. 4300d3's control-pin `Strap` stands beside this work order's support strap, renamed `SupportStrap`.
  - **`power-parts.json`** was merged as data: main's table, plus this branch's two documents (RM0433 and ST's pin data) and the MCU row's `support`, which was this branch's only change to the file.
  - **Fixtures** were merged as data: main's content, plus this branch's debug connector, supply input and `mcu.supply_source`. A script checked that this branch changed nothing else in any of them, and that no reference clashed.
  - **The reference's connectors renumbered.** 4300b gave it a console J1, so this work order's debug mini is J2 and the 3.3 V input J3, with the source on J3 pin 1.
  - **4300b's two new serial fixtures and ir's serial test builder** gained a programming path and a source, on references their own tests do not use.
  - **The CI files.** The weights are main's refit plus 1310's rows. `TIER2_MIN` is 157: main's 155, plus 1 from this work order (the support gate on KiCad's netlist) and 1 from 1310. It was counted exactly from the shard plan's list: 157 tier-2 entries.
- **Renumbered:** specification 9 to **11**, and power-parts schema 2 to **3**. The ir README's version text, validation steps 1, 11 and 17–18, and its example all follow. `4230` (PR #161) also claims specification 11; the second to merge renumbers.

**Re-taken, each moved only by the merge, with its old value in the test's own comment:**
- **The committed board crate and the UI boards** were re-blessed (`AETHELGARD_BLESS`); only their specification-hash lines moved.
- **Three digests in `order_2203`** moved with the specification's sha256 alone. The boot image did not move.
- **`solve_cli`'s reference and stress digests** moved with the specification's sha256 alone. The re-blessed topology shows the assignments unchanged.
- **The SPI extraction terms and the SCK bound case** moved because the routed copper moved: SCK 15 534 to 16 498 fF, MOSI 17 567 to 17 023 fF.
- **The strict corpus's positions** shifted with the board. The re-key was done by a subagent and checked here: 14 files, index-only except the rail edit, which was re-keyed to the same rail.
- **Expectations naming the new gate's precondition** (`mcu_support_network_judged`) gained it, in both package tests. That was CI on `c8bed77`: shard 0 failed on the identifier test, the one place the merge had not reached.
- **4300b's console test** now drops only the console's connector, so the programming path stays.

**Gates and CI** [measured]:
- `fmt` and clippy `-D warnings` (tier 2 compiled) pass, and all 1787 tier-1 tests pass, at `61ef2dc`.
- **The shard plan** (the 04:35 rule): balanced, every shard planned at 1058 s, 0 % above the mean, and the guard holds over 1961 test names.
- **CI on `61ef2dc`** (runs 43838126962 and siblings) is green on every job, the `tools` aggregate included (`TIER2_MIN` 157 held). The shards ran:

| shard | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|---|---|---|---|---|---|---|---|---|
| `c8bed77` | 12 m 50 s (failed, above) | 18 m 23 s | 16 m 17 s | 21 m 45 s | 14 m 50 s | 20 m 10 s | 18 m 01 s | 18 m 15 s |
| `61ef2dc` | 15 m 20 s | 18 m 30 s | 20 m 23 s | 20 m 34 s | 18 m 22 s | 17 m 07 s | 18 m 22 s | 20 m 34 s |

- **Over the 20-minute target:** three shards, each by under 35 s. Under the 04:35 rule that is a measurement, not a failure.
- **No refit is triggered.** The plan is balanced, and the slowest shard did not lead by 3 minutes on two consecutive runs: shard 3 led by 1.6 min, then tied with 7.

**For the architect's next docs PR:** the README's quickstart pin moves, since the reference board changed (§2 of the work order).

**After CI on `61ef2dc`**, `main` gained the docs-only PR #162 (`a45eb02`), merged here as `e683ac7`. No file outside `docs/` changed since `61ef2dc`, so the code CI judged is this head's.
