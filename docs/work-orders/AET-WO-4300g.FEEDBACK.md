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
