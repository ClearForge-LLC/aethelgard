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
  - `Pad::terminal_box` falls back to the inscribed square only where the inner box holds no grid node, so no existing pad's terminals move.
  - The connectivity judge now asks whether a track end lies in the copper (`fits`, rounding included), not in the inner box.
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
