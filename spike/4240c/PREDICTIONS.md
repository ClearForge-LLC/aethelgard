# AET-WO-4240c — predictions, written before any run (2026-11-20)

Base: `main` `8df353ee`. **Corpus:** the 29 committed specifications: every tracked `*.json` holding an `mcu`
object and a `schema_version`, outside `spike/`, `fab/`, `docs/`, `target/`, less the 5 stored topologies and
`output-speeds.json` (list: `kit/specs.txt`). Every one declares `mcu.hse` `null` (or `hse_hz: 0`).

**What is compared:** `aethelgard emit --file S --out D` (`kit/emit-sweep.sh`): its stdout, its exit code, and
the sha256 of each file it writes (`<project>.kicad_sch`, `<project>.bom.csv`, `<project>.kicad_pcb`). The
baseline is a `git archive` of `8df353ee` built apart; the branch is re-swept after each commit that could move
an emitted byte (vendoring, the parts rows, the emitter, the router, the rules).

**The mask: none.** Byte for byte, stdout included.

**Outside the corpus, and predicted to move:** the gate-written `fp-lib-table` that `check erc`/`check drc`
write into their own scratch (`library::tables`) gains one row, `Crystal`, for every board, because a
vendored library is named whether or not a board uses it. It is not an emitted file. **Prediction:** ERC and
DRC verdicts on the null boards do not move (checked on `sentinel-01-minimal`, `sentinel-01`,
`sentinel-02-bench` in the tools image).

## Predictions

- **P1.** All 29 specifications: emit stdout, exit codes and every written file are byte-identical between
  the baseline and the branch, after every commit. The specifications that `emit` refuses on `main` refuse the
  same way, by the same kind and text.
- **P2.** Vendoring the two files moves no emitted byte; `PINS` and `FILES` go from 52 to 54.
- **P3.** Each vendored file's git blob sha equals GitLab's files-API `blob_id` at ref `10.0.7`:
  `Device.kicad_symdir/Crystal_GND24.kicad_sym` `97e45475d4465df4facb473b25bb69962c8db463`,
  `Crystal.pretty/Crystal_SMD_3225-4Pin_3.2x2.5mm.kicad_mod` `11874e166802d0400169fbbbf26757304810e09d`
  (fetched 2026-11-20 ~16:08Z: both equal).
- **P4.** The crystal row's symbol id `Crystal:Crystal_GND24` does not exist upstream; the symbol is
  `Device:Crystal_GND24` (the `Crystal.kicad_symdir` path is a 404 at `10.0.7`).
- **P5.** A 16 MHz crystal fixture on the real part (`STM32H753ZITx`) emits at library level (the CLI refuses it
  `HseNotGenerated` until `4240b`), routes with no via on either oscillator net, and passes ERC and DRC; the
  crystal lands within a few millimetres of PH0/PH1 (LQFP-144 pins 23 and 24).
- **P6.** `Cs` cannot be bounded on both sides from held sources: `extract` bounds each oscillator net's copper
  from above only, and DS12117 gives the OSC pins' capacitance as `CIO` 5 pF **typical** with no minimum or
  maximum (Table 49 p. 137 rev Y; Table 146 p. 241 rev V). So the §1.4 stop fires before any load judgement is
  built; the report gives each net's copper bound and the pin figure, labelled typical.
- **P7.** Each oscillator net's copper bound, as routed, is under 1 pF (a few millimetres of 0.2 mm track and
  three pads on a 2-layer board).
- **P8.** With `Cs` = 5 pF (pin, typical) + the copper bound, `2(CL − Cs)` for CL 12 pF is about 12 pF, so
  `4240a`'s declared 18 pF pair (series 9 pF) would load the crystal at about 15 pF: a mismatch, reported as a
  finding, not changed.
