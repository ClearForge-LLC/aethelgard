# AET-WO-1306 — One strict reader for every KiCad file a caller hands a gate

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main` after `AET-WO-1305`, named in the kickoff.
**Branch:** `wo/aet-1306-strict-reader`.
**Author:** Claude (architect) · **Builder:** strong tier, with a full adversarial pass (`docs/roadmap.md` §6). A reader that disagrees with KiCad is exactly where a confident wrong pass hides.
**Phase:** P3. It **gates `AET-WO-3300`**, alongside `1601` and `1305`.

**Grounds:**
- **`AET-WO-1305` F6, measured by its adversarial pass.** `aethelgard_eda::sexpr::Sx::child` returns a repeated child's *first* match, but KiCad takes the *last*. A board with a second `(thickness 0.2)` passed `check drc`, and passed the first build of `extract`.
  - Every reader that goes through `child()` on caller input is open to the same class: the gate judges one value while KiCad, and the fabricator after it, use another.
- **`AET-WO-1305` fixed this for `extract` alone** (`aethelgard-eda/src/extract.rs`: `allowed`, `strict`, `bounded`, `footprint`, `read`). It allowlists each node's children, with their arity and whether they may repeat, and refuses everything else. All 31 breaking boards in its adversarial corpus are refused.
- **The other readers of caller input,** found by the architect with a search for `.child(` (40 calls in six files; confirm each):
  - `board.rs` (`check drc`'s board preparation);
  - `parity.rs` (the board and the schematic, for DRC with parity);
  - `route.rs` (`read`, used by `extract`, and the router's own reading of a board);
  - the CLI's `kicad.rs` (`check erc`'s schematic refusal, `AET-WO-1601` §1.5).
- **The other readers parse vendored data, not caller input:** `library.rs`, and the parts of `schematic.rs` and `pcb.rs` that read the vendored KiCad libraries, pinned by `aethelgard-kicad-lib`'s hashes.

> **What this is:** one strict reader, shared by every gate that reads a `.kicad_pcb` or `.kicad_sch` a caller supplies, and a compiled fence that keeps first-match reads off caller input.
>
> It is NOT:
> - a change to what any emitter writes;
> - a change to any verdict on an input that KiCad and the gate already read alike.

**Cadence:** build-only. Stop on §7. One PR, left unmerged.

## 1. Scope

1. **One strict reader in `aethelgard-eda`,** grown from 1305's `allowed` and `strict` walk into a module of its own (for example `strict.rs`).
   - It covers both file kinds, `.kicad_pcb` and `.kicad_sch`, as far as any gate reads them.
   - For every node it reads, it states which children may appear, their arity, and whether they may repeat.
   - **A repeated child that may not repeat is refused, never resolved first-or-last.** So is an unknown child where the gate's meaning depends on the node.
   - Refusals are typed: exit 3, as `MalformedBoard` or `MalformedSchematic`, each naming the path to the node (`INV-06`).
2. **Every gate reading caller input runs it first,** before any tool and before any judgement:
   - `check drc` (with and without `--spec` and `--schematic`);
   - `check erc`;
   - `extract`;
   - `verify`'s copies of each.

   Whatever the gate then reads, it reads from the strictly read tree, and never re-parses with first-match lookups.
3. **A compiled fence.** First-match `Sx::child` stays for vendored data, which hashes pin.
   - A crate-local `clippy.toml` `disallowed-methods`, as `AET-WO-1204` used, or a type that only the strict reader produces, keeps `child` off every caller-input path.
   - A guard test pins which modules may use `child`, so a renamed method or a new module cannot slip past.
   - Choose the mechanism, and justify it in FEEDBACK.
4. **The corpus becomes a regression test for every gate.**
   - 1305's adversarial corpus (31 boards, `dup_thickness` among them), together with schematic counterparts you build for each class that applies, is refused by every gate that reads that kind.
   - The corpus lives in the repository's test fixtures, with each file's class named.

## 2. Invariants and fences

- `INV-03`, `INV-06`, `RUL-03`, `RUL-19`. No `unsafe`.
- **Fence:**
  - `aethelgard-eda`'s `sexpr.rs`, the new module, and the readers named above;
  - the CLI's `kicad.rs` and its error mapping;
  - tests and their fixtures;
  - this work order's FEEDBACK.
- **Every committed generated file stays byte-identical,** and every existing gate's verdict on every existing fixture stays as it is. Measure this. Any change is a §7 stop.
- `TIER2_MIN` rises by exactly the tier-2 tests added.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance

- **`dup_thickness` is refused** by `check drc` (exit 3, naming the path). It passed before.
- **Every board in the corpus** is refused by `check drc`, `extract` and `verify`'s board step. Every schematic counterpart is refused by `check erc` and by parity.
- **The reference board and schematic** (`aethelgard generate`) pass every gate exactly as before, and `verify` on the reference is byte-identical to `main`'s.
- **The fence fires:**
  - a first-match `child` added to a caller-input reader fails the build or clippy;
  - renaming the strict reader's entry point fails the guard test.
- **The gates** (`CLAUDE.md`) pass, and `gates`, `tools` and `image` are green in CI.

## 5. Adversarial pass — full

1. **Find a file KiCad reads one way and a gate reads another,** and that still passes. Try: repeated children, an unknown child, a quoted string versus a bare atom, an escaped character, an empty list, deep nesting, a very long atom, and a UTF-8 edge case.
2. **Find a caller-input path** that still reaches `child` or another first-match lookup.
3. **Find an existing fixture whose verdict changed.**

## 7. Flag-and-stop conditions

- An existing fixture, or the reference, is refused by the strict reader. That is a disagreement to rule on, not to code around.
- KiCad's own reading of a construct cannot be established from KiCad 10.0.7's source or behaviour in the pinned image.
- Meeting §1.3 needs a change outside the fence.
