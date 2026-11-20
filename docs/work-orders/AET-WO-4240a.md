# AET-WO-4240a — The crystal in the specification: `mcu.hse`, its ppm budget, and a clock-tree judge, with no firmware yet

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-4240a-hse-ir`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass.
**Phase:** P4 (Sentinel-02's clock). Written 2026-11-20. **Cadence:** a build with one measured stop. **After `4240v`** (#259, merged). **Takes the next free specification version** (20 or 21; see §3).

**Grounds** (`AET-WO-4240` FEEDBACK D1–D5, D8, D10, ruled on #240, comment 7014986899; roadmap §5 and §7.1):
- **The tree is ruled.** H1 is HSE → PLL1 → 64 MHz `sys_ck`, every prescaler 1, plus VOS2 (`4240v`, merged).
  - The crystal is Scotty's 16 MHz 3225, CL ≤ 12 pF, `ABM8AIG-16.000MHZ-12-2Z-T3`. It is **provisional** until AN2867's current revision is held: only Rev 10 is (F-AN).
- **The split puts the IR first.** The order is `4240a` (IR), then `4240b` (generator and twin), then `4240c` (eda), then a bypass bench before `4500`.
- **What the IR has today:**
  - `mcu.hse_hz: u32`, refused unless 0 (`CrystalNotSupported`, `ir/src/lib.rs` about L1634, L3297);
  - the HSI's ppm in three places (`motor.rs` `HSI_FAST_PPM`/`HSI_SLOW_PPM`, used by `serial.rs` and the timing checks);
  - a latent panic: `solve.rs` about L406, `i2c_timing(..).expect(…)` (`4240` F-PANIC). An 8 MHz H0 reaches it.

> **What this is**, as `4240` D3, D4 and D8 recommend and the split accepted:
> - **D4:** `mcu.hse`, an optional object replacing `mcu.hse_hz`, with `null` for none. Either `crystal` (a parts row, its load capacitors, the pull bound, the operating temperature range and the service life) or `external` (a rational frequency, `hz_num`/`hz_den`, and a cited ppm);
> - **D3:** one ppm-budget function in `aethelgard-ir`, `tol₂₅ + stab(T) + aging(life) + pull` per side. It replaces the IR's HSI copies, with the HSI as one source among others, and every timing check takes the board's source;
> - **D8:** the shared register decoder, plus a judge that reads `PLLCFGR` itself (`PLL1RGE`, `PLL1VCOSEL`, `DIVP1EN`) and judges the PLL's reference and VCO ranges strictly inside, DS12117's `Gm_crit_max` for a crystal, and every derived clock against `4240v`'s VOS limits;
> - the solver reserving PH0 and PH1 for a crystal, and PH0 for an external clock;
> - a crystal or external specification **solves**, and `generate`, `emit` and `simulate` refuse it by name, `HseNotGenerated`, until `4240b`;
> - F-PANIC as a named refusal.
>
> It is NOT:
> - the bring-up firmware, the twin's RCC rule, or Kani (`4240b`);
> - the footprint, routing or `Cs` from `extract` (`4240c`);
> - any change to a specification that declares no HSE. Their solved words must not move.

## 1. Build

1. **Measure first,** in `spike/4240a/`, with predictions committed before the runs.
   - On `main`, solve all 31 of `4240`'s fixtures and every committed specification. Record each solved word.
   - The branch must reproduce every one byte for byte for a specification with `mcu.hse` = `null`.
   - **Stop and report** if any moves.
2. **The schema.**
   - `mcu.hse` replaces `mcu.hse_hz`. A version-19 file's `hse_hz: 0` reads as `null`, and a non-zero value was always refused.
   - Every committed specification is renumbered by the repository's script, version lines only.
   - `SOLVED_SCHEMA_VERSION` moves only if `SolvedClock` gains fields. Say which.
3. **The parts row.**
   - The crystal's row: `ABM8AIG`, 16 MHz, CL 12 pF, with tolerance, stability, aging, ESR max and `C0` from its datasheet, each cited. The datasheet is held in `~/.cache/aet-4240/ds/` (`spike/4240/ds/SHA256SUMS`).
   - **The load capacitors:** `4240` picked no C0G part, because `Cs` comes from `extract` (`4240c`). Choose one, and say which:
     - (a) `load_caps` declared by value, with `4240c` adding the part;
     - (b) a C0G row picked now with its value from a stated `Cs`.
     - Neither may present an uncited number as cited.
4. **The budget.** One function, cited per term.
   - The operating temperature range is a new declaration, checked to lie inside the part's.
   - The pull is declared, because no held datasheet states pullability.
   - In external mode, the declared ppm is used alone, with its citation.
   - Every check that took the HSI's ppm takes the board's source's: serial, DShot and PWM `holds`, and I2C `TIMINGR`. For `null` it is the HSI, as today.
5. **The judge** (`4240` §4.2 and D8), in `aethelgard-ir`:
   - decode with the shared function;
   - read `PLLCFGR` directly;
   - refuse by name: a reference on a range boundary (F-RGE: strictly inside); a VCO outside its `PLL1VCOSEL` range; `Gm_crit_max` exceeded; any clock over its VOS limit;
   - AN2867's oscillation margin is **reported uncited and judges nothing** until the current AN2867 is held. Show the 2.7× figure as information only.
   - `OscillatorNotCited` gets its first test.
6. **The solver.** PH0 and PH1 are reserved for a crystal, and PH0 alone for an external clock. A conflicting assignment is refused by name. Measure that no `hse: null` board's pins move.
7. **The refusals:** `HseNotGenerated` at `generate`, `emit` and `simulate`, and F-PANIC's `.expect` as a named refusal. 8 MHz H0 is the test.
8. **Tests:**
   - the 16 MHz crystal specification solves to `4240` §7's predicted words (`PLLCKSELR`, `PLLCFGR`, `PLL1DIVR`);
   - an 8 MHz external (`4240` D7 (c)) solves to `DIVM1` 5, `DIVN1` 159 and `DIVP1` 3;
   - each refusal is shown by name.
   - **Mutants:**
     - a reference on a boundary passes: a test fails;
     - the budget drops aging: a test fails;
     - the HSI's ppm is used for a crystal board: a test fails.

## 2. Acceptance

- **The measurement** is in FEEDBACK. No `hse: null` solved word moves.
- **The two HSE specifications solve** to the predicted words, and everything else above is shown.
- **No emitted byte or ELF moves,** and neither does the bench package. If its version line moves the package, say so; the architect dispatches.
- **The CLAUDE.md gates pass, and CI is green.** `TIER2_MIN` rises by your own tier-2 count.

## 3. Invariants and fences

- **Invariants:** `INV-01`, `INV-02`, `INV-06`, `INV-07` (the judge is not the generator; `RUL-20`).
- **Fence:**
  - `crates/aethelgard-ir/src` (`mcu.hse`, the budget, the decoder and judge, the solver's reservation, the refusals);
  - the parts table (the crystal row, and a C0G row if (b));
  - `crates/aethelgard-cli/src` (`HseNotGenerated` at `generate`, `emit` and `simulate` only);
  - `crates/aethelgard-hwdef/src`, the HSE citation only, if the schema change reaches it;
  - committed specifications (the version line only);
  - tests;
  - `spike/4240a/`;
  - FEEDBACK;
  - `fab/sentinel-01-bench/package/`, byte-for-byte commit only, if the version line moves it.
  - **Not:** `aethelgard-svd-hal`, the twin, eda, `boards/`, workflows, or `ci/` (except a weight row P1/P2 asks for).
- **The specification version.** `1210` (draws no board) also bumps it.
  - Whoever merges first takes 20; the second merges `main` and renumbers by the script to 21. Say on the bus which you took.
- **Running beside you:**
  - `1210`: the version, and IR `validate`;
  - `1209c`: IR `validate`, a new kind in the same enum;
  - `2306` (#269) and `2305` (#271): the package queue;
  - `4241`: firmware selection and the judge;
  - `4405`: pincheck.

  Merge `main` before READY.
- **One container of yours at a time,** under `capped-run`, at `nice -n 10`.
- One PR, left unmerged. Write `~/.cache/aet-4240a/pr-title-body.md`.

## 5. Adversarial pass, full

- **A crystal board that passes the judge but cannot start:** `Gm_crit` at its edge, or a `C0` the row omits.
- **A budget that undercounts:** temperature outside the part's range, life beyond aging's stated span, or a pull of 0.
- **A judge that agrees with the generator because it shares its code** (`RUL-20`).
- **A `null` board whose timing checks moved** because the HSI path was refactored.

## 7. Flag-and-stop conditions

- **A solved word of a `hse: null` specification moves.** Stop with the diff.
- **A term of the budget has no cited source** in the held datasheet. Stop and name it. Do not invent one.
- **The decoder cannot serve both `4400g`'s foreign cross-check and this judge** without changing the cross-check's results. Stop with the case.
