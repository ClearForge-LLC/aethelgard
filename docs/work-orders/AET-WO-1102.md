# AET-WO-1102 — Identifiers every output carries unchanged: `RUL-14` narrowed by measurement

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1102-identifier-set`.
**Author:** Claude (architect) · **Builder:** a cheap builder with a targeted adversarial pass (`docs/roadmap.md` §6).
**Phase:** P1 tidying, carried from P3 (`AET-WO-3300` F14 and F15).
**Cadence:** spike (§1.1), then build. Stop on §7. One PR, left unmerged.

**Grounds:**
- **`RUL-14` today.** An identifier is 1 to 64 bytes of printable ASCII `0x21`–`0x7E`, and none of `/`, `{`, `}`, `$`, `[` or `]`.
  - Those six were measured in `AET-WO-1300` D6, against KiCad's schematic and board only.
  - The outputs since then were never measured.
- **The outputs since then:**
  - the BOM (CSV, `AET-WO-1300`);
  - the placement file (CSV, `AET-WO-3300`);
  - Gerber X2 attributes, where `%TO.N,<net>*%` names every net and `*` and `%` delimit;
  - the drill file;
  - the Renode platform (`AET-WO-2100`);
  - the generated firmware and board crate (`AET-WO-1400`, `3301`);
  - the archive (`AET-WO-2300`);
  - the fabrication package's file names, every one `<project>-…` (`AET-WO-3300` F13).
- **Two holes `3300` found, each refused only at export (exit 5), after every gate has passed:**
  - a net named `2026-11-13` puts a date into a Gerber (F14);
  - a reference `U"4,X` shifts every later column of the placement CSV, because KiCad does not escape `"` there (F15).
- **The place to refuse is `validate`**, before any gate runs (`INV-06`).

> **What this is:** measure what every output does with each printable ASCII character in each kind of identifier, then narrow `RUL-14` to the set every output carries unchanged, plus a refusal of anything shaped like a date or a time of day.
>
> It is NOT:
> - a change to the export's own guards (`strip_clock`, `placement_rows`), which stay as defence in depth;
> - Unicode;
> - a rename of anything committed.

## 1. Scope

### 1.1 Spike: every character, every identifier kind, every output

**Write the predictions into FEEDBACK before any run.**

**The kinds of identifier:**
- the project name;
- a voltage domain id;
- a net name;
- a bus id;
- a device reference;
- a part name, where one is the specification's;
- a scenario name.

**The characters:**
- each of the 88 characters `RUL-14` admits today;
- `-` and `_` too, as controls.

**The method.** For each kind and character, take the reference specification and change one identifier to carry the character in the middle (`AB<c>CD`). Then run, in the pinned image:
- `aethelgard validate`;
- `aethelgard generate`;
- `aethelgard verify --scenarios scenarios/sentinel-01/pullup-2k2.json --fab-out`.

**What counts as damage.** Record every output in which the identifier does not appear byte for byte, or appears but changes how the file parses:
- a CSV column count;
- an S-expression token;
- a Gerber attribute;
- a Rust or Renode syntax error;
- an invalid file name on Windows (`< > : " / \ | ? *`);
- a refusal by any gate or export, with its exit code.

Also try a date shape (`2026-11-13`) and a time shape (`12:34:56`), each inside an otherwise safe identifier.

Batch the runs. One `verify` per character per kind is about 90 × 7 × 4 minutes, which is too long. Combine several characters into one specification where they cannot interfere, and say how.

**The table goes into FEEDBACK:** character × kind × output, with the prediction beside each result.

### 1.2 Narrow `RUL-14` in `aethelgard-ir`

- **Remove every character the spike shows any output damages.** Expected, to be confirmed or refuted by measurement: `"`, `,`, `*`, `%`, `\`, `:`, `<`, `>`, `|`, `?`, plus anything Rust or Renode needs.
- **Refuse any identifier containing `dddd-dd-dd` or `dd:dd:dd`** (`d` an ASCII digit).
- **One function, `identifier`, still decides.** Its refused set and its shape rule are one table, and the error message lists exactly that table.
- **Every committed specification and scenario stays valid.** The reference, the fixtures and the corpus must validate unchanged; if one does not, that is a §7 stop.
- `validate` refuses with `InvalidIdentifier` and its path, exit 3, before any gate runs.

### 1.3 Prove it end to end

- **A tier-1 test** feeds every refused character and both shapes through `validate`, each refused by path.
- **One tier-2 test** runs the specification whose identifiers use every admitted character at once through `verify --fab-out`. It passes, and every output carries every identifier byte for byte.
- `TIER2_MIN` rises by exactly that one test. Add its weight to `ci/test-weights.tsv`.

## 2. Invariants and fences

- `INV-01`, `INV-06`, `RUL-14` (narrowed only by what the spike measures), `RUL-19`. No `unsafe`.
- **Fence:**
  - `crates/aethelgard-ir/src/lib.rs`: `identifier`, its table and `InvalidIdentifier`'s message;
  - tests and their fixtures;
  - `ci.yml` (`TIER2_MIN`) and `ci/test-weights.tsv`;
  - this work order's FEEDBACK.
- **`RUL-14` itself is amended by the architect** at reconciliation, from your table. Do not edit `docs/architecture.md`.
- **Every committed generated file stays byte-identical.**
- One PR, left unmerged. **Push and revoke by the FUR-WO-0301 F7 route.**

## 3. Acceptance

- **The §1.1 table is in FEEDBACK,** with predictions beside results, and the batching is explained.
- **The narrowed set is exactly the characters the table shows damaged,** each with its row. The date and time shapes are refused.
- **The F14 and F15 cases each fail at `validate`** (exit 3, the path named), no longer at export.
- **Every committed specification still validates.** Every gate's verdict on the reference is byte-identical to `main`'s.
- **The CLAUDE.md gates pass,** and CI is green with every shard under 15 minutes.

## 5. Adversarial pass — targeted

1. **Get an identifier through `validate` that some output damages.** Try:
   - a character the spike did not try in that position;
   - a shape split across two identifiers that an output joins, such as a reference followed by a field;
   - a case-folding collision.
2. **Find an identifier position `validate` does not check at all.**

## 7. Flag-and-stop conditions

- A committed specification, fixture or scenario would become invalid.
- A character must be refused for one output but is required by another (KiCad's own `+3V3`, for example).
- An output damages an identifier made only of `[A-Za-z0-9_-]`.
