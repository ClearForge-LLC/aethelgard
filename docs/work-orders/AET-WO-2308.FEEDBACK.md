# AET-WO-2308 — FEEDBACK: verify's backlog

**Branch:** `wo/aet-2308-verify-backlog` from `main` `c9d4096b`; `main` `c79b9820` merged in (docs only). **Builder:** Claude (Opus 5.5 orchestrating), with one fresh Opus subagent for the targeted adversarial pass. Its report was checked at source before use. Written 2026-11-22.

**Status:** see §7. **The answer, in six lines:**
- **No stand-in reaches a fab file unlabelled.**
  - (a) A symbol stand-in, and a declared bulk's source, are carried into the JLCPCB BOM's new fifth column, `Note`, and into a FAB-NOTES section, "Parts with a note".
  - (b) A drawn twin stand-in refuses both fab flags by name (`StandInInFabPackage`, exit 2).
  - Shown through `verify --fab-out`: the bench board's U4 (the W25Q symbol stand-in), in the image, and in `verify_2k2_tier2`'s `--fab-out` run. Shown before any gate: a drawn ICM20948, in `fab_tier1`.
- **L3:** the campaign gate's `tool_version` is the Renode build that every run's archive names (`1.17.0+20260907gitf1dd1b4af.aet1605`, measured). It was `null`.
- **L4:** an empty directory is `against_extra` `dir/` under `--against` and `--against-fab`. Prove's provenance check is unchanged.
- **LP2:** verify's tail is `conclude()`. A tier-1 test runs it on one failing report in every fab mode and gets one exit code, `EXIT_FAIL`.
- **F16:** both `BindingMismatch` guards have a tier-1 killing test. **Row 6:** 28 rows are reclassed as ruled: 7 `observed`, 19 `ignored-by-model`, and 2 `independent-rule` by `judge_tree`. No `independent-rule` row now names a judge outside `aethelgard_ir`.
- **Bytes:**
  - The sweep is identical: 34 specifications, 280 files.
  - Every committed specification's fab-path verdict is byte-identical, base against branch (5 of 5, run).
  - The bench package moves in exactly 3 of its 17 files, by rows 1 and 2, every line named in §2.
  - **Mutants: 19 of 19 killed** (16 tier 1 and 3 tier 2; §4).

## 0. How it was made

- **Predictions first:** `spike/2308/PREDICTIONS.md`, committed at `fdc906e3` before any build. The results beside them are in `spike/2308/RESULTS.md`.
- **Bus:** Q1–Q5 were posted at 08:47Z with leans. **They were ruled to the leans at 08:58Z, with two conditions**, both met (§2):
  - Q1: (b) must refuse no committed package, shown by running.
  - Q5: no packaging verdict may move, shown by `verify --fab-out`.
- **Kit (`spike/2308/kit/`):**
  - `mutants.py`: one mutant per guard, run on a copy of the tree.
  - `fab-paths.sh`: base against branch on every HSE board and every committed board that draws a twin stand-in, plus the bench specification.
  - `bench.sh`: CI's bench-package job in the image.
  - The sweep is `spike/1211d/measure/sweep.sh` and `same.sh`, unchanged.
- **Evidence:** `~/.cache/aet-2308/`: `baseline/`, `branch-sweep/`, `fab-paths/`, `fab-br/`, `fab-base/`, `bench-*`, `mut/` and `adv/`.

## 1. What was built

