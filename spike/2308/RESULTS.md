# AET-WO-2308 — results, beside `PREDICTIONS.md`

Every run on Furnace, 2026-11-22. The base is `c9d4096b`. Evidence is in `~/.cache/aet-2308/`.

## R0. Baselines (before any change)

- **Package:** `fab/sentinel-01-bench/` hashed file by file (`baseline/fab-SHA256SUMS`, 20 files).
- **The sweep:** `spike/1211d/measure/sweep.sh`, run with the base release CLI (`target-base`), over 34 specifications (`baseline/sweep`).
- **Stand-ins in every base `emit`:**
  - 23 of the 34 boards carry a BOM note.
  - The only note is W25Q's symbol stand-in, on 41 rows.
  - No committed board draws a twin stand-in with a board, except `crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json` (U5, ICM20948; see R2).
- **A twin stand-in reaches emit's BOM with an empty note.** `imu-baro-1v8.json` drawn, with one ICM20948 and declared pin capacitances, emits `U5,ICM-20948,Sensor_Motion:InvenSense_QFN-24_3x3mm_P0.4mm,Sensor_Motion:ICM-20948,` (`standin/out`).
- **D1 (the drawn bulk, `spike/1211f/d1-drawn.json`) under the base `verify --fab-out`,** with one baseline scenario:
  - It passes validate, solve, the ledger and the harness check.
  - It stops at `CampaignCoverageShort` (serial port `CMD`, ADC input `DRV_IPROPI`, rail `VDD_3V3`).
  - So the hazard is one set of scenarios away (`d1reach/`).

## R1. Row 1: the shapes measured

| File in the package | Can carry a label? | Shape |
|---|---|---|
| `.bom.csv` (emit's) | the note column, for a symbol stand-in and a bulk's source; **nothing for a twin stand-in** (the emitter is fenced) | as emitted |
| `-jlcpcb-bom.csv` | a fifth column, `Note`. The sample has four (`Sample-BOM_JLCSMT.xlsx`, read cell by cell). JLCPCB's KiCad page calls a five-column BOM "aligned with the sample" [read], so that an upload tolerates `Note` is [inferred] | (a) |
| `FAB-NOTES.md` | any text | (a): a section listing every noted part |
| `-pos.csv`, `-jlcpcb-cpl.csv` | KiCad's file is unedited, and the CPL has no value column | named by reference in the FAB-NOTES section |
| Gerbers, drill, ELF | no part values | — |

**Ruled 08:58Z (Q1–Q4):**
- (a) for symbol stand-ins and a bulk's source;
- (b) `StandInInFabPackage` for a twin stand-in;
- the sheet (`.kicad_sch`, from the fenced generator) is a FEEDBACK row.

## R2. Q1's condition: (b) refuses no committed package (run, not read)

- **The bench package:**
  - The spec is `examples/sentinel-01.json`, with ci.yml's five `bench-package` scenarios, `--fab-out`, the branch binary and an empty `PATH`.
  - Every pre-gate check passes. The run stops at `ToolMissing kicad-cli`, exit 4.
- **`sentinel-02-power.json`** with its three scenarios and `--fab-out`:
  - Base refuses it as `CampaignCoverageShort` (exit 3).
  - No package of it exists, and no test or workflow runs a fab flag on it, except `battery_sag_tier1`'s coverage test.
  - The stand-in check is therefore placed **after** the coverage check, so this board's verdict is byte-identical to base.

## R6. Row 6: what judges each field (read at source, then tested)

- `twin/src/hse.rs` `judge_clock` compares each whole `CR`, `PLLCKSELR` and `CFGR` word written against RM0433's reset value with the header's bits set. It never uses the generator's word. So every field of those words is held, but by a rule in `aethelgard_twin`, which is not `RUL-20` (1).
- Renode's `STM32H7_RCC.cs` is the copy whose L185 and L186 match the ledger's existing citations (sha256 `7f10d491…`).

| Field | Renode | Class |
|---|---|---|
| HSION | L137 stored, L147 HSIRDY reads it | observed |
| HSIDIV | L148 stored, L1546 divides hsi_ck | observed |
| CSION | L160, L169 | observed |
| HSI48ON | L172, L173 | observed |
| PLL2ON | L199, L200 | observed |
| PLL3ON | L201, L202 | observed |
| TIMPRE | L231 stored, L1490–1494 the timers' multiplier | observed |
| HSIKERON, CSIKERON, D1CKRDY, D2CKRDY | L146, L170, L174, L175 tags | ignored-by-model |
| STOPWUCK, STOPKERWUCK, RTCPRE, HRTIMSEL, MCO1PRE, MCO1, MCO2PRE, MCO2 | L227–230, L233–236 tags | ignored-by-model |
| HSIRDY, HSIDIVF, CSIRDY, HSI48RDY, HSERDY, PLL1RDY, PLL2RDY, PLL3RDY, SWS | read-only (L147, L158, L169, L173, L185, L197, L200, L202, L226): the written bit is dropped | ignored-by-model |
| PLLSRC_CSI, PLLSRC_HSE | PLLSRC's bits (L267) | independent-rule, by `aethelgard_ir::clock::judge_tree` |

- The base table has 28 `independent-rule` rows whose judge names no `aethelgard_ir` rule: exactly these. The branch has none (`every_independent_rule_row_is_judged_by_an_ir_rule`).
- Ruled 08:58Z (Q5) as built.
