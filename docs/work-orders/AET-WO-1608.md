# AET-WO-1608 — `aethelgard init --template`, and errors that quote the fix: a specification a person starts from, which cannot pass until they fill it

**Repo:** `ClearForge-LLC/aethelgard` · **Base:** `main`, named in the kickoff.
**Branch:** `wo/aet-1608-init-template`.
**Author:** Claude (architect) · **Builder:** standard tier, with a targeted adversarial pass.
**Phase:** P4, backlog wave 3. Written 2026-11-23. **Cadence:** a measurement, then a build with one stop.

**Grounds** (roadmap §7.1 L923: "Specification authoring: `aethelgard init --template`, and errors that quote the fix", owner "named at P4's gate, before any announcement"; triage W3-AUTHOR, row 895):
- **Review 2 B7** (`docs/reviews/2026-11-15-mythos-5-2.md:272`): northstar §6's "too hard to write" failure. The reference was 3.4 kB at version 8. Today `examples/sentinel-02-bench.json` is 5.9 kB and `sentinel-01.json` 22.1 kB.
- **The CLI:** subcommands are the `Command` enum (`cli/src/main.rs:75`); exit codes at `:48-65`; `validate` at `:2720`. `emit` refuses an unwritable output as `UnwritableOutput`, exit 3, and never follows a symlink (`:3105-3140`).
- **MCP:** every leaf subcommand is a tool, except `help` and `mcp` (`cli/src/mcp.rs:69`). The list is pinned by `cli/tests/mcp-tools.json` (`mcp_snapshot_tier1.rs`). A new subcommand becomes a tool unless excluded there.
- **`include_str!` from `examples/` will not package.** CI's package gate refuses an include target outside the crate (`.github/workflows/ci.yml:163-171`; `ir/src/orientation.rs:96`, `cli/src/fab.rs:1296` say the same). The CLI is a published crate.
- **`2307`'s pattern:** a field the person must write is left empty, which `validate` refuses by name (`reason: ""` → `InvalidSource`). Here the same holds: `project_name: ""` is `InvalidIdentifier` (`ir/src/lib.rs:3008`, 1–64 bytes), a `source: ""` is `InvalidSource` (`:1515`).
- **The IR's refusals:** 173 `IrValidationError` kinds (`ir/src/lib.rs:1485`, `kind()` at `:2425`), plus the solver's and the CLI's `SchemaViolation` (serde's text, `main.rs:2972`).

> **What this is:**
> - **(1) `aethelgard init --template <name> --out <file>`.** It writes one specification from a template held in the CLI crate. It prints one JSON line naming the template, the file and every placeholder's path, and exits 0.
>   - Every placeholder fails `validate` by its own kind and path until it is filled (`INV-06`). A template that passes `validate` unfilled is a defect.
>   - It never overwrites: an existing `--out`, a symlink or a FIFO is `UnwritableOutput`, exit 3. An unknown template is a usage error, exit 64.
>   - Each template is held to its `examples/` source by a test that reads the example at test time and compares after the placeholders, field for field.
> - **(2) Errors that quote the fix, for a measured few kinds only.** For the N kinds a hand-written specification meets most often (§1.1), the message ends with the exact key and value shape that satisfies it. IR text changes only where a test pins the new text.
> - Each refusal comes with a test, and each test with a mutant it kills.
>
> It is NOT:
> - a default for any value a person must state, a wizard, or an interactive prompt;
> - a migration of older files (`1215` lists what they lack);
> - a change to any rule, kind name, exit code, or another refusal's bytes;
> - a specification bump.

## 1. Build

1. **Measure first,** with predictions committed in `spike/1608/`:
   - **the corpus:** each committed specification with one key deleted, and each string emptied, one at a time; `validate` on each; the refusal kinds counted;
   - each candidate template's size, and which of its fields a person must own (name, sources, acknowledgement reasons, others you propose);
   - `cargo package -p aethelgard-cli --list` with the templates in the crate.
2. **Stop with:**
   - **the templates** (which examples, where they live in the crate, and the placeholder list for each);
   - **N and the kinds,** with each new message's text;
   - **MCP: on or off.** On, `init` writes a file at a caller's path, as `emit` does, and the snapshot gains a row. Off, `mcp.rs:69` excludes it by name. Recommend one, with the reason.
3. **Build to the ruling:** the subcommand, the templates, the tests, and the README's command and refusal rows.

## 2. Acceptance

- **Each template, unfilled,** is refused once per placeholder: fill all but one, and `validate` names that one by kind and path.
- **Each template, filled with test strings,** passes `validate` and `solve`; it is at the current `SPECIFICATION_SCHEMA_VERSION`, by test.
- **Each chosen kind's new text** is pinned by a test; every other refusal's bytes are unchanged, across every refusal test.
- **Mutants:** a placeholder given a value; a template drifted from its example; an existing `--out` truncated; a new message that names the wrong key. A test kills each.
- **The CLAUDE.md gates pass, CI is green, and the package gate passes.**

## 3. Fences

- **Fence:** `crates/aethelgard-cli` (`src/main.rs`, a new `src/init.rs`, `templates/`, `src/mcp.rs` only if ruled off, `tests/mcp-tools.json` only if ruled on, new tests, `README.md`); `crates/aethelgard-ir`'s error text for the N kinds and their tests; `spike/1608/` and FEEDBACK.
- **Not:** `examples/` (read, never written); any rule in the IR; the solver, eda, the generators; workflows; `ci/` except weight rows.
- **Running beside you:** the backlog wave in roadmap §5. Merge `main` before READY; a specification bump on `main` moves the templates with it.
- One PR, left unmerged. Write `~/.cache/aet-1608/pr-title-body.md`.

## 5. Adversarial pass, targeted

- **A placeholder that passes:** a numeric or enum field cannot hold `""`. Find every one a person must own; a serde `SchemaViolation` is not a refusal by name.
- **An acknowledgement** carried over from an example that the template's own changes make stale.
- **`--out`** naming a directory, a dangling symlink, or a path through one.
- **A quoted fix** that, pasted as written, is refused by the next rule.

## 7. Flag-and-stop conditions

- **After §1.2:** stop with the templates, the kinds and the MCP answer.
- **A field a person must own** cannot be made to fail `validate` by name without a new rule. Stop: that is an IR change, not this work order's.
- **The corpus's most common kind is `SchemaViolation`** (serde's text, not the IR's). Stop with the count: quoting a fix there needs a parser change or a dependency.
- **A template cannot be held to its example** inside the package gate. Stop with the options.
