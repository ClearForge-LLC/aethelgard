# AET-WO-3300 — FEEDBACK

**Branch:** `wo/aet-3300-fab-package` · **Base:** `origin/main` 8f04772 (`AET-WO-1306`) · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Cadence:** spike (§1.1, as amended 2026-11-13 on `claude/argon-repo-docs-synthesis-rjrb0d`), then build.

**Outcome: STOPPED in the spike, on §7.1.** KiCad 10.0.7 cannot be made to write deterministic Gerbers with its own options. Every Gerber layer carries two lines from the wall clock. No option, environment variable or advanced setting removes or fixes them: the spike measured this, and KiCad's source at tag 10.0.7 confirms it. Everything else the package needs is deterministic, measured: the drill file (with KiCad's own `--excellon-min-header`), the placement CSV, and the zone refill. Without the two clock lines, every Gerber byte is identical across two runs at each of two paths. No code is written. §1.1 forbids post-processing KiCad's files without the architect's ruling, and that ruling is the one decision needed (F1).

Entry schema: `finding · where · type · recommendation · decision-needed`.

## §1.1 The determinism table

**Method.**
- **The board.** `aethelgard generate --spec examples/sentinel-01-minimal.json`, `kicad/sentinel-01.kicad_pcb`, sha256 `7b9b0e3d…2070` (the reference board, as in `AET-WO-1305` F5).
- **The prepared copy (amendment, `AET-WO-1306` F9).** It is the gate's prepared copy, made exactly as `kicad::prepare(Check::Drc)` makes it: the strict reader, then `board::prepare_read` with `rules::SETUP`, `input.kicad_pro` and `input.kicad_dru` from `Rules::from_specification`, the vendored libraries in `lib/`, and project-local library tables naming them. A probe crate in its own workspace called those functions; it is not committed. The prepared board is `55e1418a…e33e`, the same at A and B; only the two library tables differ, because they name the path.
- **The runs.** In the pinned image (`c69ff9bc…`), as root, with `TMPDIR` unset (CI's shape), under rootless podman: `--memory 16g --memory-swap 16g --pids-limit -1`, then 0 written to the container's `memory.swap.max`. Read back: `memory.max=17179869184`, `memory.swap.max=0`, `pids.max=max`. Each command ran with `env -i HOME=<workdir> TMPDIR=<workdir>`, as the sandbox gives a tool.
- **Two runs at each of two paths.** A is `/var/lib/aethelgard/aethelgard-spike-a/tool`; B is `/work/b/a-much/deeper-path/tool`. The four runs are 1.2 s or more apart, so a seconds-resolution clock always differs.
- **The layers.** `F.Cu, B.Cu, F.Paste, B.Paste, F.SilkS, B.SilkS, F.Mask, B.Mask, Edge.Cuts`: every copper, mask, paste, silkscreen and edge layer the board uses.

"Distinct" counts the different sha256 values among the four runs; 1 is deterministic.

| id | command (on the prepared copy) | prediction (written before any run) | result | as predicted? |
|---|---|---|---|---|
| G0 | `pcb export gerbers --layers … --check-zones` | differs in `%TF.CreationDate` and the `G04 Created by KiCad … date` line; `.gbrjob` `CreationDate`; no path anywhere | **all 10 files: 4 distinct.** Each layer differs in exactly 2 lines: line 2, `%TF.CreationDate,2026-11-13T03:32:32+00:00*%`, and line 9, `G04 Created by KiCad (PCBNEW 10.0.7) date 2026-11-13 03:32:32*`. The job file differs in 1 line, `"CreationDate"`. No output byte depends on the path: no output contains A's or B's directory names. With those lines removed: 1 distinct, for every file | yes |
| G1 | G0 + `--no-x2` | still differs: KiCad writes the attributes as comments instead | 4 distinct; line 2 becomes `G04 #@! TF.CreationDate,…*`, and line 9 is unchanged. Without them: 1 distinct | yes |
| G2 | G0 with `SOURCE_DATE_EPOCH=1700000000` | still differs: KiCad 10 does not read it | 4 distinct, the same two lines carrying the wall clock | yes |
| Z | the refill by `--check-zones` | deterministic; B.Cu carries the GND pour as G36/G37 regions | **deterministic.** Without the clock lines B.Cu is 1 distinct across A and B, and it carries the pour: one G36/G37 region under `%TO.N,GND*%`, 40,914 bytes | yes |
| Z0 | G0 without `--check-zones` | B.Cu carries no pour | B.Cu has no G36/G37 region, 5,324 bytes. With the clock lines removed, the other eight layers and the job file are byte-identical to G0's (checked file by file); B.Cu differs in 1,448 lines. **So the refill flag is load-bearing,** and a test must pin that the pour is there | yes |
| D0 | `pcb export drill` (Excellon, defaults) | differs in the header's date | 4 distinct; 2 lines: `; DRILL file KiCad 10.0.7 date 2026-11-13T03:32:56` and `; #@! TF.CreationDate,…`. Without them: 1 distinct | yes |
| D1 | D0 + `--excellon-min-header` | identical at both paths | **1 distinct** (`a045ab03…`). The header loses the `FORMAT`, `TF.GenerationSoftware`, `TF.FileFunction,MixedPlating,1,2`, `FMAT,2` and `TA.AperFunction,Plated,PTH,ViaDrill` comments. It keeps `M48`, `METRIC`, `T1C0.300` and decimal coordinates | yes |
| D2 | D0 with `SOURCE_DATE_EPOCH` | — (not predicted separately) | 4 distinct, as D0 | — |
| P0 | `pcb export pos --format csv --units mm` | identical | **1 distinct** (`5056acf6…`), 15 rows (C1–C6, R1–R5, U1–U4), all on `top` | yes |
| P1 | `pcb export pos --format ascii --units mm` | differs in a dated header | 4 distinct; line 1 is `### Footprint positions - created on <date> ###` | yes |

**Read at source.** These are KiCad's raw files at tag 10.0.7, downloaded from gitlab.com/kicad/code/kicad and grepped:
- **`common/plotters/GERBER_plotter.cpp:289-291`** (`StartPlot`) is unconditional: `wxDateTime date = wxDateTime::Now();`, then `"G04 Created by KiCad ({}) date {}*"`.
- **`common/gbr_metadata.cpp:44`** (`GbrMakeCreationDateAttributeString`): `wxDateTime date( wxDateTime::GetTimeNow() );`. It serves the X2 attribute, the X1 comment, the job file and the drill file's `TF.CreationDate`.
- **`pcbnew/exporters/gendrill_excellon_writer.cpp:562-569`:** the dated `; DRILL file` line sits inside `if( !m_minimalHeader )`, which is why D1 is deterministic.
- **`place_file_exporter.cpp:229` and `:311`:** the ASCII position file and the report are dated, and the CSV is not.
- **`common/string_utils.cpp:819-821`:** `GetISO8601CurrentDateTime` is `wxDateTime::Now()`.

None of these files reads an environment variable or `SOURCE_DATE_EPOCH`. `common/advanced_config.cpp` has no key about dates or reproducibility. `--board-plot-params` reads the board's plot settings, and none of those is a date. So no KiCad 10.0.7 option makes the Gerbers deterministic: **§7.1.**

**Also observed.**
- Every export writes `input.kicad_prl` and `.config/kicad/10.0/*.json` into the workdir (the tool's `HOME`). That is where the sandbox already confines KiCad, and none of it is an output.
- `%TF.ProjectId,input,696e7075-742e-46b6-9963-61645f706362,rev?*%` names the copy's stem. Its GUID is the stem's bytes (`696e7075742e` is `input.`), so it does not depend on the path.

## Decision needed first (F1): may the package's Gerbers drop KiCad's two clock lines?

**Recommendation: rule (a).**

- **(a) A strict, line-exact removal of the clock, and nothing else.** For each Gerber layer, line 2 must match `%TF.CreationDate,<ISO 8601 with offset>*%` and line 9 must match `G04 Created by KiCad (PCBNEW 10.0.7) date <YYYY-MM-DD hh:mm:ss>*`. Each must occur exactly once in the file, at those places, and the remaining bytes must carry no other date. Both lines are deleted, not rewritten.
  - Any departure is a `ToolError`, exit 5, never a package. The rule is pinned by a test against KiCad's actual output, so a KiCad bump that moves a line fails loudly.
  - `TF.CreationDate` is an optional standard attribute and `G04` is a comment, so the result is still a conforming X2 Gerber, measured identical across paths and runs.
  - The verdict's `fab` hashes are the hashes of the bytes written, so `INV-03`'s "hashes equal files" holds exactly.
  - The job file (`.gbrjob`) is left out of the package. It is not a layer, and `FAB-NOTES.md` states the stackup.
- **(b) A fixed clock for KiCad** (a `libfaketime` preload). It adds a binary to the image and an `LD_PRELOAD` inside the sandbox, which the empty environment exists to exclude. Not recommended.
- **(c) Hash the Gerbers with the clock lines masked.** The verdict would then name hashes that are not the files' hashes, against the clause's own text. Not recommended.
- **(d) Our own Gerber writer from the strict reading.** It would have to fill zones the way KiCad's DRC fills them, which is the copper DRC judged. Too large, and a second source of copper. Not recommended.

**What the ruling unblocks, already decided by measurement** (§1.1's "choose and record why"):
- **Drill:** `kicad-cli pcb export drill --excellon-min-header`. It is KiCad's own option and needs no post-processing (D1). The minimal header omits the plating comment, so `FAB-NOTES.md` would state that every hole is a plated via of 0.3 mm (`route::DRILL_NM`). If the architect prefers the full header, the same rule as (a) applies to its two dated lines instead.
- **Placement:** `kicad-cli pcb export pos --format csv --units mm --side both` (P0), because it is deterministic as measured and is the format assemblers read. Our own writer from the strict reader would be byte-stable too, but it would be a second reading of footprint positions with nothing gained.
- **Gerbers:** exported from the gate's prepared copy with `--check-zones` (the amendment). Z0 shows the flag is load-bearing, so the build pins the B.Cu pour.

## Findings

| # | finding · where | type | recommendation | decision-needed |
|---|---|---|---|---|
| **F1** | **KiCad 10.0.7 writes the wall clock into every Gerber (2 lines) and the job file (1 line), with no option to stop it.** Measured G0–G2; read at source. `GERBER_plotter.cpp` `StartPlot`, `gbr_metadata.cpp` | §7.1 stop | rule (a) above | **yes** |
| F2 | **Build fork, for after the ruling.** The copy is named `input.kicad_pcb`, so `TF.ProjectId` and every file name say `input`. Naming the export's copy `<project>.kicad_pcb` would make the Gerbers name the project, with a GUID that is still path-independent (it is derived from the name). To be measured before relying on it | design | name the copy after the project in the export only; `check drc`'s copy name is unchanged | no |

**Evidence (not committed):** `~/.cache/aet-3300/spike/`, holding `PREDICTIONS.md` (written before the runs), `spike.sh`, `out/<config>/<A|B>-<n>/` (every file KiCad wrote, and the logs), and the probe crate.

**Not done, because of the stop:** §1.2 to §1.4, the reference probe, and an adversarial pass on code, since there is none. `TIER2_MIN` is unchanged, because no test was added.

## Adversarial pass (fresh subagent, against the §7 claim): not falsified

The brief was to falsify the stop, not to confirm it: make KiCad 10.0.7 write a byte-identical Gerber at two paths and two times by KiCad's own means. External clocks and editing the output were out of bounds. It used its own copies of the prepared boards and the same capped runner. Each config ran twice at each of two paths, more than 1.2 s apart, and every file came out with 4 distinct hashes (`~/.cache/aet-3300/adv/out/<cfg>.summary`).

- **Gerber flags not tried in the spike:** `--no-netlist`, `--disable-aperture-macros`, `--precision 5`, `--no-protel-ext`, `--use-drill-file-origin`, `-D CURRENT_DATE=… -D PLOT_DATE_TIME=…`, `--board-plot-params`, all of them together, and all of them with `--no-x2`. Lines 2 and 9 still carry the clock in each.
- **The board's own `(pcbplotparams …)`** with `usegerberattributes no`, `usegerberadvancedattributes no` and `creategerberjobfile no`, plus `--board-plot-params`, in three combinations. KiCad parses them, since the files become `.gbr`, but the dated lines stay, now as `G04 #@!` comments. `.gbrjob` is still written: the CLI ignores `creategerberjobfile`. In production the gate's setup block replaces any such block anyway.
- **`TZ=UTC LC_ALL=C SOURCE_DATE_EPOCH=1700000000`:** still dated.
- **`pcb export gerber` (singular):** does not exist in 10.0.7.
- **`kicad-cli jobset run` with a `pcb_export_gerbers` job:** it runs the same plotter and gives the same dated lines, written into a random-UUID directory.
- **Strings of `kicad-cli`, `_pcbnew.kiface` (the plotter) and `libkicommon.so.10.0.7`, in ASCII, UTF-16 and UTF-32:**
  - no `SOURCE_DATE`, `Reproduc`, `FixedDate` or `deterministic` (I re-ran the grep: no match);
  - the KiCad environment variables are directories and debugging switches only;
  - no advanced-config key concerns time or dates;
  - the disassembly of `GetISO8601CurrentDateTime` is `wxDateTime::GetTmNow` then `Format`, with no `getenv` and no branch.

**Spot-checked by the builder:** `F_bppx_P3`'s B.Cu at A-1 against B-2 differs in exactly lines 2 and 9, and the strings grep returns no match.

## Gates

- **Host:** `cargo fmt --all -- --check` passes. `cargo clippy --workspace --all-targets -- -D warnings` passes. `cargo test --workspace` passes, 936 tests with 0 failed, with `TMPDIR` a 0700 directory under `~/.cache`. A first run with `TMPDIR` in the session scratchpad under `/tmp` failed two `adversarial_1300` tests: 1601 refuses scratch under `/tmp`, as the kickoff says. That was the environment, not the diff.
- **Tier 2 / CI:** no code changed, so `gates`, `tools` and `image` run on the PR as they do on main.
- **Reduction pass (diff):** −3 rows net · 1 duplicate removed (F2–F4 restated "What the ruling unblocks") · 0 dead paths · 0 protected kept · 0 findings.
