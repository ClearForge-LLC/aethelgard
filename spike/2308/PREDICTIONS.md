# AET-WO-2308 — predictions, written before any build or verify run

Written 2026-11-22 08:46Z on `wo/aet-2308-verify-backlog` at `c9d4096b`, before any change to a crate. Already done when this was written, and so not a prediction:
- **Read at source:** `eda/src/fab.rs` (`bom_rows`, `jlcpcb`, `jlcpcb_notes`), `cli/src/fab.rs` (`derive`), `cli/src/verify.rs` (the tail, `package`, the campaign gate), `cli/src/tree.rs` (`diff`), `cli/src/ledger.rs` (the HSE rows), `twin/src/hse.rs` (`ClockRule`, `judge_clock`), Renode's `STM32H7_RCC.cs` (the copy whose lines the ledger cites: L185 `HSERDY` and L186 `HSEBYP` match), and JLCPCB's sample `Sample-BOM_JLCSMT.xlsx` (sha256 `f6fa6b2c…`, 1604's copy) and its three help pages.
- **Measured:**
  - The base sweep (`spike/1211d/measure/sweep.sh`, the base release CLI): 34 specifications.
  - The noted BOM rows of every base `emit`. 23 of the 34 boards carry a note. **The only note is W25Q's symbol stand-in**, 41 rows.
  - A probe of `ledger::TABLE`: 28 `independent-rule` rows have a judge that names no `aethelgard_ir` rule. They are exactly the loop at `ledger.rs` L857–917.
  - D1 (`spike/1211f/d1-drawn.json`) under the base `verify --fab-out` with one baseline scenario: the first pre-gate refusal is `CampaignCoverageShort` (serial port, ADC input, rail). So D1 gets past validate, solve, the ledger and the harness check. Only scenarios stand between it and a package.

## What a stand-in is here (read)

There are three sources.

| Source | Example | Emit's BOM note | What a package prints today |
|---|---|---|---|
| A symbol that stands in for the ordered part (`schematic_arms.rs` `stand_in`) | W25Q64JVSSIQ on `Memory_Flash:W25Q32JVSS` (AET-WO-1300 D2): U4 of the bench package | carried | `.bom.csv` carries it. `-jlcpcb-bom.csv`, `FAB-NOTES.md`, `-pos.csv` and `-jlcpcb-cpl.csv` name U4 with no label |
| A declared bulk's source (`declared_bulk`) | D1's CVM2, "D1 fixture stand-in, not a design" | carried | as above, once a pin-device board is packaged |
| **A twin stand-in part** (`parts.json` `stand_in: true`, "never for a purchase", `parts.rs` L1280) | ICM20948 and ICP10111. ICM20948 has a drawable arm (`schematic_arms.rs` L262, `stand_in: None`) | **empty** | named only as LIFECYCLE EOL/NRND in the banner, never as a stand-in |

## Predictions

**P1 · Row 1.** Shape (a) for the first two sources and (b) for the third.
- The JLCPCB BOM gains a fifth column, `Note`, after the four sample columns. Each kind is grouped by (value, footprint, note).
  - `FAB-NOTES.md` gains a section listing every noted part: its reference, its value and its note.
  - A twin stand-in drawn on a board refuses `verify --fab-out` and `--against-fab` by name (`StandInInFabPackage`) before any gate. Emit's `.bom.csv` cannot carry its label, because the emitter is fenced.
- P1.1: in the bench package, `sentinel-01-bench-jlcpcb-bom.csv` moves on every line (14: the header and 13 kinds). U4's line carries W25Q's note.
- P1.2: `FAB-NOTES.md` moves by one new section, which lists U4, and by the JLCPCB section's sentences on the columns. `MANIFEST.json` moves by those two files' hashes and by the verdict line (P2.2).
- P1.3: no other package file moves. Every Gerber, the drill file, `-pos.csv`, `.bom.csv`, `-jlcpcb-cpl.csv` and the ELF stay byte-identical.
- P1.4: the base-vs-branch sweep (validate, solve, emit) is identical on all 34 specifications. No row touches emit.

**P2 · L3.**
- P2.1: the campaign gate's `tool_version` becomes the Renode version that every run's archive header names, equal to `simulate`'s `Renode v1.17.0`. Today it is `null`.
- P2.2: this moves the verdict line that `MANIFEST.json` embeds, so the package moves by row 2 as well.

**P3 · L4.** An empty directory in an `--against` directory, or in an `--against-fab` one, is reported as `against_extra` with a trailing `/`. Today neither reports anything. `prove`'s provenance caller (`main.rs` L1275) is left as it is.

**P4 · LP2.** A tier-1 test runs `verify`'s own tail on one failing report, with each fab mode and without one. It sees one exit code and one verdict apart from `fab`. A mutant that ties the failing exit to the fab mode fails it.

**P5 · F16.** Two tier-1 tests hand `fab::derive` one binding that differs while the other holds. Each sees `BindingMismatch` and its own message. A mutant that deletes either guard fails its test. Neither test needs KiCad.

**P6 · Row 6.** Read at Renode's source, the 28 rows split three ways:
- **7 `observed`:** HSION, HSIDIV, CSION, HSI48ON, PLL2ON, PLL3ON and TIMPRE. The model stores each field and acts on it (cited lines).
- **19 `ignored-by-model`:** tags (HSIKERON, CSIKERON, D1CKRDY, D2CKRDY, STOPWUCK, STOPKERWUCK, RTCPRE, HRTIMSEL, MCO#PRE, MCO#) and read-only bits whose write the model drops (HSIRDY, HSIDIVF, CSIRDY, HSI48RDY, HSERDY, PLL1RDY, PLL2RDY, PLL3RDY, SWS).
- **2 stay `independent-rule`:** PLLSRC_CSI and PLLSRC_HSE. They are PLLSRC's bits, which `aethelgard_ir::clock::judge_tree` holds, so their judge becomes `pll_rule`.

Each row's note names the twin's clock rule as what also holds the whole word.
- P6.1: no HSE board's packaging verdict moves. All three HSE specifications have `acknowledge_unwitnessed: []`, and `RCC.CR` already holds unwitnessed rows (HSEBYP, HSERDY), so `--fab-out` is refused before and after. What grows is the list of unacknowledged rows and the entries `missing_entries` asks for (P6.2).
- P6.2: `RCC.CFGR` gains its first unwitnessed rows. `PLLCKSELR` gains none.
- P6.3: no committed verdict or golden names these rows (grep: none outside source and docs).
