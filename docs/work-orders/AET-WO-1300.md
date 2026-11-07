# AET-WO-1300 — The schematic emitter, the gate's own DRC rules, and how the board gets placed and routed

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` after `AET-WO-1200` has merged (named in the kickoff; verify live HEAD before branching).
**Branch:** `wo/aet-1300-schematic`.
**Author:** Claude (architect) · **Builder:** a **strong** builder, with a full pass on the DRC rule injection (`docs/roadmap.md` §6). A fresh session on Furnace.
**Phase:** P1 · **Gate clauses served:**
- **Clause 1, schematic half:** `kicad-cli sch erc --exit-code-violations` exits 0 on the emitted schematic.
- **Clause 1's rule condition:** DRC judges against rules the gate derives from the specification.
- **Clause 4, schematic half:** regeneration is byte-identical.

Placement is `AET-WO-1301`, and routing with the board's DRC is `AET-WO-1302`. This work order's spike scopes both.

**Grounds:**
- `docs/architecture.md` §4:
  - `RUL-03`, as amended: emit KiCad S-expressions directly; `--exit-code-violations`; and the gate supplies every rule-bearing file. It overwrites the board's `(setup …)` and writes the project's design settings and a `.kicad_dru` from the specification;
  - `RUL-11`;
  - `RUL-16`.
- `docs/architecture.md` §2.4 and §2.5, the KiCad measurements.
- `docs/roadmap.md` §5, the `1300`, `1301` and `1302` lines. The routing leads, all unverified:
  - `kicad-cli` 10 has no Specctra DSN export;
  - Freerouting would need Java and Python outside Renode;
  - the prior is a deterministic Rust grid router;
  - zones may need their filled polygons written by the emitter.
- `docs/work-orders/AET-WO-1100.FEEDBACK.md`, A3. Identifiers allowed by `RUL-14` can still carry characters an S-expression must quote: `"`, `\`, `(`, `)`. The emitter must quote or escape correctly.
- `docs/northstar.md` §4: `INV-01`, `INV-03` and `INV-07`.
- The seams:
  - `crates/aethelgard-eda` (a stub);
  - `crates/aethelgard-cli/src/kicad.rs` and `check drc` (where the rule injection lands);
  - `container/fixtures/`;
  - the KiCad 10.0.7 image.

> **What this is:** the first emitter. It turns a `SolvedTopology` plus its `Specification` into a `.kicad_sch` and a bill of materials, ERC-clean and byte-deterministic. `check drc` also learns to judge a board against rules it writes itself from the specification. The spike also settles how placement and routing will be done.
>
> It is NOT:
> - the `.kicad_pcb` beyond what the spike needs to measure;
> - placement or routing, which are `1301` and `1302`;
> - parasitic extraction;
> - fabrication output.

**Cadence:** spike first, then STOP for the architect's ruling, then build, all in one session. One PR for both halves, left unmerged.

## 0. Spike — the questions it must answer

Record every answer in `docs/work-orders/AET-WO-1300.FEEDBACK.md` under "Spike", with the CI run or command that measured it. KiCad exists only in the image, so these are proven in CI's `tools` job (`docs/roadmap.md` §6). Then STOP.

