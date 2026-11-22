# AET-WO-1215 — An older specification is refused with every change it lacks, each with the version and work order that made it

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1215-version-changes`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4. Written 2026-11-22. **Cadence:** a measurement, then a build.

**Grounds** (roadmap §7.1: "`validate` names only the first field an older specification lacks. Refuse instead with the complete list…", the Grok review 2026-11-17, round 2, accepted then):
- A file of another version is refused by name (`UnsupportedSchemaVersion`), and nothing tells its author what to add. The changes live only as prose in `crates/aethelgard-ir/README.md` §`Specification`.
- The version moved 21 → 22 → 23 in one day, and goes to 24 with `1211e`. Every bump strands every file written against the last one (northstar §6: "so hard to write that people hand-edit outputs").
- **`INV-06` stands:** the refusal names what is missing. Nothing is migrated, and no value is filled.

> **What this is:**
> - **A version table in `aethelgard-ir`:** one row per version, 2 through current. Each row holds:
>   - its work order;
>   - the key paths it requires, adds as optional, replaces or removes;
>   - the rules it adds that refuse an older file's content, by kind.
>   - It is cited row by row to the README's prose, which stays the human text.
> - **`UnsupportedSchemaVersion` for an older file** names the file's version and the current one. It lists every row between them, in order, with its work order and paths:
>   - in the message, compactly;
>   - in a structured field of the refusal body, so an agent can read it.
> - **A newer file** (above current) is refused as today, naming the current version.
> - **A test pins the table's last row to the current version literal,** so a bump without a row fails CI. The kit (`spike/4240a/kit/renumber.py`) gains nothing: the row is written by hand by the bump's work order.
>
> It is NOT:
> - a `migrate` command;
> - a default for any field, or any change to which files validate;
> - a skeleton generator.

## 1. Build

1. **Measure first,** in `spike/1215/`:
   - Every version 2–23's changes, read from the README prose and checked against `git log -p` of the version literal and the schema. Where the prose and the code disagree, stop and report both.
   - Today's refusal for a version-20 and a version-1 file (the bytes).
2. **The table, and the refusal's message and body.**
   - Read `cli/README.md`'s refusal contract.
   - `2307` adds `error.entries` to one refusal. Use the same mechanism, and merge `main` after it if it lands first, so the body gains one optional field shape, not two.
3. **Tests:**
   - a version-20 file lists rows 21, 22 and 23, exactly;
   - a version-1 file lists 2 to 23;
   - a version-24 file (newer) names the current version;
   - the last row's version equals the literal;
   - every path the table names is a key the current schema has, except `replaced` and `removed` paths, which it must not have.
   - **Mutants:** drop a row; swap two rows. A test kills each.

## 2. Acceptance

- **The measurement is in FEEDBACK.** Every row is cited to the prose and to the commit that made it.
- **No file that validates today stops validating, and no refusal of a current-version file changes.**
- **The CLAUDE.md gates pass, and CI is green.** `TIER2_MIN` is unchanged.

## 3. Fences

- **Fence:**
  - `crates/aethelgard-ir` (the table and the refusal) and its README;
  - the CLI's refusal plumbing only as far as the structured field needs, and `cli/README.md`'s refusal rows;
  - tests, `spike/1215/`, FEEDBACK.
- **Not:**
  - any specification;
  - the kit;
  - workflows, or `ci/` (except a weight row).
- **Running beside you:**
  - `1211e` (IR, specification 24): its bump must add row 24. Whoever merges second writes or rebases the row onto main, by merging main.
  - `1214` (IR `validate`, 13c);
  - `2307` (the CLI's refusal body);
  - `1211b` (generator, solved 16);
  - `1211d` (eda).
- Merge `main` before READY. One PR, left unmerged. Write `~/.cache/aet-1215/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A change the table misses:** a renumbered work order (the README's "renumbered from" notes), a key made optional, a rule added without a key, or a solved-version change that a specification cannot see.
- **A message that suggests a value.** It must name the path only.

## 7. Flag-and-stop conditions

- **The README prose and the code disagree about a version.** Stop and report both.
