# AET-WO-1102 — FEEDBACK

**Branch:** `wo/aet-1102-identifier-set` · **Base:** `origin/main` e3e3cd2 · **Builder:** Claude (Sonnet 5.5), targeted adversarial pass by a fresh subagent.

Entry schema: `finding · where · type · recommendation · decision-needed`.

## 1. Predictions (written before any spike run, 2026-11-13)

Read from the code and from `AET-WO-3300` F13–F15, not measured. Each is checked against the table in §2.

**Outputs and what I expect each to do with an identifier**

| Output | Where an identifier lands | Prediction |
|---|---|---|
| `.kicad_sch`, `.kicad_pcb` | S-expression atoms, quoted (`sexpr::quote`) | Carries every admitted character unchanged. `AET-WO-1300`'s existing test already shows `"`, `\`, `(`, `)`, `;`, `'`, `^`, `~`, `\|`, `,`, `*`, `?`, `#`. KiCad's own netlist reads them back. |
| BOM CSV | reference column (RFC 4180, `field()` quotes `,` `"` CR LF) | Carries every character unchanged. |
| Placement CSV (`pos.csv`) | reference | Damaged by `"` only (F15: KiCad does not escape it). `,` is safe because the field is quoted. |
| Gerber X2 | `%TO.N,<net>*%`, `%TO.C,<ref>*%` | `,` `*` `%` `\` are escaped by KiCad as `\uXXXX`, so the bytes differ: damaged. Dates and times from names are refused at export (F14). |
| Drill file | net attributes in the header | Same as Gerber for the same four characters. |
| Renode `.repl` | the project name in the file name; other identifiers where the platform names them | Not damaged by any character in a net or reference. Damaged by the Windows set (`< > : " / \ \| ? *`) only through the file name. |
| Board and firmware crates | net and pin names in `///` doc comments and possibly string literals; the project name in `Cargo.toml` package names | Doc comments are safe for every character. If a literal is built with `format!("\"{}\"")`, `"` and `\` damage it; if with `{:?}`, none do. **I expect the latter.** The project name is a Cargo package name: **only `[A-Za-z0-9_-]` is valid** (no `.`, `+`, `@`, `!` …), and a leading digit or hyphen may be refused. |
| File names | `<project>-…` | Damaged by `< > : " / \ \| ? *` for the project name only. |
| Archive, scenario names | scenario name in the trace and `MANIFEST.json` (JSON-escaped) | Nothing damaged by any character. |

**Predicted damaged set, by character**

| Characters | Predicted damage | In which kind |
|---|---|---|
| `"` | Placement CSV (F15); Windows file name; Cargo name | reference; project |
| `,` `*` `%` `\` | Gerber and drill escape (`\uXXXX`) | net, reference |
| `:` `<` `>` `\|` `?` | Windows file name; Cargo name | project only |
| `.` `+` `@` `!` `#` `&` `=` `'` `;` `~` `^` `(` `)` `` ` `` and the rest of the printable set | Cargo name only | project only |
| `-` `_` (controls) | nothing | — |
| a date shape (`2026-11-13`) | Gerber: refused at export, exit 5 | net, reference, project |
| a time shape (`12:34:56`) | the same | net, reference, project (needs `:`) |

**The prediction that matters, and may trigger §7.3.** KiCad's own `+3V3` needs `+` in a net or a domain id. The project name cannot carry `+` into a Cargo package name. If the measurement bears that out, one table for every kind cannot both refuse `+` for the project and admit it for a net. I expect to stop at that point unless the measured set makes it moot, and will say so rather than invent a per-kind rule the work order does not ask for.

**Batching, to be done and explained in §2.** `validate` and `generate` run natively in seconds and start no tool, so every character × kind × `generate` run is done one by one. `verify --fab-out` (about 4 minutes in the image) is batched: one specification carries a different character in each of several kinds, and a failure is bisected only when one occurs.

## 2. Spike, native half (`validate` and `generate`), measured 2026-11-13

**How.** For each kind, each of the 88 admitted characters (and the six refused ones, as controls) goes in the middle of one identifier (`AB<c>CD`; `ab<c>cd` for the project name), and `validate` and `generate` run on the result: 6 kinds × 94 characters plus a date and a time shape, one specification per run. These start no tool and take 1.4 s each, so no batching was needed. The harness is `phaseA.py` in the session scratchpad. For each run it checks that the BOM keeps five columns, that every `.rs` file parses (`rustfmt --check`), that both `Cargo.toml` files load (`cargo metadata --offline`), and which files carry the identifier.

| Kind | `validate` refuses | `generate` refuses after `validate` passed | `generate` admits |
|---|---|---|---|
| project name | `$ / [ ] { }` | **every character except `a-z`, `0-9`, `-`** (`InvalidCrateName`, exit 2): uppercase, `_`, `.`, `+` and the rest | 38 of 94 (and the date shape `2026-11-13`) |
| voltage domain id | the six | nothing | all 88 |
| net name | the six | nothing | all 88 |
| bus id | the six | nothing | all 88 |
| I2C device reference (`U2`) | the six | **`! " # % & ' ( ) * + , - . : ; < = > ? @ \ ^ ` \| ~`, both shapes** (`InvalidDeviceName`: the Renode platform needs `[A-Za-z][A-Za-z0-9_]*`, case-folded, and unique after folding) | 63 |
| SPI device reference (`U4`) | the six | nothing | all 88 |
| scenario name | **nothing** (the scenario file is not run through `validate` at all) | nothing | all |

No output the native half writes was damaged by any character that `generate` admitted: the BOM kept five columns, no `.rs` file failed to parse, both manifests loaded. `"` and `\` reach `board.rs` raw in a doc comment, which is harmless there.

## 3. Stopped: §7.3, and a second place §1.2 cannot hold

**`generate` already refuses identifiers `validate` passes, per kind, and no single table fits them.**

- A project name made only of `[A-Za-z0-9_-]` is refused by `generate` when it holds an uppercase letter or `_` (`My_Board`, `sentinel_01`). That is §7.3 word for word: an output refuses an identifier made only of `[A-Za-z0-9_-]`.
- An I2C device reference accepts `_` but not `-` (`U-2` is refused); a project name accepts `-` but not `_`. The two sets differ.
- `+` (KiCad's own `+3V3`, §7.2) is admitted for nets, domains and buses, and refused for a project name and an I2C reference.
- So the §1.2 text, "the set every output carries unchanged" in one table, would have to be `[A-Za-z0-9]`, which refuses `+3V3` and `Net-(U1-Pad3)`, both of which `RUL-14` says must fit.

**What the fix looks like, for the architect to rule.** `identifier` takes a kind, and the table has a base set plus a stricter shape per kind (project name, I2C reference); the base set is whatever the Gerber, drill and placement measurement leaves. That is a change to §1.2's "one table", so it is yours, not mine. The alternative is to keep one table at `[A-Za-z0-9]` plus a few, and amend `RUL-14`'s own "`+3V3` fits".

**Not done, and why.** The image half (Gerber X2, drill, placement CSV, Renode load, ELF build through `verify --fab-out`) is not measured. The one command that does it, `podman run` of the pinned image plus the `memory.swap.max` write the global contract requires, was refused twice by this session's auto-mode classifier. I did not work around it. The predictions in §1 for those outputs stand untested. Nothing is built, no test or code is changed, and no PR is open.

**Needs a decision:** (1) per-kind shapes, or one narrow table; (2) permission to run the image half (28 punctuation characters plus the two shapes, six kinds combined in one specification, bisected on failure, about 30 minutes in four capped containers).
