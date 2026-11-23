# AET-WO-3308 — Freeze what was ordered: the ordered Gerbers and drill held by a committed hash list, and the bench sheet bound to the package's ELF

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-3308-freeze-ordered`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4, from the round-3 review. Written 2026-11-23. **Cadence:** a measurement, then a build with one stop.

**Grounds** (`docs/reviews/2026-11-23-mythos-5-2.md` A9, A10, Part C 3, and its response):
- **A9.** Nothing pins what was sent to the fabricator. CI's `--against-fab` re-derives the package, so a work order that moves copper recommits it and stays green.
- **Checked at `cd79024c`:** the ten files `ORDER.md` §1 says to upload (`ORDER.md:20`) are byte-identical from `ac5f8c82` (2026-11-17) to `cd79024c`:
  - copper `F_Cu.gtl`, `B_Cu.gbl`;
  - mask `.gts`, `.gbs`;
  - paste `.gtp`, `.gbp`;
  - silk `.gto`, `.gbo`;
  - edge `Edge_Cuts.gm1`;
  - drill `.drl`.
  `ee835bde` is the last commit before the order's date ended (2026-11-19 19:43 UTC). The order's commit is recorded nowhere: **ask Scotty whether he holds the uploaded zip.** If he does, its members' hashes are the source.
- **A10.** `BENCH.md:5` names the ELF `008a2a59…`, which was the package's from `7d0763eb` to `ee835bde`. `943f7112` (`4240v`, `e4e24372…`) and `8fdc9422` (`4241`, `f3e871bb…`) followed. `MANIFEST.json` holds `f3e871bb…`.
  - `BENCH.md:92` already names `f3e871bb…` for the eight-word records.
  - The B2 supply rows were predicted at `d70a99e5`, after `4240v`.
  - `BENCH.md:152` checks the flashed ELF against `MANIFEST.json`.
  - So the sheet holds predictions made against three ELFs, and the bench flashes the newest.
- **Routing.** `ci/route.sh:32` (`fab/*.md`) routes `BENCH.md` docs-only, and `ci/test-route.sh:35` asserts it. A file named `ORDERED.sha256` would already be code (`:33`). No test reads `BENCH.md`.

> **What this is:**
> - **(1) `fab/sentinel-01-bench/ORDERED.sha256`:** the ten files in `sha256sum` format. Its header names the order date, the commit range the bytes held across, and its source (Scotty's zip, or the range). It also says: **"A re-spin edits this list only by a ruling, named in a dated block added below; nothing above it is rewritten."**
> - **(2) A tier-1 test in `cli/tests/bench_board_tier1.rs`** that does three things:
>   - holds each listed file's bytes to the list, failing with the file's name;
>   - holds the package's files with a fabrication extension to exactly the listed set;
>   - refuses a list with CRLF, a duplicate or a malformed line.
> - **(3) A tier-1 test** that `BENCH.md` states one full 64-hex ELF hash, equal to `MANIFEST.json`'s `sentinel-01-bench.elf`. It fails today.
> - **(4) `BENCH.md` re-checked** against `f3e871bb…`: every predicted RAM word, register value and cited `main.rs` line, by its source label (twin, rule, generator, repeat). The text names one ELF.
> - **(5) `route.sh`** routes `fab/*/BENCH.md` as code, and `test-route.sh` pins `BENCH.md` and `ORDERED.sha256` as code.
> - Every refusal has a test, and every test a mutant it kills.
>
> It is NOT:
> - any byte under `fab/sentinel-01-bench/package/`;
> - `ORDER.md`, `BENCH-EXT-BMP581.md` (its own ELF, held off the package), or `FAB-NOTES.md`;
> - a list for Sentinel-03, which comes with its order (`4300k`);
> - a changed prediction. A word that no longer holds is a stop (`INV-07`).

## 1. Build

1. **Measure first,** with predictions committed in `spike/3308/`:
   - the ten hashes at `ac5f8c82`, `ee835bde` and `cd79024c` (prediction: equal);
   - (3) run against today's `BENCH.md` (prediction: fails, naming `008a2a59`);
   - each `BENCH.md` row against `f3e871bb…`: the twin's run of that ELF in the pinned tools image for twin rows, and the generated source at `main` for generator rows. Predict which rows hold before running.
2. **Stop with:** the source of the list, and the row-by-row table.
3. **Build (1)–(5) to the ruling.** Only `BENCH.md`'s text moves under `fab/`.

## 2. Acceptance

- **(2) fails, naming the file,** when one byte of any listed file changes, or a fabrication file is added or removed. Each case has a test.
- **(3) passes,** and fails with a hash one character off or a prefix only.
- **A PR that touches only `BENCH.md`** routes as code, and `ci/test-route.sh` passes.
- **No `package/` byte moves.**
- **The CLAUDE.md gates pass, and CI is green.**

## 3. Fences

- **Fence:**
  - `fab/sentinel-01-bench/ORDERED.sha256` (new);
  - `fab/sentinel-01-bench/BENCH.md`, text only;
  - `crates/aethelgard-cli/tests/bench_board_tier1.rs`;
  - `ci/route.sh` (`is_docs`), `ci/test-route.sh`, `ci/test-weights.tsv` rows;
  - `spike/3308/` and FEEDBACK.
- **Not:** `fab/**/package/**`; any crate's `src/`; workflows.
- **Running beside you:** the backlog wave in roadmap §5 (`4300j`, `1323`, `4408`, `1916` now). `1916` also edits `ci/`. Merge `main` before READY.
- One PR, left unmerged. Write `~/.cache/aet-3308/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A package change the list misses:** a renamed layer, a new `.gbr`, an `.drl` split in two, or a trailing space or CRLF hiding a mismatch.
- **A `BENCH.md` that passes (3) while it names another ELF elsewhere,** such as a second hash or a hash in a table row.
- **A walker of a fab directory:** `route.sh inventory` refuses one (`test-route.sh:114`), so name each path as a literal.

## 7. Flag-and-stop conditions

- **After §1.2:** stop with the list's source and the row table.
- **A predicted RAM word or register value no longer holds** against `f3e871bb…`. Stop with the row and both values.
- **Any `fab/` byte other than `BENCH.md`'s text would move.** Stop with the diff.
- **The ten hashes differ between `ac5f8c82` and `cd79024c`.** Stop: A9's premise is wrong.
