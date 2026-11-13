# AET-WO-3300 — FEEDBACK

**Branch:** `wo/aet-3300-fab-package` · **Base:** `origin/main` 8f04772 (`AET-WO-1306`); `origin/main` 72af687 (docs PR #102, which carries the §1.1 amendment) merged in as 4013e3d · **Builder:** Claude Code (Opus 5.5) as `forge` on Furnace · **Cadence:** spike (§1.1, as amended 2026-11-13), a §7.1 stop, the architect's ruling (a) on PR #103, then build.

**Outcome: READY FOR REVIEW.**
- **The spike stopped on §7.1.** KiCad 10.0.7 writes the wall clock into two lines of every Gerber, and none of its own options removes them (F1).
- **The architect ruled (a)** (PR #103, comment 6843504048): delete exactly those two lines, line-exact. The build follows the ruling's nine conditions, with one correction to condition 1, which needs confirming (D1).
- **The package:** `verify --fab-out D` writes it only when every gate passes, and the verdict names every file's sha256 under `fab`. `--against-fab D` derives the package again and names every file that differs.
- **The reference probe passes,** with identical bytes and verdicts at two paths. The 47 kΩ scenario writes nothing.
- **Process:**
  - a fresh subagent ran the full adversarial pass on the code; its must-fix and should-fix items are fixed or ruled out below;
  - every new guard was mutated once, and every mutant was killed;
  - the CLAUDE.md gates and the tools job pass (§Gates).

Entry schema: `finding · where · type · recommendation · decision-needed`.

## Decisions needed first

- **D1 (F3): the outline's creator line is line 8, not line 9.**
  - Ruling condition 1 says line 9, because my spike table said "line 2 and line 9" for every layer. I had diffed F.Cu alone.
  - In fact KiCad writes no `%TF.FilePolarity` into the Edge.Cuts header, so its creator line is line 8, which the first image run of the build found.
  - As built, each layer's position is pinned as measured in `fab::LAYERS`: 9 for eight layers, 8 for Edge.Cuts. A second committed real header (`tests/fixtures/fab/kicad-10.0.7-Edge_Cuts-header.gm1`) pins it, and each layer is refused at the other's position.
  - Everything else in conditions 1 and 2 holds as written. **Confirm the correction**, or name another rule.
- **D2 (F4): `MANIFEST.json`'s `verdict_sha256` binds the run's environment and arguments.**
  - The verdict states every gate's sandbox (`uid_boundary_unmet`), so a run as root under rootless podman, a run as uid 1000, and CI's Docker each print a different verdict. The same holds for other `--scenarios` or `--against` arguments.
  - So `--against-fab` from another environment names `MANIFEST.json` as differing while the other 14 files match. The adversarial pass measured this: runs A and D differ only in `verdict_sha256`.
  - As built, `--against-fab` stays byte-exact (§3: one changed byte in any file fails), and this is documented. The manifest now records the scenarios (name and sha256), and FAB-NOTES' re-check instruction says to use the same arguments.
  - **Alternative:** have `--against-fab` compare `verdict_sha256` separately and report it apart from the files. Choose.
- **D3 (F5): `--fab-out` without `--scenarios` writes a package.** §1.2 makes the scenarios optional, so the build allows it, and FAB-NOTES claims no campaign. `INV-03` names the fault campaign among the three gates of "verified", though. **Should `--fab-out` require `--scenarios`?**
- **D4 (F6): the manifest and the verdict cannot name each other's hashes.** §1.2 asks for both. As built:
  - the verdict's `fab.files` names every file, `MANIFEST.json` included;
  - `MANIFEST.json` names the sha256 of the verdict the same call prints without the fab flags. That is the printed verdict without its final `,"fab":{…}`, since `against_fab` and `fab` are serialized last.
  - Tier 2 checks the relation exactly. **Confirm.**

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
| G0 | `pcb export gerbers --layers … --check-zones` | differs in `%TF.CreationDate` and the `G04 Created by KiCad … date` line; `.gbrjob` `CreationDate`; no path anywhere | **all 10 files: 4 distinct.** Each layer differs in exactly 2 lines: line 2, `%TF.CreationDate,2026-11-13T03:32:32+00:00*%`, and the creator line, `G04 Created by KiCad (PCBNEW 10.0.7) date 2026-11-13 03:32:32*`. The creator line is line 9 in eight layers and line 8 in Edge.Cuts, whose header has no `%TF.FilePolarity` (*corrected at build, F3: this row first said "line 9" for every layer, from a diff of F.Cu alone*). The job file differs in 1 line, `"CreationDate"`. No output byte depends on the path: no output contains A's or B's directory names. With those lines removed: 1 distinct, for every file | yes |
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

None of these files reads an environment variable or `SOURCE_DATE_EPOCH`. `common/advanced_config.cpp` has no key about dates or reproducibility. `--board-plot-params` reads the board's plot settings, and none of those is a date. So no KiCad 10.0.7 option makes the Gerbers deterministic: **§7.1**, ruled on PR #103 (F1).

**Also observed.**
- Every export writes `input.kicad_prl` and `.config/kicad/10.0/*.json` into the workdir (the tool's `HOME`). That is where the sandbox already confines KiCad, and none of it is an output.
- `%TF.ProjectId,input,696e7075-742e-46b6-9963-61645f706362,rev?*%` names the copy's stem. Its GUID is the stem's bytes (`696e7075742e` is `input.`), so it does not depend on the path.

## The ruling, condition by condition (PR #103)

| # | condition | where it holds |
|---|---|---|
| 1 | Line-exact: line 2 `%TF.CreationDate,<ISO 8601 with offset>*%`, the creator line `G04 Created by KiCad (PCBNEW 10.0.7) date <YYYY-MM-DD hh:mm:ss>*`, each once and at its place, deleted and never rewritten | `fab::strip_clock`: exact shapes, line 1 `%TF.GenerationSoftware,KiCad,Pcbnew,10.0.7*%` pinned too. The creator line is line 9, or 8 for Edge.Cuts (D1) |
| 2 | Any departure is `ToolError` exit 5 with nothing written: a missing or moved line, a second dated line anywhere, another version | `strip_clock` refuses all of these. It also refuses any other line holding `CreationDate`, `Created by`, `dddd-dd-dd` or `dd:dd:dd`, a non-ASCII file, and a `\r`. The package is written only after every file is derived |
| 3 | A tier-1 test pins the rule on a committed real header and on each departure | `fab::tests`: `a_real_header_loses_exactly_its_two_clock_lines` (F.Cu and Edge.Cuts headers, the first 20 lines KiCad wrote in the spike) and `every_departure_from_the_measured_header_is_refused` (17 departures) |
| 4 | FAB-NOTES says the Gerbers are KiCad 10.0.7's output from the prepared copy with `--check-zones`, with the two creation-date lines removed | `aethelgard_eda::fab::notes`, "What this package is"; asserted in eda's tests and tier 2 |
| 5 | No job file; FAB-NOTES states the stackup | the package has no `.gbrjob`; "Stackup" |
| 6 | `--excellon-min-header`; FAB-NOTES: every hole a plated 0.3 mm via, metric, decimal | the statement is backed by checks: no pad of the strict reading has a drill (`no_drilled_pad`), and the drill file is metric, with exactly one tool `T1C0.300` and one hit per via (`holes`) |
| 7 | `pos --format csv --units mm --side both` | as given. Each row is also checked (`placement_rows`, adversarial G) |
| 8 | A test fails if B.Cu has no `G36`/`G37` region under `%TO.N,GND*%` | at run time, for every zone the strict reading declares (`pours`, exit 5); in tier 2 on the package's B.Cu; in unit tests |
| 9 | F2 only if measured path-independent | **not taken.** It would have renamed the gate's prepared files inside the export, and it changes only the cosmetic `TF.ProjectId,input`. The package's file names carry the project anyway |

## §3 acceptance

| §3 line | evidence |
|---|---|
| §1.1's table in FEEDBACK, predictions beside results | above (written before any code; predictions in `~/.cache/aet-3300/spike/PREDICTIONS.md`, timestamped before the runs) |
| `verify … --fab-out` passes and writes the package; `fab` hashes equal the files' sha256 | tier 2 `verify_fab_out_writes_the_package_its_verdict_names_at_any_path`; the reference probe below |
| two runs at two paths give identical bytes | the same test (`a` against `a-much/longer/path/b`: identical verdict lines and identical files); the probe below |
| `--against-fab` passes, and one changed byte in any file fails, naming it | the same test: `identical` then, after one byte flipped in each of the 15 files and one extra file, `differs`, with 15 `against_differs` and 1 `against_extra`, each naming its file |
| the 47 kΩ scenario fails and writes nothing | tier 2 `verify_fab_out_writes_nothing_when_a_gate_fails`: exit 1, `fab.status` `not_written`, the directory still empty |
| the firmware is the ELF the campaign ran; the BOM is `emit`'s | the package's ELF is read back and bound to `elf_sha256`, which `simulate` and the campaign's run id are bound to (`BindingMismatch` otherwise). Tier 2 asserts `fab.files[<project>.elf] == elf_sha256` and that the BOM equals `emit`'s file byte for byte |
| FAB-NOTES states F3's conditions, the stackup and the rule minima, and is regenerated byte-identically | eda's `the_notes_state_every_condition_the_bound_rests_on`, which checks every one of `extract::CONDITIONS` verbatim; tier 2's two-path identity |
| the gates pass; `gates`, `tools` and `image` green in CI | §Gates |

## §1.4 The reference probe

`aethelgard verify --spec examples/sentinel-01-minimal.json --scenarios scenarios/sentinel-01/pullup-2k2.json --fab-out <dir>` ran at 4013e3d in the pinned image (`c69ff9bc`), as root under rootless podman with `TMPDIR` unset, at two paths: `/probe/a` and `/var/lib/x/a-much/deeper/path/b`.
- **Both exit 0.** The two verdict lines are byte-identical (sha256 `8dffc623a1159c5826c67b3fce7c9d19b579a36be7a14652938a65a6af1bbc9f`), and so are the two packages (`diff -r`).
- **`fab`:** `status` `written`, `tool_version` `10.0.7`, `board_sha256` `7b9b0e3d…2070` (the generated board).
- **`uid_boundary`** is `false` with `user_namespace`, as every gate's is under rootless podman (`AET-WO-1601`).
- **Every `fab.files` hash equals the sha256 of the file written.**
- **Three hashes match the spike:** the ELF is `2d9ee264…`, the reference firmware of `AET-WO-3301`; the drill (`a045ab03…`) and placement (`5056acf6…`) files are the spike's D1 and P0.
- **Environment:** the hashes of `MANIFEST.json` and of the verdict are this environment's (D2). Every local run had `uid_boundary: false`.
  - **A prediction, not observed:** under CI's Docker, where the boundary can hold, those two hashes differ and the other 14 files do not.
  - CI's tools job is the first run of the export under a held uid boundary. Its tier-2 test asserts only that `fab.uid_boundary` equals the DRC gate's, whatever value that is.

The package is not committed. These hashes are the record:

| file | bytes | sha256 |
|---|---|---|
| `FAB-NOTES.md` | 4,253 | `e77ca25bda5333da49e33541c3aba5edf5aaf5a36b41dfaab3045d72fe221cea` |
| `MANIFEST.json` | 1,823 | `f1369a96e4de2654f5299f76d30bb680560378dfb56a4a4685b038a026db3d79` |
| `sentinel-01-B_Cu.gbl` | 40,806 | `09b4ef31443814cd075146d335327c598433329be4d6f0b3fcd94a715af48293` |
| `sentinel-01-B_Mask.gbs` | 347 | `8506d631dae979f7d6457a62d8824d82ad058d25b3fa680aa8bedb4c7db39bc4` |
| `sentinel-01-B_Paste.gbp` | 342 | `0d3db1fa132062fd09981dccb423680a458f024a145d0923b3c34fa271525be0` |
| `sentinel-01-B_Silkscreen.gbo` | 343 | `c33d6c7655263578ed865ab787c5ff4d141f3a821500e9293b7492099005c09e` |
| `sentinel-01-Edge_Cuts.gm1` | 496 | `7ef00338d9e4f167bf906a744eec7b07a83209da8c6121def43db1cf1ac4aedb` |
| `sentinel-01-F_Cu.gtl` | 31,686 | `70de7722c0de56ea09973a50d882487d6270400ebefa2668702c733f7247d4d0` |
| `sentinel-01-F_Mask.gts` | 6,583 | `403c2deffb199e076006a9ca32374176396a0929e9c8e2f2eb34ec3613370557` |
| `sentinel-01-F_Paste.gtp` | 6,679 | `db3e59d1e40086af8764c3d6aa05b467868f1292a6f74a6c99fd02872e17ab1b` |
| `sentinel-01-F_Silkscreen.gto` | 19,507 | `4b313e3d2df325d20ff2f43619c8052d25b142565380d678ae3c9c7629e2869b` |
| `sentinel-01-pos.csv` | 1,115 | `5056acf6332b35ce64cd0845e2abf17cd7bd7a74a6ed11d72642e89c5e21ad73` |
| `sentinel-01.bom.csv` | 1,046 | `5ae3521299618066d99bb66ec45d7014f260a43edcc0487076ae7008e26a8799` |
| `sentinel-01.drl` | 1,059 | `a045ab03f8c0e00d787687f9e4a3a9144a2830864b7e7a665ff9faae85a7bc8d` |
| `sentinel-01.elf` | 131,776 | `2d9ee2641f6e0a1b6e48f1637cefe4fa01cab3f97849a1dd074474c879fe0d2f` |


## Forks taken

| # | fork · where | why |
|---|---|---|
| F7 | **Fence: `tree.rs` and `strict_fence.rs`.** `tree::write` takes any ordered `(name, bytes)` sequence and `tree::diff` any byte map: two signature changes, with every existing caller unchanged. `strict_fence.rs` gives the two new modules roles (`eda/src/fab.rs` Other, `cli/src/fab.rs` Gate) | the package carries a binary ELF and must write `MANIFEST.json` last; a second writer would duplicate `tree::write`'s descriptor discipline |
| F8 | **The export runs in `verify`'s process**, not as a subcommand. Its tool version and sandbox report go into `fab` (`tool_version`, `uid_boundary` being the weakest of the three exports, `uid_boundary_unmet`), not into `gates` | §2: every existing verdict is unchanged apart from the new `fab` key; there is no new subcommand |
| F9 | **A missing pour, a drilled pad or a bad drill or placement file is exit 5 (`ToolError`), not exit 1** | each means KiCad's output is not what the package states; the gates already passed the design |
| F10 | `--fab-out` and `--against-fab` exclude each other (clap, exit 64). Each may be combined with `--against`, whose result is part of the verdict the manifest names. A differing `--against` gives `not_written`/`not_derived` (adversarial review) | |
| F11 | **An existing `--fab-out` must be the caller's and writable by no group or other**, checked before the gates and again just before the write (adversarial A1). On a host with umask 002, a directory made with `mkdir` is refused; an absent one is created by the write | the rule scratch already follows (§6) |
| F12 | **Not taken: a `kicad-cli version` guard.** `strip_clock` pins 10.0.7 in line 1 and the creator line of every Gerber, so a version guard was a duplicate (reduction pass). The version is still reported in `fab.tool_version` | |
| F13 | **The package is flat**, every file named after the project: `<project>-<KiCad layer suffix>`, `<project>.drl`, `<project>-pos.csv`, `<project>.bom.csv`, `<project>.elf`, `FAB-NOTES.md`, `MANIFEST.json` | what a fab house uploads |

## Findings

| # | finding · where | type | recommendation | decision-needed |
|---|---|---|---|---|
| **F1** | **KiCad 10.0.7 writes the wall clock into every Gerber (2 lines) and the job file (1 line), with no option to stop it.** §1.1; read at source | §7.1 stop | **ruled (a)** on PR #103 | done |
| F2 | Naming the export's copy after the project (spike F2) | design | not taken (condition 9) | no |
| **F3** | The outline's creator line is line 8 | correction | D1 | **yes** |
| **F4** | `verdict_sha256` binds the environment and the arguments | design | D2 | **yes** |
| **F5** | `--fab-out` without `--scenarios` | scope | D3 | **yes** |
| **F6** | The manifest/verdict cycle | design | D4 | **yes** |
| F14 | **A specification's own text can put a date into a Gerber.** A net named `2026-11-13` passes every gate; the export then refuses (exit 5, "line N carries a date or a time of day (a name from the specification can put one there)"). It fails closed, but the place to refuse is identifier validation | residual | backlog: narrow `RUL-14`'s identifiers (no date shapes, no `"`) so the generator never emits them | no |
| F15 | **KiCad does not escape `"` in a placement row** (adversarial G: a reference `U"4,X` shifted every later column). Now refused at export, exit 5. As with F14, the real home is identifier validation | fixed / backlog | as F14 | no |
| F16 | Two guards have no mutant any suite kills: the export's ELF and board `BindingMismatch` checks. A test would need a swap race inside `verify`'s scratch, as 1305's extract test does | test gap | backlog, or accept as stated | no |
| F17 | `holes()` and `pours()` are shape checks of KiCad's own output, not parsers. The adversary showed inputs KiCad never writes that pass them: a hit without X, an undefined `T2` in the body, a later region's `G37`. Other date notations (`2026/11/13`, an epoch) also pass `strip_clock`. The guard against a date left in is tier 2's two-run, two-path identity | residual | none now (scope test) | no |
| F18 | An extra empty subdirectory in a package still reads `identical` under `--against-fab`, as `--against` treats a tree | residual | none | no |
| F19 | **Protected documents this build makes stale** (for the architect's reconciliation): `README.md:13`, "extraction from the board is scheduled before any fabrication package is called verified"; `docs/architecture.md:404`, where §3 puts "fabrication package emission" in `aethelgard-eda` (the export is the CLI's `fab` module, and eda holds only the notes' text); `RUL-19` (2), "the P3 fabrication clause waits for it" and "waits for `1601` as well"; and the `roadmap.md:405` backlog row about refilled zones and the stackup, which this WO answers | doc | the architect updates them | yes (reconciliation) |
| F20 | **FAB-NOTES states copper, thickness and permittivity twice:** under Stackup, and again in `extract`'s conditions printed verbatim. And "No board has been made from it" stops being true once a board is ordered | reduction / wording | one statement from `extract::CONDITIONS`; say "No board made from it was tested by aethelgard" | no |
| F21 | **`fab::LAYERS` and `fab::describe` both map each layer:** the first to KiCad's file suffix, the second to its description | reduction | fold the description into `LAYERS` | no |


## Adversarial pass on the build (fresh subagent, Opus, full)

The brief was to break it, with §5's three items and the ruling's three attacks on the stripping. It ran against a binary snapshotted from commit 9eb58a3 in the pinned image, with runs A to M of `verify`, a racer as a third uid, and 3 million fuzzed headers. Its evidence is in `~/.cache/aet-3300-adv/`.

The builder then re-ran its attacks I and G, and its static and race controls, against a binary built from a271a73 (`scripts/rerun2.sh`, `logs/rerun2-*.out`), with each exit code captured directly:
- **I:** exit 3, `UnwritableOutput`, nothing written.
- **G:** exit 5, `ToolError`, placement row 12, nothing written.
- **Pre-filled directory, symlink at D, and a symlink swapped in during the gates:** each exit 3, nothing written.
- **A symlinked path component:** exit 0, 15 files (path semantics).
- **The positive control and M** (`scripts/rerun.sh`, `logs/rerun-A.out`): the same flags give `identical`; without `--scenarios`, only `MANIFEST.json` differs.

| attack | found | severity | now |
|---|---|---|---|
| **§5.2 two runs disagree** | `MANIFEST.json` differs between a run as root under rootless podman and one as uid 1000: only `verdict_sha256`, via each gate's `uid_boundary_unmet`. FAB-NOTES' re-check command had no `--scenarios`, so it failed on a genuine package. Varying the path, `TMPDIR`, `TZ`, cwd, umask 077 and `SOURCE_DATE_EPOCH` changed nothing | must-fix (ruling) | **D2.** The manifest records the scenarios, and the instruction names the same arguments and sandbox. Re-run: the same flags give `identical`; without `--scenarios`, only `MANIFEST.json` differs, as documented |
| **§5.1 another uid** | uid 65534 pre-created an empty `--fab-out` it owned; root's run wrote into it and the racer replaced F.Cu afterwards | should-fix | **fixed** (F11). Re-run: exit 3, `UnwritableOutput`, before any gate, nothing written |
| §5.1 same uid | appending to the ELF after the manifest lands; a planted `MANIFEST.json` leaves a complete-looking package (exit 3, no verdict) | residual | `RUL-19` (4): a process of the caller's uid is not a boundary. No verdict names those bytes |
| §5.1 static | a pre-filled directory, a symlink at D, a symlink swapped in during the gates: each exit 3, nothing written. 47 kΩ: `not_written`, D never created | none | — |
| **§5.3 words** | FAB-NOTES listed a subset of the bound's conditions as "the conditions"; it said the Gerbers came from "the copy the gate judged" (it is a second copy with that hash); the README showed the failed-gate `fab` without its null fields | should-fix | **fixed:** `extract::CONDITIONS` printed verbatim, the copy reworded, the README corrected. No claim of hardware testing or measurement was found |
| §4a a date left in | 1.59 million accepted mutations of the real header, each losing exactly lines 2 and 9. Other date notations pass and are kept | residual | F17 |
| §4b a stripped line not a date | none: both lines must match the exact shape | none | — |
| §4c a crafted specification | a dated net name passes every gate and then fails closed at export; `G04*%` is escaped by KiCad; nothing reaches line 2 or 9 | residual | F14 |
| other: placement | a reference `U"4,X` passed and wrote a `pos.csv` with shifted columns | should-fix | **fixed** (F15). Re-run: exit 5, `ToolError` at row 12, nothing written |
| other: shape checks | `holes()` and `pours()` accept inputs KiCad never writes | residual | F17 |

**Its disclosures:** it ran `git diff` twice (read-only) against the brief, and it used `mv` where the removal check blocked `rm`.

## Mutation (sized to the diff)

There was one mutant per guard this WO adds, run against the suite the guard lives in, on a copy of the tree (`~/.cache/aet-3300/mut`):
- **`strip_clock`:** line 1, line 2's shape, the creator line's shape, the creator line's index, other dated lines (`CreationDate`/`Created by`, then `dated()`, then its time branch), `\r`, ASCII.
- **`pours`:** the `G36` requirement.
- **`holes`:** the tool table, the hit count, `METRIC`.
- **`no_drilled_pad`.**
- **`fab_out_usable`:** non-empty, not a directory, the owner, the mode.
- **`placement_rows`:** the quote inside a field, the plain fields.
- **`--against`'s `not_written`.**

Two more were added at the close:
- `strip_clock`'s complete-lines check;
- the placement header.

**23 of 23 killed:**
- One `pours` mutant first survived. A test case was added (a `G37` with no `G36`), and it was then killed.
- The owner mutant survives on the host, where tests run as `forge`. It is killed as root in the image, where CI runs tier 1.

**Guards with no mutant run, because no suite could kill one without a race or a broken tool** (F16):
- the export's ELF and board `BindingMismatch` checks;
- the second `fab_out_usable` call just before the write;
- `read_output`'s regular-file and size checks;
- `weaker`, which picks the weakest sandbox report of the three exports.

## Adversarial pass on the §7 claim, before the ruling (fresh subagent): not falsified

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

- **Host, at 4013e3d:**
  - `cargo fmt --all -- --check` passes.
  - `cargo clippy --workspace --all-targets -- -D warnings` passes.
  - `cargo test --workspace` passes: 953 tests, 0 failed, with `TMPDIR` a 0700 directory under `~/.cache`. Since `AET-WO-1601`, scratch under `/tmp` is refused; a first spike-time run that put `TMPDIR` in the session scratchpad failed two `adversarial_1300` tests for that reason.
- **The CI tools job, reproduced in the pinned image at 4013e3d** (rootless podman, `--memory 16g --memory-swap 16g --pids-limit -1`, then 0 written to `memory.swap.max`; read back `memory.max=17179869184`, `memory.swap.max=0`, `pids.max=max`; `TMPDIR` unset; no edits during the run):
  - build, the bridge's tests, fmt, and clippy with tier 2 all pass;
  - `cargo test --workspace --features aethelgard-cli/with-tools` passes: 1,025 tests, 0 failed;
  - **the tier-2 canary counts 77, and the floor is now 77.** `TIER2_MIN` was 75 and rises by exactly the two tier-2 tests added in `board_tier2.rs`, with a comment in `ci.yml`.
  - The two containers that ran in under a second (the bridge's tests and fmt) ended before the cap watcher could read their cgroup.
  - An earlier run, before the adversarial fixes, failed one assertion of the new tier-2 test: FAB-NOTES names the removed lines in prose, and the test searched every file instead of the Gerbers alone. The test was fixed.
- **CI on the PR:** see the PR's checks. A green CI is not live acceptance.

## Reduction pass (diff)

**Run twice:** after the first build, and again over the final diff at a271a73. That made 2 duplicates removed · 0 dead paths · 0 protected kept · 2 findings (F20, F21: recorded, not changed, because changing them now would stale the probe's hashes and the tools run).

The two duplicates removed:
- the `kicad-cli version` guard, which `strip_clock`'s line-1 and creator-line pins already enforce;
- a runtime check that the notes list the package's files, now asserted once in tier 2.

The spike-time FEEDBACK's rows that restated "What the ruling unblocks" were cut earlier.

**Evidence (not committed):**
- the spike: `~/.cache/aet-3300/spike/`;
- the adversarial pass on the §7 claim: `~/.cache/aet-3300/adv/`;
- the adversarial pass on the code, with the builder's re-runs: `~/.cache/aet-3300-adv/`;
- mutation: `~/.cache/aet-3300/mut/`;
- the reference probe: `~/.cache/aet-3300/probe-final/`;
- the runner: `~/.cache/aet-3300/img.sh`.
