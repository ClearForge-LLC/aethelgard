# AET-WO-1608 — STOPPED after §1.2 (2026-11-23)

Measurement record: `MEASURED.md` (predictions in `PREDICTIONS.md`, committed first). No Rust is changed
on this branch. Every lean below is the builder's; each waits for the architect's ruling.

## D1. The templates: one, `sentinel-02-bench`

- **Source:** `examples/sentinel-02-bench.json` (5,893 bytes; schema 28). It is the smallest drawn,
  generable board: I2C and SPI, one sensor each, 2 layers, no acknowledgement and no lifecycle entry.
- **Where:** `crates/aethelgard-cli/templates/sentinel-02-bench.json`, stored **already emptied**: the
  example's bytes with each placeholder's value replaced by `""`, nothing else changed. `init` writes
  those bytes verbatim through `include_str!("../templates/sentinel-02-bench.json")` in `src/init.rs`.
  It parses and transforms nothing. Measured: the file packages with no manifest change
  (`MEASURED.md` §4).
- **The test holding it to its source:** it reads `examples/sentinel-02-bench.json` at test time,
  sets each listed placeholder to `""`, and asserts that equal to the template as a `serde_json::Value`,
  field for field. It also asserts that the template holds `""` at exactly the listed paths and
  nowhere else, and that its `schema_version` is `SPECIFICATION_SCHEMA_VERSION`.
- **The placeholders, ten.** Each was emptied alone in the corpus and refused by its own kind with
  its own `path`:

  | # | path | kind |
  |---|---|---|
  | 1 | `project_name` | `InvalidIdentifier` |
  | 2 | `mcu.core_supply.source` | `InvalidSource` |
  | 3 | `i2c_devices[0].pin_capacitance.declared.condition` | `InvalidSource` |
  | 4 | `i2c_devices[0].pin_capacitance.declared.source` | `InvalidSource` |
  | 5 | `spi_devices[0].pin_capacitance.declared.condition` | `InvalidSource` |
  | 6 | `spi_devices[0].pin_capacitance.declared.source` | `InvalidSource` |
  | 7 | `board.stackup.finished_thickness_um.source` | `InvalidSource` |
  | 8 | `board.stackup.dielectrics[0].source` | `InvalidSource` |
  | 9 | `board.stackup.copper_max_um.source` | `InvalidSource` |
  | 10 | `board.stackup.relative_permittivity_max_permille.source` | `InvalidSource` |

  Why these ten: the name is the person's, and every other one is a citation. The example's citations
  describe Scotty's NUCLEO bench (MB1364's VCAP wiring) and one fab's stackup, which a person's own
  board need not share.
- **`init`'s line:** `{"ok":true,"template":"sentinel-02-bench","out":"<path>","placeholders":[<the ten paths, in document order>]}`,
  exit 0.
- **Why not the other four examples:**
  - `sentinel-02-bench-hse8` declares `board.drawn: false`, a twin stand-in, so `emit` refuses it.
  - `sentinel-01-minimal` and `sentinel-01` carry 30 and 35 acknowledgement `reason`s, so 37 and 44
    placeholders, the opposite of "easy to write".
  - `sentinel-01-ext-bmp581` and both sentinel-01 files carry an `accept_lifecycle[0].statement`.
    Emptied, it is `LifecycleStatementInvalid` with **no `path`**: by kind, not by path, unless that
    refusal's bytes change.
- **Acknowledgements (WO §5 bullet 2):** the bench has none, and its `accept_lifecycle` is empty. The
  template changes no design value, so nothing the example acknowledged goes stale. `verify` will ask a
  person for `acknowledge_unwitnessed` rows (`UnwitnessedFieldNotAcknowledged`) exactly as it asks of the
  example, by name. The README row will say so.

## D2. Numbers and enums a person might own: kept, with the template's design (a decision)

