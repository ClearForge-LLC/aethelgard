# AET-WO-4300d2 — FEEDBACK (build, with one measured stop: §1.4, the placer)

**Branch:** `wo/aet-4300d2-board` from `origin/main` `27e504a` · **Work order:** `docs/work-orders/AET-WO-4300d2.md` on the docs branch (`claude/argon-repo-docs-synthesis-rjrb0d`, PR #148) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace, orchestrating subagents · **Cadence:** build, with a full adversarial pass.

Each answer below is labelled **measured**, **read** (at source), or **inferred**. The predictions in §1 were committed before any measured run, and they are not edited afterwards. The only container runs before that commit were two `ls`/`grep` listings of the image's KiCad libraries, which established that the parts exist.

## 0. Readings taken before the predictions [read]

- **R1 · `fab` is reachable for a power tree.** `verify --fab-out` runs ERC, DRC with parity, `extract`, `prove`, `build`, `simulate --firmware`, then `fab::derive` (`cli/src/verify.rs` `run_gates`). `campaign` runs only when scenarios are named (`expected` empty skips it). So §1.9's "`campaign` keeps refusing" and "`fab` passes" do not conflict: the fixture runs `verify --fab-out` with no scenario. *(Note: `verify`'s fab flags required `--scenarios` at `3300` (`INV-03`); this is re-checked at the build.)*
- **R2 · The ADC channels are in ST's XML, and the table is parsed in `svd-hal`, not `ir`.** `aethelgard-ir` holds only `PinTable`/`Pin` (`lib.rs`). `aethelgard-svd-hal::parse` reads `STM32H753ZITx.xml` and keeps a `<Signal>` only if the GPIO file gives it a `GPIO_AF`, so every `ADCn_INPm`/`INNm` signal is dropped today. 28 of 114 I/O pins carry ADC signals (61 distinct). Carrying them is one field on `ir`'s `Pin` plus a few lines in `svd-hal::parse` from the signal set it already holds. **`svd-hal` is outside this fence and inside `4300b`'s:** the change goes in one isolated commit, `4300b` is told, and it is decision D1.
- **R3 · The crossing splits two judged lines.** `extract::lines` bounds each SPI bus's SCK and MOSI. Behind the TXU0304, the MCU's `IMU_SCK`/`IMU_MOSI` reach U13's A side, and U5 sits on new B-side nets. Today's extractor would refuse U13 on the line (`UnmodelledPart`), and no B-side line would be bounded at all.
- **R4 · The datasheets state no placement distance.** LMR33630 SNVSAN3F p. 33, 10.1, items 1–4 and 9: "as close as possible", "close to", "short, wide traces". TLV755P SBVS320D p. 21, 7.4.1: "as close as possible". TXU0304 SCES935A p. 25, 12.1: "as close to the device as possible". **§1.4's first branch ("within its datasheet distance") cannot be evaluated as written:** there is no distance. Every bound below is an engine choice, and its citation says the datasheet states none (the D-B5 precedent).
- **R5 · Classes are declared by net name, and the power tree makes nets the specification cannot name.** Step 13a (`classes.rs`) admits a declared signal net, a domain id or `GND`. The battery net and each buck's SW, BOOT, VCC and FB nets are engine-made, so no declared class can name them today.
- **R6 · `fab`** refuses any drilled pad and any drill file other than one `T1C0.300` tool (`cli/src/fab.rs` `no_drilled_pad`, `holes`). FAB-NOTES' Holes section says "Every hole is a plated via". `4300f` F2 (`SPI_CONDITION` reaches `extract`'s verdict but not FAB-NOTES) is **unruled**.
- **R7 · Every power part exists in KiCad 10.0.7** (GitLab tag `10.0.7`, fetched to `~/.cache/aet-4300d2/vendor/`):
  - symbols `Regulator_Switching:LMR33630ADDA` (extends `LMR33640ADDA`), `Regulator_Linear:TLV75518PDBV` (extends `TLV70012_SOT23-5`), `Logic_LevelTranslator:TXU0304PW`, `Device:L`, `Connector_Generic:Conn_01x02`;
  - footprints `Package_SO:Texas_HSOP-8-1EP_3.9x4.9mm_P1.27mm`, `Package_TO_SOT_SMD:SOT-23-5`, `Package_SO:TSSOP-14_4.4x5mm_P0.65mm`, `Inductor_SMD:L_Vishay_IHLP-5050`, `Connector_AMASS:AMASS_XT30PW-M_1x02_P2.50mm_Horizontal`, `C_0603_1608Metric`, `C_1210_3225Metric`, `R_0603_1608Metric`.
