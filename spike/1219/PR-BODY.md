# PR #333: its current title and body

PR #333's description on GitHub was written at its first push and is now stale. `gh-write` can open a PR but not edit one, and a minted token may be spent only through it. This file is the current text, for the architect to set at merge.

**Title:** AET-WO-1219 (spike, STOPPED at §7): the STM32-coupling audit, with 1,433 IR entries classed V/N/S/D; the schema is shallowly coupled; two required keys decide the epoch

**A measurement that stops.** Spike-only: `spike/1219/` and `docs/work-orders/AET-WO-1219.FEEDBACK.md`. No crate, specification, `fab/`, workflow or `ci/` file changes, so the CLAUDE.md gates (`cargo fmt`, `clippy`, `test`) were **not run**, as the work order grants. No package dispatch. Left unmerged for the architect.

**What it measured:**
- **Order of work.** The class rule and the predictions were committed (`c9c35e09`) before the IR was read. Four dated amendments follow in RULES §6.
- **The reader.** A scratch reader (`reader.py`; no `Cargo.toml` under `spike/`) walks every serde type in `aethelgard-ir`. It cross-checks every key against all 34 committed current specifications and their serde dump (`dump.rs`, built in its own scratch workspace). Every data key is a source key, and the 16 source keys no file writes are named.
- **Classing.** Each entry was classed by a first lens. 589 of them were re-classed blind by a second lens; the two agree on 550. All 39 disagreements are settled with reasons in `overrides.tsv`; `out/disagreements.md` shows both lenses' answers.
- **Held data.** Every held D, S or N entry and every table-row citation names a document and a page, resolved from held documents (`citations.tsv`), and the join refuses one without.
- **Guards.** The reader's own guards each have a mutant, and `mutants.sh` kills all 7 for the right reason. The per-version dating is checked against every versioned file (`datecheck.py`).

**The count:**

| unit | V | N | S | D |
|---|---|---|---|---|
| specification keys (312) | 291 | 8 | 13 | 0 |
| refusal kinds (292) | 248 | 11 | 33 | 0 |
| held tables and source items (174) | 104 | 5 | 33 | 32 |
| table columns (390) | 369 | 12 | 9 | 0 |
| output types (265, not counted toward acceptance) | 163 | 34 | 68 | 0 |

**RP2350:** "not held" in all 227 N/S cells. No datasheet is on the box (`spike/1219/RP2350.md`: a byte-level scan of 227 PDFs, with a positive control). Nothing was fetched.

**Lean:** no S key and no S refusal kind must change for the engine as it stands. The coupling lives in held tables, register constants and solver code, where a second vendor adds rows or code, not keys.

The exception: if the epoch is meant to admit a second vendor without a bump, `mcu.bor_level` and `mcu.core_supply` must change at it. Both are required in all 34 specifications, and both hold ST value sets (`BOR_LEV` 0–3; RM0433 Table 34). This is a schedule question for Scotty (FEEDBACK §5).

**Decisions for the architect:** D1–D3 and F1–F3, in FEEDBACK §7.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_0165X1yumKmgqSLg9NCEoCS6
