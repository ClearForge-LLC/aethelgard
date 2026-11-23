# AET-WO-3308 FEEDBACK: freeze what was ordered, and bind the bench sheet to the package's ELF

Builder: Claude (Opus 5.5) in `~/aethelgard-3308`, branch `wo/aet-3308-freeze-ordered`, PR #362. Base `b9d5f4b6`; main `75da8053` merged before the build.

## 0. Status

**Built to the 12:25Z ruling.** The measurement and stop are in `spike/3308/` (`STOP.md`, `ROWS.md`, `results.md`, `PREDICTIONS.md`).

**What moved:**
- `fab/sentinel-01-bench/ORDERED.sha256` (new).
- `fab/sentinel-01-bench/BENCH.md`, text only.
- Five tier-1 tests in `crates/aethelgard-cli/tests/bench_board_tier1.rs`.
- `ci/route.sh` (`is_docs`) and `ci/test-route.sh`.

No `package/` byte moved, and no crate `src/` or workflow changed. `ci/test-weights.tsv` needs no row (it lists tests of 0.5 s or more). The new tests were measured each alone in the image (`--exact`, one test run per call) at 1–10 ms; the package they copy is 412 KB.

## 1. The measurement (§1.1) and the stop (§1.2)

| | Predicted | Measured |
|---|---|---|
| The ten hashes at `ac5f8c82`, `ee835bde`, `cd79024c` | equal | equal, and at `b9d5f4b6`; only `ac5f8c82` ever touched them; equal to MANIFEST |
| Test (3) on the old sheet | fails, naming `008a2a59` (the WO) | fails, but because the sheet held **no** full hash, only abbreviations (`spike/3308/test3-draft.out`) |
| Each `BENCH.md` row against `f3e871bb` | holds | **no value fails** (`ROWS.md`). 12 rows were text scoped to `008a2a59` or `e4e24372` (S); citations moved line only |

**How each label was checked** (`INV-07`):
- **Twin rows:** against `simulate --for 100ms` of the package ELF in the pinned image (verdict pass; 25/25 stored registers).
- **Generator rows:** against HEAD's generated crate. `verify --against-fab` re-derives the ELF byte for byte.
- **Rule rows:** against the verify ledger and `silicon` figures.

**Ruled 12:25Z:**
- D1 `sha256sum` form, with `#` lines.
- D2 a closed name set.
- D3 one full digest, plus a named non-ELF set.
- D4 the S rows rewritten for `f3e871bb`, with B9 part 1's lift marked as a change before the bench.
- Q-ZIP unanswered: built from the commit range, and the header says the uploaded zip outranks it.

## 2. The build

