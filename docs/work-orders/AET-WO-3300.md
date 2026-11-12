# AET-WO-3300 — The reference probe's fabrication package, emitted only by a passing `verify` and named in its verdict

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` after `AET-WO-1306`, named in the kickoff.
**Branch:** `wo/aet-3300-fab-package`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass (`docs/roadmap.md` §6).
**Phase:** P3. This work order closes **P3 gate clause 3**:

> "A fabrication package (Gerbers, bill of materials, placement file, firmware image) is emitted whose file hashes match the ones named in the passing verdict (`INV-03`)."

The bench comparison first named here is `AET-WO-3302` (reserved, human-track).

**Grounds:**
- **`aethelgard verify` (`AET-WO-3301`)** regenerates everything from a specification into scratch, runs every gate on exactly those bytes, and prints one verdict naming each artifact's hash.
- **Its gates now include:**
  - `extract` (`AET-WO-1305`), the capacitance bound against the declared bound;
  - the uid boundary (`AET-WO-1601`);
  - after `AET-WO-1306`, one strict reader for every KiCad input.
- **What exists already:**
  - the BOM (`kicad/<project>.bom.csv`, from `emit`);
  - the firmware ELF (`build`, path-independent, `AET-WO-3301` F4);
  - the routed board (`kicad/<project>.kicad_pcb`).
- **Missing:**
  - the Gerbers and drill files;
  - a placement file;
  - one package whose hashes a passing verdict names.
- **`AET-WO-1305` F3:** extraction's bound assumes copper of at most 70 µm and a board 0.34 to 2.0 mm thick. The board file declares neither, so the package must state them as fabrication instructions.
- **Scotty's ruling of 2026-11-09:** Sentinel-01's package is emitted and verified for the P3 gate, not ordered.

> **What this is:** `verify` emits the fabrication package only when every gate passes, and its verdict names every file of the package by hash.
>
> It is NOT:
> - ordering a board;
> - a bench comparison (that is `3302`);
> - a change to any generator's output.

**Cadence:** spike (§1.1), then build. Stop on §7. One PR, left unmerged.

## 1. Scope

### 1.1 Spike: are the fabrication outputs deterministic?

In the pinned image, run each of the following twice on the reference board (from `aethelgard generate`), at two different paths, and record every byte that differs:
- `kicad-cli pcb export gerbers`, every copper, mask, paste, silkscreen and edge layer the board uses;
- `kicad-cli pcb export drill`, Excellon;
- `kicad-cli pcb export pos`, CSV.

KiCad writes creation dates and paths in its Gerber X2 attributes and its drill headers. Find the options that remove them (for example `--no-x2`, or a fixed date), or a `SOURCE_DATE_EPOCH` KiCad honours. Prefer KiCad's own options to rewriting its output.

Then choose how the placement file is made, and record why:
- by `kicad-cli pcb export pos`; or
- by our own code from the strictly read board (the 1306 reader), which can be byte-stable by construction.

If no deterministic Gerber output is reachable with KiCad 10.0.7's options, that is a §7 stop. Do not post-process KiCad's files without the architect's ruling.

### 1.2 `aethelgard verify --spec S [--scenarios …] --fab-out D`

- Every gate runs as today. **Only if every gate passes**, `verify` writes the package into `D`. `D` must be empty or absent, and is written through descriptors, as `generate` writes (`AET-WO-3301`'s `tree::write`).
- **The package contains:**
  - the Gerbers and drill files from §1.1;
  - the placement file;
  - the BOM (`emit`'s, byte for byte);
  - the firmware ELF (`build`'s, byte for byte, the one `simulate` and the campaign ran);
  - `FAB-NOTES.md`;
  - `MANIFEST.json`.
- **`FAB-NOTES.md` is generated, not hand-written.** It states:
  - the stackup: layers, 1.6 mm declared thickness, FR-4;
  - copper of at most 70 µm, and a finished thickness of 0.34 to 2.0 mm (`1305` F3: the conditions the capacitance bound rests on);
  - the minimum track, space, via and drill, from the router's constants;
  - the specification's sha256;
  - the sentence that the package was emitted by a passing `verify`, and nothing beyond that (`RUL-19`).
- **`MANIFEST.json`** lists every other file with its sha256, the specification's and topology's sha256, and the verdict's own sha256.
- **The verdict names every package file's sha256,** under a new `fab` key. Those hashes equal the files written: the clause's text. The verdict is still printed whether the run passes or fails. A failing run writes nothing into `D`, and says so.
- **Gerbers go through the sandbox** like every `kicad-cli` call (`AET-WO-1601`), with the board's copy hashed and bound to the generated board.
- **Exit codes** follow `RUL-10`:
  - 0 when the run passes and the package is written;
  - 1 for a gate failure, with nothing written;
  - 3 for an unusable `--fab-out`;
  - 4 for a missing tool;
  - 5 for a tool fault.

### 1.3 `aethelgard verify --spec S --against-fab D`

This re-derives the package in scratch and compares it file by file against `D`. Exit 1 names every file that differs, is missing or is extra, as `--against` does for the tree. This is how a later reader checks that a package on disk is the one a passing verdict named.

### 1.4 The reference probe

Run `verify --spec examples/sentinel-01-minimal.json --scenarios scenarios/sentinel-01/pullup-2k2.json --fab-out <dir>` in the image.
- Two runs at different paths must give byte-identical packages and verdicts.
- Record the package's file list, sizes and hashes in FEEDBACK.
- Do **not** commit the package. It is reproducible from the specification, and its hashes in FEEDBACK are the record.

## 2. Invariants and fences

- `INV-01`, `INV-03`, `INV-04`, `INV-06`, `RUL-10`, `RUL-11` (the package's licence note), `RUL-19`. No `unsafe`.
- **Fence:**
  - the CLI's `verify.rs`, its fab module, `main.rs`'s flags, `mcp.rs`'s pinned lists and the CLI README;
  - `aethelgard-eda`, for the placement and the fab-notes text only;
  - tests;
  - this work order's FEEDBACK.
- **Every committed generated file stays byte-identical,** and every existing gate's verdict is unchanged apart from the new `fab` key on a `--fab-out` run.
- `TIER2_MIN` rises by exactly the tier-2 tests added.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance

- **§1.1's determinism table is in FEEDBACK,** with each prediction beside its result.
- **The reference probe:**
  - `verify … --fab-out` passes, writes the package, and the verdict's `fab` hashes equal the files' sha256;
  - two runs at two paths give identical bytes;
  - `--against-fab` on the package passes, and one changed byte in any file fails, naming that file.
- **The 47 kΩ scenario fails** `verify` and writes nothing into `--fab-out`.
- **The firmware in the package** is the ELF the campaign ran (its hash is in both places). The BOM is `emit`'s.
- **`FAB-NOTES.md`** states every condition of `1305` F3, the stackup and the rule minima, and is regenerated byte-identically.
- **The gates** (`CLAUDE.md`) pass, and `gates`, `tools` and `image` are green in CI.

## 5. Adversarial pass — full

1. **Get a package written by a run that failed a gate,** or one whose files differ from the hashes its verdict names: races, a pre-filled `--fab-out`, a symlink, a partial write.
2. **Make two runs on one specification disagree** on any package byte.
3. **Find words in `FAB-NOTES.md`, the README or the verdict that claim more than the gates show** (`RUL-19`): a board-level electrical claim beyond `extract`'s bound, or a claim that anything was tested on hardware.

## 7. Flag-and-stop conditions

- KiCad 10.0.7 cannot be made to write deterministic Gerbers or drill files with its own options.
- Any generator's output, or any existing verdict, must change.
- The firmware the package would carry is not the one the campaign ran.
