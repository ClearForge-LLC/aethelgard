# AET-WO-4240a — predictions, written before any run (2026-11-20)

Base: `main` `d70a99e5`. Corpus: the 27 committed specifications (every `*.json` holding an `mcu` object and a
`schema_version`, outside `spike/`, `fab/`, `docs/`, `target/`) and the 5 stored topologies (`schema_version`,
`assignments`, `specification_sha256`). That is `4240`'s 31 fixtures plus `examples/sentinel-02-ext-bmp581.json`
(`3403`, added after `4240`).

## What a "solved word" is, and the mask

**Measured per specification** (harness `kit/harness`, library level, and the CLI's `validate` and `solve` stdout
with exit code):
- `Specification::validate`'s result;
- `solve`'s whole `SolvedTopology`, serialized: every pin, `PSC`/`ARR`/`CCR`, `BRR`, `MBR`, `TIMINGR`, `CR1`,
  kernel clock, speed, the clock and everything else in it;
- the refusal, for the four refused by design;
- `SolvedTopology::validate` on it;
- every timing verdict on it: `motor::check_silicon` (DShot/PWM `holds`, its bands), `serial::check_silicon`
  (margins), `silicon::check_spi`, `silicon::check_i2c` and `vos::judge_generated`, as their `Debug` text.

**Measured per stored topology:** parse, `validate`, `vos::judge_generated`.

**The mask, declared before the runs.** Stage A (the refactor: one budget, F-PANIC, no schema change) is
compared **with no mask**: byte for byte. Stage B (the schema) may differ in exactly:
- the top-level `"schema_version"` of a solved topology (12 → 13);
- `"specification_sha256"`, because a specification's version line moves (19 → 20);
- the specification's own version in a refusal that names it (`UnsupportedSchemaVersion` text), if any.

Anything else that differs is the §7 stop.

## Predictions

- **P1.** Stage A: all 27 specifications and 5 topologies reproduce byte for byte, every verdict included.
- **P2.** Stage B: under the mask, all 27 and all 5 reproduce. No `hse: null` solved word moves; no verdict
  text moves (the serial verdict keeps its field name `hsi_ppm`).
- **P3.** Refusals by design stay as on `main`: 3 `Infeasible` (`solve/infeasible-shared`, `pigeonhole-spi`,
  `thrash-pins`) and 1 `ZeroNominalVoltage` (`examples/invalid-zero-domain.json`).
- **P4.** The 16 MHz crystal (`ABM8AIG-16.000MHZ-12-2Z-T3`) solves to `4240` §4.5's H1 words:
  `PLLCKSELR` `0x0000_0052`, `PLLCFGR` `0x0003_0004`, `PLL1DIVR` `0x0103_064F` (reference 3.2 MHz, VCO 256 MHz
  wide, `pll1_p` = `pll1_q` = 64 MHz).
- **P5.** An 8 MHz external clock solves to `DIVM1` 5, `DIVN1` 159, `DIVP1` 3: `PLLCKSELR` `0x0000_0052`,
  `PLLCFGR` `0x0003_0002` (`PLL1RGE` `00`, `PLL1VCOSEL` 1), `PLL1DIVR` `0x0103_069F`.
- **P6. The selection rule** that gives P4 and P5: the least `DIVM1` whose reference is a whole number of Hz
  strictly inside one `PLL1RGE` range; `PLL1VCOSEL` medium for a reference of at most 2 MHz, else wide; the
  least even `DIVP1` for which `DIVN1` = 64 MHz × `DIVP1` / reference is whole, in 4–512, and the VCO is in
  the selected range; `DIVQ1` = `DIVP1`; `DIVR1` left at reset (1) with `DIVR1EN` 0; `DIVM2` = `DIVM3` = 0 and
  every PLL2/PLL3 enable cleared.
- **P7.** An external 25 MHz/3 (8.33 MHz, `4240` F-HZ) is refused by name: no whole-Hz reference reaches
  exactly 64 MHz (`4240` P3: 0 settings).
- **P8.** The ABM8AIG-16 budget is ±(20 + 50 + 2 + pull) ppm per side, each term cited to ABM8AIG "REVISED
  06-13-19" pp. 1–2 (tolerance option `2`, stability option `Z`, aging first year, temperature option blank
  −40…+125 °C).
- **P9.** `Gm_crit` for it is 0.5546 mA/V (ESR 70 Ω, C0 2.0 pF, CL 12 pF, 16 MHz), under DS12117's 1.5 mA/V;
  AN2867's margin, shown uncited, 2.70×.
- **P10.** CRSF 416666 at 64 MHz on a ±102 ppm crystal budget (pull 30) holds with a margin near 22,600 ppm
  (`4240` M2: 22,652 at ±50, slope about 1).
- **P11.** Reserving PH0/PH1 moves no pin of a `null` board (their set is unchanged); `4240` M6 found no
  fixture places anything there.
- **P12.** F-PANIC: an 8 MHz H0 clock gives a named refusal at `FastPlus1M` (it needs a kernel of at least
  17 MHz at its slowest), never a panic.
- **P13.** Emitted bytes: none move but the "Solved topology of specification sha256" lines (version line).
  The bench ELF does not move. The bench package's `FAB-NOTES.md` and `MANIFEST.json` move by the version
  line only.