- **`ORDERED.sha256`** has a header (the order date, the range the bytes held across, the zip's precedence, the re-spin sentence, how a ruling block is written), then the ten entries. `cd fab/sentinel-01-bench && sha256sum --strict -c ORDERED.sha256` passes; GNU 9.7 in the image ignores the `#` lines.
- **(2)**
  - `the_package_holds_the_files_as_ordered`: every listed file's bytes, and the package's whole name set (the ten plus `NOT_ORDERED`'s seven).
  - `a_moved_added_removed_or_split_fabrication_file_is_named`: one byte, a new `.gbr` layer, a renamed layer, the drill split into `-PTH`/`-NPTH`, a file removed. Each case is run on a copy, and each finding names the file.
  - `the_ordered_list_refuses_crlf_a_duplicate_a_malformed_line_and_a_rewrite`: CRLF, no final newline, eight malformed forms, an undated block, a duplicate in one block, and an in-place rewrite of the as-ordered block. It also accepts a dated ruling block that supersedes an entry.
- **(3)**
  - `the_bench_sheet_names_the_package_elf`.
  - `a_bench_sheet_naming_another_elf_fails`: one character off, a prefix only, the ELF twice, a second full digest in a table row, the old ELF abbreviated (`…` and `...`), bare at 8 and 7 digits, after `0x` (9 digits), glued to a word, and in capitals. Register words and words of hex letters pass.
- **`BENCH.md`:**
  - It states the full ELF hash once, and says its predictions were re-checked against it.
  - The records are eight words, with words 6 and 7 in the table, and the read and clear commands take eight.
  - Every `main.rs`/`board.rs` citation is at HEAD's line, and a sentence says what the line numbers are of.
  - The S rows are rewritten. B9 part 1's pass line and the U2-empty table's B9 row carry **"Changed 2026-11-23 (`AET-WO-3308`), before the bench"**, with the old line struck through.
- **Routing:**
  - `fab/*/BENCH.md` is code.
  - `test-route.sh` keeps its docs case on `fab/b/ORDER.md` and adds a bench-sheet case and an `ORDERED.sha256` case, both code.
  - The walker case now holds `ORDER.md`, so it still refuses for its stated reason.
  - `route.sh check ci/docs-tests.tsv` passes. Under the old `route.sh` it refuses the new test, which reads `BENCH.md`; that is the route change's mutant.

## 3. Mutants (one per guard this WO adds; `~/.cache/aet-3308/mut/mutants.py`)

**24 of 24 killed** on the final tree, each run alone against its own suite (`~/.cache/aet-3308/mut/run5.log`):
- **`bench_board_tier1` mutants (23):** CRLF; final newline; hash form; lowercase hash; empty name; name characters; dated ruling block; the ruling's date (`# Ruling soon`); as-ordered end; block reset; duplicate in a block; ruling supersedes; as-ordered pin; moved bytes; missing file; extra file; a long run anywhere; seven standing alone; alone before; alone after; more than 8 after `0x`; ELF exactly once; the named non-ELF set.
- **Route mutant (1):** the route line. `test-route.sh` and `route.sh check` both fail.

**Earlier runs found four survivors, each redundant, and all four were removed** (§5):
- the count of ten;
- the ELF's bytes against MANIFEST (already held by the manifest test);
- the header-first guard. The pin fails a headerless list, and its case passed for the wrong reason: "header" matched another error;
- the `...` continuation. A bare run is a digest anyway.

## 4. Adversarial pass (targeted, §5; a fresh Opus subagent given the code, the WO and the ruling)

| # | Finding | Severity | Done |
|---|---|---|---|
| 1 | A bare or 7-digit prefix of another ELF passed (3): only `…`/`...` abbreviations were tokens | real | **fixed**. Any run of 16+ hex anywhere, 7+ standing alone, or 9+ after `0x` is a digest. The committed sheet has exactly three: the ELF and the two named |
| 2 | `0x` plus 64 hex, or a hash glued to letters, passed | weak | **fixed** for `0x` and for runs of 16 or more. A 7–15 digit run glued to a letter is still a word (`FEEDBACK`); stated in the doc comment |
| 3 | A hash split across lines or in groups passed | weak | **fixed** by #1: each piece of 7 or more is a digest |
| 4 | The tokenizer's boundary conditions had no killing case | weak | **fixed**: a case each way per condition, and a mutant each (§3) |
| 5 | The header promised dated ruling blocks the parser refused as duplicates, while an in-place hash edit passed | real | **fixed**. A `# Ruling YYYY-MM-DD` block's entries supersede; a duplicate is refused only within a block; an undated block is refused; the as-ordered block (every byte above the first ruling) is pinned by sha256 in the test |
| 6 | The uppercase case was refused for its path, not its hash; no empty-name case | weak | **fixed**: uppercase hash only; `package/` with no name |
| 7 | `I2C1.CR2`'s mask note named U2 as the last transfer | weak | **fixed**: U3's, at `0x48` (L231); value unchanged |
| 8 | The U2-empty table's B9 row lacked the `0x90` transaction | weak | **fixed**, marked as changed |
| 9 | Other moved pass-line texts (word 0 "after record 2", the eight-word commands) are unmarked | nit | **left**: their values were already predicted for this ELF before the change; only B9's lift was ruled a changed prediction |
| 10 | Line citations are not pinned to a commit | weak | **fixed**: one sentence says what the numbers are of; the statement is what holds |
| 11 | L219 keeps the VOS3 history in the past tense | nit | **left**: it names no hash |
| 12 | A commit or run id of 7+ hex digits is now a digest | nit | **stated** in `digests`' doc comment: name it in `NOT_AN_ELF` or leave it out |

## 5. Reduction pass

Reduction pass (diff): about −20 net, counted per edit · 2 duplicates removed · 4 dead paths removed · 0 protected kept · 0 findings below.
- **Duplicates removed:** the ELF-bytes assertion (the manifest test holds it), and a second `MANIFEST.json` reader (one `manifest()` now).
- **Dead paths removed:**
  - the count-of-ten assertion (the closed name set fails any dropped entry);
  - the header-first guard (the pin);
  - the ellipsis continuation (the non-ELF set is named by eight-digit prefixes);
  - the `0x` rule's third-character check (no case could kill it; dropping it only makes the rule stricter).
- **Proof:** the suite is green, and every surviving guard's mutant still fails (§3).

## 6. Findings and backlog

- **F1 (the 12:25Z ruling: a FEEDBACK row).** A local `verify --against-fab` re-derives 16 of the 17 package files byte for byte, the ELF among them. `MANIFEST.json` differs only in `verdict_sha256`.
  - The derived manifest equals the committed one with the local verdict line's hash in place (`4dcb0bea…`).
  - The verdict line depends on the machine, in a field not found. On Furnace it carries `uid_boundary_unmet: user_namespace` on 17 gates, and substituting CI's likely values does not give `b33470…`.
  - CI's bench-package log does not print the verdict.
  - **Backlog:** print the verdict line in bench-package's log, or make it machine-independent, so the manifest can be checked off CI.
- **F2.** The WO's prediction for (3) ("fails, naming `008a2a59`") assumed the sheet held a full hash. It held none, and that is the gap (3) closes.
- **F3.** `b2.py` (3403's B2 generator) composes `PWR.D3CR` as `0x8000`: it takes the written `VOSRDY` 0 to a read-only bit over the ledger's reset row. The sheet's `0xA000` is right. The script lives in `spike/3308/` only.
- **F4.** No test enforces "only by a ruling". A WO can still add a `# Ruling` block that is no ruling. The pin makes an in-place rewrite fail; a ruling block is visible in review. That is the honest limit of a tier-1 test.
- **F5.** `BENCH.md` now cites `spike/3308/ROWS.md` for its row-by-row re-check. Earlier spike directories were deleted after their builds; if this one goes, the sentence goes with it. The architect decides.
- **Q-ZIP stays open.** If Scotty finds the uploaded zip, a member that differs from the range is recorded in a ruling block.

## Gates

Run in the pinned image (`container/IMAGE` `1e0ab105…`) under `capped-run --mem 16G`, on a copy of the tree at `ebaab589` (the last code commit), with the target inside the repository (CI's layout):
- `cargo fmt --all -- --check`: 0.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: 0.
- `cargo test --workspace --locked`: 0. **2,897 passed, 0 failed**, in 165 test binaries.
- `bash ci/test-route.sh`: all 45 cases passed. `bash ci/route.sh check ci/docs-tests.tsv`: inventory ok, 9 rows.
- `cd fab/sentinel-01-bench && sha256sum --strict -c ORDERED.sha256`: 10 OK.
- `git diff b9d5f4b6 -- fab/sentinel-01-bench/package`: empty. No package byte moved.

**After the workspace run,** one test-only change: the `# Ruling soon` case and its mutant. `bench_board_tier1` (7 passed), clippy on that target, and the 24 mutants were re-run on the final tree; the workspace run was not repeated. CI on the pushed head is the remaining gate.