- **R8 · K6 is not unblocked by `4210` as built.** `4210`'s parts schema 3 (PR #143, open) carries no output delay. K6 adds it on top of schema 3, after #143 merges. Slice A recorded that the committed fixture is refused by `validate` for `OutputDelayMissing` alone.

## 1. Predictions (committed before any measured run)

| id | prediction | label |
|---|---|---|
| P1 | **Vendoring.** Seven symbol files (the two derived symbols with their parents) and eight footprints, each byte-equal to GitLab's blob at `10.0.7`, pass every library test (pinned hash, courtyard, no local clearance). Both derived symbols flatten through `library::symbol`. **Risk:** the XT30's two unnumbered through-hole pegs and the Texas HSOP's two unnumbered SMD pads are refused by a reader rule (`route::part`, `pad_box`) and need a named reading. | inferred |
| P2 | **ERC (D12).** The emitted tree passes `check erc` with no finding, and no check is waived, once: <ul><li>a rail an LDO's `power_out` pin drives carries no `PWR_FLAG`, and every other rail keeps one;</li><li>EN is tied to VIN (buck) or IN (LDO), and OE to VCCA;</li><li>PG is a no-connect.</li></ul>The first run shows at least one finding the emitter must answer (a `power_out` conflict or an undriven `power_in`). | inferred |
| P3 | **§1.4, the placer without anchors.** <ul><li>Parts whose only nets are rails (Cin, Cin_hf, Cout) land by whichever IC shares the most rails, ties to the lower reference. So U11's input capacitors land by U10 or J1, not by U11's VIN.</li><li>Parts on an engine-made label net (the divider on FB, CBOOT on BOOT, CVCC on VCC, the inductor on SW) land beside their own regulator's pin.</li></ul>So anchors are needed (branch two). | inferred |
| P4 | **The bounds, chosen now (engine choices; the datasheet states none, R4).** Copper edge to copper edge, from the part's pad on the net it shares with the pin, to that pin's pad: <ul><li>≤ 2.0 mm for Cin and Cin_hf (both at VIN and at PGND), CBOOT (BOOT), CVCC (VCC), RFBT and RFBB (FB), the LDO's Cin (IN) and Cout (OUT), and each translator bypass (VCCA, VCCB);</li><li>≤ 3.0 mm for the inductor at SW, and for the sense reservoir at its ADC pin.</li></ul>With anchors, every part meets its bound. Buck Cout, the CS pull-up and the sense divider's resistors are named as unchecked. | inferred |
| P5 | **Routing.** The 2-layer fixture routes when: <ul><li>declared classes cover the battery net and `VDD_5V` at 0.5 mm;</li><li>`VDD_3V3` takes `1308`'s via-pad class (0.7 mm pad, 0.3 mm drill) at the default width;</li><li>each buck's SW net takes its output domain's class.</li></ul>"Wide enough for the current" is named as a condition: no current-to-width source is held. The first full route fails on at least one net. | inferred |
| P6 | **Thermal and joins.** Each buck's EP gets 12 vias (4 × 3, 0.254 mm drill) into the B.Cu `GND` zone at 2 layers. J1's two pins join solid. DRC passes with the gate's `thermal_<ref>_<pad>` rules. | inferred |
| P7 | **`fab`.** The drill file has one tool per declared drill: 0.254 mm (24 hits), 0.300 mm (the routed vias), and the XT30's pin and peg drills. Its per-tool hit counts equal the board's. No existing board's package changes. | inferred |
| P8 | **The sense pin.** Carrying the channels is a table extension (one field, under 40 lines with its test). The battery-sense net solves to a free ADC pin deterministically. No existing serialized pin table or topology changes bytes, because the field is skipped when empty or held out of the serialized topology. | inferred |
| P9 | **K6.** After `4210` merges, ICM-20948's tVD.SDO max of 59 ns as a cited field gives a 95 ns round trip at the fixture's 4 MHz, which passes, as slice A measured on a table copy. If #143 is still open when everything else is green, this build stops on it. | inferred |
| P10 | **Byte identity.** No specification version moves. Every existing board's schematic, board, BOM, verdicts and fab package are byte-identical with `main`'s (`emit-all` comparison), with no hash line moved. | inferred |
| P11 | **Tier 2** gains 1–2 tests in existing `TIER2_RE` binaries: the fixture through `verify --fab-out`, and possibly one thermal or loop DRC case. So TIER2_MIN is `main`'s +1 or +2. The loop checks are tier 1. | inferred |
| P12 | **CI.** Every shard stays under 20 minutes after at most one refit of `ci/test-weights.tsv`. The fixture's `verify` adds 3–6 minutes to its shard. | inferred |
| P13 | **Size:** 3,000–4,500 lines added, most in `aethelgard-eda`. | inferred |