- **S1 Symbols and footprints.** Which KiCad 10 library symbols and footprints exist in the pinned image for:
  - the STM32H753ZI in LQFP144;
  - the BME280, the TMP117 and the W25Q64JV;
  - 0402 or 0603 resistors and capacitors.

  Record their licence (KiCad's library licence and its exception for designs). Embedding the symbols in the schematic's own `lib_symbols` makes the file self-contained; say whether that is allowed, and whether it is deterministic. Ask the same of footprints. Where a part is missing, name the smallest honest substitute.
- **S2 A minimal ERC-clean schematic.**
  - Emit, from Rust, a schematic with the MCU, one I2C sensor, its two pull-ups and decoupling, and one rail.
  - Run `aethelgard check erc`. Record every finding, and what the emitter must add to clear it: power flags, no-connect markers on unused pins, pin types.
  - KiCad needs a UUID on every item. Derive each one deterministically from content, and show two emissions to be byte-identical.
- **S3 The gate's own rules** (`RUL-03`, `INV-07`).
  - Write a small board whose own files carry looser rules: a 0.05 mm clearance in its setup block, and its own `.kicad_dru`. Give it a violation that the specification's rules catch and its own rules allow.
  - Show that a gate which overwrites the setup block, writes the project's design settings, and writes its own `.kicad_dru` into the scratch copy makes `kicad-cli pcb drc` report the violation.
  - Show that, without those steps, it does not.
  - Record which of the three steps is load-bearing, measured by removing each in turn.
- **S4 Placement and routing, for `1301` and `1302`.** Verify the roadmap's leads:
  1. whether `kicad-cli` 10.0.7 can export or import Specctra DSN or SES;
  2. whether KiCad's Python `pcbnew` module exists in the image. Do not use it; `RUL-12` limits Python to inside Renode;
  3. whether `kicad-cli pcb drc` refills zones, or whether the emitter must write filled polygons;
  4. as a proof of concept of the Rust-router prior, emit a two-layer board that places two footprints and routes two nets on a grid, from Rust. Report whether DRC with `--schematic-parity` passes with zero unconnected items, and what parity required of the schematic and board: matching references, footprint library ids, and net names.

  Recommend the shape of `1301` and `1302` from the measurements.
- **S5 Escaping.** For every identifier class the specification allows, emit a schematic using a name that carries each S-expression-special character `RUL-14` admits. Show that KiCad reads it back unchanged, using the ERC report's names. This is `AET-WO-1100`'s FEEDBACK A3.

**STOP** when §0 is answered: FEEDBACK's Spike section written, the PR open, every answer cited. The architect rules on the PR and gives the build goal in the same message.

## 1. Scope — the build half, after the ruling

Shaped by the spike. Its intent:

1. **`aethelgard-eda`:** `SolvedTopology` + `Specification` → `.kicad_sch` + bill of materials. The output is deterministic, with no `unsafe`, and `aethelgard-eda` graduates from the stub checks as `chaos` and `svd-hal` did.
2. **`check drc --spec <file>`:** the gate derives the clearance, track-width and isolation rules from the specification, and writes every rule-bearing file into the scratch copy before DRC runs (`RUL-03`). The board's own rule files are never used.
3. **`aethelgard emit --file <spec> --out <dir>`**, if the ruling adopts it: validate, solve, emit. As a command-tree leaf, it becomes an MCP tool automatically.
4. **Tests:**
   - ERC passes on the reference probe's schematic;
   - two regenerations are byte-identical;
   - S3's looser-rules board is refused under `--spec`;
   - S5's names survive the round trip.

## 2. Invariants and fences

- **`INV-07`:** the rules that judge come from the specification, never from the artifact.
- **`INV-01`:** the output is a function of the specification and the table alone.
- **No `unsafe`. No Python. No Java.**
- **Protected surfaces diff to empty:**
  - the steering documents, `README.md`, `LICENSE-*`, `docs/inputs/**`;
  - `container/`, except new fixtures under `container/fixtures/` that the spike needs, each recorded in FEEDBACK;
  - `.github/workflows/`.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route:** a `grep -o` redirect from your transcript into `gh-write prep`'s file, never the Write tool.

## 3. Acceptance

1. Every §0 answer is in FEEDBACK with its CI run.
2. After the build, §1.4 passes in `gates` and `tools`.
3. CI is green on the first push after the ruling.

## 4. Scope fence

Placement and routing beyond S4's proof of concept; parasitics; Gerbers; other parts; and any change to the tools image.

## 5. Adversarial pass — full, on the rule injection

1. Make DRC judge by the board's own rules despite `--spec`: a `.kicad_pro` that is a symlink, a `.kicad_dru` under another name, rules embedded in a footprint, or a net class defined in the board.
2. Make two emissions differ.
3. Make a name break the S-expression.

## 5a. Reduction pass — diff-sized

Run the `reduction-pass` skill on the finished diff. One line in FEEDBACK's gates.

## 6. Upward-feedback directive

`docs/work-orders/AET-WO-1300.FEEDBACK.md`, same schema. It carries:
- the S1 licence facts;
- the S3 load-bearing step;
- the S4 recommendation for `1301` and `1302`;
- the gates line;
- a promote-or-needs-none pass over `AET-WO-1200`'s FEEDBACK.

## 7. Flag-and-stop conditions

Park the branch and say so, in FEEDBACK and in the session, if any of these appears:
- A library licence that does not fit `RUL-11`.
- A design that needs Python, Java, or an image change.
- A gate that reads any rule from the board under judgement.
- A non-deterministic emission.
- `unsafe`, or a test that can only go green by being skipped, ignored, or weakened.
- Any step that needs `sudo`, a system package, or a secret beyond the push route above.

## 8. Kickoff prompt

**Step 1, in its own turn:** send `/goal` alone, read what is in force, and dismiss it.

**Step 2, in its own turn, set the goal for the spike half:**

```
/goal AET-WO-1300 spike: every §0 question has a measured answer or a recorded reason it cannot be, cited to its CI run in FEEDBACK, on an open unmerged PR from wo/aet-1300-schematic, and the build half has not started.
```

Confirm that the "Goal set:" echo matches.

**Step 3, the kickoff message:** carried by the architect at kickoff, with the live base. The architect gives the build goal in the same message as the ruling.
