# AET-WO-1216b — predictions, written before any run

Written 2026-11-22 ~08:40Z at `c9d4096b` (main), from the work order, `1216`'s FEEDBACK and ruling, `spike/1216/` and a read of the crates. Nothing had been built or run on this branch.

## P1. The oracle, re-run on `c9d4096b` (after `1211f`)

1. `spike/1216/universe.sh` still lists **34** boards: `1211f` added no committed specification outside `spike/`.
2. Walk 1 (`spec.py`) gives **byte-identical** `out/spec/*.tsv` to the committed record on all 34. `1211f` added D1's `bulk` (C2), which walk 1 does not read.
3. `judge.py`'s `lean` refuses **1 / 34**: D1, `DRV_IPROPI` 4665 mV > 4000 mV.
4. F7, D1 drawn (`spike/1211f/d1-drawn.json`): walk 2 lists IPROPI through R2 to the MCU pin, and the four U2 rows the 1216 reviewer named (EN, PH, nSLEEP, nFAULT). Classified by the pin row's kind, none is a source: EN and PH are `logic_input`, nSLEEP is `enable_input`, nFAULT is `open_drain_output`. Walk 1 and walk 2 then agree on every D1 row.

## P2. The walker, before the fixture edit

1. **Path for path:** its rows equal walk 1's on all 34 boards by net, class, rail, whether the rail can lead, and the worst millivolts. The walker runs on the specification alone, so it has no solved pin; walk 1's pin column is the one column not compared.
2. **Refused by the new step, before the edit:** D1 alone, `OffDomainAboveLimit` on `DRV_IPROPI`, **4665 mV** against **4000 mV**, its figure named as the DRV8876 row's recommended-operating 3 mA (SLVSDS7B p. 4), not a maximum.
3. D2 and `invalid-zero-domain` keep their kinds (`PartLimitUndeclared`, `ZeroNominalVoltage`): the step runs last.
4. **Exempt by the rule on committed boards:** only `sentinel-02-power`'s `VDD_1V8` (downstream of `VDD_3V3`). No committed board has an EN wired to a PG, and no connector carries `VDD_1V8`. No committed path lands on BOOT0 from a stated source: the 9.0 V limit is reached only by test.
5. **Unbounded paths, named:** every connector pin on an MCU net, test point on an MCU net, and foreign clock, on 33 boards; never in `validate`'s output.

## P3. After the fixture edit (R2 1.07 kΩ)

1. The engine's figure: 3 mA × R2's high band. With D1's −20 to 85 °C, CRCW0603 at 1 % + 100 ppm/K × 65 K + 2 % drift = 36 500 ppm, R2 high is 1109.06 Ω: **3328 mV**, refused by nothing. **0 / 34** refused by the new step.
2. At the 3.5 A peak on the typical 1000 µA/A ratio: 3.5 mA × 1109.06 Ω = **3882 mV**, under 4.0 V. This is not an engine figure.
3. A2: full scale 1900 mA at AERR's worst gives at most about 2.0 mA, about 2.2 V across 1109 Ω, against VREF's minimum of 3135 mV. It still passes.
4. Some tests outside `aethelgard-ir` that read D1 assert a value derived from R2 (an ADC code, a transfer). I expect each to follow from the solved topology and need no edit. Any literal that does move I name, and I do not edit it outside the fence.

## P4. Specification 25

1. Every committed specification other than D1 **validates, solves and emits byte-identically except its version line and each sha that embeds it** (the null sweep). D1's outputs move only by R2.
2. `fab/sentinel-01-bench/package`'s `FAB-NOTES.md` and `MANIFEST.json` move by their sha lines (as in `1211e`), and its verdict moves by the new `twin_limits` rows. So the package needs a write=true dispatch: AWAITING-PACKAGE.
3. The solved topology's version does not move.