## 2. §1.4, the placer, measured (the measured stop)

**Where it ran.** On Furnace, as `forge`, with the host build. A probe dumped every pad's placed centre and size from `pcb::write_moved`. It was an uncommitted `AET_PROBE_LAYOUT` block, applied by `~/.cache/aet-4300d2/tools/probe-on.py` and removed after each run. `~/.cache/aet-4300d2/tools/gaps.py` measured each gap. The fixture was slice A's crossing-free copy (`common/power.rs`'s filter) [measured]. Raw dumps and gaps are in `~/.cache/aet-4300d2/probe/`.

**Today's placer, without anchors** (`layout-noanchor.tsv`). 19 of 20 loop distances exceed the bounds chosen in P4 [measured]:

| part | rule | gap (mm) |
|---|---|---|
| C10 / C20 | Cin at VIN | 5.24 / 13.75 |
| C11 / C21 | Cin_hf at VIN | 6.57 / 9.14 |
| C12 / C22 | CBOOT at BOOT | 1.75 / 7.54 |
| C13 / C23 | CVCC at VCC | 2.92 / 9.26 |
| L1 / L2 | inductor at SW | 8.64 / 32.75 |
| R10 / R20 | RFBT at FB | 8.26 / 38.58 |
| R11 / R21 | RFBB at FB | 9.62 / 8.90 |
| C30 / C31 | LDO Cin at IN / Cout at OUT | 7.79 / 10.98 |

**P3 is half wrong.** Rail-only parts do land by the wrong IC. But so do label-net parts: L2 shares one label net (SW) with U11 and one rail with the MCU, and the tie goes to the lower reference, the MCU (`pcb::place`'s rule). **§1.4's first branch cannot be evaluated as written** (R4: no datasheet states a distance), so branch two was taken: anchors.

**The anchors, as built.**
- **Data.** Ownership gives each part its IC and its pins: the role maps to the regulator row's pin roles (`power-parts.json` `pins`). No new specification field was needed.
- **Placement.** Each anchored part is placed straight after its IC. It is tried at each of the four right angles, at the free spot nearest the origin that would put each of its pads on its IC pin, and the angle with the least summed squared distance is kept.
- **Room.** An IC that owns anchored parts keeps the longest side of its largest one, plus the IC gap, from every part already placed.
- **References.** An anchored part's reference text moves to `F.Fab`; on the silkscreen it made a 0603's keep-out box 4.3 × 2.83 mm.
- **Order.** The high-frequency Cin first, then CBOOT, the inductor, CVCC, the divider, the bulk Cin and Cout.

Each step was measured; the dumps are `l5`–`l10.tsv`.

**With anchors** (`l10.tsv`). Every gap but four meets P4's bound [measured]:
- Cin_hf 1.25/1.25 mm;
- CBOOT 1.25;
- CVCC 1.26–1.27;
- RFBT 1.52 and RFBB 1.67–1.70;
- the LDO's Cin 1.18 and Cout 1.33;
- L2 2.77.

**What did not meet P4** (prediction wrong):
- **The bulk Cin**, at 3.03 mm (VIN) and 4.63 mm (PGND) on U10, and 4.58 / 2.98 mm on U11. Once the 220 nF capacitor sits on the pins, which TI asks for (SNVSAN3F p. 23, 9.2.2.6), the next free spot is behind it.
- **L1**, at 3.50 mm: CBOOT and CVCC take the column beside SW.

**The bounds were then set at 5.0 mm (bulk Cin) and 4.0 mm (inductor)**, after the measurement, and each rule's text says so. That is decision D3, not a fork.

## 3. Forks taken (each for the architect to overturn)

- **K9 · The IMU behind the crossing is not drawn yet.**
  - The Sentinel-02 fixture's U5 is an ICM-20948, which no emitter draws (`UnknownPart`, as `spi-imu.json` has been since `4200`).
  - KiCad 10.0.7's only footprint for it, `Sensor_Motion:InvenSense_QFN-24_3x3mm_P0.4mm`, carries an F.Cu keepout zone with `(connect_pads (clearance 0))` [read]. The gate's strict reader refuses every `(clearance N)` but a pad shape option (`board.rs:175`, `RUL-03`). The footprint writer copies every child, so any board with that part is refused `LocalClearance` [read].
  - So it is not vendored. The crossing's drawing (`schematic/power.rs` `crossing`, `behind`) is built, but no specification reaches it until U5 can be drawn. That is decision D2, together with K6.
- **K10 · The battery is the rail `VBAT`.** It is claimed by the emitter when a tree is declared (`ReservedName` on a collision), and step 13a admits it as a class net.
- **K11 · A buck's SW net takes its output domain's class** (`stack::switch_nets`): the inductor carries the output's current. On the fixture, U10's SW is 0.5 mm (`POWER`) and U11's 0.2 mm (`SUPPLY_MCU`, the `1308` ruling's default width). Whether a class is wide enough for its current is named, not judged: no current-to-width source is held.
- **K12 · An anchored part's reference is on `F.Fab`, not the silkscreen** (`pcb::reference_on_fab`). The BOM and the placement file name every part.
- **K13 · A circle pad is reached in its inscribed square.** `Pad::inner` shrinks a box by its rounding, so a circle's inner box is its centre, which lies inside the XT30PW-M pad 2's 1.9 mm hole: the router found no terminal [measured: `Unroutable VBAT`, "J1 pad 2 has no node clear to end a track"].
  - `Pad::reach` is the inscribed square only for a pad that is all rounding (a circle or an oval); every other pad keeps its inner box, so no existing pad's terminals move. No footprint vendored before this work order has such a pad (read: every pad of `data/kicad-10.0.7/footprints` listed).
  - The router's terminals and the connectivity judge both use `reach`. A first version made the judge exact (`fits`), but `routing_tier1`'s cut test showed it then counted a track ending in a pad's rounded corner as joined, a semantic change, so it was replaced [measured].
- **K14 · `SolvedPower.sense` is a required key, always written** (`{}` when no sense is declared). A topology without it is refused, never read as "no sense solved" (`INV-06`). Only power-tree topologies, which no test digests, gain bytes.
- **K15 · `campaign` refuses a power tree as `PowerNotSimulated`.** The message names the battery-sag work order (§1.9). `PowerNotEmitted` is gone from `eda`, `emit` and `check drc`.
- **K16 · The placer measures an `fp_arc` as its whole circle's box**, and `extract`'s footprint reader admits `fp_arc` (refused on a copper layer, as every graphic is). The XT30PW-M's silkscreen rings its pegs with arcs, and both readers refused it [measured]. Without this, `§7`'s first stop would fire for the power-in connector.

## 4. Decisions needed (asked early: the build continues around them)

- **D1 · `svd-hal` is outside this fence.**
  - The pin table is parsed in `aethelgard-svd-hal::parse`, which dropped every ADC signal.
  - The extension is 8 lines in one commit (`e0e4466`): `adc_channels` filled from the `<Signal>` set `parse` already reads. Plus a test file, and the table-digest test re-taken (`cee06f3`), whose old digest still holds with the field cleared.
  - It is a table extension, so §7's second stop did not fire. `4300b` (whose fence it is) was told.
  - Accept, or move the lines?
- **D2 · The ICM-20948's footprint (K9) blocks READY.**
  - Options:
    - (a) the strict reader admits a keepout zone's `connect_pads` clearance inside a vendored footprint (a `RUL-03` amendment; a keepout fills nothing, so the clearance binds nothing);
    - (b) the emitter drops the footprint's keepout. This loses the library's no-copper-under-the-die area, so it is not recommended;
    - (c) the fixture's 1.8 V device becomes another part with a footprint the gate reads;
    - (d) the crossing waits for a 1.8 V part (ruled D9: the recommended parts need none), and this work order closes without it.
  - Every other gate of §1.9 is built and proved on slice A's crossing-free copy, the same copy slice A's tests use.
  - With K6 (which waits for #143), this is the reason the final line would be STOPPED and not READY.
- **D3 · The loop bounds.**
  - Every bound is an engine choice (R4). The ones chosen before the measurement were 2.0 mm for every part, and 3.0 mm for the inductor and the reservoir.
  - Two were set after it: 5.0 mm for the bulk Cin, behind the 220 nF capacitor, and 4.0 mm for the inductor (§2).
  - Accept, or give a source for a tighter rule? A four-layer Sentinel-02 (which TI recommends, SNVSAN3F p. 34) would not change them; the limit is in-plane.
- **D4 · SNVSAN3F 10.1 item 5 asks for a ground plane on an inner layer.** The fixture is two-layer (slice A), so it is named as not met in the verdict. Move Sentinel-02 to four layers?
- **D5 · §1.9's `fab` clause and its `campaign` clause cannot both hold** [read].
  - `verify --fab-out` and `--against-fab` require `--scenarios` (`cli/src/main.rs:118-122`: "`INV-03` counts the fault campaign", `3300`).
  - `fab::derive` is reached only from `verify`'s gates, after the campaign.
  - §1.9 keeps `campaign` refusing a power tree until the battery-sag work order. So no path writes a fabrication package for the Sentinel-02 fixture.
  - Built anyway, and proved where it can be: `fab`'s drill tools, on `1309`'s through-hole and thermal fixtures and on KiCad's own drill files pinned from the image. The adversarial pass exported the power board's drill file in the image and found it equal to `fab::drills` (measured there; no committed test holds it, because no package is derived for the fixture).
  - Options:
    - (a) `verify --fab-out` without scenarios for a specification whose campaign is refused by name (an `INV-03` exception);
    - (b) the battery-sag work order first;
    - (c) accept `fab` proved below `verify` for this work order.

## 5. The rulings, applied (PR #149, comment of 14:43 UTC, and the architect's answers to D5–D7)

- **D1 · accepted.** `svd-hal::parse`'s 8 lines stay (`e0e4466`).
- **D2 · (a).** The ICM-20948's KiCad 10.0.7 symbol and footprint are vendored again (blobs equal to GitLab's).
  - **The reader** (`board::admitted_keepout`) admits a keepout zone's `(connect_pads (clearance N))` only when the zone is a direct child of a footprint and carries `keepout`, and only when `connect_pads` holds that clearance and nothing else.
  - **The router** (`route::part`) reads a footprint zone, which must be a keepout. It must be a right-angle rectangle on one copper layer, with each of its five kinds `allowed` or `not_allowed` and no net; anything else is refused by name. The router blocks tracks on that layer, and vias, wherever their copper would reach in.
  - **The gate** (`route::check`'s `keepouts`) refuses a track, a via or thermal via, another part's pad, or a fill on that layer, inside it. KiCad's own DRC also judges rule areas.
  - **The footprint writer** refuses to turn a footprint carrying a zone.
  - **`extract`** admits a footprint's keepout as no copper.
  - **The library test** admits `(clearance` only inside a footprint keepout's `connect_pads`.
  - **The ICM-20948 is drawn** to DS-000189 p. 20 (4.1, 4.3).
- **K6 · built after #143 merged** (`1080652`).
  - **The table:** parts.json schema 4 gives every row `spi_output_delay_max_ns` with its paired citation; only an SPI part may carry it. The ICM-20948 holds 59 ns (DS-000189 p. 17, Table 8, tVD.SDO at 20 pF; note 1: characterization of 5 parts, in the datasheet's words).
  - **The check:** `power::check` reads the delays from the table. The committed fixture, crossing and all, validates and solves: the round trip is 95 ns at 4 MHz.
- **D3 · accepted as named engine choices.** SNVU573A (the EVM guide) shows its layout as images without dimensions (D-B5), so no TI distance is cited. The bounds stand as stated, each rule's text saying so.
- **D4 · Scotty's (`4500`).** The fixture stays two-layer, with SNVSAN3F 10.1 item 5 named as not met.
- **D5 · (c).** `fab` is accepted as proved below `verify` for this work order. The fixture's full package waits for the battery-sag work order. Named here and in the verdict's conditions; FAB-NOTES carries the power conditions.
- **D6 · (b).** `extract` refuses, by name (`TranslatorPinUnbounded`), a line that carries a translator's data pin. SCES935A p. 9 prints Cio once per temperature range, centred across MIN, TYP and MAX, at OE = GND and 3.3 V on both sides, so no maximum is held for this circuit [read, rendered at `~/.cache/aet-4300d2/render/txu-cio.png`].
- **D7 · READY with the crossing waiting** (asked after D2 (a) was applied).
  - **What fails:** the committed fixture is drawn, but its board does not route: "U5 pad 20 has no node clear to start a track" [measured].
  - **Why:** the QFN-24's pads are 0.2 mm wide at 0.4 mm pitch. The engine floor is 200 µm of track and clearance (`FLOOR_UM`, `RUL-03`) and the grid is 125 µm, so a track clears its neighbours only when it is centred exactly on the pad.
  - **The gate's pad clearance is also conservative here:** it measures pads by their boxes, which read the corner pads 1 and 24 as 0.177 mm apart. As rounded copper they are 0.218 mm apart [measured on the footprint; computed].
  - **What follows:** a router work order (pin-centred escape for sub-grid pads, and an exact rounded-pad clearance) brings the crossing's board up. Until then a tier-1 test pins both refusals by name.

## 6. What was built

| piece | where | proved by |
|---|---|---|
| Vendored parts: the LMR33630 and TLV755 (with their parents), the TXU0304, the inductor, Conn_01x02, the ICM-20948; 9 footprints | `aethelgard-kicad-lib`, `eda/src/library.rs` | library tests (pinned sha256, courtyard, keepout-only clearance) |
| The tree drawn: regulators, divider (`K5`), external parts, translator, sense and reservoir, J1 | `eda/src/schematic_power.rs` | ERC 0 findings in the image; tier-1 tests |
| The battery rail `VBAT` (K10); a rail a `power_out` pin drives carries no flag (D12) | `schematic.rs` | tier-1 flag test; ERC |
| Anchors from ownership; the placer orders, turns and reserves room for loop parts; their references on F.Fab | `schematic_power.rs`, `pcb.rs` | §2's measurement; every loop rule passes |
| The loop gate in `check drc --spec`: loop rules, `K5`, `route::check` (arrays, joins, keepouts), classes with widths, named conditions | `eda/src/loops.rs`, `cli/src/main.rs` `power_gate` | tier 1 (§5.1–5.3 cases) and tier 2 (the fixture passes; a moved part fails) |
| Classes: `VBAT` admitted at step 13a with a tree; SW nets at their output's class | `ir/src/lib.rs`, `eda/src/stack.rs` | tier 1 |
| `Stack::of`: thermal arrays and J1's solid join | `stack.rs`, `ir/src/power.rs` `thermal_arrays` | tier 1 (24 drills of 0.254 mm; 4 solid joins) |
| A circle pad reached in its inscribed square | `route.rs` `Pad::reach` | the fixture routes; byte identity |
| `fp_arc` measured by the placer and read by `extract` | `pcb.rs`, `extract.rs` | eda unit test; extract passes |
| `fab`: one tool per drill, a hit count per tool, plated through-holes, FAB-NOTES naming every drill, each thermal array and the power conditions | `eda/src/fab.rs`, `cli/src/fab.rs` | tier 1 against KiCad's pinned drill files; tier 2 `kicad_writes_the_pinned_drill_files`; via-only notes byte-identical (goldens) |
| ADC channels; the sense pin solved (`NoAdcPin`) | `ir/src/lib.rs`, `solve.rs`, `svd-hal` (D1) | `sense_pin.rs`, `adc_channels.rs` |
| K6: the output delay in the sensor table | `ir/src/parts.rs`, `power.rs`, `parts.json` | `ir` tests |
| D6: `TranslatorPinUnbounded`; D7: the crossing pinned | `extract.rs`; tests | tier 1 |
| `campaign` refuses a tree as `PowerNotSimulated` (battery-sag work order) | `cli/src/campaign.rs` | `campaign_tier1` |

## 7. Predictions against the results

| id | result | label | verdict |
|---|---|---|---|
| P1 | Every file vendored byte-equal to GitLab's blob. Two existing rules refused one file each: the XT30PW-M's `fp_arc` silkscreen, now measured (K16), and the ICM-20948's keepout clearance (D2). The unnumbered pads (the HSOP's mask and paste apertures, the XT30's plated pegs) are read as predicted at risk, and accepted. | measured | partly |
| P2 | One ERC finding first (`pin_not_connected` on `VDD_1V8`'s flag-row power symbol, once its flag was dropped), then 0. No waiver. | measured | right |
| P3 | Rail-only parts landed by the wrong IC. So did label-net parts: the tie went to the MCU. | measured | half wrong |
| P4 | Every bound met with anchors except the bulk Cin (3.0–4.7 mm) and L1 (3.5 mm). Those bounds were set at 5.0 and 4.0 mm, after the measurement (D3, ruled). | measured | partly |
| P5 | The first full route failed at J1's circle pad (K13); with classes declared, it routed. `VBAT`, `VDD_5V` and U10's SW are at 0.5 mm; `VDD_3V3` and U11's SW at 0.2 mm. | measured | right |
| P6 | 24 vias of 0.254 mm into B.Cu; J1's four through-holes (two pins, two plated pegs) are solid; DRC passes. | measured | right |
| P7 | Tools per drill measured in the image (sub-agent table, §8). Existing packages are unchanged (goldens). The power board's drill file equals `fab::drills` (adversarial pass, measured). | measured | right |
| P8 | A table extension. The serialized pin table's test digest moved (test-only; its old value still holds with the field cleared). No topology or verdict of an existing specification moved. | measured | partly |
| P9 | 95 ns at 4 MHz, after #143. The crossing's board waits on D7. | measured | right for K6 |
| P10 | Every existing board's schematic, board and BOM, and every verdict, is byte-identical with `main`'s, measured after each merge. The exception is `imu-baro-1v8.json`, which has no board: its `UnknownPart` refusal text now lists ICM20948 among the known parts. | measured | right |
| P11 | +2 tier-2 tests, so TIER2_MIN is `main`'s 114 + 2 = 116. | read | right |
| P12 | CI: §11. | — | — |
| P13 | About 4,700 lines of code, tests and fixtures, plus about 3,900 of vendored KiCad files. | measured | wrong (larger) |

## 8. Adversarial pass (full; a fresh Opus subagent, frozen at `065be44`)

The reviewer was given the brief (`~/.cache/aet-4300d2/adversarial-brief.md`), the code and the sources, not these conclusions. Its record is `~/.cache/aet-4300d2-adv/adversarial.md`.

**What held** [measured by the reviewer]:
- Byte identity on all 20 fixtures.
- §5.1: each of the 17 loop parts moved off fails its own rule and no other. 10 µm past its bound fails, and so does a swap of two parts.
- §5.2: a K5 swap is caught by K5, the loop rules and parity.
- §5.4: drill files with a missing tool, a hit off by one, a swapped diameter or an unplated peg are refused. The power board's real drill file equals `fab::drills`.
- ERC passes with 0 findings, legitimately. Pin roles are against the datasheets.

| id | sev | finding | disposition |
|---|---|---|---|
| A1 | high | `check drc --spec` did not run `route::check` on a power tree's board, so a short or misplaced thermal array passed DRC (only `extract` refused it) | **fixed**: `power_gate` runs `route::check`, tested (`a_starved_thermal_array_is_refused`) |
| A2 | medium | several datasheet layout items the gate does not judge were not named | **fixed**: the conditions name SNVSAN3F 10.1 items 2, 3, 7 and 8, SBVS320D 7.4.1, and the ground side of one-pad rules |
| A3 | low | `fab::drills` takes an array's count from the board | **covered by A1**: such a board now fails DRC before `fab` |
| A4 | low | the drill check compares hit counts, not positions | **named** (fork F-fab-3: counts, not positions) |
| A5 | low | the Power-tree section's opening said "did not judge" over lines that are not unjudged items | **fixed** wording |
| A6 | low | FEEDBACK stale (K13's name, D5's claim, §2's gaps against the final verdict) | **fixed** K13 and D5; §2 is the measurement as taken, and §7 gives the final values |
| A7 | low | U11's SW and `VDD_3V3` at 0.2 mm for 679 mA | **named** (the CLASSES condition; K11 for the architect) |
| A8 | low | `filter_map`/`continue` that would drop a malformed row (unreachable after `validate`) | **named**: each sits behind a validation that refuses the row first |
| A9 | low | the crossing's drawing was unexercised | **fixed after D2 (a) and K6**: the committed fixture is drawn, and a tier-1 test pins its refusals |
| A10 | info | J1's polarity rests on the footprint's silkscreen | **named** here: KiCad's `+` beside pad 2 [read]; AMASS's drawing is not held |

## 9. Mutation pass (diff-sized)

`~/.cache/aet-4300d2/tools/mutate.py` ran one mutant per guard this build added or changed, each restored from memory, never by `git checkout`. Results are in `~/.cache/aet-4300d2/mut/`.
- **Round 1:** 21 mutants, 19 killed.
- **Two survived:** the battery name's claim and the sense pin's MCU pad. A test was added for each, and both were re-run killed (round 2).
- **The fab sub-agent's own mutants:** 12, killed, after one guard was removed as implied and one test added.
- **Guards added since:** the keepout reader, router and judge, D6's refusal, K6's pairing and range, and A1's call. Each is held by a named tier-1 test (`a_keepout_is_read_routed_around_and_judged`, `the_reader_admits_a_footprints_keepout_clearance_and_nothing_more`, `the_crossing_is_drawn_and_its_limits_are_refused_by_name`, `a_starved_thermal_array_is_refused`, `an_spi_output_delay_and_its_citation_come_together`).
- **Round 3**, on the guards added after the ruling: the keepout judge, the reader's keepout-only and `connect_pads`-content conditions, the not-a-keepout refusal, D6's refusal and K6's I2C refusal were all killed.
  - The router's keepout blocking survived. Its new test (`the_router_goes_around_a_keepout`) found a real bug: the blocking sat after a `continue` that skips parts with no pads.
  - The bug is fixed, and the mutant was re-run killed.
- **Round 4**, after the reduction: the two mutants whose code it touched (`gap radii`, `sense pin drawn`) were re-run killed.

## 10. Reduction pass (diff)

A fresh subagent proposed the cuts against the four questions; I applied those that remove a copy or a dead path at low risk, then re-ran the suite and the touched mutants.

**Reduction pass (diff): −22 net · 4 duplicates removed · 3 dead paths removed · 4 protected kept · 9 findings below.**
- **Duplicates removed:**
  - one rounded-up integer square root (`pcb::isqrt_up`) for the arc's circle and the loop gap;
  - `loops::gap_nm` reuses `route::gap2` and `Pad::inner` instead of re-deriving both;
  - the sense pin's package position is read once, from `SolvedSense.position` (`power::draw` loses its table);
  - `BATTERY_NET` is imported from `ir` where used, not re-exported twice.
- **Dead paths removed:**
  - `board_tier2`'s `v_spec`;
  - a discarded binding in `power_board_tier1`;
  - `thermal_layers`' needless `pub`.
- **Protected, labelled `belt-and-suspenders:` at the site:**
  - the power row lookup (`PowerPartUnknown`);
  - the buck with no divider;
  - the `thermal_arrays` skips;
  - the refusal to turn a footprint carrying a keepout (labelled in this pass).
- **Also fixed:** two comments in `main.rs` displaced by the power-gate insertion.
- **Findings (for the architect, not cut):**
  1. `loops::LoopRule` mirrors `schematic::Anchor` field for field. Collapsing them widens `Anchor` to `pub`.
  2. The line-to-net mapping (SCK, MOSI, MISO, CS to the bus's nets) is written in both `schematic.rs` and `schematic_power.rs::crossing`.
  3. `crossing` looks up again the channel that `lines` already found.
  4. `SolvedSense.channels` is read only by tests, a topology field kept for a future consumer.
  5. `fab::holes`'s two tool-mismatch messages could share one format string; that would change asserted wording.
  6. `check_kicad` holds the specification twice: `read_spec_value` and `declared`.
  7. The `notes-before-4300d2-four-layer-power.md` golden is the four-layer one plus three edits. It is kept as byte-identity evidence.
  8. Route's diff-pair path walk still tests a track end with `inner().contains`, while connectivity and the router use `reach()`. They agree on every pad drawn today.
  9. `Part.decouples` and `Part.anchor` are two mechanisms for "this part belongs to that IC".
