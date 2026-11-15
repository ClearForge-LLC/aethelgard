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
