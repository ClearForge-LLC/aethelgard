# AET-WO-3301 — `aethelgard generate` and `aethelgard verify`: one verdict that binds a specification to every artifact it produced (`INV-01`, `INV-03`, `RUL-19`)

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-3301-verify`.
**Author:** Claude (architect) · **Builder:** a strong builder, with a **full** adversarial pass (`docs/roadmap.md` §6). A verdict that claims to bind a pair, and does not, is the confident wrong answer in its purest form.
**Phase:** P3, before `AET-WO-3300`. That work order's fabrication clause needs "file hashes [that] match the ones named in the passing verdict", and no such verdict exists.

**Grounds:**
- **The Mythos 5.2 review** (`docs/reviews/2026-11-12-mythos-5-2.md`):
  - **A3:** the CLI cannot write the board crate, the firmware or the platform. Only `AETHELGARD_BLESS=1 cargo test -p aethelgard-svd-hal --test board` does.
  - **A4:** no verdict binds the gates to one pair. `check drc --spec` passes a board against an unrelated specification, and `prove` passes a vacuous harness.
  - **B1 and Part C 1:** `verify`.
- **`RUL-19` (2):** each gate binds only its own input. Only `verify` binds a pair.
- **The generators are pure functions already:**
  - `aethelgard_svd_hal::board::generate_with(crate_name, &solved, &table, answers)`;
  - `aethelgard_twin::platform::emit(&solved)`;
  - the KiCad emitter behind `emit`.

  The bless harness in `crates/aethelgard-svd-hal/tests/board.rs` shows how they compose today.
- **The roadmap's P0 "deliberate gap"** (the `build` subcommand) is closed by this work order.

> **What this is:**
> - `aethelgard generate`, which writes everything a specification produces;
> - `aethelgard verify`, which regenerates into scratch, runs every gate on exactly those bytes, and prints one verdict naming every artifact's hash and every gate's result;
> - `prove`'s report of whether a harness is the generated one.
>
> It is NOT:
> - a change to any generator's output, or to any gate's judgement of a given input;
> - extraction (`1303`);
> - the fabrication package (`3300`).

**Cadence:** build-only. Stop only on §7. One PR, left unmerged.

## 1. Scope

1. **`aethelgard generate --spec <file> --out <dir>`.**
   - It validates and solves exactly as `solve` does. Then it writes, under `--out`, which must be empty or absent:
     - the KiCad schematic, board and BOM, exactly as `emit` writes them (`kicad/`);
     - the board crate and the firmware crate, from `generate_with`, with `answers` from `aethelgard_twin::model` exactly as the bless harness passes it (`board/`, `board/firmware/`);
     - the Renode platform, from `twin::platform::emit` (`board/<project>.repl`).
   - **The reference proves it:** `generate` on `examples/sentinel-01-minimal.json` reproduces `boards/sentinel-01/**`, byte for byte.
   - Nothing else is written, and no process is started.
2. **`aethelgard build --crate <dir>`:** the firmware ELF.
   - It runs `cargo build --release --offline --target thumbv7em-none-eabihf` as a sandboxed tool, the way `prove` already runs cargo, under the same bounds and environment rules (`RUL-13`, `architecture.md` §6).
   - It reports the ELF's path and sha256. It refuses a crate with any dependency, as `prove` does (`RUL-04`).
3. **`aethelgard verify --spec <file> [--scenarios <file>...] [--against <dir>]`.**
   - **In a scratch directory,** in order, it runs:
     1. `generate`;
     2. ERC on the schematic;
     3. DRC on the board, with schematic parity against that same schematic;
     4. `prove` on the generated board crate;
     5. `build` of the generated firmware;
     6. `simulate --firmware` with the generated platform;
     7. when scenarios are given, `campaign` over them, against that ELF and that specification, into a scratch archive.
   - **It prints one JSON verdict:**
     - the specification's sha256;
     - the topology's sha256;
     - every generated file's sha256, by path;
     - the ELF's sha256;
     - each gate's verdict and exit code;
     - the campaign's run ids and state hashes;
     - the engine version.
   - **It passes only if every gate passed,** and it fails closed on any refusal (`INV-06`). Its exit codes follow `RUL-10`'s table.
   - **With `--against <dir>`,** it first compares `<dir>` byte for byte with the regenerated tree and fails naming each differing, missing or extra file. That is `INV-01`'s "regenerate and diff", runnable by a user.
   - **The rise times are labelled** "computed from declared values" (`RUL-19`).
   - MCP exposure is generated from the tree, as always. README section.
4. **`prove` reports harness provenance (A4).** Its verdict gains:
   - `harness: "generated" | "supplied"`, where "generated" means the crate's harness file is byte-identical to what `generate_with` writes for the specification given with `--spec`;
   - `cover_count`.

   With no `--spec`, the harness is `"supplied"`. `verify` requires `"generated"`.
5. **The bless harness calls `generate`'s library function.** `crates/aethelgard-svd-hal/tests/board.rs` and `generate` share one composition function, so the two cannot drift.

## 2. Invariants and fences

- `INV-01`, `INV-03`, `INV-06`, `INV-07`, `RUL-04`, `RUL-09`, `RUL-10`, `RUL-13`, `RUL-16`, `RUL-19`. No `unsafe`.
- **Fence:**
  - the CLI: `generate`, `build`, `verify`, `prove`'s two fields, MCP and the README;
  - one composition function, in `aethelgard-svd-hal` or the CLI, that the bless harness also calls;
  - tests;
  - this work order's FEEDBACK.
- **Every committed generated file stays byte-identical.**
- **Every existing gate's verdict on an existing input is unchanged,** apart from `prove`'s two new fields.
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance

1. **`generate` on the reference** reproduces `boards/sentinel-01/**` and `emit`'s three files, byte for byte (tier 1 for the generation; tier 2 for `build`).
2. **`verify --spec examples/sentinel-01-minimal.json --scenarios scenarios/sentinel-01/pullup-2k2.json`** passes, and its verdict names every hash. The same run with `pullup-47k.json` fails with the campaign's findings. Two runs give byte-identical verdicts.
3. **The review's A4 inputs cannot pass `verify`:**
   - a board from another specification (via `--against`);
   - a hand-written vacuous harness (`prove` reports `"supplied"`, and `verify` refuses it);
   - a firmware ELF not built from the generated crate.
4. **`--against boards/sentinel-01`** (with the KiCad files from `emit`) passes. One changed byte in any file fails, naming the file.

§1.4 passes in `gates` and `tools`.

## 5. Adversarial pass — full

1. **Make `verify` pass on artifacts that did not come from its specification:** swap a file mid-run, race the scratch directory, or supply a crate whose harness matches in name only.
2. **Make a gate inside `verify` judge different bytes than the hashes the verdict names.**
3. **Make `generate` write outside `--out`,** or overwrite a file that exists.
4. **Make two `verify` runs disagree.**

## 7. Flag-and-stop conditions

- A generator's output depends on anything but the specification, the pin table and the twin's model table, so `generate` cannot be pure.
- Building the firmware needs network access or a dependency.
- A test that can only go green by being weakened.
