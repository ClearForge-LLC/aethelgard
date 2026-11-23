# AET-WO-1608 §1.1 — predictions, written before any measurement

Written 2026-11-23 before `corpus.py` was run. Not edited after.

Corpus: every committed `schema_version: 28` specification that `validate` accepts
as committed (the five valid `examples/`, plus the CLI and IR test fixtures).
Mutations, one at a time: (a) each object key at every depth deleted;
(b) each string value emptied (`""`).

- **P1.** Key deletion is dominated by `SchemaViolation` (serde's "missing field"):
  over 80 % of deletions. The structs are `deny_unknown_fields` and nearly every
  field has no default (`INV-06`); `Option` fields are written as explicit `null`.
- **P2.** A minority of deletions still pass `validate` (fields with
  `#[serde(default)]`: `acknowledge_unwitnessed`, `accept_lifecycle` and a few
  vectors): under 10 %.
- **P3.** String emptying: `InvalidIdentifier` is the most common kind (ids, nets,
  references, parts, pins), then `InvalidSource` (stackup and capacitance
  sources), then `SchemaViolation` (enum strings: `ldo`, `signal`, `right`, `top`).
- **P4.** Some emptied strings pass `validate` or are refused by a kind that does
  not name the field (a prose `condition`, a connector pin label). Predicted:
  at least one `condition` string passes when emptied — a placeholder candidate
  that would not fail by its own kind (a §5 finding if a template needs it).
- **P5.** Across both mutations together, the most common kind is
  `SchemaViolation`, so the WO §7 stop condition ("the corpus's most common kind
  is `SchemaViolation`") is met. Its text already names the missing key
  (`missing field \`x\``) but not the path; a path needs a parser change or a
  dependency (`serde_path_to_error`).
- **P6.** Templates: `sentinel-02-bench.json` (5.9 kB) is the smallest drawn,
  generable board, and the natural single template. `sentinel-01-minimal.json`
  (17.8 kB) is the second candidate. Person-owned fields in the bench: the
  `project_name`, `mcu.core_supply.source`, both `pin_capacitance` `condition`
  and `source`, and the four `stackup` sources: eight placeholders.
- **P7.** `cargo package -p aethelgard-cli --list` lists a `templates/*.json`
  under the crate root without any `include` change in its `Cargo.toml`.