- **`crates/aethelgard-eda/src/fab.rs`** (the JLCPCB BOM's note column only, as fenced):
  - `JLCPCB_BOM_HEADER` gains `,Note` after the sample's four columns.
  - `bom_rows` keeps the note. `jlcpcb` groups by value, footprint and note, so a noted part never shares an un-noted row, and writes the note as RFC 4180 quotes it.
  - `noted(bom)` lists every noted part in natural order.
  - `jlcpcb_notes` says the column is a fifth one, that JLCPCB's tolerance of it is inferred (Q2), and what to do if an upload refuses it.
- **`crates/aethelgard-cli/src/fab.rs`:**
  - `derive` appends `noted()`: "Parts with a note", one line per part, `- U4 (W25Q64JVSSIQ): <note>`.
  - The section names the two files that carry the note on the part's line, and the two (`-pos.csv`, `-jlcpcb-cpl.csv`) that name the part by reference only (Q4).
  - A board with no noted part adds no byte.
- **`crates/aethelgard-cli/src/verify.rs`:**
  - `stand_ins()` lists each drawn part the parts table holds `stand_in` (ICM20948, ICP10111). Under a fab flag, `StandInInFabPackage` refuses it by reference, exit 2.
  - That check sits **after** `CampaignCoverageShort` and before the acknowledgement check, so a board already refused for its campaign keeps that verdict (§2, R2). A vendored table that cannot be read is an internal error (exit 70), never an empty list.
  - L3: `campaign_renode()` replays each run's archive and returns the one Renode build they all name. Two builds, none, no run, or an archive that does not replay is a `ToolError`.
  - L4: both comparisons call `tree::against`.
  - LP2: the tail is `conclude(&mut report, fab_mode, &Tail) -> Result<u8, ExitCode>`. `verify` calls it and prints with the code it returns.
- **`crates/aethelgard-cli/src/tree.rs`:** `diff` and `against` share one walk. `against` also reports an empty directory under which no generated file lives, as `Extra("dir/")`. `diff` keeps its behaviour for `prove`'s provenance (`main.rs` L1275, outside the fence).
- **`crates/aethelgard-cli/src/ledger.rs`:** the HSE loop is rewritten per field, as ruled (Q5).
  - Each row cites the Renode line that stores and acts on the field, or that drops the written bit.
  - Each row names `aethelgard_twin::hse::judge_clock` as a twin-side rule, "not an aethelgard-ir one (RUL-20 (1))".
  - `PLLSRC_CSI` and `PLLSRC_HSE` take `pll_rule`.
- **Tests:**
  - eda: grouping with a noted part; `noted`.
  - cli unit: the FAB-NOTES section; F16, both bindings; LP2; L3 on synthetic archives; L4 in `tree`; the ledger invariant.
  - `fab_tier1`: the drawn stand-in, both flags; an empty directory under `--against`.
  - `verify_2k2_tier2` (an existing run): U4 labelled in the BOM, the JLCPCB BOM and FAB-NOTES; the campaign gate equal to `RENODE_BUILD`.
  - `against_fab_tier2` (an existing run): an empty directory, 19 differences.
  - The JLCPCB golden in `cli/src/fab.rs` moves by the column.

## 2. Measurements and predictions, scored

| Prediction | Result |
|---|---|
| P1.1 the bench `-jlcpcb-bom.csv` moves on all 14 lines; U4 carries the note | **holds** (diff in RESULTS R3) |
| P1.2 FAB-NOTES moves by the new section and the JLCPCB sentences; MANIFEST by those hashes and the verdict line | **holds** (R3) |
| P1.3 no other package file moves | **holds**: 14 of 17 byte-identical to the committed package |
| P1.4 the sweep is identical | **holds**: 34 specifications, 280 files |
| P2.1 the campaign version equals `simulate`'s `Renode v1.17.0` | **missed on form**: it is the archive's build string, `1.17.0+20260907gitf1dd1b4af.aet1605`, the twin's `RENODE_BUILD`. `simulate`'s entry reports `--version`'s line. Both name the same Renode |
| P2.2 MANIFEST's verdict line moves by L3 | see R3 |
| P3, P4, P5 | **hold** (tests, and their mutants killed) |
| P6 7 / 19 / 2 | **holds** |
| P6.1 no HSE packaging verdict moves | **holds, run:** `fab-paths.sh`, 5 of 5 lines byte-identical |

## 3. Adversarial pass (targeted, fresh subagent)

(filled from the subagent's report, checked at source)

## 4. Mutants: 19 of 19 killed

One per guard this work order adds or changes, each against the suite it lives in (`spike/2308/kit/mutants.py`; records in `~/.cache/aet-2308/mut/`).
- **M8 first survived.** A run whose archive names no Renode build gave `""` under the mutant, and the test's only such case paired that run with one naming a build, so the disagreement check refused it anyway. A lone such run was added, and the mutant was then killed.

## 5. Findings, not fixed here

## 6. Gates, fence, reduction pass

## 7. State
