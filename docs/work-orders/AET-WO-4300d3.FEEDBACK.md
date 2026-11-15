# AET-WO-4300d3 — FEEDBACK (build, with one measured stop: §1.0, a held source for the buck's junction)

**Branch:** `wo/aet-4300d3-judgements` from `origin/main` `dbf049e` · **Work order:** `docs/work-orders/AET-WO-4300d3.md` · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace, with subagents · **Cadence:** build, with a full adversarial pass.

Each answer is labelled **measured**, **read** (at source), or **inferred**. The predictions in §1 were committed before any new check was run. No prediction is edited after that commit. The only runs before it were:
- ghostscript text extraction and page renders of held datasheets, in the pinned tools image through `~/bin/capped-run`;
- the byte-identity baseline: `main`'s binary on every example and fixture (`~/.cache/aet-4300d3/base/`, 91 files across 19 specifications).

## 0. Readings taken before the predictions

### §1.0, the measured stop: the buck's junction [read]
- **SNVSB89 is not an LMR33630 document.** `ti.com/lit/pdf/snvsb89` serves SNVSB89A: the LM25180-Q1 PSR flyback datasheet (Nov 2018, rev. July 2019; sha256 `4cad6041…`). The pp. 23 and 32 material the work order means is in SNVSAN3F, the LMR33630 datasheet already held (sha256 `3b0920a4…`).
  - p. 23, 9.2.2.6: "it is desirable to use an electrolytic capacitor on the input in parallel with the ceramics… The moderate ESR of this capacitor can help damp any ringing on the input supply caused by the long power leads."
  - p. 32, Power Supply Recommendations: "an aluminum or tantalum input capacitor in parallel with the ceramics… A value in the range of 20 µF to 100 µF is usually sufficient to provide input damping." The same page says a snap-back (thyristor-type) TVS "is not recommended".
  - p. 6, 7.1: VIN to PGND absolute maximum is 38 V.
- **TI states no θ or Ψ for design.**
  - p. 7, 7.4: RθJA 42.9 °C/W (DDA) is "only valid for comparison… can not be used for design purposes" (JESD 51-7, simulated 4-layer JEDEC board). ψJB is 13.8 °C/W and ψJT 4.3 °C/W, as characterization parameters.
  - p. 24, 9.2.2.11, Equation 11: T_J = T_A + P_loss · RθJA, where "the effective RθJA of the device and PCB combination" is "more difficult to estimate".
  - p. 25, **Figure 9-3, "Typical RθJA versus Copper Area for a Four-Layer Board and the HSOIC (DDA) Package"**: "the copper area given in the graph is for each layer; the top and bottom layers are 2 oz. copper each, while the inner layers are 1 oz… for illustration purposes only." Read from the 300 dpi render, in °C/W at cm² per layer: 41.7 at 6.5 (where the curve starts), about 35.5 at 10, 29.5 at 15, 27.7 at 20, 26.7 at 30, 26.1 at 40, 25.3 at 50, and 24.5 at 58 (where it ends).
  - Figure 9-4 is the RNX (VQFN) package, a different curve.
- **The EVM guide (SNVU573A, held) states no θ, Ψ or thermal figure** [measured: no match for thermal, °C/W, θ or junction in its text].
- **Efficiency** (p. 27, Figures 9-7 and 9-8; DDA, 400 kHz, TA 25 °C, typical). Read from the render:
  - 5 V out at 2 A: 92.5 % at 24 V in, 90.5 % at 36 V in.
  - 3.3 V out at 0.68 A: about 85 % at 24 V in, 82 % at 36 V in.
- **So a held source exists (Figure 9-3), typical and conditioned. §1.0 does not stop.** The fixture's board is outside that figure's condition (D1).

