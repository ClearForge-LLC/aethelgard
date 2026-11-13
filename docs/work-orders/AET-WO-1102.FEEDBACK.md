# AET-WO-1102 — FEEDBACK

**Branch:** `wo/aet-1102-identifier-set` · **Base:** `origin/main` e3e3cd2 · **Builder:** Claude (Sonnet 5.5), targeted adversarial pass by a fresh subagent.

**Outcome:** stopped on §7.3 at the first push (§1–§3 below); the architect's ruling on PR #106 then said build, and §4 on is the build. The image half of the spike is measured by CI, through the tier-2 test, not by a local `podman` run (ruling item 4).

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

## 3. Stopped: §7.3, and a second place §1.2 cannot hold (superseded by the ruling, §4)

**`generate` already refuses identifiers `validate` passes, per kind, and no single table fits them.**

- A project name made only of `[A-Za-z0-9_-]` is refused by `generate` when it holds an uppercase letter or `_` (`My_Board`, `sentinel_01`). That is §7.3 word for word: an output refuses an identifier made only of `[A-Za-z0-9_-]`.
- An I2C device reference accepts `_` but not `-` (`U-2` is refused); a project name accepts `-` but not `_`. The two sets differ.
- `+` (KiCad's own `+3V3`, §7.2) is admitted for nets, domains and buses, and refused for a project name and an I2C reference.
- So the §1.2 text, "the set every output carries unchanged" in one table, would have to be `[A-Za-z0-9]`, which refuses `+3V3` and `Net-(U1-Pad3)`, both of which `RUL-14` says must fit.

**What the fix looks like, for the architect to rule.** `identifier` takes a kind, and the table has a base set plus a stricter shape per kind (project name, I2C reference); the base set is whatever the Gerber, drill and placement measurement leaves. That is a change to §1.2's "one table", so it is yours, not mine. The alternative is to keep one table at `[A-Za-z0-9]` plus a few, and amend `RUL-14`'s own "`+3V3` fits".

**Not done, and why.** The image half (Gerber X2, drill, placement CSV, Renode load, ELF build through `verify --fab-out`) is not measured. The one command that does it, `podman run` of the pinned image plus the `memory.swap.max` write the global contract requires, was refused twice by this session's auto-mode classifier. I did not work around it. The predictions in §1 for those outputs stand untested. Nothing is built, no test or code is changed, and no PR is open.

**Needs a decision:** (1) per-kind shapes, or one narrow table; (2) permission to run the image half (28 punctuation characters plus the two shapes, six kinds combined in one specification, bisected on failure, about 30 minutes in four capped containers).

## 4. The ruling, and what was built to it

The architect's ruling on PR #106: per-kind shapes in one function; `validate` refuses what `generate` refuses; the base set refuses only the six, `"`, and the date and time shapes; scenario names go through `identifier`; CI's `tools` job measures the image half through §1.3's tier-2 test; `TIER2_MIN` up by exactly one.

- **`identifier` is `check_identifier(IdentifierKind, &str)`** in `aethelgard-ir`, one table: `Plain` (domain id, net, bus id, pin, peripheral, signal, part, scenario name), `Project` (a crate name: `a-z0-9-`, a letter first, no trailing or doubled `-`, as `generate`'s `crate_name_ok`), `Reference` (`[A-Za-z][A-Za-z0-9_]*`, every I2C and SPI reference, and the `SolvedTopology`'s `i2c_devices` keys). The base set is checked first, so the date and time shapes and `"` are refused in every kind. `InvalidIdentifier` now carries `rule`, and its message names the rule broken. Uniqueness after case folding was already `validate` step 8 (`DuplicateReference`).
- **Tier 1:** `identifier_boundaries` (the base set: 25 refused values and 11 admitted, including `+3V3` and `Net-(U1-Pad3)`), `project_names_and_references_have_their_own_shapes`, `every_refused_character_and_shape_is_refused_by_its_path` (the CLI: each of the six, `"`, both shapes, in a net, the project name and a reference, exit 2 with the path; `My_Board`, `sentinel_01`, `U-2`, `U+2`; `+3V3` stays valid), `a_scenario_name_that_is_no_identifier_is_refused_before_a_run`. Two 1300 tests asserted the old wide behaviour of `emit` and were rewritten (`withstood_every_admitted_identifier_character…` now puts all 87 base characters in nets and uses narrow project and reference names; `withstood_dot_project_names…` now asserts the refusal), and one twin test moved two names from `InvalidDeviceName` to `InvalidTopology`.
- **Tier 2:** `every_admitted_identifier_character_survives_every_output` (`board_tier2.rs`): one specification, six nets carrying all 87 characters in ASCII order (each starts and ends on a character of its own; one ends in a digit so no two fold to the same name), a 63-byte domain id, 50-byte bus ids, a 61-byte scenario name, three 4-byte references (`Ua_9`, `Vb_8`, `Wc_7`; see §7, the 22-byte first try overlapped its own silkscreen), and a 38-byte project name with all of `a-z0-9-`; `verify --scenarios --fab-out` must pass, and the BOM, placement file, every Gerber's `%TO.N,<net>*%`, the drill file's net names, the file names, the manifest and the verdict must carry each identifier byte for byte. `TIER2_MIN` 79 to 80, with a comment. The weight in `ci/test-weights.tsv` is **248.00, an estimate from the sibling `--fab-out` test (247.70), not a measurement**: the local `podman` run it would need is the one the classifier refused. CI's own time replaces it.

### Deviations from the work order and ruling, each flagged

1. **Exit code 2, not 3.** §1.2 says exit 3. `validate` has always exited 2 for `InvalidIdentifier` and 3 is `InvalidJson` (`crates/aethelgard-cli/README.md`). I kept the contract and every existing test. The path is named either way.
2. **The scenario name is checked in `crates/aethelgard-cli/src/campaign.rs::read_scenario`, not in `aethelgard-chaos`.** The ruling widens the fence to chaos's `campaign.rs` line. That line cannot call `aethelgard-ir`: `workspace_shape`'s `chaos_has_only_its_known_files_keys_and_dependencies` and `the_members_depending_on_ir_are_exactly_the_four_the_fence_knows` fix chaos's dependencies and `RUL-16`'s compiled fence, and I did not touch either test. `read_scenario` is the one loader every command uses, so the check is in the same place for every path. Chaos's own `name_ok` stays, so an archive written before this still reads.
3. **`crates/aethelgard-ir/README.md` row 2 is updated** (outside the stated fence, a stale rule otherwise).
4. **Router finding, not an identifier one.** Renaming the six nets to `N0`…`N5` in the reference's own pairing makes `generate` fail with `Unroutable` (`/N5…: cannot reach U4 pad 1`) with any reference width; the same names assigned in the reference's sorted order generate. The router's success depends on the nets' name order. The tier-2 test keeps the reference's order, with a comment. Backlog: a router that completes the reference board in any order.

## 5. Adversarial pass (a fresh subagent, general-purpose, native runs only: `validate`, `generate`, `emit`, `solve`)

~1500 variants: ~35 characters at first, last, only and middle position in seven identifier positions; lengths 1, 63, 64, 65; date and time shape edges; case-folding collisions. Every variant `validate` refused was refused for a reason in the table, and every variant it accepted generated, `rustfmt`-clean, five-column BOM, each net byte for byte in the board, **except the findings below**. Spot-checked by me against the code and by running the repro where I fixed one.

| # | Finding | Type | Disposition |
|---|---|---|---|
| A1 | **A project name of 59 to 64 bytes passed `validate` and `generate` refused it** (`InvalidCrateName`: the board crate is `<project>-board`, at most 64). Reproduced: 58 generates, 59 does not | should-fix, INV-06 | **fixed**: the project rule is 1 to 58 bytes (`PROJECT_MAX`), tested at 58 and 59 in tier 1 |
| A2 | A reference equal, case folded, to a Renode stock instance or a solved peripheral (`i2c1`, `spi1`, `gpioa`) passes `validate`, and `generate` refuses it (`InvalidDeviceName`). It needs the solved topology | residual | backlog: `validate` cannot know the peripherals; `generate` refuses before any output |
| A3 | A modelled device named for a Renode instance that `STOCK` does not list (`nvic`, `cpu`, `sysbus`, `rcc`, `exti`, `timer2`, `usart1`…) or for an Antmicro keyword (`using`, `init`, `as`) passes `validate` and `generate`, and the `.repl` redeclares the name. Whether Renode refuses the load is **inferred, not run** | **open**, possible damage | backlog, for the architect: either `STOCK` is the platform's whole name set, read from the pinned platform, or a reference gets a prefix. The tier-2 test does not cover it |
| A4 | A reference differing only in case from one the emitter chose (`u1`, `c1`, `r1`, `r2` against `U1`, `C1`, `R1`, `R2`) passes `validate`; `generate` refuses it (`DuplicateReference`) | residual | backlog: refused before any output |
| A5 | A net or domain named `GND`, one starting `unconnected-`, or an emitter-named net (`U1_VCAP1`, `U4_WP`, `U4_HOLD`, `U2_SDO`) passes `validate`; `generate` refuses it (`ReservedName`) | residual | backlog: refused before any output |
| A6 | The router completes nets in name order, so a net name alone can make `generate` fail with `Unroutable` (`FLASH_CS` renamed `nCS`, `cs`, `Z`, `a`, `~`, `_X`, `U`) | residual, not an identifier rule | backlog: one table cannot fix it; see §4 deviation 4 |
| A7 | A project name that is a Windows reserved device name (`con`, `nul`, `aux`, `prn`, `com1`, `lpt1`) passes and writes `con.kicad_sch` etc. | **open**, §1.1's Windows-file-name damage | not changed: the ruling (item 3) removes only what was measured, and this is a name, not a character. Needs a ruling |
| A8 | Scenario override `bus` and `net`, and `expect_findings` entries, use chaos's `name_ok` (`0x21`-`0x7E`), so they admit the seven refused characters and the clock shapes. They reach JSON records only, and `bus` and `net` must match the specification | cosmetic | backlog |
| A9 | `validate` cannot refuse everything `generate` refuses: a pin, peripheral, part or `mcu.part` is a closed vocabulary (`UnknownPin`, `UnknownPeripheral`, `UnknownPart`) | scope | `INV-06` here means the identifier's shape; A2 to A5 and this row are the rest |
| A10 | The tier-2 test put every character behind `N<i>`, so a net starting or ending with `, * % \ -` was never exercised | should-fix | **fixed**: the nets are the ASCII runs themselves, so `!`, `3`, `B`, `Q`, `b`, `q` start a net and `2`, `A`, `P`, `a`, `p`, `~` end one |

Not run by the subagent (no tools): Gerber, drill, Renode load. CI's tier-2 test is that measurement.

## 6. Reduction pass

`broken_rule` was folded into `check_identifier` (one function, not two). `plain` is kept: it makes the 37 base-set call sites one word each. `REFUSED_IN_IDENTIFIERS` stays `pub` (the tier-1 tests read it). Nothing else added by this diff can go without a refusal returning.