WO §5 bullet 1 asks about every numeric or enum field a person must own: `""` cannot go there, and
`SchemaViolation` does not refuse by name. Measured by setting each of the bench's 30 numbers to 0
(`corpus.py`, mutant `zero`):

- **Paired with a placeholder citation (kept):** each stackup number (`finished_thickness_um`
  nominal/min/max, a dielectric's `min_thickness_um`, `copper_max_um.value`,
  `relative_permittivity_max_permille.value`) and each `pin_capacitance.declared.femtofarads`. The
  person writing the `source` beside a number is reading that number. `mcu.core_supply.mode` (enum,
  `ldo`) is paired the same way with `mcu.core_supply.source`.
- **The template's design (kept), each judged by an engine rule:**
  - `board.rules` (the engine floor, `RuleBelowFloor`, `UnsupportedViaRule`);
  - `speed_hz`, `pull_up_ohms` and the `net_constraints` bounds (the campaign's rise time, `extract`);
  - `address`, from the BMP581's own strap;
  - the domain's 3300 mV / 50 ‰;
  - `copper_layers`;
  - the enums `orientation`, `board.forward` and the layer `role`s.

  These describe the board the template is, the way the parts list does.
- **The alternative, measured: zero them in the template.** Of the 28 numbers that refuse at 0, only
  **7 name a `path`** (`board.rules`, `StackupValue`). The 21 others name an `id` or nothing
  (`schema_version` is one of them; the 20 a person might own include):
  `PinCapacitanceOutOfRange`, `FinishedThickness`, `ZeroPullUp`, `ZeroRiseTime`. `mcu.bor_level` and
  `mcu.hse_hz` **pass** at 0, which is their meaning. Holding any of the 20 by path needs a path added to
  another refusal; holding the two that pass needs a new rule. **If you rule any of them a value a
  person must own, that is the WO §7 stop ("cannot be made to fail `validate` by name without a new
  rule").**
- **Lean:** keep the numbers, ten placeholders.

## D3. N and the kinds

**The WO §7 stop condition is met:** the corpus's most common kind is `SchemaViolation`. It is
56.6 % of 2,391 example mutants and 64.5 % of 14,058 fixture mutants; 97 % of those are `missing field
\`x\``. serde's text names the key and a line and column, never the path.

**`SchemaViolation`, options (lean: a):**
- **(a) No text change.** `init` is its fix: a person who starts from a template meets no missing key,
  and 99.3 % of deletions are this kind. The other 3 % (`unknown variant \`\`, expected one of …`)
  already quote their values.
- **(b) `serde_path_to_error`.** It is **not in `Cargo.lock`**, so it is a new dependency. It adds a
  `path` to every `SchemaViolation`, which changes that refusal's bytes everywhere: 32 pinned lines in
  10 test files. The WO fences both out; it is its own work order.
- **(c) A line-and-column-to-path scanner in the CLI.** No dependency, but it is a second JSON reader
  beside serde's (the reader-desync class `AET-WO-1300` learned). It is its own work order.

**The IR's kinds, by measured frequency:**

| kind | examples | fixtures | distinct field paths | today's text already ends with its fix? |
|---|---:|---:|---:|---|
| `InvalidIdentifier` | 402 | 2,942 | 42 | **no**, for project names and references (below) |
| `UnwitnessedAcknowledgementStale` | 360 | 897 | 1 (inflated by sentinel-01's list) | **no**: it does not say which fields the ledger has |
| `InvalidSource` | 106 | 458 | 13 | yes: it names the path and ends with the shape |
| `UnwitnessedClassUnknown` | 65 | 161 | 1 | yes: it lists the three classes |
| `UnwitnessedRegisterMalformed` | 65 | 161 | 1 | yes: shape plus example (`SPI1.CFG1`) |

**Measured: the quoted fix is refused by the next rule** (WO §5 bullet 4, as `validate` behaves today):
- `project_name: ""` is refused with the base rule ("1 to 64 bytes of printable ASCII …").
- `My_Board` obeys that rule and is then refused with `PROJECT_RULE` (a crate name).
- Only `my-board` passes. A device `reference` behaves the same way (base rule, then the Renode
  identifier rule).
- `check_identifier` returns the broadest rule broken by design, so the first message a template user
  sees (placeholder 1) describes a fix that the next rule refuses.

**Lean: N = 1, `InvalidIdentifier`.**
- **The change:** its message keeps today's text and ends with the exact key and the **full** shape of
  that identifier's family, every rule of the family at once.
- **How:** `check_identifier` and its `&'static str` are unchanged (the scenario reader shares them). The
  variant gains the `IdentifierKind`, a field and not a kind name, and its `#[error]` text appends the
  shape.
- **What a pasted value meets:** it passes the identifier check whole. The next rule is then a
  different kind that names itself (`DuplicateReference`, `UnknownDomain`).
- **The proposed texts:**
  - **Project:** `` `project_name` is not an identifier: <rule>; the fix: "project_name": "<1 to 58 bytes of a-z, 0-9 and -, a letter first, not ending with -, no --, not a Windows device name such as con or com1>" ``
  - **Reference:** `` `spi_devices[0].reference` is not an identifier: <rule>; the fix: "reference": "<a letter, then letters, digits and _, at most 64 bytes, not a name the Renode platform holds (RESERVED_REFERENCES) nor a motor observer name such as pa8obs>" ``
  - **Plain:** `` `i2c_buses[0].sda.net` is not an identifier: <rule>; the fix: "net": "<1 to 64 bytes of printable ASCII 0x21-0x7E, no / { } $ [ ] \" % * , or \\, no date or time shape>" ``
  - For an array element (`reserved_pins[0]`), the key is the array's (`"reserved_pins": [..., "<shape>"]`).
- **Pinned by:** each family's new text by its own test; 9 existing lines in 6 files move with it.

**Options beyond the lean:**
- **(+1) `UnwitnessedAcknowledgementStale`** would end with the ledger's own fields for that register
  and class (`the ledger's fields of PWR.CSR1 as silicon-only: ACTVOSRDY`): pasted, it is exactly what
  passes. It is the most useful quote of the five. But its text is in `cli/src/ledger.rs`, which the
  WO's fence does not list, and its count is one field path. In only if you widen the fence.
- **(+1) `InvalidSource`,** with the key appended for uniformity: little gain, since today's text already
  names the path and ends with the shape. Lean: no.

## D4. MCP: **on** (lean)

- `init` adds no capability over MCP that the tools do not already have. `build --out` already creates
  a new file at any path the client names, with bytes the tool decides. `emit` creates directories and
  overwrites its own three file names (architecture §6, the `RUL-19` trust model).
- `init` is narrower than either: one new regular file, never an overwrite, never through a symlink or
  a FIFO, and bytes fixed in the binary.
- The model that writes a specification (`NG-04` allows it) is MCP's main caller, and northstar §6's
  "too hard to write" failure holds for it too.
- **Cost:** one row in `cli/tests/mcp-tools.json`; `mcp.rs` unchanged.
- **Off** would be a name excluded at `mcp.rs:69`, beside `help` and `mcp`.

## D5. Found on the way (backlog; outside the fence)

- **F1.** A well-formed file is refused as `InvalidJson`, exit 3, in 8 mutants: an externally tagged
  enum object left `{}` (`mcu.hse`, a strap's `tie`, an analog's `full_scale` or `transfer`).
  serde_json calls it `Syntax`, so `parse_spec` reports unreadable input, not an invalid
  specification (exit 2). Changing it changes another refusal's bytes.

## Gates on this branch

No Rust changed: `spike/1608/` only (markdown and one Python harness). The CLAUDE.md gates (fmt,
clippy, test) have nothing new to judge; CI runs on the PR.