### Hot-plug parts [read; fetched by a subagent, spot-checked at source]
- **Bulk capacitor: Nichicon UCZ** (catalog "UCZ Chip Type, High Reliability", Jun. 2024; sha256 `7ecc9608…`, `~/.cache/aet-ds/`).
  - Row `UCZ1H470M□□1GS`: 47 µF ±20 %, 50 V, 8×10 mm, -40 to +125 °C.
  - ESR maximum is 0.25 Ω at 20 °C and 100 kHz, and 3.50 Ω at -40 °C. The ESR at the board's -20 °C floor is not printed.
  - Caveats: the full part string `UCZ1H470MCL1GS` is inferred from the catalog's numbering example. No lifecycle status could be read (Nichicon's product page is a script-only app). The catalog carries no document number.
- **TVS: Littelfuse SMBJ** (06/03/20 revision; Littelfuse's own site refused the download, so the copy is from a TI E2E mirror).
  - **No SMBJ row has a stand-off above 26.1 V and a clamp at or below 38 V.**
    - SMBJ26A: VRWM 26.0 V, VC 42.1 V.
    - SMBJ28A: VRWM 28.0 V, VC 45.4 V.
  - The clamp path is therefore proved with a test-only table, not a pinned row.

### DC bias [read]
- **No held capacitor document gives a DC-bias figure** [measured]. Murata's reference sheets give only "Temperature Characteristics… No bias". Murata's chart for GCM32EC71H106KA03 is being fetched by the operator. So **§7's second stop condition is true today**: the declared derating is the only source for every capacitor.
- **The derating sources held:**
  - SBVS320D (TLV755P) p. 15: "Generally, derate ceramic capacitors by 50%". Its p. 4 note 1 adds: "Throughout this document the nominal derating on these capacitors is 50%. Make sure the effective capacitance at the pin is greater than 0.47µF". So **TLV755's 0.47 µF is an effective minimum, read at source.**
  - SNVSAN3F p. 22, in the 5 V design example: "a 10% bias de-rating" for 16 V 1210 ceramics at 5 V. **The two sources disagree** (10 % against 50 %). That disagreement is a finding, not something to average.
- **SNVSAN3F's minima.**
  - p. 19 Note: "the effective value of capacitance is defined as the actual capacitance under D.C. bias and temperature… ensure that the minimum value of effective capacitance is provided."
  - p. 23: "A minimum of 10 µF of ceramic capacitance is required on the input". This sentence does not say "effective".
  - p. 20, Table 9-2 lists "COUT (RATED CAPACITANCE)" 4 × 22 µF and "CIN + CHF 10 µF + 220 nF". The EVM uses one 10 µF 50 V part at up to 36 V.
  - **Whether the 10 µF minimum is effective is inferred, not read (D2).**

### The brown-out threshold [read]
- DS12117 p. 212, Table 115 gives VBOR rising maximum as 1.71, 2.15, 2.47 and 2.78 V for levels 0 to 3 (falling maximum 1.68, 2.06, 2.37, 2.68 V). Rising BOR0 carries note 1.

### Control pins [read]
- SNVSAN3F p. 5, pin table:
  - EN: "Do not float."
  - PG: "Open drain power-good flag output… Can be left open when not used."
- The emitter ties a buck's EN and an LDO's EN to their input, leaves PG unconnected, and ties OE to VCCA (`eda/src/schematic_power.rs:402-403`, `:481`, `:531`).

### The 3.3 V divider [measured, `main`'s solve]
- R20/R21 = 105 k / 46.4 k gives a guaranteed output of 3144.7 to 3417.2 mV. The domain is 3135 to 3465 mV.
- That leaves 9.7 mV (0.29 %) of margin at the low edge and 47.8 mV (1.45 %) at the high edge.

## 1. Predictions (committed before any new check runs)

| id | prediction | label |
|---|---|---|
| P1 | **Junction (U10, 5 V, 2000 mA).** P_out ≤ 5.237 V × 2 A = 10.47 W. η = 90 % (the 36 V curve at 2 A, which brackets 26.1 V from below) gives P_loss ≈ 1.16 W. At Figure 9-3's 20 cm² point (28.0 °C/W rounded up) the screen gives T_J ≈ 85 + 32.8 = 117.8 °C, 7 K under 125 °C. At 10 cm² (36 °C/W) it is about 127 °C, which is refused. | inferred |
| P2 | **Junction (U11, 3.3 V, 679 mA).** P_out ≤ 2.32 W. η = 81 % gives P_loss ≈ 0.55 W, so T_J ≈ 100 °C at 20 cm². | inferred |
| P3 | **DC bias.** At the declared 50 % derating, the TLV755's cin and cout each hold 0.50 µF effective against 0.47 µF, a margin of 1.06 times. If the buck's 10 µF were effective (D2), U10's and U11's single 10 µF 50 V cin would hold 5 µF and be refused. | inferred |
| P4 | **BOR rising.** VDD_3V3's minimum of 3135 mV clears level 3's 2780 mV rising maximum by 355 mV, so the fixture passes. A domain floor between 2680 and 2780 mV is refused `SupplyFloorBelowBorRising`. | inferred |
| P5 | **Hot-plug.** One UCZ 47 µF 50 V part on the board side of J1 passes for U10 and U11. Each of these is refused `BatteryInputUndamped`: a rating at or below 26.1 V, 19 µF, a pack-side part, a test-table TVS clamping at 45.4 V, and no part at all. | inferred |
| P6 | **Byte identity.** Every board without a power tree is byte-identical apart from specification-hash lines. Boards with a power tree are byte-identical too, because their declarations are not drawn. The verdicts of power-tree specifications change by their new conditions only. | inferred |
| P7 | **Tier 2** gains no test, so `TIER2_MIN` stays at 129. The fixture's existing tier-2 tests keep passing with the declarations added. | inferred |
| P8 | **Size:** 1,200–2,000 lines, almost all in `aethelgard-ir` (power.rs, power_parts.rs, the table, tests). | inferred |

## 2. Decisions for the architect (open)

- **D1 · The fixture's board is outside Figure 9-3's condition.** `sentinel-02-power.json` declares 2 copper layers. The only held θ is for a four-layer 2/1/1/2 oz board.
  - **Default taken** (it keeps every board byte-identical): use the figure and name in the verdict that the board is outside its condition, with the layer count.
  - Options:
    - (a) the default;
    - (b) refuse below four layers, and make the fixture four-layer, which changes its tier-2 board;
    - (c) a declared θ with its source, as §1.0's stop offered.
- **D2 · Is SNVSAN3F's 10 µF input minimum effective or rated?** Reading it as effective is inferred, not read (see §0).
  - **Default taken:** no effective minimum on the buck's cin and cout rows; the rated checks stay. The verdict names the shortfall each would have under the declared derating.
  - Options:
    - (a) the default;
    - (b) effective. Then the fixture fails at its single 10 µF (P3), and each buck needs a second or third 10 µF, which changes the power boards;
    - (c) effective, with the screen waiting on Murata's chart.
- **D3 · PG left open.** §1.5 offers "wired" or "tied by a strap"; the emitter leaves PG open.
  - **Default taken:** `open` is a strap a row may allow, by citation (SNVSAN3F p. 5: "Can be left open when not used"). EN may not be open.
- **D4 · The 3.3 V divider is kept.** 1 % inside at both edges needs 66 mV of slack, and the band leaves 57.5 mV.
  - A balanced pair would put each edge about 0.87 % inside.
  - Any new pair changes the power boards' BOM and value strings, which §2's byte identity forbids.
  - **Open:** re-pick the pair if the architect prefers margin to byte identity.
- **D5 · Declared hot-plug parts and control straps are not drawn.** `eda` is outside this fence. So the emitted board lacks the bulk capacitor this tree now requires, and the straps are drawn by the emitter's own rule.
  - Named as a condition in the verdict.
  - Proposed backlog row: the emitter draws `power.input.hot_plug` and reads `controls`. Owner: 4300g or the next eda work order.
